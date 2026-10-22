//! Tests for the HTML structure walker, split out of `structure.rs` to keep that
//! file under the line-count limit.

use super::*;
use crate::types::document_structure::{AnnotationKind, NodeContent, NodeIndex};

/// Indices of every `List` node, in document order.
fn list_node_indices(doc: &DocumentStructure) -> Vec<usize> {
    doc.nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| matches!(node.content, NodeContent::List { .. }))
        .map(|(i, _)| i)
        .collect()
}

/// Texts of the `ListItem` children of the list node at `list_idx`, in child order.
fn list_item_texts(doc: &DocumentStructure, list_idx: usize) -> Vec<String> {
    doc.nodes[list_idx]
        .children
        .iter()
        .map(|child| match &doc.nodes[child.0 as usize].content {
            NodeContent::ListItem { text } => text.clone(),
            other => panic!("expected a ListItem child of list node {list_idx}, got {other:?}"),
        })
        .collect()
}

/// Texts of every `Paragraph` node, in document order.
fn paragraph_texts(doc: &DocumentStructure) -> Vec<String> {
    doc.nodes
        .iter()
        .filter_map(|node| match &node.content {
            NodeContent::Paragraph { text } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn test_headings() {
    let html = "<h1>Title</h1><h2>Subtitle</h2>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    assert_eq!(doc.body_roots().count(), 1);
}

#[test]
fn test_paragraphs() {
    let html = "<p>First paragraph.</p><p>Second paragraph.</p>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    assert_eq!(doc.body_roots().count(), 2);
}

#[test]
fn test_bold_annotation() {
    let html = "<p>Hello <strong>world</strong>!</p>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());

    let para = &doc.nodes[0];
    if let NodeContent::Paragraph { ref text } = para.content {
        assert_eq!(text, "Hello world!");
    } else {
        panic!("Expected paragraph, got {:?}", para.content);
    }
    assert_eq!(para.annotations.len(), 1);
    assert_eq!(para.annotations[0].kind, AnnotationKind::Bold);
    assert_eq!(para.annotations[0].start, 6);
    assert_eq!(para.annotations[0].end, 11);
}

#[test]
fn test_italic_annotation() {
    let html = "<p><em>italic</em> text</p>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    let para = &doc.nodes[0];
    assert_eq!(para.annotations.len(), 1);
    assert_eq!(para.annotations[0].kind, AnnotationKind::Italic);
    assert_eq!(para.annotations[0].start, 0);
    assert_eq!(para.annotations[0].end, 6);
}

#[test]
fn test_link_annotation() {
    let html = r#"<p>Click <a href="https://example.com" title="Example">here</a>.</p>"#;
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    let para = &doc.nodes[0];
    assert_eq!(para.annotations.len(), 1);
    match &para.annotations[0].kind {
        AnnotationKind::Link { url, title } => {
            assert_eq!(url, "https://example.com");
            assert_eq!(title.as_deref(), Some("Example"));
        }
        other => panic!("Expected Link annotation, got {:?}", other),
    }
}

#[test]
fn test_code_block() {
    let html = r#"<pre><code class="language-rust">fn main() {}</code></pre>"#;
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    let node = &doc.nodes[0];
    match &node.content {
        NodeContent::Code { text, language } => {
            assert_eq!(text, "fn main() {}");
            assert_eq!(language.as_deref(), Some("rust"));
        }
        other => panic!("Expected Code, got {:?}", other),
    }
}

#[test]
#[cfg(feature = "office")]
fn test_math_converts_to_latex_formula_node() {
    let html = r#"<p>Before</p><math xmlns="http://www.w3.org/1998/Math/MathML"><mfrac><mn>1</mn><mn>2</mn></mfrac></math><p>After</p>"#;
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());

    let formula = doc
        .nodes
        .iter()
        .find_map(|node| match &node.content {
            NodeContent::Formula { text } => Some(text.clone()),
            _ => None,
        })
        .expect("expected a Formula node");
    assert_eq!(formula, "\\frac{1}{2}");

    assert!(
        doc.nodes
            .iter()
            .all(|node| !format!("{:?}", node.content).contains("mfrac")),
        "raw MathML tag names must not leak into any node"
    );
}

#[test]
fn test_unordered_list() {
    let html = "<ul><li>One</li><li>Two</li><li>Three</li></ul>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    assert_eq!(doc.len(), 4);
    match &doc.nodes[0].content {
        NodeContent::List { ordered } => assert!(!ordered),
        other => panic!("Expected List, got {:?}", other),
    }
    assert_eq!(doc.nodes[0].children.len(), 3);
}

/// Regression test for task #719: a `<ul>`/`<ol>` start tag only flushes the pending
/// paragraph buffer, not the pending list-item buffer. When a nested list opens while
/// the parent `<li>` still has unflushed text, that text is later flushed against
/// `list_stack.last()`, which by then points at the freshly-pushed *inner* list — so the
/// parent item is misattributed one level too deep, shifting every intermediate item down
/// and leaving the outermost list empty.
#[test]
fn test_nested_list_item_attaches_to_correct_list_level() {
    let html = "<ul><li>L1<ul><li>L2<ul><li>L3</li></ul></li></ul></li></ul>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());

    // Three List nodes and three ListItem nodes, six total.
    assert_eq!(doc.len(), 6);

    let lists: Vec<usize> = doc
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| matches!(n.content, NodeContent::List { .. }))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(lists.len(), 3, "expected exactly 3 List nodes, got {lists:?}");

    let item_text = |idx: NodeIndex| match &doc.nodes[idx.0 as usize].content {
        NodeContent::ListItem { text } => text.clone(),
        other => panic!("Expected ListItem at {idx:?}, got {other:?}"),
    };

    // Each of the three list levels must hold exactly one item, and that item's text
    // must match its own nesting depth (L1 in the outermost list, L2 in the middle
    // list, L3 in the innermost list).
    for (list_idx, expected_text) in lists.iter().zip(["L1", "L2", "L3"]) {
        let list_node = &doc.nodes[*list_idx];
        assert_eq!(
            list_node.children.len(),
            1,
            "list node {list_idx} should have exactly 1 item, got {:?}",
            list_node.children
        );
        assert_eq!(item_text(list_node.children[0]), expected_text);
    }
}

/// Regression test for task #721: content that resumes in the outer `<li>` after a
/// sublist has closed must stay list-item content.
///
/// `in_list_item` is a single bool, so the inner list's start and end handlers both
/// clear it while the outer item is still open; the trailing text then misses the
/// list-item branch of `handle_text` and lands in the paragraph buffer instead.
///
/// Against the unfixed code the outer list holds only `["before text"]` and node 4 is
/// a `Paragraph` with text `"after text"`.
#[test]
fn test_text_after_sublist_returns_to_outer_list_item() {
    let html = "<ul><li>before text<ul><li>child</li></ul>after text</li></ul>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    assert_eq!(doc.len(), 5, "expected 2 List + 3 ListItem nodes");

    let lists = list_node_indices(&doc);
    assert_eq!(lists.len(), 2, "expected exactly 2 List nodes, got {lists:?}");

    assert_eq!(
        list_item_texts(&doc, lists[0]),
        vec!["before text".to_string(), "after text".to_string()],
        "text following the sublist must become a sibling item of the outer list"
    );
    assert_eq!(list_item_texts(&doc, lists[1]), vec!["child".to_string()]);

    assert!(
        paragraph_texts(&doc).is_empty(),
        "trailing list-item text must not be emitted as a Paragraph, got {:?}",
        paragraph_texts(&doc)
    );
}

/// Task #721, three levels deep: each trailing run must rejoin the level whose item is
/// still open, not the level it was nested under.
///
/// Against the unfixed code both trailing runs land in the same paragraph buffer and
/// are emitted as a single `Paragraph` with the concatenated text `"after L2after L1"`
/// (no separator — the two text nodes are adjacent once the tags between them are
/// consumed), the outer list holds only `["L1"]` and the middle list only `["L2"]`.
#[test]
fn test_trailing_text_after_sublist_rejoins_its_own_level() {
    let html = "<ol><li>L1<ol><li>L2<ol><li>L3</li></ol>after L2</li></ol>after L1</li></ol>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    assert_eq!(doc.len(), 8, "expected 3 List + 5 ListItem nodes");

    let lists = list_node_indices(&doc);
    assert_eq!(lists.len(), 3, "expected exactly 3 List nodes, got {lists:?}");
    for list_idx in &lists {
        assert!(
            matches!(doc.nodes[*list_idx].content, NodeContent::List { ordered: true }),
            "list node {list_idx} must stay ordered"
        );
    }

    assert_eq!(
        list_item_texts(&doc, lists[0]),
        vec!["L1".to_string(), "after L1".to_string()]
    );
    assert_eq!(
        list_item_texts(&doc, lists[1]),
        vec!["L2".to_string(), "after L2".to_string()]
    );
    assert_eq!(list_item_texts(&doc, lists[2]), vec!["L3".to_string()]);

    assert!(
        paragraph_texts(&doc).is_empty(),
        "no trailing run may become a Paragraph, got {:?}",
        paragraph_texts(&doc)
    );
}

/// Task #721 on pretty-printed markup, which is what DOCX/ODT/email HTML actually looks
/// like. Two things must hold at once: the whitespace between `</ul>` and `</li>` in the
/// first item must not mint an empty list item now that it is buffered as item text, and
/// the real trailing text `E` in the second item must become an item of the outer list.
///
/// Against the unfixed code the outer list holds only `["A", "C"]` and a `Paragraph` with
/// text `"E"` exists.
#[test]
fn test_pretty_printed_sublist_keeps_trailing_text_without_empty_items() {
    let html = r#"<ul>
  <li>A
<ul>
  <li>B</li>
</ul>
  </li>
  <li>C
<ul>
  <li>D</li>
</ul>
E
  </li>
</ul>"#;
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());

    let lists = list_node_indices(&doc);
    assert_eq!(lists.len(), 3, "expected exactly 3 List nodes, got {lists:?}");

    assert_eq!(
        list_item_texts(&doc, lists[0]),
        vec!["A".to_string(), "C".to_string(), "E".to_string()],
        "trailing text after the second sublist must join the outer list"
    );
    assert_eq!(list_item_texts(&doc, lists[1]), vec!["B".to_string()]);
    assert_eq!(list_item_texts(&doc, lists[2]), vec!["D".to_string()]);

    assert!(
        paragraph_texts(&doc).is_empty(),
        "trailing list-item text must not be emitted as a Paragraph, got {:?}",
        paragraph_texts(&doc)
    );
    assert_eq!(
        doc.len(),
        8,
        "whitespace-only content between </ul> and </li> must not mint an empty ListItem"
    );
}

/// Regression test for task #727: `flush_list_item` dropped `self.annotations` on the
/// floor, so inline formatting inside an `<li>` never reached the `ListItem` node.
///
/// Against the unfixed code the item's `annotations` is empty.
#[test]
fn test_list_item_keeps_its_inline_annotations() {
    let html = "<ul><li>alpha <strong>bold</strong></li></ul>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());

    let item = doc
        .nodes
        .iter()
        .find(|node| matches!(node.content, NodeContent::ListItem { .. }))
        .expect("expected a ListItem node");
    assert!(
        matches!(&item.content, NodeContent::ListItem { text } if text == "alpha bold"),
        "unexpected item text: {:?}",
        item.content
    );
    assert_eq!(
        item.annotations.len(),
        1,
        "the item's <strong> must survive the flush, got {:?}",
        item.annotations
    );
    assert_eq!(item.annotations[0].kind, AnnotationKind::Bold);
    assert_eq!(item.annotations[0].start, 6);
    assert_eq!(item.annotations[0].end, 10);
}

/// Task #727, the worse half: because the annotation buffer was never cleared either,
/// a list item's annotations stayed pending and were claimed by the next node that
/// flushed, landing on unrelated text at offsets that mean nothing there.
///
/// Against the unfixed code the trailing paragraph carries `Bold { start: 6, end: 10 }`
/// — the offsets of "bold" inside the list item, which in "Trailing sentence text."
/// mark "ng s".
#[test]
fn test_list_item_annotations_do_not_leak_into_the_next_paragraph() {
    let html = "<ul><li>alpha <strong>bold</strong></li></ul>Trailing sentence text.";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());

    let para = doc
        .nodes
        .iter()
        .find(|node| matches!(&node.content, NodeContent::Paragraph { text } if text == "Trailing sentence text."))
        .expect("expected the trailing text to become a Paragraph");
    assert!(
        para.annotations.is_empty(),
        "list-item formatting must not be re-attributed to the following paragraph, got {:?}",
        para.annotations
    );
}

/// Task #727, half-open spans: `pop_inline` measures against whichever buffer is live,
/// so an inline element left unclosed when the item flushed would close against the
/// *next* buffer. Clearing the inline stack alongside the annotation buffer (what
/// `flush_paragraph` already does) is what stops it.
///
/// Against the unfixed code the trailing paragraph carries `Bold { start: 6, end: 17 }`,
/// i.e. "ng sentence" of "Trailing sentence" rendered bold.
#[test]
fn test_unclosed_inline_in_a_list_item_does_not_annotate_later_text() {
    let html = "<ul><li>alpha <strong>bold</li></ul>Trailing sentence</strong>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());

    let para = doc
        .nodes
        .iter()
        .find(|node| matches!(&node.content, NodeContent::Paragraph { text } if text == "Trailing sentence"))
        .expect("expected the trailing text to become a Paragraph");
    assert!(
        para.annotations.is_empty(),
        "an inline span left open in a list item must not close against later text, got {:?}",
        para.annotations
    );
}

/// Regression test for task #728: `push_list` parents through the section/container
/// stack, so a `<ul>` nested inside an `<li>` became a root-level sibling of the outer
/// list instead of a child of the item containing it.
///
/// Against the unfixed code the inner list's `parent` is `None` and
/// `body_roots().count()` is 2.
#[test]
fn test_sublist_becomes_a_child_of_its_list_item() {
    let html = "<ul><li>parent<ul><li>child</li></ul>tail</li></ul>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());

    let lists = list_node_indices(&doc);
    assert_eq!(lists.len(), 2, "expected exactly 2 List nodes, got {lists:?}");
    assert_eq!(
        list_item_texts(&doc, lists[0]),
        vec!["parent".to_string(), "tail".to_string()]
    );
    assert_eq!(list_item_texts(&doc, lists[1]), vec!["child".to_string()]);

    let parent_item = doc.nodes[lists[0]].children[0];
    assert_eq!(
        doc.nodes[lists[1]].parent,
        Some(parent_item),
        "the sublist must hang off the <li> it is written inside, not off the document root"
    );
    assert_eq!(
        doc.nodes[parent_item.0 as usize].children,
        vec![NodeIndex(lists[1] as u32)],
        "the containing item must own the sublist"
    );
    assert_eq!(doc.body_roots().count(), 1, "only the outer list may be a root node");
}

/// Task #728 with no text before the sublist: there is no `ListItem` to parent under,
/// and minting an empty one is explicitly unwanted (see
/// `test_pretty_printed_sublist_keeps_trailing_text_without_empty_items`). The sublist
/// falls back to the enclosing `List` so it still stays inside the list subtree.
///
/// Against the unfixed code the inner list's `parent` is `None`.
#[test]
fn test_textless_item_sublist_stays_inside_the_outer_list() {
    let html = "<ul><li>first</li><li><ul><li>child</li></ul></li></ul>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());

    let lists = list_node_indices(&doc);
    assert_eq!(lists.len(), 2, "expected exactly 2 List nodes, got {lists:?}");
    assert_eq!(
        doc.nodes[lists[1]].parent,
        Some(NodeIndex(lists[0] as u32)),
        "a sublist in a text-less item must not become a root-level sibling"
    );
    assert_eq!(doc.body_roots().count(), 1, "only the outer list may be a root node");
}

#[test]
fn test_ordered_list() {
    let html = "<ol><li>First</li><li>Second</li></ol>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    match &doc.nodes[0].content {
        NodeContent::List { ordered } => assert!(ordered),
        other => panic!("Expected List, got {:?}", other),
    }
}

#[test]
fn test_table() {
    let html = "<table><tr><th>Name</th><th>Age</th></tr><tr><td>Alice</td><td>30</td></tr></table>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    match &doc.nodes[0].content {
        NodeContent::Table { grid } => {
            assert_eq!(grid.rows, 2);
            assert_eq!(grid.cols, 2);
        }
        other => panic!("Expected Table, got {:?}", other),
    }
}

#[test]
fn test_nested_table_is_flattened_into_the_enclosing_cell() {
    let html = "<table><tr><td>A1</td><td>A2</td></tr><tr><td><table><tr><td>N1</td><td>N2</td></tr></table></td><td>B2</td></tr><tr><td>C1</td><td>C2</td></tr></table>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    assert_eq!(doc.nodes.len(), 1, "got {:?}", doc.nodes);
    match &doc.nodes[0].content {
        NodeContent::Table { grid } => {
            assert_eq!(grid.rows, 3);
            assert_eq!(grid.cols, 2);
            let texts: Vec<&str> = grid.cells.iter().map(|c| c.content.as_str()).collect();
            assert!(texts.contains(&"A1"), "got {texts:?}");
            assert!(texts.contains(&"C2"), "got {texts:?}");
            assert!(
                texts.iter().any(|t| t.contains("N1") && t.contains("N2")),
                "got {texts:?}"
            );
        }
        other => panic!("Expected Table, got {:?}", other),
    }
}

#[test]
fn test_heading_line_breaks_become_newlines_without_sentinels() {
    let html = "<h2><br/><br/>CHAPTER I.</h2><h1>PRIDE<br/>and<br/>PREJUDICE</h1>";
    let doc = build_document_structure(html);
    let headings: Vec<String> = doc
        .nodes
        .iter()
        .filter_map(|node| match &node.content {
            NodeContent::Heading { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(headings, vec!["CHAPTER I.", "PRIDE\nand\nPREJUDICE"]);
    assert!(headings.iter().all(|h| !h.contains('\x01')));
}

#[test]
fn test_blockquote() {
    let html = "<blockquote><p>Quoted text.</p></blockquote>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    assert_eq!(doc.body_roots().count(), 1);
    let quote = &doc.nodes[0];
    assert!(matches!(quote.content, NodeContent::Quote));
    assert_eq!(quote.children.len(), 1);
}

#[test]
fn test_blockquote_with_divs() {
    let html = r#"<div>Before</div>
<blockquote><div><div>Line one</div><div>Line two</div></div></blockquote>
<div>After</div>"#;
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok(), "validate: {:?}", doc.validate());

    let roots: Vec<_> = doc.body_roots().collect();
    println!("=== ALL NODES ===");
    for (i, node) in doc.nodes.iter().enumerate() {
        println!(
            "  [{}] {:?} parent={:?} children={:?}",
            i, node.content, node.parent, node.children
        );
    }

    let quote_idx = doc.nodes.iter().position(|n| matches!(n.content, NodeContent::Quote));
    assert!(
        quote_idx.is_some(),
        "Should have a Quote node. Roots: {:?}",
        roots.len()
    );
    let quote = &doc.nodes[quote_idx.unwrap()];
    assert!(
        !quote.children.is_empty(),
        "Quote should have children with div content"
    );

    let child_texts: Vec<_> = quote
        .children
        .iter()
        .filter_map(|ci| match &doc.nodes[ci.0 as usize].content {
            NodeContent::Paragraph { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(child_texts.contains(&"Line one"), "Quote children: {:?}", child_texts);
    assert!(child_texts.contains(&"Line two"), "Quote children: {:?}", child_texts);
}

#[test]
fn test_image() {
    let html = r#"<img src="photo.jpg" alt="A photo">"#;
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    match &doc.nodes[0].content {
        NodeContent::Image { description, .. } => {
            assert_eq!(description.as_deref(), Some("A photo"));
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[test]
fn test_mixed_inline_formatting() {
    let html = "<p><strong>bold</strong> and <em>italic</em> and <code>code</code></p>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    let para = &doc.nodes[0];
    assert_eq!(para.annotations.len(), 3);
}

#[test]
fn test_css_class_attribute() {
    let html = r#"<p class="intro highlight">Styled paragraph.</p>"#;
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    let node = &doc.nodes[0];
    let attrs = node.attributes.as_ref().expect("attributes should be set");
    assert_eq!(attrs.get("class").unwrap(), "intro highlight");
}

#[test]
fn test_entities_decoded() {
    let html = "<p>Caf&eacute; &amp; Restaurant</p>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    let para = &doc.nodes[0];
    if let NodeContent::Paragraph { ref text } = para.content {
        assert!(text.contains("Caf\u{00E9}"), "eacute should be decoded");
        assert!(text.contains('&'), "amp should be decoded to &");
        assert!(text.contains("Restaurant"));
    } else {
        panic!("Expected paragraph");
    }
}

#[test]
fn test_nested_headings_structure() {
    let html = "<h1>Top</h1><p>Intro</p><h2>Sub</h2><p>Detail</p><h1>Next</h1><p>More</p>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    assert_eq!(doc.body_roots().count(), 2);
}

#[test]
fn test_source_format_set() {
    let html = "<p>test</p>";
    let doc = build_document_structure(html);
    assert_eq!(doc.source_format.as_deref(), Some("html"));
}

#[test]
fn test_empty_html() {
    let doc = build_document_structure("");
    assert!(doc.validate().is_ok());
    assert!(doc.is_empty());
}

#[test]
fn test_whitespace_only() {
    let doc = build_document_structure("   \n\t  ");
    assert!(doc.validate().is_ok());
    assert!(doc.is_empty());
}

#[test]
fn test_script_becomes_raw_block() {
    let html = "<script>var x = 1;</script><p>Content</p>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    assert_eq!(doc.body_roots().count(), 2);
    match &doc.nodes[0].content {
        NodeContent::RawBlock { format, content } => {
            assert_eq!(format, "script");
            assert!(content.contains("var x"));
        }
        other => panic!("Expected RawBlock, got {:?}", other),
    }
}

#[test]
fn test_head_title_is_not_body_text() {
    let html = "<html><head><title>Page name</title></head><body><p>Content</p>\
                <svg><title>Sales by quarter</title><rect width=\"1\" height=\"1\"/></svg></body></html>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    let texts: Vec<&str> = doc.nodes.iter().filter_map(|node| node.content.text()).collect();
    assert_eq!(texts, vec!["Content", "Sales by quarter"]);
}

#[test]
fn should_skip_many_unclosed_elements_in_linear_time() {
    // A search of the whole rest of the document per unclosed opener would take billions of steps ~keep
    for opener in ["<script>", "<style>", "<title>", "<video>", "<audio>", "<math>"] {
        let html = format!(
            "<html><head>{}</head><body><p>end</p></body></html>",
            format!("{opener}x ").repeat(20_000)
        );
        let doc = build_document_structure(&html);
        let text: String = doc.nodes.iter().filter_map(|node| node.content.text()).collect();
        assert_eq!(text.matches('x').count(), 20_000, "{opener}");
        assert!(text.ends_with("end"), "{opener}: {text:?}");
    }
}

#[test]
fn test_strikethrough_annotation() {
    let html = "<p>Some <del>deleted</del> text</p>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    let para = &doc.nodes[0];
    assert_eq!(para.annotations.len(), 1);
    assert_eq!(para.annotations[0].kind, AnnotationKind::Strikethrough);
}

#[test]
fn test_inline_code_annotation() {
    let html = "<p>Use <code>println!</code> to print</p>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    let para = &doc.nodes[0];
    assert_eq!(para.annotations.len(), 1);
    assert_eq!(para.annotations[0].kind, AnnotationKind::Code);
}

#[test]
fn test_underline_annotation() {
    let html = "<p><u>underlined</u></p>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    let para = &doc.nodes[0];
    assert_eq!(para.annotations.len(), 1);
    assert_eq!(para.annotations[0].kind, AnnotationKind::Underline);
}

#[test]
fn test_unclosed_tags() {
    let html = "<p>Hello <strong>bold text</p><p>Next paragraph</p>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    assert!(!doc.is_empty());
}

#[test]
fn test_nested_same_tags() {
    let html = "<p><strong>outer <strong>inner</strong> text</strong></p>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    let para = &doc.nodes[0];
    assert!(!para.annotations.is_empty());
}

#[test]
fn test_self_closing_tags() {
    let html = "<p>Before<br/>After</p><hr/><img src='x.png' alt='photo'/>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    assert!(doc.len() >= 2);
}

#[test]
fn test_nested_blockquotes() {
    let html = "<blockquote><p>Outer</p><blockquote><p>Inner</p></blockquote></blockquote>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    assert_eq!(doc.body_roots().count(), 1);
    let outer = &doc.nodes[0];
    assert!(matches!(outer.content, NodeContent::Quote));
    assert!(
        outer.children.len() >= 2,
        "Outer quote should have paragraph + inner quote"
    );
}

#[test]
fn test_numeric_entity_decoding() {
    let html = "<p>&#169; and &#x2014;</p>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    let para = &doc.nodes[0];
    if let NodeContent::Paragraph { ref text } = para.content {
        assert!(
            text.contains('\u{00A9}'),
            "decimal entity should decode to copyright sign"
        );
        assert!(text.contains('\u{2014}'), "hex entity should decode to em dash");
    } else {
        panic!("Expected paragraph");
    }
}

#[test]
fn test_table_missing_cells() {
    let html = "<table><tr><td>A</td><td>B</td><td>C</td></tr><tr><td>X</td></tr></table>";
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    match &doc.nodes[0].content {
        NodeContent::Table { grid } => {
            assert_eq!(grid.rows, 2);
            assert!(grid.cols >= 1);
        }
        other => panic!("Expected Table, got {:?}", other),
    }
}

#[test]
fn test_attr_extraction_no_false_match() {
    assert_eq!(
        extract_attr(r#"subclass="wrong" class="right""#, "class"),
        Some("right")
    );
    assert_eq!(extract_attr(r#"dataclass="wrong""#, "class"), None);
}

#[test]
fn test_complex_document() {
    let html = r#"
    <html>
    <body>
        <h1>Title</h1>
        <p>Introduction with <strong>bold</strong> and <em>italic</em>.</p>
        <h2>Section 1</h2>
        <p>Content of section 1.</p>
        <ul>
            <li>Item A</li>
            <li>Item B</li>
        </ul>
        <h2>Section 2</h2>
        <pre><code class="language-python">print("hello")</code></pre>
        <table>
            <tr><th>Name</th><th>Value</th></tr>
            <tr><td>Key</td><td>123</td></tr>
        </table>
        <blockquote>
            <p>A famous quote.</p>
        </blockquote>
    </body>
    </html>
    "#;
    let doc = build_document_structure(html);
    assert!(doc.validate().is_ok());
    assert_eq!(doc.body_roots().count(), 1);
    assert!(doc.len() > 10, "Complex doc should have many nodes, got {}", doc.len());
}

/// Regression test for issue #127: `flush_definition_item`'s `dd` branch checks
/// `self.in_dd`, so it must run before the `</dd>` close-tag handler clears that flag —
/// otherwise the definition item is silently dropped and only an empty `DefinitionList`
/// marker node is produced.
#[test]
fn test_dl_dt_dd_produces_definition_item_node() {
    let html = r#"<html><body><h1>Glossary</h1><dl><dt>DEFTERM</dt><dd>DEFDESCRIPTION explaining the term.</dd></dl></body></html>"#;
    let doc = build_document_structure(html);
    let item = doc
        .nodes
        .iter()
        .find_map(|n| match &n.content {
            NodeContent::DefinitionItem { term, definition } => Some((term.clone(), definition.clone())),
            _ => None,
        })
        .expect("expected a DefinitionItem node");
    assert_eq!(item.0, "DEFTERM");
    assert_eq!(item.1, "DEFDESCRIPTION explaining the term.");
}
