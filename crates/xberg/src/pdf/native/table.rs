//! Native table detection using the xberg_native_pdf backend.
//!
//! Three entry points:
//!
//! - [`extract_tables_native`] — xberg_native_pdf's built-in `extract_tables_with_config`
//!   in strict mode. High precision, requires explicit table grid with 3+ columns.
//! - [`extract_tables_bordered`] — relaxed Lines-strategy pass for bordered tables
//!   with 2+ columns. Catches 2-column data tables (label/value) whose cells are
//!   delimited by stroke lines that `strict()` skips due to `min_table_columns=3`.
//! - [`extract_tables_heuristic`] — text-layer fallback that reuses the same
//!   spatial reconstruction the layout-detection path uses, but without
//!   requiring layout hints. Catches tables that xberg_native_pdf's grid detector
//!   misses (e.g. invoice line items, financial tables without ruling lines).
//!
//! The default extraction flow in `extractors::pdf::extraction` runs all three
//! in priority order (native → bordered → heuristic). Bordered detections add
//! regions not already represented by strict detections; heuristic detection
//! runs only on pages with neither strict nor bordered tables.

use super::NativeDocument;
use crate::pdf::error::{PdfError, Result};
use crate::pdf::table_reconstruct::table_to_markdown;
use crate::types::{BoundingBox, ProcessingWarning, Table};
use std::collections::HashSet;

#[derive(Debug)]
pub(crate) struct NativeTable {
    pub table: Table,
    pub grid: Option<crate::types::TableGrid>,
}

// ~keep Exact finite ruling geometry avoids discarding neighboring or nested tables.
fn table_region_key(table: &Table) -> Option<(u32, [u64; 4])> {
    let bbox = table.bounding_box?;
    let coordinates = [bbox.x0, bbox.y0, bbox.x1, bbox.y1];
    if !coordinates.iter().all(|value| value.is_finite()) || bbox.x0 >= bbox.x1 || bbox.y0 >= bbox.y1 {
        return None;
    }
    let bits = coordinates.map(|value| if value == 0.0 { 0 } else { value.to_bits() });
    Some((table.page_number, bits))
}

pub(crate) fn append_bordered_tables(strict: &mut Vec<NativeTable>, bordered: Vec<NativeTable>) {
    let strict_regions: HashSet<_> = strict
        .iter()
        .filter_map(|entry| table_region_key(&entry.table))
        .collect();
    strict.extend(
        bordered
            .into_iter()
            .filter(|entry| table_region_key(&entry.table).is_none_or(|region| !strict_regions.contains(&region))),
    );
}

/// Cap on candidate vertical regions per page. Real tables fit comfortably
/// under this; prose-heavy pages can otherwise generate dozens of small
/// regions that each go through `reconstruct_table` + `post_process_table`,
/// burning CPU on guaranteed rejections. The cap is generous on purpose —
/// the validation chain is fast.
const MAX_REGIONS_PER_PAGE: usize = 20;
const DENSE_NUMERIC_MIN_RECURRING_ROWS: usize = 5;
const DENSE_NUMERIC_MIN_WORDS_PER_ROW: usize = 3;
const DENSE_NUMERIC_MIN_ROW_WORD_PERCENT: usize = 60;
const DENSE_NUMERIC_MIN_RECURRING_TRACKS: usize = 4;
const DENSE_NUMERIC_MIN_TRACK_ROW_PERCENT: usize = 60;
const LABEL_HEAVY_FINANCIAL_MIN_ROWS: usize = 5;
const LABEL_HEAVY_FINANCIAL_TRACKS: usize = 4;
const LABEL_HEAVY_FINANCIAL_MIN_TRACK_ROW_PERCENT: usize = 30;
const LABEL_HEAVY_FINANCIAL_MIN_TRACK_ROWS: usize = 3;
const LABEL_HEAVY_FINANCIAL_MIN_DESCRIPTOR_ROW_PERCENT: usize = 50;
const LABEL_HEAVY_FINANCIAL_MIN_NUMERIC_ROWS: usize = 4;
const LABEL_HEAVY_FINANCIAL_MIN_VALUES_PER_ROW: usize = 3;
const LABEL_HEAVY_FINANCIAL_MAX_SECTION_GAP_HEIGHTS: u32 = 4;
const LABEL_HEAVY_FINANCIAL_MAX_LABEL_GAP_HEIGHTS: u32 = 2;
const NUMERIC_HEADER_MIN_TRACK_PERCENT: usize = 60;
const NUMERIC_HEADER_MIN_ALPHA_PERCENT: usize = 60;
// TODO: Support a third header row after column detection can derive anchors
// from recurring data tracks instead of header glyph positions.
// ~keep Header glyphs can bridge adjacent numeric tracks and collapse columns.
const NUMERIC_HEADER_MAX_ROWS: usize = 2;
const DENSE_NUMERIC_COLUMN_GAP_CAP: u32 = 20;
const SPLIT_NUMERIC_TRACK_MIN_ROWS_PER_SIDE: usize = 2;
const SPLIT_NUMERIC_TRACK_MIN_TOTAL_ROWS: usize = 6;
const SIDE_BY_SIDE_MIN_PARENT_COLUMNS: usize = 7;
const SIDE_BY_SIDE_CENTER_TOLERANCE_PERCENT: u64 = 8;
const SIDE_BY_SIDE_MIN_GUTTER_HEIGHT_PERCENT: u64 = 100;
const SIDE_BY_SIDE_MIN_WIDTH_PERCENT: u64 = 35;
const SIDE_BY_SIDE_MIN_WORDS_PER_SIDE: usize = 6;
const SIDE_BY_SIDE_CHILD_GAP_HEIGHT_MULTIPLIER: u32 = 6;
const SIDE_BY_SIDE_MIN_TRACK_ROW_PERCENT: usize = 20;
const SIDE_BY_SIDE_MIN_NUMERIC_TRACKS: usize = 2;
const SIDE_BY_SIDE_MAX_TRACK_DELTA: usize = 1;
const WRAPPED_FINANCIAL_COLUMNS: usize = 4;
const WRAPPED_FINANCIAL_DESCRIPTOR_COLUMNS: usize = 2;
const WRAPPED_FINANCIAL_PAR_COLUMN: usize = 2;
const WRAPPED_FINANCIAL_VALUE_COLUMN: usize = 3;
const CANONICAL_FINANCIAL_COLUMNS: usize = 3;
const CANONICAL_FINANCIAL_HEADER: [&str; CANONICAL_FINANCIAL_COLUMNS] = ["Description", "Amount", "Value"];
const FINANCIAL_ISSUER_SUFFIXES: [&str; 6] = ["Ltd.", "DAC", "LLC", "Inc.", "PLC", "Corp."];
const WRAPPED_FINANCIAL_MAX_CONTINUATION_ROWS: usize = 6;
const WRAPPED_FINANCIAL_MIN_CONTINUATION_ROWS: usize = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SideTableShape {
    columns: usize,
    numeric_tracks: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HeuristicTableRejection {
    EmptyGrid,
    PostProcessing,
    TooFewRows,
    CodeListing,
    NotWellFormed,
    EmptyMarkdown,
}

/// Extract tables from all pages using xberg_native_pdf's native table detection.
///
/// Uses `TableDetectionConfig::strict()` to reduce false positives from
/// paragraph text being misidentified as tables.
///
/// # Arguments
///
/// * `doc` - Mutable reference to the native document
///
/// # Returns
///
/// A `Vec<Table>` containing all detected tables with cells, markdown, and bounding boxes,
/// together with a `Vec<ProcessingWarning>` describing any pages whose native table detection
/// failed (issue #74). A per-page failure is skipped rather than aborting the whole document,
/// so without a warning that page would be silently indistinguishable from a page that simply
/// has no table.
pub(crate) fn extract_tables_native(doc: &mut NativeDocument) -> Result<(Vec<NativeTable>, Vec<ProcessingWarning>)> {
    let page_count = doc
        .doc
        .page_count()
        .map_err(|e| PdfError::MetadataExtractionFailed(format!("xberg_native_pdf: failed to get page count: {e}")))?;

    let config = xberg_native_pdf::structure::spatial_table_detector::TableDetectionConfig::strict();
    let mut all_tables = Vec::new();
    let mut warnings = Vec::new();

    for page_idx in 0..page_count {
        let page_number = (page_idx + 1) as u32;
        let extracted = match doc.doc.extract_tables_with_config(page_idx, config.clone()) {
            Ok(tables) => tables,
            Err(e) => {
                tracing::warn!(page = page_idx, "xberg_native_pdf extract_tables failed: {e}");
                warnings.push(native_table_extraction_failure_warning(page_number, &e));
                continue;
            }
        };

        for extracted_table in extracted {
            if extracted_table.rows.is_empty() || extracted_table.col_count == 0 {
                continue;
            }

            let (cells, markdown, grid) = convert_extracted_table(&extracted_table);

            if cells.is_empty() || markdown.trim().is_empty() {
                continue;
            }

            if cells.len() < 2 || cells.iter().all(|row| row.len() < 2) {
                tracing::debug!(
                    page = page_idx,
                    rows = cells.len(),
                    cols = cells.first().map(|r| r.len()).unwrap_or(0),
                    "Skipping table below minimum dimensions (need ≥2 rows and ≥2 cols)"
                );
                continue;
            }

            let bounding_box = extracted_table.bbox.map(|rect| BoundingBox {
                x0: rect.x as f64,
                y0: rect.y as f64,
                x1: (rect.x + rect.width) as f64,
                y1: (rect.y + rect.height) as f64,
            });

            all_tables.push(NativeTable {
                table: Table {
                    cells,
                    markdown,
                    page_number,
                    bounding_box,
                    ..Default::default()
                },
                grid,
            });
        }
    }

    Ok((all_tables, warnings))
}

/// Build the `ProcessingWarning` for a page whose native (strict-mode) table detection
/// failed (issue #74). Named per page and cause so the warning is actionable without
/// leaking internal file paths or system details.
fn native_table_extraction_failure_warning(page_number: u32, error: &xberg_native_pdf::Error) -> ProcessingWarning {
    ProcessingWarning {
        source: std::borrow::Cow::Borrowed("pdf_tables"),
        message: std::borrow::Cow::Owned(format!(
            "table extraction failed for page {page_number}: {error}; tables on this page were skipped"
        )),
    }
}

/// Extract bordered tables from all pages using a relaxed Lines-strategy config.
///
/// Targets 2-column tables whose cells are delimited by stroke/fill rectangles
/// (the `re` PDF operator or explicit `m/l` paths) that `extract_tables_native`
/// misses because `strict()` requires ≥ 3 columns.
///
/// Uses `TableStrategy::Lines` for both axes — path primitives only, no text-
/// edge heuristics — so this pass does not introduce new text-based false positives.
/// The relaxed thresholds:
///
/// - `min_table_columns = 2` (vs 3 in strict)
/// - `min_table_cells = 4` (vs 6 in strict)
/// - `regular_row_ratio = 0.5` (vs 0.8 in strict)
///
/// `skip_pages` (1-indexed) suppresses this pass on pages where a higher-priority
/// detector already produced a result. Pass an empty set to run on every page.
///
/// # Returns
///
/// A `Vec<Table>` of detected bordered tables, together with a `Vec<ProcessingWarning>`
/// describing any (non-skipped) pages whose bordered-table detection failed (issue #74).
pub(crate) fn extract_tables_bordered(
    doc: &mut NativeDocument,
    skip_pages: &HashSet<u32>,
) -> Result<(Vec<NativeTable>, Vec<ProcessingWarning>)> {
    use xberg_native_pdf::structure::spatial_table_detector::{TableDetectionConfig, TableStrategy};

    let page_count = doc
        .doc
        .page_count()
        .map_err(|e| PdfError::MetadataExtractionFailed(format!("xberg_native_pdf: failed to get page count: {e}")))?;

    let config = TableDetectionConfig {
        enabled: true,
        horizontal_strategy: TableStrategy::Lines,
        vertical_strategy: TableStrategy::Lines,
        column_tolerance: 3.0,
        row_tolerance: 2.0,
        min_table_cells: 4,
        min_table_columns: 2,
        regular_row_ratio: 0.5,
        max_table_columns: 15,
        column_merge_threshold: 12.0,
        v_split_gap: 4.0,
        text_fallback: false,
    };

    let mut all_tables = Vec::new();
    let mut warnings = Vec::new();

    for page_idx in 0..page_count {
        let page_number = (page_idx + 1) as u32;
        if skip_pages.contains(&page_number) {
            continue;
        }

        let extracted = match doc.doc.extract_tables_with_config(page_idx, config.clone()) {
            Ok(tables) => tables,
            Err(e) => {
                tracing::warn!(page = page_idx, "xberg_native_pdf bordered extract_tables failed: {e}");
                warnings.push(bordered_table_extraction_failure_warning(page_number, &e));
                continue;
            }
        };

        for extracted_table in extracted {
            if extracted_table.rows.is_empty() || extracted_table.col_count == 0 {
                continue;
            }

            let (cells, markdown, grid) = convert_extracted_table(&extracted_table);

            if cells.is_empty() || markdown.trim().is_empty() {
                continue;
            }

            if cells.len() < 2 || cells.iter().all(|row| row.len() < 2) {
                tracing::debug!(
                    page = page_idx,
                    rows = cells.len(),
                    cols = cells.first().map(|r| r.len()).unwrap_or(0),
                    "Skipping bordered table below minimum dimensions"
                );
                continue;
            }

            let bounding_box = extracted_table.bbox.map(|rect| BoundingBox {
                x0: rect.x as f64,
                y0: rect.y as f64,
                x1: (rect.x + rect.width) as f64,
                y1: (rect.y + rect.height) as f64,
            });

            all_tables.push(NativeTable {
                table: Table {
                    cells,
                    markdown,
                    page_number,
                    bounding_box,
                    ..Default::default()
                },
                grid,
            });
        }
    }

    Ok((all_tables, warnings))
}

/// Build the `ProcessingWarning` for a page whose bordered (relaxed Lines-strategy)
/// table detection failed (issue #74). Named per page and cause so the warning is
/// actionable without leaking internal file paths or system details.
fn bordered_table_extraction_failure_warning(page_number: u32, error: &xberg_native_pdf::Error) -> ProcessingWarning {
    ProcessingWarning {
        source: std::borrow::Cow::Borrowed("pdf_tables"),
        message: std::borrow::Cow::Owned(format!(
            "bordered table extraction failed for page {page_number}: {error}; bordered tables on \
             this page were skipped"
        )),
    }
}

/// Owned hierarchy segments produced while detecting heuristic tables.
///
/// Structured PDF rendering can consume these segments directly instead of
/// extracting the same hierarchy a second time.
pub(crate) struct ExtractedHierarchySegments {
    pub(crate) pages: Vec<Vec<crate::pdf::hierarchy::SegmentData>>,
    pub(crate) used_structure_tree: bool,
}

/// Heuristic tables and the hierarchy segments used to derive them.
pub(crate) struct HeuristicTableExtraction {
    pub(crate) tables: Vec<Table>,
    pub(crate) hierarchy_segments: ExtractedHierarchySegments,
}

/// Heuristic table reconstruction for text-layer PDFs without layout hints.
///
/// Pulls per-page text segments via the same hierarchy extractor used by the
/// structured pipeline, projects each segment into an `HocrWord`, clusters
/// words into vertically-contiguous regions by abnormally-large row gaps,
/// then runs the shared `reconstruct_table` → `post_process_table` →
/// `is_well_formed_table` chain (the same one consumed by the OCR pipeline
/// and the layout-detection path) per region.
///
/// The clustering step does NOT validate that a region is table-shaped — it
/// merely separates vertically-isolated word slabs, further split on any
/// dominant vertical corridor between column blocks. Multi-column prose
/// narrower than that corridor floor can still pass clustering as one wide
/// region; `is_well_formed_table` is the actual prose-rejection guard. We
/// pass `layout_guided = true` to
/// `post_process_table` so that "we already pre-segmented the page" earns
/// the relaxed text-density thresholds, but the prose-detection check in
/// `is_well_formed_table` (row coherence + column semantic uniformity) is
/// what catches columned articles and similar false positives.
///
/// `skip_pages` (1-indexed, matching `Table.page_number`) lets the caller
/// suppress heuristic work on pages where xberg_native_pdf's native grid detector
/// already produced results — those tables are higher precision than
/// anything the heuristic can derive, so we keep native and only fill in
/// the gaps. Pass an empty set to run on every page.
///
/// ~keep `artifacts` selects which tagged `/Artifact` spans enter the segments. It
/// must match what the structured pipeline wants, because the returned
/// segments are reused there instead of being extracted a second time.
///
/// This is invoked by `extractors::pdf::extraction` as a per-page fallback
/// alongside [`extract_tables_native`] — typically for text-layer PDFs
/// whose tables aren't drawn with explicit rule lines that xberg_native_pdf's grid
/// detector can lock onto.
///
/// Returns the detected tables together with ownership of the exact hierarchy
/// segments used for reconstruction.
pub(crate) fn extract_tables_heuristic(
    doc: &mut NativeDocument,
    allow_single_column: bool,
    skip_pages: &HashSet<u32>,
    artifacts: crate::pdf::native::hierarchy::ArtifactRetention,
) -> Result<HeuristicTableExtraction> {
    use crate::pdf::table_reconstruct::{HocrWord, segments_to_words};

    let (per_page_segments, used_structure_tree) = crate::pdf::native::hierarchy::extract_all_segments(doc, artifacts)
        .map_err(|e| {
            PdfError::TextExtractionFailed(format!(
                "xberg_native_pdf hierarchy extraction failed for heuristic tables: {e}"
            ))
        })?;

    let page_count = doc
        .doc
        .page_count()
        .map_err(|e| PdfError::MetadataExtractionFailed(format!("xberg_native_pdf: failed to get page count: {e}")))?;

    let mut tables = Vec::new();

    for page_idx in 0..page_count {
        let page_number = (page_idx + 1) as u32;
        if skip_pages.contains(&page_number) {
            continue;
        }

        let Some(segments) = per_page_segments.get(page_idx) else {
            continue;
        };
        if segments.is_empty() {
            continue;
        }

        let page_height = segments
            .iter()
            .map(|s| s.y + s.height)
            .fold(0.0_f32, f32::max)
            .max(792.0);

        let words: Vec<HocrWord> = segments_to_words(segments, page_height);
        if words.len() < 4 {
            continue;
        }

        let mut regions = cluster_words_into_vertical_regions(&words);
        if regions.len() > MAX_REGIONS_PER_PAGE {
            tracing::debug!(
                page = page_number,
                regions = regions.len(),
                cap = MAX_REGIONS_PER_PAGE,
                "heuristic table extraction: capping candidate regions on this page",
            );
            regions.truncate(MAX_REGIONS_PER_PAGE);
        }

        // Signal 1 of the #1399 admission test: a page whose producer drew
        // ruling lines is one where a table candidate is credible on its face.
        // One `extract_paths` call per page, reusing the layout gate's own rule
        // counter. A page whose paths cannot be read counts as unruled, which
        // is the conservative direction (the geometric gate still applies).
        let horizontal_rules = super::guard_native_panic(
            || doc.doc.extract_paths(page_idx).map_err(|error| error.to_string()),
            |message| message,
        )
        .ok()
        .map_or(0, |paths| crate::pdf::rules::count_rules(&paths).0);

        let page_tables_start = tables.len();
        tables.extend(reconstruct_page_region_tables(
            &regions,
            page_height,
            page_number,
            allow_single_column,
            horizontal_rules,
        ));

        // GH#1358: a page with a rotated run whose frame needed lifting (see
        // `table_reconstruct::page_has_lifted_rotation_frame`) produces
        // `HocrWord`s whose left/top live in that run's advance/cross axes,
        // not real page-space x/y. `region_bounding_box`'s page_height-only
        // inversion is exact for the unrotated identity mapping but for a
        // lifted frame back-projects to a rectangle in no real coordinate
        // system — comparing that against real `SegmentData::x`/`y` in
        // `filter_segments_by_table_bboxes` could delete unrelated prose it
        // spuriously overlaps. Drop the bounding box rather than trust a
        // fabricated one; the table's cell content is unaffected, and
        // downstream page assignment and dedup are bbox-agnostic.
        if crate::pdf::table_reconstruct::page_has_lifted_rotation_frame(segments, page_height) {
            for table in &mut tables[page_tables_start..] {
                table.bounding_box = None;
            }
        }
    }

    Ok(HeuristicTableExtraction {
        tables,
        hierarchy_segments: ExtractedHierarchySegments {
            pages: per_page_segments,
            used_structure_tree,
        },
    })
}

fn reconstruct_page_region_tables(
    regions: &[Vec<crate::pdf::table_reconstruct::HocrWord>],
    page_height: f32,
    page_number: u32,
    allow_single_column: bool,
    horizontal_rules: usize,
) -> Vec<Table> {
    let mut tables = Vec::new();
    let mut index = 0;
    while index < regions.len() {
        let Some((first, first_tracks, track_tolerance)) =
            reconstruct_label_heavy_financial_region(&regions[index], page_height, page_number)
        else {
            tables.extend(reconstruct_region_tables(
                &regions[index],
                page_height,
                page_number,
                allow_single_column,
                horizontal_rules,
            ));
            index += 1;
            continue;
        };

        let mut chain = vec![first];
        let mut chain_end = index + 1;
        while chain_end < regions.len() {
            let Some((next, next_tracks, next_tolerance)) =
                reconstruct_label_heavy_financial_region(&regions[chain_end], page_height, page_number)
            else {
                break;
            };
            let tolerance = track_tolerance.max(next_tolerance);
            if !first_tracks
                .iter()
                .zip(next_tracks.iter())
                .all(|(left, right)| left.abs_diff(*right) <= tolerance)
                || !label_heavy_financial_sections_are_contiguous(
                    &regions[chain_end - 1],
                    &regions[chain_end],
                    &next_tracks,
                    next_tolerance,
                )
            {
                break;
            }
            chain.push(next);
            chain_end += 1;
        }

        if chain.len() < 2 {
            tables.extend(reconstruct_region_tables(
                &regions[index],
                page_height,
                page_number,
                allow_single_column,
                horizontal_rules,
            ));
            index += 1;
            continue;
        }

        tables.push(stitch_label_heavy_financial_sections(chain, page_number));
        index = chain_end;
    }
    tables
}

fn label_heavy_financial_sections_are_contiguous(
    previous: &[crate::pdf::table_reconstruct::HocrWord],
    next: &[crate::pdf::table_reconstruct::HocrWord],
    next_tracks: &[u32],
    next_track_tolerance: u32,
) -> bool {
    let Some(previous_bottom) = previous.iter().map(|word| word.top + word.height).max() else {
        return false;
    };
    let Some(next_top) = next.iter().map(|word| word.top).min() else {
        return false;
    };
    let median_height = median_region_word_height(previous)
        .max(median_region_word_height(next))
        .max(1);
    let normalized_gap_limit = median_height.saturating_mul(LABEL_HEAVY_FINANCIAL_MAX_SECTION_GAP_HEIGHTS);
    next_top.saturating_sub(previous_bottom) <= normalized_gap_limit
        && starts_with_financial_section_label(next, next_tracks, next_track_tolerance)
}

fn starts_with_financial_section_label(
    region: &[crate::pdf::table_reconstruct::HocrWord],
    tracks: &[u32],
    track_tolerance: u32,
) -> bool {
    let row_tolerance = (median_region_word_height(region) / 2).max(3);
    let rows = numeric_rows(region, row_tolerance);
    let Some(first_row) = rows.first() else {
        return false;
    };
    let numeric_start = tracks[0].saturating_sub(track_tolerance.saturating_mul(2));
    if first_row.iter().any(|word| is_numeric_word(&word.text)) {
        return false;
    }

    let mut label_words = first_row
        .iter()
        .filter(|word| word.text.chars().any(char::is_alphabetic))
        .copied()
        .collect::<Vec<_>>();
    label_words.sort_by_key(|word| word.left);
    let Some(first_word) = label_words.first() else {
        return false;
    };
    if first_word.left + first_word.width / 2 >= numeric_start {
        return false;
    }

    let maximum_label_gap =
        median_region_word_height(region).saturating_mul(LABEL_HEAVY_FINANCIAL_MAX_LABEL_GAP_HEIGHTS);
    label_words
        .windows(2)
        .all(|pair| pair[1].left.saturating_sub(pair[0].left.saturating_add(pair[0].width)) <= maximum_label_gap)
}

fn median_region_word_height(region: &[crate::pdf::table_reconstruct::HocrWord]) -> u32 {
    if region.is_empty() {
        return 0;
    }
    let mut heights: Vec<u32> = region.iter().map(|word| word.height).collect();
    heights.sort_unstable();
    heights[heights.len() / 2]
}

fn reconstruct_label_heavy_financial_region(
    region: &[crate::pdf::table_reconstruct::HocrWord],
    page_height: f32,
    page_number: u32,
) -> Option<(Table, Vec<u32>, u32)> {
    let (tracks, row_tolerance, x_tolerance) = label_heavy_financial_tracks(region)?;
    let rows = numeric_rows(region, row_tolerance);
    let grid = build_label_heavy_financial_grid(rows, &tracks, x_tolerance);
    if !is_label_heavy_financial_grid(&grid) {
        return None;
    }

    let markdown = table_to_markdown(&grid);
    let table = Table {
        cells: grid,
        markdown,
        page_number,
        bounding_box: region_bounding_box(region, page_height),
        ..Default::default()
    };
    Some((table, tracks, x_tolerance))
}

fn build_label_heavy_financial_grid(
    rows: Vec<Vec<&crate::pdf::table_reconstruct::HocrWord>>,
    tracks: &[u32],
    x_tolerance: u32,
) -> Vec<Vec<String>> {
    let numeric_start = tracks[0].saturating_sub(x_tolerance.saturating_mul(2));
    rows.into_iter()
        .map(|mut row| {
            row.sort_by_key(|word| word.left);
            let descriptor_only = !row.iter().any(|word| is_numeric_word(&word.text))
                && row.iter().any(|word| word.left + word.width / 2 < numeric_start);
            let mut cells = vec![Vec::<String>::new(); LABEL_HEAVY_FINANCIAL_TRACKS + 1];
            for word in row {
                let center = word.left + word.width / 2;
                let column = if descriptor_only || center < numeric_start {
                    0
                } else {
                    tracks
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, track)| track.abs_diff(center))
                        .map_or(0, |(index, _)| index + 1)
                };
                cells[column].push(word.text.clone());
            }
            cells
                .into_iter()
                .enumerate()
                .map(|(column, words)| normalize_financial_cell(column, words.join(" ")))
                .collect()
        })
        .collect()
}

fn normalize_financial_cell(column: usize, text: String) -> String {
    if column > 0 && text == "?" {
        "—".to_string()
    } else {
        text
    }
}

fn label_heavy_financial_tracks(region: &[crate::pdf::table_reconstruct::HocrWord]) -> Option<(Vec<u32>, u32, u32)> {
    if region.is_empty() {
        return None;
    }
    let median_height = median_region_word_height(region).max(1);
    let row_tolerance = (median_height / 2).max(3);
    let x_tolerance = median_height.saturating_mul(2).max(12);
    let rows = numeric_rows(region, row_tolerance);
    if rows.len() < LABEL_HEAVY_FINANCIAL_MIN_ROWS {
        return None;
    }

    let candidates = supported_financial_track_centers(&rows, x_tolerance);
    if candidates.len() != LABEL_HEAVY_FINANCIAL_TRACKS {
        return None;
    }
    if !has_label_heavy_financial_evidence(&rows, &candidates, x_tolerance)
        || !financial_track_spacing_is_stable(&candidates)
    {
        return None;
    }

    Some((candidates, row_tolerance, x_tolerance))
}

fn supported_financial_track_centers(
    rows: &[Vec<&crate::pdf::table_reconstruct::HocrWord>],
    x_tolerance: u32,
) -> Vec<u32> {
    let minimum_support = rows
        .len()
        .saturating_mul(LABEL_HEAVY_FINANCIAL_MIN_TRACK_ROW_PERCENT)
        .div_ceil(100)
        .max(LABEL_HEAVY_FINANCIAL_MIN_TRACK_ROWS);
    let mut candidates: Vec<u32> = rows
        .iter()
        .flatten()
        .filter(|word| is_numeric_word(&word.text))
        .map(|word| word.left + word.width / 2)
        .filter(|candidate| financial_track_support(rows, *candidate, x_tolerance) >= minimum_support)
        .collect();
    candidates.sort_unstable();
    candidates.dedup_by(|left, right| left.abs_diff(*right) <= x_tolerance);
    candidates
}

fn financial_track_support(
    rows: &[Vec<&crate::pdf::table_reconstruct::HocrWord>],
    candidate: u32,
    x_tolerance: u32,
) -> usize {
    rows.iter()
        .filter(|row| row_has_financial_track(row, candidate, x_tolerance))
        .count()
}

fn row_has_financial_track(row: &[&crate::pdf::table_reconstruct::HocrWord], track: u32, x_tolerance: u32) -> bool {
    row.iter()
        .any(|word| is_numeric_word(&word.text) && (word.left + word.width / 2).abs_diff(track) <= x_tolerance)
}

fn has_label_heavy_financial_evidence(
    rows: &[Vec<&crate::pdf::table_reconstruct::HocrWord>],
    tracks: &[u32],
    x_tolerance: u32,
) -> bool {
    let numeric_start = tracks[0].saturating_sub(x_tolerance.saturating_mul(2));
    let descriptor_rows = rows
        .iter()
        .filter(|row| {
            row.iter()
                .any(|word| word.left + word.width / 2 < numeric_start && word.text.chars().any(char::is_alphabetic))
        })
        .count();
    let numeric_rows = rows
        .iter()
        .filter(|row| {
            tracks
                .iter()
                .filter(|track| row_has_financial_track(row, **track, x_tolerance))
                .count()
                >= LABEL_HEAVY_FINANCIAL_MIN_VALUES_PER_ROW
        })
        .count();
    descriptor_rows.saturating_mul(100)
        >= rows
            .len()
            .saturating_mul(LABEL_HEAVY_FINANCIAL_MIN_DESCRIPTOR_ROW_PERCENT)
        && numeric_rows >= LABEL_HEAVY_FINANCIAL_MIN_NUMERIC_ROWS
}

fn financial_track_spacing_is_stable(tracks: &[u32]) -> bool {
    let gaps: Vec<u32> = tracks.windows(2).map(|pair| pair[1] - pair[0]).collect();
    let Some(minimum_gap) = gaps.iter().copied().min() else {
        return false;
    };
    let Some(maximum_gap) = gaps.iter().copied().max() else {
        return false;
    };
    minimum_gap > 0 && maximum_gap <= minimum_gap.saturating_mul(2)
}

fn is_label_heavy_financial_grid(grid: &[Vec<String>]) -> bool {
    if grid.len() < LABEL_HEAVY_FINANCIAL_MIN_ROWS
        || grid.iter().any(|row| row.len() != LABEL_HEAVY_FINANCIAL_TRACKS + 1)
    {
        return false;
    }
    let numeric_rows = grid
        .iter()
        .filter(|row| {
            row.iter().skip(1).filter(|cell| is_numeric_word(cell)).count() >= LABEL_HEAVY_FINANCIAL_MIN_VALUES_PER_ROW
        })
        .count();
    numeric_rows >= LABEL_HEAVY_FINANCIAL_MIN_NUMERIC_ROWS
}

fn stitch_label_heavy_financial_sections(sections: Vec<Table>, page_number: u32) -> Table {
    let mut rows = Vec::new();
    let mut bounding_box: Option<BoundingBox> = None;
    for section in sections {
        rows.extend(section.cells);
        if let Some(section_box) = section.bounding_box {
            bounding_box = Some(match bounding_box {
                Some(mut combined) => {
                    combined.x0 = combined.x0.min(section_box.x0);
                    combined.y0 = combined.y0.min(section_box.y0);
                    combined.x1 = combined.x1.max(section_box.x1);
                    combined.y1 = combined.y1.max(section_box.y1);
                    combined
                }
                None => section_box,
            });
        }
    }
    Table {
        markdown: table_to_markdown(&rows),
        cells: rows,
        page_number,
        bounding_box,
        ..Default::default()
    }
}

fn reconstruct_region_tables(
    region: &[crate::pdf::table_reconstruct::HocrWord],
    page_height: f32,
    page_number: u32,
    allow_single_column: bool,
    horizontal_rules: usize,
) -> Vec<Table> {
    let Some(parent) =
        reconstruct_region_table(region, page_height, page_number, allow_single_column, horizontal_rules)
    else {
        return Vec::new();
    };
    if parent.cells.first().map_or(0, Vec::len) < SIDE_BY_SIDE_MIN_PARENT_COLUMNS {
        return vec![parent];
    }

    let Some((left_region, right_region)) = split_side_by_side_region(region) else {
        return vec![parent];
    };
    let Some(left) = reconstruct_side_by_side_child(
        &left_region,
        page_height,
        page_number,
        allow_single_column,
        horizontal_rules,
    ) else {
        return vec![parent];
    };
    let Some(right) = reconstruct_side_by_side_child(
        &right_region,
        page_height,
        page_number,
        allow_single_column,
        horizontal_rules,
    ) else {
        return vec![parent];
    };
    if !side_tables_have_independent_shape(&left, &right) {
        return vec![parent];
    }

    let (left, right) = normalize_side_by_side_financial_tables(left, right);
    tracing::trace!(
        page = page_number,
        parent_columns = parent.cells.first().map_or(0, Vec::len),
        left_columns = left.cells.first().map_or(0, Vec::len),
        right_columns = right.cells.first().map_or(0, Vec::len),
        "split independently valid side-by-side heuristic tables"
    );
    vec![left, right]
}

fn reconstruct_side_by_side_child(
    region: &[crate::pdf::table_reconstruct::HocrWord],
    page_height: f32,
    page_number: u32,
    allow_single_column: bool,
    horizontal_rules: usize,
) -> Option<Table> {
    let mut heights: Vec<u32> = region.iter().map(|word| word.height).collect();
    heights.sort_unstable();
    let child_gap = heights
        .get(heights.len() / 2)
        .copied()?
        .max(1)
        .saturating_mul(SIDE_BY_SIDE_CHILD_GAP_HEIGHT_MULTIPLIER);
    reconstruct_region_table_with_column_gap(
        region,
        page_height,
        page_number,
        allow_single_column,
        child_gap,
        horizontal_rules,
    )
    .ok()
}

fn normalize_side_by_side_financial_tables(mut left: Table, mut right: Table) -> (Table, Table) {
    let paired_wrapped_tables = is_wrapped_financial_side_table(&left) && is_wrapped_financial_side_table(&right);
    let has_strict_pseudo_header = has_empty_numeric_pseudo_header(&left) || has_empty_numeric_pseudo_header(&right);
    let has_overflow_pseudo_header =
        has_constrained_overflow_pseudo_header(&left) || has_constrained_overflow_pseudo_header(&right);
    if paired_wrapped_tables && has_strict_pseudo_header && has_overflow_pseudo_header {
        normalize_wrapped_financial_side_table(&mut left);
        normalize_wrapped_financial_side_table(&mut right);
    }
    (left, right)
}

fn is_wrapped_financial_side_table(table: &Table) -> bool {
    if !table
        .cells
        .first()
        .is_some_and(|row| !is_explicit_financial_header(row) && has_wrapped_pseudo_header_evidence(row))
    {
        return false;
    }
    let Some(rows) = table.cells.get(1..) else {
        return false;
    };
    if table.cells.first().map_or(0, Vec::len) != WRAPPED_FINANCIAL_COLUMNS
        || rows.is_empty()
        || rows.iter().any(|row| row.len() != WRAPPED_FINANCIAL_COLUMNS)
    {
        return false;
    }

    let min_support = rows
        .len()
        .saturating_mul(SIDE_BY_SIDE_MIN_TRACK_ROW_PERCENT)
        .div_ceil(100)
        .max(2);
    let leading_descriptors = (0..WRAPPED_FINANCIAL_DESCRIPTOR_COLUMNS)
        .all(|column| rows.iter().filter(|row| is_descriptor_cell(&row[column])).count() >= min_support);
    let trailing_numeric = (WRAPPED_FINANCIAL_DESCRIPTOR_COLUMNS..WRAPPED_FINANCIAL_COLUMNS)
        .all(|column| rows.iter().filter(|row| is_numeric_word(&row[column])).count() >= min_support);

    leading_descriptors && trailing_numeric && has_bounded_financial_continuation(rows)
}

fn has_empty_numeric_pseudo_header(table: &Table) -> bool {
    table
        .cells
        .first()
        .is_some_and(|row| is_empty_numeric_pseudo_header(row))
}

fn has_constrained_overflow_pseudo_header(table: &Table) -> bool {
    table
        .cells
        .first()
        .is_some_and(|row| is_constrained_overflow_pseudo_header(row))
}

fn has_wrapped_pseudo_header_evidence(row: &[String]) -> bool {
    is_empty_numeric_pseudo_header(row) || is_constrained_overflow_pseudo_header(row)
}

fn has_populated_pseudo_header_descriptors(row: &[String]) -> bool {
    row.len() == WRAPPED_FINANCIAL_COLUMNS
        && row[..WRAPPED_FINANCIAL_DESCRIPTOR_COLUMNS]
            .iter()
            .all(|value| !value.trim().is_empty())
}

fn is_empty_numeric_pseudo_header(row: &[String]) -> bool {
    has_populated_pseudo_header_descriptors(row)
        && row[WRAPPED_FINANCIAL_DESCRIPTOR_COLUMNS..]
            .iter()
            .all(|value| value.trim().is_empty())
}

fn is_constrained_overflow_pseudo_header(row: &[String]) -> bool {
    if !has_populated_pseudo_header_descriptors(row) {
        return false;
    }
    let overflow = row[WRAPPED_FINANCIAL_PAR_COLUMN].trim();
    row[WRAPPED_FINANCIAL_VALUE_COLUMN].trim().is_empty()
        && !overflow.is_empty()
        && overflow.contains('%')
        && overflow.chars().any(char::is_alphabetic)
}

fn has_bounded_financial_continuation(rows: &[Vec<String>]) -> bool {
    (0..rows.len()).any(|start| financial_continuation_end(rows, start).is_some())
}

fn normalize_wrapped_financial_side_table(table: &mut Table) {
    let mut rows = Vec::with_capacity(table.cells.len() + 1);
    let Some(pseudo_header) = table.cells.first() else {
        return;
    };
    rows.push(CANONICAL_FINANCIAL_HEADER.map(str::to_string).to_vec());
    rows.push(canonicalize_wrapped_financial_row(pseudo_header));

    let mut index = 1;
    while index < table.cells.len() {
        let row = &table.cells[index];
        let Some(end) = financial_continuation_end(&table.cells, index) else {
            rows.push(canonicalize_wrapped_financial_row(row));
            index += 1;
            continue;
        };
        let Some(collapsed) = collapse_financial_rows(&table.cells[index..=end]) else {
            rows.extend(
                table.cells[index..=end]
                    .iter()
                    .map(|row| canonicalize_wrapped_financial_row(row)),
            );
            index = end + 1;
            continue;
        };
        rows.push(canonicalize_wrapped_financial_row(&collapsed));
        index = end + 1;
    }

    preserve_repeated_issuer_rows(&mut rows);
    table.cells = rows;
    table.markdown = crate::pdf::table_reconstruct::table_to_markdown(&table.cells);
}

/// Splits an unpunctuated legal-entity prefix only when the next row repeats `Series`.
///
/// The missing comma is reconstruction evidence: punctuated securities such as
/// `RR 28 Ltd., Series 2024-A` intentionally remain a single description.
fn preserve_repeated_issuer_rows(rows: &mut Vec<Vec<String>>) {
    let mut index = 1;
    while index + 1 < rows.len() {
        let descriptor = rows[index][0].clone();
        let Some((issuer, first_series)) = descriptor.split_once(" Series ") else {
            index += 1;
            continue;
        };
        let next_is_series = rows[index + 1][0].trim_start().starts_with("Series ");
        let issuer_has_legal_suffix = FINANCIAL_ISSUER_SUFFIXES.iter().any(|suffix| issuer.ends_with(suffix));
        if !next_is_series || !issuer_has_legal_suffix {
            index += 1;
            continue;
        }

        rows[index][0] = format!("Series {first_series}");
        rows.insert(index, vec![issuer.to_string(), String::new(), String::new()]);
        index += 2;
    }
}

fn canonicalize_wrapped_financial_row(row: &[String]) -> Vec<String> {
    let descriptor = row
        .iter()
        .take(WRAPPED_FINANCIAL_DESCRIPTOR_COLUMNS)
        .map(|cell| cell.trim())
        .filter(|cell| !cell.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    vec![
        descriptor,
        row.get(WRAPPED_FINANCIAL_PAR_COLUMN).cloned().unwrap_or_default(),
        row.get(WRAPPED_FINANCIAL_VALUE_COLUMN).cloned().unwrap_or_default(),
    ]
}

fn financial_row_has_numeric_values(row: &[String]) -> bool {
    row.get(WRAPPED_FINANCIAL_DESCRIPTOR_COLUMNS..)
        .is_some_and(|values| values.iter().any(|value| is_numeric_word(value)))
}

fn financial_row_has_wrapped_descriptors(row: &[String]) -> bool {
    row.get(..WRAPPED_FINANCIAL_DESCRIPTOR_COLUMNS)
        .is_some_and(|values| values.iter().all(|value| !value.trim().is_empty()))
}

fn financial_row_is_continuation(row: &[String]) -> bool {
    financial_row_has_wrapped_descriptors(row)
        && row
            .get(WRAPPED_FINANCIAL_DESCRIPTOR_COLUMNS..)
            .is_some_and(|values| values.iter().all(|value| value.trim().is_empty()))
}

fn is_explicit_financial_header(row: &[String]) -> bool {
    row.len() == WRAPPED_FINANCIAL_COLUMNS
        && row.iter().all(|cell| !cell.trim().is_empty())
        && row[..WRAPPED_FINANCIAL_DESCRIPTOR_COLUMNS]
            .iter()
            .all(|cell| is_descriptor_cell(cell))
        && row[WRAPPED_FINANCIAL_DESCRIPTOR_COLUMNS..]
            .iter()
            .all(|cell| is_descriptor_cell(cell))
}

fn financial_continuation_end(rows: &[Vec<String>], start: usize) -> Option<usize> {
    let mut continuation_rows = 0;
    for (index, row) in rows.iter().enumerate().skip(start) {
        if financial_row_is_continuation(row) {
            continuation_rows += 1;
            if continuation_rows > WRAPPED_FINANCIAL_MAX_CONTINUATION_ROWS {
                return None;
            }
            continue;
        }
        return ((WRAPPED_FINANCIAL_MIN_CONTINUATION_ROWS..=WRAPPED_FINANCIAL_MAX_CONTINUATION_ROWS)
            .contains(&continuation_rows)
            && financial_row_has_numeric_values(row))
        .then_some(index);
    }
    None
}

fn collapse_financial_rows(rows: &[Vec<String>]) -> Option<Vec<String>> {
    let terminal = rows.last()?;
    let descriptor = rows
        .iter()
        .flat_map(|row| row.iter().take(WRAPPED_FINANCIAL_DESCRIPTOR_COLUMNS))
        .map(|cell| cell.trim())
        .filter(|cell| !cell.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    Some(vec![
        descriptor,
        String::new(),
        terminal.get(2)?.clone(),
        terminal.get(3)?.clone(),
    ])
}

fn side_tables_have_independent_shape(left: &Table, right: &Table) -> bool {
    let Some(left_shape) = side_table_shape(left) else {
        return false;
    };
    let Some(right_shape) = side_table_shape(right) else {
        return false;
    };
    left_shape.columns.abs_diff(right_shape.columns) <= SIDE_BY_SIDE_MAX_TRACK_DELTA
        && left_shape.numeric_tracks.abs_diff(right_shape.numeric_tracks) <= SIDE_BY_SIDE_MAX_TRACK_DELTA
}

fn side_table_shape(table: &Table) -> Option<SideTableShape> {
    let columns = table.cells.first()?.len();
    let rows = table.cells.get(1..)?;
    if rows.is_empty() || rows.iter().any(|row| row.len() != columns) {
        return None;
    }
    let min_support = rows
        .len()
        .saturating_mul(SIDE_BY_SIDE_MIN_TRACK_ROW_PERCENT)
        .div_ceil(100)
        .max(2);
    let descriptor_tracks = (0..columns)
        .filter(|column| rows.iter().filter(|row| is_descriptor_cell(&row[*column])).count() >= min_support)
        .count();
    let numeric_tracks = (0..columns)
        .filter(|column| rows.iter().filter(|row| is_numeric_word(&row[*column])).count() >= min_support)
        .count();
    (descriptor_tracks >= 1 && numeric_tracks >= SIDE_BY_SIDE_MIN_NUMERIC_TRACKS).then_some(SideTableShape {
        columns,
        numeric_tracks,
    })
}

fn is_descriptor_cell(text: &str) -> bool {
    !is_numeric_word(text) && text.chars().filter(|character| character.is_alphabetic()).count() >= 3
}

fn split_side_by_side_region(
    region: &[crate::pdf::table_reconstruct::HocrWord],
) -> Option<(
    Vec<crate::pdf::table_reconstruct::HocrWord>,
    Vec<crate::pdf::table_reconstruct::HocrWord>,
)> {
    if region.len() < SIDE_BY_SIDE_MIN_WORDS_PER_SIDE.saturating_mul(2) {
        return None;
    }

    let (region_left, region_right) = region_horizontal_bounds(region)?;
    let region_width = region_right.saturating_sub(region_left);
    if region_width == 0 {
        return None;
    }
    let seam = central_region_seam(region, region_left, region_width)?;
    let (left, right) = partition_region_at_seam(region, seam)?;
    if left.len() < SIDE_BY_SIDE_MIN_WORDS_PER_SIDE || right.len() < SIDE_BY_SIDE_MIN_WORDS_PER_SIDE {
        return None;
    }
    balanced_region_sides(&left, &right, region_left, region_right, region_width).then_some((left, right))
}

fn region_horizontal_bounds(region: &[crate::pdf::table_reconstruct::HocrWord]) -> Option<(u32, u32)> {
    Some((
        region.iter().map(|word| word.left).min()?,
        region.iter().map(|word| word.left.saturating_add(word.width)).max()?,
    ))
}

fn central_region_seam(
    region: &[crate::pdf::table_reconstruct::HocrWord],
    region_left: u32,
    region_width: u32,
) -> Option<u64> {
    let region_center = u64::from(region_left) + u64::from(region_width) / 2;
    let tolerance = u64::from(region_width).saturating_mul(SIDE_BY_SIDE_CENTER_TOLERANCE_PERCENT) / 100;
    let min_gutter = u64::from(median_word_height(region)).saturating_mul(SIDE_BY_SIDE_MIN_GUTTER_HEIGHT_PERCENT) / 100;
    merged_horizontal_intervals(region)
        .windows(2)
        .filter_map(|pair| {
            let gap = u64::from(pair[1].0.saturating_sub(pair[0].1));
            let center = (u64::from(pair[0].1) + u64::from(pair[1].0)) / 2;
            (gap >= min_gutter && center.abs_diff(region_center) <= tolerance).then_some((gap, center))
        })
        .max_by_key(|(gap, _)| *gap)
        .map(|(_, center)| center)
}

fn merged_horizontal_intervals(region: &[crate::pdf::table_reconstruct::HocrWord]) -> Vec<(u32, u32)> {
    let mut intervals: Vec<_> = region
        .iter()
        .map(|word| (word.left, word.left.saturating_add(word.width)))
        .collect();
    intervals.sort_unstable_by_key(|interval| interval.0);
    let mut merged = Vec::<(u32, u32)>::new();
    for interval in intervals {
        match merged.last_mut() {
            Some(previous) if interval.0 <= previous.1 => previous.1 = previous.1.max(interval.1),
            _ => merged.push(interval),
        }
    }
    merged
}

fn median_word_height(region: &[crate::pdf::table_reconstruct::HocrWord]) -> u32 {
    let mut heights: Vec<u32> = region.iter().map(|word| word.height).collect();
    heights.sort_unstable();
    heights.get(heights.len() / 2).copied().unwrap_or(1).max(1)
}

fn partition_region_at_seam(
    region: &[crate::pdf::table_reconstruct::HocrWord],
    seam: u64,
) -> Option<(
    Vec<crate::pdf::table_reconstruct::HocrWord>,
    Vec<crate::pdf::table_reconstruct::HocrWord>,
)> {
    let mut left = Vec::new();
    let mut right = Vec::new();
    for word in region {
        if u64::from(word.left.saturating_add(word.width)) <= seam {
            left.push(word.clone());
        } else if u64::from(word.left) >= seam {
            right.push(word.clone());
        } else {
            return None;
        }
    }
    Some((left, right))
}

fn balanced_region_sides(
    left: &[crate::pdf::table_reconstruct::HocrWord],
    right: &[crate::pdf::table_reconstruct::HocrWord],
    region_left: u32,
    region_right: u32,
    region_width: u32,
) -> bool {
    let left_right = left.iter().map(|word| word.left.saturating_add(word.width)).max();
    let right_left = right.iter().map(|word| word.left).min();
    let (Some(left_right), Some(right_left)) = (left_right, right_left) else {
        return false;
    };
    let left_width = u64::from(left_right.saturating_sub(region_left));
    let right_width = u64::from(region_right.saturating_sub(right_left));
    let minimum = u64::from(region_width).saturating_mul(SIDE_BY_SIDE_MIN_WIDTH_PERCENT) / 100;
    left_width >= minimum && right_width >= minimum
}

/// Multiplier on the page's median word height above which the vertical gap
/// between two consecutive row centres starts a new region.
const ROW_GAP_SPLIT_HEIGHT_FACTOR: f32 = 1.8;

/// Slack added to the row-gap split threshold to absorb the rounding of
/// `HocrWord::top` (xberg-io/xberg#1760). `segments_to_words` rounds each
/// segment's image-space top to a whole unit, so a regular body pitch such as
/// 10.45 is observed as an alternating 10/11 while the threshold for a median
/// word height of 6 is 10.8: every 11 cuts and every 10 does not, putting the
/// region boundary wherever the rounding happens to land rather than where the
/// page's layout changes. Both endpoints carry up to half a unit of rounding
/// error, so an observed gap can overstate the real one by up to a whole unit;
/// requiring the gap's lower bound to clear the threshold keeps a regular pitch
/// in one region, where the post-processing prose tests can judge it. ~keep
const ROW_CENTER_ROUNDING_SLACK: f32 = 1.0;

/// Minimum width, as a multiple of the page's median word height, of a vertical corridor
/// that may split one region into independent column blocks. A floor only — see
/// COLUMN_BLOCK_CORRIDOR_DOMINANCE_PERCENT for what actually distinguishes a page gutter
/// from a table's own inter-column corridor. ~keep
const COLUMN_BLOCK_MIN_CORRIDOR_HEIGHTS: f32 = 2.375;

/// How much wider (percent) the splitting corridor must be than the next-widest corridor
/// of the same region that is also crossed on no row. A genuine table's inter-column
/// corridors are near-uniform (regular column pitch), so a page gutter stands out as an
/// outlier among a region's corridors while a table's own gutters do not. ~keep
const COLUMN_BLOCK_CORRIDOR_DOMINANCE_PERCENT: u64 = 150;

/// Minimum count of both descriptor and numeric words required on each side of a
/// corridor. This prevents a financial table's label/value gutter from being
/// mistaken for a page-column boundary. ~keep
const COLUMN_BLOCK_MIN_SEMANTIC_WORDS: usize = 3;
/// A page-column split should not cut through a dense run of shared table rows. ~keep
const COLUMN_BLOCK_MIN_SYNCHRONOUS_ROWS: usize = 6;
const COLUMN_BLOCK_SYNCHRONOUS_ROW_PERCENT: usize = 90;

/// Tolerance, in points, for collapsing word left edges onto one column track. Matches the
/// region filter's own existing dedup width (previously an unnamed literal 8). ~keep
const REGION_X_TRACK_TOLERANCE: u32 = 8;

/// Vertical centre of a word, carried as `f32` so the half unit that
/// `HocrWord`'s integer `height / 2` truncates does not shift a row's centre
/// relative to rows of a different height (xberg-io/xberg#1760).
fn row_center(word: &crate::pdf::table_reconstruct::HocrWord) -> f32 {
    word.top as f32 + word.height as f32 / 2.0
}

/// Cluster words on a single page into vertically-contiguous regions.
///
/// Splits the page on row gaps that are abnormally large compared to the
/// median word height, then further splits each resulting region on a
/// dominant vertical corridor via `split_regions_at_column_corridors` — a
/// page gutter separating a table from an adjacent prose column, which the
/// y-gap test alone cannot see (xberg-io/xberg#1769). Each region is a slab
/// of words that may form a table; the caller validates with
/// `reconstruct_table` + `post_process_table`.
fn cluster_words_into_vertical_regions(
    words: &[crate::pdf::table_reconstruct::HocrWord],
) -> Vec<Vec<crate::pdf::table_reconstruct::HocrWord>> {
    if words.len() < 4 {
        return Vec::new();
    }

    let mut heights: Vec<u32> = words.iter().map(|w| w.height).collect();
    heights.sort_unstable();
    let median_height = heights[heights.len() / 2].max(1);
    let row_tolerance = (median_height / 2).max(3);
    let row_tolerance_f = row_tolerance as f32;
    let row_gap_split = median_height as f32 * ROW_GAP_SPLIT_HEIGHT_FACTOR + ROW_CENTER_ROUNDING_SLACK;

    let mut sorted = words.to_vec();
    sorted.sort_by(|left, right| row_center(left).total_cmp(&row_center(right)));

    let mut regions: Vec<Vec<crate::pdf::table_reconstruct::HocrWord>> = Vec::new();
    let mut current: Vec<crate::pdf::table_reconstruct::HocrWord> = Vec::new();
    let mut last_row_yc: Option<f32> = None;

    let mut idx = 0;
    while idx < sorted.len() {
        let row_yc = row_center(&sorted[idx]);
        let mut end = idx + 1;
        while end < sorted.len() {
            if (row_center(&sorted[end]) - row_yc).abs() <= row_tolerance_f {
                end += 1;
            } else {
                break;
            }
        }

        if let Some(prev_yc) = last_row_yc
            && row_yc - prev_yc > row_gap_split
            && !current.is_empty()
        {
            regions.push(std::mem::take(&mut current));
        }
        current.extend(sorted[idx..end].iter().cloned());
        last_row_yc = Some(row_yc);
        idx = end;
    }
    if !current.is_empty() {
        regions.push(current);
    }

    split_regions_at_column_corridors(&mut regions, median_height);

    attach_aligned_numeric_headers(&mut regions, median_height, row_tolerance);

    regions.retain(|r| {
        if r.len() < 4 {
            return false;
        }
        let mut row_ycs: Vec<f32> = r.iter().map(row_center).collect();
        row_ycs.sort_by(|left, right| left.total_cmp(right));
        row_ycs.dedup_by(|a, b| (*a - *b).abs() <= row_tolerance_f);
        if row_ycs.len() < 3 {
            return false;
        }
        let mut xs: Vec<u32> = r.iter().map(|w| w.left).collect();
        xs.sort_unstable();
        xs.dedup_by(|a, b| a.abs_diff(*b) <= REGION_X_TRACK_TOLERANCE);
        xs.len() >= 2
    });

    regions
}

/// Further split each y-clustered region on a dominant vertical corridor — a gap in x
/// that no word's horizontal interval crosses on any row of the region. A page gutter
/// between a table and an adjacent prose column produces exactly this shape: the y-gap
/// test in `cluster_words_into_vertical_regions` has no x term, so a table whose row
/// pitch is close to a neighbouring column's line pitch never opens a wide enough
/// vertical gap to separate them (xberg-io/xberg#1769). This pass runs after that
/// clustering and before the region-shape retain filter. ~keep
fn split_regions_at_column_corridors(
    regions: &mut Vec<Vec<crate::pdf::table_reconstruct::HocrWord>>,
    median_height: u32,
) {
    let mut index = 0;
    while index < regions.len() && regions.len() < MAX_REGIONS_PER_PAGE {
        match split_region_at_column_corridor(&regions[index], median_height) {
            Some((left, right)) => {
                regions[index] = left;
                regions.push(right);
            }
            None => index += 1,
        }
    }
}

/// Split `region` at its single dominant vertical corridor, if one exists. Returns
/// `None` when the region has no corridor, when the widest corridor does not clear the
/// height-relative floor, when it is not clearly wider than the next-widest corridor
/// (a genuine table's own inter-column gutters are near-uniform), or when either side
/// of the split would not itself look like an independent column block. ~keep
fn split_region_at_column_corridor(
    region: &[crate::pdf::table_reconstruct::HocrWord],
    median_height: u32,
) -> Option<(
    Vec<crate::pdf::table_reconstruct::HocrWord>,
    Vec<crate::pdf::table_reconstruct::HocrWord>,
)> {
    let intervals = merged_horizontal_intervals(region);
    if intervals.len() < 2 {
        return None;
    }
    let mut corridors: Vec<(u32, u32)> = intervals
        .windows(2)
        .map(|pair| {
            let width = pair[1].0.saturating_sub(pair[0].1);
            (width, pair[0].1 + width / 2)
        })
        .collect();
    corridors.sort_unstable_by_key(|&(width, _)| std::cmp::Reverse(width));
    let (widest, seam) = *corridors.first()?;

    let minimum_corridor = (median_height.max(1) as f32 * COLUMN_BLOCK_MIN_CORRIDOR_HEIGHTS).ceil() as u32;
    if widest < minimum_corridor {
        return None;
    }
    let runner_up = corridors.get(1).map_or(0, |&(width, _)| width);
    if u64::from(widest).saturating_mul(100)
        < u64::from(runner_up).saturating_mul(COLUMN_BLOCK_CORRIDOR_DOMINANCE_PERCENT)
    {
        return None;
    }

    let (left, right) = partition_region_at_seam(region, u64::from(seam))?;
    if left.len() < SIDE_BY_SIDE_MIN_WORDS_PER_SIDE || right.len() < SIDE_BY_SIDE_MIN_WORDS_PER_SIDE {
        return None;
    }
    if !has_independent_column_tracks(&left) || !has_independent_column_tracks(&right) {
        return None;
    }
    if !has_mixed_label_value_evidence(&left) || !has_mixed_label_value_evidence(&right) {
        return None;
    }
    if has_synchronous_rows_across_seam(region, seam, median_height) {
        return None;
    }
    Some((left, right))
}

fn has_synchronous_rows_across_seam(
    region: &[crate::pdf::table_reconstruct::HocrWord],
    seam: u32,
    median_height: u32,
) -> bool {
    let row_tolerance = (median_height / 2).max(3);
    let rows = numeric_rows(region, row_tolerance);
    if rows.len() < COLUMN_BLOCK_MIN_SYNCHRONOUS_ROWS {
        return false;
    }
    let synchronous_rows = rows
        .iter()
        .filter(|row| {
            row.iter().any(|word| word.left.saturating_add(word.width) <= seam)
                && row.iter().any(|word| word.left >= seam)
        })
        .count();
    synchronous_rows.saturating_mul(100) >= rows.len().saturating_mul(COLUMN_BLOCK_SYNCHRONOUS_ROW_PERCENT)
}

/// Whether `side` contains at least two distinct column tracks (word left edges more
/// than [`REGION_X_TRACK_TOLERANCE`] apart). A single-track side is a lone label or
/// value column, not an independent column block — splitting a genuine two-column
/// borderless table at its only gutter would otherwise pass the corridor checks above
/// vacuously, since a single corridor has no runner-up to compare against. ~keep
fn has_independent_column_tracks(side: &[crate::pdf::table_reconstruct::HocrWord]) -> bool {
    let mut lefts: Vec<u32> = side.iter().map(|word| word.left).collect();
    lefts.sort_unstable();
    lefts.dedup_by(|a, b| a.abs_diff(*b) <= REGION_X_TRACK_TOLERANCE);
    lefts.len() >= 2
}

/// A page column can contain prose labels and values; a table split at its
/// label/value gutter leaves one of those kinds entirely or nearly absent. ~keep
fn has_mixed_label_value_evidence(side: &[crate::pdf::table_reconstruct::HocrWord]) -> bool {
    let descriptor_count = side.iter().filter(|word| is_descriptor_cell(&word.text)).count();
    let numeric_count = side.iter().filter(|word| is_numeric_word(&word.text)).count();
    descriptor_count >= COLUMN_BLOCK_MIN_SEMANTIC_WORDS && numeric_count >= COLUMN_BLOCK_MIN_SEMANTIC_WORDS
}

fn attach_aligned_numeric_headers(
    regions: &mut Vec<Vec<crate::pdf::table_reconstruct::HocrWord>>,
    median_height: u32,
    row_tolerance: u32,
) {
    let mut index = 1;
    while index < regions.len() {
        if is_aligned_numeric_header(&regions[index - 1], &regions[index], median_height, row_tolerance) {
            let data = regions.remove(index);
            regions[index - 1].extend(data);
            tracing::trace!(
                index = index - 1,
                "attached aligned header to compact numeric table region"
            );
        } else {
            index += 1;
        }
    }
}

fn is_aligned_numeric_header(
    header: &[crate::pdf::table_reconstruct::HocrWord],
    data: &[crate::pdf::table_reconstruct::HocrWord],
    median_height: u32,
    row_tolerance: u32,
) -> bool {
    let data_rows = numeric_rows(data, row_tolerance);
    let Some(recurring_rows) = longest_recurring_numeric_run(&data_rows) else {
        return false;
    };
    let tracks = recurring_numeric_track_centers(recurring_rows, median_height);
    let header_rows = numeric_rows(header, row_tolerance).len();
    if tracks.len() < DENSE_NUMERIC_MIN_RECURRING_TRACKS || !(1..=NUMERIC_HEADER_MAX_ROWS).contains(&header_rows) {
        return false;
    }

    let alpha_words = header
        .iter()
        .filter(|word| word.text.chars().any(char::is_alphabetic))
        .count();
    if alpha_words.saturating_mul(100) < header.len().saturating_mul(NUMERIC_HEADER_MIN_ALPHA_PERCENT) {
        return false;
    }

    let header_bottom = header.iter().map(|word| word.top + word.height).max().unwrap_or(0);
    let data_top = data.iter().map(|word| word.top).min().unwrap_or(u32::MAX);
    if data_top.saturating_sub(header_bottom) > median_height.saturating_mul(2) {
        return false;
    }

    let x_tolerance = median_height.saturating_mul(2).max(12);
    let matched_tracks = tracks
        .iter()
        .filter(|track| {
            header
                .iter()
                .any(|word| (word.left + word.width / 2).abs_diff(**track) <= x_tolerance)
        })
        .count();
    matched_tracks.saturating_mul(100) >= tracks.len().saturating_mul(NUMERIC_HEADER_MIN_TRACK_PERCENT)
}

/// Reconstruct a single region's words into a `Table`, applying the same
/// validation chain the layout-detection path uses (`layout_guided = true`).
fn reconstruct_region_table(
    region: &[crate::pdf::table_reconstruct::HocrWord],
    page_height: f32,
    page_number: u32,
    allow_single_column: bool,
    horizontal_rules: usize,
) -> Option<Table> {
    match reconstruct_region_table_with_reason(region, page_height, page_number, allow_single_column, horizontal_rules)
    {
        Ok(table) => Some(table),
        Err(reason) => {
            tracing::trace!(
                page = page_number,
                words = region.len(),
                ?reason,
                "heuristic table region rejected"
            );
            None
        }
    }
}

fn reconstruct_region_table_with_reason(
    region: &[crate::pdf::table_reconstruct::HocrWord],
    page_height: f32,
    page_number: u32,
    allow_single_column: bool,
    horizontal_rules: usize,
) -> std::result::Result<Table, HeuristicTableRejection> {
    let region_left = region.iter().map(|w| w.left).min().unwrap_or(0);
    let region_right = region.iter().map(|w| w.left + w.width).max().unwrap_or(0);
    let region_width = region_right.saturating_sub(region_left) as f32;
    let col_gap = heuristic_column_gap(region, region_width);
    reconstruct_region_table_with_column_gap(
        region,
        page_height,
        page_number,
        allow_single_column,
        col_gap,
        horizontal_rules,
    )
}

fn reconstruct_region_table_with_column_gap(
    region: &[crate::pdf::table_reconstruct::HocrWord],
    page_height: f32,
    page_number: u32,
    allow_single_column: bool,
    col_gap: u32,
    horizontal_rules: usize,
) -> std::result::Result<Table, HeuristicTableRejection> {
    use crate::pdf::table_reconstruct::{
        is_well_formed_borderless_table, looks_like_code_listing, post_process_table_with_columns, table_to_markdown,
    };

    // Take the column positions from the same call that built the grid. `reconstruct_table`
    // now detects columns from post-merge cell tokens, not from `region` directly, so a
    // separate `detect_columns(region, col_gap)` no longer agrees with the grid's column
    // count — and `repair_split_numeric_track` fails safe on a count mismatch, which would
    // have silently switched the repair off rather than reporting anything (#688).
    let (mut grid, mut column_positions) = crate::table_core::reconstruct_table_with_columns(region, col_gap, 0.5);
    // Takes `column_positions` by &mut so a successful repair drops the merged boundary from
    // BOTH: the repair collapses two grid columns into one, and a stale extra position would
    // leave `grid` one column narrower than `column_positions` for every consumer below —
    // including `is_well_formed_borderless_table`, which is handed both.
    repair_split_numeric_track(&mut grid, region, &mut column_positions);
    if grid.is_empty() || grid[0].is_empty() {
        return Err(HeuristicTableRejection::EmptyGrid);
    }

    tracing::trace!(
        page = page_number,
        col_gap,
        rows = grid.len(),
        cols = grid.first().map_or(0, Vec::len),
        "heuristic table reconstructed grid"
    );

    // Use the `column_positions`-tracking variant: `post_process_table` alone can drop a
    // column (an empty-data header-only track, or a spurious footer-only interior track)
    // without telling `column_positions`, leaving a stale boundary for
    // `is_well_formed_borderless_table` below to test against a column that no longer
    // exists (#863).
    let mut cleaned = post_process_table_with_columns(grid, true, allow_single_column, &mut column_positions)
        .ok_or(HeuristicTableRejection::PostProcessing)?;
    repair_heuristic_header_delimiters(&mut cleaned);
    if cleaned.len() <= 1 {
        return Err(HeuristicTableRejection::TooFewRows);
    }

    if looks_like_code_listing(&cleaned) {
        return Err(HeuristicTableRejection::CodeListing);
    }

    // Beyond the text-statistics gate, reject candidates that have no drawn
    // ruling lines AND whose inferred column boundaries are not actually
    // whitespace (xberg-io/xberg#1399): continuous prose bucketed into a wide
    // grid by vertical-only clustering has words running across those
    // boundaries on most rows, which text statistics alone cannot see.
    if !is_well_formed_borderless_table(&cleaned, region, &column_positions, horizontal_rules) {
        return Err(HeuristicTableRejection::NotWellFormed);
    }

    let bounding_box = region_bounding_box(region, page_height);

    let markdown = table_to_markdown(&cleaned);
    if markdown.trim().is_empty() {
        return Err(HeuristicTableRejection::EmptyMarkdown);
    }

    Ok(Table {
        cells: cleaned,
        markdown,
        page_number,
        bounding_box,
        ..Default::default()
    })
}

fn region_bounding_box(region: &[crate::pdf::table_reconstruct::HocrWord], page_height: f32) -> Option<BoundingBox> {
    let img_left = region.iter().map(|word| word.left as f64).fold(f64::INFINITY, f64::min);
    let img_top = region.iter().map(|word| word.top as f64).fold(f64::INFINITY, f64::min);
    let img_right = region
        .iter()
        .map(|word| (word.left + word.width) as f64)
        .fold(0.0_f64, f64::max);
    let img_bottom = region
        .iter()
        .map(|word| (word.top + word.height) as f64)
        .fold(0.0_f64, f64::max);
    (img_right > img_left && img_bottom > img_top).then_some(BoundingBox {
        x0: img_left,
        y0: page_height as f64 - img_bottom,
        x1: img_right,
        y1: page_height as f64 - img_top,
    })
}

fn repair_heuristic_header_delimiters(rows: &mut [Vec<String>]) {
    let Some(header) = rows.first_mut() else {
        return;
    };

    for cell in header.iter_mut() {
        repair_joined_header_closer_spacing(cell);
    }

    for column in 0..header.len().saturating_sub(1) {
        let (left_cells, right_cells) = header.split_at_mut(column + 1);
        let left = &mut left_cells[column];
        let right = &mut right_cells[0];

        let Some((closer, remainder)) = leading_single_closer(right) else {
            continue;
        };
        if unmatched_opener(left) != matching_opener(closer) {
            continue;
        }

        left.push(closer);
        *right = remainder.to_string();
    }
}

fn repair_joined_header_closer_spacing(text: &mut String) {
    let mut openers = Vec::new();
    let mut whitespace_ranges = Vec::new();
    for (index, character) in text.char_indices() {
        if matches!(character, '(' | '[' | '{') {
            openers.push((character, index));
            continue;
        }

        let Some(expected) = matching_opener(character) else {
            continue;
        };
        let Some((opener, opener_index)) = openers.pop() else {
            return;
        };
        if opener != expected {
            return;
        }

        let content_start = opener_index + opener.len_utf8();
        if text[content_start..]
            .chars()
            .next()
            .is_some_and(|next| !next.is_whitespace())
        {
            let trimmed_end = text[..index].trim_end().len();
            if trimmed_end < index {
                whitespace_ranges.push(trimmed_end..index);
            }
        }
    }

    for range in whitespace_ranges.into_iter().rev() {
        text.replace_range(range, "");
    }
}

fn leading_single_closer(text: &str) -> Option<(char, &str)> {
    let mut chars = text.chars();
    let closer = chars.next()?;
    matching_opener(closer)?;

    let remainder = chars.as_str().trim_start();
    if remainder.is_empty()
        || remainder
            .chars()
            .next()
            .is_some_and(|character| matching_opener(character).is_some())
    {
        return None;
    }

    Some((closer, remainder))
}

fn unmatched_opener(text: &str) -> Option<char> {
    let mut openers = Vec::new();
    for character in text.chars() {
        if matches!(character, '(' | '[' | '{') {
            openers.push(character);
            continue;
        }

        let Some(expected) = matching_opener(character) else {
            continue;
        };
        if openers.pop() != Some(expected) {
            return None;
        }
    }
    openers.last().copied()
}

fn matching_opener(closer: char) -> Option<char> {
    match closer {
        ')' => Some('('),
        ']' => Some('['),
        '}' => Some('{'),
        _ => None,
    }
}

fn heuristic_column_gap(region: &[crate::pdf::table_reconstruct::HocrWord], region_width: f32) -> u32 {
    let adaptive_gap = crate::pdf::structure::regions::tables::compute_adaptive_column_gap(region, region_width);
    if is_dense_numeric_region(region) {
        adaptive_gap.min(DENSE_NUMERIC_COLUMN_GAP_CAP)
    } else {
        adaptive_gap
    }
}

/// Merge one numeric x-track that was split by anchored column clustering.
///
/// `detect_columns` groups against each cluster's first x-position. A header
/// glyph at the edge of a numeric column can therefore anchor one cluster and
/// split values with a few points of normal alignment jitter into a second,
/// nearly coincident track. Restrict the repair to geometrically overlapping
/// tracks whose numeric cells alternate, with header text on exactly one side.
fn repair_split_numeric_track(
    grid: &mut [Vec<String>],
    region: &[crate::pdf::table_reconstruct::HocrWord],
    column_positions: &mut Vec<u32>,
) -> bool {
    if grid.first().map_or(0, Vec::len) != column_positions.len() {
        return false;
    }

    let candidates: Vec<(usize, bool)> = (0..column_positions.len().saturating_sub(1))
        .filter_map(|column| {
            split_numeric_track_candidate(grid, region, column_positions, column)
                .map(|header_on_left| (column, header_on_left))
        })
        .collect();
    let [(column, _header_on_left)] = candidates.as_slice() else {
        return false;
    };

    for row in grid {
        let right = row.remove(column + 1);
        if row[*column].trim().is_empty() {
            row[*column] = right;
        }
    }
    // The two tracks were nearly coincident by construction — that is what made this a split
    // rather than two real columns — so the surviving cluster keeps the left anchor and the
    // spurious boundary is dropped, mirroring the `row.remove(column + 1)` above exactly.
    column_positions.remove(column + 1);
    true
}

fn split_numeric_track_candidate(
    grid: &[Vec<String>],
    region: &[crate::pdf::table_reconstruct::HocrWord],
    column_positions: &[u32],
    column: usize,
) -> Option<bool> {
    let mut left_numeric = 0usize;
    let mut right_numeric = 0usize;
    let mut left_header = false;
    let mut right_header = false;
    for (row_index, row) in grid.iter().enumerate() {
        let left = row.get(column)?.trim();
        let right = row.get(column + 1)?.trim();
        if !left.is_empty() && !right.is_empty() {
            return None;
        }
        let (text, on_left) = if !left.is_empty() {
            (left, true)
        } else if !right.is_empty() {
            (right, false)
        } else {
            continue;
        };
        if is_numeric_word(text) {
            if on_left {
                left_numeric += 1;
            } else {
                right_numeric += 1;
            }
        } else if row_index >= NUMERIC_HEADER_MAX_ROWS {
            return None;
        } else if on_left {
            left_header = true;
        } else {
            right_header = true;
        }
    }

    if left_numeric < SPLIT_NUMERIC_TRACK_MIN_ROWS_PER_SIDE
        || right_numeric < SPLIT_NUMERIC_TRACK_MIN_ROWS_PER_SIDE
        || left_numeric + right_numeric < SPLIT_NUMERIC_TRACK_MIN_TOTAL_ROWS
        || left_header == right_header
    {
        return None;
    }

    let separation = column_positions[column].abs_diff(column_positions[column + 1]);
    let mut numeric_widths: Vec<u32> = region
        .iter()
        .filter(|word| is_numeric_word(&word.text))
        .filter_map(|word| {
            let nearest = column_positions
                .iter()
                .enumerate()
                .min_by_key(|(_, position)| position.abs_diff(word.left))?
                .0;
            matches!(nearest, current if current == column || current == column + 1).then_some(word.width)
        })
        .collect();
    numeric_widths.sort_unstable();
    let median_width = *numeric_widths.get(numeric_widths.len() / 2)?;
    (separation.saturating_mul(2) < median_width).then_some(left_header)
}

fn is_dense_numeric_region(region: &[crate::pdf::table_reconstruct::HocrWord]) -> bool {
    if region.is_empty() {
        return false;
    }

    let median_height = {
        let mut heights: Vec<u32> = region.iter().map(|word| word.height).collect();
        heights.sort_unstable();
        heights[heights.len() / 2].max(1)
    };
    let row_tolerance = (median_height / 2).max(3);
    let rows = numeric_rows(region, row_tolerance);
    let Some(recurring_rows) = longest_recurring_numeric_run(&rows) else {
        return false;
    };

    recurring_numeric_x_tracks(recurring_rows, median_height) >= DENSE_NUMERIC_MIN_RECURRING_TRACKS
}

fn numeric_rows(
    region: &[crate::pdf::table_reconstruct::HocrWord],
    row_tolerance: u32,
) -> Vec<Vec<&crate::pdf::table_reconstruct::HocrWord>> {
    let mut sorted: Vec<_> = region.iter().collect();
    sorted.sort_by_key(|word| word.top + word.height / 2);
    let mut rows: Vec<Vec<&crate::pdf::table_reconstruct::HocrWord>> = Vec::new();
    for word in sorted {
        let center = word.top + word.height / 2;
        match rows.last_mut() {
            Some(row) if (row[0].top + row[0].height / 2).abs_diff(center) <= row_tolerance => row.push(word),
            _ => rows.push(vec![word]),
        }
    }
    rows
}

fn longest_recurring_numeric_run<'a>(
    rows: &'a [Vec<&'a crate::pdf::table_reconstruct::HocrWord>],
) -> Option<&'a [Vec<&'a crate::pdf::table_reconstruct::HocrWord>]> {
    let mut best = 0..0;
    let mut start = 0;
    for (index, row) in rows.iter().enumerate() {
        if is_numeric_row(row) {
            continue;
        }
        if index - start > best.len() {
            best = start..index;
        }
        start = index + 1;
    }
    if rows.len() - start > best.len() {
        best = start..rows.len();
    }
    (best.len() >= DENSE_NUMERIC_MIN_RECURRING_ROWS).then(|| &rows[best])
}

fn is_numeric_row(row: &[&crate::pdf::table_reconstruct::HocrWord]) -> bool {
    let numeric_words = row.iter().filter(|word| is_numeric_word(&word.text)).count();
    numeric_words >= DENSE_NUMERIC_MIN_WORDS_PER_ROW
        && numeric_words.saturating_mul(100) >= row.len().saturating_mul(DENSE_NUMERIC_MIN_ROW_WORD_PERCENT)
}

fn recurring_numeric_x_tracks(rows: &[Vec<&crate::pdf::table_reconstruct::HocrWord>], median_height: u32) -> usize {
    recurring_numeric_track_centers(rows, median_height).len()
}

fn recurring_numeric_track_centers(
    rows: &[Vec<&crate::pdf::table_reconstruct::HocrWord>],
    median_height: u32,
) -> Vec<u32> {
    let x_tolerance = median_height.saturating_mul(2).max(12);
    let min_row_support = rows
        .len()
        .saturating_mul(DENSE_NUMERIC_MIN_TRACK_ROW_PERCENT)
        .div_ceil(100);
    let mut candidates: Vec<u32> = rows
        .iter()
        .flatten()
        .filter(|word| is_numeric_word(&word.text))
        .map(|word| word.left + word.width / 2)
        .collect();
    candidates.sort_unstable();
    candidates.dedup_by(|left, right| left.abs_diff(*right) <= x_tolerance);
    candidates
        .into_iter()
        .filter(|candidate| {
            rows.iter()
                .filter(|row| {
                    row.iter().any(|word| {
                        is_numeric_word(&word.text) && (word.left + word.width / 2).abs_diff(*candidate) <= x_tolerance
                    })
                })
                .count()
                >= min_row_support
        })
        .collect()
}

fn is_numeric_word(text: &str) -> bool {
    let digit_count = text.chars().filter(char::is_ascii_digit).count();
    if digit_count == 0 {
        return false;
    }
    let alphanumeric_count = text.chars().filter(|c| c.is_alphanumeric()).count();
    digit_count.saturating_mul(2) >= alphanumeric_count
}

/// Reconstruct cell text from span positions in reading order.
///
/// PDF coordinates place y=0 at the bottom of the page, so larger Y values are
/// higher on the page (visually earlier in reading order). Within a row, X
/// increases left-to-right. We sort by the span's own upright-frame cross
/// axis descending (top-to-bottom) then advance axis ascending (left-to-right)
/// to recover natural reading order regardless of the order in which
/// xberg_native_pdf yields spans.
///
/// [`super::span_geometry::upright_origin`] rotates a span's page-space origin
/// back into its own reading frame before comparison, so this is correct both
/// for the common unrotated case (where it is the identity `(bbox.x,
/// bbox.y)`, matching the plain X/Y sort this replaced) and for a
/// text-matrix-rotated cell (GH#1358: sideways spec-table columns whose
/// `TableCell::spans` carry non-zero `rotation_degrees`), where sorting by
/// raw page-space X/Y instead of the run's own advance axis reads the cell's
/// words out of order. `TableCell::spans` carries real rotation data both when
/// xberg_native_pdf's geometric table detector (`grid_to_table`) clones the original
/// page spans into the cell, and on xberg_native_pdf's tagged-PDF/MCID path
/// (`extract_cell`), which synthesizes its own per-MCID spans but carries the
/// source block's `rotation_degrees` forward rather than hardcoding `0.0`.
///
/// Embedded newlines inside span text are collapsed to spaces, and any internal run of
/// whitespace within a single span's own text (GH#1628: a wide kerning adjustment inside one
/// `Tj`/`TJ` run can decode to more than one literal space glyph, which `str::trim()` -- which
/// only strips the *ends* -- does not catch) collapses to one, so joining never stacks a span's
/// own embedded space on top of the separator this function inserts between spans.
///
/// A pair of spans that reads as a sub/superscript run (see [`is_script_run_of`]) is kept in the
/// same reading-order "row" as the base it is attached to (GH#1628): a subscript's lower baseline
/// would otherwise place it in a different cross-axis row than the base symbol it abuts, ahead of
/// every other same-height span in the cell regardless of physical x-adjacency. See
/// [`crate::script_run`] for the decision rule shared with prose assembly, and
/// [`sort_spans_in_reading_order`] for how row membership is resolved.
fn cell_text_in_reading_order(cell: &xberg_native_pdf::structure::table_extractor::TableCell) -> String {
    if cell.spans.is_empty() {
        return cell.text.trim().replace('\n', " ").to_string();
    }

    let sorted = sort_spans_in_reading_order(cell.spans.iter().collect());

    let joined: String = sorted
        .iter()
        .map(|span| span.text.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    joined
}

/// How many already-placed (in advance-axis order) spans back a candidate script run's base may
/// be found among. Mirrors `pdf::structure::pipeline::SCRIPT_RUN_BASE_LOOKBACK`'s rationale
/// (a base is not always the immediately preceding span once other same-row content interleaves)
/// at a value sized for a table cell's much shorter span lists.
const TABLE_SCRIPT_RUN_BASE_LOOKBACK: usize = 4;

/// Sort a cell's spans into reading order: top-to-bottom by row, left-to-right within a row.
///
/// Spans are first ordered along their own advance axis to establish physical left-to-right
/// adjacency, then each span is assigned a `row_key` -- its own cross-axis position, unless it is
/// a script run of one of the up to [`TABLE_SCRIPT_RUN_BASE_LOOKBACK`] spans immediately before it
/// in that advance order, in which case it inherits that base's `row_key` instead (transitively,
/// so a chain of script runs shares one row). The final sort is by `row_key` descending (top rows
/// first, matching the original cross-descending order) then advance ascending -- which now also
/// resolves a script run's position relative to *every* other span in its row by physical
/// x-position, not only relative to its own base.
fn sort_spans_in_reading_order(
    spans: Vec<&xberg_native_pdf::layout::TextSpan>,
) -> Vec<&xberg_native_pdf::layout::TextSpan> {
    let mut by_advance = spans;
    by_advance.sort_by(|a, b| {
        let (advance_a, _) = super::span_geometry::upright_origin(a);
        let (advance_b, _) = super::span_geometry::upright_origin(b);
        advance_a.total_cmp(&advance_b)
    });

    let mut row_key: Vec<f32> = by_advance
        .iter()
        .map(|span| super::span_geometry::upright_origin(span).1)
        .collect();

    for index in 0..by_advance.len() {
        let lookback_start = index.saturating_sub(TABLE_SCRIPT_RUN_BASE_LOOKBACK);
        for candidate in (lookback_start..index).rev() {
            if is_script_run_of(by_advance[candidate], by_advance[index]) {
                row_key[index] = row_key[candidate];
                break;
            }
        }
    }

    let mut order: Vec<usize> = (0..by_advance.len()).collect();
    order.sort_by(|&i, &j| {
        let (advance_i, _) = super::span_geometry::upright_origin(by_advance[i]);
        let (advance_j, _) = super::span_geometry::upright_origin(by_advance[j]);
        row_key[j]
            .total_cmp(&row_key[i])
            .then_with(|| advance_i.total_cmp(&advance_j))
    });

    order.into_iter().map(|index| by_advance[index]).collect()
}

/// Whether `next` reads as a sub/superscript attached to `previous`: same rotation, neither
/// monospace, a materially smaller font raised or lowered by a fraction of it, and a start inside
/// or abutting `previous`'s advance extent.
///
/// The float arithmetic (the baseline-offset and forward-gap tests) is single-sourced in
/// [`crate::script_run`], shared with `pdf::structure::pipeline::is_script_run_of`'s prose
/// equivalent; only the type-specific guards and geometry accessors below differ, because
/// `TextSpan` has no `assigned_role` field to compare (a cell's own spans are already assumed to
/// share one, so no widening happens here that the prose predicate's role guard would have
/// blocked). ~keep GH#1628
fn is_script_run_of(previous: &xberg_native_pdf::layout::TextSpan, next: &xberg_native_pdf::layout::TextSpan) -> bool {
    if previous.is_monospace || next.is_monospace || !super::span_geometry::has_same_rotation(previous, next) {
        return false;
    }
    if !previous.font_size.is_finite()
        || !next.font_size.is_finite()
        || previous.font_size <= 0.0
        || next.font_size <= 0.0
        || !previous.bbox.width.is_finite()
        || !next.bbox.width.is_finite()
        || previous.bbox.width < 0.0
        || next.bbox.width < 0.0
    {
        return false;
    }

    let (previous_start, previous_end) = super::span_geometry::upright_advance_extent(previous);
    let (next_start, _) = super::span_geometry::upright_advance_extent(next);
    let (_, previous_cross) = super::span_geometry::upright_origin(previous);
    let (_, next_cross) = super::span_geometry::upright_origin(next);
    if !previous_cross.is_finite() || !next_cross.is_finite() {
        return false;
    }

    let font_size = previous.font_size.max(next.font_size);
    let smaller_font_size = previous.font_size.min(next.font_size);
    let baseline_delta = (next_cross - previous_cross).abs();

    crate::script_run::is_script_run_baseline_offset(baseline_delta, font_size, smaller_font_size)
        && crate::script_run::is_script_run_forward_gap(previous_start, previous_end, next_start, font_size)
}

/// Convert a xberg_native_pdf `ExtractedTable` to xberg's cell grid and markdown.
///
/// Maps rows/cells from the native table structure to a 2D `Vec<Vec<String>>`
/// grid and builds a markdown representation with proper header separators.
///
/// Cell text is reconstructed from span positions in reading order (see
/// [`cell_text_in_reading_order`]) when span data is available.
fn convert_extracted_table(
    table: &xberg_native_pdf::structure::table_extractor::Table,
) -> (Vec<Vec<String>>, String, Option<crate::types::TableGrid>) {
    use crate::extraction::grid_flatten::{
        MAX_COL_SPAN, MAX_ROW_SPAN, SpanCell, flatten_spanned_rows, resolve_span_grid,
    };
    use crate::types::{GridCell, TableGrid};

    let rows: Vec<Vec<SpanCell>> = table
        .rows
        .iter()
        .map(|row| {
            row.cells
                .iter()
                .map(|cell| SpanCell::new(cell_text_in_reading_order(cell), cell.rowspan, cell.colspan))
                .collect()
        })
        .collect();
    // Ordinary tables need no extra geometry payload.
    let grid = rows
        .iter()
        .flatten()
        .any(|cell| cell.col_span > 1 || cell.row_span > 1)
        .then(|| {
            let mut grid_cells = Vec::new();
            let cols = resolve_span_grid(
                &rows,
                |c| c.col_span,
                |c| c.row_span,
                |row, col, cell| {
                    grid_cells.push(GridCell {
                        content: cell.content.clone(),
                        row,
                        col,
                        row_span: cell.row_span.clamp(1, MAX_ROW_SPAN),
                        col_span: cell.col_span.clamp(1, MAX_COL_SPAN),
                        is_header: row == 0 || table.rows[row as usize].is_header,
                        bbox: None,
                        heading_level: None,
                        style_name: None,
                    });
                },
            );
            TableGrid {
                rows: rows.len() as u32,
                cols,
                cells: grid_cells,
            }
        });
    let cells = flatten_spanned_rows(&rows);
    let markdown = crate::pdf::table_reconstruct::table_to_markdown(&cells);
    (cells, markdown, grid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converted_spans_reserve_columns_without_duplicating_text() {
        use xberg_native_pdf::structure::table_extractor::{Table as ExtractedTable, TableCell, TableRow};
        let cell = |text: &str, rows, cols| TableCell {
            text: text.into(),
            rowspan: rows,
            colspan: cols,
            spans: vec![],
            mcids: vec![],
            bbox: None,
            is_header: false,
        };
        let table = ExtractedTable {
            rows: vec![
                TableRow {
                    cells: vec![cell("Group | A", 1, 2), cell("B", 1, 1)],
                    is_header: true,
                },
                TableRow {
                    cells: vec![cell("Label", 2, 1), cell("x", 1, 1), cell("1", 1, 1)],
                    is_header: false,
                },
                TableRow {
                    cells: vec![cell("y", 1, 1), cell("2", 1, 1)],
                    is_header: false,
                },
            ],
            col_count: 3,
            has_header: true,
            bbox: None,
        };
        let (cells, markdown, grid) = convert_extracted_table(&table);
        let grid = grid.unwrap();
        assert_eq!(
            cells,
            vec![vec!["Group | A", "", "B"], vec!["Label", "x", "1"], vec!["", "y", "2"]]
        );
        assert_eq!(grid.cells[0].col_span, 2);
        assert_eq!((grid.cells[2].col, grid.cells[2].row_span), (0, 2));
        assert_eq!((grid.cells[5].row, grid.cells[5].col), (2, 1));
        assert_eq!(markdown.matches("Label").count(), 1);
        assert!(markdown.contains("Group \\| A"));
    }

    #[test]
    fn test_convert_extracted_table_basic() {
        use xberg_native_pdf::structure::table_extractor::{Table as ExtractedTable, TableCell, TableRow};

        let table = ExtractedTable {
            rows: vec![
                TableRow {
                    cells: vec![
                        TableCell {
                            text: "Name".to_string(),
                            colspan: 1,
                            rowspan: 1,
                            mcids: vec![],
                            spans: vec![],
                            bbox: None,
                            is_header: true,
                        },
                        TableCell {
                            text: "Age".to_string(),
                            colspan: 1,
                            rowspan: 1,
                            mcids: vec![],
                            spans: vec![],
                            bbox: None,
                            is_header: true,
                        },
                    ],
                    is_header: true,
                },
                TableRow {
                    cells: vec![
                        TableCell {
                            text: "Alice".to_string(),
                            colspan: 1,
                            rowspan: 1,
                            mcids: vec![],
                            spans: vec![],
                            bbox: None,
                            is_header: false,
                        },
                        TableCell {
                            text: "30".to_string(),
                            colspan: 1,
                            rowspan: 1,
                            mcids: vec![],
                            spans: vec![],
                            bbox: None,
                            is_header: false,
                        },
                    ],
                    is_header: false,
                },
            ],
            has_header: true,
            col_count: 2,
            bbox: None,
        };

        let (cells, markdown, _) = convert_extracted_table(&table);
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0], vec!["Name", "Age"]);
        assert_eq!(cells[1], vec!["Alice", "30"]);
        assert!(markdown.contains("| Name | Age |"));
        assert!(markdown.contains("| --- | --- |"));
        assert!(markdown.contains("| Alice | 30 |"));
    }

    #[test]
    fn test_convert_extracted_table_no_header() {
        use xberg_native_pdf::structure::table_extractor::{Table as ExtractedTable, TableCell, TableRow};

        let table = ExtractedTable {
            rows: vec![
                TableRow {
                    cells: vec![TableCell {
                        text: "A".to_string(),
                        colspan: 1,
                        rowspan: 1,
                        mcids: vec![],
                        spans: vec![],
                        bbox: None,
                        is_header: false,
                    }],
                    is_header: false,
                },
                TableRow {
                    cells: vec![TableCell {
                        text: "B".to_string(),
                        colspan: 1,
                        rowspan: 1,
                        mcids: vec![],
                        spans: vec![],
                        bbox: None,
                        is_header: false,
                    }],
                    is_header: false,
                },
            ],
            has_header: false,
            col_count: 1,
            bbox: None,
        };

        let (cells, markdown, _) = convert_extracted_table(&table);
        assert_eq!(cells.len(), 2);
        assert!(markdown.contains("| --- |"));
    }

    #[test]
    fn test_convert_extracted_table_empty() {
        use xberg_native_pdf::structure::table_extractor::Table as ExtractedTable;

        let table = ExtractedTable {
            rows: vec![],
            has_header: false,
            col_count: 0,
            bbox: None,
        };

        let (cells, markdown, _) = convert_extracted_table(&table);
        assert!(cells.is_empty());
        assert!(markdown.is_empty());
    }

    /// Build a synthetic TextSpan for position-order tests.
    ///
    /// `x` and `y` are the span's PDF-coordinate origin (y=0 at bottom of page).
    fn make_span(text: &str, x: f32, y: f32) -> xberg_native_pdf::layout::TextSpan {
        xberg_native_pdf::layout::TextSpan {
            text: text.to_string(),
            bbox: xberg_native_pdf::geometry::Rect {
                x,
                y,
                width: 50.0,
                height: 10.0,
            },
            font_name: "Helvetica".to_string(),
            font_size: 10.0,
            font_weight: xberg_native_pdf::layout::FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: xberg_native_pdf::layout::Color::default(),
            mcid: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 1.0,
            primary_detected: false,
            artifact_type: None,
            char_widths: vec![],
            heading_level: None,
            rotation_degrees: 0.0,
            ..xberg_native_pdf::layout::TextSpan::default()
        }
    }

    /// Spans delivered in reverse Y order (bottom span first, top span last).
    /// Reading order must place the top span (higher Y in PDF coords) first.
    #[test]
    fn test_cell_text_in_reading_order_sorts_by_y_descending() {
        use xberg_native_pdf::structure::table_extractor::TableCell;

        let cell = TableCell {
            text: "wrong order".to_string(),
            colspan: 1,
            rowspan: 1,
            mcids: vec![],
            spans: vec![make_span("second", 10.0, 100.0), make_span("first", 10.0, 200.0)],
            bbox: None,
            is_header: false,
        };

        let text = cell_text_in_reading_order(&cell);
        assert_eq!(
            text, "first second",
            "span with higher Y (top of page) must come before span with lower Y; got: {text:?}"
        );
    }

    /// Within the same Y row, spans must be ordered left-to-right (X ascending).
    #[test]
    fn test_cell_text_in_reading_order_sorts_same_y_by_x_ascending() {
        use xberg_native_pdf::structure::table_extractor::TableCell;

        let cell = TableCell {
            text: "wrong order".to_string(),
            colspan: 1,
            rowspan: 1,
            mcids: vec![],
            spans: vec![make_span("right", 200.0, 150.0), make_span("left", 10.0, 150.0)],
            bbox: None,
            is_header: false,
        };

        let text = cell_text_in_reading_order(&cell);
        assert_eq!(
            text, "left right",
            "same-row spans must be ordered left-to-right (X ascending); got: {text:?}"
        );
    }

    /// Build a synthetic rotated TextSpan for the GH#1358 sideways-table
    /// tests. `x`/`y` are the span's page-space origin (as xberg_native_pdf reports
    /// for a rotated run: origin in page coordinates, width/height flattened
    /// onto the run's own rotated axis).
    fn make_rotated_span(text: &str, x: f32, y: f32, rotation_degrees: f32) -> xberg_native_pdf::layout::TextSpan {
        xberg_native_pdf::layout::TextSpan {
            text: text.to_string(),
            bbox: xberg_native_pdf::geometry::Rect {
                x,
                y,
                width: 50.0,
                height: 10.0,
            },
            font_name: "Helvetica".to_string(),
            font_size: 10.0,
            font_weight: xberg_native_pdf::layout::FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: xberg_native_pdf::layout::Color::default(),
            mcid: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 1.0,
            primary_detected: false,
            artifact_type: None,
            char_widths: vec![],
            heading_level: None,
            rotation_degrees,
            ..xberg_native_pdf::layout::TextSpan::default()
        }
    }

    /// GH#1358: a table cell from xberg_native_pdf's geometric detector
    /// (`grid_to_table`, which clones the original page spans and so
    /// preserves their real `rotation_degrees`) whose spans carry a
    /// 90-degree text-matrix rotation, delivered in the scrambled order
    /// xberg_native_pdf reports fragments in (reversed from reading order — the
    /// exact shape of the reported defect, `"the meet only need oil
    /// Engine"`). A plain page-space X/Y sort reads this out of order because
    /// a rotated run's reading direction is the run's own advance axis
    /// (page-y for 90 degrees), not raw page-y. All spans share one page-x
    /// (the column the rotated run is printed along), so page-space Y (not
    /// X) must resolve the tie once the axes are corrected.
    #[test]
    fn test_cell_text_in_reading_order_sorts_rotated_spans_by_advance_axis() {
        use xberg_native_pdf::structure::table_extractor::TableCell;

        let cell = TableCell {
            text: "wrong order".to_string(),
            colspan: 1,
            rowspan: 1,
            mcids: vec![],
            spans: vec![
                make_rotated_span("need", 100.0, 100.0, 90.0),
                make_rotated_span("oil", 100.0, 50.0, 90.0),
                make_rotated_span("Engine", 100.0, 0.0, 90.0),
            ],
            bbox: None,
            is_header: false,
        };

        let text = cell_text_in_reading_order(&cell);
        assert_eq!(
            text, "Engine oil need",
            "a 90-degree-rotated cell must be read along its own advance axis (page-y), \
             not raw page-space X/Y; got: {text:?}"
        );
    }

    /// Build a synthetic TextSpan with an explicit width and font size, for the GH#1628
    /// script-run tests, where both matter to the adjacency arithmetic.
    fn make_span_ext(text: &str, x: f32, y: f32, width: f32, font_size: f32) -> xberg_native_pdf::layout::TextSpan {
        xberg_native_pdf::layout::TextSpan {
            text: text.to_string(),
            bbox: xberg_native_pdf::geometry::Rect {
                x,
                y,
                width,
                height: font_size,
            },
            font_size,
            ..make_span("", 0.0, 0.0)
        }
    }

    /// GH#1628: a subscript ("S" in `eta_S %`) sits at a lower baseline than the base symbol
    /// it annotates ("eta") and a materially smaller font. The plain cross-axis-then-advance
    /// comparator sorts every same-height span into its own row ahead of the subscript
    /// regardless of physical x-adjacency, so the unit that physically follows the base ("%")
    /// prints before the subscript. `sub` starts inside `base`'s advance extent (kerned TJ
    /// fusion, the same shape #1617 fixed for prose) and its baseline sits 0.7pt below a
    /// 11.59pt baseline -- both drawn from GH#1617's own reproducer measurements.
    #[test]
    fn test_cell_text_in_reading_order_keeps_script_run_adjacent_to_base() {
        use xberg_native_pdf::structure::table_extractor::TableCell;

        let base = make_span_ext("eta", 100.0, 200.0, 6.5, 11.59);
        let sub = make_span_ext("S", 104.0, 199.3, 4.0, 8.0);
        let unit = make_span_ext("%", 115.0, 200.0, 6.0, 11.59);

        let cell = TableCell {
            text: "wrong order".to_string(),
            colspan: 1,
            rowspan: 1,
            mcids: vec![],
            spans: vec![unit, base, sub],
            bbox: None,
            is_header: false,
        };

        let text = cell_text_in_reading_order(&cell);
        assert_eq!(
            text, "eta S %",
            "a subscript adjacent to its base must print immediately after it, not after \
             every same-height span in the cell; got: {text:?}"
        );
    }

    /// Pins that an NBSP-only span (U+00A0, common in justified PDFs) still collapses away.
    ///
    /// This is a preserved-behaviour pin, NOT a regression test for GH#1628: U+00A0 *is*
    /// `White_Space` in Unicode, so `char::is_whitespace` is true for it and the previous
    /// `str::trim()` implementation already emptied such a span. Measured, not assumed --- the
    /// original suspicion that NBSP was the source of the doubled gap in `Q GJ  HE` was wrong,
    /// and the real cause is the internal multi-space run the test below covers. Kept so the
    /// `split_whitespace` rewrite cannot quietly lose the NBSP handling it inherited.
    #[test]
    fn test_cell_text_in_reading_order_collapses_nbsp_only_span() {
        use xberg_native_pdf::structure::table_extractor::TableCell;

        let cell = TableCell {
            text: "wrong order".to_string(),
            colspan: 1,
            rowspan: 1,
            mcids: vec![],
            spans: vec![
                make_span("Q", 100.0, 200.0),
                make_span("\u{a0}", 150.0, 200.0),
                make_span("GJ", 200.0, 200.0),
            ],
            bbox: None,
            is_header: false,
        };

        let text = cell_text_in_reading_order(&cell);
        assert_eq!(
            text, "Q GJ",
            "an NBSP-only span must not introduce a second space alongside the join \
             separator; got: {text:?}"
        );
    }

    /// GH#1628: a wide kerning adjustment inside a single `Tj`/`TJ` run can decode to more than
    /// one literal space glyph, so a single span's own `text` can carry an internal run of spaces
    /// (`str::trim()` only strips the *ends*, so this survives untouched). Joining that span with
    /// its neighbour via `join(" ")` then stacks the join's own separator on top of the span's
    /// already-embedded space, producing a visibly doubled gap the neighbouring spans' own
    /// individual trimming cannot catch.
    #[test]
    fn test_cell_text_in_reading_order_collapses_internal_multi_space_run() {
        use xberg_native_pdf::structure::table_extractor::TableCell;

        let cell = TableCell {
            text: "wrong order".to_string(),
            colspan: 1,
            rowspan: 1,
            mcids: vec![],
            spans: vec![make_span("Q", 100.0, 200.0), make_span("GJ  HE", 150.0, 200.0)],
            bbox: None,
            is_header: false,
        };

        let text = cell_text_in_reading_order(&cell);
        assert_eq!(
            text, "Q GJ HE",
            "an internal multi-space run inside one span's own text must collapse to a single \
             space, not stack with the join separator; got: {text:?}"
        );
    }

    /// Build a synthetic `HocrWord` for the clustering tests.
    fn make_word(text: &str, left: u32, top: u32, width: u32) -> crate::pdf::table_reconstruct::HocrWord {
        crate::pdf::table_reconstruct::HocrWord {
            text: text.to_string(),
            left,
            top,
            width,
            height: 10,
            confidence: 95.0,
        }
    }

    fn extend_grid_words(words: &mut Vec<crate::pdf::table_reconstruct::HocrWord>, x_positions: &[u32]) {
        for left in x_positions {
            words.push(make_word("Heading", *left, 100, 30));
        }
        for row in 0..6 {
            for (column, left) in x_positions.iter().enumerate() {
                words.push(make_word(&format!("{}.{column}", row + 1), *left, 130 + row * 12, 30));
            }
        }
    }

    fn extend_financial_table_words(
        words: &mut Vec<crate::pdf::table_reconstruct::HocrWord>,
        x_positions: &[u32],
        descriptor_prefix: &str,
    ) {
        for (column, left) in x_positions.iter().enumerate() {
            let heading = if column == 0 { "Security" } else { "Value" };
            words.push(make_word(heading, *left, 100, 30));
        }
        for row in 0..8 {
            words.push(make_word(
                &format!("{descriptor_prefix} bond {row}"),
                x_positions[0],
                130 + row * 12,
                60,
            ));
            for (column, left) in x_positions.iter().enumerate().skip(1) {
                words.push(make_word(
                    &format!("{},{}", row + 1, column * 113),
                    *left,
                    130 + row * 12,
                    30,
                ));
            }
        }
    }

    #[test]
    fn splits_independently_valid_side_by_side_tables() {
        let mut words = Vec::new();
        extend_financial_table_words(&mut words, &[20, 140, 220], "Cayman");
        extend_financial_table_words(&mut words, &[320, 400, 480, 560], "Ireland");

        let (left, right) = split_side_by_side_region(&words).expect("central seam should be credible");
        assert!(
            reconstruct_region_table(&left, 792.0, 1, false, 0).is_some(),
            "left child must independently pass the existing validation chain"
        );
        assert!(
            reconstruct_region_table(&right, 792.0, 1, false, 0).is_some(),
            "right child must independently pass the existing validation chain"
        );

        let tables = reconstruct_region_tables(&words, 792.0, 1, false, 0);
        assert_eq!(
            tables.len(),
            2,
            "valid side-by-side children should replace the fused parent"
        );
        assert_eq!(tables[0].cells[0].len(), 3);
        assert_eq!(tables[1].cells[0].len(), 4);
    }

    fn wrapped_financial_side(prefix: &str) -> Table {
        let cells = vec![
            vec![
                format!("{prefix} Alpha"),
                "Series 2024-A".to_string(),
                String::new(),
                String::new(),
            ],
            vec![
                "Class A1R".to_string(),
                "three-month SOFR".to_string(),
                String::new(),
                String::new(),
            ],
            vec![
                "5.87%".to_string(),
                "04/15/37".to_string(),
                "5,718".to_string(),
                "5,742,049".to_string(),
            ],
            vec![
                format!("{prefix} Beta"),
                "Series 2025-B".to_string(),
                String::new(),
                String::new(),
            ],
            vec![
                "Class BR".to_string(),
                "three-month SOFR".to_string(),
                String::new(),
                String::new(),
            ],
            vec![
                "6.08%".to_string(),
                "01/15/34".to_string(),
                "500".to_string(),
                "500,639".to_string(),
            ],
        ];
        Table {
            markdown: crate::pdf::table_reconstruct::table_to_markdown(&cells),
            cells,
            ..Default::default()
        }
    }

    fn wrapped_financial_overflow_side(prefix: &str) -> Table {
        let mut table = wrapped_financial_side(prefix);
        table.cells[0][WRAPPED_FINANCIAL_PAR_COLUMN] = "Series 2X, 3.30%".to_string();
        table.markdown = crate::pdf::table_reconstruct::table_to_markdown(&table.cells);
        table
    }

    #[test]
    fn normalizes_wrapped_side_by_side_financial_tables() {
        let left = wrapped_financial_side("Cayman");
        let right = wrapped_financial_overflow_side("Ireland");

        let (left, right) = normalize_side_by_side_financial_tables(left, right);

        assert_eq!(left.cells.len(), 5);
        assert_eq!(right.cells.len(), 5);
        assert_eq!(left.cells[0], vec!["Description", "Amount", "Value"]);
        assert_eq!(
            left.cells[1],
            vec!["Cayman Alpha Series 2024-A", "", ""],
            "the reconstructed pseudo-header must remain a data row"
        );
        assert_eq!(
            left.cells[4],
            vec![
                "Cayman Beta Series 2025-B Class BR three-month SOFR 6.08% 01/15/34",
                "500",
                "500,639"
            ]
        );
        assert!(
            left.cells.iter().all(|row| row.len() == CANONICAL_FINANCIAL_COLUMNS),
            "normalization must emit the semantic three-column grid"
        );
        assert!(
            left.markdown
                .lines()
                .filter(|line| line.starts_with('|'))
                .all(|line| line.matches('|').count() == CANONICAL_FINANCIAL_COLUMNS + 1),
            "every markdown row and separator must contain three cells"
        );
        assert!(left.markdown.starts_with("| Description | Amount | Value |"));
        assert!(left.markdown.contains("| Cayman Alpha Series 2024-A |  |  |"));
        assert!(left.markdown.contains("| 500 | 500,639 |"));
    }

    #[test]
    fn preserves_wrapped_candidate_when_peer_is_not_recognized() {
        let left = wrapped_financial_side("Cayman");
        let mut right = wrapped_financial_overflow_side("Ireland");
        right.cells[0] = ["Security", "Instrument", "Par", "Value"].map(str::to_string).to_vec();
        right.markdown = crate::pdf::table_reconstruct::table_to_markdown(&right.cells);

        let (normalized_left, normalized_right) = normalize_side_by_side_financial_tables(left.clone(), right.clone());

        assert_eq!(normalized_left.cells, left.cells);
        assert_eq!(normalized_left.markdown, left.markdown);
        assert_eq!(normalized_right.cells, right.cells);
        assert_eq!(normalized_right.markdown, right.markdown);
    }

    #[test]
    fn preserves_paired_wrapped_candidates_when_both_pseudo_headers_have_empty_numeric_tracks() {
        let left = wrapped_financial_side("Cayman");
        let right = wrapped_financial_side("Ireland");

        let (normalized_left, normalized_right) = normalize_side_by_side_financial_tables(left.clone(), right.clone());

        assert_eq!(normalized_left.cells, left.cells);
        assert_eq!(normalized_left.markdown, left.markdown);
        assert_eq!(normalized_right.cells, right.cells);
        assert_eq!(normalized_right.markdown, right.markdown);
    }

    #[test]
    fn preserves_headerless_semantic_four_column_table_with_numeric_first_row() {
        let cells = vec![
            vec!["Alpha Holdings", "Senior note", "100", "1,000"],
            vec!["Series 2024-A", "Floating rate", "", ""],
            vec!["Matures", "04/15/37", "", ""],
            vec!["Class A1R", "USD", "5,718", "5,742,049"],
            vec!["Series 2025-B", "Fixed rate", "", ""],
            vec!["Matures", "01/15/34", "", ""],
            vec!["Class BR", "EUR", "500", "500,639"],
        ]
        .into_iter()
        .map(|row| row.into_iter().map(str::to_string).collect())
        .collect::<Vec<Vec<String>>>();
        let table = Table {
            markdown: crate::pdf::table_reconstruct::table_to_markdown(&cells),
            cells,
            ..Default::default()
        };

        let (left, right) = normalize_side_by_side_financial_tables(table.clone(), table.clone());

        assert_eq!(left.cells, table.cells);
        assert_eq!(right.cells, table.cells);
        assert_eq!(left.markdown, table.markdown);
        assert_eq!(right.markdown, table.markdown);
    }

    #[test]
    fn preserves_descriptor_only_issuer_rows_in_wrapped_financial_tables() {
        let mut left = wrapped_financial_side("Cayman");
        left.cells.insert(
            3,
            vec![
                "TICP CLO VI Ltd.".to_string(),
                String::new(),
                String::new(),
                String::new(),
            ],
        );
        left.markdown = crate::pdf::table_reconstruct::table_to_markdown(&left.cells);
        let mut right = wrapped_financial_overflow_side("Ireland");
        right.cells.insert(
            3,
            vec!["Voya CLO Ltd.".to_string(), String::new(), String::new(), String::new()],
        );
        right.markdown = crate::pdf::table_reconstruct::table_to_markdown(&right.cells);

        let (left, right) = normalize_side_by_side_financial_tables(left, right);

        assert!(
            left.cells
                .contains(&vec!["TICP CLO VI Ltd.".to_string(), String::new(), String::new()])
        );
        assert!(
            right
                .cells
                .contains(&vec!["Voya CLO Ltd.".to_string(), String::new(), String::new()])
        );
        assert!(
            left.cells
                .iter()
                .all(|row| !row[0].contains("TICP CLO VI Ltd. Cayman Beta")),
            "an issuer label must not be folded into the following investment"
        );
    }

    #[test]
    fn separates_repeated_issuer_from_its_first_series() {
        let mut left = wrapped_financial_side("Cayman");
        left.cells[3][0] = "Trinitas CLO XIV Ltd.".to_string();
        left.cells.push(vec![
            "Series 2026-C".to_string(),
            "Class CR".to_string(),
            "250".to_string(),
            "250,639".to_string(),
        ]);
        left.markdown = crate::pdf::table_reconstruct::table_to_markdown(&left.cells);
        let mut right = wrapped_financial_overflow_side("Ireland");
        right.cells[3][0] = "Voya CLO Ltd.".to_string();
        right.cells.push(vec![
            "Series 2026-C".to_string(),
            "Class CR".to_string(),
            "250".to_string(),
            "250,639".to_string(),
        ]);
        right.markdown = crate::pdf::table_reconstruct::table_to_markdown(&right.cells);

        let (left, right) = normalize_side_by_side_financial_tables(left, right);

        assert!(
            left.cells
                .contains(&vec!["Trinitas CLO XIV Ltd.".to_string(), String::new(), String::new()])
        );
        assert!(
            right
                .cells
                .contains(&vec!["Voya CLO Ltd.".to_string(), String::new(), String::new()])
        );
        assert!(
            left.cells
                .iter()
                .any(|row| row[0].starts_with("Series 2025-B Class BR"))
        );
        assert!(
            left.cells
                .iter()
                .all(|row| !row[0].contains("Trinitas CLO XIV Ltd. Series"))
        );
    }

    #[test]
    fn preserves_punctuated_legal_entity_as_part_of_security_description() {
        let mut rows = vec![
            vec!["Description".to_string(), "Amount".to_string(), "Value".to_string()],
            vec![
                "RR 28 Ltd., Series 2024-A Class A1R".to_string(),
                "5,718".to_string(),
                "5,742,049".to_string(),
            ],
            vec![
                "Series 2025-B Class BR".to_string(),
                "500".to_string(),
                "500,639".to_string(),
            ],
        ];
        let expected = rows.clone();

        preserve_repeated_issuer_rows(&mut rows);

        assert_eq!(rows, expected);
    }

    #[test]
    fn preserves_ordinary_four_column_side_by_side_tables() {
        let cells = vec![
            vec!["Security", "Region", "Quantity", "Value"],
            vec!["Alpha", "Cayman", "100", "1,000"],
            vec!["Beta", "Ireland", "200", "2,000"],
            vec!["Gamma", "France", "300", "3,000"],
        ]
        .into_iter()
        .map(|row| row.into_iter().map(str::to_string).collect())
        .collect::<Vec<Vec<String>>>();
        let table = Table {
            markdown: crate::pdf::table_reconstruct::table_to_markdown(&cells),
            cells,
            ..Default::default()
        };

        let (left, right) = normalize_side_by_side_financial_tables(table.clone(), table.clone());

        assert_eq!(left.cells, table.cells);
        assert_eq!(right.cells, table.cells);
        assert_eq!(left.markdown, table.markdown);
        assert_eq!(right.markdown, table.markdown);
    }

    #[test]
    fn preserves_semantic_four_column_financial_tables_with_wrapped_descriptors() {
        let cells = vec![
            vec!["Security", "Instrument", "Par", "Value"],
            vec!["Alpha Holdings", "Senior secured note", "", ""],
            vec!["Series 2024-A", "Floating rate", "", ""],
            vec!["Matures 04/15/37", "USD", "5,718", "5,742,049"],
            vec!["Beta Holdings", "Senior secured note", "", ""],
            vec!["Series 2025-B", "Fixed rate", "", ""],
            vec!["Matures 01/15/34", "EUR", "500", "500,639"],
        ]
        .into_iter()
        .map(|row| row.into_iter().map(str::to_string).collect())
        .collect::<Vec<Vec<String>>>();
        let table = Table {
            markdown: crate::pdf::table_reconstruct::table_to_markdown(&cells),
            cells,
            ..Default::default()
        };

        let (left, right) = normalize_side_by_side_financial_tables(table.clone(), table.clone());

        assert_eq!(left.cells, table.cells);
        assert_eq!(right.cells, table.cells);
        assert_eq!(left.markdown, table.markdown);
        assert_eq!(right.markdown, table.markdown);
    }

    #[test]
    fn does_not_merge_financial_continuation_across_intervening_row() {
        let mut table = wrapped_financial_side("Cayman");
        table.cells.insert(
            1,
            vec![
                "Disclosure".to_string(),
                String::new(),
                "not applicable".to_string(),
                String::new(),
            ],
        );
        table.markdown = crate::pdf::table_reconstruct::table_to_markdown(&table.cells);

        let mut right = table.clone();
        right.cells[0][WRAPPED_FINANCIAL_PAR_COLUMN] = "Series 2X, 3.30%".to_string();
        right.markdown = crate::pdf::table_reconstruct::table_to_markdown(&right.cells);
        let (left, _) = normalize_side_by_side_financial_tables(table, right);

        assert_eq!(left.cells[1][0], "Cayman Alpha Series 2024-A");
        assert_eq!(
            left.cells[2],
            vec!["Disclosure", "not applicable", ""],
            "the intervening row must remain a separate semantic row"
        );
        assert!(
            !left.cells[1][0].contains("Class A1R"),
            "an intervening non-continuation row must stop coalescing"
        );
    }

    #[test]
    fn preserves_ordinary_five_column_table() {
        let mut words = Vec::new();
        extend_grid_words(&mut words, &[20, 120, 220, 320, 420]);

        assert!(
            label_heavy_financial_tracks(&words).is_none(),
            "an ordinary five-column numeric grid must stay on the general reconstruction path"
        );
        assert!(
            split_side_by_side_region(&words).is_none(),
            "occupied central column must prevent a side-by-side split"
        );
    }

    #[test]
    fn does_not_stitch_independent_aligned_financial_tables() {
        let x_positions = [20, 140, 220, 300, 380];
        let mut first = Vec::new();
        extend_financial_table_words(&mut first, &x_positions, "Pension");
        let mut second = Vec::new();
        extend_financial_table_words(&mut second, &x_positions, "Insurance");
        for word in &mut second {
            word.top += 300;
        }

        let (_, first_tracks, _) =
            reconstruct_label_heavy_financial_region(&first, 792.0, 1).expect("first financial table shape");
        let (_, second_tracks, second_tolerance) =
            reconstruct_label_heavy_financial_region(&second, 792.0, 1).expect("second financial table shape");
        assert_eq!(first_tracks.len(), LABEL_HEAVY_FINANCIAL_TRACKS);
        assert_eq!(first_tracks, second_tracks);
        assert!(
            !label_heavy_financial_sections_are_contiguous(&first, &second, &second_tracks, second_tolerance),
            "a large normalized vertical gap must stop same-track tables from chaining"
        );

        let tables = reconstruct_page_region_tables(&[first, second], 792.0, 1, false, 0);
        assert_eq!(tables.len(), 2, "independent aligned tables must remain distinct");
    }

    #[test]
    fn does_not_stitch_close_aligned_table_without_section_label() {
        let x_positions = [20, 140, 220, 300, 380];
        let mut first = Vec::new();
        extend_financial_table_words(&mut first, &x_positions, "Pension");
        let mut second = Vec::new();
        extend_financial_table_words(&mut second, &x_positions, "Insurance");
        second.retain(|word| !(word.top == 100 && word.left == x_positions[0]));
        for word in second.iter_mut().filter(|word| word.top == 100) {
            word.text = "2024".to_string();
        }
        for word in &mut second {
            word.top += 130;
        }

        let (_, second_tracks, second_tolerance) =
            reconstruct_label_heavy_financial_region(&second, 792.0, 1).expect("second financial table shape");
        assert!(
            !label_heavy_financial_sections_are_contiguous(&first, &second, &second_tracks, second_tolerance),
            "a nearby table without a descriptor-only section label must not chain"
        );

        let tables = reconstruct_page_region_tables(&[first, second], 792.0, 1, false, 0);
        assert_eq!(tables.len(), 2, "nearby independent tables must remain distinct");
    }

    #[test]
    fn does_not_stitch_close_aligned_table_with_ordinary_header() {
        let x_positions = [20, 140, 220, 300, 380];
        let mut first = Vec::new();
        extend_financial_table_words(&mut first, &x_positions, "Pension");
        let mut second = Vec::new();
        extend_financial_table_words(&mut second, &x_positions, "Insurance");
        for word in &mut second {
            word.top += 130;
        }

        let (_, second_tracks, second_tolerance) =
            reconstruct_label_heavy_financial_region(&second, 792.0, 1).expect("second financial table shape");
        assert!(
            !label_heavy_financial_sections_are_contiguous(&first, &second, &second_tracks, second_tolerance),
            "a nearby Security/Value header must not be treated as a continuation label"
        );

        let tables = reconstruct_page_region_tables(&[first, second], 792.0, 1, false, 0);
        assert_eq!(tables.len(), 2, "ordinary table headers must remain distinct");
    }

    #[test]
    fn preserves_grouped_eight_column_table_without_descriptors() {
        let mut words = Vec::new();
        extend_grid_words(&mut words, &[20, 100, 180, 260]);
        extend_grid_words(&mut words, &[320, 400, 480, 560]);

        let tables = reconstruct_region_tables(&words, 792.0, 1, false, 0);
        assert_eq!(
            tables.len(),
            1,
            "central grouping alone must not split an ordinary table"
        );
        assert_eq!(tables[0].cells[0].len(), 8);
    }

    /// Heuristic table extraction must not panic on an empty / minimal PDF.
    #[test]
    fn test_extract_tables_heuristic_minimal_pdf_no_panic() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/tiny.pdf");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(&path).expect("read tiny.pdf");
        let mut doc = NativeDocument::open_bytes(&bytes).expect("open tiny.pdf");
        let skip = HashSet::new();
        let extraction = extract_tables_heuristic(&mut doc, false, &skip, Default::default())
            .expect("heuristic must not error on minimal PDF");
        assert!(
            extraction.tables.is_empty(),
            "expected no tables on minimal PDF, got: {:?}",
            extraction.tables
        );
    }

    /// On a real text-layer PDF that contains a table, the heuristic should
    /// produce at least one well-formed `Table` (rows≥2, cols≥2) with a
    /// non-empty markdown rendering. Regression test for #897.
    #[test]
    fn test_extract_tables_heuristic_recovers_table_document() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/table_document.pdf");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(&path).expect("read table_document.pdf");
        let mut doc = NativeDocument::open_bytes(&bytes).expect("open table_document.pdf");

        let skip = HashSet::new();
        let page_count = doc.doc.page_count().expect("table fixture page count");
        let extraction =
            extract_tables_heuristic(&mut doc, false, &skip, Default::default()).expect("heuristic must not error");
        assert_eq!(
            extraction.hierarchy_segments.pages.len(),
            page_count,
            "owned hierarchy segments must preserve one slot per PDF page"
        );
        let tables = extraction.tables;

        if tables.is_empty() {
            eprintln!(
                "extract_tables_heuristic returned 0 tables on table_document.pdf — \
                 fixture may be borderline for the prose filter; verify manually"
            );
            return;
        }

        for t in &tables {
            assert!(t.cells.len() >= 2, "table has fewer than 2 rows: {t:?}");
            assert!(t.cells.iter().any(|r| r.len() >= 2), "no row has 2+ columns: {t:?}");
            assert!(!t.markdown.trim().is_empty(), "markdown is empty: {t:?}");
            assert!(t.page_number >= 1, "page_number must be 1-indexed: {t:?}");
            assert!(
                t.markdown.contains("| --- |") || t.markdown.contains("|---|"),
                "table markdown is missing the header separator row: {:?}",
                t.markdown
            );
            if let Some(bbox) = &t.bounding_box {
                assert!(
                    bbox.y0 < bbox.y1,
                    "bbox y0 must be less than y1 (PDF coords: bottom < top): {bbox:?}"
                );
                assert!(bbox.x0 < bbox.x1, "bbox x0 must be less than x1: {bbox:?}");
            }
        }
    }

    #[test]
    fn test_extract_tables_heuristic_repairs_embedded_fixture_numeric_track() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/pdf/embedded_images_tables.pdf");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(&path).expect("read embedded table fixture");
        let mut doc = NativeDocument::open_bytes(&bytes).expect("open embedded table fixture");

        let tables = extract_tables_heuristic(&mut doc, false, &HashSet::new(), Default::default())
            .expect("heuristic extraction")
            .tables;
        let table = tables
            .iter()
            .find(|table| table.cells[0].iter().any(|cell| cell.contains("Inhibitor")))
            .expect("polarization table");

        assert_eq!(table.cells.len(), 7);
        assert!(table.cells.iter().all(|row| row.len() == 7), "{:?}", table.cells);
        assert!(
            table.cells[0]
                .windows(2)
                .any(|cells| cells == ["icorr (A/cm2)", "Polarization resistance ( Ω )"]),
            "{:?}",
            table.cells[0]
        );
        assert!(
            table
                .markdown
                .contains("| icorr (A/cm2) | Polarization resistance ( Ω ) |"),
            "{}",
            table.markdown
        );
        assert!(
            table.cells[1..]
                .iter()
                .all(|row| row.iter().all(|cell| !cell.trim().is_empty()))
        );
    }

    #[test]
    fn test_extract_tables_heuristic_recovers_acn_financial_table() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/pdf/ft_ACN_2009_page_102_t0.pdf");
        let ground_truth_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/ground_truth/pdf/ft_ACN_2009_page_102_t0.md");
        if !path.exists() || !ground_truth_path.exists() {
            return;
        }
        let bytes = std::fs::read(&path).expect("read ACN financial table fixture");
        let mut doc = NativeDocument::open_bytes(&bytes).expect("open ACN financial table fixture");

        let tables = extract_tables_heuristic(&mut doc, false, &HashSet::new(), Default::default())
            .expect("heuristic extraction")
            .tables;

        assert_eq!(
            tables.len(),
            1,
            "ACN fixture should produce one stitched financial table"
        );
        let table = &tables[0];
        assert_eq!(table.cells.len(), 38);
        assert!(table.cells.iter().all(|row| row.len() == 5), "{:?}", table.cells);
        assert_eq!(table.cells[0], ["", "2009", "", "2008", ""]);
        assert_eq!(
            table.cells[4],
            ["SFAS 158 measurement date adjustment", "9,015", "3,074", "—", "—"]
        );
        for anchor in [
            "Changes in benefit obligation",
            "Changes in plan assets",
            "Reconciliation of funded status",
            "Amounts recognized in the Consolidated Balance Sheets consist of:",
        ] {
            assert!(
                table.markdown.contains(anchor),
                "missing ACN section anchor: {anchor}\n{}",
                table.markdown
            );
        }

        let ground_truth = std::fs::read_to_string(&ground_truth_path).expect("read ACN markdown ground truth");
        let expected_rows = ground_truth
            .lines()
            .filter(|line| line.starts_with('|'))
            .filter(|line| !line.chars().all(|character| matches!(character, '|' | '-' | ' ')))
            .map(|line| {
                line.split('|')
                    .skip(1)
                    .take(LABEL_HEAVY_FINANCIAL_TRACKS + 1)
                    .map(|cell| cell.trim().to_string())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            expected_rows.len(),
            38,
            "the checked-in ACN ground truth contains 38 logical rows"
        );
        assert_eq!(
            table.cells, expected_rows,
            "every reconstructed ACN row must map exactly to the checked-in ground truth"
        );
    }

    /// `skip_pages` (1-indexed) suppresses the heuristic on the listed pages.
    /// This is the per-page composition contract: native finds page N, we
    /// pass `{N}` to the heuristic and it must not emit any tables for page N.
    #[test]
    fn test_extract_tables_heuristic_skip_pages_honored() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/table_document.pdf");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(&path).expect("read table_document.pdf");
        let mut doc = NativeDocument::open_bytes(&bytes).expect("open table_document.pdf");

        let baseline = extract_tables_heuristic(&mut doc, false, &HashSet::new(), Default::default())
            .expect("baseline heuristic")
            .tables;
        if baseline.is_empty() {
            return;
        }
        let pages_baseline_touched: HashSet<u32> = baseline.iter().map(|t| t.page_number).collect();

        let skip = pages_baseline_touched.clone();
        let suppressed = extract_tables_heuristic(&mut doc, false, &skip, Default::default())
            .expect("skip-pages heuristic")
            .tables;

        for t in &suppressed {
            assert!(
                !skip.contains(&t.page_number),
                "page {} appeared in skip set but heuristic still emitted a table: {:?}",
                t.page_number,
                t
            );
        }
    }

    /// Vertical-gap clustering: words all within one band become one region.
    #[test]
    fn test_cluster_words_single_region_not_split() {
        let words = vec![
            make_word("a1", 100, 100, 20),
            make_word("a2", 200, 100, 20),
            make_word("b1", 100, 112, 20),
            make_word("b2", 200, 112, 20),
            make_word("c1", 100, 124, 20),
            make_word("c2", 200, 124, 20),
        ];
        let regions = cluster_words_into_vertical_regions(&words);
        assert_eq!(regions.len(), 1, "expected one region, got {regions:?}");
        assert_eq!(regions[0].len(), 6);
    }

    /// Vertical-gap clustering: words separated by a >1.8× median-height gap
    /// must split into two regions. Both tables sized to survive the retain
    /// filter (≥3 distinct rows, ≥2 distinct x-positions, ≥4 words).
    #[test]
    fn test_cluster_words_two_tables_separated_by_large_gap() {
        let words = vec![
            make_word("a1", 100, 100, 20),
            make_word("a2", 200, 100, 20),
            make_word("b1", 100, 112, 20),
            make_word("b2", 200, 112, 20),
            make_word("c1", 100, 124, 20),
            make_word("c2", 200, 124, 20),
            make_word("d1", 100, 300, 20),
            make_word("d2", 200, 300, 20),
            make_word("e1", 100, 312, 20),
            make_word("e2", 200, 312, 20),
            make_word("f1", 100, 324, 20),
            make_word("f2", 200, 324, 20),
        ];
        let regions = cluster_words_into_vertical_regions(&words);
        assert_eq!(regions.len(), 2, "expected two regions, got {regions:?}");
        for r in &regions {
            assert_eq!(r.len(), 6, "each region should have 6 words: {r:?}");
        }
        let first_top = regions[0].iter().map(|w| w.top).min().unwrap();
        let second_top = regions[1].iter().map(|w| w.top).min().unwrap();
        assert!(first_top < second_top, "regions should be ordered top-to-bottom");
    }

    /// Clustering must reject single-column (only one distinct x-position)
    /// runs — those are lists, not tables.
    #[test]
    fn test_cluster_words_rejects_single_column_run() {
        let words = vec![
            make_word("item1", 100, 100, 40),
            make_word("item2", 100, 112, 40),
            make_word("item3", 100, 124, 40),
            make_word("item4", 100, 136, 40),
        ];
        let regions = cluster_words_into_vertical_regions(&words);
        assert!(
            regions.is_empty(),
            "single-column word run must not survive clustering, got {regions:?}"
        );
    }

    /// Clustering must reject runs with fewer than 3 distinct rows — too small
    /// to be a real table.
    #[test]
    fn test_cluster_words_rejects_two_row_run() {
        let words = vec![
            make_word("a", 100, 100, 20),
            make_word("b", 200, 100, 20),
            make_word("c", 100, 112, 20),
            make_word("d", 200, 112, 20),
        ];
        let regions = cluster_words_into_vertical_regions(&words);
        assert!(
            regions.is_empty(),
            "two-row run must not survive clustering, got {regions:?}"
        );
    }

    /// Clustering returns an empty vec when fewer than 4 input words are given,
    /// since no realistic table can be reconstructed from that.
    #[test]
    fn test_cluster_words_below_minimum_returns_empty() {
        let words = vec![
            make_word("a", 0, 0, 10),
            make_word("b", 0, 12, 10),
            make_word("c", 0, 24, 10),
        ];
        let regions = cluster_words_into_vertical_regions(&words);
        assert!(regions.is_empty());
    }

    /// GH#1358: a rotated (90-degree) table's row/column grid must survive
    /// heuristic clustering.
    ///
    /// `segments_to_words` (`pdf::table_reconstruct`) is the boundary where
    /// PDF segments become the `HocrWord`s that `cluster_words_into_vertical_regions`
    /// groups into rows and columns by raw `left`/`top`. Unlike
    /// `cell_text_in_reading_order` (which already corrects word order
    /// *within* an already-assigned cell via `upright_origin`,
    /// see `test_cell_text_in_reading_order_sorts_rotated_spans_by_advance_axis`
    /// above), `segments_to_words` builds `HocrWord.left`/`.top` straight from
    /// `SegmentData.x`/`.y` — the *page-space* coordinates — with no rotation
    /// correction. For a 90-degree run the table's own row axis is page-x and
    /// its own column axis is page-y, the opposite of the unrotated case, so
    /// raw-page-space clustering groups words that share a row position
    /// (`x`) into the same table *row* when they actually belong to three
    /// different rows at the same column, and disperses one true row's cells
    /// across bogus, single-row "regions" that `cluster_words_into_vertical_regions`'s
    /// `row_ycs.len() >= 3` guard then rejects outright — the region is
    /// dropped, not merely misordered.
    ///
    /// Three rows (`x` = 0.0, 12.0, 24.0) by two columns (`y` = 0.0, 120.0),
    /// all spans sharing one 90-degree rotation, mirroring a small sideways
    /// spec table. Row/column spacing is chosen to satisfy the clustering
    /// tolerances once transformed into the upright frame (`row_tolerance` =
    /// 5, `row_gap_split` = 18 for `height` = 10): a correct implementation
    /// must find exactly one region containing all six words in row-major
    /// reading order.
    #[test]
    fn should_preserve_cell_order_in_a_rotated_table() {
        use crate::pdf::hierarchy::SegmentData;
        use crate::pdf::table_reconstruct::segments_to_words;

        fn rotated_seg(text: &str, x: f32, y: f32) -> SegmentData {
            SegmentData {
                text: text.to_string(),
                x,
                y,
                width: 50.0,
                height: 10.0,
                font_size: 10.0,
                is_bold: false,
                is_italic: false,
                is_monospace: false,
                baseline_y: y,
                rotation_degrees: 90.0,
                assigned_role: None,
            }
        }

        let segments = vec![
            rotated_seg("A1", 0.0, 0.0),
            rotated_seg("B1", 0.0, 120.0),
            rotated_seg("A2", 12.0, 0.0),
            rotated_seg("B2", 12.0, 120.0),
            rotated_seg("A3", 24.0, 0.0),
            rotated_seg("B3", 24.0, 120.0),
        ];

        let page_height = 800.0;
        let words = segments_to_words(&segments, page_height);
        let regions = cluster_words_into_vertical_regions(&words);

        assert_eq!(
            regions.len(),
            1,
            "the rotated table's six words must cluster into a single region; got {regions:?}"
        );
        let ordered: Vec<&str> = regions[0].iter().map(|w| w.text.as_str()).collect();
        assert_eq!(
            ordered,
            vec!["A1", "B1", "A2", "B2", "A3", "B3"],
            "rotated-table words must come out in row-major reading order (row0: A1,B1; \
             row1: A2,B2; row2: A3,B3), not scrambled by raw page-space x/y; got: {ordered:?}"
        );
    }

    /// Build a segment of a sideways table: page-space origin `(x, y)`, painted
    /// on a text matrix rotated by `rotation_degrees`.
    ///
    /// Mirrors the inline builder in
    /// [`should_preserve_cell_order_in_a_rotated_table`] above, but takes the
    /// rotation as a parameter so the negative and 180-degree quadrants — the
    /// ones `snap_run_rotation` in `xberg_native_pdf`'s text extractor also
    /// emits (its snap set is `0 / 90 / 180 / -90`) — can be exercised too.
    fn sideways_table_segment(text: &str, x: f32, y: f32, rotation_degrees: f32) -> crate::pdf::hierarchy::SegmentData {
        crate::pdf::hierarchy::SegmentData {
            text: text.to_string(),
            x,
            y,
            width: 50.0,
            height: 10.0,
            font_size: 10.0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: y,
            rotation_degrees,
            assigned_role: None,
        }
    }

    /// The distinct `HocrWord::left` values produced for `segments`, sorted.
    ///
    /// `left` is the only column signal `cluster_words_into_vertical_regions`
    /// has (its `xs.len() >= 2` retain guard), so collapsing distinct columns
    /// onto one `left` is what makes a table disappear rather than merely come
    /// out misordered.
    fn distinct_word_lefts(segments: &[crate::pdf::hierarchy::SegmentData], page_height: f32) -> Vec<u32> {
        let words = crate::pdf::table_reconstruct::segments_to_words(segments, page_height);
        let mut lefts: Vec<u32> = words.iter().map(|word| word.left).collect();
        lefts.sort_unstable();
        lefts.dedup();
        lefts
    }

    /// GH#1358, the quadrant [`should_preserve_cell_order_in_a_rotated_table`]
    /// does not reach: a table rotated **-90** degrees loses every column.
    ///
    /// `SegmentData::upright_origin` returns `(advance, cross)` by rotating the
    /// page-space origin by `-rotation_degrees`. For `+90` that yields
    /// `advance == +y`, non-negative for all real page content — which is
    /// precisely why the `+90` test above passes. For `-90` it yields
    /// `advance == -y`, **negative** for all real page content, which
    /// `table_reconstruct::segment_to_hocr_word_lifted` used to saturate to `0` when
    /// writing it into the unsigned `HocrWord::left` — so every column of a
    /// `-90` table landed on `left == 0`.
    ///
    /// Revert check (expect RED): drop the `frame.advance` term from
    /// `segment_to_hocr_word_lifted` and `split_segment_to_words`, restoring the bare
    /// `advance.round().max(0.0) as u32`, and this asserts `[0]`.
    ///
    /// Two columns 120 units apart on the advance axis (`y` = 520 and `y` =
    /// 400; advance runs along `-y`, so `y = 520` is the leading column) across
    /// three rows. The assertion is deliberately translation-invariant — it
    /// pins the *separation*, not absolute positions — so it constrains
    /// `UprightFrames` to preserve relative geometry without prescribing the
    /// particular origin it chooses.
    #[test]
    fn should_keep_columns_distinct_when_a_table_is_rotated_minus_ninety_degrees() {
        let segments = vec![
            sideways_table_segment("A1", 124.0, 520.0, -90.0),
            sideways_table_segment("B1", 124.0, 400.0, -90.0),
            sideways_table_segment("A2", 112.0, 520.0, -90.0),
            sideways_table_segment("B2", 112.0, 400.0, -90.0),
            sideways_table_segment("A3", 100.0, 520.0, -90.0),
            sideways_table_segment("B3", 100.0, 400.0, -90.0),
        ];

        let lefts = distinct_word_lefts(&segments, 800.0);

        assert_eq!(
            lefts.len(),
            2,
            "a -90-degree table's two columns must stay two distinct word x-positions, but the \
             unsigned HocrWord::left clamp collapsed them onto {lefts:?}"
        );
        assert_eq!(
            lefts[1] - lefts[0],
            120,
            "the two columns are 120 units apart on the rotated table's advance axis; got {lefts:?}"
        );
    }

    /// GH#1358: a **-90**-degree table must survive clustering, exactly as the
    /// `+90` one does in [`should_preserve_cell_order_in_a_rotated_table`].
    ///
    /// Same geometry as
    /// [`should_keep_columns_distinct_when_a_table_is_rotated_minus_ninety_degrees`],
    /// carried one stage further so the user-visible consequence is pinned:
    /// under the old per-word advance clamp all six words sat on `left == 0`,
    /// so the `xs.len() >= 2` guard in `cluster_words_into_vertical_regions`
    /// dropped the region outright. The page then yielded **zero** table
    /// candidates and the sideways table came out as prose — the reported
    /// #1358 symptom — rather than as a scrambled grid.
    ///
    /// This is the per-page path: `extract_tables_heuristic` runs exactly this
    /// `segments_to_words` -> `cluster_words_into_vertical_regions` pair once
    /// per page and stamps the resulting `Table::page_number`, which is what
    /// `assign_tables_and_images_to_pages` later keys `pages[].content` on, so
    /// a document-level table count can be satisfied by other pages while this
    /// page carries none.
    ///
    /// Row geometry: the cross axis is page-`x`, so `top = page_height - x -
    /// height` and the row nearest the top of the reading frame is the one at
    /// the largest `x`. Rows at `x` = 124 / 112 / 100 are 12 apart, inside the
    /// `row_gap_split` of 18 and outside the `row_tolerance` of 5 for
    /// `height = 10`, so a correct implementation must find exactly one region
    /// of three rows by two columns.
    #[test]
    fn should_cluster_a_minus_ninety_rotated_table_into_a_single_region() {
        let segments = vec![
            sideways_table_segment("A1", 124.0, 520.0, -90.0),
            sideways_table_segment("B1", 124.0, 400.0, -90.0),
            sideways_table_segment("A2", 112.0, 520.0, -90.0),
            sideways_table_segment("B2", 112.0, 400.0, -90.0),
            sideways_table_segment("A3", 100.0, 520.0, -90.0),
            sideways_table_segment("B3", 100.0, 400.0, -90.0),
        ];

        let words = crate::pdf::table_reconstruct::segments_to_words(&segments, 800.0);
        let regions = cluster_words_into_vertical_regions(&words);

        assert_eq!(
            regions.len(),
            1,
            "the -90-degree table's six words must cluster into a single region; the column \
             collapse makes the run single-column, which clustering rejects outright, so this \
             page contributes no table at all; got {regions:?}"
        );
        let ordered: Vec<&str> = regions[0].iter().map(|word| word.text.as_str()).collect();
        assert_eq!(
            ordered,
            vec!["A1", "B1", "A2", "B2", "A3", "B3"],
            "-90-degree table words must come out in row-major reading order (row0: A1,B1; \
             row1: A2,B2; row2: A3,B3); got {ordered:?}"
        );
    }

    /// GH#1358, the other negative-advance quadrant: an upside-down (**180**
    /// degree) table also loses every column.
    ///
    /// `upright_origin` for 180 degrees returns `advance == -x`, negative for
    /// all real page content, so the same per-word clamp saturated every column
    /// onto `left == 0` here too. 180 is in `snap_run_rotation`'s snap set
    /// alongside `0 / 90 / -90`, so the backend does emit it, and it needs its
    /// own case because `UprightFrames` measures each rotation separately.
    ///
    /// Columns 120 apart on page-`x` (advance runs along `-x`, so `x = 220` is
    /// the leading column), three rows on page-`y`.
    #[test]
    fn should_keep_columns_distinct_when_a_table_is_rotated_one_hundred_eighty_degrees() {
        let segments = vec![
            sideways_table_segment("A1", 220.0, 100.0, 180.0),
            sideways_table_segment("B1", 100.0, 100.0, 180.0),
            sideways_table_segment("A2", 220.0, 112.0, 180.0),
            sideways_table_segment("B2", 100.0, 112.0, 180.0),
            sideways_table_segment("A3", 220.0, 124.0, 180.0),
            sideways_table_segment("B3", 100.0, 124.0, 180.0),
        ];

        let lefts = distinct_word_lefts(&segments, 800.0);

        assert_eq!(
            lefts.len(),
            2,
            "a 180-degree table's two columns must stay two distinct word x-positions, but the \
             unsigned HocrWord::left clamp collapsed them onto {lefts:?}"
        );
        assert_eq!(
            lefts[1] - lefts[0],
            120,
            "the two columns are 120 units apart on the rotated table's advance axis; got {lefts:?}"
        );
    }

    /// GH#1358, the cross axis: a `-90` table on a page **wider than the
    /// `page_height` reference** loses its rows the same way the advance clamp
    /// lost its columns.
    ///
    /// For `-90` the cross axis is page-`x`, so
    /// `top = page_height - (x + height)`. `page_height` is derived from the
    /// segments' own `y` extent with a 792 floor
    /// (`extract_tables_heuristic`, `structure::pipeline`), so on landscape
    /// content whose `x` runs past that reference — A4 landscape is 842 wide
    /// against a 792 floor — `top` goes negative and the per-word
    /// `.max(0.0)` flattened every row onto `top == 0`. Clustering's
    /// `row_ycs.len() >= 3` guard then rejected the region, so the table
    /// disappeared even once its columns were distinct.
    ///
    /// Revert check (expect RED): drop the `frame.top` term from
    /// `segment_to_hocr_word_lifted` and `split_segment_to_words` and this finds zero
    /// regions, because all six words collapse onto a single row.
    ///
    /// Rows at `x` = 800 / 812 / 824 against `page_height` 792 give raw tops of
    /// -18 / -30 / -42; the frame translation lifts them to 24 / 12 / 0, which
    /// is three rows 12 apart — inside `row_gap_split` (18) and outside
    /// `row_tolerance` (5).
    #[test]
    fn should_keep_rows_distinct_when_a_minus_ninety_table_runs_past_the_page_height_reference() {
        let segments = vec![
            sideways_table_segment("A1", 824.0, 520.0, -90.0),
            sideways_table_segment("B1", 824.0, 400.0, -90.0),
            sideways_table_segment("A2", 812.0, 520.0, -90.0),
            sideways_table_segment("B2", 812.0, 400.0, -90.0),
            sideways_table_segment("A3", 800.0, 520.0, -90.0),
            sideways_table_segment("B3", 800.0, 400.0, -90.0),
        ];

        let words = crate::pdf::table_reconstruct::segments_to_words(&segments, 792.0);

        let mut tops: Vec<u32> = words.iter().map(|word| word.top).collect();
        tops.sort_unstable();
        tops.dedup();
        assert_eq!(
            tops,
            vec![0, 12, 24],
            "the three rows must stay 12 apart after the frame is lifted into non-negative \
             image space; a per-word clamp flattens them all onto 0"
        );

        let regions = cluster_words_into_vertical_regions(&words);
        assert_eq!(
            regions.len(),
            1,
            "a -90 table wider than the page_height reference must still cluster into one \
             region; got {regions:?}"
        );
    }

    #[test]
    fn dense_numeric_region_caps_adaptive_column_gap() {
        let mut words = Vec::new();
        for row in 0..7 {
            for col in 0..7 {
                words.push(make_word("1.000", 20 + col * 120, 100 + row as u32 * 12, 20));
            }
        }

        let adaptive = crate::pdf::structure::regions::tables::compute_adaptive_column_gap(&words, 900.0);
        assert!(adaptive > DENSE_NUMERIC_COLUMN_GAP_CAP);
        assert_eq!(heuristic_column_gap(&words, 900.0), DENSE_NUMERIC_COLUMN_GAP_CAP);
    }

    #[test]
    fn aligned_multiline_header_attaches_to_compact_numeric_table() {
        let mut header = Vec::new();
        for col in 0..7 {
            header.push(make_word("Heading", 20 + col * 120, 100, 50));
            header.push(make_word("unit", 20 + col * 120, 112, 30));
        }
        let mut data = Vec::new();
        for row in 0..6 {
            for col in 0..7 {
                data.push(make_word("1.000", 20 + col * 120, 130 + row * 12, 30));
            }
        }
        let expected_words = header.len() + data.len();
        let mut regions = vec![header, data];

        attach_aligned_numeric_headers(&mut regions, 10, 5);

        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].len(), expected_words);
    }

    #[test]
    fn compact_numeric_table_reconstructs_end_to_end() {
        let mut words = Vec::new();
        for col in 0..7 {
            words.push(make_word("Heading", 20 + col * 100, 100, 30));
        }
        for row in 0..6 {
            for col in 0..7 {
                words.push(make_word("1.000", 20 + col * 100, 130 + row * 12, 30));
            }
        }

        let regions = cluster_words_into_vertical_regions(&words);
        assert_eq!(
            regions.len(),
            1,
            "header and compact numeric body should form one candidate"
        );
        let table =
            reconstruct_region_table(&regions[0], 792.0, 1, false, 0).expect("compact table should reconstruct");

        assert_eq!(table.cells.len(), 7);
        assert!(table.cells.iter().all(|row| row.len() == 7));
        assert!(!table.markdown.trim().is_empty());
    }

    fn alternating_numeric_tracks(
        header: [&str; 2],
        word_width: u32,
    ) -> (Vec<Vec<String>>, Vec<crate::pdf::table_reconstruct::HocrWord>) {
        let mut grid = vec![vec!["ID".to_string(), header[0].to_string(), header[1].to_string()]];
        let mut words = Vec::new();
        for row in 0..6 {
            let mut cells = vec![(row + 1).to_string(), String::new(), String::new()];
            let column = if row % 3 == 0 { 2 } else { 1 };
            cells[column] = format!("{}.{row}", row + 1);
            words.push(make_word(
                &cells[column],
                if column == 1 { 100 } else { 104 },
                120 + row * 12,
                word_width,
            ));
            grid.push(cells);
        }
        (grid, words)
    }

    #[test]
    fn repairs_overlapping_mutually_exclusive_numeric_tracks() {
        let (mut grid, words) = alternating_numeric_tracks(["Polarization resistance", ""], 20);

        assert!(repair_split_numeric_track(&mut grid, &words, &mut vec![20, 100, 104]));
        assert!(grid.iter().all(|row| row.len() == 2));
        assert_eq!(grid[0][1], "Polarization resistance");
        assert!(grid[1..].iter().all(|row| !row[1].is_empty()));
    }

    /// A successful repair collapses two grid columns into one, so it must drop the merged
    /// boundary from `column_positions` as well. Callers hand BOTH to
    /// `is_well_formed_borderless_table`, so a stale extra position leaves the grid one column
    /// narrower than the positions describing it.
    ///
    /// Before this was fixed the function took `column_positions` as an immutable slice and
    /// could not shrink it: this asserted length would be 3 against a 2-column grid, and the
    /// surviving boundary set would still contain the spurious 104 track.
    #[test]
    fn repair_drops_the_merged_boundary_from_the_column_positions() {
        let (mut grid, words) = alternating_numeric_tracks(["Polarization resistance", ""], 20);
        let mut column_positions = vec![20, 100, 104];

        assert!(repair_split_numeric_track(&mut grid, &words, &mut column_positions));

        assert_eq!(
            column_positions.len(),
            grid[0].len(),
            "grid width and column_positions must stay in lockstep through a repair"
        );
        assert_eq!(
            column_positions,
            vec![20, 100],
            "the surviving cluster keeps the left anchor; the coincident 104 track is spurious"
        );
    }

    /// A REJECTED repair must leave `column_positions` untouched — the early returns must not
    /// half-apply the merge.
    #[test]
    fn rejected_repair_leaves_the_column_positions_unchanged() {
        let (mut grid, words) = alternating_numeric_tracks(["Debit", "Credit"], 20);
        let mut column_positions = vec![20, 100, 104];

        assert!(!repair_split_numeric_track(&mut grid, &words, &mut column_positions));
        assert_eq!(column_positions, vec![20, 100, 104]);
    }

    #[test]
    fn preserves_named_adjacent_sparse_numeric_columns() {
        let (mut grid, words) = alternating_numeric_tracks(["Debit", "Credit"], 20);

        assert!(!repair_split_numeric_track(&mut grid, &words, &mut vec![20, 100, 104]));
        assert!(grid.iter().all(|row| row.len() == 3));
    }

    #[test]
    fn preserves_unnamed_adjacent_sparse_numeric_columns() {
        let (mut grid, words) = alternating_numeric_tracks(["", ""], 20);

        assert!(!repair_split_numeric_track(&mut grid, &words, &mut vec![20, 100, 104]));
        assert!(grid.iter().all(|row| row.len() == 3));
    }

    #[test]
    fn preserves_geometrically_distinct_alternating_numeric_tracks() {
        let (mut grid, words) = alternating_numeric_tracks(["Combined amount", ""], 4);

        assert!(!repair_split_numeric_track(&mut grid, &words, &mut vec![20, 100, 104]));
        assert!(grid.iter().all(|row| row.len() == 3));
    }

    #[test]
    fn preserves_alternating_numeric_tracks_with_late_nonnumeric_cell() {
        let (mut grid, words) = alternating_numeric_tracks(["", ""], 20);
        grid.push(vec!["7".to_string(), "N/A footnote".to_string(), String::new()]);

        assert!(!repair_split_numeric_track(&mut grid, &words, &mut vec![20, 100, 104]));
        assert!(grid.iter().all(|row| row.len() == 3));
    }

    /// `post_process_table`'s `merge_header_only_column` drops a column whose header has
    /// text but whose data is empty on every row (here: "Extra", the third of three
    /// detected columns). `is_well_formed_borderless_table`'s straddled-boundary check
    /// must not go on testing that dropped column's boundary afterwards: nothing was
    /// ever placed there, so it reads as clean whitespace and dilutes the ratio.
    ///
    /// The surviving "Name"/"Value" boundary is straddled by a wide word on 5 of 6 data
    /// rows (5/7 ≈ 0.71, above the 0.60 `MAX_STRADDLED_BOUNDARY_RATIO`), which alone
    /// should reject this as prose, not a real table. Testing the stale "Value"/"Extra"
    /// boundary too (never straddled, since "Extra" has no data) halves the ratio to
    /// 5/14 ≈ 0.36 — under the threshold — so the unfixed code wrongly accepts it
    /// (xberg-io/xberg#863).
    #[test]
    fn stale_column_position_after_header_only_column_merge_does_not_mask_prose() {
        let mut region = vec![
            make_word("Name", 0, 0, 30),
            make_word("Value", 150, 0, 30),
            make_word("Extra", 300, 0, 30),
        ];
        // 5 rows where one wide word starts in the "Name" column and runs past the
        // "Value" boundary — the straddle a real column gap would never allow.
        for row in 0..5 {
            region.push(make_word("P", 0, 40 + row * 40, 200));
        }
        // 1 clean row that anchors "Value" as a real, narrow, non-straddled column.
        region.push(make_word("P", 0, 240, 30));
        region.push(make_word("42", 150, 240, 30));

        let result = reconstruct_region_table_with_column_gap(&region, 792.0, 1, false, 30, 0);

        assert!(
            matches!(result, Err(HeuristicTableRejection::NotWellFormed)),
            "expected the straddled 'Name'/'Value' boundary alone to reject this as prose, got {result:?}"
        );
    }

    #[test]
    fn unaligned_prose_does_not_attach_to_compact_numeric_table() {
        let header = vec![
            make_word("This", 500, 100, 30),
            make_word("paragraph", 550, 100, 60),
            make_word("continues", 620, 112, 50),
        ];
        let mut data = Vec::new();
        for row in 0..6 {
            for col in 0..7 {
                data.push(make_word("1.000", 20 + col * 80, 130 + row * 12, 30));
            }
        }
        let mut regions = vec![header, data];

        attach_aligned_numeric_headers(&mut regions, 10, 5);

        assert_eq!(regions.len(), 2);
    }

    #[test]
    fn alphabetic_region_keeps_adaptive_column_gap() {
        let mut words = Vec::new();
        for row in 0..7 {
            for col in 0..8 {
                words.push(make_word("value", 20 + col * 120, 100 + row as u32 * 12, 20));
            }
        }

        let adaptive = crate::pdf::structure::regions::tables::compute_adaptive_column_gap(&words, 900.0);
        assert_eq!(heuristic_column_gap(&words, 900.0), adaptive);
    }

    #[test]
    fn multi_column_prose_does_not_cap_adaptive_column_gap() {
        let mut words = Vec::new();
        for row in 0..8 {
            for col in 0..3 {
                words.push(make_word(
                    if row % 3 == 0 { "2024" } else { "paragraph" },
                    20 + col * 280,
                    100 + row * 12,
                    100,
                ));
            }
        }

        let adaptive = crate::pdf::structure::regions::tables::compute_adaptive_column_gap(&words, 800.0);
        assert_eq!(heuristic_column_gap(&words, 800.0), adaptive);
    }

    #[test]
    fn sparse_numeric_rows_do_not_cap_adaptive_column_gap() {
        let mut words = Vec::new();
        for row in 0..9 {
            let text = if row % 2 == 0 { "1.000" } else { "value" };
            for col in 0..5 {
                words.push(make_word(text, 20 + col * 140, 100 + row * 12, 30));
            }
        }

        let adaptive = crate::pdf::structure::regions::tables::compute_adaptive_column_gap(&words, 700.0);
        assert_eq!(heuristic_column_gap(&words, 700.0), adaptive);
    }

    #[test]
    fn heuristic_empty_region_reports_rejection_reason() {
        let rejection = reconstruct_region_table_with_reason(&[], 792.0, 1, false, 0).unwrap_err();
        assert_eq!(rejection, HeuristicTableRejection::EmptyGrid);
    }

    #[test]
    fn heuristic_code_listing_reports_rejection_reason() {
        let words = vec![
            make_word("fn", 20, 100, 20),
            make_word("main", 100, 100, 30),
            make_word("{", 20, 112, 10),
            make_word("call();", 100, 112, 50),
            make_word("}", 20, 124, 10),
            make_word("// end", 100, 124, 50),
        ];
        let rejection = reconstruct_region_table_with_reason(&words, 792.0, 1, false, 0).unwrap_err();
        assert!(
            matches!(
                rejection,
                HeuristicTableRejection::PostProcessing | HeuristicTableRejection::CodeListing
            ),
            "unexpected rejection: {rejection:?}"
        );
    }

    #[test]
    fn repairs_unmatched_header_delimiter_across_adjacent_cells() {
        let mut rows = vec![
            vec![
                "icorr (A/cm2".to_string(),
                ") Polarization resistance ( Ω )".to_string(),
            ],
            vec!["1.25".to_string(), "320".to_string()],
        ];

        repair_heuristic_header_delimiters(&mut rows);

        assert_eq!(rows[0], ["icorr (A/cm2)", "Polarization resistance ( Ω )"]);
        assert_eq!(rows[1], ["1.25", "320"]);
    }

    #[test]
    fn repairs_joined_header_closer_spacing() {
        let mut rows = vec![vec![
            "icorr (A/cm2 )".to_string(),
            "Polarization resistance ( Ω )".to_string(),
        ]];

        repair_heuristic_header_delimiters(&mut rows);

        assert_eq!(rows[0], ["icorr (A/cm2)", "Polarization resistance ( Ω )"]);
    }

    #[test]
    fn repairs_each_supported_header_delimiter() {
        for (left, right, expected_left, expected_right) in [
            ("mean (ms", ") latency", "mean (ms)", "latency"),
            ("range [low", "] high", "range [low]", "high"),
            ("mapping {key", "} value", "mapping {key}", "value"),
        ] {
            let mut rows = vec![vec![left.to_string(), right.to_string()]];

            repair_heuristic_header_delimiters(&mut rows);

            assert_eq!(rows[0], [expected_left, expected_right]);
        }
    }

    #[test]
    fn preserves_balanced_mismatched_and_nonleading_header_delimiters() {
        for (left, right) in [
            ("rate (ms)", ") latency"),
            ("range [low", ") latency"),
            ("rate (ms", "latency ) total"),
            ("ordinary header", ") notes"),
            ("rate (ms", ")) latency"),
            ("rate (ms", ") ] latency"),
        ] {
            let mut rows = vec![vec![left.to_string(), right.to_string()]];
            let expected = rows.clone();

            repair_heuristic_header_delimiters(&mut rows);

            assert_eq!(rows, expected, "unexpected repair for {left:?} and {right:?}");
        }
    }

    #[test]
    fn preserves_data_row_delimiter_boundaries() {
        let mut rows = vec![
            vec!["Metric".to_string(), "Value".to_string()],
            vec!["sample (raw".to_string(), ") normalized".to_string()],
        ];
        let expected = rows.clone();

        repair_heuristic_header_delimiters(&mut rows);

        assert_eq!(rows, expected);
    }

    #[test]
    fn respects_nested_header_delimiter_order() {
        let mut matching = vec![vec!["Statistic ([SD".to_string(), "] range".to_string()]];
        repair_heuristic_header_delimiters(&mut matching);
        assert_eq!(matching[0], ["Statistic ([SD]", "range"]);

        let mut mismatched = vec![vec!["Statistic ([SD".to_string(), ") range".to_string()]];
        let expected = mismatched.clone();
        repair_heuristic_header_delimiters(&mut mismatched);
        assert_eq!(mismatched, expected);
    }

    #[test]
    fn test_cell_text_in_reading_order_fallback_to_cell_text() {
        use xberg_native_pdf::structure::table_extractor::TableCell;

        let cell = TableCell {
            text: "  hello\nworld  ".to_string(),
            colspan: 1,
            rowspan: 1,
            mcids: vec![],
            spans: vec![],
            bbox: None,
            is_header: false,
        };

        let text = cell_text_in_reading_order(&cell);
        assert_eq!(
            text, "hello world",
            "fallback must trim and collapse newlines; got: {text:?}"
        );
    }

    /// Build a 2-column, 5-row stroke-bordered table (outer rect + one
    /// vertical divider + four horizontal dividers) as raw PDF bytes.
    ///
    /// Hand-built rather than via the (now-removed) `DocumentBuilder`
    /// writer: the two tests below assert on the GRID structure derived
    /// from the stroked lines (row/column counts), not on exact text
    /// placement, so a minimal `re`/`m`/`l`/`S` content stream reproduces
    /// the same geometry the writer used to emit. Coordinates match the
    /// original writer-built fixture exactly: outer border
    /// `50,550` to `400,750` (350x200), vertical divider at `x=200`,
    /// horizontal dividers at `y=710,670,630,590`.
    fn build_two_column_bordered_table_pdf() -> Vec<u8> {
        let rows: [(f32, &str, &str); 5] = [
            (710.0, "Item", "Status"),
            (670.0, "8", "Not correct"),
            (630.0, "27", "Incomplete"),
            (590.0, "29,30", "Missing data"),
            (550.0, "45", "Fixed"),
        ];

        let mut content = String::new();
        content.push_str("1 w\n0 0 0 RG\n");
        content.push_str("50 550 350 200 re S\n");
        content.push_str("200 550 m 200 750 l S\n");
        for y in [710.0_f32, 670.0, 630.0, 590.0] {
            content.push_str(&format!("50 {y} m 400 {y} l S\n"));
        }
        content.push_str("BT /Helvetica 10 Tf\n");
        for (row_bottom, left_text, right_text) in rows {
            let baseline = row_bottom + 20.0 - 4.0;
            content.push_str(&format!("1 0 0 1 55 {baseline} Tm ({left_text}) Tj\n"));
            content.push_str(&format!("1 0 0 1 205 {baseline} Tm ({right_text}) Tj\n"));
        }
        content.push_str("ET\n");

        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = vec![0usize];

        offsets.push(pdf.len());
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        offsets.push(pdf.len());
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

        offsets.push(pdf.len());
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] \
              /Contents 4 0 R /Resources << /Font << /Helvetica 5 0 R >> >> >>\nendobj\n",
        );

        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
        pdf.extend_from_slice(content.as_bytes());
        pdf.extend_from_slice(b"\nendstream\nendobj\n");

        offsets.push(pdf.len());
        pdf.extend_from_slice(
            b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
              /Encoding /WinAnsiEncoding >>\nendobj\n",
        );

        let xref_pos = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for &off in &offsets[1..] {
            pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                offsets.len(),
                xref_pos
            )
            .as_bytes(),
        );

        pdf
    }

    /// `extract_tables_native` (strict, min_table_columns=3) must NOT detect a
    /// 2-column bordered table. Regression baseline for issue #964.
    #[test]
    fn extract_tables_native_misses_two_column_bordered_table() {
        let bytes = build_two_column_bordered_table_pdf();
        let mut doc = NativeDocument::open_bytes(&bytes).expect("open synthetic PDF");
        let (tables, warnings) = extract_tables_native(&mut doc).expect("extract_tables_native must not error");
        assert!(
            tables.is_empty(),
            "extract_tables_native (strict, min 3 cols) must not detect a 2-column table; got: {tables:?}"
        );
        assert_eq!(
            warnings.len(),
            0,
            "a page with no table (not a failure) must not produce a warning; got: {warnings:?}"
        );
    }

    /// `extract_tables_bordered` (relaxed, min_table_columns=2) must detect the
    /// 2-column stroke-bordered table that `extract_tables_native` skips.
    #[test]
    fn extract_tables_bordered_detects_two_column_bordered_table() {
        let bytes = build_two_column_bordered_table_pdf();
        let mut doc = NativeDocument::open_bytes(&bytes).expect("open synthetic PDF");
        let skip = HashSet::new();
        let (tables, warnings) =
            extract_tables_bordered(&mut doc, &skip).expect("extract_tables_bordered must not error");
        assert!(
            !tables.is_empty(),
            "extract_tables_bordered must detect the 2-column stroke-bordered table"
        );
        let table = &tables[0].table;
        assert_eq!(table.cells.len(), 5, "expected 5 rows, got {}", table.cells.len());
        assert!(
            table.cells.iter().all(|row| row.len() == 2),
            "all rows must have 2 columns; rows: {:?}",
            table.cells.iter().map(|r| r.len()).collect::<Vec<_>>()
        );
        assert_eq!(table.page_number, 1);
        assert!(!table.markdown.trim().is_empty(), "must produce non-empty markdown");
        assert_eq!(
            warnings.len(),
            0,
            "a successfully detected table must not produce a warning; got: {warnings:?}"
        );
    }

    /// `extract_tables_bordered` must skip pages listed in `skip_pages`.
    #[test]
    fn extract_tables_bordered_skip_pages_honored() {
        let bytes = build_two_column_bordered_table_pdf();
        let mut doc = NativeDocument::open_bytes(&bytes).expect("open synthetic PDF");
        let mut skip = HashSet::new();
        skip.insert(1u32);
        let (tables, warnings) =
            extract_tables_bordered(&mut doc, &skip).expect("extract_tables_bordered must not error");
        assert!(
            tables.is_empty(),
            "skip_pages={{1}} must suppress the only page; got: {tables:?}"
        );
        assert_eq!(
            warnings.len(),
            0,
            "a page suppressed by skip_pages must not produce a warning; got: {warnings:?}"
        );
    }

    /// `native_table_extraction_failure_warning` must name the operation
    /// ("table extraction"), the exact page number, and embed the underlying xberg_native_pdf
    /// error, so a per-page native-detection failure is visible to callers instead of
    /// being indistinguishable from a page that simply has no table.
    #[test]
    fn native_table_extraction_failure_warning_names_page_and_cause() {
        let error = xberg_native_pdf::Error::InvalidPdf("corrupt content stream".to_string());
        let warning = native_table_extraction_failure_warning(3, &error);
        assert_eq!(warning.source.as_ref(), "pdf_tables");
        assert_eq!(
            warning.message.as_ref(),
            "table extraction failed for page 3: Invalid PDF: corrupt content stream; \
             tables on this page were skipped"
        );
    }

    /// `bordered_table_extraction_failure_warning` must name the operation
    /// ("bordered table extraction"), the exact page number, and embed the underlying
    /// xberg_native_pdf error.
    #[test]
    fn bordered_table_extraction_failure_warning_names_page_and_cause() {
        let error = xberg_native_pdf::Error::InvalidPdf("malformed table grid".to_string());
        let warning = bordered_table_extraction_failure_warning(7, &error);
        assert_eq!(warning.source.as_ref(), "pdf_tables");
        assert_eq!(
            warning.message.as_ref(),
            "bordered table extraction failed for page 7: Invalid PDF: malformed table grid; \
             bordered tables on this page were skipped"
        );
    }

    /// Page height of the xberg-io/xberg#1760 reproducer (595.3 x 793.7).
    const GH1760_PAGE_HEIGHT: f32 = 793.7;
    /// Word height the reproducer's 8 pt body text produces. Its lines sit 10.45 pt
    /// apart, which `segments_to_words` rounds to alternating 10/11 unit steps.
    const GH1760_BODY_HEIGHT: u32 = 8;
    /// Word height the 6.4 pt reference list produces. The list is the bulk of the
    /// page's words, so it is what sets the median height the split threshold scales.
    const GH1760_REFERENCE_HEIGHT: u32 = 6;

    /// One measured line of the reproducer: its text, its line box's left and right
    /// edges, the top of its row, and its word height — all in the image coordinates
    /// `segments_to_words` emits, where `top` counts down from the page's top edge.
    struct Gh1760Line {
        text: &'static str,
        left: u32,
        right: u32,
        top: u32,
        height: u32,
    }

    /// Lay a line's words out across its measured line box the way
    /// `split_segment_to_words_lifted` does: each word's box is interpolated from its
    /// byte offset within the line. `jitter` widens every word by one unit, standing in
    /// for the issue's `whole-lines` variant — the same line drawn as one `Tj` rather
    /// than one `Tm` + `TJ` per word, which reaches this code only as different word
    /// boxes. (`regular-heading` is not representable at all: `HocrWord` carries no
    /// font, so a bold and a regular `References` are the same input here.) ~keep
    fn gh1760_push_line(words: &mut Vec<crate::pdf::table_reconstruct::HocrWord>, line: &Gh1760Line, jitter: u32) {
        let total = line.text.len() as f32;
        let span = (line.right - line.left) as f32;
        let mut search_start = 0usize;
        for token in line.text.split_whitespace() {
            let offset = line.text[search_start..]
                .find(token)
                .map(|position| search_start + position)
                .expect("split_whitespace tokens are substrings of their line");
            search_start = offset + token.len();
            let width = ((token.len() as f32 / total) * span).round().max(1.0) as u32;
            words.push(crate::pdf::table_reconstruct::HocrWord {
                text: token.to_string(),
                left: line.left + ((offset as f32 / total) * span).round() as u32,
                top: line.top,
                width: width + jitter,
                height: line.height,
                confidence: 95.0,
            });
        }
    }

    /// The reproducer's defect page: the left column's paragraph ending beside the
    /// right column's DOI tail and its `References` heading, above the 6.4 pt
    /// reference list. Line boxes are the issue's measured ones; row tops are those
    /// xberg derives from the file (51, 62, 72, 83, 93, 103, 114 — a 10.45 pt pitch
    /// rounded to whole units).
    fn gh1760_two_column_band() -> Vec<crate::pdf::table_reconstruct::HocrWord> {
        gh1760_two_column_band_with(103, 0)
    }

    fn gh1760_two_column_band_with(references_top: u32, jitter: u32) -> Vec<crate::pdf::table_reconstruct::HocrWord> {
        let body = GH1760_BODY_HEIGHT;
        let mut lines = vec![
            Gh1760Line {
                text: "may be over or under-estimated. Nevertheless, the information provided",
                left: 38,
                right: 289,
                top: 51,
                height: body,
            },
            Gh1760Line {
                text: "by the study indicates the need for better quality control of vac-",
                left: 38,
                right: 289,
                top: 62,
                height: body,
            },
            Gh1760Line {
                text: "cines in Africa and that improved vaccination outcomes require",
                left: 38,
                right: 289,
                top: 72,
                height: body,
            },
            Gh1760Line {
                text: "more potent vaccines and/or careful attention to revaccination",
                left: 38,
                right: 289,
                top: 83,
                height: body,
            },
            Gh1760Line {
                text: "schedules.",
                left: 38,
                right: 74,
                top: 93,
                height: body,
            },
            Gh1760Line {
                text: "6. Recommendations",
                left: 38,
                right: 120,
                top: 114,
                height: body,
            },
            Gh1760Line {
                text: "Appendix A. Supplementary data",
                left: 307,
                right: 435,
                top: 51,
                height: body,
            },
            Gh1760Line {
                text: "Supplementary data to this article can be found online at https://doi.",
                left: 319,
                right: 558,
                top: 72,
                height: body,
            },
            Gh1760Line {
                text: "org/10.1016/j.vaccine.2024.126325.",
                left: 307,
                right: 438,
                top: 83,
                height: body,
            },
            Gh1760Line {
                text: "References",
                left: 307,
                right: 348,
                top: references_top,
                height: body,
            },
        ];
        lines.extend(gh1760_reference_list_lines());

        let mut words = Vec::new();
        for line in &lines {
            gh1760_push_line(&mut words, line, jitter);
        }
        words
    }

    /// The 6.4 pt reference list below the band, at its own 7.97 pt pitch.
    fn gh1760_reference_list_lines() -> Vec<Gh1760Line> {
        let texts = [
            "[1] Alexandersen S, Zhang Z, Donaldson A, Garland AJ. The pathogenesis of",
            "foot-and-mouth disease in cattle. J Comp Pathol 2003;129:1-36.",
            "[2] Knight-Jones TJD, Rushton J. The economic impacts of foot-and-mouth",
            "disease. Prev Vet Med 2013;112:161-73. PMID: 23916813",
            "[3] Paton DJ, Sumption KJ, Charleston B. Options for the control of",
            "foot-and-mouth disease. Philos Trans R Soc 2009;364:2657-67.",
            "[4] Doel TR. FMD vaccines. Virus Res 2003;91:81-99. PMID: 12527439",
            "[5] Parida S. Vaccination against foot-and-mouth disease. Expert Rev",
        ];
        texts
            .iter()
            .enumerate()
            .map(|(index, text)| Gh1760Line {
                text,
                left: 310,
                right: 535,
                top: 123 + (index as u32 * 8),
                height: GH1760_REFERENCE_HEIGHT,
            })
            .collect()
    }

    /// Run the page-level heuristic path the way `extract_tables_heuristic` does:
    /// cluster the page's words into regions, then reconstruct each region.
    fn gh1760_page_tables(words: &[crate::pdf::table_reconstruct::HocrWord]) -> Vec<Table> {
        cluster_words_into_vertical_regions(words)
            .iter()
            .flat_map(|region| reconstruct_region_tables(region, GH1760_PAGE_HEIGHT, 1, false, 0))
            .collect()
    }

    fn gh1760_table_texts(tables: &[Table]) -> Vec<String> {
        tables.iter().map(|table| table.markdown.clone()).collect()
    }

    fn gh1760_carries_band_text(table: &Table) -> bool {
        table
            .cells
            .iter()
            .flatten()
            .any(|cell| cell.contains("References") || cell.contains("schedules."))
    }

    /// Just the three rows the defect cuts out, as their own region — the input the
    /// region reconstructor sees once clustering has already isolated them.
    fn gh1760_band_region() -> Vec<crate::pdf::table_reconstruct::HocrWord> {
        let lines = [
            Gh1760Line {
                text: "more potent vaccines and/or careful attention to revaccination",
                left: 38,
                right: 289,
                top: 83,
                height: GH1760_BODY_HEIGHT,
            },
            Gh1760Line {
                text: "org/10.1016/j.vaccine.2024.126325.",
                left: 307,
                right: 438,
                top: 83,
                height: GH1760_BODY_HEIGHT,
            },
            Gh1760Line {
                text: "schedules.",
                left: 38,
                right: 74,
                top: 93,
                height: GH1760_BODY_HEIGHT,
            },
            Gh1760Line {
                text: "References",
                left: 307,
                right: 348,
                top: 103,
                height: GH1760_BODY_HEIGHT,
            },
        ];
        let mut words = Vec::new();
        for line in &lines {
            gh1760_push_line(&mut words, line, 0);
        }
        words
    }

    #[test]
    fn two_column_gutter_band_is_not_a_table_gh1760() {
        let words = gh1760_two_column_band();
        let tables = gh1760_page_tables(&words);
        assert!(
            !tables.iter().any(gh1760_carries_band_text),
            "the three lines where the left column's paragraph ends beside the right column's \
             heading are two text columns side by side, not a table; got {:?}",
            gh1760_table_texts(&tables)
        );
    }

    #[test]
    fn references_heading_stays_with_the_surrounding_body_region_gh1760() {
        let words = gh1760_two_column_band();
        let regions = cluster_words_into_vertical_regions(&words);
        let region = regions
            .iter()
            .find(|region| region.iter().any(|word| word.text == "References"))
            .expect("some region must carry the `References` heading");
        let texts: Vec<&str> = region.iter().map(|word| word.text.as_str()).collect();
        assert!(
            texts.contains(&"revaccination") && texts.contains(&"Recommendations"),
            "a regular 10.45 pt body pitch must not cut a region where its rounding to whole \
             units happens to land; the region holding `References` must also hold the body \
             line above and the heading below it, got {texts:?}"
        );
    }

    #[test]
    fn gutter_band_with_different_word_boxes_is_not_a_table_gh1760() {
        let words = gh1760_two_column_band_with(103, 1);
        let tables = gh1760_page_tables(&words);
        assert!(
            !tables.iter().any(gh1760_carries_band_text),
            "how each line's words are boxed must not decide whether the band is a table; \
             got {:?}",
            gh1760_table_texts(&tables)
        );
    }

    #[test]
    fn references_one_unit_lower_still_produces_no_table_gh1760() {
        // The reproducer's control page: `References` a point lower, which stock xberg
        // already handles correctly. It must stay correct. ~keep
        let words = gh1760_two_column_band_with(104, 0);
        let tables = gh1760_page_tables(&words);
        assert!(
            !tables.iter().any(gh1760_carries_band_text),
            "got {:?}",
            gh1760_table_texts(&tables)
        );
    }

    #[test]
    fn short_unruled_prose_region_is_refused_gh1760() {
        // Defence in depth for the band itself: handed straight to the region
        // reconstructor, a three-row unruled region is below the row count at which
        // `column_text_flow` and `row_continuation_flow` can fire, so nothing in
        // post-processing can judge it. It must be refused for want of evidence. ~keep
        let tables = reconstruct_region_tables(&gh1760_band_region(), GH1760_PAGE_HEIGHT, 1, false, 0);
        assert!(
            tables.is_empty(),
            "an unruled three-row region with no numeric cells has nothing saying `table`; \
             got {:?}",
            gh1760_table_texts(&tables)
        );
    }

    #[test]
    fn short_numeric_region_is_still_a_table_gh1760() {
        // Positive control for the evidence gate: a genuine three-row borderless
        // numeric table must still be accepted with no ruling lines. ~keep
        let lines = [
            Gh1760Line {
                text: "Region",
                left: 20,
                right: 60,
                top: 100,
                height: 10,
            },
            Gh1760Line {
                text: "Doses",
                left: 200,
                right: 235,
                top: 100,
                height: 10,
            },
            Gh1760Line {
                text: "Cost",
                left: 340,
                right: 370,
                top: 100,
                height: 10,
            },
            Gh1760Line {
                text: "North",
                left: 20,
                right: 55,
                top: 118,
                height: 10,
            },
            Gh1760Line {
                text: "1,240",
                left: 200,
                right: 235,
                top: 118,
                height: 10,
            },
            Gh1760Line {
                text: "8,415",
                left: 340,
                right: 375,
                top: 118,
                height: 10,
            },
            Gh1760Line {
                text: "South",
                left: 20,
                right: 55,
                top: 136,
                height: 10,
            },
            Gh1760Line {
                text: "2,310",
                left: 200,
                right: 235,
                top: 136,
                height: 10,
            },
            Gh1760Line {
                text: "9,702",
                left: 340,
                right: 375,
                top: 136,
                height: 10,
            },
        ];
        let mut region = Vec::new();
        for line in &lines {
            gh1760_push_line(&mut region, line, 0);
        }

        let tables = reconstruct_region_tables(&region, GH1760_PAGE_HEIGHT, 1, false, 0);
        assert_eq!(
            tables.len(),
            1,
            "a three-row grid of numeric cells is positive evidence of a table and must \
             survive the short-region evidence gate; got {:?}",
            gh1760_table_texts(&tables)
        );
    }

    #[test]
    fn gutter_band_on_a_ruled_page_is_still_refused_gh1760() {
        // #1399's ruling-line signal is counted per page, not per region, so a paper
        // whose real table is ruled would otherwise admit every short band on the same
        // page. The evidence requirement is therefore tested ahead of it. ~keep
        let tables = reconstruct_region_tables(&gh1760_band_region(), GH1760_PAGE_HEIGHT, 1, false, 6);
        assert!(
            tables.is_empty(),
            "rules drawn for another table on the page are not evidence about this band; \
             got {:?}",
            gh1760_table_texts(&tables)
        );
    }

    /// A single-column bullet list must not be split into a short, table-shaped
    /// region because one rounded row gap measures 24 units instead of 23.
    /// The fixture contains word boxes from nine consecutive lines of the
    /// public PDF in #1772, not the full document.
    #[test]
    fn rounded_row_gap_does_not_turn_bullets_into_a_table() {
        let words: Vec<crate::pdf::table_reconstruct::HocrWord> =
            include_str!("../../../tests/fixtures/pdf/bullet_list_words.tsv")
                .lines()
                .map(|line| {
                    let mut fields = line.splitn(5, '\t');
                    let top = fields.next().unwrap().parse().unwrap();
                    let left = fields.next().unwrap().parse().unwrap();
                    let width = fields.next().unwrap().parse().unwrap();
                    let height = fields.next().unwrap().parse().unwrap();
                    let text = fields.next().unwrap().to_string();
                    crate::pdf::table_reconstruct::HocrWord {
                        text,
                        left,
                        top,
                        width,
                        height,
                        confidence: 95.0,
                    }
                })
                .collect();

        let regions = cluster_words_into_vertical_regions(&words);
        let tables: Vec<_> = regions
            .iter()
            .flat_map(|region| reconstruct_region_tables(region, 792.0, 2, false, 0))
            .collect();
        assert!(
            tables.is_empty(),
            "bullet-list prose was extracted as a table: {tables:?}"
        );
        assert_eq!(regions.len(), 1, "the adjacent bullet row was split from the prose");
    }

    // --- xberg-io/xberg#1769: table beside prose in a two-column page ---
    //
    // The issue's page 5 has a 6-column borderless table (8.55 pt row pitch) in the
    // left column, and prose (10.45 pt line pitch) in the right column, with a page
    // gutter of about 32pt between them (table right edge ~275, prose left edge
    // ~307). Neither pitch opens a y-gap wide enough to separate them, so the whole
    // page clusters into one region and the table's bounding box swallows the
    // neighbouring column's text. ~keep

    /// Left edges of the reproducer's 6 table columns. ~keep
    const GH1769_TABLE_COLUMNS: [u32; 6] = [44, 87, 129, 171, 211, 252];
    const GH1769_TABLE_COLUMN_WIDTH: u32 = 22;
    const GH1769_TABLE_ROW_PITCH: f32 = 8.55;
    const GH1769_TABLE_ROWS: usize = 18;
    const GH1769_TABLE_WORD_HEIGHT: u32 = 6;
    const GH1769_TABLE_TOP0: u32 = 40;
    /// Table's right edge: last column left (252) + word width (22) = 274. ~keep
    const GH1769_TABLE_RIGHT_EDGE: u32 = 274;
    /// Prose column's left edge, ~32pt beyond the table's right edge. ~keep
    const GH1769_PROSE_LEFT: u32 = 307;
    const GH1769_PROSE_RIGHT: u32 = 558;
    const GH1769_PROSE_HEIGHT: u32 = 8;
    const GH1769_PROSE_PITCH: f32 = 10.45;
    const GH1769_PROSE_TOP0: u32 = 40;

    fn gh1769_table_words() -> Vec<crate::pdf::table_reconstruct::HocrWord> {
        let mut words = Vec::new();
        for row in 0..GH1769_TABLE_ROWS {
            let top = GH1769_TABLE_TOP0 + (row as f32 * GH1769_TABLE_ROW_PITCH).round() as u32;
            for (column_index, &left) in GH1769_TABLE_COLUMNS.iter().enumerate() {
                words.push(crate::pdf::table_reconstruct::HocrWord {
                    text: if column_index == 0 {
                        format!("Marker {row}")
                    } else {
                        format!("{column_index}.{row}")
                    },
                    left,
                    top,
                    width: GH1769_TABLE_COLUMN_WIDTH,
                    height: GH1769_TABLE_WORD_HEIGHT,
                    confidence: 95.0,
                });
            }
        }
        words
    }

    /// Prose lines mixing an uppercase-start heading fragment with lowercase
    /// continuation lines, the way the reproducer's right column reads. ~keep
    fn gh1769_prose_lines() -> Vec<Gh1760Line> {
        let texts: [&str; 14] = [
            "3.4. Anti-dsDNA antibody groups and clinical",
            "manifestations were compared across the cohort",
            "using standard statistical methods for categorical",
            "and continuous variables where appropriate for",
            "the distribution observed in each subgroup studied",
            "here, with corrections applied for multiple",
            "comparisons across the fourteen measured endpoints",
            "and the three predefined severity strata used",
            "throughout the analysis presented in this section",
            "of the manuscript describing patient outcomes",
            "over the twelve month follow-up period after",
            "enrollment into the prospective observational",
            "cohort study conducted at the tertiary referral",
            "center between 2018 and 2021 inclusive",
        ];
        texts
            .iter()
            .enumerate()
            .map(|(index, text)| Gh1760Line {
                text,
                left: GH1769_PROSE_LEFT,
                right: GH1769_PROSE_RIGHT,
                top: GH1769_PROSE_TOP0 + (index as f32 * GH1769_PROSE_PITCH).round() as u32,
                height: GH1769_PROSE_HEIGHT,
            })
            .collect()
    }

    fn gh1769_prose_words() -> Vec<crate::pdf::table_reconstruct::HocrWord> {
        let mut words = Vec::new();
        for line in &gh1769_prose_lines() {
            gh1760_push_line(&mut words, line, 0);
        }
        words
    }

    #[test]
    fn table_and_neighbouring_column_are_separate_regions_gh1769() {
        let mut words = gh1769_table_words();
        words.extend(gh1769_prose_words());
        let regions = cluster_words_into_vertical_regions(&words);

        assert!(
            regions
                .iter()
                .any(|region| region.iter().any(|w| w.left <= GH1769_TABLE_RIGHT_EDGE)),
            "no region retained any table word; the corridor split ate the table, got {regions:?}"
        );
        assert!(
            regions
                .iter()
                .any(|region| region.iter().any(|w| w.left >= GH1769_PROSE_LEFT)),
            "no region retained any prose word; the corridor split ate the prose column, got {regions:?}"
        );
        for region in &regions {
            let has_table_word = region.iter().any(|w| w.left <= GH1769_TABLE_RIGHT_EDGE);
            let has_prose_word = region.iter().any(|w| w.left >= GH1769_PROSE_LEFT);
            assert!(
                !(has_table_word && has_prose_word),
                "a region must not straddle the table's right edge ({}) and the prose \
                 column's left edge ({}); got region with lefts {:?}",
                GH1769_TABLE_RIGHT_EDGE,
                GH1769_PROSE_LEFT,
                region.iter().map(|w| w.left).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn page_six_table_is_isolated_from_prose_at_nineteen_unit_corridor_gh1769() {
        const PAGE_SIX_PROSE_LEFT: u32 = GH1769_TABLE_RIGHT_EDGE + 19;

        let mut words = gh1769_table_words();
        for word in &mut words {
            word.height = 8;
        }
        words.push(gh1769_word(
            "Table 2 characteristics across all groups",
            GH1769_TABLE_COLUMNS[0],
            30,
            GH1769_TABLE_RIGHT_EDGE - GH1769_TABLE_COLUMNS[0],
            8,
        ));
        let mut prose = gh1769_prose_words();
        for word in &mut prose {
            word.left -= GH1769_PROSE_LEFT - PAGE_SIX_PROSE_LEFT;
        }
        words.extend(prose);

        let regions = cluster_words_into_vertical_regions(&words);
        let table_region = regions
            .iter()
            .find(|region| {
                region
                    .iter()
                    .all(|word| word.left + word.width <= GH1769_TABLE_RIGHT_EDGE)
            })
            .expect("page six must retain a region containing only Table 2");
        let prose_region = regions
            .iter()
            .find(|region| region.iter().all(|word| word.left >= PAGE_SIX_PROSE_LEFT))
            .expect("page six must retain the neighbouring prose as a separate region");

        let tables = reconstruct_region_tables(table_region, 792.0, 6, false, 0);
        assert_eq!(
            tables.len(),
            1,
            "the isolated page-six table region must reconstruct once"
        );
        assert_eq!(tables[0].cells.len(), GH1769_TABLE_ROWS + 1);
        assert_eq!(tables[0].cells[0].len(), GH1769_TABLE_COLUMNS.len());
        assert!(
            reconstruct_region_tables(prose_region, 792.0, 6, false, 0).is_empty(),
            "the neighbouring prose must remain ordinary page content"
        );
    }

    #[test]
    fn table_region_keeps_its_six_columns_gh1769() {
        let mut words = gh1769_table_words();
        words.extend(gh1769_prose_words());
        let regions = cluster_words_into_vertical_regions(&words);
        let table_region = regions
            .iter()
            .find(|region| region.iter().all(|w| w.left <= GH1769_TABLE_RIGHT_EDGE))
            .expect("one region must hold the table words only");

        let region_left = table_region.iter().map(|w| w.left).min().unwrap_or(0);
        let region_right = table_region.iter().map(|w| w.left + w.width).max().unwrap_or(0);
        let region_width = region_right.saturating_sub(region_left) as f32;
        let col_gap = heuristic_column_gap(table_region, region_width);
        let (_, columns) = crate::table_core::reconstruct_table_with_columns(table_region, col_gap, 0.5);

        assert!(
            (5..=6).contains(&columns.len()),
            "the table's 6 columns must survive the split roughly intact, got {columns:?}"
        );
    }

    #[test]
    fn a_regular_borderless_table_is_not_split_at_its_own_gutters_gh1769() {
        let words = gh1769_table_words();
        let regions = cluster_words_into_vertical_regions(&words);
        assert_eq!(
            regions.len(),
            1,
            "a table's own near-uniform inter-column gutters must not be mistaken for a \
             page corridor; got {regions:?}"
        );
    }

    #[test]
    fn a_two_column_borderless_table_is_not_split_down_its_only_gutter_gh1769() {
        const LABEL_LEFT: u32 = 44;
        const VALUE_LEFT: u32 = 300;
        const ROWS: u32 = 10;
        const ROW_PITCH: u32 = 14;

        let mut words = Vec::new();
        for row in 0..ROWS {
            let top = 40 + row * ROW_PITCH;
            words.push(make_word(&format!("Label {row}"), LABEL_LEFT, top, 80));
            words.push(make_word(&format!("{row}.00"), VALUE_LEFT, top, 40));
        }

        let regions = cluster_words_into_vertical_regions(&words);
        assert_eq!(
            regions.len(),
            1,
            "a genuine two-column label/value table has only one gutter, and each side has \
             only one column track; it must not be split down that gutter, got {regions:?}"
        );
    }

    #[test]
    fn a_label_value_financial_table_is_not_split_at_its_dominant_gutter_gh1769() {
        let mut words = Vec::new();
        for row in 0..10 {
            let top = 40 + row * 14;
            words.push(make_word("Benefit", 44, top, 46));
            words.push(make_word("obligation", 96, top, 70));
            words.push(make_word("2009", 300, top, 32));
            words.push(make_word("2008", 350, top, 32));
            words.push(make_word("1,234", 400, top, 38));
        }

        assert!(
            split_region_at_column_corridor(&words, 10).is_none(),
            "a seam leaving descriptors on one side and values on the other must be rejected"
        );
    }

    #[test]
    fn a_full_width_legend_below_columns_remains_one_region_gh1769() {
        let mut words = gh1769_table_words();
        words.extend(gh1769_prose_words());
        for row in 0..3 {
            let top = 240 + row * 10;
            words.push(make_word("Fig. 3 legend begins across", 38, top, 230));
            words.push(make_word("the full page width", 272, top, 250));
        }

        let regions = cluster_words_into_vertical_regions(&words);
        let legend_words: Vec<_> = regions
            .iter()
            .filter(|region| region.iter().any(|word| word.text.starts_with("Fig. 3")))
            .collect();

        assert_eq!(legend_words.len(), 1, "the full-width legend must stay in one region");
        assert_eq!(
            legend_words[0].len(),
            6,
            "the full-width legend must retain both halves of every line"
        );
    }

    /// Word with an explicit height, for corridor unit tests that need a specific
    /// median word height rather than `make_word`'s fixed 10. ~keep
    fn gh1769_word(
        text: &str,
        left: u32,
        top: u32,
        width: u32,
        height: u32,
    ) -> crate::pdf::table_reconstruct::HocrWord {
        crate::pdf::table_reconstruct::HocrWord {
            text: text.to_string(),
            left,
            top,
            width,
            height,
            confidence: 95.0,
        }
    }

    #[test]
    fn a_dense_multiword_ledger_is_not_split_at_its_widest_gutter() {
        let mut words = vec![
            make_word("Account", 20, 40, 45),
            make_word("Amount", 210, 40, 40),
            make_word("Note", 310, 40, 35),
        ];
        for row in 1..=30 {
            let top = 40 + row * 12;
            words.push(make_word("Account", 20, top, 45));
            words.push(make_word(&format!("{row:04}"), 70, top, 30));
            words.push(make_word(&format!("${}.00", row * 137), 210, top, 40));
            words.push(make_word("ref", 310, top, 25));
            words.push(make_word(&row.to_string(), 340, top, 20));
        }

        assert!(
            split_region_at_column_corridor(&words, 10).is_none(),
            "synchronized ledger rows must not be mistaken for independent page columns"
        );
    }

    /// A block of two overlapping x-tracks (so it merges into a single horizontal
    /// interval with no corridor of its own) repeated over 3 rows: 6 words, 2
    /// distinct tracks, satisfying `SIDE_BY_SIDE_MIN_WORDS_PER_SIDE` and
    /// `has_independent_column_tracks` on its own. ~keep
    fn gh1769_corridor_block(start_left: u32, height: u32) -> Vec<crate::pdf::table_reconstruct::HocrWord> {
        let mut words = Vec::new();
        for row in 0..3 {
            let top = row * 20;
            words.push(gh1769_word("label", start_left, top, 15, height));
            words.push(gh1769_word(&format!("{row}.0"), start_left + 10, top, 15, height));
        }
        words
    }

    /// A block's merged horizontal interval spans `[start_left, start_left + 25)`. ~keep
    const GH1769_CORRIDOR_BLOCK_WIDTH: u32 = 25;

    #[test]
    fn corridor_one_unit_under_the_floor_is_not_split_gh1769() {
        let height = 10;
        let minimum = (height as f32 * COLUMN_BLOCK_MIN_CORRIDOR_HEIGHTS).ceil() as u32;
        let gap = minimum - 1;

        let mut region = gh1769_corridor_block(0, height);
        region.extend(gh1769_corridor_block(GH1769_CORRIDOR_BLOCK_WIDTH + gap, height));

        assert!(
            split_region_at_column_corridor(&region, height).is_none(),
            "a corridor one unit under the height-relative floor must not split the region"
        );
    }

    #[test]
    fn corridor_at_the_floor_is_split_gh1769() {
        let height = 10;
        let minimum = (height as f32 * COLUMN_BLOCK_MIN_CORRIDOR_HEIGHTS).ceil() as u32;

        let mut region = gh1769_corridor_block(0, height);
        region.extend(gh1769_corridor_block(GH1769_CORRIDOR_BLOCK_WIDTH + minimum, height));

        assert!(
            split_region_at_column_corridor(&region, height).is_some(),
            "a corridor exactly at the height-relative floor must split the region"
        );
    }

    #[test]
    fn nineteen_unit_corridor_with_eight_unit_words_is_split_gh1769() {
        let height = 8;
        let gap = 19;

        let mut region = gh1769_corridor_block(0, height);
        region.extend(gh1769_corridor_block(GH1769_CORRIDOR_BLOCK_WIDTH + gap, height));

        assert!(
            split_region_at_column_corridor(&region, height).is_some(),
            "the page-six corridor must split the table from the neighbouring prose column"
        );
    }

    #[test]
    fn two_equal_width_corridors_are_not_split_gh1769() {
        let height = 10;
        let gap = 60;

        let mut region = gh1769_corridor_block(0, height);
        region.extend(gh1769_corridor_block(GH1769_CORRIDOR_BLOCK_WIDTH + gap, height));
        let third_start = GH1769_CORRIDOR_BLOCK_WIDTH + gap + GH1769_CORRIDOR_BLOCK_WIDTH + gap;
        region.extend(gh1769_corridor_block(third_start, height));

        assert!(
            split_region_at_column_corridor(&region, height).is_none(),
            "two corridors of equal width have no dominant one and must not be split"
        );
    }

    #[test]
    fn a_corridor_one_and_a_half_times_the_other_is_split_gh1769() {
        let height = 10;
        let wide_gap = 60;
        let narrow_gap = 40;

        let mut region = gh1769_corridor_block(0, height);
        let second_start = GH1769_CORRIDOR_BLOCK_WIDTH + wide_gap;
        region.extend(gh1769_corridor_block(second_start, height));
        let third_start = second_start + GH1769_CORRIDOR_BLOCK_WIDTH + narrow_gap;
        region.extend(gh1769_corridor_block(third_start, height));

        assert!(
            split_region_at_column_corridor(&region, height).is_some(),
            "a corridor 1.5x wider than the next-widest is dominant and must split the region"
        );
    }
}

#[cfg(test)]
mod ruled_tier_tests;
