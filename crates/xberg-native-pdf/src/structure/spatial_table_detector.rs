//! Spatial table detection from PDF text layout.
//!
//! Implements table detection according to ISO 32000-1:2008 Section 5.2 (Coordinate Systems).
//! Uses X and Y coordinate clustering to identify table structure in PDFs that lack explicit
//! table markup in the structure tree.

use crate::layout::text_block::TextSpan;
use crate::structure::table_extractor::{Table, TableCell, TableRow, span_text_for_cell};
use std::collections::HashMap;

/// Disjoint-set (union-find) with path compression.
struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, i: usize) -> usize {
        let mut curr = i;
        while self.parent[curr] != curr {
            self.parent[curr] = self.parent[self.parent[curr]];
            curr = self.parent[curr];
        }
        curr
    }

    fn union(&mut self, i: usize, j: usize) {
        let ri = self.find(i);
        let rj = self.find(j);
        if ri != rj {
            self.parent[ri] = rj;
        }
    }

    fn groups(&mut self) -> HashMap<usize, Vec<usize>> {
        let mut result: HashMap<usize, Vec<usize>> = HashMap::new();
        for i in 0..self.parent.len() {
            let root = self.find(i);
            result.entry(root).or_default().push(i);
        }
        result
    }
}

/// Strategy for detecting table boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum TableStrategy {
    /// Use only vector lines to define boundaries.
    #[serde(rename = "lines")]
    Lines,
    /// Use only text alignment to define boundaries.
    #[serde(rename = "text")]
    Text,
    /// Use both text and lines (hybrid approach).
    #[default]
    #[serde(rename = "both")]
    Both,
}

/// Configuration for spatial table detection.
#[derive(Debug, Clone, PartialEq)]
pub struct TableDetectionConfig {
    /// Whether table detection is enabled.
    pub enabled: bool,
    /// Strategy for horizontal boundary detection.
    pub horizontal_strategy: TableStrategy,
    /// Strategy for vertical boundary detection.
    pub vertical_strategy: TableStrategy,
    /// X-coordinate tolerance for column grouping.
    pub column_tolerance: f32,
    /// Y-coordinate tolerance for row grouping.
    pub row_tolerance: f32,
    /// Minimum number of cells required for a valid table.
    pub min_table_cells: usize,
    /// Minimum number of columns required for a valid table.
    pub min_table_columns: usize,
    /// Ratio of regular rows required for a valid table structure.
    pub regular_row_ratio: f32,
    /// Maximum number of columns allowed before rejecting as false positive.
    pub max_table_columns: usize,
    /// Merge threshold for post-clustering column merge pass.
    /// Adjacent columns whose centers are within this distance are merged.
    pub column_merge_threshold: f32,
    /// Minimum gap between Y-range groups of vertical lines to trigger a cluster split.
    /// Default: 20.0. Use smaller values (e.g. 4.0) for strict mode, larger (e.g. 40.0)
    /// for relaxed mode where V-lines at mixed Y-ranges should stay together.
    pub v_split_gap: f32,
    /// Enable text-only spatial detection as a fallback when no ruling lines are found.
    ///
    /// When `true` and the page has no table-relevant paths (no ruling lines or
    /// rectangles), the detector falls through to `detect_tables_from_spans_column_aware`
    /// rather than returning an empty result.  This is the right default for structured
    /// output callers (`to_markdown`, `to_html`) that explicitly want tabular layout
    /// and is also relied on by the public `extract_tables` API for line-less PDFs.
    /// Set to `false` from callers that want the conservative
    /// "no ruling lines → no tables" behaviour (e.g. plain-text extraction paths
    /// that explicitly opt out — see `extract_page_tables`).
    ///
    /// False-positive prose / TOC / underline tables that this default would
    /// previously have surfaced are filtered post-detection by the
    /// `looks_like_prose_table` shape gate and a ≥ 3-row evidence requirement
    /// on text-only and h-rule paths.
    ///
    /// Default: `true`.
    pub text_fallback: bool,
}

impl Default for TableDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            horizontal_strategy: TableStrategy::Both,
            vertical_strategy: TableStrategy::Both,
            column_tolerance: 15.0,
            row_tolerance: 2.8,
            min_table_cells: 4,
            min_table_columns: 2,
            regular_row_ratio: 0.3,
            max_table_columns: 15,
            column_merge_threshold: 25.0,
            v_split_gap: 20.0,
            text_fallback: true,
        }
    }
}

impl TableDetectionConfig {
    /// Create a strict table detection configuration.
    pub fn strict() -> Self {
        Self {
            enabled: true,
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            column_tolerance: 2.0,
            row_tolerance: 1.0,
            min_table_cells: 6,
            min_table_columns: 3,
            regular_row_ratio: 0.8,
            max_table_columns: 12,
            column_merge_threshold: 10.0,
            v_split_gap: 4.0,
            text_fallback: true,
        }
    }

    /// Create a relaxed table detection configuration.
    pub fn relaxed() -> Self {
        Self {
            enabled: true,
            horizontal_strategy: TableStrategy::Text,
            vertical_strategy: TableStrategy::Text,
            column_tolerance: 10.0,
            row_tolerance: 5.0,
            min_table_cells: 4,
            min_table_columns: 2,
            regular_row_ratio: 0.3,
            max_table_columns: 20,
            column_merge_threshold: 30.0,
            v_split_gap: 40.0,
            text_fallback: true,
        }
    }
}

/// Validate that an extracted table is not a false positive.
///
/// Rejects:
/// - Tables with too many empty cells (> 60%).
/// - 2-column tables that contain a **continuation-row signature**: any
///   row whose left-hand cell is empty while the right-hand cell is
///   non-empty. Product data sheets draw faint cell backgrounds behind
///   label/value rows, which the spatial detector can cluster into tiny
///   2-column tables; when the right-hand value wraps onto a second line,
///   the continuation row leaves an empty left-hand label cell beside
///   the wrapped value text. This exact shape is a reliable false-positive
///   signal. Sparse 2-column tables with *legitimately* missing right-hand
///   values (e.g. "Fax: ", "N/A" rows) are NOT rejected by this rule.
fn is_valid_table(table: &Table) -> bool {
    if table.rows.is_empty() || table.col_count == 0 {
        return false;
    }

    let total_cells = table.rows.len() * table.col_count;
    let empty_cells = table
        .rows
        .iter()
        .flat_map(|r| &r.cells)
        .filter(|c| c.text.trim().is_empty())
        .count();
    let empty_ratio = empty_cells as f32 / total_cells.max(1) as f32;

    if empty_ratio > 0.6 {
        return false;
    }

    if table.col_count == 2 {
        let has_continuation_row = table
            .rows
            .iter()
            .any(|r| r.cells.len() == 2 && r.cells[0].text.trim().is_empty() && !r.cells[1].text.trim().is_empty());
        if has_continuation_row {
            return false;
        }
    }

    true
}

/// Additional gate applied to SPATIAL-only table detection (no explicit
/// lines/rulings): reject "word-per-cell" false positives where a
/// paragraph's visual gaps accidentally align into columns.
///
/// Signature: >=5 columns AND >70% of non-empty cells contain only a
/// single word. Real data tables have multi-word labels, numeric values,
/// or dense content; a paragraph mis-read as a table reads as a sentence
/// when the cells are concatenated.
///
/// This gate is NOT applied when rulings/lines define the table — in
/// that case the author explicitly marked the structure and we trust it
/// even if cells are single-character (census forms, sparse grids).
fn passes_spatial_quality_gate(table: &Table) -> bool {
    if table.col_count < 5 {
        return true;
    }
    let non_empty: Vec<&str> = table
        .rows
        .iter()
        .flat_map(|r| &r.cells)
        .map(|c| c.text.trim())
        .filter(|t| !t.is_empty())
        .collect();
    if non_empty.is_empty() {
        return true;
    }
    // A genuine numeric data table (financial / metrics slides) is legitimately
    // almost all single tokens — every cell is a *number* — so the generic
    // single-word prose gate below would wrongly reject it and flatten it into a
    // bold label plus run-on numbers. Bypass the gate
    // ONLY when the table is clearly numeric-DOMINATED (≥50% of non-empty cells
    // are data values). This is deliberately strict: number-heavy prose (an
    // academic page with inline citations/equations whose words happen to align
    // into columns) stays below 50% numeric and is still held to the prose gate,
    // so the bypass does not manufacture false tables. ~keep
    let data_values = non_empty.iter().filter(|t| is_data_value(t)).count();
    if data_values * 2 >= non_empty.len() {
        return true;
    }
    let single_word_count = non_empty.iter().filter(|t| t.split_whitespace().count() <= 1).count();
    let ratio = single_word_count as f32 / non_empty.len() as f32;
    ratio <= 0.7
}

/// A numeric / data value token: digits plus the usual numeric punctuation
/// (decimal point, thousands comma, percent, sign, currency). Requires at least
/// one digit so a bare `+` or `$` is not treated as data. Used so numeric-table
/// cells do not read as prose fragments in the spatial quality gate.
fn is_data_value(t: &str) -> bool {
    !t.is_empty()
        && t.chars().any(|c| c.is_ascii_digit())
        && t.chars().all(|c| {
            c.is_ascii_digit()
                || matches!(
                    c,
                    '.' | ',' | '%' | '+' | '-' | '\u{2212}' | '$' | '\u{20AC}' | '\u{00A3}'
                )
        })
}

/// Reject a spatial (no-rulings) "table" whose rows are wrapped paragraph
/// lines — a flowing prose page (heading + body paragraph + footer) whose
/// inter-word gaps coincidentally aligned into columns.
///
/// Signature: at least one row, when its non-empty cells are concatenated
/// left-to-right, crosses a SENTENCE boundary mid-row — an alphanumeric
/// character, a sentence terminator, a space, then a new word (e.g. "...to
/// 23,500. Stockout rate..."). Real data-table rows hold values/labels, not
/// running sentences that span a terminator into the next clause, so this
/// almost never fires on genuine tables. Only applied to spatial tables (the
/// caller is the no-rulings path); ruled tables are author-marked and
/// trusted.
///
/// Case is checked via the Unicode general category (`char::is_uppercase` /
/// `is_lowercase`), not ASCII-only, so cased non-Latin scripts (Greek,
/// Cyrillic, Armenian, …) get the same "new sentence starts with a capital"
/// signal Latin does. Scripts with no case distinction at all (Bengali,
/// Devanagari, …) can't use that signal, so their sentence-final danda
/// (`।`, `॥`) is instead treated as a terminator in its own right: a danda
/// followed by a space and another letter mid-row is itself the
/// discriminator, since a genuine data cell doesn't embed a sentence stop
/// followed by more prose in the same row.
fn looks_like_prose_paragraph(table: &Table) -> bool {
    const CASELESS_TERMINATORS: [char; 2] = ['।', '॥'];

    for row in &table.rows {
        let joined = row
            .cells
            .iter()
            .map(|c| c.text.trim())
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let chars: Vec<char> = joined.chars().collect();
        for i in 0..chars.len() {
            // Cased terminator: the exact prior-release strict signature — a
            // lowercase/digit, then `.`/`!`/`?`, then space + capital +
            // lowercase (a genuine new sentence). Kept ASCII with immediate
            // neighbours on purpose: broadening it (any alphanumeric before,
            // skip-spaces, Unicode case) over-rejected genuine data tables
            // whose cells contain abbreviations or capitalised codes ("Fig.
            // 3", "Dr. Smith", "A. Test") as prose. ~keep
            if matches!(chars[i], '.' | '!' | '?')
                && i >= 1
                && (chars[i - 1].is_ascii_lowercase() || chars[i - 1].is_ascii_digit())
                && i + 3 < chars.len()
                && chars[i + 1] == ' '
                && chars[i + 2].is_ascii_uppercase()
                && chars[i + 3].is_ascii_lowercase()
            {
                return true;
            }
            if CASELESS_TERMINATORS.contains(&chars[i]) {
                let Some(&prev) = chars[..i].iter().rev().find(|c| **c != ' ') else {
                    continue;
                };
                if !prev.is_alphanumeric() {
                    continue;
                }
                let mut after = chars[i + 1..].iter().copied().skip_while(|c| *c == ' ');
                if after.next().is_some_and(|c| c.is_alphabetic()) {
                    return true;
                }
            }
        }
    }

    // Punctuation-independent fallback: a wrapped-prose row whose column
    // break fell mid-clause carries no sentence terminator at all (a title
    // caption, a clause with no `.`/`!`/`?`), so the checks above miss it.
    // Real data-cell values are atomic (numbers, codes, capitalised labels)
    // and essentially never start with a lowercase letter; running-sentence
    // fragments frequently do ("text with", "a smaller", "on"). This mirrors
    // `looks_like_prose_table`'s per-cell signal in document.rs, but applies
    // PER ROW with no minimum cell count — that function's ≥10-cell floor
    // (needed there to avoid over-rejecting small genuine tables under a
    // table-wide ratio) is exactly what lets a small (2–3 row) fabricated
    // table slip through untested. A 2-cell floor plus a majority-of-row
    // requirement keeps ordinary short data rows (a units column, a coded
    // abbreviation) safe: those cells are numeric/capitalised, not lowercase
    // clause fragments, so they rarely cross the 50% bar even at n=2. ~keep
    for row in &table.rows {
        let cells: Vec<&str> = row
            .cells
            .iter()
            .map(|c| c.text.trim())
            .filter(|t| !t.is_empty())
            .collect();
        if cells.len() < 2 {
            continue;
        }
        let lower_starts = cells
            .iter()
            .filter(|c| c.chars().next().is_some_and(char::is_lowercase))
            .count();
        if lower_starts as f32 / cells.len() as f32 > 0.5 {
            return true;
        }
    }

    // Vertically-stacked single-character lines: a rotated axis label or
    // figure legend, drawn one glyph per line so its bbox flattens into the
    // page's normal (unrotated) column grid, reads as a cell whose text is
    // mostly 1-character lines joined by `\n` (e.g. "k\nc\no\nc\nE-Box" for a
    // vertically-drawn "clock"-style label). A genuine multi-line data cell
    // wraps at WORD boundaries — its lines hold whole words, not lone
    // letters — so this shape is essentially unique to misread rotated text. ~keep
    for row in &table.rows {
        for cell in &row.cells {
            let lines: Vec<&str> = cell.text.split('\n').map(str::trim).filter(|l| !l.is_empty()).collect();
            if lines.len() < 3 {
                continue;
            }
            let single_char_lines = lines.iter().filter(|l| l.chars().count() == 1).count();
            if single_char_lines as f32 / lines.len() as f32 > 0.5 {
                return true;
            }
        }
    }

    false
}

/// Reject a spatial (no-rulings) "table" that is actually CJK prose split into
/// columns by the wrap geometry of an unspaced script. CJK (Chinese, Japanese,
/// Korean Han/kana) writes without inter-word spaces, so wrapped prose lines
/// align into a grid the column detector mistakes for a table. A genuine CJK
/// data cell is short (a label or a number); a prose fragment is a long run of
/// ideographs/kana. Reject when any cell holds a run of `MIN_CJK_RUN` or more
/// consecutive CJK characters. Ruled / author-marked tables never reach here.
fn looks_like_cjk_prose(table: &Table) -> bool {
    const MIN_CJK_RUN: usize = 12;
    fn is_cjk(c: char) -> bool {
        matches!(
            c as u32,
            0x3040..=0x30FF      // Hiragana + Katakana ~keep
            | 0x3400..=0x4DBF    // CJK Ext A ~keep
            | 0x4E00..=0x9FFF    // CJK Unified ~keep
            | 0xF900..=0xFAFF    // CJK Compatibility ~keep
            | 0xFF66..=0xFF9F    // Halfwidth Katakana ~keep
            | 0xAC00..=0xD7AF    // Hangul Syllables ~keep
        )
    }
    table.rows.iter().flat_map(|r| &r.cells).any(|c| {
        let mut run = 0usize;
        for ch in c.text.chars() {
            if is_cjk(ch) {
                run += 1;
                if run >= MIN_CJK_RUN {
                    return true;
                }
            } else if !ch.is_whitespace() {
                run = 0;
            }
        }
        false
    })
}

/// Reject a spatial (no-rulings) "table" that is actually a bulleted list whose
/// markers and bodies aligned into columns. A genuine data table never carries
/// a cell that is *only* a bullet glyph; an untagged list rendered into two
/// columns (`• | Ship the API.`) does. Catches the structured-document
/// false-positive where a heading + bulleted list + prose are mis-fused into a
/// grid. Ruled / author-marked tables never reach this path.
fn looks_like_bulleted_list(table: &Table) -> bool {
    /// An unambiguous bullet glyph (never a legitimate data value).
    ///
    /// Deliberately a separate list from `xberg`'s `pdf::structure::list_marker::BULLET_GLYPHS`,
    /// which lives in the dependent crate and cannot be imported here. Keep the two in step when
    /// adding a glyph; the arrowheads were added for GH#1790, where an arrow-bullet list was read
    /// as a table. Widening only ever stops a list becoming a table, never the reverse. ~keep
    fn is_bullet_glyph(c: char) -> bool {
        matches!(
            c,
            '\u{2022}'
                | '\u{2023}'
                | '\u{2043}'
                | '\u{2219}'
                | '\u{25AA}'
                | '\u{25CF}'
                | '\u{25E6}'
                | '\u{00B7}'
                | '\u{2024}'
                | '\u{27A2}'
                | '\u{27A4}'
                | '\u{25BA}'
                | '\u{25B6}'
                | '\u{25CB}'
        )
    }
    fn is_list_item(t: &str) -> bool {
        let mut chars = t.chars();
        match chars.next() {
            Some(c) if is_bullet_glyph(c) => true,
            // A lone "*" cell is also a marker (but "*5" footnote-data is not). ~keep
            Some('*') => chars.next().is_none(),
            _ => false,
        }
    }
    table
        .rows
        .iter()
        .flat_map(|r| &r.cells)
        .filter(|c| !c.text.trim().is_empty())
        .any(|c| is_list_item(c.text.trim()))
}

/// Detect page column regions from an X-projection histogram of text spans.
///
/// Builds a histogram of horizontal coverage (2pt buckets), then identifies
/// runs of empty buckets as candidate column gutters.  Only gaps wider than
/// 20pt **and** at least 4% of the total page X-extent are treated as true
/// column boundaries, preventing internal table whitespace from being
/// misidentified as page column gutters.
///
/// Returns a list of `(x_min, x_max)` column regions sorted left-to-right.
fn detect_page_columns(spans: &[TextSpan]) -> Vec<(f32, f32)> {
    if spans.is_empty() {
        return Vec::new();
    }

    // 1. Find page X extent, excluding degenerate outliers.
    //
    // Per PDF 32000-1:2008 §8.3.2.3, user space is an infinite plane and
    // the CTM can produce arbitrarily large coordinates. The visible region
    // is defined by MediaBox/CropBox. Degenerate CTM transforms (e.g.,
    // rotated dvips pages) can produce span coordinates ~1e16 pt wide,
    // which would cause a multi-petabyte histogram allocation.
    //
    // Strategy: compute the median X center, then exclude any span whose
    // center is more than MAX_EXTENT from the median. This fixed safety
    // bound covers all standard page sizes while rejecting pathological
    // outliers; pages wider than 10,000pt fall back to single column. ~keep
    const MAX_EXTENT_FROM_MEDIAN: f32 = 5_000.0;

    let mut x_centers: Vec<f32> = spans.iter().map(|s| s.bbox.x + s.bbox.width * 0.5).collect();
    x_centers.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
    let median_x = x_centers[x_centers.len() / 2];

    let mut page_x_min = f32::MAX;
    let mut page_x_max = f32::MIN;
    for s in spans {
        let center = s.bbox.x + s.bbox.width * 0.5;
        if (center - median_x).abs() > MAX_EXTENT_FROM_MEDIAN {
            continue;
        }
        let left = s.bbox.x;
        let right = s.bbox.x + s.bbox.width;
        if left < page_x_min {
            page_x_min = left;
        }
        if right > page_x_max {
            page_x_max = right;
        }
    }

    if page_x_min >= page_x_max {
        // All spans were outliers or no valid extent ~keep
        return vec![(
            spans.iter().map(|s| s.bbox.x).fold(f32::MAX, f32::min),
            spans.iter().map(|s| s.bbox.x + s.bbox.width).fold(f32::MIN, f32::max),
        )];
    }

    let page_width = page_x_max - page_x_min;

    // Final safety: if width is still unreasonable after outlier filtering,
    // skip column detection entirely. Typical pages are ≤2400pt (A0). ~keep
    if page_width > 10_000.0 {
        tracing::warn!(
            page_width,
            "detect_page_columns: page width still exceeds safe limit after outlier filtering, \
             falling back to single column"
        );
        return vec![(page_x_min, page_x_max)];
    }

    let bucket_size = 2.0_f32;
    let n_buckets = ((page_width) / bucket_size).ceil() as usize + 1;
    let mut histogram = vec![0u32; n_buckets];

    for s in spans {
        let center = s.bbox.x + s.bbox.width * 0.5;
        if (center - median_x).abs() > MAX_EXTENT_FROM_MEDIAN {
            continue;
        }
        let left = s.bbox.x;
        let right = s.bbox.x + s.bbox.width;
        let b_start = ((left - page_x_min) / bucket_size).floor() as usize;
        let b_end = ((right - page_x_min) / bucket_size).ceil() as usize;
        for b in b_start..b_end.min(n_buckets) {
            histogram[b] += 1;
        }
    }

    let min_gap_pt = 20.0_f32;
    let min_gap_buckets = ((min_gap_pt / bucket_size).ceil() as usize).max(1);

    struct Gap {
        start_bucket: usize,
        len_buckets: usize,
    }

    let mut gaps = Vec::new();
    let mut gap_start: Option<usize> = None;

    for (i, &count) in histogram.iter().enumerate() {
        if count == 0 {
            if gap_start.is_none() {
                gap_start = Some(i);
            }
        } else if let Some(gs) = gap_start {
            let gap_len = i - gs;
            if gap_len >= min_gap_buckets {
                gaps.push(Gap {
                    start_bucket: gs,
                    len_buckets: gap_len,
                });
            }
            gap_start = None;
        }
    }

    // 4. For each gap, determine the "immediate region" on each side
    //    (bounded by adjacent gaps or page edges).  A gap qualifies as a
    //    page column gutter only if at least one of its immediate regions
    //    contains a span wider than `min_paragraph_width` (80pt).  This
    //    prevents inter-cell whitespace in tables from being treated as
    //    column gutters. ~keep
    let min_paragraph_width = 80.0_f32;
    let qualifying_indices: Vec<usize> = (0..gaps.len())
        .filter(|&gi| {
            let gap_left_x = page_x_min + gaps[gi].start_bucket as f32 * bucket_size;
            let gap_right_x = page_x_min + (gaps[gi].start_bucket + gaps[gi].len_buckets) as f32 * bucket_size;

            let left_bound = if gi > 0 {
                page_x_min + (gaps[gi - 1].start_bucket + gaps[gi - 1].len_buckets) as f32 * bucket_size
            } else {
                page_x_min
            };
            let right_bound = if gi + 1 < gaps.len() {
                page_x_min + gaps[gi + 1].start_bucket as f32 * bucket_size
            } else {
                page_x_max
            };

            let has_wide_left = spans.iter().any(|s| {
                let center = s.bbox.x + s.bbox.width / 2.0;
                center >= left_bound && center <= gap_left_x && s.bbox.width >= min_paragraph_width
            });

            let has_wide_right = spans.iter().any(|s| {
                let center = s.bbox.x + s.bbox.width / 2.0;
                center >= gap_right_x && center <= right_bound && s.bbox.width >= min_paragraph_width
            });

            has_wide_left || has_wide_right
        })
        .collect();
    let qualifying_gaps: Vec<&Gap> = qualifying_indices.iter().map(|&i| &gaps[i]).collect();

    if qualifying_gaps.is_empty() {
        return vec![(page_x_min, page_x_max)];
    }

    let mut columns = Vec::new();

    let first_occ = match histogram.iter().position(|&c| c > 0) {
        Some(b) => b,
        None => return Vec::new(),
    };
    let mut region_start = first_occ;

    for gap in &qualifying_gaps {
        if gap.start_bucket > region_start {
            let x_min = page_x_min + region_start as f32 * bucket_size;
            let x_max = page_x_min + gap.start_bucket as f32 * bucket_size;
            columns.push((x_min, x_max));
        }
        region_start = gap.start_bucket + gap.len_buckets;
    }

    let last_occ = histogram.iter().rposition(|&c| c > 0).unwrap_or(n_buckets - 1);
    if region_start <= last_occ {
        let x_min = page_x_min + region_start as f32 * bucket_size;
        let x_max = page_x_min + (last_occ + 1) as f32 * bucket_size;
        columns.push((x_min, x_max));
    }

    columns
}

/// Column-aware text-only table detection.
///
/// Detects page columns first (via X-projection histogram), then runs
/// `detect_tables_from_spans()` independently on each column partition.
/// This prevents multi-column academic layouts from being misinterpreted
/// as wide tables spanning the whole page.
pub fn detect_tables_from_spans_column_aware(spans: &[TextSpan], config: &TableDetectionConfig) -> Vec<Table> {
    if !config.enabled || spans.is_empty() {
        return Vec::new();
    }

    let page_cols = detect_page_columns(spans);

    if page_cols.len() <= 1 {
        return detect_tables_from_spans(spans, config);
    }

    let mut all_tables = Vec::new();
    for &(col_x_min, col_x_max) in &page_cols {
        let col_spans: Vec<TextSpan> = spans
            .iter()
            .filter(|s| {
                let span_center = s.bbox.x + s.bbox.width / 2.0;
                span_center >= col_x_min && span_center <= col_x_max
            })
            .cloned()
            .collect();
        if col_spans.is_empty() {
            continue;
        }
        let mut tables = detect_tables_from_spans(&col_spans, config);
        all_tables.append(&mut tables);
    }

    all_tables
}

/// Detect tables from spatial layout of text spans.
pub fn detect_tables_from_spans(spans: &[TextSpan], config: &TableDetectionConfig) -> Vec<Table> {
    if !config.enabled || spans.is_empty() {
        return Vec::new();
    }

    let mut columns = detect_columns(spans, config.column_tolerance, config.column_merge_threshold);

    // Greedy X-center clustering fragments a single logical cell whose
    // words are internally spaced (e.g. an agenda row "Receiving Dock
    // Inspection" laid out with wide inter-word gaps) into one column
    // per word. detect_text_edge_columns instead keeps only X edges that
    // recur across >= 3 distinct rows, so single-row word positions are
    // rejected and the true column grid (Time / Activity / Team) is
    // recovered. Cross-row recurrence is a strictly stronger column
    // signal than one row's word spacing, so prefer the text-edge result
    // whenever it yields a valid, strictly-smaller column set.
    //
    // Safety: for tables with < 3 rows, text-edge can keep no column
    // (every edge appears in < 3 rows) so it returns fewer than
    // min_table_columns and the guard below leaves greedy untouched —
    // small genuine tables are unaffected. ~keep
    // If greedy clustering produced too many columns, try text-edge
    // detection which looks for X positions that recur across multiple rows.
    if columns.len() > config.max_table_columns {
        let te_columns = detect_text_edge_columns(spans, config);
        if te_columns.len() >= config.min_table_columns.max(2) && te_columns.len() < columns.len() {
            columns = te_columns;
        }
    }

    // Borderless numeric lattice (ML / results tables). When the column gap
    // is below `column_merge_threshold`, greedy clustering fuses a dense grid
    // of short numeric cells laid out on a regular ~20pt pitch, so two values
    // share one cell ("0.69 0.76"). The text-edge detector keeps only X edges
    // that recur across >=3 rows, which on a numeric lattice recovers every
    // column. Prefer it when the spans are predominantly numeric and it splits
    // a coarser greedy set into more (still bounded) columns. The numeric-
    // predominance gate keeps prose / label-value tables (e.g. Google-Docs
    // exports) on the greedy path untouched. ~keep
    let numeric_spans = spans.iter().filter(|s| is_numeric_cell(s.text.trim())).count();
    if numeric_spans >= 10 && columns.len() <= config.max_table_columns {
        let te_columns = detect_text_edge_columns(spans, config);
        if te_columns.len() > columns.len()
            && te_columns.len() >= 5
            && te_columns.len() <= config.max_table_columns
            && is_regular_lattice(&te_columns)
        {
            // Adopt the finer lattice ONLY when it still forms a fully valid
            // grid. A sparse split that fails the quality gate would otherwise
            // drop the whole table to prose — worse than the merged-column
            // baseline. Probing here means the refinement can only refine a
            // table that stays valid, never demote one. ~keep
            let probe_rows = detect_rows(spans, config.row_tolerance);
            if probe_rows.len() >= 2 {
                let probe_grid = assign_spans_to_cells(spans, &te_columns, &probe_rows);
                if validate_table_structure_internal(&probe_grid, config) {
                    let probe_table = grid_to_table(&probe_grid, spans, None);
                    if is_valid_table(&probe_table)
                        && passes_spatial_quality_gate(&probe_table)
                        && !looks_like_prose_paragraph(&probe_table)
                    {
                        columns = te_columns;
                    }
                }
            }
        }
    }

    if columns.len() < config.min_table_columns.max(2) || columns.len() > config.max_table_columns {
        return Vec::new();
    }

    let rows = detect_rows(spans, config.row_tolerance);
    if rows.len() < 2 {
        return Vec::new();
    }

    // Baseline gate (CRITICAL): the ORIGINAL (unfiltered) columns must
    // already form a table that passes EVERY emission gate baseline
    // uses — structural validation AND the final is_valid_table /
    // passes_spatial_quality_gate checks. The row-coverage cleanup
    // below only REFINES a table that would have been emitted anyway;
    // it must never CREATE a table from content baseline treated as
    // prose. Without checking the FINAL gates here, dropping phantom
    // columns can flip a borderline case that baseline rejected on the
    // quality gate into a spurious table (observed on annots.pdf link
    // lists and right_to_left_01.pdf Arabic prose in the 70-PDF sweep). ~keep
    let orig_grid = assign_spans_to_cells(spans, &columns, &rows);
    if !validate_table_structure_internal(&orig_grid, config) {
        return Vec::new();
    }
    let orig_table = grid_to_table(&orig_grid, spans, None);
    if !is_valid_table(&orig_table)
        || !passes_spatial_quality_gate(&orig_table)
        || looks_like_prose_paragraph(&orig_table)
        || looks_like_bulleted_list(&orig_table)
        || looks_like_cjk_prose(&orig_table)
    {
        return Vec::new();
    }

    // Drop "phantom" columns created by a single cell whose
    // words are spaced apart (e.g. an agenda "Receiving Dock Inspection"
    // laid out with wide gaps → one greedy column per word). A genuine
    // table column carries content in MOST rows; a per-word phantom
    // appears in only one or two. Keep only columns whose spans occupy
    // at least 60% of rows (min 2). Phantom-column spans are then
    // re-assigned to the nearest surviving column by assign_spans_to_cells,
    // re-joining the words into their true cell. Skipped for small
    // tables (< 3 rows) where every column legitimately spans all rows. ~keep
    if rows.len() >= 3 {
        columns = filter_columns_by_row_coverage(&columns, &rows, spans);
        if columns.len() < config.min_table_columns.max(2) {
            return Vec::new();
        }
    }

    let grid = assign_spans_to_cells(spans, &columns, &rows);
    if !validate_table_structure_internal(&grid, config) {
        return Vec::new();
    }

    let table = grid_to_table(&grid, spans, None);
    if !is_valid_table(&table)
        || !passes_spatial_quality_gate(&table)
        || looks_like_prose_paragraph(&table)
        || looks_like_bulleted_list(&table)
        || looks_like_cjk_prose(&table)
    {
        return Vec::new();
    }

    // Borderless counterpart of the GH#1358 fix applied to
    // `detect_tables_from_intersections`: `detect_columns`/`detect_rows`
    // bucket purely on physical page X/Y, which recovers the right
    // clusters whatever a span's own rotation is, but the caller-visible
    // labelling of which axis is "row" and which is "column" is only
    // correct for an upright table. Applying the reorientation here, on
    // the already-finished `table` (after every gate above has already
    // judged the pre-transpose candidate), reuses
    // `reorient_table_to_rotated_frame` unmodified and keeps every
    // upstream heuristic -- `detect_columns`, `filter_columns_by_row_coverage`,
    // `detect_header_row`, and the validity/quality gates just above --
    // operating on unmodified page-space geometry exactly as before this
    // fix; only the final row/column order can change. Gated by
    // `BUCKET_BORDERLESS_TABLE_GRID_IN_ROTATED_FRAME` so it can be A/B'd
    // independently of the ruled-table fix. ~keep
    let table = if BUCKET_BORDERLESS_TABLE_GRID_IN_ROTATED_FRAME {
        reorient_table_to_rotated_frame(table)
    } else {
        table
    };
    vec![table]
}

/// Borderless-pipeline counterpart of `BUCKET_TABLE_GRID_IN_ROTATED_FRAME`:
/// gates the same `reorient_table_to_rotated_frame` post-process for
/// `detect_tables_from_spans` (the text-clustering table path with no
/// ruling lines). Flip to `false` to neutralise the fix for an A/B
/// comparison -- with this `false`, `detect_tables_from_spans` returns its
/// table byte-for-byte as it did before this change. ~keep
const BUCKET_BORDERLESS_TABLE_GRID_IN_ROTATED_FRAME: bool = true;

#[derive(Debug, Clone)]
struct ColumnCluster {
    x_center: f32,
    x_min: f32,
    x_max: f32,
    span_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
struct RowCluster {
    y_center: f32,
    y_min: f32,
    y_max: f32,
    span_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
struct GridStructure {
    columns: Vec<ColumnCluster>,
    rows: Vec<RowCluster>,
    cells: Vec<Vec<Vec<usize>>>,
}

impl GridStructure {
    fn is_row_empty(&self, row_idx: usize) -> bool {
        self.cells[row_idx].iter().all(|cell| cell.is_empty())
    }

    fn is_column_empty(&self, col_idx: usize) -> bool {
        for row in &self.cells {
            if !row[col_idx].is_empty() {
                return false;
            }
        }
        true
    }

    fn trim_empty_columns(&self) -> GridStructure {
        let num_rows = self.cells.len();
        let num_cols = self.columns.len();

        let mut first_col = 0;
        while first_col < num_cols && self.is_column_empty(first_col) {
            first_col += 1;
        }

        let mut last_col = num_cols;
        while last_col > first_col && self.is_column_empty(last_col - 1) {
            last_col -= 1;
        }

        if first_col >= last_col {
            return self.clone();
        }

        let mut active_cols = Vec::new();
        for c in first_col..last_col {
            let col_width = self.columns[c].x_max - self.columns[c].x_min;
            if col_width < 2.0 && self.is_column_empty(c) {
                continue;
            }
            active_cols.push(c);
        }

        if active_cols.is_empty() {
            return self.clone();
        }

        let new_columns: Vec<ColumnCluster> = active_cols.iter().map(|&c| self.columns[c].clone()).collect();

        let mut new_cells = Vec::with_capacity(num_rows);
        for r in 0..num_rows {
            let row_cells = active_cols.iter().map(|&c| self.cells[r][c].clone()).collect();
            new_cells.push(row_cells);
        }

        GridStructure {
            columns: new_columns,
            rows: self.rows.clone(),
            cells: new_cells,
        }
    }
}

#[derive(Debug, Clone)]
struct CellMergeInfo {
    colspan: u32,
    rowspan: u32,
    covered: bool,
}

/// Keep only columns that carry content in a meaningful
/// fraction of rows. A real table column appears in most rows; a
/// "phantom" column produced by spaced words inside a single cell (e.g.
/// "Receiving Dock Inspection" with wide inter-word gaps) appears in
/// only one or two rows. Each column's distinct-row coverage is the
/// number of rows in which at least one of its spans falls.
///
/// Threshold: >= ceil(0.6 * num_rows), floored at 2. Phantom columns
/// (coverage 1) are removed; their spans get re-assigned to the nearest
/// surviving column downstream, rejoining the words into one cell.
fn filter_columns_by_row_coverage(
    columns: &[ColumnCluster],
    rows: &[RowCluster],
    spans: &[TextSpan],
) -> Vec<ColumnCluster> {
    let num_rows = rows.len();
    if num_rows < 3 {
        return columns.to_vec();
    }
    let min_cov = (((num_rows as f32) * 0.6).ceil() as usize).max(2);

    let span_row = |sidx: usize| -> Option<usize> {
        let cy = spans[sidx].bbox.center().y;
        rows.iter().position(|r| cy <= r.y_max && cy >= r.y_min)
    };

    let kept: Vec<ColumnCluster> = columns
        .iter()
        .filter(|col| {
            let mut seen: Vec<usize> = col.span_indices.iter().filter_map(|&s| span_row(s)).collect();
            seen.sort_unstable();
            seen.dedup();
            seen.len() >= min_cov
        })
        .cloned()
        .collect();

    // Safety: never return fewer than 2 columns from here — if the
    // coverage filter would collapse the table, fall back to the
    // original columns (the caller's min-columns guard then decides). ~keep
    if kept.len() >= 2 { kept } else { columns.to_vec() }
}

fn detect_columns(spans: &[TextSpan], column_tolerance: f32, merge_threshold: f32) -> Vec<ColumnCluster> {
    // Sort span indices by X coordinate before clustering for deterministic results. ~keep
    let mut sorted_indices: Vec<usize> = (0..spans.len()).collect();
    sorted_indices.sort_by(|&a, &b| crate::utils::safe_float_cmp(spans[a].bbox.left(), spans[b].bbox.left()));

    let mut columns: Vec<ColumnCluster> = Vec::new();
    for idx in sorted_indices {
        let x = spans[idx].bbox.left();
        let mut found = false;
        for col in &mut columns {
            if (x - col.x_center).abs() < column_tolerance {
                col.span_indices.push(idx);
                col.x_min = col.x_min.min(x);
                col.x_max = col.x_max.max(x);
                // Update running average so the cluster center tracks
                // the actual midpoint. ~keep
                let n = col.span_indices.len() as f32;
                col.x_center = col.x_center * ((n - 1.0) / n) + x / n;
                found = true;
                break;
            }
        }
        if !found {
            columns.push(ColumnCluster {
                x_center: x,
                x_min: x,
                x_max: x,
                span_indices: vec![idx],
            });
        }
    }

    columns.sort_by(|a, b| crate::utils::safe_float_cmp(a.x_center, b.x_center));

    // `merge_threshold` is a fixed absolute-point gap; on its own it breaks
    // down as column count grows and the table's actual pitch shrinks (a
    // dense numeric table with a 12pt column pitch has every adjacent pair
    // fused by even a modest fixed threshold, collapsing distinct columns
    // into one). Scale the *effective* threshold down for narrow-pitch
    // tables by capping it at a fraction of the table's own median
    // inter-column gap — same ratio-based pitch reasoning as
    // `is_regular_lattice`'s on-pitch band (`[0.6, 1.6] * median`): a gap
    // under `0.6 * median` is off-pitch-small relative to this table's own
    // columns and should merge regardless of the fixed threshold, while a
    // sparse table with few, widely-spaced columns keeps the full
    // `merge_threshold` (no median signal to scale from). ~keep
    let effective_merge_threshold = if columns.len() >= 3 {
        let mut gaps: Vec<f32> = columns
            .windows(2)
            .map(|w| w[1].x_center - w[0].x_center)
            .filter(|g| *g > 0.0)
            .collect();
        if gaps.is_empty() {
            merge_threshold
        } else {
            gaps.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
            let median_gap = gaps[gaps.len() / 2];
            merge_threshold.min(median_gap * 0.6)
        }
    } else {
        merge_threshold
    };

    let mut merged: Vec<ColumnCluster> = Vec::new();
    for col in columns {
        let should_merge = merged.last().is_some_and(|prev: &ColumnCluster| {
            (col.x_center - prev.x_center).abs() < effective_merge_threshold || col.x_min <= prev.x_max
        });
        if should_merge {
            let prev = merged.last_mut().unwrap();
            prev.x_min = prev.x_min.min(col.x_min);
            prev.x_max = prev.x_max.max(col.x_max);
            let total = prev.span_indices.len() as f32 + col.span_indices.len() as f32;
            prev.x_center = prev.x_center * (prev.span_indices.len() as f32 / total)
                + col.x_center * (col.span_indices.len() as f32 / total);
            prev.span_indices.extend(col.span_indices);
        } else {
            merged.push(col);
        }
    }

    merged.sort_by(|a, b| crate::utils::safe_float_cmp(a.x_center, b.x_center));
    merged
}

/// Text-edge column detection inspired by pdfplumber/Tabula "Stream" mode.
///
/// Instead of greedily clustering span X-centres, this approach:
/// 1. Collects left-edge and right-edge X positions of every span.
/// 2. Snaps nearby X values into clusters (within `snap_tolerance`).
/// 3. Keeps only X positions that appear in `min_row_count` or more distinct
///    text rows — these are consistent alignment edges.
/// 4. Returns [`ColumnCluster`]s whose centres sit at those surviving edges.
///
/// The resulting columns are fewer and more faithful to the visual grid of
/// forms that have no vector lines.
/// A short numeric cell: optional sign, digits with an optional single decimal
/// point, optional trailing `%`. Accepts `0.69`, `100`, `-1.2`, `52%`; rejects
/// words and identifiers. Used to recognise a borderless data grid.
fn is_numeric_cell(t: &str) -> bool {
    if t.is_empty() || t.len() > 8 {
        return false;
    }
    let t = t.strip_suffix('%').unwrap_or(t);
    let t = t.strip_prefix(['+', '-', '\u{2212}']).unwrap_or(t);
    let mut seen_dot = false;
    let mut seen_digit = false;
    for c in t.chars() {
        match c {
            '0'..='9' => seen_digit = true,
            '.' if !seen_dot => seen_dot = true,
            _ => return false,
        }
    }
    seen_digit
}

/// True when the column centres sit on a near-constant pitch — the signature of
/// a numeric data lattice rather than prose that happened to align. Requires
/// ≥5 columns and tolerates up to two off-pitch gaps (e.g. a wider row-label
/// column at the left edge).
fn is_regular_lattice(cols: &[ColumnCluster]) -> bool {
    if cols.len() < 5 {
        return false;
    }
    let mut centers: Vec<f32> = cols.iter().map(|c| c.x_center).collect();
    centers.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
    let gaps: Vec<f32> = centers.windows(2).map(|w| w[1] - w[0]).collect();
    let mut sorted = gaps.clone();
    sorted.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
    let median = sorted[sorted.len() / 2];
    if median <= 0.0 {
        return false;
    }
    let on_pitch = gaps.iter().filter(|&&g| g >= median * 0.6 && g <= median * 1.6).count();
    on_pitch + 2 >= gaps.len()
}

fn detect_text_edge_columns(spans: &[TextSpan], config: &TableDetectionConfig) -> Vec<ColumnCluster> {
    if spans.is_empty() {
        return Vec::new();
    }

    let snap_tolerance = config.column_tolerance;
    let min_row_count: usize = 3;

    let row_tol = config.row_tolerance;

    let mut row_ids: Vec<usize> = Vec::with_capacity(spans.len());
    let mut row_centres: Vec<f32> = Vec::new();
    for span in spans {
        let y = span.bbox.center().y;
        let mut assigned = None;
        for (rid, rc) in row_centres.iter().enumerate() {
            if (y - rc).abs() < row_tol {
                assigned = Some(rid);
                break;
            }
        }
        match assigned {
            Some(rid) => row_ids.push(rid),
            None => {
                row_ids.push(row_centres.len());
                row_centres.push(y);
            }
        }
    }

    let mut edge_obs: Vec<(f32, usize)> = Vec::with_capacity(spans.len() * 2);
    for (i, span) in spans.iter().enumerate() {
        edge_obs.push((span.bbox.left(), row_ids[i]));
        edge_obs.push((span.bbox.right(), row_ids[i]));
    }
    // Sort by X for deterministic clustering. ~keep
    edge_obs.sort_by(|a, b| crate::utils::safe_float_cmp(a.0, b.0));

    struct XCluster {
        x_center: f32,
        count: usize,
        rows: Vec<usize>,
    }

    let mut x_clusters: Vec<XCluster> = Vec::new();
    for &(x, rid) in &edge_obs {
        let mut found = false;
        for cl in &mut x_clusters {
            if (x - cl.x_center).abs() < snap_tolerance {
                let n = cl.count as f32;
                cl.x_center = cl.x_center * (n / (n + 1.0)) + x / (n + 1.0);
                cl.count += 1;
                cl.rows.push(rid);
                found = true;
                break;
            }
        }
        if !found {
            x_clusters.push(XCluster {
                x_center: x,
                count: 1,
                rows: vec![rid],
            });
        }
    }

    let mut edges: Vec<f32> = Vec::new();
    for cl in &mut x_clusters {
        cl.rows.sort_unstable();
        cl.rows.dedup();
        if cl.rows.len() >= min_row_count {
            edges.push(cl.x_center);
        }
    }
    edges.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));

    // Deduplicate edges that ended up very close after averaging. ~keep
    let mut deduped: Vec<f32> = Vec::new();
    for &e in &edges {
        if deduped.last().is_some_and(|prev| (e - prev).abs() < snap_tolerance) {
            let prev = deduped.last_mut().unwrap();
            *prev = (*prev + e) / 2.0;
        } else {
            deduped.push(e);
        }
    }

    let mut columns: Vec<ColumnCluster> = deduped
        .iter()
        .map(|&x| ColumnCluster {
            x_center: x,
            x_min: x,
            x_max: x,
            span_indices: Vec::new(),
        })
        .collect();

    if columns.is_empty() {
        return columns;
    }

    for (idx, span) in spans.iter().enumerate() {
        let sx = span.bbox.left();
        let best = columns
            .iter()
            .enumerate()
            .min_by_key(|(_, c)| ((sx - c.x_center).abs() * 1000.0) as i32)
            .map(|(i, _)| i)
            .unwrap_or(0);
        columns[best].span_indices.push(idx);
        columns[best].x_min = columns[best].x_min.min(sx);
        columns[best].x_max = columns[best].x_max.max(sx);
    }

    columns.retain(|c| !c.span_indices.is_empty());

    columns.sort_by(|a, b| crate::utils::safe_float_cmp(a.x_center, b.x_center));
    columns
}

fn detect_rows(spans: &[TextSpan], row_tolerance: f32) -> Vec<RowCluster> {
    // Sort span indices by Y coordinate before clustering for deterministic results. ~keep
    let mut sorted_indices: Vec<usize> = (0..spans.len()).collect();
    sorted_indices.sort_by(|&a, &b| crate::utils::safe_float_cmp(spans[a].bbox.center().y, spans[b].bbox.center().y));

    let mut rows: Vec<RowCluster> = Vec::new();
    for idx in sorted_indices {
        let y = spans[idx].bbox.center().y;
        let mut found = false;
        for row in &mut rows {
            if (y - row.y_center).abs() < row_tolerance {
                row.span_indices.push(idx);
                row.y_min = row.y_min.min(y);
                row.y_max = row.y_max.max(y);
                // Update running average (same rationale as detect_columns) ~keep
                let n = row.span_indices.len() as f32;
                row.y_center = row.y_center * ((n - 1.0) / n) + y / n;
                found = true;
                break;
            }
        }
        if !found {
            rows.push(RowCluster {
                y_center: y,
                y_min: y,
                y_max: y,
                span_indices: vec![idx],
            });
        }
    }
    rows.sort_by(|a, b| crate::utils::safe_float_cmp(b.y_center, a.y_center));
    rows
}

fn assign_spans_to_cells(spans: &[TextSpan], columns: &[ColumnCluster], rows: &[RowCluster]) -> GridStructure {
    let num_cols = columns.len();
    let num_rows = rows.len();
    let mut cells: Vec<Vec<Vec<usize>>> = vec![vec![Vec::new(); num_cols]; num_rows];
    for (idx, span) in spans.iter().enumerate() {
        let span_x = span.bbox.center().x;
        let span_y = span.bbox.center().y;
        let col_idx = columns
            .iter()
            .enumerate()
            .min_by_key(|(_, col)| ((span_x - col.x_center).abs() * 1000.0) as i32)
            .map(|(i, _)| i)
            .unwrap_or(0);
        let row_idx = rows
            .iter()
            .enumerate()
            .min_by_key(|(_, row)| ((span_y - row.y_center).abs() * 1000.0) as i32)
            .map(|(i, _)| i)
            .unwrap_or(0);
        cells[row_idx][col_idx].push(idx);
    }
    GridStructure {
        columns: columns.to_vec(),
        rows: rows.to_vec(),
        cells,
    }
}

/// Maximum number of detected columns the split-column detector can
/// analyse. Grids wider than this skip the check; extremely wide
/// candidates are rare and have other defences upstream.
const MAX_MASK_COLUMNS: usize = 128;

/// Minimum share of modal rows a column-component must contain to
/// count as "significant" for split-detection purposes. Chosen to
/// admit the original split-flow shape (the DB10 reproducer's modal
/// rows split evenly across two halves) while avoiding obvious
/// overfitting. Heuristic, not corpus-calibrated.
const MIN_SPLIT_GROUP_ROW_SHARE: f32 = 0.20;

fn validate_table_structure_internal(grid: &GridStructure, config: &TableDetectionConfig) -> bool {
    let num_cols = grid.columns.len();
    let total_cells: usize = grid
        .cells
        .iter()
        .flat_map(|row| row.iter().take(num_cols))
        .map(|cell| if cell.is_empty() { 0 } else { 1 })
        .sum();
    if total_cells < config.min_table_cells {
        return false;
    }
    let cell_counts: Vec<usize> = grid
        .cells
        .iter()
        .map(|row| row.iter().take(num_cols).filter(|cell| !cell.is_empty()).count())
        .collect();
    if cell_counts.is_empty() {
        return false;
    }
    let most_common_count = *cell_counts
        .iter()
        .max_by_key(|&&count| cell_counts.iter().filter(|&&c| c == count).count())
        .unwrap_or(&0);
    if most_common_count == 0 {
        return false;
    }
    let regular_rows = cell_counts.iter().filter(|&&count| count == most_common_count).count();
    if (regular_rows as f32 / cell_counts.len() as f32) < config.regular_row_ratio {
        return false;
    }

    if has_split_modal_column_groups(grid, most_common_count) {
        return false;
    }

    true
}

/// Returns `true` when the modal rows of `grid` partition into two or
/// more disconnected column-co-occurrence components, each backed by a
/// significant share of modal rows. This signature catches "two prose
/// flows mis-clustered as one table" without rejecting hierarchical
/// tables whose modal data rows are sparse but internally connected.
///
/// The check operates only on rows whose populated-cell count equals
/// `most_common_count`. For each such row, the populated columns form
/// a co-occurrence clique. The union of those cliques forms a graph
/// over columns; its connected components are computed via bitmask
/// flood-fill. If two or more components each contain at least two
/// columns and are supported by at least `MIN_SPLIT_GROUP_ROW_SHARE`
/// of the modal rows, the grid is rejected as split-flow.
///
/// Heuristic, not corpus-calibrated.
fn has_split_modal_column_groups(grid: &GridStructure, most_common_count: usize) -> bool {
    let num_cols = grid.columns.len();

    // A meaningful split needs at least 4 columns (two groups of >=2)
    // and at least 2 populated cells per modal row. ~keep
    if !(4..=MAX_MASK_COLUMNS).contains(&num_cols) || most_common_count < 2 {
        return false;
    }

    // Collect column-occupancy masks for the modal rows. Bounded by
    // `num_cols` so `most_common_count` (computed over the same bounded
    // slice upstream) and `populated` here share one column universe;
    // also keeps every `1u128 << idx` shift in range of the u128 mask. ~keep
    let modal_masks: Vec<u128> = grid
        .cells
        .iter()
        .filter_map(|row| {
            let populated = row.iter().take(num_cols).filter(|cell| !cell.is_empty()).count();

            if populated != most_common_count {
                return None;
            }

            let mut mask = 0u128;
            for (idx, cell) in row.iter().take(num_cols).enumerate() {
                if !cell.is_empty() {
                    mask |= 1u128 << idx;
                }
            }

            if mask.count_ones() >= 2 { Some(mask) } else { None }
        })
        .collect();

    if modal_masks.len() < 4 {
        return false;
    }

    // Floor at 2 rows so a single-row outlier with a wide/narrow mask
    // can never be classified as its own "significant" component
    // — when modal_masks.len() == 4 the share alone would round to 1. ~keep
    let min_component_rows = (((modal_masks.len() as f32) * MIN_SPLIT_GROUP_ROW_SHARE).ceil() as usize).max(2);

    let mut adjacency: Vec<u128> = vec![0u128; num_cols];
    let mut active_columns: u128 = 0;

    for &mask in &modal_masks {
        active_columns |= mask;
        let mut bits = mask;
        while bits != 0 {
            let bit = bits & bits.wrapping_neg();
            let col = bit.trailing_zeros() as usize;
            adjacency[col] |= mask;
            bits &= !bit;
        }
    }

    let mut remaining = active_columns;
    let mut significant_components = 0usize;

    while remaining != 0 {
        let seed_bit = remaining & remaining.wrapping_neg();
        let mut component: u128 = 0;
        let mut frontier: u128 = seed_bit;

        while frontier != 0 {
            let bit = frontier & frontier.wrapping_neg();
            frontier &= !bit;

            if component & bit != 0 {
                continue;
            }

            component |= bit;
            let col = bit.trailing_zeros() as usize;
            frontier |= adjacency[col] & !component;
        }

        remaining &= !component;

        let component_cols = component.count_ones() as usize;
        let component_row_support = modal_masks.iter().filter(|&&mask| mask & component != 0).count();

        if component_cols >= 2 && component_row_support >= min_component_rows {
            significant_components += 1;
            if significant_components >= 2 {
                return true;
            }
        }
    }

    false
}

/// Backward compatibility: Indices of spans belonging to a table.
#[derive(Debug, Clone)]
pub struct DetectedTable {
    /// Indices of spans that belong to this table.
    pub span_indices: Vec<usize>,
}

/// Backward compatibility: Table detector wrapper.
pub struct SpatialTableDetector {
    /// Configuration for this detector.
    pub config: TableDetectionConfig,
}

impl SpatialTableDetector {
    /// Create a new detector with config.
    pub fn with_config(config: TableDetectionConfig) -> Self {
        Self { config }
    }
    /// Detect tables (wrapper).
    pub fn detect_tables(&self, spans: &[TextSpan]) -> Vec<DetectedTable> {
        detect_tables_from_spans_column_aware(spans, &self.config)
            .into_iter()
            .flat_map(|_| None)
            .collect()
    }
    /// Detect tables using visual lines and text (hybrid).
    pub fn detect_tables_hybrid(&self, spans: &[TextSpan], lines: &[crate::elements::PathContent]) -> Vec<Table> {
        detect_tables_with_lines(spans, lines, &self.config)
    }
}

fn cluster_values(values: &[f32], tolerance: f32) -> Vec<f32> {
    let mut clusters: Vec<f32> = Vec::new();
    let mut counts: Vec<u32> = Vec::new();
    for &v in values {
        if let Some(idx) = clusters.iter().position(|&c| (v - c).abs() < tolerance) {
            counts[idx] += 1;
            clusters[idx] += (v - clusters[idx]) / counts[idx] as f32;
        } else {
            clusters.push(v);
            counts.push(1);
        }
    }
    clusters
}

struct LineCluster {
    lines: Vec<usize>,
    bbox: crate::geometry::Rect,
}

impl LineCluster {
    fn new(line_idx: usize, bbox: crate::geometry::Rect) -> Self {
        Self {
            lines: vec![line_idx],
            bbox,
        }
    }
    fn add(&mut self, line_idx: usize, bbox: crate::geometry::Rect) {
        self.lines.push(line_idx);
        self.bbox = self.bbox.union(&bbox);
    }
}

fn group_lines_into_clusters(
    lines: &[crate::elements::PathContent],
    config: &TableDetectionConfig,
) -> Vec<LineCluster> {
    if lines.is_empty() {
        return Vec::new();
    }
    // All clustering geometry below works on RENDERED extents: a table rule
    // encoded as a 1 pt segment with a table-height stroke width must
    // cluster with (and span) the rules its drawn bar actually touches, not
    // the ones near its geometric speck. Computed once — pure
    // arithmetic per path. ~keep
    let rendered: Vec<crate::geometry::Rect> = lines.iter().map(|p| p.rendered_bbox()).collect();
    let mut uf = UnionFind::new(lines.len());
    let mut valid_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, path)| path.is_table_primitive())
        .map(|(i, _)| i)
        .collect();

    // Optimization: Sort by X-coordinate to enable sweep-line early exit (O(n log n)) ~keep
    valid_indices.sort_by(|&a, &b| crate::utils::safe_float_cmp(rendered[a].x, rendered[b].x));

    const EXPANSION: f32 = 3.0;
    for i in 0..valid_indices.len() {
        let idx_a = valid_indices[i];
        let bbox_a = &rendered[idx_a];
        let expanded_a = crate::geometry::Rect::new(
            bbox_a.x - EXPANSION,
            bbox_a.y - EXPANSION,
            bbox_a.width + EXPANSION * 2.0,
            bbox_a.height + EXPANSION * 2.0,
        );

        for j in (i + 1)..valid_indices.len() {
            let idx_b = valid_indices[j];
            let bbox_b = &rendered[idx_b];

            // Optimization: If the next path's X-start is beyond our search threshold,
            // no subsequent paths in the sorted list can possibly intersect. ~keep
            if bbox_b.x > expanded_a.x + expanded_a.width {
                break;
            }

            let expanded_b = crate::geometry::Rect::new(
                bbox_b.x - EXPANSION,
                bbox_b.y - EXPANSION,
                bbox_b.width + EXPANSION * 2.0,
                bbox_b.height + EXPANSION * 2.0,
            );

            if expanded_a.intersects(&expanded_b) {
                uf.union(idx_a, idx_b);
            }
        }
    }
    let mut cluster_map: HashMap<usize, LineCluster> = HashMap::new();
    for i in valid_indices {
        let root = uf.find(i);
        let bbox = rendered[i];
        cluster_map
            .entry(root)
            .and_modify(|c| c.add(i, bbox))
            .or_insert_with(|| LineCluster::new(i, bbox));
    }

    // Post-processing: split clusters whose vertical lines occupy distinct Y-ranges.
    // This prevents a small bordered table (e.g. an invoice header) from merging
    // with a large main table that happens to be nearby vertically.
    // Deterministic order: `cluster_map` is a HashMap (per-process-randomized
    // iteration), so sort clusters by their first (smallest) line index — each
    // cluster's `lines` Vec is already ascending — to keep downstream table
    // boundary order stable across runs. ~keep
    let mut raw_clusters: Vec<LineCluster> = cluster_map.into_values().collect();
    raw_clusters.sort_by_key(|c| c.lines.first().copied().unwrap_or(usize::MAX));
    let mut result: Vec<LineCluster> = Vec::with_capacity(raw_clusters.len());
    const LINE_AXIS_TOL: f32 = 2.0;
    let v_split_gap = config.v_split_gap;

    for cluster in raw_clusters {
        let mut v_ranges: Vec<(usize, f32, f32)> = Vec::new();
        for &idx in &cluster.lines {
            let path = &lines[idx];
            if path.is_vertical_line(LINE_AXIS_TOL) && rendered[idx].height.abs() > 5.0 {
                let y_min = rendered[idx].y;
                let y_max = rendered[idx].y + rendered[idx].height;
                let (y_min, y_max) = if y_min <= y_max { (y_min, y_max) } else { (y_max, y_min) };
                v_ranges.push((idx, y_min, y_max));
            }
        }

        if v_ranges.len() < 2 {
            result.push(cluster);
            continue;
        }

        v_ranges.sort_by(|a, b| crate::utils::safe_float_cmp(a.1, b.1));
        let mut bands: Vec<(f32, f32)> = Vec::new();
        let mut band_start = v_ranges[0].1;
        let mut band_end = v_ranges[0].2;
        for &(_, y_min, y_max) in &v_ranges[1..] {
            if y_min > band_end + v_split_gap {
                bands.push((band_start, band_end));
                band_start = y_min;
                band_end = y_max;
            } else {
                band_end = band_end.max(y_max);
            }
        }
        bands.push((band_start, band_end));

        if bands.len() < 2 {
            result.push(cluster);
            continue;
        }

        let mut sub_clusters: Vec<Vec<usize>> = vec![Vec::new(); bands.len()];
        for &idx in &cluster.lines {
            let bbox = &rendered[idx];
            let line_y_mid = bbox.y + bbox.height * 0.5;
            let mut best_band = 0;
            let mut best_dist = f32::MAX;
            for (bi, &(b_min, b_max)) in bands.iter().enumerate() {
                let dist = if line_y_mid >= b_min && line_y_mid <= b_max {
                    0.0
                } else {
                    (line_y_mid - b_min).abs().min((line_y_mid - b_max).abs())
                };
                if dist < best_dist {
                    best_dist = dist;
                    best_band = bi;
                }
            }
            sub_clusters[best_band].push(idx);
        }

        for sub in sub_clusters {
            if sub.is_empty() {
                continue;
            }
            let first_bbox = rendered[sub[0]];
            let mut lc = LineCluster::new(sub[0], first_bbox);
            for &idx in &sub[1..] {
                lc.add(idx, rendered[idx]);
            }
            result.push(lc);
        }
    }

    result
}

/// (C) WS0.3b — detect a header text row sitting just ABOVE the grid's top
/// ruling (a header boxed only by the page-top, or unruled above the grid).
///
/// `row_ys` is sorted descending (`row_ys[0]` = the grid's top edge) and
/// `col_xs` sorted ascending (the detected column boundaries). If a tight band
/// immediately above the top ruling — up to ~1.5x the median grid row height —
/// holds text whose cells align to the EXISTING columns and that both spans
/// at least 2 distinct columns and reaches across at least half the extent, this
/// returns the y of a new top boundary that brackets exactly that header row.
/// Otherwise `None` (table unchanged).
///
/// The distinct-columns + horizontal-span gates keep a centred title or a
/// left-aligned caption above an already-correct table from being mistaken for
/// a header row (their words cluster together instead of spanning the columns),
/// so this never alters a table that already detects correctly.
fn detect_header_row_above(spans: &[TextSpan], row_ys: &[f32], col_xs: &[f32]) -> Option<f32> {
    /// Header band reaches at most this multiple of the median row height above
    /// the grid's top ruling.
    const WINDOW_ROWS: f32 = 1.5;
    /// Header cells must reach across at least this fraction of the column
    /// extent (rejects titles/captions whose words cluster together).
    const MIN_SPAN_FRAC: f32 = 0.5;

    if row_ys.len() < 2 || col_xs.len() < 2 {
        return None;
    }
    let grid_top = row_ys[0];
    let col_lo = *col_xs.first().unwrap();
    let col_hi = *col_xs.last().unwrap();
    let col_extent = col_hi - col_lo;
    if col_extent <= 0.0 {
        return None;
    }

    let mut gaps: Vec<f32> = row_ys.windows(2).map(|w| (w[0] - w[1]).abs()).collect();
    gaps.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
    let median_row_h = gaps[gaps.len() / 2];
    if median_row_h <= 0.0 {
        return None;
    }
    let window_top = grid_top + WINDOW_ROWS * median_row_h;

    let mut cols_hit: Vec<usize> = Vec::new();
    let mut header_max_top = f32::NEG_INFINITY;
    let mut cx_min = f32::INFINITY;
    let mut cx_max = f32::NEG_INFINITY;
    let mut cy_min = f32::INFINITY;
    let mut cy_max = f32::NEG_INFINITY;
    for span in spans {
        let cy = span.bbox.center().y;
        if cy <= grid_top || cy > window_top {
            continue;
        }
        let cx = span.bbox.center().x;
        if cx < col_lo || cx > col_hi {
            continue;
        }
        let Some(ci) = (0..col_xs.len() - 1).find(|&c| cx >= col_xs[c] && cx <= col_xs[c + 1]) else {
            continue;
        };
        if !cols_hit.contains(&ci) {
            cols_hit.push(ci);
        }
        header_max_top = header_max_top.max(span.bbox.y + span.bbox.height);
        cx_min = cx_min.min(cx);
        cx_max = cx_max.max(cx);
        cy_min = cy_min.min(cy);
        cy_max = cy_max.max(cy);
    }

    if cols_hit.len() < 2 {
        return None;
    }
    if (cx_max - cx_min) < MIN_SPAN_FRAC * col_extent {
        return None;
    }
    if (cy_max - cy_min) > median_row_h {
        return None;
    }

    Some(header_max_top + 1.0)
}

fn detect_tables_in_cluster(
    spans: &[TextSpan],
    all_lines: &[crate::elements::PathContent],
    cluster: &LineCluster,
    config: &TableDetectionConfig,
) -> Vec<Table> {
    const MIN_LINE_LENGTH: f32 = 5.0;
    const LINE_AXIS_TOL: f32 = 2.0;
    let mut h_ys: Vec<f32> = Vec::new();
    let mut v_xs: Vec<f32> = Vec::new();
    for &idx in &cluster.lines {
        let path = &all_lines[idx];
        // Rendered extents: a stroke-width-encoded rule's center and length
        // come from the drawn bar, not the geometric speck. ~keep
        let bbox = path.rendered_bbox();
        if path.is_horizontal_line(LINE_AXIS_TOL) && bbox.width > MIN_LINE_LENGTH {
            h_ys.push(bbox.center().y);
        }
        if path.is_vertical_line(LINE_AXIS_TOL) && bbox.height.abs() > MIN_LINE_LENGTH {
            v_xs.push(bbox.center().x);
        }
    }
    let mut row_ys = cluster_values(&h_ys, config.row_tolerance);
    let mut col_xs = cluster_values(&v_xs, config.column_tolerance);
    if row_ys.len() < 2 || col_xs.len() < 2 {
        return Vec::new();
    }
    row_ys.sort_by(|a, b| crate::utils::safe_float_cmp(*b, *a));
    col_xs.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
    // (C) WS0.3b — include a header text row sitting just ABOVE the grid's top
    // ruling (boxed by page-top / unruled above the grid) when its cells align
    // to the already-detected columns. Adds ONE row boundary above the top and
    // widens the span-assignment region to reach it; never adds columns. Returns
    // `None` (unchanged) unless the tight gate in `detect_header_row_above` holds. ~keep
    let mut assign_bbox = cluster.bbox;
    let mut inserted_header_row = false;
    if let Some(header_top) = detect_header_row_above(spans, &row_ys, &col_xs) {
        row_ys.insert(0, header_top);
        inserted_header_row = true;
        let new_height = (header_top - assign_bbox.y).max(assign_bbox.height);
        assign_bbox = crate::geometry::Rect::new(assign_bbox.x, assign_bbox.y, assign_bbox.width, new_height);
    }
    let num_rows = row_ys.len() - 1;
    let num_cols = col_xs.len() - 1;
    if num_cols < config.min_table_columns || num_cols > config.max_table_columns {
        return Vec::new();
    }
    let mut cells: Vec<Vec<Vec<usize>>> = vec![vec![Vec::new(); num_cols]; num_rows];
    let mut assigned_any = false;
    for (orig_idx, span) in spans.iter().enumerate() {
        if !assign_bbox.intersects(&span.bbox) {
            continue;
        }
        let cx = span.bbox.center().x;
        let cy = span.bbox.center().y;
        let row_idx = (0..num_rows).find(|&r| cy <= row_ys[r] && cy >= row_ys[r + 1]);
        let col_idx = (0..num_cols).find(|&c| cx >= col_xs[c] && cx <= col_xs[c + 1]);
        if let (Some(r), Some(c)) = (row_idx, col_idx) {
            cells[r][c].push(orig_idx);
            assigned_any = true;
        }
    }
    if !assigned_any {
        return Vec::new();
    }
    let columns: Vec<ColumnCluster> = (0..num_cols)
        .map(|c| ColumnCluster {
            x_center: (col_xs[c] + col_xs[c + 1]) / 2.0,
            x_min: col_xs[c],
            x_max: col_xs[c + 1],
            span_indices: Vec::new(),
        })
        .collect();
    let all_rows: Vec<RowCluster> = (0..num_rows)
        .map(|r| RowCluster {
            y_center: (row_ys[r] + row_ys[r + 1]) / 2.0,
            y_min: row_ys[r + 1],
            y_max: row_ys[r],
            span_indices: Vec::new(),
        })
        .collect();
    let grid_full = GridStructure {
        columns: columns.clone(),
        rows: all_rows.clone(),
        cells: cells.clone(),
    };
    let mut tables = Vec::new();
    let mut current_start_row = 0;
    while current_start_row < num_rows {
        if grid_full.is_row_empty(current_start_row) {
            current_start_row += 1;
            continue;
        }
        let mut current_end_row = current_start_row;
        while current_end_row < num_rows {
            if grid_full.is_row_empty(current_end_row) {
                break;
            }
            current_end_row += 1;
        }
        if current_end_row > current_start_row {
            let sub_cells = cells[current_start_row..current_end_row].to_vec();
            let sub_rows = all_rows[current_start_row..current_end_row].to_vec();
            let mut grid = GridStructure {
                columns: columns.clone(),
                rows: sub_rows,
                cells: sub_cells,
            };
            grid = grid.trim_empty_columns();
            if validate_table_structure_internal(&grid, config) {
                // (C) WS0.3b — the inserted header row (global row 0) lives in the
                // first non-empty run, so it is local row 0 of this sub-table when
                // `current_start_row == 0`. Protect it from colspan merging. ~keep
                let protected_header_rows = usize::from(inserted_header_row && current_start_row == 0);
                let mut table = grid_to_table(
                    &grid,
                    spans,
                    Some(detect_merged_cells_visually(
                        &grid,
                        spans,
                        cluster,
                        all_lines,
                        protected_header_rows,
                    )),
                );
                let mut min_y = f32::INFINITY;
                let mut max_y = f32::NEG_INFINITY;
                for r in &grid.rows {
                    min_y = min_y.min(r.y_min);
                    max_y = max_y.max(r.y_max);
                }
                table.bbox = Some(crate::geometry::Rect::new(
                    cluster.bbox.x,
                    min_y,
                    cluster.bbox.width,
                    max_y - min_y,
                ));
                let mut header_rows_detected = 0;
                let table_width = cluster.bbox.width;
                for r in 0..table.rows.len().min(3) {
                    let row_bottom = grid.rows[r].y_min;
                    let has_separator = cluster.lines.iter().any(|&idx| {
                        let path = &all_lines[idx];
                        let rendered = path.rendered_bbox();
                        path.is_horizontal_line(LINE_AXIS_TOL)
                            && rendered.width > table_width * 0.8
                            && (rendered.center().y - row_bottom).abs() < config.row_tolerance
                    });
                    if has_separator {
                        header_rows_detected = r + 1;
                    } else if r == 0 && table.rows[r].has_colspan() {
                        header_rows_detected = 1;
                    } else {
                        break;
                    }
                }
                if header_rows_detected > 0 {
                    table.has_header = true;
                    for r in 0..header_rows_detected {
                        if r < table.rows.len() {
                            table.rows[r].is_header = true;
                            for cell in &mut table.rows[r].cells {
                                cell.is_header = true;
                            }
                        }
                    }
                }
                tables.push(table);
            }
        }
        current_start_row = current_end_row + 1;
    }
    tables
}

fn detect_merged_cells_visually(
    grid: &GridStructure,
    spans: &[TextSpan],
    cluster: &LineCluster,
    all_lines: &[crate::elements::PathContent],
    protected_header_rows: usize,
) -> Vec<Vec<CellMergeInfo>> {
    let num_rows = grid.cells.len();
    let num_cols = grid.columns.len();
    const LINE_TOLERANCE: f32 = 2.0;
    let mut merge_info: Vec<Vec<CellMergeInfo>> = (0..num_rows)
        .map(|_| {
            (0..num_cols)
                .map(|_| CellMergeInfo {
                    colspan: 1,
                    rowspan: 1,
                    covered: false,
                })
                .collect()
        })
        .collect();
    for r in 0..num_rows {
        // (C) WS0.3b — a header row reconstructed from the unruled strip ABOVE
        // the grid has no vertical rulings in its band, which would otherwise
        // colspan-merge its distinct column cells into one (dropping every cell
        // but the first). Its cells were already verified to align to separate
        // columns, so skip colspan merging for these leading rows. ~keep
        if r < protected_header_rows {
            continue;
        }
        let mut c = 0;
        while c < num_cols {
            if merge_info[r][c].covered {
                c += 1;
                continue;
            }
            let mut colspan = 1;
            let mut cell_text_width: f32 = 0.0;
            for &idx in &grid.cells[r][c] {
                cell_text_width = cell_text_width.max(spans[idx].bbox.width);
            }
            let mut total_cell_width = grid.columns[c].x_max - grid.columns[c].x_min;
            for next_c in (c + 1)..num_cols {
                let separator_x = grid.columns[next_c].x_min;
                let y_min = grid.rows[r].y_min;
                let y_max = grid.rows[r].y_max;
                let has_separator = cluster.lines.iter().any(|&idx| {
                    let path = &all_lines[idx];
                    // Rendered extents: a stroke-width-encoded column rule
                    // crosses every row its drawn bar spans, not just the
                    // band around its geometric midline. ~keep
                    let rendered = path.rendered_bbox();
                    path.is_vertical_line(LINE_TOLERANCE)
                        && (rendered.center().x - separator_x).abs() < LINE_TOLERANCE
                        && rendered.y < y_max
                        && (rendered.y + rendered.height) > y_min
                });
                if !has_separator || (cell_text_width > total_cell_width + 2.0) {
                    colspan += 1;
                    total_cell_width += grid.columns[next_c].x_max - grid.columns[next_c].x_min;
                } else {
                    break;
                }
            }
            if colspan > 1 {
                merge_info[r][c].colspan = colspan;
                for i in 1..colspan {
                    merge_info[r][c + i as usize].covered = true;
                }
            }
            c += colspan as usize;
        }
    }
    for c in 0..num_cols {
        let mut r = 0;
        while r < num_rows {
            if merge_info[r][c].covered {
                r += 1;
                continue;
            }
            let mut rowspan = 1;
            let current_colspan = merge_info[r][c].colspan;
            for next_r in (r + 1)..num_rows {
                let separator_y = grid.rows[next_r].y_max;
                let x_min = grid.columns[c].x_min;
                let x_max = grid.columns[c + current_colspan as usize - 1].x_max;
                let has_separator = cluster.lines.iter().any(|&idx| {
                    let path = &all_lines[idx];
                    // Rendered extents, mirroring the colspan check above: a
                    // row rule encoded as a short vertical segment with a
                    // table-width stroke spans every column its drawn bar
                    // crosses. ~keep
                    let rendered = path.rendered_bbox();
                    path.is_horizontal_line(LINE_TOLERANCE)
                        && (rendered.center().y - separator_y).abs() < LINE_TOLERANCE
                        && rendered.x < x_max
                        && (rendered.x + rendered.width) > x_min
                });
                if !has_separator {
                    rowspan += 1;
                } else {
                    break;
                }
            }
            if rowspan > 1 {
                merge_info[r][c].rowspan = rowspan;
                for i in 1..rowspan {
                    merge_info[r + i as usize][c].covered = true;
                    for j in 1..current_colspan {
                        merge_info[r + i as usize][c + j as usize].covered = true;
                    }
                }
            }
            r += rowspan as usize;
        }
    }
    merge_info
}

/// Snap tolerance: parallel lines within this distance share a coordinate.
const SNAP_TOL: f32 = 3.0;
/// Join tolerance: collinear segments within this gap are merged.
const JOIN_TOL: f32 = 3.0;
/// Minimum edge length after merging; shorter edges are discarded.
const MIN_EDGE_LEN: f32 = 5.0;
/// Minimum number of short segments at the same coordinate to consider them a
/// dotted/dashed line candidate.
const DOTTED_MIN_SEGMENTS: usize = 3;
/// Minimum total span (in pt) of collinear short segments to reconstitute them
/// as a single continuous edge.
const DOTTED_MIN_SPAN: f32 = 50.0;
/// Snap precision for grouping dotted-line segments by coordinate (0.1 pt).
const DOTTED_COORD_SNAP: f32 = 10.0; // multiplier: coord * DOTTED_COORD_SNAP → i32 key ~keep

/// A horizontal or vertical edge (segment).
#[derive(Debug, Clone, Copy)]
struct Edge {
    /// For H edges: the shared y coordinate. For V edges: the shared x coordinate.
    coord: f32,
    /// Start of the range (min x for H, min y for V).
    start: f32,
    /// End of the range (max x for H, max y for V).
    end: f32,
}

/// An intersection point on the grid.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Intersection {
    x: f32,
    y: f32,
}

/// A rectangular cell defined by four corner intersections.
#[derive(Debug, Clone, Copy)]
struct IntersectionCell {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

/// Extract horizontal and vertical edges from path content, decomposing rectangles.
fn extract_edges(lines: &[crate::elements::PathContent]) -> (Vec<Edge>, Vec<Edge>) {
    const LINE_AXIS_TOL: f32 = 2.0;
    let mut h_edges: Vec<Edge> = Vec::new();
    let mut v_edges: Vec<Edge> = Vec::new();

    let mut rectangles = Vec::new();
    for path in lines {
        if path.is_horizontal_line(LINE_AXIS_TOL) {
            // Rendered extents so a stroke-width-encoded rule contributes
            // the edge its drawn bar covers, not its geometric speck
            // Identical to `bbox` for ordinary thin rules. ~keep
            let rendered = path.rendered_bbox();
            h_edges.push(Edge {
                coord: rendered.center().y,
                start: rendered.left(),
                end: rendered.right(),
            });
        } else if path.is_vertical_line(LINE_AXIS_TOL) {
            let rendered = path.rendered_bbox();
            v_edges.push(Edge {
                coord: rendered.center().x,
                start: rendered.top(),
                end: rendered.bottom(),
            });
        } else if path.is_rectangle() {
            rectangles.push(path);
        }
    }

    // Thin filled bars are rules too. Merge only these explicit rules before
    // considering broad rectangles, so a background cannot justify itself. ~keep
    let mut ruled_h = h_edges.clone();
    let mut ruled_v = v_edges.clone();
    snap_and_merge(&mut ruled_h);
    snap_and_merge(&mut ruled_v);
    for path in &rectangles {
        let bbox = &path.bbox;
        let (l, r, t, b) = (bbox.left(), bbox.right(), bbox.top(), bbox.bottom());
        // Producers can paint both a cell background and an inset text background.
        // Their sides are padding, not columns, when drawn rules already close
        // this row band around them. Keep unruled fills (e.g. zebra tables). ~keep
        let inset_background = path.has_fill()
            && !path.has_stroke()
            && rectangles.iter().any(|outer| {
                outer.has_fill()
                    && !outer.has_stroke()
                    && outer.fill_color == path.fill_color
                    && outer.bbox.left() < l - SNAP_TOL
                    && outer.bbox.right() > r + SNAP_TOL
                    && (outer.bbox.top() - t).abs() <= SNAP_TOL
                    && (outer.bbox.bottom() - b).abs() <= SNAP_TOL
            });
        let inside_ruled_band = inset_background
            && ruled_h.iter().any(|top| {
                (top.coord - t).abs() <= SNAP_TOL
                    && top.start <= l + SNAP_TOL
                    && top.end >= r - SNAP_TOL
                    && ruled_h.iter().any(|bottom| {
                        (bottom.coord - b).abs() <= SNAP_TOL
                            && bottom.start <= l + SNAP_TOL
                            && bottom.end >= r - SNAP_TOL
                            && [true, false].into_iter().all(|left| {
                                ruled_v.iter().any(|side| {
                                    side.start <= t + SNAP_TOL
                                        && side.end >= b - SNAP_TOL
                                        && if left {
                                            side.coord <= l + SNAP_TOL
                                                && side.coord >= top.start.max(bottom.start) - SNAP_TOL
                                        } else {
                                            side.coord >= r - SNAP_TOL
                                                && side.coord <= top.end.min(bottom.end) + SNAP_TOL
                                        }
                                })
                            })
                    })
            });
        if inside_ruled_band {
            continue;
        }
        h_edges.push(Edge {
            coord: t,
            start: l,
            end: r,
        });
        h_edges.push(Edge {
            coord: b,
            start: l,
            end: r,
        });
        v_edges.push(Edge {
            coord: l,
            start: t,
            end: b,
        });
        v_edges.push(Edge {
            coord: r,
            start: t,
            end: b,
        });
    }
    (h_edges, v_edges)
}

/// Snap parallel edges within `SNAP_TOL` to the same coordinate, join collinear
/// segments within `JOIN_TOL`, and discard edges shorter than `MIN_EDGE_LEN`.
fn snap_and_merge(edges: &mut Vec<Edge>) {
    snap_edges(edges);
    join_collinear_edges(edges);
    reconstitute_dotted_lines(edges);
}

/// Phase 1: Sort edges by coord and snap nearby coordinates (within `SNAP_TOL`)
/// to the first coordinate in each group.
fn snap_edges(edges: &mut [Edge]) {
    if edges.is_empty() {
        return;
    }
    edges.sort_by(|a, b| crate::utils::safe_float_cmp(a.coord, b.coord));

    let mut i = 0;
    while i < edges.len() {
        let base_coord = edges[i].coord;
        let mut j = i + 1;
        while j < edges.len() && (edges[j].coord - base_coord).abs() <= SNAP_TOL {
            edges[j].coord = base_coord;
            j += 1;
        }
        i = j;
    }
}

/// Phase 2: Sort by (coord, start) and merge overlapping or adjacent collinear
/// segments into single edges.
fn join_collinear_edges(edges: &mut Vec<Edge>) {
    if edges.is_empty() {
        return;
    }
    // Sort by coord then start so a single sweep handles chains of touching
    // segments regardless of the order they were originally collected. ~keep
    edges.sort_by(|a, b| {
        crate::utils::safe_float_cmp(a.coord, b.coord).then_with(|| crate::utils::safe_float_cmp(a.start, b.start))
    });

    let mut merged: Vec<Edge> = Vec::new();
    for &edge in edges.iter() {
        // Use SNAP_TOL for the coord comparison (not f32::EPSILON) so that
        // edges whose coords were snapped from slightly different originals
        // still join correctly. ~keep
        let should_merge = merged.last().is_some_and(|prev: &Edge| {
            (prev.coord - edge.coord).abs() <= SNAP_TOL && edge.start <= prev.end + JOIN_TOL
        });
        if should_merge {
            let prev = merged.last_mut().unwrap();
            prev.end = prev.end.max(edge.end);
        } else {
            merged.push(edge);
        }
    }

    *edges = merged;
}

/// Phase 3: Group short segments (below `MIN_EDGE_LEN`) by coordinate.  When a
/// group has >= `DOTTED_MIN_SEGMENTS` members spanning >= `DOTTED_MIN_SPAN`
/// points, replace them with a single long edge. Short segments that do not
/// qualify are discarded.
fn reconstitute_dotted_lines(edges: &mut Vec<Edge>) {
    let mut dotted_groups: HashMap<i32, Vec<Edge>> = HashMap::new();
    let mut long_edges: Vec<Edge> = Vec::new();

    for &edge in edges.iter() {
        if (edge.end - edge.start) >= MIN_EDGE_LEN {
            long_edges.push(edge);
        } else {
            let key = (edge.coord * DOTTED_COORD_SNAP).round() as i32;
            dotted_groups.entry(key).or_default().push(edge);
        }
    }

    // Iterate in sorted key order: `dotted_groups` is a HashMap (per-process-
    // randomized), and the reconstituted edges are appended to `long_edges`
    // (which becomes `*edges`), so HashMap order would leak into edge order and,
    // downstream, table-cell/region order. Sorting the snapped-coordinate keys
    // makes it deterministic. ~keep
    let mut dotted_keys: Vec<i32> = dotted_groups.keys().copied().collect();
    dotted_keys.sort_unstable();
    for key in dotted_keys {
        let segments = &dotted_groups[&key];
        if segments.len() >= DOTTED_MIN_SEGMENTS {
            let min_start = segments
                .iter()
                .map(|e| e.start)
                .min_by(|a, b| crate::utils::safe_float_cmp(*a, *b))
                .unwrap();
            let max_end = segments
                .iter()
                .map(|e| e.end)
                .max_by(|a, b| crate::utils::safe_float_cmp(*a, *b))
                .unwrap();
            let total_span = max_end - min_start;
            if total_span >= DOTTED_MIN_SPAN {
                // Use the coordinate of the first segment (they are all snapped
                // to the same value within SNAP_TOL anyway). ~keep
                long_edges.push(Edge {
                    coord: segments[0].coord,
                    start: min_start,
                    end: max_end,
                });
            }
        }
    }

    // No additional short-edge discard needed: long_edges already excludes
    // short segments that were not reconstituted. ~keep
    *edges = long_edges;
}

/// Remove orphan edges that have no plausible counterpart in the other axis.
///
/// For each H-edge, keep it only if at least one V-edge has an X coordinate
/// within the H-edge's X-range (with generous tolerance).
/// For each V-edge, keep it only if at least one H-edge has an X-range that
/// overlaps with the V-edge's X coordinate (with generous tolerance).
///
/// This is purely an X-range overlap check. The Y-axis relationship is
/// intentionally ignored because the extended grid projects V-line X positions
/// across all H-line Y positions regardless of whether they share Y ranges
/// (e.g., Census tables where H-lines and V-lines occupy different Y regions).
fn filter_edges_by_coverage(h_edges: &mut Vec<Edge>, v_edges: &mut Vec<Edge>) {
    let all_x_min = h_edges
        .iter()
        .map(|e| e.start)
        .chain(v_edges.iter().map(|e| e.coord))
        .fold(f32::INFINITY, f32::min);
    let all_x_max = h_edges
        .iter()
        .map(|e| e.end)
        .chain(v_edges.iter().map(|e| e.coord))
        .fold(f32::NEG_INFINITY, f32::max);
    let x_span = (all_x_max - all_x_min).max(1.0);
    let x_tol = x_span * 0.5;

    h_edges.retain(|h| {
        v_edges
            .iter()
            .any(|v| v.coord >= h.start - x_tol && v.coord <= h.end + x_tol)
    });

    v_edges.retain(|v| {
        h_edges
            .iter()
            .any(|h| v.coord >= h.start - x_tol && v.coord <= h.end + x_tol)
    });
}

/// Find all intersection points where an H edge and a V edge actually cross.
fn find_intersections(h_edges: &[Edge], v_edges: &[Edge]) -> Vec<Intersection> {
    let mut pts: Vec<Intersection> = Vec::new();
    for h in h_edges {
        for v in v_edges {
            if v.coord >= h.start - SNAP_TOL
                && v.coord <= h.end + SNAP_TOL
                && h.coord >= v.start - SNAP_TOL
                && h.coord <= v.end + SNAP_TOL
            {
                pts.push(Intersection { x: v.coord, y: h.coord });
            }
        }
    }
    pts.sort_by(|a, b| crate::utils::safe_float_cmp(a.x, b.x).then_with(|| crate::utils::safe_float_cmp(a.y, b.y)));
    pts.dedup_by(|a, b| (a.x - b.x).abs() <= SNAP_TOL && (a.y - b.y).abs() <= SNAP_TOL);
    pts
}

/// Tolerance for judging whether a V edge spans a *candidate cell's* Y-range, at
/// cell-construction time in `build_cells_from_intersections`.
///
/// This is deliberately a SEPARATE constant from `BAND_RULE_SPAN_TOL`, not a reuse of it, even
/// though both answer the same shape of question ("does this edge actually run through the
/// range?") on opposite axes. `BAND_RULE_SPAN_TOL` was tightened from 3.0 (`SNAP_TOL`) down to
/// 1.0 specifically because on the X axis a too-LOOSE tolerance was the bug: it let a V rule that
/// fell up to 3pt short *at each end* still count as dividing a row's columns, manufacturing a
/// phantom column boundary the drawn rule did not justify (xberg-io/xberg#1588). The fix needed
/// to be conservative in the direction of NOT crediting a short edge.
///
/// On the Y axis the failure direction is the opposite. A too-TIGHT tolerance here does not merely
/// merge columns within a row that still exists — it can stop the row's cell from forming at all
/// (xberg-io/xberg#1601), which is the more severe failure. A per-cell rule drawn a few points
/// short of its own row's true top/bottom (ordinary visual padding) must still count as spanning
/// that row, while a phantom cell bridging two physically separate ruled regions — tens of points
/// apart (40pt in this file's `gh1601_graphics_free_gap_does_not_bridge_two_tables` fixture, 54pt
/// in the reporter's carrier document) — must not. `BAND_RULE_SPAN_TOL`'s 1.0pt is too tight for
/// the first case; reusing it would silently drop legitimate padded-rule tables' rows. 6.0pt sits
/// roughly 1-3x above a "a few points" (2-5pt) legitimate inset and roughly 7x below the smallest
/// observed phantom-cell gap in either direction (40 / 6 ≈ 6.7, 54 / 6 ≈ 9), so it separates the
/// two classes with comfortable margin on both sides without conflating this axis's tolerance with
/// the X axis's differently-motivated one. ~keep
const CELL_RULE_SPAN_TOL: f32 = 6.0;

/// Cap on how many widened candidate columns `build_cells_from_intersections` tries per
/// starting corner once the nearest crossing cannot close a cell (`divider_ends_below`).
/// Each extra candidate re-scans `v_edges`/`h_edges` inside `side_closes`/`band_is_ruled`,
/// so an unbounded retry turns the outer O(nx*ny) loop into O(nx^2*ny*E) on input with a
/// broken divider at every column — PR #1807's own description reports four large corpus
/// files timing out from exactly this. Measured maximum candidate index actually needed:
/// 2 across `pdf_grouped_table_headers` + `pdf_spanning_table_cells`, 3 across the full
/// `spatial_table_detector` unit-test module (including
/// `header_cell_search_skips_nonclosing_inner_corner`). 8 leaves comfortable headroom
/// above both without reintroducing the unbounded scan. ~keep
const MAX_HEADER_CELL_WIDENING_CANDIDATES: usize = 8;

/// Build cells from intersection points.
/// A cell exists when all four corners (x1,y1), (x2,y1), (x1,y2), (x2,y2) are present
/// and there is no intermediate intersection between them on either axis.
fn build_cells_from_intersections(pts: &[Intersection], h_edges: &[Edge], v_edges: &[Edge]) -> Vec<IntersectionCell> {
    use std::collections::BTreeSet;

    let mut xs: Vec<f32> = pts.iter().map(|p| p.x).collect();
    let mut ys: Vec<f32> = pts.iter().map(|p| p.y).collect();
    xs.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
    xs.dedup_by(|a, b| (*a - *b).abs() <= SNAP_TOL);
    ys.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
    ys.dedup_by(|a, b| (*a - *b).abs() <= SNAP_TOL);

    let x_idx = |xv: f32| -> Option<usize> { xs.iter().position(|&c| (c - xv).abs() <= SNAP_TOL) };
    let y_idx = |yv: f32| -> Option<usize> { ys.iter().position(|&c| (c - yv).abs() <= SNAP_TOL) };

    let nx = xs.len();
    let ny = ys.len();
    let mut present: BTreeSet<usize> = BTreeSet::new();
    for p in pts {
        if let (Some(xi), Some(yi)) = (x_idx(p.x), y_idx(p.y)) {
            present.insert(yi * nx + xi);
        }
    }

    let has = |xi: usize, yi: usize| -> bool { present.contains(&(yi * nx + xi)) };

    // Four corners are not four sides. Two unrelated ruled grids that share column X-positions
    // (the same field layout repeated after a section heading) put crossing points at all four
    // corners of the graphics-free gap between them, and a cell there swallows whatever text
    // sits in the gap (xberg-io/xberg#1601). So a cell forms only when each side is a drawn V
    // edge spanning its Y-range, or when the band between the cell's own top and bottom H rules
    // is part of a grid: a drawn rule crosses it strictly inside those rules' ends, or drawn
    // rules close both ends. That keeps rows whose sides are not drawn in that row: merged cells
    // and full-width section rows, and a zebra-shaded table's unshaded rows, where the only edges
    // at the outer x are the shaded neighbours' fill sides. A rule at one end only (an enclosing
    // table's column rule) or past them (a page frame) runs alongside a gap without closing it.
    // `band_column_groups` (xberg-io/xberg#1580) then merges any boundary no rule divides.
    // `CELL_RULE_SPAN_TOL`'s doc comment says why this axis does not share
    // `BAND_RULE_SPAN_TOL`. ~keep
    let spans_band = |edge: &Edge, y_lo: f32, y_hi: f32| -> bool {
        edge.start <= y_lo + CELL_RULE_SPAN_TOL && edge.end >= y_hi - CELL_RULE_SPAN_TOL
    };
    let v_edge_spans = |x: f32, y_lo: f32, y_hi: f32| -> bool {
        v_edges
            .iter()
            .any(|edge| (edge.coord - x).abs() <= SNAP_TOL && spans_band(edge, y_lo, y_hi))
    };

    let h_edge_across = |y: f32, x1: f32, x2: f32| -> Option<&Edge> {
        h_edges
            .iter()
            .find(|edge| (edge.coord - y).abs() <= SNAP_TOL && edge.start <= x1 + SNAP_TOL && edge.end >= x2 - SNAP_TOL)
    };
    let band_is_ruled = |x1: f32, x2: f32, y_lo: f32, y_hi: f32| -> bool {
        let (Some(top), Some(bottom)) = (h_edge_across(y_lo, x1, x2), h_edge_across(y_hi, x1, x2)) else {
            return false;
        };
        let (from, to) = (top.start.max(bottom.start), top.end.min(bottom.end));
        (v_edge_spans(from, y_lo, y_hi) && v_edge_spans(to, y_lo, y_hi))
            || v_edges
                .iter()
                .any(|edge| edge.coord > from + SNAP_TOL && edge.coord < to - SNAP_TOL && spans_band(edge, y_lo, y_hi))
    };

    let mut cells = Vec::new();
    for yi in 0..ny {
        for xi in 0..nx {
            if !has(xi, yi) {
                continue;
            }
            // A divider may end at this row boundary. Try farther crossings
            // when the nearest one cannot close a cell above it. ~keep
            let divider_ends_below = ((xi + 1)..nx).find(|&next| has(next, yi)).is_some_and(|next| {
                v_edges.iter().any(|edge| {
                    (edge.coord - xs[next]).abs() <= SNAP_TOL
                        && edge.start < ys[yi] - SNAP_TOL
                        && (edge.end - ys[yi]).abs() <= CELL_RULE_SPAN_TOL
                })
            });
            for (candidate, nxi) in ((xi + 1)..nx).filter(|&nxi| has(nxi, yi)).enumerate() {
                if candidate > 0 && (!divider_ends_below || candidate > MAX_HEADER_CELL_WIDENING_CANDIDATES) {
                    break;
                }
                let side_closes = |x: f32, nyi: usize| -> bool {
                    v_edge_spans(x, ys[yi], ys[nyi]) || band_is_ruled(xs[xi], xs[nxi], ys[yi], ys[nyi])
                };
                // A neighbouring row separator may end at just one side of a
                // spanning cell. Find a closing boundary on both sides rather
                // than abandoning the cell at that one-sided crossing. ~keep
                let mut next_yi = None;
                for (candidate_y, nyi) in ((yi + 1)..ny)
                    .filter(|&nyi| has(xi, nyi) && side_closes(xs[xi], nyi))
                    .enumerate()
                {
                    if has(nxi, nyi)
                        && side_closes(xs[nxi], nyi)
                        && (candidate_y == 0
                            || (v_edge_spans(xs[xi], ys[yi], ys[nyi])
                                && v_edge_spans(xs[nxi], ys[yi], ys[nyi])
                                && h_edge_across(ys[yi], xs[xi], xs[nxi]).is_some()
                                && h_edge_across(ys[nyi], xs[xi], xs[nxi]).is_some()))
                    {
                        next_yi = Some(nyi);
                        break;
                    }
                    // Only skip crossings from a neighbouring cell. A rule
                    // entering this cell is a real internal boundary, even if
                    // it cannot close at the chosen right edge. ~keep
                    if h_edges.iter().any(|edge| {
                        (edge.coord - ys[nyi]).abs() <= SNAP_TOL
                            && edge.end > xs[xi] + SNAP_TOL
                            && edge.start < xs[nxi] - SNAP_TOL
                    }) {
                        break;
                    }
                }

                if let Some(nyi) = next_yi
                    && (candidate == 0
                        || (v_edge_spans(xs[xi], ys[yi], ys[nyi])
                            && v_edge_spans(xs[nxi], ys[yi], ys[nyi])
                            && h_edge_across(ys[yi], xs[xi], xs[nxi]).is_some()
                            && h_edge_across(ys[nyi], xs[xi], xs[nxi]).is_some()))
                {
                    cells.push(IntersectionCell {
                        x1: xs[xi],
                        y1: ys[yi],
                        x2: xs[nxi],
                        y2: ys[nyi],
                    });
                    break;
                }
            }
        }
    }
    cells
}

/// Build grid cells from the Cartesian product of H-edge Y-positions and V-edge X-positions.
///
/// This "extended grid" approach handles the case where horizontal and vertical lines
/// don't physically intersect (e.g., H-lines in a header area and V tick marks in a data
/// area). Instead of requiring actual crossings, we project every unique V-line X coordinate
/// across every unique H-line Y coordinate to create virtual grid intersections.
fn build_extended_grid_cells(h_edges: &[Edge], v_edges: &[Edge]) -> Vec<IntersectionCell> {
    let mut ys: Vec<f32> = h_edges.iter().map(|e| e.coord).collect();
    ys.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
    ys.dedup_by(|a, b| (*a - *b).abs() <= SNAP_TOL);

    let mut xs: Vec<f32> = v_edges.iter().map(|e| e.coord).collect();
    xs.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
    xs.dedup_by(|a, b| (*a - *b).abs() <= SNAP_TOL);

    if xs.len() < 2 || ys.len() < 2 {
        return Vec::new();
    }

    let mut cells = Vec::new();
    for yi in 0..ys.len() - 1 {
        for xi in 0..xs.len() - 1 {
            cells.push(IntersectionCell {
                x1: xs[xi],
                y1: ys[yi],
                x2: xs[xi + 1],
                y2: ys[yi + 1],
            });
        }
    }
    cells
}

/// Group cells that share edges into tables using union-find.
fn group_cells_into_tables(cells: &[IntersectionCell]) -> Vec<Vec<usize>> {
    if cells.is_empty() {
        return Vec::new();
    }
    let n = cells.len();
    let mut uf = UnionFind::new(n);

    // Sweep-line prune for the O(n²) edge-adjacency scan (the hot loop on dense
    // ruled pages — CFR regulatory megafiles). BOTH adjacency tests below
    // require the cells' y-extents to touch within SNAP_TOL: horizontal
    // adjacency needs y1≈y1 (so cj.y1 ≤ ci.y2 + SNAP_TOL), and vertical
    // adjacency needs cj.y1 ≈ ci.y2 (also ≤ ci.y2 + SNAP_TOL). Iterating cells
    // in ascending-y1 order lets us `break` the inner loop once a candidate's
    // y1 clears ci.y2 + SNAP_TOL — every later candidate has an even larger y1
    // and cannot share an edge. We `union` by ORIGINAL index, and union is
    // order-independent, so the resulting partition is byte-identical to the
    // full O(n²) scan; only provably-non-adjacent pairs are skipped. ~keep
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| crate::utils::safe_float_cmp(cells[a].y1, cells[b].y1));
    for a in 0..n {
        let i = order[a];
        let ci = &cells[i];
        let y_limit = ci.y2 + SNAP_TOL;
        for &j in order.iter().skip(a + 1) {
            let cj = &cells[j];
            if cj.y1 > y_limit {
                break;
            }
            let shares_edge = // Horizontal adjacency (share a vertical edge) ~keep
                (((ci.x2 - cj.x1).abs() <= SNAP_TOL || (ci.x1 - cj.x2).abs() <= SNAP_TOL)
                    && (ci.y1 - cj.y1).abs() <= SNAP_TOL
                    && (ci.y2 - cj.y2).abs() <= SNAP_TOL)
                || // Vertical adjacency (share a horizontal edge) ~keep
                (((ci.y2 - cj.y1).abs() <= SNAP_TOL || (ci.y1 - cj.y2).abs() <= SNAP_TOL)
                    && (ci.x1 - cj.x1).abs() <= SNAP_TOL
                    && (ci.x2 - cj.x2).abs() <= SNAP_TOL);
            if shares_edge {
                uf.union(i, j);
            }
        }
    }

    // Collect groups in a DETERMINISTIC order. `groups()` returns a HashMap
    // whose iteration order is randomized per-process (Rust `RandomState`), so
    // `into_values().collect()` would yield table clusters in a different order
    // each run — leaking non-deterministic reading order on multi-table / figure
    // pages (e.g. matrix-figure pages with several detected regions). Each
    // group's `Vec` is already ascending (built `for i in 0..n`); sort the outer
    // list by each group's first (smallest) cell index so table order is stable. ~keep
    let mut groups: Vec<Vec<usize>> = uf.groups().into_values().collect();
    groups.sort_by_key(|g| g.first().copied().unwrap_or(usize::MAX));
    groups
}

/// Attach a spanning header only to an independently established body grid.
/// A few header boxes alone must not become a newly accepted table: that can
/// suppress the fallback detector for the real body on the same page. ~keep
fn attach_spanning_headers(cells: &[IntersectionCell], groups: Vec<Vec<usize>>, min_cells: usize) -> Vec<Vec<usize>> {
    let mut uf = UnionFind::new(groups.len());
    for (body_index, body) in groups.iter().enumerate() {
        if body.len() < min_cells {
            continue;
        }
        // One ruled row can be only a header above an unruled body. Attaching
        // another heading must not make that fragment claim the whole page. ~keep
        let first_y = cells[body[0]].y1;
        if body.iter().all(|&i| (cells[i].y1 - first_y).abs() <= SNAP_TOL) {
            continue;
        }
        let left = body.iter().map(|&i| cells[i].x1).fold(f32::INFINITY, f32::min);
        let right = body.iter().map(|&i| cells[i].x2).fold(f32::NEG_INFINITY, f32::max);
        let top = body.iter().map(|&i| cells[i].y2).fold(f32::NEG_INFINITY, f32::max);
        for (header_index, header) in groups.iter().enumerate() {
            if body_index == header_index
                || !header.iter().all(|&i| {
                    let c = cells[i];
                    c.y1 >= top - SNAP_TOL && c.x1 >= left - SNAP_TOL && c.x2 <= right + SNAP_TOL
                })
            {
                continue;
            }
            let shares_edge = header.iter().any(|&h| {
                body.iter().any(|&b| {
                    let (h, b) = (cells[h], cells[b]);
                    (h.y1 - b.y2).abs() <= SNAP_TOL && h.x2.min(b.x2) - h.x1.max(b.x1) > SNAP_TOL
                })
            });
            if shares_edge {
                uf.union(body_index, header_index);
            }
        }
    }
    let mut joined: Vec<Vec<usize>> = uf
        .groups()
        .into_values()
        .map(|indices| {
            let mut members: Vec<_> = indices.into_iter().flat_map(|i| groups[i].iter().copied()).collect();
            members.sort_unstable();
            members
        })
        .collect();
    joined.sort_by_key(|g| g.first().copied().unwrap_or(usize::MAX));
    joined
}

/// Minimum number of distinct columns a Y-cluster must carry independent text
/// evidence in for a row split to be credible. A cluster below this bar is a
/// single wrapped cell's continuation line, not a second logical row
/// (xberg-io/xberg#1555). ~keep
const MIN_ROW_SPLIT_EVIDENCE_COLUMNS: usize = 2;

/// Minimum number of Y-clusters that must *independently* clear
/// `MIN_ROW_SPLIT_EVIDENCE_COLUMNS` before a producer-drawn row band is treated as more than
/// one row at all. One evidenced cluster is not enough: two unrelated single-column spans can
/// land within `row_tolerance` of each other by pure chance (see the #1555 regression fixture
/// `split_rows_by_text_positions_keeps_drawn_band_as_one_row_when_only_one_cell_wraps`, where a
/// row-number span and an unrelated wrapped cell's middle baseline coincide on one Y value and
/// nowhere else) and that single coincidence must not be enough to accept a split. A second,
/// independently evidenced cluster is what tells a genuinely tabular band apart from one
/// baseline that happened to line up (xberg-io/xberg#1565). ~keep
const MIN_EVIDENCED_CLUSTERS_FOR_SPLIT: usize = 2;

/// Split table rows that contain text spans at multiple distinct Y positions into sub-rows.
///
/// This handles the hybrid case where column boundaries come from vertical lines but there
/// are no horizontal lines between individual rows. In that scenario the intersection-based
/// pipeline produces a single mega-row; this function detects multiple Y-clusters within
/// each row and splits accordingly.
///
/// The only caller (`finalize_intersection_tables`) runs exclusively on grids the drawn
/// geometry already delimited into rows (`detect_tables_from_intersections` via
/// `build_grid_from_lines`), so every row entering here is a producer-drawn row band, and a
/// split is a hypothesis about *sub*-dividing that band, not about discovering rows from
/// scratch. The drawn band boundary is authoritative; text baselines inside it are only
/// advisory.
///
/// The band is treated as more than one row only once at least
/// `MIN_EVIDENCED_CLUSTERS_FOR_SPLIT` of its Y-clusters are *independently* evidenced in
/// `MIN_ROW_SPLIT_EVIDENCE_COLUMNS` columns; see that constant's doc for why one evidenced
/// cluster is not trusted on its own. Once the band clears that bar, each remaining deficient
/// cluster (one that does not clear the evidence bar by itself) is resolved on its own terms
/// instead of vetoing the whole band (xberg-io/xberg#1565: a band mixing multi-column data
/// rows with single-column section headings, lead-ins, or wrapped continuations was previously
/// collapsed back into one mega-row in full, and the headings disappeared into whatever cell
/// absorbed them). A deficient cluster folds into the cluster directly above it in reading
/// order — the adjacent Y-cluster with the *larger* y, since PDF space is y-up and "above" on
/// the page means a larger y — only when the fold cannot manufacture evidence that was not
/// already there: the deficient cluster's populated columns must already be a non-empty subset
/// of the columns the cluster above populates. That is the signature of a wrapped
/// continuation line, which only ever repeats content in a column the cell above was already
/// using. A deficient cluster that fails this test — because it has no cluster above it at
/// all, or because it carries text in a column the cluster above left empty — is kept as a row
/// of its own, one cell wide: that is what a heading or lead-in line inside a ruled band
/// actually is.
///
/// The evidence bar is capped at the band's own column count
/// (`MIN_ROW_SPLIT_EVIDENCE_COLUMNS.min(num_cols)`): a single-column band has no second
/// column to ever produce corroborating evidence from, so requiring 2 there would reject
/// every split unconditionally, including genuinely distinct rows sharing one drawn cell
/// (e.g. a label/value pair). For `num_cols == 1` this restores the pre-#1555 behaviour of
/// accepting any non-empty cluster as evidence; the #1555 guard itself only applies where
/// it can be evaluated, at `num_cols >= 2`.
fn split_rows_by_text_positions(
    table_rows: Vec<TableRow>,
    row_cell_span_indices: &[Vec<Vec<usize>>],
    spans: &[TextSpan],
    config: &TableDetectionConfig,
) -> Vec<TableRow> {
    let mut result: Vec<TableRow> = Vec::new();

    for (row_idx, row) in table_rows.into_iter().enumerate() {
        let cell_indices = &row_cell_span_indices[row_idx];

        let mut all_ys: Vec<f32> = Vec::new();
        for col_spans in cell_indices {
            for &idx in col_spans {
                if let Some(s) = spans.get(idx) {
                    all_ys.push(s.bbox.center().y);
                }
            }
        }

        if all_ys.len() <= 1 {
            result.push(row);
            continue;
        }

        all_ys.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
        let mut y_clusters: Vec<f32> = Vec::new();
        for &y in &all_ys {
            let merged = y_clusters
                .last()
                .is_some_and(|&last| (y - last).abs() < config.row_tolerance);
            if merged {
                let last = y_clusters.last_mut().unwrap();
                *last = (*last + y) / 2.0;
            } else {
                y_clusters.push(y);
            }
        }

        if y_clusters.len() <= 1 {
            result.push(row);
            continue;
        }

        let num_cols = row.cells.len();
        let nearest_cluster_value = |sy: f32| -> f32 {
            *y_clusters
                .iter()
                .min_by_key(|&&cy| ((sy - cy).abs() * 1000.0) as i32)
                .unwrap_or(&sy)
        };

        // Tally, per cluster, which columns have at least one span assigned to it. ~keep
        let mut cluster_columns: Vec<Vec<bool>> = vec![vec![false; num_cols]; y_clusters.len()];
        for (ci, col_spans) in cell_indices.iter().enumerate() {
            for &idx in col_spans {
                if let Some(s) = spans.get(idx) {
                    let nearest = nearest_cluster_value(s.bbox.center().y);
                    if let Some(cluster_idx) = y_clusters.iter().position(|&cy| (cy - nearest).abs() < 0.01) {
                        cluster_columns[cluster_idx][ci] = true;
                    }
                }
            }
        }

        // A single-column band (num_cols == 1) can never carry cross-column
        // evidence at all — there is only one column to begin with, so the
        // wrapped-cell-vs-genuine-row ambiguity #1555 targets is undecidable
        // by this signal there. Requiring the full MIN_ROW_SPLIT_EVIDENCE_COLUMNS
        // in that case would make the gate reject every split on a
        // single-column grid unconditionally, including genuinely distinct
        // rows (e.g. label/value pairs sharing one drawn cell). Scale the
        // requirement down to the band's own column count so a single-column
        // band keeps its pre-#1555 behaviour (any non-empty cluster counts as
        // evidence) while a multi-column band still needs the full bar
        // (xberg-io/xberg#1555). ~keep
        let required_evidence_columns = MIN_ROW_SPLIT_EVIDENCE_COLUMNS.min(num_cols.max(1));
        let is_evidenced: Vec<bool> = cluster_columns
            .iter()
            .map(|cols| cols.iter().filter(|&&present| present).count() >= required_evidence_columns)
            .collect();
        let evidenced_cluster_count = is_evidenced.iter().filter(|&&evidenced| evidenced).count();

        if evidenced_cluster_count < MIN_EVIDENCED_CLUSTERS_FOR_SPLIT {
            result.push(row);
            continue;
        }

        // Resolve a fold target for every deficient cluster: it folds into the cluster
        // directly above it (ascending index + 1, i.e. the next-larger y) only when doing so
        // introduces no column the cluster above didn't already have. Evidenced clusters never
        // fold — they are anchors in their own right. Fold targets strictly increase (i can
        // only point to i + 1), so following the chain in `resolve_group` always terminates
        // without needing cycle protection. ~keep
        let mut fold_into: Vec<Option<usize>> = vec![None; y_clusters.len()];
        for i in 0..y_clusters.len().saturating_sub(1) {
            if is_evidenced[i] {
                continue;
            }
            let is_wrapped_continuation = cluster_columns[i]
                .iter()
                .zip(cluster_columns[i + 1].iter())
                .all(|(&here, &above)| !here || above);
            if is_wrapped_continuation {
                fold_into[i] = Some(i + 1);
            }
        }

        let resolve_group = |mut idx: usize| -> usize {
            while let Some(next) = fold_into[idx] {
                idx = next;
            }
            idx
        };

        let mut group_members: Vec<Vec<usize>> = vec![Vec::new(); y_clusters.len()];
        for i in 0..y_clusters.len() {
            group_members[resolve_group(i)].push(i);
        }

        // Emit one output row per non-empty group, topmost (largest y) first. A group's
        // resolved anchor index always carries the largest y in the group, since folding only
        // ever points toward a larger-y neighbor. ~keep
        let mut group_order: Vec<usize> = (0..y_clusters.len())
            .filter(|&g| !group_members[g].is_empty())
            .collect();
        group_order.sort_by(|&a, &b| crate::utils::safe_float_cmp(y_clusters[b], y_clusters[a]));

        for group_idx in group_order {
            let members = &group_members[group_idx];
            let mut new_row = TableRow::new(row.is_header);
            for ci in 0..num_cols {
                let matching_indices: Vec<usize> = cell_indices[ci]
                    .iter()
                    .copied()
                    .filter(|&idx| {
                        spans
                            .get(idx)
                            .map(|s| {
                                let nearest = nearest_cluster_value(s.bbox.center().y);
                                members.iter().any(|&m| (y_clusters[m] - nearest).abs() < 0.01)
                            })
                            .unwrap_or(false)
                    })
                    .collect();

                let cell_text = extract_cell_text(&matching_indices, spans);
                let mcids: Vec<u32> = matching_indices
                    .iter()
                    .filter_map(|&idx| spans.get(idx).and_then(|s| s.mcid))
                    .collect();

                let cell_bbox = if matching_indices.is_empty() {
                    row.cells[ci].bbox
                } else {
                    let mut b = spans[matching_indices[0]].bbox;
                    for &idx in &matching_indices[1..] {
                        b = b.union(&spans[idx].bbox);
                    }
                    Some(b)
                };

                let cell_spans = matching_indices
                    .iter()
                    .filter_map(|&idx| spans.get(idx).cloned())
                    .collect::<Vec<_>>();

                new_row.cells.push(TableCell {
                    text: cell_text,
                    spans: cell_spans,
                    colspan: 1,
                    rowspan: 1,
                    mcids,
                    bbox: cell_bbox,
                    is_header: row.is_header,
                });
            }
            result.push(new_row);
        }
    }

    result
}

/// Strip form-template numbering artifacts and decorative separators from table rows.
///
/// PDF form templates sometimes embed single-digit numbering (e.g. "1", "5") as
/// separate text spans that get concatenated into cell text as a prefix. They also
/// use rows or cells filled with dashes/underscores as decorative separators.
/// This function:
/// 1. Removes entire rows where every cell is either empty or a lone single digit.
/// 2. Strips a leading single-digit prefix from cell text when the remainder looks
///    like real content (starts with a letter, `$`, or contains `/` or `-`).
/// 3. Clears cells that contain only dashes/underscores (decorative separators).
/// 4. Removes rows where all cells are empty after separator stripping.
fn strip_form_numbering_artifacts(table_rows: &mut Vec<TableRow>) {
    // Phase 1: Remove rows where ALL cells are either empty or a lone single
    // digit (1-9), AND at least one cell actually contains a digit.  Rows that
    // are completely empty are left intact so the downstream empty-row splitting
    // logic can use them as table separators. ~keep
    table_rows.retain(|row| {
        let all_empty_or_digit = row.cells.iter().all(|c| {
            let t = c.text.trim();
            t.is_empty() || (t.len() == 1 && t.as_bytes().first().is_some_and(|b| b.is_ascii_digit() && *b != b'0'))
        });
        let has_digit = row.cells.iter().any(|c| {
            let t = c.text.trim();
            t.len() == 1 && t.as_bytes().first().is_some_and(|b| b.is_ascii_digit() && *b != b'0')
        });
        !(all_empty_or_digit && has_digit)
    });

    // Phase 2: Strip leading single-digit prefix from individual cells.
    // Track whether any stripping occurred for Phase 3.
    // Only strip when the remainder clearly looks like form data (currency, dates,
    // codes with dashes/slashes), NOT when it could be a natural phrase like
    // "3 items". ~keep
    for row in table_rows.iter_mut() {
        let mut stripped_any = false;
        for cell in &mut row.cells {
            let text = cell.text.trim();
            if text.len() < 3 {
                continue; // Need at least digit + space + char ~keep
            }
            let bytes = text.as_bytes();
            if bytes[0].is_ascii_digit() && bytes[0] != b'0' && bytes[1] == b' ' {
                let rest = text[2..].trim_start();
                if !rest.is_empty() {
                    let first = rest.as_bytes()[0];
                    // Strip when remainder starts with '$' (currency) or starts
                    // with a digit (date like "Apr 11" won't, but codes like
                    // "12111 - ..." will), or contains '-' or '/' (dates, codes). ~keep
                    let looks_like_data = first == b'$'
                        || first.is_ascii_digit()
                        || (first.is_ascii_alphabetic()
                            && (rest.contains('-') || rest.contains('/') || rest.contains(',')));
                    if looks_like_data {
                        cell.text = rest.to_string();
                        stripped_any = true;
                    }
                }
            }
        }

        // Phase 3: In rows where prefixes were stripped, clear remaining
        // lone single-digit cells (they're the same numbering artifact
        // but had no content after the digit). ~keep
        if stripped_any {
            for cell in &mut row.cells {
                let t = cell.text.trim();
                if t.len() == 1 && t.as_bytes()[0].is_ascii_digit() {
                    cell.text.clear();
                }
            }
        }
    }

    // Phase 4: Clear cells that contain only dashes and/or underscores
    // (decorative line separators in form templates, e.g. "------", "____"). ~keep
    for row in table_rows.iter_mut() {
        for cell in &mut row.cells {
            let t = cell.text.trim();
            if !t.is_empty() && t.chars().all(|c| c == '-' || c == '_') {
                cell.text.clear();
            }
        }
    }

    // Note: rows that become fully empty after Phase 4 (e.g. all-dash rows)
    // are intentionally left in place.  The downstream empty-row splitting
    // logic in detect_tables_from_intersections uses them as table separators. ~keep
}

/// Detect tables from intersections of horizontal and vertical edges, then assign text.
///
/// This implements the universal pipeline used by Tabula, pdfplumber, and PyMuPDF:
/// `Edges -> Snap/Merge -> Intersections -> Cells -> Table Groups`
///
/// GH#1358: A ruled table that is itself rotated on the page (its border
/// lines rotated along with its text) still forms a geometrically correct
/// grid here — `build_grid_from_lines` and `assign_spans_to_intersection_grid`
/// bucket every span by its *physical* page position, which is right
/// regardless of rotation. What was wrong is the *labelling* of that grid:
/// row/column order comes out top-to-bottom / left-to-right in *page* space,
/// which is only the table's own reading order when the table is upright.
/// Fixed as a post-process (`reorient_table_to_rotated_frame`, applied just
/// before this function returns) rather than inline here, so every physical
/// merge/split step above it — `merge_vertically_adjacent_tables`,
/// section-divider splitting — keeps operating on unmodified page-space
/// tables exactly as before; only the caller-visible row/column labelling of
/// the final tables changes for a confidently-rotated table. ~keep
fn detect_tables_from_intersections(
    spans: &[TextSpan],
    lines: &[crate::elements::PathContent],
    config: &TableDetectionConfig,
) -> Vec<Table> {
    let (groups, v_edges, cells_are_intersections) = build_grid_from_lines(lines, config);

    let mut tables = Vec::new();
    for (group_cells, xs, ys, num_cols) in &groups {
        let Some((table_rows, row_cell_span_indices)) =
            assign_spans_to_intersection_grid(group_cells, xs, ys, *num_cols, spans, &v_edges, cells_are_intersections)
        else {
            continue;
        };
        let sub_tables = finalize_intersection_tables(table_rows, &row_cell_span_indices, spans, config, *num_cols);
        tables.extend(sub_tables);
    }

    merge_vertically_adjacent_tables(&mut tables);

    // Post-merge: split tables at section dividers — full-width horizontal
    // lines that indicate separate form sections within a single grid.
    // Use merged H-edges (to detect full-width lines) but only snap (do NOT
    // join) V-edges — joining would merge separate per-section V-segments
    // into a single long edge, hiding the section boundary discontinuity. ~keep
    let (mut h_edges, mut v_edges) = extract_edges(lines);
    snap_and_merge(&mut h_edges);
    snap_edges(&mut v_edges);
    tables = split_tables_at_section_dividers(tables, &h_edges, &v_edges, config);

    // GH#1358 fix, gated by `BUCKET_TABLE_GRID_IN_ROTATED_FRAME`: applied last
    // so every physical/page-space step above (grid building, row splitting,
    // vertical merge, section-divider splitting) is byte-for-byte identical
    // to before this fix. Flip the constant to `false` to A/B against the
    // pre-fix behaviour with a one-line change. ~keep
    if BUCKET_TABLE_GRID_IN_ROTATED_FRAME {
        tables = tables.into_iter().map(reorient_table_to_rotated_frame).collect();
    }

    tables
}

/// Steps 1-4: extract edges, find intersections, build cells, and group them
/// into per-table cell groups with their grid boundaries.
///
/// Returns one `(group_cells, xs, ys, num_cols)` tuple per table group, plus the V edges used
/// to build the cells and whether they came from real crossings (`cells_are_intersections`) —
/// both needed downstream by `band_column_groups` (xberg-io/xberg#1580).
fn build_grid_from_lines(
    lines: &[crate::elements::PathContent],
    config: &TableDetectionConfig,
) -> (Vec<(Vec<IntersectionCell>, Vec<f32>, Vec<f32>, usize)>, Vec<Edge>, bool) {
    let (mut h_edges, mut v_edges) = extract_edges(lines);
    snap_and_merge(&mut h_edges);
    snap_and_merge(&mut v_edges);

    if h_edges.len() < 2 || v_edges.len() < 2 {
        return (Vec::new(), v_edges, false);
    }

    let intersections = find_intersections(&h_edges, &v_edges);

    // Step 2b: When intersections are sparse (< 4), filter out orphan edges
    // that have no plausible counterpart before building the extended grid.
    // This prevents unrelated edges (e.g., decorative lines far from the table)
    // from polluting the grid. ~keep
    if intersections.len() < 4 {
        filter_edges_by_coverage(&mut h_edges, &mut v_edges);
        if h_edges.len() < 2 || v_edges.len() < 2 {
            return (Vec::new(), v_edges, false);
        }
    }

    // `cells_are_intersections` records whether `cells` came from real crossings
    // (`build_cells_from_intersections`) rather than the projected `build_extended_grid_cells`
    // grid. Only real crossings license `band_column_groups` to merge a band's columns
    // (xberg-io/xberg#1580) — an extended grid has no V edge that ever spans any band, so
    // merging there would collapse every row to one cell instead of narrowing a phantom cut.
    let (cells, cells_are_intersections) = if intersections.len() >= 4 {
        let c = build_cells_from_intersections(&intersections, &h_edges, &v_edges);
        if c.is_empty() {
            // Lines exist but don't form real intersection cells — try extended grid. ~keep
            (build_extended_grid_cells(&h_edges, &v_edges), false)
        } else {
            (c, true)
        }
    } else {
        // H and V lines don't physically cross (e.g. Census table: H-lines in
        // header area, V tick marks in data area). Build a virtual grid by
        // projecting all V-line X positions across all H-line Y positions. ~keep
        (build_extended_grid_cells(&h_edges, &v_edges), false)
    };
    if cells.is_empty() {
        return (Vec::new(), v_edges, cells_are_intersections);
    }

    let table_groups = attach_spanning_headers(&cells, group_cells_into_tables(&cells), config.min_table_cells);
    let mut result = Vec::new();
    for group in &table_groups {
        let group_cells: Vec<IntersectionCell> = group.iter().map(|&i| cells[i]).collect();

        let mut xs: Vec<f32> = Vec::new();
        let mut ys: Vec<f32> = Vec::new();
        for c in &group_cells {
            xs.push(c.x1);
            xs.push(c.x2);
            ys.push(c.y1);
            ys.push(c.y2);
        }
        xs.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
        xs.dedup_by(|a, b| (*a - *b).abs() <= SNAP_TOL);
        ys.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
        ys.dedup_by(|a, b| (*a - *b).abs() <= SNAP_TOL);

        let num_cols = if xs.len() >= 2 {
            xs.len() - 1
        } else {
            continue;
        };
        if ys.len() < 2 {
            continue;
        }

        if num_cols < config.min_table_columns || num_cols > config.max_table_columns {
            continue;
        }

        result.push((group_cells, xs, ys, num_cols));
    }
    (result, v_edges, cells_are_intersections)
}

/// Tolerance for judging whether a V edge spans a row band's full height.
///
/// This is a CONTAINMENT tolerance ("does this edge actually run through the band?"), which
/// answers a different question than `SNAP_TOL`, an IDENTITY tolerance ("are these two
/// coordinates the same coordinate?"). Reusing `SNAP_TOL` let an edge fall up to 3pt short of
/// the band at *each* end and still count as spanning it, so a band up to 6pt shorter than the
/// rule beside it was cut at a column position the drawn rule does not justify. The regression
/// test below fails at 3.0 and passes at 1.0 on exactly that shape.
///
/// The phantom boundary is what lets a band of prose inside a drawn frame split into two or
/// more cells instead of staying one wide cell, which is how such a region comes to satisfy the
/// downstream minimums for being accepted as a table at all. GH#1588 reports page text going
/// missing as a result; the precise downstream path is not asserted here, because the two
/// analyses of it disagreed and this constant's own behaviour is provable without settling it.
///
/// Keep this constant independent of `SNAP_TOL` even if their values happen to coincide again
/// in the future — they measure different things and must be free to diverge. ~keep
const BAND_RULE_SPAN_TOL: f32 = 1.0;

/// Group a row band's columns into contiguous runs that no V rule actually divides.
///
/// `build_cells_from_intersections` accepts a cell as soon as its four corners are crossing
/// points, but four corners are not four sides (xberg-io/xberg#1580). When a producer stacks
/// ruled bands around an unruled full-width strip (a section heading), the strip's internal
/// column "boundaries" exist only because the neighbouring bands' V rules happen to terminate
/// on the strip's own H rules — no V rule actually runs through the strip itself. This counts
/// a column boundary at `xs[c + 1]` only when some edge in `v_edges` truly spans the band
/// `[y_lo, y_hi]` (within `BAND_RULE_SPAN_TOL`), and returns the resulting `(first_col,
/// last_col)` groups of adjacent columns no rule separates.
///
/// `cells_are_intersections` gates the whole check off for grids built by
/// `build_extended_grid_cells`: that grid exists precisely because H and V lines never cross,
/// so no V edge would ever be judged as spanning any band, and every band would collapse into
/// a single column. Only grids built from real crossings (`build_cells_from_intersections`)
/// may be narrowed this way; `false` returns one group per column, unchanged from before this
/// function existed.
fn band_column_groups(
    xs: &[f32],
    y_lo: f32,
    y_hi: f32,
    num_cols: usize,
    v_edges: &[Edge],
    cells_are_intersections: bool,
) -> Vec<(usize, usize)> {
    let boundary_is_drawn = |x: f32| -> bool {
        v_edges.iter().any(|edge| {
            (edge.coord - x).abs() <= SNAP_TOL
                && edge.start <= y_lo + BAND_RULE_SPAN_TOL
                && edge.end >= y_hi - BAND_RULE_SPAN_TOL
        })
    };

    let mut groups = Vec::new();
    let mut start = 0usize;
    for c in 0..num_cols {
        if c + 1 == num_cols || !cells_are_intersections || boundary_is_drawn(xs[c + 1]) {
            groups.push((start, c));
            start = c + 1;
        }
    }
    groups
}

/// Assign text spans to grid cells and build table rows with per-cell span
/// indices. Returns `None` when the grid is degenerate.
fn assign_spans_to_intersection_grid(
    group_cells: &[IntersectionCell],
    xs: &[f32],
    ys: &[f32],
    num_cols: usize,
    spans: &[TextSpan],
    v_edges: &[Edge],
    cells_are_intersections: bool,
) -> Option<(Vec<TableRow>, Vec<Vec<Vec<usize>>>)> {
    let num_rows = if ys.len() >= 2 {
        ys.len() - 1
    } else {
        return None;
    };

    let boundary_index = |value: f32, boundaries: &[f32]| {
        boundaries
            .iter()
            .position(|&boundary| (boundary - value).abs() <= SNAP_TOL)
    };

    // A detected cell can cover several grid intervals when neighbouring cells
    // introduce intermediate row or column boundaries. Mark its entire area;
    // each span is still assigned to just one interval below. ~keep
    let mut grid_has_cell = vec![vec![false; num_cols]; num_rows];
    for cell in group_cells {
        if let (Some(left), Some(right), Some(bottom), Some(top)) = (
            boundary_index(cell.x1, xs),
            boundary_index(cell.x2, xs),
            boundary_index(cell.y1, ys),
            boundary_index(cell.y2, ys),
        ) {
            // `boundary_index` proves each boundary exists in `xs`/`ys`, not that the
            // pair is ordered: two boundaries within SNAP_TOL of each other can snap to
            // indices in either order. `arr[bottom..top]` panics unconditionally on
            // bottom > top, so require ordering here and skip malformed geometry instead
            // of aborting the whole extraction. ~keep
            if bottom < top && left < right {
                for row in &mut grid_has_cell[bottom..top] {
                    row[left..right].fill(true);
                }
            }
        }
    }

    // Assign text spans to grid cells based on center point. Prefer the exact
    // interval before applying snap tolerance: expanding every interval makes
    // points near an internal boundary match both neighbours, and `find()`
    // would always bias them into the earlier row or column. ~keep
    let mut grid_spans: Vec<Vec<Vec<usize>>> = vec![vec![Vec::new(); num_cols]; num_rows];
    for (idx, span) in spans.iter().enumerate() {
        let cx = span.bbox.center().x;
        let cy = span.bbox.center().y;
        let col_idx = grid_interval_for_point(cx, xs);
        let row_idx = grid_interval_for_point(cy, ys);
        if let (Some(ci), Some(ri)) = (col_idx, row_idx)
            && grid_has_cell[ri][ci]
        {
            grid_spans[ri][ci].push(idx);
        }
    }

    // Build rows sorted top-to-bottom for the table.
    // In PDF coordinates, higher y = higher on page, so sort rows descending by y. ~keep
    let mut row_order: Vec<usize> = (0..num_rows).collect();
    row_order.sort_by(|&a, &b| crate::utils::safe_float_cmp(ys[b], ys[a]));

    let mut table_rows = Vec::new();
    // Track span indices per cell alongside table_rows for text-based row splitting. ~keep
    let mut row_cell_span_indices: Vec<Vec<Vec<usize>>> = Vec::new();
    for &ri in &row_order {
        let mut row = TableRow::new(false);
        let mut cell_indices_for_row: Vec<Vec<usize>> = Vec::new();
        let groups = band_column_groups(xs, ys[ri], ys[ri + 1], num_cols, v_edges, cells_are_intersections);
        for (first_col, last_col) in groups {
            let colspan = (last_col - first_col + 1) as u32;
            let group_bbox = crate::geometry::Rect::new(
                xs[first_col],
                ys[ri],
                xs[last_col + 1] - xs[first_col],
                ys[ri + 1] - ys[ri],
            );
            let group_has_cell = (first_col..=last_col).any(|ci| grid_has_cell[ri][ci]);
            if !group_has_cell {
                // Still emit empty cell so the group is accounted for. ~keep
                row.cells.push(TableCell {
                    text: String::new(),
                    spans: Vec::new(),
                    colspan,
                    rowspan: 1,
                    mcids: Vec::new(),
                    bbox: Some(group_bbox),
                    is_header: false,
                });
                cell_indices_for_row.push(Vec::new());
                continue;
            }

            // Concatenate columns left-to-right so spans on one visual line stay in reading
            // order: `extract_cell_text` only re-sorts by Y, and each column's own span list
            // is already confined to that column's X range. ~keep
            let group_span_indices: Vec<usize> = (first_col..=last_col)
                .flat_map(|ci| grid_spans[ri][ci].iter().copied())
                .collect();
            let cell_text = extract_cell_text(&group_span_indices, spans);
            let mcids: Vec<u32> = group_span_indices
                .iter()
                .filter_map(|&idx| spans.get(idx).and_then(|s| s.mcid))
                .collect();
            let cell_spans = group_span_indices
                .iter()
                .filter_map(|&idx| spans.get(idx).cloned())
                .collect::<Vec<_>>();

            row.cells.push(TableCell {
                text: cell_text,
                spans: cell_spans,
                colspan,
                rowspan: 1,
                mcids,
                bbox: Some(group_bbox),
                is_header: false,
            });
            cell_indices_for_row.push(group_span_indices);
        }
        table_rows.push(row);
        row_cell_span_indices.push(cell_indices_for_row);
    }

    Some((table_rows, row_cell_span_indices))
}

/// Return the grid interval containing `point`.
///
/// Internal boundaries are half-open and belong to the interval on their
/// right. Snap tolerance is reserved for points just outside the grid, where
/// there is no neighbouring interval to compete for ownership.
fn grid_interval_for_point(point: f32, boundaries: &[f32]) -> Option<usize> {
    let interval_count = boundaries.len().checked_sub(1)?;
    if interval_count == 0 || !point.is_finite() {
        return None;
    }

    if point < boundaries[0] {
        return (boundaries[0] - point <= SNAP_TOL).then_some(0);
    }
    if point > boundaries[interval_count] {
        return (point - boundaries[interval_count] <= SNAP_TOL).then_some(interval_count - 1);
    }

    (0..interval_count).find(|&index| {
        point >= boundaries[index]
            && (point < boundaries[index + 1] || (index + 1 == interval_count && point <= boundaries[index + 1]))
    })
}

/// GH#1358 feature gate: re-orient a fully-built ruled table from the page's
/// upright row/column axes to the table's own rotated frame when its spans
/// confidently agree on a 90/180/270 rotation. Flip to `false` to neutralise
/// the fix for an A/B comparison — with this `false`,
/// `detect_tables_from_intersections` skips `reorient_table_to_rotated_frame`
/// entirely, byte-for-byte identical to before this change. ~keep
const BUCKET_TABLE_GRID_IN_ROTATED_FRAME: bool = true;

/// The dominant rotation quadrant (0/90/180/270) of a table's constituent
/// spans, decided by *strict majority* vote over their own
/// [`TextSpan::rotation_degrees`].
///
/// This mirrors `table_extractor::cell_rotation_degrees`'s treatment of a
/// skewed (non-quadrant) span — it is counted as upright (quadrant 0) rather
/// than snapped to its nearest quadrant, since a skewed span's own frame is
/// ambiguous and upright is the pre-existing, safe default for it. It is
/// intentionally *stricter* than that cell-local helper on the tie-break,
/// though: `cell_rotation_degrees` always picks a plurality winner because a
/// wrong guess there only affects reading order inside one already-placed
/// cell. Getting a whole table's row/column axes wrong is much higher blast
/// radius, so this requires the winning quadrant to hold a strict majority
/// (> 50%) of classified spans. Anything short of that — a genuine tie, a
/// table whose spans disagree on rotation, or a table with no classifiable
/// spans at all (e.g. a purely decorative ruled grid with no text) — returns
/// `0.0`, and the caller leaves the table bucketed upright exactly as before
/// this fix, rather than guessing. ~keep
fn dominant_table_rotation_quadrant(table: &Table) -> f32 {
    let mut counts: [usize; 4] = [0; 4];
    for row in &table.rows {
        for cell in &row.cells {
            for span in &cell.spans {
                let normalized = span.rotation_degrees.rem_euclid(360.0);
                let quadrant = ((normalized / 90.0).round() as usize) % 4;
                if (normalized - (quadrant as f32) * 90.0).abs() < 0.5 {
                    counts[quadrant] += 1;
                } else {
                    counts[0] += 1;
                }
            }
        }
    }
    let total: usize = counts.iter().sum();
    if total == 0 {
        return 0.0;
    }
    let Some((index, &count)) = counts
        .iter()
        .enumerate()
        .max_by_key(|&(index, &count)| (count, std::cmp::Reverse(index)))
    else {
        return 0.0;
    };
    if count * 2 > total { (index as f32) * 90.0 } else { 0.0 }
}

/// Re-orient one finished table from the page's upright row/column axes to
/// the table's own rotated frame, when `dominant_table_rotation_quadrant`
/// finds a confident non-zero rotation.
///
/// The bucketing that already ran (`assign_spans_to_intersection_grid`, plus
/// `finalize_intersection_tables`'s row splitting and this function's own
/// upstream merge/section-divider steps) places every span by its *physical*
/// page position, which is correct regardless of rotation — a point's grid
/// cell doesn't depend on what's drawn there. What is wrong for a genuinely
/// rotated table is the *axis labelling*: `table.rows` is always ordered
/// top-to-bottom by physical Y with each row's cells left-to-right by
/// physical X, which is only the table's own reading order when the table is
/// upright. Applying the fix here, on the finished `Table`, rather than
/// earlier in the pipeline keeps every physical/page-space step upstream
/// (row splitting by text-band Y-clustering, empty-row sub-table splitting,
/// vertical-adjacency merging, section-divider splitting) working in the
/// page's own frame, where those heuristics' assumptions hold; only the
/// caller-visible row/column order of the final table changes.
///
/// `TextSpan::rotation_degrees` is the angle of the composed text-rendering
/// matrix: the baseline advances along `(cos θ, sin θ)` and the next-row
/// direction (perpendicular, "up" reversed) advances along
/// `(sin θ, -cos θ)`. Working through those vectors for each quadrant gives
/// the physical direction of the table's own row-advance and column-advance
/// axes, which is exactly the standard array-rotation transform applied to
/// the `table.rows` grid:
/// - 90°: rotate 90° clockwise — new row = old column (ascending), new
///   column = old row (reversed).
/// - 180°: reverse both row and column order; shape unchanged.
/// - 270°: rotate 90° counter-clockwise — new row = old column (reversed),
///   new column = old row (ascending).
///
/// Cell content (text, spans, bbox, mcids) moves as-is — only its (row, col)
/// slot in the grid changes; a cell's `bbox` remains the true physical
/// location of its content on the page, and `table.bbox` (a physical union
/// over all cells) is untouched. `has_header` is passed through unexamined:
/// this pipeline never sets it (see `assign_spans_to_intersection_grid`),
/// so there is nothing here to re-derive for a header *row* that no longer
/// exists after a transpose.
fn reorient_table_to_rotated_frame(table: Table) -> Table {
    // Only called when `BUCKET_TABLE_GRID_IN_ROTATED_FRAME` is `true` (see
    // `detect_tables_from_intersections`); the gate lives at that call site
    // so the `false` path never allocates or iterates a `Table` here at all. ~keep
    let rotation = dominant_table_rotation_quadrant(&table);
    if rotation == 0.0 {
        return table;
    }
    let Some((rows, col_count)) = rotate_table_rows(&table.rows, rotation) else {
        // Not a rectangular grid (e.g. an upstream merge padded some rows but
        // not others) — reorienting could not be done without guessing at a
        // cell's column, so leave the table bucketed upright rather than
        // risk indexing past a short row. ~keep
        return table;
    };
    Table {
        rows,
        has_header: table.has_header,
        col_count,
        bbox: table.bbox,
    }
}

/// Rotate a rectangular table grid by `rotation_degrees` (90/180/270; `0.0`
/// is a defensive no-op). Returns `None` when `rows` isn't rectangular (some
/// row's cell count differs from the first row's), since the rotation index
/// math below assumes every row has exactly `rows[0].cells.len()` cells.
fn rotate_table_rows(rows: &[TableRow], rotation_degrees: f32) -> Option<(Vec<TableRow>, usize)> {
    let num_orig_rows = rows.len();
    let num_orig_cols = rows.first().map_or(0, |row| row.cells.len());
    if rotation_degrees == 0.0 || num_orig_rows == 0 || num_orig_cols == 0 {
        return Some((rows.to_vec(), num_orig_cols));
    }
    if rows.iter().any(|row| row.cells.len() != num_orig_cols) {
        return None;
    }

    // `rotation_degrees` is always an exact multiple of 90.0 produced by
    // `dominant_table_rotation_quadrant` (`(index as f32) * 90.0`), so float
    // equality against the 90.0 / 270.0 literals below is exact, not
    // approximate. ~keep
    let transpose = rotation_degrees == 90.0 || rotation_degrees == 270.0;
    let (new_num_rows, new_num_cols) = if transpose {
        (num_orig_cols, num_orig_rows)
    } else {
        (num_orig_rows, num_orig_cols)
    };

    let mut new_rows = Vec::with_capacity(new_num_rows);
    for r_new in 0..new_num_rows {
        let mut cells = Vec::with_capacity(new_num_cols);
        for c_new in 0..new_num_cols {
            let (r_old, c_old) = if !transpose {
                // 180: both axes reverse, so the grid keeps its shape. ~keep
                (num_orig_rows - 1 - r_new, num_orig_cols - 1 - c_new)
            } else if rotation_degrees == 90.0 {
                (num_orig_rows - 1 - c_new, r_new)
            } else {
                // 270: transposes the opposite way round from 90. ~keep
                (c_new, num_orig_cols - 1 - r_new)
            };
            cells.push(rows[r_old].cells[c_old].clone());
        }
        new_rows.push(TableRow {
            cells,
            is_header: false,
        });
    }

    Some((new_rows, new_num_cols))
}

/// Row splitting, form-artifact stripping, empty-row splitting, and bbox
/// computation. Produces the final `Table` entries for one table group.
fn finalize_intersection_tables(
    table_rows: Vec<TableRow>,
    row_cell_span_indices: &[Vec<Vec<usize>>],
    spans: &[TextSpan],
    config: &TableDetectionConfig,
    num_cols: usize,
) -> Vec<Table> {
    let mut table_rows = split_rows_by_text_positions(table_rows, row_cell_span_indices, spans, config);

    strip_form_numbering_artifacts(&mut table_rows);

    // Split on completely empty rows (same strategy as cluster-based approach). ~keep
    let mut tables = Vec::new();
    let mut sub_start = 0;
    while sub_start < table_rows.len() {
        let row_is_empty = |r: &TableRow| r.cells.iter().all(|c| c.text.is_empty());
        if row_is_empty(&table_rows[sub_start]) {
            sub_start += 1;
            continue;
        }
        let mut sub_end = sub_start + 1;
        while sub_end < table_rows.len() && !row_is_empty(&table_rows[sub_end]) {
            sub_end += 1;
        }
        let sub_rows: Vec<TableRow> = table_rows[sub_start..sub_end].to_vec();
        let filled: usize = sub_rows
            .iter()
            .flat_map(|r| r.cells.iter())
            .filter(|c| !c.text.is_empty())
            .count();
        if filled >= config.min_table_cells {
            let mut min_x = f32::INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            for r in &sub_rows {
                for c in &r.cells {
                    if let Some(b) = c.bbox {
                        min_x = min_x.min(b.left());
                        min_y = min_y.min(b.top());
                        max_x = max_x.max(b.right());
                        max_y = max_y.max(b.bottom());
                    }
                }
            }
            let sub_bbox = if min_x.is_finite() {
                Some(crate::geometry::Rect::new(min_x, min_y, max_x - min_x, max_y - min_y))
            } else {
                None
            };
            tables.push(Table {
                rows: sub_rows,
                has_header: false,
                col_count: num_cols,
                bbox: sub_bbox,
            });
        }
        sub_start = sub_end;
    }
    tables
}

/// Minimum fraction of the table width that an H-edge must span to qualify
/// as a section divider.
const SECTION_DIVIDER_WIDTH_RATIO: f32 = 0.80;

/// Split each table at interior horizontal edges that span nearly the full
/// table width ("section dividers").  Returns a new list of tables where each
/// original table may have been broken into multiple smaller ones.
fn split_tables_at_section_dividers(
    tables: Vec<Table>,
    h_edges: &[Edge],
    v_edges: &[Edge],
    config: &TableDetectionConfig,
) -> Vec<Table> {
    let mut result = Vec::new();
    for table in tables {
        let parts = split_table_at_section_dividers(table, h_edges, v_edges, config);
        result.extend(parts);
    }
    result
}

/// Split a single table at section divider lines.
///
/// A section divider is a full-width H-edge at a Y position where few or no
/// V-edges cross through — indicating that the vertical lines stop at that
/// boundary (separate bordered sections stacked vertically).
fn split_table_at_section_dividers(
    table: Table,
    h_edges: &[Edge],
    v_edges: &[Edge],
    config: &TableDetectionConfig,
) -> Vec<Table> {
    let Some(bbox) = table.bbox else {
        return vec![table];
    };
    if table.rows.len() < 2 {
        return vec![table];
    }

    let table_width = bbox.right() - bbox.left();
    if table_width <= 0.0 {
        return vec![table];
    }

    // Collect Y-coordinates of H-edges that qualify as section dividers:
    // - span >= SECTION_DIVIDER_WIDTH_RATIO of the table width
    // - fall within the table's vertical range (not at the very top or bottom)
    // - few V-edges cross through that Y (sections have separate vertical grids) ~keep
    let top = bbox.top();
    let bottom = bbox.bottom();
    let margin = 2.0; // pts – ignore edges at the very top/bottom boundary ~keep

    let table_left = bbox.left();
    let table_right = bbox.right();
    let relevant_v_edges: Vec<&Edge> = v_edges
        .iter()
        .filter(|e| e.coord >= table_left - SNAP_TOL && e.coord <= table_right + SNAP_TOL)
        .collect();

    let mut divider_ys: Vec<f32> = Vec::new();
    let mut internal_rule_ys: Vec<f32> = Vec::new();
    for edge in h_edges {
        let overlap_start = edge.start.max(table_left);
        let overlap_end = edge.end.min(table_right);
        let overlap = overlap_end - overlap_start;
        if overlap < table_width * SECTION_DIVIDER_WIDTH_RATIO {
            continue;
        }
        let y = edge.coord;
        if y <= top + margin || y >= bottom - margin {
            continue;
        }
        internal_rule_ys.push(y);
        let cross_margin = SNAP_TOL + 1.0;
        let crossings = relevant_v_edges
            .iter()
            .filter(|v| v.start < y - cross_margin && v.end > y + cross_margin)
            .count();
        // A true section divider has no (or very few) V-edges crossing through.
        // Regular grid row boundaries have many V-edges crossing. ~keep
        if crossings <= 1 {
            divider_ys.push(y);
        }
    }

    if divider_ys.is_empty() {
        return vec![table];
    }

    divider_ys.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
    divider_ys.dedup_by(|a, b| (*a - *b).abs() <= SNAP_TOL);
    internal_rule_ys.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
    internal_rule_ys.dedup_by(|a, b| (*a - *b).abs() <= SNAP_TOL);

    // A V rule stopping at an H rule is section evidence only when the other rules on the
    // table behave differently. Word draws cell borders as one bar per cell with a corner
    // square at each crossing, so every V rule terminates at every H rule and every internal
    // rule qualifies above; splitting there turns a ruled table into one-row fragments that
    // fall under `min_table_cells` and vanish (GH#1656). With a single internal rule there is
    // nothing to compare against, so that case keeps the split. ~keep
    if internal_rule_ys.len() >= 2 && divider_ys.len() == internal_rule_ys.len() {
        return vec![table];
    }

    let row_bounds: Vec<Option<(f32, f32)>> = table
        .rows
        .iter()
        .map(|row| {
            let mut rmin = f32::INFINITY;
            let mut rmax = f32::NEG_INFINITY;
            for c in &row.cells {
                if let Some(b) = c.bbox {
                    rmin = rmin.min(b.top());
                    rmax = rmax.max(b.bottom());
                }
            }
            if rmin.is_finite() { Some((rmin, rmax)) } else { None }
        })
        .collect();

    let mut split_after: Vec<usize> = Vec::new();
    let tol = SNAP_TOL + 2.0; // generous tolerance for matching divider to row boundary ~keep
    for &dy in &divider_ys {
        let mut best_idx: Option<usize> = None;
        let mut best_dist = f32::INFINITY;
        for (i, bounds) in row_bounds.iter().enumerate() {
            if i >= table.rows.len().saturating_sub(1) {
                continue;
            }
            let Some((row_top, row_bot)) = bounds else {
                continue;
            };
            let dist_to_bot = (dy - row_bot).abs();
            let dist_to_top = (dy - row_top).abs();
            let min_dist = dist_to_bot.min(dist_to_top);
            if min_dist <= tol && min_dist < best_dist {
                if dist_to_bot <= dist_to_top {
                    best_idx = Some(i);
                } else if i > 0 {
                    best_idx = Some(i - 1);
                }
                best_dist = min_dist;
            }
        }
        if let Some(idx) = best_idx {
            split_after.push(idx);
        }
    }
    split_after.sort_unstable();
    split_after.dedup();

    if split_after.is_empty() {
        return vec![table];
    }

    let num_cols = table.col_count;
    let all_rows = table.rows;
    let mut sub_tables = Vec::new();
    let mut start = 0;
    for &split_idx in &split_after {
        let end = split_idx + 1;
        if end > start {
            sub_tables.push(&all_rows[start..end]);
        }
        start = end;
    }
    if start < all_rows.len() {
        sub_tables.push(&all_rows[start..]);
    }

    let mut result = Vec::new();
    for sub_rows_slice in sub_tables {
        let sub_rows: Vec<TableRow> = sub_rows_slice.to_vec();
        let filled: usize = sub_rows
            .iter()
            .flat_map(|r| r.cells.iter())
            .filter(|c| !c.text.is_empty())
            .count();
        if filled < config.min_table_cells {
            continue;
        }
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for r in &sub_rows {
            for c in &r.cells {
                if let Some(b) = c.bbox {
                    min_x = min_x.min(b.left());
                    min_y = min_y.min(b.top());
                    max_x = max_x.max(b.right());
                    max_y = max_y.max(b.bottom());
                }
            }
        }
        let sub_bbox = if min_x.is_finite() {
            Some(crate::geometry::Rect::new(min_x, min_y, max_x - min_x, max_y - min_y))
        } else {
            None
        };
        result.push(Table {
            rows: sub_rows,
            has_header: false,
            col_count: num_cols,
            bbox: sub_bbox,
        });
    }

    if result.is_empty() {
        // Don't lose data; return original if all sub-tables were too small. ~keep
        return vec![Table {
            rows: all_rows,
            has_header: false,
            col_count: num_cols,
            bbox: Some(bbox),
        }];
    }

    result
}

/// Maximum vertical gap (in points) between two table bboxes to consider them
/// adjacent and merge them into a single table.
const ADJACENT_TABLE_MERGE_GAP: f32 = 20.0;

/// Maximum allowed column count difference for merging vertically adjacent tables.
/// Tables whose column counts differ by more than this are not merged.
const MERGE_COL_DIFF_TOLERANCE: usize = 2;

/// Merge tables that are vertically adjacent (small gap between bottom of one
/// and top of another) and have similar column counts (difference <= `MERGE_COL_DIFF_TOLERANCE`).
/// When column counts differ, the narrower table's rows are padded with empty cells.
fn merge_vertically_adjacent_tables(tables: &mut Vec<Table>) {
    if tables.len() < 2 {
        return;
    }

    // Sort tables by the top-Y of their bbox (highest Y first in PDF coords). ~keep
    tables.sort_by(|a, b| {
        let ay = a.bbox.map_or(f32::NEG_INFINITY, |bb| bb.top());
        let by = b.bbox.map_or(f32::NEG_INFINITY, |bb| bb.top());
        crate::utils::safe_float_cmp(ay, by)
    });

    let mut merged: Vec<Table> = Vec::new();
    for table in tables.drain(..) {
        let should_merge = merged.last().is_some_and(|prev: &Table| {
            let col_diff = (prev.col_count as isize - table.col_count as isize).unsigned_abs();
            if col_diff > MERGE_COL_DIFF_TOLERANCE {
                return false;
            }
            match (prev.bbox, table.bbox) {
                (Some(pb), Some(tb)) => {
                    let gap = (tb.top() - pb.bottom()).abs().min((pb.top() - tb.bottom()).abs());
                    gap <= ADJACENT_TABLE_MERGE_GAP
                }
                _ => false,
            }
        });

        if should_merge {
            let prev = merged.last_mut().unwrap();
            let target_cols = prev.col_count.max(table.col_count);

            if prev.col_count < target_cols {
                let pad = target_cols - prev.col_count;
                for row in &mut prev.rows {
                    for _ in 0..pad {
                        row.cells.push(TableCell {
                            text: String::new(),
                            spans: Vec::new(),
                            colspan: 1,
                            rowspan: 1,
                            mcids: Vec::new(),
                            bbox: None,
                            is_header: row.is_header,
                        });
                    }
                }
            }

            let mut incoming_rows = table.rows;
            if table.col_count < target_cols {
                let pad = target_cols - table.col_count;
                for row in &mut incoming_rows {
                    for _ in 0..pad {
                        row.cells.push(TableCell {
                            text: String::new(),
                            spans: Vec::new(),
                            colspan: 1,
                            rowspan: 1,
                            mcids: Vec::new(),
                            bbox: None,
                            is_header: row.is_header,
                        });
                    }
                }
            }

            prev.rows.extend(incoming_rows);
            prev.col_count = target_cols;
            if let (Some(pb), Some(tb)) = (prev.bbox, table.bbox) {
                let min_x = pb.left().min(tb.left());
                let min_y = pb.top().min(tb.top());
                let max_x = pb.right().max(tb.right());
                let max_y = pb.bottom().max(tb.bottom());
                prev.bbox = Some(crate::geometry::Rect::new(min_x, min_y, max_x - min_x, max_y - min_y));
            }
            prev.has_header = prev.has_header || table.has_header;
        } else {
            merged.push(table);
        }
    }

    *tables = merged;
}

/// True when the page carries vertical-ruling evidence that should route
/// table detection through the grid pipelines instead of the
/// horizontal-rule-bounded fallback.
///
/// A vertical line counts as RULING evidence only when its drawn bar
/// crosses at least TWO of the horizontal rules: a ruling vertical bounds
/// cells BETWEEN row rules, so it spans from one rule to another (a
/// stroke-width-encoded column bar crosses every row rule of its table).
/// Anything that crosses fewer says nothing about how the page's tables
/// are ruled: an isolated heavy-stroked speck (tick mark, list dash)
/// crosses nothing, and the short dash segments of a decorative dashed
/// BOX border cross at most the one rule their box happens to overlap.
/// Both were disabling the horizontal-rule fallback page-wide, which is
/// precisely what scattered booktabs tables on pages carrying a
/// dash-bordered affiliation box. Rectangles (decomposed into edges) keep
/// their pre-existing veto.
fn has_vertical_ruling_evidence(lines: &[crate::elements::PathContent], h_edges: &[Edge]) -> bool {
    const LINE_AXIS_TOL: f32 = 2.0;
    lines.iter().any(|path| {
        if path.is_horizontal_line(LINE_AXIS_TOL) {
            return false;
        }
        if path.is_vertical_line(LINE_AXIS_TOL) {
            let r = path.rendered_bbox();
            // Count DISTINCT rule levels crossed, not raw edges: one dashed
            // rule is several collinear edges at the same y, and a border
            // "joint" speck sitting on it would otherwise read as crossing
            // two rules while touching only one. ~keep
            let mut crossed_ys: Vec<f32> = h_edges
                .iter()
                .filter(|h| r.y <= h.coord && (r.y + r.height) >= h.coord && (r.x + r.width) >= h.start && r.x <= h.end)
                .map(|h| h.coord)
                .collect();
            crossed_ys.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
            crossed_ys.dedup_by(|a, b| (*a - *b).abs() <= LINE_AXIS_TOL);
            return crossed_ys.len() >= 2;
        }
        path.is_rectangle()
    })
}

/// Group wide H-edges into x-range-coherent FAMILIES, in emission order,
/// so only rules that could bound the same table ever pair up. This fixes
/// two failure shapes in one move: a page of scattered fraction bars never
/// forms a family (no fake table from unrelated equation rules), and a
/// decorative dashed border whose y-position interleaves a real table's
/// rules lands in its own family instead of splitting the table's rules
/// apart in the adjacent-pair walk (which silently dropped the table on
/// real dash-boxed pages).
fn x_coherent_rule_families<'a>(wide: &[&'a Edge]) -> Vec<Vec<&'a Edge>> {
    // A table's boundary rules line up: booktabs top/sub-header/bottom
    // rules share their x-range to within a point (overlap/union ≈ 1.0),
    // while unrelated wide strokes — displayed-equation fraction bars,
    // decorative borders — share at most a common left margin
    // (overlap/union ≲ 0.8 even when x-starts coincide, since widths
    // differ). 0.85 splits the two populations with margin on both sides. ~keep
    const X_COHERENCE: f32 = 0.85;

    let mut uf = UnionFind::new(wide.len());
    for i in 0..wide.len() {
        for j in (i + 1)..wide.len() {
            let (a, b) = (wide[i], wide[j]);
            let overlap = a.end.min(b.end) - a.start.max(b.start);
            let union = a.end.max(b.end) - a.start.min(b.start);
            if union > 0.0 && overlap / union >= X_COHERENCE {
                uf.union(i, j);
            }
        }
    }
    // Keyed by union-find root in a BTreeMap, not a HashMap: HashMap iteration
    // order is randomized per process, and the sort below cannot recover from
    // that on its own. Families are grouped by X-RANGE COHERENCE, so `coord`
    // (the Y of a horizontal rule) is not unique across families — two rule
    // families side by side, or a decorative border sharing a table's top-rule
    // Y, tie on `a[0].coord`. `sort_by` is stable, so a tie leaves the input
    // order intact, and with a HashMap that input order is per-process random:
    // the two families then emit their tables in a different order each run,
    // which reorders the rendered table blocks in the assembled page text.
    // The BTreeMap makes the collected order deterministic; the root tiebreak
    // makes the emitted order a total order over the families, so it depends
    // only on page geometry and edge input order. Each family's `Vec` is pushed
    // in ascending `wide` index order, so `a[0]` is still the family's first
    // rule and non-tied pages sort exactly as before (byte-identical). ~keep
    let mut families: std::collections::BTreeMap<usize, Vec<&'a Edge>> = std::collections::BTreeMap::new();
    for (i, e) in wide.iter().enumerate() {
        families.entry(uf.find(i)).or_default().push(e);
    }
    let mut families: Vec<(usize, Vec<&'a Edge>)> = families.into_iter().collect();
    families.sort_by(|(root_a, a), (root_b, b)| {
        crate::utils::safe_float_cmp(a[0].coord, b[0].coord).then_with(|| root_a.cmp(root_b))
    });
    families.into_iter().map(|(_root, f)| f).collect()
}

/// Spans of one rule-bounded band that pass the containment and
/// letter-spacing guards, or `None` when the band cannot hold a table.
fn rule_band_spans(
    spans: &[TextSpan],
    y_top: f32,
    y_bot: f32,
    x_overlap_start: f32,
    x_overlap_end: f32,
) -> Option<Vec<TextSpan>> {
    let pad = 2.0;
    let mut region_spans: Vec<TextSpan> = Vec::new();
    let mut outside_width = 0.0f32;
    let mut inside_width = 0.0f32;
    for s in spans {
        let cy = s.bbox.center().y;
        if cy > y_top + pad || cy < y_bot - pad {
            continue;
        }
        let cx = s.bbox.center().x;
        if cx >= x_overlap_start - pad && cx <= x_overlap_end + pad {
            inside_width += s.bbox.width.max(0.0);
            region_spans.push(s.clone());
        } else {
            outside_width += s.bbox.width.max(0.0);
        }
    }

    if region_spans.is_empty() {
        return None;
    }

    // A pair of rules bounds a table only if the band's text is
    // horizontally CONTAINED by the rules: a table's boundary
    // rules span the rows they rule, while a fraction bar floats
    // inside surrounding math that continues to its left and
    // right (relation symbols, equation numbers). X-range-coherent
    // vinculums from an aligned multi-step derivation pass the
    // family check above, but the text spilling past the bars
    // gives them away — when a third of the band's text mass lies
    // outside the rules, they don't bound anything. (Division-free
    // so a band of zero-width spans compares 0 > 0 instead of
    // taking a NaN branch.) ~keep
    if outside_width > (outside_width + inside_width) * 0.3 {
        return None;
    }

    // Letter-spaced monospace guard: framed code and console
    // listings (zines, technical reports) draw each glyph on a
    // terminal-font grid, so the band's "words" are mostly single
    // characters whose aligned x positions look exactly like
    // column boundaries — identifiers shatter into single letters
    // (`s e g f a u l t`), addresses into single digits
    // (`0 0 : 1 4`). A real table's cells are words and numbers:
    // one-third single LETTERS or one-half single characters of
    // any kind is spread-out text, not a grid. (The digit
    // threshold is the looser of the two so genuine single-digit
    // table columns, which sit among multi-char label cells, stay
    // under it.) ~keep
    let word_count = region_spans.len();
    let mut single_any = 0usize;
    let mut single_alpha = 0usize;
    for rs in &region_spans {
        let mut chars = rs.text.trim().chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            single_any += 1;
            if c.is_alphabetic() {
                single_alpha += 1;
            }
        }
    }
    if word_count > 0 && (single_alpha * 3 >= word_count || single_any * 2 >= word_count) {
        return None;
    }

    Some(region_spans)
}

fn detect_rule_run(spans: &[TextSpan], run: Option<(f32, f32, f32, f32)>, config: &TableDetectionConfig) -> Vec<Table> {
    run.and_then(|(top, bot, x0, x1)| rule_band_spans(spans, top, bot, x0, x1))
        .map(|region| detect_tables_from_spans(&region, config))
        .unwrap_or_default()
}

fn is_single_text_line(spans: &[TextSpan]) -> bool {
    let line_h = spans.iter().map(|s| s.bbox.height).fold(0.0f32, f32::max);
    let (lo, hi) = spans.iter().fold((f32::MAX, f32::MIN), |(lo, hi), s| {
        let cy = s.bbox.center().y;
        (lo.min(cy), hi.max(cy))
    });
    hi - lo <= line_h * 0.5
}

/// Detect tables in regions bounded by horizontal rules (H-lines) when no vertical
/// lines are present.  Groups H-edges by Y-position to find horizontal table
/// boundaries, then runs text-edge detection on the spans within each bounded
/// region.  This is the "H-lines define regions, text defines columns" hybrid.
fn detect_tables_from_horizontal_rules(
    spans: &[TextSpan],
    h_edges: &[Edge],
    config: &TableDetectionConfig,
) -> Vec<Table> {
    const MIN_RULE_WIDTH: f32 = 100.0;
    const Y_SNAP: f32 = 4.0;

    let wide: Vec<&Edge> = h_edges.iter().filter(|e| (e.end - e.start) >= MIN_RULE_WIDTH).collect();
    if wide.len() < 2 {
        return Vec::new();
    }

    let families = x_coherent_rule_families(&wide);

    let mut tables = Vec::new();

    for family in &families {
        let mut y_coords: Vec<f32> = Vec::new();
        for e in family {
            let merged = y_coords.iter_mut().find(|y| (e.coord - **y).abs() <= Y_SNAP);
            if merged.is_none() {
                y_coords.push(e.coord);
            }
        }
        y_coords.sort_by(|a, b| crate::utils::safe_float_cmp(*b, *a));
        // ~keep

        if y_coords.len() < 2 {
            continue;
        }

        let x_range_for_y = |target_y: f32| -> (f32, f32) {
            let mut min_x = f32::MAX;
            let mut max_x = f32::MIN;
            for e in family {
                if (e.coord - target_y).abs() <= Y_SNAP {
                    if e.start < min_x {
                        min_x = e.start;
                    }
                    if e.end > max_x {
                        max_x = e.end;
                    }
                }
            }
            (min_x, max_x)
        };

        // A band holding one text line is too short for a table on its own:
        // a booktabs header above the midrule, or a table ruled under every
        // row. Consecutive one-line bands join the band below them. ~keep
        let mut run: Option<(f32, f32, f32, f32)> = None;
        for pair in y_coords.windows(2) {
            let y_top = pair[0];
            let y_bot = pair[1];
            let (x1_start, x1_end) = x_range_for_y(y_top);
            let (x2_start, x2_end) = x_range_for_y(y_bot);
            let x_start = x1_start.max(x2_start);
            let x_end = x1_end.min(x2_end);
            if x_end - x_start < MIN_RULE_WIDTH {
                tables.append(&mut detect_rule_run(spans, run.take(), config));
                continue;
            }
            let Some(band) = rule_band_spans(spans, y_top, y_bot, x_start, x_end) else {
                tables.append(&mut detect_rule_run(spans, run.take(), config));
                continue;
            };
            let joined = match run.take() {
                Some((top, _, x0, x1)) => (top, y_bot, x0.min(x_start), x1.max(x_end)),
                None => (y_top, y_bot, x_start, x_end),
            };
            if is_single_text_line(&band) {
                run = Some(joined);
                continue;
            }
            let mut detected = detect_rule_run(spans, Some(joined), config);
            if detected.is_empty() && joined.0 != y_top {
                tables.append(&mut detect_rule_run(
                    spans,
                    Some((joined.0, y_top, joined.2, joined.3)),
                    config,
                ));
                detected = detect_tables_from_spans(&band, config);
            }
            tables.append(&mut detected);
        }
        tables.append(&mut detect_rule_run(spans, run, config));
    }

    // Two families can bracket the same text — a dash-bordered decorative
    // box drawn around (or through) a ruled table gives both the box's
    // border family and the table's rule family a region over the same
    // rows, and each detects its own copy. Keep the TIGHTER detection when
    // two overlap: the looser region also swallows neighbouring lines
    // (footnotes, captions) as junk rows. ~keep
    let mut keep: Vec<bool> = vec![true; tables.len()];
    for i in 0..tables.len() {
        for j in (i + 1)..tables.len() {
            if !keep[i] || !keep[j] {
                continue;
            }
            let (Some(a), Some(b)) = (tables[i].bbox, tables[j].bbox) else {
                continue;
            };
            let ov_w = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
            let ov_h = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
            if ov_w <= 0.0 || ov_h <= 0.0 {
                continue;
            }
            let ov_area = ov_w * ov_h;
            let min_area = (a.width * a.height).min(b.width * b.height);
            if min_area > 0.0 && ov_area / min_area > 0.5 {
                if a.width * a.height >= b.width * b.height {
                    keep[i] = false;
                } else {
                    keep[j] = false;
                }
            }
        }
    }
    let mut it = keep.iter();
    tables.retain(|_| *it.next().unwrap_or(&true));

    tables
}

/// H-rule bounded detection: horizontal lines bound the table regions and
/// text edges define the columns. Pages with vertical ruling are left to the
/// grid detectors.
fn tables_from_horizontal_rules(
    spans: &[TextSpan],
    lines: &[crate::elements::PathContent],
    config: &TableDetectionConfig,
) -> Vec<Table> {
    let (mut h_edges, _) = extract_edges(lines);
    if h_edges.is_empty() || has_vertical_ruling_evidence(lines, &h_edges) {
        return Vec::new();
    }
    snap_and_merge(&mut h_edges);
    let mut tables = detect_tables_from_horizontal_rules(spans, &h_edges, config);
    // A logical table ruled between row *bands* (a rule under the
    // header, or between groups of rows) is emitted here as one
    // fragment per band. Merge vertically-adjacent same-column
    // fragments BEFORE the min-row filter so a table cut into e.g. a
    // [3, 2] pair rejoins into [5] instead of losing its short band to
    // the guard below. Bands sit ~one inter-row pitch apart (the rule
    // stroke + leading), so scale the vertical tolerance to the
    // fragments' median row height rather than the abutting-fragment
    // default. Safety rests on the unchanged column gating in
    // `can_merge_tables` (equal col_count + matched X-start/width): a
    // lone spurious 2-row prose strip has no same-column neighbour, so
    // it stays short and is still dropped — the guard's intent holds. ~keep
    let row_h = median_fragment_row_height(&tables);
    let y_tol = (row_h * 1.5).max(3.0);
    tables = consolidate_adjacent_table_fragments_with_tol(tables, 2.0, y_tol);
    // H-rule bounded detection lacks vertical-line evidence —
    // columns come from text-edge clustering alone (same shape as
    // the text-only fallback in `detect_tables_with_lines`).  Two-row results are
    // virtually always prose that happens to live between
    // decorative rules (annotation underlines, page borders);
    // require three rows of evidence before promoting. ~keep
    tables.retain(|t| t.rows.len() >= 3);
    tables
}

/// Runs H-rule detection on the rules no found table covers, so a rules-only
/// table survives beside a gridded one on the same page.
fn tables_from_leftover_rules(
    spans: &[TextSpan],
    lines: &[crate::elements::PathContent],
    found: &[Table],
    config: &TableDetectionConfig,
) -> Vec<Table> {
    let claimed: Option<Vec<crate::geometry::Rect>> = found.iter().map(|t| t.bbox).collect();
    let Some(claimed) = claimed.filter(|c| !c.is_empty()) else {
        return Vec::new();
    };
    let free = |r: &crate::geometry::Rect| !claimed.iter().any(|c| c.intersects(r));
    let free_lines: Vec<_> = lines.iter().filter(|p| free(&p.rendered_bbox())).cloned().collect();
    tables_from_horizontal_rules(spans, &free_lines, config)
        .into_iter()
        .filter(|t| is_valid_table(t) && t.bbox.as_ref().is_some_and(free))
        .collect()
}

/// Detect tables using vector lines and text spans (main entry point for hybrid detection).
pub fn detect_tables_with_lines(
    spans: &[TextSpan],
    lines: &[crate::elements::PathContent],
    config: &TableDetectionConfig,
) -> Vec<Table> {
    if !config.enabled || spans.is_empty() {
        return Vec::new();
    }
    match (config.horizontal_strategy, config.vertical_strategy) {
        (TableStrategy::Text, TableStrategy::Text) => {
            return detect_tables_from_spans_column_aware(spans, config);
        }
        (TableStrategy::Lines, TableStrategy::Lines) => {
            let mut tables = detect_tables_from_intersections(spans, lines, config);
            if tables.is_empty() {
                for cluster in group_lines_into_clusters(lines, config) {
                    tables.append(&mut detect_tables_in_cluster(spans, lines, &cluster, config));
                }
            }
            let mut tables: Vec<Table> = tables.into_iter().filter(is_valid_table).collect();
            let leftover = tables_from_leftover_rules(spans, lines, &tables, config);
            tables.extend(leftover);
            return tables;
        }
        _ => {}
    }
    // Both / hybrid strategy: try intersection-based first, then cluster, then H-rule bounded,
    // then text fallback. ~keep
    let mut final_tables = detect_tables_from_intersections(spans, lines, config);
    if final_tables.is_empty() {
        let clusters = group_lines_into_clusters(lines, config);
        for cluster in clusters {
            final_tables.append(&mut detect_tables_in_cluster(spans, lines, &cluster, config));
        }
    }
    // When intersection and cluster pipelines found nothing, try H-rule bounded detection:
    // use horizontal lines as table region boundaries with text-edge column detection. ~keep
    if final_tables.is_empty() {
        final_tables = tables_from_horizontal_rules(spans, lines, config);
    }
    // Filter out invalid line-based tables BEFORE overlap checking so that
    // spurious line-based tables don't shadow valid text-based ones. ~keep
    final_tables.retain(is_valid_table);

    // Only allow text-based fallback if BOTH strategies permit it AND the caller
    // explicitly enabled text-only detection (config.text_fallback=true).
    // This prevents extract_text() callers (text_fallback=false) from
    // spuriously running span-column detection alongside ruling-line tables:
    // report-style PDFs with decorative horizontal rules (e.g. swimming results)
    // would otherwise have all their data detected as a text table that renders
    // the page content a second time, causing duplicate extraction.
    // Callers that want text-based table detection (to_markdown, to_html) set
    // config.text_fallback=true explicitly. ~keep
    let allow_text_fallback = config.text_fallback
        && config.horizontal_strategy != TableStrategy::Lines
        && config.vertical_strategy != TableStrategy::Lines;

    if allow_text_fallback {
        let text_candidates = detect_tables_from_spans_column_aware(spans, config);
        for text_table in text_candidates {
            if !passes_spatial_quality_gate(&text_table) {
                continue;
            }
            // Text-only detection (no ruling lines) infers columns from word
            // x-alignment alone — two rows of column-aligned words is the
            // signature of ordinary prose (a title + a wrapped body line),
            // not a table.  Require at least three rows of evidence before
            // promoting a span cluster to a table. ~keep
            if text_table.rows.len() < 3 {
                continue;
            }
            if let Some(text_bbox) = text_table.bbox {
                let overlaps = final_tables.iter().any(|t| {
                    if let Some(line_bbox) = t.bbox {
                        line_bbox.intersects(&text_bbox)
                            || line_bbox.contains_rect(&text_bbox)
                            || text_bbox.contains_rect(&line_bbox)
                    } else {
                        false
                    }
                });
                if !overlaps {
                    final_tables.push(text_table);
                }
            }
        }
    }
    final_tables
}

fn grid_to_table(
    grid: &GridStructure,
    spans: &[TextSpan],
    visual_merge_info: Option<Vec<Vec<CellMergeInfo>>>,
) -> Table {
    let num_rows = grid.cells.len();
    let num_cols = grid.columns.len();
    let merge_info = visual_merge_info.unwrap_or_else(|| detect_merged_cells(grid, spans));
    let header_row_idx = detect_header_row(grid, spans);
    let mut table_rows = Vec::new();
    for (row_idx, row) in grid.cells.iter().enumerate() {
        let is_header = header_row_idx == Some(row_idx);
        let mut table_row = TableRow::new(is_header);
        for (col_idx, cell_span_indices) in row.iter().enumerate() {
            let mi = &merge_info[row_idx][col_idx];
            if mi.covered {
                continue;
            }
            let cell_text = extract_cell_text(cell_span_indices, spans);
            let mut cell_bbox = None;
            if !cell_span_indices.is_empty() {
                let mut b = spans[cell_span_indices[0]].bbox;
                for &idx in &cell_span_indices[1..] {
                    b = b.union(&spans[idx].bbox);
                }
                cell_bbox = Some(b);
            }
            let mcids = cell_span_indices
                .iter()
                .filter_map(|&idx| spans.get(idx).and_then(|s| s.mcid))
                .collect::<Vec<_>>();
            let cell_spans = cell_span_indices
                .iter()
                .filter_map(|&idx| spans.get(idx).cloned())
                .collect::<Vec<_>>();

            table_row.cells.push(TableCell {
                text: cell_text,
                spans: cell_spans,
                colspan: mi.colspan.min((num_cols - col_idx) as u32),
                rowspan: mi.rowspan.min((num_rows - row_idx) as u32),
                mcids,
                bbox: cell_bbox,
                is_header,
            });
        }
        table_rows.push(table_row);
    }
    let all_span_indices: Vec<usize> = grid
        .cells
        .iter()
        .flat_map(|row| row.iter().flat_map(|cell| cell.iter().copied()))
        .collect();
    let mut bbox = None;
    if !all_span_indices.is_empty() {
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for &idx in &all_span_indices {
            if let Some(s) = spans.get(idx) {
                min_x = min_x.min(s.bbox.x);
                min_y = min_y.min(s.bbox.y);
                max_x = max_x.max(s.bbox.x + s.bbox.width);
                max_y = max_y.max(s.bbox.y + s.bbox.height);
            }
        }
        bbox = Some(crate::geometry::Rect::new(min_x, min_y, max_x - min_x, max_y - min_y));
    }
    Table {
        rows: table_rows,
        has_header: header_row_idx.is_some(),
        col_count: num_cols,
        bbox,
    }
}

fn extract_cell_text(cell_span_indices: &[usize], spans: &[TextSpan]) -> String {
    if cell_span_indices.is_empty() {
        return String::new();
    }
    // Keep the span reference (not just text) so we can decide spacing based
    // on the geometric gap and CJK/fullwidth-operator boundary state, exactly
    // like the inline-flow path does in pipeline/converters/mod.rs.  Without
    // this, the previous `line.join(" ")` was unconditionally inserting a
    // space between every adjacent span on the same row, splitting compound
    // tokens like `40000≤Q＜55000` into `40000≤Q ＜55000` and dropping word-F1
    // for table-heavy CJK documents. ~keep
    let mut span_entries: Vec<(f32, &TextSpan, String)> = cell_span_indices
        .iter()
        .filter_map(|&idx| spans.get(idx).map(|s| (s.bbox.center().y, s, span_text_for_cell(s))))
        .collect();
    if span_entries.is_empty() {
        return String::new();
    }
    if span_entries.len() == 1 {
        // Trim the same source-space prefix the multi-span path trims (GH#1948):
        // a single-word cell has no preceding word, so any leading space is an
        // edge artifact, not a word separator.
        return span_entries.remove(0).2.trim().to_string();
    }
    span_entries.sort_by(|a, b| crate::utils::safe_float_cmp(b.0, a.0));

    // Group into rows by y proximity, then within a row decide separator per
    // pair of spans using the same gap/CJK rules as inline text assembly. ~keep
    let mut lines: Vec<Vec<(&TextSpan, String)>> = Vec::new();
    let mut current_line: Vec<(&TextSpan, String)> = vec![(span_entries[0].1, span_entries[0].2.clone())];
    let mut current_y = span_entries[0].0;
    for (y, span, text) in &span_entries[1..] {
        if (current_y - y).abs() <= 2.0 {
            current_line.push((span, text.clone()));
        } else {
            lines.push(current_line);
            current_line = vec![(span, text.clone())];
            current_y = *y;
        }
    }
    lines.push(current_line);

    lines
        .iter()
        .map(|line| {
            let mut out = String::new();
            for (i, (span, text)) in line.iter().enumerate() {
                if i > 0 {
                    let (prev_span, _) = line[i - 1];
                    let separator = cell_span_separator(prev_span, span);
                    out.push_str(separator);
                }
                out.push_str(text);
            }
            // A word can carry a leading space from `extract_table_word_spans`,
            // which re-emits the source space between two words whose boxes abut
            // (GH#1948). When such a word opens a cell or a wrapped line, that
            // space is at an edge, not between two words; drop it here. Cell text
            // never has meaningful leading/trailing whitespace. ~keep
            out.trim().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Decide what (if any) separator to put between two spans within the same
/// table cell row.  Mirrors the inline-flow has_horizontal_gap logic from
/// pipeline/converters/mod.rs: insert a space only when there is a real
/// horizontal gap that exceeds the inter-glyph kerning floor AND the
/// boundary is not a CJK ↔ CJK / CJK ↔ fullwidth-operator pair.
fn cell_span_separator(prev: &TextSpan, current: &TextSpan) -> &'static str {
    if prev.text.ends_with(' ') || current.text.starts_with(' ') {
        return "";
    }

    // Locale-independent numeric punctuation stays attached when the PDF
    // splits a number immediately before its decimal/grouping separator. The
    // digit-only suffix avoids swallowing sentence punctuation. ~keep
    let prev_ends_in_digit = prev.text.trim_end().chars().next_back().is_some_and(|c| c.is_numeric());
    let mut current_chars = current.text.trim_start().chars();
    let starts_numeric_group = matches!(current_chars.next(), Some('.' | ',' | '\u{066B}' | '\u{066C}'))
        && current_chars.clone().next().is_some()
        && current_chars.all(|c| c.is_numeric());
    if prev_ends_in_digit && starts_numeric_group {
        return "";
    }

    let prev_end_x = prev.bbox.x + prev.bbox.width;
    let gap = current.bbox.x - prev_end_x;
    let font_size = prev.font_size.max(current.font_size).max(1.0);

    let space_threshold = font_size * 0.15;
    if gap <= space_threshold {
        return "";
    }
    // No upper bound: a very large gap (≥ 5 em) used to be treated as a
    // column boundary and yield no separator, but the caller concatenates
    // span text when this returns "" — so tokens like `3.80%` and `4.41%`
    // were rendered as `3.80%4.41%` on wide rate tables.  Mirroring the
    // inline-flow rule (`pipeline::converters::has_horizontal_gap`), any
    // gap above the inter-glyph threshold now gets at least a single
    // space. ~keep

    // CJK / fullwidth-operator suppression — same rule as
    // pipeline::converters::has_horizontal_gap.  pdftotext keeps an
    // ideograph + adjacent fullwidth/math operator without a separator. ~keep
    let is_cjk = |c: char| {
        matches!(
            c as u32,
            0x3040..=0x309F
            | 0x30A0..=0x30FF
            | 0x4E00..=0x9FFF
            | 0xAC00..=0xD7AF
            | 0x3400..=0x4DBF
            | 0x20000..=0x2A6DF
        )
    };
    let is_fw_op = |c: char| {
        matches!(
            c as u32,
            0xFF0B | 0xFF0D | 0xFF1A | 0xFF1B
            | 0xFF1C..=0xFF1E
            | 0x2260 | 0x2248
            | 0x2264..=0x2265
            | 0x00B5 | 0x03BC
            | 0x00B1 | 0x00D7 | 0x00F7
        )
    };
    let prev_tail = prev.text.chars().next_back();
    let curr_head = current.text.chars().next();
    if let (Some(p), Some(c)) = (prev_tail, curr_head) {
        let p_cjk = is_cjk(p);
        let c_cjk = is_cjk(c);
        if (p_cjk || is_fw_op(p)) && (c_cjk || is_fw_op(c)) && (p_cjk || c_cjk) {
            return "";
        }
    }

    " "
}

/// Consolidate vertically-adjacent tables that share an identical column
/// structure into a single multi-row table.
///
/// Root cause: when a logical multi-row table is drawn
/// with a horizontal ruling line between every pair of rows (rather than
/// only at the top and bottom), the line-based detector emits one Table
/// per row strip. Each fragment is a 1- or 2-row table that fails
/// `is_real_grid()` (which requires ≥2 rows) and gets dropped, after
/// which the cells fall through to the paragraph flow with column-based
/// reading order — producing orphan `<p>40000≤Q</p>` / `<p>＜55000</p>`
/// pairs instead of `<table><td>40000≤Q＜55000</td></tr></table>`.
///
/// Two fragments are merge-candidates when:
///   * both have a `bbox`
///   * X start matches within `X_TOLERANCE`
///   * width matches within `X_TOLERANCE`
///   * column counts are equal
///   * the lower fragment's top edge (`bbox.y + bbox.height`) is within
///     `Y_TOLERANCE` of the upper fragment's bottom edge (`bbox.y`)
///
/// Sort tables top-down (PDF y-up: largest top-Y first) and merge runs
/// of consecutive fragments that satisfy the criteria. The merged table
/// preserves the union of all rows and a bbox spanning both fragments.
pub fn consolidate_adjacent_table_fragments(tables: Vec<Table>) -> Vec<Table> {
    consolidate_adjacent_table_fragments_with_tol(tables, 2.0, 3.0)
}

/// Median per-row height across table fragments, estimated as each
/// fragment's bbox height divided by its row count. Used to scale the
/// vertical merge tolerance for rule-split bands. Returns 0.0 when no
/// fragment carries usable geometry.
fn median_fragment_row_height(tables: &[Table]) -> f32 {
    let mut heights: Vec<f32> = tables
        .iter()
        .filter_map(|t| {
            let b = t.bbox?;
            let n = t.rows.len();
            if n == 0 || b.height <= 0.0 {
                None
            } else {
                Some(b.height / n as f32)
            }
        })
        .collect();
    if heights.is_empty() {
        return 0.0;
    }
    heights.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
    heights[heights.len() / 2]
}

/// Like [`consolidate_adjacent_table_fragments`] but with a caller-chosen
/// vertical merge tolerance. The default 3.0 pt tolerance assumes fragments
/// abut (a ruling line between every row leaves ~0 gap). The H-rule-bounded
/// detector, however, emits one fragment per rule-delimited band, so two
/// bands of the SAME logical table are separated by a full inter-row pitch
/// (rule stroke + leading) — often ~10-15 pt. A larger `y_tol` lets those
/// bands rejoin. Safety rests on the unchanged column gating in
/// `can_merge_tables` (equal col_count + X-start ≤ x_tol + width ≤ x_tol):
/// two genuinely distinct tables sharing all three within one row-height are
/// vanishingly rare, whereas bands of one ruled table match them exactly.
pub fn consolidate_adjacent_table_fragments_with_tol(
    tables: Vec<Table>,
    x_tolerance: f32,
    y_tolerance: f32,
) -> Vec<Table> {
    if tables.len() < 2 {
        return tables;
    }

    // Sort by top-Y descending (top of page first in PDF y-up coordinates). ~keep
    let mut sorted = tables;
    sorted.sort_by(|a, b| {
        let a_top = a.bbox.map(|b| b.y + b.height).unwrap_or(f32::NEG_INFINITY);
        let b_top = b.bbox.map(|b| b.y + b.height).unwrap_or(f32::NEG_INFINITY);
        crate::utils::safe_float_cmp(b_top, a_top)
    });

    let mut consolidated: Vec<Table> = Vec::with_capacity(sorted.len());
    for table in sorted {
        let merge_into_last = consolidated
            .last()
            .map(|last| can_merge_tables(last, &table, x_tolerance, y_tolerance))
            .unwrap_or(false);
        if merge_into_last {
            // Safety: merge_into_last is only true when consolidated.last()
            // returned Some, so last_mut() must also return Some. ~keep
            if let Some(last) = consolidated.last_mut() {
                merge_table_into(last, table);
            }
        } else {
            consolidated.push(table);
        }
    }
    consolidated
}

fn can_merge_tables(upper: &Table, lower: &Table, x_tol: f32, y_tol: f32) -> bool {
    let (Some(u_bbox), Some(l_bbox)) = (upper.bbox, lower.bbox) else {
        return false;
    };
    if upper.col_count != lower.col_count || upper.col_count == 0 {
        return false;
    }
    if (u_bbox.x - l_bbox.x).abs() > x_tol {
        return false;
    }
    if (u_bbox.width - l_bbox.width).abs() > x_tol {
        return false;
    }
    // upper sits ABOVE lower in PDF y-up: upper.bbox.y is the BOTTOM of
    // upper, lower.bbox.y + lower.bbox.height is the TOP of lower.
    // For them to be vertically adjacent, the upper.bottom must be close
    // to the lower.top.  We allow a small NEGATIVE gap (overlap) up to
    // half the smaller table's height — the line-based detector
    // occasionally produces bboxes that overhang the adjacent table by a
    // few points when ruling-rule strokes have non-zero thickness or
    // include the line's drawn extent above/below the baseline.  Real
    // distinct tables almost always have a meaningful positive gap. ~keep
    let upper_bottom = u_bbox.y;
    let lower_top = l_bbox.y + l_bbox.height;
    let gap = upper_bottom - lower_top;
    if gap > y_tol {
        return false;
    }
    let min_height = u_bbox.height.min(l_bbox.height);
    if -gap > min_height * 0.5 {
        return false;
    }
    true
}

fn merge_table_into(upper: &mut Table, lower: Table) {
    if let (Some(ub), Some(lb)) = (upper.bbox, lower.bbox) {
        let new_y = ub.y.min(lb.y);
        let new_top = (ub.y + ub.height).max(lb.y + lb.height);
        let new_x = ub.x.min(lb.x);
        let new_right = (ub.x + ub.width).max(lb.x + lb.width);
        upper.bbox = Some(crate::geometry::Rect {
            x: new_x,
            y: new_y,
            width: new_right - new_x,
            height: new_top - new_y,
        });
    }
    upper.rows.extend(lower.rows);
}

fn detect_merged_cells(grid: &GridStructure, spans: &[TextSpan]) -> Vec<Vec<CellMergeInfo>> {
    let num_rows = grid.cells.len();
    let num_cols = grid.columns.len();
    let mut merge_info: Vec<Vec<CellMergeInfo>> = (0..num_rows)
        .map(|_| {
            (0..num_cols)
                .map(|_| CellMergeInfo {
                    colspan: 1,
                    rowspan: 1,
                    covered: false,
                })
                .collect()
        })
        .collect();
    for row_idx in 0..num_rows {
        for col_idx in 0..num_cols {
            if grid.cells[row_idx][col_idx].is_empty() {
                continue;
            }
            let cell_right = grid.cells[row_idx][col_idx]
                .iter()
                .filter_map(|&idx| spans.get(idx).map(|s| s.bbox.right()))
                .fold(f32::NEG_INFINITY, f32::max);
            if cell_right == f32::NEG_INFINITY {
                continue;
            }
            let mut extra_cols = 0u32;
            for next_col in (col_idx + 1)..num_cols {
                if !grid.cells[row_idx][next_col].is_empty() {
                    break;
                }
                if cell_right > grid.columns[next_col].x_center {
                    extra_cols += 1;
                } else {
                    break;
                }
            }
            if extra_cols > 0 {
                merge_info[row_idx][col_idx].colspan = 1 + extra_cols;
                for c in 1..=(extra_cols as usize) {
                    merge_info[row_idx][col_idx + c].covered = true;
                }
            }
        }
    }
    for col_idx in 0..num_cols {
        for row_idx in 0..num_rows {
            if grid.cells[row_idx][col_idx].is_empty() || merge_info[row_idx][col_idx].covered {
                continue;
            }
            let cell_bottom = grid.cells[row_idx][col_idx]
                .iter()
                .filter_map(|&idx| spans.get(idx).map(|s| s.bbox.bottom()))
                .fold(f32::INFINITY, f32::min);
            if cell_bottom == f32::INFINITY {
                continue;
            }
            let mut extra_rows = 0u32;
            for next_row in (row_idx + 1)..num_rows {
                if !grid.cells[next_row][col_idx].is_empty() {
                    break;
                }
                if cell_bottom < grid.rows[next_row].y_center {
                    extra_rows += 1;
                } else {
                    break;
                }
            }
            if extra_rows > 0 {
                merge_info[row_idx][col_idx].rowspan = 1 + extra_rows;
                for r in 1..=(extra_rows as usize) {
                    merge_info[row_idx + r][col_idx].covered = true;
                }
            }
        }
    }
    merge_info
}

fn detect_header_row(grid: &GridStructure, spans: &[TextSpan]) -> Option<usize> {
    if grid.cells.len() < 2 {
        return None;
    }
    let first_row_spans: Vec<&TextSpan> = grid.cells[0]
        .iter()
        .flat_map(|cell| cell.iter().filter_map(|&idx| spans.get(idx)))
        .collect();
    if first_row_spans.is_empty() {
        return None;
    }
    let data_row_spans: Vec<&TextSpan> = grid.cells[1..]
        .iter()
        .flat_map(|row| {
            row.iter()
                .flat_map(|cell| cell.iter().filter_map(|&idx| spans.get(idx)))
        })
        .collect();
    if data_row_spans.is_empty() {
        return None;
    }
    let first_row_bold_ratio =
        first_row_spans.iter().filter(|s| s.font_weight.is_bold()).count() as f32 / first_row_spans.len() as f32;
    let data_bold_ratio =
        data_row_spans.iter().filter(|s| s.font_weight.is_bold()).count() as f32 / data_row_spans.len() as f32;
    if first_row_bold_ratio > 0.5 && data_bold_ratio < 0.3 {
        return Some(0);
    }
    let first_row_avg_size: f32 =
        first_row_spans.iter().map(|s| s.font_size).sum::<f32>() / first_row_spans.len() as f32;
    let data_avg_size: f32 = data_row_spans.iter().map(|s| s.font_size).sum::<f32>() / data_row_spans.len() as f32;
    if first_row_avg_size > data_avg_size + 1.5 {
        return Some(0);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::layout::text_block::{Color, FontWeight};

    #[test]
    fn row_spanning_cell_ignores_a_neighbours_one_sided_crossing() {
        // Left-hand description is split at y=20; its rule stops at x=40.
        // The alignment cell to its right is a single box from y=0 to y=40. ~keep
        let horizontal = vec![
            Edge {
                coord: 0.,
                start: 0.,
                end: 80.,
            },
            Edge {
                coord: 20.,
                start: 0.,
                end: 40.,
            },
            Edge {
                coord: 40.,
                start: 0.,
                end: 80.,
            },
        ];
        let vertical = vec![
            Edge {
                coord: 0.,
                start: 0.,
                end: 40.,
            },
            Edge {
                coord: 40.,
                start: 0.,
                end: 40.,
            },
            Edge {
                coord: 80.,
                start: 0.,
                end: 40.,
            },
        ];
        let points = find_intersections(&horizontal, &vertical);
        let cells = build_cells_from_intersections(&points, &horizontal, &vertical);
        assert!(
            cells
                .iter()
                .any(|c| c.x1 == 40. && c.x2 == 80. && c.y1 == 0. && c.y2 == 40.),
            "{cells:?}"
        );
        assert_eq!(cells.len(), 3);
    }

    #[test]
    fn later_closing_corners_do_not_bridge_separate_boxes() {
        let horizontal = vec![
            Edge {
                coord: 0.,
                start: 0.,
                end: 20.,
            },
            Edge {
                coord: 10.,
                start: 0.,
                end: 20.,
            },
            Edge {
                coord: 40.,
                start: 0.,
                end: 20.,
            },
            Edge {
                coord: 0.,
                start: 40.,
                end: 60.,
            },
            Edge {
                coord: 40.,
                start: 40.,
                end: 60.,
            },
        ];
        let vertical: Vec<_> = [0., 20., 40., 60.]
            .into_iter()
            .map(|coord| Edge {
                coord,
                start: 0.,
                end: 40.,
            })
            .collect();
        let points = find_intersections(&horizontal, &vertical);
        let cells = build_cells_from_intersections(&points, &horizontal, &vertical);
        assert!(
            !cells
                .iter()
                .any(|cell| cell.x1 == 20. && cell.x2 == 40. && cell.y1 == 0.),
            "{cells:?}"
        );
    }

    #[test]
    fn cell_search_does_not_skip_a_rule_entering_its_interior() {
        let horizontal = vec![
            Edge {
                coord: 0.,
                start: 0.,
                end: 80.,
            },
            Edge {
                coord: 20.,
                start: 0.,
                end: 30.,
            },
            Edge {
                coord: 40.,
                start: 0.,
                end: 80.,
            },
        ];
        let vertical = vec![
            Edge {
                coord: 0.,
                start: 0.,
                end: 40.,
            },
            Edge {
                coord: 80.,
                start: 0.,
                end: 40.,
            },
        ];
        let points = find_intersections(&horizontal, &vertical);
        let cells = build_cells_from_intersections(&points, &horizontal, &vertical);
        assert!(
            !cells
                .iter()
                .any(|cell| cell.x1 == 0. && cell.y1 == 0. && cell.x2 == 80. && cell.y2 == 40.),
            "{cells:?}"
        );
    }

    #[test]
    fn farther_corners_without_a_divider_ending_below_are_not_header_cells() {
        let points = [
            Intersection { x: 0., y: 0. },
            Intersection { x: 40., y: 0. },
            Intersection { x: 80., y: 0. },
            Intersection { x: 0., y: 20. },
            Intersection { x: 80., y: 20. },
        ];
        let h = [
            Edge {
                coord: 0.,
                start: 0.,
                end: 80.,
            },
            Edge {
                coord: 20.,
                start: 0.,
                end: 80.,
            },
        ];
        let v = [
            Edge {
                coord: 0.,
                start: 0.,
                end: 20.,
            },
            Edge {
                coord: 40.,
                start: 0.,
                end: 10.,
            },
            Edge {
                coord: 80.,
                start: 0.,
                end: 20.,
            },
        ];
        assert!(build_cells_from_intersections(&points, &h, &v).is_empty());
    }

    #[test]
    fn a_single_ruled_header_row_does_not_claim_the_unruled_body() {
        let mut cells: Vec<_> = (0..6)
            .map(|column| IntersectionCell {
                x1: column as f32 * 20.,
                y1: 0.,
                x2: (column + 1) as f32 * 20.,
                y2: 20.,
            })
            .collect();
        cells.push(IntersectionCell {
            x1: 0.,
            y1: 20.,
            x2: 120.,
            y2: 40.,
        });
        let groups = attach_spanning_headers(&cells, group_cells_into_tables(&cells), 6);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn sparse_header_boxes_do_not_form_a_new_table() {
        let cells = [
            IntersectionCell {
                x1: 0.,
                y1: 20.,
                x2: 80.,
                y2: 40.,
            },
            IntersectionCell {
                x1: 0.,
                y1: 0.,
                x2: 40.,
                y2: 20.,
            },
            IntersectionCell {
                x1: 40.,
                y1: 0.,
                x2: 80.,
                y2: 20.,
            },
        ];
        let groups = attach_spanning_headers(&cells, group_cells_into_tables(&cells), 6);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn inset_fills_do_not_add_columns_inside_a_ruled_band() {
        use crate::elements::PathContent;
        let mut fill = PathContent::rect(4., 0.5, 72., 19.);
        fill.stroke_color = None;
        fill.fill_color = Some(Color::black());
        let rules = vec![
            PathContent::line(0., 0., 80., 0.),
            PathContent::line(0., 20., 80., 20.),
            PathContent::line(0., 0., 0., 20.),
            PathContent::line(80., 0., 80., 20.),
        ];
        let mut outer = PathContent::rect(0., 0., 80., 20.);
        outer.stroke_color = None;
        outer.fill_color = Some(Color::black());
        let mut shaded = rules.clone();
        shaded.extend([outer, fill.clone()]);
        let (mut h, mut v) = extract_edges(&shaded);
        snap_and_merge(&mut h);
        snap_and_merge(&mut v);
        assert_eq!(h.len(), 2);
        assert_eq!(v.len(), 2);
        // A fill without a matching outer background is not assumed decorative.
        let mut single_fill = rules.clone();
        single_fill.push(fill.clone());
        let (h, v) = extract_edges(&single_fill);
        assert_eq!(h.len(), 4);
        assert_eq!(v.len(), 4);
        // Fills remain evidence when the sides of the row are not ruled.
        let (h, v) = extract_edges(&[rules[0].clone(), rules[1].clone(), fill.clone()]);
        assert_eq!(h.len(), 4);
        assert_eq!(v.len(), 2);
        // A stroked inner box is real geometry, even inside a ruled band.
        fill.stroke_color = Some(Color::black());
        let mut boxed = rules;
        boxed.push(fill);
        let (h, v) = extract_edges(&boxed);
        assert_eq!(h.len(), 4);
        assert_eq!(v.len(), 4);
    }

    #[test]
    fn spanning_header_stays_in_body_group() {
        let cells = [
            IntersectionCell {
                x1: 0.,
                y1: 40.,
                x2: 80.,
                y2: 60.,
            },
            IntersectionCell {
                x1: 0.,
                y1: 0.,
                x2: 40.,
                y2: 20.,
            },
            IntersectionCell {
                x1: 40.,
                y1: 0.,
                x2: 80.,
                y2: 20.,
            },
            IntersectionCell {
                x1: 0.,
                y1: 20.,
                x2: 40.,
                y2: 40.,
            },
            IntersectionCell {
                x1: 40.,
                y1: 20.,
                x2: 80.,
                y2: 40.,
            },
        ];
        let groups = attach_spanning_headers(&cells, group_cells_into_tables(&cells), 2);
        assert_eq!(groups.len(), 1, "{groups:?}");
    }

    #[test]
    fn unrelated_cells_stay_separate_after_header_attachment() {
        // A full two-row body clears the attachment guards. Using isolated cells
        // here would skip the new attachment logic and only test old grouping.
        let body: Vec<_> = (0..2)
            .flat_map(|row| {
                (0..3).map(move |column| IntersectionCell {
                    x1: column as f32 * 40.,
                    y1: row as f32 * 20.,
                    x2: (column + 1) as f32 * 40.,
                    y2: (row + 1) as f32 * 20.,
                })
            })
            .collect();
        for (case, candidate) in [
            (
                "gap",
                IntersectionCell {
                    x1: 0.,
                    y1: 50.,
                    x2: 120.,
                    y2: 70.,
                },
            ),
            (
                "corner",
                IntersectionCell {
                    x1: 120.,
                    y1: 40.,
                    x2: 160.,
                    y2: 60.,
                },
            ),
        ] {
            let mut cells = body.clone();
            cells.push(candidate);
            let groups = attach_spanning_headers(&cells, group_cells_into_tables(&cells), 6);
            assert_eq!(groups, vec![vec![0, 1, 2, 3, 4, 5], vec![6]], "{case}");
        }
    }

    #[test]
    fn header_cell_search_skips_nonclosing_inner_corner() {
        let points = [
            Intersection { x: 0., y: 0. },
            Intersection { x: 40., y: 0. },
            Intersection { x: 80., y: 0. },
            Intersection { x: 0., y: 20. },
            Intersection { x: 80., y: 20. },
        ];
        let h = [
            Edge {
                coord: 0.,
                start: 0.,
                end: 80.,
            },
            Edge {
                coord: 20.,
                start: 0.,
                end: 80.,
            },
        ];
        let v = [
            Edge {
                coord: 0.,
                start: 0.,
                end: 20.,
            },
            Edge {
                coord: 40.,
                start: -20.,
                end: 0.,
            },
            Edge {
                coord: 80.,
                start: 0.,
                end: 20.,
            },
        ];
        let cells = build_cells_from_intersections(&points, &h, &v);
        assert!(
            cells
                .iter()
                .any(|c| c.x1 == 0. && c.x2 == 80. && c.y1 == 0. && c.y2 == 20.),
            "{cells:?}"
        );
    }

    #[test]
    fn rowspan_preserves_label_once_and_rejects_outside_text() {
        let cells = [
            IntersectionCell {
                x1: 0.,
                y1: 0.,
                x2: 40.,
                y2: 40.,
            },
            IntersectionCell {
                x1: 40.,
                y1: 0.,
                x2: 80.,
                y2: 20.,
            },
            IntersectionCell {
                x1: 40.,
                y1: 20.,
                x2: 80.,
                y2: 40.,
            },
        ];
        let spans = [
            create_test_span("Label", 5., 28., 20., 6.),
            create_test_span("Outside", 5., 50., 20., 6.),
        ];
        let edges = [Edge {
            coord: 40.,
            start: 0.,
            end: 40.,
        }];
        let (rows, _) =
            assign_spans_to_intersection_grid(&cells, &[0., 40., 80.], &[0., 20., 40.], 2, &spans, &edges, true)
                .unwrap();
        let texts: Vec<_> = rows.iter().flat_map(|r| &r.cells).map(|c| c.text.as_str()).collect();
        assert_eq!(texts.iter().filter(|&&t| t == "Label").count(), 1, "{texts:?}");
        assert!(!texts.iter().any(|t| t.contains("Outside")));
    }

    #[test]
    fn colspan_preserves_off_center_header_once() {
        let cells = [
            IntersectionCell {
                x1: 0.,
                y1: 20.,
                x2: 80.,
                y2: 40.,
            },
            IntersectionCell {
                x1: 0.,
                y1: 0.,
                x2: 40.,
                y2: 20.,
            },
            IntersectionCell {
                x1: 40.,
                y1: 0.,
                x2: 80.,
                y2: 20.,
            },
        ];
        let spans = [create_test_span("Heading", 50., 28., 20., 6.)];
        let edges = [Edge {
            coord: 40.,
            start: 0.,
            end: 20.,
        }];
        let (rows, _) =
            assign_spans_to_intersection_grid(&cells, &[0., 40., 80.], &[0., 20., 40.], 2, &spans, &edges, true)
                .unwrap();
        assert_eq!(
            rows.iter()
                .flat_map(|r| &r.cells)
                .filter(|c| c.text == "Heading")
                .count(),
            1
        );
    }

    #[test]
    fn grid_has_cell_skips_cell_with_reversed_boundaries() {
        // y1/y2 are reversed relative to `ys`, so boundary_index resolves bottom > top;
        // this must be skipped rather than panic on `grid_has_cell[bottom..top]`.
        let cells = [IntersectionCell {
            x1: 0.,
            y1: 40.,
            x2: 40.,
            y2: 0.,
        }];
        let result = assign_spans_to_intersection_grid(&cells, &[0., 40., 80.], &[0., 20., 40.], 2, &[], &[], true);
        assert!(result.is_some(), "malformed cell geometry must not panic");
    }

    #[test]
    fn test_is_numeric_cell() {
        for ok in ["0.69", "100", "-1.2", "52%", "0", "1.00", "\u{2212}3.5"] {
            assert!(is_numeric_cell(ok), "{ok:?} should be numeric");
        }
        for no in ["Ours", "GLUE", "v3", "1e9", "0.6.6", "", "12345678.9", "p<0.05"] {
            assert!(!is_numeric_cell(no), "{no:?} should NOT be numeric");
        }
    }

    fn col_at(x: f32) -> ColumnCluster {
        ColumnCluster {
            x_center: x,
            x_min: x - 3.0,
            x_max: x + 3.0,
            span_indices: Vec::new(),
        }
    }

    #[test]
    fn grid_interval_prefers_exact_side_near_internal_boundary() {
        let boundaries = [0.0, 40.0, 70.0, 100.0];

        assert_eq!(grid_interval_for_point(68.5, &boundaries), Some(1));
        assert_eq!(grid_interval_for_point(71.0, &boundaries), Some(2));
        assert_eq!(grid_interval_for_point(70.0, &boundaries), Some(2));
        assert_eq!(grid_interval_for_point(-2.0, &boundaries), Some(0));
        assert_eq!(grid_interval_for_point(102.0, &boundaries), Some(2));
        assert_eq!(grid_interval_for_point(104.0, &boundaries), None);
    }

    #[test]
    fn intersection_grid_keeps_superscripts_and_boundary_text_in_their_cells() {
        let group_cells = [
            IntersectionCell {
                x1: 0.0,
                y1: 0.0,
                x2: 40.0,
                y2: 20.0,
            },
            IntersectionCell {
                x1: 40.0,
                y1: 0.0,
                x2: 70.0,
                y2: 20.0,
            },
            IntersectionCell {
                x1: 70.0,
                y1: 0.0,
                x2: 100.0,
                y2: 20.0,
            },
        ];
        let spans = vec![
            create_test_span("273", 10.0, 5.0, 27.0, 10.0),
            create_test_span("1", 37.0, 5.0, 2.0, 6.0),
            create_test_span("83", 45.0, 5.0, 21.0, 10.0),
            create_test_span("2", 66.0, 5.0, 2.0, 6.0),
            create_test_span("Europe", 66.0, 5.0, 10.0, 10.0),
        ];
        // A real ruled grid: column dividers span the row's full height, so
        // `band_column_groups` must not merge any of the three columns.
        let v_edges = [
            Edge {
                coord: 40.0,
                start: 0.0,
                end: 20.0,
            },
            Edge {
                coord: 70.0,
                start: 0.0,
                end: 20.0,
            },
        ];

        let (rows, _) = assign_spans_to_intersection_grid(
            &group_cells,
            &[0.0, 40.0, 70.0, 100.0],
            &[0.0, 20.0],
            3,
            &spans,
            &v_edges,
            true,
        )
        .expect("synthetic grid should be valid");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].cells[0].text, "2731");
        assert_eq!(rows[0].cells[1].text, "832");
        assert_eq!(rows[0].cells[2].text, "Europe");
    }

    // xberg-io/xberg#1580: an unruled full-width strip (a section heading) stacked between
    // two ruled 3-column bands is still cut at the ruled bands' column positions, because its
    // four corners exist as crossings (the neighbours' V rules happen to end on the strip's own
    // H rules) even though no V rule actually runs through the strip. Geometry matches the
    // issue's own reproducer: `xs = [44.8, 289.2, 317.6, 551.5]`, unruled band `y[465.4, 525.9]`.
    #[test]
    fn unruled_band_between_ruled_bands_is_not_cut_at_phantom_columns() {
        let group_cells = [
            // Ruled band y[525.9, 585.9]: real column dividers at x=289.2 and x=317.6.
            IntersectionCell {
                x1: 44.8,
                y1: 525.9,
                x2: 289.2,
                y2: 585.9,
            },
            IntersectionCell {
                x1: 289.2,
                y1: 525.9,
                x2: 317.6,
                y2: 585.9,
            },
            IntersectionCell {
                x1: 317.6,
                y1: 525.9,
                x2: 551.5,
                y2: 585.9,
            },
            // Unruled band y[465.4, 525.9]: same corner grid, but no V rule spans it — the
            // corners only exist because the ruled band above terminates its dividers there.
            IntersectionCell {
                x1: 44.8,
                y1: 465.4,
                x2: 289.2,
                y2: 525.9,
            },
            IntersectionCell {
                x1: 289.2,
                y1: 465.4,
                x2: 317.6,
                y2: 525.9,
            },
            IntersectionCell {
                x1: 317.6,
                y1: 465.4,
                x2: 551.5,
                y2: 525.9,
            },
        ];
        let xs = [44.8, 289.2, 317.6, 551.5];
        let ys = [465.4, 525.9, 585.9];

        // Ruled-band control row: one short word per column.
        let mut spans = vec![
            create_test_span("A", 140.0, 545.0, 20.0, 15.0),
            create_test_span("B", 295.0, 545.0, 15.0, 15.0),
            create_test_span("C", 400.0, 545.0, 20.0, 15.0),
        ];
        // Unruled heading strip: one continuous line, but the source spans happen to land at
        // the same X positions as the ruled band's columns above — exactly what the issue
        // reports for "8.2.7 Geen warmwater (alleen bij toepassing indirect gestookte boiler)".
        spans.extend([
            create_test_span(
                "8.2.7 Geen warmwater (alleen bij toepassing indirect",
                60.0,
                490.0,
                220.0,
                15.0,
            ),
            create_test_span("gest", 295.0, 490.0, 20.0, 15.0),
            create_test_span("ookte boiler)", 400.0, 490.0, 60.0, 15.0),
        ]);

        // The internal column dividers (x=289.2, x=317.6) span only the ruled band's height
        // [525.9, 585.9] — they terminate exactly where the unruled strip begins, which is
        // the geometry the issue reports (`build_grid_from_lines` would produce edges shaped
        // like this: rules that end on the strip's own H rules, never crossing the strip).
        let v_edges = [
            Edge {
                coord: 289.2,
                start: 525.9,
                end: 585.9,
            },
            Edge {
                coord: 317.6,
                start: 525.9,
                end: 585.9,
            },
        ];

        let (rows, _) = assign_spans_to_intersection_grid(&group_cells, &xs, &ys, 3, &spans, &v_edges, true)
            .expect("synthetic grid should be valid");

        assert_eq!(rows.len(), 2);
        let ruled_row = &rows[0];
        let unruled_row = &rows[1];

        // Control: the ruled band keeps its three columns.
        assert_eq!(ruled_row.cells.len(), 3, "ruled band must keep its three columns");
        assert_eq!(ruled_row.cells[0].text, "A");
        assert_eq!(ruled_row.cells[1].text, "B");
        assert_eq!(ruled_row.cells[2].text, "C");

        // Fix target: the unruled strip is one cell, not cut at the ruled band's column x's.
        let unruled_texts: Vec<&str> = unruled_row.cells.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(
            unruled_texts.len(),
            1,
            "unruled band must not be split at phantom column positions, got {unruled_texts:?}"
        );
        assert!(
            unruled_texts[0].contains("gest"),
            "heading text should be reassembled as one run, got {unruled_texts:?}"
        );
    }

    // xberg-io/xberg#1588: `BAND_RULE_SPAN_TOL` answers a CONTAINMENT question ("does this V
    // edge actually run through the band?"), not the IDENTITY question `SNAP_TOL` answers
    // ("are these two coordinates the same coordinate?"). Reusing `SNAP_TOL` (3.0) let an edge
    // fall up to 3pt short of the band at each end and still count as a drawn boundary.
    #[test]
    fn should_not_count_a_rule_that_stops_short_of_the_band_as_a_boundary() {
        let xs = [0.0, 40.0, 80.0];
        let (y_lo, y_hi) = (0.0, 20.0);
        let num_cols = 2;

        // The divider at x=40.0 stops 2pt short of the band at each end: within the old
        // `SNAP_TOL` (3.0) reuse, so it was wrongly counted as spanning; outside the fixed
        // 1.0 containment tolerance, so it must no longer count.
        let v_edges = [Edge {
            coord: 40.0,
            start: y_lo + 2.0,
            end: y_hi - 2.0,
        }];

        let groups = band_column_groups(&xs, y_lo, y_hi, num_cols, &v_edges, true);

        assert_eq!(
            groups,
            vec![(0, 1)],
            "a rule that stops short of the band must not split it into separate column groups, got {groups:?}"
        );
    }

    #[test]
    fn test_is_regular_lattice() {
        // Regular ~20pt pitch with one wider row-label gap on the left. ~keep
        let regular: Vec<ColumnCluster> = [113.0, 150.0, 170.0, 190.0, 210.0, 230.0]
            .iter()
            .map(|&x| col_at(x))
            .collect();
        assert!(is_regular_lattice(&regular));

        let small: Vec<ColumnCluster> = [100.0, 200.0, 300.0].iter().map(|&x| col_at(x)).collect();
        assert!(!is_regular_lattice(&small));

        // Irregular gaps (prose that happened to align) → rejected. ~keep
        let irregular: Vec<ColumnCluster> = [100.0, 140.0, 320.0, 330.0, 500.0, 505.0]
            .iter()
            .map(|&x| col_at(x))
            .collect();
        assert!(!is_regular_lattice(&irregular));
    }

    #[test]
    fn test_is_data_value() {
        for v in ["5,012", "+2%", "240", "-1.5", "1,000.50", "67", "\u{2212}3"] {
            assert!(is_data_value(v), "{v:?} should be a data value");
        }
        for w in ["FY22", "Mercury", "Body", "", "YoY", "$", "+", "Q1"] {
            assert!(!is_data_value(w), "{w:?} should NOT be a data value");
        }
    }

    #[test]
    fn test_quality_gate_admits_numeric_table_rejects_prose_split() {
        let row = |cells: &[&str]| TableRow {
            cells: cells.iter().map(|c| prose_cell(c)).collect(),
            is_header: false,
        };
        // A dense numeric metrics table: ~all single-token numeric cells. Must
        // PASS (the prose ratio excludes data values). ~keep
        let mut numeric = Table::new();
        numeric.col_count = 8;
        for r in [
            ["Body", "FY22", "FY23", "FY24", "FY25", "YoY", "Plan", "Var"],
            [
                "Mercury Transits",
                "5,012",
                "5,210",
                "5,488",
                "5,612",
                "+2%",
                "5,600",
                "+12",
            ],
            [
                "Venus Phases",
                "1,840",
                "1,902",
                "1,975",
                "2,041",
                "+3%",
                "2,030",
                "+11",
            ],
        ] {
            numeric.rows.push(row(&r));
        }
        assert!(
            passes_spatial_quality_gate(&numeric),
            "dense numeric table must pass the spatial quality gate"
        );

        // Prose accidentally split into single-word columns must still be REJECTED. ~keep
        let mut prose = Table::new();
        prose.col_count = 6;
        for _ in 0..3 {
            prose.rows.push(row(&["the", "quick", "brown", "fox", "jumps", "over"]));
        }
        assert!(
            !passes_spatial_quality_gate(&prose),
            "word-dominated single-word split must still be rejected"
        );
    }

    fn prose_cell(text: &str) -> TableCell {
        TableCell {
            text: text.to_string(),
            spans: Vec::new(),
            colspan: 1,
            rowspan: 1,
            mcids: Vec::new(),
            bbox: None,
            is_header: false,
        }
    }

    #[test]
    fn test_looks_like_cjk_prose() {
        let row = |cells: &[&str]| TableRow {
            cells: cells.iter().map(|c| prose_cell(c)).collect(),
            is_header: false,
        };
        // CJK prose mis-split into columns: a long ideograph/kana run in a cell. ~keep
        let mut prose = Table::new();
        prose.col_count = 2;
        prose.rows.push(row(&[
            "\u{30CD}\u{30B3}",
            "\u{72ED}\u{7FA9}\u{306B}\u{306F}\u{98DF}\u{8089}\u{76EE}\u{30CD}\u{30B3}\u{79D1}\u{30CD}\u{30B3}\u{5C5E}",
        ]));
        assert!(looks_like_cjk_prose(&prose));

        // A genuine CJK data table — short label + number cells — is NOT prose. ~keep
        let mut table = Table::new();
        table.col_count = 2;
        table.rows.push(row(&["\u{58F2}\u{4E0A}", "1,234"]));
        table.rows.push(row(&["\u{5229}\u{76CA}", "567"]));
        assert!(!looks_like_cjk_prose(&table));

        // Latin prose has no long CJK run. ~keep
        let mut latin = Table::new();
        latin.col_count = 2;
        latin.rows.push(row(&["Region", "North"]));
        assert!(!looks_like_cjk_prose(&latin));
    }

    #[test]
    fn test_looks_like_bulleted_list_rejects_bullet_cells() {
        let row = |cells: &[&str]| TableRow {
            cells: cells.iter().map(|c| prose_cell(c)).collect(),
            is_header: false,
        };
        // A bulleted list mis-fused into two columns: lone bullet markers in
        // the first column → recognise as a list, not a table. ~keep
        let mut list = Table::new();
        list.col_count = 2;
        list.rows.push(row(&["\u{2022}", "Ship the API."]));
        list.rows.push(row(&["\u{2022}", "Write the docs."]));
        assert!(looks_like_bulleted_list(&list));

        // A genuine two-column data table has no lone-bullet cell. ~keep
        let mut table = Table::new();
        table.col_count = 2;
        table.rows.push(row(&["Region", "Q1"]));
        table.rows.push(row(&["North", "120"]));
        assert!(!looks_like_bulleted_list(&table));

        // A cell that merely *starts* with a dash but carries text (a value or
        // a negative number) is not a bullet marker. ~keep
        let mut dash = Table::new();
        dash.col_count = 2;
        dash.rows.push(row(&["Delta", "-12"]));
        assert!(!looks_like_bulleted_list(&dash));
    }

    /// Prose gate: a wrapped paragraph mis-split into a table — a row
    /// crossing a sentence boundary ("...to 23,500. Stockout rate...") must
    /// be recognised as prose and rejected.
    #[test]
    fn test_looks_like_prose_paragraph_detects_sentence_crossing_row() {
        let mut t = Table::new();
        t.col_count = 4;
        t.rows.push(TableRow {
            cells: vec![
                prose_cell("Total SKU count grew 15%"),
                prose_cell("quarter-over-quarter to"),
                prose_cell("23,500."),
                prose_cell("Stockout rate improved by 200 basis"),
            ],
            is_header: false,
        });
        assert!(looks_like_prose_paragraph(&t));
    }

    /// Caseless scripts (Bengali here) have no capital-letter signal at all,
    /// so their sentence-final danda ('।') must be treated as a terminator
    /// in its own right. Real-world PDFs sometimes render the danda with a
    /// stray gap before it ("প্রাণী ।"), so the check must tolerate that.
    #[test]
    fn test_looks_like_prose_paragraph_detects_bengali_danda_crossing_row() {
        let mut t = Table::new();
        t.col_count = 4;
        t.rows.push(TableRow {
            cells: vec![
                prose_cell("বিড়াল একটি গার্হস্থ্য প্রজাতি"),
                prose_cell("স্তন্যপায়ী প্রাণী"),
                prose_cell("। এটি"),
                prose_cell("ফেলিডা পরিবারের একমাত্র গৃহপালিত প্রজাতি"),
            ],
            is_header: false,
        });
        assert!(looks_like_prose_paragraph(&t));
    }

    /// REGRESSION GUARD: a genuine data table (short value/label cells, no
    /// sentence crossing a row) must NOT be flagged as prose.
    #[test]
    fn test_looks_like_prose_paragraph_keeps_real_table() {
        let mut t = Table::new();
        t.col_count = 4;
        for cells in [
            ["Zone", "Pallets stored", "11,100", "-2.5%"],
            ["A", "Utilization", "87%", "-3pp"],
            ["B", "Damage rate", "0.3%", "-0.2pp"],
        ] {
            t.rows.push(TableRow {
                cells: cells.iter().map(|c| prose_cell(c)).collect(),
                is_header: false,
            });
        }
        assert!(!looks_like_prose_paragraph(&t));
    }

    #[test]
    fn test_line_clustering_multiple_tables() {
        let lines = vec![
            make_rect_path(10.0, 100.0, 50.0, 20.0),
            make_rect_path(10.0, 50.0, 50.0, 20.0), // Far away vertically ~keep
        ];
        let config = TableDetectionConfig::default();
        let clusters = group_lines_into_clusters(&lines, &config);
        assert_eq!(
            clusters.len(),
            2,
            "Should find 2 separate table regions with optimized clustering"
        );
    }

    #[test]
    fn test_line_clustering_horizontal_separation() {
        let lines = vec![
            make_rect_path(10.0, 100.0, 50.0, 20.0), // Table 1: x=10..60 ~keep
            make_rect_path(80.0, 100.0, 50.0, 20.0), // Table 2: x=80..130 (20pt gap) ~keep
        ];
        let config = TableDetectionConfig::default();
        let clusters = group_lines_into_clusters(&lines, &config);
        assert_eq!(
            clusters.len(),
            2,
            "Should find 2 separate table regions even if nearby horizontally"
        );
    }

    /// GH#1656: a page-sized background rectangle unions every primitive on
    /// the page into one cluster (`is_table_primitive`'s `<1000pt` bound
    /// admits an A4/Letter page). Document-level callers
    /// (`document/tables.rs`) now drop such rectangles via
    /// `PathContent::is_page_frame_rectangle` before calling
    /// `group_lines_into_clusters`. This documents both the pre-fix
    /// behavior (frame present → 1 cluster) and the fixed behavior (frame
    /// filtered → the table, the footer rule, and the figure border stay
    /// in 3 separate clusters). ~keep
    #[test]
    fn test_gh1656_page_frame_unions_clusters_until_caller_filters_it() {
        let media_box = (0.0, 0.0, 595.28, 842.0);
        let page_frame = make_rect_path(0.0, 0.0, 595.28, 842.0);

        // A small ruled 2-column table, matching the reporter's control
        // page 2 bbox (x[45.36,321.84] y[472.70,609.50]). ~keep
        let table_lines = vec![
            make_v_line(45.36, 472.70, 136.80),
            make_v_line(321.84, 472.70, 136.80),
            make_h_line(45.36, 609.50, 276.48),
            make_h_line(45.36, 472.70, 276.48),
        ];
        // A full-page-width footer rule, far below the table (y gap far
        // beyond the 3pt cluster expansion) — legitimate ruling, not
        // furniture. ~keep
        let footer_rule = make_h_line(0.0, 48.30, 595.28);
        // A figure border, far from both the table and the footer rule in
        // both axes. ~keep
        let figure_border = make_rect_path(400.0, 700.0, 100.0, 80.0);

        let mut all = vec![page_frame];
        all.extend(table_lines.clone());
        all.push(footer_rule.clone());
        all.push(figure_border.clone());

        let config = TableDetectionConfig::default();
        let clusters_with_frame = group_lines_into_clusters(&all, &config);
        assert_eq!(
            clusters_with_frame.len(),
            1,
            "page-sized background rect must union every primitive (documents today's defect)"
        );

        let filtered: Vec<_> = all
            .into_iter()
            .filter(|p| !p.is_page_frame_rectangle(media_box))
            .collect();
        assert_eq!(
            filtered.len(),
            6,
            "the filter must remove exactly the page frame and nothing else"
        );

        let clusters_without_frame = group_lines_into_clusters(&filtered, &config);
        assert_eq!(
            clusters_without_frame.len(),
            3,
            "table, footer rule, and figure border must stay separate once the frame is filtered"
        );
    }

    /// GH#1656 (the reporter's "observed alongside"): the carrier's drawing style, Word's way
    /// of drawing cell borders -- every rule cut at the crossings into per-cell 0.48pt filled
    /// bars with 0.48pt corner squares between them. The reporter's page-28 coordinates:
    /// 13 H rules at an 11.40pt pitch from y 472.46, V rules at x 45.12 / 151.50 / 321.60. ~keep
    fn gh1656_segmented_rule_table() -> (Vec<TextSpan>, Vec<crate::elements::PathContent>) {
        const BAR: f32 = 0.48;
        const PITCH: f32 = 11.40;
        const Y0: f32 = 472.46;
        const ROWS: usize = 12;
        let col_xs = [45.12_f32, 151.50, 321.60];
        let mut paths = Vec::new();
        for r in 0..=ROWS {
            let y = Y0 + r as f32 * PITCH;
            for pair in col_xs.windows(2) {
                paths.push(make_rect_path(pair[0] + BAR, y, pair[1] - pair[0] - BAR, BAR));
            }
            for &x in &col_xs {
                paths.push(make_rect_path(x, y, BAR, BAR));
            }
            if r < ROWS {
                for &x in &col_xs {
                    paths.push(make_rect_path(x, y + BAR, BAR, PITCH - BAR));
                }
            }
        }
        let mut spans = Vec::new();
        for r in 0..ROWS {
            let y = Y0 + (ROWS - 1 - r) as f32 * PITCH + 2.0;
            spans.push(create_test_span(&format!("{}", r + 1), 50.0, y, 10.0, 8.0));
            spans.push(create_test_span(&format!("{}", 130 + r * 15), 160.0, y, 20.0, 8.0));
        }
        (spans, paths)
    }

    /// GH#1656: with per-cell segmented rules, `build_grid_from_lines` found all 24 cells and
    /// 12 row bands, yet the table came back as its last two rows. `split_table_at_section_dividers`
    /// treated every internal H rule as a section divider, because every V segment terminates
    /// at every rule in this drawing style, split the table into one-row pieces and dropped
    /// the pieces below `min_table_cells`. A rule that *every* V rule stops at carries no
    /// section information when the same is true of every other rule on the table. ~keep
    #[test]
    fn gh1656_per_cell_segmented_rules_keep_every_row_of_the_table() {
        let (spans, paths) = gh1656_segmented_rule_table();
        let table_paths: Vec<_> = paths.into_iter().filter(|p| p.is_table_primitive()).collect();
        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            ..Default::default()
        };

        let (groups, _, _) = build_grid_from_lines(&table_paths, &config);
        assert_eq!(groups.len(), 1, "fixture assumption: one cell group");
        assert_eq!(groups[0].0.len(), 24, "fixture assumption: the grid itself is complete");
        assert_eq!(groups[0].2.len(), 13, "fixture assumption: 12 row bands");

        let tables = detect_tables_with_lines(&spans, &table_paths, &config);
        assert_eq!(tables.len(), 1, "one table, not one-row fragments: {tables:?}");
        let table = &tables[0];
        assert_eq!(table.col_count, 2);
        assert_eq!(
            table.rows.len(),
            12,
            "every row band must survive, got {:?}",
            table
                .rows
                .iter()
                .map(|r| r.cells.iter().map(|c| c.text.as_str()).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            table.rows[0].cells.iter().map(|c| c.text.as_str()).collect::<Vec<_>>(),
            ["1", "130"]
        );
        assert_eq!(
            table.rows[11].cells.iter().map(|c| c.text.as_str()).collect::<Vec<_>>(),
            ["12", "295"]
        );
    }

    fn create_test_span(text: &str, x: f32, y: f32, width: f32, height: f32) -> TextSpan {
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: text.to_string(),
            bbox: Rect::new(x, y, width, height),
            font_name: "TestFont".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 1.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        }
    }
    fn make_h_line(x: f32, y: f32, width: f32) -> crate::elements::PathContent {
        crate::elements::PathContent::line(x, y, x + width, y)
    }
    fn make_v_line(x: f32, y: f32, height: f32) -> crate::elements::PathContent {
        crate::elements::PathContent::line(x, y, x, y + height)
    }
    fn make_line_path(x1: f32, y1: f32, x2: f32, y2: f32) -> crate::elements::PathContent {
        crate::elements::PathContent::line(x1, y1, x2, y2)
    }
    fn make_rect_path(x: f32, y: f32, w: f32, h: f32) -> crate::elements::PathContent {
        crate::elements::PathContent::rect(x, y, w, h)
    }

    /// An agenda-style table has 3 real columns (Time @72,
    /// Activity @200, Team @420). The Activity cell holds multiple words
    /// laid out with wide gaps ("Receiving Dock Inspection"), each at a
    /// distinct X that occurs in only ONE row. Greedy column clustering
    /// turns every word X into a column; the cross-row text-edge
    /// detector must instead recover the 3 real columns whose edges
    /// recur across rows. Asserts the detected table has 3 columns, not
    /// one-per-word.
    #[test]
    fn test_issue6_agenda_words_not_split_into_columns() {
        // y descending = rows top→bottom. 4 rows incl. header. ~keep
        let spans = vec![
            create_test_span("Time", 72.0, 638.6, 24.4, 12.0),
            create_test_span("Activity", 200.0, 638.6, 34.8, 12.0),
            create_test_span("Team", 420.0, 638.6, 28.1, 12.0),
            create_test_span("06:00 - 07:00", 72.0, 610.6, 61.1, 12.0),
            create_test_span("Receiving", 200.0, 610.6, 43.9, 12.0),
            create_test_span("Dock", 249.9, 610.6, 22.8, 12.0),
            create_test_span("Inspection", 278.7, 610.6, 45.6, 12.0),
            create_test_span("Inbound Team", 420.0, 610.6, 65.7, 12.0),
            create_test_span("07:00 - 09:00", 72.0, 582.6, 61.1, 12.0),
            create_test_span("Bulk", 200.0, 582.6, 19.5, 12.0),
            create_test_span("Putaway", 225.4, 582.6, 38.3, 12.0),
            create_test_span("Slotting", 282.5, 582.6, 33.4, 12.0),
            create_test_span("Warehouse Ops", 420.0, 582.6, 73.5, 12.0),
            create_test_span("09:00 - 11:00", 72.0, 554.6, 61.1, 12.0),
            create_test_span("Pick", 200.0, 554.6, 18.9, 12.0),
            create_test_span("Wave", 230.0, 554.6, 24.0, 12.0),
            create_test_span("Processing", 262.0, 554.6, 48.0, 12.0),
            create_test_span("Fulfillment", 420.0, 554.6, 55.0, 12.0),
        ];
        let config = TableDetectionConfig::default();
        let tables = detect_tables_from_spans(&spans, &config);
        // Either no table (acceptable — agenda is borderline tabular) or
        // a table with the 3 real columns. What must NOT happen: a table
        // with one column per Activity word (>= 5 columns). ~keep
        if let Some(t) = tables.first() {
            let ncols = t.rows.iter().map(|r| r.cells.len()).max().unwrap_or(0);
            assert!(
                ncols <= 4,
                "agenda must not fragment Activity words into columns; got {} cols",
                ncols
            );
        }
    }

    #[test]
    fn test_lines_strategy_no_lines_returns_empty() {
        let spans = vec![
            create_test_span("A", 10.0, 100.0, 10.0, 10.0),
            create_test_span("B", 50.0, 100.0, 10.0, 10.0),
            create_test_span("C", 10.0, 80.0, 10.0, 10.0),
            create_test_span("D", 50.0, 80.0, 10.0, 10.0),
        ];
        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            ..TableDetectionConfig::default()
        };
        assert!(detect_tables_with_lines(&spans, &[], &config).is_empty());
    }

    #[test]
    fn test_horizontal_lines_only_strategy_no_false_positives() {
        let spans = vec![
            create_test_span("A", 10.0, 100.0, 10.0, 10.0),
            create_test_span("B", 50.0, 100.0, 10.0, 10.0),
            create_test_span("C", 10.0, 80.0, 10.0, 10.0),
            create_test_span("D", 50.0, 80.0, 10.0, 10.0),
        ];
        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Both,
            ..TableDetectionConfig::default()
        };
        assert!(detect_tables_with_lines(&spans, &[], &config).is_empty());
    }

    /// Text-only spatial fallback for line-less tables.
    ///
    /// `text_fallback = true` is now the default on `TableDetectionConfig` (the
    /// prose-shape filter and ≥3-row guard suppress the false positives that
    /// previously motivated a `false` default).  With the default and the `Both`
    /// strategy, `detect_tables_with_lines` with an empty lines slice falls
    /// through to the text-based path and detects the grid from span alignment
    /// alone.  Callers that explicitly want the conservative
    /// "no ruling lines → no tables" behaviour set `text_fallback = false` and
    /// the `extract_page_tables` early-return guard in document.rs short-circuits
    /// before this code is reached.
    ///
    /// This test directly calls `detect_tables_with_lines` with an empty lines
    /// slice to verify that the text-based path inside it finds the table.
    #[test]
    fn test_text_fallback_detects_lineless_grid() {
        let spans = vec![
            create_test_span("Pos", 10.0, 200.0, 25.0, 10.0),
            create_test_span("Boat", 50.0, 200.0, 25.0, 10.0),
            create_test_span("Pts", 90.0, 200.0, 20.0, 10.0),
            create_test_span("1", 10.0, 180.0, 25.0, 10.0),
            create_test_span("Alpha", 50.0, 180.0, 25.0, 10.0),
            create_test_span("14", 90.0, 180.0, 20.0, 10.0),
            create_test_span("2", 10.0, 160.0, 25.0, 10.0),
            create_test_span("Beta", 50.0, 160.0, 25.0, 10.0),
            create_test_span("17", 90.0, 160.0, 20.0, 10.0),
            create_test_span("3", 10.0, 140.0, 25.0, 10.0),
            create_test_span("Gamma", 50.0, 140.0, 25.0, 10.0),
            create_test_span("21", 90.0, 140.0, 20.0, 10.0),
        ];

        // With the Both strategy, NO lines, and text_fallback explicitly
        // enabled, the text-based fallback inside detect_tables_with_lines
        // fires and finds the grid.  The default no longer enables
        // text_fallback to avoid spurious tables on report-style PDFs that
        // would otherwise be double-emitted by extract_text. ~keep
        let config = TableDetectionConfig {
            text_fallback: true,
            ..TableDetectionConfig::default()
        };
        let tables = detect_tables_with_lines(&spans, &[], &config);
        assert_eq!(
            tables.len(),
            1,
            "Text-only fallback in detect_tables_with_lines should detect the grid (got {:?} tables)",
            tables.len()
        );
        let t = &tables[0];
        assert_eq!(t.col_count, 3, "Should detect 3 columns");
        assert_eq!(t.rows.len(), 4, "Should detect 4 rows");
    }

    /// Verify that when no lines are present and `text_fallback = false` (the default),
    /// the guard in `extract_page_tables` (outside `detect_tables_with_lines`) would
    /// prevent the text path from running.  We simulate this at the config level: using
    /// a `Lines`-only strategy ensures `detect_tables_with_lines` returns nothing when
    /// paths are empty — confirming the safety contract for the public API path.
    #[test]
    fn test_text_fallback_disabled_lines_strategy_returns_empty() {
        let spans = vec![
            create_test_span("Pos", 10.0, 200.0, 25.0, 10.0),
            create_test_span("Boat", 50.0, 200.0, 25.0, 10.0),
            create_test_span("Pts", 90.0, 200.0, 20.0, 10.0),
            create_test_span("1", 10.0, 180.0, 25.0, 10.0),
            create_test_span("Alpha", 50.0, 180.0, 25.0, 10.0),
            create_test_span("14", 90.0, 180.0, 20.0, 10.0),
        ];
        // Lines-only strategy: no lines → no tables.  This is what the public
        // extract_tables() API uses after the early-return guard fires. ~keep
        let config = TableDetectionConfig::strict(); // strict() uses Lines/Lines ~keep
        let tables = detect_tables_with_lines(&spans, &[], &config);
        assert!(
            tables.is_empty(),
            "Lines-only strategy with no ruling lines should return no tables"
        );
    }

    #[test]
    fn test_table_splitting_on_empty_row() {
        let spans = vec![
            create_test_span("T1-11", 20.0, 115.0, 10.0, 10.0),
            create_test_span("T1-12", 40.0, 115.0, 10.0, 10.0),
            create_test_span("T1-21", 20.0, 95.0, 10.0, 10.0),
            create_test_span("T1-22", 40.0, 95.0, 10.0, 10.0),
            create_test_span("T2-11", 20.0, 35.0, 10.0, 10.0),
            create_test_span("T2-12", 40.0, 35.0, 10.0, 10.0),
            create_test_span("T2-21", 20.0, 15.0, 10.0, 10.0),
            create_test_span("T2-22", 40.0, 15.0, 10.0, 10.0),
        ];
        let lines = vec![
            make_h_line(10.0, 130.0, 50.0),
            make_h_line(10.0, 110.0, 50.0),
            make_h_line(10.0, 90.0, 50.0),
            make_v_line(10.0, 90.0, 40.0),
            make_v_line(30.0, 90.0, 40.0),
            make_v_line(60.0, 90.0, 40.0),
            make_h_line(10.0, 50.0, 50.0),
            make_h_line(10.0, 30.0, 50.0),
            make_h_line(10.0, 10.0, 50.0),
            make_v_line(10.0, 10.0, 40.0),
            make_v_line(30.0, 10.0, 40.0),
            make_v_line(60.0, 10.0, 40.0),
            make_v_line(10.0, 50.0, 40.0),
        ];
        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Both,
            vertical_strategy: TableStrategy::Both,
            ..TableDetectionConfig::default()
        };
        assert_eq!(detect_tables_with_lines(&spans, &lines, &config).len(), 2);
    }

    #[test]
    fn test_detect_columns_invoice_4_columns() {
        let spans = vec![
            create_test_span("01/01", 50.0, 100.0, 50.0, 10.0),
            create_test_span("Widget", 130.0, 100.0, 220.0, 10.0),
            create_test_span("$100", 500.0, 100.0, 50.0, 10.0),
            create_test_span("$0", 600.0, 100.0, 50.0, 10.0),
            create_test_span("02/15", 50.0, 80.0, 50.0, 10.0),
            create_test_span("Service fee", 130.0, 80.0, 220.0, 10.0),
            create_test_span("$250", 500.0, 80.0, 50.0, 10.0),
            create_test_span("$50", 600.0, 80.0, 50.0, 10.0),
            create_test_span("03/20", 50.0, 60.0, 50.0, 10.0),
            create_test_span("Consulting", 130.0, 60.0, 220.0, 10.0),
            create_test_span("$500", 500.0, 60.0, 50.0, 10.0),
            create_test_span("$100", 600.0, 60.0, 50.0, 10.0),
        ];
        let config = TableDetectionConfig::default();
        let columns = detect_columns(&spans, config.column_tolerance, config.column_merge_threshold);
        assert_eq!(
            columns.len(),
            4,
            "Invoice with 4 distinct column groups should produce exactly 4 columns, got {}",
            columns.len()
        );
    }

    #[test]
    fn test_detect_columns_merges_nearby_clusters() {
        let spans = vec![
            create_test_span("A", 50.0, 100.0, 30.0, 10.0),
            create_test_span("B", 130.0, 100.0, 30.0, 10.0),
            create_test_span("C", 50.0, 80.0, 30.0, 10.0),
            create_test_span("D", 135.0, 80.0, 30.0, 10.0),
            create_test_span("E", 50.0, 60.0, 30.0, 10.0),
            create_test_span("F", 140.0, 60.0, 30.0, 10.0),
        ];
        let config = TableDetectionConfig::default();
        let columns = detect_columns(&spans, config.column_tolerance, config.column_merge_threshold);
        assert_eq!(
            columns.len(),
            2,
            "Spans at x=130/135/140 should merge into 1 column, plus x=50 = 2 total, got {}",
            columns.len()
        );
    }

    #[test]
    fn test_detect_columns_dense_pitch_stays_distinct() {
        // A dense numeric table: 6 columns on a 20pt pitch, each column's
        // spans sharing an exact left-x (so the *first* clustering pass,
        // gated on column_tolerance=15, keeps every column separate — this
        // isolates the merge-threshold pass specifically). Under the
        // default config's *fixed* 25pt column_merge_threshold, every
        // adjacent 20pt gap is below the threshold, so the old code fused
        // all 6 into 1. The threshold must scale down with this table's own
        // (much smaller than 25pt) pitch so the columns survive distinct. ~keep
        let mut spans = Vec::new();
        for row in 0..3 {
            let y = 100.0 - row as f32 * 20.0;
            for col in 0..6 {
                let x = 20.0 + col as f32 * 20.0;
                spans.push(create_test_span("9", x, y, 8.0, 10.0));
            }
        }
        let config = TableDetectionConfig::default();
        let columns = detect_columns(&spans, config.column_tolerance, config.column_merge_threshold);
        assert_eq!(
            columns.len(),
            6,
            "a 20pt-pitch dense table must keep all 6 columns distinct \
             despite the 25pt fixed merge_threshold, got {}",
            columns.len()
        );
    }

    #[test]
    fn test_detect_columns_order_independent() {
        let spans_ordered = vec![
            create_test_span("A", 50.0, 100.0, 30.0, 10.0),
            create_test_span("B", 200.0, 100.0, 30.0, 10.0),
            create_test_span("C", 400.0, 100.0, 30.0, 10.0),
            create_test_span("D", 50.0, 80.0, 30.0, 10.0),
            create_test_span("E", 200.0, 80.0, 30.0, 10.0),
            create_test_span("F", 400.0, 80.0, 30.0, 10.0),
        ];
        let spans_reversed = vec![
            create_test_span("F", 400.0, 80.0, 30.0, 10.0),
            create_test_span("E", 200.0, 80.0, 30.0, 10.0),
            create_test_span("D", 50.0, 80.0, 30.0, 10.0),
            create_test_span("C", 400.0, 100.0, 30.0, 10.0),
            create_test_span("B", 200.0, 100.0, 30.0, 10.0),
            create_test_span("A", 50.0, 100.0, 30.0, 10.0),
        ];
        let config = TableDetectionConfig::default();
        let cols_ordered = detect_columns(&spans_ordered, config.column_tolerance, config.column_merge_threshold);
        let cols_reversed = detect_columns(&spans_reversed, config.column_tolerance, config.column_merge_threshold);
        assert_eq!(
            cols_ordered.len(),
            cols_reversed.len(),
            "Column count should be independent of span order"
        );
        let centers_ordered: Vec<f32> = cols_ordered.iter().map(|c| (c.x_center * 10.0).round()).collect();
        let centers_reversed: Vec<f32> = cols_reversed.iter().map(|c| (c.x_center * 10.0).round()).collect();
        assert_eq!(
            centers_ordered, centers_reversed,
            "Column centers should match regardless of input order"
        );
    }

    #[test]
    fn test_detect_header_row_returns_none_when_no_heuristic_matches() {
        // All spans have same font size, none bold -- no header signal ~keep
        let spans = vec![
            create_test_span("A", 10.0, 100.0, 30.0, 10.0),
            create_test_span("B", 50.0, 100.0, 30.0, 10.0),
            create_test_span("C", 10.0, 80.0, 30.0, 10.0),
            create_test_span("D", 50.0, 80.0, 30.0, 10.0),
        ];
        let columns = detect_columns(&spans, 15.0, 25.0);
        let rows = detect_rows(&spans, 2.8);
        let grid = assign_spans_to_cells(&spans, &columns, &rows);
        let header = detect_header_row(&grid, &spans);
        assert_eq!(header, None, "Should return None when no heuristic matches");
    }

    #[test]
    fn test_hierarchical_header_with_visual_heuristic() {
        let spans = vec![
            create_test_span("H1", 10.0, 115.0, 35.0, 10.0),
            create_test_span("H2", 55.0, 115.0, 35.0, 10.0),
            create_test_span("Col 1", 10.0, 95.0, 35.0, 10.0),
            create_test_span("Col 2", 55.0, 95.0, 35.0, 10.0),
            create_test_span("Data 1", 10.0, 75.0, 35.0, 10.0),
            create_test_span("Data 2", 55.0, 75.0, 35.0, 10.0),
        ];
        let lines = vec![
            make_line_path(10.0, 130.0, 90.0, 130.0),
            make_line_path(10.0, 110.0, 90.0, 110.0),
            make_line_path(10.0, 90.0, 90.0, 90.0),
            make_v_line(10.0, 70.0, 60.0),
            make_v_line(50.0, 70.0, 20.0),
            make_v_line(90.0, 70.0, 60.0),
        ];
        let config = TableDetectionConfig::default();
        let tables = detect_tables_with_lines(&spans, &lines, &config);
        assert_eq!(tables.len(), 1);
        assert!(tables[0].rows[0].is_header);
        assert!(tables[0].rows[1].is_header);
    }

    #[test]
    fn test_intersection_basic_2x2_table() {
        let lines = vec![
            make_h_line(50.0, 100.0, 350.0),
            make_h_line(50.0, 200.0, 350.0),
            make_h_line(50.0, 300.0, 350.0),
            make_v_line(50.0, 100.0, 200.0),
            make_v_line(200.0, 100.0, 200.0),
            make_v_line(400.0, 100.0, 200.0),
        ];
        let spans = vec![
            create_test_span("A1", 120.0, 145.0, 20.0, 10.0),
            create_test_span("B1", 295.0, 145.0, 20.0, 10.0),
            create_test_span("A2", 120.0, 245.0, 20.0, 10.0),
            create_test_span("B2", 295.0, 245.0, 20.0, 10.0),
        ];
        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            min_table_cells: 4,
            min_table_columns: 2,
            ..TableDetectionConfig::default()
        };
        let tables = detect_tables_with_lines(&spans, &lines, &config);
        assert_eq!(tables.len(), 1, "Should detect exactly 1 table");
        let table = &tables[0];
        assert_eq!(table.rows.len(), 2, "Should have 2 rows");
        assert_eq!(table.col_count, 2, "Should have 2 columns");

        // Higher y = higher on page, so rows sorted descending by y.
        // Row at y=[200,300] (higher) comes first in display order.
        // Row at y=[100,200] (lower) comes second. ~keep
        let r0_texts: Vec<&str> = table.rows[0].cells.iter().map(|c| c.text.as_str()).collect();
        let r1_texts: Vec<&str> = table.rows[1].cells.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(r0_texts, vec!["A2", "B2"], "Top row (higher y) should be A2, B2");
        assert_eq!(r1_texts, vec!["A1", "B1"], "Bottom row (lower y) should be A1, B1");
    }

    #[test]
    fn test_intersection_snap_and_merge_edges() {
        // Two H edges at y=100 and y=101.5 (within SNAP_TOL=3) should snap. ~keep
        let mut edges = vec![
            Edge {
                coord: 100.0,
                start: 0.0,
                end: 50.0,
            },
            Edge {
                coord: 101.5,
                start: 0.0,
                end: 50.0,
            },
        ];
        snap_and_merge(&mut edges);
        assert_eq!(edges.len(), 1, "Snapped edges should merge into 1");
        assert!((edges[0].coord - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_intersection_join_collinear_segments() {
        // Two segments on same coord, gap of 2pt (within JOIN_TOL=3). ~keep
        let mut edges = vec![
            Edge {
                coord: 100.0,
                start: 0.0,
                end: 50.0,
            },
            Edge {
                coord: 100.0,
                start: 52.0,
                end: 100.0,
            },
        ];
        snap_and_merge(&mut edges);
        assert_eq!(edges.len(), 1, "Collinear segments within 3pt should join");
        assert!((edges[0].start - 0.0).abs() < 0.01);
        assert!((edges[0].end - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_intersection_discard_short_edges() {
        let mut edges = vec![
            Edge {
                coord: 100.0,
                start: 0.0,
                end: 4.0,
            }, // 4pt < MIN_EDGE_LEN ~keep
            Edge {
                coord: 200.0,
                start: 0.0,
                end: 50.0,
            },
        ];
        snap_and_merge(&mut edges);
        assert_eq!(edges.len(), 1, "Short edge should be discarded");
        assert!((edges[0].coord - 200.0).abs() < 0.01);
    }

    #[test]
    fn test_intersection_find_intersections_basic() {
        let h = vec![
            Edge {
                coord: 100.0,
                start: 0.0,
                end: 200.0,
            },
            Edge {
                coord: 200.0,
                start: 0.0,
                end: 200.0,
            },
        ];
        let v = vec![
            Edge {
                coord: 50.0,
                start: 50.0,
                end: 250.0,
            },
            Edge {
                coord: 150.0,
                start: 50.0,
                end: 250.0,
            },
        ];
        let pts = find_intersections(&h, &v);
        assert_eq!(pts.len(), 4, "2 H x 2 V = 4 intersections");
    }

    #[test]
    fn test_intersection_no_crossing_means_no_intersection() {
        let h = vec![Edge {
            coord: 100.0,
            start: 0.0,
            end: 50.0,
        }];
        let v = vec![Edge {
            coord: 100.0,
            start: 0.0,
            end: 200.0,
        }];
        let pts = find_intersections(&h, &v);
        assert!(pts.is_empty(), "Non-crossing edges should produce no intersection");
    }

    #[test]
    fn test_intersection_build_cells() {
        let pts = vec![
            Intersection { x: 0.0, y: 0.0 },
            Intersection { x: 100.0, y: 0.0 },
            Intersection { x: 0.0, y: 100.0 },
            Intersection { x: 100.0, y: 100.0 },
        ];
        // Real drawn V edges spanning the full cell height, so the "four
        // corners are not four sides" containment check accepts the cell. ~keep
        let v_edges = [
            Edge {
                coord: 0.0,
                start: 0.0,
                end: 100.0,
            },
            Edge {
                coord: 100.0,
                start: 0.0,
                end: 100.0,
            },
        ];
        let cells = build_cells_from_intersections(&pts, &[], &v_edges);
        assert_eq!(cells.len(), 1, "4 corners should produce 1 cell");
    }

    #[test]
    fn test_intersection_group_adjacent_cells() {
        let cells = vec![
            IntersectionCell {
                x1: 0.0,
                y1: 0.0,
                x2: 100.0,
                y2: 100.0,
            },
            IntersectionCell {
                x1: 100.0,
                y1: 0.0,
                x2: 200.0,
                y2: 100.0,
            },
        ];
        let groups = group_cells_into_tables(&cells);
        assert_eq!(groups.len(), 1, "Adjacent cells should be in 1 group");
    }

    #[test]
    fn test_intersection_separate_tables() {
        let cells = vec![
            IntersectionCell {
                x1: 0.0,
                y1: 0.0,
                x2: 100.0,
                y2: 100.0,
            },
            IntersectionCell {
                x1: 500.0,
                y1: 500.0,
                x2: 600.0,
                y2: 600.0,
            },
        ];
        let groups = group_cells_into_tables(&cells);
        assert_eq!(groups.len(), 2, "Distant cells should be in separate groups");
    }

    /// xberg-io/xberg#1601: two disjoint bordered tables that happen to share
    /// column X-positions (a common shape — same field layout repeated after a
    /// section heading) are separated by a 40pt band with NO path of any kind
    /// in it. `build_cells_from_intersections` accepts a cell as soon as its
    /// four corners are independently-existing crossing points, with no check
    /// that a drawn V edge actually spans the candidate cell's Y-range — so it
    /// manufactures a phantom cell bridging the two tables' shared X columns
    /// across the empty band, and the heading text sitting in the band gets
    /// admitted into that phantom cell as a one-cell row of the merged table.
    #[test]
    fn gh1601_graphics_free_gap_does_not_bridge_two_tables() {
        let lines = vec![
            // Table B (lower): rows y=[10,30],[30,50], cols x=[10,30],[30,60]. ~keep
            make_h_line(10.0, 10.0, 50.0),
            make_h_line(10.0, 30.0, 50.0),
            make_h_line(10.0, 50.0, 50.0),
            make_v_line(10.0, 10.0, 40.0),
            make_v_line(30.0, 10.0, 40.0),
            make_v_line(60.0, 10.0, 40.0),
            // Table A (upper): SAME column X-positions, rows y=[90,110],[110,130].
            // The gap y=[50,90] (40pt) carries no path at all. ~keep
            make_h_line(10.0, 90.0, 50.0),
            make_h_line(10.0, 110.0, 50.0),
            make_h_line(10.0, 130.0, 50.0),
            make_v_line(10.0, 90.0, 40.0),
            make_v_line(30.0, 90.0, 40.0),
            make_v_line(60.0, 90.0, 40.0),
        ];
        let spans = vec![
            create_test_span("B21", 12.0, 12.0, 8.0, 10.0),
            create_test_span("B22", 35.0, 12.0, 8.0, 10.0),
            create_test_span("B11", 12.0, 32.0, 8.0, 10.0),
            create_test_span("B12", 35.0, 32.0, 8.0, 10.0),
            // The section heading, printed inside the graphics-free gap. ~keep
            create_test_span("HEADINGTAG", 12.0, 60.0, 20.0, 10.0),
            create_test_span("A21", 12.0, 92.0, 8.0, 10.0),
            create_test_span("A22", 35.0, 92.0, 8.0, 10.0),
            create_test_span("A11", 12.0, 112.0, 8.0, 10.0),
            create_test_span("A12", 35.0, 112.0, 8.0, 10.0),
        ];
        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            min_table_cells: 2,
            min_table_columns: 2,
            ..TableDetectionConfig::default()
        };

        let tables = detect_tables_from_intersections(&spans, &lines, &config);

        let heading_rows: Vec<&TableRow> = tables
            .iter()
            .flat_map(|t| t.rows.iter())
            .filter(|r| r.cells.iter().any(|c| c.text.contains("HEADINGTAG")))
            .collect();
        assert!(
            heading_rows.is_empty(),
            "the heading printed in the graphics-free gap must not become a row of either \
             table (it must stay out of the element stream as a table row entirely), got: \
             {heading_rows:?}"
        );
        assert_eq!(
            tables.len(),
            2,
            "two disjoint same-column grids separated by a graphics-free gap must remain \
             two tables, not be bridged into one, got: {tables:?}"
        );
    }

    /// Negative control for xberg-io/xberg#1601: a heading sitting INSIDE a
    /// genuinely ruled band of ONE continuously-ruled table (real V edges span
    /// every row, including the heading's) must stay part of that one table.
    /// This is the shape the fix must not break — the issue's own page-2
    /// control.
    #[test]
    fn gh1601_heading_row_inside_a_ruled_band_stays_in_one_table() {
        let lines = vec![
            make_h_line(10.0, 10.0, 50.0),
            make_h_line(10.0, 30.0, 50.0),
            make_h_line(10.0, 50.0, 50.0),
            make_h_line(10.0, 70.0, 50.0),
            // V edges span the FULL height across all three row bands —
            // this is one continuously-ruled table, not two abutting ones. ~keep
            make_v_line(10.0, 10.0, 60.0),
            make_v_line(30.0, 10.0, 60.0),
            make_v_line(60.0, 10.0, 60.0),
        ];
        let spans = vec![
            create_test_span("R11", 12.0, 12.0, 8.0, 10.0),
            create_test_span("R12", 35.0, 12.0, 8.0, 10.0),
            // The heading occupies only the left column of the middle row —
            // a real row of the table, ruled above and below. ~keep
            create_test_span("HEADINGTAG", 12.0, 32.0, 8.0, 10.0),
            create_test_span("R21", 12.0, 52.0, 8.0, 10.0),
            create_test_span("R22", 35.0, 52.0, 8.0, 10.0),
        ];
        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            min_table_cells: 2,
            min_table_columns: 2,
            ..TableDetectionConfig::default()
        };

        let tables = detect_tables_from_intersections(&spans, &lines, &config);

        assert_eq!(
            tables.len(),
            1,
            "a heading inside a genuinely ruled band must not split the table apart, got: {tables:?}"
        );
        assert_eq!(
            tables[0].rows.len(),
            3,
            "all three ruled rows, including the heading's, must stay in the one table, got: {:?}",
            tables[0]
        );
        let heading_present = tables[0]
            .rows
            .iter()
            .any(|r| r.cells.iter().any(|c| c.text.contains("HEADINGTAG")));
        assert!(
            heading_present,
            "the heading row must still be present in the one table"
        );
    }

    /// xberg-io/xberg#1601 follow-up: the containment check in
    /// `build_cells_from_intersections` must use its OWN tolerance
    /// (`CELL_RULE_SPAN_TOL`), not the X-axis `BAND_RULE_SPAN_TOL` (1.0pt) —
    /// that value is too tight for legitimate per-cell rule padding. Direct,
    /// low-level test of the gate itself (not the full pipeline): a single
    /// row's V edges are drawn 3pt short of the row's true top and bottom on
    /// BOTH ends (a realistic per-cell rule inset), well inside "a few
    /// points" but past the old 1.0pt X-axis tolerance. The cell must still
    /// form.
    #[test]
    fn gh1601_cell_rule_inset_a_few_points_still_forms_the_cell() {
        let pts = vec![
            Intersection { x: 0.0, y: 0.0 },
            Intersection { x: 100.0, y: 0.0 },
            Intersection { x: 0.0, y: 20.0 },
            Intersection { x: 100.0, y: 20.0 },
        ];
        // Row is y=[0,20]; each V edge is inset 3pt from both ends
        // (y=[3,17]) instead of running the row's full height — ordinary
        // per-cell rule padding, not a phantom-table-bridging gap. ~keep
        let v_edges = [
            Edge {
                coord: 0.0,
                start: 3.0,
                end: 17.0,
            },
            Edge {
                coord: 100.0,
                start: 3.0,
                end: 17.0,
            },
        ];
        let cells = build_cells_from_intersections(&pts, &[], &v_edges);
        assert_eq!(
            cells.len(),
            1,
            "a V edge inset a few points from its own row's true boundary must still form the cell"
        );
    }

    /// Companion negative control, at the same low level: a 40pt gap between
    /// two row bands (the shape from `gh1601_graphics_free_gap_does_not_bridge_two_tables`,
    /// isolated to `build_cells_from_intersections` itself) must still be
    /// rejected under the new, looser `CELL_RULE_SPAN_TOL` (6.0pt) — the
    /// looser constant closes the false-negative gap on legitimate insets
    /// without reopening the phantom-cell bug it was introduced to fix.
    #[test]
    fn gh1601_cell_rule_span_tol_does_not_reopen_the_gap_bridge() {
        let pts = vec![
            Intersection { x: 0.0, y: 0.0 },
            Intersection { x: 100.0, y: 0.0 },
            Intersection { x: 0.0, y: 40.0 },
            Intersection { x: 100.0, y: 40.0 },
        ];
        // No V edge reaches anywhere near spanning y=[0,40] — each side's
        // edges only cover their own table's real rows, exactly like the
        // reporter's 54pt carrier gap and this file's 40pt fixture gap. ~keep
        let v_edges = [
            Edge {
                coord: 0.0,
                start: 0.0,
                end: 10.0,
            },
            Edge {
                coord: 100.0,
                start: 0.0,
                end: 10.0,
            },
            Edge {
                coord: 0.0,
                start: 30.0,
                end: 40.0,
            },
            Edge {
                coord: 100.0,
                start: 30.0,
                end: 40.0,
            },
        ];
        let cells = build_cells_from_intersections(&pts, &[], &v_edges);
        assert!(
            cells.is_empty(),
            "a 40pt graphics-free gap must still be rejected under the looser CELL_RULE_SPAN_TOL, got: {cells:?}"
        );
    }

    /// Zebra shading: fill-only rectangles on rows 0, 2 and 4 plus two full-height rules between
    /// column groups. Rows 1 and 3 have no side edge at the table's outer x (only the shaded
    /// neighbours' fill sides reach it), but the rules run through them, so they are rows of the
    /// grid and must keep their outer cells.
    #[test]
    fn zebra_unshaded_rows_keep_their_outer_cells() {
        let mut lines: Vec<crate::elements::PathContent> = [0.0, 40.0, 80.0]
            .into_iter()
            .map(|y| make_rect_path(10.0, y, 150.0, 20.0))
            .collect();
        lines.push(make_v_line(60.0, 0.0, 100.0));
        lines.push(make_v_line(110.0, 0.0, 100.0));
        let mut spans = Vec::new();
        for row in 0..5 {
            let y = 85.0 - row as f32 * 20.0;
            spans.push(create_test_span(&format!("L{row}"), 12.0, y, 8.0, 10.0));
            spans.push(create_test_span(&format!("M{row}"), 70.0, y, 8.0, 10.0));
            spans.push(create_test_span(&format!("R{row}"), 120.0, y, 8.0, 10.0));
        }
        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            min_table_cells: 2,
            min_table_columns: 2,
            ..TableDetectionConfig::default()
        };

        let tables = detect_tables_from_intersections(&spans, &lines, &config);

        assert_eq!(tables.len(), 1, "got: {tables:?}");
        let texts: Vec<Vec<&str>> = tables[0]
            .rows
            .iter()
            .map(|r| r.cells.iter().map(|c| c.text.trim()).collect())
            .collect();
        for row in 0..5 {
            let want = [format!("L{row}"), format!("M{row}"), format!("R{row}")];
            assert!(
                texts.iter().any(|cells| cells == &want),
                "row {row} must keep all three cells, got: {texts:?}"
            );
        }
    }

    #[test]
    fn cell_with_undrawn_sides_forms_when_a_rule_runs_through_its_band() {
        let edge = |coord, start, end| Edge { coord, start, end };
        let pts = vec![
            Intersection { x: 0.0, y: 0.0 },
            Intersection { x: 50.0, y: 0.0 },
            Intersection { x: 100.0, y: 0.0 },
            Intersection { x: 0.0, y: 20.0 },
            Intersection { x: 50.0, y: 20.0 },
            Intersection { x: 100.0, y: 20.0 },
        ];
        let h_edges = [edge(0.0, 0.0, 100.0), edge(20.0, 0.0, 100.0)];
        // The outer sides only reach the band's corners from the rows above and below. ~keep
        let v_edges = [
            edge(0.0, -20.0, 0.0),
            edge(0.0, 20.0, 40.0),
            edge(100.0, -20.0, 0.0),
            edge(100.0, 20.0, 40.0),
            edge(50.0, -20.0, 40.0),
        ];
        let cells = build_cells_from_intersections(&pts, &h_edges, &v_edges);
        assert_eq!(cells.len(), 2, "got: {cells:?}");
    }

    /// A full-width section row: the inner column rule stops at the row, and only the row's own
    /// outer rules span it.
    #[test]
    fn cell_with_an_undrawn_side_forms_when_rules_close_both_ends_of_its_band() {
        let edge = |coord, start, end| Edge { coord, start, end };
        let pts = vec![
            Intersection { x: 0.0, y: 0.0 },
            Intersection { x: 50.0, y: 0.0 },
            Intersection { x: 100.0, y: 0.0 },
            Intersection { x: 0.0, y: 20.0 },
            Intersection { x: 50.0, y: 20.0 },
            Intersection { x: 100.0, y: 20.0 },
        ];
        let h_edges = [edge(0.0, 0.0, 100.0), edge(20.0, 0.0, 100.0)];
        let v_edges = [
            edge(0.0, -20.0, 40.0),
            edge(100.0, -20.0, 40.0),
            edge(50.0, -20.0, 0.0),
            edge(50.0, 20.0, 40.0),
        ];
        let cells = build_cells_from_intersections(&pts, &h_edges, &v_edges);
        assert_eq!(cells.len(), 2, "got: {cells:?}");
    }

    #[test]
    fn cell_with_undrawn_sides_ignores_a_rule_at_or_past_the_ends_of_its_h_rules() {
        let edge = |coord, start, end| Edge { coord, start, end };
        let pts = vec![
            Intersection { x: 0.0, y: 0.0 },
            Intersection { x: 100.0, y: 0.0 },
            Intersection { x: 0.0, y: 40.0 },
            Intersection { x: 100.0, y: 40.0 },
        ];
        let h_edges = [edge(0.0, 0.0, 100.0), edge(40.0, 0.0, 100.0)];
        // Both rules span the gap without crossing it: an enclosing table's column rule 2.5pt
        // outside the H rules' left end (within SNAP_TOL), and a page frame at x=150. ~keep
        let v_edges = [
            edge(0.0, -20.0, 0.0),
            edge(0.0, 40.0, 60.0),
            edge(100.0, -20.0, 0.0),
            edge(100.0, 40.0, 60.0),
            edge(-2.5, -100.0, 200.0),
            edge(150.0, -100.0, 200.0),
        ];
        let cells = build_cells_from_intersections(&pts, &h_edges, &v_edges);
        assert!(cells.is_empty(), "got: {cells:?}");
    }

    #[test]
    fn test_intersection_rect_decomposition() {
        let lines = vec![crate::elements::PathContent::rect(10.0, 10.0, 100.0, 50.0)];
        let (h, v) = extract_edges(&lines);
        assert_eq!(h.len(), 2, "Rectangle should produce 2 horizontal edges");
        assert_eq!(v.len(), 2, "Rectangle should produce 2 vertical edges");
    }

    #[test]
    fn test_intersection_3x3_grid_produces_4_cells() {
        let pts = vec![
            Intersection { x: 0.0, y: 0.0 },
            Intersection { x: 50.0, y: 0.0 },
            Intersection { x: 100.0, y: 0.0 },
            Intersection { x: 0.0, y: 50.0 },
            Intersection { x: 50.0, y: 50.0 },
            Intersection { x: 100.0, y: 50.0 },
            Intersection { x: 0.0, y: 100.0 },
            Intersection { x: 50.0, y: 100.0 },
            Intersection { x: 100.0, y: 100.0 },
        ];
        // Real drawn V edges spanning the full grid height at every column. ~keep
        let v_edges = [
            Edge {
                coord: 0.0,
                start: 0.0,
                end: 100.0,
            },
            Edge {
                coord: 50.0,
                start: 0.0,
                end: 100.0,
            },
            Edge {
                coord: 100.0,
                start: 0.0,
                end: 100.0,
            },
        ];
        let cells = build_cells_from_intersections(&pts, &[], &v_edges);
        assert_eq!(cells.len(), 4, "3x3 grid should produce 4 cells");
        let groups = group_cells_into_tables(&cells);
        assert_eq!(groups.len(), 1, "All 4 cells should form 1 table");
    }

    #[test]
    fn test_dotted_line_reconstitution() {
        // 10 short H segments at y=300, each 3pt wide, spanning x=50..350
        // Each segment is below MIN_EDGE_LEN (5pt) so would normally be discarded.
        // The reconstitution pass should merge them into one edge from x=50 to x=350. ~keep
        let mut edges: Vec<Edge> = (0..10)
            .map(|i| Edge {
                coord: 300.0,
                start: 50.0 + i as f32 * 30.0,
                end: 53.0 + i as f32 * 30.0,
            })
            .collect();

        snap_and_merge(&mut edges);

        assert_eq!(edges.len(), 1, "Dotted segments should reconstitute into 1 edge");
        assert!(
            (edges[0].coord - 300.0).abs() < 0.01,
            "Reconstituted edge should be at y=300"
        );
        assert!(
            (edges[0].start - 50.0).abs() < 0.01,
            "Reconstituted edge should start at x=50"
        );
        assert!(
            (edges[0].end - 323.0).abs() < 0.01,
            "Reconstituted edge should end at x=323"
        );
    }

    #[test]
    fn test_dotted_line_too_few_segments_discarded() {
        // Only 2 short segments — below DOTTED_MIN_SEGMENTS threshold.
        // Should be discarded entirely (not reconstituted, not kept individually). ~keep
        let mut edges = vec![
            Edge {
                coord: 200.0,
                start: 10.0,
                end: 13.0,
            },
            Edge {
                coord: 200.0,
                start: 20.0,
                end: 23.0,
            },
        ];
        snap_and_merge(&mut edges);
        assert!(
            edges.is_empty(),
            "Two short segments should not be reconstituted or kept"
        );
    }

    #[test]
    fn test_dotted_line_narrow_span_discarded() {
        // 5 short segments at same coord but total span < DOTTED_MIN_SPAN (50pt).
        // Gaps between segments are > JOIN_TOL (3pt) so they won't be joined. ~keep
        let mut edges: Vec<Edge> = (0..5)
            .map(|i| Edge {
                coord: 400.0,
                start: 10.0 + i as f32 * 8.0,
                end: 13.0 + i as f32 * 8.0,
            })
            .collect();
        snap_and_merge(&mut edges);
        assert!(
            edges.is_empty(),
            "Short segments with narrow total span should be discarded"
        );
    }

    #[test]
    fn test_dotted_line_mixed_with_long_edges() {
        let mut edges = vec![Edge {
            coord: 100.0,
            start: 0.0,
            end: 200.0,
        }];
        for i in 0..10 {
            edges.push(Edge {
                coord: 300.0,
                start: 50.0 + i as f32 * 30.0,
                end: 53.0 + i as f32 * 30.0,
            });
        }
        snap_and_merge(&mut edges);
        assert_eq!(edges.len(), 2, "Long edge + reconstituted dotted line = 2 edges");
    }

    #[test]
    fn test_join_chain_of_short_segments() {
        let mut edges: Vec<Edge> = (0..10)
            .map(|i| Edge {
                coord: 100.0,
                start: i as f32 * 25.0,
                end: (i + 1) as f32 * 25.0,
            })
            .collect();

        snap_and_merge(&mut edges);

        assert_eq!(edges.len(), 1, "Chain of 10 touching H segments should join into 1");
        assert!((edges[0].start - 0.0).abs() < 0.01, "Joined edge should start at 0");
        assert!((edges[0].end - 250.0).abs() < 0.01, "Joined edge should end at 250");
    }

    #[test]
    fn test_join_tiny_vertical_segments() {
        let mut edges: Vec<Edge> = (0..10)
            .map(|i| Edge {
                coord: 50.0,
                start: i as f32 * 6.0,
                end: (i + 1) as f32 * 6.0,
            })
            .collect();

        snap_and_merge(&mut edges);

        assert_eq!(edges.len(), 1, "Chain of 10 touching V segments should join into 1");
        assert!((edges[0].start - 0.0).abs() < 0.01, "Joined edge should start at 0");
        assert!((edges[0].end - 60.0).abs() < 0.01, "Joined edge should end at 60");
    }

    #[test]
    fn test_join_segments_with_slightly_different_coords() {
        // Segments at very close but not identical coords (within SNAP_TOL)
        // should snap to the same coord and then join. ~keep
        let mut edges = vec![
            Edge {
                coord: 87.4,
                start: 36.0,
                end: 117.0,
            },
            Edge {
                coord: 87.41,
                start: 117.0,
                end: 143.0,
            },
            Edge {
                coord: 87.39,
                start: 143.0,
                end: 170.0,
            },
        ];

        snap_and_merge(&mut edges);

        assert_eq!(edges.len(), 1, "Segments at near-identical coords should snap and join");
        assert!((edges[0].start - 36.0).abs() < 0.01, "Joined edge should start at 36");
        assert!((edges[0].end - 170.0).abs() < 0.01, "Joined edge should end at 170");
    }

    #[test]
    fn rule_families_tied_on_first_coord_emit_in_input_order() {
        // Eight side-by-side families (disjoint x-ranges, so none unify),
        // every family's first rule at the same y: `a[0].coord` ties across
        // all of them, so only the root tiebreak orders the output. Under
        // the pre-fix HashMap grouping this order was per-process hash
        // order (8! arrangements), so this assertion fails on all but the
        // seed that happens to sort. ~keep
        let edges: Vec<Edge> = (0..8)
            .flat_map(|k| {
                let x0 = 200.0 * k as f32;
                [
                    Edge {
                        coord: 700.0,
                        start: x0,
                        end: x0 + 150.0,
                    },
                    Edge {
                        coord: 600.0,
                        start: x0,
                        end: x0 + 150.0,
                    },
                ]
            })
            .collect();
        let wide: Vec<&Edge> = edges.iter().collect();

        let families = x_coherent_rule_families(&wide);

        assert_eq!(families.len(), 8, "Disjoint x-ranges should stay separate families");
        for (k, family) in families.iter().enumerate() {
            assert_eq!(family.len(), 2);
            assert!(
                (family[0].coord - 700.0).abs() < 0.01,
                "First rule should be the top rule"
            );
            assert!(
                (family[0].start - 200.0 * k as f32).abs() < 0.01,
                "Tied families should emit in input order, family {k} out of place"
            );
        }
    }

    #[test]
    fn rule_families_with_distinct_first_coords_order_by_coord() {
        // Distinct `a[0].coord` values: the coord key alone decides, so the
        // input (and root) order must NOT leak through — the family given
        // second sorts first because its top rule has the smaller y. ~keep
        let edges = [
            Edge {
                coord: 500.0,
                start: 0.0,
                end: 150.0,
            },
            Edge {
                coord: 400.0,
                start: 0.0,
                end: 150.0,
            },
            Edge {
                coord: 300.0,
                start: 300.0,
                end: 450.0,
            },
            Edge {
                coord: 200.0,
                start: 300.0,
                end: 450.0,
            },
        ];
        let wide: Vec<&Edge> = edges.iter().collect();

        let families = x_coherent_rule_families(&wide);

        assert_eq!(families.len(), 2);
        assert!((families[0][0].coord - 300.0).abs() < 0.01);
        assert!((families[1][0].coord - 500.0).abs() < 0.01);
    }

    #[test]
    fn test_hybrid_line_cols_text_rows() {
        // V lines at x=50, 200, 400 (2 columns) spanning y=100..300
        // H lines at y=100 and y=300 only (top and bottom, NO middle rows)
        // This creates a single intersection-based row, but text lives at 3 Y positions. ~keep
        let lines = vec![
            make_h_line(50.0, 100.0, 350.0),
            make_h_line(50.0, 300.0, 350.0),
            make_v_line(50.0, 100.0, 200.0),
            make_v_line(200.0, 100.0, 200.0),
            make_v_line(400.0, 100.0, 200.0),
        ];
        let spans = vec![
            create_test_span("A", 60.0, 265.0, 20.0, 10.0),
            create_test_span("B", 210.0, 265.0, 20.0, 10.0),
            create_test_span("C", 60.0, 205.0, 20.0, 10.0),
            create_test_span("D", 210.0, 205.0, 20.0, 10.0),
            create_test_span("E", 60.0, 145.0, 20.0, 10.0),
            create_test_span("F", 210.0, 145.0, 20.0, 10.0),
        ];
        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            min_table_cells: 4,
            min_table_columns: 2,
            ..TableDetectionConfig::default()
        };
        let tables = detect_tables_with_lines(&spans, &lines, &config);
        assert_eq!(tables.len(), 1, "Should detect exactly 1 table");
        let table = &tables[0];
        assert_eq!(
            table.rows.len(),
            3,
            "Should have 3 rows (split from text Y positions), got {}",
            table.rows.len()
        );
        assert_eq!(table.col_count, 2, "Should have 2 columns");

        // Rows sorted top-to-bottom (descending Y in PDF coords). ~keep
        let r0: Vec<&str> = table.rows[0].cells.iter().map(|c| c.text.as_str()).collect();
        let r1: Vec<&str> = table.rows[1].cells.iter().map(|c| c.text.as_str()).collect();
        let r2: Vec<&str> = table.rows[2].cells.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(r0, vec!["A", "B"], "Top row should be A, B");
        assert_eq!(r1, vec!["C", "D"], "Middle row should be C, D");
        assert_eq!(r2, vec!["E", "F"], "Bottom row should be E, F");
    }

    #[test]
    fn test_strip_form_numbering_artifacts() {
        use crate::structure::table_extractor::{TableCell, TableRow};

        let make_cell = |text: &str| TableCell {
            text: text.to_string(),
            spans: Vec::new(),
            colspan: 1,
            rowspan: 1,
            mcids: Vec::new(),
            bbox: None,
            is_header: false,
        };

        let mut rows = vec![
            // Row 0: all single-digit -> should be removed entirely ~keep
            TableRow {
                cells: vec![make_cell("5"), make_cell(""), make_cell(""), make_cell("")],
                is_header: false,
            },
            // Row 1: digit prefix artifacts -> should be stripped ~keep
            TableRow {
                cells: vec![
                    make_cell("1 Apr 11, 2025"),
                    make_cell("1 12111 - Rinse-Fluoride Treatment"),
                    make_cell("1 $14.60"),
                    make_cell("1"),
                ],
                is_header: false,
            },
            // Row 2: no artifacts -> unchanged ~keep
            TableRow {
                cells: vec![
                    make_cell("Apr 11, 2025"),
                    make_cell("11101 - One unit of time"),
                    make_cell("$47.60"),
                    make_cell(""),
                ],
                is_header: false,
            },
            // Row 3: digit prefix but remainder starts with digit -> NOT stripped ~keep
            TableRow {
                cells: vec![make_cell("3 items"), make_cell(""), make_cell(""), make_cell("")],
                is_header: false,
            },
        ];

        strip_form_numbering_artifacts(&mut rows);

        assert_eq!(rows.len(), 3, "Single-digit-only row should be removed");

        let r0: Vec<&str> = rows[0].cells.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(r0[0], "Apr 11, 2025", "Leading '1 ' should be stripped");
        assert_eq!(
            r0[1], "12111 - Rinse-Fluoride Treatment",
            "Leading '1 ' stripped, rest starts with digit but contains '-'"
        );
        assert_eq!(r0[2], "$14.60", "Leading '1 ' stripped, rest starts with '$'");
        // "1" alone is cleared in Phase 3 because other cells in this row were stripped. ~keep
        assert_eq!(r0[3], "", "Lone '1' cleared when other cells in row were stripped");

        let r1: Vec<&str> = rows[1].cells.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(r1[0], "Apr 11, 2025");
        assert_eq!(r1[1], "11101 - One unit of time");
        assert_eq!(r1[2], "$47.60");

        // Former row 3 is now row 2: "3 items" should NOT be stripped because
        // the remainder ("items") is a plain word with no date/code indicators. ~keep
        let r2: Vec<&str> = rows[2].cells.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(
            r2[0], "3 items",
            "'3 items' should NOT be stripped (plain word, no date/code/currency)"
        );
    }

    #[test]
    fn test_strip_dash_separator_cells() {
        // T5 summary table: "------" appears as a decorative line separator
        // in a cell. After stripping, that cell should be empty. ~keep
        use crate::structure::table_extractor::{TableCell, TableRow};

        let make_cell = |text: &str| TableCell {
            text: text.to_string(),
            spans: Vec::new(),
            colspan: 1,
            rowspan: 1,
            mcids: Vec::new(),
            bbox: None,
            is_header: false,
        };

        let mut rows = vec![
            TableRow {
                cells: vec![make_cell("------"), make_cell("Total"), make_cell("$500.00")],
                is_header: false,
            },
            TableRow {
                cells: vec![make_cell("____"), make_cell("Subtotal"), make_cell("$200.00")],
                is_header: false,
            },
            TableRow {
                cells: vec![make_cell("--__--"), make_cell("Tax"), make_cell("$10.00")],
                is_header: false,
            },
            TableRow {
                cells: vec![make_cell("------"), make_cell("---"), make_cell("------")],
                is_header: false,
            },
            TableRow {
                cells: vec![make_cell("2025-01-01"), make_cell("Payment"), make_cell("$100.00")],
                is_header: false,
            },
        ];

        strip_form_numbering_artifacts(&mut rows);

        assert_eq!(rows[0].cells[0].text.trim(), "", "Dash-only cell should be cleared");
        assert_eq!(rows[0].cells[1].text, "Total");
        assert_eq!(rows[0].cells[2].text, "$500.00");

        assert_eq!(
            rows[1].cells[0].text.trim(),
            "",
            "Underscore-only cell should be cleared"
        );

        assert_eq!(
            rows[2].cells[0].text.trim(),
            "",
            "Mixed dash/underscore cell should be cleared"
        );

        // Row with all-dash cells becomes all-empty (kept for downstream
        // empty-row splitting, which uses empty rows as table separators). ~keep
        assert_eq!(rows.len(), 5, "All-dash row kept as empty separator");
        assert!(
            rows[3].cells.iter().all(|c| c.text.trim().is_empty()),
            "All-dash row should now be all-empty"
        );

        assert_eq!(rows[4].cells[0].text, "2025-01-01");
    }

    #[test]
    fn test_separate_small_and_large_table_clusters() {
        let lines = vec![
            make_rect_path(409.0, 83.0, 125.0, 0.5),
            make_rect_path(409.0, 142.0, 125.0, 0.5),
            make_rect_path(409.0, 71.0, 0.5, 72.0),
            make_rect_path(534.0, 71.0, 0.5, 72.0),
            make_rect_path(22.0, 150.0, 567.0, 0.5),
            make_rect_path(22.0, 553.0, 567.0, 0.5),
            make_rect_path(22.0, 150.0, 0.5, 403.0),
            make_rect_path(490.0, 150.0, 0.5, 403.0),
            make_rect_path(589.0, 150.0, 0.5, 403.0),
        ];

        let config = TableDetectionConfig::default();
        let clusters = group_lines_into_clusters(&lines, &config);
        assert!(
            clusters.len() >= 2,
            "Expected at least 2 clusters (header table + main table), got {}",
            clusters.len()
        );

        for cluster in &clusters {
            let mut has_header_vline = false;
            let mut has_main_vline = false;
            for &idx in &cluster.lines {
                let bbox = &lines[idx].bbox;
                if bbox.width.abs() < 2.0 && bbox.height.abs() > 5.0 {
                    let y_max = bbox.y + bbox.height;
                    if y_max < 145.0 {
                        has_header_vline = true;
                    }
                    if bbox.y >= 149.0 {
                        has_main_vline = true;
                    }
                }
            }
            assert!(
                !(has_header_vline && has_main_vline),
                "A single cluster should not contain both header V-lines (y<145) and main V-lines (y>149)"
            );
        }
    }

    /// (C) A ruled 3-column grid whose header text row sits just ABOVE the top
    /// ruling (unruled header). Its three labels align to the three detected
    /// columns and span the table width, so the row must be pulled in as the
    /// table's header (cells preserved, not merged, marked as header).
    #[test]
    fn test_ws03b_header_row_above_included() {
        let lines = vec![
            make_h_line(100.0, 560.0, 300.0),
            make_h_line(100.0, 530.0, 300.0),
            make_h_line(100.0, 500.0, 300.0),
            make_v_line(100.0, 500.0, 60.0),
            make_v_line(200.0, 500.0, 60.0),
            make_v_line(300.0, 500.0, 60.0),
            make_v_line(400.0, 500.0, 60.0),
        ];
        let spans = vec![
            create_test_span("Name", 140.0, 568.0, 20.0, 10.0),
            create_test_span("Age", 240.0, 568.0, 20.0, 10.0),
            create_test_span("City", 340.0, 568.0, 20.0, 10.0),
            create_test_span("Ann", 145.0, 545.0, 10.0, 10.0),
            create_test_span("30", 245.0, 545.0, 10.0, 10.0),
            create_test_span("NYC", 345.0, 545.0, 10.0, 10.0),
            create_test_span("Bob", 145.0, 515.0, 10.0, 10.0),
            create_test_span("41", 245.0, 515.0, 10.0, 10.0),
            create_test_span("LA", 345.0, 515.0, 10.0, 10.0),
        ];
        let config = TableDetectionConfig::default();
        let clusters = group_lines_into_clusters(&lines, &config);
        assert_eq!(clusters.len(), 1, "single ruled grid → one cluster");
        let tables = detect_tables_in_cluster(&spans, &lines, &clusters[0], &config);
        assert_eq!(tables.len(), 1, "one table expected");
        let table = &tables[0];
        assert_eq!(
            table.rows.len(),
            3,
            "header row above the top ruling must be included (2 body + 1 header)"
        );
        assert!(table.has_header, "table must be flagged as having a header");
        assert!(table.rows[0].is_header, "row 0 must be the header row");
        let header_text: String = table.rows[0]
            .cells
            .iter()
            .map(|c| c.text.trim())
            .collect::<Vec<_>>()
            .join("|");
        for label in ["Name", "Age", "City"] {
            assert!(
                header_text.contains(label),
                "header row must contain {label:?}, got {header_text:?}"
            );
        }
    }

    /// (C-negative) Unrelated text above the same grid at an x OUTSIDE the column
    /// extent does NOT align to any column, so no header row is added and the
    /// stray text is left out of the table (table unchanged: 2 body rows only).
    #[test]
    fn test_ws03b_unaligned_text_above_not_header() {
        let lines = vec![
            make_h_line(100.0, 560.0, 300.0),
            make_h_line(100.0, 530.0, 300.0),
            make_h_line(100.0, 500.0, 300.0),
            make_v_line(100.0, 500.0, 60.0),
            make_v_line(200.0, 500.0, 60.0),
            make_v_line(300.0, 500.0, 60.0),
            make_v_line(400.0, 500.0, 60.0),
        ];
        let spans = vec![
            create_test_span("Table", 520.0, 568.0, 20.0, 10.0),
            create_test_span("caption", 560.0, 568.0, 40.0, 10.0),
            create_test_span("Ann", 145.0, 545.0, 10.0, 10.0),
            create_test_span("30", 245.0, 545.0, 10.0, 10.0),
            create_test_span("NYC", 345.0, 545.0, 10.0, 10.0),
            create_test_span("Bob", 145.0, 515.0, 10.0, 10.0),
            create_test_span("41", 245.0, 515.0, 10.0, 10.0),
            create_test_span("LA", 345.0, 515.0, 10.0, 10.0),
        ];
        let config = TableDetectionConfig::default();

        let mut row_ys = vec![560.0_f32, 530.0, 500.0];
        row_ys.sort_by(|a, b| crate::utils::safe_float_cmp(*b, *a));
        let col_xs = vec![100.0_f32, 200.0, 300.0, 400.0];
        assert!(
            detect_header_row_above(&spans, &row_ys, &col_xs).is_none(),
            "text outside the column extent must not be treated as a header row"
        );

        let clusters = group_lines_into_clusters(&lines, &config);
        let tables = detect_tables_in_cluster(&spans, &lines, &clusters[0], &config);
        assert_eq!(tables.len(), 1);
        assert_eq!(
            tables[0].rows.len(),
            2,
            "no header row must be added for unaligned text above the grid"
        );
        let all_text: String = tables[0]
            .rows
            .iter()
            .flat_map(|r| r.cells.iter())
            .map(|c| c.text.clone())
            .collect();
        assert!(
            !all_text.contains("caption"),
            "unaligned caption text must stay out of the table, got {all_text:?}"
        );
    }

    #[test]
    fn test_text_edge_columns_form_layout() {
        // Simulate a form layout: text aligns to specific X positions across
        // many rows, but each row may only use a subset of columns.
        //
        // Column 1 (employer info):  left edge ~48
        // Column 2 (box codes):      left edge ~210
        // Column 3 (values):         left edge ~382
        // Column 4 (values):         left edge ~516
        //
        // We place 5+ spans at each column's X across different Y rows. ~keep

        let mut spans = Vec::new();
        let col_xs = [48.0_f32, 210.0, 382.0, 516.0];
        let row_ys = [700.0_f32, 680.0, 660.0, 640.0, 620.0, 600.0];

        for &cx in &col_xs {
            for &ry in &row_ys {
                spans.push(create_test_span("val", cx, ry, 40.0, 10.0));
            }
        }

        // Add some "noise" spans that only appear in 1-2 rows (should NOT
        // create extra columns). ~keep
        spans.push(create_test_span("noise", 130.0, 700.0, 20.0, 10.0));
        spans.push(create_test_span("noise", 132.0, 680.0, 20.0, 10.0));

        let config = TableDetectionConfig::default();
        let columns = detect_text_edge_columns(&spans, &config);

        // We expect roughly 4 column clusters (one per alignment edge),
        // possibly a few more for right-edges that also recur, but definitely
        // not 8+ as greedy clustering would produce. ~keep
        assert!(
            columns.len() >= 3 && columns.len() <= 6,
            "Expected 3-6 text-edge columns, got {}",
            columns.len()
        );

        let centres: Vec<f32> = columns.iter().map(|c| c.x_center).collect();
        for &expected_x in &col_xs {
            assert!(
                centres
                    .iter()
                    .any(|&cx| (cx - expected_x).abs() < config.column_tolerance
                        || (cx - (expected_x + 40.0)).abs() < config.column_tolerance),
                "Expected a column near x={expected_x} (or its right edge), centres={centres:?}"
            );
        }
    }

    #[test]
    fn test_text_edge_columns_noise_filtered() {
        let spans = vec![
            // Only 1 span at x=100 — below the min_row_count=3 threshold ~keep
            create_test_span("a", 100.0, 500.0, 30.0, 10.0),
            create_test_span("c", 300.0, 500.0, 30.0, 10.0),
            create_test_span("d", 300.0, 480.0, 30.0, 10.0),
            create_test_span("e", 300.0, 460.0, 30.0, 10.0),
            create_test_span("f", 300.0, 440.0, 30.0, 10.0),
        ];

        let config = TableDetectionConfig::default();
        let columns = detect_text_edge_columns(&spans, &config);

        assert!(!columns.is_empty(), "Should produce at least one column from x=300");
        for c in &columns {
            assert!(
                (c.x_center - 100.0).abs() > 15.0,
                "x=100 edge should have been filtered (only 1 row), but got column at {}",
                c.x_center
            );
        }
    }

    #[test]
    fn test_text_edge_fallback_integration() {
        // When greedy detect_columns produces >6 columns, detect_tables_from_spans
        // should fall back to text-edge detection and produce fewer columns.
        //
        // Build a layout with 4 true alignment columns but noisy X offsets
        // that cause greedy clustering (tolerance=15) to split them. ~keep
        let mut spans = Vec::new();
        let true_cols = [50.0_f32, 200.0, 350.0, 500.0];
        let row_ys = [700.0_f32, 680.0, 660.0, 640.0, 620.0];

        for (ci, &cx) in true_cols.iter().enumerate() {
            for (ri, &ry) in row_ys.iter().enumerate() {
                // Add slight jitter that stays within snap_tolerance but could
                // push greedy clustering into creating extra columns when
                // combined with different-width spans. ~keep
                let jitter = ((ci + ri) % 3) as f32 * 2.0;
                spans.push(create_test_span("v", cx + jitter, ry, 30.0, 10.0));
            }
        }

        // Also add extra scattered spans at unique X positions (each only in
        // 1 row) to bloat the greedy column count past 6. ~keep
        for i in 0..10 {
            let x = 80.0 + i as f32 * 30.0;
            spans.push(create_test_span("x", x, 700.0, 15.0, 10.0));
        }

        let config = TableDetectionConfig {
            column_tolerance: 8.0, // tight tolerance to force many greedy columns ~keep
            ..TableDetectionConfig::default()
        };

        let greedy_cols = detect_columns(&spans, config.column_tolerance, config.column_merge_threshold);
        assert!(
            greedy_cols.len() > 6,
            "Precondition: greedy should produce >6 columns, got {}",
            greedy_cols.len()
        );

        let te_cols = detect_text_edge_columns(&spans, &config);
        assert!(
            te_cols.len() < greedy_cols.len(),
            "Text-edge should produce fewer columns ({}) than greedy ({})",
            te_cols.len(),
            greedy_cols.len()
        );
    }

    #[test]
    fn test_reject_table_with_too_many_empty_cells() {
        use crate::structure::table_extractor::{Table, TableCell, TableRow};
        let col_count = 12;
        let mut rows = Vec::new();
        let mut header = TableRow::new(true);
        for c in 0..col_count {
            header.cells.push(TableCell {
                text: if c < 3 { format!("H{c}") } else { String::new() },
                spans: Vec::new(),
                colspan: 1,
                rowspan: 1,
                mcids: vec![],
                bbox: None,
                is_header: true,
            });
        }
        rows.push(header);
        for r in 0..4 {
            let mut row = TableRow::new(false);
            for c in 0..col_count {
                row.cells.push(TableCell {
                    text: if c < 2 { format!("R{r}C{c}") } else { String::new() },
                    spans: Vec::new(),
                    colspan: 1,
                    rowspan: 1,
                    mcids: vec![],
                    bbox: None,
                    is_header: false,
                });
            }
            rows.push(row);
        }
        let table = Table {
            rows,
            has_header: true,
            col_count,
            bbox: None,
        };
        // 5 rows * 12 cols = 60 total, 3 + 4*2 = 11 filled, 49 empty → 81.7% empty ~keep
        assert!(
            !is_valid_table(&table),
            "Table with >60% empty cells should be rejected"
        );
    }

    #[test]
    fn test_valid_table_passes_validation() {
        use crate::structure::table_extractor::{Table, TableCell, TableRow};
        let col_count = 3;
        let mut rows = Vec::new();
        for r in 0..4 {
            let mut row = TableRow::new(r == 0);
            for c in 0..col_count {
                row.cells.push(TableCell {
                    text: format!("R{r}C{c}"),
                    spans: Vec::new(),
                    colspan: 1,
                    rowspan: 1,
                    mcids: vec![],
                    bbox: None,
                    is_header: r == 0,
                });
            }
            rows.push(row);
        }
        let table = Table {
            rows,
            has_header: true,
            col_count,
            bbox: None,
        };
        assert!(is_valid_table(&table), "Well-populated table should pass validation");
    }

    /// Product data sheets have label/value rows that look like 2-column
    /// tables to the spatial detector (key text on the left, value on
    /// the right, with faint cell backgrounds). When the right-hand
    /// value wraps, the detector emits a continuation row whose left
    /// cell is empty — the hallmark of this false positive. Such tables
    /// must be rejected so their rows remain in the flow text.
    #[test]
    fn test_narrow_shallow_table_rejected_as_false_positive() {
        use crate::structure::table_extractor::{Table, TableCell, TableRow};
        let col_count = 2;
        let rows_data: Vec<(&str, &str)> = vec![
            ("Temperature resistance", "adhered to aluminium, -56° C to +82° C"),
            (
                "Resistance to cleaning agents",
                "adhered to aluminium, 8 h in solution (0.5% household",
            ),
            // Wrapping continuation → empty left cell. ~keep
            ("", "cleaning agents) at room temperature and 65° C, no"),
        ];
        let mut rows = Vec::new();
        for (label, value) in &rows_data {
            let mut row = TableRow::new(false);
            row.cells.push(TableCell {
                text: label.to_string(),
                spans: Vec::new(),
                colspan: 1,
                rowspan: 1,
                mcids: vec![],
                bbox: None,
                is_header: false,
            });
            row.cells.push(TableCell {
                text: value.to_string(),
                spans: Vec::new(),
                colspan: 1,
                rowspan: 1,
                mcids: vec![],
                bbox: None,
                is_header: false,
            });
            rows.push(row);
        }
        let table = Table {
            rows,
            has_header: false,
            col_count,
            bbox: None,
        };
        assert!(
            !is_valid_table(&table),
            "Narrow 2-column 'table' with an empty continuation cell must \
             be rejected so its rows stay in the flow text"
        );
    }

    /// A 2-column data table with enough filled rows is a real table
    /// and must continue to pass validation. Pins the threshold so the
    /// narrow-table guard does not regress genuine two-column tables.
    #[test]
    fn test_narrow_deep_table_still_accepted() {
        use crate::structure::table_extractor::{Table, TableCell, TableRow};
        let col_count = 2;
        let mut rows = Vec::new();
        for i in 0..6 {
            let mut row = TableRow::new(i == 0);
            row.cells.push(TableCell {
                text: format!("Key {i}"),
                spans: Vec::new(),
                colspan: 1,
                rowspan: 1,
                mcids: vec![],
                bbox: None,
                is_header: i == 0,
            });
            row.cells.push(TableCell {
                text: format!("Value {i}"),
                spans: Vec::new(),
                colspan: 1,
                rowspan: 1,
                mcids: vec![],
                bbox: None,
                is_header: i == 0,
            });
            rows.push(row);
        }
        let table = Table {
            rows,
            has_header: true,
            col_count,
            bbox: None,
        };
        assert!(
            is_valid_table(&table),
            "A 2-col × 6-row data table should still be accepted"
        );
    }

    /// A sparse 2-column table with a missing value on the right is a
    /// legitimate pattern (key/value lists, form layouts, "N/A" rows) and
    /// must NOT match the narrow-table false-positive signature, which
    /// targets empty-LEFT / filled-RIGHT continuation rows specifically.
    #[test]
    fn test_narrow_sparse_table_with_missing_right_value_accepted() {
        use crate::structure::table_extractor::{Table, TableCell, TableRow};
        let col_count = 2;
        let rows_data: Vec<(&str, &str)> = vec![
            ("Name", "ACME Corp"),
            ("Registration", "12345"),
            ("Fax", ""),
            ("Email", "info@example.com"),
        ];
        let mut rows = Vec::new();
        for (label, value) in &rows_data {
            let mut row = TableRow::new(false);
            row.cells.push(TableCell {
                text: label.to_string(),
                spans: Vec::new(),
                colspan: 1,
                rowspan: 1,
                mcids: vec![],
                bbox: None,
                is_header: false,
            });
            row.cells.push(TableCell {
                text: value.to_string(),
                spans: Vec::new(),
                colspan: 1,
                rowspan: 1,
                mcids: vec![],
                bbox: None,
                is_header: false,
            });
            rows.push(row);
        }
        let table = Table {
            rows,
            has_header: false,
            col_count,
            bbox: None,
        };
        assert!(
            is_valid_table(&table),
            "A 2-col table with a missing right-hand value but no empty-left \
             continuation row must still validate"
        );
    }

    #[test]
    fn test_text_only_tables_capped_at_max_columns() {
        let mut spans = Vec::new();
        let col_xs = [50.0_f32, 100.0, 150.0, 200.0, 250.0, 300.0, 350.0, 400.0];
        let row_ys = [700.0_f32, 680.0, 660.0, 640.0, 620.0];

        for &cx in &col_xs {
            for &ry in &row_ys {
                spans.push(create_test_span("val", cx, ry, 30.0, 10.0));
            }
        }

        // Use tight tolerance so each X position becomes its own column,
        // and cap at 6 columns via config. ~keep
        let config = TableDetectionConfig {
            column_tolerance: 5.0,
            column_merge_threshold: 8.0,
            max_table_columns: 6,
            ..TableDetectionConfig::default()
        };

        let tables = detect_tables_from_spans(&spans, &config);
        assert!(
            tables.is_empty(),
            "Text-only table with 8 columns should be rejected (max_table_columns=6), got {} table(s)",
            tables.len()
        );
    }

    #[test]
    fn test_extended_grid_when_lines_dont_cross() {
        // H-lines at y=100 and y=50 spanning full width x=0..500
        // V-lines at y=300..350 at x=0, x=100, x=200
        // These lines don't physically cross but should produce
        // a 1-row x 2-col grid via extended intersections. ~keep
        let lines = vec![
            make_h_line(0.0, 100.0, 500.0),
            make_h_line(0.0, 50.0, 500.0),
            make_v_line(0.0, 300.0, 50.0),
            make_v_line(100.0, 300.0, 50.0),
            make_v_line(200.0, 300.0, 50.0),
        ];

        let spans = vec![
            create_test_span("A", 30.0, 70.0, 20.0, 10.0),
            create_test_span("B", 130.0, 70.0, 20.0, 10.0),
        ];

        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            min_table_cells: 2,
            min_table_columns: 2,
            ..TableDetectionConfig::default()
        };

        let tables = detect_tables_from_intersections(&spans, &lines, &config);
        assert!(
            !tables.is_empty(),
            "Extended grid should produce at least one table when H and V lines don't cross"
        );
        let table = &tables[0];
        assert!(
            table.col_count >= 2,
            "Extended grid table should have at least 2 columns, got {}",
            table.col_count
        );
    }

    // ========================================================================
    // GH#1358: sideways table grid re-orientation
    // ======================================================================== ~keep

    /// Builds the ruled lines and text spans for a 2-physical-row x
    /// 3-physical-column grid: 3 H-lines at y=0/20/40 (each spanning the full
    /// x=0..90 width) crossed by 4 V-lines at x=0/30/60/90 (each spanning the
    /// full y=0..40 height), with one span per cell whose text encodes its
    /// *physical* (page-space) row/col as `"R{row}C{col}"` (row 0 = physical
    /// top, i.e. the y=20..40 band; col 0 = physical left, i.e. x=0..30).
    /// Every span gets `rotation_degrees` set to `rotation`.
    fn rotated_grid_fixture(rotation: f32) -> (Vec<crate::elements::PathContent>, Vec<TextSpan>) {
        let lines = vec![
            make_h_line(0.0, 0.0, 90.0),
            make_h_line(0.0, 20.0, 90.0),
            make_h_line(0.0, 40.0, 90.0),
            make_v_line(0.0, 0.0, 40.0),
            make_v_line(30.0, 0.0, 40.0),
            make_v_line(60.0, 0.0, 40.0),
            make_v_line(90.0, 0.0, 40.0),
        ];
        // Column centres 15/45/75 (30pt-wide columns), row centres 30 (top
        // band y=20..40) / 10 (bottom band y=0..20); 10x10 spans centred on
        // each. ~keep
        let col_left_x = [10.0_f32, 40.0, 70.0];
        let row_bottom_y = [25.0_f32, 5.0]; // row 0 (top) then row 1 (bottom) ~keep
        let mut spans = Vec::new();
        for (row, &y) in row_bottom_y.iter().enumerate() {
            for (col, &x) in col_left_x.iter().enumerate() {
                let text = format!("R{row}C{col}");
                spans.push(TextSpan {
                    rotation_degrees: rotation,
                    ..create_test_span(&text, x, y, 10.0, 10.0)
                });
            }
        }
        (lines, spans)
    }

    fn rotated_grid_config() -> TableDetectionConfig {
        TableDetectionConfig {
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            min_table_cells: 2,
            min_table_columns: 2,
            ..TableDetectionConfig::default()
        }
    }

    /// GH#1358 positive case: a ruled table whose spans are uniformly rotated
    /// 90° must have its row/column axes transposed into the table's own
    /// frame, not left bucketed on the page's physical top-to-bottom /
    /// left-to-right axes.
    ///
    /// Against unfixed code (`BUCKET_TABLE_GRID_IN_ROTATED_FRAME = false`,
    /// or equivalently deleting the `reorient_table_to_rotated_frame` call),
    /// this table comes out as the physical 2-row x 3-col grid — row 0 is
    /// `["R0C0", "R0C1", "R0C2"]` and `col_count` is `3` — so the first
    /// assertion (`col_count == 2`) fails with `col_count == 3`, and the
    /// second (row 0 is `["R1C0", "R0C0"]`) fails with row 0 actually being
    /// `["R0C0", "R0C1", "R0C2"]`.
    #[test]
    fn detect_tables_from_intersections_transposes_90_degree_rotated_table_grid() {
        let (lines, spans) = rotated_grid_fixture(90.0);
        let config = rotated_grid_config();

        let tables = detect_tables_from_intersections(&spans, &lines, &config);
        assert_eq!(tables.len(), 1, "expected exactly one detected table");
        let table = &tables[0];

        assert_eq!(
            table.col_count, 2,
            "a 90-degree table's 3 physical columns must become 2 table columns \
             (the physical row count), got {}",
            table.col_count
        );
        assert_eq!(
            table.rows.len(),
            3,
            "expected 3 rows after transposing 2 physical rows into 3"
        );

        let row_texts: Vec<Vec<&str>> = table
            .rows
            .iter()
            .map(|r| r.cells.iter().map(|c| c.text.as_str()).collect())
            .collect();
        assert_eq!(
            row_texts,
            vec![vec!["R1C0", "R0C0"], vec!["R1C1", "R0C1"], vec!["R1C2", "R0C2"],],
            "90-degree rotation must rotate the grid 90 degrees clockwise \
             (new row = old column ascending, new column = old row reversed); got {row_texts:?}"
        );
    }

    /// GH#1358 regression case: an ordinary upright table (spans at the
    /// default `rotation_degrees = 0.0`) must produce the exact physical
    /// grid it always did — this is the "0-degree path is bit-for-bit
    /// unchanged" guarantee, exercised through the same fixture geometry as
    /// the 90-degree test above so the only variable is rotation.
    #[test]
    fn detect_tables_from_intersections_leaves_upright_table_grid_unchanged() {
        let (lines, spans) = rotated_grid_fixture(0.0);
        let config = rotated_grid_config();

        let tables = detect_tables_from_intersections(&spans, &lines, &config);
        assert_eq!(tables.len(), 1, "expected exactly one detected table");
        let table = &tables[0];

        assert_eq!(table.col_count, 3, "an upright table's column count must not change");
        assert_eq!(table.rows.len(), 2, "an upright table's row count must not change");

        let row_texts: Vec<Vec<&str>> = table
            .rows
            .iter()
            .map(|r| r.cells.iter().map(|c| c.text.as_str()).collect())
            .collect();
        assert_eq!(
            row_texts,
            vec![vec!["R0C0", "R0C1", "R0C2"], vec!["R1C0", "R1C1", "R1C2"]],
            "an upright table must keep its physical top-to-bottom, \
             left-to-right row/column order; got {row_texts:?}"
        );
    }

    /// GH#1358: `dominant_table_rotation_quadrant` must require a *strict*
    /// majority, not merely a plurality — a table whose spans split evenly
    /// between two quadrants has no confident rotation and must fall back to
    /// `0.0` (upright) rather than guess. This directly exercises the
    /// "mixed case" requirement: disagreement is treated honestly (upright
    /// fallback), not resolved by silently picking whichever quadrant sorts
    /// first.
    ///
    /// Against a naive plurality-vote implementation (`count >= total / 2`
    /// or `max_by_key` alone, no strict-majority check), the tied case below
    /// would return `90.0` (quadrant index 1 beats index 0 on the
    /// `Reverse(index)` tie-break used for genuine ties within one count),
    /// not `0.0`.
    #[test]
    fn dominant_table_rotation_quadrant_requires_strict_majority_not_plurality() {
        let cell = |texts_and_rotations: &[(&str, f32)]| TableCell {
            text: String::new(),
            spans: texts_and_rotations
                .iter()
                .map(|&(text, rotation)| TextSpan {
                    rotation_degrees: rotation,
                    ..create_test_span(text, 0.0, 0.0, 10.0, 10.0)
                })
                .collect(),
            colspan: 1,
            rowspan: 1,
            mcids: Vec::new(),
            bbox: None,
            is_header: false,
        };

        // Exactly 2 spans at 0 degrees and 2 at 90 degrees: a tie, no strict
        // majority. Must fall back to upright. ~keep
        let tied = Table {
            rows: vec![TableRow {
                cells: vec![cell(&[("a", 0.0), ("b", 0.0)]), cell(&[("c", 90.0), ("d", 90.0)])],
                is_header: false,
            }],
            has_header: false,
            col_count: 2,
            bbox: None,
        };
        assert_eq!(
            dominant_table_rotation_quadrant(&tied),
            0.0,
            "a 2-vs-2 tie between quadrants must not be resolved into a rotation"
        );

        // 3 spans at 90 degrees vs 1 at 0 degrees: a strict majority (75%). ~keep
        let majority = Table {
            rows: vec![TableRow {
                cells: vec![cell(&[("a", 90.0), ("b", 90.0)]), cell(&[("c", 90.0), ("d", 0.0)])],
                is_header: false,
            }],
            has_header: false,
            col_count: 2,
            bbox: None,
        };
        assert_eq!(
            dominant_table_rotation_quadrant(&majority),
            90.0,
            "a 3-vs-1 strict majority must select the majority quadrant"
        );
    }

    // ========================================================================
    // Borderless (text-clustering) counterpart of the GH#1358 sideways-table
    // fix: `detect_tables_from_spans` has no ruling lines to define grid
    // geometry, so its columns/rows come from `detect_columns`/`detect_rows`
    // tolerance-clustering on physical page X/Y directly -- but the same
    // "physical bucketing is right, axis labelling is wrong for a rotated
    // table" defect applies, and is fixed the same way: a post-process
    // reorientation of the finished `Table`. ~keep
    // ======================================================================== ~keep

    /// Same physical layout as `rotated_grid_fixture` (2 physical rows x 3
    /// physical columns, one span per cell named by its *physical*
    /// `"R{row}C{col}"` position) but with no ruling lines at all, so it
    /// exercises the text-clustering path (`detect_tables_from_spans`)
    /// instead of the intersection path. Every span gets `rotation_degrees`
    /// set to `rotation`.
    fn borderless_rotated_grid_fixture(rotation: f32) -> Vec<TextSpan> {
        let col_left_x = [10.0_f32, 40.0, 70.0];
        let row_bottom_y = [25.0_f32, 5.0]; // row 0 (top) then row 1 (bottom) ~keep
        let mut spans = Vec::new();
        for (row, &y) in row_bottom_y.iter().enumerate() {
            for (col, &x) in col_left_x.iter().enumerate() {
                let text = format!("R{row}C{col}");
                spans.push(TextSpan {
                    rotation_degrees: rotation,
                    ..create_test_span(&text, x, y, 10.0, 10.0)
                });
            }
        }
        spans
    }

    /// Borderless positive case: a text-clustered table whose spans are
    /// uniformly rotated 90° must have its row/column axes transposed into
    /// the table's own frame, exactly like the ruled-line case in
    /// `detect_tables_from_intersections_transposes_90_degree_rotated_table_grid`.
    ///
    /// Against unfixed code (`BUCKET_BORDERLESS_TABLE_GRID_IN_ROTATED_FRAME =
    /// false`, or equivalently deleting the `reorient_table_to_rotated_frame`
    /// call in `detect_tables_from_spans`), this table comes out as the
    /// physical 2-row x 3-col grid -- `col_count == 3` (fails the first
    /// assertion, which expects `2`) and row 0 is
    /// `["R0C0", "R0C1", "R0C2"]` (fails the second assertion, which expects
    /// `["R1C0", "R0C0"]`).
    #[test]
    fn detect_tables_from_spans_transposes_90_degree_rotated_borderless_grid() {
        let spans = borderless_rotated_grid_fixture(90.0);
        let config = TableDetectionConfig::default();

        let tables = detect_tables_from_spans(&spans, &config);
        assert_eq!(tables.len(), 1, "expected exactly one detected table");
        let table = &tables[0];

        assert_eq!(
            table.col_count, 2,
            "a 90-degree borderless table's 3 physical columns must become 2 table \
             columns (the physical row count), got {}",
            table.col_count
        );
        assert_eq!(
            table.rows.len(),
            3,
            "expected 3 rows after transposing 2 physical rows into 3"
        );

        let row_texts: Vec<Vec<&str>> = table
            .rows
            .iter()
            .map(|r| r.cells.iter().map(|c| c.text.as_str()).collect())
            .collect();
        assert_eq!(
            row_texts,
            vec![vec!["R1C0", "R0C0"], vec!["R1C1", "R0C1"], vec!["R1C2", "R0C2"],],
            "90-degree rotation must rotate the borderless grid 90 degrees clockwise \
             (new row = old column ascending, new column = old row reversed); got {row_texts:?}"
        );
    }

    /// Borderless positive case for the opposite quadrant: 270° transposes
    /// the grid the other way round from 90° (new row = old column
    /// *reversed*, new column = old row ascending), pinning down the
    /// rotation direction rather than just "some transpose happened".
    ///
    /// Against unfixed code, this comes out identically to the 90-degree
    /// case above: `col_count == 3` (fails the first assertion, expects
    /// `2`) with row 0 `["R0C0", "R0C1", "R0C2"]` (fails the second
    /// assertion, which expects `["R0C2", "R1C2"]`).
    #[test]
    fn detect_tables_from_spans_transposes_270_degree_rotated_borderless_grid() {
        let spans = borderless_rotated_grid_fixture(270.0);
        let config = TableDetectionConfig::default();

        let tables = detect_tables_from_spans(&spans, &config);
        assert_eq!(tables.len(), 1, "expected exactly one detected table");
        let table = &tables[0];

        assert_eq!(
            table.col_count, 2,
            "a 270-degree borderless table's 3 physical columns must become 2 table \
             columns, got {}",
            table.col_count
        );
        assert_eq!(
            table.rows.len(),
            3,
            "expected 3 rows after transposing 2 physical rows into 3"
        );

        let row_texts: Vec<Vec<&str>> = table
            .rows
            .iter()
            .map(|r| r.cells.iter().map(|c| c.text.as_str()).collect())
            .collect();
        assert_eq!(
            row_texts,
            vec![vec!["R0C2", "R1C2"], vec!["R0C1", "R1C1"], vec!["R0C0", "R1C0"],],
            "270-degree rotation must rotate the borderless grid the opposite way \
             round from 90 degrees (new row = old column reversed, new column = old \
             row ascending); got {row_texts:?}"
        );
    }

    #[test]
    fn test_merge_vertically_adjacent_tables() {
        // Two tables with 3 columns each, bboxes separated by 5pt (< ADJACENT_TABLE_MERGE_GAP).
        // ~keep
        let table1 = Table {
            rows: vec![TableRow {
                cells: vec![
                    TableCell {
                        text: "A".into(),
                        spans: Vec::new(),
                        colspan: 1,
                        rowspan: 1,
                        mcids: vec![],
                        bbox: None,
                        is_header: false,
                    },
                    TableCell {
                        text: "B".into(),
                        spans: Vec::new(),
                        colspan: 1,
                        rowspan: 1,
                        mcids: vec![],
                        bbox: None,
                        is_header: false,
                    },
                    TableCell {
                        text: "C".into(),
                        spans: Vec::new(),
                        colspan: 1,
                        rowspan: 1,
                        mcids: vec![],
                        bbox: None,
                        is_header: false,
                    },
                ],
                is_header: false,
            }],
            has_header: false,
            col_count: 3,
            bbox: Some(Rect::new(0.0, 100.0, 300.0, 50.0)),
        };
        let table2 = Table {
            rows: vec![TableRow {
                cells: vec![
                    TableCell {
                        text: "D".into(),
                        spans: Vec::new(),
                        colspan: 1,
                        rowspan: 1,
                        mcids: vec![],
                        bbox: None,
                        is_header: false,
                    },
                    TableCell {
                        text: "E".into(),
                        spans: Vec::new(),
                        colspan: 1,
                        rowspan: 1,
                        mcids: vec![],
                        bbox: None,
                        is_header: false,
                    },
                    TableCell {
                        text: "F".into(),
                        spans: Vec::new(),
                        colspan: 1,
                        rowspan: 1,
                        mcids: vec![],
                        bbox: None,
                        is_header: false,
                    },
                ],
                is_header: false,
            }],
            has_header: false,
            col_count: 3,
            bbox: Some(Rect::new(0.0, 155.0, 300.0, 50.0)),
        };

        let mut tables = vec![table1, table2];
        merge_vertically_adjacent_tables(&mut tables);
        assert_eq!(tables.len(), 1, "Adjacent tables should be merged into one");
        assert_eq!(tables[0].rows.len(), 2, "Merged table should have 2 rows");
        assert_eq!(tables[0].col_count, 3);
    }

    #[test]
    fn test_no_merge_when_gap_too_large() {
        let table1 = Table {
            rows: vec![TableRow {
                cells: vec![TableCell {
                    text: "A".into(),
                    spans: Vec::new(),
                    colspan: 1,
                    rowspan: 1,
                    mcids: vec![],
                    bbox: None,
                    is_header: false,
                }],
                is_header: false,
            }],
            has_header: false,
            col_count: 1,
            bbox: Some(Rect::new(0.0, 100.0, 300.0, 50.0)),
        };
        let table2 = Table {
            rows: vec![TableRow {
                cells: vec![TableCell {
                    text: "B".into(),
                    spans: Vec::new(),
                    colspan: 1,
                    rowspan: 1,
                    mcids: vec![],
                    bbox: None,
                    is_header: false,
                }],
                is_header: false,
            }],
            has_header: false,
            col_count: 1,
            // Top at 200, gap = 200 - 150 = 50pt >> ADJACENT_TABLE_MERGE_GAP ~keep
            bbox: Some(Rect::new(0.0, 200.0, 300.0, 50.0)),
        };

        let mut tables = vec![table1, table2];
        merge_vertically_adjacent_tables(&mut tables);
        assert_eq!(tables.len(), 2, "Tables with large gap should NOT be merged");
    }

    #[test]
    fn test_census_h_and_v_in_different_regions() {
        // Census-style layout: H-lines at y=100, y=50 spanning full width (x=36..576)
        // V-lines at y=500..550 at positions x=36, 117, 197, 277, 357, 437, 517, 576
        // The H and V lines DON'T physically cross (different Y regions)
        // But they should produce a table via extended grid. ~keep
        let lines = vec![
            make_h_line(36.0, 100.0, 540.0),
            make_h_line(36.0, 50.0, 540.0),
            make_v_line(36.0, 500.0, 50.0),
            make_v_line(117.0, 500.0, 50.0),
            make_v_line(197.0, 500.0, 50.0),
            make_v_line(277.0, 500.0, 50.0),
            make_v_line(357.0, 500.0, 50.0),
            make_v_line(437.0, 500.0, 50.0),
            make_v_line(517.0, 500.0, 50.0),
            make_v_line(576.0, 500.0, 50.0),
        ];

        let spans = vec![
            create_test_span("A", 60.0, 70.0, 20.0, 10.0),
            create_test_span("B", 140.0, 70.0, 20.0, 10.0),
            create_test_span("C", 220.0, 70.0, 20.0, 10.0),
            create_test_span("D", 300.0, 70.0, 20.0, 10.0),
            create_test_span("E", 380.0, 70.0, 20.0, 10.0),
            create_test_span("F", 460.0, 70.0, 20.0, 10.0),
            create_test_span("G", 540.0, 70.0, 20.0, 10.0),
        ];

        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            min_table_cells: 2,
            min_table_columns: 2,
            ..TableDetectionConfig::default()
        };

        let tables = detect_tables_with_lines(&spans, &lines, &config);
        assert!(
            !tables.is_empty(),
            "Census layout with H/V in different Y regions should produce at least 1 table"
        );
        let table = &tables[0];
        // Should have ~7 columns (8 V-lines = 7 column spans) and 1 row (2 H-lines = 1 row) ~keep
        assert!(
            table.col_count >= 5,
            "Census table should have at least 5 columns, got {}",
            table.col_count
        );
    }

    #[test]
    fn test_w2_grid_not_fragmented() {
        // W-2 style: V-lines at x=100,200,300 spanning y=100..700 (full form)
        // V-lines at x=350,450 spanning y=300..500 (sub-section only)
        // H-lines at y=100,200,300,400,500,600,700
        // Should produce 1 table (or a few that merge), not 5+ fragments. ~keep
        let lines = vec![
            make_v_line(100.0, 100.0, 600.0),
            make_v_line(200.0, 100.0, 600.0),
            make_v_line(300.0, 100.0, 600.0),
            make_v_line(350.0, 300.0, 200.0),
            make_v_line(450.0, 300.0, 200.0),
            make_h_line(100.0, 100.0, 350.0),
            make_h_line(100.0, 200.0, 350.0),
            make_h_line(100.0, 300.0, 350.0),
            make_h_line(100.0, 400.0, 350.0),
            make_h_line(100.0, 500.0, 350.0),
            make_h_line(100.0, 600.0, 350.0),
            make_h_line(100.0, 700.0, 350.0),
        ];

        let spans = vec![
            create_test_span("R1C1", 120.0, 150.0, 30.0, 10.0),
            create_test_span("R1C2", 220.0, 150.0, 30.0, 10.0),
            create_test_span("R2C1", 120.0, 250.0, 30.0, 10.0),
            create_test_span("R2C2", 220.0, 250.0, 30.0, 10.0),
            create_test_span("R3C1", 120.0, 350.0, 30.0, 10.0),
            create_test_span("R3C2", 220.0, 350.0, 30.0, 10.0),
            create_test_span("R4C1", 120.0, 450.0, 30.0, 10.0),
            create_test_span("R4C2", 220.0, 450.0, 30.0, 10.0),
            create_test_span("R5C1", 120.0, 550.0, 30.0, 10.0),
            create_test_span("R5C2", 220.0, 550.0, 30.0, 10.0),
            create_test_span("R6C1", 120.0, 650.0, 30.0, 10.0),
            create_test_span("R6C2", 220.0, 650.0, 30.0, 10.0),
        ];

        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            min_table_cells: 4,
            min_table_columns: 2,
            ..TableDetectionConfig::default()
        };

        let tables = detect_tables_with_lines(&spans, &lines, &config);
        assert!(
            tables.len() <= 2,
            "W-2 grid should produce at most 2 tables (not fragmented into {})",
            tables.len()
        );
        let total_filled: usize = tables
            .iter()
            .flat_map(|t| &t.rows)
            .flat_map(|r| &r.cells)
            .filter(|c| !c.text.is_empty())
            .count();
        assert!(
            total_filled >= 8,
            "W-2 tables should capture most text spans, got {}",
            total_filled
        );
    }

    #[test]
    fn test_invoice_still_separate_tables() {
        let lines = vec![
            make_h_line(410.0, 83.0, 125.0),
            make_h_line(410.0, 142.0, 125.0),
            make_v_line(410.0, 71.0, 72.0),
            make_v_line(535.0, 71.0, 72.0),
            make_h_line(22.0, 150.0, 567.0),
            make_h_line(22.0, 553.0, 567.0),
            make_v_line(22.0, 150.0, 403.0),
            make_v_line(103.0, 150.0, 403.0),
            make_v_line(490.0, 150.0, 403.0),
            make_v_line(541.0, 150.0, 403.0),
            make_v_line(589.0, 150.0, 403.0),
        ];

        let mut spans = vec![
            create_test_span("Balance Due", 420.0, 100.0, 80.0, 10.0),
            create_test_span("$500.00", 420.0, 120.0, 60.0, 10.0),
        ];
        for i in 0..6 {
            let y = 160.0 + i as f32 * 60.0;
            spans.push(create_test_span("Date", 30.0, y, 40.0, 10.0));
            spans.push(create_test_span("Code", 110.0, y, 40.0, 10.0));
            spans.push(create_test_span("Desc", 200.0, y, 200.0, 10.0));
            spans.push(create_test_span("$100", 500.0, y, 30.0, 10.0));
        }

        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            min_table_cells: 2,
            min_table_columns: 1,
            ..TableDetectionConfig::default()
        };

        let tables = detect_tables_with_lines(&spans, &lines, &config);
        assert!(
            tables.len() >= 2,
            "Invoice should produce at least 2 separate tables (header + main), got {}",
            tables.len()
        );
    }

    #[test]
    fn test_two_column_table_detection() {
        let mut spans = vec![
            create_test_span("Abstract", 50.0, 700.0, 60.0, 12.0),
            create_test_span("We present a novel approach to language", 50.0, 680.0, 230.0, 12.0),
            create_test_span("Results show improvements across all", 50.0, 660.0, 230.0, 12.0),
            create_test_span("benchmarks with significant gains on", 50.0, 640.0, 230.0, 12.0),
            create_test_span("standard evaluation metrics.", 50.0, 620.0, 180.0, 12.0),
        ];

        spans.push(create_test_span("Model", 320.0, 700.0, 40.0, 12.0));
        spans.push(create_test_span("F1", 420.0, 700.0, 15.0, 12.0));
        spans.push(create_test_span("Acc", 500.0, 700.0, 20.0, 12.0));
        spans.push(create_test_span("BERT", 320.0, 680.0, 30.0, 12.0));
        spans.push(create_test_span("92.4", 420.0, 680.0, 25.0, 12.0));
        spans.push(create_test_span("89.1", 500.0, 680.0, 25.0, 12.0));
        spans.push(create_test_span("GPT", 320.0, 660.0, 25.0, 12.0));
        spans.push(create_test_span("91.2", 420.0, 660.0, 25.0, 12.0));
        spans.push(create_test_span("88.3", 500.0, 660.0, 25.0, 12.0));

        let config = TableDetectionConfig::default();
        let tables = detect_tables_from_spans_column_aware(&spans, &config);

        assert_eq!(
            tables.len(),
            1,
            "Should detect exactly 1 table in the right column, got {}",
            tables.len()
        );
        assert_eq!(
            tables[0].col_count, 3,
            "Table should have 3 columns, got {}",
            tables[0].col_count
        );
    }

    #[test]
    fn test_single_column_no_regression() {
        let spans = vec![
            create_test_span("Introduction", 50.0, 700.0, 80.0, 14.0),
            create_test_span(
                "This paper presents a comprehensive study of natural language",
                50.0,
                680.0,
                450.0,
                12.0,
            ),
            create_test_span(
                "processing techniques applied to large-scale document analysis.",
                50.0,
                660.0,
                430.0,
                12.0,
            ),
            create_test_span(
                "Our approach builds on recent advances in transformer architectures",
                50.0,
                640.0,
                460.0,
                12.0,
            ),
            create_test_span(
                "and demonstrates improvements across multiple benchmarks.",
                50.0,
                620.0,
                400.0,
                12.0,
            ),
            create_test_span(
                "We evaluate our method on standard datasets and report results.",
                50.0,
                600.0,
                420.0,
                12.0,
            ),
        ];

        let config = TableDetectionConfig::default();
        let tables = detect_tables_from_spans_column_aware(&spans, &config);
        assert!(
            tables.is_empty(),
            "Single-column paragraph text should not be detected as a table, got {} table(s)",
            tables.len()
        );
    }

    #[test]
    fn test_h_rule_bounded_text_table() {
        let lines = vec![make_h_line(50.0, 750.0, 350.0), make_h_line(50.0, 700.0, 350.0)];

        let spans = vec![
            create_test_span("Model", 60.0, 740.0, 50.0, 10.0),
            create_test_span("Acc", 180.0, 740.0, 30.0, 10.0),
            create_test_span("F1", 280.0, 740.0, 20.0, 10.0),
            create_test_span("BERT", 60.0, 728.0, 40.0, 10.0),
            create_test_span("84.6", 180.0, 728.0, 30.0, 10.0),
            create_test_span("83.4", 280.0, 728.0, 30.0, 10.0),
            create_test_span("GPT", 60.0, 716.0, 35.0, 10.0),
            create_test_span("82.1", 180.0, 716.0, 30.0, 10.0),
            create_test_span("81.0", 280.0, 716.0, 30.0, 10.0),
            create_test_span("XLNet", 60.0, 704.0, 45.0, 10.0),
            create_test_span("85.2", 180.0, 704.0, 30.0, 10.0),
            create_test_span("84.1", 280.0, 704.0, 30.0, 10.0),
        ];

        let config = TableDetectionConfig::default();
        let tables = detect_tables_with_lines(&spans, &lines, &config);
        assert!(
            !tables.is_empty(),
            "Should detect at least 1 table from text within H-line boundaries"
        );
        let table = &tables[0];
        assert!(
            table.col_count >= 3,
            "Expected at least 3 columns, got {}",
            table.col_count
        );
        assert!(
            table.rows.len() >= 3,
            "Expected at least 3 rows, got {}",
            table.rows.len()
        );
    }

    fn ruled_rows(rows: &[[&str; 3]], top: f32) -> Vec<TextSpan> {
        rows.iter()
            .enumerate()
            .flat_map(|(i, row)| {
                let y = top - 12.0 - i as f32 * 15.0;
                row.iter()
                    .zip([60.0, 180.0, 280.0])
                    .map(move |(text, x)| create_test_span(text, x, y, 40.0, 10.0))
            })
            .collect()
    }

    #[test]
    fn strict_lines_keep_a_rules_only_table_beside_a_grid() {
        let grid = [
            ["Item", "Qty", "Price"],
            ["Bolt", "40", "0.10"],
            ["Nut", "40", "0.05"],
            ["Washer", "80", "0.02"],
        ];
        let ruled = [
            ["Region", "Sales", "Growth"],
            ["Northland", "120", "3.1"],
            ["Eastmark", "95", "2.4"],
            ["Southvale", "88", "1.9"],
        ];
        let mut lines: Vec<_> = (0..=4)
            .map(|i| make_h_line(50.0, 750.0 - i as f32 * 15.0, 300.0))
            .collect();
        lines.extend([50.0, 150.0, 250.0, 350.0].map(|x| make_v_line(x, 690.0, 60.0)));
        lines.extend([600.0, 585.0, 540.0].map(|y| make_h_line(50.0, y, 300.0)));
        let mut spans = ruled_rows(&grid, 750.0);
        spans.extend(ruled_rows(&ruled, 600.0));

        let tables = detect_tables_with_lines(&spans, &lines, &TableDetectionConfig::strict());

        let booktabs = tables
            .iter()
            .find(|t| t.rows.iter().any(|r| r.cells.iter().any(|c| c.text == "Southvale")))
            .expect("rules-only table beside the grid");
        assert_eq!(tables.len(), 2);
        assert_eq!(booktabs.rows.len(), 4, "header above the midrule stays in the table");
        assert_eq!(booktabs.rows[0].cells[0].text, "Region");
    }

    #[test]
    fn h_rules_under_every_row_form_one_table() {
        let rows = [
            ["Code", "Name", "Rate"],
            ["A1", "Alpha", "1.5"],
            ["B2", "Beta", "2.5"],
            ["C3", "Gamma", "3.5"],
        ];
        let lines: Vec<_> = (0..=4)
            .map(|i| make_h_line(50.0, 600.0 - i as f32 * 15.0, 300.0))
            .collect();
        let spans = ruled_rows(&rows, 600.0);

        let tables = detect_tables_with_lines(&spans, &lines, &TableDetectionConfig::default());

        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].rows.len(), 4);
    }

    #[test]
    fn joined_header_band_keeps_the_body_columns_past_its_rule() {
        let lines = vec![
            make_h_line(50.0, 600.0, 300.0),
            make_h_line(60.0, 585.0, 300.0),
            make_h_line(70.0, 540.0, 300.0),
        ];
        let mut spans: Vec<_> = ["Code", "Name", "Rate"]
            .iter()
            .zip([80.0, 180.0, 280.0])
            .map(|(t, x)| create_test_span(t, x, 589.0, 40.0, 10.0))
            .collect();
        for (i, row) in [
            ["A1", "Alpha", "1.5", "x"],
            ["B2", "Beta", "2.5", "y"],
            ["C3", "Gamma", "3.5", "z"],
        ]
        .iter()
        .enumerate()
        {
            let y = 574.0 - i as f32 * 12.0;
            spans.extend(
                row.iter()
                    .zip([80.0, 180.0, 280.0, 345.0])
                    .map(|(t, x)| create_test_span(t, x, y, 20.0, 10.0)),
            );
        }

        let tables = detect_tables_with_lines(&spans, &lines, &TableDetectionConfig::default());

        let cells: Vec<&str> = tables
            .iter()
            .flat_map(|t| &t.rows)
            .flat_map(|r| &r.cells)
            .map(|c| c.text.as_str())
            .collect();
        assert!(
            cells.contains(&"z"),
            "body column past the header rule stays in the table: {cells:?}"
        );
    }

    #[test]
    fn test_split_table_at_section_dividers() {
        // Simulate a multi-section form with 3 sections separated by
        // full-width H-lines.  Each section has its OWN vertical grid lines
        // (they don't span across section boundaries).
        //
        // Section 1: y=10..40   (3 rows, H-lines at y=10,20,30,40)
        // Section 2: y=40..70   (3 rows, H-lines at y=40,50,60,70)
        // Section 3: y=70..100  (3 rows, H-lines at y=70,80,90,100)
        //
        // V-lines per section: x=10,40,70,100 but only within each section's
        // Y-range, so no V-edge crosses y=40 or y=70. ~keep

        let mut lines: Vec<crate::elements::PathContent> = Vec::new();

        for i in 0..=9 {
            let y = 10.0 + i as f32 * 10.0;
            lines.push(make_h_line(10.0, y, 90.0));
        }

        for &x in &[10.0, 40.0, 70.0, 100.0] {
            lines.push(make_v_line(x, 10.0, 30.0));
        }
        for &x in &[10.0, 40.0, 70.0, 100.0] {
            lines.push(make_v_line(x, 40.0, 30.0));
        }
        for &x in &[10.0, 40.0, 70.0, 100.0] {
            lines.push(make_v_line(x, 70.0, 30.0));
        }

        let mut spans = Vec::new();
        for row in 0..9 {
            let y = 15.0 + row as f32 * 10.0;
            for col in 0..3 {
                let x = 15.0 + col as f32 * 30.0;
                let label = format!("S{}-R{}-C{}", row / 3 + 1, row % 3 + 1, col + 1);
                spans.push(create_test_span(&label, x, y, 20.0, 8.0));
            }
        }

        let config = TableDetectionConfig {
            horizontal_strategy: TableStrategy::Lines,
            vertical_strategy: TableStrategy::Lines,
            ..TableDetectionConfig::default()
        };

        let tables = detect_tables_with_lines(&spans, &lines, &config);

        // The full-width H-lines at y=40 and y=70 have no V-edges crossing
        // through them, so they should be detected as section dividers.
        // We expect 3 tables (one per section). ~keep
        assert!(
            tables.len() >= 3,
            "Expected at least 3 tables after section-divider splitting, got {}",
            tables.len()
        );
        for (i, t) in tables.iter().enumerate() {
            assert_eq!(t.col_count, 3, "Table {} should have 3 columns, got {}", i, t.col_count);
        }
    }

    // -----------------------------------------------------------------
    // validate_table_structure_internal: split-column-group tests
    //
    // These exercise has_split_modal_column_groups, the structural
    // check that replaced the row-density gate. The detector rejects
    // grids whose modal rows partition into two or more disconnected
    // column-co-occurrence components. make_split_grid models the
    // false-positive shape (two prose flows mis-clustered into one
    // grid); make_grouped_grid models the sparse grouped-row-header
    // shape from the scientific-table regression class; the real
    // failure may also involve upstream column over-counting, while
    // this unit fixture pins the validator-level property that sparse
    // modal rows with connected populated columns are accepted.
    // ----------------------------------------------------------------- ~keep

    /// Build a minimal GridStructure with `num_rows` rows and
    /// `num_cols` columns, where every row populates exactly
    /// `populated_per_row` cells (the first N columns). Numeric
    /// fields of `ColumnCluster` / `RowCluster` are arbitrary —
    /// `validate_table_structure_internal` reads only
    /// `grid.columns.len()` and the emptiness of each cell.
    fn make_uniform_grid(num_cols: usize, num_rows: usize, populated_per_row: usize) -> GridStructure {
        let columns = (0..num_cols)
            .map(|_| ColumnCluster {
                x_center: 0.0,
                x_min: 0.0,
                x_max: 0.0,
                span_indices: vec![],
            })
            .collect();
        let rows = (0..num_rows)
            .map(|_| RowCluster {
                y_center: 0.0,
                y_min: 0.0,
                y_max: 0.0,
                span_indices: vec![],
            })
            .collect();
        let cells = (0..num_rows)
            .map(|_| {
                (0..num_cols)
                    .map(|c| if c < populated_per_row { vec![0usize] } else { vec![] })
                    .collect()
            })
            .collect();
        GridStructure { columns, rows, cells }
    }

    /// Build a GridStructure modelling two adjacent text flows
    /// mis-clustered into one candidate grid. The first `num_cols / 2`
    /// columns form the "left flow"; the remaining columns form the
    /// "right flow". Rows alternate between populating only the left
    /// flow and only the right flow. All rows have the same populated
    /// cardinality (`num_cols / 2`), so the regular-row-ratio gate
    /// passes at 1.00. `num_cols` must be even.
    fn make_split_grid(num_cols: usize, num_rows: usize) -> GridStructure {
        assert!(num_cols.is_multiple_of(2), "make_split_grid requires even num_cols");
        let half = num_cols / 2;
        let columns = (0..num_cols)
            .map(|_| ColumnCluster {
                x_center: 0.0,
                x_min: 0.0,
                x_max: 0.0,
                span_indices: vec![],
            })
            .collect();
        let rows = (0..num_rows)
            .map(|_| RowCluster {
                y_center: 0.0,
                y_min: 0.0,
                y_max: 0.0,
                span_indices: vec![],
            })
            .collect();
        let cells = (0..num_rows)
            .map(|r| {
                let left_row = r % 2 == 0;
                (0..num_cols)
                    .map(|c| {
                        let in_left_half = c < half;
                        if left_row == in_left_half { vec![0usize] } else { vec![] }
                    })
                    .collect()
            })
            .collect();
        GridStructure { columns, rows, cells }
    }

    /// Build a GridStructure modelling a hierarchical scientific table.
    /// `total_cols` columns; the first `group_cols` are populated only
    /// in the first row of each group of `group_size` consecutive rows;
    /// the remaining columns are populated in every row. Models the
    /// failure shape from arxiv_2510.24670v2: grouped row-headers above
    /// dense data columns. The over-counting that the maintainer
    /// described occurs upstream of this fixture; here we model the
    /// post-clustering grid the validator actually sees. Numeric
    /// cluster fields are arbitrary, matching the convention used by
    /// `make_uniform_grid` and `make_split_grid`.
    fn make_grouped_grid(total_cols: usize, num_rows: usize, group_cols: usize, group_size: usize) -> GridStructure {
        assert!(group_cols < total_cols, "group_cols must be < total_cols");
        assert!(group_size > 0, "group_size must be positive");
        let columns = (0..total_cols)
            .map(|_| ColumnCluster {
                x_center: 0.0,
                x_min: 0.0,
                x_max: 0.0,
                span_indices: vec![],
            })
            .collect();
        let rows = (0..num_rows)
            .map(|_| RowCluster {
                y_center: 0.0,
                y_min: 0.0,
                y_max: 0.0,
                span_indices: vec![],
            })
            .collect();
        let cells = (0..num_rows)
            .map(|r| {
                let is_group_header = r % group_size == 0;
                (0..total_cols)
                    .map(|c| {
                        let populated = if c < group_cols { is_group_header } else { true };
                        if populated { vec![0usize] } else { vec![] }
                    })
                    .collect()
            })
            .collect();
        GridStructure { columns, rows, cells }
    }

    /// 6 columns, 6 rows, modal rows alternate between {0,1,2} and
    /// {3,4,5}. Two disconnected components of 3 columns each, each
    /// with 3 modal rows of support. Default profile.
    #[test]
    fn validate_rejects_split_column_groups() {
        let grid = make_split_grid(6, 6);
        let config = TableDetectionConfig::default();
        assert!(!validate_table_structure_internal(&grid, &config));
    }

    /// Same fixture, strict profile. The strict profile's stronger
    /// regular_row_ratio (0.8) does not catch this; the split-column
    /// detector does.
    #[test]
    fn validate_rejects_split_column_groups_under_strict_profile() {
        let grid = make_split_grid(6, 6);
        let config = TableDetectionConfig::strict();
        assert!(!validate_table_structure_internal(&grid, &config));
    }

    /// 11 columns, 5 rows, every row populates every column. One
    /// component spanning all columns → accepted.
    #[test]
    fn validate_accepts_dense_table() {
        let grid = make_uniform_grid(11, 5, 11);
        let config = TableDetectionConfig::default();
        assert!(validate_table_structure_internal(&grid, &config));
    }

    /// 6 columns, 5 rows, every row populates the first 4 columns.
    /// The old density gate would have admitted this at the boundary
    /// (4/6 = 2/3). One connected component of 4 columns → accepted.
    #[test]
    fn validate_accepts_sparse_connected_table() {
        let grid = make_uniform_grid(6, 5, 4);
        let config = TableDetectionConfig::default();
        assert!(validate_table_structure_internal(&grid, &config));
    }

    /// 8 columns, 12 rows, grouped row-headers occupy the first 2
    /// columns and are populated only in the first row of each group
    /// of 4. Models arxiv_2510.24670v2's failure shape (post-
    /// clustering): 9 modal data rows populate columns 2..8, six data
    /// columns all connected, one component → accepted. The real
    /// failure may also involve upstream column over-counting; this
    /// fixture pins the validator-level property we care about:
    /// sparse modal rows whose populated columns form one connected
    /// component must be accepted.
    #[test]
    fn validate_accepts_hierarchical_grouped_table() {
        let grid = make_grouped_grid(8, 12, 2, 4);
        let config = TableDetectionConfig::default();
        assert!(validate_table_structure_internal(&grid, &config));
    }

    /// num_cols = 3 short-circuits has_split_modal_column_groups
    /// (num_cols < 4), so a small dense grid passes. Documents the
    /// boundary.
    #[test]
    fn validate_accepts_three_column_grid() {
        let grid = make_uniform_grid(3, 4, 3);
        let config = TableDetectionConfig::default();
        assert!(validate_table_structure_internal(&grid, &config));
    }

    // ========================================================================
    // consolidate_adjacent_table_fragments regression tests
    // ======================================================================== ~keep

    /// Build a minimal Table with a bbox and col_count for consolidation tests.
    fn make_fragment(x: f32, y: f32, width: f32, height: f32, cols: u32) -> Table {
        let mut t = Table::new();
        t.bbox = Some(Rect::new(x, y, width, height));
        t.col_count = cols as usize;
        // Push one empty row so consolidation has something to extend; the
        // row count grows as fragments get merged. ~keep
        t.rows.push(TableRow::new(false));
        t
    }

    /// Two vertically-adjacent fragments with identical column structure
    /// merge into a single multi-row table.  Models the fragmented-table
    /// pattern where every horizontal ruling line produces a
    /// separate 1-row Table.
    #[test]
    fn consolidate_merges_adjacent_aligned_fragments() {
        let upper = make_fragment(90.0, 480.0, 420.0, 16.0, 8);
        let lower = make_fragment(90.0, 464.0, 420.0, 16.0, 8);
        // ~keep
        let merged = consolidate_adjacent_table_fragments(vec![upper, lower]);
        assert_eq!(merged.len(), 1, "two aligned adjacent fragments must merge");
        assert_eq!(merged[0].rows.len(), 2, "merged rows are concatenated");
        let bb = merged[0].bbox.expect("merged bbox preserved");
        assert!((bb.x - 90.0).abs() < 0.1);
        assert!((bb.width - 420.0).abs() < 0.1);
        assert!((bb.height - 32.0).abs() < 0.1);
    }

    /// Fragments separated by more than `Y_TOLERANCE = 3pt` must NOT merge —
    /// they represent two distinct tables (e.g. two unrelated grids on the
    /// same page).
    #[test]
    fn consolidate_does_not_merge_non_adjacent_fragments() {
        let upper = make_fragment(90.0, 600.0, 420.0, 16.0, 8);
        let lower = make_fragment(90.0, 400.0, 420.0, 16.0, 8); // 200pt gap ~keep
        let result = consolidate_adjacent_table_fragments(vec![upper, lower]);
        assert_eq!(result.len(), 2, "well-separated fragments stay distinct");
    }

    /// A table ruled between row *bands* (not every row) emits fragments
    /// separated by a full inter-row pitch, not abutting. The default 3pt
    /// tolerance leaves them split (dropping the short band to the >=3-row
    /// guard); the row-height-scaled tolerance rejoins them. Models the
    /// PMC8103272 Table-1 [3, 2] → [5] recovery.
    #[test]
    fn consolidate_with_tol_merges_row_pitch_separated_bands() {
        // Fragment with `extra` additional empty rows beyond make_fragment's one. ~keep
        let frag = |x, y, cols, rows: usize| {
            let mut t = make_fragment(x, y, 420.0, 16.0, cols);
            for _ in 1..rows {
                t.rows.push(TableRow::new(false));
            }
            t
        };
        // upper.bottom = 480; lower.top = 452 + 16 = 468 → 12pt gap (one pitch). ~keep
        let upper = frag(90.0, 480.0, 6, 3);
        let lower = frag(90.0, 452.0, 6, 2);
        // Default tolerance: bands stay split (this is the pre-fix behaviour). ~keep
        let split = consolidate_adjacent_table_fragments(vec![upper.clone(), lower.clone()]);
        assert_eq!(split.len(), 2, "3pt tolerance leaves row-pitch bands split");
        // Row-height-scaled tolerance (1.5 * ~16pt row height = 24 > 12pt gap): merge. ~keep
        let merged = consolidate_adjacent_table_fragments_with_tol(vec![upper, lower], 2.0, 24.0);
        assert_eq!(merged.len(), 1, "row-pitch bands rejoin under scaled tolerance");
        assert_eq!(merged[0].rows.len(), 5, "[3,2] bands rejoin into a 5-row table");
    }

    /// The scaled tolerance still refuses fragments that are farther apart
    /// than a plausible single row pitch — two genuinely distinct grids.
    #[test]
    fn consolidate_with_tol_still_rejects_far_bands() {
        let upper = make_fragment(90.0, 600.0, 420.0, 16.0, 6);
        let lower = make_fragment(90.0, 400.0, 420.0, 16.0, 6);
        let merged = consolidate_adjacent_table_fragments_with_tol(vec![upper, lower], 2.0, 24.0);
        assert_eq!(merged.len(), 2, "a scaled tolerance is not an unbounded merge");
    }

    /// Fragments with different column counts must NOT merge even if
    /// they sit adjacent vertically — they aren't a single logical table.
    #[test]
    fn consolidate_does_not_merge_different_col_counts() {
        let upper = make_fragment(90.0, 480.0, 420.0, 16.0, 8);
        let lower = make_fragment(90.0, 464.0, 420.0, 16.0, 5);
        let result = consolidate_adjacent_table_fragments(vec![upper, lower]);
        assert_eq!(result.len(), 2, "different col_count blocks merging");
    }

    /// Fragments at different x-positions must NOT merge — they're in
    /// different columns of the page even if their Y ranges are adjacent.
    #[test]
    fn consolidate_does_not_merge_misaligned_x() {
        let upper = make_fragment(90.0, 480.0, 420.0, 16.0, 8);
        let lower = make_fragment(50.0, 464.0, 420.0, 16.0, 8);
        let result = consolidate_adjacent_table_fragments(vec![upper, lower]);
        assert_eq!(result.len(), 2, "misaligned-x blocks merging");
    }

    /// A chain of three+ adjacent fragments should chain-merge into one
    /// multi-row table.  This chains 8 column-aligned
    /// fragments → one 18-row table after consolidation.
    #[test]
    fn consolidate_chains_multiple_adjacent_fragments() {
        let f1 = make_fragment(90.0, 480.0, 420.0, 16.0, 8);
        let f2 = make_fragment(90.0, 464.0, 420.0, 16.0, 8);
        let f3 = make_fragment(90.0, 448.0, 420.0, 16.0, 8);
        let f4 = make_fragment(90.0, 432.0, 420.0, 16.0, 8);
        let merged = consolidate_adjacent_table_fragments(vec![f1, f2, f3, f4]);
        assert_eq!(merged.len(), 1, "chain of 4 adjacent fragments → 1 table");
        assert_eq!(merged[0].rows.len(), 4, "all rows preserved in chain merge");
    }

    /// Small vertical overlap (line-detector quirk) up to half the
    /// smaller fragment's height is still considered adjacent.
    #[test]
    fn consolidate_tolerates_small_overlap() {
        // upper bottom = 480, lower top = 484 (4pt overlap, smaller_h = 16, half = 8) ~keep
        let upper = make_fragment(90.0, 480.0, 420.0, 16.0, 8);
        let lower = make_fragment(90.0, 468.0, 420.0, 16.0, 8);
        let merged = consolidate_adjacent_table_fragments(vec![upper, lower]);
        assert_eq!(merged.len(), 1, "small overlap (< ½ row) still merges");
    }

    /// Empty input passes through unchanged.
    #[test]
    fn consolidate_empty_input() {
        let merged = consolidate_adjacent_table_fragments(vec![]);
        assert!(merged.is_empty());
    }

    /// Single-table input passes through unchanged (nothing to merge).
    #[test]
    fn consolidate_single_table_passthrough() {
        let t = make_fragment(90.0, 480.0, 420.0, 16.0, 8);
        let merged = consolidate_adjacent_table_fragments(vec![t]);
        assert_eq!(merged.len(), 1);
    }

    // ========================================================================
    // cell_span_separator regression tests
    // ======================================================================== ~keep

    /// Helper to construct a TextSpan with just the fields the separator
    /// rule actually reads: bbox, font_size, and text.
    fn ts(text: &str, x: f32, y: f32, width: f32, fs: f32) -> TextSpan {
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: text.to_string(),
            bbox: Rect::new(x, y, width, fs),
            font_size: fs,
            font_name: String::new(),
            font_weight: crate::layout::text_block::FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        }
    }

    /// Adjacent spans with a sub-em gap must NOT have a separator
    /// inserted.  Models `60000` + `≤` + `Q` + `＜` + `80000`
    /// where the operator glyphs touch / slightly overlap the digit
    /// glyphs and must be rendered as a single compound token.
    #[test]
    fn cell_span_separator_no_space_for_tight_glyphs() {
        let prev = ts("60000≤Q", 100.0, 200.0, 39.8, 10.6);
        let curr = ts("＜", 139.8, 200.0, 10.6, 10.6);
        assert_eq!(cell_span_separator(&prev, &curr), "");
    }

    /// Adjacent spans with a real gap (clear word boundary) get a
    /// single space separator inserted.
    #[test]
    fn cell_span_separator_inserts_space_for_real_gap() {
        let prev = ts("Quarter", 100.0, 200.0, 30.0, 10.0);
        let curr = ts("Total", 140.0, 200.0, 25.0, 10.0); // gap = 10pt = 1em ~keep
        assert_eq!(cell_span_separator(&prev, &curr), " ");
    }

    /// CJK ↔ fullwidth-operator boundary suppresses separator even when
    /// the geometric gap would otherwise warrant a space.
    #[test]
    fn cell_span_separator_suppresses_cjk_fullwidth_boundary() {
        let prev = ts("中", 100.0, 200.0, 12.0, 12.0);
        let curr = ts("＜", 115.0, 200.0, 12.0, 12.0); // gap = 3pt = 0.25 em ~keep
        assert_eq!(cell_span_separator(&prev, &curr), "");
    }

    /// Trailing whitespace on the preceding span suppresses separator
    /// (no double-space).
    #[test]
    fn cell_span_separator_suppresses_on_existing_trailing_space() {
        let prev = ts("Quarter ", 100.0, 200.0, 35.0, 10.0);
        let curr = ts("Total", 140.0, 200.0, 25.0, 10.0);
        assert_eq!(cell_span_separator(&prev, &curr), "");
    }

    /// GH#1948: the table word source marks a word that its source span
    /// separated with a whitespace by prefixing the word's text with a space.
    /// When the two words' boxes abut (a tight face draws the space glyph over
    /// its neighbours), the separator must not add a *second* space, and the
    /// one carried in the text must survive.
    #[test]
    fn cell_text_keeps_a_source_space_between_abutting_words() {
        let spans = vec![
            ts("corrispettivi", 100.0, 200.0, 37.7, 7.83),
            ts(" superiori", 137.0, 200.0, 27.6, 7.83),
        ];
        assert_eq!(extract_cell_text(&[0, 1], &spans), "corrispettivi superiori");
    }

    /// GH#2092: a native PDF may draw the leading digits and the locale's
    /// thousands-separator group as separate words with a visible glyph gap. ~keep
    #[test]
    fn cell_text_keeps_numeric_separator_group_attached() {
        for separator in ['.', ',', '\u{066B}', '\u{066C}'] {
            let suffix = format!("{separator}804");
            let spans = vec![
                ts("59", 100.0, 200.0, 10.0, 10.0),
                ts(&suffix, 113.0, 200.0, 20.0, 10.0),
            ];

            assert_eq!(extract_cell_text(&[0, 1], &spans), format!("59{separator}804"));
        }
    }

    #[test]
    fn cell_text_does_not_attach_sentence_punctuation() {
        let spans = vec![
            ts("Section", 100.0, 200.0, 10.0, 10.0),
            ts(".", 113.0, 200.0, 20.0, 10.0),
        ];

        assert_eq!(extract_cell_text(&[0, 1], &spans), "Section .");
    }

    /// A word that opens a cell can carry the same source-space prefix, but a
    /// cell's first word has no preceding word to separate from; the leading
    /// space must not leak into the cell text (GH#1948).
    #[test]
    fn cell_text_trims_a_source_space_prefix_at_the_cell_start() {
        let spans = vec![ts(" 400", 100.0, 200.0, 20.0, 7.83)];
        assert_eq!(extract_cell_text(&[0], &spans), "400");
    }

    /// Reproduces xberg-io/xberg#1555: a drawn row band whose baselines are
    /// 288.7 (col0), 294.7/282.7 (col1, wraps to 2 lines) and
    /// 300.7/288.7/276.7 (col2, wraps to 3 lines). Clustering naively at
    /// `strict()`'s `row_tolerance` of 1.0 yields 5 Y-clusters, and only the
    /// 288.7 cluster (col0 + col2's middle line) has evidence in >= 2
    /// columns. The split must be rejected in full and the drawn band must
    /// survive as the single row it was already built as. ~keep
    #[test]
    fn split_rows_by_text_positions_keeps_drawn_band_as_one_row_when_only_one_cell_wraps() {
        let spans = vec![
            create_test_span("1", 90.2, 288.7, 5.0, 0.0),
            create_test_span("Versterking van het", 118.6, 294.7, 60.0, 0.0),
            create_test_span("MKB-segment", 118.6, 282.7, 40.0, 0.0),
            create_test_span("Ontwikkel gerichte campagnes", 246.1, 300.7, 90.0, 0.0),
            create_test_span("een vereenvoudigde waardepropositie", 246.1, 288.7, 100.0, 0.0),
            create_test_span("om de conversie te verhogen.", 246.1, 276.7, 90.0, 0.0),
        ];

        let row = TableRow {
            cells: vec![
                prose_cell("1"),
                prose_cell("Versterking van het MKB-segment"),
                prose_cell(
                    "Ontwikkel gerichte campagnes een vereenvoudigde waardepropositie \
                     om de conversie te verhogen.",
                ),
            ],
            is_header: false,
        };

        let row_cell_span_indices = vec![vec![vec![0], vec![1, 2], vec![3, 4, 5]]];
        let config = TableDetectionConfig::strict();

        let result = split_rows_by_text_positions(vec![row], &row_cell_span_indices, &spans, &config);

        assert_eq!(
            result.len(),
            1,
            "one wrapped cell must not fracture the drawn band into extra rows"
        );
        assert_eq!(result[0].cells[0].text, "1");
        assert_eq!(result[0].cells[1].text, "Versterking van het MKB-segment");
    }

    #[test]
    fn split_rows_by_text_positions_keeps_single_column_split_behavior() {
        let spans = vec![
            create_test_span("Label", 10.0, 200.0, 30.0, 0.0),
            create_test_span("Value", 10.0, 180.0, 30.0, 0.0),
        ];
        let row = TableRow {
            cells: vec![prose_cell("Label Value")],
            is_header: false,
        };
        let row_cell_span_indices = vec![vec![vec![0, 1]]];

        let result = split_rows_by_text_positions(
            vec![row],
            &row_cell_span_indices,
            &spans,
            &TableDetectionConfig::strict(),
        );

        assert_eq!(
            result.len(),
            2,
            "the two-column evidence floor must be capped for one-column bands"
        );
        assert_eq!(result[0].cells[0].text, "Label");
        assert_eq!(result[1].cells[0].text, "Value");
    }

    /// Page-3-shaped downstream regression for #1555: one header band and three
    /// data bands each contain 1/2/3-line cells on staggered baselines. Keeping
    /// every drawn band whole must leave a four-row, three-column table that
    /// clears the real-grid gate; the old sixteen-row expansion failed it at 4/16.
    #[test]
    fn wrapped_drawn_bands_remain_a_real_four_row_grid() {
        let mut spans = vec![
            create_test_span("#", 90.2, 330.0, 5.0, 0.0),
            create_test_span("Aanbeveling", 118.6, 330.0, 60.0, 0.0),
            create_test_span("Toelichting", 246.1, 330.0, 90.0, 0.0),
        ];
        let mut rows = vec![TableRow {
            cells: vec![prose_cell("#"), prose_cell("Aanbeveling"), prose_cell("Toelichting")],
            is_header: true,
        }];
        let mut row_cell_span_indices = vec![vec![vec![0], vec![1], vec![2]]];

        for (row_number, baseline) in [(1, 288.7), (2, 242.7), (3, 196.7)] {
            let start = spans.len();
            spans.extend([
                create_test_span(&row_number.to_string(), 90.2, baseline, 5.0, 0.0),
                create_test_span("Versterking van het", 118.6, baseline + 6.0, 60.0, 0.0),
                create_test_span("MKB-segment", 118.6, baseline - 6.0, 40.0, 0.0),
                create_test_span("Ontwikkel gerichte campagnes", 246.1, baseline + 12.0, 90.0, 0.0),
                create_test_span("een vereenvoudigde waardepropositie", 246.1, baseline, 100.0, 0.0),
                create_test_span("om de conversie te verhogen.", 246.1, baseline - 12.0, 90.0, 0.0),
            ]);
            rows.push(TableRow {
                cells: vec![
                    prose_cell(&row_number.to_string()),
                    prose_cell("Versterking van het MKB-segment"),
                    prose_cell(
                        "Ontwikkel gerichte campagnes een vereenvoudigde waardepropositie \
                         om de conversie te verhogen.",
                    ),
                ],
                is_header: false,
            });
            row_cell_span_indices.push(vec![
                vec![start],
                vec![start + 1, start + 2],
                vec![start + 3, start + 4, start + 5],
            ]);
        }

        let rows = split_rows_by_text_positions(rows, &row_cell_span_indices, &spans, &TableDetectionConfig::strict());
        let table = Table {
            rows,
            has_header: true,
            col_count: 3,
            bbox: None,
        };

        assert_eq!(table.rows.len(), 4, "four producer-drawn bands must remain four rows");
        assert!(
            table.is_real_grid(),
            "the repaired table must survive the downstream real-grid gate"
        );
        assert_eq!(table.rows[1].cells[0].text, "1");
        assert_eq!(table.rows[1].cells[1].text, "Versterking van het MKB-segment");
        assert_eq!(table.rows[3].cells[0].text, "3");
        assert_eq!(table.rows[3].cells[1].text, "Versterking van het MKB-segment");
    }

    /// The hybrid case `split_rows_by_text_positions` exists for: vertical
    /// column lines exist but no horizontal rule separates two real rows
    /// crammed into one intersection cell. Each candidate row has
    /// independent text in both columns, so this split must be kept intact
    /// and produce exactly 2 rows in page order. ~keep
    #[test]
    fn split_rows_by_text_positions_splits_genuine_multi_row_band_with_two_column_evidence() {
        let spans = vec![
            create_test_span("Alice", 10.0, 200.0, 30.0, 0.0),
            create_test_span("30", 60.0, 200.0, 10.0, 0.0),
            create_test_span("Bob", 10.0, 180.0, 30.0, 0.0),
            create_test_span("25", 60.0, 180.0, 10.0, 0.0),
        ];

        let row = TableRow {
            cells: vec![prose_cell("Alice Bob"), prose_cell("30 25")],
            is_header: false,
        };

        let row_cell_span_indices = vec![vec![vec![0, 2], vec![1, 3]]];
        let config = TableDetectionConfig::strict();

        let result = split_rows_by_text_positions(vec![row], &row_cell_span_indices, &spans, &config);

        assert_eq!(result.len(), 2, "two genuinely distinct rows must survive the split");
        assert_eq!(result[0].cells[0].text, "Alice");
        assert_eq!(result[0].cells[1].text, "30");
        assert_eq!(result[1].cells[0].text, "Bob");
        assert_eq!(result[1].cells[1].text, "25");
    }

    /// REGRESSION xberg-io/xberg#1565: a producer-drawn band mixing genuine multi-column data
    /// rows with a single-column section lead-in ("Mogelijke oorzaken:") must not collapse
    /// back into one mega-row just because the lead-in itself lacks two-column evidence. The
    /// two data rows are independently evidenced, so the split is accepted; the lead-in shares
    /// no column with anything to its left/right that it doesn't already have on its own — it
    /// is the topmost cluster and has no cluster above it to fold into — so it must survive as
    /// its own one-cell-wide row instead of being fused into a data row's cell. ~keep
    #[test]
    fn split_rows_by_text_positions_keeps_heading_line_separate_amid_multi_column_rows() {
        let spans = vec![
            create_test_span("Mogelijke oorzaken:", 10.0, 220.0, 100.0, 0.0),
            create_test_span("Symptom A", 10.0, 200.0, 40.0, 0.0),
            create_test_span("Cause A", 60.0, 200.0, 40.0, 0.0),
            create_test_span("Symptom B", 10.0, 180.0, 40.0, 0.0),
            create_test_span("Cause B", 60.0, 180.0, 40.0, 0.0),
        ];

        let row = TableRow {
            cells: vec![
                prose_cell("Mogelijke oorzaken: Symptom A Symptom B"),
                prose_cell("Cause A Cause B"),
            ],
            is_header: false,
        };

        let row_cell_span_indices = vec![vec![vec![0, 1, 3], vec![2, 4]]];
        let config = TableDetectionConfig::strict();

        let result = split_rows_by_text_positions(vec![row], &row_cell_span_indices, &spans, &config);

        assert_eq!(
            result.len(),
            3,
            "the heading and both genuine data rows must each survive as their own row"
        );
        assert_eq!(result[0].cells[0].text, "Mogelijke oorzaken:");
        assert_eq!(
            result[0].cells[1].text, "",
            "the heading must not be fused into a data row's cell text"
        );
        assert_eq!(result[1].cells[0].text, "Symptom A");
        assert_eq!(result[1].cells[1].text, "Cause A");
        assert_eq!(result[2].cells[0].text, "Symptom B");
        assert_eq!(result[2].cells[1].text, "Cause B");
    }

    /// FINDING 2 (adversarial review): does the #1565 row-split fix create a new
    /// single-cell-wide row shaped exactly like `strip_form_numbering_artifacts`'s Phase 1
    /// numbering-artifact signature (every cell empty or a lone digit 1-9, with at least one
    /// lone digit) out of what used to be a non-foldable single-column lead-in? Runs the
    /// *whole* `finalize_intersection_tables` pipeline (split, then artifact-strip, then
    /// empty-row table segmentation) rather than just `split_rows_by_text_positions`, since
    /// that is the real caller order and Phase 1 runs immediately after the split.
    ///
    /// `min_table_cells` is deliberately set to 1 so the outer table-emission gate cannot
    /// mask what happens to this one row -- the question under test is row-level survival,
    /// not table-level acceptance.
    #[test]
    fn finalize_intersection_tables_deletes_lone_digit_lead_in_row() {
        let spans = vec![
            create_test_span("5", 10.0, 220.0, 5.0, 0.0),
            create_test_span("Symptom A", 10.0, 200.0, 40.0, 0.0),
            create_test_span("Cause A", 60.0, 200.0, 40.0, 0.0),
            create_test_span("Symptom B", 10.0, 180.0, 40.0, 0.0),
            create_test_span("Cause B", 60.0, 180.0, 40.0, 0.0),
        ];

        let row = TableRow {
            cells: vec![prose_cell("5 Symptom A Symptom B"), prose_cell("Cause A Cause B")],
            is_header: false,
        };

        let row_cell_span_indices = vec![vec![vec![0, 1, 3], vec![2, 4]]];
        let config = TableDetectionConfig {
            min_table_cells: 1,
            ..TableDetectionConfig::strict()
        };

        // Confirm, as a checkpoint, that the split itself still isolates "5" into its own
        // one-cell-wide row -- the same shape as the heading test above, just digit-shaped
        // instead of alphabetic. If this checkpoint ever stops holding the rest of this test
        // is testing a different mechanism than the one under review.
        let split_only = split_rows_by_text_positions(vec![row.clone()], &row_cell_span_indices, &spans, &config);
        assert_eq!(
            split_only.len(),
            3,
            "checkpoint: split must isolate the lead-in as its own row"
        );
        assert_eq!(split_only[0].cells[0].text, "5");
        assert_eq!(split_only[0].cells[1].text, "");

        let tables = finalize_intersection_tables(vec![row], &row_cell_span_indices, &spans, &config, 2);

        assert_eq!(tables.len(), 1, "the two evidenced data rows must still form one table");
        let surviving_rows: Vec<Vec<&str>> = tables[0]
            .rows
            .iter()
            .map(|r| r.cells.iter().map(|c| c.text.as_str()).collect())
            .collect();

        // Documents observed behavior: Phase 1 of `strip_form_numbering_artifacts` deletes
        // this row whole. This is the same rule the module already pins for a row that was
        // ALWAYS a standalone lone-digit row (`test_strip_form_numbering_artifacts` row 0):
        // that rule is not new, and reachable-from-a-real-band content shaped exactly like a
        // classic form row-number (a bare digit, sharing no column with any neighboring data)
        // is indistinguishable, by any signal Phase 1 has, from the artifact it exists to
        // remove. Judgment call: NOT a regression fix here -- see the finding-2 report for
        // why a content-preserving change is not being made.
        assert_eq!(
            surviving_rows,
            vec![vec!["Symptom A", "Cause A"], vec!["Symptom B", "Cause B"]],
            "lone-digit lead-in row is removed by Phase 1, same as a pre-existing standalone artifact row"
        );
    }

    /// "CLEARED" CLAIM 1 (adversarial review): a multi-hop fold chain must not manufacture
    /// column evidence that was never actually present. Three-cluster band: the topmost and
    /// bottommost clusters are each independently evidenced (2 columns), and a middle
    /// deficient cluster carries text in only ONE of those two columns. The deficient
    /// cluster's own column footprint ({0}) is a subset of the cluster above it ({0, 1}), so
    /// it is allowed to fold -- but the emitted rows must still reflect only the columns each
    /// physical cluster actually had text in; folding must never cause a group's rendered
    /// column count, or the per-column text, to exceed what the real spans support.
    #[test]
    fn split_rows_by_text_positions_fold_chain_does_not_fabricate_column_evidence() {
        let spans = vec![
            // Bottom cluster (y=180): evidenced, both columns. ~keep
            create_test_span("Alice", 10.0, 180.0, 30.0, 0.0),
            create_test_span("30", 60.0, 180.0, 10.0, 0.0),
            // Middle cluster (y=200): deficient, column 0 only -- a wrapped continuation of
            // the row above it (y=220), not of the row below. ~keep
            create_test_span("continued", 10.0, 200.0, 40.0, 0.0),
            // Top cluster (y=220): evidenced, both columns. ~keep
            create_test_span("Bob", 10.0, 220.0, 30.0, 0.0),
            create_test_span("25", 60.0, 220.0, 10.0, 0.0),
        ];

        let row = TableRow {
            cells: vec![prose_cell("Alice continued Bob"), prose_cell("30 25")],
            is_header: false,
        };

        let row_cell_span_indices = vec![vec![vec![0, 2, 3], vec![1, 4]]];
        let config = TableDetectionConfig::strict();

        let result = split_rows_by_text_positions(vec![row], &row_cell_span_indices, &spans, &config);

        assert_eq!(
            result.len(),
            2,
            "the deficient middle cluster must fold, not survive alone"
        );
        // Top group (y=220 anchor with y=200 folded in): column 0 gets both lines, column 1
        // only ever had "25" -- folding must not invent a "25 continued" or duplicate value. ~keep
        assert_eq!(result[0].cells[0].text, "Bob\ncontinued");
        assert_eq!(
            result[0].cells[1].text, "25",
            "folding a column-0-only cluster must not fabricate evidence in column 1"
        );
        assert_eq!(result[1].cells[0].text, "Alice");
        assert_eq!(result[1].cells[1].text, "30");
    }

    /// "CLEARED" CLAIM 2 (adversarial review): every span fed into a deficient-cluster-heavy
    /// band must land in exactly one output cell -- never duplicated across two groups, and
    /// never silently dropped by the `members.iter().any(...)` filter in the final
    /// cell-assembly pass. Uses the same fold-chain fixture as claim 1 and checks span
    /// accounting by content rather than by trusting row/column counts alone.
    #[test]
    fn split_rows_by_text_positions_fold_chain_drops_no_span_and_duplicates_none() {
        let spans = vec![
            create_test_span("Alice", 10.0, 180.0, 30.0, 0.0),
            create_test_span("30", 60.0, 180.0, 10.0, 0.0),
            create_test_span("continued", 10.0, 200.0, 40.0, 0.0),
            create_test_span("Bob", 10.0, 220.0, 30.0, 0.0),
            create_test_span("25", 60.0, 220.0, 10.0, 0.0),
        ];

        let row = TableRow {
            cells: vec![prose_cell("Alice continued Bob"), prose_cell("30 25")],
            is_header: false,
        };

        let row_cell_span_indices = vec![vec![vec![0, 2, 3], vec![1, 4]]];
        let config = TableDetectionConfig::strict();

        let result = split_rows_by_text_positions(vec![row], &row_cell_span_indices, &spans, &config);

        let mut seen_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for r in &result {
            for c in &r.cells {
                for word in c.text.split_whitespace() {
                    *seen_counts.entry(word).or_insert(0) += 1;
                }
            }
        }
        for expected in ["Alice", "30", "continued", "Bob", "25"] {
            assert_eq!(
                seen_counts.get(expected).copied().unwrap_or(0),
                1,
                "span text {expected:?} must appear exactly once across all output cells, got {:?}",
                seen_counts.get(expected)
            );
        }
    }
}
