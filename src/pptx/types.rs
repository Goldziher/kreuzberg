use pyo3::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ElementPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Default)]
pub struct Formatting {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub font_size: Option<f32>,
}

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

#[derive(Debug, Clone)]
pub struct TextElement {
    pub runs: Vec<Run>,
}

#[derive(Debug, Clone)]
pub struct ListItem {
    pub level: u32,
    pub is_ordered: bool,
    pub runs: Vec<Run>,
}

#[derive(Debug, Clone)]
pub struct ListElement {
    pub items: Vec<ListItem>,
}

#[derive(Debug, Clone)]
pub struct TableCell {
    pub runs: Vec<Run>,
}

#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

#[derive(Debug, Clone)]
pub struct TableElement {
    pub rows: Vec<TableRow>,
}

#[derive(Debug, Clone)]
pub struct ImageReference {
    pub id: String,
    pub target: String,
}

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

#[derive(Debug)]
pub struct Slide {
    pub slide_number: u32,
    pub elements: Vec<SlideElement>,
    pub images: Vec<ImageReference>,
    pub image_data: HashMap<String, Vec<u8>>,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_position_default() {
        let pos = ElementPosition::default();
        assert_eq!(pos.x, 0);
        assert_eq!(pos.y, 0);
    }

    #[test]
    fn test_element_position_equality() {
        let pos1 = ElementPosition { x: 100, y: 200 };
        let pos2 = ElementPosition { x: 100, y: 200 };
        let pos3 = ElementPosition { x: 100, y: 300 };
        assert_eq!(pos1, pos2);
        assert_ne!(pos1, pos3);
    }

    #[test]
    fn test_formatting_default() {
        let formatting = Formatting::default();
        assert!(!formatting.bold);
        assert!(!formatting.italic);
        assert!(!formatting.underline);
        assert!(formatting.font_size.is_none());
    }

    #[test]
    fn test_formatting_with_values() {
        let formatting = Formatting {
            bold: true,
            italic: true,
            underline: false,
            font_size: Some(12.0),
        };
        assert!(formatting.bold);
        assert!(formatting.italic);
        assert!(!formatting.underline);
        assert_eq!(formatting.font_size, Some(12.0));
    }

    #[test]
    fn test_run_extract() {
        let run = Run {
            text: "Hello World".to_string(),
            formatting: Formatting::default(),
        };
        assert_eq!(run.extract(), "Hello World");
    }

    #[test]
    fn test_run_render_plain() {
        let run = Run {
            text: "Hello".to_string(),
            formatting: Formatting::default(),
        };
        assert_eq!(run.render_as_md(), "Hello");
    }

    #[test]
    fn test_run_render_bold() {
        let run = Run {
            text: "Hello".to_string(),
            formatting: Formatting {
                bold: true,
                italic: false,
                underline: false,
                font_size: None,
            },
        };
        assert_eq!(run.render_as_md(), "**Hello**");
    }

    #[test]
    fn test_run_render_italic() {
        let run = Run {
            text: "Hello".to_string(),
            formatting: Formatting {
                bold: false,
                italic: true,
                underline: false,
                font_size: None,
            },
        };
        assert_eq!(run.render_as_md(), "*Hello*");
    }

    #[test]
    fn test_run_render_bold_italic() {
        let run = Run {
            text: "Hello".to_string(),
            formatting: Formatting {
                bold: true,
                italic: true,
                underline: false,
                font_size: None,
            },
        };
        assert_eq!(run.render_as_md(), "***Hello***");
    }

    #[test]
    fn test_text_element() {
        let text = TextElement {
            runs: vec![Run {
                text: "Test".to_string(),
                formatting: Formatting::default(),
            }],
        };
        assert_eq!(text.runs.len(), 1);
        assert_eq!(text.runs[0].text, "Test");
    }

    #[test]
    fn test_list_item() {
        let item = ListItem {
            level: 1,
            is_ordered: true,
            runs: vec![],
        };
        assert_eq!(item.level, 1);
        assert!(item.is_ordered);
        assert_eq!(item.runs.len(), 0);
    }

    #[test]
    fn test_list_element() {
        let list = ListElement { items: vec![] };
        assert_eq!(list.items.len(), 0);
    }

    #[test]
    fn test_table_cell() {
        let cell = TableCell { runs: vec![] };
        assert_eq!(cell.runs.len(), 0);
    }

    #[test]
    fn test_table_row() {
        let row = TableRow { cells: vec![] };
        assert_eq!(row.cells.len(), 0);
    }

    #[test]
    fn test_table_element() {
        let table = TableElement { rows: vec![] };
        assert_eq!(table.rows.len(), 0);
    }

    #[test]
    fn test_image_reference() {
        let img = ImageReference {
            id: "rId1".to_string(),
            target: "media/image1.png".to_string(),
        };
        assert_eq!(img.id, "rId1");
        assert_eq!(img.target, "media/image1.png");
    }

    #[test]
    fn test_slide_element_position() {
        let pos = ElementPosition { x: 100, y: 200 };
        let text = SlideElement::Text(TextElement { runs: vec![] }, pos.clone());
        assert_eq!(text.position(), pos);

        let table = SlideElement::Table(TableElement { rows: vec![] }, pos.clone());
        assert_eq!(table.position(), pos);

        let img = SlideElement::Image(
            ImageReference {
                id: "rId1".to_string(),
                target: "media/image1.png".to_string(),
            },
            pos.clone(),
        );
        assert_eq!(img.position(), pos);

        let list = SlideElement::List(ListElement { items: vec![] }, pos.clone());
        assert_eq!(list.position(), pos);
    }

    #[test]
    fn test_pptx_error_display() {
        let err = PptxError::ParseError("test error");
        assert_eq!(err.to_string(), "Parse error: test error");
    }

    #[test]
    fn test_pptx_error_from_utf8() {
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        let result = std::str::from_utf8(&invalid_utf8);
        assert!(result.is_err());
        let err: PptxError = result.unwrap_err().into();
        assert!(err.to_string().contains("UTF-8"));
    }
}
