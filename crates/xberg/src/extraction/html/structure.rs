//! HTML to `DocumentStructure` builder.
//!
//! Walks raw HTML and produces a hierarchical `DocumentStructure` using the
//! `DocumentStructureBuilder`. This is intentionally a lightweight, non-allocating
//! tag-level parser that handles the common structural elements without pulling
//! in a full DOM library.

use ahash::AHashMap;

use crate::types::builder::{self, DocumentStructureBuilder};
use crate::types::document_structure::{DocumentStructure, NodeIndex, TextAnnotation};

/// `<tag>` open/close dispatch, split out to keep this file under the line-count limit.
mod tag_handlers;

/// Build a `DocumentStructure` from raw HTML.
pub(crate) fn build_document_structure(html: &str) -> DocumentStructure {
    let mut builder = DocumentStructureBuilder::new().source_format("html");
    let mut walker = HtmlWalker::new(html, &mut builder);
    walker.walk();
    builder.build()
}

/// Tracks the kind of inline formatting active at a given byte offset.
#[derive(Debug, Clone)]
struct InlineSpan {
    kind: InlineKind,
    /// Byte offset in the accumulated text buffer where this span starts.
    text_start: u32,
}

#[derive(Debug, Clone)]
enum InlineKind {
    Bold,
    Italic,
    Code,
    Underline,
    Strikethrough,
    Link { href: String, title: Option<String> },
    Subscript,
    Superscript,
    Highlight,
}

/// Represents a `<pre><code>` block being accumulated.
#[derive(Debug)]
struct PreBlock {
    language: Option<String>,
    text: String,
}

/// Metadata about a single cell being accumulated.
#[derive(Debug)]
struct CellMeta {
    text: String,
    col_span: u32,
    row_span: u32,
    is_header: bool,
}

/// Represents a `<table>` being accumulated.
#[derive(Debug)]
struct TableAccumulator {
    rows: Vec<Vec<CellMeta>>,
    current_row: Vec<CellMeta>,
    current_cell: String,
    current_col_span: u32,
    current_row_span: u32,
    current_is_header: bool,
    in_row: bool,
    in_cell: bool,
}

impl TableAccumulator {
    fn new() -> Self {
        Self {
            rows: Vec::new(),
            current_row: Vec::new(),
            current_cell: String::new(),
            current_col_span: 1,
            current_row_span: 1,
            current_is_header: false,
            in_row: false,
            in_cell: false,
        }
    }

    fn open_row(&mut self) {
        self.current_row = Vec::new();
        self.in_row = true;
    }

    fn close_row(&mut self) {
        if self.in_row {
            self.rows.push(std::mem::take(&mut self.current_row));
            self.in_row = false;
        }
    }

    fn open_cell(&mut self, col_span: u32, row_span: u32, is_header: bool) {
        self.current_cell = String::new();
        self.current_col_span = col_span;
        self.current_row_span = row_span;
        self.current_is_header = is_header;
        self.in_cell = true;
    }

    fn close_cell(&mut self) {
        if self.in_cell {
            self.current_row.push(CellMeta {
                text: std::mem::take(&mut self.current_cell),
                col_span: self.current_col_span,
                row_span: self.current_row_span,
                is_header: self.current_is_header,
            });
            self.in_cell = false;
            self.current_col_span = 1;
            self.current_row_span = 1;
            self.current_is_header = false;
        }
    }

    fn push_text(&mut self, text: &str) {
        if self.in_cell {
            self.current_cell.push_str(text);
        }
    }
}

/// List context pushed onto the list stack.
#[derive(Debug)]
struct ListContext {
    node_idx: NodeIndex,
    /// Whether an `<li>` at this nesting level is currently open.
    ///
    /// Distinct from `HtmlWalker::in_list_item`, which only says whether text is
    /// being buffered right now: descending into a sublist flushes and clears that
    /// flag while the enclosing item is still open. This one survives the descent,
    /// so closing the sublist can resume buffering into the enclosing item.
    item_open: bool,
    /// The `ListItem` node most recently emitted at this level, if the currently open
    /// `<li>` has already produced one.
    ///
    /// Reset when an `<li>` opens, so it never names a previous sibling's item. A sublist
    /// opening while `item_open` is set is parented under this node (task #728).
    last_item_idx: Option<NodeIndex>,
}

/// Definition list context.
#[derive(Debug)]
struct DefListContext {
    list_idx: NodeIndex,
    current_term: Option<String>,
}

/// Figure context accumulating image + caption.
#[derive(Debug)]
struct FigureContext {
    img_alt: Option<String>,
    img_src: Option<String>,
    img_width: Option<String>,
    img_height: Option<String>,
    caption: Option<String>,
    in_caption: bool,
}

/// Main walker state.
struct HtmlWalker<'a, 'b> {
    src: &'a str,
    pos: usize,
    builder: &'b mut DocumentStructureBuilder,

    text_buf: String,
    inline_stack: Vec<InlineSpan>,
    annotations: Vec<TextAnnotation>,

    in_pre: bool,
    pre_block: Option<PreBlock>,
    table: Option<TableAccumulator>,
    /// Number of `<table>` elements open inside the accumulated table. A nested
    /// table is flattened into the enclosing cell instead of replacing the
    /// enclosing table.
    nested_table_depth: usize,
    list_stack: Vec<ListContext>,
    in_list_item: bool,
    list_item_text: String,
    def_list: Option<DefListContext>,
    in_dt: bool,
    in_dd: bool,
    dt_text: String,
    dd_text: String,
    figure: Option<FigureContext>,
    in_head: bool,
    meta_entries: Vec<(String, String)>,
    /// Closing tags that do not occur after the walker's position. The walker only moves
    /// forward, so they never will, and they are not searched for again.
    missing_close_tags: Vec<String>,

    pending_classes: Option<String>,
}

impl<'a, 'b> HtmlWalker<'a, 'b> {
    fn new(src: &'a str, builder: &'b mut DocumentStructureBuilder) -> Self {
        Self {
            src,
            pos: 0,
            builder,
            text_buf: String::new(),
            inline_stack: Vec::new(),
            annotations: Vec::new(),
            in_pre: false,
            pre_block: None,
            table: None,
            nested_table_depth: 0,
            list_stack: Vec::new(),
            in_list_item: false,
            list_item_text: String::new(),
            def_list: None,
            in_dt: false,
            in_dd: false,
            dt_text: String::new(),
            dd_text: String::new(),
            figure: None,
            in_head: false,
            meta_entries: Vec::new(),
            missing_close_tags: Vec::new(),
            pending_classes: None,
        }
    }

    fn walk(&mut self) {
        while self.pos < self.src.len() {
            if self.src[self.pos..].starts_with("<!--") {
                if let Some(end) = self.src[self.pos..].find("-->") {
                    self.pos += end + 3;
                } else {
                    self.pos = self.src.len();
                }
                continue;
            }

            if self.src.as_bytes()[self.pos] == b'<' {
                self.handle_tag();
            } else {
                self.handle_text();
            }
        }
        self.flush_paragraph();
    }

    fn handle_text(&mut self) {
        let start = self.pos;
        while self.pos < self.src.len() && self.src.as_bytes()[self.pos] != b'<' {
            self.pos += 1;
        }
        let raw = &self.src[start..self.pos];
        let decoded = decode_entities(raw);

        if let Some(ref mut table) = self.table {
            table.push_text(&decoded);
            return;
        }

        if let Some(ref mut pre) = self.pre_block {
            pre.text.push_str(&decoded);
            return;
        }

        if self.in_list_item {
            self.list_item_text.push_str(&decoded);
            return;
        }

        if self.in_dt {
            self.dt_text.push_str(&decoded);
            return;
        }

        if self.in_dd {
            self.dd_text.push_str(&decoded);
            return;
        }

        if let Some(ref mut fig) = self.figure
            && fig.in_caption
        {
            let cap = fig.caption.get_or_insert_with(String::new);
            cap.push_str(&decoded);
            return;
        }

        self.text_buf.push_str(&decoded);
    }

    fn handle_tag(&mut self) {
        let tag_start = self.pos;
        let Some(end) = self.src[self.pos..].find('>') else {
            self.pos = self.src.len();
            return;
        };
        let tag_content = &self.src[self.pos + 1..self.pos + end];
        self.pos += end + 1;

        if tag_content.starts_with('!') || tag_content.starts_with('?') {
            return;
        }

        let is_closing = tag_content.starts_with('/');
        let content = if is_closing { &tag_content[1..] } else { tag_content };

        let content = content.trim_end_matches('/').trim();

        let (tag_name, attrs_str) = split_tag_name(content);
        let tag_lower = tag_name.to_ascii_lowercase();

        if is_closing {
            self.handle_close_tag(&tag_lower, tag_start);
        } else {
            let is_self_closing = tag_content.ends_with('/');
            self.handle_open_tag(&tag_lower, attrs_str, is_self_closing);
        }
    }

    /// Build a `TableGrid` from accumulated rows with colspan/rowspan support.
    fn emit_table_with_spans(&mut self, rows: &[Vec<CellMeta>]) {
        use crate::types::document_structure::{GridCell, TableGrid};

        let num_rows = rows.len() as u32;

        let has_spans = rows.iter().any(|r| r.iter().any(|c| c.col_span > 1 || c.row_span > 1));

        if !has_spans {
            let simple: Vec<Vec<String>> = rows
                .iter()
                .map(|r| r.iter().map(|c| c.text.clone()).collect())
                .collect();
            self.builder.push_table_from_cells(&simple, None);
            return;
        }

        let mut grid_cells = Vec::new();
        let cols = crate::extraction::grid_flatten::resolve_span_grid(
            rows,
            |c| c.col_span,
            |c| c.row_span,
            |row_idx, col, cell| {
                grid_cells.push(GridCell {
                    content: cell.text.clone(),
                    row: row_idx,
                    col,
                    row_span: cell.row_span,
                    col_span: cell.col_span,
                    is_header: cell.is_header,
                    bbox: None,
                    heading_level: None,
                    style_name: None,
                });
            },
        );

        let grid = TableGrid {
            rows: num_rows,
            cols,
            cells: grid_cells,
        };
        self.builder.push_table(grid, None, None);
    }

    fn push_inline(&mut self, kind: InlineKind) {
        let offset = if self.in_list_item {
            self.list_item_text.len() as u32
        } else {
            self.text_buf.len() as u32
        };
        self.inline_stack.push(InlineSpan {
            kind,
            text_start: offset,
        });
    }

    fn pop_inline(&mut self, expected: InlineKind) {
        let idx = self
            .inline_stack
            .iter()
            .rposition(|s| std::mem::discriminant(&s.kind) == std::mem::discriminant(&expected));
        if let Some(i) = idx {
            let span = self.inline_stack.remove(i);
            let end = if self.in_list_item {
                self.list_item_text.len() as u32
            } else {
                self.text_buf.len() as u32
            };
            if end > span.text_start {
                let annotation = match span.kind {
                    InlineKind::Bold => builder::bold(span.text_start, end),
                    InlineKind::Italic => builder::italic(span.text_start, end),
                    InlineKind::Code => builder::code(span.text_start, end),
                    InlineKind::Underline => builder::underline(span.text_start, end),
                    InlineKind::Strikethrough => builder::strikethrough(span.text_start, end),
                    InlineKind::Subscript => TextAnnotation {
                        start: span.text_start,
                        end,
                        kind: crate::types::document_structure::AnnotationKind::Subscript,
                    },
                    InlineKind::Superscript => TextAnnotation {
                        start: span.text_start,
                        end,
                        kind: crate::types::document_structure::AnnotationKind::Superscript,
                    },
                    InlineKind::Highlight => TextAnnotation {
                        start: span.text_start,
                        end,
                        kind: crate::types::document_structure::AnnotationKind::Highlight,
                    },
                    InlineKind::Link { .. } => unreachable!("Links handled separately by pop_inline_link"),
                };
                self.annotations.push(annotation);
            }
        }
    }

    fn pop_inline_link(&mut self) {
        let idx = self
            .inline_stack
            .iter()
            .rposition(|s| matches!(s.kind, InlineKind::Link { .. }));
        if let Some(i) = idx {
            let span = self.inline_stack.remove(i);
            let end = if self.in_list_item {
                self.list_item_text.len() as u32
            } else {
                self.text_buf.len() as u32
            };
            if end > span.text_start
                && let InlineKind::Link { href, title } = span.kind
            {
                let annotation = builder::link(span.text_start, end, &href, title.as_deref());
                self.annotations.push(annotation);
            }
        }
    }

    fn flush_paragraph(&mut self) {
        let text = normalize_whitespace(&self.text_buf);
        if !text.is_empty() {
            let annotations = std::mem::take(&mut self.annotations);
            let idx = self.builder.push_paragraph(&text, annotations, None, None);
            if let Some(classes) = self.pending_classes.take() {
                let mut attrs = AHashMap::new();
                attrs.insert("class".to_string(), classes);
                self.builder.set_attributes(idx, attrs);
            }
        }
        self.text_buf.clear();
        self.annotations.clear();
        self.inline_stack.clear();
    }

    /// Create the `List` node for a `<ul>`/`<ol>` start tag, parented at the level the
    /// markup actually nests it at.
    ///
    /// A sublist is a child of the `<li>` it is written inside, so that a consumer walking
    /// the tree renders it before the item's trailing text rather than after the whole
    /// outer list. Going through `push_list` instead parents under the section/container
    /// stack, which makes every sublist a root-level sibling (task #728).
    ///
    /// Two shapes have no item node to hang the sublist on: `<li><ul>…` (the item has no
    /// text of its own, so no `ListItem` was emitted) and a `<ul>` sitting directly inside
    /// another `<ul>` with no `<li>` open. Both fall back to the enclosing `List` node,
    /// which keeps the sublist inside the list subtree without minting an empty item.
    fn push_list_node(&mut self, ordered: bool) -> NodeIndex {
        let parent = self.list_stack.last().map(|ctx| {
            if ctx.item_open {
                ctx.last_item_idx.unwrap_or(ctx.node_idx)
            } else {
                ctx.node_idx
            }
        });
        match parent {
            Some(parent_idx) => self.builder.push_nested_list(parent_idx, ordered, None),
            None => self.builder.push_list(ordered, None),
        }
    }

    /// Emit the buffered `<li>` text as a `ListItem` and reset the inline state that
    /// belonged to it.
    ///
    /// The annotation buffer is taken (not just read) and the inline stack is cleared, for
    /// the same reason `flush_paragraph` does both: `pop_inline` measures spans against
    /// `list_item_text`, which this method empties. Anything still referring to it after
    /// the flush — a completed annotation left behind, or a span whose closing tag has not
    /// arrived yet — would resolve against whatever text is buffered next and annotate an
    /// unrelated node at meaningless offsets (task #727).
    fn flush_list_item(&mut self) {
        if !self.in_list_item {
            return;
        }
        self.in_list_item = false;
        let text = normalize_whitespace(&self.list_item_text);
        let annotations = std::mem::take(&mut self.annotations);
        if !text.is_empty()
            && let Some(list_idx) = self.list_stack.last().map(|ctx| ctx.node_idx)
        {
            let item_idx = self.builder.push_list_item(list_idx, &text, annotations, None);
            if let Some(ctx) = self.list_stack.last_mut() {
                ctx.last_item_idx = Some(item_idx);
            }
        }
        self.list_item_text.clear();
        self.inline_stack.clear();
    }

    fn flush_definition_item(&mut self) {
        if self.in_dd {
            self.in_dd = false;
            if let Some(ref mut dl) = self.def_list {
                let definition = normalize_whitespace(&self.dd_text);
                if let Some(term) = dl.current_term.take() {
                    self.builder.push_definition_item(dl.list_idx, &term, &definition, None);
                }
            }
            self.dd_text.clear();
        }
        if self.in_dt {
            self.in_dt = false;
            if let Some(ref mut dl) = self.def_list {
                let term = normalize_whitespace(&self.dt_text);
                if !term.is_empty() {
                    dl.current_term = Some(term);
                }
            }
            self.dt_text.clear();
        }
    }
}

/// Split a tag body into (name, rest-of-attributes).
/// Convert a raw `<math>...</math>` XHTML subtree to LaTeX via the shared
/// MathML converter (`crate::extraction::mathml`, gated by the `office`
/// feature). Returns `None` if the fragment fails to parse, converts to empty
/// output, or the security budget is exhausted on hostile input.
#[cfg(feature = "office")]
fn convert_math_subtree_to_latex(raw_xml: &str) -> Option<String> {
    let mut budget = crate::extractors::security::SecurityBudget::from_limits(
        &crate::extractors::security::SecurityLimits::default(),
    );
    crate::extraction::mathml::convert_mathml_str_to_latex(raw_xml, &mut budget)
        .ok()
        .filter(|latex| !latex.trim().is_empty())
}

/// The `mathml` converter lives behind the `office` feature; without it, `math`
/// elements are dropped rather than mangled into stray inline text.
#[cfg(not(feature = "office"))]
fn convert_math_subtree_to_latex(_raw_xml: &str) -> Option<String> {
    None
}

fn split_tag_name(content: &str) -> (&str, &str) {
    let content = content.trim();
    if let Some(space_pos) = content.find(|c: char| c.is_ascii_whitespace()) {
        (&content[..space_pos], &content[space_pos + 1..])
    } else {
        (content, "")
    }
}

/// Extract an attribute value from a raw attributes string.
///
/// Handles both `attr="value"` and `attr='value'` forms.
fn extract_attr<'a>(attrs: &'a str, name: &str) -> Option<&'a str> {
    let search = format!("{name}=");
    let mut search_from = 0;
    let idx = loop {
        let candidate = attrs[search_from..].find(&search)?;
        let abs = search_from + candidate;
        if abs == 0 || !attrs.as_bytes()[abs - 1].is_ascii_alphanumeric() {
            break abs;
        }
        search_from = abs + 1;
    };
    let after_eq = &attrs[idx + search.len()..];
    let after_eq = after_eq.trim_start();
    if after_eq.is_empty() {
        return None;
    }
    let quote = after_eq.as_bytes()[0];
    if quote == b'"' || quote == b'\'' {
        let rest = &after_eq[1..];
        let end = rest.find(quote as char)?;
        Some(&rest[..end])
    } else {
        let end = after_eq
            .find(|c: char| c.is_ascii_whitespace() || c == '>')
            .unwrap_or(after_eq.len());
        Some(&after_eq[..end])
    }
}

/// Extract a language identifier from a class attribute like `language-rust` or `lang-python`.
fn extract_language_from_class(class: &str) -> Option<&str> {
    for cls in class.split_ascii_whitespace() {
        if let Some(lang) = cls.strip_prefix("language-") {
            return Some(lang);
        }
        if let Some(lang) = cls.strip_prefix("lang-") {
            return Some(lang);
        }
    }
    None
}

/// Decode basic HTML entities.
fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '&' {
            out.push(c);
            continue;
        }
        if let Some(entity) = read_entity_name(&mut chars, &mut out) {
            push_decoded_entity(&entity, &mut out);
        }
    }
    out
}

/// Read one entity name/reference from just after an `&`, consuming characters from `chars` up
/// to and including the terminating `;`. If the name grows past 10 characters with no `;` found
/// (not a real entity), the literal `&<partial>` read so far is written straight to `out` and
/// `None` is returned, matching [`decode_entities`]'s original inline escape hatch.
fn read_entity_name(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, out: &mut String) -> Option<String> {
    let mut entity = String::new();
    for ec in chars.by_ref() {
        if ec == ';' {
            break;
        }
        entity.push(ec);
        if entity.len() > 10 {
            out.push('&');
            out.push_str(&entity);
            entity.clear();
            break;
        }
    }
    if entity.is_empty() { None } else { Some(entity) }
}

/// Resolve a decoded entity name to its character/replacement and append it to `out`: a known
/// named entity, a numeric character reference (`#NN`/`#xNN`), or (as a fallback) the entity
/// re-encoded literally.
fn push_decoded_entity(entity: &str, out: &mut String) {
    if let Some(ch) = named_entity_char(entity) {
        out.push(ch);
        return;
    }
    push_numeric_entity_or_literal(entity, out);
}

/// The small set of named HTML entities this decoder recognizes.
fn named_entity_char(name: &str) -> Option<char> {
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => ' ',
        "copy" => '\u{00A9}',
        "reg" => '\u{00AE}',
        "trade" => '\u{2122}',
        "mdash" => '\u{2014}',
        "ndash" => '\u{2013}',
        "laquo" => '\u{00AB}',
        "raquo" => '\u{00BB}',
        "hellip" => '\u{2026}',
        "eacute" => '\u{00E9}',
        "egrave" => '\u{00E8}',
        "ecirc" => '\u{00EA}',
        "euml" => '\u{00EB}',
        "aacute" => '\u{00E1}',
        "agrave" => '\u{00E0}',
        "acirc" => '\u{00E2}',
        "auml" => '\u{00E4}',
        "iacute" => '\u{00ED}',
        "ocirc" => '\u{00F4}',
        "ouml" => '\u{00F6}',
        "uuml" => '\u{00FC}',
        "ntilde" => '\u{00F1}',
        "ccedil" => '\u{00E7}',
        "ldquo" => '\u{201C}',
        "rdquo" => '\u{201D}',
        "lsquo" => '\u{2018}',
        "rsquo" => '\u{2019}',
        "bull" => '\u{2022}',
        "middot" => '\u{00B7}',
        "euro" => '\u{20AC}',
        "pound" => '\u{00A3}',
        "yen" => '\u{00A5}',
        "times" => '\u{00D7}',
        "divide" => '\u{00F7}',
        "plusmn" => '\u{00B1}',
        _ => return None,
    })
}

/// A numeric character reference (`#NN` decimal or `#xNN`/`#XNN` hex) decodes to its code point;
/// anything else (including a numeric reference to an invalid code point) is written back out
/// literally as `&<entity>;`, since it wasn't actually decodable.
fn push_numeric_entity_or_literal(entity: &str, out: &mut String) {
    if let Some(num_str) = entity.strip_prefix('#') {
        let code_point = if num_str.starts_with('x') || num_str.starts_with('X') {
            u32::from_str_radix(&num_str[1..], 16).ok()
        } else {
            num_str.parse::<u32>().ok()
        };
        if let Some(cp) = code_point
            && let Some(ch) = char::from_u32(cp)
        {
            out.push(ch);
            return;
        }
    }
    out.push('&');
    out.push_str(entity);
    out.push(';');
}

/// Collapse runs of whitespace into single spaces and trim.
///
/// The sentinel character `\x01` marks intentional line breaks inserted by
/// `<br>` tag handling. These are converted to real newlines in the output
/// while all other whitespace (including source HTML newlines) is collapsed.
fn normalize_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_was_space = true;

    for c in s.chars() {
        if c == '\x01' {
            while out.ends_with(' ') {
                out.pop();
            }
            out.push('\n');
            last_was_space = true;
        } else if c.is_ascii_whitespace() {
            if !last_was_space {
                out.push(' ');
                last_was_space = true;
            }
        } else {
            out.push(c);
            last_was_space = false;
        }
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

#[cfg(test)]
#[path = "structure/tests.rs"]
mod tests;
