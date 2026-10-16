//! Image extraction using the xberg_native_pdf backend.
//!
//! Extracts embedded images from PDF pages via xberg_native_pdf, including
//! actual image data and metadata.

#[cfg(test)]
#[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
mod decode_skip_tests;
#[cfg(test)]
mod parallel_tests;

use super::NativeDocument;
use crate::cancellation::CancellationToken;
use crate::pdf::error::{PdfError, Result};
use bytes::Bytes;
use image::{DynamicImage, ImageFormat};
use std::borrow::Cow;
use std::io::Cursor;

/// Detect image format from magic bytes, returning a static format string.
///
/// This function validates that image data actually matches its claimed format
/// by inspecting magic bytes. If the data doesn't match any known format, it
/// returns `"raw"`.
#[inline]
fn detect_image_format_from_bytes(data: &[u8]) -> &'static str {
    if data.starts_with(b"\xff\xd8\xff") {
        "jpeg"
    } else if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        "png"
    } else if data.starts_with(b"GIF8") {
        "gif"
    } else if data.starts_with(b"II") || data.starts_with(b"MM") {
        "tiff"
    } else if data.starts_with(b"BM") {
        "bmp"
    } else if data.len() >= 8 && data[0..4] == [0x00, 0x00, 0x00, 0x0C] && data[4..8] == [0x6A, 0x50, 0x20, 0x20] {
        "jpeg2000"
    } else {
        "raw"
    }
}

/// Extract at most `limit` images in content-stream paint order.
///
/// `page_image_handles` performs the cheap content-stream/CTM pass first, allowing
/// the cap to be applied before image decompression while preserving bounding boxes,
/// inline images, and images nested in Form XObjects.
fn extract_n_images_from_page_handles(
    doc: &NativeDocument,
    page_idx: usize,
    limit: usize,
) -> Result<Vec<xberg_native_pdf::extractors::PdfImage>> {
    let handles = doc.doc.page_image_handles(page_idx).map_err(|error| {
        PdfError::ExtractionFailed(format!(
            "enumerating image handles for PDF page {}: {error}",
            page_idx + 1
        ))
    })?;
    let mut images = Vec::new();
    for handle in handles.into_iter().take(limit) {
        match handle.decode() {
            Ok(img) => images.push(img),
            Err(error) => {
                tracing::debug!(page = page_idx, "image decompression failed: {error}");
            }
        }
    }

    Ok(images)
}

/// Re-encode raw PDF pixel data as a PNG buffer.
///
/// xberg_native_pdf emits `ImageData::Raw` without self-describing headers. Re-encoding
/// to PNG makes the buffer probeable by `load_image_for_ocr`,
/// `extract_image_metadata`, VLM pipelines, etc.
///
/// Returns `Err` if the pixel buffer length does not match `w × h × bpp` or if
/// PNG encoding fails.
fn raw_pixels_to_png(
    w: u32,
    h: u32,
    format: &xberg_native_pdf::extractors::PixelFormat,
    pixels: &[u8],
) -> Result<Bytes> {
    let dynamic = match *format {
        xberg_native_pdf::extractors::PixelFormat::Grayscale => {
            let checked = exact_pixel_buffer(w, h, 1, pixels, "grayscale")?;
            let buf = image::GrayImage::from_raw(w, h, checked).ok_or_else(|| {
                PdfError::ExtractionFailed(format!(
                    "grayscale pixel buffer ({} bytes) does not fit {}×{} image",
                    pixels.len(),
                    w,
                    h
                ))
            })?;
            DynamicImage::ImageLuma8(buf)
        }
        xberg_native_pdf::extractors::PixelFormat::RGB => {
            let checked = exact_pixel_buffer(w, h, 3, pixels, "RGB")?;
            let buf = image::RgbImage::from_raw(w, h, checked).ok_or_else(|| {
                PdfError::ExtractionFailed(format!(
                    "RGB pixel buffer ({} bytes) does not fit {}×{} image",
                    pixels.len(),
                    w,
                    h
                ))
            })?;
            DynamicImage::ImageRgb8(buf)
        }
        xberg_native_pdf::extractors::PixelFormat::CMYK => {
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
            let checked = exact_pixel_buffer(w, h, 3, &rgb, "CMYK→RGB")?;
            let buf = image::RgbImage::from_raw(w, h, checked)
                .ok_or_else(|| PdfError::ExtractionFailed(format!("CMYK→RGB buffer does not fit {}×{} image", w, h)))?;
            DynamicImage::ImageRgb8(buf)
        }
    };
    let mut png_bytes = Vec::new();
    dynamic
        .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
        .map_err(|e| PdfError::ExtractionFailed(format!("PNG re-encode of raw PDF image failed: {e}")))?;
    Ok(Bytes::from(png_bytes))
}

/// Return the pixel buffer only if it holds exactly `w × h × channels` bytes.
///
/// `ImageBuffer::from_raw` rejects a buffer that is too *small* and accepts one
/// that is too *large*, keeping the extra bytes. A source decoder that pads each
/// row to an alignment boundary produces exactly that: a buffer longer than the
/// image needs. The mismatch then surfaces inside `DynamicImage::write_to`, where
/// the `image` crate asserts on the exact size and panics rather than returning an
/// error.
///
/// Checking the length here turns that panic into the same recoverable
/// `ExtractionFailed` this function already returns for an undersized buffer.
/// `to_png_bytes` in `xberg-native-pdf` carries the same guard for the same reason.
fn exact_pixel_buffer(w: u32, h: u32, channels: usize, pixels: &[u8], kind: &str) -> Result<Vec<u8>> {
    let expected = (w as usize)
        .checked_mul(h as usize)
        .and_then(|px| px.checked_mul(channels))
        .ok_or_else(|| PdfError::ExtractionFailed(format!("{kind} image dimensions {w}×{h} overflow")))?;
    if pixels.len() != expected {
        return Err(PdfError::ExtractionFailed(format!(
            "{kind} pixel buffer ({} bytes) does not match {}×{} image ({expected} bytes expected); \
             the decoded row stride does not match width × {channels}",
            pixels.len(),
            w,
            h
        )));
    }
    Ok(pixels.to_vec())
}

/// Build the `ProcessingWarning` for an image that was dropped because its raw
/// pixel buffer could not be re-encoded (issue #71). Previously this case only
/// logged via `tracing::warn!`, so callers had no structured signal that the
/// output `images` array is shorter than the document's actual image count.
fn unencodable_image_warning(image_index: u32, page_number: u32, error: &PdfError) -> crate::types::ProcessingWarning {
    crate::types::ProcessingWarning {
        source: std::borrow::Cow::Borrowed("pdf_images"),
        message: std::borrow::Cow::Owned(format!(
            "skipped image {image_index} on page {page_number}: could not be re-encoded from raw \
             pixel data ({error})"
        )),
    }
}

/// How one image XObject's OCR-ready bytes were recovered by
/// [`page_ocr_fallback_image_bytes`].
///
/// The three arms are the three recovery modes that function already distinguishes
/// internally; carrying them out lets the caller record provenance on the
/// [`crate::types::ExtractedImage`] it builds for the recovered page (issue #1444)
/// instead of throwing the distinction away.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum XObjectRecovery {
    /// `decode()` handed back an embedded JPEG stream; passed through untouched.
    EmbeddedJpeg,
    /// `decode()` handed back raw pixel data; re-encoded here to PNG.
    ReencodedPixels,
    /// `decode()` failed on a DCTDecode/JPXDecode stream, so the raw compressed
    /// stream — itself a valid standalone JPEG/JP2 file — is handed back instead.
    RawCompressedStream,
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
impl XObjectRecovery {
    /// Stable, human-readable tag recorded on the recovered image's `description`.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::EmbeddedJpeg => "embedded_jpeg",
            Self::ReencodedPixels => "reencoded_pixels",
            Self::RawCompressedStream => "raw_compressed_stream",
        }
    }
}

/// One OCR-ready image recovered from a page's image XObjects, with the provenance
/// needed to tag it in the extraction output.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[derive(Debug, Clone)]
pub(crate) struct PageFallbackImage {
    /// Standalone image file bytes an OCR backend can decode directly.
    pub bytes: Bytes,
    /// Image format of `bytes`, as reported by [`detect_image_format_from_bytes`].
    pub format: &'static str,
    /// Which recovery mode produced `bytes`.
    pub recovery: XObjectRecovery,
}

/// Collect OCR-ready image bytes for every image XObject on `page_idx`, for use as a
/// `force_ocr` fallback when whole-page rasterization silently dropped an undecodable
/// image (issue #1355).
///
/// `page_image_handles` is a cheap content-stream/CTM pass that succeeds even when the
/// renderer could not paint an image (e.g. an unsupported codec that xberg_native_pdf's page
/// renderer silently substitutes with a blank page). For each handle:
/// - `decode()` success → re-encode raw pixels to PNG, or pass the embedded JPEG through
///   as-is.
/// - `decode()` failure on a DCTDecode/JPXDecode stream → hand back the raw compressed
///   bytes, which are a valid standalone JPEG/JP2 file the OCR backend can decode itself.
/// - Any other failure → skip the image; there is no way to recover pixel data from it.
///
/// Returned in content-stream paint order; empty when the page has no image XObjects or
/// none of them yielded usable bytes.
///
// Available under `ocr-pipeline` too (not just `ocr`): its callers in
// `crate::extractors::pdf::ocr` are gated `any(ocr, ocr-pipeline)`, and the `binstall`
// CLI profile pulls `ocr-pipeline` (via `liter-llm`) without `ocr`. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn page_ocr_fallback_image_bytes(
    doc: &xberg_native_pdf::PdfDocument,
    page_idx: usize,
) -> Vec<PageFallbackImage> {
    let handles = match doc.page_image_handles(page_idx) {
        Ok(h) => h,
        Err(error) => {
            tracing::debug!(
                page = page_idx,
                "force_ocr fallback: enumerating image handles failed: {error}"
            );
            return Vec::new();
        }
    };

    let mut out = Vec::with_capacity(handles.len());
    for handle in &handles {
        match handle.decode() {
            Ok(img) => match img.data() {
                xberg_native_pdf::extractors::ImageData::Jpeg(jpeg_bytes) => out.push(PageFallbackImage {
                    bytes: Bytes::copy_from_slice(jpeg_bytes),
                    format: "jpeg",
                    recovery: XObjectRecovery::EmbeddedJpeg,
                }),
                xberg_native_pdf::extractors::ImageData::Raw { pixels, format } => {
                    match raw_pixels_to_png(img.width(), img.height(), format, pixels) {
                        Ok(bytes) => out.push(PageFallbackImage {
                            bytes,
                            format: "png",
                            recovery: XObjectRecovery::ReencodedPixels,
                        }),
                        Err(error) => {
                            tracing::debug!(page = page_idx, "force_ocr fallback: raw re-encode failed: {error}");
                        }
                    }
                }
            },
            Err(decode_err) => {
                let passthrough = matches!(
                    handle.filter_chain.last(),
                    Some(xberg_native_pdf::PdfFilter::DCTDecode) | Some(xberg_native_pdf::PdfFilter::JPXDecode)
                );
                if passthrough {
                    match handle.raw_compressed_bytes() {
                        Ok(raw) if matches!(detect_image_format_from_bytes(&raw), "jpeg" | "jpeg2000") => {
                            let format = detect_image_format_from_bytes(&raw);
                            out.push(PageFallbackImage {
                                bytes: Bytes::from(raw),
                                format,
                                recovery: XObjectRecovery::RawCompressedStream,
                            });
                        }
                        Ok(_) => {
                            tracing::debug!(
                                page = page_idx,
                                "force_ocr fallback: raw bytes not a recognizable JPEG/JP2"
                            );
                        }
                        Err(error) => {
                            tracing::debug!(
                                page = page_idx,
                                "force_ocr fallback: raw_compressed_bytes failed: {error}"
                            );
                        }
                    }
                } else {
                    tracing::debug!(
                        page = page_idx,
                        "force_ocr fallback: undecodable image, non-JPEG codec: {decode_err}"
                    );
                }
            }
        }
    }
    out
}

/// Page numbers (1-based) whose PDF-space dimensions are known and whose native text layer is
/// non-blank -- the two preconditions `should_skip_pdf_image_ocr` (`core/pipeline/mod.rs`)
/// checks before excluding a full-page image from OCR, computed here so
/// [`extract_page_images`] can skip that image's decode (and therefore its PNG re-encode)
/// entirely instead of throwing the result away later (GH#1732). Keyed to `(width, height)` in
/// PDF points.
///
/// Built from the `PageStructure` already computed alongside native text extraction
/// (`pdf::native::metadata::build_page_structure`), so this costs nothing beyond a small map
/// build -- no extra document access. Empty when `page_structure` is `None` (no page boundary
/// tracking) or carries no per-page info, which simply means no page qualifies and every image
/// goes through the ordinary decode path, exactly as before this fix.
#[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
pub(crate) fn ocr_skip_candidate_pages(
    page_structure: Option<&crate::types::PageStructure>,
) -> std::collections::HashMap<u32, (f64, f64)> {
    let mut pages = std::collections::HashMap::new();
    let Some(page_infos) = page_structure.and_then(|structure| structure.pages.as_ref()) else {
        return pages;
    };
    for page in page_infos {
        if page.is_blank == Some(false)
            && let Some(dimensions) = page.dimensions
        {
            pages.insert(page.number, (dimensions.width, dimensions.height));
        }
    }
    pages
}

/// Whether a handle's pre-decode bounding box covers enough of its page to be the "full-page
/// image" `should_skip_pdf_image_ocr` (`core/pipeline/mod.rs`) would exclude from OCR (GH#1732).
/// Mirrors that function's own area-ratio test exactly, sharing its
/// [`crate::core::pipeline::FULL_PAGE_IMAGE_AREA_RATIO`] constant so the two decisions cannot
/// drift apart.
#[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
fn covers_full_page(bbox: &xberg_native_pdf::geometry::Rect, page_width: f64, page_height: f64) -> bool {
    if page_width <= 0.0 || page_height <= 0.0 {
        return false;
    }
    let image_area = f64::from(bbox.width) * f64::from(bbox.height);
    image_area / (page_width * page_height) >= crate::core::pipeline::FULL_PAGE_IMAGE_AREA_RATIO
}

/// Without OCR compiled in, nothing ever drops an OCR-skipped image, so it is never safe to
/// treat one as unreadable -- always decode. Keeps [`outcomes_from_uncapped_page`] free of its
/// own `#[cfg]` branch. ~keep
#[cfg(not(all(feature = "ocr", feature = "tokio-runtime")))]
fn covers_full_page(_bbox: &xberg_native_pdf::geometry::Rect, _page_width: f64, _page_height: f64) -> bool {
    false
}

/// Build the metadata-only [`crate::types::ExtractedImage`] for a handle whose decode is being
/// skipped entirely (GH#1732): dimensions, bounding box, page number and alt text come from the
/// handle's cheap Phase-1 fields, `data` stays empty, and `format` is tagged `"skipped"` so a
/// consumer inspecting `format` cannot mistake it for real, decoded image bytes.
fn skipped_image_outcome(
    handle: &xberg_native_pdf::PdfImageHandle<'_>,
    page_number: u32,
    alt_text: Option<String>,
) -> PageImageOutcome {
    Ok(crate::types::ExtractedImage {
        data: Bytes::new(),
        format: Cow::Borrowed("skipped"),
        // Replaced with the document-global index by `extract_images_with_data`. ~keep
        image_index: 0,
        page_number: Some(page_number),
        width: Some(handle.width),
        height: Some(handle.height),
        colorspace: Some(format!("{:?}", handle.color_space)),
        bits_per_component: Some(handle.bits_per_component as u32),
        is_mask: false,
        description: alt_text,
        ocr_result: None,
        bounding_box: Some(crate::types::BoundingBox {
            x0: handle.bbox.x as f64,
            y0: handle.bbox.y as f64,
            x1: (handle.bbox.x + handle.bbox.width) as f64,
            y1: (handle.bbox.y + handle.bbox.height) as f64,
        }),
        source_path: None,
        image_kind: None,
        kind_confidence: None,
        cluster_id: None,
        caption: None,
        qr_codes: None,
        data_base64: None,
    })
}

/// Convert one already-decoded [`xberg_native_pdf::extractors::PdfImage`] into its
/// [`PageImageOutcome`]: `Ok` on a successful (or pass-through JPEG) encode, `Err` when a raw
/// pixel buffer could not be re-encoded to PNG (issue #71). Shared by both the capped and
/// uncapped extraction paths so the conversion logic exists once.
fn image_outcome_from_decoded(
    native_img: &xberg_native_pdf::extractors::PdfImage,
    page_number: u32,
    alt_text: Option<String>,
) -> PageImageOutcome {
    let (data, format) = match native_img.data() {
        xberg_native_pdf::extractors::ImageData::Jpeg(jpeg_bytes) => {
            let data_bytes = Bytes::copy_from_slice(jpeg_bytes);
            let actual_format = detect_image_format_from_bytes(data_bytes.as_ref());
            (data_bytes, Cow::Borrowed(actual_format))
        }
        xberg_native_pdf::extractors::ImageData::Raw { pixels, format } => {
            let bytes = raw_pixels_to_png(native_img.width(), native_img.height(), format, pixels)?;
            (bytes, Cow::Borrowed("png"))
        }
    };

    Ok(crate::types::ExtractedImage {
        data,
        format,
        // Replaced with the document-global index by `extract_images_with_data`. ~keep
        image_index: 0,
        page_number: Some(page_number),
        width: Some(native_img.width()),
        height: Some(native_img.height()),
        colorspace: Some(format!("{:?}", native_img.color_space())),
        bits_per_component: Some(native_img.bits_per_component() as u32),
        is_mask: false,
        description: alt_text,
        ocr_result: None,
        bounding_box: native_img.bbox().map(|r| crate::types::BoundingBox {
            x0: r.x as f64,
            y0: r.y as f64,
            x1: (r.x + r.width) as f64,
            y1: (r.y + r.height) as f64,
        }),
        source_path: None,
        image_kind: None,
        kind_confidence: None,
        cluster_id: None,
        caption: None,
        qr_codes: None,
        data_base64: None,
    })
}

/// Extract full image data from all pages of a PDF.
///
/// Returns a `Vec<ExtractedImage>` with complete image data and metadata, plus any
/// non-fatal `ProcessingWarning`s produced along the way (e.g. an image that could
/// not be re-encoded and was skipped — see issue #71).
/// When image extraction is disabled or no images are found, returns an empty vec.
///
/// # Arguments
///
/// * `doc` - Mutable reference to the native document
/// * `max_images_per_page` - Optional limit on images per page
/// * `cancel_token` - Optional cancellation token checked between pages
///
/// # Returns
///
/// A `Vec<ExtractedImage>` containing all extracted images with their data, and a
/// `Vec<ProcessingWarning>` describing any images that were skipped.
pub(crate) fn extract_images_with_data(
    doc: &mut NativeDocument,
    max_images_per_page: Option<u32>,
    cancel_token: Option<&CancellationToken>,
    ocr_skip_candidate_pages: &std::collections::HashMap<u32, (f64, f64)>,
) -> Result<(Vec<crate::types::ExtractedImage>, Vec<crate::types::ProcessingWarning>)> {
    if max_images_per_page == Some(0) {
        return Ok((Vec::new(), Vec::new()));
    }

    tracing::debug!(
        target: "xberg::pdf::native::images",
        event = "decompression_started",
        "extract_images_with_data entered"
    );

    let page_count = doc
        .doc
        .page_count()
        .map_err(|e| PdfError::MetadataExtractionFailed(format!("xberg_native_pdf: failed to get page count: {e}")))?;

    // Tagged-PDF `/Alt` text for `Figure` structure elements, keyed by 0-based page
    // index (issue #62). Empty for the (common) untagged-PDF case. It needs `&mut`, so
    // it runs before the page pass reborrows the document as a shared reference. ~keep
    let alt_text_by_page = super::hierarchy::extract_figure_alt_text_by_page(doc);
    let doc: &NativeDocument = doc;

    let per_page = extract_all_page_images(
        doc,
        page_count,
        max_images_per_page,
        &alt_text_by_page,
        cancel_token,
        ocr_skip_candidate_pages,
    );

    // The document-global `image_index` and the skipped-image warnings are assigned here,
    // in page order, so they do not depend on which thread ran which page. A skipped image
    // takes the index it would have had without consuming it, as the sequential loop did. ~keep
    let mut all_images = Vec::new();
    let mut warnings = Vec::new();
    let mut global_index = 0u32;
    for (page_idx, outcomes) in per_page.into_iter().enumerate() {
        let page_number = (page_idx + 1) as u32;
        for outcome in outcomes {
            match outcome {
                Ok(mut image) => {
                    image.image_index = global_index;
                    all_images.push(image);
                    global_index += 1;
                }
                Err(error) => {
                    tracing::warn!(
                        page = page_number,
                        image_index = global_index,
                        "skipping raw PDF image that could not be re-encoded: {error}"
                    );
                    warnings.push(unencodable_image_warning(global_index, page_number, &error));
                }
            }
        }
    }

    Ok((all_images, warnings))
}

/// One image's outcome on one page, in content-stream paint order: `Ok` is an image still
/// waiting for its document-global index, `Err` is the re-encode failure that skipped it.
type PageImageOutcome = std::result::Result<crate::types::ExtractedImage, PdfError>;

/// Run [`extract_page_images`] over every page, in parallel across the thread budget.
///
/// Decoding a page's embedded images is CPU-bound and independent of every other page, and
/// `xberg_native_pdf::PdfDocument` is documented `Send + Sync` with its interior mutability
/// behind mutexes for exactly this reason. This pass used to be a plain `for page_idx in
/// 0..page_count`, so configuring OCR -- which switches whole-document image extraction on
/// through `ExtractionConfig::needs_image_data` -- added a single-threaded prologue that no
/// thread budget could shorten (issue #1732). It is the same shape as the page-rendering
/// pass in `extractors/pdf/ocr/rendering.rs` (issue #1666).
///
/// `into_par_iter()` over a range is an `IndexedParallelIterator`, so `collect()` returns
/// the pages in index order and the caller's numbering is unchanged.
fn extract_all_page_images(
    doc: &NativeDocument,
    page_count: usize,
    max_images_per_page: Option<u32>,
    alt_text_by_page: &std::collections::HashMap<u32, Vec<Option<String>>>,
    cancel_token: Option<&CancellationToken>,
    ocr_skip_candidate_pages: &std::collections::HashMap<u32, (f64, f64)>,
) -> Vec<Vec<PageImageOutcome>> {
    // rayon's work-stealing pool needs OS threads; wasm32 has none, so it falls back to a
    // sequential iterator there, matching the gate on the paragraph pass in
    // `pdf/structure/pipeline.rs`. ~keep
    #[cfg(not(target_arch = "wasm32"))]
    {
        use rayon::prelude::*;
        (0..page_count)
            .into_par_iter()
            .map(|page_idx| {
                extract_page_images(
                    doc,
                    page_idx,
                    max_images_per_page,
                    alt_text_by_page,
                    cancel_token,
                    ocr_skip_candidate_pages,
                )
            })
            .collect()
    }
    #[cfg(target_arch = "wasm32")]
    {
        (0..page_count)
            .map(|page_idx| {
                extract_page_images(
                    doc,
                    page_idx,
                    max_images_per_page,
                    alt_text_by_page,
                    cancel_token,
                    ocr_skip_candidate_pages,
                )
            })
            .collect()
    }
}

/// Decode one page's embedded images, in content-stream paint order.
///
/// Returns an empty vec when the page yields no images, when the page could not be read, or
/// when `cancel_token` has already fired.
///
/// `ocr_skip_candidate_pages` names pages where a full-page image can skip its decode entirely
/// (GH#1732) -- non-empty only when the caller already determined OCR is the sole reason images
/// were requested for this document at all. Applied only to the uncapped path
/// (`max_images_per_page == None`), the common/default case and the one issue #1732 measured;
/// the capped path is left unchanged to keep this change narrow.
fn extract_page_images(
    doc: &NativeDocument,
    page_idx: usize,
    max_images_per_page: Option<u32>,
    alt_text_by_page: &std::collections::HashMap<u32, Vec<Option<String>>>,
    cancel_token: Option<&CancellationToken>,
    ocr_skip_candidate_pages: &std::collections::HashMap<u32, (f64, f64)>,
) -> Vec<PageImageOutcome> {
    #[cfg(test)]
    record_page_thread();

    if cancel_token.is_some_and(|t| t.is_cancelled()) {
        return Vec::new();
    }

    let page_number = (page_idx + 1) as u32;
    let page_alt_texts = alt_text_by_page.get(&(page_idx as u32));

    match max_images_per_page.map(|n| n as usize) {
        Some(limit) => outcomes_from_capped_page(doc, page_idx, limit, page_number, page_alt_texts),
        None => outcomes_from_uncapped_page(doc, page_idx, page_number, page_alt_texts, ocr_skip_candidate_pages),
    }
}

/// The existing eager/capped path: decode every handle up to `limit`, falling back to the eager
/// extractor when the handle-based pass comes back empty. Unchanged behavior from before
/// GH#1732's decode-skip fast path, which applies only to [`outcomes_from_uncapped_page`].
fn outcomes_from_capped_page(
    doc: &NativeDocument,
    page_idx: usize,
    limit: usize,
    page_number: u32,
    page_alt_texts: Option<&Vec<Option<String>>>,
) -> Vec<PageImageOutcome> {
    let handle_images = match extract_n_images_from_page_handles(doc, page_idx, limit) {
        Ok(images) => images,
        Err(error) => {
            tracing::debug!(
                page = page_idx,
                "capped image-handle extraction failed; falling back to eager extraction: {error}"
            );
            Vec::new()
        }
    };
    let native_images = if !handle_images.is_empty() {
        handle_images
    } else {
        match doc.doc.extract_images(page_idx) {
            Ok(imgs) => imgs.into_iter().take(limit).collect(),
            Err(e) => {
                tracing::debug!(
                    page = page_idx,
                    "xberg_native_pdf: failed to extract images (fallback): {e}"
                );
                return Vec::new();
            }
        }
    };
    decoded_images_to_outcomes(native_images, page_number, page_alt_texts)
}

/// The uncapped path (the common/default case, and the one issue #1732 measured): enumerate
/// this page's images via the cheap Phase-1 handle walk, decoding only the handles that are
/// not both full-page and OCR-skip-eligible. Falls back to the eager extractor only when handle
/// enumeration itself fails, mirroring the capped path's existing fallback-on-failure behavior.
fn outcomes_from_uncapped_page(
    doc: &NativeDocument,
    page_idx: usize,
    page_number: u32,
    page_alt_texts: Option<&Vec<Option<String>>>,
    ocr_skip_candidate_pages: &std::collections::HashMap<u32, (f64, f64)>,
) -> Vec<PageImageOutcome> {
    let handles = match doc.doc.page_image_handles(page_idx) {
        Ok(h) => h,
        Err(error) => {
            tracing::debug!(
                page = page_idx,
                "failed to enumerate image handles; falling back to eager extraction: {error}"
            );
            Vec::new()
        }
    };
    if handles.is_empty() {
        return match doc.doc.extract_images(page_idx) {
            Ok(imgs) => decoded_images_to_outcomes(imgs, page_number, page_alt_texts),
            Err(e) => {
                tracing::debug!(page = page_idx, "xberg_native_pdf: failed to extract images: {e}");
                Vec::new()
            }
        };
    }

    let page_dimensions = ocr_skip_candidate_pages.get(&page_number).copied();
    let mut outcomes = Vec::with_capacity(handles.len());
    for (page_image_position, handle) in handles.iter().enumerate() {
        let alt_text = page_alt_texts
            .and_then(|alts| alts.get(page_image_position))
            .and_then(|alt| alt.clone());
        if page_dimensions.is_some_and(|(width, height)| covers_full_page(&handle.bbox, width, height)) {
            outcomes.push(skipped_image_outcome(handle, page_number, alt_text));
            continue;
        }
        match handle.decode() {
            Ok(native_img) => outcomes.push(image_outcome_from_decoded(&native_img, page_number, alt_text)),
            Err(error) => {
                tracing::debug!(page = page_idx, "image decompression failed: {error}");
            }
        }
    }
    outcomes
}

/// Convert a page's already-decoded images into outcomes, looking up each one's alt text by its
/// content-stream paint position. Shared by the capped path and the uncapped path's
/// handle-enumeration-failed fallback.
fn decoded_images_to_outcomes(
    native_images: Vec<xberg_native_pdf::extractors::PdfImage>,
    page_number: u32,
    page_alt_texts: Option<&Vec<Option<String>>>,
) -> Vec<PageImageOutcome> {
    let mut outcomes = Vec::with_capacity(native_images.len());
    for (page_image_position, native_img) in native_images.iter().enumerate() {
        let alt_text = page_alt_texts
            .and_then(|alts| alts.get(page_image_position))
            .and_then(|alt| alt.clone());
        outcomes.push(image_outcome_from_decoded(native_img, page_number, alt_text));
    }
    outcomes
}

/// Test-only record of which threads ran a page's image extraction, so a test can assert that
/// the pass dispatched across the pool rather than infer it from wall clock, which flakes
/// under load. Mirrors `RENDER_CALL_THREAD_IDS` in `extractors/pdf/ocr/rendering.rs`.
///
/// It records the thread NAME rather than its id because the record is process-global while
/// `#[serial_test::serial]` excludes only other `#[serial]` tests: any test running beside the
/// guard that reaches this pass would otherwise add its own threads to the set and let a
/// sequential regression read as a wide one. A guard names the threads of the pool it builds
/// and counts only those, so the scope is a property of the pool rather than of which tests
/// happen to run alongside. `core/config/concurrency.rs` records the same defect class
/// against #215. ~keep
#[cfg(test)]
static PAGE_CALL_THREAD_NAMES: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<String>>> =
    std::sync::OnceLock::new();

#[cfg(test)]
fn page_call_thread_names() -> &'static std::sync::Mutex<std::collections::HashSet<String>> {
    PAGE_CALL_THREAD_NAMES.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()))
}

#[cfg(test)]
fn record_page_thread() {
    let current = std::thread::current();
    let name = current.name().unwrap_or("<unnamed>");
    let mut names = page_call_thread_names()
        .lock()
        .expect("page-thread record must not be poisoned");
    if !names.contains(name) {
        names.insert(name.to_owned());
    }
}

#[cfg(test)]
mod tests;
