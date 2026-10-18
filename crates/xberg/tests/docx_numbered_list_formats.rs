//! A list numbered in a non-Latin format, such as the Chinese 一、二、三 (`chineseCounting`),
//! was read as a bullet list: only the decimal, letter and roman `w:numFmt` values counted as
//! numbered, so such a list lost its order in the output.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test binaries print by design; org logging policy exempts tests
#![cfg(feature = "office")]

mod helpers;
use helpers::extract_bytes_document_blocking;

use std::io::Write;
use xberg::ExtractionConfig;
use zip::write::{SimpleFileOptions, ZipWriter};

const WORD_MIME_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// Build a minimal `.docx` with a body and a `word/numbering.xml` whose list 1 uses `num_fmt`.
fn build_docx(body_xml: &str, num_fmt: &str) -> Vec<u8> {
    let document_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="{W_NS}"><w:body>{body_xml}</w:body></w:document>"#
    );
    let numbering_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="{W_NS}">
  <w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="{num_fmt}"/><w:lvlText w:val="%1、"/></w:lvl></w:abstractNum>
  <w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num>
</w:numbering>"#
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
    <Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/>
</Types>"#,
        )
        .unwrap();

        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(document_xml.as_bytes()).unwrap();
        zip.start_file("word/numbering.xml", options).unwrap();
        zip.write_all(numbering_xml.as_bytes()).unwrap();

        let _ = zip.finish().unwrap();
    }
    buffer
}

fn list_item(text: &str) -> String {
    format!(
        r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>"#
    )
}

#[test]
fn a_list_in_a_non_latin_numbering_format_stays_numbered() {
    let body = format!("{}{}", list_item("第一项"), list_item("第二项"));
    let config = ExtractionConfig {
        use_cache: false,
        output_format: xberg::core::config::OutputFormat::Markdown,
        ..Default::default()
    };

    for num_fmt in ["chineseCounting", "decimalEnclosedCircle", "russianLower"] {
        let doc = extract_bytes_document_blocking(&build_docx(&body, num_fmt), WORD_MIME_TYPE, &config)
            .expect("extraction must succeed");
        assert_eq!(doc.content, "1. 第一项\n2. 第二项\n", "numFmt {num_fmt}");
    }

    let doc = extract_bytes_document_blocking(&build_docx(&body, "bullet"), WORD_MIME_TYPE, &config)
        .expect("extraction must succeed");
    assert_eq!(doc.content, "- 第一项\n- 第二项\n", "a bullet list stays a bullet list");
}
