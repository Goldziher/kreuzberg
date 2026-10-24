//! Layout detection runner for PDF pages.
//!
//! Renders pages of a PDF document in chunks, runs the layout engine on each
//! chunk in sequence, and converts pixel-space detections to PDF
//! coordinate–space [`PageLayoutResult`] values.
//!
//! Chunked rendering+detection keeps the inference working set proportional to
//! `LAYOUT_BATCH_CHUNK_SIZE` images plus the ONNX batch tensor, rather than
//! requiring the whole document's rasterised frames to be live simultaneously.
//! Each chunk is charged against `max_content_size` on its own: the rasters of
//! earlier chunks are already-assembled output, so charging them too would turn
//! a byte limit into a page limit (GH#1721).
//!
//! The resulting images, page metadata, layout hints, and raw detections feed
//! both native markdown structure recovery and OCR layout assembly.

/// Maximum number of pages sent to the layout model in a single ONNX call.
///
/// Each chunk requires:  chunk_size × 3 × 640 × 640 × 4 bytes ≈ 4.9 MB × chunk_size
/// for the batch tensor.  8 pages ≈ 39 MB vs 214 pages ≈ 1.05 GB without chunking.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
const LAYOUT_BATCH_CHUNK_SIZE: usize = 8;

/// Small white raster used when one page cannot be rendered.
///
/// Keeping a page-aligned placeholder lets downstream OCR reuse every other
/// rendered page without retrying the entire document. The matching detection
/// result is empty and uses these same dimensions.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
const FAILED_RENDER_PLACEHOLDER_SIDE: u32 = 64;

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
use crate::{
    Result, XbergError,
    core::config::{
        ExtractionConfig,
        acceleration::{AccelerationConfig, ExecutionProviderType},
        layout::{LayoutDetectionConfig, LayoutStrategy},
    },
    extractors::pdf::layout_hints::pixel_detection_to_layout_hints_pdf_space,
    pdf::layout_gate::PageGateDecision,
    pdf::structure::types::{LayoutHint, PageLayoutResult},
};

/// How [`run_layout_for_pdf_pages`] treats pages the `Auto` gate skips.
///
/// The markdown path only needs rasters for pages the model inspects, so it
/// skips rendering entirely. The OCR path reuses the layout pass's rasters as
/// OCR input images, so gated pages must still render; only inference is
/// skipped there.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GatedPageHandling {
    /// Skip both the raster render and model inference for gated pages.
    ///
    /// The page-aligned output still carries a small white placeholder image
    /// for gated pages (see [`render_failure_placeholder`]); downstream
    /// markdown consumers never read it because gated pages have no hints.
    SkipRender,
    /// Render the page raster but skip model inference for gated pages.
    ///
    /// Only the OCR entry point constructs this, so builds without an OCR
    /// feature see the variant as unused.
    #[cfg_attr(not(any(feature = "ocr", feature = "ocr-pipeline")), allow(dead_code))]
    RenderWithoutInference,
}

/// Page-aligned layout pass data: `(images, results, hints_per_page,
/// detections_per_page)`, one entry per page at every index.
///
/// - `images[i]` is the rendered RGB image for page `i`, or a small white
///   placeholder when the page failed to render or the `Auto` gate skipped it
///   on the markdown path.
/// - `results[i]` holds per-region detection metadata in PDF coordinate space.
/// - `hints_per_page[i]` holds the layout hints for page `i` (empty for
///   render-failed, gate-skipped, and no-detection pages).
/// - `detections_per_page[i]` preserves pixel-space detections for OCR layout
///   assembly (empty for render-failed and gate-skipped pages).
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
type LayoutForMarkdownOutput = (
    Vec<image::RgbImage>,
    Vec<PageLayoutResult>,
    Vec<Vec<LayoutHint>>,
    Vec<crate::layout::DetectionResult>,
);

/// Layout pass outcome plus the `Auto` gate's per-page decisions.
///
/// `data` is `None` when the gate skipped every page (the caller proceeds
/// exactly as if layout were off). `gate_decisions` is `None` under strategy
/// `Always` and always present under `Auto`, including the all-gated case, so
/// gate behavior stays auditable from extraction metadata.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
pub(super) struct LayoutRunOutput {
    pub data: Option<LayoutForMarkdownOutput>,
    pub gate_decisions: Option<Vec<PageGateDecision>>,
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
struct RenderedLayoutPage {
    page_index: usize,
    page_width_pts: f32,
    page_height_pts: f32,
    rotation: u32,
    image: Option<image::RgbImage>,
    /// `false` when the `Auto` gate skipped this page: it stays out of the
    /// ONNX batch and keeps an empty detection, exactly like a page where the
    /// model ran and found nothing.
    run_inference: bool,
    /// `Some(reason)` when `image` is `None` because rendering, OCR rotation
    /// normalization, or PNG decode failed for this page (#196). `None` for
    /// gate-skipped pages, which have no render attempt and no failure.
    render_failure: Option<String>,
    /// The CropBox in raster pixels when it trims the rendered MediaBox; the model sees only it.
    inference_crop: Option<PixelRect>,
}

/// A raster rectangle `(x, y, width, height)` in pixels, y down.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
type PixelRect = (u32, u32, u32, u32);

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
struct AssembledLayoutPage {
    image: image::RgbImage,
    result: PageLayoutResult,
    hints: Vec<LayoutHint>,
    detection: crate::layout::DetectionResult,
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
const LAYOUT_INFERENCE_RUN_ERROR_PREFIX: &str =
    "layout runner: batch detection failed: inference error: inference run failed:";

/// A successful layout pass and the effective execution provider used for
/// downstream layout-aware models.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
#[derive(Debug)]
pub(super) struct LayoutAttempt<T> {
    pub(super) output: T,
    pub(super) acceleration_override: Option<AccelerationConfig>,
    pub(super) warning: Option<crate::types::ProcessingWarning>,
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn run_layout_with_auto_cpu_retry<T>(
    layout_config: &LayoutDetectionConfig,
    execution_provider_overridden: bool,
    initial_acceleration_override: Option<AccelerationConfig>,
    mut run_once: impl FnMut(&LayoutDetectionConfig) -> Result<T>,
) -> Result<LayoutAttempt<T>> {
    let mut initial_config = layout_config.clone();
    if let Some(acceleration) = initial_acceleration_override.clone() {
        initial_config.acceleration = Some(acceleration);
    }
    let initial_error = match run_once(&initial_config) {
        Ok(output) => {
            return Ok(LayoutAttempt {
                output,
                acceleration_override: initial_acceleration_override,
                warning: None,
            });
        }
        Err(error) => error,
    };
    let uses_auto_provider = !execution_provider_overridden
        && layout_config
            .acceleration
            .as_ref()
            .is_none_or(|acceleration| acceleration.provider == ExecutionProviderType::Auto);
    let inference_run_failed = matches!(
        &initial_error,
        XbergError::Other(message) if message.starts_with(LAYOUT_INFERENCE_RUN_ERROR_PREFIX)
    );
    let already_uses_cpu = initial_acceleration_override
        .as_ref()
        .is_some_and(|acceleration| acceleration.provider == ExecutionProviderType::Cpu);
    if !uses_auto_provider || !inference_run_failed || already_uses_cpu {
        return Err(initial_error);
    }

    tracing::warn!(
        error = %initial_error,
        "layout runner: automatic execution provider failed during inference, retrying once with CPU"
    );
    let warning = layout_cpu_fallback_warning(&initial_error);
    let mut cpu_config = layout_config.clone();
    cpu_config.acceleration = Some(AccelerationConfig {
        provider: ExecutionProviderType::Cpu,
        ..Default::default()
    });
    run_once(&cpu_config)
        .map(|output| LayoutAttempt {
            output,
            acceleration_override: cpu_config.acceleration,
            warning: Some(warning),
        })
        .map_err(|cpu_error| {
            XbergError::Other(format!(
                "layout runner: CPU retry failed after automatic-provider inference run failure; \
                 initial error: {initial_error}; CPU retry error: {cpu_error}"
            ))
        })
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn rtdetr_acceleration_override(
    layout_config: &LayoutDetectionConfig,
    execution_provider_overridden: bool,
) -> Option<AccelerationConfig> {
    if execution_provider_overridden {
        return None;
    }
    let effective = crate::layout::models::rtdetr::effective_acceleration(layout_config.acceleration.as_ref());
    if effective.as_ref() == layout_config.acceleration.as_ref() {
        None
    } else {
        effective
    }
}

/// Combines the CPU-retry recovery warning with the page-render-failure
/// warning (#196) into the single slot callers observe. Both can fire
/// together (an automatic-provider retry that also hit unrenderable pages),
/// so their messages are concatenated rather than one silently discarding
/// the other.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn combine_layout_warnings(
    outer: Option<crate::types::ProcessingWarning>,
    inner: Option<crate::types::ProcessingWarning>,
) -> Option<crate::types::ProcessingWarning> {
    match (outer, inner) {
        (Some(outer), Some(inner)) => Some(crate::types::ProcessingWarning {
            source: std::borrow::Cow::Borrowed("layout"),
            message: std::borrow::Cow::Owned(format!("{}; {}", outer.message, inner.message)),
        }),
        (Some(warning), None) | (None, Some(warning)) => Some(warning),
        (None, None) => None,
    }
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn merge_render_warning(
    attempt: LayoutAttempt<(LayoutRunOutput, Option<crate::types::ProcessingWarning>)>,
) -> LayoutAttempt<LayoutRunOutput> {
    let LayoutAttempt {
        output: (output, render_warning),
        acceleration_override,
        warning,
    } = attempt;
    LayoutAttempt {
        output,
        acceleration_override,
        warning: combine_layout_warnings(warning, render_warning),
    }
}

/// Runs the layout pass and returns it paired with any `xberg_native_pdf`
/// glyph-drop warnings captured while rendering its pages (#353).
///
/// On the `tokio-runtime` path the whole pass — including every render call
/// — executes inside `spawn_blocking`, on a blocking-pool OS thread distinct
/// from the extracting task's thread. [`crate::pdf::render::take_xberg_native_pdf_render_warnings`]
/// drains a *thread-local* buffer, so it must be called from that same
/// blocking-pool thread before the closure returns; nothing else ever visits
/// that thread again to drain it later. The drained warnings are threaded
/// back out through the return value instead, so the caller can merge them
/// into the document's `processing_warnings` the same way the same-thread
/// drain in `extractors::pdf::mod` already does for OCR rasterization.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
async fn run_layout_for_pdf_pages_async(
    content: &[u8],
    layout_config: &LayoutDetectionConfig,
    thread_budget: usize,
    gated_handling: GatedPageHandling,
    security_limits: &crate::extractors::security::SecurityLimits,
    images_config: Option<&crate::core::config::ImageExtractionConfig>,
) -> Result<(LayoutAttempt<LayoutRunOutput>, Vec<crate::types::ProcessingWarning>)> {
    #[cfg(feature = "tokio-runtime")]
    {
        let owned_content = content.to_vec();
        let owned_config = layout_config.clone();
        let owned_limits = security_limits.clone();
        let owned_images_config = images_config.cloned();
        let (result, glyph_drop_warnings) = tokio::task::spawn_blocking(move || {
            let execution_provider_overridden = crate::ort_discovery::execution_provider_override().is_some();
            let acceleration_override = rtdetr_acceleration_override(&owned_config, execution_provider_overridden);
            let result = run_layout_with_auto_cpu_retry(
                &owned_config,
                execution_provider_overridden,
                acceleration_override,
                |attempt_config| {
                    run_layout_for_pdf_pages_with_security_limits(
                        &owned_content,
                        attempt_config,
                        thread_budget,
                        gated_handling,
                        &owned_limits,
                        owned_images_config.as_ref(),
                    )
                },
            )
            .map(merge_render_warning);
            // Drain this blocking-pool thread's captured glyph-drop warnings
            // before returning: still on the thread that rendered the pages,
            // which is the only place `ENGINE_PENDING_WARNINGS` (thread-local)
            // can be observed from. See the function doc comment above.
            let glyph_drop_warnings = crate::pdf::render::take_xberg_native_pdf_render_warnings();
            (result, glyph_drop_warnings)
        })
        .await
        .map_err(|error| XbergError::Other(format!("layout runner task failed: {error}")))?;
        result.map(|attempt| (attempt, glyph_drop_warnings))
    }
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn validate_batch_cardinality(expected: usize, actual: usize) -> Result<()> {
    if actual == expected {
        return Ok(());
    }

    Err(XbergError::Other(format!(
        "layout runner: batch detection returned {actual} results for {expected} rendered pages"
    )))
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn render_failure_placeholder() -> image::RgbImage {
    image::RgbImage::from_pixel(
        FAILED_RENDER_PLACEHOLDER_SIDE,
        FAILED_RENDER_PLACEHOLDER_SIDE,
        image::Rgb([u8::MAX; 3]),
    )
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn displayed_page_dimensions(width: f32, height: f32, rotation_degrees: u32) -> (f32, f32) {
    match rotation_degrees % 360 {
        90 | 270 => (height, width),
        _ => (width, height),
    }
}

/// Run the `Auto` gate over every page, or `None` under strategy `Always`.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn auto_gate_decisions(
    doc: &xberg_native_pdf::PdfDocument,
    layout_config: &LayoutDetectionConfig,
    page_count: usize,
) -> Option<Vec<PageGateDecision>> {
    match layout_config.strategy {
        LayoutStrategy::Always => None,
        LayoutStrategy::Auto => {
            let decisions = crate::pdf::layout_gate::decide_pages(doc, page_count);
            let selected = decisions.iter().filter(|decision| decision.run_layout).count();
            tracing::info!(
                pages = page_count,
                selected,
                skipped = page_count - selected,
                "layout gate: auto strategy page selection"
            );
            Some(decisions)
        }
    }
}

/// Whether the `Auto` gate selected `page_index` for the model.
///
/// `None` gate decisions (strategy `Always`) select every page; a page index
/// beyond the decision vector also runs, so a gate bug can only ever cost
/// speed, never coverage.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn gate_selects_page(gate_decisions: Option<&[PageGateDecision]>, page_index: usize) -> bool {
    gate_decisions.is_none_or(|decisions| decisions.get(page_index).is_none_or(|decision| decision.run_layout))
}

/// The two render-budget inputs that always travel together: the hard safety ceiling and the
/// caller's DPI preferences. Bundled because threading #1577's `images_config` beside
/// `security_limits` pushed these two signatures past clippy's argument limit, and because
/// passing one without the other is never correct -- a caller that clamps dimensions must also
/// honour the configured DPI, or the clamp is computed against a resolution nothing renders at.
/// Only these two functions take it; the others that carry the same pair are still under the
/// limit and were left alone. ~keep
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
#[derive(Clone, Copy)]
struct RenderBudget<'a> {
    security_limits: &'a crate::extractors::security::SecurityLimits,
    images_config: Option<&'a crate::core::config::ImageExtractionConfig>,
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn render_layout_chunk(
    doc: &xberg_native_pdf::PdfDocument,
    page_rotations: &[u32],
    chunk_start: usize,
    chunk_end: usize,
    gate_decisions: Option<&[PageGateDecision]>,
    gated_handling: GatedPageHandling,
    budget: RenderBudget<'_>,
) -> Vec<RenderedLayoutPage> {
    (chunk_start..chunk_end)
        .map(|page_index| {
            let (media_width, media_height) = doc
                .get_page_media_box(page_index)
                .map(|(llx, lly, urx, ury)| ((urx - llx).abs(), (ury - lly).abs()))
                .unwrap_or((612.0, 792.0));
            let rotation = page_rotations.get(page_index).copied().unwrap_or(0);
            let normalize_for_ocr = gated_handling == GatedPageHandling::RenderWithoutInference;
            let (page_width_pts, page_height_pts) = if normalize_for_ocr {
                (media_width, media_height)
            } else {
                displayed_page_dimensions(media_width, media_height, rotation)
            };
            let run_inference = gate_selects_page(gate_decisions, page_index);
            let skip_render = !run_inference && gated_handling == GatedPageHandling::SkipRender;
            let (image, render_failure) = if skip_render {
                (None, None)
            } else {
                match render_layout_page(
                    doc,
                    page_index,
                    page_width_pts,
                    page_height_pts,
                    rotation,
                    normalize_for_ocr,
                    budget,
                ) {
                    Ok(image) => (Some(image), None),
                    Err(reason) => (None, Some(reason)),
                }
            };

            let inference_crop = if normalize_for_ocr || rotation % 360 != 0 {
                None
            } else {
                image.as_ref().and_then(|image| visible_crop_px(doc, page_index, image))
            };
            RenderedLayoutPage {
                page_index,
                page_width_pts,
                page_height_pts,
                rotation,
                image,
                run_inference,
                render_failure,
                inference_crop,
            }
        })
        .collect()
}

/// The CropBox of an unrotated page in raster pixels, when it trims the rendered MediaBox, so the
/// model does not see bleed and printer's marks around a shrunken page.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn visible_crop_px(
    doc: &xberg_native_pdf::PdfDocument,
    page_index: usize,
    image: &image::RgbImage,
) -> Option<PixelRect> {
    let info = doc.get_page_info(page_index).ok()?;
    visible_crop_rect(info.media_box, info.crop_box?, image.dimensions())
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn visible_crop_rect(
    media: xberg_native_pdf::geometry::Rect,
    crop: xberg_native_pdf::geometry::Rect,
    (width, height): (u32, u32),
) -> Option<PixelRect> {
    // `Rect::from_points` keeps a reversed box's negative extent.
    let span = |origin: f32, extent: f32| (origin.min(origin + extent), origin.max(origin + extent));
    let ((m_x0, m_x1), (m_y0, m_y1)) = (span(media.x, media.width), span(media.y, media.height));
    let ((c_x0, c_x1), (c_y0, c_y1)) = (span(crop.x, crop.width), span(crop.y, crop.height));
    let (c_x0, c_y0, c_x1, c_y1) = (c_x0.max(m_x0), c_y0.max(m_y0), c_x1.min(m_x1), c_y1.min(m_y1));
    let (media_w, media_h) = (m_x1 - m_x0, m_y1 - m_y0);
    if media_w <= 0.0 || media_h <= 0.0 || c_x1 <= c_x0 || c_y1 <= c_y0 {
        return None;
    }
    // A CropBox that keeps over 98% of both axes is not worth a copy.
    if (c_x1 - c_x0) / media_w > 0.98 && (c_y1 - c_y0) / media_h > 0.98 {
        return None;
    }
    let sx = width as f32 / media_w;
    let sy = height as f32 / media_h;
    let x = ((c_x0 - m_x0) * sx).round().max(0.0) as u32;
    let y = ((m_y1 - c_y1) * sy).round().max(0.0) as u32;
    let w = (((c_x1 - c_x0) * sx).round() as u32).min(width.saturating_sub(x));
    let h = (((c_y1 - c_y0) * sy).round() as u32).min(height.saturating_sub(y));
    (w > 0 && h > 0).then_some((x, y, w, h))
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn inference_crop_image(page: &RenderedLayoutPage) -> Option<image::RgbImage> {
    let (x, y, w, h) = page.inference_crop?;
    let image = page.image.as_ref()?;
    Some(image::imageops::crop_imm(image, x, y, w, h).to_image())
}

/// Decoded bytes of the inference crops `inference_crop_image` would copy for these pages.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn inference_crop_bytes(pages: &[RenderedLayoutPage], positions: &[usize]) -> Result<u64> {
    positions.iter().try_fold(0_u64, |total, &position| {
        let page = &pages[position];
        let Some((_, _, w, h)) = page.inference_crop.filter(|_| page.image.is_some()) else {
            return Ok(total);
        };
        let bytes = crate::extraction::image_decode::decoded_byte_count(w, h, 3)?;
        checked_add_live(total, bytes, w, h)
    })
}

/// The engine's batch capacity, lowered until a batch that size fits with every page charged the
/// chunk's largest inference crop. Only one batch's crops are live at a time, so charging the
/// whole chunk's crops at once refuses chunks that split into valid batches.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn inference_batch_capacity(
    pages: &[RenderedLayoutPage],
    rendered_positions: &[usize],
    current_live_bytes: u64,
    (width, height): (u32, u32),
    security_limits: &crate::extractors::security::SecurityLimits,
) -> Result<usize> {
    let capacity = crate::layout::engine::layout_inference_batch_capacity(
        width,
        height,
        current_live_bytes,
        rendered_positions.len(),
        security_limits,
    )?;
    let largest_crop = rendered_positions.iter().try_fold(0_u64, |largest, &position| {
        inference_crop_bytes(pages, &[position]).map(|bytes| largest.max(bytes))
    })?;
    if largest_crop == 0 {
        return Ok(capacity);
    }
    let fits = |batch_len: usize| {
        u64::try_from(batch_len)
            .ok()
            .and_then(|count| largest_crop.checked_mul(count))
            .and_then(|crops| current_live_bytes.checked_add(crops))
            .is_some_and(|live| {
                crate::layout::engine::validate_layout_inference_peak(width, height, live, batch_len, security_limits)
                    .is_ok()
            })
    };
    // One when nothing fits, so the per-batch check reports the page that does not.
    Ok((1..=capacity).rev().find(|&batch_len| fits(batch_len)).unwrap_or(1))
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn checked_add_live(a: u64, b: u64, width: u32, height: u32) -> Result<u64> {
    a.checked_add(b)
        .ok_or_else(|| crate::extraction::image_decode::image_dimension_error(width, height, u64::MAX, u64::MAX))
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn shift_into_page_frame(page: &RenderedLayoutPage, detection: &mut crate::layout::DetectionResult) {
    let (Some((x, y, _, _)), Some(image)) = (page.inference_crop, page.image.as_ref()) else {
        return;
    };
    for d in &mut detection.detections {
        d.bbox.x1 += x as f32;
        d.bbox.x2 += x as f32;
        d.bbox.y1 += y as f32;
        d.bbox.y2 += y as f32;
    }
    (detection.page_width, detection.page_height) = image.dimensions();
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn render_layout_page(
    doc: &xberg_native_pdf::PdfDocument,
    page_index: usize,
    page_width_pts: f32,
    page_height_pts: f32,
    rotation: u32,
    normalize_for_ocr: bool,
    budget: RenderBudget<'_>,
) -> std::result::Result<image::RgbImage, String> {
    // The page's own MediaBox dimensions (pre-rotation, matching `get_page_dimensions_pt`),
    // not the possibly-swapped `page_width_pts`/`page_height_pts` above, which flip for a
    // 90/270-rotated page under `normalize_for_ocr == false` -- `effective_pdf_render_dpi`
    // only needs an aspect-insensitive area budget from `max_image_dimension`.
    let (media_width_pt, media_height_pt) = crate::pdf::render::get_page_dimensions_pt(doc, page_index);
    // `normalize_for_ocr` marks this render as OCR input (see `GatedPageHandling`), so it
    // must make the same scan-density render-DPI decision the non-layout OCR routes make via
    // `crate::image::dpi::pdf_ocr_render_dpi` -- otherwise a scanned page renders at a
    // different DPI depending on whether layout detection is on (#1828). The plain
    // `effective_pdf_render_dpi` stays in place for the markdown-structure render, which is
    // never OCR input and must not change. ~keep
    let render_dpi = if normalize_for_ocr {
        crate::image::dpi::pdf_ocr_render_dpi(doc, page_index, budget.images_config, budget.security_limits)
    } else {
        crate::image::dpi::effective_pdf_render_dpi(
            budget.images_config,
            f64::from(media_width_pt),
            f64::from(media_height_pt),
        )
    };
    let rendered = crate::pdf::render::render_page_with_safeguards(doc, page_index, render_dpi.max(1) as u32).map_err(
        |error| {
            tracing::warn!(
                page = page_index + 1,
                page_width_pts,
                page_height_pts,
                error = %error,
                "layout runner: skipping page with render failure, returning empty detections"
            );
            format!("page {} failed to render: {error}", page_index + 1)
        },
    )?;

    let rendered_data = if normalize_for_ocr {
        crate::pdf::render::normalize_rendered_page_for_ocr_with_security_limits(
            rendered.data,
            rendered.width,
            rendered.height,
            rotation,
            budget.security_limits,
        )
        .map(|(data, _, _)| data)
        .map_err(|error| {
            tracing::warn!(
                page = page_index + 1,
                rotation,
                error = %error,
                "layout runner: skipping page (OCR rotation normalization failed), returning empty detections"
            );
            format!("page {} OCR rotation normalization failed: {error}", page_index + 1)
        })?
    } else {
        rendered.data
    };

    crate::extraction::image_decode::decode_standard_rgb8_with_security_limits(&rendered_data, budget.security_limits)
        .map_err(|error| {
            tracing::warn!(
                page = page_index + 1,
                page_width_pts,
                page_height_pts,
                error = %error,
                "layout runner: skipping page (PNG decode failed), returning empty detections"
            );
            format!("page {} PNG decode failed: {error}", page_index + 1)
        })
}

/// Whether a page joins the ONNX batch: it must both be gate-selected and
/// have a real raster. Render-failed and gate-skipped pages stay out and end
/// up with empty detections.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn enters_inference_batch(page: &RenderedLayoutPage) -> bool {
    page.run_inference && page.image.is_some()
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn detect_layout_chunk(
    engine: &mut crate::layout::LayoutEngine,
    pages: &[RenderedLayoutPage],
    security_limits: &crate::extractors::security::SecurityLimits,
) -> Result<Vec<Option<crate::layout::DetectionResult>>> {
    let rendered_positions: Vec<usize> = pages
        .iter()
        .enumerate()
        .filter_map(|(position, page)| enters_inference_batch(page).then_some(position))
        .collect();
    if rendered_positions.is_empty() {
        return Ok((0..pages.len()).map(|_| None).collect());
    }

    let all_chunk_images: Vec<&image::RgbImage> = rendered_positions
        .iter()
        .map(|&position| pages[position].image.as_ref().expect("filtered to rendered pages"))
        .collect();
    let mut detections: Vec<Option<crate::layout::DetectionResult>> = (0..pages.len()).map(|_| None).collect();
    let (width, height) = all_chunk_images[0].dimensions();
    // This chunk's own rasters only. The pages already assembled into the
    // output are finished work, not part of the inference working set, so
    // charging them here would make the budget a function of document length
    // (GH#1721). Nothing caps the rasters this pass retains across a whole
    // document by default: `max_content_size` bounds only the work in flight,
    // and `max_pages`, which does bound the retained set, is `None` unless the
    // caller sets it. ~keep
    let current_live_bytes =
        crate::layout::engine::live_raster_bytes(pages.iter().filter_map(|page| page.image.as_ref()), width, height)?;
    let capacity = inference_batch_capacity(
        pages,
        &rendered_positions,
        current_live_bytes,
        (width, height),
        security_limits,
    )?;
    for position_chunk in rendered_positions.chunks(capacity) {
        let batch_live_bytes = checked_add_live(
            current_live_bytes,
            inference_crop_bytes(pages, position_chunk)?,
            width,
            height,
        )?;
        crate::layout::engine::validate_layout_inference_peak(
            width,
            height,
            batch_live_bytes,
            position_chunk.len(),
            security_limits,
        )?;
        let cropped: Vec<Option<image::RgbImage>> = position_chunk
            .iter()
            .map(|&position| inference_crop_image(&pages[position]))
            .collect();
        let images: Vec<&image::RgbImage> = position_chunk
            .iter()
            .zip(&cropped)
            .map(|(&position, crop)| {
                crop.as_ref()
                    .unwrap_or_else(|| pages[position].image.as_ref().expect("filtered to rendered pages"))
            })
            .collect();
        let results = engine
            .detect_batch(&images)
            .map_err(|error| XbergError::Other(format!("layout runner: batch detection failed: {error}")))?;
        validate_batch_cardinality(images.len(), results.len())?;
        for (&position, (mut detection, _timings)) in position_chunk.iter().zip(results) {
            shift_into_page_frame(&pages[position], &mut detection);
            detections[position] = Some(detection);
        }
    }
    Ok(detections)
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn assemble_layout_chunk(
    pages: Vec<RenderedLayoutPage>,
    detections: Vec<Option<crate::layout::DetectionResult>>,
) -> Result<Vec<AssembledLayoutPage>> {
    validate_batch_cardinality(pages.len(), detections.len())?;
    Ok(pages
        .into_iter()
        .zip(detections)
        .map(|(page, detection)| assemble_layout_page(page, detection))
        .collect())
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn assemble_layout_page(
    page: RenderedLayoutPage,
    detection: Option<crate::layout::DetectionResult>,
) -> AssembledLayoutPage {
    let image = page.image.unwrap_or_else(render_failure_placeholder);
    let detection = detection.unwrap_or_else(|| crate::layout::DetectionResult {
        page_width: image.width(),
        page_height: image.height(),
        detections: Vec::new(),
    });
    let hints = pixel_detection_to_layout_hints_pdf_space(
        &detection,
        image.width(),
        image.height(),
        page.page_width_pts,
        page.page_height_pts,
    );

    tracing::debug!(
        page = page.page_index + 1,
        detections = detection.detections.len(),
        hints = hints.len(),
        page_width_pts = page.page_width_pts,
        page_height_pts = page.page_height_pts,
        rotation = page.rotation,
        image_width_px = image.width(),
        image_height_px = image.height(),
        "layout runner: page detections"
    );

    AssembledLayoutPage {
        image,
        result: PageLayoutResult {
            page_width_pts: page.page_width_pts,
            page_height_pts: page.page_height_pts,
        },
        hints,
        detection,
    }
}

/// Builds the `ProcessingWarning` surfaced when one or more pages could not be
/// rendered, rotation-normalized, or PNG-decoded for layout analysis (#196).
/// Without this, a page that failed silently substitutes a blank placeholder
/// and an empty `DetectionResult`, indistinguishable from a page the model
/// actually examined and found empty.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn page_render_failure_warning(failures: &[String]) -> crate::types::ProcessingWarning {
    crate::types::ProcessingWarning {
        source: std::borrow::Cow::Borrowed("layout"),
        message: std::borrow::Cow::Owned(format!(
            "layout: {} page(s) could not be prepared for layout analysis and were treated as empty: {}",
            failures.len(),
            failures.join("; ")
        )),
    }
}

#[cfg(test)]
pub(super) fn run_layout_for_pdf_pages(
    content: &[u8],
    layout_config: &LayoutDetectionConfig,
    thread_budget: usize,
    gated_handling: GatedPageHandling,
) -> Result<(LayoutRunOutput, Option<crate::types::ProcessingWarning>)> {
    run_layout_for_pdf_pages_with_security_limits(
        content,
        layout_config,
        thread_budget,
        gated_handling,
        &crate::extractors::security::SecurityLimits::default(),
        None,
    )
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn run_layout_for_pdf_pages_with_security_limits(
    content: &[u8],
    layout_config: &LayoutDetectionConfig,
    thread_budget: usize,
    gated_handling: GatedPageHandling,
    security_limits: &crate::extractors::security::SecurityLimits,
    images_config: Option<&crate::core::config::ImageExtractionConfig>,
) -> Result<(LayoutRunOutput, Option<crate::types::ProcessingWarning>)> {
    let doc = xberg_native_pdf::PdfDocument::from_bytes(content.to_vec()).map_err(|e| XbergError::Parsing {
        message: format!("layout runner: failed to open PDF: {e}"),
        source: None,
    })?;

    let page_count = doc.page_count().map_err(|e| XbergError::Parsing {
        message: format!("layout runner: failed to get page count: {e}"),
        source: None,
    })?;

    if page_count == 0 {
        return Ok((
            LayoutRunOutput {
                data: Some((Vec::new(), Vec::new(), Vec::new(), Vec::new())),
                gate_decisions: None,
            },
            None,
        ));
    }

    let gate_decisions = auto_gate_decisions(&doc, layout_config, page_count);
    if let Some(decisions) = &gate_decisions
        && decisions.iter().all(|decision| !decision.run_layout)
    {
        // No page can benefit: behave exactly as if layout were off, with no
        // engine init and no renders. The OCR path then produces its own
        // rasters once, as it does without layout. ~keep
        return Ok((
            LayoutRunOutput {
                data: None,
                gate_decisions,
            },
            None,
        ));
    }

    let mut engine = crate::layout::take_or_create_engine(layout_config, thread_budget)
        .map_err(|e| XbergError::Other(format!("layout runner: engine init failed: {e}")))?;

    let page_rotations = crate::pdf::render::get_page_rotations(&doc, page_count);

    let mut all_images: Vec<image::RgbImage> = Vec::with_capacity(page_count);
    let mut all_layout_results: Vec<PageLayoutResult> = Vec::with_capacity(page_count);
    let mut all_hints: Vec<Vec<LayoutHint>> = Vec::with_capacity(page_count);
    let mut all_detections: Vec<crate::layout::DetectionResult> = Vec::with_capacity(page_count);
    let mut render_failures: Vec<String> = Vec::new();

    let total_chunks = page_count.div_ceil(LAYOUT_BATCH_CHUNK_SIZE);

    for (chunk_idx, chunk_start) in (0..page_count).step_by(LAYOUT_BATCH_CHUNK_SIZE).enumerate() {
        let chunk_end = (chunk_start + LAYOUT_BATCH_CHUNK_SIZE).min(page_count);
        let pages = render_layout_chunk(
            &doc,
            &page_rotations,
            chunk_start,
            chunk_end,
            gate_decisions.as_deref(),
            gated_handling,
            RenderBudget {
                security_limits,
                images_config,
            },
        );
        let rendered = pages.iter().filter(|page| page.image.is_some()).count();
        render_failures.extend(pages.iter().filter_map(|page| page.render_failure.clone()));
        tracing::debug!(
            chunk_idx,
            total_chunks,
            chunk_start,
            chunk_end,
            rendered,
            "layout runner: detecting chunk"
        );

        let detections = match detect_layout_chunk(&mut engine, &pages, security_limits) {
            Ok(detections) => detections,
            Err(error) => {
                crate::layout::return_engine(engine);
                return Err(error);
            }
        };
        for page in assemble_layout_chunk(pages, detections)? {
            all_images.push(page.image);
            all_layout_results.push(page.result);
            all_hints.push(page.hints);
            all_detections.push(page.detection);
        }
    }

    crate::layout::return_engine(engine);

    let render_warning = (!render_failures.is_empty()).then(|| page_render_failure_warning(&render_failures));

    Ok((
        LayoutRunOutput {
            data: Some((all_images, all_layout_results, all_hints, all_detections)),
            gate_decisions,
        },
        render_warning,
    ))
}

/// Convenience wrapper that reads `use_layout_for_markdown` and other gate
/// conditions from `config` and, when they are all satisfied, runs
/// [`run_layout_for_pdf_pages`].
///
/// The first four values are `None` when the feature is not requested, when
/// the `Auto` gate skipped every page, or on soft failure (logged so the
/// markdown path continues without layout hints). The fifth value carries the
/// `Auto` gate's per-page decisions whenever the gate ran, including the
/// all-gated case. The sixth value carries a `ProcessingWarning` when automatic
/// inference recovers on CPU or layout fails entirely. The seventh value carries
/// the proactively resolved or recovered CPU acceleration config so downstream
/// TATR/OCR models use the same provider. Rendering and inference run off the
/// async executor when a Tokio runtime is enabled. The eighth value carries any
/// `xberg_native_pdf` glyph-drop warnings captured while rendering this pass's pages
/// (#353); always present (possibly empty) rather than folded into the sixth
/// value so each cause stays a distinct, individually-deduped
/// `ProcessingWarning` when the caller merges it via `push_warning_deduped`.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
type LayoutForMarkdownOptional = (
    Option<Vec<image::RgbImage>>,
    Option<Vec<PageLayoutResult>>,
    Option<Vec<Vec<LayoutHint>>>,
    Option<Vec<crate::layout::DetectionResult>>,
    Option<Vec<PageGateDecision>>,
    Option<crate::types::ProcessingWarning>,
    Option<AccelerationConfig>,
    Vec<crate::types::ProcessingWarning>,
);

pub(super) fn layout_failure_warning(error: &crate::XbergError) -> crate::types::ProcessingWarning {
    crate::types::ProcessingWarning {
        source: std::borrow::Cow::Borrowed("layout"),
        message: std::borrow::Cow::Owned(format!(
            "layout detection failed ({error}); document extracted without layout hints"
        )),
    }
}

/// Warning emitted when automatic layout inference failed but the CPU provider
/// recovered the layout pass. The document keeps its layout while callers still
/// receive a signal that recovery occurred (#1344).
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
pub(super) fn layout_cpu_fallback_warning(error: &crate::XbergError) -> crate::types::ProcessingWarning {
    crate::types::ProcessingWarning {
        source: std::borrow::Cow::Borrowed("layout"),
        message: std::borrow::Cow::Owned(format!("automatic layout inference failed ({error}); recovered on CPU")),
    }
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
pub(super) async fn maybe_run_layout_for_markdown(
    content: &[u8],
    config: &ExtractionConfig,
) -> LayoutForMarkdownOptional {
    if !config.use_layout_for_markdown {
        return (None, None, None, None, None, None, None, Vec::new());
    }
    let Some(layout_config) = config.resolved_layout_config() else {
        return (None, None, None, None, None, None, None, Vec::new());
    };
    if config.force_ocr {
        return (None, None, None, None, None, None, None, Vec::new());
    }
    let thread_budget = crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());
    let default_security_limits = crate::extractors::security::SecurityLimits::default();
    let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
    let outcome = run_layout_for_pdf_pages_async(
        content,
        layout_config.as_ref(),
        thread_budget,
        GatedPageHandling::SkipRender,
        security_limits,
        config.images.as_ref(),
    )
    .await;

    match outcome {
        Ok((
            LayoutAttempt {
                output:
                    LayoutRunOutput {
                        data: Some((images, results, hints, detections)),
                        gate_decisions,
                    },
                acceleration_override,
                warning,
            },
            glyph_drop_warnings,
        )) => {
            let total_hints: usize = hints.iter().map(|h| h.len()).sum();
            tracing::info!(
                pages = images.len(),
                total_hints,
                "layout-for-markdown: detection succeeded"
            );
            (
                Some(images),
                Some(results),
                Some(hints),
                Some(detections),
                gate_decisions,
                warning,
                acceleration_override,
                glyph_drop_warnings,
            )
        }
        Ok((
            LayoutAttempt {
                output:
                    LayoutRunOutput {
                        data: None,
                        gate_decisions,
                    },
                acceleration_override: _,
                warning,
            },
            glyph_drop_warnings,
        )) => {
            tracing::info!("layout-for-markdown: auto gate skipped every page, continuing without layout hints");
            (
                None,
                None,
                None,
                None,
                gate_decisions,
                warning,
                None,
                glyph_drop_warnings,
            )
        }
        Err(error) => {
            tracing::warn!(
                error = %error,
                "layout-for-markdown: detection failed, continuing without layout hints"
            );
            let warning = layout_failure_warning(&error);
            (None, None, None, None, None, Some(warning), None, Vec::new())
        }
    }
}

#[cfg(all(test, paddle_ocr))]
static OCR_LAYOUT_RUN_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
#[cfg(all(test, paddle_ocr))]
static WATCHED_OCR_LAYOUT_CONTENT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(all(test, paddle_ocr))]
pub(super) fn watch_ocr_layout_runs_for(content: &[u8]) {
    OCR_LAYOUT_RUN_COUNT.store(0, std::sync::atomic::Ordering::SeqCst);
    WATCHED_OCR_LAYOUT_CONTENT.store(content.as_ptr() as usize, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(all(test, paddle_ocr))]
pub(super) fn ocr_layout_run_count() -> usize {
    WATCHED_OCR_LAYOUT_CONTENT.store(0, std::sync::atomic::Ordering::SeqCst);
    OCR_LAYOUT_RUN_COUNT.load(std::sync::atomic::Ordering::SeqCst)
}

/// Run the layout pass used by OCR without blocking a Tokio worker thread.
///
/// `result` holds `data: None` when the `Auto` gate skipped every page: the caller
/// then proceeds exactly as if layout were off, rendering its own OCR rasters once.
/// Mirrors the markdown path's CPU fallback and warning on accelerated failure (#1344).
///
/// The second element of the returned tuple carries any `xberg_native_pdf` glyph-drop
/// warnings captured while this pass rendered its pages (#353); the caller must
/// merge them into the document's `processing_warnings`.
#[cfg(all(
    feature = "pdf",
    feature = "layout-detection",
    any(feature = "ocr", feature = "ocr-pipeline")
))]
pub(super) async fn run_layout_for_ocr(
    content: &[u8],
    layout_config: &LayoutDetectionConfig,
    thread_budget: usize,
    security_limits: &crate::extractors::security::SecurityLimits,
    images_config: Option<&crate::core::config::ImageExtractionConfig>,
) -> Result<(LayoutAttempt<LayoutRunOutput>, Vec<crate::types::ProcessingWarning>)> {
    #[cfg(all(test, paddle_ocr))]
    if WATCHED_OCR_LAYOUT_CONTENT.load(std::sync::atomic::Ordering::SeqCst) == content.as_ptr() as usize {
        OCR_LAYOUT_RUN_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    // OCR consumes the layout pass's rasters as its input images, so gated
    // pages still render; only model inference is skipped for them.
    run_layout_for_pdf_pages_async(
        content,
        layout_config,
        thread_budget,
        GatedPageHandling::RenderWithoutInference,
        security_limits,
        images_config,
    )
    .await
}

#[cfg(all(test, feature = "pdf", feature = "layout-detection"))]
mod tests {
    #[cfg(target_os = "macos")]
    use super::rtdetr_acceleration_override;
    use super::{
        FAILED_RENDER_PLACEHOLDER_SIDE, GatedPageHandling, RenderBudget, RenderedLayoutPage, assemble_layout_chunk,
        displayed_page_dimensions, gate_selects_page, render_failure_placeholder, render_layout_chunk,
        run_layout_with_auto_cpu_retry, validate_batch_cardinality,
    };
    use crate::XbergError;
    use crate::core::config::acceleration::{AccelerationConfig, ExecutionProviderType};
    use crate::core::config::layout::LayoutDetectionConfig;
    use crate::ort_discovery::parse_execution_provider_override;
    use crate::pdf::layout_gate::PageGateDecision;

    fn provider(config: &LayoutDetectionConfig) -> ExecutionProviderType {
        config
            .acceleration
            .as_ref()
            .map_or(ExecutionProviderType::Auto, |acceleration| {
                acceleration.provider.clone()
            })
    }

    fn inference_run_error(message: &str) -> XbergError {
        XbergError::Other(format!(
            "layout runner: batch detection failed: inference error: inference run failed: {message}"
        ))
    }

    #[test]
    fn should_retry_once_on_cpu_when_auto_provider_inference_run_fails() {
        for config in [
            LayoutDetectionConfig::default(),
            LayoutDetectionConfig {
                acceleration: Some(AccelerationConfig {
                    provider: ExecutionProviderType::Auto,
                    ..Default::default()
                }),
                ..Default::default()
            },
        ] {
            let mut attempts = Vec::new();

            let result = run_layout_with_auto_cpu_retry(&config, false, None, |attempt_config| {
                attempts.push(provider(attempt_config));
                if attempts.len() == 1 {
                    Err(inference_run_error("CoreML ExecuteKernel failed"))
                } else {
                    Ok(42)
                }
            });

            let attempt = result.expect("CPU retry should succeed");
            assert_eq!(attempt.output, 42);
            assert_eq!(
                attempt.acceleration_override.map(|acceleration| acceleration.provider),
                Some(ExecutionProviderType::Cpu)
            );
            let warning = attempt.warning.expect("CPU recovery should be caller-visible");
            assert_eq!(warning.source, "layout");
            assert!(warning.message.contains("automatic layout inference failed"));
            assert!(warning.message.contains("CoreML ExecuteKernel failed"));
            assert!(warning.message.contains("recovered on CPU"));
            assert_eq!(attempts, [ExecutionProviderType::Auto, ExecutionProviderType::Cpu]);
        }
    }

    #[test]
    fn should_return_first_success_without_override_or_warning() {
        let config = LayoutDetectionConfig::default();
        let mut attempts = Vec::new();

        let attempt = run_layout_with_auto_cpu_retry(&config, false, None, |attempt_config| {
            attempts.push(provider(attempt_config));
            Ok(42)
        })
        .expect("first attempt should succeed");

        assert_eq!(attempt.output, 42);
        assert!(attempt.acceleration_override.is_none());
        assert!(attempt.warning.is_none());
        assert_eq!(attempts, [ExecutionProviderType::Auto]);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn should_propagate_rtdetr_auto_cpu_resolution_without_retry() {
        let config = LayoutDetectionConfig::default();
        for value in ["", " \t", "invalid"] {
            let execution_provider_overridden = parse_execution_provider_override(value).is_some();
            let override_for_value = rtdetr_acceleration_override(&config, execution_provider_overridden)
                .expect("blank or invalid environment values must not suppress macOS RT-DETR CPU resolution");
            assert_eq!(override_for_value.provider, ExecutionProviderType::Cpu);
        }
        for value in ["cpu", "coreml", "cuda", "tensorrt", "auto"] {
            let execution_provider_overridden = parse_execution_provider_override(value).is_some();
            assert!(
                rtdetr_acceleration_override(&config, execution_provider_overridden).is_none(),
                "recognized environment provider must remain authoritative: {value:?}"
            );
        }

        let acceleration_override =
            rtdetr_acceleration_override(&config, false).expect("macOS RT-DETR Auto should resolve to CPU");
        assert_eq!(acceleration_override.provider, ExecutionProviderType::Cpu);
        let mut attempts = Vec::new();

        let attempt = run_layout_with_auto_cpu_retry(&config, false, Some(acceleration_override), |attempt_config| {
            attempts.push(provider(attempt_config));
            Ok(42)
        })
        .expect("CPU-normalized first attempt should succeed");

        assert_eq!(attempt.output, 42);
        assert_eq!(
            attempt.acceleration_override.map(|acceleration| acceleration.provider),
            Some(ExecutionProviderType::Cpu)
        );
        assert!(attempt.warning.is_none());
        assert_eq!(attempts, [ExecutionProviderType::Cpu]);
    }

    #[test]
    fn should_not_retry_when_inference_run_fails_for_explicit_provider() {
        for explicit_provider in [
            ExecutionProviderType::Cpu,
            ExecutionProviderType::CoreMl,
            ExecutionProviderType::Cuda,
            ExecutionProviderType::TensorRt,
        ] {
            let config = LayoutDetectionConfig {
                acceleration: Some(AccelerationConfig {
                    provider: explicit_provider.clone(),
                    device_id: 0,
                }),
                ..Default::default()
            };
            let mut attempts = Vec::new();

            let error = run_layout_with_auto_cpu_retry(&config, false, None, |attempt_config| {
                attempts.push(provider(attempt_config));
                Err::<(), _>(inference_run_error("explicit provider failed"))
            })
            .expect_err("explicit providers must not retry");

            assert_eq!(
                error.to_string(),
                inference_run_error("explicit provider failed").to_string()
            );
            assert_eq!(attempts, [explicit_provider]);
        }
    }

    #[test]
    fn should_not_retry_when_auto_provider_failure_is_not_inference_run() {
        let config = LayoutDetectionConfig::default();
        let mut attempts = Vec::new();
        let expected = "layout runner: engine init failed: inference error: failed to load inference model";

        let error = run_layout_with_auto_cpu_retry(&config, false, None, |attempt_config| {
            attempts.push(provider(attempt_config));
            Err::<(), _>(XbergError::Other(expected.to_string()))
        })
        .expect_err("model-load failures must not retry");

        assert_eq!(error.to_string(), XbergError::Other(expected.to_string()).to_string());
        assert_eq!(attempts, [ExecutionProviderType::Auto]);
    }

    #[test]
    fn should_return_context_when_cpu_retry_fails() {
        let config = LayoutDetectionConfig::default();
        let mut attempts = Vec::new();

        let error = run_layout_with_auto_cpu_retry(&config, false, None, |attempt_config| {
            attempts.push(provider(attempt_config));
            let message = if attempts.len() == 1 {
                "CoreML ExecuteKernel failed"
            } else {
                "CPU kernel failed"
            };
            Err::<(), _>(inference_run_error(message))
        })
        .expect_err("both attempts should fail");

        assert_eq!(attempts, [ExecutionProviderType::Auto, ExecutionProviderType::Cpu]);
        let message = error.to_string();
        assert!(message.contains("initial error:"));
        assert!(message.contains("CoreML ExecuteKernel failed"));
        assert!(message.contains("CPU retry error:"));
        assert!(message.contains("CPU kernel failed"));
    }

    #[test]
    fn should_not_retry_when_execution_provider_environment_override_is_set() {
        let config = LayoutDetectionConfig::default();
        for value in ["cpu", "coreml", "cuda", "tensorrt", "auto"] {
            let mut attempts = Vec::new();
            let execution_provider_overridden = parse_execution_provider_override(value).is_some();

            let error =
                run_layout_with_auto_cpu_retry(&config, execution_provider_overridden, None, |attempt_config| {
                    attempts.push(provider(attempt_config));
                    Err::<(), _>(inference_run_error("explicit environment provider failed"))
                })
                .expect_err("a recognized environment provider override must not retry");

            assert_eq!(
                error.to_string(),
                inference_run_error("explicit environment provider failed").to_string()
            );
            assert_eq!(attempts, [ExecutionProviderType::Auto], "value: {value:?}");
        }
    }

    #[test]
    fn should_retry_when_execution_provider_environment_override_is_blank_or_invalid() {
        let config = LayoutDetectionConfig::default();

        for value in ["", " \t", "invalid"] {
            let mut attempts = Vec::new();
            let execution_provider_overridden = parse_execution_provider_override(value).is_some();

            let attempt =
                run_layout_with_auto_cpu_retry(&config, execution_provider_overridden, None, |attempt_config| {
                    attempts.push(provider(attempt_config));
                    if attempts.len() == 1 {
                        Err(inference_run_error("automatic provider failed"))
                    } else {
                        Ok(42)
                    }
                })
                .expect("a blank or invalid environment provider value must not suppress CPU retry");

            assert_eq!(attempt.output, 42);
            assert_eq!(
                attempt.acceleration_override.map(|acceleration| acceleration.provider),
                Some(ExecutionProviderType::Cpu)
            );
            assert_eq!(
                attempts,
                [ExecutionProviderType::Auto, ExecutionProviderType::Cpu],
                "value: {value:?}"
            );
        }
    }

    fn rendered_page(page_index: usize, width: u32, page_width_pts: f32) -> RenderedLayoutPage {
        RenderedLayoutPage {
            page_index,
            page_width_pts,
            page_height_pts: 200.0,
            rotation: 0,
            image: Some(image::RgbImage::new(width, 20)),
            run_inference: true,
            render_failure: None,
            inference_crop: None,
        }
    }

    fn rotated_pdf(inherited: bool, rotation: i64) -> Vec<u8> {
        use lopdf::{Document, Object, Stream, dictionary};

        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();
        let page_id = document.new_object_id();
        let content_id = document.add_object(Stream::new(dictionary! {}, Vec::new()));

        let mut page = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 100.into()],
            "Resources" => dictionary! {},
            "Contents" => content_id,
        };
        if !inherited {
            page.set("Rotate", rotation);
        }
        document.objects.insert(page_id, Object::Dictionary(page));

        let mut pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        };
        if inherited {
            pages.set("Rotate", rotation);
        }
        document.objects.insert(pages_id, Object::Dictionary(pages));

        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);

        let mut bytes = Vec::new();
        document.save_to(&mut bytes).expect("fixture PDF must serialize");
        bytes
    }

    /// A PDF whose page tree declares `/Count 2` in the `Pages` dictionary but
    /// whose `/Kids` array lists only one real page. `page_count()` trusts the
    /// declared `/Count` (xberg_native_pdf's primary reader), so the runner iterates
    /// page index 1 as if it existed; the page tree walk that resolves that
    /// index then genuinely fails, giving a real (not injected) render
    /// failure at the same seam production traffic would hit for a
    /// truncated/corrupt page tree.
    fn pdf_with_declared_count_exceeding_real_pages() -> Vec<u8> {
        use lopdf::{Document, Object, Stream, dictionary};

        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();
        let page_id = document.new_object_id();
        let content_id = document.add_object(Stream::new(dictionary! {}, Vec::new()));

        let page = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 100.into()],
            "Resources" => dictionary! {},
            "Contents" => content_id,
        };
        document.objects.insert(page_id, Object::Dictionary(page));

        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 2,
        };
        document.objects.insert(pages_id, Object::Dictionary(pages));

        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);

        let mut bytes = Vec::new();
        document.save_to(&mut bytes).expect("fixture PDF must serialize");
        bytes
    }

    fn assert_xberg_native_pdf_applies_rotation(bytes: Vec<u8>) {
        let document = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).expect("fixture PDF must open");
        let rendered =
            crate::pdf::render::render_page_with_safeguards(&document, 0, 72).expect("rotated fixture must render");
        let rotations = crate::pdf::render::get_page_rotations(&document, 1);
        let (media_width, media_height) = document
            .get_page_media_box(0)
            .map(|(llx, lly, urx, ury)| ((urx - llx).abs(), (ury - lly).abs()))
            .expect("fixture must have a MediaBox");

        assert_eq!(rotations, vec![90]);
        assert!(
            rendered.height > rendered.width,
            "rotated landscape page must render as portrait"
        );
        assert_eq!(
            displayed_page_dimensions(media_width, media_height, rotations[0]),
            (100.0, 200.0)
        );
    }

    #[test]
    fn batch_cardinality_accepts_one_result_per_rendered_page() {
        assert!(validate_batch_cardinality(3, 3).is_ok());
    }

    #[test]
    fn batch_cardinality_rejects_truncated_results() {
        let error = validate_batch_cardinality(3, 2).expect_err("truncated results must fail");
        assert!(error.to_string().contains("2 results for 3 rendered pages"));
    }

    #[test]
    fn render_failure_placeholder_is_nonempty_and_white() {
        let image = render_failure_placeholder();

        assert_eq!(
            image.dimensions(),
            (FAILED_RENDER_PLACEHOLDER_SIDE, FAILED_RENDER_PLACEHOLDER_SIDE)
        );
        assert!(image.pixels().all(|pixel| pixel.0 == [u8::MAX; 3]));
    }

    #[test]
    fn assembly_preserves_page_order_and_detection_mapping() {
        let pages = vec![rendered_page(0, 10, 100.0), rendered_page(1, 20, 110.0)];
        let detections = vec![
            Some(crate::layout::DetectionResult {
                page_width: 10,
                page_height: 20,
                detections: Vec::new(),
            }),
            Some(crate::layout::DetectionResult {
                page_width: 20,
                page_height: 20,
                detections: Vec::new(),
            }),
        ];

        let assembled = assemble_layout_chunk(pages, detections).expect("aligned chunk must assemble");

        assert_eq!(assembled[0].image.width(), 10);
        assert_eq!(assembled[0].result.page_width_pts, 100.0);
        assert_eq!(assembled[0].detection.page_width, 10);
        assert_eq!(assembled[1].image.width(), 20);
        assert_eq!(assembled[1].result.page_width_pts, 110.0);
        assert_eq!(assembled[1].detection.page_width, 20);
    }

    #[test]
    fn assembly_keeps_failed_render_slot_with_empty_detection() {
        let mut failed_page = rendered_page(1, 20, 110.0);
        failed_page.image = None;

        let assembled = assemble_layout_chunk(vec![failed_page], vec![None]).expect("failed render must stay aligned");

        assert_eq!(
            assembled[0].image.dimensions(),
            (FAILED_RENDER_PLACEHOLDER_SIDE, FAILED_RENDER_PLACEHOLDER_SIDE)
        );
        assert!(assembled[0].detection.detections.is_empty());
        assert_eq!(assembled[0].result.page_width_pts, 110.0);
    }

    #[test]
    fn assembly_rejects_detection_cardinality_mismatch() {
        let error = assemble_layout_chunk(vec![rendered_page(0, 10, 100.0)], Vec::new())
            .err()
            .expect("missing detection slot must fail");

        assert!(error.to_string().contains("0 results for 1 rendered pages"));
    }

    fn gate_decision(run_layout: bool) -> PageGateDecision {
        PageGateDecision {
            run_layout,
            reason: if run_layout {
                crate::pdf::layout_gate::GateReason::MultiColumn
            } else {
                crate::pdf::layout_gate::GateReason::PlainText
            },
        }
    }

    /// A one-page single-column prose PDF with a real text content stream:
    /// the gate must classify it as plain text and skip the model.
    fn prose_pdf(lines: usize) -> Vec<u8> {
        use lopdf::{Document, Object, Stream, dictionary};

        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();
        let font_id = document.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let mut content = String::from("BT /F1 10 Tf 72 720 Td 14 TL\n");
        for line in 0..lines {
            content.push_str(&format!(
                "(Line {line} of steady single column prose with enough characters to pass the floor.) Tj T*\n"
            ));
        }
        content.push_str("ET");
        let content_id = document.add_object(Stream::new(dictionary! {}, content.into_bytes()));
        let page_id = document.new_object_id();
        document.objects.insert(
            page_id,
            Object::Dictionary(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
                "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
                "Contents" => content_id,
            }),
        );
        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);

        let mut bytes = Vec::new();
        document.save_to(&mut bytes).expect("fixture PDF must serialize");
        bytes
    }

    /// End-to-end `Auto` on real PDF bytes: the gate skips plain prose, and
    /// the runner returns no layout data before any engine init (this test
    /// must never need ONNX models). The gate is asserted first so a gate
    /// regression fails here rather than in an engine-init attempt.
    #[test]
    fn auto_strategy_gates_prose_page_and_skips_the_layout_pass_entirely() {
        use crate::core::config::layout::LayoutStrategy;

        let bytes = prose_pdf(20);
        let doc = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).expect("fixture PDF must open");
        let decisions = crate::pdf::layout_gate::decide_pages(&doc, 1);
        assert_eq!(decisions.len(), 1);
        assert!(
            !decisions[0].run_layout,
            "prose fixture must be gated, got {:?}",
            decisions[0].reason
        );
        assert_eq!(decisions[0].reason, crate::pdf::layout_gate::GateReason::PlainText);

        let config = crate::core::config::layout::LayoutDetectionConfig {
            strategy: LayoutStrategy::Auto,
            ..Default::default()
        };
        let (output, render_warning) =
            super::run_layout_for_pdf_pages(&bytes, &config, 1, GatedPageHandling::SkipRender)
                .expect("all-gated run must succeed without a layout engine");
        assert!(output.data.is_none(), "all-gated prose must skip the layout pass");
        assert!(
            render_warning.is_none(),
            "all-gated run performs no renders and must not warn"
        );
        let recorded = output.gate_decisions.expect("auto strategy must record decisions");
        assert_eq!(recorded, decisions);
    }

    #[test]
    fn only_gate_selected_pages_with_real_rasters_enter_the_inference_batch() {
        let selected_with_image = rendered_page(0, 10, 100.0);
        let mut gated_with_image = rendered_page(1, 10, 100.0);
        gated_with_image.run_inference = false;
        let mut selected_render_failed = rendered_page(2, 10, 100.0);
        selected_render_failed.image = None;

        assert!(super::enters_inference_batch(&selected_with_image));
        assert!(!super::enters_inference_batch(&gated_with_image));
        assert!(!super::enters_inference_batch(&selected_render_failed));
    }

    #[test]
    fn gated_page_assembles_placeholder_image_with_empty_detection_and_hints() {
        let mut gated = rendered_page(0, 10, 100.0);
        gated.image = None;
        gated.run_inference = false;

        let assembled = assemble_layout_chunk(vec![gated], vec![None]).expect("gated page must stay aligned");

        assert_eq!(
            assembled[0].image.dimensions(),
            (FAILED_RENDER_PLACEHOLDER_SIDE, FAILED_RENDER_PLACEHOLDER_SIDE)
        );
        assert!(assembled[0].detection.detections.is_empty());
        assert!(assembled[0].hints.is_empty());
    }

    #[test]
    fn absent_gate_decisions_select_every_page() {
        assert!(gate_selects_page(None, 0));
        assert!(gate_selects_page(None, 7));
    }

    #[test]
    fn gate_decisions_beyond_the_vector_fail_open_to_running_the_model() {
        let decisions = vec![gate_decision(false)];
        assert!(gate_selects_page(Some(&decisions), 1));
    }

    #[test]
    fn gated_page_is_excluded_and_selected_page_is_kept() {
        let decisions = vec![gate_decision(false), gate_decision(true)];
        assert!(!gate_selects_page(Some(&decisions), 0));
        assert!(gate_selects_page(Some(&decisions), 1));
    }

    fn letter_page_pdf(crop_box: Option<[i64; 4]>) -> Vec<u8> {
        use lopdf::{Document, Object, Stream, dictionary};

        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();
        let page_id = document.new_object_id();
        let content_id = document.add_object(Stream::new(dictionary! {}, Vec::new()));
        let mut page = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Resources" => dictionary! {},
            "Contents" => content_id,
        };
        if let Some(crop_box) = crop_box {
            page.set("CropBox", crop_box.iter().map(|&v| Object::from(v)).collect::<Vec<_>>());
        }
        document.objects.insert(page_id, Object::Dictionary(page));
        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);

        let mut bytes = Vec::new();
        document.save_to(&mut bytes).expect("fixture PDF must serialize");
        bytes
    }

    /// The OCR route and a page without a CropBox keep the whole raster.
    #[test]
    fn should_infer_markdown_layout_on_the_cropbox_only() {
        let render = |bytes: Vec<u8>, gated_handling: GatedPageHandling| {
            let doc = xberg_native_pdf::PdfDocument::from_bytes(bytes).expect("fixture PDF must open");
            render_layout_chunk(
                &doc,
                &[0],
                0,
                1,
                None,
                gated_handling,
                RenderBudget {
                    security_limits: &crate::extractors::security::SecurityLimits::default(),
                    images_config: None,
                },
            )
            .remove(0)
        };
        let cropped = letter_page_pdf(Some([50, 102, 562, 742]));

        let page = render(cropped.clone(), GatedPageHandling::SkipRender);
        let image = page.image.as_ref().expect("the page renders");
        assert_eq!(image.dimensions(), (1_275, 1_650), "Letter at the default 150 dpi");
        assert_eq!(page.inference_crop, Some((104, 104, 1_067, 1_333)));

        let ocr_page = render(cropped, GatedPageHandling::RenderWithoutInference);
        assert_eq!(ocr_page.inference_crop, None, "the OCR route keeps the whole page");

        let uncropped = render(letter_page_pdf(None), GatedPageHandling::SkipRender);
        assert_eq!(uncropped.inference_crop, None);
    }

    /// SkipRender must not rasterise gated pages; RenderWithoutInference must.
    #[test]
    fn gated_handling_controls_whether_gated_pages_render() {
        let bytes = rotated_pdf(false, 90);
        let doc = xberg_native_pdf::PdfDocument::from_bytes(bytes).expect("fixture PDF must open");
        let decisions = vec![gate_decision(false)];

        let skipped = render_layout_chunk(
            &doc,
            &[0],
            0,
            1,
            Some(&decisions),
            GatedPageHandling::SkipRender,
            RenderBudget {
                security_limits: &crate::extractors::security::SecurityLimits::default(),
                images_config: None,
            },
        );
        assert!(skipped[0].image.is_none(), "gated page must not render");
        assert!(!skipped[0].run_inference);

        let rendered = render_layout_chunk(
            &doc,
            &[0],
            0,
            1,
            Some(&decisions),
            GatedPageHandling::RenderWithoutInference,
            RenderBudget {
                security_limits: &crate::extractors::security::SecurityLimits::default(),
                images_config: None,
            },
        );
        assert!(rendered[0].image.is_some(), "OCR-path gated page must still render");
        assert!(!rendered[0].run_inference, "gated page must stay out of the ONNX batch");
    }

    // `feature = "ocr"` exactly, not `any(ocr, ocr-pipeline)`: the standalone
    // arm calls `render_selected_pages_for_ocr`, which is `ocr`-gated.
    #[cfg(feature = "ocr")]
    #[test]
    fn layout_and_standalone_ocr_paths_share_rotation_normalization() {
        for rotation in [0, 90, 180, 270] {
            let bytes = rotated_pdf(false, rotation);
            let standalone = crate::extractors::pdf::ocr::render_selected_pages_for_ocr(&bytes, &[0])
                .expect("standalone OCR page should render")
                .pop()
                .expect("fixture should contain one page")
                .1
                .to_rgb8();
            let document = xberg_native_pdf::PdfDocument::from_bytes(bytes).expect("layout OCR fixture PDF must open");
            let mut layout_pages = render_layout_chunk(
                &document,
                &[rotation as u32],
                0,
                1,
                None,
                GatedPageHandling::RenderWithoutInference,
                RenderBudget {
                    security_limits: &crate::extractors::security::SecurityLimits::default(),
                    images_config: None,
                },
            );
            let layout = layout_pages
                .pop()
                .and_then(|page| page.image)
                .expect("layout OCR page should render");

            assert_eq!(layout.dimensions(), standalone.dimensions(), "PDF rotation {rotation}");
            assert_eq!(layout.as_raw(), standalone.as_raw(), "PDF rotation {rotation}");
        }
    }

    #[test]
    fn xberg_native_pdf_applies_direct_page_rotation() {
        assert_xberg_native_pdf_applies_rotation(rotated_pdf(false, 90));
    }

    #[test]
    fn xberg_native_pdf_applies_inherited_page_rotation() {
        assert_xberg_native_pdf_applies_rotation(rotated_pdf(true, 90));
    }

    /// Drives a genuine, unrenderable page through the full
    /// `run_layout_for_pdf_pages` entry point (#196): the fixture's page tree
    /// declares `/Count 2` but only lists one real page, so xberg_native_pdf's page
    /// tree walk for index 1 fails for real — this is not an injected error.
    /// The other page renders and runs inference normally, proving the
    /// warning is additive rather than replacing the successful pages.
    #[test]
    fn should_return_exact_render_failure_warning_when_a_page_fails_to_render() {
        use crate::core::config::layout::LayoutStrategy;

        let bytes = pdf_with_declared_count_exceeding_real_pages();
        let config = LayoutDetectionConfig {
            strategy: LayoutStrategy::Always,
            ..Default::default()
        };

        let (output, warning) = super::run_layout_for_pdf_pages(&bytes, &config, 1, GatedPageHandling::SkipRender)
            .expect("the run must succeed even though one page failed to render");

        let (images, _results, _hints, _detections) = output
            .data
            .expect("the one real page rendered fine, so layout data must be present");
        assert_eq!(
            images.len(),
            2,
            "both page slots must stay present despite the render failure"
        );

        let warning = warning.expect("a render failure must surface a caller-visible warning");
        assert_eq!(warning.source, "layout");
        assert_eq!(
            warning.message,
            "layout: 1 page(s) could not be prepared for layout analysis and were treated as empty: \
             page 2 failed to render: Invalid PDF: Page index 1 not found by scanning"
        );
    }

    /// A `page_count`-page PDF of blank US Letter pages. At the default 150 dpi
    /// render resolution each page rasterises to 1275 × 1650 × 3 bytes, the page
    /// size #1721's accumulation arithmetic is stated in.
    fn blank_letter_pages_pdf(page_count: usize) -> Vec<u8> {
        use lopdf::{Document, Object, Stream, dictionary};

        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();
        let mut kids = Vec::with_capacity(page_count);
        for _ in 0..page_count {
            let content_id = document.add_object(Stream::new(dictionary! {}, Vec::new()));
            let page_id = document.new_object_id();
            document.objects.insert(
                page_id,
                Object::Dictionary(dictionary! {
                    "Type" => "Page",
                    "Parent" => pages_id,
                    "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
                    "Resources" => dictionary! {},
                    "Contents" => content_id,
                }),
            );
            kids.push(Object::Reference(page_id));
        }
        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => kids,
                "Count" => page_count as i64,
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);

        let mut bytes = Vec::new();
        document.save_to(&mut bytes).expect("fixture PDF must serialize");
        bytes
    }

    /// Regression test for #1721.
    ///
    /// The inference budget is charged per chunk. Before this fix every raster
    /// the pass had already assembled was charged as well, so the running total
    /// crossed `max_content_size` partway through a long document, the whole
    /// layout pass failed, and extraction finished with no layout hints and a
    /// warning the caller could not tell from "the model found nothing".
    ///
    /// Thirteen Letter pages is the smallest fixture that crosses it at the
    /// 100 MiB default: the second chunk charged the eight retained rasters
    /// (50,490,000 B), its own five (31,556,250 B) and one model workspace
    /// (24,576,000 B), for 106,622,250 B against 104,857,600 B. Twelve pages
    /// stayed under, so a shorter fixture passes on the broken tree and proves
    /// nothing.
    #[test]
    fn layout_completes_on_a_document_longer_than_the_retained_rasters_used_to_allow() {
        use crate::core::config::layout::LayoutStrategy;

        const PAGE_COUNT: usize = 13;
        const LETTER_RASTER_AT_DEFAULT_DPI: (u32, u32) = (1_275, 1_650);
        // The second chunk is what charges the first chunk's retained rasters, so a
        // fixture at or below the chunk size cannot reach the defect at all.
        const { assert!(PAGE_COUNT > super::LAYOUT_BATCH_CHUNK_SIZE) };

        let bytes = blank_letter_pages_pdf(PAGE_COUNT);
        let config = LayoutDetectionConfig {
            strategy: LayoutStrategy::Always,
            ..Default::default()
        };

        let (output, render_warning) =
            super::run_layout_for_pdf_pages(&bytes, &config, 1, GatedPageHandling::SkipRender)
                .expect("a thirteen-page document must complete the layout pass");

        assert!(
            render_warning.is_none(),
            "every page of the fixture renders: {render_warning:?}"
        );
        let (images, _results, _hints, detections) = output
            .data
            .expect("layout data must survive a document longer than twelve pages");
        assert_eq!(images.len(), PAGE_COUNT);
        assert_eq!(detections.len(), PAGE_COUNT);
        for (page_index, (image, detection)) in images.iter().zip(&detections).enumerate() {
            assert_eq!(
                image.dimensions(),
                LETTER_RASTER_AT_DEFAULT_DPI,
                "page {} must rasterise at the standard Letter size the budget arithmetic assumes",
                page_index + 1
            );
            assert_eq!(
                (detection.page_width, detection.page_height),
                LETTER_RASTER_AT_DEFAULT_DPI,
                "page {} must carry a detection result taken from its own raster, not a placeholder",
                page_index + 1
            );
        }
    }

    /// #196's combination step: a CPU-retry recovery warning and a
    /// page-render-failure warning firing on the same attempt must both
    /// survive `combine_layout_warnings`, concatenated rather than one
    /// discarding the other.
    #[test]
    fn should_concatenate_cpu_retry_and_render_failure_warnings_without_dropping_either() {
        let cpu_retry_warning =
            super::layout_cpu_fallback_warning(&XbergError::Other("CoreML ExecuteKernel failed".to_string()));
        let render_failure_warning = super::page_render_failure_warning(&["page 2 failed to render: boom".to_string()]);

        let combined = super::combine_layout_warnings(Some(cpu_retry_warning), Some(render_failure_warning))
            .expect("both warnings present must combine rather than one being dropped");

        assert_eq!(combined.source, "layout");
        assert_eq!(
            combined.message,
            "automatic layout inference failed (CoreML ExecuteKernel failed); recovered on CPU; \
             layout: 1 page(s) could not be prepared for layout analysis and were treated as empty: \
             page 2 failed to render: boom"
        );
    }

    /// One-page fixture whose `/Font` resource is a PDF string rather than a
    /// font dictionary, mirroring `build_pdf_with_malformed_font_resource` in
    /// `tests/issue_291_dropped_glyph_warning.rs`. Duplicated here (rather
    /// than shared) because `run_layout_for_pdf_pages_async` is private to
    /// this crate and unreachable from that external integration test.
    /// `xberg_native_pdf`'s `PageRenderer::load_resources` requires every `/Font`
    /// resource to resolve to a dictionary and logs a sanitized static warning
    /// ("rendering text with fallback font data", with the diagnosis carried in
    /// structured tracing fields rather than the message -- see `page_renderer.rs`'s
    /// `load_resources`) then drops the glyph rather than failing the page.
    #[cfg(feature = "tokio-runtime")]
    fn malformed_font_layout_fixture() -> Vec<u8> {
        use lopdf::{Document, Object, Stream, dictionary};

        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();
        let font_id = document.add_object(Object::string_literal("NotAFontDict"));
        let content = "BT /F1 24 Tf 72 700 Td (Hello) Tj ET";
        let content_id = document.add_object(Stream::new(dictionary! {}, content.as_bytes().to_vec()));
        let page_id = document.new_object_id();
        document.objects.insert(
            page_id,
            Object::Dictionary(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "MediaBox" => vec![0.into(), 0.into(), 300.into(), 792.into()],
                "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
                "Contents" => content_id,
            }),
        );
        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);

        let mut bytes = Vec::new();
        document.save_to(&mut bytes).expect("fixture PDF must serialize");
        bytes
    }

    /// Regression test for #353.
    ///
    /// Layout-detection rasterization runs the whole layout pass — including
    /// every render call — inside `tokio::task::spawn_blocking`, which always
    /// executes on a blocking-pool OS thread distinct from the thread that
    /// called it. `take_xberg_native_pdf_render_warnings` drains a *thread-local*
    /// buffer (`ENGINE_PENDING_WARNINGS`), so before this fix nothing ever
    /// visited that blocking-pool thread again to drain it: any glyph-drop
    /// warning captured during layout rendering was silently lost, and kept
    /// accumulating, unbounded, on that pooled thread across documents.
    ///
    /// This drives the real `spawn_blocking` boundary
    /// (`run_layout_for_pdf_pages_async`, gated on the default-on
    /// `tokio-runtime` feature) with a page whose font resource is
    /// structurally invalid, so `xberg_native_pdf` drops every glyph drawn with it
    /// and reports the drop through `tracing::warn!` (see `crate::pdf::render`
    /// and the #291/#1364 fixture in `tests/issue_291_dropped_glyph_warning.rs`).
    #[cfg(feature = "tokio-runtime")]
    #[tokio::test]
    async fn glyph_drop_warnings_survive_the_spawn_blocking_layout_pass() {
        use crate::core::config::layout::LayoutStrategy;

        // Capture is opt-in: unless an application (here, the test) asks for it,
        // `render_page_capturing_glyph_drops` arms nothing. Asserting the return value
        // (rather than ignoring it) is required, not cosmetic — a test binary where some
        // other component already owns the `tracing` dispatcher would otherwise pass this
        // test for the wrong reason: no warnings would ever be captured at all, layout
        // render or not. ~keep
        assert!(
            crate::pdf::render::install_pdf_render_diagnostics(),
            "no other component should own the tracing dispatcher in this test binary"
        );
        // Drain any residual state left on this thread by an earlier test
        // that happened to reuse it, so the empty-buffer assertion below is
        // exact rather than "no worse than before".
        let _ = crate::pdf::render::take_xberg_native_pdf_render_warnings();

        let bytes = malformed_font_layout_fixture();
        let config = LayoutDetectionConfig {
            strategy: LayoutStrategy::Always,
            ..Default::default()
        };

        let (attempt, glyph_drop_warnings) = super::run_layout_for_pdf_pages_async(
            &bytes,
            &config,
            1,
            GatedPageHandling::SkipRender,
            &crate::extractors::security::SecurityLimits::default(),
            None,
        )
        .await
        .expect("malformed-font page must still render (and complete the layout pass) despite the glyph drop");

        assert!(
            attempt.output.data.is_some(),
            "the page rendered fine (the glyph was dropped, not the render), so layout data must be present"
        );

        assert_eq!(
            glyph_drop_warnings.len(),
            1,
            "the spawn_blocking closure must return exactly the one glyph-drop warning it captured on its own \
             thread, got: {glyph_drop_warnings:?}"
        );
        let warning = &glyph_drop_warnings[0];
        assert_eq!(
            warning.source, "pdf-render",
            "glyph-drop warnings must be sourced \"pdf-render\", got: {}",
            warning.source
        );
        assert!(
            warning.message.contains("Page 1"),
            "warning must name the affected page, got: {}",
            warning.message
        );
        assert!(
            warning.message.contains("rendering text with fallback font data"),
            "warning must carry xberg_native_pdf's sanitized static message; the actual \
             diagnosis now lives in structured tracing fields, not the message, got: {}",
            warning.message
        );

        // The critical assertion: the calling (test) thread's own thread-local
        // buffer must stay empty. The render happened on a blocking-pool
        // thread; before this fix, that thread's captured warning was never
        // returned to the caller and nothing else ever drained it, so it
        // would neither show up here nor above.
        let leaked_to_caller_thread = crate::pdf::render::take_xberg_native_pdf_render_warnings();
        assert!(
            leaked_to_caller_thread.is_empty(),
            "glyph-drop warnings must come back through the spawn_blocking closure's return value, not leak into \
             (or vanish from) the caller thread's own thread-local buffer: {leaked_to_caller_thread:?}"
        );
    }

    /// #1828: `render_layout_chunk`'s OCR-input render (`GatedPageHandling::RenderWithoutInference`,
    /// i.e. `normalize_for_ocr`) must make the same scan-density render-DPI decision the
    /// non-layout OCR routes make (`crate::image::dpi::pdf_ocr_render_dpi`), not the plain
    /// `effective_pdf_render_dpi` the markdown-structure render (`SkipRender`) uses. A scan page
    /// (one full-page 400x400px raster over a 100x100pt page, 288 dpi) renders its OCR input at
    /// that density -- 400x400px, not the 150 dpi default's ~208x208px -- while the same page's
    /// markdown-structure render is unaffected and still uses the 150 dpi default.
    ///
    /// Fails on unfixed code (plain `effective_pdf_render_dpi` for both handlings): the OCR
    /// render comes back ~208x208, equal to the markdown-structure render, instead of 400x400.
    #[test]
    fn ocr_render_uses_scan_density_while_markdown_render_keeps_the_default() {
        let scan_bytes = crate::pdf::render::build_full_page_raster_pdf((100.0, 100.0), (400, 400), 1.0, 0);
        let doc = xberg_native_pdf::PdfDocument::from_bytes(scan_bytes).expect("fixture must open");
        let page_rotations = vec![0u32];
        let security_limits = crate::extractors::security::SecurityLimits::default();
        let budget = RenderBudget {
            security_limits: &security_limits,
            images_config: None,
        };

        let ocr_pages = render_layout_chunk(
            &doc,
            &page_rotations,
            0,
            1,
            None,
            GatedPageHandling::RenderWithoutInference,
            budget,
        );
        let ocr_image = ocr_pages[0].image.as_ref().expect("scan page must render");
        assert_eq!(
            (ocr_image.width(), ocr_image.height()),
            (400, 400),
            "OCR input for a scan page must render at the raster's own 288 dpi density, not the 150 dpi default"
        );

        let markdown_pages =
            render_layout_chunk(&doc, &page_rotations, 0, 1, None, GatedPageHandling::SkipRender, budget);
        let markdown_image = markdown_pages[0].image.as_ref().expect("scan page must render");
        assert_eq!(
            (markdown_image.width(), markdown_image.height()),
            (209, 209),
            "the markdown-structure render must be unaffected and keep the 150 dpi default \
             (100pt * 150/72 rounds to 209px)"
        );
    }

    /// The crop is measured from the MediaBox's own corner, not the origin, and clamps to it.
    #[test]
    fn should_measure_visible_crop_rect_from_the_mediabox_corner() {
        use xberg_native_pdf::geometry::Rect;

        // MediaBox [-9 -9 621 801] rendered at 2 px per point.
        let media = Rect::new(-9.0, -9.0, 630.0, 810.0);
        let raster = (1_260, 1_620);
        assert_eq!(
            super::visible_crop_rect(media, Rect::new(41.0, 41.0, 530.0, 710.0), raster),
            Some((100, 100, 1_060, 1_420))
        );
        assert_eq!(
            super::visible_crop_rect(media, Rect::from_points(571.0, 751.0, 41.0, 41.0), raster),
            Some((100, 100, 1_060, 1_420)),
            "a reversed CropBox reads as the same box"
        );
        assert_eq!(
            super::visible_crop_rect(media, Rect::new(-50.0, 41.0, 621.0, 710.0), raster),
            Some((0, 100, 1_160, 1_420))
        );
        assert_eq!(super::visible_crop_rect(media, media, raster), None);
    }

    #[test]
    fn should_shift_inference_crop_detections_back_into_the_page_raster() {
        let mut raster = image::RgbImage::new(1_260, 1_620);
        raster.put_pixel(150, 130, image::Rgb([255, 0, 0]));
        let page = RenderedLayoutPage {
            image: Some(raster),
            inference_crop: Some((100, 100, 1_060, 1_420)),
            ..rendered_page(0, 1, 612.0)
        };
        let crop = super::inference_crop_image(&page).expect("the page has an inference crop");
        assert_eq!(crop.dimensions(), (1_060, 1_420));
        assert_eq!(crop.get_pixel(50, 30), &image::Rgb([255, 0, 0]));

        let mut detection = crate::layout::DetectionResult {
            page_width: 1_060,
            page_height: 1_420,
            detections: vec![crate::layout::LayoutDetection::new(
                crate::layout::LayoutClass::Formula,
                0.9,
                crate::layout::BBox::new(50.0, 30.0, 60.0, 40.0),
            )],
        };
        super::shift_into_page_frame(&page, &mut detection);
        let bbox = &detection.detections[0].bbox;
        assert_eq!((bbox.x1, bbox.y1, bbox.x2, bbox.y2), (150.0, 130.0, 160.0, 140.0));
        assert_eq!((detection.page_width, detection.page_height), (1_260, 1_620));

        let uncropped = rendered_page(0, 1_260, 612.0);
        assert!(super::inference_crop_image(&uncropped).is_none());
        super::shift_into_page_frame(&uncropped, &mut detection);
        assert_eq!(
            detection.detections[0].bbox.x1, 150.0,
            "an uncropped page keeps the model's frame"
        );
    }

    /// Only one batch's crops are live at a time; charging the whole chunk's crops refused this chunk.
    #[test]
    fn default_limits_split_eight_cropped_letter_pages_into_valid_subbatches() {
        let limits = crate::extractors::security::SecurityLimits::default();
        let letter_pages = |inference_crop| -> Vec<RenderedLayoutPage> {
            (0..8)
                .map(|page_index| RenderedLayoutPage {
                    page_height_pts: 792.0,
                    image: Some(image::RgbImage::new(1_275, 1_650)),
                    inference_crop,
                    ..rendered_page(page_index, 1, 612.0)
                })
                .collect()
        };
        let positions: Vec<usize> = (0..8).collect();
        let capacity_of = |pages: &[RenderedLayoutPage]| {
            let live = crate::layout::engine::live_raster_bytes(
                pages.iter().filter_map(|page| page.image.as_ref()),
                1_275,
                1_650,
            )
            .expect("eight page rasters");
            let capacity = super::inference_batch_capacity(pages, &positions, live, (1_275, 1_650), &limits)
                .expect("default limits must support at least one inference page");
            for batch in positions.chunks(capacity) {
                let crops = super::inference_crop_bytes(pages, batch).expect("crop bytes");
                crate::layout::engine::validate_layout_inference_peak(1_275, 1_650, live + crops, batch.len(), &limits)
                    .expect("every batch must fit default limits");
            }
            capacity
        };

        assert_eq!(
            capacity_of(&letter_pages(None)),
            2,
            "uncropped pages keep the engine's capacity"
        );
        assert_eq!(capacity_of(&letter_pages(Some((104, 104, 1_067, 1_333)))), 1);
    }
}
