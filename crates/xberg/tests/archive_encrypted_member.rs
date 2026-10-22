//! An encrypted ZIP member must not fail the whole archive. The `zip` reader refuses to open a
//! member that needs a password, and that error ended the extraction of every other member. It is
//! now listed, skipped and named, like a member that fails its CRC check (#114). ~keep

#![cfg(feature = "archives")]

use std::io::{Cursor, Write};
use xberg::core::config::ExtractionConfig;
use zip::ZipArchive;
use zip::write::{FileOptions, ZipWriter};

mod helpers;
use helpers::extract_bytes_document;

const READABLE_PAYLOAD: &[u8] = b"Readable content.";
const LOCAL_HEADER: &[u8] = b"PK\x03\x04";
const CENTRAL_HEADER: &[u8] = b"PK\x01\x02";

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
        zip.write_all(b"Secret content.").expect("write secret payload");

        zip.finish().expect("finish zip");
    }

    let mut bytes = cursor.into_inner();
    mark_encrypted(&mut bytes, LOCAL_HEADER, 6, 26, 30, b"secret.txt");
    mark_encrypted(&mut bytes, CENTRAL_HEADER, 8, 28, 46, b"secret.txt");
    bytes
}

#[tokio::test]
async fn should_extract_the_other_members_and_name_the_encrypted_one() {
    let bytes = zip_with_one_encrypted_member();
    let mut archive = ZipArchive::new(Cursor::new(bytes.clone())).expect("the fixture is a valid ZIP");
    assert!(
        archive.by_name("secret.txt").is_err(),
        "the fixture's secret.txt must need a password to open"
    );

    let result = extract_bytes_document(&bytes, "application/zip", &ExtractionConfig::default())
        .await
        .expect("an encrypted member must not fail the whole archive");

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
