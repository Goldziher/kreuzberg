//! Internal types for PPTX extraction.
//!
//! This module defines the internal data structures used to represent
//! slide elements, formatting, and text runs during XML parsing.

use ahash::AHashMap;

use crate::error::Result;

use super::content_builder::ContentBuilder;
use super::{PptxInternalSlideElement, join_runs_with_spacing, parser};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ElementPosition {
    pub(super) x: i64,
    pub(super) y: i64,
    /// Width in EMUs (from `a:ext cx`).
    pub(super) cx: i64,
    /// Height in EMUs (from `a:ext cy`).
    pub(super) cy: i64,
}

#[derive(Debug, Clone, Default)]
pub(super) struct Formatting {
    pub(super) bold: bool,
    pub(super) italic: bool,
    pub(super) underlined: bool,
    pub(super) strikethrough: bool,
    /// Font size in hundredths of a point (from `a:rPr sz`).
    pub(super) font_size: Option<u32>,
    pub(super) lang: String,
}

#[derive(Debug, Clone)]
pub(super) struct Run {
    pub(super) text: String,
    pub(super) formatting: Formatting,
    /// Relationship ID for a hyperlink attached to this run (`a:hlinkClick r:id`).
    pub(super) hyperlink_id: Option<String>,
    /// LaTeX rendering of an OMML `m:oMath`/`m:oMathPara` element and whether it
    /// was display math (`m:oMathPara`, `true`) or inline math (`m:oMath`, `false`).
    /// When `Some`, `text` is empty and rendering must use `math_latex` instead.
    pub(super) math_latex: Option<(String, bool)>,
}

impl Run {
    pub(super) fn extract(&self) -> String {
        if let Some((ref latex, _)) = self.math_latex {
            latex.clone()
        } else {
            self.text.clone()
        }
    }

    pub(super) fn render_as_md(&self) -> String {
        if let Some((ref latex, is_display)) = self.math_latex {
            if latex.is_empty() {
                return String::new();
            }
            return if is_display {
                format!("$${}$$", latex)
            } else {
                format!("${}$", latex)
            };
        }

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
        if self.formatting.strikethrough {
            result = format!("~~{}~~", result);
        }

        result
    }
}

#[derive(Debug, Clone)]
pub(super) struct TextElement {
    pub(super) runs: Vec<Run>,
    /// Whether this text element comes from a title placeholder shape.
    pub(super) is_title: bool,
}

#[derive(Debug, Clone)]
pub(super) struct ListItem {
    pub(super) level: u32,
    pub(super) is_ordered: bool,
    pub(super) runs: Vec<Run>,
    /// Whether this paragraph has an explicit bullet marker (`buAutoNum` or `buChar`).
    /// When false, the paragraph is a plain text preamble within a list shape.
    pub(super) has_bullet: bool,
}

#[derive(Debug, Clone)]
pub(super) struct ListElement {
    pub(super) items: Vec<ListItem>,
}

#[derive(Debug, Clone)]
pub(super) struct TableCell {
    pub(super) runs: Vec<Run>,
}

#[derive(Debug, Clone)]
pub(super) struct TableRow {
    pub(super) cells: Vec<TableCell>,
}

#[derive(Debug, Clone)]
pub(super) struct TableElement {
    pub(super) rows: Vec<TableRow>,
}

#[derive(Debug, Clone)]
pub(super) struct ImageReference {
    pub(super) id: String,
    pub(super) target: String,
    /// Alt text / description from shape `descr` attribute.
    pub(super) description: Option<String>,
}

/// A hyperlink relationship resolved from a slide rels file.
#[derive(Debug, Clone)]
pub(super) struct HyperlinkReference {
    pub(super) id: String,
    pub(super) url: String,
}

/// A `<c:chart>` graphic frame reference (`p:graphicFrame` with a chart
/// `graphicData` payload). The chart part itself lives in a separate ZIP
/// entry (e.g. `ppt/charts/chart1.xml`) resolved via `rel_id` against the
/// slide's relationships.
#[derive(Debug, Clone)]
pub(super) struct ChartReference {
    pub(super) rel_id: String,
    /// Text recovered from the chart part (title, category and series
    /// labels, data point values). `None` until resolved, or if resolution
    /// failed or produced no text.
    pub(super) resolved_text: Option<String>,
}

/// A `<dgm:relIds>` SmartArt/diagram graphic frame reference. The diagram
/// data model lives in a separate ZIP entry (e.g. `ppt/diagrams/data1.xml`)
/// resolved via `rel_id` (the `r:dm` relationship) against the slide's
/// relationships.
#[derive(Debug, Clone)]
pub(super) struct DiagramReference {
    pub(super) rel_id: String,
    /// Text recovered from the diagram data part (one line per node).
    /// `None` until resolved, or if resolution failed or produced no text.
    pub(super) resolved_text: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) enum SlideElement {
    Text(TextElement, ElementPosition),
    Table(TableElement, ElementPosition),
    Image(ImageReference, ElementPosition),
    List(ListElement, ElementPosition),
    Chart(ChartReference, ElementPosition),
    SmartArt(DiagramReference, ElementPosition),
    Unknown,
}

impl SlideElement {
    pub(super) fn position(&self) -> ElementPosition {
        match self {
            SlideElement::Text(_, pos)
            | SlideElement::Table(_, pos)
            | SlideElement::Image(_, pos)
            | SlideElement::List(_, pos)
            | SlideElement::Chart(_, pos)
            | SlideElement::SmartArt(_, pos) => *pos,
            SlideElement::Unknown => ElementPosition::default(),
        }
    }
}

#[derive(Debug)]
pub(super) struct Slide {
    pub(super) slide_number: u32,
    pub(super) elements: Vec<SlideElement>,
    pub(super) images: Vec<ImageReference>,
    /// Hyperlink relationships resolved from the slide rels file.
    pub(super) hyperlinks: Vec<HyperlinkReference>,
    /// All relationship IDs from the slide rels file mapped to their target,
    /// regardless of relationship type. Used to resolve chart/SmartArt
    /// `graphicData` references, which are not images or hyperlinks.
    pub(super) rel_targets: AHashMap<String, String>,
}

#[derive(Debug, Clone)]
pub(super) struct ParserConfig {
    pub(super) extract_images: bool,
    pub(super) include_slide_comment: bool,
    pub(super) plain: bool,
    /// When `false`, `![alt](target)` image references are omitted from the
    /// markdown output even though the slide element is present. Mirrors
    /// `ImageExtractionConfig::inject_placeholders`. Default: `true`.
    pub(super) inject_placeholders: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            extract_images: true,
            include_slide_comment: false,
            plain: false,
            inject_placeholders: true,
        }
    }
}

pub(super) enum ParsedContent {
    Text(TextElement),
    List(ListElement),
}

impl Slide {
    pub(super) fn from_xml(slide_number: u32, xml_data: &[u8], rels_data: Option<&[u8]>) -> Result<Self> {
        let elements = parser::parse_slide_xml(xml_data)?;

        let (images, hyperlinks, rel_targets) = if let Some(rels) = rels_data {
            let slide_rels = parser::parse_slide_rels(rels)?;
            (slide_rels.images, slide_rels.hyperlinks, slide_rels.targets)
        } else {
            (Vec::new(), Vec::new(), AHashMap::new())
        };

        Ok(Self {
            slide_number,
            elements,
            images,
            hyperlinks,
            rel_targets,
        })
    }

    fn render_text_markdown(builder: &mut ContentBuilder, text: &TextElement, config: &ParserConfig) {
        let text_content: String = if config.plain {
            join_runs_with_spacing(&text.runs, Run::extract)
        } else {
            join_runs_with_spacing(&text.runs, Run::render_as_md)
        };
        builder.add_text(&text_content);
    }

    fn render_table_markdown(builder: &mut ContentBuilder, table: &TableElement, config: &ParserConfig) {
        let extract_fn: fn(&Run) -> String = if config.plain { Run::extract } else { Run::render_as_md };
        let table_rows: Vec<Vec<String>> = table
            .rows
            .iter()
            .map(|row| {
                row.cells
                    .iter()
                    .map(|cell| join_runs_with_spacing(&cell.runs, extract_fn))
                    .collect()
            })
            .collect();
        builder.add_table(&table_rows);
    }

    fn render_list_markdown(builder: &mut ContentBuilder, list: &ListElement, config: &ParserConfig) {
        let extract_fn: fn(&Run) -> String = if config.plain { Run::extract } else { Run::render_as_md };
        for item in &list.items {
            let item_text = join_runs_with_spacing(&item.runs, extract_fn);
            if item.has_bullet {
                builder.add_list_item(item.level, item.is_ordered, &item_text);
            } else {
                builder.add_text(&item_text);
            }
        }
    }

    fn render_image_markdown(
        builder: &mut ContentBuilder,
        img_ref: &ImageReference,
        images: &[ImageReference],
        config: &ParserConfig,
    ) {
        if !config.inject_placeholders {
            return;
        }
        let target = images
            .iter()
            .find(|rel| rel.id == img_ref.id)
            .map(|rel| rel.target.as_str())
            .unwrap_or("");
        builder.add_image_with_desc(&img_ref.id, img_ref.description.as_deref(), target);
    }

    fn internal_image_element(&self, img_ref: &ImageReference, image_index: Option<u32>) -> PptxInternalSlideElement {
        let target = self
            .images
            .iter()
            .find(|relationship| relationship.id == img_ref.id)
            .map(|relationship| relationship.target.clone())
            .unwrap_or_default();
        let alt_text = img_ref
            .description
            .as_deref()
            .map(|description| description.replace('\n', " ").replace('\r', ""))
            .unwrap_or_default();
        PptxInternalSlideElement::Image {
            alt_text: alt_text.trim().to_string(),
            target,
            image_index,
        }
    }

    /// Find the element index `to_markdown` should render as the slide's
    /// title: the first explicitly-marked title with non-empty text, or
    /// (failing that) the first short (<100 char) text element, matching the
    /// same two-pass search `build_slide_structure` does for its heading.
    fn find_markdown_title_index(elements: &[SlideElement], element_indices: &[usize]) -> Option<usize> {
        element_indices
            .iter()
            .find_map(|&idx| {
                if let SlideElement::Text(text, _) = &elements[idx]
                    && text.is_title
                {
                    let plain = join_runs_with_spacing(&text.runs, Run::extract);
                    if !plain.trim().is_empty() {
                        return Some(idx);
                    }
                }
                None
            })
            .or_else(|| {
                element_indices.iter().find_map(|&idx| {
                    if let SlideElement::Text(text, _) = &elements[idx] {
                        let plain = join_runs_with_spacing(&text.runs, Run::extract);
                        let normalized = plain.replace('\n', " ");
                        if normalized.len() < 100 && !normalized.trim().is_empty() {
                            return Some(idx);
                        }
                    }
                    None
                })
            })
    }

    pub(super) fn to_markdown(&self, config: &ParserConfig) -> String {
        let mut builder = ContentBuilder::new(config.plain);

        if config.include_slide_comment {
            builder.add_slide_header(self.slide_number);
        }

        let mut element_indices: Vec<usize> = (0..self.elements.len()).collect();
        element_indices.sort_by_key(|&i| {
            let pos = self.elements[i].position();
            (pos.y, pos.x)
        });

        let title_idx = Self::find_markdown_title_index(&self.elements, &element_indices);

        if let Some(tidx) = title_idx
            && let SlideElement::Text(text, _) = &self.elements[tidx]
        {
            let text_content: String = if config.plain {
                join_runs_with_spacing(&text.runs, Run::extract)
            } else {
                join_runs_with_spacing(&text.runs, Run::render_as_md)
            };
            let normalized = text_content.replace('\n', " ");
            builder.add_title(normalized.trim());
        }

        for &idx in &element_indices {
            if Some(idx) == title_idx {
                continue;
            }

            match &self.elements[idx] {
                SlideElement::Text(text, _) => {
                    Self::render_text_markdown(&mut builder, text, config);
                }
                SlideElement::Table(table, _) => {
                    Self::render_table_markdown(&mut builder, table, config);
                }
                SlideElement::List(list, _) => {
                    Self::render_list_markdown(&mut builder, list, config);
                }
                SlideElement::Image(img_ref, _) => {
                    Self::render_image_markdown(&mut builder, img_ref, &self.images, config);
                }
                SlideElement::Chart(chart_ref, _) => {
                    if let Some(text) = chart_ref.resolved_text.as_deref() {
                        builder.add_text(text);
                    }
                }
                SlideElement::SmartArt(diagram_ref, _) => {
                    if let Some(text) = diagram_ref.resolved_text.as_deref() {
                        builder.add_text(text);
                    }
                }
                SlideElement::Unknown => {}
            }
        }

        builder.build().0
    }

    pub(super) fn to_internal_elements(
        &self,
        config: &ParserConfig,
        image_indices: &[Option<u32>],
    ) -> Vec<PptxInternalSlideElement> {
        let mut element_indices: Vec<usize> = (0..self.elements.len()).collect();
        element_indices.sort_by_key(|&i| {
            let pos = self.elements[i].position();
            (pos.y, pos.x)
        });

        let title_idx = Self::find_markdown_title_index(&self.elements, &element_indices);
        let mut internal_elements = Vec::with_capacity(element_indices.len());

        if let Some(tidx) = title_idx
            && let SlideElement::Text(text, _) = &self.elements[tidx]
        {
            let text_content = if config.plain {
                join_runs_with_spacing(&text.runs, Run::extract)
            } else {
                join_runs_with_spacing(&text.runs, Run::render_as_md)
            };
            let mut builder = ContentBuilder::new(config.plain);
            builder.add_title(text_content.replace('\n', " ").trim());
            internal_elements.push(PptxInternalSlideElement::Markdown(builder.build().0));
        }

        let mut image_ordinal = 0_usize;
        for &idx in &element_indices {
            if Some(idx) == title_idx {
                continue;
            }

            if let SlideElement::Image(img_ref, _) = &self.elements[idx] {
                let image_index = image_indices.get(image_ordinal).copied().flatten();
                image_ordinal += 1;
                if config.plain || !config.inject_placeholders {
                    if let Some(image_index) = image_index {
                        internal_elements.push(PptxInternalSlideElement::ImageOcrTextAnchor { image_index });
                    }
                    continue;
                }
                internal_elements.push(self.internal_image_element(img_ref, image_index));
                continue;
            }

            let mut builder = ContentBuilder::new(config.plain);
            match &self.elements[idx] {
                SlideElement::Text(text, _) => Self::render_text_markdown(&mut builder, text, config),
                SlideElement::Table(table, _) => Self::render_table_markdown(&mut builder, table, config),
                SlideElement::List(list, _) => Self::render_list_markdown(&mut builder, list, config),
                SlideElement::Chart(chart_ref, _) => {
                    if let Some(text) = chart_ref.resolved_text.as_deref() {
                        builder.add_text(text);
                    }
                }
                SlideElement::SmartArt(diagram_ref, _) => {
                    if let Some(text) = diagram_ref.resolved_text.as_deref() {
                        builder.add_text(text);
                    }
                }
                SlideElement::Image(_, _) | SlideElement::Unknown => {}
            }
            let markdown = builder.build().0;
            if !markdown.is_empty() {
                internal_elements.push(PptxInternalSlideElement::Markdown(markdown));
            }
        }

        internal_elements
    }

    pub(super) fn image_count(&self) -> usize {
        self.elements
            .iter()
            .filter(|e| matches!(e, SlideElement::Image(_, _)))
            .count()
    }

    pub(super) fn table_count(&self) -> usize {
        self.elements
            .iter()
            .filter(|e| matches!(e, SlideElement::Table(_, _)))
            .count()
    }
}
