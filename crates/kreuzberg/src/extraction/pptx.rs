//! PowerPoint presentation extraction functions.
//!
//! This module provides PowerPoint (PPTX) file parsing by directly reading the Office Open XML
//! format. It extracts text content, slide structure, images, and presentation metadata.
//!
//! # Attribution
//!
//! This code is based on the [pptx-to-md](https://github.com/nilskruthoff/pptx-parser) library
//! by Nils Kruthoff, licensed under MIT OR Apache-2.0. The original code has been vendored and
//! adapted to integrate with Kreuzberg's architecture. See ATTRIBUTIONS.md for full license text.
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

const P_NAMESPACE: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
const A_NAMESPACE: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const RELS_NAMESPACE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
struct ElementPosition {
    x: i64,
    y: i64,
}

#[derive(Debug, Clone, Default)]
struct Formatting {
    bold: bool,
    italic: bool,
    underlined: bool,
    lang: String,
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
        if self.formatting.underlined {
            result = format!("<u>{}</u>", result);
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
enum SlideElement {
    Text(TextElement, ElementPosition),
    Table(TableElement, ElementPosition),
    Image(ImageReference, ElementPosition),
    List(ListElement, ElementPosition),
    Unknown,
}

impl SlideElement {
    fn position(&self) -> ElementPosition {
        match self {
            SlideElement::Text(_, pos)
            | SlideElement::Table(_, pos)
            | SlideElement::Image(_, pos)
            | SlideElement::List(_, pos) => *pos,
            SlideElement::Unknown => ElementPosition::default(),
        }
    }
}

#[derive(Debug)]
struct Slide {
    slide_number: u32,
    elements: Vec<SlideElement>,
    images: Vec<ImageReference>,
}

#[derive(Debug, Clone)]
struct ParserConfig {
    extract_images: bool,
    include_slide_comment: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            extract_images: true,
            include_slide_comment: false,
        }
    }
}

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
        })
    }

    fn to_markdown(&self, config: &ParserConfig) -> String {
        let mut builder = ContentBuilder::new();

        if config.include_slide_comment {
            builder.add_slide_header(self.slide_number);
        }

        let mut element_indices: Vec<usize> = (0..self.elements.len()).collect();
        element_indices.sort_by_key(|&i| {
            let pos = self.elements[i].position();
            (pos.y, pos.x)
        });

        for &idx in &element_indices {
            match &self.elements[idx] {
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
                SlideElement::Unknown => {}
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

struct SlideIterator {
    container: PptxContainer,
    current_index: usize,
    total_slides: usize,
}

impl SlideIterator {
    fn new(container: PptxContainer) -> Self {
        let total_slides = container.slide_paths().len();
        Self {
            container,
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

use roxmltree::{Document, Node};

enum ParsedContent {
    Text(TextElement),
    List(ListElement),
}

fn parse_slide_xml(xml_data: &[u8]) -> Result<Vec<SlideElement>> {
    let xml_str =
        std::str::from_utf8(xml_data).map_err(|_| KreuzbergError::parsing("Invalid UTF-8 in slide XML".to_string()))?;

    let doc =
        Document::parse(xml_str).map_err(|e| KreuzbergError::parsing(format!("Failed to parse slide XML: {}", e)))?;

    let root = doc.root_element();
    let ns = root.tag_name().namespace();

    let c_sld = root
        .descendants()
        .find(|n| n.tag_name().name() == "cSld" && n.tag_name().namespace() == ns)
        .ok_or_else(|| KreuzbergError::parsing("No <p:cSld> tag found".to_string()))?;

    let sp_tree = c_sld
        .children()
        .find(|n| n.tag_name().name() == "spTree" && n.tag_name().namespace() == ns)
        .ok_or_else(|| KreuzbergError::parsing("No <p:spTree> tag found".to_string()))?;

    let mut elements = Vec::new();
    for child_node in sp_tree.children().filter(|n| n.is_element()) {
        elements.extend(parse_group(&child_node)?);
    }

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
        "sp" => {
            let position = extract_position(node);
            match parse_sp(node)? {
                ParsedContent::Text(text) => elements.push(SlideElement::Text(text, position)),
                ParsedContent::List(list) => elements.push(SlideElement::List(list, position)),
            }
        }
        "graphicFrame" => {
            if let Some(graphic_element) = parse_graphic_frame(node)? {
                elements.push(SlideElement::Table(graphic_element, position));
            }
        }
        "pic" => {
            let image_reference = parse_pic(node)?;
            elements.push(SlideElement::Image(image_reference, position));
        }
        "grpSp" => {
            for child in node.children().filter(|n| n.is_element()) {
                elements.extend(parse_group(&child)?);
            }
        }
        _ => elements.push(SlideElement::Unknown),
    }

    Ok(elements)
}

fn parse_sp(sp_node: &Node) -> Result<ParsedContent> {
    let tx_body_node = sp_node
        .children()
        .find(|n| n.tag_name().name() == "txBody" && n.tag_name().namespace() == Some(P_NAMESPACE))
        .ok_or_else(|| KreuzbergError::parsing("No txBody found".to_string()))?;

    let is_list = tx_body_node.descendants().any(|n| {
        n.is_element()
            && n.tag_name().name() == "pPr"
            && n.tag_name().namespace() == Some(A_NAMESPACE)
            && (n.attribute("lvl").is_some()
                || n.children().any(|child| {
                    child.is_element()
                        && (child.tag_name().name() == "buAutoNum" || child.tag_name().name() == "buChar")
                }))
    });

    if is_list {
        Ok(ParsedContent::List(parse_list(&tx_body_node)?))
    } else {
        Ok(ParsedContent::Text(parse_text(&tx_body_node)?))
    }
}

fn parse_text(tx_body_node: &Node) -> Result<TextElement> {
    let mut runs = Vec::new();

    for p_node in tx_body_node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "p" && n.tag_name().namespace() == Some(A_NAMESPACE))
    {
        let mut paragraph_runs = parse_paragraph(&p_node, true)?;
        runs.append(&mut paragraph_runs);
    }

    Ok(TextElement { runs })
}

fn parse_graphic_frame(node: &Node) -> Result<Option<TableElement>> {
    let graphic_data_node = node.descendants().find(|n| {
        n.is_element()
            && n.tag_name().name() == "graphicData"
            && n.tag_name().namespace() == Some(A_NAMESPACE)
            && n.attribute("uri") == Some("http://schemas.openxmlformats.org/drawingml/2006/table")
    });

    if let Some(graphic_data) = graphic_data_node
        && let Some(tbl_node) = graphic_data
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "tbl" && n.tag_name().namespace() == Some(A_NAMESPACE))
    {
        let table = parse_table(&tbl_node)?;
        return Ok(Some(table));
    }

    Ok(None)
}

fn parse_table(tbl_node: &Node) -> Result<TableElement> {
    let mut rows = Vec::new();

    for tr_node in tbl_node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "tr" && n.tag_name().namespace() == Some(A_NAMESPACE))
    {
        let row = parse_table_row(&tr_node)?;
        rows.push(row);
    }

    Ok(TableElement { rows })
}

fn parse_table_row(tr_node: &Node) -> Result<TableRow> {
    let mut cells = Vec::new();

    for tc_node in tr_node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "tc" && n.tag_name().namespace() == Some(A_NAMESPACE))
    {
        let cell = parse_table_cell(&tc_node)?;
        cells.push(cell);
    }

    Ok(TableRow { cells })
}

fn parse_table_cell(tc_node: &Node) -> Result<TableCell> {
    let mut runs = Vec::new();

    if let Some(tx_body_node) = tc_node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "txBody" && n.tag_name().namespace() == Some(A_NAMESPACE))
    {
        for p_node in tx_body_node
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "p" && n.tag_name().namespace() == Some(A_NAMESPACE))
        {
            let mut paragraph_runs = parse_paragraph(&p_node, false)?;
            runs.append(&mut paragraph_runs);
        }
    }

    Ok(TableCell { runs })
}

fn parse_pic(pic_node: &Node) -> Result<ImageReference> {
    let blip_node = pic_node
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "blip" && n.tag_name().namespace() == Some(A_NAMESPACE))
        .ok_or_else(|| KreuzbergError::parsing("Image blip not found".to_string()))?;

    let embed_attr = blip_node
        .attribute((RELS_NAMESPACE, "embed"))
        .or_else(|| blip_node.attribute("r:embed"))
        .ok_or_else(|| KreuzbergError::parsing("Image embed attribute not found".to_string()))?;

    let image_ref = ImageReference {
        id: embed_attr.to_string(),
        target: String::new(),
    };

    Ok(image_ref)
}

fn parse_list(tx_body_node: &Node) -> Result<ListElement> {
    let mut items = Vec::new();

    for p_node in tx_body_node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "p" && n.tag_name().namespace() == Some(A_NAMESPACE))
    {
        let (level, is_ordered) = parse_list_properties(&p_node)?;

        let runs = parse_paragraph(&p_node, true)?;

        items.push(ListItem {
            level,
            is_ordered,
            runs,
        });
    }

    Ok(ListElement { items })
}

fn parse_list_properties(p_node: &Node) -> Result<(u32, bool)> {
    let mut level = 1;
    let mut is_ordered = false;

    if let Some(p_pr_node) = p_node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "pPr" && n.tag_name().namespace() == Some(A_NAMESPACE))
    {
        if let Some(lvl_attr) = p_pr_node.attribute("lvl") {
            level = lvl_attr.parse::<u32>().unwrap_or(0) + 1;
        }

        is_ordered = p_pr_node.children().any(|n| {
            n.is_element() && n.tag_name().namespace() == Some(A_NAMESPACE) && n.tag_name().name() == "buAutoNum"
        });
    }

    Ok((level, is_ordered))
}

fn parse_paragraph(p_node: &Node, add_new_line: bool) -> Result<Vec<Run>> {
    let run_nodes: Vec<_> = p_node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "r" && n.tag_name().namespace() == Some(A_NAMESPACE))
        .collect();

    let count = run_nodes.len();
    let mut runs: Vec<Run> = Vec::new();

    for (idx, r_node) in run_nodes.iter().enumerate() {
        let mut run = parse_run(r_node)?;

        if add_new_line && idx == count - 1 {
            run.text.push('\n');
        }

        runs.push(run);
    }
    Ok(runs)
}

fn parse_run(r_node: &Node) -> Result<Run> {
    let mut text = String::new();
    let mut formatting = Formatting::default();

    if let Some(r_pr_node) = r_node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "rPr" && n.tag_name().namespace() == Some(A_NAMESPACE))
    {
        if let Some(b_attr) = r_pr_node.attribute("b") {
            formatting.bold = b_attr == "1" || b_attr.eq_ignore_ascii_case("true");
        }
        if let Some(i_attr) = r_pr_node.attribute("i") {
            formatting.italic = i_attr == "1" || i_attr.eq_ignore_ascii_case("true");
        }
        if let Some(u_attr) = r_pr_node.attribute("u") {
            formatting.underlined = u_attr != "none";
        }
        if let Some(lang_attr) = r_pr_node.attribute("lang") {
            formatting.lang = lang_attr.to_string();
        }
    }

    if let Some(t_node) = r_node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "t" && n.tag_name().namespace() == Some(A_NAMESPACE))
        && let Some(t) = t_node.text()
    {
        text.push_str(t);
    }
    Ok(Run { text, formatting })
}

fn extract_position(node: &Node) -> ElementPosition {
    let default = ElementPosition::default();

    node.descendants()
        .find(|n| n.tag_name().namespace() == Some(A_NAMESPACE) && n.tag_name().name() == "xfrm")
        .and_then(|xfrm| {
            let x = xfrm
                .children()
                .find(|n| n.tag_name().name() == "off" && n.tag_name().namespace() == Some(A_NAMESPACE))
                .and_then(|off| off.attribute("x")?.parse::<i64>().ok())?;

            let y = xfrm
                .children()
                .find(|n| n.tag_name().name() == "off" && n.tag_name().namespace() == Some(A_NAMESPACE))
                .and_then(|off| off.attribute("y")?.parse::<i64>().ok())?;

            Some(ElementPosition { x, y })
        })
        .unwrap_or(default)
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
            let normalized_target = target.strip_prefix('/').unwrap_or(target);
            let final_path = if normalized_target.starts_with("ppt/") {
                normalized_target.to_string()
            } else {
                format!("ppt/{}", normalized_target)
            };
            slide_paths.push(final_path);
        }
    }

    Ok(slide_paths)
}

/// Extract comprehensive metadata from PPTX using office_metadata module
fn extract_metadata(archive: &mut ZipArchive<File>) -> PptxMetadata {
    #[cfg(feature = "office")]
    {
        let mut metadata_map = HashMap::new();

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

        if let Ok(custom) = extract_custom_properties(archive) {
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

        PptxMetadata { fonts: Vec::new() }
    }

    #[cfg(not(feature = "office"))]
    {
        PptxMetadata { fonts: Vec::new() }
    }
}

fn extract_all_notes(container: &mut PptxContainer) -> Result<HashMap<u32, String>> {
    let mut notes = HashMap::new();

    let slide_paths: Vec<String> = container.slide_paths().to_vec();

    for (i, slide_path) in slide_paths.iter().enumerate() {
        let notes_path = slide_path.replace("slides/slide", "notesSlides/notesSlide");
        if let Ok(notes_xml) = container.read_file(&notes_path)
            && let Ok(note_text) = extract_notes_text(&notes_xml)
        {
            notes.insert((i + 1) as u32, note_text);
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

pub fn extract_pptx_from_path(path: &str, extract_images: bool) -> Result<PptxExtractionResult> {
    let config = ParserConfig {
        extract_images,
        ..Default::default()
    };

    let mut container = PptxContainer::open(path)?;

    let metadata = extract_metadata(&mut container.archive);

    let notes = extract_all_notes(&mut container)?;

    let mut iterator = SlideIterator::new(container);
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
                assert_eq!(text.runs[0].text, "Test Text\n");
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
        assert!(!config.include_slide_comment);
    }

    fn create_pptx_with_table(rows: Vec<Vec<&str>>) -> Vec<u8> {
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
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
            )
            .unwrap();

            zip.start_file("ppt/_rels/presentation.xml.rels", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#,
            )
            .unwrap();

            let mut table_xml = String::from(
                r#"<a:tbl>
                <a:tblGrid>"#,
            );
            if !rows.is_empty() {
                for _ in 0..rows[0].len() {
                    table_xml.push_str(r#"<a:gridCol w="2000000"/>"#);
                }
            }
            table_xml.push_str("</a:tblGrid>");

            for row in rows {
                table_xml.push_str(r#"<a:tr h="370840">"#);
                for cell in row {
                    table_xml.push_str(&format!(
                        r#"<a:tc>
                        <a:txBody>
                            <a:p>
                                <a:r>
                                    <a:t>{}</a:t>
                                </a:r>
                            </a:p>
                        </a:txBody>
                    </a:tc>"#,
                        cell
                    ));
                }
                table_xml.push_str("</a:tr>");
            }
            table_xml.push_str("</a:tbl>");

            let slide_xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld>
        <p:spTree>
            <p:graphicFrame>
                <p:xfrm>
                    <a:off x="1000000" y="2000000"/>
                    <a:ext cx="8000000" cy="4000000"/>
                </p:xfrm>
                <a:graphic>
                    <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
                        {}
                    </a:graphicData>
                </a:graphic>
            </p:graphicFrame>
        </p:spTree>
    </p:cSld>
</p:sld>"#,
                table_xml
            );

            zip.start_file("ppt/slides/slide1.xml", options).unwrap();
            zip.write_all(slide_xml.as_bytes()).unwrap();

            zip.start_file("docProps/core.xml", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Test Table</dc:title>
</cp:coreProperties>"#,
            )
            .unwrap();

            let _ = zip.finish().unwrap();
        }
        buffer
    }

    fn create_pptx_with_lists(list_items: Vec<(usize, bool, &str)>) -> Vec<u8> {
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
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
            )
            .unwrap();

            zip.start_file("ppt/_rels/presentation.xml.rels", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#,
            )
            .unwrap();

            let mut list_xml = String::new();
            for (level, is_ordered, text) in list_items {
                let indent = (level - 1) * 457200;
                let lvl_attr = level - 1;
                let bullet_section = if is_ordered {
                    format!(
                        r#"<a:pPr lvl="{}"><a:buAutoNum type="arabicPeriod"/></a:pPr>"#,
                        lvl_attr
                    )
                } else {
                    format!(
                        r#"<a:pPr lvl="{}" marL="{}"><a:buFont typeface="Arial"/><a:buChar char="•"/></a:pPr>"#,
                        lvl_attr, indent
                    )
                };

                list_xml.push_str(&format!(
                    r#"<p:sp>
                    <p:spPr>
                        <a:xfrm>
                            <a:off x="1000000" y="1000000"/>
                            <a:ext cx="6000000" cy="1000000"/>
                        </a:xfrm>
                    </p:spPr>
                    <p:txBody>
                        <a:bodyPr/>
                        <a:lstStyle/>
                        <a:p>
                            {}
                            <a:r>
                                <a:t>{}</a:t>
                            </a:r>
                        </a:p>
                    </p:txBody>
                </p:sp>"#,
                    bullet_section, text
                ));
            }

            let slide_xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld>
        <p:spTree>
            {}
        </p:spTree>
    </p:cSld>
</p:sld>"#,
                list_xml
            );

            zip.start_file("ppt/slides/slide1.xml", options).unwrap();
            zip.write_all(slide_xml.as_bytes()).unwrap();

            zip.start_file("docProps/core.xml", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Test Lists</dc:title>
</cp:coreProperties>"#,
            )
            .unwrap();

            let _ = zip.finish().unwrap();
        }
        buffer
    }

    fn create_pptx_with_images() -> Vec<u8> {
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
    <Default Extension="png" ContentType="image/png"/>
    <Default Extension="jpeg" ContentType="image/jpeg"/>
</Types>"#,
            )
            .unwrap();

            zip.start_file("ppt/presentation.xml", options).unwrap();
            zip.write_all(b"<?xml version=\"1.0\"?><presentation/>").unwrap();

            zip.start_file("_rels/.rels", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
            )
            .unwrap();

            zip.start_file("ppt/_rels/presentation.xml.rels", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#,
            )
            .unwrap();

            zip.start_file("ppt/slides/_rels/slide1.xml.rels", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
    <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image2.jpeg"/>
</Relationships>"#,
            )
            .unwrap();

            let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
    <p:cSld>
        <p:spTree>
            <p:pic>
                <p:nvPicPr>
                    <p:cNvPr id="1" name="Image1"/>
                </p:nvPicPr>
                <p:blipFill>
                    <a:blip r:embed="rId1"/>
                </p:blipFill>
                <p:spPr>
                    <a:xfrm>
                        <a:off x="1000000" y="1000000"/>
                        <a:ext cx="2000000" cy="2000000"/>
                    </a:xfrm>
                </p:spPr>
            </p:pic>
            <p:pic>
                <p:nvPicPr>
                    <p:cNvPr id="2" name="Image2"/>
                </p:nvPicPr>
                <p:blipFill>
                    <a:blip r:embed="rId2"/>
                </p:blipFill>
                <p:spPr>
                    <a:xfrm>
                        <a:off x="4000000" y="1000000"/>
                        <a:ext cx="2000000" cy="2000000"/>
                    </a:xfrm>
                </p:spPr>
            </p:pic>
        </p:spTree>
    </p:cSld>
</p:sld>"#;

            zip.start_file("ppt/slides/slide1.xml", options).unwrap();
            zip.write_all(slide_xml.as_bytes()).unwrap();

            let png_bytes: Vec<u8> = vec![
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00,
                0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, 0x00,
                0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
            ];
            zip.start_file("ppt/media/image1.png", options).unwrap();
            zip.write_all(&png_bytes).unwrap();

            let jpeg_bytes: Vec<u8> = vec![
                0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00,
                0x01, 0x00, 0x00, 0xFF, 0xD9,
            ];
            zip.start_file("ppt/media/image2.jpeg", options).unwrap();
            zip.write_all(&jpeg_bytes).unwrap();

            zip.start_file("docProps/core.xml", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Test Images</dc:title>
</cp:coreProperties>"#,
            )
            .unwrap();

            let _ = zip.finish().unwrap();
        }
        buffer
    }

    fn create_pptx_with_formatting() -> Vec<u8> {
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
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
            )
            .unwrap();

            zip.start_file("ppt/_rels/presentation.xml.rels", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#,
            )
            .unwrap();

            let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld>
        <p:spTree>
            <p:sp>
                <p:spPr>
                    <a:xfrm>
                        <a:off x="1000000" y="1000000"/>
                        <a:ext cx="6000000" cy="1000000"/>
                    </a:xfrm>
                </p:spPr>
                <p:txBody>
                    <a:p>
                        <a:r>
                            <a:rPr b="1"/>
                            <a:t>Bold text</a:t>
                        </a:r>
                    </a:p>
                </p:txBody>
            </p:sp>
            <p:sp>
                <p:spPr>
                    <a:xfrm>
                        <a:off x="1000000" y="2000000"/>
                        <a:ext cx="6000000" cy="1000000"/>
                    </a:xfrm>
                </p:spPr>
                <p:txBody>
                    <a:p>
                        <a:r>
                            <a:rPr i="1"/>
                            <a:t>Italic text</a:t>
                        </a:r>
                    </a:p>
                </p:txBody>
            </p:sp>
            <p:sp>
                <p:spPr>
                    <a:xfrm>
                        <a:off x="1000000" y="3000000"/>
                        <a:ext cx="6000000" cy="1000000"/>
                    </a:xfrm>
                </p:spPr>
                <p:txBody>
                    <a:p>
                        <a:r>
                            <a:rPr u="sng"/>
                            <a:t>Underline text</a:t>
                        </a:r>
                    </a:p>
                </p:txBody>
            </p:sp>
            <p:sp>
                <p:spPr>
                    <a:xfrm>
                        <a:off x="1000000" y="4000000"/>
                        <a:ext cx="6000000" cy="1000000"/>
                    </a:xfrm>
                </p:spPr>
                <p:txBody>
                    <a:p>
                        <a:r>
                            <a:rPr b="1" i="1"/>
                            <a:t>Bold italic text</a:t>
                        </a:r>
                    </a:p>
                </p:txBody>
            </p:sp>
        </p:spTree>
    </p:cSld>
</p:sld>"#;

            zip.start_file("ppt/slides/slide1.xml", options).unwrap();
            zip.write_all(slide_xml.as_bytes()).unwrap();

            zip.start_file("docProps/core.xml", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Test Formatting</dc:title>
</cp:coreProperties>"#,
            )
            .unwrap();

            let _ = zip.finish().unwrap();
        }
        buffer
    }

    #[test]
    fn test_table_extraction_with_headers_succeeds() {
        let pptx_bytes = create_pptx_with_table(vec![
            vec!["Header 1", "Header 2", "Header 3"],
            vec!["Data 1", "Data 2", "Data 3"],
            vec!["Row 2 Col 1", "Row 2 Col 2", "Row 2 Col 3"],
        ]);

        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert_eq!(result.table_count, 1, "Should detect one table");
        assert!(result.content.contains("<table>"), "Should contain table tag");
        assert!(
            result.content.contains("<th>Header 1</th>"),
            "Should render first header"
        );
        assert!(
            result.content.contains("<th>Header 2</th>"),
            "Should render second header"
        );
        assert!(
            result.content.contains("<th>Header 3</th>"),
            "Should render third header"
        );
        assert!(result.content.contains("<td>Data 1</td>"), "Should render data cell");
        assert!(
            result.content.contains("<td>Row 2 Col 2</td>"),
            "Should render second row data"
        );
    }

    #[test]
    fn test_table_extraction_multirow_multicolumn_succeeds() {
        let pptx_bytes = create_pptx_with_table(vec![
            vec!["A1", "B1", "C1", "D1"],
            vec!["A2", "B2", "C2", "D2"],
            vec!["A3", "B3", "C3", "D3"],
            vec!["A4", "B4", "C4", "D4"],
        ]);

        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert_eq!(result.table_count, 1, "Should detect one table");
        assert!(result.content.contains("<tr>"), "Should contain table rows");
        assert!(result.content.contains("A1"), "Should contain first row data");
        assert!(result.content.contains("D4"), "Should contain last row data");

        let tr_count = result.content.matches("<tr>").count();
        assert_eq!(tr_count, 4, "Should have 4 table rows");
    }

    #[test]
    fn test_table_counting_via_slide_metadata_succeeds() {
        let pptx_bytes = create_pptx_with_table(vec![vec!["Col1", "Col2"], vec!["Val1", "Val2"]]);

        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert_eq!(result.table_count, 1, "table_count should be 1");
    }

    #[test]
    fn test_table_markdown_rendering_with_special_chars() {
        let pptx_bytes = create_pptx_with_table(vec![
            vec!["Header with ampersand", "Header 2"],
            vec!["Cell data 1", "Cell data 2"],
        ]);

        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert!(result.content.contains("<table>"), "Should contain table tag");
        assert!(
            result.content.contains("<th>Header with ampersand</th>"),
            "Should contain header text"
        );
        assert!(
            result.content.contains("<td>Cell data 1</td>"),
            "Should contain cell data"
        );
    }

    #[test]
    fn test_table_extraction_empty_table_returns_one_count() {
        let pptx_bytes = create_pptx_with_table(vec![]);
        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert_eq!(result.table_count, 1, "Empty table structure should be detected");
        assert!(!result.content.contains("<td>"), "Empty table should have no cells");
    }

    #[test]
    fn test_list_extraction_ordered_list_succeeds() {
        let pptx_bytes = create_pptx_with_lists(vec![
            (1, true, "First item"),
            (1, true, "Second item"),
            (1, true, "Third item"),
        ]);

        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert!(
            result.content.contains("1. First item"),
            "Should contain ordered list item 1"
        );
        assert!(
            result.content.contains("1. Second item"),
            "Should contain ordered list item 2"
        );
        assert!(
            result.content.contains("1. Third item"),
            "Should contain ordered list item 3"
        );
    }

    #[test]
    fn test_list_extraction_unordered_list_succeeds() {
        let pptx_bytes = create_pptx_with_lists(vec![
            (1, false, "Bullet one"),
            (1, false, "Bullet two"),
            (1, false, "Bullet three"),
        ]);

        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert!(result.content.contains("- Bullet one"), "Should contain bullet point 1");
        assert!(result.content.contains("- Bullet two"), "Should contain bullet point 2");
        assert!(
            result.content.contains("- Bullet three"),
            "Should contain bullet point 3"
        );
    }

    #[test]
    fn test_list_extraction_nested_lists_with_indentation_succeeds() {
        let pptx_bytes = create_pptx_with_lists(vec![
            (1, false, "Level 1 Item"),
            (2, false, "Level 2 Item"),
            (3, false, "Level 3 Item"),
            (2, false, "Back to Level 2"),
            (1, false, "Back to Level 1"),
        ]);

        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert!(
            result.content.contains("- Level 1 Item"),
            "Should have level 1 with no indent"
        );
        assert!(
            result.content.contains("  - Level 2 Item"),
            "Should have level 2 with 2-space indent"
        );
        assert!(
            result.content.contains("    - Level 3 Item"),
            "Should have level 3 with 4-space indent"
        );
        assert!(
            result.content.contains("  - Back to Level 2"),
            "Should return to level 2 indent"
        );
        assert!(result.content.contains("- Back to Level 1"), "Should return to level 1");
    }

    #[test]
    fn test_list_extraction_mixed_ordered_unordered_succeeds() {
        let pptx_bytes = create_pptx_with_lists(vec![
            (1, true, "Ordered item 1"),
            (1, false, "Unordered item 1"),
            (1, true, "Ordered item 2"),
        ]);

        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert!(
            result.content.contains("1. Ordered item 1"),
            "Should render ordered list"
        );
        assert!(
            result.content.contains("- Unordered item 1"),
            "Should render unordered list"
        );
        assert!(
            result.content.contains("1. Ordered item 2"),
            "Should render ordered list again"
        );
    }

    #[test]
    fn test_image_extraction_from_slide_xml_succeeds() {
        let pptx_bytes = create_pptx_with_images();
        let result = extract_pptx_from_bytes(&pptx_bytes, true).unwrap();

        assert_eq!(result.image_count, 2, "Should detect 2 images");
        assert!(!result.images.is_empty(), "Should extract image data");
    }

    #[test]
    fn test_image_data_loading_from_zip_archive_succeeds() {
        let pptx_bytes = create_pptx_with_images();
        let result = extract_pptx_from_bytes(&pptx_bytes, true).unwrap();

        assert_eq!(result.images.len(), 2, "Should load 2 images");

        for (i, img) in result.images.iter().enumerate() {
            assert!(!img.data.is_empty(), "Image {} should have non-empty data", i);
        }
    }

    #[test]
    fn test_image_format_detection_succeeds() {
        let pptx_bytes = create_pptx_with_images();
        let result = extract_pptx_from_bytes(&pptx_bytes, true).unwrap();

        assert_eq!(result.images.len(), 2, "Should have 2 images");

        let formats: Vec<&str> = result.images.iter().map(|img| img.format.as_str()).collect();

        assert!(formats.contains(&"png"), "Should detect PNG format");
        assert!(formats.contains(&"jpeg"), "Should detect JPEG format");
    }

    #[test]
    fn test_image_counting_via_result_metadata_succeeds() {
        let pptx_bytes = create_pptx_with_images();
        let result = extract_pptx_from_bytes(&pptx_bytes, true).unwrap();

        assert_eq!(result.image_count, 2, "image_count should match actual images");
        assert_eq!(result.images.len(), 2, "images vector should have 2 elements");
    }

    #[test]
    fn test_image_extraction_disabled_returns_zero_images() {
        let pptx_bytes = create_pptx_with_images();
        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert_eq!(
            result.image_count, 2,
            "Should still count images even when not extracted"
        );
        assert_eq!(result.images.len(), 0, "Should not extract image data when disabled");
    }

    #[test]
    fn test_multiple_images_per_slide_extraction_succeeds() {
        let pptx_bytes = create_pptx_with_images();
        let result = extract_pptx_from_bytes(&pptx_bytes, true).unwrap();

        assert_eq!(result.slide_count, 1, "Should have 1 slide");
        assert_eq!(result.image_count, 2, "Single slide should contain 2 images");

        let indices: Vec<usize> = result.images.iter().map(|img| img.image_index).collect();
        assert_eq!(indices.len(), 2, "Should have 2 images with indices");
        assert_eq!(indices, vec![0, 1], "Should have sequential image indices");
    }

    #[test]
    fn test_formatting_bold_text_renders_as_markdown_bold() {
        let pptx_bytes = create_pptx_with_formatting();
        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert!(
            result.content.contains("**Bold text"),
            "Should render bold text with ** markers"
        );
    }

    #[test]
    fn test_formatting_italic_text_renders_as_markdown_italic() {
        let pptx_bytes = create_pptx_with_formatting();
        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert!(
            result.content.contains("*Italic text"),
            "Should render italic text with * markers"
        );
    }

    #[test]
    fn test_formatting_underline_text_renders_as_html_underline() {
        let pptx_bytes = create_pptx_with_formatting();
        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert!(
            result.content.contains("<u>Underline text"),
            "Should render underline with HTML tags"
        );
    }

    #[test]
    fn test_formatting_combined_bold_italic_renders_correctly() {
        let pptx_bytes = create_pptx_with_formatting();
        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert!(
            result.content.contains("***Bold italic text"),
            "Should render bold+italic with *** markers"
        );
    }

    #[test]
    fn test_run_render_underline_formatting() {
        let run = Run {
            text: "underlined".to_string(),
            formatting: Formatting {
                underlined: true,
                ..Default::default()
            },
        };
        assert_eq!(
            run.render_as_md(),
            "<u>underlined</u>",
            "Should wrap underlined text in <u> tags"
        );
    }

    #[test]
    fn test_run_render_all_formatting_combined() {
        let run = Run {
            text: "all formats".to_string(),
            formatting: Formatting {
                bold: true,
                italic: true,
                underlined: true,
                ..Default::default()
            },
        };
        let rendered = run.render_as_md();
        assert!(rendered.contains("***"), "Should have bold+italic markers");
        assert!(rendered.contains("<u>"), "Should have underline tags");
        assert!(rendered.contains("all formats"), "Should contain original text");
    }

    #[test]
    fn test_integration_complete_pptx_with_mixed_content_succeeds() {
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
    <Default Extension="png" ContentType="image/png"/>
</Types>"#,
            )
            .unwrap();

            zip.start_file("ppt/presentation.xml", options).unwrap();
            zip.write_all(b"<?xml version=\"1.0\"?><presentation/>").unwrap();

            zip.start_file("_rels/.rels", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
            )
            .unwrap();

            zip.start_file("ppt/_rels/presentation.xml.rels", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#,
            )
            .unwrap();

            zip.start_file("ppt/slides/_rels/slide1.xml.rels", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
</Relationships>"#,
            )
            .unwrap();

            let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
    <p:cSld>
        <p:spTree>
            <p:sp>
                <p:txBody>
                    <a:p>
                        <a:r>
                            <a:rPr b="1"/>
                            <a:t>Title with Bold</a:t>
                        </a:r>
                    </a:p>
                </p:txBody>
                <p:spPr>
                    <a:xfrm>
                        <a:off x="1000000" y="500000"/>
                    </a:xfrm>
                </p:spPr>
            </p:sp>
            <p:sp>
                <p:txBody>
                    <a:p>
                        <a:pPr lvl="0"><a:buChar char="•"/></a:pPr>
                        <a:r>
                            <a:t>List item one</a:t>
                        </a:r>
                    </a:p>
                </p:txBody>
                <p:spPr>
                    <a:xfrm>
                        <a:off x="1000000" y="1500000"/>
                    </a:xfrm>
                </p:spPr>
            </p:sp>
            <p:graphicFrame>
                <p:xfrm>
                    <a:off x="1000000" y="2500000"/>
                    <a:ext cx="4000000" cy="2000000"/>
                </p:xfrm>
                <a:graphic>
                    <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
                        <a:tbl>
                            <a:tblGrid>
                                <a:gridCol w="2000000"/>
                                <a:gridCol w="2000000"/>
                            </a:tblGrid>
                            <a:tr h="370840">
                                <a:tc>
                                    <a:txBody>
                                        <a:p>
                                            <a:r>
                                                <a:t>Header A</a:t>
                                            </a:r>
                                        </a:p>
                                    </a:txBody>
                                </a:tc>
                                <a:tc>
                                    <a:txBody>
                                        <a:p>
                                            <a:r>
                                                <a:t>Header B</a:t>
                                            </a:r>
                                        </a:p>
                                    </a:txBody>
                                </a:tc>
                            </a:tr>
                            <a:tr h="370840">
                                <a:tc>
                                    <a:txBody>
                                        <a:p>
                                            <a:r>
                                                <a:t>Data 1</a:t>
                                            </a:r>
                                        </a:p>
                                    </a:txBody>
                                </a:tc>
                                <a:tc>
                                    <a:txBody>
                                        <a:p>
                                            <a:r>
                                                <a:t>Data 2</a:t>
                                            </a:r>
                                        </a:p>
                                    </a:txBody>
                                </a:tc>
                            </a:tr>
                        </a:tbl>
                    </a:graphicData>
                </a:graphic>
            </p:graphicFrame>
            <p:pic>
                <p:nvPicPr>
                    <p:cNvPr id="1" name="TestImage"/>
                </p:nvPicPr>
                <p:blipFill>
                    <a:blip r:embed="rId1"/>
                </p:blipFill>
                <p:spPr>
                    <a:xfrm>
                        <a:off x="6000000" y="1000000"/>
                        <a:ext cx="2000000" cy="2000000"/>
                    </a:xfrm>
                </p:spPr>
            </p:pic>
        </p:spTree>
    </p:cSld>
</p:sld>"#;

            zip.start_file("ppt/slides/slide1.xml", options).unwrap();
            zip.write_all(slide_xml.as_bytes()).unwrap();

            let png_bytes: Vec<u8> = vec![
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00,
                0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, 0x00,
                0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
            ];
            zip.start_file("ppt/media/image1.png", options).unwrap();
            zip.write_all(&png_bytes).unwrap();

            zip.start_file("docProps/core.xml", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Integration Test</dc:title>
</cp:coreProperties>"#,
            )
            .unwrap();

            let _ = zip.finish().unwrap();
        }

        let result = extract_pptx_from_bytes(&buffer, true).unwrap();

        assert!(
            result.content.contains("**Title with Bold"),
            "Should contain formatted title"
        );
        assert!(result.content.contains("- List item one"), "Should contain list item");
        assert!(result.content.contains("<table>"), "Should contain table");
        assert!(result.content.contains("Header A"), "Should contain table header");
        assert!(result.content.contains("Data 1"), "Should contain table data");

        assert_eq!(result.slide_count, 1, "Should have 1 slide");
        assert_eq!(result.table_count, 1, "Should detect 1 table");
        assert_eq!(result.image_count, 1, "Should detect 1 image");
        assert_eq!(result.images.len(), 1, "Should extract 1 image");
    }

    #[test]
    fn test_integration_position_based_sorting_orders_elements_correctly() {
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
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
            )
            .unwrap();

            zip.start_file("ppt/_rels/presentation.xml.rels", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#,
            )
            .unwrap();

            let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld>
        <p:spTree>
            <p:sp>
                <p:txBody>
                    <a:p>
                        <a:r>
                            <a:t>Bottom Right</a:t>
                        </a:r>
                    </a:p>
                </p:txBody>
                <p:spPr>
                    <a:xfrm>
                        <a:off x="5000000" y="3000000"/>
                    </a:xfrm>
                </p:spPr>
            </p:sp>
            <p:sp>
                <p:txBody>
                    <a:p>
                        <a:r>
                            <a:t>Top Left</a:t>
                        </a:r>
                    </a:p>
                </p:txBody>
                <p:spPr>
                    <a:xfrm>
                        <a:off x="1000000" y="1000000"/>
                    </a:xfrm>
                </p:spPr>
            </p:sp>
            <p:sp>
                <p:txBody>
                    <a:p>
                        <a:r>
                            <a:t>Top Right</a:t>
                        </a:r>
                    </a:p>
                </p:txBody>
                <p:spPr>
                    <a:xfrm>
                        <a:off x="5000000" y="1000000"/>
                    </a:xfrm>
                </p:spPr>
            </p:sp>
            <p:sp>
                <p:txBody>
                    <a:p>
                        <a:r>
                            <a:t>Bottom Left</a:t>
                        </a:r>
                    </a:p>
                </p:txBody>
                <p:spPr>
                    <a:xfrm>
                        <a:off x="1000000" y="3000000"/>
                    </a:xfrm>
                </p:spPr>
            </p:sp>
        </p:spTree>
    </p:cSld>
</p:sld>"#;

            zip.start_file("ppt/slides/slide1.xml", options).unwrap();
            zip.write_all(slide_xml.as_bytes()).unwrap();

            zip.start_file("docProps/core.xml", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Position Test</dc:title>
</cp:coreProperties>"#,
            )
            .unwrap();

            let _ = zip.finish().unwrap();
        }

        let result = extract_pptx_from_bytes(&buffer, false).unwrap();

        let content = result.content;
        let top_left_pos = content.find("Top Left").unwrap();
        let top_right_pos = content.find("Top Right").unwrap();
        let bottom_left_pos = content.find("Bottom Left").unwrap();
        let bottom_right_pos = content.find("Bottom Right").unwrap();

        assert!(
            top_left_pos < top_right_pos,
            "Top Left should appear before Top Right (same Y, lower X)"
        );
        assert!(
            top_right_pos < bottom_left_pos,
            "Top row should appear before bottom row"
        );
        assert!(
            bottom_left_pos < bottom_right_pos,
            "Bottom Left should appear before Bottom Right (same Y, lower X)"
        );
    }

    #[test]
    fn test_integration_slide_notes_extraction_succeeds() {
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
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
            )
            .unwrap();

            zip.start_file("ppt/_rels/presentation.xml.rels", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#,
            )
            .unwrap();

            zip.start_file("ppt/slides/_rels/slide1.xml.rels", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide" Target="../notesSlides/notesSlide1.xml"/>
</Relationships>"#,
            )
            .unwrap();

            let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld>
        <p:spTree>
            <p:sp>
                <p:txBody>
                    <a:p>
                        <a:r>
                            <a:t>Slide Content</a:t>
                        </a:r>
                    </a:p>
                </p:txBody>
            </p:sp>
        </p:spTree>
    </p:cSld>
</p:sld>"#;

            zip.start_file("ppt/slides/slide1.xml", options).unwrap();
            zip.write_all(slide_xml.as_bytes()).unwrap();

            let notes_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:notes xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
         xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld>
        <p:spTree>
            <p:sp>
                <p:txBody>
                    <a:p>
                        <a:r>
                            <a:t>This is a speaker note for testing</a:t>
                        </a:r>
                    </a:p>
                </p:txBody>
            </p:sp>
        </p:spTree>
    </p:cSld>
</p:notes>"#;

            zip.start_file("ppt/notesSlides/notesSlide1.xml", options).unwrap();
            zip.write_all(notes_xml.as_bytes()).unwrap();

            zip.start_file("docProps/core.xml", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Notes Test</dc:title>
</cp:coreProperties>"#,
            )
            .unwrap();

            let _ = zip.finish().unwrap();
        }

        let result = extract_pptx_from_bytes(&buffer, false).unwrap();

        assert!(result.content.contains("Slide Content"), "Should contain slide content");
        assert!(result.content.contains("### Notes:"), "Should contain notes header");
        assert!(
            result.content.contains("This is a speaker note for testing"),
            "Should extract speaker notes"
        );
    }

    #[test]
    fn test_integration_metadata_extraction_complete() {
        let pptx_bytes = create_test_pptx_bytes(vec!["Content"]);
        let result = extract_pptx_from_bytes(&pptx_bytes, false).unwrap();

        assert_eq!(result.metadata.title, Some("Test Presentation".to_string()));
        assert_eq!(result.metadata.author, Some("Test Author".to_string()));
        assert_eq!(result.metadata.description, Some("Test Description".to_string()));
        assert_eq!(result.metadata.summary, Some("Test Subject".to_string()));
    }
}
