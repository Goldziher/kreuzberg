//! Per-page `PageContent` / `OcrElement` derivation, split out of `derive` by area.

use std::sync::Arc;

use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::ocr_elements::{OcrConfidence, OcrElement};
use crate::types::page::PageContent;
use crate::types::tables::Table;

pub(super) fn derive_pages(
    doc: &mut InternalDocument,
    content: &str,
    output_format: &crate::core::config::OutputFormat,
    content_is_plain: bool,
) -> Option<Vec<PageContent>> {
    let raw_pages = doc.prebuilt_pages.take().or_else(|| build_pages(doc, content_is_plain));
    let pages = apply_page_content_format(raw_pages, doc, output_format);
    if content_is_plain
        && !reconcile_page_boundaries(&mut doc.metadata, content, pages.as_deref())
        && drop_unusable_page_boundaries(&mut doc.metadata, content)
    {
        crate::core::diagnostics::push_warning(
            &mut doc.processing_warnings,
            "page_boundaries",
            "Page byte ranges were measured on a different rendering of the text and could not be \
             mapped onto the returned content, so they were omitted",
        );
    }
    pages
}

/// Re-point `metadata.pages.boundaries` at `content` by locating each page's text in it, in
/// order. Returns `true` only when every page was located exactly and the boundaries were
/// replaced; otherwise the boundaries are left as they were. A metadata with no boundaries
/// returns `false`.
pub(crate) fn reconcile_page_boundaries(
    metadata: &mut crate::types::Metadata,
    content: &str,
    pages: Option<&[PageContent]>,
) -> bool {
    let Some(boundaries) = metadata
        .pages
        .as_mut()
        .and_then(|structure| structure.boundaries.as_mut())
    else {
        return false;
    };
    let Some(pages) = pages.filter(|pages| pages.len() == boundaries.len()) else {
        return false;
    };
    let mut reconciled = Vec::with_capacity(boundaries.len());
    let mut search_offset = 0;
    for (boundary, page) in boundaries.iter().zip(pages) {
        if boundary.page_number != page.page_number || page.content.is_empty() {
            return false;
        }
        let Some(relative_start) = content[search_offset..].find(&page.content) else {
            return false;
        };
        let byte_start = search_offset + relative_start;
        let byte_end = byte_start + page.content.len();
        reconciled.push(crate::types::PageBoundary {
            byte_start,
            byte_end,
            page_number: page.page_number,
        });
        search_offset = byte_end;
    }
    // ~keep OCR boundaries can describe an intermediate rendering with different separators (#2128).
    // Replace only a complete exact match; unmatched pages retain their original extraction metadata.
    *boundaries = reconciled;
    true
}

/// Remove `metadata.pages.boundaries` when they cannot index `content`.
///
/// The offsets were measured on another rendering of the document, so a range that falls
/// outside `content` or inside a UTF-8 sequence is provably stale. Returns `true` when
/// the boundaries were dropped.
pub(crate) fn drop_unusable_page_boundaries(metadata: &mut crate::types::Metadata, content: &str) -> bool {
    let Some(structure) = metadata.pages.as_mut() else {
        return false;
    };
    let usable = structure.boundaries.as_ref().is_none_or(|boundaries| {
        boundaries
            .iter()
            .all(|b| b.byte_start <= b.byte_end && content.get(b.byte_start..b.byte_end).is_some())
    });
    if usable {
        return false;
    }
    structure.boundaries = None;
    true
}

/// Build per-page `PageContent` from page-grouped elements.
///
/// `content_is_plain` says that `result.content` is the plain rendering of the document:
/// plain output, or a custom format name with no renderer, which falls back to plain. Then
/// a page has the OCR text of a picture where `render_plain` writes it, so `pages[n].content`
/// and `result.content` agree. For every other output a page has the text of its elements
/// only. ~keep
pub(super) fn build_pages(doc: &InternalDocument, content_is_plain: bool) -> Option<Vec<PageContent>> {
    let mut page_map: std::collections::BTreeMap<u32, Vec<&InternalElement>> = std::collections::BTreeMap::new();

    for elem in &doc.elements {
        if let Some(page) = elem.page {
            page_map.entry(page).or_default().push(elem);
        }
    }

    if page_map.is_empty() {
        return None;
    }

    let arc_tables: Vec<Arc<Table>> = doc.tables.iter().map(|t| Arc::new(t.clone())).collect();

    let pages: Vec<PageContent> = page_map
        .into_iter()
        .map(|(page_num, elems)| {
            let mut content = String::new();
            let mut tables = Vec::new();
            let mut image_indices = Vec::new();
            for elem in &elems {
                // `render_plain` drops everything outside the body layer, so page content
                // must drop it too — otherwise running headers and footers appear in
                // `pages[n].content` but not in `result.content` for `OutputFormat::Plain`. ~keep
                if !crate::rendering::common::is_body_element(elem) {
                    continue;
                }
                if elem.kind.is_container_start() || elem.kind.is_container_end() {
                    continue;
                }
                let mut picture_ocr_text = None;
                match elem.kind {
                    ElementKind::Table { table_index } => {
                        if let Some(arc_table) = arc_tables.get(table_index as usize) {
                            tables.push(Arc::clone(arc_table));
                        }
                    }
                    ElementKind::Image { image_index } if (image_index as usize) < doc.images.len() => {
                        image_indices.push(image_index);
                        // The OCR text of a picture goes after the picture, as `render_plain`
                        // writes it. An image element of a PDF document can hold that text as
                        // its own text already. ~keep
                        if content_is_plain {
                            picture_ocr_text = crate::rendering::common::image_ocr_text(doc, elem, image_index)
                                .filter(|ocr_text| *ocr_text != elem.text);
                        }
                    }
                    _ => {}
                }
                let own_text = Some(elem.text.as_str()).filter(|text| !text.is_empty());
                for block in [own_text, picture_ocr_text].into_iter().flatten() {
                    if !content.is_empty() {
                        content.push_str("\n\n");
                    }
                    content.push_str(block);
                }
            }

            PageContent {
                page_number: page_num,
                content,
                tables,
                image_indices,
                image_preprocessing: None,
                hierarchy: None,
                is_blank: None,
                layout_regions: None,
                speaker_notes: None,
                section_name: None,
                sheet_name: None,
                ocr_confidence: None,
                native_content: None,
            }
        })
        .collect();

    Some(pages)
}

/// Re-render each page's content using the requested output format.
///
/// Called after pages are built but before `derive_document_structure_inner` moves
/// element text out of the document. For Plain/Json/Custom formats this
/// is a no-op. For Markdown/Djot/Html/DocTags, each page's element subset is
/// rendered with the same renderer used for the full document, so `pages[n].content`
/// matches the format of `result.content` after `apply_output_format`.
///
/// Pages whose `page_number` has no matching page-tagged elements (e.g., natively
/// extracted PDF pages where individual elements are not page-tracked) are returned
/// unchanged — their original content is preserved.
pub(super) fn apply_page_content_format(
    pages: Option<Vec<PageContent>>,
    doc: &InternalDocument,
    output_format: &crate::core::config::OutputFormat,
) -> Option<Vec<PageContent>> {
    use crate::core::config::OutputFormat;

    let renderer: fn(&InternalDocument) -> String = match output_format {
        OutputFormat::Markdown => crate::rendering::render_markdown,
        OutputFormat::Djot => crate::rendering::render_djot,
        OutputFormat::Html => crate::rendering::render_html,
        OutputFormat::DocTags => crate::rendering::render_doctags,
        OutputFormat::Plain | OutputFormat::Json | OutputFormat::Custom(_) => {
            return pages;
        }
    };

    let pages = pages?;
    let elements_by_page = compute_page_membership(doc);
    if elements_by_page.is_empty() {
        return Some(pages);
    }

    let pages = pages
        .into_iter()
        .map(|page| match elements_by_page.get(&page.page_number) {
            Some(elem_indices) => render_sub_page(page, elem_indices, doc, renderer),
            None => page,
        })
        .collect();

    Some(pages)
}

/// Which element indices belong to each page, keyed by page number.
///
/// Container markers (`ListStart`/`ListEnd`, quotes, groups) are never page-tagged:
/// `InternalDocumentBuilder::push_list`/`end_list` pass `page: None` even when every element
/// they wrap is tagged. Filtering strictly on `elem.page.is_some()` therefore dropped them
/// from a page's subset, and `build_comrak_ast` then saw each `ListItem` with no open list
/// parent and wrapped it in a fresh single-item list -- rendering a nested list as flat,
/// blank-line-separated bullets in `pages[N].content` (GH#1503 made this reachable by tagging
/// every element with a page). A start inherits the page of the next tagged element it opens
/// before; an end inherits the page of the last tagged element it closes after. ~keep
fn compute_page_membership(doc: &InternalDocument) -> std::collections::BTreeMap<u32, Vec<usize>> {
    let mut next_tagged_page: Vec<Option<u32>> = vec![None; doc.elements.len()];
    let mut seen: Option<u32> = None;
    for (idx, elem) in doc.elements.iter().enumerate().rev() {
        if elem.page.is_some() {
            seen = elem.page;
        }
        next_tagged_page[idx] = seen;
    }
    let mut prev_tagged_page: Vec<Option<u32>> = vec![None; doc.elements.len()];
    seen = None;
    for (idx, elem) in doc.elements.iter().enumerate() {
        prev_tagged_page[idx] = seen;
        if elem.page.is_some() {
            seen = elem.page;
        }
    }

    let mut elements_by_page: std::collections::BTreeMap<u32, Vec<usize>> = std::collections::BTreeMap::new();
    for (idx, elem) in doc.elements.iter().enumerate() {
        let page_num = elem.page.or_else(|| {
            if elem.kind.is_container_start() {
                next_tagged_page[idx]
            } else if elem.kind.is_container_end() {
                prev_tagged_page[idx]
            } else {
                None
            }
        });
        if let Some(page_num) = page_num {
            elements_by_page.entry(page_num).or_default().push(idx);
        }
    }
    elements_by_page
}

/// Re-render one page's `content` from just the elements (and the tables/images they
/// reference, renumbered) that belong to it.
fn render_sub_page(
    mut page: PageContent,
    elem_indices: &[usize],
    doc: &InternalDocument,
    renderer: fn(&InternalDocument) -> String,
) -> PageContent {
    let mut table_remap: ahash::AHashMap<u32, u32> = ahash::AHashMap::new();
    let mut sub_tables: Vec<Table> = Vec::new();
    let mut image_remap: ahash::AHashMap<u32, u32> = ahash::AHashMap::new();
    let mut sub_images: Vec<crate::types::ExtractedImage> = Vec::new();
    for &i in elem_indices {
        match doc.elements[i].kind {
            ElementKind::Table { table_index } if !table_remap.contains_key(&table_index) => {
                let new_idx = sub_tables.len() as u32;
                table_remap.insert(table_index, new_idx);
                if let Some(t) = doc.tables.get(table_index as usize) {
                    sub_tables.push(t.clone());
                }
            }
            ElementKind::Image { image_index } if !image_remap.contains_key(&image_index) => {
                let new_idx = sub_images.len() as u32;
                image_remap.insert(image_index, new_idx);
                if let Some(img) = doc.images.get(image_index as usize) {
                    sub_images.push(img.clone());
                }
            }
            _ => {}
        }
    }

    let elements: Vec<InternalElement> = elem_indices
        .iter()
        .map(|&i| {
            let mut elem = doc.elements[i].clone();
            match elem.kind {
                ElementKind::Table { ref mut table_index } => {
                    if let Some(&new_idx) = table_remap.get(table_index) {
                        *table_index = new_idx;
                    }
                }
                ElementKind::Image { ref mut image_index } => {
                    if let Some(&new_idx) = image_remap.get(image_index) {
                        *image_index = new_idx;
                    }
                }
                _ => {}
            }
            elem
        })
        .collect();

    let mut sub_doc = InternalDocument::new(&doc.source_format);
    sub_doc.elements = elements;
    sub_doc.tables = sub_tables;
    sub_doc.images = sub_images;
    // A page shows the OCR text of a picture the same way the document does. ~keep
    sub_doc.ocr_text_only = doc.ocr_text_only;
    sub_doc.append_ocr_text = doc.append_ocr_text;

    let rendered = renderer(&sub_doc);
    if !rendered.is_empty() {
        page.content = rendered;
    }
    page
}

/// Extract `OcrElement` entries from OCR-typed internal elements.
///
/// An element without geometry is kept with a zero bounding box rather than
/// discarded (#75): backends that report text without word boxes (VLM OCR, hOCR
/// without `bbox` properties) would otherwise lose their recognised text entirely.
pub(super) fn build_ocr_elements(doc: &InternalDocument) -> Option<Vec<OcrElement>> {
    let ocr_elems: Vec<OcrElement> = doc
        .elements
        .iter()
        .filter_map(|elem| {
            if let ElementKind::OcrText { level } = elem.kind {
                let geometry = elem.ocr_geometry.clone().unwrap_or_default();
                let confidence = elem.ocr_confidence.clone().unwrap_or(OcrConfidence {
                    detection: None,
                    recognition: 0.0,
                });
                Some(OcrElement {
                    text: elem.text.clone(),
                    geometry,
                    confidence,
                    level,
                    rotation: elem.ocr_rotation.clone(),
                    page_number: elem.page.unwrap_or(1),
                    parent_id: None,
                    backend_metadata: std::collections::HashMap::new(),
                })
            } else {
                None
            }
        })
        .collect();

    if ocr_elems.is_empty() { None } else { Some(ocr_elems) }
}
