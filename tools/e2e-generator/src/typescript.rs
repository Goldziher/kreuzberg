use crate::fixtures::{Assertions, Fixture};
use anyhow::{Context, Result};
use camino::Utf8Path;
use itertools::Itertools;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;

const TYPESCRIPT_HELPERS_TEMPLATE: &str = r#"import { join, resolve } from "node:path";
import { expect } from "vitest";
import type {
    ChunkingConfig,
    ExtractionConfig,
    ExtractionResult,
    ImageExtractionConfig,
    LanguageDetectionConfig,
    OcrConfig,
    PdfConfig,
    PostProcessorConfig,
    TesseractConfig,
    TokenReductionConfig,
} from "../src/types.js";

const WORKSPACE_ROOT = resolve(__dirname, "../../../../..");
const TEST_DOCUMENTS = join(WORKSPACE_ROOT, "test_documents");

type PlainRecord = Record<string, unknown>;

function isPlainRecord(value: unknown): value is PlainRecord {
    return typeof value === "object" && value !== null;
}

export function resolveDocument(relative: string): string {
    return join(TEST_DOCUMENTS, relative);
}

function assignBooleanField(target: PlainRecord, source: PlainRecord, sourceKey: string, targetKey: string): void {
    if (sourceKey in source) {
        const value = source[sourceKey];
        if (typeof value === "boolean") {
            target[targetKey] = value;
        } else if (value != null) {
            target[targetKey] = Boolean(value);
        }
    }
}

function assignNumberField(target: PlainRecord, source: PlainRecord, sourceKey: string, targetKey: string): void {
    if (sourceKey in source) {
        const value = source[sourceKey];
        if (typeof value === "number") {
            target[targetKey] = value;
        } else if (typeof value === "string") {
            const parsed = Number(value);
            if (!Number.isNaN(parsed)) {
                target[targetKey] = parsed;
            }
        }
    }
}

function mapStringArray(value: unknown): string[] | undefined {
    if (!Array.isArray(value)) {
        return undefined;
    }
    return value.filter((item): item is string => typeof item === "string");
}

function mapTesseractConfig(raw: PlainRecord): TesseractConfig {
    const config: TesseractConfig = {};
    assignNumberField(config as PlainRecord, raw, "psm", "psm");
    assignBooleanField(config as PlainRecord, raw, "enable_table_detection", "enableTableDetection");
    if (typeof raw.tessedit_char_whitelist === "string") {
        config.tesseditCharWhitelist = raw.tessedit_char_whitelist as string;
    }
    return config;
}

function mapOcrConfig(raw: PlainRecord): OcrConfig | undefined {
    const backend = raw.backend;
    if (typeof backend !== "string" || backend.length === 0) {
        return undefined;
    }

    const config: OcrConfig = { backend };
    if (typeof raw.language === "string") {
        config.language = raw.language as string;
    }

    if (isPlainRecord(raw.tesseract_config)) {
        config.tesseractConfig = mapTesseractConfig(raw.tesseract_config as PlainRecord);
    }

    return config;
}

function mapChunkingConfig(raw: PlainRecord): ChunkingConfig {
    const config: ChunkingConfig = {};
    assignNumberField(config as PlainRecord, raw, "max_chars", "maxChars");
    assignNumberField(config as PlainRecord, raw, "max_overlap", "maxOverlap");
    return config;
}

function mapImageExtractionConfig(raw: PlainRecord): ImageExtractionConfig {
    const config: ImageExtractionConfig = {};
    assignBooleanField(config as PlainRecord, raw, "extract_images", "extractImages");
    assignNumberField(config as PlainRecord, raw, "target_dpi", "targetDpi");
    assignNumberField(config as PlainRecord, raw, "max_image_dimension", "maxImageDimension");
    assignBooleanField(config as PlainRecord, raw, "auto_adjust_dpi", "autoAdjustDpi");
    assignNumberField(config as PlainRecord, raw, "min_dpi", "minDpi");
    assignNumberField(config as PlainRecord, raw, "max_dpi", "maxDpi");
    return config;
}

function mapPdfConfig(raw: PlainRecord): PdfConfig {
    const config: PdfConfig = {};
    assignBooleanField(config as PlainRecord, raw, "extract_images", "extractImages");
    if (Array.isArray(raw.passwords)) {
        config.passwords = raw.passwords.filter((item: unknown): item is string => typeof item === "string");
    }
    assignBooleanField(config as PlainRecord, raw, "extract_metadata", "extractMetadata");
    return config;
}

function mapTokenReductionConfig(raw: PlainRecord): TokenReductionConfig {
    const config: TokenReductionConfig = {};
    if (typeof raw.mode === "string") {
        config.mode = raw.mode as string;
    }
    assignBooleanField(config as PlainRecord, raw, "preserve_important_words", "preserveImportantWords");
    return config;
}

function mapLanguageDetectionConfig(raw: PlainRecord): LanguageDetectionConfig {
    const config: LanguageDetectionConfig = {};
    assignBooleanField(config as PlainRecord, raw, "enabled", "enabled");
    assignNumberField(config as PlainRecord, raw, "min_confidence", "minConfidence");
    assignBooleanField(config as PlainRecord, raw, "detect_multiple", "detectMultiple");
    return config;
}

function mapPostProcessorConfig(raw: PlainRecord): PostProcessorConfig {
    const config: PostProcessorConfig = {};
    assignBooleanField(config as PlainRecord, raw, "enabled", "enabled");
    const enabled = mapStringArray(raw.enabled_processors);
    if (enabled) {
        config.enabledProcessors = enabled;
    }
    const disabled = mapStringArray(raw.disabled_processors);
    if (disabled) {
        config.disabledProcessors = disabled;
    }
    return config;
}

export function buildConfig(raw: unknown): ExtractionConfig {
    if (!isPlainRecord(raw)) {
        return {};
    }

    const source = raw as PlainRecord;
    const result: ExtractionConfig = {};
    const target = result as PlainRecord;

    assignBooleanField(target, source, "use_cache", "useCache");
    assignBooleanField(target, source, "enable_quality_processing", "enableQualityProcessing");
    assignBooleanField(target, source, "force_ocr", "forceOcr");
    assignNumberField(target, source, "max_concurrent_extractions", "maxConcurrentExtractions");

    if (isPlainRecord(source.ocr)) {
        const mapped = mapOcrConfig(source.ocr as PlainRecord);
        if (mapped) {
            result.ocr = mapped;
        }
    }

    if (isPlainRecord(source.chunking)) {
        result.chunking = mapChunkingConfig(source.chunking as PlainRecord);
    }

    if (isPlainRecord(source.images)) {
        result.images = mapImageExtractionConfig(source.images as PlainRecord);
    }

    if (isPlainRecord(source.pdf_options)) {
        result.pdfOptions = mapPdfConfig(source.pdf_options as PlainRecord);
    }

    if (isPlainRecord(source.token_reduction)) {
        result.tokenReduction = mapTokenReductionConfig(source.token_reduction as PlainRecord);
    }

    if (isPlainRecord(source.language_detection)) {
        result.languageDetection = mapLanguageDetectionConfig(source.language_detection as PlainRecord);
    }

    if (isPlainRecord(source.postprocessor)) {
        result.postprocessor = mapPostProcessorConfig(source.postprocessor as PlainRecord);
    }

    return result;
}

export function shouldSkipFixture(
    error: unknown,
    fixtureId: string,
    requirements: string[],
    notes?: string | null,
): boolean {
    if (!(error instanceof Error)) {
        return false;
    }

    const message = `${error.name}: ${error.message}`;
    const lower = message.toLowerCase();

    const requirementHit = requirements.some((req) => lower.includes(req.toLowerCase()));
    const missingDependency = lower.includes("missingdependencyerror") || lower.includes("missing dependency");
    const unsupportedFormat = lower.includes("unsupported mime type") || lower.includes("unsupported format");

    if (missingDependency || unsupportedFormat || requirementHit) {
        const reason = missingDependency
            ? "missing dependency"
            : unsupportedFormat
              ? "unsupported format"
              : requirements.join(", ");
        console.warn(`Skipping ${fixtureId}: ${reason}. ${message}`);
        if (notes) {
            console.warn(`Notes: ${notes}`);
        }
        return true;
    }

    return false;
}

export const assertions = {
    assertExpectedMime(result: ExtractionResult, expected: string[]): void {
        if (!expected.length) {
            return;
        }
        expect(expected.some((token) => result.mimeType.includes(token))).toBe(true);
    },

    assertMinContentLength(result: ExtractionResult, minimum: number): void {
        expect(result.content.length).toBeGreaterThanOrEqual(minimum);
    },

    assertMaxContentLength(result: ExtractionResult, maximum: number): void {
        expect(result.content.length).toBeLessThanOrEqual(maximum);
    },

    assertContentContainsAny(result: ExtractionResult, snippets: string[]): void {
        if (!snippets.length) {
            return;
        }
        const lowered = result.content.toLowerCase();
        expect(snippets.some((snippet) => lowered.includes(snippet.toLowerCase()))).toBe(true);
    },

    assertContentContainsAll(result: ExtractionResult, snippets: string[]): void {
        if (!snippets.length) {
            return;
        }
        const lowered = result.content.toLowerCase();
        expect(
            snippets.every((snippet) => lowered.includes(snippet.toLowerCase())),
        ).toBe(true);
    },

    assertTableCount(result: ExtractionResult, minimum?: number | null, maximum?: number | null): void {
        const tables = Array.isArray(result.tables) ? result.tables : [];
        if (typeof minimum === "number") {
            expect(tables.length).toBeGreaterThanOrEqual(minimum);
        }
        if (typeof maximum === "number") {
            expect(tables.length).toBeLessThanOrEqual(maximum);
        }
    },

    assertDetectedLanguages(result: ExtractionResult, expected: string[], minConfidence?: number | null): void {
        if (!expected.length) {
            return;
        }
        expect(result.detectedLanguages).not.toBeNull();
        const languages = result.detectedLanguages ?? [];
        expect(expected.every((lang) => languages.includes(lang))).toBe(true);

        if (typeof minConfidence === "number" && isPlainRecord(result.metadata)) {
            const confidence = (result.metadata as PlainRecord).confidence;
            if (typeof confidence === "number") {
                expect(confidence).toBeGreaterThanOrEqual(minConfidence);
            }
        }
    },

    assertMetadataExpectation(result: ExtractionResult, path: string, expectation: PlainRecord): void {
        if (!isPlainRecord(result.metadata)) {
            throw new Error(`Metadata is not a record for path ${path}`);
        }

        const value = getMetadataPath(result.metadata as PlainRecord, path);
        if (value === undefined || value === null) {
            throw new Error(`Metadata path '${path}' missing in ${JSON.stringify(result.metadata)}`);
        }

        if ("eq" in expectation) {
            expect(valuesEqual(value, expectation.eq)).toBe(true);
        }

        if ("gte" in expectation) {
            expect(Number(value)).toBeGreaterThanOrEqual(Number(expectation.gte));
        }

        if ("lte" in expectation) {
            expect(Number(value)).toBeLessThanOrEqual(Number(expectation.lte));
        }

        if ("contains" in expectation) {
            const contains = expectation.contains;
            if (typeof value === "string" && typeof contains === "string") {
                expect(value.includes(contains)).toBe(true);
            } else if (Array.isArray(value) && Array.isArray(contains)) {
                expect(contains.every((item) => value.includes(item))).toBe(true);
            } else {
                throw new Error(`Unsupported contains expectation for path '${path}'`);
            }
        }
    },
};

function getMetadataPath(metadata: PlainRecord, path: string): unknown {
    const segments = path.split(".");
    let current: unknown = metadata;
    for (const segment of segments) {
        if (!isPlainRecord(current) || !(segment in current)) {
            return undefined;
        }
        current = (current as PlainRecord)[segment];
    }
    return current;
}

function valuesEqual(lhs: unknown, rhs: unknown): boolean {
    if (typeof lhs === "string" && typeof rhs === "string") {
        return lhs === rhs;
    }
    if (typeof lhs === "number" && typeof rhs === "number") {
        return lhs === rhs;
    }
    if (typeof lhs === "boolean" && typeof rhs === "boolean") {
        return lhs === rhs;
    }
    return JSON.stringify(lhs) === JSON.stringify(rhs);
}
"#;

pub fn generate(fixtures: &[Fixture], output_root: &Utf8Path) -> Result<()> {
    let ts_impl_dir = output_root.join("typescript/tests");

    fs::create_dir_all(&ts_impl_dir).context("Failed to create TypeScript tests directory")?;

    clean_ts_files(&ts_impl_dir)?;
    write_helpers(&ts_impl_dir)?;

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
        let file_name = format!("{}.spec.ts", to_kebab_case(&category));
        let content = render_category(&category, &fixtures)?;
        let path = ts_impl_dir.join(file_name);
        fs::write(&path, content).with_context(|| format!("Writing {}", path))?;
    }

    // Generate plugin API tests
    if !plugin_fixtures.is_empty() {
        generate_plugin_api_tests(&plugin_fixtures, &ts_impl_dir)?;
    }

    Ok(())
}

fn clean_ts_files(dir: &Utf8Path) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir.as_std_path())? {
        let entry = entry?;
        let file_name = entry
            .path()
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Skip implementation files, only remove test files
        if file_name == "helpers.ts"
            || file_name == "index.ts"
            || file_name == "types.ts"
            || file_name == "errors.ts"
            || file_name == "cli.ts"
        {
            continue;
        }

        // Remove test files (.spec.ts)
        if file_name.ends_with(".spec.ts") {
            fs::remove_file(entry.path())?;
        }
    }

    Ok(())
}

fn write_helpers(src_dir: &Utf8Path) -> Result<()> {
    let helpers_path = src_dir.join("helpers.ts");
    fs::write(&helpers_path, TYPESCRIPT_HELPERS_TEMPLATE).context("Failed to write helpers.ts")
}

fn render_category(category: &str, fixtures: &[&Fixture]) -> Result<String> {
    let mut buffer = String::new();
    writeln!(buffer, "// Auto-generated tests for {category} fixtures.\n")?;
    writeln!(buffer, "import {{ existsSync }} from \"node:fs\";")?;
    writeln!(buffer, "import {{ describe, it }} from \"vitest\";")?;
    writeln!(
        buffer,
        "import {{ assertions, buildConfig, resolveDocument, shouldSkipFixture }} from \"../src/helpers.js\";"
    )?;
    writeln!(buffer, "import {{ extractFileSync }} from \"../src/index.js\";")?;
    writeln!(buffer, "import type {{ ExtractionResult }} from \"../src/types.js\";\n")?;
    writeln!(buffer, "const TEST_TIMEOUT_MS = 60_000;\n")?;

    writeln!(buffer, "describe(\"{category} fixtures\", () => {{")?;

    for fixture in fixtures {
        buffer.write_str(&render_test(fixture)?)?;
    }

    writeln!(buffer, "}});\n")?;
    Ok(buffer)
}

fn render_test(fixture: &Fixture) -> Result<String> {
    let mut body = String::new();

    let test_name = fixture.id.clone();
    writeln!(body, "  it(\"{test_name}\", () => {{")?;

    writeln!(
        body,
        "    const documentPath = resolveDocument(\"{}\");",
        escape_ts_string(&fixture.document().path)
    )?;

    if fixture.skip().if_document_missing {
        writeln!(body, "    if (!existsSync(documentPath)) {{")?;
        writeln!(
            body,
            "      console.warn(\"Skipping {}: missing document at\", documentPath);",
            escape_ts_string(&fixture.id)
        )?;
        if let Some(notes) = fixture.skip().notes.as_ref() {
            writeln!(body, "      console.warn(\"Notes: {}\");", escape_ts_string(notes))?;
        }
        writeln!(body, "      return;\n    }}")?;
    }

    match render_config_expression(&fixture.extraction().config)? {
        None => writeln!(body, "    const config = buildConfig(undefined);")?,
        Some(config_expr) => writeln!(body, "    const config = buildConfig({config_expr});")?,
    }

    let requirements = collect_requirements(fixture);
    writeln!(body, "    let result: ExtractionResult | null = null;")?;
    writeln!(body, "    try {{")?;
    writeln!(body, "      result = extractFileSync(documentPath, null, config);")?;
    writeln!(body, "    }} catch (error) {{")?;
    if !requirements.is_empty()
        || fixture.skip().notes.is_some()
        || !fixture.document().requires_external_tool.is_empty()
    {
        writeln!(
            body,
            "      if (shouldSkipFixture(error, \"{}\", {}, {})) {{",
            escape_ts_string(&fixture.id),
            render_string_array(&requirements),
            render_optional_string(fixture.skip().notes.as_ref())
        )?;
    } else {
        writeln!(
            body,
            "      if (shouldSkipFixture(error, \"{}\", [], undefined)) {{",
            escape_ts_string(&fixture.id)
        )?;
    }
    writeln!(body, "        return;\n      }}\n      throw error;\n    }}")?;
    writeln!(body, "    if (result === null) {{\n      return;\n    }}")?;

    body.push_str(&render_assertions(&fixture.assertions()));

    writeln!(body, "  }}, TEST_TIMEOUT_MS);\n")?;

    Ok(body)
}

fn render_config_expression(config: &Map<String, Value>) -> Result<Option<String>> {
    if config.is_empty() {
        Ok(None)
    } else {
        let json = serde_json::to_string(&Value::Object(config.clone()))?;
        Ok(Some(json))
    }
}

fn render_assertions(assertions: &Assertions) -> String {
    let mut buffer = String::new();

    if !assertions.expected_mime.is_empty() {
        buffer.push_str(&format!(
            "    assertions.assertExpectedMime(result, {});\n",
            render_string_array(&assertions.expected_mime)
        ));
    }

    if let Some(min) = assertions.min_content_length {
        buffer.push_str(&format!("    assertions.assertMinContentLength(result, {min});\n"));
    }

    if let Some(max) = assertions.max_content_length {
        buffer.push_str(&format!("    assertions.assertMaxContentLength(result, {max});\n"));
    }

    if !assertions.content_contains_any.is_empty() {
        buffer.push_str(&format!(
            "    assertions.assertContentContainsAny(result, {});\n",
            render_string_array(&assertions.content_contains_any)
        ));
    }

    if !assertions.content_contains_all.is_empty() {
        buffer.push_str(&format!(
            "    assertions.assertContentContainsAll(result, {});\n",
            render_string_array(&assertions.content_contains_all)
        ));
    }

    if let Some(tables) = assertions.tables.as_ref() {
        let min = tables
            .min
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".into());
        let max = tables
            .max
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".into());
        buffer.push_str(&format!("    assertions.assertTableCount(result, {min}, {max});\n"));
    }

    if let Some(languages) = assertions.detected_languages.as_ref() {
        let expected = render_string_array(&languages.expects);
        let min_conf = languages
            .min_confidence
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".into());
        buffer.push_str(&format!(
            "    assertions.assertDetectedLanguages(result, {expected}, {min_conf});\n"
        ));
    }

    if !assertions.metadata.is_empty() {
        for (path, expectation) in &assertions.metadata {
            buffer.push_str(&format!(
                "    assertions.assertMetadataExpectation(result, \"{}\", {});\n",
                escape_ts_string(path),
                render_json_literal(expectation)
            ));
        }
    }

    buffer
}

fn render_string_array(items: &[String]) -> String {
    if items.is_empty() {
        "[]".into()
    } else {
        let quoted = items
            .iter()
            .map(|item| format!("\"{}\"", escape_ts_string(item)))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{quoted}]")
    }
}

fn render_optional_string(value: Option<&String>) -> String {
    match value {
        Some(text) => format!("\"{}\"", escape_ts_string(text)),
        None => "undefined".into(),
    }
}

fn collect_requirements(fixture: &Fixture) -> Vec<String> {
    fixture
        .skip()
        .requires_feature
        .iter()
        .chain(fixture.document().requires_external_tool.iter())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn to_kebab_case(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_whitespace() || c == '_' {
                '-'
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn escape_ts_string(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn render_json_literal(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".into())
}

fn generate_plugin_api_tests(fixtures: &[&Fixture], src_dir: &Utf8Path) -> Result<()> {
    let mut buffer = String::new();

    // File header
    writeln!(buffer, "// Auto-generated from fixtures/plugin_api/ - DO NOT EDIT")?;
    writeln!(buffer, "/**")?;
    writeln!(buffer, " * E2E tests for plugin/config/utility APIs.")?;
    writeln!(buffer, " *")?;
    writeln!(buffer, " * Generated from plugin API fixtures.")?;
    writeln!(
        buffer,
        " * To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang typescript"
    )?;
    writeln!(buffer, " */")?;
    writeln!(buffer)?;
    writeln!(buffer, "import * as fs from \"node:fs\";")?;
    writeln!(buffer, "import * as os from \"node:os\";")?;
    writeln!(buffer, "import * as path from \"node:path\";")?;
    writeln!(buffer, "import {{ describe, expect, it }} from \"vitest\";")?;
    writeln!(buffer, "import * as kreuzberg from \"../src/index.js\";")?;
    writeln!(buffer)?;

    // Group fixtures by api_category
    let mut grouped = fixtures
        .iter()
        .into_group_map_by(|fixture| {
            fixture
                .api_category
                .as_ref()
                .expect("api_category required for plugin API fixtures")
                .clone()
        })
        .into_iter()
        .collect::<Vec<_>>();
    grouped.sort_by(|a, b| a.0.cmp(&b.0));

    // Generate tests grouped by category
    for (category, mut fixtures) in grouped {
        fixtures.sort_by(|a, b| a.id.cmp(&b.id));
        let category_title = to_title_case(&category);
        writeln!(buffer, "describe(\"{category_title}\", () => {{")?;

        for fixture in fixtures {
            buffer.push_str(&render_plugin_test(fixture)?);
        }

        writeln!(buffer, "}});")?;
        writeln!(buffer)?;
    }

    let path = src_dir.join("plugin-apis.spec.ts");
    fs::write(&path, buffer).with_context(|| format!("Writing {}", path))?;

    Ok(())
}

fn render_plugin_test(fixture: &Fixture) -> Result<String> {
    let mut buffer = String::new();
    let test_spec = fixture
        .test_spec
        .as_ref()
        .expect("test_spec required for plugin API fixtures");

    // Generate test name from description
    let test_name = &fixture.description;
    writeln!(buffer, "    it(\"{}\", () => {{", escape_ts_string(test_name))?;

    // Render based on pattern
    match test_spec.pattern.as_str() {
        "simple_list" => render_simple_list_test(&mut buffer, fixture)?,
        "clear_registry" => render_clear_registry_test(&mut buffer, fixture)?,
        "graceful_unregister" => render_graceful_unregister_test(&mut buffer, fixture)?,
        "config_from_file" => render_config_from_file_test(&mut buffer, fixture)?,
        "config_discover" => render_config_discover_test(&mut buffer, fixture)?,
        "mime_from_bytes" => render_mime_from_bytes_test(&mut buffer, fixture)?,
        "mime_from_path" => render_mime_from_path_test(&mut buffer, fixture)?,
        "mime_extension_lookup" => render_mime_extension_lookup_test(&mut buffer, fixture)?,
        _ => {
            return Err(anyhow::anyhow!("Unknown plugin test pattern: {}", test_spec.pattern));
        }
    }

    writeln!(buffer, "    }});")?;
    writeln!(buffer)?;

    Ok(buffer)
}

fn render_simple_list_test(buffer: &mut String, fixture: &Fixture) -> Result<()> {
    let test_spec = fixture.test_spec.as_ref().expect("test_spec required");
    let function_name = to_camel_case(&test_spec.function_call.name);

    writeln!(buffer, "        const result = kreuzberg.{function_name}();")?;
    writeln!(buffer, "        expect(Array.isArray(result)).toBe(true);")?;

    if let Some(item_type) = &test_spec.assertions.list_item_type {
        if item_type == "string" {
            writeln!(
                buffer,
                "        expect(result.every((item) => typeof item === \"string\")).toBe(true);"
            )?;
        }
    }

    if let Some(contains) = &test_spec.assertions.list_contains {
        writeln!(
            buffer,
            "        expect(result).toContain(\"{}\");",
            escape_ts_string(contains)
        )?;
    }

    Ok(())
}

fn render_clear_registry_test(buffer: &mut String, fixture: &Fixture) -> Result<()> {
    let test_spec = fixture.test_spec.as_ref().expect("test_spec required");
    let clear_fn = to_camel_case(&test_spec.function_call.name);

    // Derive list function name from clear function (e.g., clearValidators -> listValidators)
    let list_fn = clear_fn.replace("clear", "list");

    writeln!(buffer, "        kreuzberg.{clear_fn}();")?;

    if test_spec.assertions.verify_cleanup {
        writeln!(buffer, "        const result = kreuzberg.{list_fn}();")?;
        writeln!(buffer, "        expect(result).toHaveLength(0);")?;
    }

    Ok(())
}

fn render_graceful_unregister_test(buffer: &mut String, fixture: &Fixture) -> Result<()> {
    let test_spec = fixture.test_spec.as_ref().expect("test_spec required");
    let function_name = to_camel_case(&test_spec.function_call.name);

    // Get the first argument (should be the nonexistent name)
    let arg = if let Some(Value::String(s)) = test_spec.function_call.args.first() {
        s
    } else {
        "nonexistent-item"
    };

    writeln!(
        buffer,
        "        expect(() => kreuzberg.{function_name}(\"{}\")).not.toThrow();",
        escape_ts_string(arg)
    )?;

    Ok(())
}

fn render_config_from_file_test(buffer: &mut String, fixture: &Fixture) -> Result<()> {
    let test_spec = fixture.test_spec.as_ref().expect("test_spec required");
    let setup = test_spec.setup.as_ref().expect("setup required for config_from_file");

    // Create temp file
    let temp_file_name = setup.temp_file_name.as_ref().expect("temp_file_name required");
    let temp_file_content = setup.temp_file_content.as_ref().expect("temp_file_content required");

    writeln!(
        buffer,
        "        const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), \"kreuzberg-test-\"));"
    )?;
    writeln!(
        buffer,
        "        const configPath = path.join(tmpDir, \"{}\");",
        escape_ts_string(temp_file_name)
    )?;
    writeln!(
        buffer,
        "        fs.writeFileSync(configPath, \"{}\");",
        escape_ts_string(temp_file_content)
    )?;
    writeln!(buffer)?;

    // Call ExtractionConfig.fromFile
    let class_name = test_spec
        .function_call
        .class_name
        .as_ref()
        .expect("class_name required");
    let method_name = to_camel_case(&test_spec.function_call.name);

    writeln!(
        buffer,
        "        const config = kreuzberg.{}.{method_name}(configPath);",
        class_name
    )?;
    writeln!(buffer)?;

    // Assertions
    for prop in &test_spec.assertions.object_properties {
        render_object_property_assertion(buffer, "config", prop)?;
    }

    // Cleanup
    writeln!(buffer, "        fs.rmSync(tmpDir, {{ recursive: true, force: true }});")?;

    Ok(())
}

fn render_config_discover_test(buffer: &mut String, fixture: &Fixture) -> Result<()> {
    let test_spec = fixture.test_spec.as_ref().expect("test_spec required");
    let setup = test_spec.setup.as_ref().expect("setup required for config_discover");

    // Create temp directory structure
    writeln!(
        buffer,
        "        const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), \"kreuzberg-test-\"));"
    )?;

    let temp_file_name = setup.temp_file_name.as_ref().expect("temp_file_name required");
    let temp_file_content = setup.temp_file_content.as_ref().expect("temp_file_content required");

    writeln!(
        buffer,
        "        const configPath = path.join(tmpDir, \"{}\");",
        escape_ts_string(temp_file_name)
    )?;
    writeln!(
        buffer,
        "        fs.writeFileSync(configPath, \"{}\");",
        escape_ts_string(temp_file_content)
    )?;
    writeln!(buffer)?;

    // Create subdirectory if needed
    if setup.create_subdirectory {
        let subdir_name = setup.subdirectory_name.as_ref().expect("subdirectory_name required");
        writeln!(
            buffer,
            "        const subDir = path.join(tmpDir, \"{}\");",
            escape_ts_string(subdir_name)
        )?;
        writeln!(buffer, "        fs.mkdirSync(subDir);")?;
        writeln!(buffer)?;
    }

    // Change directory if needed
    if setup.change_directory {
        writeln!(buffer, "        const originalCwd = process.cwd();")?;
        writeln!(buffer, "        try {{")?;
        writeln!(buffer, "            process.chdir(subDir);")?;
        writeln!(buffer)?;
    }

    // Call ExtractionConfig.discover
    let class_name = test_spec
        .function_call
        .class_name
        .as_ref()
        .expect("class_name required");
    let method_name = to_camel_case(&test_spec.function_call.name);

    let indent = if setup.change_directory { "    " } else { "" };
    writeln!(
        buffer,
        "        {indent}const config = kreuzberg.{}.{method_name}();",
        class_name
    )?;
    writeln!(buffer, "        {indent}")?;

    // Assertions
    for prop in &test_spec.assertions.object_properties {
        render_object_property_assertion_with_indent(buffer, "config", prop, indent)?;
    }

    // Restore directory if needed
    if setup.change_directory {
        writeln!(buffer, "        }} finally {{")?;
        writeln!(buffer, "            process.chdir(originalCwd);")?;
        writeln!(buffer, "        }}")?;
    }

    // Cleanup
    writeln!(buffer, "        fs.rmSync(tmpDir, {{ recursive: true, force: true }});")?;

    Ok(())
}

fn render_mime_from_bytes_test(buffer: &mut String, fixture: &Fixture) -> Result<()> {
    let test_spec = fixture.test_spec.as_ref().expect("test_spec required");
    let setup = test_spec.setup.as_ref().expect("setup required for mime_from_bytes");

    let test_data = setup.test_data.as_ref().expect("test_data required");
    let function_name = to_camel_case(&test_spec.function_call.name);

    // Convert test data to Buffer
    writeln!(
        buffer,
        "        const testData = Buffer.from(\"{}\");",
        escape_ts_string(test_data)
    )?;
    writeln!(buffer, "        const result = kreuzberg.{function_name}(testData);")?;

    // Assertions
    if let Some(contains) = &test_spec.assertions.string_contains {
        writeln!(
            buffer,
            "        expect(result.toLowerCase()).toContain(\"{}\");",
            escape_ts_string(&contains.to_lowercase())
        )?;
    }

    Ok(())
}

fn render_mime_from_path_test(buffer: &mut String, fixture: &Fixture) -> Result<()> {
    let test_spec = fixture.test_spec.as_ref().expect("test_spec required");
    let setup = test_spec.setup.as_ref().expect("setup required for mime_from_path");

    let temp_file_name = setup.temp_file_name.as_ref().expect("temp_file_name required");
    let temp_file_content = setup.temp_file_content.as_ref().expect("temp_file_content required");
    let function_name = to_camel_case(&test_spec.function_call.name);

    // Create temp file
    writeln!(
        buffer,
        "        const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), \"kreuzberg-test-\"));"
    )?;
    writeln!(
        buffer,
        "        const filePath = path.join(tmpDir, \"{}\");",
        escape_ts_string(temp_file_name)
    )?;
    writeln!(
        buffer,
        "        fs.writeFileSync(filePath, \"{}\");",
        escape_ts_string(temp_file_content)
    )?;
    writeln!(buffer)?;

    writeln!(buffer, "        const result = kreuzberg.{function_name}(filePath);")?;

    // Assertions
    if let Some(contains) = &test_spec.assertions.string_contains {
        writeln!(
            buffer,
            "        expect(result.toLowerCase()).toContain(\"{}\");",
            escape_ts_string(&contains.to_lowercase())
        )?;
    }

    // Cleanup
    writeln!(buffer, "        fs.rmSync(tmpDir, {{ recursive: true, force: true }});")?;

    Ok(())
}

fn render_mime_extension_lookup_test(buffer: &mut String, fixture: &Fixture) -> Result<()> {
    let test_spec = fixture.test_spec.as_ref().expect("test_spec required");
    let function_name = to_camel_case(&test_spec.function_call.name);

    // Get the first argument (should be the MIME type)
    let mime_type = if let Some(Value::String(s)) = test_spec.function_call.args.first() {
        s
    } else {
        "application/pdf"
    };

    writeln!(
        buffer,
        "        const result = kreuzberg.{function_name}(\"{}\");",
        escape_ts_string(mime_type)
    )?;
    writeln!(buffer, "        expect(Array.isArray(result)).toBe(true);")?;

    if let Some(contains) = &test_spec.assertions.list_contains {
        writeln!(
            buffer,
            "        expect(result).toContain(\"{}\");",
            escape_ts_string(contains)
        )?;
    }

    Ok(())
}

fn render_object_property_assertion(
    buffer: &mut String,
    var_name: &str,
    prop: &crate::fixtures::ObjectPropertyAssertion,
) -> Result<()> {
    render_object_property_assertion_with_indent(buffer, var_name, prop, "")
}

fn render_object_property_assertion_with_indent(
    buffer: &mut String,
    var_name: &str,
    prop: &crate::fixtures::ObjectPropertyAssertion,
    indent: &str,
) -> Result<()> {
    let path_parts: Vec<&str> = prop.path.split('.').collect();
    let ts_path = path_parts
        .iter()
        .map(|p| to_camel_case(p))
        .collect::<Vec<_>>()
        .join("?.");

    // Check existence if specified
    if let Some(exists) = prop.exists {
        if exists {
            writeln!(buffer, "        {indent}expect({var_name}.{ts_path}).toBeDefined();")?;
        } else {
            writeln!(buffer, "        {indent}expect({var_name}.{ts_path}).toBeUndefined();")?;
        }
    }

    // Check value if specified
    if let Some(value) = &prop.value {
        match value {
            Value::Number(n) => {
                writeln!(buffer, "        {indent}expect({var_name}.{ts_path}).toBe({n});")?;
            }
            Value::Bool(b) => {
                writeln!(buffer, "        {indent}expect({var_name}.{ts_path}).toBe({b});")?;
            }
            Value::String(s) => {
                writeln!(
                    buffer,
                    "        {indent}expect({var_name}.{ts_path}).toBe(\"{}\");",
                    escape_ts_string(s)
                )?;
            }
            _ => {
                writeln!(
                    buffer,
                    "        {indent}expect({var_name}.{ts_path}).toEqual({});",
                    render_json_literal(value)
                )?;
            }
        }
    }

    Ok(())
}

fn to_camel_case(s: &str) -> String {
    let parts: Vec<&str> = s.split('_').collect();
    if parts.is_empty() {
        return String::new();
    }

    let mut result = parts[0].to_string();
    for part in &parts[1..] {
        if !part.is_empty() {
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                result.push_str(&first.to_uppercase().to_string());
                result.push_str(chars.as_str());
            }
        }
    }
    result
}

fn to_title_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
