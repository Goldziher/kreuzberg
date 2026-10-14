//! Per-page bounding-box aggregation for chunk `page_spans` (#1295) and node-id backreferences
//! (#1296).
//!
//! [`crate::chunking::boundaries::calculate_page_spans`] derives the *page numbers*
//! a chunk covers from byte-range overlap against [`PageBoundary`](crate::types::PageBoundary)
//! markers — the same mechanism already used for `first_page`/`last_page`. This module adds two
//! things once a document's structured node tree ([`DocumentStructure`]) is available:
//! [`populate_page_span_bboxes`] fills each [`PageSpan`](crate::types::PageSpan)'s `bbox` with the
//! union of the bounding boxes of the body-layer nodes on that page whose text appears in the
//! chunk, and — reusing the same node/chunk match — collects those nodes' ids into the chunk's
//! [`ChunkMetadata::node_ids`](crate::types::ChunkMetadata::node_ids).
//!
//! There is currently no byte-offset mapping from rendered output back to individual
//! [`DocumentNode`](crate::types::document_structure::DocumentNode)s (tracked under #1294;
//! see [`DocumentStructure::node_rendered_offset`](crate::types::document_structure::DocumentStructure::node_rendered_offset)).
//! In its absence, node-to-chunk membership on a given page is determined by a textual
//! containment check — the same substring-matching approach
//! [`locate_page_boundaries`](crate::core::pipeline::features) already uses to align raw page
//! text with rendered content — rather than a byte-exact intersection.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use super::node_text_trie::NodeTextTrie;
use crate::types::document_structure::{ContentLayer, DocumentNode, DocumentStructure, NodeContent};
use crate::types::{BoundingBox, Chunk};

/// Minimum trimmed node-text length considered for containment matching.
///
/// Short text (e.g. a label, a number like "1.2.", or a common word like "Data") is likely to
/// appear verbatim in chunks it has nothing to do with, which would pollute that page's union
/// bbox with an unrelated node's coordinates. Ten characters is long enough that a coincidental
/// substring match across unrelated content is unlikely, while still covering most short
/// headings and list items. Nodes with shorter text are skipped for bbox aggregation — their
/// page still appears in `page_spans`, just without a bbox contribution from that node.
const MIN_NODE_TEXT_MATCH_LEN: usize = 10;

/// Pages with at least this many candidate nodes are matched through a [`NodeTextTrie`]
/// instead of testing every node against every chunk.
const TRIE_MIN_NODES: usize = 64;

/// A page's body-layer nodes, with a text index when there are enough of them to need one.
struct PageNodes<'a> {
    nodes: Vec<&'a DocumentNode>,
    trie: Option<NodeTextTrie>,
}

impl<'a> PageNodes<'a> {
    fn new(nodes: Vec<&'a DocumentNode>) -> Self {
        let trie = (nodes.len() >= TRIE_MIN_NODES).then(|| {
            let mut trie = NodeTextTrie::new();
            for (id, node) in nodes.iter().enumerate() {
                for text in node_text_for_matching(&node.content) {
                    let trimmed = text.trim();
                    if trimmed.len() >= MIN_NODE_TEXT_MATCH_LEN {
                        trie.insert(trimmed.as_bytes(), id as u32);
                    }
                }
            }
            trie
        });
        Self { nodes, trie }
    }
}

/// Fill in `bbox` on each chunk's `page_spans`, and `metadata.node_ids`, using the document's
/// structured node tree.
///
/// For every chunk and every [`PageSpan`](crate::types::PageSpan) already present on it (as
/// produced by `calculate_page_spans`), this unions the bounding boxes of all body-layer nodes
/// that:
/// - fall on that page (`node.page..=node.page_end` covers the span's page), and
/// - carry a `bbox`, and
/// - carry matching text (see [`node_text_for_matching`]) found verbatim within the chunk's
///   `content`.
///
/// The same per-page match additionally collects the `id` of every matching body-layer node
/// (regardless of whether it carries a `bbox`) into the chunk's
/// [`ChunkMetadata::node_ids`](crate::types::ChunkMetadata::node_ids), deduplicated and in
/// node-traversal order, without a second sweep over nodes × chunks.
///
/// Chunks with empty `page_spans` (no page-boundary provenance) are skipped entirely — their
/// `node_ids` stays empty. A span's `bbox` stays `None` when no node in `structure` matches.
///
/// Nodes are bucketed by page once, up front (see [`index_body_nodes_by_page`]), so each
/// chunk/page_span pair only scans the (typically small) set of nodes on its own page instead of
/// the entire node tree — this keeps the cost roughly linear in document size rather than
/// quadratic in `chunks.len() * structure.nodes.len()`.
pub(crate) fn populate_page_span_bboxes(chunks: &mut [Chunk], structure: &DocumentStructure) {
    let nodes_by_page = index_body_nodes_by_page(&structure.nodes);

    for chunk in chunks.iter_mut() {
        if chunk.metadata.page_spans.is_empty() {
            continue;
        }

        let mut matched_node_ids = Vec::new();
        for span in chunk.metadata.page_spans.iter_mut() {
            let Some(nodes) = nodes_by_page.get(&span.page) else {
                span.bbox = None;
                continue;
            };
            let (bbox, ids) = match_page_nodes(&chunk.content, nodes);
            span.bbox = bbox;
            matched_node_ids.extend(ids);
        }
        chunk.metadata.node_ids = dedup_preserve_order(matched_node_ids);
    }
}

/// Bucket body-layer nodes by every page they cover.
///
/// A node with `page_end` set (spanning pages `page..=page_end`) is indexed under every page in
/// that range, matching the membership test `populate_page_span_bboxes` used before this index
/// existed (`node.page..=node.page_end` covers the span's page). Non-body-layer nodes (headers,
/// footers, footnotes) and nodes without a `page` are omitted entirely, since they never
/// contribute to a bbox union.
fn index_body_nodes_by_page(nodes: &[DocumentNode]) -> HashMap<u32, PageNodes<'_>> {
    let mut index: HashMap<u32, Vec<&DocumentNode>> = HashMap::new();

    for node in nodes {
        if node.content_layer != ContentLayer::Body {
            continue;
        }
        let Some(start) = node.page else { continue };
        let end = node.page_end.unwrap_or(start);

        for page in start..=end {
            index.entry(page).or_default().push(node);
        }
    }

    index
        .into_iter()
        .map(|(page, nodes)| (page, PageNodes::new(nodes)))
        .collect()
}

/// Match all `nodes` (already scoped to a single page) whose text is found in `chunk_content`,
/// returning both the union of their bounding boxes and the ids of the matched nodes.
///
/// A node contributes its `id` whenever its text matches, independent of whether it carries a
/// `bbox` — `node_ids` link chunks back to structure nodes and shouldn't be gated on layout
/// coordinates being available.
fn match_page_nodes<'a>(chunk_content: &str, page: &PageNodes<'a>) -> (Option<BoundingBox>, Vec<&'a str>) {
    let mut union = None;
    let mut ids = Vec::new();

    let matched: Vec<&'a DocumentNode> = match &page.trie {
        Some(trie) => trie
            .matching_ids(chunk_content.as_bytes())
            .into_iter()
            .map(|id| page.nodes[id as usize])
            .collect(),
        None => page
            .nodes
            .iter()
            .copied()
            .filter(|node| node_text_matches_chunk(node, chunk_content))
            .collect(),
    };

    for node in matched {
        if let Some(bbox) = node.bbox {
            union = Some(union_bbox(union, bbox));
        }
        if !node.id.is_empty() {
            ids.push(node.id.as_str());
        }
    }

    (union, ids)
}

/// Deduplicate `ids` while preserving the order of first occurrence, allocating each retained id
/// exactly once. The borrowed `&str` inputs are tied to the document's node tree, so the set can
/// test membership without cloning.
fn dedup_preserve_order(ids: Vec<&str>) -> Vec<String> {
    let mut seen = HashSet::with_capacity(ids.len());
    ids.into_iter()
        .filter(|id| seen.insert(*id))
        .map(str::to_string)
        .collect()
}

/// Check whether any of `node`'s matchable text fragments (see [`node_text_for_matching`])
/// appear verbatim in `chunk_content`, after trimming and enforcing [`MIN_NODE_TEXT_MATCH_LEN`].
fn node_text_matches_chunk(node: &DocumentNode, chunk_content: &str) -> bool {
    node_text_for_matching(&node.content).into_iter().any(|text| {
        let trimmed = text.trim();
        trimmed.len() >= MIN_NODE_TEXT_MATCH_LEN && chunk_content.contains(trimmed)
    })
}

/// Extract the text fragment(s) of a node usable for chunk-containment matching.
///
/// Most node kinds carry a single, directly comparable text string. `Table` instead yields
/// one fragment per cell: a table's rendered form (pipe-delimited rows with separators) never
/// appears as a single contiguous run of its cell text, so matching per-cell lets a table-only
/// chunk still contribute a bbox/node_id when at least one of its cells appears verbatim in the
/// chunk (#212). `DefinitionItem` yields its term and definition as separate fragments for the
/// same reason — the rendered form is unlikely to place them contiguously.
///
/// Pure container nodes (`List`, `Quote`, `DefinitionList`, `PageBreak`, `RawBlock`,
/// `MetadataBlock`) have no comparable text of their own and yield nothing; their children
/// (which are matched individually) carry the text.
fn node_text_for_matching(content: &NodeContent) -> Vec<Cow<'_, str>> {
    match content {
        NodeContent::Title { text }
        | NodeContent::Heading { text, .. }
        | NodeContent::Paragraph { text }
        | NodeContent::ListItem { text }
        | NodeContent::Code { text, .. }
        | NodeContent::Formula { text }
        | NodeContent::Footnote { text } => vec![Cow::Borrowed(text.as_str())],
        NodeContent::Citation { text, .. } => vec![Cow::Borrowed(text.as_str())],
        NodeContent::Image { description, .. } => description.as_deref().map(Cow::Borrowed).into_iter().collect(),
        NodeContent::Group { heading_text, .. } => heading_text.as_deref().map(Cow::Borrowed).into_iter().collect(),
        NodeContent::Slide { title, .. } => title.as_deref().map(Cow::Borrowed).into_iter().collect(),
        NodeContent::DefinitionItem { term, definition } => {
            vec![Cow::Borrowed(term.as_str()), Cow::Borrowed(definition.as_str())]
        }
        NodeContent::Admonition { title, .. } => title.as_deref().map(Cow::Borrowed).into_iter().collect(),
        NodeContent::Table { grid } => grid
            .cells
            .iter()
            .map(|cell| Cow::Borrowed(cell.content.as_str()))
            .collect(),
        _ => Vec::new(),
    }
}

/// Union two bounding boxes into the smallest box that encloses both, or just `next` if `acc` is
/// absent.
fn union_bbox(acc: Option<BoundingBox>, next: BoundingBox) -> BoundingBox {
    match acc {
        None => next,
        Some(acc) => BoundingBox {
            x0: acc.x0.min(next.x0),
            y0: acc.y0.min(next.y0),
            x1: acc.x1.max(next.x1),
            y1: acc.y1.max(next.y1),
        },
    }
}

#[cfg(test)]
mod tests;
