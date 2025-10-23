//! Office Open XML metadata extraction
//!
//! This module provides functionality to extract comprehensive metadata from Office Open XML
//! documents (DOCX, XLSX, PPTX) by parsing the `docProps/core.xml`, `docProps/app.xml`,
//! and `docProps/custom.xml` files within the ZIP container.
//!
//! # Overview
//!
//! Office documents store metadata in three XML files:
//! - `docProps/core.xml` - Dublin Core metadata (title, creator, dates, keywords, etc.)
//! - `docProps/app.xml` - Application-specific properties (page count, word count, etc.)
//! - `docProps/custom.xml` - Custom properties defined by users or applications
//!
//! # Example
//!
//! ```no_run
//! use kreuzberg::extraction::office_metadata::{extract_core_properties, extract_docx_app_properties};
//! use std::fs::File;
//! use zip::ZipArchive;
//!
//! let file = File::open("document.docx")?;
//! let mut archive = ZipArchive::new(file)?;
//!
//! // Extract core properties
//! let core = extract_core_properties(&mut archive)?;
//! println!("Title: {:?}", core.title);
//! println!("Created: {:?}", core.created);
//!
//! // Extract DOCX app properties
//! let app = extract_docx_app_properties(&mut archive)?;
//! println!("Word count: {:?}", app.words);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod app_properties;
pub mod core_properties;
pub mod custom_properties;

pub use app_properties::{
    DocxAppProperties, PptxAppProperties, XlsxAppProperties, extract_docx_app_properties, extract_pptx_app_properties,
    extract_xlsx_app_properties,
};
pub use core_properties::{CoreProperties, extract_core_properties};
pub use custom_properties::{CustomProperties, extract_custom_properties};

use roxmltree::Node;

/// Parse text content from an XML element by tag name
///
/// Returns the text content if the element exists and has non-empty text.
pub(crate) fn parse_xml_text(node: Node, name: &str) -> Option<String> {
    node.descendants()
        .find(|n| n.has_tag_name(name))
        .and_then(|n| n.text())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(String::from)
}

/// Parse integer content from an XML element by tag name
///
/// Returns the parsed integer if the element exists and contains valid integer text.
pub(crate) fn parse_xml_int(node: Node, name: &str) -> Option<i32> {
    node.descendants()
        .find(|n| n.has_tag_name(name))
        .and_then(|n| n.text())
        .and_then(|s| s.trim().parse::<i32>().ok())
}

/// Parse boolean content from an XML element by tag name
///
/// Handles "true"/"false" string values and converts to boolean.
pub(crate) fn parse_xml_bool(node: Node, name: &str) -> Option<bool> {
    node.descendants()
        .find(|n| n.has_tag_name(name))
        .and_then(|n| n.text())
        .map(|s| s.trim())
        .and_then(|s| match s.to_lowercase().as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_xml_text() {
        let xml = r#"<root><title>Test Document</title></root>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let root = doc.root_element();

        assert_eq!(parse_xml_text(root, "title"), Some("Test Document".to_string()));
        assert_eq!(parse_xml_text(root, "missing"), None);
    }

    #[test]
    fn test_parse_xml_text_empty() {
        let xml = r#"<root><title></title></root>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let root = doc.root_element();

        assert_eq!(parse_xml_text(root, "title"), None);
    }

    #[test]
    fn test_parse_xml_int() {
        let xml = r#"<root><count>42</count></root>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let root = doc.root_element();

        assert_eq!(parse_xml_int(root, "count"), Some(42));
        assert_eq!(parse_xml_int(root, "missing"), None);
    }

    #[test]
    fn test_parse_xml_bool() {
        let xml = r#"<root><flag>true</flag><other>false</other></root>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let root = doc.root_element();

        assert_eq!(parse_xml_bool(root, "flag"), Some(true));
        assert_eq!(parse_xml_bool(root, "other"), Some(false));
        assert_eq!(parse_xml_bool(root, "missing"), None);
    }
}
