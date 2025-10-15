//! Image extraction functionality.
//!
//! This module provides functions for extracting metadata and EXIF data from images.

use crate::error::{KreuzbergError, Result};
use image::ImageReader;
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
}

/// Extract metadata from image bytes.
///
/// Extracts dimensions and format from the image.
pub fn extract_image_metadata(bytes: &[u8]) -> Result<ImageMetadata> {
    // Load image to get dimensions and format
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| KreuzbergError::Parsing(format!("Failed to read image format: {}", e)))?;

    let format = reader
        .format()
        .ok_or_else(|| KreuzbergError::Parsing("Could not determine image format".to_string()))?;

    let image = reader
        .decode()
        .map_err(|e| KreuzbergError::Parsing(format!("Failed to decode image: {}", e)))?;

    let width = image.width();
    let height = image.height();
    let format_str = format!("{:?}", format);

    Ok(ImageMetadata {
        width,
        height,
        format: format_str,
    })
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
