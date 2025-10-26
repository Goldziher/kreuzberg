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

    #[test]
    fn test_extract_image_metadata_invalid() {
        let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
        let result = extract_image_metadata(&invalid_bytes);
        assert!(result.is_err());
    }
}
