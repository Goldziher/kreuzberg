//! Markdown to a flat list of blocks, each carrying the indentation it is laid out at.
//!
//! The walk mirrors the DOCX writer's, so both formats carry over the same constructs.

use std::collections::HashMap;

use comrak::arena_tree::NodeEdge;
use comrak::nodes::{ListType, NodeValue, TableAlignment};
use comrak::{Arena, parse_document};

use crate::rendering::common::external_link_target;
use crate::rendering::markdown::comrak_options;

const BULLETS: [&str; 3] = ["\u{2022}", "\u{25E6}", "\u{25AA}"];
const UNCHECKED_TASK: &str = "\u{2610}";
const CHECKED_TASK: &str = "\u{2611}";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Script {
    #[default]
    Normal,
    Super,
    Sub,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Style {
    pub(super) bold: bool,
    pub(super) italic: bool,
    pub(super) code: bool,
    pub(super) strike: bool,
    pub(super) underline: bool,
    pub(super) highlight: bool,
    pub(super) script: Script,
    /// Index into [`Document::links`].
    pub(super) link: Option<usize>,
}

#[derive(Debug, PartialEq)]
pub(super) struct Span {
    /// May hold `\n`, a forced line break.
    pub(super) text: String,
    pub(super) style: Style,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role {
    Body,
    Heading(u8),
}

#[derive(Debug)]
pub(super) struct Block {
    /// Indentation, in indent steps.
    pub(super) depth: usize,
    /// The depth of each enclosing block quote, outermost first.
    pub(super) quotes: Vec<usize>,
    pub(super) kind: BlockKind,
}

#[derive(Debug)]
pub(super) enum BlockKind {
    Text {
        role: Role,
        spans: Vec<Span>,
        /// A list item's bullet or number, set in the indent before the text.
        marker: Option<String>,
        in_list: bool,
    },
    Code {
        lines: Vec<String>,
    },
    Rule,
    Table(Table),
}

#[derive(Debug)]
pub(super) struct Table {
    pub(super) alignments: Vec<TableAlignment>,
    pub(super) columns: usize,
    pub(super) rows: Vec<Row>,
}

#[derive(Debug)]
pub(super) struct Row {
    pub(super) header: bool,
    pub(super) cells: Vec<Vec<Span>>,
}

#[derive(Debug, Default)]
pub(super) struct Document {
    pub(super) blocks: Vec<Block>,
    /// External link targets, each once, in first-use order.
    pub(super) links: Vec<String>,
}

pub(super) fn parse(markdown: &str) -> Document {
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &comrak_options());

    let mut builder = Builder::default();
    for edge in root.traverse() {
        match edge {
            NodeEdge::Start(node) => builder.start(&node.data.borrow().value),
            NodeEdge::End(node) => builder.end(&node.data.borrow().value),
        }
    }
    builder.close_text();
    builder.document
}

#[derive(Default)]
struct StyleDepth {
    bold: usize,
    italic: usize,
    code: usize,
    strike: usize,
    underline: usize,
    highlight: usize,
    superscript: usize,
    subscript: usize,
    link: Option<usize>,
}

impl StyleDepth {
    fn style(&self) -> Style {
        Style {
            bold: self.bold > 0,
            italic: self.italic > 0,
            code: self.code > 0,
            strike: self.strike > 0,
            underline: self.underline > 0,
            highlight: self.highlight > 0,
            script: if self.superscript > 0 {
                Script::Super
            } else if self.subscript > 0 {
                Script::Sub
            } else {
                Script::Normal
            },
            link: self.link,
        }
    }
}

struct OpenText {
    role: Role,
    spans: Vec<Span>,
    marker: Option<String>,
    depth: usize,
    in_list: bool,
}

struct OpenTable {
    table: Table,
    depth: usize,
    quotes: Vec<usize>,
}

struct List {
    list_type: ListType,
    next_number: usize,
}

#[derive(Default)]
struct Builder {
    document: Document,
    link_index: HashMap<String, usize>,
    style: StyleDepth,
    text: Option<OpenText>,
    table: Option<OpenTable>,
    quotes: Vec<usize>,
    details_depth: usize,
    lists: Vec<List>,
    /// Per enclosing list item: its marker, until its first block has carried it.
    item_markers: Vec<Option<String>>,
    footnote_label: Option<String>,
}

impl Builder {
    fn start(&mut self, value: &NodeValue) {
        match value {
            NodeValue::Paragraph => self.open_text(Role::Body),
            NodeValue::Heading(heading) => self.open_text(Role::Heading(heading.level.clamp(1, 6))),
            NodeValue::CodeBlock(block) if block.info.split_ascii_whitespace().next() == Some("math") => {
                self.open_text(Role::Body);
                self.text(block.literal.strip_suffix('\n').unwrap_or(&block.literal));
                self.close_text();
            }
            NodeValue::CodeBlock(block) => self.code_block(&block.literal),
            NodeValue::FrontMatter(text) => self.code_block(text),
            NodeValue::HtmlBlock(block) => {
                self.open_text(Role::Body);
                self.text(block.literal.strip_suffix('\n').unwrap_or(&block.literal));
                self.close_text();
            }
            NodeValue::ThematicBreak => {
                self.close_text();
                self.push_block(BlockKind::Rule);
            }
            NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_) => self.open_quote(),
            NodeValue::Alert(alert) => {
                self.open_quote();
                let title = alert
                    .title
                    .clone()
                    .unwrap_or_else(|| alert.alert_type.default_title().to_string());
                self.style.bold += 1;
                self.open_text(Role::Body);
                self.text(&title);
                self.close_text();
                self.style.bold -= 1;
            }
            NodeValue::List(list) => {
                self.close_text();
                self.flush_item_marker();
                self.lists.push(List {
                    list_type: list.list_type,
                    next_number: list.start,
                });
            }
            NodeValue::Item(_) => {
                self.close_text();
                let marker = self.next_list_marker();
                self.item_markers.push(Some(marker));
            }
            NodeValue::TaskItem(task) => {
                self.close_text();
                self.next_list_marker();
                let marker = if task.symbol.is_some() {
                    CHECKED_TASK
                } else {
                    UNCHECKED_TASK
                };
                self.item_markers.push(Some(marker.to_string()));
            }
            NodeValue::DescriptionTerm => {
                self.close_text();
                self.style.bold += 1;
            }
            NodeValue::DescriptionDetails => {
                self.close_text();
                self.details_depth += 1;
            }
            NodeValue::FootnoteDefinition(definition) => {
                self.close_text();
                self.footnote_label = Some(definition.name.clone());
            }
            NodeValue::Table(_) | NodeValue::TableRow(_) | NodeValue::TableCell => self.start_table_node(value),
            _ => self.start_inline(value),
        }
    }

    fn start_table_node(&mut self, value: &NodeValue) {
        match value {
            NodeValue::Table(table) => {
                self.close_text();
                self.flush_item_marker();
                self.table = Some(OpenTable {
                    table: Table {
                        alignments: table.alignments.clone(),
                        columns: table.num_columns.max(1),
                        rows: Vec::new(),
                    },
                    depth: self.depth(),
                    quotes: self.quotes.clone(),
                });
            }
            NodeValue::TableRow(header) => {
                if let Some(open) = self.table.as_mut() {
                    open.table.rows.push(Row {
                        header: *header,
                        cells: Vec::new(),
                    });
                }
            }
            NodeValue::TableCell => {
                if let Some(row) = self.table.as_mut().and_then(|open| open.table.rows.last_mut()) {
                    row.cells.push(Vec::new());
                }
            }
            _ => {}
        }
    }

    fn start_inline(&mut self, value: &NodeValue) {
        match value {
            NodeValue::Text(text) => self.text(text),
            NodeValue::SoftBreak => self.text(" "),
            NodeValue::LineBreak => self.text("\n"),
            NodeValue::Code(code) => {
                self.style.code += 1;
                self.text(&code.literal);
                self.style.code -= 1;
            }
            NodeValue::Math(math) => self.text(&math.literal),
            NodeValue::HtmlInline(text) | NodeValue::Raw(text) => self.text(text),
            NodeValue::EscapedTag(text) => self.text(text),
            NodeValue::FootnoteReference(reference) => {
                self.style.superscript += 1;
                self.text(&reference.name);
                self.style.superscript -= 1;
            }
            NodeValue::Emph => self.style.italic += 1,
            NodeValue::Strong => self.style.bold += 1,
            NodeValue::Strikethrough => self.style.strike += 1,
            NodeValue::Underline | NodeValue::Insert => self.style.underline += 1,
            NodeValue::Highlight => self.style.highlight += 1,
            NodeValue::Superscript => self.style.superscript += 1,
            NodeValue::Subscript => self.style.subscript += 1,
            NodeValue::Link(link) if self.style.link.is_none() => {
                self.style.link = self.link_target(&link.url);
            }
            _ => {}
        }
    }

    fn end(&mut self, value: &NodeValue) {
        match value {
            NodeValue::Paragraph | NodeValue::Heading(_) => self.close_text(),
            NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_) | NodeValue::Alert(_) => {
                self.close_text();
                self.quotes.pop();
            }
            NodeValue::List(_) => {
                self.close_text();
                self.lists.pop();
            }
            NodeValue::Item(_) | NodeValue::TaskItem(_) => {
                self.close_text();
                self.flush_item_marker();
                self.item_markers.pop();
            }
            NodeValue::DescriptionTerm => {
                self.close_text();
                self.style.bold -= 1;
            }
            NodeValue::DescriptionDetails => {
                self.close_text();
                self.details_depth -= 1;
            }
            NodeValue::FootnoteDefinition(_) => {
                self.close_text();
                self.footnote_label = None;
            }
            NodeValue::Table(_) => {
                if let Some(mut open) = self.table.take() {
                    let columns = open.table.columns;
                    for row in &mut open.table.rows {
                        row.cells.resize_with(columns, Vec::new);
                    }
                    self.document.blocks.push(Block {
                        depth: open.depth,
                        quotes: open.quotes,
                        kind: BlockKind::Table(open.table),
                    });
                }
            }
            NodeValue::Emph => self.style.italic -= 1,
            NodeValue::Strong => self.style.bold -= 1,
            NodeValue::Strikethrough => self.style.strike -= 1,
            NodeValue::Underline | NodeValue::Insert => self.style.underline -= 1,
            NodeValue::Highlight => self.style.highlight -= 1,
            NodeValue::Superscript => self.style.superscript -= 1,
            NodeValue::Subscript => self.style.subscript -= 1,
            NodeValue::Link(_) => self.style.link = None,
            _ => {}
        }
    }

    /// Nesting in indent steps, the same measure the DOCX writer indents by.
    fn depth(&self) -> usize {
        self.quotes.len() + self.lists.len() + self.details_depth
    }

    fn open_quote(&mut self) {
        self.close_text();
        self.quotes.push(self.depth());
    }

    fn open_text(&mut self, role: Role) {
        self.close_text();
        let marker = self.take_item_marker();
        self.text = Some(OpenText {
            role,
            spans: Vec::new(),
            marker,
            depth: self.depth(),
            in_list: !self.lists.is_empty(),
        });

        if let Some(label) = self.footnote_label.take() {
            self.style.superscript += 1;
            self.text(&label);
            self.style.superscript -= 1;
            self.text(" ");
        }
    }

    fn close_text(&mut self) {
        let Some(open) = self.text.take() else {
            return;
        };
        self.document.blocks.push(Block {
            depth: open.depth,
            quotes: self.quotes.clone(),
            kind: BlockKind::Text {
                role: open.role,
                spans: open.spans,
                marker: open.marker,
                in_list: open.in_list,
            },
        });
    }

    fn push_block(&mut self, kind: BlockKind) {
        self.document.blocks.push(Block {
            depth: self.depth(),
            quotes: self.quotes.clone(),
            kind,
        });
    }

    fn code_block(&mut self, literal: &str) {
        self.close_text();
        self.flush_item_marker();
        let literal = literal.strip_suffix('\n').unwrap_or(literal);
        let lines = literal.split('\n').map(str::to_string).collect();
        self.push_block(BlockKind::Code { lines });
    }

    fn next_list_marker(&mut self) -> String {
        let level = self.lists.len().saturating_sub(1);
        let Some(list) = self.lists.last_mut() else {
            return BULLETS[0].to_string();
        };
        match list.list_type {
            ListType::Bullet => BULLETS[level % BULLETS.len()].to_string(),
            ListType::Ordered => {
                let number = list.next_number;
                list.next_number += 1;
                format!("{number}.")
            }
        }
    }

    fn take_item_marker(&mut self) -> Option<String> {
        self.item_markers.last_mut()?.take()
    }

    /// Give an item whose content does not open with a paragraph (an empty item, or one
    /// that starts with a nested list, a table or a code block) a line of its own for its
    /// marker, so the marker lands before that content.
    fn flush_item_marker(&mut self) {
        if self.item_markers.last().is_some_and(Option::is_some) {
            self.open_text(Role::Body);
            self.close_text();
        }
    }

    fn text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let style = self.style.style();
        let spans = if let Some(open) = self.table.as_mut() {
            let Some(cell) = open.table.rows.last_mut().and_then(|row| row.cells.last_mut()) else {
                return;
            };
            cell
        } else {
            if self.text.is_none() {
                self.open_text(Role::Body);
            }
            let Some(open) = self.text.as_mut() else {
                return;
            };
            &mut open.spans
        };
        match spans.last_mut() {
            Some(last) if last.style == style => last.text.push_str(text),
            _ => spans.push(Span {
                text: text.to_string(),
                style,
            }),
        }
    }

    fn link_target(&mut self, url: &str) -> Option<usize> {
        let target = external_link_target(url)?;
        let next = self.document.links.len();
        let index = *self.link_index.entry(target.clone()).or_insert(next);
        if index == next {
            self.document.links.push(target);
        }
        Some(index)
    }
}
