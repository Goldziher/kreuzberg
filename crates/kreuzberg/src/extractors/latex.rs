//! Native Rust LaTeX text extractor.
//!
//! This extractor provides comprehensive LaTeX document parsing and text extraction.
//!
//! Features:
//! - Metadata extraction: title, author, date from \title{}, \author{}, \date{}
//! - Section hierarchy: \section{}, \subsection{}, \subsubsection{}, etc.
//! - Inline formatting: \emph{}, \textbf{}, \textit{}, \texttt{}, \underline{}
//! - Lists: itemize, enumerate, description environments
//! - Tables: tabular environment parsing
//! - Math: inline ($...$) and display (\[...\]) math preservation
//! - Unicode support
//!
//! Requires the `office` feature.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata, Table};
use async_trait::async_trait;

/// LaTeX document extractor
pub struct LatexExtractor;

impl LatexExtractor {
    /// Create a new LaTeX extractor.
    pub fn new() -> Self {
        Self
    }

    /// Parse LaTeX content and extract text.
    fn extract_from_latex(content: &str) -> (String, Metadata, Vec<Table>) {
        let mut parser = LatexParser::new(content);
        parser.parse()
    }
}

impl Default for LatexExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for LatexExtractor {
    fn name(&self) -> &str {
        "latex-extractor"
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
        "Native Rust LaTeX document extractor with metadata and table support"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[async_trait]
impl DocumentExtractor for LatexExtractor {
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
        let latex_str = String::from_utf8_lossy(content).to_string();
        let (text, metadata, tables) = Self::extract_from_latex(&latex_str);

        Ok(ExtractionResult {
            content: text,
            mime_type: mime_type.to_string(),
            metadata,
            tables,
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-latex", "text/x-tex", "text/plain"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

/// LaTeX parser
struct LatexParser<'a> {
    source: &'a str,
    metadata: Metadata,
    tables: Vec<Table>,
    output: String,
}

impl<'a> LatexParser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            metadata: Metadata::default(),
            tables: Vec::new(),
            output: String::new(),
        }
    }

    fn parse(&mut self) -> (String, Metadata, Vec<Table>) {
        let lines: Vec<&str> = self.source.lines().collect();
        let mut in_document = false;
        let mut skip_until_end = None::<String>;
        let mut i = 0;

        // Check if this is plain TeX (contains \bye) or LaTeX (contains \begin{document})
        let is_plain_tex = self.source.contains("\\bye") && !self.source.contains("\\begin{document}");
        if is_plain_tex {
            in_document = true; // For plain TeX, treat everything as document content
        }

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            // Handle end of skipped environment
            if let Some(ref env) = skip_until_end {
                if trimmed.contains(&format!("\\end{{{}}}", env)) {
                    skip_until_end = None;
                }
                i += 1;
                continue;
            }

            // Plain TeX: stop at \bye
            if is_plain_tex && trimmed.contains("\\bye") {
                break;
            }

            // Extract metadata from preamble (LaTeX only)
            if !in_document && !is_plain_tex {
                self.extract_metadata_from_line(trimmed);
            }

            // Check for document start (LaTeX)
            if !is_plain_tex && trimmed.contains("\\begin{document}") {
                in_document = true;

                // Check if \begin{document} and \end{document} are on the same line
                if trimmed.contains("\\end{document}") {
                    // Extract content between them
                    let Some(begin_pos) = trimmed.find("\\begin{document}") else {
                        break;
                    };
                    let Some(end_pos) = trimmed.find("\\end{document}") else {
                        break;
                    };
                    let content_between = trimmed[begin_pos + 16..end_pos].trim();
                    if !content_between.is_empty() {
                        // Handle sections
                        if content_between.starts_with("\\section{") {
                            if let Some(title) = self.extract_braced(content_between, "section") {
                                self.output.push_str(&format!("\n# {}\n\n", title));
                            }
                        } else {
                            let processed = self.process_line(content_between);
                            if !processed.is_empty() {
                                self.output.push_str(&processed);
                                self.output.push('\n');
                            }
                        }
                    }
                    break;
                }

                i += 1;
                continue;
            }

            // Check for document end (LaTeX)
            if !is_plain_tex && trimmed.contains("\\end{document}") {
                break;
            }

            // Process content inside document
            if in_document {
                // Handle environments
                if trimmed.contains("\\begin{") {
                    let Some(env_name) = self.extract_env_name(trimmed) else {
                        // No environment name found, continue to next section
                        i += 1;
                        continue;
                    };
                    match env_name.as_str() {
                        "itemize" | "enumerate" | "description" => {
                            let (env_content, new_i) = self.collect_environment(&lines, i, &env_name);
                            self.process_list(&env_content, &env_name);
                            i = new_i;
                            continue;
                        }
                        "tabular" => {
                            let (env_content, new_i) = self.collect_environment(&lines, i, "tabular");
                            self.process_table(&env_content);
                            i = new_i;
                            continue;
                        }
                        "table" => {
                            let (env_content, new_i) = self.collect_environment(&lines, i, "table");
                            self.process_table_with_caption(&env_content);
                            i = new_i;
                            continue;
                        }
                        "equation" | "align" | "gather" | "multline" => {
                            // Math environments - wrap in $$
                            let (env_content, new_i) = self.collect_environment(&lines, i, &env_name);
                            self.output.push_str("$$\\begin{");
                            self.output.push_str(&env_name);
                            self.output.push_str("}\n");
                            self.output.push_str(&env_content);
                            self.output.push_str("\\end{");
                            self.output.push_str(&env_name);
                            self.output.push_str("}$$\n\n");
                            i = new_i;
                            continue;
                        }
                        _ => {
                            // Skip unknown environments
                            skip_until_end = Some(env_name);
                        }
                    }
                }

                // Handle sections
                if trimmed.starts_with("\\section{") {
                    if let Some(title) = self.extract_braced(trimmed, "section") {
                        self.output.push_str(&format!("\n# {}\n\n", title));
                    }
                } else if trimmed.starts_with("\\subsection{") {
                    if let Some(title) = self.extract_braced(trimmed, "subsection") {
                        self.output.push_str(&format!("## {}\n\n", title));
                    }
                } else if trimmed.starts_with("\\subsubsection{") {
                    if let Some(title) = self.extract_braced(trimmed, "subsubsection") {
                        self.output.push_str(&format!("### {}\n\n", title));
                    }
                } else if trimmed.starts_with("\\[") {
                    // Display math - collect until \]
                    let mut math_content = trimmed.to_string();
                    if !trimmed.contains("\\]") {
                        i += 1;
                        while i < lines.len() {
                            let math_line = lines[i];
                            math_content.push('\n');
                            math_content.push_str(math_line);
                            if math_line.trim().contains("\\]") {
                                break;
                            }
                            i += 1;
                        }
                    }
                    self.output.push_str(&math_content);
                    self.output.push('\n');
                } else if !trimmed.is_empty() && !trimmed.starts_with("%") {
                    // Skip comments
                    // Process any line with content (including lines starting with \textbf, etc.)
                    let processed = self.process_line(trimmed);
                    if !processed.is_empty() {
                        self.output.push_str(&processed);
                        self.output.push('\n');
                    }
                }
            }

            i += 1;
        }

        let content = self.output.trim().to_string();
        (content, self.metadata.clone(), self.tables.clone())
    }

    fn extract_metadata_from_line(&mut self, line: &str) {
        if line.starts_with("\\title{") {
            let Some(title) = self.extract_braced(line, "title") else {
                return;
            };
            self.metadata.additional.insert("title".to_string(), title.into());
        } else if line.starts_with("\\author{") {
            let Some(author) = self.extract_braced(line, "author") else {
                return;
            };
            self.metadata.additional.insert("author".to_string(), author.into());
        } else if line.starts_with("\\date{") {
            let Some(date) = self.extract_braced(line, "date") else {
                return;
            };
            self.metadata.additional.insert("date".to_string(), date.into());
        }
    }

    fn extract_env_name(&self, line: &str) -> Option<String> {
        if let Some(start) = line.find("\\begin{") {
            let after = &line[start + 7..];
            if let Some(end) = after.find('}') {
                return Some(after[..end].to_string());
            }
        }
        None
    }

    fn collect_environment(&self, lines: &[&str], start_idx: usize, env_name: &str) -> (String, usize) {
        let mut content = String::new();
        let mut i = start_idx + 1;
        let end_marker = format!("\\end{{{}}}", env_name);

        while i < lines.len() {
            let line = lines[i];
            if line.trim().contains(&end_marker) {
                return (content, i + 1);
            }
            content.push_str(line);
            content.push('\n');
            i += 1;
        }

        (content, i)
    }

    fn process_list(&mut self, content: &str, list_type: &str) {
        let lines: Vec<&str> = content.lines().collect();
        let mut item_num = 1;
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            // Handle nested lists
            if trimmed.contains("\\begin{") {
                let Some(env_name) = self.extract_env_name(trimmed) else {
                    i += 1;
                    continue;
                };
                if env_name == "itemize" || env_name == "enumerate" || env_name == "description" {
                    let (nested_content, new_i) = self.collect_environment(&lines, i, &env_name);
                    // Add indentation for nested list
                    let current_output_len = self.output.len();
                    self.process_list(&nested_content, &env_name);
                    // Indent the nested list output
                    let nested_output = self.output[current_output_len..].to_string();
                    self.output.truncate(current_output_len);
                    for nested_line in nested_output.lines() {
                        self.output.push_str("  ");
                        self.output.push_str(nested_line);
                        self.output.push('\n');
                    }
                    i = new_i;
                    continue;
                }
            }

            if trimmed.starts_with("\\item") {
                let Some(pos) = trimmed.find("\\item") else {
                    i += 1;
                    continue;
                };
                let after = trimmed[pos + 5..].trim();

                // Handle description list labels [label]
                if after.starts_with('[') {
                    let Some(bracket_end) = after.find(']') else {
                        i += 1;
                        continue;
                    };
                    let label = after[1..bracket_end].to_string();
                    let text = after[bracket_end + 1..].trim().to_string();
                    if list_type == "description" {
                        let processed_text = self.process_line(&text);
                        self.output.push_str(&format!("{}: {}\n", label, processed_text));
                        item_num += 1;
                        i += 1;
                        continue;
                    }
                }

                // Normal item (no brackets or non-description list)
                let prefix = if list_type == "enumerate" {
                    format!("{}. ", item_num)
                } else {
                    "- ".to_string()
                };
                self.output.push_str(&prefix);

                // Process the item text
                let item_text = self.process_line(after);
                self.output.push_str(item_text.trim());
                self.output.push('\n');
                item_num += 1;
            }

            i += 1;
        }
        self.output.push('\n');
    }

    fn process_table(&mut self, content: &str) {
        let lines: Vec<&str> = content.lines().collect();
        let mut rows: Vec<Vec<String>> = Vec::new();

        for line in lines {
            let trimmed = line.trim();
            if trimmed.starts_with("\\hline") || trimmed.is_empty() || trimmed.contains("\\begin{tabular}") {
                continue;
            }

            // Split by & and \\ for table cells
            let row_str = trimmed.replace("\\\\", "");
            let cells: Vec<String> = row_str
                .split('&')
                .map(|s| self.clean_text(s.trim()))
                .filter(|s| !s.is_empty())
                .collect();

            if !cells.is_empty() {
                rows.push(cells);
            }
        }

        if !rows.is_empty() {
            let mut markdown = String::new();
            for (i, row) in rows.iter().enumerate() {
                markdown.push('|');
                for cell in row {
                    markdown.push_str(&format!(" {} |", cell));
                }
                markdown.push('\n');

                // Add header separator after first row
                if i == 0 && rows.len() > 1 {
                    markdown.push('|');
                    for _ in row {
                        markdown.push_str(" --- |");
                    }
                    markdown.push('\n');
                }
            }

            self.output.push_str(&markdown);

            let table = Table {
                cells: rows,
                markdown: markdown.clone(),
                page_number: 1,
            };
            self.tables.push(table);
        }
    }

    fn process_table_with_caption(&mut self, content: &str) {
        // Extract caption if present
        if content.contains("\\caption{") {
            let Some(caption) = self.extract_braced_from_content(content, "caption") else {
                return;
            };
            self.output.push_str(&caption);
            self.output.push('\n');
        }

        // Extract tabular if present
        if content.contains("\\begin{tabular}") {
            let Some(start) = content.find("\\begin{tabular}") else {
                return;
            };
            let Some(end) = content.find("\\end{tabular}") else {
                return;
            };
            let tabular_content = &content[start..end + 13];
            self.process_table(tabular_content);
        }
    }

    fn process_line(&self, line: &str) -> String {
        let mut result = String::new();
        let mut chars = line.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                // Try to read command
                let mut cmd = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphabetic() {
                        cmd.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                // Handle known formatting commands
                match cmd.as_str() {
                    "textbf" => {
                        if let Some(content) = self.read_braced_from_chars(&mut chars) {
                            // Recursively process content to handle nested formatting
                            let processed = self.process_line(&content);
                            result.push_str(&processed);
                        }
                    }
                    "textit" | "emph" => {
                        if let Some(content) = self.read_braced_from_chars(&mut chars) {
                            // Recursively process content to handle nested formatting
                            let processed = self.process_line(&content);
                            result.push_str(&processed);
                        }
                    }
                    "texttt" => {
                        if let Some(content) = self.read_braced_from_chars(&mut chars) {
                            result.push_str(&content);
                        }
                    }
                    "underline" => {
                        if let Some(content) = self.read_braced_from_chars(&mut chars) {
                            // Recursively process content to handle nested formatting
                            let processed = self.process_line(&content);
                            result.push_str(&processed);
                        }
                    }
                    "font" => {
                        // Skip font declarations like \font\r="[lmroman12-regular]"
                        // Read until end of line or next command
                        while let Some(&c) = chars.peek() {
                            if c == '\\' {
                                break;
                            }
                            chars.next();
                        }
                    }
                    "usepackage" => {
                        // Skip package declarations
                        self.read_braced_from_chars(&mut chars);
                    }
                    _ => {
                        // For unknown commands, try to extract content if braced
                        if let Some(content) = self.read_braced_from_chars(&mut chars) {
                            let processed = self.process_line(&content);
                            result.push_str(&processed);
                        } else if cmd.len() == 1 {
                            // Single-letter commands like \r - just consume the space and continue
                            // The text after will be processed normally
                        } else {
                            // Unknown command without braces, skip it
                        }
                    }
                }
            } else if ch == '$' {
                // Preserve math
                result.push(ch);
                while let Some(&c) = chars.peek() {
                    result.push(chars.next().unwrap());
                    if c == '$' {
                        break;
                    }
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    fn read_braced_from_chars(&self, chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<String> {
        // Skip whitespace
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        // Check for opening brace
        if chars.peek() != Some(&'{') {
            return None;
        }
        chars.next(); // consume {

        let mut content = String::new();
        let mut depth = 1;

        for c in chars.by_ref() {
            match c {
                '{' => {
                    depth += 1;
                    content.push(c);
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(content);
                    }
                    content.push(c);
                }
                _ => content.push(c),
            }
        }

        Some(content)
    }

    fn extract_braced(&self, text: &str, command: &str) -> Option<String> {
        let pattern = format!("\\{}{{", command);
        if let Some(start) = text.find(&pattern) {
            let after = &text[start + pattern.len()..];
            let mut depth = 1;
            let mut content = String::new();

            for ch in after.chars() {
                match ch {
                    '{' => {
                        depth += 1;
                        content.push(ch);
                    }
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(self.clean_text(&content));
                        }
                        content.push(ch);
                    }
                    _ => content.push(ch),
                }
            }
        }
        None
    }

    fn extract_braced_from_content(&self, text: &str, command: &str) -> Option<String> {
        self.extract_braced(text, command)
    }

    fn clean_text(&self, text: &str) -> String {
        text.to_string()
            .replace("\\\\", "\n")
            .replace("\\&", "&")
            .replace("\\#", "#")
            .replace("\\_", "_")
            .replace("\\{", "{")
            .replace("\\}", "}")
            .replace("\\%", "%")
            .trim()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_title_extraction() {
        let latex = r#"\title{Hello World}"#;
        let (_, metadata, _) = LatexExtractor::extract_from_latex(latex);
        assert_eq!(
            metadata.additional.get("title").and_then(|v| v.as_str()),
            Some("Hello World")
        );
    }

    #[test]
    fn test_author_extraction() {
        let latex = r#"\author{John Doe}"#;
        let (_, metadata, _) = LatexExtractor::extract_from_latex(latex);
        assert!(metadata.additional.get("author").is_some());
    }

    #[test]
    fn test_section_extraction() {
        let latex = r#"\begin{document}\section{Introduction}\end{document}"#;
        let (content, _, _) = LatexExtractor::extract_from_latex(latex);
        assert!(content.contains("Introduction"));
    }
}
