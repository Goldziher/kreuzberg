//! Archive extractors for ZIP and TAR formats.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::archive::{
    extract_tar_metadata, extract_tar_text_content, extract_zip_metadata, extract_zip_text_content,
};
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::ExtractionResult;
use async_trait::async_trait;
use std::collections::HashMap;

/// ZIP archive extractor.
///
/// Extracts file lists and text content from ZIP archives.
pub struct ZipExtractor;

impl ZipExtractor {
    /// Create a new ZIP extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ZipExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ZipExtractor {
    fn name(&self) -> &str {
        "zip-extractor"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Extracts file lists and text content from ZIP archives"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[async_trait]
impl DocumentExtractor for ZipExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let metadata = extract_zip_metadata(content)?;
        let text_contents = extract_zip_text_content(content)?;

        let mut result_metadata = HashMap::new();
        result_metadata.insert("format".to_string(), serde_json::json!("ZIP"));
        result_metadata.insert("file_count".to_string(), serde_json::json!(metadata.file_count));
        result_metadata.insert("total_size".to_string(), serde_json::json!(metadata.total_size));

        // Add file list
        let file_list: Vec<serde_json::Value> = metadata
            .file_list
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "path": entry.path,
                    "size": entry.size,
                    "is_dir": entry.is_dir,
                })
            })
            .collect();
        result_metadata.insert("files".to_string(), serde_json::json!(file_list));

        // Build text output
        let mut output = format!(
            "ZIP Archive ({} files, {} bytes)\n\n",
            metadata.file_count, metadata.total_size
        );
        output.push_str("Files:\n");
        for entry in &metadata.file_list {
            output.push_str(&format!("- {} ({} bytes)\n", entry.path, entry.size));
        }

        if !text_contents.is_empty() {
            output.push_str("\n\nText File Contents:\n\n");
            for (path, content) in text_contents {
                output.push_str(&format!("=== {} ===\n{}\n\n", path, content));
            }
        }

        Ok(ExtractionResult {
            content: output,
            mime_type: mime_type.to_string(),
            metadata: result_metadata,
            tables: vec![],
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/zip", "application/x-zip-compressed"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

/// TAR archive extractor.
///
/// Extracts file lists and text content from TAR archives.
pub struct TarExtractor;

impl TarExtractor {
    /// Create a new TAR extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for TarExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for TarExtractor {
    fn name(&self) -> &str {
        "tar-extractor"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Extracts file lists and text content from TAR archives"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[async_trait]
impl DocumentExtractor for TarExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let metadata = extract_tar_metadata(content)?;
        let text_contents = extract_tar_text_content(content)?;

        let mut result_metadata = HashMap::new();
        result_metadata.insert("format".to_string(), serde_json::json!("TAR"));
        result_metadata.insert("file_count".to_string(), serde_json::json!(metadata.file_count));
        result_metadata.insert("total_size".to_string(), serde_json::json!(metadata.total_size));

        // Add file list
        let file_list: Vec<serde_json::Value> = metadata
            .file_list
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "path": entry.path,
                    "size": entry.size,
                    "is_dir": entry.is_dir,
                })
            })
            .collect();
        result_metadata.insert("files".to_string(), serde_json::json!(file_list));

        // Build text output
        let mut output = format!(
            "TAR Archive ({} files, {} bytes)\n\n",
            metadata.file_count, metadata.total_size
        );
        output.push_str("Files:\n");
        for entry in &metadata.file_list {
            output.push_str(&format!("- {} ({} bytes)\n", entry.path, entry.size));
        }

        if !text_contents.is_empty() {
            output.push_str("\n\nText File Contents:\n\n");
            for (path, content) in text_contents {
                output.push_str(&format!("=== {} ===\n{}\n\n", path, content));
            }
        }

        Ok(ExtractionResult {
            content: output,
            mime_type: mime_type.to_string(),
            metadata: result_metadata,
            tables: vec![],
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "application/x-tar",
            "application/tar",
            "application/x-gtar",
            "application/x-ustar",
        ]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use tar::Builder as TarBuilder;
    use zip::write::{FileOptions, ZipWriter};

    #[tokio::test]
    async fn test_zip_extractor() {
        let extractor = ZipExtractor::new();

        // Create a ZIP archive
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let options = FileOptions::<'_, ()>::default();

            zip.start_file("test.txt", options).unwrap();
            zip.write_all(b"Hello, World!").unwrap();

            zip.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(&bytes, "application/zip", &config)
            .await
            .unwrap();

        assert_eq!(result.mime_type, "application/zip");
        assert!(result.content.contains("ZIP Archive"));
        assert!(result.content.contains("test.txt"));
        assert!(result.content.contains("Hello, World!"));
        assert_eq!(result.metadata.get("format").unwrap(), &serde_json::json!("ZIP"));
        assert_eq!(result.metadata.get("file_count").unwrap(), &serde_json::json!(1));
    }

    #[tokio::test]
    async fn test_tar_extractor() {
        let extractor = TarExtractor::new();

        // Create a TAR archive
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut tar = TarBuilder::new(&mut cursor);

            let data = b"Hello, World!";
            let mut header = tar::Header::new_gnu();
            header.set_path("test.txt").unwrap();
            header.set_size(data.len() as u64);
            header.set_cksum();
            tar.append(&header, &data[..]).unwrap();

            tar.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(&bytes, "application/x-tar", &config)
            .await
            .unwrap();

        assert_eq!(result.mime_type, "application/x-tar");
        assert!(result.content.contains("TAR Archive"));
        assert!(result.content.contains("test.txt"));
        assert!(result.content.contains("Hello, World!"));
        assert_eq!(result.metadata.get("format").unwrap(), &serde_json::json!("TAR"));
        assert_eq!(result.metadata.get("file_count").unwrap(), &serde_json::json!(1));
    }

    #[tokio::test]
    async fn test_zip_extractor_invalid() {
        let extractor = ZipExtractor::new();
        let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(&invalid_bytes, "application/zip", &config)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tar_extractor_invalid() {
        let extractor = TarExtractor::new();
        let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(&invalid_bytes, "application/x-tar", &config)
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_zip_plugin_interface() {
        let extractor = ZipExtractor::new();
        assert_eq!(extractor.name(), "zip-extractor");
        assert_eq!(extractor.version(), "1.0.0");
        assert!(extractor.supported_mime_types().contains(&"application/zip"));
        assert_eq!(extractor.priority(), 50);
    }

    #[test]
    fn test_tar_plugin_interface() {
        let extractor = TarExtractor::new();
        assert_eq!(extractor.name(), "tar-extractor");
        assert_eq!(extractor.version(), "1.0.0");
        assert!(extractor.supported_mime_types().contains(&"application/x-tar"));
        assert!(extractor.supported_mime_types().contains(&"application/tar"));
        assert_eq!(extractor.priority(), 50);
    }
}
