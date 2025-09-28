//! Core data structures for PPTX processing

use pyo3::prelude::*;
use std::collections::HashMap;

/// Position of an element on a slide
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ElementPosition {
    pub x: i32,
    pub y: i32,
}

/// Text formatting information
#[derive(Debug, Clone, Default)]
pub struct Formatting {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub font_size: Option<f32>,
}

/// A text run with formatting
#[derive(Debug, Clone)]
pub struct Run {
    pub text: String,
    pub formatting: Formatting,
}

impl Run {
    pub fn extract(&self) -> String {
        self.text.clone()
    }

    pub fn render_as_md(&self) -> String {
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

/// Text element containing multiple runs
#[derive(Debug, Clone)]
pub struct TextElement {
    pub runs: Vec<Run>,
}

/// List item with level and ordering
#[derive(Debug, Clone)]
pub struct ListItem {
    pub level: u32,
    pub is_ordered: bool,
    pub runs: Vec<Run>,
}

/// List element containing items
#[derive(Debug, Clone)]
pub struct ListElement {
    pub items: Vec<ListItem>,
}

/// Table cell containing text runs
#[derive(Debug, Clone)]
pub struct TableCell {
    pub runs: Vec<Run>,
}

/// Table row containing cells
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

/// Table element containing rows
#[derive(Debug, Clone)]
pub struct TableElement {
    pub rows: Vec<TableRow>,
}

/// Image reference
#[derive(Debug, Clone)]
pub struct ImageReference {
    pub id: String,
    pub target: String,
}

/// Slide element variants
#[derive(Debug, Clone)]
pub enum SlideElement {
    Text(TextElement, ElementPosition),
    Table(TableElement, ElementPosition),
    Image(ImageReference, ElementPosition),
    List(ListElement, ElementPosition),
}

impl SlideElement {
    pub fn position(&self) -> ElementPosition {
        match self {
            SlideElement::Text(_, pos)
            | SlideElement::Table(_, pos)
            | SlideElement::Image(_, pos)
            | SlideElement::List(_, pos) => pos.clone(),
        }
    }
}

/// Individual slide with parsed elements
#[derive(Debug)]
pub struct Slide {
    pub slide_number: u32,
    pub elements: Vec<SlideElement>,
    pub images: Vec<ImageReference>,
    pub image_data: HashMap<String, Vec<u8>>,
}

/// PPTX metadata DTO (Data Transfer Object) extracted from docProps
#[pyclass]
#[derive(Debug, Clone)]
pub struct PptxMetadataDTO {
    #[pyo3(get)]
    pub title: Option<String>,
    #[pyo3(get)]
    pub author: Option<String>,
    #[pyo3(get)]
    pub description: Option<String>,
    #[pyo3(get)]
    pub summary: Option<String>,
    #[pyo3(get)]
    pub fonts: Vec<String>,
}

/// Final extraction result DTO (Data Transfer Object)
#[pyclass]
#[derive(Debug, Clone)]
pub struct PptxExtractionResultDTO {
    #[pyo3(get)]
    pub content: String,
    #[pyo3(get)]
    pub metadata: PptxMetadataDTO,
    #[pyo3(get)]
    pub slide_count: usize,
    #[pyo3(get)]
    pub image_count: usize,
    #[pyo3(get)]
    pub table_count: usize,
    #[pyo3(get)]
    pub images: Vec<ExtractedImageDTO>,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct ExtractedImageDTO {
    #[pyo3(get)]
    pub data: Vec<u8>,
    #[pyo3(get)]
    pub format: String,
    #[pyo3(get)]
    pub slide_number: Option<usize>,
}

/// Error types for PPTX processing
#[derive(Debug, thiserror::Error)]
pub enum PptxError {
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("XML parse error: {0}")]
    Xml(#[from] roxmltree::Error),
    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    ParseError(&'static str),
}

pub type Result<T> = std::result::Result<T, PptxError>;
