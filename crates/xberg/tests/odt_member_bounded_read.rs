//! Note: `zip` >= 9.0.1 also fails reads that exceed the declared uncompressed size, so a
//! lying archive is now rejected earlier; `Read::take` remains as the second bound.
//!
//! Regression test: an ODT ZIP member's *declared* uncompressed size is not
//! what actually gets read out of the archive.
//!
//! `ZipBombValidator::validate` (run once at archive open) computes its ratio
//! and size checks entirely from the ZIP central directory's declared
//! `compressed_size`/`uncompressed_size` fields (via `by_index_raw`, which
//! never decompresses anything). Those fields are ordinary attacker-controlled
//! metadata: the `zip` crate's actual read path
//! (`ZipFileData::find_content`, `types.rs` in the `zip` crate) bounds the
//! decompressor's input purely by the central directory's *compressed* size,
//! and nothing in the read path enforces the declared *uncompressed* size at
//! all -- `Crc32Reader` only checks its checksum once the underlying reader
//! reports EOF. So a central directory record can declare a tiny
//! `uncompressed_size` while its real deflate stream expands to far more than
//! that, and `ZipBombValidator` sees only the lie.
//!
//! The fix bounds every ZIP member read with `Read::take(MAX_ODT_MEMBER_SIZE)`
//! at the point the bytes are actually pulled out (`pre_extract_images`,
//! `pre_extract_formulas`, and both `content.xml`/`styles.xml` reads in
//! `crates/xberg/src/extractors/odt.rs`), independent of what the central
//! directory declared. This file proves that mechanism specifically for
//! `pre_extract_images`, whose output (raw, unparsed bytes) can be measured by
//! length exactly: an embedded image's real decompressed size is held fixed at
//! and one byte over `MAX_ODT_MEMBER_SIZE`, while the ZIP central directory is
//! made to lie and declare a 5-byte image. Against unfixed code (`read_to_end`
//! with no `.take()`) the "at cap" test still passes, but the "over cap" test
//! fails: the returned image would be `MAX_ODT_MEMBER_SIZE + 1` bytes long
//! (the full real payload, unbounded) instead of truncated to the cap.
//!
//! Building the crafted archive does not need a raw ZIP-byte hand-assembly
//! like `issue_197_archive_size_accounting_overflow.rs` uses: `zip::ZipWriter`
//! already computes a correct, real `compressed_size` for genuine payload
//! data, so only the central directory's `uncompressed_size` *field* needs to
//! be patched after the fact -- the local file header's own size fields are
//! provably irrelevant to reading (the `zip` crate consults them only to
//! locate where file *data* starts, via file-name/extra-field lengths).

#![cfg(feature = "office")]

use std::io::Write;
use xberg::{ExtractionConfig, Result as XbergResult, SecurityLimits};
use zip::write::{FileOptions, ZipWriter};

mod helpers;
use helpers::extract_bytes_document_blocking;

const ODT_MIME: &str = "application/vnd.oasis.opendocument.text";

/// Mirrors `MAX_ODT_MEMBER_SIZE` in `crates/xberg/src/extractors/odt.rs`
/// (`pub(crate)`, so not importable from an external integration test). Keep
/// this in sync if that constant changes.
const MAX_ODT_MEMBER_SIZE: usize = 100 * 1024 * 1024;

const CENTRAL_DIRECTORY_HEADER_SIGNATURE: u32 = 0x0201_4b50;

/// Overwrite the *central directory's* declared uncompressed-size field for
/// `entry_name` with `lie`. The real compressed data, the real
/// `compressed_size` field, and the CRC are all left untouched, so the entry
/// remains fully readable -- decompression still produces every real byte
/// that was written, since the read path bounds itself on `compressed_size`
/// alone (see the module doc above).
fn lie_about_declared_uncompressed_size(buf: &mut [u8], entry_name: &str, lie: u32) {
    // Locate the central directory via the End Of Central Directory record,
    // which -- with no ZIP comment and no ZIP64 (our archives are a few
    // hundred bytes of headers plus one large but sub-4 GiB entry) -- is
    // exactly the last 22 bytes of the file.
    let eocd = &buf[buf.len() - 22..];
    assert_eq!(
        u32::from_le_bytes(eocd[0..4].try_into().unwrap()),
        0x0605_4b50,
        "expected an End Of Central Directory record in the last 22 bytes (no comment, no zip64)"
    );
    let central_directory_offset = u32::from_le_bytes(eocd[16..20].try_into().unwrap()) as usize;

    let name_bytes = entry_name.as_bytes();
    let mut offset = central_directory_offset;
    while offset + 46 <= buf.len() {
        let signature = u32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap());
        assert_eq!(
            signature, CENTRAL_DIRECTORY_HEADER_SIGNATURE,
            "expected a central directory file header at offset {offset}"
        );
        let name_len = u16::from_le_bytes(buf[offset + 28..offset + 30].try_into().unwrap()) as usize;
        let extra_len = u16::from_le_bytes(buf[offset + 30..offset + 32].try_into().unwrap()) as usize;
        let comment_len = u16::from_le_bytes(buf[offset + 32..offset + 34].try_into().unwrap()) as usize;
        let name_start = offset + 46;
        if &buf[name_start..name_start + name_len] == name_bytes {
            buf[offset + 24..offset + 28].copy_from_slice(&lie.to_le_bytes());
            return;
        }
        offset = name_start + name_len + extra_len + comment_len;
    }
    panic!("central directory record for {entry_name:?} not found");
}

/// Build a minimal, otherwise well-formed, ODT whose `Pictures/big.bin` entry
/// really decompresses to `real_uncompressed_len` bytes but whose central
/// directory declares an unrelated 5-byte size for it. `content.xml`
/// references the image so it reaches `pre_extract_images`' output via the
/// public `ExtractedDocument.images` field.
fn odt_with_picture_size(real_uncompressed_len: usize, lie: bool) -> Vec<u8> {
    let content_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
    xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
    xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
    xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
    xmlns:xlink="http://www.w3.org/1999/xlink">
  <office:body>
    <office:text>
      <draw:frame><draw:image xlink:href="Pictures/big.bin"/></draw:frame>
    </office:text>
  </office:body>
</office:document-content>"#;

    // A single repeated byte deflates to a tiny real compressed size, keeping
    // this test fast and small on disk/in-memory despite the large logical
    // payload; the value itself (0x41 = 'A') is only there so the truncation
    // assertion can also confirm the surviving bytes are the real payload.
    let payload = vec![0x41u8; real_uncompressed_len];

    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf));
        let stored = FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("mimetype", stored).unwrap();
        zip.write_all(ODT_MIME.as_bytes()).unwrap();

        let deflated = FileOptions::<()>::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(1));
        zip.start_file("content.xml", deflated).unwrap();
        zip.write_all(content_xml.as_bytes()).unwrap();

        zip.start_file("Pictures/big.bin", deflated).unwrap();
        zip.write_all(&payload).unwrap();

        zip.finish().unwrap();
    }

    // The lie: 5 declared bytes for an entry whose real compressed data
    // decompresses to `real_uncompressed_len`. `ZipBombValidator` computes its
    // ratio from this declared value divided by the (correct, untouched)
    // declared compressed size, so a 5-byte declaration is always comfortably
    // under any configured ratio limit regardless of how large the real
    // payload is.
    if lie {
        lie_about_declared_uncompressed_size(&mut buf, "Pictures/big.bin", 5);
    }
    buf
}

/// Extract and return the single embedded image's raw bytes, or `None` when the
/// extraction produced no image.
fn extract_big_picture_bytes(bytes: &[u8]) -> XbergResult<Option<Vec<u8>>> {
    let config = ExtractionConfig {
        use_cache: false,
        security_limits: Some(SecurityLimits {
            max_compression_ratio: 100_000,
            max_archive_size: 1024 * 1024 * 1024,
            max_content_size: 1024 * 1024 * 1024,
            ..SecurityLimits::default()
        }),
        ..Default::default()
    };
    let result = extract_bytes_document_blocking(bytes, ODT_MIME, &config)?;
    Ok(result.images.and_then(|images| {
        assert_eq!(images.len(), 1, "expected exactly one embedded image");
        images.into_iter().next().map(|image| image.data.to_vec())
    }))
}

/// At exactly `MAX_ODT_MEMBER_SIZE`, every real byte must survive: the cap is
/// a ceiling on truncation, not a reason to reject or shrink a document that
/// sits right at the boundary.
#[test]
fn test_odt_picture_read_is_not_truncated_at_the_cap() {
    let bytes = odt_with_picture_size(MAX_ODT_MEMBER_SIZE, false);
    let data = extract_big_picture_bytes(&bytes)
        .expect("an image exactly at the cap must still extract")
        .expect("extraction must populate the images field for an ODT with an embedded picture");

    assert_eq!(
        data.len(),
        MAX_ODT_MEMBER_SIZE,
        "an image whose real size exactly equals the cap must not lose any bytes"
    );
    assert!(
        data.iter().all(|&b| b == 0x41),
        "every surviving byte must be the real payload byte, not padding or a truncation artifact"
    );
}

/// One byte over the cap with a central directory that declares 5 bytes. `zip` >= 9.0.1
/// fails reads that exceed the declared size, so the lie is now rejected before
/// `Read::take` is reached; either outcome is safe, but the member must never come
/// back larger than the cap.
#[test]
fn test_odt_picture_read_never_exceeds_the_cap_when_the_declared_size_lies() {
    let bytes = odt_with_picture_size(MAX_ODT_MEMBER_SIZE + 1, true);
    if let Ok(Some(data)) = extract_big_picture_bytes(&bytes) {
        assert!(
            data.len() <= MAX_ODT_MEMBER_SIZE,
            "a member over the cap must be truncated or dropped, got {} bytes",
            data.len()
        );
    }
}
