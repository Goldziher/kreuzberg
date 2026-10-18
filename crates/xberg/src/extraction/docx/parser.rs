//! Inline DOCX XML parser.
//!
//! Vendored and adapted from [docx-lite](https://github.com/v-lawyer/docx-lite) v0.2.0
//! (MIT OR Apache-2.0, V-Lawyer Team). See ATTRIBUTIONS.md for details.
//!
//! Changes from upstream:
//! - `Paragraph::to_text()` joins runs with `" "` instead of `""` (fixes #359)
//! - Adapted to use xberg's existing `quick-xml` and `zip` versions
//! - Removed file-path based APIs (we only need bytes/reader)
//! - Added markdown rendering and formatting support (fixes #376)

// TODO(xberg-io/xberg#1567): 4 cyclomatic-complexity and 39 size/complexity findings
// in this file, currently excluded via the quality-debt baseline in alef.toml. Splitting
// these needs compiler-in-the-loop verification, not a mechanical pass. Delete this
// note and the file's baseline entry together once it goes green. Help wanted.

use crate::extractors::security::{SecurityBudget, SecurityError, SecurityLimits, ZipBombValidator};
use ahash::AHashMap;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read, Seek};

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

/// Tracks document element ordering (paragraphs, tables, and drawings interleaved).
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum DocumentElement {
    Paragraph(usize),
    Table(usize),
    Drawing(usize),
    PageBreak,
}

#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default)]
pub(crate) struct Document {
    pub paragraphs: Vec<Paragraph>,
    pub note_refs_by_paragraph: AHashMap<usize, Vec<NoteRef>>,
    pub tables: Vec<Table>,
    pub headers: Vec<HeaderFooter>,
    pub footers: Vec<HeaderFooter>,
    pub footnotes: Vec<Note>,
    pub endnotes: Vec<Note>,
    pub numbering_defs: AHashMap<(i64, i64), ListType>,
    /// Document elements in their original order.
    pub elements: Vec<DocumentElement>,
    /// Parsed style catalog from `word/styles.xml`, if available.
    pub style_catalog: Option<super::styles::StyleCatalog>,
    /// Parsed theme from `word/theme/theme1.xml`, if available.
    pub theme: Option<super::theme::Theme>,
    /// Section properties parsed from `w:sectPr` elements.
    pub sections: Vec<super::section::SectionProperties>,
    /// Drawing objects parsed from `w:drawing` elements.
    pub drawings: Vec<super::drawing::Drawing>,
    /// Image relationships (rId → target path) for image extraction.
    pub image_relationships: AHashMap<String, String>,
    /// Track-changes revisions captured from `w:ins`, `w:del`, and `w:rPrChange` elements.
    pub revisions: Vec<crate::types::revisions::DocumentRevision>,
    /// Comments parsed from `word/comments.xml`, joined to the body via
    /// `w:commentRangeStart`/`w:commentRangeEnd` and `w:commentReference` (#82).
    pub comments: Vec<Comment>,
    /// Non-fatal degradation warnings accumulated while parsing (#171).
    pub warnings: Vec<crate::types::ProcessingWarning>,
}

/// A DOCX paragraph containing formatted text runs.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Paragraph {
    /// Formatted text runs that make up this paragraph.
    pub runs: Vec<Run>,
    /// Style ID applied to this paragraph (e.g. `"Heading1"`, `"Normal"`).
    pub style: Option<String>,
    /// Numbering definition ID for bulleted or numbered lists.
    pub numbering_id: Option<i64>,
    /// Indentation level within the numbering definition (0-based).
    pub numbering_level: Option<i64>,
    /// Bookmark names (`w:bookmarkStart/@w:name`) that start inside this paragraph.
    ///
    /// A generated table of contents links each entry to the `_Toc…` bookmark Word
    /// writes into the heading paragraph, so these are the link targets internal
    /// `w:hyperlink w:anchor` references resolve against. Word's `_GoBack` bookmark
    /// is filtered out — it records the last edit position, not a link target.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bookmarks: Vec<String>,
    /// True when this paragraph is part of a table of contents.
    ///
    /// Set by either of the two markers Word emits: a `w:sdt` whose
    /// `w:sdtPr/w:docPartObj/w:docPartGallery` is `Table of Contents`, or a `TOC`
    /// field code.
    #[serde(default)]
    pub in_table_of_contents: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NoteRef {
    pub(crate) id: String,
    pub(crate) note_type: NoteType,
}

/// A formatted text run within a DOCX paragraph.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Run {
    /// Plain text content of this run.
    pub text: String,
    /// Bold formatting flag.
    pub bold: bool,
    /// Italic formatting flag.
    pub italic: bool,
    /// Underline formatting flag.
    pub underline: bool,
    /// Strikethrough formatting flag.
    pub strikethrough: bool,
    /// Subscript vertical alignment flag.
    pub subscript: bool,
    /// Superscript vertical alignment flag.
    pub superscript: bool,
    /// Font size in half-points (from `w:sz`).
    pub font_size: Option<u32>,
    /// Font color as "RRGGBB" hex (from `w:color`).
    pub font_color: Option<String>,
    /// Highlight color name (from `w:highlight`).
    pub highlight: Option<String>,
    /// Hyperlink URL, if this run is wrapped in a `<w:hyperlink>`.
    pub hyperlink_url: Option<String>,
    /// LaTeX math content: (latex_source, is_display_math).
    /// When set, this run represents an equation and `text` is ignored.
    pub math_latex: Option<(String, bool)>,
}

/// A DOCX table parsed from `<w:tbl>`.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Table {
    /// Ordered list of table rows.
    pub rows: Vec<TableRow>,
    /// Table-level properties from `<w:tblPr>`.
    pub properties: Option<super::table::TableProperties>,
    /// Column width definitions from `<w:tblGrid>`.
    pub grid: Option<super::table::TableGrid>,
}

/// A single row within a DOCX table.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableRow {
    /// Cells in this row.
    pub cells: Vec<TableCell>,
    /// Row-level properties from `<w:trPr>`.
    pub properties: Option<super::table::RowProperties>,
}

/// A single cell within a DOCX table row.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableCell {
    /// Paragraphs contained in this cell.
    pub paragraphs: Vec<Paragraph>,
    /// Cell-level properties from `<w:tcPr>`.
    pub properties: Option<super::table::CellProperties>,
}

#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ListType {
    Bullet,
    Numbered,
}

/// A header or footer block from a DOCX section.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default)]
pub struct HeaderFooter {
    /// Paragraphs in this header or footer.
    pub paragraphs: Vec<Paragraph>,
    /// Tables in this header or footer.
    pub tables: Vec<Table>,
    /// Which pages this header/footer applies to.
    pub header_type: HeaderFooterType,
}

/// Specifies which pages a header or footer applies to.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default, PartialEq)]
pub enum HeaderFooterType {
    /// Default header/footer (applies to all pages not covered by First or Even).
    #[default]
    Default,
    /// First-page header/footer.
    First,
    /// Even-page header/footer.
    Even,
    /// Odd-page header/footer.
    Odd,
}

/// A footnote or endnote from a DOCX document.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub struct Note {
    /// Note identifier matching the in-text reference mark.
    pub id: String,
    /// Whether this is a footnote or an endnote.
    pub note_type: NoteType,
    /// Paragraphs of note content.
    pub paragraphs: Vec<Paragraph>,
}

/// Distinguishes footnotes from endnotes in DOCX documents.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NoteType {
    /// Note appears at the bottom of the page.
    Footnote,
    /// Note appears at the end of the document.
    Endnote,
}

/// A comment from `word/comments.xml` (#82).
///
/// Joined to the body text via `w:commentRangeStart`/`w:commentRangeEnd` (anchors,
/// not carrying content) and `w:commentReference` (the marker matched to this `id`).
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default)]
pub struct Comment {
    /// Comment identifier matching `w:commentReference/@w:id`.
    pub id: String,
    /// Comment author, if present.
    pub author: Option<String>,
    /// Paragraphs of comment content.
    pub paragraphs: Vec<Paragraph>,
}

/// Check if a formatting element is enabled (not explicitly set to false/0/none).
fn is_format_enabled(e: &BytesStart) -> bool {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == "w:val" {
            let val = attr.value.as_ref();
            return !matches!(val, "false" | "0" | "none");
        }
    }
    true
}

/// Read `w:val` attribute as i64.
fn get_val_attr(e: &BytesStart) -> Option<i64> {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == "w:val" {
            let val = attr.value.as_ref();
            return val.parse().ok();
        }
    }
    None
}

/// Read `w:val` attribute as String.
fn get_val_attr_string(e: &BytesStart) -> Option<String> {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == "w:val" {
            let val = attr.value.as_ref();
            return Some(val.to_string());
        }
    }
    None
}

/// The list kind a `w:numFmt` value stands for. `bullet` is a bullet list, and `none`, a
/// level that shows no marker, stays a bullet as before. Every other `ST_NumberFormat` value
/// numbers the items: besides the Latin decimal, letter and roman forms, that includes the CJK
/// counting forms (`chineseCounting`, `japaneseCounting`, `ideographTraditional`), the enclosed
/// and full-width digits, `ordinal`, `cardinalText`, the Hebrew, Arabic, Hindi, Thai and Russian
/// letters, and Word's `custom` format.
fn list_type_for_num_fmt(fmt: Option<&str>) -> ListType {
    match fmt {
        None | Some("bullet") | Some("none") => ListType::Bullet,
        Some(_) => ListType::Numbered,
    }
}

/// Extract the target URL from a `HYPERLINK` field instruction (#88, #239).
///
/// Matches instruction text such as `HYPERLINK "https://example.com"` or
/// `HYPERLINK "https://example.com" \o "tooltip"` (from a `w:fldSimple/@w:instr`
/// attribute or accumulated `w:instrText` run text), returning the first quoted
/// argument. Returns `None` for other field types (PAGE, REF, TOC, ...) or
/// malformed instructions.
fn extract_hyperlink_field_url(instr: &str) -> Option<String> {
    let trimmed = instr.trim();
    let head = trimmed.get(..9)?;
    if !head.eq_ignore_ascii_case("HYPERLINK") {
        return None;
    }
    let rest = trimmed.get(9..)?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    let url = rest[..end].trim();
    if url.is_empty() { None } else { Some(url.to_string()) }
}

/// Collect the standard revision-mark attributes from a `w:ins`, `w:del`, or
/// `w:rPrChange` start element: `w:id`, `w:author`, and `w:date`.
///
/// Returns `(id_opt, author_opt, date_opt)` where `id_opt` is the raw string
/// value of the `w:id` attribute (a numeric string like `"42"`).
fn collect_revision_attrs(e: &BytesStart) -> (Option<String>, Option<String>, Option<String>) {
    let mut id: Option<String> = None;
    let mut author: Option<String> = None;
    let mut date: Option<String> = None;
    for attr in e.attributes().flatten() {
        match attr.key.as_ref() {
            "w:id" => {
                id = Some(attr.value.to_string());
            }
            "w:author" => {
                author = Some(attr.value.to_string());
                if author.as_deref() == Some("") {
                    author = None;
                }
            }
            "w:date" => {
                date = Some(attr.value.to_string());
                if date.as_deref() == Some("") {
                    date = None;
                }
            }
            _ => {}
        }
    }
    (id, author, date)
}

/// Map heading style name to markdown heading level (fallback for docs without styles.xml).
fn heading_level_from_style_name(style: &str) -> Option<u8> {
    match style {
        "Title" => Some(1),
        s if s.starts_with("Heading") || s.starts_with("heading") => {
            let num_part = s.trim_start_matches("Heading").trim_start_matches("heading");
            if let Ok(n) = num_part.parse::<u8>()
                && (1..=6).contains(&n)
            {
                // The trailing digit of a Word style ID is already the 1-indexed heading
                // level: `Heading1` IS level 1. The `+ 1` that the StyleCatalog branch
                // applies belongs to `w:outlineLvl`, which is 0-indexed; applying it here
                // too reported every H1 as an H2 (#732).
                return Some(n);
            }
            None
        }
        _ => None,
    }
}

impl Document {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Render the document and return both the text and accurate page boundaries.
    ///
    /// `inject_placeholders` controls whether image reference markers (`![](image_N)`) are
    /// emitted in the markdown text. Passing `false` suppresses them, which is honoured when
    /// `ImageExtractionConfig::inject_placeholders` is set to `false` by the caller.
    pub fn extract_text_with_boundaries(
        &self,
        is_markdown: bool,
        inject_placeholders: bool,
    ) -> (String, Vec<crate::types::PageBoundary>) {
        let text = if is_markdown {
            self.to_markdown(inject_placeholders)
        } else {
            self.to_plain_text()
        };

        let mut boundaries = Vec::new();
        let mut start_idx = 0;
        let mut page_num = 1;

        for (idx, _) in text.match_indices('\x0c') {
            boundaries.push(crate::types::PageBoundary {
                byte_start: start_idx,
                byte_end: idx,
                page_number: page_num,
            });
            start_idx = idx + 1;
            page_num += 1;
        }

        boundaries.push(crate::types::PageBoundary {
            byte_start: start_idx,
            byte_end: text.len(),
            page_number: page_num,
        });

        (text, boundaries)
    }

    /// Return the 1-based page number for each top-level table in the document.
    pub fn table_page_numbers(&self) -> Vec<usize> {
        let mut table_page_numbers = Vec::new();
        let mut current_page = 1;

        for element in &self.elements {
            match element {
                DocumentElement::PageBreak => current_page += 1,
                DocumentElement::Table(_) => {
                    table_page_numbers.push(current_page);
                }
                _ => {}
            }
        }

        table_page_numbers
    }

    /// Return the 1-based page number for each drawing, in `self.drawings` index order.
    ///
    /// Derived by walking the parsed element list, exactly as `table_page_numbers` does, rather
    /// than by searching rendered markdown for a placeholder. `to_markdown` renders every drawing
    /// to the same `![alt](image)` target, so no per-image key exists in the text to search for;
    /// the previous lookup asked for one and always missed, reporting page 1 for every image
    /// (GH#1546). Walking the elements is also independent of `inject_placeholders`, which
    /// suppresses those placeholders entirely. ~keep
    pub fn drawing_page_numbers(&self) -> Vec<usize> {
        let mut drawing_page_numbers = vec![1; self.drawings.len()];
        let mut current_page = 1;

        for element in &self.elements {
            match element {
                DocumentElement::PageBreak => current_page += 1,
                DocumentElement::Drawing(index) => {
                    if let Some(page) = drawing_page_numbers.get_mut(*index) {
                        *page = current_page;
                    }
                }
                _ => {}
            }
        }

        drawing_page_numbers
    }

    /// Internal helper to ensure a blank line before appending new content.
    fn ensure_blank_line(output: &mut String) {
        if !output.is_empty() && !output.ends_with("\n\n") {
            if output.ends_with('\n') {
                output.push('\n');
            } else {
                output.push_str("\n\n");
            }
        }
    }

    /// Resolve heading level for a paragraph style using the StyleCatalog.
    ///
    /// Walks the style inheritance chain to find `outline_level`.
    /// Falls back to string-matching on style name/ID if no StyleCatalog is available.
    /// Returns 1-6 (markdown heading levels).
    pub(crate) fn resolve_heading_level(&self, style_id: &str) -> Option<u8> {
        if let Some(ref catalog) = self.style_catalog {
            let mut current_id = Some(style_id);
            let mut visited = 0;
            while let Some(id) = current_id {
                if visited > 20 {
                    break;
                }
                visited += 1;
                if let Some(style_def) = catalog.styles.get(id) {
                    if let Some(level) = style_def.paragraph_properties.outline_level {
                        // `outline_level` is a document-controlled `u8` (styles.xml's
                        // `w:outlineLvl w:val`) that can legally be as high as 255; a
                        // plain `level + 1` overflows when `level == 255` (panics under
                        // overflow-checks, wraps to 0 otherwise). `saturating_add` keeps
                        // this branch's result pinned at the `.min(6)` ceiling either way.
                        return Some(level.saturating_add(1).min(6));
                    }
                    if let Some(ref name) = style_def.name
                        && (name == "Title" || name == "title")
                    {
                        return Some(1);
                    }
                    current_id = style_def.based_on.as_deref();
                } else {
                    break;
                }
            }
        }
        heading_level_from_style_name(style_id)
    }

    /// Resolve the human-readable style name for a paragraph style using the StyleCatalog.
    ///
    /// Walks the style inheritance chain (via `w:basedOn`) until it finds a `w:name`,
    /// so styles that only set `w:basedOn` without their own name still resolve to the
    /// ancestor's canonical name. Returns `None` if no `StyleCatalog` is available or no
    /// style in the chain has a name.
    pub(crate) fn resolve_style_name(&self, style_id: &str) -> Option<String> {
        let catalog = self.style_catalog.as_ref()?;
        let mut current_id = Some(style_id);
        let mut visited = 0;
        while let Some(id) = current_id {
            if visited > 20 {
                break;
            }
            visited += 1;
            let style_def = catalog.styles.get(id)?;
            if let Some(ref name) = style_def.name {
                return Some(name.clone());
            }
            current_id = style_def.based_on.as_deref();
        }
        None
    }

    #[cfg(test)]
    pub(crate) fn extract_text(&self) -> String {
        let mut text = String::new();

        for paragraph in &self.paragraphs {
            let para_text = paragraph.to_text();
            if !para_text.is_empty() {
                text.push_str(&para_text);
                text.push('\n');
            }
        }

        for table in &self.tables {
            for row in &table.rows {
                for cell in &row.cells {
                    for paragraph in &cell.paragraphs {
                        let para_text = paragraph.to_text();
                        if !para_text.is_empty() {
                            text.push_str(&para_text);
                            text.push('\t');
                        }
                    }
                }
                text.push('\n');
            }
            text.push('\n');
        }

        text
    }

    /// Render the document as markdown.
    ///
    /// When `inject_placeholders` is `true`, drawings that reference an image
    /// emit `![alt](image)` placeholders. When `false` they are silently
    /// skipped, which is useful when the caller only wants text.
    pub(crate) fn to_markdown(&self, inject_placeholders: bool) -> String {
        use std::fmt::Write;

        let mut output = String::new();
        let mut list_counters: AHashMap<(i64, i64), usize> = AHashMap::new();
        let mut prev_was_list = false;

        if !self.elements.is_empty() {
            for element in &self.elements {
                match element {
                    DocumentElement::Paragraph(idx) => {
                        let Some(paragraph) = self.paragraphs.get(*idx) else {
                            continue;
                        };
                        self.append_paragraph_markdown(paragraph, &mut output, &mut list_counters, &mut prev_was_list);
                    }
                    DocumentElement::Table(idx) => {
                        let Some(table) = self.tables.get(*idx) else { continue };
                        Self::ensure_blank_line(&mut output);
                        if let Some(ref props) = table.properties
                            && let Some(ref caption) = props.caption
                        {
                            output.push_str(caption);
                            output.push_str("\n\n");
                        }
                        output.push_str(&table.to_markdown());
                        prev_was_list = false;
                    }
                    DocumentElement::Drawing(idx) => {
                        let Some(drawing) = self.drawings.get(*idx) else {
                            continue;
                        };
                        if drawing.image_ref.is_none() {
                            continue;
                        }
                        if inject_placeholders {
                            let alt = drawing
                                .doc_properties
                                .as_ref()
                                .and_then(|dp| dp.description.as_deref())
                                .unwrap_or("");
                            Self::ensure_blank_line(&mut output);
                            let _ = writeln!(output, "![{}](image)", alt);
                        }
                        prev_was_list = false;
                    }
                    DocumentElement::PageBreak => {
                        output.push('\x0c');
                        prev_was_list = false;
                    }
                }
            }
        } else {
            for paragraph in &self.paragraphs {
                self.append_paragraph_markdown(paragraph, &mut output, &mut list_counters, &mut prev_was_list);
            }
        }

        if !self.footnotes.is_empty() {
            output.push_str("\n\n");
            for note in &self.footnotes {
                let note_text: String = note
                    .paragraphs
                    .iter()
                    .map(|p| p.runs_to_markdown())
                    .collect::<Vec<_>>()
                    .join(" ");
                if !note_text.is_empty() {
                    let _ = writeln!(output, "[^{}]: {}", note.id, note_text);
                }
            }
        }

        if !self.endnotes.is_empty() {
            output.push_str("\n\n");
            for note in &self.endnotes {
                let note_text: String = note
                    .paragraphs
                    .iter()
                    .map(|p| p.runs_to_markdown())
                    .collect::<Vec<_>>()
                    .join(" ");
                if !note_text.is_empty() {
                    let _ = writeln!(output, "[^{}]: {}", note.id, note_text);
                }
            }
        }

        let (content_start, content_end) = blank_line_trim_range(&output);
        output.truncate(content_end);
        output.drain(..content_start);
        output
    }

    /// Render the document as plain text (no markdown formatting).
    pub(crate) fn to_plain_text(&self) -> String {
        let mut output = String::new();

        if !self.elements.is_empty() {
            for element in &self.elements {
                match element {
                    DocumentElement::Paragraph(idx) => {
                        let Some(paragraph) = self.paragraphs.get(*idx) else {
                            continue;
                        };
                        let text = paragraph.to_text();
                        if !text.is_empty() {
                            Self::ensure_blank_line(&mut output);
                            output.push_str(&text);
                        }
                    }
                    DocumentElement::Table(idx) => {
                        let Some(table) = self.tables.get(*idx) else { continue };
                        Self::ensure_blank_line(&mut output);
                        if let Some(ref props) = table.properties
                            && let Some(ref caption) = props.caption
                        {
                            output.push_str(caption);
                            output.push_str("\n\n");
                        }
                        output.push_str(&table.to_plain_text());
                    }
                    DocumentElement::Drawing(idx) => {
                        let Some(drawing) = self.drawings.get(*idx) else {
                            continue;
                        };
                        if let Some(alt) = drawing.doc_properties.as_ref().and_then(|dp| dp.description.as_deref())
                            && !alt.is_empty()
                        {
                            Self::ensure_blank_line(&mut output);
                            output.push_str(alt);
                        }
                    }
                    DocumentElement::PageBreak => {
                        output.push('\x0c');
                    }
                }
            }
        } else {
            for paragraph in &self.paragraphs {
                let text = paragraph.to_text();
                if !text.is_empty() {
                    Self::ensure_blank_line(&mut output);
                    output.push_str(&text);
                }
            }
        }

        if !self.footnotes.is_empty() {
            output.push_str("\n\n");
            for note in &self.footnotes {
                let note_text: String = note
                    .paragraphs
                    .iter()
                    .map(|p| p.to_text())
                    .collect::<Vec<_>>()
                    .join(" ");
                if !note_text.is_empty() {
                    output.push_str(&note.id);
                    output.push_str(": ");
                    output.push_str(&note_text);
                    output.push('\n');
                }
            }
        }

        if !self.endnotes.is_empty() {
            output.push_str("\n\n");
            for note in &self.endnotes {
                let note_text: String = note
                    .paragraphs
                    .iter()
                    .map(|p| p.to_text())
                    .collect::<Vec<_>>()
                    .join(" ");
                if !note_text.is_empty() {
                    output.push_str(&note.id);
                    output.push_str(": ");
                    output.push_str(&note_text);
                    output.push('\n');
                }
            }
        }

        let trimmed_end = output.trim_end().len();
        output.truncate(trimmed_end);
        let trimmed_start = output.len() - output.trim_start().len();
        if trimmed_start > 0 {
            output.drain(..trimmed_start);
        }
        output
    }

    /// Helper: append a paragraph's markdown to output, managing list transitions.
    fn append_paragraph_markdown(
        &self,
        paragraph: &Paragraph,
        output: &mut String,
        list_counters: &mut AHashMap<(i64, i64), usize>,
        prev_was_list: &mut bool,
    ) {
        let is_list = paragraph.numbering_id.is_some();

        if is_list && !*prev_was_list {
            Self::ensure_blank_line(output);
        }

        if !is_list && *prev_was_list {
            Self::ensure_blank_line(output);
        }

        let heading_level = paragraph.style.as_deref().and_then(|s| self.resolve_heading_level(s));
        let md = paragraph.to_markdown(&self.numbering_defs, list_counters, heading_level);
        if md.is_empty() {
            *prev_was_list = is_list;
            return;
        }

        let is_quote = paragraph.style.as_deref().is_some_and(|s| {
            let lower = s.to_ascii_lowercase();
            lower == "quote" || lower == "blockquote" || lower.contains("quote")
        });

        if is_list {
            if *prev_was_list {
                output.push('\n');
            }
            output.push_str(&md);
        } else if is_quote {
            Self::ensure_blank_line(output);
            output.push_str("> ");
            output.push_str(&md);
        } else {
            Self::ensure_blank_line(output);
            output.push_str(&md);
        }

        *prev_was_list = is_list;
    }
}

impl Paragraph {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Concatenate text runs to produce paragraph text.
    ///
    /// In DOCX, whitespace between words is stored inside `<w:t>` elements
    /// (e.g. `<w:t>Hello </w:t><w:t>World</w:t>`), so runs are joined
    /// directly without adding extra separators. The parser must use
    /// `trim_text(false)` to preserve this whitespace.
    pub(crate) fn to_text(&self) -> String {
        let mut text = String::new();
        for run in &self.runs {
            if let Some((ref latex, _)) = run.math_latex {
                text.push_str(latex);
            } else {
                text.push_str(&run.text);
            }
        }
        text
    }

    /// Render inline runs as markdown (no paragraph-level wrapping).
    ///
    /// Uses a two-level grouping strategy to avoid spurious marker sequences like `****`:
    /// 1. Groups consecutive runs that share the same bold/italic/hyperlink properties.
    /// 2. Within each group, opens bold/italic once and toggles underline/strikethrough per run.
    pub(crate) fn runs_to_markdown(&self) -> String {
        let mut text = String::new();
        let mut i = 0;
        while i < self.runs.len() {
            let run = &self.runs[i];

            if run.math_latex.is_some() || run.text.is_empty() {
                text.push_str(&run.to_markdown());
                i += 1;
                continue;
            }

            let group_start = i;
            let mut j = i + 1;
            while j < self.runs.len() {
                let next = &self.runs[j];
                if next.math_latex.is_some()
                    || next.text.is_empty()
                    || next.bold != run.bold
                    || next.italic != run.italic
                    || next.hyperlink_url != run.hyperlink_url
                {
                    break;
                }
                j += 1;
            }
            let group_end = j;

            let all_same_inner = self.runs[group_start..group_end]
                .iter()
                .all(|r| r.underline == run.underline && r.strikethrough == run.strikethrough);

            if all_same_inner {
                let mut merged_text = String::new();
                for r in &self.runs[group_start..group_end] {
                    merged_text.push_str(&r.text);
                }
                let merged_run = Run {
                    text: merged_text,
                    bold: run.bold,
                    italic: run.italic,
                    underline: run.underline,
                    strikethrough: run.strikethrough,
                    hyperlink_url: run.hyperlink_url.clone(),
                    ..Default::default()
                };
                text.push_str(&merged_run.to_markdown());
            } else {
                if run.hyperlink_url.is_some() {
                    text.push('[');
                }
                if run.bold && run.italic {
                    text.push_str("***");
                } else if run.bold {
                    text.push_str("**");
                } else if run.italic {
                    text.push('*');
                }

                for r in &self.runs[group_start..group_end] {
                    if r.underline {
                        text.push_str("<u>");
                    }
                    if r.strikethrough {
                        text.push_str("~~");
                    }
                    text.push_str(&r.text);
                    if r.strikethrough {
                        text.push_str("~~");
                    }
                    if r.underline {
                        text.push_str("</u>");
                    }
                }

                if run.bold && run.italic {
                    text.push_str("***");
                } else if run.bold {
                    text.push_str("**");
                } else if run.italic {
                    text.push('*');
                }
                if let Some(ref url) = run.hyperlink_url {
                    text.push_str("](");
                    text.push_str(url);
                    text.push(')');
                }
            }

            i = group_end;
        }
        text
    }

    /// Render as markdown with heading/list context.
    ///
    /// If `heading_level` is provided (resolved via `Document::resolve_heading_level`),
    /// it takes precedence over style name matching.
    pub(crate) fn to_markdown(
        &self,
        numbering_defs: &AHashMap<(i64, i64), ListType>,
        list_counters: &mut AHashMap<(i64, i64), usize>,
        heading_level: Option<u8>,
    ) -> String {
        let inline = self.runs_to_markdown();

        if let Some(level) = heading_level {
            let hashes = "#".repeat(level as usize);
            return format!("{} {}", hashes, inline);
        }

        if let (Some(num_id), Some(level)) = (self.numbering_id, self.numbering_level) {
            let indent = "  ".repeat(level as usize);
            let key = (num_id, level);
            let list_type = numbering_defs.get(&key).copied().unwrap_or(ListType::Bullet);

            match list_type {
                ListType::Bullet => {
                    return format!("{}- {}", indent, inline);
                }
                ListType::Numbered => {
                    let counter = list_counters.entry(key).or_insert(0);
                    *counter += 1;
                    return format!("{}{}. {}", indent, *counter, inline);
                }
            }
        }

        inline
    }

    pub(crate) fn add_run(&mut self, run: Run) {
        self.runs.push(run);
    }
}

impl Run {
    #[cfg(test)]
    pub(crate) fn new(text: String) -> Self {
        Self {
            text,
            ..Default::default()
        }
    }

    /// Render this run as markdown with formatting markers.
    pub(crate) fn to_markdown(&self) -> String {
        if let Some((ref latex, is_display)) = self.math_latex {
            if latex.is_empty() {
                return String::new();
            }
            return if is_display {
                format!("$${}$$", latex)
            } else {
                format!("${}$", latex)
            };
        }

        if self.text.is_empty() {
            return String::new();
        }

        let extra = (if self.bold && self.italic {
            6
        } else if self.bold || self.italic {
            4
        } else {
            0
        }) + (if self.strikethrough { 4 } else { 0 })
            + (if self.underline { 7 } else { 0 })
            + self.hyperlink_url.as_ref().map_or(0, |u| u.len() + 4);
        let mut out = String::with_capacity(self.text.len() + extra);

        if self.hyperlink_url.is_some() {
            out.push('[');
        }
        if self.underline {
            out.push_str("<u>");
        }
        if self.strikethrough {
            out.push_str("~~");
        }
        if self.bold && self.italic {
            out.push_str("***");
        } else if self.bold {
            out.push_str("**");
        } else if self.italic {
            out.push('*');
        }

        out.push_str(&self.text);

        if self.bold && self.italic {
            out.push_str("***");
        } else if self.bold {
            out.push_str("**");
        } else if self.italic {
            out.push('*');
        }
        if self.strikethrough {
            out.push_str("~~");
        }
        if self.underline {
            out.push_str("</u>");
        }
        if let Some(ref url) = self.hyperlink_url {
            out.push_str("](");
            out.push_str(url);
            out.push(')');
        }

        out
    }
}

impl TableCell {
    /// An empty cell spanning `columns` grid columns. It stands for the columns a row leaves
    /// empty with `w:gridBefore`/`w:gridAfter`, so the row's real cells sit under the grid
    /// columns they belong to wherever the row's cells are laid out by their spans.
    fn grid_padding(columns: u32) -> Self {
        Self {
            paragraphs: Vec::new(),
            properties: Some(super::table::CellProperties {
                grid_span: Some(columns),
                ..Default::default()
            }),
        }
    }
}

impl Table {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Build this table's cell grid, writing each `gridSpan`/`vMerge`-spanned cell once at
    /// its origin and leaving the columns and rows it covers blank.
    ///
    /// `render` extracts a paragraph's text (`runs_to_markdown` for Markdown output,
    /// `to_text` for plain text); it is the only difference between `to_markdown` and
    /// `to_plain_text`'s grids, so both share this builder (xberg-io/xberg#1549 — this is
    /// also the one grid `extractors/docx.rs`'s consumer-visible paths must match, instead
    /// of separately cloning a spanned cell's text into every covered column). ~keep
    pub(crate) fn to_cell_grid(&self, render: impl Fn(&Paragraph) -> String) -> Vec<Vec<String>> {
        let mut cells: Vec<Vec<String>> = Vec::new();
        for row in &self.rows {
            let mut row_cells = Vec::new();
            for cell in &row.cells {
                let is_vmerge_continue = cell
                    .properties
                    .as_ref()
                    .is_some_and(|p| matches!(p.v_merge, Some(super::table::VerticalMerge::Continue)));

                let cell_text = if is_vmerge_continue {
                    String::new()
                } else {
                    cell.paragraphs
                        .iter()
                        .map(&render)
                        .collect::<Vec<_>>()
                        .join(" ")
                        .trim()
                        .to_string()
                };
                row_cells.push(cell_text);

                let span = cell.properties.as_ref().and_then(|p| p.grid_span).unwrap_or(1);
                for _ in 1..span {
                    row_cells.push(String::new());
                }
            }
            cells.push(row_cells);
        }
        cells
    }

    /// Build the per-cell paragraph style ids for this table, in the exact layout
    /// [`Table::to_cell_grid`] produces.
    ///
    /// Same origin-once rule: a `gridSpan`/`vMerge` cell contributes its style at its origin and
    /// `None` in every column it covers, so this grid indexes cell-for-cell against the text grid
    /// and the two cannot drift apart. A cell takes the first style any of its paragraphs
    /// declares, which is the banner-row case GH#1587 is about -- a single heading-styled
    /// paragraph in a merged row-0 cell.
    ///
    /// Returns style *ids* (`"Heading2"`); resolving those to an outline level and a display name
    /// needs the document's `StyleCatalog`, which lives on the parser, not on `Table`. ~keep
    pub(crate) fn to_cell_style_grid(&self) -> Vec<Vec<Option<String>>> {
        let mut styles: Vec<Vec<Option<String>>> = Vec::new();
        for row in &self.rows {
            let mut row_styles = Vec::new();
            for cell in &row.cells {
                let is_vmerge_continue = cell
                    .properties
                    .as_ref()
                    .is_some_and(|p| matches!(p.v_merge, Some(super::table::VerticalMerge::Continue)));

                let style = if is_vmerge_continue {
                    None
                } else {
                    cell.paragraphs.iter().find_map(|p| p.style.clone())
                };
                row_styles.push(style);

                let span = cell.properties.as_ref().and_then(|p| p.grid_span).unwrap_or(1);
                for _ in 1..span {
                    row_styles.push(None);
                }
            }
            styles.push(row_styles);
        }
        styles
    }

    /// Render this table as a markdown table.
    ///
    /// Uses table row and cell properties to improve formatting:
    /// - Respects `RowProperties.is_header` to identify header rows
    /// - Handles `CellProperties.grid_span` to account for merged cells
    ///
    /// If no explicit header row is marked, treats the first row as the header.
    pub(crate) fn to_markdown(&self) -> String {
        if self.rows.is_empty() {
            return String::new();
        }

        let cells = self.to_cell_grid(Paragraph::runs_to_markdown);

        if cells.is_empty() {
            return String::new();
        }

        let num_cols = cells.iter().map(|r| r.len()).max().unwrap_or(0);
        if num_cols == 0 {
            return String::new();
        }

        let mut col_widths = vec![3usize; num_cols];
        for row in &cells {
            for (i, cell) in row.iter().enumerate() {
                col_widths[i] = col_widths[i].max(cell.len());
            }
        }

        let header_row_index = self
            .rows
            .iter()
            .position(|row| row.properties.as_ref().map(|p| p.is_header).unwrap_or(false))
            .unwrap_or(0);

        let mut md = String::new();

        for (row_idx, row) in cells.iter().enumerate() {
            md.push('|');
            for (i, cell) in row.iter().enumerate() {
                let width = col_widths.get(i).copied().unwrap_or(3);
                md.push_str(&format!(" {:width$} |", cell, width = width));
            }
            for i in row.len()..num_cols {
                let width = col_widths.get(i).copied().unwrap_or(3);
                md.push_str(&format!(" {:width$} |", "", width = width));
            }
            md.push('\n');

            if row_idx == header_row_index {
                md.push('|');
                for i in 0..num_cols {
                    let width = col_widths.get(i).copied().unwrap_or(3);
                    md.push_str(&format!(" {} |", "-".repeat(width)));
                }
                md.push('\n');
            }
        }

        md.trim_end().to_string()
    }

    /// Render this table as plain text with tab-separated cells.
    pub(crate) fn to_plain_text(&self) -> String {
        if self.rows.is_empty() {
            return String::new();
        }

        let cells = self.to_cell_grid(Paragraph::to_text);

        crate::extraction::cells_to_text(&cells)
    }
}

/// Context for tracking nested table parsing state.
///
/// Each level of table nesting gets its own context on the stack,
/// allowing arbitrary nesting depth (e.g. tables within table cells).
struct TableContext {
    table: Table,
    current_row: Option<TableRow>,
    current_cell: Option<TableCell>,
    paragraph: Option<Paragraph>,
    /// Ordinal of the most recently opened `<w:tr>` in this table (0 before the first
    /// row opens), incremented on every `<w:tr>` `Event::Start`.
    ///
    /// Together with [`Self::cell_ordinal`], lets a page-break handler firing inside a
    /// cell identify which row *and which cell of that row* it belongs to, so a
    /// `lastRenderedPageBreak` hint Word duplicated into another cell of the same
    /// straddling row can be recognized as an echo of the same physical break — while a
    /// row deep enough that one cell alone spans several page breaks still counts each
    /// of that cell's own hints separately (#1592).
    row_ordinal: u32,
    /// Ordinal of the most recently opened `<w:tc>` in this table (0 before the first
    /// cell opens), incremented on every `<w:tc>` `Event::Start`. Monotonic across the
    /// whole table, not reset per row, so every cell instance gets a distinct id. See
    /// [`Self::row_ordinal`].
    cell_ordinal: u32,
    /// `w:gridAfter` of the open row, padded in when the row closes.
    pending_grid_after: u32,
}

impl TableContext {
    fn new() -> Self {
        Self {
            table: Table::new(),
            current_row: None,
            current_cell: None,
            paragraph: None,
            row_ordinal: 0,
            cell_ordinal: 0,
            pending_grid_after: 0,
        }
    }
}

/// Output of [`DocxParser::parse_body_elements`], the element-dispatch loop shared by
/// the main document body, headers/footers, and footnotes/endnotes (#85).
///
/// Callers pick the fields relevant to their content type; e.g. `HeaderFooter` only
/// keeps `paragraphs`/`tables`, while `Note` keeps just `paragraphs` (any top-level
/// table content is flattened into it, matching how a table nested in a table cell
/// is already flattened elsewhere in this parser).
#[derive(Debug, Default)]
struct BodyParseOutputs {
    paragraphs: Vec<Paragraph>,
    note_refs_by_paragraph: AHashMap<usize, Vec<NoteRef>>,
    tables: Vec<Table>,
    drawings: Vec<super::drawing::Drawing>,
    elements: Vec<DocumentElement>,
    sections: Vec<ParsedSection>,
    /// True when a malformed placement made section-to-element ownership ambiguous.
    ambiguous_sections: bool,
    revisions: Vec<crate::types::revisions::DocumentRevision>,
    /// `w:id` values from `w:commentReference` markers encountered in this content,
    /// for validation against `word/comments.xml` (#82).
    comment_ref_ids: Vec<String>,
}

/// Section properties plus the exclusive end of the element range they govern.
///
/// Word stores a section's properties at its end: either inside the final
/// paragraph's `w:pPr`, or as the final body-level `w:sectPr`. Keeping that
/// endpoint lets post-parse layout heuristics select the section actually in
/// force without exposing a rendering-only marker in [`DocumentElement`].
#[derive(Debug)]
struct ParsedSection {
    properties: super::section::SectionProperties,
    end_element_index: Option<usize>,
}

/// Apply run-level formatting from run property child elements.
///
/// Handles `<w:b>`, `<w:i>`, `<w:u>`, `<w:strike>`, `<w:dstrike>`,
/// `<w:vertAlign>`, `<w:sz>`, `<w:color>`, and `<w:highlight>`.
/// Works for both `Event::Start` and `Event::Empty` events.
fn apply_run_formatting(e: &BytesStart, current_run: &mut Option<Run>) {
    if let Some(run) = current_run {
        match e.name().as_ref() {
            "w:b" => run.bold = is_format_enabled(e),
            "w:i" => run.italic = is_format_enabled(e),
            "w:u" => run.underline = is_format_enabled(e),
            "w:strike" | "w:dstrike" => run.strikethrough = is_format_enabled(e),
            "w:vertAlign" => {
                if let Some(val) = get_val_attr_string(e) {
                    match val.as_str() {
                        "subscript" => {
                            run.subscript = true;
                            run.superscript = false;
                        }
                        "superscript" => {
                            run.superscript = true;
                            run.subscript = false;
                        }
                        _ => {
                            run.subscript = false;
                            run.superscript = false;
                        }
                    }
                }
            }
            "w:sz" => {
                if let Some(val) = get_val_attr(e) {
                    run.font_size = Some(val as u32);
                }
            }
            "w:color" => {
                if let Some(val) = get_val_attr_string(e)
                    && val != "auto"
                    && val.len() == 6
                    && val.chars().all(|c| c.is_ascii_hexdigit())
                {
                    run.font_color = Some(val);
                }
            }
            "w:highlight" => {
                if let Some(val) = get_val_attr_string(e) {
                    const VALID_HIGHLIGHTS: &[&str] = &[
                        "yellow",
                        "green",
                        "cyan",
                        "magenta",
                        "blue",
                        "red",
                        "darkBlue",
                        "darkCyan",
                        "darkGreen",
                        "darkMagenta",
                        "darkRed",
                        "darkYellow",
                        "darkGray",
                        "lightGray",
                        "black",
                        "none",
                    ];
                    if VALID_HIGHLIGHTS.contains(&val.as_str()) {
                        run.highlight = Some(val);
                    }
                }
            }
            _ => {}
        }
    }
}

fn collect_run_property_change(e: &BytesStart, changes: &mut Vec<crate::types::revisions::PropertyChange>) {
    let (name, from) = match e.name().as_ref() {
        "w:b" => ("bold", Some(is_format_enabled(e).to_string())),
        "w:i" => ("italic", Some(is_format_enabled(e).to_string())),
        "w:u" => ("underline", Some(is_format_enabled(e).to_string())),
        "w:strike" | "w:dstrike" => ("strikethrough", Some(is_format_enabled(e).to_string())),
        "w:vertAlign" => ("vertical_align", get_val_attr_string(e)),
        "w:sz" => ("font_size", get_val_attr(e).map(|v| v.to_string())),
        "w:color" => ("font_color", get_val_attr_string(e)),
        "w:highlight" => ("highlight", get_val_attr_string(e)),
        _ => return,
    };

    if let Some(change) = changes.iter_mut().find(|change| change.name == name) {
        change.from = from;
        return;
    }

    changes.push(crate::types::revisions::PropertyChange {
        name: name.to_string(),
        from,
        to: None,
    });
}

fn run_property_value(run: &Run, name: &str) -> Option<String> {
    match name {
        "bold" => Some(run.bold.to_string()),
        "italic" => Some(run.italic.to_string()),
        "underline" => Some(run.underline.to_string()),
        "strikethrough" => Some(run.strikethrough.to_string()),
        "vertical_align" => {
            if run.subscript {
                Some("subscript".to_string())
            } else if run.superscript {
                Some("superscript".to_string())
            } else {
                None
            }
        }
        "font_size" => run.font_size.map(|v| v.to_string()),
        "font_color" => run.font_color.clone(),
        "highlight" => run.highlight.clone(),
        _ => None,
    }
}

fn push_current_run_property_changes(run: &Run, changes: &mut Vec<crate::types::revisions::PropertyChange>) {
    let current_properties = [
        ("bold", run.bold.then_some("true".to_string())),
        ("italic", run.italic.then_some("true".to_string())),
        ("underline", run.underline.then_some("true".to_string())),
        ("strikethrough", run.strikethrough.then_some("true".to_string())),
        (
            "vertical_align",
            if run.subscript {
                Some("subscript".to_string())
            } else if run.superscript {
                Some("superscript".to_string())
            } else {
                None
            },
        ),
        ("font_size", run.font_size.map(|v| v.to_string())),
        ("font_color", run.font_color.clone()),
        ("highlight", run.highlight.clone()),
    ];

    for (name, to) in current_properties {
        let Some(to) = to else {
            continue;
        };
        if changes.iter().any(|change| change.name == name) {
            continue;
        }
        changes.push(crate::types::revisions::PropertyChange {
            name: name.to_string(),
            from: None,
            to: Some(to),
        });
    }
}

fn finalize_run_property_changes(
    mut changes: Vec<crate::types::revisions::PropertyChange>,
    current_run: Option<&Run>,
) -> Vec<crate::types::revisions::PropertyChange> {
    if let Some(run) = current_run {
        for change in &mut changes {
            change.to = run_property_value(run, &change.name);
        }
        push_current_run_property_changes(run, &mut changes);
    }

    changes.into_iter().filter(|change| change.from != change.to).collect()
}

fn push_format_revision(
    revisions: &mut Vec<crate::types::revisions::DocumentRevision>,
    attrs: (Option<String>, Option<String>, Option<String>),
    property_changes: Vec<crate::types::revisions::PropertyChange>,
    current_run: Option<&Run>,
    current_paragraph_index: usize,
    revision_id_counter: &mut usize,
) {
    let (id_opt, author, timestamp) = attrs;
    let revision_id = id_opt.unwrap_or_else(|| {
        let fallback = format!("docx-fmt-{}", *revision_id_counter);
        *revision_id_counter += 1;
        fallback
    });

    revisions.push(crate::types::revisions::DocumentRevision {
        revision_id,
        author,
        timestamp,
        kind: crate::types::revisions::RevisionKind::FormatChange,
        anchor: Some(crate::types::revisions::RevisionAnchor::Paragraph {
            index: current_paragraph_index,
        }),
        delta: crate::types::revisions::RevisionDelta {
            property_changes: finalize_run_property_changes(property_changes, current_run),
            ..Default::default()
        },
    });
}

/// Copy numbering a paragraph inherits from its style onto the paragraph itself.
///
/// A paragraph is recognised as a list item through `Paragraph::numbering_id`, and
/// two separate readers ask that question: the markdown rendering in this module,
/// and the document-structure node builder in `extractors/docx.rs`. Resolving the
/// inheritance once, here, is what keeps the two answers the same; resolving it at
/// either reader would fix one and leave the other emitting plain paragraphs.
///
/// Word lets a style own the numbering reference rather than the paragraph, which
/// is how the built-in `List Bullet` and `List Number` styles work, so a list
/// authored with them carries no `w:numPr` in `document.xml` at all.
fn apply_style_numbering(catalog: &super::styles::StyleCatalog, document: &mut Document) {
    fn fill(catalog: &super::styles::StyleCatalog, paragraphs: &mut [Paragraph]) {
        for para in paragraphs {
            if para.numbering_id.is_some() {
                continue;
            }
            let Some(style_id) = para.style.as_deref() else {
                continue;
            };
            let Some((numbering_id, level)) = resolve_style_numbering(catalog, style_id) else {
                continue;
            };
            para.numbering_id = Some(numbering_id);
            para.numbering_level = Some(para.numbering_level.unwrap_or(level));
        }
    }

    fn fill_tables(catalog: &super::styles::StyleCatalog, tables: &mut [Table]) {
        for table in tables {
            for row in &mut table.rows {
                for cell in &mut row.cells {
                    fill(catalog, &mut cell.paragraphs);
                }
            }
        }
    }

    fill(catalog, &mut document.paragraphs);
    fill_tables(catalog, &mut document.tables);
    for header_footer in document.headers.iter_mut().chain(document.footers.iter_mut()) {
        fill(catalog, &mut header_footer.paragraphs);
        fill_tables(catalog, &mut header_footer.tables);
    }
    for note in document.footnotes.iter_mut().chain(document.endnotes.iter_mut()) {
        fill(catalog, &mut note.paragraphs);
    }
    for comment in &mut document.comments {
        fill(catalog, &mut comment.paragraphs);
    }
}

/// Walk a style's `basedOn` chain for the first numbering reference it carries.
///
/// The level is defaulted to 0 when the style sets `w:numId` without `w:ilvl`, which
/// is the common shape and is how Word reads it. That default is load-bearing:
/// `Paragraph::to_markdown` needs both values before it treats a paragraph as a list
/// item, so inheriting the id alone would change nothing.
///
/// The chain is bounded at 20 like `resolve_heading_level`, for the same cycles.
fn resolve_style_numbering(catalog: &super::styles::StyleCatalog, style_id: &str) -> Option<(i64, i64)> {
    let mut current_id = Some(style_id);
    let mut visited = 0;
    while let Some(id) = current_id {
        if visited > 20 {
            break;
        }
        visited += 1;
        let style_def = catalog.styles.get(id)?;
        if let Some(numbering_id) = style_def.paragraph_properties.numbering_id {
            let level = style_def
                .paragraph_properties
                .numbering_level
                .map(clamp_numbering_level)
                .unwrap_or(0);
            return Some((numbering_id, level));
        }
        current_id = style_def.based_on.as_deref();
    }
    None
}

/// Maximum indentation depth honoured for a `w:ilvl` (list nesting level).
///
/// Word's own list-formatting UI caps nesting at 9 levels (`w:ilvl` 0-8); this is also
/// the practical ceiling for `Paragraph::to_markdown`'s two-space-per-level indent and
/// for the internal-document builder's per-level `push_list` loop
/// (`extractors/docx.rs::build_internal_document`). `w:ilvl` is a bare `.parse::<i64>()`
/// of document-controlled text with no sign check or upper bound, so without this clamp
/// a crafted `w:val="-1"` turns into `usize::MAX` in the indent's `"  ".repeat(level as
/// usize)` (capacity-overflow panic), and `w:val="10000000000"` turns into a ~20 GB
/// allocation attempt (uncatchable `handle_alloc_error` abort) or, via the list-depth
/// loop, billions of `push_list` calls.
const MAX_LIST_NESTING_LEVEL: i64 = 8;

/// Clamp a raw `w:ilvl` value into the plausible `0..=MAX_LIST_NESTING_LEVEL` range.
///
/// A negative or absurdly large indentation level is malformed input, not a list depth
/// to be honoured verbatim: this clamps rather than rejecting the whole paragraph, so a
/// document with one garbage `w:ilvl` still extracts its text and list structure (just
/// pinned to the nearest valid depth), matching the crate's "preserve partial results on
/// failure" extraction-safety rule instead of discarding an otherwise-good paragraph.
fn clamp_numbering_level(level: i64) -> i64 {
    level.clamp(0, MAX_LIST_NESTING_LEVEL)
}

/// Byte range of `text` with its leading and trailing blank lines removed, keeping the
/// indentation of the first content line intact.
///
/// `Paragraph::to_markdown` encodes list nesting depth as leading spaces
/// (`"  ".repeat(level)`), so the blanket `trim_start` this replaced silently flattened
/// any list whose first item was also the document's first line: a list starting at
/// `w:ilvl="8"` rendered with zero indentation while the same list preceded by one
/// ordinary paragraph rendered with all sixteen spaces.
pub(crate) fn blank_line_trim_range(text: &str) -> (usize, usize) {
    let end = text.trim_end().len();
    let head = &text[..end];
    let start = head
        .char_indices()
        .find(|(_, character)| !character.is_whitespace())
        .map_or(end, |(index, _)| {
            head[..index].rfind('\n').map_or(0, |newline| newline + 1)
        });
    (start, end)
}

/// `text` with its leading and trailing blank lines removed, preserving the first
/// content line's indentation. See [`blank_line_trim_range`].
pub(crate) fn trim_blank_lines(text: &str) -> &str {
    let (start, end) = blank_line_trim_range(text);
    &text[start..end]
}

/// Apply paragraph-level properties from a `<w:pStyle>`, `<w:ilvl>`, or `<w:numId>` element.
///
/// Resolves the correct paragraph (table context vs top-level) automatically.
fn apply_paragraph_property(
    e: &BytesStart,
    table_stack: &mut [TableContext],
    current_paragraph: &mut Option<Paragraph>,
) {
    let para = if let Some(ctx) = table_stack.last_mut() {
        ctx.paragraph.as_mut()
    } else {
        current_paragraph.as_mut()
    };

    if let Some(para) = para {
        match e.name().as_ref() {
            "w:pStyle" => para.style = get_val_attr_string(e),
            "w:ilvl" => para.numbering_level = get_val_attr(e).map(clamp_numbering_level),
            "w:numId" => para.numbering_id = get_val_attr(e),
            _ => {}
        }
    }
}

/// Push a run into whichever paragraph is currently open: a table cell's last
/// paragraph, an in-progress table-cell paragraph, or the top-level paragraph.
/// Shared by math-formula runs and by the `</w:r>` end-tag handler.
fn push_run_to_current(table_stack: &mut [TableContext], current_paragraph: &mut Option<Paragraph>, run: Run) {
    if let Some(ctx) = table_stack.last_mut() {
        if let Some(ref mut para) = ctx.paragraph {
            para.add_run(run);
        } else if let Some(ref mut cell) = ctx.current_cell {
            if cell.paragraphs.is_empty() {
                cell.paragraphs.push(Paragraph::new());
            }
            if let Some(para) = cell.paragraphs.last_mut() {
                para.add_run(run);
            }
        }
    } else if let Some(para) = current_paragraph {
        para.add_run(run);
    }
}

/// Handle a `<w:fldChar>` event (identical whether it arrives as `Event::Start` or
/// `Event::Empty`; `w:fldChar` never has children).
///
/// Tracks the begin/separate/end field-character state machine and, on `separate`,
/// extracts a `HYPERLINK` field's URL from the instruction text accumulated in
/// `field_instruction` so the following result run(s) pick it up via
/// `current_hyperlink_url`, the same mechanism `<w:hyperlink>` uses (#88, #239).
fn apply_fld_char(
    e: &BytesStart,
    in_field_instruction: &mut bool,
    field_instruction: &mut String,
    current_hyperlink_url: &mut Option<String>,
    field_hyperlink_stack: &mut Vec<Option<String>>,
    toc: &mut TocState,
) {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == "w:fldCharType" {
            match attr.value.as_ref() {
                "begin" => {
                    *in_field_instruction = true;
                    field_instruction.clear();
                    toc.field_begin();
                }
                "separate" => {
                    *in_field_instruction = false;
                    toc.field_separate(field_instruction);
                    let url = extract_hyperlink_field_url(field_instruction);
                    field_hyperlink_stack.push(current_hyperlink_url.clone());
                    if url.is_some() {
                        *current_hyperlink_url = url;
                    }
                }
                "end" => {
                    *in_field_instruction = false;
                    toc.field_end();
                    if let Some(saved) = field_hyperlink_stack.pop() {
                        *current_hyperlink_url = saved;
                    }
                }
                _ => {}
            }
        }
    }
}

/// `w:sdtPr/w:docPartObj/w:docPartGallery/@w:val` value that marks a structured
/// document tag as a Word-generated table of contents.
const TOC_DOC_PART_GALLERY: &str = "Table of Contents";

/// Bookmark Word writes to remember the last edit position. Never a link target.
const GO_BACK_BOOKMARK: &str = "_GoBack";

/// True when a field instruction is a table-of-contents field (`TOC \o "1-3" \h \z \u`).
///
/// Only the leading keyword is matched: the switches vary per document and carry no
/// membership information. `TOA` (table of authorities) and `TC` (a TOC *entry* marker
/// placed on the target, not on the entry) are deliberately not matched.
fn is_toc_field_instruction(instruction: &str) -> bool {
    instruction
        .split_whitespace()
        .next()
        .is_some_and(|keyword| keyword.eq_ignore_ascii_case("TOC"))
}

/// Tracks whether the element-dispatch loop is currently inside a table of contents.
///
/// Word marks a generated TOC two independent ways, and normally emits both:
///
/// - a `w:sdt` structured document tag whose `w:sdtPr/w:docPartObj/w:docPartGallery`
///   has `w:val="Table of Contents"` — the reliable marker when present;
/// - a `TOC` field code, written either as `w:fldSimple/@w:instr` or as the
///   `w:fldChar` begin/separate/end triple with the instruction in `w:instrText` —
///   the only marker for a TOC that is not wrapped in an `sdt`.
///
/// The field-code form needs real nesting bookkeeping: a TOC field's *result* contains
/// one nested `PAGEREF` field per entry, so the region must close on the TOC field's own
/// `end`, not on the first `end` that arrives.
#[derive(Debug, Default)]
struct TocState {
    /// One entry per currently-open `w:sdt`; `true` once its gallery said TOC.
    sdt_stack: Vec<bool>,
    /// Number of `true` entries in `sdt_stack`, so [`Self::active`] stays O(1).
    open_toc_sdts: usize,
    /// One entry per currently-open `w:fldSimple`; `true` when its instruction is TOC.
    fld_simple_stack: Vec<bool>,
    /// Number of `true` entries in `fld_simple_stack`.
    open_toc_fld_simple: usize,
    /// Current `w:fldChar` begin/end nesting depth.
    field_depth: usize,
    /// `field_depth` of the TOC field code currently open, if any.
    toc_field_depth: Option<usize>,
}

impl TocState {
    /// True when content parsed right now belongs to a table of contents.
    fn active(&self) -> bool {
        self.open_toc_sdts > 0 || self.open_toc_fld_simple > 0 || self.toc_field_depth.is_some()
    }

    fn open_sdt(&mut self) {
        self.sdt_stack.push(false);
    }

    fn close_sdt(&mut self) {
        if self.sdt_stack.pop() == Some(true) {
            self.open_toc_sdts = self.open_toc_sdts.saturating_sub(1);
        }
    }

    /// Apply a `w:docPartGallery` to the innermost open `w:sdt`.
    fn apply_doc_part_gallery(&mut self, e: &BytesStart) {
        let is_toc = get_val_attr_string(e).is_some_and(|val| val.trim().eq_ignore_ascii_case(TOC_DOC_PART_GALLERY));
        if !is_toc {
            return;
        }
        if let Some(flag) = self.sdt_stack.last_mut()
            && !*flag
        {
            *flag = true;
            self.open_toc_sdts += 1;
        }
    }

    fn open_fld_simple(&mut self, instruction: Option<&str>) {
        let is_toc = instruction.is_some_and(is_toc_field_instruction);
        self.fld_simple_stack.push(is_toc);
        if is_toc {
            self.open_toc_fld_simple += 1;
        }
    }

    fn close_fld_simple(&mut self) {
        if self.fld_simple_stack.pop() == Some(true) {
            self.open_toc_fld_simple = self.open_toc_fld_simple.saturating_sub(1);
        }
    }

    fn field_begin(&mut self) {
        self.field_depth += 1;
    }

    /// A field's instruction is complete at `separate`; that is where the result
    /// content starts, so that is where a TOC region opens.
    fn field_separate(&mut self, instruction: &str) {
        if self.toc_field_depth.is_none() && is_toc_field_instruction(instruction) {
            self.toc_field_depth = Some(self.field_depth);
        }
    }

    fn field_end(&mut self) {
        if self.toc_field_depth == Some(self.field_depth) {
            self.toc_field_depth = None;
        }
        self.field_depth = self.field_depth.saturating_sub(1);
    }
}

/// Resolve whichever paragraph is currently open — a table cell's or the top-level one —
/// exactly as [`apply_paragraph_property`] does.
fn current_paragraph_mut<'a>(
    table_stack: &'a mut [TableContext],
    current_paragraph: &'a mut Option<Paragraph>,
) -> Option<&'a mut Paragraph> {
    if let Some(ctx) = table_stack.last_mut() {
        ctx.paragraph.as_mut()
    } else {
        current_paragraph.as_mut()
    }
}

/// Mark the open paragraph as part of a table of contents.
///
/// Called both when a paragraph opens inside an already-active TOC region and right
/// after a `w:fldChar` moves the parser into one: Word puts the TOC field's `begin`
/// inside the first entry's paragraph, so a check made only at `<w:p>` would miss it.
/// The flag is never cleared, which is what lets the *last* entry stay marked even
/// though its `</w:p>` arrives after the field's `end`.
fn mark_paragraph_in_toc(table_stack: &mut [TableContext], current_paragraph: &mut Option<Paragraph>) {
    if let Some(para) = current_paragraph_mut(table_stack, current_paragraph) {
        para.in_table_of_contents = true;
    }
}

/// Record a `w:bookmarkStart` on the open paragraph so internal `w:hyperlink w:anchor`
/// references — every entry of a generated TOC — have something to resolve against.
fn apply_bookmark_start(e: &BytesStart, table_stack: &mut [TableContext], current_paragraph: &mut Option<Paragraph>) {
    let Some(name) = e
        .attributes()
        .flatten()
        .find(|attr| attr.key.as_ref() == "w:name")
        .map(|attr| attr.value.to_string())
    else {
        return;
    };
    if name.is_empty() || name == GO_BACK_BOOKMARK {
        return;
    }
    if let Some(para) = current_paragraph_mut(table_stack, current_paragraph) {
        para.bookmarks.push(name);
    }
}

/// Page-break bookkeeping threaded through the `<w:br>` and `<w:lastRenderedPageBreak>`
/// handlers.
///
/// These fields are one piece of state: every handler that touches any of them touches
/// all of them, and they are only meaningful relative to one another.
#[derive(Debug, Default)]
struct PageBreakState {
    /// Breaks seen inside a table, flushed once the outermost `</w:tbl>` closes (#1419).
    pending_table: u32,
    /// Breaks seen after text in the current paragraph, flushed once its `</w:p>` closes.
    pending_paragraph: u32,
    /// Whether real text has been emitted since the last recorded break (#1416).
    ///
    /// Starts `true` so a break with nothing before it is still recorded.
    text_since_break: bool,
    /// `(table nesting depth, row ordinal, cell ordinal)` of the most recent table-scoped
    /// page break event seen — hint or authored, suppressed or recorded.
    ///
    /// Word writes `w:lastRenderedPageBreak` into *every* cell of a row that straddles a
    /// page boundary: one physical break, one hint per cell. [`push_or_defer_page_break`]
    /// treats a hint as that same duplicated echo — and skips it — only when it shares
    /// the *row* of this position but names a *different cell*; a hint that shares both
    /// the row and the cell (a row deep enough that one cell alone spans several breaks)
    /// is a genuinely new transition and still counts (#1592). Updated on every
    /// table-scoped break regardless of whether it was suppressed, so a chain of
    /// alternating duplicate/genuine hints across several cells of one row is tracked
    /// correctly. Cleared whenever `pending_table` is flushed, since the identity is only
    /// meaningful relative to the table currently being deferred.
    last_table_break: Option<(usize, u32, u32)>,
}

fn section_text_column_height_emu(section: &super::section::SectionProperties) -> Option<i64> {
    const EMUS_PER_TWIP: i64 = super::EMUS_PER_INCH / 1440;

    let page_height = i64::from(section.page_height_twips?);
    let top = i64::from(section.margins.top?);
    let bottom = i64::from(section.margins.bottom?);
    let usable_twips = page_height - top - bottom;
    (usable_twips > 0).then(|| usable_twips * EMUS_PER_TWIP)
}

/// Insert conservative page breaks where inline drawings alone cannot fit in
/// the text column of the section that owns them (#1559).
fn insert_missing_inline_drawing_page_breaks(
    elements: &mut Vec<DocumentElement>,
    drawings: &[super::drawing::Drawing],
    sections: &[ParsedSection],
    ambiguous_sections: bool,
) {
    if ambiguous_sections || sections.is_empty() {
        return;
    }

    let original = std::mem::take(elements);
    let Some(ends) = sections
        .iter()
        .map(|section| section.end_element_index)
        .collect::<Option<Vec<_>>>()
    else {
        *elements = original;
        return;
    };
    if ends.windows(2).any(|pair| pair[0] > pair[1]) || ends.last().is_some_and(|end| *end > original.len()) {
        *elements = original;
        return;
    }

    let mut rebuilt = Vec::with_capacity(original.len());
    let mut start = 0usize;
    for (section, end) in sections.iter().zip(ends) {
        let Some(capacity) = section_text_column_height_emu(&section.properties) else {
            rebuilt.extend_from_slice(&original[start..end]);
            start = end;
            continue;
        };

        let mut used_height = 0i64;
        for element in &original[start..end] {
            match element {
                DocumentElement::PageBreak => used_height = 0,
                DocumentElement::Drawing(index) => {
                    let height = drawings
                        .get(*index)
                        .filter(|drawing| matches!(drawing.drawing_type, super::drawing::DrawingType::Inline))
                        .and_then(|drawing| drawing.extent.as_ref())
                        .map(|extent| extent.cy)
                        .filter(|height| *height > 0);
                    if let Some(height) = height {
                        let next_height = used_height.saturating_add(height);
                        if used_height > 0 && next_height > capacity {
                            rebuilt.push(DocumentElement::PageBreak);
                            used_height = height;
                        } else {
                            used_height = next_height;
                        }
                    }
                }
                _ => {}
            }
            rebuilt.push(element.clone());
        }
        start = end;
    }
    rebuilt.extend_from_slice(&original[start..]);
    *elements = rebuilt;
}

/// Push a page-break marker, or defer it, depending on where it was encountered.
///
/// Inside a table it is always deferred to [`PageBreakState::pending_table`], flushed
/// once the outermost `</w:tbl>` closes and the table element has been pushed (#1419),
/// since a form feed cannot be written into the middle of a table that renders as a
/// single markdown block.
///
/// Word writes `w:lastRenderedPageBreak` into *every* cell of a row that straddles a
/// page boundary — one physical break, one hint per cell — so a table-deferred break
/// also carries a position: table nesting depth plus [`TableContext::row_ordinal`] and
/// [`TableContext::cell_ordinal`]. A *hint* (`is_hint = true`) that shares its row and
/// depth with [`PageBreakState::last_table_break`] but names a *different* cell is that
/// duplicated echo and is skipped; one that shares row, depth, *and* cell — a row deep
/// enough that a single cell alone spans several page breaks — is a genuinely new
/// transition and is not skipped. An authored `<w:br w:type="page"/>` (`is_hint =
/// false`) is never skipped this way — two deliberate breaks in the same row are
/// legitimate — but its position is still recorded so a later hint echoing it is
/// recognized (#1592).
///
/// Outside a table, `elements` only gains a `DocumentElement::Paragraph` entry for
/// the paragraph currently being parsed once its `</w:p>` closes — so a break
/// encountered before any text has been collected into that paragraph can be pushed
/// immediately (it correctly precedes the not-yet-pushed paragraph), but a break
/// encountered *after* text already collected in that same paragraph must not jump
/// ahead of its own paragraph's entry. That case is deferred to
/// [`PageBreakState::pending_paragraph`] and flushed right after the paragraph is
/// pushed, mirroring the table deferral above.
fn push_or_defer_page_break(
    table_stack: &[TableContext],
    current_run: Option<&Run>,
    current_paragraph: &Option<Paragraph>,
    elements: &mut Vec<DocumentElement>,
    page_breaks: &mut PageBreakState,
    is_hint: bool,
) {
    if let Some(row_context) = table_stack.last() {
        let position = (table_stack.len(), row_context.row_ordinal, row_context.cell_ordinal);
        let is_duplicate_row_hint = is_hint
            && page_breaks
                .last_table_break
                .is_some_and(|(depth, row, cell)| depth == position.0 && row == position.1 && cell != position.2);
        // Track the position of every table-scoped break seen, suppressed or not: a
        // duplicate hint in cell 2 must not hide a genuine later break in cell 3 behind
        // cell 1's stale position.
        page_breaks.last_table_break = Some(position);
        if is_duplicate_row_hint {
            // Word's per-cell echo of the row's break: the physical break was already
            // counted from a different cell of this same row.
            return;
        }
        page_breaks.pending_table += 1;
        return;
    }

    let paragraph_has_content = current_run.is_some_and(|run| !run.text.is_empty())
        || current_paragraph
            .as_ref()
            .is_some_and(|paragraph| !paragraph.runs.is_empty());

    if paragraph_has_content {
        page_breaks.pending_paragraph += 1;
    } else {
        elements.push(DocumentElement::PageBreak);
    }
}

/// Handle a `<w:br>` event (`Event::Start` or `Event::Empty`).
///
/// A `page`-type break outside a table records a `DocumentElement::PageBreak`; any
/// other break type (`column`, `textWrapping`, or no `w:type` at all) inserts a
/// newline into the current run instead (#224).
///
/// See [`push_or_defer_page_break`] for how (and when) the marker is placed relative
/// to its enclosing table or paragraph.
///
/// This is the author's own explicit break, so it is always recorded — unlike
/// `w:lastRenderedPageBreak` (see [`apply_last_rendered_page_break`]), it is never
/// suppressed as a duplicate.
fn apply_break(
    e: &BytesStart,
    table_stack: &[TableContext],
    current_run: &mut Option<Run>,
    current_paragraph: &Option<Paragraph>,
    elements: &mut Vec<DocumentElement>,
    page_breaks: &mut PageBreakState,
) {
    let mut is_page_break = false;
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == "w:type" && attr.value.as_ref() == "page" {
            is_page_break = true;
            break;
        }
    }

    if is_page_break {
        push_or_defer_page_break(
            table_stack,
            current_run.as_ref(),
            current_paragraph,
            elements,
            page_breaks,
            false,
        );
        page_breaks.text_since_break = false;
    } else if let Some(run) = current_run {
        run.text.push('\n');
    }
}

/// Handle a `w:tab` or `w:ptab` run element (`Event::Start` or `Event::Empty`): written
/// self-closing or as a start/end pair, either one stands for a tab character. A `w:tab`
/// that defines a tab stop under `w:pPr/w:tabs` comes before the paragraph's first run, so
/// there is no run to add to.
fn apply_tab(current_run: &mut Option<Run>, page_breaks: &mut PageBreakState) {
    if let Some(run) = current_run {
        run.text.push('\t');
        page_breaks.text_since_break = true;
    }
}

/// Handle a `<w:lastRenderedPageBreak/>` event (`Event::Start` or `Event::Empty`).
///
/// Word writes this hint at the start of the first run on a page *it* rendered —
/// including, redundantly, right after an authored `<w:br w:type="page"/>` that
/// already recorded the same transition (#1416). It is the only page-break signal
/// in documents Word paginated itself with no manual breaks at all, so it cannot be
/// dropped outright; instead, it is only recorded when real text has been emitted
/// since the previous break, which is exactly the case where it is *not* a redundant
/// echo of a break already counted.
///
/// Inside a table, Word additionally repeats this same hint once per cell of a row
/// that straddles a page boundary (#1592); the `text_since_break` check above cannot
/// tell that apart from a hint in a genuinely new row, since a cell's own text between
/// two hints resets it regardless. [`push_or_defer_page_break`] carries the
/// row-and-cell identity check (`is_hint = true`) that does.
///
/// See [`push_or_defer_page_break`] for how (and when) the marker is placed relative
/// to its enclosing table or paragraph, including the [`PageBreakState::pending_table`]
/// deferral for a hint seen inside a table (#1419).
fn apply_last_rendered_page_break(
    table_stack: &[TableContext],
    current_run: &Option<Run>,
    current_paragraph: &Option<Paragraph>,
    elements: &mut Vec<DocumentElement>,
    page_breaks: &mut PageBreakState,
) {
    if !page_breaks.text_since_break {
        return;
    }
    push_or_defer_page_break(
        table_stack,
        current_run.as_ref(),
        current_paragraph,
        elements,
        page_breaks,
        true,
    );
    page_breaks.text_since_break = false;
}

/// Handle a `<w:sym w:font="…" w:char="…"/>` element (#224).
///
/// `w:char` is a hex code point, interpreted against the named symbol font's own
/// glyph mapping (commonly the U+F0xx Private Use Area for fonts like Wingdings or
/// Symbol). Without per-font glyph tables, the hex value is mapped directly to a
/// Unicode scalar; when that's not a valid scalar, a placeholder is inserted and a
/// `ProcessingWarning` records the degradation (#171).
fn apply_symbol(e: &BytesStart, current_run: &mut Option<Run>, warnings: &mut Vec<crate::types::ProcessingWarning>) {
    let mut font: Option<String> = None;
    let mut code: Option<String> = None;
    for attr in e.attributes().flatten() {
        match attr.key.as_ref() {
            "w:font" => font = Some(attr.value.to_string()),
            "w:char" => code = Some(attr.value.to_string()),
            _ => {}
        }
    }

    let mapped = code
        .as_deref()
        .and_then(|c| u32::from_str_radix(c.trim_start_matches("0x").trim_start_matches("0X"), 16).ok())
        .and_then(char::from_u32);

    let Some(run) = current_run else {
        return;
    };

    match mapped {
        Some(ch) => run.text.push(ch),
        None => {
            run.text.push('\u{FFFD}');
            crate::core::diagnostics::push_warning(
                warnings,
                "docx",
                format!(
                    "Could not map w:sym character code {:?} in font {:?}; inserted a placeholder",
                    code, font
                ),
            );
        }
    }
}

/// Validate archive against ZIP bomb attacks and resource exhaustion.
///
/// Checks:
/// - Maximum uncompressed size per file (100 MB default)
/// - Maximum total number of entries (from `SecurityLimits::max_files_in_archive`,
///   10,000 by default; see `SecurityBudget::from_config`/`from_limits` for how a
///   caller-supplied `ExtractionConfig` overrides the default)
/// - Maximum total uncompressed size (500 MB default)
/// - Maximum compression ratio (`SecurityLimits::max_compression_ratio`, 100:1 by
///   default), delegated to [`ZipBombValidator`] -- every other OOXML/ODF container
///   (XLSX, PPTX, ODT, ODP, HWPX, EPUB, iWork, generic archives) routes its ratio check
///   through the same validator, and DOCX had been the one exception: the two size
///   checks above bound how large a member's *declared* uncompressed size may be, but
///   say nothing about how much smaller its *compressed* size was, so a member that
///   declares (and, being `.take()`-bounded downstream, can actually only inflate to)
///   a size within those limits can still be a compression bomb by ratio.
///
/// `pub(crate)` so the second, metadata-only archive open in
/// `extractors/docx.rs::extract_content` can run the same checks the first (parsing)
/// open does — that second open used to bypass validation entirely (GH security
/// review: unvalidated second archive open).
pub(crate) fn validate_archive_security(
    archive: &mut zip::ZipArchive<impl Read + Seek>,
    limits: &SecurityLimits,
) -> Result<(), DocxParseError> {
    use super::{MAX_TOTAL_UNCOMPRESSED_SIZE, MAX_UNCOMPRESSED_FILE_SIZE};

    if archive.len() > limits.max_files_in_archive {
        return Err(DocxParseError::SecurityLimit(format!(
            "Archive contains {} entries, exceeds limit of {}",
            archive.len(),
            limits.max_files_in_archive
        )));
    }

    let mut total_uncompressed: u64 = 0;
    for i in 0..archive.len() {
        let file = archive
            .by_index_raw(i)
            .map_err(|e| DocxParseError::SecurityLimit(format!("Failed to read ZIP entry {}: {}", i, e)))?;
        let name = file
            .name()
            .map_err(|e| DocxParseError::SecurityLimit(format!("Failed to decode ZIP entry {i}: {e}")))?;
        let size = file.size();
        if size > MAX_UNCOMPRESSED_FILE_SIZE {
            return Err(DocxParseError::SecurityLimit(format!(
                "File '{}' uncompressed size {} bytes exceeds limit of {} bytes",
                name, size, MAX_UNCOMPRESSED_FILE_SIZE
            )));
        }
        total_uncompressed = total_uncompressed.saturating_add(size);
    }

    if total_uncompressed > MAX_TOTAL_UNCOMPRESSED_SIZE {
        return Err(DocxParseError::SecurityLimit(format!(
            "Total uncompressed size {} bytes exceeds limit of {} bytes",
            total_uncompressed, MAX_TOTAL_UNCOMPRESSED_SIZE
        )));
    }

    ZipBombValidator::new(limits.clone()).validate(archive)?;

    Ok(())
}

/// Recognise a conventionally-named header/footer part, e.g. `word/header2.xml`.
///
/// Returns `(is_header, index)`. Used only as the fallback sweep in
/// [`DocxParser::parse_headers_footers`], for parts no relationship declares.
fn conventional_header_footer_part(name: &str) -> Option<(bool, u32)> {
    let stem = name.strip_prefix("word/")?.strip_suffix(".xml")?;
    let (is_header, digits) = match stem.strip_prefix("header") {
        Some(rest) => (true, rest),
        None => (false, stem.strip_prefix("footer")?),
    };
    // Reject `word/headerReference.xml` and friends: only a bare numeric suffix is
    // the conventional part naming.
    digits.parse::<u32>().ok().map(|index| (is_header, index))
}

#[derive(Debug)]
struct DocxParser<R: Read + Seek> {
    archive: zip::ZipArchive<R>,
    relationships: AHashMap<String, String>,
    styles: Option<super::styles::StyleCatalog>,
    theme: Option<super::theme::Theme>,
}

impl<R: Read + Seek> DocxParser<R> {
    /// `limits` is the caller's configured `SecurityLimits` (see `parse_document`), not a
    /// hardcoded ceiling.
    fn new(reader: R, limits: &SecurityLimits) -> Result<Self, DocxParseError> {
        let mut archive = zip::ZipArchive::new(reader)?;
        validate_archive_security(&mut archive, limits)?;

        let styles = {
            let mut styles_result = None;
            if let Ok(file) = archive.by_name("word/styles.xml") {
                let mut xml = String::new();
                if file
                    .take(super::MAX_UNCOMPRESSED_FILE_SIZE)
                    .read_to_string(&mut xml)
                    .is_ok()
                {
                    styles_result = super::styles::parse_styles_xml(&xml).ok();
                }
            }
            styles_result
        };

        let theme = {
            let mut theme_result = None;
            if let Ok(file) = archive.by_name("word/theme/theme1.xml") {
                let mut xml = String::new();
                if file
                    .take(super::MAX_UNCOMPRESSED_FILE_SIZE)
                    .read_to_string(&mut xml)
                    .is_ok()
                {
                    theme_result = super::theme::parse_theme_xml(&xml).ok();
                }
            }
            theme_result
        };

        Ok(Self {
            archive,
            relationships: AHashMap::new(),
            styles,
            theme,
        })
    }

    fn parse(mut self, budget: &mut SecurityBudget) -> Result<Document, DocxParseError> {
        let mut document = Document::new();

        if let Ok(rels_xml) = self.read_file("word/_rels/document.xml.rels") {
            self.relationships = Self::parse_relationships_xml(&rels_xml);
        }

        let document_xml = self.read_file("word/document.xml")?;
        let comment_ref_ids = self.parse_document_xml(&document_xml, &mut document, budget)?;

        if let Ok(comments_xml) = self.read_file("word/comments.xml") {
            self.parse_comments(&comments_xml, &mut document.comments, budget, &mut document.warnings)?;
        }
        if !comment_ref_ids.is_empty() {
            let known_ids: ahash::AHashSet<&str> = document.comments.iter().map(|c| c.id.as_str()).collect();
            for id in &comment_ref_ids {
                if !known_ids.contains(id.as_str()) {
                    crate::core::diagnostics::push_warning(
                        &mut document.warnings,
                        "docx",
                        format!(
                            "Comment reference id {id} has no matching entry in comments.xml; the comment text was dropped"
                        ),
                    );
                }
            }
        }

        if let Ok(numbering_xml) = self.read_file("word/numbering.xml") {
            let numbering_defs = self.parse_numbering(&numbering_xml, budget)?;
            document.numbering_defs = numbering_defs;
        }

        self.parse_headers_footers(&mut document, budget)?;

        if let Ok(footnotes_xml) = self.read_file("word/footnotes.xml") {
            self.parse_notes(
                &footnotes_xml,
                &mut document.footnotes,
                NoteType::Footnote,
                budget,
                &mut document.warnings,
            )?;
        }

        if let Ok(endnotes_xml) = self.read_file("word/endnotes.xml") {
            self.parse_notes(
                &endnotes_xml,
                &mut document.endnotes,
                NoteType::Endnote,
                budget,
                &mut document.warnings,
            )?;
        }

        document.style_catalog = self.styles.take();
        if let Some(catalog) = document.style_catalog.take() {
            apply_style_numbering(&catalog, &mut document);
            document.style_catalog = Some(catalog);
        }
        document.theme = self.theme.take();
        document.image_relationships = self
            .relationships
            .iter()
            .filter(|(_, target)| !target.starts_with("http://") && !target.starts_with("https://"))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Ok(document)
    }

    /// Parse relationship file to get rId → target mappings for hyperlinks and images.
    fn parse_relationships_xml(xml: &str) -> AHashMap<String, String> {
        let mut rels = AHashMap::new();
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) if e.name().as_ref() == "Relationship" => {
                    let mut id = None;
                    let mut target = None;
                    let mut rel_type_matches = false;
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            "Id" => id = Some(attr.value.to_string()),
                            "Target" => {
                                target = Some(attr.value.to_string());
                            }
                            "Type" => {
                                rel_type_matches = attr.value.contains("hyperlink") || attr.value.contains("image");
                            }
                            _ => {}
                        }
                    }
                    if let (Some(id_val), Some(target_val)) = (id, target)
                        && rel_type_matches
                    {
                        rels.insert(id_val, target_val);
                    }
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
            buf.clear();
        }

        rels
    }

    fn read_file(&mut self, path: &str) -> Result<String, DocxParseError> {
        let read_limit = super::MAX_UNCOMPRESSED_FILE_SIZE;

        let file = self
            .archive
            .by_name(path)
            .map_err(|_| DocxParseError::FileNotFound(path.to_string()))?;

        let mut contents = String::new();
        file.take(read_limit).read_to_string(&mut contents)?;
        Ok(contents)
    }

    /// Parse `word/document.xml`'s body content and populate `document` from it.
    ///
    /// Delegates to [`Self::parse_body_elements`] (shared with headers/footers and
    /// footnotes/endnotes/comments, #85) for the element vocabulary common to all
    /// DOCX body-like content, then additionally keeps the page-break/section/
    /// revision output that's only meaningful at the main-body level.
    ///
    /// Returns the `w:commentReference` ids seen in the body, for the caller to
    /// validate against `word/comments.xml`.
    fn parse_document_xml(
        &self,
        xml: &str,
        document: &mut Document,
        budget: &mut SecurityBudget,
    ) -> Result<Vec<String>, DocxParseError> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(false);
        let mut out = self.parse_body_elements(&mut reader, None, budget, &mut document.warnings)?;
        insert_missing_inline_drawing_page_breaks(
            &mut out.elements,
            &out.drawings,
            &out.sections,
            out.ambiguous_sections,
        );
        document.paragraphs = out.paragraphs;
        document.note_refs_by_paragraph = out.note_refs_by_paragraph;
        document.tables = out.tables;
        document.drawings = out.drawings;
        document.elements = out.elements;
        document.sections = out.sections.into_iter().map(|section| section.properties).collect();
        document.revisions = out.revisions;
        Ok(out.comment_ref_ids)
    }

    /// Shared element-dispatch loop for DOCX body-like content (#85): paragraphs,
    /// runs and their formatting, tables, hyperlinks, drawings — including
    /// DrawingML text boxes and the VML `v:textbox` fallback (#81) — field codes
    /// (`HYPERLINK` via both `w:fldSimple` and `w:fldChar`/`w:instrText`, #88/#239),
    /// symbols and non-breaking hyphens (#224), and footnote/endnote/comment
    /// reference markers (#82).
    ///
    /// Used by the main document body, headers/footers, and footnotes/endnotes/
    /// comments so all four content types recognize the same element vocabulary
    /// instead of drifting through separately-maintained reduced copies.
    ///
    /// `reader` must already be positioned at the first event *inside* the content
    /// to parse (e.g. right after `<w:document><w:body>`, or right after a
    /// `<w:footnote w:id="…">`/`<w:comment w:id="…">` start tag). When `stop_tag` is
    /// `Some(name)`, the loop returns as soon as the matching (depth-tracked)
    /// `</name>` end tag is consumed — used to carve one footnote/comment subtree
    /// out of a shared reader without a two-pass parse. When `stop_tag` is `None`,
    /// the loop runs to `Event::Eof` — used for whole-part parses (document body,
    /// headers, footers).
    fn parse_body_elements(
        &self,
        reader: &mut Reader<&[u8]>,
        stop_tag: Option<String>,
        budget: &mut SecurityBudget,
        warnings: &mut Vec<crate::types::ProcessingWarning>,
    ) -> Result<BodyParseOutputs, DocxParseError> {
        use crate::types::revisions::{
            DiffLine, DocumentRevision, PropertyChange, RevisionAnchor, RevisionDelta, RevisionKind,
        };

        let mut out = BodyParseOutputs::default();

        let mut buf = Vec::new();
        let mut current_paragraph: Option<Paragraph> = None;
        let mut current_run: Option<Run> = None;
        let mut in_text = false;
        let mut in_field_instruction = false;
        let mut in_instr_text = false;
        let mut field_instruction = String::new();
        let mut field_hyperlink_stack: Vec<Option<String>> = Vec::new();
        let mut current_hyperlink_url: Option<String> = None;
        let mut toc = TocState::default();
        let mut table_stack: Vec<TableContext> = Vec::new();
        let mut mc_fallback_depth: u32 = 0;
        let mut stop_depth: u32 = if stop_tag.is_some() { 1 } else { 0 };
        // Page-break bookkeeping shared by `w:br` and `w:lastRenderedPageBreak` (#1416, #1419).
        // `text_since_break` starts `true` so a break with nothing before it (including the
        // very first one in the document) is never treated as a spurious duplicate.
        let mut page_breaks = PageBreakState {
            text_since_break: true,
            ..PageBreakState::default()
        };
        let mut pending_section: Option<usize> = None;

        let mut revision_kind: Option<RevisionKind> = None;
        let mut revision_attrs: (Option<String>, Option<String>, Option<String>) = (None, None, None);
        let mut revision_text = String::new();
        let mut revision_id_counter: usize = 0;
        let mut in_del_text = false;
        // Depth of open `w:moveFrom` wrappers. A tracked move leaves the moved
        // content at its old place inside `w:moveFrom` (text in `w:t`, not
        // `w:delText`) and copies it to the new place inside `w:moveTo`. Word shows
        // it only at the new place, so runs, drawings, text boxes and equations
        // inside `w:moveFrom` are read but not kept. Revisions recorded inside it
        // keep their text.
        let mut move_from_depth: usize = 0;
        let mut current_paragraph_index: usize = 0;
        let mut in_run_property_change = false;
        let mut pending_format_revision_attrs: Option<(Option<String>, Option<String>, Option<String>)> = None;
        let mut pending_property_changes: Vec<PropertyChange> = Vec::new();

        loop {
            budget.step()?;
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    budget.enter()?;
                    let name = e.name();
                    if Some(name.as_ref()) == stop_tag.as_deref() {
                        stop_depth += 1;
                    }
                    match name.as_ref() {
                        "w:p" => {
                            if let Some(ctx) = table_stack.last_mut() {
                                ctx.paragraph = Some(Paragraph::new());
                            } else {
                                current_paragraph_index = out.paragraphs.len();
                                current_paragraph = Some(Paragraph::new());
                            }
                            if toc.active() {
                                mark_paragraph_in_toc(&mut table_stack, &mut current_paragraph);
                            }
                        }
                        "w:r" => {
                            let mut run = Run::default();
                            if let Some(ref url) = current_hyperlink_url {
                                run.hyperlink_url = Some(url.clone());
                            }
                            current_run = Some(run);
                        }
                        "w:t" if !in_field_instruction => {
                            in_text = true;
                        }
                        "w:moveFrom" => {
                            move_from_depth += 1;
                        }
                        "w:fldChar" => {
                            apply_fld_char(
                                e,
                                &mut in_field_instruction,
                                &mut field_instruction,
                                &mut current_hyperlink_url,
                                &mut field_hyperlink_stack,
                                &mut toc,
                            );
                            if toc.active() {
                                mark_paragraph_in_toc(&mut table_stack, &mut current_paragraph);
                            }
                        }
                        "w:sdt" => {
                            toc.open_sdt();
                        }
                        "w:docPartGallery" => {
                            toc.apply_doc_part_gallery(e);
                        }
                        "w:bookmarkStart" => {
                            apply_bookmark_start(e, &mut table_stack, &mut current_paragraph);
                        }
                        "w:instrText" => {
                            in_instr_text = true;
                        }
                        "w:fldSimple" => {
                            // Normalized, not raw: Word writes the instruction's quotes
                            // as `&quot;`, and the URL extractor looks for real quote
                            // characters. OOXML parts are XML 1.0.
                            let instr = e
                                .attributes()
                                .flatten()
                                .find(|a| a.key.as_ref() == "w:instr")
                                .and_then(|a| {
                                    a.normalized_value(quick_xml::XmlVersion::Explicit1_0)
                                        .ok()
                                        .map(|v| v.into_owned())
                                });
                            let url = instr.as_deref().and_then(extract_hyperlink_field_url);
                            field_hyperlink_stack.push(current_hyperlink_url.clone());
                            if url.is_some() {
                                current_hyperlink_url = url;
                            }
                            toc.open_fld_simple(instr.as_deref());
                            if toc.active() {
                                mark_paragraph_in_toc(&mut table_stack, &mut current_paragraph);
                            }
                        }
                        "mc:Fallback" => {
                            mc_fallback_depth += 1;
                        }
                        "w:pict" => {
                            // `parse_vml_pict` now threads `budget` through and balances
                            // its own `</w:pict>` end tag against the `enter()` above
                            // internally, so no manual `budget.leave()` is needed here. ~keep
                            let parsed = super::drawing::parse_vml_pict(reader, budget)?;
                            if mc_fallback_depth == 0
                                && move_from_depth == 0
                                && let Some(drawing) = parsed
                                && drawing.text_box_content.is_some()
                            {
                                let idx = out.drawings.len();
                                out.drawings.push(drawing);
                                out.elements.push(DocumentElement::Drawing(idx));
                                page_breaks.text_since_break = true;
                            }
                        }
                        "m:oMathPara" => {
                            let latex = super::math::collect_and_convert_omath_para(reader, budget)?;
                            if !latex.is_empty() && move_from_depth == 0 {
                                let run = Run {
                                    math_latex: Some((latex, true)),
                                    ..Default::default()
                                };
                                push_run_to_current(&mut table_stack, &mut current_paragraph, run);
                                page_breaks.text_since_break = true;
                            }
                        }
                        "m:oMath" => {
                            let latex = super::math::collect_and_convert_omath(reader, budget)?;
                            if !latex.is_empty() && move_from_depth == 0 {
                                let run = Run {
                                    math_latex: Some((latex, false)),
                                    ..Default::default()
                                };
                                push_run_to_current(&mut table_stack, &mut current_paragraph, run);
                                page_breaks.text_since_break = true;
                            }
                        }
                        "w:tbl" => {
                            table_stack.push(TableContext::new());
                        }
                        "w:tblPr" => {
                            if let Some(ctx) = table_stack.last_mut() {
                                // `parse_table_properties` now threads `budget` through and
                                // balances its own `</w:tblPr>` end tag against the `enter()`
                                // above internally, so no manual `budget.leave()` is needed here. ~keep
                                ctx.table.properties = Some(super::table::parse_table_properties(reader, budget)?);
                            }
                        }
                        "w:tblGrid" => {
                            if let Some(ctx) = table_stack.last_mut() {
                                ctx.table.grid = Some(super::table::parse_table_grid(reader, budget)?);
                            }
                        }
                        "w:tr" => {
                            if let Some(ctx) = table_stack.last_mut() {
                                ctx.current_row = Some(TableRow::default());
                                ctx.row_ordinal += 1;
                            }
                        }
                        "w:trPr" => {
                            if let Some(ctx) = table_stack.last_mut()
                                && let Some(ref mut row) = ctx.current_row
                            {
                                let (properties, skips) = super::table::parse_row_properties(reader, budget)?;
                                row.properties = Some(properties);
                                // The skipped grid columns are cells of the table grid too, so
                                // they count against the cell budget before any grid is built.
                                if skips.before > 0 {
                                    budget.add_cells(skips.before as usize)?;
                                    row.cells.push(TableCell::grid_padding(skips.before));
                                }
                                ctx.pending_grid_after = skips.after;
                            }
                        }
                        "w:tc" => {
                            if let Some(ctx) = table_stack.last_mut() {
                                ctx.current_cell = Some(TableCell::default());
                                ctx.cell_ordinal += 1;
                            }
                        }
                        "w:tcPr" => {
                            if let Some(ctx) = table_stack.last_mut()
                                && let Some(ref mut cell) = ctx.current_cell
                            {
                                cell.properties = Some(super::table::parse_cell_properties(reader, budget)?);
                            }
                        }
                        "w:b" | "w:i" | "w:u" | "w:strike" | "w:dstrike" | "w:vertAlign" | "w:sz" | "w:color"
                        | "w:highlight" => {
                            if in_run_property_change {
                                collect_run_property_change(e, &mut pending_property_changes);
                            } else {
                                apply_run_formatting(e, &mut current_run);
                            }
                        }
                        "w:pStyle" | "w:ilvl" | "w:numId" => {
                            apply_paragraph_property(e, &mut table_stack, &mut current_paragraph);
                        }
                        "w:hyperlink" => {
                            let mut has_relationship_id = false;
                            let mut relationship_url: Option<String> = None;
                            let mut anchor: Option<String> = None;
                            for attr in e.attributes().flatten() {
                                match attr.key.as_ref() {
                                    "r:id" => {
                                        has_relationship_id = true;
                                        let rid = attr.value.as_ref();
                                        relationship_url = self.relationships.get(rid).cloned();
                                    }
                                    "w:anchor" => {
                                        anchor = Some(attr.value.as_ref())
                                            .filter(|value| !value.is_empty())
                                            .map(String::from);
                                    }
                                    _ => {}
                                }
                            }
                            // `w:anchor` names a bookmark inside the document and was
                            // previously ignored, so an internal jump — every entry of a
                            // generated table of contents — produced no link at all. On its
                            // own the anchor is the whole target; alongside an `r:id` it is
                            // that external URL's fragment.
                            match (has_relationship_id, relationship_url, anchor) {
                                (_, Some(url), Some(anchor)) if !url.contains('#') => {
                                    current_hyperlink_url = Some(format!("{url}#{anchor}"));
                                }
                                (_, Some(url), _) => current_hyperlink_url = Some(url),
                                (_, None, Some(anchor)) => current_hyperlink_url = Some(format!("#{anchor}")),
                                (true, None, None) => current_hyperlink_url = None,
                                (false, None, None) => {}
                            }
                        }
                        "w:drawing" => {
                            // `parse_drawing` now threads `budget` through and balances its
                            // own `</w:drawing>` end tag against the `enter()` above
                            // internally, so no manual `budget.leave()` is needed here. ~keep
                            let drawing = super::drawing::parse_drawing(reader, budget)?;
                            if move_from_depth == 0 {
                                let idx = out.drawings.len();
                                out.drawings.push(drawing);
                                out.elements.push(DocumentElement::Drawing(idx));
                                page_breaks.text_since_break = true;
                            }
                        }
                        // `w:cr` is a carriage return inside a run, which Word shows as a line break.
                        "w:br" | "w:cr" => {
                            apply_break(
                                e,
                                &table_stack,
                                &mut current_run,
                                &current_paragraph,
                                &mut out.elements,
                                &mut page_breaks,
                            );
                        }
                        "w:tab" | "w:ptab" => apply_tab(&mut current_run, &mut page_breaks),
                        "w:lastRenderedPageBreak" => {
                            apply_last_rendered_page_break(
                                &table_stack,
                                &current_run,
                                &current_paragraph,
                                &mut out.elements,
                                &mut page_breaks,
                            );
                        }
                        "w:sectPr" => {
                            // `parse_section_properties_streaming` now threads `budget`
                            // through and balances its own `</w:sectPr>` end tag against the
                            // `enter()` above internally, so no manual `budget.leave()` is
                            // needed here. ~keep
                            let sect_props = super::section::parse_section_properties_streaming(reader, budget)?;
                            let section_index = out.sections.len();
                            out.sections.push(ParsedSection {
                                properties: sect_props,
                                end_element_index: None,
                            });
                            if !table_stack.is_empty() {
                                out.ambiguous_sections = true;
                            } else if current_paragraph.is_some() {
                                if pending_section.replace(section_index).is_some() {
                                    out.ambiguous_sections = true;
                                }
                            } else {
                                out.sections[section_index].end_element_index = Some(out.elements.len());
                            }
                        }
                        "w:ins" => {
                            revision_kind = Some(RevisionKind::Insertion);
                            revision_attrs = collect_revision_attrs(e);
                            revision_text.clear();
                        }
                        "w:del" => {
                            revision_kind = Some(RevisionKind::Deletion);
                            revision_attrs = collect_revision_attrs(e);
                            revision_text.clear();
                        }
                        "w:rPrChange" if revision_kind.is_none() => {
                            in_run_property_change = true;
                            pending_format_revision_attrs = Some(collect_revision_attrs(e));
                            pending_property_changes.clear();
                        }
                        "w:delText" => {
                            in_del_text = true;
                        }
                        _ => {}
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    let name = e.name();
                    match name.as_ref() {
                        "w:fldChar" => {
                            apply_fld_char(
                                e,
                                &mut in_field_instruction,
                                &mut field_instruction,
                                &mut current_hyperlink_url,
                                &mut field_hyperlink_stack,
                                &mut toc,
                            );
                            if toc.active() {
                                mark_paragraph_in_toc(&mut table_stack, &mut current_paragraph);
                            }
                        }
                        "w:docPartGallery" => {
                            toc.apply_doc_part_gallery(e);
                        }
                        "w:bookmarkStart" => {
                            apply_bookmark_start(e, &mut table_stack, &mut current_paragraph);
                        }
                        "w:b" | "w:i" | "w:u" | "w:strike" | "w:dstrike" | "w:vertAlign" | "w:sz" | "w:color"
                        | "w:highlight" => {
                            if in_run_property_change {
                                collect_run_property_change(e, &mut pending_property_changes);
                            } else {
                                apply_run_formatting(e, &mut current_run);
                            }
                        }
                        "w:pStyle" | "w:ilvl" | "w:numId" => {
                            apply_paragraph_property(e, &mut table_stack, &mut current_paragraph);
                        }
                        // `w:cr` is a carriage return inside a run, which Word shows as a line break.
                        "w:br" | "w:cr" => {
                            apply_break(
                                e,
                                &table_stack,
                                &mut current_run,
                                &current_paragraph,
                                &mut out.elements,
                                &mut page_breaks,
                            );
                        }
                        // `w:ptab` is an absolute position tab, such as the one that moves a
                        // footer's page number to the right margin.
                        "w:tab" | "w:ptab" => apply_tab(&mut current_run, &mut page_breaks),
                        "w:noBreakHyphen" => {
                            if let Some(ref mut run) = current_run {
                                run.text.push('\u{2011}');
                                page_breaks.text_since_break = true;
                            }
                        }
                        "w:sym" => {
                            apply_symbol(e, &mut current_run, warnings);
                            page_breaks.text_since_break = true;
                        }
                        "w:lastRenderedPageBreak" => {
                            apply_last_rendered_page_break(
                                &table_stack,
                                &current_run,
                                &current_paragraph,
                                &mut out.elements,
                                &mut page_breaks,
                            );
                        }
                        "w:footnoteReference" | "w:endnoteReference" => {
                            let note_type = if name.as_ref() == "w:endnoteReference" {
                                NoteType::Endnote
                            } else {
                                NoteType::Footnote
                            };
                            if let Some(ref mut run) = current_run {
                                for attr in e.attributes().flatten() {
                                    if attr.key.as_ref() == "w:id" {
                                        let id = attr.value.as_ref();
                                        // A reference in the body always points at a real note;
                                        // id 1 is Word's first one (see `parse_notes`).
                                        if id != "-1" && id != "0" {
                                            run.text.push_str(&format!("[^{}]", id));
                                            page_breaks.text_since_break = true;
                                            if table_stack.is_empty()
                                                && current_paragraph.is_some()
                                                && move_from_depth == 0
                                            {
                                                out.note_refs_by_paragraph
                                                    .entry(current_paragraph_index)
                                                    .or_default()
                                                    .push(NoteRef {
                                                        id: id.to_string(),
                                                        note_type,
                                                    });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        "w:commentReference" => {
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == "w:id" {
                                    let id = attr.value.as_ref();
                                    out.comment_ref_ids.push(id.to_string());
                                    if let Some(ref mut run) = current_run {
                                        run.text.push_str(&format!("[cmt:{}]", id));
                                    }
                                }
                            }
                        }
                        "w:sectPr" => {
                            let section_index = out.sections.len();
                            out.sections.push(ParsedSection {
                                properties: super::section::SectionProperties::default(),
                                end_element_index: None,
                            });
                            if !table_stack.is_empty() {
                                out.ambiguous_sections = true;
                            } else if current_paragraph.is_some() {
                                if pending_section.replace(section_index).is_some() {
                                    out.ambiguous_sections = true;
                                }
                            } else {
                                out.sections[section_index].end_element_index = Some(out.elements.len());
                            }
                        }
                        "w:tblPr" => {
                            if let Some(ctx) = table_stack.last_mut() {
                                ctx.table.properties = Some(super::table::TableProperties::default());
                            }
                        }
                        "w:tblGrid" => {
                            if let Some(ctx) = table_stack.last_mut() {
                                ctx.table.grid = Some(super::table::TableGrid::default());
                            }
                        }
                        "w:trPr" => {
                            if let Some(ctx) = table_stack.last_mut()
                                && let Some(ref mut row) = ctx.current_row
                            {
                                row.properties = Some(super::table::RowProperties::default());
                            }
                        }
                        "w:tcPr" => {
                            if let Some(ctx) = table_stack.last_mut()
                                && let Some(ref mut cell) = ctx.current_cell
                            {
                                cell.properties = Some(super::table::CellProperties::default());
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) => {
                    if in_instr_text {
                        let text = e.xml10_content();
                        budget.check_entity(&text)?;
                        budget.account_text(text.len())?;
                        field_instruction.push_str(&text);
                    } else if in_text && let Some(ref mut run) = current_run {
                        let text = e.xml10_content();
                        budget.check_entity(&text)?;
                        budget.account_text(text.len())?;
                        run.text.push_str(&text);
                        if !text.is_empty() {
                            page_breaks.text_since_break = true;
                        }
                        if revision_kind == Some(RevisionKind::Insertion) {
                            revision_text.push_str(&text);
                        }
                    } else if in_del_text {
                        let text = e.xml10_content();
                        budget.check_entity(&text)?;
                        budget.account_text(text.len())?;
                        revision_text.push_str(&text);
                    }
                }
                Ok(Event::GeneralRef(e)) => {
                    if in_text && let Some(ref mut run) = current_run {
                        let text = crate::utils::xml_utils::resolve_general_ref(&e);
                        budget.account_text(text.len())?;
                        run.text.push_str(&text);
                        if !text.is_empty() {
                            page_breaks.text_since_break = true;
                        }
                        if revision_kind == Some(RevisionKind::Insertion) {
                            revision_text.push_str(&text);
                        }
                    } else if in_del_text {
                        let text = crate::utils::xml_utils::resolve_general_ref(&e);
                        budget.account_text(text.len())?;
                        revision_text.push_str(&text);
                    }
                }
                Ok(Event::End(ref e)) => {
                    budget.leave();
                    let name = e.name();
                    if Some(name.as_ref()) == stop_tag.as_deref() {
                        stop_depth = stop_depth.saturating_sub(1);
                    }
                    match name.as_ref() {
                        "w:t" => {
                            in_text = false;
                        }
                        "w:moveFrom" => {
                            move_from_depth = move_from_depth.saturating_sub(1);
                        }
                        "w:instrText" => {
                            in_instr_text = false;
                        }
                        "w:sdt" => {
                            toc.close_sdt();
                        }
                        "w:fldSimple" => {
                            toc.close_fld_simple();
                            if let Some(saved) = field_hyperlink_stack.pop() {
                                current_hyperlink_url = saved;
                            }
                        }
                        "mc:Fallback" => {
                            mc_fallback_depth = mc_fallback_depth.saturating_sub(1);
                        }
                        "w:rPrChange" => {
                            in_run_property_change = false;
                        }
                        "w:rPr" if !in_run_property_change => {
                            if let Some(attrs) = pending_format_revision_attrs.take() {
                                push_format_revision(
                                    &mut out.revisions,
                                    attrs,
                                    std::mem::take(&mut pending_property_changes),
                                    current_run.as_ref(),
                                    current_paragraph_index,
                                    &mut revision_id_counter,
                                );
                            }
                        }
                        "w:r" => {
                            if let Some(attrs) = pending_format_revision_attrs.take() {
                                push_format_revision(
                                    &mut out.revisions,
                                    attrs,
                                    std::mem::take(&mut pending_property_changes),
                                    current_run.as_ref(),
                                    current_paragraph_index,
                                    &mut revision_id_counter,
                                );
                            }
                            if let Some(run) = current_run.take()
                                && move_from_depth == 0
                            {
                                push_run_to_current(&mut table_stack, &mut current_paragraph, run);
                            }
                        }
                        "w:p" => {
                            if let Some(ctx) = table_stack.last_mut() {
                                if let Some(para) = ctx.paragraph.take()
                                    && let Some(ref mut cell) = ctx.current_cell
                                {
                                    cell.paragraphs.push(para);
                                }
                            } else if let Some(para) = current_paragraph.take() {
                                let idx = out.paragraphs.len();
                                out.paragraphs.push(para);
                                out.elements.push(DocumentElement::Paragraph(idx));
                                for _ in 0..std::mem::take(&mut page_breaks.pending_paragraph) {
                                    out.elements.push(DocumentElement::PageBreak);
                                }
                                if let Some(section_index) = pending_section.take() {
                                    out.sections[section_index].end_element_index = Some(out.elements.len());
                                }
                            }
                        }
                        "w:tc" => {
                            if let Some(ctx) = table_stack.last_mut()
                                && let Some(cell) = ctx.current_cell.take()
                                && let Some(ref mut row) = ctx.current_row
                            {
                                budget.add_cells(1)?;
                                row.cells.push(cell);
                            }
                        }
                        "w:tr" => {
                            if let Some(ctx) = table_stack.last_mut()
                                && let Some(mut row) = ctx.current_row.take()
                            {
                                let grid_after = std::mem::take(&mut ctx.pending_grid_after);
                                if grid_after > 0 {
                                    budget.add_cells(grid_after as usize)?;
                                    row.cells.push(TableCell::grid_padding(grid_after));
                                }
                                ctx.table.rows.push(row);
                            }
                        }
                        "w:tbl" => {
                            if let Some(completed_ctx) = table_stack.pop() {
                                let completed_table = completed_ctx.table;
                                if let Some(parent_ctx) = table_stack.last_mut() {
                                    if let Some(ref mut cell) = parent_ctx.current_cell {
                                        for row in completed_table.rows {
                                            for table_cell in row.cells {
                                                for para in table_cell.paragraphs {
                                                    cell.paragraphs.push(para);
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    let idx = out.tables.len();
                                    out.tables.push(completed_table);
                                    out.elements.push(DocumentElement::Table(idx));
                                    // The outermost table close is the only point that flushes
                                    // page breaks deferred while inside a table (#1419); a nested
                                    // instead, so breaks in an inner table stay pending until the
                                    // outer one closes and only ever flush once.
                                    page_breaks.text_since_break = true;
                                    let deferred_breaks = std::mem::take(&mut page_breaks.pending_table);
                                    for _ in 0..deferred_breaks {
                                        out.elements.push(DocumentElement::PageBreak);
                                    }
                                    if deferred_breaks > 0 {
                                        page_breaks.text_since_break = false;
                                    }
                                    // The row/cell position recorded for dedup (#1592) is only
                                    // meaningful relative to the table just flushed; clear it so
                                    // a later, unrelated table's first row can't collide with it.
                                    page_breaks.last_table_break = None;
                                }
                            }
                        }
                        "w:hyperlink" => {
                            current_hyperlink_url = None;
                        }
                        "w:ins" if revision_kind == Some(RevisionKind::Insertion) => {
                            let (id_opt, author_opt, date_opt) = (
                                revision_attrs.0.take(),
                                revision_attrs.1.take(),
                                revision_attrs.2.take(),
                            );
                            let revision_id = id_opt.unwrap_or_else(|| {
                                let fallback = format!("docx-ins-{}", revision_id_counter);
                                revision_id_counter += 1;
                                fallback
                            });
                            let delta = if revision_text.is_empty() {
                                RevisionDelta::default()
                            } else {
                                RevisionDelta {
                                    content: vec![DiffLine::Added(std::mem::take(&mut revision_text))],
                                    ..Default::default()
                                }
                            };
                            out.revisions.push(DocumentRevision {
                                revision_id,
                                author: author_opt,
                                timestamp: date_opt,
                                kind: RevisionKind::Insertion,
                                anchor: Some(RevisionAnchor::Paragraph {
                                    index: current_paragraph_index,
                                }),
                                delta,
                            });
                            revision_kind = None;
                            revision_text.clear();
                        }
                        "w:del" if revision_kind == Some(RevisionKind::Deletion) => {
                            let (id_opt, author_opt, date_opt) = (
                                revision_attrs.0.take(),
                                revision_attrs.1.take(),
                                revision_attrs.2.take(),
                            );
                            let revision_id = id_opt.unwrap_or_else(|| {
                                let fallback = format!("docx-del-{}", revision_id_counter);
                                revision_id_counter += 1;
                                fallback
                            });
                            let delta = if revision_text.is_empty() {
                                RevisionDelta::default()
                            } else {
                                RevisionDelta {
                                    content: vec![DiffLine::Removed(std::mem::take(&mut revision_text))],
                                    ..Default::default()
                                }
                            };
                            out.revisions.push(DocumentRevision {
                                revision_id,
                                author: author_opt,
                                timestamp: date_opt,
                                kind: RevisionKind::Deletion,
                                anchor: Some(RevisionAnchor::Paragraph {
                                    index: current_paragraph_index,
                                }),
                                delta,
                            });
                            revision_kind = None;
                            revision_text.clear();
                        }
                        "w:delText" => {
                            in_del_text = false;
                        }
                        _ => {}
                    }
                    if stop_tag.is_some() && stop_depth == 0 {
                        break;
                    }
                }
                Ok(Event::Eof) => {
                    if in_field_instruction {
                        crate::core::diagnostics::push_warning(
                            warnings,
                            "docx",
                            "A DOCX field instruction (w:fldChar begin) was never closed with a \
                             matching separate/end; trailing body content may have been treated \
                             as field instruction text and dropped",
                        );
                    }
                    break;
                }
                Err(e) => return Err(e.into()),
                _ => {}
            }
            buf.clear();
        }

        Ok(out)
    }

    fn parse_numbering(
        &self,
        xml: &str,
        budget: &mut SecurityBudget,
    ) -> Result<AHashMap<(i64, i64), ListType>, DocxParseError> {
        let mut numbering_defs: AHashMap<(i64, i64), ListType> = AHashMap::new();
        let mut abstract_num_formats: AHashMap<i64, AHashMap<i64, ListType>> = AHashMap::new();
        let mut num_to_abstract: AHashMap<i64, i64> = AHashMap::new();

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(false);

        let mut buf = Vec::new();
        let mut current_abstract_num_id: Option<i64> = None;
        let mut current_num_id: Option<i64> = None;
        let mut current_lvl: Option<i64> = None;

        loop {
            budget.step()?;
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    budget.enter()?;
                    match e.name().as_ref() {
                        "w:abstractNum" => {
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == "w:abstractNumId" {
                                    let id_str = attr.value.as_ref();
                                    current_abstract_num_id = id_str.parse().ok();
                                }
                            }
                        }
                        "w:num" => {
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == "w:numId" {
                                    let id_str = attr.value.as_ref();
                                    current_num_id = id_str.parse().ok();
                                }
                            }
                        }
                        "w:lvl" => {
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == "w:ilvl" {
                                    let id_str = attr.value.as_ref();
                                    current_lvl = id_str.parse().ok();
                                }
                            }
                        }
                        "w:numFmt" => {
                            if let (Some(abstract_id), Some(lvl)) = (current_abstract_num_id, current_lvl) {
                                let list_type = list_type_for_num_fmt(get_val_attr_string(e).as_deref());
                                abstract_num_formats
                                    .entry(abstract_id)
                                    .or_default()
                                    .insert(lvl, list_type);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Empty(ref e)) => match e.name().as_ref() {
                    "w:abstractNumId" => {
                        if let Some(num_id) = current_num_id
                            && let Some(abstract_id) = get_val_attr(e)
                        {
                            num_to_abstract.insert(num_id, abstract_id);
                        }
                    }
                    "w:numFmt" => {
                        if let (Some(abstract_id), Some(lvl)) = (current_abstract_num_id, current_lvl) {
                            let list_type = list_type_for_num_fmt(get_val_attr_string(e).as_deref());
                            abstract_num_formats
                                .entry(abstract_id)
                                .or_default()
                                .insert(lvl, list_type);
                        }
                    }
                    _ => {}
                },
                Ok(Event::End(ref e)) => {
                    budget.leave();
                    match e.name().as_ref() {
                        "w:abstractNum" => {
                            current_abstract_num_id = None;
                            current_lvl = None;
                        }
                        "w:lvl" => {
                            current_lvl = None;
                        }
                        "w:num" => {
                            current_num_id = None;
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
            buf.clear();
        }

        for (num_id, abstract_id) in &num_to_abstract {
            if let Some(formats) = abstract_num_formats.get(abstract_id) {
                for (lvl, list_type) in formats {
                    numbering_defs.insert((*num_id, *lvl), *list_type);
                }
            }
        }

        Ok(numbering_defs)
    }

    /// Discover and parse every header/footer part in the package (#83).
    ///
    /// Earlier versions of this parser only looked for `word/header{1,2,3}.xml` and
    /// `word/footer{1,2,3}.xml`, silently dropping any part beyond index 3. The
    /// relationships in `word/_rels/document.xml.rels` are the authoritative index, so
    /// they are read first and in declaration order.
    ///
    /// The conventional filenames are then swept as a *fallback*, not a replacement:
    /// a package can carry a header/footer part that no relationship points at, and
    /// dropping those would trade one silent loss (index > 3) for another (unreferenced
    /// part). Anything already named by a relationship is skipped, so a well-formed
    /// document is unaffected.
    fn parse_headers_footers(
        &mut self,
        document: &mut Document,
        budget: &mut SecurityBudget,
    ) -> Result<(), DocxParseError> {
        let rels_xml = self.read_file("word/_rels/document.xml.rels").unwrap_or_default();
        let mut targets = Self::parse_header_footer_relationship_targets(&rels_xml);

        // Collect before touching `self.archive` mutably in the read loop below.
        let mut orphans = Vec::new();
        for name in self.archive.file_names() {
            let name = name?;
            let Some((is_header, index)) = conventional_header_footer_part(&name) else {
                continue;
            };
            if !targets.iter().any(|(_, seen)| seen == name.as_ref()) {
                orphans.push((is_header, index, name.into_owned()));
            }
        }
        // Headers before footers, each in numeric order, so output does not depend on
        // the order the zip central directory happens to list parts in.
        orphans.sort_by_key(|(is_header, index, _)| (!is_header, *index));
        targets.extend(orphans.into_iter().map(|(is_header, _, path)| (is_header, path)));

        for (is_header, path) in targets {
            match self.read_file(&path) {
                Ok(xml) => {
                    let mut header_footer = HeaderFooter::default();
                    self.parse_header_footer_content(&xml, &mut header_footer, budget, &mut document.warnings)?;
                    if is_header {
                        document.headers.push(header_footer);
                    } else {
                        document.footers.push(header_footer);
                    }
                }
                Err(_) => {
                    crate::core::diagnostics::push_warning(
                        &mut document.warnings,
                        "docx",
                        format!(
                            "{} relationship target '{}' could not be read; that {} was dropped",
                            if is_header { "Header" } else { "Footer" },
                            path,
                            if is_header { "header" } else { "footer" }
                        ),
                    );
                }
            }
        }

        Ok(())
    }

    /// Scan a relationships XML document for header/footer part targets.
    ///
    /// Returns `(is_header, resolved_zip_path)` pairs in document order. Relationship
    /// targets are resolved relative to `word/` (the directory containing
    /// `document.xml`, per OPC convention), or treated as package-root-absolute when
    /// they start with `/`.
    fn parse_header_footer_relationship_targets(xml: &str) -> Vec<(bool, String)> {
        let mut targets = Vec::new();
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) if e.name().as_ref() == "Relationship" => {
                    let mut target: Option<String> = None;
                    let mut kind: Option<bool> = None;
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            "Target" => target = Some(attr.value.to_string()),
                            "Type" => {
                                kind = if attr.value.ends_with("/header") {
                                    Some(true)
                                } else if attr.value.ends_with("/footer") {
                                    Some(false)
                                } else {
                                    None
                                };
                            }
                            _ => {}
                        }
                    }
                    if let (Some(target), Some(is_header)) = (target, kind) {
                        let resolved = match target.strip_prefix('/') {
                            Some(stripped) => stripped.to_string(),
                            None => format!("word/{}", target),
                        };
                        targets.push((is_header, resolved));
                    }
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
            buf.clear();
        }

        targets
    }

    /// Parse one header or footer part, sharing the same element vocabulary as the
    /// main document body and footnotes/endnotes (#85).
    fn parse_header_footer_content(
        &self,
        xml: &str,
        header_footer: &mut HeaderFooter,
        budget: &mut SecurityBudget,
        warnings: &mut Vec<crate::types::ProcessingWarning>,
    ) -> Result<(), DocxParseError> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(false);
        let out = self.parse_body_elements(&mut reader, None, budget, warnings)?;
        header_footer.paragraphs = out.paragraphs;
        header_footer.tables = out.tables;
        Ok(())
    }

    /// Parse `word/footnotes.xml` or `word/endnotes.xml`, delegating each individual
    /// `<w:footnote>`/`<w:endnote>` subtree to the shared body element loop (#85) so
    /// notes recognize the same tables/hyperlinks/fields/symbols as the main body.
    ///
    /// A top-level table inside a note (rare, but structurally possible) is
    /// flattened into the note's paragraph list, the same way a table nested inside
    /// a table cell is flattened by the shared loop itself.
    fn parse_notes(
        &self,
        xml: &str,
        notes: &mut Vec<Note>,
        note_type: NoteType,
        budget: &mut SecurityBudget,
        warnings: &mut Vec<crate::types::ProcessingWarning>,
    ) -> Result<(), DocxParseError> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(false);
        let mut buf = Vec::new();

        loop {
            budget.step()?;
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if matches!(e.name().as_ref(), "w:footnote" | "w:endnote") => {
                    let mut id = String::new();
                    // The separator lines above the notes carry a `w:type` (separator,
                    // continuationSeparator, continuationNotice); a real note has none or
                    // "normal". Their ids differ by producer: Word writes the separators as -1
                    // and 0 and its first note as 1, LibreOffice uses 0 and 1 and starts at 2.
                    // Skipping id 1 unconditionally dropped the first note of every Word file.
                    let mut is_separator = false;
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            "w:id" => id = attr.value.as_ref().to_string(),
                            "w:type" => is_separator = attr.value.as_ref() != "normal",
                            _ => {}
                        }
                    }
                    let stop_tag = e.name().as_ref().to_string();
                    let out = self.parse_body_elements(&mut reader, Some(stop_tag), budget, warnings)?;

                    if !is_separator && id != "-1" && id != "0" {
                        let mut paragraphs = out.paragraphs;
                        for table in out.tables {
                            for row in table.rows {
                                for cell in row.cells {
                                    paragraphs.extend(cell.paragraphs);
                                }
                            }
                        }
                        notes.push(Note {
                            id,
                            note_type,
                            paragraphs,
                        });
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(e.into()),
                _ => {}
            }
            buf.clear();
        }

        Ok(())
    }

    /// Parse `word/comments.xml`, delegating each `<w:comment>` subtree to the
    /// shared body element loop (#82, #85).
    fn parse_comments(
        &self,
        xml: &str,
        comments: &mut Vec<Comment>,
        budget: &mut SecurityBudget,
        warnings: &mut Vec<crate::types::ProcessingWarning>,
    ) -> Result<(), DocxParseError> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(false);
        let mut buf = Vec::new();

        loop {
            budget.step()?;
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == "w:comment" => {
                    let mut id = String::new();
                    let mut author: Option<String> = None;
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            "w:id" => id = attr.value.as_ref().to_string(),
                            "w:author" => {
                                author = Some(attr.value.to_string()).filter(|s| !s.is_empty());
                            }
                            _ => {}
                        }
                    }
                    let out = self.parse_body_elements(&mut reader, Some("w:comment".to_string()), budget, warnings)?;
                    let mut paragraphs = out.paragraphs;
                    for table in out.tables {
                        for row in table.rows {
                            for cell in row.cells {
                                paragraphs.extend(cell.paragraphs);
                            }
                        }
                    }
                    comments.push(Comment { id, author, paragraphs });
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(e.into()),
                _ => {}
            }
            buf.clear();
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DocxParseError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("XML parsing error: {0}")]
    Xml(#[from] quick_xml::Error),

    #[error("Required file not found in DOCX: {0}")]
    FileNotFound(String),

    #[error("Security limit exceeded: {0}")]
    SecurityLimit(String),
}

impl From<quick_xml::encoding::EncodingError> for DocxParseError {
    fn from(e: quick_xml::encoding::EncodingError) -> Self {
        DocxParseError::Xml(quick_xml::Error::Encoding(e))
    }
}

impl From<SecurityError> for DocxParseError {
    fn from(e: SecurityError) -> Self {
        DocxParseError::SecurityLimit(e.to_string())
    }
}

/// Parse a DOCX document from bytes and return the structured document.
///
/// `limits` caps the archive's entry count, per-entry and total declared size, and
/// compression ratio; callers read it from `ExtractionConfig.security_limits` (falling
/// back to `SecurityLimits::default()`) so the limits are configurable rather than
/// hardcoded ceilings baked into the parser.
pub(crate) fn parse_document(
    bytes: &[u8],
    budget: &mut SecurityBudget,
    limits: &SecurityLimits,
) -> crate::error::Result<Document> {
    let cursor = Cursor::new(bytes);
    let parser = DocxParser::new(cursor, limits)
        .map_err(|e| crate::error::XbergError::parsing(format!("DOCX parsing failed: {}", e)))?;
    parser
        .parse(budget)
        .map_err(|e| crate::error::XbergError::parsing(format!("DOCX parsing failed: {}", e)))
}

/// Extract text from DOCX bytes.
#[cfg(test)]
pub(crate) fn extract_text_from_bytes(bytes: &[u8]) -> crate::error::Result<String> {
    let mut budget = SecurityBudget::with_defaults();
    let limits = crate::extractors::security::SecurityLimits::default();
    let doc = parse_document(bytes, &mut budget, &limits)?;
    Ok(doc.extract_text())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractors::security::SecurityBudget;

    /// The default security limits `parse_document`/`validate_archive_security` enforce
    /// when a test has no `ExtractionConfig` of its own to derive them from.
    fn default_limits() -> SecurityLimits {
        SecurityLimits::default()
    }

    /// Runs are concatenated directly; whitespace comes from the XML text content.
    #[test]
    fn test_paragraph_to_text_concatenates_runs() {
        let mut para = Paragraph::new();
        para.add_run(Run::new("Hello ".to_string()));
        para.add_run(Run::new("World".to_string()));
        assert_eq!(para.to_text(), "Hello World");
    }

    /// Mid-word run splits (e.g. drop caps) must not insert extra spaces.
    #[test]
    fn test_paragraph_to_text_mid_word_split() {
        let mut para = Paragraph::new();
        para.add_run(Run::new("S".to_string()));
        para.add_run(Run::new("ermocination".to_string()));
        assert_eq!(para.to_text(), "Sermocination");
    }

    #[test]
    fn test_paragraph_to_text_single_run() {
        let mut para = Paragraph::new();
        para.add_run(Run::new("Hello".to_string()));
        assert_eq!(para.to_text(), "Hello");
    }

    #[test]
    fn test_paragraph_to_text_no_runs() {
        let para = Paragraph::new();
        assert_eq!(para.to_text(), "");
    }

    /// Whitespace between words is stored in the run text, not added by join.
    #[test]
    fn test_paragraph_to_text_whitespace_in_runs() {
        let mut para = Paragraph::new();
        para.add_run(Run::new("The ".to_string()));
        para.add_run(Run::new("quick ".to_string()));
        para.add_run(Run::new("fox".to_string()));
        assert_eq!(para.to_text(), "The quick fox");
    }

    #[test]
    fn test_run_bold_to_markdown() {
        let run = Run {
            text: "hello".to_string(),
            bold: true,
            ..Default::default()
        };
        assert_eq!(run.to_markdown(), "**hello**");
    }

    #[test]
    fn test_run_italic_to_markdown() {
        let run = Run {
            text: "hello".to_string(),
            italic: true,
            ..Default::default()
        };
        assert_eq!(run.to_markdown(), "*hello*");
    }

    #[test]
    fn test_run_bold_italic_to_markdown() {
        let run = Run {
            text: "hello".to_string(),
            bold: true,
            italic: true,
            ..Default::default()
        };
        assert_eq!(run.to_markdown(), "***hello***");
    }

    #[test]
    fn test_run_strikethrough_to_markdown() {
        let run = Run {
            text: "hello".to_string(),
            strikethrough: true,
            ..Default::default()
        };
        assert_eq!(run.to_markdown(), "~~hello~~");
    }

    #[test]
    fn test_run_hyperlink_to_markdown() {
        let run = Run {
            text: "click here".to_string(),
            hyperlink_url: Some("https://example.com".to_string()),
            ..Default::default()
        };
        assert_eq!(run.to_markdown(), "[click here](https://example.com)");
    }

    #[test]
    fn test_run_bold_hyperlink_to_markdown() {
        let run = Run {
            text: "click".to_string(),
            bold: true,
            hyperlink_url: Some("https://example.com".to_string()),
            ..Default::default()
        };
        assert_eq!(run.to_markdown(), "[**click**](https://example.com)");
    }

    #[test]
    fn test_run_empty_text_to_markdown() {
        let run = Run {
            text: String::new(),
            bold: true,
            ..Default::default()
        };
        assert_eq!(run.to_markdown(), "");
    }

    /// Adjacent bold runs must be merged to avoid spurious `****` sequences.
    #[test]
    fn test_adjacent_bold_runs_merged() {
        let mut para = Paragraph::new();
        let mut r1 = Run::new("Shuishang".to_string());
        r1.bold = true;
        let mut r2 = Run::new(" Township".to_string());
        r2.bold = true;
        para.add_run(r1);
        para.add_run(r2);
        assert_eq!(para.runs_to_markdown(), "**Shuishang Township**");
    }

    /// Adjacent italic runs must be merged to avoid spurious `**` sequences.
    #[test]
    fn test_adjacent_italic_runs_merged() {
        let mut para = Paragraph::new();
        let mut r1 = Run::new("he".to_string());
        r1.italic = true;
        let mut r2 = Run::new("llo".to_string());
        r2.italic = true;
        para.add_run(r1);
        para.add_run(r2);
        assert_eq!(para.runs_to_markdown(), "*hello*");
    }

    /// Runs with different formatting must NOT be merged.
    #[test]
    fn test_different_formatting_runs_not_merged() {
        let mut para = Paragraph::new();
        let mut r1 = Run::new("bold".to_string());
        r1.bold = true;
        let r2 = Run::new(" normal".to_string());
        para.add_run(r1);
        para.add_run(r2);
        assert_eq!(para.runs_to_markdown(), "**bold** normal");
    }

    /// Three adjacent bold runs produce a single merged bold span.
    #[test]
    fn test_three_adjacent_bold_runs_merged() {
        let mut para = Paragraph::new();
        for text in &["i", "l", "l"] {
            let mut r = Run::new(text.to_string());
            r.bold = true;
            para.add_run(r);
        }
        assert_eq!(para.runs_to_markdown(), "**ill**");
    }

    #[test]
    fn test_paragraph_heading_to_markdown() {
        let mut para = Paragraph::new();
        para.style = Some("Title".to_string());
        para.add_run(Run::new("My Title".to_string()));
        let defs = AHashMap::new();
        let mut counters = AHashMap::new();
        assert_eq!(para.to_markdown(&defs, &mut counters, Some(1)), "# My Title");
    }

    #[test]
    fn test_paragraph_heading1_to_markdown() {
        let mut para = Paragraph::new();
        para.style = Some("Heading1".to_string());
        para.add_run(Run::new("Section".to_string()));
        let defs = AHashMap::new();
        let mut counters = AHashMap::new();
        assert_eq!(para.to_markdown(&defs, &mut counters, Some(2)), "## Section");
    }

    #[test]
    fn test_paragraph_heading2_to_markdown() {
        let mut para = Paragraph::new();
        para.style = Some("Heading2".to_string());
        para.add_run(Run::new("Subsection".to_string()));
        let defs = AHashMap::new();
        let mut counters = AHashMap::new();
        assert_eq!(para.to_markdown(&defs, &mut counters, Some(3)), "### Subsection");
    }

    #[test]
    fn test_paragraph_bullet_list_to_markdown() {
        let mut para = Paragraph::new();
        para.numbering_id = Some(1);
        para.numbering_level = Some(0);
        para.add_run(Run::new("Item".to_string()));
        let mut defs = AHashMap::new();
        defs.insert((1, 0), ListType::Bullet);
        let mut counters = AHashMap::new();
        assert_eq!(para.to_markdown(&defs, &mut counters, None), "- Item");
    }

    #[test]
    fn test_paragraph_numbered_list_to_markdown() {
        let mut para = Paragraph::new();
        para.numbering_id = Some(2);
        para.numbering_level = Some(0);
        para.add_run(Run::new("Item".to_string()));
        let mut defs = AHashMap::new();
        defs.insert((2, 0), ListType::Numbered);
        let mut counters = AHashMap::new();
        assert_eq!(para.to_markdown(&defs, &mut counters, None), "1. Item");
    }

    /// Only `bullet` and `none` are not numbered. The CJK, enclosed, ordinal and other
    /// non-Latin formats used to be read as bullets, so such a list lost its order.
    #[test]
    fn test_every_numbering_format_except_bullet_and_none_is_numbered() {
        let formats = [
            "decimal",
            "upperRoman",
            "chineseCounting",
            "chineseCountingThousand",
            "japaneseCounting",
            "ideographTraditional",
            "decimalEnclosedCircle",
            "decimalFullWidth",
            "koreanDigital",
            "ordinal",
            "cardinalText",
            "hebrew1",
            "russianLower",
            "custom",
            "bullet",
            "none",
        ];
        // The last level writes its numFmt as a start/end pair instead of self-closing.
        let mut levels: String = formats
            .iter()
            .enumerate()
            .map(|(lvl, fmt)| format!(r#"<w:lvl w:ilvl="{lvl}"><w:numFmt w:val="{fmt}"/></w:lvl>"#))
            .collect();
        let paired = formats.len();
        levels.push_str(&format!(
            r#"<w:lvl w:ilvl="{paired}"><w:numFmt w:val="taiwaneseCounting"></w:numFmt></w:lvl>"#
        ));
        let xml = format!(
            r#"<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:abstractNum w:abstractNumId="0">{levels}</w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num></w:numbering>"#
        );
        let parser = DocxParser {
            archive: zip::ZipArchive::new(std::io::Cursor::new(create_minimal_zip())).unwrap(),
            relationships: AHashMap::new(),
            styles: None,
            theme: None,
        };
        let mut budget = SecurityBudget::with_defaults();
        let defs = parser.parse_numbering(&xml, &mut budget).unwrap();

        for (lvl, fmt) in formats.iter().enumerate() {
            let expected = if matches!(*fmt, "bullet" | "none") {
                ListType::Bullet
            } else {
                ListType::Numbered
            };
            assert_eq!(defs.get(&(1, lvl as i64)), Some(&expected), "numFmt {fmt}");
        }
        assert_eq!(
            defs.get(&(1, paired as i64)),
            Some(&ListType::Numbered),
            "paired numFmt"
        );
    }

    #[test]
    fn test_paragraph_nested_list_to_markdown() {
        let mut para = Paragraph::new();
        para.numbering_id = Some(1);
        para.numbering_level = Some(1);
        para.add_run(Run::new("Nested".to_string()));
        let mut defs = AHashMap::new();
        defs.insert((1, 1), ListType::Bullet);
        let mut counters = AHashMap::new();
        assert_eq!(para.to_markdown(&defs, &mut counters, None), "  - Nested");
    }

    #[test]
    fn test_heading_level_from_style_name() {
        assert_eq!(heading_level_from_style_name("Title"), Some(1));
        assert_eq!(heading_level_from_style_name("Heading1"), Some(1));
        assert_eq!(heading_level_from_style_name("Heading2"), Some(2));
        assert_eq!(heading_level_from_style_name("Heading3"), Some(3));
        assert_eq!(heading_level_from_style_name("Heading6"), Some(6));
        assert_eq!(heading_level_from_style_name("Normal"), None);
    }

    #[test]
    fn test_resolve_heading_level_with_style_catalog() {
        use super::super::styles::{ParagraphProperties, StyleCatalog, StyleDefinition, StyleType};

        let mut doc = Document::new();
        let mut catalog = StyleCatalog::default();

        catalog.styles.insert(
            "CustomHeading".to_string(),
            StyleDefinition {
                id: "CustomHeading".to_string(),
                name: Some("Custom Heading".to_string()),
                style_type: StyleType::Paragraph,
                based_on: None,
                next_style: None,
                is_default: false,
                paragraph_properties: ParagraphProperties {
                    outline_level: Some(2),
                    ..Default::default()
                },
                run_properties: Default::default(),
            },
        );

        doc.style_catalog = Some(catalog);
        assert_eq!(doc.resolve_heading_level("CustomHeading"), Some(3));
    }

    #[test]
    fn test_resolve_heading_level_inheritance_chain() {
        use super::super::styles::{ParagraphProperties, StyleCatalog, StyleDefinition, StyleType};

        let mut doc = Document::new();
        let mut catalog = StyleCatalog::default();

        catalog.styles.insert(
            "ParentStyle".to_string(),
            StyleDefinition {
                id: "ParentStyle".to_string(),
                name: Some("Parent".to_string()),
                style_type: StyleType::Paragraph,
                based_on: None,
                next_style: None,
                is_default: false,
                paragraph_properties: ParagraphProperties {
                    outline_level: Some(0),
                    ..Default::default()
                },
                run_properties: Default::default(),
            },
        );

        catalog.styles.insert(
            "ChildStyle".to_string(),
            StyleDefinition {
                id: "ChildStyle".to_string(),
                name: Some("Child".to_string()),
                style_type: StyleType::Paragraph,
                based_on: Some("ParentStyle".to_string()),
                next_style: None,
                is_default: false,
                paragraph_properties: ParagraphProperties::default(),
                run_properties: Default::default(),
            },
        );

        doc.style_catalog = Some(catalog);
        assert_eq!(doc.resolve_heading_level("ChildStyle"), Some(1));
    }

    #[test]
    fn test_underline_rendering() {
        let mut run = Run::new("underlined text".to_string());
        run.underline = true;
        assert_eq!(run.to_markdown(), "<u>underlined text</u>");
    }

    #[test]
    fn test_underline_combined_with_bold_italic() {
        let mut run = Run::new("styled".to_string());
        run.bold = true;
        run.italic = true;
        run.underline = true;
        let md = run.to_markdown();
        assert!(md.contains("<u>"));
        assert!(md.contains("</u>"));
        assert!(md.contains("**"));
        assert!(md.contains("*"));
    }

    #[test]
    fn test_header_footer_excluded_from_output() {
        let mut doc = Document::new();

        let mut header = HeaderFooter::default();
        let mut para = Paragraph::new();
        para.add_run(Run::new("Header Text".to_string()));
        header.paragraphs.push(para);
        doc.headers.push(header);

        let mut body_para = Paragraph::new();
        body_para.add_run(Run::new("Body content".to_string()));
        let idx = doc.paragraphs.len();
        doc.paragraphs.push(body_para);
        doc.elements.push(DocumentElement::Paragraph(idx));

        let mut footer = HeaderFooter::default();
        let mut footer_para = Paragraph::new();
        footer_para.add_run(Run::new("Footer Text".to_string()));
        footer.paragraphs.push(footer_para);
        doc.footers.push(footer);

        let md = doc.to_markdown(true);
        assert!(!md.contains("Header Text"), "Header should not be in markdown output");
        assert!(md.contains("Body content"), "Should contain body content");
        assert!(!md.contains("Footer Text"), "Footer should not be in markdown output");

        let plain = doc.to_plain_text();
        assert!(
            !plain.contains("Header Text"),
            "Header should not be in plain text output"
        );
        assert!(plain.contains("Body content"), "Should contain body content");
        assert!(
            !plain.contains("Footer Text"),
            "Footer should not be in plain text output"
        );

        assert_eq!(doc.headers.len(), 1);
        assert_eq!(doc.footers.len(), 1);
        assert_eq!(doc.headers[0].paragraphs[0].runs[0].text, "Header Text");
        assert_eq!(doc.footers[0].paragraphs[0].runs[0].text, "Footer Text");
    }

    #[test]
    fn test_footnote_reference_in_parsing() {
        let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r>
                        <w:t>See note</w:t>
                    </w:r>
                    <w:r>
                        <w:footnoteReference w:id="2"/>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let parser_struct = DocxParser {
            archive: zip::ZipArchive::new(std::io::Cursor::new(create_minimal_zip())).unwrap(),
            relationships: AHashMap::new(),
            styles: None,
            theme: None,
        };
        let mut document = Document::new();
        {
            let mut budget = crate::extractors::security::SecurityBudget::with_defaults();
            parser_struct
                .parse_document_xml(xml, &mut document, &mut budget)
                .unwrap();
        }

        assert_eq!(document.paragraphs.len(), 1);
        let full_text = document.paragraphs[0].to_text();
        assert!(
            full_text.contains("[^2]"),
            "Should contain footnote reference [^2], got: {}",
            full_text
        );
    }

    #[test]
    fn test_inline_tab_preserved_between_words() {
        let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r>
                        <w:t>Alpha</w:t>
                        <w:tab/>
                        <w:t>Beta</w:t>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let parser_struct = DocxParser {
            archive: zip::ZipArchive::new(std::io::Cursor::new(create_minimal_zip())).unwrap(),
            relationships: AHashMap::new(),
            styles: None,
            theme: None,
        };
        let mut document = Document::new();
        {
            let mut budget = crate::extractors::security::SecurityBudget::with_defaults();
            parser_struct
                .parse_document_xml(xml, &mut document, &mut budget)
                .unwrap();
        }

        assert_eq!(document.paragraphs.len(), 1);
        let full_text = document.paragraphs[0].to_text();
        assert_eq!(
            full_text, "Alpha\tBeta",
            "Tab character inside a run must separate the words, got: {:?}",
            full_text
        );
    }

    #[test]
    fn test_tab_stop_definition_does_not_emit_tab_character() {
        let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:pPr>
                        <w:tabs>
                            <w:tab w:val="left" w:pos="720"/>
                        </w:tabs>
                    </w:pPr>
                    <w:r>
                        <w:t>Alpha</w:t>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let parser_struct = DocxParser {
            archive: zip::ZipArchive::new(std::io::Cursor::new(create_minimal_zip())).unwrap(),
            relationships: AHashMap::new(),
            styles: None,
            theme: None,
        };
        let mut document = Document::new();
        {
            let mut budget = crate::extractors::security::SecurityBudget::with_defaults();
            parser_struct
                .parse_document_xml(xml, &mut document, &mut budget)
                .unwrap();
        }

        assert_eq!(document.paragraphs.len(), 1);
        let full_text = document.paragraphs[0].to_text();
        assert_eq!(
            full_text, "Alpha",
            "Tab-stop definition in w:pPr/w:tabs must not emit a tab character, got: {:?}",
            full_text
        );
    }

    fn parse_footnotes_xml(xml: &str) -> Vec<Note> {
        let parser_struct = DocxParser {
            archive: zip::ZipArchive::new(std::io::Cursor::new(create_minimal_zip())).unwrap(),
            relationships: AHashMap::new(),
            styles: None,
            theme: None,
        };
        let mut notes = Vec::new();
        let mut budget = crate::extractors::security::SecurityBudget::with_defaults();
        let mut warnings = Vec::new();
        parser_struct
            .parse_notes(xml, &mut notes, NoteType::Footnote, &mut budget, &mut warnings)
            .unwrap();
        notes
    }

    #[test]
    fn test_separator_footnotes_filtered() {
        // LibreOffice numbers the separators 0 and 1 and its first note 2.
        let xml = r#"<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:footnote w:type="separator" w:id="0">
                <w:p><w:r><w:separator/></w:r></w:p>
            </w:footnote>
            <w:footnote w:type="continuationSeparator" w:id="1">
                <w:p><w:r><w:continuationSeparator/></w:r></w:p>
            </w:footnote>
            <w:footnote w:id="2">
                <w:p><w:r><w:t>Actual footnote</w:t></w:r></w:p>
            </w:footnote>
        </w:footnotes>"#;

        let notes = parse_footnotes_xml(xml);

        assert_eq!(notes.len(), 1, "Only actual footnote should remain");
        assert_eq!(notes[0].id, "2");
    }

    #[test]
    fn test_word_first_footnote_is_kept() {
        // Word numbers the separators -1 and 0, so its first real note is id 1.
        let xml = r#"<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:footnote w:type="separator" w:id="-1">
                <w:p><w:r><w:separator/></w:r></w:p>
            </w:footnote>
            <w:footnote w:type="continuationSeparator" w:id="0">
                <w:p><w:r><w:continuationSeparator/></w:r></w:p>
            </w:footnote>
            <w:footnote w:id="1">
                <w:p><w:r><w:t>First footnote</w:t></w:r></w:p>
            </w:footnote>
            <w:footnote w:type="normal" w:id="2">
                <w:p><w:r><w:t>Second footnote</w:t></w:r></w:p>
            </w:footnote>
            <w:footnote w:type="continuationNotice" w:id="3">
                <w:p><w:r><w:t>continued on next page</w:t></w:r></w:p>
            </w:footnote>
        </w:footnotes>"#;

        let notes = parse_footnotes_xml(xml);
        let ids: Vec<&str> = notes.iter().map(|note| note.id.as_str()).collect();

        assert_eq!(
            ids,
            ["1", "2"],
            "real notes kept, separators and the continuation notice skipped"
        );
    }

    fn create_minimal_zip() -> Vec<u8> {
        use std::io::Write;
        let buf = Vec::new();
        let cursor = std::io::Cursor::new(buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(b"<w:document/>").unwrap();
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn test_is_format_enabled_no_val() {
        let xml = r#"<w:b/>"#;
        let mut reader = Reader::from_str(xml);
        let mut buf = Vec::new();
        if let Ok(Event::Empty(ref e)) = reader.read_event_into(&mut buf) {
            assert!(is_format_enabled(e));
        }
    }

    #[test]
    fn test_security_valid_minimal_archive() {
        use std::io::Cursor;
        let zip_data = vec![
            0x50, 0x4b, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        let cursor = Cursor::new(zip_data);
        let result = DocxParser::new(cursor, &default_limits());
        assert!(
            result.is_ok(),
            "Empty valid ZIP should pass security checks: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_security_constants_are_reasonable() {
        use super::super::{MAX_TOTAL_UNCOMPRESSED_SIZE, MAX_UNCOMPRESSED_FILE_SIZE};

        assert!(
            crate::extractors::security::SecurityLimits::default().max_files_in_archive >= 1_000,
            "Entry limit must be at least 1,000"
        );
        const {
            assert!(
                MAX_UNCOMPRESSED_FILE_SIZE >= 10 * 1024 * 1024,
                "Per-file size limit must be at least 10 MB"
            );
            assert!(
                MAX_TOTAL_UNCOMPRESSED_SIZE >= MAX_UNCOMPRESSED_FILE_SIZE,
                "Total size limit must be >= per-file limit"
            );
        }
    }

    #[test]
    fn test_security_normal_docx_passes() {
        use std::io::{Cursor, Write};

        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(b"<w:document/>").unwrap();

        zip.start_file("docProps/core.xml", options).unwrap();
        zip.write_all(b"<cp:coreProperties/>").unwrap();

        let cursor = zip.finish().unwrap();
        let data = cursor.into_inner();

        let mut archive = zip::ZipArchive::new(Cursor::new(data)).unwrap();
        let result = validate_archive_security(&mut archive, &default_limits());
        assert!(
            result.is_ok(),
            "A normal small archive must pass security validation: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_security_rejects_too_many_entries() {
        use std::io::{Cursor, Write};

        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);

        for i in 0..10_001 {
            zip.start_file(format!("file_{}.txt", i), options).unwrap();
            zip.write_all(b"").unwrap();
        }

        let cursor = zip.finish().unwrap();
        let data = cursor.into_inner();

        let mut archive = zip::ZipArchive::new(Cursor::new(data)).unwrap();
        let result = validate_archive_security(&mut archive, &default_limits());
        assert!(result.is_err(), "Archive with >10,000 entries must be rejected");

        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("10001") && err_msg.contains("10000"),
            "Error should mention actual and limit counts, got: {}",
            err_msg
        );
    }

    /// GH#639: `validate_archive_security` must honour a caller-supplied limit rather
    /// than a hardcoded ceiling. A limit above 10,000 would pass under the old hardcoded
    /// constant too, so this stays well under it - only reading `max_entries` catches it.
    #[test]
    fn test_security_rejects_too_many_entries_under_configured_limit() {
        use std::io::{Cursor, Write};

        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);

        for i in 0..5 {
            zip.start_file(format!("file_{}.txt", i), options).unwrap();
            zip.write_all(b"").unwrap();
        }

        let cursor = zip.finish().unwrap();
        let data = cursor.into_inner();

        let mut archive = zip::ZipArchive::new(Cursor::new(data)).unwrap();
        let limits = SecurityLimits {
            max_files_in_archive: 3,
            ..SecurityLimits::default()
        };
        let result = validate_archive_security(&mut archive, &limits);
        assert!(
            result.is_err(),
            "5 entries must be rejected against a configured limit of 3"
        );

        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains('5') && err_msg.contains('3'),
            "Error should mention actual and configured limit counts, got: {}",
            err_msg
        );
    }

    /// Sibling of the rejection test above: the same entry count under a configured
    /// limit that comfortably fits it must pass.
    #[test]
    fn test_security_allows_entries_within_configured_limit() {
        use std::io::{Cursor, Write};

        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);

        for i in 0..5 {
            zip.start_file(format!("file_{}.txt", i), options).unwrap();
            zip.write_all(b"").unwrap();
        }

        let cursor = zip.finish().unwrap();
        let data = cursor.into_inner();

        let mut archive = zip::ZipArchive::new(Cursor::new(data)).unwrap();
        let limits = SecurityLimits {
            max_files_in_archive: 10,
            ..SecurityLimits::default()
        };
        let result = validate_archive_security(&mut archive, &limits);
        assert!(
            result.is_ok(),
            "5 entries must pass against a configured limit of 10: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_security_rejects_oversized_file() {
        use std::io::{Cursor, Write};

        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(&[b'x'; 1024]).unwrap();

        let cursor = zip.finish().unwrap();
        let data = cursor.into_inner();

        let mut archive = zip::ZipArchive::new(Cursor::new(data)).unwrap();
        let result = validate_archive_security(&mut archive, &default_limits());
        assert!(
            result.is_ok(),
            "A 1 KB file must pass size validation: {:?}",
            result.err()
        );
    }

    /// Helper: create a minimal DOCX ZIP with the given XML as word/document.xml.
    fn create_test_docx(document_xml: &str) -> Vec<u8> {
        use std::io::{Cursor, Write};

        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(document_xml.as_bytes()).unwrap();

        let cursor = zip.finish().unwrap();
        cursor.into_inner()
    }

    #[test]
    fn test_nested_table_parsing() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:tc>
          <w:p><w:r><w:t>Outer Cell 1</w:t></w:r></w:p>
          <w:tbl>
            <w:tr>
              <w:tc>
                <w:p><w:r><w:t>Inner Cell</w:t></w:r></w:p>
              </w:tc>
            </w:tr>
          </w:tbl>
        </w:tc>
        <w:tc>
          <w:p><w:r><w:t>Outer Cell 2</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

        let bytes = create_test_docx(xml);
        let mut budget = SecurityBudget::with_defaults();
        let doc = parse_document(&bytes, &mut budget, &default_limits()).expect("parse_document should succeed");

        assert_eq!(doc.tables.len(), 1, "Expected exactly 1 (outer) table");

        let table = &doc.tables[0];
        assert_eq!(table.rows.len(), 1, "Outer table should have 1 row");
        assert_eq!(table.rows[0].cells.len(), 2, "Outer row should have 2 cells");

        let cell0 = &table.rows[0].cells[0];
        let cell0_texts: Vec<String> = cell0.paragraphs.iter().map(|p| p.to_text()).collect();
        assert!(
            cell0_texts.iter().any(|t| t.contains("Outer Cell 1")),
            "First cell must contain 'Outer Cell 1', got: {:?}",
            cell0_texts
        );
        assert!(
            cell0_texts.iter().any(|t| t.contains("Inner Cell")),
            "First cell must contain flattened 'Inner Cell', got: {:?}",
            cell0_texts
        );

        let cell1 = &table.rows[0].cells[1];
        let cell1_texts: Vec<String> = cell1.paragraphs.iter().map(|p| p.to_text()).collect();
        assert!(
            cell1_texts.iter().any(|t| t.contains("Outer Cell 2")),
            "Second cell must contain 'Outer Cell 2', got: {:?}",
            cell1_texts
        );
    }

    #[test]
    fn test_parser_loads_styles() {
        use std::io::{Cursor, Write};

        let styles_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="heading 1"/>
    <w:basedOn w:val="Normal"/>
    <w:pPr><w:outlineLvl w:val="0"/></w:pPr>
    <w:rPr><w:b/><w:sz w:val="32"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
    <w:name w:val="Normal"/>
  </w:style>
</w:styles>"#;

        let doc_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
      <w:r><w:t>Hello</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(doc_xml.as_bytes()).unwrap();
        zip.start_file("word/styles.xml", options).unwrap();
        zip.write_all(styles_xml.as_bytes()).unwrap();

        let cursor = zip.finish().unwrap();
        let bytes = cursor.into_inner();

        let mut budget = SecurityBudget::with_defaults();
        let doc = parse_document(&bytes, &mut budget, &default_limits()).expect("should parse");

        assert!(doc.style_catalog.is_some(), "Style catalog should be loaded");
        let catalog = doc.style_catalog.as_ref().unwrap();
        assert!(catalog.styles.contains_key("Heading1"));
        assert!(catalog.styles.contains_key("Normal"));

        let h1 = &catalog.styles["Heading1"];
        assert_eq!(h1.run_properties.bold, Some(true));
        assert_eq!(h1.run_properties.font_size_half_points, Some(32));
        assert_eq!(h1.paragraph_properties.outline_level, Some(0));
    }

    #[test]
    fn test_table_properties_integration() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tblPr>
        <w:tblStyle w:val="TableGrid"/>
        <w:tblW w:w="5000" w:type="dxa"/>
        <w:jc w:val="center"/>
      </w:tblPr>
      <w:tblGrid>
        <w:gridCol w:w="2500"/>
        <w:gridCol w:w="2500"/>
      </w:tblGrid>
      <w:tr>
        <w:trPr>
          <w:tblHeader/>
        </w:trPr>
        <w:tc>
          <w:tcPr>
            <w:tcW w:w="2500" w:type="dxa"/>
            <w:shd w:val="clear" w:fill="D9E2F3"/>
          </w:tcPr>
          <w:p><w:r><w:t>Header 1</w:t></w:r></w:p>
        </w:tc>
        <w:tc>
          <w:tcPr>
            <w:tcW w:w="2500" w:type="dxa"/>
            <w:gridSpan w:val="1"/>
          </w:tcPr>
          <w:p><w:r><w:t>Header 2</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
      <w:tr>
        <w:tc>
          <w:tcPr>
            <w:vMerge w:val="restart"/>
          </w:tcPr>
          <w:p><w:r><w:t>Merged</w:t></w:r></w:p>
        </w:tc>
        <w:tc>
          <w:p><w:r><w:t>Data</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

        let bytes = create_test_docx(xml);
        let mut budget = SecurityBudget::with_defaults();
        let doc = parse_document(&bytes, &mut budget, &default_limits()).expect("parse should succeed");

        assert_eq!(doc.tables.len(), 1);
        let table = &doc.tables[0];

        let tbl_props = table.properties.as_ref().expect("table should have properties");
        assert_eq!(tbl_props.style_id.as_deref(), Some("TableGrid"));
        assert_eq!(tbl_props.alignment.as_deref(), Some("center"));
        assert!(tbl_props.width.is_some());
        assert_eq!(tbl_props.width.as_ref().unwrap().value, 5000);

        let grid = table.grid.as_ref().expect("table should have grid");
        assert_eq!(grid.columns, vec![2500, 2500]);

        let row0 = &table.rows[0];
        let row_props = row0.properties.as_ref().expect("header row should have properties");
        assert!(row_props.is_header);

        let cell00 = &row0.cells[0];
        let cell_props = cell00.properties.as_ref().expect("cell should have properties");
        assert!(cell_props.shading.is_some());
        assert_eq!(cell_props.shading.as_ref().unwrap().fill.as_deref(), Some("D9E2F3"));

        let cell10 = &table.rows[1].cells[0];
        let cell10_props = cell10.properties.as_ref().expect("merged cell should have properties");
        assert_eq!(
            cell10_props.v_merge,
            Some(crate::extraction::docx::table::VerticalMerge::Restart)
        );
    }

    #[test]
    fn test_table_with_explicit_header_row_renders_correctly() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:trPr>
          <w:tblHeader/>
        </w:trPr>
        <w:tc>
          <w:p><w:r><w:t>Name</w:t></w:r></w:p>
        </w:tc>
        <w:tc>
          <w:p><w:r><w:t>Age</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
      <w:tr>
        <w:tc>
          <w:p><w:r><w:t>Alice</w:t></w:r></w:p>
        </w:tc>
        <w:tc>
          <w:p><w:r><w:t>30</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

        let bytes = create_test_docx(xml);
        let mut budget = SecurityBudget::with_defaults();
        let doc = parse_document(&bytes, &mut budget, &default_limits()).expect("parse should succeed");

        assert_eq!(doc.tables.len(), 1);
        let table = &doc.tables[0];

        let row0_props = table.rows[0]
            .properties
            .as_ref()
            .expect("first row should have properties");
        assert!(row0_props.is_header, "First row should be marked as header");

        let markdown = table.to_markdown();
        let lines: Vec<&str> = markdown.lines().collect();

        assert!(
            lines.len() >= 3,
            "Table should have at least 3 lines, got: {}",
            markdown
        );

        assert!(
            lines[1].contains("---"),
            "Second line should be separator, got: {}",
            lines[1]
        );
    }

    #[test]
    fn test_table_with_merged_cells_expands_columns() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:tc>
          <w:p><w:r><w:t>A</w:t></w:r></w:p>
        </w:tc>
        <w:tc>
          <w:p><w:r><w:t>B</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
      <w:tr>
        <w:tc>
          <w:tcPr>
            <w:gridSpan w:val="2"/>
          </w:tcPr>
          <w:p><w:r><w:t>Merged</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

        let bytes = create_test_docx(xml);
        let mut budget = SecurityBudget::with_defaults();
        let doc = parse_document(&bytes, &mut budget, &default_limits()).expect("parse should succeed");

        assert_eq!(doc.tables.len(), 1);
        let table = &doc.tables[0];

        let merged_cell = &table.rows[1].cells[0];
        let cell_props = merged_cell.properties.as_ref().expect("cell should have properties");
        assert_eq!(cell_props.grid_span, Some(2), "Cell should have grid_span=2");

        let markdown = table.to_markdown();
        let lines: Vec<&str> = markdown.lines().collect();

        let pipes_row0 = lines[0].matches('|').count();
        let pipes_row1 = lines[2].matches('|').count();

        assert_eq!(
            pipes_row0, pipes_row1,
            "All rows should have same column count in markdown"
        );
    }

    /// Helper: parse document XML through DocxParser and return the Document.
    fn parse_xml(xml: &str) -> Document {
        let parser_struct = DocxParser {
            archive: zip::ZipArchive::new(std::io::Cursor::new(create_minimal_zip())).unwrap(),
            relationships: AHashMap::new(),
            styles: None,
            theme: None,
        };
        let mut document = Document::new();
        {
            let mut budget = crate::extractors::security::SecurityBudget::with_defaults();
            parser_struct
                .parse_document_xml(xml, &mut document, &mut budget)
                .unwrap();
        }
        document
    }

    /// Helper: parse document XML through DocxParser, threading a caller-supplied
    /// budget instead of a fresh default one, so the caller can inspect the budget's
    /// residual state (e.g. leaked nesting depth, GH#1395) after parsing completes.
    fn parse_xml_with_budget(xml: &str, budget: &mut SecurityBudget) -> Document {
        let parser_struct = DocxParser {
            archive: zip::ZipArchive::new(std::io::Cursor::new(create_minimal_zip())).unwrap(),
            relationships: AHashMap::new(),
            styles: None,
            theme: None,
        };
        let mut document = Document::new();
        parser_struct.parse_document_xml(xml, &mut document, budget).unwrap();
        document
    }

    /// Helper: parse document XML through DocxParser, returning the raw `Result`
    /// instead of unwrapping (GH#384) — needed to assert on the exact error variant
    /// produced once nesting inside a delegating helper's subtree trips the depth cap.
    fn try_parse_xml_with_budget(xml: &str, budget: &mut SecurityBudget) -> Result<Document, DocxParseError> {
        let parser_struct = DocxParser {
            archive: zip::ZipArchive::new(std::io::Cursor::new(create_minimal_zip())).unwrap(),
            relationships: AHashMap::new(),
            styles: None,
            theme: None,
        };
        let mut document = Document::new();
        parser_struct.parse_document_xml(xml, &mut document, budget)?;
        Ok(document)
    }

    /// Test-only probe (GH#1395): count how many more `budget.enter()` calls succeed
    /// before the depth cap trips. A freshly-built budget accepts exactly `max_depth`
    /// more entries; if parsing leaked N depth levels, only `max_depth - N` succeed.
    /// This makes the *value* of the private depth counter observable through the
    /// budget's existing public API, without touching `extractors/security.rs`.
    fn probe_remaining_depth(budget: &mut SecurityBudget, max_depth: usize) -> usize {
        let mut successes = 0usize;
        for _ in 0..=max_depth {
            if budget.enter().is_ok() {
                successes += 1;
            } else {
                break;
            }
        }
        successes
    }

    /// GH#1395: a single `w:tblPr` inside a table must not leave the depth counter
    /// above zero after parsing — `parse_table_properties` consumes its own
    /// `</w:tblPr>`, so the outer loop's `Event::End` arm never runs for it.

    #[test]
    fn test_grid_before_and_after_keep_cells_under_their_columns() {
        let xml = wrap_body(
            r#"<w:tbl>
  <w:tblGrid><w:gridCol w:w="2000"/><w:gridCol w:w="2000"/><w:gridCol w:w="2000"/></w:tblGrid>
  <w:tr>
    <w:tc><w:p><w:r><w:t>A</w:t></w:r></w:p></w:tc>
    <w:tc><w:p><w:r><w:t>B</w:t></w:r></w:p></w:tc>
    <w:tc><w:p><w:r><w:t>C</w:t></w:r></w:p></w:tc>
  </w:tr>
  <w:tr>
    <w:trPr><w:gridBefore w:val="1"/></w:trPr>
    <w:tc><w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t>b2</w:t></w:r></w:p></w:tc>
    <w:tc><w:p><w:r><w:t>c2</w:t></w:r></w:p></w:tc>
  </w:tr>
  <w:tr>
    <w:trPr><w:gridAfter w:val="1"/></w:trPr>
    <w:tc><w:p><w:r><w:t>a3</w:t></w:r></w:p></w:tc>
    <w:tc><w:p><w:r><w:t>b3</w:t></w:r></w:p></w:tc>
  </w:tr>
</w:tbl>"#,
        );
        let doc = parse_xml(&xml);
        let table = &doc.tables[0];

        assert_eq!(
            table.to_cell_grid(Paragraph::to_text),
            vec![vec!["A", "B", "C"], vec!["", "b2", "c2"], vec!["a3", "b3", ""]],
            "a row's cells must sit under the grid columns gridBefore/gridAfter leave them"
        );

        let styles = table.to_cell_style_grid();
        assert_eq!(styles.iter().map(Vec::len).collect::<Vec<_>>(), vec![3, 3, 3]);
        assert_eq!(styles[1][1].as_deref(), Some("Heading2"));
    }

    /// The skipped grid columns are cells too: they count against `max_table_cells` before
    /// any grid is built, so padding cannot allocate past the limit.
    #[test]
    fn test_grid_padding_counts_against_the_cell_budget() {
        let budget_for = |max_table_cells| {
            SecurityBudget::from_limits(&crate::extractors::security::SecurityLimits {
                max_table_cells,
                ..Default::default()
            })
        };
        // One physical cell between one skipped column on each side: three grid cells.
        let one_cell = wrap_body(
            r#"<w:tbl><w:tr><w:trPr><w:gridBefore w:val="1"/><w:gridAfter w:val="1"/></w:trPr><w:tc><w:p><w:r><w:t>x</w:t></w:r></w:p></w:tc></w:tr></w:tbl>"#,
        );
        assert!(try_parse_xml_with_budget(&one_cell, &mut budget_for(3)).is_ok());
        assert!(matches!(
            try_parse_xml_with_budget(&one_cell, &mut budget_for(2)),
            Err(DocxParseError::SecurityLimit(_))
        ));

        // A row with no cells at all still pays for the columns it skips.
        let no_cells = wrap_body(
            r#"<w:tbl><w:tr><w:trPr><w:gridBefore w:val="1024"/><w:gridAfter w:val="1024"/></w:trPr></w:tr></w:tbl>"#,
        );
        assert!(try_parse_xml_with_budget(&no_cells, &mut budget_for(2048)).is_ok());
        assert!(matches!(
            try_parse_xml_with_budget(&no_cells, &mut budget_for(2047)),
            Err(DocxParseError::SecurityLimit(_))
        ));
    }
    #[test]
    fn should_reset_depth_counter_to_zero_after_parsing_tblpr() {
        let limits = crate::extractors::security::SecurityLimits {
            max_nesting_depth: 64,
            max_xml_depth: 64,
            ..Default::default()
        };
        let mut budget = SecurityBudget::from_limits(&limits);
        let xml = wrap_body(
            r#"<w:tbl>
                <w:tblPr><w:tblStyle w:val="TableGrid"/></w:tblPr>
                <w:tr><w:tc><w:p><w:r><w:t>x</w:t></w:r></w:p></w:tc></w:tr>
            </w:tbl>"#,
        );
        parse_xml_with_budget(&xml, &mut budget);
        assert_eq!(
            probe_remaining_depth(&mut budget, 64),
            64,
            "w:tblPr must not leak a depth level"
        );
    }

    /// GH#1395: same as `w:tblPr` — `parse_table_grid` consumes its own `</w:tblGrid>`.
    #[test]
    fn should_reset_depth_counter_to_zero_after_parsing_tblgrid() {
        let limits = crate::extractors::security::SecurityLimits {
            max_nesting_depth: 64,
            max_xml_depth: 64,
            ..Default::default()
        };
        let mut budget = SecurityBudget::from_limits(&limits);
        let xml = wrap_body(
            r#"<w:tbl>
                <w:tblGrid><w:gridCol w:w="2500"/></w:tblGrid>
                <w:tr><w:tc><w:p><w:r><w:t>x</w:t></w:r></w:p></w:tc></w:tr>
            </w:tbl>"#,
        );
        parse_xml_with_budget(&xml, &mut budget);
        assert_eq!(
            probe_remaining_depth(&mut budget, 64),
            64,
            "w:tblGrid must not leak a depth level"
        );
    }

    /// GH#1395: `parse_row_properties` consumes its own `</w:trPr>`.
    #[test]
    fn should_reset_depth_counter_to_zero_after_parsing_trpr() {
        let limits = crate::extractors::security::SecurityLimits {
            max_nesting_depth: 64,
            max_xml_depth: 64,
            ..Default::default()
        };
        let mut budget = SecurityBudget::from_limits(&limits);
        let xml = wrap_body(
            r#"<w:tbl>
                <w:tr>
                    <w:trPr><w:tblHeader/></w:trPr>
                    <w:tc><w:p><w:r><w:t>x</w:t></w:r></w:p></w:tc>
                </w:tr>
            </w:tbl>"#,
        );
        parse_xml_with_budget(&xml, &mut budget);
        assert_eq!(
            probe_remaining_depth(&mut budget, 64),
            64,
            "w:trPr must not leak a depth level"
        );
    }

    /// GH#1395: `parse_cell_properties` consumes its own `</w:tcPr>`.
    #[test]
    fn should_reset_depth_counter_to_zero_after_parsing_tcpr() {
        let limits = crate::extractors::security::SecurityLimits {
            max_nesting_depth: 64,
            max_xml_depth: 64,
            ..Default::default()
        };
        let mut budget = SecurityBudget::from_limits(&limits);
        let xml = wrap_body(
            r#"<w:tbl>
                <w:tr>
                    <w:tc>
                        <w:tcPr><w:tcW w:w="2500" w:type="dxa"/></w:tcPr>
                        <w:p><w:r><w:t>x</w:t></w:r></w:p>
                    </w:tc>
                </w:tr>
            </w:tbl>"#,
        );
        parse_xml_with_budget(&xml, &mut budget);
        assert_eq!(
            probe_remaining_depth(&mut budget, 64),
            64,
            "w:tcPr must not leak a depth level"
        );
    }

    /// GH#1395: `parse_drawing` consumes its own `</w:drawing>`.
    #[test]
    fn should_reset_depth_counter_to_zero_after_parsing_drawing() {
        let limits = crate::extractors::security::SecurityLimits {
            max_nesting_depth: 64,
            max_xml_depth: 64,
            ..Default::default()
        };
        let mut budget = SecurityBudget::from_limits(&limits);
        let xml = wrap_body(
            r#"<w:p><w:r>
                <w:drawing>
                    <wp:inline xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">
                        <wp:extent cx="914400" cy="914400"/>
                        <wp:docPr id="1" name="Picture 1"/>
                    </wp:inline>
                </w:drawing>
            </w:r></w:p>"#,
        );
        parse_xml_with_budget(&xml, &mut budget);
        assert_eq!(
            probe_remaining_depth(&mut budget, 64),
            64,
            "w:drawing must not leak a depth level"
        );
    }

    /// GH#1395: `parse_section_properties_streaming` consumes its own `</w:sectPr>`.
    #[test]
    fn should_reset_depth_counter_to_zero_after_parsing_sectpr() {
        let limits = crate::extractors::security::SecurityLimits {
            max_nesting_depth: 64,
            max_xml_depth: 64,
            ..Default::default()
        };
        let mut budget = SecurityBudget::from_limits(&limits);
        let xml = wrap_body(
            r#"<w:p><w:r><w:t>Content</w:t></w:r></w:p>
            <w:sectPr>
                <w:pgSz w:w="12240" w:h="15840"/>
            </w:sectPr>"#,
        );
        parse_xml_with_budget(&xml, &mut budget);
        assert_eq!(
            probe_remaining_depth(&mut budget, 64),
            64,
            "w:sectPr must not leak a depth level"
        );
    }

    /// GH#1395: `m:oMath` and `m:oMathPara` already thread `budget` through to
    /// `collect_and_convert_omath{,_para}`, which recursively balance every
    /// `enter()`/`leave()` pair themselves — including the enter made by the
    /// outer loop for the `m:oMath`/`m:oMathPara` start tag itself, refunded when
    /// the recursive `collect_children` consumes the matching end tag. Confirm
    /// they are NOT double-leaking (over-refunding would also be a bug: it would
    /// silently mask genuinely deep documents by under-counting).
    #[test]
    fn should_reset_depth_counter_to_zero_after_parsing_omath_para() {
        let limits = crate::extractors::security::SecurityLimits {
            max_nesting_depth: 64,
            max_xml_depth: 64,
            ..Default::default()
        };
        let mut budget = SecurityBudget::from_limits(&limits);
        let xml = wrap_body(
            r#"<w:p><w:r>
                <m:oMathPara xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
                    <m:oMath>
                        <m:r><m:rPr><m:sty m:val="p"/></m:rPr><m:t>x</m:t></m:r>
                        <m:sSup>
                            <m:e><m:r><m:t>y</m:t></m:r></m:e>
                            <m:sup><m:r><m:t>2</m:t></m:r></m:sup>
                        </m:sSup>
                    </m:oMath>
                </m:oMathPara>
            </w:r></w:p>"#,
        );
        parse_xml_with_budget(&xml, &mut budget);
        assert_eq!(
            probe_remaining_depth(&mut budget, 64),
            64,
            "m:oMathPara/m:oMath (with nested m:rPr and m:sSup) must not leak a depth level"
        );
    }

    /// GH#1395: the reporter's reproducer — a flat one-column table whose real
    /// nesting depth is small (~8) leaked `2*rows + 3` levels because every row's
    /// `w:tcPr` and `w:trPr` each leaked one level. At 600 rows that is 1203 leaked
    /// levels, enough to trip the default 1024-level cap outright with no partial
    /// result. Assert the fixed parser extracts the full table at both 400 and 600
    /// rows, with identical, exact structural counts — proving the row count no
    /// longer influences whether extraction succeeds.
    #[test]
    fn should_extract_full_table_regardless_of_row_count_after_fixing_depth_leak() {
        fn build_table_xml(rows: usize) -> String {
            // Deliberately `<w:tblPr></w:tblPr>` (Start+End events), not the
            // self-closing `<w:tblPr/>` form (a single Empty event) — only the
            // Start/End form goes through the leaking `parse_table_properties`
            // delegation path this test is guarding against.
            let mut body = String::from("<w:tbl><w:tblPr></w:tblPr><w:tblGrid><w:gridCol w:w=\"2500\"/></w:tblGrid>");
            for i in 0..rows {
                body.push_str(&format!(
                    "<w:tr><w:trPr></w:trPr><w:tc><w:tcPr></w:tcPr><w:p><w:r><w:t>row{i}</w:t></w:r></w:p></w:tc></w:tr>"
                ));
            }
            body.push_str("</w:tbl>");
            wrap_body(&body)
        }

        for rows in [400usize, 600usize] {
            let mut budget = SecurityBudget::with_defaults();
            let xml = build_table_xml(rows);
            let doc = parse_xml_with_budget(&xml, &mut budget);

            assert_eq!(doc.tables.len(), 1, "rows={rows}: expected exactly 1 table");
            assert_eq!(doc.tables[0].rows.len(), rows, "rows={rows}: row count mismatch");
            assert_eq!(
                doc.elements.len(),
                1,
                "rows={rows}: a single top-level table must yield exactly 1 document element"
            );
            assert_eq!(
                probe_remaining_depth(&mut budget, 1024),
                1024,
                "rows={rows}: depth counter must return to zero after a {rows}-row table"
            );
        }
    }

    /// GH#384: the GH#1395 fix only stopped the depth counter from *leaking* at each
    /// delegating call site — it left the content *inside* those helpers completely
    /// unmeasured. `parse_drawing` is one of them: before this fix it read through its
    /// own `</w:drawing>` without ever calling `budget.enter()`/`leave()` for anything
    /// nested inside. Confirm a genuinely deeply-nested drawing subtree now trips the
    /// depth cap instead of sailing through unaccounted, and that the exact error
    /// variant surfaces (not just "parsing failed somehow").
    #[test]
    fn should_trip_nesting_too_deep_when_drawing_subtree_nests_beyond_the_depth_cap() {
        let limits = crate::extractors::security::SecurityLimits {
            max_nesting_depth: 10,
            max_xml_depth: 10,
            ..Default::default()
        };
        let mut budget = SecurityBudget::from_limits(&limits);

        let nest_depth = 20;
        let mut inner = String::from("<a:sp>");
        for _ in 0..nest_depth {
            inner.push_str("<a:grp>");
        }
        for _ in 0..nest_depth {
            inner.push_str("</a:grp>");
        }
        inner.push_str("</a:sp>");

        let xml = wrap_body(&format!(
            "<w:p><w:r><w:drawing><wp:inline>{inner}</wp:inline></w:drawing></w:r></w:p>"
        ));

        let result = try_parse_xml_with_budget(&xml, &mut budget);

        match result {
            Err(DocxParseError::SecurityLimit(msg)) => {
                assert!(
                    msg.contains("Nesting too deep"),
                    "expected a nesting-depth security error, got: {msg}"
                );
            }
            other => {
                panic!("expected Err(DocxParseError::SecurityLimit(_)) for a deeply-nested drawing, got {other:?}")
            }
        }
    }

    /// GH#384: same gap as `parse_drawing`, but for `parse_cell_properties` — nesting
    /// inside a `w:tcPr` subtree was completely invisible to the depth budget before
    /// this fix. Confirm it now trips the depth cap with the exact `SecurityLimit`
    /// error variant.
    #[test]
    fn should_trip_nesting_too_deep_when_tcpr_subtree_nests_beyond_the_depth_cap() {
        let limits = crate::extractors::security::SecurityLimits {
            max_nesting_depth: 10,
            max_xml_depth: 10,
            ..Default::default()
        };
        let mut budget = SecurityBudget::from_limits(&limits);

        let nest_depth = 20;
        let mut inner = String::new();
        for _ in 0..nest_depth {
            inner.push_str("<evil>");
        }
        for _ in 0..nest_depth {
            inner.push_str("</evil>");
        }

        let xml = wrap_body(&format!(
            "<w:tbl><w:tr><w:tc><w:tcPr>{inner}</w:tcPr><w:p><w:r><w:t>x</w:t></w:r></w:p></w:tc></w:tr></w:tbl>"
        ));

        let result = try_parse_xml_with_budget(&xml, &mut budget);

        match result {
            Err(DocxParseError::SecurityLimit(msg)) => {
                assert!(
                    msg.contains("Nesting too deep"),
                    "expected a nesting-depth security error, got: {msg}"
                );
            }
            other => panic!("expected Err(DocxParseError::SecurityLimit(_)) for a deeply-nested tcPr, got {other:?}"),
        }
    }

    /// a25335db0a threaded `budget` through every DOCX delegating helper except
    /// `parse_vml_pict`: it consumes its own `</w:pict>` end tag with a private
    /// `depth` counter, so the main loop's `Event::End` arm never sees the closing
    /// tag and `budget.leave()` is never called for it. Confirm the fixed parser
    /// leaves the depth counter at exactly zero after parsing a single `w:pict`.
    #[test]
    fn should_reset_depth_counter_to_zero_after_parsing_pict() {
        let limits = crate::extractors::security::SecurityLimits {
            max_nesting_depth: 64,
            max_xml_depth: 64,
            ..Default::default()
        };
        let mut budget = SecurityBudget::from_limits(&limits);
        let xml = wrap_body(
            r#"<w:p><w:r>
                <w:pict><v:shape xmlns:v="urn:schemas-microsoft-com:vml"><v:textbox><w:txbxContent>
                    <w:p><w:r><w:t>Legacy VML text box.</w:t></w:r></w:p>
                </w:txbxContent></v:textbox></v:shape></w:pict>
            </w:r></w:p>"#,
        );
        parse_xml_with_budget(&xml, &mut budget);
        assert_eq!(
            probe_remaining_depth(&mut budget, 64),
            64,
            "w:pict must not leak a depth level"
        );
    }

    /// Regression test for the `w:pict` depth leak (a25335db0a): before this fix,
    /// each `<w:pict>` element permanently leaked one depth unit (the main loop's
    /// `enter()` for its opening tag was never matched by a `leave()`), even though
    /// each element's *real* nesting is trivial and never overlaps with the next.
    /// Legacy `.doc`-to-`.docx` conversions and documents saved by older Word
    /// versions commonly contain many `w:pict` elements, so this leak would
    /// eventually trip `SecurityError::NestingTooDeep` on a perfectly legitimate
    /// document. Use a count comfortably above the default `max_nesting_depth`
    /// (1024) so the unfixed code genuinely fails this test.
    #[test]
    fn should_extract_all_pict_textboxes_regardless_of_element_count_after_fixing_depth_leak() {
        fn build_pict_xml(count: usize) -> String {
            let mut body = String::new();
            for i in 0..count {
                body.push_str(&format!(
                    r#"<w:p><w:r><w:pict><v:shape xmlns:v="urn:schemas-microsoft-com:vml"><v:textbox><w:txbxContent>
                        <w:p><w:r><w:t>pict{i}</w:t></w:r></w:p>
                    </w:txbxContent></v:textbox></v:shape></w:pict></w:r></w:p>"#
                ));
            }
            wrap_body(&body)
        }

        let count = 1500;
        let mut budget = SecurityBudget::with_defaults();
        let xml = build_pict_xml(count);
        let doc = try_parse_xml_with_budget(&xml, &mut budget)
            .expect("legacy VML w:pict content must not trip the depth cap");

        assert_eq!(
            doc.drawings.len(),
            count,
            "expected exactly {count} extracted text boxes"
        );
        assert_eq!(
            doc.paragraphs.len(),
            count,
            "each w:pict is wrapped in its own top-level w:p, so paragraph count must match"
        );
        assert_eq!(
            probe_remaining_depth(&mut budget, 1024),
            1024,
            "depth counter must return to zero after {count} w:pict elements"
        );
    }

    /// GH#384-style gap for `w:pict`: before this fix, none of `parse_vml_pict`,
    /// `parse_vml_textbox`, or `collect_txbx_content_text` called `budget.step()`,
    /// so content nested inside a `w:pict` was completely exempt from the
    /// iteration-count limit — a single `w:pict` padded with many trivial elements
    /// was processed with zero budget enforcement. Confirm the fixed parser now
    /// trips `SecurityError::TooManyIterations` once padding inside `w:pict`
    /// exceeds the configured cap, with the exact error variant surfaced.
    #[test]
    fn should_trip_too_many_iterations_when_pict_subtree_is_padded_beyond_the_iteration_cap() {
        let limits = crate::extractors::security::SecurityLimits {
            max_iterations: 50,
            ..Default::default()
        };
        let mut budget = SecurityBudget::from_limits(&limits);

        let padding_count = 200;
        let mut inner = String::new();
        for _ in 0..padding_count {
            inner.push_str("<v:fill/>");
        }

        let xml = wrap_body(&format!(
            r#"<w:p><w:r><w:pict><v:shape xmlns:v="urn:schemas-microsoft-com:vml">
                {inner}
            </v:shape></w:pict></w:r></w:p>"#
        ));

        let result = try_parse_xml_with_budget(&xml, &mut budget);

        match result {
            Err(DocxParseError::SecurityLimit(msg)) => {
                assert!(
                    msg.contains("Too many iterations"),
                    "expected an iteration-count security error, got: {msg}"
                );
            }
            other => {
                panic!("expected Err(DocxParseError::SecurityLimit(_)) for a padded w:pict subtree, got {other:?}")
            }
        }
    }

    /// GH#384: threading `budget` into the table/drawing/section delegating helpers
    /// must not turn ordinary, real-world-shaped documents into false rejections.
    /// Parses a table with populated `tblPr`/`tblGrid`/`trPr`/`tcPr`, an inline
    /// drawing, and a section — all well within the default depth cap — and asserts
    /// the structural results are still fully populated, not just that parsing
    /// returned `Ok`.
    #[test]
    fn should_extract_realistic_table_and_drawing_without_false_rejection() {
        let xml = wrap_body(
            r#"<w:tbl>
                <w:tblPr><w:tblStyle w:val="TableGrid"/><w:tblW w:w="5000" w:type="dxa"/></w:tblPr>
                <w:tblGrid><w:gridCol w:w="2500"/><w:gridCol w:w="2500"/></w:tblGrid>
                <w:tr>
                    <w:trPr><w:tblHeader/></w:trPr>
                    <w:tc>
                        <w:tcPr>
                            <w:tcW w:w="2500" w:type="dxa"/>
                            <w:shd w:val="clear" w:color="auto" w:fill="D9E2F3"/>
                        </w:tcPr>
                        <w:p><w:r><w:t>Header A</w:t></w:r></w:p>
                    </w:tc>
                    <w:tc>
                        <w:tcPr><w:tcW w:w="2500" w:type="dxa"/></w:tcPr>
                        <w:p><w:r><w:t>Header B</w:t></w:r></w:p>
                    </w:tc>
                </w:tr>
                <w:tr>
                    <w:tc>
                        <w:tcPr><w:tcW w:w="2500" w:type="dxa"/></w:tcPr>
                        <w:p><w:r><w:t>Row 1</w:t></w:r></w:p>
                    </w:tc>
                    <w:tc>
                        <w:tcPr><w:tcW w:w="2500" w:type="dxa"/></w:tcPr>
                        <w:p><w:r><w:t>Value</w:t></w:r></w:p>
                    </w:tc>
                </w:tr>
            </w:tbl>
            <w:p><w:r>
                <w:drawing>
                    <wp:inline xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">
                        <wp:extent cx="914400" cy="457200"/>
                        <wp:docPr id="1" name="Picture 1"/>
                    </wp:inline>
                </w:drawing>
            </w:r></w:p>
            <w:sectPr>
                <w:pgSz w:w="12240" w:h="15840"/>
                <w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"
                         w:header="720" w:footer="720" w:gutter="0"/>
            </w:sectPr>"#,
        );

        let mut budget = SecurityBudget::with_defaults();
        let doc = try_parse_xml_with_budget(&xml, &mut budget)
            .expect("a realistic table + drawing + section must not trip the default depth cap");

        assert_eq!(doc.tables.len(), 1, "expected exactly one table");
        assert_eq!(doc.tables[0].rows.len(), 2, "expected a header row and a data row");
        assert!(
            doc.tables[0].properties.is_some(),
            "table properties must still be populated"
        );
        assert!(doc.tables[0].grid.is_some(), "table grid must still be populated");
        assert_eq!(doc.drawings.len(), 1, "expected exactly one drawing");
        assert_eq!(doc.sections.len(), 1, "expected exactly one section");
        assert_eq!(
            doc.sections[0].page_width_twips,
            Some(12240),
            "section properties must still be populated"
        );
    }

    /// Helper: parse document XML with custom relationships.
    fn parse_xml_with_rels(xml: &str, rels: AHashMap<String, String>) -> Document {
        let parser_struct = DocxParser {
            archive: zip::ZipArchive::new(std::io::Cursor::new(create_minimal_zip())).unwrap(),
            relationships: rels,
            styles: None,
            theme: None,
        };
        let mut document = Document::new();
        {
            let mut budget = crate::extractors::security::SecurityBudget::with_defaults();
            parser_struct
                .parse_document_xml(xml, &mut document, &mut budget)
                .unwrap();
        }
        document
    }

    /// Wrap body XML in a w:document envelope.
    fn wrap_body(body: &str) -> String {
        format!(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body>{}</w:body></w:document>"#,
            body
        )
    }

    fn inline_drawing_xml(id: usize, height_emu: i64) -> String {
        format!(
            r#"<w:p><w:r><w:drawing>
                <wp:inline xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">
                    <wp:extent cx="5400000" cy="{height_emu}"/>
                    <wp:docPr id="{id}" name="Picture {id}"/>
                    <a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
                        <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
                            <pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
                                <pic:blipFill><a:blip r:embed="rId1"/></pic:blipFill>
                            </pic:pic>
                        </a:graphicData>
                    </a:graphic>
                </wp:inline>
            </w:drawing></w:r></w:p>"#
        )
    }

    #[test]
    fn test_plain_paragraph_text() {
        let xml = wrap_body(r#"<w:p><w:r><w:t>Hello World</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        assert_eq!(doc.paragraphs.len(), 1);
        assert_eq!(doc.paragraphs[0].to_text(), "Hello World");
    }

    #[test]
    fn test_multiple_paragraphs() {
        let xml = wrap_body(
            r#"<w:p><w:r><w:t>First</w:t></w:r></w:p>
               <w:p><w:r><w:t>Second</w:t></w:r></w:p>
               <w:p><w:r><w:t>Third</w:t></w:r></w:p>"#,
        );
        let doc = parse_xml(&xml);
        assert_eq!(doc.paragraphs.len(), 3);
        assert_eq!(doc.paragraphs[0].to_text(), "First");
        assert_eq!(doc.paragraphs[1].to_text(), "Second");
        assert_eq!(doc.paragraphs[2].to_text(), "Third");

        let plain = doc.to_plain_text();
        assert!(plain.contains("First"));
        assert!(plain.contains("Second"));
        assert!(plain.contains("Third"));
    }

    #[test]
    fn test_empty_paragraph() {
        let xml = wrap_body(r#"<w:p></w:p>"#);
        let doc = parse_xml(&xml);
        assert_eq!(doc.paragraphs.len(), 1);
        assert_eq!(doc.paragraphs[0].to_text(), "");
    }

    #[test]
    fn test_multiple_runs_in_paragraph() {
        let xml = wrap_body(
            r#"<w:p>
                <w:r><w:t>Hello </w:t></w:r>
                <w:r><w:t>World</w:t></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        assert_eq!(doc.paragraphs[0].to_text(), "Hello World");
    }

    #[test]
    fn should_not_record_a_note_reference_at_a_move_source() {
        let xml = wrap_body(
            r#"<w:p><w:moveFrom w:id="1" w:author="a"><w:r><w:t>Moved</w:t><w:footnoteReference w:id="2"/></w:r></w:moveFrom></w:p>
               <w:p><w:moveTo w:id="3" w:author="a"><w:r><w:t>Moved</w:t><w:footnoteReference w:id="2"/></w:r></w:moveTo></w:p>"#,
        );
        let doc = parse_xml(&xml);

        assert_eq!(doc.paragraphs[0].to_text(), "");
        assert_eq!(doc.paragraphs[1].to_text(), "Moved[^2]");
        let recorded: Vec<usize> = doc.note_refs_by_paragraph.keys().copied().collect();
        assert_eq!(recorded, vec![1], "only the move destination references the note");
    }

    #[test]
    fn test_line_break_in_run() {
        let xml = wrap_body(r#"<w:p><w:r><w:t>Before</w:t><w:br/><w:t>After</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        let text = doc.paragraphs[0].to_text();
        assert!(text.contains("Before"));
        assert!(text.contains("After"));
        assert!(text.contains('\n'));
    }

    #[test]
    fn test_carriage_return_in_run_breaks_the_line() {
        for cr in ["<w:cr/>", "<w:cr></w:cr>"] {
            let xml = wrap_body(&format!(
                r#"<w:p><w:r><w:t>line one</w:t>{cr}<w:t>line two</w:t></w:r></w:p>"#
            ));
            let doc = parse_xml(&xml);
            assert_eq!(doc.paragraphs[0].to_text(), "line one\nline two", "{cr}");
        }
    }

    #[test]
    fn test_position_tab_in_run_separates_the_words() {
        for tab in [
            r#"<w:ptab w:relativeTo="margin" w:alignment="right" w:leader="none"/>"#,
            r#"<w:ptab w:relativeTo="margin" w:alignment="right" w:leader="none"></w:ptab>"#,
            "<w:tab></w:tab>",
        ] {
            let xml = wrap_body(&format!(
                r#"<w:p><w:r><w:t>Report</w:t>{tab}<w:t>Page 1</w:t></w:r></w:p>"#
            ));
            let doc = parse_xml(&xml);
            assert_eq!(doc.paragraphs[0].to_text(), "Report\tPage 1", "{tab}");
        }
    }

    /// The same text box with self-closing and with paired run elements, including a paired
    /// tab stop definition, which must still add nothing.
    #[test]
    fn should_keep_breaks_and_tabs_but_not_tab_stops_in_textboxes() {
        let self_closing = r#"<w:p><w:pPr><w:tabs><w:tab w:val="left" w:pos="720"/></w:tabs></w:pPr>
                   <w:r><w:t>one</w:t><w:cr/><w:t>two</w:t><w:ptab w:relativeTo="margin" w:alignment="right" w:leader="none"/><w:t>three</w:t><w:tab/><w:t>four</w:t></w:r></w:p>"#;
        let paired = r#"<w:p><w:pPr><w:tabs><w:tab w:val="left" w:pos="720"></w:tab></w:tabs></w:pPr>
                   <w:r><w:t>one</w:t><w:cr></w:cr><w:t>two</w:t><w:ptab w:relativeTo="margin" w:alignment="right" w:leader="none"></w:ptab><w:t>three</w:t><w:tab></w:tab><w:t>four</w:t></w:r></w:p>"#;
        for content in [self_closing, paired] {
            let xml = wrap_body(&format!(
                r#"<w:p><w:r><w:pict><v:shape xmlns:v="urn:schemas-microsoft-com:vml"><v:textbox><w:txbxContent>
                 {content}
               </w:txbxContent></v:textbox></v:shape></w:pict></w:r></w:p>"#
            ));
            let limits = crate::extractors::security::SecurityLimits {
                max_nesting_depth: 64,
                max_xml_depth: 64,
                ..Default::default()
            };
            let mut budget = SecurityBudget::from_limits(&limits);
            let doc = parse_xml_with_budget(&xml, &mut budget);
            let textboxes: Vec<&str> = doc
                .drawings
                .iter()
                .filter_map(|drawing| drawing.text_box_content.as_deref())
                .collect();
            assert_eq!(textboxes, vec!["one\ntwo\tthree\tfour"], "{content}");
            assert_eq!(
                probe_remaining_depth(&mut budget, 64),
                64,
                "paired run elements must not leak a depth level"
            );
        }
    }

    /// Word writes both `<w:br w:type="page"/>` (the author's break) and, at the
    /// start of the first run on the new page, `<w:lastRenderedPageBreak/>` (its own
    /// record of the same transition). Counting both inflates the page total and
    /// produces a spurious zero-length page between them (GH#1416).
    #[test]
    fn should_yield_one_page_boundary_when_manual_break_is_followed_by_last_rendered_hint() {
        let xml = wrap_body(
            r#"<w:p><w:r><w:t>Page one text</w:t><w:br w:type="page"/></w:r></w:p>
               <w:p><w:r><w:lastRenderedPageBreak/><w:t>Page two text</w:t></w:r></w:p>"#,
        );
        let doc = parse_xml(&xml);

        let page_break_count = doc
            .elements
            .iter()
            .filter(|e| matches!(e, DocumentElement::PageBreak))
            .count();
        assert_eq!(
            page_break_count, 1,
            "the render hint duplicates the manual break and must not be counted again"
        );

        let (text, boundaries) = doc.extract_text_with_boundaries(true, true);
        assert_eq!(boundaries.len(), 2, "one break must produce exactly two pages");
        assert_eq!(boundaries[0].page_number, 1);
        assert_eq!(boundaries[1].page_number, 2);
        assert_ne!(
            boundaries[0].byte_start, boundaries[0].byte_end,
            "page 1 must not be reported as a zero-length blank page"
        );
        assert!(text[boundaries[0].byte_start..boundaries[0].byte_end].contains("Page one text"));
        assert!(text[boundaries[1].byte_start..boundaries[1].byte_end].contains("Page two text"));
    }

    /// A document Word paginated itself (no manual `w:br` at all) carries only
    /// `w:lastRenderedPageBreak` hints. The fix for GH#1416 must not drop those
    /// outright — that's the only page-break signal such a document has.
    #[test]
    fn should_paginate_when_document_has_only_last_rendered_page_break_hints() {
        let xml = wrap_body(
            r#"<w:p><w:r><w:t>Page one text</w:t></w:r></w:p>
               <w:p><w:r><w:lastRenderedPageBreak/><w:t>Page two text</w:t></w:r></w:p>"#,
        );
        let doc = parse_xml(&xml);

        let page_break_count = doc
            .elements
            .iter()
            .filter(|e| matches!(e, DocumentElement::PageBreak))
            .count();
        assert_eq!(
            page_break_count, 1,
            "the sole render hint must still register a page break"
        );

        let (text, boundaries) = doc.extract_text_with_boundaries(true, true);
        assert_eq!(boundaries.len(), 2);
        assert_eq!(boundaries[0].page_number, 1);
        assert_eq!(boundaries[1].page_number, 2);
        assert!(text[boundaries[0].byte_start..boundaries[0].byte_end].contains("Page one text"));
        assert!(text[boundaries[1].byte_start..boundaries[1].byte_end].contains("Page two text"));
    }

    /// A page break inside a table cannot be written into the rendered markdown
    /// without splitting the table in half, so it must be deferred to right after
    /// the completed `DocumentElement::Table` — not dropped (GH#1419).
    #[test]
    fn should_place_deferred_break_after_table_when_break_occurs_inside_table() {
        let xml = wrap_body(
            r#"<w:p><w:r><w:t>Before table</w:t></w:r></w:p>
               <w:tbl>
                 <w:tblPr></w:tblPr>
                 <w:tblGrid><w:gridCol w:w="2000"/></w:tblGrid>
                 <w:tr><w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr>
                     <w:p><w:r><w:t>Cell one</w:t><w:br w:type="page"/></w:r></w:p>
                 </w:tc></w:tr>
                 <w:tr><w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr>
                     <w:p><w:r><w:t>Cell two</w:t></w:r></w:p>
                 </w:tc></w:tr>
               </w:tbl>
               <w:p><w:r><w:t>After table</w:t></w:r></w:p>"#,
        );
        let doc = parse_xml(&xml);

        assert_eq!(doc.tables.len(), 1);
        let table_markdown = doc.tables[0].to_markdown();
        assert!(
            !table_markdown.contains('\x0c'),
            "the table markdown must not contain a form feed: {table_markdown}"
        );
        assert!(table_markdown.contains("Cell one"));
        assert!(table_markdown.contains("Cell two"));

        // Elements must read: Paragraph, Table, PageBreak, Paragraph — the break comes
        // *after* the table, not inside it and not dropped.
        let kinds: Vec<&str> = doc
            .elements
            .iter()
            .map(|e| match e {
                DocumentElement::Paragraph(_) => "paragraph",
                DocumentElement::Table(_) => "table",
                DocumentElement::PageBreak => "page_break",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, vec!["paragraph", "table", "page_break", "paragraph"]);

        let (text, boundaries) = doc.extract_text_with_boundaries(true, true);
        assert_eq!(
            boundaries.len(),
            2,
            "the table-internal break must still produce two pages"
        );
        let page_one = &text[boundaries[0].byte_start..boundaries[0].byte_end];
        let page_two = &text[boundaries[1].byte_start..boundaries[1].byte_end];
        assert!(page_one.contains("Before table"));
        assert!(page_one.contains("Cell one"));
        assert!(
            page_one.contains("Cell two"),
            "the table must not be split across the boundary"
        );
        assert!(page_two.contains("After table"));
    }

    /// A break inside a *nested* table must be deferred just like a top-level one,
    /// but flushed exactly once when the outermost `</w:tbl>` closes — not once per
    /// nesting level (GH#1419).
    #[test]
    fn should_not_double_count_page_break_inside_nested_table() {
        let xml = wrap_body(
            r#"<w:p><w:r><w:t>Before table</w:t></w:r></w:p>
               <w:tbl>
                 <w:tblPr></w:tblPr>
                 <w:tblGrid><w:gridCol w:w="2000"/></w:tblGrid>
                 <w:tr><w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr>
                     <w:tbl>
                       <w:tblPr></w:tblPr>
                       <w:tblGrid><w:gridCol w:w="1000"/></w:tblGrid>
                       <w:tr><w:tc><w:tcPr><w:tcW w:w="1000" w:type="dxa"/></w:tcPr>
                           <w:p><w:r><w:t>Inner cell</w:t><w:br w:type="page"/></w:r></w:p>
                       </w:tc></w:tr>
                     </w:tbl>
                 </w:tc></w:tr>
               </w:tbl>
               <w:p><w:r><w:t>After table</w:t></w:r></w:p>"#,
        );
        let doc = parse_xml(&xml);

        assert_eq!(
            doc.tables.len(),
            1,
            "the inner table is flattened into the outer cell, not a separate element"
        );
        let page_break_count = doc
            .elements
            .iter()
            .filter(|e| matches!(e, DocumentElement::PageBreak))
            .count();
        assert_eq!(
            page_break_count, 1,
            "the break must be flushed once at the outermost table close, not once per nesting level"
        );

        let (_, boundaries) = doc.extract_text_with_boundaries(true, true);
        assert_eq!(boundaries.len(), 2);
    }

    /// Word writes the render hint for a break it already saw even when that break
    /// was inside a table: the hint appears right after the table in the next
    /// paragraph, with no real content between the deferred break and the hint.
    /// That must still collapse to a single boundary (GH#1416 + GH#1419 combined).
    #[test]
    fn should_dedupe_last_rendered_hint_immediately_following_deferred_table_break() {
        let xml = wrap_body(
            r#"<w:p><w:r><w:t>Before table</w:t></w:r></w:p>
               <w:tbl>
                 <w:tblPr></w:tblPr>
                 <w:tblGrid><w:gridCol w:w="2000"/></w:tblGrid>
                 <w:tr><w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr>
                     <w:p><w:r><w:t>Cell one</w:t><w:br w:type="page"/></w:r></w:p>
                 </w:tc></w:tr>
               </w:tbl>
               <w:p><w:r><w:lastRenderedPageBreak/><w:t>After table</w:t></w:r></w:p>"#,
        );
        let doc = parse_xml(&xml);

        let page_break_count = doc
            .elements
            .iter()
            .filter(|e| matches!(e, DocumentElement::PageBreak))
            .count();
        assert_eq!(
            page_break_count, 1,
            "the hint right after the table duplicates the deferred table break"
        );

        let (text, boundaries) = doc.extract_text_with_boundaries(true, true);
        assert_eq!(boundaries.len(), 2);
        assert_ne!(
            boundaries[0].byte_start, boundaries[0].byte_end,
            "page 1 must not be reported as a zero-length blank page"
        );
        assert!(text[boundaries[1].byte_start..boundaries[1].byte_end].contains("After table"));
    }

    /// GH#1592 reproducer `A-body`: page breaks between body paragraphs, outside any
    /// table. No table-deferral logic is exercised here at all; this is the baseline
    /// the table cases below are compared against.
    #[test]
    fn gh1592_body_only_breaks_count_once_each() {
        let xml = wrap_body(
            r#"<w:p><w:r><w:t>body 0</w:t></w:r></w:p>
               <w:p><w:r><w:lastRenderedPageBreak/><w:t>body 1</w:t></w:r></w:p>
               <w:p><w:r><w:lastRenderedPageBreak/><w:t>body 2</w:t></w:r></w:p>"#,
        );
        let doc = parse_xml(&xml);

        let (_, boundaries) = doc.extract_text_with_boundaries(true, true);
        assert_eq!(
            boundaries.len(),
            3,
            "two breaks between body paragraphs must yield three pages"
        );
    }

    /// GH#1592 reproducer `B-rows`: one `lastRenderedPageBreak` hint per row, written
    /// into the first cell only — the shape Word writes when a row's own single-cell
    /// content is what straddles the boundary. Already correct before the fix; kept as
    /// a regression guard alongside `C`.
    #[test]
    fn gh1592_one_hint_per_row_counts_once_per_row() {
        let xml = wrap_body(
            r#"<w:p><w:r><w:t>before</w:t></w:r></w:p>
               <w:tbl><w:tblPr></w:tblPr><w:tblGrid><w:gridCol w:w="2000"/><w:gridCol w:w="2000"/></w:tblGrid>
                 <w:tr><w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr>
                     <w:p><w:r><w:lastRenderedPageBreak/></w:r><w:r><w:t>r0c0</w:t></w:r></w:p>
                 </w:tc><w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr>
                     <w:p><w:r><w:t>r0c1</w:t></w:r></w:p>
                 </w:tc></w:tr>
                 <w:tr><w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr>
                     <w:p><w:r><w:lastRenderedPageBreak/></w:r><w:r><w:t>r1c0</w:t></w:r></w:p>
                 </w:tc><w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr>
                     <w:p><w:r><w:t>r1c1</w:t></w:r></w:p>
                 </w:tc></w:tr>
                 <w:tr><w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr>
                     <w:p><w:r><w:lastRenderedPageBreak/></w:r><w:r><w:t>r2c0</w:t></w:r></w:p>
                 </w:tc><w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr>
                     <w:p><w:r><w:t>r2c1</w:t></w:r></w:p>
                 </w:tc></w:tr>
               </w:tbl>
               <w:p><w:r><w:t>after</w:t></w:r></w:p>"#,
        );
        let doc = parse_xml(&xml);

        let (_, boundaries) = doc.extract_text_with_boundaries(true, true);
        assert_eq!(boundaries.len(), 4, "three rows, one hint each, must yield four pages");
    }

    /// GH#1592 reproducer `C-both-cells`: the defect. The *same* three physical breaks
    /// as `B` above, but Word wrote each row's hint into *both* cells — the shape Word
    /// actually produces for a straddling row. Duplicates across cells of one row must
    /// collapse to a single break; rows must still count separately from each other.
    #[test]
    fn gh1592_hint_duplicated_into_every_cell_of_a_row_counts_once_per_row() {
        let xml = wrap_body(
            r#"<w:p><w:r><w:t>before</w:t></w:r></w:p>
               <w:tbl><w:tblPr></w:tblPr><w:tblGrid><w:gridCol w:w="2000"/><w:gridCol w:w="2000"/></w:tblGrid>
                 <w:tr><w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr>
                     <w:p><w:r><w:lastRenderedPageBreak/></w:r><w:r><w:t>r0c0</w:t></w:r></w:p>
                 </w:tc><w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr>
                     <w:p><w:r><w:lastRenderedPageBreak/></w:r><w:r><w:t>r0c1</w:t></w:r></w:p>
                 </w:tc></w:tr>
                 <w:tr><w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr>
                     <w:p><w:r><w:lastRenderedPageBreak/></w:r><w:r><w:t>r1c0</w:t></w:r></w:p>
                 </w:tc><w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr>
                     <w:p><w:r><w:lastRenderedPageBreak/></w:r><w:r><w:t>r1c1</w:t></w:r></w:p>
                 </w:tc></w:tr>
                 <w:tr><w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr>
                     <w:p><w:r><w:lastRenderedPageBreak/></w:r><w:r><w:t>r2c0</w:t></w:r></w:p>
                 </w:tc><w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr>
                     <w:p><w:r><w:lastRenderedPageBreak/></w:r><w:r><w:t>r2c1</w:t></w:r></w:p>
                 </w:tc></w:tr>
               </w:tbl>
               <w:p><w:r><w:t>after</w:t></w:r></w:p>"#,
        );
        let doc = parse_xml(&xml);

        let page_break_count = doc
            .elements
            .iter()
            .filter(|e| matches!(e, DocumentElement::PageBreak))
            .count();
        assert_eq!(
            page_break_count, 3,
            "three rows must contribute three breaks, not six (one per duplicated hint) \
             or one (collapsed as a single run)"
        );

        let (_, boundaries) = doc.extract_text_with_boundaries(true, true);
        assert_eq!(
            boundaries.len(),
            4,
            "the per-cell duplicated hint must not inflate or collapse the row count"
        );
    }

    /// GH#1592 reproducer `D-one-cell-deep`: all breaks fall inside a single deep cell
    /// of a single row, each preceded by that cell's own paragraph text. These are
    /// genuinely distinct transitions and must all count, even though they share both
    /// the table and the row with each other — the row-level dedup added for `C` must
    /// key on cell identity too, or this collapses to one break exactly like `C` did.
    #[test]
    fn gh1592_multiple_hints_in_one_deep_cell_of_one_row_all_count() {
        let xml = wrap_body(
            r#"<w:p><w:r><w:t>before</w:t></w:r></w:p>
               <w:tbl><w:tblPr></w:tblPr><w:tblGrid><w:gridCol w:w="2000"/><w:gridCol w:w="2000"/></w:tblGrid>
                 <w:tr><w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr>
                     <w:p><w:r><w:t>long 0</w:t></w:r></w:p>
                     <w:p><w:r><w:lastRenderedPageBreak/></w:r><w:r><w:t>long 1</w:t></w:r></w:p>
                     <w:p><w:r><w:lastRenderedPageBreak/></w:r><w:r><w:t>long 2</w:t></w:r></w:p>
                     <w:p><w:r><w:lastRenderedPageBreak/></w:r><w:r><w:t>long 3</w:t></w:r></w:p>
                 </w:tc><w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr>
                     <w:p><w:r><w:t>side</w:t></w:r></w:p>
                 </w:tc></w:tr>
               </w:tbl>
               <w:p><w:r><w:t>after</w:t></w:r></w:p>"#,
        );
        let doc = parse_xml(&xml);

        let page_break_count = doc
            .elements
            .iter()
            .filter(|e| matches!(e, DocumentElement::PageBreak))
            .count();
        assert_eq!(
            page_break_count, 3,
            "three hints inside one cell of one row are three distinct transitions"
        );

        let (_, boundaries) = doc.extract_text_with_boundaries(true, true);
        assert_eq!(boundaries.len(), 4);
    }

    /// GH#1559: Word can paginate a vertical block of inline images without
    /// writing a `lastRenderedPageBreak` for one of the transitions.
    #[test]
    fn should_infer_missing_page_break_when_inline_drawings_exceed_section_height() {
        let xml = wrap_body(&format!(
            r#"<w:p><w:r><w:t>PAGE ONE TEXT</w:t></w:r></w:p>
               {}
               <w:p><w:r><w:lastRenderedPageBreak/><w:t>PAGE TWO TEXT</w:t></w:r></w:p>
               {}{}{}
               <w:p><w:r><w:lastRenderedPageBreak/><w:t>TAIL MARKER</w:t></w:r></w:p>
               <w:sectPr>
                 <w:pgSz w:w="11906" w:h="16838"/>
                 <w:pgMar w:top="1417" w:right="1417" w:bottom="1417" w:left="1417"/>
               </w:sectPr>"#,
            inline_drawing_xml(1, 4_000_000),
            inline_drawing_xml(2, 3_500_000),
            inline_drawing_xml(3, 3_500_000),
            inline_drawing_xml(4, 3_500_000),
        ));
        let doc = parse_xml(&xml);

        assert_eq!(doc.drawing_page_numbers(), vec![1, 2, 2, 3]);
        let page_breaks = doc
            .elements
            .iter()
            .filter(|element| matches!(element, DocumentElement::PageBreak))
            .count();
        assert_eq!(page_breaks, 3, "two recorded breaks plus one inferred break");

        let (text, boundaries) = doc.extract_text_with_boundaries(true, true);
        assert_eq!(boundaries.len(), 4);
        let tail = &boundaries[3];
        assert!(text[tail.byte_start..tail.byte_end].contains("TAIL MARKER"));
    }

    /// Section properties describe the content that precedes them. A paragraph-level
    /// `sectPr` therefore closes the first span after that paragraph, while the final
    /// body-level `sectPr` closes the second span (#1559).
    #[test]
    fn should_use_the_geometry_of_each_section_for_inferred_breaks() {
        let xml = wrap_body(&format!(
            r#"{}{}
               <w:p><w:pPr><w:sectPr>
                 <w:pgSz w:w="10000" w:h="10000"/>
                 <w:pgMar w:top="1000" w:right="1000" w:bottom="1000" w:left="1000"/>
               </w:sectPr></w:pPr><w:r><w:t>END FIRST SECTION</w:t></w:r></w:p>
               {}{}
               <w:sectPr>
                 <w:pgSz w:w="12000" w:h="12000"/>
                 <w:pgMar w:top="500" w:right="500" w:bottom="500" w:left="500"/>
               </w:sectPr>"#,
            inline_drawing_xml(1, 3_000_000),
            inline_drawing_xml(2, 3_000_000),
            inline_drawing_xml(3, 3_000_000),
            inline_drawing_xml(4, 3_000_000),
        ));
        let doc = parse_xml(&xml);

        assert_eq!(doc.sections.len(), 2);
        assert_eq!(doc.sections[0].page_height_twips, Some(10_000));
        assert_eq!(doc.sections[1].page_height_twips, Some(12_000));
        let page_breaks = doc
            .elements
            .iter()
            .filter(|element| matches!(element, DocumentElement::PageBreak))
            .count();
        assert_eq!(page_breaks, 1, "only the tighter first section should overflow");
        assert_eq!(doc.drawing_page_numbers(), vec![1, 2, 2, 2]);
    }

    #[test]
    fn should_skip_only_a_section_whose_page_geometry_is_incomplete() {
        let xml = wrap_body(&format!(
            r#"{}{}
               <w:p><w:pPr><w:sectPr><w:pgSz w:w="10000" w:h="10000"/></w:sectPr></w:pPr></w:p>
               {}{}
               <w:sectPr>
                 <w:pgSz w:w="10000" w:h="10000"/>
                 <w:pgMar w:top="1000" w:right="1000" w:bottom="1000" w:left="1000"/>
               </w:sectPr>"#,
            inline_drawing_xml(1, 3_000_000),
            inline_drawing_xml(2, 3_000_000),
            inline_drawing_xml(3, 3_000_000),
            inline_drawing_xml(4, 3_000_000),
        ));
        let doc = parse_xml(&xml);

        assert_eq!(doc.drawing_page_numbers(), vec![1, 1, 1, 2]);
    }

    #[test]
    fn should_not_create_a_blank_page_for_one_oversized_inline_drawing() {
        let xml = wrap_body(&format!(
            r#"{}
               <w:sectPr>
                 <w:pgSz w:w="10000" w:h="10000"/>
                 <w:pgMar w:top="1000" w:right="1000" w:bottom="1000" w:left="1000"/>
               </w:sectPr>"#,
            inline_drawing_xml(1, 6_000_000),
        ));
        let doc = parse_xml(&xml);
        assert!(
            !doc.elements
                .iter()
                .any(|element| matches!(element, DocumentElement::PageBreak))
        );
        assert_eq!(doc.drawing_page_numbers(), vec![1]);
    }

    #[test]
    fn should_ignore_anchored_and_dimensionless_drawings_and_allow_an_exact_fit() {
        let mut elements = vec![
            DocumentElement::Drawing(0),
            DocumentElement::Drawing(1),
            DocumentElement::Drawing(2),
            DocumentElement::Drawing(3),
            DocumentElement::Drawing(4),
        ];
        let drawings = vec![
            super::super::drawing::Drawing {
                extent: Some(super::super::drawing::Extent { cx: 1, cy: 4_000_000 }),
                ..Default::default()
            },
            super::super::drawing::Drawing {
                drawing_type: super::super::drawing::DrawingType::Anchored(Default::default()),
                extent: Some(super::super::drawing::Extent { cx: 1, cy: 4_000_000 }),
                ..Default::default()
            },
            super::super::drawing::Drawing::default(),
            super::super::drawing::Drawing {
                extent: Some(super::super::drawing::Extent { cx: 1, cy: 1_080_000 }),
                ..Default::default()
            },
            super::super::drawing::Drawing {
                extent: Some(super::super::drawing::Extent { cx: 1, cy: -1 }),
                ..Default::default()
            },
        ];
        let sections = vec![ParsedSection {
            properties: super::super::section::SectionProperties {
                page_height_twips: Some(10_000),
                margins: super::super::section::PageMargins {
                    top: Some(1_000),
                    bottom: Some(1_000),
                    ..Default::default()
                },
                ..Default::default()
            },
            end_element_index: Some(elements.len()),
        }];

        insert_missing_inline_drawing_page_breaks(&mut elements, &drawings, &sections, false);

        assert!(
            !elements
                .iter()
                .any(|element| matches!(element, DocumentElement::PageBreak)),
            "4,000,000 + 1,080,000 EMU exactly fills the column"
        );

        let mut reset_elements = vec![
            DocumentElement::Drawing(0),
            DocumentElement::PageBreak,
            DocumentElement::Drawing(0),
        ];
        let reset_sections = vec![ParsedSection {
            properties: sections[0].properties.clone(),
            end_element_index: Some(reset_elements.len()),
        }];
        insert_missing_inline_drawing_page_breaks(&mut reset_elements, &drawings, &reset_sections, false);
        assert_eq!(
            reset_elements
                .iter()
                .filter(|element| matches!(element, DocumentElement::PageBreak))
                .count(),
            1,
            "the recorded break must reset the height budget without gaining a duplicate"
        );
    }

    #[test]
    fn should_disable_inference_when_a_section_is_nested_in_a_table() {
        let xml = wrap_body(&format!(
            r#"{}{}
               <w:tbl><w:tblPr/><w:tblGrid><w:gridCol w:w="2000"/></w:tblGrid>
                 <w:tr><w:tc><w:p><w:pPr><w:sectPr>
                   <w:pgSz w:w="10000" w:h="10000"/>
                   <w:pgMar w:top="1000" w:bottom="1000"/>
                 </w:sectPr></w:pPr></w:p></w:tc></w:tr>
               </w:tbl>
               <w:sectPr>
                 <w:pgSz w:w="10000" w:h="10000"/>
                 <w:pgMar w:top="1000" w:bottom="1000"/>
               </w:sectPr>"#,
            inline_drawing_xml(1, 3_000_000),
            inline_drawing_xml(2, 3_000_000),
        ));
        let doc = parse_xml(&xml);

        assert!(
            !doc.elements
                .iter()
                .any(|element| matches!(element, DocumentElement::PageBreak)),
            "ambiguous section ownership must disable synthetic pagination"
        );
    }

    #[test]
    fn should_disable_inference_for_nonmonotonic_section_endpoints() {
        let mut elements = vec![DocumentElement::Drawing(0), DocumentElement::Drawing(1)];
        let drawings = vec![
            super::super::drawing::Drawing {
                extent: Some(super::super::drawing::Extent { cx: 1, cy: 3_000_000 }),
                ..Default::default()
            },
            super::super::drawing::Drawing {
                extent: Some(super::super::drawing::Extent { cx: 1, cy: 3_000_000 }),
                ..Default::default()
            },
        ];
        let properties = super::super::section::SectionProperties {
            page_height_twips: Some(10_000),
            margins: super::super::section::PageMargins {
                top: Some(1_000),
                bottom: Some(1_000),
                ..Default::default()
            },
            ..Default::default()
        };
        let sections = vec![
            ParsedSection {
                properties: properties.clone(),
                end_element_index: Some(2),
            },
            ParsedSection {
                properties,
                end_element_index: Some(1),
            },
        ];

        insert_missing_inline_drawing_page_breaks(&mut elements, &drawings, &sections, false);

        assert!(
            !elements
                .iter()
                .any(|element| matches!(element, DocumentElement::PageBreak)),
            "invalid section ownership must leave the original elements untouched"
        );
    }

    /// GH#1562: quick-xml emits XML and numeric references as `GeneralRef`
    /// events, so both text-box routes must resolve them explicitly.
    #[test]
    fn should_preserve_entity_references_in_drawingml_and_vml_textboxes() {
        let escaped = "Research &amp; Development &lt;tagged&gt; &#8364;50 &amp; more";
        let xml = wrap_body(&format!(
            r#"<w:p><w:r><w:t>BODY: {escaped}</w:t></w:r></w:p>
               <w:p><w:r><w:drawing>
                 <wps:wsp xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape">
                   <wps:txbx><w:txbxContent><w:p><w:r><w:t>{escaped}</w:t></w:r></w:p></w:txbxContent></wps:txbx>
                 </wps:wsp>
               </w:drawing></w:r></w:p>
               <w:p><w:r><w:pict><v:shape xmlns:v="urn:schemas-microsoft-com:vml">
                 <v:textbox><w:txbxContent><w:p><w:r><w:t>{escaped}</w:t></w:r></w:p></w:txbxContent></v:textbox>
               </v:shape></w:pict></w:r></w:p>"#,
        ));
        let doc = parse_xml(&xml);
        let expected = "Research & Development <tagged> €50 & more";

        assert_eq!(doc.paragraphs[0].to_text(), format!("BODY: {expected}"));
        let textboxes: Vec<&str> = doc
            .drawings
            .iter()
            .filter_map(|drawing| drawing.text_box_content.as_deref())
            .collect();
        assert_eq!(textboxes, vec![expected, expected]);
    }

    #[test]
    fn should_account_textbox_text_against_the_content_budget() {
        let xml = wrap_body(
            r#"<w:p><w:r><w:drawing>
                 <wps:wsp xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape">
                   <wps:txbx><w:txbxContent><w:p><w:r><w:t>1234&amp;6789</w:t></w:r></w:p></w:txbxContent></wps:txbx>
                 </wps:wsp>
               </w:drawing></w:r></w:p>"#,
        );
        let limits = crate::extractors::security::SecurityLimits {
            max_content_size: 8,
            ..Default::default()
        };
        let mut budget = SecurityBudget::from_limits(&limits);

        let result = try_parse_xml_with_budget(&xml, &mut budget);
        assert!(matches!(result, Err(DocxParseError::SecurityLimit(_))));
    }

    #[test]
    fn test_bold_formatting() {
        let xml = wrap_body(r#"<w:p><w:r><w:rPr><w:b/></w:rPr><w:t>Bold</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        assert!(doc.paragraphs[0].runs[0].bold);
        let md = doc.to_markdown(true);
        assert!(md.contains("**Bold**"), "Markdown: {}", md);
    }

    #[test]
    fn test_bold_disabled_with_val_0() {
        let xml = wrap_body(r#"<w:p><w:r><w:rPr><w:b w:val="0"/></w:rPr><w:t>Not Bold</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        assert!(!doc.paragraphs[0].runs[0].bold);
    }

    #[test]
    fn test_italic_formatting() {
        let xml = wrap_body(r#"<w:p><w:r><w:rPr><w:i/></w:rPr><w:t>Italic</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        assert!(doc.paragraphs[0].runs[0].italic);
        let md = doc.to_markdown(true);
        assert!(md.contains("*Italic*"), "Markdown: {}", md);
    }

    #[test]
    fn test_bold_italic_combined() {
        let xml = wrap_body(r#"<w:p><w:r><w:rPr><w:b/><w:i/></w:rPr><w:t>Both</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        let run = &doc.paragraphs[0].runs[0];
        assert!(run.bold);
        assert!(run.italic);
        let md = doc.to_markdown(true);
        assert!(md.contains("***Both***"), "Markdown: {}", md);
    }

    #[test]
    fn test_underline_formatting() {
        let xml = wrap_body(r#"<w:p><w:r><w:rPr><w:u w:val="single"/></w:rPr><w:t>Underlined</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        assert!(doc.paragraphs[0].runs[0].underline);
    }

    #[test]
    fn test_underline_none_disabled() {
        let xml = wrap_body(r#"<w:p><w:r><w:rPr><w:u w:val="none"/></w:rPr><w:t>No Underline</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        assert!(!doc.paragraphs[0].runs[0].underline);
    }

    #[test]
    fn test_strikethrough_formatting() {
        let xml = wrap_body(r#"<w:p><w:r><w:rPr><w:strike/></w:rPr><w:t>Struck</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        assert!(doc.paragraphs[0].runs[0].strikethrough);
        let md = doc.to_markdown(true);
        assert!(md.contains("~~Struck~~"), "Markdown: {}", md);
    }

    #[test]
    fn test_double_strikethrough() {
        let xml = wrap_body(r#"<w:p><w:r><w:rPr><w:dstrike/></w:rPr><w:t>DStruck</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        assert!(doc.paragraphs[0].runs[0].strikethrough);
    }

    #[test]
    fn test_external_hyperlink() {
        let mut rels = AHashMap::new();
        rels.insert("rId1".to_string(), "https://example.com".to_string());

        let xml = wrap_body(r#"<w:p><w:hyperlink r:id="rId1"><w:r><w:t>Click here</w:t></w:r></w:hyperlink></w:p>"#);
        let doc = parse_xml_with_rels(&xml, rels);
        assert_eq!(doc.paragraphs.len(), 1);
        let run = &doc.paragraphs[0].runs[0];
        assert_eq!(run.text, "Click here");
        assert_eq!(run.hyperlink_url.as_deref(), Some("https://example.com"));

        let md = doc.to_markdown(true);
        assert!(md.contains("[Click here](https://example.com)"), "Markdown: {}", md);
    }

    #[test]
    fn test_hyperlink_with_no_relationship() {
        let xml = wrap_body(r#"<w:p><w:hyperlink r:id="rId99"><w:r><w:t>Broken link</w:t></w:r></w:hyperlink></w:p>"#);
        let doc = parse_xml(&xml);
        let run = &doc.paragraphs[0].runs[0];
        assert_eq!(run.text, "Broken link");
        assert!(run.hyperlink_url.is_none());
    }

    #[test]
    fn test_multiple_hyperlinks() {
        let mut rels = AHashMap::new();
        rels.insert("rId1".to_string(), "https://one.com".to_string());
        rels.insert("rId2".to_string(), "https://two.com".to_string());

        let xml = wrap_body(
            r#"<w:p>
                <w:hyperlink r:id="rId1"><w:r><w:t>First</w:t></w:r></w:hyperlink>
                <w:r><w:t> and </w:t></w:r>
                <w:hyperlink r:id="rId2"><w:r><w:t>Second</w:t></w:r></w:hyperlink>
            </w:p>"#,
        );
        let doc = parse_xml_with_rels(&xml, rels);
        let md = doc.to_markdown(true);
        assert!(md.contains("[First](https://one.com)"), "Markdown: {}", md);
        assert!(md.contains("[Second](https://two.com)"), "Markdown: {}", md);
    }

    #[test]
    fn test_basic_2x2_table() {
        let xml = wrap_body(
            r#"<w:tbl>
                <w:tr>
                    <w:tc><w:p><w:r><w:t>A1</w:t></w:r></w:p></w:tc>
                    <w:tc><w:p><w:r><w:t>B1</w:t></w:r></w:p></w:tc>
                </w:tr>
                <w:tr>
                    <w:tc><w:p><w:r><w:t>A2</w:t></w:r></w:p></w:tc>
                    <w:tc><w:p><w:r><w:t>B2</w:t></w:r></w:p></w:tc>
                </w:tr>
            </w:tbl>"#,
        );
        let doc = parse_xml(&xml);
        assert_eq!(doc.tables.len(), 1);
        let table = &doc.tables[0];
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].cells.len(), 2);

        let md = doc.to_markdown(true);
        assert!(md.contains("A1"), "Markdown: {}", md);
        assert!(md.contains("B2"), "Markdown: {}", md);

        let plain = doc.to_plain_text();
        assert!(plain.contains("A1"), "Plain: {}", plain);
        assert!(plain.contains("B2"), "Plain: {}", plain);
    }

    #[test]
    fn test_table_with_caption() {
        let xml = wrap_body(
            r#"<w:tbl>
                <w:tblPr>
                    <w:tblCaption w:val="My Table Caption"/>
                </w:tblPr>
                <w:tr>
                    <w:tc><w:p><w:r><w:t>Cell</w:t></w:r></w:p></w:tc>
                </w:tr>
                <w:tr>
                    <w:tc><w:p><w:r><w:t>Data</w:t></w:r></w:p></w:tc>
                </w:tr>
            </w:tbl>"#,
        );
        let doc = parse_xml(&xml);
        assert_eq!(doc.tables.len(), 1);
        let caption = doc.tables[0].properties.as_ref().and_then(|p| p.caption.as_deref());
        assert_eq!(caption, Some("My Table Caption"));

        let md = doc.to_markdown(true);
        assert!(md.contains("My Table Caption"), "Caption should be in markdown: {}", md);

        let plain = doc.to_plain_text();
        assert!(
            plain.contains("My Table Caption"),
            "Caption should be in plain text: {}",
            plain
        );
    }

    #[test]
    fn test_table_column_span() {
        let xml = wrap_body(
            r#"<w:tbl>
                <w:tr>
                    <w:tc>
                        <w:tcPr><w:gridSpan w:val="2"/></w:tcPr>
                        <w:p><w:r><w:t>Spanning</w:t></w:r></w:p>
                    </w:tc>
                </w:tr>
                <w:tr>
                    <w:tc><w:p><w:r><w:t>Left</w:t></w:r></w:p></w:tc>
                    <w:tc><w:p><w:r><w:t>Right</w:t></w:r></w:p></w:tc>
                </w:tr>
            </w:tbl>"#,
        );
        let doc = parse_xml(&xml);
        let table = &doc.tables[0];
        let first_cell = &table.rows[0].cells[0];
        assert_eq!(first_cell.properties.as_ref().and_then(|p| p.grid_span), Some(2));
    }

    #[test]
    fn test_table_vertical_merge() {
        let xml = wrap_body(
            r#"<w:tbl>
                <w:tr>
                    <w:tc>
                        <w:tcPr><w:vMerge w:val="restart"/></w:tcPr>
                        <w:p><w:r><w:t>Merged</w:t></w:r></w:p>
                    </w:tc>
                    <w:tc><w:p><w:r><w:t>Right1</w:t></w:r></w:p></w:tc>
                </w:tr>
                <w:tr>
                    <w:tc>
                        <w:tcPr><w:vMerge/></w:tcPr>
                        <w:p></w:p>
                    </w:tc>
                    <w:tc><w:p><w:r><w:t>Right2</w:t></w:r></w:p></w:tc>
                </w:tr>
            </w:tbl>"#,
        );
        let doc = parse_xml(&xml);
        let table = &doc.tables[0];
        let cell_0_0 = &table.rows[0].cells[0];
        assert_eq!(
            cell_0_0.properties.as_ref().and_then(|p| p.v_merge.as_ref()),
            Some(&super::super::table::VerticalMerge::Restart)
        );
        let cell_1_0 = &table.rows[1].cells[0];
        assert_eq!(
            cell_1_0.properties.as_ref().and_then(|p| p.v_merge.as_ref()),
            Some(&super::super::table::VerticalMerge::Continue)
        );
    }

    #[test]
    fn test_table_empty_cells() {
        let xml = wrap_body(
            r#"<w:tbl>
                <w:tr>
                    <w:tc><w:p><w:r><w:t>Has content</w:t></w:r></w:p></w:tc>
                    <w:tc><w:p></w:p></w:tc>
                </w:tr>
            </w:tbl>"#,
        );
        let doc = parse_xml(&xml);
        let table = &doc.tables[0];
        assert_eq!(table.rows[0].cells.len(), 2);
        let md = doc.to_markdown(true);
        assert!(md.contains("Has content"), "Markdown: {}", md);
    }

    /// A list authored with Word's built-in `List Bullet` style carries no
    /// `w:numPr` in `document.xml`: the numbering reference lives on the style.
    /// The paragraph still has to read as a list item (GH#1663).
    #[test]
    fn a_paragraph_inherits_the_numbering_its_style_carries() {
        let mut catalog = super::super::styles::StyleCatalog::default();
        catalog.styles.insert(
            "ListBullet".to_string(),
            super::super::styles::StyleDefinition {
                id: "ListBullet".to_string(),
                name: Some("List Bullet".to_string()),
                style_type: super::super::styles::StyleType::Paragraph,
                based_on: None,
                next_style: None,
                is_default: false,
                paragraph_properties: super::super::styles::ParagraphProperties {
                    numbering_id: Some(1),
                    ..Default::default()
                },
                run_properties: Default::default(),
            },
        );
        let mut doc = Document {
            paragraphs: vec![Paragraph {
                style: Some("ListBullet".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };

        apply_style_numbering(&catalog, &mut doc);

        assert_eq!(doc.paragraphs[0].numbering_id, Some(1));
        assert_eq!(
            doc.paragraphs[0].numbering_level,
            Some(0),
            "a style that sets w:numId without w:ilvl is level 0, and to_markdown needs both"
        );
    }

    /// Numbering written on the paragraph itself outranks the style's, so a list
    /// item that overrides its style keeps its own definition and level.
    #[test]
    fn paragraph_numbering_outranks_the_numbering_on_its_style() {
        let mut catalog = super::super::styles::StyleCatalog::default();
        catalog.styles.insert(
            "ListBullet".to_string(),
            super::super::styles::StyleDefinition {
                id: "ListBullet".to_string(),
                name: Some("List Bullet".to_string()),
                style_type: super::super::styles::StyleType::Paragraph,
                based_on: None,
                next_style: None,
                is_default: false,
                paragraph_properties: super::super::styles::ParagraphProperties {
                    numbering_id: Some(1),
                    ..Default::default()
                },
                run_properties: Default::default(),
            },
        );
        let mut doc = Document {
            paragraphs: vec![Paragraph {
                style: Some("ListBullet".to_string()),
                numbering_id: Some(7),
                numbering_level: Some(2),
                ..Default::default()
            }],
            ..Default::default()
        };

        apply_style_numbering(&catalog, &mut doc);

        assert_eq!(doc.paragraphs[0].numbering_id, Some(7));
        assert_eq!(doc.paragraphs[0].numbering_level, Some(2));
    }

    const LIST_BULLET_STYLES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
  <w:style w:type="paragraph" w:styleId="ListBullet">
    <w:name w:val="List Bullet"/>
    <w:basedOn w:val="Normal"/>
    <w:pPr><w:numPr><w:numId w:val="1"/></w:numPr></w:pPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="ListBulletChild">
    <w:name w:val="List Bullet Child"/>
    <w:basedOn w:val="ListBullet"/>
  </w:style>
</w:styles>"#;

    fn list_styled_paragraph(text: &str) -> String {
        format!(r#"<w:p><w:pPr><w:pStyle w:val="ListBullet"/></w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>"#)
    }

    fn docx_with_list_style(body: &str, extra_parts: &[(&str, &str)]) -> Vec<u8> {
        use std::io::Write;
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(wrap_body(body).as_bytes()).unwrap();
        zip.start_file("word/styles.xml", options).unwrap();
        zip.write_all(LIST_BULLET_STYLES_XML.as_bytes()).unwrap();
        for (path, xml) in extra_parts {
            zip.start_file(*path, options).unwrap();
            zip.write_all(xml.as_bytes()).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    fn parse_bytes(bytes: &[u8]) -> Document {
        let mut budget = SecurityBudget::with_defaults();
        parse_document(bytes, &mut budget, &default_limits()).expect("docx parses")
    }

    /// End-to-end proof that `apply_style_numbering` is wired into `DocxParser::parse`:
    /// a real archive through `parse_document`, not the extracted helper called directly
    /// against a hand-built `Document` (GH#1663).
    #[test]
    fn a_body_paragraph_inherits_list_numbering_from_its_style_through_parse_document() {
        let body = format!("{}{}", list_styled_paragraph("First"), list_styled_paragraph("Second"));
        let doc = parse_bytes(&docx_with_list_style(&body, &[]));
        assert_eq!(doc.paragraphs.len(), 2);
        assert_eq!(doc.paragraphs[0].numbering_id, Some(1));
        assert_eq!(doc.paragraphs[0].numbering_level, Some(0));
        let md = doc.to_markdown(true);
        assert_eq!(md, "- First\n- Second", "markdown: {md:?}");
    }

    /// The numbering reference is found through `basedOn`: `ListBulletChild` carries no
    /// `w:numPr` of its own and inherits it from `ListBullet` (GH#1663).
    #[test]
    fn a_paragraph_inherits_list_numbering_from_a_style_two_levels_up() {
        let body = r#"<w:p><w:pPr><w:pStyle w:val="ListBulletChild"/></w:pPr><w:r><w:t>Nested</w:t></w:r></w:p>"#;
        let doc = parse_bytes(&docx_with_list_style(body, &[]));
        assert_eq!(doc.paragraphs.len(), 1);
        assert_eq!(doc.paragraphs[0].numbering_id, Some(1));
        assert_eq!(doc.paragraphs[0].numbering_level, Some(0));
        assert_eq!(doc.to_markdown(true), "- Nested");
    }

    #[test]
    fn a_table_cell_paragraph_inherits_list_numbering_from_its_style() {
        let body = format!(
            "<w:tbl><w:tr><w:tc>{}</w:tc></w:tr></w:tbl>",
            list_styled_paragraph("In a cell")
        );
        let doc = parse_bytes(&docx_with_list_style(&body, &[]));
        let cell = &doc.tables[0].rows[0].cells[0];
        assert_eq!(cell.paragraphs[0].numbering_id, Some(1));
        assert_eq!(cell.paragraphs[0].numbering_level, Some(0));
    }

    #[test]
    fn a_header_paragraph_inherits_list_numbering_from_its_style() {
        let header = format!(
            r#"<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">{}</w:hdr>"#,
            list_styled_paragraph("In a header")
        );
        let doc = parse_bytes(&docx_with_list_style("", &[("word/header1.xml", &header)]));
        assert_eq!(doc.headers.len(), 1, "header part was picked up");
        assert_eq!(doc.headers[0].paragraphs[0].numbering_id, Some(1));
    }

    #[test]
    fn a_footnote_paragraph_inherits_list_numbering_from_its_style() {
        // ids -1/0/1 are reserved separators; 2 is the first real footnote id.
        let footnotes = format!(
            r#"<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:footnote w:id="2">{}</w:footnote></w:footnotes>"#,
            list_styled_paragraph("In a footnote")
        );
        let doc = parse_bytes(&docx_with_list_style("", &[("word/footnotes.xml", &footnotes)]));
        assert_eq!(doc.footnotes.len(), 1);
        assert_eq!(doc.footnotes[0].paragraphs[0].numbering_id, Some(1));
    }

    #[test]
    fn a_comment_paragraph_inherits_list_numbering_from_its_style() {
        let comments = format!(
            r#"<w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:comment w:id="0" w:author="A">{}</w:comment></w:comments>"#,
            list_styled_paragraph("In a comment")
        );
        let doc = parse_bytes(&docx_with_list_style("", &[("word/comments.xml", &comments)]));
        assert_eq!(doc.comments.len(), 1);
        assert_eq!(doc.comments[0].paragraphs[0].numbering_id, Some(1));
    }

    #[test]
    fn test_bullet_list_extraction() {
        let xml = wrap_body(
            r#"<w:p>
                <w:pPr>
                    <w:numId w:val="1"/>
                    <w:ilvl w:val="0"/>
                </w:pPr>
                <w:r><w:t>Bullet item</w:t></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        assert_eq!(doc.paragraphs.len(), 1);
        assert_eq!(doc.paragraphs[0].to_text(), "Bullet item");
        assert!(doc.paragraphs[0].numbering_id.is_some());
    }

    #[test]
    fn test_heading_style() {
        let xml = wrap_body(
            r#"<w:p>
                <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
                <w:r><w:t>My Heading</w:t></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        assert_eq!(doc.paragraphs[0].style.as_deref(), Some("Heading1"));
        let md = doc.to_markdown(true);
        assert!(md.contains("# My Heading"), "Markdown: {}", md);
    }

    #[test]
    fn test_heading2_style() {
        let xml = wrap_body(
            r#"<w:p>
                <w:pPr><w:pStyle w:val="Heading2"/></w:pPr>
                <w:r><w:t>Sub Heading</w:t></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        let md = doc.to_markdown(true);
        assert!(md.contains("## Sub Heading"), "Markdown: {}", md);
    }

    #[test]
    fn test_title_style() {
        let xml = wrap_body(
            r#"<w:p>
                <w:pPr><w:pStyle w:val="Title"/></w:pPr>
                <w:r><w:t>Document Title</w:t></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        let md = doc.to_markdown(true);
        assert!(md.contains("Document Title"), "Markdown: {}", md);
    }

    #[test]
    fn test_inline_drawing_with_alt_text() {
        let xml = wrap_body(
            r#"<w:p><w:r>
                <w:drawing>
                    <wp:inline xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">
                        <wp:extent cx="914400" cy="914400"/>
                        <wp:docPr id="1" name="Picture 1" descr="A logo image"/>
                        <a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
                            <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
                                <pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
                                    <pic:blipFill>
                                        <a:blip r:embed="rId5"/>
                                    </pic:blipFill>
                                </pic:pic>
                            </a:graphicData>
                        </a:graphic>
                    </wp:inline>
                </w:drawing>
            </w:r></w:p>"#,
        );
        let doc = parse_xml(&xml);
        assert_eq!(doc.drawings.len(), 1);
        let drawing = &doc.drawings[0];
        assert_eq!(
            drawing.doc_properties.as_ref().and_then(|dp| dp.description.as_deref()),
            Some("A logo image")
        );
        assert_eq!(drawing.image_ref.as_deref(), Some("rId5"));

        let md = doc.to_markdown(true);
        assert!(md.contains("![A logo image]"), "Markdown: {}", md);
    }

    #[test]
    fn test_drawing_dimensions() {
        let xml = wrap_body(
            r#"<w:p><w:r>
                <w:drawing>
                    <wp:inline xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">
                        <wp:extent cx="1828800" cy="914400"/>
                        <wp:docPr id="1" name="Pic"/>
                    </wp:inline>
                </w:drawing>
            </w:r></w:p>"#,
        );
        let doc = parse_xml(&xml);
        let extent = doc.drawings[0].extent.as_ref().unwrap();
        assert_eq!(extent.cx, 1828800);
        assert_eq!(extent.cy, 914400);
    }

    #[test]
    fn test_section_properties_parsed() {
        let xml = wrap_body(
            r#"<w:p><w:r><w:t>Content</w:t></w:r></w:p>
            <w:sectPr>
                <w:pgSz w:w="12240" w:h="15840"/>
                <w:pgMar w:top="1440" w:right="1800" w:bottom="1440" w:left="1800"/>
            </w:sectPr>"#,
        );
        let doc = parse_xml(&xml);
        assert!(!doc.sections.is_empty(), "Should have sections");
        let sect = &doc.sections[0];
        assert_eq!(sect.page_width_twips, Some(12240));
        assert_eq!(sect.page_height_twips, Some(15840));
        assert_eq!(sect.margins.top, Some(1440));
        assert_eq!(sect.margins.left, Some(1800));
    }

    #[test]
    fn test_footnote_reference_marker() {
        let xml = wrap_body(
            r#"<w:p>
                <w:r><w:t>Main text</w:t></w:r>
                <w:r><w:footnoteReference w:id="2"/></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        let text = doc.paragraphs[0].to_text();
        assert!(text.contains("[^2]"), "Should contain footnote marker: {}", text);
    }

    #[test]
    fn test_footnote_separator_ids_filtered() {
        let xml = wrap_body(
            r#"<w:p>
                <w:r><w:footnoteReference w:id="0"/></w:r>
                <w:r><w:footnoteReference w:id="1"/></w:r>
                <w:r><w:t>text</w:t></w:r>
                <w:r><w:footnoteReference w:id="2"/></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        let text = doc.paragraphs[0].to_text();
        assert!(!text.contains("[^0]"), "Separator id 0 should be filtered");
        assert!(text.contains("[^1]"), "Word's first footnote is id 1: {}", text);
        assert!(text.contains("[^2]"), "Real footnote 2 should be present");
    }

    #[test]
    fn test_field_instruction_skipped_result_kept() {
        let xml = wrap_body(
            r#"<w:p>
                <w:r><w:t>Before </w:t></w:r>
                <w:r><w:fldChar w:fldCharType="begin"/></w:r>
                <w:r><w:instrText> SEQ Figure \* ARABIC </w:instrText></w:r>
                <w:r><w:fldChar w:fldCharType="separate"/></w:r>
                <w:r><w:t>2</w:t></w:r>
                <w:r><w:fldChar w:fldCharType="end"/></w:r>
                <w:r><w:t> After</w:t></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        let text = doc.paragraphs[0].to_text();
        assert!(text.contains("Before"), "Text: {}", text);
        assert!(text.contains("After"), "Text: {}", text);
        assert!(text.contains("2"), "Field result '2' should be kept: {}", text);
        assert!(!text.contains("SEQ"), "Field instruction should be skipped: {}", text);
    }

    #[test]
    fn test_page_field_result_kept() {
        let xml = wrap_body(
            r#"<w:p>
                <w:r><w:t>Page </w:t></w:r>
                <w:r><w:fldChar w:fldCharType="begin"/></w:r>
                <w:r><w:instrText> PAGE </w:instrText></w:r>
                <w:r><w:fldChar w:fldCharType="separate"/></w:r>
                <w:r><w:t>1</w:t></w:r>
                <w:r><w:fldChar w:fldCharType="end"/></w:r>
                <w:r><w:t> of 5</w:t></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        let text = doc.paragraphs[0].to_text();
        assert_eq!(text.trim(), "Page 1 of 5", "Text: '{}'", text);
    }

    #[test]
    fn test_text_after_field_resumes() {
        let xml = wrap_body(
            r#"<w:p>
                <w:r><w:fldChar w:fldCharType="begin"/></w:r>
                <w:r><w:instrText> NUMPAGES </w:instrText></w:r>
                <w:r><w:fldChar w:fldCharType="separate"/></w:r>
                <w:r><w:t>10</w:t></w:r>
                <w:r><w:fldChar w:fldCharType="end"/></w:r>
                <w:r><w:t>Normal text</w:t></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        let text = doc.paragraphs[0].to_text();
        assert!(text.contains("Normal text"), "Text: {}", text);
        assert!(text.contains("10"), "Field result should be kept: {}", text);
    }

    #[test]
    fn test_math_text_extracted() {
        let xml = wrap_body(
            r#"<w:p>
                <m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
                    <m:r>
                        <m:t>E=mc</m:t>
                    </m:r>
                    <m:sSup>
                        <m:e><m:r><m:t></m:t></m:r></m:e>
                        <m:sup><m:r><m:t>2</m:t></m:r></m:sup>
                    </m:sSup>
                </m:oMath>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        let text = doc.paragraphs[0].to_text();
        assert!(text.contains("E=mc"), "Math text should contain E=mc: {}", text);
        assert!(text.contains("^{2}"), "Math text should contain ^{{2}}: {}", text);
        let md = doc.paragraphs[0].runs_to_markdown();
        assert!(md.starts_with('$'), "Inline math should start with $: {}", md);
        assert!(md.ends_with('$'), "Inline math should end with $: {}", md);
    }

    #[test]
    fn test_element_ordering_preserved() {
        let xml = wrap_body(
            r#"<w:p><w:r><w:t>Para 1</w:t></w:r></w:p>
            <w:tbl>
                <w:tr><w:tc><w:p><w:r><w:t>Cell</w:t></w:r></w:p></w:tc></w:tr>
                <w:tr><w:tc><w:p><w:r><w:t>Data</w:t></w:r></w:p></w:tc></w:tr>
            </w:tbl>
            <w:p><w:r><w:t>Para 2</w:t></w:r></w:p>"#,
        );
        let doc = parse_xml(&xml);
        assert_eq!(doc.elements.len(), 3);
        assert!(matches!(doc.elements[0], DocumentElement::Paragraph(0)));
        assert!(matches!(doc.elements[1], DocumentElement::Table(0)));
        assert!(matches!(doc.elements[2], DocumentElement::Paragraph(1)));

        let md = doc.to_markdown(true);
        let para1_pos = md.find("Para 1").unwrap();
        let cell_pos = md.find("Cell").unwrap();
        let para2_pos = md.find("Para 2").unwrap();
        assert!(para1_pos < cell_pos, "Para 1 before table");
        assert!(cell_pos < para2_pos, "Table before Para 2");
    }

    #[test]
    fn test_empty_document() {
        let xml = wrap_body("");
        let doc = parse_xml(&xml);
        assert!(doc.paragraphs.is_empty());
        assert!(doc.tables.is_empty());
        let md = doc.to_markdown(true);
        assert!(md.trim().is_empty(), "Empty doc markdown: '{}'", md);
    }

    #[test]
    fn test_paragraph_with_only_whitespace() {
        let xml = wrap_body(r#"<w:p><w:r><w:t>   </w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        assert_eq!(doc.paragraphs[0].to_text(), "   ");
    }

    #[test]
    fn test_extract_lorem_ipsum_docx() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/docx/lorem_ipsum.docx");
        if let Ok(bytes) = std::fs::read(&path) {
            let text = super::super::extract_text(&bytes).unwrap();
            assert!(!text.is_empty(), "Should extract text from lorem ipsum");
            assert!(
                text.contains("Lorem"),
                "Should contain 'Lorem': {}",
                &text[..100.min(text.len())]
            );
        }
    }

    #[test]
    fn test_extract_word_tables_docx() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/docx/word_tables.docx");
        if let Ok(bytes) = std::fs::read(&path) {
            let text = super::super::extract_text(&bytes).unwrap();
            assert!(!text.is_empty(), "Should extract text from word tables");
        }
    }

    #[test]
    fn test_extract_unit_test_lists_docx() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/docx/unit_test_lists.docx");
        if let Ok(bytes) = std::fs::read(&path) {
            let text = super::super::extract_text(&bytes).unwrap();
            assert!(!text.is_empty(), "Should extract text from list document");
        }
    }

    #[test]
    fn test_extract_python_docx_test_file() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/vendored/python-docx/test.docx");
        if let Ok(bytes) = std::fs::read(&path) {
            let text = super::super::extract_text(&bytes).unwrap();
            assert!(!text.is_empty(), "Should extract text from python-docx test.docx");
        }
    }

    #[test]
    fn test_extract_python_docx_having_images() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/vendored/python-docx/having-images.docx");
        if let Ok(bytes) = std::fs::read(&path) {
            let text = super::super::extract_text(&bytes).unwrap();
            let _ = text;
        }
    }

    #[test]
    fn test_extract_word_sample_no_field_leaks() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/docx/word_sample.docx");
        if let Ok(bytes) = std::fs::read(&path) {
            let text = super::super::extract_text(&bytes).unwrap();
            assert!(!text.is_empty());
        }
    }

    /// Regression: adjacent bold runs in textbox.docx must not produce `****`.
    #[test]
    fn test_textbox_no_spurious_bold_markers() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/docx/textbox.docx");
        if let Ok(bytes) = std::fs::read(&path) {
            let mut budget = SecurityBudget::with_defaults();
            let doc = super::parse_document(&bytes, &mut budget, &default_limits()).unwrap();
            let md = doc.to_markdown(true);
            assert!(
                !md.contains("****"),
                "Markdown output should not contain spurious '****' sequences. Got:\n{}",
                md
            );
        }
    }

    #[test]
    fn test_extract_unit_test_formatting_no_headers() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/docx/unit_test_formatting.docx");
        if let Ok(bytes) = std::fs::read(&path) {
            let text = super::super::extract_text(&bytes).unwrap();
            assert!(!text.is_empty());
        }
    }

    #[test]
    fn test_to_markdown_inject_placeholders_true() {
        use crate::extraction::docx::drawing::{DocProperties, Drawing, DrawingType};

        let mut doc = Document::new();

        let mut para = Paragraph::new();
        para.add_run(Run::new("Hello world".to_string()));
        let p_idx = doc.paragraphs.len();
        doc.paragraphs.push(para);
        doc.elements.push(DocumentElement::Paragraph(p_idx));

        let drawing = Drawing {
            drawing_type: DrawingType::Inline,
            extent: None,
            doc_properties: Some(DocProperties {
                id: Some("1".to_string()),
                name: Some("Pic".to_string()),
                description: Some("alt text".to_string()),
            }),
            image_ref: Some("rId1".to_string()),
            text_box_content: None,
        };
        let d_idx = doc.drawings.len();
        doc.drawings.push(drawing);
        doc.elements.push(DocumentElement::Drawing(d_idx));

        let md = doc.to_markdown(true);
        assert!(
            md.contains("![alt text](image)"),
            "Expected image placeholder, got: {md}"
        );
        assert!(md.contains("Hello world"));
    }

    #[test]
    fn test_to_markdown_inject_placeholders_false() {
        use crate::extraction::docx::drawing::{DocProperties, Drawing, DrawingType};

        let mut doc = Document::new();

        let mut para = Paragraph::new();
        para.add_run(Run::new("Hello world".to_string()));
        let p_idx = doc.paragraphs.len();
        doc.paragraphs.push(para);
        doc.elements.push(DocumentElement::Paragraph(p_idx));

        let drawing = Drawing {
            drawing_type: DrawingType::Inline,
            extent: None,
            doc_properties: Some(DocProperties {
                id: Some("1".to_string()),
                name: Some("Pic".to_string()),
                description: Some("alt text".to_string()),
            }),
            image_ref: Some("rId1".to_string()),
            text_box_content: None,
        };
        let d_idx = doc.drawings.len();
        doc.drawings.push(drawing);
        doc.elements.push(DocumentElement::Drawing(d_idx));

        let md = doc.to_markdown(false);
        assert!(!md.contains("!["), "Should NOT contain image placeholder, got: {md}");
        assert!(md.contains("Hello world"), "Text content must be preserved");
    }

    /// `w:ilvl` is a bare `.parse::<i64>()` of an attacker-controlled attribute, and the
    /// value reaches `"  ".repeat(level as usize)`. `-1 as usize` is `usize::MAX`, so the
    /// repeat's internal `len().checked_mul(n)` panics with "capacity overflow"; a large
    /// positive value asks for an allocation big enough to abort the process outright.
    /// These pin the clamp directly, because the end-to-end DOCX tests cannot: a list
    /// starting at a non-zero level renders flat today, so every level above 0 produces
    /// identical markdown there regardless of the cap.
    #[test]
    fn clamp_numbering_level_floors_a_negative_level_at_zero() {
        assert_eq!(clamp_numbering_level(-1), 0);
        assert_eq!(clamp_numbering_level(i64::MIN), 0);
    }

    #[test]
    fn clamp_numbering_level_caps_an_oversized_level_at_the_nesting_ceiling() {
        assert_eq!(clamp_numbering_level(10_000_000_000), MAX_LIST_NESTING_LEVEL);
        assert_eq!(clamp_numbering_level(i64::MAX), MAX_LIST_NESTING_LEVEL);
    }

    #[test]
    fn clamp_numbering_level_leaves_every_level_word_itself_permits_untouched() {
        for level in 0..=MAX_LIST_NESTING_LEVEL {
            assert_eq!(
                clamp_numbering_level(level),
                level,
                "level {level} is within Word's own range"
            );
        }
    }
}
