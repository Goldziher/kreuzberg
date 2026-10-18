//! An empty content control (`w:sdt`) holds Word's prompt, such as "Click or tap here to
//! enter text.", and flags it with `w:sdtPr/w:showingPlcHdr`. The prompt is not document
//! content, but the DOCX parser read it like the author's text, so every unfilled field
//! of a form or template put the prompt into `result.content`.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test binaries print by design; org logging policy exempts tests
#![cfg(feature = "office")]

mod helpers;
use helpers::extract_bytes_document_blocking;

use std::io::Write;
use xberg::ExtractionConfig;
use zip::write::{SimpleFileOptions, ZipWriter};

const WORD_MIME_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const PROMPT: &str = "Click or tap here to enter text.";

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

fn extract(body_xml: &str) -> String {
    let config = ExtractionConfig {
        use_cache: false,
        ..Default::default()
    };
    extract_bytes_document_blocking(&build_docx(body_xml), WORD_MIME_TYPE, &config)
        .expect("extraction must succeed")
        .content
}

/// An inline content control: `flag` goes into its `w:sdtPr`, `content` into its runs.
fn inline_control(flag: &str, content: &str) -> String {
    format!(
        r#"<w:sdt><w:sdtPr><w:id w:val="1"/><w:placeholder><w:docPart w:val="DefaultPlaceholder"/></w:placeholder>{flag}</w:sdtPr><w:sdtContent><w:r><w:rPr><w:rStyle w:val="PlaceholderText"/></w:rPr><w:t>{content}</w:t></w:r></w:sdtContent></w:sdt>"#
    )
}

#[test]
fn placeholder_prompts_are_not_document_content() {
    let content = extract(&format!(
        r#"<w:p><w:r><w:t xml:space="preserve">Name: </w:t></w:r>{empty}</w:p>
<w:p><w:r><w:t xml:space="preserve">City: </w:t></w:r>{filled}</w:p>
<w:sdt><w:sdtPr><w:id w:val="3"/><w:showingPlcHdr/></w:sdtPr><w:sdtContent><w:p><w:r><w:t>Enter a summary here.</w:t></w:r></w:p></w:sdtContent></w:sdt>
<w:p><w:r><w:t>End.</w:t></w:r></w:p>"#,
        empty = inline_control("<w:showingPlcHdr/>", PROMPT),
        filled = inline_control("", "Berlin"),
    ));

    assert!(!content.contains(PROMPT), "an inline prompt was extracted: {content:?}");
    assert!(
        !content.contains("Enter a summary here."),
        "a block-level prompt was extracted: {content:?}"
    );
    for kept in ["Name:", "City: Berlin", "End."] {
        assert!(content.contains(kept), "{kept:?} missing from {content:?}");
    }
}

#[test]
fn the_placeholder_flag_is_read_in_both_forms_and_can_be_switched_off() {
    let cases = [
        ("<w:showingPlcHdr/>", false),
        ("<w:showingPlcHdr></w:showingPlcHdr>", false),
        (r#"<w:showingPlcHdr w:val="true"/>"#, false),
        (r#"<w:showingPlcHdr w:val="1"/>"#, false),
        (r#"<w:showingPlcHdr w:val="false"/>"#, true),
        (r#"<w:showingPlcHdr w:val="0"/>"#, true),
        (r#"<w:showingPlcHdr w:val="off"></w:showingPlcHdr>"#, true),
    ];
    for (flag, kept) in cases {
        let content = extract(&format!(
            r#"<w:p><w:r><w:t xml:space="preserve">Value: </w:t></w:r>{}</w:p>"#,
            inline_control(flag, "FIELD TEXT")
        ));
        assert_eq!(content.contains("FIELD TEXT"), kept, "{flag}: {content:?}");
        assert!(content.contains("Value:"), "{flag}: {content:?}");
    }
}

#[test]
fn only_the_control_that_shows_its_placeholder_is_skipped() {
    // A filled outer control around an empty inner one, and the reverse.
    let inner_empty = format!(
        r#"<w:sdt><w:sdtPr><w:id w:val="10"/></w:sdtPr><w:sdtContent><w:r><w:t xml:space="preserve">Outer kept </w:t></w:r>{}<w:r><w:t xml:space="preserve"> after</w:t></w:r></w:sdtContent></w:sdt>"#,
        inline_control("<w:showingPlcHdr/>", PROMPT)
    );
    let outer_empty = format!(
        r#"<w:sdt><w:sdtPr><w:id w:val="11"/><w:showingPlcHdr/></w:sdtPr><w:sdtContent><w:r><w:t xml:space="preserve">Outer prompt </w:t></w:r>{}</w:sdtContent></w:sdt>"#,
        inline_control("", "INNER")
    );
    let content = extract(&format!(
        "<w:p>{inner_empty}</w:p><w:p><w:r><w:t>Second.</w:t></w:r>{outer_empty}</w:p>"
    ));

    assert!(content.contains("Outer kept  after"), "{content:?}");
    assert!(!content.contains(PROMPT), "{content:?}");
    assert!(!content.contains("Outer prompt"), "{content:?}");
    assert!(
        !content.contains("INNER"),
        "everything inside an empty control is its prompt: {content:?}"
    );
    assert!(content.contains("Second."), "{content:?}");
}

#[test]
fn a_placeholder_in_a_table_cell_leaves_the_cell_empty() {
    let content = extract(&format!(
        r#"<w:tbl><w:tr><w:tc><w:p><w:r><w:t>Label</w:t></w:r></w:p></w:tc><w:tc><w:p>{}</w:p></w:tc></w:tr><w:tr><w:tc><w:p><w:r><w:t>Other</w:t></w:r></w:p></w:tc><w:tc><w:p>{}</w:p></w:tc></w:tr></w:tbl>"#,
        inline_control("<w:showingPlcHdr/>", PROMPT),
        inline_control("", "Filled"),
    ));

    assert!(!content.contains(PROMPT), "{content:?}");
    for kept in ["Label", "Other", "Filled"] {
        assert!(content.contains(kept), "{kept:?} missing from {content:?}");
    }
}

/// Text box paragraphs are read by their own collector, so a placeholder inside a text
/// box has to be recognized there too, in both the VML and the DrawingML form.
#[test]
fn a_placeholder_inside_a_text_box_is_skipped() {
    let box_paragraphs = format!(
        r#"<w:p><w:r><w:t xml:space="preserve">Boxed: </w:t></w:r>{}</w:p><w:p><w:r><w:t>BOX TEXT</w:t></w:r></w:p>"#,
        inline_control("<w:showingPlcHdr/>", PROMPT)
    );
    let vml = format!(
        r#"<w:pict><v:shape xmlns:v="urn:schemas-microsoft-com:vml"><v:textbox><w:txbxContent>{box_paragraphs}</w:txbxContent></v:textbox></v:shape></w:pict>"#
    );
    let drawingml = format!(
        r#"<w:drawing><wps:wsp xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"><wps:txbx><w:txbxContent>{box_paragraphs}</w:txbxContent></wps:txbx></wps:wsp></w:drawing>"#
    );

    for (kind, text_box) in [("VML", vml), ("DrawingML", drawingml)] {
        let content = extract(&format!("<w:p><w:r>{text_box}</w:r></w:p>"));
        assert!(!content.contains(PROMPT), "{kind}: {content:?}");
        assert!(content.contains("BOX TEXT"), "{kind}: {content:?}");
        assert!(content.contains("Boxed:"), "{kind}: {content:?}");
    }
}

/// A text box that is itself the prompt of an empty control is skipped by the body loop.
#[test]
fn a_text_box_inside_a_placeholder_is_skipped() {
    let content = extract(
        r#"<w:p><w:r><w:t>Before.</w:t></w:r></w:p><w:sdt><w:sdtPr><w:showingPlcHdr/></w:sdtPr><w:sdtContent><w:p><w:r><w:pict><v:shape xmlns:v="urn:schemas-microsoft-com:vml"><v:textbox><w:txbxContent><w:p><w:r><w:t>PROMPT BOX</w:t></w:r></w:p></w:txbxContent></v:textbox></v:shape></w:pict></w:r></w:p></w:sdtContent></w:sdt>"#,
    );

    assert!(!content.contains("PROMPT BOX"), "{content:?}");
    assert!(content.contains("Before."), "{content:?}");
}
