use super::*;
use crate::types::{ChunkMetadata, ChunkType, PageSpan};

fn body_node(text: &str, page: u32, bbox: Option<BoundingBox>) -> DocumentNode {
    DocumentNode {
        id: String::new(),
        content: NodeContent::Paragraph { text: text.to_string() },
        parent: None,
        children: Vec::new(),
        content_layer: ContentLayer::Body,
        page: Some(page),
        page_end: None,
        bbox,
        annotations: Vec::new(),
        attributes: None,
    }
}

fn body_node_with_id(id: &str, text: &str, page: u32, bbox: Option<BoundingBox>) -> DocumentNode {
    DocumentNode {
        id: id.to_string(),
        content: NodeContent::Paragraph { text: text.to_string() },
        parent: None,
        children: Vec::new(),
        content_layer: ContentLayer::Body,
        page: Some(page),
        page_end: None,
        bbox,
        annotations: Vec::new(),
        attributes: None,
    }
}

fn bbox(x0: f64, y0: f64, x1: f64, y1: f64) -> BoundingBox {
    BoundingBox { x0, y0, x1, y1 }
}

fn chunk_with_spans(content: &str, spans: Vec<PageSpan>) -> Chunk {
    Chunk {
        content: content.to_string(),
        chunk_type: ChunkType::default(),
        embedding: None,
        sparse_embedding: None,
        late_interaction: None,
        metadata: ChunkMetadata {
            byte_start: 0,
            byte_end: content.len(),
            token_count: None,
            chunk_index: 0,
            total_chunks: 1,
            first_page: spans.first().map(|s| s.page),
            last_page: spans.last().map(|s| s.page),
            heading_context: None,
            heading_path: Vec::new(),
            image_indices: Vec::new(),
            node_ids: Vec::new(),
            page_spans: spans,
            classifications: Vec::new(),
        },
    }
}

#[test]
fn should_populate_bbox_when_single_page_node_text_found_in_chunk() {
    let structure = DocumentStructure {
        nodes: vec![body_node(
            "Hello world, this is page one.",
            1,
            Some(bbox(10.0, 20.0, 100.0, 200.0)),
        )],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "Hello world, this is page one.",
        vec![PageSpan { page: 1, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(chunks[0].metadata.page_spans.len(), 1);
    assert_eq!(chunks[0].metadata.page_spans[0].page, 1);
    assert_eq!(
        chunks[0].metadata.page_spans[0].bbox,
        Some(bbox(10.0, 20.0, 100.0, 200.0))
    );
}

#[test]
fn should_union_bboxes_when_multi_page_chunk_has_a_node_on_each_page() {
    let structure = DocumentStructure {
        nodes: vec![
            body_node("Content from page three.", 3, Some(bbox(72.2, 255.8, 400.0, 400.0))),
            body_node("Content from page four.", 4, Some(bbox(56.6, 610.1, 530.3, 766.1))),
        ],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "Content from page three.\n\nContent from page four.",
        vec![PageSpan { page: 3, bbox: None }, PageSpan { page: 4, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    let spans = &chunks[0].metadata.page_spans;
    assert_eq!(
        spans.len(),
        2,
        "chunk spanning pages 3-4 must keep one PageSpan per page"
    );
    assert_eq!(spans[0].page, 3);
    assert_eq!(spans[0].bbox, Some(bbox(72.2, 255.8, 400.0, 400.0)));
    assert_eq!(spans[1].page, 4);
    assert_eq!(spans[1].bbox, Some(bbox(56.6, 610.1, 530.3, 766.1)));
}

#[test]
fn should_union_multiple_node_bboxes_on_the_same_page() {
    let structure = DocumentStructure {
        nodes: vec![
            body_node("First paragraph on the page.", 1, Some(bbox(10.0, 10.0, 50.0, 50.0))),
            body_node("Second paragraph on the page.", 1, Some(bbox(60.0, 60.0, 120.0, 120.0))),
        ],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "First paragraph on the page.\n\nSecond paragraph on the page.",
        vec![PageSpan { page: 1, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(
        chunks[0].metadata.page_spans[0].bbox,
        Some(bbox(10.0, 10.0, 120.0, 120.0))
    );
}

#[test]
fn should_leave_bbox_none_when_no_node_bbox_available() {
    let structure = DocumentStructure {
        nodes: vec![body_node("Hello world, this is page one.", 1, None)],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "Hello world, this is page one.",
        vec![PageSpan { page: 1, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(chunks[0].metadata.page_spans[0].bbox, None);
}

#[test]
fn should_leave_bbox_none_when_node_text_not_found_in_chunk() {
    let structure = DocumentStructure {
        nodes: vec![body_node(
            "Unrelated content elsewhere.",
            1,
            Some(bbox(1.0, 1.0, 2.0, 2.0)),
        )],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "Completely different chunk text.",
        vec![PageSpan { page: 1, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(chunks[0].metadata.page_spans[0].bbox, None);
}

#[test]
fn should_skip_non_body_layer_nodes() {
    let mut header = body_node("Running header text.", 1, Some(bbox(1.0, 1.0, 2.0, 2.0)));
    header.content_layer = ContentLayer::Header;
    let structure = DocumentStructure {
        nodes: vec![header],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "Running header text.",
        vec![PageSpan { page: 1, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(
        chunks[0].metadata.page_spans[0].bbox, None,
        "header/footer/footnote nodes must not contribute bboxes"
    );
}

#[test]
fn should_noop_when_chunk_has_no_page_spans() {
    let structure = DocumentStructure {
        nodes: vec![body_node("Some text.", 1, Some(bbox(1.0, 1.0, 2.0, 2.0)))],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans("Some text.", Vec::new())];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert!(chunks[0].metadata.page_spans.is_empty());
}

#[test]
fn should_match_node_that_spans_a_page_range() {
    let mut node = body_node("Table caption spanning pages.", 2, Some(bbox(5.0, 5.0, 15.0, 15.0)));
    node.page_end = Some(3);
    let structure = DocumentStructure {
        nodes: vec![node],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "Table caption spanning pages.",
        vec![PageSpan { page: 3, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(
        chunks[0].metadata.page_spans[0].bbox,
        Some(bbox(5.0, 5.0, 15.0, 15.0)),
        "node with page_end covering the span page must match"
    );
}

#[test]
fn should_not_match_short_text_below_min_length_threshold() {
    let structure = DocumentStructure {
        nodes: vec![body_node("Hi", 1, Some(bbox(1.0, 1.0, 2.0, 2.0)))],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "Hi there, this chunk contains Hi as a substring.",
        vec![PageSpan { page: 1, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(
        chunks[0].metadata.page_spans[0].bbox, None,
        "text shorter than MIN_NODE_TEXT_MATCH_LEN must not be treated as a membership signal"
    );
}

#[test]
fn should_not_match_nine_character_text_just_below_the_threshold() {
    let structure = DocumentStructure {
        nodes: vec![body_node("Data 1.2.", 1, Some(bbox(1.0, 1.0, 2.0, 2.0)))],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "See Data 1.2. for the summary table on this page.",
        vec![PageSpan { page: 1, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(
        chunks[0].metadata.page_spans[0].bbox, None,
        "9-char text is below MIN_NODE_TEXT_MATCH_LEN (10) and must not match"
    );
}

#[test]
fn union_bbox_expands_to_enclose_both_boxes() {
    let result = union_bbox(Some(bbox(0.0, 0.0, 10.0, 10.0)), bbox(5.0, -5.0, 20.0, 8.0));
    assert_eq!(result, bbox(0.0, -5.0, 20.0, 10.0));
}

#[test]
fn union_bbox_returns_next_when_acc_is_none() {
    let result = union_bbox(None, bbox(1.0, 2.0, 3.0, 4.0));
    assert_eq!(result, bbox(1.0, 2.0, 3.0, 4.0));
}

#[test]
fn index_body_nodes_by_page_buckets_multi_page_node_under_every_covered_page() {
    let mut spanning = body_node("Table caption spanning pages.", 2, Some(bbox(5.0, 5.0, 15.0, 15.0)));
    spanning.page_end = Some(4);
    let single = body_node("Single page paragraph text.", 3, Some(bbox(1.0, 1.0, 2.0, 2.0)));
    let mut header = body_node("Running header text on page.", 3, Some(bbox(9.0, 9.0, 9.0, 9.0)));
    header.content_layer = ContentLayer::Header;

    let nodes = [spanning, single, header];
    let index = index_body_nodes_by_page(&nodes);

    assert_eq!(
        index.get(&2).map(|page| page.nodes.len()),
        Some(1),
        "page 2 must only see the spanning node"
    );
    assert_eq!(
        index.get(&3).map(|page| page.nodes.len()),
        Some(2),
        "page 3 must see both the spanning node and the single-page node, but not the header"
    );
    assert_eq!(
        index.get(&4).map(|page| page.nodes.len()),
        Some(1),
        "page 4 must only see the spanning node"
    );
    assert!(!index.contains_key(&1), "page 1 has no covering nodes");
}

#[test]
fn should_populate_node_ids_for_matching_nodes_and_leave_empty_when_no_node_matches() {
    let structure = DocumentStructure {
        nodes: vec![
            body_node_with_id(
                "node-1",
                "First paragraph on page one.",
                1,
                Some(bbox(10.0, 10.0, 50.0, 50.0)),
            ),
            body_node_with_id(
                "node-2",
                "Second paragraph on page one.",
                1,
                Some(bbox(60.0, 60.0, 120.0, 120.0)),
            ),
            body_node_with_id(
                "node-3",
                "Unrelated paragraph on page two.",
                2,
                Some(bbox(1.0, 1.0, 2.0, 2.0)),
            ),
        ],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![
        chunk_with_spans(
            "First paragraph on page one.\n\nSecond paragraph on page one.",
            vec![PageSpan { page: 1, bbox: None }],
        ),
        chunk_with_spans(
            "Completely different chunk text that matches no node.",
            vec![PageSpan { page: 2, bbox: None }],
        ),
    ];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(
        chunks[0].metadata.node_ids,
        vec!["node-1".to_string(), "node-2".to_string()],
        "chunk must reference exactly the ids of the nodes whose text it covers, in traversal order"
    );
    assert!(
        chunks[1].metadata.node_ids.is_empty(),
        "chunk matching no node must have empty node_ids"
    );
}

#[test]
fn should_dedupe_node_id_when_the_same_multi_page_node_matches_more_than_one_span() {
    let mut node = body_node_with_id(
        "node-multi",
        "Table caption spanning pages.",
        2,
        Some(bbox(5.0, 5.0, 15.0, 15.0)),
    );
    node.page_end = Some(3);
    let structure = DocumentStructure {
        nodes: vec![node],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "Table caption spanning pages.",
        vec![PageSpan { page: 2, bbox: None }, PageSpan { page: 3, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(
        chunks[0].metadata.node_ids,
        vec!["node-multi".to_string()],
        "a node spanning multiple pages must contribute its id once, not once per page"
    );
}

#[test]
fn should_populate_node_id_even_when_the_matching_node_has_no_bbox() {
    let structure = DocumentStructure {
        nodes: vec![body_node_with_id(
            "node-no-bbox",
            "Hello world, this is page one.",
            1,
            None,
        )],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "Hello world, this is page one.",
        vec![PageSpan { page: 1, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(chunks[0].metadata.page_spans[0].bbox, None);
    assert_eq!(chunks[0].metadata.node_ids, vec!["node-no-bbox".to_string()]);
}

fn node_with_content(id: &str, content: NodeContent, page: u32, bbox: Option<BoundingBox>) -> DocumentNode {
    DocumentNode {
        id: id.to_string(),
        content,
        parent: None,
        children: Vec::new(),
        content_layer: ContentLayer::Body,
        page: Some(page),
        page_end: None,
        bbox,
        annotations: Vec::new(),
        attributes: None,
    }
}

/// Regression test for #212: `Table { grid }` never contributed a bbox/node_id
/// because `node_text_for_matching` had no arm for it — matching per-cell now lets
/// a table-only chunk pick up its structure backlink and viewer highlight.
#[test]
fn should_populate_bbox_and_node_id_for_table_node_via_cell_text() {
    use crate::types::document_structure::{GridCell, TableGrid};

    let grid = TableGrid {
        rows: 1,
        cols: 2,
        cells: vec![
            GridCell {
                content: "Revenue by region".to_string(),
                row: 0,
                col: 0,
                row_span: 1,
                col_span: 1,
                is_header: true,
                bbox: None,
                heading_level: None,
                style_name: None,
            },
            GridCell {
                content: "Q3".to_string(),
                row: 0,
                col: 1,
                row_span: 1,
                col_span: 1,
                is_header: true,
                bbox: None,
                heading_level: None,
                style_name: None,
            },
        ],
    };
    let structure = DocumentStructure {
        nodes: vec![node_with_content(
            "table-1",
            NodeContent::Table { grid },
            1,
            Some(bbox(1.0, 1.0, 50.0, 50.0)),
        )],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "| Revenue by region | Q3 |\n|---|---|\n",
        vec![PageSpan { page: 1, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(chunks[0].metadata.page_spans[0].bbox, Some(bbox(1.0, 1.0, 50.0, 50.0)));
    assert_eq!(chunks[0].metadata.node_ids, vec!["table-1".to_string()]);
}

/// A table whose cell text does not appear in the chunk must not match.
#[test]
fn should_not_match_table_node_when_no_cell_text_found_in_chunk() {
    use crate::types::document_structure::{GridCell, TableGrid};

    let grid = TableGrid {
        rows: 1,
        cols: 1,
        cells: vec![GridCell {
            content: "Completely unrelated cell content".to_string(),
            row: 0,
            col: 0,
            row_span: 1,
            col_span: 1,
            is_header: false,
            bbox: None,
            heading_level: None,
            style_name: None,
        }],
    };
    let structure = DocumentStructure {
        nodes: vec![node_with_content(
            "table-2",
            NodeContent::Table { grid },
            1,
            Some(bbox(1.0, 1.0, 2.0, 2.0)),
        )],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "Some other paragraph entirely.",
        vec![PageSpan { page: 1, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(chunks[0].metadata.page_spans[0].bbox, None);
    assert!(chunks[0].metadata.node_ids.is_empty());
}

/// Regression test for #212: `Image { description }` never contributed a bbox/node_id.
#[test]
fn should_populate_bbox_and_node_id_for_image_node_via_description() {
    let structure = DocumentStructure {
        nodes: vec![node_with_content(
            "image-1",
            NodeContent::Image {
                description: Some("Diagram of the quarterly revenue funnel".to_string()),
                image_index: Some(0),
                src: None,
            },
            2,
            Some(bbox(4.0, 4.0, 40.0, 40.0)),
        )],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "![Diagram of the quarterly revenue funnel](img.png)",
        vec![PageSpan { page: 2, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(chunks[0].metadata.page_spans[0].bbox, Some(bbox(4.0, 4.0, 40.0, 40.0)));
    assert_eq!(chunks[0].metadata.node_ids, vec!["image-1".to_string()]);
}

/// An `Image` with no `description` has nothing to match against and must not
/// contribute a bbox/node_id (there is no fragment to search for).
#[test]
fn should_not_match_image_node_without_description() {
    let structure = DocumentStructure {
        nodes: vec![node_with_content(
            "image-2",
            NodeContent::Image {
                description: None,
                image_index: Some(0),
                src: None,
            },
            1,
            Some(bbox(1.0, 1.0, 2.0, 2.0)),
        )],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "Any content at all.",
        vec![PageSpan { page: 1, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(chunks[0].metadata.page_spans[0].bbox, None);
    assert!(chunks[0].metadata.node_ids.is_empty());
}

/// Regression test for #212: `Group { heading_text }` never contributed a bbox/node_id.
#[test]
fn should_populate_bbox_and_node_id_for_group_node_via_heading_text() {
    let structure = DocumentStructure {
        nodes: vec![node_with_content(
            "group-1",
            NodeContent::Group {
                label: None,
                heading_level: Some(2),
                heading_text: Some("Key-value configuration section".to_string()),
            },
            1,
            Some(bbox(1.0, 1.0, 20.0, 20.0)),
        )],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "## Key-value configuration section\n\nSetting: value",
        vec![PageSpan { page: 1, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(chunks[0].metadata.page_spans[0].bbox, Some(bbox(1.0, 1.0, 20.0, 20.0)));
    assert_eq!(chunks[0].metadata.node_ids, vec!["group-1".to_string()]);
}

/// Regression test for #212: `Slide { title }` never contributed a bbox/node_id.
#[test]
fn should_populate_bbox_and_node_id_for_slide_node_via_title() {
    let structure = DocumentStructure {
        nodes: vec![node_with_content(
            "slide-1",
            NodeContent::Slide {
                number: 3,
                title: Some("Roadmap for next quarter".to_string()),
            },
            3,
            Some(bbox(0.0, 0.0, 100.0, 100.0)),
        )],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "# Roadmap for next quarter\n\nSlide body text.",
        vec![PageSpan { page: 3, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(
        chunks[0].metadata.page_spans[0].bbox,
        Some(bbox(0.0, 0.0, 100.0, 100.0))
    );
    assert_eq!(chunks[0].metadata.node_ids, vec!["slide-1".to_string()]);
}

/// Regression test for #212: `DefinitionItem { term, definition }` never contributed
/// a bbox/node_id; both the term and the definition are checked as independent
/// candidate fragments since the rendered form rarely places them contiguously.
#[test]
fn should_populate_bbox_and_node_id_for_definition_item_via_term_or_definition() {
    let structure = DocumentStructure {
        nodes: vec![node_with_content(
            "def-1",
            NodeContent::DefinitionItem {
                term: "Throughput".to_string(),
                definition: "The number of requests processed per second".to_string(),
            },
            1,
            Some(bbox(2.0, 2.0, 30.0, 30.0)),
        )],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    // Chunk contains only the definition, not the term verbatim next to it.
    let mut chunks = vec![chunk_with_spans(
        "The number of requests processed per second is a key metric.",
        vec![PageSpan { page: 1, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(chunks[0].metadata.page_spans[0].bbox, Some(bbox(2.0, 2.0, 30.0, 30.0)));
    assert_eq!(chunks[0].metadata.node_ids, vec!["def-1".to_string()]);
}

/// Regression test for #212: `Admonition { title }` never contributed a bbox/node_id.
#[test]
fn should_populate_bbox_and_node_id_for_admonition_node_via_title() {
    let structure = DocumentStructure {
        nodes: vec![node_with_content(
            "admonition-1",
            NodeContent::Admonition {
                kind: "warning".to_string(),
                title: Some("Deprecated configuration option".to_string()),
            },
            1,
            Some(bbox(3.0, 3.0, 33.0, 33.0)),
        )],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "> **Deprecated configuration option**\n>\n> Use the new flag instead.",
        vec![PageSpan { page: 1, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(chunks[0].metadata.page_spans[0].bbox, Some(bbox(3.0, 3.0, 33.0, 33.0)));
    assert_eq!(chunks[0].metadata.node_ids, vec!["admonition-1".to_string()]);
}

/// A pure container node with no directly comparable text (`Quote`) must not match
/// anything, confirming the `_ => Vec::new()` fallback arm.
#[test]
fn should_not_match_pure_container_node_with_no_text() {
    let structure = DocumentStructure {
        nodes: vec![node_with_content(
            "quote-1",
            NodeContent::Quote,
            1,
            Some(bbox(1.0, 1.0, 2.0, 2.0)),
        )],
        source_format: None,
        relationships: Vec::new(),
        node_types: Vec::new(),
    };
    let mut chunks = vec![chunk_with_spans(
        "> Some quoted text that a Quote container cannot match on its own.",
        vec![PageSpan { page: 1, bbox: None }],
    )];

    populate_page_span_bboxes(&mut chunks, &structure);

    assert_eq!(chunks[0].metadata.page_spans[0].bbox, None);
    assert!(chunks[0].metadata.node_ids.is_empty());
}

/// One page holding every node is the worst case for the per-page node scan: each chunk is
/// matched against all of them.
#[test]
fn bbox_population_scales_linearly_when_all_nodes_share_one_page() {
    fn time_for(nodes: usize) -> f64 {
        let texts: Vec<String> = (0..nodes)
            .map(|i| format!("paragraph number {i} with some filler words"))
            .collect();
        let structure_nodes: Vec<DocumentNode> = texts
            .iter()
            .map(|t| body_node(t, 1, Some(bbox(0.0, 0.0, 1.0, 1.0))))
            .collect();
        let structure = DocumentStructure {
            nodes: structure_nodes,
            source_format: None,
            relationships: Vec::new(),
            node_types: Vec::new(),
        };
        let per_chunk = 8;
        let mut chunks: Vec<Chunk> = texts
            .chunks(per_chunk)
            .map(|group| chunk_with_spans(&group.join("\n\n"), vec![PageSpan { page: 1, bbox: None }]))
            .collect();
        let start = std::time::Instant::now();
        populate_page_span_bboxes(&mut chunks, &structure);
        start.elapsed().as_secs_f64() * 1e3
    }
    // ~keep Best of several runs: a single wall-clock sample flaked on loaded CI runners (macOS).
    let best = |nodes: usize| (0..5).map(|_| time_for(nodes)).fold(f64::INFINITY, f64::min);
    let small = best(1_000).max(0.05);
    let large = best(8_000);
    assert!(
        large / small < 24.0,
        "8x nodes took {:.1}x as long ({small:.2}ms -> {large:.2}ms)",
        large / small
    );
}

#[test]
fn trie_matching_selects_exactly_the_nodes_substring_matching_selects() {
    let texts: Vec<String> = (0..200)
        .map(|i| match i % 5 {
            0 => format!("  shared heading text {}  ", i % 7),
            1 => format!("short{i}"),
            2 => format!("unique paragraph body number {i} ends here"),
            3 => "shared heading text 3".to_string(),
            _ => format!("日本語の段落 {i} が続きます"),
        })
        .collect();
    let nodes: Vec<DocumentNode> = texts
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let mut node = body_node_with_id(&format!("n{i}"), t, 1, Some(bbox(i as f64, 0.0, i as f64 + 1.0, 1.0)));
            node.id = format!("n{i}");
            node
        })
        .collect();
    let with_trie = PageNodes::new(nodes.iter().collect());
    assert!(with_trie.trie.is_some());
    let brute = PageNodes {
        nodes: nodes.iter().collect(),
        trie: None,
    };
    for content in [
        "",
        "shared heading text 3 and shared heading text 4",
        "unique paragraph body number 12 ends here plus 日本語の段落 4 が続きます and short1",
        "xx unique paragraph body number 77 ends here",
        &texts.join("\n"),
    ] {
        let (fast_bbox, fast_ids) = match_page_nodes(content, &with_trie);
        let (slow_bbox, slow_ids) = match_page_nodes(content, &brute);
        assert_eq!(fast_ids, slow_ids, "ids for {content:?}");
        assert_eq!(
            fast_bbox.map(|b| (b.x0, b.y0, b.x1, b.y1)),
            slow_bbox.map(|b| (b.x0, b.y0, b.x1, b.y1))
        );
    }
}
