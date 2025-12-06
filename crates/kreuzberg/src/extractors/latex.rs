//! Native Rust LaTeX text extractor.
//!
//! This extractor provides comprehensive LaTeX document parsing and text extraction,
//! replacing Pandoc for LaTeX files (.tex, .latex).
//!
//! Features:
//! - Metadata extraction: title, author, date from \title{}, \author{}, \date{}
//! - Section hierarchy: \section{}, \subsection{}, \subsubsection{}, etc.
//! - Inline formatting: \emph{}, \textbf{}, \textit{}, \texttt{}, \sout{}
//! - Lists: itemize, enumerate, description environments
//! - Tables: tabular environment parsing
//! - Code blocks: verbatim, obeylines, Verbatim environments
//! - Quotes: quote, quotation environments
//! - Links: \href{url}{text}, \url{}
//! - Inline code: \verb!code!
//! - Math: inline ($...$) and display ($$...$$) math preservation
//! - Citations: \cite{} extraction
//! - Footnotes: \footnote{} content extraction
//! - Special characters and Unicode support
//! - Images: \includegraphics{} references
//!
//! Requires the `office` feature.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata, Table};
use async_trait::async_trait;
use regex::Regex;

/// LaTeX document extractor
pub struct LatexExtractor;

impl LatexExtractor {
    /// Create a new LaTeX extractor.
    pub fn new() -> Self {
        Self
    }

    /// Parse LaTeX content and extract text.
    fn extract_from_latex(content: &str) -> (String, Metadata, Vec<Table>) {
        let mut extractor = LatexParser::new(content);
        let text = extractor.parse();
        let metadata = extractor.metadata;
        let tables = extractor.tables;

        (text, metadata, tables)
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

/// Internal LaTeX parser
struct LatexParser {
    content: String,
    metadata: Metadata,
    tables: Vec<Table>,
}

impl LatexParser {
    fn new(content: &str) -> Self {
        Self {
            content: content.to_string(),
            metadata: Metadata::default(),
            tables: Vec::new(),
        }
    }

    fn parse(&mut self) -> String {
        // Extract metadata from preamble
        self.extract_metadata();

        // Process content and extract text
        self.extract_content()
    }

    fn extract_metadata(&mut self) {
        // Extract title
        if let Some(title) = self.extract_command_content("title") {
            self.metadata
                .additional
                .insert("title".to_string(), title.clone().into());
        }

        // Extract authors (handle \and separator)
        if let Some(author_str) = self.extract_command_content("author") {
            let authors: Vec<String> = author_str.split(" \\and ").map(|a| a.trim().to_string()).collect();

            // Store first author if available
            if let Some(first_author) = authors.first() {
                self.metadata
                    .additional
                    .insert("author".to_string(), first_author.clone().into());
            }

            self.metadata.additional.insert("authors".to_string(), authors.into());
        }

        // Extract date
        if let Some(date) = self.extract_command_content("date") {
            self.metadata.date = Some(date);
        }

        // Extract document class
        if let Some(docclass) = self.extract_command_with_args("documentclass") {
            self.metadata
                .additional
                .insert("documentclass".to_string(), docclass.into());
        }
    }

    fn extract_command_content(&self, command: &str) -> Option<String> {
        // In regex, \\ matches a literal backslash
        // In Rust raw string, r"\\" is the two characters \ and \
        let pattern = format!(r"\\{}\{{([^}}]*)\}}", regex::escape(command));
        Regex::new(&pattern)
            .ok()
            .and_then(|re| re.captures(&self.content))
            .map(|caps| caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default())
    }

    fn extract_command_with_args(&self, command: &str) -> Option<String> {
        // Match \command[arg1]{arg2} or \command{arg}
        let pattern = format!(r"\\{}(?:\[([^\]]*)\])?\{{([^}}]*)\}}", regex::escape(command));
        Regex::new(&pattern)
            .ok()
            .and_then(|re| re.captures(&self.content))
            .and_then(|caps| caps.get(1).or_else(|| caps.get(2)).map(|m| m.as_str().to_string()))
    }

    fn extract_content(&mut self) -> String {
        let mut result = String::new();
        let mut chars = self.content.chars().peekable();
        let mut in_document = false;
        let mut in_preamble = true;

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                // Look ahead for command
                if chars.peek().is_some() {
                    if let Some(&'{') = chars.peek() {
                        chars.next(); // consume {
                        let (content, _) = self.read_braced_content(&mut chars);
                        result.push_str(&self.process_braced_content(&content));
                        continue;
                    }

                    // Check for environment starts
                    let cmd = self.read_command_name(&mut chars);

                    match cmd.as_str() {
                        // Compiler requires this pattern
                        "begin" => {
                            // Read environment name
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let mut env_name = String::new();
                                while let Some(&c) = chars.peek() {
                                    if c == '}' {
                                        chars.next();
                                        break;
                                    }
                                    env_name.push(chars.next().unwrap());
                                }

                                match env_name.as_str() {
                                    "document" => {
                                        in_document = true;
                                        in_preamble = false;
                                    }
                                    "tabular" => {
                                        // Extract table
                                        let table_content = self.extract_environment(&mut chars, "tabular");
                                        if let Some((text, table)) = self.parse_tabular(&table_content) {
                                            result.push_str(&text);
                                            result.push('\n');
                                            self.tables.push(table);
                                        }
                                    }
                                    "itemize" | "enumerate" | "description" => {
                                        let list_content = self.extract_environment(&mut chars, &env_name);
                                        let list_text = self.parse_list(&list_content, &env_name);
                                        result.push_str(&list_text);
                                        result.push('\n');
                                    }
                                    "quote" | "quotation" => {
                                        let quote_content = self.extract_environment(&mut chars, &env_name);
                                        result.push_str(&self.process_inline_content(&quote_content));
                                        result.push('\n');
                                    }
                                    "verbatim" | "Verbatim" | "obeylines" => {
                                        let code_content = self.extract_environment(&mut chars, &env_name);
                                        result.push_str(&code_content);
                                        result.push('\n');
                                    }
                                    _ => {
                                        let env_content = self.extract_environment(&mut chars, &env_name);
                                        result.push_str(&self.process_inline_content(&env_content));
                                        result.push('\n');
                                    }
                                }
                            }
                        }
                        "end" => {
                            // Consume the environment name
                            if let Some('{') = chars.peek() {
                                chars.next();
                                while let Some(&c) = chars.peek() {
                                    if c == '}' {
                                        chars.next();
                                        break;
                                    }
                                    chars.next();
                                }
                            }
                        }
                        "title" | "author" | "date" | "maketitle" | "usepackage" | "documentclass" | "newcommand"
                        | "renewcommand" | "chapter" | "appendix" => {
                            // Skip these commands but handle their arguments if present
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (_content, _) = self.read_braced_content(&mut chars);
                            }
                            // Also skip optional arguments
                            if let Some('[') = chars.peek() {
                                chars.next();
                                while let Some(&c) = chars.peek() {
                                    if c == ']' {
                                        chars.next();
                                        break;
                                    }
                                    chars.next();
                                }
                            }
                        }
                        "section" => {
                            // Skip optional argument if present
                            if let Some('[') = chars.peek() {
                                chars.next();
                                while let Some(&c) = chars.peek() {
                                    if c == ']' {
                                        chars.next();
                                        break;
                                    }
                                    chars.next();
                                }
                            }
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (section_title, _) = self.read_braced_content(&mut chars);
                                let processed = self.process_inline_content(&section_title);
                                result.push_str(&format!("\n# {}\n\n", processed));
                            }
                        }
                        "subsection" => {
                            if let Some('[') = chars.peek() {
                                chars.next();
                                while let Some(&c) = chars.peek() {
                                    if c == ']' {
                                        chars.next();
                                        break;
                                    }
                                    chars.next();
                                }
                            }
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (subsection_title, _) = self.read_braced_content(&mut chars);
                                let processed = self.process_inline_content(&subsection_title);
                                result.push_str(&format!("## {}\n\n", processed));
                            }
                        }
                        "subsubsection" => {
                            if let Some('[') = chars.peek() {
                                chars.next();
                                while let Some(&c) = chars.peek() {
                                    if c == ']' {
                                        chars.next();
                                        break;
                                    }
                                    chars.next();
                                }
                            }
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (subsubsection_title, _) = self.read_braced_content(&mut chars);
                                let processed = self.process_inline_content(&subsubsection_title);
                                result.push_str(&format!("### {}\n\n", processed));
                            }
                        }
                        "paragraph" => {
                            if let Some('[') = chars.peek() {
                                chars.next();
                                while let Some(&c) = chars.peek() {
                                    if c == ']' {
                                        chars.next();
                                        break;
                                    }
                                    chars.next();
                                }
                            }
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (para_title, _) = self.read_braced_content(&mut chars);
                                let processed = self.process_inline_content(&para_title);
                                result.push_str(&format!("#### {}\n\n", processed));
                            }
                        }
                        "subparagraph" => {
                            if let Some('[') = chars.peek() {
                                chars.next();
                                while let Some(&c) = chars.peek() {
                                    if c == ']' {
                                        chars.next();
                                        break;
                                    }
                                    chars.next();
                                }
                            }
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (subpara_title, _) = self.read_braced_content(&mut chars);
                                let processed = self.process_inline_content(&subpara_title);
                                result.push_str(&format!("##### {}\n\n", processed));
                            }
                        }
                        "footnote" => {
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (footnote_content, _) = self.read_braced_content(&mut chars);
                                let processed = self.process_inline_content(&footnote_content);
                                result.push_str(&format!(" [^{}]", result.len() % 1000));
                                result.push_str(&format!("\n\n[^{}]: {}\n\n", result.len() % 1000, processed));
                            }
                        }
                        "href" => {
                            // \href{url}{text}
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (url, _) = self.read_braced_content(&mut chars);
                                if let Some('{') = chars.peek() {
                                    chars.next();
                                    let (link_text, _) = self.read_braced_content(&mut chars);
                                    let processed_text = self.process_inline_content(&link_text);
                                    result.push_str(&format!("[{}]({})", processed_text, url));
                                }
                            }
                        }
                        "url" => {
                            // \url{url}
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (url, _) = self.read_braced_content(&mut chars);
                                result.push_str(&url);
                            }
                        }
                        "includegraphics" => {
                            // Handle optional parameters [width=...] and required {filename}
                            if let Some('[') = chars.peek() {
                                chars.next();
                                while let Some(&c) = chars.peek() {
                                    if c == ']' {
                                        chars.next();
                                        break;
                                    }
                                    chars.next();
                                }
                            }
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (filename, _) = self.read_braced_content(&mut chars);
                                result.push_str(&format!("[image: {}]", filename));
                            }
                        }
                        "cite" => {
                            // \cite[page]{key} or \cite{key}
                            if let Some('[') = chars.peek() {
                                chars.next();
                                let mut pages = String::new();
                                while let Some(&c) = chars.peek() {
                                    if c == ']' {
                                        chars.next();
                                        break;
                                    }
                                    pages.push(chars.next().unwrap());
                                }
                                if let Some('{') = chars.peek() {
                                    chars.next();
                                    let (key, _) = self.read_braced_content(&mut chars);
                                    if pages.is_empty() {
                                        result.push_str(&format!("[{}]", key));
                                    } else {
                                        result.push_str(&format!("[{}:{}]", key, pages));
                                    }
                                }
                            } else if let Some('{') = chars.peek() {
                                chars.next();
                                let (key, _) = self.read_braced_content(&mut chars);
                                result.push_str(&format!("[{}]", key));
                            }
                        }
                        "emph" | "textit" => {
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (content, _) = self.read_braced_content(&mut chars);
                                let processed = self.process_inline_content(&content);
                                result.push_str(&format!("*{}*", processed));
                            }
                        }
                        "textbf" | "textbold" => {
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (content, _) = self.read_braced_content(&mut chars);
                                let processed = self.process_inline_content(&content);
                                result.push_str(&format!("**{}**", processed));
                            }
                        }
                        "texttt" | "ttfamily" | "textmonospace" => {
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (content, _) = self.read_braced_content(&mut chars);
                                result.push_str(&format!("`{}`", content));
                            }
                        }
                        "sout" => {
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (content, _) = self.read_braced_content(&mut chars);
                                let processed = self.process_inline_content(&content);
                                result.push_str(&format!("~~{}~~", processed));
                            }
                        }
                        "textsuperscript" => {
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (content, _) = self.read_braced_content(&mut chars);
                                result.push_str(&format!("^{}", content));
                            }
                        }
                        "textsubscript" => {
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (content, _) = self.read_braced_content(&mut chars);
                                result.push_str(&format!("_{}", content));
                            }
                        }
                        "verb" => {
                            // \verb!code! - read until next delimiter
                            if let Some(&delim) = chars.peek()
                                && !delim.is_alphanumeric()
                                && delim != '{'
                                && delim != '['
                            {
                                chars.next();
                                let mut code = String::new();
                                while let Some(&c) = chars.peek() {
                                    if c == delim {
                                        chars.next();
                                        break;
                                    }
                                    code.push(chars.next().unwrap());
                                }
                                result.push_str(&format!("`{}`", code));
                            }
                        }
                        "ldots" | "textellipsis" => {
                            result.push_str("...");
                            // Skip {} if present
                            if let Some('{') = chars.peek() {
                                chars.next();
                                if let Some('}') = chars.peek() {
                                    chars.next();
                                }
                            }
                        }
                        "textless" => {
                            result.push('<');
                            if let Some('{') = chars.peek() {
                                chars.next();
                                if let Some('}') = chars.peek() {
                                    chars.next();
                                }
                            }
                        }
                        "textgreater" => {
                            result.push('>');
                            if let Some('{') = chars.peek() {
                                chars.next();
                                if let Some('}') = chars.peek() {
                                    chars.next();
                                }
                            }
                        }
                        "textbackslash" => {
                            result.push('\\');
                            if let Some('{') = chars.peek() {
                                chars.next();
                                if let Some('}') = chars.peek() {
                                    chars.next();
                                }
                            }
                        }
                        // Escaped special characters
                        "&" | "#" | "_" | "{" | "}" | "%" => match cmd.as_str() {
                            "&" => result.push('&'),
                            "#" => result.push('#'),
                            "_" => result.push('_'),
                            "{" => result.push('{'),
                            "}" => result.push('}'),
                            "%" => result.push('%'),
                            _ => {}
                        },
                        "ensuremath" => {
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (math, _) = self.read_braced_content(&mut chars);
                                result.push_str(&format!("${}$", math));
                            }
                        }
                        // Line break
                        "\\" => {
                            result.push('\n');
                        }
                        // Horizontal rule
                        "rule" => {
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (_, _) = self.read_braced_content(&mut chars);
                                if let Some('{') = chars.peek() {
                                    chars.next();
                                    let (_, _) = self.read_braced_content(&mut chars);
                                }
                            }
                            result.push_str("---\n");
                        }
                        "doublespacing" | "setcounter" | "enumi" | "enumii" | "enumiii" | "enumiv"
                        | "VerbatimFootnotes" => {
                            // Skip these formatting commands
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (_content, _) = self.read_braced_content(&mut chars);
                            }
                        }
                        _ => {
                            // Unknown command - check if it takes arguments
                            if let Some('{') = chars.peek() {
                                chars.next();
                                let (content, _) = self.read_braced_content(&mut chars);
                                // For most unknown commands, process their content
                                result.push_str(&self.process_inline_content(&content));
                            } else if let Some('[') = chars.peek() {
                                chars.next();
                                while let Some(&c) = chars.peek() {
                                    if c == ']' {
                                        chars.next();
                                        break;
                                    }
                                    chars.next();
                                }
                            }
                        }
                    }
                } else {
                    result.push(ch);
                }
            } else if ch == '$' {
                // Math mode - check for display math $$
                if let Some(&'$') = chars.peek() {
                    chars.next(); // consume second $
                    result.push_str("$$");
                    let mut math = String::new();
                    let mut escaped = false;
                    while let Some(&c) = chars.peek() {
                        if escaped {
                            math.push(c);
                            chars.next();
                            escaped = false;
                        } else if c == '\\' {
                            math.push(c);
                            chars.next();
                            escaped = true;
                        } else if c == '$' {
                            chars.next();
                            if let Some(&'$') = chars.peek() {
                                chars.next();
                                break;
                            } else {
                                math.push(c);
                            }
                        } else {
                            math.push(c);
                            chars.next();
                        }
                    }
                    result.push_str(&math);
                    result.push_str("$$");
                } else {
                    // Inline math
                    result.push('$');
                    let mut math = String::new();
                    let mut escaped = false;
                    while let Some(&c) = chars.peek() {
                        if escaped {
                            math.push(c);
                            chars.next();
                            escaped = false;
                        } else if c == '\\' {
                            math.push(c);
                            chars.next();
                            escaped = true;
                        } else if c == '$' {
                            chars.next();
                            break;
                        } else {
                            math.push(c);
                            chars.next();
                        }
                    }
                    result.push_str(&math);
                    result.push('$');
                }
            } else if in_document || !in_preamble {
                // Only include non-preamble content
                result.push(ch);
            }
        }

        // Clean up multiple newlines
        while result.contains("\n\n\n") {
            result = result.replace("\n\n\n", "\n\n");
        }

        result.trim().to_string()
    }

    fn read_command_name(&self, chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
        let mut cmd = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_alphabetic() || c == '*' {
                cmd.push(chars.next().unwrap());
            } else if (cmd.is_empty() || cmd == "\\") && !c.is_alphabetic() {
                // Single character command like \{, \$, etc.
                cmd.push(chars.next().unwrap());
                break;
            } else {
                break;
            }
        }
        cmd
    }

    fn read_braced_content(&self, chars: &mut std::iter::Peekable<std::str::Chars>) -> (String, usize) {
        let mut content = String::new();
        let mut depth = 1;

        while let Some(&c) = chars.peek() {
            if c == '\\' {
                content.push(chars.next().unwrap());
                if let Some(&_next) = chars.peek() {
                    content.push(chars.next().unwrap());
                }
            } else if c == '{' {
                depth += 1;
                content.push(chars.next().unwrap());
            } else if c == '}' {
                chars.next();
                depth -= 1;
                if depth == 0 {
                    break;
                }
                content.push('}');
            } else {
                content.push(chars.next().unwrap());
            }
        }

        (content, chars.count())
    }

    fn process_braced_content(&self, content: &str) -> String {
        self.process_inline_content(content)
    }

    fn process_inline_content(&self, content: &str) -> String {
        let mut result = String::new();
        let mut chars = content.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(&_next) = chars.peek() {
                    let cmd = self.read_command_name(&mut chars);
                    match cmd.as_str() {
                        "&" => result.push('&'),
                        "#" => result.push('#'),
                        "_" => result.push('_'),
                        "{" => result.push('{'),
                        "}" => result.push('}'),
                        "%" => result.push('%'),
                        "textgreater" | "textless" | "textbackslash" => {
                            // These would normally require {} but process them here
                            match cmd.as_str() {
                                "textgreater" => result.push('>'),
                                "textless" => result.push('<'),
                                "textbackslash" => result.push('\\'),
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                } else {
                    result.push(ch);
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    fn extract_environment(&self, chars: &mut std::iter::Peekable<std::str::Chars>, env_name: &str) -> String {
        let mut content = String::new();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                // Check if this is the end of the environment
                let mut check_str = String::from("\\");

                while let Some(&c) = chars.peek() {
                    if c.is_alphabetic() || c == '{' || c == '*' {
                        check_str.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                if check_str.starts_with(&format!("\\end{{{}", env_name)) || check_str == "\\end" {
                    // Found end of environment
                    // consume up to closing }
                    let mut depth = 0;
                    let mut found_brace = false;
                    for c in chars.by_ref() {
                        if c == '{' {
                            depth += 1;
                            found_brace = true;
                        } else if c == '}' {
                            depth -= 1;
                            if depth == 0 && found_brace {
                                break;
                            }
                        }
                    }
                    break;
                } else {
                    content.push_str(&check_str);
                }
            } else {
                content.push(ch);
            }
        }

        content
    }

    fn parse_list(&self, content: &str, list_type: &str) -> String {
        let mut result = String::new();
        let mut in_item = false;
        let mut current_item = String::new();
        let mut item_count = 1;

        let mut chars = content.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if chars.peek().is_some() {
                    if let Some(&'i') = chars.peek() {
                        // Could be \item
                        let mut cmd = String::from("\\");
                        while let Some(&c) = chars.peek() {
                            if c.is_alphabetic() {
                                cmd.push(chars.next().unwrap());
                            } else {
                                break;
                            }
                        }

                        if cmd == "\\item" {
                            if in_item && !current_item.is_empty() {
                                let prefix = match list_type {
                                    "enumerate" => format!("{}. ", item_count),
                                    "description" => {
                                        // Extract label from [...]
                                        String::new()
                                    }
                                    _ => "- ".to_string(),
                                };
                                result.push_str(&prefix);
                                result.push_str(current_item.trim());
                                result.push('\n');
                                item_count += 1;
                            }

                            in_item = true;
                            current_item.clear();

                            // Check for optional parameter [label]
                            if let Some('[') = chars.peek() {
                                chars.next();
                                let mut label = String::new();
                                while let Some(&c) = chars.peek() {
                                    if c == ']' {
                                        chars.next();
                                        break;
                                    }
                                    label.push(chars.next().unwrap());
                                }
                                if list_type == "description" {
                                    result.push_str(&format!("{}: ", label));
                                }
                            }
                        } else {
                            current_item.push_str(&cmd);
                        }
                    } else {
                        current_item.push(ch);
                    }
                } else {
                    current_item.push(ch);
                }
            } else {
                current_item.push(ch);
            }
        }

        if in_item && !current_item.is_empty() {
            let prefix = match list_type {
                "enumerate" => format!("{}. ", item_count),
                _ => "- ".to_string(),
            };
            result.push_str(&prefix);
            result.push_str(current_item.trim());
            result.push('\n');
        }

        result
    }

    fn parse_tabular(&self, content: &str) -> Option<(String, Table)> {
        // Parse LaTeX tabular environment
        let mut cells = Vec::new();
        let mut current_row = Vec::new();
        let mut current_cell = String::new();

        let mut chars = content.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '&' {
                // Cell separator
                let processed = self.process_inline_content(current_cell.trim());
                current_row.push(processed);
                current_cell.clear();
            } else if ch == '\\' {
                if let Some(&next_c) = chars.peek() {
                    if next_c == '\\' {
                        chars.next();
                        // Row separator
                        let processed = self.process_inline_content(current_cell.trim());
                        current_row.push(processed);

                        if !current_row.is_empty() {
                            cells.push(current_row.clone());
                        }

                        current_row.clear();
                        current_cell.clear();

                        // Skip any \hline commands
                        while let Some(&c) = chars.peek() {
                            if c == 'h' {
                                let mut hline_check = String::new();
                                while let Some(&h) = chars.peek() {
                                    if h.is_alphabetic() {
                                        hline_check.push(chars.next().unwrap());
                                    } else {
                                        break;
                                    }
                                }
                                if hline_check == "hline" {
                                    continue;
                                } else {
                                    break;
                                }
                            } else if c.is_whitespace() {
                                chars.next();
                            } else {
                                break;
                            }
                        }
                    } else {
                        current_cell.push(ch);
                    }
                } else {
                    current_cell.push(ch);
                }
            } else if !ch.is_whitespace() || !current_cell.is_empty() {
                current_cell.push(ch);
            }
        }

        // Don't forget last cell and row
        if !current_cell.is_empty() {
            let processed = self.process_inline_content(current_cell.trim());
            current_row.push(processed);
        }

        if !current_row.is_empty() {
            cells.push(current_row);
        }

        if !cells.is_empty() {
            // Build markdown representation
            let mut markdown = String::new();
            for (i, row) in cells.iter().enumerate() {
                markdown.push('|');
                for cell in row {
                    markdown.push_str(&format!(" {} |", cell));
                }
                markdown.push('\n');
                // Add header separator after first row
                if i == 0 && cells.len() > 1 {
                    markdown.push_str(&"|".repeat(row.len() + 1));
                    markdown.push('\n');
                }
            }

            let table = Table {
                cells,
                markdown: markdown.clone(),
                page_number: 1,
            };

            Some((markdown, table))
        } else {
            None
        }
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
        let latex = r#"\section{Introduction}"#;
        let (content, _, _) = LatexExtractor::extract_from_latex(latex);
        assert!(content.contains("Introduction"));
    }
}
