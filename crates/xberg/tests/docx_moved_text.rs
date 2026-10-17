//! A tracked move leaves the moved text at its old place inside `w:moveFrom` runs,
//! written with `w:t` rather than `w:delText`, and writes it again at the new place
//! inside `w:moveTo`. Word shows it only at the new place. The DOCX parser collected
//! every `w:t`, so moved text appeared twice in `result.content`: where it was moved
//! from and where it was moved to.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test binaries print by design; org logging policy exempts tests
#![cfg(feature = "office")]

mod helpers;
use helpers::extract_bytes_document_blocking;

use std::io::Write;
use xberg::ExtractionConfig;
use zip::write::{SimpleFileOptions, ZipWriter};

const WORD_MIME_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

/// Build a minimal `.docx` around a caller-supplied `<w:body>` content.
fn build_docx(body_xml: &str) -> Vec<u8> {
    let document_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>{body_xml}</w:body>
</w:document>"#
    );

    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default();

        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
    <Default Extension="xml" ContentType="application/xml"/>
    <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
        )
        .unwrap();

        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(document_xml.as_bytes()).unwrap();

        let _ = zip.finish().unwrap();
    }
    buffer
}

#[test]
fn moved_text_is_kept_once_at_its_new_place() {
    let bytes = build_docx(
        r#"<w:p><w:r><w:t xml:space="preserve">Old place: </w:t></w:r><w:moveFromRangeStart w:id="1" w:name="move1"/><w:moveFrom w:id="2" w:author="a"><w:r><w:t>MOVED SENTENCE</w:t></w:r></w:moveFrom><w:moveFromRangeEnd w:id="1"/><w:r><w:t xml:space="preserve"> stays.</w:t></w:r></w:p>
<w:p><w:r><w:t xml:space="preserve">New place: </w:t></w:r><w:moveToRangeStart w:id="3" w:name="move1"/><w:moveTo w:id="4" w:author="a"><w:r><w:t>MOVED SENTENCE</w:t></w:r></w:moveTo><w:moveToRangeEnd w:id="3"/></w:p>
<w:p><w:r><w:t xml:space="preserve">Deleted: </w:t></w:r><w:del w:id="5" w:author="a"><w:r><w:delText>GONE</w:delText></w:r></w:del></w:p>"#,
    );
    let config = ExtractionConfig {
        use_cache: false,
        ..Default::default()
    };

    let doc = extract_bytes_document_blocking(&bytes, WORD_MIME_TYPE, &config).expect("extraction must succeed");

    assert_eq!(
        doc.content.matches("MOVED SENTENCE").count(),
        1,
        "moved text must appear once, at its new place: {:?}",
        doc.content
    );
    assert!(
        doc.content.contains("New place: MOVED SENTENCE"),
        "the moved text is missing from its new place: {:?}",
        doc.content
    );
    assert!(
        doc.content.contains("Old place:  stays."),
        "the text around the old place must stay: {:?}",
        doc.content
    );
    assert!(
        !doc.content.contains("GONE"),
        "deleted text must stay out: {:?}",
        doc.content
    );
}
