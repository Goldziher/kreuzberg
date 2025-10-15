use crate::error::{KreuzbergError, Result};
use crate::text::normalize_spaces;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tokio::process::Command;

/// Extract content from file using Pandoc (convert to markdown)
pub async fn extract_content(path: &Path, from_format: &str) -> Result<String> {
    // Create temporary output file
    let temp_dir = std::env::temp_dir();
    let output_path = temp_dir.join(format!(
        "pandoc_output_{}_{}.md",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));

    // Build pandoc command
    let mut cmd = Command::new("pandoc");
    cmd.arg(path)
        .arg(format!("--from={}", from_format))
        .arg("--to=markdown")
        .arg("--standalone")
        .arg("--wrap=preserve")
        .arg("--quiet")
        .arg("--output")
        .arg(&output_path);

    // Execute
    let output = cmd.output().await.map_err(|e| {
        // Failed to execute pandoc - this is an IO error (command not found, etc.) ~keep
        std::io::Error::other(format!("Failed to execute pandoc: {}", e))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = fs::remove_file(&output_path).await;

        // Subprocess error analysis - wrap only if format/parsing error detected ~keep
        let stderr_lower = stderr.to_lowercase();
        if stderr_lower.contains("format")
            || stderr_lower.contains("unsupported")
            || stderr_lower.contains("error:")
            || stderr_lower.contains("failed")
        {
            return Err(KreuzbergError::Parsing(format!(
                "Pandoc format/parsing error: {}",
                stderr
            )));
        }

        // True system error - bubble up as IO error ~keep
        return Err(std::io::Error::other(format!("Pandoc system error: {}", stderr)).into());
    }

    // Read output
    let content = fs::read_to_string(&output_path)
        .await
        .map_err(|e| KreuzbergError::Parsing(format!("Failed to read pandoc output: {}", e)))?;

    // Cleanup
    let _ = fs::remove_file(&output_path).await;

    Ok(normalize_spaces(&content))
}

/// Extract metadata from file using Pandoc JSON output
pub async fn extract_metadata(path: &Path, from_format: &str) -> Result<HashMap<String, Value>> {
    // Create temporary output file
    let temp_dir = std::env::temp_dir();
    let metadata_path = temp_dir.join(format!(
        "pandoc_meta_{}_{}.json",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));

    // Build pandoc command
    let mut cmd = Command::new("pandoc");
    cmd.arg(path)
        .arg(format!("--from={}", from_format))
        .arg("--to=json")
        .arg("--standalone")
        .arg("--quiet")
        .arg("--output")
        .arg(&metadata_path);

    // Execute
    let output = cmd.output().await.map_err(|e| {
        // Failed to execute pandoc - this is an IO error (command not found, etc.) ~keep
        std::io::Error::other(format!("Failed to execute pandoc: {}", e))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = fs::remove_file(&metadata_path).await;

        // Subprocess error analysis - wrap only if format/parsing error detected ~keep
        let stderr_lower = stderr.to_lowercase();
        if stderr_lower.contains("format")
            || stderr_lower.contains("unsupported")
            || stderr_lower.contains("error:")
            || stderr_lower.contains("failed")
        {
            return Err(KreuzbergError::Parsing(format!(
                "Pandoc metadata extraction format/parsing error: {}",
                stderr
            )));
        }

        // True system error - bubble up as IO error ~keep
        return Err(std::io::Error::other(format!("Pandoc metadata extraction system error: {}", stderr)).into());
    }

    // Read JSON
    let json_content = fs::read_to_string(&metadata_path)
        .await
        .map_err(|e| KreuzbergError::Parsing(format!("Failed to read pandoc JSON output: {}", e)))?;

    // Cleanup
    let _ = fs::remove_file(&metadata_path).await;

    // Parse JSON
    let json_data: Value = serde_json::from_str(&json_content)
        .map_err(|e| KreuzbergError::Parsing(format!("Failed to parse pandoc JSON: {}", e)))?;

    // Extract metadata
    extract_metadata_from_json(&json_data)
}

/// Valid metadata field names (must match Python's _VALID_METADATA_KEYS)
const VALID_METADATA_KEYS: &[&str] = &[
    "abstract",
    "authors",
    "categories",
    "character_count",
    "citations",
    "code_blocks",
    "comments",
    "content",
    "copyright",
    "created_at",
    "created_by",
    "description",
    "fonts",
    "headers",
    "height",
    "identifier",
    "keywords",
    "languages",
    "license",
    "line_count",
    "links",
    "modified_at",
    "modified_by",
    "organization",
    "parse_error",
    "publisher",
    "references",
    "sheet_count",
    "sheet_names",
    "status",
    "subject",
    "subtitle",
    "summary",
    "title",
    "total_cells",
    "version",
    "warning",
    "width",
    "word_count",
    "email_from",
    "email_to",
    "email_cc",
    "email_bcc",
    "date",
    "attachments",
    "table_count",
    "tables_summary",
    "quality_score",
    "image_preprocessing",
    "source_format",
    "converted_via",
    "error",
    "error_context",
    "json_schema",
    "notes",
    "note",
    "name",
    "body",
    "text",
    "message",
    "attributes",
    "token_reduction",
    "processing_errors",
    "extraction_error",
    "element_count",
    "unique_elements",
];

/// Extract metadata from Pandoc JSON AST
fn extract_metadata_from_json(json: &Value) -> Result<HashMap<String, Value>> {
    let mut metadata = HashMap::new();

    // Get meta object
    if let Some(meta) = json.get("meta").and_then(|m| m.as_object()) {
        for (key, value) in meta {
            let pandoc_key = get_pandoc_key(key);
            // Filter out invalid metadata keys (matches Python behavior)
            if !VALID_METADATA_KEYS.contains(&pandoc_key.as_str()) {
                continue;
            }
            if let Some(extracted) = extract_meta_value(value) {
                metadata.insert(pandoc_key, extracted);
            }
        }
    }

    // Extract citations from blocks
    if let Some(blocks) = json.get("blocks").and_then(|b| b.as_array()) {
        let mut citations = Vec::new();
        extract_citations_from_blocks(blocks, &mut citations);

        if !citations.is_empty() {
            if let Some(existing) = metadata.get_mut("citations") {
                if let Some(arr) = existing.as_array_mut() {
                    for cite in citations {
                        if !arr.contains(&Value::String(cite.clone())) {
                            arr.push(Value::String(cite));
                        }
                    }
                }
            } else {
                metadata.insert(
                    "citations".to_string(),
                    Value::Array(citations.into_iter().map(Value::String).collect()),
                );
            }
        }
    }

    // Extract citations from top-level if present
    if let Some(citations) = json.get("citations").and_then(|c| c.as_array()) {
        let cite_ids: Vec<String> = citations
            .iter()
            .filter_map(|c| c.get("citationId").and_then(|id| id.as_str()).map(String::from))
            .collect();

        if !cite_ids.is_empty() {
            metadata.insert(
                "citations".to_string(),
                Value::Array(cite_ids.into_iter().map(Value::String).collect()),
            );
        }
    }

    Ok(metadata)
}

/// Map Pandoc metadata keys to standard keys
fn get_pandoc_key(key: &str) -> String {
    match key {
        "abstract" => "summary".to_string(),
        "date" => "created_at".to_string(),
        "contributors" | "author" => "authors".to_string(),
        "institute" => "organization".to_string(),
        _ => key.to_string(),
    }
}

/// Extract value from Pandoc metadata node
fn extract_meta_value(node: &Value) -> Option<Value> {
    if let Some(obj) = node.as_object() {
        let node_type = obj.get("t")?.as_str()?;
        let content = obj.get("c");

        match node_type {
            "MetaString" => {
                if let Some(s) = content.and_then(|c| c.as_str()) {
                    return Some(Value::String(s.to_string()));
                }
            }
            "MetaInlines" => {
                if let Some(inlines) = content.and_then(|c| c.as_array()) {
                    return extract_inlines(inlines);
                }
            }
            "MetaList" => {
                if let Some(list) = content.and_then(|c| c.as_array()) {
                    let mut values = Vec::new();
                    for item in list {
                        if let Some(val) = extract_meta_value(item) {
                            if let Some(arr) = val.as_array() {
                                values.extend_from_slice(arr);
                            } else {
                                values.push(val);
                            }
                        }
                    }
                    if !values.is_empty() {
                        return Some(Value::Array(values));
                    }
                }
            }
            "MetaBlocks" => {
                if let Some(blocks) = content.and_then(|c| c.as_array()) {
                    let mut texts = Vec::new();
                    for block in blocks {
                        if let Some(block_obj) = block.as_object()
                            && block_obj.get("t")?.as_str()? == "Para"
                            && let Some(para_content) = block_obj.get("c").and_then(|c| c.as_array())
                            && let Some(text) = extract_inlines(para_content)
                            && let Some(s) = text.as_str()
                        {
                            texts.push(s.to_string());
                        }
                    }
                    if !texts.is_empty() {
                        return Some(Value::String(texts.join(" ")));
                    }
                }
            }
            "MetaMap" => {
                if let Some(map) = content.and_then(|c| c.as_object()) {
                    let mut result = serde_json::Map::new();
                    for (k, v) in map {
                        if let Some(val) = extract_meta_value(v) {
                            result.insert(k.clone(), val);
                        }
                    }
                    if !result.is_empty() {
                        return Some(Value::Object(result));
                    }
                }
            }
            _ => {}
        }
    }

    None
}

/// Extract inline text from Pandoc inline nodes
fn extract_inlines(inlines: &[Value]) -> Option<Value> {
    let mut texts = Vec::new();

    for inline in inlines {
        if let Some(text) = extract_inline_text(inline) {
            texts.push(text);
        }
    }

    let result = texts.join("");
    if result.is_empty() {
        None
    } else {
        Some(Value::String(result))
    }
}

/// Extract text from a single inline node
fn extract_inline_text(node: &Value) -> Option<String> {
    if let Some(obj) = node.as_object() {
        let node_type = obj.get("t")?.as_str()?;

        match node_type {
            "Str" => {
                return obj.get("c")?.as_str().map(String::from);
            }
            "Space" => {
                return Some(" ".to_string());
            }
            "Emph" | "Strong" | "Strikeout" | "Superscript" | "Subscript" | "SmallCaps" => {
                if let Some(content) = obj.get("c").and_then(|c| c.as_array()) {
                    return extract_inlines(content).and_then(|v| v.as_str().map(String::from));
                }
            }
            "Code" => {
                // Code: [Attr, Text]
                if let Some(arr) = obj.get("c").and_then(|c| c.as_array())
                    && arr.len() == 2
                {
                    return arr[1].as_str().map(String::from);
                }
            }
            "Link" | "Image" => {
                // Link/Image: [Attr, [Inline], Target]
                if let Some(arr) = obj.get("c").and_then(|c| c.as_array())
                    && arr.len() == 3
                    && let Some(inlines) = arr[1].as_array()
                {
                    return extract_inlines(inlines).and_then(|v| v.as_str().map(String::from));
                }
            }
            "Quoted" => {
                // Quoted: [QuoteType, [Inline]]
                if let Some(arr) = obj.get("c").and_then(|c| c.as_array())
                    && arr.len() == 2
                    && let Some(inlines) = arr[1].as_array()
                {
                    return extract_inlines(inlines).and_then(|v| v.as_str().map(String::from));
                }
            }
            "Cite" => {
                // Cite: [Citation], [Inline]
                if let Some(arr) = obj.get("c").and_then(|c| c.as_array())
                    && arr.len() == 2
                    && let Some(inlines) = arr[1].as_array()
                {
                    return extract_inlines(inlines).and_then(|v| v.as_str().map(String::from));
                }
            }
            "Math" => {
                // Math: [MathType, Text]
                if let Some(arr) = obj.get("c").and_then(|c| c.as_array())
                    && arr.len() == 2
                {
                    return arr[1].as_str().map(String::from);
                }
            }
            "LineBreak" | "SoftBreak" => {
                return Some("\n".to_string());
            }
            _ => {}
        }
    }

    None
}

/// Extract citations from block nodes
fn extract_citations_from_blocks(blocks: &[Value], citations: &mut Vec<String>) {
    for block in blocks {
        if let Some(obj) = block.as_object() {
            let block_type = obj.get("t").and_then(|t| t.as_str());

            // Check if this is a Cite block
            if block_type == Some("Cite")
                && let Some(arr) = obj.get("c").and_then(|c| c.as_array())
                && let Some(cite_list) = arr.first().and_then(|c| c.as_array())
            {
                for cite in cite_list {
                    if let Some(cite_id) = cite.get("citationId").and_then(|id| id.as_str()) {
                        citations.push(cite_id.to_string());
                    }
                }
            }

            // Recursively check content
            if let Some(content) = obj.get("c") {
                if let Some(nested_blocks) = content.as_array() {
                    extract_citations_from_blocks(nested_blocks, citations);
                } else if let Some(nested_obj) = content.as_object() {
                    // Handle nested structures
                    for value in nested_obj.values() {
                        if let Some(arr) = value.as_array() {
                            extract_citations_from_blocks(arr, citations);
                        }
                    }
                }
            }
        }
    }
}

/// Wrapper functions for backwards compatibility
pub async fn extract_with_pandoc(path: &Path, from_format: &str) -> Result<(String, HashMap<String, Value>)> {
    let content = extract_content(path, from_format).await?;
    let metadata = extract_metadata(path, from_format).await?;
    Ok((content, metadata))
}

pub async fn extract_with_pandoc_from_bytes(
    bytes: &[u8],
    from_format: &str,
    extension: &str,
) -> Result<(String, HashMap<String, Value>)> {
    // Create temporary file
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!(
        "pandoc_temp_{}_{}.{}",
        std::process::id(),
        uuid::Uuid::new_v4(),
        extension
    ));

    fs::write(&temp_file, bytes).await?;

    let result = extract_with_pandoc(&temp_file, from_format).await;

    let _ = fs::remove_file(&temp_file).await;

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_get_pandoc_key() {
        assert_eq!(get_pandoc_key("abstract"), "summary");
        assert_eq!(get_pandoc_key("date"), "created_at");
        assert_eq!(get_pandoc_key("author"), "authors");
        assert_eq!(get_pandoc_key("contributors"), "authors");
        assert_eq!(get_pandoc_key("institute"), "organization");
        assert_eq!(get_pandoc_key("title"), "title");
    }

    #[test]
    fn test_extract_meta_value_string() {
        let node = json!({
            "t": "MetaString",
            "c": "Test Title"
        });

        let result = extract_meta_value(&node).unwrap();
        assert_eq!(result, Value::String("Test Title".to_string()));
    }

    #[test]
    fn test_extract_meta_value_inlines() {
        let node = json!({
            "t": "MetaInlines",
            "c": [
                {"t": "Str", "c": "Hello"},
                {"t": "Space"},
                {"t": "Str", "c": "World"}
            ]
        });

        let result = extract_meta_value(&node).unwrap();
        assert_eq!(result, Value::String("Hello World".to_string()));
    }

    #[test]
    fn test_extract_meta_value_list() {
        let node = json!({
            "t": "MetaList",
            "c": [
                {"t": "MetaString", "c": "Author1"},
                {"t": "MetaString", "c": "Author2"}
            ]
        });

        let result = extract_meta_value(&node).unwrap();
        assert_eq!(
            result,
            Value::Array(vec![
                Value::String("Author1".to_string()),
                Value::String("Author2".to_string())
            ])
        );
    }

    #[test]
    fn test_extract_inline_text_str() {
        let node = json!({"t": "Str", "c": "Hello"});
        let result = extract_inline_text(&node).unwrap();
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_extract_inline_text_space() {
        let node = json!({"t": "Space"});
        let result = extract_inline_text(&node).unwrap();
        assert_eq!(result, " ");
    }

    #[test]
    fn test_extract_inline_text_emph() {
        let node = json!({
            "t": "Emph",
            "c": [
                {"t": "Str", "c": "emphasized"}
            ]
        });
        let result = extract_inline_text(&node).unwrap();
        assert_eq!(result, "emphasized");
    }

    #[test]
    fn test_extract_inline_text_code() {
        let node = json!({
            "t": "Code",
            "c": [["", [], []], "code_snippet"]
        });
        let result = extract_inline_text(&node).unwrap();
        assert_eq!(result, "code_snippet");
    }

    #[test]
    fn test_extract_inlines() {
        let inlines = vec![
            json!({"t": "Str", "c": "Hello"}),
            json!({"t": "Space"}),
            json!({"t": "Emph", "c": [{"t": "Str", "c": "World"}]}),
        ];

        let result = extract_inlines(&inlines).unwrap();
        assert_eq!(result, Value::String("Hello World".to_string()));
    }

    #[test]
    fn test_extract_citations_from_blocks() {
        let blocks = vec![json!({
            "t": "Cite",
            "c": [
                [
                    {"citationId": "cite1"},
                    {"citationId": "cite2"}
                ],
                []
            ]
        })];

        let mut citations = Vec::new();
        extract_citations_from_blocks(&blocks, &mut citations);

        assert_eq!(citations, vec!["cite1", "cite2"]);
    }

    #[test]
    fn test_extract_metadata_from_json() {
        let json = json!({
            "meta": {
                "title": {"t": "MetaString", "c": "Test Document"},
                "author": {"t": "MetaList", "c": [
                    {"t": "MetaString", "c": "Author One"}
                ]},
                "date": {"t": "MetaString", "c": "2024-01-01"}
            },
            "blocks": []
        });

        let metadata = extract_metadata_from_json(&json).unwrap();

        assert_eq!(
            metadata.get("title").unwrap(),
            &Value::String("Test Document".to_string())
        );
        assert_eq!(
            metadata.get("authors").unwrap(),
            &Value::Array(vec![Value::String("Author One".to_string())])
        );
        assert_eq!(
            metadata.get("created_at").unwrap(),
            &Value::String("2024-01-01".to_string())
        );
    }

    #[test]
    fn test_metadata_field_filtering() {
        // Test that invalid metadata fields are filtered out
        let json = json!({
            "meta": {
                "title": {"t": "MetaString", "c": "Valid Title"},
                "invalid_field": {"t": "MetaString", "c": "Should be filtered"},
                "random_key": {"t": "MetaString", "c": "Not in valid keys"},
                "author": {"t": "MetaString", "c": "Valid Author"}
            },
            "blocks": []
        });

        let metadata = extract_metadata_from_json(&json).unwrap();

        // Valid fields should be present
        assert!(metadata.contains_key("title"));
        assert!(metadata.contains_key("authors")); // author -> authors mapping

        // Invalid fields should be filtered out
        assert!(!metadata.contains_key("invalid_field"));
        assert!(!metadata.contains_key("random_key"));
    }
}
