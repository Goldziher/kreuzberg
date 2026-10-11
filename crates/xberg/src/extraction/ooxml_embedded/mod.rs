//! Embedded object extraction from OOXML (DOCX/PPTX) archives.
//!
//! OOXML files are ZIP archives that may contain embedded objects in:
//! - DOCX: `word/embeddings/` directory
//! - PPTX: `ppt/embeddings/` directory
//!
//! This module extracts those embedded files, detects their MIME type,
//! and recursively processes them through the extraction pipeline.

use crate::core::config::ExtractionConfig;
use crate::types::{ArchiveEntry, ProcessingWarning};
use std::borrow::Cow;
use std::io::{Cursor, Read};

/// Build a `ProcessingWarning` tagged with this module's conventional `<source_label>_embedded_objects` source.
fn embedded_objects_warning(source_label: &str, message: String) -> ProcessingWarning {
    ProcessingWarning {
        source: Cow::Owned(format!("{}_embedded_objects", source_label)),
        message: Cow::Owned(message),
    }
}

/// Collect ZIP entry names under `embeddings_prefix` (entries whose name is strictly
/// longer than the prefix -- i.e. the directory entry itself is excluded).
fn collect_embedding_names(archive: &mut zip::ZipArchive<Cursor<&[u8]>>, embeddings_prefix: &str) -> Vec<String> {
    (0..archive.len())
        .filter_map(|i| {
            let file = archive.by_index(i).ok()?;
            let name = file.name().ok()?.into_owned();
            if name.starts_with(embeddings_prefix) && name.len() > embeddings_prefix.len() {
                Some(name)
            } else {
                None
            }
        })
        .collect()
}

/// Truncate `embedding_names` to `max_files_in_archive`, pushing a warning naming how many
/// entries were dropped when the cap was hit.
fn enforce_max_files_in_archive(
    embedding_names: &mut Vec<String>,
    max_files_in_archive: usize,
    embeddings_prefix: &str,
    source_label: &str,
    warnings: &mut Vec<ProcessingWarning>,
) {
    if embedding_names.len() <= max_files_in_archive {
        return;
    }
    let skipped = embedding_names.len() - max_files_in_archive;
    warnings.push(embedded_objects_warning(
        source_label,
        format!(
            "Skipped {} embedded object(s) under '{}': max_files_in_archive ({}) reached",
            skipped, embeddings_prefix, max_files_in_archive
        ),
    ));
    embedding_names.truncate(max_files_in_archive);
}

/// Upper bound for both the initial allocation hint and the actual read of a single
/// embedded file. `file.size()` (read at the call site) is the *declared* uncompressed size
/// from the ZIP central directory: it is attacker-controlled and is not verified against the
/// real decompressed byte count before we use it. A forged declaration (e.g. a
/// multi-terabyte value backed by a few bytes of real compressed data) must not
/// translate into an equally large `Vec::with_capacity` call, which allocates before a
/// single byte is read.
///
/// Prefers the caller's configured `max_embedded_file_bytes` (default 50 MiB, see
/// `ExtractionConfig::default_max_embedded_file_bytes`) since that is the limit this
/// module already enforces on the *actual* extracted size below -- one cap governs
/// both the hint and the acceptance check. If the caller has explicitly disabled the
/// per-file cap (`None`), fall back to the archive-wide `SecurityLimits::max_archive_size`
/// (default 500 MiB) as a hard backstop: no single embedded member should be allowed to
/// force a larger up-front allocation than the whole-archive budget the caller already
/// agreed to.
fn embedded_file_capacity_cap(
    config: &ExtractionConfig,
    security_limits: &crate::extractors::security::SecurityLimits,
) -> u64 {
    config
        .max_embedded_file_bytes
        .unwrap_or(security_limits.max_archive_size as u64)
}

/// Clamp an untrusted declared size to at most `cap` bytes.
///
/// `declared` is meant to be a size read straight from archive metadata the caller does not
/// control (e.g. a ZIP central-directory uncompressed-size field), so it must never be used
/// as-is to size an allocation: a forged multi-terabyte declaration would otherwise translate
/// directly into an equally large `Vec::with_capacity` request before a single byte is read.
/// Pulled out as its own function so the clamp itself -- not just its effect once wired into
/// the extraction loop -- has a direct, allocation-free unit test.
fn clamp_declared_size(declared: u64, cap: u64) -> u64 {
    declared.min(cap)
}

/// Extract embedded objects from an OOXML ZIP archive and recursively process them.
///
/// Scans the given `embeddings_prefix` directory (e.g. `word/embeddings/` or
/// `ppt/embeddings/`) inside the ZIP archive for embedded files. Known formats
/// (.xlsx, .pdf, .docx, .pptx, etc.) are recursively extracted. OLE compound
/// files (oleObject*.bin) are skipped with a warning unless their format can be
/// identified.
///
/// Returns `(children, warnings)` suitable for attaching to `InternalDocument`.
pub(crate) async fn extract_ooxml_embedded_objects(
    zip_bytes: &[u8],
    embeddings_prefix: &str,
    source_label: &str,
    config: &ExtractionConfig,
) -> (Vec<ArchiveEntry>, Vec<ProcessingWarning>) {
    let mut children = Vec::new();
    let mut warnings = Vec::new();

    let cursor = Cursor::new(zip_bytes);
    let mut archive = match zip::ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(_) => return (children, warnings),
    };

    let mut embedding_names = collect_embedding_names(&mut archive, embeddings_prefix);
    if embedding_names.is_empty() {
        return (children, warnings);
    }

    let security_limits = config.security_limits.clone().unwrap_or_default();
    enforce_max_files_in_archive(
        &mut embedding_names,
        security_limits.max_files_in_archive,
        embeddings_prefix,
        source_label,
        &mut warnings,
    );

    if config.max_archive_depth == 0 {
        warnings.push(embedded_objects_warning(
            source_label,
            format!(
                "Skipped {} embedded object(s) under '{}': max_archive_depth reached",
                embedding_names.len(),
                embeddings_prefix
            ),
        ));
        return (children, warnings);
    }

    let mut child_config = config.clone();
    child_config.max_archive_depth = config.max_archive_depth.saturating_sub(1);

    let embedded_capacity_cap = embedded_file_capacity_cap(config, &security_limits);

    for entry_name in &embedding_names {
        let (child, mut entry_warnings) = process_embedded_entry(
            &mut archive,
            entry_name,
            embeddings_prefix,
            source_label,
            embedded_capacity_cap,
            &child_config,
        )
        .await;
        warnings.append(&mut entry_warnings);
        if let Some(child) = child {
            children.push(child);
        }
    }

    (children, warnings)
}

/// Read, classify, and recursively extract a single embedded-object archive entry.
///
/// Returns the resulting `ArchiveEntry` (`None` when the entry was skipped or failed) plus
/// any warnings raised along the way, mirroring the original inline `for` loop body of
/// `extract_ooxml_embedded_objects` one entry at a time.
async fn process_embedded_entry(
    archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
    entry_name: &str,
    embeddings_prefix: &str,
    source_label: &str,
    embedded_capacity_cap: u64,
    child_config: &ExtractionConfig,
) -> (Option<ArchiveEntry>, Vec<ProcessingWarning>) {
    let mut warnings = Vec::new();
    let filename = entry_name
        .strip_prefix(embeddings_prefix)
        .unwrap_or(entry_name)
        .to_string();

    let Some(data) = read_embedded_entry_bytes(
        archive,
        entry_name,
        &filename,
        embedded_capacity_cap,
        source_label,
        &mut warnings,
    ) else {
        return (None, warnings);
    };

    let is_ole_binary = data.len() >= 4 && data[0..4] == [0xD0, 0xCF, 0x11, 0xE0];
    let child = if is_ole_binary {
        extract_ole_entry(&data, filename, child_config, source_label, &mut warnings).await
    } else {
        extract_regular_entry(&data, filename, child_config, source_label, &mut warnings).await
    };

    (child, warnings)
}

/// Read one archive entry's bytes, enforcing `embedded_capacity_cap` on both the
/// allocation hint and the actual read, and skipping (with a pushed warning where
/// applicable) an unreadable, empty, or oversized entry.
fn read_embedded_entry_bytes(
    archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
    entry_name: &str,
    filename: &str,
    embedded_capacity_cap: u64,
    source_label: &str,
    warnings: &mut Vec<ProcessingWarning>,
) -> Option<Vec<u8>> {
    let data = match archive.by_name(entry_name) {
        Ok(file) => {
            // `file.size()` is attacker-controlled declared metadata (see the comment
            // on `embedded_capacity_cap` above); clamp the allocation hint so a forged
            // value cannot force an immediate huge allocation. `Vec::with_capacity` is
            // only a hint -- it does not by itself bound how far `read_to_end` can grow
            // the buffer -- so the read itself is bounded via `.take()` below too.
            let capacity_hint = clamp_declared_size(file.size(), embedded_capacity_cap) as usize;
            let mut buf = Vec::with_capacity(capacity_hint);
            // Read at most one byte past the cap: this lets the size check below still
            // detect and report an oversized entry (it observes `cap + 1` bytes), while
            // guaranteeing `buf` itself can never grow past `embedded_capacity_cap + 1`
            // regardless of what the archive's central directory claims or what the
            // entry actually decompresses to.
            let read_cap = embedded_capacity_cap.saturating_add(1);
            if file.take(read_cap).read_to_end(&mut buf).is_err() {
                warnings.push(embedded_objects_warning(
                    source_label,
                    format!("Failed to read embedded file '{}'", filename),
                ));
                return None;
            }
            buf
        }
        Err(_) => return None,
    };

    if data.is_empty() {
        return None;
    }

    if data.len() as u64 > embedded_capacity_cap {
        warnings.push(embedded_objects_warning(
            source_label,
            format!(
                "Skipped embedded file '{}': size {} bytes exceeds cap {} bytes",
                filename,
                data.len(),
                embedded_capacity_cap
            ),
        ));
        return None;
    }

    Some(data)
}

/// Unwrap and recursively extract an OLE (CFB) compound-file embedded object.
async fn extract_ole_entry(
    data: &[u8],
    filename: String,
    child_config: &ExtractionConfig,
    source_label: &str,
    warnings: &mut Vec<ProcessingWarning>,
) -> Option<ArchiveEntry> {
    match extract_ole_embedded_object(data) {
        Some((inner_bytes, inner_mime)) => {
            match crate::core::extractor::extract_bytes(&inner_bytes, &inner_mime, child_config).await {
                Ok(result) => Some(ArchiveEntry {
                    path: filename,
                    mime_type: inner_mime,
                    result: Box::new(result),
                }),
                Err(e) => {
                    warnings.push(embedded_objects_warning(
                        source_label,
                        format!("Failed to extract embedded OLE object '{}': {}", filename, e),
                    ));
                    None
                }
            }
        }
        None => {
            warnings.push(embedded_objects_warning(
                source_label,
                format!(
                    "Skipped OLE compound file '{}': format identification not supported",
                    filename
                ),
            ));
            None
        }
    }
}

/// Detect a non-OLE embedded entry's MIME type and recursively extract it.
async fn extract_regular_entry(
    data: &[u8],
    filename: String,
    child_config: &ExtractionConfig,
    source_label: &str,
    warnings: &mut Vec<ProcessingWarning>,
) -> Option<ArchiveEntry> {
    let detected_mime = crate::core::mime::detect_mime_type_from_bytes(data).ok().or_else(|| {
        std::path::Path::new(&filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| mime_guess::from_ext(ext).first())
            .map(|m| m.to_string())
    });

    let file_mime = match detected_mime {
        Some(m) if m != "application/octet-stream" => m,
        _ => {
            warnings.push(embedded_objects_warning(
                source_label,
                format!(
                    "Skipped embedded file '{}': MIME type could not be determined",
                    filename
                ),
            ));
            return None;
        }
    };

    match crate::core::extractor::extract_bytes(data, &file_mime, child_config).await {
        Ok(result) => Some(ArchiveEntry {
            path: filename,
            mime_type: file_mime,
            result: Box::new(result),
        }),
        Err(e) => {
            warnings.push(embedded_objects_warning(
                source_label,
                format!("Failed to extract embedded '{}': {}", filename, e),
            ));
            None
        }
    }
}

/// Attempt to identify and unwrap an OLE (CFB) compound-file embedded object.
///
/// Two shapes are recognized:
/// - A "Package" stream: the OLE wrapper carries a modern Office document (e.g. an
///   embedded `.xlsx` chart source) verbatim as an OPC/ZIP package in a stream named
///   `Package`. The stream bytes are returned as-is with their detected MIME type.
/// - A legacy binary root stream (`WordDocument`, `PowerPoint Document`, `Workbook`, or
///   `Book`): the OLE container itself *is* the legacy `.doc`/`.ppt`/`.xls` document, so
///   the original bytes are handed back with the matching legacy MIME type for the
///   existing OLE-aware extractors to parse.
///
/// Returns `None` when the container can't be opened or none of the above streams are
/// present, so the caller can fall back to a "format identification not supported"
/// warning instead of silently dropping the object.
///
/// Only compiled when the `cfb` dependency is guaranteed active (via `office`, `hwp`, or
/// `email`); other feature combinations (e.g. `excel` alone, which also calls this
/// module) keep the pre-existing warn-and-skip behavior.
#[cfg(any(feature = "office", feature = "hwp", feature = "email"))]
pub(crate) fn extract_ole_embedded_object(data: &[u8]) -> Option<(Vec<u8>, String)> {
    let mut compound_file = cfb::CompoundFile::open(Cursor::new(data)).ok()?;

    if compound_file.exists("Package") {
        let mut stream = compound_file.open_stream("Package").ok()?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).ok()?;
        if buf.is_empty() {
            return None;
        }
        let mime = crate::core::mime::detect_mime_type_from_bytes(&buf).ok()?;
        return Some((buf, mime));
    }

    let legacy_mime = if compound_file.exists("WordDocument") {
        "application/msword"
    } else if compound_file.exists("PowerPoint Document") {
        "application/vnd.ms-powerpoint"
    } else if compound_file.exists("Workbook") || compound_file.exists("Book") {
        "application/vnd.ms-excel"
    } else {
        return None;
    };

    Some((data.to_vec(), legacy_mime.to_string()))
}

/// Fallback used when the `cfb` dependency isn't active for the enabled feature set
/// (e.g. `excel` without `office`/`hwp`/`email`): OLE objects are always reported as
/// unidentifiable rather than attempting extraction.
#[cfg(not(any(feature = "office", feature = "hwp", feature = "email")))]
pub(crate) fn extract_ole_embedded_object(_data: &[u8]) -> Option<(Vec<u8>, String)> {
    None
}

#[cfg(all(test, feature = "office"))]
mod tests;
