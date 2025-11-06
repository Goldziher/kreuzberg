use crate::fixtures::{Assertions, Fixture};
use anyhow::{Context, Result};
use camino::Utf8Path;
use itertools::Itertools;
use serde_json::{Map, Value};
use std::fmt::Write as _;
use std::fs;
use std::io::Write;

pub fn generate(fixtures: &[Fixture], output_root: &Utf8Path) -> Result<()> {
    let rust_root = output_root.join("rust");
    let tests_dir = rust_root.join("tests");

    fs::create_dir_all(&tests_dir).context("Failed to create Rust tests directory")?;

    clean_rs_files(&tests_dir)?;

    let mut grouped = fixtures
        .iter()
        .into_group_map_by(|fixture| fixture.category().to_string())
        .into_iter()
        .collect::<Vec<_>>();
    grouped.sort_by(|a, b| a.0.cmp(&b.0));

    for (category, mut fixtures) in grouped {
        fixtures.sort_by(|a, b| a.id.cmp(&b.id));
        let file_name = format!("{}_tests.rs", sanitize_identifier(&category));
        let content = render_category(&category, &fixtures)?;
        let path = tests_dir.join(file_name);
        fs::write(&path, content).with_context(|| format!("Writing {}", path))?;
    }

    Ok(())
}

fn clean_rs_files(dir: &Utf8Path) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir.as_std_path())? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|ext| ext == "rs") {
            fs::remove_file(entry.path())?;
        }
    }

    Ok(())
}

fn render_category(category: &str, fixtures: &[&Fixture]) -> Result<String> {
    let mut buffer = Vec::new();
    writeln!(
        buffer,
        "// Auto-generated tests for {category} fixtures.\n#![allow(clippy::too_many_lines)]"
    )?;
    writeln!(buffer, "use e2e_rust::{{assertions, resolve_document}};")?;
    writeln!(buffer, "use kreuzberg::core::config::ExtractionConfig;")?;

    let needs_error_import = fixtures.iter().any(|fixture| {
        !fixture.skip.requires_feature.is_empty() || !fixture.document.requires_external_tool.is_empty()
    });

    if needs_error_import {
        writeln!(buffer, "use kreuzberg::KreuzbergError;\n")?;
    } else {
        writeln!(buffer)?;
    }

    for fixture in fixtures {
        buffer.write_all(render_test(fixture)?.as_bytes())?;
    }

    Ok(String::from_utf8(buffer)?)
}

fn render_test(fixture: &Fixture) -> Result<String> {
    let mut test_body = String::new();

    let test_name = format!("test_{}", sanitize_identifier(&fixture.id));
    writeln!(
        test_body,
        "#[test]\nfn {test_name}() {{\n    // {}\n",
        fixture.description
    )?;

    let doc_path = &fixture.document.path;
    writeln!(
        test_body,
        "    let document_path = resolve_document(\"{}\");",
        escape_rust_string(doc_path)
    )?;

    if fixture.skip.if_document_missing {
        writeln!(
            test_body,
            "    if !document_path.exists() {{\n        println!(\"Skipping {id}: missing document at {{}}\", document_path.display());\n        return;\n    }}",
            id = fixture.id
        )?;
    }

    let config_literal = render_config_literal(&fixture.extraction.config)?;
    if config_literal.trim().is_empty() || config_literal.trim() == "{}" {
        writeln!(test_body, "    let config = ExtractionConfig::default();\n")?;
    } else {
        writeln!(
            test_body,
            "    let config: ExtractionConfig = serde_json::from_str(r#\"{config}\"#)\n        .expect(\"Fixture config should deserialize\");\n",
            config = config_literal
        )?;
    }

    writeln!(
        test_body,
        "    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {{"
    )?;
    if !fixture.skip.requires_feature.is_empty() || !fixture.document.requires_external_tool.is_empty() {
        writeln!(
            test_body,
            "        Err(KreuzbergError::MissingDependency(dep)) => {{\n            println!(\"Skipping {id}: missing dependency {{dep}}\", dep=dep);\n            return;\n        }},",
            id = fixture.id
        )?;
        writeln!(
            test_body,
            "        Err(KreuzbergError::UnsupportedFormat(fmt)) => {{\n            println!(\"Skipping {id}: unsupported format {{fmt}} (requires optional tool)\", fmt=fmt);\n            return;\n        }},",
            id = fixture.id
        )?;
    }
    writeln!(
        test_body,
        "        Err(err) => panic!(\"Extraction failed for {id}: {{err:?}}\"),\n        Ok(result) => result,\n    }};\n",
        id = fixture.id
    )?;

    test_body.push_str(&render_assertions(&fixture.assertions));

    writeln!(test_body, "}}\n")?;

    Ok(test_body)
}

fn render_config_literal(config: &Map<String, Value>) -> Result<String> {
    if config.is_empty() {
        Ok(String::new())
    } else {
        let value = Value::Object(config.clone());
        Ok(serde_json::to_string_pretty(&value)?)
    }
}

fn render_assertions(assertions: &Assertions) -> String {
    let mut buffer = String::new();

    if !assertions.expected_mime.is_empty() {
        buffer.push_str(&format!(
            "    assertions::assert_expected_mime(&result, &{});\n",
            render_string_slice(&assertions.expected_mime)
        ));
    }

    if let Some(min) = assertions.min_content_length {
        buffer.push_str(&format!("    assertions::assert_min_content_length(&result, {min});\n"));
    }

    if let Some(max) = assertions.max_content_length {
        buffer.push_str(&format!("    assertions::assert_max_content_length(&result, {max});\n"));
    }

    if !assertions.content_contains_any.is_empty() {
        buffer.push_str(&format!(
            "    assertions::assert_content_contains_any(&result, &{});\n",
            render_string_slice(&assertions.content_contains_any)
        ));
    }

    if !assertions.content_contains_all.is_empty() {
        buffer.push_str(&format!(
            "    assertions::assert_content_contains_all(&result, &{});\n",
            render_string_slice(&assertions.content_contains_all)
        ));
    }

    if let Some(tables) = assertions.tables.as_ref() {
        let min = tables
            .min
            .map(|value| format!("Some({value})"))
            .unwrap_or_else(|| "None".into());
        let max = tables
            .max
            .map(|value| format!("Some({value})"))
            .unwrap_or_else(|| "None".into());
        buffer.push_str(&format!("    assertions::assert_table_count(&result, {min}, {max});\n",));
    }

    if let Some(languages) = assertions.detected_languages.as_ref() {
        let expected = render_string_slice(&languages.expects);
        let min_conf = languages
            .min_confidence
            .map(|v| format!("Some({v})"))
            .unwrap_or_else(|| "None".into());
        buffer.push_str(&format!(
            "    assertions::assert_detected_languages(&result, &{expected}, {min_conf});\n"
        ));
    }

    if !assertions.metadata.is_empty() {
        for (path, expectation) in &assertions.metadata {
            buffer.push_str(&format!(
                "    assertions::assert_metadata_expectation(&result, \"{}\", &{});\n",
                escape_rust_string(path),
                render_json_expression(expectation)
            ));
        }
    }

    buffer
}

fn render_json_expression(value: &serde_json::Value) -> String {
    format!("serde_json::json!({})", value)
}

fn render_string_slice(values: &[String]) -> String {
    let parts = values
        .iter()
        .map(|value| format!("\"{}\"", escape_rust_string(value)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{}]", parts)
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

fn escape_rust_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
