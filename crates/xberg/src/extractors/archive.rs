//! Archive extractors for ZIP, TAR, 7z, and GZIP formats.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::archive::{
    ArchiveMetadata as ExtractedMetadata, extract_7z_file_bytes, extract_7z_metadata, extract_7z_text_content,
    extract_gzip_with_bytes, extract_tar_file_bytes, extract_tar_metadata, extract_tar_text_content,
    extract_zip_file_bytes, extract_zip_metadata, extract_zip_text_content,
};
use crate::extractors::security::ZipBombValidator;
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::{ArchiveMetadata, Metadata, ProcessingWarning};
use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;
use std::io::Cursor;

/// `ProcessingWarning::source` used for every degradation reported by the archive extractors.
const ARCHIVE_WARNING_SOURCE: &str = "archive";

/// Build an `InternalDocument` from archive metadata and text contents.
///
/// Shared inner function — takes pre-computed children and warnings.
fn build_archive_doc_inner(
    extraction_metadata: ExtractedMetadata,
    text_contents: AHashMap<String, String>,
    format_name: &'static str,
    mime_type: &str,
    children: Vec<crate::types::ArchiveEntry>,
    processing_warnings: Vec<ProcessingWarning>,
) -> InternalDocument {
    let file_names: Vec<String> = extraction_metadata
        .file_list
        .iter()
        .map(|entry| entry.path.clone())
        .collect();

    let archive_metadata = ArchiveMetadata {
        format: Cow::Borrowed(format_name),
        file_count: extraction_metadata.file_count as u32,
        file_list: file_names,
        total_size: extraction_metadata.total_size,
        compressed_size: None,
    };

    let mut additional = AHashMap::new();
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
    additional.insert(Cow::Borrowed("files"), serde_json::json!(file_details));

    let metadata = Metadata {
        format: Some(crate::types::FormatMetadata::Archive(archive_metadata)),
        additional,
        ..Default::default()
    };

    let mut doc = InternalDocument::new(format_name.to_lowercase());
    doc.mime_type = mime_type.to_string();
    doc.metadata = metadata;

    let mut idx = 0u32;
    let summary = format!(
        "{} Archive ({} files, {} bytes)",
        format_name, extraction_metadata.file_count, extraction_metadata.total_size
    );
    doc.push_element(InternalElement::text(ElementKind::Paragraph, &summary, 0).with_index(idx));
    idx += 1;

    let mut file_list = String::from("Files:\n");
    for entry in &extraction_metadata.file_list {
        file_list.push_str(&format!("- {} ({} bytes)\n", entry.path, entry.size));
    }
    doc.push_element(InternalElement::text(ElementKind::Paragraph, &file_list, 0).with_index(idx));
    idx += 1;

    // `text_contents` is an `AHashMap`, and aHash randomizes iteration order per process, so
    // extracting the same archive twice produced the members in a different order each run.
    // Emit them in the archive's own order instead, which also makes the bodies agree with the
    // "Files:" listing printed just above. Members the listing does not cover sort after it by
    // path, so the ordering stays total even if the two disagree (#121).
    let listing_order: AHashMap<&str, usize> = extraction_metadata
        .file_list
        .iter()
        .enumerate()
        .map(|(position, entry)| (entry.path.as_str(), position))
        .collect();
    let mut members: Vec<(&String, &String)> = text_contents.iter().collect();
    members.sort_by(|(left, _), (right, _)| {
        let rank = |path: &String| listing_order.get(path.as_str()).copied().unwrap_or(usize::MAX);
        rank(left).cmp(&rank(right)).then_with(|| left.cmp(right))
    });

    for (path, content) in members {
        let text = format!("=== {} ===\n{}", path, content);
        doc.push_element(InternalElement::text(ElementKind::Paragraph, &text, 0).with_index(idx));
        idx += 1;
    }

    doc.children = if children.is_empty() { None } else { Some(children) };
    doc.processing_warnings = processing_warnings;

    doc
}

/// Returns true if `path` names an archive/tooling bookkeeping file (macOS `.DS_Store`,
/// `__MACOSX/` AppleDouble resource forks, `._`-prefixed AppleDouble sidecars, Python
/// `__pycache__/`/`.pyc`/`.pyo` bytecode, or Windows `Thumbs.db`/`desktop.ini`) rather than
/// a real document, so it can be filtered out of archive `children` before extraction.
fn is_archive_metadata_path(path: &str) -> bool {
    let components: Vec<&str> = path.split(['/', '\\']).collect();
    let basename = components.last().copied().unwrap_or(path);

    let has_bookkeeping_dir = components
        .iter()
        .any(|component| *component == "__MACOSX" || *component == "__pycache__");
    if has_bookkeeping_dir {
        return true;
    }

    if basename == ".DS_Store" || basename == "Thumbs.db" || basename == "desktop.ini" {
        return true;
    }

    if basename.starts_with("._") {
        return true;
    }

    let lower = basename.to_ascii_lowercase();
    lower.ends_with(".pyc") || lower.ends_with(".pyo")
}

/// What an archive reader pulled out of the container: entry index, decoded text and raw bytes.
struct ArchiveContents {
    extraction_metadata: ExtractedMetadata,
    text_contents: AHashMap<String, String>,
    file_bytes: AHashMap<String, Vec<u8>>,
}

/// Async version with recursive extraction of archive children.
///
/// When `config.max_archive_depth > current_depth`, extracts each file in `file_bytes`
/// by detecting its MIME type and dispatching to the appropriate extractor.
async fn build_archive_doc(
    contents: ArchiveContents,
    format_name: &'static str,
    mime_type: &str,
    config: &ExtractionConfig,
    current_depth: usize,
) -> InternalDocument {
    let ArchiveContents {
        extraction_metadata,
        text_contents,
        file_bytes,
    } = contents;
    let mut children = Vec::new();
    let mut processing_warnings = Vec::new();
    let mut filtered_paths: Vec<String> = Vec::new();

    // A non-directory entry that the archive index lists but whose bytes never made it
    // into `file_bytes` failed to decompress (bad CRC, truncated deflate stream, ...).
    // It is absent from the text contents *and* from `children`, so name it instead of
    // letting the document look complete (#114, #115).
    let unreadable_entries: Vec<String> = extraction_metadata
        .file_list
        .iter()
        .filter(|entry| !entry.is_dir && !file_bytes.contains_key(&entry.path))
        .map(|entry| entry.path.clone())
        .collect();
    if !unreadable_entries.is_empty() {
        let message = format!(
            "Skipped {} archive entr{} that could not be read: {}",
            unreadable_entries.len(),
            if unreadable_entries.len() == 1 { "y" } else { "ies" },
            crate::core::diagnostics::format_entry_list(&unreadable_entries)
        );
        crate::core::diagnostics::push_warning(&mut processing_warnings, ARCHIVE_WARNING_SOURCE, message);
    }

    if config.max_archive_depth > current_depth && !file_bytes.is_empty() {
        for (path, bytes) in &file_bytes {
            // A timed-out extraction cancels this token (see
            // `ExtractionConfig::ensure_cancel_token`); each entry launches its own
            // recursive extraction, so stop starting new ones once cancelled rather
            // than working through every remaining entry.
            if config
                .cancel_token
                .as_ref()
                .is_some_and(crate::cancellation::CancellationToken::is_cancelled)
            {
                let message = "Extraction cancelled; remaining archive entries were not processed".to_string();
                crate::core::diagnostics::push_warning(&mut processing_warnings, ARCHIVE_WARNING_SOURCE, message);
                break;
            }

            if is_archive_metadata_path(path) {
                filtered_paths.push(path.clone());
                continue;
            }

            let sniffed_mime = crate::core::mime::detect_mime_type_from_bytes(bytes).ok();

            // Sniffing sees markdown/CSV/YAML as plain UTF-8 and returns `text/plain`,
            // so fall back to the extension (as the top-level path does) to reach their
            // real extractors; a concrete sniff (PDF, DOCX, ...) still wins. Only default
            // to plain text when the extension itself maps to a textual type — an
            // unsniffable, extensionless (or unknown-extension) file is treated as
            // `application/octet-stream` so the skip below fires instead of misreporting
            // binary garbage as `text/plain`. ~keep
            let file_mime = match sniffed_mime {
                Some(m) if m != crate::core::mime::PLAIN_TEXT_MIME_TYPE => m,
                sniffed => crate::core::mime::detect_mime_type(path, false)
                    .ok()
                    .or(sniffed)
                    .unwrap_or_else(|| "application/octet-stream".to_string()),
            };

            if file_mime == "application/octet-stream" {
                filtered_paths.push(path.clone());
                continue;
            }

            let mut child_config = config.clone();
            child_config.max_archive_depth = config.max_archive_depth.saturating_sub(current_depth + 1);

            // Boxed: this is the recursive arm (build_archive_doc -> extract_bytes ->
            // build_archive_doc), so an unboxed child future is stored inline in this
            // one and every nesting level adds its whole size to the stack frame. A
            // single nested ZIP was already enough to overflow the stack in a debug
            // build. Heap-allocating the child keeps the per-level stack cost constant,
            // matching how every other recursive await in core::extractor is written.
            match Box::pin(crate::core::extractor::extract_bytes(bytes, &file_mime, &child_config)).await {
                Ok(result) => {
                    children.push(crate::types::ArchiveEntry {
                        path: path.clone(),
                        mime_type: file_mime,
                        result: Box::new(result),
                    });
                }
                Err(e) => {
                    processing_warnings.push(ProcessingWarning {
                        source: Cow::Borrowed("archive_recursive_extraction"),
                        message: Cow::Owned(format!("Failed to extract '{}': {}", path, e)),
                    });
                }
            }
        }
    }

    if !filtered_paths.is_empty() {
        // `file_bytes` is a hash map, so its iteration order is not stable; sort so the
        // warning text is deterministic for a given archive.
        filtered_paths.sort();
        let message = format!(
            "Filtered {} bookkeeping/binary entr{} (e.g. .DS_Store, __MACOSX, __pycache__, .pyc) \
             from archive children: {}",
            filtered_paths.len(),
            if filtered_paths.len() == 1 { "y" } else { "ies" },
            crate::core::diagnostics::format_entry_list(&filtered_paths)
        );
        crate::core::diagnostics::push_warning(&mut processing_warnings, ARCHIVE_WARNING_SOURCE, message);
    }

    build_archive_doc_inner(
        extraction_metadata,
        text_contents,
        format_name,
        mime_type,
        children,
        processing_warnings,
    )
}
#[cfg_attr(alef, alef(skip))]
/// ZIP archive extractor.
///
/// Extracts file lists and text content from ZIP archives.
pub struct ZipExtractor;

impl ZipExtractor {
    /// Create a new ZIP extractor.
    pub(crate) fn new() -> Self {
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
        "Xberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for ZipExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let limits = config.security_limits.clone().unwrap_or_default();

        let cursor = Cursor::new(content);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| crate::error::XbergError::parsing(format!("Failed to read ZIP archive: {}", e)))?;
        let validator = ZipBombValidator::new(limits.clone());
        validator
            .validate(&mut archive)
            .map_err(|e| crate::error::XbergError::validation(e.to_string()))?;

        let extraction_metadata = extract_zip_metadata(content, &limits)?;
        let text_contents = extract_zip_text_content(content, &limits)?;
        let file_bytes = extract_zip_file_bytes(content, &limits)?;
        Ok(build_archive_doc(
            ArchiveContents {
                extraction_metadata,
                text_contents,
                file_bytes,
            },
            "ZIP",
            mime_type,
            config,
            0,
        )
        .await)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/zip", "application/x-zip-compressed"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg_attr(alef, alef(skip))]
/// TAR archive extractor.
///
/// Extracts file lists and text content from TAR archives.
pub struct TarExtractor;

impl TarExtractor {
    /// Create a new TAR extractor.
    pub(crate) fn new() -> Self {
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
        "Xberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for TarExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let limits = config.security_limits.clone().unwrap_or_default();
        let extraction_metadata = extract_tar_metadata(content, &limits)?;
        let text_contents = extract_tar_text_content(content, &limits)?;
        let file_bytes = extract_tar_file_bytes(content, &limits)?;
        Ok(build_archive_doc(
            ArchiveContents {
                extraction_metadata,
                text_contents,
                file_bytes,
            },
            "TAR",
            mime_type,
            config,
            0,
        )
        .await)
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

#[cfg_attr(alef, alef(skip))]
/// 7z archive extractor.
///
/// Extracts file lists and text content from 7z archives.
pub struct SevenZExtractor;

impl SevenZExtractor {
    /// Create a new 7z extractor.
    pub(crate) fn new() -> Self {
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
        "Xberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for SevenZExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let limits = config.security_limits.clone().unwrap_or_default();
        let extraction_metadata = extract_7z_metadata(content, &limits)?;
        let text_contents = extract_7z_text_content(content, &limits)?;
        let file_bytes = extract_7z_file_bytes(content, &limits)?;
        Ok(build_archive_doc(
            ArchiveContents {
                extraction_metadata,
                text_contents,
                file_bytes,
            },
            "7Z",
            mime_type,
            config,
            0,
        )
        .await)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-7z-compressed"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg_attr(alef, alef(skip))]
/// Gzip archive extractor.
///
/// Decompresses gzip files and extracts text content from the compressed data.
pub struct GzipExtractor;

impl GzipExtractor {
    /// Create a new gzip extractor.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for GzipExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for GzipExtractor {
    fn name(&self) -> &str {
        "gzip-extractor"
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
        "Decompresses and extracts text content from gzip-compressed files"
    }

    fn author(&self) -> &str {
        "Xberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for GzipExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let limits = config.security_limits.clone().unwrap_or_default();
        let (extraction_metadata, text_contents, file_bytes) = extract_gzip_with_bytes(content, &limits)?;
        Ok(build_archive_doc(
            ArchiveContents {
                extraction_metadata,
                text_contents,
                file_bytes,
            },
            "GZIP",
            mime_type,
            config,
            0,
        )
        .await)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/gzip", "application/x-gzip"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests;
