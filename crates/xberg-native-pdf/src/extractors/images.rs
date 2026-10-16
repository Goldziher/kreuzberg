//! Image extraction from PDF XObject resources.
//!
//! This module provides functionality to extract images from PDF documents,
//! including JPEG pass-through for DCT-encoded images and raw pixel decoding
//! for other image types.
//!
//! Phase 5

use crate::error::{Error, Result};
use crate::extractors::ccitt_bilevel;
use crate::geometry::Rect;
use crate::object::ObjectRef;
use std::cmp::min;
use std::path::Path;

/// A PDF image with metadata and pixel data.
///
/// Represents an image extracted from a PDF, including dimensions,
/// color space information, and the actual image data (either JPEG
/// or raw pixels).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PdfImage {
    /// Image width in pixels
    width: u32,
    /// Image height in pixels
    height: u32,
    /// Color space of the image
    color_space: ColorSpace,
    /// Bits per color component (typically 8)
    bits_per_component: u8,
    /// Image data (JPEG or raw pixels)
    #[serde(skip_serializing_if = "ImageData::is_empty")]
    data: ImageData,
    /// Optional bounding box in PDF user space
    bbox: Option<Rect>,
    /// Rotation in degrees
    rotation_degrees: i32,
    /// Transformation matrix
    matrix: [f32; 6],
    /// CCITT decompression parameters (for 1-bit bilevel images)
    #[serde(skip)]
    ccitt_params: Option<crate::decoders::CcittParams>,
    /// Embedded ICC profile associated with the image's colour space,
    /// if any. For a plain `/ICCBased` image this is the profile from
    /// the array; for an `Indexed` image with an `ICCBased` base this
    /// is the base profile. `None` when the document only used
    /// device-dependent colour. Consumed by `save_as_*` to drive the
    /// CMYK→sRGB conversion through the CMM instead of the §10.3.5
    /// additive-clamp fallback.
    #[serde(skip)]
    icc_profile: Option<std::sync::Arc<crate::color::IccProfile>>,
    /// Rendering intent from the image dictionary's `/Intent`, or the
    /// graphics-state default per ISO 32000-1:2008 §8.6.5.8.
    rendering_intent: crate::color::RenderingIntent,
    /// Whether `data` still holds the image's raw samples, i.e. the values
    /// the image dictionary's own entries are expressed in.
    ///
    /// Consumers that range-test stored samples against dictionary values —
    /// colour-key `/Mask` (§8.9.6.4) — are only meaningful in that space.
    /// Every rescale leaves it: mapping a non-default `/Decode` into the
    /// buffer, expanding an Indexed image to palette RGB, unpacking a
    /// sub-byte sample to the 8-bit range, and collapsing a 16-bit sample to
    /// its high byte. Extracted CCITT images are expanded to grayscale samples;
    /// caller-constructed CCITT buffers may remain raw, with their `/Decode`
    /// polarity carried separately in `ccitt_params`.
    #[serde(skip)]
    samples_are_raw: bool,
    /// Whether a non-default `/Decode` array is already mapped into `data`.
    ///
    /// Strictly narrower than `!samples_are_raw`, and not interchangeable
    /// with it: unpacking a sub-byte sample, expanding an Indexed image and
    /// reducing 16-bit samples all leave the raw space *without* folding in
    /// `/Decode`. A consumer that applies `/Decode` itself — separation-plate
    /// routing — must read this one, or it drops a map the image is still
    /// owed (e.g. a `/Decode [1 0]` inversion) on every sub-byte image.
    #[serde(skip)]
    decode_folded_in: bool,
    /// For an `/Indexed` image, the raw index stream unpacked to one byte per
    /// pixel (values `0..=2^bpc-1`), kept alongside the palette-expanded RGB
    /// `data` holds for display. A colour-key `/Mask` on an Indexed image
    /// states its range in this index space (ISO 32000-1 §8.9.6.4), not in
    /// the expanded RGB `samples_are_raw` tracks — so that flag alone cannot
    /// tell a consumer whether masking is possible. `None` for every other
    /// colour space. A JPX-coded Indexed image keeps its decoded index plane
    /// here only when it carries a colour-key `/Mask`. (GH#1899, GH#1885) ~keep
    #[serde(skip)]
    raw_indexed_samples: Option<Vec<u8>>,
    /// The raw samples, one byte per component per pixel in the declared bit depth, kept only
    /// when the dictionary carries a colour-key `/Mask` and an image unpacked from 1, 2 or 4 bits
    /// or with a `/Decode` folded in left that space (GH#1904). The mask ranges are written in that
    /// space, so the renderer tests the mask against these instead. An `/Indexed` image keeps its
    /// indices in `raw_indexed_samples`. ~keep
    #[serde(skip)]
    color_key_samples: Option<Vec<u8>>,
    /// The JPEG 2000 opacity channel and whether its colour is already premultiplied, kept only
    /// when `/SMaskInData` is 1 or 2 and the image has no `/SMask`. (GH#1902) ~keep
    #[serde(skip)]
    soft_mask_in_data: Option<JpxSoftMaskInData>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct JpxSoftMaskInData {
    samples: Vec<u8>,
    premultiplied: bool,
}

impl JpxSoftMaskInData {
    pub(crate) fn samples(&self) -> &[u8] {
        &self.samples
    }

    pub(crate) fn is_premultiplied(&self) -> bool {
        self.premultiplied
    }
}

impl PdfImage {
    /// Create a new PDF image.
    pub fn new(width: u32, height: u32, color_space: ColorSpace, bits_per_component: u8, data: ImageData) -> Self {
        Self {
            width,
            height,
            color_space,
            bits_per_component,
            data,
            bbox: None,
            rotation_degrees: 0,
            matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            ccitt_params: None,
            icc_profile: None,
            rendering_intent: crate::color::RenderingIntent::default(),
            samples_are_raw: true,
            decode_folded_in: false,
            raw_indexed_samples: None,
            color_key_samples: None,
            soft_mask_in_data: None,
        }
    }

    /// Whether the stored samples are still in the image's raw sample space
    /// (see the field docs for what leaves it).
    pub fn samples_are_raw(&self) -> bool {
        self.samples_are_raw
    }

    /// Record that the stored samples have been transformed out of the raw
    /// sample space.
    pub(crate) fn set_samples_are_raw(&mut self, raw: bool) {
        self.samples_are_raw = raw;
    }

    /// Whether a non-default `/Decode` is already mapped into the stored
    /// samples, so a consumer that applies `/Decode` itself must not do so
    /// again. Do not substitute `!samples_are_raw()` — see the field docs.
    pub fn decode_folded_in(&self) -> bool {
        self.decode_folded_in
    }

    /// Record that a non-default `/Decode` has been mapped into the stored
    /// samples.
    pub(crate) fn set_decode_folded_in(&mut self, folded: bool) {
        self.decode_folded_in = folded;
    }

    /// The raw index stream of an `/Indexed` image, one byte per pixel, kept
    /// beside the palette-expanded RGB `data()` holds for display. `None` for
    /// every non-Indexed image and for a JPX-coded Indexed image. See the
    /// field docs. (GH#1899)
    pub(crate) fn raw_indexed_samples(&self) -> Option<&[u8]> {
        self.raw_indexed_samples.as_deref()
    }

    /// Record an `/Indexed` image's raw index plane.
    pub(crate) fn set_raw_indexed_samples(&mut self, samples: Vec<u8>) {
        self.raw_indexed_samples = Some(samples);
    }

    /// The raw samples a colour-key `/Mask` is tested against, when the stored samples left the
    /// raw sample space.
    pub(crate) fn color_key_samples(&self) -> Option<&[u8]> {
        self.color_key_samples.as_deref()
    }

    /// The opacity channel a JPEG 2000 image carries as its soft mask (`/SMaskInData`).
    pub(crate) fn soft_mask_in_data(&self) -> Option<&JpxSoftMaskInData> {
        self.soft_mask_in_data.as_ref()
    }

    /// Create a new PDF image with spatial metadata.
    pub fn with_spatial(
        width: u32,
        height: u32,
        color_space: ColorSpace,
        bits_per_component: u8,
        data: ImageData,
        bbox: Rect,
        rotation: i32,
        matrix: [f32; 6],
    ) -> Self {
        Self {
            width,
            height,
            color_space,
            bits_per_component,
            data,
            bbox: Some(bbox),
            rotation_degrees: rotation,
            matrix,
            ccitt_params: None,
            icc_profile: None,
            rendering_intent: crate::color::RenderingIntent::default(),
            samples_are_raw: true,
            decode_folded_in: false,
            raw_indexed_samples: None,
            color_key_samples: None,
            soft_mask_in_data: None,
        }
    }

    /// Create a new PDF image with a bounding box (convenience wrapper).
    pub fn with_bbox(
        width: u32,
        height: u32,
        color_space: ColorSpace,
        bits_per_component: u8,
        data: ImageData,
        bbox: Rect,
    ) -> Self {
        Self::with_spatial(
            width,
            height,
            color_space,
            bits_per_component,
            data,
            bbox,
            0,
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        )
    }

    /// Create a new PDF image with CCITT parameters.
    pub fn with_ccitt_params(
        width: u32,
        height: u32,
        color_space: ColorSpace,
        bits_per_component: u8,
        data: ImageData,
        ccitt_params: crate::decoders::CcittParams,
    ) -> Self {
        Self {
            width,
            height,
            color_space,
            bits_per_component,
            data,
            bbox: None,
            rotation_degrees: 0,
            matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            ccitt_params: Some(ccitt_params),
            icc_profile: None,
            rendering_intent: crate::color::RenderingIntent::default(),
            samples_are_raw: true,
            decode_folded_in: false,
            raw_indexed_samples: None,
            color_key_samples: None,
            soft_mask_in_data: None,
        }
    }

    /// Get the image width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get the image height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get the image color space.
    pub fn color_space(&self) -> &ColorSpace {
        &self.color_space
    }

    /// Get bits per component.
    pub fn bits_per_component(&self) -> u8 {
        self.bits_per_component
    }

    /// Get the image data.
    pub fn data(&self) -> &ImageData {
        &self.data
    }

    /// Get the bounding box if available.
    pub fn bbox(&self) -> Option<&Rect> {
        self.bbox.as_ref()
    }

    /// Set the bounding box for this image.
    pub fn set_bbox(&mut self, bbox: Rect) {
        self.bbox = Some(bbox);
    }

    /// Get rotation in degrees.
    pub fn rotation_degrees(&self) -> i32 {
        self.rotation_degrees
    }

    /// Set rotation in degrees.
    pub fn set_rotation_degrees(&mut self, rotation: i32) {
        self.rotation_degrees = rotation;
    }

    /// Get transformation matrix.
    pub fn matrix(&self) -> [f32; 6] {
        self.matrix
    }

    /// Set transformation matrix.
    pub fn set_matrix(&mut self, matrix: [f32; 6]) {
        self.matrix = matrix;
    }

    /// Set CCITT decompression parameters for this image.
    pub fn set_ccitt_params(&mut self, params: crate::decoders::CcittParams) {
        self.ccitt_params = Some(params);
    }

    /// Get CCITT decompression parameters if available.
    pub fn ccitt_params(&self) -> Option<&crate::decoders::CcittParams> {
        self.ccitt_params.as_ref()
    }

    /// Embedded ICC profile associated with the image, if any.
    pub fn icc_profile(&self) -> Option<&std::sync::Arc<crate::color::IccProfile>> {
        self.icc_profile.as_ref()
    }

    /// Attach an ICC profile (used by extractors; colour conversion
    /// picks it up automatically when present).
    pub fn set_icc_profile(&mut self, profile: std::sync::Arc<crate::color::IccProfile>) {
        self.icc_profile = Some(profile);
    }

    /// Rendering intent — ISO 32000-1:2008 §8.6.5.8, defaults to
    /// `RelativeColorimetric`.
    pub fn rendering_intent(&self) -> crate::color::RenderingIntent {
        self.rendering_intent
    }

    /// Set the rendering intent (used by extractors when they see an
    /// explicit `/Intent` entry on the image dictionary).
    pub fn set_rendering_intent(&mut self, intent: crate::color::RenderingIntent) {
        self.rendering_intent = intent;
    }

    /// Build the source→sRGB transform from this image's embedded ICC
    /// profile (if any). Returns `None` when the image uses purely
    /// device-dependent colour, or when no profile was resolved at
    /// extraction time.
    ///
    /// The resulting transform is component-agnostic: callers pick the
    /// matching `Transform::convert_{cmyk,rgb,gray}_*` method based on
    /// the source pixel format. Used by the `decode_cmyk_jpeg_to_rgb_…`,
    /// `cmyk_to_rgb_with_transform`, and `save_raw_as_*` paths.
    fn build_icc_transform(&self) -> Option<crate::color::Transform> {
        self.icc_profile
            .as_ref()
            .map(|p| crate::color::Transform::new_srgb_target(p.clone(), self.rendering_intent))
    }

    /// Save the image as PNG format.
    pub fn save_as_png(&self, path: impl AsRef<Path>) -> Result<()> {
        match &self.data {
            ImageData::Jpeg(jpeg_data) => {
                if self.color_space.components() == 4 {
                    let transform = self.build_icc_transform();
                    let rgb = decode_cmyk_jpeg_to_rgb_with_profile(jpeg_data, transform.as_ref())?;
                    let buf = image::ImageBuffer::<image::Rgb<u8>, _>::from_raw(self.width, self.height, rgb)
                        .ok_or_else(|| Error::Image("Invalid CMYK image dimensions".to_string()))?;
                    buf.save_with_format(path, image::ImageFormat::Png)
                        .map_err(|e| Error::Image(format!("Failed to save PNG: {}", e)))
                } else {
                    save_jpeg_as_png(jpeg_data, path)
                }
            }
            ImageData::Raw { pixels, format } => {
                // Always build the transform if a profile is present; the
                // save helper picks the right convert_* method for the
                // pixel format. RGB/Gray ICCBased samples would otherwise
                // be written as-is, which is wrong when the profile is
                // wide-gamut (Adobe RGB, ProPhoto, …) or a calibrated
                // grayscale other than sRGB gamma. ~keep
                let transform = self.build_icc_transform();
                save_raw_as_png(pixels, self.width, self.height, *format, transform.as_ref(), path)
            }
        }
    }

    /// Save the image as JPEG format.
    pub fn save_as_jpeg(&self, path: impl AsRef<Path>) -> Result<()> {
        match &self.data {
            // Pass-through for RGB / grayscale JPEGs — viewers handle those
            // uniformly. CMYK JPEGs (4-channel ColorSpace such as DeviceCMYK
            // or ICCBased N=4) must be decoded and re-encoded as RGB because
            // most viewers either fail to open CMYK JPEGs or display them
            // with inverted or washed-out colors. `decode_cmyk_jpeg_to_rgb`
            // pulls CMYK samples via `jpeg-decoder`, inspects the APP14
            // Adobe marker to detect the inverted-channel convention
            // Photoshop / InDesign / WPS write, inverts when present, then
            // does a naive CMYK→RGB conversion (full ICC profile handling
            // is a follow-up). ~keep
            ImageData::Jpeg(jpeg_data) => {
                if self.color_space.components() == 4 {
                    let transform = self.build_icc_transform();
                    let rgb = decode_cmyk_jpeg_to_rgb_with_profile(jpeg_data, transform.as_ref())?;
                    let buf = image::ImageBuffer::<image::Rgb<u8>, _>::from_raw(self.width, self.height, rgb)
                        .ok_or_else(|| Error::Image("Invalid CMYK image dimensions".to_string()))?;
                    buf.save_with_format(path, image::ImageFormat::Jpeg)
                        .map_err(|e| Error::Image(format!("Failed to save JPEG: {}", e)))
                } else {
                    std::fs::write(path, jpeg_data).map_err(Error::from)
                }
            }
            ImageData::Raw { pixels, format } => {
                let transform = self.build_icc_transform();
                save_raw_as_jpeg(pixels, self.width, self.height, *format, transform.as_ref(), path)
            }
        }
    }

    /// Convert image to PNG bytes in memory.
    pub fn to_png_bytes(&self) -> Result<Vec<u8>> {
        use image::ImageEncoder;
        use image::codecs::png::{CompressionType, FilterType, PngEncoder};
        use std::io::Cursor;

        let mut buffer = Cursor::new(Vec::new());
        // Adaptive per-scanline filtering (Sub/Up/Avg/Paeth) is where PNG's
        // deflate win actually comes from: with `NoFilter` deflate cannot
        // exploit a smooth gradient and the stream stays near-raw — a 256x256
        // ramp encodes *larger* than its uncompressed samples. Purely a
        // container-size choice; PNG is lossless, so the decoded pixels are
        // byte-identical either way. ~keep
        let encoder = PngEncoder::new_with_quality(&mut buffer, CompressionType::Default, FilterType::Adaptive);

        match &self.data {
            ImageData::Raw { pixels, format } => {
                let expected_gray = (self.width * self.height) as usize;
                let expected_rgb = expected_gray * 3;

                if *format == PixelFormat::Grayscale
                    && matches!(self.color_space, ColorSpace::DeviceGray | ColorSpace::CalGray)
                    && pixels.len() == expected_gray
                {
                    // image 0.25 changed `write_image` to take
                    // `ExtendedColorType` — `ColorType::*` now converts
                    // through `Into`. API-only change, same semantics. ~keep
                    encoder
                        .write_image(pixels, self.width, self.height, image::ColorType::L8.into())
                        .map_err(|e| Error::Encode(format!("Failed to encode PNG: {}", e)))?;
                } else if *format == PixelFormat::RGB && pixels.len() == expected_rgb {
                    encoder
                        .write_image(pixels, self.width, self.height, image::ColorType::Rgb8.into())
                        .map_err(|e| Error::Encode(format!("Failed to encode PNG: {}", e)))?;
                } else {
                    let dynamic_image = self.to_dynamic_image()?;
                    let rgb = dynamic_image.to_rgb8();
                    // `ImageBuffer::from_raw` accepts a buffer at least as long
                    // as the image needs, so a mis-declared depth can leave the
                    // RGB buffer larger than width×height×3. Reject that here
                    // with a recoverable error instead of handing the encoder a
                    // mismatched buffer, which it asserts on (panicking through
                    // the FFI boundary where callers cannot catch it). ~keep
                    if rgb.as_raw().len() != expected_rgb {
                        return Err(Error::Encode(format!(
                            "image buffer length {} does not match {}x{} RGB image ({} expected)",
                            rgb.as_raw().len(),
                            self.width,
                            self.height,
                            expected_rgb
                        )));
                    }
                    encoder
                        .write_image(rgb.as_raw(), self.width, self.height, image::ColorType::Rgb8.into())
                        .map_err(|e| Error::Encode(format!("Failed to encode PNG: {}", e)))?;
                }
            }
            ImageData::Jpeg(_) => {
                let dynamic_image = self.to_dynamic_image()?;
                let rgb = dynamic_image.to_rgb8();
                encoder
                    .write_image(rgb.as_raw(), self.width, self.height, image::ColorType::Rgb8.into())
                    .map_err(|e| Error::Encode(format!("Failed to encode PNG: {}", e)))?;
            }
        }

        Ok(buffer.into_inner())
    }

    /// Convert image to a base64 data URI for embedding in HTML.
    pub fn to_base64_data_uri(&self) -> Result<String> {
        use base64::{Engine, engine::general_purpose::STANDARD};

        match &self.data {
            ImageData::Jpeg(jpeg_data) => {
                let base64_str = STANDARD.encode(jpeg_data);
                Ok(format!("data:image/jpeg;base64,{}", base64_str))
            }
            ImageData::Raw { .. } => {
                let png_bytes = self.to_png_bytes()?;
                let base64_str = STANDARD.encode(&png_bytes);
                Ok(format!("data:image/png;base64,{}", base64_str))
            }
        }
    }

    /// Convert this PDF image to a `DynamicImage`.
    pub fn to_dynamic_image(&self) -> Result<image::DynamicImage> {
        match &self.data {
            ImageData::Jpeg(jpeg_data) => self.jpeg_to_dynamic_image(jpeg_data),
            ImageData::Raw { pixels, format } => self.raw_to_dynamic_image(pixels, format),
        }
    }

    /// [`to_dynamic_image`](Self::to_dynamic_image) for `ImageData::Jpeg`.
    fn jpeg_to_dynamic_image(&self, jpeg_data: &[u8]) -> Result<image::DynamicImage> {
        if self.color_space.components() == 4 {
            // 4-component (DeviceCMYK / ICCBased N=4) JPEGs must go
            // through the Adobe-aware CMYK path. image::load_from_memory
            // routes them through zune-jpeg's own CMYK->RGB, which does
            // not honor the APP14 inversion and yields near-black output
            // (see decode_cmyk_jpeg_to_rgb_with_profile). ~keep
            let transform = self.build_icc_transform();
            let rgb = decode_cmyk_jpeg_to_rgb_with_profile(jpeg_data, transform.as_ref())?;
            return image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_raw(self.width, self.height, rgb)
                .ok_or_else(|| Error::Decode("Invalid CMYK image dimensions".to_string()))
                .map(image::DynamicImage::ImageRgb8);
        }
        tracing::debug!(
            "Decoding JPEG data ({} bytes), starts with: {:02X?}",
            jpeg_data.len(),
            &jpeg_data[..min(jpeg_data.len(), 16)]
        );
        image::load_from_memory(jpeg_data).map_err(|e| Error::Decode(format!("Failed to decode JPEG: {}", e)))
    }

    /// [`to_dynamic_image`](Self::to_dynamic_image) for `ImageData::Raw`.
    fn raw_to_dynamic_image(&self, pixels: &[u8], format: &PixelFormat) -> Result<image::DynamicImage> {
        let is_1bpc_gray = self.bits_per_component == 1 && matches!(self.color_space, ColorSpace::DeviceGray);
        if is_1bpc_gray && self.ccitt_params.is_some() {
            return self.ccitt_to_dynamic_image(pixels);
        }
        if is_1bpc_gray {
            return self.unpack_1bpc_gray_to_dynamic_image(pixels);
        }
        match (format, self.color_space) {
            (PixelFormat::RGB, ColorSpace::DeviceRGB) => {
                image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_raw(self.width, self.height, pixels.to_vec())
                    .ok_or_else(|| Error::Decode("Invalid image dimensions".to_string()))
                    .map(image::DynamicImage::ImageRgb8)
            }
            (PixelFormat::Grayscale, ColorSpace::DeviceGray) => {
                image::ImageBuffer::<image::Luma<u8>, Vec<u8>>::from_raw(self.width, self.height, pixels.to_vec())
                    .ok_or_else(|| Error::Decode("Invalid image dimensions".to_string()))
                    .map(image::DynamicImage::ImageLuma8)
            }
            _ => {
                let rgb_pixels = match format {
                    PixelFormat::Grayscale => pixels.iter().flat_map(|&g| vec![g, g, g]).collect(),
                    PixelFormat::CMYK => cmyk_to_rgb_with_transform(pixels, self.build_icc_transform().as_ref()),
                    PixelFormat::RGB => pixels.to_vec(),
                };
                image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_raw(self.width, self.height, rgb_pixels)
                    .ok_or_else(|| Error::Decode("Invalid image dimensions".to_string()))
                    .map(image::DynamicImage::ImageRgb8)
            }
        }
    }

    /// [`raw_to_dynamic_image`](Self::raw_to_dynamic_image) for a genuinely
    /// CCITT-filtered 1-bit DeviceGray image. Only reached from there — see
    /// `extract_image_from_xobject`, which sets `ccitt_params` solely when
    /// the XObject's `/Filter` is CCITTFaxDecode. ~keep
    fn ccitt_to_dynamic_image(&self, pixels: &[u8]) -> Result<image::DynamicImage> {
        let Some(params) = self.ccitt_params.clone() else {
            unreachable!("guarded by ccitt_params.is_some() above")
        };

        let decompressed = ccitt_bilevel::decompress_ccitt(pixels, &params)?;
        let grayscale = ccitt_bilevel::bilevel_to_grayscale(&decompressed, self.width, self.height);

        image::ImageBuffer::<image::Luma<u8>, Vec<u8>>::from_raw(self.width, self.height, grayscale)
            .ok_or_else(|| Error::Decode("Invalid image dimensions".to_string()))
            .map(image::DynamicImage::ImageLuma8)
    }

    /// [`raw_to_dynamic_image`](Self::raw_to_dynamic_image) for packed 1-bit
    /// DeviceGray rows, `ceil(width / 8)` bytes each, MSB first.
    /// `extract_image_from_xobject` unpacks sub-byte images to 8-bit samples
    /// (with /Decode applied) before storing, so this branch only serves
    /// externally constructed images; it unpacks with the ISO 32000-1
    /// §8.9.5.2 Table 90 default semantics: sample bit 0 -> component 0.0
    /// (black), bit 1 -> component 1.0 (white). ~keep
    fn unpack_1bpc_gray_to_dynamic_image(&self, pixels: &[u8]) -> Result<image::DynamicImage> {
        // `/Width` and `/Height` are document-declared and untrusted, so the
        // output buffer size is checked before allocating — the same 256 MiB
        // cap `expand_indexed_to_rgb_with_transform` and
        // `samples_to_decoded_bytes` use elsewhere in this file. ~keep
        const MAX_GRAYSCALE_OUTPUT_BYTES: usize = 256 * 1024 * 1024;
        let output_bytes = (self.width as usize).checked_mul(self.height as usize).ok_or_else(|| {
            Error::Decode(format!(
                "1-bit DeviceGray image output size overflow: {} x {} exceeds usize",
                self.width, self.height
            ))
        })?;
        if output_bytes > MAX_GRAYSCALE_OUTPUT_BYTES {
            return Err(Error::Decode(format!(
                "1-bit DeviceGray image decode would produce {output_bytes} bytes, exceeds guard \
                 limit of {MAX_GRAYSCALE_OUTPUT_BYTES} bytes (width={}, height={})",
                self.width, self.height
            )));
        }
        let row_bytes = (self.width as usize).div_ceil(8);
        let mut grayscale = Vec::with_capacity(output_bytes);
        for row in 0..self.height as usize {
            let row_start = row * row_bytes;
            for col in 0..self.width as usize {
                let byte_idx = row_start + col / 8;
                let bit = pixels.get(byte_idx).map(|b| (b >> (7 - (col % 8))) & 1).unwrap_or(1);
                grayscale.push(if bit == 0 { 0x00 } else { 0xFF });
            }
        }

        image::ImageBuffer::<image::Luma<u8>, Vec<u8>>::from_raw(self.width, self.height, grayscale)
            .ok_or_else(|| Error::Decode("Invalid image dimensions".to_string()))
            .map(image::DynamicImage::ImageLuma8)
    }
}

/// Image data representation.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(untagged)]
pub enum ImageData {
    /// JPEG-encoded image data.
    Jpeg(Vec<u8>),
    /// Raw pixel data with a specified format.
    Raw {
        /// Raw pixel bytes.
        pixels: Vec<u8>,
        /// Pixel format (RGB, Grayscale, CMYK).
        format: PixelFormat,
    },
}

impl ImageData {
    /// Returns true if the image data is empty.
    pub fn is_empty(&self) -> bool {
        match self {
            ImageData::Jpeg(data) => data.is_empty(),
            ImageData::Raw { pixels, .. } => pixels.is_empty(),
        }
    }
}

/// PDF color space types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ColorSpace {
    /// RGB color space (3 components).
    DeviceRGB,
    /// Grayscale color space (1 component).
    DeviceGray,
    /// CMYK color space (4 components).
    DeviceCMYK,
    /// Indexed (palette-based) color space.
    Indexed,
    /// Calibrated grayscale.
    CalGray,
    /// Calibrated RGB.
    CalRGB,
    /// CIE L*a*b* color space.
    Lab,
    /// ICC profile-based color space with N components.
    ICCBased(usize),
    /// Separation (spot color) space.
    Separation,
    /// DeviceN (multi-ink) color space.
    DeviceN,
    /// Pattern color space.
    Pattern,
}

impl ColorSpace {
    /// Returns the number of color components for this color space.
    pub fn components(&self) -> usize {
        match self {
            ColorSpace::DeviceGray => 1,
            ColorSpace::DeviceRGB => 3,
            ColorSpace::DeviceCMYK => 4,
            ColorSpace::Indexed => 1,
            ColorSpace::CalGray => 1,
            ColorSpace::CalRGB => 3,
            ColorSpace::Lab => 3,
            ColorSpace::ICCBased(n) => *n,
            ColorSpace::Separation => 1,
            ColorSpace::DeviceN => 4,
            ColorSpace::Pattern => 0,
        }
    }
}

/// Pixel format for raw image data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum PixelFormat {
    /// RGB format (3 bytes per pixel).
    RGB,
    /// Grayscale format (1 byte per pixel).
    Grayscale,
    /// CMYK format (4 bytes per pixel).
    CMYK,
}

impl PixelFormat {
    /// Returns the number of bytes per pixel for this format.
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            PixelFormat::Grayscale => 1,
            PixelFormat::RGB => 3,
            PixelFormat::CMYK => 4,
        }
    }
}

fn color_space_to_pixel_format(color_space: &ColorSpace) -> PixelFormat {
    match color_space {
        ColorSpace::DeviceGray => PixelFormat::Grayscale,
        ColorSpace::DeviceRGB => PixelFormat::RGB,
        ColorSpace::DeviceCMYK => PixelFormat::CMYK,
        ColorSpace::Indexed => PixelFormat::RGB,
        ColorSpace::CalGray => PixelFormat::Grayscale,
        ColorSpace::CalRGB => PixelFormat::RGB,
        ColorSpace::Lab => PixelFormat::RGB,
        ColorSpace::ICCBased(n) => match n {
            1 => PixelFormat::Grayscale,
            3 => PixelFormat::RGB,
            4 => PixelFormat::CMYK,
            _ => PixelFormat::RGB,
        },
        ColorSpace::Separation => PixelFormat::Grayscale,
        ColorSpace::DeviceN => PixelFormat::CMYK,
        ColorSpace::Pattern => PixelFormat::RGB,
    }
}

/// Parse a ColorSpace name from a PDF object.
pub fn parse_color_space(obj: &crate::object::Object) -> Result<ColorSpace> {
    use crate::object::Object;

    match obj {
        Object::Name(name) => match name.as_str() {
            "DeviceRGB" => Ok(ColorSpace::DeviceRGB),
            "DeviceGray" => Ok(ColorSpace::DeviceGray),
            "DeviceCMYK" => Ok(ColorSpace::DeviceCMYK),
            "Pattern" => Ok(ColorSpace::Pattern),
            other => Err(Error::Image(format!("Unsupported color space: {}", other))),
        },
        Object::Array(arr) if !arr.is_empty() => {
            if let Some(name) = arr[0].as_name() {
                match name {
                    "Indexed" => Ok(ColorSpace::Indexed),
                    "CalGray" => Ok(ColorSpace::CalGray),
                    "CalRGB" => Ok(ColorSpace::CalRGB),
                    "Lab" => Ok(ColorSpace::Lab),
                    "ICCBased" => Ok(ColorSpace::ICCBased(iccbased_num_components(arr))),
                    "Separation" => Ok(ColorSpace::Separation),
                    "DeviceN" => Ok(ColorSpace::DeviceN),
                    "Pattern" => Ok(ColorSpace::Pattern),
                    other => Err(Error::Image(format!("Unsupported array color space: {}", other))),
                }
            } else {
                Err(Error::Image("Color space array must start with a name".to_string()))
            }
        }
        _ => Err(Error::Image(format!("Invalid color space object: {:?}", obj))),
    }
}

/// The `/N` component count from an `[/ICCBased <stream>]` color space array,
/// defaulting to 3 when the stream dict is missing, unresolved, or carries no
/// `/N` integer.
fn iccbased_num_components(arr: &[crate::object::Object]) -> usize {
    use crate::object::Object;

    let Some(stream_dict) = arr.get(1).and_then(|obj| obj.as_dict()) else {
        return 3;
    };
    stream_dict
        .get("N")
        .and_then(|obj| match obj {
            Object::Integer(n) => Some(*n as usize),
            _ => None,
        })
        .unwrap_or(3)
}

/// True when a 1-bit image's `/Decode` array is `[1 0]` (inverted) rather
/// than the DeviceGray default `[0 1]`. ISO 32000-1:2008 8.9.5.2 Table 90:
/// for a 1-bit component the default Decode maps sample 0 -> black, 1 ->
/// white; `[1 0]` reverses that (0 -> white, 1 -> black). Absent, malformed,
/// or non-inverted arrays are treated as the default (no inversion).
fn decode_array_inverts_1bpc(decode: Option<&crate::object::Object>) -> bool {
    let arr = match decode.and_then(|o| o.as_array()) {
        Some(a) if a.len() == 2 => a,
        _ => return false,
    };
    let as_num = |o: &crate::object::Object| o.as_integer().map(|i| i as f64).or_else(|| o.as_real());
    matches!((as_num(&arr[0]), as_num(&arr[1])), (Some(lo), Some(hi)) if lo > hi)
}

/// The number of inks a `/DeviceN` colour space names (ISO 32000-1 §8.6.6.5,
/// `[/DeviceN names alternate tint]`), or `None` for any other space.
///
/// `ColorSpace::DeviceN` is a unit variant: the parser keeps the tag and drops
/// the name array, so `components()` can only answer a flat 4. Sample geometry
/// needs the true count, and 4 is merely the most common one.
fn devicen_ink_count(cs_obj: &crate::object::Object) -> Option<usize> {
    let arr = cs_obj.as_array()?;
    if !matches!(arr.first(), Some(crate::object::Object::Name(n)) if n == "DeviceN") {
        return None;
    }
    match arr.get(1)? {
        crate::object::Object::Array(names) if !names.is_empty() => Some(names.len()),
        _ => None,
    }
}

/// Per-component `(Dmin, Dmax)` pairs from a `/Decode` array, or `None` when
/// the array is absent or does not hold `2 × ncomp` numbers.
fn decode_ranges(decode: Option<&crate::object::Object>, ncomp: usize) -> Option<Vec<(f32, f32)>> {
    let arr = decode.and_then(|o| o.as_array())?;
    if arr.len() != ncomp * 2 {
        return None;
    }
    let as_num = |o: &crate::object::Object| o.as_integer().map(|i| i as f64).or_else(|| o.as_real());
    let mut out = Vec::with_capacity(ncomp);
    for pair in arr.as_chunks::<2>().0 {
        out.push((as_num(&pair[0])? as f32, as_num(&pair[1])? as f32));
    }
    Some(out)
}

/// Interleaved image samples as one 8-bit byte per component with `/Decode`
/// applied (ISO 32000-1 §8.9.5.2): each raw sample maps through
/// `Dmin + raw · (Dmax − Dmin) / (2^bpc − 1)`, then scales to `0..=255`.
/// Sub-byte samples are unpacked from their MSB-first, byte-aligned rows, so
/// the result always holds `width × height × ncomp` bytes.
/// Returns `None` for a `/BitsPerComponent` outside the spec's {1, 2, 4, 8}
/// (Table 89) rather than trusting it as shift arithmetic: a malformed value
/// must surface as a recoverable decode error, never a panic.
fn samples_to_decoded_bytes(
    data: &[u8],
    width: u32,
    height: u32,
    ncomp: usize,
    bpc: u8,
    ranges: Option<&[(f32, f32)]>,
) -> Option<Vec<u8>> {
    if !matches!(bpc, 1 | 2 | 4 | 8) || ncomp == 0 {
        return None;
    }
    let bpc = usize::from(bpc);
    let (samples_per_row, row_bytes, total) = unpacked_sample_geometry(width, height, ncomp, bpc)?;
    let lut = build_decode_lut(ncomp, bpc, ranges);

    if bpc == 8 {
        return Some(
            data.iter()
                .enumerate()
                .map(|(i, &b)| lut[i % ncomp][b as usize])
                .collect(),
        );
    }
    let mask = (1u32 << bpc) - 1;
    let mut out = Vec::with_capacity(total);
    for row in 0..height as usize {
        let row_start = row * row_bytes;
        for s in 0..samples_per_row {
            let bit_offset = s * bpc;
            // Past the end of a short stream the sample reads as zero, which
            // is what the packed path produced before unpacking existed. ~keep
            let byte = data.get(row_start + bit_offset / 8).copied().unwrap_or(0);
            // Cannot underflow: bpc < 8 here (the bpc == 8 case returned
            // above) and `bit_offset % 8` is at most 8 - bpc for those depths. ~keep
            let shift = 8 - bpc - (bit_offset % 8);
            let raw = ((byte >> shift) as u32) & mask;
            out.push(lut[s % ncomp][raw as usize]);
        }
    }
    Some(out)
}

/// Row/total-byte geometry for [`samples_to_decoded_bytes`]'s output buffer,
/// with the overflow and 256 MiB output-size guards it always applied.
/// Returns `(samples_per_row, row_bytes, total)`, or `None` when a product
/// overflows `usize` or the unpacked buffer would exceed the cap.
fn unpacked_sample_geometry(width: u32, height: u32, ncomp: usize, bpc: usize) -> Option<(usize, usize, usize)> {
    // Sizes come from /Width and /Height, which a hostile file controls, so
    // every product is checked: a wrapped `usize` (reachable on 32-bit wasm
    // with ordinary dimensions) would under-reserve and then grow until the
    // allocator aborts, and an allocation failure aborts the process rather
    // than returning an error. ~keep
    let samples_per_row = (width as usize).checked_mul(ncomp)?;
    let total = samples_per_row.checked_mul(height as usize)?;
    let row_bytes = samples_per_row.checked_mul(bpc)?.div_ceil(8);
    // A stream shorter than its declared size is padded, matching what the
    // packed path already did. Refusing it instead would drop the image back
    // to its packed bytes with `/Decode` never applied, which renders the
    // negative of the intended picture — a louder wrong answer than the
    // displaced one padding gives.
    //
    // Refusing was also what bounded this allocation, so bound it directly:
    // the same 256 MiB ceiling `expand_indexed_to_rgb_with_transform` uses. ~keep
    /// Hard cap on the unpacked output buffer (256 MiB), matching the
    /// Indexed expander. A legitimate image does not reach it; a hostile
    /// `/Width` × `/Height` does.
    const MAX_UNPACKED_OUTPUT_BYTES: usize = 256 * 1024 * 1024;
    if total > MAX_UNPACKED_OUTPUT_BYTES {
        // Refusing here is the same fallback the short-stream case was moved
        // off: the caller keeps the packed bytes and any /Decode goes
        // unapplied, which renders the negative of the picture. Nothing in a
        // real corpus reaches this, so the trade is worth it against an
        // adversarial /Width × /Height — but say so rather than letting a
        // wrong-polarity image out silently. ~keep
        tracing::warn!(
            "Refusing to unpack a {total}-byte image buffer (cap {MAX_UNPACKED_OUTPUT_BYTES}); \
             samples stay packed and any /Decode is left unapplied"
        );
        return None;
    }
    Some((samples_per_row, row_bytes, total))
}

/// Per-component 8-bit `/Decode` + bit-depth-scaling lookup table (ISO
/// 32000-1 §8.9.5.2): `Dmin + raw · (Dmax − Dmin) / (2^bpc − 1)`, scaled to
/// `0..=255`. Indexed by raw sample value, one table per component.
fn build_decode_lut(ncomp: usize, bpc: usize, ranges: Option<&[(f32, f32)]>) -> Vec<[u8; 256]> {
    let levels = 1usize << bpc;
    let max = (levels - 1) as f32;
    (0..ncomp)
        .map(|comp| {
            let (lo, hi) = ranges.map_or((0.0, 1.0), |r| r[comp]);
            let mut table = [0u8; 256];
            for (raw, slot) in table.iter_mut().enumerate().take(levels) {
                let v = lo + (raw as f32) * (hi - lo) / max;
                *slot = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
            table
        })
        .collect()
}

fn expected_image_filter_output_size(
    dict: &std::collections::HashMap<String, crate::object::Object>,
    width: u32,
    height: u32,
    components: usize,
    bits_per_component: u8,
) -> Result<Option<usize>> {
    if components == 0 || bits_per_component == 0 {
        return Ok(None);
    }

    let row_bits = (width as usize)
        .checked_mul(components)
        .and_then(|samples| samples.checked_mul(bits_per_component as usize))
        .ok_or_else(|| Error::Decode("image raster dimensions overflow".to_string()))?;
    let row_bytes = row_bits.div_ceil(8);
    let raster_bytes = row_bytes
        .checked_mul(height as usize)
        .ok_or_else(|| Error::Decode("image raster byte count overflow".to_string()))?;

    let Some(params) = crate::object::extract_decode_params(dict.get("DecodeParms")) else {
        return Ok(Some(raster_bytes));
    };
    if params.predictor == 1 {
        return Ok(Some(raster_bytes));
    }
    if params.checked_pixel_bytes_per_row()? != row_bytes {
        return Ok(None);
    }

    let encoded_bytes = params
        .checked_bytes_per_row()?
        .checked_mul(height as usize)
        .ok_or_else(|| Error::Decode("predictor-framed image byte count overflow".to_string()))?;
    Ok(Some(encoded_bytes))
}

fn decode_dimension_bounded_image_stream(
    doc: Option<&crate::document::PdfDocument>,
    xobject: &crate::object::Object,
    obj_ref: Option<ObjectRef>,
    expected_filter_output_size: Option<usize>,
) -> Result<Vec<u8>> {
    match (doc, obj_ref, expected_filter_output_size) {
        (Some(document), Some(reference), Some(expected_size)) => {
            document.decode_image_stream_with_encryption(xobject, reference, expected_size)
        }
        (Some(document), Some(reference), None) => document.decode_stream_with_encryption(xobject, reference),
        (_, _, Some(expected_size)) => xobject.decode_image_stream_data(expected_size),
        (_, _, None) => xobject.decode_stream_data(),
    }
}

// Test-only count of calls into `extract_image_from_xobject`, the single function every
// image-decode path (`PdfDocument::extract_images` and `PdfImageHandle::decode`) funnels
// through. Thread-local rather than a process-wide `AtomicUsize`: `cargo test`'s default
// runner puts each test on its own thread, so a thread-local counter needs no cross-test
// locking to stay free of interference from unrelated tests decoding images concurrently
// elsewhere in the same binary (GH#1732 regression coverage). A doc comment (`///`) here
// would attach to the macro invocation rather than the generated static, which rustc
// warns is meaningless -- a plain `//` comment avoids that warning. ~keep
#[cfg(test)]
thread_local! {
    static DECODE_CALL_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Current value of this thread's [`DECODE_CALL_COUNT`].
#[cfg(test)]
pub(crate) fn decode_call_count() -> usize {
    DECODE_CALL_COUNT.with(std::cell::Cell::get)
}

/// Resolve an image dictionary's `/ColorSpace` entry to a direct object:
/// one indirect-reference hop (with the document's named-resource map
/// substituted for a plain `/Name`), then a second hop for array-form
/// spaces whose second element is itself commonly an indirect reference
/// to an ICC profile / palette stream (`[/ICCBased <ref>]`,
/// `[/Indexed <base> <hi> <palette_ref>]`). `parse_color_space` only
/// inspects the immediate `Object::Stream` dict, so leaving that second
/// reference unresolved silently falls back to `N = 3` and mislabels a
/// CMYK (N = 4) image as RGB. Split out of [`resolve_image_xobject_metadata`]
/// purely to keep that function shorter. ~keep
fn resolve_image_color_space_object(
    doc: Option<&crate::document::PdfDocument>,
    color_space_obj: &crate::object::Object,
    color_space_map: Option<&std::collections::HashMap<String, crate::object::Object>>,
) -> Result<crate::object::Object> {
    use crate::object::Object;

    let resolved_color_space = if let Some(d) = doc {
        let res = if let Some(obj_ref) = color_space_obj.as_reference() {
            d.load_object(obj_ref)?
        } else {
            color_space_obj.clone()
        };
        if let Object::Name(ref name) = res {
            if let Some(map) = color_space_map {
                map.get(name).cloned().unwrap_or(res)
            } else {
                res
            }
        } else {
            res
        }
    } else {
        color_space_obj.clone()
    };

    let resolved_color_space = if let (Some(doc_mut), Object::Array(arr)) = (doc, &resolved_color_space) {
        if arr.len() > 1 {
            if let Some(second_ref) = arr[1].as_reference() {
                match doc_mut.load_object(second_ref) {
                    Ok(resolved_second) => {
                        let mut new_arr = arr.clone();
                        new_arr[1] = resolved_second;
                        Object::Array(new_arr)
                    }
                    _ => resolved_color_space,
                }
            } else {
                resolved_color_space
            }
        } else {
            resolved_color_space
        }
    } else {
        resolved_color_space
    };

    Ok(resolved_color_space)
}

/// An image XObject's `/Width`, `/Height` and effective `/BitsPerComponent`,
/// plus whether it is an `/ImageMask`. Split out of
/// [`resolve_image_xobject_metadata`] purely to keep that function shorter. ~keep
struct ImageDimensions {
    width: u32,
    height: u32,
    bits_per_component: u8,
    is_image_mask: bool,
}

/// Resolve `/Width`, `/Height` and `/BitsPerComponent` (each of which may be
/// an indirect reference per ISO 32000-1 §7.3.10) and whether `/ImageMask`
/// is set. See [`ImageDimensions`] for why this is split out.
fn resolve_image_dimensions(
    doc: Option<&crate::document::PdfDocument>,
    dict: &std::collections::HashMap<String, crate::object::Object>,
) -> Result<ImageDimensions> {
    use crate::object::Object;

    let resolve_int = |obj: &Object| -> Option<i64> {
        if let (Some(d), Some(r)) = (doc, obj.as_reference()) {
            d.load_object(r).ok().and_then(|o| o.as_integer())
        } else {
            obj.as_integer()
        }
    };

    let width = dict
        .get("Width")
        .and_then(resolve_int)
        .ok_or_else(|| Error::Image("Image missing /Width".to_string()))? as u32;

    let height = dict
        .get("Height")
        .and_then(resolve_int)
        .ok_or_else(|| Error::Image("Image missing /Height".to_string()))? as u32;

    let is_image_mask = dict
        .get("ImageMask")
        .is_some_and(|value| matches!(value, Object::Boolean(true)));
    // ImageMask defaults to one bit when /BitsPerComponent is omitted;
    // ordinary sampled images retain the historical eight-bit default. ~keep
    let default_bits_per_component = if is_image_mask { 1 } else { 8 };
    let bits_per_component = dict
        .get("BitsPerComponent")
        .and_then(resolve_int)
        .unwrap_or(default_bits_per_component) as u8;

    Ok(ImageDimensions {
        width,
        height,
        bits_per_component,
        is_image_mask,
    })
}

/// Which codec an image XObject's `/Filter` chain selects, plus the parsed
/// CCITT parameters when applicable. Split out of
/// [`resolve_image_xobject_metadata`] purely to keep that function shorter. ~keep
struct ImageFilterInfo {
    is_jbig2: bool,
    is_jpx: bool,
    is_jpeg_only: bool,
    is_jpeg_chain: bool,
    is_ccitt: bool,
    ccitt_params: Option<crate::decoders::CcittParams>,
}

/// Classify an image dictionary's `/Filter` chain and, for CCITT, parse its
/// `/DecodeParms`. See [`ImageFilterInfo`] for why this is split out.
fn resolve_image_filter_info(
    dict: &std::collections::HashMap<String, crate::object::Object>,
    width: u32,
    height: u32,
) -> ImageFilterInfo {
    use crate::object::Object;

    let filter_names = if let Some(filter_obj) = dict.get("Filter") {
        match filter_obj {
            Object::Name(name) => vec![name.clone()],
            Object::Array(filters) => filters.iter().filter_map(|f| f.as_name().map(String::from)).collect(),
            _ => vec![],
        }
    } else {
        vec![]
    };

    let has_dct = filter_names.iter().any(|name| name == "DCTDecode");
    let is_jpeg_only = has_dct && filter_names.len() == 1;
    let is_jpeg_chain = has_dct && filter_names.len() > 1;

    let is_jbig2 = filter_names.iter().any(|n| n.eq_ignore_ascii_case("JBIG2Decode"));

    let is_jpx = filter_names.iter().any(|n| n.eq_ignore_ascii_case("JPXDecode"));

    let is_ccitt = filter_names.iter().any(|n| n.eq_ignore_ascii_case("CCITTFaxDecode"));

    let ccitt_params = if is_ccitt {
        let ccitt_index = filter_names
            .iter()
            .position(|name| name.eq_ignore_ascii_case("CCITTFaxDecode"))
            .unwrap_or(0);
        let mut params =
            crate::object::extract_ccitt_params_with_width_at_index(dict.get("DecodeParms"), ccitt_index, Some(width))
                .unwrap_or(crate::decoders::CcittParams {
                    columns: width,
                    rows: Some(height),
                    ..crate::decoders::CcittParams::default()
                });
        if params.rows.is_none() {
            params.rows = Some(height);
        }
        // ISO 32000-1 7.4.6 /BlackIs1 and 8.9.5.2 Table 90 /Decode both
        // independently invert the bilevel sample polarity. Fold them into
        // the decoder's single polarity flag; two inversions cancel out.
        if decode_array_inverts_1bpc(dict.get("Decode")) {
            params.black_is_1 = !params.black_is_1;
        }
        Some(params)
    } else {
        None
    };

    ImageFilterInfo {
        is_jbig2,
        is_jpx,
        is_jpeg_only,
        is_jpeg_chain,
        is_ccitt,
        ccitt_params,
    }
}

/// Resolve an already-parsed color space's Indexed palette (if any) and its
/// direct or output-intent ICC profile (if any). Fails fast when the color
/// space is Indexed but its palette cannot be resolved, so the error points
/// at the real root cause instead of a downstream "Invalid RGB image
/// dimensions" symptom. Split out of [`resolve_image_xobject_metadata`]
/// purely to keep that function shorter. ~keep
fn resolve_indexed_and_icc_profile(
    doc: Option<&crate::document::PdfDocument>,
    color_space: ColorSpace,
    resolved_color_space: &crate::object::Object,
) -> Result<(
    Option<IndexedResolution>,
    Option<std::sync::Arc<crate::color::IccProfile>>,
)> {
    // For Indexed color spaces, resolve the base color space and palette now so we
    // can expand indices to RGB after decoding the stream. Without this, raw
    // Indexed pixel data (1 byte per pixel) is mislabelled as RGB (3 bytes per
    // pixel) and ImageBuffer::from_raw rejects the wrong length. ~keep
    let indexed_resolution: Option<IndexedResolution> = if color_space == ColorSpace::Indexed {
        let resolved = resolve_indexed_palette(doc, resolved_color_space)?;
        if resolved.is_none() {
            return Err(Error::Image(
                "Unable to resolve Indexed color space palette".to_string(),
            ));
        }
        resolved
    } else {
        None
    };

    // For a plain (non-Indexed) `[/ICCBased <stream>]` colour space,
    // capture the profile bytes so the CMM can convert through the
    // document's actual source characterisation instead of the
    // §10.3.5 additive-clamp fallback.
    //
    // When the image uses plain `/DeviceCMYK` with no ICC profile of
    // its own, fall back to the document's `/OutputIntents` CMYK
    // profile if one exists — the standard PDF/X assumption per
    // ISO 32000-1:2008 §14.11.5. ~keep
    let direct_icc_profile = if matches!(color_space, ColorSpace::ICCBased(_)) {
        resolve_icc_profile_from_obj(doc, resolved_color_space)
    } else if color_space == ColorSpace::DeviceCMYK {
        doc.and_then(|d| d.output_intent_cmyk_profile())
    } else {
        None
    };

    Ok((indexed_resolution, direct_icc_profile))
}

/// Metadata resolved from an image XObject dictionary before pixel decoding
/// begins: dimensions, color space (with any Indexed palette or ICC profile
/// already resolved), rendering intent, and which stream filter chain
/// applies. Split out of `extract_image_from_xobject` purely to keep that
/// function's decode branch -- which threads `stored_bpc`, `samples_are_raw`
/// and `decode_folded_in` through several codecs -- legible on its own. ~keep
struct ImageXObjectMetadata {
    width: u32,
    height: u32,
    bits_per_component: u8,
    color_space: ColorSpace,
    resolved_color_space: crate::object::Object,
    indexed_resolution: Option<IndexedResolution>,
    direct_icc_profile: Option<std::sync::Arc<crate::color::IccProfile>>,
    rendering_intent: crate::color::RenderingIntent,
    is_jbig2: bool,
    is_jpx: bool,
    /// Set when `color_space` above is a placeholder because the JPXDecode
    /// image named no `/ColorSpace` (see `resolve_image_xobject_metadata`).
    /// The caller must replace it with the colour space `decode_jpx_image`
    /// actually decoded before storing it on the returned `PdfImage`. ~keep
    jpx_color_space_is_placeholder: bool,
    is_jpeg_only: bool,
    is_jpeg_chain: bool,
    is_ccitt: bool,
    ccitt_params: Option<crate::decoders::CcittParams>,
}

/// Resolve dimensions, color space, ICC/rendering-intent metadata, and the
/// applicable stream filter chain for an image XObject dictionary. See
/// [`ImageXObjectMetadata`] for why this is split from the decode step.
fn resolve_image_xobject_metadata(
    doc: Option<&crate::document::PdfDocument>,
    dict: &std::collections::HashMap<String, crate::object::Object>,
    color_space_map: Option<&std::collections::HashMap<String, crate::object::Object>>,
) -> Result<ImageXObjectMetadata> {
    use crate::object::Object;

    let ImageDimensions {
        width,
        height,
        bits_per_component,
        is_image_mask,
    } = resolve_image_dimensions(doc, dict)?;

    let ImageFilterInfo {
        is_jbig2,
        is_jpx,
        is_jpeg_only,
        is_jpeg_chain,
        is_ccitt,
        ccitt_params,
    } = resolve_image_filter_info(dict, width, height);

    let default_mask_color_space = Object::Name("DeviceGray".to_string());
    // ISO 32000-1 Table 89 permits /ColorSpace to be omitted for a JPXDecode
    // image: the codestream carries its own colour space, matching the
    // precedent for /BitsPerComponent defaulting to 8 above. The placeholder
    // here is never read for pixel interpretation -- decode_jpx_image derives
    // the real component count from the codestream and the caller overwrites
    // `color_space` with it once decoding is done -- it only needs to parse
    // into a valid ColorSpace so the rest of this function can run. ~keep
    let default_jpx_color_space = Object::Name("DeviceRGB".to_string());
    let color_space_missing_ok_for_jpx = is_jpx && !is_image_mask;
    let color_space_obj = match dict.get("ColorSpace") {
        Some(color_space) => color_space,
        None if is_image_mask => &default_mask_color_space,
        None if color_space_missing_ok_for_jpx => &default_jpx_color_space,
        None => return Err(Error::Image("Image missing /ColorSpace".to_string())),
    };

    let resolved_color_space = resolve_image_color_space_object(doc, color_space_obj, color_space_map)?;

    let color_space = parse_color_space(&resolved_color_space)?;
    let (indexed_resolution, direct_icc_profile) =
        resolve_indexed_and_icc_profile(doc, color_space, &resolved_color_space)?;

    // Per §8.6.5.8, an image dictionary may override the graphics-state
    // rendering intent via `/Intent`. Unrecognised names fall through
    // to `RelativeColorimetric`. ~keep
    let rendering_intent = dict
        .get("Intent")
        .and_then(|obj| obj.as_name())
        .map(crate::color::RenderingIntent::from_pdf_name)
        .unwrap_or_default();

    Ok(ImageXObjectMetadata {
        width,
        height,
        bits_per_component,
        color_space,
        resolved_color_space,
        indexed_resolution,
        direct_icc_profile,
        rendering_intent,
        is_jbig2,
        is_jpx,
        jpx_color_space_is_placeholder: dict.get("ColorSpace").is_none() && color_space_missing_ok_for_jpx,
        is_jpeg_only,
        is_jpeg_chain,
        is_ccitt,
        ccitt_params,
    })
}

/// Extract an image from an XObject stream.
pub fn extract_image_from_xobject(
    doc: Option<&crate::document::PdfDocument>,
    xobject: &crate::object::Object,
    obj_ref: Option<ObjectRef>,
    color_space_map: Option<&std::collections::HashMap<String, crate::object::Object>>,
) -> Result<PdfImage> {
    #[cfg(test)]
    DECODE_CALL_COUNT.with(|count| count.set(count.get() + 1));

    let dict = xobject
        .as_dict()
        .ok_or_else(|| Error::Image("XObject is not a stream".to_string()))?;

    let subtype = dict
        .get("Subtype")
        .and_then(|obj| obj.as_name())
        .ok_or_else(|| Error::Image("XObject missing /Subtype".to_string()))?;

    if subtype != "Image" {
        return Err(Error::Image(format!("XObject subtype is not Image: {}", subtype)));
    }

    let ImageXObjectMetadata {
        mut width,
        mut height,
        bits_per_component,
        mut color_space,
        resolved_color_space,
        indexed_resolution,
        mut direct_icc_profile,
        rendering_intent,
        is_jbig2,
        is_jpx,
        jpx_color_space_is_placeholder,
        is_jpeg_only,
        is_jpeg_chain,
        is_ccitt,
        ccitt_params,
    } = resolve_image_xobject_metadata(doc, dict, color_space_map)?;

    // Bit depth of the buffer actually stored: raised to 8 when a transform
    // below (sub-byte unpack, /Decode application, 16-bit reduction) rewrites
    // the samples as one byte per component. ~keep
    let mut stored_bpc = bits_per_component;
    // Cleared when a transform below moves the stored samples out of the raw
    // sample space, so consumers that range-test them against dictionary
    // values (colour-key /Mask) can tell. ~keep
    let mut samples_are_raw = true;
    // Set only when a non-default /Decode is actually folded into the stored
    // samples. Tracked apart from `samples_are_raw` because most transforms
    // below leave the raw space without applying /Decode, and a consumer that
    // applies it itself (plate routing) must still do so for those. ~keep
    let mut decode_folded_in = false;
    // Populated only for an /Indexed image, whose raw index plane a colour-key
    // /Mask needs (GH#1899); see `raw_indexed_samples` field docs. ~keep
    let mut raw_indexed_samples: Option<Vec<u8>> = None;
    // A colour-key /Mask is tested against the raw samples, so they are kept wherever unpacking
    // below moves the stored samples out of that space. (GH#1904) ~keep
    let has_color_key_mask = matches!(dict.get("Mask"), Some(crate::object::Object::Array(_)));
    let mut color_key_samples = None;
    // With `/SMaskInData` 1 or 2 and no `/SMask`, the JPEG 2000 opacity channel is the image's soft
    // mask (ISO 32000-1 Table 89). A `/SMask` entry takes precedence. (GH#1902) ~keep
    let soft_mask_in_data = (is_jpx && dict.get("SMask").is_none())
        .then(|| {
            dict.get("SMaskInData").and_then(|obj| match (doc, obj.as_reference()) {
                (Some(d), Some(r)) => d.load_object(r).ok().and_then(|o| o.as_integer()),
                _ => obj.as_integer(),
            })
        })
        .flatten()
        .filter(|value| matches!(value, 1 | 2));
    let mut jpx_opacity = None;
    let data = if is_jbig2 {
        decode_jbig2_image(xobject, obj_ref, dict, doc, width, height)?
    } else if is_jpx {
        let JpxDecoded {
            data: jpx_data,
            width: jpx_width,
            height: jpx_height,
            opacity,
        } = decode_jpx_image(
            xobject,
            obj_ref,
            doc,
            &color_space,
            jpx_color_space_is_placeholder,
            (width, height),
            indexed_resolution.as_ref(),
        )?;
        // The codestream's size is authoritative for a /JPXDecode image (ISO 32000-1 §7.4.9,
        // GH#1900); decode_jpx_image already warned if it disagreed with the dictionary's
        // declared /Width and /Height. Adopt it unconditionally so the pixel buffer -- always
        // sized to the codestream -- matches the dimensions `PdfImage` reports. ~keep
        width = jpx_width;
        height = jpx_height;
        jpx_opacity = opacity;
        // An `/Indexed` image decodes to one index byte per pixel, looked up in the
        // dictionary's palette exactly as any other codec's index stream is below. (GH#1885) ~keep
        if let Some(ir) = indexed_resolution.as_ref()
            && let ImageData::Raw { pixels: indices, .. } = &jpx_data
        {
            samples_are_raw = false;
            stored_bpc = 8;
            // A colour-key /Mask on an /Indexed image states its range in these indices, not in
            // the RGB below. (GH#1885) ~keep
            if has_color_key_mask {
                raw_indexed_samples = Some(indices.clone());
            }
            ImageData::Raw {
                pixels: expand_indexed_image(indices, ir, width, height, 8, rendering_intent)?,
                format: PixelFormat::RGB,
            }
        } else {
            // The placeholder colour space set above (dict named no /ColorSpace)
            // was never the real one; replace it with what the codestream
            // actually decoded to, so downstream consumers of `color_space` (the
            // colour-key /Mask component count, in particular) see the real
            // component count rather than the placeholder's. ~keep
            if jpx_color_space_is_placeholder && let ImageData::Raw { format, .. } = &jpx_data {
                color_space = match format {
                    PixelFormat::Grayscale => ColorSpace::DeviceGray,
                    PixelFormat::RGB => ColorSpace::DeviceRGB,
                    PixelFormat::CMYK => ColorSpace::DeviceCMYK,
                };
                // The ICC decision above ran against the placeholder, and `/DeviceRGB` takes
                // neither its ICCBased arm nor its DeviceCMYK one -- so the §14.11.5 OutputIntent
                // fallback was never consulted for a codestream that turns out to be
                // four-component, and the image reached its consumers with no profile at all.
                // Redo that one decision now the real colour space is known. Nothing is
                // overwritten: the placeholder provably yields `None`. (GH#1839) ~keep
                if color_space == ColorSpace::DeviceCMYK {
                    direct_icc_profile = doc.and_then(|d| d.output_intent_cmyk_profile());
                }
            }
            jpx_data
        }
    } else if is_jpeg_only || is_jpeg_chain {
        let decoded = if let (Some(d), Some(ref_id)) = (doc.as_ref(), obj_ref) {
            d.decode_stream_with_encryption(xobject, ref_id)?
        } else {
            xobject.decode_stream_data()?
        };
        ImageData::Jpeg(decoded)
    } else if let Some(params) = ccitt_params.as_ref() {
        let compressed = if let (Some(d), Some(ref_id)) = (doc.as_ref(), obj_ref) {
            d.decode_stream_with_encryption(xobject, ref_id)?
        } else {
            xobject.decode_stream_data()?
        };
        let packed = ccitt_bilevel::decompress_ccitt(&compressed, params)?;
        let pixels = ccitt_bilevel::bilevel_to_grayscale(&packed, width, height);
        let expected_len = (width as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| Error::Decode(format!("CCITT image dimensions overflow: {width} x {height}")))?;
        if pixels.len() != expected_len {
            return Err(Error::Decode(format!(
                "CCITT image decoded to {} grayscale samples, expected {expected_len} for {width} x {height}",
                pixels.len()
            )));
        }
        stored_bpc = 8;
        samples_are_raw = false;
        decode_folded_in = decode_array_inverts_1bpc(dict.get("Decode"));
        ImageData::Raw {
            pixels,
            format: PixelFormat::Grayscale,
        }
    } else {
        let expected_filter_output_size =
            expected_image_filter_output_size(dict, width, height, color_space.components(), bits_per_component)?;
        let decoded_data = decode_dimension_bounded_image_stream(doc, xobject, obj_ref, expected_filter_output_size)?;
        if let Some(ir) = indexed_resolution.as_ref() {
            let expanded =
                expand_indexed_image(&decoded_data, ir, width, height, bits_per_component, rendering_intent)?;
            // Palette lookup replaces index samples with 8-bit RGB, so the buffer
            // is no longer in the space or the depth the dictionary describes. ~keep
            samples_are_raw = false;
            stored_bpc = 8;
            // A colour-key /Mask on this image states its range in the raw
            // index space, not the RGB above, so keep it too. (GH#1899) ~keep
            raw_indexed_samples = Some(unpack_indexed_indices(
                &decoded_data,
                width,
                height,
                bits_per_component,
            )?);
            ImageData::Raw {
                pixels: expanded,
                format: PixelFormat::RGB,
            }
        } else {
            let pixel_format = color_space_to_pixel_format(&color_space);
            // ISO 32000-1 §8.9.5.2: BitsPerComponent 16 stores each colour
            // sample as a big-endian 16-bit value. The rest of the image
            // pipeline (and the render target, an 8-bit tiny_skia pixmap)
            // assumes 8-bit samples, so reduce each sample to 8 bits. Without
            // this the raw buffer is twice the expected length; the lenient
            // `ImageBuffer::from_raw` then builds an oversized image whose
            // buffer the PNG encoder rejects with a panic (`assertion
            // left == right failed: Invalid buffer length`).
            //
            // The reduction rounds `v * 255 / 65535` to the nearest 8-bit
            // value rather than dropping the low byte (`v >> 8`, i.e. floor):
            // truncation biases every sample downward by up to ~1 LSB, most
            // visibly darkening near-white highlights. Full u16 precision
            // through extraction is deferred — no current consumer
            // benefits, as both the PNG path and the rasteriser are 8-bit. ~keep
            let reduced: Vec<u8> = if bits_per_component == 16 {
                // Rescaling 0..65535 to 0..255 leaves the raw sample space
                // exactly as the sub-byte unpack does. The flag clears at the
                // `effective_bpc` check below, which owns the stored-versus-
                // declared depth rule for every codec. A colour-key /Mask
                // states its bounds in the file's 0..65535 space and must not
                // be range-tested against these bytes. ~keep
                decoded_data
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|sample| reduce_16_to_8(sample[0], sample[1]))
                    .collect()
            } else {
                decoded_data
            };
            let bpc_after_reduce = if bits_per_component == 16 {
                8
            } else {
                bits_per_component
            };

            // `ColorSpace::DeviceN` does not carry its ink count — the parser
            // discards the name array and `components()` answers a flat 4 — so
            // read the real count here. It sets the unpacker's row geometry:
            // with the wrong value every row starts at the wrong offset, and a
            // stream sized for its true ink count runs out partway down, so
            // the tail of the image is padded with fabricated zeros. ~keep
            let ncomp = devicen_ink_count(&resolved_color_space).unwrap_or_else(|| color_space.components());
            // The stored buffer is one byte per component at the stride
            // `PixelFormat` declares, and that stride is only ever 1, 3 or 4.
            // A colour space whose component count is anything else — a
            // 2- or 6-ink `/DeviceN`, an `/ICCBased` with N ∉ {1,3,4} — cannot
            // be expressed in it, so unpacking would have to write a buffer
            // whose length contradicts its own format. Leave the stream packed
            // instead: `bits_per_component` stays sub-byte, which is exactly
            // what every consumer already checks before reading samples. ~keep
            let representable = ncomp == pixel_format.bytes_per_pixel();
            // Lab samples are not confined to [0, 1], so the linear-to-byte
            // mapping does not apply to them. Unrepresentable multi-ink
            // colour spaces likewise stay packed. CCITT has already taken
            // the eager bilevel-to-grayscale branch above. ~keep
            let keep_raw = color_space == ColorSpace::Lab || !representable;
            let ranges =
                decode_ranges(dict.get("Decode"), ncomp).filter(|r| r.iter().any(|&(lo, hi)| (lo, hi) != (0.0, 1.0)));

            // The stored buffer contract is one 8-bit byte per component
            // (`PixelFormat::bytes_per_pixel`), so sub-byte samples are
            // unpacked here and a non-default /Decode is applied at
            // 1/2/4/8 bpc on this raw-sample path (ISO 32000-1 §8.9.5.2).
            // DCT- and JPX-coded images keep their encoded stream and do
            // not pass through here, so /Decode is not applied to them. ~keep
            let unpacked = if keep_raw || (bpc_after_reduce == 8 && ranges.is_none()) {
                None
            } else {
                samples_to_decoded_bytes(&reduced, width, height, ncomp, bpc_after_reduce, ranges.as_deref())
            };
            let pixels = match unpacked {
                Some(samples) => {
                    stored_bpc = 8;
                    // This arm runs only when the buffer was rewritten — the
                    // identity case (8 bpc, no /Decode) took the `None` branch
                    // above — so the samples have left the raw space either by
                    // the ×255/×85/×17 sub-byte rescale or by /Decode itself.
                    // Colour-key /Mask (§8.9.6.4) range-tests against bounds
                    // stated in the original 0..2^bpc−1 space and reads this. ~keep
                    samples_are_raw = false;
                    // Plate routing applies /Decode itself, so it needs the
                    // narrower fact: only `ranges` actually folds one in. ~keep
                    decode_folded_in = ranges.is_some();
                    // A 16-bit sample does not fit the kept byte plane, so its mask stays skipped. ~keep
                    if has_color_key_mask && matches!(bits_per_component, 1 | 2 | 4 | 8) {
                        color_key_samples =
                            Some(unpack_raw_samples(&reduced, width, height, ncomp, bits_per_component));
                    }
                    samples
                }
                None => reduced,
            };
            ImageData::Raw {
                pixels,
                format: pixel_format,
            }
        }
    };

    // JBIG2 and CCITT decode produce 8-bit-per-channel pixels regardless of
    // the XObject's BitsPerComponent (which is 1). 16-bit samples were
    // collapsed to their high byte above (and an Indexed base always expands
    // to 8-bit RGB), so the stored pixels are 8-bit per component in those
    // cases too. ~keep
    let stored_depth = if is_jbig2 || is_ccitt || bits_per_component == 16 {
        8
    } else {
        stored_bpc
    };
    // A stored depth that no longer matches the declared /BitsPerComponent
    // means the samples left the space the dictionary describes: the 16-bit
    // reduce and the JBIG2 decoder's 8-bit output both land here. Cleared
    // centrally so a codec expansion cannot forget the flag — the JBIG2
    // branch did, and reported raw samples at 8 bpc against a declared 1. ~keep
    if i64::from(stored_depth) != i64::from(bits_per_component) {
        samples_are_raw = false;
    }
    // A JPEG 2000 decode yields 8-bit samples, and ISO 32000-1 Table 89 says a
    // /JPXDecode image's /BitsPerComponent is ignored, so the reported depth is
    // 8 whatever the dictionary declares (GH#1901). This is not a move out of
    // the raw sample space, so it does not take part in the check above. ~keep
    let effective_bpc = if is_jpx { 8 } else { stored_depth };
    let mut image = PdfImage::new(width, height, color_space, effective_bpc, data);
    image.set_samples_are_raw(samples_are_raw);
    image.set_decode_folded_in(decode_folded_in);
    if let Some(indices) = raw_indexed_samples {
        image.set_raw_indexed_samples(indices);
    }
    image.color_key_samples = color_key_samples;
    image.soft_mask_in_data = jpx_opacity
        .zip(soft_mask_in_data)
        .map(|(samples, value)| JpxSoftMaskInData {
            samples,
            premultiplied: value == 2,
        });

    // Attach the ICC profile if we found one — prefer the direct ICCBased
    // profile, then fall back to an Indexed base's profile so the CMM has
    // something to work with for palette-backed CMYK/Lab images too. ~keep
    if let Some(p) = direct_icc_profile {
        image.set_icc_profile(p);
    } else if let Some(ir) = indexed_resolution.as_ref()
        && let Some(p) = ir.base_profile.clone()
    {
        image.set_icc_profile(p);
    }
    image.set_rendering_intent(rendering_intent);

    if let Some(ccitt_params) = ccitt_params {
        image.set_ccitt_params(ccitt_params);
    }

    Ok(image)
}

/// Extract and parse an `ICCBased` colour-space's profile stream.
///
/// Accepts either a fully-resolved `[/ICCBased <Stream>]` array (the
/// stream is an `Object::Stream` directly), or a `[/ICCBased <Ref>]`
/// array where the second element is a live reference — in that case
/// `doc` must be supplied so we can dereference.
///
/// Returns `None` if:
///   - `cs_obj` isn't an ICCBased array,
///   - the profile stream can't be decoded,
///   - the profile bytes fail ICC header validation, or
///   - the declared `/N` disagrees with the profile header's
///     colourSpace signature (PDF §8.6.5.5 mandates they match).
///
/// No error is returned — callers treat "no profile" as "fall back to
/// device colour space" per §8.6.5.5's /Alternate clause.
pub(crate) fn resolve_icc_profile_from_obj(
    doc: Option<&crate::document::PdfDocument>,
    cs_obj: &crate::object::Object,
) -> Option<std::sync::Arc<crate::color::IccProfile>> {
    use crate::object::Object;

    let Object::Array(arr) = cs_obj else {
        return None;
    };
    if arr.len() < 2 || arr[0].as_name() != Some("ICCBased") {
        return None;
    }

    let profile_obj = match (&arr[1], doc) {
        (Object::Stream { .. }, _) => arr[1].clone(),
        (Object::Reference(r), Some(d)) => match d.load_object(*r) {
            Ok(obj) => obj,
            Err(_) => return None,
        },
        _ => return None,
    };

    let Object::Stream { dict, .. } = &profile_obj else {
        return None;
    };
    // `N` is mandatory per PDF 32000-1 §8.6.5.5 Table 66. ~keep
    let n = dict
        .get("N")
        .and_then(|obj| obj.as_integer())
        .filter(|n| matches!(*n, 1 | 3 | 4))? as u8;

    let bytes = profile_obj.decode_stream_data().ok()?;
    let profile = crate::color::IccProfile::parse(bytes, n)?;
    Some(std::sync::Arc::new(profile))
}

/// Outcome of resolving an `[/Indexed base hival lookup]` colour space:
/// the palette in the base's pixel format, plus the base's ICC profile
/// when the base is `ICCBased`.
pub(crate) struct IndexedResolution {
    pub base_fmt: PixelFormat,
    /// The bytes in each palette entry: the base's component count, which for a `/DeviceN` base is
    /// its colorant count rather than the four `base_fmt` holds. (GH#1913) ~keep
    pub components: usize,
    pub palette: Vec<u8>,
    /// `None` for device-dependent bases or bases we already folded
    /// colourimetrically (e.g. Lab, whose palette is rewritten to RGB
    /// before being returned).
    pub base_profile: Option<std::sync::Arc<crate::color::IccProfile>>,
}

impl IndexedResolution {
    /// The last index the palette holds an entry for: `hival`, or lower when the lookup is short.
    /// The expander paints any index past it black.
    fn highest_index(&self) -> u8 {
        let entries = self.palette.len() / self.components;
        u8::try_from(entries.saturating_sub(1)).unwrap_or(u8::MAX)
    }
}

/// Resolve an Indexed color space's base color space and palette lookup bytes.
///
/// PDF Indexed color spaces are `[/Indexed base hival lookup]` where `lookup`
/// is either a byte string or a stream of `(hival + 1) * N` bytes (N = number
/// of components in the base color space).
fn resolve_indexed_palette(
    doc: Option<&crate::document::PdfDocument>,
    cs_obj: &crate::object::Object,
) -> Result<Option<IndexedResolution>> {
    use crate::object::Object;

    let Object::Array(arr) = cs_obj else {
        return Ok(None);
    };
    if arr.len() < 4 {
        return Ok(None);
    }

    let base = resolve_indexed_base(doc, arr)?;
    let hival = resolve_indexed_hival(doc, arr)?;
    let n = devicen_ink_count(&base.base_obj).unwrap_or_else(|| base.base_fmt.bytes_per_pixel());
    let Some(palette_bytes) = resolve_indexed_palette_bytes(doc, arr, hival, n)? else {
        return Ok(None);
    };

    // Device-independent colour-space palettes must be converted to
    // RGB before being handed to the expander, which assumes palette
    // bytes are already in the output colour space. Without this step
    // Lab triples are mis-interpreted as raw RGB and render with
    // perceptually wrong colours. ~keep
    if matches!(base.base_cs, ColorSpace::Lab) {
        let white = extract_lab_whitepoint(&base.base_obj);
        let rgb_palette = lab_palette_to_rgb(&palette_bytes, white);
        // Lab palettes are now RGB; no base ICC profile to carry through. ~keep
        return Ok(Some(IndexedResolution {
            base_fmt: PixelFormat::RGB,
            components: 3,
            palette: rgb_palette,
            base_profile: None,
        }));
    }

    Ok(Some(IndexedResolution {
        base_fmt: base.base_fmt,
        components: n,
        palette: palette_bytes,
        base_profile: base.base_profile,
    }))
}

/// The base color space resolved from an `[/Indexed base hival lookup]`
/// array: its parsed [`ColorSpace`]/[`PixelFormat`], the resolved base object
/// (needed again for a Lab whitepoint lookup), and its ICC profile when the
/// base is `/ICCBased`.
struct IndexedBase {
    base_obj: crate::object::Object,
    base_cs: ColorSpace,
    base_fmt: PixelFormat,
    base_profile: Option<std::sync::Arc<crate::color::IccProfile>>,
}

/// Resolve `arr[1]`, the base color space of an `[/Indexed base hival
/// lookup]` array. See [`resolve_indexed_palette`].
fn resolve_indexed_base(
    doc: Option<&crate::document::PdfDocument>,
    arr: &[crate::object::Object],
) -> Result<IndexedBase> {
    use crate::object::Object;

    // Resolve the base color-space object. When it's an array like
    // [/ICCBased <stream_ref>], resolve inner references so
    // parse_color_space can read /N from the ICC stream dict. ~keep
    let base_obj = if let Some(d) = doc {
        let outer = if let Some(r) = arr[1].as_reference() {
            d.load_object(r)?
        } else {
            arr[1].clone()
        };
        if let Object::Array(mut inner) = outer {
            for item in inner.iter_mut() {
                if let Some(r) = item.as_reference()
                    && let Ok(resolved) = d.load_object(r)
                {
                    *item = resolved;
                }
            }
            Object::Array(inner)
        } else {
            outer
        }
    } else {
        arr[1].clone()
    };
    let base_cs = parse_color_space(&base_obj)?;
    let base_fmt = color_space_to_pixel_format(&base_cs);

    // When the base is `/ICCBased`, capture the profile bytes so the
    // extractor can later hand them to a CMM. Parse failures reduce to
    // `None` — the decoder then falls back to §10.3.5 CMYK→RGB math as
    // if no profile were present. ~keep
    let base_profile = if matches!(base_cs, ColorSpace::ICCBased(_)) {
        resolve_icc_profile_from_obj(doc, &base_obj)
    } else {
        None
    };

    Ok(IndexedBase {
        base_obj,
        base_cs,
        base_fmt,
        base_profile,
    })
}

/// Resolve `arr[2]`, the `hival` bound on valid palette indices, from an
/// `[/Indexed base hival lookup]` array. Invalid or missing values resolve
/// to `None` ("unknown"), which skips the palette-truncation step. See
/// [`resolve_indexed_palette`].
fn resolve_indexed_hival(
    doc: Option<&crate::document::PdfDocument>,
    arr: &[crate::object::Object],
) -> Result<Option<usize>> {
    let hival_obj = if let Some(d) = doc {
        if let Some(r) = arr[2].as_reference() {
            d.load_object(r)?
        } else {
            arr[2].clone()
        }
    } else {
        arr[2].clone()
    };
    Ok(hival_obj
        .as_integer()
        .and_then(|i| if (0..=255).contains(&i) { Some(i as usize) } else { None }))
}

/// Resolve `arr[3]`, the palette lookup bytes, from an `[/Indexed base hival
/// lookup]` array, truncated to the logical length implied by `hival`. `None`
/// when the lookup is neither a string nor a stream, or decodes empty. See
/// [`resolve_indexed_palette`].
fn resolve_indexed_palette_bytes(
    doc: Option<&crate::document::PdfDocument>,
    arr: &[crate::object::Object],
    hival: Option<usize>,
    n: usize,
) -> Result<Option<Vec<u8>>> {
    use crate::object::Object;

    let lookup_obj = if let Some(d) = doc {
        if let Some(r) = arr[3].as_reference() {
            d.load_object(r)?
        } else {
            arr[3].clone()
        }
    } else {
        arr[3].clone()
    };
    let mut palette_bytes = match &lookup_obj {
        Object::String(s) => s.clone(),
        Object::Stream { .. } => lookup_obj.decode_stream_data()?,
        _ => return Ok(None),
    };
    if palette_bytes.is_empty() {
        return Ok(None);
    }

    // Truncate palette to the logical length implied by hival so that indices
    // greater than hival fall into the out-of-range branch of the expander.
    // Per PDF 32000-1:2008 §8.6.6.3 the lookup is exactly (hival + 1) * N bytes;
    // anything beyond that is stray data that must not be mapped to pixels. ~keep
    if let Some(h) = hival {
        let expected = (h + 1).saturating_mul(n);
        if expected > 0 && palette_bytes.len() > expected {
            palette_bytes.truncate(expected);
        }
    }

    Ok(Some(palette_bytes))
}

/// Reduce a big-endian 16-bit colour sample (`hi`, `lo` bytes) to 8 bits,
/// rounding `v * 255 / 65535` to the nearest value. Preferred over the crude
/// high-byte drop (`v >> 8`), which floors and biases every sample downward by
/// up to ~1 LSB. `v == 0xFFFF` maps to `255` and `v == 0` to `0` exactly.
#[inline]
fn reduce_16_to_8(hi: u8, lo: u8) -> u8 {
    let v = u16::from_be_bytes([hi, lo]) as u32;
    ((v * 255 + 32_767) / 65_535) as u8
}

/// Expand packed Indexed image indices into RGB bytes using the palette.
///
/// Supports 1, 2, 4, and 8 bit-per-component index streams. Rows are padded
/// to byte boundaries per the PDF spec.
///
/// Returns `Err(Error::Image)` when the requested dimensions would require
/// more than `MAX_INDEXED_OUTPUT_BYTES` to decode, or when the `usize`
/// arithmetic on `width * height * channels` / `width * bpc` overflows,
/// or when the input `raw` buffer is too short to supply every row of the
/// requested height. This is an input-amplification guard for maliciously
/// crafted PDFs that pair tiny streams with extreme Indexed image
/// dimensions.
#[cfg(test)]
fn expand_indexed_to_rgb(
    raw: &[u8],
    palette: &[u8],
    base_fmt: PixelFormat,
    width: u32,
    height: u32,
    bpc: u8,
) -> Result<Vec<u8>> {
    expand_indexed_to_rgb_with_transform(IndexedExpandParams {
        raw,
        palette,
        base_fmt,
        components: base_fmt.bytes_per_pixel(),
        width,
        height,
        bpc,
        transform: None,
    })
}

/// Parameters for [`expand_indexed_to_rgb_with_transform`]: an Indexed
/// image's packed index stream, its resolved palette, and how to interpret
/// both. Grouped into a struct purely to keep the entry function's own
/// parameter list short; each field is exactly one of the prior positional
/// arguments. ~keep
struct IndexedExpandParams<'a> {
    raw: &'a [u8],
    palette: &'a [u8],
    base_fmt: PixelFormat,
    /// Bytes per palette entry; see [`IndexedResolution::components`].
    components: usize,
    width: u32,
    height: u32,
    bpc: u8,
    transform: Option<&'a crate::color::Transform>,
}

/// Expand an `/Indexed` image's index samples to 8-bit RGB through its resolved palette, routing
/// CMYK palette entries through the base colour space's ICC profile when it has one.
fn expand_indexed_image(
    raw: &[u8],
    ir: &IndexedResolution,
    width: u32,
    height: u32,
    bpc: u8,
    rendering_intent: crate::color::RenderingIntent,
) -> Result<Vec<u8>> {
    let transform = ir
        .base_profile
        .clone()
        .map(|p| crate::color::Transform::new_srgb_target(p, rendering_intent));
    expand_indexed_to_rgb_with_transform(IndexedExpandParams {
        raw,
        palette: &ir.palette,
        base_fmt: ir.base_fmt,
        components: ir.components,
        width,
        height,
        bpc,
        transform: transform.as_ref(),
    })
}

/// Like [`expand_indexed_to_rgb`] but routes CMYK palette entries
/// through an ICC transform when one is supplied. Used during image
/// extraction when the base colour space is `/ICCBased` with N=4.
fn expand_indexed_to_rgb_with_transform(params: IndexedExpandParams<'_>) -> Result<Vec<u8>> {
    let bpc = params.bpc;

    // ISO 32000-2 §8.9.5.1 mandates bpc ∈ {1, 2, 4, 8} for Indexed color
    // spaces. Anything else (0, 3, 5, 6, 7, 9, 12, 16, …) used to be
    // accepted silently — bpc=0 was coerced to 1 and any other value fell
    // through the `read_index` `_ => 0` arm, producing a solid palette-
    // entry-0 image with no error. Reject up front so malformed input is
    // surfaced instead of decoded into nonsense pixels. ~keep
    if !matches!(bpc, 1 | 2 | 4 | 8) {
        return Err(Error::Image(format!(
            "Indexed image has invalid /BitsPerComponent {bpc} \
             (PDF spec requires 1, 2, 4, or 8)"
        )));
    }

    let w = params.width as usize;
    let h = params.height as usize;
    let n = params.components;
    let (bytes_per_row, output_bytes) = indexed_expand_geometry(w, h, bpc, 3)?;
    validate_indexed_input_len(params.raw, bytes_per_row, h)?;

    let mut out = Vec::with_capacity(output_bytes);
    decode_indexed_pixels(&params, w, h, bytes_per_row, n, &mut out);
    Ok(out)
}

/// Row-byte stride for an Indexed image of size `w × h` at `bpc` bits per
/// index, and the total output size for `n` bytes per decoded pixel, with the
/// same overflow and 256 MiB output-size guard [`expand_indexed_to_rgb_with_transform`]
/// always applied. `n` is 3 for the RGB expansion; [`expand_indexed_to_base_samples`]
/// passes its base colour space's own byte count instead. (GH#1898)
fn indexed_expand_geometry(w: usize, h: usize, bpc: u8, n: usize) -> Result<(usize, usize)> {
    /// Hard cap on the decoded output buffer size (256 MiB). Legitimate
    /// Indexed images in real PDFs are several orders of magnitude below
    /// this — the cap only fires on pathological / adversarial inputs
    /// where `width * height` is billions of pixels.
    const MAX_INDEXED_OUTPUT_BYTES: usize = 256 * 1024 * 1024;

    let bytes_per_row = w.checked_mul(bpc as usize).map(|v| v.div_ceil(8)).ok_or_else(|| {
        Error::Image(format!(
            "Indexed image row width overflow: {w} × {bpc} bpc exceeds usize"
        ))
    })?;

    let output_bytes = w.checked_mul(h).and_then(|v| v.checked_mul(n)).ok_or_else(|| {
        Error::Image(format!(
            "Indexed image output size overflow: {w} × {h} × {n} exceeds usize"
        ))
    })?;

    if output_bytes > MAX_INDEXED_OUTPUT_BYTES {
        return Err(Error::Image(format!(
            "Indexed image decode would produce {output_bytes} bytes, \
             exceeds guard limit of {MAX_INDEXED_OUTPUT_BYTES} bytes \
             (width={w}, height={h})"
        )));
    }

    Ok((bytes_per_row, output_bytes))
}

/// Reject an Indexed index stream too short to cover every row of `h` rows
/// at `bytes_per_row` bytes each. Truncated streams used to be silently
/// zero-padded, which lets a malicious PDF pair a 10-byte stream with a
/// 10 000 × 10 000 image and force a ~300 MiB allocation filled with default
/// palette entry 0. ~keep
fn validate_indexed_input_len(raw: &[u8], bytes_per_row: usize, h: usize) -> Result<()> {
    let required_bytes = bytes_per_row.checked_mul(h).ok_or_else(|| {
        Error::Image(format!(
            "Indexed image required-input size overflow: {bytes_per_row} × {h} exceeds usize"
        ))
    })?;
    if raw.len() < required_bytes {
        return Err(Error::Image(format!(
            "Indexed image index stream truncated: {} bytes available, \
             {} required ({} bytes/row × {} rows)",
            raw.len(),
            required_bytes,
            bytes_per_row,
            h
        )));
    }
    Ok(())
}

/// One byte per sample from a packed sample stream of `ncomp` components at `bpc` bits (1, 2, 4
/// or 8), with each row starting on a byte boundary. A short stream reads as zeros, as the
/// unpacked samples do. Called only after the caller's own unpack accepted the geometry.
fn unpack_raw_samples(raw: &[u8], width: u32, height: u32, ncomp: usize, bpc: u8) -> Vec<u8> {
    let samples_per_row = width as usize * ncomp;
    let bytes_per_row = (samples_per_row * usize::from(bpc)).div_ceil(8);
    let mut out = Vec::with_capacity(samples_per_row * height as usize);
    for y in 0..height as usize {
        let row = raw.get(y * bytes_per_row..).unwrap_or(&[]);
        let row = &row[..bytes_per_row.min(row.len())];
        out.extend((0..samples_per_row).map(|i| read_indexed_pixel(row, i, bpc) as u8));
    }
    out
}

/// Read one packed Indexed pixel from `row` at column `x`, given index depth
/// `bpc` (validated to be in {1, 2, 4, 8} by the caller).
fn read_indexed_pixel(row: &[u8], x: usize, bpc: u8) -> usize {
    match bpc {
        8 => row.get(x).copied().unwrap_or(0) as usize,
        4 => {
            let byte_idx = x / 2;
            let b = row.get(byte_idx).copied().unwrap_or(0);
            if x.is_multiple_of(2) {
                (b >> 4) as usize
            } else {
                (b & 0x0F) as usize
            }
        }
        2 => {
            let byte_idx = x / 4;
            let b = row.get(byte_idx).copied().unwrap_or(0);
            let shift = 6 - (x % 4) * 2;
            ((b >> shift) & 0x03) as usize
        }
        1 => {
            let byte_idx = x / 8;
            let b = row.get(byte_idx).copied().unwrap_or(0);
            let shift = 7 - (x % 8);
            ((b >> shift) & 0x01) as usize
        }
        // Unreachable: bpc is validated to be in {1, 2, 4, 8} above
        // before this is called, so this arm only exists to satisfy
        // exhaustiveness on `u8`. ~keep
        _ => unreachable!("bpc validated to {{1,2,4,8}} before read_indexed_pixel"),
    }
}

/// Fill `out` with RGB bytes for every pixel of an Indexed image, mapping
/// each packed index through `params.palette`.
fn decode_indexed_pixels(
    params: &IndexedExpandParams<'_>,
    w: usize,
    h: usize,
    bytes_per_row: usize,
    n: usize,
    out: &mut Vec<u8>,
) {
    for y in 0..h {
        let row_start = y * bytes_per_row;
        let row_end = (row_start + bytes_per_row).min(params.raw.len());
        let row: &[u8] = if row_start < params.raw.len() {
            &params.raw[row_start..row_end]
        } else {
            &[]
        };
        for x in 0..w {
            let idx = read_indexed_pixel(row, x, params.bpc);
            let off = idx * n;
            if off + n > params.palette.len() {
                out.extend_from_slice(&[0, 0, 0]);
                continue;
            }
            match params.base_fmt {
                PixelFormat::RGB => out.extend_from_slice(&params.palette[off..off + 3]),
                PixelFormat::Grayscale => {
                    let g = params.palette[off];
                    out.push(g);
                    out.push(g);
                    out.push(g);
                }
                PixelFormat::CMYK => {
                    // A `/DeviceN` entry with other than four colorants is read as CMYK from its
                    // first four, a missing one taken as no ink. (GH#1913) ~keep
                    let ink = |i: usize| if i < n { params.palette[off + i] } else { 0 };
                    let (c, m, y_c, k) = (ink(0), ink(1), ink(2), ink(3));
                    let [r, g, b] = if let Some(t) = params.transform {
                        t.convert_cmyk_pixel(c, m, y_c, k)
                    } else {
                        cmyk_pixel_to_rgb(c, m, y_c, k)
                    };
                    out.push(r);
                    out.push(g);
                    out.push(b);
                }
            }
        }
    }
}

/// Unpack an Indexed image's packed index stream to one byte per pixel
/// (values `0..=2^bpc-1`), with no palette lookup. Used for colour-key
/// `/Mask` on an Indexed image, whose range is stated in this raw index space
/// rather than the palette-expanded RGB `expand_indexed_to_rgb_with_transform`
/// produces for display (ISO 32000-1 §8.9.6.4). (GH#1899)
fn unpack_indexed_indices(raw: &[u8], width: u32, height: u32, bpc: u8) -> Result<Vec<u8>> {
    if !matches!(bpc, 1 | 2 | 4 | 8) {
        return Err(Error::Image(format!(
            "Indexed image has invalid /BitsPerComponent {bpc} \
             (PDF spec requires 1, 2, 4, or 8)"
        )));
    }
    let w = width as usize;
    let h = height as usize;
    let (bytes_per_row, output_bytes) = indexed_expand_geometry(w, h, bpc, 1)?;
    validate_indexed_input_len(raw, bytes_per_row, h)?;

    let mut out = Vec::with_capacity(output_bytes);
    for y in 0..h {
        let row_start = y * bytes_per_row;
        let row_end = (row_start + bytes_per_row).min(raw.len());
        let row: &[u8] = if row_start < raw.len() {
            &raw[row_start..row_end]
        } else {
            &[]
        };
        for x in 0..w {
            out.push(read_indexed_pixel(row, x, bpc) as u8);
        }
    }
    Ok(out)
}

/// Expand an Indexed image's packed index stream into its base colour
/// space's own bytes per pixel — a straight palette lookup with no RGB
/// conversion — for separation-plate routing. `extract_image_from_xobject`
/// always expands an Indexed image to RGB for on-screen display, which would
/// misattribute a CMYK/Separation/DeviceN base's ink if handed to plate
/// routing directly (GH#1898). Shares bpc validation, row geometry, and
/// out-of-range handling with [`expand_indexed_to_rgb_with_transform`]; the
/// only difference is the per-pixel output width (`components`, the bytes in
/// one palette entry, instead of a fixed 3) and that the looked-up bytes are copied verbatim.
pub(crate) fn expand_indexed_to_base_samples(
    raw: &[u8],
    palette: &[u8],
    components: usize,
    width: u32,
    height: u32,
    bpc: u8,
) -> Result<Vec<u8>> {
    if !matches!(bpc, 1 | 2 | 4 | 8) {
        return Err(Error::Image(format!(
            "Indexed image has invalid /BitsPerComponent {bpc} \
             (PDF spec requires 1, 2, 4, or 8)"
        )));
    }
    let w = width as usize;
    let h = height as usize;
    let n = components;
    let (bytes_per_row, output_bytes) = indexed_expand_geometry(w, h, bpc, n)?;
    validate_indexed_input_len(raw, bytes_per_row, h)?;

    let mut out = Vec::with_capacity(output_bytes);
    for y in 0..h {
        let row_start = y * bytes_per_row;
        let row_end = (row_start + bytes_per_row).min(raw.len());
        let row: &[u8] = if row_start < raw.len() {
            &raw[row_start..row_end]
        } else {
            &[]
        };
        for x in 0..w {
            let idx = read_indexed_pixel(row, x, bpc);
            let off = idx * n;
            if off + n > palette.len() {
                out.resize(out.len() + n, 0);
                continue;
            }
            out.extend_from_slice(&palette[off..off + n]);
        }
    }
    Ok(out)
}

/// An `/Indexed` image XObject's palette-expanded samples in its BASE colour
/// space (e.g. raw C/M/Y/K bytes for an `/Indexed /DeviceCMYK` image), for a
/// caller that routes ink to separation plates rather than rendering to
/// screen.
pub(crate) struct IndexedBaseSamples {
    pub samples: Vec<u8>,
    /// The bytes per pixel in `samples`, one per base component. (GH#1913) ~keep
    pub components: usize,
    pub width: u32,
    pub height: u32,
}

/// Decode an `/Indexed` image XObject's samples in its base colour space,
/// bypassing the RGB conversion [`extract_image_from_xobject`] performs for
/// on-screen rendering.
///
/// ISO 32000-1 §7.4.5 puts an Indexed image's ink intent in its base colour
/// space; separation-plate routing needs the base's own component bytes, and
/// converting to RGB first (as the extractor does for display) would throw
/// that intent away before routing ever sees it. (GH#1898)
///
/// Returns `Ok(None)` when the XObject's colour space is not `/Indexed`, or
/// when its stream is JBIG2-, DCT- or CCITT-coded — each of those decodes
/// through its own dedicated path in `extract_image_from_xobject` and never
/// reaches the raw index bytes this function expects. A JPX-coded stream
/// decodes to its index plane at the codestream's own size and is looked up
/// here like any other. (GH#1916)
pub(crate) fn decode_indexed_image_in_base_space(
    doc: Option<&crate::document::PdfDocument>,
    xobject: &crate::object::Object,
    obj_ref: Option<ObjectRef>,
    color_space_map: Option<&std::collections::HashMap<String, crate::object::Object>>,
) -> Result<Option<IndexedBaseSamples>> {
    let dict = xobject
        .as_dict()
        .ok_or_else(|| Error::Image("XObject is not a stream".to_string()))?;

    let ImageXObjectMetadata {
        width,
        height,
        bits_per_component,
        color_space,
        indexed_resolution,
        is_jbig2,
        is_jpx,
        jpx_color_space_is_placeholder,
        is_jpeg_only,
        is_jpeg_chain,
        is_ccitt,
        ..
    } = resolve_image_xobject_metadata(doc, dict, color_space_map)?;

    let Some(ir) = indexed_resolution else {
        return Ok(None);
    };
    // A JPEG 2000 index plane is one byte per pixel at the codestream's own size, whatever the
    // dictionary's /Width, /Height and /BitsPerComponent say (ISO 32000-1 §7.4.9). (GH#1916) ~keep
    if is_jpx {
        let JpxDecoded {
            data, width, height, ..
        } = decode_jpx_image(
            xobject,
            obj_ref,
            doc,
            &color_space,
            jpx_color_space_is_placeholder,
            (width, height),
            Some(&ir),
        )?;
        let ImageData::Raw { pixels: indices, .. } = data else {
            return Ok(None);
        };
        let samples = expand_indexed_to_base_samples(&indices, &ir.palette, ir.components, width, height, 8)?;
        return Ok(Some(IndexedBaseSamples {
            samples,
            components: ir.components,
            width,
            height,
        }));
    }
    // `extract_image_from_xobject` only reaches the palette-expansion branch
    // this mirrors when none of these codecs claims the stream first --
    // JBIG2, DCT and CCITT each take their own decode branch there and
    // never consult `indexed_resolution` at all, however the dictionary's
    // `/ColorSpace` reads. Match that fallthrough exactly so this function
    // never treats an encoded stream's bytes as a raw index plane. ~keep
    if is_jbig2 || is_jpeg_only || is_jpeg_chain || is_ccitt {
        return Ok(None);
    }

    let expected_size = expected_image_filter_output_size(
        dict,
        width,
        height,
        ColorSpace::Indexed.components(),
        bits_per_component,
    )?;
    let decoded_data = decode_dimension_bounded_image_stream(doc, xobject, obj_ref, expected_size)?;
    let samples = expand_indexed_to_base_samples(
        &decoded_data,
        &ir.palette,
        ir.components,
        width,
        height,
        bits_per_component,
    )?;
    Ok(Some(IndexedBaseSamples {
        samples,
        components: ir.components,
        width,
        height,
    }))
}

/// Convert a single DeviceCMYK pixel to RGB.
///
/// Shared conversion math used by both bulk CMYK->RGB and Indexed palette
/// expansion so the two paths cannot drift apart.
///
/// Uses the PROCESS-INK conversion (`color::cmyk_to_rgb`, tetralinear over the
/// 16 measured ink corners of the CMYK cube), NOT the naive additive-then-clamp
/// `R = 1 - min(1, C+K)`. The additive form treats the inks as pure subtractive
/// primaries and so renders 100% K as `#000000` and 100% cyan as `#00FFFF`;
/// DeviceCMYK is a *device* space, so the colour is what the inks look like -
/// 100% K is `#231F20`, 100% cyan `#00ADEF`. This is the same conversion the
/// text/vector paths already use, so a DeviceCMYK image now matches the colour
/// of DeviceCMYK text and fills on the same page. For `/ICCBased` CMYK a real
/// CMM (qcms / lcms2) still takes precedence when a profile is available; this
/// is the no-profile fallback.
pub(crate) fn cmyk_pixel_to_rgb(c: u8, m: u8, y: u8, k: u8) -> [u8; 3] {
    let (r, g, b) = crate::color::cmyk_to_rgb(c as f32 / 255.0, m as f32 / 255.0, y as f32 / 255.0, k as f32 / 255.0);
    [
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    ]
}

/// Extract `/WhitePoint` from a Lab colour-space PDF object.
///
/// The object is `[/Lab << /WhitePoint [Xw Yw Zw] >>]`. Returns the
/// whitepoint as `[Xw, Yw, Zw]`, falling back to D65 if absent.
fn extract_lab_whitepoint(cs_obj: &crate::object::Object) -> [f64; 3] {
    const D65: [f64; 3] = [0.9505, 1.0, 1.0890];
    let arr = match cs_obj {
        crate::object::Object::Array(a) => a,
        _ => return D65,
    };
    if arr.len() < 2 {
        return D65;
    }
    let dict = match &arr[1] {
        crate::object::Object::Dictionary(d) => d,
        _ => return D65,
    };
    let wp = match dict.get("WhitePoint") {
        Some(crate::object::Object::Array(a)) if a.len() >= 3 => a,
        _ => return D65,
    };
    let f = |obj: &crate::object::Object| -> Option<f64> {
        match obj {
            crate::object::Object::Real(v) => Some(*v),
            crate::object::Object::Integer(v) => Some(*v as f64),
            _ => None,
        }
    };
    match (f(&wp[0]), f(&wp[1]), f(&wp[2])) {
        (Some(x), Some(y), Some(z)) => [x, y, z],
        _ => D65,
    }
}

/// Convert a Lab-encoded palette to sRGB.
///
/// Each entry is 3 bytes: L* (byte 0), a* (byte 1), b* (byte 2).
/// Decoding per PDF 32000-1:2008 §8.6.5.4:
///   L* = byte_0 / 255.0 × 100.0
///   a* = byte_1 − 128.0   (default /Range [−128 127])
///   b* = byte_2 − 128.0
///
/// Then Lab → XYZ (whitepoint-relative) → sRGB with standard gamma.
pub(crate) fn lab_palette_to_rgb(palette: &[u8], white: [f64; 3]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(palette.len());
    for chunk in palette.chunks(3) {
        if chunk.len() < 3 {
            rgb.extend_from_slice(&[0, 0, 0]);
            continue;
        }
        let [r, g, b] = lab_pixel_to_rgb(chunk[0], chunk[1], chunk[2], white);
        rgb.push(r);
        rgb.push(g);
        rgb.push(b);
    }
    rgb
}

// NOTE: The XYZ→linear-sRGB matrix below assumes a D65 whitepoint. Lab CIEs
// whose `/WhitePoint` is non-D65 (D50 is common in print workflows) would
// strictly need chromatic adaptation (e.g., Bradford) from the source
// whitepoint to D65 before the sRGB matrix. We intentionally omit that for
// now — the vast majority of PDF `/Lab` spaces we encounter are D65 — but
// the caller's `white` is still used to scale `xw, yw, zw` so D65 and
// near-D65 whitepoints produce correct output. Non-D65 spaces will have a
// minor chromatic-adaptation error until this is revisited. ~keep
fn lab_pixel_to_rgb(l_byte: u8, a_byte: u8, b_byte: u8, white: [f64; 3]) -> [u8; 3] {
    let l_star = l_byte as f64 / 255.0 * 100.0;
    let a_star = a_byte as f64 - 128.0;
    let b_star = b_byte as f64 - 128.0;

    let fy = (l_star + 16.0) / 116.0;
    let fx = a_star / 500.0 + fy;
    let fz = fy - b_star / 200.0;

    let [xw, yw, zw] = white;
    let x = xw * f_inv(fx);
    let y = yw * f_inv(fy);
    let z = zw * f_inv(fz);

    // XYZ → linear sRGB (D65 matrix, IEC 61966-2-1:1999) ~keep
    let r_lin = 3.2406254773 * x - 1.5372079722 * y - 0.4986285987 * z;
    let g_lin = -0.9689307147 * x + 1.8757560609 * y + 0.0415175580 * z;
    let b_lin = 0.0557101204 * x - 0.2040210506 * y + 1.0569959423 * z;

    [srgb_gamma(r_lin), srgb_gamma(g_lin), srgb_gamma(b_lin)]
}

fn f_inv(t: f64) -> f64 {
    const DELTA: f64 = 6.0 / 29.0;
    if t > DELTA {
        t * t * t
    } else {
        3.0 * DELTA * DELTA * (t - 4.0 / 29.0)
    }
}

fn srgb_gamma(lin: f64) -> u8 {
    let v = if lin <= 0.0031308 {
        12.92 * lin
    } else {
        1.055 * lin.powf(1.0 / 2.4) - 0.055
    };
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// Convert a raw CMYK byte stream (4 bytes per pixel) to straight RGB bytes
/// (3 bytes per pixel) using the naive per-pixel conversion.
///
/// This is a non-ICC conversion and does not handle Adobe-inverted JPEG CMYK;
/// for JPEG-encoded CMYK streams use `decode_adobe_cmyk_jpeg` instead.
pub fn cmyk_to_rgb(cmyk: &[u8]) -> Vec<u8> {
    cmyk_to_rgb_with_transform(cmyk, None)
}

/// Like [`cmyk_to_rgb`] but routes through an ICC transform when given,
/// and falls through to §10.3.5 otherwise. Used by save_raw_as_* when
/// the source image carries an ICC profile.
pub fn cmyk_to_rgb_with_transform(cmyk: &[u8], transform: Option<&crate::color::Transform>) -> Vec<u8> {
    if let Some(t) = transform {
        return t.convert_cmyk_buffer(cmyk);
    }
    let mut rgb = Vec::with_capacity((cmyk.len() / 4) * 3);
    for chunk in cmyk.as_chunks::<4>().0 {
        let [r, g, b] = cmyk_pixel_to_rgb(chunk[0], chunk[1], chunk[2], chunk[3]);
        rgb.push(r);
        rgb.push(g);
        rgb.push(b);
    }
    rgb
}

/// Decode a CMYK-colourspace JPEG to straight RGB bytes, applying Adobe's
/// Thin wrapper that falls back to the intent-less, profile-less
/// variant — kept as the public, backwards-compatible entry point.
pub fn decode_cmyk_jpeg_to_rgb(jpeg_data: &[u8]) -> Result<Vec<u8>> {
    decode_cmyk_jpeg_to_rgb_with_profile(jpeg_data, None)
}

/// Decode a DeviceCMYK JPEG to raw 8-bpc CMYK samples (W*H*4 bytes,
/// channel order C, M, Y, K). Output is the raw DCT sample plane treated
/// as straight CMYK ink (0 = no ink, 255 = full coverage), matching how
/// poppler / Ghostscript render DCTDecode CMYK streams inside a PDF.
///
/// `jpeg-decoder` 0.3 applies Adobe's `255 - x` inversion to EVERY
/// 4-component JPEG that carries an Adobe APP14 marker
/// (`color_convert_line_cmyk` for `transform = 0`; `color_convert_line_ycck`
/// for `transform = 2`). PDF renderers do not do that inversion - they use
/// the raw DCT samples as the CMYK ink. So whenever an Adobe marker is
/// present, xberg-native-pdf must undo the decoder's inversion with a second
/// `255 - x` to recover the raw samples:
///
/// - `color_transform = 0` (plain CMYK, Photoshop / Distiller default) -
///   undo the decoder's `color_convert_line_cmyk` inversion.
/// - `color_transform = 2` (YCCK, Adobe Illustrator default) - the decoder
///   ran YCbCr->RGB plus `255 - K`; the same `255 - x` on all four channels
///   recovers the raw CMYK plane.
/// - no APP14 marker - the samples are used as-is (non-Adobe convention),
///   left unchanged to avoid disturbing non-Adobe JPEG handling.
///
/// The jpeg-decoder inversion contract is pinned by
/// `tests/test_jpeg_decoder_cmyk_contract.rs`; the Adobe decode path is
/// covered by `tests/test_cmyk_jpeg_adobe_inversion.rs`. If a future
/// jpeg-decoder release changes its inversion behaviour, those tests fire
/// before real fixtures regress.
///
/// Used by the separation pipeline to route CMYK image channels directly
/// to the matching ink plates without going through a colour-space
/// conversion to RGB and back. Only the rasterizer consumes this;
/// without it the function would be dead code under `-D warnings`.
pub(crate) fn decode_cmyk_jpeg_to_raw_cmyk(jpeg_data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = jpeg_decoder::Decoder::new(std::io::Cursor::new(jpeg_data));
    let cmyk = decoder
        .decode()
        .map_err(|e| Error::Decode(format!("Failed to decode CMYK JPEG: {}", e)))?;
    let info = decoder
        .info()
        .ok_or_else(|| Error::Decode("JPEG info unavailable".to_string()))?;

    let pixel_count = (info.width as usize) * (info.height as usize);
    let expected = pixel_count * 4;
    if cmyk.len() < expected {
        return Err(Error::Decode(format!(
            "CMYK JPEG decoded {} bytes, expected {}",
            cmyk.len(),
            expected
        )));
    }

    let mut raw = cmyk;
    raw.truncate(expected);
    if matches!(scan_app14_color_transform(jpeg_data), Some(0) | Some(2)) {
        for b in raw.iter_mut() {
            *b = 255 - *b;
        }
    }
    Ok(raw)
}

/// Like [`decode_cmyk_jpeg_to_rgb`] but applies the given ICC transform
/// when provided, falling back to §10.3.5 otherwise. Used internally by
/// `PdfImage::save_as_*` when the source image carries an ICCBased
/// colour space (or when the document's `OutputIntents` supplied a
/// default CMYK profile).
///
/// APP14 handling matches `decode_cmyk_jpeg_to_raw_cmyk`: when an Adobe
/// marker is present (CMYK transform 0 or YCCK transform 2), jpeg-decoder's
/// `255 - x` inversion is undone to recover the raw DCT samples poppler
/// treats as straight CMYK; without a marker the samples pass through.
pub fn decode_cmyk_jpeg_to_rgb_with_profile(
    jpeg_data: &[u8],
    transform: Option<&crate::color::Transform>,
) -> Result<Vec<u8>> {
    let mut decoder = jpeg_decoder::Decoder::new(std::io::Cursor::new(jpeg_data));
    let cmyk = decoder
        .decode()
        .map_err(|e| Error::Decode(format!("Failed to decode CMYK JPEG: {}", e)))?;
    let info = decoder
        .info()
        .ok_or_else(|| Error::Decode("JPEG info unavailable".to_string()))?;

    let pixel_count = (info.width as usize) * (info.height as usize);
    let expected = pixel_count * 4;
    if cmyk.len() < expected {
        return Err(Error::Decode(format!(
            "CMYK JPEG decoded {} bytes, expected {}",
            cmyk.len(),
            expected
        )));
    }

    let straight_cmyk: Vec<u8> = if matches!(scan_app14_color_transform(jpeg_data), Some(0) | Some(2)) {
        cmyk[..expected].iter().map(|b| 255 - *b).collect()
    } else {
        cmyk[..expected].to_vec()
    };

    if let Some(t) = transform {
        return Ok(t.convert_cmyk_buffer(&straight_cmyk));
    }

    // §10.3.5 additive-clamp fallback. ~keep
    let mut rgb = Vec::with_capacity(pixel_count * 3);
    for chunk in straight_cmyk.as_chunks::<4>().0 {
        let [r, g, b] = cmyk_pixel_to_rgb(chunk[0], chunk[1], chunk[2], chunk[3]);
        rgb.push(r);
        rgb.push(g);
        rgb.push(b);
    }
    Ok(rgb)
}

/// Walk the JPEG marker stream for an Adobe APP14 ("Adobe") segment and
/// return its `color_transform` byte. The byte is 0 for plain CMYK
/// (Photoshop), 1 for YCbCr (3-channel), 2 for YCCK (Adobe Illustrator).
fn scan_app14_color_transform(jpeg_data: &[u8]) -> Option<u8> {
    let mut i = 0;
    while i + 1 < jpeg_data.len() {
        if jpeg_data[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = jpeg_data[i + 1];
        i += 2;
        if marker == 0x00 || marker == 0xFF {
            continue;
        }
        if matches!(marker, 0xD0..=0xD9) || marker == 0x01 {
            continue;
        }
        if i + 1 >= jpeg_data.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([jpeg_data[i], jpeg_data[i + 1]]) as usize;
        if seg_len < 2 || i + seg_len > jpeg_data.len() {
            break;
        }
        if marker == 0xEE && seg_len >= 14 {
            let payload = &jpeg_data[i + 2..i + seg_len];
            if payload.len() >= 12 && payload.starts_with(b"Adobe") {
                return Some(payload[11]);
            }
        }
        if marker == 0xDA {
            break;
        }
        i += seg_len;
    }
    None
}

fn save_jpeg_as_png(jpeg_data: &[u8], path: impl AsRef<Path>) -> Result<()> {
    use image::ImageFormat;
    let img = image::load_from_memory_with_format(jpeg_data, ImageFormat::Jpeg)
        .map_err(|e| Error::Image(format!("Failed to decode JPEG: {}", e)))?;
    img.save_with_format(path, ImageFormat::Png)
        .map_err(|e| Error::Image(format!("Failed to save PNG: {}", e)))
}

/// Decide whether a given ICC transform should actually be applied to a
/// buffer of the given pixel format. The transform was compiled for
/// whatever component count the profile advertised; applying it to a
/// mismatched buffer (e.g. a 4-component CMYK transform to a 3-channel
/// RGB buffer) would produce garbage. A `None` transform, or a
/// transform whose profile components disagree with `format`, is
/// suppressed so the caller falls through to identity / fallback math.
fn icc_matches_format(
    transform: Option<&crate::color::Transform>,
    format: PixelFormat,
) -> Option<&crate::color::Transform> {
    let t = transform?;
    let needed = match format {
        PixelFormat::RGB => 3,
        PixelFormat::Grayscale => 1,
        PixelFormat::CMYK => 4,
    };
    if t.source_n_components() == needed {
        Some(t)
    } else {
        None
    }
}

fn save_raw_as_png(
    pixels: &[u8],
    width: u32,
    height: u32,
    format: PixelFormat,
    transform: Option<&crate::color::Transform>,
    path: impl AsRef<Path>,
) -> Result<()> {
    use image::{ImageBuffer, ImageFormat, Luma, Rgb};

    match format {
        PixelFormat::RGB => {
            // RGB source through an ICC profile (Adobe RGB, ProPhoto, wide-
            // gamut cameras) → convert to sRGB before writing. With no
            // profile the bytes are assumed sRGB already and passed through. ~keep
            let rgb = match icc_matches_format(transform, format) {
                Some(t) => t.convert_rgb_buffer(pixels),
                None => pixels.to_vec(),
            };
            let img = ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, rgb)
                .ok_or_else(|| Error::Image("Invalid RGB image dimensions".to_string()))?;
            img.save_with_format(path, ImageFormat::Png)
                .map_err(|e| Error::Image(format!("Failed to save PNG: {}", e)))
        }
        PixelFormat::Grayscale => {
            // A Gray ICC profile promotes to sRGB RGB; without one the
            // single channel is written as an L8 PNG. ~keep
            if let Some(t) = icc_matches_format(transform, format) {
                let rgb = t.convert_gray_buffer(pixels);
                let img = ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, rgb)
                    .ok_or_else(|| Error::Image("Invalid grayscale image dimensions".to_string()))?;
                img.save_with_format(path, ImageFormat::Png)
                    .map_err(|e| Error::Image(format!("Failed to save PNG: {}", e)))
            } else {
                let img = ImageBuffer::<Luma<u8>, _>::from_raw(width, height, pixels.to_vec())
                    .ok_or_else(|| Error::Image("Invalid grayscale image dimensions".to_string()))?;
                img.save_with_format(path, ImageFormat::Png)
                    .map_err(|e| Error::Image(format!("Failed to save PNG: {}", e)))
            }
        }
        PixelFormat::CMYK => {
            let rgb = cmyk_to_rgb_with_transform(pixels, icc_matches_format(transform, format));
            let img = ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, rgb)
                .ok_or_else(|| Error::Image("Invalid CMYK image dimensions".to_string()))?;
            img.save_with_format(path, ImageFormat::Png)
                .map_err(|e| Error::Image(format!("Failed to save PNG: {}", e)))
        }
    }
}

fn save_raw_as_jpeg(
    pixels: &[u8],
    width: u32,
    height: u32,
    format: PixelFormat,
    transform: Option<&crate::color::Transform>,
    path: impl AsRef<Path>,
) -> Result<()> {
    use image::{ImageBuffer, ImageFormat, Luma, Rgb};

    match format {
        PixelFormat::RGB => {
            let rgb = match icc_matches_format(transform, format) {
                Some(t) => t.convert_rgb_buffer(pixels),
                None => pixels.to_vec(),
            };
            let img = ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, rgb)
                .ok_or_else(|| Error::Image("Invalid RGB image dimensions".to_string()))?;
            img.save_with_format(path, ImageFormat::Jpeg)
                .map_err(|e| Error::Image(format!("Failed to save JPEG: {}", e)))
        }
        PixelFormat::Grayscale => {
            if let Some(t) = icc_matches_format(transform, format) {
                let rgb = t.convert_gray_buffer(pixels);
                let img = ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, rgb)
                    .ok_or_else(|| Error::Image("Invalid grayscale image dimensions".to_string()))?;
                img.save_with_format(path, ImageFormat::Jpeg)
                    .map_err(|e| Error::Image(format!("Failed to save JPEG: {}", e)))
            } else {
                let img = ImageBuffer::<Luma<u8>, _>::from_raw(width, height, pixels.to_vec())
                    .ok_or_else(|| Error::Image("Invalid grayscale image dimensions".to_string()))?;
                img.save_with_format(path, ImageFormat::Jpeg)
                    .map_err(|e| Error::Image(format!("Failed to save JPEG: {}", e)))
            }
        }
        PixelFormat::CMYK => {
            let rgb = cmyk_to_rgb_with_transform(pixels, icc_matches_format(transform, format));
            let img = ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, rgb)
                .ok_or_else(|| Error::Image("Invalid CMYK image dimensions".to_string()))?;
            img.save_with_format(path, ImageFormat::Jpeg)
                .map_err(|e| Error::Image(format!("Failed to save JPEG: {}", e)))
        }
    }
}

/// Bound the pixel buffer a JBIG2 decode will need before allocating it.
///
/// `/Width` and `/Height` come straight from the image dictionary with no
/// upper bound, and hayro-jbig2 decodes the embedded codestream using its
/// OWN internal header — a tiny valid codestream paired with astronomical
/// declared dimensions decodes fine and then sizes the pixel buffer from the
/// dictionary alone. `width * height` in `u32` panics on overflow in debug
/// and silently wraps to a too-small capacity in release, after which
/// `push_pixel`/`next_line` grow the buffer unbounded to make up the
/// difference. Bound it the same 256 MiB cap `samples_to_decoded_bytes` and
/// `expand_indexed_to_rgb_with_transform` use for their own dictionary-driven
/// allocations. ~keep
fn checked_jbig2_pixel_capacity(width: u32, height: u32) -> Result<usize> {
    const MAX_JBIG2_PIXEL_BYTES: usize = 256 * 1024 * 1024;
    let pixel_count = (width as usize).checked_mul(height as usize).ok_or_else(|| {
        Error::Image(format!(
            "JBIG2 image pixel-count overflow: {width} × {height} exceeds usize"
        ))
    })?;
    if pixel_count > MAX_JBIG2_PIXEL_BYTES {
        return Err(Error::Image(format!(
            "JBIG2 image would decode to {pixel_count} pixel bytes, exceeds guard limit of \
             {MAX_JBIG2_PIXEL_BYTES} bytes (width={width}, height={height})"
        )));
    }
    Ok(pixel_count)
}

/// Decode a JBIG2-compressed PDF image stream into raw grayscale pixels.
fn decode_jbig2_image(
    xobject: &crate::object::Object,
    obj_ref: Option<ObjectRef>,
    dict: &std::collections::HashMap<String, crate::object::Object>,
    doc: Option<&crate::document::PdfDocument>,
    width: u32,
    height: u32,
) -> Result<ImageData> {
    // Bound the declared dimensions before decoding a single byte of the
    // codestream: hayro-jbig2 reads its own internal header, so a tiny valid
    // stream can carry an astronomical `/Width` × `/Height` and still decode
    // "successfully", after which nothing else in this function would have
    // caught the size before allocating. Fail fast on the dictionary values
    // alone rather than doing wasted decode work first. ~keep
    let pixel_count = checked_jbig2_pixel_capacity(width, height)?;

    // The Jbig2Decoder in src/decoders/jbig2.rs is a pass-through: it returns
    // the raw compressed bitstream unchanged, which is exactly what hayro-jbig2
    // needs as input. ~keep
    let jbig2_bytes: Vec<u8> = if let (Some(d), Some(ref_id)) = (doc.as_ref(), obj_ref) {
        d.decode_stream_with_encryption(xobject, ref_id)?
    } else {
        xobject.decode_stream_data()?
    };

    // Load optional JBIG2Globals (shared symbol dictionaries referenced by multiple
    // embedded JBIG2 streams in the same PDF). ~keep
    let globals: Option<Vec<u8>> = (|| -> Option<Vec<u8>> {
        let dp = dict.get("DecodeParms")?.as_dict()?;
        let globals_ref = dp.get("JBIG2Globals")?.as_reference()?;
        let d = doc.as_ref()?;
        let globals_obj = d.load_object(globals_ref).ok()?;
        d.decode_stream_with_encryption(&globals_obj, globals_ref).ok()
    })();

    let image = hayro_jbig2::Image::new_embedded(&jbig2_bytes, globals.as_deref())
        .map_err(|e| Error::Image(format!("JBIG2 decode error: {e}")))?;

    struct PixelCollector {
        pixels: Vec<u8>,
        row_buf: Vec<u8>,
    }

    impl hayro_jbig2::Decoder for PixelCollector {
        fn push_pixel(&mut self, black: bool) {
            self.row_buf.push(if black { 0 } else { 255 });
        }

        // chunk_count is the number of 8-pixel groups, not individual pixels. ~keep
        fn push_pixel_chunk(&mut self, black: bool, chunk_count: u32) {
            let v = if black { 0u8 } else { 255u8 };
            let n = chunk_count as usize * 8;
            self.row_buf.extend(std::iter::repeat_n(v, n));
        }

        fn next_line(&mut self) {
            self.pixels.append(&mut self.row_buf);
        }
    }

    let mut collector = PixelCollector {
        pixels: Vec::with_capacity(pixel_count),
        row_buf: Vec::with_capacity(width as usize),
    };

    image
        .decode(&mut collector)
        .map_err(|e| Error::Image(format!("JBIG2 pixel decode error: {e}")))?;

    Ok(ImageData::Raw {
        pixels: collector.pixels,
        format: PixelFormat::Grayscale,
    })
}

#[cfg(test)]
mod jbig2_pixel_capacity_tests {
    use super::checked_jbig2_pixel_capacity;

    #[test]
    fn should_accept_an_ordinary_scanned_page_size() {
        // A typical 300 DPI US Letter scan: well under the 256 MiB cap. ~keep
        let capacity = checked_jbig2_pixel_capacity(2550, 3300).expect("an ordinary page size must not be rejected");

        assert_eq!(capacity, 2550 * 3300);
    }

    #[test]
    fn should_reject_dimensions_whose_pixel_count_exceeds_the_guard_cap() {
        // 20_000 x 20_000 = 400_000_000 pixel bytes, over the 256 MiB (268_435_456
        // byte) cap; the tiny declared codestream size this stands in for is
        // exactly the shape a hostile JBIG2 XObject uses. ~keep
        let result = checked_jbig2_pixel_capacity(20_000, 20_000);

        assert!(result.is_err(), "oversized JBIG2 image must be rejected, not decoded");
    }

    #[test]
    fn should_reject_dimensions_whose_product_overflows_usize() {
        let result = checked_jbig2_pixel_capacity(u32::MAX, u32::MAX);

        assert!(
            result.is_err(),
            "overflowing width * height must be rejected, not panic"
        );
    }
}

/// Decode a JBIG2 `/ImageMask` into packed 1-bit rows in PDF *sample* convention.
///
/// The renderer's stencil path needs packed MSB-first rows padded to a byte boundary,
/// where sample 0 paints under the default `/Decode [0 1]`. `decode_jbig2_image`
/// produces one grayscale byte per pixel instead, so without this the raw JBIG2
/// bitstream reaches the stencil loop's length check and is rejected as
/// "ImageMask stream too short" -- compressed input is far smaller than the bitmap it
/// expands to, so that check can never pass for JBIG2 and every mask is dropped.
///
/// Scanners producing Mixed Raster Content put ALL page text in these masks, so a
/// dropped mask is a page that renders as its blank background: text-layer-free scans
/// then OCR to nothing, with no error surfaced anywhere.
pub(crate) fn decode_jbig2_image_mask_bits(
    xobject: &crate::object::Object,
    obj_ref: Option<ObjectRef>,
    dict: &std::collections::HashMap<String, crate::object::Object>,
    doc: Option<&crate::document::PdfDocument>,
    width: u32,
    height: u32,
) -> Result<Vec<u8>> {
    let decoded = decode_jbig2_image(xobject, obj_ref, dict, doc, width, height)?;
    let ImageData::Raw { pixels, .. } = decoded else {
        return Err(Error::Image(
            "JBIG2 ImageMask decode did not yield raw pixels".to_string(),
        ));
    };

    pack_image_mask_rows(&pixels, width, height)
}

/// Pack one grayscale byte per pixel into MSB-first 1-bit rows in PDF *sample*
/// convention, each row padded to a byte boundary.
///
/// hayro-jbig2 emits 0 for ink and 255 for background. An `/ImageMask` paints where
/// the sample is 0 under the default `/Decode [0 1]`, so ink must *clear* the bit and
/// background must *set* it. Pixels missing from a short decode are treated as
/// background: leaving a region unpainted is recoverable, painting spurious ink over
/// the page is not.
fn pack_image_mask_rows(pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    // `/Width` and `/Height` come straight from the image dictionary with no
    // upper bound. A JBIG2 `/ImageMask` decodes its codestream using
    // hayro-jbig2's own embedded header, so a tiny valid stream can still
    // declare astronomical dimensions here, sizing both this allocation and
    // the nested per-pixel loop below from the dictionary alone. Bound the
    // packed output the same 256 MiB cap `expand_indexed_to_rgb_with_transform`
    // and `samples_to_decoded_bytes` use for their own dictionary-driven
    // allocations: `row_bytes * height` bounded this way also bounds the
    // `y * width + x` index arithmetic in the loop, so no separate overflow
    // check is needed there. ~keep
    const MAX_MASK_OUTPUT_BYTES: usize = 256 * 1024 * 1024;

    let row_bytes = (width as usize).div_ceil(8);
    let packed_len = row_bytes.checked_mul(height as usize).ok_or_else(|| {
        Error::Image(format!(
            "ImageMask row-bytes overflow: {row_bytes} × {height} exceeds usize"
        ))
    })?;
    if packed_len > MAX_MASK_OUTPUT_BYTES {
        return Err(Error::Image(format!(
            "ImageMask decode would produce {packed_len} bytes, exceeds guard limit of \
             {MAX_MASK_OUTPUT_BYTES} bytes (width={width}, height={height})"
        )));
    }

    let mut packed = vec![0u8; packed_len];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let is_background = pixels.get(y * width as usize + x).is_none_or(|&pixel| pixel >= 128);
            if is_background {
                packed[y * row_bytes + (x / 8)] |= 0x80 >> (x % 8);
            }
        }
    }
    Ok(packed)
}

/// Decode a JPEG 2000 (`/JPXDecode`) image stream into raw interleaved samples.
///
/// `JpxDecoder` is a pass-through, so `decode_stream_*` yields the raw JPEG 2000
/// codestream, which `hayro-jpeg2000` (`decoders::jpx::decode_jpx`) decodes to
/// 8-bit component-interleaved samples. An `/Indexed` image, given its resolved palette in
/// `indexed`, decodes instead to one palette index per pixel.
///
/// Returns the decoded pixels alongside the width and height the codestream itself declares,
/// which the caller must use in place of the dictionary's `declared_size` -- see the size check
/// below (GH#1900).
fn decode_jpx_image(
    xobject: &crate::object::Object,
    obj_ref: Option<ObjectRef>,
    doc: Option<&crate::document::PdfDocument>,
    color_space: &ColorSpace,
    color_space_is_placeholder: bool,
    declared_size: (u32, u32),
    indexed: Option<&IndexedResolution>,
) -> Result<JpxDecoded> {
    let codestream: Vec<u8> = if let (Some(d), Some(ref_id)) = (doc.as_ref(), obj_ref) {
        d.decode_stream_with_encryption(xobject, ref_id)?
    } else {
        xobject.decode_stream_data()?
    };

    // ISO 32000-1 §7.4.9 puts the `/Indexed` palette in charge, so any `pclr` palette in the
    // codestream is not applied; the caller looks the indices up. (GH#1885) ~keep
    if let Some(highest_index) = indexed.map(IndexedResolution::highest_index) {
        let indices = crate::decoders::jpx::decode_jpx_indices(&codestream, highest_index)?;
        warn_on_jpx_size_mismatch(declared_size, (indices.width, indices.height));
        return Ok(JpxDecoded {
            data: ImageData::Raw {
                pixels: indices.samples,
                format: PixelFormat::Grayscale,
            },
            width: indices.width,
            height: indices.height,
            opacity: indices.opacity,
        });
    }
    // An explicit `/ColorSpace` is authoritative (ISO 32000-1 §7.4.9), so a codestream palette that
    // resolves to a different component count is rejected rather than reinterpreted. ~keep
    if !color_space_is_placeholder
        && let Some(columns) = crate::decoders::jpx::codestream_palette_columns(&codestream)
        && usize::from(columns) != color_space.components()
    {
        return Err(Error::UnsupportedFilter(format!(
            "JPXDecode: codestream palette has {columns} components, but /ColorSpace has {}",
            color_space.components()
        )));
    }
    // ISO 32000-1 §7.4.9 makes the XObject's `/ColorSpace` authoritative over the codestream, and
    // it is load-bearing rather than informational: for a BARE codestream hayro-jpeg2000 infers
    // `Srgb` from the component count and then treats a 4th channel as alpha, so a CMYK image
    // reports 3 colour components and loses its K plane unless the declared count contradicts it
    // (GH#1850). `Pattern` reports 0 components and is filtered out as meaningless here.
    //
    // When the dictionary named no `/ColorSpace` at all, §7.4.9's authoritative entry does not
    // exist and `color_space` is the `/DeviceRGB` placeholder resolve_image_xobject_metadata
    // installed to let the rest of extraction run. Passing its 3 would assert a count the document
    // never made, and for a bare four-component codestream that assertion is what dropped the K
    // plane (GH#1883). `None` sends the decoder to the codestream's own `SIZ` header instead. ~keep
    let declared_components = if color_space_is_placeholder {
        None
    } else {
        u8::try_from(color_space.components()).ok().filter(|&n| n > 0)
    };
    let img = crate::decoders::jpx::decode_jpx(&codestream, declared_components)?;

    // ISO 32000-1 §7.4.9 says /Width and /Height are informative for a /JPXDecode image -- the
    // codestream is authoritative for its own size, unlike the component count above, which has no
    // codestream-only answer once a `/ColorSpace` is declared. A mismatch used to reach `PdfImage`
    // as the DICTIONARY'S size paired with a buffer sized for the CODESTREAM'S: a consumer that
    // trusts `width * height * components` either read a scrambled prefix at the wrong stride or
    // rejected the buffer outright as the wrong length, and neither recorded why (GH#1900).
    // Returning the codestream's own size keeps the two in agreement for every consumer. ~keep
    warn_on_jpx_size_mismatch(declared_size, (img.width, img.height));

    let format = match img.num_components {
        1 => PixelFormat::Grayscale,
        3 => PixelFormat::RGB,
        4 => PixelFormat::CMYK,
        n => {
            return Err(Error::UnsupportedFilter(format!(
                "JPXDecode: unsupported JPEG 2000 component count {n}"
            )));
        }
    };

    Ok(JpxDecoded {
        data: ImageData::Raw {
            pixels: img.samples,
            format,
        },
        width: img.width,
        height: img.height,
        opacity: img.opacity,
    })
}

/// Record a `JpxSizeMismatch` warning when a `/JPXDecode` image's dictionary declares a size other
/// than its codestream's. The codestream's size is the one used (ISO 32000-1 §7.4.9, GH#1900).
fn warn_on_jpx_size_mismatch(declared: (u32, u32), codestream: (u32, u32)) {
    if declared == codestream {
        return;
    }
    let ((declared_width, declared_height), (width, height)) = (declared, codestream);
    let msg = format!(
        "SPEC NOTE: /JPXDecode image dictionary declares {declared_width}x{declared_height}, but \
         its codestream's SIZ marker segment declares {width}x{height}. ISO 32000-1:2008 Section 7.4.9 \
         treats /Width and /Height as informative for JPEG 2000, so the codestream's size is used."
    );
    tracing::warn!("{}", msg);
    crate::extractors::warnings::push_global_warning(crate::extractors::warnings::Warning {
        category: crate::extractors::warnings::WarningCategory::JpxSizeMismatch,
        page: None,
        message: msg,
        spec_section: Some("7.4.9"),
    });
}

/// A decoded `/JPXDecode` image: its samples, the codestream's own size, and its opacity channel
/// when it carries one.
struct JpxDecoded {
    data: ImageData,
    width: u32,
    height: u32,
    opacity: Option<Vec<u8>>,
}

/// Expand an inline image's abbreviated dictionary (ISO 32000-1 §8.9.7 Table 91)
/// into the equivalent image XObject dictionary.
///
/// As well as expanding the abbreviations (`/W` → `/Width`, …) this supplies the
/// `/Subtype /Image` that an inline image never carries: §8.9.7 says an inline
/// image's dictionary holds "a subset of the entries in the image dictionary",
/// with the subtype implied by the `BI` operator rather than written out. Callers
/// hand the result to the image-XObject decoder, which *requires* `/Subtype` — so
/// without this the decoder rejects every inline image with "XObject missing
/// /Subtype", and the callers, which use `if let Ok(..)`, drop them SILENTLY.
pub fn expand_inline_image_dict(
    mut dict: std::collections::HashMap<String, crate::object::Object>,
) -> std::collections::HashMap<String, crate::object::Object> {
    use std::collections::HashMap;
    const KEY_ABBREVS: [(&str, &str); 9] = [
        ("W", "Width"),
        ("H", "Height"),
        ("CS", "ColorSpace"),
        ("BPC", "BitsPerComponent"),
        ("F", "Filter"),
        ("DP", "DecodeParms"),
        ("IM", "ImageMask"),
        ("I", "Interpolate"),
        ("D", "Decode"),
    ];
    let mut expanded = HashMap::new();
    // A dictionary carrying BOTH forms of one key (`/F` and `/Filter`) must
    // resolve the same way every run: the abbreviated form wins, matching
    // pdf.js's dict.get("F", "Filter"). Draining the abbreviations first makes
    // that precedence structural; deciding it inside a HashMap iteration made
    // it per-process hash-seed luck. ~keep
    for (abbrev, full) in KEY_ABBREVS {
        let value = match dict.remove(abbrev) {
            Some(v) => {
                dict.remove(full);
                Some(v)
            }
            None => dict.remove(full),
        };
        if let Some(v) = value {
            expanded.insert(full.to_string(), v);
        }
    }
    for (key, value) in dict {
        expanded.insert(key, value);
    }
    // §8.9.7 Table 92: inline images abbreviate the VALUES too, not just the
    // keys - `/CS /RGB`, `/F /Fl`. Expanding only the keys leaves the decoder
    // looking at a colour space called "RGB", which it does not know. ~keep
    for (key, map) in [
        ("ColorSpace", colorspace_abbrev as fn(&str) -> Option<&'static str>),
        ("Filter", filter_abbrev as fn(&str) -> Option<&'static str>),
    ] {
        if let Some(v) = expanded.remove(key) {
            expanded.insert(key.to_string(), expand_inline_abbrev(v, map));
        }
    }
    // §8.9.7: the subtype is implied by `BI`, never written in the dictionary.
    // The image-XObject decoder requires it, so supply it here. Do not clobber a
    // dictionary that somehow carries one already. ~keep
    expanded
        .entry("Subtype".to_string())
        .or_insert_with(|| crate::object::Object::Name("Image".to_string()));
    expanded
}

/// §8.9.7 Table 92 colour-space abbreviations. An unabbreviated name (or a name
/// we do not recognise, e.g. a `/Resources /ColorSpace` entry like `/CS0`) passes
/// through untouched.
fn colorspace_abbrev(name: &str) -> Option<&'static str> {
    match name {
        "G" => Some("DeviceGray"),
        "RGB" => Some("DeviceRGB"),
        "CMYK" => Some("DeviceCMYK"),
        "I" => Some("Indexed"),
        _ => None,
    }
}

/// §8.9.7 Table 92 filter abbreviations.
fn filter_abbrev(name: &str) -> Option<&'static str> {
    match name {
        "AHx" => Some("ASCIIHexDecode"),
        "A85" => Some("ASCII85Decode"),
        "LZW" => Some("LZWDecode"),
        "Fl" => Some("FlateDecode"),
        "RL" => Some("RunLengthDecode"),
        "CCF" => Some("CCITTFaxDecode"),
        "DCT" => Some("DCTDecode"),
        _ => None,
    }
}

/// Rewrite abbreviated names in an inline-image value. Handles a bare name, and
/// an array (a filter chain, or an `[/I /RGB 255 <lookup>]` indexed space, whose
/// BASE name is itself abbreviated).
fn expand_inline_abbrev(value: crate::object::Object, map: fn(&str) -> Option<&'static str>) -> crate::object::Object {
    use crate::object::Object;
    match value {
        Object::Name(n) => match map(&n) {
            Some(full) => Object::Name(full.to_string()),
            None => Object::Name(n),
        },
        Object::Array(items) => Object::Array(items.into_iter().map(|item| expand_inline_abbrev(item, map)).collect()),
        other => other,
    }
}

#[cfg(test)]
mod inline_image_dict_tests {
    use super::*;
    use crate::object::Object;
    use std::collections::HashMap;

    fn dict(pairs: &[(&str, Object)]) -> HashMap<String, Object> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    /// The decoder requires `/Subtype`, and an inline image never carries one
    /// (§8.9.7 - it is implied by `BI`). Without this every inline image was
    /// rejected with "XObject missing /Subtype" and silently dropped.
    #[test]
    fn supplies_the_implied_image_subtype() {
        let out = expand_inline_image_dict(dict(&[("W", Object::Integer(26)), ("H", Object::Integer(1))]));
        assert_eq!(out.get("Subtype"), Some(&Object::Name("Image".to_string())));
        assert_eq!(out.get("Width"), Some(&Object::Integer(26)));
        assert_eq!(out.get("Height"), Some(&Object::Integer(1)));
    }

    /// §8.9.7 Table 92: inline images abbreviate the VALUES too. Expanding only
    /// the keys left the decoder looking at a colour space named "RGB".
    #[test]
    fn expands_abbreviated_colour_space_and_filter_values() {
        let out = expand_inline_image_dict(dict(&[
            ("CS", Object::Name("RGB".to_string())),
            ("F", Object::Name("Fl".to_string())),
        ]));
        assert_eq!(out.get("ColorSpace"), Some(&Object::Name("DeviceRGB".to_string())));
        assert_eq!(out.get("Filter"), Some(&Object::Name("FlateDecode".to_string())));
    }

    #[test]
    fn expands_every_table_92_abbreviation() {
        for (abbr, full) in [
            ("G", "DeviceGray"),
            ("RGB", "DeviceRGB"),
            ("CMYK", "DeviceCMYK"),
            ("I", "Indexed"),
        ] {
            let out = expand_inline_image_dict(dict(&[("CS", Object::Name(abbr.to_string()))]));
            assert_eq!(
                out.get("ColorSpace"),
                Some(&Object::Name(full.to_string())),
                "colour space /{abbr}"
            );
        }
        for (abbr, full) in [
            ("AHx", "ASCIIHexDecode"),
            ("A85", "ASCII85Decode"),
            ("LZW", "LZWDecode"),
            ("Fl", "FlateDecode"),
            ("RL", "RunLengthDecode"),
            ("CCF", "CCITTFaxDecode"),
            ("DCT", "DCTDecode"),
        ] {
            let out = expand_inline_image_dict(dict(&[("F", Object::Name(abbr.to_string()))]));
            assert_eq!(
                out.get("Filter"),
                Some(&Object::Name(full.to_string())),
                "filter /{abbr}"
            );
        }
    }

    /// A filter CHAIN, and an indexed space whose base name is itself abbreviated.
    #[test]
    fn expands_abbreviations_inside_arrays() {
        let out = expand_inline_image_dict(dict(&[
            (
                "F",
                Object::Array(vec![Object::Name("A85".to_string()), Object::Name("Fl".to_string())]),
            ),
            (
                "CS",
                Object::Array(vec![
                    Object::Name("I".to_string()),
                    Object::Name("RGB".to_string()),
                    Object::Integer(255),
                ]),
            ),
        ]));
        assert_eq!(
            out.get("Filter"),
            Some(&Object::Array(vec![
                Object::Name("ASCII85Decode".to_string()),
                Object::Name("FlateDecode".to_string()),
            ]))
        );
        assert_eq!(
            out.get("ColorSpace"),
            Some(&Object::Array(vec![
                Object::Name("Indexed".to_string()),
                Object::Name("DeviceRGB".to_string()),
                Object::Integer(255),
            ]))
        );
    }

    /// A name we do not recognise (a `/Resources /ColorSpace` entry like `/CS0`,
    /// or an already-full name) must pass through untouched.
    #[test]
    fn leaves_unabbreviated_and_named_spaces_alone() {
        let out = expand_inline_image_dict(dict(&[("CS", Object::Name("CS0".to_string()))]));
        assert_eq!(out.get("ColorSpace"), Some(&Object::Name("CS0".to_string())));
        let out = expand_inline_image_dict(dict(&[("CS", Object::Name("DeviceGray".to_string()))]));
        assert_eq!(out.get("ColorSpace"), Some(&Object::Name("DeviceGray".to_string())));
    }

    /// Both forms of one key in the same dictionary (the pdf-association
    /// duplicate-key fixture does this for /F//Filter, /W//Width, /DP//
    /// DecodeParms): the abbreviated form must win, and deterministically —
    /// before, the winner was HashMap iteration order, a fresh hash seed per
    /// process, and the fixture's image count flapped between runs.
    #[test]
    fn abbreviated_key_beats_its_full_twin() {
        let out = expand_inline_image_dict(dict(&[
            ("F", Object::Name("AHx".to_string())),
            ("Filter", Object::Name("A85".to_string())),
            ("W", Object::Integer(20)),
            ("Width", Object::Integer(999)),
            ("DP", Object::Null),
            ("DecodeParms", Object::Integer(15)),
        ]));
        assert_eq!(out.get("Filter"), Some(&Object::Name("ASCIIHexDecode".to_string())));
        assert_eq!(out.get("Width"), Some(&Object::Integer(20)));
        assert_eq!(out.get("DecodeParms"), Some(&Object::Null));
    }
}

#[cfg(test)]
mod indexed_tests {
    use super::*;

    #[test]
    fn expand_indexed_rgb_8bpc() {
        let palette = vec![0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255];
        let raw = vec![0, 1, 2, 3];
        let out = expand_indexed_to_rgb(&raw, &palette, PixelFormat::RGB, 2, 2, 8).unwrap();
        assert_eq!(out, vec![0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255]);
    }

    #[test]
    fn expand_indexed_gray_base_to_rgb() {
        let palette = vec![10, 128, 255];
        let raw = vec![0, 1, 2];
        let out = expand_indexed_to_rgb(&raw, &palette, PixelFormat::Grayscale, 3, 1, 8).unwrap();
        assert_eq!(out, vec![10, 10, 10, 128, 128, 128, 255, 255, 255]);
    }

    #[test]
    fn expand_indexed_out_of_range_index() {
        let palette = vec![10, 20, 30, 40, 50, 60];
        let raw = vec![0, 5];
        let out = expand_indexed_to_rgb(&raw, &palette, PixelFormat::RGB, 2, 1, 8).unwrap();
        assert_eq!(out, vec![10, 20, 30, 0, 0, 0]);
    }

    /// The highest index is the last entry the lookup actually holds, the same boundary the
    /// expander paints black past: `hival` for a full lookup, lower for a short one.
    #[test]
    fn highest_index_is_the_last_entry_the_palette_holds() {
        use crate::object::Object;
        let highest = |hival: i64, lookup: Vec<u8>| {
            let cs = Object::Array(vec![
                Object::Name("Indexed".to_string()),
                Object::Name("DeviceRGB".to_string()),
                Object::Integer(hival),
                Object::String(lookup),
            ]);
            resolve_indexed_palette(None, &cs).unwrap().unwrap().highest_index()
        };
        assert_eq!(highest(15, vec![7; 16 * 3]), 15, "a full lookup ends at hival");
        assert_eq!(
            highest(255, vec![7; 256 * 3]),
            255,
            "a full 256-entry lookup ends at 255"
        );
        assert_eq!(
            highest(255, vec![7; 4 * 3 + 2]),
            3,
            "a short lookup ends at its last whole entry"
        );
        assert_eq!(highest(255, vec![7; 2]), 0, "a lookup with no whole entry clamps to 0");
    }

    #[test]
    fn resolve_indexed_palette_truncates_to_hival() {
        use crate::object::Object;
        // [/Indexed /DeviceRGB 1 <inline palette>] — hival = 1, so 2 entries * 3 = 6 bytes.
        // Provide an oversized 12-byte palette; the extra 6 bytes must be dropped so
        // that indices > hival cannot pick up stray lookup data. ~keep
        let cs = Object::Array(vec![
            Object::Name("Indexed".to_string()),
            Object::Name("DeviceRGB".to_string()),
            Object::Integer(1),
            Object::String(vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120]),
        ]);
        let ir = resolve_indexed_palette(None, &cs).unwrap().unwrap();
        assert_eq!(ir.base_fmt, PixelFormat::RGB);
        assert_eq!(ir.palette, vec![10, 20, 30, 40, 50, 60]);
        assert!(ir.base_profile.is_none(), "DeviceRGB base has no ICC profile");
        let (fmt, palette) = (ir.base_fmt, ir.palette);

        let raw = vec![0, 1, 2];
        let out = expand_indexed_to_rgb(&raw, &palette, fmt, 3, 1, 8).unwrap();
        assert_eq!(out, vec![10, 20, 30, 40, 50, 60, 0, 0, 0]);
    }

    /// GH#1913: a palette over a `/DeviceN` base holds one byte per colorant in each entry, and the
    /// composite reads a two-colorant entry as CMYK with no ink in the missing two.
    #[test]
    fn a_devicen_palette_is_read_one_byte_per_colorant() {
        use crate::object::Object;
        let cs = Object::Array(vec![
            Object::Name("Indexed".to_string()),
            Object::Array(vec![
                Object::Name("DeviceN".to_string()),
                Object::Array(vec![
                    Object::Name("Spot-A".to_string()),
                    Object::Name("Spot-B".to_string()),
                ]),
                Object::Name("DeviceCMYK".to_string()),
                Object::Null,
            ]),
            Object::Integer(1),
            Object::String(vec![0xFF, 0x00, 0x00, 0xFF]),
        ]);
        let ir = resolve_indexed_palette(None, &cs).unwrap().unwrap();
        assert_eq!(ir.components, 2, "one byte per colorant");
        assert_eq!(ir.highest_index(), 1, "two entries");
        let rgb = expand_indexed_image(&[0, 1], &ir, 2, 1, 8, crate::color::RenderingIntent::default()).unwrap();
        assert_eq!(
            rgb[..3],
            cmyk_pixel_to_rgb(0xFF, 0, 0, 0),
            "entry 0 is the first colorant"
        );
        assert_eq!(
            rgb[3..],
            cmyk_pixel_to_rgb(0, 0xFF, 0, 0),
            "entry 1 is the second colorant"
        );
        let base = expand_indexed_to_base_samples(&[0, 1], &ir.palette, ir.components, 2, 1, 8).unwrap();
        assert_eq!(
            base,
            vec![0xFF, 0x00, 0x00, 0xFF],
            "the plates see each entry's own two bytes"
        );
    }

    /// A palette over a `/Lab` base is converted to RGB when it is resolved, and the expander steps
    /// through the converted palette three bytes per entry.
    #[test]
    fn a_lab_palette_is_stepped_as_rgb_after_conversion() {
        use crate::object::Object;
        let (black, white) = ([0u8, 128, 128], [255u8, 128, 128]);
        let cs = Object::Array(vec![
            Object::Name("Indexed".to_string()),
            Object::Array(vec![
                Object::Name("Lab".to_string()),
                Object::Dictionary(std::collections::HashMap::new()),
            ]),
            Object::Integer(1),
            Object::String([black, white].concat()),
        ]);
        let ir = resolve_indexed_palette(None, &cs).unwrap().unwrap();
        assert_eq!(ir.highest_index(), 1, "two entries");
        let d65 = [0.9505, 1.0, 1.0890];
        let rgb = expand_indexed_image(&[0, 1], &ir, 2, 1, 8, crate::color::RenderingIntent::default()).unwrap();
        assert_eq!(rgb[..3], lab_palette_to_rgb(&black, d65)[..], "entry 0 is Lab black");
        assert_eq!(rgb[3..], lab_palette_to_rgb(&white, d65)[..], "entry 1 is Lab white");
        assert_ne!(rgb[..3], rgb[3..], "the two entries differ");
    }

    #[test]
    fn expand_indexed_cmyk_base_matches_cmyk_to_rgb() {
        // Palette has a single CMYK entry; expansion must match the shared helper. ~keep
        let palette = vec![64, 128, 192, 32];
        let raw = vec![0];
        let out = expand_indexed_to_rgb(&raw, &palette, PixelFormat::CMYK, 1, 1, 8).unwrap();
        let expected = cmyk_pixel_to_rgb(64, 128, 192, 32);
        assert_eq!(out, expected.to_vec());
    }

    #[test]
    fn cmyk_pixel_uses_process_inks_not_additive() {
        // DeviceCMYK images convert via the process-ink corners (matching the
        // text/vector paths), NOT the naive additive `1 - min(1, C+K)`. 100% K is
        // the K ink #231F20 (not #000000); process cyan is #00ADEF (not #00FFFF). ~keep
        assert_eq!(cmyk_pixel_to_rgb(0, 0, 0, 255), [0x23, 0x1F, 0x20]);
        assert_eq!(cmyk_pixel_to_rgb(255, 0, 0, 0), [0x00, 0xAD, 0xEF]);
        assert_eq!(cmyk_pixel_to_rgb(0, 0, 0, 0), [255, 255, 255]);
    }

    #[test]
    fn expand_indexed_1bpc_with_row_padding() {
        // 2-entry palette, 5x2 image at 1 bpc. 5 bits → 1 byte per row (3 bits padding).
        // Row 0 indices: 0,1,0,1,0 → top nibble 01010xxx = 0x50
        // Row 1 indices: 1,1,0,0,1 → top nibble 11001xxx = 0xC8 ~keep
        let palette = vec![10, 20, 30, 200, 210, 220];
        let raw = vec![0x50, 0xC8];
        let out = expand_indexed_to_rgb(&raw, &palette, PixelFormat::RGB, 5, 2, 1).unwrap();
        assert_eq!(
            out,
            vec![
                10, 20, 30, 200, 210, 220, 10, 20, 30, 200, 210, 220, 10, 20, 30, 200, 210, 220, 200, 210, 220, 10, 20,
                30, 10, 20, 30, 200, 210, 220,
            ]
        );
    }

    #[test]
    fn expand_indexed_2bpc_with_row_padding() {
        // 4-entry palette, 3x1 image at 2 bpc. 6 bits → 1 byte per row (2 bits padding).
        // indices 0,1,2 → 00 01 10 xx → 0x18 ~keep
        let palette = vec![0, 0, 0, 10, 20, 30, 40, 50, 60, 70, 80, 90];
        let raw = vec![0x18];
        let out = expand_indexed_to_rgb(&raw, &palette, PixelFormat::RGB, 3, 1, 2).unwrap();
        assert_eq!(out, vec![0, 0, 0, 10, 20, 30, 40, 50, 60]);
    }

    #[test]
    fn expand_indexed_4bpc_packs_two_per_byte() {
        // 4x1 image, 4bpc: 2 indices per byte, high nibble first ~keep
        let palette = vec![0, 0, 0, 10, 20, 30, 40, 50, 60, 70, 80, 90];
        // indices: 0,1,2,3 → packed: 0x01, 0x23 ~keep
        let raw = vec![0x01, 0x23];
        let out = expand_indexed_to_rgb(&raw, &palette, PixelFormat::RGB, 4, 1, 4).unwrap();
        assert_eq!(out, vec![0, 0, 0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    }

    #[test]
    fn expand_indexed_rejects_overflow_dimensions() {
        // Dimensions that overflow usize when computing w * h * 3. Previously
        // Vec::with_capacity(w*h*3) would panic or reserve absurd amounts. ~keep
        let palette = vec![0, 0, 0, 255, 0, 0];
        let raw = vec![0, 1];
        let huge = u32::MAX / 2;
        let result = expand_indexed_to_rgb(&raw, &palette, PixelFormat::RGB, huge, huge, 8);
        assert!(result.is_err(), "overflow dimensions must be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("overflow") || err.contains("exceeds"),
            "expected overflow/limit error, got: {err}"
        );
    }

    #[test]
    fn expand_indexed_rejects_truncated_stream() {
        // 10x10 8bpc image requires 100 index bytes. Supplying 10 used to
        // silently zero-pad the remaining rows; now it's an error. ~keep
        let palette = vec![10, 20, 30, 40, 50, 60];
        let raw = vec![0; 10];
        let result = expand_indexed_to_rgb(&raw, &palette, PixelFormat::RGB, 10, 10, 8);
        assert!(result.is_err(), "truncated stream must be rejected");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("truncated"), "expected truncated error, got: {err}");
    }

    #[test]
    fn expand_indexed_rejects_output_over_cap() {
        // 12 000 × 12 000 × 3 = 432 MB > 256 MB guard. The MAX_INDEXED_OUTPUT_BYTES
        // check fires before we inspect `raw.len()`, so the test doesn't need to
        // allocate a 144 MB stream — an empty buffer is enough to prove the cap
        // rejects the request. ~keep
        let palette = vec![0, 0, 0];
        let raw: Vec<u8> = Vec::new();
        let result = expand_indexed_to_rgb(&raw, &palette, PixelFormat::RGB, 12_000, 12_000, 8);
        assert!(result.is_err(), "oversized output must be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("guard limit") || err.contains("exceeds"),
            "expected output-size guard error, got: {err}"
        );
    }

    #[test]
    fn expand_indexed_rejects_bpc_zero() {
        // bpc = 0 used to be coerced to 1 by `bpc.max(1)`, silently
        // accepting a malformed PDF. Now it must be rejected. ~keep
        let palette = vec![0, 0, 0, 255, 0, 0];
        let raw = vec![0xFF];
        let result = expand_indexed_to_rgb(&raw, &palette, PixelFormat::RGB, 1, 1, 0);
        assert!(result.is_err(), "bpc=0 must be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("BitsPerComponent") || err.contains("bpc"),
            "expected bpc error, got: {err}"
        );
    }

    #[test]
    fn expand_indexed_rejects_unsupported_bpc() {
        // 3, 5, 6, 7, 9, 12, 16, … are all invalid for Indexed. Previously
        // the `_ => 0` arm in `read_index` silently mapped every pixel to
        // palette entry 0, returning a solid-color image. Now they're
        // rejected up front. ~keep
        let palette = vec![0, 0, 0, 255, 0, 0];
        let raw = vec![0xFF];
        for bpc in [3u8, 5, 6, 7, 9, 12, 16] {
            let result = expand_indexed_to_rgb(&raw, &palette, PixelFormat::RGB, 1, 1, bpc);
            assert!(result.is_err(), "bpc={bpc} must be rejected");
        }
    }

    #[test]
    fn expand_indexed_accepts_all_spec_bpc_values() {
        let palette = vec![0, 0, 0, 255, 0, 0, 10, 20, 30, 40, 50, 60];
        let raw = vec![0xFF];
        for bpc in [1u8, 2, 4, 8] {
            let result = expand_indexed_to_rgb(&raw, &palette, PixelFormat::RGB, 1, 1, bpc);
            assert!(result.is_ok(), "bpc={bpc} must be accepted, got {result:?}");
        }
    }

    // Regression test. Per ISO 32000-1 §8.6.6.3, the lookup element of
    // `[/Indexed base hival lookup]` must be either a byte string or a stream.
    // Historical behaviour when it was neither: `resolve_indexed_palette` returned
    // `Ok(None)` and `extract_image_from_xobject` silently fell back to treating
    // the raw 1-byte/pixel index stream as 3-byte/pixel RGB, producing the
    // misleading "Invalid RGB image dimensions" error. The fix returns an
    // explicit `Error::Image("Unable to resolve Indexed color space palette")`. ~keep
    #[test]
    fn resolve_indexed_palette_array_lookup_returns_none() {
        use crate::object::Object;
        let cs = Object::Array(vec![
            Object::Name("Indexed".to_string()),
            Object::Name("DeviceRGB".to_string()),
            Object::Integer(1),
            Object::Array(vec![
                Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Integer(0)]),
                Object::Array(vec![Object::Integer(255), Object::Integer(255), Object::Integer(255)]),
            ]),
        ]);
        assert!(resolve_indexed_palette(None, &cs).unwrap().is_none());
    }

    #[test]
    fn extract_image_errors_when_indexed_lookup_is_array() {
        use crate::object::Object;
        use std::collections::HashMap;

        let mut dict = HashMap::new();
        dict.insert("Subtype".to_string(), Object::Name("Image".to_string()));
        dict.insert("Width".to_string(), Object::Integer(2));
        dict.insert("Height".to_string(), Object::Integer(1));
        dict.insert("BitsPerComponent".to_string(), Object::Integer(8));
        dict.insert(
            "ColorSpace".to_string(),
            Object::Array(vec![
                Object::Name("Indexed".to_string()),
                Object::Name("DeviceRGB".to_string()),
                Object::Integer(1),
                Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Integer(0)]),
            ]),
        );
        let xobject = Object::Stream {
            dict,
            data: bytes::Bytes::from_static(&[0, 1]),
        };

        let err =
            extract_image_from_xobject(None, &xobject, None, None).expect_err("Indexed with Array lookup must error");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("Unable to resolve Indexed color space palette"),
            "error message should identify palette-resolution failure, got: {msg}"
        );
        assert!(
            !msg.contains("Invalid RGB image dimensions"),
            "must not fall through to misleading RGB-dimension error, got: {msg}"
        );
    }

    #[test]
    fn lab_pixel_mid_gray() {
        // Lab(50, 0, 0) = perceptual mid-gray → sRGB ~(119, 119, 119).
        // Byte encoding: L=128, a=128, b=128. ~keep
        let d65: [f64; 3] = [0.9505, 1.0, 1.0890];
        let [r, g, b] = super::lab_pixel_to_rgb(128, 128, 128, d65);
        for (label, v, expected) in [("R", r, 119), ("G", g, 119), ("B", b, 119)] {
            let diff = (v as i32 - expected).abs();
            assert!(
                diff <= 3,
                "Lab(50,0,0) {label}: expected ~{expected}, got {v} (Δ={diff})"
            );
        }
    }

    #[test]
    fn lab_pixel_white() {
        // Lab(100, 0, 0) = white → sRGB ~(255, 255, 255).
        // Byte encoding: L=255, a=128, b=128. ~keep
        let d65: [f64; 3] = [0.9505, 1.0, 1.0890];
        let [r, g, b] = super::lab_pixel_to_rgb(255, 128, 128, d65);
        for (label, v) in [("R", r), ("G", g), ("B", b)] {
            assert!(v >= 250, "Lab(100,0,0) {label}: expected ~255, got {v}");
        }
    }

    #[test]
    fn lab_pixel_black() {
        // Lab(0, 0, 0) = black → sRGB ~(0, 0, 0).
        // Byte encoding: L=0, a=128, b=128. ~keep
        let d65: [f64; 3] = [0.9505, 1.0, 1.0890];
        let [r, g, b] = super::lab_pixel_to_rgb(0, 128, 128, d65);
        for (label, v) in [("R", r), ("G", g), ("B", b)] {
            assert!(v <= 5, "Lab(0,0,0) {label}: expected ~0, got {v}");
        }
    }

    #[test]
    fn lab_pixel_red_tint() {
        // Lab(50, 80, 0) has a strong red-magenta tint.
        // Byte encoding: L=128, a=208 (128+80), b=128. ~keep
        let d65: [f64; 3] = [0.9505, 1.0, 1.0890];
        let [r, g, b] = super::lab_pixel_to_rgb(128, 208, 128, d65);
        assert!(r > g + 50, "Lab(50,80,0) should have R >> G: R={r}, G={g}");
        assert!(r > b, "Lab(50,80,0) should have R > B: R={r}, B={b}");
    }

    #[test]
    fn lab_palette_round_trip() {
        let d65: [f64; 3] = [0.9505, 1.0, 1.0890];
        let palette: Vec<u8> = vec![0, 128, 128, 128, 128, 128, 255, 128, 128];
        let rgb = super::lab_palette_to_rgb(&palette, d65);
        assert_eq!(rgb.len(), 9, "3 Lab entries → 9 RGB bytes");
        assert!(rgb[0] <= 5 && rgb[1] <= 5 && rgb[2] <= 5);
        assert!(rgb[6] >= 250 && rgb[7] >= 250 && rgb[8] >= 250);
    }

    #[test]
    fn extract_lab_whitepoint_d65() {
        use crate::object::Object;
        let cs = Object::Array(vec![
            Object::Name("Lab".to_string()),
            Object::Dictionary({
                let mut d = std::collections::HashMap::new();
                d.insert(
                    "WhitePoint".to_string(),
                    Object::Array(vec![Object::Real(0.9505), Object::Real(1.0), Object::Real(1.0890)]),
                );
                d
            }),
        ]);
        let wp = super::extract_lab_whitepoint(&cs);
        assert!((wp[0] - 0.9505).abs() < 1e-6);
        assert!((wp[1] - 1.0).abs() < 1e-6);
        assert!((wp[2] - 1.0890).abs() < 1e-6);
    }

    #[test]
    fn extract_lab_whitepoint_missing_falls_back_to_d65() {
        use crate::object::Object;
        let cs = Object::Name("Lab".to_string());
        let wp = super::extract_lab_whitepoint(&cs);
        assert!((wp[0] - 0.9505).abs() < 1e-6);
    }
}

/// A PDF stream filter as stored in the `/Filter` key of an image XObject.
///
/// Knowing the filter chain lets callers decide whether to decode (e.g. skip
/// decompression for JPEG re-embed pipelines that only need `raw_compressed_bytes`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum PdfFilter {
    /// JPEG (DCTDecode) — compressed bytes are a valid JPEG file.
    DCTDecode,
    /// JPEG 2000 (JPXDecode).
    JPXDecode,
    /// Deflate/zlib (FlateDecode).
    FlateDecode,
    /// LZW compression (LZWDecode).
    LZWDecode,
    /// CCITT Group 3/4 fax (CCITTFaxDecode).
    CCITTFaxDecode,
    /// JBIG2 bi-level compression.
    JBIG2Decode,
    /// ASCII hex encoding (ASCIIHexDecode).
    ASCIIHexDecode,
    /// ASCII base-85 encoding (ASCII85Decode).
    ASCII85Decode,
    /// Run-length encoding (RunLengthDecode).
    RunLengthDecode,
    /// Crypt filter (used with encrypted streams).
    Crypt,
    /// Any filter not listed above; carries the raw PDF name.
    Other(String),
}

impl PdfFilter {
    /// Map a PDF filter name (or its abbreviated form) to a `PdfFilter` variant.
    pub fn from_name(name: &str) -> Self {
        match name {
            "DCTDecode" | "DCT" => PdfFilter::DCTDecode,
            "JPXDecode" => PdfFilter::JPXDecode,
            "FlateDecode" | "Fl" => PdfFilter::FlateDecode,
            "LZWDecode" | "LZW" => PdfFilter::LZWDecode,
            "CCITTFaxDecode" | "CCF" => PdfFilter::CCITTFaxDecode,
            "JBIG2Decode" => PdfFilter::JBIG2Decode,
            "ASCIIHexDecode" | "AHx" => PdfFilter::ASCIIHexDecode,
            "ASCII85Decode" | "A85" => PdfFilter::ASCII85Decode,
            "RunLengthDecode" | "RL" => PdfFilter::RunLengthDecode,
            "Crypt" => PdfFilter::Crypt,
            other => PdfFilter::Other(other.to_string()),
        }
    }
}

/// Parses the `/Filter` entry of an image dictionary into a `Vec<PdfFilter>`.
///
/// The spec allows either a single name (`/DCTDecode`) or an array of names
/// (`[/ASCII85Decode /FlateDecode]`).
pub(crate) fn parse_filter_chain(dict: &std::collections::HashMap<String, crate::object::Object>) -> Vec<PdfFilter> {
    use crate::object::Object;
    match dict.get("Filter") {
        Some(Object::Name(n)) => vec![PdfFilter::from_name(n)],
        Some(Object::Array(arr)) => arr
            .iter()
            .filter_map(|o| o.as_name())
            .map(PdfFilter::from_name)
            .collect(),
        _ => vec![],
    }
}

/// Internal image source stored inside a [`PdfImageHandle`].
#[derive(Clone)]
enum PdfImageSource {
    /// Indirect Image XObject reference; loaded on demand.
    XObject(ObjectRef),
    /// Inline image: pre-built `Object::Stream` plus the raw compressed bytes.
    Inline {
        /// Synthetic `Object::Stream` built from the inline dict + data —
        /// ready to pass directly to `extract_image_from_xobject`.
        stream_object: crate::object::Object,
        /// Raw compressed bytes as they appeared between `ID` and `EI`.
        /// Stored as `bytes::Bytes` (cheaply cloneable, refcounted) so that
        /// the same allocation can be shared with the Stream data field
        /// without duplicating a potentially large JPEG/JBIG2/etc payload.
        compressed_bytes: bytes::Bytes,
    },
}

/// A lightweight handle to a PDF image that has **not** been decoded yet.
///
/// Created by [`crate::PdfDocument::page_image_handles`], which walks the page content
/// stream and reads XObject dictionary metadata without decompressing any stream.
/// Callers can inspect the metadata fields to decide which images to materialise,
/// then call [`decode`](PdfImageHandle::decode) or
/// [`raw_compressed_bytes`](PdfImageHandle::raw_compressed_bytes) only on those
/// they actually need.
///
/// # Example
///
/// ```no_run
/// # use xberg_native_pdf::PdfDocument;
/// # let bytes = std::fs::read("page.pdf").unwrap();
/// let doc = PdfDocument::from_bytes(bytes).unwrap();
/// // Phase 1: enumerate without decompression
/// let handles = doc.page_image_handles(0).unwrap();
/// // Phase 2: decode only images larger than a thumbnail
/// let images: Vec<_> = handles
///     .into_iter()
///     .filter(|h| h.width >= 200 && h.height >= 200)
///     .map(|h| h.decode())
///     .collect::<Result<_, _>>()
///     .unwrap();
/// ```
#[derive(Clone)]
#[non_exhaustive]
pub struct PdfImageHandle<'doc> {
    /// Image width in pixels (from XObject `/Width`).
    pub width: u32,
    /// Image height in pixels (from XObject `/Height`).
    pub height: u32,
    /// Colour space (from XObject `/ColorSpace`).
    pub color_space: ColorSpace,
    /// Bits per component (from XObject `/BitsPerComponent`).
    pub bits_per_component: u8,
    /// Compressed stream length in bytes (from XObject `/Length`).
    ///
    /// For inline images this is `data.len()` as stored between `ID` and `EI`.
    pub byte_size_compressed: u64,
    /// Ordered list of filters applied to the stream (outermost first).
    pub filter_chain: Vec<PdfFilter>,
    /// `true` if the image is an inline image (embedded in the content stream).
    pub is_inline: bool,
    /// Zero-based index of this image among all images painted on the page,
    /// in content-stream paint order.
    pub paint_order: usize,
    /// Axis-aligned bounding box of this image in PDF user space, computed
    /// during Phase 1 by applying the current transformation matrix to the
    /// unit rectangle `[0,0,1,1]`.
    pub bbox: crate::geometry::Rect,
    /// Rotation angle in degrees (0, 90, 180, or 270), derived from the CTM
    /// during Phase 1.
    pub rotation_degrees: f32,

    ctm: crate::content::Matrix,
    doc: &'doc crate::document::PdfDocument,
    source: PdfImageSource,
    /// Active resource `/ColorSpace` subdictionary (name → resolved Object) for
    /// this image's scope, so `decode()` can resolve a resource-name
    /// `/ColorSpace` (e.g. `/CS0`) the same way the renderer does. Empty when the
    /// image's colour space needs no resource lookup (the common case), which
    /// preserves the original `color_space_map=None` decode path.
    color_space_resources: std::collections::HashMap<String, crate::object::Object>,
    /// For an Indexed image (`[/Indexed base hival lookup]`, §8.6.6.3), the
    /// resolved de-indexed *base* colour space; `None` for non-Indexed images.
    indexed_base: Option<ColorSpace>,
}

impl<'doc> PdfImageHandle<'doc> {
    /// Decode this image into a [`PdfImage`].
    ///
    /// This is the expensive operation: it decompresses the image stream,
    /// decodes pixels, and applies colour-space conversions as needed.
    ///
    /// Takes `&self` so a single handle supports a two-phase inspect → raw →
    /// decode flow ([`raw_compressed_bytes`](Self::raw_compressed_bytes) then
    /// `decode`) without re-enumerating the page.
    pub fn decode(&self) -> Result<PdfImage> {
        use crate::extractors::extract_image_from_xobject;

        let xobject_for_extract;
        let (obj, obj_ref) = match &self.source {
            PdfImageSource::XObject(obj_ref) => {
                xobject_for_extract = self.doc.load_object(*obj_ref)?;
                (&xobject_for_extract, Some(*obj_ref))
            }
            PdfImageSource::Inline { stream_object, .. } => (stream_object, None),
        };

        // Pass the active resource ColorSpace map so a resource-name
        // `/ColorSpace` (e.g. `/CS0`) resolves the same way the renderer does.
        // `None` when empty preserves the original decode path for the common
        // case where the image's colour space needs no resource lookup. ~keep
        let cs_map = if self.color_space_resources.is_empty() {
            None
        } else {
            Some(&self.color_space_resources)
        };
        let mut image = extract_image_from_xobject(Some(self.doc), obj, obj_ref, cs_map)?;

        // Use pre-computed bbox and rotation from Phase 1 — no need to call
        // back into document.rs helpers here. ~keep
        image.set_bbox(self.bbox);
        image.set_matrix([self.ctm.a, self.ctm.b, self.ctm.c, self.ctm.d, self.ctm.e, self.ctm.f]);
        image.set_rotation_degrees(self.rotation_degrees as i32);

        Ok(image)
    }

    /// Return the raw compressed bytes exactly as stored in the PDF stream,
    /// **without** decompressing them.
    ///
    /// For JPEG images (`filter_chain == [DCTDecode]`) these bytes form a valid
    /// JPEG file and can be written directly to disk or forwarded to a downstream
    /// pipeline without recompression.
    ///
    /// Takes `&self` so it can be combined with [`decode`](Self::decode) on the
    /// same handle (inspect → raw → decode) without re-enumerating the page.
    pub fn raw_compressed_bytes(&self) -> Result<Vec<u8>> {
        match &self.source {
            PdfImageSource::XObject(obj_ref) => {
                let obj = self.doc.load_object(*obj_ref)?;
                match obj {
                    crate::object::Object::Stream { data, .. } => Ok(data.to_vec()),
                    _ => Err(crate::error::Error::Image("XObject is not a stream".to_string())),
                }
            }
            PdfImageSource::Inline { compressed_bytes, .. } => Ok(compressed_bytes.to_vec()),
        }
    }

    /// For an Indexed image, the de-indexed *base* colour space.
    ///
    /// When [`color_space`](Self::color_space) is [`ColorSpace::Indexed`] the
    /// image samples are single palette indices (`components() == 1`) into an
    /// `[/Indexed base hival lookup]` array (§8.6.6.3); this returns the resolved
    /// `base` colour space — the space in which the de-indexed output pixels are
    /// expressed. Returns `None` for every non-Indexed image (and only if an
    /// Indexed `base` could not be parsed at all).
    pub fn indexed_base(&self) -> Option<ColorSpace> {
        self.indexed_base
    }
}

impl std::fmt::Debug for PdfImageHandle<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdfImageHandle")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("color_space", &self.color_space)
            .field("bits_per_component", &self.bits_per_component)
            .field("byte_size_compressed", &self.byte_size_compressed)
            .field("filter_chain", &self.filter_chain)
            .field("is_inline", &self.is_inline)
            .field("paint_order", &self.paint_order)
            .field("bbox", &self.bbox)
            .field("rotation_degrees", &self.rotation_degrees)
            .field("color_space_resources", &self.color_space_resources)
            .field("indexed_base", &self.indexed_base)
            .finish_non_exhaustive()
    }
}

/// Derive a rotation angle in degrees from a transformation matrix.
///
/// Computes `atan2(b, a)` and rounds to the nearest integer degree.
fn matrix_to_rotation(m: crate::content::Matrix) -> f32 {
    let angle_rad = m.b.atan2(m.a);
    let angle_deg = angle_rad.to_degrees();
    let normalized = angle_deg % 360.0;
    if normalized < 0.0 {
        normalized + 360.0
    } else {
        normalized
    }
}

/// Transform an axis-aligned bounding rectangle by a CTM.
///
/// Transforms all four corners and returns the axis-aligned bounding box of the
/// result, which correctly handles rotation, shear, and negative scaling.
fn transform_bbox_with_ctm(rect: &crate::geometry::Rect, ctm: crate::content::Matrix) -> crate::geometry::Rect {
    let x0 = rect.x;
    let y0 = rect.y;
    let x1 = rect.x + rect.width;
    let y1 = rect.y + rect.height;

    let tx0 = ctm.a * x0 + ctm.c * y0 + ctm.e;
    let ty0 = ctm.b * x0 + ctm.d * y0 + ctm.f;

    let tx1 = ctm.a * x1 + ctm.c * y0 + ctm.e;
    let ty1 = ctm.b * x1 + ctm.d * y0 + ctm.f;

    let tx2 = ctm.a * x0 + ctm.c * y1 + ctm.e;
    let ty2 = ctm.b * x0 + ctm.d * y1 + ctm.f;

    let tx3 = ctm.a * x1 + ctm.c * y1 + ctm.e;
    let ty3 = ctm.b * x1 + ctm.d * y1 + ctm.f;

    let min_x = tx0.min(tx1).min(tx2).min(tx3);
    let max_x = tx0.max(tx1).max(tx2).max(tx3);
    let min_y = ty0.min(ty1).min(ty2).min(ty3);
    let max_y = ty0.max(ty1).max(ty2).max(ty3);

    crate::geometry::Rect {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        height: max_y - min_y,
    }
}

/// Resolve a handle's reported colour space from a raw `/ColorSpace` entry.
///
/// Shared by the XObject and inline handle builders so the two paths cannot
/// drift. Returns the `(color_space, indexed_base)` pair stored on the handle:
///
/// - **Resource-name resolution.** If `entry` is an `Object::Name` that is *not*
///   one of the standard device spaces (`DeviceGray`/`DeviceRGB`/`DeviceCMYK`/
///   `Pattern`, which "always identify the corresponding colour spaces directly"
///   and "never refer to resources", §8.6.3/§8.9.7), it is looked up in
///   `color_space_resources` (the active `/Resources/ColorSpace` subdictionary),
///   mirroring the renderer / `extract_image_from_xobject`.
/// - **Indirect-ref hop.** If the entry (or looked-up value) is a `Reference`,
///   one `load_object` hop is resolved via `doc`.
/// - **Indexed (§8.6.6.3).** When the resolved object is an
///   `[/Indexed base hival lookup]` array, returns `(ColorSpace::Indexed,
///   Some(base))` with the de-indexed `base` resolved (including an indirect
///   `base` ref and inner refs, e.g. `[/ICCBased <stream_ref>]`), reusing the
///   same base-resolution logic as `resolve_indexed_palette`. If the base cannot
///   be parsed, returns `(ColorSpace::Indexed, None)`.
/// - Otherwise returns `(parse_color_space(resolved)?, None)`, falling back to
///   `DeviceRGB` on parse failure.
fn resolve_color_space_for_handle(
    entry: &crate::object::Object,
    color_space_resources: &std::collections::HashMap<String, crate::object::Object>,
    doc: Option<&crate::document::PdfDocument>,
) -> (ColorSpace, Option<ColorSpace>) {
    use crate::object::Object;

    // (a) Resource-name → resolved Object (skip standard device names, which
    // always identify their space directly and never refer to resources). ~keep
    let after_name = match entry {
        Object::Name(name) if !matches!(name.as_str(), "DeviceGray" | "DeviceRGB" | "DeviceCMYK" | "Pattern") => {
            color_space_resources
                .get(name)
                .cloned()
                .unwrap_or_else(|| entry.clone())
        }
        _ => entry.clone(),
    };

    let resolved = match (after_name.as_reference(), doc) {
        (Some(r), Some(d)) => d.load_object(r).unwrap_or(after_name),
        _ => after_name,
    };

    // (c) `[/Indexed base hival lookup]` (§8.6.6.3): report Indexed + base. ~keep
    if let Object::Array(arr) = &resolved
        && arr.first().and_then(|o| o.as_name()) == Some("Indexed")
        && arr.len() >= 2
    {
        // Resolve the base object, mirroring resolve_indexed_palette: an
        // indirect base ref, plus inner refs of an array base such as
        // [/ICCBased <stream_ref>] so parse_color_space can read /N. ~keep
        let base_obj = resolve_indexed_base_obj_for_handle(doc, arr);
        let base = parse_color_space(&base_obj).ok();
        return (ColorSpace::Indexed, base);
    }

    match parse_color_space(&resolved) {
        Ok(cs) => (cs, None),
        Err(_) => (ColorSpace::DeviceRGB, None),
    }
}

/// Resolve `arr[1]`, the base color-space object of an `[/Indexed base hival
/// lookup]` array, for [`resolve_color_space_for_handle`]. Unlike
/// [`resolve_indexed_base`], a `load_object` failure falls back to the
/// unresolved reference rather than propagating an error, since this helper
/// reports a best-effort `(ColorSpace, Option<ColorSpace>)` pair rather than
/// a `Result`.
fn resolve_indexed_base_obj_for_handle(
    doc: Option<&crate::document::PdfDocument>,
    arr: &[crate::object::Object],
) -> crate::object::Object {
    use crate::object::Object;

    let Some(d) = doc else {
        return arr[1].clone();
    };
    let outer = if let Some(r) = arr[1].as_reference() {
        d.load_object(r).unwrap_or_else(|_| arr[1].clone())
    } else {
        arr[1].clone()
    };
    let Object::Array(mut inner) = outer else {
        return outer;
    };
    for item in inner.iter_mut() {
        if let Some(r) = item.as_reference()
            && let Ok(resolved) = d.load_object(r)
        {
            *item = resolved;
        }
    }
    Object::Array(inner)
}

/// Build a `PdfImageHandle` from an Image XObject dictionary entry.
///
/// Returns `None` if the XObject reference cannot be resolved or the dict lacks
/// required fields (`Width`, `Height`), or if those fields contain non-positive
/// values.
pub(crate) fn image_handle_from_xobject<'doc>(
    doc: &'doc crate::document::PdfDocument,
    obj_ref: ObjectRef,
    xobject_dict: &std::collections::HashMap<String, crate::object::Object>,
    ctm: crate::content::Matrix,
    paint_order: usize,
    color_space_resources: &std::collections::HashMap<String, crate::object::Object>,
) -> Option<PdfImageHandle<'doc>> {
    // /Width and /Height may be indirect references (ISO 32000-1 §7.3.10);
    // resolve them the same way `extract_image_from_xobject` does. ~keep
    let resolve_int = |o: &crate::object::Object| -> Option<i64> {
        match o.as_reference() {
            Some(r) => doc.load_object(r).ok().and_then(|v| v.as_integer()),
            None => o.as_integer(),
        }
    };
    let w = xobject_dict
        .get("Width")
        .and_then(resolve_int)
        .filter(|&n| n > 0)
        .map(|n| n as u32)?;
    let h = xobject_dict
        .get("Height")
        .and_then(resolve_int)
        .filter(|&n| n > 0)
        .map(|n| n as u32)?;
    // /BitsPerComponent and /Length are likewise permitted to be indirect
    // (§7.3.10); /Length in particular is routinely indirect in the wild
    // (issue #1444's fixture has `/Length 9 0 R`). ~keep
    let is_image_mask = xobject_dict
        .get("ImageMask")
        .is_some_and(|value| matches!(value, crate::object::Object::Boolean(true)));
    let bpc = xobject_dict
        .get("BitsPerComponent")
        .and_then(resolve_int)
        .unwrap_or(if is_image_mask { 1 } else { 8 }) as u8;
    let byte_size = xobject_dict
        .get("Length")
        .and_then(resolve_int)
        .filter(|&n| n >= 0)
        .map(|n| n as u64)
        .unwrap_or(0);
    let filter_chain = parse_filter_chain(xobject_dict);

    // Resolve the reported colour space via the shared helper: resource-name →
    // map entry, one indirect-ref hop, and `[/Indexed base ...]` (§8.6.6.3) →
    // `Indexed` + de-indexed base. `/ColorSpace` is forbidden on an `/ImageMask`
    // (§8.9.6.2), so route the same placeholder `resolve_image_xobject_metadata`
    // uses through the helper when it is absent -- GH#1825: a plain DeviceRGB
    // default here disagreed with the DeviceGray the extraction path reports for
    // the same image. ~keep
    let missing_color_space_default = if is_image_mask {
        crate::object::Object::Name("DeviceGray".to_string())
    } else {
        crate::object::Object::Name("DeviceRGB".to_string())
    };
    let (color_space, indexed_base) = match xobject_dict.get("ColorSpace") {
        Some(entry) => resolve_color_space_for_handle(entry, color_space_resources, Some(doc)),
        None => resolve_color_space_for_handle(&missing_color_space_default, color_space_resources, Some(doc)),
    };

    // Compute bbox and rotation in Phase 1 while the CTM is in scope. ~keep
    let unit_rect = crate::geometry::Rect::new(0.0, 0.0, 1.0, 1.0);
    let bbox = transform_bbox_with_ctm(&unit_rect, ctm);
    let rotation_degrees = matrix_to_rotation(ctm);

    Some(PdfImageHandle {
        width: w,
        height: h,
        color_space,
        bits_per_component: bpc,
        byte_size_compressed: byte_size,
        filter_chain,
        is_inline: false,
        paint_order,
        bbox,
        rotation_degrees,
        ctm,
        doc,
        source: PdfImageSource::XObject(obj_ref),
        color_space_resources: color_space_resources.clone(),
        indexed_base,
    })
}

/// Read `/Width`, `/Height`, and `/BitsPerComponent` from an inline image's
/// expanded dictionary. Per §8.9.7 these are direct objects, unlike the
/// indirect-reference-tolerant XObject path. Returns `None` when `/Width` or
/// `/Height` is missing or non-positive.
fn inline_image_dimensions(
    expanded: &std::collections::HashMap<String, crate::object::Object>,
) -> Option<(u32, u32, u8)> {
    let w = expanded
        .get("Width")
        .and_then(|o| o.as_integer())
        .filter(|&n| n > 0)
        .map(|n| n as u32)?;
    let h = expanded
        .get("Height")
        .and_then(|o| o.as_integer())
        .filter(|&n| n > 0)
        .map(|n| n as u32)?;
    let bpc = expanded
        .get("BitsPerComponent")
        .and_then(|o| o.as_integer())
        .unwrap_or(8) as u8;
    Some((w, h, bpc))
}

/// Build a `PdfImageHandle` from an inline image (`BI`/`ID`/`EI` sequence).
pub(crate) fn image_handle_from_inline<'doc>(
    doc: &'doc crate::document::PdfDocument,
    dict: &std::collections::HashMap<String, crate::object::Object>,
    data: Vec<u8>,
    ctm: crate::content::Matrix,
    paint_order: usize,
    color_space_resources: &std::collections::HashMap<String, crate::object::Object>,
) -> Option<PdfImageHandle<'doc>> {
    use crate::object::Object;

    let expanded = crate::extractors::expand_inline_image_dict(dict.clone());

    // Unlike an Image XObject's dictionary, an inline image's dictionary entries
    // (`/W`, `/H`, `/BPC`, ...) must be direct objects per §8.9.7 — the content
    // stream that carries them has no object numbers to reference. `/ColorSpace`
    // is the one entry the spec still lets name an external resource, and that
    // narrow exception is handled by `resolve_color_space_for_handle` below.
    // These bare reads of `/Width`, `/Height` and `/BitsPerComponent` are
    // therefore deliberate, not an oversight of the indirect-reference sweep
    // applied to the XObject paths elsewhere in this file. ~keep
    let (w, h, bpc) = inline_image_dimensions(&expanded)?;
    let byte_size = data.len() as u64;
    let filter_chain = parse_filter_chain(&expanded);

    // Resolve the reported colour space via the same shared helper as the
    // XObject path so the two agree. For inline images a resource-name
    // `/ColorSpace` into `/Resources/ColorSpace` is explicitly legal (§8.9.7),
    // and `[/Indexed base ...]` (§8.6.6.3) reports `Indexed` + de-indexed base.
    // `/ColorSpace` is forbidden on an `/ImageMask` (§8.9.6.2) just as it is for
    // an XObject mask, so the missing-entry default mirrors that site's
    // GH#1825 fix rather than defaulting to DeviceRGB unconditionally. ~keep
    let is_image_mask = expanded
        .get("ImageMask")
        .is_some_and(|value| matches!(value, Object::Boolean(true)));
    let missing_color_space_default = if is_image_mask {
        Object::Name("DeviceGray".to_string())
    } else {
        Object::Name("DeviceRGB".to_string())
    };
    let (color_space, indexed_base) = match expanded.get("ColorSpace") {
        Some(entry) => resolve_color_space_for_handle(entry, color_space_resources, Some(doc)),
        None => resolve_color_space_for_handle(&missing_color_space_default, color_space_resources, Some(doc)),
    };

    // Compute bbox and rotation in Phase 1 while the CTM is in scope. ~keep
    let unit_rect = crate::geometry::Rect::new(0.0, 0.0, 1.0, 1.0);
    let bbox = transform_bbox_with_ctm(&unit_rect, ctm);
    let rotation_degrees = matrix_to_rotation(ctm);

    // Build a synthetic Object::Stream so decode() can call extract_image_from_xobject.
    // Share a single Bytes allocation between the Stream (for decode) and the
    // handle (for raw_compressed_bytes). Bytes is refcounted, so this avoids
    // duplicating potentially large image payloads (e.g. 10 MB JPEG → 20 MB RSS). ~keep
    let mut stream_dict = expanded;
    stream_dict.insert("Subtype".to_string(), Object::Name("Image".to_string()));
    let compressed_bytes = bytes::Bytes::from(data);
    let stream_object = Object::Stream {
        dict: stream_dict,
        data: compressed_bytes.clone(),
    };

    Some(PdfImageHandle {
        width: w,
        height: h,
        color_space,
        bits_per_component: bpc,
        byte_size_compressed: byte_size,
        filter_chain,
        is_inline: true,
        paint_order,
        bbox,
        rotation_degrees,
        ctm,
        doc,
        source: PdfImageSource::Inline {
            stream_object,
            compressed_bytes,
        },
        color_space_resources: color_space_resources.clone(),
        indexed_base,
    })
}

#[cfg(test)]
mod handle_color_space_tests {
    use super::*;
    use crate::object::{Object, ObjectRef};
    use std::collections::HashMap;

    fn empty_map() -> HashMap<String, Object> {
        HashMap::new()
    }

    /// `[/Indexed /DeviceRGB 1 <00 00 00 FF FF FF>]` as a direct array.
    fn direct_indexed_rgb() -> Object {
        Object::Array(vec![
            Object::Name("Indexed".to_string()),
            Object::Name("DeviceRGB".to_string()),
            Object::Integer(1),
            Object::String(vec![0, 0, 0, 255, 255, 255]),
        ])
    }

    #[test]
    fn helper_direct_indexed_reports_indexed_with_rgb_base() {
        let entry = direct_indexed_rgb();
        let (cs, base) = resolve_color_space_for_handle(&entry, &empty_map(), None);
        assert_eq!(cs, ColorSpace::Indexed);
        assert_eq!(base, Some(ColorSpace::DeviceRGB));
    }

    #[test]
    fn helper_resource_name_resolves_to_device_gray() {
        // `/CS0` → /DeviceGray via the resource map (not a standard device name,
        // so it is looked up). ~keep
        let mut map = empty_map();
        map.insert("CS0".to_string(), Object::Name("DeviceGray".to_string()));
        let entry = Object::Name("CS0".to_string());
        let (cs, base) = resolve_color_space_for_handle(&entry, &map, None);
        assert_eq!(cs, ColorSpace::DeviceGray);
        assert_eq!(base, None);
    }

    #[test]
    fn helper_resource_name_resolves_to_indexed() {
        let mut map = empty_map();
        map.insert("CS0".to_string(), direct_indexed_rgb());
        let entry = Object::Name("CS0".to_string());
        let (cs, base) = resolve_color_space_for_handle(&entry, &map, None);
        assert_eq!(cs, ColorSpace::Indexed);
        assert_eq!(base, Some(ColorSpace::DeviceRGB));
    }

    #[test]
    fn helper_standard_device_name_not_resource_resolved() {
        // A standard device name must resolve directly and never be looked up,
        // even if the map (incorrectly) shadows it. ~keep
        let mut map = empty_map();
        map.insert("DeviceRGB".to_string(), Object::Name("DeviceGray".to_string()));
        let entry = Object::Name("DeviceRGB".to_string());
        let (cs, base) = resolve_color_space_for_handle(&entry, &map, None);
        assert_eq!(cs, ColorSpace::DeviceRGB);
        assert_eq!(base, None);
    }

    /// Build a PDF whose object `99 0 obj` is the given colour-space object, so
    /// `resolve_color_space_for_handle(99 0 R, .., Some(doc))` can resolve the
    /// indirect-ref hop. The catalog/pages are minimal placeholders.
    fn pdf_with_cs_object(cs_body: &str) -> Vec<u8> {
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets: Vec<(u32, usize)> = Vec::new();

        offsets.push((1, pdf.len()));
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        offsets.push((2, pdf.len()));
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

        offsets.push((3, pdf.len()));
        pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 1 1] >>\nendobj\n");

        offsets.push((99, pdf.len()));
        pdf.extend_from_slice(format!("99 0 obj\n{}\nendobj\n", cs_body).as_bytes());

        let xref_off = pdf.len();
        // Emit a single contiguous xref subsection covering 0..=99 with most
        // entries free; only the objects we wrote are marked in-use. ~keep
        let max_id = 99u32;
        pdf.extend_from_slice(format!("xref\n0 {}\n", max_id + 1).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for id in 1..=max_id {
            if let Some((_, off)) = offsets.iter().find(|(oid, _)| *oid == id) {
                pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
            } else {
                pdf.extend_from_slice(b"0000000000 65535 f \n");
            }
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                max_id + 1,
                xref_off
            )
            .as_bytes(),
        );
        pdf
    }

    #[test]
    fn helper_indirect_indexed_array_de_indexed() {
        let doc =
            crate::document::PdfDocument::from_bytes(pdf_with_cs_object("[/Indexed /DeviceRGB 1 <000000FFFFFF>]"))
                .expect("synthetic pdf parses");
        let entry = Object::Reference(ObjectRef::new(99, 0));
        let (cs, base) = resolve_color_space_for_handle(&entry, &empty_map(), Some(&doc));
        assert_eq!(cs, ColorSpace::Indexed);
        assert_eq!(base, Some(ColorSpace::DeviceRGB));
    }

    #[test]
    fn helper_indirect_plain_device_gray() {
        let doc = crate::document::PdfDocument::from_bytes(pdf_with_cs_object("/DeviceGray")).unwrap();
        let entry = Object::Reference(ObjectRef::new(99, 0));
        let (cs, base) = resolve_color_space_for_handle(&entry, &empty_map(), Some(&doc));
        assert_eq!(cs, ColorSpace::DeviceGray);
        assert_eq!(base, None);
    }

    #[test]
    fn inline_indexed_consistent_with_xobject() {
        // The inline path uses the same helper, so a direct Indexed array must
        // report `Indexed` + DeviceRGB base — identical to the XObject contract. ~keep
        let entry = direct_indexed_rgb();
        let (xobj_cs, xobj_base) = resolve_color_space_for_handle(&entry, &empty_map(), None);
        let (inline_cs, inline_base) = resolve_color_space_for_handle(&entry, &empty_map(), None);
        assert_eq!(xobj_cs, inline_cs);
        assert_eq!(xobj_base, inline_base);
        assert_eq!(inline_cs, ColorSpace::Indexed);
        assert_eq!(inline_base, Some(ColorSpace::DeviceRGB));
    }

    /// Build a PDF with an image XObject (object 99) so a handle can be built
    /// against a real document and `decode()` exercised. The image is a 1×1
    /// uncompressed sample; `cs_name` is written verbatim as its `/ColorSpace`.
    fn pdf_with_image_xobject(cs_name: &str, sample: &[u8]) -> Vec<u8> {
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets: Vec<(u32, usize)> = Vec::new();

        offsets.push((1, pdf.len()));
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        offsets.push((2, pdf.len()));
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
        offsets.push((3, pdf.len()));
        pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 1 1] >>\nendobj\n");

        offsets.push((99, pdf.len()));
        let header = format!(
            "99 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 \
             /BitsPerComponent 8 /ColorSpace {} /Length {} >>\nstream\n",
            cs_name,
            sample.len()
        );
        pdf.extend_from_slice(header.as_bytes());
        pdf.extend_from_slice(sample);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");

        let xref_off = pdf.len();
        let max_id = 99u32;
        pdf.extend_from_slice(format!("xref\n0 {}\n", max_id + 1).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for id in 1..=max_id {
            if let Some((_, off)) = offsets.iter().find(|(oid, _)| *oid == id) {
                pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
            } else {
                pdf.extend_from_slice(b"0000000000 65535 f \n");
            }
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                max_id + 1,
                xref_off
            )
            .as_bytes(),
        );
        pdf
    }

    #[test]
    fn decode_resolves_resource_name_cs() {
        // Image XObject with `/ColorSpace /CS0`; the page `/Resources/ColorSpace`
        // maps `/CS0 → /DeviceGray`. Before the fix, decode() failed with
        // "Unsupported color space: CS0"; now it succeeds. ~keep
        let doc = crate::document::PdfDocument::from_bytes(pdf_with_image_xobject("/CS0", &[128])).unwrap();
        let obj_ref = ObjectRef::new(99, 0);
        let xobj = doc.load_object(obj_ref).unwrap();
        let dict = xobj.as_dict().expect("image xobject dict").clone();

        let mut map = empty_map();
        map.insert("CS0".to_string(), Object::Name("DeviceGray".to_string()));

        let handle = image_handle_from_xobject(&doc, obj_ref, &dict, crate::content::Matrix::identity(), 0, &map)
            .expect("handle builds");

        assert_eq!(handle.color_space, ColorSpace::DeviceGray);
        assert!(handle.indexed_base().is_none());
        let image = handle.decode().expect("decode resolves resource-name CS");
        assert_eq!(*image.color_space(), ColorSpace::DeviceGray);
    }

    #[test]
    fn xobject_direct_indexed_handle_reports_indexed_with_base() {
        let doc = crate::document::PdfDocument::from_bytes(pdf_with_image_xobject(
            "[/Indexed /DeviceRGB 1 <000000FFFFFF>]",
            &[0],
        ))
        .unwrap();
        let obj_ref = ObjectRef::new(99, 0);
        let dict = doc.load_object(obj_ref).unwrap().as_dict().unwrap().clone();
        let handle = image_handle_from_xobject(
            &doc,
            obj_ref,
            &dict,
            crate::content::Matrix::identity(),
            0,
            &empty_map(),
        )
        .unwrap();
        assert_eq!(handle.color_space, ColorSpace::Indexed);
        assert_eq!(handle.indexed_base(), Some(ColorSpace::DeviceRGB));
    }
}

#[cfg(test)]
mod png_bytes_panic_safety_tests {
    use super::*;

    /// A raw RGB buffer longer than width×height×3 (e.g. a 16-bit-per-component
    /// image whose samples were not collapsed to 8-bit) must NOT panic the PNG
    /// encoder — it must surface a recoverable `Error` that crosses the FFI
    /// boundary so Python/other callers can catch it. Regression for the
    /// `assertion left == right failed: Invalid buffer length` panic.
    #[test]
    fn to_png_bytes_rejects_oversized_rgb_buffer_without_panicking() {
        let (w, h) = (4u32, 2u32);
        // Twice the bytes a 4x2 RGB image needs (mimics undownsampled 16-bit). ~keep
        let pixels = vec![0u8; (w * h * 3 * 2) as usize];
        let img = PdfImage::new(
            w,
            h,
            ColorSpace::DeviceRGB,
            8,
            ImageData::Raw {
                pixels,
                format: PixelFormat::RGB,
            },
        );
        let result = img.to_png_bytes();
        assert!(
            result.is_err(),
            "oversized RGB buffer must yield a recoverable Err, not a panic"
        );
    }

    /// PNG is lossless, so the filter choice is purely a container-size
    /// decision: `NoFilter` leaves the stream near-raw because deflate alone
    /// cannot exploit a smooth ramp, while adaptive per-scanline filtering
    /// (Sub/Up/Avg/Paeth) reduces each row to a near-constant residual first.
    /// Pins both halves — identical samples, materially smaller container.
    #[test]
    fn png_export_adaptive_filter_shrinks_gradient() {
        let (w, h) = (256u32, 256u32);
        let mut pixels = Vec::with_capacity((w * h * 3) as usize);
        for y in 0..h {
            for x in 0..w {
                // Diagonal ramp: each scanline is the previous one shifted by
                // one, the exact shape a Sub/Up/Paeth predictor flattens. ~keep
                let v = ((x + y) % 256) as u8;
                pixels.extend_from_slice(&[v, v.wrapping_add(64), v.wrapping_add(128)]);
            }
        }
        let raw_len = pixels.len();
        let img = PdfImage::new(
            w,
            h,
            ColorSpace::DeviceRGB,
            8,
            ImageData::Raw {
                pixels: pixels.clone(),
                format: PixelFormat::RGB,
            },
        );
        let png = img.to_png_bytes().expect("gradient must encode");

        let decoded = image::load_from_memory(&png).expect("valid PNG").to_rgb8();
        assert_eq!(
            decoded.as_raw().as_slice(),
            pixels.as_slice(),
            "PNG must round-trip the samples byte-for-byte"
        );

        assert!(
            png.len() * 8 <= raw_len,
            "adaptive filtering must compress the ramp at least 8x: {} bytes from {} raw",
            png.len(),
            raw_len
        );
    }

    /// A correctly sized 8-bit RGB buffer (the shape produced after 16-bit
    /// samples are downsampled at parse time) encodes to a valid PNG.
    #[test]
    fn to_png_bytes_encodes_correctly_sized_rgb() {
        let (w, h) = (4u32, 2u32);
        let pixels = vec![128u8; (w * h * 3) as usize];
        let img = PdfImage::new(
            w,
            h,
            ColorSpace::DeviceRGB,
            8,
            ImageData::Raw {
                pixels,
                format: PixelFormat::RGB,
            },
        );
        let png = img.to_png_bytes().expect("correctly sized RGB must encode");
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']), "valid PNG signature");
    }

    /// The 16→8 reduction rounds `v*255/65535` to nearest and hits the endpoints
    /// exactly, rather than flooring via a high-byte drop (WS1.8b). The floor
    /// form `v >> 8` would map 0xFF80 → 0xFF (255) here too, but biases the
    /// mid-range and interior values downward; these anchors pin the rounding.
    #[test]
    fn reduce_16_to_8_rounds_to_nearest() {
        assert_eq!(reduce_16_to_8(0x00, 0x00), 0, "black stays black");
        assert_eq!(reduce_16_to_8(0xFF, 0xFF), 255, "white stays white");
        // 0x0080 = 128: 128*255/65535 = 0.498 → rounds to 0; high-byte drop also 0. ~keep
        assert_eq!(reduce_16_to_8(0x00, 0x80), 0);
        // 0x0100 = 256: 256*255/65535 = 0.996 → rounds to 1 (floor >> 8 = 1 too). ~keep
        assert_eq!(reduce_16_to_8(0x01, 0x00), 1);
        // 0x8080 = 32896: 32896*255/65535 = 127.998 → 128; floor >>8 = 0x80 = 128. ~keep
        assert_eq!(reduce_16_to_8(0x80, 0x80), 128);
        // 0xFF7F = 65407: 65407*255/65535 = 254.5 → rounds up to 255; >>8 = 255. ~keep
        assert_eq!(reduce_16_to_8(0xFF, 0x7F), 255);
        // Monotonic and full-range: every high-byte value round-trips near itself. ~keep
        for hi in 0u8..=255 {
            let out = reduce_16_to_8(hi, hi);
            assert!(
                out >= hi.saturating_sub(1) && out <= hi.saturating_add(1),
                "reduction stays within 1 LSB of the high byte for hi={hi}"
            );
        }
    }
}

#[cfg(test)]
mod ccitt_decode_array_polarity_tests {
    use super::*;
    use crate::object::Object;
    use std::collections::HashMap;

    /// Pack an MSB-first "0"/"1" bit string into bytes, zero-padding the
    /// final byte (same convention as the CCITT decoder's own hand-built
    /// codestream tests in `src/decoders/ccitt.rs`).
    fn pack_bits(bits: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut acc = 0u8;
        let mut n = 0u8;
        for ch in bits.chars() {
            acc = (acc << 1) | (ch == '1') as u8;
            n += 1;
            if n == 8 {
                bytes.push(acc);
                acc = 0;
                n = 0;
            }
        }
        if n > 0 {
            bytes.push(acc << (8 - n));
        }
        bytes
    }

    /// Hand-built CCITT G4 (T.6) codestream for an 8x3 bilevel image whose
    /// *raw* decoded runs are black-majority: rows 0-1 solid black, row 2
    /// black with a 2-pixel-wide white notch at columns 3-4. Every run uses
    /// Horizontal mode ("001" + a Modified-Huffman white-run code + a
    /// black-run code from ITU-T T.4 - the same tables `decode_row_g4`
    /// reads), which is reference-line-independent, so each row's bits are
    /// self-contained and can be verified without tracing 2D prediction:
    ///   row 0/1: white-run 0 ("00110101"), black-run 8 ("000101")
    ///   row 2:   white-run 0, black-run 3 ("10"); white-run 2 ("0111"),
    ///            black-run 3
    /// This mirrors the real corpus defect (govdocs 00339_005342 page 0):
    /// some scanners emit a black-majority raw codestream and rely on the
    /// image's /Decode [1 0] to restore the true white-majority page.
    fn black_majority_g4_stream() -> Vec<u8> {
        let row_all_black = format!("001{}{}", "00110101", "000101");
        let row_notch = format!("001{}{}001{}{}", "00110101", "10", "0111", "10");
        pack_bits(&format!("{row_all_black}{row_all_black}{row_notch}"))
    }

    /// Build a minimal CCITTFaxDecode image XObject (8x3, K=-1) wrapping
    /// `black_majority_g4_stream`, with an optional `/Decode` override.
    fn ccitt_xobject(decode: Option<[i64; 2]>) -> Object {
        let mut decode_parms = HashMap::new();
        decode_parms.insert("K".to_string(), Object::Integer(-1));
        decode_parms.insert("Columns".to_string(), Object::Integer(8));
        decode_parms.insert("Rows".to_string(), Object::Integer(3));

        let mut dict = HashMap::new();
        dict.insert("Subtype".to_string(), Object::Name("Image".to_string()));
        dict.insert("Width".to_string(), Object::Integer(8));
        dict.insert("Height".to_string(), Object::Integer(3));
        dict.insert("BitsPerComponent".to_string(), Object::Integer(1));
        dict.insert("ColorSpace".to_string(), Object::Name("DeviceGray".to_string()));
        dict.insert("Filter".to_string(), Object::Name("CCITTFaxDecode".to_string()));
        dict.insert("DecodeParms".to_string(), Object::Dictionary(decode_parms));
        if let Some([lo, hi]) = decode {
            dict.insert(
                "Decode".to_string(),
                Object::Array(vec![Object::Integer(lo), Object::Integer(hi)]),
            );
        }

        Object::Stream {
            dict,
            data: bytes::Bytes::from(black_majority_g4_stream()),
        }
    }

    /// Decode `xobject` to a Luma8 image and count white (0xFF) vs black
    /// (0x00) pixels.
    fn white_black_counts(xobject: &Object) -> (u32, u32) {
        let img = extract_image_from_xobject(None, xobject, None, None).expect("decode hand-built CCITT test image");
        let luma = img
            .to_dynamic_image()
            .expect("decode CCITT test image pixels")
            .into_luma8();
        let white = luma.iter().filter(|&&v| v == 0xFF).count() as u32;
        let black = luma.iter().filter(|&&v| v == 0x00).count() as u32;
        (white, black)
    }

    /// Baseline: no `/Decode` override (implicit default `[0 1]`), no
    /// `/BlackIs1`. The raw codestream is black-majority (22/24 px), so the
    /// correctly-decoded image must stay black-majority - this un-inverted
    /// case must keep working after the `/Decode` fix below.
    #[test]
    fn default_decode_keeps_raw_black_majority_polarity() {
        let (white, black) = white_black_counts(&ccitt_xobject(None));
        assert_eq!(white + black, 24);
        assert!(
            black > white,
            "expected black-majority (raw, no /Decode override), got white={white} black={black}"
        );
    }

    /// The corpus bug (govdocs 00339_005342 page 0): `/Decode [1 0]` on the
    /// image XObject must invert CCITT polarity, turning this same
    /// black-majority raw codestream into a white-majority image - a
    /// mostly-white page with a small black mark - matching poppler.
    /// Before the fix, `extract_image_from_xobject` never read `/Decode`
    /// for the CCITT path, so this asserted black-majority (the bug).
    #[test]
    fn decode_1_0_inverts_to_white_majority() {
        let (white, black) = white_black_counts(&ccitt_xobject(Some([1, 0])));
        assert_eq!(white + black, 24);
        assert!(
            white > black,
            "expected white-majority under /Decode [1 0], got white={white} black={black}"
        );
        assert_eq!(black, 2, "exactly the 2-pixel notch should render black (the 'mark')");
    }

    /// `/Decode [0 1]` written out explicitly is the non-inverted default
    /// and must behave identically to an absent `/Decode` entry.
    #[test]
    fn decode_0_1_is_a_no_op() {
        let (white, black) = white_black_counts(&ccitt_xobject(Some([0, 1])));
        assert_eq!((white, black), white_black_counts(&ccitt_xobject(None)));
    }

    #[test]
    fn extracted_ccitt_data_contains_decoded_grayscale_samples() {
        let image = extract_image_from_xobject(None, &ccitt_xobject(None), None, None)
            .expect("decode hand-built CCITT test image");

        assert_eq!(image.bits_per_component(), 8);
        assert!(image.ccitt_params().is_some(), "CCITT codec metadata must be preserved");
        match image.data() {
            ImageData::Raw { pixels, format } => {
                assert_eq!(*format, PixelFormat::Grayscale);
                assert_eq!(
                    pixels.len(),
                    24,
                    "8 x 3 image must expose one byte per grayscale sample"
                );
                assert_eq!(pixels.iter().filter(|&&sample| sample == 0x00).count(), 22);
                assert_eq!(pixels.iter().filter(|&&sample| sample == 0xFF).count(), 2);
            }
            other => panic!("expected decoded raw grayscale samples, got {other:?}"),
        }
    }
}

/// Regression coverage for the non-CCITT 1-bit `/DeviceGray` image path:
/// a raw packed-bit image (no `/Filter`, or `/Filter /FlateDecode`) used
/// to be unconditionally force-fed through the CCITT decompressor
/// (`to_dynamic_image`'s single `bits_per_component == 1 &&
/// DeviceGray` branch had no filter check), which fails to decode
/// already-unpacked bits as if they were CCITT-compressed data and drops
/// the image entirely. `/ImageMask` variants of the same filters were
/// unaffected (they go through the separate `render_image_mask` bit
/// unpacker), which is why the bug only showed on plain (non-mask)
/// images.
#[cfg(test)]
mod non_ccitt_1bpc_devicegray_tests {
    use super::*;
    use crate::object::Object;
    use std::collections::HashMap;

    /// 8x3 raw packed-bit image: rows 0-1 all-black, row 2 black with a
    /// 2-pixel-wide white notch at columns 3-4 (mirrors the CCITT
    /// polarity tests' pattern for an easy visual cross-check). Under the
    /// default `/Decode [0 1]`, sample bit 0 -> black, bit 1 -> white.
    fn black_majority_rows() -> [u8; 3] {
        [0x00, 0x00, 0x18]
    }

    /// Build a minimal 1-bit `/DeviceGray` image XObject wrapping
    /// `black_majority_rows`, either uncompressed or FlateDecode-filtered,
    /// with an optional `/Decode` override.
    fn devicegray_1bpc_xobject(flate: bool, decode: Option<[i64; 2]>) -> Object {
        let raw: Vec<u8> = black_majority_rows().to_vec();
        let (filter, data) = if flate {
            use flate2::Compression;
            use flate2::write::ZlibEncoder;
            use std::io::Write;
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&raw).expect("flate-compress test bitmap");
            (Some("FlateDecode"), encoder.finish().expect("finish flate stream"))
        } else {
            (None, raw)
        };

        let mut dict = HashMap::new();
        dict.insert("Subtype".to_string(), Object::Name("Image".to_string()));
        dict.insert("Width".to_string(), Object::Integer(8));
        dict.insert("Height".to_string(), Object::Integer(3));
        dict.insert("BitsPerComponent".to_string(), Object::Integer(1));
        dict.insert("ColorSpace".to_string(), Object::Name("DeviceGray".to_string()));
        if let Some(f) = filter {
            dict.insert("Filter".to_string(), Object::Name(f.to_string()));
        }
        if let Some([lo, hi]) = decode {
            dict.insert(
                "Decode".to_string(),
                Object::Array(vec![Object::Integer(lo), Object::Integer(hi)]),
            );
        }

        Object::Stream {
            dict,
            data: bytes::Bytes::from(data),
        }
    }

    fn white_black_counts(xobject: &Object) -> (u32, u32) {
        let img =
            extract_image_from_xobject(None, xobject, None, None).expect("decode hand-built non-CCITT 1bpc test image");
        let luma = img
            .to_dynamic_image()
            .expect("decode non-CCITT 1bpc test image pixels")
            .into_luma8();
        let white = luma.iter().filter(|&&v| v == 0xFF).count() as u32;
        let black = luma.iter().filter(|&&v| v == 0x00).count() as u32;
        (white, black)
    }

    /// Before the fix this failed outright (the CCITT decompressor
    /// rejects raw packed bits as malformed input), not just mis-decoded.
    #[test]
    fn uncompressed_1bpc_devicegray_decodes_without_ccitt() {
        let (white, black) = white_black_counts(&devicegray_1bpc_xobject(false, None));
        assert_eq!(white + black, 24);
        assert_eq!(black, 22, "22 of 24 pixels are black per the hand-built bitmap");
        assert_eq!(white, 2, "the 2-pixel notch at row 2 cols 3-4 is white");
    }

    #[test]
    fn flate_decoded_1bpc_devicegray_decodes_without_ccitt() {
        let (white, black) = white_black_counts(&devicegray_1bpc_xobject(true, None));
        assert_eq!(white + black, 24);
        assert_eq!(black, 22);
        assert_eq!(white, 2);
    }

    /// `/Decode [1 0]` inverts polarity for the non-CCITT path exactly
    /// like it does for CCITT (ISO 32000-1 §8.9.5.2 Table 90) — the
    /// bug's fallback path always produced a fully dropped image, so
    /// there was no polarity to even get wrong before this fix.
    #[test]
    fn decode_1_0_inverts_polarity_on_non_ccitt_path() {
        let (white, black) = white_black_counts(&devicegray_1bpc_xobject(true, Some([1, 0])));
        assert_eq!(white + black, 24);
        assert_eq!(white, 22, "polarity inverted: majority is now white");
        assert_eq!(black, 2, "the notch is now the black pixels");
    }

    /// `/Width` and `/Height` are document-declared; the 1-bit `/DeviceGray`
    /// unpacking branch used to allocate `Vec::with_capacity(width * height)`
    /// with no overflow check or size cap. `u32::MAX x u32::MAX` doesn't
    /// overflow the `usize` multiplication on a 64-bit target (the product
    /// is still well under `usize::MAX`), so it reached
    /// `Vec::with_capacity` with a byte count exceeding `isize::MAX`.
    ///
    /// Before the fix: this panics with "capacity overflow" (raised by
    /// `Vec`'s internal allocation-size check) instead of returning a
    /// recoverable `Error`. After the fix, the checked-multiply + 256 MiB
    /// cap rejects the request up front and returns `Err`.
    #[test]
    fn huge_declared_dimensions_are_rejected_without_allocating() {
        let img = PdfImage::new(
            u32::MAX,
            u32::MAX,
            ColorSpace::DeviceGray,
            1,
            ImageData::Raw {
                pixels: Vec::new(),
                format: PixelFormat::Grayscale,
            },
        );

        let result = img.to_dynamic_image();

        assert!(
            result.is_err(),
            "huge declared /Width x /Height must be rejected, not allocated"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("overflow") || err.contains("exceeds"),
            "expected an overflow/limit error, got: {err}"
        );
    }
}

/// Coverage for the `/ImageMask` bit packing that JBIG2 masks go through.
///
/// The routing gap these guard against dropped every JBIG2 mask on a Mixed Raster
/// Content scan — where all the page text lives — leaving pages that rendered as
/// their blank background and OCR'd to nothing, with no error surfaced anywhere.
#[cfg(test)]
mod image_mask_packing_tests {
    use super::pack_image_mask_rows;

    const INK: u8 = 0;
    const BACKGROUND: u8 = 255;

    #[test]
    fn should_clear_the_bit_for_ink_and_set_it_for_background() {
        let pixels = [
            INK, BACKGROUND, INK, BACKGROUND, BACKGROUND, BACKGROUND, BACKGROUND, BACKGROUND,
        ];

        let packed = pack_image_mask_rows(&pixels, 8, 1).expect("decode within the guard cap must not error");

        // MSB-first: bit 7 is x=0. Ink clears, background sets => 0b0101_1111. ~keep
        assert_eq!(packed, vec![0b0101_1111]);
    }

    #[test]
    fn should_start_each_row_on_its_own_byte_boundary() {
        // 9 px wide => 2 bytes per row. Row 1 must start at byte 2, not bit 9, or every
        // row after the first is shifted and the mask renders as diagonal noise.
        // Row 0 all ink, row 1 all background. ~keep
        let pixels = [[INK; 9], [BACKGROUND; 9]].concat();

        let packed = pack_image_mask_rows(&pixels, 9, 2).expect("decode within the guard cap must not error");

        assert_eq!(packed.len(), 4, "2 rows x 2 bytes");
        assert_eq!(packed[0], 0b0000_0000, "row 0 px 0-7: ink");
        // The 7 trailing bits of each row are padding. The stencil consumer iterates
        // `0..width` and never reads them, so their value is don't-care; they are left
        // at the zero-initialised value rather than being written. ~keep
        assert_eq!(packed[1], 0b0000_0000, "row 0 px 8 ink + 7 untouched pad bits");
        assert_eq!(packed[2], 0b1111_1111, "row 1 px 0-7: background");
        assert_eq!(packed[3], 0b1000_0000, "row 1 px 8 background + 7 pad bits");
    }

    #[test]
    fn should_treat_a_short_decode_as_background_rather_than_ink() {
        // A truncated decode must never paint: an unpainted region is recoverable,
        // spurious ink over the page is not. ~keep
        let pixels = [INK, INK];

        let packed = pack_image_mask_rows(&pixels, 8, 1).expect("decode within the guard cap must not error");

        assert_eq!(packed, vec![0b0011_1111], "only the 2 decoded pixels are ink");
    }

    #[test]
    fn should_treat_mid_gray_at_the_threshold_as_background() {
        // The predicate is `>= 128`; 127 is ink, 128 is background. ~keep
        let pixels = [127, 128, 127, 128, 128, 128, 128, 128];

        let packed = pack_image_mask_rows(&pixels, 8, 1).expect("decode within the guard cap must not error");

        assert_eq!(packed, vec![0b0101_1111]);
    }

    #[test]
    fn should_produce_a_fully_set_buffer_when_every_pixel_is_background() {
        // The blank-page shape: nothing paints, which is exactly what a dropped mask
        // used to look like. Distinguishing this from ink is the whole point. ~keep
        let packed =
            pack_image_mask_rows(&[BACKGROUND; 16], 16, 1).expect("decode within the guard cap must not error");

        assert_eq!(packed, vec![0xFF, 0xFF]);
    }

    #[test]
    fn should_produce_a_fully_cleared_buffer_when_every_pixel_is_ink() {
        let packed = pack_image_mask_rows(&[INK; 16], 16, 1).expect("decode within the guard cap must not error");

        assert_eq!(packed, vec![0x00, 0x00]);
    }

    #[test]
    fn should_reject_dimensions_whose_packed_size_exceeds_the_guard_cap() {
        // 100_000 x 100_000 packs to ~1.16 GiB, comfortably over the 256 MiB
        // cap, and the source pixel slice is empty — proving the guard
        // triggers before any pixel is read, not merely after a slow scan. ~keep
        let result = pack_image_mask_rows(&[], 100_000, 100_000);

        assert!(result.is_err(), "oversized ImageMask must be rejected, not decoded");
    }

    #[test]
    fn should_reject_dimensions_whose_row_byte_product_overflows_usize() {
        let result = pack_image_mask_rows(&[], u32::MAX, u32::MAX);

        assert!(
            result.is_err(),
            "overflowing row_bytes * height must be rejected, not panic"
        );
    }
}

/// Palette resolution and reported depth of expanded palette images (GH#1901, GH#1903).
#[cfg(test)]
mod palette_expansion_tests {
    use super::*;

    fn jpx_xobject(color_space: Option<crate::object::Object>) -> crate::object::Object {
        use crate::object::Object;
        const PALETTE_CMYK_JP2: &[u8] = include_bytes!("../../tests/fixtures/jpx/gh1885_palette_cmyk.jp2");
        let mut dict = std::collections::HashMap::new();
        dict.insert("Subtype".to_string(), Object::Name("Image".to_string()));
        dict.insert("Width".to_string(), Object::Integer(120));
        dict.insert("Height".to_string(), Object::Integer(40));
        // ISO 32000-1 Table 89: a JPXDecode image's /BitsPerComponent is ignored, so this stale
        // depth must not reach the extracted image. (GH#1901)
        dict.insert("BitsPerComponent".to_string(), Object::Integer(4));
        dict.insert("Filter".to_string(), Object::Name("JPXDecode".to_string()));
        if let Some(cs) = color_space {
            dict.insert("ColorSpace".to_string(), cs);
        }
        Object::Stream {
            dict,
            data: bytes::Bytes::from_static(PALETTE_CMYK_JP2),
        }
    }

    /// A lossy palette JPEG 2000 image that names no `/Indexed` decodes through the codestream's
    /// own palette, clamped, to the same samples as the same palette given as an `/Indexed` lookup
    /// in its base space. The samples stay in the palette's device space: a CMYK palette reports
    /// CMYK, whether the dictionary declared `/DeviceCMYK` or named no space. (GH#1903)
    #[test]
    fn a_codestream_palette_without_indexed_keeps_the_palette_device_space() {
        use crate::object::Object;
        // The fixture's `pclr` entry `i` is CMYK `(0, 0, 0, 255 - i)`.
        let palette = (0..=255u8).rev().flat_map(|k| [0, 0, 0, k]).collect();
        let indexed = Object::Array(vec![
            Object::Name("Indexed".to_string()),
            Object::Name("DeviceCMYK".to_string()),
            Object::Integer(255),
            Object::String(palette),
        ]);
        let oracle = decode_indexed_image_in_base_space(None, &jpx_xobject(Some(indexed)), None, None)
            .expect("the /Indexed image")
            .expect("an /Indexed image has base-space samples");
        assert_eq!(oracle.components, 4, "the oracle is a CMYK lookup");
        for declared in [None, Some(Object::Name("DeviceCMYK".to_string()))] {
            let label = format!("{declared:?}");
            let image = extract_image_from_xobject(None, &jpx_xobject(declared), None, None)
                .unwrap_or_else(|e| panic!("{label}: the codestream palette must resolve: {e:?}"));
            let ImageData::Raw { pixels, format } = image.data() else {
                panic!("{label}: raw pixels expected");
            };
            assert_eq!(*format, PixelFormat::CMYK, "{label}");
            assert_eq!(
                *pixels, oracle.samples,
                "{label}: the samples must match the /Indexed lookup"
            );
            assert_eq!(*image.color_space(), ColorSpace::DeviceCMYK, "{label}");
            assert_eq!(image.bits_per_component(), 8, "{label}");
        }

        // A declared space the palette does not fit stays authoritative. ~keep
        let err = extract_image_from_xobject(
            None,
            &jpx_xobject(Some(Object::Name("DeviceRGB".to_string()))),
            None,
            None,
        )
        .expect_err("a four-column palette does not fit /DeviceRGB");
        assert!(
            format!("{err:?}").contains("codestream palette has 4 components, but /ColorSpace has 3"),
            "unexpected failure: {err:?}"
        );
    }

    /// An expanded `/Indexed` image reports the 8 bits per component its RGB samples have, not the
    /// dictionary's packed index depth. (GH#1901)
    #[test]
    fn an_expanded_indexed_image_reports_8_bits_per_component() {
        use crate::object::Object;
        let mut dict = std::collections::HashMap::new();
        dict.insert("Subtype".to_string(), Object::Name("Image".to_string()));
        dict.insert("Width".to_string(), Object::Integer(2));
        dict.insert("Height".to_string(), Object::Integer(1));
        dict.insert("BitsPerComponent".to_string(), Object::Integer(4));
        dict.insert(
            "ColorSpace".to_string(),
            Object::Array(vec![
                Object::Name("Indexed".to_string()),
                Object::Name("DeviceRGB".to_string()),
                Object::Integer(1),
                Object::String(vec![255, 0, 0, 0, 0, 255]),
            ]),
        );
        let xobject = Object::Stream {
            dict,
            data: bytes::Bytes::from_static(&[0x01]),
        };
        let image = extract_image_from_xobject(None, &xobject, None, None).expect("a 4-bit indexed image");
        let ImageData::Raw { pixels, format } = image.data() else {
            panic!("raw pixels expected");
        };
        assert_eq!(
            (*format, pixels.as_slice()),
            (PixelFormat::RGB, &[255, 0, 0, 0, 0, 255][..])
        );
        assert_eq!(image.bits_per_component(), 8);
    }

    /// A JPEG 2000 image with no palette reports the 8 bits per component its decoded samples have,
    /// not a stale `/BitsPerComponent`, which ISO 32000-1 Table 89 says is ignored. (GH#1901)
    #[test]
    fn a_jpeg2000_image_reports_8_bits_per_component() {
        use crate::object::Object;
        const INDICES_GREY_JP2: &[u8] = include_bytes!("../../tests/fixtures/jpx/gh1885_indices_grey.jp2");
        let mut dict = std::collections::HashMap::new();
        dict.insert("Subtype".to_string(), Object::Name("Image".to_string()));
        dict.insert("Width".to_string(), Object::Integer(120));
        dict.insert("Height".to_string(), Object::Integer(40));
        dict.insert("BitsPerComponent".to_string(), Object::Integer(4));
        dict.insert("ColorSpace".to_string(), Object::Name("DeviceGray".to_string()));
        dict.insert("Filter".to_string(), Object::Name("JPXDecode".to_string()));
        let xobject = Object::Stream {
            dict,
            data: bytes::Bytes::from_static(INDICES_GREY_JP2),
        };
        let image = extract_image_from_xobject(None, &xobject, None, None).expect("a greyscale JPEG 2000 image");
        let ImageData::Raw { pixels, format } = image.data() else {
            panic!("raw pixels expected");
        };
        assert_eq!((*format, pixels.len()), (PixelFormat::Grayscale, 120 * 40));
        assert_eq!(image.bits_per_component(), 8);
        assert!(
            image.samples_are_raw(),
            "the decoded samples are the image's own samples, so a colour-key /Mask still applies"
        );
    }
}
