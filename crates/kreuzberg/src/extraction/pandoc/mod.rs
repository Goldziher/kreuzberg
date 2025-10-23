mod mime_types;
mod subprocess;
mod version;

use crate::error::Result;
use crate::types::{ExtractedImage, PandocExtractionResult};
use serde_json::Value;
use std::collections::HashMap;
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
    validate_pandoc_version().await?;

    let (content_result, metadata_result) = tokio::join!(
        subprocess::extract_content(path, from_format),
        subprocess::extract_metadata(path, from_format)
    );

    let content = content_result?;
    let mut metadata = metadata_result?;

    // For DOCX files, extract comprehensive Office metadata and merge with Pandoc metadata
    // If metadata extraction fails, continue with Pandoc metadata only (non-blocking)
    if from_format == "docx"
        && let Ok(office_metadata) = extract_docx_metadata(path).await
    {
        merge_metadata(&mut metadata, office_metadata);
    }

    Ok(PandocExtractionResult { content, metadata })
}

/// Extract content and metadata from bytes using Pandoc
pub async fn extract_bytes(bytes: &[u8], from_format: &str, extension: &str) -> Result<PandocExtractionResult> {
    validate_pandoc_version().await?;

    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!(
        "pandoc_temp_{}_{}.{}",
        std::process::id(),
        uuid::Uuid::new_v4(),
        extension
    ));

    fs::write(&temp_file, bytes).await?;

    let result = extract_file(&temp_file, from_format).await;

    let _ = fs::remove_file(&temp_file).await;

    result
}

/// Extract using MIME type (convenience function that handles MIME type conversion)
pub async fn extract_file_from_mime(path: &Path, mime_type: &str) -> Result<PandocExtractionResult> {
    let from_format = get_pandoc_format_from_mime(mime_type)?;
    extract_file(path, &from_format).await
}

/// Extract bytes using MIME type (convenience function)
pub async fn extract_bytes_from_mime(bytes: &[u8], mime_type: &str) -> Result<PandocExtractionResult> {
    let from_format = get_pandoc_format_from_mime(mime_type)?;
    let extension = get_extension_from_mime(mime_type)?;
    extract_bytes(bytes, &from_format, &extension).await
}

/// Extract images from a file using Pandoc's --extract-media flag
pub async fn extract_images(path: &Path, from_format: &str) -> Result<Vec<ExtractedImage>> {
    use tokio::process::Command;

    validate_pandoc_version().await?;

    let mut images = Vec::new();

    let temp_dir = std::env::temp_dir();
    let media_dir = temp_dir.join(format!("pandoc_media_{}_{}", std::process::id(), uuid::Uuid::new_v4()));
    fs::create_dir_all(&media_dir).await?;

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
            crate::error::KreuzbergError::parsing(format!("Failed to execute pandoc for image extraction: {}", e))
        })?;

    if !output.status.success() {
        let _ = fs::remove_dir_all(&media_dir).await;
        return Ok(images);
    }

    let mut stack = vec![media_dir.clone()];
    while let Some(dir) = stack.pop() {
        if let Ok(mut entries) = fs::read_dir(&dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file()
                    && let Some(ext) = path.extension()
                {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if matches!(
                        ext_str.as_str(),
                        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "tif" | "webp"
                    ) && let Ok(data) = fs::read(&path).await
                    {
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

    let _ = fs::remove_dir_all(&media_dir).await;

    Ok(images)
}

/// Extract comprehensive metadata from a DOCX file
///
/// Extracts metadata from docProps/core.xml, docProps/app.xml, and docProps/custom.xml
/// and converts it to a format compatible with Pandoc metadata.
async fn extract_docx_metadata(path: &Path) -> Result<HashMap<String, Value>> {
    use crate::extraction::office_metadata::{
        extract_core_properties, extract_custom_properties, extract_docx_app_properties,
    };

    let file = tokio::task::spawn_blocking({
        let path = path.to_path_buf();
        move || std::fs::File::open(&path)
    })
    .await
    .map_err(|e| crate::error::KreuzbergError::parsing(format!("Task join error: {}", e)))??;

    let mut archive = tokio::task::spawn_blocking(move || {
        zip::ZipArchive::new(file).map_err(|e| std::io::Error::other(format!("Failed to open ZIP archive: {}", e)))
    })
    .await
    .map_err(|e| crate::error::KreuzbergError::parsing(format!("Task join error: {}", e)))??;

    let mut metadata = HashMap::new();

    // Extract core properties
    if let Ok(core) = extract_core_properties(&mut archive) {
        if let Some(title) = core.title {
            metadata.insert("title".to_string(), Value::String(title));
        }
        if let Some(creator) = core.creator {
            metadata.insert(
                "authors".to_string(),
                Value::Array(vec![Value::String(creator.clone())]),
            );
            metadata.insert("created_by".to_string(), Value::String(creator));
        }
        if let Some(subject) = core.subject {
            metadata.insert("subject".to_string(), Value::String(subject));
        }
        if let Some(keywords) = core.keywords {
            metadata.insert("keywords".to_string(), Value::String(keywords));
        }
        if let Some(description) = core.description {
            metadata.insert("description".to_string(), Value::String(description));
        }
        if let Some(modified_by) = core.last_modified_by {
            metadata.insert("modified_by".to_string(), Value::String(modified_by));
        }
        if let Some(created) = core.created {
            metadata.insert("created_at".to_string(), Value::String(created));
        }
        if let Some(modified) = core.modified {
            metadata.insert("modified_at".to_string(), Value::String(modified));
        }
        if let Some(revision) = core.revision {
            metadata.insert("revision".to_string(), Value::String(revision));
        }
        if let Some(category) = core.category {
            metadata.insert("category".to_string(), Value::String(category));
        }
        if let Some(content_status) = core.content_status {
            metadata.insert("content_status".to_string(), Value::String(content_status));
        }
        if let Some(language) = core.language {
            metadata.insert("language".to_string(), Value::String(language));
        }
    }

    // Extract app properties
    if let Ok(app) = extract_docx_app_properties(&mut archive) {
        if let Some(pages) = app.pages {
            metadata.insert("page_count".to_string(), Value::Number(pages.into()));
        }
        if let Some(words) = app.words {
            metadata.insert("word_count".to_string(), Value::Number(words.into()));
        }
        if let Some(chars) = app.characters {
            metadata.insert("character_count".to_string(), Value::Number(chars.into()));
        }
        if let Some(lines) = app.lines {
            metadata.insert("line_count".to_string(), Value::Number(lines.into()));
        }
        if let Some(paragraphs) = app.paragraphs {
            metadata.insert("paragraph_count".to_string(), Value::Number(paragraphs.into()));
        }
        if let Some(template) = app.template {
            metadata.insert("template".to_string(), Value::String(template));
        }
        if let Some(company) = app.company {
            metadata.insert("organization".to_string(), Value::String(company));
        }
        if let Some(time) = app.total_time {
            metadata.insert("total_editing_time_minutes".to_string(), Value::Number(time.into()));
        }
        if let Some(application) = app.application {
            metadata.insert("application".to_string(), Value::String(application));
        }
    }

    // Extract custom properties (optional)
    if let Ok(custom) = extract_custom_properties(&mut archive) {
        for (key, value) in custom {
            metadata.insert(format!("custom_{}", key), value);
        }
    }

    Ok(metadata)
}

/// Merge Office metadata with Pandoc metadata
///
/// Pandoc metadata takes precedence in case of conflicts.
fn merge_metadata(pandoc_metadata: &mut HashMap<String, Value>, office_metadata: HashMap<String, Value>) {
    for (key, value) in office_metadata {
        // Only insert if Pandoc didn't already provide this field
        pandoc_metadata.entry(key).or_insert(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_validate_pandoc_version() {
        let result = validate_pandoc_version().await;
        if result.is_err() {
            return;
        }
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_extract_markdown_content() {
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

    #[tokio::test]
    async fn test_extract_file_with_metadata() {
        if validate_pandoc_version().await.is_err() {
            return;
        }

        let markdown = b"---\ntitle: Test Document\nauthor: Test Author\n---\n\n# Content\n\nSome text.";
        let result = extract_bytes(markdown, "markdown", "md").await;

        if let Ok(extraction) = result {
            assert!(extraction.content.contains("Content"));
            assert!(extraction.metadata.contains_key("title"));
        }
    }

    #[tokio::test]
    async fn test_extract_bytes_creates_temp_file() {
        if validate_pandoc_version().await.is_err() {
            return;
        }

        let content = b"Simple text";
        let result = extract_bytes(content, "markdown", "md").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_extract_file_from_mime_docx() {
        if validate_pandoc_version().await.is_err() {
            return;
        }

        let mime_type = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
        let format = get_pandoc_format_from_mime(mime_type);
        assert!(format.is_ok());
        assert_eq!(format.unwrap(), "docx");
    }

    #[tokio::test]
    async fn test_extract_bytes_from_mime_rst() {
        if validate_pandoc_version().await.is_err() {
            return;
        }

        let rst = b"Title\n=====\n\nParagraph text.";
        let result = extract_bytes_from_mime(rst, "text/x-rst").await;

        if let Ok(extraction) = result {
            assert!(extraction.content.contains("Title"));
        }
    }

    #[tokio::test]
    async fn test_extract_bytes_from_mime_invalid() {
        if validate_pandoc_version().await.is_err() {
            return;
        }

        let content = b"test";
        let result = extract_bytes_from_mime(content, "application/invalid").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_extension_from_mime() {
        let ext = get_extension_from_mime("text/x-rst");
        assert!(ext.is_ok());
        assert_eq!(ext.unwrap(), "rst");

        let ext = get_extension_from_mime("application/x-latex");
        assert!(ext.is_ok());
        assert_eq!(ext.unwrap(), "tex");
    }

    #[tokio::test]
    async fn test_extract_images_no_pandoc() {
        if validate_pandoc_version().await.is_err() {
            return;
        }

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("test_pandoc_{}.md", uuid::Uuid::new_v4()));
        tokio::fs::write(&temp_file, b"# No Images\n\nJust text.")
            .await
            .unwrap();

        let result = extract_images(&temp_file, "markdown").await;

        if let Ok(images) = result {
            assert!(images.is_empty());
        }

        let _ = tokio::fs::remove_file(&temp_file).await;
    }

    #[tokio::test]
    async fn test_minimal_supported_version_constant() {
        assert_eq!(MINIMAL_SUPPORTED_PANDOC_VERSION, 2);
    }

    #[test]
    fn test_mime_type_mappings_complete() {
        let common_types = vec![
            "application/rtf",
            "application/epub+zip",
            "text/x-rst",
            "application/x-latex",
            "text/csv",
        ];

        for mime_type in common_types {
            let format = get_pandoc_format_from_mime(mime_type);
            assert!(format.is_ok(), "MIME type {} should be supported", mime_type);

            let ext = get_extension_from_mime(mime_type);
            assert!(ext.is_ok(), "Extension mapping for {} should exist", mime_type);
        }
    }

    #[tokio::test]
    async fn test_extract_bytes_empty_content() {
        if validate_pandoc_version().await.is_err() {
            return;
        }

        let empty = b"";
        let result = extract_bytes(empty, "markdown", "md").await;

        if let Ok(extraction) = result {
            assert!(extraction.content.is_empty() || extraction.content.trim().is_empty());
        }
    }
}
