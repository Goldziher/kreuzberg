#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use super::pipeline::filter_public_ocr_elements;
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
use super::rendering::page_dimensions_pt;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use super::scoring::{MIN_OCR_NATIVE_ALNUM_RETENTION_RATIO, NativeTextStats, evaluate_native_text_for_ocr};
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use crate::core::config::ExtractionConfig;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use crate::core::config::OcrQualityThresholds;

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn normalize_mixed_ocr_document_page(doc: &mut crate::types::internal::InternalDocument, page_number: u32) {
    for element in &mut doc.elements {
        if !matches!(element.kind, crate::types::internal::ElementKind::PageBreak) {
            element.page = Some(page_number);
        }
    }
    for table in &mut doc.tables {
        table.page_number = page_number;
    }
    for image in &mut doc.images {
        image.page_number = Some(page_number);
    }
}
/// Height-axis points-per-pixel ratio for one page's raster, used to scale
/// `element.ocr_geometry`'s pixel-space quad-edge height into the font-size
/// resolver's PDF-points unit (see
/// [`crate::pdf::structure::adapters::OcrFontSizeScale`]). Falls back to a no-op
/// scale of `1.0` when there is no raster height to divide by, or the computed
/// ratio is not a finite positive number -- mirrors `ocr_points_per_pixel`'s same
/// guard, for the same reason: leave the pixel value unconverted rather than
/// fabricate a scale or divide by zero.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn mixed_route_geometry_points_per_pixel(page_height_pt: f32, image_height_px: u32) -> f32 {
    const NO_OP_POINTS_PER_PIXEL: f32 = 1.0;
    if image_height_px == 0 {
        return NO_OP_POINTS_PER_PIXEL;
    }
    let scale = page_height_pt / image_height_px as f32;
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        NO_OP_POINTS_PER_PIXEL
    }
}
/// Returns the assembled per-page document alongside the bare, unclassified paragraphs
/// used to build it -- the latter feeds [`extract_mixed_ocr_native`]'s document-global
/// heading/list heuristic pass, which needs every OCR'd page's paragraphs in hand at
/// once (see that function's own comments, and `extract_with_ocr_for_page`'s doc comment
/// on `skip_document_global_heuristic` for why a single page can't run this heuristic
/// itself).
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn assemble_mixed_ocr_page_document(
    mut doc: crate::types::internal::InternalDocument,
    preserve_structure: bool,
    page_number: u32,
    page_height: u32,
    geometry_points_per_pixel: f32,
    margins: crate::pdf::native::text::PageMarginFractions,
) -> (
    crate::types::internal::InternalDocument,
    Vec<crate::pdf::structure::types::PdfParagraph>,
    OcrMarginFilterOutcome,
) {
    if preserve_structure {
        let outcome = filter_structured_ocr_page_by_page_margins(&mut doc, page_height as f32, margins);
        flip_document_bboxes_to_bottom_left(&mut doc, f64::from(page_height));
        normalize_mixed_ocr_document_page(&mut doc, page_number);
        return (doc, Vec::new(), outcome);
    }
    // `doc.elements[].bbox` is already in PDF points by the time this runs (the caller
    // rescales it via `rescale_ocr_bboxes_to_page_points` before calling this), and
    // `page_height` is the page's own height in points, so the bbox-height font-size
    // fallback needs no further scaling. `doc.elements[].ocr_geometry`, in contrast, is
    // NOT touched by that rescale -- it stays raw OCR raster pixels (see
    // `extraction::derive::OcrElement::geometry`'s documented raster-pixel-space
    // contract) -- so the quad-edge fallback (sceptre/paddle) still needs the real
    // points-per-pixel ratio for this page, `geometry_points_per_pixel`. See
    // `pdf::structure::adapters::OcrFontSizeScale` for why these can't share one scalar.
    let font_size_scale =
        crate::pdf::structure::adapters::OcrFontSizeScale::bbox_already_in_points(geometry_points_per_pixel);
    let mut paragraphs = crate::pdf::structure::adapters::ocr_doc_to_paragraphs(&doc, page_height, font_size_scale);
    let outcome = filter_ocr_paragraphs_by_page_margins(&mut paragraphs, page_height as f32, margins);
    if !paragraphs.is_empty() {
        let mut assembled = crate::pdf::structure::assemble_internal_document(
            vec![paragraphs.clone()],
            &doc.tables,
            Some(&doc.images),
            &[],
            &Default::default(),
        );
        assembled.processing_warnings = std::mem::take(&mut doc.processing_warnings);
        doc = assembled;
    }

    normalize_mixed_ocr_document_page(&mut doc, page_number);
    (doc, paragraphs, outcome)
}
/// Flat OCR-text document for a page whose backend produced tables or OCR elements
/// but no structured document.
///
/// Mirrors the paragraph shape of the raw-text fallback in `append_ocr_replacements`
/// so the page reads identically, while giving its assets a document to travel in.
///
/// OCR page text is normalized to LF first: backend output is not uniformly LF-only.
/// Tesseract emits LF, but the VLM backend (`crate::llm::vlm_ocr`) returns the model's
/// markdown verbatim out of an HTTP JSON body, which routinely carries `\r\n`. Splitting
/// raw would fold the entire page into a single block element (#316).
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn flat_ocr_page_document(text: &str) -> crate::types::internal::InternalDocument {
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    use crate::types::ocr_elements::OcrElementLevel;

    let mut doc = InternalDocument::new("pdf");
    let text = crate::extraction::transform::normalize_line_endings(text);
    for paragraph in text
        .split("\n\n")
        .map(str::trim)
        .filter(|paragraph| !paragraph.is_empty())
    {
        doc.push_element(InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Block,
            },
            paragraph,
            0,
        ));
    }
    doc
}
/// Attach a page's OCR tables and OCR elements to its structured document.
///
/// The mixed route used to discard both (#60): only `ocr_internal_document` was kept,
/// so tables recognised on an OCR'd page and every word-level bounding box were lost.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn attach_page_ocr_payload(
    doc: &mut crate::types::internal::InternalDocument,
    tables: Vec<crate::types::Table>,
    elements: Vec<crate::types::OcrElement>,
    page_number: u32,
) {
    if doc.tables.is_empty() {
        doc.tables = tables;
    }
    if !elements.is_empty() {
        let mut elements = elements;
        for element in &mut elements {
            element.page_number = page_number;
        }
        doc.prebuilt_ocr_elements.get_or_insert_with(Vec::new).extend(elements);
    }
}
/// Copy the payload fields that a page document already carried before the document-global
/// restructuring heuristic (`heuristically_restructured_ocr_pages`, in `pipeline.rs`) rebuilt
/// it: the heuristic's combined document has no notion of the backend's raw per-word OCR
/// elements, this page's earlier warnings, or its OCR coordinate frame -- all three come from
/// the page document `build_mixed_ocr_page_document`/`build_pipeline_ocr_page_document` already
/// built. Missing the frame here silently drops it from every non-`Plain` mixed-route output
/// while every other unit test stays green, because the heuristic only runs when structured
/// output is requested (GH#1645).
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn carry_page_ocr_payload_forward(
    existing: &crate::types::internal::InternalDocument,
    new_page_doc: &mut crate::types::internal::InternalDocument,
) {
    new_page_doc.prebuilt_ocr_elements = existing.prebuilt_ocr_elements.clone();
    new_page_doc.processing_warnings = existing.processing_warnings.clone();
    new_page_doc.ocr_coordinate_frame = existing.ocr_coordinate_frame;
    if let Some(modes) = existing
        .metadata
        .additional
        .get(crate::ocr_metadata_keys::OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY)
    {
        new_page_doc.metadata.additional.insert(
            std::borrow::Cow::Borrowed(crate::ocr_metadata_keys::OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY),
            modes.clone(),
        );
    }
}
/// Rescale an OCR backend's pixel-space bounding boxes into the PDF page's own
/// coordinate space before its structured document is assembled (#1423).
///
/// On non-OCR pages, `document.nodes[].bbox`, `pages[].hierarchy.blocks[].bbox`, and
/// `chunks[].metadata.page_spans[].bbox` are all in PDF points with a bottom-left
/// origin. On OCR'd pages they previously stayed in raw Tesseract raster pixels
/// (top-left origin), with no field anywhere reporting the raster size needed to
/// convert them back.
///
/// `element` bboxes (word/line/block boxes from the OCR document) are only scaled
/// from pixels to points here, still top-left; `ocr_doc_to_paragraphs`
/// (`crate::pdf::structure::adapters::pdf_block_bbox`) performs the top-left ->
/// bottom-left flip further down the pipeline using the page height passed to
/// [`assemble_mixed_ocr_page_document`] — which must therefore be in points, not
/// raster pixels, from this point on.
///
/// `table` bounding boxes are copied through unchanged by every later step (no flip
/// is applied to them anywhere else in the pipeline), so this function performs the
/// full pixel-to-point conversion *and* the y-flip for those directly, matching the
/// bottom-left/points contract documented on [`crate::types::Table::bounding_box`].
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn rescale_ocr_bboxes_to_page_points(
    doc: Option<&mut crate::types::internal::InternalDocument>,
    tables: &mut [crate::types::Table],
    image_width_px: u32,
    image_height_px: u32,
    page_width_pt: f32,
    page_height_pt: f32,
) {
    if image_width_px == 0 || image_height_px == 0 {
        // No raster dimensions to convert from (e.g. a synthetic/test document with
        // no rendered page behind it) — leave bboxes as-is rather than dividing by
        // zero or fabricating a scale factor.
        return;
    }
    let scale_x = f64::from(page_width_pt) / f64::from(image_width_px);
    let scale_y = f64::from(page_height_pt) / f64::from(image_height_px);

    if let Some(doc) = doc {
        rescale_document_bboxes(doc, scale_x, scale_y);
    }

    let page_height_pt_f64 = f64::from(page_height_pt);
    for table in tables.iter_mut() {
        if let Some(bbox) = table.bounding_box.as_mut() {
            // `convert_ocr_table` (crates/xberg/src/ocr/tesseract_backend.rs) stores the
            // raw pixel rect verbatim as {x0: left, y0: top, x1: right, y1: bottom} —
            // top-left origin, unscaled pixels. Convert and flip in one step.
            let (left_px, top_px, right_px, bottom_px) = (bbox.x0, bbox.y0, bbox.x1, bbox.y1);
            bbox.x0 = left_px * scale_x;
            bbox.x1 = right_px * scale_x;
            bbox.y0 = page_height_pt_f64 - bottom_px * scale_y;
            bbox.y1 = page_height_pt_f64 - top_px * scale_y;
        }
    }
}
/// ~keep Coordinate contract of the consumer of a public OCR contribution.
/// Detached PDF stages use the same bottom-left raster frame as assembled hOCR;
/// their page owner subsequently scales that canonical frame to PDF points.
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
pub(super) enum PublicOcrOutputFrame {
    ImagePixels,
    PdfRaster { processed: (u32, u32), raster: (u32, u32) },
    PdfPage { raster: (u32, u32), points: (f32, f32) },
}

#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
pub(super) fn place_public_ocr_bboxes(doc: &mut crate::types::internal::InternalDocument, frame: PublicOcrOutputFrame) {
    let height = match frame {
        PublicOcrOutputFrame::ImagePixels => return,
        PublicOcrOutputFrame::PdfRaster { processed, raster } => {
            if processed.0 != 0 && processed.1 != 0 {
                rescale_document_bboxes(
                    doc,
                    f64::from(raster.0) / f64::from(processed.0),
                    f64::from(raster.1) / f64::from(processed.1),
                );
            }
            f64::from(raster.1)
        }
        PublicOcrOutputFrame::PdfPage { raster, points } => {
            rescale_ocr_bboxes_to_page_points(Some(doc), &mut [], raster.0, raster.1, points.0, points.1);
            f64::from(points.1)
        }
    };
    flip_document_bboxes_to_bottom_left(doc, height);
}

#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
fn flip_document_bboxes_to_bottom_left(doc: &mut crate::types::internal::InternalDocument, height: f64) {
    for element in &mut doc.elements {
        if let Some(bbox) = element.bbox.as_mut() {
            flip_bbox_to_bottom_left(bbox, height);
        }
    }
}

#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
fn rescale_document_bboxes(doc: &mut crate::types::internal::InternalDocument, scale_x: f64, scale_y: f64) {
    for element in &mut doc.elements {
        if let Some(bbox) = element.bbox.as_mut() {
            bbox.x0 *= scale_x;
            bbox.x1 *= scale_x;
            bbox.y0 *= scale_y;
            bbox.y1 *= scale_y;
        }
    }
}

#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
fn flip_bbox_to_bottom_left(bbox: &mut crate::types::BoundingBox, height: f64) {
    let top = bbox.y0;
    bbox.y0 = height - bbox.y1;
    bbox.y1 = height - top;
}

/// Undo a single quarter-turn on one point, mapping a backend's post-auto-rotate
/// pixel space back to the pre-rotation raster it was actually given.
///
/// `processed_width`/`processed_height` are the dimensions of the space `(x, y)`
/// is currently in (i.e. the auto-rotated image the backend ran detection on).
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn undo_auto_rotate_point(
    x: f64,
    y: f64,
    correction_degrees: u16,
    processed_width: f64,
    processed_height: f64,
) -> (f64, f64) {
    match correction_degrees {
        90 => (y, processed_width - x),
        180 => (processed_width - x, processed_height - y),
        270 => (processed_height - y, x),
        _ => (x, y),
    }
}
/// Undo an OCR backend's `auto_rotate` orientation correction on a structured
/// document's element bboxes, mapping them back from the rotated image the
/// backend actually OCR'd to the raster its caller rendered and will rescale
/// (#633).
///
/// Some backends (currently PaddleOCR, see
/// `paddle_ocr::backend::rotate_for_detected_orientation`) detect a scanned
/// page's orientation and rotate their input image before OCR when
/// `OcrConfig::auto_rotate` is set, recording that in the result metadata
/// (`ocr_metadata_keys::OCR_AUTO_ROTATED_METADATA_KEY` /
/// `OCR_ORIENTATION_DEGREES_METADATA_KEY` /
/// `OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY` / `..._HEIGHT_...`). Their
/// `ocr_internal_document` bboxes are built directly from that rotated raster
/// and are never mapped back — every other caller of this document assumes
/// bboxes are in the raster the backend actually read (which
/// `rescale_ocr_bboxes_to_page_points`'s callers resolve via
/// [`resolved_ocr_layout_dimensions`]), which after a 90/270 correction has
/// different — swapped — dimensions than the rotated one the bboxes are
/// actually in. Left uncorrected, the pixel->point rescale divides by the wrong
/// axis and both position and reading order come out wrong.
///
/// A no-op when the backend didn't auto-rotate (the metadata key is absent),
/// which covers every backend and the overwhelmingly common `auto_rotate: false`
/// default.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn undo_auto_rotate_document_bboxes(
    doc: &mut crate::types::internal::InternalDocument,
    metadata: &crate::types::Metadata,
    render_width: u32,
    render_height: u32,
) {
    let auto_rotated = metadata
        .additional
        .get(crate::ocr_metadata_keys::OCR_AUTO_ROTATED_METADATA_KEY)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !auto_rotated {
        return;
    }
    let Some(orientation) = metadata
        .additional
        .get(crate::ocr_metadata_keys::OCR_ORIENTATION_DEGREES_METADATA_KEY)
        .and_then(serde_json::Value::as_i64)
    else {
        return;
    };
    if !matches!(orientation, 0 | 90 | 180 | 270) {
        return;
    }
    let correction_degrees = ((360 - orientation).rem_euclid(360)) as u16;
    if correction_degrees == 0 {
        return;
    }
    // Prefer the backend's own reported processed-image size; fall back to the
    // swap a lossless quarter-turn of the original raster implies, in case a
    // future backend sets `auto_rotated` without the paired width/height keys.
    let reported_dimensions = metadata
        .additional
        .get(crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY)
        .and_then(serde_json::Value::as_u64)
        .zip(
            metadata
                .additional
                .get(crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY)
                .and_then(serde_json::Value::as_u64),
        );
    let (processed_width, processed_height) = match reported_dimensions {
        Some((width, height)) => (width as f64, height as f64),
        None if matches!(correction_degrees, 90 | 270) => (f64::from(render_height), f64::from(render_width)),
        None => (f64::from(render_width), f64::from(render_height)),
    };
    for element in &mut doc.elements {
        let Some(bbox) = element.bbox.as_mut() else {
            continue;
        };
        let (x0, y0) = undo_auto_rotate_point(bbox.x0, bbox.y0, correction_degrees, processed_width, processed_height);
        let (x1, y1) = undo_auto_rotate_point(bbox.x1, bbox.y1, correction_degrees, processed_width, processed_height);
        bbox.x0 = x0.min(x1);
        bbox.x1 = x0.max(x1);
        bbox.y0 = y0.min(y1);
        bbox.y1 = y0.max(y1);
    }
}

/// Build the per-page structured document for the single-backend mixed OCR route,
/// carrying the backend's tables and OCR elements instead of dropping them (#60).
///
/// Returns `None` only when the backend produced nothing structured at all, which
/// keeps the raw-text replacement path unchanged for plain-text pages.
///
/// `image_width_px`/`image_height_px` are the rendered page raster's pixel
/// dimensions and `page_width_pt`/`page_height_pt` are the PDF page's own MediaBox
/// size in points; together they let every OCR bbox be rescaled into the page's
/// coordinate space before assembly (#1423).
///
/// Returns the assembled document alongside the bare, unclassified paragraphs
/// [`assemble_mixed_ocr_page_document`] built it from -- see that function's doc
/// comment.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
#[expect(
    clippy::too_many_arguments,
    reason = "six of the eight are one page's geometry (pixel size, point size, margins); grouping \
              them into a struct is the better shape but would touch all seven call sites, so it is \
              deliberately left as follow-up rather than bundled into a lint fix"
)]
pub(super) fn build_mixed_ocr_page_document(
    result: &mut crate::types::ExtractedDocument,
    public_ocr_config: &crate::core::config::OcrConfig,
    page_number: u32,
    image_width_px: u32,
    image_height_px: u32,
    page_width_pt: f32,
    page_height_pt: f32,
    margins: crate::pdf::native::text::PageMarginFractions,
) -> Option<(
    crate::types::internal::InternalDocument,
    Vec<crate::pdf::structure::types::PdfParagraph>,
)> {
    let mut backend_tables = std::mem::take(&mut result.tables);
    let mut raw_backend_elements = result.ocr_elements.take().unwrap_or_default();
    let (element_layout_width, element_layout_height) =
        resolved_ocr_layout_dimensions(&result.metadata, image_width_px, image_height_px);
    let (backend_elements, element_margin_outcome) = public_ocr_elements_for_pdf_page(
        &mut raw_backend_elements,
        public_ocr_config,
        page_number,
        element_layout_height,
        margins,
    );
    let mut doc = match result.ocr_internal_document.take() {
        Some(doc) => doc,
        None if backend_tables.is_empty() && backend_elements.is_empty() => return None,
        None => flat_ocr_page_document(&result.content),
    };
    undo_auto_rotate_document_bboxes(&mut doc, &result.metadata, image_width_px, image_height_px);
    // ~keep The pixel frame is the backend's PROCESSED image, not the page render: preprocessing
    // resamples a sub-300 dpi render up to `target_dpi` before recognition, so the boxes come back
    // in the resampled frame and dividing by the render's size makes them too large by exactly that
    // ratio -- a table on a 150 dpi render landed at twice its size, off the page (GH#1895). These
    // are the same dimensions `public_ocr_elements_for_pdf_page` above already resolves for the
    // element boxes, which is why elements were right and tables were not.
    rescale_ocr_bboxes_to_page_points(
        Some(&mut doc),
        &mut backend_tables,
        element_layout_width,
        element_layout_height,
        page_width_pt,
        page_height_pt,
    );
    attach_page_ocr_payload(&mut doc, backend_tables, Vec::new(), page_number);
    // `assemble_mixed_ocr_page_document`/`ocr_doc_to_paragraphs` still take the page
    // nearest point loses at most ~0.5pt, negligible next to the pixel-vs-point unit
    // bug this rescale fixes.
    let page_height_rounded_pt = page_height_pt.max(0.0).round() as u32;
    let geometry_points_per_pixel = mixed_route_geometry_points_per_pixel(page_height_pt, image_height_px);
    let (mut assembled, paragraphs, outcome) = assemble_mixed_ocr_page_document(
        doc,
        result.document.is_some(),
        page_number,
        page_height_rounded_pt,
        geometry_points_per_pixel,
        margins,
    );
    if outcome.removed {
        result.content = if result.document.is_some() {
            crate::rendering::render_plain(&assembled)
        } else {
            ocr_paragraphs_plain_text(&paragraphs)
        };
    }
    if (outcome.missing_geometry
        || (result.document.is_none() && paragraphs.is_empty() && !result.content.trim().is_empty()))
        && (margins.top != 0.0 || margins.bottom != 0.0)
    {
        assembled
            .processing_warnings
            .push(ocr_margin_filter_capability_warning());
    }
    if element_margin_outcome.missing_geometry && !backend_elements.is_empty() {
        crate::core::diagnostics::push_warning_deduped(
            &mut assembled.processing_warnings,
            ocr_margin_filter_capability_warning(),
        );
    }
    attach_page_ocr_payload(&mut assembled, Vec::new(), backend_elements, page_number);
    // ~keep The frame is sourced from the same `resolved_ocr_layout_dimensions` call above
    // that decided where `backend_elements`' bboxes live, so the frame and the elements
    // cannot disagree by construction (GH#1645). Never fabricated: `resolved_ocr_layout_dimensions`
    // only falls back to the render raster when page-local processed metadata is absent or
    // invalid -- that fallback is still the raster the elements actually live in, not a
    // guess -- and a degenerate render raster (0x0) leaves both zero, so no frame is recorded.
    if element_layout_width != 0 && element_layout_height != 0 {
        assembled.ocr_coordinate_frame = Some(crate::types::internal::OcrPageCoordinateFrame::new(
            page_number,
            element_layout_width,
            element_layout_height,
        ));
    }
    Some((assembled, paragraphs))
}
/// Convert one OCR formula bbox to PDF points.
///
/// Backends can rescale the page image before OCR; when the result metadata
/// carries the processed dimensions, those describe the bbox's pixel space
/// and take precedence over the rendered dimensions.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn formula_bbox_to_page_points(
    formula: &mut crate::types::Formula,
    doc: &xberg_native_pdf::PdfDocument,
    page_idx: usize,
    metadata: Option<&crate::types::Metadata>,
    rendered_w: u32,
    rendered_h: u32,
) {
    if let Some(bbox) = formula.bbox {
        let (px_w, px_h) = metadata
            .and_then(processed_ocr_layout_dimensions)
            .unwrap_or((rendered_w, rendered_h));
        let (w_pt, h_pt) = crate::pdf::render::get_page_dimensions_pt(doc, page_idx);
        formula.bbox = Some(crate::pdf::render::pixel_bbox_to_pdf_points(
            bbox, px_w, px_h, w_pt, h_pt,
        ));
    }
}
/// Flip the bboxes of a document's table elements from a top-left to a bottom-left
/// origin, in points.
///
/// `crate::pdf::structure::assembly::push_table_element` copies `Table::bounding_box`
/// verbatim onto the table's element, so on the pipeline route that element inherits the
/// table's raw top-left pixel rect while every paragraph element around it was already
/// flipped (in pixel space) by `ocr_doc_to_paragraphs`. Once
/// [`rescale_ocr_bboxes_to_page_points`] has put both in points, only the table elements
/// still need the flip the single-backend route gives them before assembly.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn flip_table_element_bboxes_to_bottom_left(
    doc: &mut crate::types::internal::InternalDocument,
    page_height_pt: f32,
) {
    let page_height_pt = f64::from(page_height_pt);
    for element in &mut doc.elements {
        if matches!(element.kind, crate::types::internal::ElementKind::Table { .. })
            && let Some(bbox) = element.bbox.as_mut()
        {
            flip_bbox_to_bottom_left(bbox, page_height_pt);
        }
    }
}
/// Build the per-page structured document for the multi-stage pipeline / `vlm_fallback`
/// route, converting its pixel-space bboxes into the PDF page's point space (#1423).
///
/// The single-backend route's [`build_mixed_ocr_page_document`] cannot be reused as a
/// shared choke point: it takes the backend's *raw* OCR document and rescales it before
/// running assembly, whereas `run_ocr_pipeline` returns a document `extract_with_ocr` has
/// already assembled — its element bboxes carry the top-left -> bottom-left flip applied
/// with the *raster's* pixel height, so re-assembling it here would flip them a second
/// time. Only the pixel -> point scale is missing, which is exactly what
/// [`rescale_ocr_bboxes_to_page_points`] applies to document elements (tables, whose
/// bboxes are raw top-left pixel rects on this route too, still get the full
/// scale-and-flip).
///
/// `raster_size_px` is the rendered page image this route OCR'd; `page_size_pt` is the
/// page's own MediaBox size in points.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn build_pipeline_ocr_page_document(
    doc: Option<crate::types::internal::InternalDocument>,
    mut tables: Vec<crate::types::Table>,
    elements: Vec<crate::types::OcrElement>,
    page_text: &str,
    page_number: u32,
    raster_size_px: (u32, u32),
    page_size_pt: (f32, f32),
) -> Option<crate::types::internal::InternalDocument> {
    if doc.is_none() && tables.is_empty() && elements.is_empty() {
        return None;
    }
    let mut doc = doc.unwrap_or_else(|| flat_ocr_page_document(page_text));
    let (raster_width_px, raster_height_px) = raster_size_px;
    let (page_width_pt, page_height_pt) = page_size_pt;

    // Tables already folded into the assembled document are a separate allocation from
    // the `tables` returned alongside it, so each is converted exactly once.
    let mut assembled_tables = std::mem::take(&mut doc.tables);
    rescale_ocr_bboxes_to_page_points(
        Some(&mut doc),
        &mut assembled_tables,
        raster_width_px,
        raster_height_px,
        page_width_pt,
        page_height_pt,
    );
    if raster_width_px != 0 && raster_height_px != 0 {
        flip_table_element_bboxes_to_bottom_left(&mut doc, page_height_pt);
    }
    doc.tables = assembled_tables;
    rescale_ocr_bboxes_to_page_points(
        None,
        &mut tables,
        raster_width_px,
        raster_height_px,
        page_width_pt,
        page_height_pt,
    );

    attach_page_ocr_payload(&mut doc, tables, elements, page_number);
    // ~keep Unlike `build_mixed_ocr_page_document`, this route has no page-local processed-
    // image metadata to resolve against: `doc`'s OCR elements are in `raster_size_px` pixel
    // space at this point (see this function's doc comment), so the raster IS the frame
    // (GH#1645). Never fabricated: skipped entirely when the raster is degenerate (0x0).
    if raster_width_px != 0 && raster_height_px != 0 {
        doc.ocr_coordinate_frame = Some(crate::types::internal::OcrPageCoordinateFrame::new(
            page_number,
            raster_width_px,
            raster_height_px,
        ));
    }
    normalize_mixed_ocr_document_page(&mut doc, page_number);
    Some(doc)
}

/// Wraps a single OCR backend in a one-stage [`crate::core::config::OcrPipelineConfig`] so
/// [`extract_mixed_ocr_native`] can route it through [`run_ocr_pipeline_for_page`] --
/// the only per-page entry point that threads `layout_detections` down into
/// [`extract_with_ocr_for_page`]'s pixel-space layout classification -- instead of this
/// route's own raw `backend.process_image_owned` fast path, which never accepted layout
/// detections at all (#665: `--layout` alone produced byte-identical mixed-route output).
/// Mirrors the `classical_stage` construction in [`crate::core::config::ocr::OcrConfig::effective_pipeline`].
///
/// Only used when this call actually has layout detections to offer (see
/// `layout_detections_for_mixed` in [`extract_mixed_ocr_native`]); the plain single-backend
/// fast path is untouched when layout is off, so non-layout mixed-route output stays
/// byte-identical.
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "layout-detection"
))]
pub(super) fn single_stage_pipeline_for_layout(
    ocr_config: &crate::core::config::OcrConfig,
) -> crate::core::config::OcrPipelineConfig {
    crate::core::config::OcrPipelineConfig {
        stages: vec![crate::core::config::OcrPipelineStage {
            backend: ocr_config.backend.clone(),
            priority: 100,
            language: if ocr_config.language.len() == 1 && ocr_config.language[0] == "eng" {
                None
            } else {
                Some(ocr_config.language.clone())
            },
            tesseract_config: ocr_config.tesseract_config.clone(),
            paddle_ocr_config: None,
            paddle_ocr_settings: None,
            vlm_config: None,
            backend_options: ocr_config.backend_options.clone(),
        }],
        quality_thresholds: ocr_config.effective_thresholds(),
    }
}

/// Looks up one document-wide 0-based page's own layout detection out of the whole-document
/// pass [`extract_mixed_ocr_native`] runs (#665).
///
/// Performs no coordinate transform: `detections` is exactly what
/// `layout_runner::run_layout_for_ocr` produced (pixel space, at the resolution its own
/// per-page render used), and this function returns that same value unchanged for whichever
/// page it belongs to. The rescale to this page's *own* OCR raster
/// (`scale_detection_to_dimensions` / `scale_detection_to_ocr_coordinates`) and, later, to PDF
/// points (`rescale_ocr_bboxes_to_page_points` inside `build_pipeline_ocr_page_document`) both
/// happen downstream, inside `extract_with_ocr_for_page` -- not here. Keeping this lookup a
/// pure index (rather than folding a rescale into it) means a page-alignment bug here shows up
/// as a wrong page's detection landing on the wrong page, not as a subtly-wrong coordinate on
/// the right one.
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    feature = "pdf",
    feature = "layout-detection"
))]
pub(super) fn detection_for_mixed_route_page(
    detections: Option<&[crate::layout::DetectionResult]>,
    page_idx: usize,
) -> Option<&crate::layout::DetectionResult> {
    detections.and_then(|detections| detections.get(page_idx))
}
/// Merge per-page OCR text into the native text, replacing each OCR'd page's
/// byte range in place.
///
/// Boundaries are processed in reverse byte order so earlier offsets stay valid
/// after each replacement. An OCR entry that is empty (or whitespace-only) is
/// skipped rather than applied: an empty OCR result must never overwrite a page's
/// native text, or a page whose backend produced nothing would silently lose its
/// already-extracted content.
// Gated to `ocr` rather than `any(ocr, ocr-pipeline)` to match its only
// callers in the `#[cfg(all(test, feature = "ocr"))]` test module. ~keep
#[cfg(all(test, feature = "ocr"))]
pub(crate) fn merge_ocr_pages_into_native(
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
    ocr_results: &ahash::AHashMap<u32, String>,
) -> String {
    let accepted =
        accepted_ocr_page_replacements(native_text, boundaries, ocr_results, &OcrQualityThresholds::default());
    apply_ocr_page_replacements(native_text, boundaries, &accepted)
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn destructive_ocr_information_loss(
    native_page: &str,
    ocr_text: &str,
    thresholds: &OcrQualityThresholds,
) -> Option<(usize, usize)> {
    let native_decision = evaluate_native_text_for_ocr(native_page, Some(1), thresholds);
    if native_decision.fallback {
        return None;
    }

    let ocr_stats = NativeTextStats::compute(ocr_text, thresholds);
    let retained_ratio = ocr_stats.alnum as f64 / native_decision.stats.alnum.max(1) as f64;
    (retained_ratio < MIN_OCR_NATIVE_ALNUM_RETENTION_RATIO).then_some((native_decision.stats.alnum, ocr_stats.alnum))
}
/// Keep only OCR results that can be applied consistently to every mixed-output
/// representation: non-empty text with a matching, valid UTF-8 page boundary.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn accepted_ocr_page_replacements(
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
    ocr_results: &ahash::AHashMap<u32, String>,
    thresholds: &OcrQualityThresholds,
) -> ahash::AHashMap<u32, String> {
    let mut page_counts = std::collections::HashMap::new();
    for boundary in boundaries {
        *page_counts.entry(boundary.page_number).or_insert(0usize) += 1;
    }

    let mut valid_boundaries: Vec<&crate::types::PageBoundary> = boundaries
        .iter()
        .filter(|boundary| {
            page_counts.get(&boundary.page_number) == Some(&1)
                && boundary.page_number > 0
                && boundary.byte_start <= boundary.byte_end
                && boundary.byte_end <= native_text.len()
                && native_text.is_char_boundary(boundary.byte_start)
                && native_text.is_char_boundary(boundary.byte_end)
        })
        .collect();
    valid_boundaries.sort_unstable_by_key(|boundary| (boundary.byte_start, boundary.byte_end));

    let mut overlapping_pages = std::collections::HashSet::new();
    let mut active: Option<&crate::types::PageBoundary> = None;
    for boundary in &valid_boundaries {
        if let Some(previous) = active
            && boundary.byte_start < previous.byte_end
        {
            overlapping_pages.insert(previous.page_number);
            overlapping_pages.insert(boundary.page_number);
        }
        if active.is_none_or(|previous| boundary.byte_end > previous.byte_end) {
            active = Some(boundary);
        }
    }

    let valid_page_ranges: std::collections::HashMap<u32, (usize, usize)> = valid_boundaries
        .into_iter()
        .filter(|boundary| !overlapping_pages.contains(&boundary.page_number))
        .map(|boundary| (boundary.page_number, (boundary.byte_start, boundary.byte_end)))
        .collect();

    for (&page, text) in ocr_results {
        if !text.trim().is_empty() && !valid_page_ranges.contains_key(&page) {
            tracing::warn!(
                page,
                "rejecting mixed OCR page without one valid, non-overlapping text boundary"
            );
        }
    }

    // An accepted replacement OVERWRITES the page's native byte range, so a page whose
    // OCR came back effectively empty must not be accepted: doing so deletes whatever the
    // native text layer had and makes the OCR run return *less* than not running it at all.
    // `!text.trim().is_empty()` is too weak a bar -- a single stray character cleared it,
    // which is exactly what a blank/failed page render produces. Use the same blank
    // threshold the rest of the crate uses so one definition governs both. ~keep
    ocr_results
        .iter()
        .filter(|(page, text)| {
            let Some(&(byte_start, byte_end)) = valid_page_ranges.get(page) else {
                return false;
            };
            if crate::extraction::blank_detection::is_page_text_blank(text) {
                tracing::warn!(
                    page = **page,
                    chars = text.trim().chars().count(),
                    "rejecting mixed OCR page whose OCR output is blank; keeping native text for this page"
                );
                return false;
            }
            let native_page = &native_text[byte_start..byte_end];
            if let Some((native_alnum, ocr_alnum)) = destructive_ocr_information_loss(native_page, text, thresholds) {
                tracing::warn!(
                    page = **page,
                    native_alnum,
                    ocr_alnum,
                    "rejecting mixed OCR page that would discard most of a healthy native text layer"
                );
                return false;
            }
            true
        })
        .map(|(&page, text)| (page, text.clone()))
        .collect()
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn apply_ocr_page_replacements(
    native_text: &str,
    boundaries: &[crate::types::PageBoundary],
    accepted: &ahash::AHashMap<u32, String>,
) -> String {
    let mut result = native_text.to_string();

    let mut sorted_boundaries: Vec<&crate::types::PageBoundary> = boundaries
        .iter()
        .filter(|boundary| accepted.contains_key(&boundary.page_number))
        .collect();
    sorted_boundaries.sort_unstable_by_key(|boundary| std::cmp::Reverse((boundary.byte_start, boundary.page_number)));

    for boundary in sorted_boundaries {
        if let Some(ocr_text) = accepted.get(&boundary.page_number) {
            result.replace_range(boundary.byte_start..boundary.byte_end, ocr_text);
        }
    }

    result
}
/// Re-map page boundaries onto the text produced by `apply_ocr_page_replacements`.
///
/// Replacing a page's byte range with OCR text of a different length shifts every
/// later offset, so the input boundaries describe the NATIVE text and are wrong for
/// the merged result. Without this, anything downstream that maps a byte offset back
/// to a page number -- including page tagging on the flat-document path -- either
/// mis-attributes content or has to give up and emit no pages at all. Walks forward
/// accumulating the per-page delta; unreplaced pages simply shift by the running
/// total, so gaps between boundaries are preserved. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn boundaries_after_replacements(
    boundaries: &[crate::types::PageBoundary],
    accepted: &ahash::AHashMap<u32, String>,
) -> Vec<crate::types::PageBoundary> {
    let mut adjusted: Vec<crate::types::PageBoundary> = boundaries.to_vec();
    adjusted.sort_by_key(|boundary| boundary.byte_start);

    let mut delta: isize = 0;
    for boundary in &mut adjusted {
        let original_start = boundary.byte_start;
        let original_end = boundary.byte_end;
        boundary.byte_start = original_start.saturating_add_signed(delta);
        if let Some(ocr_text) = accepted.get(&boundary.page_number) {
            let old_len = original_end.saturating_sub(original_start);
            delta += ocr_text.len() as isize - old_len as isize;
        }
        boundary.byte_end = original_end.saturating_add_signed(delta);
    }

    adjusted
}
/// Replace native text-flow elements on OCR'd pages while preserving the
/// structured document's tables, images, and reading-order position.
///
/// PDF list markers do not carry page numbers, so page ownership is inferred
/// from balanced container spans before filtering. Page breaks are rebuilt
/// from the resulting page sequence, and relationships are remapped to the
/// final element indices (or dropped when either indexed endpoint was removed).
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn merge_ocr_pages_into_internal_document(
    doc: &mut crate::types::internal::InternalDocument,
    ocr_results: &ahash::AHashMap<u32, String>,
) {
    merge_structured_ocr_pages_into_internal_document(doc, ocr_results, &ahash::AHashMap::new());
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn merge_structured_ocr_pages_into_internal_document(
    doc: &mut crate::types::internal::InternalDocument,
    ocr_results: &ahash::AHashMap<u32, String>,
    structured_pages: &ahash::AHashMap<u32, crate::types::internal::InternalDocument>,
) {
    let replacements: std::collections::BTreeMap<u32, &str> = ocr_results
        .iter()
        .filter_map(|(&page, text)| (!text.trim().is_empty()).then_some((page, text.as_str())))
        .collect();
    if replacements.is_empty() {
        return;
    }

    let containers = analyze_container_markers(&doc.elements);
    let anchors = replacement_anchors(&doc.elements, &containers.inferred_pages, &replacements);
    // Assets carried by a per-page OCR document are re-indexed into the parent's
    // collections instead of being discarded. Discarding them used to force the
    // raw-text fallback in `append_ocr_replacements`, which dropped every table the
    // OCR'd page produced (#57) and destroyed the asset-to-page association (#59).
    let mut assets = MergedOcrAssets::new(doc.tables.len() as u32, doc.images.len() as u32);
    let planned = plan_merged_elements(
        &doc.elements,
        &containers,
        &replacements,
        structured_pages,
        &anchors,
        &mut assets,
    );
    let (rebuilt, old_to_new, planned_to_new) = rebuild_planned_elements(planned, doc.elements.len());
    remap_relationships(&mut doc.relationships, &old_to_new, &rebuilt);
    remap_relationships(&mut assets.relationships, &planned_to_new, &rebuilt);
    doc.relationships.extend(assets.relationships);
    doc.elements = rebuilt;
    doc.tables.extend(assets.tables);
    doc.images.extend(assets.images);
    if !assets.ocr_elements.is_empty() {
        doc.prebuilt_ocr_elements
            .get_or_insert_with(Vec::new)
            .extend(assets.ocr_elements);
    }
    let mut segmentation_modes = structured_pages
        .iter()
        .filter(|(page_number, _)| replacements.contains_key(page_number))
        .flat_map(|(_, page)| {
            page.metadata
                .additional
                .get(crate::ocr_metadata_keys::OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY)
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .cloned()
        })
        .collect::<Vec<_>>();
    segmentation_modes.sort_by_key(|entry| entry.get("page_number").and_then(serde_json::Value::as_u64));
    if !segmentation_modes.is_empty() {
        doc.metadata.additional.insert(
            std::borrow::Cow::Borrowed(crate::ocr_metadata_keys::OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY),
            serde_json::Value::Array(segmentation_modes),
        );
    }
}
/// Tables, images and OCR elements lifted out of per-page OCR documents and
/// re-indexed into the parent document's collections.
///
/// `table_base` / `image_base` are the parent's collection lengths before the
/// merge, so a page-local index `i` becomes `base + already_merged + i`.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) struct MergedOcrAssets {
    pub(super) table_base: u32,
    pub(super) image_base: u32,
    pub(super) tables: Vec<crate::types::Table>,
    pub(super) images: Vec<crate::types::ExtractedImage>,
    pub(super) ocr_elements: Vec<crate::types::OcrElement>,
    relationships: Vec<crate::types::internal::Relationship>,
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
impl MergedOcrAssets {
    fn new(table_base: u32, image_base: u32) -> Self {
        Self {
            table_base,
            image_base,
            tables: Vec::new(),
            images: Vec::new(),
            ocr_elements: Vec::new(),
            relationships: Vec::new(),
        }
    }

    fn next_table_index(&self) -> u32 {
        self.table_base + self.tables.len() as u32
    }

    fn next_image_index(&self) -> u32 {
        self.image_base + self.images.len() as u32
    }
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) struct PlannedOcrElement {
    pub(super) element: crate::types::internal::InternalElement,
    pub(super) old_index: Option<usize>,
    pub(super) page: Option<u32>,
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn replacement_anchors<'a>(
    elements: &[crate::types::internal::InternalElement],
    inferred_pages: &[Option<u32>],
    replacements: &std::collections::BTreeMap<u32, &'a str>,
) -> std::collections::BTreeMap<usize, Vec<(u32, &'a str)>> {
    let mut anchors = std::collections::BTreeMap::new();
    for (&page, &text) in replacements {
        let anchor = elements
            .iter()
            .enumerate()
            .find(|(index, element)| {
                inferred_pages[*index]
                    .or(element.page)
                    .is_some_and(|element_page| element_page >= page)
            })
            .map_or(elements.len(), |(index, _)| index);
        anchors.entry(anchor).or_insert_with(Vec::new).push((page, text));
    }
    anchors
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn plan_merged_elements(
    elements: &[crate::types::internal::InternalElement],
    containers: &ContainerMarkerAnalysis,
    replacements: &std::collections::BTreeMap<u32, &str>,
    structured_pages: &ahash::AHashMap<u32, crate::types::internal::InternalDocument>,
    anchors: &std::collections::BTreeMap<usize, Vec<(u32, &str)>>,
    assets: &mut MergedOcrAssets,
) -> Vec<PlannedOcrElement> {
    use crate::types::internal::ElementKind;

    let mut planned = Vec::with_capacity(elements.len() + replacements.len());
    for (old_index, element) in elements.iter().enumerate() {
        append_ocr_replacements(&mut planned, anchors.get(&old_index), structured_pages, assets);
        if containers.drop_marker[old_index] {
            continue;
        }
        if matches!(element.kind, ElementKind::PageBreak) {
            continue;
        }
        let page = element.page.or(containers.inferred_pages[old_index]);
        let preserve_asset = matches!(element.kind, ElementKind::Image { .. });
        if !preserve_asset && page.is_some_and(|page| replacements.contains_key(&page)) {
            continue;
        }
        let mut element = element.clone();
        if matches!(element.kind, ElementKind::Image { .. })
            && page.is_some_and(|page| replacements.contains_key(&page))
        {
            element.suppress_image_ocr_rendering();
        }
        planned.push(PlannedOcrElement {
            element,
            old_index: Some(old_index),
            page,
        });
    }
    append_ocr_replacements(&mut planned, anchors.get(&elements.len()), structured_pages, assets);
    planned
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn append_ocr_replacements(
    planned: &mut Vec<PlannedOcrElement>,
    replacements: Option<&Vec<(u32, &str)>>,
    structured_pages: &ahash::AHashMap<u32, crate::types::internal::InternalDocument>,
    assets: &mut MergedOcrAssets,
) {
    use crate::types::internal::{ElementKind, InternalElement};
    use crate::types::ocr_elements::OcrElementLevel;

    for &(page, text) in replacements.into_iter().flatten() {
        // Usability is decided before re-indexing so a rejected page never leaks its
        // tables/images into `assets`.
        let structured_page = structured_pages.get(&page).filter(|doc| {
            !doc.tables.is_empty()
                || !doc.images.is_empty()
                || doc
                    .elements
                    .iter()
                    .any(|element| !matches!(element.kind, ElementKind::PageBreak) && !element.text.trim().is_empty())
        });
        if let Some(structured_page) = structured_page {
            let (elements, local_to_new) = reindex_structured_ocr_page(structured_page, page, assets);
            {
                let mut relationships = structured_page.relationships.clone();
                remap_relationships(&mut relationships, &local_to_new, &elements);
                let offset = planned.len() as u32;
                assets
                    .relationships
                    .extend(relationships.into_iter().map(|mut relationship| {
                        relationship.source += offset;
                        if let crate::types::internal::RelationshipTarget::Index(target) = &mut relationship.target {
                            *target += offset;
                        }
                        relationship
                    }));
            }
            planned.extend(elements.into_iter().map(|element| PlannedOcrElement {
                element,
                old_index: None,
                page: Some(page),
            }));
            continue;
        }
        // Backend text verbatim (see `flat_ocr_page_document`): normalize before splitting.
        let text = crate::extraction::transform::normalize_line_endings(text);
        for paragraph in text.split("\n\n").map(str::trim).filter(|text| !text.is_empty()) {
            let element = InternalElement::text(
                ElementKind::OcrText {
                    level: OcrElementLevel::Block,
                },
                paragraph,
                0,
            )
            .with_page(page);
            planned.push(PlannedOcrElement {
                element,
                old_index: None,
                page: Some(page),
            });
        }
    }
}
/// Move an OCR'd page's tables, images and OCR elements into the parent document's
/// collections and rewrite the page's element references to the new parent indices.
///
/// Page-local `Table { table_index }` / `Image { image_index }` references are only
/// meaningful against the page document's own collections, so they must be rebased
/// before the elements are spliced into the parent (#59). Assets the page document
/// carries but never references from its element list still get a reference emitted,
/// so a table produced by OCR cannot silently vanish (#57).
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn reindex_structured_ocr_page(
    page_doc: &crate::types::internal::InternalDocument,
    page: u32,
    assets: &mut MergedOcrAssets,
) -> (Vec<crate::types::internal::InternalElement>, Vec<Option<u32>>) {
    use crate::types::internal::{ElementKind, InternalElement};

    let table_base = assets.next_table_index();
    let image_base = assets.next_image_index();

    for table in &page_doc.tables {
        let mut table = table.clone();
        table.page_number = page;
        assets.tables.push(table);
    }
    for (local_index, image) in page_doc.images.iter().enumerate() {
        let mut image = image.clone();
        image.page_number = Some(page);
        image.image_index = image_base + local_index as u32;
        assets.images.push(image);
    }
    if let Some(page_ocr_elements) = page_doc.prebuilt_ocr_elements.as_ref() {
        assets
            .ocr_elements
            .extend(page_ocr_elements.iter().cloned().map(|mut element| {
                element.page_number = page;
                element
            }));
    }

    let mut referenced_tables = vec![false; page_doc.tables.len()];
    let mut referenced_images = vec![false; page_doc.images.len()];
    let mut elements = Vec::with_capacity(page_doc.elements.len());
    let mut local_to_new = vec![None; page_doc.elements.len()];
    for (local_index, element) in page_doc.elements.iter().enumerate() {
        if matches!(element.kind, ElementKind::PageBreak) {
            continue;
        }
        let mut element = element.clone();
        match element.kind {
            ElementKind::Table { table_index } => {
                let Some(referenced) = referenced_tables.get_mut(table_index as usize) else {
                    // Dangling page-local reference: the table it points at does not exist.
                    continue;
                };
                *referenced = true;
                element.kind = ElementKind::Table {
                    table_index: table_base + table_index,
                };
            }
            ElementKind::Image { image_index } => {
                let Some(referenced) = referenced_images.get_mut(image_index as usize) else {
                    continue;
                };
                *referenced = true;
                element.kind = ElementKind::Image {
                    image_index: image_base + image_index,
                };
            }
            _ => {}
        }
        element.page = Some(page);
        local_to_new[local_index] = Some(elements.len() as u32);
        elements.push(element);
    }

    for (local_index, referenced) in referenced_tables.iter().enumerate() {
        if !*referenced {
            elements.push(
                InternalElement::text(
                    ElementKind::Table {
                        table_index: table_base + local_index as u32,
                    },
                    "",
                    0,
                )
                .with_page(page),
            );
        }
    }
    for (local_index, referenced) in referenced_images.iter().enumerate() {
        if !*referenced {
            elements.push(
                InternalElement::text(
                    ElementKind::Image {
                        image_index: image_base + local_index as u32,
                    },
                    "",
                    0,
                )
                .with_page(page),
            );
        }
    }

    (elements, local_to_new)
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn rebuild_planned_elements(
    planned: Vec<PlannedOcrElement>,
    old_len: usize,
) -> (
    Vec<crate::types::internal::InternalElement>,
    Vec<Option<u32>>,
    Vec<Option<u32>>,
) {
    use crate::types::internal::{ElementKind, InternalElement};

    let mut old_to_new = vec![None; old_len];
    let mut rebuilt = Vec::with_capacity(planned.len());
    let mut planned_to_new = Vec::with_capacity(planned.len());
    let mut previous_page = None;
    for planned_element in planned {
        if let (Some(previous), Some(current)) = (previous_page, planned_element.page)
            && previous != current
        {
            rebuilt.push(InternalElement::text(ElementKind::PageBreak, "", 0));
        }
        if let Some(page) = planned_element.page {
            previous_page = Some(page);
        }
        if let Some(old_index) = planned_element.old_index {
            old_to_new[old_index] = Some(rebuilt.len() as u32);
        }
        planned_to_new.push(Some(rebuilt.len() as u32));
        rebuilt.push(planned_element.element);
    }
    for (index, element) in rebuilt.iter_mut().enumerate() {
        *element = element.clone().with_index(index as u32);
    }
    (rebuilt, old_to_new, planned_to_new)
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn remap_relationships(
    relationships: &mut Vec<crate::types::internal::Relationship>,
    old_to_new: &[Option<u32>],
    rebuilt: &[crate::types::internal::InternalElement],
) {
    use crate::types::internal::RelationshipTarget;

    let retained_anchors: std::collections::HashSet<&str> =
        rebuilt.iter().filter_map(|element| element.anchor.as_deref()).collect();
    relationships.retain_mut(|relationship| {
        let Some(source) = old_to_new.get(relationship.source as usize).copied().flatten() else {
            return false;
        };
        relationship.source = source;
        match &mut relationship.target {
            RelationshipTarget::Index(target) => {
                let Some(remapped) = old_to_new.get(*target as usize).copied().flatten() else {
                    return false;
                };
                *target = remapped;
            }
            RelationshipTarget::Key(key) if !retained_anchors.contains(key.as_str()) => return false,
            RelationshipTarget::Key(_) => {}
        }
        true
    });
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) struct ContainerMarkerAnalysis {
    pub(super) inferred_pages: Vec<Option<u32>>,
    pub(super) drop_marker: Vec<bool>,
}
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn analyze_container_markers(
    elements: &[crate::types::internal::InternalElement],
) -> ContainerMarkerAnalysis {
    use crate::types::internal::ElementKind;

    fn matching_container(start: ElementKind, end: ElementKind) -> bool {
        matches!(
            (start, end),
            (ElementKind::ListStart { .. }, ElementKind::ListEnd)
                | (ElementKind::QuoteStart, ElementKind::QuoteEnd)
                | (ElementKind::GroupStart, ElementKind::GroupEnd)
        )
    }

    let mut analysis = ContainerMarkerAnalysis {
        inferred_pages: vec![None; elements.len()],
        drop_marker: vec![false; elements.len()],
    };
    let mut stack: Vec<(usize, ElementKind)> = Vec::new();
    for (index, element) in elements.iter().enumerate() {
        if element.kind.is_container_start() {
            stack.push((index, element.kind));
            continue;
        }
        if !element.kind.is_container_end() {
            continue;
        }
        let Some(&(start_index, start_kind)) = stack.last() else {
            analysis.drop_marker[index] = true;
            continue;
        };
        if !matching_container(start_kind, element.kind) {
            analysis.drop_marker[index] = true;
            continue;
        }
        stack.pop();
        let pages: std::collections::HashSet<u32> = elements[start_index..=index]
            .iter()
            .filter_map(|element| element.page)
            .collect();
        if pages.len() == 1 {
            let page = pages.iter().next().copied();
            analysis.inferred_pages[start_index] = page;
            analysis.inferred_pages[index] = page;
        } else {
            analysis.drop_marker[start_index] = true;
            analysis.drop_marker[index] = true;
        }
    }
    for (start_index, _) in stack {
        analysis.drop_marker[start_index] = true;
    }
    analysis
}

// The OCR metadata keys come from `crate::ocr_metadata_keys`, which is ungated, rather
// than from `crate::ocr`: this PDF OCR path also compiles under `ocr-pipeline` (VLM OCR,
// e.g. the `binstall` CLI) or under `layout-detection` alone (layout without any OCR
// backend enabled), where the `ocr` module — gated on `ocr`/`ocr-wasm` — is absent. ~keep
#[cfg(any(
    feature = "ocr",
    feature = "ocr-wasm",
    all(feature = "ocr-pipeline", feature = "pdf")
))]
use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};
// Same rationale, scoped to `layout-detection` only: `resolved_ocr_correction_degrees` and
// `transform_ocr_elements_to_render_space` (both `layout-detection`-only) are the sole
// readers of these two key names in this file.
#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
use crate::ocr_metadata_keys::{OCR_AUTO_ROTATED_METADATA_KEY, OCR_ORIENTATION_DEGREES_METADATA_KEY};

#[cfg(any(
    feature = "ocr",
    feature = "ocr-wasm",
    all(feature = "ocr-pipeline", feature = "pdf")
))]
pub(super) fn valid_ocr_layout_dimension(value: &serde_json::Value) -> Option<u32> {
    let value = value.as_f64()?;
    if !value.is_finite() || value <= 0.0 || value > u32::MAX as f64 || value.fract() != 0.0 {
        return None;
    }
    Some(value as u32)
}

#[cfg(any(
    feature = "ocr",
    feature = "ocr-wasm",
    all(feature = "ocr-pipeline", feature = "pdf")
))]
pub(super) fn processed_ocr_layout_dimensions(metadata: &crate::types::Metadata) -> Option<(u32, u32)> {
    let width = metadata
        .additional
        .get(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY)
        .and_then(valid_ocr_layout_dimension);
    let height = metadata
        .additional
        .get(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY)
        .and_then(valid_ocr_layout_dimension);

    match (width, height) {
        (Some(width), Some(height)) => Some((width, height)),
        _ => None,
    }
}

// Defined exactly where it is called, which is not a single feature set. Three
// call sites, none of which subsumes the others:
//   * `extract_with_ocr_for_page`'s `layout-detection` branch, which additionally
//     requires `any(ocr, ocr-wasm)`;
//   * that same function's `not(layout-detection)` branch, which rides only on its own
//     `any(ocr, ocr-pipeline)` gate;
//   * the element-geometry call sites -- `build_mixed_ocr_page_document` here and the
//     `#[cfg(feature = "pdf")]` public-element block in `extract_with_ocr_for_page` --
//     which need `pdf` and no OCR frontend at all.
// A plain `any(ocr, ocr-wasm)` left the `liter-llm` build (ocr-pipeline, no ocr, no
// layout-detection) calling a function that did not exist -- undetected since 2026-07-29
// because no CI leg built that combination until the xberg-cli feature legs began
// executing. Omitting the `pdf` arm broke `formula-recognition,pdf` (layout-detection +
// ocr-pipeline, no OCR backend) exactly the same way: the two element-geometry call sites
// stayed in while the definition compiled out. ~keep
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    any(
        feature = "ocr",
        feature = "ocr-wasm",
        feature = "pdf",
        not(feature = "layout-detection")
    )
))]
pub(super) fn resolved_ocr_layout_dimensions(
    metadata: &crate::types::Metadata,
    render_width: u32,
    render_height: u32,
) -> (u32, u32) {
    processed_ocr_layout_dimensions(metadata).unwrap_or((render_width, render_height))
}
#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
pub(super) fn scale_detection_to_dimensions(
    detection: &crate::layout::DetectionResult,
    target_width: u32,
    target_height: u32,
) -> crate::layout::DetectionResult {
    if detection.page_width == 0 || detection.page_height == 0 || target_width == 0 || target_height == 0 {
        return detection.clone();
    }

    let scale_x = target_width as f32 / detection.page_width as f32;
    let scale_y = target_height as f32 / detection.page_height as f32;
    let mut scaled = detection.clone();
    scaled.page_width = target_width;
    scaled.page_height = target_height;
    for region in &mut scaled.detections {
        region.bbox.x1 *= scale_x;
        region.bbox.y1 *= scale_y;
        region.bbox.x2 *= scale_x;
        region.bbox.y2 *= scale_y;
    }
    scaled
}
#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
pub(super) fn resolved_ocr_correction_degrees(metadata: &crate::types::Metadata) -> Option<u16> {
    if !metadata
        .additional
        .get(OCR_AUTO_ROTATED_METADATA_KEY)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }
    let orientation = metadata
        .additional
        .get(OCR_ORIENTATION_DEGREES_METADATA_KEY)
        .and_then(serde_json::Value::as_i64)?;
    if !matches!(orientation, 0 | 90 | 180 | 270) {
        return None;
    }
    Some(((360 - orientation) % 360) as u16)
}
#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
pub(super) fn rotate_detection(
    mut detection: crate::layout::DetectionResult,
    correction_degrees: u16,
) -> crate::layout::DetectionResult {
    let source_width = detection.page_width as f32;
    let source_height = detection.page_height as f32;
    for region in &mut detection.detections {
        let (x1, y1, x2, y2) = (region.bbox.x1, region.bbox.y1, region.bbox.x2, region.bbox.y2);
        match correction_degrees {
            90 => {
                region.bbox.x1 = source_height - y2;
                region.bbox.y1 = x1;
                region.bbox.x2 = source_height - y1;
                region.bbox.y2 = x2;
            }
            180 => {
                region.bbox.x1 = source_width - x2;
                region.bbox.y1 = source_height - y2;
                region.bbox.x2 = source_width - x1;
                region.bbox.y2 = source_height - y1;
            }
            270 => {
                region.bbox.x1 = y1;
                region.bbox.y1 = source_width - x2;
                region.bbox.x2 = y2;
                region.bbox.y2 = source_width - x1;
            }
            _ => {}
        }
    }
    if matches!(correction_degrees, 90 | 270) {
        std::mem::swap(&mut detection.page_width, &mut detection.page_height);
    }
    detection
}
#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
pub(super) fn scale_detection_to_ocr_coordinates(
    detection: &crate::layout::DetectionResult,
    metadata: &crate::types::Metadata,
    render_width: u32,
    render_height: u32,
) -> crate::layout::DetectionResult {
    let Some((final_width, final_height)) = processed_ocr_layout_dimensions(metadata) else {
        return scale_detection_to_dimensions(detection, render_width, render_height);
    };
    let Some(correction_degrees) = resolved_ocr_correction_degrees(metadata) else {
        return scale_detection_to_dimensions(detection, final_width, final_height);
    };
    let (pre_rotation_width, pre_rotation_height) = if matches!(correction_degrees, 90 | 270) {
        (final_height, final_width)
    } else {
        (final_width, final_height)
    };
    let scaled = scale_detection_to_dimensions(detection, pre_rotation_width, pre_rotation_height);
    rotate_detection(scaled, correction_degrees)
}
#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
pub(super) fn inverse_rotate_ocr_point(
    x: f64,
    y: f64,
    correction_degrees: u16,
    pre_rotation_width: f64,
    pre_rotation_height: f64,
) -> (f64, f64) {
    match correction_degrees {
        90 => (y, pre_rotation_height - x),
        180 => (pre_rotation_width - x, pre_rotation_height - y),
        270 => (pre_rotation_width - y, x),
        _ => (x, y),
    }
}
#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
pub(super) fn transform_ocr_point_to_render(
    point: (u32, u32),
    correction_degrees: u16,
    pre_rotation_dimensions: (u32, u32),
    render_dimensions: (u32, u32),
) -> (u32, u32) {
    let (pre_width, pre_height) = pre_rotation_dimensions;
    let (render_width, render_height) = render_dimensions;
    let (x, y) = inverse_rotate_ocr_point(
        point.0 as f64,
        point.1 as f64,
        correction_degrees,
        pre_width as f64,
        pre_height as f64,
    );
    let render_x = (x * render_width as f64 / pre_width as f64)
        .round()
        .clamp(0.0, render_width as f64) as u32;
    let render_y = (y * render_height as f64 / pre_height as f64)
        .round()
        .clamp(0.0, render_height as f64) as u32;
    (render_x, render_y)
}
#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
pub(super) fn transform_ocr_geometry_to_render(
    geometry: &crate::types::OcrBoundingGeometry,
    correction_degrees: u16,
    pre_rotation_dimensions: (u32, u32),
    render_dimensions: (u32, u32),
) -> crate::types::OcrBoundingGeometry {
    match geometry {
        crate::types::OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height,
        } => {
            let first = transform_ocr_point_to_render(
                (*left, *top),
                correction_degrees,
                pre_rotation_dimensions,
                render_dimensions,
            );
            let second = transform_ocr_point_to_render(
                (left.saturating_add(*width), top.saturating_add(*height)),
                correction_degrees,
                pre_rotation_dimensions,
                render_dimensions,
            );
            let left = first.0.min(second.0);
            let top = first.1.min(second.1);
            crate::types::OcrBoundingGeometry::Rectangle {
                left,
                top,
                width: first.0.max(second.0).saturating_sub(left),
                height: first.1.max(second.1).saturating_sub(top),
            }
        }
        crate::types::OcrBoundingGeometry::Quadrilateral { points } => {
            let points = points
                .iter()
                .copied()
                .map(|point| {
                    transform_ocr_point_to_render(
                        point.into(),
                        correction_degrees,
                        pre_rotation_dimensions,
                        render_dimensions,
                    )
                    .into()
                })
                .collect();
            crate::types::OcrBoundingGeometry::Quadrilateral { points }
        }
    }
}
/// Whether a processed-image pixel space and a render raster can be related by a scale at all.
///
/// ~keep A >1% divergence between the two axes' scale factors means the two rasters are not the
/// same page, so no per-axis scale maps one onto the other; the same bar the standalone-image
/// route applies in `extractors::image::whole_image_ocr_coordinate_transform`, and the tolerance
/// is that route's constant so the two cannot drift apart.
#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn ocr_render_scale_is_trustworthy(processed_dimensions: (u32, u32), render_dimensions: (u32, u32)) -> bool {
    let (processed_width, processed_height) = processed_dimensions;
    let (render_width, render_height) = render_dimensions;
    if processed_width == 0 || processed_height == 0 || render_width == 0 || render_height == 0 {
        return false;
    }
    let scale_x = f64::from(render_width) / f64::from(processed_width);
    let scale_y = f64::from(render_height) / f64::from(processed_height);
    (scale_x - scale_y).abs() / scale_x.max(scale_y)
        <= crate::extractors::image::MAX_OCR_COORDINATE_SCALE_RELATIVE_DIFFERENCE
}
/// Map OCR elements from the backend's processed-image pixel space into the render raster's, or
/// report that the two spaces cannot be related.
///
/// ~keep Returns `None` -- never untransformed elements -- when the backend reported no processed
/// dimensions, auto-rotated without a usable orientation, or reported a raster that is not a
/// scaled copy of the render raster. Handing back untransformed elements was silent and
/// destructive: Tesseract normalizes to `target_dpi` (300) against a ~150 dpi page render, so
/// every word box sat ~2x outside the render-space table bbox, `select_table_elements`'
/// `MIN_CELL_ELEMENT_IOW` test dropped all of them, and the region published a table holding none
/// of the page's values. PaddleOCR records its processed dims as the unresized raster, which is
/// why the same route was sound there (xberg-io/xberg#1813). The caller must skip TATR on `None`
/// rather than run it on elements in the wrong space.
#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
pub(super) fn transform_ocr_elements_to_render_space(
    elements: &[crate::types::OcrElement],
    metadata: &crate::types::Metadata,
    render_width: u32,
    render_height: u32,
) -> Option<Vec<crate::types::OcrElement>> {
    let (final_width, final_height) = processed_ocr_layout_dimensions(metadata)?;
    let auto_rotated = metadata
        .additional
        .get(OCR_AUTO_ROTATED_METADATA_KEY)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let correction_degrees = resolved_ocr_correction_degrees(metadata);
    if auto_rotated && correction_degrees.is_none() {
        return None;
    }
    let correction_degrees = correction_degrees.unwrap_or(0);
    let pre_rotation_dimensions = if matches!(correction_degrees, 90 | 270) {
        (final_height, final_width)
    } else {
        (final_width, final_height)
    };
    if !ocr_render_scale_is_trustworthy(pre_rotation_dimensions, (render_width, render_height)) {
        return None;
    }
    Some(
        elements
            .iter()
            .cloned()
            .map(|mut element| {
                element.geometry = transform_ocr_geometry_to_render(
                    &element.geometry,
                    correction_degrees,
                    pre_rotation_dimensions,
                    (render_width, render_height),
                );
                element
            })
            .collect(),
    )
}

/// Scale factor from OCR raster pixels to PDF points for one page, used to convert
/// pixel-derived font-size proxies into the same unit as the heading heuristic's
/// absolute-point constants (see `pdf::structure::adapters::resolve_ocr_font_size_pt`).
///
/// Requires the PDF document this OCR pass rendered from (`lazy_pdf_render_state`);
/// when that is unavailable — the caller supplied pre-rendered `images` directly, so
/// there is no `xberg_native_pdf::PdfDocument` in hand to read a MediaBox from — falls back
/// to `1.0` (pixels treated as points). That degrades the absolute-gap term of the
/// heading heuristic back toward today's behavior for that call path only; the
/// ratio-based term, which dominates in practice, is scale-invariant and unaffected.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
// Both readers are the OCR-document assembly blocks in `extract_with_ocr_for_page` -- one
// gated on `layout-detection` *with* `ocr`/`ocr-wasm`, the other on `not(layout-detection)`.
// `layout-detection` without either OCR frontend (the `formula-recognition,pdf` CI leg)
// compiles both out while `ocr-pipeline` still brings this function in. ~keep
#[cfg_attr(
    all(feature = "layout-detection", not(feature = "ocr"), not(feature = "ocr-wasm")),
    allow(dead_code)
)]
pub(super) fn ocr_points_per_pixel(
    #[cfg(feature = "pdf")] lazy_pdf_render_state: Option<&(xberg_native_pdf::PdfDocument, usize, Vec<u32>)>,
    page_idx: usize,
    page_height_px: u32,
) -> f32 {
    #[cfg(feature = "pdf")]
    {
        if page_height_px == 0 {
            return 1.0;
        }
        lazy_pdf_render_state
            .map(|(doc, _, _)| page_dimensions_pt(doc, page_idx).1 / page_height_px as f32)
            .filter(|scale| scale.is_finite() && *scale > 0.0)
            .unwrap_or(1.0)
    }
    #[cfg(not(feature = "pdf"))]
    {
        let _ = (page_idx, page_height_px);
        1.0
    }
}
#[cfg(all(any(feature = "ocr", feature = "ocr-wasm"), feature = "layout-detection"))]
pub(super) fn assemble_ocr_page_paragraphs(
    doc: &crate::types::internal::InternalDocument,
    page_height: u32,
    detection: Option<&crate::layout::DetectionResult>,
    points_per_pixel: f32,
    // The page's PDF `/Rotate` value (0/90/180/270), or `0` when unknown (e.g. no
    // `pdf` feature). Threaded to the detached-list-marker reattachment passes
    // below so their baseline/indent comparisons run in the rotation-corrected
    // frame instead of the raw raster one (#760) -- see
    // `pdf::structure::pipeline::DetachedMarkerFrame`.
    page_rotation_degrees: u32,
) -> Vec<crate::pdf::structure::types::PdfParagraph> {
    // `doc`'s bbox AND ocr_geometry are still both raw OCR raster pixels at this point in
    // the pure-OCR route (the pixel -> point rescale runs later, in
    // `build_pipeline_ocr_page_document`), so one real points-per-pixel ratio scales both
    // font-size fallback branches identically. See
    // `pdf::structure::adapters::OcrFontSizeScale` for the mixed route, where that is not
    // true.
    let font_size_scale = crate::pdf::structure::adapters::OcrFontSizeScale::uniform(points_per_pixel);
    #[cfg(feature = "ocr")]
    if let Some(detection) = detection {
        let hints = super::super::layout_hints::detection_to_layout_hints_pixel_space(detection, page_height as f32);
        let mut paragraphs = crate::pdf::structure::adapters::ocr_doc_to_layout_paragraphs(
            doc,
            page_height,
            &hints,
            0.5,
            0.2,
            font_size_scale,
        );
        apply_ocr_text_list_fallback(&mut paragraphs);
        // #729: a bare marker no ML hint ever classified (`is_list_item` still
        // `false`) is invisible to `reattach_ocr_layout_list_markers` below, whose
        // marker-side test requires the opposite -- see
        // `adapters::reattach_detached_ocr_list_markers`'s doc comment. Runs first so
        // both passes only ever see markers still in their own precondition's state.
        crate::pdf::structure::adapters::reattach_detached_ocr_list_markers(&mut paragraphs, page_rotation_degrees);
        // #729: `regroup_layout_lines_by_element` (above, inside
        // `ocr_doc_to_layout_paragraphs`) isolates an ML-hinted list marker into its
        // own paragraph and never rejoins it to its body. Gated independently of
        // `pipeline::REATTACH_DETACHED_LIST_MARKERS` -- see
        // `adapters::REATTACH_OCR_LAYOUT_LIST_MARKERS`'s doc comment.
        crate::pdf::structure::adapters::reattach_ocr_layout_list_markers(&mut paragraphs, page_rotation_degrees);
        return paragraphs;
    }
    // Both parameters are read only by the `#[cfg(feature = "ocr")]` block above, but this
    // function is gated on `any(ocr, ocr-wasm)`, so `ocr-wasm` + `layout-detection` leaves
    // them unread. No CI leg builds that pair, so nothing catches it. ~keep
    #[cfg(not(feature = "ocr"))]
    let _ = (detection, page_rotation_degrees);

    crate::pdf::structure::adapters::ocr_doc_to_paragraphs(doc, page_height, font_size_scale)
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct OcrMarginFilterOutcome {
    pub(super) removed: bool,
    pub(super) missing_geometry: bool,
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn filter_structured_ocr_page_by_page_margins(
    doc: &mut crate::types::internal::InternalDocument,
    page_height: f32,
    margins: crate::pdf::native::text::PageMarginFractions,
) -> OcrMarginFilterOutcome {
    if margins.top == 0.0 && margins.bottom == 0.0 {
        return OcrMarginFilterOutcome::default();
    }
    let original = std::mem::take(&mut doc.elements);
    let mut map = vec![None; original.len()];
    let mut outcome = OcrMarginFilterOutcome::default();
    for (index, element) in original.into_iter().enumerate() {
        let keep = if let Some(bbox) = element.bbox {
            crate::pdf::native::text::baseline_is_inside_page_margins(
                page_height - bbox.y1 as f32,
                0.0,
                page_height,
                margins,
            )
        } else {
            outcome.missing_geometry = true;
            true
        };
        if keep {
            map[index] = Some(doc.elements.len() as u32);
            doc.elements.push(element);
        } else {
            outcome.removed = true;
        }
    }
    remap_relationships(&mut doc.relationships, &map, &doc.elements);
    outcome
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn filter_ocr_paragraphs_by_page_margins(
    paragraphs: &mut Vec<crate::pdf::structure::types::PdfParagraph>,
    page_height: f32,
    margins: crate::pdf::native::text::PageMarginFractions,
) -> OcrMarginFilterOutcome {
    if margins.top == 0.0 && margins.bottom == 0.0 {
        return OcrMarginFilterOutcome::default();
    }

    let original_len = paragraphs.len();
    let mut missing_geometry = original_len == 0;
    paragraphs.retain(|paragraph| {
        let Some((_, baseline_y, _, _)) = paragraph.block_bbox else {
            missing_geometry = true;
            return true;
        };
        crate::pdf::native::text::baseline_is_inside_page_margins(baseline_y, 0.0, page_height, margins)
    });

    OcrMarginFilterOutcome {
        removed: paragraphs.len() != original_len,
        missing_geometry,
    }
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn filter_ocr_elements_by_page_margins(
    elements: &mut Vec<crate::types::OcrElement>,
    page_height: u32,
    margins: crate::pdf::native::text::PageMarginFractions,
) -> OcrMarginFilterOutcome {
    if margins.top == 0.0 && margins.bottom == 0.0 {
        return OcrMarginFilterOutcome::default();
    }
    if page_height == 0 {
        return OcrMarginFilterOutcome {
            removed: false,
            missing_geometry: !elements.is_empty(),
        };
    }

    let original_len = elements.len();
    let mut missing_geometry = false;
    elements.retain(|element| {
        let bottom_y = match &element.geometry {
            crate::types::OcrBoundingGeometry::Rectangle { top, width, height, .. } if *width > 0 && *height > 0 => {
                top.saturating_add(*height)
            }
            crate::types::OcrBoundingGeometry::Quadrilateral { points } if points.len() == 4 => {
                let min_x = points.iter().map(|point| point.x).min().unwrap_or(0);
                let max_x = points.iter().map(|point| point.x).max().unwrap_or(0);
                let min_y = points.iter().map(|point| point.y).min().unwrap_or(0);
                let max_y = points.iter().map(|point| point.y).max().unwrap_or(0);
                if min_x == max_x || min_y == max_y {
                    missing_geometry = true;
                    return true;
                }
                max_y
            }
            _ => {
                missing_geometry = true;
                return true;
            }
        };
        let baseline_y = page_height.saturating_sub(bottom_y.min(page_height)) as f32;
        crate::pdf::native::text::baseline_is_inside_page_margins(baseline_y, 0.0, page_height as f32, margins)
    });

    OcrMarginFilterOutcome {
        removed: elements.len() != original_len,
        missing_geometry,
    }
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn public_ocr_elements_for_pdf_page(
    elements: &mut [crate::types::OcrElement],
    config: &crate::core::config::ocr::OcrConfig,
    page_number: u32,
    page_height: u32,
    margins: crate::pdf::native::text::PageMarginFractions,
) -> (Vec<crate::types::OcrElement>, OcrMarginFilterOutcome) {
    for element in elements.iter_mut() {
        element.page_number = page_number;
    }
    let mut public_elements = filter_public_ocr_elements(elements, config);
    let outcome = filter_ocr_elements_by_page_margins(&mut public_elements, page_height, margins);
    if outcome.removed {
        public_elements = filter_public_ocr_elements(&public_elements, config);
    }
    (public_elements, outcome)
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn should_use_document_processing(
    supports_document_processing: bool,
    path_available: bool,
    margins: crate::pdf::native::text::PageMarginFractions,
) -> bool {
    supports_document_processing && path_available && margins.top == 0.0 && margins.bottom == 0.0
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn ocr_paragraphs_plain_text(paragraphs: &[crate::pdf::structure::types::PdfParagraph]) -> String {
    paragraphs
        .iter()
        .map(|paragraph| {
            if !paragraph.text.is_empty() {
                return paragraph.text.clone();
            }
            paragraph
                .lines
                .iter()
                .map(|line| {
                    line.segments
                        .iter()
                        .map(|segment| segment.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn ocr_margin_filter_capability_warning() -> crate::types::ProcessingWarning {
    crate::types::ProcessingWarning {
        source: std::borrow::Cow::Borrowed("ocr"),
        message: std::borrow::Cow::Borrowed(
            "Configured PDF page margins could not be applied to some OCR text because the backend did not return \
             text geometry; that OCR text was preserved unfiltered.",
        ),
    }
}
// Both call sites are the OCR-paragraph assembly blocks in `extract_with_ocr_for_page`:
// one gated on `layout-detection` *with* `ocr`/`ocr-wasm`, the other on
// `not(layout-detection)`. `layout-detection` without either OCR frontend (the
// `formula-recognition,pdf` CI leg) compiles both out, so a bare
// `any(ocr, ocr-pipeline)` gate here left the function -- and pipeline.rs's import of
// it -- with no caller. ~keep
#[cfg(all(
    any(feature = "ocr", feature = "ocr-pipeline"),
    any(feature = "ocr", feature = "ocr-wasm", not(feature = "layout-detection"))
))]
pub(super) fn apply_ocr_layout_content_filter(
    paragraphs: &mut [crate::pdf::structure::types::PdfParagraph],
    config: &ExtractionConfig,
) {
    let Some(filter) = config.content_filter.as_ref() else {
        return;
    };
    crate::pdf::structure::pipeline::un_mark_layout_furniture_per_config(
        paragraphs,
        filter.include_headers,
        filter.include_footers,
        filter.include_footnotes,
    );
}
/// Fill in `is_list_item` for paragraphs the OCR layout route left unclassified,
/// and OVERRIDE a layout classification that disagrees with an unambiguous text
/// list marker.
///
/// `ocr_doc_to_layout_paragraphs` (`crate::pdf::structure::adapters`) -- the OCR
/// counterpart of the native-PDF `finalize_paragraph`
/// (`crate::pdf::structure::pipeline`) -- derives `is_list_item` *exclusively* from
/// a layout-detection `ListItem` hint at >= 0.8 confidence
/// (`crate::pdf::structure::layout_classify::apply_hint_to_paragraph`). It never
/// falls back to a text-level marker check the way the native-PDF assembler always
/// does (`looks_like_list_item` runs unconditionally in `finalize_paragraph`,
/// independent of any layout hint). RT-DETR-style layout models commonly detect a
/// run of bulleted/numbered lines as one "Text" region rather than per-item
/// `ListItem` boxes, or miss/mislabel individual items outright -- including,
/// observed on `ordinance_2197_scanned.pdf`, classifying a numbered item as a
/// `Title`/`SectionHeader` (`## 8. Maximum height of structures: 50'`). When that
/// happens the item silently loses its list classification, and
/// `heuristically_restructured_ocr_pages`'s document-wide "already structured" gate
/// then refuses to re-derive it from segments, because that gate exists precisely
/// to protect a *correct* layout classification found elsewhere in the document
/// (see its doc comment). See #695.
///
/// This adds a text-marker pass directly onto the paragraphs the layout route
/// already built, so layout ADDS structure instead of silently dropping or
/// misclassifying what the text alone would have shown:
/// - A paragraph left with no classification at all (the common case) is filled in.
/// - A paragraph the layout route classified as a heading (`heading_level.is_some()`,
///   ordinarily from a `Title`/`SectionHeader` hint) is OVERRIDDEN when its text
///   unambiguously opens with a list marker: `heading_level` is cleared so the two
///   classifications never coexist (`assembly.rs`'s paragraph-to-element step checks
///   `heading_level` first and would otherwise render it as a heading, silently
///   discarding the list flag this function just set). This is safe specifically
///   because `looks_like_list_item` already rejects numbered SECTION headings via
///   `is_numbered_section_heading` ("1. INTRODUCTION", "3.2 Methods", "IV. Results"),
///   so text that passes the predicate is not a real heading in the first place --
///   the layout hint was wrong, not the text shape.
/// - A paragraph classified as code, a formula, or page furniture is NEVER touched:
///   those classifications are about the paragraph's *nature*, not a competing guess
///   at the same nature the way a `Text`/heading misclassification is, so a
///   coincidental marker-shaped prefix (a numbered code line, an OCR'd page-footer
///   digit) must not flip them.
/// - A paragraph the layout route already classified as a list item is left as-is.
///
/// Also reused, unmodified, as a *post-heuristic* fallback on the non-layout OCR
/// routes (`extract_mixed_ocr_native`, `run_ocr_pipeline`) for pages the document-global
/// heading/list heuristic (`heuristically_restructured_ocr_pages`) never classified --
/// either because it declined to run at all (`Plain` output, or another page in the same
/// document already carried an ML layout classification) or because it dropped a page
/// during the page split. Applying it there too (instead of only after ML layout hints)
/// closes the gap where a non-layout OCR document got zero list-item recovery of any kind
/// (#713). It is applied strictly *after* the heuristic has had its chance, never before:
/// pre-setting `is_list_item` earlier would itself flip `heuristically_restructured_ocr_pages`'s
/// "already structured" gate and skip heading detection for the whole document, trading a
/// list-item win for a heading-detection regression -- see that function's own doc comment.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn apply_ocr_text_list_fallback(paragraphs: &mut [crate::pdf::structure::types::PdfParagraph]) {
    for paragraph in paragraphs.iter_mut() {
        if paragraph.is_list_item || paragraph.is_code_block || paragraph.is_formula || paragraph.is_page_furniture {
            continue;
        }
        if crate::pdf::structure::pipeline::looks_like_list_item(paragraph.text.trim()) {
            paragraph.is_list_item = true;
            paragraph.heading_level = None;
            paragraph.layout_class = Some(crate::pdf::structure::types::LayoutHintClass::ListItem);
        }
    }
}
/// Rebuild paragraphs from the flat OCR text for each page that has none.
///
/// A page whose text the backend found entirely claimed by detected tables (#1571) is not
/// refilled: its flat text holds only lines the tables already carry, so a refill would print the
/// table a second time ahead of it. Keyed to that backend report rather than to the presence of a
/// table, because an embedded-image retry can add tables and text to a page whose own pass was
/// blank. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn fill_unstructured_ocr_pages(
    page_paragraphs: &mut [Option<Vec<crate::pdf::structure::types::PdfParagraph>>],
    page_texts: &[String],
    pages_text_claimed_by_tables: &[bool],
) {
    for (page_index, paragraphs) in page_paragraphs.iter_mut().enumerate() {
        let claimed_by_tables = pages_text_claimed_by_tables.get(page_index).copied().unwrap_or(false);
        if paragraphs.as_ref().is_none_or(Vec::is_empty) && !claimed_by_tables {
            let fallback = crate::pdf::structure::adapters::ocr_text_to_paragraphs(&page_texts[page_index]);
            if !fallback.is_empty() {
                *paragraphs = Some(fallback);
            }
        }
    }
}
/// Run the document-global heading/list heuristic
/// (`pdf::structure::extract_document_structure_from_segments`, the same font-clustering
/// pass the native xberg_native_pdf path uses) over already-built OCR paragraphs.
///
/// The heuristic is document-global: `build_heading_map` clusters font sizes across
/// every page, and `sparse_multi_page_heading_map` needs at least two pages in hand.
/// `ocr_doc_to_paragraphs` / `ocr_doc_to_layout_paragraphs` build paragraphs one OCR
/// page at a time as OCR runs, so they cannot host this pass -- it can only run once
/// every page's paragraphs exist, i.e. here, after the whole document has been OCR'd.
///
/// Returns `None` (leaving the caller's pre-existing, unstructured assembly in place)
/// when:
/// - `config.output_format` is [`OutputFormat::Plain`]: plain-text output must stay
///   byte-identical to before this heuristic existed. The heuristic only ever changes
///   `heading_level` / `is_list_item`, which downstream assembly turns into different
///   `ElementKind`s -- never touched for `Plain`.
/// - Any paragraph already carries a `heading_level` or `is_list_item` set by ML
///   layout detection (`ocr_doc_to_layout_paragraphs`). That path already recovers
///   structure (measured 13/12/13 headings on the reference fixture); re-deriving
///   structure from bare segments would discard that classification, not add to it.
/// - The heuristic itself returns no elements, or errors (logged, not propagated: the
///   caller's existing unstructured assembly is always a safe fallback here).
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn heuristically_restructured_ocr_pages(
    pages: &[Vec<crate::pdf::structure::types::PdfParagraph>],
    page_heights: &[f32],
    collected_tables: &[crate::types::Table],
    config: &ExtractionConfig,
) -> Option<crate::types::internal::InternalDocument> {
    if config.output_format == crate::core::config::OutputFormat::Plain && config.content_filter.is_none() {
        return None;
    }
    let strip_repeating_text = config
        .content_filter
        .as_ref()
        .is_some_and(|filter| filter.strip_repeating_text);
    let already_structured = pages
        .iter()
        .flatten()
        .any(|paragraph| paragraph.heading_level.is_some() || paragraph.is_list_item);
    if already_structured {
        if !strip_repeating_text {
            return None;
        }
        let mut filtered_pages = pages.to_vec();
        crate::pdf::structure::pipeline::strip_repeating_text_from_pages(&mut filtered_pages, page_heights);
        return Some(crate::pdf::structure::assemble_internal_document(
            filtered_pages,
            collected_tables,
            None,
            &[],
            &Default::default(),
        ));
    }

    let all_page_segments = crate::pdf::structure::adapters::segments_from_ocr_pages(pages);
    let k_clusters = config
        .pdf_options
        .as_ref()
        .and_then(|opts| opts.hierarchy.as_ref())
        .map(|hierarchy| hierarchy.k_clusters)
        .unwrap_or_else(|| crate::core::config::HierarchyConfig::default().k_clusters);
    let (strip_repeating_text, include_headers, include_footers, include_footnotes, include_watermarks) = config
        .content_filter
        .as_ref()
        .map(|filter| {
            (
                filter.strip_repeating_text,
                filter.include_headers,
                filter.include_footers,
                filter.include_footnotes,
                filter.include_watermarks,
            )
        })
        .unwrap_or((false, false, false, false, false));

    let result = crate::pdf::structure::extract_document_structure_from_segments(
        all_page_segments,
        crate::pdf::structure::SegmentStructureConfig {
            k_clusters,
            tables: collected_tables,
            outline_entries: &[],
            strip_repeating_text,
            include_headers,
            include_footers,
            include_footnotes,
            include_watermarks,
            used_structure_tree: false,
            image_positions: &[],
            images: None,
            inject_placeholders: false,
            layout_hints: None,
            allow_single_column: true,
            cancel_token: config.cancel_token.as_ref(),
            #[cfg(feature = "layout-detection")]
            layout_images: None,
            #[cfg(feature = "layout-detection")]
            layout_results: None,
            #[cfg(feature = "layout-detection")]
            table_model: crate::core::config::layout::TableModel::default(),
            #[cfg(feature = "layout-detection")]
            table_overlap_preference: crate::core::config::layout::TableOverlapPreference::default(),
            #[cfg(feature = "layout-detection")]
            acceleration: None,
            #[cfg(feature = "layout-detection")]
            session_thread_budget: 0,
        },
    );

    match result {
        Ok(doc) if !doc.elements.is_empty() && restructured_document_retains_prose(pages, collected_tables, &doc) => {
            Some(doc)
        }
        Ok(_) => None,
        Err(error) => {
            tracing::warn!(
                error = %error,
                "OCR document-level heading/list heuristic failed; falling back to unstructured OCR paragraphs"
            );
            None
        }
    }
}

/// Every canonical alphanumeric character of the baseline text must survive in the
/// restructured text for the document-global heuristic's output to be trusted (GH#1622).
///
/// `extract_document_structure_from_segments` rebuilds paragraphs from bare
/// `SegmentData` geometry with no knowledge of the ML layout regions (Picture, Table)
/// that `ocr_doc_to_layout_paragraphs` already classified -- it can silently drop a
/// segment its own font-clustering heuristic treats as furniture or noise. The
/// previous gate here only asked whether the *whole document* still had one non-empty,
/// non-table element (`.any(..)`), so a single surviving word anywhere passed it even
/// when an entire page's body vanished; measurement-discipline requires an absolute
/// count, not a document-wide existence check.
///
/// This is a hardening of a guard proven too weak on its own terms, not a confirmed fix
/// for any specific report: GH#1622's "entire page body lost" symptom was not reproduced
/// on the fixture available while investigating it (that document never exercises this
/// heuristic branch at all, since it already carries ML-detected headings).
///
/// Compares CHARACTERS, not words: a first attempt compared word tokens and regressed two
/// existing tests (`mixed_ocr_*_route_promotes_document_global_headings`). The restructuring
/// pass legitimately re-wraps text across the line/segment boundaries it reads -- on that
/// fixture, "list of findings" split across a line break came back as "list offindings",
/// dropping one space. No content word was lost, but word-level tokenization sees "of" and
/// "findings" vanish and "offindings" appear, reading a benign re-wrap as loss. Every
/// underlying letter is still present and in order-preserving-enough position; comparing the
/// alphanumeric character multiset (ignoring whitespace/word-boundary placement entirely,
/// the same way `pdf::structure::assembly::table_coverage_tokens` ignores punctuation
/// placement) is insensitive to exactly this class of re-wrap while remaining exactly as
/// sensitive to genuine deletion: a dropped word still drops that many characters from the
/// multiset, measurably. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
const MIN_OCR_STRUCTURE_HEURISTIC_CHAR_RETENTION: f64 = 1.0;

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn ocr_structure_heuristic_canonical_chars(text: &str) -> Vec<char> {
    text.chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Fraction of `baseline_text`'s canonical alphanumeric characters (as a multiset) that also
/// appear in `restructured_text`. Mirrors `extractors::image::alphanumeric_token_retention`'s
/// proven shape -- the fix for the same class of loss on the plain-image layout path -- but is
/// reimplemented locally rather than exported: that function is private to `image.rs` and
/// this is only the second call site (`avoid-duplication`: extract after the third). Unlike
/// that word-token version, this compares characters (see the constant's doc comment for why).
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn ocr_structure_heuristic_char_retention(restructured_text: &str, baseline_text: &str) -> f64 {
    let baseline_chars = ocr_structure_heuristic_canonical_chars(baseline_text);
    if baseline_chars.is_empty() {
        return 1.0;
    }

    let mut restructured_char_counts = std::collections::HashMap::<char, usize>::new();
    for character in ocr_structure_heuristic_canonical_chars(restructured_text) {
        *restructured_char_counts.entry(character).or_default() += 1;
    }

    let retained = baseline_chars
        .iter()
        .filter(|character| {
            let Some(count) = restructured_char_counts.get_mut(*character) else {
                return false;
            };
            if *count == 0 {
                return false;
            }
            *count -= 1;
            true
        })
        .count();

    retained as f64 / baseline_chars.len() as f64
}

/// Whether `doc` -- built by `extract_document_structure_from_segments` from
/// `segments_from_ocr_pages(pages)` -- actually kept the prose `pages` (and
/// `collected_tables`) carried.
///
/// `segments_from_ocr_pages` harvests `SegmentData` only out of `PdfParagraph.lines`.
/// A bare-text OCR backend with no per-line geometry (the VLM backend never populates
/// `ocr_internal_document` or `ocr_elements` -- see
/// `crate::llm::vlm_ocr::VlmOcrBackend::process_image`) builds its paragraphs via
/// `ocr_text_to_paragraphs`, which carries the page's content in `.text` and leaves
/// `.lines` empty. Every such page therefore contributes zero segments to this
/// heuristic no matter how much prose it holds, so `extract_document_structure_from_segments`
/// reconstructs zero paragraphs for it. Declining here sends the caller to its own
/// `.text`-based fallback assembly instead, which never loses prose this way. The
/// per-paragraph `lines.is_empty()` refusal below is unconditional (not folded into the
/// token check) so a geometry-backed page retaining its prose must not hide a separate
/// bare-text page that contributed no segments.
///
/// Beyond that, the token check below (GH#1622) compares `doc`'s rendered plain text
/// against the *lossless* baseline assembly of the same `pages`/`collected_tables`
/// (`assemble_internal_document`, unconditionally preserved regardless of layout
/// classification -- see `ocr_doc_to_layout_paragraphs` and `layout_classify`'s Picture
/// arm) and requires every baseline token to survive; anything less rejects the
/// heuristic's output entirely so the caller falls back to that same lossless baseline. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn restructured_document_retains_prose(
    pages: &[Vec<crate::pdf::structure::types::PdfParagraph>],
    collected_tables: &[crate::types::Table],
    doc: &crate::types::internal::InternalDocument,
) -> bool {
    if pages
        .iter()
        .flatten()
        .any(|paragraph| !paragraph.text.trim().is_empty() && paragraph.lines.is_empty())
    {
        return false;
    }
    let had_prose = pages
        .iter()
        .flatten()
        .any(|paragraph| !paragraph.text.trim().is_empty() || !paragraph.lines.is_empty())
        || collected_tables.iter().any(|table| !table.markdown.trim().is_empty());
    if !had_prose {
        return true;
    }

    let baseline = crate::pdf::structure::assemble_internal_document(
        pages.to_vec(),
        collected_tables,
        None,
        &[],
        &Default::default(),
    );
    let baseline_text = crate::rendering::render_plain(&baseline);
    let restructured_text = crate::rendering::render_plain(doc);
    ocr_structure_heuristic_char_retention(&restructured_text, &baseline_text)
        >= MIN_OCR_STRUCTURE_HEURISTIC_CHAR_RETENTION
}
/// Split the single, document-wide [`crate::types::internal::InternalDocument`]
/// [`heuristically_restructured_ocr_pages`] produced back into one per-page document per
/// real OCR'd page number, for [`extract_mixed_ocr_native`].
///
/// That function's per-page `structured_ocr_pages` map is what
/// [`merge_structured_ocr_pages_into_internal_document`] uses to splice each OCR'd page's
/// structure back into the surrounding native document at the right position -- it needs
/// one document per page, not one document spanning every OCR'd page. The combined
/// document's elements/tables/images all carry a real page number (`element.page`,
/// `table.page_number`, `image.page_number`) because the caller pads its `pages` argument
/// to the heuristic out to the *document's* full page count and only populates the slots
/// for actually-OCR'd pages (see the caller), so `extract_document_structure_from_segments`
/// numbers pages 1:1 with real page numbers, not with position in a filtered subset.
///
/// `ElementKind::Table`/`ElementKind::Image` index into the *combined* document's
/// `tables`/`images` vecs; each per-page document gets its own 0-based vecs, so those
/// indices are remapped here. Relationships (e.g. caption associations) are dropped: they
/// index into the combined document's element list, which no longer exists in one piece
/// after this split, matching the pre-existing per-page builders
/// ([`build_mixed_ocr_page_document`], [`build_pipeline_ocr_page_document`]), neither of
/// which carries cross-page relationships either.
///
/// A page in `ocr_page_numbers` that ended up with no elements, tables, or images after
/// the split (e.g. the heuristic dropped an empty page) has no entry in the returned map,
/// mirroring the `None`-means-"nothing structured" contract the caller's pre-existing
/// per-page builders already use.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn split_document_global_ocr_structure_by_page(
    doc: crate::types::internal::InternalDocument,
    ocr_page_numbers: &[u32],
) -> ahash::AHashMap<u32, crate::types::internal::InternalDocument> {
    use crate::types::internal::ElementKind;

    let mut elements_by_page: ahash::AHashMap<u32, Vec<crate::types::internal::InternalElement>> =
        ahash::AHashMap::new();
    for element in doc.elements {
        if matches!(element.kind, ElementKind::PageBreak) {
            continue;
        }
        if let Some(page) = element.page {
            elements_by_page.entry(page).or_default().push(element);
        }
    }

    let tables = doc.tables;
    let images = doc.images;

    let mut result = ahash::AHashMap::with_capacity(ocr_page_numbers.len());
    for &page_number in ocr_page_numbers {
        let Some(mut elements) = elements_by_page.remove(&page_number) else {
            continue;
        };

        let mut page_tables = Vec::new();
        let mut page_images = Vec::new();
        for element in &mut elements {
            match &mut element.kind {
                ElementKind::Table { table_index } => {
                    if let Some(table) = tables.get(*table_index as usize) {
                        let new_index = page_tables.len() as u32;
                        page_tables.push(table.clone());
                        *table_index = new_index;
                    }
                }
                ElementKind::Image { image_index } => {
                    if let Some(image) = images.get(*image_index as usize) {
                        let new_index = page_images.len() as u32;
                        page_images.push(image.clone());
                        *image_index = new_index;
                    }
                }
                _ => {}
            }
        }

        if elements.is_empty() && page_tables.is_empty() && page_images.is_empty() {
            continue;
        }

        let mut page_doc = crate::types::internal::InternalDocument::new("pdf");
        page_doc.elements = elements;
        page_doc.tables = page_tables;
        page_doc.images = page_images;
        result.insert(page_number, page_doc);
    }
    result
}
/// Convert a TATR-recognized table into the public [`crate::types::Table`],
/// carrying over its `detection_bbox` and assigning a deterministic `table_id`.
///
/// `table_index` is the table's 0-based position in the document's push order
/// (see the caller), so the id is `"table-{table_index + 1}"` — never derived
/// from randomness or wall-clock time, so the same input document always
/// produces the same id. See [`crate::types::Table::table_id`] for the shared
/// scheme doc.
#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
pub(super) fn recognized_table_to_public_table(
    recognized: &crate::RecognizedTable,
    page_number: u32,
    table_index: usize,
) -> crate::types::Table {
    crate::types::Table {
        cells: recognized.cells.clone(),
        markdown: recognized.markdown.clone(),
        page_number,
        bounding_box: Some(crate::types::BoundingBox {
            x0: recognized.detection_bbox.x1 as f64,
            y0: recognized.detection_bbox.y1 as f64,
            x1: recognized.detection_bbox.x2 as f64,
            y1: recognized.detection_bbox.y2 as f64,
        }),
        table_id: Some(format!("table-{}", table_index + 1)),
        columns: recognized.cells.first().cloned(),
        cell_styles: Vec::new(),
    }
}

#[cfg(all(test, feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
mod public_ocr_handoff_tests {
    use super::*;
    use crate::extraction::derive::{adopt_public_ocr_document, derive_extraction_result};
    use crate::types::document_structure::{AnnotationKind, NodeContent, TextAnnotation};
    use crate::types::internal::{
        ElementKind, InternalDocument, InternalElement, Relationship, RelationshipKind, RelationshipTarget,
    };
    use crate::types::{BoundingBox, ExtractedDocument, ExtractedImage, PageBoundary, Table};

    const TABLE: &str =
        "<table><tr><td rowspan=\"2\">A</td><td colspan=\"2\">B</td></tr><tr><td>C</td><td>D</td></tr></table>";
    const STALE: &str = "Superseded native page text.";
    const APPENDIX: &str = "Native appendix remains.";

    fn public_result() -> ExtractedDocument {
        let node = |content| serde_json::json!({"content": content, "content_layer": "body"});
        ExtractedDocument {
            content: "OCR opening paragraph.\n\nA B C D\n\nTable caption outside the table.".into(),
            document: Some(
                serde_json::from_value(serde_json::json!({
                    "nodes": [
                        node(serde_json::json!({"node_type": "paragraph", "text": "OCR opening paragraph."})),
                        node(serde_json::json!({"node_type": "raw_block", "format": "html", "content": TABLE})),
                        node(serde_json::json!({"node_type": "paragraph", "text": "Table caption outside the table."}))
                    ],
                    "relationships": [{"source": 2, "target": 1, "kind": "caption"}]
                }))
                .unwrap(),
            ),
            ..Default::default()
        }
    }

    fn adopted_page(
        result: &mut ExtractedDocument,
        margins: crate::pdf::native::text::PageMarginFractions,
    ) -> InternalDocument {
        adopt_public_ocr_document(result, &Default::default()).unwrap();
        let (page, paragraphs) =
            build_mixed_ocr_page_document(result, &Default::default(), 1, 400, 800, 200.0, 400.0, margins).unwrap();
        assert!(
            paragraphs.is_empty(),
            "explicit public nodes must not be offered for paragraph reconstruction"
        );
        page
    }

    fn native_document() -> InternalDocument {
        let mut doc = InternalDocument::new("pdf");
        doc.append_ocr_text = true;
        doc.tables.push(Table {
            cells: vec![vec!["native inventory cell".into()]],
            markdown: "| native inventory cell |".into(),
            page_number: 1,
            ..Default::default()
        });
        let nested_ocr = ExtractedDocument {
            content: "OCR opening paragraph.".into(),
            ..Default::default()
        };
        doc.images.push(ExtractedImage {
            image_index: 0,
            page_number: Some(1),
            ocr_result: Some(Box::new(nested_ocr)),
            ..Default::default()
        });
        doc.annotations = Some(vec![
            serde_json::from_value(serde_json::json!({
                "annotation_type": "text", "content": "Native review note", "page_number": 1, "author": "Reviewer"
            }))
            .unwrap(),
        ]);
        doc.push_element(InternalElement::text(ElementKind::Paragraph, STALE, 0).with_page(1));
        doc.push_element(InternalElement::text(ElementKind::Table { table_index: 0 }, "", 0).with_page(1));
        doc.push_element(InternalElement::text(ElementKind::Image { image_index: 0 }, "", 0).with_page(1));
        doc.push_element(InternalElement::text(ElementKind::PageBreak, "", 0));
        let mut appendix = InternalElement::text(ElementKind::Paragraph, APPENDIX, 0).with_page(2);
        appendix.annotations.push(TextAnnotation {
            start: 0,
            end: 6,
            kind: AnnotationKind::Bold,
        });
        doc.push_element(appendix);
        for (source, target) in [(4, 2), (0, 2), (4, 1)] {
            doc.relationships.push(Relationship {
                source,
                target: RelationshipTarget::Index(target),
                kind: RelationshipKind::Caption,
            });
        }
        doc
    }

    fn selected_replacements(text: &str) -> ahash::AHashMap<u32, String> {
        let native_text = format!("{STALE}\n\n{APPENDIX}");
        let boundaries = [
            PageBoundary {
                byte_start: 0,
                byte_end: STALE.len(),
                page_number: 1,
            },
            PageBoundary {
                byte_start: STALE.len() + 2,
                byte_end: native_text.len(),
                page_number: 2,
            },
        ];
        accepted_ocr_page_replacements(
            &native_text,
            &boundaries,
            &ahash::AHashMap::from_iter([(1, text.into())]),
            &Default::default(),
        )
    }

    #[test]
    fn public_ocr_handoff_selected_hybrid_replaces_flow_but_retains_native_assets_and_surviving_references() {
        let mut native = native_document();
        let assets_before = serde_json::to_value((&native.tables, &native.images, &native.annotations)).unwrap();
        let mut result = public_result();
        let page = adopted_page(&mut result, Default::default());
        let accepted = selected_replacements(&result.content);
        assert!(accepted.contains_key(&1));
        merge_structured_ocr_pages_into_internal_document(
            &mut native,
            &accepted,
            &ahash::AHashMap::from_iter([(1, page)]),
        );
        assert_eq!(
            serde_json::to_value((&native.tables, &native.images, &native.annotations)).unwrap(),
            assets_before
        );
        assert!(
            !native
                .elements
                .iter()
                .any(|element| element.text == STALE || matches!(element.kind, ElementKind::Table { .. }))
        );
        let appendix = native.elements.iter().find(|element| element.text == APPENDIX).unwrap();
        assert_eq!(appendix.annotations.len(), 1);
        assert_eq!(
            native.relationships.len(),
            2,
            "only the surviving native reference and adopted caption remain"
        );
        for relationship in &native.relationships {
            assert!((relationship.source as usize) < native.elements.len());
            let RelationshipTarget::Index(target) = relationship.target else {
                panic!("fixture references must stay resolved");
            };
            assert!((target as usize) < native.elements.len());
        }
        assert!(native.relationships.iter().any(|relationship| {
            let RelationshipTarget::Index(target) = relationship.target else {
                return false;
            };
            native.elements[relationship.source as usize].text == APPENDIX
                && matches!(native.elements[target as usize].kind, ElementKind::Image { .. })
        }));
        let derived = derive_extraction_result(native, true, crate::core::config::OutputFormat::Markdown);
        let markdown = derived
            .formatted_content
            .as_ref()
            .expect("Markdown output must be rendered");
        assert_eq!(markdown.matches(TABLE).count(), 1, "{markdown}");
        assert_eq!(
            markdown.matches("OCR opening paragraph.").count(),
            1,
            "nested image OCR must not render twice"
        );
        assert!(!markdown.contains(STALE) && !markdown.contains("native inventory cell"));
        assert!(markdown.contains("**Native** appendix remains."), "{markdown}");
        assert!(markdown.find(TABLE).unwrap() < markdown.find("Table caption outside").unwrap());
        assert_eq!(derived.tables.len(), 1);
        assert_eq!(derived.images.as_ref().unwrap().len(), 1);
        assert_eq!(
            derived.annotations.as_ref().unwrap()[0].content.as_deref(),
            Some("Native review note")
        );
        let structure = derived.document.unwrap();
        let appendix = structure
            .nodes
            .iter()
            .find(|node| matches!(&node.content, NodeContent::Paragraph { text } if text == APPENDIX))
            .unwrap();
        assert_eq!(
            appendix.annotations,
            vec![TextAnnotation {
                start: 0,
                end: 6,
                kind: AnnotationKind::Bold
            }]
        );
        assert_eq!(structure.relationships.len(), 2);
        assert!(structure.relationships.iter().any(|relationship| matches!(
            structure.nodes[relationship.target.0 as usize].content,
            NodeContent::RawBlock { .. }
        )));
    }

    #[test]
    fn public_ocr_handoff_rejected_page_leaves_native_document_and_assets_unchanged() {
        let mut native = native_document();
        let before = serde_json::to_value(&native).unwrap();
        let rendered_before = crate::rendering::render_markdown(&native);
        let mut result = public_result();
        let mut page = adopted_page(&mut result, Default::default());
        // ~keep Failure injection at the composition boundary, not support for public asset
        // inventories: even an unselected page carrying assets must not leak them.
        page.tables.push(Table {
            cells: vec![vec!["rejected table".into()]],
            ..Default::default()
        });
        page.images.push(ExtractedImage {
            image_index: 0,
            ..Default::default()
        });
        let accepted = selected_replacements("");
        assert!(accepted.is_empty());
        merge_structured_ocr_pages_into_internal_document(
            &mut native,
            &accepted,
            &ahash::AHashMap::from_iter([(1, page)]),
        );
        assert_eq!(serde_json::to_value(&native).unwrap(), before);
        let rendered = crate::rendering::render_markdown(&native);
        assert_eq!(rendered, rendered_before);
        assert!(rendered.contains(STALE));
        assert!(!rendered.contains(TABLE) && !rendered.contains("rejected table"));
    }

    #[test]
    fn public_ocr_handoff_supplied_bboxes_are_filtered_scaled_and_flipped_without_losing_structure() {
        let mut result = public_result();
        let structure = result.document.as_mut().unwrap();
        let mut footer = structure.nodes[0].clone();
        footer.content = NodeContent::Paragraph {
            text: "Bottom margin prose.".into(),
        };
        structure.nodes.push(footer);
        for (node, bbox) in structure.nodes.iter_mut().zip([
            BoundingBox {
                x0: 20.0,
                y0: 0.0,
                x1: 200.0,
                y1: 40.0,
            },
            BoundingBox {
                x0: 80.0,
                y0: 200.0,
                x1: 360.0,
                y1: 400.0,
            },
            BoundingBox {
                x0: 80.0,
                y0: 410.0,
                x1: 360.0,
                y1: 450.0,
            },
            BoundingBox {
                x0: 20.0,
                y0: 760.0,
                x1: 200.0,
                y1: 790.0,
            },
        ]) {
            node.bbox = Some(bbox);
        }
        let page = adopted_page(
            &mut result,
            crate::pdf::native::text::PageMarginFractions { top: 0.1, bottom: 0.1 },
        );
        assert_eq!(page.elements.len(), 2);
        assert!(matches!(page.elements[0].kind, ElementKind::RawBlock));
        assert_eq!(page.elements[0].text, TABLE);
        assert_eq!(
            page.elements[0].bbox,
            Some(BoundingBox {
                x0: 40.0,
                y0: 200.0,
                x1: 180.0,
                y1: 300.0
            })
        );
        assert_eq!(
            page.elements[1].bbox,
            Some(BoundingBox {
                x0: 40.0,
                y0: 175.0,
                x1: 180.0,
                y1: 195.0
            })
        );
        assert!(
            page.processing_warnings.is_empty(),
            "complete supplied geometry must not report a missing-geometry capability"
        );
        assert_eq!(
            page.relationships,
            vec![Relationship {
                source: 1,
                target: RelationshipTarget::Index(0),
                kind: RelationshipKind::Caption
            }]
        );
        assert!(page.elements.iter().all(|element| element.page == Some(1)
            && element.ocr_geometry.is_none()
            && element.ocr_confidence.is_none()));
        assert!(!result.content.contains("OCR opening paragraph.") && !result.content.contains("Bottom margin prose."));
        assert_eq!(crate::rendering::render_markdown(&page).matches(TABLE).count(), 1);
    }

    #[test]
    fn public_ocr_handoff_absent_bboxes_remain_absent_and_report_margin_capability() {
        let mut result = public_result();
        let transcript = result.content.clone();
        let page = adopted_page(
            &mut result,
            crate::pdf::native::text::PageMarginFractions { top: 0.1, bottom: 0.1 },
        );
        assert_eq!(page.elements.len(), 3);
        assert_eq!(result.content, transcript);
        assert!(page.elements.iter().all(|element| element.bbox.is_none()
            && element.ocr_geometry.is_none()
            && element.ocr_confidence.is_none()));
        assert_eq!(page.processing_warnings.len(), 1);
        let warning = ocr_margin_filter_capability_warning();
        assert_eq!(page.processing_warnings[0].source, warning.source);
        assert_eq!(page.processing_warnings[0].message, warning.message);
        assert_eq!(crate::rendering::render_markdown(&page).matches(TABLE).count(), 1);
        assert_eq!(page.relationships.len(), 1);
    }
}

#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline")))]
mod tests {
    use super::*;
    use crate::pdf::structure::types::PdfParagraph;
    use crate::types::internal::InternalDocument;
    use crate::types::internal_builder::InternalDocumentBuilder;

    /// A geometry-backed paragraph (populated `lines`, empty `.text`), matching the shape
    /// `ocr_doc_to_layout_paragraphs` actually produces. `restructured_document_retains_prose`
    /// unconditionally rejects a paragraph that has prose `.text` but empty `.lines` (the
    /// bare-text VLM-backend shape) before it ever reaches the token-retention check this
    /// module adds, so a `.lines`-populated paragraph is what exercises that check.
    fn bare_paragraph(text: &str) -> PdfParagraph {
        let segment = crate::pdf::hierarchy::SegmentData {
            text: text.to_string(),
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 12.0,
            font_size: 12.0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: 0.0,
            rotation_degrees: 0.0,
            assigned_role: None,
        };
        let line = crate::pdf::structure::types::PdfLine {
            segments: vec![segment],
            baseline_y: 0.0,
            dominant_font_size: 12.0,
            is_bold: false,
            is_monospace: false,
        };
        PdfParagraph {
            word_count: PdfParagraph::compute_word_count("", std::slice::from_ref(&line)),
            text: String::new(),
            lines: vec![line],
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            layout_region_path: None,
            caption_for: None,
            block_bbox: None,
        }
    }

    fn doc_with_paragraph(text: &str) -> InternalDocument {
        let mut builder = InternalDocumentBuilder::new("pdf");
        builder.push_paragraph(text, vec![], None, None);
        builder.build()
    }

    /// GH#1622, direction 1: a heuristic-restructured document that dropped a word the
    /// lossless baseline carried must be rejected, so the caller falls back to the
    /// lossless assembly and the word is recovered rather than silently lost.
    #[test]
    fn should_reject_restructured_document_that_dropped_a_baseline_word() {
        let pages = vec![vec![bare_paragraph(
            "Site 07 access blocked by washout near ridge trail",
        )]];
        let restructured = doc_with_paragraph("Site 07 access blocked by washout");

        assert!(
            !restructured_document_retains_prose(&pages, &[], &restructured),
            "dropping 'near ridge trail' must be detected and rejected"
        );
    }

    /// GH#1622, direction 2 (negative control against #1616-style duplication): a
    /// restructured document that retains every baseline word exactly once must be
    /// accepted as-is -- proving the gate does not force a spurious fallback (which
    /// would duplicate content) when nothing was lost.
    #[test]
    fn should_accept_restructured_document_that_retains_every_baseline_word_once() {
        let pages = vec![vec![bare_paragraph("Site 07 access blocked by washout")]];
        let restructured = doc_with_paragraph("Site 07 access blocked by washout");

        assert!(
            restructured_document_retains_prose(&pages, &[], &restructured),
            "a fully-retained restructuring must not be rejected"
        );
    }

    /// A table's markdown content counts toward the baseline: text TATR moved out of
    /// plain paragraphs and into a recognized table must not be treated as lost.
    #[test]
    fn should_treat_recognized_table_markdown_as_retained_baseline_text() {
        let pages: Vec<Vec<PdfParagraph>> = vec![vec![]];
        let table = crate::types::Table {
            markdown: "| Samples | Status |\n| --- |\n| 6 | Complete |".to_string(),
            page_number: 1,
            ..Default::default()
        };
        let mut builder = InternalDocumentBuilder::new("pdf");
        builder.push_table(table.clone(), Some(1), None);
        let restructured = builder.build();

        assert!(restructured_document_retains_prose(&pages, &[table], &restructured));
    }

    /// A restructured document with no elements at all, compared against a baseline
    /// that had real prose, must be rejected -- the "entire body lost" shape of GH#1622.
    #[test]
    fn should_reject_an_empty_restructured_document_when_baseline_had_prose() {
        let pages = vec![vec![bare_paragraph("This entire page of prose vanished")]];
        let empty = InternalDocumentBuilder::new("pdf").build();

        assert!(!restructured_document_retains_prose(&pages, &[], &empty));
    }
}
