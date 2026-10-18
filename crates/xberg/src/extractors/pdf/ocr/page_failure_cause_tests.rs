//! #2149: a failed OCR page keeps the typed variant chain of its backend error, and none of the
//! source text. Drives a registered synthetic backend through the real PDF pipeline.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tracing_subscriber::layer::{Context, SubscriberExt as _};
use tracing_subscriber::{Layer, Registry};

use super::pipeline::{describe_page_failure, extract_with_ocr_with_page_hints};
use super::tests::tests::build_minimal_two_page_pdf_with_sizes;
use crate::core::config::{ExtractionConfig, OcrConfig};
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};

const PATH_SENTINEL: &str = "/Users/sentinel-home/PATH_SENTINEL/model.onnx";
const SECRET_SENTINEL: &str = "SECRET_SENTINEL_9f3a";

#[derive(Debug)]
enum SyntheticOcrError {
    Detect(SyntheticIoError),
}

#[derive(Debug)]
struct SyntheticIoError(String);

impl std::fmt::Display for SyntheticOcrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "detect stage failed at {PATH_SENTINEL}")
    }
}
impl std::fmt::Display for SyntheticIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "token {}", self.0)
    }
}
impl std::error::Error for SyntheticOcrError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Detect(inner) => Some(inner),
        }
    }
}
impl std::error::Error for SyntheticIoError {}

fn nested_error() -> crate::XbergError {
    crate::XbergError::Ocr {
        message: "Synthetic OCR operation failed".to_string(),
        source: Some(Box::new(SyntheticOcrError::Detect(SyntheticIoError(
            SECRET_SENTINEL.to_string(),
        )))),
    }
}

/// Fails every page, or only the square one when `fail_square_only`.
struct NestedFailureBackend {
    name: &'static str,
    fail_square_only: bool,
    calls: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl OcrBackend for NestedFailureBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, data: &[u8], _: &OcrConfig) -> crate::Result<crate::types::ExtractedDocument> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let image = image::load_from_memory(data).expect("synthetic page decodes");
        if self.fail_square_only && image.width() != image.height() {
            return Ok(crate::types::ExtractedDocument {
                content: "Readable synthetic page text with enough ordinary words to be kept.".to_string(),
                ..Default::default()
            });
        }
        Err(nested_error())
    }
}

impl Plugin for NestedFailureBackend {
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

#[derive(Clone, Default)]
struct EventCapture(Arc<Mutex<Vec<String>>>);

struct Flatten(String);
impl tracing::field::Visit for Flatten {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.push_str(&format!("{}={value:?} ", field.name()));
    }
}

impl<S: tracing::Subscriber> Layer<S> for EventCapture {
    fn on_event(&self, event: &tracing::Event<'_>, _: Context<'_, S>) {
        let mut flat = Flatten(String::new());
        event.record(&mut flat);
        self.0.lock().expect("capture lock").push(flat.0);
    }
}

type RunOutput = crate::Result<Option<crate::types::internal::InternalDocument>>;

async fn run(
    name: &'static str,
    fail_square_only: bool,
) -> (RunOutput, Vec<crate::types::OcrPageFailure>, usize, Vec<String>) {
    let calls = Arc::new(AtomicUsize::new(0));
    crate::plugins::register_ocr_backend(Arc::new(NestedFailureBackend {
        name,
        fail_square_only,
        calls: calls.clone(),
    }))
    .unwrap();
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: name.to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let pdf = build_minimal_two_page_pdf_with_sizes((612.0, 792.0), (504.0, 504.0));
    let capture = EventCapture::default();
    let subscriber = Registry::default().with(capture.clone());
    let _guard = tracing::subscriber::set_default(subscriber);
    let result = extract_with_ocr_with_page_hints(
        Some(&pdf),
        None,
        #[cfg(feature = "layout-detection")]
        None,
        &config,
        None,
        None,
    )
    .await;
    drop(_guard);
    crate::plugins::unregister_ocr_backend(name).unwrap();
    let events = capture.0.lock().unwrap().clone();
    match result {
        Ok(output) => {
            let document = output.4;
            let failures = document
                .as_ref()
                .map(|d| d.ocr_page_failures.clone())
                .unwrap_or_default();
            (Ok(document), failures, calls.load(Ordering::SeqCst), events)
        }
        Err(failed) => (
            Err(failed.error),
            failed.page_failures,
            calls.load(Ordering::SeqCst),
            events,
        ),
    }
}

fn assert_clean(label: &str, text: &str) {
    assert!(!text.contains(PATH_SENTINEL), "{label} leaks the path: {text}");
    assert!(!text.contains("PATH_SENTINEL"), "{label} leaks the path: {text}");
    assert!(!text.contains(SECRET_SENTINEL), "{label} leaks the secret: {text}");
}

const CHAIN: &str = "[cause chain: Detect > SyntheticIoError]";

#[test]
fn describe_keeps_variant_names_and_drops_source_text() {
    let described = describe_page_failure(&nested_error());
    assert!(described.contains(CHAIN), "{described}");
    assert_clean("description", &described);
    // Negative control: the full source chain does carry the sentinels, so the check above
    // can fail.
    let mut full = String::new();
    let mut current = std::error::Error::source(&nested_error()).map(|e| e.to_string());
    full.push_str(&current.take().unwrap_or_default());
    assert!(full.contains(PATH_SENTINEL));
    let plain = crate::XbergError::Ocr {
        message: "plain".into(),
        source: None,
    };
    assert_eq!(describe_page_failure(&plain), plain.to_string());
}

#[test]
fn trace_capture_control_sees_a_planted_sentinel() {
    let capture = EventCapture::default();
    let subscriber = Registry::default().with(capture.clone());
    tracing::subscriber::with_default(subscriber, || tracing::warn!(error = %SECRET_SENTINEL, "planted"));
    let events = capture.0.lock().unwrap().join("\n");
    assert!(events.contains(SECRET_SENTINEL), "{events}");
}

#[tokio::test]
#[serial_test::serial]
async fn all_pages_failing_keeps_the_cause_chain_everywhere() {
    let (result, failures, calls, events) = run("issue-2149-all-fail", false).await;
    assert_eq!(calls, 2, "one backend call per page");
    assert_eq!(failures.len(), 2, "{failures:?}");
    for failure in &failures {
        assert!(failure.error.contains(CHAIN), "{}", failure.error);
        assert_clean("failure record", &failure.error);
        assert!(!failure.recovered);
    }
    let error = result.expect_err("every page failed");
    let text = error.to_string();
    assert!(text.contains(CHAIN), "{text}");
    assert_clean("run error", &text);
    let traced = events.join("\n");
    assert!(traced.contains(CHAIN), "{traced}");
    assert_clean("trace", &traced);
}

#[tokio::test]
#[serial_test::serial]
async fn a_partly_failing_run_keeps_the_cause_chain_in_the_warning_and_record() {
    let (result, failures, calls, events) = run("issue-2149-partial", true).await;
    assert_eq!(calls, 2, "one backend call per page");
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert_eq!(failures[0].page, 2);
    assert!(failures[0].error.contains(CHAIN), "{}", failures[0].error);
    assert_clean("failure record", &failures[0].error);
    let document = result.expect("page 1 was read").expect("a document");
    let warning = document
        .processing_warnings
        .iter()
        .find(|w| w.message.contains("OCR of page 2 failed"))
        .unwrap_or_else(|| panic!("no page 2 warning: {:?}", document.processing_warnings));
    assert!(warning.message.contains(CHAIN), "{}", warning.message);
    assert_clean("warning", &warning.message);
    let traced = events.join("\n");
    assert!(traced.contains(CHAIN), "{traced}");
    assert_clean("trace", &traced);
}
