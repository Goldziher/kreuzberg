//! An encrypted ZIP member must not fail the whole archive. The `zip` reader refuses to open a
//! member that needs a password, and that error ended the extraction of every other member. It is
//! now listed, skipped and named, like a member that fails its CRC check (#114). ~keep

#![cfg(feature = "archives")]

use std::io::{Cursor, Write};
use xberg::core::config::ExtractionConfig;
use xberg::{ExtractedDocument, FormatMetadata, SecurityLimits};
use zip::ZipArchive;
use zip::result::ZipError;
use zip::write::{FileOptions, ZipWriter};

mod helpers;
use helpers::extract_bytes_document;

const READABLE_PAYLOAD: &[u8] = b"Readable content.";
const SECRET_PAYLOAD: &[u8] = b"Secret content.";
const LOCAL_HEADER: &[u8] = b"PK\x03\x04";
const CENTRAL_HEADER: &[u8] = b"PK\x01\x02";
const LOCAL_FLAGS_OFFSET: usize = 6;
const CENTRAL_FLAGS_OFFSET: usize = 8;
const LOCAL_METHOD_OFFSET: usize = 8;
const CENTRAL_METHOD_OFFSET: usize = 10;
const LOCAL_SIZE_OFFSET: usize = 22;
const CENTRAL_SIZE_OFFSET: usize = 24;

/// Set general purpose bit 0, which marks a member as encrypted, on every header of `name`. The
/// offsets are those of the local file header and the central directory header. ~keep
fn mark_encrypted(
    bytes: &mut [u8],
    signature: &[u8],
    flag_offset: usize,
    name_length_offset: usize,
    name_offset: usize,
    name: &[u8],
) {
    let mut start = 0;
    while let Some(found) = bytes[start..]
        .windows(signature.len())
        .position(|window| window == signature)
    {
        let header = start + found;
        let name_length = u16::from_le_bytes([
            bytes[header + name_length_offset],
            bytes[header + name_length_offset + 1],
        ]) as usize;
        if bytes.get(header + name_offset..header + name_offset + name_length) == Some(name) {
            bytes[header + flag_offset] |= 1;
        }
        start = header + signature.len();
    }
}

fn zip_with_one_encrypted_member() -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("readable.txt", options).expect("write readable.txt");
        zip.write_all(READABLE_PAYLOAD).expect("write readable payload");

        zip.start_file("secret.txt", options).expect("write secret.txt");
        zip.write_all(SECRET_PAYLOAD).expect("write secret payload");

        zip.finish().expect("finish zip");
    }

    let mut bytes = cursor.into_inner();
    mark_encrypted(&mut bytes, LOCAL_HEADER, 6, 26, 30, b"secret.txt");
    mark_encrypted(&mut bytes, CENTRAL_HEADER, 8, 28, 46, b"secret.txt");
    bytes
}

fn assert_password_required(bytes: &[u8], name: &str) {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).expect("the fixture is a valid ZIP");
    assert!(matches!(
        archive.by_name(name),
        Err(ZipError::UnsupportedArchive(ZipError::PASSWORD_REQUIRED))
    ));
}

fn assert_listing(result: &ExtractedDocument) {
    let Some(FormatMetadata::Archive(metadata)) = result.metadata.format.as_ref() else {
        panic!("expected archive metadata");
    };
    assert_eq!(metadata.file_list, ["readable.txt", "secret.txt"]);
    assert_eq!(metadata.file_count, 2);
}

fn header_offsets(bytes: &[u8]) -> (usize, usize) {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).expect("valid fixture archive");
    let index = archive.index_for_name("secret.txt").expect("secret entry exists");
    let file = archive.by_index_raw(index).expect("secret entry has readable headers");
    (
        usize::try_from(file.header_start()).unwrap(),
        usize::try_from(file.central_header_start()).unwrap(),
    )
}

#[tokio::test]
async fn should_extract_the_other_members_and_name_the_encrypted_one() {
    let bytes = zip_with_one_encrypted_member();
    assert_password_required(&bytes, "secret.txt");

    let result = extract_bytes_document(&bytes, "application/zip", &ExtractionConfig::default())
        .await
        .expect("an encrypted member must not fail the whole archive");
    assert_listing(&result);

    assert!(
        result.content.contains("Readable content."),
        "the readable member must still be extracted: {:?}",
        result.content
    );
    assert!(
        result.content.contains("- secret.txt"),
        "the encrypted member must still be listed: {:?}",
        result.content
    );
    assert!(
        !result.content.contains("Secret content."),
        "the encrypted member's bytes must not be read: {:?}",
        result.content
    );
    let messages: Vec<&str> = result
        .processing_warnings
        .iter()
        .filter(|warning| warning.source == "archive")
        .map(|warning| warning.message.as_ref())
        .collect();
    assert_eq!(messages, ["Skipped 1 archive entry that could not be read: secret.txt"]);
}

#[tokio::test]
async fn should_list_and_name_every_member_when_all_are_encrypted() {
    let mut bytes = zip_with_one_encrypted_member();
    mark_encrypted(&mut bytes, LOCAL_HEADER, LOCAL_FLAGS_OFFSET, 26, 30, b"readable.txt");
    mark_encrypted(
        &mut bytes,
        CENTRAL_HEADER,
        CENTRAL_FLAGS_OFFSET,
        28,
        46,
        b"readable.txt",
    );
    for name in ["readable.txt", "secret.txt"] {
        assert_password_required(&bytes, name);
    }

    let result = extract_bytes_document(&bytes, "application/zip", &ExtractionConfig::default())
        .await
        .expect("an all-encrypted archive still has a readable listing");
    assert_listing(&result);
    assert!(!result.content.contains("=== "));
    assert!(!result.content.contains("Readable content."));
    assert!(!result.content.contains("Secret content."));
    assert!(result.children.is_none());
    assert_eq!(result.processing_warnings.len(), 1);
    assert_eq!(result.processing_warnings[0].source, "archive");
    assert_eq!(
        result.processing_warnings[0].message,
        "Skipped 2 archive entries that could not be read: readable.txt, secret.txt"
    );
}

#[tokio::test]
async fn should_preserve_healthy_members_when_a_compression_method_is_unsupported() {
    let mut bytes = zip_with_one_encrypted_member();
    let (local, central) = header_offsets(&bytes);
    bytes[local + LOCAL_FLAGS_OFFSET] &= !1;
    bytes[central + CENTRAL_FLAGS_OFFSET] &= !1;
    for offset in [local + LOCAL_METHOD_OFFSET, central + CENTRAL_METHOD_OFFSET] {
        bytes[offset..offset + 2].copy_from_slice(&u16::MAX.to_le_bytes());
    }
    let mut archive = ZipArchive::new(Cursor::new(&bytes)).expect("valid fixture archive");
    assert!(matches!(
        archive.by_name("secret.txt"),
        Err(ZipError::CompressionMethodNotSupported(u16::MAX))
    ));

    let result = extract_bytes_document(&bytes, "application/zip", &ExtractionConfig::default())
        .await
        .expect("an unsupported member must not fail the healthy one");
    assert_listing(&result);
    assert!(result.content.contains("Readable content."));
    assert!(!result.content.contains("Secret content."));
    assert_eq!(result.processing_warnings.len(), 1);
    assert_eq!(result.processing_warnings[0].source, "archive");
    assert_eq!(
        result.processing_warnings[0].message,
        "Skipped 1 archive entry that could not be read: secret.txt"
    );
}

#[tokio::test]
async fn should_enforce_security_accounting_before_skipping_an_encrypted_member() {
    let original = zip_with_one_encrypted_member();
    let total_size = READABLE_PAYLOAD.len() + SECRET_PAYLOAD.len();
    let mut high_ratio = original.clone();
    let (local, central) = header_offsets(&high_ratio);
    const DECLARED_SIZE: u32 = 8192;
    for offset in [local + LOCAL_SIZE_OFFSET, central + CENTRAL_SIZE_OFFSET] {
        high_ratio[offset..offset + 4].copy_from_slice(&DECLARED_SIZE.to_le_bytes());
    }
    let cases = [
        (
            &original,
            SecurityLimits {
                max_files_in_archive: 1,
                ..Default::default()
            },
            "Archive has too many files: 2 (max: 1)".to_string(),
        ),
        (
            &original,
            SecurityLimits {
                max_archive_size: total_size - 1,
                ..Default::default()
            },
            format!("Archive too large: {total_size} bytes (max: {} bytes)", total_size - 1),
        ),
        (
            &high_ratio,
            SecurityLimits::default(),
            "Potential ZIP bomb detected:".to_string(),
        ),
    ];
    for (bytes, limits, expected) in cases {
        assert_password_required(bytes, "secret.txt");
        let config = ExtractionConfig {
            security_limits: Some(limits),
            ..Default::default()
        };
        let error = extract_bytes_document(bytes, "application/zip", &config)
            .await
            .expect_err("security validation must run before the encrypted member can be skipped");
        assert!(matches!(error, xberg::XbergError::Validation { .. }));
        assert!(error.to_string().contains(&expected), "unexpected guard: {error}");
    }
}
