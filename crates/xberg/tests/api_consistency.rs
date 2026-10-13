//! API consistency tests for ExtractionConfig and related types.
//!
//! This test suite validates that:
//! 1. ExtractionConfig serialization is complete with all fields
//! 2. All required configuration fields are present
//! 3. Configuration types maintain consistency across different formats
//! 4. No configuration fields are accidentally hidden or lost

use serde_json::json;
use xberg::core::config::ExtractionConfig;
use xberg::core::config::OutputFormat;
#[cfg(feature = "tree-sitter")]
use xberg::core::config::{TreeSitterConfig, TreeSitterProcessConfig};

#[test]
fn test_extraction_config_serialization_includes_all_fields() {
    let config = ExtractionConfig::default();
    let json = serde_json::to_value(&config).expect("Failed to serialize config");

    assert!(
        json.get("use_cache").is_some(),
        "Missing 'use_cache' field in serialized config"
    );
    assert!(
        json.get("enable_quality_processing").is_some(),
        "Missing 'enable_quality_processing' field"
    );
    assert!(
        json.get("force_ocr").is_some(),
        "Missing 'force_ocr' field in serialized config"
    );
    assert!(
        json.get("max_concurrent_extractions").is_some(),
        "Missing 'max_concurrent_extractions' field"
    );
    assert!(
        json.get("result_format").is_some(),
        "Missing 'result_format' field in serialized config"
    );
    assert!(
        json.get("output_format").is_some(),
        "Missing 'output_format' field in serialized config"
    );
}

#[test]
fn test_extraction_config_defaults_are_correct() {
    let config = ExtractionConfig::default();

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
fn test_extraction_config_serialization_roundtrip() {
    let config = ExtractionConfig::default();

    let json_string = serde_json::to_string(&config).expect("Failed to serialize");

    let deserialized: ExtractionConfig =
        serde_json::from_str(&json_string).expect("Failed to deserialize config from JSON");

    assert_eq!(
        config.use_cache, deserialized.use_cache,
        "use_cache should survive roundtrip"
    );
    assert_eq!(
        config.enable_quality_processing, deserialized.enable_quality_processing,
        "enable_quality_processing should survive roundtrip"
    );
    assert_eq!(
        config.force_ocr, deserialized.force_ocr,
        "force_ocr should survive roundtrip"
    );
    assert_eq!(
        config.result_format, deserialized.result_format,
        "result_format should survive roundtrip"
    );
    assert_eq!(
        config.output_format, deserialized.output_format,
        "output_format should survive roundtrip"
    );
}

#[test]
fn test_extraction_config_json_structure() {
    let config = ExtractionConfig::default();
    let json = serde_json::to_value(&config).expect("Failed to serialize config");

    let obj = json.as_object().expect("Config should serialize as object");

    let expected_fields = vec![
        "use_cache",
        "enable_quality_processing",
        "force_ocr",
        "max_concurrent_extractions",
        "result_format",
        "output_format",
    ];

    for field in expected_fields {
        assert!(obj.contains_key(field), "Missing field in JSON: {}", field);
    }
}

#[test]
fn test_extraction_config_values_are_correct_types() {
    let config = ExtractionConfig::default();
    let json = serde_json::to_value(&config).expect("Failed to serialize config");

    assert!(
        json.get("use_cache").expect("Value not found").is_boolean(),
        "use_cache should be boolean"
    );
    assert!(
        json.get("enable_quality_processing")
            .expect("Value not found")
            .is_boolean(),
        "enable_quality_processing should be boolean"
    );
    assert!(
        json.get("force_ocr").expect("Value not found").is_boolean(),
        "force_ocr should be boolean"
    );
    assert!(
        json.get("result_format").expect("Value not found").is_string(),
        "result_format should be string"
    );
    assert!(
        json.get("output_format").expect("Value not found").is_string(),
        "output_format should be string"
    );
}

#[test]
fn test_extraction_config_with_custom_values() {
    let config = ExtractionConfig {
        use_cache: false,
        force_ocr: true,
        max_concurrent_extractions: Some(8),
        ..ExtractionConfig::default()
    };

    let json = serde_json::to_value(&config).expect("Failed to serialize");

    assert_eq!(json.get("use_cache").expect("Value not found"), &json!(false));
    assert_eq!(json.get("force_ocr").expect("Value not found"), &json!(true));
    assert_eq!(
        json.get("max_concurrent_extractions").expect("Value not found"),
        &json!(8)
    );
}

#[test]
fn test_extraction_config_partial_json_parsing() {
    let partial_json = json!({
        "use_cache": false,
    });

    let config: ExtractionConfig = serde_json::from_value(partial_json).expect("Failed to parse partial config");

    assert!(!config.use_cache, "Explicit use_cache should be respected");
    assert!(
        config.enable_quality_processing,
        "Omitted enable_quality_processing should use default"
    );
    assert!(!config.force_ocr, "Omitted force_ocr should use default");
}

#[test]
fn test_extraction_config_empty_json_uses_defaults() {
    let empty_json = json!({});

    let config: ExtractionConfig = serde_json::from_value(empty_json).expect("Failed to parse empty config");

    let default_config = ExtractionConfig::default();
    assert_eq!(config.use_cache, default_config.use_cache);
    assert_eq!(
        config.enable_quality_processing,
        default_config.enable_quality_processing
    );
    assert_eq!(config.force_ocr, default_config.force_ocr);
    assert_eq!(config.result_format, default_config.result_format);
    assert_eq!(config.output_format, default_config.output_format);
}

#[test]
fn test_extraction_config_output_format_valid_values() {
    let json_plain = json!({"output_format": "plain"});
    let config_plain: ExtractionConfig =
        serde_json::from_value(json_plain).expect("Failed to parse plain output_format");
    assert_eq!(config_plain.output_format, OutputFormat::Plain);

    let json_markdown = json!({"output_format": "markdown"});
    let config_markdown: ExtractionConfig =
        serde_json::from_value(json_markdown).expect("Failed to parse markdown output_format");
    assert_eq!(config_markdown.output_format, OutputFormat::Markdown);

    let json_html = json!({"output_format": "html"});
    let config_html: ExtractionConfig = serde_json::from_value(json_html).expect("Failed to parse html output_format");
    assert_eq!(config_html.output_format, OutputFormat::Html);
}

#[test]
fn test_extraction_config_result_format_valid_values() {
    let json_unified = json!({"result_format": "unified"});
    let config_unified: ExtractionConfig =
        serde_json::from_value(json_unified).expect("Failed to parse unified result_format");
    let _ = config_unified.result_format;
}

#[test]
fn test_extraction_config_no_unknown_fields_in_default() {
    let config = ExtractionConfig::default();
    let json = serde_json::to_value(&config).expect("Failed to serialize");
    let obj = json.as_object().expect("Should be object");

    let expected_fields = vec![
        "use_cache",
        "enable_quality_processing",
        "ocr",
        "force_ocr",
        "ocr_strategy",
        "disable_ocr",
        // GH#1752 split three behaviours out of "is an `ocr` block present": each is an
        // Option<bool> whose None reproduces the previous derived behaviour. ~keep
        "ocr_near_empty_fallback",
        "ocr_scanned_page_quality_gate",
        "ocr_embedded_images",
        "chunking",
        "content_filter",
        "images",
        "pdf_options",
        "token_reduction",
        "language_detection",
        "pages",
        "keywords",
        "postprocessor",
        "html_options",
        "html_output",
        "max_concurrent_extractions",
        "result_format",
        "output_format",
        "escape_markdown",
        "table_anchors",
        "jupyter_cell_rendering",
        "apply_notebook_cell_tags",
        "include_document_structure",
        "security_limits",
        "acceleration",
        "cache_namespace",
        "cache_ttl_secs",
        "concurrency",
        "csv",
        "email",
        "geojson",
        "layout",
        "max_archive_depth",
        "max_embedded_file_bytes",
        "mime_detection_policy",
        "strict_parse",
        "extraction_timeout_secs",
        "tree_sitter",
        "use_layout_for_markdown",
        "transcription",
        "url",
    ];

    for key in obj.keys() {
        assert!(
            expected_fields.contains(&key.as_str()),
            "Unexpected field in config: {}",
            key
        );
    }
}

#[test]
fn test_extraction_config_needs_image_processing() {
    let mut config = ExtractionConfig::default();

    assert!(
        !config.needs_image_processing(),
        "Default config should not need image processing"
    );

    config.ocr = Some(xberg::OcrConfig {
        backend: "tesseract".to_string(),
        language: vec!["eng".to_string()],
        ..Default::default()
    });
    assert!(
        config.needs_image_processing(),
        "Config with OCR should need image processing"
    );

    config.ocr = None;
    config.images = Some(xberg::ImageExtractionConfig {
        extract_images: true,
        target_dpi: 150,
        max_image_dimension: 2000,
        inject_placeholders: true,
        auto_adjust_dpi: true,
        min_dpi: 72,
        max_dpi: 600,
        max_images_per_page: None,
        classify: true,
        include_page_rasters: false,
        run_ocr_on_images: true,
        ocr_text_only: false,
        append_ocr_text: false,
        include_data_base64: false,
        output_format: xberg::core::config::extraction::ImageOutputFormat::Native,
        #[cfg(feature = "svg")]
        svg: xberg::core::config::extraction::SvgOptions::default(),
    });
    assert!(
        config.needs_image_processing(),
        "Config with image extraction should need image processing"
    );
}

#[test]
fn test_output_format_serialization_lowercase() {
    let json = serde_json::json!({"output_format": "markdown"});
    let config: ExtractionConfig = serde_json::from_value(json).expect("Failed to parse");
    let reserialized = serde_json::to_value(&config).expect("Failed to reserialize");

    assert_eq!(reserialized["output_format"], "markdown");
}

#[test]
fn test_extraction_config_field_presence_consistency() {
    let config = ExtractionConfig::default();
    let json1 = serde_json::to_value(&config).expect("Failed to serialize");

    let config2 = ExtractionConfig {
        force_ocr: true,
        ..ExtractionConfig::default()
    };
    let json2 = serde_json::to_value(&config2).expect("Failed to serialize");

    let keys1: Vec<_> = json1.as_object().expect("Expected object value").keys().collect();
    let keys2: Vec<_> = json2.as_object().expect("Expected object value").keys().collect();

    assert_eq!(keys1.len(), keys2.len(), "Configs should have same number of keys");
}

#[test]
fn test_output_format_all_variants() {
    let formats = vec![
        OutputFormat::Plain,
        OutputFormat::Markdown,
        OutputFormat::Html,
        OutputFormat::Djot,
    ];

    for fmt in &formats {
        let serialized = serde_json::to_value(fmt.clone()).expect("Failed to serialize");
        let deserialized: OutputFormat = serde_json::from_value(serialized).expect("Failed to deserialize");
        assert_eq!(*fmt, deserialized, "Format should survive roundtrip");
    }
}

#[test]
fn test_include_document_structure_default_is_false() {
    let config = ExtractionConfig::default();
    assert!(
        !config.include_document_structure,
        "Default include_document_structure should be false"
    );
}

#[test]
fn test_include_document_structure_serialization_roundtrip() {
    let config = ExtractionConfig {
        include_document_structure: true,
        ..ExtractionConfig::default()
    };

    let json_string = serde_json::to_string(&config).expect("Failed to serialize");

    let deserialized: ExtractionConfig =
        serde_json::from_str(&json_string).expect("Failed to deserialize config from JSON");

    assert_eq!(
        config.include_document_structure, deserialized.include_document_structure,
        "include_document_structure should survive roundtrip"
    );
    assert!(
        deserialized.include_document_structure,
        "Deserialized include_document_structure should be true"
    );

    let config_false = ExtractionConfig {
        include_document_structure: false,
        ..ExtractionConfig::default()
    };

    let json_string_false = serde_json::to_string(&config_false).expect("Failed to serialize");
    let deserialized_false: ExtractionConfig =
        serde_json::from_str(&json_string_false).expect("Failed to deserialize config from JSON");

    assert!(
        !deserialized_false.include_document_structure,
        "Explicitly false include_document_structure should survive roundtrip"
    );
}

#[cfg(feature = "tree-sitter")]
#[test]
fn test_tree_sitter_process_config_defaults() {
    let config = TreeSitterProcessConfig::default();
    assert!(config.structure, "Default structure should be true");
    assert!(config.imports, "Default imports should be true");
    assert!(config.exports, "Default exports should be true");
    assert!(!config.comments, "Default comments should be false");
    assert!(!config.docstrings, "Default docstrings should be false");
    assert!(!config.symbols, "Default symbols should be false");
    assert!(!config.diagnostics, "Default diagnostics should be false");
    assert!(config.chunk_max_size.is_none(), "Default chunk_max_size should be None");
}

#[cfg(feature = "tree-sitter")]
#[test]
fn test_tree_sitter_config_defaults() {
    let config = TreeSitterConfig::default();
    assert!(config.cache_dir.is_none(), "Default cache_dir should be None");
    assert!(config.languages.is_none(), "Default languages should be None");
    assert!(config.groups.is_none(), "Default groups should be None");
    assert!(config.process.structure, "Default process.structure should be true");
}

#[cfg(feature = "tree-sitter")]
#[test]
fn test_tree_sitter_config_serialization_roundtrip() {
    let config = TreeSitterConfig {
        enabled: true,
        cache_dir: Some("/tmp/grammars".into()),
        languages: Some(vec!["python".to_string(), "rust".to_string()]),
        groups: Some(vec!["web".to_string()]),
        process: TreeSitterProcessConfig {
            structure: true,
            imports: true,
            exports: false,
            comments: true,
            docstrings: true,
            symbols: false,
            diagnostics: false,
            data_extraction: false,
            chunk_max_size: Some(4000),
            content_mode: Default::default(),
        },
    };

    let json_string = serde_json::to_string(&config).expect("Failed to serialize");
    let deserialized: TreeSitterConfig = serde_json::from_str(&json_string).expect("Failed to deserialize");

    assert_eq!(config.cache_dir, deserialized.cache_dir);
    assert_eq!(config.languages, deserialized.languages);
    assert_eq!(config.groups, deserialized.groups);
    assert_eq!(config.process.structure, deserialized.process.structure);
    assert_eq!(config.process.exports, deserialized.process.exports);
    assert_eq!(config.process.comments, deserialized.process.comments);
    assert_eq!(config.process.docstrings, deserialized.process.docstrings);
    assert_eq!(config.process.chunk_max_size, deserialized.process.chunk_max_size);
}

#[cfg(feature = "tree-sitter")]
#[test]
fn test_tree_sitter_config_in_extraction_config_roundtrip() {
    let config = ExtractionConfig {
        tree_sitter: Some(TreeSitterConfig {
            languages: Some(vec!["python".to_string()]),
            ..TreeSitterConfig::default()
        }),
        ..ExtractionConfig::default()
    };

    let json_string = serde_json::to_string(&config).expect("Failed to serialize");
    let deserialized: ExtractionConfig = serde_json::from_str(&json_string).expect("Failed to deserialize");

    let ts = deserialized
        .tree_sitter
        .expect("tree_sitter should be present after roundtrip");
    assert_eq!(ts.languages, Some(vec!["python".to_string()]));
    assert!(ts.process.structure, "process.structure should default to true");
}

#[cfg(feature = "tree-sitter")]
#[test]
fn test_tree_sitter_partial_json_parsing() {
    let json = json!({
        "tree_sitter": {
            "languages": ["rust"],
            "process": {
                "comments": true
            }
        }
    });

    let config: ExtractionConfig = serde_json::from_value(json).expect("Failed to parse");
    let ts = config.tree_sitter.expect("tree_sitter should be present");
    assert_eq!(ts.languages, Some(vec!["rust".to_string()]));
    assert!(ts.groups.is_none(), "Omitted groups should be None");
    assert!(ts.process.comments, "Explicit comments=true should be respected");
    assert!(ts.process.structure, "Omitted structure should default to true");
    assert!(!ts.process.symbols, "Omitted symbols should default to false");
}

#[cfg(feature = "tree-sitter")]
#[test]
fn test_tree_sitter_process_config_all_fields_serialized() {
    let config = TreeSitterProcessConfig {
        structure: false,
        imports: false,
        exports: false,
        comments: true,
        docstrings: true,
        symbols: true,
        diagnostics: true,
        data_extraction: true,
        chunk_max_size: Some(2000),
        content_mode: Default::default(),
    };

    let json = serde_json::to_value(&config).expect("Failed to serialize");
    let obj = json.as_object().expect("Should be object");

    let expected_fields = [
        "structure",
        "imports",
        "exports",
        "comments",
        "docstrings",
        "symbols",
        "diagnostics",
        "data_extraction",
        "chunk_max_size",
    ];
    for field in expected_fields {
        assert!(obj.contains_key(field), "Missing field: {field}");
    }
}

#[cfg(feature = "tree-sitter")]
#[test]
fn test_format_metadata_code_variant_serialization() {
    use xberg::types::metadata::{CodeMetadata, FormatMetadata};

    let metadata = FormatMetadata::Code(CodeMetadata {
        chunks: Vec::new(),
        data: None,
    });
    let json = serde_json::to_value(&metadata).expect("Failed to serialize FormatMetadata::Code");

    assert_eq!(json["format_type"], "code", "format_type tag should be 'code'");
    assert!(json["chunks"].is_array(), "chunks should serialize as an array");
}

#[cfg(feature = "tree-sitter")]
#[test]
fn test_tslp_types_reexported() {
    let _: xberg::ProcessConfig = xberg::ProcessConfig::new("rust");

    let _kind = xberg::StructureKind::Function;
    let _kind = xberg::StructureKind::Class;
    let _kind = xberg::StructureKind::Method;

    let _kind = xberg::ExportKind::Named;
    let _kind = xberg::ExportKind::Default;

    let _kind = xberg::CommentKind::Line;
    let _kind = xberg::CommentKind::Block;

    let _sev = xberg::DiagnosticSeverity::Error;
    let _sev = xberg::DiagnosticSeverity::Warning;

    let metrics = xberg::FileMetrics::default();
    assert_eq!(metrics.total_lines, 0);
}
