//! PDF text extraction using the xberg_native_pdf backend.

use super::NativeDocument;
use super::span_geometry::{
    has_same_rotation, is_horizontal_ltr, is_ltr_writing_mode, is_unrotated, upright_advance_extent,
    upright_cross_extent,
};
use crate::core::config::{ExtractionConfig, PageConfig};
use crate::pdf::error::{PdfError, Result};
use crate::pdf::metadata::PdfExtractionMetadata;
use crate::pdf::structure::constants::{COALESCE_THRESHOLD, MAX_GLYPH_JITTER_PT, MIN_DISORDER_COUNT};
use crate::pdf::text::{contains_html_markup, fix_pdf_control_chars};
use crate::types::{PageBoundary, PageContent};
use std::borrow::Cow;
use std::collections::HashMap;
use xberg_native_pdf::document::ReadingOrder;

/// Per-page fabricated-mapping character counts `(fabricated, total)`, indexed by zero-based
/// page number, gathered while the main text pass reads each page's raw `ColumnAware` spans.
///
/// `None` when the document has default-off optional-content (OCG) layers: the text pass then
/// reads layer-*filtered* spans for content assembly, which is not the same span set
/// `scan_detect::page_has_fabricated_text` grades, so provenance falls back to reading each
/// page separately for such documents rather than reporting counts that would silently change
/// `fabricated_text_pages` (issue #1744).
type PageProvenanceCounts = Option<Vec<(usize, usize)>>;

/// Result type for PDF text extraction with optional page tracking.
type PdfTextExtractionResult = (
    String,
    Option<Vec<PageBoundary>>,
    Option<Vec<PageContent>>,
    PageProvenanceCounts,
);

// #1574: these were 0.06/0.05 through 1.1.0. `top_margin_fraction`/`bottom_margin_fraction`
// went from a dead config knob (unread before commit ddba546dca5) to an active OCR-paragraph
// filter in 1.1.0 without a changelog entry, so a default-config scan lost every page title
// that happened to sit in the top 6% band with no warning at all. Restoring 0.0 makes the
// filter opt-in: it now only runs when a caller sets a margin explicitly. ~keep
const DEFAULT_TOP_MARGIN_FRACTION: f32 = 0.0;
const DEFAULT_BOTTOM_MARGIN_FRACTION: f32 = 0.0;

#[derive(Debug, Clone, Copy)]
pub(crate) struct PageMarginFractions {
    pub(crate) top: f32,
    pub(crate) bottom: f32,
}

impl Default for PageMarginFractions {
    fn default() -> Self {
        Self {
            top: DEFAULT_TOP_MARGIN_FRACTION,
            bottom: DEFAULT_BOTTOM_MARGIN_FRACTION,
        }
    }
}

impl PageMarginFractions {
    pub(crate) fn from_extraction_config(config: Option<&ExtractionConfig>) -> Self {
        let defaults = Self::default();
        let top = config
            .and_then(|config| config.pdf_options.as_ref())
            .and_then(|pdf| pdf.top_margin_fraction)
            .unwrap_or(defaults.top);
        let bottom = config
            .and_then(|config| config.pdf_options.as_ref())
            .and_then(|pdf| pdf.bottom_margin_fraction)
            .unwrap_or(defaults.bottom);
        let include_headers = config
            .and_then(|config| config.content_filter.as_ref())
            .is_some_and(|filter| filter.include_headers);
        let include_footers = config
            .and_then(|config| config.content_filter.as_ref())
            .is_some_and(|filter| filter.include_footers);

        Self {
            top: if include_headers { 0.0 } else { top },
            bottom: if include_footers { 0.0 } else { bottom },
        }
    }
}

/// Result type for unified PDF text and metadata extraction.
///
/// Contains text, optional page boundaries, optional per-page content, and metadata.
pub type NativeUnifiedExtractionResult = (
    String,
    Option<Vec<PageBoundary>>,
    Option<Vec<PageContent>>,
    PdfExtractionMetadata,
);

/// Extract text and metadata from a PDF document in a single pass.
///
/// This is the native equivalent of `extract_text_and_metadata_from_pdf_document`.
/// It extracts both text and metadata in one pass through the document.
pub(crate) fn extract_text_and_metadata(
    doc: &mut NativeDocument,
    extraction_config: Option<&ExtractionConfig>,
) -> Result<NativeUnifiedExtractionResult> {
    let page_config = extraction_config.and_then(|c| c.pages.as_ref());
    let margins = PageMarginFractions::from_extraction_config(extraction_config);
    let (text, boundaries, page_contents, provenance_counts) =
        extract_text_from_native_document(doc, page_config, extraction_config, margins)?;

    let scanned_min_confidence = extraction_config
        .map(|c| c.ocr_strategy.effective_min_confidence())
        .unwrap_or(crate::core::config::DEFAULT_SCANNED_MIN_CONFIDENCE);
    let ocr_quality_thresholds = extraction_config
        .and_then(|c| c.ocr.as_ref())
        .and_then(|o| o.quality_thresholds.clone())
        .unwrap_or_default();
    let metadata = super::metadata::extract_metadata_from_native_document(
        doc,
        boundaries.as_deref(),
        &text,
        scanned_min_confidence,
        &ocr_quality_thresholds,
        provenance_counts.as_deref(),
    )?;

    Ok((text, boundaries, page_contents, metadata))
}

/// Extract text spans with bounding boxes from a single page.
///
/// Returns `(text_spans)` where each span contains the text, x, y, width, and height
/// in PDF coordinate space (points, y=0 at bottom of page).
///
/// This is used by reading-order reconstruction to project spans onto layout regions.
#[cfg(feature = "layout-detection")]
pub(crate) fn extract_spans_from_page(
    doc: &mut xberg_native_pdf::PdfDocument,
    page_index: usize,
    margins: PageMarginFractions,
) -> Result<(Vec<crate::extractors::pdf::rotation::TextSpan>, bool)> {
    use xberg_native_pdf::document::ReadingOrder;

    let mut page_text_data = super::guard_native_panic(
        || {
            doc.extract_page_text_with_options(page_index, ReadingOrder::ColumnAware)
                .map_err(|e| PdfError::TextExtractionFailed(format!("Failed to extract page text: {}", e)))
        },
        |panic| PdfError::TextExtractionFailed(format!("Page text extraction panicked in xberg_native_pdf: {}", panic)),
    )?;
    let (page_bottom, page_top) = page_vertical_bounds(doc, page_index)?;
    retain_spans_inside_page_margins(&mut page_text_data.spans, page_bottom, page_top, margins);
    let reordered_sparse_columns = reorder_sparse_two_column_page(&mut page_text_data.spans, page_text_data.page_width);

    let spans = page_text_data.spans.iter().map(rotation_span).collect();

    Ok((spans, reordered_sparse_columns))
}

/// Whether the text pass must hand out per-page byte offsets, independent of whether the
/// caller asked to collect `PageContent` itself.
///
/// Some OCR routes replace only *some* pages rather than the whole document, and those routes
/// need the boundaries this predicate gates:
///
/// - `force_ocr_pages` non-empty and an `ocr` block both already required boundaries before
///   GH#1752.
/// - `ocr_near_empty_fallback == Some(true)` and `ocr_scanned_page_quality_gate == Some(true)`
///   now also require them, even without an `ocr` block. Before this widening, a caller who set
///   either flag alone got no boundaries at all: `extractors/pdf/mod.rs` skips
///   `extract_mixed_ocr_native` entirely without them, the per-page text-quality gate collapses
///   to one document-wide verdict, and a single flagged page escalates to whole-document OCR --
///   the cost inversion GH#1752 exists to fix.
///
/// Deliberately does *not* check `ocr_embedded_images` (irrelevant to page boundaries -- it
/// gates whether embedded image bytes are read, not page-text offsets) or whether an automatic
/// OCR backend is registered: being wrong in the "too eager" direction here just tracks
/// boundaries nobody ends up using, which is silent-safe, while being wrong "too narrow" is the
/// bug this function fixes. ~keep
pub(crate) fn page_boundaries_required(config: &ExtractionConfig) -> bool {
    config.force_ocr_pages.as_ref().is_some_and(|pages| !pages.is_empty())
        || config.ocr.is_some()
        || config.ocr_near_empty_fallback == Some(true)
        || config.ocr_scanned_page_quality_gate == Some(true)
}

/// Extract text from a xberg_native_pdf document with optional page boundary tracking.
///
/// Mirrors the signature and behaviour of `extract_text_from_pdf_document`.
///
/// When `page_config` is `Some`, tracks byte offsets and optionally collects
/// per-page `PageContent` entries.
///
/// When `page_config` is `None` but `extraction_config` requires per-page boundaries, per
/// [`page_boundaries_required`], boundary tracking is enabled automatically with a default
/// `PageConfig` so that the mixed-OCR and quality-threshold codepaths receive the offsets they
/// need.
///
/// Otherwise the fast path is used (no per-page tracking).
pub(crate) fn extract_text_from_native_document(
    doc: &mut NativeDocument,
    page_config: Option<&PageConfig>,
    extraction_config: Option<&ExtractionConfig>,
    margins: PageMarginFractions,
) -> Result<PdfTextExtractionResult> {
    let needs_boundaries = extraction_config.is_some_and(page_boundaries_required);
    let cancel_token = extraction_config.and_then(|config| config.cancel_token.as_ref());

    if let Some(config) = page_config {
        extract_text_with_tracking(doc, config, margins, cancel_token)
    } else if needs_boundaries {
        let default_config = PageConfig::default();
        extract_text_with_tracking(doc, &default_config, margins, cancel_token)
    } else {
        extract_text_fast_path(doc, margins, cancel_token)
    }
}

/// The blank line written between two pages when no page marker is configured.
///
/// `extractors::pdf::extraction::join_pages_with_boundaries` re-joins the same pages after
/// reading-order reordering and has to produce the same offsets, so it reads this rather
/// than repeating the literal. ~keep
pub(crate) const PAGE_SEPARATOR: &str = "\n\n";

/// Extract and clean one page's text, alongside its fabricated-mapping counts when available.
///
/// See [`PageProvenanceCounts`] for when the second element is `None`.
fn extract_one_page_text(
    doc: &xberg_native_pdf::PdfDocument,
    page_index: usize,
    excluded_layers: &std::collections::HashSet<String>,
    margins: PageMarginFractions,
) -> Result<(String, Option<(usize, usize)>)> {
    let (page_text, provenance_counts) = extract_page_text_column_aware(doc, page_index, excluded_layers, margins)?;
    Ok((apply_text_cleanup(&page_text).into_owned(), provenance_counts))
}

/// Extract every page's cleaned text, in page order.
///
/// Pages are read in ascending order and that order is load-bearing, not incidental. A
/// page's text depends on which pages were read before it: the document handle shares
/// resolved font sets and TrueType CMaps between pages through its own caches, and where
/// two subsets of one base font disagree about a glyph id the first one loaded wins (see
/// `share_truetype_cmaps` and the font caches in `xberg-native-pdf`'s `document::fonts`).
/// Reading the pages in any other order, including concurrently, silently changes the
/// extracted text. Measured on a 731-page document: ascending order reproduces the same
/// bytes on every run, while parsing the same pages two at a time over one handle drops
/// text and lands on a different result each run. Removing that order dependence is
/// GH#1725; until it is gone this loop must stay in page order. ~keep
///
/// Also returns each page's fabricated-mapping character counts (see [`PageProvenanceCounts`]),
/// captured from the same raw `ColumnAware` spans this loop already reads for content, so the
/// provenance pass in `pdf/scan_detect.rs` no longer has to read every page a second time
/// (issue #1744).
fn extract_all_page_texts(
    doc: &xberg_native_pdf::PdfDocument,
    margins: PageMarginFractions,
    cancel_token: Option<&crate::cancellation::CancellationToken>,
) -> Result<(Vec<String>, PageProvenanceCounts)> {
    let page_count = doc
        .page_count()
        .map_err(|e| PdfError::TextExtractionFailed(format!("Failed to get page count: {}", e)))?;

    // Issue #67: default-off optional-content (OCG/layer) groups per
    // `/OCProperties/D` (ISO 32000-1:2008 §8.11.4). Computed once per
    // document; empty for the common case of no `/OCProperties`.
    let excluded_layers = xberg_native_pdf::optional_content::compute_default_off_ocgs(doc);
    let mut provenance_counts = excluded_layers.is_empty().then(|| Vec::with_capacity(page_count));

    let mut texts = Vec::with_capacity(page_count);
    for page_idx in 0..page_count {
        if cancel_token.is_some_and(crate::cancellation::CancellationToken::is_cancelled) {
            return Err(PdfError::Cancelled);
        }
        let (text, counts) = extract_one_page_text(doc, page_idx, &excluded_layers, margins)?;
        texts.push(text);
        if let Some(collected) = provenance_counts.as_mut() {
            collected
                .push(counts.expect("extract_one_page_text must report provenance counts when no layers are excluded"));
        }
    }

    Ok((texts, provenance_counts))
}

/// Fast path: extract text without page tracking.
///
/// Extracts every page through [`extract_all_page_texts`], then concatenates the
/// pages in order into a single string.
fn extract_text_fast_path(
    doc: &NativeDocument,
    margins: PageMarginFractions,
    cancel_token: Option<&crate::cancellation::CancellationToken>,
) -> Result<PdfTextExtractionResult> {
    let (page_texts, provenance_counts) = extract_all_page_texts(&doc.doc, margins, cancel_token)?;

    let separators = page_texts.len().saturating_sub(1) * PAGE_SEPARATOR.len();
    let mut content = String::with_capacity(page_texts.iter().map(String::len).sum::<usize>() + separators);

    for (page_idx, page_text) in page_texts.iter().enumerate() {
        if page_idx > 0 {
            content.push_str(PAGE_SEPARATOR);
        }
        content.push_str(page_text);
    }

    Ok((content, None, None, provenance_counts))
}

/// Extract text with page boundary and content tracking.
///
/// Mirrors `extract_text_lazy_with_tracking`: tracks byte
/// offsets for each page, optionally collects per-page `PageContent`, and inserts
/// page markers when configured.
fn extract_text_with_tracking(
    doc: &NativeDocument,
    config: &PageConfig,
    margins: PageMarginFractions,
    cancel_token: Option<&crate::cancellation::CancellationToken>,
) -> Result<PdfTextExtractionResult> {
    let (page_texts, provenance_counts) = extract_all_page_texts(&doc.doc, margins, cancel_token)?;
    let page_count = page_texts.len();

    let markers: Vec<String> = if config.insert_page_markers {
        (1..=page_count)
            .map(|page_number| config.marker_format.replace("{page_num}", &page_number.to_string()))
            .collect()
    } else {
        Vec::new()
    };

    let separators: usize = if config.insert_page_markers {
        markers.iter().map(String::len).sum()
    } else {
        page_count.saturating_sub(1) * PAGE_SEPARATOR.len()
    };

    let mut content = String::with_capacity(page_texts.iter().map(String::len).sum::<usize>() + separators);
    let mut boundaries = Vec::with_capacity(page_count);
    let mut page_contents = if config.extract_pages {
        Some(Vec::with_capacity(page_count))
    } else {
        None
    };

    for (page_idx, cleaned) in page_texts.into_iter().enumerate() {
        let page_number = page_idx + 1;

        if config.insert_page_markers {
            content.push_str(&markers[page_idx]);
        } else if page_idx > 0 {
            content.push_str(PAGE_SEPARATOR);
        }

        let byte_start = content.len();
        content.push_str(&cleaned);
        let byte_end = content.len();

        boundaries.push(PageBoundary {
            byte_start,
            byte_end,
            page_number: page_number as u32,
        });

        if let Some(ref mut pages) = page_contents {
            let is_blank = Some(crate::extraction::blank_detection::is_page_text_blank(&cleaned));
            pages.push(PageContent {
                page_number: page_number as u32,
                content: cleaned,
                tables: Vec::new(),
                image_indices: Vec::new(),
                image_preprocessing: None,
                hierarchy: None,
                is_blank,
                layout_regions: None,
                speaker_notes: None,
                section_name: None,
                sheet_name: None,
                ocr_confidence: None,
                native_content: None,
            });
        }
    }

    Ok((content, Some(boundaries), page_contents, provenance_counts))
}

/// Collect Widget annotation field values for the given page, sorted top-to-bottom.
///
/// Returns `(mid_y_pdf, value_text)` pairs. `mid_y_pdf` is the vertical midpoint of
/// the Widget's bounding rectangle in PDF page coordinates (Y=0 at bottom of page,
/// higher values are higher on the page). The list is sorted descending by Y so that
/// entries nearer the top of the page come first, preserving visual reading order when
/// the values are appended to the assembled span text.
///
/// Empty values and annotations without a `/V` entry are excluded. This function is
/// intentionally infallible: a failed `get_annotations` call is logged at DEBUG level
/// and returns an empty list so that the rest of the extraction path is unaffected.
fn collect_widget_field_values(doc: &xberg_native_pdf::PdfDocument, page_index: usize) -> Vec<(f64, String)> {
    let annotations = match doc.get_annotations(page_index) {
        Ok(a) => a,
        Err(e) => {
            tracing::debug!(
                page = page_index,
                "xberg_native_pdf: could not read annotations for widget values: {e}"
            );
            return Vec::new();
        }
    };

    let mut widgets: Vec<(f64, String)> = annotations
        .into_iter()
        .filter(|a| a.subtype_enum == xberg_native_pdf::AnnotationSubtype::Widget)
        .filter_map(|a| {
            let value = a.field_value?.trim().to_string();
            if value.is_empty() {
                return None;
            }
            let mid_y = a.rect.map_or(f64::NEG_INFINITY, |r| (r[1] + r[3]) / 2.0);
            Some((mid_y, value))
        })
        .collect();

    widgets.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    widgets
}

/// Append Widget form-field values that are absent from `text`.
///
/// Handles interactive (non-flattened) PDFs where field values live only in Widget `/V`
/// entries and are absent from the page content stream. Values already present in `text`
/// (e.g. flattened PDFs where the appearance stream was rendered into the content stream)
/// are skipped to prevent duplication.
///
/// Deduplication uses substring matching: if `value` appears anywhere in `text` the field
/// is skipped. This is intentionally simple — the common case is a verbatim match between
/// the rendered appearance text and the Widget `/V` string. It can produce false negatives
/// when the field value is a substring of surrounding prose (e.g. value "Smith" suppressed
/// when content already contains "John Smith"). This is an acceptable trade-off to avoid
/// duplicating values in flattened PDFs; tighter word-boundary deduplication can be added
/// when evidence of real-world false negatives is available.
///
/// Values are appended after all content-stream text, not interleaved at their bounding-box
/// positions. This is the intended ordering for the initial implementation: interactive
/// PDFs rarely have dense label+value proximity requirements, and span-level interleaving
/// would require re-sorting the column-aware span list which is not guaranteed to be
/// monotonically ordered by Y.
///
/// Appends in top-to-bottom page order (descending by annotation mid-Y).
fn append_missing_widget_values(text: &mut String, widgets: &[(f64, String)]) {
    for (_, value) in widgets {
        if !text.contains(value.as_str()) {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(value);
        }
    }
}

/// Returns true when `spans` exhibits the glyph-fragmentation signature (issue #962).
///
/// See `crate::pdf::structure::constants` for the threshold values and their justification.
///
/// xberg_native_pdf's ColumnAware reading order groups all spans at one y-level before moving
/// to the next. For Word-exported PDFs where each glyph has its own BT…ET block with a
/// sinusoidal y-jitter, this produces groups ordered by y-level rather than by reading
/// order: "et" (y=703) appears before "H" (y=700) even though "H" comes first visually.
///
/// Two-part signature:
/// 1. Both spans are short (≤ 3 chars): per-glyph BT/ET always produces single-character
///    spans; multi-character spans are word-level and cannot be glyph artifacts.
/// 2. The spans are on the same visual line (y-gap ≤ MAX_GLYPH_JITTER_PT when heights
///    are zero, or < half the measured height otherwise) yet the x-coordinate resets
///    significantly leftward — indicating a new y-group started mid-reading-order.
///
/// ≥ MIN_DISORDER_COUNT such events means position-based reconstruction is needed.
fn is_fragmented_span_list(spans: &[xberg_native_pdf::layout::TextSpan]) -> bool {
    let mut disorder_count = 0;
    for window in spans.windows(2) {
        let prev = &window[0];
        let cur = &window[1];

        if prev.text.chars().count() > 3 || cur.text.chars().count() > 3 {
            continue;
        }

        let y_gap = (prev.bbox.y - cur.bbox.y).abs();

        let eff_height = prev.bbox.height.max(cur.bbox.height);
        let same_line = if eff_height > 0.0 {
            y_gap < eff_height * 0.5
        } else {
            y_gap <= MAX_GLYPH_JITTER_PT
        };

        if same_line && cur.bbox.x < prev.bbox.x - prev.font_size {
            disorder_count += 1;
            if disorder_count >= MIN_DISORDER_COUNT {
                return true;
            }
        }
    }
    false
}

/// Rebuild readable text from a glyph-fragmented span list (issue #962).
///
/// Algorithm:
/// 1. Sort spans by y-descending (top-of-page first in PDF coordinates).
/// 2. Group by chained y-proximity: consecutive spans within COALESCE_THRESHOLD pt
///    of the previous span belong to the same visual line.
/// 3. Within each group sort by x-ascending (left-to-right reading order).
/// 4. Concatenate, inserting a space wherever the x-gap between adjacent spans
///    exceeds font_size * 0.5.
fn rebuild_text_from_fragmented_spans(spans: &[xberg_native_pdf::layout::TextSpan]) -> String {
    if spans.is_empty() {
        return String::new();
    }

    let notes = numeric_notes(spans);
    let mut sorted: Vec<&xberg_native_pdf::layout::TextSpan> = spans.iter().collect();
    sorted.sort_by(|a, b| b.bbox.y.partial_cmp(&a.bbox.y).unwrap_or(std::cmp::Ordering::Equal));

    let mut groups: Vec<Vec<&xberg_native_pdf::layout::TextSpan>> = Vec::new();
    for span in sorted {
        let belongs = groups.last().is_some_and(|g| {
            let prev_y = g.last().unwrap().bbox.y;
            (span.bbox.y - prev_y).abs() <= COALESCE_THRESHOLD
        });
        if belongs {
            groups.last_mut().unwrap().push(span);
        } else {
            groups.push(vec![span]);
        }
    }

    let mut result = String::new();
    for (gi, group) in groups.iter_mut().enumerate() {
        group.sort_by(|a, b| a.bbox.x.partial_cmp(&b.bbox.x).unwrap_or(std::cmp::Ordering::Equal));
        if gi > 0 {
            result.push('\n');
        }
        let font_size = group.iter().map(|s| s.font_size).fold(0.0_f32, f32::max);
        let space_threshold = font_size * 0.5;
        let mut prev_end_x = f32::NEG_INFINITY;
        let mut previous = None;
        for (index, span) in group.iter().enumerate() {
            if (prev_end_x.is_finite() && span.bbox.x - prev_end_x > space_threshold)
                || previous.is_some_and(|prev| {
                    needs_numeric_script_boundary(prev, span, &notes, group.get(index + 1).copied())
                })
            {
                result.push(' ');
            }
            result.push_str(&span.text);
            prev_end_x = span.bbox.x + span.bbox.width;
            previous = Some(*span);
        }
    }
    result
}

// A small digit beside another digit can be a footnote or a power. Require a
// matching, smaller note below the reference before changing an existing join.
// Inspect only nearby spans and keep one definition per number, so dense pages
// do not turn this into an all-pairs scan.
pub(super) fn numeric_notes(
    spans: &[xberg_native_pdf::layout::TextSpan],
) -> HashMap<&str, &xberg_native_pdf::layout::TextSpan> {
    let mut notes: HashMap<&str, &xberg_native_pdf::layout::TextSpan> = HashMap::new();
    for (index, marker) in spans.iter().enumerate() {
        let number = marker.text.trim();
        if number.is_empty()
            || number.len() > 3
            || !number.bytes().all(|c| c.is_ascii_digit())
            || !is_unrotated(marker)
            || !is_ltr_writing_mode(marker)
        {
            continue;
        }
        let Some(body) = spans
            .iter()
            .skip(index + 1)
            .take(4)
            .find(|span| !span.text.trim().is_empty())
        else {
            continue;
        };
        let gap = body.bbox.x - (marker.bbox.x + marker.bbox.width);
        if !is_unrotated(body)
            || !is_ltr_writing_mode(body)
            || !body.text.trim_start().starts_with(char::is_alphabetic)
            || body.text.split_whitespace().count() < 2
            || marker.font_size > body.font_size
            || gap < -0.1 * body.font_size
            || gap > body.font_size
            || (marker.bbox.y - body.bbox.y).abs() > body.font_size * 0.75
        {
            continue;
        }
        notes
            .entry(number)
            .and_modify(|previous| {
                if body.bbox.y < previous.bbox.y {
                    *previous = body;
                }
            })
            .or_insert(body);
    }
    notes
}

pub(super) fn needs_numeric_script_boundary(
    base: &xberg_native_pdf::layout::TextSpan,
    script: &xberg_native_pdf::layout::TextSpan,
    notes: &HashMap<&str, &xberg_native_pdf::layout::TextSpan>,
    following: Option<&xberg_native_pdf::layout::TextSpan>,
) -> bool {
    let Some((prose, number)) = base.text.trim_end().rsplit_once(char::is_whitespace) else {
        return false;
    };
    let Some(note) = notes.get(script.text.as_str()) else {
        return false;
    };
    if base.text.ends_with(char::is_whitespace)
        || !prose.chars().any(char::is_alphabetic)
        || !number.chars().all(|c| c.is_ascii_digit())
        || !is_unrotated(base)
        || !is_unrotated(script)
        || !is_ltr_writing_mode(base)
        || !is_ltr_writing_mode(script)
        || note.font_size >= base.font_size * 0.9
        || base.bbox.y - note.bbox.y < base.font_size * 1.5
        || (script.font_size >= base.font_size * 0.8 && script.text_rise.abs() < 0.10)
    {
        return false;
    }
    // A matching footnote number can also occur as a power on the same page.
    // An adjacent operator is evidence of an expression, not a note reference.
    if following.is_some_and(|next| {
        next.text.trim_start().starts_with(['+', '-', '−', '×', '÷', '=', '^'])
            && (next.bbox.y - base.bbox.y).abs() <= base.font_size * 0.5
            && (next.bbox.x - (script.bbox.x + script.bbox.width)).abs() <= base.font_size
    }) {
        return false;
    }
    let gap = script.bbox.x - (base.bbox.x + base.bbox.width);
    gap >= -0.1 * base.font_size
        && gap <= 0.25 * base.font_size
        && (base.bbox.y - script.bbox.y).abs() <= base.font_size * 0.5
}

const INLINE_FRAGMENT_GAP_RATIO: f32 = 0.1;
// Detached glyphs are stream-local; bounding the lookup avoids quadratic work on dense pages.
const MAX_INLINE_FRAGMENT_ANCHOR_LOOKBACK: usize = 256;
const ROW_RESET_MIN_BACKTRACK_EMS: f32 = 4.0;

#[derive(Clone, Copy)]
struct OrderedSpan<'a> {
    span: &'a xberg_native_pdf::layout::TextSpan,
    glue_to_previous: bool,
    following: Option<&'a xberg_native_pdf::layout::TextSpan>,
}

/// Do the two spans share a line?
///
/// Measured on each span's own cross axis so that a 90-degree rotated pair,
/// whose shared baseline is a page-x column rather than a page-y row, is still
/// recognised as one line. Identical to the previous page-y test for unrotated
/// spans. Only meaningful for spans of equal rotation; callers check that.
fn spans_overlap_on_cross_axis(
    first: &xberg_native_pdf::layout::TextSpan,
    second: &xberg_native_pdf::layout::TextSpan,
) -> bool {
    let (first_low, first_high) = upright_cross_extent(first);
    let (second_low, second_high) = upright_cross_extent(second);
    first_high.min(second_high) > first_low.max(second_low)
}

fn is_short_inline_fragment(span: &xberg_native_pdf::layout::TextSpan) -> bool {
    let mut chars = span.text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    let char_count = 1 + chars.count();
    if char_count > 3 || span.text.chars().all(char::is_whitespace) {
        return false;
    }
    !(char_count == 1 && matches!(first, 'a' | 'A' | 'I'))
}

fn has_rtl_or_bidi_content(text: &str) -> bool {
    text.chars()
        .any(|character| xberg_native_pdf::text::is_rtl_text(character as u32))
}

/// Find the parent word a short detached fragment should rejoin.
///
/// Gated on the writing mode only (`wmode` / `rtl_draw_logical`). Rotation is
/// deliberately *not* a reason to refuse the join: a rotated table header is
/// horizontal LTR text painted along a rotated baseline, and refusing to anchor
/// its fragments is what leaves rotated tables glued and word-reversed
/// (GitHub #1358). The candidate must still carry the *same* rotation as the
/// fragment, and all gap arithmetic runs in that rotation's upright frame.
fn find_inline_fragment_anchor(
    index: usize,
    spans: &[xberg_native_pdf::layout::TextSpan],
    anchors: &[Option<usize>],
) -> Option<usize> {
    let span = &spans[index];
    if span.split_boundary_before
        || !is_short_inline_fragment(span)
        || !is_ltr_writing_mode(span)
        || has_rtl_or_bidi_content(&span.text)
    {
        return None;
    }

    let (span_start, _) = upright_advance_extent(span);
    let search_start = index.saturating_sub(MAX_INLINE_FRAGMENT_ANCHOR_LOOKBACK);
    (search_start..index)
        .filter(|candidate_index| anchors[*candidate_index].is_none())
        .filter_map(|candidate_index| {
            let candidate = &spans[candidate_index];
            if !is_ltr_writing_mode(candidate)
                || has_rtl_or_bidi_content(&candidate.text)
                || !has_same_rotation(candidate, span)
                || !spans_overlap_on_cross_axis(candidate, span)
            {
                return None;
            }
            let (_, candidate_end) = upright_advance_extent(candidate);
            let gap = span_start - candidate_end;
            let tolerance = candidate.font_size.max(span.font_size) * INLINE_FRAGMENT_GAP_RATIO;
            (gap >= -tolerance && gap <= tolerance).then_some((candidate_index, gap.abs()))
        })
        .min_by(|(_, first_gap), (_, second_gap)| {
            first_gap.partial_cmp(second_gap).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(candidate_index, _)| candidate_index)
}

fn order_spans_with_inline_fragments<'a>(
    spans: &'a [xberg_native_pdf::layout::TextSpan],
    notes: &HashMap<&str, &xberg_native_pdf::layout::TextSpan>,
) -> Vec<OrderedSpan<'a>> {
    let mut anchors = vec![None; spans.len()];
    for index in 0..spans.len() {
        anchors[index] = find_inline_fragment_anchor(index, spans, &anchors);
    }

    let mut children = vec![Vec::new(); spans.len()];
    for (index, anchor) in anchors.iter().enumerate() {
        if let Some(anchor) = anchor {
            children[*anchor].push(index);
        }
    }
    for attached in &mut children {
        attached.sort_by(|first, second| {
            // Along each fragment's own advance axis, so rotated fragments are
            // re-inserted in reading order rather than page-x order.
            let (first_start, _) = upright_advance_extent(&spans[*first]);
            let (second_start, _) = upright_advance_extent(&spans[*second]);
            first_start
                .partial_cmp(&second_start)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    let mut ordered = Vec::with_capacity(spans.len());
    for (index, span) in spans.iter().enumerate() {
        if anchors[index].is_some() {
            continue;
        }
        ordered.push(OrderedSpan {
            span,
            glue_to_previous: false,
            following: spans.get(index + 1),
        });
        ordered.extend(children[index].iter().map(|child| OrderedSpan {
            span: &spans[*child],
            glue_to_previous: !needs_numeric_script_boundary(span, &spans[*child], notes, spans.get(*child + 1)),
            following: spans.get(*child + 1),
        }));
    }
    ordered
}

fn is_ltr_pair(previous: &xberg_native_pdf::layout::TextSpan, span: &xberg_native_pdf::layout::TextSpan) -> bool {
    is_ltr_writing_mode(previous)
        && is_ltr_writing_mode(span)
        && !has_rtl_or_bidi_content(&previous.text)
        && !has_rtl_or_bidi_content(&span.text)
}

fn has_rtl_content(previous: &xberg_native_pdf::layout::TextSpan, span: &xberg_native_pdf::layout::TextSpan) -> bool {
    has_rtl_or_bidi_content(&previous.text)
        || has_rtl_or_bidi_content(&span.text)
        || previous.rtl_draw_logical
        || span.rtl_draw_logical
}

fn append_span_separator(
    text: &mut String,
    previous: &xberg_native_pdf::layout::TextSpan,
    current: OrderedSpan<'_>,
    paragraph_gap_threshold: f32,
    allow_ltr_row_resets: bool,
    notes: &HashMap<&str, &xberg_native_pdf::layout::TextSpan>,
) {
    if current.glue_to_previous {
        return;
    }

    let span = current.span;

    // A change of text-matrix rotation is a hard block boundary. xberg_native_pdf lifts
    // rotated runs out of the horizontal flow and appends them as their own
    // blocks, and the two bboxes are flattened onto different axes, so no gap
    // arithmetic across the boundary is meaningful. This is also what keeps an
    // upright running footer readable on a page whose body is rotated.
    if !has_same_rotation(previous, span) {
        text.push_str("\n\n");
        return;
    }

    // Everything below runs in the pair's shared upright frame: identical to the
    // raw page axes when the pair is unrotated, axis-swapped when it is not.
    let (previous_start, previous_end) = upright_advance_extent(previous);
    let (span_start, span_end) = upright_advance_extent(span);
    let (previous_baseline, _) = upright_cross_extent(previous);
    let (span_baseline, _) = upright_cross_extent(span);
    let baseline_gap = (previous_baseline - span_baseline).abs();

    let reset_threshold = previous.font_size.max(span.font_size) * ROW_RESET_MIN_BACKTRACK_EMS;
    if allow_ltr_row_resets && is_ltr_pair(previous, span) && span_start < previous_start - reset_threshold {
        if baseline_gap > paragraph_gap_threshold {
            text.push_str("\n\n");
        } else {
            text.push('\n');
        }
        return;
    }

    if needs_numeric_script_boundary(previous, span, notes, current.following) {
        text.push(' ');
        return;
    }

    if span.split_boundary_before {
        if !previous.text.ends_with(char::is_whitespace) && !span.text.starts_with(char::is_whitespace) {
            text.push(' ');
        }
        return;
    }

    let effective_height = span.bbox.height.max(previous.bbox.height).max(span.font_size * 0.5);
    if baseline_gap < effective_height * 0.5 {
        let visual_gap = if span_start < previous_start && has_rtl_content(previous, span) {
            previous_start - span_end
        } else {
            span_start - previous_end
        };
        if visual_gap > span.font_size * 0.15 {
            text.push(' ');
        }
    } else if baseline_gap > paragraph_gap_threshold {
        text.push_str("\n\n");
    } else {
        text.push('\n');
    }
}

fn assemble_page_text(spans: &[xberg_native_pdf::layout::TextSpan]) -> String {
    let mut heights: Vec<f32> = spans.iter().map(|span| span.bbox.height).collect();
    heights.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_height = if heights.is_empty() {
        1.0
    } else {
        heights[heights.len() / 2]
    };
    let paragraph_gap_threshold = median_height * 1.5;

    tracing::debug!(
        span_count = spans.len(),
        median_height,
        paragraph_gap_threshold,
        "paragraph break detection initialized"
    );

    let notes = numeric_notes(spans);
    let ordered = order_spans_with_inline_fragments(spans, &notes);
    let allow_ltr_row_resets = !spans
        .iter()
        .any(|span| span.rtl_draw_logical || has_rtl_or_bidi_content(&span.text));
    let mut text = String::with_capacity(spans.len() * 20);
    let mut prev_span: Option<&xberg_native_pdf::layout::TextSpan> = None;

    for current in ordered {
        let span = current.span;
        if let Some(prev) = prev_span {
            append_span_separator(
                &mut text,
                prev,
                current,
                paragraph_gap_threshold,
                allow_ltr_row_resets,
                &notes,
            );
        }
        text.push_str(&span.text);
        prev_span = Some(span);
    }

    text
}

// xberg_native_pdf's XY-Cut does not split regions with fewer than five spans.
// These guards cover the issue #1345 four-span sentence without reclassifying
// sparse tables or forms as prose columns.
const MIN_SPARSE_COLUMN_GUTTER_FRACTION: f32 = 0.05;
const MIN_SPARSE_COLUMN_GUTTER_PTS: f32 = 15.0;
const MIN_SPARSE_COLUMN_CONTENT_WIDTH_PTS: f32 = 144.0;
const MIN_SPARSE_COLUMN_WORDS: usize = 2;
const MIN_SPARSE_COLUMN_WORDS_PER_SIDE: usize = 6;
const MIN_SPARSE_COLUMN_ALPHA_CHARS: usize = 8;
const MIN_SPARSE_COLUMN_ALPHA_RATIO: f32 = 0.55;
const MIN_SPARSE_COLUMN_VERTICAL_OVERLAP: f32 = 0.5;
const XY_CUT_MIN_SPANS_FOR_SPLIT: usize = 5;

fn is_sparse_column_prose(span: &xberg_native_pdf::layout::TextSpan) -> bool {
    let alpha_chars = span.text.chars().filter(|character| character.is_alphabetic()).count();
    let non_whitespace_chars = span.text.chars().filter(|character| !character.is_whitespace()).count();
    let word_count = span.text.split_whitespace().count();
    let geometry_is_valid = span.bbox.x.is_finite()
        && span.bbox.y.is_finite()
        && span.bbox.width.is_finite()
        && span.bbox.height.is_finite()
        && span.bbox.width > 0.0;

    geometry_is_valid
        && !span.is_monospace
        && is_horizontal_ltr(span)
        && !has_rtl_or_bidi_content(&span.text)
        && !span.text.contains(':')
        && word_count >= MIN_SPARSE_COLUMN_WORDS
        && alpha_chars >= MIN_SPARSE_COLUMN_ALPHA_CHARS
        && alpha_chars as f32 / non_whitespace_chars.max(1) as f32 >= MIN_SPARSE_COLUMN_ALPHA_RATIO
}

fn sparse_columns_overlap(
    left: &[&xberg_native_pdf::layout::TextSpan],
    right: &[&xberg_native_pdf::layout::TextSpan],
) -> bool {
    let extent = |side: &[&xberg_native_pdf::layout::TextSpan]| {
        side.iter()
            .map(|span| span.bbox.y)
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), y| {
                (low.min(y), high.max(y))
            })
    };
    let (left_low, left_high) = extent(left);
    let (right_low, right_high) = extent(right);
    let overlap = (left_high.min(right_high) - left_low.max(right_low)).max(0.0);
    let shorter_extent = (left_high - left_low).min(right_high - right_low);

    shorter_extent > 0.0 && overlap / shorter_extent >= MIN_SPARSE_COLUMN_VERTICAL_OVERLAP
}

fn sparse_columns_continue_one_sentence(
    left: &[&xberg_native_pdf::layout::TextSpan],
    right: &[&xberg_native_pdf::layout::TextSpan],
) -> bool {
    let mut left_by_y = left.to_vec();
    let mut right_by_y = right.to_vec();
    left_by_y.sort_by(|first, second| second.bbox.y.total_cmp(&first.bbox.y));
    right_by_y.sort_by(|first, second| second.bbox.y.total_cmp(&first.bbox.y));
    let starts_lowercase = |span: &&xberg_native_pdf::layout::TextSpan| {
        span.text
            .chars()
            .find(|character| character.is_alphabetic())
            .is_some_and(char::is_lowercase)
    };
    let starts_uppercase = |span: &&xberg_native_pdf::layout::TextSpan| {
        span.text
            .chars()
            .find(|character| character.is_alphabetic())
            .is_some_and(char::is_uppercase)
    };
    let has_terminal = |span: &&xberg_native_pdf::layout::TextSpan| span.text.trim_end().ends_with(['.', '!', '?']);
    let continuations = [&left_by_y[1], &right_by_y[0], &right_by_y[1]];
    let all_spans = left_by_y.iter().chain(&right_by_y);

    starts_uppercase(&left_by_y[0])
        && continuations.into_iter().all(starts_lowercase)
        && all_spans.clone().filter(|span| has_terminal(span)).count() == 1
        && has_terminal(&right_by_y[1])
}

fn is_sparse_column_split(spans: &[xberg_native_pdf::layout::TextSpan], split_x: f32, min_gutter: f32) -> bool {
    let left: Vec<_> = spans.iter().filter(|span| span.bbox.x < split_x).collect();
    let right: Vec<_> = spans.iter().filter(|span| span.bbox.x >= split_x).collect();
    if left.len() != 2 || right.len() != 2 {
        return false;
    }
    let word_count = |side: &[&xberg_native_pdf::layout::TextSpan]| {
        side.iter()
            .map(|span| span.text.split_whitespace().count())
            .sum::<usize>()
    };
    if word_count(&left) < MIN_SPARSE_COLUMN_WORDS_PER_SIDE || word_count(&right) < MIN_SPARSE_COLUMN_WORDS_PER_SIDE {
        return false;
    }
    let left_right = left
        .iter()
        .map(|span| span.bbox.x + span.bbox.width)
        .fold(f32::NEG_INFINITY, f32::max);

    split_x - left_right >= min_gutter
        && sparse_columns_overlap(&left, &right)
        && sparse_columns_continue_one_sentence(&left, &right)
}

fn sparse_column_split(spans: &[xberg_native_pdf::layout::TextSpan], page_width: f32) -> Option<f32> {
    let has_sparse_prose_shape =
        spans.len() == XY_CUT_MIN_SPANS_FOR_SPLIT - 1 && spans.iter().all(is_sparse_column_prose);
    let content_left = spans.iter().map(|span| span.bbox.x).fold(f32::INFINITY, f32::min);
    let content_right = spans
        .iter()
        .map(|span| span.bbox.x + span.bbox.width)
        .fold(f32::NEG_INFINITY, f32::max);
    if !has_sparse_prose_shape || content_right - content_left < MIN_SPARSE_COLUMN_CONTENT_WIDTH_PTS {
        return None;
    }
    let min_gutter = (page_width * MIN_SPARSE_COLUMN_GUTTER_FRACTION).max(MIN_SPARSE_COLUMN_GUTTER_PTS);
    let mut starts: Vec<f32> = spans.iter().map(|span| span.bbox.x).collect();
    starts.sort_by(f32::total_cmp);
    starts.dedup_by(|left, right| (*left - *right).abs() <= f32::EPSILON);

    starts
        .into_iter()
        .find(|&split_x| is_sparse_column_split(spans, split_x, min_gutter))
}

/// Reorder the guarded four-span, two-column sentence shape.
///
/// Returns `true` only when the sparse prose classifier matched and reordered
/// the spans. Callers use this signal to preserve the result across a broad
/// single layout hint.
pub(crate) fn reorder_sparse_two_column_page(
    spans: &mut [xberg_native_pdf::layout::TextSpan],
    page_width: f32,
) -> bool {
    let Some(split_x) = sparse_column_split(spans, page_width) else {
        return false;
    };
    spans.sort_by(|left, right| {
        let left_column = usize::from(left.bbox.x >= split_x);
        let right_column = usize::from(right.bbox.x >= split_x);
        left_column
            .cmp(&right_column)
            .then_with(|| right.bbox.y.total_cmp(&left.bbox.y))
            .then_with(|| left.bbox.x.total_cmp(&right.bbox.x))
    });
    true
}

// Issue #1397: a dense two-column body (a full page of prose, not the guarded
// four-span sentence above) is never split by xberg_native_pdf's own `ColumnAware`
// XY-Cut on some documents, so xberg's span-level assembler falls through to
// full-page-width Y order — welding left- and right-column lines at the same
// height into one interleaved element, mid-sentence, and welding distinct
// per-column headings (e.g. "Funding" + "References") into one heading
// element. No downstream reordering pass can repair (2): the interleaving is
// already baked into the element text by the time it is produced.
const MIN_DENSE_COLUMN_CONTENT_WIDTH_PTS: f32 = 200.0;
// 2%, not 3%. On the reporting document (A4, 595pt, columns at x=37.6 and
// x=306.6) symmetric margins put the left column's right edge at 288.4, so the
// real gutter is ~18.2pt — against a 3% threshold of 17.85pt that is a 0.35pt
// margin, and any page whose widest left-column line falls a point short of
// full justification would silently stop being repaired. 2% gives 11.9pt on
// A4, still far above the intra-line word spacing (~3-5pt at a 10pt font) that
// is the only thing this must not mistake for a column boundary.
const MIN_DENSE_COLUMN_GUTTER_FRACTION: f32 = 0.02;
const MIN_DENSE_COLUMN_GUTTER_PTS: f32 = 10.0;
// GH#1655: a gutter has a maximum plausible width too, not just a minimum. Without a
// cap, `widest_gap_midpoint` accepts a producer-emitted blank line (two whitespace
// spans opening a 329.6pt gap) or a footer split across both page margins (a 374.1pt
// gap between a label and a page number) as legitimate per-line gutter evidence, and
// both then outvote the page's real content into a false two-column split. 0.25 reuses
// `MAX_REDIRECT_DISTANCE_FRACTION`'s bound and reasoning below (149pt on A4): the
// reporter's five genuine hanging-number tab gaps top out at 25.2pt (4.2% of the
// 595.28pt page), the two junk witnesses are at 55.4% and 62.8%, and this repo's own
// dense two-column fixtures put their true gutter at 9.8% of page width -- 25% sits
// with wide margin above every real gutter measured and well below both junk gaps. ~keep
const MAX_DENSE_COLUMN_GUTTER_FRACTION: f32 = 0.25;
const MIN_DENSE_COLUMN_SPANS_PER_SIDE: usize = 6;
// Hanging clause numbers and list labels occupy a narrow x-band beside the
// column body. If the median gutter estimate lands inside that band, snapping
// to its left edge restores the true gutter. Six percent covers the 18-23pt
// labels observed on A4/Letter pages while excluding body prose and wide furniture.
const MAX_DENSE_COLUMN_SPLIT_SNAP_SPAN_FRACTION: f32 = 0.06;
// Repeated hanging labels share a left edge modulo sub-point PDF transform and
// font-positioning noise. Requiring an aligned population prevents one-off
// narrow gutter-crossing furniture from moving the page-wide split.
const DENSE_COLUMN_SPLIT_SNAP_X_TOLERANCE_PTS: f32 = 1.0;
// A label can be emitted as overlapping fragments, where moving the split out
// of one fragment reveals another straddler immediately to its left. Bound the
// fixed-point search so malformed geometry cannot make this pass unbounded.
const MAX_DENSE_COLUMN_SPLIT_SNAP_PASSES: usize = 4;
// A full-width furniture span (running header/footer, page-wide rule,
// full-width title) spans nearly the entire printable width regardless of
// the two-column layout beneath it, whereas a genuine column is bounded by
// the page margins AND the gutter and can never reach much past ~45% of the
// page width even on a page with unusually narrow margins. On the reporting
// document from the worked example above (A4, 595pt wide, columns at
// x=37.6/x=306.6), each column is 250.8pt wide = 42.2% of page width, while
// a running header spanning x=37.6..557 is 519.4pt = 87.3% of page width.
// 0.55 sits 13 points above the column ceiling (headroom for
// justification/kerning noise on an unusually wide column line) and over 30
// points below a typical full-width furniture span, so it cleanly separates
// the two without needing per-document calibration. It remains ONE of two
// signals a line is furniture (see `line_is_boundary`) rather than the sole
// one: narrower furniture that still crosses the gutter is caught by the
// straddle test below instead of by widening this threshold, which would
// reclassify genuinely single-column pages as two columns (see the
// `single_column_page_with_wide_and_narrow_lines_is_not_split` regression
// guard in the tests below).
const FULL_WIDTH_FURNITURE_FRACTION: f32 = 0.55;

/// A furniture width no span reaches, for the checks that ask about a split already chosen.
///
/// GH#2109: once the split is known, a span that does not cross it lies inside one column
/// and cannot be furniture however wide it is; a line that does cross it is still a band
/// boundary through `line_is_boundary`'s straddle test. A justified main column of one
/// 375pt span per line beside a margin-note column is wider than
/// `FULL_WIDTH_FURNITURE_FRACTION` of the page, and treating each of its lines as furniture
/// cut the page into one-line bands that never reached the column quorum. ~keep
const NO_WIDTH_FURNITURE: f32 = f32::INFINITY;
// Two spans on the same visual line never differ in `y` by more than
// sub-point float noise from the PDF coordinate transform; two distinct lines
// are always at least a line-height apart (~14pt for the 11pt-font fixtures
// below, and body text is never set with negative leading). 0.5pt sits
// comfortably inside the first gap and nowhere near the second.
const LINE_Y_TOLERANCE_PTS: f32 = 0.5;
// A single line with a coincidentally wide internal gap (heavy justification,
// a dotted table-of-contents leader) must not be read as a real column
// gutter on an otherwise single-column page. Requiring this many independent
// lines to agree on the same gutter position before trusting it applies the
// same density bar `MIN_DENSE_COLUMN_SPANS_PER_SIDE` applies to a column's
// population, to the evidence for the gutter's existence.
const MIN_DENSE_COLUMN_SPLIT_LINES: usize = MIN_DENSE_COLUMN_SPANS_PER_SIDE;
// GH#1603: a single outlier line (one long justified line, one stray word) can close
// the true gutter as a page-wide corridor while a same-shaped-but-irrelevant corridor
// sits elsewhere on the page -- on a hanging-number/list-label layout, that corridor
// is the number-to-text indent, present on *every* line, so it always wins
// `redirect_split_out_of_content`'s `max_by(width)` once the real gutter is narrower
// than `min_gutter` or closed by that one outlier. Bounding how far the widest
// corridor may move the split distinguishes a legitimate relocation from an
// illegitimate one without having to characterise the corridor's shape at all.
// Measured: GH#1545's misdetected table/prose median moves 61.6pt to the real
// table/prose gutter; the reporter's corpus records genuine `detect_split_x` misses
// relocated by up to 92.7pt (30.9pt and 38.3pt elsewhere in the same corpus). GH#1603
// itself relocates the split 230pt, straight into a clause's own hanging-number
// indent. 25% of page width (149pt on A4) sits with ~57pt of headroom above the
// largest legitimate move measured and ~80pt of margin below the smallest
// illegitimate one. ~keep
const MAX_REDIRECT_DISTANCE_FRACTION: f32 = 0.25;
// A page-wide corridor that exactly one non-furniture line runs through is still a
// gutter: a centred footer, a caption, or a heading set across both columns crosses
// the gutter on precisely one line, while a table row or a table-of-contents leader
// crosses it on every line. Measured on the reporter's 18-page carrier the true gutter
// (292.51..304.87 on A4) is crossed by no line on 16 pages and by exactly one on the
// other two -- so a tolerance of one restores exactly that gutter and opens nothing
// inside a table. Consulted only for a split that `MIN_DENSE_COLUMN_SPLIT_LINES` lines already
// run through, i.e. one that demonstrably sits inside a column. ~keep
const MAX_GUTTER_CROSSING_LINES: usize = 1;

/// The largest share of the lines *outside* the band the corridor search is scoped to that may
/// disagree with the incoming split and still leave it read as the page's own gutter.
///
/// GH#1801. Measured over the nine carriers this gate has to separate, as a fraction of the
/// non-furniture lines outside the band: the three pages whose split must be KEPT sit at 3.7%,
/// 2.2% and 1.4% (3 of 81, 1 of 46, 1 of 71), and the lowest page that must still be REDIRECTED
/// sits at 15.6% (15 of 96; the others are 34.7%, 46.6% and 40.0%). 0.08 is about a factor of two
/// clear of each side -- a real margin, not an order of magnitude, so re-measure it rather than
/// assume it if a new carrier lands near it. ~keep
const MAX_OUT_OF_BAND_SPLIT_DISAGREEMENT_FRACTION: f32 = 0.08;
// GH#1762 adversarial review: `redirect_survives_a_second_table_closing_the_band_from_below_gh1762`
// (text.rs tests) shows a second full-width table below the rescued band does NOT
// exceed this tolerance -- but only because the band's pre-existing boundary line
// happens to end exactly at the corridor's edge and contributes zero crossings there.
// A page where BOTH the pre-existing boundary and a second table's boundary
// genuinely cross the true gutter is not covered by any test and remains unproven,
// not proven safe. ~keep
// GH#1545: two regions with different leading (a table on 8.05pt beside prose on
// 10.45pt) are never grouped into a shared line by `group_into_lines`, so per-line
// gutter evidence only ever sees each region's *internal* gaps and the median lands
// inside one of them. A page-wide whitespace corridor does see the boundary between
// them. The per-line median stays authoritative whenever it already sits in such a
// corridor -- which is every ordinary two-column page, and every page whose corridor
// is closed by narrow gutter-crossing furniture (the case per-line evidence exists
// for) -- so the corridor is consulted only when the median demonstrably sits inside
// content rather than inside whitespace. ~keep
// Two sides of a gutter that pair up row-for-row are one table whose rows carry the
// meaning (label left, value right); reordering column-major would destroy them, which
// is what `dense_two_column_table_keeps_row_order` guards. Two sides that do NOT pair
// up are independent regions and may be separated. Measured on the two fixtures: the
// GH#1545 table/prose page pairs 3 of 47 lines (0.064) while the table guard pairs 8 of
// 8 (1.000), so 0.5 sits in open space with no fixture anywhere near it. ~keep
const MAX_CROSS_GUTTER_ROW_PAIRING_FRACTION: f32 = 0.5;
// A repeated label/value panel ("Sex % Sex %") welds its two halves per row when the
// region is emitted row-major. The panel boundary cannot be found by gutter width --
// measured on the GH#1545 page it is 3.28pt against a 1.95pt word space, a 1.3pt margin
// at a 6.475pt font -- but the columns' left edges repeat exactly, so the boundary is
// recoverable as "a text column immediately following a numeric one". The measured
// per-column numeric fractions there are 0.02 / 0.96 / 0.00 / 1.00, so these thresholds
// sit in a gap almost as wide as the range itself and a table that does not separate
// this cleanly declines instead of guessing. ~keep
const MIN_PANEL_VALUE_COLUMN_NUMERIC_FRACTION: f32 = 0.8;
const MAX_PANEL_LABEL_COLUMN_NUMERIC_FRACTION: f32 = 0.2;
// A panel needs a label column and a value column, so splitting is only meaningful
// from four columns up, and a boundary that would leave a one-column panel is rejected. ~keep
const MIN_PANEL_SPLIT_COLUMNS: usize = 4;
const MIN_COLUMNS_PER_PANEL: usize = 2;
// A caption or title inside the region runs across every column as ordinary prose, so
// its words land between the column edges rather than on them. Emitting it panel-major
// would tear it in half. Measured on the GH#1545 page the title aligns 0.20 of its
// spans to a column edge while all 30 real rows align 0.50 or more. ~keep
const MIN_GRID_ROW_COLUMN_ALIGNMENT_FRACTION: f32 = 0.4;

/// One visual line: span indices in left-to-right (`x` ascending) order.
type SpanLine = Vec<usize>;

/// Sort every span index top-to-bottom, then left-to-right.
///
/// This is the single global sort the rest of `reorder_dense_two_column_page`
/// is built on: line grouping, per-line gutter detection, and band bucketing
/// below all walk this order without re-sorting the whole page again.
fn spans_sorted_top_to_bottom(spans: &[xberg_native_pdf::layout::TextSpan]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..spans.len()).collect();
    order.sort_by(|&a, &b| {
        spans[b]
            .bbox
            .y
            .total_cmp(&spans[a].bbox.y)
            .then_with(|| spans[a].bbox.x.total_cmp(&spans[b].bbox.x))
    });
    order
}

/// Bucket a top-to-bottom-sorted span order into visual lines (the
/// band-splitting pass's first step).
///
/// A line is anchored on its topmost span; a new line starts once `y` drifts
/// more than `LINE_Y_TOLERANCE_PTS` from that anchor, so gradual drift across
/// many spans can never chain unrelated lines together. Each line is then
/// re-sorted left-to-right on its own (a handful of spans at most) so that
/// two spans on the same line with slightly different `y` cannot leave the
/// line out of x-order, which the per-line gutter sweep below requires.
fn group_into_lines(spans: &[xberg_native_pdf::layout::TextSpan], order: &[usize]) -> Vec<SpanLine> {
    let mut lines: Vec<SpanLine> = Vec::new();
    let mut anchor_y = f32::NAN;
    for &index in order {
        let y = spans[index].bbox.y;
        if lines.is_empty() || (anchor_y - y).abs() > LINE_Y_TOLERANCE_PTS {
            anchor_y = y;
            lines.push(Vec::new());
        }
        lines.last_mut().expect("just pushed above").push(index);
    }
    for line in &mut lines {
        line.sort_by(|&a, &b| spans[a].bbox.x.total_cmp(&spans[b].bbox.x));
    }
    lines
}

/// Widest gap at least `min_gutter` and at most `max_gutter` wide between
/// consecutive, left-to-right sorted `(left, right)` edges, or `None` if
/// nothing reaches it.
///
/// Tracking the running rightmost edge already seen (rather than just the
/// previous span's right edge) means a span nested inside an earlier one can
/// never be mistaken for the start of a gap. Shared by the per-line gutter
/// check below, the only caller left after per-band segmentation replaced the
/// old single whole-page projection.
///
/// GH#1655: `max_gutter` (`MAX_DENSE_COLUMN_GUTTER_FRACTION`) rejects a gap wide
/// enough that it cannot be a gutter at all -- a blank line's two whitespace spans, or
/// a footer split across both page margins, each open a gap of several hundred points
/// on an ordinary page and would otherwise outvote genuine narrow gutters into a false
/// split. Note this rejects the *widest* candidate outright rather than falling back to
/// the next-widest one on the same line: a line whose only internal gap is that wide
/// has no real gutter evidence to offer either way. ~keep
fn widest_gap_midpoint(mut edges: impl Iterator<Item = (f32, f32)>, min_gutter: f32, max_gutter: f32) -> Option<f32> {
    let (_, mut running_right) = edges.next()?;
    let mut best_gap = 0.0_f32;
    let mut best_split = None;
    for (left, right) in edges {
        let gap = left - running_right;
        if gap > best_gap {
            best_gap = gap;
            best_split = Some((running_right + left) / 2.0);
        }
        running_right = running_right.max(right);
    }
    if best_gap < min_gutter || best_gap > max_gutter {
        None
    } else {
        best_split
    }
}

/// True if `span` carries visible text rather than only whitespace.
///
/// GH#1655: some PDF producers emit the tab between a hanging number and its heading
/// (or a blank line) as its own space-only span with its own font change, so it never
/// merges into a neighbouring span. Such a span occupies an x-range but carries no
/// content, and must not be treated as column population or as evidence that a line
/// runs through a gutter. ~keep
fn span_has_ink(span: &xberg_native_pdf::layout::TextSpan) -> bool {
    !span.text.trim().is_empty()
}

/// True if any span on `line` is full-width furniture by
/// `FULL_WIDTH_FURNITURE_FRACTION` (the pre-existing, width-only signal).
fn line_has_width_furniture(
    spans: &[xberg_native_pdf::layout::TextSpan],
    line: &SpanLine,
    furniture_width: f32,
) -> bool {
    line.iter().any(|&index| spans[index].bbox.width >= furniture_width)
}

// GH#1742: a table row's own multiple internal cell gaps are not gutter evidence, but
// nothing before this excluded them from the vote. A two-column line with a hanging
// number on EACH margin -- both the GH#1484/#1603 fixtures and the reporter's own
// carrier construct rows shaped exactly this way -- already opens three internal gaps
// (the left number-to-text indent, the gutter itself, and the right number-to-text
// indent), so three is not a safe ceiling: `redirect_split_out_of_content_must_not_
// relocate_into_a_hanging_number_indent_gh1603` and `split_inside_a_column_is_moved_
// to_a_gutter_one_footer_line_crosses` both regress at three, because it excludes the
// very lines that carry the true gutter. A genuine table row does not stop at three:
// the reporter's own reproducer tables are five columns (four gaps) and the issue's
// own narrower three-column shape is called out as *not* covered by this rule at all.
// Four sits one above the two-hanging-number ceiling and at the four-gap floor the
// reproducer's own tables measure. ~keep
const MIN_GRID_ROW_GAP_COUNT: usize = 4;

/// True if `line`'s inked spans are separated by at least `MIN_GRID_ROW_GAP_COUNT`
/// internal gaps each at least `min_gutter` wide -- the shape of a multi-column table
/// row, never a hanging-number or ordinary prose line.
///
/// GH#1742: on a two-column page that also carries a table, a table row on its own
/// leading is never grouped into a shared line with the opposite column (see
/// `redirect_split_out_of_content`'s doc comment), so its own internal cell gaps are
/// the only gaps `widest_gap_midpoint` ever sees for that line -- and the widest of
/// them, deep inside the table, was being counted as if it were gutter evidence.
/// Excluding a line with this many internal gaps from the vote (in `detect_split_x`)
/// and from the hanging-label snap's candidate pool (in `aligned_hanging_label_left_edge`)
/// removes that pollution at its source, before any downstream redirect or guard has
/// to reason about it. ~keep
fn line_has_grid_row_gaps(spans: &[xberg_native_pdf::layout::TextSpan], line: &SpanLine, min_gutter: f32) -> bool {
    let mut edges: Vec<(f32, f32)> = line
        .iter()
        .filter(|&&index| span_has_ink(&spans[index]))
        .map(|&index| (spans[index].bbox.left(), spans[index].bbox.right()))
        .collect();
    edges.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut edges = edges.into_iter();
    let Some((_, mut running_right)) = edges.next() else {
        return false;
    };
    let mut gap_count = 0usize;
    for (left, right) in edges {
        if left - running_right >= min_gutter {
            gap_count += 1;
        }
        running_right = running_right.max(right);
    }
    gap_count >= MIN_GRID_ROW_GAP_COUNT
}

// GH#1756: one shared left edge is a margin, not a grid. Two is the floor at which a
// set of column edges describes a table at all, and a row that `line_has_grid_row_gaps`
// accepts has five or more cells, so a genuine table's grid clears this with room to
// spare -- it only refuses the degenerate case where the excluded rows agree on a
// single x, where every line on the page that starts there would otherwise be read as
// a table line. ~keep
const MIN_GRID_COLUMNS_FOR_CELL_GAP_VOTES: usize = 2;

/// True if the gap at `midpoint` that `line` is about to vote for is a gap *between*
/// two of `columns` -- a table's own cell gap -- and `line` is itself a row of that
/// table rather than prose running across it.
///
/// GH#1756: `line_has_grid_row_gaps` removes a table's *full* rows from
/// `detect_split_x`'s vote, but a table typeset by a journal also carries sparse lines
/// -- the second and third line of a wrapped column header, a standard-deviation line
/// under its row, a row whose long label closes the gap to its first value, a group row
/// with empty cells, a units line. Each opens only one to three internal gaps, so each
/// keeps its vote, and each votes for a gap deep inside the table. With the full rows
/// gone those sparse lines are the *majority* of the vote: measured on the reporter's
/// carrier the vote fell from 38 midpoints to 11, the median moved from 383.3 to 449.1,
/// and the distance from the true gutter corridor grew from 84.5pt to 150.3pt -- 1.5pt
/// past `MAX_REDIRECT_DISTANCE_FRACTION` of the page width (148.8pt), so the redirect
/// that had been rescuing the page could no longer reach it.
///
/// The rows the gap-count filter already excluded are what identifies the rest of the
/// table: they establish its column grid (`strong_column_edges`, which still requires
/// `MIN_DENSE_COLUMN_SPLIT_LINES` independent rows to agree on an edge).
/// `row_follows_column_grid` -- the same predicate `order_region_by_panels` uses to
/// tell a table row from a caption running across one, reused rather than reinvented --
/// then recognises the sparse rows whatever their gap count.
///
/// The vote's own position is the second half of the test, and it is what keeps this
/// from throwing away real evidence: a table row's *last* gap, between its rightmost
/// cell and the opposite column, IS the page gutter and is the only per-line evidence
/// some pages have. Only a gap that falls between two of the table's own column edges
/// is a cell gap. Both halves are load-bearing in opposite directions: without the
/// alignment test, ordinary prose whose gap happens to fall across a table elsewhere on
/// the page loses its vote; without the position test, a table row whose widest gap is
/// the gutter loses its vote, which on the GH#1742 reproducer moves the median 5pt into
/// the left column's longest line. ~keep
fn vote_is_a_grid_cell_gap(
    spans: &[xberg_native_pdf::layout::TextSpan],
    line: &SpanLine,
    columns: &[f32],
    midpoint: f32,
) -> bool {
    if columns.len() < MIN_GRID_COLUMNS_FOR_CELL_GAP_VOTES {
        return false;
    }
    let (Some(&first), Some(&last)) = (columns.first(), columns.last()) else {
        return false;
    };
    midpoint > first && midpoint < last && row_follows_column_grid(spans, line, columns)
}

// GH#1742 (reproducer page 4): a three-column table row opens only two internal
// gaps, one short of `MIN_GRID_ROW_GAP_COUNT`, so it is never excluded from
// `detect_split_x`'s vote and can still make the median land inside the table. Once
// it has, that internal gap is real whitespace on every row that has it, so no span
// crosses the split there either -- `redirect_split_out_of_content`'s own
// `lines_crossing` count (spans literally straddling the split) stays far below
// `MIN_DENSE_COLUMN_SPLIT_LINES` and the widened corridor search that would find the
// true gutter never runs. Two is the floor for "this line has more than one
// internal boundary" -- an ordinary two-column body line (including a hanging
// clause number's own indent-to-gutter gap) has exactly one. It is deliberately
// lower than `MIN_GRID_ROW_GAP_COUNT`: unlike the vote exclusion, a false positive
// here only widens the search, and the redirect's existing downstream guards
// (`both_sides_are_columns`, `corridor_is_hanging_label_indent`, the redirect
// distance cap) still have to accept whatever candidate that search turns up. ~keep
const MIN_TABLE_ROW_INTERNAL_GAP_COUNT: usize = 2;

/// True if `line` has at least `MIN_TABLE_ROW_INTERNAL_GAP_COUNT` internal gaps each
/// at least `min_gutter` wide, and `x` falls inside one of them.
///
/// Unlike `line_has_grid_row_gaps`, this does not exclude the line from anything --
/// it is evidence for `redirect_split_out_of_content` that a split's freedom from
/// `lines_crossing` on this particular line came from sitting in a multi-column
/// row's own cell gap, not from genuinely clean whitespace.
fn line_has_internal_gap_at(
    spans: &[xberg_native_pdf::layout::TextSpan],
    line: &SpanLine,
    min_gutter: f32,
    x: f32,
) -> bool {
    let mut edges: Vec<(f32, f32)> = line
        .iter()
        .filter(|&&index| span_has_ink(&spans[index]))
        .map(|&index| (spans[index].bbox.left(), spans[index].bbox.right()))
        .collect();
    edges.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut edges = edges.into_iter();
    let Some((_, mut running_right)) = edges.next() else {
        return false;
    };
    let mut gap_count = 0usize;
    let mut x_in_a_gap = false;
    for (left, right) in edges {
        if left - running_right >= min_gutter {
            gap_count += 1;
            if x >= running_right && x <= left {
                x_in_a_gap = true;
            }
        }
        running_right = running_right.max(right);
    }
    gap_count >= MIN_TABLE_ROW_INTERNAL_GAP_COUNT && x_in_a_gap
}

/// How many non-furniture lines have `x` inside one of their own multi-column
/// internal gaps (`line_has_internal_gap_at`).
fn lines_with_internal_gap_at(
    spans: &[xberg_native_pdf::layout::TextSpan],
    lines: &[SpanLine],
    furniture_width: f32,
    min_gutter: f32,
    x: f32,
) -> usize {
    lines
        .iter()
        .filter(|&line| !line_has_width_furniture(spans, line, furniture_width))
        .filter(|&line| line_has_internal_gap_at(spans, line, min_gutter, x))
        .count()
}

/// The lines of the one content band that carries the split's own inside-a-table-gap
/// population, or `None` when no single band carries `MIN_DENSE_COLUMN_SPLIT_LINES` of
/// them.
///
/// GH#1762: the corridor search `redirect_split_out_of_content` runs is page-wide, but
/// the split it rescues is applied *per band*. `build_bands` already sets a full-width
/// table apart -- every one of its rows straddles a split that sits inside a column
/// below it, so every one is a boundary line -- and yet those same rows still close the
/// gutter for the band beneath them, where nothing is written across it at all. On the
/// reporter's page a five-row table at the top of the page left `page_whitespace_
/// corridors` and `page_low_occupancy_corridors` both empty, so a split sitting between
/// two columns of a four-column table lower down had nowhere to be moved to and stayed
/// there; removing that table (their page 2) or moving its columns clear of the gutter
/// (their page 3) makes the same page-wide search find the corridor at once.
///
/// The band is chosen by the population that made the split wrong in the first place:
/// the lines with the split inside one of their own multi-column cell gaps
/// (`line_has_internal_gap_at`, the GH#1742 signal). Requiring one band to carry
/// `MIN_DENSE_COLUMN_SPLIT_LINES` of them applies the same bar the page-wide count
/// applies, to the band rather than to the page, so a page whose evidence is spread
/// thinly across several bands keeps the page-wide search it has always had. ~keep
fn band_lines_around_table_gap_split(
    spans: &[xberg_native_pdf::layout::TextSpan],
    lines: &[SpanLine],
    furniture_width: f32,
    min_gutter: f32,
    split_x: f32,
) -> Option<Vec<SpanLine>> {
    let mut bands: Vec<(usize, Vec<SpanLine>)> = vec![(0, Vec::new())];
    for line in lines {
        let (population, band) = bands.last_mut().expect("bands always holds the band being accumulated");
        if line_is_boundary(spans, line, furniture_width, split_x) {
            // A boundary line is emitted *between* the band above it and the band
            // below it, so it is context for both: the caption over a table, the
            // introductory line before it. Only a band separated from this one by such
            // a line is out of scope. ~keep
            band.push(line.clone());
            bands.push((0, vec![line.clone()]));
            continue;
        }
        *population += usize::from(line_has_internal_gap_at(spans, line, min_gutter, split_x));
        band.push(line.clone());
    }
    bands
        .into_iter()
        .max_by_key(|&(population, _)| population)
        .filter(|&(population, _)| population >= MIN_DENSE_COLUMN_SPLIT_LINES)
        .map(|(_, band)| band)
}

/// Establish the page's gutter x-position from independent per-line evidence.
///
/// Each line is checked in isolation for an internal gap at least
/// `min_gutter` wide: a genuine two-column line always has exactly this shape
/// (one run of spans per side). Because the check is per line, a furniture
/// line elsewhere on the page — even one narrower than
/// `FULL_WIDTH_FURNITURE_FRACTION` that crosses the gutter without an
/// internal gap of its own — can never corrupt another line's evidence. That
/// is what a single whole-page projection could not guarantee, and is the fix
/// for furniture narrower than the width threshold that used to close the
/// projection and suppress the repair for the whole page.
///
/// Requires at least `MIN_DENSE_COLUMN_SPLIT_LINES` agreeing lines and
/// returns their median split point, robust to the rare line whose own gap
/// sits a little off from the rest (e.g. a heading whose two sides are
/// narrower than the body columns beneath it).
///
/// GH#1655: a line with no inked span at all -- a producer-emitted blank line made
/// of two whitespace spans -- is not gutter evidence: its "gap" separates nothing,
/// not two columns of real content. Filtered here in addition to
/// `MAX_DENSE_COLUMN_GUTTER_FRACTION` in `widest_gap_midpoint` below, which independently
/// rejects the same line's gap for being implausibly wide -- either fix alone
/// already removes it from the vote. ~keep
///
/// GH#1756: the lines `line_has_grid_row_gaps` rejects are not only removed from the
/// vote, they are read first, for the column grid they establish. A line that sits on
/// that grid and votes for a gap between two of its columns is a table row whatever
/// its own gap count, and is removed too (`vote_is_a_grid_cell_gap`). ~keep
fn detect_split_x(spans: &[xberg_native_pdf::layout::TextSpan], lines: &[SpanLine], page_width: f32) -> Option<f32> {
    let min_gutter = (page_width * MIN_DENSE_COLUMN_GUTTER_FRACTION).max(MIN_DENSE_COLUMN_GUTTER_PTS);
    let max_gutter = page_width * MAX_DENSE_COLUMN_GUTTER_FRACTION;
    let furniture_width = page_width * FULL_WIDTH_FURNITURE_FRACTION;

    let voting_lines = || {
        lines
            .iter()
            .filter(|&line| !line_has_width_furniture(spans, line, furniture_width))
            .filter(|&line| line.iter().any(|&index| span_has_ink(&spans[index])))
    };
    let grid_rows: Vec<SpanLine> = voting_lines()
        .filter(|&line| line_has_grid_row_gaps(spans, line, min_gutter))
        .cloned()
        .collect();
    let grid_columns = strong_column_edges(spans, &grid_rows);

    let mut midpoints: Vec<f32> = voting_lines()
        .filter(|&line| !line_has_grid_row_gaps(spans, line, min_gutter))
        .filter_map(|line| {
            let edges = line
                .iter()
                .map(|&index| (spans[index].bbox.left(), spans[index].bbox.right()));
            let midpoint = widest_gap_midpoint(edges, min_gutter, max_gutter)?;
            (!vote_is_a_grid_cell_gap(spans, line, &grid_columns, midpoint)).then_some(midpoint)
        })
        .collect();
    if midpoints.len() < MIN_DENSE_COLUMN_SPLIT_LINES {
        return sole_whitespace_gutter(spans, lines, page_width);
    }
    midpoints.sort_by(f32::total_cmp);
    let mid = midpoints.len() / 2;
    Some(if midpoints.len().is_multiple_of(2) {
        (midpoints[mid - 1] + midpoints[mid]) / 2.0
    } else {
        midpoints[mid]
    })
}

/// Move a split that still cuts through a word after snapping to the page's own
/// whitespace corridor (GH#1545).
///
/// A split is only a gutter if nothing is written across it. `detect_split_x`'s
/// median can land inside content when the page's per-line evidence is drawn from
/// one region only — a table on 8.05pt leading beside prose on 10.45pt is never
/// grouped into shared lines, so every midpoint comes from the table's internal
/// gaps and the median sits between two of the table's own columns.
///
/// Deliberately applied *after* `snap_split_left_of_hanging_labels`, which is the
/// existing remedy for the one case where a split legitimately starts out inside a
/// span: a hanging clause number. On the GH#1484 fixture the median cuts six label
/// spans and snapping already resolves it, so this pass sees a clean split and
/// leaves it alone. Only a split that survives snapping still cutting a word is
/// redirected here.
///
/// GH#1603: the widest corridor is not always the right one. On a hanging-number
/// page the number-to-text indent is present on every line and is wider than a true
/// gutter narrower than `min_gutter` (or one closed by a single outlier line), so
/// `max_by` hands back the indent -- moving the split into a clause instead of onto
/// its gutter. `MAX_REDIRECT_DISTANCE_FRACTION` bounds how far this pass may move the
/// split from the incoming (detected/snapped) one; a move past that bound falls back
/// to `split_x` unchanged rather than relocating into unrelated content.
///
/// A corridor is whitespace, and `page_whitespace_corridors` reads whitespace as
/// "no span's bbox" -- so one line set across the gutter (a centred footer, a
/// heading spanning both columns) closes the gutter for the whole page, and a split
/// the per-line median has put *inside* a column then has nothing to be moved to.
/// That page is not left alone by the reorder: every line the split runs through
/// becomes a band boundary, and whatever is left between them is reordered against
/// a split that is not a gutter. So when the split is crossed by
/// `MIN_DENSE_COLUMN_SPLIT_LINES` lines or more, the search is widened to corridors
/// that at most `MAX_GUTTER_CROSSING_LINES` lines cross -- still bounded by the
/// same distance cap, and skipping the hanging-label indents that
/// `corridor_is_hanging_label_indent` recognises and bounded on both sides by a
/// column of running text (`both_sides_are_columns`). A split that sits in whitespace,
/// or that a single heading crosses, never reaches that second search.
///
/// GH#1742 (reproducer page 4): a split can sit inside a table column without a
/// single span crossing it at all, when the table's own leading never shares a line
/// with the opposite column (see `line_has_grid_row_gaps`'s doc comment) -- every
/// line that runs through the split does so through its own internal cell gap, never
/// through a span. `lines_crossing` alone then stays at whatever incidental furniture
/// or heading happens to cross, far below `MIN_DENSE_COLUMN_SPLIT_LINES`, and the
/// widened search that would find the true gutter never runs even though the split
/// is just as much inside a column as one that does cut a span on every line.
/// `lines_with_internal_gap_at` counts that population directly and, once it alone
/// clears the threshold, admits the split to the same widened search a literal
/// `lines_crossing` count would have. ~keep
fn redirect_split_out_of_content(
    spans: &[xberg_native_pdf::layout::TextSpan],
    lines: &[SpanLine],
    page_width: f32,
    split_x: f32,
) -> f32 {
    let cuts_a_span = spans
        .iter()
        .any(|span| span.bbox.left() < split_x && span.bbox.right() > split_x);
    let furniture_width = page_width * FULL_WIDTH_FURNITURE_FRACTION;
    let min_gutter = (page_width * MIN_DENSE_COLUMN_GUTTER_FRACTION).max(MIN_DENSE_COLUMN_GUTTER_PTS);
    let split_inside_a_table_gap =
        lines_with_internal_gap_at(spans, lines, furniture_width, min_gutter, split_x) >= MIN_DENSE_COLUMN_SPLIT_LINES;
    if !cuts_a_span && !split_inside_a_table_gap {
        return split_x;
    }
    // GH#1762: ask the corridor question of the band the split is wrong in, not of the
    // whole page -- a table in another band, already set apart by its own boundary
    // lines, must not close this band's gutter. Falls back to the page when no single
    // band carries the evidence. ~keep
    let band = band_lines_around_table_gap_split(spans, lines, furniture_width, min_gutter, split_x);
    // GH#1801: only redirect a split the rest of the page disagrees with. A real out-of-band
    // population is required as well as a low disagreement rate inside it, because an empty or
    // tiny one reports agreement it never measured -- see `out_of_band_split_agreement`. ~keep
    if let Some(band_lines) = band.as_deref() {
        let (outside, disagreeing) =
            out_of_band_split_agreement(spans, lines, band_lines, furniture_width, min_gutter, split_x);
        if outside >= MIN_DENSE_COLUMN_SPLIT_LINES
            && disagreeing < MIN_DENSE_COLUMN_SPLIT_LINES
            && disagreeing as f32 <= outside as f32 * MAX_OUT_OF_BAND_SPLIT_DISAGREEMENT_FRACTION
        {
            return split_x;
        }
    }
    let search_lines: &[SpanLine] = band.as_deref().unwrap_or(lines);
    let max_redirect_distance = page_width * MAX_REDIRECT_DISTANCE_FRACTION;
    let max_label_width = page_width * MAX_DENSE_COLUMN_SPLIT_SNAP_SPAN_FRACTION;
    // GH#1800: the same two qualifications PATH 2 (below) already applies to a corridor a
    // split is inside a column, applied here too. Before this fix PATH 1 took the WIDEST
    // whitespace corridor with no qualification at all, and a sparse table's own cell gap
    // is routinely wider than the page's real gutter (53 vs 21pt, 37 vs 24, 24 vs 15 on the
    // reporter's pages) -- so a table beside prose always won the "widest" comparison before
    // either gate had a chance to run. Excluding a hanging-label indent and requiring both
    // flanks to read as columns are exactly the tests that already keep PATH 2 from doing
    // the same thing; the asymmetry, not a missing third gate, was the defect. ~keep
    let qualifies = |&corridor: &(f32, f32)| {
        !corridor_is_hanging_label_indent(spans, search_lines, max_label_width, corridor)
            && both_sides_are_columns(spans, search_lines, furniture_width, (corridor.0 + corridor.1) / 2.0)
    };
    let widest_within_reach = |corridors: Vec<(f32, f32)>| {
        corridors
            .into_iter()
            .max_by(|a, b| (a.1 - a.0).total_cmp(&(b.1 - b.0)))
            .map(|(left, right)| (left + right) / 2.0)
            .filter(|candidate| (candidate - split_x).abs() <= max_redirect_distance)
    };
    // A single crossing heading admits this search, so the split may already be the gutter.
    // Only a corridor with a column's worth of rows on each side may replace it. ~keep
    let whitespace_corridors: Vec<(f32, f32)> =
        page_whitespace_corridors(spans, search_lines, furniture_width, min_gutter)
            .into_iter()
            .filter(|corridor| {
                both_sides_fill_column_rows(spans, search_lines, furniture_width, (corridor.0 + corridor.1) / 2.0)
                    && qualifies(corridor)
            })
            .collect();
    if let Some(candidate) = widest_within_reach(whitespace_corridors) {
        return candidate;
    }

    // No empty corridor within reach. A split that a single line runs through (a
    // heading set across both columns) is left where the per-line evidence put it,
    // as before. A split that `MIN_DENSE_COLUMN_SPLIT_LINES` lines run through --
    // literally, or (GH#1742) through their own internal cell gap without a span
    // crossing at all -- is not in a gutter at all -- it is inside a column, and
    // every one of those lines is about to become a band boundary -- so the corridor
    // search is widened to bands that at most `MAX_GUTTER_CROSSING_LINES` lines
    // cross, minus the hanging-label indents that are wider than a real gutter on
    // every clause-numbered page (the GH#1603 shape, seen from the corridor's side).
    //
    // GH#1762 adversarial review: this admission gate must ask the question of the
    // same population the search below it (`page_low_occupancy_corridors`) is about
    // to run against -- `search_lines`, not the page-wide `lines`. Asking it of the
    // whole page let an unrelated table elsewhere inflate or deflate the crossing
    // count against a threshold guarding a search scoped to one band. ~keep
    if lines_crossing(spans, search_lines, furniture_width, split_x) < MIN_DENSE_COLUMN_SPLIT_LINES
        && !split_inside_a_table_gap
    {
        return split_x;
    }
    let corridors = page_low_occupancy_corridors(
        spans,
        search_lines,
        furniture_width,
        min_gutter,
        MAX_GUTTER_CROSSING_LINES,
    )
    .into_iter()
    .filter(qualifies)
    .collect();
    widest_within_reach(corridors).unwrap_or(split_x)
}

/// True if a split at `x` is one `reorder_band_columns` would actually accept once it
/// is handed one: both sides read as a column (`RegionClass::Prose` or `Reference`),
/// or one side does and the two sides do not pair up row for row.
///
/// This is the occupancy test a corridor has to pass before a split is moved into it
/// from inside a column: a gutter separates two columns of running text, whereas the
/// gap between a table's cells, between a legend's letters and their captions, or
/// between a narrative column and a chart, separates content the per-band reorder
/// gates would refuse -- and a split placed there still reorders whatever band those
/// gates happen to let through.
///
/// GH#1742: requiring literally *both* sides to classify as `Prose`/`Reference` was
/// stricter than the gate `reorder_band_columns` itself applies once a band is handed
/// a split -- that gate already accepts one `Table`/`Form`/`Mixed` side, provided the
/// two sides do not pair up row for row (`MAX_CROSS_GUTTER_ROW_PAIRING_FRACTION`, the
/// GH#1545 fix). A page whose left column is a table top to bottom and whose right
/// column is ordinary prose (reproducer p4) never passed the stricter gate, so the
/// widened search always discarded the true gutter and left the split inside the
/// table. Mirroring the same two-part test here closes that gap without weakening it:
/// a label/value table that pairs almost every row (`split_inside_a_table_column_is_
/// not_moved_to_the_cell_gap`) still fails on pairing fraction alone. ~keep
fn both_sides_are_columns(
    spans: &[xberg_native_pdf::layout::TextSpan],
    lines: &[SpanLine],
    furniture_width: f32,
    x: f32,
) -> bool {
    let indices: Vec<usize> = lines
        .iter()
        .filter(|&line| !line_has_width_furniture(spans, line, furniture_width))
        .flat_map(|line| line.iter().copied())
        .filter(|&index| span_has_ink(&spans[index]))
        .collect();
    let (left, right): (Vec<usize>, Vec<usize>) = indices.iter().copied().partition(|&index| spans[index].bbox.x < x);
    let left_reorderable = xberg_native_pdf::layout::classify_region(spans, &left).is_reorderable_column();
    let right_reorderable = xberg_native_pdf::layout::classify_region(spans, &right).is_reorderable_column();
    if left_reorderable && right_reorderable {
        return true;
    }
    if !left_reorderable && !right_reorderable {
        return false;
    }
    cross_gutter_row_pairing_fraction(spans, &indices, x) <= MAX_CROSS_GUTTER_ROW_PAIRING_FRACTION
}

/// True if both sides of `x` carry ink on `MIN_DENSE_COLUMN_SPANS_PER_SIDE` non-furniture
/// lines. A lone header label or page number is one line however many spans it is written in.
fn both_sides_fill_column_rows(
    spans: &[xberg_native_pdf::layout::TextSpan],
    lines: &[SpanLine],
    furniture_width: f32,
    x: f32,
) -> bool {
    let rows_on_side = |left_side: bool| {
        lines
            .iter()
            .filter(|&line| !line_has_width_furniture(spans, line, furniture_width))
            .filter(|line| {
                line.iter()
                    .any(|&index| span_has_ink(&spans[index]) && (spans[index].bbox.x < x) == left_side)
            })
            .count()
    };
    rows_on_side(true) >= MIN_DENSE_COLUMN_SPANS_PER_SIDE && rows_on_side(false) >= MIN_DENSE_COLUMN_SPANS_PER_SIDE
}

/// True if one of the line's inked spans is written across `x`.
fn line_crosses(spans: &[xberg_native_pdf::layout::TextSpan], line: &SpanLine, x: f32) -> bool {
    line.iter().any(|&index| {
        let span = &spans[index];
        span_has_ink(span) && span.bbox.left() < x && span.bbox.right() > x
    })
}

/// How many non-furniture lines have an inked span written across `x`.
fn lines_crossing(
    spans: &[xberg_native_pdf::layout::TextSpan],
    lines: &[SpanLine],
    furniture_width: f32,
    x: f32,
) -> usize {
    lines
        .iter()
        .filter(|&line| !line_has_width_furniture(spans, line, furniture_width))
        .filter(|&line| line_crosses(spans, line, x))
        .count()
}

/// `(lines outside `band`, how many of them disagree with a split at `x`)`, counting only
/// non-furniture lines. A line disagrees when one of its inked spans is written across the split,
/// or when the split falls inside one of its own multi-column internal gaps -- the same two
/// signals `lines_crossing` and `lines_with_internal_gap_at` count page-wide.
///
/// GH#1801: `band_lines_around_table_gap_split` exists so the corridor question is asked of the
/// band the split is wrong in rather than of the whole page (GH#1762). On a page whose table spans
/// the *full* width that scoping inverts. Inside such a table the gaps between its own columns are
/// the only corridors there are, so the search moves a split that was already the page's gutter
/// into one of them; every prose line below the table then straddles the new split and becomes a
/// band boundary, and the two columns come out interleaved line by line.
///
/// What separates those pages from the one the band scoping was written for is what the rest of the
/// page says about the split, which is why both halves of this pair are returned. The disagreement
/// count alone is not enough: on a page where the band *is* the page (a table whose rows interleave
/// in y with the prose beside it, so no boundary line ever separates them) there is no rest of the
/// page to consult, and a count of 0 means "nothing was examined", not "everything agrees" -- the
/// GH#1742 and GH#1545 carriers measure 0 and 7 lines outside the band against 46 to 81 on the
/// GH#1801 carriers. The caller therefore requires a real out-of-band population as well as a low
/// disagreement rate within it (`MAX_OUT_OF_BAND_SPLIT_DISAGREEMENT_FRACTION`). ~keep
fn out_of_band_split_agreement(
    spans: &[xberg_native_pdf::layout::TextSpan],
    lines: &[SpanLine],
    band: &[SpanLine],
    furniture_width: f32,
    min_gutter: f32,
    x: f32,
) -> (usize, usize) {
    let mut outside = 0usize;
    let mut disagreeing = 0usize;
    for line in lines
        .iter()
        .filter(|&line| !band.contains(line))
        .filter(|&line| !line_has_width_furniture(spans, line, furniture_width))
    {
        outside += 1;
        if line_crosses(spans, line, x) || line_has_internal_gap_at(spans, line, min_gutter, x) {
            disagreeing += 1;
        }
    }
    (outside, disagreeing)
}

/// Every maximal x-interval at least `min_gutter` wide that at most
/// `max_crossing_lines` non-furniture lines run through, counting only spans
/// that carry ink (a whitespace-only span occupies nothing).
///
/// `page_whitespace_corridors` below demands zero occupancy, so one gutter-
/// crossing line narrower than `furniture_width` -- a centred footer, a
/// caption, a heading set across both columns -- deletes the gutter from the
/// corridor list for the whole page. Tolerating a single crossing *line* (not
/// span: a line emitted as several fragments is still one line) restores
/// exactly that gutter and nothing else: a table row or a table-of-contents
/// leader crosses on every line, so a band inside a table never qualifies.
fn page_low_occupancy_corridors(
    spans: &[xberg_native_pdf::layout::TextSpan],
    lines: &[SpanLine],
    furniture_width: f32,
    min_gutter: f32,
    max_crossing_lines: usize,
) -> Vec<(f32, f32)> {
    // (left, right, line) of every inked span on a non-furniture line.
    let extents: Vec<(f32, f32, usize)> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| !line_has_width_furniture(spans, line, furniture_width))
        .flat_map(|(line_index, line)| line.iter().map(move |&index| (index, line_index)))
        .filter(|&(index, _)| span_has_ink(&spans[index]))
        .map(|(index, line_index)| (spans[index].bbox.left(), spans[index].bbox.right(), line_index))
        .filter(|(left, right, _)| left.is_finite() && right.is_finite() && right > left)
        .collect();
    if extents.is_empty() {
        return Vec::new();
    }
    let mut edges: Vec<f32> = extents.iter().flat_map(|&(left, right, _)| [left, right]).collect();
    edges.sort_by(f32::total_cmp);
    edges.dedup();

    // Occupancy of each elementary interval between consecutive edges, as the
    // number of distinct lines with an inked span covering it; consecutive
    // low-occupancy intervals merge into one corridor.
    let mut corridors: Vec<(f32, f32)> = Vec::new();
    let mut open: Option<f32> = None;
    let mut crossing_lines: Vec<usize> = Vec::new();
    for window in edges.windows(2) {
        let (lo, hi) = (window[0], window[1]);
        crossing_lines.clear();
        for &(left, right, line) in &extents {
            if left < hi && right > lo && !crossing_lines.contains(&line) {
                crossing_lines.push(line);
                if crossing_lines.len() > max_crossing_lines {
                    break;
                }
            }
        }
        if crossing_lines.len() <= max_crossing_lines {
            open.get_or_insert(lo);
        } else if let Some(start) = open.take()
            && lo - start >= min_gutter
        {
            corridors.push((start, lo));
        }
    }
    if let Some(start) = open {
        let end = *edges.last().expect("edges is non-empty when extents is");
        if end - start >= min_gutter {
            corridors.push((start, end));
        }
    }
    corridors
}

/// True if `line` carries an inked span that starts on or after `far_wall` --
/// text following a label on the label's own line, rather than unrelated content
/// in a neighbouring column across the corridor.
///
/// GH#1742: a hanging label always has its clause text on the *same visual line*,
/// immediately after the label-to-text indent. A table's edge column has nothing
/// there -- the row's other cells sit on the label side of the corridor, and
/// whatever text appears past the corridor on that same `y` belongs to an
/// unrelated, unpaired line in the opposite column. ~keep
fn line_has_far_side_successor(spans: &[xberg_native_pdf::layout::TextSpan], line: &SpanLine, far_wall: f32) -> bool {
    line.iter()
        .any(|&index| span_has_ink(&spans[index]) && spans[index].bbox.left() >= far_wall)
}

/// True if `corridor` is a hanging-label indent rather than a column gutter:
/// its left wall is a stack of narrow spans (at most `max_label_width` wide)
/// that share a left edge, `MIN_DENSE_COLUMN_SPLIT_LINES` or more of them --
/// the population `aligned_hanging_label_left_edge` snaps a split away from,
/// seen from the corridor's side.
///
/// A gutter's left wall is the ragged right edge of the left column's body
/// lines: wide spans whose left edges sit at the column margin, not at the
/// wall. Only the left wall is examined, because a hanging label always
/// precedes the text it labels; the *right* wall of a real gutter is very
/// often the right column's own label stack, which must not disqualify it.
///
/// GH#1742: a narrow left-aligned stack alone is not enough -- a multi-column
/// table's edge column (the reporter's `ignotum` stack, reproducer p1's own last
/// column) is exactly that shape too, but labels *nothing*: no inked span
/// follows on the far side of the corridor on the same line, because the row's
/// remaining cells sit on the label side and whatever text starts past the
/// corridor belongs to an unrelated, unpaired line. `line_has_far_side_successor`
/// requires the label's own successor text to be present before a line counts
/// toward the population, which a table's edge cells never satisfy. ~keep
fn corridor_is_hanging_label_indent(
    spans: &[xberg_native_pdf::layout::TextSpan],
    lines: &[SpanLine],
    max_label_width: f32,
    corridor: (f32, f32),
) -> bool {
    let wall = corridor.0;
    let mut left_edges: Vec<f32> = lines
        .iter()
        .filter(|line| line_has_far_side_successor(spans, line, corridor.1))
        .filter_map(|line| {
            line.iter()
                .filter_map(|&index| {
                    let bbox = &spans[index].bbox;
                    (bbox.width > 0.0
                        && bbox.width <= max_label_width
                        && bbox.right() <= wall + DENSE_COLUMN_SPLIT_SNAP_X_TOLERANCE_PTS
                        && bbox.right() >= wall - max_label_width)
                        .then_some(bbox.left())
                })
                .min_by(f32::total_cmp)
        })
        .collect();
    left_edges.sort_by(f32::total_cmp);
    left_edges.iter().enumerate().any(|(start, &left_edge)| {
        left_edges[start..]
            .iter()
            .take_while(|&&candidate| candidate - left_edge <= DENSE_COLUMN_SPLIT_SNAP_X_TOLERANCE_PTS)
            .count()
            >= MIN_DENSE_COLUMN_SPLIT_LINES
    })
}

/// Every maximal x-interval at least `min_gutter` wide that no inked, non-furniture
/// span occupies anywhere on the page.
///
/// This is the whole-page projection the per-line detector above deliberately
/// replaced, kept here as *corroboration* rather than as the primary signal. Its
/// known weakness is unchanged — furniture narrower than `furniture_width` that
/// crosses a gutter closes the corridor — but that only ever removes a candidate,
/// so a page it cannot read simply falls back to the per-line median.
///
/// GH#1655: a whitespace-only span (a tab-stop-as-space-span, a blank line) must not
/// be able to close a real gutter as "occupied" -- it occupies an x-range but carries
/// no content, so it is excluded here the same way `page_low_occupancy_corridors`
/// already excludes it. ~keep
fn page_whitespace_corridors(
    spans: &[xberg_native_pdf::layout::TextSpan],
    lines: &[SpanLine],
    furniture_width: f32,
    min_gutter: f32,
) -> Vec<(f32, f32)> {
    let mut extents: Vec<(f32, f32)> = lines
        .iter()
        .filter(|&line| !line_has_width_furniture(spans, line, furniture_width))
        .flat_map(|line| line.iter())
        .filter(|&&index| span_has_ink(&spans[index]))
        .map(|&index| (spans[index].bbox.left(), spans[index].bbox.right()))
        .filter(|(left, right)| left.is_finite() && right.is_finite())
        .collect();
    extents.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut corridors = Vec::new();
    let mut running_right = match extents.first() {
        Some(&(_, right)) => right,
        None => return corridors,
    };
    for (left, right) in extents {
        if left - running_right >= min_gutter {
            corridors.push((running_right, left));
        }
        running_right = running_right.max(right);
    }
    corridors
}

/// The page's only whitespace corridor, when it is a gutter too narrow for the native
/// XY-cut to find and the lines never share a baseline across it.
///
/// GH#2102: `detect_split_x` votes line by line, so it needs `MIN_DENSE_COLUMN_SPLIT_LINES`
/// lines that each carry spans from BOTH columns. A column of margin notes set on its own
/// leading beside the main text never pairs with it (the notes sit 9.6pt apart, the body
/// 13pt), so the only lines that vote are the few that coincide -- and the table rows that
/// used to make up the quorum are excluded since GH#1742. The corridor between the columns
/// is still empty of ink from the top of the page to the bottom, which is the same
/// whole-page evidence `redirect_split_out_of_content` already trusts to move a wrong split.
///
/// It is accepted on the same terms that redirect applies to a corridor (a column's worth of
/// rows on each side, not a hanging-label indent, two sides that read as columns), and only
/// when it is the page's single qualifying corridor: more than one is a table or a
/// three-column page, which this two-column repair does not handle. It must also be narrower
/// than `XYCutStrategy::DEFAULT_MIN_VALLEY_WIDTH`. A wider corridor is one the native
/// `ColumnAware` order has already cut, and a page whose vote fell below quorum for that
/// reason is deliberately handed back to it (GH#1756) rather than repaired here.
///
/// GH#2109: a justified column is one span per line, and a column wider than
/// `FULL_WIDTH_FURNITURE_FRACTION` of the page (375pt of 595pt, beside a note column) has
/// every line discarded as furniture, so neither the corridor's edge nor the column quorum is
/// visible. The corridor is therefore first sought with no span treated as furniture, and only
/// when that finds none with the usual furniture exclusion, which still tolerates a header set
/// across the gutter. ~keep
fn sole_whitespace_gutter(
    spans: &[xberg_native_pdf::layout::TextSpan],
    lines: &[SpanLine],
    page_width: f32,
) -> Option<f32> {
    let min_gutter = (page_width * MIN_DENSE_COLUMN_GUTTER_FRACTION).max(MIN_DENSE_COLUMN_GUTTER_PTS);
    let max_gutter = page_width * MAX_DENSE_COLUMN_GUTTER_FRACTION;
    let furniture_width = page_width * FULL_WIDTH_FURNITURE_FRACTION;
    let max_label_width = page_width * MAX_DENSE_COLUMN_SPLIT_SNAP_SPAN_FRACTION;
    let qualifying_corridors = |corridor_furniture_width: f32| -> Vec<(f32, f32)> {
        page_whitespace_corridors(spans, lines, corridor_furniture_width, min_gutter)
            .into_iter()
            .filter(|&(left, right)| right - left <= max_gutter)
            .filter(|&corridor| {
                let mid = (corridor.0 + corridor.1) / 2.0;
                both_sides_fill_column_rows(spans, lines, NO_WIDTH_FURNITURE, mid)
                    && !corridor_is_hanging_label_indent(spans, lines, max_label_width, corridor)
                    && both_sides_are_columns(spans, lines, NO_WIDTH_FURNITURE, mid)
            })
            .collect()
    };
    let mut corridors = qualifying_corridors(NO_WIDTH_FURNITURE);
    if corridors.is_empty() {
        corridors = qualifying_corridors(furniture_width);
    }
    let &[(left, right)] = corridors.as_slice() else {
        return None;
    };
    (right - left < xberg_native_pdf::XYCutStrategy::DEFAULT_MIN_VALLEY_WIDTH).then_some((left + right) / 2.0)
}

/// Move a gutter estimate out of a repeated hanging-label band.
///
/// A single narrow straddler remains a boundary line. Snapping requires the
/// same independent evidence count used to trust the dense-column split, so a
/// one-off centred label or rule cannot erase a legitimate band boundary.
fn snap_split_left_of_hanging_labels(
    spans: &[xberg_native_pdf::layout::TextSpan],
    lines: &[SpanLine],
    page_width: f32,
    mut split_x: f32,
) -> f32 {
    let max_snap_width = page_width * MAX_DENSE_COLUMN_SPLIT_SNAP_SPAN_FRACTION;
    let min_gutter = (page_width * MIN_DENSE_COLUMN_GUTTER_FRACTION).max(MIN_DENSE_COLUMN_GUTTER_PTS);
    for _ in 0..MAX_DENSE_COLUMN_SPLIT_SNAP_PASSES {
        let Some(left_edge) = aligned_hanging_label_left_edge(spans, lines, max_snap_width, min_gutter, split_x) else {
            break;
        };
        split_x = left_edge;
    }
    split_x
}

/// GH#1742: `min_gutter` excludes a table's own grid rows from the candidate pool the
/// same way `detect_split_x` does (`line_has_grid_row_gaps`) -- a numeric table column
/// whose cells straddle the split is otherwise indistinguishable from a stack of
/// hanging clause numbers, and was being snapped to as if it were one.
fn aligned_hanging_label_left_edge(
    spans: &[xberg_native_pdf::layout::TextSpan],
    lines: &[SpanLine],
    max_snap_width: f32,
    min_gutter: f32,
    split_x: f32,
) -> Option<f32> {
    let mut left_edges = lines
        .iter()
        .filter(|&line| !line_has_grid_row_gaps(spans, line, min_gutter))
        .filter_map(|line| {
            line.iter()
                .filter_map(|&index| {
                    let bbox = &spans[index].bbox;
                    (bbox.width > 0.0
                        && bbox.width <= max_snap_width
                        && bbox.left() < split_x
                        && bbox.right() > split_x)
                        .then_some(bbox.left())
                })
                .min_by(f32::total_cmp)
        })
        .collect::<Vec<_>>();
    left_edges.sort_by(f32::total_cmp);

    for (start, &left_edge) in left_edges.iter().enumerate() {
        let aligned_count = left_edges[start..]
            .iter()
            .take_while(|&&candidate| candidate - left_edge <= DENSE_COLUMN_SPLIT_SNAP_X_TOLERANCE_PTS)
            .count();
        if aligned_count >= MIN_DENSE_COLUMN_SPLIT_LINES {
            return Some(left_edge);
        }
    }
    None
}

/// A page region between two consecutive boundary (furniture) lines, in
/// document order.
enum Band {
    /// Ordinary column content: span indices in their existing top-to-bottom,
    /// left-to-right order, offered to `reorder_band_columns` below.
    Content(Vec<usize>),
    /// A single boundary line, emitted where it already sits and never
    /// folded into either column.
    Boundary(SpanLine),
}

/// True if `line` is furniture that separates two bands rather than column
/// content: full-width by `FULL_WIDTH_FURNITURE_FRACTION` (the pre-existing
/// signal), or straddling the page's gutter (`left < split_x < right` for one
/// of its spans). The straddle test is what per-line segmentation adds: it
/// catches furniture narrower than the width threshold that a single
/// whole-page projection could not tell apart from real column content.
fn line_is_boundary(
    spans: &[xberg_native_pdf::layout::TextSpan],
    line: &SpanLine,
    furniture_width: f32,
    split_x: f32,
) -> bool {
    line.iter().any(|&index| {
        let bbox = &spans[index].bbox;
        bbox.width >= furniture_width || (bbox.left() < split_x && bbox.right() > split_x)
    })
}

/// Split the page's lines into bands at boundary lines (the band-splitting
/// step). Consecutive non-boundary lines accumulate into one `Content` band;
/// each boundary line becomes its own single-line `Boundary` band in place,
/// so it stays between the band above it and the band below it.
fn build_bands(
    spans: &[xberg_native_pdf::layout::TextSpan],
    lines: &[SpanLine],
    furniture_width: f32,
    split_x: f32,
) -> Vec<Band> {
    let mut bands = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    for line in lines {
        if !line_is_boundary(spans, line, furniture_width, split_x) {
            current.extend(line.iter().copied());
            continue;
        }
        if !current.is_empty() {
            bands.push(Band::Content(std::mem::take(&mut current)));
        }
        bands.push(Band::Boundary(line.clone()));
    }
    if !current.is_empty() {
        bands.push(Band::Content(current));
    }
    bands
}

/// Try to reorder one content band column-major (per-band column detection).
///
/// Splits the band's spans on `split_x`, then applies the same density
/// (`MIN_DENSE_COLUMN_SPANS_PER_SIDE`) and `classify_region` gates the
/// original whole-page repair used, scoped to this band alone. A band with
/// too few spans on either side, or that fails the prose/reference
/// classification, stays in its existing order — a table or form band is not
/// corrupted by a prose band elsewhere on the same page.
///
/// GH#1655: the density gate counts only inked spans, not raw indices -- a stack of
/// hanging-number tab spaces or blank-line spans on one side must not be able to
/// manufacture a false quorum. `left`/`right` themselves still carry every span,
/// whitespace included, into `classify_region` and the emitted order below: only
/// what gets *counted* changes, not what gets *emitted*, or content would be dropped. ~keep
fn reorder_band_columns(
    spans: &[xberg_native_pdf::layout::TextSpan],
    band: &[usize],
    split_x: f32,
) -> Option<Vec<usize>> {
    let (left, right): (Vec<usize>, Vec<usize>) =
        band.iter().copied().partition(|&index| spans[index].bbox.x < split_x);
    let left_ink = left.iter().filter(|&&index| span_has_ink(&spans[index])).count();
    let right_ink = right.iter().filter(|&&index| span_has_ink(&spans[index])).count();
    if left_ink < MIN_DENSE_COLUMN_SPANS_PER_SIDE || right_ink < MIN_DENSE_COLUMN_SPANS_PER_SIDE {
        return None;
    }
    let left_reorderable = xberg_native_pdf::layout::classify_region(spans, &left).is_reorderable_column();
    let right_reorderable = xberg_native_pdf::layout::classify_region(spans, &right).is_reorderable_column();
    if !left_reorderable && !right_reorderable {
        return None;
    }
    if !(left_reorderable && right_reorderable)
        && cross_gutter_row_pairing_fraction(spans, band, split_x) > MAX_CROSS_GUTTER_ROW_PAIRING_FRACTION
    {
        return None;
    }
    let left = if left_reorderable {
        left
    } else {
        order_region_by_panels(spans, left)
    };
    let right = if right_reorderable {
        right
    } else {
        order_region_by_panels(spans, right)
    };
    Some(left.into_iter().chain(right).collect())
}

/// Group one region's spans into visual rows, top-to-bottom then left-to-right.
fn region_rows(spans: &[xberg_native_pdf::layout::TextSpan], region: &[usize]) -> Vec<SpanLine> {
    let mut order = region.to_vec();
    order.sort_by(|&a, &b| {
        spans[b]
            .bbox
            .y
            .total_cmp(&spans[a].bbox.y)
            .then_with(|| spans[a].bbox.x.total_cmp(&spans[b].bbox.x))
    });
    group_into_lines(spans, &order)
}

/// Fraction of the band's rows that place spans on both sides of `split_x`.
///
/// One table with a label column and a value column pairs every row across the
/// gutter; two regions that merely sit side by side (a table beside a prose
/// column, each on its own leading) pair almost none. That is the difference
/// between a page whose rows carry the meaning and a page whose regions do.
///
/// GH#1655: only inked spans count as pairing evidence -- a whitespace span on the
/// far side of `split_x` (the number-to-title tab of a hanging-number heading) must
/// not be able to fake a row pairing and suppress a legitimate reorder. ~keep
pub(super) fn cross_gutter_row_pairing_fraction(
    spans: &[xberg_native_pdf::layout::TextSpan],
    band: &[usize],
    split_x: f32,
) -> f32 {
    let rows = region_rows(spans, band);
    if rows.is_empty() {
        return 0.0;
    }
    let paired = rows
        .iter()
        .filter(|row| {
            row.iter()
                .any(|&index| span_has_ink(&spans[index]) && spans[index].bbox.x < split_x)
                && row
                    .iter()
                    .any(|&index| span_has_ink(&spans[index]) && spans[index].bbox.x >= split_x)
        })
        .count();
    paired as f32 / rows.len() as f32
}

/// True when `text` is a bare numeric cell rather than a label.
///
/// `-` is deliberately excluded: a range label such as `18-24` is a row label,
/// not a value, and admitting it would make a label column read as numeric.
fn is_numeric_cell(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty()
        && trimmed
            .chars()
            .all(|character| character.is_ascii_digit() || matches!(character, '.' | ',' | '%'))
}

/// Left-edge x positions that at least `MIN_DENSE_COLUMN_SPLIT_LINES` distinct
/// rows agree on — the region's column grid.
fn strong_column_edges(spans: &[xberg_native_pdf::layout::TextSpan], rows: &[SpanLine]) -> Vec<f32> {
    let mut edges: Vec<(f32, usize)> = rows
        .iter()
        .enumerate()
        .flat_map(|(row_index, row)| row.iter().map(move |&index| (index, row_index)))
        .map(|(index, row_index)| (spans[index].bbox.left(), row_index))
        .collect();
    edges.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut columns = Vec::new();
    let mut cluster: Vec<(f32, usize)> = Vec::new();
    for edge in edges {
        let split = cluster
            .first()
            .is_some_and(|&(first, _)| edge.0 - first > DENSE_COLUMN_SPLIT_SNAP_X_TOLERANCE_PTS);
        if split {
            push_supported_column(&mut columns, &cluster);
            cluster.clear();
        }
        cluster.push(edge);
    }
    push_supported_column(&mut columns, &cluster);
    columns
}

fn push_supported_column(columns: &mut Vec<f32>, cluster: &[(f32, usize)]) {
    let Some(&(first, _)) = cluster.first() else {
        return;
    };
    let mut supporting: Vec<usize> = cluster.iter().map(|&(_, row)| row).collect();
    supporting.sort_unstable();
    supporting.dedup();
    if supporting.len() >= MIN_DENSE_COLUMN_SPLIT_LINES {
        columns.push(first);
    }
}

/// Index of the rightmost column whose left edge is at or left of `x`.
fn column_index_for_x(columns: &[f32], x: f32) -> Option<usize> {
    columns
        .iter()
        .rposition(|&column| x >= column - DENSE_COLUMN_SPLIT_SNAP_X_TOLERANCE_PTS)
}

/// Column indices at which a repeated label/value panel restarts, i.e. a
/// predominantly textual column immediately following a predominantly numeric one.
fn panel_boundary_columns(
    spans: &[xberg_native_pdf::layout::TextSpan],
    rows: &[SpanLine],
    columns: &[f32],
) -> Vec<usize> {
    let mut totals = vec![0usize; columns.len()];
    let mut numeric = vec![0usize; columns.len()];
    for &index in rows.iter().flatten() {
        if let Some(column) = column_index_for_x(columns, spans[index].bbox.left()) {
            totals[column] += 1;
            numeric[column] += usize::from(is_numeric_cell(&spans[index].text));
        }
    }
    let fraction = |column: usize| {
        if totals[column] == 0 {
            return None;
        }
        Some(numeric[column] as f32 / totals[column] as f32)
    };
    (0..columns.len().saturating_sub(1))
        .filter(|&column| {
            let (Some(value), Some(label)) = (fraction(column), fraction(column + 1)) else {
                return false;
            };
            value >= MIN_PANEL_VALUE_COLUMN_NUMERIC_FRACTION && label <= MAX_PANEL_LABEL_COLUMN_NUMERIC_FRACTION
        })
        .map(|column| column + 1)
        .collect()
}

fn panels_are_wide_enough(boundaries: &[usize], column_count: usize) -> bool {
    let mut start = 0usize;
    for &boundary in boundaries {
        if boundary.saturating_sub(start) < MIN_COLUMNS_PER_PANEL {
            return false;
        }
        start = boundary;
    }
    column_count.saturating_sub(start) >= MIN_COLUMNS_PER_PANEL
}

/// True when `row`'s spans sit on the column grid rather than running across it.
///
/// A caption or title inside the region is ordinary prose: its words land between
/// the column edges, not on them.
fn row_follows_column_grid(spans: &[xberg_native_pdf::layout::TextSpan], row: &SpanLine, columns: &[f32]) -> bool {
    if row.is_empty() {
        return false;
    }
    let aligned = row
        .iter()
        .filter(|&&index| {
            let left = spans[index].bbox.left();
            columns
                .iter()
                .any(|&column| (left - column).abs() <= DENSE_COLUMN_SPLIT_SNAP_X_TOLERANCE_PTS)
        })
        .count();
    aligned as f32 / row.len() as f32 >= MIN_GRID_ROW_COLUMN_ALIGNMENT_FRACTION
}

/// Reorder a non-prose region panel-major (GH#1545's second symptom).
///
/// A statistics table that repeats the same label/value pair across the page
/// ("Sex % Sex %") welds its two halves on every row when the region is emitted
/// row-major. The panel boundary is not findable by gutter width — it is a few
/// tenths of a point wider than a word space — so it is recovered from the column
/// grid instead. Returns `region` unchanged whenever the grid does not clearly
/// support a split, so an ordinary table is never rearranged on a guess.
fn order_region_by_panels(spans: &[xberg_native_pdf::layout::TextSpan], region: Vec<usize>) -> Vec<usize> {
    let rows = region_rows(spans, &region);
    let columns = strong_column_edges(spans, &rows);
    if columns.len() < MIN_PANEL_SPLIT_COLUMNS {
        return region;
    }
    let boundaries = panel_boundary_columns(spans, &rows, &columns);
    if boundaries.is_empty() || !panels_are_wide_enough(&boundaries, columns.len()) {
        return region;
    }

    let leading = rows
        .iter()
        .take_while(|row| !row_follows_column_grid(spans, row, &columns))
        .count();
    if rows[leading..]
        .iter()
        .any(|row| !row_follows_column_grid(spans, row, &columns))
    {
        return region;
    }

    let panel_of = |index: usize| {
        let column = column_index_for_x(&columns, spans[index].bbox.left());
        boundaries
            .iter()
            .filter(|&&boundary| column.is_some_and(|column| column >= boundary))
            .count()
    };
    let mut ordered: Vec<usize> = rows[..leading].iter().flatten().copied().collect();
    for panel in 0..=boundaries.len() {
        for row in &rows[leading..] {
            ordered.extend(row.iter().copied().filter(|&index| panel_of(index) == panel));
        }
    }
    ordered
}

/// Concatenate bands into the final emission order (the emission-ordering
/// step).
///
/// Each boundary line is emitted between the band above it and the band
/// below it, in true document order — solving the mid-column-furniture
/// placement a single global sort key could not, as a direct consequence of
/// segmenting by band instead of assigning every span one global column
/// position. Returns `None` if not a single band qualified for the column
/// reorder, so the caller can leave `spans` completely untouched rather than
/// apply a no-op permutation.
fn emit_band_order(spans: &[xberg_native_pdf::layout::TextSpan], bands: Vec<Band>, split_x: f32) -> Option<Vec<usize>> {
    let mut any_reordered = false;
    let mut order = Vec::new();
    for band in bands {
        match band {
            Band::Boundary(line) => order.extend(line),
            Band::Content(indices) => match reorder_band_columns(spans, &indices, split_x) {
                Some(reordered) => {
                    any_reordered = true;
                    order.extend(reordered);
                }
                None => order.extend(indices),
            },
        }
    }
    any_reordered.then_some(order)
}

/// Reorder `spans` in place to match `order`, a permutation of
/// `0..spans.len()`.
fn apply_span_order(spans: &mut [xberg_native_pdf::layout::TextSpan], order: &[usize]) {
    let mut taken: Vec<Option<xberg_native_pdf::layout::TextSpan>> =
        spans.iter_mut().map(|span| Some(std::mem::take(span))).collect();
    for (slot, &source) in spans.iter_mut().zip(order) {
        *slot = taken[source].take().expect("each source index is used exactly once");
    }
}

/// Reorder a dense two-column page (issue #1397) that xberg_native_pdf's own
/// `ColumnAware` reading order fails to split.
///
/// Unlike `reorder_sparse_two_column_page` above (which repairs a single
/// guarded four-span sentence), this targets the common case of a full page
/// of two-column body text. GH#1397 follow-up: rather than one global
/// left/right partition, the page is first segmented into horizontal bands at
/// gutter-crossing ("boundary") lines (`build_bands`), and column detection —
/// gutter position via `detect_split_x`, split gate and `classify_region` via
/// `reorder_band_columns` — runs independently per band. A band with a clean
/// gutter splits into two columns; a band without one (or one that fails the
/// prose/reference gate) stays in its existing order; a boundary line is
/// simply emitted where it already sits, between the bands on either side of
/// it.
///
/// This resolves both gaps the earlier single-projection approach left open:
/// furniture narrower than `FULL_WIDTH_FURNITURE_FRACTION` that still crosses
/// the gutter no longer corrupts the *other* lines' gutter evidence (each
/// line's internal gap is checked in isolation), and furniture strictly
/// between the columns' vertical extent lands at its true interleaved
/// position (its own band boundary) instead of a global "after the left
/// column, before the right column" placeholder.
///
/// KNOWN LIMITATIONS (still unhandled): the gutter x-position itself
/// (`split_x`) is detected once for the whole page and reused for every
/// band's left/right partition and for the boundary straddle test — a
/// document whose true gutter shifts between bands (e.g. a differently
/// laid-out region after a full-width figure) is not re-detected per band.
/// Splitting the page into many small bands (frequent short furniture between
/// brief paragraphs) can also starve individual bands of the
/// `MIN_DENSE_COLUMN_SPANS_PER_SIDE` spans-per-side the reorder gate requires,
/// even though the page as a whole is clearly two columns. And columns whose
/// body lines are not row-aligned at all (no line ever has spans from both
/// sides within `LINE_Y_TOLERANCE_PTS`) can starve `detect_split_x` of the
/// per-line evidence it needs.
pub(crate) fn reorder_dense_two_column_page(spans: &mut [xberg_native_pdf::layout::TextSpan], page_width: f32) -> bool {
    let content_left = spans.iter().map(|span| span.bbox.x).fold(f32::INFINITY, f32::min);
    let content_right = spans
        .iter()
        .map(|span| span.bbox.x + span.bbox.width)
        .fold(f32::NEG_INFINITY, f32::max);
    if spans.len() < 2 || content_right - content_left < MIN_DENSE_COLUMN_CONTENT_WIDTH_PTS {
        return false;
    }

    let order = spans_sorted_top_to_bottom(spans);
    let lines = group_into_lines(spans, &order);
    let Some(detected_split_x) = detect_split_x(spans, &lines, page_width) else {
        return false;
    };
    let split_x = snap_split_left_of_hanging_labels(spans, &lines, page_width, detected_split_x);
    let split_x = redirect_split_out_of_content(spans, &lines, page_width, split_x);

    let bands = build_bands(spans, &lines, NO_WIDTH_FURNITURE, split_x);
    let Some(final_order) = emit_band_order(spans, bands, split_x) else {
        return false;
    };

    tracing::debug!(
        target: "xberg::pdf::column_split",
        detected_split_x,
        final_split_x = split_x,
        page_width,
        span_count = spans.len(),
        "dense two-column page reordered"
    );
    apply_span_order(spans, &final_order);
    true
}

/// Build a page's `PageText` (spans + derived chars + dimensions), honouring
/// optional-content (OCG/layer) visibility (issue #67).
///
/// `PdfDocument::extract_page_text_with_options` always treats every layer as
/// visible; a default-OFF `/OCProperties` layer that mirrors the page's content
/// (a common PDF-authoring pattern for redlines/translations/print-vs-screen
/// variants) then contributes a second, hidden-in-every-viewer copy of the page
/// text. When `excluded_layers` is non-empty, this instead calls xberg_native_pdf's
/// filtered span extraction so the surfaced text matches what any viewer
/// actually renders. An empty set is byte-identical to the unfiltered call.
fn page_text_with_options_excluding_layers(
    doc: &xberg_native_pdf::PdfDocument,
    page_index: usize,
    excluded_layers: &std::collections::HashSet<String>,
) -> xberg_native_pdf::error::Result<xberg_native_pdf::layout::PageText> {
    if excluded_layers.is_empty() {
        let mut page = doc.extract_page_text_with_options(page_index, ReadingOrder::ColumnAware)?;
        prefer_tagged_rtl_structure_order(doc, page_index, &mut page);
        return Ok(page);
    }

    let spans = doc.extract_spans_filtered_with_reading_order(
        page_index,
        ReadingOrder::ColumnAware,
        excluded_layers.clone(),
        Default::default(),
    )?;
    let (_, _, page_width, page_height) = doc.get_page_media_box(page_index)?;
    let mut page = xberg_native_pdf::layout::PageText {
        chars: spans.iter().flat_map(|s| s.to_chars()).collect(),
        spans,
        page_width,
        page_height,
    };
    prefer_tagged_rtl_structure_order(doc, page_index, &mut page);
    Ok(page)
}

fn prefer_tagged_rtl_structure_order(
    doc: &xberg_native_pdf::PdfDocument,
    page_index: usize,
    page: &mut xberg_native_pdf::layout::PageText,
) {
    let has_rtl = page.spans.iter().any(|span| {
        span.text
            .chars()
            .any(|character| xberg_native_pdf::text::is_rtl_text(character as u32))
    });
    if !has_rtl {
        return;
    }
    let Ok(ordered) = xberg_native_pdf::pipeline::order_page_spans(doc, page_index, page.spans.clone()) else {
        return;
    };
    if ordered.iter().any(|span| {
        span.order_info.source == xberg_native_pdf::pipeline::ordered_span::ReadingOrderSource::StructureTree
    }) {
        page.spans = ordered.into_iter().map(|span| span.span).collect();
        page.chars = page.spans.iter().flat_map(|span| span.to_chars()).collect();
    }
}

fn page_vertical_bounds(doc: &xberg_native_pdf::PdfDocument, page_index: usize) -> Result<(f32, f32)> {
    let (_, lower_y, _, upper_y) = doc.get_page_media_box(page_index).map_err(|error| {
        PdfError::TextExtractionFailed(format!(
            "Failed to read page {} media box for margin filtering: {error}",
            page_index + 1
        ))
    })?;
    Ok((lower_y.min(upper_y), lower_y.max(upper_y)))
}

pub(crate) fn baseline_is_inside_page_margins(
    baseline_y: f32,
    page_bottom: f32,
    page_top: f32,
    margins: PageMarginFractions,
) -> bool {
    let page_height = page_top - page_bottom;
    if !page_height.is_finite() || page_height <= 0.0 {
        return true;
    }

    let bottom_cutoff = page_bottom + page_height * margins.bottom;
    let top_cutoff = page_top - page_height * margins.top;
    baseline_y >= bottom_cutoff && baseline_y <= top_cutoff
}

/// `(low, high)` of a span's extent along **page** y, honouring its rotation.
///
/// A span's `bbox.width`/`bbox.height` are flattened onto the run's own axis
/// (see `span_geometry`), so for a rotated run the page-y extent is driven by
/// the advance (`width`), not the font height: a 90-degree run advances along
/// page-y. The `span_geometry` helpers work in the span's *upright* frame,
/// where the cross axis maps to page-x for such a run, so they are the wrong
/// tool for a page-space margin test. ~keep
fn span_page_y_extent(span: &xberg_native_pdf::layout::TextSpan) -> (f32, f32) {
    let (sin, cos) = span.rotation_degrees.to_radians().sin_cos();
    let origin = span.bbox.y;
    let advance = span.bbox.width * sin;
    let cross = span.bbox.height * cos;
    let corners = [origin, origin + advance, origin + cross, origin + advance + cross];
    corners
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), corner| {
            (low.min(*corner), high.max(*corner))
        })
}

/// Whether a span escapes the header/footer furniture bands.
///
/// Unrotated spans keep the original single-baseline test byte-for-byte: their
/// origin y is representative of a shallow horizontal line of text. A rotated
/// run's origin is not — a side stamp anchored in the footer band can extend
/// most of the way up the page, and testing only its origin deleted the whole
/// run as furniture (`rotated_text_repair.rs`). Its midpoint is the equivalent
/// representative interior point, so a stamp genuinely confined to the band is
/// still dropped. ~keep
fn span_is_inside_page_margins(
    span: &xberg_native_pdf::layout::TextSpan,
    page_bottom: f32,
    page_top: f32,
    margins: PageMarginFractions,
) -> bool {
    if is_unrotated(span) {
        return baseline_is_inside_page_margins(span.bbox.y, page_bottom, page_top, margins);
    }
    let (low, high) = span_page_y_extent(span);
    baseline_is_inside_page_margins((low + high) / 2.0, page_bottom, page_top, margins)
}

fn retain_spans_inside_page_margins(
    spans: &mut Vec<xberg_native_pdf::layout::TextSpan>,
    page_bottom: f32,
    page_top: f32,
    margins: PageMarginFractions,
) {
    spans.retain(|span| span_is_inside_page_margins(span, page_bottom, page_top, margins));
}

/// Extract text from one page with column-aware ordering and guarded repairs.
///
/// Applies sparse-column and glyph-fragmentation repairs before assembling the
/// page text.
///
/// Also returns the page's fabricated-mapping character counts, captured from the raw spans
/// before margin filtering or reordering mutate them — `Some` only when `excluded_layers` is
/// empty, i.e. `page_text_with_options_excluding_layers` took its fast path and made exactly
/// the same `extract_page_text_with_options(.., ColumnAware)` call that
/// `scan_detect::page_has_fabricated_text` used to make separately for every page (issue
/// #1744). When layers are excluded the two callers would otherwise read different span sets,
/// so provenance is left to its own separate read for such documents; see
/// [`PageProvenanceCounts`]. ~keep
fn extract_page_text_column_aware(
    doc: &xberg_native_pdf::PdfDocument,
    page_index: usize,
    excluded_layers: &std::collections::HashSet<String>,
    margins: PageMarginFractions,
) -> Result<(String, Option<(usize, usize)>)> {
    let (page_bottom, page_top) = page_vertical_bounds(doc, page_index)?;
    let mut widgets = collect_widget_field_values(doc, page_index);
    widgets
        .retain(|(baseline_y, _)| baseline_is_inside_page_margins(*baseline_y as f32, page_bottom, page_top, margins));

    let mut page_text_data = super::guard_native_panic(
        || {
            page_text_with_options_excluding_layers(doc, page_index, excluded_layers).map_err(|e| {
                PdfError::TextExtractionFailed(format!("Page {} text extraction failed: {}", page_index + 1, e))
            })
        },
        |panic| {
            PdfError::TextExtractionFailed(format!(
                "Page {} text extraction panicked in xberg_native_pdf: {}",
                page_index + 1,
                panic
            ))
        },
    )?;

    let provenance_counts = excluded_layers
        .is_empty()
        .then(|| crate::pdf::scan_detect::fabricated_char_counts(&page_text_data.spans));

    retain_spans_inside_page_margins(&mut page_text_data.spans, page_bottom, page_top, margins);

    reorder_sparse_two_column_page(&mut page_text_data.spans, page_text_data.page_width);
    reorder_dense_two_column_page(&mut page_text_data.spans, page_text_data.page_width);

    let rotation_spans = page_text_data.spans.iter().map(rotation_span).collect::<Vec<_>>();
    let mut text = if let Some(repaired) = crate::extractors::pdf::rotation::repair_rotated_page_text(&rotation_spans) {
        repaired
    } else if is_fragmented_span_list(&page_text_data.spans) {
        tracing::debug!(
            span_count = page_text_data.spans.len(),
            "glyph fragmentation detected — rebuilding text from span positions (#962)"
        );
        rebuild_text_from_fragmented_spans(&page_text_data.spans)
    } else {
        assemble_page_text(&page_text_data.spans)
    };

    append_missing_widget_values(&mut text, &widgets);

    Ok((text, provenance_counts))
}

fn rotation_span(span: &xberg_native_pdf::layout::TextSpan) -> crate::extractors::pdf::rotation::TextSpan {
    crate::extractors::pdf::rotation::TextSpan {
        text: span.text.clone(),
        x: span.bbox.x,
        y: span.bbox.y,
        width: span.bbox.width,
        height: span.bbox.height,
        rotation_degrees: span.rotation_degrees,
    }
}

/// Apply common text cleanup: fix control chars and optionally convert HTML.
///
/// Returns a `Cow` to avoid allocation when the text is already clean.
fn apply_text_cleanup(text: &str) -> Cow<'_, str> {
    let cleaned = fix_pdf_control_chars(text);

    #[cfg(feature = "html")]
    if contains_html_markup(&cleaned) {
        return Cow::Owned(crate::pdf::text::convert_html_page_text(&cleaned));
    }

    #[cfg(not(feature = "html"))]
    let _ = contains_html_markup(&cleaned);

    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;
    use xberg_native_pdf::geometry::Rect;
    use xberg_native_pdf::layout::TextSpan;

    fn span(text: &str, x: f32, y: f32, height: f32, font_size: f32) -> TextSpan {
        span_with_width(text, x, y, font_size * 0.6, height, font_size)
    }

    fn span_with_width(text: &str, x: f32, y: f32, width: f32, height: f32, font_size: f32) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            bbox: Rect { x, y, width, height },
            font_size,
            ..TextSpan::default()
        }
    }

    fn extract_fixture_page(relative: &str) -> Option<String> {
        let bytes = crate::utils::read_test_fixture(relative)?;
        let document = xberg_native_pdf::PdfDocument::from_bytes(bytes).expect("RTL PDF fixture must parse");
        Some(
            extract_page_text_column_aware(
                &document,
                0,
                &std::collections::HashSet::new(),
                PageMarginFractions::default(),
            )
            .expect("RTL PDF fixture must extract")
            .0,
        )
    }

    #[test]
    fn tagged_hebrew_page_uses_logical_structure_order() {
        let Some(text) = extract_fixture_page("pdf/rtl_hebrew_native.pdf") else {
            return;
        };
        let positions: Vec<_> = [
            "דוח בדיקת חילוץ בעברית",
            "מספר בקשה:",
            "REQ-2026-104",
            "OCR-v2.1",
            "שלבי הבדיקה",
            "מצב",
        ]
        .iter()
        .map(|anchor| {
            text.find(anchor)
                .unwrap_or_else(|| panic!("missing {anchor:?} in {text:?}"))
        })
        .collect();
        assert!(
            positions.windows(2).all(|pair| pair[0] < pair[1]),
            "wrong logical order: {text:?}"
        );
    }

    #[test]
    fn tagged_arabic_page_uses_logical_structure_order() {
        let Some(text) = extract_fixture_page("pdf/rtl_arabic_native.pdf") else {
            return;
        };
        let positions: Vec<_> = ["تقرير", "رقم الطلب:", "REQ-2026-104", "OCR", "خطوات", "الحالة"]
            .iter()
            .map(|anchor| {
                text.find(anchor)
                    .unwrap_or_else(|| panic!("missing {anchor:?} in {text:?}"))
            })
            .collect();
        assert!(
            positions.windows(2).all(|pair| pair[0] < pair[1]),
            "wrong logical order: {text:?}"
        );
    }

    #[test]
    fn tagged_rtl_filtered_layer_path_uses_logical_structure_order() {
        let Some(bytes) = crate::utils::read_test_fixture("pdf/rtl_hebrew_native.pdf") else {
            return;
        };
        let document = xberg_native_pdf::PdfDocument::from_bytes(bytes).expect("RTL PDF fixture must parse");
        let excluded = std::collections::HashSet::from(["absent-test-layer".to_string()]);
        let page = page_text_with_options_excluding_layers(&document, 0, &excluded)
            .expect("filtered RTL PDF fixture must extract");
        let text = assemble_page_text(&page.spans);
        let positions: Vec<_> = ["מספר בקשה:", "REQ-2026-104", "OCR-v2.1", "שלבי הבדיקה", "מצב"]
            .iter()
            .map(|anchor| {
                text.find(anchor)
                    .unwrap_or_else(|| panic!("missing {anchor:?} in {text:?}"))
            })
            .collect();

        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn numeric_footnote_is_not_glued_to_prose() {
        let spans = vec![
            span_with_width("Il successivo comma 3", 121.1, 317.57, 135.52274, 12.96, 12.96),
            span_with_width("5", 256.73, 317.57, 4.26, 8.52, 8.52),
            span_with_width("5", 85.104, 148.10, 3.24, 6.48, 6.48),
            span_with_width("L’articolo 16", 91.464, 144.62, 52.917, 9.96, 9.96),
        ];
        assert!(assemble_page_text(&spans).starts_with("Il successivo comma 3 5\n"));
        assert!(rebuild_text_from_fragmented_spans(&spans).starts_with("Il successivo comma 3 5\n"));
    }

    #[test]
    fn numeric_footnote_text_rise_gets_a_separator() {
        let body = span_with_width("comma 3", 10.0, 100.0, 40.0, 12.0, 12.0);
        let mut marker = span_with_width("5", 50.1, 100.0, 5.0, 12.0, 12.0);
        marker.text_rise = 0.3;
        let note = [
            span_with_width("5", 10.0, 20.0, 4.0, 6.0, 6.0),
            span_with_width("A supporting note", 15.0, 20.0, 60.0, 8.0, 8.0),
        ];
        assert!(assemble_page_text(&[body, marker, note[0].clone(), note[1].clone()]).starts_with("comma 3 5\n"));
    }

    #[test]
    fn numeric_footnote_keeps_detached_marker_at_its_anchor() {
        let spans = vec![
            span_with_width("comma 3", 100.0, 100.0, 60.0, 12.0, 12.0),
            span_with_width("Next paragraph", 100.0, 70.0, 70.0, 12.0, 12.0),
            span_with_width("5", 160.1, 100.0, 4.0, 8.0, 8.0),
            span_with_width("5", 100.0, 20.0, 4.0, 6.0, 6.0),
            span_with_width("A supporting note", 105.0, 20.0, 60.0, 8.0, 8.0),
        ];
        assert!(assemble_page_text(&spans).starts_with("comma 3 5\n\nNext paragraph\n"));
    }

    #[test]
    fn raised_exponent_in_prose_keeps_its_existing_join() {
        let spans = vec![
            span_with_width("compute 2", 100.0, 100.0, 60.0, 12.0, 12.0),
            span_with_width("3", 160.1, 104.0, 4.0, 8.0, 8.0),
        ];
        assert_eq!(assemble_page_text(&spans), "compute 23");
    }

    #[test]
    fn exponent_with_a_matching_note_number_keeps_its_join() {
        let spans = vec![
            span_with_width("The theorem says that 1", 100.0, 100.0, 120.0, 12.0, 12.0),
            span_with_width("2", 220.1, 100.0, 4.0, 8.0, 8.0),
            span_with_width(" + 1", 224.1, 100.0, 20.0, 12.0, 12.0),
            span_with_width("2", 100.0, 20.0, 4.0, 6.0, 6.0),
            span_with_width("An unrelated footnote", 105.0, 20.0, 60.0, 8.0, 8.0),
        ];
        assert!(assemble_page_text(&spans).starts_with("The theorem says that 12 + 1\n"));
        assert!(rebuild_text_from_fragmented_spans(&spans).starts_with("The theorem says that 12 + 1\n"));
    }

    #[test]
    fn numeric_note_evidence_must_be_smaller_and_below_the_reference() {
        let body = span_with_width("comma 3", 100.0, 100.0, 60.0, 12.0, 12.0);
        let marker = span_with_width("5", 160.1, 100.0, 4.0, 8.0, 8.0);
        for (y, size) in [(200.0, 8.0), (20.0, 12.0)] {
            let definitions = [
                span_with_width("5", 100.0, y, 4.0, 6.0, 6.0),
                span_with_width("A numbered paragraph", 105.0, y, 60.0, size, size),
            ];
            let notes = numeric_notes(&definitions);
            assert!(!needs_numeric_script_boundary(&body, &marker, &notes, None));
        }
        let definitions = [
            span_with_width("5", 100.0, 20.0, 4.0, 6.0, 6.0),
            span_with_width("A supporting note", 105.0, 20.0, 60.0, 8.0, 8.0),
        ];
        let notes = numeric_notes(&definitions);
        let distant = span_with_width("5", 343.0, 100.0, 4.0, 8.0, 8.0);
        assert!(!needs_numeric_script_boundary(&body, &distant, &notes, None));
        let spaced_body = span_with_width("comma 3 ", 100.0, 100.0, 60.0, 12.0, 12.0);
        assert!(!needs_numeric_script_boundary(&spaced_body, &marker, &notes, None));
    }

    #[test]
    fn numeric_footnote_controls_keep_existing_joins() {
        for (body, script, size, expected) in [
            ("3", "5", 12.0, "35"),
            ("x", "2", 8.0, "x2"),
            ("there are 2", "2", 8.0, "there are 22"),
            ("2", "32+1", 8.0, "232+1"),
        ] {
            let spans = vec![
                span_with_width(body, 10.0, 100.0, 40.0, 12.0, 12.0),
                span_with_width(script, 50.1, 100.0, 8.0, size, size),
            ];
            assert_eq!(assemble_page_text(&spans), expected);
            assert_eq!(rebuild_text_from_fragmented_spans(&spans), expected);
        }
    }

    #[test]
    fn should_exclude_native_spans_by_configured_page_margins() {
        let mut spans = vec![
            span("header", 20.0, 950.0, 10.0, 10.0),
            span("top boundary", 20.0, 900.0, 10.0, 10.0),
            span("body", 20.0, 400.0, 10.0, 10.0),
            span("bottom boundary", 20.0, 100.0, 10.0, 10.0),
            span("footer", 20.0, 40.0, 10.0, 10.0),
        ];

        retain_spans_inside_page_margins(
            &mut spans,
            0.0,
            1000.0,
            PageMarginFractions {
                top: 0.10,
                bottom: 0.10,
            },
        );

        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            ["top boundary", "body", "bottom boundary"]
        );
    }

    /// A 90-degree side stamp anchored in the footer band must survive: its
    /// origin sits inside the band, but a rotated run advances along page-y, so
    /// the stamp reaches far into the body. Testing only `bbox.y` deleted the
    /// whole run (`rotated_text_repair.rs`'s side-stamp assertion). The second
    /// span is the control: rotated too, but genuinely confined to the band, so
    /// the filter must still drop it. ~keep
    #[test]
    fn should_keep_a_rotated_side_stamp_that_reaches_out_of_the_footer_band() {
        let mut stamp = span_with_width("side stamp", 60.0, 18.0, 112.0, 11.0, 9.0);
        stamp.rotation_degrees = 90.0;
        let mut confined = span_with_width("rotated footer", 300.0, 18.0, 12.0, 11.0, 9.0);
        confined.rotation_degrees = 90.0;

        let mut spans = vec![stamp, confined];
        // #1574: `PageMarginFractions::default()` is now 0.0/0.0 (opt-in filter), so this
        // geometry test -- which is about the rotated-run advance, not about defaults --
        // uses the pre-1574 default fractions explicitly. ~keep
        retain_spans_inside_page_margins(
            &mut spans,
            0.0,
            792.0,
            PageMarginFractions {
                top: 0.06,
                bottom: 0.05,
            },
        );

        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            ["side stamp"],
            "a rotated run reaching into the body must survive while one confined to the band is dropped"
        );
    }

    /// #1574: default margins are 0.0, so `PageMarginFractions::default()` must not
    /// remove anything -- the header and footer here would have been dropped by the
    /// pre-fix 0.06/0.05 defaults. ~keep
    #[test]
    fn should_not_filter_by_default_margins() {
        let mut spans = vec![
            span("header", 20.0, 860.0, 10.0, 10.0),
            span("body", 20.0, 500.0, 10.0, 10.0),
            span("footer", 20.0, 130.0, 10.0, 10.0),
        ];

        retain_spans_inside_page_margins(&mut spans, 100.0, 900.0, PageMarginFractions::default());

        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            ["header", "body", "footer"]
        );
    }

    /// Same geometry as the removed default-margin case above, but with explicit
    /// non-zero fractions -- keeps the non-zero-page-origin accounting under test now
    /// that the defaults themselves resolve to 0.0.
    #[test]
    fn should_resolve_configured_margins_and_account_for_non_zero_page_origin() {
        let mut spans = vec![
            span("header", 20.0, 860.0, 10.0, 10.0),
            span("body", 20.0, 500.0, 10.0, 10.0),
            span("footer", 20.0, 130.0, 10.0, 10.0),
        ];

        retain_spans_inside_page_margins(
            &mut spans,
            100.0,
            900.0,
            PageMarginFractions {
                top: 0.06,
                bottom: 0.05,
            },
        );

        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            ["body"]
        );
    }

    /// #1574: a default-config document (`pdf_options: None`, and `Some(PdfConfig::default())`
    /// with both margin fields `None`) must resolve to no filtering at all -- through 1.1.0 this
    /// resolved to 0.06/0.05 and silently dropped a top-of-page title on every default scan.
    #[test]
    fn should_resolve_no_margins_for_a_default_config() {
        let margins = PageMarginFractions::from_extraction_config(None);
        assert_eq!(margins.top, 0.0);
        assert_eq!(margins.bottom, 0.0);

        let margins = PageMarginFractions::from_extraction_config(Some(&ExtractionConfig::default()));
        assert_eq!(margins.top, 0.0);
        assert_eq!(margins.bottom, 0.0);

        let config = ExtractionConfig {
            pdf_options: Some(crate::core::config::PdfConfig::default()),
            ..ExtractionConfig::default()
        };
        let margins = PageMarginFractions::from_extraction_config(Some(&config));
        assert_eq!(margins.top, 0.0);
        assert_eq!(margins.bottom, 0.0);
    }

    #[test]
    fn should_disable_respective_pdf_margin_when_content_filter_includes_furniture() {
        let mut config = ExtractionConfig {
            pdf_options: Some(crate::core::config::PdfConfig {
                top_margin_fraction: Some(0.25),
                bottom_margin_fraction: Some(0.20),
                ..crate::core::config::PdfConfig::default()
            }),
            ..ExtractionConfig::default()
        };
        config.content_filter = Some(crate::core::config::ContentFilterConfig {
            include_headers: true,
            ..crate::core::config::ContentFilterConfig::default()
        });

        let margins = PageMarginFractions::from_extraction_config(Some(&config));

        assert_eq!(margins.top, 0.0);
        assert_eq!(margins.bottom, 0.20);

        config.content_filter = Some(crate::core::config::ContentFilterConfig {
            include_footers: true,
            ..crate::core::config::ContentFilterConfig::default()
        });
        let margins = PageMarginFractions::from_extraction_config(Some(&config));

        assert_eq!(margins.top, 0.25);
        assert_eq!(margins.bottom, 0.0);
    }

    /// Build a list of N single-char spans that each trigger a same-line x-disorder
    /// event. All at the same y (zero height fallback path), each span's x is
    /// `prev.x - font_size - 1` so cur.x < prev.x - font_size is always true.
    fn disorder_spans(count: usize) -> Vec<TextSpan> {
        let font_size = 12.0_f32;
        let mut spans = Vec::with_capacity(count + 1);
        let mut x = 300.0_f32;
        for _i in 0..=count {
            spans.push(span("A", x, 700.0, 0.0, font_size));
            x = x - font_size - 1.0;
        }
        spans
    }

    #[test]
    fn fragmentation_detected_at_threshold() {
        let spans = disorder_spans(MIN_DISORDER_COUNT);
        assert!(
            is_fragmented_span_list(&spans),
            "should detect fragmentation at exactly MIN_DISORDER_COUNT ({MIN_DISORDER_COUNT}) events"
        );
    }

    #[test]
    fn fragmentation_not_detected_below_threshold() {
        let spans = disorder_spans(MIN_DISORDER_COUNT - 1);
        assert!(
            !is_fragmented_span_list(&spans),
            "must NOT detect fragmentation with {} events (threshold is {MIN_DISORDER_COUNT})",
            MIN_DISORDER_COUNT - 1
        );
    }

    #[test]
    fn long_spans_never_count_toward_disorder() {
        let font_size = 12.0_f32;
        let mut spans = Vec::new();
        let mut x = 500.0_f32;
        for _ in 0..20 {
            spans.push(span("word", x, 700.0, 0.0, font_size));
            x = x - font_size - 1.0;
        }
        assert!(
            !is_fragmented_span_list(&spans),
            "word-level spans (> 3 chars) must never trigger fragmentation detection"
        );
    }

    #[test]
    fn large_y_gap_not_classified_as_same_line() {
        let spans = vec![span("A", 300.0, 700.0, 0.0, 12.0), span("B", 50.0, 686.0, 0.0, 12.0)];
        assert!(
            !is_fragmented_span_list(&spans),
            "14 pt y-gap must not be classified as same-line (MAX_GLYPH_JITTER_PT={MAX_GLYPH_JITTER_PT})"
        );
    }

    #[test]
    fn empty_spans_returns_false() {
        assert!(!is_fragmented_span_list(&[]));
    }

    #[test]
    fn single_span_returns_false() {
        assert!(!is_fragmented_span_list(&[span("A", 100.0, 700.0, 0.0, 12.0)]));
    }

    #[test]
    fn detached_subscripts_are_reinserted_into_chemical_formula() {
        let spans = vec![
            span_with_width("H", 100.0, 100.0, 6.0, 10.0, 10.0),
            span_with_width("SO", 108.0, 100.0, 12.0, 10.0, 10.0),
            span_with_width("solution", 124.0, 100.0, 36.0, 10.0, 10.0),
            span_with_width("2", 106.0, 96.0, 2.0, 6.0, 6.0),
            span_with_width("4", 120.0, 96.0, 2.0, 6.0, 6.0),
        ];

        assert_eq!(assemble_page_text(&spans), "H2SO4 solution");
    }

    #[test]
    fn detached_phone_suffix_is_reinserted_without_space() {
        let spans = vec![
            span_with_width("273.879.750", 100.0, 100.0, 60.0, 10.0, 10.0),
            span_with_width("Population", 100.0, 75.0, 45.0, 10.0, 10.0),
            span_with_width("1", 160.0, 103.0, 3.0, 6.0, 6.0),
        ];

        assert_eq!(assemble_page_text(&spans), "273.879.7501\n\nPopulation");
    }

    #[test]
    fn detached_final_glyph_is_reinserted_into_word() {
        let spans = vec![
            span_with_width("eli", 100.0, 100.0, 15.0, 10.0, 10.0),
            span_with_width("Table", 40.0, 75.0, 25.0, 10.0, 10.0),
            span_with_width("t", 115.0, 100.0, 5.0, 10.0, 10.0),
        ];

        assert_eq!(assemble_page_text(&spans), "elit\n\nTable");
    }

    #[test]
    fn far_left_reset_starts_new_row_even_when_vertical_bands_overlap() {
        let spans = vec![
            span_with_width("1.000", 500.0, 100.0, 30.0, 10.0, 10.0),
            span_with_width("002", 30.0, 99.0, 18.0, 10.0, 10.0),
        ];

        assert_eq!(assemble_page_text(&spans), "1.000\n002");
    }

    #[test]
    fn far_left_reset_does_not_split_rtl_text() {
        let mut next = span_with_width("العالم", 430.0, 100.0, 35.0, 10.0, 10.0);
        next.split_boundary_before = true;
        let spans = vec![span_with_width("مرحبا", 500.0, 100.0, 30.0, 10.0, 10.0), next];

        assert_eq!(assemble_page_text(&spans), "مرحبا العالم");
    }

    #[test]
    fn rtl_advance_separates_distinct_table_cells() {
        let spans = vec![
            span_with_width("الحالة", 500.0, 100.0, 40.0, 10.0, 10.0),
            span_with_width("الرمز", 300.0, 100.0, 40.0, 10.0, 10.0),
            span_with_width("العدد", 100.0, 100.0, 40.0, 10.0, 10.0),
        ];

        assert_eq!(assemble_page_text(&spans), "الحالة الرمز العدد");
    }

    #[test]
    fn far_left_reset_respects_rtl_span_metadata_for_ascii_text() {
        let mut previous = span_with_width("first", 500.0, 100.0, 30.0, 10.0, 10.0);
        previous.rtl_draw_logical = true;
        let mut next = span_with_width("second", 430.0, 100.0, 35.0, 10.0, 10.0);
        next.rtl_draw_logical = true;
        next.split_boundary_before = true;

        assert_eq!(assemble_page_text(&[previous, next]), "first second");
    }

    #[test]
    fn far_left_reset_does_not_split_ascii_numbers_on_rtl_page() {
        let mut number = span_with_width("123", 500.0, 100.0, 20.0, 10.0, 10.0);
        number.split_boundary_before = true;
        let mut next_number = span_with_width("456", 430.0, 100.0, 20.0, 10.0, 10.0);
        next_number.split_boundary_before = true;
        let spans = vec![
            span_with_width("مرحبا", 570.0, 100.0, 30.0, 10.0, 10.0),
            number,
            next_number,
        ];

        assert_eq!(assemble_page_text(&spans), "مرحبا 123 456");
    }

    #[test]
    fn moderate_math_backtrack_does_not_start_new_row() {
        let mut denominator = span_with_width("denominator", 65.0, 96.0, 55.0, 10.0, 10.0);
        denominator.split_boundary_before = true;
        let spans = vec![
            span_with_width("numerator", 100.0, 104.0, 45.0, 10.0, 10.0),
            denominator,
        ];

        assert_eq!(assemble_page_text(&spans), "numerator denominator");
    }

    #[test]
    fn far_left_reset_does_not_split_rotated_text() {
        let mut previous = span_with_width("first", 500.0, 100.0, 30.0, 10.0, 10.0);
        previous.rotation_degrees = 90.0;
        let mut next = span_with_width("second", 430.0, 100.0, 35.0, 10.0, 10.0);
        next.rotation_degrees = 90.0;
        next.split_boundary_before = true;

        assert_eq!(assemble_page_text(&[previous, next]), "first second");
    }

    /// A span painted with a rotated text matrix. `x`/`y` stay page-space (that
    /// is what xberg_native_pdf reports); `width` is the glyph-advance run along the
    /// rotated baseline and `height` the font extent across it.
    fn rotated_span(text: &str, x: f32, y: f32, width: f32, height: f32, rotation_degrees: f32) -> TextSpan {
        let mut span = span_with_width(text, x, y, width, height, height);
        span.rotation_degrees = rotation_degrees;
        span
    }

    /// #1358 / #294 — a detached fragment of a rotated word must rejoin its
    /// parent instead of being stranded at the end of the run.
    ///
    /// Revert check (expect RED): restore the `rotation_degrees.abs() <=
    /// f32::EPSILON` term in `span_geometry::is_ltr_writing_mode`'s callers —
    /// i.e. use `is_horizontal_ltr` again in `find_inline_fragment_anchor` — and
    /// this asserts `"MotorcrafPremiumt"`.
    #[test]
    fn should_rejoin_detached_fragment_of_rotated_word_when_rotation_matches() {
        let spans = vec![
            rotated_span("Motorcraf", 400.0, 100.0, 45.0, 10.0, 90.0),
            rotated_span("Premium", 400.0, 155.0, 40.0, 10.0, 90.0),
            rotated_span("t", 400.0, 145.0, 5.0, 10.0, 90.0),
        ];

        assert_eq!(assemble_page_text(&spans), "Motorcraft Premium");
    }

    /// #1358 / #294 — the anchor must still refuse to bridge two different
    /// rotations, so a rotated fragment never steals an upright parent.
    #[test]
    fn should_not_anchor_fragment_across_differing_rotations() {
        let spans = vec![
            span_with_width("Motorcraf", 400.0, 100.0, 45.0, 10.0, 10.0),
            rotated_span("t", 445.0, 100.0, 5.0, 10.0, 90.0),
        ];

        assert_eq!(find_inline_fragment_anchor(1, &spans, &[None, None]), None);
    }

    /// #1358 / #293 — a sideways table reads down its own rows, not across
    /// them: words on one rotated line are space-joined and the next rotated
    /// line starts a new line.
    ///
    /// Revert check (expect RED): restore the page-axis `y_gap` / `bbox.x`
    /// arithmetic in `append_span_separator` and this asserts
    /// `"Enginecoolant\n\n18.6\n\nquarts"` — every word of a line glued, every
    /// line boundary turned into a paragraph break.
    #[test]
    fn should_read_rotated_table_rows_along_their_own_axis() {
        let spans = vec![
            rotated_span("Engine", 400.0, 100.0, 30.0, 10.0, 90.0),
            rotated_span("coolant", 400.0, 132.0, 32.0, 10.0, 90.0),
            rotated_span("18.6", 388.0, 100.0, 22.0, 10.0, 90.0),
            rotated_span("quarts", 388.0, 124.0, 30.0, 10.0, 90.0),
        ];

        assert_eq!(assemble_page_text(&spans), "Engine coolant\n18.6 quarts");
    }

    /// #1358 / #293 — the mixed page. A whole-page rotation transform would fix
    /// the rotated body and break the upright running footer; only a per-run
    /// frame reads both correctly, with a hard block break between them.
    ///
    /// Revert check (expect RED): with the page-axis arithmetic restored this
    /// asserts `"Enginecoolant\n\n18.6\n\nquarts\n\nPage 264"`.
    #[test]
    fn should_read_rotated_body_and_upright_footer_on_same_page() {
        let spans = vec![
            rotated_span("Engine", 400.0, 100.0, 30.0, 10.0, 90.0),
            rotated_span("coolant", 400.0, 132.0, 32.0, 10.0, 90.0),
            rotated_span("18.6", 388.0, 100.0, 22.0, 10.0, 90.0),
            rotated_span("quarts", 388.0, 124.0, 30.0, 10.0, 90.0),
            span_with_width("Page", 60.0, 40.0, 25.0, 10.0, 10.0),
            span_with_width("264", 88.0, 40.0, 15.0, 10.0, 10.0),
        ];

        assert_eq!(assemble_page_text(&spans), "Engine coolant\n18.6 quarts\n\nPage 264");
    }

    /// #1358 — upright pages must be byte-identical after the change. Two
    /// wrapped body lines plus a paragraph break, all rotation 0.
    #[test]
    fn should_not_change_upright_page_assembly() {
        let spans = vec![
            span_with_width("Engine", 60.0, 700.0, 30.0, 10.0, 10.0),
            span_with_width("coolant", 92.0, 700.0, 32.0, 10.0, 10.0),
            span_with_width("18.6", 60.0, 688.0, 22.0, 10.0, 10.0),
            span_with_width("quarts", 84.0, 688.0, 30.0, 10.0, 10.0),
            span_with_width("Next", 60.0, 640.0, 25.0, 10.0, 10.0),
        ];

        assert_eq!(assemble_page_text(&spans), "Engine coolant\n18.6 quarts\n\nNext");
    }

    #[test]
    fn inline_fragment_anchor_rejects_non_ltr_geometry() {
        let mut anchor = span_with_width("word", 100.0, 100.0, 30.0, 10.0, 10.0);
        anchor.rtl_draw_logical = true;
        let mut fragment = span_with_width("2", 130.0, 100.0, 3.0, 6.0, 6.0);
        fragment.rtl_draw_logical = true;
        let spans = vec![anchor, fragment];

        assert_eq!(find_inline_fragment_anchor(1, &spans, &[None, None]), None);
    }

    #[test]
    fn inline_fragment_anchor_search_is_local() {
        let mut spans = vec![span_with_width("anchor", 100.0, 100.0, 30.0, 10.0, 10.0)];
        spans.extend(
            (0..=MAX_INLINE_FRAGMENT_ANCHOR_LOOKBACK)
                .map(|index| span_with_width("filler", 300.0, index as f32, 30.0, 10.0, 10.0)),
        );
        spans.push(span_with_width("2", 130.0, 100.0, 3.0, 6.0, 6.0));
        let anchors = vec![None; spans.len()];

        assert_eq!(find_inline_fragment_anchor(spans.len() - 1, &spans, &anchors), None);
    }

    #[test]
    fn split_boundary_before_forces_space_between_adjacent_spans() {
        let mut next = span_with_width("002", 130.0, 100.0, 18.0, 10.0, 10.0);
        next.split_boundary_before = true;
        let spans = vec![span_with_width("1.000", 100.0, 100.0, 30.0, 10.0, 10.0), next];

        assert_eq!(assemble_page_text(&spans), "1.000 002");
    }

    #[test]
    fn line_local_repair_preserves_column_aware_order() {
        let spans = vec![
            span_with_width("left-top", 40.0, 100.0, 40.0, 10.0, 10.0),
            span_with_width("left-bottom", 40.0, 80.0, 50.0, 10.0, 10.0),
            span_with_width("right-top", 300.0, 100.0, 45.0, 10.0, 10.0),
            span_with_width("right-bottom", 300.0, 80.0, 55.0, 10.0, 10.0),
        ];

        assert_eq!(
            assemble_page_text(&spans),
            "left-top\n\nleft-bottom\n\nright-top\n\nright-bottom"
        );
    }

    #[test]
    fn sparse_two_column_prose_reorders_by_column() {
        let mut spans = vec![
            span_with_width("The committee reviewed the annual", 60.0, 712.0, 175.0, 11.0, 11.0),
            span_with_width("approved the budget for the", 330.0, 712.0, 145.0, 11.0, 11.0),
            span_with_width("report and", 60.0, 698.0, 52.0, 11.0, 11.0),
            span_with_width("coming fiscal year.", 330.0, 698.0, 92.0, 11.0, 11.0),
        ];

        assert!(reorder_sparse_two_column_page(&mut spans, 612.0));

        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            [
                "The committee reviewed the annual",
                "report and",
                "approved the budget for the",
                "coming fiscal year."
            ]
        );
    }

    #[test]
    fn sparse_two_column_table_keeps_row_order() {
        let mut spans = vec![
            span_with_width(
                "Regional revenue for the northern market.",
                60.0,
                712.0,
                210.0,
                11.0,
                11.0,
            ),
            span_with_width("Annual total for the current period.", 330.0, 712.0, 190.0, 11.0, 11.0),
            span_with_width(
                "Operating expense for the northern market.",
                60.0,
                698.0,
                220.0,
                11.0,
                11.0,
            ),
            span_with_width(
                "Quarterly total for the current period.",
                330.0,
                698.0,
                200.0,
                11.0,
                11.0,
            ),
        ];
        let original = spans.iter().map(|span| span.text.clone()).collect::<Vec<_>>();

        assert!(!reorder_sparse_two_column_page(&mut spans, 612.0));

        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            original.iter().map(String::as_str).collect::<Vec<_>>()
        );
    }

    #[test]
    fn sparse_verbose_form_keeps_row_order() {
        let mut spans = vec![
            span_with_width(
                "Account holder full legal name appears here:",
                60.0,
                712.0,
                215.0,
                11.0,
                11.0,
            ),
            span_with_width(
                "Mailing address for all official correspondence:",
                330.0,
                712.0,
                225.0,
                11.0,
                11.0,
            ),
            span_with_width(
                "Emergency contact relationship and telephone number:",
                60.0,
                698.0,
                235.0,
                11.0,
                11.0,
            ),
            span_with_width(
                "Preferred delivery method for annual notices:",
                330.0,
                698.0,
                215.0,
                11.0,
                11.0,
            ),
        ];
        let original = spans.iter().map(|span| span.text.clone()).collect::<Vec<_>>();

        assert!(!reorder_sparse_two_column_page(&mut spans, 612.0));
        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            original.iter().map(String::as_str).collect::<Vec<_>>()
        );
    }

    #[test]
    fn sparse_lowercase_table_keeps_row_order() {
        let mut spans = vec![
            span_with_width(
                "regional revenue for the northern market",
                60.0,
                712.0,
                210.0,
                11.0,
                11.0,
            ),
            span_with_width("annual total for the current period", 330.0, 712.0, 190.0, 11.0, 11.0),
            span_with_width(
                "operating expense for the northern market",
                60.0,
                698.0,
                220.0,
                11.0,
                11.0,
            ),
            span_with_width(
                "quarterly total for the current period.",
                330.0,
                698.0,
                200.0,
                11.0,
                11.0,
            ),
        ];
        let original = spans.iter().map(|span| span.text.clone()).collect::<Vec<_>>();

        assert!(!reorder_sparse_two_column_page(&mut spans, 612.0));
        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            original.iter().map(String::as_str).collect::<Vec<_>>()
        );
    }

    /// Build the interleaved (pre-fix) span order a dense two-column page
    /// naturally arrives in: sorted by full-page-width Y, so left- and
    /// right-column lines at the same height are adjacent. Each column is one
    /// coherent paragraph behind a one-word heading, mirroring GH#1397
    /// ("Funding" / "References" welded together at the same height).
    fn dense_two_column_spans() -> Vec<TextSpan> {
        const LEFT_X: f32 = 60.0;
        const RIGHT_X: f32 = 320.0;
        let left_heading = span_with_width("Funding", LEFT_X, 830.0, 70.0, 11.0, 11.0);
        let right_heading = span_with_width("References", RIGHT_X, 830.0, 90.0, 11.0, 11.0);
        let left_body = [
            "The committee reviewed annual budget totals",
            "and approved new funding for the coming year",
            "after several rounds of careful review by",
            "senior staff members from every department",
            "who evaluated priorities across the whole",
            "organization before reaching a final decision",
            "that reflected both short and long term goals",
            "for sustainable growth across all programs",
        ];
        let right_body = [
            "Numerous studies have examined similar",
            "programs across comparable institutions",
            "using consistent methodology and controls",
            "for measuring outcomes over multiple years",
            "researchers found consistent positive trends",
            "supporting continued investment going forward",
            "additional citations appear in the appendix",
            "for readers seeking further detail here",
        ];

        let mut spans = vec![left_heading, right_heading];
        for (row, (left_line, right_line)) in left_body.iter().copied().zip(right_body.iter().copied()).enumerate() {
            let y = 816.0 - row as f32 * 14.0;
            spans.push(span_with_width(left_line, LEFT_X, y, 200.0, 11.0, 11.0));
            spans.push(span_with_width(right_line, RIGHT_X, y, 190.0, 11.0, 11.0));
        }
        spans
    }

    #[test]
    fn dense_two_column_prose_reorders_by_column() {
        let mut spans = dense_two_column_spans();

        assert!(reorder_dense_two_column_page(&mut spans, 612.0));

        let texts = spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>();
        assert_eq!(
            texts,
            [
                "Funding",
                "The committee reviewed annual budget totals",
                "and approved new funding for the coming year",
                "after several rounds of careful review by",
                "senior staff members from every department",
                "who evaluated priorities across the whole",
                "organization before reaching a final decision",
                "that reflected both short and long term goals",
                "for sustainable growth across all programs",
                "References",
                "Numerous studies have examined similar",
                "programs across comparable institutions",
                "using consistent methodology and controls",
                "for measuring outcomes over multiple years",
                "researchers found consistent positive trends",
                "supporting continued investment going forward",
                "additional citations appear in the appendix",
                "for readers seeking further detail here",
            ]
        );
    }

    const GH1484_PAGE_WIDTH: f32 = 595.0;
    const GH1484_RIGHT_NUMBER_X: f32 = 304.87;

    fn dense_two_column_hanging_number_spans() -> Vec<TextSpan> {
        const LEFT_NUMBER_X: f32 = 36.0;
        const LEFT_TEXT_X: f32 = 64.34;
        const LEFT_TEXT_WIDTH: f32 = 226.8;
        const RIGHT_TEXT_X: f32 = 333.19;
        const ROW_COUNT: usize = 12;

        let mut spans = Vec::new();
        for row in 0..ROW_COUNT {
            let y = 816.0 - row as f32 * 14.0;
            if row.is_multiple_of(2) {
                spans.push(span_with_width(
                    &format!("15.{}", row / 2 + 1),
                    LEFT_NUMBER_X,
                    y,
                    17.84,
                    11.0,
                    11.0,
                ));
            }
            spans.push(span_with_width(
                &format!("The left clause line {row} continues with ordinary agreement terms"),
                LEFT_TEXT_X,
                y,
                LEFT_TEXT_WIDTH,
                11.0,
                11.0,
            ));
            if row.is_multiple_of(2) {
                spans.push(span_with_width(
                    &format!("16.{}", row / 2 + 5),
                    GH1484_RIGHT_NUMBER_X,
                    y,
                    17.84,
                    11.0,
                    11.0,
                ));
            }
            spans.push(span_with_width(
                &format!("The right clause line {row} continues with ordinary agreement terms"),
                RIGHT_TEXT_X,
                y,
                220.0,
                11.0,
                11.0,
            ));
        }
        spans
    }

    /// GH#1484: alternating numbered and continuation lines yield two gutter-midpoint
    /// populations. Their median lands just inside the right column's hanging-number
    /// band, so every numbered line used to become a boundary and starve the content
    /// bands below the dense-column population gate.
    #[test]
    fn dense_two_column_hanging_numbers_reorder_by_column() {
        let mut spans = dense_two_column_hanging_number_spans();
        let expected = spans
            .iter()
            .filter(|span| span.bbox.x < GH1484_RIGHT_NUMBER_X)
            .chain(spans.iter().filter(|span| span.bbox.x >= GH1484_RIGHT_NUMBER_X))
            .map(|span| span.text.clone())
            .collect::<Vec<_>>();

        let order = spans_sorted_top_to_bottom(&spans);
        let lines = group_into_lines(&spans, &order);
        let detected_split = detect_split_x(&spans, &lines, GH1484_PAGE_WIDTH).expect("numbered page has a gutter");
        assert!(detected_split > GH1484_RIGHT_NUMBER_X && detected_split < GH1484_RIGHT_NUMBER_X + 17.84);
        assert_eq!(
            snap_split_left_of_hanging_labels(&spans, &lines, GH1484_PAGE_WIDTH, detected_split),
            GH1484_RIGHT_NUMBER_X
        );

        assert!(reorder_dense_two_column_page(&mut spans, GH1484_PAGE_WIDTH));

        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            expected.iter().map(String::as_str).collect::<Vec<_>>()
        );
    }

    #[test]
    fn dense_two_column_unnumbered_control_keeps_detected_split() {
        const PAGE_WIDTH: f32 = 612.0;
        let spans = dense_two_column_spans();
        let order = spans_sorted_top_to_bottom(&spans);
        let lines = group_into_lines(&spans, &order);
        let detected_split = detect_split_x(&spans, &lines, PAGE_WIDTH).expect("control has a gutter");

        assert_eq!(
            snap_split_left_of_hanging_labels(&spans, &lines, PAGE_WIDTH, detected_split),
            detected_split
        );
    }

    #[test]
    fn dense_column_split_does_not_snap_to_furniture_or_one_off_label() {
        const PAGE_WIDTH: f32 = 595.0;
        const SPLIT_X: f32 = 300.0;
        let mut furniture = (0..MIN_DENSE_COLUMN_SPLIT_LINES)
            .map(|row| span_with_width("centred furniture", 250.0, 800.0 - row as f32 * 14.0, 100.0, 11.0, 11.0))
            .collect::<Vec<_>>();
        furniture.push(span_with_width("note", 295.0, 700.0, 20.0, 11.0, 11.0));
        let order = spans_sorted_top_to_bottom(&furniture);
        let lines = group_into_lines(&furniture, &order);

        assert_eq!(
            snap_split_left_of_hanging_labels(&furniture, &lines, PAGE_WIDTH, SPLIT_X),
            SPLIT_X
        );
    }

    #[test]
    fn dense_column_split_repeats_when_first_snap_reveals_another_fragment() {
        const PAGE_WIDTH: f32 = 595.0;
        const INITIAL_SPLIT_X: f32 = 305.64;
        const FIRST_FRAGMENT_LEFT: f32 = 304.87;
        const SECOND_FRAGMENT_LEFT: f32 = 300.0;

        let mut spans = Vec::new();
        for row in 0..MIN_DENSE_COLUMN_SPLIT_LINES {
            let y = 800.0 - row as f32 * 14.0;
            spans.push(span_with_width("prefix", SECOND_FRAGMENT_LEFT, y, 5.2, 11.0, 11.0));
            spans.push(span_with_width("number", FIRST_FRAGMENT_LEFT, y, 17.84, 11.0, 11.0));
        }
        let order = spans_sorted_top_to_bottom(&spans);
        let lines = group_into_lines(&spans, &order);
        let max_snap_width = PAGE_WIDTH * MAX_DENSE_COLUMN_SPLIT_SNAP_SPAN_FRACTION;
        let min_gutter = (PAGE_WIDTH * MIN_DENSE_COLUMN_GUTTER_FRACTION).max(MIN_DENSE_COLUMN_GUTTER_PTS);

        assert_eq!(
            aligned_hanging_label_left_edge(&spans, &lines, max_snap_width, min_gutter, INITIAL_SPLIT_X),
            Some(FIRST_FRAGMENT_LEFT)
        );
        assert_eq!(
            aligned_hanging_label_left_edge(&spans, &lines, max_snap_width, min_gutter, FIRST_FRAGMENT_LEFT),
            Some(SECOND_FRAGMENT_LEFT)
        );
        assert_eq!(
            snap_split_left_of_hanging_labels(&spans, &lines, PAGE_WIDTH, INITIAL_SPLIT_X),
            SECOND_FRAGMENT_LEFT
        );
    }

    #[test]
    fn dense_two_column_prose_assembles_without_interleaving_or_heading_weld() {
        let mut spans = dense_two_column_spans();

        assert!(reorder_dense_two_column_page(&mut spans, 612.0));

        assert_eq!(
            assemble_page_text(&spans),
            "Funding\n\
             The committee reviewed annual budget totals\n\
             and approved new funding for the coming year\n\
             after several rounds of careful review by\n\
             senior staff members from every department\n\
             who evaluated priorities across the whole\n\
             organization before reaching a final decision\n\
             that reflected both short and long term goals\n\
             for sustainable growth across all programs\n\n\
             References\n\
             Numerous studies have examined similar\n\
             programs across comparable institutions\n\
             using consistent methodology and controls\n\
             for measuring outcomes over multiple years\n\
             researchers found consistent positive trends\n\
             supporting continued investment going forward\n\
             additional citations appear in the appendix\n\
             for readers seeking further detail here"
        );
    }

    #[test]
    fn dense_two_column_table_keeps_row_order() {
        const LEFT_X: f32 = 60.0;
        const RIGHT_X: f32 = 320.0;
        let left_body = [
            "The committee reviewed annual budget totals",
            "and approved new funding for the coming year",
            "after several rounds of careful review by",
            "senior staff members from every department",
            "who evaluated priorities across the whole",
            "organization before reaching a final decision",
            "that reflected both short and long term goals",
            "for sustainable growth across all programs",
        ];
        let right_cells = ["12.3", "45.6", "78.9", "10.1", "21.2", "33.4", "45.5", "67.8"];

        let mut spans = Vec::new();
        for (row, (left_line, right_cell)) in left_body.iter().copied().zip(right_cells.iter().copied()).enumerate() {
            let y = 816.0 - row as f32 * 14.0;
            spans.push(span_with_width(left_line, LEFT_X, y, 200.0, 11.0, 11.0));
            spans.push(span_with_width(right_cell, RIGHT_X, y, 30.0, 11.0, 11.0));
        }
        let original = spans.iter().map(|span| span.text.clone()).collect::<Vec<_>>();

        assert!(!reorder_dense_two_column_page(&mut spans, 612.0));
        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            original.iter().map(String::as_str).collect::<Vec<_>>()
        );
    }

    /// GH#1397 follow-up: a running header and footer each cross the gutter
    /// (width 497pt on a 612pt page = 81.2%, well past
    /// `FULL_WIDTH_FURNITURE_FRACTION`'s 55% threshold), which used to close
    /// the projection gap and suppress the whole-page repair. The header and
    /// footer must now be excluded from the gutter search and the column
    /// partition, and the repair must still fire: header first, then the
    /// entire left column, then the entire right column, then the footer.
    #[test]
    fn dense_two_column_prose_reorders_around_header_and_footer() {
        let mut spans = vec![span_with_width(
            "Quarterly Report - Internal Distribution Only",
            60.0,
            850.0,
            497.0,
            11.0,
            11.0,
        )];
        spans.extend(dense_two_column_spans());
        spans.push(span_with_width("Page 1 of 12", 60.0, 700.0, 497.0, 11.0, 11.0));

        assert!(reorder_dense_two_column_page(&mut spans, 612.0));

        let texts = spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>();
        assert_eq!(
            texts,
            [
                "Quarterly Report - Internal Distribution Only",
                "Funding",
                "The committee reviewed annual budget totals",
                "and approved new funding for the coming year",
                "after several rounds of careful review by",
                "senior staff members from every department",
                "who evaluated priorities across the whole",
                "organization before reaching a final decision",
                "that reflected both short and long term goals",
                "for sustainable growth across all programs",
                "References",
                "Numerous studies have examined similar",
                "programs across comparable institutions",
                "using consistent methodology and controls",
                "for measuring outcomes over multiple years",
                "researchers found consistent positive trends",
                "supporting continued investment going forward",
                "additional citations appear in the appendix",
                "for readers seeking further detail here",
                "Page 1 of 12",
            ]
        );
    }

    /// GH#1397 follow-up: a full-width heading can sit well above the two
    /// columns without being at the very top edge of the page ("mid-page"
    /// furniture) — e.g. a document title printed a few lines above where
    /// the two-column body starts. It must stay above BOTH columns in the
    /// output, exactly like a page-top running header, since the rule is
    /// purely relative to the columns' own vertical extent, not to any
    /// absolute page position.
    #[test]
    fn dense_two_column_prose_keeps_midpage_heading_above_both_columns() {
        let mut spans = vec![span_with_width(
            "Annual Committee Findings",
            60.0,
            840.0,
            497.0,
            11.0,
            11.0,
        )];
        spans.extend(dense_two_column_spans());

        assert!(reorder_dense_two_column_page(&mut spans, 612.0));

        let texts = spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>();
        assert_eq!(texts[0], "Annual Committee Findings");
        let heading_index = 0;
        let funding_index = texts.iter().position(|&text| text == "Funding").unwrap();
        let references_index = texts.iter().position(|&text| text == "References").unwrap();
        assert!(heading_index < funding_index && heading_index < references_index);
    }

    /// Build one `row_count`-row two-column band (left/right pair per row),
    /// starting at `y_start` and stepping down a line-height (14pt) per row.
    /// Text is long, ordinary prose so `classify_region` reads each side as
    /// `Prose`, and `label` keys the sentence text so tests can assert exact
    /// row identity and order.
    fn two_column_band(row_count: usize, y_start: f32, label: &str) -> Vec<TextSpan> {
        const LEFT_X: f32 = 60.0;
        const RIGHT_X: f32 = 320.0;
        let mut spans = Vec::with_capacity(row_count * 2);
        for row in 0..row_count {
            let y = y_start - row as f32 * 14.0;
            let left_text = format!("The {label} left column continues with sentence number {row} of the report");
            let right_text = format!("The {label} right column continues with sentence number {row} of the report");
            spans.push(span_with_width(&left_text, LEFT_X, y, 200.0, 11.0, 11.0));
            spans.push(span_with_width(&right_text, RIGHT_X, y, 190.0, 11.0, 11.0));
        }
        spans
    }

    /// Assert a two-band-plus-furniture page reorders each band column-major
    /// (left rows then right rows) with `furniture_text` landing strictly
    /// between the two bands. Shared by the wide-banner and narrow-rule tests
    /// below: both exercise the same band-splitting/per-band-reorder path,
    /// differing only in how the furniture line is detected as a boundary.
    fn assert_bands_reordered_around_furniture(spans: &mut [TextSpan], furniture_text: &str, rows_per_band: usize) {
        assert!(reorder_dense_two_column_page(spans, 612.0));

        let texts = spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>();
        let furniture_index = texts.iter().position(|&text| text == furniture_text).unwrap();
        assert_eq!(
            furniture_index,
            rows_per_band * 2,
            "furniture must land strictly after the whole band above it"
        );
        assert_eq!(texts.len(), rows_per_band * 4 + 1);
        for row in 0..rows_per_band {
            assert_eq!(
                texts[row],
                format!("The first left column continues with sentence number {row} of the report")
            );
            assert_eq!(
                texts[rows_per_band + row],
                format!("The first right column continues with sentence number {row} of the report")
            );
        }
        let below_start = furniture_index + 1;
        for row in 0..rows_per_band {
            assert_eq!(
                texts[below_start + row],
                format!("The second left column continues with sentence number {row} of the report")
            );
            assert_eq!(
                texts[below_start + rows_per_band + row],
                format!("The second right column continues with sentence number {row} of the report")
            );
        }
    }

    /// GH#1397 follow-up: a wide (0.66 of page width) centred banner sitting
    /// strictly inside the two columns' vertical extent must not land at the
    /// old global "after the whole left column, before the whole right
    /// column" placeholder position. Per-band segmentation must instead treat
    /// it as a boundary line splitting the page into a band above it and a
    /// band below it, each independently reordered column-major, with the
    /// banner emitted strictly between them — its true interleaved position.
    #[test]
    fn dense_two_column_prose_reorders_around_midpage_banner() {
        const ROWS_PER_BAND: usize = 7;
        const PAGE_WIDTH: f32 = 612.0;
        const BANNER_TEXT: &str = "Quarterly Report - Company Wide Distribution Banner";

        let band_above = two_column_band(ROWS_PER_BAND, 830.0, "first");
        let band_below = two_column_band(ROWS_PER_BAND, 830.0 - ROWS_PER_BAND as f32 * 14.0 - 20.0, "second");
        let banner_y = 830.0 - (ROWS_PER_BAND as f32 - 1.0) * 14.0 - 10.0;
        let banner_width = PAGE_WIDTH * 0.66;
        let banner_x = (PAGE_WIDTH - banner_width) / 2.0;

        let mut spans = band_above;
        spans.push(span_with_width(
            BANNER_TEXT,
            banner_x,
            banner_y,
            banner_width,
            11.0,
            11.0,
        ));
        spans.extend(band_below);

        assert_bands_reordered_around_furniture(&mut spans, BANNER_TEXT, ROWS_PER_BAND);
    }

    /// GH#1397 follow-up: furniture narrower than `FULL_WIDTH_FURNITURE_FRACTION`
    /// (0.55) that still crosses the gutter used to close the single whole-page
    /// gutter projection and suppress the repair entirely, exactly as if there
    /// were no gutter at all. Per-line gutter detection must not let this one
    /// line corrupt the *other* lines' evidence: the columns above and below
    /// must still be repaired, and the rule must land at its true interleaved
    /// position between them rather than nowhere.
    #[test]
    fn dense_two_column_prose_reorders_around_narrow_gutter_crossing_rule() {
        const ROWS_PER_BAND: usize = 7;
        const PAGE_WIDTH: f32 = 612.0;
        const RULE_TEXT: &str = "----------";

        let band_above = two_column_band(ROWS_PER_BAND, 830.0, "first");
        let band_below = two_column_band(ROWS_PER_BAND, 830.0 - ROWS_PER_BAND as f32 * 14.0 - 20.0, "second");
        let rule_y = 830.0 - (ROWS_PER_BAND as f32 - 1.0) * 14.0 - 10.0;
        // 0.30 of the page width: well under FULL_WIDTH_FURNITURE_FRACTION
        // (0.55), but wide enough, centred on the ~290pt gutter these columns
        // produce (left column right edge 260, right column left edge 320),
        // to straddle it on both sides.
        let rule_width = PAGE_WIDTH * 0.30;

        let mut spans = band_above;
        spans.push(span_with_width(RULE_TEXT, 200.0, rule_y, rule_width, 2.0, 2.0));
        spans.extend(band_below);

        assert_bands_reordered_around_furniture(&mut spans, RULE_TEXT, ROWS_PER_BAND);
    }

    /// Regression guard: a genuine single-column page with both wide
    /// (near-furniture-width) and narrow lines must NOT be split. All lines
    /// share the same left edge (there is only one column to begin with), so
    /// excluding the wide lines as "furniture" from the gutter search must
    /// not manufacture an artificial gap among the remaining narrow lines.
    /// Splitting a genuinely single-column page scrambles correct output,
    /// which is worse than leaving the (non-existent) repair unapplied.
    #[test]
    fn single_column_page_with_wide_and_narrow_lines_is_not_split() {
        const COLUMN_X: f32 = 60.0;
        let lines: [(&str, f32); 8] = [
            ("This is a long justified line of body text filling", 470.0),
            ("the page width almost completely from margin", 470.0),
            ("to margin, as ordinary single-column prose does", 470.0),
            ("Short line.", 90.0),
            ("Another full-width line of ordinary body text here", 470.0),
            ("Brief.", 90.0),
            ("A further wide line completing this single paragraph", 470.0),
            ("End.", 90.0),
        ];
        let mut spans = Vec::new();
        for (row, (text, width)) in lines.iter().enumerate() {
            let y = 800.0 - row as f32 * 14.0;
            spans.push(span_with_width(text, COLUMN_X, y, *width, 11.0, 11.0));
        }
        let original = spans.iter().map(|span| span.text.clone()).collect::<Vec<_>>();

        assert!(!reorder_dense_two_column_page(&mut spans, 612.0));
        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            original.iter().map(String::as_str).collect::<Vec<_>>()
        );
    }

    /// GH#1545: a two-panel statistics table beside a prose column was emitted in
    /// full-width Y order, splicing the prose apart mid-sentence and welding the
    /// table's own panels together row by row.
    ///
    /// Three gates declined in cascade, and the issue's own proposed fix (have
    /// `detect_split_x` return the *set* of gutters) addressed none of them:
    ///
    /// 1. `detect_split_x` returned 232.99 — inside the table. The table (8.05pt
    ///    leading) and the prose (10.45pt) are not baseline-aligned, so
    ///    `group_into_lines` never groups them into a shared line and per-line gutter
    ///    evidence only ever saw the table's internal gaps. A page-wide whitespace
    ///    corridor does see the real boundary; `page_whitespace_corridors` supplies it.
    /// 2. Even forced to the ideal split the gate declined: the table side classifies
    ///    `Form`, and `reorder_band_columns` required *both* sides to be
    ///    `is_reorderable_column()`. That was the binding constraint.
    ///    `MAX_CROSS_GUTTER_ROW_PAIRING_FRACTION` now admits one non-prose side when
    ///    the two do not pair up row for row.
    /// 3. With the repair declining, `apply_xy_cut_if_column_aware` also declined —
    ///    `select_reading_order` needs prose lines on both sides — so the page fell
    ///    through to plain top-to-bottom order, which is the reported defect.
    ///
    /// Geometry is transcribed verbatim, one span per `<word>`, from `pdftotext
    /// -bbox`'s output on page 1 of the GH#1545 repro PDF: `x = xMin`,
    /// `width = xMax - xMin`, `height = yMax - yMin`. `pdftotext -bbox` is
    /// top-left-origin/y-down; this crate's `Rect`/`TextSpan::bbox` is
    /// bottom-left-origin/y-up (see `group_into_lines`'s descending-`y`
    /// top-to-bottom sort, and every `y` in the `dense_two_column_*` fixtures
    /// above decreasing top-to-bottom on the page), so `y = PAGE_HEIGHT - yMin`.
    /// This transcription is the only surviving copy of that geometry. No position
    /// is invented and no word's measured gap is pre-merged into a wider span — if
    /// `xberg_native_pdf`'s own glyph-to-span coalescing fuses adjacent words in
    /// production, that fusion happens upstream of this function's input.
    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one span literal per pdftotext word, transcribed verbatim for fidelity"
    )]
    fn gh1545_table_beside_prose_emits_table_then_prose() {
        const PAGE_WIDTH: f32 = 595.0;
        // The table's rightmost measured value column ends at x=285.622; the
        // prose column starts at x=303.600. 295.0 is the midpoint of that
        // real gutter — the split a correct "table beside prose" reorder
        // would have to use, independent of whatever `detect_split_x` finds. ~keep
        const IDEAL_TABLE_PROSE_SPLIT_X: f32 = 295.0;
        // The measured edges either side of that gutter, and the left edge of the
        // table's second label/value panel. ~keep
        const TABLE_RIGHT_EDGE_X: f32 = 285.622;
        const PROSE_LEFT_EDGE_X: f32 = 303.600;
        const PANEL_B_LEFT_EDGE_X: f32 = 164.803;
        // The title row sits above every table row and runs across both panels. ~keep
        const TITLE_ROW_Y: f32 = 730.0;

        #[rustfmt::skip]
        let mut spans = vec![
            span_with_width("Table", 47.700, 735.026, 17.507, 6.475, 6.475),
            span_with_width("1", 67.153, 735.026, 3.892, 6.475, 6.475),
            span_with_width("Sample", 72.991, 735.026, 23.730, 6.475, 6.475),
            span_with_width("characteristics", 98.667, 735.026, 44.730, 6.475, 6.475),
            span_with_width("of", 145.343, 735.026, 5.838, 6.475, 6.475),
            span_with_width("the", 153.127, 735.026, 9.730, 6.475, 6.475),
            span_with_width("Northfield", 164.803, 735.026, 29.953, 6.475, 6.475),
            span_with_width("and", 196.702, 735.026, 11.676, 6.475, 6.475),
            span_with_width("Eastgate", 210.324, 735.026, 27.629, 6.475, 6.475),
            span_with_width("cohorts.", 239.899, 735.026, 24.899, 6.475, 6.475),
            span_with_width("Sex", 47.700, 718.926, 12.061, 6.475, 6.475),
            span_with_width("Female", 47.700, 710.876, 23.338, 6.475, 6.475),
            span_with_width("Male", 47.700, 702.826, 15.169, 6.475, 6.475),
            span_with_width("Age", 47.700, 694.776, 12.453, 6.475, 6.475),
            span_with_width("18-24", 62.099, 694.776, 17.899, 6.475, 6.475),
            span_with_width("25-34", 47.700, 686.726, 17.899, 6.475, 6.475),
            span_with_width("35-44", 47.700, 678.676, 17.899, 6.475, 6.475),
            span_with_width("45-54", 47.700, 670.626, 17.899, 6.475, 6.475),
            span_with_width("55-64", 47.700, 662.576, 17.899, 6.475, 6.475),
            span_with_width("Ethnicity", 47.700, 654.526, 26.453, 6.475, 6.475),
            span_with_width("Group", 47.700, 646.476, 19.453, 6.475, 6.475),
            span_with_width("one", 69.099, 646.476, 11.676, 6.475, 6.475),
            span_with_width("Group", 47.700, 638.426, 19.453, 6.475, 6.475),
            span_with_width("two", 69.099, 638.426, 10.892, 6.475, 6.475),
            span_with_width("Group", 47.700, 630.376, 19.453, 6.475, 6.475),
            span_with_width("three", 69.099, 630.376, 15.953, 6.475, 6.475),
            span_with_width("Group", 47.700, 622.326, 19.453, 6.475, 6.475),
            span_with_width("four", 69.099, 622.326, 12.061, 6.475, 6.475),
            span_with_width("Group", 47.700, 614.276, 19.453, 6.475, 6.475),
            span_with_width("five", 69.099, 614.276, 10.892, 6.475, 6.475),
            span_with_width("Living", 47.700, 606.226, 18.284, 6.475, 6.475),
            span_with_width("location", 67.930, 606.226, 24.122, 6.475, 6.475),
            span_with_width("City", 47.700, 598.176, 12.054, 6.475, 6.475),
            span_with_width("Suburb", 47.700, 590.126, 22.568, 6.475, 6.475),
            span_with_width("Town", 47.700, 582.076, 17.115, 6.475, 6.475),
            span_with_width("Rural", 47.700, 574.026, 16.723, 6.475, 6.475),
            span_with_width("Highest", 47.700, 565.976, 23.730, 6.475, 6.475),
            span_with_width("education", 73.376, 565.976, 30.352, 6.475, 6.475),
            span_with_width("No", 47.700, 557.926, 8.946, 6.475, 6.475),
            span_with_width("qualifications", 58.592, 557.926, 40.460, 6.475, 6.475),
            span_with_width("Secondary", 47.700, 549.876, 33.460, 6.475, 6.475),
            span_with_width("school", 83.106, 549.876, 20.230, 6.475, 6.475),
            span_with_width("Diploma", 47.700, 541.826, 25.669, 6.475, 6.475),
            span_with_width("Undergraduate", 47.700, 533.776, 46.690, 6.475, 6.475),
            span_with_width("degree", 96.336, 533.776, 21.791, 6.475, 6.475),
            span_with_width("Postgraduate", 47.700, 525.726, 41.636, 6.475, 6.475),
            span_with_width("degree", 91.282, 525.726, 21.791, 6.475, 6.475),
            span_with_width("Employment", 47.700, 517.676, 38.899, 6.475, 6.475),
            span_with_width("status", 88.545, 517.676, 18.676, 6.475, 6.475),
            span_with_width("Full-time", 47.700, 509.626, 26.831, 6.475, 6.475),
            span_with_width("employed", 76.477, 509.626, 30.345, 6.475, 6.475),
            span_with_width("Part-time", 47.700, 501.576, 28.392, 6.475, 6.475),
            span_with_width("employed", 78.038, 501.576, 30.345, 6.475, 6.475),
            span_with_width("Retired", 47.700, 493.526, 22.561, 6.475, 6.475),
            span_with_width("Not", 47.700, 485.476, 10.892, 6.475, 6.475),
            span_with_width("employed", 60.538, 485.476, 30.345, 6.475, 6.475),
            span_with_width("%", 148.100, 718.926, 6.223, 6.475, 6.475),
            span_with_width("Sex", 165.000, 718.926, 12.061, 6.475, 6.475),
            span_with_width("51.5", 148.100, 710.876, 13.622, 6.475, 6.475),
            span_with_width("Female", 165.000, 710.876, 23.338, 6.475, 6.475),
            span_with_width("48.2", 148.100, 702.826, 13.622, 6.475, 6.475),
            span_with_width("Male", 165.000, 702.826, 15.169, 6.475, 6.475),
            span_with_width("11.1", 148.100, 694.776, 13.622, 6.475, 6.475),
            span_with_width("Age", 165.000, 694.776, 12.453, 6.475, 6.475),
            span_with_width("18-24", 179.399, 694.776, 17.899, 6.475, 6.475),
            span_with_width("19.2", 148.100, 686.726, 13.622, 6.475, 6.475),
            span_with_width("25-34", 165.000, 686.726, 17.899, 6.475, 6.475),
            span_with_width("20.6", 148.100, 678.676, 13.622, 6.475, 6.475),
            span_with_width("35-44", 165.000, 678.676, 17.899, 6.475, 6.475),
            span_with_width("15.9", 148.100, 670.626, 13.622, 6.475, 6.475),
            span_with_width("45-54", 165.000, 670.626, 17.899, 6.475, 6.475),
            span_with_width("21.0", 148.100, 662.576, 13.622, 6.475, 6.475),
            span_with_width("55-64", 165.000, 662.576, 17.899, 6.475, 6.475),
            span_with_width("%", 272.000, 718.926, 6.223, 6.475, 6.475),
            span_with_width("51.7", 272.000, 710.876, 13.622, 6.475, 6.475),
            span_with_width("48.3", 272.000, 702.826, 13.622, 6.475, 6.475),
            span_with_width("12.1", 272.000, 694.776, 13.622, 6.475, 6.475),
            span_with_width("18.8", 272.000, 686.726, 13.622, 6.475, 6.475),
            span_with_width("17.4", 272.000, 678.676, 13.622, 6.475, 6.475),
            span_with_width("20.2", 272.000, 670.626, 13.622, 6.475, 6.475),
            span_with_width("17.2", 272.000, 662.576, 13.622, 6.475, 6.475),
            span_with_width("17.3", 148.100, 646.476, 13.622, 6.475, 6.475),
            span_with_width("Group", 165.000, 646.476, 19.453, 6.475, 6.475),
            span_with_width("one", 186.399, 646.476, 11.676, 6.475, 6.475),
            span_with_width("1.9", 148.100, 638.426, 9.730, 6.475, 6.475),
            span_with_width("Group", 165.000, 638.426, 19.453, 6.475, 6.475),
            span_with_width("two", 186.399, 638.426, 10.892, 6.475, 6.475),
            span_with_width("0.3", 148.100, 630.376, 9.730, 6.475, 6.475),
            span_with_width("Group", 165.000, 630.376, 19.453, 6.475, 6.475),
            span_with_width("three", 186.399, 630.376, 15.953, 6.475, 6.475),
            span_with_width("0.4", 148.100, 622.326, 9.730, 6.475, 6.475),
            span_with_width("Group", 165.000, 622.326, 19.453, 6.475, 6.475),
            span_with_width("four", 186.399, 622.326, 12.061, 6.475, 6.475),
            span_with_width("3.2", 148.100, 614.276, 9.730, 6.475, 6.475),
            span_with_width("Group", 165.000, 614.276, 19.453, 6.475, 6.475),
            span_with_width("five", 186.399, 614.276, 10.892, 6.475, 6.475),
            span_with_width("14.2", 272.000, 646.476, 13.622, 6.475, 6.475),
            span_with_width("2.4", 272.000, 638.426, 9.730, 6.475, 6.475),
            span_with_width("0.6", 272.000, 630.376, 9.730, 6.475, 6.475),
            span_with_width("1.1", 272.000, 622.326, 9.730, 6.475, 6.475),
            span_with_width("2.8", 272.000, 614.276, 9.730, 6.475, 6.475),
            span_with_width("24.5", 148.100, 598.176, 13.622, 6.475, 6.475),
            span_with_width("City", 165.000, 598.176, 12.054, 6.475, 6.475),
            span_with_width("18.1", 148.100, 590.126, 13.622, 6.475, 6.475),
            span_with_width("Suburb", 165.000, 590.126, 22.568, 6.475, 6.475),
            span_with_width("26.8", 148.100, 582.076, 13.622, 6.475, 6.475),
            span_with_width("Town", 165.000, 582.076, 17.115, 6.475, 6.475),
            span_with_width("28.8", 148.100, 574.026, 13.622, 6.475, 6.475),
            span_with_width("Rural", 165.000, 574.026, 16.723, 6.475, 6.475),
            span_with_width("26.1", 272.000, 598.176, 13.622, 6.475, 6.475),
            span_with_width("19.4", 272.000, 590.126, 13.622, 6.475, 6.475),
            span_with_width("24.9", 272.000, 582.076, 13.622, 6.475, 6.475),
            span_with_width("29.6", 272.000, 574.026, 13.622, 6.475, 6.475),
            span_with_width("1.2", 148.100, 557.926, 9.730, 6.475, 6.475),
            span_with_width("No", 165.000, 557.926, 8.946, 6.475, 6.475),
            span_with_width("qualifications", 175.892, 557.926, 40.460, 6.475, 6.475),
            span_with_width("6.4", 148.100, 549.876, 9.730, 6.475, 6.475),
            span_with_width("Secondary", 165.000, 549.876, 33.460, 6.475, 6.475),
            span_with_width("school", 200.406, 549.876, 20.230, 6.475, 6.475),
            span_with_width("22.5", 148.100, 541.826, 13.622, 6.475, 6.475),
            span_with_width("Diploma", 165.000, 541.826, 25.669, 6.475, 6.475),
            span_with_width("19.8", 148.100, 533.776, 13.622, 6.475, 6.475),
            span_with_width("Undergraduate", 165.000, 533.776, 46.690, 6.475, 6.475),
            span_with_width("degree", 213.636, 533.776, 21.791, 6.475, 6.475),
            span_with_width("27.9", 148.100, 525.726, 13.622, 6.475, 6.475),
            span_with_width("Postgraduate", 165.000, 525.726, 41.636, 6.475, 6.475),
            span_with_width("degree", 208.582, 525.726, 21.791, 6.475, 6.475),
            span_with_width("1.8", 272.000, 557.926, 9.730, 6.475, 6.475),
            span_with_width("7.1", 272.000, 549.876, 9.730, 6.475, 6.475),
            span_with_width("21.8", 272.000, 541.826, 13.622, 6.475, 6.475),
            span_with_width("20.4", 272.000, 533.776, 13.622, 6.475, 6.475),
            span_with_width("26.3", 272.000, 525.726, 13.622, 6.475, 6.475),
            span_with_width("43.3", 148.100, 509.626, 13.622, 6.475, 6.475),
            span_with_width("Full-time", 165.000, 509.626, 26.831, 6.475, 6.475),
            span_with_width("employed", 193.777, 509.626, 30.345, 6.475, 6.475),
            span_with_width("15.7", 148.100, 501.576, 13.622, 6.475, 6.475),
            span_with_width("Part-time", 165.000, 501.576, 28.392, 6.475, 6.475),
            span_with_width("employed", 195.338, 501.576, 30.345, 6.475, 6.475),
            span_with_width("15.0", 148.100, 493.526, 13.622, 6.475, 6.475),
            span_with_width("Retired", 165.000, 493.526, 22.561, 6.475, 6.475),
            span_with_width("8.4", 148.100, 485.476, 9.730, 6.475, 6.475),
            span_with_width("Not", 165.000, 485.476, 10.892, 6.475, 6.475),
            span_with_width("employed", 177.838, 485.476, 30.345, 6.475, 6.475),
            span_with_width("41.9", 272.000, 509.626, 13.622, 6.475, 6.475),
            span_with_width("16.4", 272.000, 501.576, 13.622, 6.475, 6.475),
            span_with_width("14.6", 272.000, 493.526, 13.622, 6.475, 6.475),
            span_with_width("9.2", 272.000, 485.476, 9.730, 6.475, 6.475),
            span_with_width("Participants", 303.600, 736.103, 44.404, 7.862, 7.862),
            span_with_width("in", 350.367, 736.103, 6.613, 7.862, 7.862),
            span_with_width("the", 359.343, 736.103, 11.815, 7.862, 7.862),
            span_with_width("Northfield", 373.521, 736.103, 36.371, 7.862, 7.862),
            span_with_width("cohort", 412.255, 736.103, 23.622, 7.862, 7.862),
            span_with_width("who", 438.240, 736.103, 15.589, 7.862, 7.862),
            span_with_width("reported", 456.192, 736.103, 31.654, 7.862, 7.862),
            span_with_width("low", 490.209, 736.103, 12.750, 7.862, 7.862),
            span_with_width("confidence", 303.600, 725.653, 41.106, 7.863, 7.863),
            span_with_width("in", 347.069, 725.653, 6.613, 7.863, 7.863),
            span_with_width("the", 356.045, 725.653, 11.815, 7.863, 7.863),
            span_with_width("programme", 370.223, 725.653, 43.452, 7.863, 7.863),
            span_with_width("were,", 416.038, 725.653, 20.782, 7.863, 7.863),
            span_with_width("compared", 439.183, 725.653, 37.791, 7.863, 7.863),
            span_with_width("with", 479.337, 725.653, 15.113, 7.863, 7.863),
            span_with_width("those", 496.813, 725.653, 20.791, 7.863, 7.863),
            span_with_width("who", 303.600, 715.203, 15.589, 7.863, 7.863),
            span_with_width("reported", 321.552, 715.203, 31.654, 7.863, 7.863),
            span_with_width("high", 355.569, 715.203, 16.065, 7.863, 7.863),
            span_with_width("confidence,", 373.997, 715.203, 43.469, 7.863, 7.863),
            span_with_width("more", 419.829, 715.203, 19.363, 7.863, 7.863),
            span_with_width("likely", 441.555, 715.203, 18.887, 7.863, 7.863),
            span_with_width("to", 462.805, 715.203, 7.089, 7.863, 7.863),
            span_with_width("be", 472.257, 715.203, 9.452, 7.863, 7.863),
            span_with_width("aged", 484.072, 715.203, 18.904, 7.863, 7.863),
            span_with_width("35", 505.339, 715.203, 9.452, 7.863, 7.863),
            span_with_width("to", 303.600, 704.753, 7.089, 7.862, 7.862),
            span_with_width("44", 313.052, 704.753, 9.452, 7.862, 7.862),
            span_with_width("years,", 324.867, 704.753, 23.145, 7.862, 7.862),
            span_with_width("to", 350.375, 704.753, 7.089, 7.862, 7.862),
            span_with_width("live", 359.827, 704.753, 12.750, 7.862, 7.862),
            span_with_width("in", 374.940, 704.753, 6.613, 7.862, 7.862),
            span_with_width("a", 383.916, 704.753, 4.726, 7.862, 7.862),
            span_with_width("city,", 391.005, 704.753, 15.113, 7.862, 7.862),
            span_with_width("to", 408.481, 704.753, 7.089, 7.862, 7.862),
            span_with_width("hold", 417.933, 704.753, 16.065, 7.862, 7.862),
            span_with_width("no", 436.361, 704.753, 9.452, 7.862, 7.862),
            span_with_width("post-school", 448.176, 704.753, 43.461, 7.862, 7.862),
            span_with_width("qualification,", 303.600, 694.303, 47.243, 7.863, 7.863),
            span_with_width("and", 353.206, 694.303, 14.178, 7.863, 7.863),
            span_with_width("to", 369.747, 694.303, 7.089, 7.863, 7.863),
            span_with_width("report", 379.199, 694.303, 22.202, 7.863, 7.863),
            span_with_width("that", 403.764, 694.303, 14.178, 7.863, 7.863),
            span_with_width("they", 420.305, 694.303, 16.065, 7.863, 7.863),
            span_with_width("had", 438.733, 694.303, 14.178, 7.863, 7.863),
            span_with_width("not", 455.274, 694.303, 11.815, 7.863, 7.863),
            span_with_width("voted", 469.452, 694.303, 20.791, 7.863, 7.863),
            span_with_width("at", 492.606, 694.303, 7.089, 7.863, 7.863),
            span_with_width("the", 303.600, 683.853, 11.815, 7.863, 7.863),
            span_with_width("most", 317.778, 683.853, 18.419, 7.863, 7.863),
            span_with_width("recent", 338.560, 683.853, 23.622, 7.863, 7.863),
            span_with_width("municipal", 364.545, 683.853, 35.895, 7.863, 7.863),
            span_with_width("election.", 402.803, 683.853, 31.654, 7.863, 7.863),
            span_with_width("The", 436.820, 683.853, 14.646, 7.863, 7.863),
            span_with_width("same", 453.829, 683.853, 20.782, 7.863, 7.863),
            span_with_width("pattern", 476.974, 683.853, 26.461, 7.863, 7.863),
            span_with_width("was", 505.798, 683.853, 15.113, 7.863, 7.863),
            span_with_width("not", 303.600, 673.403, 11.815, 7.862, 7.862),
            span_with_width("observed", 317.778, 673.403, 34.960, 7.862, 7.862),
            span_with_width("in", 355.101, 673.403, 6.613, 7.862, 7.862),
            span_with_width("the", 364.077, 673.403, 11.815, 7.862, 7.862),
            span_with_width("Eastgate", 378.255, 673.403, 33.550, 7.862, 7.862),
            span_with_width("cohort,", 414.168, 673.403, 25.984, 7.862, 7.862),
            span_with_width("where", 442.515, 673.403, 23.146, 7.862, 7.862),
            span_with_width("the", 468.024, 673.403, 11.815, 7.862, 7.862),
            span_with_width("strongest", 482.202, 673.403, 34.961, 7.862, 7.862),
            span_with_width("association", 303.600, 662.953, 42.517, 7.863, 7.863),
            span_with_width("was", 348.480, 662.953, 15.113, 7.863, 7.863),
            span_with_width("with", 365.956, 662.953, 15.113, 7.863, 7.863),
            span_with_width("employment", 383.432, 662.953, 46.291, 7.863, 7.863),
            span_with_width("status", 432.086, 662.953, 22.678, 7.863, 7.863),
            span_with_width("rather", 457.127, 662.953, 22.202, 7.863, 7.863),
            span_with_width("than", 481.692, 662.953, 16.541, 7.863, 7.863),
            span_with_width("with", 500.596, 662.953, 15.113, 7.863, 7.863),
            span_with_width("age", 303.600, 652.503, 14.178, 7.862, 7.862),
            span_with_width("or", 320.141, 652.503, 7.556, 7.862, 7.862),
            span_with_width("education.", 330.060, 652.503, 39.219, 7.862, 7.862),
            span_with_width("Full", 371.642, 652.503, 13.694, 7.862, 7.862),
            span_with_width("model", 387.699, 652.503, 23.145, 7.862, 7.862),
            span_with_width("output", 413.207, 652.503, 23.630, 7.862, 7.862),
            span_with_width("for", 439.200, 652.503, 9.920, 7.862, 7.862),
            span_with_width("both", 451.483, 652.503, 16.541, 7.862, 7.862),
            span_with_width("cohorts", 470.387, 652.503, 27.872, 7.862, 7.862),
            span_with_width("is", 500.622, 652.503, 6.137, 7.862, 7.862),
            span_with_width("given", 303.600, 642.053, 20.315, 7.863, 7.863),
            span_with_width("in", 326.278, 642.053, 6.613, 7.863, 7.863),
            span_with_width("Tables", 335.254, 642.053, 25.508, 7.863, 7.863),
            span_with_width("2", 363.125, 642.053, 4.726, 7.863, 7.863),
            span_with_width("and", 370.214, 642.053, 14.178, 7.863, 7.863),
            span_with_width("3.", 386.755, 642.053, 7.089, 7.863, 7.863),
            span_with_width("Percentages", 396.207, 642.053, 47.719, 7.863, 7.863),
            span_with_width("in", 446.289, 642.053, 6.613, 7.863, 7.863),
            span_with_width("Table", 455.265, 642.053, 21.259, 7.863, 7.863),
            span_with_width("1", 478.887, 642.053, 4.726, 7.863, 7.863),
            span_with_width("are", 485.976, 642.053, 12.283, 7.863, 7.863),
            span_with_width("column", 303.600, 631.603, 27.395, 7.863, 7.863),
            span_with_width("percentages", 333.358, 631.603, 46.776, 7.863, 7.863),
            span_with_width("and", 382.497, 631.603, 14.178, 7.863, 7.863),
            span_with_width("may", 399.038, 631.603, 16.056, 7.863, 7.863),
            span_with_width("not", 417.457, 631.603, 11.815, 7.863, 7.863),
            span_with_width("sum", 431.635, 631.603, 16.057, 7.863, 7.863),
            span_with_width("to", 450.055, 631.603, 7.089, 7.863, 7.863),
            span_with_width("one", 459.507, 631.603, 14.178, 7.863, 7.863),
            span_with_width("hundred", 476.048, 631.603, 31.187, 7.863, 7.863),
            span_with_width("where", 509.598, 631.603, 23.146, 7.863, 7.863),
            span_with_width("a", 303.600, 621.153, 4.726, 7.862, 7.862),
            span_with_width("category", 310.689, 621.153, 32.597, 7.862, 7.862),
            span_with_width("was", 345.649, 621.153, 15.113, 7.862, 7.862),
            span_with_width("left", 363.125, 621.153, 11.339, 7.862, 7.862),
            span_with_width("blank", 376.827, 621.153, 20.315, 7.862, 7.862),
            span_with_width("by", 399.505, 621.153, 8.976, 7.862, 7.862),
            span_with_width("the", 410.844, 621.153, 11.815, 7.862, 7.862),
            span_with_width("respondent.", 425.022, 621.153, 44.889, 7.862, 7.862),
            span_with_width("Weighting", 472.274, 621.153, 37.791, 7.862, 7.862),
            span_with_width("was", 303.600, 610.703, 15.113, 7.863, 7.863),
            span_with_width("applied", 321.076, 610.703, 27.404, 7.863, 7.863),
            span_with_width("to", 350.843, 610.703, 7.089, 7.863, 7.863),
            span_with_width("the", 360.295, 610.703, 11.815, 7.863, 7.863),
            span_with_width("age", 374.473, 610.703, 14.178, 7.863, 7.863),
            span_with_width("and", 391.014, 610.703, 14.178, 7.863, 7.863),
            span_with_width("sex", 407.555, 610.703, 13.226, 7.863, 7.863),
            span_with_width("margins", 423.144, 610.703, 30.226, 7.863, 7.863),
            span_with_width("of", 455.733, 610.703, 7.089, 7.863, 7.863),
            span_with_width("each", 465.185, 610.703, 18.428, 7.863, 7.863),
            span_with_width("cohort", 485.976, 610.703, 23.622, 7.863, 7.863),
            span_with_width("separately,", 303.600, 600.253, 41.573, 7.862, 7.862),
            span_with_width("using", 347.536, 600.253, 20.315, 7.862, 7.862),
            span_with_width("the", 370.214, 600.253, 11.815, 7.862, 7.862),
            span_with_width("published", 384.392, 600.253, 36.380, 7.862, 7.862),
            span_with_width("municipal", 423.135, 600.253, 35.896, 7.862, 7.862),
            span_with_width("register", 461.394, 600.253, 28.339, 7.862, 7.862),
            span_with_width("as", 492.096, 600.253, 8.976, 7.862, 7.862),
            span_with_width("the", 303.600, 589.803, 11.815, 7.863, 7.863),
            span_with_width("reference", 317.778, 589.803, 35.904, 7.863, 7.863),
            span_with_width("distribution", 356.045, 589.803, 41.097, 7.863, 7.863),
            span_with_width("for", 399.505, 589.803, 9.920, 7.863, 7.863),
            span_with_width("both.", 411.788, 589.803, 18.904, 7.863, 7.863),
            span_with_width("Respondents", 433.055, 589.803, 50.082, 7.863, 7.863),
            span_with_width("who", 485.500, 589.803, 15.589, 7.863, 7.863),
            span_with_width("completed", 303.600, 579.353, 39.210, 7.863, 7.863),
            span_with_width("fewer", 345.173, 579.353, 20.783, 7.863, 7.863),
            span_with_width("than", 368.319, 579.353, 16.541, 7.863, 7.863),
            span_with_width("half", 387.223, 579.353, 13.702, 7.863, 7.863),
            span_with_width("of", 403.288, 579.353, 7.089, 7.863, 7.863),
            span_with_width("the", 412.740, 579.353, 11.815, 7.863, 7.863),
            span_with_width("items", 426.918, 579.353, 20.306, 7.863, 7.863),
            span_with_width("were", 449.587, 579.353, 18.420, 7.863, 7.863),
            span_with_width("excluded", 470.370, 579.353, 34.017, 7.863, 7.863),
            span_with_width("before", 303.600, 568.903, 24.097, 7.863, 7.863),
            span_with_width("weighting,", 330.060, 568.903, 38.267, 7.863, 7.863),
            span_with_width("which", 370.690, 568.903, 21.726, 7.863, 7.863),
            span_with_width("removed", 394.779, 568.903, 33.065, 7.863, 7.863),
            span_with_width("a", 430.207, 568.903, 4.726, 7.863, 7.863),
            span_with_width("small", 437.296, 568.903, 19.831, 7.863, 7.863),
            span_with_width("number", 459.490, 568.903, 28.815, 7.863, 7.863),
            span_with_width("of", 490.668, 568.903, 7.089, 7.863, 7.863),
            span_with_width("cases", 500.120, 568.903, 22.202, 7.863, 7.863),
            span_with_width("from", 303.600, 558.453, 17.000, 7.862, 7.862),
            span_with_width("each", 322.963, 558.453, 18.428, 7.862, 7.862),
            span_with_width("cohort", 343.754, 558.453, 23.621, 7.862, 7.862),
            span_with_width("and", 369.738, 558.453, 14.178, 7.862, 7.862),
            span_with_width("did", 386.279, 558.453, 11.339, 7.862, 7.862),
            span_with_width("not", 399.981, 558.453, 11.815, 7.862, 7.862),
            span_with_width("change", 414.159, 558.453, 27.880, 7.862, 7.862),
            span_with_width("the", 444.402, 558.453, 11.815, 7.862, 7.862),
            span_with_width("direction", 458.580, 558.453, 32.122, 7.862, 7.862),
            span_with_width("of", 493.065, 558.453, 7.089, 7.862, 7.862),
            span_with_width("any", 502.517, 558.453, 13.702, 7.862, 7.862),
            span_with_width("reported", 303.600, 548.003, 31.654, 7.863, 7.863),
            span_with_width("association.", 337.617, 548.003, 44.880, 7.863, 7.863),
            span_with_width("The", 384.860, 548.003, 14.645, 7.863, 7.863),
            span_with_width("analysis", 401.868, 548.003, 30.702, 7.863, 7.863),
            span_with_width("was", 434.933, 548.003, 15.113, 7.863, 7.863),
            span_with_width("pre-registered.", 452.409, 548.003, 55.267, 7.863, 7.863),
        ];

        let prose_order_before: Vec<String> = spans
            .iter()
            .filter(|span| span.bbox.x >= IDEAL_TABLE_PROSE_SPLIT_X)
            .map(|span| span.text.clone())
            .collect();

        let order = spans_sorted_top_to_bottom(&spans);
        let lines = group_into_lines(&spans, &order);
        let detected = detect_split_x(&spans, &lines, PAGE_WIDTH).expect("a split is detectable");
        let snapped = snap_split_left_of_hanging_labels(&spans, &lines, PAGE_WIDTH, detected);
        assert!(
            spans
                .iter()
                .any(|span| span.bbox.left() < snapped && span.bbox.right() > snapped),
            "the per-line median is expected to still cut a word here ({snapped}); if it no longer \
             does, this fixture has stopped exercising the redirect"
        );
        let split_x = redirect_split_out_of_content(&spans, &lines, PAGE_WIDTH, snapped);
        assert!(
            split_x > TABLE_RIGHT_EDGE_X && split_x < PROSE_LEFT_EDGE_X,
            "split must land in the real table/prose gutter, got {split_x}"
        );

        assert!(
            reorder_dense_two_column_page(&mut spans, PAGE_WIDTH),
            "a table beside a prose column must be reordered, not left in full-width Y order"
        );

        let table_spans = spans
            .iter()
            .filter(|span| span.bbox.x < IDEAL_TABLE_PROSE_SPLIT_X)
            .count();
        let first_prose = spans
            .iter()
            .position(|span| span.bbox.x >= IDEAL_TABLE_PROSE_SPLIT_X)
            .expect("the prose column survives the reorder");
        assert_eq!(
            first_prose, table_spans,
            "every table span must be emitted before every prose span"
        );

        let prose_order_after: Vec<&str> = spans
            .iter()
            .filter(|span| span.bbox.x >= IDEAL_TABLE_PROSE_SPLIT_X)
            .map(|span| span.text.as_str())
            .collect();
        assert_eq!(
            prose_order_after, prose_order_before,
            "the prose column's own reading order must be untouched"
        );

        let prose = prose_order_after.join(" ");
        assert!(
            prose.contains("more likely to be aged 35 to 44 years"),
            "the sentence must not be spliced by table rows, got: {prose}"
        );

        // Panel-major emission: panel A is the label/value pair rooted at x=47.700
        // and x=148.100, panel B the pair at x=164.803 and x=272.000. Every span of
        // the first must precede every span of the second, so a row no longer welds
        // "51.5" onto the next panel's "Female". The title is measured out: it runs
        // across both panels as ordinary prose and is emitted ahead of them as a
        // non-grid row, so it legitimately holds spans on both sides. ~keep
        let panel_of_row = |span: &xberg_native_pdf::layout::TextSpan| {
            (span.bbox.y < TITLE_ROW_Y && span.bbox.x < IDEAL_TABLE_PROSE_SPLIT_X)
                .then(|| usize::from(span.bbox.x >= PANEL_B_LEFT_EDGE_X))
        };
        let last_panel_a = spans.iter().rposition(|span| panel_of_row(span) == Some(0));
        let first_panel_b = spans.iter().position(|span| panel_of_row(span) == Some(1));
        let (Some(last_panel_a), Some(first_panel_b)) = (last_panel_a, first_panel_b) else {
            panic!("both table panels must survive the reorder");
        };
        assert!(
            last_panel_a < first_panel_b,
            "panel A must be emitted whole before panel B, but panel A's last span sits at \
             {last_panel_a} and panel B's first at {first_panel_b}"
        );
    }

    // GH#1603: on a two-column page with hanging clause numbers, a corridor formed
    // by the number-to-text indent is present on EVERY line, while the true gutter
    // between columns can be closed by a single justified line whose text pokes past
    // it. `page_whitespace_corridors` then offers only the indent to `max_by`, and
    // `redirect_split_out_of_content` relocates the split from the real gutter into
    // the middle of a column -- separating every clause number from its own text.
    const GH1603_PAGE_WIDTH: f32 = 595.32;
    // The true gutter's per-line median, and the indent it must not be replaced by.
    const GH1603_TRUE_GUTTER_SPLIT_X: f32 = 293.92;
    const GH1603_LEFT_INDENT_MID_X: f32 = 63.84;

    /// A4-width, two-column, hanging-clause-number page whose real gutter (~10pt,
    /// between x=283.84/295.84 and x=304.0) is narrower than `min_gutter` (11.9pt),
    /// while the number-to-text indent on each side (~20pt / ~15pt) comfortably
    /// clears it. Row 3's left clause line is deliberately long enough (222pt) to
    /// straddle the true gutter's per-line median, forcing `redirect_split_out_of_content`
    /// to fire; every other row is a plain-width control so the true gutter still wins
    /// `detect_split_x`'s per-line vote.
    ///
    /// Left and right columns are baseline-offset by 1.32pt (rows 4-11) so their lines
    /// never group across the gutter -- the same structure the issue's real PDF has --
    /// except for rows 0-3, which stay aligned so their combined line's widest gap is
    /// the true gutter itself, giving `detect_split_x` the votes it needs to find it.
    fn gh1603_narrow_gutter_hanging_number_spans() -> Vec<TextSpan> {
        let mut spans = Vec::new();
        // Rows 0-3: left and right columns aligned on the same baseline. Row 3 is the
        // outlier whose left text (73.84 + 222.0 = 295.84) pokes past the true gutter.
        for row in 0..4 {
            let y = 900.0 - row as f32 * 14.0;
            let left_text_width = if row == 3 { 222.0 } else { 210.0 };
            spans.push(span_with_width(&format!("{row}.1"), 36.0, y, 17.84, 11.0, 11.0));
            spans.push(span_with_width(
                &format!("The left clause line {row} continues with ordinary agreement terms"),
                73.84,
                y,
                left_text_width,
                11.0,
                11.0,
            ));
            spans.push(span_with_width(&format!("{row}.2"), 304.0, y, 17.84, 11.0, 11.0));
            spans.push(span_with_width(
                &format!("The right clause line {row} continues with ordinary agreement terms"),
                336.84,
                y,
                220.0,
                11.0,
                11.0,
            ));
        }
        // Rows 4-7: left has its own hanging number; right is a plain continuation
        // line (no number) on an offset baseline.
        for row in 4..8 {
            let y = 900.0 - row as f32 * 14.0;
            spans.push(span_with_width(&format!("{row}.1"), 36.0, y, 17.84, 11.0, 11.0));
            spans.push(span_with_width(
                &format!("The left clause line {row} continues with ordinary agreement terms"),
                73.84,
                y,
                210.0,
                11.0,
                11.0,
            ));
            spans.push(span_with_width(
                &format!("The right continuation line {row} follows the previous clause"),
                336.84,
                y - 1.32,
                220.0,
                11.0,
                11.0,
            ));
        }
        // Rows 8-11: mirror image -- left is a plain continuation line, right has its
        // own hanging number.
        for row in 8..12 {
            let y = 900.0 - row as f32 * 14.0;
            spans.push(span_with_width(
                &format!("The left continuation line {row} follows the previous clause"),
                73.84,
                y,
                210.0,
                11.0,
                11.0,
            ));
            spans.push(span_with_width(&format!("{row}.2"), 304.0, y - 1.32, 17.84, 11.0, 11.0));
            spans.push(span_with_width(
                &format!("The right clause line {row} continues with ordinary agreement terms"),
                336.84,
                y - 1.32,
                220.0,
                11.0,
                11.0,
            ));
        }
        spans
    }

    /// GH#1603: `redirect_split_out_of_content` must not relocate the split into a
    /// hanging-number indent. `detect_split_x` correctly finds the true (narrow)
    /// gutter; only row 3's outlier line straddles it, which is enough to trigger the
    /// redirect. Before the fix, `page_whitespace_corridors` offers only the left
    /// number-to-text indent (the true gutter is closed by row 3 and falls below
    /// `min_gutter`), and `max_by` hands it back -- moving the split 230pt into the
    /// left margin, between every clause number and its own text.
    #[test]
    fn redirect_split_out_of_content_must_not_relocate_into_a_hanging_number_indent_gh1603() {
        let spans = gh1603_narrow_gutter_hanging_number_spans();
        let order = spans_sorted_top_to_bottom(&spans);
        let lines = group_into_lines(&spans, &order);

        let detected = detect_split_x(&spans, &lines, GH1603_PAGE_WIDTH).expect("hanging-number page has a gutter");
        assert!(
            (detected - GH1603_TRUE_GUTTER_SPLIT_X).abs() < 0.01,
            "detect_split_x must find the true gutter, got {detected}"
        );

        let snapped = snap_split_left_of_hanging_labels(&spans, &lines, GH1603_PAGE_WIDTH, detected);
        assert_eq!(
            snapped, detected,
            "no narrow span straddles the true gutter, so the snap must leave it alone"
        );
        assert!(
            spans
                .iter()
                .any(|span| span.bbox.left() < snapped && span.bbox.right() > snapped),
            "row 3's outlier line is expected to still straddle the split ({snapped}); if it no \
             longer does, this fixture has stopped exercising the redirect"
        );

        let redirected = redirect_split_out_of_content(&spans, &lines, GH1603_PAGE_WIDTH, snapped);
        assert!(
            (redirected - GH1603_LEFT_INDENT_MID_X).abs() > 1.0,
            "redirect must not relocate the split into the hanging-number indent at \
             {GH1603_LEFT_INDENT_MID_X}, got {redirected}"
        );
        assert_eq!(
            redirected, snapped,
            "with both hanging-number indents excluded there is no legitimate replacement \
             corridor, so the split must fall back to the true gutter, got {redirected}"
        );
    }

    /// GH#1603 end-to-end: every clause number must stay immediately followed by its
    /// own clause text after the reorder, never hoisted ahead of the whole column.
    #[test]
    fn dense_two_column_hanging_numbers_survive_narrow_gutter_redirect_gh1603() {
        let mut spans = gh1603_narrow_gutter_hanging_number_spans();
        assert!(
            reorder_dense_two_column_page(&mut spans, GH1603_PAGE_WIDTH),
            "a hanging-number two-column page must still be reordered"
        );

        let mut checked = 0;
        for (index, span) in spans.iter().enumerate() {
            let Some((row_str, side)) = span.text.split_once('.') else {
                continue;
            };
            if side != "1" && side != "2" {
                continue;
            }
            let Ok(row) = row_str.parse::<usize>() else {
                continue;
            };
            let expected_prefix = if side == "1" {
                format!("The left clause line {row}")
            } else {
                format!("The right clause line {row}")
            };
            let next = spans.get(index + 1).map(|s| s.text.as_str()).unwrap_or("");
            assert!(
                next.starts_with(&expected_prefix),
                "clause number {:?} at position {index} must be immediately followed by its own \
                 clause text, found {next:?} -- numbers must never be hoisted ahead of their clauses",
                span.text
            );
            checked += 1;
        }
        // Rows 0-3 each carry both a left and right number (8), rows 4-7 carry only a
        // left number (4), and rows 8-11 carry only a right number (4).
        assert_eq!(
            checked, 16,
            "all 16 clause numbers in the fixture must have been checked"
        );
    }

    const CORRIDOR_PAGE_WIDTH: f32 = 595.32;
    // Geometry lifted from the reporter's carrier, page 1 (A4): hanging clause numbers at
    // x=36.0 (left) and x=304.87 (right), body text at 64.34 and 333.19, the left
    // column's longest lines ending at 292.51, so the true gutter is 292.51..304.87.
    const CORRIDOR_TRUE_GUTTER_MID_X: f32 = (292.51 + 304.87) / 2.0;

    /// Two clause-numbered columns whose baselines are offset by 1.32pt (above
    /// `LINE_Y_TOLERANCE_PTS`), so almost no line carries spans from both columns and
    /// the per-line votes are the two number indents; four coincidentally aligned rows
    /// vote for the gutter with a midpoint 3.4pt *inside* the left column's longest
    /// lines. A heading crosses each indent, and a centred footer crosses the gutter --
    /// one line each, which closes all three as `page_whitespace_corridors` corridors.
    fn corridor_closed_by_one_footer_line_spans(with_footer: bool) -> Vec<TextSpan> {
        let mut spans = Vec::new();
        spans.push(span_with_width(
            "Article 1 Applicability",
            36.0,
            920.0,
            160.0,
            11.0,
            11.0,
        ));
        spans.push(span_with_width(
            "Article 3 Price and payment",
            304.87,
            920.0,
            130.0,
            11.0,
            11.0,
        ));
        for row in 0..15 {
            let y = 900.0 - row as f32 * 14.0;
            let cross = row % 4 == 1 && row < 16; // rows 1, 5, 9, 13 share a baseline
            let left_width = if cross { 208.96 } else { 228.17 }; // 273.3 vs 292.51
            spans.push(span_with_width(&format!("1.{}", row + 1), 36.0, y, 11.4, 11.0, 11.0));
            spans.push(span_with_width(
                &format!("left clause line {row} continues with ordinary agreement terms"),
                64.34,
                y,
                left_width,
                11.0,
                11.0,
            ));
            let right_y = if cross { y } else { y - 1.32 };
            spans.push(span_with_width(
                &format!("3.{}", row + 1),
                304.87,
                right_y,
                13.4,
                11.0,
                11.0,
            ));
            spans.push(span_with_width(
                &format!("right clause line {row} continues with ordinary agreement terms"),
                333.19,
                right_y,
                220.0,
                11.0,
                11.0,
            ));
        }
        if with_footer {
            spans.push(span_with_width(
                "takes precedence.        \u{a9} 2020 NLdigital",
                222.89,
                60.0,
                149.57,
                9.0,
                9.0,
            ));
        }
        spans
    }

    fn corridor_fixture_lines(spans: &[TextSpan]) -> Vec<SpanLine> {
        let order = spans_sorted_top_to_bottom(spans);
        group_into_lines(spans, &order)
    }

    /// The per-line median lands 3.4pt inside the left column (the fixture reproduces
    /// the carrier's vote population), the footer closes the gutter as a whitespace
    /// corridor, and the split is then moved into the gutter through the
    /// low-occupancy search rather than left inside the column.
    #[test]
    fn split_inside_a_column_is_moved_to_a_gutter_one_footer_line_crosses() {
        let spans = corridor_closed_by_one_footer_line_spans(true);
        let lines = corridor_fixture_lines(&spans);
        let furniture_width = CORRIDOR_PAGE_WIDTH * FULL_WIDTH_FURNITURE_FRACTION;
        let min_gutter = (CORRIDOR_PAGE_WIDTH * MIN_DENSE_COLUMN_GUTTER_FRACTION).max(MIN_DENSE_COLUMN_GUTTER_PTS);

        let detected = detect_split_x(&spans, &lines, CORRIDOR_PAGE_WIDTH).expect("a split is detected");
        assert!(
            detected < 292.51 && detected > 280.0,
            "the fixture must put the median inside the left column's longest lines, got {detected}"
        );
        let snapped = snap_split_left_of_hanging_labels(&spans, &lines, CORRIDOR_PAGE_WIDTH, detected);
        assert_eq!(snapped, detected, "no narrow span straddles the median");
        assert!(
            lines_crossing(&spans, &lines, furniture_width, snapped) >= MIN_DENSE_COLUMN_SPLIT_LINES,
            "the long left lines must run through the split, or the fixture no longer exercises the search"
        );
        assert!(
            page_whitespace_corridors(&spans, &lines, furniture_width, min_gutter).is_empty(),
            "the footer and the two headings must close every whitespace corridor"
        );
        let low_occupancy = page_low_occupancy_corridors(&spans, &lines, furniture_width, min_gutter, 1);
        assert!(
            low_occupancy
                .iter()
                .any(|&(l, r)| (l - 292.51).abs() < 0.01 && (r - 304.87).abs() < 0.01),
            "the gutter must come back once one crossing line is tolerated, got {low_occupancy:?}"
        );
        let max_label_width = CORRIDOR_PAGE_WIDTH * MAX_DENSE_COLUMN_SPLIT_SNAP_SPAN_FRACTION;
        let kept: Vec<(f32, f32)> = low_occupancy
            .iter()
            .copied()
            .filter(|&c| !corridor_is_hanging_label_indent(&spans, &lines, max_label_width, c))
            .collect();
        assert_eq!(
            kept.len(),
            1,
            "both number indents are label-walled and must be excluded, leaving the gutter: {kept:?}"
        );

        assert!(
            both_sides_are_columns(&spans, &lines, furniture_width, CORRIDOR_TRUE_GUTTER_MID_X),
            "two clause columns must read as columns on both sides of the gutter"
        );

        let redirected = redirect_split_out_of_content(&spans, &lines, CORRIDOR_PAGE_WIDTH, snapped);
        assert!(
            (redirected - CORRIDOR_TRUE_GUTTER_MID_X).abs() < 0.01,
            "the split must land mid-gutter at {CORRIDOR_TRUE_GUTTER_MID_X}, got {redirected}"
        );

        // End to end: every clause of Article 1 precedes every clause of Article 3.
        let mut reordered = spans.clone();
        assert!(reorder_dense_two_column_page(&mut reordered, CORRIDOR_PAGE_WIDTH));
        let last_left = reordered.iter().rposition(|s| s.text.starts_with("1.")).unwrap();
        let first_right = reordered.iter().position(|s| s.text.starts_with("3.")).unwrap();
        assert!(
            last_left < first_right,
            "the left column must be emitted before the right one; the last left clause \
             number sits at {last_left}, the first right one at {first_right}"
        );
    }

    /// Without the footer the gutter is an ordinary whitespace corridor and the
    /// existing redirect already finds it; the widened search is never consulted and
    /// the outcome is the same, which is what keeps the change out of every page that
    /// is repaired today.
    #[test]
    fn split_inside_a_column_still_takes_the_whitespace_corridor_when_nothing_crosses_it() {
        let spans = corridor_closed_by_one_footer_line_spans(false);
        let lines = corridor_fixture_lines(&spans);
        let detected = detect_split_x(&spans, &lines, CORRIDOR_PAGE_WIDTH).expect("a split is detected");
        let redirected = redirect_split_out_of_content(&spans, &lines, CORRIDOR_PAGE_WIDTH, detected);
        assert!(
            (redirected - CORRIDOR_TRUE_GUTTER_MID_X).abs() < 0.01,
            "the whitespace corridor must still win on its own, got {redirected}"
        );
    }

    /// A split that a single heading runs through is the ordinary two-column page and
    /// must be left exactly where the per-line evidence put it.
    #[test]
    fn split_crossed_by_one_heading_is_not_redirected() {
        let mut spans = Vec::new();
        spans.push(span_with_width(
            "A heading set across both columns",
            150.0,
            920.0,
            300.0,
            11.0,
            11.0,
        ));
        for row in 0..8 {
            let y = 900.0 - row as f32 * 14.0;
            spans.push(span_with_width("left body text of the row", 36.0, y, 240.0, 11.0, 11.0));
            spans.push(span_with_width(
                "right body text of the row",
                306.0,
                y,
                240.0,
                11.0,
                11.0,
            ));
        }
        let lines = corridor_fixture_lines(&spans);
        let furniture_width = CORRIDOR_PAGE_WIDTH * FULL_WIDTH_FURNITURE_FRACTION;
        let detected = detect_split_x(&spans, &lines, CORRIDOR_PAGE_WIDTH).expect("a split is detected");
        assert_eq!(
            lines_crossing(&spans, &lines, furniture_width, detected),
            1,
            "only the heading crosses"
        );
        let redirected = redirect_split_out_of_content(&spans, &lines, CORRIDOR_PAGE_WIDTH, detected);
        assert_eq!(redirected, detected, "one crossing line is not grounds for a search");
    }

    /// A split inside a table's description column is crossed by every row, but the
    /// gap to the value column is not a gutter: short cells do not read as a column,
    /// so the widened search declines and the split stays where it was.
    #[test]
    fn split_inside_a_table_column_is_not_moved_to_the_cell_gap() {
        let mut spans = Vec::new();
        spans.push(span_with_width(
            "Product list valid for all regions and partners",
            36.0,
            920.0,
            380.0,
            11.0,
            11.0,
        ));
        for row in 0..12 {
            let y = 900.0 - row as f32 * 14.0;
            spans.push(span_with_width(&format!("V-{row:03}"), 36.0, y, 40.0, 11.0, 11.0));
            spans.push(span_with_width(
                "software subscription licence per channel for the analytics platform",
                90.0,
                y,
                300.0,
                11.0,
                11.0,
            ));
            spans.push(span_with_width("$120", 420.0, y, 30.0, 11.0, 11.0));
            spans.push(span_with_width("$1,200", 480.0, y, 40.0, 11.0, 11.0));
        }
        let lines = corridor_fixture_lines(&spans);
        let furniture_width = CORRIDOR_PAGE_WIDTH * FULL_WIDTH_FURNITURE_FRACTION;
        // A split placed by hand inside the description column, as a bimodal median can.
        let split_x = 250.0;
        assert!(lines_crossing(&spans, &lines, furniture_width, split_x) >= MIN_DENSE_COLUMN_SPLIT_LINES);
        assert!(
            !both_sides_are_columns(&spans, &lines, furniture_width, 405.0),
            "the price cells must not read as a column"
        );
        let redirected = redirect_split_out_of_content(&spans, &lines, CORRIDOR_PAGE_WIDTH, split_x);
        assert_eq!(
            redirected, split_x,
            "the cell gap at ~405 is not a gutter; the split must stay"
        );
    }

    /// A band inside a table is crossed on every row, so tolerating one crossing line
    /// opens nothing there.
    #[test]
    fn low_occupancy_corridors_open_nothing_inside_a_table() {
        let mut spans = Vec::new();
        for row in 0..8 {
            let y = 900.0 - row as f32 * 14.0;
            spans.push(span_with_width("cell one", 36.0, y, 100.0, 11.0, 11.0));
            spans.push(span_with_width(
                "cell two spanning the middle of the page",
                150.0,
                y,
                300.0,
                11.0,
                11.0,
            ));
            spans.push(span_with_width("cell three", 470.0, y, 80.0, 11.0, 11.0));
        }
        let lines = corridor_fixture_lines(&spans);
        let furniture_width = CORRIDOR_PAGE_WIDTH * FULL_WIDTH_FURNITURE_FRACTION;
        let min_gutter = (CORRIDOR_PAGE_WIDTH * MIN_DENSE_COLUMN_GUTTER_FRACTION).max(MIN_DENSE_COLUMN_GUTTER_PTS);
        let strict = page_whitespace_corridors(&spans, &lines, furniture_width, min_gutter);
        let tolerant = page_low_occupancy_corridors(&spans, &lines, furniture_width, min_gutter, 1);
        assert_eq!(
            strict, tolerant,
            "the two cell gaps are corridors either way; nothing new opens"
        );
        assert_eq!(strict.len(), 2);
    }

    const GH1742_PAGE_WIDTH: f32 = 595.0;
    const GH1742_LEFT_X: f32 = 38.0;
    const GH1742_RIGHT_X: f32 = 307.0;
    const GH1742_LEFT_WIDTH: f32 = 250.0;
    const GH1742_RIGHT_WIDTH: f32 = 240.0;
    const GH1742_TRUE_GUTTER_MID_X: f32 = (288.0 + 307.0) / 2.0;
    const GH1742_TABLE_COLUMNS: [(f32, f32); 5] =
        [(38.0, 30.0), (84.0, 30.0), (130.0, 30.0), (176.0, 30.0), (222.0, 10.0)];
    const GH1742_TABLE_ROWS: usize = 10;
    const GH1742_TABLE_ROW_HEIGHT: f32 = 9.0;

    /// GH#1742 (reproducer p1 shape): six paired prose rows carry the page's real
    /// gutter evidence (left column ends at 288, right column starts at 307), then a
    /// 5-column table -- own, denser leading, last column narrow -- fills the rest of
    /// the left column, and a single centred page number sits in the gutter near the
    /// foot of the page. Before the fix, the table's 10 rows each vote once for their
    /// own widest *internal* cell gap (deep inside the table, nowhere near the real
    /// gutter), outvoting the 6 genuine gutter lines and landing the median at 76.0 --
    /// measured directly in `detect_split_x_ignores_table_grid_row_votes_gh1742` below.
    fn gh1742_two_column_page_with_lower_table_and_page_number() -> Vec<TextSpan> {
        let mut spans = Vec::new();
        for row in 0..MIN_DENSE_COLUMN_SPLIT_LINES {
            let y = 900.0 - row as f32 * 14.0;
            spans.push(span_with_width(
                &format!("left column body text for row {row}"),
                GH1742_LEFT_X,
                y,
                GH1742_LEFT_WIDTH,
                11.0,
                11.0,
            ));
            spans.push(span_with_width(
                &format!("right column body text for row {row}"),
                GH1742_RIGHT_X,
                y,
                GH1742_RIGHT_WIDTH,
                11.0,
                11.0,
            ));
        }
        let table_top = 900.0 - MIN_DENSE_COLUMN_SPLIT_LINES as f32 * 14.0 - 6.0;
        for row in 0..GH1742_TABLE_ROWS {
            let y = table_top - row as f32 * GH1742_TABLE_ROW_HEIGHT;
            for (column, &(x, width)) in GH1742_TABLE_COLUMNS.iter().enumerate() {
                spans.push(span_with_width(&format!("c{column}"), x, y, width, 6.5, 6.5));
            }
        }
        let page_number_width = 4.5;
        let page_number_x = GH1742_TRUE_GUTTER_MID_X - page_number_width / 2.0;
        spans.push(span_with_width("6", page_number_x, 40.0, page_number_width, 8.0, 8.0));
        spans
    }

    /// GH#1742: `detect_split_x`'s median must survive a table's own internal-gap
    /// votes and land in the true gutter. Measured on this fixture pre-fix: the
    /// table's 10 rows each contribute one vote for their widest internal cell gap
    /// (76.0, between the table's first two columns), outvoting the 6 real gutter
    /// votes (297.5) and landing the median at 76.0 -- deep inside the left column.
    #[test]
    fn detect_split_x_ignores_table_grid_row_votes_gh1742() {
        let spans = gh1742_two_column_page_with_lower_table_and_page_number();
        let order = spans_sorted_top_to_bottom(&spans);
        let lines = group_into_lines(&spans, &order);

        let detected = detect_split_x(&spans, &lines, GH1742_PAGE_WIDTH)
            .expect("six paired prose rows meet the quorum on their own");
        assert!(
            (detected - GH1742_TRUE_GUTTER_MID_X).abs() < 0.5,
            "the table's grid rows must not pollute the vote; expected the true gutter \
             at {GH1742_TRUE_GUTTER_MID_X}, got {detected}"
        );
    }

    /// GH#1742 end to end (reproducer p1 shape): the page must reorder column-major --
    /// the whole left column (prose then table, in their existing top-to-bottom order)
    /// followed by the whole right column, with the page number left as its own
    /// trailing boundary line. Before the fix, the polluted vote (76.0) lands inside
    /// both the left prose spans and one of the table's own columns, every line
    /// becomes a single-line boundary band, no band ever reorders, and the page is
    /// left exactly as extracted -- interleaved left/right rows, exactly the reported
    /// defect. Measured directly against unmodified code: `reorder_dense_two_column_page`
    /// returns `false` and every span keeps its original (interleaved) position.
    #[test]
    fn dense_two_column_page_with_lower_table_and_page_number_reorders_by_column_gh1742() {
        let mut spans = gh1742_two_column_page_with_lower_table_and_page_number();

        assert!(
            reorder_dense_two_column_page(&mut spans, GH1742_PAGE_WIDTH),
            "a two-column page with a table in the lower half of one column and a \
             page number in the gutter must still be reordered"
        );

        let mut expected: Vec<String> = (0..MIN_DENSE_COLUMN_SPLIT_LINES)
            .map(|row| format!("left column body text for row {row}"))
            .collect();
        for _row in 0..GH1742_TABLE_ROWS {
            expected.extend((0..GH1742_TABLE_COLUMNS.len()).map(|column| format!("c{column}")));
        }
        expected.extend((0..MIN_DENSE_COLUMN_SPLIT_LINES).map(|row| format!("right column body text for row {row}")));
        expected.push("6".to_string());

        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            expected.iter().map(String::as_str).collect::<Vec<_>>(),
            "left column (prose then table) must precede the right column, with the \
             page number trailing as its own boundary line"
        );
    }

    const GH1742_P6_TABLE_ROWS: usize = 8;
    const GH1742_P6_TABLE_COLUMNS: [(f32, f32); 4] = [(38.0, 30.0), (84.0, 30.0), (130.0, 30.0), (176.0, 30.0)];
    // The table's fifth column starts just inside the left column and straddles the
    // gutter, matching the reporter's own measurement of a numeric column that begins
    // in-column and runs past the true split. ~keep
    const GH1742_P6_STRADDLE_COLUMN_X: f32 = 283.0;
    const GH1742_P6_STRADDLE_COLUMN_WIDTH: f32 = 32.0;

    /// GH#1742 (reproducer p6 shape): a full-width table sits above two ordinary prose
    /// columns; the table's fifth column starts inside the left column and straddles
    /// the true gutter. Before the fix, `aligned_hanging_label_left_edge` read that
    /// numeric column as a stack of hanging clause numbers and snapped the split from
    /// the true gutter into the table's own fourth-column gap; `detect_split_x` was
    /// separately polluted by the table's own multi-column internal gaps. Fixing the
    /// vote (`line_has_grid_row_gaps` in both `detect_split_x` and
    /// `aligned_hanging_label_left_edge`) removes both failure paths at once: the table
    /// rows never enter the vote, so the median comes only from the six real gutter
    /// lines below, and the same exclusion removes the straddling column from the
    /// snap's candidate pool. Every table row still straddles the (correct) split, so
    /// each stays a single boundary line in its own top-to-bottom, left-to-right
    /// order -- "table, then prose", matching the reporter's own expected output.
    fn gh1742_full_width_table_above_two_column_prose() -> Vec<TextSpan> {
        let mut spans = Vec::new();
        for row in 0..GH1742_P6_TABLE_ROWS {
            let y = 950.0 - row as f32 * 9.0;
            for (column, &(x, width)) in GH1742_P6_TABLE_COLUMNS.iter().enumerate() {
                spans.push(span_with_width(&format!("c{column}"), x, y, width, 6.5, 6.5));
            }
            spans.push(span_with_width(
                "269,533",
                GH1742_P6_STRADDLE_COLUMN_X,
                y,
                GH1742_P6_STRADDLE_COLUMN_WIDTH,
                6.5,
                6.5,
            ));
        }
        let prose_top = 950.0 - GH1742_P6_TABLE_ROWS as f32 * 9.0 - 20.0;
        for row in 0..MIN_DENSE_COLUMN_SPLIT_LINES {
            let y = prose_top - row as f32 * 14.0;
            spans.push(span_with_width(
                &format!("left column body text for row {row}"),
                GH1742_LEFT_X,
                y,
                GH1742_LEFT_WIDTH,
                11.0,
                11.0,
            ));
            spans.push(span_with_width(
                &format!("right column body text for row {row}"),
                GH1742_RIGHT_X,
                y,
                GH1742_RIGHT_WIDTH,
                11.0,
                11.0,
            ));
        }
        spans
    }

    #[test]
    fn detect_split_x_ignores_full_width_table_grid_row_votes_gh1742() {
        let spans = gh1742_full_width_table_above_two_column_prose();
        let order = spans_sorted_top_to_bottom(&spans);
        let lines = group_into_lines(&spans, &order);

        let detected = detect_split_x(&spans, &lines, GH1742_PAGE_WIDTH)
            .expect("six paired prose rows meet the quorum on their own");
        assert!(
            (detected - GH1742_TRUE_GUTTER_MID_X).abs() < 0.5,
            "the full-width table's grid rows must not pollute the vote; expected the \
             true gutter at {GH1742_TRUE_GUTTER_MID_X}, got {detected}"
        );

        let snapped = snap_split_left_of_hanging_labels(&spans, &lines, GH1742_PAGE_WIDTH, detected);
        assert_eq!(
            snapped, detected,
            "the straddling numeric column must not be read as a hanging-label stack"
        );
    }

    #[test]
    fn dense_two_column_page_with_full_width_table_above_reorders_table_then_prose_gh1742() {
        let mut spans = gh1742_full_width_table_above_two_column_prose();

        assert!(
            reorder_dense_two_column_page(&mut spans, GH1742_PAGE_WIDTH),
            "a full-width table above a two-column prose body must still be reordered"
        );

        let mut expected = Vec::new();
        for _row in 0..GH1742_P6_TABLE_ROWS {
            expected.extend((0..GH1742_P6_TABLE_COLUMNS.len()).map(|column| format!("c{column}")));
            expected.push("269,533".to_string());
        }
        expected.extend((0..MIN_DENSE_COLUMN_SPLIT_LINES).map(|row| format!("left column body text for row {row}")));
        expected.extend((0..MIN_DENSE_COLUMN_SPLIT_LINES).map(|row| format!("right column body text for row {row}")));

        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            expected.iter().map(String::as_str).collect::<Vec<_>>(),
            "the table must stay in its own row order, followed by the reordered \
             left-then-right prose columns"
        );
    }

    const GH1742_P4_NORMAL_ROWS: usize = 10;
    const GH1742_P4_COLLAPSED_ROWS: usize = 6;
    // Three columns, wide last column -- only two internal gaps per row, below
    // `MIN_GRID_ROW_GAP_COUNT`, so fix #1 alone does not remove these rows from the
    // vote. Left column occupies the full page height; the right column is ordinary
    // prose on its own (unrelated) leading, never sharing a baseline with the table.
    // Positioned so the (wrong) per-row vote sits within `MAX_REDIRECT_DISTANCE_FRACTION`
    // of the true gutter -- otherwise the pre-existing GH#1603 distance cap discards
    // any candidate the widened search finds, independent of fix #3. ~keep
    const GH1742_P4_TABLE_COLUMNS: [(f32, f32); 3] = [(38.0, 147.0), (200.0, 30.0), (245.0, 45.0)];
    const GH1742_P4_TABLE_ROW_HEIGHT: f32 = 9.0;
    // A deliberately offset leading (not a multiple of the table's or prose's own row
    // height) so no table row ever lands on the same visual line as a prose row. ~keep
    const GH1742_P4_TABLE_Y_OFFSET: f32 = 0.7;

    /// GH#1742 (reproducer p4 shape): the entire left column, top to bottom, is a
    /// 3-column table (wide last column, so only two internal gaps per row -- fix #1's
    /// `MIN_GRID_ROW_GAP_COUNT` of four does not exclude these rows from the vote);
    /// the right column is ordinary prose; a centred page number sits in the gutter.
    ///
    /// Ten rows keep the plain 3-column shape and vote for their own internal gap
    /// (deep inside the table); six rows collapse the second column into the third
    /// (one wide cell starting right after the first), so they contribute no vote but
    /// their wide cell occupies exactly the x-range the other ten rows vote for --
    /// forcing that wrong median to both cut a span and cross
    /// `MIN_DENSE_COLUMN_SPLIT_LINES` lines, which is what sends
    /// `redirect_split_out_of_content` into the widened low-occupancy search rather
    /// than leaving the (already-wrong) median untouched. `page_whitespace_corridors`
    /// still finds nothing there (the page number closes the true gutter), and the
    /// widened search's own candidate is refused by `both_sides_are_columns` unless
    /// the left (table) side is allowed to be `Table`/`Mixed`/`Form` -- exactly the gap
    /// fix #3 closes, mirroring what `reorder_band_columns` already accepts once a
    /// split is actually handed to it.
    fn gh1742_full_height_table_column_with_page_number() -> Vec<TextSpan> {
        let mut spans = Vec::new();
        for row in 0..GH1742_P4_NORMAL_ROWS {
            let y = 900.0 - row as f32 * GH1742_P4_TABLE_ROW_HEIGHT - GH1742_P4_TABLE_Y_OFFSET;
            for (column, &(x, width)) in GH1742_P4_TABLE_COLUMNS.iter().enumerate() {
                spans.push(span_with_width(&format!("t{column}"), x, y, width, 6.5, 6.5));
            }
        }
        for row in 0..GH1742_P4_COLLAPSED_ROWS {
            let y =
                900.0 - (GH1742_P4_NORMAL_ROWS + row) as f32 * GH1742_P4_TABLE_ROW_HEIGHT - GH1742_P4_TABLE_Y_OFFSET;
            let (col0_x, col0_width) = GH1742_P4_TABLE_COLUMNS[0];
            let (col2_x, col2_width) = GH1742_P4_TABLE_COLUMNS[2];
            spans.push(span_with_width("t0", col0_x, y, col0_width, 6.5, 6.5));
            spans.push(span_with_width(
                "wide",
                col0_x + col0_width,
                y,
                col2_x + col2_width - (col0_x + col0_width),
                6.5,
                6.5,
            ));
        }
        for row in 0..(GH1742_P4_NORMAL_ROWS + GH1742_P4_COLLAPSED_ROWS) {
            let y = 900.0 - row as f32 * 14.0;
            spans.push(span_with_width(
                &format!("right column body text for row {row}"),
                GH1742_RIGHT_X,
                y,
                GH1742_RIGHT_WIDTH,
                11.0,
                11.0,
            ));
        }
        let page_number_width = 4.5;
        let page_number_x = GH1742_TRUE_GUTTER_MID_X - page_number_width / 2.0;
        spans.push(span_with_width("6", page_number_x, 40.0, page_number_width, 8.0, 8.0));
        spans
    }

    #[test]
    fn dense_two_column_page_with_full_height_table_column_reorders_table_then_prose_gh1742() {
        let mut spans = gh1742_full_height_table_column_with_page_number();

        assert!(
            reorder_dense_two_column_page(&mut spans, GH1742_PAGE_WIDTH),
            "a full-height table column beside ordinary prose, with a page number in \
             the gutter, must still be reordered"
        );

        let mut expected = Vec::new();
        for _row in 0..GH1742_P4_NORMAL_ROWS {
            expected.extend((0..GH1742_P4_TABLE_COLUMNS.len()).map(|column| format!("t{column}")));
        }
        for _row in 0..GH1742_P4_COLLAPSED_ROWS {
            expected.push("t0".to_string());
            expected.push("wide".to_string());
        }
        expected.extend(
            (0..(GH1742_P4_NORMAL_ROWS + GH1742_P4_COLLAPSED_ROWS))
                .map(|row| format!("right column body text for row {row}")),
        );
        expected.push("6".to_string());

        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            expected.iter().map(String::as_str).collect::<Vec<_>>(),
            "the table column must precede the prose column, with the page number \
             trailing as its own boundary line"
        );
    }

    const GH1742_P4B_INTRO_WIDTH: f32 = 200.0;

    /// GH#1742 (reproducer p4, real mechanism, traced against `/1742.pdf` page 4 with a
    /// temporary instrumented test -- 271 spans, 122 lines, `detect_split_x` = 186.392,
    /// exactly one line crossing that split, 71 lines with an internal gap at it).
    ///
    /// Unlike `gh1742_full_height_table_column_with_page_number` above, no span is
    /// engineered to literally straddle the wrong split on more than one line: every
    /// table row is a plain 3-column row (two internal gaps, below
    /// `MIN_GRID_ROW_GAP_COUNT`, so fix #1 does not touch the vote), each row's own
    /// gap is real whitespace, and the only span in the whole page that crosses
    /// `detect_split_x`'s median is a single wide introductory line above the table
    /// (the reporter's own carrier has exactly this shape: a paragraph before "Table
    /// 2"). Before this fix, `redirect_split_out_of_content`'s `cuts_a_span` gate was
    /// satisfied by that one intro line, but its own `lines_crossing` count then never
    /// rose above 1 -- the ten table rows evidence the split is inside a column only
    /// through their own internal gaps, never through a span crossing -- so the
    /// widened corridor search never ran and the split stayed inside the table.
    fn gh1742_full_height_table_column_no_crossing_spans() -> Vec<TextSpan> {
        let mut spans = Vec::new();
        let (col0_x, _) = GH1742_P4_TABLE_COLUMNS[0];
        spans.push(span_with_width(
            "an introductory paragraph precedes the table on its own line",
            col0_x,
            910.0,
            GH1742_P4B_INTRO_WIDTH,
            8.0,
            8.0,
        ));
        for row in 0..GH1742_P4_NORMAL_ROWS {
            let y = 900.0 - row as f32 * GH1742_P4_TABLE_ROW_HEIGHT - GH1742_P4_TABLE_Y_OFFSET;
            for (column, &(x, width)) in GH1742_P4_TABLE_COLUMNS.iter().enumerate() {
                spans.push(span_with_width(&format!("t{column}"), x, y, width, 6.5, 6.5));
            }
        }
        for row in 0..GH1742_P4_NORMAL_ROWS {
            let y = 900.0 - row as f32 * 14.0;
            spans.push(span_with_width(
                &format!("right column body text for row {row}"),
                GH1742_RIGHT_X,
                y,
                GH1742_RIGHT_WIDTH,
                11.0,
                11.0,
            ));
        }
        let page_number_width = 4.5;
        let page_number_x = GH1742_TRUE_GUTTER_MID_X - page_number_width / 2.0;
        spans.push(span_with_width("6", page_number_x, 40.0, page_number_width, 8.0, 8.0));
        spans
    }

    /// GH#1742 (reproducer p4, real mechanism): with no span crossing the wrong split
    /// on more than one line, `redirect_split_out_of_content` must still be reached
    /// through `lines_with_internal_gap_at`'s table-row evidence, and the page must
    /// still reorder table-then-prose rather than emit top-to-bottom.
    #[test]
    fn dense_two_column_page_with_table_column_and_no_crossing_spans_reorders_table_then_prose_gh1742() {
        let spans = gh1742_full_height_table_column_no_crossing_spans();
        let order = spans_sorted_top_to_bottom(&spans);
        let lines = group_into_lines(&spans, &order);

        let detected = detect_split_x(&spans, &lines, GH1742_PAGE_WIDTH).expect("table rows vote for a split");
        let snapped = snap_split_left_of_hanging_labels(&spans, &lines, GH1742_PAGE_WIDTH, detected);
        let furniture_width = GH1742_PAGE_WIDTH * FULL_WIDTH_FURNITURE_FRACTION;
        assert_eq!(
            lines_crossing(&spans, &lines, furniture_width, snapped),
            1,
            "only the intro line may cross the wrong split, or this fixture no longer \
             exercises the no-span-crossing mechanism"
        );

        let mut spans = spans;
        assert!(
            reorder_dense_two_column_page(&mut spans, GH1742_PAGE_WIDTH),
            "a table column beside ordinary prose, with the wrong split crossed by only \
             one line, must still be reordered"
        );

        let mut expected = vec!["an introductory paragraph precedes the table on its own line".to_string()];
        for _row in 0..GH1742_P4_NORMAL_ROWS {
            expected.extend((0..GH1742_P4_TABLE_COLUMNS.len()).map(|column| format!("t{column}")));
        }
        expected.extend((0..GH1742_P4_NORMAL_ROWS).map(|row| format!("right column body text for row {row}")));
        expected.push("6".to_string());

        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            expected.iter().map(String::as_str).collect::<Vec<_>>(),
            "the intro line and table column must precede the prose column, with the \
             page number trailing as its own boundary line"
        );
    }

    const GH1742_P1_PAGE_WIDTH: f32 = 595.28;

    /// GH#1742 real reproducer, page 1, transcribed verbatim (one span per
    /// `PdfDocument::extract_spans` entry, `x`/`y`/`width`/`height` unmodified) from
    /// `/1742.pdf` -- not committed; see the issue's own geometry table. A five-column
    /// table (`Pars`/`Ex anno`/`Valor` plus a narrow fifth column at `x=262`, right
    /// edges up to `286.90`, matching the issue's `ignotum` stack) fills the lower half
    /// of the left column; a centred page number (`"1"`, `x=295.42..299.87`) sits in
    /// the gutter. Before fix #2, `corridor_is_hanging_label_indent` reads the fifth
    /// column's narrow, left-aligned cells as a hanging-label stack (nothing
    /// distinguishes them from a real clause number) and the widened corridor search
    /// refuses the true gutter, leaving `detect_split_x`'s polluted median (inside the
    /// table) as the final split.
    fn gh1742_p1_real_reproducer_spans() -> Vec<TextSpan> {
        #[rustfmt::skip]
        let spans = vec![
            span_with_width("2.1. Lorem ipsum dolor sit amet", 38.00, 770.00, 133.05, 9.50, 9.50),
            span_with_width("incididunt ut labore et dolore magna aliqua enim ad minim veniam", 38.00, 759.55, 247.55, 8.50, 8.50),
            span_with_width("ut labore et dolore magna aliqua enim ad minim veniam quis", 38.00, 749.10, 227.23, 8.50, 8.50),
            span_with_width("labore et dolore magna aliqua enim ad minim veniam quis nostrud", 38.00, 738.65, 248.49, 8.50, 8.50),
            span_with_width("et dolore magna aliqua enim ad minim veniam quis nostrud", 38.00, 728.20, 222.50, 8.50, 8.50),
            span_with_width("dolore magna aliqua enim ad minim veniam quis nostrud", 38.00, 717.75, 213.05, 8.50, 8.50),
            span_with_width("magna aliqua enim ad minim veniam quis nostrud exercitation", 38.00, 707.30, 232.89, 8.50, 8.50),
            span_with_width("aliqua enim ad minim veniam quis nostrud exercitation ullamco", 38.00, 696.85, 236.19, 8.50, 8.50),
            span_with_width("enim ad minim veniam quis nostrud exercitation ullamco laboris", 38.00, 686.40, 238.54, 8.50, 8.50),
            span_with_width("ut labore et dolore magna aliqua enim ad minim veniam quis", 307.00, 686.40, 227.23, 8.50, 8.50),
            span_with_width("labore et dolore magna aliqua enim ad minim veniam quis nostrud", 307.00, 675.95, 248.49, 8.50, 8.50),
            span_with_width("2.2. Consectetur adipiscing elit", 38.00, 665.50, 129.36, 9.50, 9.50),
            span_with_width("velit esse cillum fugiat nulla pariatur excepteur sint occaecat", 38.00, 655.05, 225.81, 8.50, 8.50),
            span_with_width("esse cillum fugiat nulla pariatur excepteur sint occaecat cupidatat", 38.00, 644.60, 245.19, 8.50, 8.50),
            span_with_width("cillum fugiat nulla pariatur excepteur sint occaecat cupidatat non", 38.00, 634.15, 241.42, 8.50, 8.50),
            span_with_width("fugiat nulla pariatur excepteur sint occaecat cupidatat non proident", 38.00, 623.70, 250.41, 8.50, 8.50),
            span_with_width("nulla pariatur excepteur sint occaecat cupidatat non proident sunt", 38.00, 613.25, 245.68, 8.50, 8.50),
            span_with_width("pariatur excepteur sint occaecat cupidatat non proident sunt culpa", 38.00, 602.80, 248.05, 8.50, 8.50),
            span_with_width("minim veniam quis nostrud exercitation ullamco laboris nisi aliquip", 307.00, 602.80, 247.99, 8.50, 8.50),
            span_with_width("veniam quis nostrud exercitation ullamco laboris nisi aliquip ex ea", 307.00, 592.35, 246.12, 8.50, 8.50),
            span_with_width("Table 1", 38.00, 581.90, 23.34, 7.00, 7.00),
            span_with_width("Lorem ipsum dolor sit amet in consectetur adipiscing elit.", 38.00, 573.85, 175.84, 7.00, 7.00),
            span_with_width("Pars", 92.00, 563.80, 14.39, 7.00, 7.00),
            span_with_width("Ex anno", 140.00, 563.80, 25.68, 7.00, 7.00),
            span_with_width("Valor", 185.00, 563.80, 16.34, 7.00, 7.00),
            span_with_width("Dolor", 44.00, 555.75, 16.72, 7.00, 7.00),
            span_with_width("91%", 92.00, 555.75, 14.01, 7.00, 7.00),
            span_with_width("Anno 1983", 140.00, 555.75, 33.86, 7.00, 7.00),
            span_with_width("Praedictum", 185.00, 555.75, 35.40, 7.00, 7.00),
            span_with_width("Sit amet", 44.00, 547.70, 25.68, 7.00, 7.00),
            span_with_width("80.5%", 92.00, 547.70, 19.85, 7.00, 7.00),
            span_with_width("Anno 2023", 140.00, 547.70, 33.86, 7.00, 7.00),
            span_with_width("Exclusio", 185.00, 547.70, 26.06, 7.00, 7.00),
            span_with_width("Lorem", 44.00, 539.65, 19.84, 7.00, 7.00),
            span_with_width("40-70%", 92.00, 539.65, 24.12, 7.00, 7.00),
            span_with_width("Anno 2016", 140.00, 539.65, 33.86, 7.00, 7.00),
            span_with_width("Conflictus", 185.00, 539.65, 30.73, 7.00, 7.00),
            span_with_width("Ipsum", 44.00, 531.60, 19.06, 7.00, 7.00),
            span_with_width("10-20%", 92.00, 531.60, 24.12, 7.00, 7.00),
            span_with_width("Nulla", 140.00, 531.60, 15.95, 7.00, 7.00),
            span_with_width("Nullum", 185.00, 531.60, 21.78, 7.00, 7.00),
            span_with_width("Magna", 44.00, 523.55, 21.40, 7.00, 7.00),
            span_with_width("10-15%", 92.00, 523.55, 24.12, 7.00, 7.00),
            span_with_width("Anno 2001", 140.00, 523.55, 33.86, 7.00, 7.00),
            span_with_width("Peior", 185.00, 523.55, 16.34, 7.00, 7.00),
            span_with_width("Aliqua", 44.00, 515.50, 19.45, 7.00, 7.00),
            span_with_width("50-79%", 92.00, 515.50, 24.12, 7.00, 7.00),
            span_with_width("Anno 2019", 140.00, 515.50, 33.86, 7.00, 7.00),
            span_with_width("Melior", 185.00, 515.50, 19.05, 7.00, 7.00),
            span_with_width("Veniam", 44.00, 507.45, 23.73, 7.00, 7.00),
            span_with_width("5-55%", 92.00, 507.45, 20.23, 7.00, 7.00),
            span_with_width("Anno 1983", 140.00, 507.45, 33.86, 7.00, 7.00),
            span_with_width("Praedictum", 185.00, 507.45, 35.40, 7.00, 7.00),
            span_with_width("Nostrud", 44.00, 499.40, 24.51, 7.00, 7.00),
            span_with_width("62%", 92.00, 499.40, 14.01, 7.00, 7.00),
            span_with_width("Anno 2023", 140.00, 499.40, 33.86, 7.00, 7.00),
            span_with_width("Exclusio", 185.00, 499.40, 26.06, 7.00, 7.00),
            span_with_width("Ullamco", 44.00, 491.35, 25.28, 7.00, 7.00),
            span_with_width("33%", 92.00, 491.35, 14.01, 7.00, 7.00),
            span_with_width("Anno 2016", 140.00, 491.35, 33.86, 7.00, 7.00),
            span_with_width("Conflictus", 185.00, 491.35, 30.73, 7.00, 7.00),
            span_with_width("Laboris", 44.00, 483.30, 22.95, 7.00, 7.00),
            span_with_width("17%", 92.00, 483.30, 14.01, 7.00, 7.00),
            span_with_width("Nulla", 140.00, 483.30, 15.95, 7.00, 7.00),
            span_with_width("Nullum", 185.00, 483.30, 21.78, 7.00, 7.00),
            span_with_width("Nisi", 44.00, 475.25, 11.66, 7.00, 7.00),
            span_with_width("91%", 92.00, 475.25, 14.01, 7.00, 7.00),
            span_with_width("Anno 2001", 140.00, 475.25, 33.86, 7.00, 7.00),
            span_with_width("Peior", 185.00, 475.25, 16.34, 7.00, 7.00),
            span_with_width("Aliquip", 44.00, 467.20, 21.01, 7.00, 7.00),
            span_with_width("80.5%", 92.00, 467.20, 19.85, 7.00, 7.00),
            span_with_width("Anno 2019", 140.00, 467.20, 33.86, 7.00, 7.00),
            span_with_width("Melior", 185.00, 467.20, 19.05, 7.00, 7.00),
            span_with_width("Commodo", 44.00, 459.15, 32.28, 7.00, 7.00),
            span_with_width("40-70%", 92.00, 459.15, 24.12, 7.00, 7.00),
            span_with_width("Anno 1983", 140.00, 459.15, 33.86, 7.00, 7.00),
            span_with_width("Praedictum", 185.00, 459.15, 35.40, 7.00, 7.00),
            span_with_width("Duis", 44.00, 451.10, 14.00, 7.00, 7.00),
            span_with_width("10-20%", 92.00, 451.10, 24.12, 7.00, 7.00),
            span_with_width("Anno 2023", 140.00, 451.10, 33.86, 7.00, 7.00),
            span_with_width("Exclusio", 185.00, 451.10, 26.06, 7.00, 7.00),
            span_with_width("Aute", 44.00, 443.05, 14.40, 7.00, 7.00),
            span_with_width("10-15%", 92.00, 443.05, 24.12, 7.00, 7.00),
            span_with_width("Anno 2016", 140.00, 443.05, 33.86, 7.00, 7.00),
            span_with_width("Conflictus", 185.00, 443.05, 30.73, 7.00, 7.00),
            span_with_width("Irure", 44.00, 435.00, 14.39, 7.00, 7.00),
            span_with_width("50-79%", 92.00, 435.00, 24.12, 7.00, 7.00),
            span_with_width("Nulla", 140.00, 435.00, 15.95, 7.00, 7.00),
            span_with_width("Nullum", 185.00, 435.00, 21.78, 7.00, 7.00),
            span_with_width("Velit", 44.00, 426.95, 13.62, 7.00, 7.00),
            span_with_width("5-55%", 92.00, 426.95, 20.23, 7.00, 7.00),
            span_with_width("Anno 2001", 140.00, 426.95, 33.86, 7.00, 7.00),
            span_with_width("Peior", 185.00, 426.95, 16.34, 7.00, 7.00),
            span_with_width("Esse", 44.00, 418.90, 15.56, 7.00, 7.00),
            span_with_width("62%", 92.00, 418.90, 14.01, 7.00, 7.00),
            span_with_width("Anno 2019", 140.00, 418.90, 33.86, 7.00, 7.00),
            span_with_width("Melior", 185.00, 418.90, 19.05, 7.00, 7.00),
            span_with_width("Cillum", 44.00, 410.85, 19.44, 7.00, 7.00),
            span_with_width("33%", 92.00, 410.85, 14.01, 7.00, 7.00),
            span_with_width("Anno 1983", 140.00, 410.85, 33.86, 7.00, 7.00),
            span_with_width("Praedictum", 185.00, 410.85, 35.40, 7.00, 7.00),
            span_with_width("Fugiat", 44.00, 402.80, 19.45, 7.00, 7.00),
            span_with_width("17%", 92.00, 402.80, 14.01, 7.00, 7.00),
            span_with_width("Anno 2023", 140.00, 402.80, 33.86, 7.00, 7.00),
            span_with_width("Exclusio", 185.00, 402.80, 26.06, 7.00, 7.00),
            span_with_width("Nulla", 44.00, 394.75, 15.95, 7.00, 7.00),
            span_with_width("91%", 92.00, 394.75, 14.01, 7.00, 7.00),
            span_with_width("Anno 2016", 140.00, 394.75, 33.86, 7.00, 7.00),
            span_with_width("Conflictus", 185.00, 394.75, 30.73, 7.00, 7.00),
            span_with_width("Sint", 44.00, 386.70, 12.06, 7.00, 7.00),
            span_with_width("80.5%", 92.00, 386.70, 19.85, 7.00, 7.00),
            span_with_width("Nulla", 140.00, 386.70, 15.95, 7.00, 7.00),
            span_with_width("Nullum", 185.00, 386.70, 21.78, 7.00, 7.00),
            span_with_width("Culpa", 44.00, 378.65, 18.28, 7.00, 7.00),
            span_with_width("40-70%", 92.00, 378.65, 24.12, 7.00, 7.00),
            span_with_width("Anno 2001", 140.00, 378.65, 33.86, 7.00, 7.00),
            span_with_width("Peior", 185.00, 378.65, 16.34, 7.00, 7.00),
            span_with_width("Officia", 44.00, 370.60, 19.84, 7.00, 7.00),
            span_with_width("10-20%", 92.00, 370.60, 24.12, 7.00, 7.00),
            span_with_width("Anno 2019", 140.00, 370.60, 33.86, 7.00, 7.00),
            span_with_width("Melior", 185.00, 370.60, 19.05, 7.00, 7.00),
            span_with_width("Mollit", 44.00, 362.55, 16.33, 7.00, 7.00),
            span_with_width("10-15%", 92.00, 362.55, 24.12, 7.00, 7.00),
            span_with_width("Anno 1983", 140.00, 362.55, 33.86, 7.00, 7.00),
            span_with_width("Praedictum", 185.00, 362.55, 35.40, 7.00, 7.00),
            span_with_width("Anim", 44.00, 354.50, 15.95, 7.00, 7.00),
            span_with_width("50-79%", 92.00, 354.50, 24.12, 7.00, 7.00),
            span_with_width("Anno 2023", 140.00, 354.50, 33.86, 7.00, 7.00),
            span_with_width("Exclusio", 185.00, 354.50, 26.06, 7.00, 7.00),
            span_with_width("2.3. Sed do eiusmod tempor", 307.00, 770.00, 119.34, 9.50, 9.50),
            span_with_width("adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore", 307.00, 759.55, 251.34, 8.50, 8.50),
            span_with_width("elit sed do eiusmod tempor incididunt ut labore et dolore magna", 307.00, 749.10, 239.53, 8.50, 8.50),
            span_with_width("sed do eiusmod tempor incididunt ut labore et dolore magna aliqua", 307.00, 738.65, 251.34, 8.50, 8.50),
            span_with_width("do eiusmod tempor incididunt ut labore et dolore magna aliqua", 307.00, 728.20, 235.28, 8.50, 8.50),
            span_with_width("eiusmod tempor incididunt ut labore et dolore magna aliqua enim", 307.00, 717.75, 244.25, 8.50, 8.50),
            span_with_width("tempor incididunt ut labore et dolore magna aliqua enim ad minim", 307.00, 707.30, 246.60, 8.50, 8.50),
            span_with_width("incididunt ut labore et dolore magna aliqua enim ad minim veniam", 307.00, 696.85, 247.55, 8.50, 8.50),
            span_with_width("et dolore magna aliqua enim ad minim veniam quis nostrud", 307.00, 665.50, 222.50, 8.50, 8.50),
            span_with_width("dolore magna aliqua enim ad minim veniam quis nostrud", 307.00, 655.05, 213.05, 8.50, 8.50),
            span_with_width("magna aliqua enim ad minim veniam quis nostrud exercitation", 307.00, 644.60, 232.89, 8.50, 8.50),
            span_with_width("aliqua enim ad minim veniam quis nostrud exercitation ullamco", 307.00, 634.15, 236.19, 8.50, 8.50),
            span_with_width("enim ad minim veniam quis nostrud exercitation ullamco laboris", 307.00, 623.70, 238.54, 8.50, 8.50),
            span_with_width("ad minim veniam quis nostrud exercitation ullamco laboris nisi", 307.00, 613.25, 232.87, 8.50, 8.50),
            span_with_width("quis nostrud exercitation ullamco laboris nisi aliquip ex ea", 307.00, 581.90, 216.36, 8.50, 8.50),
            span_with_width("nostrud exercitation ullamco laboris nisi aliquip ex ea commodo", 307.00, 571.45, 238.08, 8.50, 8.50),
            span_with_width("Cura", 262.00, 563.80, 15.17, 7.00, 7.00),
            span_with_width("exercitation ullamco laboris nisi aliquip ex ea commodo consequat", 307.00, 561.00, 248.96, 8.50, 8.50),
            span_with_width("ignotum", 262.00, 555.75, 24.90, 7.00, 7.00),
            span_with_width("ullamco laboris nisi aliquip ex ea commodo consequat duis aute", 307.00, 550.55, 239.99, 8.50, 8.50),
            span_with_width("ignotum", 262.00, 547.70, 24.90, 7.00, 7.00),
            span_with_width("nullum", 262.00, 539.65, 20.62, 7.00, 7.00),
            span_with_width("laboris nisi aliquip ex ea commodo consequat duis aute irure in", 307.00, 540.10, 236.68, 8.50, 8.50),
            span_with_width("ignotum", 262.00, 531.60, 24.90, 7.00, 7.00),
            span_with_width("nisi aliquip ex ea commodo consequat duis aute irure in", 307.00, 529.65, 209.29, 8.50, 8.50),
            span_with_width("peius", 262.00, 523.55, 16.73, 7.00, 7.00),
            span_with_width("aliquip ex ea commodo consequat duis aute irure in reprehenderit", 307.00, 519.20, 247.09, 8.50, 8.50),
            span_with_width("ignotum", 262.00, 515.50, 24.90, 7.00, 7.00),
            span_with_width("ex ea commodo consequat duis aute irure in reprehenderit", 307.00, 508.75, 220.16, 8.50, 8.50),
            span_with_width("melius", 262.00, 507.45, 20.22, 7.00, 7.00),
            span_with_width("ignotum", 262.00, 499.40, 24.90, 7.00, 7.00),
            span_with_width("ea commodo consequat duis aute irure in reprehenderit voluptate", 307.00, 498.30, 245.68, 8.50, 8.50),
            span_with_width("ignotum", 262.00, 491.35, 24.90, 7.00, 7.00),
            span_with_width("nullum", 262.00, 483.30, 20.62, 7.00, 7.00),
            span_with_width("2.4. Ut labore et dolore magna", 307.00, 477.40, 128.32, 9.50, 9.50),
            span_with_width("ignotum", 262.00, 475.25, 24.90, 7.00, 7.00),
            span_with_width("peius", 262.00, 467.20, 16.73, 7.00, 7.00),
            span_with_width("excepteur sint occaecat cupidatat non proident sunt culpa qui", 307.00, 466.95, 230.57, 8.50, 8.50),
            span_with_width("ignotum", 262.00, 459.15, 24.90, 7.00, 7.00),
            span_with_width("sint occaecat cupidatat non proident sunt culpa qui officia deserunt", 307.00, 456.50, 250.89, 8.50, 8.50),
            span_with_width("melius", 262.00, 451.10, 20.22, 7.00, 7.00),
            span_with_width("occaecat cupidatat non proident sunt culpa qui officia deserunt", 307.00, 446.05, 235.30, 8.50, 8.50),
            span_with_width("ignotum", 262.00, 443.05, 24.90, 7.00, 7.00),
            span_with_width("ignotum", 262.00, 435.00, 24.90, 7.00, 7.00),
            span_with_width("cupidatat non proident sunt culpa qui officia deserunt mollit anim id", 307.00, 435.60, 250.87, 8.50, 8.50),
            span_with_width("nullum", 262.00, 426.95, 20.62, 7.00, 7.00),
            span_with_width("non proident sunt culpa qui officia deserunt mollit anim id est", 307.00, 425.15, 227.72, 8.50, 8.50),
            span_with_width("ignotum", 262.00, 418.90, 24.90, 7.00, 7.00),
            span_with_width("proident sunt culpa qui officia deserunt mollit anim id est laborum", 307.00, 414.70, 244.24, 8.50, 8.50),
            span_with_width("peius", 262.00, 410.85, 16.73, 7.00, 7.00),
            span_with_width("sunt culpa qui officia deserunt mollit anim id est laborum lorem", 307.00, 404.25, 234.78, 8.50, 8.50),
            span_with_width("ignotum", 262.00, 402.80, 24.90, 7.00, 7.00),
            span_with_width("melius", 262.00, 394.75, 20.22, 7.00, 7.00),
            span_with_width("culpa qui officia deserunt mollit anim id est laborum lorem ipsum", 307.00, 393.80, 241.38, 8.50, 8.50),
            span_with_width("ignotum", 262.00, 386.70, 24.90, 7.00, 7.00),
            span_with_width("qui officia deserunt mollit anim id est laborum lorem ipsum dolor sit", 307.00, 383.35, 250.83, 8.50, 8.50),
            span_with_width("ignotum", 262.00, 378.65, 24.90, 7.00, 7.00),
            span_with_width("nullum", 262.00, 370.60, 20.62, 7.00, 7.00),
            span_with_width("officia deserunt mollit anim id est laborum lorem ipsum dolor sit", 307.00, 372.90, 237.12, 8.50, 8.50),
            span_with_width("ignotum", 262.00, 362.55, 24.90, 7.00, 7.00),
            span_with_width("deserunt mollit anim id est laborum lorem ipsum dolor sit amet", 307.00, 362.45, 233.82, 8.50, 8.50),
            span_with_width("peius", 262.00, 354.50, 16.73, 7.00, 7.00),
            span_with_width("mollit anim id est laborum lorem ipsum dolor sit amet consectetur", 307.00, 352.00, 244.68, 8.50, 8.50),
            span_with_width("anim id est laborum lorem ipsum dolor sit amet consectetur", 307.00, 341.55, 222.49, 8.50, 8.50),
            span_with_width("id est laborum lorem ipsum dolor sit amet consectetur adipiscing", 307.00, 331.10, 241.86, 8.50, 8.50),
            span_with_width("est laborum lorem ipsum dolor sit amet consectetur adipiscing elit", 307.00, 320.65, 246.11, 8.50, 8.50),
            span_with_width("laborum lorem ipsum dolor sit amet consectetur adipiscing elit sed", 307.00, 310.20, 248.47, 8.50, 8.50),
            span_with_width("lorem ipsum dolor sit amet consectetur adipiscing elit sed do", 307.00, 299.75, 227.22, 8.50, 8.50),
            span_with_width("ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod", 307.00, 289.30, 238.09, 8.50, 8.50),
            span_with_width("dolor sit amet consectetur adipiscing elit sed do eiusmod tempor", 307.00, 278.85, 241.88, 8.50, 8.50),
            span_with_width("sit amet consectetur adipiscing elit sed do eiusmod tempor", 307.00, 268.40, 220.62, 8.50, 8.50),
            span_with_width("amet consectetur adipiscing elit sed do eiusmod tempor incididunt", 307.00, 257.95, 248.02, 8.50, 8.50),
            span_with_width("consectetur adipiscing elit sed do eiusmod tempor incididunt ut", 307.00, 247.50, 236.21, 8.50, 8.50),
            span_with_width("adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore", 307.00, 237.05, 251.34, 8.50, 8.50),
            span_with_width("elit sed do eiusmod tempor incididunt ut labore et dolore magna", 307.00, 226.60, 239.53, 8.50, 8.50),
            span_with_width("sed do eiusmod tempor incididunt ut labore et dolore magna aliqua", 307.00, 216.15, 251.34, 8.50, 8.50),
            span_with_width("do eiusmod tempor incididunt ut labore et dolore magna aliqua", 307.00, 205.70, 235.28, 8.50, 8.50),
            span_with_width("eiusmod tempor incididunt ut labore et dolore magna aliqua enim", 307.00, 195.25, 244.25, 8.50, 8.50),
            span_with_width("tempor incididunt ut labore et dolore magna aliqua enim ad minim", 307.00, 184.80, 246.60, 8.50, 8.50),
            span_with_width("incididunt ut labore et dolore magna aliqua enim ad minim veniam", 307.00, 174.35, 247.55, 8.50, 8.50),
            span_with_width("1", 295.42, 30.00, 4.45, 8.00, 8.00),
        ];
        spans
    }

    /// GH#1742: the fifth table column's narrow cells (`x=262`, right edge `286.90`)
    /// must not be read as a hanging-label stack -- nothing follows any of them on
    /// their own line past the corridor, unlike a real clause number.
    #[test]
    fn corridor_is_hanging_label_indent_rejects_a_table_edge_column_gh1742() {
        let spans = gh1742_p1_real_reproducer_spans();
        let order = spans_sorted_top_to_bottom(&spans);
        let lines = group_into_lines(&spans, &order);
        let page_width = GH1742_P1_PAGE_WIDTH;
        let max_label_width = page_width * MAX_DENSE_COLUMN_SPLIT_SNAP_SPAN_FRACTION;
        // The corridor the widened search actually finds on this page: the true
        // gutter, narrowed by the centred page number to a single crossing line. ~keep
        let corridor = (286.899, 307.0);

        assert!(
            !corridor_is_hanging_label_indent(&spans, &lines, max_label_width, corridor),
            "a table's edge column must not disqualify the page's real gutter as a \
             hanging-label indent"
        );
    }

    /// GH#1742 end to end (reproducer page 1, verbatim geometry): the page must
    /// reorder column-major -- the whole left column (prose, then the five-column
    /// table) followed by the whole right column, with the centred page number left
    /// as its own trailing boundary line. Before fix #2,
    /// `reorder_dense_two_column_page` leaves the median inside the table (`~240`),
    /// every straddling prose line becomes a single-line boundary band, and the page
    /// comes out with both columns interleaved -- the reported `2.1 … 2.3` weld.
    #[test]
    fn dense_two_column_page_reorders_by_column_on_real_reproducer_page_one_gh1742() {
        let mut spans = gh1742_p1_real_reproducer_spans();
        let original_order: Vec<String> = spans.iter().map(|span| span.text.clone()).collect();

        assert!(
            reorder_dense_two_column_page(&mut spans, GH1742_P1_PAGE_WIDTH),
            "a two-column page with a five-column table in the lower half of one \
             column and a page number in the gutter must still be reordered"
        );

        let reordered: Vec<String> = spans.iter().map(|span| span.text.clone()).collect();
        assert_ne!(
            reordered, original_order,
            "the page must not be left in its original interleaved order"
        );

        // The left column's first heading must now be immediately followed by the
        // left column's own next paragraph, not by the right column's `2.3` heading
        // (the reported weld) or by any table cell.
        let heading_index = reordered
            .iter()
            .position(|text| text == "2.1. Lorem ipsum dolor sit amet")
            .expect("left column's first heading must survive the reorder");
        assert_eq!(
            reordered[heading_index + 1],
            "incididunt ut labore et dolore magna aliqua enim ad minim veniam",
            "the left column's heading must be followed by its own paragraph, not by \
             the right column's heading or a table cell"
        );

        // The whole left column (ending in the table's last row) must precede the
        // whole right column (starting with its own heading `2.3.`).
        let last_table_cell_index = reordered
            .iter()
            .position(|text| text == "Anim")
            .expect("the table's last row must survive the reorder");
        let right_heading_index = reordered
            .iter()
            .position(|text| text == "2.3. Sed do eiusmod tempor")
            .expect("the right column's heading must survive the reorder");
        assert!(
            last_table_cell_index < right_heading_index,
            "the left column, table included, must be emitted before the right column"
        );
    }

    const GH1756_PAGE_WIDTH: f32 = 595.3;
    /// The reporter's own whitespace corridor between the two columns: the left
    /// column's right edge and the right column's left edge, midpoint 298.8.
    const GH1756_GUTTER_CORRIDOR: (f32, f32) = (291.0, 306.6);
    /// The column grid the carrier's full table rows establish (issue #1756's own
    /// measurement: 312.6 / 397.2 / 441.0 / 488.4 / 534.7), as `(left, width)`. Every
    /// consecutive pair leaves a gap wider than `min_gutter` (11.9pt at this page
    /// width), so a full row opens four internal gaps and `line_has_grid_row_gaps`
    /// already excludes it from the vote on both builds.
    const GH1756_GRID_ROW: [(f32, f32); 5] = [
        (312.6, 30.0),
        (397.2, 20.0),
        (441.0, 20.0),
        (488.4, 21.1),
        (534.7, 22.0),
    ];
    /// One full row above the wrapped column header, then 26 below it -- the 27 grid
    /// rows the issue counts, covering y 325.0..700.3, the extent all six sparse lines
    /// fall inside.
    const GH1756_TOP_GRID_ROW_Y: f32 = 700.3;
    const GH1756_FIRST_BODY_GRID_ROW_Y: f32 = 675.0;
    const GH1756_BODY_GRID_ROW_COUNT: usize = 26;
    const GH1756_GRID_ROW_LEADING: f32 = 14.0;

    /// The six sparse table lines of the reporter's page 1, as `(y, cells)` with each
    /// cell `(left, width)`. Each reproduces the three facts the issue measures for
    /// that line -- its internal gap count (one to three, always under
    /// `MIN_GRID_ROW_GAP_COUNT`, so it keeps its vote), the midpoint of its widest gap,
    /// and how many of its spans sit on the grid above -- rather than the carrier's
    /// literal text:
    ///
    /// | y | issue: gaps / midpoint / aligned | fixture: gaps / midpoint / aligned |
    /// |---|---|---|
    /// | 691.7 | 1 / 482.1 / 2 of 2 | 1 / 482.1 / 2 of 2 |
    /// | 683.1 | 1 / 471.7 / 2 of 4 | 1 / 471.7 / 2 of 2 |
    /// | 652.9 | 1 / 449.1 / 2 of 2 | 1 / 449.1 / 2 of 2 |
    /// | 627.2 | 3 / 522.1 / 5 of 7 | 3 / 522.1 / 5 of 7 |
    /// | 601.5 | 3 / 465.7 / 5 of 5 | 3 / 464.7 / 5 of 5 |
    /// | 327.3 | 2 / 449.1 / 2 of 3 | 2 / 449.1 / 2 of 3 |
    ///
    /// The y-601.5 midpoint is 464.7 rather than the issue's 465.7: that line's widest
    /// gap runs between two zero-width empty-cell markers, which can only sit on the
    /// grid's own column edges, so the midpoint is pinned to (441.0 + 488.4) / 2. The
    /// 1.0pt difference does not move the median, which is the 449.1 six places in.
    const GH1756_SPARSE_TABLE_LINES: [(f32, &[(f32, f32)]); 6] = [
        (691.7, &[(441.0, 34.8), (488.4, 34.0)]),
        (683.1, &[(441.0, 14.0), (488.4, 22.0)]),
        (652.9, &[(397.2, 12.6), (488.4, 20.0)]),
        (
            627.2,
            &[
                (312.6, 20.0),
                (334.0, 6.0),
                (341.0, 45.0),
                (397.2, 25.0),
                (441.0, 28.0),
                (488.4, 21.1),
                (534.7, 22.0),
            ],
        ),
        (
            601.5,
            &[(312.6, 77.4), (397.2, 0.0), (441.0, 0.0), (488.4, 0.0), (534.7, 22.0)],
        ),
        (327.3, &[(320.0, 30.0), (397.2, 12.6), (488.4, 20.0)]),
    ];
    /// The reporter's page 3 removes the four sparse lines whose widest gap lies
    /// furthest right (y 691.7, 683.1, 627.2, 601.5), leaving these two.
    const GH1756_P3_SPARSE_TABLE_LINES: [usize; 2] = [2, 5];

    /// The five ordinary prose lines that keep their vote on every build, contributing
    /// the issue's measured midpoints 255.0, 276.0, 298.8, 304.8 and 365.0 in that
    /// order. The first two sit inside the table's y extent and off its column grid, so
    /// they are the fixture's own control that the grid exclusion discriminates by
    /// alignment and not merely by y.
    const GH1756_PROSE_VOTE_LINES: [(f32, &[(f32, f32)]); 5] = [
        (668.0, &[(37.6, 211.4), (261.0, 30.0)]),
        (640.0, &[(37.6, 232.4), (282.0, 9.0)]),
        (760.0, &[(37.6, 253.4), (306.6, 235.0)]),
        (746.0, &[(37.6, 253.4), (318.6, 223.0)]),
        (720.0, &[(306.6, 46.4), (377.0, 60.0)]),
    ];

    /// The band below the table that makes the wrong split *act*: the issue's own
    /// reordering band (its y 418.7..472.9, left 33 inked `Prose` spans, right 6
    /// `Mixed`, row pairing 0.23). Interleaved left-only prose lines and right-only
    /// cells, neither of which straddles the bad split at 449.1, so the whole run stays
    /// one content band with six inked spans a side and no row pairing across it --
    /// `reorder_band_columns` accepts it, `reorder_dense_two_column_page` returns
    /// `true`, and the page is emitted in y order. None of these lines has an internal
    /// gap of its own, so the band adds no vote on either build.
    const GH1756_REORDERING_BAND_TOP_Y: f32 = 297.0;
    const GH1756_REORDERING_BAND_LEADING: f32 = 7.0;
    const GH1756_REORDERING_BAND_LINES: usize = 2 * MIN_DENSE_COLUMN_SPANS_PER_SIDE;
    const GH1756_BAND_PROSE_TEXT: &str = "left column body text continuing past the table";
    const GH1756_BAND_PROSE_X: f32 = 37.6;
    const GH1756_BAND_PROSE_WIDTH: f32 = 200.0;
    const GH1756_BAND_CELL_X: f32 = 488.4;
    const GH1756_BAND_CELL_WIDTH: f32 = 60.0;

    fn gh1756_push_line(spans: &mut Vec<TextSpan>, y: f32, cells: &[(f32, f32)]) {
        for &(x, width) in cells {
            // The carrier marks an empty table cell with a zero-width U+200B, which
            // `trim()` does not remove -- so it counts as ink and holds its column's
            // position, which is why the y-601.5 group row aligns 5 of 5. ~keep
            let text = if width == 0.0 { "\u{200b}" } else { "cell" };
            spans.push(span_with_width(text, x, y, width, 7.0, 7.0));
        }
    }

    fn gh1756_push_grid_rows(spans: &mut Vec<TextSpan>) {
        gh1756_push_line(spans, GH1756_TOP_GRID_ROW_Y, &GH1756_GRID_ROW);
        for row in 0..GH1756_BODY_GRID_ROW_COUNT {
            let y = GH1756_FIRST_BODY_GRID_ROW_Y - row as f32 * GH1756_GRID_ROW_LEADING;
            gh1756_push_line(spans, y, &GH1756_GRID_ROW);
        }
    }

    fn gh1756_push_reordering_band(spans: &mut Vec<TextSpan>) {
        for line in 0..GH1756_REORDERING_BAND_LINES {
            let y = GH1756_REORDERING_BAND_TOP_Y - line as f32 * GH1756_REORDERING_BAND_LEADING;
            if line.is_multiple_of(2) {
                spans.push(span_with_width(
                    GH1756_BAND_PROSE_TEXT,
                    GH1756_BAND_PROSE_X,
                    y,
                    GH1756_BAND_PROSE_WIDTH,
                    8.5,
                    8.5,
                ));
            } else {
                spans.push(span_with_width(
                    "cell",
                    GH1756_BAND_CELL_X,
                    y,
                    GH1756_BAND_CELL_WIDTH,
                    7.0,
                    7.0,
                ));
            }
        }
    }

    fn gh1756_page_with_sparse_lines(sparse_lines: &[usize]) -> Vec<TextSpan> {
        let mut spans = Vec::new();
        gh1756_push_grid_rows(&mut spans);
        for &line in sparse_lines {
            let (y, cells) = GH1756_SPARSE_TABLE_LINES[line];
            gh1756_push_line(&mut spans, y, cells);
        }
        for &(y, cells) in &GH1756_PROSE_VOTE_LINES {
            gh1756_push_line(&mut spans, y, cells);
        }
        gh1756_push_reordering_band(&mut spans);
        spans
    }

    /// GH#1756 (the reporter's page 1): a journal table whose 27 full rows are already
    /// excluded from `detect_split_x`'s vote, plus the six *sparse* table lines that
    /// are not -- a wrapped column header's second and third line, an SD line under its
    /// row, a row whose long label closes the gap to its first value, a group row with
    /// empty cells, and a units line. Five ordinary prose lines are the only genuine
    /// gutter evidence left. On v1.2.7 the six sparse lines outvote them six to five
    /// and the median lands at 449.1, deep inside the right column and 150.3pt from the
    /// true corridor -- 1.5pt past what `MAX_REDIRECT_DISTANCE_FRACTION` (148.8pt at
    /// this page width) lets the redirect reach back.
    fn gh1756_sparse_table_rows_page() -> Vec<TextSpan> {
        gh1756_page_with_sparse_lines(&[0, 1, 2, 3, 4, 5])
    }

    /// GH#1756 (the reporter's page 3): the same page with four of the six sparse lines
    /// removed.
    fn gh1756_two_sparse_table_rows_page() -> Vec<TextSpan> {
        gh1756_page_with_sparse_lines(&GH1756_P3_SPARSE_TABLE_LINES)
    }

    /// GH#1756's control page: the same 27-row table with no sparse lines at all, and
    /// six genuine two-column prose lines interleaved with the table's own rows.
    /// Excluding a table's sparse rows must not cost the page the gutter evidence that
    /// merely sits beside the table.
    fn gh1756_prose_beside_a_full_grid_table() -> Vec<TextSpan> {
        let mut spans = Vec::new();
        gh1756_push_grid_rows(&mut spans);
        for line in 0..MIN_DENSE_COLUMN_SPLIT_LINES {
            let y = 668.0 - line as f32 * GH1756_GRID_ROW_LEADING;
            gh1756_push_line(&mut spans, y, &[(37.6, 253.4), (306.6, 235.0)]);
        }
        spans
    }

    fn gh1756_detect_split_x(spans: &[TextSpan]) -> Option<f32> {
        let order = spans_sorted_top_to_bottom(spans);
        let lines = group_into_lines(spans, &order);
        detect_split_x(spans, &lines, GH1756_PAGE_WIDTH)
    }

    /// GH#1756: with the table's full rows already out of the vote, its six sparse
    /// lines are the majority of what is left and carry the median to 449.1 -- inside
    /// the right column, and further from the gutter than the redirect may reach. The
    /// sparse lines sit on the very column grid the excluded full rows establish, so
    /// excluding them too leaves the five prose votes, one short of the
    /// `MIN_DENSE_COLUMN_SPLIT_LINES` quorum: the repair declines and the page falls to
    /// the XY-cut, which reads it in column order (the reporter measures exactly that
    /// on their page 2).
    #[test]
    fn detect_split_x_declines_when_sparse_table_rows_outvote_the_gutter_gh1756() {
        let spans = gh1756_sparse_table_rows_page();

        assert_eq!(
            gh1756_detect_split_x(&spans),
            None,
            "the table's sparse lines must not be gutter evidence: with only the five \
             genuine prose votes left the vote is below quorum and must decline rather \
             than place the split at 449.1, inside the right column"
        );
    }

    /// GH#1756 adversarial review: `apply_xy_cut_if_column_aware` (hierarchy.rs) feeds
    /// the XY-cut's heading-run pre-pass a gutter from `detect_column_gutter`, a
    /// detector with no table-grid exclusion (`vote_is_a_grid_cell_gap` is
    /// `detect_split_x`-only). The failure this was suspected of reaching would be
    /// `detect_column_gutter` returning a corridor INSIDE the table grid
    /// (`GH1756_GRID_ROW` spans x 312.6..556.7) on the same sparse-table-row page
    /// `detect_split_x` must decline on. Measured directly: it returns `None` on both
    /// of #1756's fixtures, because `prose_two_column_gutter`'s own column-count
    /// clustering step requires exactly 2 significant left-edge clusters and the
    /// table's five-column grid produces 5, so every detector in the `or_else` chain
    /// declines independently of any grid-gap exclusion. The finding's mechanism does
    /// not reach this page. ~keep
    #[test]
    fn detect_column_gutter_also_declines_on_the_sparse_table_rows_pages_gh1756() {
        let sparse = gh1756_sparse_table_rows_page();
        let two_sparse = gh1756_two_sparse_table_rows_page();

        assert_eq!(
            xberg_native_pdf::pipeline::reading_order::detect_column_gutter(&sparse),
            None,
            "detect_column_gutter must not place a gutter inside the table's own grid"
        );
        assert_eq!(
            xberg_native_pdf::pipeline::reading_order::detect_column_gutter(&two_sparse),
            None,
            "detect_column_gutter must not place a gutter inside the table's own grid"
        );
    }

    /// GH#1756 (the reporter's page 3): two sparse lines instead of six. On v1.2.7 the
    /// median lands at 304.8, inside the gutter corridor, by luck of where the two
    /// survivors' gaps fall -- the page reads correctly but for the wrong reason. Both
    /// survivors are still table lines on the grid, so the repair declines here too and
    /// the page reaches the same correct column order through the XY-cut.
    #[test]
    fn detect_split_x_declines_on_two_sparse_table_rows_gh1756() {
        let spans = gh1756_two_sparse_table_rows_page();

        assert_eq!(
            gh1756_detect_split_x(&spans),
            None,
            "both surviving sparse lines sit on the table's own column grid, so the \
             five prose votes are all that is left and the vote is below quorum"
        );
    }

    fn gh1756_grid_columns(spans: &[TextSpan]) -> Vec<f32> {
        let order = spans_sorted_top_to_bottom(spans);
        let lines = group_into_lines(spans, &order);
        let min_gutter = (GH1756_PAGE_WIDTH * MIN_DENSE_COLUMN_GUTTER_FRACTION).max(MIN_DENSE_COLUMN_GUTTER_PTS);
        let furniture_width = GH1756_PAGE_WIDTH * FULL_WIDTH_FURNITURE_FRACTION;
        let grid_rows: Vec<SpanLine> = lines
            .iter()
            .filter(|&line| !line_has_width_furniture(spans, line, furniture_width))
            .filter(|&line| line.iter().any(|&index| span_has_ink(&spans[index])))
            .filter(|&line| line_has_grid_row_gaps(spans, line, min_gutter))
            .cloned()
            .collect();
        strong_column_edges(spans, &grid_rows)
    }

    fn gh1756_line_at(spans: &[TextSpan], y: f32) -> SpanLine {
        let order = spans_sorted_top_to_bottom(spans);
        group_into_lines(spans, &order)
            .into_iter()
            .find(|line| (spans[line[0]].bbox.y - y).abs() < LINE_Y_TOLERANCE_PTS)
            .expect("the fixture must carry a line on this baseline")
    }

    /// GH#1756: the predicate's two halves discriminate in opposite directions, so each
    /// needs its own control. A sparse table line is on the grid *and* votes for a gap
    /// between two of its columns. A prose line whose own gap happens to fall across
    /// the table's columns is not on the grid, and keeps its vote. A prose line in the
    /// real gutter is neither.
    #[test]
    fn only_a_grid_row_voting_for_its_own_cell_gap_is_excluded_gh1756() {
        let spans = gh1756_sparse_table_rows_page();
        let columns = gh1756_grid_columns(&spans);

        assert_eq!(
            columns,
            GH1756_GRID_ROW.iter().map(|&(x, _)| x).collect::<Vec<_>>(),
            "the full rows the gap-count filter already excluded must establish the \
             table's own column grid"
        );
        assert!(
            vote_is_a_grid_cell_gap(&spans, &gh1756_line_at(&spans, 652.9), &columns, 449.1),
            "the SD line under `Age, y` sits on the grid and votes for the gap between \
             the table's own second and fourth columns"
        );
        assert!(
            !vote_is_a_grid_cell_gap(&spans, &gh1756_line_at(&spans, 720.0), &columns, 365.0),
            "a prose line whose gap falls across the table's columns, but whose spans \
             start nowhere near their edges, must keep its vote"
        );
        assert!(
            !vote_is_a_grid_cell_gap(&spans, &gh1756_line_at(&spans, 760.0), &columns, 298.8),
            "a line voting for the page's real gutter, left of the table entirely, must \
             keep its vote"
        );
    }

    /// GH#1756's end-to-end control: six genuine two-column prose lines whose baselines
    /// interleave a 27-row table's own rows keep their votes and carry the median to
    /// the true gutter. Without this the exclusion could pass its own tests by
    /// swallowing every line level with a table.
    #[test]
    fn detect_split_x_keeps_prose_votes_beside_a_table_gh1756() {
        let spans = gh1756_prose_beside_a_full_grid_table();

        let detected = gh1756_detect_split_x(&spans).expect("six paired prose lines meet the quorum on their own");
        assert!(
            detected > GH1756_GUTTER_CORRIDOR.0 && detected < GH1756_GUTTER_CORRIDOR.1,
            "prose beside a table must keep its gutter vote; expected a split inside \
             {GH1756_GUTTER_CORRIDOR:?}, got {detected}"
        );
    }

    /// GH#1756 end to end: with the vote below quorum `reorder_dense_two_column_page`
    /// declines and leaves every span exactly where it was, handing the page to the
    /// XY-cut fallback. On v1.2.7 it instead returns `true` on a split of 449.1 and the
    /// page is emitted in y order -- the left column's `2.6.` heading welded to the
    /// right column's `Table 4`, and prose from the two columns interleaved.
    #[test]
    fn dense_two_column_page_with_sparse_table_rows_declines_the_repair_gh1756() {
        let mut spans = gh1756_sparse_table_rows_page();
        let original: Vec<(String, f32, f32)> = spans
            .iter()
            .map(|span| (span.text.clone(), span.bbox.x, span.bbox.y))
            .collect();

        assert!(
            !reorder_dense_two_column_page(&mut spans, GH1756_PAGE_WIDTH),
            "a page whose only remaining gutter evidence is five prose lines must \
             decline the dense two-column repair rather than split inside a column"
        );

        let after: Vec<(String, f32, f32)> = spans
            .iter()
            .map(|span| (span.text.clone(), span.bbox.x, span.bbox.y))
            .collect();
        assert_eq!(after, original, "a declined repair must leave the page untouched");
    }

    const GH1762_PAGE_WIDTH: f32 = 595.28;
    /// The page's real gutter: the left column's widest line ends at 288.16, the right
    /// column starts at 307.0 -- the whitespace corridor pages 2 and 3 of the reporter's
    /// own reproducer find, and redirect into, at 297.6.
    const GH1762_GUTTER_CORRIDOR: (f32, f32) = (288.15894, 307.0);
    /// The baseline of Table 2's last row: everything above this belongs to the
    /// full-width table's own band.
    const GH1762_TABLE_2_BOTTOM_Y: f32 = 644.15;

    /// GH#1762, the reporter's reproducer page 1, transcribed from the attached PDF:
    /// one `span_with_width` per span `PdfDocument::extract_spans` returns, x / y /
    /// width / height / font size unmodified. The one edit is the left table's first
    /// header cell, whose Latin the repository's spell check reads as a misspelled
    /// English word; the replacement keeps the cell's advance width and its length.
    ///
    /// A full-width `Table 2` occupies the top of the page, its third column running
    /// x 205..~390 straight across the page's gutter. Below it the page is two columns:
    /// the left is `Table 3`, four columns (a description at x 44 and three numeric
    /// columns at 190 / 228 / 262) on its own 8.05pt leading; the right is prose on the
    /// body's 10.45pt leading, opening with `4.1. Lorem ipsum dolor sit amet`.
    ///
    /// Measured on this fixture before the fix, matching the issue's own trace:
    /// `detect_split_x` returns 216.784 -- between Table 3's second and third numeric
    /// columns, because the table's rows never share a line with the right column
    /// (different leading) so every vote is one of the table's own cell gaps, and a
    /// four-column row opens three internal gaps, one short of
    /// `MIN_GRID_ROW_GAP_COUNT`. `redirect_split_out_of_content` then finds
    /// `cuts_a_span`, 16 lines with the split inside their own cell gap, and *no*
    /// corridor at all -- neither empty nor low-occupancy -- because every row of
    /// Table 2 is written across the gutter. The split stays at 216.784 and the band
    /// below Table 2 is reordered around it, welding the table's numbers to the right
    /// column's prose.
    fn gh1762_p1_real_reproducer_spans() -> Vec<TextSpan> {
        #[rustfmt::skip]
        let spans = vec![
            span_with_width("Table 2", 38.00, 795.00, 23.34, 7.00, 7.00),
            span_with_width("consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua enim ad minim veniam quis nostrud exercitation ullamco laboris nisi", 38.00, 786.95, 499.92, 7.00, 7.00),
            span_with_width("Comparatio", 44.00, 776.90, 36.18, 7.00, 7.00),
            span_with_width("Numerus", 120.00, 776.90, 28.39, 7.00, 7.00),
            span_with_width("Summa superior", 205.00, 776.90, 51.35, 7.00, 7.00),
            span_with_width("Summa inferior", 400.00, 776.90, 47.45, 7.00, 7.00),
            span_with_width("Lorem vs. ipsum", 44.00, 768.85, 51.35, 7.00, 7.00),
            span_with_width("80", 120.00, 768.85, 7.78, 7.00, 7.00),
            span_with_width("lorem ipsum dolor sit amet consectetur adipiscing elit sed", 205.00, 768.85, 177.39, 7.00, 7.00),
            span_with_width("lorem ipsum dolor sit amet", 400.00, 768.85, 82.07, 7.00, 7.00),
            span_with_width("ipsum dolor sit amet consectetur adipiscing elit sed do", 205.00, 760.80, 167.68, 7.00, 7.00),
            span_with_width("ipsum dolor sit amet consectetur", 400.00, 760.80, 100.76, 7.00, 7.00),
            span_with_width("dolor sit amet consectetur adipiscing elit sed do eiusmod", 205.00, 752.75, 175.46, 7.00, 7.00),
            span_with_width("dolor sit amet consectetur", 400.00, 752.75, 80.14, 7.00, 7.00),
            span_with_width("Lorem vs. ipsum", 44.00, 741.70, 51.35, 7.00, 7.00),
            span_with_width("69", 120.00, 741.70, 7.78, 7.00, 7.00),
            span_with_width("elit sed do eiusmod tempor incididunt ut labore et dolore", 205.00, 741.70, 173.91, 7.00, 7.00),
            span_with_width("do eiusmod tempor incididunt ut", 400.00, 741.70, 99.21, 7.00, 7.00),
            span_with_width("sed do eiusmod tempor incididunt ut labore et dolore", 205.00, 733.65, 163.02, 7.00, 7.00),
            span_with_width("eiusmod tempor incididunt ut", 400.00, 733.65, 89.48, 7.00, 7.00),
            span_with_width("do eiusmod tempor incididunt ut labore et dolore magna", 205.00, 725.60, 173.14, 7.00, 7.00),
            span_with_width("tempor incididunt ut labore et", 400.00, 725.60, 90.26, 7.00, 7.00),
            span_with_width("Lorem vs. ipsum", 44.00, 714.55, 51.35, 7.00, 7.00),
            span_with_width("58", 120.00, 714.55, 7.78, 7.00, 7.00),
            span_with_width("labore et dolore magna aliqua enim ad minim veniam quis", 205.00, 714.55, 179.35, 7.00, 7.00),
            span_with_width("aliqua enim ad minim veniam quis", 400.00, 714.55, 105.42, 7.00, 7.00),
            span_with_width("et dolore magna aliqua enim ad minim veniam quis nostrud", 205.00, 706.50, 183.24, 7.00, 7.00),
            span_with_width("enim ad minim veniam quis", 400.00, 706.50, 84.80, 7.00, 7.00),
            span_with_width("dolore magna aliqua enim ad minim veniam quis nostrud", 205.00, 698.45, 175.45, 7.00, 7.00),
            span_with_width("ad minim veniam quis nostrud", 400.00, 698.45, 92.97, 7.00, 7.00),
            span_with_width("Lorem vs. ipsum", 44.00, 687.40, 51.35, 7.00, 7.00),
            span_with_width("47", 120.00, 687.40, 7.78, 7.00, 7.00),
            span_with_width("minim veniam quis nostrud exercitation ullamco laboris nisi", 205.00, 687.40, 182.05, 7.00, 7.00),
            span_with_width("laboris nisi aliquip ex ea commodo", 400.00, 687.40, 106.98, 7.00, 7.00),
            span_with_width("veniam quis nostrud exercitation ullamco laboris nisi aliquip", 205.00, 679.35, 183.62, 7.00, 7.00),
            span_with_width("nisi aliquip ex ea commodo", 400.00, 679.35, 84.42, 7.00, 7.00),
            span_with_width("quis nostrud exercitation ullamco laboris nisi aliquip ex ea", 205.00, 671.30, 178.18, 7.00, 7.00),
            span_with_width("aliquip ex ea commodo consequat", 400.00, 671.30, 106.22, 7.00, 7.00),
            span_with_width("Lorem vs. ipsum", 44.00, 660.25, 51.35, 7.00, 7.00),
            span_with_width("36", 120.00, 660.25, 7.78, 7.00, 7.00),
            span_with_width("nisi aliquip ex ea commodo consequat duis aute irure in", 205.00, 660.25, 172.35, 7.00, 7.00),
            span_with_width("irure in reprehenderit voluptate velit", 400.00, 660.25, 109.71, 7.00, 7.00),
            span_with_width("aliquip ex ea commodo consequat duis aute irure in", 205.00, 652.20, 159.91, 7.00, 7.00),
            span_with_width("in reprehenderit voluptate velit", 400.00, 652.20, 93.76, 7.00, 7.00),
            span_with_width("ex ea commodo consequat duis aute irure in reprehenderit", 205.00, 644.15, 181.31, 7.00, 7.00),
            span_with_width("reprehenderit voluptate velit esse", 400.00, 644.15, 103.10, 7.00, 7.00),
            span_with_width("Table 3", 38.00, 613.25, 23.34, 7.00, 7.00),
            span_with_width("4.1. Lorem ipsum dolor sit amet", 307.00, 613.25, 133.05, 9.50, 9.50),
            span_with_width("ut labore et dolore magna aliqua enim ad minim veniam quis nostrud exercitation", 38.00, 605.20, 250.16, 7.00, 7.00),
            span_with_width("aliquip ex ea commodo consequat duis aute irure in reprehenderit voluptate velit", 38.00, 597.15, 248.23, 7.00, 7.00),
            span_with_width("veniam quis nostrud exercitation ullamco laboris nisi aliquip ex ea", 307.00, 592.35, 246.12, 8.50, 8.50),
            span_with_width("Designatio", 44.00, 587.10, 31.12, 7.00, 7.00),
            span_with_width("Primus", 190.00, 587.10, 21.78, 7.00, 7.00),
            span_with_width("Secundus Tertius", 228.00, 587.10, 55.39, 7.00, 7.00),
            span_with_width("quis nostrud exercitation ullamco laboris nisi aliquip ex ea", 307.00, 581.90, 216.36, 8.50, 8.50),
            span_with_width("lorem ipsum dolor sit amet consectetur", 44.00, 579.05, 120.20, 7.00, 7.00),
            span_with_width("21.00", 190.00, 579.05, 17.51, 7.00, 7.00),
            span_with_width("20.48", 228.00, 579.05, 17.51, 7.00, 7.00),
            span_with_width("18.23", 262.00, 579.05, 17.51, 7.00, 7.00),
            span_with_width("nostrud exercitation ullamco laboris nisi aliquip ex ea commodo", 307.00, 571.45, 238.08, 8.50, 8.50),
            span_with_width("sit amet consectetur adipiscing elit sed do", 44.00, 571.00, 129.56, 7.00, 7.00),
            span_with_width("18.96", 190.00, 571.00, 17.51, 7.00, 7.00),
            span_with_width("18.32", 228.00, 571.00, 17.51, 7.00, 7.00),
            span_with_width("21.04", 262.00, 571.00, 17.51, 7.00, 7.00),
            span_with_width("adipiscing elit sed do eiusmod tempor", 44.00, 562.95, 117.10, 7.00, 7.00),
            span_with_width("20.85", 190.00, 562.95, 17.51, 7.00, 7.00),
            span_with_width("19.10", 228.00, 562.95, 17.51, 7.00, 7.00),
            span_with_width("6.10", 262.00, 562.95, 13.62, 7.00, 7.00),
            span_with_width("exercitation ullamco laboris nisi aliquip ex ea commodo consequat", 307.00, 561.00, 248.96, 8.50, 8.50),
            span_with_width("do eiusmod tempor incididunt ut labore et", 44.00, 554.90, 128.39, 7.00, 7.00),
            span_with_width("20.48", 190.00, 554.90, 17.51, 7.00, 7.00),
            span_with_width("18.23", 228.00, 554.90, 17.51, 7.00, 7.00),
            span_with_width("20.27", 262.00, 554.90, 17.51, 7.00, 7.00),
            span_with_width("ullamco laboris nisi aliquip ex ea commodo consequat duis aute", 307.00, 550.55, 239.99, 8.50, 8.50),
            span_with_width("incididunt ut labore et dolore magna aliqua", 44.00, 546.85, 131.90, 7.00, 7.00),
            span_with_width("18.32", 190.00, 546.85, 17.51, 7.00, 7.00),
            span_with_width("21.04", 228.00, 546.85, 17.51, 7.00, 7.00),
            span_with_width("19.61", 262.00, 546.85, 17.51, 7.00, 7.00),
            span_with_width("laboris nisi aliquip ex ea commodo consequat duis aute irure in", 307.00, 540.10, 236.68, 8.50, 8.50),
            span_with_width("et dolore magna aliqua enim ad minim", 44.00, 538.80, 118.66, 7.00, 7.00),
            span_with_width("19.10", 190.00, 538.80, 17.51, 7.00, 7.00),
            span_with_width("6.10", 228.00, 538.80, 13.62, 7.00, 7.00),
            span_with_width("19.24", 262.00, 538.80, 17.51, 7.00, 7.00),
            span_with_width("aliqua enim ad minim veniam quis nostrud", 44.00, 530.75, 130.71, 7.00, 7.00),
            span_with_width("18.23", 190.00, 530.75, 17.51, 7.00, 7.00),
            span_with_width("20.27", 228.00, 530.75, 17.51, 7.00, 7.00),
            span_with_width("16.63", 262.00, 530.75, 17.51, 7.00, 7.00),
            span_with_width("nisi aliquip ex ea commodo consequat duis aute irure in", 307.00, 529.65, 209.29, 8.50, 8.50),
            span_with_width("minim veniam quis nostrud exercitation", 44.00, 522.70, 120.98, 7.00, 7.00),
            span_with_width("21.04", 190.00, 522.70, 17.51, 7.00, 7.00),
            span_with_width("19.61", 228.00, 522.70, 17.51, 7.00, 7.00),
            span_with_width("21.42", 262.00, 522.70, 17.51, 7.00, 7.00),
            span_with_width("aliquip ex ea commodo consequat duis aute irure in reprehenderit", 307.00, 519.20, 247.09, 8.50, 8.50),
            span_with_width("nostrud exercitation ullamco laboris nisi", 44.00, 514.65, 122.15, 7.00, 7.00),
            span_with_width("6.10", 190.00, 514.65, 13.62, 7.00, 7.00),
            span_with_width("19.24", 228.00, 514.65, 17.51, 7.00, 7.00),
            span_with_width("5.81", 262.00, 514.65, 13.62, 7.00, 7.00),
            span_with_width("ex ea commodo consequat duis aute irure in reprehenderit", 307.00, 508.75, 220.16, 8.50, 8.50),
            span_with_width("laboris nisi aliquip ex ea commodo", 44.00, 506.60, 106.98, 7.00, 7.00),
            span_with_width("20.27", 190.00, 506.60, 17.51, 7.00, 7.00),
            span_with_width("16.63", 228.00, 506.60, 17.51, 7.00, 7.00),
            span_with_width("21.00", 262.00, 506.60, 17.51, 7.00, 7.00),
            span_with_width("ex ea commodo consequat duis aute irure", 44.00, 498.55, 130.34, 7.00, 7.00),
            span_with_width("19.61", 190.00, 498.55, 17.51, 7.00, 7.00),
            span_with_width("21.42", 228.00, 498.55, 17.51, 7.00, 7.00),
            span_with_width("18.96", 262.00, 498.55, 17.51, 7.00, 7.00),
            span_with_width("ea commodo consequat duis aute irure in reprehenderit voluptate", 307.00, 498.30, 245.68, 8.50, 8.50),
            span_with_width("consequat duis aute irure in reprehenderit", 44.00, 490.50, 129.56, 7.00, 7.00),
            span_with_width("19.24", 190.00, 490.50, 17.51, 7.00, 7.00),
            span_with_width("5.81", 228.00, 490.50, 13.62, 7.00, 7.00),
            span_with_width("20.85", 262.00, 490.50, 17.51, 7.00, 7.00),
            span_with_width("commodo consequat duis aute irure in reprehenderit voluptate velit", 307.00, 487.85, 251.34, 8.50, 8.50),
            span_with_width("irure in reprehenderit voluptate velit esse", 44.00, 482.45, 126.44, 7.00, 7.00),
            span_with_width("16.63", 190.00, 482.45, 17.51, 7.00, 7.00),
            span_with_width("21.00", 228.00, 482.45, 17.51, 7.00, 7.00),
            span_with_width("20.48", 262.00, 482.45, 17.51, 7.00, 7.00),
            span_with_width("consequat duis aute irure in reprehenderit voluptate velit esse", 307.00, 477.40, 231.97, 8.50, 8.50),
            span_with_width("voluptate velit esse cillum fugiat nulla", 44.00, 474.40, 115.16, 7.00, 7.00),
            span_with_width("21.42", 190.00, 474.40, 17.51, 7.00, 7.00),
            span_with_width("18.96", 228.00, 474.40, 17.51, 7.00, 7.00),
            span_with_width("18.32", 262.00, 474.40, 17.51, 7.00, 7.00),
            span_with_width("duis aute irure in reprehenderit voluptate velit esse cillum fugiat", 307.00, 466.95, 237.63, 8.50, 8.50),
            span_with_width("cillum fugiat nulla pariatur excepteur sint", 44.00, 466.35, 124.88, 7.00, 7.00),
            span_with_width("5.81", 190.00, 466.35, 13.62, 7.00, 7.00),
            span_with_width("20.85", 228.00, 466.35, 17.51, 7.00, 7.00),
            span_with_width("19.10", 262.00, 466.35, 17.51, 7.00, 7.00),
            span_with_width("aute irure in reprehenderit voluptate velit esse cillum fugiat nulla", 307.00, 456.50, 239.99, 8.50, 8.50),
            span_with_width("irure in reprehenderit voluptate velit esse cillum fugiat nulla pariatur", 307.00, 446.05, 252.26, 8.50, 8.50),
            span_with_width("in reprehenderit voluptate velit esse cillum fugiat nulla pariatur", 307.00, 435.60, 232.90, 8.50, 8.50),
            span_with_width("reprehenderit voluptate velit esse cillum fugiat nulla pariatur", 307.00, 425.15, 223.92, 8.50, 8.50),
            span_with_width("voluptate velit esse cillum fugiat nulla pariatur excepteur sint", 307.00, 414.70, 226.29, 8.50, 8.50),
            span_with_width("velit esse cillum fugiat nulla pariatur excepteur sint occaecat", 307.00, 404.25, 225.81, 8.50, 8.50),
            span_with_width("esse cillum fugiat nulla pariatur excepteur sint occaecat cupidatat", 307.00, 393.80, 245.19, 8.50, 8.50),
            span_with_width("cillum fugiat nulla pariatur excepteur sint occaecat cupidatat non", 307.00, 383.35, 241.42, 8.50, 8.50),
            span_with_width("fugiat nulla pariatur excepteur sint occaecat cupidatat non proident", 307.00, 372.90, 250.41, 8.50, 8.50),
        ];
        spans
    }

    /// GH#1762 adversarial review: `gh1762_p1_real_reproducer_spans` plus a second
    /// full-width table ("Table 4") below Table 3, in the same column shape as
    /// Table 2 -- a description cell at x 44, a short numeric cell at 120, and a wide
    /// cell at x 205 (width ~177, right edge ~382) that straddles any plausible split
    /// between the two columns exactly the way Table 2's own rows do. `line_is_boundary`
    /// therefore marks every one of Table 4's rows a boundary line, the same way it
    /// already marks Table 2's.
    ///
    /// `band_lines_around_table_gap_split` pushes a boundary line into BOTH the band
    /// above it and the band it opens (`text.rs` band-splitting loop). Before this
    /// table the band under Table 2 was the page's LAST band, so it carried exactly
    /// one boundary line (Table 2's own closing row) and `MAX_GUTTER_CROSSING_LINES`
    /// (1) tolerated it. With Table 4 added, that band becomes a MIDDLE band bounded
    /// by two boundary lines -- Table 2's closing row above and Table 4's first row
    /// below -- both of which carry a cell covering the true gutter, so the per-band
    /// corridor search now sees 2 gutter-crossing lines where it tolerates only 1. ~keep
    fn gh1762_p1_with_second_table_below_spans() -> Vec<TextSpan> {
        let mut spans = gh1762_p1_real_reproducer_spans();
        const TABLE_4_ROWS: [(&str, &str, &str, &str); 6] = [
            (
                "Quartus vs. quintus",
                "25",
                "lorem ipsum dolor sit amet consectetur adipiscing elit sed",
                "lorem ipsum dolor sit amet",
            ),
            (
                "Quartus vs. quintus",
                "31",
                "ipsum dolor sit amet consectetur adipiscing elit sed do",
                "ipsum dolor sit amet consectetur",
            ),
            (
                "Quartus vs. quintus",
                "44",
                "dolor sit amet consectetur adipiscing elit sed do eiusmod",
                "dolor sit amet consectetur",
            ),
            (
                "Quartus vs. quintus",
                "52",
                "elit sed do eiusmod tempor incididunt ut labore et dolore",
                "do eiusmod tempor incididunt ut",
            ),
            (
                "Quartus vs. quintus",
                "67",
                "sed do eiusmod tempor incididunt ut labore et dolore",
                "eiusmod tempor incididunt ut",
            ),
            (
                "Quartus vs. quintus",
                "73",
                "do eiusmod tempor incididunt ut labore et dolore magna",
                "tempor incididunt ut labore et",
            ),
        ];
        spans.push(span_with_width("Table 4", 38.00, 345.00, 23.34, 7.00, 7.00));
        let mut y = 335.95;
        for (label, number, wide_cell, right_cell) in TABLE_4_ROWS {
            spans.push(span_with_width(label, 44.00, y, 51.35, 7.00, 7.00));
            spans.push(span_with_width(number, 120.00, y, 7.78, 7.00, 7.00));
            spans.push(span_with_width(wide_cell, 205.00, y, 177.39, 7.00, 7.00));
            spans.push(span_with_width(right_cell, 400.00, y, 82.07, 7.00, 7.00));
            y -= 8.05;
        }
        spans
    }

    /// GH#1762 adversarial review, primary claim (measured, not reproduced): a
    /// second full-width table below Table 3 was suspected of turning the rescued
    /// band's one gutter-crossing boundary line into two, pushing
    /// `page_low_occupancy_corridors` (tolerance `MAX_GUTTER_CROSSING_LINES` = 1)
    /// over its limit and silently restoring the pre-#1762 defect.
    ///
    /// Measured directly: the band DOES pick up a second boundary line (Table 4's
    /// opening row), but the pre-existing boundary above it is the SAME line the
    /// original fixture's own doc comment already flags as surviving "by accident
    /// of position" -- its one span ends at x=286.22998, the corridor's own left
    /// edge, so it contributes ZERO crossings to the (286.22998, 307.0) interval
    /// (`right > lo` is false at exact equality). Table 4's row is the only
    /// GENUINE crossing there, so the interval's count is 1, not 2, and
    /// `MAX_GUTTER_CROSSING_LINES` still has headroom. The redirect still finds
    /// the true gutter (296.615, inside `GH1762_GUTTER_CORRIDOR`). A second table
    /// positioned so BOTH its own boundary and the pre-existing one genuinely
    /// cross the interval would still exceed the tolerance -- this fixture just
    /// isn't that page, and the finding is not confirmed by it. ~keep
    #[test]
    fn redirect_survives_a_second_table_closing_the_band_from_below_gh1762() {
        let spans = gh1762_p1_with_second_table_below_spans();
        let order = spans_sorted_top_to_bottom(&spans);
        let lines = group_into_lines(&spans, &order);
        // The same incoming split the original (single-table) fixture computes
        // (`the_page_wide_corridor_search_still_finds_nothing_gh1762` pins it to
        // 216.7825): fixed here rather than recomputed via `detect_split_x` on the
        // extended fixture, so this test isolates the band-scoping mechanism
        // `band_lines_around_table_gap_split` is responsible for from any unrelated
        // shift in the page's overall gutter vote that adding a second table's own
        // internal cell gaps might independently cause. ~keep
        let snapped = 216.7825_f32;
        let min_gutter = (GH1762_PAGE_WIDTH * MIN_DENSE_COLUMN_GUTTER_FRACTION).max(MIN_DENSE_COLUMN_GUTTER_PTS);
        let furniture_width = GH1762_PAGE_WIDTH * FULL_WIDTH_FURNITURE_FRACTION;

        let band = band_lines_around_table_gap_split(&spans, &lines, furniture_width, min_gutter, snapped)
            .expect("the band under Table 3's own internal boundary and above Table 4 still carries the quorum");
        let boundary_lines_in_band = band
            .iter()
            .filter(|line| line_is_boundary(&spans, line, furniture_width, snapped))
            .count();
        assert_eq!(
            boundary_lines_in_band, 2,
            "the band must now carry two boundary lines: the pre-existing one above \
             it and Table 4's opening row below it"
        );

        let corridors =
            page_low_occupancy_corridors(&spans, &band, furniture_width, min_gutter, MAX_GUTTER_CROSSING_LINES);
        assert_eq!(
            corridors,
            [(175.9, 190.0), (286.22998, 307.0)],
            "the true gutter corridor must still be found: the pre-existing boundary \
             line ends exactly at 286.22998 and contributes no crossing there, so \
             Table 4's one genuine crossing stays within MAX_GUTTER_CROSSING_LINES"
        );

        let redirected = redirect_split_out_of_content(&spans, &lines, GH1762_PAGE_WIDTH, snapped);
        assert!(
            redirected > GH1762_GUTTER_CORRIDOR.0 && redirected < GH1762_GUTTER_CORRIDOR.1,
            "a second full-width table below Table 3 must not regress the redirect: \
             expected a split inside {GH1762_GUTTER_CORRIDOR:?}, got {redirected}"
        );
    }

    fn gh1762_snapped_split(spans: &[TextSpan]) -> (Vec<SpanLine>, f32) {
        let order = spans_sorted_top_to_bottom(spans);
        let lines = group_into_lines(spans, &order);
        let detected = detect_split_x(spans, &lines, GH1762_PAGE_WIDTH)
            .expect("the left table's own cell gaps meet the quorum on their own");
        let snapped = snap_split_left_of_hanging_labels(spans, &lines, GH1762_PAGE_WIDTH, detected);
        (lines, snapped)
    }

    /// GH#1762: the corridor search is page-wide, but the split it rescues is applied
    /// per band. `build_bands` has already set Table 2 apart -- every one of its rows
    /// is a boundary line, because the split runs through it -- yet the same rows still
    /// close the gutter for the band below, where nothing is written across it at all.
    /// Asking the corridor question of that band alone finds the gutter, exactly as the
    /// whole-page question finds it on the reporter's pages 2 and 3 once Table 2 is
    /// gone or moved clear of it.
    #[test]
    fn redirect_escapes_a_table_when_another_bands_table_closes_the_gutter_gh1762() {
        let spans = gh1762_p1_real_reproducer_spans();
        let (lines, snapped) = gh1762_snapped_split(&spans);

        let redirected = redirect_split_out_of_content(&spans, &lines, GH1762_PAGE_WIDTH, snapped);
        assert!(
            redirected > GH1762_GUTTER_CORRIDOR.0 && redirected < GH1762_GUTTER_CORRIDOR.1,
            "a split sitting inside a table's own cell gap must be redirected to the \
             gutter of the band it will be applied to; expected a split inside \
             {GH1762_GUTTER_CORRIDOR:?}, got {redirected} (incoming {snapped})"
        );
    }

    /// GH#1762: the page-wide corridor search must still see nothing, so the fixture's
    /// own failure mode is exactly the one the issue describes -- no corridor anywhere
    /// on the page -- and the band-scoped search is demonstrably what finds the gutter,
    /// not some incidental change to the page-wide one.
    #[test]
    fn the_page_wide_corridor_search_still_finds_nothing_gh1762() {
        let spans = gh1762_p1_real_reproducer_spans();
        let (lines, snapped) = gh1762_snapped_split(&spans);
        let min_gutter = (GH1762_PAGE_WIDTH * MIN_DENSE_COLUMN_GUTTER_FRACTION).max(MIN_DENSE_COLUMN_GUTTER_PTS);
        let furniture_width = GH1762_PAGE_WIDTH * FULL_WIDTH_FURNITURE_FRACTION;

        assert_eq!(
            snapped, 216.7825,
            "the four-column table's own cell gap still wins the vote"
        );
        assert!(
            page_whitespace_corridors(&spans, &lines, furniture_width, min_gutter).is_empty(),
            "Table 2's rows close every page-wide whitespace corridor"
        );
        assert!(
            page_low_occupancy_corridors(&spans, &lines, furniture_width, min_gutter, MAX_GUTTER_CROSSING_LINES)
                .is_empty(),
            "Table 2 crosses the gutter on far more than MAX_GUTTER_CROSSING_LINES lines"
        );
    }

    /// GH#1762: the band the corridor question is asked of must be a proper part of the
    /// page that leaves Table 2 out, and the gutter must be empty inside it. Without
    /// this the redirect could reach the right answer by some unrelated route and the
    /// scoping itself would never be shown to have done anything.
    #[test]
    fn the_corridor_search_is_scoped_to_the_band_under_the_full_width_table_gh1762() {
        let spans = gh1762_p1_real_reproducer_spans();
        let (lines, snapped) = gh1762_snapped_split(&spans);
        let min_gutter = (GH1762_PAGE_WIDTH * MIN_DENSE_COLUMN_GUTTER_FRACTION).max(MIN_DENSE_COLUMN_GUTTER_PTS);
        let furniture_width = GH1762_PAGE_WIDTH * FULL_WIDTH_FURNITURE_FRACTION;

        let band = band_lines_around_table_gap_split(&spans, &lines, furniture_width, min_gutter, snapped)
            .expect("one band carries every line the split sits inside the cell gap of");
        assert_eq!(
            band.len(),
            37,
            "the band is the 36 lines under Table 3's caption, plus the boundary line above them"
        );
        assert!(
            band.iter()
                .flatten()
                .all(|&index| spans[index].bbox.y < GH1762_TABLE_2_BOTTOM_Y),
            "Table 2's own rows must be out of scope: they are a band of their own, \
             separated from this one by boundary lines"
        );
        assert_eq!(
            page_whitespace_corridors(&spans, &band, furniture_width, min_gutter),
            [(286.22998, 307.0)],
            "inside that band the page's real gutter is the one and only empty corridor"
        );
    }

    /// GH#1762 end to end: the band below Table 2 must read column-major -- the whole of
    /// Table 3 (its caption, its description lines, then its rows) before the right
    /// column's `4.1.` heading and its body. Before the fix the band is reordered around
    /// 216.784 instead: `Secundus`/`Tertius` and every value under them land on the
    /// right of the split and are emitted after -- and interleaved with -- the right
    /// column's prose.
    #[test]
    fn dense_two_column_page_reorders_below_a_full_width_table_gh1762() {
        let mut spans = gh1762_p1_real_reproducer_spans();

        assert!(
            reorder_dense_two_column_page(&mut spans, GH1762_PAGE_WIDTH),
            "a two-column band under a full-width table must still be reordered"
        );

        let order: Vec<&str> = spans.iter().map(|span| span.text.as_str()).collect();
        let position = |text: &str| {
            order
                .iter()
                .position(|&candidate| candidate == text)
                .unwrap_or_else(|| panic!("`{text}` must survive the reorder"))
        };

        let heading = position("4.1. Lorem ipsum dolor sit amet");
        assert!(
            position("Table 3") < heading,
            "Table 3's caption belongs to the left column and must precede the right \
             column's heading"
        );
        assert!(
            position("Secundus Tertius") < heading,
            "Table 3's own column labels must stay in the left column, not be pushed \
             past the right column's heading by a split inside the table"
        );
        assert!(
            position("5.81") < heading,
            "the last cell of Table 3's last row must precede the right column's heading"
        );
        assert_eq!(
            order[heading + 1],
            "veniam quis nostrud exercitation ullamco laboris nisi aliquip ex ea",
            "the right column's heading must be followed by its own first body line, \
             not by a table cell"
        );
    }

    const GH1655_PAGE_WIDTH: f32 = 595.28;
    const GH1655_NUMBER_X: f32 = 45.22;
    const GH1655_TITLE_X: f32 = 80.68;

    struct Gh1655Row {
        number: &'static str,
        number_width: f32,
        gap_to_title: f32,
        title: &'static str,
        title_width: f32,
    }

    const GH1655_ROWS: [Gh1655Row; 5] = [
        Gh1655Row {
            number: "6",
            number_width: 6.6,
            gap_to_title: 25.2,
            title: "INBEDRIJFSTELLEN VAN HET TOESTEL",
            title_width: 200.0,
        },
        Gh1655Row {
            number: "6.1",
            number_width: 17.0,
            gap_to_title: 18.4,
            title: "Vullen en ontluchten van het cv-systeem",
            title_width: 210.0,
        },
        Gh1655Row {
            number: "6.1.1",
            number_width: 20.0,
            gap_to_title: 12.4,
            title: "CV-systeem",
            title_width: 70.0,
        },
        Gh1655Row {
            number: "6.1.2",
            number_width: 20.0,
            gap_to_title: 12.4,
            title: "Warmwatervoorziening",
            title_width: 140.0,
        },
        Gh1655Row {
            number: "6.1.3",
            number_width: 20.0,
            gap_to_title: 12.4,
            title: "Gastoevoer",
            title_width: 70.0,
        },
    ];

    /// GH#1655: reproduces the reporter's carrier -- a single-column page of
    /// hanging-number headings (`6.1.1` <tab> `CV-systeem`), where the producer emits
    /// the number-to-title tab as its own space-only span (a font change, so it never
    /// merges with a neighbour), plus two invisible empty-line spans elsewhere on the
    /// page and a two-span footer. Coordinates match the reporter's own measurements:
    /// numbers at x0=45.22, titles at x0=80.68, footer spans at 43.38..169.53 and
    /// 543.59..553.62, page width 595.28; the five per-row gaps (25.2, 18.4, 12.4,
    /// 12.4, 12.4) and the blank-line gap (329.6, midpoint 212.2) reproduce the
    /// reporter's exact measurements.
    ///
    /// `include_space_spans` selects the reporter's control page
    /// (`gh1655_hanging_number_control_page_without_space_spans_is_not_reordered`),
    /// which omits every space-only span -- the five tab spans and the two blank-line
    /// spans -- and must behave identically. ~keep
    fn gh1655_hanging_number_heading_spans(include_space_spans: bool) -> Vec<TextSpan> {
        let mut spans = Vec::new();
        for (row, entry) in GH1655_ROWS.iter().enumerate() {
            let y = 900.0 - row as f32 * 14.0;
            spans.push(span_with_width(
                entry.number,
                GH1655_NUMBER_X,
                y,
                entry.number_width,
                11.0,
                11.0,
            ));
            if include_space_spans {
                let space_left = GH1655_NUMBER_X + entry.number_width;
                let space_right = GH1655_TITLE_X - entry.gap_to_title;
                spans.push(span_with_width(
                    " ",
                    space_left,
                    y,
                    space_right - space_left,
                    11.0,
                    11.0,
                ));
            }
            spans.push(span_with_width(
                entry.title,
                GH1655_TITLE_X,
                y,
                entry.title_width,
                11.0,
                11.0,
            ));
        }
        if include_space_spans {
            spans.push(span_with_width(" ", 36.0, 830.0, 11.4, 11.0, 11.0));
            spans.push(span_with_width(" ", 377.0, 830.0, 20.0, 11.0, 11.0));
        }
        spans.push(span_with_width("Intergas Verwarming BV", 43.38, 60.0, 126.15, 9.0, 9.0));
        spans.push(span_with_width("30", 543.59, 60.0, 10.03, 9.0, 9.0));
        spans
    }

    /// GH#1655 defect page. Before the fix, all 7 lines (5 heading tabs, the blank
    /// line, the footer) vote for a split -- meeting `MIN_DENSE_COLUMN_SPLIT_LINES` (6)
    /// -- and the page is misread as two columns, tearing every number away from its
    /// own title. After the fix, the blank line is excluded for carrying no ink and the
    /// footer's 374pt gap is excluded by `MAX_DENSE_COLUMN_GUTTER_FRACTION`, leaving
    /// only the 5 genuine heading lines -- below quorum -- so `detect_split_x` returns
    /// `None` and the page is left alone. The `detect_split_x` assertion is the load-
    /// bearing one: with a 5-heading band this small, `classify_region`'s existing
    /// Table/row-pairing guards (unrelated to this fix, already covered elsewhere)
    /// independently refuse the reorder regardless of quorum, so asserting only the
    /// end-to-end result would pass even with this fix reverted. Asserting
    /// `detect_split_x` directly proves the quorum fix itself fired. ~keep
    #[test]
    fn gh1655_hanging_number_headings_with_tab_spans_and_footer_are_not_reordered() {
        let spans = gh1655_hanging_number_heading_spans(true);
        let order = spans_sorted_top_to_bottom(&spans);
        let lines = group_into_lines(&spans, &order);
        assert_eq!(
            lines.len(),
            7,
            "fixture must produce exactly the reporter's 7 voting lines"
        );
        assert_eq!(
            detect_split_x(&spans, &lines, GH1655_PAGE_WIDTH),
            None,
            "only 5 of the 7 lines are genuine gutter evidence, below the quorum of \
             MIN_DENSE_COLUMN_SPLIT_LINES -- if this is Some, the ink filter or the \
             max-gutter cap has regressed"
        );

        let mut spans = spans;
        let original = spans.iter().map(|span| span.text.clone()).collect::<Vec<_>>();
        assert!(
            !reorder_dense_two_column_page(&mut spans, GH1655_PAGE_WIDTH),
            "a single-column page of hanging-number headings must not be read as two columns"
        );
        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            original.iter().map(String::as_str).collect::<Vec<_>>(),
            "span order must be unchanged when the page is correctly left alone"
        );
    }

    /// GH#1655 control page: identical to the defect page but with every space-only
    /// span omitted (the five tab spans and the two blank-line spans). Must behave the
    /// same as the defect page -- not reordered. With the blank line gone entirely,
    /// only `MAX_DENSE_COLUMN_GUTTER_FRACTION` is exercised here (there is no
    /// whitespace-only line left for the ink filter to remove): pre-fix the footer's
    /// gap alone is (wrongly) accepted, reaching the 6-line quorum with the 5 headings;
    /// post-fix it is excluded, leaving 5 lines. ~keep
    #[test]
    fn gh1655_hanging_number_control_page_without_space_spans_is_not_reordered() {
        let spans = gh1655_hanging_number_heading_spans(false);
        let order = spans_sorted_top_to_bottom(&spans);
        let lines = group_into_lines(&spans, &order);
        assert_eq!(lines.len(), 6, "5 headings plus the footer, with no blank line");
        assert_eq!(
            detect_split_x(&spans, &lines, GH1655_PAGE_WIDTH),
            None,
            "the footer's gap must not count toward the quorum -- if this is Some, \
             MAX_DENSE_COLUMN_GUTTER_FRACTION has regressed"
        );

        let mut spans = spans;
        let original = spans.iter().map(|span| span.text.clone()).collect::<Vec<_>>();
        assert!(!reorder_dense_two_column_page(&mut spans, GH1655_PAGE_WIDTH));
        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            original.iter().map(String::as_str).collect::<Vec<_>>()
        );
    }

    /// GH#1655: `widest_gap_midpoint` must reject a gap that is implausibly wide to be
    /// a real gutter. The footer gap reproduces the reporter's own measurement (374pt
    /// on a 595.28pt page = 62.8%, far past `MAX_DENSE_COLUMN_GUTTER_FRACTION`'s 25%).
    #[test]
    fn widest_gap_midpoint_rejects_a_gap_wider_than_the_maximum_plausible_gutter() {
        let min_gutter = (GH1655_PAGE_WIDTH * MIN_DENSE_COLUMN_GUTTER_FRACTION).max(MIN_DENSE_COLUMN_GUTTER_PTS);
        let max_gutter = GH1655_PAGE_WIDTH * MAX_DENSE_COLUMN_GUTTER_FRACTION;
        let footer_edges = [(43.38_f32, 169.53_f32), (543.59_f32, 553.62_f32)];

        assert_eq!(
            widest_gap_midpoint(footer_edges.into_iter(), min_gutter, max_gutter),
            None,
            "a 374pt gap on a 595.28pt page must not be accepted as a gutter"
        );
    }

    /// Companion negative control: an ordinary two-column gutter well inside the cap
    /// (`dense_two_column_spans`'s own 60pt gutter on a 612pt page) must still be
    /// accepted, proving the cap does not also reject real gutters.
    #[test]
    fn widest_gap_midpoint_accepts_an_ordinary_two_column_gutter() {
        const PAGE_WIDTH: f32 = 612.0;
        let min_gutter = (PAGE_WIDTH * MIN_DENSE_COLUMN_GUTTER_FRACTION).max(MIN_DENSE_COLUMN_GUTTER_PTS);
        let max_gutter = PAGE_WIDTH * MAX_DENSE_COLUMN_GUTTER_FRACTION;
        let edges = [(60.0_f32, 260.0_f32), (320.0_f32, 510.0_f32)];

        assert_eq!(
            widest_gap_midpoint(edges.into_iter(), min_gutter, max_gutter),
            Some(290.0),
            "a 60pt gutter on a 612pt page must still be accepted"
        );
    }

    /// GH#1655: `reorder_band_columns`'s density gate must count only inked spans, not
    /// raw indices. The left side below has 5 real prose lines plus 2 whitespace-only
    /// spans -- 7 raw indices, meeting `MIN_DENSE_COLUMN_SPANS_PER_SIDE` (6) by count
    /// alone -- but only 5 carry ink, so the gate must still refuse the band even
    /// though the right side is a clean, reorderable 6-line prose column.
    #[test]
    fn reorder_band_columns_ignores_whitespace_spans_toward_the_density_gate() {
        const SPLIT_X: f32 = 300.0;
        const LEFT_X: f32 = 60.0;
        const RIGHT_X: f32 = 320.0;
        let left_lines = [
            "The committee reviewed annual budget totals",
            "and approved new funding for the coming year",
            "after several rounds of careful review by",
            "senior staff members from every department",
            "who evaluated priorities across the whole",
        ];
        let right_lines = [
            "Numerous studies have examined similar",
            "programs across comparable institutions",
            "using consistent methodology and controls",
            "for measuring outcomes over multiple years",
            "researchers found consistent positive trends",
            "supporting continued investment going forward",
        ];

        let mut spans = Vec::new();
        for (row, text) in left_lines.iter().enumerate() {
            spans.push(span_with_width(
                text,
                LEFT_X,
                830.0 - row as f32 * 14.0,
                200.0,
                11.0,
                11.0,
            ));
        }
        spans.push(span_with_width(" ", LEFT_X, 830.0 - 5.0 * 14.0, 5.0, 11.0, 11.0));
        spans.push(span_with_width(" ", LEFT_X, 830.0 - 6.0 * 14.0, 5.0, 11.0, 11.0));
        for (row, text) in right_lines.iter().enumerate() {
            spans.push(span_with_width(
                text,
                RIGHT_X,
                830.0 - row as f32 * 14.0,
                200.0,
                11.0,
                11.0,
            ));
        }
        let band: Vec<usize> = (0..spans.len()).collect();

        let left_raw = band.iter().filter(|&&index| spans[index].bbox.x < SPLIT_X).count();
        assert_eq!(
            left_raw, 7,
            "fixture must meet the raw per-side quorum by index count alone"
        );

        assert!(
            reorder_band_columns(&spans, &band, SPLIT_X).is_none(),
            "only 5 of the left side's 7 spans carry ink; the density gate must still refuse the band"
        );
    }

    /// Build a `page_count`-page PDF where every page carries `rows` lines of Standard-14
    /// text in four columns and opens with a token unique to that page (`PAGEMARK0007`).
    ///
    /// The text is real content-stream operators, not an empty `/MediaBox`, so each page
    /// costs real parsing, font-metric and span-assembly work, so the page-order test
    /// exercises the same per-page path a real document does.
    fn build_paged_text_pdf(page_count: usize, rows: usize) -> Vec<u8> {
        let font_obj = 3 + 2 * page_count;
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets: Vec<usize> = Vec::new();

        offsets.push(pdf.len());
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        offsets.push(pdf.len());
        let kids: String = (0..page_count).map(|i| format!("{} 0 R ", 3 + i)).collect();
        pdf.extend_from_slice(
            format!(
                "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
                kids.trim_end(),
                page_count
            )
            .as_bytes(),
        );

        for page in 0..page_count {
            offsets.push(pdf.len());
            pdf.extend_from_slice(
                format!(
                    "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
                     /Contents {} 0 R /Resources << /Font << /F1 {} 0 R >> >> >>\nendobj\n",
                    3 + page,
                    3 + page_count + page,
                    font_obj
                )
                .as_bytes(),
            );
        }

        const COLUMN_X: [f32; 4] = [40.0, 180.0, 320.0, 460.0];
        for page in 0..page_count {
            let mut stream = String::new();
            for row in 0..rows {
                let y = 760.0 - (row as f32) * 14.0;
                for (col, x) in COLUMN_X.iter().enumerate() {
                    let text = if row == 0 && col == 0 {
                        format!("PAGEMARK{:04}", page + 1)
                    } else {
                        format!("p{}r{}c{} lorem ipsum", page + 1, row, col)
                    };
                    stream.push_str(&format!("BT /F1 10 Tf {:.1} {:.1} Td ({}) Tj ET\n", x, y, text));
                }
            }
            offsets.push(pdf.len());
            pdf.extend_from_slice(
                format!(
                    "{} 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
                    3 + page_count + page,
                    stream.len(),
                    stream
                )
                .as_bytes(),
            );
        }

        offsets.push(pdf.len());
        pdf.extend_from_slice(
            format!(
                "{} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
                 /Encoding /WinAnsiEncoding >>\nendobj\n",
                font_obj
            )
            .as_bytes(),
        );

        let xref_pos = pdf.len();
        let total_objs = offsets.len() + 1;
        pdf.extend_from_slice(format!("xref\n0 {}\n", total_objs).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f\r\n");
        for &off in &offsets {
            pdf.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                total_objs, xref_pos
            )
            .as_bytes(),
        );
        pdf
    }

    /// The page number carried by every `PAGEMARK` token in `text`, in the order they appear.
    fn pagemark_sequence(text: &str) -> Vec<usize> {
        text.match_indices("PAGEMARK")
            .filter_map(|(at, _)| text.get(at + "PAGEMARK".len()..at + "PAGEMARK".len() + 4))
            .filter_map(|digits| digits.parse::<usize>().ok())
            .collect()
    }

    /// #1723: both text paths must emit the pages in ascending order and unaltered. A test
    /// that only counts pages passes on a shuffled document, so this pins the sequence and
    /// the per-page bytes: the collected texts must equal a page-by-page run, the
    /// concatenation must carry the page markers in ascending order, and each tracked page
    /// boundary must slice out exactly its own page. Page order is what a reader of
    /// `extract_all_page_texts` is most likely to trade away for speed, and the text it
    /// feeds is the same text every downstream consumer indexes by offset.
    #[test]
    fn native_page_text_preserves_page_order_and_content() {
        let page_count = 24;
        let pdf = build_paged_text_pdf(page_count, 10);
        let mut doc = NativeDocument::open_bytes(&pdf).expect("fixture must open");
        let margins = PageMarginFractions::default();

        let excluded_layers = xberg_native_pdf::optional_content::compute_default_off_ocgs(&doc.doc);
        let sequential: Vec<String> = (0..page_count)
            .map(|page_idx| {
                extract_one_page_text(&doc.doc, page_idx, &excluded_layers, margins)
                    .expect("page must extract")
                    .0
            })
            .collect();
        assert!(
            sequential.iter().all(|page| !page.trim().is_empty()),
            "fixture pages must carry text, otherwise this test proves nothing"
        );

        let (collected, _) =
            extract_all_page_texts(&doc.doc, margins, None).expect("collecting every page must succeed");
        assert_eq!(
            collected, sequential,
            "the collected page texts must match a page-by-page run, page for page"
        );

        let (content, _, _, _) =
            extract_text_from_native_document(&mut doc, None, None, margins).expect("fast path must succeed");
        // The separator is spelled out rather than read from `PAGE_SEPARATOR`: an assertion
        // built from the same constant the code writes cannot fail when that constant
        // changes, which is the one thing every consumer's byte offsets depend on. ~keep
        assert_eq!(
            content,
            sequential.join("\n\n"),
            "fast path must concatenate pages in order, separated by one blank line"
        );
        assert_eq!(
            pagemark_sequence(&content),
            (1..=page_count).collect::<Vec<_>>(),
            "page markers must appear in ascending page order"
        );

        let page_config = PageConfig {
            extract_pages: true,
            ..PageConfig::default()
        };
        let (tracked, boundaries, pages, _) =
            extract_text_from_native_document(&mut doc, Some(&page_config), None, margins)
                .expect("tracking path must succeed");
        let boundaries = boundaries.expect("tracking path must report boundaries");
        let pages = pages.expect("tracking path must report page contents");
        assert_eq!(boundaries.len(), page_count);
        assert_eq!(pages.len(), page_count);

        for (page_idx, boundary) in boundaries.iter().enumerate() {
            assert_eq!(boundary.page_number, (page_idx + 1) as u32);
            let slice = &tracked[boundary.byte_start..boundary.byte_end];
            assert_eq!(
                slice,
                sequential[page_idx],
                "boundary {} must slice out its own page",
                page_idx + 1
            );
            assert_eq!(pages[page_idx].content, sequential[page_idx]);
            assert_eq!(pages[page_idx].page_number, (page_idx + 1) as u32);
        }
    }

    #[test]
    fn native_page_text_stops_before_work_when_cancelled() {
        let pdf = build_paged_text_pdf(2, 10);
        let doc = NativeDocument::open_bytes(&pdf).expect("fixture must open");
        let token = crate::cancellation::CancellationToken::new();
        token.cancel();

        let error = extract_all_page_texts(&doc.doc, PageMarginFractions::default(), Some(&token))
            .expect_err("a cancelled extraction must not read any page text");

        assert!(matches!(error, PdfError::Cancelled));
    }

    /// #1744: the provenance pass in `pdf/scan_detect.rs` used to read every page's text a
    /// second time to grade its fabricated-mapping ratio, after the main text pass had already
    /// read the same raw spans once. `extract_text_and_metadata` must now carry those spans'
    /// counts forward instead, so the whole-document separate read
    /// (`scan_detect::fabricated_provenance_page_indices`) is never reached for a document with
    /// no excluded optional-content layers — the common case.
    ///
    /// [`FABRICATED_PROVENANCE_SECOND_PASS_CALLS`](crate::pdf::scan_detect::FABRICATED_PROVENANCE_SECOND_PASS_CALLS)
    /// is incremented only inside that whole-document function, so a count of zero after this
    /// call proves the second read did not happen, not just that the final numbers happen to
    /// agree. The expected page list is computed independently, on its own document handle,
    /// before the counter is reset, so computing it cannot mask a regression.
    ///
    /// The counter is thread-local (issue #1752 review): a process-global counter was
    /// incremented by ANY test in the binary reaching `fabricated_provenance_page_indices` via
    /// the production path, not only `#[serial]`-marked siblings -- `#[serial]` excludes other
    /// `#[serial]` tests, not the hundreds of ordinary ones. `extract_text_and_metadata` below
    /// runs synchronously on this test's own thread, so its thread-local counter observes
    /// exactly this call graph. ~keep
    #[test]
    #[serial_test::serial]
    fn provenance_is_not_read_a_second_time_for_a_document_with_no_excluded_layers() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/non_ascii_text.pdf");
        let bytes = std::fs::read(&path).expect("corpus document must read");
        let thresholds = crate::core::config::OcrQualityThresholds::default();

        let baseline_doc = NativeDocument::open_bytes(&bytes).expect("corpus document must open");
        let expected_fabricated: Vec<u32> = crate::pdf::scan_detect::fabricated_provenance_page_indices(
            &baseline_doc.doc,
            thresholds.min_provenance_fallback_ratio,
            thresholds.min_total_non_whitespace,
        )
        .into_iter()
        .map(|index| index as u32 + 1)
        .collect();
        assert!(
            !expected_fabricated.is_empty(),
            "fixture must fabricate at least one page for this test to mean anything"
        );

        crate::pdf::scan_detect::FABRICATED_PROVENANCE_SECOND_PASS_CALLS
            .with(|counter| counter.store(0, std::sync::atomic::Ordering::SeqCst));

        let mut doc = NativeDocument::open_bytes(&bytes).expect("corpus document must open a second time");
        let (_, _, _, metadata) = extract_text_and_metadata(&mut doc, None).expect("extraction must succeed");

        assert_eq!(
            crate::pdf::scan_detect::FABRICATED_PROVENANCE_SECOND_PASS_CALLS
                .with(|counter| counter.load(std::sync::atomic::Ordering::SeqCst)),
            0,
            "extract_text_and_metadata must not read every page's text a second time for provenance (issue #1744)"
        );
        assert_eq!(
            metadata.pdf_specific.fabricated_text_pages,
            Some(expected_fabricated),
            "fabricated_text_pages must match the pre-#1744 page-by-page computation exactly"
        );
    }

    /// GH#1801: the redirect is suppressed only when a real population of lines outside the
    /// band agrees the split is already the gutter. Pins all three terms of that gate, because
    /// each one alone admits a page the other two reject: the population floor (an empty
    /// out-of-band set reports agreement it never measured), the absolute cap, and the rate.
    #[test]
    fn gh1801_out_of_band_agreement_needs_a_population_and_a_low_rate() {
        const SPLIT_X: f32 = 225.0;
        const MIN_GUTTER: f32 = 20.0;
        const FURNITURE_WIDTH: f32 = 400.0;

        let mut spans: Vec<TextSpan> = Vec::new();
        let mut lines: Vec<SpanLine> = Vec::new();
        let mut band: Vec<SpanLine> = Vec::new();

        // A full-width table: every row straddles the split, so every row disagrees with it.
        // These are the band, and the gate must ignore them -- they are the reason the corridor
        // search was scoped to a band in the first place (GH#1762).
        for row in 0..3 {
            let y = 700.0 - row as f32 * 10.0;
            let first = spans.len();
            spans.push(span_with_width("left cell", 50.0, y, 150.0, 7.0, 7.0));
            spans.push(span_with_width("right cell", 250.0, y, 150.0, 7.0, 7.0));
            let line = vec![first, first + 1];
            lines.push(line.clone());
            band.push(line);
        }

        // Two columns of prose below it. Each line lies wholly on one side of the split, so it
        // neither crosses it nor holds it inside one of its own gaps: it agrees.
        for row in 0..8 {
            let y = 600.0 - row as f32 * 10.0;
            for column_x in [50.0f32, 250.0] {
                let first = spans.len();
                spans.push(span_with_width("prose", column_x, y, 150.0, 7.0, 7.0));
                lines.push(vec![first]);
            }
        }

        let (outside, disagreeing) =
            out_of_band_split_agreement(&spans, &lines, &band, FURNITURE_WIDTH, MIN_GUTTER, SPLIT_X);
        assert_eq!(
            (outside, disagreeing),
            (16, 0),
            "the sixteen prose lines are outside the band and all agree; the three table rows are in it"
        );
        assert!(
            outside >= MIN_DENSE_COLUMN_SPLIT_LINES
                && disagreeing < MIN_DENSE_COLUMN_SPLIT_LINES
                && disagreeing as f32 <= outside as f32 * MAX_OUT_OF_BAND_SPLIT_DISAGREEMENT_FRACTION,
            "a page whose prose agrees must keep its split"
        );

        // The rate term is load-bearing on its own: three straddling lines are still under the
        // absolute cap of six, but 3 of 19 is 15.8% -- above the 8% a real gutter shows.
        let mut with_straddlers = lines.clone();
        for row in 0..3 {
            let first = spans.len();
            spans.push(span_with_width(
                "a full width caption",
                50.0,
                500.0 - row as f32 * 10.0,
                350.0,
                7.0,
                7.0,
            ));
            with_straddlers.push(vec![first]);
        }
        let (outside, disagreeing) =
            out_of_band_split_agreement(&spans, &with_straddlers, &band, FURNITURE_WIDTH, MIN_GUTTER, SPLIT_X);
        assert_eq!((outside, disagreeing), (19, 3), "the three captions cross the split");
        assert!(
            disagreeing < MIN_DENSE_COLUMN_SPLIT_LINES,
            "the absolute cap alone would still keep this split"
        );
        assert!(
            disagreeing as f32 > outside as f32 * MAX_OUT_OF_BAND_SPLIT_DISAGREEMENT_FRACTION,
            "the rate term is what redirects it"
        );

        // The documented trap: when the band is the whole page there is nothing outside it, and
        // a disagreement count of zero means nothing was examined rather than everyone agreeing.
        let (outside, disagreeing) =
            out_of_band_split_agreement(&spans, &lines, &lines, FURNITURE_WIDTH, MIN_GUTTER, SPLIT_X);
        assert_eq!(
            (outside, disagreeing),
            (0, 0),
            "no line is outside a band that is the page"
        );
        assert!(
            outside < MIN_DENSE_COLUMN_SPLIT_LINES,
            "the population floor must reject a vacuous agreement"
        );
    }

    /// GH#1752: `page_boundaries_required` must widen boundary tracking to the two per-page OCR
    /// settings without changing behaviour for any caller that leaves them unset. This is the
    /// load-bearing assertion for that: with both settings at their default `None`, boundary
    /// tracking must stay off, because `None == Some(true)` is always false and the widened
    /// predicate must reduce byte-for-byte to the pre-GH#1752 predicate (`force_ocr_pages`
    /// non-empty or `ocr.is_some()`).
    #[test]
    fn page_boundaries_required_default_config_stays_on_the_fast_path() {
        let pdf = build_paged_text_pdf(2, 3);
        let mut doc = NativeDocument::open_bytes(&pdf).expect("fixture must open");
        let margins = PageMarginFractions::default();
        let config = ExtractionConfig::default();

        let (_, boundaries, _, _) =
            extract_text_from_native_document(&mut doc, None, Some(&config), margins).expect("extraction must succeed");

        assert!(
            boundaries.is_none(),
            "a default config must stay on the fast path with no boundaries -- this is what \
             proves the widening is byte-identical to the pre-GH#1752 predicate at defaults"
        );
    }

    #[test]
    fn page_boundaries_required_ocr_block_alone_enables_boundaries() {
        let pdf = build_paged_text_pdf(2, 3);
        let mut doc = NativeDocument::open_bytes(&pdf).expect("fixture must open");
        let margins = PageMarginFractions::default();
        let config = ExtractionConfig {
            ocr: Some(crate::core::config::OcrConfig::default()),
            ..ExtractionConfig::default()
        };

        let (_, boundaries, _, _) =
            extract_text_from_native_document(&mut doc, None, Some(&config), margins).expect("extraction must succeed");

        assert!(
            boundaries.is_some(),
            "an `ocr` block alone must enable boundary tracking"
        );
    }

    /// GH#1752: the actual regression this ticket fixes -- before the widening, this configuration
    /// left `needs_boundaries` `false` because the old predicate only checked `config.ocr.is_some()`.
    #[test]
    fn page_boundaries_required_near_empty_fallback_alone_enables_boundaries() {
        let pdf = build_paged_text_pdf(2, 3);
        let mut doc = NativeDocument::open_bytes(&pdf).expect("fixture must open");
        let margins = PageMarginFractions::default();
        let config = ExtractionConfig {
            ocr_near_empty_fallback: Some(true),
            ..ExtractionConfig::default()
        };

        let (_, boundaries, _, _) =
            extract_text_from_native_document(&mut doc, None, Some(&config), margins).expect("extraction must succeed");

        assert!(
            boundaries.is_some(),
            "ocr_near_empty_fallback = Some(true) without an `ocr` block must still enable \
             boundary tracking (GH#1752)"
        );
    }

    /// GH#1752: same regression as above, for the sibling setting.
    #[test]
    fn page_boundaries_required_scanned_page_quality_gate_alone_enables_boundaries() {
        let pdf = build_paged_text_pdf(2, 3);
        let mut doc = NativeDocument::open_bytes(&pdf).expect("fixture must open");
        let margins = PageMarginFractions::default();
        let config = ExtractionConfig {
            ocr_scanned_page_quality_gate: Some(true),
            ..ExtractionConfig::default()
        };

        let (_, boundaries, _, _) =
            extract_text_from_native_document(&mut doc, None, Some(&config), margins).expect("extraction must succeed");

        assert!(
            boundaries.is_some(),
            "ocr_scanned_page_quality_gate = Some(true) without an `ocr` block must still \
             enable boundary tracking (GH#1752)"
        );
    }

    #[test]
    fn page_boundaries_required_ocr_block_wins_even_with_settings_explicitly_off() {
        let pdf = build_paged_text_pdf(2, 3);
        let mut doc = NativeDocument::open_bytes(&pdf).expect("fixture must open");
        let margins = PageMarginFractions::default();
        let config = ExtractionConfig {
            ocr: Some(crate::core::config::OcrConfig::default()),
            ocr_near_empty_fallback: Some(false),
            ocr_scanned_page_quality_gate: Some(false),
            ..ExtractionConfig::default()
        };

        let (_, boundaries, _, _) =
            extract_text_from_native_document(&mut doc, None, Some(&config), margins).expect("extraction must succeed");

        assert!(
            boundaries.is_some(),
            "an `ocr` block alone is still sufficient; Some(false) on the sibling settings \
             does not take anything away"
        );
    }

    /// GH#1800: a two-cell-per-row sparse table sits in the left column (cell 0 at x
    /// 40..65, cell 1 at x 105..130 -- columns 2-4 of the reporter's 5-column table are
    /// never filled in this reproducer, which is what "sparse" means here), beside an
    /// ordinary prose column starting at x 148. The table's own cell gap (65..105, 40pt)
    /// is wider than the true column gutter (130..148, 18pt) -- 2.2x, the same order as
    /// the reporter's measured 53-vs-21 and 37-vs-24 pages. `detect_split_x` is presumed
    /// to have landed inside the table's second cell (a bimodal median artifact); PATH 1
    /// must not redirect it to the table's own cell gap just because that gap is wider. ~keep
    #[test]
    fn redirect_prefers_the_true_gutter_over_a_wider_sparse_table_cell_gap_gh1800() {
        let mut spans = Vec::new();
        for row in 0..8 {
            let y = 500.0 - row as f32 * 8.0;
            spans.push(span_with_width(&format!("{row}"), 40.0, y, 25.0, 8.0, 8.0));
            spans.push(span_with_width(&format!("{row}00"), 105.0, y, 25.0, 8.0, 8.0));
        }
        for row in 0..8 {
            let y = 650.0 - row as f32 * 12.0;
            spans.push(span_with_width(
                "some prose text filling out this line of the right column",
                148.0,
                y,
                250.0,
                12.0,
                12.0,
            ));
        }
        let lines = corridor_fixture_lines(&spans);
        let furniture_width = CORRIDOR_PAGE_WIDTH * FULL_WIDTH_FURNITURE_FRACTION;
        let min_gutter = (CORRIDOR_PAGE_WIDTH * MIN_DENSE_COLUMN_GUTTER_FRACTION).max(MIN_DENSE_COLUMN_GUTTER_PTS);
        let corridors = page_whitespace_corridors(&spans, &lines, furniture_width, min_gutter);
        assert_eq!(
            corridors,
            vec![(65.0, 105.0), (130.0, 148.0)],
            "both the false table gap and the true gutter must be present as raw candidates"
        );

        let split_x = 115.0; // inside the table's second cell, as a bimodal median would land
        let redirected = redirect_split_out_of_content(&spans, &lines, CORRIDOR_PAGE_WIDTH, split_x);
        assert!(
            (redirected - 139.0).abs() < 1.0,
            "expected the 18pt true gutter (mid 139), got {redirected} -- \
             the 40pt table cell gap (mid 85) must not win merely for being wider"
        );
    }

    const LANDSCAPE_PAGE_WIDTH: f32 = 1368.0;

    /// A 1368pt landscape page: two 13pt prose columns at x 29.9 and 319.3 (gutter
    /// 292.5..319.3, under this page's `min_gutter`), a 55pt title set across the gutter,
    /// a lone header label at x 651 (optionally written as six spans), optionally a page
    /// number at x 1331, and small labels under the left column. The only whitespace
    /// corridors run beside the header label.
    fn landscape_title_over_gutter_spans(with_page_number: bool, label_in_six_spans: bool) -> Vec<TextSpan> {
        let top = 792.0;
        let mut spans = vec![
            span_with_width("4", 21.6, top - 768.5, 13.6, 20.0, 20.0),
            span_with_width("Carpenters", 651.0, top - 43.7, 78.0, 13.0, 13.0),
            span_with_width("25", 1331.4, top - 48.7, 16.2, 13.0, 13.0),
            span_with_width("Silver Morning", 29.9, top - 311.8, 381.4, 55.0, 55.0),
            span_with_width("Storytelling", 29.6, top - 619.8, 123.1, 14.0, 14.0),
            span_with_width("Painter", 139.2, top - 648.9, 45.4, 11.0, 11.0),
            span_with_width("Window", 126.4, top - 678.6, 44.7, 11.0, 11.0),
        ];
        if !with_page_number {
            spans.retain(|span| span.text != "25");
        }
        if label_in_six_spans {
            spans.retain(|span| span.text != "Carpenters");
            for (i, piece) in ["Ca", "rp", "en", "te", "r", "s"].into_iter().enumerate() {
                spans.push(span_with_width(
                    piece,
                    651.0 + i as f32 * 13.0,
                    top - 43.7,
                    13.0,
                    13.0,
                    13.0,
                ));
            }
        }
        let left_widths = [
            262.6, 250.4, 251.6, 219.8, 228.1, 251.5, 233.9, 229.4, 251.1, 243.9, 240.1, 135.4,
        ];
        let left_text = [
            "Loud ocean warm mountain the city film and",
            "careful, weather quiet season an picture",
            "road festival in dark table hand kitchen",
            "rain on wild. Beautiful at a mountain",
            "storytelling calm blue for market gold",
            "sculpture to orchard note station crew. Year",
            "lantern morning of festival page harbor,",
            "shoreline beautiful painter loud warm",
            "mountain sculpture. Bright a copper was",
            "storytelling city film road dark are river",
            "by a storytelling hand careful light",
            "shoreline it beautiful.",
        ];
        for (row, (text, width)) in left_text.into_iter().zip(left_widths).enumerate() {
            let y = top - (355.3 + row as f32 * 17.0);
            spans.push(span_with_width(text, 29.9, y, width, 13.0, 13.0));
        }
        let right_rows = [0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13];
        let right_widths = [
            247.7, 281.8, 258.8, 246.5, 252.2, 274.5, 233.0, 106.5, 275.7, 248.8, 239.8, 218.6, 106.8,
        ];
        let right_text = [
            "Brand weather remarkable can rhythm say",
            "festival mountain, new industrial storytelling",
            "rain we wild go studio cabin calm blue north",
            "old picture. Up festival Bread Canvas so",
            "One Sculpture gold be, Kitchen carpenters",
            "garden mountain do shoreline as two remarkable",
            "orchard festival note industrial red",
            "mountain crew sun.",
            "Carpenters sea station year metal page loud",
            "warm city film grain lantern morning painter",
            "a festival map or chalk careful, paper",
            "mountain weather, key store fog no a",
            "storytelling ocean.",
        ];
        for ((row, text), width) in right_rows.into_iter().zip(right_text).zip(right_widths) {
            let y = top - (355.3 + row as f32 * 17.0);
            spans.push(span_with_width(text, 319.3, y, width, 13.0, 13.0));
        }
        spans
    }

    /// The title crossing the gutter sends the split to the corridor search. The corridor
    /// between the right column and the header label reads as a column boundary page-wide,
    /// but only one row sits right of it. The split must stay in the gutter and the columns
    /// must come out left then right. Without the page number that corridor is also the
    /// widest one in reach. Writing the label as six spans must not turn it into a column.
    #[test]
    fn split_crossed_by_a_title_is_not_redirected_beside_a_lone_header_label() {
        for with_page_number in [true, false] {
            for label_in_six_spans in [false, true] {
                assert_title_page_keeps_its_gutter(landscape_title_over_gutter_spans(
                    with_page_number,
                    label_in_six_spans,
                ));
            }
        }
    }

    /// A single-column page whose split landed inside the body: every body line crosses it,
    /// so the widened search runs, and the only corridor is the margin beside the running
    /// header. That move stands even though the margin side holds one row, because leaving
    /// the split inside the body reorders the page against a split that is not a gutter.
    #[test]
    fn split_inside_a_single_column_still_moves_to_the_margin_beside_a_running_header() {
        let page_width = 666.0;
        let mut spans = vec![
            span_with_width("78", 70.6, 840.0, 10.0, 9.0, 9.0),
            span_with_width("UE 8. Circulation - Métabolismes", 86.0, 840.0, 188.5, 9.0, 9.0),
        ];
        for row in 0..14 {
            let y = 800.0 - row as f32 * 14.0;
            spans.push(span_with_width(
                "Traitement nécessaire pour prévenir la fonte musculaire et la lithiase",
                148.6,
                y,
                360.0,
                10.0,
                10.0,
            ));
        }
        let lines = corridor_fixture_lines(&spans);
        let furniture_width = page_width * FULL_WIDTH_FURNITURE_FRACTION;
        let split_x = 230.0;
        assert!(lines_crossing(&spans, &lines, furniture_width, split_x) >= MIN_DENSE_COLUMN_SPLIT_LINES);
        let redirected = redirect_split_out_of_content(&spans, &lines, page_width, split_x);
        assert!(
            redirected < 148.6,
            "the split must leave the body for the margin, got {redirected}"
        );
    }

    fn assert_title_page_keeps_its_gutter(mut spans: Vec<TextSpan>) {
        let lines = corridor_fixture_lines(&spans);
        let detected = detect_split_x(&spans, &lines, LANDSCAPE_PAGE_WIDTH).expect("a split is detected");
        assert!(
            (292.5..319.3).contains(&detected),
            "the fixture must put the split in the gutter, got {detected}"
        );
        let redirected = redirect_split_out_of_content(&spans, &lines, LANDSCAPE_PAGE_WIDTH, detected);
        assert_eq!(redirected, detected, "a corridor beside a lone label is not a gutter");

        assert!(reorder_dense_two_column_page(&mut spans, LANDSCAPE_PAGE_WIDTH));
        let order: Vec<&str> = spans.iter().map(|span| span.text.as_str()).collect();
        let position = |text: &str| order.iter().position(|&candidate| candidate == text).unwrap();
        assert!(
            position("shoreline it beautiful.") < position("Brand weather remarkable can rhythm say"),
            "the left column must be read whole before the right one: {order:?}"
        );
    }

    const GH2102_PAGE_WIDTH: f32 = 595.28;
    const GH2102_NOTE_RIGHT_EDGE: f32 = 125.3;
    const GH2102_MAIN_X: f32 = 138.3;
    const GH2102_TABLE_COLUMNS: [(f32, f32); 5] =
        [(155.8, 27.8), (219.5, 8.0), (294.4, 8.0), (357.9, 4.0), (413.6, 72.0)];

    fn gh2102_note(index: usize, x_right: f32, top: f32) -> TextSpan {
        let width = 60.0 + ((index * 7) % 20) as f32;
        span_with_width(
            &format!("margin note line {index}"),
            x_right - width,
            top - index as f32 * 9.6,
            width,
            8.0,
            8.0,
        )
    }

    /// GH#2102 geometry, taken from the reporter's page: 8pt notes right-aligned to x=125.3
    /// on 9.6pt leading, a 10pt main column from x=138.3 on 13pt leading (a 13pt gutter), and
    /// a margin label on the same baseline as the main heading. No note line shares a
    /// baseline with a main line, so no line carries spans from both columns. The spans are
    /// returned in the full-page-width Y-then-X order `ColumnAware` produced for it. ~keep
    fn gh2102_margin_note_page(note_count: usize, with_table: bool) -> Vec<TextSpan> {
        let mut spans = vec![
            span_with_width("margin label", 82.4, 741.6, 42.9, 8.0, 8.0),
            span_with_width("main", GH2102_MAIN_X, 741.6, 68.3, 11.0, 11.0),
            span_with_width("heading", 206.6, 741.6, 20.8, 11.0, 11.0),
            span_with_width("text", 227.4, 741.6, 106.9, 11.0, 11.0),
        ];
        spans.extend((0..note_count).map(|index| gh2102_note(index, GH2102_NOTE_RIGHT_EDGE, 732.0)));
        for row in 0..8 {
            let y = 714.9 - row as f32 * 13.0;
            spans.push(span_with_width(
                &format!("main body {row} a"),
                GH2102_MAIN_X,
                y,
                200.0,
                10.0,
                10.0,
            ));
            spans.push(span_with_width(
                &format!("main body {row} b"),
                338.3,
                y,
                175.0,
                10.0,
                10.0,
            ));
        }
        if with_table {
            for row in 0..6 {
                let y = 560.0 - row as f32 * 21.0;
                for (column, &(x, width)) in GH2102_TABLE_COLUMNS.iter().enumerate() {
                    spans.push(span_with_width(&format!("t{row}c{column}"), x, y, width, 8.0, 8.0));
                }
            }
        }
        spans.sort_by(|a, b| {
            b.bbox
                .y
                .total_cmp(&a.bbox.y)
                .then_with(|| a.bbox.x.total_cmp(&b.bbox.x))
        });
        spans
    }

    fn gh2102_expected_column_major(note_count: usize, with_table: bool) -> Vec<String> {
        let mut expected = vec!["margin label".to_string()];
        expected.extend((0..note_count).map(|index| format!("margin note line {index}")));
        expected.extend(["main", "heading", "text"].map(String::from));
        for row in 0..8 {
            expected.push(format!("main body {row} a"));
            expected.push(format!("main body {row} b"));
        }
        if with_table {
            for row in 0..6 {
                expected.extend((0..GH2102_TABLE_COLUMNS.len()).map(|column| format!("t{row}c{column}")));
            }
        }
        expected
    }

    fn gh2102_corridor_count(spans: &[TextSpan], lines: &[SpanLine]) -> usize {
        let min_gutter = (GH2102_PAGE_WIDTH * MIN_DENSE_COLUMN_GUTTER_FRACTION).max(MIN_DENSE_COLUMN_GUTTER_PTS);
        let furniture_width = GH2102_PAGE_WIDTH * FULL_WIDTH_FURNITURE_FRACTION;
        page_whitespace_corridors(spans, lines, furniture_width, min_gutter).len()
    }

    #[test]
    fn margin_notes_on_their_own_leading_reorder_before_the_main_column_gh2102() {
        let mut spans = gh2102_margin_note_page(12, false);

        assert!(reorder_dense_two_column_page(&mut spans, GH2102_PAGE_WIDTH));

        let expected = gh2102_expected_column_major(12, false);
        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            expected.iter().map(String::as_str).collect::<Vec<_>>(),
            "the margin label and its notes must be read whole before the main heading"
        );
    }

    #[test]
    fn margin_notes_beside_a_table_reorder_before_the_main_column_gh2102() {
        let mut spans = gh2102_margin_note_page(12, true);

        assert!(reorder_dense_two_column_page(&mut spans, GH2102_PAGE_WIDTH));

        let expected = gh2102_expected_column_major(12, true);
        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            expected.iter().map(String::as_str).collect::<Vec<_>>(),
            "the table's own grid must not be what decides whether the notes are separated"
        );
    }

    #[test]
    fn a_few_notes_beside_the_main_text_are_not_a_gutter_gh2102() {
        let spans = gh2102_margin_note_page(MIN_DENSE_COLUMN_SPANS_PER_SIDE - 2, false);
        let order = spans_sorted_top_to_bottom(&spans);
        let lines = group_into_lines(&spans, &order);
        assert_eq!(
            gh2102_corridor_count(&spans, &lines),
            1,
            "the fixture must still present the corridor, or this test would pass for the wrong reason"
        );

        assert_eq!(detect_split_x(&spans, &lines, GH2102_PAGE_WIDTH), None);
    }

    #[test]
    fn two_whitespace_corridors_are_not_one_gutter_gh2102() {
        let mut spans = gh2102_margin_note_page(12, false);
        spans.retain(|span| !span.text.starts_with("main body"));
        for row in 0..8 {
            let y = 714.9 - row as f32 * 13.0;
            spans.push(span_with_width(
                &format!("main body {row} a"),
                GH2102_MAIN_X,
                y,
                160.0,
                10.0,
                10.0,
            ));
            spans.push(span_with_width(
                &format!("main body {row} b"),
                298.3,
                y,
                172.0,
                10.0,
                10.0,
            ));
        }
        spans.extend((0..12).map(|index| gh2102_note(index, 565.0, 731.0)));
        spans.sort_by(|a, b| {
            b.bbox
                .y
                .total_cmp(&a.bbox.y)
                .then_with(|| a.bbox.x.total_cmp(&b.bbox.x))
        });
        let order = spans_sorted_top_to_bottom(&spans);
        let lines = group_into_lines(&spans, &order);
        assert_eq!(
            gh2102_corridor_count(&spans, &lines),
            2,
            "the fixture must present two corridors, or this test would pass for the wrong reason"
        );

        assert_eq!(detect_split_x(&spans, &lines, GH2102_PAGE_WIDTH), None);
    }

    const GH2109_NOTE_COUNT: usize = 12;
    const GH2109_BODY_ROWS: usize = 10;
    const GH2109_BODY_WIDTH: f32 = 375.0;
    const GH2109_NOTE_WIDTH: f32 = 70.0;
    const GH2109_GUTTER: f32 = 13.0;

    /// GH#2109 geometry, taken from the reporter's page: a justified 10pt main column of one
    /// 375pt span per line (wider than `FULL_WIDTH_FURNITURE_FRACTION` of the page) at `body_x`,
    /// and 8pt notes at `note_x` on their own leading. No note line shares a baseline with a
    /// body line. Spans are in the Y-then-X order `ColumnAware` produced. ~keep
    fn gh2109_justified_body_with_notes(body_x: f32, note_x: f32) -> Vec<TextSpan> {
        let mut spans: Vec<TextSpan> = (0..GH2109_BODY_ROWS)
            .map(|row| {
                span_with_width(
                    &gh2109_body_text(row),
                    body_x,
                    741.9 - row as f32 * 13.0,
                    GH2109_BODY_WIDTH,
                    10.0,
                    10.0,
                )
            })
            .collect();
        spans.extend((0..GH2109_NOTE_COUNT).map(|index| {
            span_with_width(
                &format!("note line {index}"),
                note_x,
                741.6 - index as f32 * 9.6,
                GH2109_NOTE_WIDTH,
                8.0,
                8.0,
            )
        }));
        spans.sort_by(|a, b| {
            b.bbox
                .y
                .total_cmp(&a.bbox.y)
                .then_with(|| a.bbox.x.total_cmp(&b.bbox.x))
        });
        spans
    }

    fn gh2109_body_text(row: usize) -> String {
        format!("body line {row} {}", "justified running text ".repeat(3))
    }

    fn gh2109_expected(notes_first: bool) -> Vec<String> {
        let body = (0..GH2109_BODY_ROWS).map(gh2109_body_text);
        let notes = (0..GH2109_NOTE_COUNT).map(|index| format!("note line {index}"));
        if notes_first {
            notes.chain(body).collect()
        } else {
            body.chain(notes).collect()
        }
    }

    fn assert_reordered(mut spans: Vec<TextSpan>, notes_first: bool) {
        assert!(reorder_dense_two_column_page(&mut spans, GH2102_PAGE_WIDTH));
        let expected = gh2109_expected(notes_first);
        assert_eq!(
            spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
            expected.iter().map(String::as_str).collect::<Vec<_>>(),
        );
    }

    #[test]
    fn right_margin_notes_are_read_after_a_justified_main_column_gh2109() {
        let spans = gh2109_justified_body_with_notes(82.0, 82.0 + GH2109_BODY_WIDTH + GH2109_GUTTER);
        assert_reordered(spans, false);
    }

    #[test]
    fn left_margin_notes_are_read_before_a_justified_main_column_gh2109() {
        let spans = gh2109_justified_body_with_notes(82.0 + GH2109_NOTE_WIDTH + GH2109_GUTTER, 82.0);
        assert_reordered(spans, true);
    }
}
