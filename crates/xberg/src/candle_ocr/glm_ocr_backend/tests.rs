use super::*;

/// A key no other test in this process shares, so the process-wide pool cannot
/// leak a resolution between tests.
#[cfg(feature = "layout-detection")]
fn unique_cache_dir(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("xberg-gh1718-{tag}-{:?}", std::thread::current().id()))
}

/// GH#1718: the layout model was resolved and SHA-256 verified once per page.
#[cfg(feature = "layout-detection")]
#[test]
fn the_layout_model_is_resolved_once_for_every_page_of_a_document() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let cache_dir = unique_cache_dir("once");
    let resolved = PathBuf::from("/models/pp_doclayout_v3.onnx");
    let resolves = AtomicUsize::new(0);

    // 22 pages: the document GH#1718 measured at about 2.9 GB of hashing.
    for page in 0..22 {
        let path = pool_get_or_init(&LAYOUT_MODEL_PATH_POOL, cache_dir.clone(), || {
            resolves.fetch_add(1, Ordering::SeqCst);
            Ok::<PathBuf, crate::XbergError>(resolved.clone())
        })
        .expect("the model path must resolve");
        assert_eq!(*path, resolved, "page {page} must see the same model path");
    }

    assert_eq!(
        resolves.load(Ordering::SeqCst),
        1,
        "the model must be resolved and verified once, not once per page"
    );
}

/// A download or verification failure must not poison the pool for the rest of
/// the process; the next page retries.
#[cfg(feature = "layout-detection")]
#[test]
fn a_failed_layout_model_resolution_is_not_cached() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let cache_dir = unique_cache_dir("retry");
    let resolved = PathBuf::from("/models/pp_doclayout_v3.onnx");
    let resolves = AtomicUsize::new(0);

    let first = pool_get_or_init(&LAYOUT_MODEL_PATH_POOL, cache_dir.clone(), || {
        resolves.fetch_add(1, Ordering::SeqCst);
        Err::<PathBuf, crate::XbergError>(crate::XbergError::Ocr {
            message: "layout model unavailable".to_string(),
            source: None,
        })
    });
    assert!(first.is_err(), "the first resolve must report the failure");

    let second = pool_get_or_init(&LAYOUT_MODEL_PATH_POOL, cache_dir, || {
        resolves.fetch_add(1, Ordering::SeqCst);
        Ok::<PathBuf, crate::XbergError>(resolved.clone())
    })
    .expect("the retry must resolve");

    assert_eq!(*second, resolved);
    assert_eq!(resolves.load(Ordering::SeqCst), 2, "a failed resolve must be retried");
}

#[test]
fn test_glm_ocr_backend_creation() {
    let backend = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default());
    assert_eq!(backend.name(), "candle-glm-ocr");
    assert_eq!(backend.backend_type(), OcrBackendType::Candle);
}

#[test]
fn test_glm_ocr_emits_structured_markdown() {
    let backend = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default());
    assert!(backend.emits_structured_markdown());
}

#[test]
fn test_glm_ocr_language_support() {
    let backend = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default());
    assert!(backend.supports_language("eng"));
    assert!(backend.supports_language("zho"));
    assert!(backend.supports_language("jpn"));
    assert!(backend.supports_language("unknown"));
}

#[test]
fn test_glm_ocr_supported_languages() {
    let backend = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default());
    let langs = backend.supported_languages();
    assert!(langs.contains(&"eng".to_string()));
    assert!(langs.contains(&"zho".to_string()));
    assert!(langs.contains(&"fra".to_string()));
}

#[tokio::test]
async fn should_reject_oversized_declared_dimensions_before_loading_glm_model() {
    let backend = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::WholePage);
    let bytes = crate::extraction::image_decode::bmp_with_declared_dimensions(6000, 6000);

    let error = backend
        .process_image(&bytes, &OcrConfig::default())
        .await
        .expect_err("GLM-OCR must validate the decoded-byte budget before model initialization");

    assert!(matches!(error, crate::XbergError::Validation { .. }));
    assert!(error.to_string().contains("6000x6000"));
    assert!(error.to_string().contains("security_limits.max_content_size"));
}

#[test]
fn test_parse_options_defaults() {
    let config = OcrConfig::default();
    let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
        .parse_options(&config)
        .unwrap();
    assert_eq!(opts.task, GlmOcrTask::Ocr);
    assert_eq!(opts.device, DevicePreference::Auto);
    assert!(!opts.enable_chart_understanding);
}

#[test]
fn test_parse_options_custom_task() {
    let config = OcrConfig {
        backend_options: Some(serde_json::json!({"task": "table"})),
        ..Default::default()
    };
    let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
        .parse_options(&config)
        .unwrap();
    assert_eq!(opts.task, GlmOcrTask::Table);
}

#[test]
fn test_parse_options_formula_task() {
    let config = OcrConfig {
        backend_options: Some(serde_json::json!({"task": "formula"})),
        ..Default::default()
    };
    let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
        .parse_options(&config)
        .unwrap();
    assert_eq!(opts.task, GlmOcrTask::Formula);
}

#[test]
fn test_parse_options_custom_device() {
    let config = OcrConfig {
        backend_options: Some(serde_json::json!({"device": "cpu"})),
        ..Default::default()
    };
    let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
        .parse_options(&config)
        .unwrap();
    assert_eq!(opts.device, DevicePreference::Cpu);
}

#[test]
fn test_parse_options_enable_chart_understanding_true() {
    let config = OcrConfig {
        backend_options: Some(serde_json::json!({"enable_chart_understanding": true})),
        ..Default::default()
    };
    let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
        .parse_options(&config)
        .unwrap();
    assert!(opts.enable_chart_understanding);
}

#[test]
fn test_parse_options_enable_chart_understanding_false() {
    let config = OcrConfig {
        backend_options: Some(serde_json::json!({"enable_chart_understanding": false})),
        ..Default::default()
    };
    let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
        .parse_options(&config)
        .unwrap();
    assert!(!opts.enable_chart_understanding);
}

#[test]
fn test_parse_options_chart_understanding_default() {
    let config = OcrConfig::default();
    let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
        .parse_options(&config)
        .unwrap();
    assert!(!opts.enable_chart_understanding);
}

#[test]
fn test_parse_options_combined() {
    let config = OcrConfig {
        backend_options: Some(serde_json::json!({
            "task": "chart",
            "device": "cuda",
            "enable_chart_understanding": true
        })),
        ..Default::default()
    };
    let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
        .parse_options(&config)
        .unwrap();
    assert_eq!(opts.task, GlmOcrTask::Chart);
    assert_eq!(opts.device, DevicePreference::Cuda);
    assert!(opts.enable_chart_understanding);
}

#[test]
fn test_parse_options_non_object_json_returns_contextual_errors() {
    let config = OcrConfig {
        backend_options: Some(serde_json::json!([1, 2, 3])),
        ..Default::default()
    };
    let error = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
        .parse_options(&config)
        .unwrap_err()
        .to_string();
    assert!(error.contains("candle-glm-ocr backend_options"));

    let config = OcrConfig {
        backend_options: Some(serde_json::json!("ocr")),
        ..Default::default()
    };
    let error = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
        .parse_options(&config)
        .unwrap_err()
        .to_string();
    assert!(error.contains("candle-glm-ocr backend_options"));
}

#[test]
fn test_parse_options_empty_object_returns_defaults() {
    let config = OcrConfig {
        backend_options: Some(serde_json::json!({})),
        ..Default::default()
    };
    let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
        .parse_options(&config)
        .unwrap();
    assert_eq!(opts.task, GlmOcrTask::Ocr);
    assert_eq!(opts.device, DevicePreference::Auto);
    assert!(!opts.enable_chart_understanding);
    assert!(opts.cache_dir.is_none());
}

#[test]
fn test_parse_options_layout_mode_and_cache_dir() {
    let config = OcrConfig {
        backend_options: Some(serde_json::json!({
            "layout_mode": "paired",
            "cache_dir": "/tmp/glm-cache"
        })),
        ..Default::default()
    };
    let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
        .parse_options(&config)
        .unwrap();
    assert_eq!(opts.layout_mode, LayoutMode::Paired);
    assert_eq!(opts.cache_dir.as_deref(), Some(Path::new("/tmp/glm-cache")));
}

#[test]
fn test_initialize_and_shutdown() {
    let backend = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default());
    assert!(backend.initialize().is_ok());
    assert!(backend.shutdown().is_ok());
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_task_for_label_table() {
    use crate::layout::LayoutClass;
    assert_eq!(task_for_label(LayoutClass::Table, false), GlmOcrTask::Table);
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_task_for_label_formula() {
    use crate::layout::LayoutClass;
    assert_eq!(task_for_label(LayoutClass::Formula, false), GlmOcrTask::Formula);
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_task_for_label_text() {
    use crate::layout::LayoutClass;
    assert_eq!(task_for_label(LayoutClass::Text, false), GlmOcrTask::Ocr);
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_task_for_label_chart_disabled() {
    use crate::layout::LayoutClass;
    assert_eq!(task_for_label(LayoutClass::Chart, false), GlmOcrTask::Caption);
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_task_for_label_chart_enabled() {
    use crate::layout::LayoutClass;
    assert_eq!(task_for_label(LayoutClass::Chart, true), GlmOcrTask::Chart);
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_parse_and_route_chart_with_understanding_enabled() {
    use crate::layout::LayoutClass;
    let config = OcrConfig {
        backend_options: Some(serde_json::json!({"enable_chart_understanding": true})),
        ..Default::default()
    };
    let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
        .parse_options(&config)
        .unwrap();
    let routed_task = task_for_label(LayoutClass::Chart, opts.enable_chart_understanding);
    assert_eq!(routed_task, GlmOcrTask::Chart);
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_parse_and_route_chart_with_understanding_disabled() {
    use crate::layout::LayoutClass;
    let config = OcrConfig {
        backend_options: Some(serde_json::json!({"enable_chart_understanding": false})),
        ..Default::default()
    };
    let opts = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default())
        .parse_options(&config)
        .unwrap();
    let routed_task = task_for_label(LayoutClass::Chart, opts.enable_chart_understanding);
    assert_eq!(routed_task, GlmOcrTask::Caption);
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_wrap_output_formula() {
    let wrapped = wrap_output(GlmOcrTask::Formula, "x^2 + y^2 = r^2");
    assert!(wrapped.starts_with("$$\n"));
    assert!(wrapped.ends_with("\n$$"));
    assert!(wrapped.contains("x^2 + y^2 = r^2"));
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_strip_formula_delimiters_removes_wrapping_dollars() {
    let wrapped = "$$\nE = mc^2\n$$";
    let result = strip_formula_delimiters(wrapped);
    assert_eq!(result, "E = mc^2");
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_strip_formula_delimiters_handles_pre_wrapped_content() {
    let pre_wrapped = "$$x^2 + y^2 = z^2$$";
    let result = strip_formula_delimiters(pre_wrapped);
    assert_eq!(result, "x^2 + y^2 = z^2");
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_strip_formula_delimiters_preserves_undecorated_content() {
    let plain = "a + b = c";
    let result = strip_formula_delimiters(plain);
    assert_eq!(result, "a + b = c");
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_formula_extraction_from_wrapped_output() {
    let task = GlmOcrTask::Formula;
    let raw_latex = "E = mc^2";
    let wrapped = wrap_output(task, raw_latex);
    let stripped = strip_formula_delimiters(&wrapped);
    assert_eq!(stripped, raw_latex);
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_wrap_output_chart() {
    let wrapped = wrap_output(GlmOcrTask::Chart, r#"{"type":"bar"}"#);
    assert!(wrapped.starts_with("```json\n"));
    assert!(wrapped.ends_with("\n```"));
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_wrap_output_table_passthrough() {
    let table = "| A | B |\n|---|---|\n| 1 | 2 |";
    let wrapped = wrap_output(GlmOcrTask::Table, table);
    assert_eq!(wrapped, table);
}

#[test]
fn test_pool_get_or_init_caches_on_first_miss() {
    use std::sync::Arc as StdArc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let pool: EngineCache<&str, u32> = EngineCache::unbounded();
    let init_count = StdArc::new(AtomicUsize::new(0));

    let init_count_clone = StdArc::clone(&init_count);
    let result1 = pool_get_or_init(&pool, "test_key", || {
        init_count_clone.fetch_add(1, Ordering::SeqCst);
        Ok::<u32, String>(42)
    });

    assert!(result1.is_ok());
    assert_eq!(init_count.load(Ordering::SeqCst), 1, "Initializer should run once");

    let init_count_clone = StdArc::clone(&init_count);
    let result2 = pool_get_or_init(&pool, "test_key", || {
        init_count_clone.fetch_add(1, Ordering::SeqCst);
        Ok::<u32, String>(99)
    });

    assert!(result2.is_ok());
    assert_eq!(
        init_count.load(Ordering::SeqCst),
        1,
        "Initializer should still have run exactly once"
    );

    let v1 = result1.unwrap();
    let v2 = result2.unwrap();
    assert!(Arc::ptr_eq(&v1, &v2), "Cached values should be the same Arc instance");
    assert_eq!(*v1, 42, "First initializer's value should be stored");
}

#[test]
fn test_pool_get_or_init_concurrent_access() {
    use std::sync::Arc as StdArc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    let pool: StdArc<EngineCache<&str, u32>> = StdArc::new(EngineCache::unbounded());
    let init_count = StdArc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for _ in 0..5 {
        let pool_clone = StdArc::clone(&pool);
        let init_count_clone = StdArc::clone(&init_count);

        let handle = thread::spawn(move || {
            let result = pool_get_or_init(&pool_clone, "concurrent_key", || {
                init_count_clone.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(1));
                Ok::<u32, String>(42)
            });
            result.unwrap()
        });
        handles.push(handle);
    }

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    for i in 1..results.len() {
        assert!(
            Arc::ptr_eq(&results[0], &results[i]),
            "All concurrent callers should receive the same Arc instance"
        );
    }

    let final_count = init_count.load(Ordering::SeqCst);
    assert_eq!(
        final_count, 1,
        "Initializer must run exactly once, not once per racing caller"
    );
}

#[test]
fn test_pool_get_or_init_failed_load_does_not_poison() {
    let pool: EngineCache<&str, u32> = EngineCache::unbounded();

    let failed = pool_get_or_init(&pool, "key", || Err::<u32, String>("load failed".to_string()));
    assert!(failed.is_err(), "a failing init must not be papered over");

    let recovered = pool_get_or_init(&pool, "key", || Ok::<u32, String>(7));
    assert_eq!(
        *recovered.expect("a later caller retries after a failed load"),
        7,
        "the retry must build a fresh value rather than reuse a poisoned entry"
    );
}

#[test]
fn test_glm_ocr_zero_regions_fallback_guard() {
    assert_eq!(GlmOcrTask::Ocr, GlmOcrTask::default());
}

// --- issue #187: paired-mode table bounding boxes must survive re-parsing ---

#[cfg(feature = "layout-detection")]
#[test]
fn test_is_table_region_true_for_table_class() {
    use crate::layout::LayoutClass;
    assert!(is_table_region(LayoutClass::Table));
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_is_table_region_false_for_text_class() {
    use crate::layout::LayoutClass;
    assert!(!is_table_region(LayoutClass::Text));
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_merge_table_bounding_boxes_attaches_bbox_matching_detection_order() {
    use crate::types::Table;
    use crate::types::extraction::BoundingBox;

    // Simulates two Table-class GFM tables parsed out of `content` by
    // `extract_gfm_tables`, in the same order the source detections appeared.
    let mut tables = vec![
        Table {
            cells: vec![vec!["A".to_string()]],
            markdown: "| A |\n|---|".to_string(),
            page_number: 1,
            ..Default::default()
        },
        Table {
            cells: vec![vec!["B".to_string()]],
            markdown: "| B |\n|---|".to_string(),
            page_number: 1,
            ..Default::default()
        },
    ];

    let bboxes = vec![
        BoundingBox {
            x0: 10.0,
            y0: 20.0,
            x1: 100.0,
            y1: 200.0,
        },
        BoundingBox {
            x0: 5.0,
            y0: 6.0,
            x1: 7.0,
            y1: 8.0,
        },
    ];

    merge_table_bounding_boxes(&mut tables, &bboxes);

    assert_eq!(tables.len(), 2, "table count must be unchanged by the merge");
    assert_eq!(
        tables[0].bounding_box,
        Some(BoundingBox {
            x0: 10.0,
            y0: 20.0,
            x1: 100.0,
            y1: 200.0
        })
    );
    assert_eq!(
        tables[1].bounding_box,
        Some(BoundingBox {
            x0: 5.0,
            y0: 6.0,
            x1: 7.0,
            y1: 8.0
        })
    );
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_process_paired_table_detection_yields_exactly_one_table_with_source_bbox() {
    use crate::core::config::OcrConfig;
    use crate::layout::LayoutClass;
    use crate::types::extraction::BoundingBox;

    let table_bbox = BoundingBox {
        x0: 12.0,
        y0: 34.0,
        x1: 512.0,
        y1: 734.0,
    };
    let table_output = "| Name | Age |\n|------|-----|\n| Alice | 30 |";
    let text_output = "Just some plain OCR'd prose.";
    let table = crate::layout::LayoutDetection::new(
        LayoutClass::Table,
        1.0,
        crate::layout::BBox::new(12.0, 34.0, 512.0, 734.0),
    );
    let text =
        crate::layout::LayoutDetection::new(LayoutClass::Text, 1.0, crate::layout::BBox::new(0.0, 0.0, 0.0, 0.0));
    let mut page = PairedPage::new(2);
    page.push_output(&table, GlmOcrTask::Table, table_output);
    page.push_output(&text, GlmOcrTask::Ocr, text_output);
    let (content, formulas, table_bboxes) = page.finish();
    assert_eq!(content, format!("{table_output}\n\n{text_output}"));
    assert!(formulas.is_empty());

    let config = OcrConfig::default();
    let mut doc = super::super::ocr_result::build_ocr_document(
        content,
        Vec::new(),
        &[],
        &config,
        super::super::ocr_result::OcrDocumentContext {
            mime_type: std::borrow::Cow::Borrowed("text/markdown"),
            backend_name: "candle-glm-ocr",
            plain_text_task: true,
        },
    );
    merge_table_bounding_boxes(&mut doc.tables, &table_bboxes);

    assert_eq!(
        doc.tables.len(),
        1,
        "the Text-class region must not produce a table entry"
    );
    assert_eq!(doc.tables[0].bounding_box, Some(table_bbox));
}

// --- issue #190: checkbox selected/unselected state must not be lost to OCR ---

#[cfg(feature = "layout-detection")]
#[test]
fn test_checkbox_marker_for_class_selected_is_x_marker() {
    use crate::layout::LayoutClass;
    assert_eq!(checkbox_marker_for_class(LayoutClass::CheckboxSelected), Some("[x]"));
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_checkbox_marker_for_class_unselected_is_empty_marker() {
    use crate::layout::LayoutClass;
    assert_eq!(checkbox_marker_for_class(LayoutClass::CheckboxUnselected), Some("[ ]"));
}

#[cfg(feature = "layout-detection")]
#[test]
fn test_checkbox_marker_for_class_none_for_non_checkbox_class() {
    use crate::layout::LayoutClass;
    assert_eq!(checkbox_marker_for_class(LayoutClass::Text), None);
    assert_eq!(checkbox_marker_for_class(LayoutClass::Table), None);
}

#[cfg(feature = "layout-detection")]
#[tokio::test]
async fn should_reject_invalid_paired_image_before_loading_models() {
    let cache = tempfile::tempdir().expect("create model cache directory");
    let error = process_paired(
        Vec::new(),
        DevicePreference::Cpu,
        DType::F32,
        false,
        cache.path().to_path_buf(),
        "main".to_string(),
    )
    .await
    .expect_err("invalid encoded image should fail before model loading");
    match error {
        crate::XbergError::Ocr { message, source } => {
            assert!(message.starts_with("GLM-OCR paired: image decode failed:"), "{message}");
            assert!(source.is_some());
        }
        other => panic!("unexpected error: {other}"),
    }
    assert_eq!(std::fs::read_dir(cache.path()).unwrap().count(), 0);
}

#[cfg(feature = "layout-detection")]
#[test]
fn should_clip_paired_crops_to_the_image_and_preserve_pixels() {
    let img = image::RgbImage::from_fn(2, 2, |x, y| image::Rgb([x as u8, y as u8, 123]));
    let limits = crate::extractors::security::SecurityLimits::default();
    let bytes = encode_paired_crop(&img, &crate::layout::BBox::new(-3.0, -4.0, 7.0, 8.0), &limits).unwrap();
    let decoded = image::load_from_memory(&bytes).unwrap().to_rgb8();
    assert_eq!(decoded, img);
    let bytes = encode_paired_crop(&img, &crate::layout::BBox::new(9.0, 9.0, 10.0, 10.0), &limits).unwrap();
    let decoded = image::load_from_memory(&bytes).unwrap().to_rgb8();
    assert_eq!(decoded.dimensions(), (1, 1));
    assert_eq!(decoded.get_pixel(0, 0), img.get_pixel(1, 1));
}

#[cfg(feature = "layout-detection")]
#[test]
fn should_enforce_the_paired_crop_live_memory_budget_before_encoding() {
    let img = image::RgbImage::new(2, 2);
    let limits = crate::extractors::security::SecurityLimits {
        max_content_size: 12,
        ..Default::default()
    };
    let error = encode_paired_crop(&img, &crate::layout::BBox::new(0.0, 0.0, 2.0, 2.0), &limits).unwrap_err();
    match error {
        crate::XbergError::Validation { message, source } => {
            assert_eq!(
                message,
                "Image dimensions 2x2 require 262184 live image-processing bytes, exceeding or invalid under security_limits.max_content_size (12 bytes)"
            );
            assert!(source.is_none());
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[cfg(feature = "layout-detection")]
#[test]
fn should_keep_formula_metadata_and_repeated_paired_text() {
    let bbox = crate::layout::BBox::new(12.0, 34.0, 56.0, 78.0);
    let mut detection = crate::layout::LayoutDetection::new(crate::layout::LayoutClass::Formula, 1.0, bbox);
    let mut page = PairedPage::new(3);
    page.push_output(&detection, GlmOcrTask::Formula, "$$ x^2 $$");
    detection.class_name = crate::layout::LayoutClass::Text;
    page.push_output(&detection, GlmOcrTask::Ocr, "Repeated text.");
    page.push_output(&detection, GlmOcrTask::Ocr, "Repeated text.");
    let (content, formulas, table_bboxes) = page.finish();
    assert_eq!(content, "$$\nx^2\n$$\n\nRepeated text.\n\nRepeated text.");
    assert_eq!(content.matches("Repeated text.").count(), 2);
    assert_eq!(formulas.len(), 1);
    assert_eq!(formulas[0].latex, "x^2");
    assert_eq!(formulas[0].page, Some(1));
    assert_eq!(
        formulas[0].bbox,
        Some(crate::types::extraction::BoundingBox {
            x0: 12.0,
            y0: 34.0,
            x1: 56.0,
            y1: 78.0
        })
    );
    assert_eq!(table_bboxes.len(), 0);
}
