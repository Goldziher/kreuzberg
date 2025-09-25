//! Relationship parsing utilities for PPTX

use crate::pptx::types::{ImageReference, Result};
use roxmltree::Document;

/// Parse slide relationships to extract image references
pub fn parse_slide_rels(xml_data: &[u8]) -> Result<Vec<ImageReference>> {
    let xml_str = std::str::from_utf8(xml_data)?;
    let doc = Document::parse(xml_str)?;
    let root = doc.root_element();

    let mut image_refs = Vec::new();

    for rel_node in root.children().filter(|n| n.tag_name().name() == "Relationship") {
        if let (Some(id), Some(target), Some(rel_type)) = (
            rel_node.attribute("Id"),
            rel_node.attribute("Target"),
            rel_node.attribute("Type"),
        ) {
            // Check if this is an image relationship
            if rel_type.contains("image") {
                image_refs.push(ImageReference {
                    id: id.to_string(),
                    target: target.to_string(),
                });
            }
        }
    }

    Ok(image_refs)
}

/// Parse presentation relationships to find slide paths
pub fn parse_presentation_rels(xml_data: &[u8]) -> Result<Vec<String>> {
    let xml_str = std::str::from_utf8(xml_data)?;
    let doc = Document::parse(xml_str)?;
    let root = doc.root_element();

    let mut slide_paths = Vec::new();

    for rel_node in root.children().filter(|n| n.tag_name().name() == "Relationship") {
        if let (Some(target), Some(rel_type)) = (rel_node.attribute("Target"), rel_node.attribute("Type")) {
            // Check if this is a slide relationship
            if rel_type.contains("slide") && !rel_type.contains("slideMaster") {
                // Handle different path formats
                let full_path = if target.starts_with("/ppt/") {
                    // Absolute path like "/ppt/slides/slide71.xml"
                    target.trim_start_matches('/').to_string()
                } else if target.starts_with("ppt/") {
                    // Relative path like "ppt/slides/slide71.xml"
                    target.to_string()
                } else if target.starts_with("slides/") {
                    // Relative to ppt/ like "slides/slide71.xml"
                    format!("ppt/{}", target)
                } else {
                    // Bare filename like "slide71.xml"
                    format!("ppt/slides/{}", target)
                };
                slide_paths.push(full_path);
            }
        }
    }

    slide_paths.sort();
    Ok(slide_paths)
}
