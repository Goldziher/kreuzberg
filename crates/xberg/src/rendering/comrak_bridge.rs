//! Convert an `InternalDocument` into a comrak AST.
//!
//! This module builds a comrak `AstNode` tree from our internal flat element
//! representation.  The resulting tree can be serialized to CommonMark, HTML,
//! or any other format comrak supports via `comrak::format_commonmark` etc.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashSet;

use comrak::nodes::{
    Ast, AstNode, LineColumn, NodeAlert, NodeCode, NodeCodeBlock, NodeFootnoteDefinition, NodeFootnoteReference,
    NodeHeading, NodeLink, NodeList, NodeMath, NodeTable, NodeValue, TableAlignment,
};

use crate::types::document_structure::{AnnotationKind, ContentLayer, TextAnnotation};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement, list_item_source_label_from_attributes};

use super::common::{
    FootnoteCollector, NestingKind, RenderState, annotated_math, handle_container_end, is_body_element,
    is_container_end, parse_metadata_entries, sorted_inline_annotations, valid_math_annotation,
};

/// Minimum valid ATX heading depth (CommonMark `#`).
const MIN_HEADING_LEVEL: u8 = 1;

/// Maximum valid ATX heading depth (CommonMark `######`). A level beyond this
/// is not a valid ATX heading — more than six leading `#` characters are not
/// recognized as a heading on re-parse and silently degrade to a paragraph —
/// so a level reported by upstream structure detection is clamped rather than
/// forwarded verbatim.
const MAX_HEADING_LEVEL: u8 = 6;

/// Allocate a comrak AST node in the arena with the given `NodeValue`.
fn mk<'a>(arena: &'a comrak::Arena<'a>, value: NodeValue) -> &'a AstNode<'a> {
    let ast = Ast::new(value, LineColumn { line: 0, column: 0 });
    arena.alloc(AstNode::new(RefCell::new(ast)))
}

/// Create an inline `Text` node with normalized whitespace.
///
/// Collapses multiple consecutive spaces into one (fixes MD064) and trims
/// leading/trailing whitespace from emphasis spans (fixes MD037).
fn mk_text<'a>(arena: &'a comrak::Arena<'a>, text: &str) -> &'a AstNode<'a> {
    let normalized = normalize_text(text);
    mk(arena, NodeValue::Text(Cow::Owned(normalized)))
}

/// Collapse multiple consecutive spaces into a single space.
fn normalize_text(text: &str) -> String {
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
    result
}

/// Rewrite the AST so text inside code spans, code blocks and math cannot turn into raw HTML in
/// Markdown output. Math becomes code math: a ```` ```math ```` fence for display math alone in a
/// paragraph, `` $`tex`$ `` otherwise. Math whose TeX holds a backtick or `$`, or that shares a
/// block (widened to its outermost tight list item) with code holding `$`, is written as a plain
/// code span. The HTML writer escapes all of these itself.
pub(crate) fn keep_code_literal<'a>(root: &'a AstNode<'a>, arena: &'a comrak::Arena<'a>) {
    for node in root.descendants().collect::<Vec<_>>() {
        let math = match &mut node.data.borrow_mut().value {
            NodeValue::CodeBlock(block) => {
                block.literal = unix_lines(&block.literal);
                block.info = one_line(&block.info);
                continue;
            }
            // comrak's writer indexes the first byte of a code span.
            NodeValue::Code(code) if code.literal.is_empty() => None,
            NodeValue::Math(math) => Some(std::mem::take(math)),
            _ => continue,
        };
        match math {
            Some(math) => write_math_as_code(node, math),
            None => node.detach(),
        }
    }
    for node in root.descendants().collect::<Vec<_>>() {
        trim_paragraph_start(node);
        // An empty paragraph writes nothing, but it opens a loose item with a blank line, and an
        // item may begin with only one, so the item's real content would fall out of it.
        if matches!(node.data.borrow().value, NodeValue::Paragraph) && node.first_child().is_none() {
            node.detach();
        }
    }
    for node in root.descendants() {
        set_tight(node);
    }
    // Readers pair "$" across code spans and across a tight item's paragraphs, which comrak writes one newline apart.
    let mut seen = HashSet::new();
    let blocks: Vec<_> = root
        .descendants()
        .filter(|node| matches!(&node.data.borrow().value, NodeValue::Code(code) if code.literal.contains('$')))
        .filter_map(|node| node.ancestors().find(|ancestor| ancestor.data.borrow().value.block()))
        .map(|block| {
            block
                .ancestors()
                .filter(|node| in_tight_item(node))
                .last()
                .unwrap_or(block)
        })
        .filter(|block| seen.insert(std::ptr::from_ref(*block)))
        .collect();
    // A block inside another in the set is covered by the outer block's walk.
    let outermost = blocks.into_iter().filter(|block| {
        !block
            .ancestors()
            .skip(1)
            .any(|ancestor| seen.contains(&std::ptr::from_ref(ancestor)))
    });
    for block in outermost {
        for node in block.descendants().collect::<Vec<_>>() {
            escape_dollars(node, arena);
        }
    }
    // comrak writes no "**" for a Strong inside a Strong, so code spans either side of one are
    // adjacent in the output. Flatten it, as the writer does, so the merge below sees them.
    for node in root.descendants().collect::<Vec<_>>() {
        flatten_nested_strong(node);
    }
    for node in root.descendants().collect::<Vec<_>>() {
        merge_following_code(node);
    }
}

/// A reader strips a paragraph's leading whitespace, but after a list marker it moves the item's
/// content column, which comrak does not follow when it indents the item's other blocks.
fn trim_paragraph_start<'a>(node: &'a AstNode<'a>) {
    if !matches!(node.data.borrow().value, NodeValue::Paragraph) {
        return;
    }
    while let Some(child) = node.first_child() {
        let empty = match &mut child.data.borrow_mut().value {
            NodeValue::Text(text) => {
                let blank = text.len() - text.trim_start_matches([' ', '\t', '\r', '\n']).len();
                if blank > 0 {
                    text.to_mut().drain(..blank);
                }
                text.is_empty()
            }
            _ => false,
        };
        if !empty {
            return;
        }
        child.detach();
    }
}

/// comrak writes a tight item's blocks one newline apart, which a reader reads back only if each
/// item is one paragraph, optionally followed by a list that can interrupt it: CommonMark 5.2 lets
/// a list interrupt a paragraph only if its first item is not empty and, when ordered, starts at 1.
fn set_tight<'a>(node: &'a AstNode<'a>) {
    if !matches!(node.data.borrow().value, NodeValue::List(_)) {
        return;
    }
    let interrupts_paragraph = |list: &'a AstNode<'a>| {
        matches!(&list.data.borrow().value, NodeValue::List(nested)
            if nested.list_type == comrak::nodes::ListType::Bullet || nested.start == 1)
            && list
                .first_child()
                .and_then(|item| item.first_child())
                .is_some_and(opens_with_text)
    };
    let tight = node.children().all(|item| {
        let mut blocks = item.children();
        let (first, second) = (blocks.next(), blocks.next());
        first.is_some_and(opens_with_text) && blocks.next().is_none() && second.is_none_or(interrupts_paragraph)
    });
    if let NodeValue::List(list) = &mut node.data.borrow_mut().value {
        list.tight = tight;
    }
}

fn opens_with_text<'a>(paragraph: &'a AstNode<'a>) -> bool {
    matches!(paragraph.data.borrow().value, NodeValue::Paragraph)
        && paragraph
            .first_child()
            .is_some_and(|child| match &child.data.borrow().value {
                NodeValue::Text(text) => !text.is_empty(),
                NodeValue::SoftBreak | NodeValue::LineBreak => false,
                _ => true,
            })
}

fn flatten_nested_strong<'a>(node: &'a AstNode<'a>) {
    let nested = matches!(node.data.borrow().value, NodeValue::Strong)
        && node
            .parent()
            .is_some_and(|parent| matches!(parent.data.borrow().value, NodeValue::Strong));
    if !nested {
        return;
    }
    for child in node.children().collect::<Vec<_>>() {
        node.insert_before(child);
    }
    node.detach();
}

/// comrak sizes each code span's backtick fence alone, so a reader pairs a backtick of one span
/// with a lone backtick inside the next. Adjacent spans read as one, so they are written as one.
fn merge_following_code<'a>(node: &'a AstNode<'a>) {
    if !matches!(node.data.borrow().value, NodeValue::Code(_)) {
        return;
    }
    while let Some(next) = node.next_sibling() {
        let literal = match &mut next.data.borrow_mut().value {
            NodeValue::Text(text) if text.is_empty() => None,
            NodeValue::Code(code) => Some(std::mem::take(&mut code.literal)),
            _ => break,
        };
        next.detach();
        if let (Some(literal), NodeValue::Code(code)) = (literal, &mut node.data.borrow_mut().value) {
            code.literal.push_str(&literal);
        }
    }
}

fn in_tight_item(node: &AstNode<'_>) -> bool {
    matches!(node.data.borrow().value, NodeValue::Item(_) | NodeValue::TaskItem(_))
        && node
            .parent()
            .is_some_and(|list| matches!(&list.data.borrow().value, NodeValue::List(list) if list.tight))
}

fn write_math_as_code<'a>(node: &'a AstNode<'a>, math: NodeMath) {
    if math.literal.trim().is_empty() {
        node.detach();
        return;
    }
    if math.display_math
        && let Some(paragraph) = node.parent().filter(|parent| holds_only(parent, node))
    {
        for child in paragraph.children().collect::<Vec<_>>() {
            child.detach();
        }
        paragraph.data.borrow_mut().value = NodeValue::CodeBlock(Box::new(NodeCodeBlock {
            fenced: true,
            fence_char: b'`',
            fence_length: 3,
            fence_offset: 0,
            info: "math".to_string(),
            literal: unix_lines(&math.literal),
            closed: true,
        }));
        return;
    }
    let literal = one_line(&math.literal);
    // Code math ends at the first "`$", and a dollar-only parser's math at the first "$".
    node.data.borrow_mut().value = if literal.contains(['`', '$']) {
        NodeValue::Code(NodeCode {
            num_backticks: 1,
            literal,
        })
    } else {
        NodeValue::Math(NodeMath {
            dollar_math: false,
            display_math: math.display_math,
            literal,
        })
    };
}

/// Write each `$` in Text as `\$` and code math as a code span. comrak escapes a backslash in Text.
fn escape_dollars<'a>(node: &'a AstNode<'a>, arena: &'a comrak::Arena<'a>) {
    let mut data = node.data.borrow_mut();
    match &mut data.value {
        NodeValue::Math(math) => {
            let literal = std::mem::take(&mut math.literal);
            data.value = NodeValue::Code(NodeCode {
                num_backticks: 1,
                literal,
            });
        }
        NodeValue::Text(text) if text.contains('$') => {
            let prose = std::mem::take(text).into_owned();
            let (head, last) = prose.rsplit_once('$').expect("text holds a dollar");
            for piece in head.split('$') {
                if !piece.is_empty() {
                    node.insert_before(mk(arena, NodeValue::Text(Cow::Owned(piece.to_string()))));
                }
                node.insert_before(mk(arena, NodeValue::Raw(r"\$".to_string())));
            }
            *text = Cow::Owned(last.to_string());
        }
        _ => {}
    }
}

/// A CommonMark line ends at a lone "\r" too, and comrak re-adds a quote or list prefix only after "\n".
fn unix_lines(text: &str) -> String {
    text.split("\r\n")
        .map(|line| line.replace('\r', "\n"))
        .collect::<Vec<_>>()
        .join("\r\n")
}

/// A line inside a code span can open an HTML block, which interrupts the paragraph.
fn one_line(text: &str) -> String {
    text.replace("\r\n", " ").replace(['\r', '\n'], " ")
}

fn holds_only<'a>(paragraph: &'a AstNode<'a>, node: &'a AstNode<'a>) -> bool {
    matches!(paragraph.data.borrow().value, NodeValue::Paragraph)
        && paragraph.children().all(|child| {
            child.same_node(node)
                || match &child.data.borrow().value {
                    NodeValue::Text(text) => text.trim().is_empty(),
                    NodeValue::SoftBreak | NodeValue::LineBreak => true,
                    _ => false,
                }
        })
}

/// Check whether ALL annotations on an element cover the full text with the
/// same annotation kind.  Returns the kind if so.
fn uniform_annotation_kind(elem: &InternalElement) -> Option<&AnnotationKind> {
    let mut formatting_ann: Option<&AnnotationKind> = None;
    for ann in &elem.annotations {
        if valid_math_annotation(&elem.text, ann) {
            return None;
        }
        match &ann.kind {
            AnnotationKind::Bold | AnnotationKind::Italic | AnnotationKind::Strikethrough => {
                if ann.start == 0 && ann.end as usize >= elem.text.len() {
                    if formatting_ann.is_some() {
                        return None;
                    }
                    formatting_ann = Some(&ann.kind);
                } else {
                    return None;
                }
            }
            _ => {}
        }
    }
    formatting_ann
}

/// Check if text ends at a sentence boundary (period, exclamation, question mark, colon).
fn ends_at_sentence_boundary(text: &str) -> bool {
    let trimmed = text.trim_end();
    trimmed.ends_with('.') || trimmed.ends_with('!') || trimmed.ends_with('?') || trimmed.ends_with(':')
}

/// Returns true if two annotation kinds are "merge-compatible" (same variant).
fn same_annotation_variant(a: &AnnotationKind, b: &AnnotationKind) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

/// Pre-process elements: merge consecutive fully-annotated paragraphs of the
/// same kind into a single paragraph.  Returns a new element list with the
/// consolidated paragraphs.
fn consolidate_paragraphs(elements: &[InternalElement]) -> Vec<ConsolidatedElement> {
    let mut result: Vec<ConsolidatedElement> = Vec::with_capacity(elements.len());
    let mut i = 0;

    while i < elements.len() {
        let elem = &elements[i];

        if elem.kind == ElementKind::Paragraph && elem.layer == ContentLayer::Body && !elem.text.is_empty() {
            let uniform = uniform_annotation_kind(elem);
            tracing::trace!(
                idx = i,
                text_len = elem.text.len(),
                annotation_count = elem.annotations.len(),
                uniform_kind = ?uniform,
                text_preview = elem.text.chars().take(60).collect::<String>(),
                "paragraph consolidation candidate"
            );
        }
        if elem.kind == ElementKind::Paragraph
            && elem.layer == ContentLayer::Body
            && !elem.text.is_empty()
            && let Some(kind) = uniform_annotation_kind(elem)
        {
            let mut merged_text = elem.text.clone();
            let mut j = i + 1;
            while j < elements.len() {
                if ends_at_sentence_boundary(&merged_text) {
                    break;
                }
                let next = &elements[j];
                if next.kind != ElementKind::Paragraph || next.layer != ContentLayer::Body || next.text.is_empty() {
                    break;
                }
                if let Some(next_kind) = uniform_annotation_kind(next)
                    && same_annotation_variant(kind, next_kind)
                {
                    merged_text.push(' ');
                    merged_text.push_str(&next.text);
                    j += 1;
                    continue;
                }
                break;
            }

            if j > i + 1 {
                tracing::debug!(
                    start_idx = i,
                    end_idx = j,
                    merged_count = j - i,
                    kind = ?kind,
                    merged_text_len = merged_text.len(),
                    "consolidated paragraphs"
                );
                let ann = TextAnnotation {
                    start: 0,
                    end: merged_text.len() as u32,
                    kind: kind.clone(),
                };
                result.push(ConsolidatedElement::Merged {
                    text: merged_text,
                    annotations: vec![ann],
                });
                i = j;
                continue;
            }
        }

        result.push(ConsolidatedElement::Original(i));
        i += 1;
    }

    result
}

/// Either a reference to an original element (by index) or a merged paragraph.
enum ConsolidatedElement {
    Original(usize),
    Merged {
        text: String,
        annotations: Vec<TextAnnotation>,
    },
}

impl ConsolidatedElement {
    fn resolve<'b>(&'b self, elements: &'b [InternalElement]) -> ElementView<'b> {
        match self {
            ConsolidatedElement::Original(idx) => ElementView::Ref(&elements[*idx]),
            ConsolidatedElement::Merged { text, annotations, .. } => ElementView::Merged { text, annotations },
        }
    }

    fn original_index(&self) -> Option<usize> {
        match self {
            ConsolidatedElement::Original(idx) => Some(*idx),
            ConsolidatedElement::Merged { .. } => None,
        }
    }
}

enum ElementView<'a> {
    Ref(&'a InternalElement),
    Merged {
        text: &'a str,
        annotations: &'a [TextAnnotation],
    },
}

/// Build inline comrak nodes from text with byte-range annotations.
///
/// Sorts annotations by start position, preferring wider spans first and `Link`
/// before formatting annotations when ranges coincide.  Annotations fully
/// contained within an outer annotation are nested recursively rather than
/// skipped, so `Bold[0,5] + Link[0,5]` produces `[**text**](url)` instead of
/// dropping the link.
fn build_inlines<'a>(
    arena: &'a comrak::Arena<'a>,
    parent: &'a AstNode<'a>,
    text: &str,
    annotations: &[TextAnnotation],
) {
    if annotations.is_empty() {
        if !text.is_empty() {
            parent.append(mk_text(arena, text));
        }
        return;
    }

    let sorted = sorted_inline_annotations(text, annotations);

    let len = text.len() as u32;
    let mut pos: u32 = 0;
    let mut i = 0;

    while i < sorted.len() {
        let ann = sorted[i];
        let start = text.ceil_char_boundary(ann.start.min(len) as usize) as u32;
        let end = text.floor_char_boundary(ann.end.min(len) as usize) as u32;

        if start < pos {
            tracing::trace!(
                ann_start = start,
                ann_end = end,
                current_pos = pos,
                "skipping annotation already consumed as inner"
            );
            i += 1;
            continue;
        }

        if start >= end {
            i += 1;
            continue;
        }

        if start > pos {
            let gap = &text[pos as usize..start as usize];
            if !gap.is_empty() {
                parent.append(mk_text(arena, gap));
            }
        }

        let span = &text[start as usize..end as usize];

        // `sorted` is ordered by start, so the first annotation starting at or after `end` ends
        // the scan. Without the cut every annotation rescans the whole tail, which is quadratic
        // for a block with many annotations (GH#2099). ~keep
        let inner: Vec<TextAnnotation> = sorted[i + 1..]
            .iter()
            .map_while(|ia| {
                let is = text.ceil_char_boundary(ia.start.min(len) as usize) as u32;
                (is < end).then_some((ia, is))
            })
            .filter_map(|(ia, is)| {
                let ie = text.floor_char_boundary(ia.end.min(len) as usize) as u32;
                if is >= start && ie <= end && is < ie {
                    Some(TextAnnotation {
                        start: is - start,
                        end: ie - start,
                        kind: ia.kind.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        append_annotated_span(arena, parent, span, &ann.kind, &inner);
        pos = end;

        i += 1;
        while i < sorted.len() {
            let ns = text.ceil_char_boundary(sorted[i].start.min(len) as usize) as u32;
            if ns < end {
                i += 1;
            } else {
                break;
            }
        }
    }

    if (pos as usize) < text.len() {
        let tail = &text[pos as usize..];
        if !tail.is_empty() {
            parent.append(mk_text(arena, tail));
        }
    }
}

/// Create a comrak inline wrapper node for the given annotation kind and
/// append it to `parent`.
///
/// `inner` contains annotations (with offsets relative to `span`) that are
/// fully contained within this span and should be rendered as nested nodes
/// rather than plain text.
///
/// Trims leading/trailing whitespace from emphasis/strong spans to avoid
/// MD037 (spaces inside emphasis markers). Whitespace outside the markers
/// is emitted as separate Text nodes.
fn append_annotated_span<'a>(
    arena: &'a comrak::Arena<'a>,
    parent: &'a AstNode<'a>,
    span: &str,
    kind: &AnnotationKind,
    inner: &[TextAnnotation],
) {
    if let Some((literal, display_math)) = annotated_math(span, kind) {
        parent.append(mk(
            arena,
            NodeValue::Math(NodeMath {
                dollar_math: true,
                display_math,
                literal: literal.to_string(),
            }),
        ));
        return;
    }

    let (leading_ws, trimmed, trailing_ws) = if matches!(
        kind,
        AnnotationKind::Bold | AnnotationKind::Italic | AnnotationKind::Strikethrough
    ) {
        let trimmed = span.trim();
        if trimmed.is_empty() {
            if !span.is_empty() {
                parent.append(mk_text(arena, span));
            }
            return;
        }
        let leading = &span[..span.len() - span.trim_start().len()];
        let trailing = &span[span.trim_end().len()..];
        (leading, trimmed, trailing)
    } else {
        ("", span, "")
    };

    let leading_len = leading_ws.len() as u32;
    let trimmed_len = trimmed.len() as u32;
    let adjusted_inner: Vec<TextAnnotation>;
    let inner_for_node: &[TextAnnotation] = if leading_len == 0 {
        inner
    } else {
        adjusted_inner = inner
            .iter()
            .filter_map(|ia| {
                let is = ia.start.saturating_sub(leading_len);
                let ie = ia.end.saturating_sub(leading_len).min(trimmed_len);
                if is < ie {
                    Some(TextAnnotation {
                        start: is,
                        end: ie,
                        kind: ia.kind.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();
        &adjusted_inner
    };

    if !leading_ws.is_empty() {
        parent.append(mk_text(arena, leading_ws));
    }

    match kind {
        AnnotationKind::Bold => {
            let strong = mk(arena, NodeValue::Strong);
            build_inlines(arena, strong, trimmed, inner_for_node);
            parent.append(strong);
        }
        AnnotationKind::Italic => {
            let emph = mk(arena, NodeValue::Emph);
            build_inlines(arena, emph, trimmed, inner_for_node);
            parent.append(emph);
        }
        AnnotationKind::Code => {
            if !trimmed.is_empty() {
                let code = mk(
                    arena,
                    NodeValue::Code(NodeCode {
                        num_backticks: 1,
                        literal: normalize_text(trimmed),
                    }),
                );
                parent.append(code);
            }
        }
        AnnotationKind::Strikethrough => {
            let strike = mk(arena, NodeValue::Strikethrough);
            build_inlines(arena, strike, trimmed, inner_for_node);
            parent.append(strike);
        }
        AnnotationKind::Underline => {
            let underline = mk(arena, NodeValue::Underline);
            build_inlines(arena, underline, trimmed, inner_for_node);
            parent.append(underline);
        }
        AnnotationKind::Subscript => {
            let sub = mk(arena, NodeValue::Subscript);
            build_inlines(arena, sub, trimmed, inner_for_node);
            parent.append(sub);
        }
        AnnotationKind::Superscript => {
            let sup = mk(arena, NodeValue::Superscript);
            build_inlines(arena, sup, trimmed, inner_for_node);
            parent.append(sup);
        }
        AnnotationKind::Highlight => {
            let hl = mk(arena, NodeValue::Highlight);
            build_inlines(arena, hl, trimmed, inner_for_node);
            parent.append(hl);
        }
        AnnotationKind::Link { url, title } => {
            let link = mk(
                arena,
                NodeValue::Link(Box::new(NodeLink {
                    url: url.clone(),
                    title: title.as_deref().unwrap_or("").to_string(),
                })),
            );
            build_inlines(arena, link, trimmed, inner);
            parent.append(link);
        }
        AnnotationKind::Color { .. } | AnnotationKind::FontSize { .. } | AnnotationKind::Custom { .. } => {
            if inner_for_node
                .iter()
                .any(|annotation| valid_math_annotation(trimmed, annotation))
            {
                let visible_children: Vec<_> = inner_for_node
                    .iter()
                    .filter(|annotation| {
                        annotation.kind.math_display().is_some()
                            || !matches!(
                                &annotation.kind,
                                AnnotationKind::Color { .. }
                                    | AnnotationKind::FontSize { .. }
                                    | AnnotationKind::Custom { .. }
                            )
                    })
                    .cloned()
                    .collect();
                build_inlines(arena, parent, trimmed, &visible_children);
            } else {
                parent.append(mk_text(arena, trimmed));
            }
        }
    }

    if !trailing_ws.is_empty() {
        parent.append(mk_text(arena, trailing_ws));
    }
}

/// Build a comrak `Table` subtree from a 2-D cell grid.
///
/// Returns `None` when the grid has no columns at all (every row is empty).
/// Comrak's CommonMark writer only emits the required `|---|` delimiter row
/// as a side effect of writing the last header *cell*; with zero cells that
/// write never happens, so a zero-column table would render as a single bare
/// `|` line — not a valid GFM table, and not parseable as one on re-read.
/// Callers should fall back to something else (e.g. the table's flat
/// `markdown` field) rather than emit that.
fn build_table<'a>(arena: &'a comrak::Arena<'a>, cells: &[Vec<String>], has_header: bool) -> Option<&'a AstNode<'a>> {
    let num_cols = cells.iter().map(|r| r.len()).max().unwrap_or(0);
    if num_cols == 0 {
        return None;
    }
    let synthetic_header_rows = usize::from(!has_header);

    let table_node = mk(
        arena,
        NodeValue::Table(Box::new(NodeTable {
            alignments: vec![TableAlignment::None; num_cols],
            num_columns: num_cols,
            num_rows: cells.len() + synthetic_header_rows,
            num_nonempty_cells: cells.iter().flat_map(|r| r.iter()).filter(|c| !c.is_empty()).count(),
        })),
    );

    if !has_header {
        let header_row = mk(arena, NodeValue::TableRow(true));
        for _ in 0..num_cols {
            header_row.append(mk(arena, NodeValue::TableCell));
        }
        table_node.append(header_row);
    }

    for (row_idx, row) in cells.iter().enumerate() {
        let is_header = has_header && row_idx == 0;
        let row_node = mk(arena, NodeValue::TableRow(is_header));

        for col in 0..num_cols {
            let cell_node = mk(arena, NodeValue::TableCell);
            let content = row.get(col).map(|s| s.as_str()).unwrap_or("");
            if !content.is_empty() {
                cell_node.append(mk_text(arena, content));
            }
            row_node.append(cell_node);
        }

        table_node.append(row_node);
    }

    Some(table_node)
}

/// Bullet glyphs that OCR/plain-text extraction commonly reads as unordered
/// list markers.
const BULLET_MARKER_CHARS: &[char] = &['-', '*', '•', '‣', '◦'];

/// Byte length of a leading list marker in `text` — a bullet glyph, or a
/// digit run followed by `.`/`)` — when followed by at least one space.
///
/// A `ListItem` element is expected to carry only its label in
/// [`InternalElement::text`]; the marker itself (`1.`, `-`, …) is emitted by
/// the comrak list/item writer. A source that leaves its own marker in the
/// label text (OCR reads the marker as part of the line) would otherwise
/// produce a doubled marker such as `1. 1. Label`, so callers stripping list
/// content use this to detect and remove it first. Returns 0 when no such
/// marker is present, leaving genuine content untouched.
fn redundant_list_marker_len(text: &str) -> usize {
    let mut chars = text.chars();
    if let Some(first) = chars.next()
        && BULLET_MARKER_CHARS.contains(&first)
    {
        let bullet_len = first.len_utf8();
        let rest = &text[bullet_len..];
        let space_len = rest.len() - rest.trim_start_matches(' ').len();
        return if space_len > 0 { bullet_len + space_len } else { 0 };
    }

    let digits_len = text.find(|c: char| !c.is_ascii_digit()).unwrap_or(text.len());
    if digits_len == 0 {
        return 0;
    }
    let after_digits = &text[digits_len..];
    let Some(delimiter) = after_digits.chars().next().filter(|&c| c == '.' || c == ')') else {
        return 0;
    };
    let after_delimiter = &after_digits[delimiter.len_utf8()..];
    let space_len = after_delimiter.len() - after_delimiter.trim_start_matches(' ').len();
    if space_len == 0 {
        return 0;
    }
    digits_len + delimiter.len_utf8() + space_len
}

/// Strip a redundant leading list marker (see [`redundant_list_marker_len`])
/// from `text`, shifting `annotations` to match.
///
/// Returns the possibly-shortened text together with adjusted annotations.
/// Annotations that fall entirely within the stripped marker are dropped;
/// annotations that overlap it are clipped to start at the new text origin.
/// Allocates only when a marker is actually found, so the common case (no
/// embedded marker) is zero-cost.
fn strip_redundant_list_marker<'a>(
    text: &'a str,
    annotations: &'a [TextAnnotation],
) -> (&'a str, Cow<'a, [TextAnnotation]>) {
    let marker_len = redundant_list_marker_len(text);
    if marker_len == 0 {
        return (text, Cow::Borrowed(annotations));
    }

    let offset = marker_len as u32;
    let adjusted: Vec<TextAnnotation> = annotations
        .iter()
        .filter(|ann| ann.end > offset)
        .map(|ann| TextAnnotation {
            start: ann.start.saturating_sub(offset),
            end: ann.end - offset,
            kind: ann.kind.clone(),
        })
        .collect();
    (&text[marker_len..], Cow::Owned(adjusted))
}

/// An entry on the container stack, tracking what comrak node to append
/// children into.
struct ContainerEntry<'a> {
    node: &'a AstNode<'a>,
    kind: ContainerKind,
}

#[derive(Clone, Copy)]
enum ContainerKind {
    List,
    BlockQuote,
    Group,
}

/// Build a comrak AST from an `InternalDocument`.
///
/// The returned node is a `Document` root whose children mirror the document
/// body content.  Footnotes are appended after body elements.  Non-body
/// elements (headers, footers) are excluded.
pub(crate) fn build_comrak_ast<'a>(doc: &InternalDocument, arena: &'a comrak::Arena<'a>) -> &'a AstNode<'a> {
    let root = mk(arena, NodeValue::Document);
    let footnotes = FootnoteCollector::new(doc);
    let mut state = RenderState::default();
    let consolidated = consolidate_paragraphs(&doc.elements);

    tracing::debug!(
        total_elements = doc.elements.len(),
        consolidated_elements = consolidated.len(),
        tables = doc.tables.len(),
        images = doc.images.len(),
        "building comrak AST"
    );

    let mut container_stack: Vec<ContainerEntry<'a>> = Vec::new();

    /// Return the current parent node (top of container stack, or root).
    fn current_parent<'b, 'a>(root: &'b &'a AstNode<'a>, stack: &'b [ContainerEntry<'a>]) -> &'a AstNode<'a> {
        stack.last().map(|e| e.node).unwrap_or(*root)
    }

    for consolidated_elem in &consolidated {
        let orig_idx = consolidated_elem.original_index();
        let view = consolidated_elem.resolve(&doc.elements);

        let (elem_kind, elem_text, elem_annotations, elem_depth, _elem_anchor, elem_attributes, render_image_ocr) =
            match &view {
                ElementView::Ref(elem) => {
                    if !is_body_element(elem) {
                        continue;
                    }
                    if is_container_end(elem) {
                        handle_container_end(&elem.kind, &mut state);
                        match elem.kind {
                            ElementKind::ListEnd => pop_container(&mut container_stack, ContainerKind::List),
                            ElementKind::QuoteEnd => pop_container(&mut container_stack, ContainerKind::BlockQuote),
                            ElementKind::GroupEnd => pop_container(&mut container_stack, ContainerKind::Group),
                            _ => {}
                        }
                        continue;
                    }
                    state.pop_to_depth(elem.depth);
                    (
                        elem.kind,
                        elem.text.as_str(),
                        elem.annotations.as_slice(),
                        elem.depth,
                        elem.anchor.as_deref(),
                        elem.attributes.as_ref(),
                        elem.should_render_image_ocr(),
                    )
                }
                ElementView::Merged { text, annotations, .. } => {
                    (ElementKind::Paragraph, *text, *annotations, 0u16, None, None, true)
                }
            };

        let parent = current_parent(&root, &container_stack);

        let parent = if matches!(parent.data.borrow().value, NodeValue::List(..))
            && !matches!(elem_kind, ElementKind::ListItem { .. } | ElementKind::ListEnd)
        {
            let last_item = parent
                .children()
                .filter(|c| matches!(c.data.borrow().value, NodeValue::Item(..) | NodeValue::TaskItem(..)))
                .last();
            match last_item {
                Some(item) => item,
                None => {
                    let list_meta = match parent.data.borrow().value {
                        NodeValue::List(ref m) => *m,
                        // The outer `matches!(parent.data.borrow().value, NodeValue::List(..))`
                        // check above already confirmed `parent` holds `NodeValue::List`, and
                        // `parent` is not reassigned and its `data` is never mutated between
                        // that check and this match, so this arm cannot be reached. ~keep
                        _ => unreachable!("parent was already confirmed to be NodeValue::List above"),
                    };
                    let implicit_item = mk(arena, NodeValue::Item(list_meta));
                    parent.append(implicit_item);
                    implicit_item
                }
            }
        } else {
            parent
        };

        match elem_kind {
            ElementKind::Title => {
                let heading = mk(
                    arena,
                    NodeValue::Heading(NodeHeading {
                        level: 1,
                        setext: false,
                        closed: false,
                    }),
                );
                build_inlines(arena, heading, elem_text, elem_annotations);
                parent.append(heading);
            }

            ElementKind::Heading { level } => {
                let heading = mk(
                    arena,
                    NodeValue::Heading(NodeHeading {
                        level: level.clamp(MIN_HEADING_LEVEL, MAX_HEADING_LEVEL),
                        setext: false,
                        closed: false,
                    }),
                );
                build_inlines(arena, heading, elem_text, elem_annotations);
                parent.append(heading);
            }

            ElementKind::Paragraph => {
                if elem_text.is_empty() && elem_annotations.is_empty() {
                    tracing::trace!(index = orig_idx, "skipping empty paragraph");
                    continue;
                }
                let para = mk(arena, NodeValue::Paragraph);
                build_inlines(arena, para, elem_text, elem_annotations);
                parent.append(para);
            }

            ElementKind::ListItem { ordered } => {
                // comrak's CommonMark writer takes the marker style and the running
                // number from the *enclosing* `List` node alone -- an `Item`'s own
                // `NodeList` is not consulted -- and CommonMark cannot express a
                // lettered or parenthesized marker like "B." or "(a)" as a list
                // ordinal at all. So when the source label survived, the label is
                // written as leading text and the enclosing list is forced to
                // bulleted: emitting it as an ordered list would print a synthesized
                // "1." beside the real "B.", renumbering a document whose clauses are
                // cross-referenced by their printed label. A bullet defers to the
                // label instead of competing with it.
                let source_label =
                    list_item_source_label_from_attributes(elem_attributes).filter(|label| !label.is_empty());
                let numbered = ordered && source_label.is_none();
                let list_type = if numbered {
                    comrak::nodes::ListType::Ordered
                } else {
                    comrak::nodes::ListType::Bullet
                };
                let item_list = comrak::nodes::NodeList {
                    list_type,
                    bullet_char: b'-',
                    start: 1,
                    tight: true,
                    ..Default::default()
                };
                let item = mk(arena, NodeValue::Item(item_list));
                let item_para = mk(arena, NodeValue::Paragraph);
                if let Some(label) = source_label {
                    item_para.append(mk_text(arena, &format!("{label} ")));
                }
                let (item_text, item_annotations) = strip_redundant_list_marker(elem_text, elem_annotations);
                build_inlines(arena, item_para, item_text, item_annotations.as_ref());
                item.append(item_para);

                let list_parent = if matches!(parent.data.borrow().value, NodeValue::List(..)) {
                    // A run opened by `ListStart` is already numbered; a labelled item
                    // joining it has to demote the whole run, or comrak resumes counting
                    // around it.
                    if !numbered && let NodeValue::List(list) = &mut parent.data.borrow_mut().value {
                        list.list_type = comrak::nodes::ListType::Bullet;
                        list.bullet_char = b'-';
                    }
                    parent
                } else {
                    let implicit_list = mk(
                        arena,
                        NodeValue::List(comrak::nodes::NodeList {
                            list_type,
                            bullet_char: b'-',
                            start: 1,
                            tight: true,
                            ..Default::default()
                        }),
                    );
                    parent.append(implicit_list);
                    implicit_list
                };
                list_parent.append(item);
            }

            ElementKind::Code => {
                let lang = elem_attributes
                    .and_then(|attrs| attrs.get("language").map(|s| s.as_str()))
                    .unwrap_or("");
                let code_block = mk(
                    arena,
                    NodeValue::CodeBlock(Box::new(NodeCodeBlock {
                        fenced: true,
                        fence_char: b'`',
                        fence_length: 3,
                        fence_offset: 0,
                        info: lang.to_string(),
                        literal: elem_text.to_string(),
                        closed: true,
                    })),
                );
                parent.append(code_block);
            }

            ElementKind::Formula => {
                let math = mk(
                    arena,
                    NodeValue::Math(NodeMath {
                        dollar_math: true,
                        display_math: true,
                        literal: elem_text.to_string(),
                    }),
                );
                let para = mk(arena, NodeValue::Paragraph);
                para.append(math);
                parent.append(para);
            }

            ElementKind::Table { table_index } => {
                if let Some(table) = doc.tables.get(table_index as usize) {
                    if doc.table_anchors
                        && let Some(table_id) = table.table_id.as_deref()
                    {
                        let anchor = mk(arena, NodeValue::Paragraph);
                        anchor.append(mk_text(arena, &format!("[TABLE:{table_id}]")));
                        parent.append(anchor);
                    }
                    let has_header =
                        !matches!(doc.source_format.as_str(), "csv" | "orgmode" | "typst") || table.columns.is_some();
                    match build_table(arena, &table.cells, has_header) {
                        Some(table_node) => {
                            tracing::trace!(table_index, rows = table.cells.len(), "rendering table");
                            parent.append(table_node);
                        }
                        None if !table.markdown.trim().is_empty() => {
                            let para = mk(arena, NodeValue::Paragraph);
                            para.append(mk_text(arena, &table.markdown));
                            parent.append(para);
                        }
                        None => {}
                    }
                }
            }

            ElementKind::Image { image_index } => {
                let image = doc.images.get(image_index as usize);
                let desc = image.and_then(|img| img.description.as_deref()).unwrap_or("");
                let url = match image {
                    None => {
                        if desc.is_empty() {
                            continue;
                        }
                        String::new()
                    }
                    Some(img) => {
                        if !img.data.is_empty() {
                            format!("image_{}.{}", image_index, img.format)
                        } else if let Some(ref path) = img.source_path {
                            path.clone()
                        } else {
                            format!("image_{}.bin", image_index)
                        }
                    }
                };

                let has_ocr = render_image_ocr
                    && image
                        .and_then(|img| img.ocr_result.as_ref())
                        .is_some_and(|result| !result.content.is_empty());

                if doc.ocr_text_only && has_ocr {
                    let ocr_result = image.and_then(|img| img.ocr_result.as_ref()).unwrap();
                    let ocr_para = mk(arena, NodeValue::Paragraph);
                    ocr_para.append(mk_text(arena, &ocr_result.content));
                    parent.append(ocr_para);
                } else {
                    let para = mk(arena, NodeValue::Paragraph);
                    let img_node = mk(
                        arena,
                        NodeValue::Image(Box::new(NodeLink {
                            url,
                            title: String::new(),
                        })),
                    );
                    img_node.append(mk_text(arena, desc));
                    para.append(img_node);
                    parent.append(para);

                    if render_image_ocr
                        && doc.append_ocr_text
                        && let Some(ocr_result) = image.and_then(|img| img.ocr_result.as_ref())
                        && !ocr_result.content.is_empty()
                    {
                        let ocr_para = mk(arena, NodeValue::Paragraph);
                        ocr_para.append(mk_text(arena, &ocr_result.content));
                        parent.append(ocr_para);
                    }
                }
            }

            ElementKind::FootnoteRef => {
                if let Some(n) = orig_idx.and_then(|idx| footnotes.ref_number(idx as u32)) {
                    let label = n.to_string();
                    let fnref = mk(
                        arena,
                        NodeValue::FootnoteReference(Box::new(NodeFootnoteReference {
                            name: label.clone(),
                            texts: Vec::new(),
                            ref_num: n,
                            ix: n,
                        })),
                    );
                    // A reference follows the block that cites it, so it joins that block when
                    // the block holds inline text.
                    let inline_parent = if let Some(last) = parent.last_child() {
                        if matches!(last.data.borrow().value, NodeValue::Paragraph | NodeValue::Heading(_)) {
                            last
                        } else {
                            let p = mk(arena, NodeValue::Paragraph);
                            parent.append(p);
                            p
                        }
                    } else {
                        let p = mk(arena, NodeValue::Paragraph);
                        parent.append(p);
                        p
                    };
                    inline_parent.append(fnref);
                }
            }

            ElementKind::FootnoteDefinition => {}

            ElementKind::CommentRef => {}

            ElementKind::CommentDefinition => {}

            ElementKind::Citation => {}

            ElementKind::PageBreak => {}

            ElementKind::Slide { .. } => {
                parent.append(mk(arena, NodeValue::ThematicBreak));
                if !elem_text.is_empty() {
                    let heading = mk(
                        arena,
                        NodeValue::Heading(NodeHeading {
                            level: 2,
                            setext: false,
                            closed: false,
                        }),
                    );
                    build_inlines(arena, heading, elem_text, elem_annotations);
                    parent.append(heading);
                }
            }

            ElementKind::DefinitionTerm => {
                let dt = mk(arena, NodeValue::Paragraph);
                build_inlines(arena, dt, elem_text, elem_annotations);
                parent.append(dt);
            }

            ElementKind::DefinitionDescription => {
                let dd = mk(arena, NodeValue::Paragraph);
                let prefix = format!(": {}", elem_text);
                build_inlines(arena, dd, &prefix, &[]);
                parent.append(dd);
            }

            ElementKind::Admonition => {
                let kind_str = elem_attributes
                    .and_then(|attrs| attrs.get("kind").map(|s| s.as_str()))
                    .unwrap_or("note");
                let title = elem_attributes.and_then(|attrs| attrs.get("title").map(|s| s.as_str()));

                let alert_type = match kind_str.to_lowercase().as_str() {
                    "note" => Some(comrak::nodes::AlertType::Note),
                    "tip" | "hint" => Some(comrak::nodes::AlertType::Tip),
                    "important" => Some(comrak::nodes::AlertType::Important),
                    "warning" | "warn" => Some(comrak::nodes::AlertType::Warning),
                    "caution" | "danger" | "error" => Some(comrak::nodes::AlertType::Caution),
                    _ => None,
                };

                if let Some(at) = alert_type {
                    let alert = mk(
                        arena,
                        NodeValue::Alert(Box::new(NodeAlert {
                            alert_type: at,
                            title: title.map(|s| s.to_string()),
                            multiline: false,
                            fence_length: 0,
                            fence_offset: 0,
                        })),
                    );
                    if !elem_text.is_empty() {
                        let para = mk(arena, NodeValue::Paragraph);
                        build_inlines(arena, para, elem_text, elem_annotations);
                        alert.append(para);
                    }
                    parent.append(alert);
                } else {
                    let bq = mk(arena, NodeValue::BlockQuote);
                    let title_display = title.unwrap_or(kind_str);
                    let title_para = mk(arena, NodeValue::Paragraph);
                    let strong = mk(arena, NodeValue::Strong);
                    strong.append(mk_text(arena, title_display));
                    title_para.append(strong);
                    bq.append(title_para);

                    if !elem_text.is_empty() {
                        let body_para = mk(arena, NodeValue::Paragraph);
                        build_inlines(arena, body_para, elem_text, elem_annotations);
                        bq.append(body_para);
                    }
                    parent.append(bq);
                }
            }

            ElementKind::RawBlock => {
                // A raw block is written as it is. Consecutive ones form one run, one per line
                // (pulldown-cmark reports an HTML block line by line), and a blank line ends the
                // run, so the next block starts a line of its own instead of being glued onto
                // the raw text (`</div># Heading`). ~keep
                let text = elem_text.trim_end_matches('\n');
                let merged = parent.last_child().is_some_and(|last| {
                    if let NodeValue::Raw(previous) = &mut last.data.borrow_mut().value {
                        *previous = format!("{}\n{text}\n\n", previous.trim_end_matches('\n'));
                        true
                    } else {
                        false
                    }
                });
                if !merged {
                    parent.append(mk(arena, NodeValue::Raw(format!("{text}\n\n"))));
                }
            }

            ElementKind::MetadataBlock => {
                let entries = parse_metadata_entries(elem_text);
                if !entries.is_empty() {
                    for (key, value) in &entries {
                        let para = mk(arena, NodeValue::Paragraph);
                        let strong = mk(arena, NodeValue::Strong);
                        strong.append(mk_text(arena, key));
                        para.append(strong);
                        para.append(mk_text(arena, &format!(": {}", value)));
                        parent.append(para);
                    }
                } else if !elem_text.is_empty() {
                    let para = mk(arena, NodeValue::Paragraph);
                    para.append(mk_text(arena, elem_text));
                    parent.append(para);
                }
            }

            ElementKind::OcrText { .. } => {
                if !elem_text.is_empty() {
                    let para = mk(arena, NodeValue::Paragraph);
                    build_inlines(arena, para, elem_text, elem_annotations);
                    parent.append(para);
                }
            }

            ElementKind::ListStart { ordered } => {
                state.push_container(NestingKind::List { ordered, item_count: 0 }, elem_depth);

                let list_meta = NodeList {
                    list_type: if ordered {
                        comrak::nodes::ListType::Ordered
                    } else {
                        comrak::nodes::ListType::Bullet
                    },
                    bullet_char: b'-',
                    start: 1,
                    tight: true,
                    ..Default::default()
                };
                let list_node = mk(arena, NodeValue::List(list_meta));

                let target = if matches!(parent.data.borrow().value, NodeValue::List(..)) {
                    let last_item = parent
                        .children()
                        .filter(|c| matches!(c.data.borrow().value, NodeValue::Item(..) | NodeValue::TaskItem(..)))
                        .last();
                    match last_item {
                        Some(item) => item,
                        None => {
                            let item = mk(arena, NodeValue::Item(list_meta));
                            parent.append(item);
                            item
                        }
                    }
                } else {
                    parent
                };
                target.append(list_node);

                container_stack.push(ContainerEntry {
                    node: list_node,
                    kind: ContainerKind::List,
                });
            }

            ElementKind::ListEnd => {}

            ElementKind::QuoteStart => {
                state.push_container(NestingKind::BlockQuote, elem_depth);
                let bq = mk(arena, NodeValue::BlockQuote);
                parent.append(bq);
                container_stack.push(ContainerEntry {
                    node: bq,
                    kind: ContainerKind::BlockQuote,
                });
            }

            ElementKind::QuoteEnd => {}

            ElementKind::GroupStart => {
                state.push_container(NestingKind::Group, elem_depth);
                container_stack.push(ContainerEntry {
                    node: parent,
                    kind: ContainerKind::Group,
                });
            }

            ElementKind::GroupEnd => {}
        }
    }

    let defs = footnotes.definitions();
    for entry in defs {
        let label = entry.number.to_string();
        let fndef = mk(
            arena,
            NodeValue::FootnoteDefinition(NodeFootnoteDefinition {
                name: label,
                total_references: 1,
            }),
        );
        let para = mk(arena, NodeValue::Paragraph);
        para.append(mk_text(arena, &entry.text));
        fndef.append(para);
        root.append(fndef);
    }

    for elem in &doc.elements {
        if elem.kind == ElementKind::Citation {
            let key = elem.anchor.as_deref().unwrap_or("?");
            let fndef = mk(
                arena,
                NodeValue::FootnoteDefinition(NodeFootnoteDefinition {
                    name: key.to_string(),
                    total_references: 1,
                }),
            );
            let para = mk(arena, NodeValue::Paragraph);
            para.append(mk_text(arena, &elem.text));
            fndef.append(para);
            root.append(fndef);
        }
    }

    // Comment definitions (#300) aren't tracked by `FootnoteCollector`, so surface
    // them the same way `Citation` is above — keyed by anchor — instead of
    // silently dropping the comment body now that it has its own `ElementKind`.
    for elem in &doc.elements {
        if elem.kind == ElementKind::CommentDefinition {
            let key = elem.anchor.as_deref().unwrap_or("?");
            let fndef = mk(
                arena,
                NodeValue::FootnoteDefinition(NodeFootnoteDefinition {
                    name: key.to_string(),
                    total_references: 1,
                }),
            );
            let para = mk(arena, NodeValue::Paragraph);
            para.append(mk_text(arena, &elem.text));
            fndef.append(para);
            root.append(fndef);
        }
    }

    #[cfg(debug_assertions)]
    if let Err(e) = root.validate() {
        tracing::warn!(?e, "comrak AST validation failed — output may be malformed");
    }

    root
}

/// Pop the innermost container matching the given kind from the stack.
fn pop_container(stack: &mut Vec<ContainerEntry<'_>>, target: ContainerKind) {
    for i in (0..stack.len()).rev() {
        if matches!(
            (&stack[i].kind, &target),
            (ContainerKind::List, ContainerKind::List)
                | (ContainerKind::BlockQuote, ContainerKind::BlockQuote)
                | (ContainerKind::Group, ContainerKind::Group)
        ) {
            stack.remove(i);
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::document_structure::{AnnotationKind, ContentLayer, TextAnnotation};
    use crate::types::internal_builder::InternalDocumentBuilder;
    use comrak::{Options, format_commonmark};

    /// Helper: build AST from doc and render to CommonMark string.
    fn render(doc: &InternalDocument) -> String {
        let arena = comrak::Arena::new();
        let root = build_comrak_ast(doc, &arena);
        let mut output = String::new();
        format_commonmark(root, &Options::default(), &mut output).unwrap();
        output
    }

    #[test]
    fn test_title() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_title("My Document", None, None);
        let doc = b.build();
        let out = render(&doc);
        assert!(out.contains("# My Document"), "got: {}", out);
    }

    #[test]
    fn test_heading_levels() {
        for level in 1u8..=6 {
            let mut b = InternalDocumentBuilder::new("test");
            b.push_heading(level, "Heading", None, None);
            let doc = b.build();
            let out = render(&doc);
            let hashes = "#".repeat(level as usize);
            assert!(
                out.contains(&format!("{} Heading", hashes)),
                "level {}: got {}",
                level,
                out
            );
        }
    }

    #[test]
    fn test_paragraph() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph("Hello world.", vec![], None, None);
        let doc = b.build();
        let out = render(&doc);
        assert!(out.contains("Hello world."), "got: {}", out);
    }

    #[test]
    fn test_empty_paragraph_skipped() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph("", vec![], None, None);
        let doc = b.build();
        let out = render(&doc);
        assert!(out.trim().is_empty(), "expected empty, got: {}", out);
    }

    #[test]
    fn test_bold_annotation() {
        let mut b = InternalDocumentBuilder::new("test");
        let ann = vec![TextAnnotation {
            start: 0,
            end: 5,
            kind: AnnotationKind::Bold,
        }];
        b.push_paragraph("Hello world", ann, None, None);
        let doc = b.build();
        let out = render(&doc);
        assert!(out.contains("**Hello**"), "got: {}", out);
        assert!(out.contains("world"), "got: {}", out);
    }

    #[test]
    fn test_italic_annotation() {
        let mut b = InternalDocumentBuilder::new("test");
        let ann = vec![TextAnnotation {
            start: 0,
            end: 5,
            kind: AnnotationKind::Italic,
        }];
        b.push_paragraph("Hello world", ann, None, None);
        let doc = b.build();
        let out = render(&doc);
        assert!(out.contains("*Hello*"), "got: {}", out);
    }

    #[test]
    fn test_code_block() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_code("fn main() {}", Some("rust"), None, None);
        let doc = b.build();
        let out = render(&doc);
        assert!(out.contains("```rust"), "got: {}", out);
        assert!(out.contains("fn main() {}"), "got: {}", out);
    }

    #[test]
    fn test_table() {
        let mut b = InternalDocumentBuilder::new("test");
        let cells = vec![
            vec!["Name".to_string(), "Age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
        ];
        b.push_table_from_cells(&cells, None, None);
        let doc = b.build();
        let out = render(&doc);
        assert!(out.contains("Name"), "got: {}", out);
        assert!(out.contains("Alice"), "got: {}", out);
    }

    #[test]
    fn test_list_items() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_list(false);
        b.push_list_item("Alpha", false, vec![], None, None);
        b.push_list_item("Beta", false, vec![], None, None);
        b.end_list();
        let doc = b.build();
        let out = render(&doc);
        assert!(out.contains("Alpha"), "got: {}", out);
        assert!(out.contains("Beta"), "got: {}", out);
    }

    #[test]
    fn test_blockquote() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_quote_start();
        b.push_paragraph("Quoted text.", vec![], None, None);
        b.push_quote_end();
        let doc = b.build();
        let out = render(&doc);
        assert!(out.contains("> Quoted text."), "got: {}", out);
    }

    #[test]
    fn test_paragraph_consolidation() {
        let mut b = InternalDocumentBuilder::new("test");
        let ann1 = vec![TextAnnotation {
            start: 0,
            end: 5,
            kind: AnnotationKind::Italic,
        }];
        let ann2 = vec![TextAnnotation {
            start: 0,
            end: 5,
            kind: AnnotationKind::Italic,
        }];
        b.push_paragraph("Hello", ann1, None, None);
        b.push_paragraph("World", ann2, None, None);
        let doc = b.build();
        let out = render(&doc);
        assert!(out.contains("*Hello World*"), "got: {}", out);
    }

    #[test]
    fn test_annotation_on_multibyte_char_boundary() {
        let mut b = InternalDocumentBuilder::new("test");
        let ann = vec![TextAnnotation {
            start: 1,
            end: 11,
            kind: AnnotationKind::Bold,
        }];
        b.push_paragraph("«ярко»", ann, None, None);
        let doc = b.build();
        let out = render(&doc);
        assert!(out.contains("ярко"), "Cyrillic content should be present, got: {}", out);
    }

    #[test]
    fn test_annotation_on_valid_multibyte_boundaries() {
        let mut b = InternalDocumentBuilder::new("test");
        let ann = vec![TextAnnotation {
            start: 0,
            end: 12,
            kind: AnnotationKind::Bold,
        }];
        b.push_paragraph("Привет мир", ann, None, None);
        let doc = b.build();
        let out = render(&doc);
        assert!(out.contains("**Привет**"), "got: {}", out);
        assert!(out.contains("мир"), "got: {}", out);
    }

    #[test]
    fn test_footnote() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph("See note", vec![], None, None);
        let _ref_idx = b.push_footnote_ref("1", "fn1", None);
        let def_idx = b.push_footnote_definition("This is the footnote text.", "fn1", None);
        b.set_layer(def_idx, ContentLayer::Footnote);
        let doc = b.build();
        let out = render(&doc);
        assert!(out.contains("footnote"), "should contain footnote marker, got: {}", out);
    }

    #[test]
    fn test_footnote_ref_joins_the_heading_it_follows() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_heading(1, "Title", None, None);
        b.push_footnote_ref("1", "fn1", None);
        b.push_paragraph("Body", vec![], None, None);
        let def_idx = b.push_footnote_definition("A note.", "fn1", None);
        b.set_layer(def_idx, ContentLayer::Footnote);
        let out = render(&b.build());
        assert!(out.starts_with("# Title[^1]\n\nBody\n"), "got: {}", out);
    }

    /// Regression test for issue #762: image links must appear in markdown when image
    /// data is available via xberg_native_pdf extraction (non-empty `data` field).
    #[test]
    fn test_image_with_data_renders_link() {
        use crate::types::ExtractedImage;
        use crate::types::internal::ElementKind;

        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph("Before image.", vec![], None, None);
        b.push_element(crate::types::internal::InternalElement::text(
            ElementKind::Image { image_index: 0 },
            "",
            0,
        ));
        let mut doc = b.build();
        doc.images.push(ExtractedImage {
            data: bytes::Bytes::from_static(b"\x89PNG"),
            format: std::borrow::Cow::Borrowed("png"),
            image_index: 0,
            page_number: Some(1),
            width: Some(100),
            height: Some(100),
            colorspace: None,
            bits_per_component: None,
            is_mask: false,
            description: None,
            ocr_result: None,
            bounding_box: None,
            source_path: None,
            image_kind: None,
            kind_confidence: None,
            cluster_id: None,
            caption: None,
            qr_codes: None,
            data_base64: None,
        });
        let out = render(&doc);
        assert!(out.contains("image_0.png"), "image link must appear; got: {}", out);
        assert!(out.contains("!["), "must use image markdown syntax; got: {}", out);
    }

    /// Regression test for issue #762: image with no data but known to exist (placeholder)
    /// must still emit a link with a `.bin` fallback URL rather than being silently dropped.
    #[test]
    fn test_image_placeholder_with_empty_data_renders_fallback_link() {
        use crate::types::ExtractedImage;
        use crate::types::internal::ElementKind;

        let mut b = InternalDocumentBuilder::new("test");
        b.push_element(crate::types::internal::InternalElement::text(
            ElementKind::Image { image_index: 0 },
            "",
            0,
        ));
        let mut doc = b.build();
        doc.images.push(ExtractedImage {
            data: bytes::Bytes::new(),
            format: std::borrow::Cow::Borrowed("unknown"),
            image_index: 0,
            page_number: Some(1),
            width: None,
            height: None,
            colorspace: None,
            bits_per_component: None,
            is_mask: false,
            description: None,
            ocr_result: None,
            bounding_box: None,
            source_path: None,
            image_kind: None,
            kind_confidence: None,
            cluster_id: None,
            caption: None,
            qr_codes: None,
            data_base64: None,
        });
        let out = render(&doc);
        assert!(
            out.contains("image_0.bin"),
            "empty-data image must emit placeholder link; got: {:?}",
            out
        );
    }

    /// Image element with out-of-bounds index and no description must be silently dropped.
    #[test]
    fn test_image_out_of_bounds_index_dropped() {
        use crate::types::internal::ElementKind;

        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph("Only text.", vec![], None, None);
        b.push_element(crate::types::internal::InternalElement::text(
            ElementKind::Image { image_index: 99 },
            "",
            0,
        ));
        let doc = b.build();
        let out = render(&doc);
        assert!(
            !out.contains("image_99"),
            "out-of-bounds image must be dropped; got: {}",
            out
        );
        assert!(out.contains("Only text."), "paragraph must still render; got: {}", out);
    }

    #[test]
    fn test_heading_directly_under_list_does_not_panic() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_list(false);
        b.push_heading(3, "1 (7)", None, None);
        b.end_list();
        let doc = b.build();
        let out = render(&doc);
        assert!(out.contains("1 (7)"), "heading text must appear in output; got: {out}");
    }

    #[test]
    fn test_paragraph_directly_under_list_does_not_panic() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_list(false);
        b.push_paragraph("Orphan paragraph.", vec![], None, None);
        b.end_list();
        let doc = b.build();
        let out = render(&doc);
        assert!(
            out.contains("Orphan paragraph."),
            "paragraph text must appear in output; got: {out}"
        );
    }

    #[test]
    fn test_heading_before_first_list_item_then_real_item() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_list(false);
        b.push_heading(3, "Stray label", None, None);
        b.push_list_item("Real item", false, vec![], None, None);
        b.end_list();
        let doc = b.build();
        let out = render(&doc);
        assert!(out.contains("Stray label"), "stray heading must appear; got: {out}");
        assert!(out.contains("Real item"), "real list item must appear; got: {out}");
    }

    #[test]
    fn test_ordered_list_with_stray_heading_does_not_panic() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_list(true);
        b.push_heading(3, "Stray ordered label", None, None);
        b.push_list_item("First ordered item", true, vec![], None, None);
        b.end_list();
        let doc = b.build();
        let out = render(&doc);
        assert!(
            out.contains("Stray ordered label"),
            "stray heading must appear; got: {out}"
        );
        assert!(
            out.contains("First ordered item"),
            "ordered item must appear; got: {out}"
        );
    }

    #[test]
    fn test_link_annotation() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph(
            "click here",
            vec![TextAnnotation {
                start: 0,
                end: 10,
                kind: AnnotationKind::Link {
                    url: "https://example.com".to_string(),
                    title: None,
                },
            }],
            None,
            None,
        );
        let doc = b.build();
        let out = render(&doc);
        assert!(
            out.contains("[click here](https://example.com)"),
            "plain link must render; got: {out}"
        );
    }

    /// Regression for #1086: bold+hyperlink on the same byte range must produce
    /// `[**text**](url)`, not `**text**` with the link silently dropped.
    #[test]
    fn test_bold_link_same_range() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph(
            "click",
            vec![
                TextAnnotation {
                    start: 0,
                    end: 5,
                    kind: AnnotationKind::Bold,
                },
                TextAnnotation {
                    start: 0,
                    end: 5,
                    kind: AnnotationKind::Link {
                        url: "https://example.com".to_string(),
                        title: None,
                    },
                },
            ],
            None,
            None,
        );
        let doc = b.build();
        let out = render(&doc);
        assert!(
            out.contains("[**click**](https://example.com)"),
            "bold link must render as [**text**](url); got: {out}"
        );
    }

    /// Italic + hyperlink same range → `[*text*](url)`.
    #[test]
    fn test_italic_link_same_range() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph(
            "click",
            vec![
                TextAnnotation {
                    start: 0,
                    end: 5,
                    kind: AnnotationKind::Italic,
                },
                TextAnnotation {
                    start: 0,
                    end: 5,
                    kind: AnnotationKind::Link {
                        url: "https://example.com".to_string(),
                        title: None,
                    },
                },
            ],
            None,
            None,
        );
        let doc = b.build();
        let out = render(&doc);
        assert!(
            out.contains("[*click*](https://example.com)"),
            "italic link must render as [*text*](url); got: {out}"
        );
    }

    /// Bold + italic + hyperlink same range → `[***text***](url)`.
    #[test]
    fn test_bold_italic_link_same_range() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph(
            "click",
            vec![
                TextAnnotation {
                    start: 0,
                    end: 5,
                    kind: AnnotationKind::Bold,
                },
                TextAnnotation {
                    start: 0,
                    end: 5,
                    kind: AnnotationKind::Italic,
                },
                TextAnnotation {
                    start: 0,
                    end: 5,
                    kind: AnnotationKind::Link {
                        url: "https://example.com".to_string(),
                        title: None,
                    },
                },
            ],
            None,
            None,
        );
        let doc = b.build();
        let out = render(&doc);
        assert!(
            out.contains("[***click***](https://example.com)"),
            "bold italic link must render as [***text***](url); got: {out}"
        );
    }

    /// Formatting annotation contained within (but shorter than) the link span.
    /// "Visit **docs** here" where the link covers the whole phrase.
    #[test]
    fn test_bold_contained_within_link_span() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph(
            "Visit docs here",
            vec![
                TextAnnotation {
                    start: 6,
                    end: 10,
                    kind: AnnotationKind::Bold,
                },
                TextAnnotation {
                    start: 0,
                    end: 15,
                    kind: AnnotationKind::Link {
                        url: "https://example.com/docs".to_string(),
                        title: None,
                    },
                },
            ],
            None,
            None,
        );
        let doc = b.build();
        let out = render(&doc);
        assert!(
            out.contains("[Visit **docs** here](https://example.com/docs)"),
            "bold contained within link must nest; got: {out}"
        );
    }

    /// Partial overlap: Link[0,7] + Bold[5,10] on "Hello world".
    ///
    /// The Bold annotation starts inside the Link span but ends outside it.
    /// There is no unambiguous nesting semantics for this case, so the Bold is
    /// intentionally dropped. This test documents that behavior so a future
    /// refactor does not accidentally recurse infinitely trying to nest them.
    #[test]
    fn test_partial_overlap_annotation_is_dropped() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph(
            "Hello world",
            vec![
                TextAnnotation {
                    start: 0,
                    end: 7,
                    kind: AnnotationKind::Link {
                        url: "https://example.com".to_string(),
                        title: None,
                    },
                },
                TextAnnotation {
                    start: 6,
                    end: 11,
                    kind: AnnotationKind::Bold,
                },
            ],
            None,
            None,
        );
        let doc = b.build();
        let out = render(&doc);
        assert!(
            out.contains("[Hello w](https://example.com)"),
            "partial-overlap Bold must be dropped; got: {out}"
        );
        assert!(
            out.contains("orld"),
            "text after partial overlap must still render; got: {out}"
        );
        assert!(
            !out.contains("**"),
            "no bold markers expected for partial overlap; got: {out}"
        );
    }

    /// Inline code contained within a link span: `[\`code\`](url)`.
    ///
    /// Code spans are CommonMark literals — formatting cannot nest inside them —
    /// so inner_for_node is ignored for `AnnotationKind::Code`. This test verifies
    /// that code-within-link still renders correctly despite that exception.
    #[test]
    fn test_code_within_link() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph(
            "code",
            vec![
                TextAnnotation {
                    start: 0,
                    end: 4,
                    kind: AnnotationKind::Link {
                        url: "https://example.com".to_string(),
                        title: None,
                    },
                },
                TextAnnotation {
                    start: 0,
                    end: 4,
                    kind: AnnotationKind::Code,
                },
            ],
            None,
            None,
        );
        let doc = b.build();
        let out = render(&doc);
        assert!(
            out.contains("[`code`](https://example.com)"),
            "code within link must render as [`code`](url); got: {out}"
        );
    }

    /// Strikethrough contained within a link span.
    ///
    /// `append_annotated_span` now recurses via `build_inlines` for Strikethrough.
    /// Verify this produces `[~~text~~](url)` rather than dropping the strikethrough.
    #[test]
    fn test_strikethrough_within_link() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph(
            "dead",
            vec![
                TextAnnotation {
                    start: 0,
                    end: 4,
                    kind: AnnotationKind::Link {
                        url: "https://example.com".to_string(),
                        title: None,
                    },
                },
                TextAnnotation {
                    start: 0,
                    end: 4,
                    kind: AnnotationKind::Strikethrough,
                },
            ],
            None,
            None,
        );
        let doc = b.build();
        let out = render(&doc);
        assert!(
            out.contains("[~~dead~~](https://example.com)"),
            "strikethrough within link must render as [~~text~~](url); got: {out}"
        );
    }

    #[test]
    fn test_non_overlapping_link_and_bold_unaffected() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph(
            "Hello world",
            vec![
                TextAnnotation {
                    start: 0,
                    end: 5,
                    kind: AnnotationKind::Bold,
                },
                TextAnnotation {
                    start: 6,
                    end: 11,
                    kind: AnnotationKind::Link {
                        url: "https://example.com".to_string(),
                        title: None,
                    },
                },
            ],
            None,
            None,
        );
        let doc = b.build();
        let out = render(&doc);
        assert!(out.contains("**Hello**"), "bold must still render; got: {out}");
        assert!(
            out.contains("[world](https://example.com)"),
            "link must still render; got: {out}"
        );
    }

    /// OCR-derived structure detection reads a numbered list marker as part of
    /// the line's own text, so `ListItem.text` still carries "1. " even though
    /// the element is correctly flagged `ElementKind::ListItem { ordered: true }`.
    /// Without stripping that redundant marker, comrak emits its own "1. "
    /// marker followed by the untouched label — which itself starts with
    /// "1." — and comrak's CommonMark writer backslash-escapes that second
    /// period (to keep it from being misread as a second list start on
    /// re-parse), producing "1. 1\. Land Use…" instead of a single clean
    /// "1. Land Use…" line. Fails without the fix in
    /// `strip_redundant_list_marker`: the line would contain the doubled
    /// "1. 1" marker and a stray "\." rather than the clean text.
    #[test]
    fn test_ordered_list_item_strips_embedded_numeral_marker() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_list(true);
        b.push_list_item("1. Land Use: Live/Work Townhomes", true, vec![], None, None);
        b.end_list();
        let doc = b.build();
        let out = render(&doc);
        assert!(
            out.lines().any(|l| l.trim() == "1. Land Use: Live/Work Townhomes"),
            "expected a single clean marker with no doubling or escaping; got: {out:?}"
        );
        assert!(!out.contains("1. 1"), "marker must not be doubled; got: {out:?}");
        assert!(
            !out.contains("1\\."),
            "period after the label's own leading digit must not be escaped; got: {out:?}"
        );
    }

    /// Mirrors [`test_ordered_list_item_strips_embedded_numeral_marker`] for
    /// unordered lists, where OCR reads the bullet glyph as literal text.
    /// Without the fix, comrak's own "- " marker precedes the untouched
    /// "- Bullet content" label, and the label's own leading "-" is itself
    /// backslash-escaped (line-leading dash), producing "- \- Bullet content".
    #[test]
    fn test_bullet_list_item_strips_embedded_dash_marker() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_list(false);
        b.push_list_item("- Bullet content", false, vec![], None, None);
        b.end_list();
        let doc = b.build();
        let out = render(&doc);
        assert!(
            out.lines().any(|l| l.trim() == "- Bullet content"),
            "expected a single clean bullet marker; got: {out:?}"
        );
        assert!(!out.contains("- -"), "marker must not be doubled; got: {out:?}");
        assert!(
            !out.contains("\\-"),
            "label's own leading dash must not be escaped; got: {out:?}"
        );
    }

    /// Verifies `strip_redundant_list_marker` shifts annotation byte offsets
    /// to match the shortened text rather than leaving them pointed at the
    /// now-removed marker prefix. "Land Use" is bold in the *original*
    /// (marker-inclusive) offsets; after the 3-byte "1. " marker is stripped,
    /// the same span must still land on "Land Use", not on "d Use:" (an
    /// unshifted span) or nothing (a dropped annotation).
    #[test]
    fn test_ordered_list_item_marker_strip_shifts_annotation_offsets() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_list(true);
        b.push_list_item(
            "1. Land Use: Live/Work Townhomes",
            true,
            vec![TextAnnotation {
                start: 3,
                end: 11,
                kind: AnnotationKind::Bold,
            }],
            None,
            None,
        );
        b.end_list();
        let doc = b.build();
        let out = render(&doc);
        assert!(
            out.lines().any(|l| l.trim() == "1. **Land Use**: Live/Work Townhomes"),
            "bold span must follow the stripped marker onto \"Land Use\"; got: {out:?}"
        );
    }

    /// Heading levels beyond ATX's 1-6 range are not valid CommonMark: more
    /// than six leading `#` characters are not recognized as a heading on
    /// re-parse. A structure-detection bug that ever emits a level like 9
    /// must not be passed straight through to comrak. Fails without the
    /// `.clamp(MIN_HEADING_LEVEL, MAX_HEADING_LEVEL)` fix: the line would
    /// read "######### Out of range" (9 hashes) instead of the clamped
    /// "###### Out of range" (6 hashes).
    #[test]
    fn test_heading_level_above_max_is_clamped() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_element(InternalElement::text(
            ElementKind::Heading { level: 9 },
            "Out of range",
            0,
        ));
        let doc = b.build();
        let out = render(&doc);
        assert!(
            out.lines().any(|l| l == "###### Out of range"),
            "level must be clamped to 6 hashes; got: {out:?}"
        );
        assert!(
            !out.contains("####### "),
            "no more than 6 hashes may appear; got: {out:?}"
        );
    }

    /// A level of 0 is likewise out of range — comrak still writes the
    /// marker's trailing space even for zero hashes, so unclamped it renders
    /// as a bare " Out of range" line: no `#` at all, and thus not a heading
    /// on re-parse. Fails without the clamp: no line equals "# Out of range".
    #[test]
    fn test_heading_level_zero_is_clamped() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_element(InternalElement::text(
            ElementKind::Heading { level: 0 },
            "Out of range",
            0,
        ));
        let doc = b.build();
        let out = render(&doc);
        assert!(
            out.lines().any(|l| l == "# Out of range"),
            "level must be clamped up to 1 hash; got: {out:?}"
        );
    }

    /// A table whose every row is entirely empty (`num_cols == 0`, e.g. a
    /// garbage table-region detection on an OCR page) must not reach comrak's
    /// table writer: with zero cells, `format_table_cell` — which is what
    /// emits the mandatory `|---|` delimiter row — never runs, so the "table"
    /// serializes to a single bare "|" line. That is not valid GFM (no
    /// delimiter row) and does not round-trip as a table. Fails without the
    /// `num_cols == 0` guard in `build_table`: output would contain "|".
    #[test]
    fn test_table_with_all_empty_rows_emits_no_malformed_pipe() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph("Before degenerate table.", vec![], None, None);
        b.push_table_from_cells(&[vec![]], None, None);
        let doc = b.build();
        let out = render(&doc);
        assert!(
            !out.contains('|'),
            "a columnless table must not leak a bare, unmatched pipe; got: {out:?}"
        );
        assert!(
            out.contains("Before degenerate table."),
            "surrounding content must still render; got: {out:?}"
        );
    }

    fn bold_word_doc(words: usize) -> InternalDocument {
        let text = "word ".repeat(words);
        let annotations = (0..words as u32)
            .map(|i| TextAnnotation {
                start: i * 5,
                end: i * 5 + 4,
                kind: AnnotationKind::Bold,
            })
            .collect();
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph(&text, annotations, None, None);
        b.build()
    }

    #[test]
    fn test_many_disjoint_annotations_render_each_span() {
        let out = render(&bold_word_doc(50));
        assert_eq!(out.trim_end(), vec!["**word**"; 50].join(" "));
    }

    /// Annotations of one block are scanned once, not once per annotation (GH#2099). Quadratic
    /// growth makes the large input about 64x slower than the small one; linear growth about 8x.
    /// The floor keeps a fast small run from turning timer noise into a failure. ~keep
    #[test]
    fn test_render_time_scales_linearly_with_annotations_in_one_block() {
        const SMALL: usize = 4_000;
        const LARGE: usize = 32_000;
        let time = |words: usize| {
            let doc = bold_word_doc(words);
            let start = std::time::Instant::now();
            let out = render(&doc);
            assert!(out.starts_with("**word** **word**"));
            start.elapsed()
        };
        let small = time(SMALL).max(std::time::Duration::from_millis(20));
        let large = time(LARGE);
        assert!(
            large < small * 24,
            "{LARGE} annotations took {large:?} against {small:?} for {SMALL}: growth is superlinear"
        );
    }
    #[test]
    fn should_keep_issue_2123_annotated_math_latex_unescaped() {
        let text = r"From $5 to $10; $\alpha^{2}$ holds.";
        let start = text.find(r"$\alpha").unwrap() as u32;
        let end = start + r"$\alpha^{2}$".len() as u32;
        let mut builder = InternalDocumentBuilder::new("test");
        builder.push_paragraph(
            text,
            vec![TextAnnotation {
                start,
                end,
                kind: AnnotationKind::Custom {
                    name: "xberg.math".into(),
                    value: Some("inline".into()),
                },
            }],
            None,
            None,
        );
        let doc = builder.build();
        let arena = comrak::Arena::new();
        let ast = build_comrak_ast(&doc, &arena);
        let math: Vec<_> = ast
            .descendants()
            .filter_map(|node| {
                let data = node.data.borrow();
                if let NodeValue::Math(math) = &data.value {
                    Some((math.literal.clone(), math.display_math))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(math, vec![(r"\alpha^{2}".into(), false)]);
        assert_eq!(
            crate::rendering::render_markdown(&doc).trim(),
            r"From $5 to $10; $`\alpha^{2}`$ holds."
        );
    }

    #[test]
    fn should_preserve_issue_2123_math_under_bold_and_links_without_consolidation_loss() {
        let text = r"$\alpha$";
        let mut builder = InternalDocumentBuilder::new("test");
        for _ in 0..2 {
            builder.push_paragraph(
                text,
                vec![
                    TextAnnotation {
                        start: 0,
                        end: text.len() as u32,
                        kind: AnnotationKind::math(false),
                    },
                    TextAnnotation {
                        start: 0,
                        end: text.len() as u32,
                        kind: AnnotationKind::Bold,
                    },
                    TextAnnotation {
                        start: 0,
                        end: text.len() as u32,
                        kind: AnnotationKind::Link {
                            url: "https://example.com".into(),
                            title: None,
                        },
                    },
                ],
                None,
                None,
            );
        }
        let doc = builder.build();
        let arena = comrak::Arena::new();
        let ast = build_comrak_ast(&doc, &arena);
        let math: Vec<_> = ast
            .descendants()
            .filter(|node| matches!(node.data.borrow().value, NodeValue::Math(_)))
            .collect();
        assert_eq!(math.len(), 2);
        assert!(
            math.iter()
                .all(|node| matches!(node.parent().unwrap().data.borrow().value, NodeValue::Strong))
        );
        assert_eq!(crate::rendering::render_markdown(&doc).matches(r"\alpha").count(), 2);
        let html = crate::rendering::render_html(&doc);
        assert_eq!(html.matches("data-math-style=\"inline\"").count(), 2);
        assert_eq!(html.matches("<strong>").count(), 2);
        assert_eq!(html.matches("<a href=\"https://example.com\">").count(), 2);
    }

    #[test]
    fn should_keep_issue_2123_adjacent_equations_and_literal_whitespace() {
        let text = "$a  + b$$c\n+d$";
        let first = "$a  + b$".len() as u32;
        let mut builder = InternalDocumentBuilder::new("test");
        builder.push_paragraph(
            text,
            vec![
                TextAnnotation {
                    start: 0,
                    end: first,
                    kind: AnnotationKind::math(false),
                },
                TextAnnotation {
                    start: first,
                    end: text.len() as u32,
                    kind: AnnotationKind::math(false),
                },
            ],
            None,
            None,
        );
        let doc = builder.build();
        let arena = comrak::Arena::new();
        let ast = build_comrak_ast(&doc, &arena);
        let literals: Vec<_> = ast
            .descendants()
            .filter_map(|node| match &node.data.borrow().value {
                NodeValue::Math(math) => Some(math.literal.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(literals, ["a  + b", "c\n+d"]);
        assert_eq!(crate::rendering::render_plain(&doc).trim(), text);
    }

    #[test]
    fn should_reject_issue_2123_invalid_math_byte_ranges() {
        for (text, start, end) in [("$x$", 0, 99), ("$x$", 99, 100), ("$x$", 2, 1), ("$α$", 1, 2)] {
            let mut builder = InternalDocumentBuilder::new("test");
            builder.push_paragraph(
                text,
                vec![TextAnnotation {
                    start,
                    end,
                    kind: AnnotationKind::math(false),
                }],
                None,
                None,
            );
            let doc = builder.build();
            let arena = comrak::Arena::new();
            let ast = build_comrak_ast(&doc, &arena);
            assert_eq!(
                ast.descendants()
                    .filter(|node| matches!(node.data.borrow().value, NodeValue::Math(_)))
                    .count(),
                0,
                "invalid byte range {start}..{end} in {text}"
            );
        }
    }

    #[test]
    fn should_leave_issue_2123_unmarked_dollars_and_code_as_prose() {
        let text = r"From $5 to $10 and $x$; $HOME; 5$; $\alpha$";
        let mut builder = InternalDocumentBuilder::new("test");
        builder.push_paragraph(text, vec![], None, None);
        builder.push_paragraph(
            "$x$",
            vec![TextAnnotation {
                start: 0,
                end: 3,
                kind: AnnotationKind::Custom {
                    name: "other.math".into(),
                    value: Some("inline".into()),
                },
            }],
            None,
            None,
        );
        builder.push_paragraph(
            "$x$",
            vec![
                TextAnnotation {
                    start: 0,
                    end: 3,
                    kind: AnnotationKind::math(false),
                },
                TextAnnotation {
                    start: 0,
                    end: 3,
                    kind: AnnotationKind::Code,
                },
            ],
            None,
            None,
        );
        builder.push_table_from_cells(&[vec!["From $5 to $10 and $x$".into()]], None, None);
        let doc = builder.build();
        let arena = comrak::Arena::new();
        let ast = build_comrak_ast(&doc, &arena);
        assert_eq!(
            ast.descendants()
                .filter(|node| matches!(node.data.borrow().value, NodeValue::Math(_)))
                .count(),
            0
        );
        let html = crate::rendering::render_html(&doc);
        assert!(!html.contains("data-math-style"));
        assert!(html.contains("From $5 to $10 and $x$"));
        assert!(html.contains(r"$\alpha$"));
    }
    #[test]
    fn should_preserve_issue_2123_math_inside_transparent_annotations() {
        let mut cases = 0;
        for kind in [
            AnnotationKind::Color { value: "red".into() },
            AnnotationKind::FontSize { value: "12pt".into() },
            AnnotationKind::Custom {
                name: "other.annotation".into(),
                value: None,
            },
        ] {
            for (prefix, suffix) in [("", ""), ("π before ", " after")] {
                let math = r"$\alpha  + x$";
                let text = format!("{prefix}{math}{suffix}");
                let start = prefix.len() as u32;
                let end = start + math.len() as u32;
                let mut builder = InternalDocumentBuilder::new("test");
                builder.push_paragraph(
                    &text,
                    vec![
                        TextAnnotation {
                            start,
                            end,
                            kind: AnnotationKind::math(false),
                        },
                        TextAnnotation {
                            start: 0,
                            end: text.len() as u32,
                            kind: kind.clone(),
                        },
                        TextAnnotation {
                            start,
                            end,
                            kind: AnnotationKind::Bold,
                        },
                    ],
                    None,
                    None,
                );
                let doc = builder.build();
                let arena = comrak::Arena::new();
                let ast = build_comrak_ast(&doc, &arena);
                let nodes: Vec<_> = ast
                    .descendants()
                    .filter(|node| matches!(node.data.borrow().value, NodeValue::Math(_)))
                    .collect();
                assert_eq!(nodes.len(), 1, "transparent annotation {kind:?}: {text}");
                if let NodeValue::Math(value) = &nodes[0].data.borrow().value {
                    assert_eq!(value.literal, r"\alpha  + x");
                }
                assert!(matches!(
                    nodes[0].parent().unwrap().data.borrow().value,
                    NodeValue::Strong
                ));
                assert_eq!(
                    crate::rendering::render_markdown(&doc).trim(),
                    format!(r"{prefix}**$`\alpha  + x`$**{suffix}")
                );
                let html = crate::rendering::render_html(&doc);
                assert_eq!(html.matches("data-math-style=\"inline\"").count(), 1);
                assert_eq!(html.matches("<strong>").count(), 1);
                cases += 1;
            }
        }
        assert_eq!(cases, 6);
    }

    #[test]
    fn should_keep_issue_2123_math_free_transparent_annotation_behavior() {
        let mut builder = InternalDocumentBuilder::new("test");
        builder.push_paragraph(
            "plain",
            vec![
                TextAnnotation {
                    start: 0,
                    end: 5,
                    kind: AnnotationKind::Color { value: "red".into() },
                },
                TextAnnotation {
                    start: 0,
                    end: 5,
                    kind: AnnotationKind::Bold,
                },
            ],
            None,
            None,
        );
        assert_eq!(crate::rendering::render_markdown(&builder.build()).trim(), "plain");
    }
    #[test]
    fn should_flatten_issue_2123_transparent_annotation_chains() {
        let text = r"$\alpha  + x$";
        let mut annotations = vec![
            TextAnnotation {
                start: 0,
                end: text.len() as u32,
                kind: AnnotationKind::Color { value: "red".into() }
            };
            5000
        ];
        annotations.push(TextAnnotation {
            start: 0,
            end: text.len() as u32,
            kind: AnnotationKind::math(false),
        });
        let mut builder = InternalDocumentBuilder::new("test");
        builder.push_paragraph(text, annotations, None, None);
        let doc = builder.build();
        assert_eq!(crate::rendering::render_markdown(&doc).trim(), r"$`\alpha  + x`$");
        assert_eq!(
            crate::rendering::render_html(&doc)
                .matches("data-math-style=\"inline\"")
                .count(),
            1
        );
    }

    fn math_span(text: &str, span: &str) -> TextAnnotation {
        let start = text.find(span).unwrap() as u32;
        TextAnnotation {
            start,
            end: start + span.len() as u32,
            kind: AnnotationKind::math(span.starts_with("$$")),
        }
    }

    fn push_math_paragraph(builder: &mut InternalDocumentBuilder, text: &str, spans: &[&str]) {
        let annotations = spans.iter().map(|span| math_span(text, span)).collect();
        builder.push_paragraph(text, annotations, None, None);
    }

    /// The literal of every code span, code block and math node in `markdown`, failing on any
    /// raw HTML, link or image a reader would see.
    fn literals(markdown: &str, options: &Options) -> Vec<String> {
        let arena = comrak::Arena::new();
        comrak::parse_document(&arena, markdown, options)
            .descendants()
            .filter_map(|node| match &node.data.borrow().value {
                NodeValue::Code(code) => Some(code.literal.clone()),
                NodeValue::CodeBlock(block) => Some(block.literal.trim_end().to_string()),
                NodeValue::Math(math) => Some(math.literal.clone()),
                value @ (NodeValue::HtmlInline(_)
                | NodeValue::HtmlBlock(_)
                | NodeValue::Link(_)
                | NodeValue::Image(_)) => panic!("{value:?} in: {markdown}"),
                _ => None,
            })
            .collect()
    }

    /// Re-parse `markdown` as plain CommonMark, as xberg's own options read it, and with dollar
    /// math only, the way remark-math reads it.
    fn assert_literals(markdown: &str, expected: &[&str]) {
        assert_eq!(literals(markdown, &Options::default()), expected, "{markdown}");
        assert_eq!(
            literals(markdown, &crate::rendering::markdown::comrak_options()),
            expected,
            "{markdown}"
        );
        let mut dollars = Options::default();
        dollars.extension.math_dollars = true;
        assert_no_html(markdown, &dollars);
    }

    fn assert_no_html(markdown: &str, options: &Options) {
        literals(markdown, options);
    }

    fn code_annotation(text: &str, span: &str) -> TextAnnotation {
        let start = text.find(span).unwrap() as u32;
        TextAnnotation {
            start,
            end: start + span.len() as u32,
            kind: AnnotationKind::Code,
        }
    }

    #[test]
    fn should_write_math_as_code_math() {
        let mut builder = InternalDocumentBuilder::new("test");
        push_math_paragraph(&mut builder, r"Let $a<b$ and $$x^2$$ hold.", &["$a<b$", "$$x^2$$"]);
        push_math_paragraph(&mut builder, r"$$\sum_i x_i$$", &[r"$$\sum_i x_i$$"]);
        builder.push_formula("y = x", None, None);
        let markdown = crate::rendering::render_markdown(&builder.build());
        assert_eq!(
            markdown,
            "Let $`a<b`$ and $`x^2`$ hold.\n\n```math\n\\sum_i x_i\n```\n\n```math\ny = x\n```\n"
        );
    }

    #[test]
    fn should_keep_hostile_math_inside_code_spans_and_fences() {
        let mut builder = InternalDocumentBuilder::new("test");
        for tex in [
            "<img src=x onerror=alert(1)>",
            "<https://x>",
            "x\n\n<img src=x>",
            "a`$<img src=x>",
        ] {
            let span = format!("${tex}$");
            push_math_paragraph(&mut builder, &format!("See {span} here"), &[span.as_str()]);
        }
        builder.push_formula("<script>\nalert(1)\n</script>", None, None);
        builder.push_formula("a\n```\n<b>", None, None);
        let doc = builder.build();

        let markdown = crate::rendering::render_markdown(&doc);
        assert_literals(
            &markdown,
            &[
                "<img src=x onerror=alert(1)>",
                "<https://x>",
                "x  <img src=x>",
                "a`$<img src=x>",
                "<script>\nalert(1)\n</script>",
                "a\n```\n<b>",
            ],
        );
        assert!(markdown.contains("See ``a`$<img src=x>`` here"), "{markdown}");
        assert!(markdown.contains("````math\na\n```\n<b>\n````"), "{markdown}");

        let html = crate::rendering::render_html(&doc);
        assert!(
            html.contains(r#"<span data-math-style="inline">&lt;img src=x onerror=alert(1)&gt;</span>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<span data-math-style="display">&lt;script&gt;"#),
            "{html}"
        );
    }

    #[test]
    fn should_end_a_run_of_raw_blocks_before_the_next_block() {
        let mut builder = InternalDocumentBuilder::new("test");
        builder.push_paragraph("Intro.", vec![], None, None);
        builder.push_raw_block("html", "<div class=\"note\">", None);
        builder.push_raw_block("html", "Raw HTML block", None);
        builder.push_raw_block("html", "</div>", None);
        builder.push_heading(1, "Heading", None, None);
        builder.push_raw_block("jsx", "<Badge />", None);
        builder.push_list(false);
        builder.push_list_item("item", false, vec![], None, None);
        builder.end_list();

        let markdown = crate::rendering::render_markdown(&builder.build());
        assert_eq!(
            markdown,
            "Intro.\n\n<div class=\"note\">\nRaw HTML block\n</div>\n\n# Heading\n\n<Badge />\n\n- item\n"
        );
    }

    #[test]
    fn should_keep_line_breaks_from_ending_code() {
        let mut builder = InternalDocumentBuilder::new("test");
        builder.push_quote_start();
        builder.push_formula("x\r<img src=x onerror=alert(1)>", None, None);
        builder.push_code("a\r\n```\r<img src=x>", None, None, None);
        builder.push_quote_end();
        builder.push_code("b", Some("x`\r~~~\r<img src=x>"), None, None);
        builder.push_list(false);
        builder.push_list_item("item", false, vec![], None, None);
        builder.push_formula("y\r<img src=x>", None, None);
        builder.end_list();

        let markdown = crate::rendering::render_markdown(&builder.build());
        assert_literals(
            &markdown,
            &[
                "x\n<img src=x onerror=alert(1)>",
                "a\r\n```\n<img src=x>",
                "b",
                "y\n<img src=x>",
            ],
        );
    }

    #[test]
    fn should_write_prose_tabs_as_spaces_and_leave_code_alone() {
        let mut builder = InternalDocumentBuilder::new("test");
        let text = "Tab\there and `&#2;`<img src=x>";
        let annotation = code_annotation(text, "`&#2;`<img src=x>");
        builder.push_paragraph(text, vec![annotation], None, None);
        builder.push_code("``&#2;`\n<img src=x>", None, None, None);

        let markdown = crate::rendering::render_markdown(&builder.build());
        assert_literals(&markdown, &["`&#2;`<img src=x>", "``&#2;`\n<img src=x>"]);
        assert!(markdown.starts_with("Tab here and "), "{markdown}");
    }

    #[test]
    fn should_not_strip_an_arxiv_watermark_from_a_line_holding_code() {
        let mut builder = InternalDocumentBuilder::new("pdf");
        builder.push_paragraph("Notes.", vec![], None, None);
        builder.push_code("arXiv:2301.00001v1\n<img src=x>", None, None, None);
        let markdown = crate::rendering::render_markdown(&builder.build());
        assert_literals(&markdown, &["arXiv:2301.00001v1\n<img src=x>"]);

        let mut builder = InternalDocumentBuilder::new("pdf");
        builder.push_quote_start();
        builder.push_code("w1. w2 w3 w4 w5 w6 w7 w8 w9\narXiv:2301.00001v1", None, None, None);
        builder.push_code("<img src=x>", None, None, None);
        builder.push_quote_end();
        let markdown = crate::rendering::render_markdown(&builder.build());
        assert_literals(
            &markdown,
            &["w1. w2 w3 w4 w5 w6 w7 w8 w9\narXiv:2301.00001v1", "<img src=x>"],
        );
    }

    #[test]
    fn should_strip_an_arxiv_watermark_without_joining_the_next_block() {
        let watermark = "$5. arXiv:2301.00001v1";
        let mut builder = InternalDocumentBuilder::new("pdf");
        builder.push_paragraph(watermark, vec![], None, None);
        push_math_paragraph(&mut builder, "Let $x^2$ hold.", &["$x^2$"]);
        let markdown = crate::rendering::render_markdown(&builder.build());
        assert_eq!(markdown, "$5.\n\nLet $`x^2`$ hold.\n");

        let mut builder = InternalDocumentBuilder::new("pdf");
        builder.push_paragraph(watermark, vec![], None, None);
        builder.push_list(false);
        builder.push_list_item("item", false, vec![], None, None);
        let text = "b c$<img src=x onerror=alert(1)>";
        let annotation = code_annotation(text, "c$<img src=x onerror=alert(1)>");
        builder.push_paragraph(text, vec![annotation], None, None);
        builder.end_list();
        let markdown = crate::rendering::render_markdown(&builder.build());
        assert_literals(&markdown, &["c$<img src=x onerror=alert(1)>"]);
        assert!(markdown.starts_with("$5.\n\n- item"), "{markdown}");
    }

    #[test]
    fn should_escape_prose_dollars_beside_code_holding_a_dollar() {
        let mut builder = InternalDocumentBuilder::new("test");
        for (text, span) in [
            (
                "p $b$x`y$<img src=x onerror=alert(1)>$",
                "$x`y$<img src=x onerror=alert(1)>$",
            ),
            (
                "see $$q $a$$<img src=x onerror=alert(1)>$",
                "$a$$<img src=x onerror=alert(1)>$",
            ),
            (
                r"p $b\$x$<img src=x onerror=alert(1)>$",
                "$x$<img src=x onerror=alert(1)>$",
            ),
            ("Costs $5 and $x^2$.", "$x^2$"),
        ] {
            push_math_paragraph(&mut builder, text, &[span]);
        }
        let text = "$x$5 then a$<img src=x>";
        let annotations = vec![
            TextAnnotation {
                start: 0,
                end: 3,
                kind: AnnotationKind::math(false),
            },
            code_annotation(text, "a$<img src=x>"),
        ];
        builder.push_paragraph(text, annotations, None, None);
        builder.push_list(false);
        builder.push_list_item("Pay $a", false, vec![], None, None);
        let text = "b c$<img src=x onerror=alert(1)>";
        let annotation = code_annotation(text, "c$<img src=x onerror=alert(1)>");
        builder.push_paragraph(text, vec![annotation], None, None);
        builder.end_list();

        let markdown = crate::rendering::render_markdown(&builder.build());
        assert_literals(
            &markdown,
            &[
                "x`y$<img src=x onerror=alert(1)>",
                "a$$<img src=x onerror=alert(1)>",
                "x$<img src=x onerror=alert(1)>",
                "x^2",
                "x",
                "a$<img src=x>",
                "c$<img src=x onerror=alert(1)>",
            ],
        );
        assert!(markdown.contains("Costs $5 and $`x^2`$."), "{markdown}");
    }

    #[test]
    fn should_escape_each_prose_dollar_once_beside_many_code_spans() {
        let mut builder = InternalDocumentBuilder::new("test");
        let text = "a $1 x$ b $2 y$ c $3";
        let annotations = vec![code_annotation(text, "x$"), code_annotation(text, "y$")];
        builder.push_paragraph(text, annotations, None, None);
        let markdown = crate::rendering::render_markdown(&builder.build());
        assert_literals(&markdown, &["x$", "y$"]);
        assert!(markdown.contains(r"a \$1 `x$` b \$2 `y$` c \$3"), "{markdown}");
    }

    #[test]
    fn should_merge_adjacent_code_spans() {
        let mut builder = InternalDocumentBuilder::new("test");
        let text = "ac`<img src=x>";
        let annotations = vec![
            TextAnnotation {
                start: 0,
                end: 1,
                kind: AnnotationKind::Code,
            },
            code_annotation(text, "c`<img src=x>"),
        ];
        builder.push_paragraph(text, annotations, None, None);
        let text = "a$c`<img src=x>$";
        let mut annotations = vec![code_annotation(text, "a")];
        annotations.push(TextAnnotation {
            start: 1,
            end: text.len() as u32,
            kind: AnnotationKind::math(false),
        });
        builder.push_paragraph(text, annotations, None, None);
        // comrak writes a Strong nested in a Strong with no delimiter, in either nesting order.
        let text = "ac`<img src=x>";
        let span = |start: u32, end: usize, kind: AnnotationKind| TextAnnotation {
            start,
            end: end as u32,
            kind,
        };
        let annotations = vec![
            span(0, text.len(), AnnotationKind::Bold),
            span(0, 1, AnnotationKind::Code),
            span(1, text.len(), AnnotationKind::Bold),
            span(1, text.len(), AnnotationKind::Code),
        ];
        builder.push_paragraph(text, annotations, None, None);
        let annotations = vec![
            span(0, text.len(), AnnotationKind::Bold),
            span(0, 1, AnnotationKind::Bold),
            span(0, 1, AnnotationKind::Code),
            span(1, text.len(), AnnotationKind::Code),
        ];
        builder.push_paragraph(text, annotations, None, None);

        let markdown = crate::rendering::render_markdown(&builder.build());
        assert_literals(
            &markdown,
            &["ac`<img src=x>", "ac`<img src=x>", "ac`<img src=x>", "ac`<img src=x>"],
        );
    }

    #[test]
    fn should_drop_empty_math_and_code() {
        let mut builder = InternalDocumentBuilder::new("test");
        builder.push_paragraph("text", vec![], None, None);
        builder.push_formula(" ", None, None);
        push_math_paragraph(&mut builder, "a $ $ b", &["$ $"]);
        let text = "c \u{1} d";
        builder.push_paragraph(text, vec![code_annotation(text, "\u{1}")], None, None);
        assert_eq!(
            crate::rendering::render_markdown(&builder.build()),
            "text\n\na  b\n\nc  d\n"
        );
    }

    #[test]
    fn should_not_write_a_list_tight_when_an_empty_item_follows_a_paragraph() {
        // The empty item cannot interrupt the paragraph, so the lines would merge into it.
        let mut builder = InternalDocumentBuilder::new("test");
        builder.push_list(false);
        builder.push_list_item("$", false, vec![], None, None);
        builder.push_list(true);
        builder.push_list_item("", true, vec![], None, None);
        let text = "$$a<!-->$$";
        builder.push_list_item(text, true, vec![math_span(text, text)], None, None);
        builder.end_list();
        builder.end_list();
        assert_literals(&crate::rendering::render_markdown(&builder.build()), &["a<!-->"]);
    }

    #[test]
    fn should_not_write_a_paragraph_after_a_table_in_a_tight_item() {
        // It would be read as a row of the table.
        let mut builder = InternalDocumentBuilder::new("test");
        builder.push_list(false);
        builder.push_list_item("", false, vec![], None, None);
        builder.push_table_from_cells(&[vec![String::new(), String::new()]], None, None);
        let text = "$|$<x r=)>";
        let annotations = vec![math_span(text, "$|$"), code_annotation(text, "<x r=)>")];
        builder.push_paragraph(text, annotations, None, None);
        builder.end_list();
        assert_literals(&crate::rendering::render_markdown(&builder.build()), &["|", "<x r=)>"]);
    }

    #[test]
    fn should_keep_an_item_blank_led_text_from_moving_its_content_column() {
        // A reader puts the content column after the marker's spaces, comrak indents by the marker.
        let mut builder = InternalDocumentBuilder::new("test");
        builder.push_list(true);
        builder.push_list_item(" $", true, vec![], None, None);
        builder.push_formula(">", None, None);
        builder.push_paragraph("1", vec![], None, None);
        builder.push_list_item("", true, vec![], None, None);
        builder.push_paragraph("$-", vec![], None, None);
        builder.push_list_item("", true, vec![], None, None);
        push_math_paragraph(&mut builder, "$$>$<t>$$$", &["$$>$<t>$$"]);
        builder.end_list();
        assert_literals(&crate::rendering::render_markdown(&builder.build()), &[">", ">$<t>"]);

        let mut builder = InternalDocumentBuilder::new("test");
        builder.push_list(false);
        builder.push_list_item(" $", false, vec![], None, None);
        builder.push_list(true);
        builder.push_list_item("\n$", true, vec![], None, None);
        builder.push_code("", Some("`<x>"), None, None);
        builder.end_list();
        builder.end_list();
        let markdown = crate::rendering::render_markdown(&builder.build());
        assert_literals(&markdown, &[""]);
        assert!(markdown.starts_with("- $\n  1. $\n"), "{markdown}");
    }

    #[test]
    fn should_keep_an_empty_texted_item_s_blocks_inside_it() {
        let mut builder = InternalDocumentBuilder::new("test");
        builder.push_list(true);
        builder.push_list_item("", true, vec![], None, None);
        builder.push_paragraph("x", vec![], None, None);
        builder.push_code("y", None, None, None);
        builder.push_list_item("z", true, vec![], None, None);
        builder.end_list();
        let markdown = crate::rendering::render_markdown(&builder.build());
        assert_literals(&markdown, &["y"]);
        assert_eq!(markdown, "1. x\n   \n   ```\n   y\n   ```\n\n2. z\n");
        let arena = comrak::Arena::new();
        let root = comrak::parse_document(&arena, &markdown, &crate::rendering::markdown::comrak_options());
        let item = root.first_child().and_then(|list| list.first_child()).unwrap();
        assert_eq!(item.children().count(), 2, "{markdown}");
    }

    #[test]
    fn should_write_an_ordinary_list_tight() {
        let mut builder = InternalDocumentBuilder::new("test");
        builder.push_list(false);
        builder.push_list_item("Alpha", false, vec![], None, None);
        builder.push_list_item("Beta", false, vec![], None, None);
        builder.end_list();
        builder.push_paragraph("Then", vec![], None, None);
        builder.push_list(true);
        builder.push_list_item("One", true, vec![], None, None);
        builder.push_list(false);
        builder.push_list_item("Nested", false, vec![], None, None);
        builder.end_list();
        builder.push_list_item("Two", true, vec![], None, None);
        builder.end_list();
        assert_eq!(
            crate::rendering::render_markdown(&builder.build()),
            "- Alpha\n- Beta\n\nThen\n\n1. One\n   - Nested\n2. Two\n"
        );
    }
}
