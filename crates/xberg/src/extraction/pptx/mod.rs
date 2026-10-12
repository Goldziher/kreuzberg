//! PowerPoint presentation extraction functions.
//!
//! This module provides PowerPoint (PPTX) file parsing by directly reading the Office Open XML
//! format. It extracts text content, slide structure, images, and presentation metadata.
//!
//! # Attribution
//!
//! This code is based on the [pptx-to-md](https://github.com/nilskruthoff/pptx-parser) library
//! by Nils Kruthoff, licensed under MIT OR Apache-2.0. The original code has been vendored and
//! adapted to integrate with Xberg's architecture. See ATTRIBUTIONS.md for full license text.
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
//! ```ignore
//! use xberg::extraction::pptx::{extract_pptx_from_path, PptxExtractionOptions};
//!
//! # fn example() -> xberg::Result<()> {
//! let result = extract_pptx_from_path("presentation.pptx", &PptxExtractionOptions::default())?;
//!
//! println!("Slide count: {}", result.slide_count);
//! println!("Image count: {}", result.image_count);
//! println!("Content:\n{}", result.content);
//! # Ok(())
//! # }
//! ```

mod comments;
mod container;
mod content_builder;
mod elements;
mod image_handling;
mod metadata;
mod parser;

use ahash::AHashMap;
use bytes::Bytes;

use crate::core::diagnostics::push_warning;
use crate::error::Result;
use crate::types::builder::{self, DocumentStructureBuilder};
use crate::types::document_structure::TextAnnotation;
use crate::types::extraction::BoundingBox;
use crate::types::{ExtractedImage, PptxExtractionResult, ProcessingWarning};

use container::{PptxContainer, SlideIterator};
use content_builder::ContentBuilder;
use elements::{ParserConfig, Run, SlideElement};
use image_handling::detect_image_format;
use metadata::{extract_all_notes, extract_metadata, extract_section_names};

/// Options for PPTX content extraction.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub struct PptxExtractionOptions {
    /// Whether to extract embedded images.
    pub extract_images: bool,
    /// Optional page configuration for boundary tracking.
    pub page_config: Option<crate::core::config::PageConfig>,
    /// Whether to output plain text (no markdown).
    pub plain: bool,
    /// Whether to build the `DocumentStructure` tree.
    pub include_structure: bool,
    /// Whether to emit `![alt](target)` references in markdown output.
    pub inject_placeholders: bool,
    /// Security limits applied at container open: entry count
    /// (`max_files_in_archive`), aggregate uncompressed size (`max_archive_size`),
    /// and compression ratio (`max_compression_ratio`), from
    /// `ExtractionConfig.security_limits`. Defaults to `SecurityLimits::default()`
    /// when unset.
    pub security_limits: crate::extractors::security::SecurityLimits,
    /// Maximum number of slides the presentation may contain, from
    /// `ExtractionConfig.security_limits.max_pages` (#1451). `None` (the
    /// default) means unlimited.
    pub max_pages: Option<usize>,
}

/// Crate-internal PPTX extraction output.
///
/// `slide_contents` keeps archive-derived slide numbers and structural image
/// identity alongside rendered non-image content. The public result remains
/// unchanged while the internal extractor avoids parsing user-authored text as
/// image provenance. ~keep
pub(crate) struct PptxInternalExtraction {
    pub(crate) result: PptxExtractionResult,
    pub(crate) slide_contents: Vec<PptxInternalSlide>,
    /// `(latex, is_display)` for every math run, in slide order. The text
    /// flattens a math run into its LaTeX, which cannot be told apart from
    /// author text that holds the same characters.
    pub(crate) formulas: Vec<(String, bool)>,
    /// Whether the text is plain. Plain text holds the bare LaTeX of a math
    /// run; markdown text holds `$$latex$$` or `$latex$`.
    pub(crate) plain_output: bool,
}

pub(crate) struct PptxInternalSlide {
    pub(crate) slide_number: u32,
    pub(crate) elements: Vec<PptxInternalSlideElement>,
}

pub(crate) enum PptxInternalSlideElement {
    Markdown(String),
    Image {
        alt_text: String,
        target: String,
        image_index: Option<u32>,
    },
    /// A picture that gets no placeholder. Its position keeps the place for the OCR text
    /// of the picture `image_index`; with no element there, no later step can show it. ~keep
    ImageOcrTextAnchor {
        image_index: u32,
    },
}

impl Default for PptxExtractionOptions {
    fn default() -> Self {
        Self {
            extract_images: true,
            page_config: None,
            plain: false,
            include_structure: false,
            inject_placeholders: true,
            security_limits: crate::extractors::security::SecurityLimits::default(),
            max_pages: crate::extractors::security::SecurityLimits::default().max_pages,
        }
    }
}

/// Reject a presentation whose slide count exceeds `options.max_pages` before any
/// per-slide work (text rendering, chart/diagram resolution, image lookup) begins
/// (#1451).
///
/// Unlike PDF, where the page count needs a fallback parser because some
/// documents defeat the primary one (see `extractors::pdf::enforce_page_limit`),
/// a PPTX's slide count has no such failure mode here: `PptxContainer::open`/
/// `from_bytes` already resolve `slide_paths` (via
/// `ppt/_rels/presentation.xml.rels`, falling back to scanning
/// `ppt/slides/slideN.xml` names) before this function ever runs, so the count is
/// exact by the time it is checked.
fn enforce_slide_limit(slide_count: usize, max_pages: Option<usize>) -> Result<()> {
    Ok(crate::extractors::security::enforce_page_count(slide_count, max_pages)?)
}

/// Join text runs with smart spacing: inserts a space between adjacent runs
/// only when the previous run doesn't end with whitespace and the next run
/// doesn't start with whitespace.
fn join_runs_with_spacing(runs: &[Run], extract: impl Fn(&Run) -> String) -> String {
    let mut result = String::new();
    for run in runs {
        let text = extract(run);
        if !result.is_empty() && !text.is_empty() {
            let ends_ws = result.ends_with(|c: char| c.is_whitespace());
            let starts_ws = text.starts_with(|c: char| c.is_whitespace());
            if !ends_ws && !starts_ws {
                result.push(' ');
            }
        }
        result.push_str(&text);
    }
    result
}

/// Extract PPTX content from a file path.
///
/// # Arguments
///
/// * `path` - Path to the PPTX file
/// * `options` - Extraction options controlling image extraction, formatting, etc.
///
/// # Returns
///
/// The public extraction result plus archive-derived per-slide content used
/// internally to assign trustworthy page metadata.
pub(crate) fn extract_pptx_from_path_with_slide_contents(
    path: &str,
    options: &PptxExtractionOptions,
    warnings: &mut Vec<ProcessingWarning>,
) -> Result<PptxInternalExtraction> {
    let container = PptxContainer::open(path, &options.security_limits)?;
    extract_pptx_from_container(container, options, warnings)
}

/// Extract PPTX content from a byte buffer.
///
/// # Arguments
///
/// * `data` - Raw PPTX file bytes
/// * `options` - Extraction options controlling image extraction, formatting, etc.
///
/// # Returns
///
/// A `PptxExtractionResult` containing extracted content, metadata, and images.
#[cfg(test)]
pub(crate) fn extract_pptx_from_bytes(
    data: &[u8],
    options: &PptxExtractionOptions,
    warnings: &mut Vec<ProcessingWarning>,
) -> Result<PptxExtractionResult> {
    Ok(extract_pptx_from_bytes_with_slide_contents(data, options, warnings)?.result)
}

pub(crate) fn extract_pptx_from_bytes_with_slide_contents(
    data: &[u8],
    options: &PptxExtractionOptions,
    warnings: &mut Vec<ProcessingWarning>,
) -> Result<PptxInternalExtraction> {
    let container = PptxContainer::from_bytes(data, &options.security_limits)?;
    extract_pptx_from_container(container, options, warnings)
}

fn ordered_slide_images(slide: &elements::Slide) -> Vec<(&elements::ImageReference, &elements::ElementPosition)> {
    let mut images: Vec<_> = slide
        .elements
        .iter()
        .filter_map(|element| match element {
            SlideElement::Image(image, position) => Some((image, position)),
            _ => None,
        })
        .collect();
    images.sort_by_key(|(_, position)| (position.y, position.x));
    images
}

/// Read one slide's referenced images (by relationship ID, not iteration
/// position -- see #91) and append each as an `ExtractedImage`. A reference
/// the image map has no bytes for is skipped with a `ProcessingWarning`,
/// rather than aborting the slide. ~keep
fn extract_slide_images<R: std::io::Read + std::io::Seek>(
    slide: &elements::Slide,
    iterator: &mut SlideIterator<R>,
    extracted_images: &mut Vec<ExtractedImage>,
    warnings: &mut Vec<ProcessingWarning>,
) -> Vec<Option<u32>> {
    let Ok(image_data) = iterator.get_slide_images(slide) else {
        return Vec::new();
    };

    let image_elements = ordered_slide_images(slide);
    let mut image_indices = Vec::with_capacity(image_elements.len());

    // Keep image indices in Slide::to_markdown's stable visual order. ~keep
    for (img_ref, pos) in image_elements {
        let Some(data) = image_data.get(&img_ref.id) else {
            image_indices.push(None);
            push_warning(
                warnings,
                "pptx",
                format!(
                    "Image '{}' referenced on slide {} could not be read; it was not extracted",
                    img_ref.id, slide.slide_number
                ),
            );
            continue;
        };

        let format = detect_image_format(data);
        let image_index = extracted_images.len();
        image_indices.push(Some(image_index as u32));

        let width = if pos.cx > 0 { Some((pos.cx / 9525) as u32) } else { None };
        let height = if pos.cy > 0 { Some((pos.cy / 9525) as u32) } else { None };
        let description = img_ref.description.clone();
        let bbox = position_to_bbox(pos);

        let (image_kind, kind_confidence) =
            crate::extraction::image_kind::classify(crate::extraction::image_kind::ImageClassifyInput {
                bytes: data,
                format: format.as_ref(),
                width,
                height,
                colorspace: None,
                bits_per_component: None,
                is_mask: false,
            });

        extracted_images.push(ExtractedImage {
            data: Bytes::from(data.clone()),
            format,
            image_index: image_index as u32,
            page_number: Some(slide.slide_number),
            width,
            height,
            colorspace: None,
            bits_per_component: None,
            is_mask: false,
            description,
            ocr_result: None,
            bounding_box: bbox,
            source_path: None,
            image_kind: Some(image_kind),
            kind_confidence: Some(kind_confidence),
            cluster_id: None,
            caption: None,
            qr_codes: None,
            data_base64: None,
        });
    }

    image_indices
}

/// Mark each tracked page as non-blank when an extracted image landed on it,
/// then build the final `PageStructure` from the boundaries/page-contents
/// `ContentBuilder::build` produced (both `None` when page tracking was never
/// configured).
fn build_page_structure(
    slide_count: usize,
    boundaries: Option<Vec<crate::types::PageBoundary>>,
    mut page_contents: Option<Vec<crate::types::PageContent>>,
    extracted_images: &[ExtractedImage],
) -> (
    Option<Vec<crate::types::PageContent>>,
    Option<crate::types::PageStructure>,
) {
    if let Some(ref mut pcs) = page_contents {
        for pc in pcs.iter_mut() {
            if extracted_images
                .iter()
                .any(|img| img.page_number == Some(pc.page_number))
            {
                pc.is_blank = Some(false);
            }
        }
    }

    let page_structure = boundaries.as_ref().map(|bounds| crate::types::PageStructure {
        total_count: slide_count as u32,
        unit_type: crate::types::PageUnitType::Slide,
        boundaries: Some(bounds.clone()),
        pages: page_contents.as_ref().map(|pcs| {
            pcs.iter()
                .map(|pc| crate::types::PageInfo {
                    number: pc.page_number,
                    title: None,
                    dimensions: None,
                    image_count: None,
                    table_count: None,
                    hidden: None,
                    is_blank: pc.is_blank,
                    has_vector_graphics: false,
                })
                .collect()
        }),
    });

    (page_contents, page_structure)
}

/// Per-presentation accumulators threaded across every slide, plus the
/// read-only context (notes, section names, parser/page config) one slide's
/// processing needs. Bundled so [`process_one_slide`] stays under the
/// parameter-count limit; this type has no callers outside this module.
struct SlideProcessingContext<'a> {
    notes: &'a std::collections::HashMap<u32, String>,
    section_names: &'a std::collections::HashMap<u32, String>,
    config: &'a ParserConfig,
    page_config: Option<&'a crate::core::config::PageConfig>,
    plain: bool,
    content_builder: &'a mut ContentBuilder,
    doc_builder: &'a mut Option<DocumentStructureBuilder>,
    image_index_counter: &'a mut u32,
    slide_contents: &'a mut Vec<PptxInternalSlide>,
    collected_hyperlinks: &'a mut Vec<(String, Option<String>)>,
    collected_formulas: &'a mut Vec<(String, bool)>,
    extracted_images: &'a mut Vec<ExtractedImage>,
    total_image_count: &'a mut usize,
    total_table_count: &'a mut usize,
}

/// Render, record, and structure-build a single slide, exactly as
/// `extract_pptx_from_container`'s loop body did inline before this was
/// split out.
fn process_one_slide<R: std::io::Read + std::io::Seek>(
    slide: elements::Slide,
    iterator: &mut SlideIterator<R>,
    ctx: &mut SlideProcessingContext,
    warnings: &mut Vec<ProcessingWarning>,
) {
    let byte_start = if ctx.page_config.is_some() {
        ctx.content_builder.start_slide(slide.slide_number)
    } else {
        0
    };

    let slide_content = slide.to_markdown(ctx.config);
    ctx.content_builder.add_text(&slide_content);

    let slide_notes = ctx.notes.get(&slide.slide_number).cloned();
    if let Some(ref note_text) = slide_notes {
        ctx.content_builder.add_notes(note_text);
    }

    let image_indices = if ctx.config.extract_images {
        extract_slide_images(&slide, iterator, ctx.extracted_images, warnings)
    } else {
        Vec::new()
    };
    let mut internal_elements = slide.to_internal_elements(ctx.config, &image_indices);
    if let Some(ref note_text) = slide_notes
        && !note_text.trim().is_empty()
    {
        let mut note_content = String::new();
        if ctx.plain {
            note_content.push_str("Notes:\n");
        } else {
            note_content.push_str("### Notes:\n");
        }
        note_content.push_str(note_text);
        internal_elements.push(PptxInternalSlideElement::Markdown(note_content));
    }
    ctx.slide_contents.push(PptxInternalSlide {
        slide_number: slide.slide_number,
        elements: internal_elements,
    });

    let slide_section = ctx.section_names.get(&slide.slide_number).cloned();

    if ctx.page_config.is_some() {
        ctx.content_builder.end_slide(
            slide.slide_number,
            byte_start,
            slide_content.clone(),
            slide_notes,
            slide_section,
        );
    }

    if let Some(builder) = ctx.doc_builder.as_mut() {
        build_slide_structure(&slide, builder, ctx.image_index_counter);
    }

    collect_slide_hyperlinks(&slide, ctx.collected_hyperlinks);
    collect_slide_formulas(&slide, ctx.collected_formulas);

    *ctx.total_image_count += slide.image_count();
    *ctx.total_table_count += slide.table_count();
}

/// Parser config plus every non-slide part read from a PPTX container: core/app/custom
/// metadata, speaker notes (by slide path), section names (by slide path), and comments.
type PptxContainerMetadata = (
    ParserConfig,
    crate::types::metadata::PptxMetadata,
    std::collections::HashMap<String, String>,
    std::collections::HashMap<u32, String>,
    std::collections::HashMap<u32, String>,
    Option<Vec<crate::types::revisions::DocumentRevision>>,
);

/// Resolve the parser config and read every non-slide part of the container:
/// core/app/custom metadata, speaker notes, section names, and comments.
fn gather_pptx_container_metadata<R: std::io::Read + std::io::Seek>(
    container: &mut PptxContainer<R>,
    options: &PptxExtractionOptions,
    warnings: &mut Vec<ProcessingWarning>,
) -> Result<PptxContainerMetadata> {
    let config = ParserConfig {
        extract_images: options.extract_images,
        plain: options.plain,
        inject_placeholders: options.inject_placeholders,
        ..Default::default()
    };

    let (metadata, office_metadata) = extract_metadata(&mut container.archive, warnings);
    let notes = extract_all_notes(container, warnings)?;
    let section_names = extract_section_names(container)?;

    let slide_paths_for_comments = container.slide_paths().to_vec();
    let revisions = comments::extract_comments(container, &slide_paths_for_comments, warnings);

    Ok((config, metadata, office_metadata, notes, section_names, revisions))
}

fn extract_pptx_from_container<R: std::io::Read + std::io::Seek>(
    mut container: PptxContainer<R>,
    options: &PptxExtractionOptions,
    warnings: &mut Vec<ProcessingWarning>,
) -> Result<PptxInternalExtraction> {
    enforce_slide_limit(container.slide_paths().len(), options.max_pages)?;

    let page_config = options.page_config.as_ref();
    let include_structure = options.include_structure;

    let (config, metadata, office_metadata, notes, section_names, revisions) =
        gather_pptx_container_metadata(&mut container, options, warnings)?;

    let mut iterator = SlideIterator::new(container);
    let slide_count = iterator.slide_count();

    let estimated_capacity = slide_count.saturating_mul(1000).max(8192);
    let mut content_builder = ContentBuilder::with_page_config(estimated_capacity, page_config.cloned(), options.plain);

    let mut total_image_count = 0;
    let mut total_table_count = 0;
    let mut slide_contents = Vec::with_capacity(slide_count);
    let mut extracted_images = Vec::new();
    let mut collected_hyperlinks: Vec<(String, Option<String>)> = Vec::new();
    let mut collected_formulas: Vec<(String, bool)> = Vec::new();
    let mut doc_builder = if include_structure {
        Some(DocumentStructureBuilder::new().source_format("pptx"))
    } else {
        None
    };
    let mut image_index_counter: u32 = 0;

    let mut ctx = SlideProcessingContext {
        notes: &notes,
        section_names: &section_names,
        config: &config,
        page_config,
        plain: options.plain,
        content_builder: &mut content_builder,
        doc_builder: &mut doc_builder,
        image_index_counter: &mut image_index_counter,
        slide_contents: &mut slide_contents,
        collected_hyperlinks: &mut collected_hyperlinks,
        collected_formulas: &mut collected_formulas,
        extracted_images: &mut extracted_images,
        total_image_count: &mut total_image_count,
        total_table_count: &mut total_table_count,
    };
    while let Some(slide) = iterator.next_slide(warnings)? {
        process_one_slide(slide, &mut iterator, &mut ctx, warnings);
    }

    let (content, boundaries, page_contents) = content_builder.build();
    let (page_contents, page_structure) =
        build_page_structure(slide_count, boundaries, page_contents, &extracted_images);

    let document = doc_builder.map(|b| b.build()).filter(|d| !d.is_empty());

    Ok(PptxInternalExtraction {
        result: PptxExtractionResult {
            content,
            metadata,
            slide_count,
            image_count: total_image_count,
            table_count: total_table_count,
            images: extracted_images,
            page_structure,
            page_contents,
            document,
            hyperlinks: collected_hyperlinks.into_iter().map(Into::into).collect(),
            office_metadata,
            revisions,
        },
        slide_contents,
        formulas: collected_formulas,
        plain_output: options.plain,
    })
}

/// Build annotations from a sequence of text runs, tracking byte offsets.
///
/// Returns the concatenated plain text, the corresponding annotations, and the
/// LaTeX of the math runs. Math leaves the text so the caller can emit it as a
/// formula node, the shape DOCX uses for math runs inside a paragraph.
fn runs_to_text_and_annotations(runs: &[Run]) -> (String, Vec<TextAnnotation>, Vec<String>) {
    let mut text = String::new();
    let mut annotations = Vec::new();
    let formulas = runs_math(runs).map(|(latex, _)| latex.clone()).collect();

    for run in runs {
        if run.math_latex.is_some() {
            continue;
        }

        let run_text = run.extract();
        if run_text.is_empty() {
            continue;
        }

        if !text.is_empty() {
            let ends_ws = text.ends_with(|c: char| c.is_whitespace());
            let starts_ws = run_text.starts_with(|c: char| c.is_whitespace());
            if !ends_ws && !starts_ws {
                text.push(' ');
            }
        }

        let start = text.len() as u32;
        text.push_str(&run_text);
        let end = text.len() as u32;

        if run.formatting.bold {
            annotations.push(builder::bold(start, end));
        }
        if run.formatting.italic {
            annotations.push(builder::italic(start, end));
        }
        if run.formatting.underlined {
            annotations.push(builder::underline(start, end));
        }
        if run.formatting.strikethrough {
            annotations.push(builder::strikethrough(start, end));
        }
        if let Some(sz) = run.formatting.font_size {
            let pts = sz as f64 / 100.0;
            let value = if pts.fract() == 0.0 {
                format!("{}pt", pts as u32)
            } else {
                format!("{:.1}pt", pts)
            };
            annotations.push(builder::font_size(start, end, &value));
        }
    }

    (text, annotations, formulas)
}

/// Convert an `ElementPosition` with dimensions to a `BoundingBox`.
///
/// EMU coordinates are converted to points (1 inch = 914400 EMU = 72 pt).
fn position_to_bbox(pos: &elements::ElementPosition) -> Option<BoundingBox> {
    if pos.x == 0 && pos.y == 0 && pos.cx == 0 && pos.cy == 0 {
        return None;
    }
    const EMU_PER_PT: f64 = 914_400.0 / 72.0;
    Some(BoundingBox {
        x0: pos.x as f64 / EMU_PER_PT,
        y0: pos.y as f64 / EMU_PER_PT,
        x1: (pos.x + pos.cx) as f64 / EMU_PER_PT,
        y1: (pos.y + pos.cy) as f64 / EMU_PER_PT,
    })
}

/// Push a text element as either the slide's first heading (title-like text,
/// only the first one wins) or a paragraph, with its formula runs pushed
/// separately and its dominant language recorded as a paragraph attribute.
fn push_text_element(
    doc_builder: &mut DocumentStructureBuilder,
    text: &elements::TextElement,
    bbox: Option<BoundingBox>,
    first_title_seen: &mut bool,
) {
    let (plain_text, annotations, formulas) = runs_to_text_and_annotations(&text.runs);
    for formula in &formulas {
        doc_builder.push_formula(formula, None);
    }
    let normalized = plain_text.replace('\n', " ");
    let is_title_elem = text.is_title || (normalized.len() < 100 && !normalized.trim().is_empty());

    if is_title_elem && !*first_title_seen {
        *first_title_seen = true;
        doc_builder.push_heading(1, normalized.trim(), None, bbox);
    } else if !plain_text.trim().is_empty() {
        let node_idx = doc_builder.push_paragraph(&plain_text, annotations, None, bbox);

        if let Some(lang) = text.runs.iter().find_map(|r| {
            let l = &r.formatting.lang;
            if l.is_empty() { None } else { Some(l.clone()) }
        }) {
            let mut attrs = AHashMap::new();
            attrs.insert("lang".to_string(), lang);
            doc_builder.set_attributes(node_idx, attrs);
        }
    }
}

fn push_table_element(doc_builder: &mut DocumentStructureBuilder, table: &elements::TableElement) {
    let cells: Vec<Vec<String>> = table
        .rows
        .iter()
        .map(|row| {
            row.cells
                .iter()
                .map(|cell| join_runs_with_spacing(&cell.runs, Run::extract))
                .collect()
        })
        .collect();
    if !cells.is_empty() {
        doc_builder.push_table_from_cells(&cells, None);
    }
}

fn push_list_element(doc_builder: &mut DocumentStructureBuilder, list: &elements::ListElement) {
    if list.items.is_empty() {
        return;
    }
    let is_ordered = list.items.first().is_some_and(|item| item.is_ordered);
    let list_node = doc_builder.push_list(is_ordered, None);
    for item in &list.items {
        let (item_text, formulas) = runs_to_text_and_math(&item.runs);
        for formula in &formulas {
            doc_builder.push_formula(formula, None);
        }
        if !item_text.trim().is_empty() {
            doc_builder.push_list_item(list_node, item_text.trim(), Vec::new(), None);
        }
    }
}

fn push_image_element(
    doc_builder: &mut DocumentStructureBuilder,
    img_ref: &elements::ImageReference,
    bbox: Option<BoundingBox>,
    image_index_counter: &mut u32,
) {
    let desc = img_ref.description.as_deref().or({
        if img_ref.target.is_empty() {
            None
        } else {
            Some(img_ref.target.as_str())
        }
    });
    doc_builder.push_image(desc, Some(*image_index_counter), None, bbox);
    *image_index_counter += 1;
}

/// Populate the document structure builder for a single slide.
fn build_slide_structure(
    slide: &elements::Slide,
    doc_builder: &mut DocumentStructureBuilder,
    image_index_counter: &mut u32,
) {
    let mut sorted_indices: Vec<usize> = (0..slide.elements.len()).collect();
    sorted_indices.sort_by_key(|&i| {
        let pos = slide.elements[i].position();
        (pos.y, pos.x)
    });

    let slide_title = sorted_indices
        .iter()
        .find_map(|&idx| {
            if let SlideElement::Text(text, _) = &slide.elements[idx]
                && text.is_title
            {
                let plain = join_runs_with_spacing(&text.runs, Run::extract);
                if !plain.trim().is_empty() {
                    return Some(plain.trim().to_string());
                }
            }
            None
        })
        .or_else(|| {
            sorted_indices.iter().find_map(|&idx| {
                if let SlideElement::Text(text, _) = &slide.elements[idx] {
                    let plain = join_runs_with_spacing(&text.runs, Run::extract);
                    let normalized = plain.replace('\n', " ");
                    if normalized.len() < 100 && !normalized.trim().is_empty() {
                        return Some(normalized.trim().to_string());
                    }
                }
                None
            })
        });

    doc_builder.push_slide(slide.slide_number, slide_title.as_deref());

    let mut first_title_seen = false;

    for &idx in &sorted_indices {
        let elem = &slide.elements[idx];
        let pos = elem.position();
        let bbox = position_to_bbox(&pos);

        match elem {
            SlideElement::Text(text, _) => {
                push_text_element(doc_builder, text, bbox, &mut first_title_seen);
            }
            SlideElement::Table(table, _) => {
                push_table_element(doc_builder, table);
            }
            SlideElement::List(list, _) => {
                push_list_element(doc_builder, list);
            }
            SlideElement::Image(img_ref, _) => {
                push_image_element(doc_builder, img_ref, bbox, image_index_counter);
            }
            SlideElement::Chart(chart_ref, _) => {
                if let Some(text) = chart_ref.resolved_text.as_deref()
                    && !text.trim().is_empty()
                {
                    doc_builder.push_paragraph(text, vec![], None, bbox);
                }
            }
            SlideElement::SmartArt(diagram_ref, _) => {
                if let Some(text) = diagram_ref.resolved_text.as_deref()
                    && !text.trim().is_empty()
                {
                    doc_builder.push_paragraph(text, vec![], None, bbox);
                }
            }
            SlideElement::Unknown => {}
        }
    }

    doc_builder.exit_container();
}

/// Collect hyperlinks from all runs in a slide by resolving `hlinkClick` rIds
/// against the slide's hyperlink relationships.
fn collect_slide_hyperlinks(slide: &elements::Slide, out: &mut Vec<(String, Option<String>)>) {
    let mut visit_runs = |runs: &[Run]| {
        for run in runs {
            if let Some(ref hlink_id) = run.hyperlink_id
                && let Some(href) = slide.hyperlinks.iter().find(|h| h.id == *hlink_id)
            {
                let label = if run.text.trim().is_empty() {
                    None
                } else {
                    Some(run.text.trim().to_string())
                };
                out.push((href.url.clone(), label));
            }
        }
    };

    for elem in &slide.elements {
        match elem {
            SlideElement::Text(text, _) => visit_runs(&text.runs),
            SlideElement::List(list, _) => {
                for item in &list.items {
                    visit_runs(&item.runs);
                }
            }
            SlideElement::Table(table, _) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        visit_runs(&cell.runs);
                    }
                }
            }
            _ => {}
        }
    }
}

/// The LaTeX and display flag of every math run that carries content.
///
/// One place decides what counts as a math run, for both the text path and the
/// slide structure path.
fn runs_math(runs: &[Run]) -> impl Iterator<Item = &(String, bool)> {
    runs.iter()
        .filter_map(|run| run.math_latex.as_ref())
        .filter(|(latex, _)| !latex.is_empty())
}

/// Collect the LaTeX of every math run in a slide's text and list shapes.
///
/// Table cells are left out on purpose. Their math is inline content of a grid
/// cell: lifting it into a formula element would either empty the cell or
/// repeat the equation next to the table. The markdown extractor keeps inline
/// math inside a table cell for the same reason.
fn collect_slide_formulas(slide: &elements::Slide, out: &mut Vec<(String, bool)>) {
    let mut visit_runs = |runs: &[Run]| out.extend(runs_math(runs).cloned());

    for elem in &slide.elements {
        match elem {
            SlideElement::Text(text, _) => visit_runs(&text.runs),
            SlideElement::List(list, _) => {
                for item in &list.items {
                    visit_runs(&item.runs);
                }
            }
            _ => {}
        }
    }
}

/// Split a run sequence into its text and the LaTeX of its math runs.
fn runs_to_text_and_math(runs: &[Run]) -> (String, Vec<String>) {
    let text = join_runs_with_spacing(runs, |run| {
        if run.math_latex.is_some() {
            String::new()
        } else {
            run.extract()
        }
    });
    let formulas = runs_math(runs).map(|(latex, _)| latex.clone()).collect();
    (text, formulas)
}

#[cfg(test)]
pub(crate) mod tests;
