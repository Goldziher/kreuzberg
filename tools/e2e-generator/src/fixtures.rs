use anyhow::{Context, Result, bail};
use camino::{Utf8Path, Utf8PathBuf};
use itertools::Itertools;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use walkdir::WalkDir;

/// Parsed fixture definition shared across generators.
/// Supports both document extraction and plugin API fixtures.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Fixture {
    pub id: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub description: String,
    #[serde(default)]
    pub category: Option<String>,

    #[serde(default)]
    pub document: Option<DocumentSpec>,
    #[serde(default)]
    pub extraction: Option<ExtractionSpec>,
    #[serde(default)]
    pub assertions: Option<Assertions>,
    #[serde(default)]
    pub skip: Option<SkipDirective>,

    #[serde(default)]
    pub api_category: Option<String>,
    #[serde(default)]
    pub api_function: Option<String>,
    #[serde(default)]
    pub test_spec: Option<PluginTestSpec>,
    #[serde(default)]
    pub plugin_skip: Option<PluginSkipDirective>,

    #[serde(skip)]
    pub source: Utf8PathBuf,
}

impl Fixture {
    pub fn category(&self) -> &str {
        self.category
            .as_deref()
            .expect("category should be resolved during load")
    }

    /// Returns true if this is a plugin API fixture
    pub fn is_plugin_api(&self) -> bool {
        self.api_category.is_some()
    }

    /// Returns true if this is a document extraction fixture
    pub fn is_document_extraction(&self) -> bool {
        self.document.is_some()
    }

    /// Get document spec for document extraction fixtures.
    /// Panics if called on a plugin API fixture.
    pub fn document(&self) -> &DocumentSpec {
        self.document
            .as_ref()
            .expect("document field required for document extraction fixtures")
    }

    /// Get extraction spec for document extraction fixtures.
    /// Returns a default if not specified. Panics if called on a plugin API fixture.
    pub fn extraction(&self) -> ExtractionSpec {
        self.extraction.clone().unwrap_or_default()
    }

    /// Get assertions for document extraction fixtures.
    /// Returns a default if not specified. Panics if called on a plugin API fixture.
    pub fn assertions(&self) -> Assertions {
        self.assertions.clone().unwrap_or_default()
    }

    /// Get skip directive for document extraction fixtures.
    /// Returns a default if not specified. Panics if called on a plugin API fixture.
    pub fn skip(&self) -> SkipDirective {
        self.skip.clone().unwrap_or_default()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentSpec {
    pub path: String,
    #[serde(default)]
    pub media_type: Option<String>,
    #[serde(default)]
    pub requires_external_tool: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ExtractionSpec {
    #[serde(default)]
    pub config: Map<String, Value>,
    #[serde(default)]
    pub force_async: bool,
    #[serde(default)]
    pub chunking: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Assertions {
    #[serde(default, deserialize_with = "deserialize_expected_mime")]
    pub expected_mime: Vec<String>,
    #[serde(default)]
    pub min_content_length: Option<usize>,
    #[serde(default)]
    pub max_content_length: Option<usize>,
    #[serde(default)]
    pub content_contains_any: Vec<String>,
    #[serde(default)]
    pub content_contains_all: Vec<String>,
    #[serde(default)]
    pub tables: Option<TableAssertion>,
    #[serde(default)]
    pub detected_languages: Option<DetectedLanguageAssertion>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TableAssertion {
    #[serde(default)]
    pub min: Option<usize>,
    #[serde(default)]
    pub max: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DetectedLanguageAssertion {
    pub expects: Vec<String>,
    #[serde(default)]
    pub min_confidence: Option<f32>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct SkipDirective {
    #[serde(default = "default_true")]
    pub if_document_missing: bool,
    #[serde(default)]
    pub requires_feature: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for SkipDirective {
    fn default() -> Self {
        Self {
            if_document_missing: true,
            requires_feature: Vec::new(),
            notes: None,
        }
    }
}

fn deserialize_expected_mime<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    let mut output = Vec::new();
    match value {
        Value::Null => {}
        Value::String(s) => output.push(s),
        Value::Array(items) => {
            for item in items {
                match item {
                    Value::String(s) => output.push(s),
                    other => {
                        return Err(serde::de::Error::custom(format!(
                            "expected string in expected_mime array, got {other}"
                        )));
                    }
                }
            }
        }
        other => {
            return Err(serde::de::Error::custom(format!(
                "expected string or array for expected_mime, got {other}"
            )));
        }
    }
    Ok(output)
}

/// Test specification for plugin API fixtures
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct PluginTestSpec {
    /// Test pattern identifier (e.g., "simple_list", "clear_registry")
    pub pattern: String,
    /// Optional setup steps before test execution
    #[serde(default)]
    pub setup: Option<PluginSetup>,
    /// Function call specification
    pub function_call: PluginFunctionCall,
    /// Assertions to verify
    pub assertions: PluginAssertions,
    /// Optional teardown steps after test execution
    #[serde(default)]
    pub teardown: Option<PluginTeardown>,
}

/// Setup configuration for plugin API tests
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PluginSetup {
    /// Whether to create a temporary file
    #[serde(default)]
    pub create_temp_file: bool,
    /// Name of temporary file to create
    #[serde(default)]
    pub temp_file_name: Option<String>,
    /// Content to write to temporary file
    #[serde(default)]
    pub temp_file_content: Option<String>,
    /// Whether to create a temporary directory
    #[serde(default)]
    pub create_temp_dir: bool,
    /// Whether to create a subdirectory in temp dir
    #[serde(default)]
    pub create_subdirectory: bool,
    /// Name of subdirectory to create
    #[serde(default)]
    pub subdirectory_name: Option<String>,
    /// Whether to change to subdirectory for test
    #[serde(default)]
    pub change_directory: bool,
    /// Test data (e.g., bytes for MIME detection)
    #[serde(default)]
    pub test_data: Option<String>,
    /// Special initialization required (e.g., for Go document extractors)
    #[serde(default)]
    pub lazy_init_required: Option<LazyInitSpec>,
}

/// Lazy initialization specification
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct LazyInitSpec {
    /// Languages requiring initialization
    pub languages: Vec<String>,
    /// Action to perform for initialization
    pub init_action: String,
    /// Data needed for initialization
    #[serde(default)]
    pub init_data: Option<Value>,
}

/// Function call specification
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct PluginFunctionCall {
    /// Function name (snake_case, will be converted per language)
    pub name: String,
    /// Arguments to pass (use ${var} for substitutions)
    #[serde(default)]
    pub args: Vec<Value>,
    /// Whether this is a class/static method
    #[serde(default)]
    pub is_method: bool,
    /// Class name if is_method is true
    #[serde(default)]
    pub class_name: Option<String>,
}

/// Assertions for plugin API tests
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PluginAssertions {
    /// Expected return type
    #[serde(default)]
    pub return_type: Option<String>,
    /// If return_type is list, the type of items
    #[serde(default)]
    pub list_item_type: Option<String>,
    /// Item that must be in returned list
    #[serde(default)]
    pub list_contains: Option<String>,
    /// Whether list should be empty
    #[serde(default)]
    pub list_empty: bool,
    /// Substring that must be in returned string
    #[serde(default)]
    pub string_contains: Option<String>,
    /// Assert that function does not throw/error
    #[serde(default)]
    pub does_not_throw: bool,
    /// Object properties to verify
    #[serde(default)]
    pub object_properties: Vec<ObjectPropertyAssertion>,
    /// Verify list is empty after clear operation
    #[serde(default)]
    pub verify_cleanup: bool,
}

/// Assertion for object property
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ObjectPropertyAssertion {
    /// Property path (dot notation, e.g., 'chunking.max_chars')
    pub path: String,
    /// Expected value
    #[serde(default)]
    pub value: Option<Value>,
    /// Whether property should exist (true) or not exist (false)
    #[serde(default)]
    pub exists: Option<bool>,
}

/// Teardown configuration for plugin API tests
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PluginTeardown {
    /// Whether to restore original directory
    #[serde(default)]
    pub restore_directory: bool,
}

/// Skip directive for plugin API fixtures
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PluginSkipDirective {
    /// Languages to skip this test for
    #[serde(default)]
    pub languages: Vec<String>,
    /// Reason for skipping
    #[serde(default)]
    pub reason: Option<String>,
}

/// Load fixtures from directory.
pub fn load_fixtures(fixtures_dir: &Utf8Path) -> Result<Vec<Fixture>> {
    let mut fixtures = Vec::new();

    for entry in WalkDir::new(fixtures_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = Utf8PathBuf::from_path_buf(entry.into_path())
            .map_err(|_| anyhow::anyhow!("Fixture path is not valid UTF-8"))?;

        if path
            .file_name()
            .is_some_and(|name| name == "schema.json" || name.starts_with('_'))
        {
            continue;
        }

        if path.extension() != Some("json") {
            continue;
        }

        let contents = std::fs::read_to_string(&path).with_context(|| format!("Failed to read fixture {}", path))?;
        let mut fixture: Fixture = serde_json::from_str(&contents).with_context(|| format!("Parsing {path}"))?;

        if !fixture.is_document_extraction() && !fixture.is_plugin_api() {
            bail!(
                "Fixture {} must have either 'document' (document extraction) or 'api_category' (plugin API) field",
                path
            );
        }

        if fixture.is_document_extraction() && fixture.is_plugin_api() {
            bail!("Fixture {} cannot have both 'document' and 'api_category' fields", path);
        }

        if fixture.category.is_none() {
            let category = path.parent().and_then(Utf8Path::file_name).map(|name| name.to_string());
            fixture.category = category;
        }

        if fixture.category.is_none() {
            bail!("Fixture {path} missing category");
        }

        fixture.source = path;
        fixtures.push(fixture);
    }

    fixtures.sort_by_key(|fixture| (fixture.category.clone(), fixture.id.clone()));
    let duplicates = fixtures
        .iter()
        .tuple_windows()
        .filter(|(a, b)| a.id == b.id)
        .map(|(a, _)| a.id.clone())
        .collect::<Vec<_>>();

    if !duplicates.is_empty() {
        bail!("Duplicate fixture ids found: {:?}", duplicates);
    }

    Ok(fixtures)
}
