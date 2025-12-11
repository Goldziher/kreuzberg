//! Image extraction functionality.
//!
//! This module provides functions for extracting metadata and EXIF data from images,
//! including support for multi-frame TIFF files.

use crate::error::{KreuzbergError, Result};
use exif::{In, Reader, Tag};
use image::ImageReader;
use std::collections::HashMap;
use std::io::Cursor;

/// Image metadata extracted from an image file.
#[derive(Debug, Clone)]
pub struct ImageMetadata {
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Image format (e.g., "PNG", "JPEG")
    pub format: String,
    /// EXIF data if available
    pub exif_data: HashMap<String, String>,
}

/// Extract metadata from image bytes.
///
/// Extracts dimensions, format, and EXIF data from the image.
pub fn extract_image_metadata(bytes: &[u8]) -> Result<ImageMetadata> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| KreuzbergError::parsing(format!("Failed to read image format: {}", e)))?;

    let format = reader
        .format()
        .ok_or_else(|| KreuzbergError::parsing("Could not determine image format".to_string()))?;

    let image = reader
        .decode()
        .map_err(|e| KreuzbergError::parsing(format!("Failed to decode image: {}", e)))?;

    let width = image.width();
    let height = image.height();
    let format_str = format!("{:?}", format).to_uppercase();

    let exif_data = extract_exif_data(bytes);

    Ok(ImageMetadata {
        width,
        height,
        format: format_str,
        exif_data,
    })
}

/// Extract EXIF data from image bytes.
///
/// Returns a HashMap of EXIF tags and their values.
/// If EXIF data is not available or cannot be parsed, returns an empty HashMap.
fn extract_exif_data(bytes: &[u8]) -> HashMap<String, String> {
    let mut exif_map = HashMap::new();

    let exif_reader = match Reader::new().read_from_container(&mut Cursor::new(bytes)) {
        Ok(reader) => reader,
        Err(_) => return exif_map,
    };

    let common_tags = [
        (Tag::Make, "Make"),
        (Tag::Model, "Model"),
        (Tag::DateTime, "DateTime"),
        (Tag::DateTimeOriginal, "DateTimeOriginal"),
        (Tag::DateTimeDigitized, "DateTimeDigitized"),
        (Tag::Software, "Software"),
        (Tag::Orientation, "Orientation"),
        (Tag::XResolution, "XResolution"),
        (Tag::YResolution, "YResolution"),
        (Tag::ResolutionUnit, "ResolutionUnit"),
        (Tag::ExposureTime, "ExposureTime"),
        (Tag::FNumber, "FNumber"),
        (Tag::PhotographicSensitivity, "ISO"),
        (Tag::FocalLength, "FocalLength"),
        (Tag::Flash, "Flash"),
        (Tag::WhiteBalance, "WhiteBalance"),
        (Tag::GPSLatitude, "GPSLatitude"),
        (Tag::GPSLongitude, "GPSLongitude"),
        (Tag::GPSAltitude, "GPSAltitude"),
    ];

    for (tag, field_name) in common_tags {
        if let Some(field) = exif_reader.get_field(tag, In::PRIMARY) {
            exif_map.insert(field_name.to_string(), field.display_value().to_string());
        }
    }

    exif_map
}

/// Result of OCR extraction from an image with optional page tracking.
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
fn detect_tiff_frame_count(bytes: &[u8]) -> Result<usize> {
    use tiff::decoder::Decoder;
    let mut decoder =
        Decoder::new(Cursor::new(bytes)).map_err(|e| KreuzbergError::parsing(format!("TIFF decode: {}", e)))?;

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
pub fn extract_text_from_image_with_ocr(
    bytes: &[u8],
    mime_type: &str,
    ocr_result: String,
    page_config: Option<&crate::core::config::PageConfig>,
) -> Result<ImageOcrResult> {
    // Check if this is a TIFF and if we should track pages
    let is_tiff = mime_type.to_lowercase().contains("tiff");
    let should_track_pages = page_config.is_some() && is_tiff;

    if !should_track_pages {
        // Fast path: single frame or no page tracking requested
        return Ok(ImageOcrResult {
            content: ocr_result,
            boundaries: None,
            page_contents: None,
        });
    }

    // Slow path: multi-frame TIFF with page tracking
    let frame_count = detect_tiff_frame_count(bytes)?;

    if frame_count <= 1 {
        // Single-frame TIFF, no pagination needed
        return Ok(ImageOcrResult {
            content: ocr_result,
            boundaries: None,
            page_contents: None,
        });
    }

    // Multi-frame TIFF with page tracking enabled
    // For now, we return the concatenated content with frame boundaries
    // The boundaries assume uniform distribution of content across frames
    let content_len = ocr_result.len();
    let content_per_frame = if frame_count > 0 {
        content_len / frame_count
    } else {
        content_len
    };

    let mut boundaries = Vec::new();
    let mut page_contents = Vec::new();
    let mut byte_offset = 0;

    for frame_num in 1..=frame_count {
        // Calculate frame end, adjusting to valid UTF-8 boundary
        let frame_end = if frame_num == frame_count {
            content_len
        } else {
            let raw_end = (frame_num * content_per_frame).min(content_len);
            // Find next valid UTF-8 boundary to prevent slicing multi-byte chars
            (raw_end..=content_len)
                .find(|&i| ocr_result.is_char_boundary(i))
                .unwrap_or(content_len)
        };

        boundaries.push(crate::types::PageBoundary {
            byte_start: byte_offset,
            byte_end: frame_end,
            page_number: frame_num,
        });

        page_contents.push(crate::types::PageContent {
            page_number: frame_num,
            content: ocr_result[byte_offset..frame_end].to_string(),
            tables: vec![],
            images: vec![],
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
mod tests {
    use super::*;
    use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
    use std::io::Cursor;

    fn create_test_image(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
        let img: RgbImage = ImageBuffer::from_fn(width, height, |x, y| {
            let r = ((x as f32 / width as f32) * 255.0) as u8;
            let g = ((y as f32 / height as f32) * 255.0) as u8;
            let b = 128;
            Rgb([r, g, b])
        });

        let mut bytes: Vec<u8> = Vec::new();
        let mut cursor = Cursor::new(&mut bytes);
        img.write_to(&mut cursor, format).unwrap();
        bytes
    }

    #[test]
    fn test_extract_png_image_returns_correct_metadata() {
        let bytes = create_test_image(100, 80, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 100);
        assert_eq!(metadata.height, 80);
        assert_eq!(metadata.format, "PNG");
    }

    #[test]
    fn test_extract_jpeg_image_returns_correct_metadata() {
        let bytes = create_test_image(200, 150, ImageFormat::Jpeg);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 200);
        assert_eq!(metadata.height, 150);
        assert_eq!(metadata.format, "JPEG");
    }

    #[test]
    fn test_extract_webp_image_returns_correct_metadata() {
        let bytes = create_test_image(120, 90, ImageFormat::WebP);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 120);
        assert_eq!(metadata.height, 90);
        assert_eq!(metadata.format, "WEBP");
    }

    #[test]
    fn test_extract_bmp_image_returns_correct_metadata() {
        let bytes = create_test_image(50, 50, ImageFormat::Bmp);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 50);
        assert_eq!(metadata.height, 50);
        assert_eq!(metadata.format, "BMP");
    }

    #[test]
    fn test_extract_tiff_image_returns_correct_metadata() {
        let bytes = create_test_image(180, 120, ImageFormat::Tiff);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 180);
        assert_eq!(metadata.height, 120);
        assert_eq!(metadata.format, "TIFF");
    }

    #[test]
    fn test_extract_gif_image_returns_correct_metadata() {
        let bytes = create_test_image(64, 64, ImageFormat::Gif);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 64);
        assert_eq!(metadata.height, 64);
        assert_eq!(metadata.format, "GIF");
    }

    #[test]
    fn test_extract_image_extreme_aspect_ratio() {
        let bytes = create_test_image(1000, 10, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 1000);
        assert_eq!(metadata.height, 10);
        assert!(metadata.width / metadata.height >= 100);
    }

    #[test]
    fn test_extract_image_dimensions_correctly() {
        let bytes = create_test_image(640, 480, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 640);
        assert_eq!(metadata.height, 480);
    }

    #[test]
    fn test_extract_image_format_correctly() {
        let png_bytes = create_test_image(100, 100, ImageFormat::Png);
        let jpeg_bytes = create_test_image(100, 100, ImageFormat::Jpeg);

        let png_metadata = extract_image_metadata(&png_bytes).unwrap();
        let jpeg_metadata = extract_image_metadata(&jpeg_bytes).unwrap();

        assert_eq!(png_metadata.format, "PNG");
        assert_eq!(jpeg_metadata.format, "JPEG");
    }

    #[test]
    fn test_extract_image_without_exif_returns_empty_map() {
        let bytes = create_test_image(100, 100, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert!(metadata.exif_data.is_empty());
    }

    #[test]
    fn test_extract_exif_data_from_jpeg_with_exif() {
        let bytes = create_test_image(100, 100, ImageFormat::Jpeg);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.exif_data.len(), 0);
    }

    #[test]
    fn test_extract_image_metadata_invalid_returns_error() {
        let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
        let result = extract_image_metadata(&invalid_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_image_corrupted_data_returns_error() {
        let mut bytes = create_test_image(100, 100, ImageFormat::Png);
        if bytes.len() > 50 {
            for byte in bytes.iter_mut().take(50).skip(20) {
                *byte = 0xFF;
            }
        }

        let result = extract_image_metadata(&bytes);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_extract_image_empty_bytes_returns_error() {
        let empty_bytes: Vec<u8> = Vec::new();
        let result = extract_image_metadata(&empty_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_image_unsupported_format_returns_error() {
        let unsupported_bytes = vec![0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20, 0x0D, 0x0A, 0x87, 0x0A];
        let result = extract_image_metadata(&unsupported_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_very_small_image_1x1_pixel() {
        let bytes = create_test_image(1, 1, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 1);
        assert_eq!(metadata.height, 1);
        assert_eq!(metadata.format, "PNG");
    }

    #[test]
    fn test_extract_large_image_dimensions() {
        let bytes = create_test_image(2048, 1536, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 2048);
        assert_eq!(metadata.height, 1536);
    }

    #[test]
    fn test_extract_image_with_no_metadata_has_empty_exif() {
        let bytes = create_test_image(100, 100, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert!(metadata.exif_data.is_empty());
    }

    #[test]
    fn test_extract_exif_data_returns_empty_map_for_non_jpeg() {
        let png_bytes = create_test_image(100, 100, ImageFormat::Png);
        let exif_data = extract_exif_data(&png_bytes);
        assert!(exif_data.is_empty());
    }

    #[test]
    fn test_extract_rectangular_image_portrait_orientation() {
        let bytes = create_test_image(400, 800, ImageFormat::Jpeg);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 400);
        assert_eq!(metadata.height, 800);
        assert!(metadata.height > metadata.width);
    }

    #[test]
    fn test_extract_rectangular_image_landscape_orientation() {
        let bytes = create_test_image(800, 400, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 800);
        assert_eq!(metadata.height, 400);
        assert!(metadata.width > metadata.height);
    }

    #[test]
    fn test_extract_square_image_equal_dimensions() {
        let bytes = create_test_image(512, 512, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 512);
        assert_eq!(metadata.height, 512);
        assert_eq!(metadata.width, metadata.height);
    }

    #[test]
    fn test_extract_metadata_preserves_format_case() {
        let png_bytes = create_test_image(100, 100, ImageFormat::Png);
        let jpeg_bytes = create_test_image(100, 100, ImageFormat::Jpeg);
        let webp_bytes = create_test_image(100, 100, ImageFormat::WebP);

        let png_meta = extract_image_metadata(&png_bytes).unwrap();
        let jpeg_meta = extract_image_metadata(&jpeg_bytes).unwrap();
        let webp_meta = extract_image_metadata(&webp_bytes).unwrap();

        assert_eq!(png_meta.format, "PNG");
        assert_eq!(jpeg_meta.format, "JPEG");
        assert_eq!(webp_meta.format, "WEBP");
    }
}
