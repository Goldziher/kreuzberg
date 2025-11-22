use crate::fixtures::{Assertions, Fixture};
use anyhow::{Context, Result};
use camino::Utf8Path;
use itertools::Itertools;
use serde_json::{Map, Value};
use std::fmt::Write as _;
use std::fs;

const GO_HELPERS_TEMPLATE: &str = r#"package e2e

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"unicode"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

var (
	workspaceRoot = func() string {
		wd, err := os.Getwd()
		if err != nil {
			panic(fmt.Sprintf("failed to determine working directory: %v", err))
		}
		root := filepath.Clean(filepath.Join(wd, "..", ".."))
		abs, err := filepath.Abs(root)
		if err != nil {
			panic(fmt.Sprintf("failed to resolve workspace root: %v", err))
		}
		return abs
	}()
	testDocuments = filepath.Join(workspaceRoot, "test_documents")
)

func resolveDocument(relative string) string {
	return filepath.Join(testDocuments, filepath.FromSlash(relative))
}

func ensureDocument(t *testing.T, relative string, skipIfMissing bool) string {
	t.Helper()
	path := resolveDocument(relative)
	if _, err := os.Stat(path); err != nil {
		if skipIfMissing && os.IsNotExist(err) {
			t.Skipf("Skipping %s: missing document at %s", relative, path)
		}
		t.Fatalf("document %s unavailable: %v", path, err)
	}
	return path
}

func buildConfig(t *testing.T, raw []byte) *kreuzberg.ExtractionConfig {
	t.Helper()
	if len(raw) == 0 {
		return nil
	}
	var cfg kreuzberg.ExtractionConfig
	if err := json.Unmarshal(raw, &cfg); err != nil {
		t.Fatalf("failed to decode extraction config: %v", err)
	}
	return &cfg
}

func shouldSkipMissingDependency(err error) bool {
	if err == nil {
		return false
	}
	message := strings.Map(func(r rune) rune {
		if unicode.IsSpace(r) {
			return ' '
		}
		return r
	}, strings.ToLower(err.Error()))

	if strings.Contains(message, "missing dependency") || strings.Contains(message, "libreoffice") {
		return true
	}
	return false
}

func runExtraction(t *testing.T, relativePath string, configJSON []byte) *kreuzberg.ExtractionResult {
	t.Helper()
	documentPath := ensureDocument(t, relativePath, true)
	config := buildConfig(t, configJSON)
	result, err := kreuzberg.ExtractFileSync(documentPath, config)
	if err != nil {
		if shouldSkipMissingDependency(err) {
			t.Skipf("Skipping %s: dependency unavailable (%v)", relativePath, err)
		}
		t.Fatalf("extractFileSync(%s) failed: %v", documentPath, err)
	}
	return result
}

func assertExpectedMime(t *testing.T, result *kreuzberg.ExtractionResult, expected []string) {
	t.Helper()
	if len(expected) == 0 {
		return
	}
	for _, token := range expected {
		if strings.Contains(strings.ToLower(result.MimeType), strings.ToLower(token)) {
			return
		}
	}
	t.Fatalf("expected MIME %q to include one of %v", result.MimeType, expected)
}

func assertMinContentLength(t *testing.T, result *kreuzberg.ExtractionResult, minimum int) {
	t.Helper()
	if len(result.Content) < minimum {
		t.Fatalf("expected content length >= %d, got %d", minimum, len(result.Content))
	}
}

func assertMaxContentLength(t *testing.T, result *kreuzberg.ExtractionResult, maximum int) {
	t.Helper()
	if len(result.Content) > maximum {
		t.Fatalf("expected content length <= %d, got %d", maximum, len(result.Content))
	}
}

func assertContentContainsAny(t *testing.T, result *kreuzberg.ExtractionResult, snippets []string) {
	t.Helper()
	if len(snippets) == 0 {
		return
	}
	lowered := strings.ToLower(result.Content)
	for _, snippet := range snippets {
		if strings.Contains(lowered, strings.ToLower(snippet)) {
			return
		}
	}
	t.Fatalf("expected content to contain any of %v", snippets)
}

func assertContentContainsAll(t *testing.T, result *kreuzberg.ExtractionResult, snippets []string) {
	t.Helper()
	if len(snippets) == 0 {
		return
	}
	lowered := strings.ToLower(result.Content)
	missing := make([]string, 0)
	for _, snippet := range snippets {
		if !strings.Contains(lowered, strings.ToLower(snippet)) {
			missing = append(missing, snippet)
		}
	}
	if len(missing) > 0 {
		t.Fatalf("expected content to contain all snippets %v, missing %v", snippets, missing)
	}
}

func assertTableCount(t *testing.T, result *kreuzberg.ExtractionResult, min, max *int) {
	t.Helper()
	count := len(result.Tables)
	if min != nil && count < *min {
		t.Fatalf("expected at least %d tables, found %d", *min, count)
	}
	if max != nil && count > *max {
		t.Fatalf("expected at most %d tables, found %d", *max, count)
	}
}

func assertDetectedLanguages(t *testing.T, result *kreuzberg.ExtractionResult, expected []string, minConfidence *float64) {
	t.Helper()
	if len(expected) == 0 {
		return
	}
	langs := result.DetectedLanguages
	if len(langs) == 0 {
		t.Fatalf("expected detected languages %v but field is empty", expected)
	}
	missing := make([]string, 0)
	for _, lang := range expected {
		found := false
		for _, candidate := range langs {
			if strings.EqualFold(candidate, lang) {
				found = true
				break
			}
		}
		if !found {
			missing = append(missing, lang)
		}
	}
	if len(missing) > 0 {
		t.Fatalf("expected languages %v, missing %v", expected, missing)
	}

	if minConfidence != nil {
		metadata := metadataAsMap(t, result.Metadata)
		if value, ok := lookupMetadataValue(metadata, "confidence").(float64); ok {
			if value < *minConfidence {
				t.Fatalf("expected confidence >= %f, got %f", *minConfidence, value)
			}
		}
	}
}

func assertMetadataExpectation(t *testing.T, result *kreuzberg.ExtractionResult, path string, expectation []byte) {
	t.Helper()
	if len(expectation) == 0 {
		return
	}

	metadata := metadataAsMap(t, result.Metadata)
	value := lookupMetadataValue(metadata, path)
	if value == nil {
		t.Fatalf("metadata path %q missing", path)
	}

	var spec map[string]any
	if err := json.Unmarshal(expectation, &spec); err != nil {
		t.Fatalf("failed to decode metadata expectation for %s: %v", path, err)
	}

	if expected, ok := spec["eq"]; ok {
		if !valuesEqual(value, expected) {
			t.Fatalf("expected metadata %q == %v, got %v", path, expected, value)
		}
	}
	if gte, ok := spec["gte"]; ok {
		if !compareFloat(value, gte, true) {
			t.Fatalf("expected metadata %q >= %v, got %v", path, gte, value)
		}
	}
	if lte, ok := spec["lte"]; ok {
		if !compareFloat(value, lte, false) {
			t.Fatalf("expected metadata %q <= %v, got %v", path, lte, value)
		}
	}
	if contains, ok := spec["contains"]; ok {
		if !valueContains(value, contains) {
			t.Fatalf("expected metadata %q to contain %v, got %v", path, contains, value)
		}
	}
}

func metadataAsMap(t *testing.T, metadata kreuzberg.Metadata) map[string]any {
	t.Helper()
	bytes, err := json.Marshal(metadata)
	if err != nil {
		t.Fatalf("failed to encode metadata: %v", err)
	}
	var out map[string]any
	if err := json.Unmarshal(bytes, &out); err != nil {
		t.Fatalf("failed to decode metadata map: %v", err)
	}
	return out
}

func lookupMetadataValue(metadata map[string]any, path string) any {
	current := any(metadata)
	for _, segment := range strings.Split(path, ".") {
		asMap, ok := current.(map[string]any)
		if !ok {
			return nil
		}
		value, exists := asMap[segment]
		if !exists {
			return nil
		}
		current = value
	}
	return current
}

func valuesEqual(a, b any) bool {
	switch av := a.(type) {
	case string:
		if bv, ok := b.(string); ok {
			return av == bv
		}
	case float64:
		if bv, ok := b.(float64); ok {
			return av == bv
		}
	case bool:
		if bv, ok := b.(bool); ok {
			return av == bv
		}
	case []any:
		bv, ok := b.([]any)
		if !ok || len(av) != len(bv) {
			return false
		}
		for i := range av {
			if !valuesEqual(av[i], bv[i]) {
				return false
			}
		}
		return true
	}
	return false
}

func compareFloat(actual any, expected any, gte bool) bool {
	actualFloat, ok := toFloat(actual)
	if !ok {
		return false
	}
	expectedFloat, ok := toFloat(expected)
	if !ok {
		return false
	}
	if gte {
		return actualFloat >= expectedFloat
	}
	return actualFloat <= expectedFloat
}

func toFloat(value any) (float64, bool) {
	switch v := value.(type) {
	case float64:
		return v, true
	case int:
		return float64(v), true
	case int64:
		return float64(v), true
	case json.Number:
		f, err := v.Float64()
		if err != nil {
			return 0, false
		}
		return f, true
	default:
		return 0, false
	}
}

func valueContains(value any, expectation any) bool {
	switch v := value.(type) {
	case string:
		if needle, ok := expectation.(string); ok {
			return strings.Contains(strings.ToLower(v), strings.ToLower(needle))
		}
	case []any:
		switch needle := expectation.(type) {
		case []any:
			for _, candidate := range needle {
				if !valueContains(v, candidate) {
					return false
				}
			}
			return true
		default:
			for _, item := range v {
				if valuesEqual(item, needle) {
					return true
				}
			}
		}
	}
	return false
}

func intPtr(value int) *int {
	return &value
}

func floatPtr(value float64) *float64 {
	return &value
}
"#;

pub fn generate(fixtures: &[Fixture], output_root: &Utf8Path) -> Result<()> {
    let go_root = output_root.join("go");
    fs::create_dir_all(&go_root).context("failed to create go e2e directory")?;

    write_go_mod(&go_root)?;
    clean_tests(&go_root)?;
    write_helpers(&go_root)?;

    // Separate document extraction and plugin API fixtures
    let doc_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_document_extraction()).collect();

    let plugin_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_plugin_api()).collect();

    // Generate document extraction tests
    let mut grouped = doc_fixtures
        .into_iter()
        .into_group_map_by(|fixture| fixture.category().to_string())
        .into_iter()
        .collect::<Vec<_>>();
    grouped.sort_by(|a, b| a.0.cmp(&b.0));

    for (category, mut fixtures) in grouped {
        fixtures.sort_by(|a, b| a.id.cmp(&b.id));
        let filename = format!("{}_test.go", category.to_lowercase());
        let content = render_category(&category, &fixtures)?;
        fs::write(go_root.join(&filename), content)
            .with_context(|| format!("failed to write Go test file {filename}"))?;
    }

    // Generate plugin API tests
    if !plugin_fixtures.is_empty() {
        generate_plugin_api_tests(&go_root, &plugin_fixtures)?;
    }

    Ok(())
}

fn write_go_mod(go_root: &Utf8Path) -> Result<()> {
    let go_mod = go_root.join("go.mod");
    let template = r#"module github.com/Goldziher/kreuzberg/e2e/go

go 1.25

require github.com/Goldziher/kreuzberg/packages/go v0.0.0

replace github.com/Goldziher/kreuzberg/packages/go => ../../packages/go
"#;
    fs::write(go_mod.as_std_path(), template).context("failed to write go.mod")?;
    Ok(())
}

fn clean_tests(go_root: &Utf8Path) -> Result<()> {
    if !go_root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(go_root.as_std_path())? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|ext| ext == "go") {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "helpers_test.go" || name.ends_with("_test.go") {
                fs::remove_file(entry.path())?;
            }
        }
    }
    Ok(())
}

fn write_helpers(go_root: &Utf8Path) -> Result<()> {
    let helpers_path = go_root.join("helpers_test.go");
    fs::write(helpers_path.as_std_path(), GO_HELPERS_TEMPLATE).context("failed to write helpers_test.go")?;
    Ok(())
}

fn render_category(category: &str, fixtures: &[&Fixture]) -> Result<String> {
    let mut buffer = String::new();
    writeln!(buffer, "// Code generated by kreuzberg-e2e-generator. DO NOT EDIT.")?;
    writeln!(buffer, "// Category: {category}")?;
    writeln!(buffer)?;
    writeln!(buffer, "package e2e")?;
    writeln!(buffer)?;
    writeln!(buffer, "import \"testing\"")?;
    writeln!(buffer)?;

    for fixture in fixtures {
        buffer.push_str(&render_test(fixture)?);
        buffer.push('\n');
    }

    Ok(indent_with_tabs(&buffer))
}

fn render_test(fixture: &Fixture) -> Result<String> {
    let mut code = String::new();
    let test_name = format!(
        "Test{}{}",
        to_go_pascal_case(fixture.category()),
        to_go_pascal_case(&fixture.id)
    );
    writeln!(code, "func {test_name}(t *testing.T) {{")?;
    writeln!(
        code,
        "    result := runExtraction(t, {}, {})",
        go_string_literal(&fixture.document().path),
        render_config_literal(&fixture.extraction().config)?
    )?;
    code.push_str(&render_assertions(&fixture.assertions()));
    writeln!(code, "}}")?;
    Ok(code)
}

fn render_assertions(assertions: &Assertions) -> String {
    let mut buffer = String::new();

    if !assertions.expected_mime.is_empty() {
        writeln!(
            buffer,
            "    assertExpectedMime(t, result, {})",
            render_string_slice(&assertions.expected_mime)
        )
        .unwrap();
    }
    if let Some(min) = assertions.min_content_length {
        writeln!(buffer, "    assertMinContentLength(t, result, {min})").unwrap();
    }
    if let Some(max) = assertions.max_content_length {
        writeln!(buffer, "    assertMaxContentLength(t, result, {max})").unwrap();
    }
    if !assertions.content_contains_any.is_empty() {
        writeln!(
            buffer,
            "    assertContentContainsAny(t, result, {})",
            render_string_slice(&assertions.content_contains_any)
        )
        .unwrap();
    }
    if !assertions.content_contains_all.is_empty() {
        writeln!(
            buffer,
            "    assertContentContainsAll(t, result, {})",
            render_string_slice(&assertions.content_contains_all)
        )
        .unwrap();
    }
    if let Some(tables) = assertions.tables.as_ref() {
        let min_literal = tables
            .min
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let max_literal = tables
            .max
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        writeln!(buffer, "    assertTableCount(t, result, {min_literal}, {max_literal})").unwrap();
    }
    if let Some(lang) = assertions.detected_languages.as_ref() {
        let expected = render_string_slice(&lang.expects);
        let min_conf = lang
            .min_confidence
            .map(|v| format!("floatPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        writeln!(buffer, "    assertDetectedLanguages(t, result, {expected}, {min_conf})").unwrap();
    }
    buffer
}

fn render_config_literal(config: &Map<String, Value>) -> Result<String> {
    if config.is_empty() {
        Ok("nil".to_string())
    } else {
        let json = Value::Object(config.clone());
        let literal = serde_json::to_string_pretty(&json)?;
        Ok(format!("[]byte(`{}`)", literal))
    }
}

fn render_string_slice(values: &[String]) -> String {
    if values.is_empty() {
        "nil".to_string()
    } else {
        let mut literal = String::from("[]string{");
        literal.push_str(
            &values
                .iter()
                .map(|value| go_string_literal(value))
                .collect::<Vec<_>>()
                .join(", "),
        );
        literal.push('}');
        literal
    }
}

fn go_string_literal(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Convert a snake_case or UPPER_CASE identifier to PascalCase for Go test names
fn to_go_pascal_case(value: &str) -> String {
    value
        .split('_')
        .filter(|s| !s.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
            }
        })
        .collect()
}

/// Convert space-based indentation (4 spaces per level) to tab-based indentation.
/// Go's gofmt expects tabs, not spaces.
fn indent_with_tabs(text: &str) -> String {
    text.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.is_empty() {
                String::new()
            } else {
                let indent_count = (line.len() - trimmed.len()) / 4;
                format!("{}{}", "\t".repeat(indent_count), trimmed)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Generate plugin API tests in Go
fn generate_plugin_api_tests(go_root: &Utf8Path, fixtures: &[&Fixture]) -> Result<()> {
    let mut buffer = String::new();

    // File header
    writeln!(buffer, "// Auto-generated from fixtures/plugin_api/ - DO NOT EDIT")?;
    writeln!(buffer, "//")?;
    writeln!(buffer, "// E2E tests for plugin/config/utility APIs.")?;
    writeln!(buffer, "//")?;
    writeln!(buffer, "// Generated from plugin API fixtures.")?;
    writeln!(
        buffer,
        "// To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang go"
    )?;
    writeln!(buffer)?;
    writeln!(buffer, "package e2e")?;
    writeln!(buffer)?;
    writeln!(buffer, "import (")?;
    writeln!(buffer, "    \"os\"")?;
    writeln!(buffer, "    \"path/filepath\"")?;
    writeln!(buffer, "    \"strings\"")?;
    writeln!(buffer, "    \"testing\"")?;
    writeln!(buffer)?;
    writeln!(
        buffer,
        "    kreuzberg \"github.com/Goldziher/kreuzberg/packages/go/kreuzberg\""
    )?;
    writeln!(buffer, ")")?;
    writeln!(buffer)?;

    // Group fixtures by API category
    let mut grouped: Vec<(String, Vec<&Fixture>)> = Vec::new();
    for fixture in fixtures.iter() {
        let category = fixture
            .api_category
            .as_ref()
            .with_context(|| format!("Fixture '{}' missing api_category", fixture.id))?
            .as_str()
            .to_string();

        // Find or create the category group
        if let Some(entry_pos) = grouped.iter().position(|(cat, _)| cat == &category) {
            grouped[entry_pos].1.push(fixture);
        } else {
            grouped.push((category, vec![fixture]));
        }
    }
    grouped.sort_by(|a, b| a.0.cmp(&b.0));

    // Generate tests for each category
    for (category, mut category_fixtures) in grouped {
        category_fixtures.sort_by(|a, b| a.id.cmp(&b.id));

        // Category header comment
        writeln!(buffer, "// {} Tests", to_title_case(&category))?;
        writeln!(buffer)?;

        for fixture in category_fixtures {
            buffer.push_str(&render_plugin_test(fixture)?);
            buffer.push('\n');
        }
    }

    // Write to file
    let output_path = go_root.join("plugin_apis_test.go");
    let formatted_buffer = indent_with_tabs(&buffer);
    fs::write(output_path.as_std_path(), formatted_buffer).context("failed to write plugin_apis_test.go")?;

    Ok(())
}

/// Render a single plugin API test function
fn render_plugin_test(fixture: &Fixture) -> Result<String> {
    let test_spec = fixture
        .test_spec
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing test_spec", fixture.id))?;

    let mut code = String::new();

    // Generate test function name
    let test_name = format!("Test{}", to_pascal_case(&test_spec.function_call.name));
    writeln!(code, "func {test_name}(t *testing.T) {{")?;

    // Handle different test patterns
    match test_spec.pattern.as_str() {
        "simple_list" => render_simple_list(fixture, test_spec, &mut code)?,
        "clear_registry" => render_clear_registry(fixture, test_spec, &mut code)?,
        "graceful_unregister" => render_graceful_unregister(fixture, test_spec, &mut code)?,
        "config_from_file" => render_config_from_file(fixture, test_spec, &mut code)?,
        "config_discover" => render_config_discover(fixture, test_spec, &mut code)?,
        "mime_from_bytes" => render_mime_from_bytes(fixture, test_spec, &mut code)?,
        "mime_from_path" => render_mime_from_path(fixture, test_spec, &mut code)?,
        "mime_extension_lookup" => render_mime_extension_lookup(fixture, test_spec, &mut code)?,
        _ => anyhow::bail!("Unknown test pattern: {}", test_spec.pattern),
    }

    writeln!(code, "}}")?;
    Ok(code)
}

/// Convert snake_case to PascalCase, handling acronyms like OCR
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            // Handle special acronyms that should be all caps
            match word.to_uppercase().as_str() {
                "OCR" => "OCR".to_string(),
                _ => {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                }
            }
        })
        .collect()
}

/// Convert snake_case to Title Case (with spaces)
fn to_title_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Render a simple list test
fn render_simple_list(
    _fixture: &Fixture,
    test_spec: &crate::fixtures::PluginTestSpec,
    code: &mut String,
) -> Result<()> {
    let func_name = to_pascal_case(&test_spec.function_call.name);

    // Check for lazy initialization
    if let Some(setup) = &test_spec.setup {
        if let Some(lazy_init) = &setup.lazy_init_required {
            if lazy_init.languages.contains(&"go".to_string()) {
                // Special handling for Go lazy initialization
                writeln!(code, "    tmpDir := t.TempDir()")?;
                writeln!(code, "    testFile := filepath.Join(tmpDir, \"test.pdf\")")?;
                writeln!(code, "    pdfContent := []byte(\"%PDF-1.4\\\\n%EOF\\\\n\")")?;
                writeln!(
                    code,
                    "    if err := os.WriteFile(testFile, pdfContent, 0644); err != nil {{"
                )?;
                writeln!(code, "        t.Fatalf(\"Failed to write test PDF file: %v\", err)")?;
                writeln!(code, "    }}")?;
                writeln!(code)?;
                writeln!(code, "    // This will initialize the PDF extractor")?;
                writeln!(code, "    _, _ = kreuzberg.ExtractFileSync(testFile, nil)")?;
                writeln!(code)?;
            }
        }
    }

    writeln!(code, "    result, err := kreuzberg.{}()", func_name)?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"{} failed: %v\", err)", func_name)?;
    writeln!(code, "    }}")?;
    writeln!(code, "    if result == nil {{")?;
    writeln!(code, "        t.Fatal(\"Result should not be nil\")")?;
    writeln!(code, "    }}")?;

    Ok(())
}

/// Render a clear registry test
fn render_clear_registry(
    _fixture: &Fixture,
    test_spec: &crate::fixtures::PluginTestSpec,
    code: &mut String,
) -> Result<()> {
    let clear_func = to_pascal_case(&test_spec.function_call.name);
    let list_func = clear_func.replace("Clear", "List");

    writeln!(code, "    err := kreuzberg.{}()", clear_func)?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"{} failed: %v\", err)", clear_func)?;
    writeln!(code, "    }}")?;
    writeln!(code)?;
    writeln!(code, "    result, err := kreuzberg.{}()", list_func)?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"{} failed: %v\", err)", list_func)?;
    writeln!(code, "    }}")?;
    writeln!(code, "    if len(result) != 0 {{")?;
    writeln!(
        code,
        "        t.Errorf(\"Expected empty list after clear, got %d items\", len(result))"
    )?;
    writeln!(code, "    }}")?;

    Ok(())
}

/// Render a graceful unregister test
fn render_graceful_unregister(
    _fixture: &Fixture,
    test_spec: &crate::fixtures::PluginTestSpec,
    code: &mut String,
) -> Result<()> {
    let func_name = to_pascal_case(&test_spec.function_call.name);
    let arg = test_spec
        .function_call
        .args
        .first()
        .and_then(|v| v.as_str())
        .unwrap_or("nonexistent-backend-xyz");

    writeln!(code, "    err := kreuzberg.{}(\"{}\")", func_name, arg)?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(
        code,
        "        t.Errorf(\"{} should not error for nonexistent item: %v\", err)",
        func_name
    )?;
    writeln!(code, "    }}")?;

    Ok(())
}

/// Render a config_from_file test
fn render_config_from_file(
    fixture: &Fixture,
    test_spec: &crate::fixtures::PluginTestSpec,
    code: &mut String,
) -> Result<()> {
    let setup = test_spec
        .setup
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing setup for config_from_file", fixture.id))?;

    let file_content = setup
        .temp_file_content
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing temp_file_content", fixture.id))?;

    let file_name = setup
        .temp_file_name
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing temp_file_name", fixture.id))?;

    writeln!(code, "    tmpDir := t.TempDir()")?;
    writeln!(code, "    configPath := filepath.Join(tmpDir, \"{}\")", file_name)?;
    writeln!(code)?;
    writeln!(code, "    configContent := `{}`", file_content)?;
    writeln!(
        code,
        "    if err := os.WriteFile(configPath, []byte(configContent), 0644); err != nil {{"
    )?;
    writeln!(code, "        t.Fatalf(\"Failed to write config file: %v\", err)")?;
    writeln!(code, "    }}")?;
    writeln!(code)?;

    let method_name = to_pascal_case(&test_spec.function_call.name);

    writeln!(code, "    config, err := kreuzberg.Config{}(configPath)", method_name)?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"{} failed: %v\", err)", method_name)?;
    writeln!(code, "    }}")?;
    writeln!(code)?;

    // Render property assertions
    for prop in &test_spec.assertions.object_properties {
        render_property_assertion(prop, code)?;
    }

    Ok(())
}

/// Render a config_discover test
fn render_config_discover(
    fixture: &Fixture,
    test_spec: &crate::fixtures::PluginTestSpec,
    code: &mut String,
) -> Result<()> {
    let setup = test_spec
        .setup
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing setup for config_discover", fixture.id))?;

    let file_content = setup
        .temp_file_content
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing temp_file_content", fixture.id))?;

    let file_name = setup
        .temp_file_name
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing temp_file_name", fixture.id))?;

    let subdir_name = setup
        .subdirectory_name
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing subdirectory_name", fixture.id))?;

    writeln!(code, "    tmpDir := t.TempDir()")?;
    writeln!(code, "    configPath := filepath.Join(tmpDir, \"{}\")", file_name)?;
    writeln!(code)?;
    writeln!(
        code,
        "    if err := os.WriteFile(configPath, []byte(`{}`), 0644); err != nil {{",
        file_content
    )?;
    writeln!(code, "        t.Fatalf(\"Failed to write config file: %v\", err)")?;
    writeln!(code, "    }}")?;
    writeln!(code)?;
    writeln!(code, "    subDir := filepath.Join(tmpDir, \"{}\")", subdir_name)?;
    writeln!(code, "    if err := os.MkdirAll(subDir, 0755); err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"Failed to create subdirectory: %v\", err)")?;
    writeln!(code, "    }}")?;
    writeln!(code)?;
    writeln!(code, "    originalDir, err := os.Getwd()")?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"Failed to get current directory: %v\", err)")?;
    writeln!(code, "    }}")?;
    writeln!(code, "    defer os.Chdir(originalDir)")?;
    writeln!(code)?;
    writeln!(code, "    if err := os.Chdir(subDir); err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"Failed to change directory: %v\", err)")?;
    writeln!(code, "    }}")?;
    writeln!(code)?;

    let method_name = to_pascal_case(&test_spec.function_call.name);

    writeln!(code, "    config, err := kreuzberg.Config{}()", method_name)?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"{} failed: %v\", err)", method_name)?;
    writeln!(code, "    }}")?;
    writeln!(code)?;
    writeln!(code, "    if config == nil {{")?;
    writeln!(
        code,
        "        t.Fatal(\"Config should be discovered from parent directory\")"
    )?;
    writeln!(code, "    }}")?;

    // Render property assertions
    for prop in &test_spec.assertions.object_properties {
        render_property_assertion(prop, code)?;
    }

    Ok(())
}

/// Render a mime_from_bytes test
fn render_mime_from_bytes(
    fixture: &Fixture,
    test_spec: &crate::fixtures::PluginTestSpec,
    code: &mut String,
) -> Result<()> {
    let setup = test_spec
        .setup
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing setup for mime_from_bytes", fixture.id))?;

    let test_data = setup
        .test_data
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing test_data", fixture.id))?;

    let func_name = to_pascal_case(&test_spec.function_call.name);

    writeln!(code, "    testData := []byte(\"{}\")", test_data.replace('\\', "\\\\"))?;
    writeln!(code, "    mime, err := kreuzberg.{}(testData)", func_name)?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"{} failed: %v\", err)", func_name)?;
    writeln!(code, "    }}")?;
    writeln!(code)?;

    if let Some(contains) = &test_spec.assertions.string_contains {
        writeln!(
            code,
            "    if !strings.Contains(strings.ToLower(mime), \"{}\") {{",
            contains
        )?;
        writeln!(
            code,
            "        t.Errorf(\"Expected MIME to contain '{}', got %q\", mime)",
            contains
        )?;
        writeln!(code, "    }}")?;
    }

    Ok(())
}

/// Render a mime_from_path test
fn render_mime_from_path(
    fixture: &Fixture,
    test_spec: &crate::fixtures::PluginTestSpec,
    code: &mut String,
) -> Result<()> {
    let setup = test_spec
        .setup
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing setup for mime_from_path", fixture.id))?;

    let file_name = setup
        .temp_file_name
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing temp_file_name", fixture.id))?;

    let file_content = setup
        .temp_file_content
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing temp_file_content", fixture.id))?;

    let func_name = to_pascal_case(&test_spec.function_call.name);

    writeln!(code, "    tmpDir := t.TempDir()")?;
    writeln!(code, "    testFile := filepath.Join(tmpDir, \"{}\")", file_name)?;
    writeln!(
        code,
        "    if err := os.WriteFile(testFile, []byte(\"{}\"), 0644); err != nil {{",
        file_content.replace('"', "\\\"")
    )?;
    writeln!(code, "        t.Fatalf(\"Failed to write test file: %v\", err)")?;
    writeln!(code, "    }}")?;
    writeln!(code)?;
    writeln!(code, "    mime, err := kreuzberg.{}(testFile)", func_name)?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"{} failed: %v\", err)", func_name)?;
    writeln!(code, "    }}")?;
    writeln!(code)?;

    if let Some(contains) = &test_spec.assertions.string_contains {
        writeln!(
            code,
            "    if !strings.Contains(strings.ToLower(mime), \"{}\") {{",
            contains
        )?;
        writeln!(
            code,
            "        t.Errorf(\"Expected MIME to contain '{}', got %q\", mime)",
            contains
        )?;
        writeln!(code, "    }}")?;
    }

    Ok(())
}

/// Render a mime_extension_lookup test
fn render_mime_extension_lookup(
    fixture: &Fixture,
    test_spec: &crate::fixtures::PluginTestSpec,
    code: &mut String,
) -> Result<()> {
    let func_name = to_pascal_case(&test_spec.function_call.name);
    let arg = test_spec.function_call.args.first().with_context(|| {
        format!(
            "Fixture '{}' function '{}' missing argument",
            fixture.id, test_spec.function_call.name
        )
    })?;
    let mime_type = arg
        .as_str()
        .with_context(|| format!("Fixture '{}' argument is not a string", fixture.id))?;

    writeln!(
        code,
        "    extensions, err := kreuzberg.{}(\"{}\")",
        func_name, mime_type
    )?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"{} failed: %v\", err)", func_name)?;
    writeln!(code, "    }}")?;
    writeln!(code)?;
    writeln!(code, "    if extensions == nil {{")?;
    writeln!(code, "        t.Fatal(\"Extensions list should not be nil\")")?;
    writeln!(code, "    }}")?;

    if let Some(contains) = &test_spec.assertions.list_contains {
        writeln!(code)?;
        writeln!(code, "    found := false")?;
        writeln!(code, "    for _, ext := range extensions {{")?;
        writeln!(code, "        if ext == \"{}\" {{", contains)?;
        writeln!(code, "            found = true")?;
        writeln!(code, "            break")?;
        writeln!(code, "        }}")?;
        writeln!(code, "    }}")?;
        writeln!(code, "    if !found {{")?;
        writeln!(
            code,
            "        t.Errorf(\"Expected extensions to contain '{}', got %v\", extensions)",
            contains
        )?;
        writeln!(code, "    }}")?;
    }

    Ok(())
}

/// Render property assertion for config objects
fn render_property_assertion(prop: &crate::fixtures::ObjectPropertyAssertion, code: &mut String) -> Result<()> {
    let parts: Vec<&str> = prop.path.split('.').collect();

    if parts.len() == 1 {
        // Top-level property
        if let Some(exists) = prop.exists {
            if exists {
                writeln!(code, "    if config.{} == nil {{", to_pascal_case(parts[0]))?;
                writeln!(code, "        t.Fatal(\"Config should have {} property\")", parts[0])?;
                writeln!(code, "    }}")?;
            }
        }
    } else if parts.len() == 2 {
        // Nested property
        let parent = to_pascal_case(parts[0]);
        let child = to_pascal_case(parts[1]);

        if let Some(exists) = prop.exists {
            if exists {
                writeln!(code, "    if config.{} == nil {{", parent)?;
                writeln!(code, "        t.Fatal(\"Config should have {} property\")", parts[0])?;
                writeln!(code, "    }}")?;
            }
        }

        if let Some(value) = &prop.value {
            match value {
                Value::Number(n) => {
                    writeln!(
                        code,
                        "    if config.{}.{} == nil || *config.{}.{} != {} {{",
                        parent, child, parent, child, n
                    )?;
                    writeln!(
                        code,
                        "        t.Errorf(\"Expected {}.{}={}, got %v\", *config.{}.{})",
                        parts[0], parts[1], n, parent, child
                    )?;
                    writeln!(code, "    }}")?;
                }
                Value::Bool(b) => {
                    writeln!(
                        code,
                        "    if config.{}.{} == nil || *config.{}.{} != {} {{",
                        parent, child, parent, child, b
                    )?;
                    writeln!(
                        code,
                        "        t.Errorf(\"Expected {}.{}={}, got %v\", *config.{}.{})",
                        parts[0], parts[1], b, parent, child
                    )?;
                    writeln!(code, "    }}")?;
                }
                Value::String(s) => {
                    writeln!(
                        code,
                        "    if config.{}.{} == nil || *config.{}.{} != \"{}\" {{",
                        parent, child, parent, child, s
                    )?;
                    writeln!(
                        code,
                        "        t.Errorf(\"Expected {}.{}={}, got %v\", *config.{}.{})",
                        parts[0], parts[1], s, parent, child
                    )?;
                    writeln!(code, "    }}")?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}
