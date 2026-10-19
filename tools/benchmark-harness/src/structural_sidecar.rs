//! Typed structural sidecar and the canonical SF1 structural metric.
//!
//! A [`StructuralSidecar`] is a deterministic, typed description of a document's
//! *structure* — headings with hierarchy, nested list items, tables with a full
//! cell grid, figure/caption/footnote binding edges, formulas, images, and a
//! global reading order. It is persisted per document as `<id>.structural.json`
//! (see `scripts/build_structural_sidecar.py`) and derived here from GFM
//! markdown using the **same** pulldown-cmark options as
//! [`crate::markdown_quality::parse_markdown_blocks`]
//! ([`crate::markdown_quality::md_parser_options`]).
//!
//! # Spans
//!
//! GFM pipe tables cannot express row/column spans, so a sidecar derived from
//! markdown has every cell `rowspan == colspan == 1` and
//! `spans_recoverable == false`. HTML/ParseBench-sourced tables (built by the
//! Python builder from the source HTML) can carry real spans with
//! `spans_recoverable == true`.
//!
//! # The metric: [`score_structural`]
//!
//! Seven dimensions are scored, then rolled up with structural weights (heading
//! 2.0 / table topology 1.5 / table content 1.5 / list 1.0 / paragraph 0.5, plus
//! binding-edges 0.5), normalized over the dimensions actually present in either
//! document, and finally folded with the LIS reading-order score via
//! [`crate::markdown_quality::fold_order_into_sf1`]:
//!
//! - **D0** paragraph content-F1
//! - **D1** heading hierarchy (level agreement + ancestry Jaccard)
//! - **D2** list nesting (depth + ordered agreement)
//! - **D3** table topology, a GriTS-like grid F1 (a fabricated table scores 0)
//! - **D4** caption / footnote binding-edge F1
//! - **D5** reading order via longest-increasing-subsequence
//! - **D6** table cell-content F1 (GriTS-Con, position-independent), folded into
//!   the rollup at weight 1.5 (equal to D3 topology), gated on table presence
//!
//! A fabricated table — a predicted table where the GT has none — scores 0 on
//! D3, which then pulls the whole SF1 down (it can no longer hide as matched
//! prose after the D3 § metric fix in [`crate::markdown_quality`]).

use std::collections::{HashMap, HashSet};

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};

use crate::markdown_quality::{compute_order_score, fold_order_into_sf1, inline_math_tex};
use crate::quality::{compute_f1, tokenize};

// `structural_sidecar.rs` is itself attached to `crate::quality` via `#[path]` (see
// `quality.rs`), which means its own submodules do not inherit an implicit
// `structural_sidecar/`-named directory -- this attribute is required, not stylistic. ~keep
#[path = "structural_sidecar/scoring.rs"]
mod scoring;

pub(crate) use scoring::diagnostic_matches;
pub use scoring::{StructuralScore, score_markdown, score_structural};

/// A single cell in a table's grid.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Cell {
    pub row: usize,
    pub col: usize,
    pub rowspan: usize,
    pub colspan: usize,
    pub is_header: bool,
    pub text: String,
}

/// A table node: a full cell grid plus span-recoverability metadata.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TableNode {
    pub n_rows: usize,
    pub n_cols: usize,
    pub header_rows: usize,
    pub cells: Vec<Cell>,
    /// `false` for GFM pipe tables (spans cannot be expressed); `true` when the
    /// grid was recovered from source HTML and may carry real spans.
    pub spans_recoverable: bool,
}

/// A typed structural node. Serialized with an internal `"kind"` tag.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StructuralNode {
    Heading {
        level: u8,
        /// Node index of the immediately enclosing heading, if any.
        parent: Option<usize>,
        /// Ancestor heading texts, outermost first.
        path: Vec<String>,
        text: String,
    },
    ListItem {
        /// 0-based nesting depth.
        depth: usize,
        ordered: bool,
        /// Node index of the enclosing list item, if any.
        parent_item: Option<usize>,
        text: String,
    },
    Table(TableNode),
    Figure {
        /// Node index of the bound caption, if any.
        caption: Option<usize>,
        text: String,
    },
    Caption {
        /// Node index this caption binds to (figure/table/image), if any.
        binds_to: Option<usize>,
        text: String,
    },
    Footnote {
        /// Node index this footnote annotates, if any.
        binds_to: Option<usize>,
        text: String,
    },
    Formula {
        display: bool,
        text: String,
    },
    Image {
        alt: String,
    },
    Paragraph {
        text: String,
    },
}

impl StructuralNode {
    /// A representative text for content-similarity matching.
    pub(crate) fn repr_text(&self) -> String {
        match self {
            StructuralNode::Heading { text, .. }
            | StructuralNode::ListItem { text, .. }
            | StructuralNode::Figure { text, .. }
            | StructuralNode::Caption { text, .. }
            | StructuralNode::Footnote { text, .. }
            | StructuralNode::Formula { text, .. }
            | StructuralNode::Paragraph { text } => text.clone(),
            StructuralNode::Image { alt } => alt.clone(),
            StructuralNode::Table(t) => t.cells.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join(" "),
        }
    }

    pub(crate) fn kind_name(&self) -> &'static str {
        match self {
            StructuralNode::Heading { .. } => "heading",
            StructuralNode::ListItem { .. } => "list_item",
            StructuralNode::Table(_) => "table",
            StructuralNode::Figure { .. } => "figure",
            StructuralNode::Caption { .. } => "caption",
            StructuralNode::Footnote { .. } => "footnote",
            StructuralNode::Formula { .. } => "formula",
            StructuralNode::Image { .. } => "image",
            StructuralNode::Paragraph { .. } => "paragraph",
        }
    }
}

/// A typed structural description of one document.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct StructuralSidecar {
    pub nodes: Vec<StructuralNode>,
    /// Node indices in reading order. Derived from GFM in document order, but
    /// kept explicit so an HTML-sourced builder can supply a corrected order.
    pub reading_order: Vec<usize>,
}

/// In-progress table cell grid for [`MarkdownParseState`]. Not scored directly -- folded into a
/// [`TableNode`] on `Event::End(TagEnd::Table)`.
struct TableBuild {
    cells: Vec<Cell>,
    row: usize,
    col: usize,
    n_cols: usize,
    header_rows: usize,
    in_header: bool,
}

/// Mutable state threaded through the single-pass GFM event stream in
/// [`StructuralSidecar::from_markdown`] -- one method per event category, rather than one very
/// long `match`, to keep each piece of parsing logic short and separately readable. Carries no
/// behavior beyond what `from_markdown` had inline before this split.
struct MarkdownParseState {
    nodes: Vec<StructuralNode>,
    // Heading hierarchy stack: (level, node_index, text). ~keep
    heading_stack: Vec<(u8, usize, String)>,
    list_ordered: Vec<bool>,
    item_text: Vec<String>,
    item_ordered: Vec<bool>,
    table: Option<TableBuild>,
    current_text: String,
    in_heading: Option<u8>,
    in_code_block: bool,
    in_math_fence: bool,
}

impl MarkdownParseState {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            heading_stack: Vec::new(),
            list_ordered: Vec::new(),
            item_text: Vec::new(),
            item_ordered: Vec::new(),
            table: None,
            current_text: String::new(),
            in_heading: None,
            in_code_block: false,
            in_math_fence: false,
        }
    }

    fn flush_paragraph(&mut self) {
        let content = std::mem::take(&mut self.current_text);
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            self.nodes.push(StructuralNode::Paragraph {
                text: trimmed.to_string(),
            });
        }
    }

    fn start_heading(&mut self, level: u8) {
        self.flush_paragraph();
        self.in_heading = Some(level);
    }

    fn end_heading(&mut self) {
        let Some(level) = self.in_heading.take() else {
            return;
        };
        let text = std::mem::take(&mut self.current_text).trim().to_string();
        if text.is_empty() {
            return;
        }
        while self.heading_stack.last().is_some_and(|(l, _, _)| *l >= level) {
            self.heading_stack.pop();
        }
        let parent = self.heading_stack.last().map(|(_, idx, _)| *idx);
        let path: Vec<String> = self.heading_stack.iter().map(|(_, _, t)| t.clone()).collect();
        let idx = self.nodes.len();
        self.nodes.push(StructuralNode::Heading {
            level,
            parent,
            path,
            text: text.clone(),
        });
        self.heading_stack.push((level, idx, text));
    }

    fn start_code_block(&mut self, kind: &CodeBlockKind) {
        self.flush_paragraph();
        self.in_code_block = true;
        self.in_math_fence =
            matches!(kind, CodeBlockKind::Fenced(info) if info.split_ascii_whitespace().next() == Some("math"));
    }

    fn end_code_block(&mut self) {
        self.in_code_block = false;
        let content = std::mem::take(&mut self.current_text);
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return;
        }
        if self.in_math_fence || is_formula(trimmed) {
            self.nodes.push(StructuralNode::Formula {
                display: true,
                text: trimmed.to_string(),
            });
        } else {
            // Code blocks are content — fold into the paragraph pool. ~keep
            self.nodes.push(StructuralNode::Paragraph {
                text: trimmed.to_string(),
            });
        }
    }

    fn start_table(&mut self) {
        self.flush_paragraph();
        self.table = Some(TableBuild {
            cells: Vec::new(),
            row: 0,
            col: 0,
            n_cols: 0,
            header_rows: 0,
            in_header: false,
        });
    }

    fn end_table(&mut self) {
        let Some(tb) = self.table.take() else {
            return;
        };
        let n_rows = if tb.cells.is_empty() {
            0
        } else {
            tb.cells.iter().map(|c| c.row).max().unwrap_or(0) + 1
        };
        self.nodes.push(StructuralNode::Table(TableNode {
            n_rows,
            n_cols: tb.n_cols,
            header_rows: tb.header_rows,
            cells: tb.cells,
            spans_recoverable: false,
        }));
    }

    fn start_table_head(&mut self) {
        if let Some(tb) = self.table.as_mut() {
            tb.in_header = true;
        }
    }

    fn end_table_head(&mut self) {
        if let Some(tb) = self.table.as_mut() {
            tb.in_header = false;
            tb.n_cols = tb.n_cols.max(tb.col);
            tb.row += 1;
            tb.col = 0;
        }
    }

    fn start_table_row(&mut self) {
        if let Some(tb) = self.table.as_mut() {
            tb.col = 0;
        }
    }

    fn end_table_row(&mut self) {
        if let Some(tb) = self.table.as_mut() {
            tb.n_cols = tb.n_cols.max(tb.col);
            tb.row += 1;
        }
    }

    fn end_table_cell(&mut self) {
        let Some(tb) = self.table.as_mut() else {
            return;
        };
        let text = std::mem::take(&mut self.current_text).trim().to_string();
        if tb.in_header {
            tb.header_rows = tb.header_rows.max(tb.row + 1);
        }
        tb.cells.push(Cell {
            row: tb.row,
            col: tb.col,
            rowspan: 1,
            colspan: 1,
            is_header: tb.in_header,
            text,
        });
        tb.col += 1;
    }

    fn start_list(&mut self, ordered: bool) {
        if self.list_ordered.is_empty() {
            self.flush_paragraph();
        }
        self.list_ordered.push(ordered);
    }

    fn end_list(&mut self) {
        self.list_ordered.pop();
    }

    fn start_item(&mut self) {
        self.item_text.push(String::new());
        self.item_ordered
            .push(self.list_ordered.last().copied().unwrap_or(false));
    }

    fn end_item(&mut self) {
        let text = self.item_text.pop().unwrap_or_default().trim().to_string();
        let ordered = self.item_ordered.pop().unwrap_or(false);
        let depth = self.item_text.len(); // remaining open items = ancestor count ~keep
        if !text.is_empty() {
            self.nodes.push(StructuralNode::ListItem {
                depth,
                ordered,
                parent_item: None, // filled in post-pass (not scored) ~keep
                text,
            });
        }
    }

    fn start_image(&mut self) {
        self.flush_paragraph();
        self.current_text.push_str("\u{0}IMG\u{0}"); // sentinel so image alt is captured separately ~keep
    }

    fn end_image(&mut self) {
        if let Some(rest) = self.current_text.strip_prefix("\u{0}IMG\u{0}") {
            let alt = rest.trim().to_string();
            self.current_text.clear();
            self.nodes.push(StructuralNode::Image { alt });
        }
    }

    fn push_text(&mut self, text: &str) {
        if let Some(buf) = self.item_text.last_mut() {
            push_spaced(buf, text);
        } else if self.current_text.starts_with("\u{0}IMG\u{0}") {
            self.current_text.push_str(text);
        } else {
            push_spaced(&mut self.current_text, text);
        }
    }

    fn soft_break(&mut self) {
        if self.in_code_block {
            self.current_text.push('\n');
        } else if let Some(buf) = self.item_text.last_mut() {
            buf.push(' ');
        } else {
            self.current_text.push(' ');
        }
    }

    fn hard_break(&mut self) {
        if let Some(buf) = self.item_text.last_mut() {
            buf.push(' ');
        } else {
            self.current_text.push('\n');
        }
    }

    fn push_inline_math(&mut self, text: &str) {
        if let Some(buf) = self.item_text.last_mut() {
            push_spaced(buf, text);
        } else {
            push_spaced(&mut self.current_text, text);
        }
    }

    fn display_math(&mut self, text: &str) {
        self.flush_paragraph();
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            self.nodes.push(StructuralNode::Formula {
                display: true,
                text: trimmed.to_string(),
            });
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => self.start_heading(level as u8),
            Event::End(TagEnd::Heading(_)) => self.end_heading(),
            Event::Start(Tag::CodeBlock(kind)) => self.start_code_block(&kind),
            Event::End(TagEnd::CodeBlock) => self.end_code_block(),
            Event::Start(Tag::Table(_)) => self.start_table(),
            Event::End(TagEnd::Table) => self.end_table(),
            Event::Start(Tag::TableHead) => self.start_table_head(),
            Event::End(TagEnd::TableHead) => self.end_table_head(),
            Event::Start(Tag::TableRow) => self.start_table_row(),
            Event::End(TagEnd::TableRow) => self.end_table_row(),
            Event::End(TagEnd::TableCell) => self.end_table_cell(),
            Event::Start(Tag::List(start)) => self.start_list(start.is_some()),
            Event::End(TagEnd::List(_)) => self.end_list(),
            Event::Start(Tag::Item) => self.start_item(),
            Event::End(TagEnd::Item) => self.end_item(),
            Event::Start(Tag::Image { .. }) => self.start_image(),
            Event::End(TagEnd::Image) => self.end_image(),
            Event::Start(Tag::Paragraph) if self.table.is_none() && self.item_text.is_empty() => {
                self.flush_paragraph();
            }
            Event::End(TagEnd::Paragraph) if self.table.is_none() && self.item_text.is_empty() => {
                self.flush_paragraph();
            }
            Event::Text(text) | Event::Code(text) => self.push_text(&text),
            Event::SoftBreak => self.soft_break(),
            Event::HardBreak => self.hard_break(),
            Event::InlineMath(text) => self.push_inline_math(inline_math_tex(&text)),
            Event::DisplayMath(text) => self.display_math(&text),
            _ => {}
        }
    }

    fn finish(mut self) -> StructuralSidecar {
        self.flush_paragraph();
        fill_list_parents(&mut self.nodes);
        bind_captions_and_footnotes(&mut self.nodes);
        let reading_order = (0..self.nodes.len()).collect();
        StructuralSidecar {
            nodes: self.nodes,
            reading_order,
        }
    }
}

impl StructuralSidecar {
    /// Serialize to pretty JSON (`<id>.structural.json`).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Derive a sidecar deterministically from GFM markdown, using the same
    /// pulldown-cmark options as [`crate::markdown_quality::parse_markdown_blocks`].
    pub fn from_markdown(md: &str) -> Self {
        let mut state = MarkdownParseState::new();
        for event in Parser::new_ext(md, crate::markdown_quality::md_parser_options()) {
            state.handle_event(event);
        }
        state.finish()
    }
}

/// Append `text` to `buf`, inserting a separating space when needed (mirrors the
/// spacing rule in `parse_markdown_blocks`).
fn push_spaced(buf: &mut String, text: &str) {
    if !buf.is_empty() && !buf.ends_with(' ') && !buf.ends_with('\n') {
        buf.push(' ');
    }
    buf.push_str(text);
}

/// True if a fenced block's content looks like a LaTeX formula.
fn is_formula(content: &str) -> bool {
    content.trim_start().starts_with('\\')
        || content.contains("\\frac")
        || content.contains("\\sum")
        || content.contains("\\int")
        || content.contains("\\begin{")
}

/// Fill `parent_item` for list items by scanning backwards for the nearest
/// earlier item at `depth - 1`. Informational only (not scored).
fn fill_list_parents(nodes: &mut [StructuralNode]) {
    for i in 0..nodes.len() {
        let depth = match &nodes[i] {
            StructuralNode::ListItem { depth, .. } => *depth,
            _ => continue,
        };
        if depth == 0 {
            continue;
        }
        let mut parent = None;
        for j in (0..i).rev() {
            if let StructuralNode::ListItem { depth: d, .. } = &nodes[j]
                && *d == depth - 1
            {
                parent = Some(j);
                break;
            }
        }
        if let StructuralNode::ListItem { parent_item, .. } = &mut nodes[i] {
            *parent_item = parent;
        }
    }
}

/// Convert caption-like and footnote-like paragraphs into [`StructuralNode::Caption`]
/// / [`StructuralNode::Footnote`] nodes, binding each to the nearest preceding
/// figure/table/image (captions) or content node (footnotes).
fn bind_captions_and_footnotes(nodes: &mut [StructuralNode]) {
    for i in 0..nodes.len() {
        let text = match &nodes[i] {
            StructuralNode::Paragraph { text } => text.clone(),
            _ => continue,
        };
        if is_footnote_text(&text) {
            let target = nearest_preceding(nodes, i, |n| {
                matches!(n, StructuralNode::Paragraph { .. } | StructuralNode::Table(_))
            });
            nodes[i] = StructuralNode::Footnote { binds_to: target, text };
        } else if is_caption_text(&text) {
            let target = nearest_preceding(nodes, i, |n| {
                matches!(
                    n,
                    StructuralNode::Image { .. } | StructuralNode::Table(_) | StructuralNode::Figure { .. }
                )
            });
            nodes[i] = StructuralNode::Caption { binds_to: target, text };
        }
    }
}

fn nearest_preceding(nodes: &[StructuralNode], from: usize, pred: impl Fn(&StructuralNode) -> bool) -> Option<usize> {
    (0..from).rev().find(|&j| pred(&nodes[j]))
}

fn is_caption_text(text: &str) -> bool {
    let lower = text.trim_start().to_ascii_lowercase();
    const PREFIXES: [&str; 7] = ["figure", "fig.", "table", "chart", "diagram", "scheme", "plate"];
    PREFIXES.iter().any(|p| {
        lower.strip_prefix(p).is_some_and(|rest| {
            rest.trim_start()
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit() || c == ':')
        })
    })
}

fn is_footnote_text(text: &str) -> bool {
    // GFM footnote definition `[^id]: …` rendered as literal text without the option enabled. ~keep
    let t = text.trim_start();
    t.starts_with("[^") && t.contains("]:")
}
