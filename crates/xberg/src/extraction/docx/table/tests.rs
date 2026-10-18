use super::*;

#[test]
fn test_parse_table_properties_full() {
    let xml = r#"<w:tblPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:tblStyle w:val="TableGrid"/>
        <w:tblW w:w="5000" w:type="dxa"/>
        <w:jc w:val="center"/>
        <w:tblLayout w:type="fixed"/>
        <w:tblLook w:val="0460"/>
        <w:tblBorders>
            <w:top w:val="single" w:sz="12" w:color="000000" w:space="0"/>
            <w:bottom w:val="single" w:sz="12" w:color="000000" w:space="0"/>
        </w:tblBorders>
        <w:tblCellMar>
            <w:top w:w="0" w:type="dxa"/>
            <w:left w:w="108" w:type="dxa"/>
        </w:tblCellMar>
        <w:tblInd w:w="108" w:type="dxa"/>
    </w:tblPr>"#;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    reader.read_event_into(&mut buf).unwrap();
    buf.clear();

    let mut budget = SecurityBudget::with_defaults();
    let props = parse_table_properties(&mut reader, &mut budget).unwrap();

    assert_eq!(props.style_id, Some("TableGrid".to_string()));
    assert_eq!(
        props.width,
        Some(TableWidth {
            value: 5000,
            width_type: "dxa".to_string()
        })
    );
    assert_eq!(props.alignment, Some("center".to_string()));
    assert_eq!(props.layout, Some("fixed".to_string()));
    assert!(props.look.is_some());
    assert!(props.borders.is_some());
    assert!(props.cell_margins.is_some());
    assert_eq!(
        props.indent,
        Some(TableWidth {
            value: 108,
            width_type: "dxa".to_string()
        })
    );
}

#[test]
fn test_parse_row_properties() {
    let xml = r#"<w:trPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:trHeight w:val="720" w:hRule="atLeast"/>
        <w:tblHeader/>
        <w:cantSplit/>
    </w:trPr>"#;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    reader.read_event_into(&mut buf).unwrap();
    buf.clear();

    let mut budget = SecurityBudget::with_defaults();
    let (props, skips) = parse_row_properties(&mut reader, &mut budget).unwrap();

    assert_eq!(skips, RowGridSkips::default());
    assert_eq!(props.height, Some(720));
    assert_eq!(props.height_rule, Some("atLeast".to_string()));
    assert!(props.is_header);
    assert!(props.cant_split);
}

fn parse_row_grid_skips(xml: &str) -> RowGridSkips {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    reader.read_event_into(&mut buf).unwrap();
    buf.clear();

    let mut budget = SecurityBudget::with_defaults();
    parse_row_properties(&mut reader, &mut budget).unwrap().1
}

#[test]
fn test_parse_row_properties_grid_before_and_after() {
    let skips = parse_row_grid_skips(
        r#"<w:trPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:gridBefore w:val="2"/>
        <w:gridAfter w:val="1"/>
    </w:trPr>"#,
    );

    assert_eq!(skips, RowGridSkips { before: 2, after: 1 });
}

#[test]
fn test_parse_row_properties_drops_an_implausible_grid_skip() {
    let skips = parse_row_grid_skips(
        r#"<w:trPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:gridBefore w:val="4294967295"/>
    </w:trPr>"#,
    );

    assert_eq!(skips.before, 0);
}

/// `RowProperties` is public, and callers may build it with an exhaustive literal. The grid
/// skips live in `RowGridSkips` instead, so this literal must keep compiling.
#[test]
fn test_row_properties_keeps_its_public_fields() {
    let props = RowProperties {
        height: None,
        height_rule: None,
        is_header: false,
        cant_split: false,
    };
    assert_eq!(props, RowProperties::default());
}

#[test]
fn test_parse_cell_properties_merged() {
    let xml = r#"<w:tcPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:tcW w:w="2000" w:type="dxa"/>
        <w:gridSpan w:val="3"/>
        <w:vMerge w:val="restart"/>
    </w:tcPr>"#;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    reader.read_event_into(&mut buf).unwrap();
    buf.clear();

    let mut budget = SecurityBudget::with_defaults();
    let props = parse_cell_properties(&mut reader, &mut budget).unwrap();

    assert_eq!(
        props.width,
        Some(TableWidth {
            value: 2000,
            width_type: "dxa".to_string()
        })
    );
    assert_eq!(props.grid_span, Some(3));
    assert_eq!(props.v_merge, Some(VerticalMerge::Restart));
}

#[test]
fn test_parse_cell_properties_shading() {
    let xml = r#"<w:tcPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:shd w:val="clear" w:color="auto" w:fill="D9E2F3"/>
        <w:tcBorders>
            <w:top w:val="single" w:sz="8" w:color="000000"/>
            <w:left w:val="single" w:sz="8" w:color="000000"/>
        </w:tcBorders>
        <w:vAlign w:val="center"/>
    </w:tcPr>"#;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    reader.read_event_into(&mut buf).unwrap();
    buf.clear();

    let mut budget = SecurityBudget::with_defaults();
    let props = parse_cell_properties(&mut reader, &mut budget).unwrap();

    assert!(props.shading.is_some());
    let shading = props.shading.unwrap();
    assert_eq!(shading.fill, Some("D9E2F3".to_string()));
    assert_eq!(shading.color, Some("auto".to_string()));
    assert_eq!(shading.val, Some("clear".to_string()));

    assert!(props.borders.is_some());
    assert_eq!(props.vertical_align, Some("center".to_string()));
}

#[test]
fn test_parse_table_grid() {
    let xml = r#"<w:tblGrid xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:gridCol w:w="2500"/>
        <w:gridCol w:w="2500"/>
        <w:gridCol w:w="2000"/>
        <w:gridCol w:w="2000"/>
    </w:tblGrid>"#;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    reader.read_event_into(&mut buf).unwrap();
    buf.clear();

    let mut budget = SecurityBudget::with_defaults();
    let grid = parse_table_grid(&mut reader, &mut budget).unwrap();

    assert_eq!(grid.columns, vec![2500, 2500, 2000, 2000]);
}

#[test]
fn test_parse_table_look() {
    let xml = r#"<w:tblLook xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" w:val="0460"/>"#;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let event = reader.read_event_into(&mut buf).unwrap();

    if let Event::Empty(e) = event {
        let look = parse_table_look(&e);

        assert!(look.first_row);
        assert!(look.last_row);
        assert!(!look.first_column);
    } else {
        panic!("Expected Empty event");
    }
}

#[test]
fn test_vmerge_continue() {
    let xml = r#"<w:tcPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:vMerge/>
    </w:tcPr>"#;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    reader.read_event_into(&mut buf).unwrap();
    buf.clear();

    let mut budget = SecurityBudget::with_defaults();
    let props = parse_cell_properties(&mut reader, &mut budget).unwrap();

    assert_eq!(props.v_merge, Some(VerticalMerge::Continue));
}

#[test]
fn test_empty_table_properties() {
    let xml = r#"<w:tblPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"/>"#;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    reader.read_event_into(&mut buf).unwrap();
    buf.clear();

    let props = TableProperties::default();

    assert!(props.style_id.is_none());
    assert!(props.width.is_none());
    assert!(props.alignment.is_none());
}

#[test]
fn test_cell_margins() {
    let xml = r#"<w:tblCellMar xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:top w:w="100" w:type="dxa"/>
        <w:bottom w:w="100" w:type="dxa"/>
        <w:left w:w="50" w:type="dxa"/>
        <w:right w:w="50" w:type="dxa"/>
    </w:tblCellMar>"#;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    reader.read_event_into(&mut buf).unwrap();
    buf.clear();

    let margins = parse_cell_margins_element(&mut reader);

    assert_eq!(margins.top, Some(100));
    assert_eq!(margins.bottom, Some(100));
    assert_eq!(margins.left, Some(50));
    assert_eq!(margins.right, Some(50));
}

#[test]
fn test_border_styles() {
    let xml = r#"<w:tblBorders xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:top w:val="single" w:sz="12" w:color="FF0000" w:space="0"/>
        <w:bottom w:val="double" w:sz="24" w:color="0000FF" w:space="1"/>
        <w:left w:val="dashed" w:sz="8" w:color="auto"/>
        <w:right w:val="dotted" w:sz="4"/>
    </w:tblBorders>"#;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    reader.read_event_into(&mut buf).unwrap();
    buf.clear();

    let borders = parse_table_borders(&mut reader);

    assert!(borders.top.is_some());
    let top = borders.top.unwrap();
    assert_eq!(top.style, "single");
    assert_eq!(top.size, Some(12));
    assert_eq!(top.color, Some("FF0000".to_string()));
    assert_eq!(top.space, Some(0));

    assert!(borders.bottom.is_some());
    let bottom = borders.bottom.unwrap();
    assert_eq!(bottom.style, "double");
    assert_eq!(bottom.size, Some(24));

    assert!(borders.left.is_some());
    let left = borders.left.unwrap();
    assert_eq!(left.style, "dashed");
    assert_eq!(left.color, Some("auto".to_string()));

    assert!(borders.right.is_some());
    let right = borders.right.unwrap();
    assert_eq!(right.style, "dotted");
}

#[test]
fn test_table_properties_round_trip_serialize() {
    let props = TableProperties {
        style_id: Some("TableGrid".to_string()),
        width: Some(TableWidth {
            value: 5000,
            width_type: "dxa".to_string(),
        }),
        alignment: Some("center".to_string()),
        layout: Some("fixed".to_string()),
        look: Some(TableLook {
            first_row: true,
            last_row: true,
            first_column: false,
            last_column: false,
            no_h_band: false,
            no_v_band: false,
        }),
        borders: None,
        cell_margins: None,
        indent: Some(TableWidth {
            value: 108,
            width_type: "dxa".to_string(),
        }),
        caption: None,
    };

    let json = serde_json::to_string(&props).unwrap();
    let deserialized: TableProperties = serde_json::from_str(&json).unwrap();

    assert_eq!(props, deserialized);
}

#[test]
fn test_cell_properties_round_trip_serialize() {
    let props = CellProperties {
        width: Some(TableWidth {
            value: 2000,
            width_type: "dxa".to_string(),
        }),
        grid_span: Some(3),
        v_merge: Some(VerticalMerge::Restart),
        borders: None,
        shading: Some(CellShading {
            fill: Some("D9E2F3".to_string()),
            color: Some("auto".to_string()),
            val: Some("clear".to_string()),
        }),
        margins: None,
        vertical_align: Some("center".to_string()),
        text_direction: None,
        no_wrap: false,
    };

    let json = serde_json::to_string(&props).unwrap();
    let deserialized: CellProperties = serde_json::from_str(&json).unwrap();

    assert_eq!(props, deserialized);
}
