//! Metadata extraction and notes handling.
//!
//! This module provides functionality for extracting metadata from PPTX files
//! and extracting notes from slides.

use std::collections::HashMap;
use std::io::{Read, Seek};
use zip::ZipArchive;

use crate::core::diagnostics::push_warning;
use crate::error::Result;
use crate::text::utf8_validation;
use crate::types::ProcessingWarning;
use crate::types::metadata::PptxMetadata;
use roxmltree::Document;

#[cfg(feature = "office")]
use crate::extraction::office_metadata::{
    app_properties::{DOC_SECURITY_KEY, PptxAppProperties, decode_doc_security_flags},
    core_properties::CoreProperties,
    extract_core_properties, extract_custom_properties, extract_pptx_app_properties,
};
#[cfg(feature = "office")]
use serde_json::Value;

use super::container::PptxContainer;

use crate::extraction::ooxml_constants::{
    DRAWINGML_NAMESPACE, PRESENTATIONML_2010_NAMESPACE, PRESENTATIONML_NAMESPACE,
};

/// Extract comprehensive metadata from PPTX using office_metadata module.
///
/// Returns `(PptxMetadata, HashMap<String, String>)` where the second element
/// contains office metadata keys (title, author, created_by, etc.). A failed
/// read of `docProps/core.xml`, `docProps/app.xml`, or the custom properties
/// part is surfaced as a `ProcessingWarning` rather than silently returning a
/// document with those fields missing (#238).
#[cfg(feature = "office")]
fn merge_core_properties(metadata_map: &mut HashMap<String, String>, core: CoreProperties) {
    if let Some(title) = core.title {
        metadata_map.insert("title".to_string(), title);
    }
    if let Some(creator) = core.creator {
        metadata_map.insert("author".to_string(), creator.clone());
        metadata_map.insert("created_by".to_string(), creator);
    }
    if let Some(subject) = core.subject {
        metadata_map.insert("subject".to_string(), subject.clone());
        metadata_map.insert("summary".to_string(), subject);
    }
    if let Some(keywords) = core.keywords {
        metadata_map.insert("keywords".to_string(), keywords);
    }
    if let Some(description) = core.description {
        metadata_map.insert("description".to_string(), description);
    }
    if let Some(modified_by) = core.last_modified_by {
        metadata_map.insert("modified_by".to_string(), modified_by);
    }
    if let Some(created) = core.created {
        metadata_map.insert("created_at".to_string(), created);
    }
    if let Some(modified) = core.modified {
        metadata_map.insert("modified_at".to_string(), modified);
    }
    if let Some(revision) = core.revision {
        metadata_map.insert("revision".to_string(), revision);
    }
    if let Some(category) = core.category {
        metadata_map.insert("category".to_string(), category);
    }
}

/// Merge PPTX application properties into `metadata_map`, returning the slide count
/// and slide names/titles for `PptxMetadata` (both are also tracked in `app`, but the
/// caller keeps them as separate fields rather than a submap). ~keep
#[cfg(feature = "office")]
fn merge_app_properties(metadata_map: &mut HashMap<String, String>, app: PptxAppProperties) -> (usize, Vec<String>) {
    let mut slide_count = 0;
    let mut slide_names = Vec::new();

    if let Some(slides) = app.slides {
        metadata_map.insert("slide_count".to_string(), slides.to_string());
        slide_count = slides.max(0) as usize;
    }
    if let Some(notes) = app.notes {
        metadata_map.insert("notes_count".to_string(), notes.to_string());
    }
    if let Some(hidden_slides) = app.hidden_slides {
        metadata_map.insert("hidden_slides".to_string(), hidden_slides.to_string());
    }
    if !app.slide_titles.is_empty() {
        slide_names = app.slide_titles.clone();
        metadata_map.insert("slide_titles".to_string(), app.slide_titles.join(", "));
    }
    if let Some(presentation_format) = app.presentation_format {
        metadata_map.insert("presentation_format".to_string(), presentation_format);
    }
    if let Some(company) = app.company {
        metadata_map.insert("organization".to_string(), company);
    }
    if let Some(application) = app.application {
        metadata_map.insert("application".to_string(), application);
    }
    if let Some(app_version) = app.app_version {
        metadata_map.insert("application_version".to_string(), app_version);
    }
    // #230: surface the raw DocSecurity integer plus its decoded ECMA-376
    // flags. `PptxAppProperties` never reaches `FormatMetadata::Pptx`, so
    // without this the presentation's protection state was discarded entirely. ~keep
    if let Some(raw) = app.doc_security {
        metadata_map.insert(DOC_SECURITY_KEY.to_string(), raw.to_string());
        for (key, value) in decode_doc_security_flags(raw) {
            metadata_map.insert(key.to_string(), value.to_string());
        }
    }

    (slide_count, slide_names)
}

#[cfg(feature = "office")]
fn merge_custom_properties(metadata_map: &mut HashMap<String, String>, custom: HashMap<String, Value>) {
    for (key, value) in custom {
        let value_str = match value {
            Value::String(s) => s,
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(_) | Value::Object(_) => value.to_string(),
        };
        metadata_map.insert(format!("custom_{}", key), value_str);
    }
}

#[cfg_attr(not(feature = "office"), allow(unused_variables))]
pub(super) fn extract_metadata<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    warnings: &mut Vec<ProcessingWarning>,
) -> (PptxMetadata, HashMap<String, String>) {
    #[cfg(feature = "office")]
    {
        let mut metadata_map = HashMap::new();
        let mut slide_count = 0;
        let mut slide_names = Vec::new();

        match extract_core_properties(archive) {
            Ok(core) => merge_core_properties(&mut metadata_map, core),
            Err(e) => push_warning(
                warnings,
                "pptx",
                format!(
                    "Could not read docProps/core.xml: {}; core document metadata was not extracted",
                    e
                ),
            ),
        }

        match extract_pptx_app_properties(archive) {
            Ok(app) => (slide_count, slide_names) = merge_app_properties(&mut metadata_map, app),
            Err(e) => push_warning(
                warnings,
                "pptx",
                format!(
                    "Could not read docProps/app.xml: {}; application metadata was not extracted",
                    e
                ),
            ),
        }

        match extract_custom_properties(archive) {
            Ok(custom) => merge_custom_properties(&mut metadata_map, custom),
            Err(e) => push_warning(
                warnings,
                "pptx",
                format!(
                    "Could not read custom document properties: {}; custom metadata was not extracted",
                    e
                ),
            ),
        }

        (
            PptxMetadata {
                slide_count: slide_count as u32,
                slide_names,
                image_count: None,
                table_count: None,
            },
            metadata_map,
        )
    }

    #[cfg(not(feature = "office"))]
    {
        (
            PptxMetadata {
                slide_count: 0,
                slide_names: Vec::new(),
            },
            HashMap::new(),
        )
    }
}

/// Extract speaker notes for every slide, keyed by 1-indexed slide number.
///
/// Notes parts have independent numbering; each slide's relationship identifies
/// its notes part (#2138). Missing notes are normal; parse failures retain the
/// remaining slides' notes and surface a `ProcessingWarning` (#238). ~keep
pub(super) fn extract_all_notes<R: Read + Seek>(
    container: &mut PptxContainer<R>,
    warnings: &mut Vec<ProcessingWarning>,
) -> Result<HashMap<u32, String>> {
    let mut notes = HashMap::new();

    let slide_paths: Vec<String> = container.slide_paths().to_vec();

    for (i, slide_path) in slide_paths.iter().enumerate() {
        let notes_path = match resolve_notes_path(container, slide_path) {
            Ok(Some(path)) => path,
            Ok(None) => continue,
            Err(error) => {
                push_warning(
                    warnings,
                    "pptx",
                    format!(
                        "Could not resolve speaker notes for slide {}: {}; notes were not extracted",
                        i + 1,
                        error
                    ),
                );
                continue;
            }
        };
        let notes_xml = match container.read_file(&notes_path) {
            Ok(data) => data,
            Err(_) => continue,
        };
        match extract_notes_text(&notes_xml) {
            Ok(note_text) => {
                notes.insert((i + 1) as u32, note_text);
            }
            Err(e) => push_warning(
                warnings,
                "pptx",
                format!(
                    "Could not parse speaker notes for slide {} ('{}'): {}; notes were not extracted",
                    i + 1,
                    notes_path,
                    e
                ),
            ),
        }
    }

    Ok(notes)
}

fn resolve_notes_path<R: Read + Seek>(container: &mut PptxContainer<R>, slide_path: &str) -> Result<Option<String>> {
    let rels_path = container.get_slide_rels_path(slide_path);
    if container.archive.index_for_name(&rels_path).is_none() {
        return Ok(None);
    }
    let rels_data = container.read_file(&rels_path)?;
    let Some(target) = super::parser::parse_slide_rels(&rels_data)?.notes_target else {
        return Ok(None);
    };
    let base = slide_path.rsplit_once('/').map_or("", |(directory, _)| directory);
    crate::extractors::security::resolve_container_entry(base, &target)
        .map(Some)
        .map_err(|error| {
            crate::error::XbergError::parsing(format!("Invalid speaker notes relationship target: {error}"))
        })
}

/// Build the slide-id -> 1-indexed-position map from the document's `<p:sldIdLst>`.
fn build_slide_id_positions(doc: &Document) -> HashMap<u32, u32> {
    let mut id_to_position: HashMap<u32, u32> = HashMap::new();
    let Some(sld_id_lst) = doc
        .descendants()
        .find(|node| node.has_tag_name((PRESENTATIONML_NAMESPACE, "sldIdLst")))
    else {
        return id_to_position;
    };

    let mut position: u32 = 1;
    for child in sld_id_lst.children() {
        if !child.has_tag_name((PRESENTATIONML_NAMESPACE, "sldId")) {
            continue;
        }
        if let Some(id_str) = child.attribute("id")
            && let Ok(id) = id_str.parse::<u32>()
        {
            id_to_position.insert(id, position);
        }
        position += 1;
    }

    id_to_position
}

/// Map each slide id in `sld_id_lst`'s `<p14:sectionSldId>` children to `name` in `result`,
/// using `id_to_position` to translate slide id to 1-indexed slide position.
fn insert_section_slide_positions(
    sld_id_lst: roxmltree::Node,
    name: &str,
    id_to_position: &HashMap<u32, u32>,
    result: &mut HashMap<u32, String>,
) {
    for sld_id in sld_id_lst.children() {
        if !sld_id.has_tag_name((PRESENTATIONML_2010_NAMESPACE, "sectionSldId")) {
            continue;
        }
        if let Some(id_str) = sld_id.attribute("id")
            && let Ok(id) = id_str.parse::<u32>()
            && let Some(&position) = id_to_position.get(&id)
        {
            result.insert(position, name.to_string());
        }
    }
}

/// Build the slide-position -> section-name map from the document's `<p14:sectionLst>`.
fn build_section_names(doc: &Document, id_to_position: &HashMap<u32, u32>) -> HashMap<u32, String> {
    let mut result: HashMap<u32, String> = HashMap::new();
    let Some(section_lst) = doc
        .descendants()
        .find(|node| node.has_tag_name((PRESENTATIONML_2010_NAMESPACE, "sectionLst")))
    else {
        return result;
    };

    for section in section_lst.children() {
        if !section.has_tag_name((PRESENTATIONML_2010_NAMESPACE, "section")) {
            continue;
        }
        let name = match section.attribute("name") {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };
        for sld_id_lst in section.children() {
            if !sld_id_lst.has_tag_name((PRESENTATIONML_2010_NAMESPACE, "sectionSldIdLst")) {
                continue;
            }
            insert_section_slide_positions(sld_id_lst, name, id_to_position, &mut result);
        }
    }

    result
}

/// Extract section names from `ppt/presentation.xml`.
///
/// Reads the `<p14:sectionLst>` extension element (PowerPoint 2010+) and maps
/// each slide position (1-indexed) to the name of the section it belongs to.
/// Returns an empty map when no sections exist or the feature is unavailable.
pub(super) fn extract_section_names<R: Read + Seek>(container: &mut PptxContainer<R>) -> Result<HashMap<u32, String>> {
    let xml = match container.read_file("ppt/presentation.xml") {
        Ok(data) => data,
        Err(_) => return Ok(HashMap::new()),
    };

    let xml_str = match crate::text::utf8_validation::from_utf8(&xml) {
        Ok(s) => s,
        Err(_) => return Ok(HashMap::new()),
    };

    let doc = match roxmltree::Document::parse(xml_str) {
        Ok(d) => d,
        Err(_) => return Ok(HashMap::new()),
    };

    let id_to_position = build_slide_id_positions(&doc);
    if id_to_position.is_empty() {
        return Ok(HashMap::new());
    }

    Ok(build_section_names(&doc, &id_to_position))
}

fn extract_notes_text(notes_xml: &[u8]) -> Result<String> {
    let xml_str = utf8_validation::from_utf8(notes_xml)
        .map_err(|e| crate::error::XbergError::parsing(format!("Invalid UTF-8 in notes XML: {}", e)))?;

    let doc = Document::parse(xml_str)
        .map_err(|e| crate::error::XbergError::parsing(format!("Failed to parse notes XML: {}", e)))?;

    let mut text_parts = Vec::with_capacity(16);
    for node in doc.descendants() {
        if node.has_tag_name((DRAWINGML_NAMESPACE, "t"))
            && let Some(text) = node.text()
        {
            text_parts.push(text);
        }
    }

    Ok(text_parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use zip::write::{SimpleFileOptions, ZipWriter};

    /// Default security limits for tests with no `ExtractionConfig` of their own to
    /// derive one from.
    fn default_security_limits() -> crate::extractors::security::SecurityLimits {
        crate::extractors::security::SecurityLimits::default()
    }

    fn sld_id_lst_xml(slide_ids: &[(u32, &str)]) -> String {
        let mut sld_id_lst = String::new();
        for (i, (id, _)) in slide_ids.iter().enumerate() {
            sld_id_lst.push_str(&format!(r#"<p:sldId id="{}" r:id="rId{}"/>"#, id, i + 1));
        }
        sld_id_lst
    }

    fn section_lst_xml(sections: &[(&str, &[u32])]) -> String {
        let mut section_lst = String::new();
        for (name, ids) in sections {
            let mut sld_ids = String::new();
            for id in *ids {
                sld_ids.push_str(&format!(r#"<p14:sectionSldId id="{}"/>"#, id));
            }
            section_lst.push_str(&format!(
                r#"<p14:section name="{}"><p14:sectionSldIdLst>{}</p14:sectionSldIdLst></p14:section>"#,
                name, sld_ids
            ));
        }
        section_lst
    }

    fn presentation_rels_xml(slide_ids: &[(u32, &str)]) -> String {
        let mut pres_rels = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
        );
        for (i, (_, path)) in slide_ids.iter().enumerate() {
            pres_rels.push_str(&format!(
                r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="{}"/>"#,
                i + 1,
                path
            ));
        }
        pres_rels.push_str("</Relationships>");
        pres_rels
    }

    fn make_pptx_with_sections(slide_ids: &[(u32, &str)], sections: &[(&str, &[u32])]) -> Vec<u8> {
        use std::io::Write;

        let sld_id_lst = sld_id_lst_xml(slide_ids);
        let section_lst = section_lst_xml(sections);

        let presentation_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main">
  <p:sldIdLst>{}</p:sldIdLst>
  <p:extLst>
    <p:ext uri="{{521415D9-36F7-43E2-AB2F-B90AF26B5E84}}">
      <p14:sectionLst>{}</p14:sectionLst>
    </p:ext>
  </p:extLst>
</p:presentation>"#,
            sld_id_lst, section_lst
        );

        let mut buffer = Vec::new();
        let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
        let opts = SimpleFileOptions::default();

        zip.start_file("[Content_Types].xml", opts).unwrap();
        zip.write_all(
            b"<?xml version=\"1.0\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"/>",
        )
        .unwrap();

        zip.start_file("_rels/.rels", opts).unwrap();
        zip.write_all(b"<?xml version=\"1.0\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"/>").unwrap();

        zip.start_file("ppt/_rels/presentation.xml.rels", opts).unwrap();
        zip.write_all(presentation_rels_xml(slide_ids).as_bytes()).unwrap();

        zip.start_file("ppt/presentation.xml", opts).unwrap();
        zip.write_all(presentation_xml.as_bytes()).unwrap();

        for (i, (_, path)) in slide_ids.iter().enumerate() {
            let slide_xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>Slide {}</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld>
</p:sld>"#,
                i + 1
            );
            zip.start_file(format!("ppt/{}", path), opts).unwrap();
            zip.write_all(slide_xml.as_bytes()).unwrap();
        }

        let _ = zip.finish().unwrap();
        buffer
    }

    #[test]
    fn test_extract_section_names_two_sections() {
        let pptx = make_pptx_with_sections(
            &[
                (256, "slides/slide1.xml"),
                (257, "slides/slide2.xml"),
                (258, "slides/slide3.xml"),
            ],
            &[("Introduction", &[256]), ("Main Content", &[257, 258])],
        );
        let mut container = PptxContainer::from_bytes(&pptx, &default_security_limits()).unwrap();
        let sections = extract_section_names(&mut container).unwrap();

        assert_eq!(sections.get(&1), Some(&"Introduction".to_string()));
        assert_eq!(sections.get(&2), Some(&"Main Content".to_string()));
        assert_eq!(sections.get(&3), Some(&"Main Content".to_string()));
    }

    #[test]
    fn test_extract_section_names_no_sections() {
        let pptx = make_pptx_with_sections(&[(256, "slides/slide1.xml"), (257, "slides/slide2.xml")], &[]);
        let mut container = PptxContainer::from_bytes(&pptx, &default_security_limits()).unwrap();
        let sections = extract_section_names(&mut container).unwrap();

        assert!(sections.is_empty(), "Expected empty map when no sections defined");
    }

    #[test]
    fn test_extract_section_names_single_section_all_slides() {
        let pptx = make_pptx_with_sections(
            &[(300, "slides/slide1.xml"), (301, "slides/slide2.xml")],
            &[("Only Section", &[300, 301])],
        );
        let mut container = PptxContainer::from_bytes(&pptx, &default_security_limits()).unwrap();
        let sections = extract_section_names(&mut container).unwrap();

        assert_eq!(sections.get(&1), Some(&"Only Section".to_string()));
        assert_eq!(sections.get(&2), Some(&"Only Section".to_string()));
    }

    #[test]
    fn test_extract_section_names_invalid_utf8_returns_empty() {
        use std::io::Write;

        let mut buffer = Vec::new();
        let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
        let opts = SimpleFileOptions::default();

        zip.start_file("[Content_Types].xml", opts).unwrap();
        zip.write_all(
            b"<?xml version=\"1.0\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"/>",
        )
        .unwrap();

        zip.start_file("_rels/.rels", opts).unwrap();
        zip.write_all(b"<?xml version=\"1.0\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"/>").unwrap();

        zip.start_file("ppt/_rels/presentation.xml.rels", opts).unwrap();
        zip.write_all(b"<?xml version=\"1.0\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide1.xml\"/></Relationships>").unwrap();

        zip.start_file("ppt/presentation.xml", opts).unwrap();
        zip.write_all(b"\xff\xfe invalid utf8 \x80\x81\x82").unwrap();

        zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
        zip.write_all(b"<?xml version=\"1.0\"?><p:sld xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"><p:cSld><p:spTree/></p:cSld></p:sld>").unwrap();

        let _ = zip.finish().unwrap();

        let mut container = PptxContainer::from_bytes(&buffer, &default_security_limits()).unwrap();
        let sections = extract_section_names(&mut container).unwrap();
        assert!(
            sections.is_empty(),
            "Expected empty map for invalid UTF-8 presentation.xml"
        );
    }
}
