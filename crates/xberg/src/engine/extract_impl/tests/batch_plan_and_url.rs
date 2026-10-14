use super::*;

#[test]
fn engine_batch_execution_plan_clamps_explicit_zero_to_one() {
    let config = ExtractionConfig {
        max_concurrent_extractions: Some(0),
        ..Default::default()
    };

    assert_eq!(
        resolve_engine_batch_execution_plan_for(&config, LayoutBatchWorkload::None, 8).workers,
        1
    );
}

#[test]
fn engine_batch_execution_plan_without_layout_respects_input_count() {
    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(4),
            max_concurrent_ocr: None,
        }),
        ..Default::default()
    };
    let inputs = vec![ExtractInput::default()];

    assert_eq!(resolve_engine_batch_execution_plan(&config, &inputs).workers, 1);
}

#[cfg(layout_detection)]
#[test]
fn engine_batch_classifies_all_markdown_pdfs_for_single_layout_worker() {
    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: None,
        }),
        layout: Some(Default::default()),
        use_layout_for_markdown: true,
        disable_ocr: true,
        ..Default::default()
    };
    let inputs = vec![ExtractInput::from_uri("document.pdf"); 4];

    assert_eq!(classify_layout_batch(&config, &inputs), LayoutBatchWorkload::All);
    let plan = resolve_engine_batch_execution_plan(&config, &inputs);
    assert_eq!(plan.workers, 1);
    assert_eq!(plan.thread_budget, 8);
}

#[cfg(layout_detection)]
#[test]
fn engine_batch_classifies_disabled_layout_as_none_when_ocr_is_disabled() {
    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: None,
        }),
        layout: Some(Default::default()),
        use_layout_for_markdown: false,
        disable_ocr: true,
        ..Default::default()
    };
    let inputs = vec![ExtractInput::from_uri("document.pdf"); 4];

    assert_eq!(classify_layout_batch(&config, &inputs), LayoutBatchWorkload::None);
    assert_eq!(resolve_engine_batch_execution_plan(&config, &inputs).workers, 4);
}

#[cfg(layout_detection)]
#[test]
fn engine_batch_classifies_partial_input_layout_override_as_mixed() {
    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: None,
        }),
        use_layout_for_markdown: true,
        disable_ocr: true,
        ..Default::default()
    };
    let layout_input = ExtractInput {
        config: Some(crate::core::config::FileExtractionConfig {
            layout: Some(Default::default()),
            ..Default::default()
        }),
        ..ExtractInput::from_uri("layout.pdf")
    };
    let inputs = vec![
        layout_input,
        ExtractInput::from_uri("plain.pdf"),
        ExtractInput::from_uri("plain.pdf"),
        ExtractInput::from_uri("plain.pdf"),
    ];

    assert_eq!(classify_layout_batch(&config, &inputs), LayoutBatchWorkload::Mixed);
    let plan = resolve_engine_batch_execution_plan(&config, &inputs);
    assert_eq!(plan.workers, 2);
    assert_eq!(plan.thread_budget, 4);
}

#[cfg(layout_detection)]
#[test]
fn engine_batch_classifies_ocr_capable_layout_as_mixed() {
    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: None,
        }),
        layout: Some(Default::default()),
        use_layout_for_markdown: false,
        disable_ocr: false,
        ..Default::default()
    };
    let inputs = vec![ExtractInput::from_uri("image.png"); 4];

    assert_eq!(classify_layout_batch(&config, &inputs), LayoutBatchWorkload::Mixed);
    assert_eq!(resolve_engine_batch_execution_plan(&config, &inputs).workers, 2);
}

#[cfg(layout_detection)]
#[test]
fn engine_batch_classifies_ordinary_batch_as_non_layout() {
    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: None,
        }),
        ..Default::default()
    };
    let inputs = vec![ExtractInput::from_uri("document.txt"); 4];

    assert_eq!(classify_layout_batch(&config, &inputs), LayoutBatchWorkload::None);
    assert_eq!(resolve_engine_batch_execution_plan(&config, &inputs).workers, 4);
}

#[cfg(all(layout_detection, feature = "url-ingestion"))]
#[test]
fn engine_batch_plan_ignores_shared_url_count_and_layout_overrides() {
    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: None,
        }),
        ..Default::default()
    };
    let shared = ExtractInput {
        config: Some(crate::core::config::FileExtractionConfig {
            layout: Some(Default::default()),
            ..Default::default()
        }),
        ..ExtractInput::from_uri("https://example.com/document.pdf")
    };
    assert!(shared_group_uri(&shared).is_some());

    let local = ExtractInput::from_uri("local.pdf");
    let all_plan = resolve_engine_batch_execution_plan(&config, &[shared, local.clone()]);
    assert_eq!(all_plan.workers, 2);
    assert_eq!(all_plan.thread_budget, 4);

    let pending = VecDeque::from([(1, local, "local.pdf".to_string())]);
    let pending_plan = resolve_pending_batch_execution_plan(&config, &pending);
    assert_eq!(pending_plan.workers, 1);
    assert_eq!(pending_plan.thread_budget, 8);
}

#[cfg(layout_detection)]
#[test]
fn engine_batch_concurrency_detects_per_input_layout_override() {
    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(4),
            max_concurrent_ocr: None,
        }),
        ..Default::default()
    };
    let inputs = vec![ExtractInput {
        config: Some(crate::core::config::FileExtractionConfig {
            layout: Some(Default::default()),
            ..Default::default()
        }),
        ..Default::default()
    }];

    let plan = resolve_engine_batch_execution_plan(&config, &inputs);
    assert_eq!(plan.workers, 1);
    assert_eq!(plan.thread_budget, 4);
}

#[cfg(feature = "url-ingestion")]
#[tokio::test]
async fn url_markdown_page_runs_through_pipeline_and_preserves_source_mime() {
    let config = ExtractionConfig::default();
    let links = vec![ExtractedUri {
        url: "https://example.com/next".to_string(),
        label: Some("next".to_string()),
        page: None,
        kind: UriKind::Hyperlink,
    }];

    let result = run_url_page_pipeline(
        "alpha beta gamma delta epsilon zeta eta theta".to_string(),
        true,
        "text/html; charset=utf-8",
        "",
        links,
        &config,
    )
    .await
    .unwrap();

    assert_eq!(result.mime_type, "text/html");
    assert_eq!(result.metadata.output_format.as_deref(), Some("plain"));
    assert_eq!(result.uris.as_ref().map(Vec::len), Some(1));
}

#[cfg(feature = "url-ingestion")]
#[tokio::test]
async fn url_page_rejects_untrusted_content_type_as_public_mime() {
    let result = run_url_page_pipeline(
        "safe content".to_string(),
        true,
        "text/html\r\nx-injected: value",
        "",
        Vec::new(),
        &ExtractionConfig::default(),
    )
    .await
    .unwrap();

    assert_eq!(result.mime_type, "text/html");
}

/// GH CI E2E `test_metadata_access`: a crawled page is restamped `text/html`, so
/// `metadata.format.html` must be populated even though the extraction itself ran over
/// crawlberg's pre-rendered markdown and never touched the HTML extractor.
#[cfg(all(feature = "url-ingestion", feature = "html"))]
#[tokio::test]
async fn url_html_page_recovers_format_metadata_from_source_html_when_content_is_markdown() {
    let source_html = "<html><head><title>Simple Table Test</title></head><body><h1>Heading</h1></body></html>";

    let result = run_url_page_pipeline(
        "# Heading\n\nalpha beta gamma".to_string(),
        true,
        "text/html",
        source_html,
        Vec::new(),
        &ExtractionConfig::default(),
    )
    .await
    .unwrap();

    assert_eq!(result.mime_type, "text/html");
    let Some(crate::types::FormatMetadata::Html(html_metadata)) = result.metadata.format else {
        panic!("expected FormatMetadata::Html; got {:?}", result.metadata.format);
    };
    assert_eq!(html_metadata.title.as_deref(), Some("Simple Table Test"));
}

/// Negative control for the test above: with no source HTML to recover from, the format field
/// stays `None` rather than being invented.
#[cfg(all(feature = "url-ingestion", feature = "html"))]
#[tokio::test]
async fn url_page_without_source_html_leaves_format_metadata_unset() {
    let result = run_url_page_pipeline(
        "alpha beta gamma".to_string(),
        true,
        "text/html",
        "",
        Vec::new(),
        &ExtractionConfig::default(),
    )
    .await
    .unwrap();

    assert!(
        result.metadata.format.is_none(),
        "format must stay None with no HTML to read; got {:?}",
        result.metadata.format
    );
}

#[cfg(feature = "tree-sitter")]
#[tokio::test]
async fn extract_py_local_uri_returns_source_code_mime() {
    use crate::core::config::TreeSitterConfig;

    let dir = tempdir().unwrap();
    let path = dir.path().join("hello.py");
    File::create(&path)
        .unwrap()
        .write_all(b"def greet(name):\n    return f'Hello, {name}!'\n")
        .unwrap();

    let config = ExtractionConfig {
        tree_sitter: Some(TreeSitterConfig::default()),
        ..Default::default()
    };

    let output = crate::engine::Engine::new_default()
        .extract(ExtractInput::from_uri(path.to_string_lossy()), &config)
        .await
        .unwrap();

    assert_eq!(output.results.len(), 1, "expected one result");
    assert_eq!(
        output.results[0].mime_type, "text/x-source-code",
        "Python file must extract as text/x-source-code"
    );
    assert!(output.results[0].content.len() >= 5, "content must be non-trivial");
}

#[test]
fn downloaded_specific_http_mime_remains_authoritative_under_every_policy() {
    for policy in [
        crate::MimeDetectionPolicy::PreferContent,
        crate::MimeDetectionPolicy::TrustExtension,
        crate::MimeDetectionPolicy::ContentOnly,
    ] {
        let config = ExtractionConfig {
            mime_detection_policy: policy,
            ..Default::default()
        };
        let resolved = resolve_bytes_mime_type(
            Some("application/pdf"),
            Some("document.txt"),
            br#"{"kind":"json"}"#,
            &config,
        )
        .unwrap();
        assert_eq!(resolved, "application/pdf", "unexpected MIME for {policy:?}");
    }
}

#[test]
fn downloaded_octet_stream_uses_policy_with_derived_filename() {
    let cases = [
        (crate::MimeDetectionPolicy::PreferContent, "application/json"),
        (crate::MimeDetectionPolicy::TrustExtension, "text/plain"),
        (crate::MimeDetectionPolicy::ContentOnly, "application/json"),
    ];
    for (policy, expected) in cases {
        let config = ExtractionConfig {
            mime_detection_policy: policy,
            ..Default::default()
        };
        let resolved = resolve_bytes_mime_type(
            Some("application/octet-stream"),
            Some("download.txt"),
            br#"{"kind":"json"}"#,
            &config,
        )
        .unwrap();
        assert_eq!(resolved, expected, "unexpected MIME for {policy:?}");
    }
}

#[test]
fn prefer_content_bytes_falls_back_from_unsupported_specialized_extension_to_plain_text() {
    let config = ExtractionConfig {
        mime_detection_policy: crate::MimeDetectionPolicy::PreferContent,
        ..Default::default()
    };

    let resolved = resolve_bytes_mime_type(None, Some("feed.atom"), b"ordinary prose without markup", &config).unwrap();

    assert_eq!(resolved, "text/plain");
}

#[test]
fn content_only_bytes_ignores_a_supported_filename_extension() {
    let config = ExtractionConfig {
        mime_detection_policy: crate::MimeDetectionPolicy::ContentOnly,
        ..Default::default()
    };

    let resolved = resolve_bytes_mime_type(None, Some("document.txt"), br#"{"kind":"content"}"#, &config).unwrap();

    assert_eq!(resolved, "application/json");
}

#[cfg(feature = "tree-sitter")]
#[test]
fn generic_text_keeps_tree_sitter_content_detection_without_bypassing_trusted_extensions() {
    use crate::core::config::TreeSitterConfig;

    let source = b"#!/usr/bin/env python3\nprint('hello')\n";
    let mut config = ExtractionConfig {
        tree_sitter: Some(TreeSitterConfig::default()),
        ..Default::default()
    };
    let detected = resolve_bytes_mime_type(None, Some("script.unknown"), source, &config).unwrap();
    assert_eq!(detected, "text/x-source-code");

    config.mime_detection_policy = crate::MimeDetectionPolicy::TrustExtension;
    let trusted = resolve_bytes_mime_type(None, Some("script.txt"), source, &config).unwrap();
    assert_eq!(trusted, "text/plain");
}

/// Regression: a shared-URL batch result that maps to no input slot (e.g.
/// crawlberg drops a panicked task as an empty-URL pair) must NOT cause its
/// input to vanish. The sweep fills every unfilled slot with an error so
/// `results + errors == inputs` always holds.
#[cfg(all(feature = "tokio-runtime", feature = "url-ingestion"))]
#[test]
fn fill_dropped_shared_slots_reattaches_or_synthesizes_errors() {
    use std::collections::VecDeque;

    let shared_items = vec![
        SharedUrlItem {
            index: 0,
            source: "http://a/".into(),
            uri: "http://a/".into(),
            config: ExtractionConfig::default(),
        },
        SharedUrlItem {
            index: 1,
            source: "http://b/".into(),
            uri: "http://b/".into(),
            config: ExtractionConfig::default(),
        },
        SharedUrlItem {
            index: 2,
            source: "http://c/".into(),
            uri: "http://c/".into(),
            config: ExtractionConfig::default(),
        },
    ];
    let mut items: Vec<Option<BatchItemResult>> = vec![
        Some(BatchItemResult {
            index: 0,
            source: "http://a/".into(),
            result: Err(crate::XbergError::Other("a".into())),
        }),
        None,
        Some(BatchItemResult {
            index: 2,
            source: "http://c/".into(),
            result: Err(crate::XbergError::Other("c".into())),
        }),
    ];
    let mut unmatched = VecDeque::new();
    unmatched.push_back(crate::XbergError::Other("task panicked: boom".into()));

    fill_dropped_shared_slots(&shared_items, &mut items, unmatched);

    assert!(items.iter().all(Option::is_some), "every shared slot must be filled");
    let filled = items[1].as_ref().expect("slot 1 filled");
    assert_eq!(filled.index, 1);
    assert_eq!(filled.source, "http://b/");
    match &filled.result {
        Err(crate::XbergError::Other(message)) => {
            assert!(message.contains("task panicked: boom"), "got: {message}");
        }
        _ => panic!("expected the re-attached panic error in slot 1"),
    }
}

/// When no unmatched error was captured, the synthesized error names the URL.
#[cfg(all(feature = "tokio-runtime", feature = "url-ingestion"))]
#[test]
fn fill_dropped_shared_slots_synthesizes_when_no_captured_error() {
    use std::collections::VecDeque;

    let shared_items = vec![SharedUrlItem {
        index: 0,
        source: "http://x/".into(),
        uri: "http://x/".into(),
        config: ExtractionConfig::default(),
    }];
    let mut items: Vec<Option<BatchItemResult>> = vec![None];

    fill_dropped_shared_slots(&shared_items, &mut items, VecDeque::new());

    match &items[0].as_ref().expect("slot 0 filled").result {
        Err(crate::XbergError::Other(message)) => {
            assert!(
                message.contains("http://x/"),
                "synthesized error names the URL, got: {message}"
            );
        }
        _ => panic!("expected a synthesized error naming the URL"),
    }
}

#[cfg(all(feature = "tokio-runtime", feature = "url-ingestion"))]
#[tokio::test]
async fn shared_url_duration_includes_fetch_without_extending_conversion_timeout() {
    let config = ExtractionConfig {
        extraction_timeout_secs: Some(1),
        ..ExtractionConfig::default()
    };
    let shared = SharedUrlItem {
        index: 0,
        source: "http://example.com/".into(),
        uri: "http://example.com/".into(),
        config,
    };
    let batch_started = Instant::now() - std::time::Duration::from_millis(25);
    let conversion = async { Ok(ExtractionResult::single(ExtractedDocument::default())) };

    let item = finalize_shared_item(&shared, batch_started, conversion).await;

    let output = item.result.expect("immediate conversion remains within its timeout");
    assert_eq!(output.results.len(), 1);
    assert!(
        output.results[0].metadata.extraction_duration_ms.unwrap_or_default() >= 25,
        "duration must include time before conversion began"
    );
}
