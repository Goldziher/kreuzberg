//! ODT (OpenDocument) metadata extraction from meta.xml
//!
//! Extracts metadata from OpenDocument Text files following the OASIS OpenDocument standard.

use crate::error::{KreuzbergError, Result};
use std::io::Read;
use zip::ZipArchive;

/// OpenDocument metadata from meta.xml
///
/// Contains metadata fields defined by the OASIS OpenDocument Format standard.
/// Uses Dublin Core elements (dc:) and OpenDocument meta elements (meta:).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OdtProperties {
    /// Document title (dc:title)
    pub title: Option<String>,
    /// Document subject/topic (dc:subject)
    pub subject: Option<String>,
    /// Current document creator/author (dc:creator)
    pub creator: Option<String>,
    /// Initial creator of the document (meta:initial-creator)
    pub initial_creator: Option<String>,
    /// Keywords or tags (meta:keyword)
    pub keywords: Option<String>,
    /// Document description (dc:description)
    pub description: Option<String>,
    /// Current modification date (dc:date)
    pub date: Option<String>,
    /// Initial creation date (meta:creation-date)
    pub creation_date: Option<String>,
    /// Document language (dc:language)
    pub language: Option<String>,
    /// Generator/application that created the document (meta:generator)
    pub generator: Option<String>,
    /// Editing duration in ISO 8601 format (meta:editing-duration)
    pub editing_duration: Option<String>,
    /// Number of edits/revisions (meta:editing-cycles)
    pub editing_cycles: Option<String>,
    /// Document statistics - page count (meta:page-count)
    pub page_count: Option<i32>,
    /// Document statistics - word count (meta:word-count)
    pub word_count: Option<i32>,
    /// Document statistics - character count (meta:character-count)
    pub character_count: Option<i32>,
    /// Document statistics - paragraph count (meta:paragraph-count)
    pub paragraph_count: Option<i32>,
    /// Document statistics - table count (meta:table-count)
    pub table_count: Option<i32>,
    /// Document statistics - image count (meta:image-count)
    pub image_count: Option<i32>,
}

/// Extract ODT metadata from an OpenDocument file
///
/// Parses `meta.xml` from the ZIP archive and extracts OpenDocument metadata.
///
/// # Arguments
///
/// * `archive` - ZIP archive containing the OpenDocument file
///
/// # Returns
///
/// Returns `OdtProperties` with extracted metadata. Fields that are not present
/// in the document will be `None`.
///
/// # Errors
///
/// Returns an error if:
/// - The ZIP archive cannot be read
/// - The meta.xml file is malformed
/// - XML parsing fails
///
/// # Example
///
/// ```no_run
/// use kreuzberg::extraction::office_metadata::extract_odt_properties;
/// use std::fs::File;
/// use zip::ZipArchive;
///
/// let file = File::open("document.odt")?;
/// let mut archive = ZipArchive::new(file)?;
/// let props = extract_odt_properties(&mut archive)?;
///
/// println!("Title: {:?}", props.title);
/// println!("Creator: {:?}", props.creator);
/// println!("Created: {:?}", props.creation_date);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn extract_odt_properties<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>) -> Result<OdtProperties> {
    let mut xml_content = String::new();

    match archive.by_name("meta.xml") {
        Ok(mut file) => {
            file.read_to_string(&mut xml_content)
                .map_err(|e| KreuzbergError::parsing(format!("Failed to read meta.xml: {}", e)))?;
        }
        Err(_) => {
            return Ok(OdtProperties::default());
        }
    }

    let doc = roxmltree::Document::parse(&xml_content)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to parse meta.xml: {}", e)))?;

    let root = doc.root_element();

    // Extract Dublin Core elements
    let title = super::parse_xml_text(root, "title");
    let subject = super::parse_xml_text(root, "subject");
    let creator = super::parse_xml_text(root, "creator");
    let description = super::parse_xml_text(root, "description");
    let language = super::parse_xml_text(root, "language");
    let date = super::parse_xml_text(root, "date");

    // Extract OpenDocument meta elements
    let initial_creator = super::parse_xml_text(root, "initial-creator");
    let keywords = super::parse_xml_text(root, "keyword");
    let creation_date = super::parse_xml_text(root, "creation-date");
    let generator = super::parse_xml_text(root, "generator");
    let editing_duration = super::parse_xml_text(root, "editing-duration");
    let editing_cycles = super::parse_xml_text(root, "editing-cycles");

    // Extract document statistics
    let page_count = super::parse_xml_int(root, "page-count");
    let word_count = super::parse_xml_int(root, "word-count");
    let character_count = super::parse_xml_int(root, "character-count");
    let paragraph_count = super::parse_xml_int(root, "paragraph-count");
    let table_count = super::parse_xml_int(root, "table-count");
    let image_count = super::parse_xml_int(root, "image-count");

    Ok(OdtProperties {
        title,
        subject,
        creator,
        initial_creator,
        keywords,
        description,
        date,
        creation_date,
        language,
        generator,
        editing_duration,
        editing_cycles,
        page_count,
        word_count,
        character_count,
        paragraph_count,
        table_count,
        image_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    fn create_test_zip_with_meta_xml(meta_xml: &str) -> ZipArchive<Cursor<Vec<u8>>> {
        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let mut zip = zip::ZipWriter::new(cursor);

        let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("meta.xml", options).unwrap();
        zip.write_all(meta_xml.as_bytes()).unwrap();

        let cursor = zip.finish().unwrap();
        ZipArchive::new(cursor).unwrap()
    }

    #[test]
    fn test_extract_odt_properties_full() {
        let meta_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
                      xmlns:dc="http://purl.org/dc/elements/1.1/"
                      xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0"
                      office:version="1.3">
  <office:meta>
    <dc:title>Test Document</dc:title>
    <dc:subject>Testing</dc:subject>
    <dc:creator>John Doe</dc:creator>
    <meta:initial-creator>Jane Smith</meta:initial-creator>
    <dc:description>A test document for ODT metadata</dc:description>
    <meta:keyword>test, metadata, odt</meta:keyword>
    <dc:language>en-US</dc:language>
    <meta:creation-date>2024-01-01T10:00:00Z</meta:creation-date>
    <dc:date>2024-01-02T15:30:00Z</dc:date>
    <meta:generator>LibreOffice/24.2</meta:generator>
    <meta:editing-duration>PT2H30M</meta:editing-duration>
    <meta:editing-cycles>5</meta:editing-cycles>
    <meta:page-count>10</meta:page-count>
    <meta:word-count>1500</meta:word-count>
    <meta:character-count>9000</meta:character-count>
    <meta:paragraph-count>45</meta:paragraph-count>
    <meta:table-count>3</meta:table-count>
    <meta:image-count>7</meta:image-count>
  </office:meta>
</office:document-meta>"#;

        let mut archive = create_test_zip_with_meta_xml(meta_xml);
        let props = extract_odt_properties(&mut archive).unwrap();

        assert_eq!(props.title, Some("Test Document".to_string()));
        assert_eq!(props.subject, Some("Testing".to_string()));
        assert_eq!(props.creator, Some("John Doe".to_string()));
        assert_eq!(props.initial_creator, Some("Jane Smith".to_string()));
        assert_eq!(props.keywords, Some("test, metadata, odt".to_string()));
        assert_eq!(props.description, Some("A test document for ODT metadata".to_string()));
        assert_eq!(props.language, Some("en-US".to_string()));
        assert_eq!(props.creation_date, Some("2024-01-01T10:00:00Z".to_string()));
        assert_eq!(props.date, Some("2024-01-02T15:30:00Z".to_string()));
        assert_eq!(props.generator, Some("LibreOffice/24.2".to_string()));
        assert_eq!(props.editing_duration, Some("PT2H30M".to_string()));
        assert_eq!(props.editing_cycles, Some("5".to_string()));
        assert_eq!(props.page_count, Some(10));
        assert_eq!(props.word_count, Some(1500));
        assert_eq!(props.character_count, Some(9000));
        assert_eq!(props.paragraph_count, Some(45));
        assert_eq!(props.table_count, Some(3));
        assert_eq!(props.image_count, Some(7));
    }

    #[test]
    fn test_extract_odt_properties_minimal() {
        let meta_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
                      xmlns:dc="http://purl.org/dc/elements/1.1/"
                      xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0"
                      office:version="1.3">
  <office:meta>
    <dc:creator>Alice</dc:creator>
    <meta:creation-date>2024-01-01T10:00:00Z</meta:creation-date>
  </office:meta>
</office:document-meta>"#;

        let mut archive = create_test_zip_with_meta_xml(meta_xml);
        let props = extract_odt_properties(&mut archive).unwrap();

        assert_eq!(props.creator, Some("Alice".to_string()));
        assert_eq!(props.creation_date, Some("2024-01-01T10:00:00Z".to_string()));
        assert_eq!(props.title, None);
        assert_eq!(props.keywords, None);
        assert_eq!(props.word_count, None);
    }

    #[test]
    fn test_extract_odt_properties_empty_elements() {
        let meta_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
                      xmlns:dc="http://purl.org/dc/elements/1.1/"
                      xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0"
                      office:version="1.3">
  <office:meta>
    <dc:title></dc:title>
    <dc:creator>Bob</dc:creator>
  </office:meta>
</office:document-meta>"#;

        let mut archive = create_test_zip_with_meta_xml(meta_xml);
        let props = extract_odt_properties(&mut archive).unwrap();

        assert_eq!(props.title, None);
        assert_eq!(props.creator, Some("Bob".to_string()));
    }

    #[test]
    fn test_extract_odt_properties_missing_file() {
        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let zip = zip::ZipWriter::new(cursor);
        let cursor = zip.finish().unwrap();
        let mut archive = ZipArchive::new(cursor).unwrap();

        let props = extract_odt_properties(&mut archive).unwrap();
        assert_eq!(props, OdtProperties::default());
    }

    #[test]
    fn test_extract_odt_properties_malformed_xml() {
        let meta_xml = "not valid xml <";
        let mut archive = create_test_zip_with_meta_xml(meta_xml);

        let result = extract_odt_properties(&mut archive);
        assert!(result.is_err());
    }
}
