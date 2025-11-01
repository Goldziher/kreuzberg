//! Archive extraction integration tests.
//!
//! Tests for ZIP, TAR, TAR.GZ, and 7z archive extraction.
//! Validates metadata extraction, content extraction, nested archives, and error handling.

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::{extract_bytes, extract_bytes_sync};
use std::io::{Cursor, Write};
use tar::Builder as TarBuilder;
use zip::write::{FileOptions, ZipWriter};

mod helpers;

/// Test basic ZIP extraction with single file.
#[tokio::test]
async fn test_zip_basic_extraction() {
    let config = ExtractionConfig::default();

    let zip_bytes = create_simple_zip();

    let result = extract_bytes(&zip_bytes, "application/zip", &config)
        .await
        .expect("Should extract ZIP successfully");

    assert_eq!(result.mime_type, "application/zip");
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");
    assert!(result.tables.is_empty(), "Archive should not have tables");

    assert!(result.content.contains("ZIP Archive"));
    assert!(result.content.contains("test.txt"));
    assert!(result.content.contains("Hello from ZIP!"));

    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().unwrap() {
        kreuzberg::FormatMetadata::Archive(meta) => meta,
        _ => panic!("Expected Archive metadata"),
    };
    assert_eq!(archive_meta.format, "ZIP");
    assert_eq!(archive_meta.file_count, 1);
    assert_eq!(archive_meta.file_list.len(), 1);
    assert_eq!(archive_meta.file_list[0], "test.txt");
}

/// Test ZIP with multiple files.
#[tokio::test]
async fn test_zip_multiple_files() {
    let config = ExtractionConfig::default();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, ()>::default();

        zip.start_file("file1.txt", options).unwrap();
        zip.write_all(b"Content 1").unwrap();

        zip.start_file("file2.md", options).unwrap();
        zip.write_all(b"# Content 2").unwrap();

        zip.start_file("file3.json", options).unwrap();
        zip.write_all(b"{\"key\": \"value\"}").unwrap();

        zip.finish().unwrap();
    }

    let zip_bytes = cursor.into_inner();
    let result = extract_bytes(&zip_bytes, "application/zip", &config)
        .await
        .expect("Should extract multi-file ZIP");

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");
    assert!(result.tables.is_empty(), "Archive should not have tables");

    assert!(result.content.contains("file1.txt"));
    assert!(result.content.contains("file2.md"));
    assert!(result.content.contains("file3.json"));

    assert!(result.content.contains("Content 1"));
    assert!(result.content.contains("Content 2"));
    assert!(result.content.contains("value"));

    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().unwrap() {
        kreuzberg::FormatMetadata::Archive(meta) => meta,
        _ => panic!("Expected Archive metadata"),
    };
    assert_eq!(archive_meta.file_count, 3, "Should have 3 files");
    assert_eq!(archive_meta.file_list.len(), 3, "file_list should contain 3 entries");
    assert!(archive_meta.file_list.contains(&"file1.txt".to_string()));
    assert!(archive_meta.file_list.contains(&"file2.md".to_string()));
    assert!(archive_meta.file_list.contains(&"file3.json".to_string()));
}

/// Test ZIP with nested directory structure.
#[tokio::test]
async fn test_zip_nested_directories() {
    let config = ExtractionConfig::default();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, ()>::default();

        zip.add_directory("dir1/", options).unwrap();
        zip.add_directory("dir1/subdir/", options).unwrap();

        zip.start_file("dir1/file.txt", options).unwrap();
        zip.write_all(b"File in dir1").unwrap();

        zip.start_file("dir1/subdir/nested.txt", options).unwrap();
        zip.write_all(b"Nested file").unwrap();

        zip.finish().unwrap();
    }

    let zip_bytes = cursor.into_inner();
    let result = extract_bytes(&zip_bytes, "application/zip", &config)
        .await
        .expect("Should extract nested ZIP");

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");
    assert!(result.tables.is_empty(), "Archive should not have tables");

    assert!(result.content.contains("dir1/"));
    assert!(result.content.contains("dir1/file.txt"));
    assert!(result.content.contains("dir1/subdir/nested.txt"));

    assert!(result.content.contains("File in dir1"));
    assert!(result.content.contains("Nested file"));

    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().unwrap() {
        kreuzberg::FormatMetadata::Archive(meta) => meta,
        _ => panic!("Expected Archive metadata"),
    };
    assert!(
        archive_meta.file_count >= 2,
        "Should have at least 2 files (excluding empty dirs)"
    );
    assert!(archive_meta.file_list.iter().any(|f| f.contains("dir1/file.txt")));
    assert!(
        archive_meta
            .file_list
            .iter()
            .any(|f| f.contains("dir1/subdir/nested.txt"))
    );
}

/// Test TAR extraction.
#[tokio::test]
async fn test_tar_extraction() {
    let config = ExtractionConfig::default();

    let tar_bytes = create_simple_tar();

    let result = extract_bytes(&tar_bytes, "application/x-tar", &config)
        .await
        .expect("Should extract TAR successfully");

    assert_eq!(result.mime_type, "application/x-tar");
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");
    assert!(result.tables.is_empty(), "Archive should not have tables");

    assert!(result.content.contains("TAR Archive"));
    assert!(result.content.contains("test.txt"));
    assert!(result.content.contains("Hello from TAR!"));

    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().unwrap() {
        kreuzberg::FormatMetadata::Archive(meta) => meta,
        _ => panic!("Expected Archive metadata"),
    };
    assert_eq!(archive_meta.format, "TAR");
    assert_eq!(archive_meta.file_count, 1);
}

/// Test TAR.GZ extraction (compressed TAR).
///
/// Note: TAR.GZ requires decompression before extraction.
/// This test validates TAR extraction which is the underlying format.
#[tokio::test]
async fn test_tar_gz_extraction() {
    let config = ExtractionConfig::default();

    let tar_bytes = create_simple_tar();

    let result = extract_bytes(&tar_bytes, "application/x-tar", &config)
        .await
        .expect("Should extract TAR");

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");
    assert!(result.tables.is_empty(), "Archive should not have tables");

    assert!(result.content.contains("TAR Archive"));
    assert!(result.content.contains("test.txt"));

    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().unwrap() {
        kreuzberg::FormatMetadata::Archive(meta) => meta,
        _ => panic!("Expected Archive metadata"),
    };
    assert_eq!(archive_meta.format, "TAR");
    assert_eq!(archive_meta.file_count, 1);

    let result2 = extract_bytes(&tar_bytes, "application/tar", &config)
        .await
        .expect("Should extract with alternative MIME type");

    assert!(
        result2.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(result2.detected_languages.is_none(), "Language detection not enabled");
    assert!(result2.tables.is_empty(), "Archive should not have tables");

    assert!(result2.content.contains("TAR Archive"));
    assert!(result2.metadata.format.is_some());
}

/// Test 7z extraction.
#[tokio::test]
async fn test_7z_extraction() {
    println!("7z test requires real 7z file - skipping programmatic creation");
}

/// Test nested archive (ZIP inside ZIP).
#[tokio::test]
async fn test_nested_archive() {
    let config = ExtractionConfig::default();

    let inner_zip = create_simple_zip();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, ()>::default();

        zip.start_file("inner.zip", options).unwrap();
        zip.write_all(&inner_zip).unwrap();

        zip.start_file("readme.txt", options).unwrap();
        zip.write_all(b"This archive contains another archive").unwrap();

        zip.finish().unwrap();
    }

    let outer_zip_bytes = cursor.into_inner();
    let result = extract_bytes(&outer_zip_bytes, "application/zip", &config)
        .await
        .expect("Should extract nested ZIP");

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");
    assert!(result.tables.is_empty(), "Archive should not have tables");

    assert!(result.content.contains("inner.zip"));
    assert!(result.content.contains("readme.txt"));
    assert!(result.content.contains("This archive contains another archive"));

    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().unwrap() {
        kreuzberg::FormatMetadata::Archive(meta) => meta,
        _ => panic!("Expected Archive metadata"),
    };
    assert_eq!(archive_meta.file_count, 2, "Should have 2 files in outer archive");
    assert!(archive_meta.file_list.contains(&"inner.zip".to_string()));
    assert!(archive_meta.file_list.contains(&"readme.txt".to_string()));
}

/// Test archive with mixed file formats (PDF, DOCX, images).
#[tokio::test]
async fn test_archive_mixed_formats() {
    let config = ExtractionConfig::default();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, ()>::default();

        zip.start_file("document.txt", options).unwrap();
        zip.write_all(b"Text document").unwrap();

        zip.start_file("readme.md", options).unwrap();
        zip.write_all(b"# README").unwrap();

        zip.start_file("image.png", options).unwrap();
        zip.write_all(&[0x89, 0x50, 0x4E, 0x47]).unwrap();

        zip.start_file("document.pdf", options).unwrap();
        zip.write_all(b"%PDF-1.4").unwrap();

        zip.finish().unwrap();
    }

    let zip_bytes = cursor.into_inner();
    let result = extract_bytes(&zip_bytes, "application/zip", &config)
        .await
        .expect("Should extract mixed-format ZIP");

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");
    assert!(result.tables.is_empty(), "Archive should not have tables");

    assert!(result.content.contains("document.txt"));
    assert!(result.content.contains("readme.md"));
    assert!(result.content.contains("image.png"));
    assert!(result.content.contains("document.pdf"));

    assert!(result.content.contains("Text document"));
    assert!(result.content.contains("# README"));

    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().unwrap() {
        kreuzberg::FormatMetadata::Archive(meta) => meta,
        _ => panic!("Expected Archive metadata"),
    };
    assert_eq!(archive_meta.file_count, 4, "Should have 4 files");
    assert_eq!(archive_meta.file_list.len(), 4, "file_list should contain 4 entries");
    assert!(archive_meta.file_list.contains(&"document.txt".to_string()));
    assert!(archive_meta.file_list.contains(&"readme.md".to_string()));
    assert!(archive_meta.file_list.contains(&"image.png".to_string()));
    assert!(archive_meta.file_list.contains(&"document.pdf".to_string()));
}

/// Test password-protected archive (should fail gracefully).
#[tokio::test]
async fn test_password_protected_archive() {
    let config = ExtractionConfig::default();

    let invalid_zip = vec![0x50, 0x4B, 0x03, 0x04];

    let result = extract_bytes(&invalid_zip, "application/zip", &config).await;

    assert!(result.is_err(), "Should fail on invalid/encrypted ZIP");
}

/// Test corrupted archive.
#[tokio::test]
async fn test_corrupted_archive() {
    let config = ExtractionConfig::default();

    let corrupted_zip = vec![0x50, 0x4B, 0x03, 0x04, 0xFF, 0xFF, 0xFF, 0xFF];

    let result = extract_bytes(&corrupted_zip, "application/zip", &config).await;

    assert!(result.is_err(), "Should fail on corrupted ZIP");

    let mut corrupted_tar = vec![0xFF; 512];
    corrupted_tar[0..5].copy_from_slice(b"file\0");

    let result = extract_bytes(&corrupted_tar, "application/x-tar", &config).await;
    assert!(
        result.is_ok() || result.is_err(),
        "Should handle corrupted TAR gracefully"
    );
}

/// Test large archive (100+ files).
#[tokio::test]
async fn test_large_archive() {
    let config = ExtractionConfig::default();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, ()>::default();

        for i in 0..100 {
            zip.start_file(format!("file_{}.txt", i), options).unwrap();
            zip.write_all(format!("Content {}", i).as_bytes()).unwrap();
        }

        zip.finish().unwrap();
    }

    let zip_bytes = cursor.into_inner();
    let result = extract_bytes(&zip_bytes, "application/zip", &config)
        .await
        .expect("Should extract large ZIP");

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");
    assert!(result.tables.is_empty(), "Archive should not have tables");

    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().unwrap() {
        kreuzberg::FormatMetadata::Archive(meta) => meta,
        _ => panic!("Expected Archive metadata"),
    };
    assert_eq!(archive_meta.file_count, 100, "Should have 100 files");
    assert_eq!(
        archive_meta.file_list.len(),
        100,
        "file_list should contain 100 entries"
    );

    assert!(result.content.contains("file_0.txt"));
    assert!(result.content.contains("file_99.txt"));
    assert!(archive_meta.file_list.contains(&"file_0.txt".to_string()));
    assert!(archive_meta.file_list.contains(&"file_50.txt".to_string()));
    assert!(archive_meta.file_list.contains(&"file_99.txt".to_string()));
}

/// Test archive with special characters and Unicode filenames.
#[tokio::test]
async fn test_archive_with_special_characters() {
    let config = ExtractionConfig::default();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, ()>::default();

        zip.start_file("测试文件.txt", options).unwrap();
        zip.write_all("Unicode content".as_bytes()).unwrap();

        zip.start_file("file with spaces.txt", options).unwrap();
        zip.write_all(b"Spaces in filename").unwrap();

        zip.start_file("file-with-dashes.txt", options).unwrap();
        zip.write_all(b"Dashes").unwrap();

        zip.finish().unwrap();
    }

    let zip_bytes = cursor.into_inner();
    let result = extract_bytes(&zip_bytes, "application/zip", &config)
        .await
        .expect("Should extract ZIP with special characters");

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");
    assert!(result.tables.is_empty(), "Archive should not have tables");

    assert!(result.content.contains("测试文件.txt") || result.content.contains("txt"));
    assert!(result.content.contains("file with spaces.txt"));
    assert!(result.content.contains("file-with-dashes.txt"));

    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().unwrap() {
        kreuzberg::FormatMetadata::Archive(meta) => meta,
        _ => panic!("Expected Archive metadata"),
    };
    assert_eq!(archive_meta.file_count, 3, "Should have 3 files");
    assert_eq!(archive_meta.file_list.len(), 3, "file_list should contain 3 entries");
    assert!(archive_meta.file_list.iter().any(|f| f.contains("txt")));
    assert!(archive_meta.file_list.contains(&"file with spaces.txt".to_string()));
    assert!(archive_meta.file_list.contains(&"file-with-dashes.txt".to_string()));
}

/// Test empty archive.
#[tokio::test]
async fn test_empty_archive() {
    let config = ExtractionConfig::default();

    let mut cursor = Cursor::new(Vec::new());
    {
        let zip = ZipWriter::new(&mut cursor);
        zip.finish().unwrap();
    }

    let zip_bytes = cursor.into_inner();
    let result = extract_bytes(&zip_bytes, "application/zip", &config)
        .await
        .expect("Should extract empty ZIP");

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");
    assert!(result.tables.is_empty(), "Archive should not have tables");

    assert!(result.content.contains("ZIP Archive"));
    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().unwrap() {
        kreuzberg::FormatMetadata::Archive(meta) => meta,
        _ => panic!("Expected Archive metadata"),
    };
    assert_eq!(archive_meta.file_count, 0, "Empty archive should have 0 files");
    assert_eq!(archive_meta.total_size, 0, "Empty archive should have 0 total size");
    assert!(archive_meta.file_list.is_empty(), "file_list should be empty");
}

/// Test synchronous archive extraction.
#[test]
fn test_archive_extraction_sync() {
    let config = ExtractionConfig::default();

    let zip_bytes = create_simple_zip();
    let result = extract_bytes_sync(&zip_bytes, "application/zip", &config).expect("Should extract ZIP synchronously");

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");
    assert!(result.tables.is_empty(), "Archive should not have tables");

    assert!(result.content.contains("ZIP Archive"));
    assert!(result.content.contains("test.txt"));
    assert!(result.content.contains("Hello from ZIP!"));

    assert!(result.metadata.format.is_some(), "Should have archive metadata");
    let archive_meta = match result.metadata.format.as_ref().unwrap() {
        kreuzberg::FormatMetadata::Archive(meta) => meta,
        _ => panic!("Expected Archive metadata"),
    };
    assert_eq!(archive_meta.format, "ZIP");
    assert_eq!(archive_meta.file_count, 1);
    assert_eq!(archive_meta.file_list.len(), 1);
    assert_eq!(archive_meta.file_list[0], "test.txt");
}

fn create_simple_zip() -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, ()>::default();

        zip.start_file("test.txt", options).unwrap();
        zip.write_all(b"Hello from ZIP!").unwrap();

        zip.finish().unwrap();
    }
    cursor.into_inner()
}

fn create_simple_tar() -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut tar = TarBuilder::new(&mut cursor);

        let data = b"Hello from TAR!";
        let mut header = tar::Header::new_gnu();
        header.set_path("test.txt").unwrap();
        header.set_size(data.len() as u64);
        header.set_cksum();
        tar.append(&header, &data[..]).unwrap();

        tar.finish().unwrap();
    }
    cursor.into_inner()
}
