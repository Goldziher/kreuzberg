use super::*;

fn make_paragraph(text: &str, page: Option<u32>, index: u32) -> DocumentNode {
    let content = NodeContent::Paragraph { text: text.to_string() };
    DocumentNode {
        id: NodeId::generate(content.node_type_str(), text, page, index).to_string(),
        content,
        parent: None,
        children: vec![],
        content_layer: ContentLayer::Body,
        page,
        page_end: None,
        bbox: None,
        annotations: vec![],
        attributes: None,
    }
}

#[test]
fn test_empty_document_validates() {
    let doc = DocumentStructure::new();
    assert!(doc.validate().is_ok());
    assert!(doc.is_empty());
    assert_eq!(doc.len(), 0);
}

#[test]
fn test_single_node_validates() {
    let mut doc = DocumentStructure::new();
    doc.push_node(make_paragraph("Hello world", Some(1), 0));
    assert!(doc.validate().is_ok());
    assert_eq!(doc.len(), 1);
}

#[test]
fn test_parent_child_relationship() {
    let mut doc = DocumentStructure::new();

    let group_content = NodeContent::Group {
        label: None,
        heading_level: Some(1),
        heading_text: Some("Section 1".to_string()),
    };
    let group = DocumentNode {
        id: NodeId::generate("group", "Section 1", Some(1), 0).to_string(),
        content: group_content,
        parent: None,
        children: vec![],
        content_layer: ContentLayer::Body,
        page: Some(1),
        page_end: None,
        bbox: None,
        annotations: vec![],
        attributes: None,
    };
    let group_idx = doc.push_node(group);

    let child = make_paragraph("Child paragraph", Some(1), 1);
    let child_idx = doc.push_node(child);

    doc.add_child(group_idx, child_idx);

    assert!(doc.validate().is_ok());
    assert_eq!(doc.nodes[0].children.len(), 1);
    assert_eq!(doc.nodes[1].parent, Some(NodeIndex(0)));
}

#[test]
fn test_validation_catches_bad_parent() {
    let mut doc = DocumentStructure::new();
    let mut node = make_paragraph("Bad parent", Some(1), 0);
    node.parent = Some(NodeIndex(99));
    doc.push_node(node);

    assert!(doc.validate().is_err());
}

#[test]
fn test_validation_catches_inconsistent_parent_child() {
    let mut doc = DocumentStructure::new();

    let parent = DocumentNode {
        id: NodeId::generate("group", "", Some(1), 0).to_string(),
        content: NodeContent::Group {
            label: None,
            heading_level: None,
            heading_text: None,
        },
        parent: None,
        children: vec![],
        content_layer: ContentLayer::Body,
        page: Some(1),
        page_end: None,
        bbox: None,
        annotations: vec![],
        attributes: None,
    };
    doc.push_node(parent);

    let mut child = make_paragraph("Orphan child", Some(1), 1);
    child.parent = Some(NodeIndex(0));
    doc.push_node(child);

    assert!(doc.validate().is_err());
}

#[test]
fn test_validation_catches_bad_child() {
    let mut doc = DocumentStructure::new();

    let parent = DocumentNode {
        id: NodeId::generate("group", "", Some(1), 0).to_string(),
        content: NodeContent::Group {
            label: None,
            heading_level: None,
            heading_text: None,
        },
        parent: None,
        children: vec![NodeIndex(99)],
        content_layer: ContentLayer::Body,
        page: Some(1),
        page_end: None,
        bbox: None,
        annotations: vec![],
        attributes: None,
    };
    doc.push_node(parent);

    assert!(doc.validate().is_err());
}

#[test]
fn test_body_and_furniture_roots() {
    let mut doc = DocumentStructure::new();

    doc.push_node(make_paragraph("Body content", Some(1), 0));

    let mut header = make_paragraph("Page header", Some(1), 1);
    header.content_layer = ContentLayer::Header;
    doc.push_node(header);

    let mut footer = make_paragraph("Page footer", Some(1), 2);
    footer.content_layer = ContentLayer::Footer;
    doc.push_node(footer);

    assert!(doc.validate().is_ok());

    let body: Vec<_> = doc.body_roots().collect();
    assert_eq!(body.len(), 1);

    let furniture: Vec<_> = doc.furniture_roots().collect();
    assert_eq!(furniture.len(), 2);
}

#[test]
fn test_node_id_deterministic() {
    let id1 = NodeId::generate("paragraph", "Hello world", Some(1), 0);
    let id2 = NodeId::generate("paragraph", "Hello world", Some(1), 0);
    assert_eq!(id1, id2);

    let id3 = NodeId::generate("paragraph", "Different text", Some(1), 0);
    assert_ne!(id1, id3);

    let id4 = NodeId::generate("paragraph", "Hello world", Some(2), 0);
    assert_ne!(id1, id4);

    let id5 = NodeId::generate("heading", "Hello world", Some(1), 0);
    assert_ne!(id1, id5);

    let id6 = NodeId::generate("paragraph", "Hello world", Some(1), 1);
    assert_ne!(id1, id6);

    let id_none = NodeId::generate("paragraph", "Hello world", None, 0);
    let id_some_0 = NodeId::generate("paragraph", "Hello world", Some(0), 0);
    assert_ne!(id_none, id_some_0);
}

#[test]
fn test_node_content_text() {
    assert_text_content_variants();
    assert_non_text_content_variants();
}

fn assert_content_text(content: NodeContent, expected: Option<&str>) {
    assert_eq!(content.text(), expected);
}

fn assert_text_content_variants() {
    assert_content_text(
        NodeContent::Paragraph {
            text: "Hello".to_string(),
        },
        Some("Hello"),
    );
    assert_content_text(
        NodeContent::Title {
            text: "Title".to_string(),
        },
        Some("Title"),
    );
    assert_content_text(
        NodeContent::Heading {
            level: 1,
            text: "H1".to_string(),
        },
        Some("H1"),
    );
    assert_content_text(
        NodeContent::DefinitionItem {
            term: "Term".to_string(),
            definition: "Def".to_string(),
        },
        Some("Term"),
    );
    assert_content_text(
        NodeContent::Citation {
            key: "k".to_string(),
            text: "Text".to_string(),
        },
        Some("Text"),
    );
    assert_content_text(
        NodeContent::RawBlock {
            format: "html".to_string(),
            content: "<b>hi</b>".to_string(),
        },
        Some("<b>hi</b>"),
    );
}

fn assert_non_text_content_variants() {
    assert_content_text(NodeContent::PageBreak, None);
    assert_content_text(NodeContent::Quote, None);
    assert_content_text(
        NodeContent::Group {
            label: None,
            heading_level: None,
            heading_text: None,
        },
        None,
    );
    assert_content_text(
        NodeContent::Slide {
            number: 1,
            title: Some("Slide".to_string()),
        },
        None,
    );
    assert_content_text(NodeContent::DefinitionList, None);
    assert_content_text(
        NodeContent::Admonition {
            kind: "note".to_string(),
            title: None,
        },
        None,
    );
    assert_content_text(
        NodeContent::MetadataBlock {
            entries: vec![("k".to_string(), "v".to_string()).into()],
        },
        None,
    );
}

#[test]
fn test_new_node_type_str() {
    assert_eq!(NodeContent::Slide { number: 1, title: None }.node_type_str(), "slide");
    assert_eq!(NodeContent::DefinitionList.node_type_str(), "definition_list");
    assert_eq!(
        NodeContent::DefinitionItem {
            term: "t".to_string(),
            definition: "d".to_string()
        }
        .node_type_str(),
        "definition_item"
    );
    assert_eq!(
        NodeContent::Citation {
            key: "k".to_string(),
            text: "t".to_string()
        }
        .node_type_str(),
        "citation"
    );
    assert_eq!(
        NodeContent::Admonition {
            kind: "note".to_string(),
            title: None
        }
        .node_type_str(),
        "admonition"
    );
    assert_eq!(
        NodeContent::RawBlock {
            format: "html".to_string(),
            content: "x".to_string()
        }
        .node_type_str(),
        "raw_block"
    );
    assert_eq!(
        NodeContent::MetadataBlock { entries: vec![] }.node_type_str(),
        "metadata_block"
    );
}

#[test]
fn test_new_annotation_serde_roundtrip() {
    let ann = TextAnnotation {
        start: 0,
        end: 5,
        kind: AnnotationKind::Highlight,
    };
    let json = serde_json::to_string(&ann).expect("serialize");
    let de: TextAnnotation = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(de.kind, AnnotationKind::Highlight);

    let ann = TextAnnotation {
        start: 0,
        end: 5,
        kind: AnnotationKind::Color {
            value: "#ff0000".to_string(),
        },
    };
    let json = serde_json::to_string(&ann).expect("serialize");
    let de: TextAnnotation = serde_json::from_str(&json).expect("deserialize");
    match &de.kind {
        AnnotationKind::Color { value } => assert_eq!(value, "#ff0000"),
        _ => panic!("Expected Color"),
    }

    let ann = TextAnnotation {
        start: 0,
        end: 5,
        kind: AnnotationKind::FontSize {
            value: "12pt".to_string(),
        },
    };
    let json = serde_json::to_string(&ann).expect("serialize");
    let de: TextAnnotation = serde_json::from_str(&json).expect("deserialize");
    match &de.kind {
        AnnotationKind::FontSize { value } => assert_eq!(value, "12pt"),
        _ => panic!("Expected FontSize"),
    }

    let ann = TextAnnotation {
        start: 0,
        end: 5,
        kind: AnnotationKind::Custom {
            name: "bg-color".to_string(),
            value: Some("yellow".to_string()),
        },
    };
    let json = serde_json::to_string(&ann).expect("serialize");
    let de: TextAnnotation = serde_json::from_str(&json).expect("deserialize");
    match &de.kind {
        AnnotationKind::Custom { name, value } => {
            assert_eq!(name, "bg-color");
            assert_eq!(value.as_deref(), Some("yellow"));
        }
        _ => panic!("Expected Custom"),
    }
}

#[test]
fn test_new_node_content_serde_roundtrip() {
    let content = NodeContent::Slide {
        number: 3,
        title: Some("My Slide".to_string()),
    };
    let json = serde_json::to_value(&content).expect("serialize");
    assert_eq!(json.get("node_type").unwrap(), "slide");
    assert_eq!(json.get("number").unwrap(), 3);
    assert_eq!(json.get("title").unwrap(), "My Slide");

    let content = NodeContent::Citation {
        key: "doe2024".to_string(),
        text: "Doe (2024)".to_string(),
    };
    let json = serde_json::to_value(&content).expect("serialize");
    assert_eq!(json.get("node_type").unwrap(), "citation");
    assert_eq!(json.get("key").unwrap(), "doe2024");

    let content = NodeContent::MetadataBlock {
        entries: vec![
            ("From".to_string(), "alice@example.com".to_string()).into(),
            ("Subject".to_string(), "Hello".to_string()).into(),
        ],
    };
    let json = serde_json::to_value(&content).expect("serialize");
    assert_eq!(json.get("node_type").unwrap(), "metadata_block");
    let entries = json.get("entries").unwrap().as_array().unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn test_serde_roundtrip() {
    let mut doc = DocumentStructure::new();

    let group_content = NodeContent::Group {
        label: Some("section".to_string()),
        heading_level: Some(1),
        heading_text: Some("Introduction".to_string()),
    };
    let group = DocumentNode {
        id: NodeId::generate("group", "Introduction", Some(1), 0).to_string(),
        content: group_content,
        parent: None,
        children: vec![],
        content_layer: ContentLayer::Body,
        page: Some(1),
        page_end: None,
        bbox: Some(BoundingBox {
            x0: 10.0,
            y0: 20.0,
            x1: 500.0,
            y1: 50.0,
        }),
        annotations: vec![],
        attributes: None,
    };
    let group_idx = doc.push_node(group);

    let para_content = NodeContent::Paragraph {
        text: "Hello world".to_string(),
    };
    let para = DocumentNode {
        id: NodeId::generate("paragraph", "Hello world", Some(1), 1).to_string(),
        content: para_content,
        parent: None,
        children: vec![],
        content_layer: ContentLayer::Body,
        page: Some(1),
        page_end: None,
        bbox: None,
        annotations: vec![TextAnnotation {
            start: 0,
            end: 5,
            kind: AnnotationKind::Bold,
        }],
        attributes: None,
    };
    let para_idx = doc.push_node(para);
    doc.add_child(group_idx, para_idx);

    assert!(doc.validate().is_ok());

    let json = serde_json::to_string(&doc).expect("serialize");
    let deserialized: DocumentStructure = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(deserialized.len(), 2);
    assert!(deserialized.validate().is_ok());
    assert_eq!(deserialized.nodes[0].children.len(), 1);
    assert_eq!(deserialized.nodes[1].parent, Some(NodeIndex(0)));
}

#[test]
fn test_serde_node_type_tag() {
    let content = NodeContent::Heading {
        level: 2,
        text: "My Heading".to_string(),
    };
    let json = serde_json::to_value(&content).expect("serialize");

    assert_eq!(json.get("node_type").unwrap(), "heading");
    assert_eq!(json.get("level").unwrap(), 2);
    assert_eq!(json.get("text").unwrap(), "My Heading");
}

#[test]
fn test_serde_annotation_roundtrip() {
    let annotation = TextAnnotation {
        start: 10,
        end: 20,
        kind: AnnotationKind::Link {
            url: "https://example.com".to_string(),
            title: Some("Example".to_string()),
        },
    };

    let json = serde_json::to_string(&annotation).expect("serialize");
    let deserialized: TextAnnotation = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(deserialized.start, 10);
    assert_eq!(deserialized.end, 20);
    match &deserialized.kind {
        AnnotationKind::Link { url, title } => {
            assert_eq!(url, "https://example.com");
            assert_eq!(title.as_deref(), Some("Example"));
        }
        _ => panic!("Expected Link annotation"),
    }
}

#[test]
fn test_table_grid_serde() {
    let grid = TableGrid {
        rows: 2,
        cols: 3,
        cells: vec![
            GridCell {
                content: "Header 1".to_string(),
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
                content: "Cell 1".to_string(),
                row: 1,
                col: 0,
                row_span: 1,
                col_span: 1,
                is_header: false,
                bbox: None,
                heading_level: None,
                style_name: None,
            },
        ],
    };

    let json = serde_json::to_string(&grid).expect("serialize");
    let deserialized: TableGrid = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(deserialized.rows, 2);
    assert_eq!(deserialized.cols, 3);
    assert_eq!(deserialized.cells.len(), 2);
    assert!(deserialized.cells[0].is_header);
    assert!(!deserialized.cells[1].is_header);
}

#[test]
fn test_content_layer_default() {
    let layer: ContentLayer = Default::default();
    assert_eq!(layer, ContentLayer::Body);
}

#[test]
fn test_bounding_box_from_f32_tuple() {
    let bbox: BoundingBox = (10.5f32, 20.5f32, 100.5f32, 200.5f32).into();
    assert!((bbox.x0 - 10.5).abs() < f64::EPSILON);
    assert!((bbox.y0 - 20.5).abs() < f64::EPSILON);
    assert!((bbox.x1 - 100.5).abs() < f64::EPSILON);
    assert!((bbox.y1 - 200.5).abs() < f64::EPSILON);
}

#[test]
fn test_skip_serializing_empty_fields() {
    let node = make_paragraph("Simple", Some(1), 0);
    let json = serde_json::to_value(&node).expect("serialize");

    assert!(json.get("parent").is_none());
    assert!(json.get("children").is_none());
    assert!(json.get("page_end").is_none());
    assert!(json.get("bbox").is_none());
    assert!(json.get("annotations").is_none());
    assert!(json.get("attributes").is_none());

    assert!(json.get("id").is_some());
    assert_eq!(json.get("id").unwrap(), &serde_json::Value::String(node.id.to_string()));

    assert!(json.get("content").is_some());
    assert!(json.get("page").is_some());
}

#[test]
#[allow(deprecated)]
fn test_node_rendered_offset_is_unimplemented_stub() {
    let mut doc = DocumentStructure::new();
    doc.push_node(make_paragraph("Hello", Some(1), 0));
    assert_eq!(
        doc.node_rendered_offset(NodeIndex(0)),
        None,
        "node_rendered_offset is a documented stub (#1294/#1295) and must always return None"
    );
}

#[test]
fn test_node_id_serializes_as_plain_string() {
    let id = NodeId::generate("paragraph", "Hello", Some(1), 0);
    let json = serde_json::to_value(&id).expect("serialize");
    assert!(
        json.is_string(),
        "NodeId must serialize as a bare string, got: {json:?}"
    );
}

#[test]
fn test_node_id_stable_across_generations() {
    let id_a = NodeId::generate("paragraph", "Hello world", Some(3), 5);
    let id_b = NodeId::generate("paragraph", "Hello world", Some(3), 5);
    assert_eq!(id_a, id_b);
    assert_eq!(id_a.to_string(), id_b.to_string());
}

#[test]
fn test_node_id_unique_for_duplicate_content_at_different_positions() {
    let id_0 = NodeId::generate("paragraph", "Repeated", Some(1), 0);
    let id_1 = NodeId::generate("paragraph", "Repeated", Some(1), 1);
    assert_ne!(
        id_0, id_1,
        "duplicate content at different indices must have distinct ids"
    );
}

#[test]
fn test_node_id_present_in_full_document_json() {
    let mut doc = DocumentStructure::new();
    doc.push_node(make_paragraph("First", Some(1), 0));
    doc.push_node(make_paragraph("Repeated", Some(1), 1));
    doc.push_node(make_paragraph("Repeated", Some(1), 2));

    let json = serde_json::to_value(&doc).expect("serialize");
    let ids: Vec<String> = json["nodes"]
        .as_array()
        .expect("nodes array")
        .iter()
        .map(|n| n["id"].as_str().expect("id present as string").to_string())
        .collect();

    assert_eq!(ids.len(), 3);
    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(
        unique.len(),
        3,
        "all node ids in one document must be unique, got: {ids:?}"
    );
}

#[test]
fn should_roundtrip_issue_2123_math_without_changing_annotation_enum_shape() {
    for (display, value) in [(false, "inline"), (true, "display")] {
        let kind = AnnotationKind::math(display);
        assert_eq!(kind.math_display(), Some(display));
        let json = serde_json::to_value(&kind).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"annotation_type":"custom", "name":"xberg.math", "value":value})
        );
        assert_eq!(serde_json::from_value::<AnnotationKind>(json).unwrap(), kind);
    }
    for (name, value) in [
        ("math", Some("inline")),
        ("xberg.math", None),
        ("xberg.math", Some("false")),
        ("xberg.Math", Some("display")),
    ] {
        assert_eq!(
            AnnotationKind::Custom {
                name: name.into(),
                value: value.map(str::to_string)
            }
            .math_display(),
            None
        );
    }
    assert_eq!(AnnotationKind::Bold.math_display(), None);
}
