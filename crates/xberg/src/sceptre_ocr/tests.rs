use super::*;
use sceptre::{Point, Quad};

fn line(text: &str, confidence: f32) -> TextLine {
    TextLine {
        quad: Quad {
            points: [
                Point::new(10.4, 20.2),
                Point::new(110.6, 20.0),
                Point::new(111.0, 42.8),
                Point::new(10.0, 43.1),
            ],
        },
        text: text.to_string(),
        confidence,
    }
}

#[test]
fn should_map_iso_codes_to_sceptre_groups() {
    assert_eq!(language_group("eng"), Some(Language::English));
    assert_eq!(language_group("de"), Some(Language::Latin));
    assert_eq!(language_group("zh_Hans"), Some(Language::ChineseSimplified));
    assert_eq!(language_group("jpn"), Some(Language::Japanese));
    assert_eq!(language_group("jpn_vert"), Some(Language::Japanese));
    assert_eq!(language_group("kor"), Some(Language::Korean));
    assert_eq!(language_group("tel"), Some(Language::Telugu));
    assert_eq!(language_group("kan"), Some(Language::Kannada));
    assert_eq!(language_group("ukr"), Some(Language::Cyrillic));
    assert_eq!(language_group("ara"), None);
    assert_eq!(language_group("cat"), None);
    assert_eq!(language_group("kaz"), None);
}

#[test]
fn shared_validation_should_accept_every_sceptre_language_alias() {
    for language in supported_language_aliases() {
        assert!(
            crate::core::config_validation::validate_language_code(language).is_ok(),
            "Sceptre language alias {language} should pass shared validation"
        );
    }
}

#[test]
fn should_allow_english_with_one_non_english_group() {
    let languages = vec!["eng".to_string(), "deu".to_string(), "fra".to_string()];
    assert_eq!(
        resolve_language_group(&languages).expect("compatible languages"),
        Language::Latin
    );
}

#[test]
fn should_normalize_easyocr_script_tokens() {
    assert_eq!(language_group("CH_SIM"), Some(Language::ChineseSimplified));
    assert_eq!(language_group("rs_latin"), Some(Language::Latin));
    assert_eq!(language_group("rs-cyrillic"), Some(Language::Cyrillic));
}

#[test]
fn should_reject_incompatible_language_groups() {
    let languages = vec!["jpn".to_string(), "kor".to_string()];
    let error = resolve_language_group(&languages).expect_err("groups must be incompatible");
    assert!(error.to_string().contains("cannot combine language groups"));
}

#[test]
fn should_parse_direct_sceptre_options_and_override_languages() {
    let config = OcrConfig {
        language: vec!["rus".to_string()],
        backend_options: Some(serde_json::json!({
            "recognition": { "batch_size": 4 },
            "concurrency": { "max_threads": 2 }
        })),
        ..Default::default()
    };
    let (effective, _) = SceptreOcrBackend::effective_config(&config).expect("valid config");
    assert_eq!(effective.recognition.batch_size, 4);
    assert_eq!(effective.concurrency.max_threads, Some(2));
    assert_eq!(effective.model.languages, vec![Language::Cyrillic]);
}

#[test]
fn sceptre_rejects_an_empty_explicit_model_cache_root() {
    let config = OcrConfig {
        backend: "sceptre".to_string(),
        backend_options: Some(serde_json::json!({"model": {"cache_dir": ""}})),
        ..OcrConfig::default()
    };

    let error =
        SceptreOcrBackend::effective_config(&config).expect_err("an empty explicit model cache root must be rejected");

    assert_eq!(
        error.to_string(),
        "Validation error: OCR model cache_dir must not be empty"
    );
}

#[test]
fn sceptre_preserves_an_explicit_model_cache_root() {
    let config = OcrConfig {
        backend: "sceptre".to_string(),
        backend_options: Some(serde_json::json!({"model": {"cache_dir": "/request/cache"}})),
        ..OcrConfig::default()
    };

    let (effective, _) = SceptreOcrBackend::effective_config(&config).expect("explicit cache root should resolve");

    assert_eq!(effective.model.cache_dir, Some("/request/cache".into()));
}

#[test]
fn sceptre_reader_cache_key_distinguishes_model_cache_roots() {
    let mut first = sceptre::OcrConfig::default();
    first.model.cache_dir = Some("/first/cache".into());
    let mut second = first.clone();
    second.model.cache_dir = Some("/second/cache".into());

    assert_ne!(
        reader_cache_key(&first).expect("first config should serialize"),
        reader_cache_key(&second).expect("second config should serialize")
    );
}

#[cfg(auto_rotate)]
#[test]
fn sceptre_orientation_detector_cache_distinguishes_root_and_acceleration() {
    let backend = SceptreOcrBackend::new().expect("backend should initialize");
    let first_config = OcrConfig {
        backend_options: Some(serde_json::json!({"model": {"cache_dir": "/first/cache"}})),
        ..OcrConfig::default()
    };
    let second_config = OcrConfig {
        backend_options: Some(serde_json::json!({"model": {"cache_dir": "/second/cache"}})),
        ..OcrConfig::default()
    };
    let (first_config, _) = SceptreOcrBackend::effective_config(&first_config).expect("first config should resolve");
    let (second_config, _) = SceptreOcrBackend::effective_config(&second_config).expect("second config should resolve");
    let first = backend
        .orientation_detector_for_config(
            &first_config,
            Some(crate::core::config::AccelerationConfig {
                provider: ExecutionProviderType::Cpu,
                device_id: 0,
            }),
        )
        .expect("first detector should resolve");
    let repeated = backend
        .orientation_detector_for_config(
            &first_config,
            Some(crate::core::config::AccelerationConfig {
                provider: ExecutionProviderType::Cpu,
                device_id: 0,
            }),
        )
        .expect("repeated detector should resolve");
    let other_root = backend
        .orientation_detector_for_config(
            &second_config,
            Some(crate::core::config::AccelerationConfig {
                provider: ExecutionProviderType::Cpu,
                device_id: 0,
            }),
        )
        .expect("detector for another root should resolve");
    let other_acceleration = backend
        .orientation_detector_for_config(
            &first_config,
            Some(crate::core::config::AccelerationConfig {
                provider: ExecutionProviderType::Cuda,
                device_id: 1,
            }),
        )
        .expect("detector for another acceleration config should resolve");

    assert!(Arc::ptr_eq(&first, &repeated));
    assert!(!Arc::ptr_eq(&first, &other_root));
    assert!(!Arc::ptr_eq(&first, &other_acceleration));
}

#[cfg(auto_rotate)]
#[test]
fn sceptre_orientation_detector_cache_is_bounded_when_roots_change() {
    let backend = SceptreOcrBackend::new().expect("backend should initialize");

    for index in 0..=ORIENTATION_DETECTOR_CACHE_CAPACITY {
        backend
            .orientation_detector_cell(
                std::path::Path::new(&format!("/cache/{index}")),
                Some(crate::core::config::AccelerationConfig {
                    provider: ExecutionProviderType::Cpu,
                    device_id: 0,
                }),
            )
            .expect("detector should resolve");
    }

    assert_eq!(
        backend
            .orientation_detectors
            .lock()
            .expect("detector cache lock should remain available")
            .entries
            .len(),
        ORIENTATION_DETECTOR_CACHE_CAPACITY
    );
}

#[cfg(auto_rotate)]
#[test]
fn sceptre_orientation_detector_cache_rejects_a_fifth_live_root() {
    let backend = SceptreOcrBackend::new().expect("backend should initialize");
    let mut active = Vec::new();
    for index in 0..ORIENTATION_DETECTOR_CACHE_CAPACITY {
        active.push(
            backend
                .orientation_detector_cell(
                    std::path::Path::new(&format!("/active/{index}")),
                    Some(crate::core::config::AccelerationConfig {
                        provider: ExecutionProviderType::Cpu,
                        device_id: 0,
                    }),
                )
                .expect("active detector should resolve"),
        );
    }

    let repeated = backend
        .orientation_detector_cell(
            std::path::Path::new("/active/0"),
            Some(crate::core::config::AccelerationConfig {
                provider: ExecutionProviderType::Cpu,
                device_id: 0,
            }),
        )
        .expect("an existing active detector should be reused");
    let error = match backend.orientation_detector_cell(
        std::path::Path::new("/active/overflow"),
        Some(crate::core::config::AccelerationConfig {
            provider: ExecutionProviderType::Cpu,
            device_id: 0,
        }),
    ) {
        Ok(_) => panic!("a fifth live detector must exceed bounded admission"),
        Err(error) => error,
    };

    assert!(Arc::ptr_eq(&active[0], &repeated));
    assert_eq!(
        error.to_string(),
        "OCR error: Sceptre orientation detector cache is at its bounded capacity of 4 active configurations"
    );
    assert_eq!(
        backend
            .orientation_detectors
            .lock()
            .expect("detector cache lock should remain available")
            .entries
            .len(),
        ORIENTATION_DETECTOR_CACHE_CAPACITY
    );
}

#[cfg(auto_rotate)]
#[test]
fn sceptre_disabled_auto_rotation_does_not_cache_a_detector() {
    let backend = SceptreOcrBackend::new().expect("backend should initialize");
    let request = OcrConfig {
        auto_rotate: false,
        backend_options: Some(serde_json::json!({"model": {"cache_dir": "/unused/cache"}})),
        ..OcrConfig::default()
    };
    let (effective, _) = SceptreOcrBackend::effective_config(&request).expect("config should resolve");

    let rotation = backend
        .rotation_context(&request, &effective)
        .expect("rotation context should resolve");

    assert!(rotation.detector.is_none());
    assert_eq!(
        backend
            .orientation_detectors
            .lock()
            .expect("detector cache lock should remain available")
            .entries
            .len(),
        0
    );
}

#[test]
fn should_propagate_xberg_thread_budget_when_unspecified() {
    let (effective, _) = SceptreOcrBackend::effective_config(&OcrConfig::default()).expect("valid config");
    assert_eq!(
        effective.concurrency.max_threads,
        Some(crate::core::config::concurrency::active_thread_budget())
    );
}

#[test]
fn should_clamp_sceptre_threads_to_xberg_budget() {
    let config = OcrConfig {
        backend_options: Some(serde_json::json!({ "concurrency": { "max_threads": usize::MAX } })),
        ..Default::default()
    };
    let (effective, _) = SceptreOcrBackend::effective_config(&config).expect("valid config");
    assert_eq!(
        effective.concurrency.max_threads,
        Some(crate::core::config::concurrency::active_thread_budget())
    );
}

#[cfg(all(feature = "sceptre-ocr-tract", not(feature = "sceptre-ocr-ort")))]
#[test]
fn tract_only_build_should_select_tract_by_default() {
    let (effective, _) = SceptreOcrBackend::effective_config(&OcrConfig::default()).expect("valid config");
    assert_eq!(effective.model.backend, Backend::Tract);
}

#[cfg(not(feature = "sceptre-ocr-tract"))]
#[test]
fn ort_only_build_should_reject_explicit_tract() {
    let config = OcrConfig {
        backend_options: Some(serde_json::json!({ "model": { "backend": "tract" } })),
        ..Default::default()
    };
    let error = SceptreOcrBackend::effective_config(&config).expect_err("tract must require its feature");
    assert!(error.to_string().contains("sceptre-ocr-tract"));
}

// Candle is never the compile-time default (unlike tract in a tract-only build):
// selecting it always requires an explicit `backend_options.model.backend = "candle"`,
// so there is no "candle_only_build_should_select_candle_by_default" counterpart here.
#[cfg(not(feature = "sceptre-ocr-candle"))]
#[test]
fn build_without_candle_should_reject_explicit_candle() {
    let config = OcrConfig {
        backend_options: Some(serde_json::json!({ "model": { "backend": "candle" } })),
        ..Default::default()
    };
    let error = SceptreOcrBackend::effective_config(&config).expect_err("candle must require its feature");
    assert!(error.to_string().contains("sceptre-ocr-candle"));
}

#[cfg(feature = "sceptre-ocr-candle")]
#[test]
fn should_accept_explicit_candle_backend_when_compiled_in() {
    let config = OcrConfig {
        backend_options: Some(serde_json::json!({ "model": { "backend": "candle" } })),
        ..Default::default()
    };
    let (effective, _) =
        SceptreOcrBackend::effective_config(&config).expect("candle backend must be accepted explicitly");
    assert_eq!(effective.model.backend, Backend::Candle);
}

#[test]
fn should_ignore_options_owned_by_other_backends() {
    let config = OcrConfig {
        backend_options: Some(serde_json::json!({ "custom_backend_option": true })),
        ..Default::default()
    };
    let parsed = parse_sceptre_options(&config).expect("unrelated options must be ignored");
    assert_eq!(
        parsed.recognition.batch_size,
        sceptre::OcrConfig::default().recognition.batch_size
    );
}

#[test]
fn should_reject_unknown_sceptre_section_option() {
    let config = OcrConfig {
        backend_options: Some(serde_json::json!({ "recognition": { "typo": true } })),
        ..Default::default()
    };
    let error = parse_sceptre_options(&config).expect_err("unknown Sceptre fields must fail");
    assert!(error.to_string().contains("backend_options.recognition"));
}

#[test]
fn should_reject_non_cpu_acceleration() {
    let config = OcrConfig {
        acceleration: Some(crate::core::config::AccelerationConfig {
            provider: ExecutionProviderType::CoreMl,
            ..Default::default()
        }),
        ..Default::default()
    };

    let error = SceptreOcrBackend::effective_config(&config).expect_err("CoreML must be rejected");
    assert!(error.to_string().contains("CPU-only"));
}

#[cfg(not(auto_rotate))]
#[test]
fn should_reject_auto_rotate_without_capability() {
    let config = OcrConfig {
        auto_rotate: true,
        ..Default::default()
    };

    let error = SceptreOcrBackend::effective_config(&config).expect_err("auto-rotation must require its feature");
    assert!(error.to_string().contains("auto-rotation is unavailable"));
}

#[test]
fn should_build_line_geometry_and_clamp_confidence() {
    let element = line_to_element(&line("hello", 1.4));
    assert_eq!(element.text, "hello");
    assert_eq!(element.level, OcrElementLevel::Line);
    assert_eq!(element.confidence.recognition, 1.0);
    assert_eq!(
        element.geometry,
        OcrBoundingGeometry::Quadrilateral {
            points: [(10, 20), (111, 20), (111, 43), (10, 43)]
                .into_iter()
                .map(Into::into)
                .collect()
        }
    );
}

#[test]
fn should_join_lines_and_build_layout_document() {
    let output = BlockingOutput {
        result: OcrResult {
            lines: vec![line("first", 0.9), line("second", 0.8)],
        },
        width: 640,
        height: 480,
        #[cfg(auto_rotate)]
        orientation: None,
        #[cfg(auto_rotate)]
        auto_rotated: false,
    };
    let result = build_document(output, vec!["eng".to_string()], &OcrConfig::default());
    assert_eq!(result.content, "first\nsecond");
    let internal = result.ocr_internal_document.expect("internal OCR document");
    assert_eq!(internal.elements.len(), 2);
    let first = &internal.elements[0];
    assert_eq!(
        first.ocr_confidence.as_ref().expect("confidence").recognition,
        0.9_f32 as f64
    );
    assert_eq!(
        first.bbox,
        Some(BoundingBox {
            x0: 10.0,
            y0: 20.0,
            x1: 111.0,
            y1: 43.0,
        })
    );
    assert!(result.ocr_elements.is_none());
}

#[test]
fn should_filter_requested_line_elements_by_confidence() {
    let config = OcrConfig {
        element_config: Some(crate::types::OcrElementConfig {
            include_elements: true,
            min_level: OcrElementLevel::Line,
            min_confidence: 0.85,
            build_hierarchy: false,
        }),
        ..Default::default()
    };
    let elements = vec![line_to_element(&line("keep", 0.9)), line_to_element(&line("drop", 0.8))];
    let selected = select_output_elements(&elements, &config).expect("one selected line");
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].text, "keep");
}

/// Regression test for issue #186: Sceptre must publish confidence
/// statistics under the same keys Tesseract uses, so quality gating that
/// reads `mean_text_conf` / `median_word_conf` / `p10_word_conf` /
/// `word_count` / `low_conf_word_count` works against Sceptre output too.
#[test]
fn should_publish_confidence_statistics_in_metadata() {
    let output = BlockingOutput {
        result: OcrResult {
            // percentages: 90, 80, 40, 100 -> mean 77, sorted [40,80,90,100] ~keep
            lines: vec![line("a", 0.9), line("b", 0.8), line("c", 0.4), line("d", 1.0)],
        },
        width: 100,
        height: 100,
        #[cfg(auto_rotate)]
        orientation: None,
        #[cfg(auto_rotate)]
        auto_rotated: false,
    };

    let metadata = build_metadata(&["eng".to_string()], &output);

    assert_eq!(metadata.additional.get("mean_text_conf"), Some(&serde_json::json!(77)));
    assert_eq!(
        metadata.additional.get("median_word_conf"),
        Some(&serde_json::json!(85))
    );
    assert_eq!(metadata.additional.get("p10_word_conf"), Some(&serde_json::json!(40)));
    assert_eq!(metadata.additional.get("word_count"), Some(&serde_json::json!(4)));
    assert_eq!(
        metadata.additional.get("low_conf_word_count"),
        Some(&serde_json::json!(1))
    );
}

/// With no recognized lines there is nothing to report; the confidence keys
/// must be absent rather than fabricated zeros (matching Tesseract's
/// behavior of only inserting `mean_text_conf` when `>= 0`).
#[test]
fn should_omit_confidence_statistics_when_no_lines_recognized() {
    let output = BlockingOutput {
        result: OcrResult { lines: vec![] },
        width: 100,
        height: 100,
        #[cfg(auto_rotate)]
        orientation: None,
        #[cfg(auto_rotate)]
        auto_rotated: false,
    };

    let metadata = build_metadata(&["eng".to_string()], &output);

    assert!(metadata.additional.get("mean_text_conf").is_none());
    assert!(metadata.additional.get("word_count").is_none());
}

/// Regression test for issue #186: `psm` must not carry Tesseract's PSM-3
/// default, since Sceptre has no Page Segmentation Mode concept.
#[test]
fn should_not_report_tesseract_psm_for_sceptre_metadata() {
    let output = BlockingOutput {
        result: OcrResult {
            lines: vec![line("hello", 0.9)],
        },
        width: 100,
        height: 100,
        #[cfg(auto_rotate)]
        orientation: None,
        #[cfg(auto_rotate)]
        auto_rotated: false,
    };

    let metadata = build_metadata(&["eng".to_string()], &output);
    let Some(FormatMetadata::Ocr(ocr_metadata)) = metadata.format else {
        panic!("expected FormatMetadata::Ocr");
    };
    assert_eq!(
        ocr_metadata.psm, 0,
        "Sceptre has no PSM; must not claim Tesseract's PSM 3"
    );
}

#[test]
fn cache_key_should_change_with_reader_configuration() {
    let first = sceptre::OcrConfig::default();
    let mut second = first.clone();
    second.recognition.batch_size = 8;
    assert_ne!(
        reader_cache_key(&first).expect("key"),
        reader_cache_key(&second).expect("key")
    );
}

#[tokio::test]
async fn reader_cache_should_reuse_cell_for_same_key() {
    let backend = SceptreOcrBackend::new().expect("backend should construct");
    let first = backend.reader_cell("english").await.expect("cache lookup");
    let second = backend.reader_cell("english").await.expect("cache lookup");

    assert!(Arc::ptr_eq(&first, &second));
    let cache = backend.readers.lock().expect("cache lock");
    assert_eq!(cache.entries.len(), 1);
    assert_eq!(cache.recency, VecDeque::from(["english".to_string()]));
}

#[tokio::test]
async fn reader_cache_should_evict_least_recently_used_inactive_cell() {
    let backend = SceptreOcrBackend::new().expect("backend should construct");
    for index in 0..READER_CACHE_CAPACITY {
        drop(
            backend
                .reader_cell(&format!("reader-{index}"))
                .await
                .expect("cache lookup"),
        );
    }

    drop(backend.reader_cell("reader-0").await.expect("cache lookup"));
    drop(backend.reader_cell("replacement").await.expect("cache lookup"));

    let cache = backend.readers.lock().expect("cache lock");
    assert_eq!(cache.entries.len(), READER_CACHE_CAPACITY);
    assert!(cache.entries.contains_key("reader-0"));
    assert!(!cache.entries.contains_key("reader-1"));
    assert!(cache.entries.contains_key("replacement"));
    assert_eq!(
        cache.recency,
        VecDeque::from([
            "reader-2".to_string(),
            "reader-3".to_string(),
            "reader-0".to_string(),
            "replacement".to_string(),
        ])
    );
}

#[tokio::test]
async fn reader_cache_should_bound_distinct_overflow_admission() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    const OVERFLOW_REQUESTS: usize = 8;

    let backend = Arc::new(SceptreOcrBackend::new().expect("backend should construct"));
    let mut cached = Vec::with_capacity(READER_CACHE_CAPACITY);
    for index in 0..READER_CACHE_CAPACITY {
        cached.push(
            backend
                .reader_cell(&format!("reader-{index}"))
                .await
                .expect("cache lookup"),
        );
    }

    let first_overflow = backend.reader_cell("overflow-0").await.expect("overflow admission");
    let same_overflow = backend.reader_cell("overflow-0").await.expect("overflow reuse");
    assert!(Arc::ptr_eq(&first_overflow, &same_overflow));

    let active_overflow = Arc::new(AtomicUsize::new(0));
    let maximum_overflow = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::with_capacity(OVERFLOW_REQUESTS);
    for index in 1..=OVERFLOW_REQUESTS {
        let backend = Arc::clone(&backend);
        let active_overflow = Arc::clone(&active_overflow);
        let maximum_overflow = Arc::clone(&maximum_overflow);
        tasks.push(tokio::spawn(async move {
            let cell = backend
                .reader_cell(&format!("overflow-{index}"))
                .await
                .expect("overflow admission");
            let active = active_overflow.fetch_add(1, Ordering::SeqCst) + 1;
            maximum_overflow.fetch_max(active, Ordering::SeqCst);
            tokio::task::yield_now().await;
            active_overflow.fetch_sub(1, Ordering::SeqCst);
            drop(cell);
        }));
    }

    tokio::task::yield_now().await;
    assert_eq!(maximum_overflow.load(Ordering::SeqCst), 0);
    drop(same_overflow);
    drop(first_overflow);
    for task in tasks {
        task.await.expect("overflow task");
    }

    assert_eq!(maximum_overflow.load(Ordering::SeqCst), READER_OVERFLOW_CAPACITY);
    assert_eq!(backend.overflow_slots.available_permits(), READER_OVERFLOW_CAPACITY);
    let cache = backend.readers.lock().expect("cache lock");
    assert_eq!(cache.entries.len(), READER_CACHE_CAPACITY);
    assert!(cache.overflow.len() <= READER_OVERFLOW_CAPACITY);
    drop(cache);
    drop(cached);
}

#[tokio::test]
async fn should_reject_invalid_image_without_loading_models() {
    let backend = SceptreOcrBackend::new().expect("backend should construct");
    let error = backend
        .process_image(b"not an image", &OcrConfig::default())
        .await
        .expect_err("invalid image data must fail");

    assert_eq!(error.to_string(), "OCR error: Sceptre OCR operation failed");
    let XbergError::Ocr { source, .. } = error else {
        panic!("invalid Sceptre image data must return an OCR error");
    };
    assert!(
        source.is_some(),
        "the Sceptre decode error must remain in the source chain"
    );
}

#[test]
fn should_reject_oversized_declared_dimensions_before_sceptre_decode() {
    let bytes = crate::extraction::image_decode::bmp_with_declared_dimensions(6000, 6000);

    let error = decode_sceptre_image(&bytes)
        .expect_err("Sceptre decode must apply the default security budget before model work");

    let XbergError::Ocr {
        source: Some(source), ..
    } = error
    else {
        panic!("Sceptre budget rejection must retain the validation error as its source");
    };
    assert!(source.to_string().contains("6000x6000"));
    assert!(source.to_string().contains("security_limits.max_content_size"));
}

#[test]
fn plugin_version_matches_sceptre_crate() {
    let backend = SceptreOcrBackend::new().expect("backend should construct");
    assert_eq!(backend.version(), sceptre::VERSION);
}

/// Axis-aligned `TextLine` at the given extent, for block-grouping tests.
fn word_line(text: &str, x0: f32, x1: f32, y0: f32, y1: f32, confidence: f32) -> TextLine {
    TextLine {
        quad: Quad {
            points: [
                Point::new(x0, y0),
                Point::new(x1, y0),
                Point::new(x1, y1),
                Point::new(x0, y1),
            ],
        },
        text: text.to_string(),
        confidence,
    }
}

/// Regression for the "no block/paragraph grouping" WP-D defect (#668):
/// `build_internal_document` never set `attributes`, so
/// `pdf::structure::adapters::hocr_block_id` always returned `None` and
/// every sceptre line became its own paragraph. This mirrors the
/// geometric grouping already applied to PaddleOCR
/// (`ocr::conversion::assign_line_block_ids`, #631) via the intentionally
/// duplicated `assign_sceptre_block_ids` (see its doc comment for why it
/// is not a shared call).
///
/// Against unfixed code (`internal.attributes` left `None`, as it was
/// before this change), `block_id(0)` and `block_id(1)` both return `None`
/// and the `.expect(...)` calls below panic.
#[test]
fn should_assign_shared_block_id_to_close_lines_and_a_new_id_after_a_gap() {
    let elements = vec![
        line_to_element(&word_line("First line", 10.0, 200.0, 100.0, 120.0, 0.9)),
        line_to_element(&word_line("Second line", 10.0, 200.0, 124.0, 144.0, 0.9)),
        line_to_element(&word_line("Far below", 10.0, 200.0, 400.0, 420.0, 0.9)),
    ];

    let document = build_internal_document(&elements);

    let block_id = |index: usize| {
        document.elements[index]
            .attributes
            .as_ref()
            .and_then(|attributes| attributes.get(HOCR_BLOCK_ID_ATTRIBUTE))
            .cloned()
    };

    let first = block_id(0).expect("first line must carry a block id");
    let second = block_id(1).expect("second line must carry a block id");
    let third = block_id(2).expect("third line must carry a block id");

    assert_eq!(first, second, "vertically adjacent lines must share a block id");
    assert_ne!(second, third, "a large vertical gap must start a new block");
}

/// Regression for a structural defect in `lines_share_block`, found by feeding it
/// Sceptre's own detected geometry (not a Tesseract proxy) for
/// `test_documents/pdf_scanned/ordinance_2197_scanned.pdf`: Sceptre's `width_ths`
/// detection parameter under-merges this line, so one physical line surfaces as 9
/// separate `TextLine` entries ("In" / "accordance" / "with" / "Section" / "2-133,"
/// / "the" / "PD" / "must" / "be"), reproduced here with their real detected
/// coordinates.
///
/// Each fragment's vertical gap against its predecessor is already 0 (their y-ranges
/// overlap), so the vertical-gap check alone never rejected them. The bug was the
/// horizontal overlap/left-edge fallback that follows it: consecutive word fragments
/// don't x-overlap and their left edges march rightward across the line, so
/// `overlap_ratio` stays 0 and `left_edge_offset` (~47-260px) blows past the ~16-23px
/// threshold derived from the line's own ~32-36px height -- producing one spurious
/// block per word instead of one block for the whole line.
///
/// Against unfixed code (no y-center short-circuit in `lines_share_block`), each
/// fragment after the first starts a new block, so `block_id(1)` (`"accordance"`)
/// differs from `block_id(0)` (`"In"`) and the first `assert_eq!` below fails.
#[test]
fn should_merge_word_fragments_of_one_sceptre_line_sharing_a_ycenter() {
    let elements = vec![
        line_to_element(&word_line("In", 718.0, 752.0, 906.0, 938.0, 0.9)),
        line_to_element(&word_line("accordance", 765.0, 923.0, 905.0, 941.0, 0.9)),
        line_to_element(&word_line("with", 936.0, 1002.0, 908.0, 940.0, 0.9)),
        line_to_element(&word_line("Section", 1015.0, 1123.0, 905.0, 941.0, 0.9)),
        line_to_element(&word_line("2-133,", 1135.0, 1223.0, 905.0, 941.0, 0.9)),
        line_to_element(&word_line("the", 1244.0, 1292.0, 908.0, 940.0, 0.9)),
        line_to_element(&word_line("PD", 1306.0, 1354.0, 908.0, 940.0, 0.9)),
        line_to_element(&word_line("must", 1368.0, 1444.0, 910.0, 942.0, 0.9)),
        line_to_element(&word_line("be", 1458.0, 1496.0, 908.0, 940.0, 0.9)),
        // A genuinely new paragraph, far below: must still start a new block.
        line_to_element(&word_line("2_", 285.0, 305.0, 1059.0, 1087.0, 0.9)),
    ];

    let document = build_internal_document(&elements);

    let block_id = |index: usize| {
        document.elements[index]
            .attributes
            .as_ref()
            .and_then(|attributes| attributes.get(HOCR_BLOCK_ID_ATTRIBUTE))
            .cloned()
            .expect("every line must carry a block id")
    };

    let line_block_ids: Vec<String> = (0..9).map(block_id).collect();
    for (index, id) in line_block_ids.iter().enumerate().skip(1) {
        assert_eq!(
            *id, line_block_ids[0],
            "word fragment {index} must share the same-line block id as the first fragment"
        );
    }

    assert_ne!(
        block_id(9),
        line_block_ids[0],
        "a distant, unrelated line must still start a new block"
    );
}

/// Sceptre counterpart of
/// `ocr::conversion::assign_line_block_ids_splits_indent_marked_paragraphs_with_uniform_leading`.
/// The PaddleOCR grouper gained an indent check after a 16-page scanned
/// document produced exactly 16 paragraphs; `assign_sceptre_block_ids` was
/// copied from it before that check existed and never received it, so this
/// same page collapses into a single block under sceptre.
///
/// Paragraph breaks here are marked ONLY by first-line indentation (left
/// margin 200px, indent 300px): every line sits 16px below its predecessor,
/// well inside the 0.6x-line-height gap threshold, and every pair overlaps
/// horizontally. Against unfixed code all eight entries come back as
/// `"sceptre-block-1"`.
#[test]
fn should_split_indent_marked_paragraphs_that_share_uniform_leading() {
    let elements = vec![
        // Paragraph 1: indented first line, two flush continuation lines.
        line_to_element(&word_line("Para one first", 300.0, 2200.0, 200.0, 234.0, 0.9)),
        line_to_element(&word_line("continues here", 200.0, 2200.0, 250.0, 284.0, 0.9)),
        line_to_element(&word_line("and ends", 200.0, 1000.0, 300.0, 334.0, 0.9)),
        // Paragraph 2: indented first line, same 16px gap as within paragraph 1.
        line_to_element(&word_line("Para two first", 300.0, 2200.0, 350.0, 384.0, 0.9)),
        line_to_element(&word_line("continues here", 200.0, 2200.0, 400.0, 434.0, 0.9)),
        line_to_element(&word_line("and ends", 200.0, 700.0, 450.0, 484.0, 0.9)),
        // Paragraph 3: indented first line, same 16px gap again.
        line_to_element(&word_line("Para three first", 300.0, 2200.0, 500.0, 534.0, 0.9)),
        line_to_element(&word_line("continues here", 200.0, 2200.0, 550.0, 584.0, 0.9)),
    ];

    let block_ids = assign_sceptre_block_ids(&elements);

    assert_eq!(
        block_ids,
        vec![
            "sceptre-block-1",
            "sceptre-block-1",
            "sceptre-block-1",
            "sceptre-block-2",
            "sceptre-block-2",
            "sceptre-block-2",
            "sceptre-block-3",
            "sceptre-block-3",
        ],
        "indented first lines must start a new block even though the vertical gap never changes"
    );
}

/// Companion control to the indent-split test: OCR boxes are never
/// pixel-perfect, so continuation lines flush with the block's established
/// left margin must not be split apart by sub-pixel left-edge jitter.
#[test]
fn should_not_split_flush_continuation_lines_on_left_edge_jitter() {
    let elements = vec![
        line_to_element(&word_line("Indented first", 300.0, 2200.0, 200.0, 234.0, 0.9)),
        line_to_element(&word_line("flush second", 201.0, 2200.0, 250.0, 284.0, 0.9)),
        line_to_element(&word_line("flush third", 199.0, 2200.0, 300.0, 334.0, 0.9)),
    ];

    let block_ids = assign_sceptre_block_ids(&elements);

    assert_eq!(
        block_ids,
        vec!["sceptre-block-1", "sceptre-block-1", "sceptre-block-1"],
        "sub-pixel left-edge jitter on flush continuation lines must not start a new block"
    );
}
