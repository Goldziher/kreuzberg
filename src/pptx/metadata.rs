use crate::pptx::types::{PptxMetadataDTO, Result};
use roxmltree::Document;

pub fn extract_metadata(core_xml: &[u8]) -> Result<PptxMetadataDTO> {
    let xml_str = std::str::from_utf8(core_xml)?;
    let doc = Document::parse(xml_str)?;
    let root = doc.root_element();

    let mut metadata = PptxMetadataDTO {
        title: None,
        author: None,
        description: None,
        summary: None,
        fonts: Vec::new(),
    };

    for node in root.descendants() {
        match node.tag_name().name() {
            "title" => {
                if let Some(text) = node.text() {
                    metadata.title = Some(text.trim().to_string());
                }
            }
            "creator" => {
                if let Some(text) = node.text() {
                    metadata.author = Some(text.trim().to_string());
                }
            }
            "description" => {
                if let Some(text) = node.text() {
                    metadata.description = Some(text.trim().to_string());
                }
            }
            "subject" => {
                if let Some(text) = node.text()
                    && metadata.description.is_none()
                {
                    metadata.description = Some(text.trim().to_string());
                }
            }
            _ => {}
        }
    }

    Ok(metadata)
}
