//! AsciiDoc extractor.
//!
//! Parses the structural subset of AsciiDoc that carries document meaning, rather than
//! flattening the source into undifferentiated paragraphs the way the plain-text
//! extractor used to (#228):
//!
//! - **Document header**: `= Title`, the implicit author line, and `:attribute: value`
//!   entries (attributes are also substituted into body text as `{name}` references).
//! - **Sections**: `== `, `=== ` … `====== ` become real headings at the matching level.
//! - **Lists**: `*`/`-` unordered and `.` ordered items, including nesting depth.
//! - **Delimited blocks**: `----` listing blocks (with the language from a preceding
//!   `[source,lang]` attribute line), `....` literal blocks, `____` quote blocks and
//!   `====` example blocks.
//! - **Admonitions**: both the inline `NOTE: text` form and the `[NOTE]` + `====` block
//!   form.
//! - **Tables**: `|===` delimited tables, including multi-line rows, become structured
//!   tables rather than pipe-noise paragraphs.
//! - **Inline markup**: constrained `*strong*`, `_emphasis_`, `` `mono` `` and the
//!   `link:target[text]` / `https://url[text]` macros become text annotations.
//!
//! `TextMetadata.headers`, `.links` and `.code_blocks` are populated from the parse.
//!
//! Malformed input degrades: an unterminated delimited block or table is closed at
//! end-of-input and reported as a [`ProcessingWarning`] instead of failing extraction.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::transform::normalize_line_endings;
use crate::extractors::security::SecurityBudget;
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::types::ProcessingWarning;
use crate::types::document_structure::{AnnotationKind, TextAnnotation};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::metadata::Metadata;
use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;

mod syntax;
use syntax::{
    MathNotation, can_close_macro, count_columns, is_delimiter, list_marker, macro_delimiters, parse_attribute_entry,
    parse_block_attributes, parse_block_title, parse_constrained_span, parse_cross_reference, parse_link_macro,
    parse_math_macro, parse_passthrough, section_level, single_attribute, split_inline_admonition, split_table_row,
};

/// Inline admonition labels recognised in the `LABEL: text` form.
const ADMONITION_LABELS: [&str; 5] = ["NOTE", "TIP", "IMPORTANT", "WARNING", "CAUTION"];

/// AsciiDoc document extractor.
///
/// Produces headings, lists, code blocks, tables, admonitions and quotes from AsciiDoc
/// source, together with `TextMetadata` carrying the document's headers, links and code
/// blocks.
#[cfg_attr(alef, alef(skip))]
pub struct AsciiDocExtractor;

impl AsciiDocExtractor {
    /// Create a new AsciiDoc extractor.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for AsciiDocExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Everything a single AsciiDoc parse produces.
struct ParseOutput {
    document: InternalDocument,
    headers: Vec<String>,
    links: Vec<(String, String)>,
    code_blocks: Vec<(String, String)>,
}

/// Line-oriented AsciiDoc parser.
///
/// Held as a struct so each block form gets its own short handler instead of one
/// monolithic loop.
struct AsciiDocParser<'a> {
    lines: Vec<&'a str>,
    index: usize,
    builder: InternalDocumentBuilder,
    /// Open list nesting: one entry per level, `true` when that level is ordered.
    list_stack: Vec<bool>,
    /// Document attributes from `:name: value` entries, used for `{name}` substitution.
    attributes: AHashMap<String, String>,
    /// Comma-separated values of the most recent `[...]` block attribute line.
    pending_attrs: Vec<String>,
    /// Text of the most recent `.Block title` line.
    pending_title: Option<String>,
    headers: Vec<String>,
    links: Vec<(String, String)>,
    code_blocks: Vec<(String, String)>,
    warnings: Vec<ProcessingWarning>,
    title: Option<String>,
    authors: Option<Vec<String>>,
}

impl<'a> AsciiDocParser<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            lines: text.lines().collect(),
            index: 0,
            builder: InternalDocumentBuilder::new("asciidoc"),
            list_stack: Vec::new(),
            attributes: AHashMap::new(),
            pending_attrs: Vec::new(),
            pending_title: None,
            headers: Vec::new(),
            links: Vec::new(),
            code_blocks: Vec::new(),
            warnings: Vec::new(),
            title: None,
            authors: None,
        }
    }

    fn parse(mut self) -> ParseOutput {
        self.parse_document_header();
        while self.index < self.lines.len() {
            self.parse_block();
        }
        self.close_lists();

        let mut metadata = Metadata {
            title: self.title.clone(),
            authors: self.authors.clone(),
            ..Default::default()
        };
        for (key, value) in &self.attributes {
            metadata
                .additional
                .insert(Cow::Owned(format!("asciidoc_{key}")), serde_json::json!(value));
        }
        self.builder.set_metadata(metadata);
        for warning in std::mem::take(&mut self.warnings) {
            self.builder.add_warning(warning);
        }

        ParseOutput {
            document: self.builder.build(),
            headers: self.headers,
            links: self.links,
            code_blocks: self.code_blocks,
        }
    }

    /// Consume the document header: `= Title`, the implicit author line, attribute entries.
    fn parse_document_header(&mut self) {
        self.skip_blank_and_comment_lines();
        let Some(first) = self.lines.get(self.index).copied() else {
            return;
        };
        let Some(rest) = first.strip_prefix("= ") else {
            return;
        };
        let (text, annotations) = self.parse_inline(rest.trim());
        let idx = self.builder.push_title(&text, None, None);
        if !annotations.is_empty() {
            self.builder.set_annotations(idx, annotations);
        }
        self.headers.push(text.clone());
        self.title = Some(text);
        self.index += 1;

        if let Some(author_line) = self.lines.get(self.index).copied() {
            let trimmed = author_line.trim();
            let is_author = !trimmed.is_empty()
                && !trimmed.starts_with(':')
                && !trimmed.starts_with("//")
                && !trimmed.starts_with('=');
            if is_author {
                let authors: Vec<String> = trimmed.split(';').map(|a| a.trim().to_string()).collect();
                self.authors = Some(authors);
                self.index += 1;
            }
        }
        self.consume_attribute_entries();
    }

    /// Consume a run of `:name: value` attribute entries and blank/comment lines.
    fn consume_attribute_entries(&mut self) {
        while let Some(line) = self.lines.get(self.index).copied() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") {
                self.index += 1;
                continue;
            }
            let Some(entry) = parse_attribute_entry(trimmed) else {
                return;
            };
            self.attributes.insert(entry.0, entry.1);
            self.index += 1;
        }
    }

    fn skip_blank_and_comment_lines(&mut self) {
        while let Some(line) = self.lines.get(self.index).copied() {
            let trimmed = line.trim();
            if trimmed.is_empty() || (trimmed.starts_with("//") && !is_delimiter(trimmed, '/')) {
                self.index += 1;
            } else {
                return;
            }
        }
    }

    /// Dispatch a single block starting at the current line.
    fn parse_block(&mut self) {
        let Some(raw) = self.lines.get(self.index).copied() else {
            return;
        };
        let trimmed = raw.trim();

        if trimmed.is_empty() {
            self.index += 1;
            return;
        }
        if is_delimiter(trimmed, '/') {
            self.skip_comment_block();
            return;
        }
        if trimmed.starts_with("//") {
            self.index += 1;
            return;
        }
        if let Some(entry) = parse_attribute_entry(trimmed) {
            self.attributes.insert(entry.0, entry.1);
            self.index += 1;
            return;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            self.pending_attrs = parse_block_attributes(trimmed);
            self.index += 1;
            return;
        }
        if let Some(title) = parse_block_title(trimmed) {
            let title = self.substitute_attributes(title);
            self.pending_title = Some(title);
            self.index += 1;
            return;
        }
        if trimmed.starts_with("|===") {
            self.parse_table();
            return;
        }
        if let Some(level) = section_level(trimmed) {
            self.parse_section_heading(level, trimmed);
            return;
        }
        if is_delimiter(trimmed, '+')
            && let Some(notation) = self.pending_math_notation()
        {
            self.parse_math_block(trimmed, notation);
            return;
        }
        if is_delimiter(trimmed, '-') || is_delimiter(trimmed, '.') {
            self.parse_verbatim_block(trimmed);
            return;
        }
        if is_delimiter(trimmed, '_') {
            self.parse_quote_block(trimmed);
            return;
        }
        if is_delimiter(trimmed, '=') {
            self.parse_example_block(trimmed);
            return;
        }
        if let Some((label, body)) = split_inline_admonition(trimmed) {
            self.push_admonition_with_body(&label.to_lowercase(), body);
            self.index += 1;
            return;
        }
        if let Some((depth, ordered)) = list_marker(trimmed) {
            self.parse_list_item(depth, ordered, trimmed);
            return;
        }
        self.parse_paragraph();
    }

    fn parse_section_heading(&mut self, level: usize, trimmed: &str) {
        self.close_lists();
        let text = trimmed[level..].trim();
        let (text, annotations) = self.parse_inline(text);
        let idx = self.builder.push_heading(level as u8, &text, None, None);
        if !annotations.is_empty() {
            self.builder.set_annotations(idx, annotations);
        }
        self.headers.push(text);
        self.index += 1;
        self.pending_attrs.clear();
        self.pending_title = None;
    }

    /// The math notation a pending `[latexmath]`, `[asciimath]`, or `[stem]`
    /// block attribute selects.
    ///
    /// `stem` follows the document's `:stem:` attribute, which AsciiDoc defines
    /// as AsciiMath unless the document names `latexmath`.
    fn pending_math_notation(&self) -> Option<MathNotation> {
        self.pending_attrs.first().and_then(|attr| {
            match attr
                .split(',')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase()
                .as_str()
            {
                "latexmath" => Some(MathNotation::Latex),
                "asciimath" => Some(MathNotation::AsciiMath),
                "stem" => Some(self.stem_notation()),
                _ => None,
            }
        })
    }

    /// What `stem` means in this document.
    fn stem_notation(&self) -> MathNotation {
        match self.attributes.get("stem").map(|v| v.trim().to_ascii_lowercase()) {
            Some(value) if value == "latexmath" || value == "latex" => MathNotation::Latex,
            _ => MathNotation::AsciiMath,
        }
    }

    /// Parse a `++++` passthrough block that a math attribute introduced.
    ///
    /// The body is one display equation, so it becomes a formula element.
    fn parse_math_block(&mut self, delimiter: &str, notation: MathNotation) {
        self.close_lists();
        self.index += 1;
        let (body, terminated) = self.collect_until_delimiter(delimiter);
        if !terminated {
            self.warn("unterminated delimited block closed at end of input");
        }
        let latex = self.math_to_latex(body.trim(), notation);
        if let Some(latex) = latex
            && !latex.is_empty()
        {
            self.builder.push_formula(&latex, None, None);
        }
        self.pending_attrs.clear();
        self.pending_title = None;
    }

    /// Convert math in `notation` to LaTeX.
    ///
    /// LaTeX passes through with its delimiters removed, since the formula
    /// element holds bare LaTeX. AsciiMath goes through the shared converter.
    fn math_to_latex(&mut self, source: &str, notation: MathNotation) -> Option<String> {
        if source.is_empty() {
            return None;
        }
        match notation {
            MathNotation::Latex => {
                let bare = crate::extraction::derive::strip_math_delimiters(source).trim();
                if bare.is_empty() { None } else { Some(bare.to_string()) }
            }
            // The AsciiMath converter needs `mathemascii`, which `office` brings
            // in. A build without it keeps the block as its own notation rather
            // than dropping the mathematics.
            #[cfg(feature = "office")]
            MathNotation::AsciiMath => {
                let mut budget = SecurityBudget::with_defaults();
                crate::extraction::asciimath::convert_asciimath_to_latex(source, &mut budget)
            }
            #[cfg(not(feature = "office"))]
            MathNotation::AsciiMath => Some(source.to_string()),
        }
    }

    /// Parse a `----` listing block or a `....` literal block into a code element.
    fn parse_verbatim_block(&mut self, delimiter: &str) {
        self.close_lists();
        let language = self.pending_source_language();
        let title = self.pending_title.take();
        self.index += 1;
        let (body, terminated) = self.collect_until_delimiter(delimiter);
        if !terminated {
            self.warn("unterminated delimited block closed at end of input");
        }
        let idx = self.builder.push_code(&body, language.as_deref(), None, None);
        if let Some(title) = title {
            self.builder.set_attributes(idx, single_attribute("title", &title));
        }
        self.code_blocks.push((language.unwrap_or_default(), body));
        self.pending_attrs.clear();
    }

    fn parse_quote_block(&mut self, delimiter: &str) {
        self.close_lists();
        self.index += 1;
        let (body, terminated) = self.collect_until_delimiter(delimiter);
        if !terminated {
            self.warn("unterminated quote block closed at end of input");
        }
        self.builder.push_quote_start();
        for paragraph in body.split("\n\n") {
            let trimmed = paragraph.trim();
            if trimmed.is_empty() {
                continue;
            }
            let (text, annotations) = self.parse_inline(&trimmed.replace('\n', " "));
            self.builder.push_paragraph(&text, annotations, None, None);
        }
        self.builder.push_quote_end();
        self.pending_attrs.clear();
        self.pending_title = None;
    }

    /// Parse an `====` example block. With a preceding `[NOTE]`-style attribute it is an
    /// admonition; otherwise its body is emitted as ordinary paragraphs.
    fn parse_example_block(&mut self, delimiter: &str) {
        self.close_lists();
        let admonition = self.pending_admonition_kind();
        self.index += 1;
        let (body, terminated) = self.collect_until_delimiter(delimiter);
        if !terminated {
            self.warn("unterminated example block closed at end of input");
        }
        let joined = body.split_whitespace().collect::<Vec<_>>().join(" ");
        match admonition {
            Some(kind) => self.push_admonition_with_body(&kind, &joined),
            None => {
                if !joined.is_empty() {
                    let (text, annotations) = self.parse_inline(&joined);
                    self.builder.push_paragraph(&text, annotations, None, None);
                }
            }
        }
        self.pending_attrs.clear();
        self.pending_title = None;
    }

    fn push_admonition_with_body(&mut self, kind: &str, body: &str) {
        self.close_lists();
        let idx = self.builder.push_admonition(kind, None, None);
        let (text, annotations) = self.parse_inline(body.trim());
        if !text.is_empty() {
            self.builder.set_text(idx, &text);
        }
        if !annotations.is_empty() {
            self.builder.set_annotations(idx, annotations);
        }
        self.pending_attrs.clear();
        self.pending_title = None;
    }

    /// Parse a `|===` delimited table into a structured table element.
    fn parse_table(&mut self) {
        self.close_lists();
        let expected_columns = self.pending_column_count();
        let title = self.pending_title.take();
        self.index += 1;

        let mut cells: Vec<String> = Vec::new();
        let mut first_row_width = 0usize;
        let mut terminated = false;
        while let Some(raw) = self.lines.get(self.index).copied() {
            self.index += 1;
            let trimmed = raw.trim();
            if trimmed.starts_with("|===") {
                terminated = true;
                break;
            }
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with('|') {
                let row = split_table_row(trimmed);
                if first_row_width == 0 {
                    first_row_width = row.len();
                }
                cells.extend(row.into_iter().map(|c| self.render_cell(&c)));
            } else if let Some(last) = cells.last_mut() {
                last.push(' ');
                last.push_str(trimmed);
            }
        }
        if !terminated {
            self.warn("unterminated table block closed at end of input");
        }

        let columns = expected_columns.unwrap_or(first_row_width).max(1);
        let grid: Vec<Vec<String>> = cells.chunks(columns).map(<[String]>::to_vec).collect();
        if grid.is_empty() {
            self.pending_attrs.clear();
            return;
        }
        let idx = self.builder.push_table_from_cells(&grid, None, None);
        if let Some(title) = title {
            self.builder.set_attributes(idx, single_attribute("title", &title));
        }
        self.pending_attrs.clear();
    }

    fn parse_list_item(&mut self, depth: usize, ordered: bool, trimmed: &str) {
        self.open_lists(depth, ordered);
        let content = trimmed
            .split_once(char::is_whitespace)
            .map_or("", |(_, rest)| rest)
            .trim();
        let mut text = content.to_string();
        self.index += 1;
        while let Some(next) = self.lines.get(self.index).copied() {
            let next_trimmed = next.trim();
            if next_trimmed.is_empty() || list_marker(next_trimmed).is_some() || section_level(next_trimmed).is_some() {
                break;
            }
            text.push(' ');
            text.push_str(next_trimmed);
            self.index += 1;
        }
        let (text, annotations) = self.parse_inline(&text);
        self.builder.push_list_item(&text, ordered, annotations, None, None);
        self.pending_attrs.clear();
        self.pending_title = None;
    }

    fn parse_paragraph(&mut self) {
        self.close_lists();
        let mut text = String::new();
        while let Some(raw) = self.lines.get(self.index).copied() {
            let trimmed = raw.trim();
            if trimmed.is_empty() || self.starts_new_block(trimmed) {
                break;
            }
            if !text.is_empty() && !text.ends_with('\n') {
                text.push(' ');
            }
            // A line ending in ` +` ends with a hard line break ~keep
            match trimmed.strip_suffix(" +") {
                Some(line) => {
                    text.push_str(line);
                    text.push('\n');
                }
                None => text.push_str(trimmed),
            }
            self.index += 1;
        }
        let text = text.trim_end_matches('\n');
        if text.is_empty() {
            self.index += 1;
            return;
        }
        let (text, annotations) = self.parse_inline(text);
        self.builder.push_paragraph(&text, annotations, None, None);
        self.pending_attrs.clear();
        self.pending_title = None;
    }

    /// True when `trimmed` opens a block that must not be absorbed into a paragraph.
    fn starts_new_block(&self, trimmed: &str) -> bool {
        section_level(trimmed).is_some()
            || list_marker(trimmed).is_some()
            || trimmed.starts_with("|===")
            || trimmed.starts_with("//")
            || parse_attribute_entry(trimmed).is_some()
            || parse_block_title(trimmed).is_some()
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
            || is_delimiter(trimmed, '-')
            || is_delimiter(trimmed, '.')
            || is_delimiter(trimmed, '=')
            || is_delimiter(trimmed, '_')
            || split_inline_admonition(trimmed).is_some()
    }

    /// Collect raw lines until a line equal to `delimiter`. Returns the body and whether
    /// the closing delimiter was actually found.
    fn collect_until_delimiter(&mut self, delimiter: &str) -> (String, bool) {
        let mut body: Vec<&str> = Vec::new();
        while let Some(raw) = self.lines.get(self.index).copied() {
            self.index += 1;
            if raw.trim() == delimiter {
                return (body.join("\n"), true);
            }
            body.push(raw);
        }
        (body.join("\n"), false)
    }

    fn skip_comment_block(&mut self) {
        self.index += 1;
        while let Some(raw) = self.lines.get(self.index).copied() {
            self.index += 1;
            if is_delimiter(raw.trim(), '/') {
                return;
            }
        }
        self.warn("unterminated comment block closed at end of input");
    }

    fn open_lists(&mut self, depth: usize, ordered: bool) {
        while self.list_stack.len() > depth {
            self.builder.end_list();
            self.list_stack.pop();
        }
        if self.list_stack.len() == depth
            && let Some(last) = self.list_stack.last().copied()
            && last != ordered
        {
            self.builder.end_list();
            self.list_stack.pop();
        }
        while self.list_stack.len() < depth {
            self.builder.push_list(ordered);
            self.list_stack.push(ordered);
        }
    }

    fn close_lists(&mut self) {
        while !self.list_stack.is_empty() {
            self.builder.end_list();
            self.list_stack.pop();
        }
    }

    fn pending_source_language(&self) -> Option<String> {
        let mut iter = self.pending_attrs.iter();
        let first = iter.next()?;
        if !first.eq_ignore_ascii_case("source") {
            return None;
        }
        iter.next().filter(|lang| !lang.is_empty()).cloned()
    }

    fn pending_admonition_kind(&self) -> Option<String> {
        let first = self.pending_attrs.first()?;
        ADMONITION_LABELS
            .into_iter()
            .find(|label| first.eq_ignore_ascii_case(label))
            .map(str::to_lowercase)
    }

    fn pending_column_count(&self) -> Option<usize> {
        self.pending_attrs
            .iter()
            .find_map(|attr| attr.strip_prefix("cols="))
            .map(|spec| count_columns(spec.trim_matches('"')))
            .filter(|count| *count > 0)
    }

    fn render_cell(&mut self, raw: &str) -> String {
        self.parse_inline(raw).0
    }

    fn warn(&mut self, message: &'static str) {
        self.warnings.push(ProcessingWarning {
            source: Cow::Borrowed("asciidoc"),
            message: Cow::Borrowed(message),
        });
    }

    /// Replace `{name}` references with defined document attributes.
    ///
    /// Undefined references are left verbatim: AsciiDoc treats them as literal text by
    /// default, and dropping them would silently lose content.
    fn substitute_attributes(&self, text: &str) -> String {
        if !text.contains('{') || self.attributes.is_empty() {
            return text.to_string();
        }
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        let last_delimiters = macro_delimiters(text);
        // A passthrough macro opens a word, as in `parse_inline` ~keep
        let mut at_boundary = true;
        while let Some(ch) = rest.chars().next() {
            // A passthrough keeps `{name}` unless it asks for attributes (`pass:a[...]`) ~keep
            if (rest.starts_with("+++") || (at_boundary && rest.starts_with("pass:")))
                && can_close_macro(rest, text.len() - rest.len(), last_delimiters)
                && let Some(passthrough) = parse_passthrough(rest)
            {
                let source = &rest[..passthrough.consumed];
                if passthrough.substitutes_attributes {
                    // `pass:a[` and `]` around the substituted text ~keep
                    out.push_str(&source[..source.len() - passthrough.text.len() - 1]);
                    out.push_str(&self.substitute_attributes(&passthrough.text));
                    out.push(']');
                } else {
                    out.push_str(source);
                }
                rest = &rest[passthrough.consumed..];
                at_boundary = false;
                continue;
            }
            // `\{name}` is an escaped reference; `parse_inline` drops the backslash ~keep
            if let Some(after) = rest.strip_prefix("\\{") {
                out.push_str("\\{");
                rest = after;
                at_boundary = false;
                continue;
            }
            if ch == '{' {
                let after = &rest[1..];
                let name_end = after
                    .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
                    .unwrap_or(after.len());
                if after[name_end..].starts_with('}')
                    && let Some(value) = self.attributes.get(&after[..name_end])
                {
                    out.push_str(value);
                    rest = &after[name_end + 1..];
                    at_boundary = false;
                    continue;
                }
            }
            out.push(ch);
            at_boundary = ch.is_whitespace() || ch == '(' || ch == '[';
            rest = &rest[ch.len_utf8()..];
        }
        out
    }

    /// Strip AsciiDoc inline markup, producing plain text plus annotations.
    fn parse_inline(&mut self, raw: &str) -> (String, Vec<TextAnnotation>) {
        let substituted = self.substitute_attributes(raw);
        let mut out = String::with_capacity(substituted.len());
        let mut annotations = Vec::new();
        let mut rest = substituted.as_str();
        let last_delimiters = macro_delimiters(&substituted);
        // An AsciiDoc constrained span may only open at the start of the text or after
        // whitespace / an opening bracket, so `midword*not*bold` stays literal.
        let mut at_boundary = true;

        while !rest.is_empty() {
            // An inline math macro stays in the sentence, as inline math does
            // for markdown, but reaches the text as LaTeX between `$`
            // delimiters rather than as the raw macro.
            if let Some((consumed, source, notation)) = parse_math_macro(rest, self.stem_notation()) {
                if let Some(latex) = self.math_to_latex(&source, notation) {
                    out.push('$');
                    out.push_str(&latex);
                    out.push('$');
                }
                rest = &rest[consumed..];
                at_boundary = false;
                continue;
            }
            // A backslash before markup keeps the markup character literal: `\*not bold*` ~keep
            if let Some(after) = rest.strip_prefix('\\')
                && let Some(escaped) = after.chars().next()
                && "*_`#^~+<[{".contains(escaped)
            {
                out.push(escaped);
                rest = &after[escaped.len_utf8()..];
                at_boundary = false;
                continue;
            }
            // The `pass:` and `xref:` macros open a word, so `bypass:[x]` stays literal ~keep
            let macro_allowed = at_boundary || !(rest.starts_with("pass:") || rest.starts_with("xref:"));
            if macro_allowed
                && can_close_macro(rest, substituted.len() - rest.len(), last_delimiters)
                && let Some((consumed, text)) = parse_passthrough(rest)
                    .map(|passthrough| (passthrough.consumed, passthrough.text))
                    .or_else(|| parse_cross_reference(rest))
            {
                out.push_str(&text);
                rest = &rest[consumed..];
                at_boundary = false;
                continue;
            }
            if let Some((consumed, display, url)) = parse_link_macro(rest) {
                let start = out.len() as u32;
                out.push_str(&display);
                annotations.push(TextAnnotation {
                    start,
                    end: out.len() as u32,
                    kind: AnnotationKind::Link {
                        url: url.clone(),
                        title: None,
                    },
                });
                self.links.push((display, url));
                rest = &rest[consumed..];
                at_boundary = false;
                continue;
            }
            if at_boundary && let Some((consumed, inner, kind)) = parse_constrained_span(rest) {
                let start = out.len() as u32;
                out.push_str(&inner);
                annotations.push(TextAnnotation {
                    start,
                    end: out.len() as u32,
                    kind,
                });
                rest = &rest[consumed..];
                at_boundary = false;
                continue;
            }
            let Some(ch) = rest.chars().next() else { break };
            out.push(ch);
            at_boundary = ch.is_whitespace() || ch == '(' || ch == '[';
            rest = &rest[ch.len_utf8()..];
        }

        (out, annotations)
    }
}

impl Plugin for AsciiDocExtractor {
    fn name(&self) -> &str {
        "asciidoc-extractor"
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
        "Extracts structured content from AsciiDoc documents"
    }

    fn author(&self) -> &str {
        "Xberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for AsciiDocExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let mut budget = SecurityBudget::from_config(config);
        budget.account_text(content.len())?;

        // See the identical rationale on `extractors::text::PlainTextExtractor::extract_content`
        // (#171, #395): `replaced_characters` is recorded by the decoder itself, at the point
        // data is actually lost, so it stays accurate under `quality`'s mojibake cleanup where
        // scanning the decoded text for U+FFFD afterwards would not. ~keep
        let outcome = crate::utils::decode_with_provenance(content, None);
        let decoded_lossily = outcome.replaced_characters;
        let source = normalize_line_endings(crate::utils::strip_bom(&outcome.text));

        let parsed = AsciiDocParser::new(&source).parse();
        let mut document = parsed.document;
        document.mime_type = mime_type.to_string();
        if decoded_lossily {
            crate::core::diagnostics::push_lossy_decode_warning(
                &mut document.processing_warnings,
                "asciidoc",
                "AsciiDoc source",
            );
        }

        let body: String = document
            .elements
            .iter()
            .map(|element| element.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        document.metadata.format = Some(crate::types::FormatMetadata::Text(crate::types::TextMetadata {
            line_count: source.lines().count() as u32,
            word_count: body.split_whitespace().count() as u32,
            character_count: body.chars().count() as u32,
            headers: Some(parsed.headers).filter(|h| !h.is_empty()),
            links: Some(parsed.links.into_iter().map(Into::into).collect()).filter(|l: &Vec<_>| !l.is_empty()),
            code_blocks: Some(parsed.code_blocks.into_iter().map(Into::into).collect())
                .filter(|blocks: &Vec<_>| !blocks.is_empty()),
        }));

        Ok(document)
    }

    fn supported_mime_types(&self) -> &[&str] {
        // `text/x-asciidoc` is declared as an alias of `text/asciidoc` in core/mime.rs.
        // The registry resolves extractors by exact string with no alias awareness, so an
        // unclaimed alias would be advertised as supported and then fail as
        // UnsupportedFormat (#229).
        &["text/asciidoc", "text/x-asciidoc"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests;
