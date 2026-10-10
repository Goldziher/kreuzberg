//! OCR page failure records: `ExtractedDocument::ocr_page_failures` for a PDF page whose OCR
//! backend call fails while extraction continues (see <https://github.com/xberg-io/xberg/issues/2064>).
//!
//! A stub backend returns no text (or an error) for the page render and a known word (or a
//! table) for the embedded raster. The raster is not square and the page is, so the stub tells
//! the two calls apart by the image size. The document has two pages and only the second
//! carries the raster. OCR of extracted embedded images is off, so the retry on the embedded
//! image is the only call that sees the raster.

#![allow(deprecated)]
#![cfg(all(feature = "pdf", feature = "ocr"))]

mod helpers;

use async_trait::async_trait;
use helpers::extract_bytes_document_blocking;
use std::sync::Arc;
use xberg::ExtractedDocument;
use xberg::OcrPageFailure;
use xberg::core::config::{ExtractionConfig, OcrConfig, OcrPipelineConfig, OcrPipelineStage};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};

/// Page size in points.
const PAGE_PT: u32 = 200;
/// Embedded raster size in pixels. Not square, so no page render has this size.
const RASTER_WIDTH_PX: u32 = 300;
const RASTER_HEIGHT_PX: u32 = 420;
/// The word the stub reads from the embedded raster.
const RECOVERED_WORD: &str = "Quarterly";
/// The warning text every embedded-image retry adds.
const RETRY_WARNING: &str = "OCR was retried on the embedded image bytes";
/// The warning text of a failed page that the retry recovered.
const FAILED_AND_RECOVERED_WARNING: &str = "its text was recovered from the page's embedded image XObjects";

/// What the stub does with the page render.
#[derive(Clone, Copy)]
enum RenderOutcome {
    Blank,
    Fails,
}

/// The width and height from a PNG header, or `None` for bytes that are not a PNG.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((width, height))
}

/// What the stub reads from the embedded raster.
#[derive(Clone, Copy)]
enum RasterOutcome {
    Word,
    TableOnly,
}

struct StubBackend {
    name: &'static str,
    render: RenderOutcome,
    raster: RasterOutcome,
}

impl Plugin for StubBackend {
    fn name(&self) -> &str {
        self.name
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
impl OcrBackend for StubBackend {
    async fn process_image(&self, image_bytes: &[u8], _config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        if png_size(image_bytes) == Some((RASTER_WIDTH_PX, RASTER_HEIGHT_PX)) {
            let mut recovered = ExtractedDocument::default();
            match self.raster {
                RasterOutcome::Word => recovered.content = RECOVERED_WORD.to_string(),
                RasterOutcome::TableOnly => {
                    recovered.tables = vec![xberg::types::Table {
                        cells: vec![vec!["Item".to_string(), "Amount".to_string()]],
                        markdown: "| Item | Amount |\n| --- | --- |\n".to_string(),
                        ..Default::default()
                    }]
                }
            }
            return Ok(recovered);
        }
        match self.render {
            RenderOutcome::Blank => Ok(ExtractedDocument::default()),
            RenderOutcome::Fails => Err(xberg::XbergError::Ocr {
                message: "stub render failure".to_string(),
                source: None,
            }),
        }
    }
    fn supports_language(&self, _language: &str) -> bool {
        true
    }
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

/// A two-page PDF: page 1 (object 6) is empty, page 2 (object 3) paints one grey DeviceGray
/// raster over the page.
fn pdf_with_image_on_page_two() -> Vec<u8> {
    let raster = vec![0xA0u8; (RASTER_WIDTH_PX * RASTER_HEIGHT_PX) as usize];
    let content = format!("q {PAGE_PT} 0 0 {PAGE_PT} 0 0 cm /Im0 Do Q\n");

    let mut buf = Vec::<u8>::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::new();
    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<</Type /Catalog /Pages 2 0 R>>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<</Type /Pages /Kids [6 0 R 3 0 R] /Count 2>>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        format!(
            "3 0 obj\n<</Type /Page /MediaBox [0 0 {PAGE_PT} {PAGE_PT}] /Parent 2 0 R /Contents 4 0 R \
             /Resources <</XObject <</Im0 5 0 R>> >> >>\nendobj\n"
        )
        .as_bytes(),
    );
    offsets.push(buf.len());
    buf.extend_from_slice(
        format!(
            "4 0 obj\n<</Length {}>>\nstream\n{content}\nendstream\nendobj\n",
            content.len() + 1
        )
        .as_bytes(),
    );
    offsets.push(buf.len());
    buf.extend_from_slice(
        format!(
            "5 0 obj\n<</Type /XObject /Subtype /Image /Width {RASTER_WIDTH_PX} /Height {RASTER_HEIGHT_PX} \
             /ColorSpace /DeviceGray /BitsPerComponent 8 /Length {}>>\nstream\n",
            raster.len()
        )
        .as_bytes(),
    );
    buf.extend_from_slice(&raster);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        format!("6 0 obj\n<</Type /Page /MediaBox [0 0 {PAGE_PT} {PAGE_PT}] /Parent 2 0 R>>\nendobj\n").as_bytes(),
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

/// `force_ocr_pages` on the page with the raster, with or without one Tesseract pipeline stage.
fn per_page_config(with_pipeline: bool) -> ExtractionConfig {
    let pipeline = with_pipeline.then(|| OcrPipelineConfig {
        stages: vec![OcrPipelineStage {
            backend: "tesseract".to_string(),
            priority: 100,
            language: None,
            tesseract_config: None,
            paddle_ocr_config: None,
            paddle_ocr_settings: None,
            vlm_config: None,
            backend_options: None,
        }],
        quality_thresholds: Default::default(),
    });
    ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            pipeline,
            ..Default::default()
        }),
        force_ocr_pages: Some(vec![2]),
        ocr_embedded_images: Some(false),
        use_cache: false,
        ..Default::default()
    }
}

/// Extract the fixture with a stub backend that reads the word from the embedded raster.
fn extract(render: RenderOutcome, config: &ExtractionConfig) -> ExtractedDocument {
    extract_with("tesseract", render, RasterOutcome::Word, config)
}

/// Extract the fixture with the stub backend registered as `name`.
fn extract_with(
    name: &'static str,
    render: RenderOutcome,
    raster: RasterOutcome,
    config: &ExtractionConfig,
) -> ExtractedDocument {
    let pdf = pdf_with_image_on_page_two();
    let _ = unregister_ocr_backend(name);
    register_ocr_backend(Arc::new(StubBackend { name, render, raster })).expect("the stub backend registers");
    let result = extract_bytes_document_blocking(&pdf, "application/pdf", config);
    unregister_ocr_backend(name).expect("the stub backend unregisters");
    result.expect("the retry must recover the page, so extraction must not error")
}

fn assert_recovered(result: &ExtractedDocument, expected_warning: &str) {
    assert!(
        result.content.contains(RECOVERED_WORD),
        "the page's text must come from its embedded image; content: {:?}",
        result.content
    );
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains(expected_warning)),
        "the retry must be reported; warnings: {:?}",
        result.processing_warnings
    );
}

/// The one record a failed and recovered page 2 gives.
fn recovered_page_two() -> Vec<OcrPageFailure> {
    vec![OcrPageFailure {
        page: 2,
        error: "OCR error: stub render failure".to_string(),
        recovered: true,
    }]
}

#[test]
#[serial_test::serial]
fn a_blank_page_gives_no_record_without_a_pipeline() {
    let result = extract(RenderOutcome::Blank, &per_page_config(false));
    assert_recovered(&result, RETRY_WARNING);
    assert_eq!(result.ocr_page_failures, None, "a blank page is not a failed page");
}

#[test]
#[serial_test::serial]
fn a_blank_page_gives_no_record_with_a_pipeline() {
    let result = extract(RenderOutcome::Blank, &per_page_config(true));
    assert_recovered(&result, RETRY_WARNING);
    assert_eq!(result.ocr_page_failures, None, "a blank page is not a failed page");
}

#[test]
#[serial_test::serial]
fn a_failed_and_recovered_page_gives_one_record_without_a_pipeline() {
    let result = extract(RenderOutcome::Fails, &per_page_config(false));
    assert_recovered(&result, FAILED_AND_RECOVERED_WARNING);
    assert_eq!(result.ocr_page_failures, Some(recovered_page_two()));
}

#[test]
#[serial_test::serial]
fn a_failed_and_recovered_page_gives_one_record_with_a_pipeline() {
    let result = extract(RenderOutcome::Fails, &per_page_config(true));
    assert_recovered(&result, FAILED_AND_RECOVERED_WARNING);
    assert_eq!(result.ocr_page_failures, Some(recovered_page_two()));
}

/// A failed page whose embedded image gives a table and no text is recovered content: the
/// warning says "content", and the record still reports the page as recovered.
#[test]
#[serial_test::serial]
fn a_failed_page_recovered_as_a_table_is_recorded_as_recovered_content() {
    let result = extract_with(
        "tesseract",
        RenderOutcome::Fails,
        RasterOutcome::TableOnly,
        &per_page_config(false),
    );

    assert!(
        !result.content.contains(RECOVERED_WORD),
        "the stub read no word from the raster; content: {:?}",
        result.content
    );
    assert_eq!(
        result
            .processing_warnings
            .iter()
            .filter(|warning| warning.message
                == "OCR of page 2 failed (OCR error: stub render failure); its content was recovered from the page's \
                    embedded image XObjects instead.")
            .count(),
        1,
        "warnings: {:?}",
        result.processing_warnings
    );
    assert_eq!(result.ocr_page_failures, Some(recovered_page_two()));
}

/// A backend with any other name than `tesseract` gets no synthesized pipeline, so the per-page
/// route calls it directly. These two tests are the only ones that reach that branch with a
/// failed and recovered page.
const DIRECT_BACKEND: &str = "ocr-page-failures-direct-backend";

fn direct_backend_config() -> ExtractionConfig {
    let mut config = per_page_config(false);
    config.ocr.as_mut().expect("the config has an OCR block").backend = DIRECT_BACKEND.to_string();
    config
}

#[test]
#[serial_test::serial]
fn a_directly_called_backend_records_the_page_recovered_as_text() {
    let result = extract_with(
        DIRECT_BACKEND,
        RenderOutcome::Fails,
        RasterOutcome::Word,
        &direct_backend_config(),
    );
    assert_recovered(&result, FAILED_AND_RECOVERED_WARNING);
    assert_eq!(result.ocr_page_failures, Some(recovered_page_two()));
}

#[test]
#[serial_test::serial]
fn a_directly_called_backend_records_the_page_recovered_as_a_table() {
    let result = extract_with(
        DIRECT_BACKEND,
        RenderOutcome::Fails,
        RasterOutcome::TableOnly,
        &direct_backend_config(),
    );
    assert_eq!(
        result
            .processing_warnings
            .iter()
            .filter(|warning| warning.message
                == "OCR of page 2 failed (OCR error: stub render failure); its content was recovered from the page's \
                    embedded image XObjects instead.")
            .count(),
        1,
        "warnings: {:?}",
        result.processing_warnings
    );
    assert_eq!(result.ocr_page_failures, Some(recovered_page_two()));
}

/// The whole-document route (`force_ocr`) records every failed page and its recovery state.
#[test]
#[serial_test::serial]
fn a_whole_document_run_records_every_failed_page() {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            ..Default::default()
        }),
        force_ocr: true,
        ocr_embedded_images: Some(false),
        use_cache: false,
        ..Default::default()
    };
    let result = extract(RenderOutcome::Fails, &config);
    assert_recovered(&result, FAILED_AND_RECOVERED_WARNING);
    assert_eq!(
        result.ocr_page_failures,
        Some(vec![
            OcrPageFailure {
                page: 1,
                error: "OCR error: stub render failure".to_string(),
                recovered: false,
            },
            OcrPageFailure {
                page: 2,
                error: "OCR error: stub render failure".to_string(),
                recovered: true,
            },
        ])
    );
}

/// What a backend of the tests below does with one image.
#[derive(Clone, Copy)]
enum Read {
    Fails,
    Blank,
    Word,
}

/// A backend with one outcome for the embedded raster and one for every other image.
struct TwoOutcomeBackend {
    name: &'static str,
    page: Read,
    raster: Read,
}

impl Plugin for TwoOutcomeBackend {
    fn name(&self) -> &str {
        self.name
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
impl OcrBackend for TwoOutcomeBackend {
    async fn process_image(&self, image_bytes: &[u8], _config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        let read = if png_size(image_bytes) == Some((RASTER_WIDTH_PX, RASTER_HEIGHT_PX)) {
            self.raster
        } else {
            self.page
        };
        match read {
            Read::Fails => Err(xberg::XbergError::Ocr {
                message: format!("{} is down", self.name),
                source: None,
            }),
            Read::Blank => Ok(ExtractedDocument::default()),
            Read::Word => {
                let mut read = ExtractedDocument::default();
                read.content = RECOVERED_WORD.to_string();
                Ok(read)
            }
        }
    }
    fn supports_language(&self, _language: &str) -> bool {
        true
    }
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

/// Extract `content` with `backends` registered for the time of the call.
fn extract_with_backends(
    content: &[u8],
    mime_type: &str,
    backends: Vec<TwoOutcomeBackend>,
    config: &ExtractionConfig,
) -> xberg::Result<ExtractedDocument> {
    let names: Vec<&'static str> = backends.iter().map(|backend| backend.name).collect();
    for backend in backends {
        let _ = unregister_ocr_backend(backend.name);
        register_ocr_backend(Arc::new(backend)).expect("the backend registers");
    }
    let result = extract_bytes_document_blocking(content, mime_type, config);
    for name in names {
        unregister_ocr_backend(name).expect("the backend unregisters");
    }
    result
}

/// A three-page PDF: page 1 has a text layer of `words` words, pages 2 and 3 each paint the
/// grey raster over the page and have no text.
fn pdf_with_text_cover_and_two_scans(words: usize) -> Vec<u8> {
    let raster = vec![0xA0u8; (RASTER_WIDTH_PX * RASTER_HEIGHT_PX) as usize];
    let scan = format!("q {PAGE_PT} 0 0 {PAGE_PT} 0 0 cm /Im0 Do Q\n");
    let tokens: Vec<String> = (0..words).map(|index| format!("word{index}")).collect();
    let mut cover = String::from("BT /F1 6 Tf 8 TL 10 180 Td\n");
    for line in tokens.chunks(5) {
        cover.push_str(&format!("({}) Tj T*\n", line.join(" ")));
    }
    cover.push_str("ET\n");

    let mut objects: Vec<Vec<u8>> = vec![
        b"<</Type /Catalog /Pages 2 0 R>>".to_vec(),
        b"<</Type /Pages /Kids [3 0 R 6 0 R 8 0 R] /Count 3>>".to_vec(),
        format!(
            "<</Type /Page /MediaBox [0 0 {PAGE_PT} {PAGE_PT}] /Parent 2 0 R /Contents 4 0 R \
             /Resources <</Font <</F1 5 0 R>> >> >>"
        )
        .into_bytes(),
        format!("<</Length {}>>\nstream\n{cover}endstream", cover.len()).into_bytes(),
        b"<</Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding>>".to_vec(),
    ];
    for contents_object in [7, 9] {
        objects.push(
            format!(
                "<</Type /Page /MediaBox [0 0 {PAGE_PT} {PAGE_PT}] /Parent 2 0 R /Contents {contents_object} 0 R \
                 /Resources <</XObject <</Im0 10 0 R>> >> >>"
            )
            .into_bytes(),
        );
        objects.push(format!("<</Length {}>>\nstream\n{scan}endstream", scan.len()).into_bytes());
    }
    let mut image = format!(
        "<</Type /XObject /Subtype /Image /Width {RASTER_WIDTH_PX} /Height {RASTER_HEIGHT_PX} \
         /ColorSpace /DeviceGray /BitsPerComponent 8 /Length {}>>\nstream\n",
        raster.len()
    )
    .into_bytes();
    image.extend_from_slice(&raster);
    image.extend_from_slice(b"\nendstream");
    objects.push(image);

    let mut buf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(buf.len());
        buf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        buf.extend_from_slice(object);
        buf.extend_from_slice(b"\nendobj\n");
    }
    let xref_offset = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
    for offset in &offsets {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<</Size {} /Root 1 0 R>>\n", offsets.len() + 1).as_bytes());
    buf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());
    buf
}

const DOWN_BACKEND: &str = "ocr-page-failures-down-backend";
const DOWN_ERROR: &str = "OCR error: ocr-page-failures-down-backend is down";

/// Default settings (`ocr_strategy` is `auto`) and a backend that fails on every image.
fn extract_with_a_backend_that_is_down(words_on_the_cover: usize) -> ExtractedDocument {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: DOWN_BACKEND.to_string(),
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };
    extract_with_backends(
        &pdf_with_text_cover_and_two_scans(words_on_the_cover),
        "application/pdf",
        vec![TwoOutcomeBackend {
            name: DOWN_BACKEND,
            page: Read::Fails,
            raster: Read::Fails,
        }],
        &config,
    )
    .expect("the native text of the cover page is kept, so extraction must not error")
}

fn failed_page(page: u32, error: &str, recovered: bool) -> OcrPageFailure {
    OcrPageFailure {
        page,
        error: error.to_string(),
        recovered,
    }
}

/// A cover page with three words makes the whole document poor text, so OCR runs on all of it
/// in one pass. The pass fails, and the native text is kept. Only the two scanned pages get a
/// record, as they do with a longer cover page: the cover page has its native text.
#[test]
#[serial_test::serial]
fn a_failed_whole_document_fallback_records_the_scanned_pages_only() {
    let result = extract_with_a_backend_that_is_down(3);

    assert!(
        result.content.contains("word0"),
        "the native text of the cover page is kept; content: {:?}",
        result.content
    );
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("OCR fallback failed")),
        "the fallback for the whole document must be the route that failed; warnings: {:?}",
        result.processing_warnings
    );
    assert_eq!(
        result.ocr_page_failures,
        Some(vec![
            failed_page(2, DOWN_ERROR, false),
            failed_page(3, DOWN_ERROR, false)
        ])
    );
}

/// A cover page with twenty words is good text, so OCR runs on the two scanned pages only.
#[test]
#[serial_test::serial]
fn a_longer_cover_page_gives_the_same_scanned_page_records() {
    let result = extract_with_a_backend_that_is_down(20);

    assert!(
        result.content.contains("word19"),
        "the native text of the cover page is kept; content: {:?}",
        result.content
    );
    let failures = result.ocr_page_failures.expect("the scanned pages failed");
    assert_eq!(
        failures
            .iter()
            .map(|failure| (failure.page, failure.recovered))
            .collect::<Vec<_>>(),
        vec![(2, false), (3, false)]
    );
}

const FIRST_STAGE: &str = "ocr-page-failures-first-stage";
const SECOND_STAGE: &str = "ocr-page-failures-second-stage";
const FIRST_STAGE_ERROR: &str = "OCR error: ocr-page-failures-first-stage is down";

fn two_stage_pipeline() -> OcrPipelineConfig {
    let stage = |backend: &str, priority: u32| OcrPipelineStage {
        backend: backend.to_string(),
        priority,
        language: None,
        tesseract_config: None,
        paddle_ocr_config: None,
        paddle_ocr_settings: None,
        vlm_config: None,
        backend_options: None,
    };
    OcrPipelineConfig {
        stages: vec![stage(FIRST_STAGE, 100), stage(SECOND_STAGE, 50)],
        quality_thresholds: Default::default(),
    }
}

/// The first stage fails on every image. The second stage reads nothing from a page render and
/// reads the word from the embedded raster.
fn two_stage_backends() -> Vec<TwoOutcomeBackend> {
    vec![
        TwoOutcomeBackend {
            name: FIRST_STAGE,
            page: Read::Fails,
            raster: Read::Fails,
        },
        TwoOutcomeBackend {
            name: SECOND_STAGE,
            page: Read::Blank,
            raster: Read::Word,
        },
    ]
}

/// Both pages of the two-page fixture go through the two stages. Page 1 has no raster, so no
/// stage reads it. The second stage reads page 2 from its raster.
fn extract_two_pages_with_two_stages() -> ExtractedDocument {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: FIRST_STAGE.to_string(),
            pipeline: Some(two_stage_pipeline()),
            ..Default::default()
        }),
        force_ocr_pages: Some(vec![1, 2]),
        ocr_embedded_images: Some(false),
        use_cache: false,
        ..Default::default()
    };
    extract_with_backends(
        &pdf_with_image_on_page_two(),
        "application/pdf",
        two_stage_backends(),
        &config,
    )
    .expect("the second stage reads page 2, so extraction must not error")
}

#[test]
#[serial_test::serial]
fn a_page_read_by_a_later_stage_gives_no_record() {
    let result = extract_two_pages_with_two_stages();

    assert!(
        result.content.contains(RECOVERED_WORD),
        "the second stage read page 2; content: {:?}",
        result.content
    );
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains(FIRST_STAGE) && warning.message.contains("failed and was skipped")),
        "the failed stage stays in the warnings; warnings: {:?}",
        result.processing_warnings
    );
    let failures = result.ocr_page_failures.expect("page 1 was read by no stage");
    assert!(
        failures.iter().all(|failure| failure.page != 2),
        "page 2 has text from the second stage; records: {failures:?}"
    );
}

#[test]
#[serial_test::serial]
fn a_page_no_stage_read_keeps_the_first_failure() {
    let result = extract_two_pages_with_two_stages();

    assert_eq!(
        result.ocr_page_failures,
        Some(vec![failed_page(1, FIRST_STAGE_ERROR, false)])
    );
}

/// The selected pages of the document with the twenty-word cover page go through the two stages.
/// The first stage fails on every image and the second stage reads nothing, so no stage reads a
/// selected page.
fn extract_selected_pages_that_no_stage_reads(pages: Vec<u32>) -> ExtractedDocument {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: FIRST_STAGE.to_string(),
            pipeline: Some(two_stage_pipeline()),
            ..Default::default()
        }),
        force_ocr_pages: Some(pages),
        ocr_embedded_images: Some(false),
        use_cache: false,
        ..Default::default()
    };
    let backends = vec![
        TwoOutcomeBackend {
            name: FIRST_STAGE,
            page: Read::Fails,
            raster: Read::Fails,
        },
        TwoOutcomeBackend {
            name: SECOND_STAGE,
            page: Read::Blank,
            raster: Read::Blank,
        },
    ];
    let result = extract_with_backends(
        &pdf_with_text_cover_and_two_scans(20),
        "application/pdf",
        backends,
        &config,
    )
    .expect("the second stage returns a blank page, so extraction must not error");
    assert!(
        result.content.contains("word19"),
        "the native text of the cover page is kept; content: {:?}",
        result.content
    );
    result
}

/// The cover page is the selected page. It keeps its native text, so its record is recovered.
#[test]
#[serial_test::serial]
fn a_selected_page_that_keeps_its_native_text_is_recorded_as_recovered() {
    let result = extract_selected_pages_that_no_stage_reads(vec![1]);

    assert_eq!(
        result.ocr_page_failures,
        Some(vec![failed_page(1, FIRST_STAGE_ERROR, true)])
    );
}

/// The cover page and one scanned page are selected. The scanned page has no native text, so
/// only the record of the cover page is recovered.
#[test]
#[serial_test::serial]
fn a_selected_page_without_native_text_is_recorded_as_not_recovered() {
    let result = extract_selected_pages_that_no_stage_reads(vec![1, 2]);

    assert_eq!(
        result.ocr_page_failures,
        Some(vec![
            failed_page(1, FIRST_STAGE_ERROR, true),
            failed_page(2, FIRST_STAGE_ERROR, false)
        ])
    );
}

/// A pipeline of one stage reads the two-page fixture as one document. The stage fails on both
/// page renders and reads the word from the raster of page 2, so its result has a record for
/// each page. Page 1 stays blank, and its record is in the result one time.
#[test]
#[serial_test::serial]
fn a_page_the_returned_stage_failed_on_has_one_record() {
    let mut pipeline = two_stage_pipeline();
    pipeline.stages.truncate(1);
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: FIRST_STAGE.to_string(),
            pipeline: Some(pipeline),
            ..Default::default()
        }),
        force_ocr: true,
        ocr_embedded_images: Some(false),
        use_cache: false,
        ..Default::default()
    };

    let result = extract_with_backends(
        &pdf_with_image_on_page_two(),
        "application/pdf",
        vec![TwoOutcomeBackend {
            name: FIRST_STAGE,
            page: Read::Fails,
            raster: Read::Word,
        }],
        &config,
    )
    .expect("the stage reads page 2 from its raster, so extraction must not error");

    assert_eq!(
        result.ocr_page_failures,
        Some(vec![
            failed_page(1, FIRST_STAGE_ERROR, false),
            failed_page(2, FIRST_STAGE_ERROR, true)
        ])
    );
}

/// An image file goes through the two stages as one page. No stage reads it.
#[test]
#[serial_test::serial]
fn an_image_file_keeps_the_record_of_a_failed_stage() {
    let mut png = Vec::new();
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(64, 48, image::Rgb([255, 255, 255])))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("the image encodes");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: FIRST_STAGE.to_string(),
            pipeline: Some(two_stage_pipeline()),
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let result = extract_with_backends(&png, "image/png", two_stage_backends(), &config)
        .expect("the second stage returns a blank page, so extraction must not error");

    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains(FIRST_STAGE) && warning.message.contains("failed and was skipped")),
        "the pipeline ran and its first stage failed; warnings: {:?}",
        result.processing_warnings
    );
    assert_eq!(
        result.ocr_page_failures,
        Some(vec![failed_page(1, FIRST_STAGE_ERROR, false)])
    );
}

const CANCELLING_STAGE: &str = "ocr-page-failures-cancelling-stage";
/// The start of the warning of a whole-document fallback whose pipeline stages all failed.
const CANCELLED_FALLBACK_WARNING: &str = "OCR fallback failed (Parsing error: All OCR pipeline backends failed";

/// A backend that cancels the extraction on its first call and then fails.
struct CancellingBackend {
    token: xberg::cancellation::CancellationToken,
}

impl Plugin for CancellingBackend {
    fn name(&self) -> &str {
        CANCELLING_STAGE
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
impl OcrBackend for CancellingBackend {
    async fn process_image(&self, _image_bytes: &[u8], _config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        self.token.cancel();
        Err(xberg::XbergError::Ocr {
            message: "the run was cancelled".to_string(),
            source: None,
        })
    }
    fn supports_language(&self, _language: &str) -> bool {
        true
    }
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

/// The first stage of a two-stage pipeline cancels the extraction during the fallback for the
/// whole document. Each stage then ends with the cancellation, and the pipeline reports that
/// every stage failed. The native text of the cover page is kept, as it is for a failed run.
#[test]
#[serial_test::serial]
fn a_cancelled_whole_document_fallback_adds_no_record() {
    let token = xberg::cancellation::CancellationToken::new();
    let mut pipeline = two_stage_pipeline();
    pipeline.stages[0].backend = CANCELLING_STAGE.to_string();
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: CANCELLING_STAGE.to_string(),
            pipeline: Some(pipeline),
            ..Default::default()
        }),
        cancel_token: Some(token.clone()),
        use_cache: false,
        ..Default::default()
    };
    let _ = unregister_ocr_backend(CANCELLING_STAGE);
    register_ocr_backend(Arc::new(CancellingBackend { token: token.clone() })).expect("the backend registers");

    let result = extract_with_backends(
        &pdf_with_text_cover_and_two_scans(3),
        "application/pdf",
        vec![TwoOutcomeBackend {
            name: SECOND_STAGE,
            page: Read::Blank,
            raster: Read::Blank,
        }],
        &config,
    );
    unregister_ocr_backend(CANCELLING_STAGE).expect("the backend unregisters");
    let result = result.expect("the native text of the cover page is kept");

    assert!(token.is_cancelled(), "the first stage ran and cancelled the extraction");
    assert!(
        result.content.contains("word0"),
        "the native text of the cover page is kept; content: {:?}",
        result.content
    );
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains(CANCELLED_FALLBACK_WARNING)),
        "the fallback for the whole document ended with the failure of every stage; warnings: {:?}",
        result.processing_warnings
    );
    assert_eq!(result.ocr_page_failures, None);
}
