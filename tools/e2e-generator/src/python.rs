use crate::fixtures::{Assertions, Fixture, PluginAssertions, PluginTestSpec};
use anyhow::{Context, Result};
use camino::Utf8Path;
use itertools::Itertools;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;

const PYTHON_HELPERS_TEMPLATE: &str = r#"from __future__ import annotations

from collections.abc import Mapping
from pathlib import Path
from typing import Any

import pytest

from kreuzberg import (
    ChunkingConfig,
    ExtractionConfig,
    ImageExtractionConfig,
    LanguageDetectionConfig,
    OcrConfig,
    PdfConfig,
    PostProcessorConfig,
    TokenReductionConfig,
)

_WORKSPACE_ROOT = Path(__file__).resolve().parent.parent.parent.parent
_TEST_DOCUMENTS = _WORKSPACE_ROOT / "test_documents"


def resolve_document(relative: str) -> Path:
    """Return absolute path to a document in test_documents."""

    return _TEST_DOCUMENTS / relative


def build_config(config: dict[str, Any] | None) -> ExtractionConfig:
    """Construct an ExtractionConfig from a plain dictionary."""

    if not config:
        return ExtractionConfig()

    kwargs: dict[str, Any] = {}

    for key in ("use_cache", "enable_quality_processing", "force_ocr"):
        if key in config:
            kwargs[key] = config[key]

    if (ocr_data := config.get("ocr")) is not None:
        kwargs["ocr"] = OcrConfig(**ocr_data)

    if (chunking_data := config.get("chunking")) is not None:
        kwargs["chunking"] = ChunkingConfig(**chunking_data)

    if (images_data := config.get("images")) is not None:
        kwargs["images"] = ImageExtractionConfig(**images_data)

    if (pdf_options := config.get("pdf_options")) is not None:
        kwargs["pdf_options"] = PdfConfig(**pdf_options)

    if (token_reduction := config.get("token_reduction")) is not None:
        kwargs["token_reduction"] = TokenReductionConfig(**token_reduction)

    if (language_detection := config.get("language_detection")) is not None:
        kwargs["language_detection"] = LanguageDetectionConfig(**language_detection)

    if (postprocessor := config.get("postprocessor")) is not None:
        kwargs["postprocessor"] = PostProcessorConfig(**postprocessor)

    return ExtractionConfig(**kwargs)


def assert_expected_mime(result: Any, expected: list[str]) -> None:
    if not expected:
        return
    if not any(token in result.mime_type for token in expected):
        pytest.fail(f"Expected MIME {result.mime_type!r} to match one of {expected!r}")


def assert_min_content_length(result: Any, minimum: int) -> None:
    if len(result.content) < minimum:
        pytest.fail(
            f"Expected content length >= {minimum}, got {len(result.content)}"
        )


def assert_max_content_length(result: Any, maximum: int) -> None:
    if len(result.content) > maximum:
        pytest.fail(
            f"Expected content length <= {maximum}, got {len(result.content)}"
        )


def assert_content_contains_any(result: Any, snippets: list[str]) -> None:
    if not snippets:
        return
    lowered = result.content.lower()
    preview = result.content[:160]
    if not any(snippet.lower() in lowered for snippet in snippets):
        pytest.fail(
            f"Expected content to contain any of {snippets!r}. Preview: {preview!r}"
        )


def assert_content_contains_all(result: Any, snippets: list[str]) -> None:
    if not snippets:
        return
    lowered = result.content.lower()
    missing = [snippet for snippet in snippets if snippet.lower() not in lowered]
    if missing:
        pytest.fail(
            f"Expected content to contain all snippets {snippets!r}. Missing {missing!r}"
        )


def assert_table_count(result: Any, minimum: int | None, maximum: int | None) -> None:
    count = len(getattr(result, "tables", []) or [])
    if minimum is not None and count < minimum:
        pytest.fail(f"Expected at least {minimum} tables, found {count}")
    if maximum is not None and count > maximum:
        pytest.fail(f"Expected at most {maximum} tables, found {count}")


def assert_detected_languages(
    result: Any, expected: list[str], min_confidence: float | None
) -> None:
    if not expected:
        return
    languages = result.detected_languages
    if languages is None:
        pytest.fail("Expected detected languages but field is None")

    missing = [lang for lang in expected if lang not in languages]
    if missing:
        pytest.fail(f"Expected languages {expected!r}, missing {missing!r}")

    if min_confidence is not None:
        confidence = (
            result.metadata.get("confidence")
            if isinstance(result.metadata, Mapping)
            else None
        )
        if confidence is not None and confidence < min_confidence:
            pytest.fail(
                f"Expected confidence >= {min_confidence}, got {confidence}"
            )


def assert_metadata_expectation(result: Any, path: str, expectation: dict[str, Any]) -> None:
    value = _lookup_path(result.metadata, path)
    if value is None:
        pytest.fail(f"Metadata path '{path}' missing in {result.metadata!r}")

    if "eq" in expectation and not _values_equal(value, expectation["eq"]):
        pytest.fail(
            f"Expected metadata '{path}' == {expectation['eq']!r}, got {value!r}"
        )

    if "gte" in expectation:
        actual = float(value)
        if actual < float(expectation["gte"]):
            pytest.fail(
                f"Expected metadata '{path}' >= {expectation['gte']}, got {actual}"
            )

    if "lte" in expectation:
        actual = float(value)
        if actual > float(expectation["lte"]):
            pytest.fail(
                f"Expected metadata '{path}' <= {expectation['lte']}, got {actual}"
            )

    if "contains" in expectation:
        expected_values = expectation["contains"]
        if isinstance(value, str) and isinstance(expected_values, str):
            if expected_values not in value:
                pytest.fail(
                    f"Expected metadata '{path}' string to contain {expected_values!r}"
                )
        elif isinstance(value, (list, tuple, set)):
            missing = [item for item in expected_values if item not in value]
            if missing:
                pytest.fail(
                    f"Expected metadata '{path}' to contain {expected_values!r}, missing {missing!r}"
                )
        else:
            pytest.fail(
                f"Unsupported contains expectation for metadata '{path}': {value!r}"
            )

    if expectation.get("exists") is False:
        pytest.fail("exists=False not supported for metadata expectations")


def _lookup_path(metadata: Mapping[str, Any], path: str) -> Any:
    current: Any = metadata
    for segment in path.split("."):
        if not isinstance(current, Mapping) or segment not in current:
            return None
        current = current[segment]
    return current


def _values_equal(lhs: Any, rhs: Any) -> bool:
    if isinstance(lhs, str) and isinstance(rhs, str):
        return lhs == rhs
    if isinstance(lhs, (int, float)) and isinstance(rhs, (int, float)):
        return float(lhs) == float(rhs)
    if isinstance(lhs, bool) and isinstance(rhs, bool):
        return lhs is rhs
    return bool(lhs == rhs)
"#;

pub fn generate(fixtures: &[Fixture], output_root: &Utf8Path) -> Result<()> {
    let python_root = output_root.join("python");
    let tests_dir = python_root.join("tests");

    fs::create_dir_all(&tests_dir).context("Failed to create python tests directory")?;

    clean_tests(&tests_dir)?;
    write_init_files(&python_root)?;
    write_helpers(&python_root)?;

    // Separate document extraction and plugin API fixtures
    let doc_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_document_extraction()).collect();
    let api_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_plugin_api()).collect();

    // Generate document extraction tests
    let mut grouped = doc_fixtures
        .into_iter()
        .into_group_map_by(|fixture| fixture.category().to_string())
        .into_iter()
        .collect::<Vec<_>>();
    grouped.sort_by(|a, b| a.0.cmp(&b.0));

    for (category, mut fixtures) in grouped {
        fixtures.sort_by(|a, b| a.id.cmp(&b.id));
        let filename = format!("test_{}.py", sanitize_identifier(&category));
        let content = render_category(&category, &fixtures)?;
        fs::write(tests_dir.join(&filename), content)
            .with_context(|| format!("Failed to write Python test file {filename}"))?;
    }

    // Generate plugin API tests
    if !api_fixtures.is_empty() {
        generate_plugin_api_tests(&api_fixtures, &tests_dir)?;
    }

    Ok(())
}

fn clean_tests(dir: &Utf8Path) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir.as_std_path())? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|ext| ext == "py") {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("test_") || name == "helpers.py" {
                fs::remove_file(entry.path())?;
            }
        }
    }

    Ok(())
}

fn write_init_files(root: &Utf8Path) -> Result<()> {
    fs::create_dir_all(root.as_std_path())?;

    let init_root = root.join("__init__.py");
    fs::write(init_root.as_std_path(), "\"\"\"Generated E2E package.\"\"\"\n")?;

    let tests_dir = root.join("tests");
    fs::create_dir_all(tests_dir.as_std_path())?;
    let tests_init = tests_dir.join("__init__.py");
    fs::write(tests_init.as_std_path(), "\"\"\"Generated tests.\"\"\"\n")?;
    Ok(())
}

fn write_helpers(root: &Utf8Path) -> Result<()> {
    let helpers_path = root.join("tests").join("helpers.py");
    fs::write(helpers_path.as_std_path(), PYTHON_HELPERS_TEMPLATE).context("Failed to write helpers.py")?;
    Ok(())
}

fn render_category(category: &str, fixtures: &[&Fixture]) -> Result<String> {
    let mut buffer = String::new();
    writeln!(buffer, "# Auto-generated tests for {category} fixtures.")?;
    writeln!(buffer, "from __future__ import annotations\n")?;
    writeln!(buffer, "import pytest")?;
    writeln!(buffer)?;
    writeln!(buffer, "from kreuzberg import extract_file_sync")?;
    writeln!(buffer)?;
    writeln!(buffer, "from . import helpers\n")?;
    buffer.push('\n');

    for fixture in fixtures {
        buffer.push_str(&render_test(fixture)?);
        buffer.push('\n');
    }

    Ok(buffer)
}

fn render_test(fixture: &Fixture) -> Result<String> {
    let mut code = String::new();
    let test_name = format!("test_{}", sanitize_identifier(&fixture.id));
    writeln!(code, "def {test_name}() -> None:")?;
    writeln!(code, "    \"\"\"{}\"\"\"", escape_python_string(&fixture.description))?;
    writeln!(code)?;
    writeln!(
        code,
        "    document_path = helpers.resolve_document({})",
        python_string_literal(&fixture.document().path)
    )?;
    writeln!(code, "    if not document_path.exists():")?;
    writeln!(
        code,
        "        pytest.skip(f\"Skipping {}: missing document at {{document_path}}\")",
        fixture.id
    )?;
    writeln!(code)?;

    let config_literal = render_config_literal(&fixture.extraction().config);
    writeln!(code, "    config = helpers.build_config({})", config_literal)?;
    writeln!(code)?;

    writeln!(code, "    result = extract_file_sync(document_path, None, config)")?;
    writeln!(code)?;

    code.push_str(&render_assertions(&fixture.assertions()));

    Ok(code)
}

fn render_assertions(assertions: &Assertions) -> String {
    let mut buffer = String::new();

    if !assertions.expected_mime.is_empty() {
        writeln!(
            buffer,
            "    helpers.assert_expected_mime(result, {})",
            render_string_list(&assertions.expected_mime)
        )
        .unwrap();
    }
    if let Some(min) = assertions.min_content_length {
        writeln!(buffer, "    helpers.assert_min_content_length(result, {min})").unwrap();
    }
    if let Some(max) = assertions.max_content_length {
        writeln!(buffer, "    helpers.assert_max_content_length(result, {max})").unwrap();
    }
    if !assertions.content_contains_any.is_empty() {
        writeln!(
            buffer,
            "    helpers.assert_content_contains_any(result, {})",
            render_string_list(&assertions.content_contains_any)
        )
        .unwrap();
    }
    if !assertions.content_contains_all.is_empty() {
        writeln!(
            buffer,
            "    helpers.assert_content_contains_all(result, {})",
            render_string_list(&assertions.content_contains_all)
        )
        .unwrap();
    }
    if let Some(tables) = assertions.tables.as_ref() {
        let min_literal = tables
            .min
            .map(|value| value.to_string())
            .unwrap_or_else(|| "None".to_string());
        let max_literal = tables
            .max
            .map(|value| value.to_string())
            .unwrap_or_else(|| "None".to_string());
        writeln!(
            buffer,
            "    helpers.assert_table_count(result, {min_literal}, {max_literal})"
        )
        .unwrap();
    }
    if let Some(languages) = assertions.detected_languages.as_ref() {
        let expected = render_string_list(&languages.expects);
        let min_conf = languages
            .min_confidence
            .map(|v| v.to_string())
            .unwrap_or_else(|| "None".to_string());
        writeln!(
            buffer,
            "    helpers.assert_detected_languages(result, {expected}, {min_conf})"
        )
        .unwrap();
    }
    for (path, expectation) in &assertions.metadata {
        writeln!(
            buffer,
            "    helpers.assert_metadata_expectation(result, {}, {})",
            python_string_literal(path),
            render_python_value(expectation)
        )
        .unwrap();
    }

    if !buffer.ends_with('\n') {
        buffer.push('\n');
    }

    buffer
}

fn render_config_literal(config: &Map<String, Value>) -> String {
    if config.is_empty() {
        "None".to_string()
    } else {
        let value = Value::Object(config.clone());
        render_python_value(&value)
    }
}

fn render_string_list(values: &[String]) -> String {
    if values.is_empty() {
        "[]".to_string()
    } else {
        let parts = values
            .iter()
            .map(|value| python_string_literal(value))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{parts}]")
    }
}

fn render_python_value(value: &Value) -> String {
    match value {
        Value::Null => "None".to_string(),
        Value::Bool(b) => {
            if *b {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => python_string_literal(s),
        Value::Array(items) => {
            let parts = items.iter().map(render_python_value).collect::<Vec<_>>().join(", ");
            format!("[{parts}]")
        }
        Value::Object(map) => {
            let parts = map
                .iter()
                .map(|(key, value)| format!("{}: {}", python_string_literal(key), render_python_value(value)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{parts}}}")
        }
    }
}

fn sanitize_identifier(input: &str) -> String {
    let mut ident = input
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' => c.to_ascii_lowercase(),
            _ => '_',
        })
        .collect::<String>();
    while ident.contains("__") {
        ident = ident.replace("__", "_");
    }
    ident.trim_matches('_').to_string()
}

fn escape_python_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn python_string_literal(value: &str) -> String {
    format!("\"{}\"", escape_python_string(value))
}

// Plugin API test generation

fn generate_plugin_api_tests(fixtures: &[&Fixture], output_dir: &Utf8Path) -> Result<()> {
    let test_file = output_dir.join("test_plugin_apis.py");

    let mut content = String::new();

    // Header
    content.push_str("# Auto-generated from fixtures/plugin_api/ - DO NOT EDIT\n");
    content.push_str("\"\"\"\n");
    content.push_str("E2E tests for plugin/config/utility APIs.\n");
    content.push('\n');
    content.push_str("Generated from plugin API fixtures.\n");
    content.push_str("To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang python\n");
    content.push_str("\"\"\"\n\n");
    content.push_str("from __future__ import annotations\n\n");
    content.push_str("import os\n");
    content.push_str("from pathlib import Path\n\n");
    content.push_str("import kreuzberg\n");
    content.push_str("from kreuzberg import ExtractionConfig\n\n");

    // Generate test functions grouped by API category
    let grouped = group_by_category(fixtures);

    for (category, fixtures) in grouped {
        writeln!(&mut content, "\n# {} Tests\n", category_to_title(category))?;

        for fixture in fixtures {
            generate_python_test_function(fixture, &mut content)?;
        }
    }

    fs::write(&test_file, content).with_context(|| format!("Failed to write {test_file}"))?;

    Ok(())
}

fn group_by_category<'a>(fixtures: &[&'a Fixture]) -> BTreeMap<&'a str, Vec<&'a Fixture>> {
    let mut grouped: BTreeMap<&str, Vec<&Fixture>> = BTreeMap::new();
    for fixture in fixtures {
        let category = fixture.api_category.as_ref().unwrap().as_str();
        grouped.entry(category).or_default().push(fixture);
    }
    grouped
}

fn category_to_title(category: &str) -> String {
    category
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn generate_python_test_function(fixture: &Fixture, buf: &mut String) -> Result<()> {
    let test_spec = fixture.test_spec.as_ref().unwrap();
    let test_name = format!("test_{}", fixture.id);

    // Function signature
    match test_spec.pattern.as_str() {
        "config_from_file" | "mime_from_path" => {
            writeln!(buf, "def {}(tmp_path: Path) -> None:", test_name)?;
        }
        "config_discover" => {
            writeln!(buf, "def {}(tmp_path: Path, monkeypatch) -> None:", test_name)?;
        }
        _ => {
            writeln!(buf, "def {}() -> None:", test_name)?;
        }
    }

    // Docstring
    writeln!(buf, "    \"\"\"{}\"\"\"", escape_python_string(&fixture.description))?;

    // Generate test body based on pattern
    match test_spec.pattern.as_str() {
        "simple_list" => generate_simple_list_test(fixture, test_spec, buf)?,
        "clear_registry" => generate_clear_registry_test(fixture, test_spec, buf)?,
        "graceful_unregister" => generate_graceful_unregister_test(fixture, test_spec, buf)?,
        "config_from_file" => generate_config_from_file_test(fixture, test_spec, buf)?,
        "config_discover" => generate_config_discover_test(fixture, test_spec, buf)?,
        "mime_from_bytes" => generate_mime_from_bytes_test(fixture, test_spec, buf)?,
        "mime_from_path" => generate_mime_from_path_test(fixture, test_spec, buf)?,
        "mime_extension_lookup" => generate_mime_extension_lookup_test(fixture, test_spec, buf)?,
        _ => anyhow::bail!("Unknown test pattern: {}", test_spec.pattern),
    }

    writeln!(buf)?;
    Ok(())
}

fn generate_simple_list_test(_fixture: &Fixture, test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let func_name = &test_spec.function_call.name;
    let assertions = &test_spec.assertions;

    // Call the function
    writeln!(buf, "    result = kreuzberg.{}()", func_name)?;

    // Generate assertions
    writeln!(buf, "    assert isinstance(result, list)")?;

    if let Some(item_type) = &assertions.list_item_type {
        if item_type == "string" {
            writeln!(buf, "    assert all(isinstance(item, str) for item in result)")?;
        }
    }

    if let Some(contains) = &assertions.list_contains {
        writeln!(buf, "    assert \"{}\" in result", contains)?;
    }

    Ok(())
}

fn generate_clear_registry_test(_fixture: &Fixture, test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let func_name = &test_spec.function_call.name;

    // Call clear function
    writeln!(buf, "    kreuzberg.{}()", func_name)?;

    // Verify cleanup by calling list function
    let list_func = func_name.replace("clear_", "list_");
    writeln!(buf, "    result = kreuzberg.{}()", list_func)?;
    writeln!(buf, "    assert len(result) == 0")?;

    Ok(())
}

fn generate_graceful_unregister_test(_fixture: &Fixture, test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let func_name = &test_spec.function_call.name;
    let arg = &test_spec.function_call.args[0];
    let arg_str = arg.as_str().unwrap();

    // Should not raise
    writeln!(buf, "    kreuzberg.{}(\"{}\")", func_name, arg_str)?;

    Ok(())
}

fn generate_config_from_file_test(_fixture: &Fixture, test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let setup = test_spec.setup.as_ref().unwrap();
    let file_content = setup.temp_file_content.as_ref().unwrap();
    let file_name = setup.temp_file_name.as_ref().unwrap();

    // Create temp file
    writeln!(buf, "    config_path = tmp_path / \"{}\"", file_name)?;
    writeln!(buf, "    config_path.write_text(\"\"\"{}\"\"\")", file_content)?;
    writeln!(buf)?;

    // Load config
    writeln!(buf, "    config = ExtractionConfig.from_file(str(config_path))")?;
    writeln!(buf)?;

    // Generate assertions
    generate_object_property_assertions(&test_spec.assertions, buf)?;

    Ok(())
}

fn generate_config_discover_test(_fixture: &Fixture, test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let setup = test_spec.setup.as_ref().unwrap();
    let file_content = setup.temp_file_content.as_ref().unwrap();
    let file_name = setup.temp_file_name.as_ref().unwrap();
    let subdir = setup.subdirectory_name.as_ref().unwrap();

    // Create config in parent dir
    writeln!(buf, "    config_path = tmp_path / \"{}\"", file_name)?;
    writeln!(buf, "    config_path.write_text(\"\"\"{}\"\"\")", file_content)?;
    writeln!(buf)?;

    // Create subdirectory
    writeln!(buf, "    subdir = tmp_path / \"{}\"", subdir)?;
    writeln!(buf, "    subdir.mkdir()")?;
    writeln!(buf, "    monkeypatch.chdir(subdir)")?;
    writeln!(buf)?;

    // Discover config
    writeln!(buf, "    config = ExtractionConfig.discover()")?;
    writeln!(buf, "    assert config is not None")?;
    writeln!(buf)?;

    // Generate assertions
    generate_object_property_assertions(&test_spec.assertions, buf)?;

    Ok(())
}

fn generate_mime_from_bytes_test(_fixture: &Fixture, test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let setup = test_spec.setup.as_ref().unwrap();
    let test_data = setup.test_data.as_ref().unwrap();
    let func_name = &test_spec.function_call.name;

    // Convert test data to bytes (escape sequences already in JSON are preserved)
    writeln!(buf, "    test_bytes = b\"{}\"", test_data)?;
    writeln!(buf, "    result = kreuzberg.{}(test_bytes)", func_name)?;
    writeln!(buf)?;

    // Generate assertions
    if let Some(contains) = &test_spec.assertions.string_contains {
        writeln!(buf, "    assert \"{}\" in result.lower()", contains)?;
    }

    Ok(())
}

fn generate_mime_from_path_test(_fixture: &Fixture, test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let setup = test_spec.setup.as_ref().unwrap();
    let file_name = setup.temp_file_name.as_ref().unwrap();
    let file_content = setup.temp_file_content.as_ref().unwrap();
    let func_name = &test_spec.function_call.name;

    // Create temp file
    writeln!(buf, "    test_file = tmp_path / \"{}\"", file_name)?;
    writeln!(buf, "    test_file.write_text(\"{}\")", file_content)?;
    writeln!(buf)?;

    // Detect MIME
    writeln!(buf, "    result = kreuzberg.{}(str(test_file))", func_name)?;
    writeln!(buf)?;

    // Generate assertions
    if let Some(contains) = &test_spec.assertions.string_contains {
        writeln!(buf, "    assert \"{}\" in result.lower()", contains)?;
    }

    Ok(())
}

fn generate_mime_extension_lookup_test(_fixture: &Fixture, test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let func_name = &test_spec.function_call.name;
    let arg = &test_spec.function_call.args[0];
    let mime_type = arg.as_str().unwrap();

    writeln!(buf, "    result = kreuzberg.{}(\"{}\")", func_name, mime_type)?;
    writeln!(buf, "    assert isinstance(result, list)")?;

    if let Some(contains) = &test_spec.assertions.list_contains {
        writeln!(buf, "    assert \"{}\" in result", contains)?;
    }

    Ok(())
}

fn generate_object_property_assertions(assertions: &PluginAssertions, buf: &mut String) -> Result<()> {
    for prop in &assertions.object_properties {
        let parts: Vec<&str> = prop.path.split('.').collect();

        if let Some(exists) = prop.exists {
            if exists {
                let mut path = "config".to_string();
                for part in &parts {
                    writeln!(buf, "    assert {}.{} is not None", path, part)?;
                    path = format!("{}.{}", path, part);
                }
            }
        }

        if let Some(value) = &prop.value {
            let path = format!("config.{}", prop.path);
            match value {
                Value::Number(n) => writeln!(buf, "    assert {} == {}", path, n)?,
                Value::Bool(b) => writeln!(buf, "    assert {} is {}", path, if *b { "True" } else { "False" })?,
                Value::String(s) => writeln!(buf, "    assert {} == \"{}\"", path, s)?,
                _ => {}
            }
        }
    }

    Ok(())
}
