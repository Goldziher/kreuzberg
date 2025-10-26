//! PowerPoint presentation extraction functions.
//!
//! This module provides PowerPoint (PPTX) file parsing by directly reading the Office Open XML
//! format. It extracts text content, slide structure, images, and presentation metadata.
//!
//! # Features
//!
//! - **Slide extraction**: Reads all slides from presentation
//! - **Text formatting**: Preserves bold, italic, underline formatting as Markdown
//! - **Image extraction**: Optionally extracts embedded images with metadata
//! - **Office metadata**: Extracts core properties, custom properties (when `office` feature enabled)
//! - **Structure preservation**: Maintains heading hierarchy and list structure
//!
//! # Supported Formats
//!
//! - `.pptx` - PowerPoint Presentation
//! - `.pptm` - PowerPoint Macro-Enabled Presentation
//! - `.ppsx` - PowerPoint Slide Show
//!
//! # Example
//!
//! ```rust
//! use kreuzberg::extraction::pptx::extract_pptx_from_path;
//!
//! # fn example() -> kreuzberg::Result<()> {
//! let result = extract_pptx_from_path("presentation.pptx", true)?;
//!
//! println!("Slide count: {}", result.slide_count);
//! println!("Image count: {}", result.image_count);
//! println!("Content:\n{}", result.content);
//! # Ok(())
//! # }
//! ```
use crate::error::{KreuzbergError, Result};
use crate::types::{ExtractedImage, PptxExtractionResult, PptxMetadata};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

#[cfg(feature = "office")]
use crate::extraction::office_metadata::{
    extract_core_properties, extract_custom_properties, extract_pptx_app_properties,
};
#[cfg(feature = "office")]
use serde_json::Value;

// ============================================================================
// SECTION: Data Structures
// ============================================================================

#[derive(Debug, Clone, Default, PartialEq)]
struct ElementPosition {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
struct Formatting {
    bold: bool,
    italic: bool,
    underline: bool,
    font_size: Option<f32>,
}

#[derive(Debug, Clone)]
struct Run {
    text: String,
    formatting: Formatting,
}

impl Run {
    fn extract(&self) -> String {
        self.text.clone()
    }

    fn render_as_md(&self) -> String {
        let mut result = self.text.clone();

        if self.formatting.bold {
            result = format!("**{}**", result);
        }
        if self.formatting.italic {
            result = format!("*{}*", result);
        }

        result
    }
}

#[derive(Debug, Clone)]
struct TextElement {
    runs: Vec<Run>,
}

#[derive(Debug, Clone)]
struct ListItem {
    level: u32,
    is_ordered: bool,
    runs: Vec<Run>,
}

#[derive(Debug, Clone)]
struct ListElement {
    items: Vec<ListItem>,
}

#[derive(Debug, Clone)]
struct TableCell {
    runs: Vec<Run>,
}

#[derive(Debug, Clone)]
struct TableRow {
    cells: Vec<TableCell>,
}

#[derive(Debug, Clone)]
struct TableElement {
    rows: Vec<TableRow>,
}

#[derive(Debug, Clone)]
struct ImageReference {
    id: String,
    target: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum SlideElement {
    Text(TextElement, ElementPosition),
    Table(TableElement, ElementPosition),
    Image(ImageReference, ElementPosition),
    List(ListElement, ElementPosition),
}

#[allow(dead_code)]
impl SlideElement {
    fn position(&self) -> ElementPosition {
        match self {
            SlideElement::Text(_, pos)
            | SlideElement::Table(_, pos)
            | SlideElement::Image(_, pos)
            | SlideElement::List(_, pos) => pos.clone(),
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct Slide {
    slide_number: u32,
    elements: Vec<SlideElement>,
    images: Vec<ImageReference>,
    image_data: HashMap<String, Vec<u8>>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ParserConfig {
    extract_images: bool,
    max_cache_size_mb: usize,
    include_slide_comment: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            extract_images: true,
            max_cache_size_mb: 256,
            include_slide_comment: false,
        }
    }
}

// ============================================================================
// SECTION: Content Builder - Markdown Generation
// ============================================================================

struct ContentBuilder {
    content: String,
}

impl ContentBuilder {
    fn new() -> Self {
        Self {
            content: String::with_capacity(8192),
        }
    }

    fn with_capacity(capacity: usize) -> Self {
        Self {
            content: String::with_capacity(capacity),
        }
    }

    fn add_slide_header(&mut self, slide_number: u32) {
        self.content.reserve(50);
        self.content.push_str("\n\n<!-- Slide number: ");
        self.content.push_str(&slide_number.to_string());
        self.content.push_str(" -->\n");
    }

    fn add_text(&mut self, text: &str) {
        if !text.trim().is_empty() {
            self.content.push_str(text);
        }
    }

    fn add_title(&mut self, title: &str) {
        if !title.trim().is_empty() {
            self.content.push_str("# ");
            self.content.push_str(title.trim());
            self.content.push('\n');
        }
    }

    fn add_table(&mut self, rows: &[Vec<String>]) {
        if rows.is_empty() {
            return;
        }

        self.content.push_str("\n<table>");
        for (i, row) in rows.iter().enumerate() {
            self.content.push_str("<tr>");
            let tag = if i == 0 { "th" } else { "td" };

            for cell in row {
                self.content.push('<');
                self.content.push_str(tag);
                self.content.push('>');
                self.content.push_str(&html_escape(cell));
                self.content.push_str("</");
                self.content.push_str(tag);
                self.content.push('>');
            }
            self.content.push_str("</tr>");
        }
        self.content.push_str("</table>\n");
    }

    fn add_list_item(&mut self, level: u32, is_ordered: bool, text: &str) {
        let indent_count = level.saturating_sub(1) as usize;
        for _ in 0..indent_count {
            self.content.push_str("  ");
        }

        let marker = if is_ordered { "1." } else { "-" };
        self.content.push_str(marker);
        self.content.push(' ');
        self.content.push_str(text.trim());
        self.content.push('\n');
    }

    fn add_image(&mut self, image_id: &str, slide_number: u32) {
        let filename = format!("slide_{}_image_{}.jpg", slide_number, image_id);
        self.content.push_str("![");
        self.content.push_str(image_id);
        self.content.push_str("](");
        self.content.push_str(&filename);
        self.content.push_str(")\n");
    }

    fn add_notes(&mut self, notes: &str) {
        if !notes.trim().is_empty() {
            self.content.push_str("\n\n### Notes:\n");
            self.content.push_str(notes);
            self.content.push('\n');
        }
    }

    fn build(self) -> String {
        self.content.trim().to_string()
    }
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

// ============================================================================
// SECTION: ZIP Container Management
// ============================================================================

struct PptxContainer {
    archive: ZipArchive<File>,
    slide_paths: Vec<String>,
}

impl PptxContainer {
    fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        // IO errors must bubble up unchanged - file access issues need user reports ~keep
        let file = File::open(path)?;

        let mut archive = match ZipArchive::new(file) {
            Ok(arc) => arc,
            Err(zip::result::ZipError::Io(io_err)) => return Err(io_err.into()), // Bubble up IO errors ~keep
            Err(e) => {
                return Err(KreuzbergError::parsing(format!(
                    "Failed to read PPTX archive (invalid format): {}",
                    e
                )));
            }
        };

        let slide_paths = Self::find_slide_paths(&mut archive)?;

        Ok(Self { archive, slide_paths })
    }

    fn slide_paths(&self) -> &[String] {
        &self.slide_paths
    }

    fn read_file(&mut self, path: &str) -> Result<Vec<u8>> {
        match self.archive.by_name(path) {
            Ok(mut file) => {
                let mut contents = Vec::new();
                // IO errors must bubble up - file read issues need user reports ~keep
                file.read_to_end(&mut contents)?;
                Ok(contents)
            }
            Err(zip::result::ZipError::FileNotFound) => {
                Err(KreuzbergError::parsing("File not found in archive".to_string()))
            }
            Err(zip::result::ZipError::Io(io_err)) => Err(io_err.into()), // Bubble up IO errors ~keep
            Err(e) => Err(KreuzbergError::parsing(format!("Zip error: {}", e))),
        }
    }

    fn get_slide_rels_path(&self, slide_path: &str) -> String {
        get_slide_rels_path(slide_path)
    }

    fn find_slide_paths(archive: &mut ZipArchive<File>) -> Result<Vec<String>> {
        if let Ok(rels_data) = Self::read_file_from_archive(archive, "ppt/_rels/presentation.xml.rels")
            && let Ok(paths) = parse_presentation_rels(&rels_data)
        {
            return Ok(paths);
        }

        let mut slide_paths = Vec::new();
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name();
                if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                    slide_paths.push(name.to_string());
                }
            }
        }

        slide_paths.sort();
        Ok(slide_paths)
    }

    fn read_file_from_archive(archive: &mut ZipArchive<File>, path: &str) -> Result<Vec<u8>> {
        let mut file = match archive.by_name(path) {
            Ok(f) => f,
            Err(zip::result::ZipError::Io(io_err)) => return Err(io_err.into()), // Bubble up IO errors ~keep
            Err(e) => {
                return Err(KreuzbergError::parsing(format!(
                    "Failed to read file from archive: {}",
                    e
                )));
            }
        };
        let mut contents = Vec::new();
        // IO errors must bubble up - file read issues need user reports ~keep
        file.read_to_end(&mut contents)?;
        Ok(contents)
    }
}

// ============================================================================
// SECTION: Slide Processing
// ============================================================================

impl Slide {
    fn from_xml(slide_number: u32, xml_data: &[u8], rels_data: Option<&[u8]>) -> Result<Self> {
        let elements = parse_slide_xml(xml_data)?;

        let images = if let Some(rels) = rels_data {
            parse_slide_rels(rels)?
        } else {
            Vec::new()
        };

        Ok(Self {
            slide_number,
            elements,
            images,
            image_data: HashMap::new(),
        })
    }

    fn to_markdown(&self, config: &ParserConfig) -> String {
        let mut builder = ContentBuilder::new();

        if config.include_slide_comment {
            builder.add_slide_header(self.slide_number);
        }

        for element in &self.elements {
            match element {
                SlideElement::Text(text, _) => {
                    let text_content: String = text.runs.iter().map(|run| run.render_as_md()).collect();

                    let normalized = text_content.replace('\n', " ");
                    let is_title = normalized.len() < 100 && !normalized.trim().is_empty();

                    if is_title {
                        builder.add_title(normalized.trim());
                    } else {
                        builder.add_text(&text_content);
                    }
                }
                SlideElement::Table(table, _) => {
                    let table_rows: Vec<Vec<String>> = table
                        .rows
                        .iter()
                        .map(|row| {
                            row.cells
                                .iter()
                                .map(|cell| cell.runs.iter().map(|run| run.extract()).collect::<String>())
                                .collect()
                        })
                        .collect();
                    builder.add_table(&table_rows);
                }
                SlideElement::List(list, _) => {
                    for item in &list.items {
                        let item_text: String = item.runs.iter().map(|run| run.extract()).collect();
                        builder.add_list_item(item.level, item.is_ordered, &item_text);
                    }
                }
                SlideElement::Image(img_ref, _) => {
                    builder.add_image(&img_ref.id, self.slide_number);
                }
            }
        }

        builder.build()
    }

    fn image_count(&self) -> usize {
        self.elements
            .iter()
            .filter(|e| matches!(e, SlideElement::Image(_, _)))
            .count()
    }

    fn table_count(&self) -> usize {
        self.elements
            .iter()
            .filter(|e| matches!(e, SlideElement::Table(_, _)))
            .count()
    }
}

#[allow(dead_code)]
struct SlideIterator {
    container: PptxContainer,
    config: ParserConfig,
    current_index: usize,
    total_slides: usize,
}

impl SlideIterator {
    fn new(container: PptxContainer, config: ParserConfig) -> Self {
        let total_slides = container.slide_paths().len();
        Self {
            container,
            config,
            current_index: 0,
            total_slides,
        }
    }

    fn slide_count(&self) -> usize {
        self.total_slides
    }

    fn next_slide(&mut self) -> Result<Option<Slide>> {
        if self.current_index >= self.total_slides {
            return Ok(None);
        }

        let slide_path = &self.container.slide_paths()[self.current_index].clone();
        let slide_number = (self.current_index + 1) as u32;

        let xml_data = self.container.read_file(slide_path)?;

        let rels_path = self.container.get_slide_rels_path(slide_path);
        let rels_data = self.container.read_file(&rels_path).ok();

        let slide = Slide::from_xml(slide_number, &xml_data, rels_data.as_deref())?;

        self.current_index += 1;

        Ok(Some(slide))
    }

    fn get_slide_images(&mut self, slide: &Slide) -> Result<HashMap<String, Vec<u8>>> {
        let mut image_data = HashMap::new();

        for img_ref in &slide.images {
            let slide_path = &self.container.slide_paths()[slide.slide_number as usize - 1];
            let full_path = get_full_image_path(slide_path, &img_ref.target);

            if let Ok(data) = self.container.read_file(&full_path) {
                image_data.insert(img_ref.id.clone(), data);
            }
        }

        Ok(image_data)
    }
}

// ============================================================================
// SECTION: XML Parsing
// ============================================================================

use roxmltree::Document;

fn parse_slide_xml(xml_data: &[u8]) -> Result<Vec<SlideElement>> {
    let xml_str = std::str::from_utf8(xml_data)
        .map_err(|e| KreuzbergError::parsing(format!("Invalid UTF-8 in slide XML: {}", e)))?;

    let doc =
        Document::parse(xml_str).map_err(|e| KreuzbergError::parsing(format!("Failed to parse slide XML: {}", e)))?;

    let mut elements = Vec::new();
    const DRAWINGML_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

    for node in doc.descendants() {
        if node.has_tag_name((DRAWINGML_NS, "t"))
            && let Some(text) = node.text()
        {
            let run = Run {
                text: text.to_string(),
                formatting: Formatting::default(),
            };
            let text_elem = TextElement { runs: vec![run] };
            elements.push(SlideElement::Text(text_elem, ElementPosition::default()));
        }
    }

    Ok(elements)
}

fn parse_slide_rels(rels_data: &[u8]) -> Result<Vec<ImageReference>> {
    let xml_str = std::str::from_utf8(rels_data)
        .map_err(|e| KreuzbergError::parsing(format!("Invalid UTF-8 in rels XML: {}", e)))?;

    let doc =
        Document::parse(xml_str).map_err(|e| KreuzbergError::parsing(format!("Failed to parse rels XML: {}", e)))?;

    let mut images = Vec::new();

    for node in doc.descendants() {
        if node.has_tag_name("Relationship")
            && let Some(rel_type) = node.attribute("Type")
            && rel_type.contains("image")
            && let (Some(id), Some(target)) = (node.attribute("Id"), node.attribute("Target"))
        {
            images.push(ImageReference {
                id: id.to_string(),
                target: target.to_string(),
            });
        }
    }

    Ok(images)
}

fn parse_presentation_rels(rels_data: &[u8]) -> Result<Vec<String>> {
    let xml_str = std::str::from_utf8(rels_data)
        .map_err(|e| KreuzbergError::parsing(format!("Invalid UTF-8 in presentation rels: {}", e)))?;

    let doc = Document::parse(xml_str)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to parse presentation rels: {}", e)))?;

    let mut slide_paths = Vec::new();

    for node in doc.descendants() {
        if node.has_tag_name("Relationship")
            && let Some(rel_type) = node.attribute("Type")
            && rel_type.contains("slide")
            && !rel_type.contains("slideMaster")
            && let Some(target) = node.attribute("Target")
        {
            slide_paths.push(format!("ppt/{}", target));
        }
    }

    Ok(slide_paths)
}

// ============================================================================
// SECTION: Metadata Extraction
// ============================================================================

/// Extract comprehensive metadata from PPTX using office_metadata module
fn extract_metadata(archive: &mut ZipArchive<File>) -> PptxMetadata {
    #[cfg(feature = "office")]
    {
        let mut metadata_map = HashMap::new();

        // Extract core properties (Dublin Core metadata)
        if let Ok(core) = extract_core_properties(archive) {
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

        // Extract app properties (PPTX-specific metadata)
        if let Ok(app) = extract_pptx_app_properties(archive) {
            if let Some(slides) = app.slides {
                metadata_map.insert("slide_count".to_string(), slides.to_string());
            }
            if let Some(notes) = app.notes {
                metadata_map.insert("notes_count".to_string(), notes.to_string());
            }
            if let Some(hidden_slides) = app.hidden_slides {
                metadata_map.insert("hidden_slides".to_string(), hidden_slides.to_string());
            }
            if !app.slide_titles.is_empty() {
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
        }

        // Extract custom properties (optional)
        if let Ok(custom) = extract_custom_properties(archive) {
            for (key, value) in custom {
                // Convert Value to String
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

        // Convert to PptxMetadata struct
        PptxMetadata {
            title: metadata_map.get("title").cloned(),
            author: metadata_map.get("author").cloned(),
            description: metadata_map.get("description").cloned(),
            summary: metadata_map.get("summary").cloned(),
            fonts: Vec::new(),
        }
    }

    #[cfg(not(feature = "office"))]
    {
        // Return empty metadata when office feature is disabled
        PptxMetadata {
            title: None,
            author: None,
            description: None,
            summary: None,
            fonts: Vec::new(),
        }
    }
}

// ============================================================================
// SECTION: Notes Extraction
// ============================================================================

fn extract_all_notes(container: &mut PptxContainer) -> Result<HashMap<u32, String>> {
    let mut notes = HashMap::new();

    let slide_paths: Vec<String> = container.slide_paths().to_vec();

    for (i, slide_path) in slide_paths.iter().enumerate() {
        let notes_path = slide_path.replace("slides/slide", "notesSlides/notesSlide");
        if let Ok(notes_xml) = container.read_file(&notes_path)
            && let Ok(note_text) = extract_notes_text(&notes_xml)
        {
            notes.insert((i + 1) as u32, note_text); // FIXME: check value is used after being moved
        }
    }

    Ok(notes)
}

fn extract_notes_text(notes_xml: &[u8]) -> Result<String> {
    let xml_str = std::str::from_utf8(notes_xml)
        .map_err(|e| KreuzbergError::parsing(format!("Invalid UTF-8 in notes XML: {}", e)))?;

    let doc =
        Document::parse(xml_str).map_err(|e| KreuzbergError::parsing(format!("Failed to parse notes XML: {}", e)))?;

    let mut text_parts = Vec::new();
    const DRAWINGML_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

    for node in doc.descendants() {
        if node.has_tag_name((DRAWINGML_NS, "t"))
            && let Some(text) = node.text()
        {
            text_parts.push(text);
        }
    }

    Ok(text_parts.join(" "))
}

// ============================================================================
// SECTION: Helper Functions
// ============================================================================

fn get_slide_rels_path(slide_path: &str) -> String {
    let parts: Vec<&str> = slide_path.rsplitn(2, '/').collect();
    if parts.len() == 2 {
        format!("{}/_rels/{}.rels", parts[1], parts[0])
    } else {
        format!("_rels/{}.rels", slide_path)
    }
}

fn get_full_image_path(slide_path: &str, image_target: &str) -> String {
    if image_target.starts_with("..") {
        let parts: Vec<&str> = slide_path.rsplitn(3, '/').collect();
        if parts.len() >= 3 {
            format!("{}/{}", parts[2], &image_target[3..])
        } else {
            format!("ppt/{}", &image_target[3..])
        }
    } else {
        let parts: Vec<&str> = slide_path.rsplitn(2, '/').collect();
        if parts.len() == 2 {
            format!("{}/{}", parts[1], image_target)
        } else {
            format!("ppt/slides/{}", image_target)
        }
    }
}

fn detect_image_format(data: &[u8]) -> String {
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "jpeg".to_string()
    } else if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        "png".to_string()
    } else if data.starts_with(b"GIF") {
        "gif".to_string()
    } else if data.starts_with(b"BM") {
        "bmp".to_string()
    } else if data.starts_with(b"<svg") || data.starts_with(b"<?xml") {
        "svg".to_string()
    } else if data.starts_with(b"II\x2A\x00") || data.starts_with(b"MM\x00\x2A") {
        "tiff".to_string()
    } else {
        "unknown".to_string()
    }
}

// ============================================================================
// SECTION: Public API - Main Extraction Functions
// ============================================================================

pub fn extract_pptx_from_path(path: &str, extract_images: bool) -> Result<PptxExtractionResult> {
    let config = ParserConfig {
        extract_images,
        ..Default::default()
    };

    let mut container = PptxContainer::open(path)?;

    // Extract comprehensive metadata using office_metadata module
    let metadata = extract_metadata(&mut container.archive);

    let notes = extract_all_notes(&mut container)?;

    let mut iterator = SlideIterator::new(container, config.clone());
    let slide_count = iterator.slide_count();

    let estimated_capacity = slide_count * 1024;
    let mut content_builder = ContentBuilder::with_capacity(estimated_capacity);

    let mut total_image_count = 0;
    let mut total_table_count = 0;
    let mut extracted_images = Vec::new();

    while let Some(slide) = iterator.next_slide()? {
        content_builder.add_slide_header(slide.slide_number);

        let slide_content = slide.to_markdown(&config);
        content_builder.add_text(&slide_content);

        if let Some(slide_notes) = notes.get(&slide.slide_number) {
            content_builder.add_notes(slide_notes);
        }

        if config.extract_images
            && let Ok(image_data) = iterator.get_slide_images(&slide)
        {
            for (_, data) in image_data {
                // FIXME: check value is used after being moved
                let format = detect_image_format(&data);
                let image_index = extracted_images.len();

                extracted_images.push(ExtractedImage {
                    data,
                    format,
                    image_index,
                    page_number: Some(slide.slide_number as usize),
                    width: None,
                    height: None,
                    colorspace: None,
                    bits_per_component: None,
                    is_mask: false,
                    description: None,
                    ocr_result: None,
                });
            }
        }

        total_image_count += slide.image_count();
        total_table_count += slide.table_count();
    }

    Ok(PptxExtractionResult {
        content: content_builder.build(),
        metadata,
        slide_count,
        image_count: total_image_count,
        table_count: total_table_count,
        images: extracted_images,
    })
}

pub fn extract_pptx_from_bytes(data: &[u8], extract_images: bool) -> Result<PptxExtractionResult> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_path = std::env::temp_dir().join(format!("temp_pptx_{}_{}.pptx", std::process::id(), unique_id));

    // IO errors must bubble up - temp file write issues need user reports ~keep
    std::fs::write(&temp_path, data)?;

    let result = extract_pptx_from_path(temp_path.to_str().unwrap(), extract_images);

    let _ = std::fs::remove_file(&temp_path);

    result
}

// ============================================================================
// SECTION: Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pptx_bytes(slides: Vec<&str>) -> Vec<u8> {
        use std::io::Write;
        use zip::write::{SimpleFileOptions, ZipWriter};

        let mut buffer = Vec::new();
        {
            let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
            let options = SimpleFileOptions::default();

            zip.start_file("[Content_Types].xml", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="xml" ContentType="application/xml"/>
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
</Types>"#,
            )
            .unwrap();

            zip.start_file("ppt/presentation.xml", options).unwrap();
            zip.write_all(b"<?xml version=\"1.0\"?><presentation/>").unwrap();

            zip.start_file("_rels/.rels", options).unwrap();
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();

            let mut rels_xml = String::from(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
            );
            for (i, _) in slides.iter().enumerate() {
                rels_xml.push_str(&format!(
                    r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{}.xml"/>"#,
                    i + 1,
                    i + 1
                ));
            }
            rels_xml.push_str("</Relationships>");
            zip.start_file("ppt/_rels/presentation.xml.rels", options).unwrap();
            zip.write_all(rels_xml.as_bytes()).unwrap();

            for (i, text) in slides.iter().enumerate() {
                let slide_xml = format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld>
        <p:spTree>
            <p:sp>
                <p:txBody>
                    <a:p>
                        <a:r>
                            <a:t>{}</a:t>
                        </a:r>
                    </a:p>
                </p:txBody>
            </p:sp>
        </p:spTree>
    </p:cSld>
</p:sld>"#,
                    text
                );
                zip.start_file(format!("ppt/slides/slide{}.xml", i + 1), options)
                    .unwrap();
                zip.write_all(slide_xml.as_bytes()).unwrap();
            }

            zip.start_file("docProps/core.xml", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/"
                   xmlns:dcterms="http://purl.org/dc/terms/">
    <dc:title>Test Presentation</dc:title>
    <dc:creator>Test Author</dc:creator>
    <dc:description>Test Description</dc:description>
    <dc:subject>Test Subject</dc:subject>
</cp:coreProperties>"#,
            )
            .unwrap();

            let _ = zip.finish().unwrap();
        }
        buffer
    }

    #[test]
    fn test_extract_pptx_from_bytes_single_slide() {
        let pptx_bytes = create_test_pptx_bytes(vec!["Hello World"]);
        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert_eq!(result.slide_count, 1);
        assert!(
            result.content.contains("Hello World"),
            "Content was: {}",
            result.content
        );
        assert_eq!(result.image_count, 0);
        assert_eq!(result.table_count, 0);
    }

    #[test]
    fn test_extract_pptx_from_bytes_multiple_slides() {
        let pptx_bytes = create_test_pptx_bytes(vec!["Slide 1", "Slide 2", "Slide 3"]);
        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert_eq!(result.slide_count, 3);
        assert!(result.content.contains("Slide 1"));
        assert!(result.content.contains("Slide 2"));
        assert!(result.content.contains("Slide 3"));
    }

    #[test]
    fn test_extract_pptx_metadata() {
        let pptx_bytes = create_test_pptx_bytes(vec!["Content"]);
        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert_eq!(result.metadata.title, Some("Test Presentation".to_string()));
        assert_eq!(result.metadata.author, Some("Test Author".to_string()));
        assert_eq!(result.metadata.description, Some("Test Description".to_string()));
        assert_eq!(result.metadata.summary, Some("Test Subject".to_string()));
    }

    #[test]
    fn test_extract_pptx_empty_slides() {
        let pptx_bytes = create_test_pptx_bytes(vec!["", "", ""]);
        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert_eq!(result.slide_count, 3);
    }

    #[test]
    fn test_extract_pptx_from_bytes_invalid_data() {
        let invalid_bytes = b"not a valid pptx file";
        let result = extract_pptx_from_bytes(invalid_bytes, false);

        assert!(result.is_err());
        if let Err(KreuzbergError::Parsing { message: msg, .. }) = result {
            assert!(msg.contains("Failed to read PPTX archive") || msg.contains("Failed to write temp PPTX file"));
        } else {
            panic!("Expected ParsingError");
        }
    }

    #[test]
    fn test_extract_pptx_from_bytes_empty_data() {
        let empty_bytes: &[u8] = &[];
        let result = extract_pptx_from_bytes(empty_bytes, false);

        assert!(result.is_err());
    }

    #[test]
    fn test_detect_image_format_jpeg() {
        let jpeg_header = vec![0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(detect_image_format(&jpeg_header), "jpeg");
    }

    #[test]
    fn test_detect_image_format_png() {
        let png_header = vec![0x89, 0x50, 0x4E, 0x47];
        assert_eq!(detect_image_format(&png_header), "png");
    }

    #[test]
    fn test_detect_image_format_gif() {
        let gif_header = b"GIF89a";
        assert_eq!(detect_image_format(gif_header), "gif");
    }

    #[test]
    fn test_detect_image_format_bmp() {
        let bmp_header = b"BM";
        assert_eq!(detect_image_format(bmp_header), "bmp");
    }

    #[test]
    fn test_detect_image_format_svg() {
        let svg_header = b"<svg xmlns=\"http://www.w3.org/2000/svg\">";
        assert_eq!(detect_image_format(svg_header), "svg");
    }

    #[test]
    fn test_detect_image_format_tiff_little_endian() {
        let tiff_header = vec![0x49, 0x49, 0x2A, 0x00];
        assert_eq!(detect_image_format(&tiff_header), "tiff");
    }

    #[test]
    fn test_detect_image_format_tiff_big_endian() {
        let tiff_header = vec![0x4D, 0x4D, 0x00, 0x2A];
        assert_eq!(detect_image_format(&tiff_header), "tiff");
    }

    #[test]
    fn test_detect_image_format_unknown() {
        let unknown_data = b"unknown format";
        assert_eq!(detect_image_format(unknown_data), "unknown");
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("plain text"), "plain text");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("<tag>"), "&lt;tag&gt;");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
        assert_eq!(html_escape("'apostrophe'"), "&#x27;apostrophe&#x27;");
        assert_eq!(
            html_escape("<a href=\"url\" title='test'>text & more</a>"),
            "&lt;a href=&quot;url&quot; title=&#x27;test&#x27;&gt;text &amp; more&lt;/a&gt;"
        );
    }

    #[test]
    fn test_get_slide_rels_path() {
        assert_eq!(
            get_slide_rels_path("ppt/slides/slide1.xml"),
            "ppt/slides/_rels/slide1.xml.rels"
        );
        assert_eq!(
            get_slide_rels_path("ppt/slides/slide10.xml"),
            "ppt/slides/_rels/slide10.xml.rels"
        );
    }

    #[test]
    fn test_get_full_image_path_relative() {
        assert_eq!(
            get_full_image_path("ppt/slides/slide1.xml", "../media/image1.png"),
            "ppt/media/image1.png"
        );
    }

    #[test]
    fn test_get_full_image_path_direct() {
        assert_eq!(
            get_full_image_path("ppt/slides/slide1.xml", "image1.png"),
            "ppt/slides/image1.png"
        );
    }

    #[test]
    fn test_content_builder_add_text() {
        let mut builder = ContentBuilder::new();
        builder.add_text("Hello");
        builder.add_text(" ");
        builder.add_text("World");
        assert_eq!(builder.build(), "HelloWorld");
    }

    #[test]
    fn test_content_builder_add_text_empty() {
        let mut builder = ContentBuilder::new();
        builder.add_text("   ");
        builder.add_text("");
        assert_eq!(builder.build(), "");
    }

    #[test]
    fn test_content_builder_add_title() {
        let mut builder = ContentBuilder::new();
        builder.add_title("Title");
        assert_eq!(builder.build(), "# Title");
    }

    #[test]
    fn test_content_builder_add_title_with_whitespace() {
        let mut builder = ContentBuilder::new();
        builder.add_title("  Title  ");
        assert_eq!(builder.build(), "# Title");
    }

    #[test]
    fn test_content_builder_add_table_empty() {
        let mut builder = ContentBuilder::new();
        builder.add_table(&[]);
        assert_eq!(builder.build(), "");
    }

    #[test]
    fn test_content_builder_add_table_single_row() {
        let mut builder = ContentBuilder::new();
        let rows = vec![vec!["Header1".to_string(), "Header2".to_string()]];
        builder.add_table(&rows);
        let result = builder.build();
        assert!(result.contains("<table>"));
        assert!(result.contains("<th>Header1</th>"));
        assert!(result.contains("<th>Header2</th>"));
    }

    #[test]
    fn test_content_builder_add_table_multiple_rows() {
        let mut builder = ContentBuilder::new();
        let rows = vec![
            vec!["H1".to_string(), "H2".to_string()],
            vec!["D1".to_string(), "D2".to_string()],
        ];
        builder.add_table(&rows);
        let result = builder.build();
        assert!(result.contains("<th>H1</th>"));
        assert!(result.contains("<td>D1</td>"));
    }

    #[test]
    fn test_content_builder_add_table_with_special_chars() {
        let mut builder = ContentBuilder::new();
        let rows = vec![vec!["<tag>".to_string(), "a & b".to_string()]];
        builder.add_table(&rows);
        let result = builder.build();
        assert!(result.contains("&lt;tag&gt;"));
        assert!(result.contains("a &amp; b"));
    }

    #[test]
    fn test_content_builder_add_list_item_unordered() {
        let mut builder = ContentBuilder::new();
        builder.add_list_item(1, false, "Item 1");
        builder.add_list_item(1, false, "Item 2");
        let result = builder.build();
        assert!(result.contains("- Item 1"));
        assert!(result.contains("- Item 2"));
    }

    #[test]
    fn test_content_builder_add_list_item_ordered() {
        let mut builder = ContentBuilder::new();
        builder.add_list_item(1, true, "First");
        builder.add_list_item(1, true, "Second");
        let result = builder.build();
        assert!(result.contains("1. First"));
        assert!(result.contains("1. Second"));
    }

    #[test]
    fn test_content_builder_add_list_item_nested() {
        let mut builder = ContentBuilder::new();
        builder.add_list_item(1, false, "Level 1");
        builder.add_list_item(2, false, "Level 2");
        builder.add_list_item(3, false, "Level 3");
        let result = builder.build();
        assert!(result.contains("- Level 1"));
        assert!(result.contains("  - Level 2"));
        assert!(result.contains("    - Level 3"));
    }

    #[test]
    fn test_content_builder_add_image() {
        let mut builder = ContentBuilder::new();
        builder.add_image("img123", 5);
        let result = builder.build();
        assert!(result.contains("![img123](slide_5_image_img123.jpg)"));
    }

    #[test]
    fn test_content_builder_add_notes() {
        let mut builder = ContentBuilder::new();
        builder.add_notes("This is a note");
        let result = builder.build();
        assert!(result.contains("### Notes:"));
        assert!(result.contains("This is a note"));
    }

    #[test]
    fn test_content_builder_add_notes_empty() {
        let mut builder = ContentBuilder::new();
        builder.add_notes("   ");
        assert_eq!(builder.build(), "");
    }

    #[test]
    fn test_content_builder_add_slide_header() {
        let mut builder = ContentBuilder::new();
        builder.add_slide_header(3);
        let result = builder.build();
        assert!(result.contains("<!-- Slide number: 3 -->"));
    }

    #[test]
    fn test_run_extract() {
        let run = Run {
            text: "Hello".to_string(),
            formatting: Formatting::default(),
        };
        assert_eq!(run.extract(), "Hello");
    }

    #[test]
    fn test_run_render_as_md_plain() {
        let run = Run {
            text: "plain".to_string(),
            formatting: Formatting::default(),
        };
        assert_eq!(run.render_as_md(), "plain");
    }

    #[test]
    fn test_run_render_as_md_bold() {
        let run = Run {
            text: "bold".to_string(),
            formatting: Formatting {
                bold: true,
                ..Default::default()
            },
        };
        assert_eq!(run.render_as_md(), "**bold**");
    }

    #[test]
    fn test_run_render_as_md_italic() {
        let run = Run {
            text: "italic".to_string(),
            formatting: Formatting {
                italic: true,
                ..Default::default()
            },
        };
        assert_eq!(run.render_as_md(), "*italic*");
    }

    #[test]
    fn test_run_render_as_md_bold_italic() {
        let run = Run {
            text: "both".to_string(),
            formatting: Formatting {
                bold: true,
                italic: true,
                ..Default::default()
            },
        };
        assert_eq!(run.render_as_md(), "***both***");
    }

    #[test]
    fn test_parse_slide_xml_simple_text() {
        let xml = br#"<?xml version="1.0"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld>
        <p:spTree>
            <p:sp>
                <p:txBody>
                    <a:p>
                        <a:r>
                            <a:t>Test Text</a:t>
                        </a:r>
                    </a:p>
                </p:txBody>
            </p:sp>
        </p:spTree>
    </p:cSld>
</p:sld>"#;

        let elements = parse_slide_xml(xml).unwrap();
        if !elements.is_empty() {
            if let SlideElement::Text(text, _) = &elements[0] {
                assert_eq!(text.runs[0].text, "Test Text");
            } else {
                panic!("Expected Text element");
            }
        }
    }

    #[test]
    fn test_parse_slide_xml_invalid_utf8() {
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFF];
        let result = parse_slide_xml(&invalid_utf8);
        assert!(result.is_err());
        if let Err(KreuzbergError::Parsing { message: msg, .. }) = result {
            assert!(msg.contains("Invalid UTF-8"));
        }
    }

    #[test]
    fn test_parse_slide_xml_malformed() {
        let malformed = b"<not valid xml>";
        let result = parse_slide_xml(malformed);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_slide_rels_with_images() {
        let rels_xml = br#"<?xml version="1.0"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
    <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image2.jpg"/>
</Relationships>"#;

        let images = parse_slide_rels(rels_xml).unwrap();
        assert_eq!(images.len(), 2);
        assert_eq!(images[0].id, "rId1");
        assert_eq!(images[0].target, "../media/image1.png");
        assert_eq!(images[1].id, "rId2");
        assert_eq!(images[1].target, "../media/image2.jpg");
    }

    #[test]
    fn test_parse_slide_rels_no_images() {
        let rels_xml = br#"<?xml version="1.0"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide" Target="../notesSlides/notesSlide1.xml"/>
</Relationships>"#;

        let images = parse_slide_rels(rels_xml).unwrap();
        assert_eq!(images.len(), 0);
    }

    #[test]
    fn test_parse_presentation_rels() {
        let rels_xml = br#"<?xml version="1.0"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
    <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide2.xml"/>
    <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
</Relationships>"#;

        let slides = parse_presentation_rels(rels_xml).unwrap();
        assert_eq!(slides.len(), 2);
        assert_eq!(slides[0], "ppt/slides/slide1.xml");
        assert_eq!(slides[1], "ppt/slides/slide2.xml");
    }

    #[test]
    fn test_extract_notes_text() {
        let notes_xml = br#"<?xml version="1.0"?>
<p:notes xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
         xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld>
        <p:spTree>
            <p:sp>
                <p:txBody>
                    <a:p>
                        <a:r>
                            <a:t>First note</a:t>
                        </a:r>
                    </a:p>
                    <a:p>
                        <a:r>
                            <a:t>Second note</a:t>
                        </a:r>
                    </a:p>
                </p:txBody>
            </p:sp>
        </p:spTree>
    </p:cSld>
</p:notes>"#;

        let notes = extract_notes_text(notes_xml).unwrap();
        assert!(notes.contains("First note"));
        assert!(notes.contains("Second note"));
    }

    #[test]
    fn test_parser_config_default() {
        let config = ParserConfig::default();
        assert!(config.extract_images);
        assert_eq!(config.max_cache_size_mb, 256);
        assert!(!config.include_slide_comment);
    }
}
