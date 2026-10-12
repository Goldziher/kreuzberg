//! Pipeline orchestration tests.

use super::*;
use crate::core::config::OutputFormat;
use crate::types::Metadata;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use serial_test::serial;
use std::borrow::Cow;

#[cfg(feature = "summarization")]
struct SummarizationLifecycleTestProcessor {
    priority: i32,
    fail_initialize: bool,
    fail_shutdown: bool,
    marker: Option<&'static str>,
}

#[cfg(feature = "tokio-runtime")]
struct HandoffRaceProcessor {
    shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
    executed_after_shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
    mutation_error: Option<std::sync::Arc<std::sync::Mutex<Option<String>>>>,
    // #1749: the reentrant mutation attempted from inside `process` below must land on the
    // *same* isolated registry `run_pipeline_with_isolated_registry` is snapshotting from, not
    // the process-wide global -- so the processor carries a handle to it instead of reaching
    // for `crate::plugins::unregister_post_processor`. ~keep
    state: std::sync::Arc<initialization::ProcessorRegistryState>,
}

#[cfg(feature = "tokio-runtime")]
impl crate::plugins::Plugin for HandoffRaceProcessor {
    fn name(&self) -> &str {
        "handoff-race"
    }

    fn version(&self) -> String {
        "test".to_string()
    }

    fn shutdown(&self) -> Result<()> {
        self.shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}

#[cfg(feature = "tokio-runtime")]
#[async_trait::async_trait]
impl crate::plugins::PostProcessor for HandoffRaceProcessor {
    async fn process(&self, _: &mut crate::types::ExtractedDocument, _: &ExtractionConfig) -> Result<()> {
        if let Some(error_slot) = &self.mutation_error {
            let error = self
                .state
                .unregister("handoff-race")
                .expect_err("lifecycle mutation during processing must be rejected");
            *error_slot.lock().unwrap() = Some(error.to_string());
        }
        if self.shutdown.load(std::sync::atomic::Ordering::SeqCst) {
            self.executed_after_shutdown
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
        Ok(())
    }

    fn processing_stage(&self) -> crate::plugins::ProcessingStage {
        crate::plugins::ProcessingStage::Early
    }
}

#[cfg(feature = "summarization")]
impl crate::plugins::Plugin for SummarizationLifecycleTestProcessor {
    fn name(&self) -> &str {
        "summarization"
    }

    fn version(&self) -> String {
        "test".to_string()
    }

    fn initialize(&self) -> Result<()> {
        if self.fail_initialize {
            return Err(crate::XbergError::Other("test initialization failure".to_string()));
        }
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        if self.fail_shutdown {
            return Err(crate::XbergError::Other("test shutdown failure".to_string()));
        }
        Ok(())
    }
}

#[cfg(feature = "summarization")]
#[async_trait::async_trait]
impl crate::plugins::PostProcessor for SummarizationLifecycleTestProcessor {
    async fn process(&self, result: &mut crate::types::ExtractedDocument, _: &ExtractionConfig) -> Result<()> {
        if let Some(marker) = self.marker {
            result
                .metadata
                .additional
                .insert(Cow::Borrowed("lifecycle_test"), serde_json::json!(marker));
        }
        Ok(())
    }

    fn processing_stage(&self) -> crate::plugins::ProcessingStage {
        crate::plugins::ProcessingStage::Middle
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

#[cfg(feature = "summarization")]
fn summarization_test_config() -> ExtractionConfig {
    ExtractionConfig {
        summarization: Some(crate::core::config::SummarizationConfig::default()),
        ..Default::default()
    }
}

#[cfg(feature = "summarization")]
fn restore_builtin_summarization() {
    retry_while_registry_in_use(|| crate::plugins::unregister_post_processor("summarization"));
    retry_while_registry_in_use(crate::plugins::processor::builtin::summarization::register);
}

/// Maximum attempts before a lifecycle mutation is treated as genuinely stuck.
///
/// Carries `retry_while_registry_in_use`'s own cfg: without it the constants outlive the only
/// function that reads them on any feature set that compiles it out, and `-D warnings` turns
/// that into a hard error on the narrow no-ORT legs while every wide-feature leg stays green.
/// #1749 narrowed this from `any(summarization, tokio-runtime)`: the `tokio-runtime`-only
/// callers (the pipeline lifecycle-gate tests) now use an isolated `ProcessorRegistryState`
/// instead, so `tokio-runtime` alone no longer has a caller left -- this must track exactly the
/// feature combinations the remaining call sites below require, or a `pdf,ocr`-only leg (which
/// enables `tokio-runtime` via `ocr` but none of these) is dead code again under `-D warnings`. ~keep
#[cfg(any(
    feature = "summarization",
    all(feature = "captioning", feature = "redaction", feature = "chunking"),
    all(feature = "captioning", feature = "tree-sitter")
))]
const REGISTRY_MUTATION_ATTEMPTS: usize = 100;

/// Delay between attempts, long enough for a concurrent extraction to drop its snapshot lease.
#[cfg(any(
    feature = "summarization",
    all(feature = "captioning", feature = "redaction", feature = "chunking"),
    all(feature = "captioning", feature = "tree-sitter")
))]
const REGISTRY_MUTATION_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(20);

/// Retry a post-processor lifecycle mutation while the registry reports it is in use.
///
/// `with_registration_update` refuses a mutation whenever a snapshot lease is live, and the
/// documented contract (`initialization.rs`) is that this failure is *retryable* -- so
/// `.unwrap()`ing it asserts an exclusivity this binary cannot provide. `#[serial]` only orders a
/// test against the crate's other `#[serial]` tests, while dozens of non-serial tests here run
/// real extractions and hold that lease. Honour the contract instead of racing it. ~keep
#[cfg(any(
    feature = "summarization",
    all(feature = "captioning", feature = "redaction", feature = "chunking"),
    all(feature = "captioning", feature = "tree-sitter")
))]
fn retry_while_registry_in_use<T>(mut mutation: impl FnMut() -> crate::Result<T>) -> T {
    for _ in 0..REGISTRY_MUTATION_ATTEMPTS {
        match mutation() {
            Ok(value) => return value,
            Err(crate::XbergError::Other(message)) if message.contains("retry the lifecycle mutation") => {
                std::thread::sleep(REGISTRY_MUTATION_RETRY_DELAY);
            }
            Err(error) => panic!("post-processor lifecycle mutation failed: {error}"),
        }
    }
    panic!("post-processor registry still in use by a concurrent extraction after retrying");
}

/// Run a lifecycle mutation whose *intended* outcome is an error, retrying only the busy-registry
/// refusal so a concurrent extraction cannot satisfy an `is_err()` assertion for the wrong reason. ~keep
#[cfg(feature = "summarization")]
fn registry_mutation_outcome<T>(mut mutation: impl FnMut() -> crate::Result<T>) -> crate::Result<T> {
    retry_while_registry_in_use(|| match mutation() {
        Err(crate::XbergError::Other(message)) if message.contains("retry the lifecycle mutation") => {
            Err(crate::XbergError::Other(message))
        }
        outcome => Ok(outcome),
    })
}

/// Build an `InternalDocument` with a single paragraph element for pipeline tests.
fn make_doc(content: &str, mime: &str) -> InternalDocument {
    let mut doc = InternalDocument::new("plain");
    doc.mime_type = mime.to_string();
    if !content.is_empty() {
        doc.push_element(InternalElement::text(ElementKind::Paragraph, content, 0));
    }
    doc
}

/// Build an `InternalDocument` with content, mime, and custom metadata.
fn make_doc_with_metadata(content: &str, mime: &str, metadata: Metadata) -> InternalDocument {
    let mut doc = make_doc(content, mime);
    doc.metadata = metadata;
    doc
}

const VALIDATION_MARKER_KEY: &str = "registry_validation_marker";
#[cfg(feature = "quality")]
const QUALITY_VALIDATION_MARKER: &str = "quality_validation_test";
const POSTPROCESSOR_VALIDATION_MARKER: &str = "postprocessor_validation_test";
const ORDER_VALIDATION_MARKER: &str = "order_validation_test";

/// Ensure the quality processor is registered and cache is fresh.
#[cfg(feature = "quality")]
fn ensure_quality_processor() {
    let registry = crate::plugins::registry::get_post_processor_registry();
    let mut reg = registry.write();
    let _ = reg.register(std::sync::Arc::new(crate::text::QualityProcessor));
    drop(reg);
    let _ = clear_processor_cache();
}

#[tokio::test]
#[serial]
async fn test_run_pipeline_basic() {
    let mut doc = make_doc("test", "text/plain");
    doc.metadata.additional.insert(
        Cow::Borrowed(VALIDATION_MARKER_KEY),
        serde_json::json!(ORDER_VALIDATION_MARKER),
    );
    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert_eq!(processed.content, "test");
}

/// Build a minimal `StructuredExtractionConfig` pointed at a closed local port so the LLM
/// call (when `liter-llm` is compiled in) fails fast without touching the network, and the
/// non-`liter-llm` build takes its "requires the 'liter-llm' feature" warning branch instead.
/// Either way `structured_output` stays `None` -- the case GH#1624 is about. ~keep
#[cfg(feature = "heuristics")]
fn make_failing_structured_extraction_config() -> crate::core::config::llm::StructuredExtractionConfig {
    crate::core::config::llm::StructuredExtractionConfig {
        schema: serde_json::json!({"type": "object"}),
        schema_name: crate::core::config::llm::StructuredExtractionConfig::default_schema_name(),
        schema_description: None,
        strict: false,
        prompt: None,
        llm: crate::core::config::llm::LlmConfig {
            model: "openai/gpt-4o-mini".to_string(),
            base_url: Some("http://127.0.0.1:1/v1".to_string()),
            timeout_secs: Some(1),
            max_retries: Some(0),
            ..Default::default()
        },
    }
}

#[tokio::test]
#[serial]
#[cfg(feature = "heuristics")]
async fn test_requested_structured_extraction_without_output_does_not_score_schema_all_valid() {
    let doc = make_doc("Ordinary paragraph with enough text for coverage.", "text/plain");
    let config = ExtractionConfig {
        structured_extraction: Some(make_failing_structured_extraction_config()),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();

    assert!(
        processed.structured_output.is_none(),
        "the LLM call must not succeed against a closed local port"
    );
    let confidence = processed
        .extraction_confidence
        .expect("the heuristics feature must populate extraction_confidence");
    assert_eq!(
        confidence.schema_compliance,
        crate::heuristics::confidence::SchemaCompliance::AllInvalid,
        "GH#1624: a requested-but-unproduced structured_output must not score as AllValid"
    );
    assert_eq!(
        confidence.combined, 0.6,
        "text_coverage 1.0 folds the OCR weight in (0.6) but the schema weight must drop out"
    );
}

#[tokio::test]
#[serial]
#[cfg(feature = "heuristics")]
async fn test_ordinary_extraction_without_structured_config_keeps_full_schema_score() {
    let doc = make_doc("Ordinary paragraph with enough text for coverage.", "text/plain");
    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await.unwrap();

    let confidence = processed
        .extraction_confidence
        .expect("the heuristics feature must populate extraction_confidence");
    assert_eq!(
        confidence.schema_compliance,
        crate::heuristics::confidence::SchemaCompliance::AllValid,
        "structured extraction was never requested, so nothing failed validation"
    );
    assert_eq!(
        confidence.combined, 1.0,
        "an ordinary extraction with no structured config must not lose confidence score"
    );
}

#[test]
#[cfg(feature = "heuristics")]
fn schema_compliance_helper_scores_failure_strictly_below_otherwise_identical_success() {
    use crate::heuristics::confidence::{ConfidenceSignals, ConfidenceWeights, SchemaCompliance, score_confidence};

    let requested_config = ExtractionConfig {
        structured_extraction: Some(make_failing_structured_extraction_config()),
        ..Default::default()
    };
    let succeeded = ExtractedDocument {
        structured_output: Some(serde_json::json!({"title": "ok"})),
        ..Default::default()
    };
    let failed = ExtractedDocument {
        structured_output: None,
        ..Default::default()
    };

    let success_compliance = structured_extraction_compliance(&requested_config, &succeeded);
    let failure_compliance = structured_extraction_compliance(&requested_config, &failed);
    assert_eq!(success_compliance, SchemaCompliance::AllValid);
    assert_eq!(failure_compliance, SchemaCompliance::AllInvalid);

    let weights = ConfidenceWeights::default();
    let text_coverage = 0.8_f32;
    let ocr_aggregate = Some(0.7_f32);
    let success_confidence = score_confidence(
        ConfidenceSignals {
            text_coverage,
            ocr_aggregate,
            schema_compliance: success_compliance,
        },
        weights,
    );
    let failure_confidence = score_confidence(
        ConfidenceSignals {
            text_coverage,
            ocr_aggregate,
            schema_compliance: failure_compliance,
        },
        weights,
    );

    let expected_success: f32 =
        text_coverage * weights.text_coverage + 0.7 * weights.ocr_aggregate + 1.0 * weights.schema_compliance;
    let expected_failure: f32 =
        text_coverage * weights.text_coverage + 0.7 * weights.ocr_aggregate + 0.0 * weights.schema_compliance;
    assert_eq!(success_confidence.combined, expected_success);
    assert_eq!(failure_confidence.combined, expected_failure);
    assert!(
        failure_confidence.combined < success_confidence.combined,
        "a failed structured extraction ({}) must score strictly below an otherwise identical successful one ({})",
        failure_confidence.combined,
        success_confidence.combined
    );
}

#[tokio::test]
#[serial]
#[cfg(feature = "quality")]
async fn test_pipeline_with_quality_processing() {
    ensure_quality_processor();
    let doc = make_doc("This is a test document with some meaningful content.", "text/plain");
    let config = ExtractionConfig {
        enable_quality_processing: true,
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert!(processed.quality_score.is_some());
}

#[tokio::test]
#[serial]
#[cfg(all(feature = "quality", feature = "summarization"))]
async fn builtin_processors_recover_after_public_registry_clear() {
    initialization::initialize_features();
    retry_while_registry_in_use(crate::plugins::clear_post_processors);

    let doc = make_doc(
        "The first paragraph explains the problem. The second paragraph provides enough text for a summary.",
        "text/plain",
    );
    let config = ExtractionConfig {
        enable_quality_processing: true,
        summarization: Some(crate::core::config::SummarizationConfig::default()),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert!(processed.quality_score.is_some());
    assert!(processed.summary.is_some());

    retry_while_registry_in_use(|| crate::plugins::unregister_post_processor("summarization"));
    let doc = make_doc(
        "The first paragraph explains the problem. The second paragraph provides enough text for a summary.",
        "text/plain",
    );
    let processed = run_pipeline(doc, &config).await;
    let restore_result = crate::plugins::processor::builtin::summarization::register();
    restore_result.unwrap();
    let processed = processed.unwrap();
    assert!(processed.quality_score.is_some());
    assert!(processed.summary.is_none());
}

#[tokio::test]
#[serial]
#[cfg(all(feature = "quality", feature = "summarization"))]
async fn unregister_remains_effective_during_pending_builtin_recovery() {
    retry_while_registry_in_use(crate::plugins::clear_post_processors);
    retry_while_registry_in_use(|| crate::plugins::unregister_post_processor("summarization"));

    let config = ExtractionConfig {
        enable_quality_processing: true,
        summarization: Some(crate::core::config::SummarizationConfig::default()),
        ..Default::default()
    };
    let processed = run_pipeline(
        make_doc(
            "Enough document content exists to produce an extractive summary.",
            "text/plain",
        ),
        &config,
    )
    .await;
    let restore_result = crate::plugins::processor::builtin::summarization::register();

    restore_result.unwrap();
    let processed = processed.unwrap();
    assert!(processed.quality_score.is_some());
    assert!(processed.summary.is_none());
}

// #1749: this test races the module's registration gate against a real pipeline run
// deliberately, so it needs its own isolated `ProcessorRegistryState` rather than the
// process-wide registry -- `#[serial]` only ordered it against the crate's other `#[serial]`
// tests, never against the dozens of non-serial tests elsewhere in this binary that hold a
// `ProcessorSnapshotLease` on the global state while running a real extraction. When one of
// those happened to be active at the instant this test's `with_gate` call sampled the lease
// count, the call was rejected before its closure ever ran, and `started_sender` was dropped
// unsent -- `started_receiver.recv().unwrap()` then panicked on a disconnected channel. An
// isolated registry has no such unrelated activity to race. ~keep
#[tokio::test(flavor = "current_thread")]
#[cfg(feature = "tokio-runtime")]
async fn lifecycle_wait_keeps_async_runtime_schedulable() {
    use std::sync::Arc;
    use std::sync::mpsc;
    use std::time::Duration;

    let state = Arc::new(initialization::ProcessorRegistryState::new_isolated());
    state.ensure_cache_current().unwrap();
    let (started_sender, started_receiver) = mpsc::channel();
    let (release_sender, release_receiver) = mpsc::channel();
    let update_state = Arc::clone(&state);
    let update_thread = std::thread::spawn(move || {
        update_state.with_gate(|| {
            started_sender.send(()).unwrap();
            Ok::<_, crate::XbergError>(release_receiver.recv().unwrap())
        })
    });
    started_receiver.recv().unwrap();

    let watchdog_sender = release_sender.clone();
    let watchdog = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        let _ = watchdog_sender.send("watchdog");
    });
    let release_from_async = async move {
        tokio::task::yield_now().await;
        let _ = release_sender.send("async");
    };
    let config = ExtractionConfig::default();
    let (pipeline_result, ()) = tokio::join!(
        run_pipeline_with_isolated_registry(make_doc("test", "text/plain"), &config, Arc::clone(&state)),
        release_from_async
    );

    let release_source = update_thread.join().unwrap().unwrap();
    watchdog.join().unwrap();
    pipeline_result.unwrap();
    assert_eq!(release_source, "async");
}

// #1749: isolated for the same reason as `lifecycle_wait_keeps_async_runtime_schedulable` --
// the `unregister` below deliberately runs while this test's own pipeline is parked mid-snapshot
// and must succeed unconditionally (not merely "eventually"), which only an isolated registry
// with no unrelated concurrent activity can guarantee. ~keep
#[test]
#[cfg(feature = "tokio-runtime")]
fn processor_handoff_rejects_a_snapshot_after_concurrent_shutdown() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, mpsc};

    let state = Arc::new(initialization::ProcessorRegistryState::new_isolated());
    let shutdown = Arc::new(AtomicBool::new(false));
    let executed_after_shutdown = Arc::new(AtomicBool::new(false));
    state
        .register(Arc::new(HandoffRaceProcessor {
            shutdown: Arc::clone(&shutdown),
            executed_after_shutdown: Arc::clone(&executed_after_shutdown),
            mutation_error: None,
            state: Arc::clone(&state),
        }))
        .unwrap();
    state.ensure_cache_current().unwrap();

    let (snapshot_sender, snapshot_receiver) = mpsc::channel();
    let (resume_sender, resume_receiver) = mpsc::channel();
    let pipeline_state = Arc::clone(&state);
    let pipeline_thread = std::thread::spawn(move || {
        initialization::test_support::set_before_processor_snapshot_hook(Box::new(move || {
            snapshot_sender.send(()).unwrap();
            resume_receiver.recv().unwrap();
        }));
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(run_pipeline_with_isolated_registry(
                make_doc("test", "text/plain"),
                &ExtractionConfig::default(),
                pipeline_state,
            ))
    });
    snapshot_receiver.recv().unwrap();
    state.unregister("handoff-race").unwrap();
    assert!(shutdown.load(Ordering::SeqCst));
    resume_sender.send(()).unwrap();
    pipeline_thread.join().unwrap().unwrap();

    assert!(!executed_after_shutdown.load(Ordering::SeqCst));
}

// #1749: isolated for the same reason as the two tests above -- `concurrent_unregister` must be
// driven by this test's own synchronization (the "after registration update began" hook), not by
// an accident of unrelated concurrent activity on a shared registry. ~keep
#[test]
#[cfg(feature = "tokio-runtime")]
fn processor_handoff_lease_rejects_shutdown_until_pipeline_finishes() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, mpsc};

    let state = Arc::new(initialization::ProcessorRegistryState::new_isolated());
    let shutdown = Arc::new(AtomicBool::new(false));
    let executed_after_shutdown = Arc::new(AtomicBool::new(false));
    state
        .register(Arc::new(HandoffRaceProcessor {
            shutdown: Arc::clone(&shutdown),
            executed_after_shutdown: Arc::clone(&executed_after_shutdown),
            mutation_error: None,
            state: Arc::clone(&state),
        }))
        .unwrap();
    state.ensure_cache_current().unwrap();

    let (handoff_sender, handoff_receiver) = mpsc::channel();
    let (pipeline_resume_sender, pipeline_resume_receiver) = mpsc::channel();
    let pipeline_state = Arc::clone(&state);
    let pipeline_thread = std::thread::spawn(move || {
        initialization::test_support::set_after_processor_snapshot_validated_hook(Box::new(move || {
            handoff_sender.send(()).unwrap();
            pipeline_resume_receiver.recv().unwrap();
        }));
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(run_pipeline_with_isolated_registry(
                make_doc("test", "text/plain"),
                &ExtractionConfig::default(),
                pipeline_state,
            ))
    });
    handoff_receiver.recv().unwrap();

    let (mutation_sender, mutation_receiver) = mpsc::channel();
    let unregister_state = Arc::clone(&state);
    let unregister_thread = std::thread::spawn(move || {
        initialization::test_support::set_after_registration_update_began_hook(Box::new(move || {
            mutation_sender.send(()).unwrap();
        }));
        unregister_state.unregister("handoff-race")
    });
    mutation_receiver.recv().unwrap();
    assert!(!shutdown.load(Ordering::SeqCst));
    pipeline_resume_sender.send(()).unwrap();
    pipeline_thread.join().unwrap().unwrap();
    let concurrent_unregister = unregister_thread.join().unwrap();
    state.unregister("handoff-race").unwrap();

    assert!(concurrent_unregister.is_err());
    assert!(shutdown.load(Ordering::SeqCst));
    assert!(!executed_after_shutdown.load(Ordering::SeqCst));
}

// #1749: isolated for the same reason as the three tests above. This test used to need the
// `retry_while_registry_in_use` dance documented below just to get its own setup registered
// without racing an unrelated concurrent extraction's lease; an isolated registry has no such
// lease to race, so the registration below always succeeds on the first try.
#[test]
#[cfg(feature = "tokio-runtime")]
fn reentrant_lifecycle_mutation_returns_in_use_error_without_deadlock() {
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex, mpsc};
    use std::time::Duration;

    let state = Arc::new(initialization::ProcessorRegistryState::new_isolated());
    let mutation_error = Arc::new(Mutex::new(None));
    let thread_error = Arc::clone(&mutation_error);
    state
        .register(Arc::new(HandoffRaceProcessor {
            shutdown: Arc::new(AtomicBool::new(false)),
            executed_after_shutdown: Arc::new(AtomicBool::new(false)),
            mutation_error: Some(Arc::clone(&thread_error)),
            state: Arc::clone(&state),
        }))
        .unwrap();
    let (result_sender, result_receiver) = mpsc::channel();
    let pipeline_state = Arc::clone(&state);
    let pipeline_thread = std::thread::spawn(move || {
        let pipeline_result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(run_pipeline_with_isolated_registry(
                make_doc("test", "text/plain"),
                &ExtractionConfig::default(),
                pipeline_state,
            ));
        result_sender.send(pipeline_result).unwrap();
    });

    let pipeline_result = result_receiver
        .recv_timeout(Duration::from_millis(250))
        .expect("reentrant lifecycle mutation must not deadlock the pipeline");
    pipeline_thread.join().unwrap();

    pipeline_result.unwrap();
    let error = mutation_error.lock().unwrap().clone().unwrap();
    assert!(error.contains("in use by an active extraction"));
}

#[tokio::test]
#[serial]
#[cfg(feature = "summarization")]
async fn failed_explicit_builtin_registration_preserves_suppression() {
    retry_while_registry_in_use(crate::plugins::clear_post_processors);
    retry_while_registry_in_use(|| crate::plugins::unregister_post_processor("summarization"));
    let registration = registry_mutation_outcome(|| {
        crate::plugins::register_post_processor(std::sync::Arc::new(SummarizationLifecycleTestProcessor {
            priority: 90,
            fail_initialize: true,
            fail_shutdown: false,
            marker: None,
        }))
    });

    let processed = run_pipeline(
        make_doc("Enough content exists to create a summary.", "text/plain"),
        &summarization_test_config(),
    )
    .await;
    restore_builtin_summarization();

    let registration_error = registration.expect_err("initialize failure must reject the registration");
    assert!(registration_error.to_string().contains("test initialization failure"));
    assert!(processed.unwrap().summary.is_none());
}

#[tokio::test]
#[serial]
#[cfg(feature = "summarization")]
async fn failed_builtin_replacement_triggers_automatic_recovery() {
    retry_while_registry_in_use(crate::plugins::clear_post_processors);
    initialization::initialize_processor_cache().unwrap();
    retry_while_registry_in_use(|| {
        crate::plugins::register_post_processor(std::sync::Arc::new(SummarizationLifecycleTestProcessor {
            priority: 90,
            fail_initialize: false,
            fail_shutdown: true,
            marker: None,
        }))
    });
    let replacement = registry_mutation_outcome(|| {
        crate::plugins::register_post_processor(std::sync::Arc::new(SummarizationLifecycleTestProcessor {
            priority: 91,
            fail_initialize: false,
            fail_shutdown: false,
            marker: None,
        }))
    });

    let processed = run_pipeline(
        make_doc("Enough content exists to create a summary.", "text/plain"),
        &summarization_test_config(),
    )
    .await;
    restore_builtin_summarization();

    let replacement_error = replacement.expect_err("shutdown failure must reject the replacement");
    assert!(replacement_error.to_string().contains("test shutdown failure"));
    assert!(processed.unwrap().summary.is_some());
}

#[tokio::test]
#[serial]
#[cfg(feature = "summarization")]
async fn bootstrap_preserves_custom_processor_with_builtin_name() {
    let custom: std::sync::Arc<dyn crate::plugins::PostProcessor> =
        std::sync::Arc::new(SummarizationLifecycleTestProcessor {
            priority: 97,
            fail_initialize: false,
            fail_shutdown: false,
            marker: Some("custom-summarization"),
        });
    retry_while_registry_in_use(crate::plugins::clear_post_processors);
    retry_while_registry_in_use(|| crate::plugins::register_post_processor(std::sync::Arc::clone(&custom)));

    let processed = run_pipeline(make_doc("test", "text/plain"), &ExtractionConfig::default()).await;
    let registered = crate::plugins::registry::get_post_processor_registry()
        .read()
        .get_for_stage(crate::plugins::ProcessingStage::Middle)
        .into_iter()
        .find(|processor| processor.name() == "summarization")
        .unwrap();
    let identity_preserved = std::sync::Arc::ptr_eq(&registered, &custom);
    let priority = registered.priority();
    restore_builtin_summarization();

    let processed = processed.unwrap();
    assert!(identity_preserved);
    assert_eq!(priority, 97);
    assert_eq!(processed.metadata.additional["lifecycle_test"], "custom-summarization");
}

#[test]
#[serial]
#[cfg(all(feature = "quality", feature = "summarization"))]
fn concurrent_builtin_recovery_waits_for_complete_registration() {
    const CALLER_COUNT: usize = 8;

    retry_while_registry_in_use(crate::plugins::clear_post_processors);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(CALLER_COUNT));
    let callers = (0..CALLER_COUNT)
        .map(|_| {
            let barrier = std::sync::Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                initialization::initialize_features();
                crate::plugins::registry::get_post_processor_registry().read().list()
            })
        })
        .collect::<Vec<_>>();

    for caller in callers {
        let processor_names = caller.join().unwrap();
        assert!(processor_names.iter().any(|name| name == "quality-processing"));
        assert!(processor_names.iter().any(|name| name == "summarization"));
    }
}

#[tokio::test]
#[serial]
async fn test_pipeline_without_quality_processing() {
    let doc = make_doc("test", "text/plain");
    let config = ExtractionConfig {
        enable_quality_processing: false,
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert!(processed.quality_score.is_none());
}

#[tokio::test]
#[serial]
#[cfg(feature = "chunking")]
async fn test_pipeline_with_chunking() {
    let doc = make_doc(
        &"This is a long text that should be chunked. ".repeat(100),
        "text/plain",
    );
    let config = ExtractionConfig {
        chunking: Some(crate::ChunkingConfig {
            max_characters: 500,
            overlap: 50,
            trim: true,
            chunker_type: crate::ChunkerType::Text,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    let chunks = processed.chunks.as_ref().expect("chunks should be present");
    assert!(chunks.len() > 1);
}

#[tokio::test]
#[serial]
async fn test_pipeline_without_chunking() {
    let doc = make_doc("test", "text/plain");
    let config = ExtractionConfig {
        chunking: None,
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert!(processed.chunks.is_none());
}

#[tokio::test]
#[serial]
async fn test_pipeline_preserves_metadata() {
    use ahash::AHashMap;
    let mut additional = AHashMap::new();
    additional.insert(Cow::Borrowed("source"), serde_json::json!("test"));
    additional.insert(Cow::Borrowed("page"), serde_json::json!(1));

    let doc = make_doc_with_metadata(
        "test",
        "text/plain",
        Metadata {
            additional,
            ..Default::default()
        },
    );
    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert_eq!(
        processed.metadata.additional.get("source").unwrap(),
        &serde_json::json!("test")
    );
    assert_eq!(
        processed.metadata.additional.get("page").unwrap(),
        &serde_json::json!(1)
    );
}

#[tokio::test]
#[serial]
async fn test_pipeline_preserves_tables() {
    use crate::types::Table;

    let table = Table {
        cells: vec![vec!["A".to_string(), "B".to_string()]],
        markdown: "| A | B |".to_string(),
        page_number: 0,
        bounding_box: None,
        ..Default::default()
    };

    let mut doc = make_doc("test", "text/plain");
    doc.tables.push(table);
    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert_eq!(processed.tables.len(), 1);
    assert_eq!(processed.tables[0].cells.len(), 1);
}

#[tokio::test]
#[serial]
async fn test_pipeline_empty_content() {
    {
        let registry = crate::plugins::registry::get_post_processor_registry();
        registry.write().shutdown_all().unwrap();
    }
    {
        let registry = crate::plugins::registry::get_validator_registry();
        registry.write().shutdown_all().unwrap();
    }

    let doc = make_doc("", "text/plain");
    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert_eq!(processed.content, "");
}

#[tokio::test]
#[serial]
#[cfg(feature = "chunking")]
async fn test_pipeline_with_all_features() {
    #[cfg(feature = "quality")]
    ensure_quality_processor();
    let doc = make_doc(&"This is a comprehensive test document. ".repeat(50), "text/plain");
    let config = ExtractionConfig {
        enable_quality_processing: true,
        chunking: Some(crate::ChunkingConfig {
            max_characters: 500,
            overlap: 50,
            trim: true,
            chunker_type: crate::ChunkerType::Text,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    #[cfg(feature = "quality")]
    assert!(processed.quality_score.is_some());
    assert!(processed.chunks.is_some());
}

#[tokio::test]
#[serial]
#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
async fn test_pipeline_with_keyword_extraction() {
    crate::plugins::registry::get_validator_registry()
        .write()
        .shutdown_all()
        .unwrap();
    crate::plugins::registry::get_post_processor_registry()
        .write()
        .shutdown_all()
        .unwrap();

    let _ = crate::keywords::register_keyword_processor();
    clear_processor_cache().unwrap();

    let doc = make_doc(
        r#"
Machine learning is a branch of artificial intelligence that focuses on
building systems that can learn from data. Deep learning is a subset of
machine learning that uses neural networks with multiple layers.
Natural language processing enables computers to understand human language.
            "#,
        "text/plain",
    );
    #[cfg(feature = "keywords-yake")]
    let keyword_config = crate::keywords::KeywordConfig::yake();

    #[cfg(all(feature = "keywords-rake", not(feature = "keywords-yake")))]
    let keyword_config = crate::keywords::KeywordConfig::rake();

    let config = ExtractionConfig {
        keywords: Some(keyword_config),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();

    let keywords = processed
        .extracted_keywords
        .as_ref()
        .expect("Should have extracted keywords");
    assert!(!keywords.is_empty(), "Should have extracted keywords");

    let first_keyword = &keywords[0];
    assert!(!first_keyword.text.is_empty());
    assert!(first_keyword.score > 0.0);
}

#[tokio::test]
#[serial]
#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
async fn test_pipeline_without_keyword_config() {
    let doc = make_doc("Machine learning and artificial intelligence.", "text/plain");

    let config = ExtractionConfig {
        keywords: None,
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();

    assert!(!processed.metadata.additional.contains_key("keywords"));
}

#[tokio::test]
#[serial]
#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
async fn test_pipeline_keyword_extraction_short_content() {
    crate::plugins::registry::get_validator_registry()
        .write()
        .shutdown_all()
        .unwrap();
    crate::plugins::registry::get_post_processor_registry()
        .write()
        .shutdown_all()
        .unwrap();

    let doc = make_doc("Short text", "text/plain");

    #[cfg(feature = "keywords-yake")]
    let keyword_config = crate::keywords::KeywordConfig::yake();

    #[cfg(all(feature = "keywords-rake", not(feature = "keywords-yake")))]
    let keyword_config = crate::keywords::KeywordConfig::rake();

    let config = ExtractionConfig {
        keywords: Some(keyword_config),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();

    assert!(!processed.metadata.additional.contains_key("keywords"));
}

#[tokio::test]
#[serial]
async fn test_postprocessor_runs_before_validator() {
    use crate::plugins::{Plugin, PostProcessor, ProcessingStage, Validator};
    use async_trait::async_trait;
    use std::sync::Arc;

    struct TestPostProcessor;
    impl Plugin for TestPostProcessor {
        fn name(&self) -> &str {
            "test-processor"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl PostProcessor for TestPostProcessor {
        async fn process(&self, result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
            result
                .metadata
                .additional
                .insert(Cow::Borrowed("processed"), serde_json::json!(true));
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Middle
        }
    }

    struct TestValidator;
    impl Plugin for TestValidator {
        fn name(&self) -> &str {
            "test-validator"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl Validator for TestValidator {
        async fn validate(&self, result: &ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
            let should_validate = result
                .metadata
                .additional
                .get(VALIDATION_MARKER_KEY)
                .and_then(|v| v.as_str())
                == Some(POSTPROCESSOR_VALIDATION_MARKER);

            if !should_validate {
                return Ok(());
            }

            let processed = result
                .metadata
                .additional
                .get("processed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if !processed {
                return Err(crate::XbergError::Validation {
                    message: "Post-processor did not run before validator".to_string(),
                    source: None,
                });
            }
            Ok(())
        }
    }

    let pp_registry = crate::plugins::registry::get_post_processor_registry();
    let val_registry = crate::plugins::registry::get_validator_registry();

    clear_processor_cache().unwrap();
    pp_registry.write().shutdown_all().unwrap();
    val_registry.write().shutdown_all().unwrap();
    clear_processor_cache().unwrap();

    {
        let mut registry = pp_registry.write();
        registry.register(Arc::new(TestPostProcessor)).unwrap();
    }

    {
        let mut registry = val_registry.write();
        registry.register(Arc::new(TestValidator)).unwrap();
    }

    clear_processor_cache().unwrap();

    let mut doc = make_doc("test", "text/plain");
    doc.metadata.additional.insert(
        Cow::Borrowed(VALIDATION_MARKER_KEY),
        serde_json::json!(POSTPROCESSOR_VALIDATION_MARKER),
    );

    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: true,
            enabled_set: None,
            disabled_set: None,
            enabled_processors: None,
            disabled_processors: None,
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await;

    pp_registry.write().shutdown_all().unwrap();
    val_registry.write().shutdown_all().unwrap();

    assert!(processed.is_ok(), "Validator should have seen post-processor metadata");
    let processed = processed.unwrap();
    assert_eq!(
        processed.metadata.additional.get("processed"),
        Some(&serde_json::json!(true)),
        "Post-processor metadata should be present"
    );
}

#[tokio::test]
#[serial]
#[cfg(feature = "quality")]
async fn test_quality_processing_runs_before_validator() {
    ensure_quality_processor();
    use crate::plugins::{Plugin, Validator};
    use async_trait::async_trait;
    use std::sync::Arc;

    struct QualityValidator;
    impl Plugin for QualityValidator {
        fn name(&self) -> &str {
            "quality-validator"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl Validator for QualityValidator {
        async fn validate(&self, result: &ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
            let should_validate = result
                .metadata
                .additional
                .get(VALIDATION_MARKER_KEY)
                .and_then(|v| v.as_str())
                == Some(QUALITY_VALIDATION_MARKER);

            if !should_validate {
                return Ok(());
            }

            if result.quality_score.is_none() {
                return Err(crate::XbergError::Validation {
                    message: "Quality processing did not run before validator".to_string(),
                    source: None,
                });
            }
            Ok(())
        }
    }

    let val_registry = crate::plugins::registry::get_validator_registry();
    {
        let mut registry = val_registry.write();
        registry.register(Arc::new(QualityValidator)).unwrap();
    }

    let mut doc = make_doc("This is meaningful test content for quality scoring.", "text/plain");
    doc.metadata.additional.insert(
        Cow::Borrowed(VALIDATION_MARKER_KEY),
        serde_json::json!(QUALITY_VALIDATION_MARKER),
    );

    let config = ExtractionConfig {
        enable_quality_processing: true,
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await;

    {
        let mut registry = val_registry.write();
        registry.remove("quality-validator").unwrap();
    }

    assert!(processed.is_ok(), "Validator should have seen quality_score");
}

#[tokio::test]
#[serial]
async fn test_multiple_postprocessors_run_before_validator() {
    use crate::plugins::{Plugin, PostProcessor, ProcessingStage, Validator};
    use async_trait::async_trait;
    use std::sync::Arc;

    struct EarlyProcessor;
    impl Plugin for EarlyProcessor {
        fn name(&self) -> &str {
            "early-proc"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl PostProcessor for EarlyProcessor {
        async fn process(&self, result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
            let mut order = result
                .metadata
                .additional
                .get("execution_order")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            order.push(serde_json::json!("early"));
            result
                .metadata
                .additional
                .insert(Cow::Borrowed("execution_order"), serde_json::json!(order));
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Early
        }
    }

    struct LateProcessor;
    impl Plugin for LateProcessor {
        fn name(&self) -> &str {
            "late-proc"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl PostProcessor for LateProcessor {
        async fn process(&self, result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
            let mut order = result
                .metadata
                .additional
                .get("execution_order")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            order.push(serde_json::json!("late"));
            result
                .metadata
                .additional
                .insert(Cow::Borrowed("execution_order"), serde_json::json!(order));
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Late
        }
    }

    struct OrderValidator;
    impl Plugin for OrderValidator {
        fn name(&self) -> &str {
            "order-validator"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl Validator for OrderValidator {
        async fn validate(&self, result: &ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
            let should_validate = result
                .metadata
                .additional
                .get(VALIDATION_MARKER_KEY)
                .and_then(|v| v.as_str())
                == Some(ORDER_VALIDATION_MARKER);

            if !should_validate {
                return Ok(());
            }

            let order = result
                .metadata
                .additional
                .get("execution_order")
                .and_then(|v| v.as_array())
                .ok_or_else(|| crate::XbergError::Validation {
                    message: "No execution order found".to_string(),
                    source: None,
                })?;

            if order.len() != 2 {
                return Err(crate::XbergError::Validation {
                    message: format!("Expected 2 processors to run, got {}", order.len()),
                    source: None,
                });
            }

            if order[0] != "early" || order[1] != "late" {
                return Err(crate::XbergError::Validation {
                    message: format!("Wrong execution order: {:?}", order),
                    source: None,
                });
            }

            Ok(())
        }
    }

    let pp_registry = crate::plugins::registry::get_post_processor_registry();
    let val_registry = crate::plugins::registry::get_validator_registry();

    pp_registry.write().shutdown_all().unwrap();
    val_registry.write().shutdown_all().unwrap();
    clear_processor_cache().unwrap();

    {
        let mut registry = pp_registry.write();
        registry.register(Arc::new(EarlyProcessor)).unwrap();
        registry.register(Arc::new(LateProcessor)).unwrap();
    }

    {
        let mut registry = val_registry.write();
        registry.register(Arc::new(OrderValidator)).unwrap();
    }

    clear_processor_cache().unwrap();

    let doc = make_doc("test", "text/plain");

    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await;

    pp_registry.write().shutdown_all().unwrap();
    val_registry.write().shutdown_all().unwrap();
    clear_processor_cache().unwrap();

    assert!(processed.is_ok(), "All processors should run before validator");
}

#[tokio::test]
#[serial]
#[cfg(feature = "chunking")]
async fn test_middle_postprocessors_run_after_explicit_chunking() {
    use crate::plugins::{Plugin, PostProcessor, ProcessingStage};
    use async_trait::async_trait;
    use std::sync::Arc;

    const CHUNK_MARKER: &str = "middle_saw_chunks";

    struct ChunkAwareMiddleProcessor;
    impl Plugin for ChunkAwareMiddleProcessor {
        fn name(&self) -> &str {
            "chunk-aware-middle"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl PostProcessor for ChunkAwareMiddleProcessor {
        async fn process(&self, result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
            result.metadata.additional.insert(
                Cow::Borrowed(CHUNK_MARKER),
                serde_json::json!(result.chunks.as_ref().is_some_and(|chunks| !chunks.is_empty())),
            );
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Middle
        }
    }

    let registry = crate::plugins::registry::get_post_processor_registry();
    {
        let mut reg = registry.write();
        reg.register(Arc::new(ChunkAwareMiddleProcessor)).unwrap();
    }
    clear_processor_cache().unwrap();

    let doc = make_doc(&"chunk me ".repeat(100), "text/plain");
    let config = ExtractionConfig {
        chunking: Some(crate::ChunkingConfig {
            max_characters: 80,
            overlap: 0,
            trim: true,
            chunker_type: crate::ChunkerType::Text,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await;

    {
        let mut reg = registry.write();
        reg.remove("chunk-aware-middle").unwrap();
    }
    clear_processor_cache().unwrap();

    let processed = processed.unwrap();
    assert_eq!(
        processed.metadata.additional.get(CHUNK_MARKER),
        Some(&serde_json::json!(true)),
        "Middle-stage processors should see explicit chunking output"
    );
}

#[tokio::test]
#[serial]
#[cfg(all(feature = "captioning", feature = "redaction", feature = "chunking"))]
async fn captioning_prepass_keeps_redaction_and_chunks_consistent() {
    use crate::core::config::{CaptioningConfig, LlmConfig, RedactionConfig};
    use crate::plugins::{Plugin, PostProcessor, ProcessingStage};
    use crate::types::ExtractedImage;
    use async_trait::async_trait;
    use bytes::Bytes;
    use std::sync::Arc;

    const ORIGINAL_PII: &str = "alice@example.com";
    const CAPTION_PREFIX: &str = "Photo owned by";

    struct StubCaptioningProcessor;

    impl Plugin for StubCaptioningProcessor {
        fn name(&self) -> &str {
            CAPTIONING_PROCESSOR_NAME
        }

        fn version(&self) -> String {
            "1.0.0".to_string()
        }

        fn initialize(&self) -> Result<()> {
            Ok(())
        }

        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl PostProcessor for StubCaptioningProcessor {
        async fn process(&self, result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
            for image in result.images.iter_mut().flatten() {
                let caption = format!("{CAPTION_PREFIX} {ORIGINAL_PII}");
                image.description = Some(caption.clone());
                image.caption = Some(caption);
            }
            result.processing_warnings.push(crate::types::ProcessingWarning {
                source: Cow::Borrowed("captioning"),
                message: Cow::Borrowed("synthetic caption warning"),
            });
            result.llm_usage.get_or_insert_default().push(crate::types::LlmUsage {
                model: "synthetic-caption-model".to_string(),
                source: "captioning".to_string(),
                ..Default::default()
            });
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Middle
        }

        fn priority(&self) -> i32 {
            50
        }
    }

    initialization::initialize_features();
    retry_while_registry_in_use(crate::plugins::processor::builtin::redaction::register);
    let registry = crate::plugins::registry::get_post_processor_registry();
    registry.write().register(Arc::new(StubCaptioningProcessor)).unwrap();
    clear_processor_cache().unwrap();

    let mut doc = make_doc(&format!("Contact {ORIGINAL_PII}."), "application/pdf");
    doc.processing_warnings.push(crate::types::ProcessingWarning {
        source: Cow::Borrowed("extraction"),
        message: Cow::Borrowed("synthetic extraction warning"),
    });
    doc.llm_usage = Some(vec![crate::types::LlmUsage {
        model: "synthetic-extraction-model".to_string(),
        source: "extraction".to_string(),
        ..Default::default()
    }]);
    let image_index = doc.push_image(ExtractedImage {
        data: Bytes::from_static(b"synthetic image"),
        format: Cow::Borrowed("png"),
        image_index: 0,
        width: Some(100),
        height: Some(100),
        ..Default::default()
    });
    doc.push_element(InternalElement::text(ElementKind::Image { image_index }, "", 0));

    let config = ExtractionConfig {
        captioning: Some(CaptioningConfig {
            llm: LlmConfig::default(),
            prompt: None,
            min_image_area: 1,
            alt_text: Default::default(),
        }),
        redaction: Some(RedactionConfig::default()),
        chunking: Some(crate::ChunkingConfig {
            max_characters: 500,
            overlap: 0,
            ..Default::default()
        }),
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await;

    retry_while_registry_in_use(crate::plugins::processor::builtin::captioning::register);
    clear_processor_cache().unwrap();

    let processed = processed.unwrap();
    assert!(
        processed.content.contains(CAPTION_PREFIX),
        "redacted output must retain the generated caption: {}",
        processed.content
    );
    assert!(
        !processed.content.contains(ORIGINAL_PII),
        "redacted output must not restore original PII: {}",
        processed.content
    );
    if let Some(formatted_content) = processed.formatted_content.as_deref() {
        assert!(
            formatted_content.contains(CAPTION_PREFIX),
            "formatted output must retain the generated caption: {formatted_content}"
        );
        assert!(
            !formatted_content.contains(ORIGINAL_PII),
            "formatted output must not restore original PII: {formatted_content}"
        );
    }

    let caption = processed
        .images
        .as_deref()
        .and_then(|images| images.first())
        .and_then(|image| image.caption.as_deref())
        .expect("captioning must retain the image caption");
    assert!(caption.contains(CAPTION_PREFIX), "image caption was lost: {caption}");
    assert!(
        !caption.contains(ORIGINAL_PII),
        "image caption must be redacted: {caption}"
    );

    let chunk_text = processed
        .chunks
        .as_ref()
        .expect("chunking must produce chunks")
        .iter()
        .map(|chunk| chunk.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        chunk_text.contains(CAPTION_PREFIX),
        "chunks must include the generated caption: {chunk_text}"
    );
    assert!(
        !chunk_text.contains(ORIGINAL_PII),
        "redacted chunks must not contain original PII: {chunk_text}"
    );
    assert_eq!(
        processed
            .processing_warnings
            .iter()
            .map(|warning| warning.source.as_ref())
            .collect::<Vec<_>>(),
        vec!["extraction", "captioning"],
        "captioning prepass must preserve existing warnings and append its own"
    );
    assert_eq!(
        processed
            .llm_usage
            .as_ref()
            .expect("prepass must preserve LLM usage")
            .iter()
            .map(|usage| usage.source.as_str())
            .collect::<Vec<_>>(),
        vec!["extraction", "captioning"],
        "captioning prepass must preserve existing usage and append its own"
    );
}

/// #355: the captioning prepass derives a full `ExtractedDocument` from `doc`
/// before the pipeline's main (second) derivation runs on the same `doc`. That
/// first derivation destructively `.remove()`s `CODE_INTELLIGENCE_SCRATCH_KEY`
/// from `metadata.additional` (see `derive::derive_extraction_result`), and the
/// prepass then overwrites `doc.metadata` wholesale with the already-stripped
/// copy. Without carrying the computed payload back onto `doc`, the second
/// derivation finds no scratch key and silently falls back to a degraded
/// `CodeMetadata`-only `code_intelligence` (losing metrics, structure, imports,
/// exports, etc. — see #259). Assert the exact full payload survives.
#[tokio::test]
#[serial]
#[cfg(all(feature = "captioning", feature = "tree-sitter"))]
async fn captioning_prepass_preserves_full_code_intelligence_scratch_payload() {
    use crate::core::config::{CaptioningConfig, LlmConfig};
    use crate::plugins::{Plugin, PostProcessor, ProcessingStage};
    use crate::types::metadata::{CodeMetadata, FormatMetadata};
    use async_trait::async_trait;
    use std::sync::Arc;

    struct NoopCaptioningProcessor;

    impl Plugin for NoopCaptioningProcessor {
        fn name(&self) -> &str {
            CAPTIONING_PROCESSOR_NAME
        }

        fn version(&self) -> String {
            "1.0.0".to_string()
        }

        fn initialize(&self) -> Result<()> {
            Ok(())
        }

        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl PostProcessor for NoopCaptioningProcessor {
        async fn process(&self, _result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Middle
        }

        fn priority(&self) -> i32 {
            50
        }
    }

    initialization::initialize_features();
    let registry = crate::plugins::registry::get_post_processor_registry();
    registry.write().register(Arc::new(NoopCaptioningProcessor)).unwrap();
    clear_processor_cache().unwrap();

    let expected_code_intelligence = serde_json::json!({
        "language": "python",
        "metrics": {"total_lines": 1},
    });

    let mut doc = make_doc("def f(): pass", "text/x-python");
    doc.metadata.format = Some(FormatMetadata::Code(CodeMetadata::default()));
    doc.metadata.additional.insert(
        Cow::Borrowed(crate::extractors::code::CODE_INTELLIGENCE_SCRATCH_KEY),
        expected_code_intelligence.clone(),
    );

    let config = ExtractionConfig {
        captioning: Some(CaptioningConfig {
            llm: LlmConfig::default(),
            prompt: None,
            min_image_area: 1,
            alt_text: Default::default(),
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();

    // Restore the real captioning processor for subsequent tests in this module.
    retry_while_registry_in_use(crate::plugins::processor::builtin::captioning::register);
    clear_processor_cache().unwrap();

    assert_eq!(
        processed.code_intelligence,
        Some(expected_code_intelligence),
        "captioning prepass must not degrade code_intelligence on the pipeline's second derivation"
    );
    assert!(
        !processed
            .metadata
            .additional
            .contains_key(crate::extractors::code::CODE_INTELLIGENCE_SCRATCH_KEY),
        "scratch key must never leak into final metadata"
    );
}

#[tokio::test]
#[serial]
async fn test_run_pipeline_with_output_format_plain() {
    let doc = make_doc("test content", "text/plain");

    let config = crate::core::config::ExtractionConfig {
        output_format: OutputFormat::Plain,
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert_eq!(processed.content, "test content");
    assert_eq!(processed.metadata.output_format, Some("plain".to_string()));
}

#[tokio::test]
#[serial]
async fn test_pipeline_honors_include_watermarks_for_markdown() {
    let watermark = "Research title 7 arXiv:2401.12345v2 [cs.CL] 9 Jan 2024";
    let default_config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let stripped = run_pipeline(make_doc(watermark, "application/pdf"), &default_config)
        .await
        .unwrap();

    let preserve_config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        content_filter: Some(crate::core::config::ContentFilterConfig {
            include_watermarks: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let preserved = run_pipeline(make_doc(watermark, "application/pdf"), &preserve_config)
        .await
        .unwrap();

    assert!(!stripped.content.contains("arXiv:2401.12345v2"));
    assert!(preserved.content.contains("arXiv:2401.12345v2 [cs.CL] 9 Jan 2024"));
}

#[tokio::test]
#[serial]
async fn test_run_pipeline_with_output_format_djot() {
    let doc = make_doc("test content", "text/djot");

    let config = crate::core::config::ExtractionConfig {
        output_format: OutputFormat::Djot,
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert!(!processed.content.is_empty());
    assert_eq!(processed.metadata.output_format, Some("djot".to_string()));
}

#[tokio::test]
#[serial]
async fn test_run_pipeline_with_output_format_html() {
    let doc = make_doc("test content", "text/plain");

    let config = crate::core::config::ExtractionConfig {
        output_format: OutputFormat::Html,
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert!(processed.content.contains("test content"));
    assert_eq!(processed.metadata.output_format, Some("html".to_string()));
}

#[tokio::test]
#[serial]
#[cfg(feature = "quality")]
async fn test_nfc_normalization_decomposes_to_composed() {
    let doc = make_doc("caf\u{0065}\u{0301}", "text/plain");
    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert_eq!(processed.content, "caf\u{00e9}");
    assert!(!processed.content.contains('\u{0301}'));
}

#[tokio::test]
#[serial]
#[cfg(feature = "quality")]
async fn test_nfc_normalization_idempotent_on_ascii() {
    let doc = make_doc("Hello, world! 123", "text/plain");
    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert_eq!(processed.content, "Hello, world! 123");
}

#[tokio::test]
#[serial]
#[cfg(feature = "quality")]
async fn test_nfc_normalization_applies_to_page_content() {
    let mut doc = InternalDocument::new("plain");
    doc.mime_type = "text/plain".to_string();
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "re\u{0301}sume\u{0301}", 0).with_page(1));
    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert!(processed.content.contains("r\u{00e9}sum\u{00e9}"));
    let pages = processed.pages.unwrap();
    assert_eq!(pages[0].content, "r\u{00e9}sum\u{00e9}");
}

#[tokio::test]
#[serial]
#[cfg(feature = "quality")]
async fn nfc_normalization_applies_to_the_kept_text_layer() {
    let mut doc = InternalDocument::new("plain");
    doc.mime_type = "text/plain".to_string();
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "resume", 0));
    doc.prebuilt_pages = Some(vec![crate::types::PageContent {
        page_number: 1,
        content: "re\u{0301}sume\u{0301}".to_string(),
        tables: Vec::new(),
        image_indices: Vec::new(),
        image_preprocessing: None,
        hierarchy: None,
        is_blank: None,
        layout_regions: None,
        speaker_notes: None,
        section_name: None,
        sheet_name: None,
        ocr_confidence: None,
        native_content: Some("cafe\u{0301}".to_string()),
    }]);
    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    let pages = processed.pages.unwrap();
    assert_eq!(pages[0].native_content.as_deref(), Some("caf\u{00e9}"));
    assert_eq!(pages[0].content, "r\u{00e9}sum\u{00e9}");
}

#[tokio::test]
#[serial]
async fn test_run_pipeline_applies_output_format_last() {
    let doc = make_doc("test", "text/plain");

    let config = crate::core::config::ExtractionConfig {
        output_format: OutputFormat::Djot,
        enable_quality_processing: false,
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert_eq!(processed.metadata.output_format, Some("djot".to_string()));
}

#[tokio::test]
#[serial]
#[cfg(all(feature = "pdf", feature = "chunking"))]
async fn test_chunking_populates_page_numbers_for_pdf() {
    use crate::core::config::ChunkingConfig;

    let pdf_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/issue-636-chunk-pages.pdf");

    if !pdf_path.exists() {
        return;
    }

    let pdf_bytes = std::fs::read(&pdf_path).unwrap();

    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_characters: 500,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = crate::core::extractor::extract_bytes(&pdf_bytes, "application/pdf", &config)
        .await
        .unwrap();

    assert!(result.chunks.is_some(), "Chunks should be produced");
    let chunks = result.chunks.as_ref().unwrap();
    assert!(!chunks.is_empty(), "Should have at least one chunk");

    let chunks_with_pages = chunks.iter().filter(|c| c.metadata.first_page.is_some()).count();
    assert!(
        chunks_with_pages > 0,
        "At least some chunks should have page numbers, but none do. Total chunks: {}",
        chunks.len()
    );
}

#[tokio::test]
#[serial]
#[cfg(feature = "chunking")]
async fn test_pipeline_chunks_content_matches_output_format_markdown() {
    use crate::core::config::{ChunkerType, ChunkingConfig};
    use crate::types::internal::ElementKind;

    let mut doc = InternalDocument::new("plain");
    doc.mime_type = "text/plain".to_string();
    doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Section", 0));
    doc.push_element(InternalElement::text(
        ElementKind::Paragraph,
        "Body text for the section. ".repeat(10),
        0,
    ));

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        chunking: Some(ChunkingConfig {
            max_characters: 200,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            ..Default::default()
        }),
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = run_pipeline(doc, &config).await.unwrap();

    assert_eq!(result.metadata.output_format, Some("markdown".to_string()));
    assert!(
        result.content.contains('#'),
        "top-level content must contain markdown heading, got: {:?}",
        &result.content[..result.content.len().min(120)]
    );

    let chunks = result.chunks.expect("chunks must be produced");
    assert!(!chunks.is_empty(), "at least one chunk must be produced");
    let all_chunk_content: String = chunks.iter().map(|c| c.content.as_str()).collect::<Vec<_>>().join("\n");
    assert!(
        all_chunk_content.contains('#'),
        "chunks[].content must contain markdown syntax, got: {:?}",
        &all_chunk_content[..all_chunk_content.len().min(200)]
    );
}

#[test]
fn test_append_ocr_text_for_pptx_images() {
    use crate::types::ExtractedImage;
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    use std::borrow::Cow;

    let mut doc = InternalDocument::new("pptx");
    doc.append_ocr_text = true;
    doc.elements
        .push(InternalElement::text(ElementKind::Paragraph, "Before image.", 0));
    doc.elements.push(InternalElement::text(
        ElementKind::Paragraph,
        "![img](../media/image-1.jpeg)",
        0,
    ));
    doc.elements
        .push(InternalElement::text(ElementKind::Paragraph, "After image.", 0));

    doc.images.push(ExtractedImage {
        data: bytes::Bytes::new(),
        format: Cow::Borrowed("jpeg"),
        image_index: 0,
        page_number: Some(1),
        width: Some(100),
        height: Some(100),
        colorspace: None,
        bits_per_component: None,
        is_mask: false,
        description: None,
        ocr_result: Some(Box::new(crate::types::ExtractedDocument {
            content: "OCR text here".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        })),
        bounding_box: None,
        source_path: None,
        image_kind: None,
        kind_confidence: None,
        cluster_id: None,
        caption: None,
        qr_codes: None,
        data_base64: None,
    });

    let inline_images = super::images_without_an_element(&doc);
    super::append_embedded_image_ocr_text(&mut doc, &inline_images);

    assert_eq!(
        doc.elements.len(),
        4,
        "should have 4 elements (original 3 + 1 OCR paragraph)"
    );
    assert_eq!(doc.elements[2].text, "OCR text here");

    let rendered = crate::rendering::render_markdown(&doc);
    assert!(rendered.contains("OCR text here"));
}

/// How the OCR text of a picture reaches Markdown: after the placeholder by default, for every
/// source format, and one time for a picture that has an inline placeholder and an image element.
#[cfg(feature = "tokio-runtime")]
mod picture_ocr_text_rendering_tests {
    use super::{image_ocr_text_rendering, images_without_an_element, run_pipeline};
    use crate::core::config::{ExtractionConfig, ImageExtractionConfig, OutputFormat};
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    use crate::types::{ExtractedDocument, ExtractedImage};
    use serial_test::serial;

    const PICTURE_WORDS: &str = "CRATE 17 KEEP DRY";
    const OTHER_PICTURE_WORDS: &str = "PALLET 4 THIS SIDE UP";
    const INLINE_PLACEHOLDER: &str = "![label](label.png)";

    fn picture(ocr_text: &str) -> ExtractedImage {
        ExtractedImage {
            ocr_result: Some(Box::new(ExtractedDocument {
                content: ocr_text.to_string(),
                ..Default::default()
            })),
            ..Default::default()
        }
    }

    fn image_element(image_index: u32) -> InternalElement {
        InternalElement::text(ElementKind::Image { image_index }, "", 0)
    }

    /// The two inline steps as the pipeline runs them: with the pictures that nothing shows
    /// in the document as the extractor made it.
    fn append_embedded_image_ocr_text(document: &mut InternalDocument) {
        let inline_images = images_without_an_element(document);
        super::append_embedded_image_ocr_text(document, &inline_images);
    }

    fn replace_embedded_image_markdown_with_ocr(document: &mut InternalDocument) {
        let inline_images = images_without_an_element(document);
        super::replace_embedded_image_markdown_with_ocr(document, &inline_images);
    }

    /// A document as the DOCX extractor makes it: one paragraph and one image element.
    fn document_with_a_picture_element() -> InternalDocument {
        let mut document = InternalDocument::new("docx");
        document.push_element(InternalElement::text(ElementKind::Paragraph, "Stock list.", 0));
        document.push_element(image_element(0));
        document.images = vec![picture(PICTURE_WORDS)];
        document
    }

    fn markdown_config(images: Option<ImageExtractionConfig>) -> ExtractionConfig {
        ExtractionConfig {
            output_format: OutputFormat::Markdown,
            images,
            ..Default::default()
        }
    }

    #[test]
    fn a_missing_images_block_reads_as_the_default_block() {
        let default_block = ExtractionConfig {
            images: Some(ImageExtractionConfig::default()),
            ..Default::default()
        };
        assert_eq!(image_ocr_text_rendering(&ExtractionConfig::default()), (false, true));
        assert_eq!(image_ocr_text_rendering(&default_block), (false, true));

        let both_changed = ExtractionConfig {
            images: Some(ImageExtractionConfig {
                ocr_text_only: true,
                append_ocr_text: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(image_ocr_text_rendering(&both_changed), (true, false));
    }

    #[test]
    fn an_images_block_read_without_the_option_has_it_on() {
        let images: ImageExtractionConfig = serde_json::from_str("{}").expect("an empty images block is valid");
        assert!(images.append_ocr_text);
        assert!(!images.ocr_text_only);
    }

    #[tokio::test]
    #[serial]
    async fn markdown_has_the_picture_words_after_the_placeholder_by_default() {
        for (images, case) in [
            (None, "no images block"),
            (Some(ImageExtractionConfig::default()), "default images block"),
        ] {
            let result = run_pipeline(document_with_a_picture_element(), &markdown_config(images))
                .await
                .unwrap();

            let placeholder = result
                .content
                .find("![")
                .unwrap_or_else(|| panic!("{case}: {:?}", result.content));
            let words = result
                .content
                .find(PICTURE_WORDS)
                .unwrap_or_else(|| panic!("{case}: {:?}", result.content));
            assert!(placeholder < words, "{case}: {:?}", result.content);
            assert_eq!(
                result.content.matches(PICTURE_WORDS).count(),
                1,
                "{case}: {:?}",
                result.content
            );
        }
    }

    #[tokio::test]
    #[serial]
    async fn markdown_has_the_placeholder_alone_when_append_ocr_text_is_off() {
        let config = markdown_config(Some(ImageExtractionConfig {
            append_ocr_text: false,
            ..Default::default()
        }));

        let result = run_pipeline(document_with_a_picture_element(), &config).await.unwrap();

        assert!(result.content.contains("!["), "content: {:?}", result.content);
        assert_eq!(
            result.content.matches(PICTURE_WORDS).count(),
            0,
            "content: {:?}",
            result.content
        );
    }

    #[tokio::test]
    #[serial]
    async fn markdown_has_the_words_and_no_placeholder_with_ocr_text_only() {
        let config = markdown_config(Some(ImageExtractionConfig {
            ocr_text_only: true,
            ..Default::default()
        }));

        let result = run_pipeline(document_with_a_picture_element(), &config).await.unwrap();

        assert!(!result.content.contains("!["), "content: {:?}", result.content);
        assert_eq!(
            result.content.matches(PICTURE_WORDS).count(),
            1,
            "content: {:?}",
            result.content
        );
    }

    /// The HTML extractor makes an inline placeholder and an image element for one picture.
    #[tokio::test]
    #[serial]
    async fn a_picture_with_an_inline_placeholder_and_an_element_gives_its_words_once() {
        for format in [OutputFormat::Plain, OutputFormat::Markdown, OutputFormat::Djot] {
            for ocr_text_only in [false, true] {
                let mut document = InternalDocument::new("html");
                document.push_element(InternalElement::text(ElementKind::Paragraph, INLINE_PLACEHOLDER, 0));
                document.push_element(image_element(0));
                document.images = vec![picture(PICTURE_WORDS)];
                let config = ExtractionConfig {
                    output_format: format.clone(),
                    images: Some(ImageExtractionConfig {
                        ocr_text_only,
                        ..Default::default()
                    }),
                    ..Default::default()
                };

                let result = run_pipeline(document, &config).await.unwrap();

                assert_eq!(
                    result.content.matches(PICTURE_WORDS).count(),
                    1,
                    "{format:?}, ocr_text_only={ocr_text_only}: {:?}",
                    result.content
                );
            }
        }
    }

    #[test]
    fn an_inline_placeholder_pairs_with_the_picture_that_has_no_element() {
        let document = || {
            let mut document = InternalDocument::new("pptx");
            document.push_element(image_element(0));
            document.push_element(InternalElement::text(ElementKind::Paragraph, INLINE_PLACEHOLDER, 0));
            document.images = vec![picture(PICTURE_WORDS), picture(OTHER_PICTURE_WORDS)];
            document
        };

        let mut appended = document();
        appended.append_ocr_text = true;
        append_embedded_image_ocr_text(&mut appended);
        let texts: Vec<&str> = appended.elements.iter().map(|element| element.text.as_str()).collect();
        assert_eq!(texts, ["", INLINE_PLACEHOLDER, OTHER_PICTURE_WORDS]);

        let mut replaced = document();
        replaced.ocr_text_only = true;
        replace_embedded_image_markdown_with_ocr(&mut replaced);
        let texts: Vec<&str> = replaced.elements.iter().map(|element| element.text.as_str()).collect();
        assert_eq!(texts, ["", OTHER_PICTURE_WORDS]);
    }

    #[cfg(all(any(feature = "pdf", feature = "office"), feature = "ocr"))]
    #[tokio::test]
    #[serial]
    async fn an_inline_placeholder_does_not_take_the_words_of_a_picture_with_an_ocr_text_anchor() {
        for (images, case) in [
            (None, "no images block"),
            (
                Some(ImageExtractionConfig {
                    ocr_text_only: true,
                    ..Default::default()
                }),
                "ocr_text_only on",
            ),
        ] {
            let mut document = InternalDocument::new("pdf");
            document.push_element(InternalElement::text(ElementKind::Paragraph, INLINE_PLACEHOLDER, 0));
            document.push_element(InternalElement::image_ocr_text_anchor(0, None));
            document.images = vec![picture(PICTURE_WORDS)];
            let config = ExtractionConfig {
                images,
                ..Default::default()
            };

            let result = run_pipeline(document, &config).await.expect("the pipeline runs");

            assert!(
                result.content.contains(INLINE_PLACEHOLDER),
                "{case}: {:?}",
                result.content
            );
            assert_eq!(
                result.content.matches(PICTURE_WORDS).count(),
                1,
                "{case}: {:?}",
                result.content
            );
        }
    }

    #[test]
    fn appended_inline_picture_text_stays_on_the_placeholders_page() {
        let mut document = InternalDocument::new("pptx");
        document.append_ocr_text = true;
        document.push_element(InternalElement::text(ElementKind::Paragraph, INLINE_PLACEHOLDER, 3).with_page(2));
        document.images = vec![picture(PICTURE_WORDS)];

        append_embedded_image_ocr_text(&mut document);

        assert_eq!(document.elements[1].page, Some(2));
        assert_eq!(document.elements[1].depth, 3);
        let result = crate::extraction::derive::derive_extraction_result(document, false, OutputFormat::Markdown);
        let pages = result.pages.expect("the tagged page is derived");
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].page_number, 2);
        assert_eq!(pages[0].content.matches(PICTURE_WORDS).count(), 1);
    }

    #[test]
    fn an_inline_placeholder_in_a_table_cell_does_not_take_the_words_of_a_picture_with_an_element() {
        let document = || {
            let mut document = InternalDocument::new("html");
            document.push_element(image_element(0));
            document.images = vec![picture(PICTURE_WORDS)];
            document.tables.push(crate::types::Table {
                cells: vec![vec![INLINE_PLACEHOLDER.to_string()]],
                ..Default::default()
            });
            document
        };

        let mut appended = document();
        appended.append_ocr_text = true;
        append_embedded_image_ocr_text(&mut appended);
        assert_eq!(appended.tables[0].cells[0][0], INLINE_PLACEHOLDER);

        let mut replaced = document();
        replaced.ocr_text_only = true;
        replace_embedded_image_markdown_with_ocr(&mut replaced);
        assert_eq!(replaced.tables[0].cells[0][0], INLINE_PLACEHOLDER);
    }
}

#[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
mod full_page_image_ocr_tests {
    use bytes::Bytes;

    use super::{image_ocr_positions, should_skip_pdf_image_ocr};
    use crate::types::ExtractedImage;
    use crate::types::extraction::BoundingBox;
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    use crate::types::ocr_elements::OcrElementLevel;
    use crate::types::page::{PageInfo, PageStructure, PageUnitType};

    fn pdf_document() -> InternalDocument {
        let mut document = InternalDocument::new("pdf");
        document.mime_type = "application/pdf".to_string();
        document.metadata.pages = Some(PageStructure {
            total_count: 2,
            unit_type: PageUnitType::Page,
            boundaries: None,
            pages: Some(vec![page_info(1), page_info(2)]),
        });
        document
    }

    fn page_info(number: u32) -> PageInfo {
        PageInfo {
            number,
            title: None,
            dimensions: Some((100.0, 100.0).into()),
            image_count: None,
            table_count: None,
            hidden: None,
            is_blank: None,
            has_vector_graphics: false,
        }
    }

    fn image(image_index: u32, page_number: u32, bounding_box: BoundingBox) -> ExtractedImage {
        ExtractedImage {
            data: Bytes::new(),
            image_index,
            page_number: Some(page_number),
            bounding_box: Some(bounding_box),
            ..Default::default()
        }
    }

    fn full_page_box() -> BoundingBox {
        BoundingBox {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 100.0,
        }
    }

    #[test]
    fn should_skip_only_full_page_pdf_images_with_existing_page_text() {
        let mut document = pdf_document();
        document.push_element(
            InternalElement::text(
                ElementKind::OcrText {
                    level: OcrElementLevel::Block,
                },
                "page-level OCR text",
                0,
            )
            .with_page(1),
        );
        document.images = vec![
            image(0, 1, full_page_box()),
            image(
                1,
                1,
                BoundingBox {
                    x0: 10.0,
                    y0: 10.0,
                    x1: 40.0,
                    y1: 40.0,
                },
            ),
            image(2, 2, full_page_box()),
        ];

        assert_eq!(image_ocr_positions(&document), vec![1, 2]);
    }

    #[test]
    fn should_return_no_ocr_work_when_every_image_repeats_page_text() {
        let mut document = pdf_document();
        document.push_element(InternalElement::text(ElementKind::Paragraph, "already extracted", 0).with_page(1));
        document.images = vec![image(0, 1, full_page_box())];

        assert!(image_ocr_positions(&document).is_empty());
    }

    #[test]
    fn should_return_no_ocr_work_for_page_rasters_without_bounding_boxes() {
        let mut document = pdf_document();
        document.push_element(
            InternalElement::text(
                ElementKind::OcrText {
                    level: OcrElementLevel::Block,
                },
                "page-level OCR text",
                0,
            )
            .with_page(1),
        );
        document.images = vec![ExtractedImage {
            data: Bytes::from_static(b"page raster"),
            image_index: 0,
            page_number: Some(1),
            bounding_box: None,
            image_kind: Some(crate::types::ImageKind::PageRaster),
            ..Default::default()
        }];

        assert_eq!(image_ocr_positions(&document), Vec::<usize>::new());
        assert!(document.images[0].ocr_result.is_none());
    }

    #[test]
    fn should_not_apply_pdf_deduplication_to_other_formats() {
        let mut document = pdf_document();
        document.source_format = "pptx".to_string();
        document.push_element(InternalElement::text(ElementKind::Paragraph, "slide text", 0).with_page(1));
        let full_page_image = image(0, 1, full_page_box());

        assert!(!should_skip_pdf_image_ocr(&document, &full_page_image));
    }
}

/// Smoke tests for `apply_output_format_pass`.
///
/// These operate directly on `ExtractedDocument` without invoking the full extractor,
/// proving the pass executes correctly when called at the pipeline level.
#[cfg(feature = "image-encode")]
mod output_format_pass_tests {
    use std::borrow::Cow;
    use std::io::Cursor;

    use bytes::Bytes;
    use image::{DynamicImage, ImageFormat};

    use crate::core::config::extraction::{ImageExtractionConfig, ImageOutputFormat};
    use crate::types::{ExtractedDocument, ExtractedImage};

    use super::{apply_output_format_pass, apply_output_format_pass_with_security_limits};

    fn make_jpeg_bytes() -> Bytes {
        use image::codecs::jpeg::JpegEncoder;
        let img = image::RgbImage::new(8, 8);
        let mut buf: Vec<u8> = Vec::new();
        JpegEncoder::new_with_quality(&mut buf, 85)
            .encode_image(&DynamicImage::ImageRgb8(img))
            .expect("test JPEG encode");
        Bytes::from(buf)
    }

    fn make_png_bytes() -> Bytes {
        let img = image::RgbImage::new(8, 8);
        let mut buf: Vec<u8> = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .expect("test PNG encode");
        Bytes::from(buf)
    }

    fn make_image(data: Bytes, format: &'static str) -> ExtractedImage {
        ExtractedImage {
            data,
            format: Cow::Borrowed(format),
            ..Default::default()
        }
    }

    /// Both decodable images are re-encoded to PNG; no warnings are pushed.
    #[test]
    fn both_images_re_encoded_to_png_no_warnings() {
        let mut result = ExtractedDocument {
            images: Some(vec![
                make_image(make_jpeg_bytes(), "jpeg"),
                make_image(make_png_bytes(), "png"),
            ]),
            ..Default::default()
        };

        let cfg = ImageExtractionConfig {
            output_format: ImageOutputFormat::Png,
            ..Default::default()
        };

        apply_output_format_pass(&mut result, &cfg);

        let images = result.images.as_ref().expect("images must be present");
        assert_eq!(images[0].format.as_ref(), "png", "jpeg must be re-encoded to png");
        assert_eq!(images[1].format.as_ref(), "png", "already-png must remain png");
        assert!(
            result.processing_warnings.is_empty(),
            "no warnings expected for decodable images; got: {:?}",
            result.processing_warnings
        );
    }

    /// Without the `svg` feature: an SVG image is left untouched and a
    /// `ProcessingWarning` is pushed for it (it is an untranslatable format).
    #[cfg(not(feature = "svg"))]
    #[test]
    fn svg_image_skipped_with_warning() {
        let svg_bytes = Bytes::from_static(b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>");
        let original_svg = svg_bytes.clone();

        let mut result = ExtractedDocument {
            images: Some(vec![
                make_image(make_jpeg_bytes(), "jpeg"),
                make_image(svg_bytes, "svg"),
            ]),
            ..Default::default()
        };

        let cfg = ImageExtractionConfig {
            output_format: ImageOutputFormat::Png,
            ..Default::default()
        };

        apply_output_format_pass(&mut result, &cfg);

        let images = result.images.as_ref().expect("images must be present");
        assert_eq!(images[0].format.as_ref(), "png", "jpeg must be re-encoded");
        assert_eq!(images[1].format.as_ref(), "svg", "svg must be untouched");
        assert_eq!(images[1].data, original_svg, "svg bytes must be untouched");

        assert_eq!(result.processing_warnings.len(), 1, "one warning for svg");
        assert_eq!(
            result.processing_warnings[0].source.as_ref(),
            "image_encoder",
            "warning source must be image_encoder"
        );
    }

    /// With the `svg` feature: an SVG image is rasterized to the target format
    /// (PNG here) via `resvg`/`usvg`.  No warning is pushed — the encode succeeds.
    #[cfg(feature = "svg")]
    #[test]
    fn svg_image_skipped_with_warning() {
        let svg_bytes = Bytes::from_static(b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>");

        let mut result = ExtractedDocument {
            images: Some(vec![
                make_image(make_jpeg_bytes(), "jpeg"),
                make_image(svg_bytes, "svg"),
            ]),
            ..Default::default()
        };

        let cfg = ImageExtractionConfig {
            output_format: ImageOutputFormat::Png,
            ..Default::default()
        };

        apply_output_format_pass(&mut result, &cfg);

        let images = result.images.as_ref().expect("images must be present");
        assert_eq!(images[0].format.as_ref(), "png", "jpeg must be re-encoded to png");
        assert_eq!(images[1].format.as_ref(), "png", "svg must be rasterized to png");
        assert!(
            result.processing_warnings.is_empty(),
            "no warnings expected when svg is rasterized successfully; got: {:?}",
            result.processing_warnings
        );
    }

    /// When output_format is Native the pass is a no-op.
    #[test]
    fn native_target_is_no_op() {
        let original = make_jpeg_bytes();
        let mut result = ExtractedDocument {
            images: Some(vec![make_image(original.clone(), "jpeg")]),
            ..Default::default()
        };

        let cfg = ImageExtractionConfig {
            output_format: ImageOutputFormat::Native,
            ..Default::default()
        };

        apply_output_format_pass(&mut result, &cfg);

        let images = result.images.as_ref().expect("images must be present");
        assert_eq!(images[0].data, original, "bytes must be untouched for Native");
        assert_eq!(images[0].format.as_ref(), "jpeg", "format must be untouched");
        assert!(result.processing_warnings.is_empty());
    }

    #[test]
    fn request_security_limit_reaches_output_encoder() {
        let original = make_png_bytes();
        let mut result = ExtractedDocument {
            images: Some(vec![make_image(original.clone(), "png")]),
            ..Default::default()
        };
        let config = ImageExtractionConfig {
            output_format: ImageOutputFormat::Jpeg { quality: 85 },
            ..Default::default()
        };
        let limits = crate::extractors::security::SecurityLimits {
            max_content_size: 100,
            ..Default::default()
        };

        apply_output_format_pass_with_security_limits(&mut result, &config, Some(&limits));

        let image = &result.images.as_ref().expect("image result")[0];
        assert_eq!(
            image.data, original,
            "rejected output conversion must preserve source data"
        );
        assert_eq!(image.format.as_ref(), "png");
        assert_eq!(result.processing_warnings.len(), 1);
        assert!(
            result.processing_warnings[0]
                .message
                .contains("security_limits.max_content_size")
        );
    }
}

/// Unit tests for `apply_data_base64_pass`.
///
/// Directly exercises the private pass without going through the full extractor,
/// mirroring the approach used by `output_format_pass_tests` above.
mod data_base64_pass_tests {
    use std::borrow::Cow;

    use base64::Engine as _;
    use bytes::Bytes;

    use crate::core::config::extraction::ImageExtractionConfig;
    use crate::types::{ExtractedDocument, ExtractedImage};

    use super::apply_data_base64_pass;

    fn make_image(data: Bytes) -> ExtractedImage {
        ExtractedImage {
            data,
            format: Cow::Borrowed("png"),
            ..Default::default()
        }
    }

    /// When `include_data_base64` is `true` every image's `data_base64` must be
    /// `Some(base64::STANDARD.encode(image.data))`.
    #[test]
    fn include_data_base64_true_encodes_all_images() {
        let first_bytes = Bytes::from_static(b"\x89PNG\r\n\x1a\n");
        let second_bytes = Bytes::from_static(b"\xff\xd8\xff");

        let mut result = ExtractedDocument {
            images: Some(vec![make_image(first_bytes.clone()), make_image(second_bytes.clone())]),
            ..Default::default()
        };

        let cfg = ImageExtractionConfig {
            include_data_base64: true,
            ..Default::default()
        };

        apply_data_base64_pass(&mut result, &cfg);

        let images = result.images.as_ref().expect("images must be present");
        assert_eq!(
            images[0].data_base64,
            Some(base64::engine::general_purpose::STANDARD.encode(&first_bytes)),
            "first image data_base64 must be the STANDARD-encoded bytes"
        );
        assert_eq!(
            images[1].data_base64,
            Some(base64::engine::general_purpose::STANDARD.encode(&second_bytes)),
            "second image data_base64 must be the STANDARD-encoded bytes"
        );
    }

    /// When `include_data_base64` is `false` (the default) no image must have
    /// its `data_base64` field populated.
    #[test]
    fn include_data_base64_false_leaves_field_none() {
        let mut result = ExtractedDocument {
            images: Some(vec![
                make_image(Bytes::from_static(b"\x89PNG\r\n\x1a\n")),
                make_image(Bytes::from_static(b"\xff\xd8\xff")),
            ]),
            ..Default::default()
        };

        let cfg = ImageExtractionConfig {
            include_data_base64: false,
            ..Default::default()
        };

        apply_data_base64_pass(&mut result, &cfg);

        let images = result.images.as_ref().expect("images must be present");
        for (idx, image) in images.iter().enumerate() {
            assert_eq!(
                image.data_base64, None,
                "image[{idx}].data_base64 must remain None when include_data_base64 is false"
            );
        }
    }
}

// #1576: `images.run_ocr_on_images` (per-extracted-image OCR) is a different setting from
// document-level page OCR. A test here used to re-derive the buggy suppression logic inline
// (`config.images.map(|i| i.run_ocr_on_images).unwrap_or(false)`) and assert that
// `run_ocr_on_images=true` suppressed `RunFallback` -- that assertion WAS the bug, not the
// contract. `extractors/pdf/mod.rs`'s `OcrGateOutcome::RunFallback` arm no longer reads
// `config.images` at all, so there is nothing left to unit-test at that granularity; the
// regression coverage is an end-to-end extraction in
// `extractors::pdf::tests::images_config_does_not_suppress_scanned_page_ocr`. Left as a plain
// comment, not a doc comment: it documents a removed test, not the module below it. ~keep

mod document_counts {
    use super::super::populate_document_counts;
    use crate::types::page::{PageContent, PageStructure, PageUnitType};
    use crate::types::{ExtractedDocument, ExtractedImage, Metadata, Table};

    fn page_structure(total_count: u32) -> PageStructure {
        PageStructure {
            total_count,
            unit_type: PageUnitType::Page,
            boundaries: None,
            pages: None,
        }
    }

    fn page(page_number: u32) -> PageContent {
        PageContent {
            page_number,
            content: String::new(),
            tables: Vec::new(),
            image_indices: Vec::new(),
            image_preprocessing: None,
            hierarchy: None,
            is_blank: None,
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
            ocr_confidence: None,
            native_content: None,
        }
    }

    #[test]
    fn pages_come_from_metadata_page_count() {
        let mut result = ExtractedDocument {
            metadata: Metadata {
                pages: Some(page_structure(5)),
                ..Default::default()
            },
            tables: vec![Table::default(), Table::default()],
            images: Some(vec![ExtractedImage::default()]),
            pages: None,
            ..Default::default()
        };
        populate_document_counts(&mut result, 0);
        assert_eq!(result.counts.pages, 5, "pages must read metadata.total_count");
        assert_eq!(result.counts.tables, 2);
        assert_eq!(result.counts.images, 1);
    }

    #[test]
    fn pages_fall_back_to_materialized_pages_len() {
        let mut result = ExtractedDocument {
            metadata: Metadata::default(),
            pages: Some(vec![page(1), page(2), page(3)]),
            ..Default::default()
        };
        populate_document_counts(&mut result, 0);
        assert_eq!(result.counts.pages, 3);
        assert_eq!(result.counts.tables, 0);
        assert_eq!(result.counts.images, 0);
    }

    #[test]
    fn non_paginated_input_reports_zero_pages() {
        let mut result = ExtractedDocument {
            content: "plain text".to_string(),
            ..Default::default()
        };
        populate_document_counts(&mut result, 0);
        assert_eq!(result.counts.pages, 0);
        assert_eq!(result.counts.tables, 0);
        assert_eq!(result.counts.images, 0);
    }

    #[test]
    fn zero_metadata_page_count_falls_back_to_pages_len() {
        let mut result = ExtractedDocument {
            metadata: Metadata {
                pages: Some(page_structure(0)),
                ..Default::default()
            },
            pages: Some(vec![page(1), page(2)]),
            ..Default::default()
        };
        populate_document_counts(&mut result, 0);
        assert_eq!(result.counts.pages, 2);
    }

    /// GH#1888: a scanned PDF extracted with OCR disabled and no `PageConfig` builds
    /// neither `metadata.pages` (no page boundaries were tracked) nor `pages` (no
    /// per-page content was collected), so both of the fallbacks above fall through --
    /// yet `metadata.format`'s PDF page count was already read from the page tree
    /// during metadata extraction, independent of page tracking. `populate_document_counts`
    /// must fall back to it instead of reporting `0` for a document that plainly has pages.
    #[test]
    #[cfg(feature = "pdf")]
    fn pages_fall_back_to_pdf_format_metadata_page_count() {
        use crate::pdf::metadata::PdfMetadata;
        use crate::types::FormatMetadata;

        let mut result = ExtractedDocument {
            metadata: Metadata {
                pages: None,
                format: Some(FormatMetadata::Pdf(PdfMetadata {
                    page_count: Some(3),
                    ..Default::default()
                })),
                ..Default::default()
            },
            pages: None,
            ..Default::default()
        };
        populate_document_counts(&mut result, 0);
        assert_eq!(
            result.counts.pages, 3,
            "counts.pages must fall back to metadata.format's PDF page_count (GH#1888)"
        );
    }

    /// Non-PDF format metadata carries no page count of its own, so the fallback must not
    /// invent one -- the final `unwrap_or(0)` tier still applies.
    #[test]
    #[cfg(feature = "pdf")]
    fn non_pdf_format_metadata_does_not_supply_a_fallback_page_count() {
        use crate::types::{ExcelMetadata, FormatMetadata};

        let mut result = ExtractedDocument {
            metadata: Metadata {
                pages: None,
                format: Some(FormatMetadata::Excel(ExcelMetadata::default())),
                ..Default::default()
            },
            pages: None,
            ..Default::default()
        };
        populate_document_counts(&mut result, 0);
        assert_eq!(result.counts.pages, 0);
    }
}

/// GH#1662, GH#1752: `ExtractionConfig::runs_ocr_on_embedded_images` must be the ONLY
/// expression of "do embedded images get OCR'd". This module used to re-derive it from
/// `images.run_ocr_on_images` and `ocr.is_some()`, while `needs_image_data` -- the READ gate a
/// container consults before it reads an embedded image's bytes at all -- was defined against
/// the method. The two copies drifted, a container attached an image with an empty buffer, and
/// the OCR path reported `Could not determine image format` on the zero bytes. A comment did
/// not prevent that, so assert the structural property: no executable line of this module
/// mentions `run_ocr_on_images` at all, and the method is still called. ~keep
#[test]
fn embedded_image_ocr_gate_has_no_second_copy() {
    const PIPELINE_SOURCE: &str = include_str!("mod.rs");

    let rederived: Vec<(usize, &str)> = PIPELINE_SOURCE
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with("//") && line.contains("run_ocr_on_images"))
        .map(|(index, line)| (index + 1, line.trim()))
        .collect();
    assert!(
        rederived.is_empty(),
        "core/pipeline/mod.rs must call ExtractionConfig::runs_ocr_on_embedded_images() rather than \
         re-deriving the embedded-image OCR gate (GH#1662); found {rederived:?}"
    );

    assert!(
        PIPELINE_SOURCE.contains("config.runs_ocr_on_embedded_images()"),
        "core/pipeline/mod.rs must still gate embedded-image OCR on runs_ocr_on_embedded_images()"
    );
}

/// `fold_embedded_image_ocr_text`: the OCR text of an embedded picture enters the element list
/// and the prebuilt page content before any renderer runs.
#[cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]
mod embedded_image_ocr_text_fold_tests {
    use super::fold_embedded_image_ocr_text;
    use crate::types::internal::{
        ElementKind, InternalDocument, InternalElement, Relationship, RelationshipKind, RelationshipTarget,
    };
    use crate::types::page::PageContent;
    use crate::types::{ExtractedDocument, ExtractedImage};

    const NATIVE_TEXT: &str = "Stock list for the north depot.";
    const PICTURE_WORDS: &str = "CRATE 17 KEEP DRY";

    fn picture(page_number: u32, ocr_text: Option<&str>) -> ExtractedImage {
        ExtractedImage {
            page_number: Some(page_number),
            ocr_result: ocr_text.map(|text| {
                Box::new(ExtractedDocument {
                    content: text.to_string(),
                    ..Default::default()
                })
            }),
            ..Default::default()
        }
    }

    fn paragraph(text: &str, page_number: Option<u32>) -> InternalElement {
        let mut element = InternalElement::text(ElementKind::Paragraph, text, 0);
        element.page = page_number;
        element
    }

    fn image_element(image_index: u32, page_number: u32) -> InternalElement {
        InternalElement::text(ElementKind::Image { image_index }, "", 0).with_page(page_number)
    }

    fn text_only_anchor(image_index: u32, page_number: u32) -> InternalElement {
        InternalElement::image_ocr_text_anchor(image_index, Some(page_number))
    }

    fn page_break() -> InternalElement {
        InternalElement::text(ElementKind::PageBreak, "", 0)
    }

    fn page(page_number: u32, content: &str) -> PageContent {
        PageContent {
            page_number,
            content: content.to_string(),
            tables: Vec::new(),
            image_indices: Vec::new(),
            image_preprocessing: None,
            hierarchy: None,
            is_blank: None,
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
            ocr_confidence: None,
            native_content: None,
        }
    }

    /// One page of native text with one picture, the picture's element built by `anchor`.
    fn one_page_document(anchor: InternalElement, ocr_text: Option<&str>) -> InternalDocument {
        let mut document = InternalDocument::new("pdf");
        document.elements = vec![paragraph(NATIVE_TEXT, Some(1)), anchor];
        document.images = vec![picture(1, ocr_text)];
        document.prebuilt_pages = Some(vec![page(1, NATIVE_TEXT)]);
        document
    }

    fn page_contents(document: &InternalDocument) -> Vec<&str> {
        document
            .prebuilt_pages
            .as_ref()
            .expect("the document has prebuilt pages")
            .iter()
            .map(|page| page.content.as_str())
            .collect()
    }

    fn kinds_and_texts(document: &InternalDocument) -> Vec<(ElementKind, &str)> {
        document
            .elements
            .iter()
            .map(|element| (element.kind, element.text.as_str()))
            .collect()
    }

    #[test]
    fn a_text_only_anchor_with_text_becomes_one_paragraph_on_its_page() {
        let mut document = one_page_document(text_only_anchor(0, 1), Some(PICTURE_WORDS));

        fold_embedded_image_ocr_text(&mut document);

        assert_eq!(
            kinds_and_texts(&document),
            vec![
                (ElementKind::Paragraph, NATIVE_TEXT),
                (ElementKind::Paragraph, PICTURE_WORDS)
            ]
        );
        assert_eq!(document.elements[1].page, Some(1));
        assert_eq!(document.elements[1].image_ocr_text_anchor_index(), None);
        assert_eq!(document.elements[1].public_attributes(), None);
        assert_eq!(
            page_contents(&document),
            vec![format!("{NATIVE_TEXT}\n\n{PICTURE_WORDS}")]
        );
    }

    #[test]
    fn a_text_only_anchor_without_text_is_removed() {
        for ocr_text in [None, Some("")] {
            let mut document = one_page_document(text_only_anchor(0, 1), ocr_text);

            fold_embedded_image_ocr_text(&mut document);

            assert_eq!(kinds_and_texts(&document), vec![(ElementKind::Paragraph, NATIVE_TEXT)]);
            assert_eq!(page_contents(&document), vec![NATIVE_TEXT]);
        }
    }

    #[test]
    fn a_placeholder_element_stays_and_its_page_gains_the_text_once() {
        let mut document = one_page_document(image_element(0, 1), Some(PICTURE_WORDS));

        fold_embedded_image_ocr_text(&mut document);

        assert_eq!(
            kinds_and_texts(&document),
            vec![
                (ElementKind::Paragraph, NATIVE_TEXT),
                (ElementKind::Image { image_index: 0 }, "")
            ]
        );
        assert_eq!(
            page_contents(&document),
            vec![format!("{NATIVE_TEXT}\n\n{PICTURE_WORDS}")]
        );
    }

    #[test]
    fn a_page_with_no_content_gains_the_text_without_a_separator() {
        let mut document = one_page_document(image_element(0, 1), Some(PICTURE_WORDS));
        document.prebuilt_pages = Some(vec![page(1, "")]);

        fold_embedded_image_ocr_text(&mut document);

        assert_eq!(page_contents(&document), vec![PICTURE_WORDS]);
    }

    #[test]
    fn the_text_goes_to_the_page_of_the_picture_only() {
        let mut document = one_page_document(text_only_anchor(0, 2), Some(PICTURE_WORDS));
        document.images = vec![picture(2, Some(PICTURE_WORDS))];
        document.prebuilt_pages = Some(vec![page(1, NATIVE_TEXT), page(2, "")]);

        fold_embedded_image_ocr_text(&mut document);

        assert_eq!(page_contents(&document), vec![NATIVE_TEXT, PICTURE_WORDS]);
        assert_eq!(document.elements[1].page, Some(2));
    }

    #[test]
    fn a_suppressed_image_element_adds_nothing_to_its_page() {
        let mut element = image_element(0, 1);
        element.suppress_image_ocr_rendering();
        let mut document = one_page_document(element, Some(PICTURE_WORDS));

        fold_embedded_image_ocr_text(&mut document);

        assert_eq!(document.elements.len(), 2, "an image element is never removed");
        assert_eq!(page_contents(&document), vec![NATIVE_TEXT]);
    }

    /// A paragraph that only looks like an anchor, with no text and no anchor attribute, stays.
    #[test]
    fn an_empty_paragraph_that_is_no_anchor_stays() {
        let mut document = one_page_document(paragraph("", Some(1)), Some(PICTURE_WORDS));

        fold_embedded_image_ocr_text(&mut document);

        assert_eq!(
            kinds_and_texts(&document),
            vec![(ElementKind::Paragraph, NATIVE_TEXT), (ElementKind::Paragraph, "")]
        );
        assert_eq!(page_contents(&document), vec![NATIVE_TEXT]);
    }

    #[test]
    fn a_removed_anchor_leaves_each_relationship_on_its_elements() {
        let mut document = InternalDocument::new("pdf");
        document.elements = vec![
            paragraph("Figure 1", Some(1)),
            text_only_anchor(0, 1),
            paragraph(NATIVE_TEXT, Some(1)),
            paragraph("See figure 1", Some(1)),
        ];
        document.images = vec![picture(1, None)];
        document.relationships = vec![
            Relationship {
                source: 3,
                target: RelationshipTarget::Index(0),
                kind: RelationshipKind::InternalLink,
            },
            Relationship {
                source: 0,
                target: RelationshipTarget::Index(1),
                kind: RelationshipKind::Caption,
            },
            Relationship {
                source: 0,
                target: RelationshipTarget::Index(3),
                kind: RelationshipKind::InternalLink,
            },
        ];

        fold_embedded_image_ocr_text(&mut document);

        assert_eq!(document.elements.len(), 3);
        assert_eq!(
            document.relationships,
            vec![
                Relationship {
                    source: 2,
                    target: RelationshipTarget::Index(0),
                    kind: RelationshipKind::InternalLink,
                },
                Relationship {
                    source: 0,
                    target: RelationshipTarget::Index(2),
                    kind: RelationshipKind::InternalLink,
                },
            ],
            "each link moves with its source and its target, and the caption of the removed picture goes with it"
        );
        assert_eq!(document.elements[2].text, "See figure 1");
    }

    #[test]
    fn a_page_that_held_only_a_removed_anchor_leaves_no_page_break() {
        let first = paragraph("First page.", Some(1));
        let last = paragraph("Last page.", Some(3));
        let cases: Vec<(Vec<InternalElement>, Vec<ElementKind>)> = vec![
            (
                vec![
                    first.clone(),
                    page_break(),
                    text_only_anchor(0, 2),
                    page_break(),
                    last.clone(),
                ],
                vec![ElementKind::Paragraph, ElementKind::PageBreak, ElementKind::Paragraph],
            ),
            (
                vec![text_only_anchor(0, 1), page_break(), last.clone()],
                vec![ElementKind::Paragraph],
            ),
            (
                vec![first.clone(), page_break(), text_only_anchor(0, 2)],
                vec![ElementKind::Paragraph],
            ),
            (
                vec![first.clone(), page_break(), page_break(), last.clone()],
                vec![
                    ElementKind::Paragraph,
                    ElementKind::PageBreak,
                    ElementKind::PageBreak,
                    ElementKind::Paragraph,
                ],
            ),
        ];

        for (elements, expected) in cases {
            let mut document = InternalDocument::new("pdf");
            document.elements = elements;
            document.images = vec![picture(2, None)];

            fold_embedded_image_ocr_text(&mut document);

            let kinds: Vec<ElementKind> = document.elements.iter().map(|element| element.kind).collect();
            assert_eq!(kinds, expected);
        }
    }

    #[test]
    fn a_page_that_keeps_its_paragraph_keeps_its_page_break() {
        let mut document = InternalDocument::new("pdf");
        document.elements = vec![paragraph(NATIVE_TEXT, Some(1)), page_break(), text_only_anchor(0, 2)];
        document.images = vec![picture(2, Some(PICTURE_WORDS))];

        fold_embedded_image_ocr_text(&mut document);

        assert_eq!(
            kinds_and_texts(&document),
            vec![
                (ElementKind::Paragraph, NATIVE_TEXT),
                (ElementKind::PageBreak, ""),
                (ElementKind::Paragraph, PICTURE_WORDS)
            ]
        );
    }

    /// Page markers follow the page breaks, so they are made after the fold: a page that held
    /// only a picture with no text gets no marker, as it gets none without the picture.
    #[tokio::test]
    async fn a_page_that_held_only_a_removed_anchor_gets_no_page_marker() {
        let mut document = InternalDocument::new("pdf");
        document.elements = vec![
            paragraph("First page.", Some(1)),
            page_break(),
            text_only_anchor(0, 2),
            page_break(),
            paragraph("Last page.", Some(3)),
        ];
        document.images = vec![picture(2, None)];
        let config = crate::core::config::ExtractionConfig {
            pages: Some(crate::core::config::PageConfig {
                insert_page_markers: true,
                marker_format: "[page {page_num}]".to_string(),
                ..Default::default()
            }),
            postprocessor: Some(crate::core::config::PostProcessorConfig {
                enabled: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = super::run_pipeline(document, &config).await.expect("the pipeline runs");

        assert!(result.content.contains("[page 1]"), "{:?}", result.content);
        assert!(result.content.contains("[page 3]"), "{:?}", result.content);
        assert!(!result.content.contains("[page 2]"), "{:?}", result.content);
    }

    /// The paragraphs of a flat document can have no page tag. A formatted page is rendered
    /// from the elements tagged with it, so a tag on the picture's paragraph alone would
    /// replace the native text of the page with the picture's words.
    #[test]
    fn a_formatted_page_keeps_its_native_text_when_the_paragraphs_have_no_page_tag() {
        let mut document = one_page_document(text_only_anchor(0, 1), Some(PICTURE_WORDS));
        document.elements[0].page = None;

        fold_embedded_image_ocr_text(&mut document);
        assert_eq!(document.elements[1].page, None);
        let result = crate::extraction::derive::derive_extraction_result(
            document,
            false,
            crate::core::config::OutputFormat::Markdown,
        );

        let pages = result.pages.expect("the prebuilt pages are returned");
        assert_eq!(
            pages[0].content.matches(NATIVE_TEXT).count(),
            1,
            "{:?}",
            pages[0].content
        );
        assert_eq!(
            pages[0].content.matches(PICTURE_WORDS).count(),
            1,
            "{:?}",
            pages[0].content
        );
    }
}

/// `image_ocr_positions` for a document from the image extractor: the picture is the file
/// itself, and the extractor has already read it with OCR.
#[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
mod image_file_ocr_eligibility_tests {
    use super::image_ocr_positions;
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    use crate::types::{ExtractedImage, Table};

    /// A document from `source_format` with these paragraphs, one image element and its picture.
    fn document(source_format: &str, paragraphs: &[&str]) -> InternalDocument {
        let mut document = InternalDocument::new(source_format);
        for text in paragraphs {
            document.push_element(InternalElement::text(ElementKind::Paragraph, *text, 0));
        }
        document.push_element(InternalElement::text(ElementKind::Image { image_index: 0 }, "", 0));
        document.images = vec![ExtractedImage::default()];
        document
    }

    fn with_table(mut document: InternalDocument, rows: &[&[&str]]) -> InternalDocument {
        let table_index = document.push_table(Table {
            cells: rows
                .iter()
                .map(|row| row.iter().map(|cell| (*cell).to_string()).collect())
                .collect(),
            ..Default::default()
        });
        document.push_element(InternalElement::text(ElementKind::Table { table_index }, "", 0));
        document
    }

    #[test]
    fn an_image_file_with_text_sends_no_picture_to_image_ocr() {
        assert!(image_ocr_positions(&document("image", &["CRATE 17 KEEP DRY"])).is_empty());
    }

    #[test]
    fn an_image_file_with_a_table_and_no_text_sends_no_picture_to_image_ocr() {
        let table_only = with_table(document("image", &[]), &[&["Item", "Bay"], &["PALLET", "NORTH"]]);

        assert!(image_ocr_positions(&table_only).is_empty());
    }

    #[test]
    fn an_image_file_with_no_text_and_no_table_keeps_its_picture_eligible() {
        assert_eq!(image_ocr_positions(&document("image", &[])), vec![0]);
        assert_eq!(image_ocr_positions(&document("image", &[" \n"])), vec![0]);
        assert_eq!(
            image_ocr_positions(&with_table(document("image", &[]), &[&["", " "]])),
            vec![0]
        );
    }

    /// The text of an image element describes the picture. It is not a read of the file.
    #[test]
    fn an_image_file_with_text_on_its_image_element_alone_keeps_its_picture_eligible() {
        let mut described = InternalDocument::new("image");
        described.push_element(InternalElement::text(
            ElementKind::Image { image_index: 0 },
            "CRATE 17 KEEP DRY",
            0,
        ));
        described.images = vec![ExtractedImage::default()];

        assert_eq!(image_ocr_positions(&described), vec![0]);
        assert!(image_ocr_positions(&document("image", &["CRATE 17 KEEP DRY"])).is_empty());
    }

    /// A picture inside another kind of document is not the document, whatever text it has.
    #[test]
    fn a_picture_inside_another_document_with_text_stays_eligible() {
        assert_eq!(image_ocr_positions(&document("docx", &["CRATE 17 KEEP DRY"])), vec![0]);
        assert_eq!(
            image_ocr_positions(&with_table(document("docx", &[]), &[&["PALLET", "NORTH"]])),
            vec![0]
        );
    }
}
