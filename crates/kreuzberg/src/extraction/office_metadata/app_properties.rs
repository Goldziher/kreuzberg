//! Application properties extraction from docProps/app.xml
//!
//! Extracts format-specific metadata from Office Open XML documents.

use crate::error::{KreuzbergError, Result};
use roxmltree::Node;
use std::io::Read;
use zip::ZipArchive;

/// Application properties from docProps/app.xml for DOCX
///
/// Contains Word-specific document statistics and metadata.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DocxAppProperties {
    /// Application name (e.g., "Microsoft Office Word")
    pub application: Option<String>,
    /// Application version
    pub app_version: Option<String>,
    /// Template filename
    pub template: Option<String>,
    /// Total editing time in minutes
    pub total_time: Option<i32>,
    /// Number of pages
    pub pages: Option<i32>,
    /// Number of words
    pub words: Option<i32>,
    /// Number of characters (excluding spaces)
    pub characters: Option<i32>,
    /// Number of characters (including spaces)
    pub characters_with_spaces: Option<i32>,
    /// Number of lines
    pub lines: Option<i32>,
    /// Number of paragraphs
    pub paragraphs: Option<i32>,
    /// Company name
    pub company: Option<String>,
    /// Document security level
    pub doc_security: Option<i32>,
    /// Scale crop flag
    pub scale_crop: Option<bool>,
    /// Links up to date flag
    pub links_up_to_date: Option<bool>,
    /// Shared document flag
    pub shared_doc: Option<bool>,
    /// Hyperlinks changed flag
    pub hyperlinks_changed: Option<bool>,
}

/// Application properties from docProps/app.xml for XLSX
///
/// Contains Excel-specific document metadata.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct XlsxAppProperties {
    /// Application name (e.g., "Microsoft Excel")
    pub application: Option<String>,
    /// Application version
    pub app_version: Option<String>,
    /// Document security level
    pub doc_security: Option<i32>,
    /// Scale crop flag
    pub scale_crop: Option<bool>,
    /// Links up to date flag
    pub links_up_to_date: Option<bool>,
    /// Shared document flag
    pub shared_doc: Option<bool>,
    /// Hyperlinks changed flag
    pub hyperlinks_changed: Option<bool>,
    /// Company name
    pub company: Option<String>,
    /// Worksheet names
    pub worksheet_names: Vec<String>,
}

/// Application properties from docProps/app.xml for PPTX
///
/// Contains PowerPoint-specific document metadata.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PptxAppProperties {
    /// Application name (e.g., "Microsoft Office PowerPoint")
    pub application: Option<String>,
    /// Application version
    pub app_version: Option<String>,
    /// Total editing time in minutes
    pub total_time: Option<i32>,
    /// Company name
    pub company: Option<String>,
    /// Document security level
    pub doc_security: Option<i32>,
    /// Scale crop flag
    pub scale_crop: Option<bool>,
    /// Links up to date flag
    pub links_up_to_date: Option<bool>,
    /// Shared document flag
    pub shared_doc: Option<bool>,
    /// Hyperlinks changed flag
    pub hyperlinks_changed: Option<bool>,
    /// Number of slides
    pub slides: Option<i32>,
    /// Number of notes
    pub notes: Option<i32>,
    /// Number of hidden slides
    pub hidden_slides: Option<i32>,
    /// Number of multimedia clips
    pub multimedia_clips: Option<i32>,
    /// Presentation format (e.g., "Widescreen", "Standard")
    pub presentation_format: Option<String>,
    /// Slide titles
    pub slide_titles: Vec<String>,
}

/// Extract DOCX application properties from an Office Open XML document
///
/// Parses `docProps/app.xml` and extracts Word-specific metadata.
pub fn extract_docx_app_properties<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>) -> Result<DocxAppProperties> {
    let mut xml_content = String::new();

    match archive.by_name("docProps/app.xml") {
        Ok(mut file) => {
            file.read_to_string(&mut xml_content)
                .map_err(|e| KreuzbergError::parsing(format!("Failed to read app.xml: {}", e)))?;
        }
        Err(_) => {
            return Ok(DocxAppProperties::default());
        }
    }

    let doc = roxmltree::Document::parse(&xml_content)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to parse app.xml: {}", e)))?;

    let root = doc.root_element();

    Ok(DocxAppProperties {
        application: super::parse_xml_text(root, "Application"),
        app_version: super::parse_xml_text(root, "AppVersion"),
        template: super::parse_xml_text(root, "Template"),
        total_time: super::parse_xml_int(root, "TotalTime"),
        pages: super::parse_xml_int(root, "Pages"),
        words: super::parse_xml_int(root, "Words"),
        characters: super::parse_xml_int(root, "Characters"),
        characters_with_spaces: super::parse_xml_int(root, "CharactersWithSpaces"),
        lines: super::parse_xml_int(root, "Lines"),
        paragraphs: super::parse_xml_int(root, "Paragraphs"),
        company: super::parse_xml_text(root, "Company"),
        doc_security: super::parse_xml_int(root, "DocSecurity"),
        scale_crop: super::parse_xml_bool(root, "ScaleCrop"),
        links_up_to_date: super::parse_xml_bool(root, "LinksUpToDate"),
        shared_doc: super::parse_xml_bool(root, "SharedDoc"),
        hyperlinks_changed: super::parse_xml_bool(root, "HyperlinksChanged"),
    })
}

/// Extract XLSX application properties from an Office Open XML document
///
/// Parses `docProps/app.xml` and extracts Excel-specific metadata including worksheet names.
pub fn extract_xlsx_app_properties<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>) -> Result<XlsxAppProperties> {
    let mut xml_content = String::new();

    match archive.by_name("docProps/app.xml") {
        Ok(mut file) => {
            file.read_to_string(&mut xml_content)
                .map_err(|e| KreuzbergError::parsing(format!("Failed to read app.xml: {}", e)))?;
        }
        Err(_) => {
            return Ok(XlsxAppProperties::default());
        }
    }

    let doc = roxmltree::Document::parse(&xml_content)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to parse app.xml: {}", e)))?;

    let root = doc.root_element();

    // Extract worksheet names from TitlesOfParts vector
    let worksheet_names = extract_titles_of_parts(root);

    Ok(XlsxAppProperties {
        application: super::parse_xml_text(root, "Application"),
        app_version: super::parse_xml_text(root, "AppVersion"),
        doc_security: super::parse_xml_int(root, "DocSecurity"),
        scale_crop: super::parse_xml_bool(root, "ScaleCrop"),
        links_up_to_date: super::parse_xml_bool(root, "LinksUpToDate"),
        shared_doc: super::parse_xml_bool(root, "SharedDoc"),
        hyperlinks_changed: super::parse_xml_bool(root, "HyperlinksChanged"),
        company: super::parse_xml_text(root, "Company"),
        worksheet_names,
    })
}

/// Extract PPTX application properties from an Office Open XML document
///
/// Parses `docProps/app.xml` and extracts PowerPoint-specific metadata including slide information.
pub fn extract_pptx_app_properties<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>) -> Result<PptxAppProperties> {
    let mut xml_content = String::new();

    match archive.by_name("docProps/app.xml") {
        Ok(mut file) => {
            file.read_to_string(&mut xml_content)
                .map_err(|e| KreuzbergError::parsing(format!("Failed to read app.xml: {}", e)))?;
        }
        Err(_) => {
            return Ok(PptxAppProperties::default());
        }
    }

    let doc = roxmltree::Document::parse(&xml_content)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to parse app.xml: {}", e)))?;

    let root = doc.root_element();

    // Extract slide titles from TitlesOfParts vector
    let slide_titles = extract_titles_of_parts(root);

    // Extract presentation format from PresentationFormat element
    let presentation_format = super::parse_xml_text(root, "PresentationFormat");

    Ok(PptxAppProperties {
        application: super::parse_xml_text(root, "Application"),
        app_version: super::parse_xml_text(root, "AppVersion"),
        total_time: super::parse_xml_int(root, "TotalTime"),
        company: super::parse_xml_text(root, "Company"),
        doc_security: super::parse_xml_int(root, "DocSecurity"),
        scale_crop: super::parse_xml_bool(root, "ScaleCrop"),
        links_up_to_date: super::parse_xml_bool(root, "LinksUpToDate"),
        shared_doc: super::parse_xml_bool(root, "SharedDoc"),
        hyperlinks_changed: super::parse_xml_bool(root, "HyperlinksChanged"),
        slides: super::parse_xml_int(root, "Slides"),
        notes: super::parse_xml_int(root, "Notes"),
        hidden_slides: super::parse_xml_int(root, "HiddenSlides"),
        multimedia_clips: super::parse_xml_int(root, "MMClips"),
        presentation_format,
        slide_titles,
    })
}

/// Extract titles from TitlesOfParts vt:vector element
///
/// Handles the vt:vector/vt:lpstr structure used for worksheet/slide names.
fn extract_titles_of_parts(root: Node) -> Vec<String> {
    let mut titles = Vec::new();

    // Find TitlesOfParts element
    if let Some(titles_node) = root.descendants().find(|n| n.has_tag_name("TitlesOfParts")) {
        // Find vt:vector element
        if let Some(vector_node) = titles_node.descendants().find(|n| n.has_tag_name("vector")) {
            // Extract all vt:lpstr elements
            for lpstr_node in vector_node.descendants().filter(|n| n.has_tag_name("lpstr")) {
                if let Some(text) = lpstr_node.text() {
                    let text = text.trim();
                    if !text.is_empty() {
                        titles.push(text.to_string());
                    }
                }
            }
        }
    }

    titles
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    fn create_test_zip_with_app_xml(app_xml: &str) -> ZipArchive<Cursor<Vec<u8>>> {
        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let mut zip = zip::ZipWriter::new(cursor);

        let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("docProps/app.xml", options).unwrap();
        zip.write_all(app_xml.as_bytes()).unwrap();

        let cursor = zip.finish().unwrap();
        ZipArchive::new(cursor).unwrap()
    }

    #[test]
    fn test_extract_docx_app_properties() {
        let app_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties">
    <Application>Microsoft Office Word</Application>
    <AppVersion>16.0000</AppVersion>
    <TotalTime>120</TotalTime>
    <Pages>5</Pages>
    <Words>1000</Words>
    <Characters>5500</Characters>
    <CharactersWithSpaces>6500</CharactersWithSpaces>
    <Lines>100</Lines>
    <Paragraphs>50</Paragraphs>
    <Company>Acme Corp</Company>
    <DocSecurity>0</DocSecurity>
    <ScaleCrop>false</ScaleCrop>
</Properties>"#;

        let mut archive = create_test_zip_with_app_xml(app_xml);
        let props = extract_docx_app_properties(&mut archive).unwrap();

        assert_eq!(props.application, Some("Microsoft Office Word".to_string()));
        assert_eq!(props.app_version, Some("16.0000".to_string()));
        assert_eq!(props.total_time, Some(120));
        assert_eq!(props.pages, Some(5));
        assert_eq!(props.words, Some(1000));
        assert_eq!(props.characters, Some(5500));
        assert_eq!(props.characters_with_spaces, Some(6500));
        assert_eq!(props.lines, Some(100));
        assert_eq!(props.paragraphs, Some(50));
        assert_eq!(props.company, Some("Acme Corp".to_string()));
        assert_eq!(props.doc_security, Some(0));
        assert_eq!(props.scale_crop, Some(false));
    }

    #[test]
    fn test_extract_xlsx_app_properties_with_worksheets() {
        let app_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"
            xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">
    <Application>Microsoft Excel</Application>
    <AppVersion>16.0300</AppVersion>
    <Company>Test Company</Company>
    <TitlesOfParts>
        <vt:vector size="3" baseType="lpstr">
            <vt:lpstr>Sheet1</vt:lpstr>
            <vt:lpstr>Sheet2</vt:lpstr>
            <vt:lpstr>Sheet3</vt:lpstr>
        </vt:vector>
    </TitlesOfParts>
</Properties>"#;

        let mut archive = create_test_zip_with_app_xml(app_xml);
        let props = extract_xlsx_app_properties(&mut archive).unwrap();

        assert_eq!(props.application, Some("Microsoft Excel".to_string()));
        assert_eq!(props.app_version, Some("16.0300".to_string()));
        assert_eq!(props.company, Some("Test Company".to_string()));
        assert_eq!(props.worksheet_names, vec!["Sheet1", "Sheet2", "Sheet3"]);
    }

    #[test]
    fn test_extract_pptx_app_properties() {
        let app_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"
            xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">
    <Application>Microsoft Office PowerPoint</Application>
    <AppVersion>16.0000</AppVersion>
    <TotalTime>45</TotalTime>
    <Slides>10</Slides>
    <Notes>5</Notes>
    <HiddenSlides>2</HiddenSlides>
    <MMClips>3</MMClips>
    <PresentationFormat>Widescreen</PresentationFormat>
    <TitlesOfParts>
        <vt:vector size="2" baseType="lpstr">
            <vt:lpstr>Title Slide</vt:lpstr>
            <vt:lpstr>Agenda</vt:lpstr>
        </vt:vector>
    </TitlesOfParts>
</Properties>"#;

        let mut archive = create_test_zip_with_app_xml(app_xml);
        let props = extract_pptx_app_properties(&mut archive).unwrap();

        assert_eq!(props.application, Some("Microsoft Office PowerPoint".to_string()));
        assert_eq!(props.slides, Some(10));
        assert_eq!(props.notes, Some(5));
        assert_eq!(props.hidden_slides, Some(2));
        assert_eq!(props.multimedia_clips, Some(3));
        assert_eq!(props.presentation_format, Some("Widescreen".to_string()));
        assert_eq!(props.slide_titles, vec!["Title Slide", "Agenda"]);
    }

    #[test]
    fn test_extract_app_properties_missing_file() {
        // Test DOCX
        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let zip = zip::ZipWriter::new(cursor);
        let cursor = zip.finish().unwrap();
        let mut archive = ZipArchive::new(cursor).unwrap();

        let docx = extract_docx_app_properties(&mut archive).unwrap();
        assert_eq!(docx, DocxAppProperties::default());

        // Test XLSX with a fresh archive
        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let zip = zip::ZipWriter::new(cursor);
        let cursor = zip.finish().unwrap();
        let mut archive = ZipArchive::new(cursor).unwrap();

        let xlsx = extract_xlsx_app_properties(&mut archive).unwrap();
        assert_eq!(xlsx, XlsxAppProperties::default());
    }

    #[test]
    fn test_extract_titles_of_parts_empty() {
        let xml = r#"<Properties xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">
            <TitlesOfParts><vt:vector size="0" baseType="lpstr"></vt:vector></TitlesOfParts>
        </Properties>"#;

        let doc = roxmltree::Document::parse(xml).unwrap();
        let titles = extract_titles_of_parts(doc.root_element());
        assert_eq!(titles, Vec::<String>::new());
    }
}
