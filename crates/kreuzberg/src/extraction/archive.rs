//! Archive extraction functionality.
//!
//! This module provides functions for extracting file lists and contents from archives.

use crate::error::{KreuzbergError, Result};
use sevenz_rust::SevenZReader;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use tar::Archive as TarArchive;
use zip::ZipArchive;

/// Archive metadata extracted from an archive file.
#[derive(Debug, Clone)]
pub struct ArchiveMetadata {
    /// Archive format (e.g., "ZIP", "TAR")
    pub format: String,
    /// List of files in the archive
    pub file_list: Vec<ArchiveEntry>,
    /// Total number of files
    pub file_count: usize,
    /// Total uncompressed size in bytes
    pub total_size: u64,
}

/// Information about a single file in an archive.
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    /// File path within the archive
    pub path: String,
    /// File size in bytes
    pub size: u64,
    /// Whether this is a directory
    pub is_dir: bool,
}

/// Extract metadata from a ZIP archive.
pub fn extract_zip_metadata(bytes: &[u8]) -> Result<ArchiveMetadata> {
    let cursor = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| KreuzbergError::Parsing(format!("Failed to read ZIP archive: {}", e)))?;

    let mut file_list = Vec::new();
    let mut total_size = 0u64;

    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| KreuzbergError::Parsing(format!("Failed to read ZIP entry: {}", e)))?;

        let path = file.name().to_string();
        let size = file.size();
        let is_dir = file.is_dir();

        if !is_dir {
            total_size += size;
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

/// Extract metadata from a TAR archive.
pub fn extract_tar_metadata(bytes: &[u8]) -> Result<ArchiveMetadata> {
    let cursor = Cursor::new(bytes);
    let mut archive = TarArchive::new(cursor);

    let mut file_list = Vec::new();
    let mut total_size = 0u64;
    let mut file_count = 0;

    let entries = archive
        .entries()
        .map_err(|e| KreuzbergError::Parsing(format!("Failed to read TAR archive: {}", e)))?;

    for entry_result in entries {
        let entry = entry_result.map_err(|e| KreuzbergError::Parsing(format!("Failed to read TAR entry: {}", e)))?;

        let path = entry
            .path()
            .map_err(|e| KreuzbergError::Parsing(format!("Failed to read TAR entry path: {}", e)))?
            .to_string_lossy()
            .to_string();

        let size = entry.size();
        let is_dir = entry.header().entry_type().is_dir();

        if !is_dir {
            total_size += size;
        }

        file_count += 1;
        file_list.push(ArchiveEntry { path, size, is_dir });
    }

    Ok(ArchiveMetadata {
        format: "TAR".to_string(),
        file_list,
        file_count,
        total_size,
    })
}

/// Extract text content from files within a ZIP archive.
///
/// Only extracts files with common text extensions: .txt, .md, .json, .xml, .html, .csv, .log
pub fn extract_zip_text_content(bytes: &[u8]) -> Result<HashMap<String, String>> {
    let cursor = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| KreuzbergError::Parsing(format!("Failed to read ZIP archive: {}", e)))?;

    let mut contents = HashMap::new();
    let text_extensions = [
        ".txt", ".md", ".json", ".xml", ".html", ".csv", ".log", ".yaml", ".yml", ".toml",
    ];

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| KreuzbergError::Parsing(format!("Failed to read ZIP entry: {}", e)))?;

        let path = file.name().to_string();

        // Only extract text files
        if !file.is_dir() && text_extensions.iter().any(|ext| path.to_lowercase().ends_with(ext)) {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_ok() {
                contents.insert(path, content);
            }
        }
    }

    Ok(contents)
}

/// Extract text content from files within a TAR archive.
///
/// Only extracts files with common text extensions: .txt, .md, .json, .xml, .html, .csv, .log
pub fn extract_tar_text_content(bytes: &[u8]) -> Result<HashMap<String, String>> {
    let cursor = Cursor::new(bytes);
    let mut archive = TarArchive::new(cursor);

    let mut contents = HashMap::new();
    let text_extensions = [
        ".txt", ".md", ".json", ".xml", ".html", ".csv", ".log", ".yaml", ".yml", ".toml",
    ];

    let entries = archive
        .entries()
        .map_err(|e| KreuzbergError::Parsing(format!("Failed to read TAR archive: {}", e)))?;

    for entry_result in entries {
        let mut entry =
            entry_result.map_err(|e| KreuzbergError::Parsing(format!("Failed to read TAR entry: {}", e)))?;

        let path = entry
            .path()
            .map_err(|e| KreuzbergError::Parsing(format!("Failed to read TAR entry path: {}", e)))?
            .to_string_lossy()
            .to_string();

        // Only extract text files
        if !entry.header().entry_type().is_dir() && text_extensions.iter().any(|ext| path.to_lowercase().ends_with(ext))
        {
            let mut content = String::new();
            if entry.read_to_string(&mut content).is_ok() {
                contents.insert(path, content);
            }
        }
    }

    Ok(contents)
}

/// Extract metadata from a 7z archive.
pub fn extract_7z_metadata(bytes: &[u8]) -> Result<ArchiveMetadata> {
    let cursor = Cursor::new(bytes);
    let archive = SevenZReader::new(cursor, bytes.len() as u64, "".into())
        .map_err(|e| KreuzbergError::Parsing(format!("Failed to read 7z archive: {}", e)))?;

    let mut file_list = Vec::new();
    let mut total_size = 0u64;

    for entry in &archive.archive().files {
        let path = entry.name().to_string();
        let size = entry.size();
        let is_dir = entry.is_directory();

        if !is_dir {
            total_size += size;
        }

        file_list.push(ArchiveEntry { path, size, is_dir });
    }

    let file_count = file_list.len();

    Ok(ArchiveMetadata {
        format: "7Z".to_string(),
        file_list,
        file_count,
        total_size,
    })
}

/// Extract text content from files within a 7z archive.
///
/// Only extracts files with common text extensions: .txt, .md, .json, .xml, .html, .csv, .log
pub fn extract_7z_text_content(bytes: &[u8]) -> Result<HashMap<String, String>> {
    let cursor = Cursor::new(bytes);
    let mut archive = SevenZReader::new(cursor, bytes.len() as u64, "".into())
        .map_err(|e| KreuzbergError::Parsing(format!("Failed to read 7z archive: {}", e)))?;

    let mut contents = HashMap::new();
    let text_extensions = [
        ".txt", ".md", ".json", ".xml", ".html", ".csv", ".log", ".yaml", ".yml", ".toml",
    ];

    archive
        .for_each_entries(|entry, reader| {
            let path = entry.name().to_string();

            // Only extract text files
            if !entry.is_directory() && text_extensions.iter().any(|ext| path.to_lowercase().ends_with(ext)) {
                let mut content = Vec::new();
                if let Ok(_) = reader.read_to_end(&mut content)
                    && let Ok(text) = String::from_utf8(content)
                {
                    contents.insert(path, text);
                }
            }
            Ok(true)
        })
        .map_err(|e| KreuzbergError::Parsing(format!("Failed to read 7z entries: {}", e)))?;

    Ok(contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tar::Builder as TarBuilder;
    use zip::write::{FileOptions, ZipWriter};

    #[test]
    fn test_extract_zip_metadata() {
        // Create a simple ZIP archive in memory
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let options = FileOptions::<'_, ()>::default();

            zip.start_file("test.txt", options).unwrap();
            zip.write_all(b"Hello, World!").unwrap();

            zip.start_file("dir/file.md", options).unwrap();
            zip.write_all(b"# Header").unwrap();

            zip.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let metadata = extract_zip_metadata(&bytes).unwrap();

        assert_eq!(metadata.format, "ZIP");
        assert_eq!(metadata.file_count, 2);
        assert_eq!(metadata.file_list.len(), 2);
        assert!(metadata.total_size > 0);
    }

    #[test]
    fn test_extract_tar_metadata() {
        // Create a simple TAR archive in memory
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut tar = TarBuilder::new(&mut cursor);

            let data1 = b"Hello, World!";
            let mut header1 = tar::Header::new_gnu();
            header1.set_path("test.txt").unwrap();
            header1.set_size(data1.len() as u64);
            header1.set_cksum();
            tar.append(&header1, &data1[..]).unwrap();

            let data2 = b"# Header";
            let mut header2 = tar::Header::new_gnu();
            header2.set_path("dir/file.md").unwrap();
            header2.set_size(data2.len() as u64);
            header2.set_cksum();
            tar.append(&header2, &data2[..]).unwrap();

            tar.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let metadata = extract_tar_metadata(&bytes).unwrap();

        assert_eq!(metadata.format, "TAR");
        assert_eq!(metadata.file_count, 2);
        assert_eq!(metadata.file_list.len(), 2);
        assert!(metadata.total_size > 0);
    }

    #[test]
    fn test_extract_zip_text_content() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let options = FileOptions::<'_, ()>::default();

            zip.start_file("test.txt", options).unwrap();
            zip.write_all(b"Hello, World!").unwrap();

            zip.start_file("readme.md", options).unwrap();
            zip.write_all(b"# README").unwrap();

            zip.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let contents = extract_zip_text_content(&bytes).unwrap();

        assert_eq!(contents.len(), 2);
        assert_eq!(contents.get("test.txt").unwrap(), "Hello, World!");
        assert_eq!(contents.get("readme.md").unwrap(), "# README");
    }

    #[test]
    fn test_extract_tar_text_content() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut tar = TarBuilder::new(&mut cursor);

            let data1 = b"Hello, World!";
            let mut header1 = tar::Header::new_gnu();
            header1.set_path("test.txt").unwrap();
            header1.set_size(data1.len() as u64);
            header1.set_cksum();
            tar.append(&header1, &data1[..]).unwrap();

            let data2 = b"# README";
            let mut header2 = tar::Header::new_gnu();
            header2.set_path("readme.md").unwrap();
            header2.set_size(data2.len() as u64);
            header2.set_cksum();
            tar.append(&header2, &data2[..]).unwrap();

            tar.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let contents = extract_tar_text_content(&bytes).unwrap();

        assert_eq!(contents.len(), 2);
        assert_eq!(contents.get("test.txt").unwrap(), "Hello, World!");
        assert_eq!(contents.get("readme.md").unwrap(), "# README");
    }

    #[test]
    fn test_extract_zip_metadata_invalid() {
        let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
        let result = extract_zip_metadata(&invalid_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_tar_metadata_invalid() {
        let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
        let result = extract_tar_metadata(&invalid_bytes);
        assert!(result.is_err());
    }
}
