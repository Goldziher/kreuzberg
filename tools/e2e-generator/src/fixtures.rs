use anyhow::{Context, Result, bail};
use camino::{Utf8Path, Utf8PathBuf};
use itertools::Itertools;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use walkdir::WalkDir;

/// Parsed fixture definition shared across generators.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Fixture {
    pub id: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub description: String,
    #[serde(default)]
    pub category: Option<String>,
    pub document: DocumentSpec,
    #[serde(default)]
    pub extraction: ExtractionSpec,
    pub assertions: Assertions,
    #[serde(default)]
    pub skip: SkipDirective,
    #[serde(skip)]
    pub source: Utf8PathBuf,
}

impl Fixture {
    pub fn category(&self) -> &str {
        self.category
            .as_deref()
            .expect("category should be resolved during load")
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
