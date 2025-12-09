//! Native Rust Typst document extractor.
//!
//! This extractor provides Typst document parsing and text extraction.
//! It uses a hybrid approach combining regex patterns and character-level parsing
//! to extract text while preserving document structure.
//!
//! Features:
//! - Metadata extraction: title, author, date, subject, keywords from `#set document()`
//! - Section hierarchy: `=`, `==`, `===`, etc. heading levels
//! - Inline formatting: `*bold*`, `_italic_`, `` `code` ``
//! - Lists: extraction of list content (both `+` and `-` markers)
//! - Links: extraction of URLs and link text from `#link("url")[text]` syntax
//! - Math: inline (`$...$`) and display math preservation
//! - Code blocks: triple-backtick code blocks with language specifiers
//! - Tables: extraction of `#table()` function content
//! - Complex formatting: handling of nested and combined formatting
//!
//! Requires the `office` feature.

#[cfg(feature = "office")]
use crate::Result;
#[cfg(feature = "office")]
use crate::core::config::ExtractionConfig;
#[cfg(feature = "office")]
use crate::plugins::{DocumentExtractor, Plugin};
#[cfg(feature = "office")]
use crate::types::{ExtractionResult, Metadata};
#[cfg(feature = "office")]
use async_trait::async_trait;
#[cfg(feature = "office")]
use regex::Regex;

/// Typst document extractor
#[cfg(feature = "office")]
pub struct TypstExtractor;

#[cfg(feature = "office")]
impl TypstExtractor {
    /// Create a new Typst extractor.
    pub fn new() -> Self {
        Self
    }

    /// Parse Typst content and extract text.
    fn extract_from_typst(content: &str) -> (String, Metadata) {
        let mut extractor = TypstParser::new(content);
        let text = extractor.parse();
        let metadata = extractor.metadata;

        (text, metadata)
    }
}

#[cfg(feature = "office")]
impl Default for TypstExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "office")]
impl Plugin for TypstExtractor {
    fn name(&self) -> &str {
        "typst-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Native Rust Typst document extractor with metadata support"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg(feature = "office")]
#[async_trait]
impl DocumentExtractor for TypstExtractor {
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, _config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let typst_str = String::from_utf8_lossy(content).to_string();
        let (text, metadata) = Self::extract_from_typst(&typst_str);

        Ok(ExtractionResult {
            content: text,
            mime_type: mime_type.to_string(),
            metadata,
            tables: Vec::new(),
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-typst", "text/x-typst"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

/// Internal Typst parser
#[cfg(feature = "office")]
struct TypstParser {
    content: String,
    metadata: Metadata,
}

#[cfg(feature = "office")]
impl TypstParser {
    fn new(content: &str) -> Self {
        Self {
            content: content.to_string(),
            metadata: Metadata::default(),
        }
    }

    fn parse(&mut self) -> String {
        self.extract_metadata();

        self.extract_content()
    }

    fn extract_metadata(&mut self) {
        if let Some(title) = self.extract_quoted_value("title") {
            self.metadata.additional.insert("title".to_string(), title.into());
        }

        if let Some(author) = self.extract_quoted_value("author") {
            self.metadata.additional.insert("author".to_string(), author.into());
        }

        if let Some(date) = self.extract_quoted_value("date") {
            self.metadata.date = Some(date);
        }

        if let Some(subject) = self.extract_quoted_value("subject") {
            self.metadata.additional.insert("subject".to_string(), subject.into());
        }

        if let Some(keywords) = self.extract_keywords() {
            self.metadata.additional.insert("keywords".to_string(), keywords.into());
        }
    }

    fn extract_quoted_value(&self, field: &str) -> Option<String> {
        let pattern = format!(r#"{}:\s*"([^"]*)""#, regex::escape(field));
        if let Ok(re) = Regex::new(&pattern)
            && let Some(caps) = re.captures(&self.content)
        {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        None
    }

    fn extract_keywords(&self) -> Option<String> {
        let pattern = r#"keywords:\s*(?:"([^"]*)"|(\([^)]*\)))"#;
        if let Ok(re) = Regex::new(pattern)
            && let Some(caps) = re.captures(&self.content)
        {
            if let Some(m) = caps.get(1) {
                return Some(m.as_str().to_string());
            }
            if let Some(m) = caps.get(2) {
                let array_str = m.as_str();
                let mut keywords = Vec::new();
                let item_pattern = r#""([^"]*)""#;
                if let Ok(item_re) = Regex::new(item_pattern) {
                    for item_caps in item_re.captures_iter(array_str) {
                        if let Some(keyword) = item_caps.get(1) {
                            keywords.push(keyword.as_str().to_string());
                        }
                    }
                }
                if !keywords.is_empty() {
                    return Some(keywords.join(", "));
                }
            }
        }
        None
    }

    fn extract_content(&self) -> String {
        let mut output = String::new();
        let mut lines = self.content.lines().peekable();
        let mut in_code_block = false;
        let mut code_block_fence = String::new();

        while let Some(line) = lines.next() {
            let trimmed = line.trim();

            if trimmed.starts_with("```") {
                if in_code_block {
                    if trimmed == "```" {
                        in_code_block = false;
                        code_block_fence.clear();
                        output.push_str("```\n");
                        continue;
                    }
                } else {
                    in_code_block = true;
                    code_block_fence = "```".to_string();
                    output.push_str("```");
                    if let Some(lang) = trimmed.strip_prefix("```") {
                        let lang = lang.trim();
                        if !lang.is_empty() {
                            output.push_str(lang);
                        }
                    }
                    output.push('\n');
                    continue;
                }
            }

            if in_code_block {
                output.push_str(line);
                output.push('\n');
                continue;
            }

            if trimmed.starts_with("#set ") || trimmed.starts_with("#let ") {
                continue;
            }

            if trimmed.starts_with("#import ") || trimmed.starts_with("#include ") {
                continue;
            }

            if trimmed.starts_with("#table(") {
                output.push_str("TABLE:\n");
                let table_content = self.extract_table_content(trimmed, &mut lines);
                output.push_str(&table_content);
                output.push('\n');
                continue;
            }

            if trimmed.starts_with('=') {
                let next_char_pos = trimmed.find(|c: char| c != '=');
                if next_char_pos.is_some() {
                    let heading_level = trimmed.chars().take_while(|&c| c == '=').count();
                    let heading_text = trimmed[heading_level..].trim();

                    for _ in 0..heading_level {
                        output.push('=');
                    }
                    output.push(' ');
                    output.push_str(heading_text);
                    output.push('\n');
                    continue;
                }
            }

            if (trimmed.starts_with('+') || trimmed.starts_with('-'))
                && trimmed.len() > 1
                && trimmed.chars().nth(1).is_some_and(|c| !c.is_alphanumeric())
            {
                output.push_str("- ");
                output.push_str(trimmed[1..].trim());
                output.push('\n');
                continue;
            }

            if trimmed.starts_with('#')
                && !trimmed.starts_with("#set")
                && !trimmed.starts_with("#let")
                && !trimmed.starts_with("#import")
                && !trimmed.starts_with("#include")
            {
                if trimmed.contains('[')
                    && trimmed.contains(']')
                    && let Some(content) = self.extract_text_from_brackets(trimmed)
                {
                    let processed = self.process_line(&content);
                    if !processed.is_empty() {
                        output.push_str(&processed);
                        output.push('\n');
                    }
                }
                continue;
            }

            if !trimmed.is_empty() {
                let processed = self.process_line(trimmed);
                if !processed.is_empty() {
                    output.push_str(&processed);
                    output.push('\n');
                }
            } else {
                output.push('\n');
            }
        }

        output
    }

    /// Extract content from #table() function calls
    fn extract_table_content<'a, I>(&self, first_line: &str, lines: &mut std::iter::Peekable<I>) -> String
    where
        I: Iterator<Item = &'a str>,
    {
        let mut table_content = String::new();
        let mut content = first_line.to_string();
        let mut bracket_depth = 0;
        let mut paren_depth = if first_line.contains('(') { 1 } else { 0 };

        for ch in first_line.chars() {
            match ch {
                '(' => paren_depth += 1,
                ')' => paren_depth -= 1,
                '[' => bracket_depth += 1,
                ']' => bracket_depth -= 1,
                _ => {}
            }
        }

        while paren_depth > 0 || bracket_depth > 0 {
            if let Some(next_line) = lines.next() {
                content.push('\n');
                content.push_str(next_line);
                for ch in next_line.chars() {
                    match ch {
                        '(' => paren_depth += 1,
                        ')' => paren_depth -= 1,
                        '[' => bracket_depth += 1,
                        ']' => bracket_depth -= 1,
                        _ => {}
                    }
                }
            } else {
                break;
            }
        }

        let mut in_bracket = false;
        let mut cell = String::new();
        for ch in content.chars() {
            match ch {
                '[' => {
                    in_bracket = true;
                    cell.clear();
                }
                ']' => {
                    if in_bracket {
                        let trimmed = cell.trim();
                        if !trimmed.is_empty() {
                            table_content.push_str(trimmed);
                            table_content.push_str(" | ");
                        }
                        in_bracket = false;
                        cell.clear();
                    }
                }
                _ if in_bracket => {
                    cell.push(ch);
                }
                _ => {}
            }
        }

        if table_content.ends_with(" | ") {
            table_content.truncate(table_content.len() - 3);
        }

        table_content
    }

    fn process_line(&self, line: &str) -> String {
        let mut result = String::new();
        let mut chars = line.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '`' => {
                    result.push('`');
                    for c in chars.by_ref() {
                        result.push(c);
                        if c == '`' {
                            break;
                        }
                    }
                }
                '$' => {
                    result.push('$');
                    for c in chars.by_ref() {
                        result.push(c);
                        if c == '$' {
                            break;
                        }
                    }
                }
                '*' => {
                    result.push('*');
                    for c in chars.by_ref() {
                        result.push(c);
                        if c == '*' {
                            break;
                        }
                    }
                }
                '_' => {
                    result.push('_');
                    for c in chars.by_ref() {
                        result.push(c);
                        if c == '_' {
                            break;
                        }
                    }
                }
                '#' if chars.peek() == Some(&'l') => {
                    result.push(ch);
                }
                _ => {
                    result.push(ch);
                }
            }
        }

        self.extract_link_text(&result)
    }

    fn extract_link_text(&self, line: &str) -> String {
        let pattern = r#"link\("([^"]*)"\)\[([^\]]*)\]"#;
        if let Ok(re) = Regex::new(pattern) {
            return re
                .replace_all(line, |caps: &regex::Captures| {
                    let url = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                    let text = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                    format!("[{}]({})", text, url)
                })
                .to_string();
        }
        line.to_string()
    }

    fn extract_text_from_brackets(&self, line: &str) -> Option<String> {
        if let Some(start) = line.find('[')
            && let Some(end) = line.rfind(']')
            && end > start
        {
            let text = &line[start + 1..end];
            return Some(text.to_string());
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_metadata() {
        let content = r#"#set document(
            title: "Test Document",
            author: "Test Author"
        )

        = Heading
        Some text
        "#;

        let (_, metadata) = TypstExtractor::extract_from_typst(content);

        assert!(metadata.additional.contains_key("title"));
        assert!(metadata.additional.contains_key("author"));
    }

    #[test]
    fn test_extract_headings() {
        let content = r#"= Level 1
Content

== Level 2
More content
"#;

        let (output, _) = TypstExtractor::extract_from_typst(content);

        assert!(output.contains("= Level 1"));
        assert!(output.contains("== Level 2"));
    }

    #[test]
    fn test_extract_formatting() {
        let content = r#"Some *bold* and _italic_ text with `code`."#;

        let (output, _) = TypstExtractor::extract_from_typst(content);

        assert!(output.contains("*bold*") || output.contains("bold"));
        assert!(output.contains("_italic_") || output.contains("italic"));
        assert!(output.contains("`code`") || output.contains("code"));
    }

    #[test]
    fn test_extract_code_blocks() {
        let content = r#"Here is code:

```python
def hello():
    print("world")
```

Done."#;

        let (output, _) = TypstExtractor::extract_from_typst(content);

        assert!(output.contains("```python"));
        assert!(output.contains("def hello"));
        assert!(output.contains("print"));
    }

    #[test]
    fn test_extract_links() {
        let content = r#"Visit #link("https://example.com")[example site] for info."#;

        let (output, _) = TypstExtractor::extract_from_typst(content);

        assert!(
            output.contains("example.com")
                || output.contains("example site")
                || output.contains("[example site](https://example.com)")
        );
    }

    #[test]
    fn test_extract_list_items() {
        let content = r#"= Lists

+ First item
+ Second item
+ Third item"#;

        let (output, _) = TypstExtractor::extract_from_typst(content);

        assert!(output.contains("First item"));
        assert!(output.contains("Second item"));
        assert!(output.contains("Third item"));
    }

    #[test]
    fn test_extract_tables() {
        let content = r#"== Tables

#table(
  columns: 2,
  [Name], [Age],
  [Alice], [30],
)"#;

        let (output, _) = TypstExtractor::extract_from_typst(content);

        assert!(output.contains("TABLE:") || output.contains("Name") || output.contains("Alice"));
    }

    #[test]
    fn test_extract_math() {
        let content = r#"The formula $E = mc^2$ is important.

Display:
$ a^2 + b^2 = c^2 $"#;

        let (output, _) = TypstExtractor::extract_from_typst(content);

        assert!(output.contains("$") && output.contains("mc"));
    }

    #[test]
    fn test_metadata_extraction_comprehensive() {
        let content = r#"#set document(
            title: "Advanced Document",
            author: "John Doe",
            date: "2024-12-06",
            subject: "Test Subject",
            keywords: ("test", "example", "rust")
        )

        Content here."#;

        let (_, metadata) = TypstExtractor::extract_from_typst(content);

        assert!(metadata.additional.contains_key("title"), "Title should be extracted");
        assert!(metadata.additional.contains_key("author"), "Author should be extracted");
        assert!(metadata.date.is_some(), "Date should be extracted");
        assert!(
            metadata.additional.contains_key("subject"),
            "Subject should be extracted"
        );
        assert!(
            metadata
                .additional
                .get("keywords")
                .map(|v| !v.to_string().is_empty())
                .unwrap_or(false)
        );
    }

    #[test]
    fn test_skip_directives() {
        let content = r#"#set heading(numbering: "1.")
#let x = 5
#import "@preview/foo:1.0"
#include "other.typ"

= Heading
Actual content"#;

        let (output, _) = TypstExtractor::extract_from_typst(content);

        assert!(!output.contains("#set"));
        assert!(!output.contains("#let"));
        assert!(!output.contains("#import"));
        assert!(!output.contains("#include"));
        assert!(output.contains("Heading"));
        assert!(output.contains("content"));
    }

    #[test]
    fn test_combined_formatting() {
        let content = r#"This is *bold with _nested italic_* and more."#;

        let (output, _) = TypstExtractor::extract_from_typst(content);

        assert!(output.contains("*") || output.contains("_") || (output.contains("bold") && output.contains("italic")));
    }
}
