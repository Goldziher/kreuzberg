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

    let mut grouped = fixtures
        .iter()
        .into_group_map_by(|fixture| fixture.category().to_string())
        .into_iter()
        .collect::<Vec<_>>();
    grouped.sort_by(|a, b| a.0.cmp(&b.0));

    for (category, mut fixtures) in grouped {
        fixtures.sort_by(|a, b| a.id.cmp(&b.id));
        let filename = format!("{}_test.go", sanitize_identifier(&category));
        let content = render_category(&category, &fixtures)?;
        fs::write(go_root.join(&filename), content)
            .with_context(|| format!("failed to write Go test file {filename}"))?;
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

    Ok(buffer)
}

fn render_test(fixture: &Fixture) -> Result<String> {
    let mut code = String::new();
    let test_name = format!(
        "Test{}_{}",
        sanitize_identifier(fixture.category()),
        sanitize_identifier(&fixture.id)
    );
    writeln!(code, "func {test_name}(t *testing.T) {{")?;
    writeln!(
        code,
        "    result := runExtraction(t, {}, {})",
        go_string_literal(&fixture.document.path),
        render_config_literal(&fixture.extraction.config)?
    )?;
    code.push_str(&render_assertions(&fixture.assertions));
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

fn sanitize_identifier(value: &str) -> String {
    let mut ident = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            ident.push(ch.to_ascii_uppercase());
        } else {
            ident.push('_');
        }
    }
    while ident.starts_with('_') {
        ident.remove(0);
    }
    if ident.is_empty() { "Fixture".to_string() } else { ident }
}
