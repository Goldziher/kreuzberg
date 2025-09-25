//! XML parsing utilities for PPTX slides

use super::constants::P_NAMESPACE;
use crate::pptx::types::{
    ElementPosition, Formatting, ImageReference, ListElement, ListItem, PptxError, Result, Run, SlideElement,
    TableCell, TableElement, TableRow, TextElement,
};
use roxmltree::{Document, Node};

enum ParsedContent {
    Text(TextElement),
    List(ListElement),
}

/// Parse slide XML data into structured elements
pub fn parse_slide_xml(xml_data: &[u8]) -> Result<Vec<SlideElement>> {
    let xml_str = std::str::from_utf8(xml_data)?;
    let doc = Document::parse(xml_str)?;
    let root = doc.root_element();
    let ns = root.tag_name().namespace();

    let c_sld = root
        .descendants()
        .find(|n| n.tag_name().name() == "cSld" && n.tag_name().namespace() == ns)
        .ok_or(PptxError::ParseError("No <p:cSld> tag found"))?;

    let sp_tree = c_sld
        .children()
        .find(|n| n.tag_name().name() == "spTree" && n.tag_name().namespace() == ns)
        .ok_or(PptxError::ParseError("No <p:spTree> tag found"))?;

    let mut elements = Vec::new();
    for child_node in sp_tree.children().filter(|n| n.is_element()) {
        elements.extend(parse_group(&child_node)?);
    }

    // Sort by element type first (text, tables, lists before images), then by position
    elements.sort_by_key(|element| {
        let pos = element.position();
        let type_priority = match element {
            SlideElement::Text(_, _) => 0,
            SlideElement::Table(_, _) => 1,
            SlideElement::List(_, _) => 2,
            SlideElement::Image(_, _) => 3,
        };
        (type_priority, pos.y, pos.x)
    });

    Ok(elements)
}

fn parse_group(node: &Node) -> Result<Vec<SlideElement>> {
    let mut elements = Vec::new();
    let tag_name = node.tag_name().name();
    let namespace = node.tag_name().namespace().unwrap_or("");

    if namespace != P_NAMESPACE {
        return Ok(elements);
    }

    let position = extract_position(node);

    match tag_name {
        "sp" => match parse_sp(node)? {
            ParsedContent::Text(text) => elements.push(SlideElement::Text(text, position)),
            ParsedContent::List(list) => elements.push(SlideElement::List(list, position)),
        },
        "graphicFrame" => {
            if let Some(table) = parse_graphic_frame(node)? {
                elements.push(SlideElement::Table(table, position));
            }
        }
        "pic" => {
            if let Ok(image_ref) = parse_pic(node) {
                elements.push(SlideElement::Image(image_ref, position));
            }
            // Skip images that can't be parsed rather than failing
        }
        "grpSp" => {
            for child in node.children().filter(|n| n.is_element()) {
                elements.extend(parse_group(&child)?);
            }
        }
        _ => {} // Skip unknown elements silently
    }

    Ok(elements)
}

fn extract_position(node: &Node) -> ElementPosition {
    // Look for xfrm (transform) elements to get position
    for child in node.descendants() {
        if child.tag_name().name() == "xfrm" {
            if let Some(off) = child.children().find(|n| n.tag_name().name() == "off") {
                let x = off.attribute("x").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
                let y = off.attribute("y").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
                return ElementPosition { x, y };
            }
        }
    }
    ElementPosition::default()
}

fn parse_sp(node: &Node) -> Result<ParsedContent> {
    // Find text body
    let tx_body = node
        .descendants()
        .find(|n| n.tag_name().name() == "txBody" && n.tag_name().namespace() == Some(P_NAMESPACE));

    if let Some(body) = tx_body {
        return parse_text_body(&body);
    }

    // Return empty text if no text body found
    Ok(ParsedContent::Text(TextElement { runs: vec![] }))
}

fn parse_text_body(body: &Node) -> Result<ParsedContent> {
    let mut text_runs = Vec::new();
    let mut list_items = Vec::new();
    let mut is_list = false;

    for p_node in body.children().filter(|n| n.tag_name().name() == "p") {
        // Check if this paragraph is part of a list
        let (level, is_ordered) = parse_list_properties(&p_node)?;
        if level > 0 {
            is_list = true;
            let runs = parse_paragraph_runs(&p_node)?;
            list_items.push(ListItem {
                level,
                is_ordered,
                runs,
            });
        } else {
            // Regular text paragraph
            let runs = parse_paragraph_runs(&p_node)?;
            text_runs.extend(runs);
        }
    }

    if is_list {
        Ok(ParsedContent::List(ListElement { items: list_items }))
    } else {
        Ok(ParsedContent::Text(TextElement { runs: text_runs }))
    }
}

fn parse_paragraph_runs(p_node: &Node) -> Result<Vec<Run>> {
    let mut runs = Vec::new();

    for r_node in p_node.descendants().filter(|n| n.tag_name().name() == "r") {
        if let Some(t_node) = r_node.children().find(|n| n.tag_name().name() == "t") {
            let text = t_node.text().unwrap_or("").to_string();
            let formatting = parse_run_properties(&r_node);
            runs.push(Run { text, formatting });
        }
    }

    // Add line break after paragraph
    if !runs.is_empty() {
        runs.push(Run {
            text: "\n".to_string(),
            formatting: Formatting::default(),
        });
    }

    Ok(runs)
}

fn parse_run_properties(r_node: &Node) -> Formatting {
    let mut formatting = Formatting::default();

    if let Some(r_pr) = r_node.children().find(|n| n.tag_name().name() == "rPr") {
        // Bold
        if r_pr
            .children()
            .any(|n| n.tag_name().name() == "b" && n.attribute("val") != Some("0"))
        {
            formatting.bold = true;
        }

        // Italic
        if r_pr
            .children()
            .any(|n| n.tag_name().name() == "i" && n.attribute("val") != Some("0"))
        {
            formatting.italic = true;
        }

        // Underline
        if r_pr
            .children()
            .any(|n| n.tag_name().name() == "u" && n.attribute("val") != Some("none"))
        {
            formatting.underline = true;
        }

        // Font size
        if let Some(sz_node) = r_pr.children().find(|n| n.tag_name().name() == "sz") {
            if let Some(val) = sz_node.attribute("val") {
                formatting.font_size = val.parse::<f32>().ok().map(|v| v / 100.0);
                // Convert from points*100
            }
        }
    }

    formatting
}

fn parse_list_properties(p_node: &Node) -> Result<(u32, bool)> {
    let mut level = 0u32;
    let mut is_ordered = false;

    if let Some(p_pr_node) = p_node.children().find(|n| n.tag_name().name() == "pPr") {
        // Get level
        if let Some(lvl_attr) = p_pr_node.attribute("lvl") {
            level = lvl_attr.parse().unwrap_or(0);
        }

        // Check for bullet/numbering # codespell:ignore buAutoNum buChar
        is_ordered = p_pr_node.children().any(|n| {
            n.tag_name().name() == "buAutoNum" || (n.tag_name().name() == "buChar" && n.attribute("char").is_some())
        });

        // If we found bullet properties, ensure minimum level 1
        // "bu" prefix is correct OOXML naming convention (buAutoNum, buChar, etc) # codespell:ignore bu
        if (is_ordered || p_pr_node.children().any(|n| n.tag_name().name().starts_with("bu"))) && level == 0 {
            level = 1;
        }
    }

    Ok((level, is_ordered))
}

fn parse_graphic_frame(node: &Node) -> Result<Option<TableElement>> {
    // Look for table in graphic frame
    let table_node = node.descendants().find(|n| n.tag_name().name() == "tbl");

    if let Some(table) = table_node {
        return Ok(Some(parse_table(&table)?));
    }

    Ok(None)
}

fn parse_table(table_node: &Node) -> Result<TableElement> {
    let mut rows = Vec::new();

    for tr_node in table_node.children().filter(|n| n.tag_name().name() == "tr") {
        let mut cells = Vec::new();

        for tc_node in tr_node.children().filter(|n| n.tag_name().name() == "tc") {
            let mut cell_runs = Vec::new();

            // Find text in cell
            for tx_body in tc_node.descendants().filter(|n| n.tag_name().name() == "txBody") {
                for p_node in tx_body.children().filter(|n| n.tag_name().name() == "p") {
                    let runs = parse_paragraph_runs(&p_node)?;
                    cell_runs.extend(runs);
                }
            }

            cells.push(TableCell { runs: cell_runs });
        }

        if !cells.is_empty() {
            rows.push(TableRow { cells });
        }
    }

    Ok(TableElement { rows })
}

fn parse_pic(node: &Node) -> Result<ImageReference> {
    // Find the blipFill element
    if let Some(blip_fill) = node.descendants().find(|n| n.tag_name().name() == "blipFill") {
        if let Some(blip) = blip_fill.children().find(|n| n.tag_name().name() == "blip") {
            if let Some(embed_attr) = blip.attribute("embed") {
                return Ok(ImageReference {
                    id: embed_attr.to_string(),
                    target: String::new(), // Will be resolved later via relationships
                });
            }
        }
    }

    Err(PptxError::ParseError("No image reference found in pic element"))
}
