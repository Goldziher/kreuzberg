//! Derivation pipeline: converts `InternalDocument` → `DocumentStructure` + `ExtractedDocument`.
//!
//! This module bridges the internal flat document representation produced by extractors
//! and the public-facing types consumed by callers. It handles:
//!
//! - **Relationship resolution**: `RelationshipTarget::Key` → `RelationshipTarget::Index`
//! - **Tree reconstruction**: Flat elements → hierarchical `DocumentStructure`
//! - **Content string derivation**: Concatenation of text-carrying elements
//! - **ExtractedDocument assembly**: Combining all outputs into the final result

use std::borrow::Cow;

use ahash::AHashMap;

use crate::types::document_structure::{
    DocumentNode, DocumentRelationship, DocumentStructure, GridCell, NodeContent, NodeId, NodeIndex, TableGrid,
};
use crate::types::extraction::{ExtractedDocument, ExtractionMethod};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement, RelationshipTarget};
use crate::types::tables::Table;

mod formulas;
mod pages;
use formulas::derive_formulas;
pub(crate) use formulas::strip_math_delimiters;
use pages::{apply_page_content_format, build_ocr_elements, build_pages};

/// Cap on how many unresolvable keys a single warning names before it summarises
/// the rest as a count, so a badly broken document cannot produce an unbounded
/// message.
const MAX_REPORTED_UNRESOLVED_KEYS: usize = 10;

/// Resolve `RelationshipTarget::Key` entries to `RelationshipTarget::Index`.
///
/// Builds an anchor index from elements with non-`None` anchors, then resolves
/// each key-based relationship target. Unresolvable keys are skipped — the
/// relationship is left as `Key` and excluded from the final `DocumentStructure`
/// relationships — and reported in one `ProcessingWarning` naming them.
pub(crate) fn resolve_relationships(doc: &mut InternalDocument) {
    let mut anchor_map: AHashMap<&str, u32> = AHashMap::new();
    for (idx, elem) in doc.elements.iter().enumerate() {
        if matches!(elem.kind, ElementKind::FootnoteRef | ElementKind::CommentRef) {
            continue;
        }
        if let Some(anchor) = elem.anchor.as_deref() {
            anchor_map.entry(anchor).or_insert(idx as u32);
        }
    }

    let mut unresolved: Vec<String> = Vec::new();
    for rel in &mut doc.relationships {
        if let RelationshipTarget::Key(ref key) = rel.target {
            match anchor_map.get(key.as_str()) {
                Some(&idx) => {
                    rel.target = RelationshipTarget::Index(idx);
                }
                None => {
                    log::debug!("Unresolvable relationship key: {}", key);
                    unresolved.push(key.clone());
                }
            }
        }
    }

    if !unresolved.is_empty() {
        unresolved.sort();
        unresolved.dedup();
        let total = unresolved.len();
        unresolved.truncate(MAX_REPORTED_UNRESOLVED_KEYS);
        let listed = unresolved.join(", ");
        let suffix = if total > unresolved.len() {
            format!(" (and {} more)", total - unresolved.len())
        } else {
            String::new()
        };
        // One warning for the document rather than one per key: cross-references usually
        // break as a set, and a per-key warning would flood `processing_warnings` on a
        // large document. Previously this was `log::debug!` only, so a citation or
        // cross-reference that failed to resolve vanished from `DocumentStructure` with
        // no diagnostic at all (#74). ~keep
        crate::core::diagnostics::push_warning(
            &mut doc.processing_warnings,
            "relationships",
            format!(
                "{total} cross-reference target(s) could not be resolved and were dropped from the \
                 document structure: {listed}{suffix}"
            ),
        );
    }
}

/// Inner implementation that assumes relationships are already resolved.
///
/// Takes `&mut` so it can move data out of elements via `std::mem::take`,
/// avoiding clones. Callers that still need `elem.text` (build_pages,
/// build_ocr_elements) must run before this function.
/// Handle one element of the flat `InternalDocument`, appending nodes to `ds` and
/// updating `stack`/`elem_to_node` as needed.
///
/// Called once per non-consumed element (a definition description already folded into
/// its paired term via `def_pairs` is skipped by the caller) by
/// [`derive_document_structure_inner`]'s main loop -- pulled into a named helper per
/// element kind rather than left as one large loop body.
fn process_document_element(
    ds: &mut DocumentStructure,
    stack: &mut Vec<(u16, NodeIndex)>,
    elem_to_node: &mut [Option<NodeIndex>],
    doc: &mut InternalDocument,
    elem_idx: usize,
    def_pairs: &AHashMap<usize, usize>,
) {
    match doc.elements[elem_idx].kind {
        ElementKind::ListEnd | ElementKind::QuoteEnd | ElementKind::GroupEnd => {
            close_container(stack, ds, doc.elements[elem_idx].kind);
            return;
        }
        ElementKind::FootnoteRef | ElementKind::CommentRef => {
            return;
        }
        _ => {}
    }

    let elem = &doc.elements[elem_idx];

    if elem.kind.is_container_start() {
        pop_stack_to_depth(stack, elem.depth);
        let content = match elem.kind {
            ElementKind::ListStart { ordered } => NodeContent::List { ordered },
            ElementKind::QuoteStart => NodeContent::Quote,
            ElementKind::GroupStart => NodeContent::Group {
                label: elem.attributes.as_ref().and_then(|a| a.get("label").cloned()),
                heading_level: None,
                heading_text: None,
            },
            _ => unreachable!("variant already checked by is_container_start()"),
        };
        let node_idx = push_node(ds, stack, content, elem, elem_idx as u32);
        elem_to_node[elem_idx] = Some(node_idx);
        stack.push((elem.depth, node_idx));
        return;
    }

    if let ElementKind::Heading { level } = elem.kind {
        handle_heading_element(ds, stack, elem_to_node, doc, elem_idx, level);
        return;
    }

    if let Some(&desc_idx) = def_pairs.get(&elem_idx) {
        handle_definition_pair(ds, stack, elem_to_node, doc, elem_idx, desc_idx);
        return;
    }

    if matches!(
        elem.kind,
        ElementKind::DefinitionTerm | ElementKind::DefinitionDescription
    ) {
        handle_definition_term_or_description(ds, stack, elem_to_node, doc, elem_idx);
        return;
    }

    if stack
        .last()
        .is_some_and(|(_, idx)| matches!(ds.nodes[idx.0 as usize].content, NodeContent::DefinitionList))
    {
        stack.pop();
    }

    pop_stack_to_depth(stack, elem.depth);
    let content = element_to_node_content(&mut doc.elements[elem_idx], &doc.tables, &doc.images);
    let annotations = std::mem::take(&mut doc.elements[elem_idx].annotations);
    let node_idx = push_node_with_annotations(
        ds,
        stack,
        content,
        &doc.elements[elem_idx],
        annotations,
        elem_idx as u32,
    );
    elem_to_node[elem_idx] = Some(node_idx);
}

/// Handle a `Heading` element: wraps it in a derived `Group` node (so later siblings
/// nest under the heading) with the heading itself as that group's first child.
fn handle_heading_element(
    ds: &mut DocumentStructure,
    stack: &mut Vec<(u16, NodeIndex)>,
    elem_to_node: &mut [Option<NodeIndex>],
    doc: &mut InternalDocument,
    elem_idx: usize,
    level: u8,
) {
    let elem = &doc.elements[elem_idx];
    pop_stack_to_depth(stack, elem.depth);

    let text = std::mem::take(&mut doc.elements[elem_idx].text);
    let annotations = std::mem::take(&mut doc.elements[elem_idx].annotations);
    let elem = &doc.elements[elem_idx];

    let group_content = NodeContent::Group {
        label: None,
        heading_level: Some(level),
        heading_text: Some(text.clone()),
    };

    let group_idx = push_node(ds, stack, group_content, elem, elem_idx as u32);

    let heading_node_index = ds.len() as u32;
    let heading_node = DocumentNode {
        id: NodeId::generate("heading", &text, elem.page, heading_node_index).to_string(),
        content: NodeContent::Heading { level, text },
        parent: Some(group_idx),
        children: vec![],
        content_layer: elem.layer,
        page: elem.page,
        page_end: None,
        bbox: elem.bbox,
        annotations,
        attributes: elem.public_attributes(),
    };
    let heading_idx = ds.push_node(heading_node);
    ds.nodes[group_idx.0 as usize].children.push(heading_idx);

    elem_to_node[elem_idx] = Some(group_idx);
    stack.push((elem.depth, group_idx));
}

/// Handle a `DefinitionTerm` element already paired with its following
/// `DefinitionDescription` (see `def_pairs`): both fold into one `DefinitionItem` node.
fn handle_definition_pair(
    ds: &mut DocumentStructure,
    stack: &mut Vec<(u16, NodeIndex)>,
    elem_to_node: &mut [Option<NodeIndex>],
    doc: &mut InternalDocument,
    elem_idx: usize,
    desc_idx: usize,
) {
    let elem = &doc.elements[elem_idx];
    pop_stack_to_depth(stack, elem.depth);

    let is_in_def_list = stack
        .last()
        .is_some_and(|(_, idx)| matches!(ds.nodes[idx.0 as usize].content, NodeContent::DefinitionList));
    if !is_in_def_list {
        let dl_idx = push_node(ds, stack, NodeContent::DefinitionList, elem, elem_idx as u32);
        stack.push((elem.depth, dl_idx));
    }

    let term = std::mem::take(&mut doc.elements[elem_idx].text);
    let definition = std::mem::take(&mut doc.elements[desc_idx].text);
    let elem = &doc.elements[elem_idx];
    let content = NodeContent::DefinitionItem { term, definition };
    let node_idx = push_node(ds, stack, content, elem, elem_idx as u32);
    elem_to_node[elem_idx] = Some(node_idx);
    elem_to_node[desc_idx] = Some(node_idx);
}

/// Handle a `DefinitionTerm` or `DefinitionDescription` element that arrived without its
/// pair (an unmatched term, or a description whose preceding element was not a term).
fn handle_definition_term_or_description(
    ds: &mut DocumentStructure,
    stack: &mut Vec<(u16, NodeIndex)>,
    elem_to_node: &mut [Option<NodeIndex>],
    doc: &mut InternalDocument,
    elem_idx: usize,
) {
    let elem = &doc.elements[elem_idx];
    pop_stack_to_depth(stack, elem.depth);

    let is_in_def_list = stack
        .last()
        .is_some_and(|(_, idx)| matches!(ds.nodes[idx.0 as usize].content, NodeContent::DefinitionList));
    if !is_in_def_list {
        let dl_idx = push_node(ds, stack, NodeContent::DefinitionList, elem, elem_idx as u32);
        stack.push((elem.depth, dl_idx));
    }

    let content = element_to_node_content(&mut doc.elements[elem_idx], &doc.tables, &doc.images);
    let annotations = std::mem::take(&mut doc.elements[elem_idx].annotations);
    let node_idx = push_node_with_annotations(
        ds,
        stack,
        content,
        &doc.elements[elem_idx],
        annotations,
        elem_idx as u32,
    );
    elem_to_node[elem_idx] = Some(node_idx);
}

fn derive_document_structure_inner(doc: &mut InternalDocument) -> DocumentStructure {
    let mut ds = DocumentStructure::with_capacity(doc.elements.len());
    ds.source_format = Some(doc.source_format.to_string());

    let mut stack: Vec<(u16, NodeIndex)> = Vec::new();

    let mut elem_to_node: Vec<Option<NodeIndex>> = vec![None; doc.elements.len()];

    let mut consumed: Vec<bool> = vec![false; doc.elements.len()];

    let mut def_pairs: AHashMap<usize, usize> = AHashMap::new();
    for i in 0..doc.elements.len().saturating_sub(1) {
        if matches!(doc.elements[i].kind, ElementKind::DefinitionTerm)
            && matches!(doc.elements[i + 1].kind, ElementKind::DefinitionDescription)
        {
            def_pairs.insert(i, i + 1);
            consumed[i + 1] = true;
        }
    }

    for (elem_idx, &was_consumed) in consumed.iter().enumerate() {
        if was_consumed {
            continue;
        }
        process_document_element(&mut ds, &mut stack, &mut elem_to_node, doc, elem_idx, &def_pairs);
    }

    for rel in &doc.relationships {
        if let RelationshipTarget::Index(target_elem_idx) = rel.target {
            let source_node = elem_to_node
                .get(rel.source as usize)
                .and_then(|n| *n)
                .or_else(|| (0..rel.source as usize).rev().find_map(|i| elem_to_node[i]));
            let target_node = elem_to_node.get(target_elem_idx as usize).and_then(|n| *n);
            if let (Some(src), Some(tgt)) = (source_node, target_node) {
                ds.relationships.push(DocumentRelationship {
                    source: src,
                    target: tgt,
                    kind: rel.kind,
                });
            }
        }
    }

    debug_assert!(
        ds.validate().is_ok(),
        "DocumentStructure validation failed: {:?}",
        ds.validate()
    );

    ds.finalize_node_types();
    ds
}

/// Close the nearest explicit container matching an end marker.
///
/// Derived heading groups may sit above an explicit container on the stack. An
/// end marker closes both those derived groups and its matching container,
/// rather than mistaking the heading group for the explicit group itself.
fn close_container(stack: &mut Vec<(u16, NodeIndex)>, ds: &DocumentStructure, end_kind: ElementKind) {
    let Some(container_position) = stack.iter().rposition(|(_, node_idx)| {
        let content = &ds.nodes[node_idx.0 as usize].content;
        matches!(
            (end_kind, content),
            (ElementKind::ListEnd, NodeContent::List { .. })
                | (ElementKind::QuoteEnd, NodeContent::Quote)
                | (
                    ElementKind::GroupEnd,
                    NodeContent::Group {
                        heading_level: None,
                        ..
                    }
                )
        )
    }) else {
        return;
    };

    stack.truncate(container_position);
}

/// Pop the stack until the top has depth strictly less than `target_depth`.
fn pop_stack_to_depth(stack: &mut Vec<(u16, NodeIndex)>, target_depth: u16) {
    while stack.last().is_some_and(|(d, _)| *d >= target_depth) {
        stack.pop();
    }
}

/// Push a DocumentNode under the current stack top (or as root if stack is empty).
/// Clones annotations from the element. For cases where annotations have already
/// been taken, use `push_node_with_annotations` instead.
fn push_node(
    ds: &mut DocumentStructure,
    stack: &[(u16, NodeIndex)],
    content: NodeContent,
    elem: &InternalElement,
    _index: u32,
) -> NodeIndex {
    push_node_with_annotations(ds, stack, content, elem, elem.annotations.clone(), _index)
}

/// Push a DocumentNode with explicitly provided annotations (avoids cloning when
/// annotations have already been taken from the element).
fn push_node_with_annotations(
    ds: &mut DocumentStructure,
    stack: &[(u16, NodeIndex)],
    content: NodeContent,
    elem: &InternalElement,
    annotations: Vec<crate::types::document_structure::TextAnnotation>,
    _index: u32,
) -> NodeIndex {
    let node_type = content.node_type_str();
    let text_for_id = content.text().unwrap_or("");

    let node_index_val = ds.len() as u32;
    let node = DocumentNode {
        id: NodeId::generate(node_type, text_for_id, elem.page, node_index_val).to_string(),
        content,
        parent: None,
        children: vec![],
        content_layer: elem.layer,
        page: elem.page,
        page_end: None,
        bbox: elem.bbox,
        annotations,
        attributes: elem.public_attributes(),
    };

    let node_idx = ds.push_node(node);

    if let Some((_, parent_idx)) = stack.last() {
        ds.add_child(*parent_idx, node_idx);
    }

    node_idx
}

/// Convert an `InternalElement` + `ElementKind` into `NodeContent`.
///
/// Takes `&mut` so it can move text out via `std::mem::take` (pages/OCR have
/// already consumed what they need before this is called).
fn element_to_node_content(
    elem: &mut InternalElement,
    tables: &[Table],
    images: &[crate::types::ExtractedImage],
) -> NodeContent {
    match elem.kind {
        ElementKind::Title => NodeContent::Title {
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::Paragraph => NodeContent::Paragraph {
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::ListItem { .. } => NodeContent::ListItem {
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::Code => NodeContent::Code {
            text: std::mem::take(&mut elem.text),
            language: elem.attributes.as_ref().and_then(|a| a.get("language").cloned()),
        },
        ElementKind::Formula => NodeContent::Formula {
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::FootnoteDefinition => NodeContent::Footnote {
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::CommentDefinition => NodeContent::Comment {
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::Citation => NodeContent::Citation {
            key: elem.anchor.clone().unwrap_or_default(),
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::Table { table_index } => table_node_content(elem, table_index, tables),
        ElementKind::Image { image_index } => image_node_content(elem, image_index, images),
        ElementKind::PageBreak => NodeContent::PageBreak,
        ElementKind::Slide { number } => NodeContent::Slide {
            number,
            title: if elem.text.is_empty() {
                None
            } else {
                Some(std::mem::take(&mut elem.text))
            },
        },
        ElementKind::DefinitionTerm | ElementKind::DefinitionDescription => {
            definition_term_or_description_content(elem)
        }
        ElementKind::Admonition => admonition_node_content(elem),
        ElementKind::RawBlock => raw_block_node_content(elem),
        ElementKind::MetadataBlock => {
            let entries = parse_metadata_entries(&elem.text).into_iter().map(Into::into).collect();
            NodeContent::MetadataBlock { entries }
        }
        ElementKind::OcrText { .. } => NodeContent::Paragraph {
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::ListStart { ordered } => NodeContent::List { ordered },
        ElementKind::QuoteStart => NodeContent::Quote,
        ElementKind::GroupStart => NodeContent::Group {
            label: None,
            heading_level: None,
            heading_text: None,
        },
        ElementKind::Heading { level } => NodeContent::Heading {
            level,
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::FootnoteRef | ElementKind::CommentRef => NodeContent::Paragraph {
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::ListEnd | ElementKind::QuoteEnd | ElementKind::GroupEnd => {
            unreachable!("container end markers should be filtered before this point")
        }
    }
}

fn table_node_content(elem: &mut InternalElement, table_index: u32, tables: &[Table]) -> NodeContent {
    let native_grid = elem
        .attributes
        .as_mut()
        .and_then(|attributes| attributes.remove(crate::types::internal::NATIVE_TABLE_GRID_ATTRIBUTE))
        .and_then(|encoded| match serde_json::from_str::<TableGrid>(&encoded) {
            Ok(grid) => Some(grid),
            Err(error) => {
                tracing::warn!(table_index, %error, "discarding invalid internal native table grid");
                None
            }
        });
    let grid = if let Some(table) = tables.get(table_index as usize) {
        table_to_grid(table, native_grid.as_ref())
    } else {
        TableGrid {
            rows: 0,
            cols: 0,
            cells: vec![],
        }
    };
    NodeContent::Table { grid }
}

fn image_node_content(
    elem: &InternalElement,
    image_index: u32,
    images: &[crate::types::ExtractedImage],
) -> NodeContent {
    let description = images.get(image_index as usize).and_then(|img| img.description.clone());
    let src = elem.attributes.as_ref().and_then(|attrs| attrs.get("src").cloned());
    NodeContent::Image {
        description,
        image_index: Some(image_index),
        src,
    }
}

fn definition_term_or_description_content(elem: &mut InternalElement) -> NodeContent {
    let text = std::mem::take(&mut elem.text);
    if matches!(elem.kind, ElementKind::DefinitionTerm) {
        NodeContent::DefinitionItem {
            term: text,
            definition: String::new(),
        }
    } else {
        NodeContent::DefinitionItem {
            term: String::new(),
            definition: text,
        }
    }
}

fn admonition_node_content(elem: &InternalElement) -> NodeContent {
    let attrs = elem.attributes.as_ref();
    NodeContent::Admonition {
        kind: attrs
            .and_then(|a| a.get("kind").cloned())
            .unwrap_or_else(|| "note".to_string()),
        title: attrs.and_then(|a| a.get("title").cloned()),
    }
}

fn raw_block_node_content(elem: &mut InternalElement) -> NodeContent {
    let attrs = elem.attributes.as_ref();
    NodeContent::RawBlock {
        format: attrs.and_then(|a| a.get("format").cloned()).unwrap_or_default(),
        content: std::mem::take(&mut elem.text),
    }
}

/// Convert an internal `Table` to a `TableGrid`.
pub(crate) fn native_grid_matches_table(table: &Table, grid: &TableGrid) -> bool {
    // ~keep: Later table repair may replace the dense cells. Only reuse native geometry
    // while its dimensions and text still describe that same table.
    grid.rows as usize == table.cells.len()
        && table.cells.iter().all(|row| row.len() == grid.cols as usize)
        && grid.cells.iter().all(|cell| {
            table
                .cells
                .get(cell.row as usize)
                .and_then(|row| row.get(cell.col as usize))
                == Some(&cell.content)
        })
        && grid.cells.iter().filter(|cell| !cell.content.is_empty()).count()
            == table.cells.iter().flatten().filter(|cell| !cell.is_empty()).count()
}

fn table_to_grid(table: &Table, native_grid: Option<&TableGrid>) -> TableGrid {
    if let Some(grid) = native_grid.filter(|grid| native_grid_matches_table(table, grid)) {
        return grid.clone();
    }

    let rows = table.cells.len() as u32;
    let cols = table.cells.iter().map(|r| r.len()).max().unwrap_or(0) as u32;

    let mut cells = Vec::new();
    for (row_idx, row) in table.cells.iter().enumerate() {
        for (col_idx, cell_content) in row.iter().enumerate() {
            let style = table
                .cell_styles
                .iter()
                .find(|s| s.row as usize == row_idx && s.col as usize == col_idx);
            cells.push(GridCell {
                content: cell_content.clone(),
                row: row_idx as u32,
                col: col_idx as u32,
                row_span: 1,
                col_span: 1,
                is_header: row_idx == 0,
                bbox: None,
                heading_level: style.and_then(|s| s.heading_level),
                style_name: style.and_then(|s| s.style_name.clone()),
            });
        }
    }

    TableGrid { rows, cols, cells }
}

/// Parse "key: value" lines from metadata text into `(key, value)` pairs.
fn parse_metadata_entries(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                Some((key, value))
            } else {
                Some((line.to_string(), String::new()))
            }
        })
        .collect()
}

/// Derive the plain-text `content` and its `mime_type`.
///
/// A document produced by `From<ExtractedDocument> for InternalDocument` has no element
/// tree, so `render_plain` yields nothing. Its already-extracted text lives in
/// `pre_rendered_content` and must be returned verbatim rather than dropped. Only the
/// empty rendering falls back, so a document that does have elements always wins.
fn derive_content_and_mime_type(doc: &mut InternalDocument) -> (String, Cow<'static, str>) {
    let mut content = crate::rendering::render_plain(doc);
    if content.is_empty()
        && let Some(pre_rendered) = doc.pre_rendered_content.as_ref()
    {
        content = pre_rendered.clone();
    }

    let mime_type: Cow<'static, str> = if doc.mime_type != "application/octet-stream" {
        Cow::Owned(std::mem::take(&mut doc.mime_type))
    } else {
        Cow::Borrowed(source_format_to_mime_type(&doc.source_format))
    };

    (content, mime_type)
}

/// Pre-render or render `doc` in `output_format`, reusing an already-matching
/// `doc.pre_rendered_content` instead of re-rendering. `None` for `Plain`, since plain
/// content is carried separately as `content`.
fn compute_formatted_content(
    doc: &mut InternalDocument,
    output_format: &crate::core::config::OutputFormat,
) -> Option<String> {
    match output_format {
        crate::core::config::OutputFormat::Plain => None,
        crate::core::config::OutputFormat::Markdown => {
            if doc.pre_rendered_content.is_some() && doc.metadata.output_format.as_deref() == Some("markdown") {
                doc.pre_rendered_content.take()
            } else {
                Some(crate::rendering::render_markdown(doc))
            }
        }
        crate::core::config::OutputFormat::Djot => {
            if doc.pre_rendered_content.is_some() && doc.metadata.output_format.as_deref() == Some("djot") {
                doc.pre_rendered_content.take()
            } else {
                Some(crate::rendering::render_djot(doc))
            }
        }
        crate::core::config::OutputFormat::Html => {
            if doc.pre_rendered_content.is_some() && doc.metadata.output_format.as_deref() == Some("html") {
                doc.pre_rendered_content.take()
            } else {
                Some(crate::rendering::render_html(doc))
            }
        }
        crate::core::config::OutputFormat::Json => {
            if doc.pre_rendered_content.is_some() && doc.metadata.output_format.as_deref() == Some("json") {
                doc.pre_rendered_content.take()
            } else {
                Some(crate::rendering::render_json(doc))
            }
        }
        crate::core::config::OutputFormat::DocTags => {
            if doc.pre_rendered_content.is_some() && doc.metadata.output_format.as_deref() == Some("doctags") {
                doc.pre_rendered_content.take()
            } else {
                Some(crate::rendering::render_doctags(doc))
            }
        }
        crate::core::config::OutputFormat::Custom(name) => {
            // A prior `clear_renderers()` call (e.g. by a sibling test, or any consumer
            // resetting the plugin lifecycle) empties this global registry, including the
            // built-ins. Self-heal before dispatch so a built-in reached only through
            // `Custom` (such as "dot") is never permanently lost. ~keep
            crate::plugins::ensure_renderers_initialized();
            let registry = crate::plugins::registry::get_renderer_registry();
            let registry = registry.read();
            match registry.render(name, doc) {
                Ok(rendered) => Some(rendered),
                Err(e) => {
                    tracing::warn!(renderer = %name, error = %e, "Custom renderer failed, falling back to plain");
                    // #208: `tracing::warn!` is invisible to API/binding consumers — the
                    // only channel they can observe is `processing_warnings`. Without
                    // this, a typo'd or unregistered custom format silently produced
                    // plain text with no way for the caller to detect the fallback. ~keep
                    crate::core::diagnostics::push_warning(
                        &mut doc.processing_warnings,
                        "output-format",
                        format!(
                            "requested output format '{name}' has no registered renderer ({e}); \
                             returned plain text instead"
                        ),
                    );
                    None
                }
            }
        }
    }
}

/// Warn about any URI dropped at `InternalDocument::MAX_URIS`, then deduplicate and
/// return the kept URIs (`None` when there are none).
fn derive_uris(doc: &mut InternalDocument) -> Option<Vec<crate::types::uri::ExtractedUri>> {
    // #76: `push_uri` caps collection at `InternalDocument::MAX_URIS` and silently
    // discarded the rest, so a document with more links than the cap was
    // indistinguishable from one that genuinely has exactly `MAX_URIS`. Name the
    // loss; only when it actually happened, so a normal document stays warning-free. ~keep
    if doc.uris_dropped > 0 {
        let dropped = doc.uris_dropped;
        // Report the cap itself, not `doc.uris.len()`: the derivation runs a second
        // time after the captioning prepass, by which point the list has been
        // de-duplicated and shortened. A length-derived count would produce a second,
        // differently-worded warning that `push_warning`'s dedup could not collapse. ~keep
        let kept = InternalDocument::MAX_URIS;
        let found = kept + dropped;
        crate::core::diagnostics::push_warning(
            &mut doc.processing_warnings,
            "uris",
            format!(
                "Collected the first {kept} of {found} URIs; {dropped} were dropped at the \
                 per-document limit and are missing from the result"
            ),
        );
    }

    if doc.uris.is_empty() {
        None
    } else {
        let mut seen = ahash::AHashSet::with_capacity(doc.uris.len());
        doc.uris.retain(|uri| seen.insert((uri.url.clone(), uri.kind)));
        Some(std::mem::take(&mut doc.uris))
    }
}

/// `code_intelligence` is documented (types/extraction.rs) as carrying the full
/// `tree_sitter_language_pack::ProcessResult` — metrics, structure, imports, exports,
/// comments, docstrings, symbols, diagnostics, chunks and the hierarchical data tree.
/// `extractors/code.rs` stashes that entire serialized result under
/// `CODE_INTELLIGENCE_SCRATCH_KEY` in `metadata.additional` (the typed `CodeMetadata` on
/// `Metadata::format` only carries `chunks`/`data`, so it has no room for the rest).
/// Prefer that full payload; `.remove()` so it never leaks into the final
/// `ExtractedDocument.metadata.additional` map. Fall back to serializing just
/// `CodeMetadata` for documents that reach this point without going through
/// `CodeExtractor` (e.g. synthetic `InternalDocument`s built by tests or other callers
/// that set `FormatMetadata::Code` directly). ~keep
#[cfg(feature = "tree-sitter")]
fn derive_code_intelligence(doc: &mut InternalDocument) -> Option<serde_json::Value> {
    let is_code_metadata = matches!(
        doc.metadata.format.as_ref(),
        Some(crate::types::metadata::FormatMetadata::Code(_))
    );
    let full_process_result = if is_code_metadata {
        doc.metadata
            .additional
            .remove(crate::extractors::code::CODE_INTELLIGENCE_SCRATCH_KEY)
    } else {
        None
    };
    full_process_result.or_else(|| match doc.metadata.format.as_ref() {
        Some(crate::types::metadata::FormatMetadata::Code(code_metadata)) => serde_json::to_value(code_metadata).ok(),
        _ => None,
    })
}

/// Derive a complete `ExtractedDocument` from an `InternalDocument`.
///
/// This is the main entry point for the derivation pipeline. It:
/// 1. Resolves relationships (needed by renderers for footnotes)
/// 2. Renders plain-text content (for post-processors)
/// 3. Pre-renders formatted content if output_format != Plain
/// 4. Groups elements by page into `PageContent`
/// 5. Extracts OCR elements for backward compatibility
/// 6. Optionally derives `DocumentStructure` (assumes relationships resolved)
/// 7. Assembles the final `ExtractedDocument`
#[cfg_attr(alef, alef(skip))]
pub fn derive_extraction_result(
    doc: InternalDocument,
    include_document_structure: bool,
    output_format: crate::core::config::OutputFormat,
) -> ExtractedDocument {
    derive(doc, include_document_structure, output_format, true)
}

/// Derive the document that the registry hands to a registered public renderer.
///
/// Its `content` is the plain rendering and it has the document structure. The caller gets
/// the renderer's output, not this `content`, so its pages are built as for every output
/// that is not plain: from the text of their elements only. ~keep
pub(crate) fn derive_renderer_input(doc: InternalDocument) -> ExtractedDocument {
    derive(doc, true, crate::core::config::OutputFormat::Plain, false)
}

/// The derivation pipeline. `is_result` says that the caller gets this document: then its
/// `content` is the output when no formatted content is rendered.
fn derive(
    mut doc: InternalDocument,
    include_document_structure: bool,
    output_format: crate::core::config::OutputFormat,
    is_result: bool,
) -> ExtractedDocument {
    tracing::debug!(
        element_count = doc.elements.len(),
        source_format = %doc.source_format,
        include_document_structure,
        "derivation pipeline starting"
    );
    resolve_relationships(&mut doc);

    let (content, mime_type) = derive_content_and_mime_type(&mut doc);
    let formatted_content = compute_formatted_content(&mut doc, &output_format);

    // `content`, the plain rendering, is the output the caller gets exactly when the caller
    // gets this document and `formatted_content` is `None`: plain output, or a custom format
    // name with no renderer. The document for a registered renderer is not the caller's. ~keep
    let content_is_plain = is_result && formatted_content.is_none();
    let raw_pages = doc
        .prebuilt_pages
        .take()
        .or_else(|| build_pages(&doc, content_is_plain));
    let pages = apply_page_content_format(raw_pages, &doc, &output_format);
    let ocr_elements = doc.prebuilt_ocr_elements.take().or_else(|| build_ocr_elements(&doc));

    let formulas = derive_formulas(&mut doc);

    let document = if include_document_structure {
        Some(derive_document_structure_inner(&mut doc))
    } else {
        None
    };

    let uris = derive_uris(&mut doc);
    #[cfg(feature = "tree-sitter")]
    let code_intelligence = derive_code_intelligence(&mut doc);

    let images = if doc.images.is_empty() { None } else { Some(doc.images) };

    let metadata = doc.metadata;

    let extraction_method = metadata
        .additional
        .get("extraction_method")
        .and_then(serde_json::Value::as_str)
        .and_then(ExtractionMethod::from_metadata_value);

    tracing::debug!(
        content_length = content.len(),
        has_document_structure = document.is_some(),
        "derivation pipeline complete"
    );
    ExtractedDocument {
        content,
        mime_type,
        metadata,
        extraction_method,
        tables: doc.tables,
        images,
        pages,
        ocr_elements,
        document,
        processing_warnings: std::mem::take(&mut doc.processing_warnings),
        annotations: std::mem::take(&mut doc.annotations),
        children: std::mem::take(&mut doc.children),
        uris,
        llm_usage: std::mem::take(&mut doc.llm_usage),
        revisions: std::mem::take(&mut doc.revisions),
        form_fields: std::mem::take(&mut doc.form_fields),
        ocr_page_failures: Some(std::mem::take(&mut doc.ocr_page_failures)).filter(|failures| !failures.is_empty()),
        formulas,
        #[cfg(feature = "tree-sitter")]
        code_intelligence,
        formatted_content,
        ..Default::default()
    }
}

/// Map source format identifiers to MIME types.
fn source_format_to_mime_type(format: &str) -> &'static str {
    match format {
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "doc" => "application/msword",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "ppt" => "application/vnd.ms-powerpoint",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "xls" => "application/vnd.ms-excel",
        "html" => "text/html",
        "markdown" | "md" => "text/markdown",
        "xml" => "application/xml",
        "json" => "application/json",
        "yaml" | "yml" => "application/yaml",
        "toml" => "application/toml",
        "csv" => "text/csv",
        "eml" | "msg" => "message/rfc822",
        "pst" => "application/vnd.ms-outlook-pst",
        "rtf" => "application/rtf",
        "txt" | "text" => "text/plain",
        "djot" => "text/djot",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_extraction_result;
