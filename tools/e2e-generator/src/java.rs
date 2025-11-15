use crate::fixtures::{Assertions, Fixture};
use anyhow::{Context, Result};
use camino::Utf8Path;
use itertools::Itertools;
use serde_json::{Map, Value};
use std::fmt::Write as _;
use std::fs;

const JAVA_HELPERS_TEMPLATE: &str = r#"package com.kreuzberg.e2e;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.kreuzberg.ExtractionResult;
import com.kreuzberg.Kreuzberg;
import com.kreuzberg.KreuzbergException;
import com.kreuzberg.Table;
import org.junit.jupiter.api.Assumptions;

import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assertions.fail;

/**
 * Helper utilities for E2E tests.
 */
public final class E2EHelpers {
    private static final Path WORKSPACE_ROOT =
            Paths.get("").toAbsolutePath().getParent().getParent().getParent();
    private static final Path TEST_DOCUMENTS = WORKSPACE_ROOT.resolve("test_documents");
    private static final ObjectMapper MAPPER = new ObjectMapper();

    private E2EHelpers() { }

    public static Path resolveDocument(String relativePath) {
        return TEST_DOCUMENTS.resolve(relativePath);
    }

    public static Map<String, Object> buildConfig(JsonNode configNode) {
        if (configNode == null || configNode.isNull() || !configNode.isObject()) {
            return null;
        }
        try {
            @SuppressWarnings("unchecked")
            Map<String, Object> result = MAPPER.convertValue(configNode, Map.class);
            return result;
        } catch (Exception e) {
            throw new RuntimeException("Failed to parse config", e);
        }
    }

    public static String skipReasonFor(
            Exception error,
            String fixtureId,
            List<String> requirements,
            String notes
    ) {
        String message = error.getMessage() != null ? error.getMessage() : "";
        String lowered = message.toLowerCase();
        boolean requirementHit = requirements.stream()
                .anyMatch(req -> lowered.contains(req.toLowerCase()));
        boolean missingDependency = error instanceof KreuzbergException.MissingDependency
                || lowered.contains("missing dependency");
        boolean unsupportedFormat = lowered.contains("unsupported format");

        if (!missingDependency && !unsupportedFormat && !requirementHit) {
            return null;
        }

        String reason;
        if (missingDependency) {
            if (error instanceof KreuzbergException.MissingDependency) {
                KreuzbergException.MissingDependency md = (KreuzbergException.MissingDependency) error;
                String dep = md.getDependency();
                reason = dep != null && !dep.isEmpty()
                        ? "missing dependency " + dep
                        : "missing dependency";
            } else {
                reason = "missing dependency";
            }
        } else if (unsupportedFormat) {
            reason = "unsupported format";
        } else {
            reason = "requires " + String.join(", ", requirements);
        }

        String details = String.format(
                "Skipping %s: %s. %s: %s",
                fixtureId,
                reason,
                error.getClass().getSimpleName(),
                message
        );
        if (notes != null && !notes.isEmpty()) {
            details += " Notes: " + notes;
        }
        System.err.println(details);
        return details;
    }

    public static void runFixture(
            String fixtureId,
            String relativePath,
            JsonNode configNode,
            List<String> requirements,
            String notes,
            boolean skipIfMissing,
            TestCallback callback
    ) throws Exception {
        Path documentPath = resolveDocument(relativePath);

        if (skipIfMissing && !Files.exists(documentPath)) {
            String msg = String.format("Skipping %s: missing document at %s", fixtureId, documentPath);
            System.err.println(msg);
            Assumptions.assumeTrue(false, msg);
            return;
        }

        Map<String, Object> config = buildConfig(configNode);
        ExtractionResult result;
        try {
            result = Kreuzberg.extractFile(documentPath.toString(), config);
        } catch (Exception e) {
            String skipReason = skipReasonFor(e, fixtureId, requirements, notes);
            if (skipReason != null) {
                Assumptions.assumeTrue(false, skipReason);
                return;
            }
            throw e;
        }

        callback.run(result);
    }

    @FunctionalInterface
    public interface TestCallback {
        void run(ExtractionResult result) throws Exception;
    }

    /**
     * Assertion utilities for E2E tests.
     */
    public static final class Assertions {
        private Assertions() { }

        public static void assertExpectedMime(ExtractionResult result, List<String> expected) {
            if (expected.isEmpty()) {
                return;
            }
            String mimeType = result.getMimeType();
            boolean matches = expected.stream()
                    .anyMatch(token -> mimeType != null && mimeType.contains(token));
            assertTrue(matches,
                    String.format("Expected mime type to contain one of %s, got %s", expected, mimeType));
        }

        public static void assertMinContentLength(ExtractionResult result, int minimum) {
            String content = result.getContent();
            int length = content != null ? content.length() : 0;
            assertTrue(length >= minimum,
                    String.format("Expected content length >= %d, got %d", minimum, length));
        }

        public static void assertMaxContentLength(ExtractionResult result, int maximum) {
            String content = result.getContent();
            int length = content != null ? content.length() : 0;
            assertTrue(length <= maximum,
                    String.format("Expected content length <= %d, got %d", maximum, length));
        }

        public static void assertContentContainsAny(ExtractionResult result, List<String> snippets) {
            if (snippets.isEmpty()) {
                return;
            }
            String content = result.getContent();
            String lowered = content != null ? content.toLowerCase() : "";
            boolean matches = snippets.stream()
                    .anyMatch(snippet -> lowered.contains(snippet.toLowerCase()));
            assertTrue(matches,
                    String.format("Expected content to contain any of %s", snippets));
        }

        public static void assertContentContainsAll(ExtractionResult result, List<String> snippets) {
            if (snippets.isEmpty()) {
                return;
            }
            String content = result.getContent();
            String lowered = content != null ? content.toLowerCase() : "";
            boolean allMatch = snippets.stream()
                    .allMatch(snippet -> lowered.contains(snippet.toLowerCase()));
            assertTrue(allMatch,
                    String.format("Expected content to contain all of %s", snippets));
        }

        public static void assertTableCount(
                ExtractionResult result,
                Integer minimum,
                Integer maximum
        ) {
            List<Table> tables = result.getTables();
            int count = tables != null ? tables.size() : 0;
            if (minimum != null) {
                assertTrue(count >= minimum,
                        String.format("Expected table count >= %d, got %d", minimum, count));
            }
            if (maximum != null) {
                assertTrue(count <= maximum,
                        String.format("Expected table count <= %d, got %d", maximum, count));
            }
        }

        public static void assertDetectedLanguages(
                ExtractionResult result,
                List<String> expected,
                Double minConfidence
        ) {
            if (expected.isEmpty()) {
                return;
            }
            List<String> languages = result.getDetectedLanguages();
            assertNotNull(languages, "Expected detected languages to be present");
            boolean allFound = expected.stream()
                    .allMatch(lang -> languages.contains(lang));
            assertTrue(allFound,
                    String.format("Expected languages %s to be in %s", expected, languages));

            if (minConfidence != null) {
                Map<String, Object> metadata = result.getMetadata();
                if (metadata != null && metadata.containsKey("confidence")) {
                    Object confObj = metadata.get("confidence");
                    double confidence = confObj instanceof Number
                            ? ((Number) confObj).doubleValue()
                            : 0.0;
                    assertTrue(confidence >= minConfidence,
                            String.format("Expected confidence >= %f, got %f", minConfidence, confidence));
                }
            }
        }

        public static void assertMetadataExpectation(
                ExtractionResult result,
                String path,
                Map<String, Object> expectation
        ) {
            Map<String, Object> metadata = result.getMetadata();
            Object value = fetchMetadataValue(metadata, path);
            assertNotNull(value, String.format("Metadata path '%s' missing", path));

            if (expectation.containsKey("eq")) {
                Object expected = expectation.get("eq");
                assertTrue(valuesEqual(value, expected),
                        String.format("Expected %s to equal %s", value, expected));
            }

            if (expectation.containsKey("gte")) {
                Object expected = expectation.get("gte");
                double actualNum = convertNumeric(value);
                double expectedNum = convertNumeric(expected);
                assertTrue(actualNum >= expectedNum,
                        String.format("Expected %f >= %f", actualNum, expectedNum));
            }

            if (expectation.containsKey("lte")) {
                Object expected = expectation.get("lte");
                double actualNum = convertNumeric(value);
                double expectedNum = convertNumeric(expected);
                assertTrue(actualNum <= expectedNum,
                        String.format("Expected %f <= %f", actualNum, expectedNum));
            }

            if (expectation.containsKey("contains")) {
                Object contains = expectation.get("contains");
                if (value instanceof String && contains instanceof String) {
                    assertTrue(((String) value).contains((String) contains),
                            String.format("Expected '%s' to contain '%s'", value, contains));
                } else if (value instanceof List && contains instanceof List) {
                    @SuppressWarnings("unchecked")
                    List<Object> valueList = (List<Object>) value;
                    @SuppressWarnings("unchecked")
                    List<Object> containsList = (List<Object>) contains;
                    boolean allContained = containsList.stream()
                            .allMatch(valueList::contains);
                    assertTrue(allContained,
                            String.format("Expected %s to contain all of %s", value, contains));
                } else {
                    fail(String.format("Unsupported contains expectation for path '%s'", path));
                }
            }
        }

        private static Object fetchMetadataValue(Map<String, Object> metadata, String path) {
            if (metadata == null) {
                return null;
            }
            Object current = metadata;
            for (String segment : path.split("\\.")) {
                if (!(current instanceof Map)) {
                    return null;
                }
                @SuppressWarnings("unchecked")
                Map<String, Object> map = (Map<String, Object>) current;
                current = map.get(segment);
            }
            return current;
        }

        private static boolean valuesEqual(Object lhs, Object rhs) {
            if (lhs == null && rhs == null) {
                return true;
            }
            if (lhs == null || rhs == null) {
                return false;
            }
            if (lhs instanceof String && rhs instanceof String) {
                return lhs.equals(rhs);
            }
            if (isNumericLike(lhs) && isNumericLike(rhs)) {
                return convertNumeric(lhs) == convertNumeric(rhs);
            }
            return lhs.equals(rhs);
        }

        private static boolean isNumericLike(Object value) {
            return value instanceof Number;
        }

        private static double convertNumeric(Object value) {
            if (value instanceof Number) {
                return ((Number) value).doubleValue();
            }
            throw new IllegalArgumentException("Cannot convert to numeric: " + value);
        }
    }
}
"#;

const JAVA_POM_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>

    <groupId>com.kreuzberg</groupId>
    <artifactId>kreuzberg-e2e</artifactId>
    <version>1.0-SNAPSHOT</version>

    <properties>
        <maven.compiler.source>22</maven.compiler.source>
        <maven.compiler.target>22</maven.compiler.target>
        <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
        <junit.version>5.11.3</junit.version>
        <jackson.version>2.18.2</jackson.version>
    </properties>

    <dependencies>
        <dependency>
            <groupId>com.kreuzberg</groupId>
            <artifactId>kreuzberg</artifactId>
            <version>4.0.0-rc1</version>
            <scope>system</scope>
            <systemPath>${project.basedir}/../../../packages/java/target/kreuzberg-4.0.0-rc1.jar</systemPath>
        </dependency>

        <dependency>
            <groupId>org.junit.jupiter</groupId>
            <artifactId>junit-jupiter</artifactId>
            <version>${junit.version}</version>
            <scope>test</scope>
        </dependency>

        <dependency>
            <groupId>com.fasterxml.jackson.core</groupId>
            <artifactId>jackson-databind</artifactId>
            <version>${jackson.version}</version>
        </dependency>
    </dependencies>

    <build>
        <plugins>
            <plugin>
                <groupId>org.apache.maven.plugins</groupId>
                <artifactId>maven-surefire-plugin</artifactId>
                <version>3.5.2</version>
            </plugin>
        </plugins>
    </build>
</project>
"#;

pub fn generate(fixtures: &[Fixture], output_root: &Utf8Path) -> Result<()> {
    let java_root = output_root.join("java");
    let src_test = java_root.join("src/test/java/com/kreuzberg/e2e");

    fs::create_dir_all(&src_test).context("Failed to create Java test directory")?;

    write_helpers(&src_test)?;
    write_package_info(&src_test)?;
    write_pom(&java_root)?;
    clean_test_files(&src_test)?;

    let mut grouped = fixtures
        .iter()
        .into_group_map_by(|fixture| fixture.category().to_string())
        .into_iter()
        .collect::<Vec<_>>();
    grouped.sort_by(|a, b| a.0.cmp(&b.0));

    for (category, mut fixtures) in grouped {
        fixtures.sort_by(|a, b| a.id.cmp(&b.id));
        let class_name = to_java_class_name(&category) + "Test";
        let content = render_category(&category, &class_name, &fixtures)?;
        let path = src_test.join(format!("{}.java", class_name));
        fs::write(&path, content).with_context(|| format!("Writing {}", path))?;
    }

    Ok(())
}

fn clean_test_files(src_test: &Utf8Path) -> Result<()> {
    if !src_test.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(src_test.as_std_path())? {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().is_some_and(|name| name == "E2EHelpers.java") {
            continue;
        }
        if path.extension().is_some_and(|ext| ext == "java") {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

fn write_helpers(src_test: &Utf8Path) -> Result<()> {
    let helpers_path = src_test.join("E2EHelpers.java");
    fs::write(&helpers_path, JAVA_HELPERS_TEMPLATE).context("Failed to write Java helpers")
}

fn write_package_info(src_test: &Utf8Path) -> Result<()> {
    let package_info_path = src_test.join("package-info.java");
    let content = r#"/**
 * E2E test utilities and generated test classes for Kreuzberg.
 *
 * <p>This package contains auto-generated test classes organized by fixture category.
 * Tests use JUnit 5 and validate document extraction across multiple formats.
 *
 * @since 4.0.0
 */
package com.kreuzberg.e2e;
"#;
    fs::write(&package_info_path, content).context("Failed to write package-info.java")
}

fn write_pom(java_root: &Utf8Path) -> Result<()> {
    let pom_path = java_root.join("pom.xml");
    fs::write(&pom_path, JAVA_POM_TEMPLATE).context("Failed to write pom.xml")
}

fn render_category(category: &str, class_name: &str, fixtures: &[&Fixture]) -> Result<String> {
    let mut buffer = String::new();
    writeln!(buffer, "package com.kreuzberg.e2e;")?;
    writeln!(buffer)?;
    writeln!(buffer, "// CHECKSTYLE.OFF: UnusedImports - generated code")?;
    writeln!(buffer, "// CHECKSTYLE.OFF: LineLength - generated code")?;
    writeln!(buffer, "import com.fasterxml.jackson.databind.JsonNode;")?;
    writeln!(buffer, "import com.fasterxml.jackson.databind.ObjectMapper;")?;
    writeln!(buffer, "import org.junit.jupiter.api.Test;")?;
    writeln!(buffer)?;
    writeln!(buffer, "import java.util.Arrays;")?;
    writeln!(buffer, "import java.util.Collections;")?;
    writeln!(buffer, "import java.util.List;")?;
    writeln!(buffer, "import java.util.Map;")?;
    writeln!(buffer, "// CHECKSTYLE.ON: UnusedImports")?;
    writeln!(buffer, "// CHECKSTYLE.ON: LineLength")?;
    writeln!(buffer)?;
    writeln!(buffer, "/** Auto-generated tests for {} fixtures. */", category)?;
    writeln!(buffer, "public class {} {{", class_name)?;
    writeln!(
        buffer,
        "    private static final ObjectMapper MAPPER = new ObjectMapper();"
    )?;
    writeln!(buffer)?;

    for fixture in fixtures {
        buffer.push_str(&render_test(fixture)?);
        writeln!(buffer)?;
    }

    writeln!(buffer, "}}")?;
    Ok(buffer)
}

fn render_test(fixture: &Fixture) -> Result<String> {
    let mut body = String::new();

    writeln!(body, "    @Test")?;
    writeln!(
        body,
        "    public void {}() throws Exception {{",
        to_java_method_name(&fixture.id)
    )?;

    let config_expr = render_config_expression(&fixture.extraction.config)?;
    let config_var = match config_expr {
        Some(json) => {
            writeln!(
                body,
                "        JsonNode config = MAPPER.readTree({});",
                render_java_string(&json)
            )?;
            "config"
        }
        None => {
            writeln!(body, "        JsonNode config = null;")?;
            "config"
        }
    };

    let requirements = collect_requirements(fixture);
    let requirements_expr = render_string_list(&requirements);
    let notes_expr = render_optional_string(fixture.skip.notes.as_ref());
    let skip_flag = if fixture.skip.if_document_missing {
        "true"
    } else {
        "false"
    };

    writeln!(body, "        E2EHelpers.runFixture(")?;
    writeln!(body, "            {},", render_java_string(&fixture.id))?;
    writeln!(body, "            {},", render_java_string(&fixture.document.path))?;
    writeln!(body, "            {},", config_var)?;
    writeln!(body, "            {},", requirements_expr)?;
    writeln!(body, "            {},", notes_expr)?;
    writeln!(body, "            {},", skip_flag)?;
    writeln!(body, "            result -> {{")?;

    let assertions = render_assertions(&fixture.assertions);
    if !assertions.is_empty() {
        body.push_str(&assertions);
    }

    writeln!(body, "            }}")?;
    writeln!(body, "        );")?;
    writeln!(body, "    }}")?;

    Ok(body)
}

fn render_assertions(assertions: &Assertions) -> String {
    let mut buffer = String::new();

    if !assertions.expected_mime.is_empty() {
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertExpectedMime(result, {});\n",
            render_string_list(&assertions.expected_mime)
        ));
    }

    if let Some(min) = assertions.min_content_length {
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertMinContentLength(result, {});\n",
            min
        ));
    }

    if let Some(max) = assertions.max_content_length {
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertMaxContentLength(result, {});\n",
            max
        ));
    }

    if !assertions.content_contains_any.is_empty() {
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertContentContainsAny(result, {});\n",
            render_string_list(&assertions.content_contains_any)
        ));
    }

    if !assertions.content_contains_all.is_empty() {
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertContentContainsAll(result, {});\n",
            render_string_list(&assertions.content_contains_all)
        ));
    }

    if let Some(tables) = assertions.tables.as_ref() {
        let min_literal = tables.min.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
        let max_literal = tables.max.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertTableCount(result, {}, {});\n",
            min_literal, max_literal
        ));
    }

    if let Some(languages) = assertions.detected_languages.as_ref() {
        let expected = render_string_list(&languages.expects);
        let min_conf = languages
            .min_confidence
            .map(|v| format!("{:.2}", v))
            .unwrap_or_else(|| "null".to_string());
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertDetectedLanguages(result, {}, {});\n",
            expected, min_conf
        ));
    }

    if !assertions.metadata.is_empty() {
        for (path, expectation) in &assertions.metadata {
            buffer.push_str(&format!(
                "                E2EHelpers.Assertions.assertMetadataExpectation(result, {}, {});\n",
                render_java_string(path),
                render_java_map(expectation)
            ));
        }
    }

    buffer
}

fn render_config_expression(config: &Map<String, Value>) -> Result<Option<String>> {
    if config.is_empty() {
        Ok(None)
    } else {
        let json_str = serde_json::to_string(config)?;
        Ok(Some(json_str))
    }
}

fn render_string_list(items: &[String]) -> String {
    if items.is_empty() {
        "Collections.emptyList()".to_string()
    } else {
        let content = items
            .iter()
            .map(|s| render_java_string(s))
            .collect::<Vec<_>>()
            .join(", ");
        format!("Arrays.asList({})", content)
    }
}

fn render_optional_string(value: Option<&String>) -> String {
    match value {
        Some(text) => render_java_string(text),
        None => "null".to_string(),
    }
}

fn render_java_string(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{}\"", escaped)
}

fn render_java_map(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            if map.is_empty() {
                return "Collections.emptyMap()".to_string();
            }
            let pairs = map
                .iter()
                .map(|(k, v)| format!("{}, {}", render_java_string(k), render_java_value(v)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("Map.of({})", pairs)
        }
        _ => render_java_value(value),
    }
}

fn render_java_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => render_java_string(s),
        Value::Array(arr) => {
            if arr.is_empty() {
                "Collections.emptyList()".to_string()
            } else {
                let items = arr.iter().map(render_java_value).collect::<Vec<_>>().join(", ");
                format!("Arrays.asList({})", items)
            }
        }
        Value::Object(map) => render_java_map(&Value::Object(map.clone())),
    }
}

fn to_java_class_name(input: &str) -> String {
    let mut output = String::new();
    let mut capitalize_next = true;

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            if capitalize_next {
                output.push(ch.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                output.push(ch);
            }
        } else {
            capitalize_next = true;
        }
    }

    if output.is_empty() {
        "Fixture".to_string()
    } else if output.chars().next().unwrap().is_ascii_digit() {
        format!("Test{}", output)
    } else {
        output
    }
}

fn to_java_method_name(input: &str) -> String {
    let mut output = String::new();
    let mut capitalize_next = false;

    for (idx, ch) in input.chars().enumerate() {
        if ch.is_ascii_alphanumeric() {
            if idx == 0 {
                output.push(ch.to_ascii_lowercase());
            } else if capitalize_next {
                output.push(ch.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                output.push(ch);
            }
        } else {
            capitalize_next = true;
        }
    }

    if output.is_empty() {
        "testFixture".to_string()
    } else if output.chars().next().unwrap().is_ascii_digit() {
        format!("test{}", output)
    } else {
        output
    }
}

fn collect_requirements(fixture: &Fixture) -> Vec<String> {
    fixture
        .skip
        .requires_feature
        .iter()
        .chain(fixture.document.requires_external_tool.iter())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect()
}
