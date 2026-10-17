//! A table row can leave grid columns empty before its first cell (`w:gridBefore`) or
//! after its last (`w:gridAfter`), so its cells start further right than the row above.
//! The DOCX table grid ignored both, so such a row's cells moved left, under the wrong
//! columns, in `result.tables[].cells`, the table's markdown and `result.content`.

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

/// A three-column table: a full row, a row that starts one column in, and a row that
/// ends one column early.
const TABLE_XML: &str = r#"<w:tbl>
  <w:tblGrid><w:gridCol w:w="2000"/><w:gridCol w:w="2000"/><w:gridCol w:w="2000"/></w:tblGrid>
  <w:tr>
    <w:tc><w:p><w:r><w:t>Item</w:t></w:r></w:p></w:tc>
    <w:tc><w:p><w:r><w:t>Q1</w:t></w:r></w:p></w:tc>
    <w:tc><w:p><w:r><w:t>Q2</w:t></w:r></w:p></w:tc>
  </w:tr>
  <w:tr>
    <w:trPr><w:gridBefore w:val="1"/></w:trPr>
    <w:tc><w:p><w:r><w:t>10</w:t></w:r></w:p></w:tc>
    <w:tc><w:p><w:r><w:t>20</w:t></w:r></w:p></w:tc>
  </w:tr>
  <w:tr>
    <w:trPr><w:gridAfter w:val="1"/></w:trPr>
    <w:tc><w:p><w:r><w:t>Total</w:t></w:r></w:p></w:tc>
    <w:tc><w:p><w:r><w:t>30</w:t></w:r></w:p></w:tc>
  </w:tr>
</w:tbl>"#;

#[test]
fn grid_before_and_after_keep_cells_under_their_columns() {
    let config = ExtractionConfig {
        use_cache: false,
        output_format: xberg::core::config::OutputFormat::Markdown,
        ..Default::default()
    };

    let result = extract_bytes_document_blocking(&build_docx(TABLE_XML), WORD_MIME_TYPE, &config)
        .expect("extraction must succeed");

    let table = result.tables.first().expect("a table must be extracted");
    let expected: Vec<Vec<String>> = [["Item", "Q1", "Q2"], ["", "10", "20"], ["Total", "30", ""]]
        .iter()
        .map(|row| row.iter().map(|cell| cell.to_string()).collect())
        .collect();
    assert_eq!(table.cells, expected);

    for markdown in [table.markdown.as_str(), result.content.as_str()] {
        for row in ["|  | 10 | 20 |", "| Total | 30 |  |"] {
            assert!(markdown.contains(row), "{row:?} missing from {markdown:?}");
        }
    }
}
