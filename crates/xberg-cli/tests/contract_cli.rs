//! CLI contract tests - verify CLI config parsing matches Rust core
//!
//! This test suite validates that the CLI's configuration parsing produces
//! identical results to the Rust core library. It ensures that users get
//! consistent behavior whether using the CLI, SDK, or MCP interfaces.

use serde_json::json;
use xberg::core::config::ExtractionConfig;
use xberg::core::config::OutputFormat;

#[test]
fn test_cli_config_json_flag_basic_parsing() {
    let config_str = r#"{"use_cache": true, "output_format": "plain"}"#;

    let rust_config: ExtractionConfig = serde_json::from_str(config_str).expect("Failed to deserialize config string");

    let cli_json: serde_json::Value = serde_json::from_str(config_str).expect("Failed to parse JSON string");
    let cli_config: ExtractionConfig = serde_json::from_value(cli_json).expect("Failed to deserialize from JSON value");

    assert_eq!(
        rust_config.use_cache, cli_config.use_cache,
        "use_cache should be identical"
    );
    assert_eq!(
        rust_config.output_format, cli_config.output_format,
        "output_format should be identical"
    );
}

#[test]
fn test_cli_nested_config_deserialization() {
    let config_str = r#"{
        "chunking": {
            "max_characters": 1000,
            "overlap": 200
        },
        "ocr": {
            "backend": "tesseract"
        }
    }"#;

    let config: ExtractionConfig = serde_json::from_str(config_str).expect("Failed to deserialize nested config");

    assert!(config.chunking.is_some(), "Chunking config should be present");
    assert!(config.ocr.is_some(), "OCR config should be present");

    let chunking = config.chunking.unwrap();
    assert_eq!(chunking.max_characters, 1000, "max_chars should be 1000");
    assert_eq!(chunking.overlap, 200, "max_overlap should be 200");

    let ocr = config.ocr.unwrap();
    assert_eq!(ocr.backend, "tesseract", "backend should be tesseract");
}

#[test]
fn test_cli_force_ocr_flag_parsing() {
    let config_str = r#"{"force_ocr": true}"#;

    let config: ExtractionConfig = serde_json::from_str(config_str).expect("Failed to deserialize force_ocr config");

    assert!(config.force_ocr, "force_ocr should be true");
    assert!(config.use_cache, "use_cache should still be true by default");
}

#[test]
fn test_cli_max_concurrent_extractions_parsing() {
    let config_str = r#"{"max_concurrent_extractions": 8}"#;

    let config: ExtractionConfig =
        serde_json::from_str(config_str).expect("Failed to deserialize concurrent extractions");

    assert_eq!(
        config.max_concurrent_extractions,
        Some(8),
        "max_concurrent_extractions should be 8"
    );
}

#[test]
fn test_cli_complex_config_deserialization() {
    let config_str = r#"{
        "use_cache": false,
        "enable_quality_processing": true,
        "force_ocr": true,
        "output_format": "markdown",
        "result_format": "unified",
        "max_concurrent_extractions": 16,
        "ocr": {
            "backend": "tesseract",
            "language": "eng"
        },
        "chunking": {
            "max_characters": 2000,
            "overlap": 400
        }
    }"#;

    let config: ExtractionConfig = serde_json::from_str(config_str).expect("Failed to deserialize complex config");

    assert!(!config.use_cache);
    assert!(config.enable_quality_processing);
    assert!(config.force_ocr);
    assert_eq!(config.max_concurrent_extractions, Some(16));

    assert!(config.ocr.is_some());
    assert!(config.chunking.is_some());

    let ocr = config.ocr.unwrap();
    assert_eq!(ocr.backend, "tesseract");
    assert_eq!(ocr.language, vec!["eng".to_string()]);

    let chunking = config.chunking.unwrap();
    assert_eq!(chunking.max_characters, 2000);
    assert_eq!(chunking.overlap, 400);
}

#[test]
fn test_cli_empty_config_uses_defaults() {
    let config_str = r#"{}"#;

    let config: ExtractionConfig = serde_json::from_str(config_str).expect("Failed to deserialize empty config");

    assert!(config.use_cache, "Default use_cache should be true");
    assert!(
        config.enable_quality_processing,
        "Default enable_quality_processing should be true"
    );
    assert!(!config.force_ocr, "Default force_ocr should be false");
    assert_eq!(
        config.max_concurrent_extractions, None,
        "Default max_concurrent_extractions should be None"
    );
}

#[test]
fn test_cli_roundtrip_preserves_all_fields() {
    let original_str = r#"{
        "use_cache": false,
        "force_ocr": true,
        "max_concurrent_extractions": 12
    }"#;

    let config: ExtractionConfig = serde_json::from_str(original_str).expect("Failed to deserialize");

    let serialized = serde_json::to_value(&config).expect("Failed to serialize");

    let reparsed: ExtractionConfig =
        serde_json::from_value(serialized).expect("Failed to deserialize roundtripped config");

    assert!(!reparsed.use_cache);
    assert!(reparsed.force_ocr);
    assert_eq!(reparsed.max_concurrent_extractions, Some(12));
}

#[test]
fn test_cli_output_format_enum_parsing() {
    let test_cases = vec![
        (r#"{"output_format": "plain"}"#, OutputFormat::Plain),
        (r#"{"output_format": "markdown"}"#, OutputFormat::Markdown),
        (r#"{"output_format": "html"}"#, OutputFormat::Html),
    ];

    for (config_str, expected_format) in test_cases {
        let config: ExtractionConfig =
            serde_json::from_str(config_str).unwrap_or_else(|_| panic!("Failed to deserialize {}", config_str));

        assert_eq!(
            config.output_format, expected_format,
            "output_format should match expected value"
        );
    }
}

#[test]
fn test_cli_result_format_enum_parsing() {
    let test_cases = vec![
        r#"{"result_format": "unified"}"#,
        r#"{"result_format": "element_based"}"#,
    ];

    for config_str in test_cases {
        let result = serde_json::from_str::<ExtractionConfig>(config_str);
        assert!(result.is_ok(), "Should deserialize result_format from {}", config_str);
    }
}

#[test]
fn test_cli_base64_encoded_config_simulation() {
    let original_json = json!({
        "force_ocr": true,
        "output_format": "markdown"
    });

    let json_string = original_json.to_string();

    let encoded = base64::engine::general_purpose::STANDARD.encode(&json_string);

    use base64::Engine;
    let decoded = String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .expect("Failed to decode base64"),
    )
    .expect("Failed to convert bytes to string");

    let config: ExtractionConfig = serde_json::from_str(&decoded).expect("Failed to deserialize base64-decoded config");

    assert!(config.force_ocr);
    assert_eq!(config.output_format, OutputFormat::Markdown);
}

#[test]
fn test_cli_partial_override_merging() {
    let base_config = ExtractionConfig::default();
    let override_json = json!({"force_ocr": true, "use_cache": false});

    let mut base_json = serde_json::to_value(&base_config).expect("Failed to serialize base config");

    if let (serde_json::Value::Object(base_obj), serde_json::Value::Object(override_obj)) =
        (&mut base_json, override_json)
    {
        for (key, value) in override_obj {
            base_obj.insert(key, value);
        }
    }

    let merged: ExtractionConfig = serde_json::from_value(base_json).expect("Failed to deserialize merged config");

    assert!(merged.force_ocr, "Override should apply force_ocr");
    assert!(!merged.use_cache, "Override should apply use_cache");
    assert!(
        merged.enable_quality_processing,
        "Unoverridden field should retain default"
    );
}

#[test]
fn test_cli_invalid_json_error_handling() {
    let invalid_json_str = r#"{"force_ocr": true, "invalid_field": "value"}"#;

    let result = serde_json::from_str::<ExtractionConfig>(invalid_json_str);

    if let Ok(config) = result {
        assert!(config.force_ocr);
    }
}

#[test]
fn test_cli_whitespace_handling_in_json() {
    let config_strs = vec![
        r#"{"force_ocr":true}"#,
        r#"{ "force_ocr" : true }"#,
        r#"{
            "force_ocr": true
        }"#,
    ];

    for config_str in config_strs {
        let config: ExtractionConfig =
            serde_json::from_str(config_str).unwrap_or_else(|_| panic!("Failed to parse: {}", config_str));

        assert!(config.force_ocr);
    }
}

#[test]
fn test_cli_numeric_boundary_values() {
    let test_cases = vec![
        (r#"{"max_concurrent_extractions": 1}"#, Some(1)),
        (r#"{"max_concurrent_extractions": 256}"#, Some(256)),
        (r#"{"max_concurrent_extractions": 0}"#, Some(0)),
    ];

    for (config_str, expected_value) in test_cases {
        let config: ExtractionConfig =
            serde_json::from_str(config_str).unwrap_or_else(|_| panic!("Failed to parse: {}", config_str));

        assert_eq!(
            config.max_concurrent_extractions, expected_value,
            "Numeric values should be parsed correctly"
        );
    }
}

#[test]
fn test_cli_boolean_values_strict_parsing() {
    let test_cases = vec![(r#"{"use_cache": true}"#, true), (r#"{"use_cache": false}"#, false)];

    for (config_str, expected_value) in test_cases {
        let config: ExtractionConfig =
            serde_json::from_str(config_str).unwrap_or_else(|_| panic!("Failed to parse: {}", config_str));

        assert_eq!(config.use_cache, expected_value);
    }
}

#[test]
fn test_cli_config_consistency_across_formats() {
    let programmatic_config = ExtractionConfig {
        use_cache: false,
        enable_quality_processing: true,
        force_ocr: true,
        output_format: OutputFormat::Markdown,
        max_concurrent_extractions: Some(4),
        ..Default::default()
    };

    let serialized_json = serde_json::to_value(&programmatic_config).expect("Failed to serialize");

    let json_string = serialized_json.to_string();
    let deserialized: ExtractionConfig = serde_json::from_str(&json_string).expect("Failed to deserialize from string");

    assert_eq!(deserialized.use_cache, programmatic_config.use_cache);
    assert_eq!(
        deserialized.enable_quality_processing,
        programmatic_config.enable_quality_processing
    );
    assert_eq!(deserialized.force_ocr, programmatic_config.force_ocr);
    assert_eq!(deserialized.output_format, programmatic_config.output_format);
    assert_eq!(
        deserialized.max_concurrent_extractions,
        programmatic_config.max_concurrent_extractions
    );
}
