//! ZIP archive extraction.
//!
//! Provides functions for extracting metadata and text content from ZIP archives.

use super::{ArchiveEntry, ArchiveMetadata, TEXT_EXTENSIONS};
use crate::error::{Result, XbergError};
use crate::extractors::security::SecurityLimits;
use ahash::AHashMap;
use std::io::{Cursor, Read};
use zip::ZipArchive;

/// Extract metadata from a ZIP archive.
///
/// # Arguments
///
/// * `bytes` - The ZIP archive bytes
/// * `limits` - Security limits for archive extraction
///
/// # Returns
///
/// Returns `ArchiveMetadata` containing:
/// - Format: "ZIP"
/// - File list with paths, sizes, and directory flags
/// - Total file count
/// - Total uncompressed size
///
/// # Errors
///
/// Returns an error if the ZIP archive cannot be read or parsed,
/// or if security limits are exceeded.
pub(crate) fn extract_zip_metadata(bytes: &[u8], limits: &SecurityLimits) -> Result<ArchiveMetadata> {
    let cursor = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| XbergError::parsing(format!("Failed to read ZIP archive: {}", e)))?;

    if archive.len() > limits.max_files_in_archive {
        return Err(XbergError::validation(format!(
            "ZIP archive has too many files: {} (max: {})",
            archive.len(),
            limits.max_files_in_archive
        )));
    }

    let mut file_list = Vec::with_capacity(archive.len());
    let mut total_size = 0u64;

    for i in 0..archive.len() {
        // Listing an entry needs neither its password nor its compression method, so an
        // encrypted member is listed like any other. ~keep
        let file = archive
            .by_index_raw(i)
            .map_err(|e| XbergError::parsing(format!("Failed to read ZIP entry: {}", e)))?;

        let path = file
            .name()
            .map_err(|error| XbergError::parsing(format!("Invalid ZIP entry name: {error}")))?
            .into_owned();
        let size = file.size();
        let is_dir = file.is_dir();

        if !is_dir {
            total_size += size;
        }

        if total_size > limits.max_archive_size as u64 {
            return Err(XbergError::validation(format!(
                "ZIP archive total uncompressed size exceeds limit: {} bytes (max: {} bytes)",
                total_size, limits.max_archive_size
            )));
        }

        file_list.push(ArchiveEntry { path, size, is_dir });
    }

    Ok(ArchiveMetadata {
        format: "ZIP".to_string(),
        file_list,
        file_count: archive.len(),
        total_size,
    })
}

/// Extract text content from files within a ZIP archive.
///
/// Only extracts files with common text extensions: .txt, .md, .json, .xml, .html, .csv, .log, .yaml, .toml
///
/// # Arguments
///
/// * `bytes` - The ZIP archive bytes
///
/// # Returns
///
/// Returns a `HashMap` mapping file paths to their text content.
/// Binary files and files with non-text extensions are excluded.
///
/// # Errors
///
/// Returns an error if the ZIP archive cannot be read or parsed.
pub(crate) fn extract_zip_text_content(bytes: &[u8], limits: &SecurityLimits) -> Result<AHashMap<String, String>> {
    let cursor = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| XbergError::parsing(format!("Failed to read ZIP archive: {}", e)))?;

    if archive.len() > limits.max_files_in_archive {
        return Err(XbergError::validation(format!(
            "ZIP archive has too many files: {} (max: {})",
            archive.len(),
            limits.max_files_in_archive
        )));
    }

    let estimated_text_files = archive.len().saturating_mul(3).saturating_div(10).max(2);
    let mut contents = AHashMap::with_capacity(estimated_text_files);
    let mut total_content_size = 0usize;

    for i in 0..archive.len() {
        // A member that cannot be opened (encrypted, or compressed with a method this build
        // does not read) is skipped like one that fails to decompress below; the archive
        // extractor names it. ~keep
        let mut file = match archive.by_index(i) {
            Ok(file) => file,
            Err(error) => {
                tracing::warn!(index = i, error = %error, "skipping ZIP member: open failed");
                continue;
            }
        };

        let path = file
            .name()
            .map_err(|error| XbergError::parsing(format!("Invalid ZIP entry name: {error}")))?
            .into_owned();

        if !file.is_dir() && TEXT_EXTENSIONS.iter().any(|ext| path.to_lowercase().ends_with(ext)) {
            // `zip` 2.4.2 builds the entry's `Read` impl as `Crc32Reader::new(Decompressor::new(..))`
            // with no `Take` on the decompressed side (only the *compressed* side is bounded) --
            // the declared `size()` header field is not enforced by the reader at all, so a member
            // can expand arbitrarily regardless of what it claims. Bound the read ourselves at
            // `max_content_size`: that is the aggregate budget this member's decoded text is about
            // to be checked against a few lines down, so no single member can legitimately need
            // more than the whole document's text budget. `+1` lets us tell "exactly at the limit"
            // apart from "over the limit" instead of silently truncating a hostile member's content.
            let cap = limits.max_content_size as u64;
            let mut raw = Vec::new();
            let mut limited = (&mut file).take(cap.saturating_add(1));
            if let Err(e) = limited.read_to_end(&mut raw) {
                tracing::warn!(member = %path, error = %e, "skipping ZIP member: read failed");
                continue;
            }
            if raw.len() as u64 > cap {
                // Reject rather than silently truncate or skip: the pre-existing code already
                // surfaced a hard error once the *decoded* total crossed `max_content_size`
                // (below), so capping the read must not become a quieter failure mode than the
                // one it replaces.
                return Err(XbergError::validation(format!(
                    "ZIP archive member '{}' exceeds max_content_size while decompressing (limit: {} bytes)",
                    path, limits.max_content_size
                )));
            }
            let content = super::decode_archive_text(&raw, &path);
            total_content_size = total_content_size.saturating_add(content.len());
            if total_content_size > limits.max_content_size {
                return Err(XbergError::validation(format!(
                    "ZIP archive text content exceeds limit: {} bytes (max: {} bytes)",
                    total_content_size, limits.max_content_size
                )));
            }
            contents.insert(path, content);
        }
    }

    Ok(contents)
}

/// Extract raw file bytes for all non-directory entries in a ZIP archive.
///
/// Returns a `HashMap` mapping file paths to their raw byte content.
/// Respects security limits for file count and total archive size.
///
/// # Arguments
///
/// * `bytes` - The ZIP archive bytes
/// * `limits` - Security limits for archive extraction
///
/// # Errors
///
/// Returns an error if the ZIP archive cannot be read or if security limits are exceeded.
pub(crate) fn extract_zip_file_bytes(bytes: &[u8], limits: &SecurityLimits) -> Result<AHashMap<String, Vec<u8>>> {
    let cursor = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| XbergError::parsing(format!("Failed to read ZIP archive: {}", e)))?;

    if archive.len() > limits.max_files_in_archive {
        return Err(XbergError::validation(format!(
            "ZIP archive has too many files: {} (max: {})",
            archive.len(),
            limits.max_files_in_archive
        )));
    }

    let mut file_bytes = AHashMap::with_capacity(archive.len());
    let mut total_size = 0usize;

    for i in 0..archive.len() {
        // Skipped like an unreadable member; see `extract_zip_text_content`. ~keep
        let mut file = match archive.by_index(i) {
            Ok(file) => file,
            Err(error) => {
                tracing::warn!(index = i, error = %error, "skipping ZIP member: open failed");
                continue;
            }
        };

        if file.is_dir() {
            continue;
        }

        let path = file
            .name()
            .map_err(|error| XbergError::parsing(format!("Invalid ZIP entry name: {error}")))?
            .into_owned();
        // See the comment in `extract_zip_text_content`: the underlying `zip` reader has no
        // `Take` on the decompressed side, so `size()` (the declared header value) bounds
        // nothing about this read. Cap it ourselves at `max_archive_size` -- the same budget
        // `total_size` is checked against below, so no single member can legitimately need
        // more than the whole archive's budget. The `+1` distinguishes "exactly at the limit"
        // from "over" instead of silently truncating a hostile member's bytes.
        let cap = limits.max_archive_size as u64;
        let mut content = Vec::new();
        let mut limited = (&mut file).take(cap.saturating_add(1));
        if limited.read_to_end(&mut content).is_ok() {
            if content.len() as u64 > cap {
                // Reject rather than silently truncate: the pre-existing code already surfaced
                // a hard error once the running `total_size` crossed `max_archive_size` (below),
                // so capping the read must not become a quieter failure mode than that.
                return Err(XbergError::validation(format!(
                    "ZIP archive member '{}' exceeds max_archive_size while decompressing (limit: {} bytes)",
                    path, limits.max_archive_size
                )));
            }
            total_size = total_size.saturating_add(content.len());
            if total_size > limits.max_archive_size {
                return Err(XbergError::validation(format!(
                    "ZIP archive total extracted size exceeds limit: {} bytes (max: {} bytes)",
                    total_size, limits.max_archive_size
                )));
            }
            file_bytes.insert(path, content);
        }
    }

    Ok(file_bytes)
}
