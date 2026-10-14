use std::borrow::Cow;
use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;

use tempfile::tempdir;

use super::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::core::config::concurrency::LayoutBatchWorkload;

#[path = "tests/batch_plan_and_url.rs"]
mod batch_plan_and_url;

#[test]
fn extraction_cache_namespaces_invalidate_pre_f32_pdf_results() {
    assert_eq!(CACHE_KEY_NAMESPACE, b"xberg-engine-extract-v2");
    assert_eq!(BATCH_CACHE_KEY_NAMESPACE, b"xberg-engine-extract-batch-v2");
}

#[test]
fn effective_input_config_borrows_base_when_input_has_no_overrides() {
    let base = ExtractionConfig::default();
    let input = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None);

    let effective = effective_input_config(&input, &base);

    assert!(matches!(effective, Cow::Borrowed(_)));
    assert!(std::ptr::eq(effective.as_ref(), &base));
}

#[test]
fn effective_input_config_owns_merged_config_when_input_has_overrides() {
    let base = ExtractionConfig::default();
    let input = ExtractInput {
        config: Some(crate::core::config::FileExtractionConfig {
            force_ocr: Some(true),
            ..Default::default()
        }),
        ..ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None)
    };

    let effective = effective_input_config(&input, &base);

    assert!(matches!(effective, Cow::Owned(_)));
    assert!(effective.force_ocr);
}

#[test]
fn effective_input_config_preserves_cancelled_token_without_installing_one() {
    let token = crate::cancellation::CancellationToken::new();
    token.cancel();
    let base = ExtractionConfig {
        extraction_timeout_secs: Some(30),
        cancel_token: Some(token),
        ..Default::default()
    };
    let input = ExtractInput {
        config: Some(crate::core::config::FileExtractionConfig {
            force_ocr: Some(true),
            ..Default::default()
        }),
        ..ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None)
    };

    let effective = effective_input_config(&input, &base);

    assert!(matches!(effective, Cow::Owned(_)));
    assert!(
        effective
            .cancel_token
            .as_ref()
            .is_some_and(|token| token.is_cancelled())
    );

    let without_token = ExtractionConfig {
        extraction_timeout_secs: Some(30),
        cancel_token: None,
        ..Default::default()
    };
    let input_without_overrides = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None);
    let effective = effective_input_config(&input_without_overrides, &without_token);
    assert!(matches!(effective, Cow::Borrowed(_)));
    assert!(effective.cancel_token.is_none());
}

#[test]
fn content_cache_key_ignores_the_internal_timeout_cancellation_token() {
    let input = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None);
    let without_token = ExtractionConfig {
        extraction_timeout_secs: Some(30),
        ..Default::default()
    };
    let mut with_token = without_token.clone();
    with_token.ensure_cancel_token();

    assert_eq!(
        content_cache_key(&input, &without_token),
        content_cache_key(&input, &with_token)
    );
}

#[test]
fn content_cache_key_changes_when_per_input_config_changes() {
    let base = ExtractionConfig::default();
    let without_override = ExtractInput::from_bytes(b"same bytes".to_vec(), "text/plain", None);
    let with_override = ExtractInput {
        config: Some(crate::core::config::FileExtractionConfig {
            force_ocr: Some(true),
            ..Default::default()
        }),
        ..without_override.clone()
    };

    assert_ne!(
        content_cache_key(&without_override, &base),
        content_cache_key(&with_override, &base),
        "a per-input override must participate in the content cache key"
    );
}

#[test]
fn content_cache_key_changes_with_keep_native_content() {
    let input = ExtractInput::from_bytes(b"same bytes".to_vec(), "application/pdf", None);
    let pages = |keep_native_content: bool| ExtractionConfig {
        pages: Some(crate::core::config::PageConfig {
            extract_pages: true,
            keep_native_content,
            ..Default::default()
        }),
        ..Default::default()
    };

    let off = content_cache_key(&input, &pages(false));
    assert!(off.is_some(), "a bytes input with the cache on has a key");
    assert_ne!(off, content_cache_key(&input, &pages(true)));
}

#[test]
fn a_per_input_override_with_keep_native_content_needs_extract_pages() {
    let with_pages = |pages: crate::core::config::PageConfig| ExtractInput {
        config: Some(crate::core::config::FileExtractionConfig {
            pages: Some(pages),
            ..Default::default()
        }),
        ..ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None)
    };
    let rejected = with_pages(crate::core::config::PageConfig {
        keep_native_content: true,
        ..Default::default()
    });
    let accepted = with_pages(crate::core::config::PageConfig {
        extract_pages: true,
        keep_native_content: true,
        ..Default::default()
    });
    let no_override = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None);

    let error = validate_input_overrides(&[no_override.clone(), rejected])
        .expect_err("an override with the setting on and extract_pages off is invalid");
    assert!(matches!(error, XbergError::Validation { .. }), "got: {error:?}");
    validate_input_overrides(&[no_override, accepted]).expect("a valid override passes");
}

#[tokio::test]
async fn extract_bytes_input_returns_envelope() {
    let config = ExtractionConfig::default();
    let output = crate::engine::Engine::new_default()
        .extract(ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None), &config)
        .await
        .unwrap();

    assert_eq!(output.results.len(), 1);
    assert_eq!(output.summary.inputs, 1);
    assert_eq!(output.summary.results, 1);
    assert_eq!(output.results[0].content.trim(), "hello");
}

#[tokio::test]
async fn extract_local_uri_returns_envelope() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("doc.txt");
    File::create(&path).unwrap().write_all(b"hello path").unwrap();

    let config = ExtractionConfig::default();
    let output = crate::engine::Engine::new_default()
        .extract(ExtractInput::from_uri(path.to_string_lossy()), &config)
        .await
        .unwrap();

    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].content.trim(), "hello path");
}

#[tokio::test]
async fn extract_file_uri_returns_envelope() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("doc.txt");
    File::create(&path).unwrap().write_all(b"hello file uri").unwrap();

    let config = ExtractionConfig::default();
    let output = crate::engine::Engine::new_default()
        .extract(ExtractInput::from_uri(format!("file://{}", path.display())), &config)
        .await
        .unwrap();

    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].content.trim(), "hello file uri");
}

#[tokio::test]
async fn extract_rejects_local_path_when_policy_disallows_it() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("doc.txt");
    File::create(&path).unwrap().write_all(b"hello local policy").unwrap();

    let mut config = ExtractionConfig::default();
    config.url.allow_local_file_inputs = false;
    let error = crate::engine::Engine::new_default()
        .extract(ExtractInput::from_uri(path.to_string_lossy()), &config)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("local filesystem path inputs are disabled"));
}

#[tokio::test]
async fn extract_rejects_non_local_file_uri_host() {
    let config = ExtractionConfig::default();
    let error = crate::engine::Engine::new_default()
        .extract(ExtractInput::from_uri("file://evilhost/tmp/doc.txt"), &config)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("unsupported non-local file URI host"));
}

#[tokio::test]
async fn extract_file_uri_accepts_localhost_host() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("doc.txt");
    File::create(&path)
        .unwrap()
        .write_all(b"hello localhost file uri")
        .unwrap();

    let config = ExtractionConfig::default();
    let output = crate::engine::Engine::new_default()
        .extract(
            ExtractInput::from_uri(format!("file://localhost{}", path.display())),
            &config,
        )
        .await
        .unwrap();

    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].content.trim(), "hello localhost file uri");
}

#[tokio::test]
async fn extract_rejects_unsupported_scheme() {
    let config = ExtractionConfig::default();
    let error = crate::engine::Engine::new_default()
        .extract(ExtractInput::from_uri("s3://bucket/file.txt"), &config)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("unsupported URI scheme"));
}

#[tokio::test]
async fn extract_batch_collects_mixed_inputs() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("doc.txt");
    File::create(&path).unwrap().write_all(b"hello batch path").unwrap();

    let config = ExtractionConfig::default();
    let output = crate::engine::Engine::new_default()
        .extract_batch(
            vec![
                ExtractInput::from_bytes(b"hello batch bytes".to_vec(), "text/plain", None),
                ExtractInput::from_uri(path.to_string_lossy()),
            ],
            &config,
        )
        .await
        .unwrap();

    assert_eq!(output.results.len(), 2);
    assert_eq!(output.summary.inputs, 2);
    assert!(output.errors.is_empty());
}

#[tokio::test]
async fn extract_batch_collects_unsupported_scheme_error() {
    let config = ExtractionConfig::default();
    let output = crate::engine::Engine::new_default()
        .extract_batch(
            vec![
                ExtractInput::from_bytes(b"hello batch bytes".to_vec(), "text/plain", None),
                ExtractInput::from_uri("s3://bucket/doc.txt"),
            ],
            &config,
        )
        .await
        .unwrap();

    assert_eq!(output.results.len(), 1);
    assert_eq!(output.errors.len(), 1);
    assert_eq!(output.summary.inputs, 2);
    assert_eq!(output.summary.results, 1);
    assert_eq!(output.summary.errors, 1);
    assert_eq!(output.errors[0].index, 1);
    assert_eq!(output.errors[0].code, 1010);
    assert_eq!(output.errors[0].error_type, "unsupported_format");
}

#[tokio::test]
async fn extract_batch_applies_item_timeout() {
    let item = run_batch_item(0, "<test>".to_string(), Some(1), None, || async {
        std::future::pending::<()>().await;
        Ok(ExtractionResult::default())
    })
    .await;

    let error = item.result.unwrap_err();
    assert_eq!(error_code(&error), 1014);
    assert_eq!(error_type(&error), "timeout");
}

#[test]
fn should_map_every_variant_to_its_canonical_code_and_type() {
    let cases: [(XbergError, u32, &str); 18] = [
        (XbergError::Io(std::io::Error::other("t")), 1000, "io"),
        (XbergError::parsing("t"), 1001, "parsing"),
        (XbergError::ocr("t"), 1002, "ocr"),
        (XbergError::validation("t"), 1003, "validation"),
        (XbergError::cache("t"), 1004, "cache"),
        (XbergError::image_processing("t"), 1005, "image_processing"),
        (XbergError::serialization("t"), 1006, "serialization"),
        (
            XbergError::MissingDependency("t".to_string()),
            1007,
            "missing_dependency",
        ),
        (
            XbergError::Plugin {
                message: "t".to_string(),
                plugin_name: "p".to_string(),
            },
            1008,
            "plugin",
        ),
        (XbergError::LockPoisoned("t".to_string()), 1009, "lock_poisoned"),
        (
            XbergError::UnsupportedFormat("t/mime".to_string()),
            1010,
            "unsupported_format",
        ),
        (XbergError::embedding("t"), 1011, "embedding"),
        (XbergError::reranking("t"), 1012, "reranking"),
        (XbergError::transcription("t"), 1013, "transcription"),
        (
            XbergError::Timeout {
                elapsed_ms: 1,
                limit_ms: 2,
            },
            1014,
            "timeout",
        ),
        (XbergError::Cancelled, 1015, "cancelled"),
        (XbergError::security("t"), 1016, "security"),
        (XbergError::Other("t".to_string()), 1017, "other"),
    ];

    for (error, expected_code, expected_type) in cases {
        assert_eq!(error_code(&error), expected_code);
        assert_eq!(error_type(&error), expected_type);
    }
}

#[tokio::test]
async fn batch_scheduler_prioritizes_larger_inputs() {
    let directory = tempdir().unwrap();
    let large_path = directory.path().join("large.pdf");
    let mut large_file = File::create(&large_path).unwrap();
    large_file.write_all(&[0; 32]).unwrap();

    let pending = [
        (0, ExtractInput::from_bytes([0], "application/pdf", None), "small"),
        (1, ExtractInput::from_uri(large_path.to_string_lossy()), "large"),
        (2, ExtractInput::from_bytes([0; 8], "application/pdf", None), "medium"),
    ]
    .into_iter()
    .map(|(index, input, source)| (index, input, source.to_string()))
    .collect();

    let scheduled = prioritize_pending_batch_items(pending, &ExtractionConfig::default()).await;

    assert_eq!(
        scheduled.iter().map(|(index, _, _)| *index).collect::<Vec<_>>(),
        [1, 2, 0]
    );
}

#[tokio::test]
async fn batch_scheduler_respects_per_input_local_policy() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("large.pdf");
    File::create(&path).unwrap().write_all(&[0; 32]).unwrap();
    let mut denied = ExtractInput::from_uri(path.to_string_lossy());
    denied.config = Some(crate::core::config::FileExtractionConfig {
        url: Some(crate::core::config::UrlExtractionConfig {
            allow_local_file_inputs: false,
            ..Default::default()
        }),
        ..Default::default()
    });
    let pending = [
        (0, ExtractInput::from_bytes([0], "application/pdf", None), "small"),
        (1, denied, "denied"),
        (2, ExtractInput::from_bytes([0; 8], "application/pdf", None), "medium"),
    ]
    .into_iter()
    .map(|(index, input, source)| (index, input, source.to_string()))
    .collect();

    let scheduled = prioritize_pending_batch_items(pending, &ExtractionConfig::default()).await;

    assert_eq!(
        scheduled.iter().map(|(index, _, _)| *index).collect::<Vec<_>>(),
        [2, 1, 0]
    );
}

#[tokio::test]
async fn batch_scheduler_preserves_tie_order_and_remote_slots() {
    let pending = [
        (0, ExtractInput::from_bytes([0; 8], "application/pdf", None), "first"),
        (1, ExtractInput::from_uri("https://example.com/a.pdf"), "remote"),
        (2, ExtractInput::from_bytes([0; 32], "application/pdf", None), "large"),
        (3, ExtractInput::from_bytes([0; 8], "application/pdf", None), "second"),
    ]
    .into_iter()
    .map(|(index, input, source)| (index, input, source.to_string()))
    .collect();

    let scheduled = prioritize_pending_batch_items(pending, &ExtractionConfig::default()).await;

    assert_eq!(
        scheduled.iter().map(|(index, _, _)| *index).collect::<Vec<_>>(),
        [2, 1, 0, 3]
    );
}

#[test]
fn batch_scheduler_prioritizes_only_when_work_will_queue() {
    assert!(!should_prioritize_pending_batch_items(4, 1));
    assert!(!should_prioritize_pending_batch_items(4, 4));
    assert!(should_prioritize_pending_batch_items(5, 4));
}

#[test]
fn batch_scheduler_does_not_probe_disallowed_local_inputs() {
    let bare = ExtractInput::from_uri("/private/automount/doc.pdf");
    let file_uri = ExtractInput::from_uri("file:///private/automount/doc.pdf");
    let mut config = ExtractionConfig::default();
    config.url.allow_local_file_inputs = false;
    config.url.allow_file_uris = false;

    assert_eq!(local_batch_path(&bare, &config), None);
    assert_eq!(local_batch_path(&file_uri, &config), None);
}

#[tokio::test]
async fn batch_scheduler_restores_public_result_order_after_prioritizing() {
    let directory = tempdir().unwrap();
    let contents = ["small".to_string(), "large ".repeat(32), "medium medium".to_string()];
    let mut inputs = Vec::new();
    for (index, content) in contents.iter().enumerate() {
        let path = directory.path().join(format!("{index}.txt"));
        File::create(&path).unwrap().write_all(content.as_bytes()).unwrap();
        inputs.push(ExtractInput::from_uri(path.to_string_lossy()));
    }

    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(2),
            max_concurrent_ocr: None,
        }),
        max_concurrent_extractions: Some(2),
        ..Default::default()
    };
    let output = crate::engine::Engine::new_default()
        .extract_batch(inputs, &config)
        .await
        .unwrap();

    assert!(output.errors.is_empty());
    assert_eq!(
        output
            .results
            .iter()
            .map(|document| document.content.trim())
            .collect::<Vec<_>>(),
        contents.iter().map(|content| content.trim()).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn bounded_batch_scheduler_caps_in_flight_tasks() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let pending = (0..8).collect::<VecDeque<_>>();
    let completed = run_bounded_batch_tasks(pending, 2, {
        let active = Arc::clone(&active);
        let peak = Arc::clone(&peak);
        move |index| {
            let active = Arc::clone(&active);
            let peak = Arc::clone(&peak);
            async move {
                let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                active.fetch_sub(1, Ordering::SeqCst);
                BatchItemResult {
                    index,
                    source: index.to_string(),
                    result: Ok(ExtractionResult::default()),
                }
            }
        }
    })
    .await
    .unwrap();

    assert_eq!(completed.len(), 8);
    assert_eq!(peak.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn batch_input_progress_scope_reports_zero_based_item_indices() {
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct IndexSink(Mutex<Vec<usize>>);

    impl crate::engine::seams::ProgressSink for IndexSink {
        fn emit(&self, _event: crate::engine::seams::ProgressEvent) {}

        fn emit_ocr_page(
            &self,
            _page: usize,
            _total: usize,
            _completed: usize,
            _backend: &str,
            input_index: Option<usize>,
        ) {
            self.0
                .lock()
                .expect("progress index mutex poisoned")
                .push(input_index.expect("batch item should carry its index"));
        }
    }

    let sink = Arc::new(IndexSink::default());
    for index in 0..2 {
        scope_batch_input_progress(sink.clone(), index, async {
            crate::engine::seams::emit_ocr_page(1, 1, "tesseract");
        })
        .await;
    }

    assert_eq!(*sink.0.lock().expect("progress index mutex poisoned"), vec![0, 1]);
}

#[tokio::test]
async fn bounded_batch_scheduler_preserves_completion_and_error_indices() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let stage = Arc::new(AtomicUsize::new(0));
    let pending = (0..3).collect::<VecDeque<_>>();
    let completed = run_bounded_batch_tasks(pending, 3, {
        let stage = Arc::clone(&stage);
        move |index| {
            let stage = Arc::clone(&stage);
            async move {
                let prerequisite = match index {
                    0 => 2,
                    2 => 1,
                    _ => 0,
                };
                while stage.load(Ordering::SeqCst) < prerequisite {
                    tokio::task::yield_now().await;
                }
                stage.fetch_add(1, Ordering::SeqCst);
                let result = if index == 1 {
                    Err(XbergError::Other("indexed failure".to_string()))
                } else {
                    Ok(ExtractionResult::default())
                };
                BatchItemResult {
                    index,
                    source: index.to_string(),
                    result,
                }
            }
        }
    })
    .await
    .unwrap();

    assert_eq!(completed.iter().map(|item| item.index).collect::<Vec<_>>(), [1, 2, 0]);
    assert!(completed[0].result.is_err());
    assert_eq!(completed[0].source, "1");
}

#[test]
#[cfg(layout_detection)]
fn engine_batch_execution_plan_matches_layout_aware_resolution() {
    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(4),
            max_concurrent_ocr: None,
        }),
        ..Default::default()
    };
    let non_layout = resolve_engine_batch_execution_plan_for(&config, LayoutBatchWorkload::None, 8);
    assert_eq!(non_layout.workers, 4);
    assert_eq!(non_layout.thread_budget, 1);
    let layout = resolve_engine_batch_execution_plan_for(&config, LayoutBatchWorkload::All, 8);
    assert_eq!(layout.workers, 1);
    assert_eq!(layout.thread_budget, 4);

    let explicit = ExtractionConfig {
        max_concurrent_extractions: Some(2),
        ..config
    };
    let layout_explicit = resolve_engine_batch_execution_plan_for(&explicit, LayoutBatchWorkload::All, 8);
    assert_eq!(layout_explicit.workers, 1);
    assert_eq!(layout_explicit.thread_budget, 4);
    let non_layout_explicit = resolve_engine_batch_execution_plan_for(&explicit, LayoutBatchWorkload::None, 8);
    assert_eq!(non_layout_explicit.workers, 2);
    assert_eq!(non_layout_explicit.thread_budget, 2);
}

#[test]
fn engine_batch_base_config_applies_plan_budget_once() {
    let base = Arc::new(ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: None,
        }),
        ..Default::default()
    });

    let adjusted = resolve_batch_base_config(&base, 2);
    assert_eq!(
        adjusted.concurrency.as_ref().and_then(|config| config.max_threads),
        Some(2)
    );
    assert!(!Arc::ptr_eq(&base, &adjusted));

    let reused = resolve_batch_base_config(&adjusted, 2);
    assert!(Arc::ptr_eq(&adjusted, &reused));
}

/// Only the thread budget is divided across batch workers. `max_concurrent_ocr`
/// is a bound on host memory, which every worker shares, so dividing it would be
/// wrong and dropping it silently returns the caller to the automatic limit --
/// the exact setting they reached for `max_concurrent_ocr` to escape.
#[test]
fn engine_batch_base_config_carries_the_configured_recognition_limit() {
    let base = Arc::new(ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: Some(3),
        }),
        ..Default::default()
    });

    let adjusted = resolve_batch_base_config(&base, 2);
    let concurrency = adjusted
        .concurrency
        .as_ref()
        .expect("batch config keeps a concurrency block");
    assert_eq!(concurrency.max_threads, Some(2), "the thread budget is the divided one");
    assert_eq!(
        concurrency.max_concurrent_ocr,
        Some(3),
        "the caller's recognition limit survives the budget rewrite"
    );
}

/// Regression test for task #709: `resolve_input_config` is the single choke point
/// both `extract_one` (the single-input `extract`/`extract_batch_sequential` path) and
/// the shared-URL-group construction in `extract_batch_concurrent` go through.
/// Installing the internal cancellation token here — before either `run_batch_item`'s
/// or `finalize_shared_item`'s own timeout wrapper races the extraction — guarantees
/// `token.cancel()` at those call sites has the SAME token this config's extractor
/// checkpoints observe, not a disconnected one.
#[test]
fn resolve_input_config_installs_a_cancel_token_when_a_timeout_is_configured() {
    let base = ExtractionConfig {
        extraction_timeout_secs: Some(30),
        cancel_token: None,
        ..Default::default()
    };
    let input = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None);

    let resolved = resolve_input_config(&input, &base);

    assert!(
        resolved.cancel_token.is_some(),
        "resolve_input_config must install an internal token when a timeout is configured"
    );
}

/// A caller-supplied token (the REST cancel path, `DELETE /jobs/{id}`) must survive
/// `resolve_input_config` unchanged, so it keeps working exactly as before.
#[test]
fn resolve_input_config_preserves_a_caller_supplied_cancel_token() {
    let supplied = crate::cancellation::CancellationToken::new();
    let base = ExtractionConfig {
        extraction_timeout_secs: Some(30),
        cancel_token: Some(supplied.clone()),
        ..Default::default()
    };
    let input = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None);

    let resolved = resolve_input_config(&input, &base);

    supplied.cancel();
    assert!(
        resolved.cancel_token.expect("token must survive").is_cancelled(),
        "resolve_input_config must keep the caller's own token (a clone of the same Arc), \
         not swap in an unrelated one that never observes the caller's cancel() call"
    );
}

/// Companion to `resolve_input_config`'s tests above, for the concurrent-batch path:
/// `resolve_batch_input_config` feeds both `run_batch_item`'s cancel_token argument and
/// the `resolved_config` passed to `extract_one_resolved`, so it must install the same
/// guarantee without breaking the existing Arc-reuse fast path when nothing changed.
#[test]
fn resolve_batch_input_config_shares_the_arc_when_nothing_needs_installing() {
    let base = Arc::new(ExtractionConfig {
        extraction_timeout_secs: None,
        cancel_token: None,
        ..Default::default()
    });
    let input = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None);
    let thread_budget = crate::core::config::concurrency::resolve_thread_budget(base.concurrency.as_ref());

    let resolved = resolve_batch_input_config(&input, &base, thread_budget);

    assert!(
        Arc::ptr_eq(&resolved, &base),
        "no timeout, no cancel_token need, and no thread-budget change must reuse the \
         SAME Arc, not clone the config"
    );
}

#[test]
fn resolve_batch_input_config_installs_a_cancel_token_when_a_timeout_is_configured() {
    let base = Arc::new(ExtractionConfig {
        extraction_timeout_secs: Some(30),
        cancel_token: None,
        ..Default::default()
    });
    let input = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None);
    let thread_budget = crate::core::config::concurrency::resolve_thread_budget(base.concurrency.as_ref());

    let resolved = resolve_batch_input_config(&input, &base, thread_budget);

    assert!(
        resolved.cancel_token.is_some(),
        "resolve_batch_input_config must install an internal token when a timeout is \
         configured, even though nothing else forced a clone"
    );
}

/// The per-input companion to `engine_batch_base_config_carries_the_configured_recognition_limit`.
/// Both resolvers rewrite the concurrency block when the batch divides the thread
/// budget, and `resolve_batch_input_config` is the one every batch item goes
/// through, so a caller's `max_concurrent_ocr` has to survive the rewrite here
/// too. Dropping it returns recognition to the automatic limit, which is the
/// setting the caller reached for the field to escape.
#[test]
fn resolve_batch_input_config_carries_the_configured_recognition_limit() {
    let base = Arc::new(ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: Some(3),
        }),
        ..Default::default()
    });
    let input = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None);

    let resolved = resolve_batch_input_config(&input, &base, 2);

    let concurrency = resolved
        .concurrency
        .as_ref()
        .expect("batch config keeps a concurrency block");
    assert_eq!(concurrency.max_threads, Some(2), "the thread budget is the divided one");
    assert_eq!(
        concurrency.max_concurrent_ocr,
        Some(3),
        "the caller's recognition limit survives the budget rewrite"
    );
}

/// An input that carries its own overrides takes the clone-and-merge path rather
/// than the `Arc::clone` fast path, and the budget rewrite then runs on the merged
/// config. `FileExtractionConfig` has no concurrency block of its own, so the
/// base's recognition limit is the one that must come out the far side.
#[test]
fn resolve_batch_input_config_carries_the_recognition_limit_through_a_file_override() {
    let base = Arc::new(ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: Some(3),
        }),
        ..Default::default()
    });
    let mut input = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None);
    input.config = Some(crate::core::config::FileExtractionConfig {
        force_ocr: Some(true),
        ..Default::default()
    });

    let resolved = resolve_batch_input_config(&input, &base, 2);

    assert!(resolved.force_ocr, "the per-input override is applied");
    assert_eq!(
        resolved.concurrency.as_ref().and_then(|c| c.max_concurrent_ocr),
        Some(3),
        "the recognition limit survives the override merge and the budget rewrite"
    );
}
