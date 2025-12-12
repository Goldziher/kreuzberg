//! Archive extractors for ZIP, TAR, and 7z formats.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::archive::{
    ArchiveMetadata as ExtractedMetadata, extract_7z_metadata, extract_7z_text_content, extract_tar_metadata,
    extract_tar_text_content, extract_zip_metadata, extract_zip_text_content,
};
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ArchiveMetadata, ExtractionResult, Metadata};
use async_trait::async_trait;
use std::collections::HashMap;

/// Build an ExtractionResult from archive metadata and text contents.
///
/// This helper function eliminates duplication across ZIP/TAR/7Z extractors by centralizing
/// the logic for transforming extracted metadata into the final result structure.
fn build_archive_result(
    extraction_metadata: ExtractedMetadata,
    text_contents: HashMap<String, String>,
    format_name: &str,
    mime_type: &str,
) -> ExtractionResult {
    let file_names: Vec<String> = extraction_metadata
        .file_list
        .iter()
        .map(|entry| entry.path.clone())
        .collect();

    let archive_metadata = ArchiveMetadata {
        format: format_name.to_string(),
        file_count: extraction_metadata.file_count,
        file_list: file_names,
        total_size: extraction_metadata.total_size as usize,
        compressed_size: None,
    };

    let mut additional = HashMap::new();
    let file_details: Vec<serde_json::Value> = extraction_metadata
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
    additional.insert("files".to_string(), serde_json::json!(file_details));

    let mut output = format!(
        "{} Archive ({} files, {} bytes)\n\n",
        format_name, extraction_metadata.file_count, extraction_metadata.total_size
    );
    output.push_str("Files:\n");
    for entry in &extraction_metadata.file_list {
        output.push_str(&format!("- {} ({} bytes)\n", entry.path, entry.size));
    }

    if !text_contents.is_empty() {
        output.push_str("\n\nText File Contents:\n\n");
        for (path, content) in text_contents {
            output.push_str(&format!("=== {} ===\n{}\n\n", path, content));
        }
    }

    ExtractionResult {
        content: output,
        mime_type: mime_type.to_string(),
        metadata: Metadata {
            format: Some(crate::types::FormatMetadata::Archive(archive_metadata)),
            additional,
            ..Default::default()
        },
        tables: vec![],
        detected_languages: None,
        chunks: None,
        images: None,
        pages: None,
    }
}

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

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
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
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, _config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let extraction_metadata = extract_zip_metadata(content)?;
        let text_contents = extract_zip_text_content(content)?;
        Ok(build_archive_result(
            extraction_metadata,
            text_contents,
            "ZIP",
            mime_type,
        ))
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

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
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
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, _config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let extraction_metadata = extract_tar_metadata(content)?;
        let text_contents = extract_tar_text_content(content)?;
        Ok(build_archive_result(
            extraction_metadata,
            text_contents,
            "TAR",
            mime_type,
        ))
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

/// 7z archive extractor.
///
/// Extracts file lists and text content from 7z archives.
pub struct SevenZExtractor;

impl SevenZExtractor {
    /// Create a new 7z extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for SevenZExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for SevenZExtractor {
    fn name(&self) -> &str {
        "7z-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Extracts file lists and text content from 7z archives"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[async_trait]
impl DocumentExtractor for SevenZExtractor {
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, _config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let extraction_metadata = extract_7z_metadata(content)?;
        let text_contents = extract_7z_text_content(content)?;
        Ok(build_archive_result(
            extraction_metadata,
            text_contents,
            "7Z",
            mime_type,
        ))
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-7z-compressed"]
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
        assert!(result.metadata.format.is_some());
        let archive_meta = match result.metadata.format.as_ref().unwrap() {
            crate::types::FormatMetadata::Archive(meta) => meta,
            _ => panic!("Expected Archive metadata"),
        };
        assert_eq!(archive_meta.format, "ZIP");
        assert_eq!(archive_meta.file_count, 1);
    }

    #[tokio::test]
    async fn test_tar_extractor() {
        let extractor = TarExtractor::new();

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
        assert!(result.metadata.format.is_some());
        let archive_meta = match result.metadata.format.as_ref().unwrap() {
            crate::types::FormatMetadata::Archive(meta) => meta,
            _ => panic!("Expected Archive metadata"),
        };
        assert_eq!(archive_meta.format, "TAR");
        assert_eq!(archive_meta.file_count, 1);
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
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert!(extractor.supported_mime_types().contains(&"application/zip"));
        assert_eq!(extractor.priority(), 50);
    }

    #[test]
    fn test_tar_plugin_interface() {
        let extractor = TarExtractor::new();
        assert_eq!(extractor.name(), "tar-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert!(extractor.supported_mime_types().contains(&"application/x-tar"));
        assert!(extractor.supported_mime_types().contains(&"application/tar"));
        assert_eq!(extractor.priority(), 50);
    }
}
