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
        ZipArchive::new(cursor).map_err(|e| KreuzbergError::parsing(format!("Failed to read ZIP archive: {}", e)))?;

    let mut file_list = Vec::new();
    let mut total_size = 0u64;

    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| KreuzbergError::parsing(format!("Failed to read ZIP entry: {}", e)))?;

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
        .map_err(|e| KreuzbergError::parsing(format!("Failed to read TAR archive: {}", e)))?;

    for entry_result in entries {
        let entry = entry_result.map_err(|e| KreuzbergError::parsing(format!("Failed to read TAR entry: {}", e)))?;

        let path = entry
            .path()
            .map_err(|e| KreuzbergError::parsing(format!("Failed to read TAR entry path: {}", e)))?
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
        ZipArchive::new(cursor).map_err(|e| KreuzbergError::parsing(format!("Failed to read ZIP archive: {}", e)))?;

    let mut contents = HashMap::new();
    let text_extensions = [
        ".txt", ".md", ".json", ".xml", ".html", ".csv", ".log", ".yaml", ".yml", ".toml",
    ];

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| KreuzbergError::parsing(format!("Failed to read ZIP entry: {}", e)))?;

        let path = file.name().to_string();

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
        .map_err(|e| KreuzbergError::parsing(format!("Failed to read TAR archive: {}", e)))?;

    for entry_result in entries {
        let mut entry =
            entry_result.map_err(|e| KreuzbergError::parsing(format!("Failed to read TAR entry: {}", e)))?;

        let path = entry
            .path()
            .map_err(|e| KreuzbergError::parsing(format!("Failed to read TAR entry path: {}", e)))?
            .to_string_lossy()
            .to_string();

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
        .map_err(|e| KreuzbergError::parsing(format!("Failed to read 7z archive: {}", e)))?;

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
        .map_err(|e| KreuzbergError::parsing(format!("Failed to read 7z archive: {}", e)))?;

    let mut contents = HashMap::new();
    let text_extensions = [
        ".txt", ".md", ".json", ".xml", ".html", ".csv", ".log", ".yaml", ".yml", ".toml",
    ];

    archive
        .for_each_entries(|entry, reader| {
            let path = entry.name().to_string();

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
        .map_err(|e| KreuzbergError::parsing(format!("Failed to read 7z entries: {}", e)))?;

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

    // Archive Format Tests

    #[test]
    fn test_extract_zip_metadata_with_directories() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let options = FileOptions::<'_, ()>::default();

            zip.add_directory("dir1/", options).unwrap();
            zip.add_directory("dir1/subdir/", options).unwrap();

            zip.start_file("dir1/file1.txt", options).unwrap();
            zip.write_all(b"content1").unwrap();

            zip.start_file("dir1/subdir/file2.txt", options).unwrap();
            zip.write_all(b"content2").unwrap();

            zip.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let metadata = extract_zip_metadata(&bytes).unwrap();

        assert_eq!(metadata.format, "ZIP");
        assert_eq!(metadata.file_count, 4);
        assert_eq!(metadata.total_size, 16);

        let dir_count = metadata.file_list.iter().filter(|e| e.is_dir).count();
        assert_eq!(dir_count, 2);
    }

    #[test]
    fn test_extract_tar_metadata_with_directories() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut tar = TarBuilder::new(&mut cursor);

            let mut header_dir = tar::Header::new_gnu();
            header_dir.set_path("dir1/").unwrap();
            header_dir.set_size(0);
            header_dir.set_entry_type(tar::EntryType::Directory);
            header_dir.set_cksum();
            tar.append(&header_dir, &[][..]).unwrap();

            let data = b"content1";
            let mut header1 = tar::Header::new_gnu();
            header1.set_path("dir1/file1.txt").unwrap();
            header1.set_size(data.len() as u64);
            header1.set_cksum();
            tar.append(&header1, &data[..]).unwrap();

            tar.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let metadata = extract_tar_metadata(&bytes).unwrap();

        assert_eq!(metadata.format, "TAR");
        assert_eq!(metadata.file_count, 2);

        let dir_count = metadata.file_list.iter().filter(|e| e.is_dir).count();
        assert_eq!(dir_count, 1);
    }

    #[test]
    fn test_extract_tar_gz_metadata() {
        let mut tar_data = Vec::new();
        {
            let mut tar = TarBuilder::new(&mut tar_data);

            let data = b"Hello from gzip!";
            let mut header = tar::Header::new_gnu();
            header.set_path("test.txt").unwrap();
            header.set_size(data.len() as u64);
            header.set_cksum();
            tar.append(&header, &data[..]).unwrap();

            tar.finish().unwrap();
        }

        // Test that we can extract metadata from the uncompressed TAR
        // (gzip decompression would happen before archive extraction in real usage)
        let metadata = extract_tar_metadata(&tar_data).unwrap();
        assert_eq!(metadata.format, "TAR");
        assert_eq!(metadata.file_count, 1);
        assert_eq!(metadata.file_list[0].path, "test.txt");
    }

    #[test]
    fn test_extract_7z_metadata_with_files() {
        use sevenz_rust::SevenZWriter;

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut sz = SevenZWriter::new(&mut cursor).unwrap();

            sz.push_archive_entry(
                sevenz_rust::SevenZArchiveEntry::from_path("test.txt", "test.txt".to_string()),
                Some(Cursor::new(b"Hello 7z!".to_vec())),
            )
            .unwrap();

            sz.push_archive_entry(
                sevenz_rust::SevenZArchiveEntry::from_path("data.json", "data.json".to_string()),
                Some(Cursor::new(b"{\"key\":\"value\"}".to_vec())),
            )
            .unwrap();

            sz.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let metadata = extract_7z_metadata(&bytes).unwrap();

        assert_eq!(metadata.format, "7Z");
        assert_eq!(metadata.file_count, 2);
        assert!(metadata.total_size > 0);
    }

    // Nested Archive Tests

    #[test]
    fn test_extract_zip_within_zip() {
        let mut inner_cursor = Cursor::new(Vec::new());
        {
            let mut inner_zip = ZipWriter::new(&mut inner_cursor);
            let options = FileOptions::<'_, ()>::default();

            inner_zip.start_file("inner.txt", options).unwrap();
            inner_zip.write_all(b"Nested content").unwrap();

            inner_zip.finish().unwrap();
        }
        let inner_bytes = inner_cursor.into_inner();

        let mut outer_cursor = Cursor::new(Vec::new());
        {
            let mut outer_zip = ZipWriter::new(&mut outer_cursor);
            let options = FileOptions::<'_, ()>::default();

            outer_zip.start_file("archive.zip", options).unwrap();
            outer_zip.write_all(&inner_bytes).unwrap();

            outer_zip.start_file("readme.txt", options).unwrap();
            outer_zip.write_all(b"Outer content").unwrap();

            outer_zip.finish().unwrap();
        }

        let outer_bytes = outer_cursor.into_inner();
        let metadata = extract_zip_metadata(&outer_bytes).unwrap();

        assert_eq!(metadata.file_count, 2);

        let archive_entry = metadata.file_list.iter().find(|e| e.path == "archive.zip");
        assert!(archive_entry.is_some());
        assert!(archive_entry.unwrap().size > 0);
    }

    #[test]
    fn test_extract_tar_within_tar() {
        let mut inner_cursor = Cursor::new(Vec::new());
        {
            let mut inner_tar = TarBuilder::new(&mut inner_cursor);

            let data = b"Nested content";
            let mut header = tar::Header::new_gnu();
            header.set_path("inner.txt").unwrap();
            header.set_size(data.len() as u64);
            header.set_cksum();
            inner_tar.append(&header, &data[..]).unwrap();

            inner_tar.finish().unwrap();
        }
        let inner_bytes = inner_cursor.into_inner();

        let mut outer_cursor = Cursor::new(Vec::new());
        {
            let mut outer_tar = TarBuilder::new(&mut outer_cursor);

            let mut header1 = tar::Header::new_gnu();
            header1.set_path("archive.tar").unwrap();
            header1.set_size(inner_bytes.len() as u64);
            header1.set_cksum();
            outer_tar.append(&header1, &inner_bytes[..]).unwrap();

            let data = b"Outer content";
            let mut header2 = tar::Header::new_gnu();
            header2.set_path("readme.txt").unwrap();
            header2.set_size(data.len() as u64);
            header2.set_cksum();
            outer_tar.append(&header2, &data[..]).unwrap();

            outer_tar.finish().unwrap();
        }

        let outer_bytes = outer_cursor.into_inner();
        let metadata = extract_tar_metadata(&outer_bytes).unwrap();

        assert_eq!(metadata.file_count, 2);

        let archive_entry = metadata.file_list.iter().find(|e| e.path == "archive.tar");
        assert!(archive_entry.is_some());
    }

    // Error Handling Tests

    #[test]
    fn test_extract_zip_corrupted_data() {
        let mut valid_cursor = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut valid_cursor);
            let options = FileOptions::<'_, ()>::default();

            zip.start_file("test.txt", options).unwrap();
            zip.write_all(b"content").unwrap();

            zip.finish().unwrap();
        }

        let mut corrupted = valid_cursor.into_inner();
        corrupted.truncate(corrupted.len() / 2);

        let result = extract_zip_metadata(&corrupted);
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, KreuzbergError::Parsing { .. }));
        }
    }

    #[test]
    fn test_extract_tar_corrupted_data() {
        let mut valid_cursor = Cursor::new(Vec::new());
        {
            let mut tar = TarBuilder::new(&mut valid_cursor);

            let data = b"content";
            let mut header = tar::Header::new_gnu();
            header.set_path("test.txt").unwrap();
            header.set_size(data.len() as u64);
            header.set_cksum();
            tar.append(&header, &data[..]).unwrap();

            tar.finish().unwrap();
        }

        let mut corrupted = valid_cursor.into_inner();
        corrupted[100] = 0xFF;

        let result = extract_tar_metadata(&corrupted);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_zip_empty_archive() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let zip = ZipWriter::new(&mut cursor);
            zip.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let metadata = extract_zip_metadata(&bytes).unwrap();

        assert_eq!(metadata.format, "ZIP");
        assert_eq!(metadata.file_count, 0);
        assert_eq!(metadata.total_size, 0);
        assert_eq!(metadata.file_list.len(), 0);
    }

    #[test]
    fn test_extract_tar_empty_archive() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut tar = TarBuilder::new(&mut cursor);
            tar.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let metadata = extract_tar_metadata(&bytes).unwrap();

        assert_eq!(metadata.format, "TAR");
        assert_eq!(metadata.file_count, 0);
        assert_eq!(metadata.total_size, 0);
        assert_eq!(metadata.file_list.len(), 0);
    }

    // Content Extraction Tests

    #[test]
    fn test_extract_zip_multiple_text_files() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let options = FileOptions::<'_, ()>::default();

            zip.start_file("file1.txt", options).unwrap();
            zip.write_all(b"Content 1").unwrap();

            zip.start_file("file2.md", options).unwrap();
            zip.write_all(b"# Markdown").unwrap();

            zip.start_file("data.json", options).unwrap();
            zip.write_all(b"{\"key\":\"value\"}").unwrap();

            zip.start_file("binary.bin", options).unwrap();
            zip.write_all(&[0xFF, 0xFE, 0xFD]).unwrap();

            zip.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let contents = extract_zip_text_content(&bytes).unwrap();

        assert_eq!(contents.len(), 3);
        assert_eq!(contents.get("file1.txt").unwrap(), "Content 1");
        assert_eq!(contents.get("file2.md").unwrap(), "# Markdown");
        assert_eq!(contents.get("data.json").unwrap(), "{\"key\":\"value\"}");
        assert!(!contents.contains_key("binary.bin"));
    }

    #[test]
    fn test_extract_tar_multiple_text_files() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut tar = TarBuilder::new(&mut cursor);

            let files = vec![
                ("file1.txt", b"Content 1" as &[u8]),
                ("file2.md", b"# Markdown"),
                ("data.xml", b"<root>data</root>"),
                ("config.yaml", b"key: value"),
            ];

            for (path, data) in files {
                let mut header = tar::Header::new_gnu();
                header.set_path(path).unwrap();
                header.set_size(data.len() as u64);
                header.set_cksum();
                tar.append(&header, data).unwrap();
            }

            tar.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let contents = extract_tar_text_content(&bytes).unwrap();

        assert_eq!(contents.len(), 4);
        assert_eq!(contents.get("file1.txt").unwrap(), "Content 1");
        assert_eq!(contents.get("file2.md").unwrap(), "# Markdown");
        assert_eq!(contents.get("data.xml").unwrap(), "<root>data</root>");
        assert_eq!(contents.get("config.yaml").unwrap(), "key: value");
    }

    #[test]
    fn test_extract_zip_preserves_directory_structure() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let options = FileOptions::<'_, ()>::default();

            zip.add_directory("root/", options).unwrap();
            zip.add_directory("root/sub1/", options).unwrap();
            zip.add_directory("root/sub2/", options).unwrap();

            zip.start_file("root/file1.txt", options).unwrap();
            zip.write_all(b"Root file").unwrap();

            zip.start_file("root/sub1/file2.txt", options).unwrap();
            zip.write_all(b"Sub1 file").unwrap();

            zip.start_file("root/sub2/file3.txt", options).unwrap();
            zip.write_all(b"Sub2 file").unwrap();

            zip.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let metadata = extract_zip_metadata(&bytes).unwrap();

        let paths: Vec<&str> = metadata.file_list.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"root/"));
        assert!(paths.contains(&"root/sub1/"));
        assert!(paths.contains(&"root/sub2/"));
        assert!(paths.contains(&"root/file1.txt"));
        assert!(paths.contains(&"root/sub1/file2.txt"));
        assert!(paths.contains(&"root/sub2/file3.txt"));
    }

    // Edge Cases Tests

    #[test]
    fn test_extract_zip_with_large_file() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let options = FileOptions::<'_, ()>::default();

            let large_content = "x".repeat(10_000);

            zip.start_file("large.txt", options).unwrap();
            zip.write_all(large_content.as_bytes()).unwrap();

            zip.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let metadata = extract_zip_metadata(&bytes).unwrap();

        assert_eq!(metadata.file_count, 1);
        assert_eq!(metadata.total_size, 10_000);

        let contents = extract_zip_text_content(&bytes).unwrap();
        assert_eq!(contents.get("large.txt").unwrap().len(), 10_000);
    }

    #[test]
    fn test_extract_zip_with_many_files() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let options = FileOptions::<'_, ()>::default();

            for i in 0..100 {
                let filename = format!("file_{}.txt", i);
                let content = format!("Content {}", i);

                zip.start_file(&filename, options).unwrap();
                zip.write_all(content.as_bytes()).unwrap();
            }

            zip.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let metadata = extract_zip_metadata(&bytes).unwrap();

        assert_eq!(metadata.file_count, 100);
        assert_eq!(metadata.file_list.len(), 100);

        let contents = extract_zip_text_content(&bytes).unwrap();
        assert_eq!(contents.len(), 100);
    }

    #[test]
    fn test_extract_zip_with_long_paths() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let options = FileOptions::<'_, ()>::default();

            let long_path = format!("{}/file.txt", "a".repeat(200));

            zip.start_file(&long_path, options).unwrap();
            zip.write_all(b"Deep file").unwrap();

            zip.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let metadata = extract_zip_metadata(&bytes).unwrap();

        assert_eq!(metadata.file_count, 1);
        assert!(metadata.file_list[0].path.len() > 200);

        let contents = extract_zip_text_content(&bytes).unwrap();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_extract_7z_text_content() {
        use sevenz_rust::SevenZWriter;

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut sz = SevenZWriter::new(&mut cursor).unwrap();

            sz.push_archive_entry(
                sevenz_rust::SevenZArchiveEntry::from_path("test.txt", "test.txt".to_string()),
                Some(Cursor::new(b"Hello 7z text!".to_vec())),
            )
            .unwrap();

            sz.push_archive_entry(
                sevenz_rust::SevenZArchiveEntry::from_path("readme.md", "readme.md".to_string()),
                Some(Cursor::new(b"# 7z README".to_vec())),
            )
            .unwrap();

            sz.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let contents = extract_7z_text_content(&bytes).unwrap();

        assert_eq!(contents.len(), 2);
        assert_eq!(contents.get("test.txt").unwrap(), "Hello 7z text!");
        assert_eq!(contents.get("readme.md").unwrap(), "# 7z README");
    }

    #[test]
    fn test_extract_7z_empty_archive() {
        use sevenz_rust::SevenZWriter;

        let mut cursor = Cursor::new(Vec::new());
        {
            let sz = SevenZWriter::new(&mut cursor).unwrap();
            sz.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let metadata = extract_7z_metadata(&bytes).unwrap();

        assert_eq!(metadata.format, "7Z");
        assert_eq!(metadata.file_count, 0);
        assert_eq!(metadata.total_size, 0);
    }

    #[test]
    fn test_extract_tar_with_large_file() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut tar = TarBuilder::new(&mut cursor);

            let large_content = "y".repeat(50_000);

            let mut header = tar::Header::new_gnu();
            header.set_path("large.txt").unwrap();
            header.set_size(large_content.len() as u64);
            header.set_cksum();
            tar.append(&header, large_content.as_bytes()).unwrap();

            tar.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let metadata = extract_tar_metadata(&bytes).unwrap();

        assert_eq!(metadata.file_count, 1);
        assert_eq!(metadata.total_size, 50_000);

        let contents = extract_tar_text_content(&bytes).unwrap();
        assert_eq!(contents.get("large.txt").unwrap().len(), 50_000);
    }

    #[test]
    fn test_extract_zip_text_content_filters_non_text_extensions() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let options = FileOptions::<'_, ()>::default();

            zip.start_file("document.txt", options).unwrap();
            zip.write_all(b"Text file").unwrap();

            zip.start_file("image.png", options).unwrap();
            zip.write_all(&[0x89, 0x50, 0x4E, 0x47]).unwrap();

            zip.start_file("binary.exe", options).unwrap();
            zip.write_all(&[0x4D, 0x5A]).unwrap();

            zip.start_file("config.toml", options).unwrap();
            zip.write_all(b"[section]").unwrap();

            zip.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let contents = extract_zip_text_content(&bytes).unwrap();

        assert_eq!(contents.len(), 2);
        assert!(contents.contains_key("document.txt"));
        assert!(contents.contains_key("config.toml"));
        assert!(!contents.contains_key("image.png"));
        assert!(!contents.contains_key("binary.exe"));
    }

    #[test]
    fn test_extract_7z_corrupted_data() {
        let invalid_7z_data = vec![0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C, 0x00];

        let result = extract_7z_metadata(&invalid_7z_data);
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, KreuzbergError::Parsing { .. }));
        }
    }
}
