//! Image extraction functionality.
//!
//! This module provides functions for extracting metadata and EXIF data from images,
//! including support for multi-frame TIFF files.

use crate::error::{Result, XbergError};
use crate::extraction::exif::extract_exif_data;
use crate::extraction::heif::is_heif_container;
#[cfg(feature = "ocr")]
use crate::extraction::image_decode::image_dimension_error;
use crate::extraction::image_decode::{
    ImageDecodeBudget, decode_standard_image_with_security_limits, decode_standard_rgb8_with_security_limits,
    decoded_byte_count,
};
use crate::extractors::security::SecurityLimits;
use std::collections::HashMap;
#[cfg(feature = "ocr")]
use std::io::Cursor;

/// JP2 file signature: 12-byte box starting with length 0x0000000C and type "jP  "
const JP2_MAGIC: &[u8] = &[0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20];

#[cfg(feature = "ocr")]
fn jp2_peak_decoded_bytes(width: u32, height: u32, num_channels: u8, has_alpha: bool) -> Result<u64> {
    let pixel_count = decoded_byte_count(width, height, 1)?;
    let source_channels = u64::from(num_channels) + u64::from(u8::from(has_alpha));
    let source_bytes = pixel_count
        .checked_mul(source_channels)
        .ok_or_else(|| image_dimension_error(width, height, u64::MAX, u64::MAX))?;
    let rgb_bytes = pixel_count
        .checked_mul(u64::from(image::ColorType::Rgb8.bytes_per_pixel()))
        .ok_or_else(|| image_dimension_error(width, height, u64::MAX, u64::MAX))?;
    if num_channels == 3 && !has_alpha {
        Ok(rgb_bytes)
    } else {
        source_bytes
            .checked_add(rgb_bytes)
            .ok_or_else(|| image_dimension_error(width, height, u64::MAX, u64::MAX))
    }
}

#[cfg(feature = "ocr")]
fn jp2_peak_live_bytes(
    width: u32,
    height: u32,
    num_channels: u8,
    has_alpha: bool,
    encoded_bytes: usize,
) -> Result<u64> {
    jp2_peak_decoded_bytes(width, height, num_channels, has_alpha)?
        .checked_add(u64::try_from(encoded_bytes).unwrap_or(u64::MAX))
        .ok_or_else(|| image_dimension_error(width, height, u64::MAX, u64::MAX))
}

#[cfg(feature = "ocr")]
fn jbig2_gray_peak_live_bytes(width: u32, height: u32, encoded_bytes: usize) -> Result<u64> {
    decoded_byte_count(width, height, u64::from(image::ColorType::L8.bytes_per_pixel()))?
        .checked_add(u64::try_from(encoded_bytes).unwrap_or(u64::MAX))
        .ok_or_else(|| image_dimension_error(width, height, u64::MAX, u64::MAX))
}

#[cfg(feature = "ocr")]
fn jbig2_rgb_peak_live_bytes(width: u32, height: u32, encoded_bytes: usize) -> Result<u64> {
    let decoded_and_encoded = jbig2_gray_peak_live_bytes(width, height, encoded_bytes)?;
    decoded_and_encoded
        .checked_add(decoded_byte_count(
            width,
            height,
            u64::from(image::ColorType::Rgb8.bytes_per_pixel()),
        )?)
        .ok_or_else(|| image_dimension_error(width, height, u64::MAX, u64::MAX))
}

#[cfg(feature = "ocr")]
fn validate_encoded_image_input(bytes: &[u8], limits: &SecurityLimits) -> Result<()> {
    ImageDecodeBudget::from_security_limits(limits).validate(1, 1, u64::try_from(bytes.len()).unwrap_or(u64::MAX))
}

/// Check if bytes start with JPEG 2000 magic bytes.
pub(crate) fn is_jp2(bytes: &[u8]) -> bool {
    bytes.len() >= JP2_MAGIC.len() && bytes[..JP2_MAGIC.len()] == *JP2_MAGIC
}

/// Check if bytes start with J2K codestream magic (SOC marker).
#[cfg(feature = "ocr")]
pub(crate) fn is_j2k(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0] == 0xFF && bytes[1] == 0x4F && bytes[2] == 0xFF && bytes[3] == 0x51
}

/// Image metadata extracted from an image file.
#[derive(Debug, Clone)]
pub(crate) struct ExtractedImageMetadata {
    /// Image width in pixels
    pub(crate) width: u32,
    /// Image height in pixels
    pub(crate) height: u32,
    /// Image format (e.g., "PNG", "JPEG")
    pub(crate) format: String,
    /// EXIF data if available
    pub(crate) exif_data: HashMap<String, String>,
}

/// Parse JP2 file header boxes to extract image dimensions.
///
/// Supports both JP2 container format (ISO 15444-1 Annex I) and raw J2K codestream.
/// Uses pure Rust header parsing without external dependencies.
fn decode_jp2_metadata(bytes: &[u8]) -> Result<ExtractedImageMetadata> {
    if is_jp2(bytes) {
        return parse_jp2_boxes(bytes);
    }

    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0x4F {
        return parse_j2k_siz(bytes);
    }

    Err(XbergError::parsing("Not a valid JPEG 2000 file".to_string()))
}

/// Compute a JP2 box's payload start and length, resolving the extended (64-bit) length form
/// (`box_len == 1`) and the to-end-of-file form (`box_len == 0`) the same way the ISO 15444-1
/// box header does.
fn jp2_box_extent(bytes: &[u8], offset: usize, box_len: usize, len: usize) -> (usize, usize) {
    if box_len == 1 && offset + 16 <= len {
        let ext_len = u64::from_be_bytes([
            bytes[offset + 8],
            bytes[offset + 9],
            bytes[offset + 10],
            bytes[offset + 11],
            bytes[offset + 12],
            bytes[offset + 13],
            bytes[offset + 14],
            bytes[offset + 15],
        ]) as usize;
        (offset + 16, ext_len)
    } else if box_len == 0 {
        (offset + 8, len - offset)
    } else {
        (offset + 8, box_len)
    }
}

/// Read an `ihdr` box's height/width fields (big-endian `u32` each, height first, per ISO
/// 15444-1) starting at `data_start`, if its 8-byte fixed portion fits within `bytes[..len]`.
fn read_ihdr_dimensions(bytes: &[u8], data_start: usize, len: usize) -> Option<(u32, u32)> {
    if data_start + 8 > len {
        return None;
    }
    let height = u32::from_be_bytes([
        bytes[data_start],
        bytes[data_start + 1],
        bytes[data_start + 2],
        bytes[data_start + 3],
    ]);
    let width = u32::from_be_bytes([
        bytes[data_start + 4],
        bytes[data_start + 5],
        bytes[data_start + 6],
        bytes[data_start + 7],
    ]);
    Some((width, height))
}

/// Scan a `jp2h` box's sub-boxes for its nested `ihdr` box and return its dimensions.
fn find_ihdr_in_jp2h(
    bytes: &[u8],
    offset: usize,
    data_start: usize,
    actual_len: usize,
    len: usize,
) -> Option<(u32, u32)> {
    let end = offset + actual_len.min(len - offset);
    let mut sub_offset = data_start;
    while sub_offset + 8 <= end {
        let sub_len = u32::from_be_bytes([
            bytes[sub_offset],
            bytes[sub_offset + 1],
            bytes[sub_offset + 2],
            bytes[sub_offset + 3],
        ]) as usize;
        let sub_type = &bytes[sub_offset + 4..sub_offset + 8];
        let sub_data = sub_offset + 8;

        if sub_type == b"ihdr"
            && let Some(dims) = read_ihdr_dimensions(bytes, sub_data, len)
        {
            return Some(dims);
        }

        if sub_len < 8 {
            break;
        }
        sub_offset += sub_len;
    }
    None
}

/// Build the [`ExtractedImageMetadata`] for a JPEG 2000 image once its `ihdr` dimensions are known.
fn jp2_dimensions_result(bytes: &[u8], width: u32, height: u32) -> ExtractedImageMetadata {
    ExtractedImageMetadata {
        width,
        height,
        format: "JPEG2000".to_string(),
        exif_data: extract_exif_data(bytes),
    }
}

/// Parse JP2 container boxes to find ihdr (Image Header) box.
fn parse_jp2_boxes(bytes: &[u8]) -> Result<ExtractedImageMetadata> {
    let mut offset = 0;
    let len = bytes.len();

    while offset + 8 <= len {
        let box_len =
            u32::from_be_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]) as usize;
        let box_type = &bytes[offset + 4..offset + 8];

        let (data_start, actual_len) = jp2_box_extent(bytes, offset, box_len, len);

        if box_type == b"ihdr"
            && let Some((width, height)) = read_ihdr_dimensions(bytes, data_start, len)
        {
            return Ok(jp2_dimensions_result(bytes, width, height));
        }

        if box_type == b"jp2h"
            && let Some((width, height)) = find_ihdr_in_jp2h(bytes, offset, data_start, actual_len, len)
        {
            return Ok(jp2_dimensions_result(bytes, width, height));
        }

        if actual_len < 8 {
            break;
        }
        offset += actual_len;
    }

    Err(XbergError::parsing("JP2 file missing ihdr box".to_string()))
}

/// Parse J2K raw codestream SIZ marker for image dimensions.
fn parse_j2k_siz(bytes: &[u8]) -> Result<ExtractedImageMetadata> {
    if let Some(offset) = memchr::memmem::find(bytes, &[0xFF, 0x51]) {
        let data_start = offset + 4;
        if data_start + 18 <= bytes.len() {
            let xsiz = u32::from_be_bytes([
                bytes[data_start + 2],
                bytes[data_start + 3],
                bytes[data_start + 4],
                bytes[data_start + 5],
            ]);
            let ysiz = u32::from_be_bytes([
                bytes[data_start + 6],
                bytes[data_start + 7],
                bytes[data_start + 8],
                bytes[data_start + 9],
            ]);
            let xosiz = u32::from_be_bytes([
                bytes[data_start + 10],
                bytes[data_start + 11],
                bytes[data_start + 12],
                bytes[data_start + 13],
            ]);
            let yosiz = u32::from_be_bytes([
                bytes[data_start + 14],
                bytes[data_start + 15],
                bytes[data_start + 16],
                bytes[data_start + 17],
            ]);

            let width = xsiz.saturating_sub(xosiz);
            let height = ysiz.saturating_sub(yosiz);

            return Ok(ExtractedImageMetadata {
                width,
                height,
                format: "JPEG2000".to_string(),
                exif_data: extract_exif_data(bytes),
            });
        }
    }

    Err(XbergError::parsing("J2K codestream missing SIZ marker".to_string()))
}

/// Decode JPEG 2000 image bytes to an RGB image using hayro-jpeg2000.
///
/// Pure Rust, memory-safe decoder. No temp files needed.
#[cfg(all(feature = "ocr", test))]
pub(crate) fn decode_jp2_to_rgb(bytes: &[u8]) -> Result<image::RgbImage> {
    let limits = SecurityLimits::default();
    decode_jp2_to_rgb_with_security_limits(bytes, &limits)
}

#[cfg(feature = "ocr")]
fn decode_jp2_to_rgb_with_security_limits(bytes: &[u8], limits: &SecurityLimits) -> Result<image::RgbImage> {
    use hayro_jpeg2000::{DecodeSettings, DecoderContext, Image as Jp2Image};

    validate_encoded_image_input(bytes, limits)?;
    let jp2 = Jp2Image::new(bytes, &DecodeSettings::default())
        .map_err(|e| XbergError::parsing(format!("JP2 decode failed: {}", e)))?;
    let width = jp2.width();
    let height = jp2.height();
    let has_alpha = jp2.has_alpha();
    let num_channels = jp2.color_space().num_channels();
    let peak_live_bytes = jp2_peak_live_bytes(width, height, num_channels, has_alpha, bytes.len())?;
    ImageDecodeBudget::from_security_limits(limits).validate(width, height, peak_live_bytes)?;
    // hayro-jpeg2000 0.4 threads a caller-owned `DecoderContext` through `decode` so the
    // sample buffers can be reused across images, and returns a borrowing `DecodedImage`
    // rather than the interleaved `Vec<u8>` 0.3 handed back. `data_u8` is that same
    // interleaved unsigned-8-bit view, so everything below is unchanged.
    let mut decoder_context = DecoderContext::default();
    let pixels = jp2
        .decode(&mut decoder_context)
        .map_err(|e| XbergError::parsing(format!("JP2 pixel decode failed: {}", e)))?
        .data_u8();

    let rgb_bytes = match (num_channels, has_alpha) {
        (1, false) => {
            let mut rgb = Vec::with_capacity(pixels.len() * 3);
            for &g in &pixels {
                rgb.push(g);
                rgb.push(g);
                rgb.push(g);
            }
            rgb
        }
        (1, true) => {
            let mut rgb = Vec::with_capacity((pixels.len() / 2) * 3);
            for chunk in pixels.as_chunks::<2>().0 {
                rgb.push(chunk[0]);
                rgb.push(chunk[0]);
                rgb.push(chunk[0]);
            }
            rgb
        }
        (3, false) => pixels,
        (3, true) => {
            let mut rgb = Vec::with_capacity((pixels.len() / 4) * 3);
            for chunk in pixels.as_chunks::<4>().0 {
                rgb.push(chunk[0]);
                rgb.push(chunk[1]);
                rgb.push(chunk[2]);
            }
            rgb
        }
        (4, false) => {
            let mut rgb = Vec::with_capacity((pixels.len() / 4) * 3);
            for chunk in pixels.as_chunks::<4>().0 {
                let c = chunk[0] as f32 / 255.0;
                let m = chunk[1] as f32 / 255.0;
                let y = chunk[2] as f32 / 255.0;
                let k = chunk[3] as f32 / 255.0;
                rgb.push(((1.0 - c) * (1.0 - k) * 255.0) as u8);
                rgb.push(((1.0 - m) * (1.0 - k) * 255.0) as u8);
                rgb.push(((1.0 - y) * (1.0 - k) * 255.0) as u8);
            }
            rgb
        }
        _ => {
            return Err(XbergError::parsing(format!(
                "Unsupported JP2 color space: {} channels, alpha={}",
                num_channels, has_alpha
            )));
        }
    };

    image::RgbImage::from_raw(width, height, rgb_bytes)
        .ok_or_else(|| XbergError::parsing("Failed to construct RGB image from JP2 data".to_string()))
}

/// JBIG2 file signature: 0x97 0x4A 0x42 0x32 0x0D 0x0A 0x1A 0x0A
#[cfg(feature = "ocr")]
const JBIG2_MAGIC: &[u8] = &[0x97, 0x4A, 0x42, 0x32, 0x0D, 0x0A, 0x1A, 0x0A];

/// Check if bytes start with JBIG2 magic bytes.
#[cfg(feature = "ocr")]
pub(crate) fn is_jbig2(bytes: &[u8]) -> bool {
    bytes.len() >= JBIG2_MAGIC.len() && bytes[..JBIG2_MAGIC.len()] == *JBIG2_MAGIC
}

/// Decode JBIG2 image bytes to a grayscale image using hayro-jbig2.
///
/// JBIG2 is a bi-level (1-bit) image compression format commonly used in scanned PDFs.
/// The decoder converts black/white pixels to grayscale (0/255) for OCR processing.
#[cfg(feature = "ocr")]
fn decode_jbig2_to_gray_with_security_limits(bytes: &[u8], limits: &SecurityLimits) -> Result<image::GrayImage> {
    use hayro_jbig2::{Decoder, Image};

    struct GrayDecoder {
        pixels: Vec<u8>,
        max_pixels: usize,
        exceeded_dimensions: bool,
    }

    impl Decoder for GrayDecoder {
        fn push_pixel(&mut self, black: bool) {
            if self.pixels.len() >= self.max_pixels {
                self.exceeded_dimensions = true;
                return;
            }
            self.pixels.push(if black { 0 } else { 255 });
        }

        fn push_pixel_chunk(&mut self, black: bool, chunk_count: u32) {
            let luma = if black { 0 } else { 255 };
            let Some(count) = (chunk_count as usize).checked_mul(8) else {
                self.exceeded_dimensions = true;
                return;
            };
            let Some(new_len) = self.pixels.len().checked_add(count) else {
                self.exceeded_dimensions = true;
                return;
            };
            if new_len > self.max_pixels {
                self.exceeded_dimensions = true;
                return;
            }
            self.pixels.resize(new_len, luma);
        }

        fn next_line(&mut self) {}
    }

    validate_encoded_image_input(bytes, limits)?;
    let jbig2_image = Image::new(bytes).map_err(|e| XbergError::parsing(format!("JBIG2 decode failed: {e}")))?;
    let width = jbig2_image.width();
    let height = jbig2_image.height();
    let decoded_bytes = decoded_byte_count(width, height, u64::from(image::ColorType::L8.bytes_per_pixel()))?;
    let peak_live_bytes = jbig2_gray_peak_live_bytes(width, height, bytes.len())?;
    ImageDecodeBudget::from_security_limits(limits).validate(width, height, peak_live_bytes)?;

    let max_pixels = usize::try_from(decoded_bytes)
        .map_err(|_| image_dimension_error(width, height, decoded_bytes, decoded_bytes))?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(max_pixels)
        .map_err(|error| XbergError::parsing(format!("Failed to reserve JBIG2 decoded image buffer: {error}")))?;
    let mut decoder = GrayDecoder {
        pixels,
        max_pixels,
        exceeded_dimensions: false,
    };
    jbig2_image
        .decode(&mut decoder)
        .map_err(|e| XbergError::parsing(format!("JBIG2 decode failed: {e}")))?;
    if decoder.exceeded_dimensions {
        return Err(XbergError::Validation {
            message: format!("JBIG2 decompressed beyond its declared {width}x{height} image dimensions"),
            source: None,
        });
    }

    image::GrayImage::from_raw(width, height, decoder.pixels)
        .ok_or_else(|| XbergError::parsing("Failed to construct grayscale image from JBIG2 data".to_string()))
}

/// Load image bytes for OCR, with JPEG 2000 and JBIG2 fallback support.
///
/// The standard `image` crate does not support JPEG 2000 or JBIG2 formats.
/// This function detects these formats by magic bytes and uses `hayro-jpeg2000`
/// / `hayro-jbig2` for decoding, falling back to the standard `image` crate
/// for all other formats.
#[cfg(feature = "ocr")]
pub(crate) fn load_image_for_ocr(image_bytes: &[u8], limits: &SecurityLimits) -> Result<image::DynamicImage> {
    decode_image_to_rgb8_with_security_limits(image_bytes, limits).map(image::DynamicImage::ImageRgb8)
}

/// Meters-per-inch, used to convert a PNG `pHYs` pixels-per-metre density into DPI.
#[cfg(feature = "ocr-pipeline")]
const PNG_METERS_PER_INCH: f64 = 0.0254;

/// Read the embedded pixel density from a PNG's `pHYs` chunk, if present, and report it as DPI.
///
/// PNG stores physical pixel density in pixels-per-unit on each axis, independent of the image's
/// actual pixel dimensions (GH#1630: xberg 1.1.5 discarded this and always assumed 72 DPI, so a
/// genuine 300 DPI scan was rescaled even when the caller's `target_dpi` was already 300).
///
/// Performs a bounded scan of the chunk stream (8-byte signature, then `length + type + data +
/// crc` chunks), stopping at the first `IDAT` since `pHYs` always precedes the image data when
/// present. Returns `None` — leaving the caller's existing 72 DPI assumption untouched — for
/// anything that is not a well-formed PNG carrying a metre-unit `pHYs` chunk with a positive,
/// finite horizontal density: not a PNG, no `pHYs` chunk, a malformed chunk stream, an
/// unspecified/unknown unit (unit code 0), or (defensively) a non-finite conversion.
///
/// Only the horizontal density (`xppu`) is reported. `source_dpi` downstream is a single scalar
/// hint to Tesseract; real-world scans essentially never have anisotropic `pHYs` axes, so `xppu`
/// alone is enough without adding a second, almost-always-identical value to plumb through.
///
/// `ocr-pipeline`, not `ocr`, is the gate: `ocr` implies `ocr-pipeline`, and every caller of this
/// function (`ocr::processor::execution::resolve_known_source_dpi` via the `ocr` backend, and
/// `extractors::image::normalize_image_bytes_for_ocr` via the extractor boundary, which only
/// requires `ocr-pipeline`) must see the same symbol regardless of which builds `ocr` on top of.
#[cfg(feature = "ocr-pipeline")]
pub(crate) fn png_pixel_density_dpi(bytes: &[u8]) -> Option<f64> {
    const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    const PHYS_CHUNK_DATA_LEN: usize = 9;
    const PHYS_UNIT_METER: u8 = 1;

    if bytes.len() < PNG_SIGNATURE.len() || bytes[..PNG_SIGNATURE.len()] != PNG_SIGNATURE {
        return None;
    }

    let mut offset = PNG_SIGNATURE.len();
    while offset.checked_add(8)? <= bytes.len() {
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
        let chunk_type = &bytes[offset + 4..offset + 8];
        let data_start = offset + 8;
        let data_end = data_start.checked_add(length)?;
        if data_end.checked_add(4)? > bytes.len() {
            return None;
        }
        if chunk_type == b"IDAT" {
            return None;
        }
        if chunk_type == b"pHYs" {
            if length != PHYS_CHUNK_DATA_LEN {
                return None;
            }
            let data = &bytes[data_start..data_end];
            let xppu = u32::from_be_bytes(data[0..4].try_into().ok()?);
            let unit = data[8];
            if unit != PHYS_UNIT_METER || xppu == 0 {
                return None;
            }
            let dpi = f64::from(xppu) * PNG_METERS_PER_INCH;
            return (dpi.is_finite() && dpi > 0.0).then_some(dpi);
        }
        offset = data_end + 4;
    }
    None
}

/// Read the first IFD's `XResolution`/`YResolution`/`ResolutionUnit` from a classic TIFF and
/// report it as DPI (GH#2143: a 300 DPI nonstandard-size TIFF was assumed 72 DPI and resampled).
///
/// Dependency-free and bounds-checked so it is available under `ocr-pipeline`, where the `tiff`
/// crate is not. Returns `None` for non-TIFF or BigTIFF input, truncated structures, a missing or
/// non-inch/centimetre unit, zero or non-finite rationals, and axes that disagree by more than 1%
/// (a single scalar hint cannot represent anisotropic density).
#[cfg(feature = "ocr-pipeline")]
pub(crate) fn tiff_pixel_density_dpi(bytes: &[u8]) -> Option<f64> {
    const TAG_X_RESOLUTION: u16 = 282;
    const TAG_Y_RESOLUTION: u16 = 283;
    const TAG_RESOLUTION_UNIT: u16 = 296;
    const TYPE_SHORT: u16 = 3;
    const TYPE_RATIONAL: u16 = 5;
    const ENTRY_LEN: usize = 12;
    const CM_PER_INCH: f64 = 2.54;
    const AXIS_TOLERANCE: f64 = 0.01;

    let little = match bytes.get(..2)? {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    let u16_at = |at: usize| -> Option<u16> {
        let raw: [u8; 2] = bytes.get(at..at.checked_add(2)?)?.try_into().ok()?;
        Some(if little {
            u16::from_le_bytes(raw)
        } else {
            u16::from_be_bytes(raw)
        })
    };
    let u32_at = |at: usize| -> Option<u32> {
        let raw: [u8; 4] = bytes.get(at..at.checked_add(4)?)?.try_into().ok()?;
        Some(if little {
            u32::from_le_bytes(raw)
        } else {
            u32::from_be_bytes(raw)
        })
    };
    if u16_at(2)? != 42 {
        return None;
    }
    let ifd = usize::try_from(u32_at(4)?).ok()?;
    let entry_count = usize::from(u16_at(ifd)?);
    let rational = |entry: usize| -> Option<f64> {
        let offset = usize::try_from(u32_at(entry + 8)?).ok()?;
        let (numerator, denominator) = (u32_at(offset)?, u32_at(offset.checked_add(4)?)?);
        (numerator != 0 && denominator != 0).then(|| f64::from(numerator) / f64::from(denominator))
    };

    let (mut x, mut y, mut unit) = (None, None, None);
    for index in 0..entry_count {
        let entry = ifd.checked_add(2)?.checked_add(index.checked_mul(ENTRY_LEN)?)?;
        let (tag, kind, count) = (u16_at(entry)?, u16_at(entry + 2)?, u32_at(entry + 4)?);
        if count != 1 {
            continue;
        }
        match (tag, kind) {
            (TAG_X_RESOLUTION, TYPE_RATIONAL) => x = rational(entry),
            (TAG_Y_RESOLUTION, TYPE_RATIONAL) => y = rational(entry),
            (TAG_RESOLUTION_UNIT, TYPE_SHORT) => unit = u16_at(entry + 8),
            _ => {}
        }
    }

    let (x, y) = (x?, y?);
    if (x - y).abs() / x.max(y) > AXIS_TOLERANCE {
        return None;
    }
    let dpi = match unit? {
        2 => x,
        3 => x * CM_PER_INCH,
        _ => return None,
    };
    (dpi.is_finite() && dpi > 0.0).then_some(dpi)
}

/// Read a caller-supplied source-resolution hint out of `OcrConfig.backend_options`, in DPI.
///
/// Mirrors `ocr::tesseract_backend::TesseractBackend::source_dpi_from_backend_options` (which
/// reads the same `[SOURCE_DPI_BACKEND_OPTION]` key off the same `backend_options` field after
/// the config has been converted to a `TesseractConfig`): an absent, malformed, or non-positive
/// value is "unknown" rather than an error. That copy lives in a module gated on the full `ocr`
/// feature and is only reachable once a caller has a backend, so it cannot be reused directly
/// from the extractor boundary (`extractors::image::normalize_image_bytes_for_ocr`), which only
/// requires `ocr-pipeline` and runs *before* any backend conversion. This is the canonical
/// implementation for that earlier point; `TesseractBackend`'s copy should eventually delegate
/// here instead of re-implementing the same three-line filter (GH#1621 is exactly this drift
/// shape: two independently maintained readers of the same value).
///
/// [SOURCE_DPI_BACKEND_OPTION]: crate::core::config::ocr::SOURCE_DPI_BACKEND_OPTION
#[cfg(feature = "ocr-pipeline")]
pub(crate) fn explicit_source_dpi_from_ocr_config(config: &crate::core::config::OcrConfig) -> Option<f64> {
    config
        .backend_options
        .as_ref()
        .and_then(|options| options.get(crate::core::config::ocr::SOURCE_DPI_BACKEND_OPTION))
        .and_then(serde_json::Value::as_f64)
        .filter(|dpi| dpi.is_finite() && *dpi > 0.0)
}

/// Portrait page sizes, in inches, that [`infer_dpi_from_standard_page_size`] tests a raster's
/// pixel dimensions against (width, height). Both orientations of each entry are tried.
#[cfg(feature = "ocr-pipeline")]
const STANDARD_PAGE_SIZES_INCHES: &[(f64, f64)] = &[
    (8.5, 11.0),        // US Letter
    (8.5, 14.0),        // US Legal
    (11.0, 17.0),       // Tabloid / Ledger
    (8.2677, 11.6929),  // ISO A4 (210mm x 297mm)
    (11.6929, 16.5354), // ISO A3 (297mm x 420mm)
];

/// Scan resolutions real scanners/renderers commonly use. A page-size match is only trusted
/// when the implied DPI lands near one of these — otherwise arbitrary pixel dimensions with a
/// page-shaped aspect ratio (e.g. a photo) would be misread as a page at an implausible DPI.
#[cfg(feature = "ocr-pipeline")]
const CANDIDATE_SCAN_DPIS: &[u32] = &[72, 96, 100, 120, 150, 200, 240, 300, 400, 600, 1200];

/// Relative tolerance for (a) the horizontal- and vertical-implied DPI to agree with each other,
/// and (b) their average to land near one of [`CANDIDATE_SCAN_DPIS`].
#[cfg(feature = "ocr-pipeline")]
const PAGE_SIZE_MATCH_RELATIVE_TOLERANCE: f64 = 0.01;

/// Infer a source DPI from pixel dimensions that match a standard page size at a plausible scan
/// resolution (#1788): a PNG with no `pHYs` chunk was previously always assumed to be 72 DPI, so
/// a 2550x3300 pixel page — letter size at exactly 300 DPI — was rescaled by a spurious 1.236x
/// factor before OCR, destroying 58% of a dense page's readable words. Many tools (e.g. PIL's
/// `Image.save` without `dpi=`) write PNGs with no density tag at all, so identical pixels
/// otherwise gave different OCR text depending only on how the file was saved.
///
/// Returns `None` — leaving the caller's existing 72 DPI assumption unchanged — when the pixel
/// dimensions do not match any [`STANDARD_PAGE_SIZES_INCHES`] entry at a DPI near
/// [`CANDIDATE_SCAN_DPIS`], in either orientation. This is deliberate: an image whose pixel
/// dimensions match no standard page size gets no invented DPI, not a guessed one.
#[cfg(feature = "ocr-pipeline")]
fn infer_dpi_from_standard_page_size(width: u32, height: u32) -> Option<f64> {
    if width == 0 || height == 0 {
        return None;
    }
    let (px_w, px_h) = (f64::from(width), f64::from(height));

    for &(page_w, page_h) in STANDARD_PAGE_SIZES_INCHES {
        for (candidate_px_w, candidate_px_h) in [(px_w, px_h), (px_h, px_w)] {
            let dpi_w = candidate_px_w / page_w;
            let dpi_h = candidate_px_h / page_h;
            if !(dpi_w.is_finite() && dpi_h.is_finite()) || dpi_w <= 0.0 || dpi_h <= 0.0 {
                continue;
            }
            let average_dpi = (dpi_w + dpi_h) / 2.0;
            let axis_spread = (dpi_w - dpi_h).abs() / average_dpi;
            if axis_spread > PAGE_SIZE_MATCH_RELATIVE_TOLERANCE {
                continue;
            }
            let near_a_common_scan_resolution = CANDIDATE_SCAN_DPIS.iter().any(|&candidate| {
                (average_dpi - f64::from(candidate)).abs() / f64::from(candidate) <= PAGE_SIZE_MATCH_RELATIVE_TOLERANCE
            });
            if near_a_common_scan_resolution {
                return Some(average_dpi);
            }
        }
    }
    None
}

/// Resolve the resolution to report for a raster, in priority order:
///
/// 1. `explicit_source_dpi` — a caller-supplied hint (see
///    [`explicit_source_dpi_from_ocr_config`] for the `OcrConfig.backend_options` reader; the PDF
///    OCR route supplies its own via `TesseractConfig::source_dpi` directly). A caller that
///    explicitly said "my source is 72 DPI" must not be overridden by embedded metadata.
/// 2. The image's own embedded pixel-density metadata, when present (GH#1630; PNG
///    `pHYs` and the first TIFF IFD — see [`png_pixel_density_dpi`], [`tiff_pixel_density_dpi`]).
/// 3. A standard-page-size inference from the pixel dimensions themselves (#1788; see
///    [`infer_dpi_from_standard_page_size`]).
/// 4. `None`, meaning the historical 72 DPI assumption applies further down the pipeline — the
///    pixel dimensions matched no standard page size, so no DPI is invented for them.
///
/// Shared by both consumers — `ocr::processor::execution::perform_ocr` and
/// `extractors::image::normalize_image_bytes_for_ocr` — so the precedence cannot drift between
/// them the way two independent PNG chunk scanners would (GH#1621).
#[cfg(feature = "ocr-pipeline")]
pub(crate) fn resolve_known_source_dpi(
    explicit_source_dpi: Option<f64>,
    image_bytes: &[u8],
    width: u32,
    height: u32,
) -> Option<f64> {
    explicit_source_dpi
        .or_else(|| png_pixel_density_dpi(image_bytes))
        .or_else(|| tiff_pixel_density_dpi(image_bytes))
        .or_else(|| infer_dpi_from_standard_page_size(width, height))
}

pub(crate) fn decode_image_to_rgb8_with_security_limits(
    image_bytes: &[u8],
    limits: &SecurityLimits,
) -> Result<image::RgbImage> {
    #[cfg(feature = "ocr")]
    {
        if is_jp2(image_bytes) || is_j2k(image_bytes) {
            return decode_jp2_to_rgb_with_security_limits(image_bytes, limits);
        }
        if is_jbig2(image_bytes) {
            let gray = decode_jbig2_to_gray_with_security_limits(image_bytes, limits)?;
            let (width, height) = gray.dimensions();
            let peak_bytes = jbig2_rgb_peak_live_bytes(width, height, image_bytes.len())?;
            ImageDecodeBudget::from_security_limits(limits).validate(width, height, peak_bytes)?;
            return Ok(image::DynamicImage::ImageLuma8(gray).into_rgb8());
        }
    }
    decode_standard_rgb8_with_security_limits(image_bytes, limits)
}

// Both callers are `#[cfg(feature = "ocr")]` tests in this file's `tests` module, so a
// bare `cfg(test)` leaves it dead in any test build without `ocr` (the
// `formula-recognition,pdf` CI leg). ~keep
#[cfg(all(test, feature = "ocr"))]
pub(crate) fn decode_image_with_security_limits(
    image_bytes: &[u8],
    limits: &SecurityLimits,
) -> Result<image::DynamicImage> {
    #[cfg(feature = "ocr")]
    {
        if is_jp2(image_bytes) || is_j2k(image_bytes) {
            return decode_jp2_to_rgb_with_security_limits(image_bytes, limits).map(image::DynamicImage::ImageRgb8);
        }
        if is_jbig2(image_bytes) {
            return decode_jbig2_to_gray_with_security_limits(image_bytes, limits).map(image::DynamicImage::ImageLuma8);
        }
    }
    decode_standard_image_with_security_limits(image_bytes, limits)
}

/// Extract metadata from image bytes.
///
/// Extracts dimensions, format, and EXIF data from the image.
/// Standard formats are header-probed without allocating their pixel buffers; JPEG 2000
/// dimensions come from JP2/J2K headers, and HEIF-family dimensions come from the primary
/// image handle when the `heic` feature is enabled. EXIF is read from the original bytes.
#[cfg(test)]
pub(crate) fn extract_image_metadata(bytes: &[u8]) -> Result<ExtractedImageMetadata> {
    let limits = SecurityLimits::default();
    extract_image_metadata_with_security_limits(bytes, &limits)
}

pub(crate) fn extract_image_metadata_with_security_limits(
    bytes: &[u8],
    limits: &SecurityLimits,
) -> Result<ExtractedImageMetadata> {
    let budget = ImageDecodeBudget::from_security_limits(limits);
    if (is_jp2(bytes) || (bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0x4F))
        && let Ok(metadata) = decode_jp2_metadata(bytes)
    {
        let decoded_bytes = decoded_byte_count(
            metadata.width,
            metadata.height,
            u64::from(image::ColorType::Rgb8.bytes_per_pixel()),
        )?;
        budget.validate(metadata.width, metadata.height, decoded_bytes)?;
        #[cfg(feature = "ocr")]
        decode_jp2_to_rgb_with_security_limits(bytes, limits)?;
        return Ok(metadata);
    }

    if is_heif_container(bytes) {
        let exif_data = extract_exif_data(bytes);
        #[cfg(feature = "heic")]
        {
            use xberg_libheif::HeifContext;

            let context = HeifContext::read_from_bytes(bytes)
                .map_err(|error| XbergError::parsing(format!("Failed to read HEIF container: {error}")))?;
            let handle = context
                .primary_image_handle()
                .map_err(|error| XbergError::parsing(format!("Failed to read HEIF primary image handle: {error}")))?;
            let width = handle.width();
            let height = handle.height();
            let decoded_bytes =
                decoded_byte_count(width, height, u64::from(image::ColorType::Rgba8.bytes_per_pixel()))?;
            budget.validate(width, height, decoded_bytes)?;
            return Ok(ExtractedImageMetadata {
                width,
                height,
                format: "HEIF".to_string(),
                exif_data,
            });
        }
        #[cfg(not(feature = "heic"))]
        {
            let _ = exif_data;
            return Err(XbergError::parsing(
                "HEIF/HEIC/AVIF decoding requires the `heic` Cargo feature".to_string(),
            ));
        }
    }

    let decoded = decode_standard_image_with_security_limits(bytes, limits)?;
    let format = image::guess_format(bytes)
        .map_err(|error| XbergError::parsing(format!("Failed to read image format: {error}")))?;
    Ok(ExtractedImageMetadata {
        width: decoded.width(),
        height: decoded.height(),
        format: format!("{format:?}").to_uppercase(),
        exif_data: extract_exif_data(bytes),
    })
}

/// Result of OCR extraction from an image with optional page tracking.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub struct ImageOcrResult {
    /// Extracted text content
    pub content: String,
    /// Character byte boundaries per frame (for multi-frame TIFFs)
    pub boundaries: Option<Vec<crate::types::PageBoundary>>,
    /// Per-frame content information
    pub page_contents: Option<Vec<crate::types::PageContent>>,
}

/// Detects the number of frames in a TIFF file.
///
/// Returns the count of image frames/pages in a TIFF. Single-frame TIFFs return 1.
/// Invalid or non-TIFF data returns an error.
///
/// # Arguments
/// * `bytes` - Raw TIFF file bytes
///
/// # Returns
/// Frame count if valid TIFF, error otherwise.
#[cfg(feature = "ocr")]
pub(crate) fn detect_tiff_frame_count(bytes: &[u8]) -> Result<usize> {
    use tiff::decoder::Decoder;
    let mut decoder =
        Decoder::new(Cursor::new(bytes)).map_err(|e| XbergError::parsing(format!("TIFF decode: {}", e)))?;

    let mut count = 1;
    while decoder.next_image().is_ok() {
        count += 1;
    }
    Ok(count)
}

/// Extract text from image bytes using OCR with optional page tracking for multi-frame TIFFs.
///
/// This function:
/// - Detects if the image is a multi-frame TIFF
/// - For multi-frame TIFFs with PageConfig enabled, iterates frames and tracks boundaries
/// - For single-frame images or when page tracking is disabled, runs OCR on the whole image
/// - Returns (content, boundaries, page_contents) tuple
///
/// # Arguments
/// * `bytes` - Image file bytes
/// * `mime_type` - MIME type (e.g., "image/tiff")
/// * `ocr_result` - OCR backend result containing the text
/// * `page_config` - Optional page configuration for boundary tracking
///
/// # Returns
/// ImageOcrResult with content and optional boundaries for pagination
#[cfg(feature = "ocr")]
pub(crate) fn extract_text_from_image_with_ocr(
    bytes: &[u8],
    mime_type: &str,
    ocr_result: String,
    page_config: Option<&crate::core::config::PageConfig>,
) -> Result<ImageOcrResult> {
    let is_tiff = mime_type.to_lowercase().contains("tiff");
    let should_track_pages = page_config.is_some() && is_tiff;

    if !should_track_pages {
        return Ok(ImageOcrResult {
            content: ocr_result,
            boundaries: None,
            page_contents: None,
        });
    }

    let frame_count = detect_tiff_frame_count(bytes)?;

    if frame_count <= 1 {
        return Ok(ImageOcrResult {
            content: ocr_result,
            boundaries: None,
            page_contents: None,
        });
    }

    let content_len = ocr_result.len();
    let content_per_frame = content_len.checked_div(frame_count).unwrap_or(content_len);

    let mut boundaries = Vec::new();
    let mut page_contents = Vec::new();
    let mut byte_offset = 0;

    for frame_num in 1..=frame_count {
        let frame_end = if frame_num == frame_count {
            content_len
        } else {
            let raw_end = (frame_num * content_per_frame).min(content_len);
            (raw_end..=content_len)
                .find(|&i| ocr_result.is_char_boundary(i))
                .unwrap_or(content_len)
        };

        boundaries.push(crate::types::PageBoundary {
            byte_start: byte_offset,
            byte_end: frame_end,
            page_number: frame_num as u32,
        });

        let frame_text = &ocr_result[byte_offset..frame_end];
        page_contents.push(crate::types::PageContent {
            page_number: frame_num as u32,
            content: frame_text.to_string(),
            tables: vec![],
            image_indices: vec![],
            image_preprocessing: None,
            hierarchy: None,
            is_blank: Some(crate::extraction::blank_detection::is_page_text_blank(frame_text)),
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
            ocr_confidence: None,
            native_content: None,
        });

        byte_offset = frame_end;
    }

    Ok(ImageOcrResult {
        content: ocr_result,
        boundaries: Some(boundaries),
        page_contents: Some(page_contents),
    })
}

#[cfg(test)]
#[path = "image/tests.rs"]
mod tests;

#[cfg(all(test, feature = "ocr"))]
#[path = "image/jp2_decode_tests.rs"]
mod jp2_decode_tests;
