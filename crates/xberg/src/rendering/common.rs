//! Shared rendering infrastructure for `InternalDocument`-based renderers.
//!
//! Provides nesting state tracking, annotated text rendering, footnote collection,
//! table formatting helpers, and HTML escaping.

use std::borrow::Cow;

use crate::types::document_structure::{AnnotationKind, ContentLayer, TextAnnotation};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement, RelationshipKind, RelationshipTarget};

/// Kind of container on the nesting stack.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NestingKind {
    List { ordered: bool, item_count: u32 },
    BlockQuote,
    Group,
}

/// Tracks nesting depth during a linear pass over elements.
#[derive(Debug, Default)]
pub(crate) struct RenderState {
    /// Stack of `(depth, kind)` entries.
    stack: Vec<(u16, NestingKind)>,
}

impl RenderState {
    /// Push a container onto the nesting stack.
    pub(crate) fn push_container(&mut self, kind: NestingKind, depth: u16) {
        self.stack.push((depth, kind));
    }

    /// Pop the top container if it matches the given kind category.
    pub(crate) fn pop_container(&mut self, kind: &NestingKind) {
        for i in (0..self.stack.len()).rev() {
            if matches!(
                (&self.stack[i].1, kind),
                (NestingKind::List { .. }, NestingKind::List { .. })
                    | (NestingKind::BlockQuote, NestingKind::BlockQuote)
                    | (NestingKind::Group, NestingKind::Group)
            ) {
                self.stack.remove(i);
                return;
            }
        }
    }

    /// Pop entries whose depth >= the given depth (fallback for missing end markers).
    pub(crate) fn pop_to_depth(&mut self, depth: u16) {
        while let Some(&(d, _)) = self.stack.last() {
            if d >= depth {
                self.stack.pop();
            } else {
                break;
            }
        }
    }

    /// Current list nesting depth.
    pub(crate) fn list_depth(&self) -> usize {
        self.stack
            .iter()
            .filter(|(_, k)| matches!(k, NestingKind::List { .. }))
            .count()
    }

    /// Current blockquote nesting depth.
    pub(crate) fn blockquote_depth(&self) -> usize {
        self.stack
            .iter()
            .filter(|(_, k)| matches!(k, NestingKind::BlockQuote))
            .count()
    }

    /// Increment and return the next list item number for the innermost list.
    pub(crate) fn next_list_number(&mut self) -> u32 {
        for (_, kind) in self.stack.iter_mut().rev() {
            if let NestingKind::List {
                ordered: true,
                item_count,
            } = kind
            {
                *item_count += 1;
                return *item_count;
            }
            if let NestingKind::List { ordered: false, .. } = kind {
                break;
            }
        }
        1
    }
}

/// Render text with byte-range annotations, calling `emit` for each annotated span.
///
/// Annotations are sorted by `(start, end)`. Overlapping annotations (where
/// `start < current_pos`) are skipped, matching the existing renderer behavior.
///
/// Plain (unannotated) text segments are passed through without transformation.
#[cfg(test)]
pub(crate) fn render_annotated_text(
    text: &str,
    annotations: &[TextAnnotation],
    emit: impl Fn(&str, &AnnotationKind) -> String,
) -> String {
    render_annotated_text_with_plain(text, annotations, emit, |s| s.to_string())
}

pub(crate) fn render_annotated_text_with_plain(
    text: &str,
    annotations: &[TextAnnotation],
    emit: impl Fn(&str, &AnnotationKind) -> String,
    plain: impl Fn(&str) -> String,
) -> String {
    if annotations.is_empty() {
        return plain(text);
    }

    let mut sorted: Vec<&TextAnnotation> = annotations.iter().collect();
    sorted.sort_by_key(|a| (a.start, a.end));

    let bytes = text.as_bytes();
    let len = bytes.len() as u32;
    let mut pos: u32 = 0;
    let mut out = String::with_capacity(text.len() + 64);

    for ann in &sorted {
        // `TextAnnotation::start`/`end` are byte offsets that may originate from any
        // extractor, not just the ones in this crate that are provably char-boundary-safe
        // (e.g. `pdf::structure::assembly::extract_text_and_annotations`, which derives
        // them from `text.len()` on the exact same buffer). Nothing here guarantees that
        // in general, so — matching `comrak_bridge::build_comrak_ast`'s identical guard on
        // the same annotation type — clamp to the nearest char boundary before slicing
        // `text`, rather than trusting the offset outright and panicking on a mid-codepoint
        // cut.
        let start = text.ceil_char_boundary(ann.start.min(len) as usize) as u32;
        let end = text.floor_char_boundary(ann.end.min(len) as usize) as u32;
        if start < pos || start >= end {
            continue;
        }
        if start > pos {
            out.push_str(&plain(&text[pos as usize..start as usize]));
        }
        let span = &text[start as usize..end as usize];
        out.push_str(&emit(span, &ann.kind));
        pos = end;
    }

    if (pos as usize) < bytes.len() {
        out.push_str(&plain(&text[pos as usize..]));
    }

    out
}

/// Collected footnote data: definition text and assigned number.
#[derive(Debug)]
pub(crate) struct FootnoteEntry {
    pub(crate) text: String,
    pub(crate) number: u32,
}

/// Pre-scans elements and relationships to build a sequential footnote numbering.
#[derive(Debug)]
pub(crate) struct FootnoteCollector {
    /// Map from element index (FootnoteRef) -> assigned number.
    ref_numbers: ahash::AHashMap<u32, u32>,
    /// Ordered definitions.
    definitions: Vec<FootnoteEntry>,
}

/// Maps each `FootnoteDefinition` element's anchor to its `(element index, text)`.
/// Split out of [`FootnoteCollector::new`] purely to shorten that constructor.
fn build_footnote_def_by_anchor(doc: &InternalDocument) -> ahash::AHashMap<String, (u32, String)> {
    let mut def_by_anchor: ahash::AHashMap<String, (u32, String)> = ahash::AHashMap::new();
    for (i, elem) in doc.elements.iter().enumerate() {
        if elem.kind == ElementKind::FootnoteDefinition
            && let Some(ref anchor) = elem.anchor
        {
            def_by_anchor.insert(anchor.clone(), (i as u32, elem.text.clone()));
        }
    }
    def_by_anchor
}

/// Maps each `FootnoteRef` element index to the anchor of the definition it points at,
/// first from `doc.relationships`, then falling back to a `FootnoteRef` element's own
/// anchor or text for refs no relationship covers. Split out of
/// [`FootnoteCollector::new`] purely to shorten that constructor.
fn build_footnote_ref_to_def_anchor(doc: &InternalDocument) -> ahash::AHashMap<u32, String> {
    let mut ref_to_def_anchor: ahash::AHashMap<u32, String> = ahash::AHashMap::new();
    for rel in &doc.relationships {
        if rel.kind == RelationshipKind::FootnoteReference {
            match &rel.target {
                RelationshipTarget::Key(key) => {
                    ref_to_def_anchor.insert(rel.source, key.clone());
                }
                RelationshipTarget::Index(idx) => {
                    if let Some(elem) = doc.elements.get(*idx as usize)
                        && let Some(ref anchor) = elem.anchor
                    {
                        ref_to_def_anchor.insert(rel.source, anchor.clone());
                    }
                }
            }
        }
    }

    for (i, elem) in doc.elements.iter().enumerate() {
        if elem.kind == ElementKind::FootnoteRef {
            let idx = i as u32;
            if !ref_to_def_anchor.contains_key(&idx) {
                if let Some(ref anchor) = elem.anchor {
                    ref_to_def_anchor.insert(idx, anchor.clone());
                } else if !elem.text.is_empty() {
                    ref_to_def_anchor.insert(idx, elem.text.clone());
                }
            }
        }
    }
    ref_to_def_anchor
}

impl FootnoteCollector {
    /// Scan the document and build footnote mappings.
    pub(crate) fn new(doc: &InternalDocument) -> Self {
        let def_by_anchor = build_footnote_def_by_anchor(doc);
        let ref_to_def_anchor = build_footnote_ref_to_def_anchor(doc);

        let mut ref_numbers: ahash::AHashMap<u32, u32> = ahash::AHashMap::new();
        let mut anchor_to_number: ahash::AHashMap<String, u32> = ahash::AHashMap::new();
        let mut next_number: u32 = 1;
        let mut definitions = Vec::new();

        for (i, elem) in doc.elements.iter().enumerate() {
            if elem.kind == ElementKind::FootnoteRef {
                let idx = i as u32;
                if let Some(anchor) = ref_to_def_anchor.get(&idx) {
                    let number = *anchor_to_number.entry(anchor.clone()).or_insert_with(|| {
                        let n = next_number;
                        next_number += 1;
                        let text = def_by_anchor.get(anchor).map(|(_, t)| t.clone()).unwrap_or_default();
                        definitions.push(FootnoteEntry { text, number: n });
                        n
                    });
                    ref_numbers.insert(idx, number);
                }
            }
        }

        // A definition that no reference points at is still authored content.
        // `definitions` was previously populated only from inside the FootnoteRef
        // loop above, so an unreferenced definition never reached any renderer and
        // was silently lost. Append the orphans after the referenced ones, in
        // document order, continuing the same numbering. See #68.
        for elem in &doc.elements {
            if elem.kind != ElementKind::FootnoteDefinition {
                continue;
            }
            let Some(anchor) = elem.anchor.as_ref() else {
                continue;
            };
            if anchor_to_number.contains_key(anchor) {
                continue;
            }
            let number = next_number;
            next_number += 1;
            anchor_to_number.insert(anchor.clone(), number);
            definitions.push(FootnoteEntry {
                text: elem.text.clone(),
                number,
            });
        }

        Self {
            ref_numbers,
            definitions,
        }
    }

    /// Get the footnote number for a FootnoteRef element at the given index.
    pub(crate) fn ref_number(&self, elem_index: u32) -> Option<u32> {
        self.ref_numbers.get(&elem_index).copied()
    }

    /// Get ordered footnote definitions.
    pub(crate) fn definitions(&self) -> &[FootnoteEntry] {
        &self.definitions
    }
}

/// Render a table (from `Table.cells`) as a GFM pipe table.
///
/// This is the crate's single table-to-markdown renderer: every extractor and
/// every output renderer routes through it, so the same table serialises
/// identically whether it came from a PDF, a DOCX, an HTML page or OCR
/// (xberg-io/xberg#220).
///
/// The grid is normalised to the width of its *widest* row, so a row carrying
/// more cells than the header keeps every one of them instead of having the
/// overflow silently dropped (xberg-io/xberg#221); short rows are padded so the
/// pipe columns stay aligned with the header.
///
/// Cell content is escaped so no cell can break out of the row it lives in:
/// `|` becomes `\|` and any line break becomes `<br>` (xberg-io/xberg#163).
pub(crate) fn render_table_markdown(cells: &[Vec<String>]) -> String {
    let mut out = String::new();
    render_table_markdown_into(&mut out, cells);
    out
}

/// Render a GFM pipe table into an existing buffer.
///
/// Identical contract to [`render_table_markdown`]; callers that can pre-size
/// the buffer from a capacity estimate use this to skip the intermediate
/// allocation.
pub(crate) fn render_table_markdown_into(out: &mut String, cells: &[Vec<String>]) {
    if cells.is_empty() {
        return;
    }
    let num_cols = cells.iter().map(|r| r.len()).max().unwrap_or(0);
    if num_cols == 0 {
        return;
    }

    if let Some(header) = cells.first() {
        push_table_row(out, header, num_cols);

        out.push('|');
        for _ in 0..num_cols {
            out.push_str(" --- |");
        }
        out.push('\n');
    }

    for row in cells.iter().skip(1) {
        push_table_row(out, row, num_cols);
    }
}

/// Push one pipe-delimited row, padded out to `num_cols` columns.
fn push_table_row(out: &mut String, row: &[String], num_cols: usize) {
    out.push('|');
    for col in 0..num_cols {
        out.push(' ');
        let content = row.get(col).map(String::as_str).unwrap_or("");
        push_escaped_cell(out, content);
        out.push_str(" |");
    }
    out.push('\n');
}

/// Stand-in for a line break inside a table cell. A raw newline ends the table
/// row, splitting one cell's content across two rows (xberg-io/xberg#163).
const CELL_LINE_BREAK: &str = "<br>";

/// Push `content` into `out`, escaping every character that would let a cell
/// break out of its row. Avoids allocation when there is nothing to escape (the
/// common case for table cell content).
fn push_escaped_cell(out: &mut String, content: &str) {
    if memchr::memchr3(b'|', b'\n', b'\r', content.as_bytes()).is_none() {
        out.push_str(content);
        return;
    }
    let mut chars = content.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '|' => out.push_str("\\|"),
            '\r' => {
                // Consume the LF of a CRLF pair so it yields one break, not two.
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push_str(CELL_LINE_BREAK);
            }
            '\n' => out.push_str(CELL_LINE_BREAK),
            _ => out.push(ch),
        }
    }
}

/// Render a table as plain space-separated text.
pub(crate) fn render_table_plain(cells: &[Vec<String>]) -> String {
    if cells.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    for row in cells {
        out.push_str(&row.join(" "));
        out.push('\n');
    }
    out
}

/// Render a table as djot pipe table (same syntax as GFM).
pub(crate) fn render_table_djot(cells: &[Vec<String>]) -> String {
    render_table_markdown(cells)
}

/// Normalize inline text for consistent output across renderers.
///
/// - Collapses multiple consecutive whitespace (spaces, tabs) into a single space
/// - Replaces newlines with spaces (mid-paragraph line breaks from PDF extraction)
/// - Strips control characters (< 0x20) except tab
pub(crate) fn normalize_inline_text(text: &str) -> Cow<'_, str> {
    let needs_normalization = text.as_bytes().windows(2).any(|w| w[0] == b' ' && w[1] == b' ')
        || text.bytes().any(|b| b < 0x20 && b != b'\t');
    if !needs_normalization {
        return Cow::Borrowed(text);
    }

    let mut result = String::with_capacity(text.len());
    let mut prev_space = false;
    for ch in text.chars() {
        if ch == '\n' || ch == ' ' {
            if !prev_space {
                result.push(' ');
            }
            prev_space = true;
        } else if ch < '\u{20}' && ch != '\t' {
        } else {
            prev_space = false;
            result.push(ch);
        }
    }
    Cow::Owned(result)
}

/// Ensure the output has a trailing newline (but not doubled).
pub(crate) fn ensure_trailing_newline(out: &mut String) {
    if !out.ends_with('\n') {
        out.push('\n');
    }
}

/// Trim trailing whitespace, then ensure exactly one trailing newline.
pub(crate) fn finalize_output(mut out: String) -> String {
    let trimmed_len = out.trim_end().len();
    if trimmed_len == 0 {
        return String::new();
    }
    out.truncate(trimmed_len);
    out.push('\n');
    out
}

/// Prefix every line of `text` with the blockquote prefix (`> ` repeated N times).
pub(crate) fn apply_blockquote_prefix(text: &str, depth: usize) -> Cow<'_, str> {
    if depth == 0 {
        return Cow::Borrowed(text);
    }
    let prefix = "> ".repeat(depth);
    let mut out = String::with_capacity(text.len() + prefix.len() * text.lines().count());
    for line in text.lines() {
        out.push_str(&prefix);
        out.push_str(line);
        out.push('\n');
    }
    Cow::Owned(out)
}

/// Push a block of text, optionally applying blockquote prefixes.
pub(crate) fn push_with_bq(out: &mut String, text: &str, bq_depth: usize) {
    if bq_depth > 0 {
        out.push_str(&apply_blockquote_prefix(text, bq_depth));
    } else {
        out.push_str(text);
    }
}

/// Handle container end elements (ListEnd/QuoteEnd/GroupEnd) by popping the
/// corresponding entry from the nesting state. Returns `true` if a container
/// was handled.
pub(crate) fn handle_container_end(kind: &ElementKind, state: &mut RenderState) -> bool {
    match kind {
        ElementKind::ListEnd => {
            state.pop_container(&NestingKind::List {
                ordered: false,
                item_count: 0,
            });
            true
        }
        ElementKind::QuoteEnd => {
            state.pop_container(&NestingKind::BlockQuote);
            true
        }
        ElementKind::GroupEnd => {
            state.pop_container(&NestingKind::Group);
            true
        }
        _ => false,
    }
}

/// Check if an element should be rendered in the body pass.
pub(crate) fn is_body_element(elem: &InternalElement) -> bool {
    elem.layer == ContentLayer::Body
}

/// The OCR text of the picture that the image element `elem` shows, where there is text to add.
///
/// `None` for a picture with no OCR text, and for a picture whose text whole-page OCR
/// replaced. Plain output and the plain content of a page both read it here, so the two
/// cannot disagree about a picture. ~keep
pub(crate) fn image_ocr_text<'a>(
    doc: &'a InternalDocument,
    elem: &InternalElement,
    image_index: u32,
) -> Option<&'a str> {
    if !elem.should_render_image_ocr() {
        return None;
    }
    doc.images
        .get(image_index as usize)?
        .ocr_result
        .as_ref()
        .map(|result| result.content.as_str())
        .filter(|content| !content.is_empty())
}

/// Check if an element is a container end marker.
pub(crate) fn is_container_end(elem: &InternalElement) -> bool {
    elem.kind.is_container_end()
}

/// Get the language attribute from an element's attributes map.
pub(crate) fn get_language(elem: &InternalElement) -> Option<&str> {
    elem.attributes
        .as_ref()
        .and_then(|attrs| attrs.get("language").map(|s| s.as_str()))
}

/// Get the admonition kind from attributes.
pub(crate) fn get_admonition_kind(elem: &InternalElement) -> &str {
    elem.attributes
        .as_ref()
        .and_then(|attrs| attrs.get("kind").map(|s| s.as_str()))
        .unwrap_or("note")
}

/// Get the admonition title from attributes.
pub(crate) fn get_admonition_title(elem: &InternalElement) -> Option<&str> {
    elem.attributes
        .as_ref()
        .and_then(|attrs| attrs.get("title").map(|s| s.as_str()))
}

/// Get metadata entries from the text (stored as `key: value` lines).
pub(crate) fn parse_metadata_entries(text: &str) -> Vec<(&str, &str)> {
    text.lines()
        .filter_map(|line| {
            let idx = line.find(':')?;
            let key = line[..idx].trim();
            let value = line[idx + 1..].trim();
            if key.is_empty() { None } else { Some((key, value)) }
        })
        .collect()
}

/// Human-readable label for a [`PdfAnnotationType`], shared by every
/// renderer's annotation appendix (issue #63).
pub(crate) fn annotation_type_label(kind: crate::types::annotations::PdfAnnotationType) -> &'static str {
    use crate::types::annotations::PdfAnnotationType;
    match kind {
        PdfAnnotationType::Text => "Text",
        PdfAnnotationType::Highlight => "Highlight",
        PdfAnnotationType::Link => "Link",
        PdfAnnotationType::Stamp => "Stamp",
        PdfAnnotationType::Underline => "Underline",
        PdfAnnotationType::StrikeOut => "StrikeOut",
        PdfAnnotationType::Squiggly => "Squiggly",
        PdfAnnotationType::Ink => "Ink",
        PdfAnnotationType::Square => "Square",
        PdfAnnotationType::Circle => "Circle",
        PdfAnnotationType::Polygon => "Polygon",
        PdfAnnotationType::PolyLine => "PolyLine",
        PdfAnnotationType::Line => "Line",
        PdfAnnotationType::Caret => "Caret",
        PdfAnnotationType::FileAttachment => "FileAttachment",
        PdfAnnotationType::Sound => "Sound",
        PdfAnnotationType::Movie => "Movie",
        PdfAnnotationType::Other => "Other",
    }
}

/// The best available text for a rendered annotation: the QuadPoints-derived
/// marked-up text (Highlight/Underline/StrikeOut/Squiggly) takes priority
/// over the free-form comment/URL in `content`, since the marked text is what
/// the annotation is actually about.
pub(crate) fn annotation_display_text(annotation: &crate::types::annotations::PdfAnnotation) -> Option<&str> {
    annotation
        .marked_text
        .as_deref()
        .or(annotation.content.as_deref())
        .filter(|s| !s.is_empty())
}

/// Escape a string for safe inclusion in HTML text content (not attributes).
///
/// Only the three characters that matter for text nodes are escaped, matching
/// the minimal escaping `comrak`'s own HTML formatter performs for body text.
pub(crate) fn escape_html_text(input: &str) -> Cow<'_, str> {
    if !input.contains(['&', '<', '>']) {
        return Cow::Borrowed(input);
    }
    let mut out = String::with_capacity(input.len() + 16);
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    Cow::Owned(out)
}

// ~keep Read only by the DOCX and PDF writers, which need `office` and `pdf`; a
// hand-kept union-of-consumers `cfg` is what drifted and failed the 1.3.0 publish (GH#1951).
/// The target of a link a reader can open outside the document, in the URL parser's
/// percent-encoded form. Fragment and relative links have no such target, so a binary
/// format keeps their text and drops the link.
#[allow(dead_code)]
pub(crate) fn external_link_target(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    matches!(parsed.scheme(), "http" | "https" | "mailto" | "ftp").then(|| parsed.into())
}

#[cfg(test)]
mod tests;
