//! Notes extraction from notesSlides

use crate::pptx::types::Result;
use crate::pptx::utils::get_slide_notes_path;
use roxmltree::Document;
use std::collections::HashMap;

/// Extract speaker notes for a slide
pub fn extract_slide_notes(notes_xml: &[u8]) -> Result<String> {
    let xml_str = std::str::from_utf8(notes_xml)?;
    let doc = Document::parse(xml_str)?;
    let root = doc.root_element();

    let mut notes_text = String::new();

    for node in root.descendants() {
        if node.tag_name().name() == "t" {
            if let Some(text) = node.text() {
                notes_text.push_str(text);
            }
        }
    }

    Ok(notes_text.trim().to_string())
}

/// Extract notes for all slides that have them
pub fn extract_all_notes(container: &mut crate::pptx::container::PptxContainer) -> Result<HashMap<u32, String>> {
    let mut notes = HashMap::new();
    let slide_paths = container.slide_paths().to_vec();

    for (index, slide_path) in slide_paths.iter().enumerate() {
        let slide_number = (index + 1) as u32;

        let notes_path = get_slide_notes_path(slide_path);

        if let Ok(notes_xml) = container.read_file(&notes_path) {
            if let Ok(notes_text) = extract_slide_notes(&notes_xml) {
                if !notes_text.is_empty() {
                    notes.insert(slide_number, notes_text);
                }
            }
        }
    }

    Ok(notes)
}
