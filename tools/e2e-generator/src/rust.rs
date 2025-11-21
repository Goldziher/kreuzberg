use crate::fixtures::{Assertions, Fixture, PluginTestSpec};
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
        let file_name = format!("{}_tests.rs", sanitize_identifier(&category));
        let content = render_category(&category, &fixtures)?;
        let path = tests_dir.join(file_name);
        fs::write(&path, content).with_context(|| format!("Writing {}", path))?;
    }

    // Generate plugin API tests
    if !api_fixtures.is_empty() {
        generate_plugin_api_tests(&api_fixtures, &tests_dir)?;
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
        !fixture.skip().requires_feature.is_empty() || !fixture.document().requires_external_tool.is_empty()
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

    let doc_path = &fixture.document().path;
    writeln!(
        test_body,
        "    let document_path = resolve_document(\"{}\");",
        escape_rust_string(doc_path)
    )?;

    if fixture.skip().if_document_missing {
        writeln!(
            test_body,
            "    if !document_path.exists() {{\n        println!(\"Skipping {id}: missing document at {{}}\", document_path.display());\n        return;\n    }}",
            id = fixture.id
        )?;
    }

    let config_literal = render_config_literal(&fixture.extraction().config)?;
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
    if !fixture.skip().requires_feature.is_empty() || !fixture.document().requires_external_tool.is_empty() {
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

    test_body.push_str(&render_assertions(&fixture.assertions()));

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

fn generate_plugin_api_tests(fixtures: &[&Fixture], output_dir: &Utf8Path) -> Result<()> {
    let test_file = output_dir.join("plugin_apis_tests.rs");

    let mut buffer = String::new();

    // File header
    writeln!(buffer, "// Auto-generated tests for plugin API fixtures.")?;
    writeln!(buffer, "#![allow(clippy::too_many_lines)]")?;
    writeln!(buffer)?;

    // Imports
    writeln!(buffer, "use kreuzberg::core::config::ExtractionConfig;")?;
    writeln!(buffer, "use kreuzberg::{{list_validators, clear_validators}};")?;
    writeln!(
        buffer,
        "use kreuzberg::{{list_post_processors, clear_post_processors}};"
    )?;
    writeln!(
        buffer,
        "use kreuzberg::{{list_ocr_backends, clear_ocr_backends, unregister_ocr_backend}};"
    )?;
    writeln!(
        buffer,
        "use kreuzberg::{{list_document_extractors, clear_document_extractors, unregister_document_extractor}};"
    )?;
    writeln!(
        buffer,
        "use kreuzberg::{{detect_mime_type, detect_mime_type_from_path, get_extensions_for_mime}};"
    )?;
    writeln!(buffer, "use std::path::Path;")?;
    writeln!(buffer)?;

    // Generate tests
    for fixture in fixtures {
        generate_plugin_test(fixture, &mut buffer)?;
    }

    fs::write(&test_file, buffer)
        .with_context(|| format!("Failed to write Rust plugin API test file {}", test_file))?;

    Ok(())
}

fn generate_plugin_test(fixture: &Fixture, buf: &mut String) -> Result<()> {
    let test_spec = fixture
        .test_spec
        .as_ref()
        .with_context(|| format!("Fixture {} missing test_spec", fixture.id))?;

    let test_name = format!("test_{}", sanitize_identifier(&fixture.id));

    writeln!(buf, "#[test]")?;
    writeln!(buf, "fn {}() {{", test_name)?;
    writeln!(buf, "    // {}", fixture.description)?;
    writeln!(buf)?;

    // Generate test body based on pattern
    match test_spec.pattern.as_str() {
        "simple_list" => generate_simple_list_test_rust(test_spec, buf)?,
        "clear_registry" => generate_clear_registry_test_rust(test_spec, buf)?,
        "graceful_unregister" => generate_graceful_unregister_test_rust(test_spec, buf)?,
        "config_from_file" => generate_config_from_file_test_rust(test_spec, buf)?,
        "config_discover" => generate_config_discover_test_rust(test_spec, buf)?,
        "mime_from_bytes" => generate_mime_from_bytes_test_rust(test_spec, buf)?,
        "mime_from_path" => generate_mime_from_path_test_rust(test_spec, buf)?,
        "mime_extension_lookup" => generate_mime_extension_lookup_test_rust(test_spec, buf)?,
        _ => anyhow::bail!("Unknown test pattern: {}", test_spec.pattern),
    }

    writeln!(buf, "}}")?;
    writeln!(buf)?;

    Ok(())
}

fn generate_simple_list_test_rust(test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let func_name = &test_spec.function_call.name;
    let assertions = &test_spec.assertions;

    // Call function
    writeln!(buf, "    let result = {}();", func_name)?;

    // Assertions
    if let Some(item_type) = &assertions.list_item_type {
        if item_type == "string" {
            writeln!(buf, "    assert!(result.iter().all(|s| !s.is_empty()));")?;
        }
    }

    if let Some(contains) = &assertions.list_contains {
        writeln!(
            buf,
            "    assert!(result.contains(&\"{}\".to_string()));",
            escape_rust_string(contains)
        )?;
    }

    Ok(())
}

fn generate_clear_registry_test_rust(test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let func_name = &test_spec.function_call.name;

    // Call clear function
    writeln!(buf, "    {}();", func_name)?;

    // Verify cleanup
    let list_func = func_name.replace("clear_", "list_");
    writeln!(buf, "    let result = {}();", list_func)?;
    writeln!(buf, "    assert!(result.is_empty());")?;

    Ok(())
}

fn generate_graceful_unregister_test_rust(test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let func_name = &test_spec.function_call.name;
    let arg = &test_spec.function_call.args[0];
    let arg_str = arg
        .as_str()
        .with_context(|| format!("Expected string argument in {}", func_name))?;

    // Should not panic
    writeln!(buf, "    {}(\"{}\");", func_name, escape_rust_string(arg_str))?;

    Ok(())
}

fn generate_config_from_file_test_rust(test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let setup = test_spec.setup.as_ref().context("config_from_file requires setup")?;
    let file_content = setup
        .temp_file_content
        .as_ref()
        .context("config_from_file requires temp_file_content")?;
    let file_name = setup
        .temp_file_name
        .as_ref()
        .context("config_from_file requires temp_file_name")?;

    // Create temp file
    writeln!(
        buf,
        "    let temp_dir = tempfile::tempdir().expect(\"Failed to create temp dir\");"
    )?;
    writeln!(
        buf,
        "    let config_path = temp_dir.path().join(\"{}\");",
        escape_rust_string(file_name)
    )?;
    writeln!(
        buf,
        "    std::fs::write(&config_path, r#\"{}\"#).expect(\"Failed to write config file\");",
        escape_rust_string(file_content)
    )?;
    writeln!(buf)?;

    // Load config
    writeln!(buf, "    let config = ExtractionConfig::from_file(&config_path)")?;
    writeln!(buf, "        .expect(\"Failed to load config from file\");")?;
    writeln!(buf)?;

    // Generate assertions
    generate_object_property_assertions_rust(&test_spec.assertions, buf)?;

    Ok(())
}

fn generate_config_discover_test_rust(test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let setup = test_spec.setup.as_ref().context("config_discover requires setup")?;
    let file_content = setup
        .temp_file_content
        .as_ref()
        .context("config_discover requires temp_file_content")?;
    let file_name = setup
        .temp_file_name
        .as_ref()
        .context("config_discover requires temp_file_name")?;
    let subdir = setup
        .subdirectory_name
        .as_ref()
        .context("config_discover requires subdirectory_name")?;

    // Create temp directory structure
    writeln!(
        buf,
        "    let temp_dir = tempfile::tempdir().expect(\"Failed to create temp dir\");"
    )?;
    writeln!(
        buf,
        "    let config_path = temp_dir.path().join(\"{}\");",
        escape_rust_string(file_name)
    )?;
    writeln!(
        buf,
        "    std::fs::write(&config_path, r#\"{}\"#).expect(\"Failed to write config file\");",
        escape_rust_string(file_content)
    )?;
    writeln!(buf)?;

    writeln!(
        buf,
        "    let subdir = temp_dir.path().join(\"{}\");",
        escape_rust_string(subdir)
    )?;
    writeln!(
        buf,
        "    std::fs::create_dir(&subdir).expect(\"Failed to create subdirectory\");"
    )?;
    writeln!(buf)?;

    // Change directory and discover
    writeln!(
        buf,
        "    let _guard = temp_cwd::TempCwd::new(&subdir).expect(\"Failed to change directory\");"
    )?;
    writeln!(buf, "    let config = ExtractionConfig::discover()")?;
    writeln!(buf, "        .expect(\"Failed to discover config\");")?;
    writeln!(buf, "    assert!(config.is_some());")?;
    writeln!(buf, "    let config = config.unwrap();")?;
    writeln!(buf)?;

    // Generate assertions
    generate_object_property_assertions_rust(&test_spec.assertions, buf)?;

    Ok(())
}

fn generate_mime_from_bytes_test_rust(test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let setup = test_spec.setup.as_ref().context("mime_from_bytes requires setup")?;
    let test_data = setup.test_data.as_ref().context("mime_from_bytes requires test_data")?;
    let assertions = &test_spec.assertions;

    // Parse hex string to bytes
    writeln!(
        buf,
        "    let data = hex::decode(\"{}\").expect(\"Failed to decode hex\");",
        test_data
    )?;

    // Call detect_mime_type
    writeln!(buf, "    let result = detect_mime_type(&data);")?;

    // Assertions
    if let Some(expected) = &assertions.string_contains {
        writeln!(
            buf,
            "    assert!(result.contains(\"{}\"));",
            escape_rust_string(expected)
        )?;
    }

    Ok(())
}

fn generate_mime_from_path_test_rust(test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let setup = test_spec.setup.as_ref().context("mime_from_path requires setup")?;
    let file_name = setup
        .temp_file_name
        .as_ref()
        .context("mime_from_path requires temp_file_name")?;
    let file_content = setup
        .temp_file_content
        .as_ref()
        .context("mime_from_path requires temp_file_content")?;
    let assertions = &test_spec.assertions;

    // Create temp file
    writeln!(
        buf,
        "    let temp_dir = tempfile::tempdir().expect(\"Failed to create temp dir\");"
    )?;
    writeln!(
        buf,
        "    let file_path = temp_dir.path().join(\"{}\");",
        escape_rust_string(file_name)
    )?;
    writeln!(
        buf,
        "    std::fs::write(&file_path, \"{}\").expect(\"Failed to write file\");",
        escape_rust_string(file_content)
    )?;
    writeln!(buf)?;

    // Call detect_mime_type_from_path
    writeln!(buf, "    let result = detect_mime_type_from_path(&file_path)")?;
    writeln!(buf, "        .expect(\"Failed to detect MIME type\");")?;

    // Assertions
    if let Some(expected) = &assertions.string_contains {
        writeln!(
            buf,
            "    assert!(result.contains(\"{}\"));",
            escape_rust_string(expected)
        )?;
    }

    Ok(())
}

fn generate_mime_extension_lookup_test_rust(test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let func_call = &test_spec.function_call;
    let arg = &func_call.args[0];
    let mime_type = arg.as_str().context("Expected string argument for MIME type")?;
    let assertions = &test_spec.assertions;

    // Call get_extensions_for_mime
    writeln!(
        buf,
        "    let result = get_extensions_for_mime(\"{}\");",
        escape_rust_string(mime_type)
    )?;

    // Assertions
    if let Some(contains) = &assertions.list_contains {
        writeln!(
            buf,
            "    assert!(result.contains(&\"{}\".to_string()));",
            escape_rust_string(contains)
        )?;
    }

    Ok(())
}

fn generate_object_property_assertions_rust(
    assertions: &crate::fixtures::PluginAssertions,
    buf: &mut String,
) -> Result<()> {
    if !assertions.object_properties.is_empty() {
        for prop in &assertions.object_properties {
            let path = &prop.path;

            if let Some(exists) = prop.exists {
                if exists {
                    writeln!(buf, "    // Verify {} exists", path)?;

                    // Simple existence check for top-level fields
                    if !path.contains('.') {
                        writeln!(buf, "    let _ = &config.{};", path)?;
                    }
                } else {
                    writeln!(buf, "    // Verify {} does not exist (not implemented)", path)?;
                }
            }

            if let Some(value) = &prop.value {
                let rust_path = path.replace('.', ".");
                match value {
                    Value::Number(n) => {
                        writeln!(buf, "    assert_eq!(config.{}, {});", rust_path, n)?;
                    }
                    Value::String(s) => {
                        writeln!(
                            buf,
                            "    assert_eq!(config.{}, \"{}\");",
                            rust_path,
                            escape_rust_string(&s)
                        )?;
                    }
                    Value::Bool(b) => {
                        writeln!(buf, "    assert_eq!(config.{}, {});", rust_path, b)?;
                    }
                    _ => {
                        writeln!(buf, "    // Complex value assertion not yet implemented for {}", path)?;
                    }
                }
            }
        }
    }

    Ok(())
}
