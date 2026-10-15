use super::*;

fn run_in_isolated_environment(name: &str, variables: &[(&str, &str)]) -> bool {
    if std::env::var("XBERG_ENV_OVERRIDE_CHILD").as_deref() == Ok(name) {
        return true;
    }
    let output = isolated_test_command(name)
        .env("XBERG_ENV_OVERRIDE_CHILD", name)
        .env("RUST_MIN_STACK", "16777216")
        .envs(variables.iter().copied())
        .output()
        .expect("isolated environment override test");
    assert_child_succeeded(&output, name);
    false
}

fn qualified_test_name(name: &str) -> String {
    let module = module_path!().strip_prefix("xberg::").expect("crate module prefix");
    format!("{module}::{name}")
}

fn isolated_test_command(name: &str) -> std::process::Command {
    let mut command = std::process::Command::new(std::env::current_exe().expect("test executable"));
    command.args(["--exact", &qualified_test_name(name), "--nocapture"]);
    // CI's system ORT needs inherited loader paths; preserve every non-XBERG variable. ~keep
    // Snapshot inheritance so parallel tests cannot add overrides before spawn. ~keep
    command
        .env_clear()
        .envs(std::env::vars_os().filter(|(key, _)| !key.to_string_lossy().to_ascii_uppercase().starts_with("XBERG_")));
    command
}

fn assert_child_succeeded(output: &std::process::Output, name: &str) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("running 1 test"),
        "child did not run exactly one test: {stdout}"
    );
    let expected = format!("test {} ... ok", qualified_test_name(name));
    assert!(
        stdout.lines().any(|line| line == expected),
        "expected named test was not successful: {stdout}"
    );
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn should_reject_child_filter_that_runs_no_tests() {
    let name = "nonexistent_environment_override_negative_control";
    let output = isolated_test_command(name)
        .output()
        .expect("zero-test negative control");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("running 0 tests"));
    assert!(std::panic::catch_unwind(|| assert_child_succeeded(&output, name)).is_err());
}

#[test]
fn should_inherit_runtime_environment_without_inheriting_xberg_overrides() {
    const NAME: &str = "should_inherit_runtime_environment_without_inheriting_xberg_overrides";
    if std::env::var("XBERG_ENV_OVERRIDE_CHILD").as_deref() == Ok(NAME) {
        assert_eq!(
            std::env::var("ENV_OVERRIDE_RUNTIME_SENTINEL").as_deref(),
            Ok("runtime-inherited")
        );
        assert!(std::env::var_os("XBERG_CACHE_ENABLED").is_none());
        assert!(std::env::var_os("xberg_runtime_test_override").is_none());
        assert!(std::env::var_os("XBERG_ENV_INHERITANCE_STAGE").is_none());
        let mut config = ExtractionConfig::default();
        config
            .apply_env_overrides()
            .expect("only the explicit case override applies");
        assert_eq!((config.use_cache, config.disable_ocr), (true, true));
        return;
    }
    if std::env::var("XBERG_ENV_INHERITANCE_STAGE").as_deref() == Ok("seeded") {
        assert_eq!(
            std::env::var("ENV_OVERRIDE_RUNTIME_SENTINEL").as_deref(),
            Ok("runtime-inherited")
        );
        assert_eq!(
            std::env::var("XBERG_CACHE_ENABLED").as_deref(),
            Ok("accidental-override")
        );
        assert!(!run_in_isolated_environment(NAME, &[("XBERG_DISABLE_OCR", "1")]));
        return;
    }
    let output = isolated_test_command(NAME)
        .env("XBERG_ENV_INHERITANCE_STAGE", "seeded")
        .env("ENV_OVERRIDE_RUNTIME_SENTINEL", "runtime-inherited")
        .env("XBERG_CACHE_ENABLED", "accidental-override")
        .env("xberg_runtime_test_override", "lowercase-override")
        .output()
        .expect("seeded intermediate process");
    assert_child_succeeded(&output, NAME);
}

fn validation_message(error: XbergError) -> String {
    match error {
        XbergError::Validation { message, source: None } => message,
        other => panic!("expected a validation error without a source, got {other:?}"),
    }
}

#[test]
fn should_accept_case_insensitive_booleans_and_numeric_ocr_flags() {
    if !run_in_isolated_environment(
        "should_accept_case_insensitive_booleans_and_numeric_ocr_flags",
        &[
            ("XBERG_CACHE_ENABLED", "TrUe"),
            ("XBERG_DISABLE_OCR", "1"),
            ("XBERG_STRICT_PARSE", "0"),
        ],
    ) {
        return;
    }
    let mut config = ExtractionConfig {
        use_cache: false,
        strict_parse: true,
        ..Default::default()
    };
    config.apply_env_overrides().expect("supported boolean spellings");
    assert_eq!(
        (config.use_cache, config.disable_ocr, config.strict_parse),
        (true, true, false)
    );
}

#[test]
fn should_reject_numeric_cache_flag_before_applying_later_overrides() {
    if !run_in_isolated_environment(
        "should_reject_numeric_cache_flag_before_applying_later_overrides",
        &[("XBERG_CACHE_ENABLED", "1"), ("XBERG_DISABLE_OCR", "1")],
    ) {
        return;
    }
    let mut config = ExtractionConfig::default();
    let error = config
        .apply_env_overrides()
        .expect_err("numeric cache flag is unsupported");
    assert_eq!(
        validation_message(error),
        "Invalid value for XBERG_CACHE_ENABLED: '1'. Must be 'true' or 'false'."
    );
    assert!(!config.disable_ocr);
}

#[test]
fn should_validate_chunk_size_against_existing_overlap_before_override() {
    if !run_in_isolated_environment(
        "should_validate_chunk_size_against_existing_overlap_before_override",
        &[
            ("XBERG_OCR_LANGUAGE", "fra"),
            ("XBERG_CHUNKING_MAX_CHARS", "1"),
            ("XBERG_CHUNKING_MAX_OVERLAP", "0"),
        ],
    ) {
        return;
    }
    let mut config = ExtractionConfig::default();
    let defaults = ChunkingConfig::default();
    let error = config
        .apply_env_overrides()
        .expect_err("existing overlap is validated first");
    assert_eq!(
        validation_message(error),
        format!("max_overlap ({}) must be less than max_chars (1)", defaults.overlap)
    );
    assert_eq!(config.ocr.expect("earlier override is retained").language, ["fra"]);
    let chunking = config.chunking.expect("parent is created before validation");
    assert_eq!(
        (chunking.max_characters, chunking.overlap),
        (defaults.max_characters, defaults.overlap)
    );
}

#[test]
fn should_preserve_integer_parse_error_without_creating_parent() {
    if !run_in_isolated_environment(
        "should_preserve_integer_parse_error_without_creating_parent",
        &[("XBERG_CHUNKING_MAX_CHARS", " 42")],
    ) {
        return;
    }
    let mut config = ExtractionConfig::default();
    let error = config
        .apply_env_overrides()
        .expect_err("integer whitespace is rejected");
    assert_eq!(
        validation_message(error),
        "Invalid value for XBERG_CHUNKING_MAX_CHARS: ' 42'. Must be a positive integer."
    );
    assert!(config.chunking.is_none());
}

#[test]
fn should_forward_credentials_after_creating_vlm_without_enabling_structured_extraction() {
    if !run_in_isolated_environment(
        "should_forward_credentials_after_creating_vlm_without_enabling_structured_extraction",
        &[
            ("XBERG_VLM_OCR_MODEL", "provider/vision"),
            ("XBERG_LLM_API_KEY", "example-key"),
            ("XBERG_LLM_BASE_URL", "https://example.test"),
        ],
    ) {
        return;
    }
    let mut config = ExtractionConfig::default();
    config
        .apply_env_overrides()
        .expect("credentials configure the explicitly selected model");
    assert!(config.structured_extraction.is_none());
    let vlm = config.ocr.expect("OCR config").vlm_config.expect("VLM config");
    assert_eq!(vlm.model, "provider/vision");
    assert_eq!(vlm.api_key.as_deref(), Some("example-key"));
    assert_eq!(vlm.base_url.as_deref(), Some("https://example.test"));
}

#[test]
fn should_retain_embedding_mutation_when_conflicting_plugin_is_rejected() {
    if !run_in_isolated_environment(
        "should_retain_embedding_mutation_when_conflicting_plugin_is_rejected",
        &[
            ("XBERG_VLM_EMBEDDING_MODEL", "provider/embedding"),
            ("XBERG_EMBEDDING_PLUGIN_NAME", "custom"),
        ],
    ) {
        return;
    }
    let mut config = ExtractionConfig::default();
    let error = config
        .apply_env_overrides()
        .expect_err("two embedding backends conflict");
    assert_eq!(
        validation_message(error),
        "XBERG_EMBEDDING_PLUGIN_NAME and XBERG_VLM_EMBEDDING_MODEL are mutually exclusive — set one or the other, not both."
    );
    let embedding = config
        .chunking
        .expect("chunking created first")
        .embedding
        .expect("embedding applied first");
    match embedding.model {
        super::super::super::processing::EmbeddingModelType::Llm { llm } => assert_eq!(llm.model, "provider/embedding"),
        other => panic!("expected LLM embedding, got {other:?}"),
    }
}

#[test]
fn should_trim_csv_prefixes_and_preserve_duplicates() {
    if !run_in_isolated_environment(
        "should_trim_csv_prefixes_and_preserve_duplicates",
        &[
            ("XBERG_CSV_DELIMITER", "|"),
            ("XBERG_CSV_COMMENT_PREFIXES", " #, , // ,#"),
        ],
    ) {
        return;
    }
    let mut config = ExtractionConfig::default();
    config.apply_env_overrides().expect("valid CSV overrides");
    let csv = config.csv.expect("CSV parent");
    assert_eq!(csv.delimiter.as_deref(), Some("|"));
    assert_eq!(csv.comment_prefixes, ["#", "//", "#"]);
}
