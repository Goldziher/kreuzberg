#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) type EncodedPage = (usize, std::sync::Arc<Vec<u8>>, u32, u32);
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
type RenderedPages = Vec<(usize, image::DynamicImage)>;
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
type PageRenderFailures = Vec<(usize, crate::XbergError)>;
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
type PageRenderOutcomes = (RenderedPages, PageRenderFailures);
/// Render only specific PDF pages to images for OCR processing.
///
/// `page_indices` are 0-indexed. Only the requested pages are rendered,
/// returned as `(page_index, image)` pairs.
// Gated to `ocr` rather than `any(ocr, ocr-pipeline)` to match its only
// callers in the `#[cfg(all(test, feature = "ocr"))]` test module. ~keep
#[cfg(all(test, feature = "ocr", feature = "pdf"))]
pub(crate) fn render_selected_pages_for_ocr(
    content: &[u8],
    page_indices: &[usize],
) -> crate::Result<Vec<(usize, image::DynamicImage)>> {
    let (doc, page_count, page_rotations) = open_pdf_for_page_ocr(content)?;
    let valid_indices = valid_page_indices(page_indices, page_count);
    let (pages, failures) = render_selected_pages_from_document(
        &doc,
        &page_rotations,
        &valid_indices,
        &crate::extractors::security::SecurityLimits::default(),
        None,
    );
    if let Some((_, error)) = failures.into_iter().next() {
        return Err(error);
    }
    Ok(pages)
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn open_pdf_for_page_ocr(content: &[u8]) -> crate::Result<(xberg_native_pdf::PdfDocument, usize, Vec<u32>)> {
    let doc = xberg_native_pdf::PdfDocument::from_bytes(content.to_vec()).map_err(|e| crate::XbergError::Parsing {
        message: format!("Failed to open PDF for rendering: {}", e),
        source: None,
    })?;

    let page_count = doc.page_count().map_err(|e| crate::XbergError::Parsing {
        message: format!("Failed to get PDF page count: {}", e),
        source: None,
    })?;

    let page_rotations = crate::pdf::render::get_page_rotations(&doc, page_count);
    Ok((doc, page_count, page_rotations))
}
/// Page MediaBox size in points, falling back to US Letter (612x792pt) when the
/// PDF omits a MediaBox or it cannot be read.
///
/// Mirrors `crate::pdf::render`'s private page-dimension lookup; duplicated here
/// (rather than made `pub(crate)` there) because that module builds DPI-safeguard
/// logic on top of it that has no bearing on this file, and this needs only the
/// two-line MediaBox read to convert OCR pixel bboxes back into the PDF page's own
/// coordinate space (#1423).
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn page_dimensions_pt(doc: &xberg_native_pdf::PdfDocument, page_index: usize) -> (f32, f32) {
    doc.get_page_media_box(page_index)
        .map(|(llx, lly, urx, ury)| ((urx - llx).abs(), (ury - lly).abs()))
        .unwrap_or((612.0, 792.0))
}
/// Per-page `/Rotate` values and MediaBox dimensions (points) for a document whose page
/// rasters were rendered by someone else and handed to OCR pre-rendered — in practice the
/// layout-detection pass, which always has the original PDF bytes alongside its rasters.
///
/// One open serves both hints `ocr_config_with_page_rotation_hint` takes: the rotation and the
/// `source_dpi` the raster width implies (#1753). An unreadable document yields the same
/// hint-free result the route had before, not an error: neither hint is load-bearing. ~keep
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) struct PreRenderedPageGeometry {
    /// One `/Rotate` value per page, zero-filled for a document that would not open.
    pub(super) rotations: Vec<u32>,
    /// One `(width, height)` MediaBox pair in points per page, empty for a document that
    /// would not open. Callers index it with `get`, so the empty case needs no separate
    /// branch. ~keep
    pub(super) dimensions_pt: Vec<(f32, f32)>,
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn pre_rendered_page_geometry(content: &[u8], page_count: usize) -> PreRenderedPageGeometry {
    match xberg_native_pdf::PdfDocument::from_bytes(content.to_vec()) {
        Ok(doc) => PreRenderedPageGeometry {
            rotations: crate::pdf::render::get_page_rotations(&doc, page_count),
            dimensions_pt: (0..page_count).map(|page| page_dimensions_pt(&doc, page)).collect(),
        },
        Err(error) => {
            tracing::warn!(
                %error,
                "failed to open PDF to read pre-rendered page geometry; continuing without rotation or DPI hints"
            );
            PreRenderedPageGeometry {
                rotations: vec![0; page_count],
                dimensions_pt: Vec::new(),
            }
        }
    }
}

/// Derive the [`super::pipeline::ocr_config_with_page_rotation_hint`] `source_dpi` value for a
/// page raster this route did not render itself.
///
/// Both axes must agree on the implied resolution before the hint is trusted. A raster that is
/// not a whole-page, MediaBox-oriented render of this page — a display-oriented render of a
/// 90/270-rotated page, or a crop — disagrees on a non-square page and gets no hint at all,
/// which leaves it on the preprocessor's 72-DPI assumption rather than a confidently wrong
/// number. ~keep
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn pre_rendered_page_source_dpi(
    page_dimensions_pt: (f32, f32),
    rendered_width_px: u32,
    rendered_height_px: u32,
) -> Option<f64> {
    /// Largest relative disagreement between the two axes' implied DPI still attributable to a
    /// renderer rounding each axis independently to a whole pixel.
    const AXIS_DPI_AGREEMENT_TOLERANCE: f64 = 0.02;

    let (width_pt, height_pt) = page_dimensions_pt;
    let width_dpi = crate::pdf::render::rendered_page_dpi(rendered_width_px, width_pt)?;
    let height_dpi = crate::pdf::render::rendered_page_dpi(rendered_height_px, height_pt)?;
    if (width_dpi - height_dpi).abs() > width_dpi * AXIS_DPI_AGREEMENT_TOLERANCE {
        tracing::debug!(
            width_dpi,
            height_dpi,
            "pre-rendered page raster axes disagree on resolution; leaving source_dpi unknown"
        );
        return None;
    }
    Some(width_dpi)
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn open_pdf_for_full_ocr(content: &[u8]) -> crate::Result<(xberg_native_pdf::PdfDocument, usize, Vec<u32>)> {
    let doc = xberg_native_pdf::PdfDocument::from_bytes(content.to_vec()).map_err(|e| crate::XbergError::Parsing {
        message: format!("Failed to open PDF for OCR streaming: {:?}", e),
        source: None,
    })?;
    let page_count = doc.page_count().map_err(|e| crate::XbergError::Parsing {
        message: format!("Failed to get document page count: {:?}", e),
        source: None,
    })?;
    let page_rotations = crate::pdf::render::get_page_rotations(&doc, page_count);
    Ok((doc, page_count, page_rotations))
}
/// Luma value at or below which a sampled pixel counts as ink.
///
/// Mid-gray, matching the `< 128` threshold the render-path glyph-ink assertions use
/// (`crate::pdf::render`'s `dark_pixels_in_cell`).
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) const INK_LUMA_THRESHOLD: u8 = 128;
/// Sample every Nth pixel on both axes when probing a page raster for ink.
///
/// A blank-substituted page raster is uniformly white, so any subsample detects it;
/// 4 keeps the probe at 1/16 of the pixels (≈131k samples for a 150-DPI Letter page).
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) const INK_PROBE_STRIDE: u32 = 4;
/// Fraction of sampled pixels that must be ink for the raster to count as non-blank.
///
/// 0.01% of a 150-DPI Letter page's subsample is ~13 pixels — below a single glyph's
/// ink, but far above the zero a blank-substituted raster yields.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) const INK_BLANK_MAX_DARK_RATIO: f64 = 0.0001;
/// Longest OCR text (in non-whitespace characters) that still justifies paying for an
/// ink probe of the page raster.
///
/// The probe exists only to catch a backend that *describes* a blank page instead of
/// returning nothing ("The image is entirely blank."), which `is_page_text_blank`'s
/// 3-character floor reads as content. Such answers are a sentence or two; a page that
/// was genuinely transcribed runs far longer. Gating on length keeps the PNG decode off
/// the hot path for real pages.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) const MAX_INK_PROBE_TEXT_CHARS: usize = 200;
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) use crate::image::dpi::{OCR_PNG_ENCODE_BYTES_PER_PIXEL, OCR_PNG_ENCODE_FIXED_BYTES};
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn validate_png_encode_batch_peak<'a>(
    images: impl IntoIterator<Item = &'a image::DynamicImage>,
    parallel: bool,
    security_limits: &crate::extractors::security::SecurityLimits,
) -> crate::Result<()> {
    let mut dimensions = (1, 1);
    let mut source_bytes = 0_u64;
    let mut output_bytes = 0_u64;
    let mut conversion_bytes = 0_u64;
    for image in images {
        dimensions = (image.width(), image.height());
        source_bytes = source_bytes
            .checked_add(u64::try_from(image.as_bytes().len()).unwrap_or(u64::MAX))
            .ok_or_else(|| {
                crate::extraction::image_decode::image_dimension_error(dimensions.0, dimensions.1, u64::MAX, u64::MAX)
            })?;
        let conversion = crate::extraction::image_decode::decoded_byte_count(dimensions.0, dimensions.1, 3)?;
        conversion_bytes = if parallel {
            conversion_bytes.checked_add(conversion)
        } else {
            Some(conversion_bytes.max(conversion))
        }
        .ok_or_else(|| {
            crate::extraction::image_decode::image_dimension_error(dimensions.0, dimensions.1, u64::MAX, u64::MAX)
        })?;
        let output = crate::extraction::image_decode::decoded_byte_count(
            dimensions.0,
            dimensions.1,
            OCR_PNG_ENCODE_BYTES_PER_PIXEL,
        )?
        .checked_add(OCR_PNG_ENCODE_FIXED_BYTES)
        .ok_or_else(|| {
            crate::extraction::image_decode::image_dimension_error(dimensions.0, dimensions.1, u64::MAX, u64::MAX)
        })?;
        output_bytes = output_bytes.checked_add(output).ok_or_else(|| {
            crate::extraction::image_decode::image_dimension_error(dimensions.0, dimensions.1, u64::MAX, u64::MAX)
        })?;
    }
    let additional_bytes = conversion_bytes.checked_add(output_bytes).ok_or_else(|| {
        crate::extraction::image_decode::image_dimension_error(dimensions.0, dimensions.1, u64::MAX, u64::MAX)
    })?;
    crate::extraction::image_decode::validate_image_live_bytes(
        dimensions.0,
        dimensions.1,
        source_bytes,
        additional_bytes,
        security_limits,
    )
}
/// Charge every page in `images` against `security_limits.max_content_size` on its own,
/// never as a running total.
///
/// `max_content_size` bounds what a single page may cost to render and encode; summing a
/// whole batch against it makes that per-image ceiling a function of the batch width
/// instead, so a wide batch is refused whole and every page in it is silently dropped. Three
/// call sites each grew their own copy of this per-page loop over
/// [`validate_png_encode_batch_peak`] (#1665, #1731, #1748): this is the one place left, so
/// a fourth route cannot reintroduce the sum by calling the batch-peak function directly with
/// a whole slice again. Each call is still independently `false` (sequential) accounting:
/// this validates what one page costs in isolation, not what the caller's own batch
/// concurrency adds on top -- callers whose encode step runs pages in parallel already bound
/// that width separately (by memory, by the thread budget), and this only ever guards the
/// per-page ceiling `max_content_size` actually documents. ~keep
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn validate_png_encode_pages_individually<'a>(
    images: impl IntoIterator<Item = &'a image::DynamicImage>,
    security_limits: &crate::extractors::security::SecurityLimits,
) -> crate::Result<()> {
    for image in images {
        validate_png_encode_batch_peak(std::iter::once(image), false, security_limits)?;
    }
    Ok(())
}

#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
mod png_encode_peak_tests;
/// #1577: `render_full_pdf_ocr_batch` / `render_selected_pages_from_document` must render at
/// the DPI `ImageExtractionConfig` requests, not the historical literal 150.
#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
mod render_dpi_tests;

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn clone_rgb_for_png_encode(
    image: &image::DynamicImage,
    security_limits: &crate::extractors::security::SecurityLimits,
) -> crate::Result<image::RgbImage> {
    crate::extraction::image_decode::validate_dynamic_image_additional_live_bytes(
        image,
        security_limits,
        3 + OCR_PNG_ENCODE_BYTES_PER_PIXEL,
        OCR_PNG_ENCODE_FIXED_BYTES,
    )?;
    Ok(image.to_rgb8())
}
/// Whether the rendered page raster carries essentially no ink.
///
/// Issue #1444: when xberg_native_pdf cannot draw a page's image XObjects it substitutes a
/// blank white bitmap, and a chatty backend then answers with a *description* of that
/// blankness rather than empty text — which [`is_page_text_blank`] accepts as content,
/// suppressing the XObject fallback. Looking at the pixels the backend was actually
/// given settles the question independently of what it said.
///
/// Returns `false` when `png_bytes` cannot be decoded: an undecodable raster is not
/// evidence of blankness, and the caller must not escalate on a guess.
///
/// [`is_page_text_blank`]: crate::extraction::blank_detection::is_page_text_blank
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn page_raster_is_blank(
    png_bytes: &[u8],
    security_limits: &crate::extractors::security::SecurityLimits,
) -> bool {
    let Ok(luma) =
        crate::extraction::image_decode::decode_standard_luma8_with_security_limits(png_bytes, security_limits)
    else {
        tracing::debug!("ink probe: page raster could not be decoded; not treating it as blank");
        return false;
    };
    let (width, height) = luma.dimensions();
    if width == 0 || height == 0 {
        return true;
    }

    let mut sampled: u64 = 0;
    let mut dark: u64 = 0;
    for y in (0..height).step_by(INK_PROBE_STRIDE as usize) {
        for x in (0..width).step_by(INK_PROBE_STRIDE as usize) {
            sampled += 1;
            if luma.get_pixel(x, y).0[0] < INK_LUMA_THRESHOLD {
                dark += 1;
            }
        }
    }

    (dark as f64) <= (sampled as f64) * INK_BLANK_MAX_DARK_RATIO
}
/// Whether this page should be treated as blank for the purposes of the image-XObject
/// OCR fallback.
///
/// Blank by text (the pre-existing [`is_page_text_blank`] rule) **or** blank by ink: a
/// short OCR answer over a raster with no ink on it is a description of a blank page,
/// not a transcription of one. The text test is free and runs first; the ink probe is
/// additionally gated on [`MAX_INK_PROBE_TEXT_CHARS`] so a genuinely transcribed page
/// never pays for a PNG decode.
///
/// [`is_page_text_blank`]: crate::extraction::blank_detection::is_page_text_blank
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) struct XObjectFallbackTrigger {
    pub(super) needs_fallback: bool,
    /// Whether the page raster itself came back blank -- the only evidence that the rasterizer
    /// could not draw the page's images. Reported separately because the retry also runs when a
    /// perfectly-drawn page simply yields no OCR text, and the warning must not claim a draw
    /// failure then (GH#1826). ~keep
    pub(super) draw_failed: bool,
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn page_needs_xobject_fallback(
    ocr_text: &str,
    page_png: &[u8],
    security_limits: &crate::extractors::security::SecurityLimits,
) -> XObjectFallbackTrigger {
    let text_blank = crate::extraction::blank_detection::is_page_text_blank(ocr_text);
    let non_whitespace = ocr_text.chars().filter(|c| !c.is_whitespace()).count();
    if !text_blank && non_whitespace > MAX_INK_PROBE_TEXT_CHARS {
        return XObjectFallbackTrigger {
            needs_fallback: false,
            draw_failed: false,
        };
    }
    let draw_failed = page_raster_is_blank(page_png, security_limits);
    XObjectFallbackTrigger {
        needs_fallback: text_blank || draw_failed,
        draw_failed,
    }
}
/// What one page's image-XObject OCR recovery attempt produced.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
#[derive(Debug)]
pub(super) struct XObjectRecoveryOutcome {
    /// Concatenated OCR text of every embedded image that yielded any; empty when none did.
    pub(super) text: String,
    /// `text` without the lines the recovered tables carry, which the page prints from those
    /// tables instead (#2014). The same as `text` when no image yielded a table.
    pub(super) text_outside_tables: String,
    /// How many image XObjects were handed to the backend.
    pub(super) attempted: usize,
    /// The recovered images themselves, provenance-tagged for the output's `images` array.
    pub(super) images: Vec<crate::types::ExtractedImage>,
    /// LLM usage emitted while retrying the embedded image bytes.
    pub(super) llm_usage: Vec<crate::types::LlmUsage>,
    /// Structured tables emitted by the recovery backend.
    pub(super) tables: Vec<crate::types::Table>,
    /// Formulas emitted by the recovery backend.
    pub(super) formulas: Vec<crate::types::Formula>,
    /// First preprocessing record in paint order, matching the per-page metadata surface.
    pub(super) image_preprocessing: Option<crate::types::ImagePreprocessingMetadata>,
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) async fn recover_image_xobjects(
    backend: &std::sync::Arc<dyn crate::plugins::OcrBackend>,
    fallback_images: &[crate::pdf::native::images::PageFallbackImage],
    page_idx: usize,
    total_pages: usize,
    ocr_config: &crate::core::config::OcrConfig,
    budget: &mut crate::extractors::security::SecurityBudget,
) -> crate::Result<XObjectRecoveryOutcome> {
    let mut outcome = XObjectRecoveryOutcome {
        text: String::new(),
        text_outside_tables: String::new(),
        attempted: fallback_images.len(),
        images: Vec::with_capacity(fallback_images.len()),
        llm_usage: Vec::new(),
        tables: Vec::new(),
        formulas: Vec::new(),
        image_preprocessing: None,
    };
    for (image_index, fallback) in fallback_images.iter().enumerate() {
        budget.step()?;
        let recovery =
            collect_xobject_recovery_result(backend, fallback, page_idx, ocr_config, budget, &mut outcome).await;
        crate::engine::seams::emit_ocr_page(page_idx + 1, total_pages, backend.name());
        recovery?;
        outcome.images.push(crate::types::ExtractedImage {
            data: fallback.bytes.clone(),
            format: std::borrow::Cow::Borrowed(fallback.format),
            image_index: image_index as u32,
            page_number: Some((page_idx + 1) as u32),
            source_path: Some(format!("xobject:page{}:{}", page_idx + 1, image_index)),
            description: Some(format!(
                "recovered from raw image XObject ({}) after the page rasterizer produced a blank page",
                fallback.recovery.as_str()
            )),
            ..Default::default()
        });
    }
    Ok(outcome)
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn account_xobject_structured_output(
    result: &crate::types::ExtractedDocument,
    retain_image_preprocessing: bool,
    budget: &mut crate::extractors::security::SecurityBudget,
) -> crate::Result<()> {
    for table in &result.tables {
        budget.account_text(table.markdown.len())?;
        if let Some(table_id) = &table.table_id {
            budget.account_text(table_id.len())?;
        }
        for column in table.columns.iter().flatten() {
            budget.account_text(column.len())?;
        }
        for row in &table.cells {
            budget.add_cells(row.len())?;
            for cell in row {
                budget.account_text(cell.len())?;
            }
        }
    }
    for formula in &result.formulas {
        budget.account_text(formula.latex.len())?;
    }
    for usage in result.llm_usage.iter().flatten() {
        budget.account_text(usage.model.len())?;
        budget.account_text(usage.source.len())?;
        if let Some(finish_reason) = &usage.finish_reason {
            budget.account_text(finish_reason.len())?;
        }
    }
    if retain_image_preprocessing && let Some(metadata) = &result.metadata.image_preprocessing {
        budget.account_text(metadata.resample_method.len())?;
        if let Some(resize_error) = &metadata.resize_error {
            budget.account_text(resize_error.len())?;
        }
    }
    Ok(())
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) async fn collect_xobject_recovery_result(
    backend: &std::sync::Arc<dyn crate::plugins::OcrBackend>,
    fallback: &crate::pdf::native::images::PageFallbackImage,
    page_idx: usize,
    ocr_config: &crate::core::config::OcrConfig,
    budget: &mut crate::extractors::security::SecurityBudget,
    outcome: &mut XObjectRecoveryOutcome,
) -> crate::Result<()> {
    let result = match backend.process_image(&fallback.bytes, ocr_config).await {
        Ok(result) => result,
        Err(error) => {
            tracing::debug!(
                page = page_idx,
                "force_ocr fallback: OCR of embedded image bytes failed: {error}"
            );
            return Ok(());
        }
    };
    append_xobject_text(&mut outcome.text, &result.content, budget)?;
    append_xobject_text(
        &mut outcome.text_outside_tables,
        &xobject_text_outside_tables(&result),
        budget,
    )?;
    account_xobject_structured_output(&result, outcome.image_preprocessing.is_none(), budget)?;
    outcome.llm_usage.extend(result.llm_usage.unwrap_or_default());
    if outcome.image_preprocessing.is_none() {
        outcome.image_preprocessing = result.metadata.image_preprocessing;
    }
    let page_number = (page_idx + 1) as u32;
    outcome.tables.extend(result.tables.into_iter().map(|mut table| {
        table.page_number = page_number;
        table
    }));
    outcome.formulas.extend(result.formulas.into_iter().map(|mut formula| {
        formula.page = Some(page_number);
        formula
    }));
    Ok(())
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn append_xobject_text(
    page_text: &mut String,
    image_text: &str,
    budget: &mut crate::extractors::security::SecurityBudget,
) -> crate::Result<()> {
    if image_text.trim().is_empty() {
        return Ok(());
    }
    let separator_len = usize::from(!page_text.is_empty()) * 2;
    budget.account_text(separator_len.saturating_add(image_text.len()))?;
    if separator_len != 0 {
        page_text.push_str("\n\n");
    }
    page_text.push_str(image_text);
    Ok(())
}
/// One embedded image's text without the lines its tables carry (#2014).
///
/// A backend that detects tables returns its page document with the lines those tables carry
/// already removed by the shared table-coverage filter, which compares line and table boxes in
/// the one pixel space the backend recognised them in (#1571). `content` keeps those lines,
/// because the standalone-image route prints no separate table. The PDF page prints the retry's
/// tables itself, so its paragraphs come from this text; built from `content`, they would print
/// each table twice. ~keep
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn xobject_text_outside_tables(result: &crate::types::ExtractedDocument) -> std::borrow::Cow<'_, str> {
    #[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
    if !result.tables.is_empty()
        && let Some(document) = result.ocr_internal_document.as_ref()
    {
        return std::borrow::Cow::Owned(ocr_document_text(document));
    }
    std::borrow::Cow::Borrowed(&result.content)
}
/// The text elements of an OCR page document as flat text: lines of one block on consecutive
/// lines, blocks apart by a blank line, the grouping the PDF paragraph builder merges on. ~keep
// ~keep hOCR, PaddleOCR, and Sceptre all emit this internal key, while the hOCR parser module is
// ~keep unavailable to the `ocr-wasm` feature that also compiles this renderer.
#[cfg(all(any(feature = "ocr", feature = "ocr-wasm"), feature = "pdf"))]
const OCR_BLOCK_ID_ATTRIBUTE: &str = "hocr_block_id";

#[cfg(all(any(feature = "ocr", feature = "ocr-wasm"), feature = "pdf"))]
fn ocr_document_text(document: &crate::types::internal::InternalDocument) -> String {
    let mut text = String::new();
    let mut previous_block: Option<&str> = None;
    for element in &document.elements {
        if matches!(element.kind, crate::types::internal::ElementKind::PageBreak) || element.text.trim().is_empty() {
            continue;
        }
        let block = element
            .attributes
            .as_ref()
            .and_then(|attributes| attributes.get(OCR_BLOCK_ID_ATTRIBUTE))
            .map(String::as_str);
        if !text.is_empty() {
            text.push_str(if block.is_some() && block == previous_block {
                "\n"
            } else {
                "\n\n"
            });
        }
        text.push_str(&element.text);
        previous_block = block;
    }
    text
}
/// OCR a page's embedded image XObjects directly, bypassing the whole-page rasterizer.
///
/// Used when the page render came back blank (see [`page_needs_xobject_fallback`]) but the
/// page does carry image XObjects the renderer could not paint (issue #1355/#1444).
///
/// A page that is one full-page scan retries with the scan hints its render gets: the
/// whole-image PSM and the known-scan signal that selects the default preprocessing, since its
/// embedded image is that same scan (#1907). The rotation and DPI hints describe the page
/// render, not the embedded image, so the retry does not carry them. ~keep
///
/// Returns `None` when the page has no recoverable image XObjects at all, so the caller can
/// tell "nothing to try" apart from "tried and got nothing" and avoid warning about a page
/// that was simply empty.
///
/// Provenance: each recovered image is tagged `source_path = "xobject:page{N}:{i}"` (`N`
/// 1-based, `i` the image's 0-based paint order on that page) with the recovery mode in
/// `description`. This reuses the existing `source_path` convention (DOCX/ODT record
/// `media/imageN.png` there) rather than adding a field to `ExtractedImage`, which would
/// require regenerating every language binding.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) async fn recover_page_text_from_image_xobjects(
    backend: &std::sync::Arc<dyn crate::plugins::OcrBackend>,
    render_doc: &xberg_native_pdf::PdfDocument,
    page_idx: usize,
    total_pages: usize,
    ocr_config: &crate::core::config::OcrConfig,
    budget: &mut crate::extractors::security::SecurityBudget,
) -> crate::Result<Option<XObjectRecoveryOutcome>> {
    let fallback_images = crate::pdf::native::images::page_ocr_fallback_image_bytes(render_doc, page_idx);
    if fallback_images.is_empty() {
        return Ok(None);
    }
    let whole_page_raster = crate::pdf::scan_detect::full_page_raster_density(render_doc, page_idx).is_some();
    let ocr_config =
        super::pipeline::ocr_config_with_page_rotation_hint(ocr_config, 0, None, whole_page_raster, false, false);
    recover_image_xobjects(backend, &fallback_images, page_idx, total_pages, &ocr_config, budget)
        .await
        .map(Some)
}
/// Lazily open — at most once — a PDF document used *only* by the image-XObject OCR
/// fallback.
///
/// The main `lazy_pdf_render_state` is deliberately not opened when the caller supplied
/// pre-rendered `images` (the layout-detection route), because its page-rotation and
/// points-per-pixel lookups are indexed differently there. The fallback needs nothing but
/// the page's XObject table, so it gets its own handle rather than perturbing those
/// lookups. Opening is deferred until a page actually comes back blank, so the common
/// case pays nothing.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn fallback_render_document<'a>(
    memo: &'a mut Option<Option<xberg_native_pdf::PdfDocument>>,
    content: Option<&[u8]>,
) -> Option<&'a xberg_native_pdf::PdfDocument> {
    memo.get_or_insert_with(|| {
        let bytes = content?;
        match open_pdf_for_full_ocr(bytes) {
            Ok((doc, _, _)) => Some(doc),
            Err(error) => {
                tracing::debug!("force_ocr fallback: reopening the PDF for XObject recovery failed: {error}");
                None
            }
        }
    })
    .as_ref()
}
/// Whether `page_idx` is a scan (one full-page raster), for the whole-image Tesseract PSM
/// hint (`ocr_config_with_page_rotation_hint`'s `whole_page_raster` argument).
///
/// `lazy_pdf_render_state` answers this directly when it is open (the non-layout OCR routes,
/// which render pages themselves). The layout-detection route hands in pre-rendered `images`
/// instead, so `lazy_pdf_render_state` is never opened there (see its `let` in
/// `extract_with_ocr_for_page`) -- without a fallback, that route always got `false` here,
/// silently keeping a scanned page's PSM at the engine default when layout detection was on,
/// even though the very same page under `force_ocr` alone got the whole-image PSM (#1828).
/// Falls back to the same lazily-opened, memoized handle the XObject recovery fallback uses
/// just below in `extract_with_ocr_for_page`, for the same reason: nothing here needs anything
/// `lazy_pdf_render_state` is indexed for beyond the page's own raster geometry. ~keep
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn whole_page_raster_for_ocr_page(
    lazy_pdf_render_state: Option<&(xberg_native_pdf::PdfDocument, usize, Vec<u32>)>,
    fallback_pdf_state: &mut Option<Option<xberg_native_pdf::PdfDocument>>,
    content: Option<&[u8]>,
    page_idx: usize,
) -> bool {
    ocr_page_document(lazy_pdf_render_state, fallback_pdf_state, content)
        .is_some_and(|doc| crate::pdf::scan_detect::full_page_raster_density(doc, page_idx).is_some())
}

/// Whether `page_idx` takes Tesseract block mode: automatic routing listed its one-indexed
/// `page_number` in `single_block_pages`, and the page carries no raster large enough to be a
/// scan ([`crate::pdf::scan_detect::carries_scan_raster`]), read from the same document
/// [`whole_page_raster_for_ocr_page`] reads. With no document at hand, the list decides.
///
/// Automatic routing asks for block mode on a page whose text layer has no usable character
/// map, because such a page is vector text that block mode keeps on its rows (#1896). A scan
/// with an unmapped text layer is still a scan: block mode loses the table reconstruction the
/// scan's own segmentation mode gives it, so such a page keeps that mode.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn single_block_for_ocr_page(
    single_block_pages: Option<&std::collections::HashSet<u32>>,
    page_number: usize,
    lazy_pdf_render_state: Option<&(xberg_native_pdf::PdfDocument, usize, Vec<u32>)>,
    fallback_pdf_state: &mut Option<Option<xberg_native_pdf::PdfDocument>>,
    content: Option<&[u8]>,
    page_idx: usize,
) -> bool {
    let listed = single_block_pages.is_some_and(|pages| u32::try_from(page_number).is_ok_and(|n| pages.contains(&n)));
    listed
        && !ocr_page_document(lazy_pdf_render_state, fallback_pdf_state, content)
            .is_some_and(|doc| crate::pdf::scan_detect::carries_scan_raster(doc, page_idx))
}

/// The document the page-classification helpers above read: the render state when it is open,
/// else the lazily opened fallback handle.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn ocr_page_document<'a>(
    lazy_pdf_render_state: Option<&'a (xberg_native_pdf::PdfDocument, usize, Vec<u32>)>,
    fallback_pdf_state: &'a mut Option<Option<xberg_native_pdf::PdfDocument>>,
    content: Option<&[u8]>,
) -> Option<&'a xberg_native_pdf::PdfDocument> {
    match lazy_pdf_render_state {
        Some((doc, _, _)) => Some(doc),
        None => fallback_render_document(fallback_pdf_state, content),
    }
}
/// The DPI to render `page_idx` at for OCR.
///
/// Thin wrapper over [`crate::image::dpi::pdf_ocr_render_dpi`], which also backs the
/// layout-detection route's OCR render (`extractors::pdf::layout_runner`) so both routes make
/// the same scan-density decision (#1828). See that function's doc comment for the decision
/// itself.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn ocr_page_render_dpi(
    doc: &xberg_native_pdf::PdfDocument,
    page_idx: usize,
    images_config: Option<&crate::core::config::ImageExtractionConfig>,
    security_limits: &crate::extractors::security::SecurityLimits,
) -> i32 {
    crate::image::dpi::pdf_ocr_render_dpi(doc, page_idx, images_config, security_limits)
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn render_one_full_pdf_ocr_page(
    doc: &xberg_native_pdf::PdfDocument,
    page_rotations: &[u32],
    page_idx: usize,
    security_limits: &crate::extractors::security::SecurityLimits,
    images_config: Option<&crate::core::config::ImageExtractionConfig>,
) -> crate::Result<EncodedPage> {
    #[cfg(test)]
    record_render_thread();
    // GH#1786 + GH#1796: the scan-density choice lives in `ocr_page_render_dpi`, and it is made
    // per page HERE rather than in the batch loop above, so the parallel render keeps it. Doing
    // it in the caller would have serialized the batch again. ~keep
    let render_dpi = ocr_page_render_dpi(doc, page_idx, images_config, security_limits);
    let rendered =
        crate::pdf::render::render_page_with_safeguards(doc, page_idx, render_dpi.max(1) as u32).map_err(|e| {
            crate::XbergError::Parsing {
                message: format!("Failed to render page {} for OCR: {:?}", page_idx, e),
                source: None,
            }
        })?;
    let rotation = page_rotations.get(page_idx).copied().unwrap_or(0);
    let (data, width, height) = crate::pdf::render::normalize_rendered_page_for_ocr_with_security_limits(
        rendered.data,
        rendered.width,
        rendered.height,
        rotation,
        security_limits,
    )?;
    Ok((page_idx, std::sync::Arc::new(data), width, height))
}
/// Render every page in `page_range` for the `force_ocr` (whole-document) route, in parallel
/// across the configured thread budget -- the same mechanism
/// `render_selected_pages_from_document` uses for the sibling `force_ocr_pages` route (#1666).
/// Before this fix, `force_ocr` rendered each batch with a plain sequential loop regardless of
/// `concurrency.max_threads`, leaving rasterization a floor no thread count could lower on this
/// route even after #1666 fixed the sibling (#1796). `.par_iter().map(...).collect()` preserves
/// `page_range`'s order, matching the previous sequential loop's output order exactly. ~keep
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn render_full_pdf_ocr_batch(
    doc: &xberg_native_pdf::PdfDocument,
    page_rotations: &[u32],
    page_range: std::ops::Range<usize>,
    security_limits: &crate::extractors::security::SecurityLimits,
    images_config: Option<&crate::core::config::ImageExtractionConfig>,
) -> crate::Result<Vec<EncodedPage>> {
    // rayon's work-stealing pool needs OS threads; wasm32 has none, so this falls back to a
    // sequential iterator there, matching `render_selected_pages_from_document`. ~keep
    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    {
        crate::pdf::render::par_render_pages_collecting_warnings(page_range.collect(), |page_idx| {
            render_one_full_pdf_ocr_page(doc, page_rotations, page_idx, security_limits, images_config)
        })
    }
    #[cfg(any(not(feature = "tokio-runtime"), target_arch = "wasm32"))]
    {
        page_range
            .map(|page_idx| render_one_full_pdf_ocr_page(doc, page_rotations, page_idx, security_limits, images_config))
            .collect()
    }
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn valid_page_indices(page_indices: &[usize], page_count: usize) -> Vec<usize> {
    page_indices
        .iter()
        .copied()
        .filter(|&idx| {
            if idx < page_count {
                true
            } else {
                tracing::warn!(
                    page = idx + 1,
                    page_count,
                    "force_ocr_pages: page {} is out of range (document has {} pages), skipping",
                    idx + 1,
                    page_count
                );
                false
            }
        })
        .collect()
}
/// Render one page: the per-page body `render_selected_pages_from_document` runs, either in
/// parallel across the thread pool or sequentially on `wasm32` (which has no OS threads for
/// rayon's work-stealing pool to use).
///
/// #1690/#1747: `RENDER_CALL_THREAD_NAMES` below is test-only instrumentation (compiled
/// under `cfg(test)` plus the gates of its only users, so it never reaches a release build
/// and is never dead under a feature leg that lacks those users) that lets a test observe
/// which OS threads actually executed page renders -- the mechanism a regression here
/// breaks -- rather than inferring parallelism from wall-clock duration, which flakes under
/// shared-box load. Records thread NAMES rather than raw `ThreadId`s: the set is
/// process-global, so a guard must be able to tell its own pool's threads apart from any
/// other test's, e.g. via a caller-chosen name prefix -- otherwise a concurrently running
/// extraction can inflate the set and let a sequential regression here read as parallel
/// (the same defect class `pdf::native::images::PAGE_CALL_THREAD_NAMES` fixed against
/// #1732, and `core/config/concurrency.rs` against #215). See
/// `parallel_render_dispatches_across_more_than_one_thread` in `ocr/tests.rs`. ~keep
#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) static RENDER_CALL_THREAD_NAMES: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<String>>> =
    std::sync::OnceLock::new();
#[cfg(all(test, feature = "ocr", feature = "pdf"))]
pub(super) fn clear_render_call_thread_ids() {
    RENDER_CALL_THREAD_NAMES
        .get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()))
        .lock()
        .unwrap()
        .clear();
}
#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn record_render_thread() {
    let current = std::thread::current();
    let name = current.name().unwrap_or("<unnamed>").to_owned();
    RENDER_CALL_THREAD_NAMES
        .get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()))
        .lock()
        .unwrap()
        .insert(name);
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn render_one_selected_page(
    doc: &xberg_native_pdf::PdfDocument,
    page_rotations: &[u32],
    idx: usize,
    security_limits: &crate::extractors::security::SecurityLimits,
    images_config: Option<&crate::core::config::ImageExtractionConfig>,
) -> crate::Result<(usize, image::DynamicImage)> {
    #[cfg(test)]
    record_render_thread();
    let render_dpi = ocr_page_render_dpi(doc, idx, images_config, security_limits);
    let rendered =
        crate::pdf::render::render_page_with_safeguards(doc, idx, render_dpi.max(1) as u32).map_err(|e| {
            crate::XbergError::Parsing {
                message: format!("Failed to render PDF page {}: {}", idx + 1, e),
                source: None,
            }
        })?;
    let rotation = page_rotations.get(idx).copied().unwrap_or(0);
    let (data, _, _) = crate::pdf::render::normalize_rendered_page_for_ocr_with_security_limits(
        rendered.data,
        rendered.width,
        rendered.height,
        rotation,
        security_limits,
    )?;
    let img = crate::extraction::image_decode::decode_standard_image_with_security_limits(&data, security_limits)
        .map_err(|e| crate::XbergError::Parsing {
            message: format!("Failed to decode rendered page {}: {}", idx + 1, e),
            source: None,
        })?;
    Ok((idx, img))
}

/// Render every page in `page_indices`, in parallel across the configured thread budget.
///
/// PDF page rasterization (the pixels-from-vectors work inside `render_page_with_safeguards`)
/// is CPU-bound and, per page, independent of every other page: `xberg_native_pdf::PdfDocument`
/// is documented `Send + Sync` for exactly this reason (its own doc comment: "warm cache hits
/// stay fully parallel"). Rendering used to run in a plain sequential loop regardless of the
/// thread budget, while the sibling PNG-encode step a few lines away in `pipeline.rs` already
/// used `.par_iter()` -- so widening `concurrency.max_threads` only ever widened the OCR
/// recognition and encode stages, leaving rasterization as a floor no thread count could lower
/// (issue #1666). `.par_iter().map(...).collect()` preserves `page_indices`' order, matching
/// the previous sequential loop's output order exactly.
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn render_selected_pages_from_document(
    doc: &xberg_native_pdf::PdfDocument,
    page_rotations: &[u32],
    page_indices: &[usize],
    security_limits: &crate::extractors::security::SecurityLimits,
    images_config: Option<&crate::core::config::ImageExtractionConfig>,
) -> PageRenderOutcomes {
    // rayon's work-stealing pool needs OS threads; wasm32 has none, so this falls back to a
    // sequential iterator there, matching the same gate used for the PNG-encode parallel path
    // in `pipeline.rs`. ~keep
    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    {
        let outcomes = crate::pdf::render::par_render_pages_collecting_results(page_indices.to_vec(), |idx| {
            render_one_selected_page(doc, page_rotations, idx, security_limits, images_config)
        });
        partition_page_render_outcomes(outcomes)
    }
    #[cfg(any(not(feature = "tokio-runtime"), target_arch = "wasm32"))]
    {
        let outcomes = page_indices
            .iter()
            .map(|&idx| {
                (
                    idx,
                    render_one_selected_page(doc, page_rotations, idx, security_limits, images_config),
                )
            })
            .collect();
        partition_page_render_outcomes(outcomes)
    }
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
fn partition_page_render_outcomes(
    outcomes: Vec<(usize, crate::Result<(usize, image::DynamicImage)>)>,
) -> PageRenderOutcomes {
    let mut pages = Vec::with_capacity(outcomes.len());
    let mut failures = Vec::new();
    for (page_index, outcome) in outcomes {
        match outcome {
            Ok(page) => pages.push(page),
            Err(error) => failures.push((page_index, error)),
        }
    }
    (pages, failures)
}
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(super) fn share_rendered_page_images(
    page_images: Vec<(usize, image::DynamicImage)>,
) -> Vec<(usize, std::sync::Arc<image::DynamicImage>)> {
    page_images
        .into_iter()
        .map(|(page_idx, image)| (page_idx, std::sync::Arc::new(image)))
        .collect()
}
#[cfg(all(test, feature = "ocr", feature = "pdf"))]
mod xobject_text_tests;
