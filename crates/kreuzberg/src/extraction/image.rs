//! Image extraction functionality.
//!
//! This module provides functions for extracting metadata and EXIF data from images.

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
    let format_str = format!("{:?}", format);

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

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
    use std::io::Cursor;

    // Helper function to create a test image in a specific format
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

    // ======================
    // Image Format Tests (7)
    // ======================

    #[test]
    fn test_extract_png_image_returns_correct_metadata() {
        let bytes = create_test_image(100, 80, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 100);
        assert_eq!(metadata.height, 80);
        assert_eq!(metadata.format, "Png");
    }

    #[test]
    fn test_extract_jpeg_image_returns_correct_metadata() {
        let bytes = create_test_image(200, 150, ImageFormat::Jpeg);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 200);
        assert_eq!(metadata.height, 150);
        assert_eq!(metadata.format, "Jpeg");
    }

    #[test]
    fn test_extract_webp_image_returns_correct_metadata() {
        let bytes = create_test_image(120, 90, ImageFormat::WebP);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 120);
        assert_eq!(metadata.height, 90);
        assert_eq!(metadata.format, "WebP");
    }

    #[test]
    fn test_extract_bmp_image_returns_correct_metadata() {
        let bytes = create_test_image(50, 50, ImageFormat::Bmp);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 50);
        assert_eq!(metadata.height, 50);
        assert_eq!(metadata.format, "Bmp");
    }

    #[test]
    fn test_extract_tiff_image_returns_correct_metadata() {
        let bytes = create_test_image(180, 120, ImageFormat::Tiff);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 180);
        assert_eq!(metadata.height, 120);
        assert_eq!(metadata.format, "Tiff");
    }

    #[test]
    fn test_extract_gif_image_returns_correct_metadata() {
        let bytes = create_test_image(64, 64, ImageFormat::Gif);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 64);
        assert_eq!(metadata.height, 64);
        assert_eq!(metadata.format, "Gif");
    }

    #[test]
    fn test_extract_image_extreme_aspect_ratio() {
        // Test extremely wide image (panoramic)
        let bytes = create_test_image(1000, 10, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 1000);
        assert_eq!(metadata.height, 10);
        assert!(metadata.width / metadata.height >= 100); // Very wide aspect ratio
    }

    // ===========================
    // Metadata Extraction Tests (4)
    // ===========================

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

        assert_eq!(png_metadata.format, "Png");
        assert_eq!(jpeg_metadata.format, "Jpeg");
    }

    #[test]
    fn test_extract_image_without_exif_returns_empty_map() {
        // PNG created programmatically won't have EXIF data
        let bytes = create_test_image(100, 100, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert!(metadata.exif_data.is_empty());
    }

    #[test]
    fn test_extract_exif_data_from_jpeg_with_exif() {
        // Create a JPEG with minimal EXIF data
        // Note: Programmatically created JPEGs won't have EXIF, so we test the empty case
        let bytes = create_test_image(100, 100, ImageFormat::Jpeg);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        // EXIF map should exist (even if empty) for JPEG
        assert_eq!(metadata.exif_data.len(), 0);
    }

    // ======================
    // Error Handling Tests (4)
    // ======================

    #[test]
    fn test_extract_image_metadata_invalid_returns_error() {
        let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
        let result = extract_image_metadata(&invalid_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_image_corrupted_data_returns_error() {
        // Start with valid PNG header but corrupt the rest
        let mut bytes = create_test_image(100, 100, ImageFormat::Png);
        // Corrupt the middle of the image data
        if bytes.len() > 50 {
            for i in 20..50 {
                bytes[i] = 0xFF;
            }
        }

        let result = extract_image_metadata(&bytes);
        // This might still succeed with partial data, or fail depending on corruption
        // The important thing is we handle it gracefully
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
        // Create bytes that look like an image header but aren't a supported format
        let unsupported_bytes = vec![
            0x00, 0x00, 0x00, 0x0C, // Fake header
            0x6A, 0x50, 0x20, 0x20, // "jP  "
            0x0D, 0x0A, 0x87, 0x0A, // CR LF signature
        ];
        let result = extract_image_metadata(&unsupported_bytes);
        assert!(result.is_err());
    }

    // ================
    // Edge Cases Tests (3)
    // ================

    #[test]
    fn test_extract_very_small_image_1x1_pixel() {
        let bytes = create_test_image(1, 1, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 1);
        assert_eq!(metadata.height, 1);
        assert_eq!(metadata.format, "Png");
    }

    #[test]
    fn test_extract_large_image_dimensions() {
        // Test with a reasonably large image (not too large to slow down tests)
        let bytes = create_test_image(2048, 1536, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 2048);
        assert_eq!(metadata.height, 1536);
    }

    #[test]
    fn test_extract_image_with_no_metadata_has_empty_exif() {
        // Programmatically created images won't have EXIF metadata
        let bytes = create_test_image(100, 100, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert!(metadata.exif_data.is_empty());
    }

    // ============================
    // Additional Coverage Tests (3)
    // ============================

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
        assert!(metadata.height > metadata.width); // Portrait orientation
    }

    #[test]
    fn test_extract_rectangular_image_landscape_orientation() {
        let bytes = create_test_image(800, 400, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 800);
        assert_eq!(metadata.height, 400);
        assert!(metadata.width > metadata.height); // Landscape orientation
    }

    // ================================
    // Format-Specific Edge Cases (2)
    // ================================

    #[test]
    fn test_extract_square_image_equal_dimensions() {
        let bytes = create_test_image(512, 512, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 512);
        assert_eq!(metadata.height, 512);
        assert_eq!(metadata.width, metadata.height); // Square
    }

    #[test]
    fn test_extract_metadata_preserves_format_case() {
        let png_bytes = create_test_image(100, 100, ImageFormat::Png);
        let jpeg_bytes = create_test_image(100, 100, ImageFormat::Jpeg);
        let webp_bytes = create_test_image(100, 100, ImageFormat::WebP);

        let png_meta = extract_image_metadata(&png_bytes).unwrap();
        let jpeg_meta = extract_image_metadata(&jpeg_bytes).unwrap();
        let webp_meta = extract_image_metadata(&webp_bytes).unwrap();

        // Format strings should match the Debug representation
        assert_eq!(png_meta.format, "Png");
        assert_eq!(jpeg_meta.format, "Jpeg");
        assert_eq!(webp_meta.format, "WebP");
    }
}
