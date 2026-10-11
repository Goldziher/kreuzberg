use super::*;
use ::tar::Builder as TarBuilder;
use ::zip::write::{FileOptions, ZipWriter};
use std::io::{Cursor, Write};

#[test]
fn test_extract_gzip_metadata() {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(b"Hello from gzip!").unwrap();
    let compressed = encoder.finish().unwrap();

    let metadata = extract_gzip_metadata(&compressed, &default_limits()).unwrap();
    assert_eq!(metadata.format, "GZIP");
    assert_eq!(metadata.file_count, 1);
    assert_eq!(metadata.total_size, 16);
}

#[test]
fn test_extract_gzip_text_content() {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(b"Hello from gzip!").unwrap();
    let compressed = encoder.finish().unwrap();

    let contents = extract_gzip_text_content(&compressed, &default_limits()).unwrap();
    assert_eq!(contents.len(), 1);
    assert!(contents.values().next().unwrap().contains("Hello from gzip!"));
}

#[test]
fn test_decompress_gzip() {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(b"test content").unwrap();
    let compressed = encoder.finish().unwrap();

    let decompressed = decompress_gzip(&compressed, &default_limits()).unwrap();
    assert_eq!(String::from_utf8(decompressed).unwrap(), "test content");
}

#[test]
fn test_extract_gzip_invalid_data() {
    let invalid = vec![0, 1, 2, 3, 4, 5];
    let result = extract_gzip_metadata(&invalid, &default_limits());
    assert!(result.is_err());
}

#[test]
fn test_extract_gzip_empty_content() {
    use flate2::Compression;
    use flate2::write::GzEncoder;

    let encoder = GzEncoder::new(Vec::new(), Compression::default());
    let compressed = encoder.finish().unwrap();

    let metadata = extract_gzip_metadata(&compressed, &default_limits()).unwrap();
    assert_eq!(metadata.format, "GZIP");
    assert_eq!(metadata.total_size, 0);
}

#[test]
fn test_zip_too_many_files_rejected() {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();

        for i in 0..5 {
            let filename = format!("file_{}.txt", i);
            zip.start_file(&filename, options).unwrap();
            zip.write_all(b"content").unwrap();
        }
        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let limits = SecurityLimits {
        max_files_in_archive: 3,
        ..SecurityLimits::default()
    };
    let result = extract_zip_metadata(&bytes, &limits);
    assert!(result.is_err());
}

#[test]
fn test_gzip_bomb_rejected() {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&[b'A'; 1024]).unwrap();
    let compressed = encoder.finish().unwrap();

    let limits = SecurityLimits {
        max_archive_size: 100,
        ..SecurityLimits::default()
    };
    let result = extract_gzip_metadata(&compressed, &limits);
    assert!(result.is_err());
}

#[test]
fn test_extract_gzip_compressed_tar_metadata() {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let mut tar_data = Vec::new();
    {
        let mut tar = TarBuilder::new(&mut tar_data);

        let data1 = b"Hello from tar.gz!";
        let mut header1 = ::tar::Header::new_gnu();
        header1.set_path("test.txt").unwrap();
        header1.set_size(data1.len() as u64);
        header1.set_cksum();
        tar.append(&header1, &data1[..]).unwrap();

        let data2 = b"# Markdown file";
        let mut header2 = ::tar::Header::new_gnu();
        header2.set_path("readme.md").unwrap();
        header2.set_size(data2.len() as u64);
        header2.set_cksum();
        tar.append(&header2, &data2[..]).unwrap();

        tar.finish().unwrap();
    }

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&tar_data).unwrap();
    let gzip_compressed = encoder.finish().unwrap();

    let metadata = extract_gzip_metadata(&gzip_compressed, &default_limits()).unwrap();

    assert_eq!(metadata.format, "GZIP+TAR");
    assert_eq!(metadata.file_count, 2);
    assert_eq!(metadata.file_list.len(), 2);
    assert!(metadata.total_size > 0);

    let paths: Vec<&str> = metadata.file_list.iter().map(|e| e.path.as_str()).collect();
    assert!(paths.contains(&"test.txt"));
    assert!(paths.contains(&"readme.md"));
}

#[test]
fn test_extract_gzip_compressed_tar_text_content() {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let mut tar_data = Vec::new();
    {
        let mut tar = TarBuilder::new(&mut tar_data);

        let data1 = b"Hello from tar.gz!";
        let mut header1 = ::tar::Header::new_gnu();
        header1.set_path("test.txt").unwrap();
        header1.set_size(data1.len() as u64);
        header1.set_cksum();
        tar.append(&header1, &data1[..]).unwrap();

        let data2 = b"# Markdown content";
        let mut header2 = ::tar::Header::new_gnu();
        header2.set_path("readme.md").unwrap();
        header2.set_size(data2.len() as u64);
        header2.set_cksum();
        tar.append(&header2, &data2[..]).unwrap();

        tar.finish().unwrap();
    }

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&tar_data).unwrap();
    let gzip_compressed = encoder.finish().unwrap();

    let contents = extract_gzip_text_content(&gzip_compressed, &default_limits()).unwrap();

    assert_eq!(contents.len(), 2);
    assert_eq!(contents.get("test.txt").unwrap(), "Hello from tar.gz!");
    assert_eq!(contents.get("readme.md").unwrap(), "# Markdown content");
}

#[test]
fn test_extract_gzip_compressed_tar_both() {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let mut tar_data = Vec::new();
    {
        let mut tar = TarBuilder::new(&mut tar_data);

        let data = b"Combined test content";
        let mut header = ::tar::Header::new_gnu();
        header.set_path("combined.txt").unwrap();
        header.set_size(data.len() as u64);
        header.set_cksum();
        tar.append(&header, &data[..]).unwrap();

        tar.finish().unwrap();
    }

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&tar_data).unwrap();
    let gzip_compressed = encoder.finish().unwrap();

    let (metadata, contents) = extract_gzip(&gzip_compressed, &default_limits()).unwrap();

    assert_eq!(metadata.format, "GZIP+TAR");
    assert_eq!(metadata.file_count, 1);
    assert_eq!(contents.get("combined.txt").unwrap(), "Combined test content");
}

/// A tracing `Layer` that records the `replaced_characters` field of every emitted
/// event, keyed by whether the field was present at all.
#[derive(Clone, Default)]
struct ReplacedCharactersCapture {
    events: std::sync::Arc<std::sync::Mutex<Vec<Option<bool>>>>,
}

impl<S> tracing_subscriber::Layer<S> for ReplacedCharactersCapture
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        struct Visitor(Option<bool>);
        impl tracing::field::Visit for Visitor {
            fn record_debug(&mut self, _field: &tracing::field::Field, _value: &dyn std::fmt::Debug) {}
            fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
                if field.name() == "replaced_characters" {
                    self.0 = Some(value);
                }
            }
        }
        let mut visitor = Visitor(None);
        event.record(&mut visitor);
        self.events.lock().unwrap().push(visitor.0);
    }
}

/// #395: `decode_with_provenance` reports data loss at the point the decode
/// actually happens, so it must survive into the existing lossy-decode warning
/// as a `replaced_characters` field instead of being discarded the way
/// `safe_decode` discarded it.
///
/// Deliberately not run under `quality`: there chardetng resolves arbitrary
/// bytes to a single-byte encoding that maps all of 0x00-0xFF, so nothing is
/// *replaced* -- see the identical note on
/// `extractors::text::should_warn_when_text_source_is_not_valid_utf8`.
#[cfg(not(feature = "quality"))]
#[test]
fn decode_archive_text_reports_replaced_characters_true_for_invalid_utf8() {
    use tracing_subscriber::layer::SubscriberExt as _;

    let capture = ReplacedCharactersCapture::default();
    let filter = tracing_subscriber::EnvFilter::new("warn");
    let subscriber = tracing_subscriber::registry().with(filter).with(capture.clone());

    let bytes: &[u8] = &[b'A', 0xFF, 0xFE, b'B'];
    let text = tracing::subscriber::with_default(subscriber, || decode_archive_text(bytes, "bad.txt"));

    assert!(!text.is_empty(), "decode must still return text, got {text:?}");
    let events = capture.events.lock().unwrap();
    assert_eq!(events.len(), 1, "expected exactly one warning event, got {events:?}");
    assert_eq!(
        events[0],
        Some(true),
        "expected a replaced_characters=true field on the warning, got {:?}",
        events[0]
    );
}

/// Covers per-member rejection and error naming for an oversized ZIP member's text
/// content -- NOT memory-boundedness. `extract_zip_text_content` takes `bytes: &[u8]` and
/// builds a concrete `zip::read::ZipFile` reader internally; there is no seam here to
/// substitute a counting/instrumented `Read` for it, so the actual property the `.take()`
/// exists for (the reader is never asked for more than `cap + 1` bytes) cannot be observed
/// from this test. It is covered by inspection only. What this test does prove: a member
/// whose decompressed content dwarfs `max_content_size` is rejected by a *member-scoped*
/// error that names the member, rather than surfacing only from the aggregate
/// `total_content_size` check (which, once several members have been summed, can no longer
/// report which one was responsible).
///
/// Neutralisation that must break this test: replace `.take(cap.saturating_add(1))` with
/// `.take(u64::MAX)` in `extract_zip_text_content`. That neutralisation does NOT break this test on its
/// own -- the post-read `raw.len() as u64 > cap` check a few lines below still fires and
/// still names "huge.txt", so the assertion still passes. Only removing that length check
/// too (or renaming the error's member field) would fail it, which is exactly the point:
/// this test cannot distinguish a bounded reader from an unbounded one.
/// `--features archives`.
#[test]
fn test_zip_text_content_names_offending_member_when_it_exceeds_content_cap() {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();
        zip.start_file("huge.txt", options).unwrap();
        // Highly compressible filler large enough to dwarf a tiny cap without needing a
        // gigabyte-scale allocation in the test process.
        zip.write_all(&vec![b'A'; 200_000]).unwrap();
        zip.finish().unwrap();
    }
    let bytes = cursor.into_inner();
    let limits = SecurityLimits {
        max_content_size: 1_000,
        ..SecurityLimits::default()
    };

    let result = extract_zip_text_content(&bytes, &limits);

    let error = result.expect_err("a member whose decompressed size dwarfs max_content_size must be rejected");
    let message = error.to_string();
    assert!(
        message.contains("huge.txt"),
        "the rejection must name the offending member instead of only reporting an \
         aggregate total that cannot identify one: {message}"
    );
}

/// Same defect family, different call: `extract_zip_file_bytes`. See the doc comment on
/// `test_zip_text_content_names_offending_member_when_it_exceeds_content_cap` for why this
/// covers per-member rejection and naming, not memory-boundedness.
///
/// Neutralisation that must break this test: replace `.take(cap.saturating_add(1))` with
/// `.take(u64::MAX)` in `extract_zip_file_bytes` -- and, as above, that alone does not break it, since the
/// `content.len() as u64 > cap` check still fires and still names "huge.bin".
/// `--features archives`.
#[test]
fn test_zip_file_bytes_names_offending_member_when_it_exceeds_archive_cap() {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();
        zip.start_file("huge.bin", options).unwrap();
        zip.write_all(&vec![0xABu8; 200_000]).unwrap();
        zip.finish().unwrap();
    }
    let bytes = cursor.into_inner();
    let limits = SecurityLimits {
        max_archive_size: 1_000,
        ..SecurityLimits::default()
    };

    let result = extract_zip_file_bytes(&bytes, &limits);

    let error = result.expect_err("a member whose decompressed size dwarfs max_archive_size must be rejected");
    let message = error.to_string();
    assert!(
        message.contains("huge.bin"),
        "the rejection must name the offending member instead of only reporting an \
         aggregate total that cannot identify one: {message}"
    );
}

/// TAR analogue of `test_zip_text_content_names_offending_member_when_it_exceeds_content_cap`.
/// Same limitation applies: `extract_tar_text_content` takes `bytes: &[u8]` and builds a
/// concrete `tar::Entry` reader internally, with no seam to inject a counting `Read`, so
/// this covers per-member rejection and error naming only -- memory-boundedness rests on
/// inspection.
///
/// Neutralisation that must break this test: replace `.take(cap.saturating_add(1))` with
/// `.take(u64::MAX)` in `extract_tar_text_content`. As with ZIP, that alone does not break it: the
/// `raw.len() as u64 > cap` check still fires and still names "huge.txt".
/// `--features archives`.
#[test]
fn test_tar_text_content_names_offending_member_when_it_exceeds_content_cap() {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut tar = TarBuilder::new(&mut cursor);
        let data = vec![b'B'; 200_000];
        let mut header = ::tar::Header::new_gnu();
        header.set_path("huge.txt").unwrap();
        header.set_size(data.len() as u64);
        header.set_cksum();
        tar.append(&header, &data[..]).unwrap();
        tar.finish().unwrap();
    }
    let bytes = cursor.into_inner();
    let limits = SecurityLimits {
        max_content_size: 1_000,
        ..SecurityLimits::default()
    };

    let result = extract_tar_text_content(&bytes, &limits);

    let error = result.expect_err("a member whose declared size dwarfs max_content_size must be rejected");
    let message = error.to_string();
    assert!(
        message.contains("huge.txt"),
        "the rejection must name the offending member instead of only reporting an \
         aggregate total that cannot identify one: {message}"
    );
}

/// TAR analogue of `test_zip_file_bytes_names_offending_member_when_it_exceeds_archive_cap`.
///
/// Neutralisation that must break this test: replace `.take(cap.saturating_add(1))` with
/// `.take(u64::MAX)` in `extract_tar_file_bytes`. As above, that alone does not break it: the
/// `content.len() as u64 > cap` check still fires and still names "huge.bin".
/// `--features archives`.
#[test]
fn test_tar_file_bytes_names_offending_member_when_it_exceeds_archive_cap() {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut tar = TarBuilder::new(&mut cursor);
        let data = vec![0xCDu8; 200_000];
        let mut header = ::tar::Header::new_gnu();
        header.set_path("huge.bin").unwrap();
        header.set_size(data.len() as u64);
        header.set_cksum();
        tar.append(&header, &data[..]).unwrap();
        tar.finish().unwrap();
    }
    let bytes = cursor.into_inner();
    let limits = SecurityLimits {
        max_archive_size: 1_000,
        ..SecurityLimits::default()
    };

    let result = extract_tar_file_bytes(&bytes, &limits);

    let error = result.expect_err("a member whose declared size dwarfs max_archive_size must be rejected");
    let message = error.to_string();
    assert!(
        message.contains("huge.bin"),
        "the rejection must name the offending member instead of only reporting an \
         aggregate total that cannot identify one: {message}"
    );
}

/// A member that is already valid UTF-8 never reaches the lossy-decode branch at
/// all, so it must not emit any warning -- and therefore no `replaced_characters`
/// field -- regardless of build configuration.
#[test]
fn decode_archive_text_emits_no_warning_for_valid_utf8() {
    use tracing_subscriber::layer::SubscriberExt as _;

    let capture = ReplacedCharactersCapture::default();
    let filter = tracing_subscriber::EnvFilter::new("warn");
    let subscriber = tracing_subscriber::registry().with(filter).with(capture.clone());

    let text = tracing::subscriber::with_default(subscriber, || {
        decode_archive_text("Hello, World!".as_bytes(), "clean.txt")
    });

    assert_eq!(text, "Hello, World!");
    assert!(
        capture.events.lock().unwrap().is_empty(),
        "valid UTF-8 must not emit a decode warning, got {:?}",
        capture.events.lock().unwrap()
    );
}
