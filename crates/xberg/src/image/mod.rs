/// DPI detection and normalization for scanned images.
pub mod dpi;
// Both need the `ocr-pipeline` dependency set (`fast_image_resize` in particular, which
// `layout-detection` does not pull); only `dpi` is dependency-free enough to build under
// `layout-detection` alone. ~keep
/// Image preprocessing pipeline: denoising, deskew, binarization, rotation.
#[cfg(feature = "ocr-pipeline")]
pub mod preprocessing;
/// Image resize helpers used before OCR to normalize resolution.
#[cfg(feature = "ocr-pipeline")]
pub mod resize;
/// Text-size estimate used to pick a resample factor for a raster of unknown resolution.
#[cfg(feature = "ocr-pipeline")]
pub(crate) mod text_height;

// Re-exported only for the Tesseract processor (`ocr::processor::execution`); the
// standalone-image path calls `preprocessing::normalize_image_dpi_owned` by full path,
// so under `ocr-pipeline` alone this alias has no consumer.
#[cfg(feature = "ocr")]
pub(crate) use preprocessing::normalize_image_dpi_owned;
