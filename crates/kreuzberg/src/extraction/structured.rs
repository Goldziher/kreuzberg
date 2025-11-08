//! Structured data extraction (JSON, YAML, TOML).
//!
//! Parses structured data formats and extracts text content while preserving
//! schema information and metadata.
//!
//! # Supported Formats
//!
//! - **JSON**: Using `serde_json` with schema extraction
//! - **YAML**: Using `serde_yaml`
//! - **TOML**: Using `toml`
//!
//! # Features
//!
//! - **Text extraction**: Identifies text fields by common keywords (title, description, etc.)
//! - **Schema extraction**: Optional JSON schema generation
//! - **Depth limiting**: Prevents stack overflow on deeply nested data
//! - **Flattening**: Converts nested structures to flat text representation
//!
//! # Example
//!
//! ```rust
//! use kreuzberg::extraction::structured::parse_json;
//!
//! # fn example() -> kreuzberg::Result<()> {
//! let json = br#"{"title": "Example", "description": "Test document"}"#;
//! let result = parse_json(json, None)?;
//!
//! assert!(result.content.contains("Example"));
//! assert!(result.content.contains("Test document"));
//! # Ok(())
//! # }
//! ```
use crate::error::{KreuzbergError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredDataResult {
    pub content: String,
    pub format: String,
    pub metadata: HashMap<String, String>,
    pub text_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonExtractionConfig {
    pub extract_schema: bool,
    pub max_depth: usize,
    pub array_item_limit: usize,
    pub include_type_info: bool,
    pub flatten_nested_objects: bool,
    pub custom_text_field_patterns: Vec<String>,
}

impl Default for JsonExtractionConfig {
    fn default() -> Self {
        Self {
            extract_schema: false,
            max_depth: 10,
            array_item_limit: 100,
            include_type_info: false,
            flatten_nested_objects: true,
            custom_text_field_patterns: Vec::new(),
        }
    }
}

const TEXT_FIELD_KEYWORDS: &[&str] = &[
    "title",
    "name",
    "subject",
    "description",
    "content",
    "body",
    "text",
    "message",
];

pub fn parse_json(data: &[u8], config: Option<JsonExtractionConfig>) -> Result<StructuredDataResult> {
    let config = config.unwrap_or_default();

    let value: serde_json::Value =
        serde_json::from_slice(data).map_err(|e| KreuzbergError::parsing(format!("Failed to parse JSON: {}", e)))?;

    let mut metadata = HashMap::new();
    let mut text_fields = Vec::new();

    if config.extract_schema
        && let Ok(schema_json) = serde_json::to_string(&extract_json_schema(&value, "", 0, &config))
    {
        metadata.insert("json_schema".to_string(), schema_json);
    }

    let text_parts = extract_from_json_value(&value, "", &config, &mut metadata, &mut text_fields);
    let content = text_parts.join("\n");

    Ok(StructuredDataResult {
        content,
        format: "json".to_string(),
        metadata,
        text_fields,
    })
}

fn extract_json_schema(
    value: &serde_json::Value,
    path: &str,
    depth: usize,
    config: &JsonExtractionConfig,
) -> serde_json::Value {
    if depth >= config.max_depth {
        return serde_json::json!({"max_depth_reached": true});
    }

    match value {
        serde_json::Value::Null => serde_json::json!({"type": "null"}),
        serde_json::Value::Bool(_) => serde_json::json!({"type": "bool"}),
        serde_json::Value::Number(_) => serde_json::json!({"type": "number"}),
        serde_json::Value::String(_) => serde_json::json!({"type": "string"}),
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                serde_json::json!({"type": "array", "length": 0})
            } else if arr.len() <= config.array_item_limit {
                serde_json::json!({
                    "type": "array",
                    "length": arr.len(),
                    "items": extract_json_schema(&arr[0], &format!("{}[0]", path), depth + 1, config)
                })
            } else {
                serde_json::json!({
                    "type": "array",
                    "length": arr.len(),
                    "truncated": true,
                    "items": {"type": "truncated"}
                })
            }
        }
        serde_json::Value::Object(obj) => {
            let mut properties = serde_json::Map::new();
            for (key, val) in obj {
                let key_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };
                properties.insert(key.clone(), extract_json_schema(val, &key_path, depth + 1, config));
            }
            serde_json::json!({"type": "object", "properties": properties})
        }
    }
}

fn extract_from_json_value(
    value: &serde_json::Value,
    prefix: &str,
    config: &JsonExtractionConfig,
    metadata: &mut HashMap<String, String>,
    text_fields: &mut Vec<String>,
) -> Vec<String> {
    match value {
        serde_json::Value::Object(obj) => {
            let mut text_parts = Vec::new();
            for (key, val) in obj {
                let full_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                text_parts.extend(extract_from_json_value(val, &full_key, config, metadata, text_fields));
            }
            text_parts
        }
        serde_json::Value::Array(arr) => {
            let mut text_parts = Vec::new();
            for (i, item) in arr.iter().enumerate() {
                let item_key = if prefix.is_empty() {
                    format!("item_{}", i)
                } else {
                    format!("{}[{}]", prefix, i)
                };
                text_parts.extend(extract_from_json_value(item, &item_key, config, metadata, text_fields));
            }
            text_parts
        }
        serde_json::Value::String(s) => {
            if !s.trim().is_empty() {
                let formatted = if config.include_type_info {
                    format!("{} (string): {}", prefix, s)
                } else {
                    format!("{}: {}", prefix, s)
                };

                if is_text_field(prefix, &config.custom_text_field_patterns) {
                    metadata.insert(prefix.to_string(), s.clone());
                    text_fields.push(prefix.to_string());
                }

                vec![formatted]
            } else {
                Vec::new()
            }
        }
        serde_json::Value::Number(n) => {
            let formatted = if config.include_type_info {
                format!("{} (number): {}", prefix, n)
            } else {
                format!("{}: {}", prefix, n)
            };
            vec![formatted]
        }
        serde_json::Value::Bool(b) => {
            let formatted = if config.include_type_info {
                format!("{} (bool): {}", prefix, b)
            } else {
                format!("{}: {}", prefix, b)
            };
            vec![formatted]
        }
        serde_json::Value::Null => Vec::new(),
    }
}

fn is_text_field(key: &str, custom_patterns: &[String]) -> bool {
    let key_lower = key.to_lowercase();

    for keyword in TEXT_FIELD_KEYWORDS {
        if key_lower.contains(keyword) {
            return true;
        }
    }

    for pattern in custom_patterns {
        if key_lower.contains(&pattern.to_lowercase()) {
            return true;
        }
    }

    false
}

pub fn parse_yaml(data: &[u8]) -> Result<StructuredDataResult> {
    let yaml_str =
        std::str::from_utf8(data).map_err(|e| KreuzbergError::parsing(format!("Invalid UTF-8 in YAML: {}", e)))?;

    let value: serde_json::Value = serde_yaml_ng::from_str(yaml_str)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to parse YAML: {}", e)))?;

    let mut metadata = HashMap::new();
    let mut text_fields = Vec::new();

    let text_parts = extract_from_value(&value, "", &mut metadata, &mut text_fields);
    let content = text_parts.join("\n");

    Ok(StructuredDataResult {
        content,
        format: "yaml".to_string(),
        metadata,
        text_fields,
    })
}

fn extract_from_value(
    value: &serde_json::Value,
    prefix: &str,
    metadata: &mut HashMap<String, String>,
    text_fields: &mut Vec<String>,
) -> Vec<String> {
    match value {
        serde_json::Value::Null => Vec::new(),
        serde_json::Value::Bool(b) => vec![format!("{}: {}", prefix, b)],
        serde_json::Value::Number(n) => vec![format!("{}: {}", prefix, n)],
        serde_json::Value::String(s) => {
            if !s.trim().is_empty() {
                let formatted = format!("{}: {}", prefix, s);

                if is_text_field(prefix, &[]) {
                    metadata.insert(prefix.to_string(), s.to_string());
                    text_fields.push(prefix.to_string());
                }

                vec![formatted]
            } else {
                Vec::new()
            }
        }
        serde_json::Value::Array(arr) => {
            let mut text_parts = Vec::new();
            for (i, item) in arr.iter().enumerate() {
                let item_key = if prefix.is_empty() {
                    format!("item_{}", i)
                } else {
                    format!("{}[{}]", prefix, i)
                };
                text_parts.extend(extract_from_value(item, &item_key, metadata, text_fields));
            }
            text_parts
        }
        serde_json::Value::Object(obj) => {
            let mut text_parts = Vec::new();
            for (key, val) in obj {
                let full_key = if prefix.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", prefix, key)
                };
                text_parts.extend(extract_from_value(val, &full_key, metadata, text_fields));
            }
            text_parts
        }
    }
}

pub fn parse_toml(data: &[u8]) -> Result<StructuredDataResult> {
    let toml_str =
        std::str::from_utf8(data).map_err(|e| KreuzbergError::parsing(format!("Invalid UTF-8 in TOML: {}", e)))?;

    let value: toml::Value =
        toml::from_str(toml_str).map_err(|e| KreuzbergError::parsing(format!("Failed to parse TOML: {}", e)))?;

    let mut metadata = HashMap::new();
    let mut text_fields = Vec::new();

    let text_parts = extract_from_toml_value(&value, "", &mut metadata, &mut text_fields);
    let content = text_parts.join("\n");

    Ok(StructuredDataResult {
        content,
        format: "toml".to_string(),
        metadata,
        text_fields,
    })
}

fn extract_from_toml_value(
    value: &toml::Value,
    prefix: &str,
    metadata: &mut HashMap<String, String>,
    text_fields: &mut Vec<String>,
) -> Vec<String> {
    match value {
        toml::Value::String(s) => {
            if !s.trim().is_empty() {
                let formatted = format!("{}: {}", prefix, s);

                if is_text_field(prefix, &[]) {
                    metadata.insert(prefix.to_string(), s.clone());
                    text_fields.push(prefix.to_string());
                }

                vec![formatted]
            } else {
                Vec::new()
            }
        }
        toml::Value::Integer(i) => vec![format!("{}: {}", prefix, i)],
        toml::Value::Float(f) => vec![format!("{}: {}", prefix, f)],
        toml::Value::Boolean(b) => vec![format!("{}: {}", prefix, b)],
        toml::Value::Datetime(d) => vec![format!("{}: {}", prefix, d)],
        toml::Value::Array(arr) => {
            let mut text_parts = Vec::new();
            for (i, item) in arr.iter().enumerate() {
                let item_key = if prefix.is_empty() {
                    format!("item_{}", i)
                } else {
                    format!("{}[{}]", prefix, i)
                };
                text_parts.extend(extract_from_toml_value(item, &item_key, metadata, text_fields));
            }
            text_parts
        }
        toml::Value::Table(table) => {
            let mut text_parts = Vec::new();
            for (key, val) in table {
                let full_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                text_parts.extend(extract_from_toml_value(val, &full_key, metadata, text_fields));
            }
            text_parts
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_simple() {
        let json = r#"{"name": "John", "age": 30}"#;
        let result = parse_json(json.as_bytes(), None).unwrap();
        assert_eq!(result.format, "json");
        assert!(result.content.contains("name: John"));
        assert!(result.content.contains("age: 30"));
    }

    #[test]
    fn test_parse_json_nested() {
        let json = r#"{"user": {"name": "Alice", "email": "alice@example.com"}}"#;
        let result = parse_json(json.as_bytes(), None).unwrap();
        assert!(result.content.contains("user.name: Alice"));
        assert!(result.content.contains("user.email: alice@example.com"));
    }

    #[test]
    fn test_parse_json_array() {
        let json = r#"{"items": ["apple", "banana", "cherry"]}"#;
        let result = parse_json(json.as_bytes(), None).unwrap();
        assert!(result.content.contains("items[0]: apple"));
        assert!(result.content.contains("items[1]: banana"));
        assert!(result.content.contains("items[2]: cherry"));
    }

    #[test]
    fn test_parse_json_with_schema() {
        let json = r#"{"name": "Test", "count": 42}"#;
        let config = JsonExtractionConfig {
            extract_schema: true,
            ..Default::default()
        };
        let result = parse_json(json.as_bytes(), Some(config)).unwrap();
        assert!(result.metadata.contains_key("json_schema"));
    }

    #[test]
    fn test_parse_yaml_simple() {
        let yaml = "name: John\nage: 30";
        let result = parse_yaml(yaml.as_bytes()).unwrap();
        assert_eq!(result.format, "yaml");
        assert!(result.content.contains("name: John"));
        assert!(result.content.contains("age: 30"));
    }

    #[test]
    fn test_parse_yaml_nested() {
        let yaml = "user:\n  name: Alice\n  email: alice@example.com";
        let result = parse_yaml(yaml.as_bytes()).unwrap();
        assert!(result.content.contains("user.name: Alice"));
        assert!(result.content.contains("user.email: alice@example.com"));
    }

    #[test]
    fn test_parse_toml_simple() {
        let toml = "name = \"John\"\nage = 30";
        let result = parse_toml(toml.as_bytes()).unwrap();
        assert_eq!(result.format, "toml");
        assert!(result.content.contains("name: John"));
        assert!(result.content.contains("age: 30"));
    }

    #[test]
    fn test_parse_toml_table() {
        let toml = "[user]\nname = \"Alice\"\nemail = \"alice@example.com\"";
        let result = parse_toml(toml.as_bytes()).unwrap();
        assert!(result.content.contains("user.name: Alice"));
        assert!(result.content.contains("user.email: alice@example.com"));
    }

    #[test]
    fn test_text_field_detection() {
        assert!(is_text_field("title", &[]));
        assert!(is_text_field("user.name", &[]));
        assert!(is_text_field("description", &[]));
        assert!(!is_text_field("id", &[]));
        assert!(!is_text_field("count", &[]));
    }

    #[test]
    fn test_parse_json_invalid() {
        let json = "not valid json {";
        let result = parse_json(json.as_bytes(), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_yaml_invalid() {
        let yaml = "invalid: [unclosed";
        let result = parse_yaml(yaml.as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_toml_invalid() {
        let toml = "invalid = [unclosed";
        let result = parse_toml(toml.as_bytes());
        assert!(result.is_err());
    }
}
