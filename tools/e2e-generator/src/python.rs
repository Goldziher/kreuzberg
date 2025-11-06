use crate::fixtures::{Assertions, Fixture};
use anyhow::{Context, Result};
use camino::Utf8Path;
use itertools::Itertools;
use serde_json::{Map, Value};
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

    let mut grouped = fixtures
        .iter()
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
        python_string_literal(&fixture.document.path)
    )?;
    writeln!(code, "    if not document_path.exists():")?;
    writeln!(
        code,
        "        pytest.skip(f\"Skipping {}: missing document at {{document_path}}\")",
        fixture.id
    )?;
    writeln!(code)?;

    let config_literal = render_config_literal(&fixture.extraction.config);
    writeln!(code, "    config = helpers.build_config({})", config_literal)?;
    writeln!(code)?;

    writeln!(code, "    result = extract_file_sync(document_path, None, config)")?;
    writeln!(code)?;

    code.push_str(&render_assertions(&fixture.assertions));

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
