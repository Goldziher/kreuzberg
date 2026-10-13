//! Tagged PDFs mark running headers, footers and page numbers as
//! `/Artifact /Pagination` marked content (ISO 32000-1:2008 §14.8.2.2).
//!
//! The structured (Markdown) pipeline dropped every artifact span during segment
//! extraction, before `ContentFilterConfig` was consulted, so `include_headers` /
//! `include_footers` could not bring that text back. Documents that carry their
//! title, revision or document number in a tagged running header or footer lost
//! it entirely in Markdown output while plain-text output kept it.

#![cfg(feature = "pdf")]

mod helpers;
use helpers::extract_bytes_document_blocking;

use xberg::core::config::{ContentFilterConfig, ExtractionConfig, OutputFormat};

const HEADER: &str = "WAREHOUSE OPERATIONS MANUAL REVISION 4";
const PAGE_NUMBER: &str = "Page 1 of 1";
const FOOTER: &str = "Document No. WOM-2026-04";
/// Enough body text that the artifacts stay well under the 30% of native tokens the
/// structured pipeline may omit before it falls back to flat native text (which
/// never filtered artifacts and so cannot show this bug).
const BODY: [&str; 6] = [
    "RECEIVING: Inbound pallets are inspected at the dock before they are shelved",
    "and any damaged cartons are set aside for the carrier to collect the next day.",
    "STORAGE: Heavy items go on the lowest racks and fragile items on the middle racks.",
    "PICKING: Orders are picked in the sequence printed on the route sheet so that",
    "each aisle is visited once and the cart returns to the packing tables quickly.",
    "SHIPPING: Packed orders are weighed and labelled before the afternoon pickup.",
];

/// One Letter-size page: a `/Header` artifact and a `/PageNumber` artifact at the
/// top, body paragraphs, and a `/Footer` artifact at the bottom.
fn tagged_pagination_pdf() -> Vec<u8> {
    let body: String = BODY
        .iter()
        .enumerate()
        .map(|(index, line)| format!("BT /F1 11 Tf 72 {} Td ({line}) Tj ET\n", 650 - 16 * index))
        .collect();
    let content = format!(
        "/Artifact << /Type /Pagination /Subtype /Header >> BDC\n\
         BT /F1 12 Tf 200 750 Td ({HEADER}) Tj ET\n\
         EMC\n\
         /Artifact << /Type /Pagination /Subtype /PageNumber >> BDC\n\
         BT /F1 10 Tf 480 730 Td ({PAGE_NUMBER}) Tj ET\n\
         EMC\n\
         {body}\
         /Artifact << /Type /Pagination /Subtype /Footer >> BDC\n\
         BT /F1 10 Tf 230 40 Td ({FOOTER}) Tj ET\n\
         EMC\n"
    );
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
         /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"
            .to_string(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        format!("<< /Length {} >>\nstream\n{content}endstream", content.len()),
    ];

    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn extract_markdown(content_filter: Option<ContentFilterConfig>) -> String {
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        use_cache: false,
        content_filter,
        ..Default::default()
    };
    extract_bytes_document_blocking(&tagged_pagination_pdf(), "application/pdf", &config)
        .expect("extraction must succeed")
        .content
}

fn assert_body_present(markdown: &str) {
    for line in BODY {
        assert!(markdown.contains(line), "body line {line:?} missing:\n{markdown}");
    }
}

fn filter(include_headers: bool, include_footers: bool) -> Option<ContentFilterConfig> {
    Some(ContentFilterConfig {
        include_headers,
        include_footers,
        ..Default::default()
    })
}

#[test]
fn default_markdown_drops_pagination_artifacts() {
    let markdown = extract_markdown(None);

    assert_body_present(&markdown);
    for artifact in [HEADER, PAGE_NUMBER, FOOTER] {
        assert!(
            !markdown.contains(artifact),
            "{artifact:?} should be stripped by default:\n{markdown}"
        );
    }
}

#[test]
fn include_headers_keeps_header_and_page_number_artifacts() {
    let markdown = extract_markdown(filter(true, false));

    assert_body_present(&markdown);
    assert!(markdown.contains(HEADER), "header artifact missing:\n{markdown}");
    assert!(
        markdown.contains(PAGE_NUMBER),
        "page number artifact missing:\n{markdown}"
    );
    assert!(!markdown.contains(FOOTER), "footer should stay stripped:\n{markdown}");
}

#[test]
fn include_footers_keeps_footer_and_page_number_artifacts() {
    let markdown = extract_markdown(filter(false, true));

    assert_body_present(&markdown);
    assert!(markdown.contains(FOOTER), "footer artifact missing:\n{markdown}");
    assert!(
        markdown.contains(PAGE_NUMBER),
        "page number artifact missing:\n{markdown}"
    );
    assert!(!markdown.contains(HEADER), "header should stay stripped:\n{markdown}");
}
