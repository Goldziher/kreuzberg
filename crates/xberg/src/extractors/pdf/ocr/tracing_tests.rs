use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use tracing::Instrument as _;
use tracing_subscriber::layer::{Context, SubscriberExt as _};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{Layer, Registry};

use crate::core::config::{ConcurrencyConfig, ExtractionConfig, OcrConfig, OcrPipelineConfig, OcrQualityThresholds};
use crate::engine::seams::{ProgressEvent, ProgressSink, scope_progress};
use crate::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};

const BACKENDS: [&str; 2] = ["issue-2131-caller-a", "issue-2131-caller-b"];
const OCR_TEXT: &str =
    "This scanned invoice contains complete readable words and the final total amount is 125.50 USD.";

#[derive(Default)]
struct Fields(BTreeMap<String, String>);

impl tracing::field::Visit for Fields {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name().to_string(), format!("{value:?}"));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.insert(field.name().to_string(), value.to_string());
    }
}

struct CallerJob(String);

#[derive(Debug)]
struct CapturedEvent {
    message: String,
    backend: String,
    caller_jobs: Vec<String>,
}

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<CapturedEvent>>>);

impl<S> Layer<S> for Capture
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &tracing::span::Attributes<'_>, id: &tracing::Id, context: Context<'_, S>) {
        let mut fields = Fields::default();
        attrs.record(&mut fields);
        if let Some(job) = fields.0.remove("caller_job") {
            context
                .span(id)
                .expect("new span exists")
                .extensions_mut()
                .insert(CallerJob(job));
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, context: Context<'_, S>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        let message = fields.0.remove("message").unwrap_or_default();
        if !matches!(
            message.as_str(),
            "Pipeline: trying OCR backend"
                | "Pipeline: backend produced result"
                | "test OCR backend completed"
                | "request context control"
        ) {
            return;
        }
        let caller_jobs = context
            .event_scope(event)
            .map(|scope| {
                scope
                    .from_root()
                    .filter_map(|span| span.extensions().get::<CallerJob>().map(|job| job.0.clone()))
                    .collect()
            })
            .unwrap_or_default();
        self.0.lock().expect("capture lock").push(CapturedEvent {
            message,
            backend: fields
                .0
                .remove("backend")
                .expect("captured event identifies its backend"),
            caller_jobs,
        });
    }
}

struct TestBackend {
    name: &'static str,
    overlap: Arc<tokio::sync::Barrier>,
}

impl Plugin for TestBackend {
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

#[async_trait::async_trait]
impl OcrBackend for TestBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    fn supports_document_processing(&self) -> bool {
        false
    }

    async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<crate::ExtractedDocument> {
        self.overlap.wait().await;
        tokio::task::yield_now().await;
        tracing::debug!(backend = self.name, "test OCR backend completed");
        Ok(crate::ExtractedDocument {
            content: OCR_TEXT.to_string(),
            ..Default::default()
        })
    }
}

#[derive(Debug)]
struct PageProgress {
    page: usize,
    total: usize,
    completed: usize,
    backend: String,
    input_index: Option<usize>,
}

#[derive(Default)]
struct RecordingSink(Mutex<Vec<PageProgress>>);

impl ProgressSink for RecordingSink {
    fn emit(&self, _: ProgressEvent) {}
    fn emit_ocr_page(&self, page: usize, total: usize, completed: usize, backend: &str, input_index: Option<usize>) {
        self.0.lock().expect("progress lock").push(PageProgress {
            page,
            total,
            completed,
            backend: backend.to_string(),
            input_index,
        });
    }
}

fn mixed_pdf() -> Vec<u8> {
    use lopdf::{Document, Object, Stream, dictionary};
    let mut pdf = Document::with_version("1.4");
    let pages_id = pdf.new_object_id();
    let font = pdf.add_object(dictionary! {"Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica"});
    let raster = pdf.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "Width" => 2, "Height" => 2,
            "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
        },
        vec![0, 0, 0, 255, 255, 255, 255, 255, 255, 0, 0, 0],
    ));
    let resources = pdf.add_object(dictionary! {
        "Font" => dictionary! {"F1" => font}, "XObject" => dictionary! {"Scan" => raster},
    });
    let mut kids = Vec::new();
    for page in 0..3 {
        let operations = if page == 0 {
            b"BT /F1 8 Tf 1 20 Td (Native invoice) Tj ET".as_slice()
        } else {
            b"q 30 0 0 40 0 0 cm /Scan Do Q".as_slice()
        };
        let contents = pdf.add_object(Stream::new(dictionary! {}, operations.to_vec()));
        let page_id = pdf.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "Resources" => resources,
            "MediaBox" => vec![0.into(), 0.into(), 30.into(), 40.into()], "Contents" => contents,
        });
        kids.push(Object::Reference(page_id));
    }
    pdf.objects.insert(
        pages_id,
        dictionary! {"Type" => "Pages", "Kids" => kids, "Count" => 3}.into(),
    );
    let catalog = pdf.add_object(dictionary! {"Type" => "Catalog", "Pages" => pages_id});
    pdf.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).expect("save synthetic mixed PDF");
    bytes
}

fn config(backend: &str, pipeline: bool) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        concurrency: Some(ConcurrencyConfig {
            max_threads: Some(2),
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            backend: backend.to_string(),
            pipeline: pipeline.then(|| OcrPipelineConfig {
                stages: vec![
                    serde_json::from_value(serde_json::json!({"backend": backend, "priority": 100}))
                        .expect("stage config"),
                ],
                quality_thresholds: OcrQualityThresholds {
                    pipeline_min_quality: 0.0,
                    ..Default::default()
                },
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn run_caller(index: usize, pipeline: bool, sink: Arc<RecordingSink>, pdf: &[u8]) {
    let backend = BACKENDS[index];
    let span = tracing::info_span!("request", caller_job = backend);
    scope_progress(
        sink,
        Some(index),
        async {
            tracing::debug!(backend, "request context control");
            let native_text = "Native invoice";
            let boundaries = [
                crate::types::PageBoundary {
                    page_number: 1,
                    byte_start: 0,
                    byte_end: native_text.len(),
                },
                crate::types::PageBoundary {
                    page_number: 2,
                    byte_start: native_text.len(),
                    byte_end: native_text.len(),
                },
                crate::types::PageBoundary {
                    page_number: 3,
                    byte_start: native_text.len(),
                    byte_end: native_text.len(),
                },
            ];
            let result = super::extract_mixed_ocr_native(
                native_text,
                &boundaries,
                &[2, 3],
                pdf,
                &config(backend, pipeline),
                None,
            )
            .await
            .expect("mixed OCR succeeds");
            assert_eq!(result.1.len(), 2, "both OCR pages must reach accepted replacements");
            assert_eq!(result.1[&2], OCR_TEXT);
            assert_eq!(result.1[&3], OCR_TEXT);
        }
        .instrument(span),
    )
    .await;
}

fn assert_progress(sinks: &[Arc<RecordingSink>; 2]) {
    for (index, sink) in sinks.iter().enumerate() {
        let pages = sink.0.lock().expect("progress lock");
        assert_eq!(pages.len(), 2, "each caller must retain both OCR progress events");
        let mut numbers = pages.iter().map(|p| p.page).collect::<Vec<_>>();
        numbers.sort_unstable();
        assert_eq!(numbers, vec![2, 3]);
        let mut completed = pages.iter().map(|p| p.completed).collect::<Vec<_>>();
        completed.sort_unstable();
        assert_eq!(completed, vec![1, 2]);
        for page in pages.iter() {
            assert_eq!(page.total, 3);
            assert_eq!(page.input_index, Some(index));
            assert_eq!(page.backend, BACKENDS[index]);
        }
    }
}

fn assert_event_ancestry(capture: &Capture, pipeline: bool) {
    let events = capture.0.lock().expect("capture lock");
    let count = |message: &str| events.iter().filter(|event| event.message == message).count();
    assert_eq!(
        count("request context control"),
        2,
        "subscriber must observe both caller controls"
    );
    assert_eq!(count("Pipeline: trying OCR backend"), if pipeline { 4 } else { 0 });
    assert_eq!(count("Pipeline: backend produced result"), if pipeline { 4 } else { 0 });
    assert_eq!(count("test OCR backend completed"), 4);
    for event in events.iter() {
        assert_eq!(
            event.caller_jobs,
            vec![event.backend.clone()],
            "OCR event must retain its own caller ancestor and fields: {event:?}"
        );
    }
}

fn parallel_callers(pipeline: bool) {
    let capture = Capture::default();
    let sinks = [Arc::new(RecordingSink::default()), Arc::new(RecordingSink::default())];
    let overlap = Arc::new(tokio::sync::Barrier::new(4));
    for name in BACKENDS {
        register_ocr_backend(Arc::new(TestBackend {
            name,
            overlap: Arc::clone(&overlap),
        }))
        .expect("register test backend");
    }
    tracing::subscriber::with_default(Registry::default().with(capture.clone()), || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime")
            .block_on(async {
                let pdf = mixed_pdf();
                tokio::time::timeout(std::time::Duration::from_secs(10), async {
                    tokio::join!(
                        run_caller(0, pipeline, sinks[0].clone(), &pdf),
                        run_caller(1, pipeline, sinks[1].clone(), &pdf)
                    );
                })
                .await
                .expect("four parallel OCR backend calls must complete");
            });
    });
    for name in BACKENDS {
        unregister_ocr_backend(name).expect("unregister test backend");
    }
    assert_progress(&sinks);
    assert_event_ancestry(&capture, pipeline);
}

fn child_executed_one_test(output: &std::process::Output, test_name: &str) -> bool {
    let stdout = String::from_utf8_lossy(&output.stdout);
    output.status.success()
        && stdout.lines().any(|line| line == format!("test {test_name} ... ok"))
        && stdout.contains("running 1 test")
        && stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;")
}

fn in_registry_child(suffix: &str) -> bool {
    const CHILD_VARIABLE: &str = "XBERG_OCR_TRACING_2131_CHILD";
    let test_name = format!("extractors::pdf::ocr::pipeline::tracing_tests::{suffix}");
    if std::env::var(CHILD_VARIABLE).as_deref() == Ok(test_name.as_str()) {
        return true;
    }
    // ~keep Other parallel tests clear OCR registries without taking a shared serial lock.
    let output = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", &test_name, "--nocapture"])
        .env(CHILD_VARIABLE, &test_name)
        .output()
        .expect("isolated OCR registry test");
    assert!(
        child_executed_one_test(&output, &test_name),
        "OCR child must execute exactly one successful test:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    false
}

#[test]
fn should_preserve_caller_ancestry_and_progress_in_parallel_pdf_ocr_pipeline() {
    if in_registry_child("should_preserve_caller_ancestry_and_progress_in_parallel_pdf_ocr_pipeline") {
        parallel_callers(true);
    }
}

#[test]
fn should_preserve_caller_ancestry_and_progress_in_parallel_single_backend_pdf_ocr() {
    if in_registry_child("should_preserve_caller_ancestry_and_progress_in_parallel_single_backend_pdf_ocr") {
        parallel_callers(false);
    }
}
