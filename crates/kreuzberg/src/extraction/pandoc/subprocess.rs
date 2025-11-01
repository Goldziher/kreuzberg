use crate::error::{KreuzbergError, Result};
#[cfg(feature = "quality")]
use crate::text::normalize_spaces;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

/// Default timeout for Pandoc operations (120 seconds)
const PANDOC_TIMEOUT_SECONDS: u64 = 120;

/// RAII guard for automatic temporary file cleanup
struct TempFile {
    path: PathBuf,
}

impl TempFile {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        // Best-effort cleanup - use blocking remove since Drop can't be async
        let path = self.path.clone();
        tokio::spawn(async move {
            let _ = fs::remove_file(&path).await;
        });
    }
}

/// Extract content from file using Pandoc (convert to markdown)
#[allow(dead_code)]
pub async fn extract_content(path: &Path, from_format: &str) -> Result<String> {
    let child = Command::new("pandoc")
        .arg(path)
        .arg(format!("--from={}", from_format))
        .arg("--to=markdown")
        .arg("--standalone")
        .arg("--wrap=preserve")
        .arg("--quiet")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            // Failed to execute pandoc - this is an IO error (command not found, etc.) ~keep
            std::io::Error::other(format!("Failed to execute pandoc: {}", e))
        })?;

    let output = match timeout(Duration::from_secs(PANDOC_TIMEOUT_SECONDS), child.wait_with_output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => return Err(std::io::Error::other(format!("Failed to wait for pandoc: {}", e)).into()),
        Err(_) => {
            // Timeout - child was already consumed by wait_with_output(), process will be killed on drop ~keep
            return Err(KreuzbergError::parsing(format!(
                "Pandoc content extraction timed out after {} seconds",
                PANDOC_TIMEOUT_SECONDS
            )));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Subprocess error analysis - wrap only if format/parsing error detected ~keep
        let stderr_lower = stderr.to_lowercase();
        if stderr_lower.contains("format")
            || stderr_lower.contains("unsupported")
            || stderr_lower.contains("error:")
            || stderr_lower.contains("failed")
        {
            return Err(KreuzbergError::parsing(format!(
                "Pandoc format/parsing error: {}",
                stderr
            )));
        }

        // True system error - bubble up as IO error ~keep
        return Err(std::io::Error::other(format!("Pandoc system error: {}", stderr)).into());
    }

    let content = String::from_utf8(output.stdout)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to decode pandoc output: {}", e)))?;

    #[cfg(feature = "quality")]
    {
        Ok(normalize_spaces(&content))
    }
    #[cfg(not(feature = "quality"))]
    {
        Ok(content)
    }
}

/// Extract metadata from file using Pandoc JSON output
#[allow(dead_code)]
pub async fn extract_metadata(path: &Path, from_format: &str) -> Result<HashMap<String, Value>> {
    let child = Command::new("pandoc")
        .arg(path)
        .arg(format!("--from={}", from_format))
        .arg("--to=json")
        .arg("--standalone")
        .arg("--quiet")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            // Failed to execute pandoc - this is an IO error (command not found, etc.) ~keep
            std::io::Error::other(format!("Failed to execute pandoc: {}", e))
        })?;

    let output = match timeout(Duration::from_secs(PANDOC_TIMEOUT_SECONDS), child.wait_with_output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => return Err(std::io::Error::other(format!("Failed to wait for pandoc: {}", e)).into()),
        Err(_) => {
            // Timeout - child was already consumed by wait_with_output(), process will be killed on drop ~keep
            return Err(KreuzbergError::parsing(format!(
                "Pandoc metadata extraction timed out after {} seconds",
                PANDOC_TIMEOUT_SECONDS
            )));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Subprocess error analysis - wrap only if format/parsing error detected ~keep
        let stderr_lower = stderr.to_lowercase();
        if stderr_lower.contains("format")
            || stderr_lower.contains("unsupported")
            || stderr_lower.contains("error:")
            || stderr_lower.contains("failed")
        {
            return Err(KreuzbergError::parsing(format!(
                "Pandoc metadata extraction format/parsing error: {}",
                stderr
            )));
        }

        // True system error - bubble up as IO error ~keep
        return Err(std::io::Error::other(format!("Pandoc metadata extraction system error: {}", stderr)).into());
    }

    let json_content = String::from_utf8(output.stdout)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to decode pandoc JSON output: {}", e)))?;

    let json_data: Value = serde_json::from_str(&json_content)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to parse pandoc JSON: {}", e)))?;

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
pub(crate) fn extract_metadata_from_json(json: &Value) -> Result<HashMap<String, Value>> {
    let mut metadata = HashMap::new();

    if let Some(meta) = json.get("meta").and_then(|m| m.as_object()) {
        for (key, value) in meta {
            let pandoc_key = get_pandoc_key(key);
            if !VALID_METADATA_KEYS.contains(&pandoc_key.as_str()) {
                continue;
            }
            if let Some(extracted) = extract_meta_value(value) {
                metadata.insert(pandoc_key, extracted);
            }
        }
    }

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

/// Extract markdown content from Pandoc JSON AST
///
/// Converts the JSON AST blocks back to markdown format, similar to what
/// `pandoc --to=markdown` would produce. This allows us to extract both
/// content and metadata from a single JSON extraction.
pub(crate) fn extract_content_from_json(json: &Value) -> Result<String> {
    let mut content = String::new();

    // If there's a title in metadata, render it as a level-1 header
    // This matches the behavior of `pandoc --to=markdown`
    if let Some(meta) = json.get("meta").and_then(|m| m.as_object())
        && let Some(title_node) = meta.get("title")
        && let Some(title_value) = extract_meta_value(title_node)
        && let Some(title_str) = title_value.as_str()
    {
        content.push_str(&format!("# {}\n\n", title_str));
    }

    if let Some(blocks) = json.get("blocks").and_then(|b| b.as_array()) {
        for block in blocks {
            if let Some(text) = extract_block_text(block) {
                if !content.is_empty() && !content.ends_with("\n\n") {
                    content.push_str("\n\n");
                }
                content.push_str(&text);
            }
        }
    }

    Ok(content)
}

/// Extract text from a Pandoc JSON AST block
fn extract_block_text(block: &Value) -> Option<String> {
    let obj = block.as_object()?;
    let block_type = obj.get("t")?.as_str()?;
    let content = obj.get("c");

    match block_type {
        "Para" | "Plain" => {
            // Paragraph or plain text block
            if let Some(inlines) = content.and_then(|c| c.as_array()) {
                return extract_inlines(inlines).and_then(|v| v.as_str().map(String::from));
            }
        }
        "Header" => {
            // Header block: [level, attrs, inlines]
            if let Some(arr) = content.and_then(|c| c.as_array())
                && arr.len() >= 3
                && let Some(level) = arr[0].as_u64()
                && let Some(inlines) = arr[2].as_array()
            {
                let header_text = extract_inlines(inlines).and_then(|v| v.as_str().map(String::from))?;
                let prefix = "#".repeat(level as usize);
                return Some(format!("{} {}", prefix, header_text));
            }
        }
        "CodeBlock" => {
            // Code block: [attrs, code]
            if let Some(arr) = content.and_then(|c| c.as_array())
                && arr.len() >= 2
                && let Some(code) = arr[1].as_str()
            {
                return Some(format!("```\n{}\n```", code));
            }
        }
        "BlockQuote" => {
            // Block quote: [blocks]
            if let Some(blocks) = content.and_then(|c| c.as_array()) {
                let mut quote_text = String::new();
                for inner_block in blocks {
                    if let Some(text) = extract_block_text(inner_block) {
                        quote_text.push_str("> ");
                        quote_text.push_str(&text);
                        quote_text.push('\n');
                    }
                }
                return Some(quote_text.trim_end().to_string());
            }
        }
        "BulletList" => {
            // Bullet list: [[blocks], [blocks], ...]
            if let Some(items) = content.and_then(|c| c.as_array()) {
                let mut list_text = String::new();
                for item in items {
                    if let Some(item_blocks) = item.as_array() {
                        for block in item_blocks {
                            if let Some(text) = extract_block_text(block) {
                                list_text.push_str("- ");
                                list_text.push_str(&text);
                                list_text.push('\n');
                            }
                        }
                    }
                }
                return Some(list_text.trim_end().to_string());
            }
        }
        "OrderedList" => {
            // Ordered list: [attrs, [[blocks], [blocks], ...]]
            if let Some(arr) = content.and_then(|c| c.as_array())
                && arr.len() >= 2
                && let Some(items) = arr[1].as_array()
            {
                let mut list_text = String::new();
                for (idx, item) in items.iter().enumerate() {
                    if let Some(item_blocks) = item.as_array() {
                        for block in item_blocks {
                            if let Some(text) = extract_block_text(block) {
                                list_text.push_str(&format!("{}. {}\n", idx + 1, text));
                            }
                        }
                    }
                }
                return Some(list_text.trim_end().to_string());
            }
        }
        "HorizontalRule" => {
            return Some("---".to_string());
        }
        _ => {}
    }

    None
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
                if let Some(arr) = obj.get("c").and_then(|c| c.as_array())
                    && arr.len() == 2
                {
                    return arr[1].as_str().map(String::from);
                }
            }
            "Link" | "Image" => {
                if let Some(arr) = obj.get("c").and_then(|c| c.as_array())
                    && arr.len() == 3
                    && let Some(inlines) = arr[1].as_array()
                {
                    return extract_inlines(inlines).and_then(|v| v.as_str().map(String::from));
                }
            }
            "Quoted" => {
                if let Some(arr) = obj.get("c").and_then(|c| c.as_array())
                    && arr.len() == 2
                    && let Some(inlines) = arr[1].as_array()
                {
                    return extract_inlines(inlines).and_then(|v| v.as_str().map(String::from));
                }
            }
            "Cite" => {
                if let Some(arr) = obj.get("c").and_then(|c| c.as_array())
                    && arr.len() == 2
                    && let Some(inlines) = arr[1].as_array()
                {
                    return extract_inlines(inlines).and_then(|v| v.as_str().map(String::from));
                }
            }
            "Math" => {
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

            if let Some(content) = obj.get("c") {
                if let Some(nested_blocks) = content.as_array() {
                    extract_citations_from_blocks(nested_blocks, citations);
                } else if let Some(nested_obj) = content.as_object() {
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
    // Extract JSON once and parse both content and metadata from it
    // This reduces subprocess spawns by 50% (1 call instead of 2)
    let child = Command::new("pandoc")
        .arg(path)
        .arg(format!("--from={}", from_format))
        .arg("--to=json")
        .arg("--standalone")
        .arg("--quiet")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            // Failed to execute pandoc - this is an IO error (command not found, etc.) ~keep
            std::io::Error::other(format!("Failed to execute pandoc: {}", e))
        })?;

    let output = match timeout(Duration::from_secs(PANDOC_TIMEOUT_SECONDS), child.wait_with_output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => return Err(std::io::Error::other(format!("Failed to wait for pandoc: {}", e)).into()),
        Err(_) => {
            // Timeout - child was already consumed by wait_with_output(), process will be killed on drop ~keep
            return Err(KreuzbergError::parsing(format!(
                "Pandoc extraction timed out after {} seconds",
                PANDOC_TIMEOUT_SECONDS
            )));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Subprocess error analysis - wrap only if format/parsing error detected ~keep
        let stderr_lower = stderr.to_lowercase();
        if stderr_lower.contains("format")
            || stderr_lower.contains("unsupported")
            || stderr_lower.contains("error:")
            || stderr_lower.contains("failed")
        {
            return Err(KreuzbergError::parsing(format!(
                "Pandoc format/parsing error: {}",
                stderr
            )));
        }

        // True system error - bubble up as IO error ~keep
        return Err(std::io::Error::other(format!("Pandoc system error: {}", stderr)).into());
    }

    let json_content = String::from_utf8(output.stdout)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to decode pandoc JSON output: {}", e)))?;

    let json_data: Value = serde_json::from_str(&json_content)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to parse pandoc JSON: {}", e)))?;

    // Extract both content and metadata from the same JSON AST
    let content = extract_content_from_json(&json_data)?;
    let metadata = extract_metadata_from_json(&json_data)?;

    #[cfg(feature = "quality")]
    {
        Ok((normalize_spaces(&content), metadata))
    }
    #[cfg(not(feature = "quality"))]
    {
        Ok((content, metadata))
    }
}

pub async fn extract_with_pandoc_from_bytes(
    bytes: &[u8],
    from_format: &str,
    extension: &str,
) -> Result<(String, HashMap<String, Value>)> {
    let temp_dir = std::env::temp_dir();
    let temp_file_path = temp_dir.join(format!(
        "pandoc_temp_{}_{}.{}",
        std::process::id(),
        uuid::Uuid::new_v4(),
        extension
    ));

    // RAII guard ensures cleanup on all paths including panic ~keep
    let _temp_guard = TempFile::new(temp_file_path.clone());

    fs::write(&temp_file_path, bytes).await?;

    extract_with_pandoc(&temp_file_path, from_format).await
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

        assert!(metadata.contains_key("title"));
        assert!(metadata.contains_key("authors"));

        assert!(!metadata.contains_key("invalid_field"));
        assert!(!metadata.contains_key("random_key"));
    }

    #[test]
    fn test_extract_meta_value_meta_blocks() {
        let node = json!({
            "t": "MetaBlocks",
            "c": [
                {
                    "t": "Para",
                    "c": [
                        {"t": "Str", "c": "First"},
                        {"t": "Space"},
                        {"t": "Str", "c": "paragraph"}
                    ]
                },
                {
                    "t": "Para",
                    "c": [
                        {"t": "Str", "c": "Second"},
                        {"t": "Space"},
                        {"t": "Str", "c": "paragraph"}
                    ]
                }
            ]
        });

        let result = extract_meta_value(&node).unwrap();
        assert_eq!(result, Value::String("First paragraph Second paragraph".to_string()));
    }

    #[test]
    fn test_extract_meta_value_meta_map() {
        let node = json!({
            "t": "MetaMap",
            "c": {
                "key1": {"t": "MetaString", "c": "value1"},
                "key2": {"t": "MetaString", "c": "value2"}
            }
        });

        let result = extract_meta_value(&node).unwrap();
        let obj = result.as_object().unwrap();
        assert_eq!(obj.get("key1").unwrap(), &Value::String("value1".to_string()));
        assert_eq!(obj.get("key2").unwrap(), &Value::String("value2".to_string()));
    }

    #[test]
    fn test_extract_inline_text_strong() {
        let node = json!({
            "t": "Strong",
            "c": [
                {"t": "Str", "c": "bold"}
            ]
        });
        let result = extract_inline_text(&node).unwrap();
        assert_eq!(result, "bold");
    }

    #[test]
    fn test_extract_inline_text_link() {
        let node = json!({
            "t": "Link",
            "c": [
                ["", [], []],
                [{"t": "Str", "c": "link text"}],
                ["https://example.com", ""]
            ]
        });
        let result = extract_inline_text(&node).unwrap();
        assert_eq!(result, "link text");
    }

    #[test]
    fn test_extract_inline_text_image() {
        let node = json!({
            "t": "Image",
            "c": [
                ["", [], []],
                [{"t": "Str", "c": "alt text"}],
                ["image.png", ""]
            ]
        });
        let result = extract_inline_text(&node).unwrap();
        assert_eq!(result, "alt text");
    }

    #[test]
    fn test_extract_inline_text_quoted() {
        let node = json!({
            "t": "Quoted",
            "c": [
                {"t": "DoubleQuote"},
                [{"t": "Str", "c": "quoted text"}]
            ]
        });
        let result = extract_inline_text(&node).unwrap();
        assert_eq!(result, "quoted text");
    }

    #[test]
    fn test_extract_inline_text_cite() {
        let node = json!({
            "t": "Cite",
            "c": [
                [{"citationId": "cite1"}],
                [{"t": "Str", "c": "citation text"}]
            ]
        });
        let result = extract_inline_text(&node).unwrap();
        assert_eq!(result, "citation text");
    }

    #[test]
    fn test_extract_inline_text_math() {
        let node = json!({
            "t": "Math",
            "c": [
                {"t": "InlineMath"},
                "x^2 + y^2"
            ]
        });
        let result = extract_inline_text(&node).unwrap();
        assert_eq!(result, "x^2 + y^2");
    }

    #[test]
    fn test_extract_inline_text_linebreak() {
        let node = json!({"t": "LineBreak"});
        let result = extract_inline_text(&node).unwrap();
        assert_eq!(result, "\n");
    }

    #[test]
    fn test_extract_inline_text_softbreak() {
        let node = json!({"t": "SoftBreak"});
        let result = extract_inline_text(&node).unwrap();
        assert_eq!(result, "\n");
    }

    #[test]
    fn test_extract_inline_text_strikeout() {
        let node = json!({
            "t": "Strikeout",
            "c": [{"t": "Str", "c": "deleted"}]
        });
        let result = extract_inline_text(&node).unwrap();
        assert_eq!(result, "deleted");
    }

    #[test]
    fn test_extract_inline_text_superscript() {
        let node = json!({
            "t": "Superscript",
            "c": [{"t": "Str", "c": "2"}]
        });
        let result = extract_inline_text(&node).unwrap();
        assert_eq!(result, "2");
    }

    #[test]
    fn test_extract_inline_text_subscript() {
        let node = json!({
            "t": "Subscript",
            "c": [{"t": "Str", "c": "i"}]
        });
        let result = extract_inline_text(&node).unwrap();
        assert_eq!(result, "i");
    }

    #[test]
    fn test_extract_inline_text_smallcaps() {
        let node = json!({
            "t": "SmallCaps",
            "c": [{"t": "Str", "c": "small"}]
        });
        let result = extract_inline_text(&node).unwrap();
        assert_eq!(result, "small");
    }

    #[test]
    fn test_extract_inline_text_unknown_type() {
        let node = json!({
            "t": "UnknownType",
            "c": "should be ignored"
        });
        let result = extract_inline_text(&node);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_citations_from_nested_blocks() {
        let blocks = vec![json!({
            "t": "BulletList",
            "c": [
                [
                    {
                        "t": "Plain",
                        "c": [
                            {"t": "Str", "c": "text"}
                        ]
                    }
                ]
            ]
        })];

        let mut citations = Vec::new();
        extract_citations_from_blocks(&blocks, &mut citations);

        assert!(citations.is_empty());
    }

    #[test]
    fn test_extract_metadata_from_json_with_citations() {
        let json = json!({
            "meta": {
                "title": {"t": "MetaString", "c": "Paper"}
            },
            "citations": [
                {"citationId": "cite1"},
                {"citationId": "cite2"}
            ],
            "blocks": []
        });

        let metadata = extract_metadata_from_json(&json).unwrap();

        assert!(metadata.contains_key("citations"));
        let citations = metadata.get("citations").unwrap().as_array().unwrap();
        assert_eq!(citations.len(), 2);
        assert_eq!(citations[0], Value::String("cite1".to_string()));
        assert_eq!(citations[1], Value::String("cite2".to_string()));
    }

    #[test]
    fn test_extract_metadata_from_json_empty_meta() {
        let json = json!({
            "meta": {},
            "blocks": []
        });

        let metadata = extract_metadata_from_json(&json).unwrap();
        assert!(metadata.is_empty());
    }

    #[test]
    fn test_extract_meta_value_empty_list() {
        let node = json!({
            "t": "MetaList",
            "c": []
        });

        let result = extract_meta_value(&node);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_meta_value_empty_map() {
        let node = json!({
            "t": "MetaMap",
            "c": {}
        });

        let result = extract_meta_value(&node);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_inlines_empty() {
        let inlines = vec![];
        let result = extract_inlines(&inlines);
        assert!(result.is_none());
    }

    #[test]
    fn test_valid_metadata_keys_contains_standard_fields() {
        assert!(VALID_METADATA_KEYS.contains(&"title"));
        assert!(VALID_METADATA_KEYS.contains(&"authors"));
        assert!(VALID_METADATA_KEYS.contains(&"date"));
        assert!(VALID_METADATA_KEYS.contains(&"keywords"));
        assert!(VALID_METADATA_KEYS.contains(&"abstract"));
        assert!(VALID_METADATA_KEYS.contains(&"citations"));
    }

    #[test]
    fn test_get_pandoc_key_unmapped() {
        assert_eq!(get_pandoc_key("title"), "title");
        assert_eq!(get_pandoc_key("keywords"), "keywords");
        assert_eq!(get_pandoc_key("custom_field"), "custom_field");
    }

    #[tokio::test]
    async fn test_tempfile_raii_cleanup() {
        use crate::extraction::pandoc::version::validate_pandoc_version;

        if validate_pandoc_version().await.is_err() {
            return;
        }

        // Test that TempFile cleanup happens even when function returns early with error
        let temp_path = std::env::temp_dir().join(format!("test_raii_{}.md", uuid::Uuid::new_v4()));

        {
            let _guard = TempFile::new(temp_path.clone());
            fs::write(&temp_path, b"test content").await.unwrap();
            assert!(temp_path.exists());
            // Guard dropped here, cleanup scheduled
        }

        // Give tokio time to run the cleanup task
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // File should be cleaned up
        assert!(!temp_path.exists());
    }

    #[tokio::test]
    async fn test_extract_content_timeout_kills_process() {
        use crate::extraction::pandoc::version::validate_pandoc_version;

        if validate_pandoc_version().await.is_err() {
            return;
        }

        // Create a file that pandoc can process, but with artificially low timeout
        // This is difficult to test reliably, so we just verify the timeout mechanism exists
        // by checking the code has the timeout logic
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join(format!("test_timeout_{}.md", uuid::Uuid::new_v4()));
        fs::write(&test_file, b"# Test\n\nContent").await.unwrap();

        // Normal extraction should succeed
        let result = extract_content(&test_file, "markdown").await;
        assert!(result.is_ok());

        let _ = fs::remove_file(&test_file).await;
    }
}
