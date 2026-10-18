use super::super::*;
use super::default_overrides;
use xberg::{ExecutionProviderType, ExtractionConfig};

#[test]
fn test_acceleration_applied() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        acceleration: Some(AccelerationArg::Cpu),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let accel = config.acceleration.unwrap();
    assert_eq!(accel.provider, ExecutionProviderType::Cpu);
}

#[test]
fn test_extract_pages_applied() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        extract_pages: Some(true),
        page_markers: Some(true),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let pages = config.pages.unwrap();
    assert!(pages.extract_pages);
    assert!(pages.insert_page_markers);
}

#[test]
fn test_extract_images_applied() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        extract_images: Some(true),
        target_dpi: Some(150),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let images = config.images.unwrap();
    assert!(images.extract_images);
    assert_eq!(images.target_dpi, 150);
}

#[cfg(feature = "analysis")]
#[test]
fn test_token_reduction_applied() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        token_reduction: Some(ReductionLevelArg::Aggressive),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let reduction = config.token_reduction.unwrap();
    assert_eq!(reduction.mode, "aggressive");
}

#[test]
fn test_msg_codepage_applied() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        format: FormatOverrides {
            msg_codepage: Some(1251),
            ..Default::default()
        },
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let email = config.email.unwrap();
    assert_eq!(email.msg_fallback_codepage, Some(1251));
}

#[test]
fn test_max_concurrent_applied() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        max_concurrent: Some(4),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    assert_eq!(config.max_concurrent_extractions, Some(4));
}

#[test]
fn test_max_threads_applied() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        max_threads: Some(2),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let concurrency = config.concurrency.unwrap();
    assert_eq!(concurrency.max_threads, Some(2));
}

#[test]
fn test_max_concurrent_ocr_applied() {
    let mut config = ExtractionConfig::default();
    let overrides = ExtractionOverrides {
        max_threads: Some(16),
        max_concurrent_ocr: Some(4),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    let concurrency = config.concurrency.unwrap();
    assert_eq!(concurrency.max_threads, Some(16));
    assert_eq!(concurrency.max_concurrent_ocr, Some(4));
}

#[test]
fn test_include_structure_applied() {
    let mut config = ExtractionConfig::default();
    assert!(!config.include_document_structure);
    let overrides = ExtractionOverrides {
        include_structure: Some(true),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    assert!(config.include_document_structure);
}

#[test]
fn test_strict_parse_applied() {
    let mut config = ExtractionConfig::default();
    assert!(!config.strict_parse);
    let overrides = ExtractionOverrides {
        strict_parse: Some(true),
        ..default_overrides()
    };
    overrides.apply(&mut config);
    assert!(config.strict_parse);
}

#[test]
fn test_validate_max_concurrent_zero() {
    let overrides = ExtractionOverrides {
        max_concurrent: Some(0),
        ..default_overrides()
    };
    let err = overrides.validate().unwrap_err();
    assert!(err.to_string().contains("--max-concurrent must be at least 1"));
}

#[test]
fn test_validate_max_threads_zero() {
    let overrides = ExtractionOverrides {
        max_threads: Some(0),
        ..default_overrides()
    };
    let err = overrides.validate().unwrap_err();
    assert!(err.to_string().contains("--max-threads must be at least 1"));
}

#[test]
fn test_validate_max_concurrent_ocr_zero() {
    let overrides = ExtractionOverrides {
        max_concurrent_ocr: Some(0),
        ..default_overrides()
    };
    let err = overrides.validate().unwrap_err();
    assert!(err.to_string().contains("--max-concurrent-ocr must be at least 1"));
}

#[test]
fn test_no_overrides_leaves_config_unchanged() {
    let original = ExtractionConfig::default();
    let mut config = original.clone();
    let overrides = default_overrides();
    overrides.apply(&mut config);

    assert!(config.ocr.is_none());
    assert!(config.chunking.is_none());
    assert!(config.use_cache);
    assert!(config.enable_quality_processing);
    assert!(!config.force_ocr);
    assert!(config.language_detection.is_none());
    assert!(config.pages.is_none());
    assert!(config.images.is_none());
    assert!(config.token_reduction.is_none());
    assert!(config.email.is_none());
    assert!(config.acceleration.is_none());
    assert!(config.concurrency.is_none());
    assert!(!config.include_document_structure);
}
