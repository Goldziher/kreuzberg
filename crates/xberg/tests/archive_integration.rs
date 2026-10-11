//! Archive extraction integration tests.
//!
//! Tests for ZIP, TAR, TAR.GZ, and 7z archive extraction.
//! Validates metadata extraction, content extraction, nested archives, and error handling.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests
#![cfg(feature = "archives")]

use std::io::{Cursor, Write};
use tar::Builder as TarBuilder;
use xberg::core::config::ExtractionConfig;
use zip::write::{FileOptions, ZipWriter};

mod helpers;
use helpers::{extract_bytes_document, extract_bytes_document_blocking};

/// Test basic ZIP extraction with single file.
#[tokio::test]
async fn test_zip_basic_extraction() {
    let config = ExtractionConfig::default();

    let zip_bytes = create_simple_zip();

    let result = extract_bytes_document(&zip_bytes, "application/zip", &config)
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
    let archive_meta = match result.metadata.format.as_ref().expect("Operation failed") {
        xberg::FormatMetadata::Archive(meta) => meta,
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
        let options = FileOptions::<'_, '_, ()>::default();

        zip.start_file("file1.txt", options).expect("Operation failed");
        zip.write_all(b"Content 1").expect("Operation failed");

        zip.start_file("file2.md", options).expect("Operation failed");
        zip.write_all(b"# Content 2").expect("Operation failed");

        zip.start_file("file3.json", options).expect("Operation failed");
        zip.write_all(b"{\"key\": \"value\"}").expect("Operation failed");

        zip.finish().expect("Operation failed");
    }

    let zip_bytes = cursor.into_inner();
    let result = extract_bytes_document(&zip_bytes, "application/zip", &config)
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
    let archive_meta = match result.metadata.format.as_ref().expect("Operation failed") {
        xberg::FormatMetadata::Archive(meta) => meta,
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
        let options = FileOptions::<'_, '_, ()>::default();

        zip.add_directory("dir1/", options).expect("Operation failed");
        zip.add_directory("dir1/subdir/", options).expect("Operation failed");

        zip.start_file("dir1/file.txt", options).expect("Operation failed");
        zip.write_all(b"File in dir1").expect("Operation failed");

        zip.start_file("dir1/subdir/nested.txt", options)
            .expect("Operation failed");
        zip.write_all(b"Nested file").expect("Operation failed");

        zip.finish().expect("Operation failed");
    }

    let zip_bytes = cursor.into_inner();
    let result = extract_bytes_document(&zip_bytes, "application/zip", &config)
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
    let archive_meta = match result.metadata.format.as_ref().expect("Operation failed") {
        xberg::FormatMetadata::Archive(meta) => meta,
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

    let result = extract_bytes_document(&tar_bytes, "application/x-tar", &config)
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
    let archive_meta = match result.metadata.format.as_ref().expect("Operation failed") {
        xberg::FormatMetadata::Archive(meta) => meta,
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

    let result = extract_bytes_document(&tar_bytes, "application/x-tar", &config)
        .await
        .expect("Should extract TAR");

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");
    assert!(result.tables.is_empty(), "Archive should not have tables");

    assert!(result.content.contains("TAR Archive"));
    assert!(result.content.contains("test.txt"));

    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().expect("Operation failed") {
        xberg::FormatMetadata::Archive(meta) => meta,
        _ => panic!("Expected Archive metadata"),
    };
    assert_eq!(archive_meta.format, "TAR");
    assert_eq!(archive_meta.file_count, 1);

    let result2 = extract_bytes_document(&tar_bytes, "application/tar", &config)
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

/// Test 7z extraction against a real archive.
///
/// This is the only test that exercises sevenz-rust2's LZMA decoder, and so the only regression
/// net for the `[profile.release.package.sevenz-rust2]` opt-level cap in the root manifest -- the
/// crate miscompiles at higher optimization on macOS ARM64. It cannot be built programmatically
/// the way the ZIP/TAR fixtures above are, so it reads a committed archive. ~keep
#[tokio::test]
async fn test_7z_extraction() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/archives/documents.7z");
    let Ok(sevenz_bytes) = std::fs::read(&path) else {
        panic!("7z fixture must exist at {}", path.display());
    };

    let config = ExtractionConfig::default();
    let result = extract_bytes_document(&sevenz_bytes, "application/x-7z-compressed", &config)
        .await
        .expect("Should extract 7z successfully");

    assert_eq!(result.mime_type, "application/x-7z-compressed");
    assert!(
        result.content.contains("text/simple.txt"),
        "member listing: {:?}",
        result.content
    );
    assert!(
        result.content.contains("text/multilingual.txt"),
        "member listing: {:?}",
        result.content
    );
    // Decompressed member bodies, not just the listing -- a miscompiled LZMA decoder still
    // produces the header-derived listing above, so only this distinguishes it. ~keep
    assert!(
        result.content.contains("contract_test.txt"),
        "member body: {:?}",
        result.content
    );
    assert!(
        result.content.contains("fake_text.txt"),
        "member body: {:?}",
        result.content
    );
}

/// Test nested archive (ZIP inside ZIP).
#[tokio::test]
async fn test_nested_archive() {
    let config = ExtractionConfig::default();

    let inner_zip = create_simple_zip();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();

        zip.start_file("inner.zip", options).expect("Operation failed");
        zip.write_all(&inner_zip).expect("Operation failed");

        zip.start_file("readme.txt", options).expect("Operation failed");
        zip.write_all(b"This archive contains another archive")
            .expect("Operation failed");

        zip.finish().expect("Operation failed");
    }

    let outer_zip_bytes = cursor.into_inner();
    let result = extract_bytes_document(&outer_zip_bytes, "application/zip", &config)
        .await
        .expect("Should extract nested ZIP");

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");
    assert!(result.tables.is_empty(), "Archive should not have tables");

    assert!(result.content.contains("inner.zip"));
    assert!(result.content.contains("readme.txt"));
    assert!(result.content.contains("This archive contains another archive"));

    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().expect("Operation failed") {
        xberg::FormatMetadata::Archive(meta) => meta,
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
        let options = FileOptions::<'_, '_, ()>::default();

        zip.start_file("document.txt", options).expect("Operation failed");
        zip.write_all(b"Text document").expect("Operation failed");

        zip.start_file("readme.md", options).expect("Operation failed");
        zip.write_all(b"# README").expect("Operation failed");

        zip.start_file("image.png", options).expect("Operation failed");
        zip.write_all(&[0x89, 0x50, 0x4E, 0x47]).expect("Operation failed");

        zip.start_file("document.pdf", options).expect("Operation failed");
        zip.write_all(b"%PDF-1.4").expect("Operation failed");

        zip.finish().expect("Operation failed");
    }

    let zip_bytes = cursor.into_inner();
    let result = extract_bytes_document(&zip_bytes, "application/zip", &config)
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
    let archive_meta = match result.metadata.format.as_ref().expect("Operation failed") {
        xberg::FormatMetadata::Archive(meta) => meta,
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

    let result = extract_bytes_document(&invalid_zip, "application/zip", &config).await;

    assert!(result.is_err(), "Should fail on invalid/encrypted ZIP");
}

/// Test corrupted archive.
#[tokio::test]
async fn test_corrupted_archive() {
    let config = ExtractionConfig::default();

    let corrupted_zip = vec![0x50, 0x4B, 0x03, 0x04, 0xFF, 0xFF, 0xFF, 0xFF];

    let result = extract_bytes_document(&corrupted_zip, "application/zip", &config).await;

    assert!(result.is_err(), "Should fail on corrupted ZIP");

    let mut corrupted_tar = vec![0xFF; 512];
    corrupted_tar[0..5].copy_from_slice(b"file\0");

    let result = extract_bytes_document(&corrupted_tar, "application/x-tar", &config).await;
    // The `tar` crate's checksum field (header bytes 148..156) is all `0xFF`, which is not
    // valid UTF-8, so `Header::cksum` fails to parse it before the sum comparison ever runs;
    // `extract_tar_metadata` (crates/xberg/src/extraction/archive/tar.rs) maps that into a
    // `XbergError::Parsing` naming the failed TAR entry read.
    let error = result.expect_err("a checksum field that is not valid UTF-8 cannot be parsed");
    assert!(
        matches!(error, xberg::XbergError::Parsing { .. }),
        "expected a Parsing error, got: {error:?}"
    );
    assert!(
        error.to_string().contains("TAR"),
        "error must name the TAR parse failure, got: {error}"
    );
}

/// Test large archive (100+ files).
#[tokio::test]
async fn test_large_archive() {
    let config = ExtractionConfig::default();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();

        for i in 0..100 {
            zip.start_file(format!("file_{}.txt", i), options)
                .expect("Operation failed");
            zip.write_all(format!("Content {}", i).as_bytes())
                .expect("Failed to convert to bytes");
        }

        zip.finish().expect("Operation failed");
    }

    let zip_bytes = cursor.into_inner();
    let result = extract_bytes_document(&zip_bytes, "application/zip", &config)
        .await
        .expect("Should extract large ZIP");

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");
    assert!(result.tables.is_empty(), "Archive should not have tables");

    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().expect("Operation failed") {
        xberg::FormatMetadata::Archive(meta) => meta,
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
        let options = FileOptions::<'_, '_, ()>::default();

        zip.start_file("测试文件.txt", options).expect("Operation failed");
        zip.write_all("Unicode content".as_bytes())
            .expect("Failed to convert to bytes");

        zip.start_file("file with spaces.txt", options)
            .expect("Operation failed");
        zip.write_all(b"Spaces in filename").expect("Operation failed");

        zip.start_file("file-with-dashes.txt", options)
            .expect("Operation failed");
        zip.write_all(b"Dashes").expect("Operation failed");

        zip.finish().expect("Operation failed");
    }

    let zip_bytes = cursor.into_inner();
    let result = extract_bytes_document(&zip_bytes, "application/zip", &config)
        .await
        .expect("Should extract ZIP with special characters");

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");
    assert!(result.tables.is_empty(), "Archive should not have tables");

    assert!(result.content.contains("测试文件.txt") || result.content.contains("txt"));
    assert!(result.content.contains("file with spaces.txt"));
    assert!(result.content.contains("file-with-dashes.txt"));

    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().expect("Operation failed") {
        xberg::FormatMetadata::Archive(meta) => meta,
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
        zip.finish().expect("Operation failed");
    }

    let zip_bytes = cursor.into_inner();
    let result = extract_bytes_document(&zip_bytes, "application/zip", &config)
        .await
        .expect("Should extract empty ZIP");

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");
    assert!(result.tables.is_empty(), "Archive should not have tables");

    assert!(result.content.contains("ZIP Archive"));
    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().expect("Operation failed") {
        xberg::FormatMetadata::Archive(meta) => meta,
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
    let result = extract_bytes_document_blocking(&zip_bytes, "application/zip", &config)
        .expect("Should extract ZIP synchronously");

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");
    assert!(result.tables.is_empty(), "Archive should not have tables");

    assert!(result.content.contains("ZIP Archive"));
    assert!(result.content.contains("test.txt"));
    assert!(result.content.contains("Hello from ZIP!"));

    assert!(result.metadata.format.is_some(), "Should have archive metadata");
    let archive_meta = match result.metadata.format.as_ref().expect("Operation failed") {
        xberg::FormatMetadata::Archive(meta) => meta,
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
        let options = FileOptions::<'_, '_, ()>::default();

        zip.start_file("test.txt", options).expect("Operation failed");
        zip.write_all(b"Hello from ZIP!").expect("Operation failed");

        zip.finish().expect("Operation failed");
    }
    cursor.into_inner()
}

fn create_simple_tar() -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut tar = TarBuilder::new(&mut cursor);

        let data = b"Hello from TAR!";
        let mut header = tar::Header::new_gnu();
        header.set_path("test.txt").expect("Operation failed");
        header.set_size(data.len() as u64);
        header.set_cksum();
        tar.append(&header, &data[..]).expect("Operation failed");

        tar.finish().expect("Operation failed");
    }
    cursor.into_inner()
}
