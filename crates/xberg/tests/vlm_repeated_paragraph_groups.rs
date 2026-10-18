//! Regression coverage for #2132: a scanned page transcribed by a VLM lost one of eight repeated
//! paragraph groups. A stub registered as the page OCR backend returns the same eight groups of
//! three sentences for every page, the way the VLM backend returns its visible text unchanged,
//! so this exercises only what Xberg does between the backend and the result: page assembly,
//! the content filter and the output format.
//!
//! The check is an absolute occurrence count of each sentence, not a ratio over the retained
//! text, because a ratio rises when a repeated group is the part that was dropped.

#![allow(deprecated)]
#![cfg(all(feature = "pdf", feature = "ocr"))]

mod helpers;

use async_trait::async_trait;
use helpers::extract_bytes_document_blocking;
use std::sync::Arc;
use xberg::ExtractedDocument;
use xberg::core::config::{ContentFilterConfig, ExtractionConfig, OcrConfig, OutputFormat, PageConfig};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};

const PAGE_PT: u32 = 200;
const RASTER_PX: u32 = 64;
const PAGE_COUNT: usize = 3;
const GROUPS_PER_PAGE: usize = 8;
const SENTENCES: [&str; 3] = [
    "The committee reviewed the quarterly figures.",
    "Each department submitted its report on time.",
    "No further action was required from the board.",
];

fn page_text() -> String {
    let group = SENTENCES.join(" ");
    vec![group; GROUPS_PER_PAGE].join("\n\n")
}

struct TextStub;

impl Plugin for TextStub {
    fn name(&self) -> &str {
        "tesseract"
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> xberg::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> xberg::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl OcrBackend for TextStub {
    async fn process_image(&self, _image_bytes: &[u8], _config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        let mut page = ExtractedDocument::default();
        page.content = page_text();
        Ok(page)
    }
    fn supports_language(&self, _language: &str) -> bool {
        true
    }
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

/// A PDF whose pages carry only a grey raster and no text, so every page needs OCR.
fn raster_only_pdf() -> Vec<u8> {
    let raster = vec![0xA0u8; (RASTER_PX * RASTER_PX) as usize];
    let content = format!("q {PAGE_PT} 0 0 {PAGE_PT} 0 0 cm /Im0 Do Q\n");
    let first_page_object = 4;
    let kids: String = (0..PAGE_COUNT)
        .map(|index| format!("{} 0 R", first_page_object + index))
        .collect::<Vec<_>>()
        .join(" ");

    let mut buf = Vec::<u8>::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::new();
    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<</Type /Catalog /Pages 2 0 R>>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(format!("2 0 obj\n<</Type /Pages /Kids [{kids}] /Count {PAGE_COUNT}>>\nendobj\n").as_bytes());
    offsets.push(buf.len());
    buf.extend_from_slice(
        format!(
            "3 0 obj\n<</Type /XObject /Subtype /Image /Width {RASTER_PX} /Height {RASTER_PX} \
             /ColorSpace /DeviceGray /BitsPerComponent 8 /Length {}>>\nstream\n",
            raster.len()
        )
        .as_bytes(),
    );
    buf.extend_from_slice(&raster);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let contents_object = first_page_object + PAGE_COUNT;
    for index in 0..PAGE_COUNT {
        offsets.push(buf.len());
        buf.extend_from_slice(
            format!(
                "{} 0 obj\n<</Type /Page /MediaBox [0 0 {PAGE_PT} {PAGE_PT}] /Parent 2 0 R \
                 /Contents {contents_object} 0 R /Resources <</XObject <</Im0 3 0 R>> >> >>\nendobj\n",
                first_page_object + index
            )
            .as_bytes(),
        );
    }
    offsets.push(buf.len());
    buf.extend_from_slice(
        format!(
            "{contents_object} 0 obj\n<</Length {}>>\nstream\n{content}\nendstream\nendobj\n",
            content.len() + 1
        )
        .as_bytes(),
    );

    let xref_offset = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
    for offset in &offsets {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<</Size {} /Root 1 0 R>>\n", offsets.len() + 1).as_bytes());
    buf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());
    buf
}

fn config(output_format: OutputFormat, content_filter: Option<ContentFilterConfig>) -> ExtractionConfig {
    ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            ..Default::default()
        }),
        force_ocr: true,
        output_format,
        content_filter,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    }
}

fn extract(config: &ExtractionConfig) -> ExtractedDocument {
    let pdf = raster_only_pdf();
    let _ = unregister_ocr_backend("tesseract");
    register_ocr_backend(Arc::new(TextStub)).expect("the stub backend registers");
    let result = extract_bytes_document_blocking(&pdf, "application/pdf", config);
    unregister_ocr_backend("tesseract").expect("the stub backend unregisters");
    result.expect("extraction of the raster-only fixture must succeed")
}

fn assert_groups_survive(label: &str, combined: &str, pages: &[(u32, &str)]) {
    assert_eq!(pages.len(), PAGE_COUNT, "{label}: page count");
    for sentence in SENTENCES {
        assert_eq!(
            combined.matches(sentence).count(),
            GROUPS_PER_PAGE * PAGE_COUNT,
            "{label}: occurrences of {sentence:?} in the combined content"
        );
        for (page_number, content) in pages {
            assert_eq!(
                content.matches(sentence).count(),
                GROUPS_PER_PAGE,
                "{label}: occurrences of {sentence:?} on page {page_number}"
            );
        }
    }
}

fn assert_every_group_survives(label: &str, result: &ExtractedDocument) {
    let pages: Vec<(u32, &str)> = result
        .pages
        .as_ref()
        .expect("per-page content was requested")
        .iter()
        .map(|page| (page.page_number, page.content.as_str()))
        .collect();
    assert_groups_survive(label, &result.content, &pages);
}

#[test]
#[serial_test::serial]
fn plain_output_without_a_content_filter_keeps_every_repeated_group() {
    let result = extract(&config(OutputFormat::Plain, None));
    assert_every_group_survives("plain, no filter", &result);
}

#[test]
#[serial_test::serial]
fn markdown_output_without_a_content_filter_keeps_every_repeated_group() {
    let result = extract(&config(OutputFormat::Markdown, None));
    assert_every_group_survives("markdown, no filter", &result);
}

#[test]
#[serial_test::serial]
fn plain_output_with_the_default_content_filter_keeps_every_repeated_group() {
    let result = extract(&config(OutputFormat::Plain, Some(ContentFilterConfig::default())));
    assert_every_group_survives("plain, default filter", &result);
}

#[test]
fn the_occurrence_check_rejects_a_dropped_group() {
    let intact = page_text();
    let combined = vec![intact.clone(); PAGE_COUNT].join("\n\n");
    let pages: Vec<(u32, &str)> = (1..=PAGE_COUNT as u32)
        .map(|number| (number, intact.as_str()))
        .collect();
    assert_groups_survive("control", &combined, &pages);

    let dropped = combined.replacen(&SENTENCES.join(" "), "", 1);
    let outcome = std::panic::catch_unwind(|| assert_groups_survive("planted drop", &dropped, &pages));
    assert!(
        outcome.is_err(),
        "a combined text missing one group must fail the check"
    );
}
