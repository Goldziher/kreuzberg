#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use lopdf::{Document, Object, Stream, dictionary};
use xberg::core::config::{ContentFilterConfig, ExtractionConfig, OutputFormat, PdfBackend, PdfConfig};

const PAGE_COUNT: usize = 10;
const BODY_LINES_PER_PAGE: usize = 6;
const MATRIX_CASES: usize = 16;
const DEFAULT_CASES: usize = 2;
const PAGE_WIDTH: i64 = 612;
const PAGE_HEIGHT: i64 = 792;
const HEADER_Y: usize = 750;
const FOOTER_Y: usize = 40;
const BODY_TOP_Y: usize = 640;
const BODY_LINE_GAP: usize = 50;

fn header(page: usize) -> String {
    format!("Page {page:03}")
}

fn footer(page: usize) -> String {
    format!("Page {page:03} of {PAGE_COUNT:03}")
}

fn body(page: usize, line: usize) -> String {
    format!("Record {page:03} row {line:03} confirms materials arrive safely before storage.")
}

fn page_stream(page: usize) -> Vec<u8> {
    let mut content = format!(
        "BT /F1 9 Tf 260 {HEADER_Y} Td ({}) Tj ET\n\
         BT /F1 9 Tf 260 {FOOTER_Y} Td ({}) Tj ET\n",
        header(page),
        footer(page)
    );
    for line in 1..=BODY_LINES_PER_PAGE {
        let y = BODY_TOP_Y - (line - 1) * BODY_LINE_GAP;
        content.push_str(&format!("BT /F1 11 Tf 72 {y} Td ({}) Tj ET\n", body(page, line)));
    }
    content.into_bytes()
}

fn geometric_pagination_pdf() -> Vec<u8> {
    let mut document = Document::with_version("1.7");
    let pages = document.new_object_id();
    let font = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let mut children: Vec<Object> = Vec::with_capacity(PAGE_COUNT);
    for page in 1..=PAGE_COUNT {
        let stream = document.add_object(Stream::new(dictionary! {}, page_stream(page)));
        let page = document.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages, "Contents" => stream,
            "MediaBox" => vec![0.into(), 0.into(), PAGE_WIDTH.into(), PAGE_HEIGHT.into()],
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
        });
        children.push(page.into());
    }
    document.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => children, "Count" => PAGE_COUNT as i64,
        }),
    );
    let catalog = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    document.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    document
        .save_to(&mut bytes)
        .expect("synthetic pagination PDF must serialize");
    let native = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).expect("native parses synthetic pagination");
    assert_eq!(native.page_count().expect("native counts synthetic pages"), PAGE_COUNT);
    bytes
}

fn markdown(bytes: &[u8], filter: Option<ContentFilterConfig>, extract_tables: bool) -> String {
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        disable_ocr: true,
        use_cache: false,
        content_filter: filter,
        pdf_options: Some(PdfConfig {
            backend: PdfBackend::Native,
            extract_tables,
            ..Default::default()
        }),
        ..Default::default()
    };
    let document = extract_bytes_document_blocking(bytes, "application/pdf", &config)
        .expect("geometric pagination PDF must extract");
    document.content
}

fn assert_census(content: &str, include_headers: bool, include_footers: bool, case: &str) {
    let lines = content.lines().map(str::trim).collect::<Vec<_>>();
    let mut expected_body_lines = 0;
    for page in 1..=PAGE_COUNT {
        for line in 1..=BODY_LINES_PER_PAGE {
            let expected = body(page, line);
            assert_eq!(
                lines.iter().filter(|text| **text == expected).count(),
                1,
                "body census: case={case}, expected={expected:?}\n{content}"
            );
            expected_body_lines += 1;
        }
        for (expected, included) in [(header(page), include_headers), (footer(page), include_footers)] {
            assert_eq!(
                lines.iter().filter(|text| **text == expected).count(),
                usize::from(included),
                "pagination census: case={case}, expected={expected:?}\n{content}"
            );
        }
    }
    assert_eq!(expected_body_lines, PAGE_COUNT * BODY_LINES_PER_PAGE);
}

#[test]
fn should_preserve_only_requested_geometric_pagination_regions() {
    let bytes = geometric_pagination_pdf();
    let cases = [false, true]
        .into_iter()
        .flat_map(|tables| (0..8).map(move |flags| (tables, flags)))
        .collect::<Vec<_>>();
    assert_eq!(cases.len(), MATRIX_CASES);
    let mut verified = 0;
    for (extract_tables, flags) in cases {
        let filter = ContentFilterConfig {
            include_headers: flags & 1 != 0,
            include_footers: flags & 2 != 0,
            strip_repeating_text: flags & 4 != 0,
            ..Default::default()
        };
        let expected_headers = filter.include_headers;
        let expected_footers = filter.include_footers;
        let case = format!("flags={flags}, extract_tables={extract_tables}");
        let content = markdown(&bytes, Some(filter), extract_tables);
        assert_census(&content, expected_headers, expected_footers, &case);
        verified += 1;
    }
    assert_eq!(verified, MATRIX_CASES);
}

#[test]
fn should_remove_geometric_pagination_by_default_without_losing_body_lines() {
    let bytes = geometric_pagination_pdf();
    let cases = [false, true];
    assert_eq!(cases.len(), DEFAULT_CASES);
    let mut verified = 0;
    for extract_tables in cases {
        let content = markdown(&bytes, None, extract_tables);
        assert_census(
            &content,
            false,
            false,
            &format!("default, extract_tables={extract_tables}"),
        );
        verified += 1;
    }
    assert_eq!(verified, DEFAULT_CASES);
}

#[test]
fn should_preserve_requested_geometric_footers_without_retaining_headers() {
    let bytes = geometric_pagination_pdf();
    let filter = ContentFilterConfig {
        include_headers: false,
        include_footers: true,
        strip_repeating_text: false,
        ..Default::default()
    };
    let content = markdown(&bytes, Some(filter), false);
    assert_census(&content, false, true, "footer-only, extract_tables=false");
}
