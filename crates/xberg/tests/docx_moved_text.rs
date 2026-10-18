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

const VML_BOX: &str = r#"<w:pict><v:shape xmlns:v="urn:schemas-microsoft-com:vml"><v:textbox><w:txbxContent><w:p><w:r><w:t>VML BOX</w:t></w:r></w:p></w:txbxContent></v:textbox></v:shape></w:pict>"#;
const DRAWINGML_BOX: &str = r#"<w:drawing><wps:wsp xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"><wps:txbx><w:txbxContent><w:p><w:r><w:t>DRAWINGML BOX</w:t></w:r></w:p></w:txbxContent></wps:txbx></wps:wsp></w:drawing>"#;
const EQUATION: &str = r#"<m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><m:r><m:t>qz</m:t></m:r></m:oMath>"#;
const DISPLAY_EQUATION: &str = r#"<m:oMathPara xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><m:oMath><m:r><m:t>wv</m:t></m:r></m:oMath></m:oMathPara>"#;

/// Text boxes and equations are read by their own collectors, outside the run text, so a
/// move source must hold them back as well.
#[test]
fn moved_text_boxes_and_equations_are_kept_once() {
    let moved = format!("<w:r>{VML_BOX}</w:r><w:r>{DRAWINGML_BOX}</w:r>{EQUATION}{DISPLAY_EQUATION}");
    let bytes = build_docx(&format!(
        r#"<w:p><w:r><w:t xml:space="preserve">Old place: </w:t></w:r><w:moveFrom w:id="1" w:author="a">{moved}</w:moveFrom></w:p>
<w:p><w:r><w:t xml:space="preserve">New place: </w:t></w:r><w:moveTo w:id="2" w:author="a">{moved}</w:moveTo></w:p>"#
    ));
    let config = ExtractionConfig {
        use_cache: false,
        ..Default::default()
    };

    let doc = extract_bytes_document_blocking(&bytes, WORD_MIME_TYPE, &config).expect("extraction must succeed");

    for text in ["VML BOX", "DRAWINGML BOX", "qz", "wv"] {
        assert_eq!(
            doc.content.matches(text).count(),
            1,
            "{text:?} must appear once, from the move destination: {:?}",
            doc.content
        );
    }
}

/// An insertion that was later moved keeps its revision text, though the move source
/// stays out of the content.
#[test]
fn an_insertion_inside_a_move_source_keeps_its_revision_text() {
    let bytes = build_docx(
        r#"<w:p><w:moveFrom w:id="1" w:author="a"><w:ins w:id="7" w:author="b"><w:r><w:t>INSERTED THEN MOVED</w:t></w:r></w:ins></w:moveFrom></w:p>
<w:p><w:moveTo w:id="2" w:author="a"><w:r><w:t>INSERTED THEN MOVED</w:t></w:r></w:moveTo></w:p>"#,
    );
    let config = ExtractionConfig {
        use_cache: false,
        ..Default::default()
    };

    let doc = extract_bytes_document_blocking(&bytes, WORD_MIME_TYPE, &config).expect("extraction must succeed");

    assert_eq!(
        doc.content.matches("INSERTED THEN MOVED").count(),
        1,
        "the move source must stay out of the content: {:?}",
        doc.content
    );
    let revisions = doc.revisions.expect("the insertion must be recorded");
    let insertion = revisions
        .iter()
        .find(|revision| revision.revision_id == "7")
        .expect("the nested insertion must keep its revision");
    assert_eq!(insertion.kind, xberg::RevisionKind::Insertion);
    assert_eq!(insertion.author.as_deref(), Some("b"));
    assert_eq!(
        insertion.delta.content,
        vec![xberg::DiffLine::Added("INSERTED THEN MOVED".to_string())]
    );
}

/// A move within one text box: the text box collector reads its paragraphs on its own,
/// so it must hold back the move source as the body loop does.
#[test]
fn a_move_inside_a_text_box_is_kept_once() {
    let box_paragraphs = r#"<w:p><w:r><w:t xml:space="preserve">Old: </w:t></w:r><w:moveFrom w:id="1" w:author="a"><w:r><w:t>MOVED &amp; BOXED</w:t><w:tab/></w:r></w:moveFrom></w:p><w:p><w:r><w:t xml:space="preserve">New: </w:t></w:r><w:moveTo w:id="2" w:author="a"><w:r><w:t>MOVED &amp; BOXED</w:t></w:r></w:moveTo></w:p>"#;
    let vml = format!(
        r#"<w:pict><v:shape xmlns:v="urn:schemas-microsoft-com:vml"><v:textbox><w:txbxContent>{box_paragraphs}</w:txbxContent></v:textbox></v:shape></w:pict>"#
    );
    let drawingml = format!(
        r#"<w:drawing><wps:wsp xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"><wps:txbx><w:txbxContent>{box_paragraphs}</w:txbxContent></wps:txbx></wps:wsp></w:drawing>"#
    );
    let config = ExtractionConfig {
        use_cache: false,
        ..Default::default()
    };

    for (kind, text_box) in [("VML", vml), ("DrawingML", drawingml)] {
        let bytes = build_docx(&format!("<w:p><w:r>{text_box}</w:r></w:p>"));
        let doc = extract_bytes_document_blocking(&bytes, WORD_MIME_TYPE, &config).expect("extraction must succeed");

        assert_eq!(
            doc.content, "Old: \nNew: MOVED & BOXED",
            "{kind}: the move source, its tab included, must leave only the text around it"
        );
    }
}
