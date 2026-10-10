//! Unit tests for the page records of an OCR run that failed and of a pipeline stage whose
//! result is not returned. A sibling module of `tests.rs`, registered the same way in `mod.rs`.

use super::pipeline::{
    FailedOcrRun, OcrRunResult, attach_unread_page_failures, failed_run_page_failures, run_ocr_pipeline,
};
use super::tests::tests::build_minimal_two_page_pdf_with_sizes;
use crate::core::config::{ExtractionConfig, OcrQualityThresholds};

fn page_record(page: u32, error: &str, recovered: bool) -> crate::types::OcrPageFailure {
    crate::types::OcrPageFailure {
        page,
        error: error.to_string(),
        recovered,
    }
}

/// Native text "Cover" on page 1 and no text on pages 2 and 3.
fn cover_page_boundaries() -> (&'static str, Vec<crate::types::PageBoundary>) {
    let boundary = |page_number: u32, byte_start: usize, byte_end: usize| crate::types::PageBoundary {
        byte_start,
        byte_end,
        page_number,
    };
    ("Cover", vec![boundary(1, 0, 5), boundary(2, 5, 5), boundary(3, 5, 5)])
}

/// The run has no record of its own. Page 1 has native text, so only pages 2 and 3 get a record.
#[test]
fn a_failed_run_without_page_records_records_the_pages_without_native_text() {
    let (native_text, boundaries) = cover_page_boundaries();
    let run = FailedOcrRun::from(crate::XbergError::Ocr {
        message: "render failed".to_string(),
        source: None,
    });

    assert_eq!(
        failed_run_page_failures(&run, 3, native_text, &boundaries),
        vec![
            page_record(2, "OCR error: render failed", false),
            page_record(3, "OCR error: render failed", false),
        ]
    );
}

/// The run has a record for each page. The record of page 1 goes, because page 1 has native
/// text. The record of page 2 is not recovered: the output of the run is not in the result.
#[test]
fn a_failed_run_keeps_its_own_records_only_for_the_pages_without_native_text() {
    let (native_text, boundaries) = cover_page_boundaries();
    let run = FailedOcrRun {
        error: crate::XbergError::Ocr {
            message: "every page failed".to_string(),
            source: None,
        },
        page_failures: vec![
            page_record(3, "third page error", false),
            page_record(1, "first page error", false),
            page_record(2, "second page error", true),
        ],
    };

    assert_eq!(
        failed_run_page_failures(&run, 3, native_text, &boundaries),
        vec![
            page_record(2, "second page error", false),
            page_record(3, "third page error", false),
        ],
        "the run's own records stay for pages 2 and 3, in page order"
    );
}

/// Without page boundaries the pages that have native text are not known.
#[test]
fn a_failed_run_without_page_boundaries_gives_no_record() {
    let run = FailedOcrRun {
        error: crate::XbergError::Ocr {
            message: "every page failed".to_string(),
            source: None,
        },
        page_failures: vec![page_record(2, "second page error", false)],
    };

    assert!(
        failed_run_page_failures(&run, 3, "Cover", &[]).is_empty(),
        "no page can be named without page boundaries"
    );
}

/// Pages 5 and 6 of a document, run as one unit. The returned result has text for page 5 only.
#[test]
fn a_stage_record_is_kept_as_not_recovered_only_for_a_page_without_text() {
    let page_texts = vec!["read by the returned stage".to_string(), String::new()];

    let document = attach_unread_page_failures(
        None,
        "read by the returned stage",
        &page_texts,
        4,
        vec![
            page_record(5, "fifth page error", true),
            page_record(6, "sixth page error", true),
        ],
    )
    .expect("a kept record needs a document");

    assert_eq!(
        document.ocr_page_failures,
        vec![page_record(6, "sixth page error", false)],
        "the output of the stage that recovered page 6 is not in the result"
    );
}

/// Pages 5, 6 and 7 of a document. The returned result has text for page 5 and its own record
/// for page 7.
#[test]
fn a_kept_stage_record_joins_the_records_of_the_returned_document_in_page_order() {
    let page_texts = vec!["read by the returned stage".to_string(), String::new(), String::new()];
    let mut returned = crate::types::internal::InternalDocument::new("pdf");
    returned
        .ocr_page_failures
        .push(page_record(7, "seventh page error", false));

    let document = attach_unread_page_failures(
        Some(returned),
        "read by the returned stage",
        &page_texts,
        4,
        vec![page_record(6, "sixth page error", false)],
    )
    .expect("the returned document stays");

    assert_eq!(
        document.ocr_page_failures,
        vec![
            page_record(6, "sixth page error", false),
            page_record(7, "seventh page error", false),
        ]
    );
}

/// The returned result has a record for page 6, and so has a stage. Page 6 gets one record,
/// with the error of the returned result.
#[test]
fn a_page_with_a_record_in_the_returned_document_keeps_one_record() {
    let page_texts = vec!["read by the returned stage".to_string(), String::new()];
    let mut returned = crate::types::internal::InternalDocument::new("pdf");
    returned
        .ocr_page_failures
        .push(page_record(6, "returned stage error", false));

    let document = attach_unread_page_failures(
        Some(returned),
        "read by the returned stage",
        &page_texts,
        4,
        vec![page_record(6, "earlier stage error", false)],
    )
    .expect("the returned document stays");

    assert_eq!(
        document.ocr_page_failures,
        vec![page_record(6, "returned stage error", false)]
    );
}

/// A backend that reads a document in one call returns one text for all of its pages, so
/// page 3 has no page text of its own. The whole text decides: it is not blank, so the stage
/// record of page 3 goes.
#[test]
fn a_stage_record_for_a_page_without_its_own_text_goes_when_the_whole_text_is_not_blank() {
    let whole_text = "read by the returned stage";

    let document = attach_unread_page_failures(
        None,
        whole_text,
        &[whole_text.to_string()],
        0,
        vec![page_record(3, "third page error", false)],
    );

    assert!(
        document.is_none(),
        "no record is kept, so no document is made; records: {:?}",
        document.map(|document| document.ocr_page_failures)
    );
}

/// The same result with a blank whole text: the stage record of page 3 stays.
#[test]
fn a_stage_record_for_a_page_without_its_own_text_stays_when_the_whole_text_is_blank() {
    let document = attach_unread_page_failures(
        None,
        " \n",
        &[String::new()],
        0,
        vec![page_record(3, "third page error", true)],
    )
    .expect("a kept record needs a document");

    assert_eq!(
        document.ocr_page_failures,
        vec![page_record(3, "third page error", false)]
    );
}

/// A backend that tells the two pages of `build_minimal_two_page_pdf_with_sizes` apart by
/// their shape. `None` fails the page; `Some` returns that text.
struct PageShapeBackend {
    name: &'static str,
    square_page: Option<&'static str>,
    other_page: Option<&'static str>,
}

#[async_trait::async_trait]
impl crate::plugins::OcrBackend for PageShapeBackend {
    fn backend_type(&self) -> crate::plugins::OcrBackendType {
        crate::plugins::OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(
        &self,
        data: &[u8],
        _: &crate::core::config::OcrConfig,
    ) -> crate::Result<crate::types::ExtractedDocument> {
        let image = image::load_from_memory(data).map_err(|error| crate::XbergError::Ocr {
            message: format!("test backend could not decode page: {error}"),
            source: None,
        })?;
        let outcome = if image.width() == image.height() {
            self.square_page
        } else {
            self.other_page
        };
        match outcome {
            Some(text) => Ok(crate::types::ExtractedDocument {
                content: text.to_string(),
                ..Default::default()
            }),
            None => Err(crate::XbergError::Ocr {
                message: format!("{} failure", self.name),
                source: None,
            }),
        }
    }
}

impl crate::plugins::Plugin for PageShapeBackend {
    fn name(&self) -> &str {
        self.name
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

/// The configuration of a two-stage pipeline, and the pipeline. The first backend has the
/// higher priority.
fn two_stage_config(
    first_name: &str,
    second_name: &str,
    pipeline_min_quality: f64,
) -> (ExtractionConfig, crate::core::config::OcrPipelineConfig) {
    use crate::core::config::{OcrConfig, OcrPipelineConfig, OcrPipelineStage};

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
    let pipeline = OcrPipelineConfig {
        stages: vec![stage(first_name, 100), stage(second_name, 50)],
        quality_thresholds: OcrQualityThresholds {
            pipeline_min_quality,
            ..Default::default()
        },
    };
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: first_name.to_string(),
            pipeline: Some(pipeline.clone()),
            ..Default::default()
        }),
        ..Default::default()
    };
    (config, pipeline)
}

/// Run a two-stage pipeline over a PDF whose page 2 is square. Returns the document of the run.
async fn run_two_stage_pipeline(
    first: PageShapeBackend,
    second: PageShapeBackend,
    pipeline_min_quality: f64,
) -> OcrRunResult<Option<crate::types::internal::InternalDocument>> {
    let (first_name, second_name) = (first.name, second.name);
    let (config, pipeline) = two_stage_config(first_name, second_name, pipeline_min_quality);
    crate::plugins::register_ocr_backend(std::sync::Arc::new(first)).unwrap();
    crate::plugins::register_ocr_backend(std::sync::Arc::new(second)).unwrap();
    let pdf = build_minimal_two_page_pdf_with_sizes((612.0, 792.0), (504.0, 504.0));

    let result = run_ocr_pipeline(
        Some(&pdf),
        None,
        #[cfg(feature = "layout-detection")]
        None,
        &config,
        &pipeline,
        None,
        None,
    )
    .await;

    crate::plugins::unregister_ocr_backend(first_name).unwrap();
    crate::plugins::unregister_ocr_backend(second_name).unwrap();
    result.map(|output| output.3)
}

const READABLE_PAGE_TEXT: &str = "Successful OCR text with enough readable words for the primary page.";

/// The first stage fails on page 2 and reads little on page 1. The second stage reads page 1
/// and returns page 2 blank, and its result is the one returned.
#[tokio::test]
#[serial_test::serial]
async fn an_outscored_stage_keeps_its_failed_page() {
    let document = run_two_stage_pipeline(
        PageShapeBackend {
            name: "outscored-first-stage",
            square_page: None,
            other_page: Some("zz"),
        },
        PageShapeBackend {
            name: "outscored-second-stage",
            square_page: Some(""),
            other_page: Some(READABLE_PAGE_TEXT),
        },
        2.0,
    )
    .await
    .expect("the second stage read page 1")
    .expect("a kept record needs a document");

    assert_eq!(
        document.ocr_page_failures,
        vec![page_record(2, "OCR error: outscored-first-stage failure", false)]
    );
}

/// The first stage fails on both pages. The second stage is accepted: it reads page 1 and
/// returns page 2 blank.
#[tokio::test]
#[serial_test::serial]
async fn an_accepted_stage_keeps_only_the_failure_of_the_page_it_left_blank() {
    let document = run_two_stage_pipeline(
        PageShapeBackend {
            name: "accepted-first-stage",
            square_page: None,
            other_page: None,
        },
        PageShapeBackend {
            name: "accepted-second-stage",
            square_page: Some(""),
            other_page: Some(READABLE_PAGE_TEXT),
        },
        0.0,
    )
    .await
    .expect("the second stage read page 1")
    .expect("a kept record needs a document");

    assert!(
        document.processing_warnings.iter().any(|warning| warning
            .message
            .contains("'accepted-first-stage' failed and was skipped")),
        "the failed stage stays in the warnings: {:?}",
        document.processing_warnings
    );
    assert_eq!(
        document.ocr_page_failures,
        vec![page_record(2, "OCR error: accepted-first-stage failure", false)],
        "page 1 was read by the second stage, so only page 2 keeps its record"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn a_pipeline_whose_stages_all_fail_carries_every_page_record() {
    let failed_run = run_two_stage_pipeline(
        PageShapeBackend {
            name: "all-fail-first-stage",
            square_page: None,
            other_page: None,
        },
        PageShapeBackend {
            name: "all-fail-second-stage",
            square_page: None,
            other_page: None,
        },
        0.0,
    )
    .await
    .expect_err("no stage read a page");

    assert!(
        failed_run
            .error
            .to_string()
            .contains("All OCR pipeline backends failed"),
        "error: {}",
        failed_run.error
    );
    assert_eq!(
        failed_run.page_failures,
        vec![
            page_record(1, "OCR error: all-fail-first-stage failure", false),
            page_record(2, "OCR error: all-fail-first-stage failure", false),
        ]
    );
}

/// The run of a whole document that goes one page at a time. Both stages fail on both pages.
/// The pipeline names its own backends, as
/// `mixed_ocr_later_page_pipeline_route_does_not_leak_page_one_marker` does: a test that
/// runs at the same time and asks for a built-in backend cannot get one of these. ~keep
#[cfg(paddle_ocr)]
#[tokio::test]
#[serial_test::serial]
async fn a_failed_page_by_page_pipeline_run_carries_every_page_record() {
    let (first_name, second_name) = ("page-by-page-first-stage", "page-by-page-second-stage");
    let (config, _) = two_stage_config(first_name, second_name, 0.0);
    for name in [first_name, second_name] {
        crate::plugins::register_ocr_backend(std::sync::Arc::new(PageShapeBackend {
            name,
            square_page: None,
            other_page: None,
        }))
        .unwrap();
    }

    let pdf = build_minimal_two_page_pdf_with_sizes((612.0, 792.0), (504.0, 504.0));
    let result = Box::pin(super::pipeline::extract_full_document_ocr_pipeline_per_page(
        &pdf,
        &ExtractionConfig {
            force_ocr: true,
            ..config
        },
        None,
        None,
        #[cfg(feature = "layout-detection")]
        None,
    ))
    .await;

    crate::plugins::unregister_ocr_backend(first_name).unwrap();
    crate::plugins::unregister_ocr_backend(second_name).unwrap();
    let failed_run = result.expect_err("every stage failed on every page");

    let mut pages: Vec<u32> = failed_run.page_failures.iter().map(|failure| failure.page).collect();
    pages.sort_unstable();
    assert_eq!(pages, vec![1, 2]);
    assert!(
        failed_run
            .page_failures
            .iter()
            .all(|failure| !failure.recovered && failure.error.contains("All OCR pipeline backends failed")),
        "records: {:?}",
        failed_run.page_failures
    );
}
