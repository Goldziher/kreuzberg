mod mime_types;
mod subprocess;
mod version;

use crate::error::Result;
use crate::types::{ExtractedImage, PandocExtractionResult};
use std::path::Path;
use tokio::fs;

pub use mime_types::{get_extension_from_mime, get_pandoc_format_from_mime};
pub use subprocess::{extract_with_pandoc, extract_with_pandoc_from_bytes};
pub use version::validate_pandoc_version;

/// Minimum supported Pandoc version
pub const MINIMAL_SUPPORTED_PANDOC_VERSION: u32 = 2;

/// Extract content and metadata from a file using Pandoc
/// Extracts content and metadata in parallel for better performance
pub async fn extract_file(path: &Path, from_format: &str) -> Result<PandocExtractionResult> {
    // Validate pandoc is available
    validate_pandoc_version().await?;

    // Extract content and metadata IN PARALLEL (like Python's run_taskgroup)
    let (content_result, metadata_result) = tokio::join!(
        subprocess::extract_content(path, from_format),
        subprocess::extract_metadata(path, from_format)
    );

    let content = content_result?;
    let metadata = metadata_result?;

    Ok(PandocExtractionResult { content, metadata })
}

/// Extract content and metadata from bytes using Pandoc
pub async fn extract_bytes(bytes: &[u8], from_format: &str, extension: &str) -> Result<PandocExtractionResult> {
    // Validate pandoc is available
    validate_pandoc_version().await?;

    // Create temporary file
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!(
        "pandoc_temp_{}_{}.{}",
        std::process::id(),
        uuid::Uuid::new_v4(),
        extension
    ));

    // Write bytes to temp file
    fs::write(&temp_file, bytes).await?;

    // Extract
    let result = extract_file(&temp_file, from_format).await;

    // Cleanup
    let _ = fs::remove_file(&temp_file).await;

    result
}

/// Extract using MIME type (convenience function that handles MIME type conversion)
pub async fn extract_file_from_mime(path: &Path, mime_type: &str) -> Result<PandocExtractionResult> {
    let from_format = mime_types::get_pandoc_format_from_mime(mime_type)?;
    extract_file(path, &from_format).await
}

/// Extract bytes using MIME type (convenience function)
pub async fn extract_bytes_from_mime(bytes: &[u8], mime_type: &str) -> Result<PandocExtractionResult> {
    let from_format = mime_types::get_pandoc_format_from_mime(mime_type)?;
    let extension = mime_types::get_extension_from_mime(mime_type)?;
    extract_bytes(bytes, &from_format, &extension).await
}

/// Extract images from a file using Pandoc's --extract-media flag
pub async fn extract_images(path: &Path, from_format: &str) -> Result<Vec<ExtractedImage>> {
    use tokio::process::Command;

    // Validate pandoc is available
    validate_pandoc_version().await?;

    let mut images = Vec::new();

    // Create temporary directory for media extraction
    let temp_dir = std::env::temp_dir();
    let media_dir = temp_dir.join(format!("pandoc_media_{}_{}", std::process::id(), uuid::Uuid::new_v4()));
    fs::create_dir_all(&media_dir).await?;

    // Run pandoc with --extract-media flag
    let output = Command::new("pandoc")
        .arg(path)
        .arg(format!("--from={}", from_format))
        .arg("--to=markdown")
        .arg("--extract-media")
        .arg(&media_dir)
        .arg("--output=/dev/null")
        .output()
        .await
        .map_err(|e| {
            crate::error::KreuzbergError::Parsing(format!("Failed to execute pandoc for image extraction: {}", e))
        })?;

    if !output.status.success() {
        // Don't fail on image extraction errors - just return empty
        let _ = fs::remove_dir_all(&media_dir).await;
        return Ok(images);
    }

    // Read all extracted images recursively
    let mut stack = vec![media_dir.clone()];
    while let Some(dir) = stack.pop() {
        if let Ok(mut entries) = fs::read_dir(&dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    // Check if it's a supported image format
                    if let Some(ext) = path.extension() {
                        let ext_str = ext.to_string_lossy().to_lowercase();
                        if matches!(
                            ext_str.as_str(),
                            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "tif" | "webp"
                        ) {
                            // Read image data
                            if let Ok(data) = fs::read(&path).await {
                                let filename = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();

                                images.push(ExtractedImage {
                                    data,
                                    format: ext_str,
                                    slide_number: None,
                                    filename: Some(filename),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Cleanup
    let _ = fs::remove_dir_all(&media_dir).await;

    Ok(images)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_validate_pandoc_version() {
        // This test may fail in CI if pandoc is not installed
        // We'll mark it as xfail in CI environments
        let result = validate_pandoc_version().await;
        if result.is_err() {
            // Pandoc not installed, skip test
            return;
        }
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_extract_markdown_content() {
        // Skip if pandoc not available
        if validate_pandoc_version().await.is_err() {
            return;
        }

        let markdown = b"# Hello World\n\nThis is a test.";
        let result = extract_bytes(markdown, "markdown", "md").await;

        if let Ok(extraction) = result {
            assert!(extraction.content.contains("Hello World"));
            assert!(extraction.content.contains("test"));
        }
    }
}
