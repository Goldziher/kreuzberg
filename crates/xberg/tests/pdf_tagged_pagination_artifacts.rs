#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use lopdf::{Document, Object, Stream, dictionary};
use xberg::core::config::{ContentFilterConfig, ExtractionConfig, OutputFormat, PdfConfig};

const ARTIFACTS: [(&str, &str, usize); 4] = [
    ("Header", "Operating procedure revision four", 750),
    ("Footer", "Document reference ABC123", 40),
    ("PageNumber", "Page 1 of 1", 730),
    ("Watermark", "CONTROLLED COPY", 400),
];
const BODY: [&str; 6] = [
    "Incoming deliveries are inspected before the materials enter the warehouse.",
    "The receiving team records the quantities and checks the packaging for damage.",
    "Heavy materials are stored on the lowest shelves and inspected every week.",
    "Orders are collected according to the printed route to minimize walking time.",
    "The packing team verifies each item before sealing and labeling the cartons.",
    "Completed shipments are weighed and dispatched before the afternoon collection.",
];

fn pagination_pdf() -> Vec<u8> {
    let mut document = Document::with_version("1.7");
    let pages = document.new_object_id();
    let font = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let mut content = String::new();
    for (subtype, text, y) in ARTIFACTS {
        content.push_str(&format!(
            "/Artifact << /Type /Pagination /Subtype /{subtype} >> BDC \
             BT /F1 10 Tf 72 {y} Td ({text}) Tj ET EMC\n"
        ));
    }
    for (index, line) in BODY.iter().enumerate() {
        content.push_str(&format!("BT /F1 11 Tf 72 {} Td ({line}) Tj ET\n", 650 - index * 18));
    }
    let stream = document.add_object(Stream::new(dictionary! {}, content.into_bytes()));
    let page = document.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => stream,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
    });
    document.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1,
        }),
    );
    let catalog = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    document.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    document
        .save_to(&mut bytes)
        .expect("synthetic pagination PDF must serialize");
    bytes
}

fn markdown(bytes: &[u8], filter: Option<ContentFilterConfig>, extract_tables: bool) -> String {
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        use_cache: false,
        content_filter: filter,
        pdf_options: Some(PdfConfig {
            extract_tables,
            ..Default::default()
        }),
        ..Default::default()
    };
    let content = extract_bytes_document_blocking(bytes, "application/pdf", &config)
        .expect("pagination PDF must extract")
        .content;
    for line in BODY {
        assert!(content.contains(line), "body line missing: {line:?}\n{content}");
    }
    content
}

#[test]
fn should_drop_pagination_artifacts_by_default() {
    let bytes = pagination_pdf();
    for extract_tables in [false, true] {
        let content = markdown(&bytes, None, extract_tables);
        for (_, text, _) in ARTIFACTS {
            assert!(!content.contains(text), "default retained {text:?}\n{content}");
        }
    }
}

#[test]
fn should_retain_only_requested_pagination_artifacts_with_or_without_table_segment_reuse() {
    let bytes = pagination_pdf();
    for extract_tables in [false, true] {
        for flags in 0..8 {
            let filter = ContentFilterConfig {
                include_headers: flags & 1 != 0,
                include_footers: flags & 2 != 0,
                include_watermarks: flags & 4 != 0,
                ..Default::default()
            };
            // ~keep Pagination does not identify a page number's running region; either opt-in retains it.
            let expected = [
                filter.include_headers,
                filter.include_footers,
                filter.include_headers || filter.include_footers,
                filter.include_watermarks,
            ];
            let content = markdown(&bytes, Some(filter), extract_tables);
            for ((_, text, _), retained) in ARTIFACTS.into_iter().zip(expected) {
                assert_eq!(
                    content.contains(text),
                    retained,
                    "flags={flags}, extract_tables={extract_tables}, artifact={text:?}\n{content}"
                );
            }
        }
    }
}
