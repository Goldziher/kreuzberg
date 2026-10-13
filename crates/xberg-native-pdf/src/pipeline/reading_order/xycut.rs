//! XY-Cut recursive spatial partitioning for multi-column text layout.
//!
//! This module implements the XY-Cut algorithm per PDF Spec Section 9.4 for
//! recursive geometric analysis without semantic heuristics. Uses projection
//! profiles to detect column boundaries in complex layouts.
//!
//! Per ISO 32000-1:2008:
//! - Section 9.4: Text Objects and coordinates
//! - Section 14.7: Logical Structure (prefers structure tree when available)
//!
//! # Algorithm Overview
//!
//! 1. Compute horizontal projection (white space density across X)
//! 2. Find valleys (gaps) where density < threshold
//! 3. Split region at widest valley (vertical line)
//! 4. Recursively partition left and right sub-regions
//! 5. Alternate to vertical projection if no horizontal valleys found
//! 6. Base case: Sort spans top-to-bottom, left-to-right
//!
//! # Performance
//!
//! Typical newspaper page: ~100 spans, < 5ms processing time
//! Recursive depth: O(log n) for balanced columns

// TODO(xberg-io/xberg#1567): 4 cyclomatic-complexity and 13 size/complexity findings
// in this file, currently excluded via the quality-debt baseline in alef.toml. Splitting
// these needs compiler-in-the-loop verification, not a mechanical pass. Delete this
// note and the file's baseline entry together once it goes green. Help wanted.

use super::{ReadingOrderContext, ReadingOrderStrategy};
use crate::error::Result;
use crate::geometry::Rect;
use crate::layout::TextSpan;
use crate::pipeline::{OrderedTextSpan, ReadingOrderInfo};

/// Maximum density-array length for XY-cut projection profiles.
///
/// A normal PDF page is at most a few thousand points wide/tall. This limit of
/// 100 000 bins is generous (≈ 33× a 3000-point A0 page) while being small
/// enough to never cause an allocation problem. Spans whose bounding-box span
/// exceeds this limit are the result of a degenerate CTM; returning `None` from
/// the projection safely skips the split instead of attempting a multi-terabyte
/// allocation that would abort the process via `handle_alloc_error`.
const MAX_PROJECTION_SIZE: usize = 100_000;

/// Coarse classification of a region for the multi-column-prose
/// fix. Used to gate the tight-gutter cut: tight cuts are only accepted on
/// regions that *positively* identify as prose, so the same XY-cut recursion
/// no longer corrupts table cells (the lesson — see lines 73–101).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegionKind {
    /// Tall stack of wide lines OR tall stack of half-column lines with
    /// substantial content per line. Safe to apply tight-gutter cuts.
    Prose,
    /// Short cells in a grid (mean characters per line < 8). Tight cuts
    /// here corrupt cell ordering — the canonical google_doc population
    /// table that reverted two earlier attempts is the prototype.
    Table,
    /// Anything else — too few lines, mixed shapes, decorative regions.
    /// Default to the behaviour (no tight cut).
    Mixed,
}

/// Contiguous run of bold-or-larger-font spans spanning ≥ 2 visual lines
/// that the XY-cut splitter must treat as an atomic block. Built by
/// `find_heading_runs` BEFORE recursive partitioning,
/// then substituted into the partition input as a single wide synthetic
/// span so cluster-detection / valley-finding can't drive a vertical
/// cut THROUGH a wrapped heading.
///
/// After partition completes, `expand_blocks` projects the synthetic
/// placeholder back into its constituent original spans, preserving
/// each span's per-glyph metadata for downstream consumers
/// (markdown converter heading-level inference, layout-preserving
/// DOCX export, etc.).
#[derive(Debug, Clone)]
struct HeadingRun {
    /// Indices into the original `&[TextSpan]` slice, in reading order
    /// (top-to-bottom, left-to-right within a line).
    span_indices: Vec<usize>,
    /// Union of the constituent spans' bboxes. Substituted for each
    /// individual bbox during partition so the heading appears as one
    /// wide bbox.
    combined_bbox: Rect,
}

/// Within a valley run `[start, end)` of a projection profile's density
/// array, choose the split OFFSET at the run's deepest (lowest-density)
/// point rather than its arithmetic midpoint (GH#1763).
///
/// A wide interior run classified "below threshold" is not uniformly
/// empty: `horizontal_projection_indexed` only excludes spans wider than
/// 55% of the region and spans with fewer than 2 non-whitespace
/// characters, so a full-width caption line or a single-char table-cell
/// row can still occupy bins inside the run with nonzero (but
/// sub-threshold) density. The run's arithmetic midpoint has no relation
/// to where that content sits, so it can land squarely inside a figure
/// caption even though a genuinely empty gutter exists elsewhere in the
/// same run.
///
/// Tie-break order, applied to the contiguous sub-runs that attain the
/// run's minimum density value:
///   1. Widest sub-run wins — a genuine open gutter is wide; an isolated
///      single-bin dip that happens to share the same minimum density
///      is not a gutter and should not win over a real one.
///   2. On a width tie, the sub-run whose center is nearest the WHOLE
///      run's arithmetic midpoint wins — keeps the choice deterministic
///      and, when nothing else distinguishes the candidates, close to
///      the pre-fix behavior.
///
/// When the run is uniformly at its minimum density throughout (the
/// common case: a real, empty column gutter with no stray content), the
/// single minimal sub-run IS the whole run, so this returns exactly the
/// old midpoint — the fix only changes behavior in the buggy case where
/// sub-threshold content is unevenly distributed inside the run. That
/// exactness is why centers are computed in f32 as `(lo + hi) / 2`:
/// an integer `lo + width / 2` truncates, which would shift the split
/// by half a unit on every odd-width run. That arithmetic is pinned by
/// `a_candidate_that_would_cut_a_span_is_rejected`, whose chosen gap has
/// odd width -- NOT by `uniform_run_split_is_unchanged_from_the_legacy_midpoint`,
/// which cannot reach it: a uniform run's midpoint is by definition at the
/// floor, so the guard below returns before any centre is computed. ~keep
/// Share of the projected region a below-threshold run must exceed before its midpoint is
/// treated as untrustworthy (GH#1763).
///
/// A real column gutter is narrow -- 15 to 80 pt on a region of roughly 500 pt, so 3% to 16%
/// -- and its midpoint is the gutter, which is why splitting there worked for years. The
/// GH#1763 page is the opposite case: its "valley" is 254 pt of a 523 pt region, 48.6%,
/// because the left column is a figure and a 7.2 pt caption that fall below the density
/// threshold almost everywhere. A run that wide is not a gutter at all, it is a sparse
/// region, and its arithmetic midpoint says nothing about where the columns divide.
///
/// 35% sits well above any plausible gutter and well below the reporting page. Relocating
/// regardless of run width was measured across 230 corpus documents and was not an
/// improvement: 23 documents changed and, by absolute dictionary-valid word count, more got
/// worse than better. ~keep
const SPARSE_VALLEY_REGION_SHARE: f32 = 0.35;

fn deepest_valley_point(density: &[f32], start: usize, end: usize, split_is_clear: &dyn Fn(f32) -> bool) -> f32 {
    debug_assert!(start < end && end <= density.len());
    if start >= end || end > density.len() {
        return (start + end) as f32 / 2.0;
    }
    let run_mid = (start + end) as f32 / 2.0;
    let min_density = density[start..end].iter().copied().fold(f32::INFINITY, f32::min);

    // A narrow run IS the gutter, and its midpoint is the right place to split; only a run
    // too wide to be a gutter has an untrustworthy midpoint. See SPARSE_VALLEY_REGION_SHARE. ~keep
    if ((end - start) as f32) <= density.len() as f32 * SPARSE_VALLEY_REGION_SHARE {
        return (start + end) as f32 / 2.0;
    }

    // Relocate ONLY when the midpoint actually lands on content -- the defect's own
    // precondition. If the midpoint already sits at the run's density floor, the split is
    // already falling through empty space and cutting nothing, so moving it to some wider
    // empty region elsewhere changes reading order for no benefit. Measured: without this
    // guard the fix altered 23 of 230 corpus documents and, counted by dictionary-valid
    // words, made 11 worse against 8 better -- the split was being relocated on pages that
    // had nothing wrong with them. A split line of odd width falls between two bins; either
    // one being at the floor is enough to leave it alone. ~keep
    let mid_low = run_mid.floor() as usize;
    let mid_high = (run_mid.ceil() as usize).min(end - 1);
    if density[mid_low] == min_density || density[mid_high] == min_density {
        return run_mid;
    }

    let mut candidates: Vec<(usize, f32)> = Vec::new();
    let mut i = start;
    while i < end {
        if density[i] != min_density {
            i += 1;
            continue;
        }
        let sub_start = i;
        while i < end && density[i] == min_density {
            i += 1;
        }
        candidates.push((i - sub_start, (sub_start + i) as f32 / 2.0));
    }

    candidates.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| crate::utils::safe_float_cmp((left.1 - run_mid).abs(), (right.1 - run_mid).abs()))
    });

    // A zero in this profile does NOT mean no glyphs: `horizontal_projection_indexed`
    // deliberately omits spans wider than 55% of the region, spans of fewer than two
    // non-whitespace characters, and the part of every span beyond its estimated text core.
    // Seeking the deepest point therefore steers the split straight at the regions those
    // omissions create, which is the opposite of what a midpoint did by accident. Measured
    // without this check: 22 of 230 corpus documents changed and words came apart at
    // single-character spans -- "virgin" into "v" + "irgin", "test" into "t" + "est" --
    // because the split landed between two spans of one word. Candidates are therefore
    // checked against the real span extents, not the profile, and a run whose candidates all
    // cut something keeps the midpoint, which is exactly the pre-GH#1763 behaviour. ~keep
    for (_, center) in candidates {
        if split_is_clear(center) {
            return center;
        }
    }
    run_mid
}

/// Union of the bboxes of `spans[indices]`. Empty index list yields a
/// zero-sized rect at the origin (never built in practice — guarded by
/// the caller).
fn union_bboxes(spans: &[TextSpan], indices: &[usize]) -> Rect {
    let mut x_min = f32::MAX;
    let mut y_min = f32::MAX;
    let mut x_max = f32::MIN;
    let mut y_max = f32::MIN;
    for &i in indices {
        let b = spans[i].bbox;
        x_min = x_min.min(b.left());
        x_max = x_max.max(b.right());
        y_min = y_min.min(b.top());
        y_max = y_max.max(b.bottom());
    }
    if x_min == f32::MAX {
        return Rect::default();
    }
    Rect::from_points(x_min, y_min, x_max, y_max)
}

/// XY-Cut recursive spatial partitioning strategy.
///
/// Detects columns using projection profiles and white space analysis.
/// Suitable for newspapers, academic papers, and multi-column layouts.
pub struct XYCutStrategy {
    /// Minimum number of spans in a region before attempting split (default: 5).
    /// Prevents excessive recursion on small regions.
    pub min_spans_for_split: usize,

    /// Valley threshold as fraction of peak projection density (default: 0.3).
    /// Lower values detect narrower gutters, higher values only detect wide gaps.
    pub valley_threshold: f32,

    /// Minimum valley width in points (default: 15.0).
    /// Prevents detecting single-character gaps as column boundaries.
    pub min_valley_width: f32,

    /// Enable horizontal partitioning first, fallback to vertical (default: true).
    ///
    /// Per PDF Spec ISO 32000-1:2008 §14.8.4 (Logical Structure reading order),
    /// column detection is the primary purpose of XY-Cut — horizontal-first
    /// (vertical cut line) splits columns before rows, matching Western
    /// top-down-left-to-right reading order in multi-column documents.
    /// Callers with row-dominant layouts can override via
    /// `with_prefer_horizontal(false)`.
    pub prefer_horizontal: bool,
}

/// Cap on `partition_indexed` recursion depth. Real layouts nest only a few
/// splits deep; this bound only fires on the singleton-peel pathology (many
/// distinct-Y header/footer strips) where unbounded depth is O(n² log n). Set
/// high enough that no real document reaches it.
const MAX_PARTITION_DEPTH: u32 = 64;

// GH#1808: visual-line y-tolerance for the full-width-line peel and the whole-line
// partition assignment. Matches `find_heading_runs`'s own `same_line` tolerance rather than
// introducing a second answer to "are these two spans on the same line".
const XYCUT_LINE_Y_TOLERANCE_PTS: f32 = 1.0;
// A line whose inked extent reaches this fraction of the region's own width is full-width
// furniture (a legend, a running header, a caption) rather than column content -- no real
// two-column body line's own width can reach here, since a column is bounded by the page
// margin AND the gutter (never much past ~48% of the region on a normal two-column split).
const FULL_WIDTH_LINE_FRACTION: f32 = 0.90;
// A single full-width line is an ordinary title or footer -- already handled by
// `is_single_column_region`'s bridge exclusion elsewhere -- and peeling it alone would cost
// a recursive call for no benefit. The population this fix targets starts at multi-line
// legends and table notes (GH#1808's own reproducer is 8 lines).
const MIN_PEELED_FULL_WIDTH_LINES: usize = 2;
// The gap, in ems of the line's own max font size, at which a line stops being "continuous"
// across a column cut. A row's two cells are separated by the gutter (several ems); a
// legend's own font runs abut at normal word/kern spacing.
const MAX_INTRA_LINE_GAP_EM: f32 = 1.0;
// A run narrower than this many ems is a label, a marker or a cell rather than a prose line.
const LABEL_RUN_MAX_EM: f32 = 8.0;
const MIN_LABEL_VALUE_ROWS: usize = 3;
// GH#2110: a continuous line with at least this much ink on each side of a candidate column
// cut is prose running across the gutter, not column content. Matches the minimum column
// width a cut must leave on each side. ~keep
const MIN_STRADDLING_INK_PTS: f32 = 60.0;

/// Group `indices` into visual lines: anchored (not chained) on `y`, top to bottom, and
/// further split on `x` wherever consecutive items are more than `MAX_INTRA_LINE_GAP_EM`
/// ems apart -- so two unrelated fragments that merely share a baseline (a header printed
/// once per column, at the same `y` but 50pt apart) are never treated as one line. A line
/// returned by this function is therefore a genuinely CONTINUOUS run of ink, which is what
/// both `peel_full_width_line_bands` and `partition_lines_at` need "line" to mean. ~keep
fn group_indices_into_lines(all_spans: &[TextSpan], indices: &[usize]) -> Vec<Vec<usize>> {
    let mut order: Vec<usize> = indices.to_vec();
    order.sort_by(|&a, &b| {
        all_spans[b]
            .bbox
            .y
            .total_cmp(&all_spans[a].bbox.y)
            .then_with(|| all_spans[a].bbox.left().total_cmp(&all_spans[b].bbox.left()))
    });
    let mut y_buckets: Vec<Vec<usize>> = Vec::new();
    let mut anchor_y = f32::NAN;
    for index in order {
        let y = all_spans[index].bbox.y;
        if y_buckets.is_empty() || (anchor_y - y).abs() > XYCUT_LINE_Y_TOLERANCE_PTS {
            anchor_y = y;
            y_buckets.push(Vec::new());
        }
        y_buckets.last_mut().expect("just pushed above").push(index);
    }
    let mut lines: Vec<Vec<usize>> = Vec::new();
    for mut bucket in y_buckets {
        // A bucket is in `y` order, and a right-column line set up to 1pt higher sorts before
        // the left column's line, so the gap test below needs `x` order. ~keep
        bucket.sort_by(|&a, &b| all_spans[a].bbox.left().total_cmp(&all_spans[b].bbox.left()));
        let mut run: Vec<usize> = Vec::new();
        // Measured from the run's rightmost edge, so a span nested inside a longer one
        // never opens a gap. ~keep
        let mut run_right = f32::MIN;
        for index in bucket {
            if let Some(&last) = run.last() {
                let gap = all_spans[index].bbox.left() - run_right;
                let max_font = all_spans[last].font_size.max(all_spans[index].font_size);
                if gap > max_font * MAX_INTRA_LINE_GAP_EM {
                    lines.push(std::mem::take(&mut run));
                    run_right = f32::MIN;
                }
            }
            run_right = run_right.max(all_spans[index].bbox.right());
            run.push(index);
        }
        if !run.is_empty() {
            lines.push(run);
        }
    }
    lines
}

impl Default for XYCutStrategy {
    fn default() -> Self {
        Self {
            min_spans_for_split: 5,
            valley_threshold: 0.3,
            // 15pt. A fix for multi-column prose interleaving was attempted
            // TWICE and REVERTED both times — the 70-PDF sweep caught data
            // corruption in the google_doc population table's digits
            // ("273.879.7501" -> "1273.879.750") each time:
            //
            //   Attempt 1 — lower min_valley_width 15 -> 12 so the tight
            //   ~12pt two-column gutter is detected. Also split the
            //   table's ~12pt inter-cell gaps -> reordered digits.
            //
            //   Attempt 2 — a structural find_two_column_prose_split
            //   (exactly-two recurring left-edge clusters, wide columns,
            //   clean gutter) tried before the single-column check. It
            //   never fired on the target page's WHOLE extent (three left-edge
            //   clusters: full-width intro/footer @60 + left @82 + right
            //   @312, because is_single_column blocks band separation
            //   first), yet it DID fire on a 2-column sub-region of the
            //   google_doc table and reordered cells.
            //
            // Root cause: the same XY-Cut machinery orders both
            // prose-columns and table-cells. Any sensitivity increase
            // that catches tight 2-column prose also splits
            // table cells and corrupts data. A correct fix needs a
            // real table-vs-prose classifier (column cells are short
            // values; prose columns are tall stacks of wide lines) AND
            // recursive band-separation of full-width header/footer rows
            // before column detection — a substantial XY-Cut redesign,
            // validated against the full CI corpus, not a local tweak. ~keep
            min_valley_width: Self::DEFAULT_MIN_VALLEY_WIDTH,
            prefer_horizontal: true,
        }
    }
}

impl XYCutStrategy {
    /// Narrowest empty corridor, in points, the default strategy cuts through. A gutter
    /// narrower than this is invisible to its projection-valley search, so a caller
    /// repairing the pages the cut misses can use it as the boundary of that population. ~keep
    pub const DEFAULT_MIN_VALLEY_WIDTH: f32 = 15.0;

    /// Create a new XY-Cut strategy with default parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom valley threshold (0.0-1.0).
    pub fn with_valley_threshold(mut self, threshold: f32) -> Self {
        self.valley_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Create with custom minimum valley width.
    pub fn with_min_valley_width(mut self, width: f32) -> Self {
        self.min_valley_width = width.max(1.0);
        self
    }

    /// Enable or disable horizontal partitioning first preference.
    pub fn with_prefer_horizontal(mut self, prefer: bool) -> Self {
        self.prefer_horizontal = prefer;
        self
    }

    /// Core recursive partitioning algorithm.
    ///
    /// Public for use by MarkdownConverter's ColumnAware reading order mode.
    ///
    /// Runs a pre-pass that detects multi-line heading runs
    /// (bold or larger-than-body font, ≥ 2 wrapped lines with matching
    /// X-extent) and locks them as atomic blocks the recursive splitter
    /// cannot split. Without this, a wrapped heading whose tail lines
    /// Y-overlap with adjacent-column dense content (table caption, table
    /// row, image label) gets bucketed across columns: line 1 glued to the
    /// body paragraph, line 2..N orphaned into the wrong block — and the
    /// markdown converter then promotes the orphan tail to a phantom
    /// heading (`### …`) in the wrong location.
    ///
    /// `column_gutter` is the mid-X of the page's column gutter when the
    /// caller has detected one; it keeps the heading-run pre-pass from
    /// folding a heading that opens the other column into a wrapped
    /// heading's run (GH#1757). `None` leaves that fold unconditional.
    pub fn partition_region(&self, spans: &[TextSpan], column_gutter: Option<f32>) -> Vec<Vec<TextSpan>> {
        let heading_runs = self.find_heading_runs(spans, column_gutter);
        if heading_runs.is_empty() {
            // Hot path: no headings found, skip the synthesize/expand
            // pair entirely so the cost is bounded to one O(n log n) sort
            // inside find_heading_runs. ~keep
            let indices: Vec<usize> = (0..spans.len()).collect();
            let index_groups = self.partition_indexed(spans, &indices);
            return index_groups
                .into_iter()
                .map(|group| group.into_iter().map(|i| spans[i].clone()).collect())
                .collect();
        }

        let (synthetic, synthetic_origin) = self.synthesize_for_partition(spans, &heading_runs);
        let synth_indices: Vec<usize> = (0..synthetic.len()).collect();
        let synth_groups = self.partition_indexed(&synthetic, &synth_indices);

        self.expand_blocks(synth_groups, spans, &synthetic_origin)
    }

    /// Detect contiguous bold/large-font runs that span ≥ 2 lines with
    /// matching X-extent (i.e. wrapped subsection headings).
    ///
    /// Per the fix-543 plan §A.2: two adjacent spans (in reading
    /// order) are considered to belong to the same heading run when
    /// ALL of the following hold:
    ///
    /// 1. Both are heading-like (bold, OR font_size > median × 1.15).
    /// 2. Same font_size (within 0.5 pt epsilon).
    /// 3. Same bold flag.
    /// 4. Next span's left edge is within `[prev.left, prev.left + 6pt]`
    ///    (wrapped heading lines often re-indent by up to ~6pt).
    /// 5. Next span sits ≤ 1.5 × line-height below the previous span
    ///    (a single-line gap; double-line gaps are paragraph breaks).
    ///
    /// `median_font_size` is computed across non-bold spans so heavy
    /// bold runs don't bias the body-size estimate upward.
    fn find_heading_runs(&self, spans: &[TextSpan], column_gutter: Option<f32>) -> Vec<HeadingRun> {
        if spans.len() < 2 {
            return Vec::new();
        }

        // Median body font size from NON-bold spans only. Bold spans
        // typically sit at heading sizes (bigger than body), so including
        // them biases the median high and we'd miss bold headings whose
        // size sits between body and the heavier weight tier. ~keep
        let mut non_bold_sizes: Vec<f32> = spans
            .iter()
            .filter(|s| !s.font_weight.is_bold())
            .map(|s| s.font_size)
            .filter(|&sz| sz > 0.0)
            .collect();
        let median_body = if non_bold_sizes.is_empty() {
            let mut sizes: Vec<f32> = spans.iter().map(|s| s.font_size).filter(|&sz| sz > 0.0).collect();
            if sizes.is_empty() {
                return Vec::new();
            }
            sizes.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
            sizes[sizes.len() / 2]
        } else {
            non_bold_sizes.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
            non_bold_sizes[non_bold_sizes.len() / 2]
        };
        let heading_size_floor = median_body * 1.15;

        let is_heading_like = |s: &TextSpan| -> bool { s.font_weight.is_bold() || s.font_size > heading_size_floor };

        // Sort indices by reading order (top of page first; Rect::top()
        // is the SMALLER Y of the normalized rect — see comment at
        // line ~885 — so larger Y = higher on page in PDF coords;
        // we want DESCENDING Y here). ~keep
        let mut order: Vec<usize> = (0..spans.len()).collect();
        order.sort_by(|&a, &b| {
            let y_cmp = crate::utils::safe_float_cmp(spans[b].bbox.top(), spans[a].bbox.top());
            if y_cmp != std::cmp::Ordering::Equal {
                return y_cmp;
            }
            crate::utils::safe_float_cmp(spans[a].bbox.left(), spans[b].bbox.left())
        });

        // Cluster reading-order-adjacent heading-like spans into runs.
        // The same line may carry multiple bold spans (one per Tj
        // segment); we collapse runs across lines, not within a line. ~keep
        let indent_tolerance = 6.0_f32;
        let font_eps = 0.5_f32;
        let mut runs: Vec<Vec<usize>> = Vec::new();
        let mut current: Vec<usize> = Vec::new();

        for &idx in &order {
            let span = &spans[idx];

            // A whitespace-only span carries no ink, so it must not break an
            // otherwise-continuous heading run. A `TJ` array's tab kern
            // (`[(3.)-1329.5(Title)] TJ`) turns into exactly such a span,
            // always `FontWeight::Normal` regardless of the surrounding
            // bold context (see `advance.rs`'s synthetic-space
            // construction) — so `is_heading_like` below would reject it
            // and sever the run at the marker/title boundary, splitting a
            // numbered heading's marker from its own title into two
            // separate (non-heading, single-span) fragments while the
            // wrapped continuation line still merges into one. The
            // narrower fragment's union bbox then no longer covers the
            // marker's column position, letting an unrelated span at the
            // marker's row/column slot in between during partitioning.
            // Skipping (not breaking, not joining) the whitespace span
            // keeps the run open across it, exactly as the same producer
            // setting the marker in a separate text object (no kern, no
            // synthetic space) already does. ~keep
            if span.text.chars().all(char::is_whitespace) {
                continue;
            }

            if !is_heading_like(span) {
                if !current.is_empty() {
                    runs.push(std::mem::take(&mut current));
                }
                continue;
            }

            if current.is_empty() {
                current.push(idx);
                continue;
            }

            let last_idx = *current.last().unwrap();
            let last = &spans[last_idx];

            let size_ok = (span.font_size - last.font_size).abs() <= font_eps;
            let bold_ok = span.font_weight.is_bold() == last.font_weight.is_bold();

            // Same-line: top within 1 pt of last's top — fold without
            // applying indent/leading checks (both spans belong to the
            // SAME wrapped-heading line, e.g. two bold Tj segments). ~keep
            let same_line = (span.bbox.top() - last.bbox.top()).abs() <= 1.0;

            // Rows are sorted by top across the WHOLE page, so a heading
            // opening the other column can sort between a wrapped heading's
            // two lines (GH#1757: 0.25 pt below line 1, 78.9 pt away across
            // the gutter). Skip it — neither fold it in nor let it close the
            // run — so the run stays open for the real continuation line,
            // which is the whole point of this pre-pass. Folding it in makes
            // it the run's last span and the continuation line then fails the
            // indent test against the WRONG column's x; letting it break the
            // run leaves two single-line candidates that the >= 2 distinct
            // lines filter below drops. This sits BEFORE the size/weight
            // tests because both failure modes cost the run: the issue's
            // 10 pt variant fails `size_ok` and breaks it instead.
            //
            // Inert unless the caller supplied a gutter, so every XY-cut
            // entry point on an output path must pass one — see
            // `PdfDocument::detect_column_gutter`. ~keep
            if same_line && Self::same_line_span_belongs_to_other_column(span, last, column_gutter) {
                continue;
            }

            if size_ok && bold_ok && same_line {
                current.push(idx);
                continue;
            }

            // Different line: enforce indent (4) + leading (5).
            // line_height = max of the two spans' bbox heights, plus a
            // floor of font_size to handle ascender-only / descender-only
            // glyphs with collapsed bboxes. ~keep
            let line_h = last.bbox.height.max(span.bbox.height).max(last.font_size).max(1.0);
            let leading_tolerance = line_h * 1.5;

            // PDF coords: y grows up, so the wrapped line sits at a
            // SMALLER bbox.top than the previous line. The gap between
            // last's bottom and span's top should fit inside the leading
            // tolerance. ~keep
            let last_bottom = last.bbox.top();
            // ~keep
            let span_top = span.bbox.top();
            let vertical_gap = (last_bottom - span_top).abs();

            let indent_ok = span.bbox.left() >= last.bbox.left() - indent_tolerance
                && span.bbox.left() <= last.bbox.left() + indent_tolerance;
            let leading_ok = vertical_gap <= leading_tolerance;

            if size_ok && bold_ok && indent_ok && leading_ok {
                current.push(idx);
            } else {
                runs.push(std::mem::take(&mut current));
                current.push(idx);
            }
        }
        if !current.is_empty() {
            runs.push(current);
        }

        // A run becomes a HeadingRun only when it spans ≥ 2 distinct
        // lines. Single-line bold spans (inline emphasis, lone short
        // headings) don't need locking — XY-cut handles them correctly
        // already, and locking them would be a no-op for the splitter
        // but adds overhead. ~keep
        runs.into_iter()
            .filter_map(|span_indices| {
                if span_indices.len() < 2 {
                    return None;
                }
                let mut distinct_lines = std::collections::BTreeSet::new();
                for &i in &span_indices {
                    distinct_lines.insert(spans[i].bbox.top().round() as i32);
                }
                if distinct_lines.len() < 2 {
                    return None;
                }
                Some(HeadingRun {
                    combined_bbox: union_bboxes(spans, &span_indices),
                    span_indices,
                })
            })
            .collect()
    }

    /// Whether `span`, which shares a line with the current run's last span
    /// `last`, in fact belongs to a DIFFERENT column — in which case it is
    /// neither run material nor a reason to close the run.
    ///
    /// The answer is geometric and exact: the two spans are in different
    /// columns when one ends before the gutter and the other begins after it.
    /// A span that straddles the gutter (a full-width banner heading) is in
    /// neither column and is never separated from anything by this test.
    ///
    /// `None` — a caller with no gutter to give — answers `false`, so the
    /// fold is unconditional exactly as it was before GH#1757. A width-based
    /// stand-in was measured and rejected: on a page with no detected gutter
    /// the gaps it would have to reject are the same size as the gaps inside
    /// legitimate heading lines (median 82 pt, half at or above the 79 pt of
    /// GH#1757's own gutter, on one corpus document), so no threshold
    /// separates the two populations and every such page would change. ~keep
    fn same_line_span_belongs_to_other_column(span: &TextSpan, last: &TextSpan, column_gutter: Option<f32>) -> bool {
        let Some(gutter_x) = column_gutter else {
            return false;
        };
        let (left_box, right_box) = if span.bbox.left() <= last.bbox.left() {
            (span, last)
        } else {
            (last, span)
        };
        left_box.bbox.right() <= gutter_x && right_box.bbox.left() >= gutter_x
    }

    /// Build a synthetic span list where each detected `HeadingRun`
    /// collapses to ONE wide synthetic span carrying the union bbox.
    /// Non-heading spans pass through unchanged.
    ///
    /// Returns:
    /// - `synthetic`: the input to `partition_indexed`.
    /// - `synthetic_origin[k]`: indices of ORIGINAL spans backing
    ///   synthetic span `k`. Length 1 for pass-throughs, ≥ 2 for
    ///   heading-run placeholders. Used by `expand_blocks` to project
    ///   partition output back into original-span space.
    fn synthesize_for_partition(&self, spans: &[TextSpan], runs: &[HeadingRun]) -> (Vec<TextSpan>, Vec<Vec<usize>>) {
        let mut in_run: Vec<Option<usize>> = vec![None; spans.len()];
        for (r_idx, run) in runs.iter().enumerate() {
            for &i in &run.span_indices {
                in_run[i] = Some(r_idx);
            }
        }

        let mut synthetic: Vec<TextSpan> = Vec::with_capacity(spans.len());
        let mut origins: Vec<Vec<usize>> = Vec::with_capacity(spans.len());
        let mut emitted_run = vec![false; runs.len()];

        for (i, span) in spans.iter().enumerate() {
            match in_run[i] {
                None => {
                    synthetic.push(span.clone());
                    origins.push(vec![i]);
                }
                Some(r_idx) if !emitted_run[r_idx] => {
                    let run = &runs[r_idx];
                    let mut placeholder = span.clone();
                    placeholder.bbox = run.combined_bbox;
                    // Concatenate the run's text with single spaces so
                    // is_single_column_region's core-width estimate is
                    // proportional to the actual heading length, not the
                    // single first-line fragment. ~keep
                    let mut combined_text = String::new();
                    for (k, &si) in run.span_indices.iter().enumerate() {
                        if k > 0 {
                            combined_text.push(' ');
                        }
                        combined_text.push_str(&spans[si].text);
                    }
                    placeholder.text = combined_text;
                    synthetic.push(placeholder);
                    origins.push(run.span_indices.clone());
                    emitted_run[r_idx] = true;
                }
                Some(_) => {}
            }
        }

        (synthetic, origins)
    }

    /// Project partition groups from synthetic-span space back into
    /// original-span space, expanding each heading-run placeholder into
    /// its constituent original spans (in their original ordering).
    fn expand_blocks(
        &self,
        synth_groups: Vec<Vec<usize>>,
        original: &[TextSpan],
        synthetic_origin: &[Vec<usize>],
    ) -> Vec<Vec<TextSpan>> {
        synth_groups
            .into_iter()
            .map(|group| {
                let mut out = Vec::with_capacity(group.len());
                for synth_idx in group {
                    for &orig_idx in &synthetic_origin[synth_idx] {
                        out.push(original[orig_idx].clone());
                    }
                }
                out
            })
            .collect()
    }

    /// Index-based recursive partitioning — returns groups of indices into the input span slice.
    ///
    /// Avoids cloning TextSpan at every recursive split level. Spans are only
    /// read through shared reference; indices are partitioned instead.
    fn partition_indexed(&self, all_spans: &[TextSpan], indices: &[usize]) -> Vec<Vec<usize>> {
        self.partition_indexed_depth(all_spans, indices, 0)
    }

    /// Depth-bounded recursive partition. `find_vertical_split_indexed` permits
    /// singleton peels, so without a cap a page with many distinct-Y
    /// header/footer strips can recurse O(n) deep (O(n² log n) work);
    /// `MAX_PARTITION_DEPTH` bounds it.
    fn partition_indexed_depth(&self, all_spans: &[TextSpan], indices: &[usize], depth: u32) -> Vec<Vec<usize>> {
        if indices.is_empty() {
            return Vec::new();
        }

        // Base case: small region, don't split further via the recursive
        // partitioner. Below `min_spans_for_split`, the statistical
        // prose/table classifiers (`classify_region_kind`,
        // `detect_two_column_prose`, `detect_narrow_gutter_prose`) all have
        // their own internal minimum-span floors (6/8/24) far above this
        // one and unconditionally decline to classify — so a flat
        // "impose Y-then-X row-major order" was applied even to a genuine
        // sparse 2-column page (a 2-column, 2-row prose page —
        // 4 spans — read back row-major/interleaved instead of
        // column-major).
        //
        // A geometric gutter check alone can't distinguish "sparse 2-column
        // prose" from "a 2x2 row-major table" at this scale either — both
        // produce an identical clean-gutter signature, and the table-row
        // guard inside `find_horizontal_split_indexed` needs >=3 rows to
        // reach confidence, so it's a no-op at 2 rows. Rather than commit
        // to a (left, right) column grouping we can't justify being
        // correct, fall back to the page's own content-stream emission
        // order when a clean gutter exists at all (PDFium parity, per the
        // reporter's own cross-tool probe: PDFium performs no prose/table
        // decision here either, it just follows stream order). This
        // relies on the empirical tendency of table generators to emit
        // cells row-major and column-generators to emit column-major, in
        // the absence of any other geometric signal being decidable at
        // this scale. Falls back to the flat Y-then-X sort when no clean
        // gutter is found at all (ordinary short single-column snippets). ~keep
        if indices.len() < self.min_spans_for_split {
            if self.find_horizontal_split_indexed(all_spans, indices).is_some() {
                let mut stream_order: Vec<usize> = indices.to_vec();
                stream_order.sort_by_key(|&i| all_spans[i].sequence);
                return vec![stream_order];
            }
            return vec![self.sort_indices(all_spans, indices)];
        }

        if depth >= MAX_PARTITION_DEPTH {
            return vec![self.sort_indices(all_spans, indices)];
        }

        // Two-column-prose probe BEFORE the
        // single-column short-circuit. Tight gutters (~10-15pt) that
        // sit below `min_valley_width` defeat the standard projection-
        // valley detector, and the wide+dense heuristic inside
        // `is_single_column_region` mis-classifies the body as one
        // column because each line's bbox spans the narrow gutter.
        // The probe positively identifies the 2-column-prose shape
        // (gutter-radius left-edge clusters + ≥6 narrow lines +
        // classify_region_kind == Prose) and only fires when ALL of
        // those signals agree. Critically, the Prose gate prevents
        // the false positive that reverted earlier attempts on a
        // 2-column sub-region of the google_doc population table
        // (mean_chars < 8 → Table → bail).
        //
        // **Band-separation first**: when the probe would fire AND a
        // clean vertical band-separation (top header / body / bottom
        // footer) is available, peel the band off BEFORE the column
        // cut. Without this step, full-width header / footer rows
        // get absorbed into one of the two column halves and end up
        // mid-page in reading order — the failure mode on the
        // 1256-page French Bible where the chapter-header
        // band and page-number footer were full-width and span the
        // gutter. The signal for "band": a vertical split whose
        // smaller side has ≤ 25 % of the region's spans (a tight
        // band relative to the body it sits next to).
        // Two-column-prose detector based on line-start clustering.
        // When it fires, peel any wide Y-band first (title / authors
        // / abstract / footer often span the gutter) before the
        // column cut, so they don't get fragmented across columns.
        // Each peeled band is re-classified inside the recursive
        // call.
        //
        // Classify once and pass to both prose detectors below; each gated on
        // `classify_region_kind == Prose` and re-ran the same line clustering. ~keep
        let region_kind = self.classify_region_kind(all_spans, indices);
        if let Some(gutter_x) = self.detect_two_column_prose(all_spans, indices, region_kind) {
            if let Some((above, below)) = self.find_vertical_split_indexed(all_spans, indices) {
                tracing::trace!(
                    above = above.len(),
                    below = below.len(),
                    "peeling Y-band before column cut"
                );
                let mut result = self.partition_indexed_depth(all_spans, &above, depth + 1);
                result.extend(self.partition_indexed_depth(all_spans, &below, depth + 1));
                return result;
            }
            let (left, right): (Vec<usize>, Vec<usize>) = indices
                .iter()
                .copied()
                .partition(|&i| all_spans[i].bbox.left() < gutter_x);
            if !left.is_empty() && !right.is_empty() {
                tracing::trace!(
                    gutter_x,
                    left = left.len(),
                    right = right.len(),
                    "two-column-prose detected"
                );
                let mut result = self.partition_indexed_depth(all_spans, &left, depth + 1);
                result.extend(self.partition_indexed_depth(all_spans, &right, depth + 1));
                return result;
            }
        }

        // Narrow-gutter prose detector — second pass for layouts
        // where the line-start cluster shape is masked by outlier
        // singletons (title / caption / equation rows scattering
        // extra clusters that block the primary detector). Cuts
        // directly at the gap-cluster centre WITHOUT peeling a
        // Y-band first: for these pages `find_vertical_split`
        // tends to fire on mid-body paragraph gaps and bisect
        // the body across the peel — both halves then lose
        // enough gutter signal that the column cut never reaches
        // them on recursion. ~keep
        if let Some(gutter_x) = self.detect_narrow_gutter_prose(all_spans, indices, region_kind) {
            let (left, right): (Vec<usize>, Vec<usize>) = indices
                .iter()
                .copied()
                .partition(|&i| all_spans[i].bbox.left() < gutter_x);
            if !left.is_empty() && !right.is_empty() {
                tracing::trace!(
                    gutter_x,
                    left = left.len(),
                    right = right.len(),
                    "narrow-gutter prose detected"
                );
                let mut result = self.partition_indexed_depth(all_spans, &left, depth + 1);
                result.extend(self.partition_indexed_depth(all_spans, &right, depth + 1));
                return result;
            }
        }

        // Detect single-column body text up-front and skip all spatial
        // splits. Real body text has density dips (indented code, short
        // last-lines, paragraph breaks) that would otherwise trigger
        // spurious horizontal (column) or vertical (row) splits,
        // scrambling reading order. The subsequent sort-by-Y already
        // handles row order within a column. ~keep
        if self.is_single_column_region(all_spans, indices) {
            return vec![self.sort_indices(all_spans, indices)];
        }

        // GH#1808: a figure legend, a running header, or a table's own caption runs the
        // FULL width of the region. `find_horizontal_split_indexed` below assigns content
        // by inked width majority per LINE (GH#1808 fix), which already keeps a torn line
        // whole, but a full-width line still ends up entirely on one side of a column cut
        // that should not have run through it at all -- reading the legend as a body line
        // of whichever column it landed on, instead of as its own band before or after
        // both columns. Peeling such a run off as its own band first, rather than refusing
        // the column cut outright, is what keeps ordinary two-column prose (a full-width
        // title, an abstract header, a running header, a footer) from losing its column
        // split entirely: `partition_region` already does the same for Y-bands via
        // `find_vertical_split_indexed` peeling ahead of `detect_two_column_prose`, for the
        // same reason. ~keep
        if let Some(bands) = self.peel_full_width_line_bands(all_spans, indices) {
            let mut result = Vec::new();
            for band in bands {
                result.extend(self.partition_indexed_depth(all_spans, &band, depth + 1));
            }
            return result;
        }

        let split_h = |s: &Self, sp: &[TextSpan], idx: &[usize]| s.find_horizontal_split_indexed(sp, idx);
        let split_v = |s: &Self, sp: &[TextSpan], idx: &[usize]| s.find_vertical_split_indexed(sp, idx);

        let first_split = if self.prefer_horizontal { split_h } else { split_v };
        let second_split = if self.prefer_horizontal { split_v } else { split_h };

        if let Some((a, b)) = first_split(self, all_spans, indices) {
            let mut result = self.partition_indexed_depth(all_spans, &a, depth + 1);
            result.extend(self.partition_indexed_depth(all_spans, &b, depth + 1));
            return result;
        }

        if let Some((a, b)) = second_split(self, all_spans, indices) {
            let mut result = self.partition_indexed_depth(all_spans, &a, depth + 1);
            result.extend(self.partition_indexed_depth(all_spans, &b, depth + 1));
            return result;
        }

        vec![self.sort_indices(all_spans, indices)]
    }

    /// Classifier verdict for a region — used to gate the tight-gutter
    /// column-split path so the same XY-cut recursion no longer
    /// corrupts table cells (the lesson).
    ///
    /// See the inline post-mortem at lines 73–101: two prior attempts at
    /// the multi-column-prose fix were reverted by the 70-PDF sweep when
    /// they accidentally fired on a 2-column sub-region of a real table
    /// and reordered digits. The fix has to *positively identify prose*
    /// before allowing the tight cut — not merely *fail to identify
    /// table*. This classifier is that positive identification.
    fn classify_region_kind(&self, all_spans: &[TextSpan], indices: &[usize]) -> RegionKind {
        if indices.len() < 6 {
            return RegionKind::Mixed;
        }

        let mut x_min = f32::MAX;
        let mut x_max = f32::MIN;
        for &i in indices {
            x_min = x_min.min(all_spans[i].bbox.left());
            x_max = x_max.max(all_spans[i].bbox.right());
        }
        let region_width = x_max - x_min;
        if region_width <= 10.0 {
            return RegionKind::Mixed;
        }

        let mut lines: std::collections::BTreeMap<i32, (f32, f32, usize)> = std::collections::BTreeMap::new();
        for &i in indices {
            let s = &all_spans[i];
            let y_key = s.bbox.top().round() as i32;
            let nonws_chars = s.text.chars().filter(|c| !c.is_whitespace()).count();
            let entry = lines.entry(y_key).or_insert((f32::MAX, f32::MIN, 0));
            entry.0 = entry.0.min(s.bbox.left());
            entry.1 = entry.1.max(s.bbox.right());
            entry.2 += nonws_chars;
        }

        let line_count = lines.len();
        if line_count < 6 {
            // Too few lines to be a substantial prose body. Headings,
            // captions, single paragraphs all land here — leave them to
            // the default XY-cut behaviour. ~keep
            return RegionKind::Mixed;
        }

        // Per-line statistics: average char count and the count of
        // "narrow" lines whose extent < 0.6 × region_width (a column-half
        // line) and "wide" lines whose extent ≥ 0.6 × region_width (a
        // body-text or table-row line). Table cells are narrow; tables
        // have many such narrow lines but with very short content. ~keep
        let mut total_chars = 0usize;
        let mut narrow_lines = 0usize;
        let mut wide_lines = 0usize;
        for (left, right, chars) in lines.values() {
            total_chars += chars;
            let extent = (*right - *left).max(0.0);
            if extent < region_width * 0.6 {
                narrow_lines += 1;
            } else {
                wide_lines += 1;
            }
        }
        let mean_chars = total_chars as f32 / line_count as f32;

        // PROSE: tall stack of wide lines OR tall stack of half-column
        // lines with substantial content per line.
        //   - mean_chars > 20: real prose, not table cells
        //   - line_count ≥ 6: substantial column
        //   - either:
        //     * majority of lines are wide (single-column body), OR
        //     * majority of lines are narrow with mean_chars > 20
        //       (two half-column lines with prose content) ~keep
        let mostly_wide = wide_lines * 2 > line_count;
        let mostly_narrow = narrow_lines * 2 > line_count;
        if mean_chars > 20.0 && (mostly_wide || mostly_narrow) {
            return RegionKind::Prose;
        }

        // SHORT-LINE PROSE (short-verse two-column bodies): the
        // `mean_chars > 20` guard above deliberately rejected short-verse
        // two-column bodies (Bible / lexicon editions — a verse fragment
        // per column-line is often < 20 non-whitespace chars) along with
        // short-cell tables. The guard was doing two jobs at once. Here we
        // re-admit ONLY the short-line case that carries a *strong central
        // gutter corridor* a short-cell table cannot fake: a single
        // persistent vertical gutter near the region centre, present on a
        // high fraction of lines, with balanced left/right char mass and
        // ≤ 2 left-edge clusters. A label+data table fails this on
        // concentration/coverage (its gaps scatter across cell
        // boundaries), centre (the dominant gap sits off-centre),
        // char-balance (the label column is tiny), or left-edge clusters
        // (≥ 3 columns). The long-line accept path above is byte-unchanged. ~keep
        if mean_chars <= 20.0 && self.short_line_central_corridor_prose(all_spans, indices, x_min, region_width) {
            return RegionKind::Prose;
        }

        // TABLE: lots of narrow lines, short content per line (mean_chars
        // < 8). The google_doc population table —
        // the canonical regression that reverted attempts 1 & 2 — sits
        // squarely here (digit-only cells, ≤ 7 chars each). ~keep
        if mean_chars < 8.0 {
            return RegionKind::Table;
        }

        // Anything in between (e.g. captions with headings, mixed
        // figure-and-text bands) → don't risk the tight cut. ~keep
        RegionKind::Mixed
    }

    /// Short-line two-column-prose admission.
    ///
    /// Called from `classify_region_kind` ONLY for the short-line case
    /// (`mean_chars <= 20`) that the long-line prose guard rejects. A
    /// short-verse two-column body (verse-per-line bibles/lexicons) has
    /// short lines yet a strong, table-independent central gutter; a
    /// short-cell numeric table has short lines and NO such corridor.
    ///
    /// Returns `true` only when ALL of the following hold — each one a
    /// length-independent discriminator a short-cell label+data table
    /// cannot satisfy:
    ///   - a single persistent vertical gutter exists: per-line largest
    ///     within-line gap clusters at one X (10 pt radius) covering
    ///     **≥ 70 %** of gap-bearing lines (concentration) and present on
    ///     **≥ 60 %** of all lines (coverage) — a table's dominant gap
    ///     scatters across cell boundaries and appears on a minority of
    ///     rows;
    ///   - that gutter sits near the region centre: offset ∈
    ///     **[0.30, 0.70]·region_width** — a label+data table's dominant
    ///     gap sits off-centre;
    ///   - **left/right char balance:** non-whitespace char mass on each
    ///     side of the gutter is **≥ 35 %** of the total — a label column
    ///     is lopsided (one side is tiny numeric labels);
    ///   - **≤ 2 left-edge clusters** left of the gutter (30 pt radius) —
    ///     a real two-column body starts each column at one X; an
    ///     N-column table has ≥ 3 left-edge clusters (the fix-534
    ///     `left_edge_clusters >= 3 → Mixed` rule).
    fn short_line_central_corridor_prose(
        &self,
        all_spans: &[TextSpan],
        indices: &[usize],
        x_min: f32,
        region_width: f32,
    ) -> bool {
        if region_width <= 0.0 {
            return false;
        }

        // Re-cluster spans into lines, keeping PER-SPAN (left, right, chars)
        // so we can find the within-line gutter gap and split char mass. ~keep
        let mut lines: std::collections::BTreeMap<i32, Vec<(f32, f32, usize)>> = std::collections::BTreeMap::new();
        for &i in indices {
            let s = &all_spans[i];
            let y_key = s.bbox.top().round() as i32;
            let nonws = s.text.chars().filter(|c| !c.is_whitespace()).count();
            lines
                .entry(y_key)
                .or_default()
                .push((s.bbox.left(), s.bbox.right(), nonws));
        }
        let total_lines = lines.len();
        if total_lines == 0 {
            return false;
        }

        // Per-line: largest within-line gap and its midpoint X. A gap of
        // ≥ 6 pt suppresses ordinary 2–5 pt word spacing. ~keep
        const MIN_GAP_PT: f32 = 6.0;
        let mut gap_positions: Vec<f32> = Vec::new();
        for line_spans in lines.values() {
            if line_spans.len() < 2 {
                continue;
            }
            let mut sorted = line_spans.clone();
            sorted.sort_by(|a, b| crate::utils::safe_float_cmp(a.0, b.0));
            let mut largest_gap = 0.0_f32;
            let mut largest_mid = 0.0_f32;
            for w in sorted.windows(2) {
                let gap = w[1].0 - w[0].1;
                if gap > largest_gap {
                    largest_gap = gap;
                    largest_mid = (w[0].1 + w[1].0) * 0.5;
                }
            }
            if largest_gap >= MIN_GAP_PT {
                gap_positions.push(largest_mid);
            }
        }
        if gap_positions.is_empty() {
            return false;
        }

        const CLUSTER_RADIUS_PT: f32 = 10.0;
        let mut sorted_gaps = gap_positions.clone();
        sorted_gaps.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
        let mut best_size = 0usize;
        let mut best_center = 0.0_f32;
        for &pivot in &sorted_gaps {
            let lo = pivot - CLUSTER_RADIUS_PT;
            let hi = pivot + CLUSTER_RADIUS_PT;
            let mut count = 0usize;
            let mut sum = 0.0_f32;
            for &g in &sorted_gaps {
                if g >= lo && g <= hi {
                    count += 1;
                    sum += g;
                }
            }
            if count > best_size {
                best_size = count;
                best_center = sum / count as f32;
            }
        }
        if best_size == 0 {
            return false;
        }

        // Concentration ≥ 70 % of gap-bearing lines at one X. ~keep
        if best_size * 10 < gap_positions.len() * 7 {
            return false;
        }
        // Coverage ≥ 60 % of ALL lines carry the corridor. ~keep
        if best_size * 10 < total_lines * 6 {
            return false;
        }
        let gutter_offset = best_center - x_min;
        if gutter_offset < region_width * 0.30 || gutter_offset > region_width * 0.70 {
            return false;
        }

        // Left/right non-whitespace char balance about the corridor:
        // each side ≥ 35 % of total. A label-column table is lopsided. ~keep
        let mut left_chars = 0usize;
        let mut right_chars = 0usize;
        for line_spans in lines.values() {
            for &(l, r, chars) in line_spans {
                let mid = (l + r) * 0.5;
                if mid < best_center {
                    left_chars += chars;
                } else {
                    right_chars += chars;
                }
            }
        }
        let total_chars = left_chars + right_chars;
        if total_chars == 0 {
            return false;
        }
        if (left_chars as f32) < total_chars as f32 * 0.35 || (right_chars as f32) < total_chars as f32 * 0.35 {
            return false;
        }

        // ≤ 2 left-edge clusters left of the corridor (30 pt radius). A
        // real two-column body starts its left column at one X (one
        // cluster, maybe two counting a paragraph indent); an N-column
        // table left of the corridor has several cell-start X's → ≥ 3
        // clusters. Cluster EVERY span left-edge that lies left of the
        // corridor (not just each line's minimum) so multi-column cell
        // starts are not collapsed into one cluster. ~keep
        const LEFT_CLUSTER_RADIUS_PT: f32 = 30.0;
        let mut clusters: Vec<(f32, usize)> = Vec::new();
        for line_spans in lines.values() {
            for &(l, _, _) in line_spans {
                if l >= best_center {
                    continue;
                }
                if let Some(c) = clusters
                    .iter_mut()
                    .find(|(c, _)| (*c - l).abs() <= LEFT_CLUSTER_RADIUS_PT)
                {
                    let count = c.1 as f32;
                    c.0 = (c.0 * count + l) / (count + 1.0);
                    c.1 += 1;
                } else {
                    clusters.push((l, 1));
                }
            }
        }
        // Drop singleton/noise clusters (< 2 lines) before counting, so a
        // lone outlier left-edge doesn't inflate the count. ~keep
        let dominant_left_clusters = clusters.iter().filter(|(_, n)| *n >= 2).count();
        if dominant_left_clusters >= 3 {
            return false;
        }

        true
    }

    /// Two-column-prose probe — does this region look like two
    /// side-by-side columns of prose with a tight gutter (~10-15pt)?
    ///
    /// Called from `is_single_column_region` when the wide+dense
    /// heuristic would otherwise short-circuit the region as
    /// single-column. Distinguishing signal: most lines fit inside
    /// **one** half of the region width (column-half lines), and the
    /// left edges cluster into exactly **two** groups separated by
    /// approximately half the region width.
    ///
    /// Gated on `classify_region_kind == Prose` so the same machinery
    /// doesn't fire on a 2-column sub-region of a table (an earlier
    /// failure mode).
    ///
    /// Returns `Some(gutter_x)` when a 2-column prose layout is
    /// detected — the caller treats that as a non-single-column verdict
    /// and lets `find_horizontal_split_indexed` cut at the gutter.
    fn detect_two_column_prose(
        &self,
        all_spans: &[TextSpan],
        indices: &[usize],
        region_kind: RegionKind,
    ) -> Option<f32> {
        if indices.len() < 8 {
            return None;
        }

        let mut x_min = f32::MAX;
        let mut x_max = f32::MIN;
        for &i in indices {
            x_min = x_min.min(all_spans[i].bbox.left());
            x_max = x_max.max(all_spans[i].bbox.right());
        }
        let region_width = x_max - x_min;
        if region_width < 200.0 {
            // Real two-column bodies span at least ~200pt (the
            // narrowest two-column layout in the corpus is ~250pt for a
            // letter-page body inside ~250pt margins). ~keep
            return None;
        }

        // Cluster spans into lines by rounded Y. Keep PER-SPAN
        // (left, right) data so we can detect within-line gaps —
        // the canonical multi-column interleave puts
        // a left-col span (left=82) and a right-col span (left=312)
        // on the same Y baseline. The whole-line bbox.right -
        // bbox.left = 358 pt looks "wide" (358 > 0.6 × 500 = 300)
        // even though each side is a narrow column half. ~keep
        let mut lines_spans: std::collections::BTreeMap<i32, Vec<(f32, f32)>> = std::collections::BTreeMap::new();
        for &i in indices {
            let s = &all_spans[i];
            let y_key = s.bbox.top().round() as i32;
            lines_spans
                .entry(y_key)
                .or_default()
                .push((s.bbox.left(), s.bbox.right()));
        }
        if lines_spans.len() < 6 {
            return None;
        }

        // For each line, find the largest gap between adjacent spans.
        // A line is treated as multiple "half-lines" if a gap ≥ 10 pt
        // splits it; each side of the gap contributes its leftmost-x
        // to `narrow_lefts`. This is the lesson: the row-by-
        // row interleave shape spans the gutter as bbox
        // but has a clear gap within each line. ~keep
        let narrow_threshold = region_width * 0.6;
        let intra_line_gap_threshold = 10.0_f32;
        let mut narrow_lefts: Vec<f32> = Vec::new();
        // Count "narrow" lines for the majority check — a line with
        // a within-line gap contributes 1 to this count regardless of
        // how many half-lines it produces, so the majority threshold
        // stays comparable to single-column reasoning. ~keep
        let mut narrow_line_count = 0usize;
        for line_spans in lines_spans.values() {
            let mut sorted = line_spans.clone();
            sorted.sort_by(|a, b| crate::utils::safe_float_cmp(a.0, b.0));
            let mut largest_gap = 0.0_f32;
            let mut split_idx: Option<usize> = None;
            for (i, w) in sorted.windows(2).enumerate() {
                let gap = w[1].0 - w[0].1;
                if gap > largest_gap {
                    largest_gap = gap;
                    split_idx = Some(i);
                }
            }
            let line_left = sorted.first().map(|(l, _)| *l).unwrap_or(0.0);
            let line_right = sorted.last().map(|(_, r)| *r).unwrap_or(0.0);
            let line_extent = (line_right - line_left).max(0.0);

            if let Some(si) = split_idx
                && largest_gap >= intra_line_gap_threshold
            {
                narrow_lefts.push(line_left);
                if let Some(&(right_side_left, _)) = sorted.get(si + 1) {
                    narrow_lefts.push(right_side_left);
                }
                narrow_line_count += 1;
                continue;
            }

            if line_extent < narrow_threshold {
                narrow_lefts.push(line_left);
                narrow_line_count += 1;
            }
        }
        // Majority of lines must be narrow — otherwise this isn't a
        // 2-column body, it's a single-column body with a few short
        // last-lines. ~keep
        if narrow_line_count * 2 < lines_spans.len() {
            return None;
        }

        // Cluster the narrow left-edges. Two clusters separated by
        // approximately half the region width = 2-column prose. ~keep
        let cluster_radius = 30.0_f32;
        let mut clusters: Vec<(f32, usize)> = Vec::new();
        for &x in &narrow_lefts {
            if let Some(c) = clusters.iter_mut().find(|(c, _)| (*c - x).abs() <= cluster_radius) {
                let count = c.1 as f32;
                c.0 = (c.0 * count + x) / (count + 1.0);
                c.1 += 1;
            } else {
                clusters.push((x, 1));
            }
        }

        // Want exactly 2 substantial clusters separated by ~half-width.
        // ≥ 3 clusters = either a table or a band-mixed region — bail. ~keep
        if clusters.len() != 2 {
            return None;
        }
        clusters.sort_by(|a, b| crate::utils::safe_float_cmp(a.0, b.0));
        let (c1_x, c1_n) = clusters[0];
        let (c2_x, c2_n) = clusters[1];

        // Each cluster needs substantial coverage — ≥ 3 lines, or 20 %
        // of the line count, whichever is larger. Reject lopsided
        // shapes (header + body-paragraph). ~keep
        let min_cluster = 3usize.max(narrow_lefts.len() / 5);
        if c1_n < min_cluster || c2_n < min_cluster {
            return None;
        }

        // Gap between cluster centres ≥ 30 % of region width (the
        // gutter + right-column left-margin). For a tight gutter of
        // ~12pt with two ~250pt columns the gap is ~250pt out of 512pt
        // → ~49 %, well above the floor. ~keep
        let gap = c2_x - c1_x;
        if gap < region_width * 0.30 {
            return None;
        }

        // Positive identification of prose — required by the
        // classifier to avoid the google_doc 2-col table
        // sub-region false positive. ~keep
        if region_kind != RegionKind::Prose {
            return None;
        }

        // Gutter midpoint as the cut. The cluster centres are the left
        // edges of the two columns; the gutter sits between the right
        // edge of column 1 and the left edge of column 2. We don't
        // track right edges per cluster, so approximate the gutter
        // centre as halfway between the two cluster centres — that's
        // close enough; the actual partition uses `bbox.left()` per
        // span so individual spans land cleanly on either side. ~keep
        let gutter_x = (c1_x + c2_x) * 0.5;
        Some(gutter_x)
    }

    /// Second-pass 2-column-prose detector for the narrow-gutter case
    /// that `detect_two_column_prose` (the line-start-cluster detector)
    /// misses.
    ///
    /// Two-column papers that emit body text at character-cluster
    /// granularity (each glyph its own span) confuse the line-start
    /// detector: titles, captions, and equation labels contribute
    /// outlier singleton clusters in addition to the two body
    /// columns, so the `clusters.len() != 2` gate rejects. Their
    /// gutters are also often narrower than `min_valley_width` so
    /// the primary projection-valley path in
    /// `find_horizontal_split_indexed` rejects as well.
    ///
    /// Distinguishing signal that works regardless of outlier rows:
    /// the **largest within-line gap** on each body line lives at
    /// roughly the same X coordinate (the gutter) across a strong
    /// majority of lines. Cluster those gap positions; if one cluster
    /// covers ≥ 60 % of the body lines AND the region classifies as
    /// `Prose`, the page is two-column prose and the cluster centre
    /// is the gutter X.
    ///
    /// Returns the gutter X coordinate (an actual gap position, not
    /// a midpoint estimate) when the pattern is detected.
    ///
    /// The Prose-classifier gate keeps tables out: table rows have
    /// their largest gap at variable X across rows (different cell
    /// widths), so the gap-position cluster never dominates.
    fn detect_narrow_gutter_prose(
        &self,
        all_spans: &[TextSpan],
        indices: &[usize],
        region_kind: RegionKind,
    ) -> Option<f32> {
        if indices.len() < 24 {
            return None;
        }
        let mut x_min = f32::MAX;
        let mut x_max = f32::MIN;
        for &i in indices {
            x_min = x_min.min(all_spans[i].bbox.left());
            x_max = x_max.max(all_spans[i].bbox.right());
        }
        let region_width = x_max - x_min;
        if region_width < 200.0 {
            return None;
        }

        let mut lines: std::collections::BTreeMap<i32, Vec<(f32, f32)>> = std::collections::BTreeMap::new();
        for &i in indices {
            let s = &all_spans[i];
            let y_key = s.bbox.top().round() as i32;
            lines.entry(y_key).or_default().push((s.bbox.left(), s.bbox.right()));
        }
        if lines.len() < 12 {
            return None;
        }

        // For each line, find the largest within-line gap (≥ 6 pt
        // suppresses ordinary word-spacing of 2–5 pt). Record the gap's
        // midpoint X. ~keep
        const MIN_GAP_PT: f32 = 6.0;
        let mut gap_positions: Vec<f32> = Vec::new();
        for line_spans in lines.values() {
            if line_spans.len() < 2 {
                continue;
            }
            let mut sorted = line_spans.clone();
            sorted.sort_by(|a, b| crate::utils::safe_float_cmp(a.0, b.0));
            let mut largest_gap = 0.0_f32;
            let mut largest_mid = 0.0_f32;
            for w in sorted.windows(2) {
                let gap = w[1].0 - w[0].1;
                if gap > largest_gap {
                    largest_gap = gap;
                    largest_mid = (w[0].1 + w[1].0) * 0.5;
                }
            }
            if largest_gap >= MIN_GAP_PT {
                gap_positions.push(largest_mid);
            }
        }

        // Need at least 12 gap-bearing lines to cluster — fewer is
        // statistical noise. ~keep
        if gap_positions.len() < 12 {
            return None;
        }

        // Cluster the gap positions with a 10 pt radius (tight; the
        // gutter is at one specific X with minor line-to-line drift).
        // Sliding-window two-pointer scan over the sorted positions —
        // both `left` and `right` only advance forward, so total
        // work is O(n) instead of the previous O(n²) pivot scan
        // (thesis-style PDFs with hundreds of gap-bearing rows pay
        // visibly in that nested loop). ~keep
        const CLUSTER_RADIUS_PT: f32 = 10.0;
        let mut sorted_gaps = gap_positions.clone();
        sorted_gaps.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
        // Prefix sums let us read window-sum in O(1) given (left, right). ~keep
        let mut prefix: Vec<f32> = Vec::with_capacity(sorted_gaps.len() + 1);
        prefix.push(0.0);
        for &x in &sorted_gaps {
            prefix.push(prefix.last().unwrap() + x);
        }
        let mut best_size = 0usize;
        let mut best_center = 0.0_f32;
        let mut left = 0usize;
        let mut right = 0usize;
        for &pivot in &sorted_gaps {
            while left < sorted_gaps.len() && sorted_gaps[left] < pivot - CLUSTER_RADIUS_PT {
                left += 1;
            }
            while right < sorted_gaps.len() && sorted_gaps[right] <= pivot + CLUSTER_RADIUS_PT {
                right += 1;
            }
            let count = right - left;
            let sum = prefix[right] - prefix[left];
            if count > best_size {
                best_size = count;
                best_center = sum / count as f32;
            }
        }

        // Concentration: ≥ 70 % of gap-bearing lines cluster at the
        // same X. Distinguishes 2-col prose (one gutter) from
        // tables (gaps at several cell boundaries, lower
        // concentration). ~keep
        if best_size * 10 < gap_positions.len() * 7 {
            return None;
        }
        if best_size < 12 {
            return None;
        }
        if best_size * 5 < lines.len() {
            return None;
        }

        let gutter_offset = best_center - x_min;
        if gutter_offset < region_width * 0.2 || gutter_offset > region_width * 0.8 {
            return None;
        }

        // Prose gate — same safety as `detect_two_column_prose`.
        // Tables with narrow cell gaps fail the classifier
        // (`mean_chars < 8` → `Table`), preventing the gap-cluster
        // signal from misfiring on tabular content. Short-verse
        // two-column bodies now also pass this gate: although
        // their `mean_chars <= 20`, `classify_region_kind`'s short-line
        // central-corridor admission arm returns `Prose` for them, so a
        // routed short-verse body is cut here rather than re-collapsed.
        // ~keep
        if region_kind != RegionKind::Prose {
            return None;
        }

        Some(best_center)
    }

    /// Heuristic: does the region look like a single column of body text?
    ///
    /// Called **before** horizontal split attempts. When true, the region
    /// is returned as a single sorted group, bypassing both horizontal
    /// (column) and vertical (row) splits. This prevents XY-Cut from
    /// fragmenting body text at density dips caused by indentation or
    /// short last-lines.
    ///
    /// Detection: cluster spans into lines by rounded top-Y, then count
    /// lines that are both **wide** (extent ≥ 60% region width) and
    /// **dense** (covered ratio ≥ 80%). Body-text lines satisfy both.
    /// Aligned multi-column rows look "wide" because their extent spans
    /// the gutter, but fail the density check because the gutter is empty.
    /// Split `indices` into y-ordered bands at the boundaries of every contiguous run of
    /// FULL-WIDTH lines, or `None` when there is no such run (the overwhelmingly common
    /// case -- the hot path pays one line grouping).
    ///
    /// A line (already a continuous run of ink -- see `group_indices_into_lines`) is
    /// full-width when its inked extent reaches `FULL_WIDTH_LINE_FRACTION` of the region's
    /// own width. GH#1808. ~keep
    fn peel_full_width_line_bands(&self, all_spans: &[TextSpan], indices: &[usize]) -> Option<Vec<Vec<usize>>> {
        let lines = group_indices_into_lines(all_spans, indices);
        if lines.len() < MIN_PEELED_FULL_WIDTH_LINES {
            return None;
        }
        let region_left = indices
            .iter()
            .map(|&i| all_spans[i].bbox.left())
            .fold(f32::MAX, f32::min);
        let region_right = indices
            .iter()
            .map(|&i| all_spans[i].bbox.right())
            .fold(f32::MIN, f32::max);
        let region_width = region_right - region_left;
        if !(region_width.is_finite() && region_width > 0.0) {
            return None;
        }

        let is_full_width = |line: &[usize]| -> bool {
            let line_left = line.iter().map(|&i| all_spans[i].bbox.left()).fold(f32::MAX, f32::min);
            let line_right = line.iter().map(|&i| all_spans[i].bbox.right()).fold(f32::MIN, f32::max);
            (line_right - line_left) / region_width >= FULL_WIDTH_LINE_FRACTION
        };
        let flags: Vec<bool> = lines.iter().map(|line| is_full_width(line)).collect();
        if !flags.contains(&true) {
            return None;
        }

        let mut bands: Vec<Vec<usize>> = Vec::new();
        let mut normal: Vec<usize> = Vec::new();
        let mut peeled_any = false;
        let mut index = 0usize;
        while index < lines.len() {
            if !flags[index] {
                normal.extend(&lines[index]);
                index += 1;
                continue;
            }
            let run_start = index;
            while index < lines.len() && flags[index] {
                index += 1;
            }
            let run = &lines[run_start..index];
            if run.len() >= MIN_PEELED_FULL_WIDTH_LINES {
                if !normal.is_empty() {
                    bands.push(std::mem::take(&mut normal));
                }
                bands.push(run.iter().flatten().copied().collect());
                peeled_any = true;
            } else {
                for line in run {
                    normal.extend(line);
                }
            }
        }
        if !normal.is_empty() {
            bands.push(normal);
        }
        (peeled_any && bands.len() >= 2).then_some(bands)
    }

    /// Assign each of `indices`' visual lines, whole, to the side holding the majority of
    /// its inked width -- never split a single line's spans across `split_x`. A line
    /// entirely on one side is unaffected: its majority side is trivially its only side, so
    /// this partitions identically to a per-span `left edge < split_x` test there. Ties (a
    /// line whose inked width splits exactly evenly) go left, matching the pre-existing
    /// left-edge bias. GH#1808. ~keep
    fn partition_lines_at(&self, all_spans: &[TextSpan], indices: &[usize], split_x: f32) -> (Vec<usize>, Vec<usize>) {
        let lines = group_indices_into_lines(all_spans, indices);
        let mut goes_right: std::collections::HashMap<usize, bool> = std::collections::HashMap::new();
        for line in &lines {
            let mut left_width = 0.0f32;
            let mut right_width = 0.0f32;
            for &i in line {
                let bbox = &all_spans[i].bbox;
                left_width += (bbox.right().min(split_x) - bbox.left()).max(0.0);
                right_width += (bbox.right() - bbox.left().max(split_x)).max(0.0);
            }
            let line_goes_right = right_width > left_width;
            for &i in line {
                goes_right.insert(i, line_goes_right);
            }
        }
        // `Iterator::partition` sends predicate-true items into its FIRST returned vec, so
        // the predicate must test "belongs on the left" to produce `(left, right)` in that
        // order -- inverted, this silently swaps every region's two sides. ~keep
        indices
            .iter()
            .copied()
            .partition(|i| !goes_right.get(i).copied().unwrap_or(false))
    }

    fn straddling_line_count(&self, all_spans: &[TextSpan], indices: &[usize], split_x: f32) -> usize {
        group_indices_into_lines(all_spans, indices)
            .iter()
            .filter(|line| {
                let (left_ink, right_ink) = line.iter().fold((0.0f32, 0.0f32), |(left, right), &i| {
                    let bbox = &all_spans[i].bbox;
                    (
                        left + (bbox.right().min(split_x) - bbox.left()).max(0.0),
                        right + (bbox.right() - bbox.left().max(split_x)).max(0.0),
                    )
                });
                left_ink >= MIN_STRADDLING_INK_PTS && right_ink >= MIN_STRADDLING_INK_PTS
            })
            .count()
    }

    fn is_single_column_region(&self, all_spans: &[TextSpan], indices: &[usize]) -> bool {
        if indices.len() < 3 {
            return false;
        }
        let mut x_min = f32::MAX;
        let mut x_max = f32::MIN;
        for &i in indices {
            x_min = x_min.min(all_spans[i].bbox.left());
            x_max = x_max.max(all_spans[i].bbox.right());
        }
        let region_width = x_max - x_min;
        if region_width <= 10.0 {
            return true;
        }

        // Store both bbox.right and core_right for each span. bbox.right
        // can be over-estimated by extractors (trailing whitespace,
        // stretched advance widths) which makes multi-column lines look
        // like one wide continuous run; core_right (char_count × em) is
        // a conservative fallback used ONLY when adjacent bbox edges
        // overlap (a signal of bbox inflation).
        // ~keep
        let mut lines: std::collections::BTreeMap<i32, Vec<(f32, f32, f32)>> = std::collections::BTreeMap::new();
        for &i in indices {
            let s = &all_spans[i];
            let y_key = s.bbox.top().round() as i32;
            let char_count = s.text.chars().filter(|c| !c.is_whitespace()).count().max(1) as f32;
            let approx_char_width = (s.font_size * 0.45).max(2.5);
            let core_right = s.bbox.left() + char_count * approx_char_width;
            lines
                .entry(y_key)
                .or_default()
                .push((s.bbox.left(), s.bbox.right(), core_right));
        }
        if lines.len() < 3 {
            return false;
        }

        // A real column gutter recurs at roughly the SAME X position
        // across multiple lines. Sparse title-page layouts (Title /
        // Subtitle / Byline) also have wide inter-word gaps, but their
        // gap positions are scattered — not a gutter. Collect all gap
        // positions (mid-gap X), then check whether a consistent cluster
        // of gap positions appears on ≥30% of lines.
        //
        // Gap uses bbox.right, but if adjacent bboxes OVERLAP (classic
        // signature of extractor-inflated bbox widths), re-check with
        // conservative core_right estimates so column detection is not
        // defeated by trailing whitespace inflation. ~keep
        let max_gap = self.min_valley_width;
        let mut gap_positions: Vec<f32> = Vec::new();
        for line_spans in lines.values() {
            let mut sorted = line_spans.clone();
            sorted.sort_by(|a, b| crate::utils::safe_float_cmp(a.0, b.0));
            for w in sorted.windows(2) {
                let bbox_gap = w[1].0 - w[0].1;
                let (effective_gap, gap_end_left) = if bbox_gap < 0.0 {
                    (w[1].0 - w[0].2, w[0].2)
                } else {
                    (bbox_gap, w[0].1)
                };
                if effective_gap >= max_gap {
                    gap_positions.push((gap_end_left + w[1].0) * 0.5);
                }
            }
        }
        // Centered-block guard: a CENTERED title/subtitle/
        // byline block (each line horizontally centered, varying widths)
        // produces accidental gap clusters that look like a column
        // gutter — but it is NOT columnar, and treating it as columns
        // scrambles reading order ("Quarterly Inventory Review" centered
        // title read as 3 columns → "Quarterly" / "Spring" / ... ).
        //
        // The distinguishing signal: a REAL multi-column layout has the
        // left column starting at a consistent left edge across rows
        // (low variance of per-line leftmost x). Centered text has its
        // leftmost x scattered (each line centered with a different
        // width). Compute the spread of per-line leftmost edges; if it
        // is large relative to the region width, the block is centered,
        // not columnar, so do NOT treat the gap cluster as a gutter.
        // Centered iff the per-line leftmost edges do NOT share a common
        // left margin. A left-aligned layout (single column OR real
        // multi-column) has most rows starting at the same x (the left
        // margin), so the largest cluster of leftmost edges covers a
        // majority of lines. Centered text has each line's leftmost edge
        // scattered (different per line), so no cluster dominates.
        //
        // Using a cluster fraction (not raw spread) is robust to rows
        // that only contain right-column content — those push the spread
        // up but do not change the fact that the left margin still
        // dominates the remaining rows. (Raw spread mis-classified the
        // two-column test where the last row held only a right cell.) ~keep
        let looks_centered = {
            let mins: Vec<f32> = lines
                .values()
                .map(|ls| ls.iter().map(|(l, _, _)| *l).fold(f32::MAX, f32::min))
                .collect();
            if mins.len() < 2 {
                false
            } else {
                let tol = 10.0_f32;
                // Largest count of leftmost-edges within ±tol of any single edge.
                // Sort once + binary-search the window instead of the O(k^2)
                // all-pairs scan; the max count is a multiset property so this is
                // identical to the pairwise version. ~keep
                let largest = {
                    let mut sorted = mins.clone();
                    sorted.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
                    sorted
                        .iter()
                        .map(|&a| {
                            let lo = sorted.partition_point(|&x| x < a - tol);
                            let hi = sorted.partition_point(|&x| x <= a + tol);
                            hi - lo
                        })
                        .max()
                        .unwrap_or(0)
                };
                (largest as f32) < (mins.len() as f32) * 0.5
            }
        };

        // A SMALL centered block (title / subtitle / byline — few lines,
        // scattered leftmost edges) is treated as a single column so its
        // lines stay in top-to-bottom order and a centered multi-word
        // title is not split into per-word "columns". Gated
        // to <= 6 lines so it only catches title-page-style blocks: a
        // real multi-column body has many lines and is never classified
        // centered here (its left column starts at a consistent margin,
        // giving a small leftmost-spread anyway). ~keep
        if looks_centered && lines.len() <= 6 {
            return true;
        }

        // Cluster gap positions: count, for each observed gap, how many
        // other gaps fall within ±20pt. If any cluster contains gaps
        // from ≥30% of lines, it's a genuine column gutter. ~keep
        if !gap_positions.is_empty() && !looks_centered {
            let cluster_radius = 20.0_f32;
            // Require ≥3 gap positions (or 20% of lines, whichever is
            // larger) clustered within ±20pt. 20% accommodates pages
            // where header/footer/title rows dilute the body-line count
            // but a real multi-column body still dominates. ~keep
            let min_cluster = (3usize).max(lines.len() / 5);
            // Sort once + binary-search each gap's ±radius window instead of the
            // O(k^2) all-pairs scan. Returns false iff some gap's window holds
            // >= min_cluster gaps — identical to the pairwise version. ~keep
            let mut sorted_gaps = gap_positions.clone();
            sorted_gaps.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
            for &pos in &sorted_gaps {
                let lo = sorted_gaps.partition_point(|&p| p < pos - cluster_radius);
                let hi = sorted_gaps.partition_point(|&p| p <= pos + cluster_radius);
                if hi - lo >= min_cluster {
                    return false;
                }
            }
        }

        let width_threshold = region_width * 0.6;
        let mut wide_dense_lines = 0usize;
        for line_spans in lines.values() {
            let mut sorted = line_spans.clone();
            sorted.sort_by(|a, b| crate::utils::safe_float_cmp(a.0, b.0));
            let extent_left = sorted.first().unwrap().0;
            let extent_right = sorted.iter().map(|(_, r, _)| *r).fold(f32::MIN, f32::max);
            let extent = extent_right - extent_left;
            if extent < width_threshold {
                continue;
            }
            // Use core_right (char-count estimate) rather than bbox.right
            // for coverage. bbox.right is inflated by tab characters and
            // trailing whitespace — tab-expanded table rows would otherwise
            // score 100% coverage and be misidentified as dense body text. ~keep
            let mut covered = 0.0f32;
            let mut last_end = f32::MIN;
            for &(l, _, cr) in &sorted {
                let effective_right = cr.min(extent_right);
                let start = l.max(last_end);
                if effective_right > start {
                    covered += effective_right - start;
                    last_end = effective_right;
                }
            }
            if covered >= extent * 0.8 {
                wide_dense_lines += 1;
            }
        }
        wide_dense_lines * 2 >= lines.len()
    }

    /// Find vertical line (X-axis) split using index-based partitioning.
    ///
    /// Rejects lopsided splits where one side contains fewer than ~10% of
    /// the region's spans — those come from single-column pages where
    /// indentation or stray content creates a spurious density dip at
    /// one edge of the projection, not from a real column boundary.
    fn find_horizontal_split_indexed(
        &self,
        all_spans: &[TextSpan],
        indices: &[usize],
    ) -> Option<(Vec<usize>, Vec<usize>)> {
        let profile = self.horizontal_projection_indexed(all_spans, indices)?;

        let split_x = if let Some((vs, ve, vw)) = self.find_valley(&profile) {
            if vw < self.min_valley_width {
                return None;
            }
            // Deepest point within the valley run, not its midpoint
            // (GH#1763) — see `deepest_valley_point` for why. ~keep
            let x_min = profile.x_min;
            let split_is_clear = |offset: f32| {
                let x = x_min + offset;
                !indices.iter().any(|&i| {
                    let bbox = &all_spans[i].bbox;
                    bbox.left() < x && x < bbox.right()
                })
            };
            x_min + deepest_valley_point(&profile.density, vs, ve, &split_is_clear)
        } else {
            self.find_split_between_peaks(&profile)?
        };

        // Reject splits where either resulting sub-column would be
        // narrower than ~60 pt (about 6 body-text characters at
        // 10 pt). Without this check, XY-cut recursion sub-splits
        // a single body column into sliver sub-blocks at internal
        // whitespace valleys (paragraph indentation, justified-line
        // trailing gaps, isolated short words), turning what should
        // be a clean column-major emit of a multi-column page into
        // a band-chunked stream. PDF spec §9.4.4 mentions "natural
        // reading order" but does not mandate a
        // minimum column width; this is a descriptive heuristic —
        // a real body column holds at least ~6 characters. ~keep
        const MIN_RESULT_WIDTH_PT: f32 = 60.0;
        let mut left_x_min = f32::MAX;
        let mut left_x_max = f32::MIN;
        let mut right_x_min = f32::MAX;
        let mut right_x_max = f32::MIN;
        for &i in indices {
            let l = all_spans[i].bbox.left();
            let r = all_spans[i].bbox.right();
            if l < split_x {
                left_x_min = left_x_min.min(l);
                left_x_max = left_x_max.max(r);
            } else {
                right_x_min = right_x_min.min(l);
                right_x_max = right_x_max.max(r);
            }
        }
        let left_w = left_x_max - left_x_min;
        let right_w = right_x_max - right_x_min;
        if left_w < MIN_RESULT_WIDTH_PT || right_w < MIN_RESULT_WIDTH_PT {
            return None;
        }

        // GH#2110: two or more prose lines that run across the cut -- the paragraphs above and
        // below a two-column table -- have no column to belong to, and assigning each to the
        // side holding most of its ink scatters one paragraph over both columns. Refuse the
        // cut so the row split peels them off as bands first. The margin note that widens the
        // region is why `peel_full_width_line_bands` never saw them as full-width. ~keep
        if self.straddling_line_count(all_spans, indices, split_x) >= MIN_PEELED_FULL_WIDTH_LINES {
            return None;
        }

        // GH#1808: assign whole LINES, not individual spans. A line whose inked spans
        // straddle `split_x` -- a figure legend or a table note broken across the gutter
        // at its own font-run boundaries -- is not column content, and partitioning it
        // span-by-span tears it in two: its own comment used to claim "the column split
        // boundary will still assign them correctly by left edge", which only holds while
        // a full-width LINE is exactly one span. `partition_lines_at` sends the whole line
        // to the side holding the majority of its inked width, so a line entirely on one
        // side partitions exactly as `left edge < split_x` already did (unaffected), and
        // only a straddling line's assignment changes. ~keep
        let (left, right) = self.partition_lines_at(all_spans, indices, split_x);

        if left.is_empty() || right.is_empty() {
            return None;
        }

        // Real column splits produce balanced partitions. A 95/5 split is
        // almost always from edge dips or stray content, not a column. ~keep
        let min_side = (indices.len() / 10).max(2);
        if left.len() < min_side || right.len() < min_side {
            return None;
        }

        // Table-row guard (PMC8025747). A genuine column gutter is a
        // vertical CORRIDOR: the left column's glyphs END before the gutter
        // and the right column's glyphs BEGIN after it, so the two sides are
        // X-disjoint. A data-table row, by contrast, starts at the left
        // margin but its cells run the FULL width of the region; partitioning
        // such rows by left edge throws the wide rows into `left` while the
        // right-hand cells (their own spans) land in `right`. Taking the cut
        // anyway slices the table's rows into shattered left/right cell groups
        // — the canonical PMC8025747 p2 failure (a prose column stacked above
        // a full-width data table), and the google_doc population-table hazard
        // the post-mortem at lines 73–101 records.
        //
        // Table-row SIGNATURE: SEVERAL left-side rows each span the ENTIRE
        // right column — their glyph content reaches past the right column's
        // far edge (`right_x_max`). A data table has MANY full-width rows (the
        // header and every data row run the whole region width), so when rows
        // are bucketed into `left` by their left edge, multiple of them blanket
        // the whole right column. By contrast:
        //   * a genuine left prose / reference column ENDS before the gutter,
        //     so its lines stop well short of `right_x_max` (never counted);
        //   * a single wide mis-split OCR line (alice) yields at most one or
        //     two straddling spans — never the recurring full-width pattern;
        //   * the single-column google_doc population table short-circuits at
        //     `is_single_column_region` and never reaches here.
        // Requiring ≥ 3 such rows is what isolates the real table from those
        // cases, so the guard is SUBTRACTIVE: it only ever REJECTS a column
        // cut that would shred a table (the recursion then falls back to a row
        // cut and reads the table row-major), never adds or reorders anything.
        //
        // `core_right` (left edge + non-whitespace-char count × ~0.5 em) is
        // used instead of `bbox.right` so trailing-whitespace / advance-width
        // bbox inflation on a real left column's last word is not mistaken for
        // a glyph crossing the gutter. `overlap_tol` (~ one body em) lets a
        // single straddling glyph slip past. ~keep
        let mut right_x_max = f32::MIN;
        let mut max_font = 0.0f32;
        for &i in &right {
            right_x_max = right_x_max.max(all_spans[i].bbox.right());
            max_font = max_font.max(all_spans[i].bbox.height.abs());
        }
        let overlap_tol = max_font.max(10.0);
        let full_width_left_rows = left
            .iter()
            .filter(|&&i| {
                let s = &all_spans[i];
                let nonws = s.text.chars().filter(|c| !c.is_whitespace()).count().max(1) as f32;
                let approx_char_width = (s.font_size * 0.45).max(2.5);
                s.bbox.left() + nonws * approx_char_width >= right_x_max - overlap_tol
            })
            .count();
        if full_width_left_rows >= 3 {
            // ≥ 3 left rows each blanket the right column ⇒ this is a table-row
            // slice, not a column gutter. Don't take the column cut; the
            // recursion falls back to a row (horizontal) split and reads the
            // table row-major. ~keep
            return None;
        }

        if self.cuts_label_value_rows(all_spans, indices, split_x) {
            return None;
        }

        Some((left, right))
    }

    /// True when the cut at `split_x` would separate short label/value pairs that share a
    /// row but sit a hair off each other's baseline (a key-value form, a numbered list's
    /// marker from its text, a table's row header from its cells): at least
    /// `MIN_LABEL_VALUE_ROWS` such rows have a left-side run and a right-side run, and most
    /// have one side narrower than `LABEL_RUN_MAX_EM`. Prose columns fail the width test --
    /// a column line is a full column wide -- so a real gutter is unaffected. Cutting here
    /// would read every label before any value, so the caller falls back to a row cut.
    /// Rows on one exact baseline are not counted: they were never joined by the line
    /// grouping either, and their handling is left as it was. ~keep
    fn cuts_label_value_rows(&self, all_spans: &[TextSpan], indices: &[usize], split_x: f32) -> bool {
        let lines = group_indices_into_lines(all_spans, indices);
        let extent = |line: &[usize]| -> (f32, f32, f32, f32) {
            line.iter().fold((f32::MAX, f32::MIN, f32::MIN, 0.0f32), |acc, &i| {
                let span = &all_spans[i];
                (
                    acc.0.min(span.bbox.left()),
                    acc.1.max(span.bbox.right()),
                    acc.2.max(span.bbox.y),
                    acc.3.max(span.font_size),
                )
            })
        };
        let mut paired_rows = 0usize;
        let mut short_rows = 0usize;
        for pair in lines.windows(2) {
            let (left_min, left_max, left_y, left_font) = extent(&pair[0]);
            let (right_min, right_max, right_y, right_font) = extent(&pair[1]);
            let offset = (left_y - right_y).abs();
            if left_max > split_x || right_min < split_x || offset == 0.0 || offset > XYCUT_LINE_Y_TOLERANCE_PTS {
                continue;
            }
            paired_rows += 1;
            let narrow_side = (left_max - left_min).min(right_max - right_min);
            if narrow_side < LABEL_RUN_MAX_EM * left_font.max(right_font) {
                short_rows += 1;
            }
        }
        paired_rows >= MIN_LABEL_VALUE_ROWS && short_rows * 5 >= paired_rows * 3
    }

    /// Fallback column split: find the deepest trough between the two
    /// strongest density peaks. Used when the standard valley detection
    /// fails because narrow table-cell spans partially fill the gutter.
    ///
    /// Returns the split X coordinate (absolute, not relative to x_min) if
    /// a genuine trough exists — i.e., the minimum between the peaks is ≤
    /// 50% of the weaker peak density.
    fn find_split_between_peaks(&self, profile: &ProjectionProfile) -> Option<f32> {
        let density = &profile.density;
        let n = density.len();
        if n < 3 {
            return None;
        }

        // Smooth with a small box filter (window = min_valley_width) to
        // average out individual narrow peaks before finding mass centres. ~keep
        let smooth_window = (self.min_valley_width as usize).max(3);
        let half = smooth_window / 2;

        // Smooth into a reused thread-local buffer instead of a fresh `Vec` per
        // failed-valley node. Window-mean is unchanged. (Confirmed not a source
        // of the p.692 non-determinism: the buffer is cleared+refilled to exactly
        // `n` each call and never read out of range.) ~keep
        thread_local! {
            static SMOOTH_SCRATCH: std::cell::RefCell<Vec<f32>> =
                const { std::cell::RefCell::new(Vec::new()) };
        }
        SMOOTH_SCRATCH.with(|cell| {
            let mut smoothed = cell.borrow_mut();
            smoothed.clear();
            smoothed.extend((0..n).map(|i| {
                let s = i.saturating_sub(half);
                let e = (i + half + 1).min(n);
                let sum: f32 = density[s..e].iter().sum();
                sum / (e - s) as f32
            }));

            // Find the strongest peak in each half. Use `safe_float_cmp` for
            // NaN-safe total ordering — matches the comparator used elsewhere
            // in the reading-order code so `density` sentinel values can't
            // reach a `partial_cmp` that maps them to `Equal`. ~keep
            let mid = n / 2;
            let left_peak = (0..mid).max_by(|&a, &b| crate::utils::safe_float_cmp(smoothed[a], smoothed[b]))?;
            let right_peak = (mid..n).max_by(|&a, &b| crate::utils::safe_float_cmp(smoothed[a], smoothed[b]))?;

            if smoothed[left_peak] == 0.0 || smoothed[right_peak] == 0.0 {
                return None;
            }

            let search_start = left_peak.min(right_peak) + 1;
            let search_end = left_peak.max(right_peak);
            if search_start >= search_end {
                return None;
            }

            let trough_pos =
                (search_start..search_end).min_by(|&a, &b| crate::utils::safe_float_cmp(smoothed[a], smoothed[b]))?;

            let weaker_peak = smoothed[left_peak].min(smoothed[right_peak]);
            if smoothed[trough_pos] > weaker_peak * 0.5 {
                return None;
            }

            if trough_pos < self.min_valley_width as usize || trough_pos + self.min_valley_width as usize > n {
                return None;
            }

            Some(profile.x_min + trough_pos as f32)
        })
    }

    /// Find horizontal line (Y-axis) split using index-based partitioning.
    ///
    /// Returns `(above, below)` where `above` holds spans whose rectangle
    /// edge is at larger Y (higher on page in PDF coordinates) and must be
    /// processed first in reading order. PDF Spec ISO 32000-1:2008 §8.3.2.3
    /// defines the default user-space coordinate system with origin at the
    /// lower-left corner and Y increasing upward.
    fn find_vertical_split_indexed(
        &self,
        all_spans: &[TextSpan],
        indices: &[usize],
    ) -> Option<(Vec<usize>, Vec<usize>)> {
        let profile = self.vertical_projection_indexed(all_spans, indices)?;
        let (valley_start, valley_end, valley_width) = self.find_valley(&profile)?;

        if valley_width < self.min_valley_width {
            return None;
        }

        // Deepest point within the valley run, not its midpoint (GH#1763,
        // same fix as the horizontal split — see `deepest_valley_point`). ~keep
        let y_min = profile.y_min;
        let split_is_clear = |offset: f32| {
            let y = y_min + offset;
            !indices.iter().any(|&i| {
                let bbox = &all_spans[i].bbox;
                bbox.top() < y && y < bbox.bottom()
            })
        };
        let split_y = y_min + deepest_valley_point(&profile.density, valley_start, valley_end, &split_is_clear);

        // `Rect::top()` returns `self.y`, the SMALLER Y coordinate of the
        // normalized rectangle — the method name follows a screen-coordinate
        // convention (Y grows downward) but PDF user space has Y growing
        // upward, so in PDF terms `bbox.top()` is actually the LOWER edge of
        // the glyph's bounding box. The predicate `bbox.top() >= split_y`
        // therefore classifies a span into `above` only when its *lowest*
        // point is already above the split line, i.e. the entire span sits
        // above the cut. Since `split_y` is the midpoint of a horizontal
        // projection valley (an empty band by construction), spans should
        // not straddle it in practice -- and since GH#1763 the chosen point is
        // additionally checked against the real span extents, because a zero in
        // the profile does not by itself mean no glyphs are there. Any span that
        // still straddles (e.g. a tall header glyph whose ascenders dip into the
        // valley) falls into `below`. ~keep
        let (above, below): (Vec<usize>, Vec<usize>) =
            indices.iter().partition(|&&i| all_spans[i].bbox.top() >= split_y);

        if above.is_empty() || below.is_empty() {
            return None;
        }

        // Row (vertical) splits legitimately produce singleton top
        // partitions for lone headers/titles, so we accept down to 1
        // span per side. The column (horizontal) split is stricter since
        // single-span columns are almost always spurious. ~keep
        let min_side = (indices.len() / 10).max(1);
        if above.len() < min_side || below.len() < min_side {
            return None;
        }

        Some((above, below))
    }

    /// Calculate horizontal projection profile from indexed spans.
    fn horizontal_projection_indexed(&self, all_spans: &[TextSpan], indices: &[usize]) -> Option<ProjectionProfile> {
        if indices.is_empty() {
            return None;
        }

        let mut x_min = f32::MAX;
        let mut x_max = f32::MIN;
        let mut y_min = f32::MAX;
        let mut y_max = f32::MIN;

        for &i in indices {
            let span = &all_spans[i];
            x_min = x_min.min(span.bbox.left());
            x_max = x_max.max(span.bbox.right());
            y_min = y_min.min(span.bbox.top());
            y_max = y_max.max(span.bbox.bottom());
        }

        let width = (x_max - x_min).ceil() as usize;
        if width > MAX_PROJECTION_SIZE {
            tracing::warn!(
                width,
                max = MAX_PROJECTION_SIZE,
                "horizontal projection width exceeds MAX_PROJECTION_SIZE, skipping region (degenerate CTM?)"
            );
            return None;
        }
        let mut density = vec![0.0; width];

        // Text extractors frequently over-estimate span bbox widths
        // (trailing whitespace, stretched advance widths). That makes a
        // full-width projection falsely fill the inter-column gutter on
        // multi-column pages. We project each span's TEXT CORE footprint
        // anchored to its LEFT edge (where glyphs actually start), with
        // length proportional to character count. The left edge is
        // reliable; the right edge is not.
        //
        // Additionally, spans whose core width exceeds 55% of the region
        // width are full-width elements (section headers, figure captions,
        // table titles) that span both columns. Including them fills the
        // inter-column gutter in the density array and prevents valley
        // detection. They are excluded from the projection; the column
        // split boundary will still assign them correctly by left edge. ~keep
        let region_width = (x_max - x_min).max(1.0);
        for &i in indices {
            let span = &all_spans[i];
            let height = span.bbox.bottom() - span.bbox.top();
            let char_count = span.text.chars().filter(|c| !c.is_whitespace()).count().max(1);
            // 0.45em per char is a reasonable average across common PDF
            // fonts (Helvetica/Times/Arial at body size) and narrower
            // than the 0.5em advance used for monospace. ~keep
            let approx_char_width = (span.font_size * 0.45).max(2.5);
            let core_width = char_count as f32 * approx_char_width;
            let span_width = span.bbox.right() - span.bbox.left();
            if span_width > region_width * 0.55 {
                continue;
            }
            // Skip isolated single-character/digit spans (table cell values
            // like 'G', 'T', '1', 'A') that scatter across the full X range
            // and fill the column gutter in the density profile. Body text
            // spans always contain multiple characters. ~keep
            if char_count < 2 {
                continue;
            }
            let core_left = span.bbox.left();
            let core_right = (core_left + core_width).min(span.bbox.right());
            let x_start = (core_left - x_min).max(0.0).ceil() as usize;
            let x_end = (core_right - x_min).ceil() as usize;

            for j in x_start..x_end.min(width) {
                density[j] += height;
            }
        }

        Some(ProjectionProfile { density, x_min, y_min })
    }

    /// Calculate vertical projection profile from indexed spans.
    fn vertical_projection_indexed(&self, all_spans: &[TextSpan], indices: &[usize]) -> Option<ProjectionProfile> {
        if indices.is_empty() {
            return None;
        }

        let mut x_min = f32::MAX;
        let mut x_max = f32::MIN;
        let mut y_min = f32::MAX;
        let mut y_max = f32::MIN;

        for &i in indices {
            let span = &all_spans[i];
            x_min = x_min.min(span.bbox.left());
            x_max = x_max.max(span.bbox.right());
            y_min = y_min.min(span.bbox.top());
            y_max = y_max.max(span.bbox.bottom());
        }

        let height = (y_max - y_min).ceil() as usize;
        if height > MAX_PROJECTION_SIZE {
            tracing::warn!(
                height,
                max = MAX_PROJECTION_SIZE,
                "vertical projection height exceeds MAX_PROJECTION_SIZE, skipping region (degenerate CTM?)"
            );
            return None;
        }
        let mut density = vec![0.0; height];

        for &i in indices {
            let span = &all_spans[i];
            let y_start = (span.bbox.top() - y_min).max(0.0).ceil() as usize;
            let y_end = (span.bbox.bottom() - y_min).ceil() as usize;
            let w = span.bbox.right() - span.bbox.left();

            for j in y_start..y_end.min(height) {
                density[j] += w;
            }
        }

        Some(ProjectionProfile { density, x_min, y_min })
    }

    /// Find the widest valley (white space gap) in projection profile.
    ///
    /// Only considers INTERIOR valleys — gaps sandwiched between two
    /// non-empty regions. Leading/trailing empty bands (margin space
    /// outside the actual content extent) are ignored; they represent
    /// page margins, not column gutters, and picking them would produce
    /// meaningless splits.
    fn find_valley(&self, profile: &ProjectionProfile) -> Option<(usize, usize, f32)> {
        if profile.density.is_empty() {
            return None;
        }

        let peak = profile.density.iter().copied().fold(0.0, f32::max);

        if peak == 0.0 {
            return None;
        }

        let first_nonzero = profile.density.iter().position(|&d| d > 0.0)?;
        let last_nonzero = profile.density.iter().rposition(|&d| d > 0.0)?;

        let threshold = peak * self.valley_threshold;
        let mut valleys = Vec::new();
        let mut in_valley = false;
        let mut valley_start = 0;

        for (i, &density) in profile.density.iter().enumerate() {
            if density < threshold {
                if !in_valley {
                    valley_start = i;
                    in_valley = true;
                }
            } else if in_valley {
                valleys.push((valley_start, i));
                in_valley = false;
            }
        }

        if in_valley {
            valleys.push((valley_start, profile.density.len()));
        }

        // Merge adjacent interior valley segments separated by a narrow
        // bridge (≤ half the minimum valley width). A callout box or small
        // figure positioned in the column gutter creates a density bump
        // that splits what should be a single valley into two fragments.
        // Bridging re-joins them so the gap is still recognised as a
        // column boundary. ~keep
        let bridge_limit = (self.min_valley_width / 2.0).ceil() as usize;
        let interior: Vec<(usize, usize)> = valleys
            .into_iter()
            .filter(|&(start, end)| start > first_nonzero && end <= last_nonzero + 1)
            .collect();
        let mut merged: Vec<(usize, usize)> = Vec::with_capacity(interior.len());
        for seg in interior {
            if let Some(last) = merged.last_mut()
                && seg.0 <= last.1 + bridge_limit
            {
                last.1 = last.1.max(seg.1);
                continue;
            }
            merged.push(seg);
        }
        merged
            .into_iter()
            .map(|(start, end)| (start, end, (end - start) as f32))
            .max_by(|a, b| crate::utils::safe_float_cmp(a.2, b.2))
    }

    /// Test-only wrapper exposing `deepest_valley_point` (a free function)
    /// as an associated fn so tests can call it the same way as the other
    /// `#[cfg(test)]` wrappers in this file.
    #[cfg(test)]
    fn deepest_point_wrapper(density: &[f32], start: usize, end: usize) -> f32 {
        deepest_valley_point(density, start, end, &|_| true)
    }

    /// As [`Self::deepest_point_wrapper`], but with the span-straddle check the real
    /// callers supply, so a test can pin that a candidate cutting a span is rejected.
    #[cfg(test)]
    fn deepest_point_wrapper_checked(
        density: &[f32],
        start: usize,
        end: usize,
        split_is_clear: &dyn Fn(f32) -> bool,
    ) -> f32 {
        deepest_valley_point(density, start, end, split_is_clear)
    }

    /// Old (pre-GH#1763) split-point formula, kept only so the fixed
    /// behaviour can be asserted against what the bug used to produce. ~keep
    #[cfg(test)]
    fn legacy_valley_midpoint(start: usize, end: usize) -> f32 {
        (start + end) as f32 / 2.0
    }

    /// Test-only wrapper for horizontal projection on a contiguous slice.
    #[cfg(test)]
    fn horizontal_projection(&self, spans: &[TextSpan]) -> Option<ProjectionProfile> {
        let indices: Vec<usize> = (0..spans.len()).collect();
        self.horizontal_projection_indexed(spans, &indices)
    }

    /// Test-only wrapper for vertical projection on a contiguous slice.
    #[cfg(test)]
    fn vertical_projection(&self, spans: &[TextSpan]) -> Option<ProjectionProfile> {
        let indices: Vec<usize> = (0..spans.len()).collect();
        self.vertical_projection_indexed(spans, &indices)
    }

    /// Sort spans in reading order (top-to-bottom, left-to-right).
    #[cfg(test)]
    fn sort_spans<'a>(&self, spans: &'a [TextSpan]) -> Vec<&'a TextSpan> {
        let mut sorted: Vec<_> = spans.iter().collect();

        sorted.sort_by(|a, b| {
            let y_cmp = crate::utils::safe_float_cmp(b.bbox.top(), a.bbox.top());
            if y_cmp != std::cmp::Ordering::Equal {
                return y_cmp;
            }
            crate::utils::safe_float_cmp(a.bbox.left(), b.bbox.left())
        });

        sorted
    }

    /// Sort indices in reading order (top-to-bottom, left-to-right).
    ///
    /// Uses [`crate::utils::row_aware_span_cmp`]'s row-banded baseline
    /// comparator rather than a strict `bbox.top()` sort (GH#1600). A
    /// subscript or superscript run shares its base run's baseline
    /// (`bbox.y`) to within a fraction of a point but is drawn in a
    /// visibly smaller font, so its `top()` (`y + height`) differs from
    /// the base run's by roughly the height difference — several points,
    /// comfortably more than any OTHER same-row cell's `top()` gap. A
    /// strict `top()` sort therefore treats the subscript as a separate,
    /// lower "line" and can insert an unrelated same-row cell between a
    /// base glyph and its own subscript. `row_aware_span_cmp` quantizes Y
    /// into `ROW_BAND_TOLERANCE_PT`-wide bands before comparing, so runs
    /// sharing a baseline band stay ordered by X regardless of height. ~keep
    fn sort_indices(&self, all_spans: &[TextSpan], indices: &[usize]) -> Vec<usize> {
        let mut sorted: Vec<usize> = indices.to_vec();
        sorted.sort_by(|&a, &b| {
            crate::utils::row_aware_span_cmp(
                all_spans[a].bbox.y,
                all_spans[a].bbox.x,
                all_spans[b].bbox.y,
                all_spans[b].bbox.x,
            )
        });
        sorted
    }
}

/// Internal projection profile representation.
struct ProjectionProfile {
    /// Density values (height or width accumulated per bin)
    density: Vec<f32>,

    /// Origin coordinates
    x_min: f32,
    y_min: f32,
}

impl ReadingOrderStrategy for XYCutStrategy {
    fn apply(&self, spans: Vec<TextSpan>, context: &ReadingOrderContext) -> Result<Vec<OrderedTextSpan>> {
        // Detects multi-line heading runs and routes the
        // partition through synthetic-span space so the splitter treats
        // each wrapped heading as a single atomic block. When no
        // headings are found we use the original index-only path that
        // avoids span clones during recursion. ~keep
        let heading_runs = self.find_heading_runs(&spans, context.column_gutter);

        let index_groups: Vec<Vec<usize>> = if heading_runs.is_empty() {
            let indices: Vec<usize> = (0..spans.len()).collect();
            self.partition_indexed(&spans, &indices)
        } else {
            let (synthetic, synthetic_origin) = self.synthesize_for_partition(&spans, &heading_runs);
            let synth_indices: Vec<usize> = (0..synthetic.len()).collect();
            let synth_groups = self.partition_indexed(&synthetic, &synth_indices);
            // Project synthetic-space groups back to ORIGINAL-span
            // indices (so the move-out below works on the input Vec). ~keep
            synth_groups
                .into_iter()
                .map(|group| {
                    let mut out = Vec::with_capacity(group.len());
                    for synth_idx in group {
                        out.extend(synthetic_origin[synth_idx].iter().copied());
                    }
                    out
                })
                .collect()
        };

        // Build result — moves spans out by index (no extra clone) ~keep
        let mut ordered = Vec::with_capacity(spans.len());
        // Convert spans to indexable storage for O(1) moves ~keep
        let mut span_slots: Vec<Option<TextSpan>> = spans.into_iter().map(Some).collect();
        let mut order_index = 0usize;

        for (group_idx, group) in index_groups.iter().enumerate() {
            for &i in group {
                if let Some(span) = span_slots[i].take() {
                    ordered.push(
                        OrderedTextSpan::with_info(span, order_index, ReadingOrderInfo::xycut()).with_group(group_idx),
                    );
                    order_index += 1;
                }
            }
        }

        Ok(ordered)
    }

    fn name(&self) -> &'static str {
        "XYCutStrategy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;

    fn make_span(x: f32, y: f32, width: f32, height: f32) -> TextSpan {
        make_span_text(x, y, width, height, "test", 12.0)
    }

    /// Like make_span but with realistic body-text density (~72 non-whitespace chars
    /// at 12pt, matching a full Letter-width column). Used when is_single_column_region
    /// must correctly identify a wide single-column page as not multi-column.
    fn make_body_span(x: f32, y: f32, width: f32, height: f32) -> TextSpan {
        // 72 non-whitespace characters at 12pt → core_width = 72 × 5.4 = 388.8pt
        // which is 83% of a 468pt column — enough to pass the 80% dense check. ~keep
        let text = "abcdefghijklmnopqrstuvwxyz".repeat(3); // 78 non-whitespace chars ~keep
        make_span_text(x, y, width, height, &text, 12.0)
    }

    fn make_span_text(x: f32, y: f32, width: f32, height: f32, text: &str, font_size: f32) -> TextSpan {
        use crate::layout::{Color, FontWeight};

        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: text.to_string(),
            bbox: Rect::new(x, y, width, height),
            font_size,
            font_name: "Arial".to_string(),
            font_weight: FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: Color { r: 0.0, g: 0.0, b: 0.0 },
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
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

    #[test]
    fn test_single_column_no_split() {
        let strategy = XYCutStrategy::new();
        let spans = vec![
            make_span(10.0, 100.0, 50.0, 10.0),
            make_span(10.0, 85.0, 50.0, 10.0),
            make_span(10.0, 70.0, 50.0, 10.0),
        ];

        let groups = strategy.partition_region(&spans, None);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 3);
    }

    /// Realistic A4/Letter single-column page: 60 lines of body text,
    /// 14pt leading, one paragraph gap (30pt) mid-page. Only one body
    /// column exists, so XY-Cut must return exactly one group and
    /// preserve top-to-bottom reading order. A density-dip split at the
    /// paragraph gap would fragment the page and non-monotonically
    /// interleave paragraph contents.
    #[test]
    fn test_single_column_body_text_no_fragmentation() {
        let strategy = XYCutStrategy::new();
        let mut spans = Vec::new();
        let line_height = 12.0;
        let leading = 14.0;
        let left = 72.0;
        let right = 540.0;
        let width = right - left;
        let mut y = 720.0;
        for i in 0..60 {
            // Insert a paragraph gap in the middle (30pt, larger than min_valley_width=15pt) ~keep
            if i == 30 {
                y -= 30.0;
            }
            // Use realistic body text density (78 non-whitespace chars at 12pt) so
            // is_single_column_region correctly classifies the region as single-column. ~keep
            spans.push(make_body_span(left, y, width, line_height));
            y -= leading;
        }

        let groups = strategy.partition_region(&spans, None);
        assert_eq!(
            groups.len(),
            1,
            "single-column body text must not be split by XY-Cut (got {} groups)",
            groups.len()
        );
        assert_eq!(groups[0].len(), 60, "all 60 spans must be preserved");

        let mut last_y = f32::MAX;
        for s in &groups[0] {
            assert!(
                s.bbox.top() <= last_y + 0.01,
                "reading order must be top-to-bottom: {} > {}",
                s.bbox.top(),
                last_y
            );
            last_y = s.bbox.top();
        }
    }

    /// After a vertical (row) split, the partition at higher Y (top of
    /// page in PDF coords) must be processed first in reading order so
    /// that header content appears before body content.
    #[test]
    fn test_vertical_split_preserves_top_to_bottom_order() {
        use crate::pipeline::reading_order::{ReadingOrderContext, ReadingOrderStrategy};

        let mut strategy = XYCutStrategy::new();
        strategy.min_spans_for_split = 2;

        let make = |text: &str, x: f32, y: f32, w: f32| {
            let mut s = make_span(x, y, w, 12.0);
            s.text = text.to_string();
            s
        };
        // Two columns at y ∈ {200, 180, 160} (body), header at y=400.
        // Horizontal split will find the column gutter first; within each
        // column the header must still come out first in reading order. ~keep
        let spans = vec![
            make("HEADER LEFT", 50.0, 400.0, 200.0),
            make("HEADER RIGHT", 300.0, 400.0, 200.0),
            make("body-L1", 50.0, 200.0, 150.0),
            make("body-R1", 300.0, 200.0, 150.0),
            make("body-L2", 50.0, 180.0, 150.0),
            make("body-R2", 300.0, 180.0, 150.0),
        ];
        let context = ReadingOrderContext::new();
        let ordered = strategy.apply(spans, &context).unwrap();

        let texts: Vec<&str> = ordered.iter().map(|o| o.span.text.as_str()).collect();
        assert!(
            texts[0].contains("HEADER"),
            "expected HEADER first, got sequence {:?}",
            texts
        );
    }

    /// Single-column page with a tall header band ("Title" or "Chapter
    /// heading") at the top. XY-Cut may validly split the header from
    /// the body (vertical Y-split) but must not further split the body
    /// into per-paragraph chunks.
    #[test]
    fn test_single_column_with_header_at_most_two_groups() {
        let strategy = XYCutStrategy::new();
        let mut spans = Vec::new();

        spans.push(make_span(72.0, 750.0, 468.0, 24.0));

        let mut y = 670.0;
        for _ in 0..40 {
            spans.push(make_span(72.0, y, 468.0, 12.0));
            y -= 14.0;
        }

        let groups = strategy.partition_region(&spans, None);
        assert!(
            groups.len() <= 2,
            "single-column with header should produce at most 2 groups, got {}",
            groups.len()
        );
        let total: usize = groups.iter().map(|g| g.len()).sum();
        assert_eq!(total, 41);
    }

    #[test]
    fn test_two_column_split() {
        let mut strategy = XYCutStrategy::new();
        strategy.min_spans_for_split = 2;

        let spans = vec![
            make_span(10.0, 100.0, 50.0, 10.0),
            make_span(10.0, 85.0, 50.0, 10.0),
            make_span(100.0, 100.0, 50.0, 10.0),
            make_span(100.0, 85.0, 50.0, 10.0),
        ];

        let groups = strategy.partition_region(&spans, None);
        assert!(!groups.is_empty(), "Expected at least 1 group");
        let total_spans: usize = groups.iter().map(|g| g.len()).sum();
        assert_eq!(total_spans, 4, "Expected all 4 spans to be preserved");
    }

    /// GH#1600. A subscript run (shorter height, baseline dropped a
    /// fraction of a point below its base run) must sort immediately after
    /// its base run, not after a sibling cell in the next column whose
    /// `top()` happens to land between them.
    ///
    /// Geometry lifted from the reporter's reproducer (`Q`/`HE`/`GJ`
    /// row): base "Q" at y=612.13 h=11.59 (top=623.72), subscript "HE" at
    /// y=611.50 h=7.34 (top=618.84 — baseline only 0.63pt below the base,
    /// but top() differs by ~4.9pt because subscript glyphs are drawn in a
    /// visibly smaller font), unit cell "GJ" in the next column at
    /// y=612.13 h=11.59 (top=623.72, tied with the base). Sorting by
    /// `top()` descending places GJ (tied top, lower x than nothing to its
    /// left) ahead of HE, tearing "QHE" into "Q" ... "HE" with "GJ" wedged
    /// between them. All three share one baseline band (`ROW_BAND_TOLERANCE_PT`
    /// = 3.0pt covers the 0.63pt baseline gap easily), so a baseline-aware,
    /// row-banded comparator keeps Q and HE adjacent. ~keep
    #[test]
    fn test_subscript_sorts_immediately_after_base_not_after_next_column() {
        let strategy = XYCutStrategy::new();
        let spans = vec![
            make_span_text(206.74, 612.13, 8.20, 11.59, "Q", 11.59),
            make_span_text(212.76, 611.50, 6.83, 7.34, "HE", 7.34),
            make_span_text(253.33, 612.13, 12.08, 11.59, "GJ", 11.59),
        ];

        let groups = strategy.partition_region(&spans, None);
        let texts: Vec<&str> = groups.iter().flatten().map(|s| s.text.as_str()).collect();
        assert_eq!(
            texts,
            vec!["Q", "HE", "GJ"],
            "subscript HE must stay adjacent to base Q, ahead of the next column's GJ"
        );
    }

    /// GH#1600 negative control. Two ordinary body-text lines at a real
    /// line-height apart (14pt — typical single-spaced 12pt body leading)
    /// must NOT be treated as one row band and interleaved by X, even
    /// though they land in the same tiny (`n < min_spans_for_split`)
    /// region that reaches `sort_indices`. `ROW_BAND_TOLERANCE_PT` is
    /// 3.0pt; a 14pt gap is 4.6x that, so the two lines fall into
    /// different bands and stay in top-to-bottom, row-major order. This
    /// guards the fix above from overreaching: the row-band tolerance is
    /// narrow enough to keep a subscript with its base (0.63pt baseline
    /// gap) without also merging two genuinely separate lines whose
    /// columns would otherwise look identical to the subscript case (each
    /// side narrower than `MIN_RESULT_WIDTH_PT`, so no column split is
    /// found and both lines land in the same `sort_indices` call). ~keep
    #[test]
    fn test_row_band_does_not_merge_two_distinct_lines() {
        let strategy = XYCutStrategy::new();
        let spans = vec![
            make_span_text(10.0, 200.0, 50.0, 10.0, "A1", 10.0),
            make_span_text(100.0, 200.0, 50.0, 10.0, "A2", 10.0),
            make_span_text(10.0, 186.0, 50.0, 10.0, "B1", 10.0),
            make_span_text(100.0, 186.0, 50.0, 10.0, "B2", 10.0),
        ];

        let groups = strategy.partition_region(&spans, None);
        let texts: Vec<&str> = groups.iter().flatten().map(|s| s.text.as_str()).collect();
        assert_eq!(
            texts,
            vec!["A1", "A2", "B1", "B2"],
            "two distinct 14pt-apart lines must stay in row-major order, not interleave by X"
        );
    }

    #[test]
    fn test_three_column_layout() {
        let strategy = XYCutStrategy::new();
        // Realistic column widths (≥ 60 pt per column, ≥ 6 body chars at
        // 10 pt — find_horizontal_split rejects narrower splits since
        // body columns are never sliver-wide). ~keep
        let spans = vec![
            make_span(10.0, 100.0, 100.0, 10.0),
            make_span(10.0, 85.0, 100.0, 10.0),
            make_span(180.0, 100.0, 100.0, 10.0),
            make_span(180.0, 85.0, 100.0, 10.0),
            make_span(350.0, 100.0, 100.0, 10.0),
            make_span(350.0, 85.0, 100.0, 10.0),
        ];

        let groups = strategy.partition_region(&spans, None);
        assert!(groups.len() >= 2, "Expected at least 2 groups, got {}", groups.len());
    }

    #[test]
    fn test_small_region_no_split() {
        let strategy = XYCutStrategy::new();
        let spans = vec![make_span(10.0, 100.0, 50.0, 10.0)];

        let groups = strategy.partition_region(&spans, None);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 1);
    }

    #[test]
    fn test_sort_order() {
        let strategy = XYCutStrategy::new();
        let spans = vec![
            make_span(100.0, 70.0, 50.0, 10.0),
            make_span(10.0, 100.0, 50.0, 10.0),
            make_span(100.0, 100.0, 50.0, 10.0),
            make_span(10.0, 70.0, 50.0, 10.0),
        ];

        let sorted = strategy.sort_spans(&spans);

        assert_eq!(sorted[0].bbox.top(), 100.0);
        assert_eq!(sorted[0].bbox.left(), 10.0);
        assert_eq!(sorted[1].bbox.top(), 100.0);
        assert_eq!(sorted[1].bbox.left(), 100.0);
    }

    #[test]
    fn test_horizontal_projection() {
        let strategy = XYCutStrategy::new();
        let spans = vec![make_span(10.0, 100.0, 30.0, 10.0), make_span(100.0, 100.0, 30.0, 10.0)];

        if let Some(profile) = strategy.horizontal_projection(&spans) {
            assert!(!profile.density.is_empty());
            assert!(profile.density.len() >= 120);

            let gap_start = 30;
            let gap_end = 90;
            if gap_end <= profile.density.len() {
                let gap_region = &profile.density[gap_start..gap_end];
                let gap_density: f32 = gap_region.iter().sum();
                assert!(gap_density < 1.0);
            }
        }
    }

    #[test]
    fn test_vertical_projection() {
        let strategy = XYCutStrategy::new();
        let spans = vec![make_span(10.0, 100.0, 50.0, 20.0), make_span(10.0, 50.0, 50.0, 20.0)];

        if let Some(profile) = strategy.vertical_projection(&spans) {
            assert!(!profile.density.is_empty());
            assert!(profile.density.len() > 50);
        }
    }

    #[test]
    fn test_narrow_gap_rejected() {
        let strategy = XYCutStrategy::new();
        let spans = vec![make_span(10.0, 100.0, 30.0, 10.0), make_span(45.0, 100.0, 30.0, 10.0)];

        let groups = strategy.partition_region(&spans, None);
        assert_eq!(groups.len(), 1);
    }

    /// Regression test for Bug 2: degenerate CTM places spans at ~100 trillion PDF points.
    /// horizontal_projection_indexed must return None instead of attempting a
    /// ~100-trillion-element vec allocation (which triggers handle_alloc_error → abort).
    #[test]
    fn test_degenerate_ctm_horizontal_projection_returns_none() {
        let strategy = XYCutStrategy::new();
        // Observed crash coordinate: 99_992_777_785_344 PDF points on a ~3968-point page. ~keep
        let degenerate_x: f32 = 99_992_777_785_344.0;
        let spans = vec![
            make_span(10.0, 100.0, 30.0, 10.0),
            make_span(degenerate_x, 100.0, 30.0, 10.0),
        ];

        let result = strategy.horizontal_projection(&spans);
        assert!(
            result.is_none(),
            "expected None for projection spanning ~100 trillion points, got Some"
        );
    }

    /// Vertical projection must also return None for degenerate CTM y-coordinates.
    #[test]
    fn test_degenerate_ctm_vertical_projection_returns_none() {
        let strategy = XYCutStrategy::new();
        let degenerate_y: f32 = 99_992_777_785_344.0;
        let spans = vec![
            make_span(10.0, 100.0, 30.0, 10.0),
            make_span(10.0, degenerate_y, 30.0, 10.0),
        ];

        let result = strategy.vertical_projection(&spans);
        assert!(
            result.is_none(),
            "expected None for projection spanning ~100 trillion points, got Some"
        );
    }

    /// A CENTERED title/subtitle/byline block (each line
    /// centered, scattered leftmost edges) must NOT be split into
    /// per-word "columns". The centered "Quarterly Inventory Review"
    /// title (3 large words at the same Y with wide gaps) plus centered
    /// subtitle/byline previously aligned accidentally into fake columns,
    /// scrambling reading order. The centered-block guard must keep the
    /// whole block as ONE group so the title line stays intact.
    #[test]
    fn test_issue1_centered_title_block_not_split_into_columns() {
        let strategy = XYCutStrategy::new();
        // Centered title (y=612, fs=28), subtitle (y=572), byline (y=532).
        // Leftmost edges scattered: 145 / 185 / 210 (centered, not columnar). ~keep
        let spans = vec![
            make_span_text(145.0, 612.0, 115.0, 28.0, "Quarterly", 28.0),
            make_span_text(300.0, 612.0, 115.0, 28.0, "Inventory", 28.0),
            make_span_text(430.0, 612.0, 92.0, 28.0, "Review", 28.0),
            make_span_text(185.0, 572.0, 40.0, 14.0, "Spring", 14.0),
            make_span_text(238.0, 572.0, 31.0, 14.0, "2025", 14.0),
            make_span_text(300.0, 572.0, 70.0, 14.0, "Distribution", 14.0),
            make_span_text(210.0, 532.0, 45.0, 10.0, "Northwind", 10.0),
            make_span_text(290.0, 532.0, 34.0, 10.0, "Traders", 10.0),
        ];
        let groups = strategy.partition_region(&spans, None);
        assert_eq!(
            groups.len(),
            1,
            "centered title block must stay one group, got {} groups",
            groups.len()
        );
        let g0: Vec<&str> = groups[0].iter().map(|s| s.text.as_str()).collect();
        let qi = g0.iter().position(|t| *t == "Quarterly").unwrap();
        let ii = g0.iter().position(|t| *t == "Inventory").unwrap();
        let ri = g0.iter().position(|t| *t == "Review").unwrap();
        assert!(qi < ii && ii < ri, "title words out of order: {:?}", g0);
    }

    /// XYCut must assign distinct group_id values to spans in different
    /// spatial partitions so that converters can keep each column's content
    /// contiguous instead of interleaving by Y-coordinate.
    #[test]
    fn test_xycut_group_id_two_column_layout() {
        use crate::pipeline::reading_order::{ReadingOrderContext, ReadingOrderStrategy};

        let mut strategy = XYCutStrategy::new();
        strategy.min_spans_for_split = 2;

        let make = |text: &str, x: f32, y: f32, w: f32| {
            let mut s = make_span(x, y, w, 12.0);
            s.text = text.to_string();
            s
        };
        let spans = vec![
            make("Description", 50.0, 100.0, 150.0),
            make("Amount", 400.0, 100.0, 150.0),
            make("Widget A", 50.0, 120.0, 150.0),
            make("$150.00", 400.0, 120.0, 150.0),
            make("Widget B", 50.0, 140.0, 150.0),
            make("Discount", 400.0, 140.0, 150.0),
            make("$25.00", 400.0, 160.0, 150.0),
        ];

        let context = ReadingOrderContext::new();
        let ordered = strategy.apply(spans, &context).unwrap();

        assert!(
            ordered.iter().all(|s| s.group_id.is_some()),
            "all spans should have group_id set by XYCut"
        );

        let left_groups: Vec<usize> = ordered
            .iter()
            .filter(|s| s.span.bbox.left() < 300.0)
            .map(|s| s.group_id.unwrap())
            .collect();
        let right_groups: Vec<usize> = ordered
            .iter()
            .filter(|s| s.span.bbox.left() >= 300.0)
            .map(|s| s.group_id.unwrap())
            .collect();

        assert!(
            left_groups.windows(2).all(|w| w[0] == w[1]),
            "left column spans should share the same group_id: {:?}",
            left_groups
        );
        assert!(
            right_groups.windows(2).all(|w| w[0] == w[1]),
            "right column spans should share the same group_id: {:?}",
            right_groups
        );

        assert_ne!(
            left_groups[0], right_groups[0],
            "left and right columns should have different group_ids"
        );

        let left_orders: Vec<usize> = ordered
            .iter()
            .filter(|s| s.span.bbox.left() < 300.0)
            .map(|s| s.reading_order)
            .collect();
        let right_orders: Vec<usize> = ordered
            .iter()
            .filter(|s| s.span.bbox.left() >= 300.0)
            .map(|s| s.reading_order)
            .collect();
        let left_max = *left_orders.iter().max().unwrap();
        let right_min = *right_orders.iter().min().unwrap();
        let left_min = *left_orders.iter().min().unwrap();
        let right_max = *right_orders.iter().max().unwrap();
        assert!(
            left_max < right_min || right_max < left_min,
            "columns must be contiguous in reading order: left={:?} right={:?}",
            left_orders,
            right_orders
        );
    }

    /// Plain-text rendering must keep group_id-separated columns as
    /// contiguous blocks, not interleave them by Y-coordinate.
    #[test]
    fn test_group_id_plain_text_no_interleave() {
        use crate::pipeline::reading_order::{ReadingOrderContext, ReadingOrderStrategy};

        let mut strategy = XYCutStrategy::new();
        strategy.min_spans_for_split = 2;

        let make = |text: &str, x: f32, y: f32, w: f32| {
            let mut s = make_span(x, y, w, 12.0);
            s.text = text.to_string();
            s
        };
        let spans = vec![
            make("Description", 50.0, 100.0, 150.0),
            make("Amount", 400.0, 100.0, 150.0),
            make("Widget A", 50.0, 120.0, 150.0),
            make("$150.00", 400.0, 120.0, 150.0),
            make("Widget B", 50.0, 140.0, 150.0),
            make("Discount", 400.0, 140.0, 150.0),
            make("$25.00", 400.0, 160.0, 150.0),
        ];

        let context = ReadingOrderContext::new();
        let ordered = strategy.apply(spans, &context).unwrap();

        let order: Vec<&str> = ordered.iter().map(|o| o.span.text.as_str()).collect();

        for expected in ["Description", "Amount", "Widget A", "$150.00"] {
            assert!(order.contains(&expected), "missing {expected:?}: {order:?}");
        }

        // Each group_id-separated column must occupy one contiguous run of the
        // reading order. Interleaving them by Y — the defect this guards — would
        // scatter each column's spans through the other's. ~keep
        let run_is_contiguous = |members: &[&str]| {
            let mut indices: Vec<usize> = members
                .iter()
                .map(|t| order.iter().position(|s| s == t).expect("span present"))
                .collect();
            indices.sort_unstable();
            indices.windows(2).all(|w| w[1] == w[0] + 1)
        };
        assert!(
            run_is_contiguous(&["Description", "Widget A", "Widget B"]),
            "left column must be one contiguous run: {order:?}"
        );
        assert!(
            run_is_contiguous(&["Amount", "$150.00", "Discount", "$25.00"]),
            "right column must be one contiguous run: {order:?}"
        );
    }

    /// Builder for a bold heading span at a given font size. Used by the
    /// fix-543 tests to construct the "bold/large-font run spanning ≥ 2
    /// lines" shape the pre-partition heading lock must catch.
    fn make_bold_span(x: f32, y: f32, width: f32, text: &str, font_size: f32) -> TextSpan {
        use crate::layout::FontWeight;
        let mut s = make_span_text(x, y, width, font_size, text, font_size);
        s.font_weight = FontWeight::Bold;
        s
    }

    /// fix-543 unit: `find_heading_runs` must detect a 2-line bold
    /// heading whose wrapped tail line sits below the first line with
    /// matching X-extent. Single-line bold spans or paragraph-gap
    /// shapes must NOT be returned.
    #[test]
    fn find_heading_runs_detects_2_line_bold_heading() {
        let strategy = XYCutStrategy::new();

        // Body baseline (12pt regular) — establishes the median. ~keep
        let mut spans = Vec::new();
        let body_left = 72.0;
        let body_width = 200.0;
        let mut y = 720.0;
        for _ in 0..10 {
            spans.push(make_body_span(body_left, y, body_width, 12.0));
            y -= 14.0;
        }

        spans.push(make_bold_span(body_left, 500.0, 180.0, "2.3 Performance and", 14.0));
        spans.push(make_bold_span(
            body_left,
            484.0,
            180.0,
            "Advantages of Vari-linear Network",
            14.0,
        ));

        let runs = strategy.find_heading_runs(&spans, None);
        assert_eq!(runs.len(), 1, "expected exactly one heading run, got {runs:?}");
        assert_eq!(
            runs[0].span_indices.len(),
            2,
            "expected the run to cover both heading lines"
        );

        // A LONE bold span (no second line) must NOT be locked: that
        // case is a single-line heading that XY-cut already handles. ~keep
        let mut spans_single = vec![make_body_span(body_left, 720.0, body_width, 12.0); 5];
        spans_single.push(make_bold_span(body_left, 500.0, 180.0, "Lone Heading", 14.0));
        let runs_single = strategy.find_heading_runs(&spans_single, None);
        assert!(
            runs_single.is_empty(),
            "single-line bold runs must not produce a HeadingRun"
        );
    }

    /// fix-543 unit: the canonical repro shape — left-column 2-line
    /// bold heading whose wrapped tail line Y-overlaps right-column
    /// dense content (table caption + rows). Pre-fix, line 2 of the
    /// heading was bucketed into the RIGHT block; post-fix the lock
    /// keeps both heading lines in the LEFT block, adjacent to the
    /// left-column body paragraph.
    #[test]
    fn partition_keeps_heading_in_left_block() {
        let strategy = XYCutStrategy::new();

        let left_col_x = 72.0_f32;
        let right_col_x = 362.0_f32;
        let col_width = 260.0_f32;

        let mut spans = Vec::new();

        // Left column: 2-line bold heading at Y=500/484, then 8 body
        // lines below at Y=460..360 (so the body paragraph anchors the
        // left block in reading order). ~keep
        spans.push(make_bold_span(left_col_x, 500.0, 180.0, "2.3 Performance and", 14.0));
        spans.push(make_bold_span(
            left_col_x,
            484.0,
            220.0,
            "Advantages of Vari-linear Network",
            14.0,
        ));
        let mut y = 460.0_f32;
        for _ in 0..8 {
            spans.push(make_body_span(left_col_x, y, col_width, 12.0));
            y -= 14.0;
        }

        // Right column: dense table-caption-style content that
        // Y-overlaps the heading's second line (Y=484). The
        // pre-fix block-assignment step pulled the heading's tail
        // into THIS column because the geometry was alone in
        // deciding bucket membership. ~keep
        spans.push(make_body_span(right_col_x, 500.0, col_width, 12.0));
        spans.push(make_body_span(right_col_x, 484.0, col_width, 12.0));
        spans.push(make_body_span(right_col_x, 468.0, col_width, 12.0));
        spans.push(make_body_span(right_col_x, 452.0, col_width, 12.0));
        spans.push(make_body_span(right_col_x, 436.0, col_width, 12.0));
        spans.push(make_body_span(right_col_x, 420.0, col_width, 12.0));
        spans.push(make_body_span(right_col_x, 404.0, col_width, 12.0));

        let groups = strategy.partition_region(&spans, None);

        let heading_first_group = groups
            .iter()
            .position(|g| g.iter().any(|s| s.text.contains("2.3 Performance and")))
            .expect("heading line 1 must land in some group");
        let heading_second_group = groups
            .iter()
            .position(|g| g.iter().any(|s| s.text.contains("Advantages of Vari-linear Network")))
            .expect("heading line 2 must land in some group");

        assert_eq!(
            heading_first_group, heading_second_group,
            "both heading lines must end up in the SAME block — pre-fix \
             they split across left/right column blocks"
        );

        let group = &groups[heading_first_group];
        for s in group {
            assert!(
                s.bbox.left() < right_col_x,
                "heading + body group must stay in the LEFT column; \
                 stray span at x={} (right_col starts at {}): {:?}",
                s.bbox.left(),
                right_col_x,
                s.text
            );
        }
    }

    /// fix-543 unit: a wrapped heading's tail line must stay in its own
    /// column block. Pre-fix the tail was bucketed into the right-hand
    /// column and ordered after that column's body, orphaning it from the
    /// heading it belongs to.
    #[test]
    fn wrapped_heading_tail_stays_in_its_column_block() {
        use crate::pipeline::reading_order::ReadingOrderContext;

        let strategy = XYCutStrategy::new();

        let left_col_x = 72.0_f32;
        let right_col_x = 362.0_f32;
        let col_width = 260.0_f32;

        let mut spans = Vec::new();
        spans.push(make_bold_span(left_col_x, 500.0, 180.0, "Performance and", 14.0));
        spans.push(make_bold_span(
            left_col_x,
            484.0,
            220.0,
            "Advantages of Vari-linear Network",
            14.0,
        ));
        let mut y = 460.0_f32;
        for _ in 0..6 {
            spans.push(make_body_span(left_col_x, y, col_width, 12.0));
            y -= 14.0;
        }
        for ky in [500.0, 484.0, 468.0, 452.0, 436.0, 420.0] {
            spans.push(make_body_span(right_col_x, ky, col_width, 12.0));
        }

        let context = ReadingOrderContext::new();
        let ordered = strategy.apply(spans, &context).expect("apply");
        let order: Vec<&str> = ordered.iter().map(|o| o.span.text.as_str()).collect();

        // Both heading halves must appear, and BOTH must precede any
        // right-column content. Pre-fix, the wrapped-heading tail was
        // bucketed into the right-column block and emitted AFTER the
        // right column's body, then promoted to a fresh heading level
        // by `heading_level_ratio` (since it lost its body
        // continuation) — that's the phantom `### …` in the wrong
        // location. Post-fix the lock keeps both heading lines in the
        // left block adjacent to each other. ~keep
        let pos_first = order
            .iter()
            .position(|t| t.contains("Performance and"))
            .expect("heading line 1 must appear in reading order");
        let pos_second = order
            .iter()
            .position(|t| t.contains("Advantages of Vari-linear Network"))
            .expect("heading line 2 must appear in reading order");

        assert_eq!(
            pos_second,
            pos_first + 1,
            "the wrapped heading's two lines must stay adjacent: {order:?}"
        );
        // Both heading lines belong at the very top of the left-column
        // block, not floating somewhere after the right-column body.
        // (Pre-fix the orphan tail landed deep into the document.) ~keep
        let cap = ((order.len() as f32) * 0.30) as usize;
        assert!(
            pos_second < cap.max(2),
            "heading-line-2 ordered late — likely the pre-fix \
             orphan-in-wrong-column behaviour. pos_second={pos_second}, \
             cap={cap}, order={order:?}"
        );
    }

    /// GH#1738: mid-X between the left column's right edge (237.56) and the
    /// right column's left edge (312.60) on the reproducer's page 1.
    const GH1738_GUTTER_X: f32 = 275.08;

    /// The GH#1738 reproducer's page 1 as a fixture, with the right column's
    /// caption parameterised. `(10.0, 810.40)` is the measured original; the
    /// GH#1757 negative control re-runs the same page with the caption at the
    /// heading's own size and on its row.
    ///
    /// Geometry transcribed verbatim from `PdfDocument::extract_spans` on
    /// page 1 of the GH#1738 reproducer (a two-column A4 page: a bold
    /// numbered heading opening the top of the left column, a bold
    /// caption at the top of the right column, and a body underneath
    /// each) — `x`, `y` (top-origin) and `font_size` are the measured
    /// values, and `y` decreases top-to-bottom exactly like every other
    /// `y` in this file's `dense_two_column_*` fixtures. No position is
    /// invented. ~keep
    fn gh1738_page(caption_font_size: f32, caption_top: f32) -> Vec<TextSpan> {
        let kern_space = |x: f32, y: f32, width: f32, font_size: f32| {
            let mut s = make_span_text(x, y, width, font_size, " ", font_size);
            s.offset_semantic = true;
            s
        };
        let body = |x: f32, y: f32, width: f32, text: &str| make_span_text(x, y, width, 8.3, text, 8.3);

        #[rustfmt::skip]
        let spans = vec![
            make_bold_span(30.07, 809.09, 7.51, "3.", 9.0),
            kern_space(37.58, 809.09, 0.28, 9.0),
            make_bold_span(312.60, caption_top, 129.00, "Fig. 6. Branderdruk (P1-P2)", caption_font_size),
            make_bold_span(49.54, 809.09, 188.02, "INSTRUCTIES VOOR DE GASTECHNISCHE ", 9.0),
            make_bold_span(49.54, 799.39, 67.66, "INSTALLATEUR", 9.0),
            make_bold_span(30.07, 782.02, 12.51, "3.1", 9.0),
            kern_space(42.58, 782.02, 0.28, 9.0),
            make_bold_span(49.54, 782.02, 158.68, "GASAANSLUITING EN INSTALLATIE.", 9.0),
            body(30.10, 764.00, 6.92, "1."),
            body(42.80, 764.00, 224.46, "Werk altijd volgens de laatste eisen van de geldende normen"),
            body(42.80, 755.00, 116.33, "en de plaatselijke voorschriften."),
            body(30.10, 737.00, 6.92, "2."),
            body(42.80, 737.00, 202.33, "Plaats bij te verwachten vuil in het gas bij voorkeur een"),
            body(42.80, 728.00, 78.05, "gaszeef in de leiding."),
            body(30.10, 710.00, 6.92, "3."),
            body(42.80, 710.00, 225.82, "Als het gasblok op dichtheid wordt gecontroleerd, gebeurt dat"),
            body(42.80, 701.00, 189.36, "met een druk van ten hoogste 500 mm waterkolom."),
            body(30.10, 683.00, 6.92, "4."),
            body(42.80, 683.00, 223.45, "De fabrieksafstelling van de tweetrapsregeling bedraagt 21,6"),
            body(42.80, 674.00, 224.47, "kW voor warm water en 14 kW voor de verwarming. Voor het"),
            body(42.80, 665.00, 219.31, "aanpassen van de verwarmingsinstelling, zie het kopje over"),
            body(42.80, 656.00, 199.91, "de tweetrapsregeling. De branderdruk is de uitlaatdruk"),
            body(42.80, 647.00, 220.75, "gemeten ten opzichte van de vuurhaarddruk. Voor de plaats"),
            body(42.80, 638.00, 213.45, "van de meetnippels, zie fig. 5. Sluit voor het meten van de"),
            body(42.80, 629.00, 210.56, "branderdruk de slangen van de drukverschilmeter aan op"),
            body(42.80, 620.00, 196.75, "beide meetnippels. In fig. 6 is het nominale vermogen"),
            body(42.80, 611.00, 113.12, "uitgezet tegen de branderdruk."),
            make_bold_span(30.10, 519.00, 135.65, "Fig. 5. Branderdrukinstelling", 10.0),
            make_bold_span(312.60, 518.40, 79.52, "Tweetrapsregeling", 9.0),
            body(312.60, 500.30, 227.13, "Als de verwarmingsinstallatie meer of minder vermogen nodig"),
            body(312.60, 491.30, 234.65, "heeft dan de fabrieksafstelling van 14 kW, kan de capaciteit van"),
            body(312.60, 482.30, 223.06, "de ketel met de tweetrapsregeling worden aangepast aan de"),
            body(312.60, 473.30, 220.77, "installatie. Op de fabriek wordt de hoogste belasting voor de"),
            body(312.60, 464.30, 234.97, "warmwatervoorziening afgesteld op het nominale vermogen. De"),
            body(312.60, 455.30, 197.63, "tweetrapsregeling (zie fig. 7) wordt als volgt ingesteld:"),
            body(312.60, 446.30, 87.90, "a. Spoel onbekrachtigd."),
            body(322.50, 437.30, 225.84, "Lage belasting instellen met de zeskante stelschroef A (let op"),
            body(322.50, 428.30, 128.87, "dat deze vrij ligt van stelschroef B)."),
            body(312.60, 419.30, 76.36, "b. Spoel bekrachtigd"),
            body(322.50, 410.30, 205.03, "Hoogste belasting controleren en zo nodig afstellen met"),
            body(322.50, 401.30, 50.31, "stelschroef B."),
            body(322.50, 392.30, 191.66, "Stift tegenhouden met een inbussleutel van 2,5 mm."),
            body(312.60, 383.30, 94.84, "c. Stelschroef A aflakken."),
            body(312.60, 374.30, 115.11, "d. Branderdrukken controleren."),
            make_bold_span(312.60, 293.20, 120.10, "Fig. 7. Tweetrapsregeling", 10.0),
        ];
        spans
    }

    /// GH#1738: a numbered heading whose producer set the marker and title
    /// in ONE `TJ` array with a kern for the tab (`[(3.)-1329.5(Title )] TJ`)
    /// must not absorb a right-column caption that Y-overlaps its first
    /// line. The kern becomes a space-only span that is always
    /// `FontWeight::Normal` (see `extractors/text/advance.rs`), which used
    /// to break `find_heading_runs`'s clustering right at the
    /// marker/title boundary: the marker ("3.") was left out of the run
    /// while the wrapped title ("…GASTECHNISCHE" / "INSTALLATEUR")
    /// formed a run on its own, whose narrower union bbox no longer
    /// covered the marker's column position and let the caption's span
    /// land between the run's two original lines once expanded. ~keep
    #[test]
    fn gh1738_kern_tab_heading_does_not_absorb_other_column_caption() {
        use crate::pipeline::reading_order::ReadingOrderContext;

        let strategy = XYCutStrategy::new();
        let spans = gh1738_page(10.0, 810.40);

        let context = ReadingOrderContext::new();
        let ordered = strategy.apply(spans, &context).expect("apply");
        let order: Vec<&str> = ordered.iter().map(|o| o.span.text.as_str()).collect();

        let pos_marker = order
            .iter()
            .position(|&t| t == "3.")
            .expect("the heading marker must appear in reading order");
        let pos_title = order
            .iter()
            .position(|&t| t == "INSTRUCTIES VOOR DE GASTECHNISCHE ")
            .expect("the heading title must appear in reading order");
        let pos_wrap = order
            .iter()
            .position(|&t| t == "INSTALLATEUR")
            .expect("the heading's wrapped second line must appear in reading order");
        let pos_caption = order
            .iter()
            .position(|&t| t == "Fig. 6. Branderdruk (P1-P2)")
            .expect("the other column's caption must appear in reading order");

        assert_eq!(
            pos_title,
            pos_marker + 1,
            "the marker and title must stay adjacent: {order:?}"
        );
        assert_eq!(
            pos_wrap,
            pos_title + 1,
            "the wrapped second line must stay adjacent to the title, not have \
             the other column's caption spliced in between: {order:?}"
        );
        assert!(
            pos_caption < pos_marker || pos_caption > pos_wrap,
            "the other column's caption must not land inside the heading run \
             (marker={pos_marker}, title={pos_title}, wrap={pos_wrap}, \
             caption={pos_caption}): {order:?}"
        );
    }

    /// GH#1757 left column, line 1: the chapter title following the `3.` marker.
    const GH1757_TITLE: &str = "INSTRUKTIES VOOR DE GASTECHNISCHE ";
    /// GH#1757 right column: the section heading that hijacked the run.
    const GH1757_RIGHT_HEADING: &str = "3.2 VERBRANDINGSGASAFVOER EN LUCHTTOEVOER";
    /// GH#1757: mid-X of the gutter between the page's two detected columns
    /// (`Detected 2 columns: [(34.622, 276.310), (276.310, 560.031)]`).
    const GH1757_GUTTER_X: f32 = 276.31;

    /// GH#1757: page 4 of the installation manual (A4, 595 × 842). A numbered
    /// chapter heading wraps to a second line at the top of the LEFT column
    /// while the RIGHT column opens with a section heading in the same face,
    /// 0.25 pt lower. `right_top`, `right_bold` and `right_font_size` select
    /// the rows of the issue's variants table.
    ///
    /// Heading positions are the reporter's measured values, in PDF
    /// coordinates (`y` = box top, decreasing down the page) — the left
    /// heading's two lines at 804.93 / 794.13 and the right heading at
    /// 804.68 are the content stream's own `Tm` operands. Body lines carry
    /// the manual's measured leading with paraphrased text, exactly as the
    /// reporter's reproducer sets them. ~keep
    fn gh1757_wrapped_heading_over_two_columns(
        right_top: f32,
        right_bold: bool,
        right_font_size: f32,
    ) -> Vec<TextSpan> {
        let kern_space = |x: f32, y: f32, width: f32, font_size: f32| {
            let mut s = make_span_text(x, y, width, font_size, " ", font_size);
            s.offset_semantic = true;
            s
        };
        let body = |x: f32, y: f32, width: f32, text: &str| make_span_text(x, y, width, 8.3, text, 8.3);

        let right_heading = if right_bold {
            make_bold_span(322.7, right_top, 237.1, GH1757_RIGHT_HEADING, right_font_size)
        } else {
            make_span_text(
                322.7,
                right_top,
                237.1,
                right_font_size,
                GH1757_RIGHT_HEADING,
                right_font_size,
            )
        };

        // The producer draws the whole RIGHT column as one text object and the
        // whole LEFT column as a second, so the right heading precedes the
        // left heading in span order. ~keep
        let mut spans = vec![right_heading];
        for (i, text) in [
            "Het afvoersysteem en de uitmonding voldoen aan de geldende",
            "norm voor gesloten toestellen met ventilator in een",
            "opstellingsruimte. De afvoerleiding mag op afschot naar het",
            "toestel liggen, want bij de toegestane lengte en de",
            "voorgeschreven mantel ontstaat er geen condens. Een",
            "doorvoer naar buiten ligt op een afschot van ten minste vijf",
            "millimeter per meter naar buiten, zodat er geen regen in kan",
            "lopen. Het toestel vangt zelf geen condens of regenwater op.",
        ]
        .into_iter()
        .enumerate()
        {
            spans.push(body(322.7, 784.8 - (i as f32) * 10.5, 237.3, text));
        }
        spans.push(make_bold_span(
            322.7,
            668.0,
            149.8,
            "3.2.1 AANSLUITING OP DE KETEL",
            9.0,
        ));
        for (i, text) in [
            "Het toestel wordt geleverd met een aansluitset voor een",
            "bovenaansluiting met twee stompen van rond 80 mm. Op",
            "bestelling is een set voor een achteraansluiting leverbaar,",
            "eveneens met twee stompen van rond 80 mm.",
        ]
        .into_iter()
        .enumerate()
        {
            spans.push(body(322.7, 648.2 - (i as f32) * 10.5, 237.3, text));
        }

        spans.push(make_bold_span(34.6, 804.93, 7.5, "3.", 9.0));
        spans.push(kern_space(42.14, 804.93, 0.28, 9.0));
        spans.push(make_bold_span(55.9, 804.93, 185.2, GH1757_TITLE, 9.0));
        spans.push(make_bold_span(55.9, 794.13, 67.6, "INSTALLATEUR", 9.0));
        spans.push(make_bold_span(
            34.6,
            773.3,
            179.9,
            "3.1 GASAANSLUITING EN INSTALLATIE.",
            9.0,
        ));
        for (i, text) in [
            "1. Werk altijd volgens de laatste eisen en de plaatselijke",
            "voorschriften.",
            "2. Plaats bij te verwachten vuil in het gas bij voorkeur een",
            "gaszeef.",
            "3. Een dichtheidscontrole van het gasblok gebeurt met een druk",
            "van ten hoogste 500 mm waterkolom.",
            "4. Heeft de installatie minder vermogen nodig dan de",
            "fabrieksafstelling, dan kan de branderdruk naar de gewenste",
            "capaciteit worden aangepast volgens figuur 3.",
        ]
        .into_iter()
        .enumerate()
        {
            spans.push(body(34.6, 753.5 - (i as f32) * 10.5, 236.8, text));
        }
        spans
    }

    /// Texts of the spans backing each detected heading run, in run order.
    fn heading_run_texts<'a>(spans: &'a [TextSpan], runs: &[HeadingRun]) -> Vec<Vec<&'a str>> {
        runs.iter()
            .map(|r| r.span_indices.iter().map(|&i| spans[i].text.as_str()).collect())
            .collect()
    }

    /// GH#1757: the left column's wrapped chapter heading must be locked as
    /// ONE run even though the right column's section heading sorts between
    /// its two lines. The right heading sits across the detected gutter, so
    /// it is neither folded into the run nor allowed to close it.
    #[test]
    fn wrapped_heading_run_survives_other_column_heading_gh1757() {
        let strategy = XYCutStrategy::new();
        let spans = gh1757_wrapped_heading_over_two_columns(804.68, true, 9.0);

        let runs = strategy.find_heading_runs(&spans, Some(GH1757_GUTTER_X));
        let texts = heading_run_texts(&spans, &runs);

        assert_eq!(
            texts,
            vec![vec!["3.", GH1757_TITLE, "INSTALLATEUR"]],
            "expected exactly one locked heading run covering the marker, the \
             title and its wrapped continuation line"
        );
        assert!(
            !texts.iter().flatten().any(|t| *t == GH1757_RIGHT_HEADING),
            "the other column's heading must not be part of any heading run: {texts:?}"
        );
    }

    /// GH#1757: the skip is gated entirely on a known gutter. A caller with
    /// none must get the pre-GH#1757 behaviour verbatim — the far span folds
    /// in and the run is lost — so that pages nobody classified as
    /// multi-column are untouched.
    ///
    /// A width-based stand-in for the gutter was built and measured, and it
    /// is why this test asserts the defect rather than the fix: on one corpus
    /// document it fired 1385 times with a median gap of 82 pt, half of them
    /// at or above the 79 pt gap of GH#1757's own gutter. No threshold
    /// separates a cross-gutter gap from an in-line one without a gutter to
    /// measure against. The fix is to give every output path the gutter (see
    /// `PdfDocument::detect_column_gutter`), not to guess at one here. ~keep
    #[test]
    fn heading_run_fold_is_unchanged_without_a_known_gutter_gh1757() {
        let strategy = XYCutStrategy::new();
        let spans = gh1757_wrapped_heading_over_two_columns(804.68, true, 9.0);

        assert!(
            strategy.find_heading_runs(&spans, None).is_empty(),
            "with no gutter the far span must still fold in, exactly as before"
        );
    }

    /// GH#1757 at the entry points that actually reach the output lenses.
    ///
    /// `find_heading_runs` runs about a dozen times per page from several
    /// call sites; `postprocess_spans` is only one of them, and threading the
    /// gutter through it alone left the reproducer welded because the text
    /// and markdown lenses read the ordering produced by `partition_region`
    /// and by `apply`. Both must honour the gutter, so both are asserted
    /// here: the marker, its title and the wrapped continuation line come out
    /// adjacent and ahead of the other column's heading. ~keep
    #[test]
    fn output_path_entry_points_honour_the_gutter_gh1757() {
        use crate::pipeline::reading_order::ReadingOrderContext;

        let strategy = XYCutStrategy::new();
        let spans = gh1757_wrapped_heading_over_two_columns(804.68, true, 9.0);

        let partition_groups = strategy.partition_region(&spans, Some(GH1757_GUTTER_X));
        let from_partition: Vec<&str> = partition_groups.iter().flatten().map(|s| s.text.as_str()).collect();

        let context = ReadingOrderContext::new().with_column_gutter(GH1757_GUTTER_X);
        let applied = strategy.apply(spans.clone(), &context).expect("apply");
        let from_apply: Vec<&str> = applied.iter().map(|o| o.span.text.as_str()).collect();

        for (label, order) in [("partition_region", &from_partition), ("apply", &from_apply)] {
            let position = |needle: &str| {
                order
                    .iter()
                    .position(|t| *t == needle)
                    .unwrap_or_else(|| panic!("{label}: {needle:?} missing from {order:?}"))
            };
            let marker = position("3.");
            let title = position(GH1757_TITLE);
            let wrap = position("INSTALLATEUR");
            let other = position(GH1757_RIGHT_HEADING);

            assert_eq!(
                title,
                marker + 1,
                "{label}: marker and title must stay adjacent: {order:?}"
            );
            assert_eq!(
                wrap,
                title + 1,
                "{label}: the wrapped line must follow the title: {order:?}"
            );
            assert!(
                other > wrap,
                "{label}: the other column's heading must not precede or split the \
                 heading run (marker={marker}, title={title}, wrap={wrap}, \
                 other={other}): {order:?}"
            );
        }
    }

    /// GH#1757 control (the reproducer's page 2): the right column's heading
    /// 1.5 pt ABOVE line 1 sorts before it and falls outside the 1 pt
    /// same-line window, so the left heading already locks correctly today.
    /// It must keep doing so.
    #[test]
    fn wrapped_heading_run_intact_when_other_column_heading_sits_higher_gh1757() {
        let strategy = XYCutStrategy::new();
        let spans = gh1757_wrapped_heading_over_two_columns(806.43, true, 9.0);

        assert_eq!(
            heading_run_texts(&spans, &strategy.find_heading_runs(&spans, Some(GH1757_GUTTER_X))),
            vec![vec!["3.", GH1757_TITLE, "INSTALLATEUR"]],
            "the control page's heading run must stay intact"
        );
    }

    /// GH#1757 variants table, the two rows that weld on the stock build:
    /// the right heading on exactly the same baseline as line 1, and the
    /// right heading one point larger. Both must leave the left column's run
    /// whole.
    ///
    /// Asserting the run's exact membership matters here: "the right heading
    /// is in no run" is also true of the defect, which produces no runs at
    /// all. ~keep
    #[test]
    fn other_column_bold_heading_variants_keep_the_run_gh1757() {
        let strategy = XYCutStrategy::new();

        for (right_top, right_font_size, label) in [
            (804.9295_f32, 9.0_f32, "same baseline"),
            (804.68_f32, 10.0_f32, "10 pt"),
        ] {
            let spans = gh1757_wrapped_heading_over_two_columns(right_top, true, right_font_size);
            let texts = heading_run_texts(&spans, &strategy.find_heading_runs(&spans, Some(GH1757_GUTTER_X)));
            assert_eq!(
                texts,
                vec![vec!["3.", GH1757_TITLE, "INSTALLATEUR"]],
                "variant '{label}': the left column's heading run must stay whole"
            );
        }
    }

    /// GH#1757 variants table, the regular-face row. It never welded — a
    /// 9 pt regular span on a 8.3 pt body page is not heading-like, so
    /// clustering rejects it before any same-line test. Green before and
    /// after the fix; pinned so a later widening of `is_heading_like` cannot
    /// quietly turn the other column's opening line into run material.
    ///
    /// The left column's run is still lost on this variant — a far
    /// non-heading-like span BREAKS the run rather than being skipped, which
    /// is the issue's option 3 and is not addressed here. ~keep
    #[test]
    fn other_column_regular_face_line_never_joins_the_run_gh1757() {
        let strategy = XYCutStrategy::new();
        let spans = gh1757_wrapped_heading_over_two_columns(804.68, false, 9.0);

        let texts = heading_run_texts(&spans, &strategy.find_heading_runs(&spans, Some(GH1757_GUTTER_X)));

        assert!(
            !texts.iter().flatten().any(|t| *t == GH1757_RIGHT_HEADING),
            "a regular-face line in the other column must not join a heading run: {texts:?}"
        );
    }

    /// GH#1757 negative control for GH#1738. That test's caption clears
    /// `size_ok` (10 pt against the heading's 9 pt) AND sits 1.31 pt above
    /// the heading row, outside the 1 pt same-line window — two independent
    /// reasons it never reached the same-line fold. Put it at the heading's
    /// own size and 0.25 pt below its row, as GH#1757's page has it, and it
    /// takes exactly the GH#1757 path: the stock build folds it in, the
    /// wrapped line then fails the indent test against it, and the run is
    /// lost. `apply` then welds the whole top row into one line.
    #[test]
    fn gh1738_equal_size_caption_on_the_heading_row_is_not_absorbed_gh1757() {
        use crate::pipeline::reading_order::ReadingOrderContext;

        let strategy = XYCutStrategy::new();
        let spans = gh1738_page(9.0, 808.84);

        assert_eq!(
            heading_run_texts(&spans, &strategy.find_heading_runs(&spans, Some(GH1738_GUTTER_X))),
            vec![vec!["3.", "INSTRUCTIES VOOR DE GASTECHNISCHE ", "INSTALLATEUR"]],
            "the caption must not be folded into the heading run"
        );

        let context = ReadingOrderContext::new().with_column_gutter(GH1738_GUTTER_X);
        let ordered = strategy.apply(spans, &context).expect("apply");
        let order: Vec<&str> = ordered.iter().map(|o| o.span.text.as_str()).collect();

        let position = |needle: &str| {
            order
                .iter()
                .position(|t| *t == needle)
                .unwrap_or_else(|| panic!("{needle:?} must appear in reading order: {order:?}"))
        };
        let pos_marker = position("3.");
        let pos_title = position("INSTRUCTIES VOOR DE GASTECHNISCHE ");
        let pos_wrap = position("INSTALLATEUR");
        let pos_caption = position("Fig. 6. Branderdruk (P1-P2)");

        assert_eq!(
            pos_title,
            pos_marker + 1,
            "the marker and title must stay adjacent: {order:?}"
        );
        assert_eq!(
            pos_wrap,
            pos_title + 1,
            "the wrapped second line must stay adjacent to the title: {order:?}"
        );
        assert!(
            pos_caption < pos_marker || pos_caption > pos_wrap,
            "an equal-size caption on the heading's own row must not land inside \
             the heading run (marker={pos_marker}, title={pos_title}, \
             wrap={pos_wrap}, caption={pos_caption}): {order:?}"
        );
    }

    /// A 2-column body where the gutter is narrower than
    /// `min_valley_width` AND the line-start cluster shape carries
    /// outlier singletons (title / caption / equation labels) so
    /// `detect_two_column_prose` bails on `clusters.len() != 2`.
    /// The narrow-gutter prose detector should catch this via
    /// gap-position clustering.
    #[test]
    fn test_narrow_gutter_prose_with_outlier_singletons() {
        let strategy = XYCutStrategy::new();
        let make_word = |x: f32, y: f32, text: &str| {
            let w = (text.chars().count() as f32 * 5.4).max(3.0);
            make_span_text(x, y, w, 12.0, text, 12.0)
        };

        let mut spans = Vec::new();
        // 14 body lines with a tight gutter at x ≈ 295 (gap is
        // ~10 pt: left column ends at ~285, right column starts at
        // ~305). Each side has multiple per-word spans so the line
        // density is realistic. ~keep
        for i in 0..14 {
            let y = 600.0 - (i as f32) * 14.0;
            let left_words = ["Dwarf", "spheroidal", "galaxies", "of", "the", "Local", "Group", "are"];
            let mut x = 40.0;
            for w in left_words {
                spans.push(make_word(x, y, w));
                x += (w.chars().count() as f32 * 5.4) + 2.5;
            }
            let right_words = [
                "The",
                "Schwarzschild",
                "modeling",
                "technique",
                "offers",
                "another",
                "approach",
                "to",
            ];
            let mut x = 305.0;
            for w in right_words {
                spans.push(make_word(x, y, w));
                x += (w.chars().count() as f32 * 5.4) + 2.5;
            }
        }
        // Outlier singletons (title / caption / equation labels)
        // whose left edges don't align with either column. Under
        // detect_two_column_prose these produce extra clusters and
        // block detection — the narrow-gutter detector should still
        // catch the body via gap-position clustering. ~keep
        spans.push(make_word(145.0, 700.0, "Title text spanning"));
        spans.push(make_word(214.0, 680.0, "Caption that wraps somewhere"));
        spans.push(make_word(455.0, 670.0, "(1)"));
        spans.push(make_word(505.0, 660.0, "(2)"));

        let groups = strategy.partition_region(&spans, None);
        assert!(
            groups.len() >= 2,
            "expected at least 2 groups (column split) for narrow-gutter 2-col body \
             with outlier singletons; got {} group(s)",
            groups.len()
        );

        for (gi, g) in groups.iter().enumerate() {
            let has_left = g.iter().any(|s| s.bbox.left() < 200.0);
            let has_right = g.iter().any(|s| s.bbox.left() >= 305.0);
            assert!(
                !(has_left && has_right),
                "group {} contains spans from both columns — the column split did \
                 not separate them: {:?}",
                gi,
                g.iter().map(|s| (s.text.clone(), s.bbox.left())).collect::<Vec<_>>()
            );
        }
    }

    /// Negative: a single-column body with one large figure caption
    /// produces a strong within-line gap on the caption row but no
    /// recurring gap pattern across body lines. The narrow-gutter
    /// detector must NOT fire (would scramble reading order).
    #[test]
    fn test_narrow_gutter_prose_negative_single_col_with_caption() {
        let strategy = XYCutStrategy::new();
        let make_word = |x: f32, y: f32, text: &str| {
            let w = (text.chars().count() as f32 * 5.4).max(3.0);
            make_span_text(x, y, w, 12.0, text, 12.0)
        };

        let mut spans = Vec::new();
        for i in 0..14 {
            let y = 600.0 - (i as f32) * 14.0;
            let words = [
                "This",
                "is",
                "an",
                "ordinary",
                "single",
                "column",
                "body",
                "paragraph",
                "with",
                "no",
                "interior",
                "gutter",
                "or",
                "wide",
                "gap",
            ];
            let mut x = 40.0;
            for w in words {
                spans.push(make_word(x, y, w));
                x += (w.chars().count() as f32 * 5.4) + 2.5;
            }
        }
        // One row that DOES have a within-line gap (figure caption
        // with a label on the right). This single outlier must not
        // make the page look 2-column. ~keep
        spans.push(make_word(40.0, 410.0, "Figure"));
        spans.push(make_word(80.0, 410.0, "caption"));
        spans.push(make_word(300.0, 410.0, "(continued)"));

        let groups = strategy.partition_region(&spans, None);
        // For a true single-column page, partition_region should
        // return either ONE group or a small number from row/header
        // splits — never a column split that lands left-side spans
        // in one group and right-side spans in another.
        // Count groups that contain at least one body span (x < 100): ~keep
        let body_groups = groups
            .iter()
            .filter(|g| g.iter().any(|s| s.bbox.left() < 100.0 && s.text != "Figure"))
            .count();
        assert!(
            body_groups <= 1,
            "narrow-gutter detector wrongly column-split a single-column body: \
             body spans landed in {} groups",
            body_groups
        );
    }

    /// xycut mirror: a short-verse two-column body —
    /// short tokens per column-line (`mean_chars <= 20`) but a strong
    /// balanced central gutter — must classify as `Prose` (so it gets
    /// cut) and be accepted by `detect_narrow_gutter_prose`, even though
    /// the long-line `mean_chars > 20` guard would reject it.
    #[test]
    fn test_short_verse_two_column_classified_prose_and_cut() {
        let strategy = XYCutStrategy::new();
        let make_word = |x: f32, y: f32, text: &str| {
            let w = (text.chars().count() as f32 * 5.4).max(3.0);
            make_span_text(x, y, w, 12.0, text, 12.0)
        };

        // Two columns: left starts at x=40, right at x=240. Each verse
        // line carries two short, EQUAL-length 4-char tokens per side
        // (8 non-whitespace chars/side → 16 chars/line, so mean_chars
        // ≤ 20 and the long-line prose guard does NOT apply — this
        // exercises the new short-line admission arm). The left column's
        // right edge lands consistently near x≈94 and the gutter gap
        // midpoint is stable at ≈167 every line (region x≈40..≈294,
        // width≈254, gutter offset ≈0.50·width). Stable gap → high
        // corridor concentration; equal token counts → balanced char
        // mass; two tight left-column start X's (40, 72) within one
        // column → ≤ 2 left-edge clusters. Uniform token widths keep the
        // within-line gap midpoint inside the 10 pt clustering radius. ~keep
        let left_lines = [
            ["comm", "lalu"],
            ["crea", "ciel"],
            ["terr", "etai"],
            ["info", "vide"],
            ["surf", "labi"],
            ["espr", "leau"],
        ];
        let right_lines = [
            ["EtD1", "ditq"],
            ["lumi", "soit"],
            ["etla", "fut1"],
            ["Dieu", "vitq"],
            ["bonn", "ilse"],
            ["aral", "obsc"],
        ];
        let mut spans = Vec::new();
        // 24 lines total (4 verse-stanzas of 6) so the body clears the
        // ≥12 gap-bearing-line floor in detect_narrow_gutter_prose. ~keep
        for rep in 0..4 {
            for i in 0..6 {
                let y = 600.0 - ((rep * 6 + i) as f32) * 14.0;
                spans.push(make_word(40.0, y, left_lines[i][0]));
                spans.push(make_word(72.0, y, left_lines[i][1]));
                spans.push(make_word(240.0, y, right_lines[i][0]));
                spans.push(make_word(272.0, y, right_lines[i][1]));
            }
        }

        let indices: Vec<usize> = (0..spans.len()).collect();
        assert_eq!(
            strategy.classify_region_kind(&spans, &indices),
            RegionKind::Prose,
            "short-verse two-column body with a strong balanced central \
             corridor must classify as Prose despite mean_chars <= 20"
        );
        assert!(
            strategy
                .detect_narrow_gutter_prose(&spans, &indices, strategy.classify_region_kind(&spans, &indices),)
                .is_some(),
            "detect_narrow_gutter_prose must accept the routed short-verse \
             body (gutter found) so it is cut at the gutter"
        );
    }

    /// xycut mirror — negative: a short-cell multi-column
    /// numeric table (four narrow digit columns → short cells with ≥ 3
    /// left-edge clusters and scattered within-line gaps) must STILL
    /// classify as `Table` and NOT be accepted for cutting.
    #[test]
    fn test_short_cell_label_table_still_table_not_cut() {
        let strategy = XYCutStrategy::new();
        let make_word = |x: f32, y: f32, text: &str| {
            let w = (text.chars().count() as f32 * 5.4).max(3.0);
            make_span_text(x, y, w, 12.0, text, 12.0)
        };

        // A lopsided label+data table: a tiny numeric label column at
        // x=40 (a single digit, ~1 char) and a wide data column at x=100
        // (~8 chars). The within-line gutter is consistent, so the
        // corridor concentration/coverage/centre guards alone would NOT
        // reject it — but the left/right non-whitespace char balance is
        // grossly lopsided (label side ≈ 11 % of chars, well under the
        // 35 % floor), the length-independent table discriminator. A
        // genuine two-column verse body has balanced sides; this table
        // does not, so the short-line admission must reject it.
        // mean_chars ≈ 9 (≥ 8), so it does NOT fall through the
        // `mean_chars < 8 → Table` branch either — the balance check is
        // what keeps it out of Prose. ~keep
        let mut spans = Vec::new();
        let labels = ["7", "8", "9", "5", "3", "1"];
        let data = ["12345678", "23456781", "34567812", "45678123"];
        for i in 0..24 {
            let y = 600.0 - (i as f32) * 14.0;
            spans.push(make_word(40.0, y, labels[i % labels.len()]));
            spans.push(make_word(100.0, y, data[i % data.len()]));
        }

        let indices: Vec<usize> = (0..spans.len()).collect();
        assert_ne!(
            strategy.classify_region_kind(&spans, &indices),
            RegionKind::Prose,
            "lopsided label+data table must NOT be admitted as Prose \
             (left/right char mass is unbalanced — label column is tiny)"
        );
        assert!(
            strategy
                .detect_narrow_gutter_prose(&spans, &indices, strategy.classify_region_kind(&spans, &indices),)
                .is_none(),
            "detect_narrow_gutter_prose must reject the short-cell table \
             (no central-corridor Prose admission) so it is NOT cut"
        );
    }

    #[test]
    fn test_degenerate_ctm_partition_region_does_not_abort() {
        let strategy = XYCutStrategy::new();
        let degenerate_x: f32 = 99_992_777_785_344.0;
        let spans = vec![
            make_span(10.0, 100.0, 30.0, 10.0),
            make_span(10.0, 85.0, 30.0, 10.0),
            make_span(10.0, 70.0, 30.0, 10.0),
            make_span(10.0, 55.0, 30.0, 10.0),
            make_span(10.0, 40.0, 30.0, 10.0),
            make_span(degenerate_x, 100.0, 30.0, 10.0),
        ];

        let groups = strategy.partition_region(&spans, None);
        let total: usize = groups.iter().map(|g| g.len()).sum();
        assert_eq!(total, spans.len(), "all spans must be preserved");
    }

    /// Many distinct-Y single spans is the singleton-peel pathology. With the
    /// depth cap, `partition_region` must still terminate and preserve every
    /// span (the cap falls back to a flat sort, which keeps all indices).
    #[test]
    fn test_partition_indexed_depth_guard_preserves_all_spans() {
        let mut strategy = XYCutStrategy::new();
        strategy.min_spans_for_split = 2;

        // 300 spans, each on its own Y band — deeper than MAX_PARTITION_DEPTH. ~keep
        let spans: Vec<TextSpan> = (0..300)
            .map(|i| make_span(10.0, (i as f32) * 11.0, 30.0, 10.0))
            .collect();

        let groups = strategy.partition_region(&spans, None);
        let total: usize = groups.iter().map(|g| g.len()).sum();
        assert_eq!(total, spans.len(), "depth guard must not drop spans");
    }

    /// Reproduces GH#1763: a figure-caption block (left) beside a body
    /// column (right) whose true empty gutter sits OFF the interior
    /// valley's arithmetic midpoint.
    ///
    /// Geometry (all X in points, region x_min = 0):
    ///   - `CAP1`/`CAP2` (2/3 non-ws chars, 10pt font ⇒ core width 9/13.5pt,
    ///     both left-edge 0) overlap at bin 0, giving density 20 there —
    ///     ABOVE the run's threshold (18 = 0.3 × peak 60) so `find_valley`'s
    ///     interior filter (`start > first_nonzero`) admits the run that
    ///     follows instead of treating the whole region as one leading
    ///     margin. This is the "super-threshold strip at the left content
    ///     edge" the bug fix's interior-run gate requires.
    ///   - `FIG.3.A` (bold, 14pt, 7 non-ws chars ⇒ core width 44.1pt,
    ///     left-edge 60) and `CAP4`/`CAP5` (30/25 non-ws chars, 10pt,
    ///     left-edges 150/320) are ragged sub-threshold caption content —
    ///     real ink, never above 18 density, ending at x = 342 (CAP5's
    ///     core right edge = 320 + 25×4.5 = 432.5, ceil 433 — the LAST
    ///     content before the true gutter).
    ///   - `BODY1..BODY6` (56 non-ws chars, 10pt ⇒ core width 252pt,
    ///     left-edge 500, six identical lines) set the peak: 6 × 10 = 60.
    ///
    /// The resulting horizontal-projection run below threshold spans bins
    /// [9, 500) (width 491, comfortably the widest and only interior
    /// valley — `BODY` is uniform so it contributes no valley of its
    /// own). Within that run the true empty gutter is [433, 500) (width
    /// 67) — clearly off-center: the run's own arithmetic midpoint,
    /// (9 + 500) / 2 = 254.5, lands inside `CAP4`'s span (density 10 at
    /// that x), not in the empty band.
    ///
    /// All five upstream column/prose detectors decline on this fixture
    /// before reaching the valley split, so the bug path is genuinely
    /// exercised: `detect_two_column_prose` sees 5 left-edge clusters
    /// (0, 60, 150, 320, 500 — the staggered caption starts plus the
    /// body's own), not the exactly-2 it requires; `detect_narrow_gutter_prose`
    /// declines outright (11 spans < its 24-span floor); and
    /// `is_single_column_region` returns false because no single line's
    /// extent reaches 60% of the 752pt region width. ~keep
    fn gh1763_page() -> Vec<TextSpan> {
        let cap1 = make_span_text(0.0, 740.0, 9.0, 10.0, "c1", 10.0);
        let cap2 = make_span_text(0.0, 725.0, 13.5, 10.0, "c2z", 10.0);
        let fig3 = make_bold_span(60.0, 760.0, 44.1, "FIG.3.A", 14.0);
        let cap4 = make_span_text(150.0, 705.0, 135.0, 10.0, &format!("CAP4{}", "x".repeat(26)), 10.0);
        let cap5 = make_span_text(320.0, 685.0, 112.5, 10.0, &format!("CAP5{}", "x".repeat(21)), 10.0);

        let mut spans = vec![cap1, cap2, fig3, cap4, cap5];
        for (i, y) in [655.0, 635.0, 615.0, 595.0, 575.0, 555.0].into_iter().enumerate() {
            spans.push(make_span_text(
                500.0,
                y,
                252.0,
                10.0,
                &format!("BODY{}{}", i, "x".repeat(51)),
                10.0,
            ));
        }
        spans
    }

    /// RED-then-GREEN unit test for the GH#1763 fix. Asserts the profile,
    /// the chosen valley run, and the resulting split coordinate
    /// explicitly — not just inferred from final group membership. Before
    /// the fix, `find_horizontal_split_indexed` used
    /// `legacy_valley_midpoint(vs, ve)` (254.5) here; that value falls
    /// inside `CAP4`'s span, which this test also pins down. ~keep
    /// Center of the widest zero-density sub-run in the GH#1763 fixture's
    /// valley. It falls BETWEEN two bins because that sub-run has odd
    /// width; both neighbouring bins are asserted empty below, which is
    /// the property that matters. Pinned as a constant so the split-point
    /// assertion and the density lookups that prove it lands in real empty
    /// space cannot drift apart. ~keep
    const DEEPEST_POINT: f32 = 466.5;

    /// The GH#1763 fix must be a NO-OP on a uniformly empty valley run --
    /// the ordinary case of a real column gutter. It is not enough that the
    /// new split be "close": `find_horizontal_split_indexed` feeds the value
    /// straight into a coordinate comparison, so a half-unit shift reassigns
    /// any span whose edge falls in between. An earlier revision computed the
    /// sub-run center as `start + width / 2` in `usize`, which truncates and
    /// moved the split by 0.5 on EVERY odd-width run -- silently changing the
    /// common case this fix exists to leave alone. ~keep
    /// The guard that keeps the GH#1763 relocation to pages that actually have the defect.
    /// Here the midpoint already falls in empty space, and a WIDER empty region sits
    /// off-centre. Relocating would be pointless -- the split was cutting nothing where it
    /// was -- and it is exactly this case that made the unguarded fix rewrite the reading
    /// order of 23 of 230 corpus documents, 11 of which got worse by dictionary-valid word
    /// count. The split must not move. ~keep
    /// A zero in the projection does not mean no glyphs sit there:
    /// `horizontal_projection_indexed` omits spans under two non-whitespace characters,
    /// spans wider than 55% of the region, and everything past a span's estimated text core.
    /// Seeking the deepest point walks straight into those blind spots, and the corpus showed
    /// the result -- words coming apart at single-character spans, "virgin" into "v" +
    /// "irgin". A candidate that cuts a real span must be rejected in favour of the next, and
    /// a run whose candidates all cut something must keep the midpoint. ~keep
    #[test]
    fn a_candidate_that_would_cut_a_span_is_rejected() {
        // content | gap A (narrow, clear) | content over the midpoint | gap B (wider, crossed)
        let mut density = vec![0.0f32; 40];
        for bin in (0..10).chain(18..23) {
            density[bin] = 3.0;
        }
        let (start, end) = (0usize, 40usize);
        // The midpoint must sit ON content, or the floor guard returns it before any
        // candidate is considered and this pins nothing.
        assert_eq!(density[20], 3.0, "midpoint must be on content for this test to bite");

        // Gap B (23..40, width 17) beats gap A (10..18, width 8) on width, but an invisible
        // span -- one the projection omitted -- runs straight through gap B's centre.
        let invisible_span = 28.0f32..35.0f32;
        let clear = |offset: f32| !(invisible_span.start < offset && offset < invisible_span.end);

        assert_eq!(
            XYCutStrategy::deepest_point_wrapper(&density, start, end),
            31.5,
            "unchecked, the widest gap wins and the split lands inside the hidden span"
        );
        assert_eq!(
            XYCutStrategy::deepest_point_wrapper_checked(&density, start, end, &clear),
            14.0,
            "checked, the split falls back to the narrower gap that cuts nothing"
        );
    }

    /// When every candidate would cut a span, the split must stay exactly where it was
    /// before GH#1763 -- the midpoint -- rather than picking the least-bad cut. ~keep
    #[test]
    fn a_run_whose_candidates_all_cut_something_keeps_the_midpoint() {
        let mut density = vec![0.0f32; 40];
        for bin in (0..10).chain(18..23) {
            density[bin] = 3.0;
        }
        let (start, end) = (0usize, 40usize);
        let midpoint = XYCutStrategy::legacy_valley_midpoint(start, end);
        assert_eq!(
            density[20], 3.0,
            "midpoint must be on content, or the floor guard decides this"
        );

        assert_eq!(
            XYCutStrategy::deepest_point_wrapper_checked(&density, start, end, &|_| false),
            midpoint,
            "no clear candidate means the pre-GH#1763 midpoint stands"
        );
    }

    #[test]
    fn a_split_already_falling_through_empty_space_does_not_move() {
        // content | narrow gap (holds the midpoint) | content | WIDER gap, off-centre
        let mut density = vec![0.0f32; 40];
        for bin in (0..17).chain(23..26) {
            density[bin] = 3.0;
        }
        let (start, end) = (0usize, 40usize);
        let midpoint = XYCutStrategy::legacy_valley_midpoint(start, end);
        assert_eq!(midpoint, 20.0);
        assert_eq!(density[20], 0.0, "the midpoint must start out in empty space");
        assert!(
            (26..40).len() > (17..23).len(),
            "the off-centre empty region must be the WIDER one, or this pins nothing"
        );

        assert_eq!(
            XYCutStrategy::deepest_point_wrapper(&density, start, end),
            midpoint,
            "a split already falling through empty space must stay where it is"
        );
    }

    #[test]
    fn uniform_run_split_is_unchanged_from_the_legacy_midpoint() {
        // This pins a behavioural guarantee -- a uniformly empty run splits exactly where it
        // always did -- and not the centre arithmetic, which it cannot reach: a uniform run's
        // midpoint is by definition at the density floor, so the floor guard returns first.
        // `a_candidate_that_would_cut_a_span_is_rejected` is what pins the f32 centre.
        // The runs are still sized to clear the sparse-run gate so the guarantee is delivered
        // by the floor guard rather than by short-circuiting earlier still.
        for (start, end) in [(3usize, 8usize), (3, 9), (0, 5), (0, 4), (10, 17), (2, 5), (1, 64)] {
            let density = vec![0.0f32; end];
            let share = (end - start) as f32 / end as f32;
            assert!(
                share > SPARSE_VALLEY_REGION_SHARE,
                "run [{start},{end}) is {share} of its region; the sparse gate would short-circuit it"
            );
            assert_eq!(
                XYCutStrategy::deepest_point_wrapper(&density, start, end),
                XYCutStrategy::legacy_valley_midpoint(start, end),
                "uniformly empty run [{start},{end}) must split exactly where it always did"
            );
        }
    }

    /// A below-threshold run narrow enough to BE a gutter keeps its midpoint without the
    /// candidate search running at all -- that is the case GH#1763 must not disturb, and it
    /// is most of the corpus. ~keep
    #[test]
    fn a_run_narrow_enough_to_be_a_gutter_keeps_its_midpoint() {
        // 20 empty bins in a 200-bin region: 10%, the shape of a real column gutter. An
        // off-centre single-bin dip would otherwise win the candidate search.
        let mut density = vec![5.0f32; 200];
        for bin in 90..110 {
            density[bin] = 1.0;
        }
        density[92] = 0.0;
        let (start, end) = (90usize, 110usize);
        assert!(
            ((end - start) as f32) <= density.len() as f32 * SPARSE_VALLEY_REGION_SHARE,
            "this run must be narrow enough for the gate to fire, or the test pins nothing"
        );
        assert_eq!(
            XYCutStrategy::deepest_point_wrapper(&density, start, end),
            XYCutStrategy::legacy_valley_midpoint(start, end),
            "a gutter-width run must split at its midpoint, as it always did"
        );
    }

    /// The GH#1763 reporter could not supply a reproducing PDF, but did attach the
    /// horizontal projection their page 8 actually produced under stock v1.2.7. Running
    /// the real profile is what binds this fix to the reported page rather than to a
    /// hand-built approximation of it: the synthetic `gh1763_page` fixture exercises the
    /// same mechanism, but only this asserts the reported page now splits where the
    /// reporter said it should.
    ///
    /// Their stated numbers, all reproduced below: threshold 98.03, valley run
    /// x 53.6..307.6, buggy midpoint x 180.6, target x 301.6. The valley's empty core is
    /// 12 pt wide -- UNDER `min_valley_width` (15) -- which is why the width gate must
    /// keep applying to the run as a whole and not to its core, as the issue warns. ~keep
    #[test]
    fn the_reported_gh1763_profile_now_splits_at_its_empty_core() {
        const PROFILE: &str = include_str!("../../../tests/fixtures/gh1763_horizontal_density_profile.txt");
        const X_MIN: f32 = 37.587;
        const VALLEY_THRESHOLD: f32 = 0.3;
        const MIN_VALLEY_WIDTH: f32 = 15.0;

        let density: Vec<f32> = PROFILE
            .lines()
            .filter(|line| !line.starts_with('#'))
            .map(|line| line.parse().expect("fixture must hold one float per line"))
            .collect();
        assert_eq!(density.len(), 523, "fixture must be the reporter's full profile");

        let peak = density.iter().copied().fold(f32::MIN, f32::max);
        let threshold = VALLEY_THRESHOLD * peak;
        assert_eq!(
            (peak * 100.0).round() / 100.0,
            326.77,
            "profile peak must match the reporter's"
        );
        assert_eq!(
            (threshold * 100.0).round() / 100.0,
            98.03,
            "valley threshold must match the reporter's"
        );

        let mut runs: Vec<(usize, usize)> = Vec::new();
        let mut index = 0usize;
        while index < density.len() {
            if density[index] < threshold {
                let start = index;
                while index < density.len() && density[index] < threshold {
                    index += 1;
                }
                if start > 0 && index < density.len() {
                    runs.push((start, index));
                }
            } else {
                index += 1;
            }
        }
        let (valley_start, valley_end) = runs
            .into_iter()
            .max_by_key(|(start, end)| end - start)
            .expect("the profile must contain an interior valley");
        assert_eq!(
            (X_MIN + valley_start as f32, X_MIN + valley_end as f32),
            (53.587, 307.587),
            "widest interior valley run must be the one the reporter measured"
        );

        let run_width = (valley_end - valley_start) as f32;
        assert!(
            run_width >= MIN_VALLEY_WIDTH,
            "the width gate applies to the whole run, which passes it"
        );

        let buggy = X_MIN + XYCutStrategy::legacy_valley_midpoint(valley_start, valley_end);
        assert_eq!(buggy, 180.587, "pre-fix split must be the midpoint the reporter saw");

        let fixed = X_MIN + XYCutStrategy::deepest_point_wrapper(&density, valley_start, valley_end);
        assert_eq!(
            fixed, 301.587,
            "fixed split must be the empty core the reporter identified"
        );
        assert_eq!(
            density[(fixed - X_MIN) as usize],
            0.0,
            "the fixed split must land on a zero-density bin"
        );
    }

    #[test]
    fn find_valley_selects_the_deepest_point_not_the_midpoint_gh1763() {
        let strategy = XYCutStrategy::new();
        let spans = gh1763_page();

        let profile = strategy
            .horizontal_projection(&spans)
            .expect("non-empty span set must produce a projection profile");

        let (valley_start, valley_end, valley_width) = strategy
            .find_valley(&profile)
            .expect("a wide interior valley must be found");
        assert_eq!(
            (valley_start, valley_end, valley_width),
            (9, 500, 491.0),
            "unexpected valley run bounds"
        );

        let buggy_midpoint = XYCutStrategy::legacy_valley_midpoint(valley_start, valley_end);
        assert_eq!(buggy_midpoint, 254.5, "pre-fix formula must land inside CAP4's span");
        assert_eq!(
            profile.density[254], 10.0,
            "the pre-fix midpoint must land on real (sub-threshold) caption content, not empty space"
        );

        let deepest = XYCutStrategy::deepest_point_wrapper(&profile.density, valley_start, valley_end);
        assert_eq!(
            deepest, DEEPEST_POINT,
            "fixed split must land at the center of the widest zero-density sub-run"
        );
        assert_eq!(
            (profile.density[466], profile.density[467]),
            (0.0, 0.0),
            "the bins either side of the fixed split must be truly empty, not caption content"
        );
        assert_ne!(deepest, buggy_midpoint, "the fix must actually move the split point");
    }

    /// Behavioral counterpart: `find_horizontal_split_indexed` (the real
    /// caller, not a hand-rolled reimplementation) must partition the
    /// GH#1763 fixture so every caption fragment (`CAP1`, `CAP2`,
    /// `FIG.3.A`, `CAP4`, `CAP5`) stays on the left and every body line
    /// (`BODY*`) stays on the right — pre-fix, `CAP5` crossed into the
    /// body side because the buggy midpoint (254.5) sits to the LEFT of
    /// CAP5's own left edge (320). ~keep
    #[test]
    fn find_horizontal_split_indexed_keeps_caption_fragments_together_gh1763() {
        let strategy = XYCutStrategy::new();
        let spans = gh1763_page();
        let indices: Vec<usize> = (0..spans.len()).collect();

        let (left, right) = strategy
            .find_horizontal_split_indexed(&spans, &indices)
            .expect("a valid column split must be found");

        assert_eq!(
            left,
            vec![0, 1, 2, 3, 4],
            "left side must hold exactly the 5 caption fragments"
        );
        assert_eq!(
            right,
            vec![5, 6, 7, 8, 9, 10],
            "right side must hold exactly the 6 body lines"
        );
    }

    /// Integration-level counterpart via the public `partition_region`
    /// entry point. Column purity — no caption fragment sharing a final
    /// group with any body line, and vice versa — is asserted rather
    /// than "same group_id for all 5 caption fragments", because the
    /// sparse, widely-spaced caption fragments legitimately subdivide
    /// further under recursion (each such sub-split is rejected by
    /// `MIN_RESULT_WIDTH_PT`, so in practice they land in one group, but
    /// the invariant that must hold regardless is column purity). ~keep
    #[test]
    fn gh1763_caption_fragments_never_bleed_into_the_body_column() {
        let strategy = XYCutStrategy::new();
        let spans = gh1763_page();

        let groups = strategy.partition_region(&spans, None);

        let group_of = |text: &str| -> usize {
            groups
                .iter()
                .position(|g| g.iter().any(|s| s.text == text))
                .unwrap_or_else(|| panic!("{text} missing from output: {groups:?}"))
        };

        let caption_texts = ["c1", "c2z", "FIG.3.A"];
        let caption_groups: Vec<usize> = caption_texts.iter().map(|t| group_of(t)).collect();
        let cap4_group = groups
            .iter()
            .position(|g| g.iter().any(|s| s.text.starts_with("CAP4")))
            .expect("CAP4 fragment missing");
        let cap5_group = groups
            .iter()
            .position(|g| g.iter().any(|s| s.text.starts_with("CAP5")))
            .expect("CAP5 fragment missing");
        let body_groups: Vec<usize> = (0..6)
            .map(|i| {
                groups
                    .iter()
                    .position(|g| g.iter().any(|s| s.text.starts_with(&format!("BODY{i}"))))
                    .unwrap_or_else(|| panic!("BODY{i} missing from output: {groups:?}"))
            })
            .collect();

        for &cg in caption_groups.iter().chain([&cap4_group, &cap5_group]) {
            assert!(
                !body_groups.contains(&cg),
                "a caption fragment must never share a group with body content: {groups:?}"
            );
        }
        // The specific manifestation of the bug: CAP5 must stay with FIG.3.A,
        // not fall into the body group. ~keep
        assert_eq!(
            cap5_group, caption_groups[2],
            "CAP5 must group with FIG.3.A, not drift to the body column: {groups:?}"
        );
    }

    /// GH#1808 reproducer: a two-column region (left/right lines at y=300/285/270) sits
    /// above an 8-line figure legend that runs the full region width (x 40..300). Each
    /// legend line is written as 3 font-run fragments with small (5pt) inter-run gaps --
    /// the shape a `Tm`+`TJ`-per-run producer emits for a caption. Region width is 260pt
    /// (40..300), so a legend line's own 260pt extent is exactly full width.
    fn gh1808_two_columns_over_torn_legend_spans() -> Vec<TextSpan> {
        let mut spans = Vec::new();
        for &y in &[300.0, 285.0, 270.0] {
            spans.push(make_span_text(40.0, y, 100.0, 10.0, "left column line", 10.0));
            spans.push(make_span_text(200.0, y, 100.0, 10.0, "right column line", 10.0));
        }
        for row in 0..8 {
            let y = 150.0 - row as f32 * 12.0;
            spans.push(make_span_text(40.0, y, 60.0, 7.17, "LEG-A", 7.17)); // 40..100
            spans.push(make_span_text(105.0, y, 95.0, 7.17, "LEG-B", 7.17)); // 105..200, gap 5
            spans.push(make_span_text(205.0, y, 95.0, 7.17, "LEG-C", 7.17)); // 205..300, gap 5
        }
        spans
    }

    /// RED-then-GREEN unit test for GH#1808's peel (`peel_full_width_line_bands`). Before
    /// the fix (no peeling at all) this method did not exist and every legend fragment
    /// reached `find_horizontal_split_indexed` ungrouped, where a column cut near x=150-190
    /// would tear each legend line at its own font-run boundary. Neutering the fix (see the
    /// commit message for the verbatim pre-fix failure) makes this method return `None`;
    /// with the fix, the legend is peeled into its own band, entirely separate from the two
    /// column lines, and every legend line's 3 fragments stay together in that one band. ~keep
    #[test]
    fn peel_full_width_line_bands_keeps_a_torn_legend_whole_gh1808() {
        let strategy = XYCutStrategy::new();
        let spans = gh1808_two_columns_over_torn_legend_spans();
        let indices: Vec<usize> = (0..spans.len()).collect();

        let bands = strategy
            .peel_full_width_line_bands(&spans, &indices)
            .expect("an 8-line full-width legend must be peeled off as its own band(s)");

        assert_eq!(bands.len(), 2, "expected exactly [column band, legend band]: {bands:?}");
        let column_band = &bands[0];
        let legend_band = &bands[1];
        assert_eq!(
            column_band.len(),
            6,
            "the two columns' 6 lines must form the first band"
        );
        assert_eq!(
            legend_band.len(),
            24,
            "all 8 legend lines' 24 fragments must form the second band"
        );

        // No legend line's 3 fragments may be split across bands: each of LEG-A/B/C's 8
        // occurrences must appear only in `legend_band`, never in `column_band`.
        for &index in legend_band {
            assert!(
                spans[index].text.starts_with("LEG-"),
                "a legend fragment landed outside the legend band: {}",
                spans[index].text
            );
        }
        for &index in column_band {
            assert!(
                !spans[index].text.starts_with("LEG-"),
                "a column line leaked into the legend band: {}",
                spans[index].text
            );
        }
    }

    /// Control for the peel: when each legend line is already ONE span (the reporter's own
    /// p2, which passed before the fix), the peel still fires identically -- the fix must
    /// not change behavior for input that was never torn in the first place. ~keep
    #[test]
    fn peel_full_width_line_bands_control_single_span_legend_gh1808() {
        let strategy = XYCutStrategy::new();
        let mut spans = Vec::new();
        for &y in &[300.0, 285.0, 270.0] {
            spans.push(make_span_text(40.0, y, 100.0, 10.0, "left column line", 10.0));
            spans.push(make_span_text(200.0, y, 100.0, 10.0, "right column line", 10.0));
        }
        for row in 0..8 {
            let y = 150.0 - row as f32 * 12.0;
            spans.push(make_span_text(40.0, y, 260.0, 7.17, "LEGEND", 7.17));
        }
        let indices: Vec<usize> = (0..spans.len()).collect();

        let bands = strategy
            .peel_full_width_line_bands(&spans, &indices)
            .expect("the single-span legend must still be peeled off");
        assert_eq!(bands.len(), 2);
        assert_eq!(
            bands[1].len(),
            8,
            "8 single-span legend lines must form the legend band"
        );
    }

    /// RED-then-GREEN unit test for GH#1808's assignment fix (`partition_lines_at`), in
    /// isolation from the peel. A continuous 5-fragment line (font-run gaps of 4pt at
    /// 20pt font, well under `MAX_INTRA_LINE_GAP_EM`) runs from x=40 to x=420, straddling
    /// `split_x = 200.0` with the majority of its inked width (212 of 394pt) on the right.
    /// Before the fix (a per-span `left edge < split_x` test) this line tore: its first two
    /// fragments (left edge < 200) landed in `left`, the remaining three in `right`. ~keep
    #[test]
    fn partition_lines_at_keeps_a_straddling_line_whole_gh1808() {
        let strategy = XYCutStrategy::new();
        let mut spans = vec![
            make_span_text(40.0, 300.0, 50.0, 10.0, "clear-left-1", 10.0),
            make_span_text(40.0, 285.0, 50.0, 10.0, "clear-left-2", 10.0),
            make_span_text(300.0, 300.0, 50.0, 10.0, "clear-right-1", 10.0),
            make_span_text(300.0, 285.0, 50.0, 10.0, "clear-right-2", 10.0),
        ];
        let straddling_start = spans.len();
        for (left, right) in [
            (40.0, 116.0),
            (120.0, 196.0),
            (200.0, 276.0),
            (280.0, 356.0),
            (360.0, 420.0),
        ] {
            spans.push(make_span_text(left, 200.0, right - left, 20.0, "STRADDLE", 20.0));
        }
        let indices: Vec<usize> = (0..spans.len()).collect();

        let (left, right) = strategy.partition_lines_at(&spans, &indices, 200.0);

        assert_eq!(left, vec![0, 1], "only the two clear-left lines belong on the left");
        let mut expected_right: Vec<usize> = vec![2, 3];
        expected_right.extend(straddling_start..straddling_start + 5);
        expected_right.sort_unstable();
        let mut actual_right = right;
        actual_right.sort_unstable();
        assert_eq!(
            actual_right, expected_right,
            "the straddling line's 5 fragments must all move to its majority side (right), none left behind"
        );
    }

    /// Two columns whose baselines differ by 0.6pt (the right column set higher) share
    /// `y` buckets. Without the per-bucket `x` sort the right line sorts first, the gap to
    /// the left line is negative, and both columns' lines merge into one "line" that
    /// `partition_lines_at` then sends to one side. ~keep
    #[test]
    fn partition_lines_at_separates_columns_whose_baselines_differ() {
        let strategy = XYCutStrategy::new();
        let mut spans = Vec::new();
        for row in 0..3 {
            let y = 300.0 - row as f32 * 15.0;
            spans.push(make_span_text(40.0, y, 140.0, 10.0, "left column line", 10.0));
            spans.push(make_span_text(220.0, y + 0.6, 140.0, 10.0, "right column line", 10.0));
        }
        let indices: Vec<usize> = (0..spans.len()).collect();

        let (left, right) = strategy.partition_lines_at(&spans, &indices, 200.0);

        assert_eq!(left, vec![0, 2, 4], "the left column's lines");
        assert_eq!(right, vec![1, 3, 5], "the right column's lines");
    }

    /// A span nested inside a longer one on the same line (a fake-bold overprint, a
    /// superscript set 0.5pt higher) must not open a gap: measured from the nested span's
    /// own right edge, the next span looked 350pt away and the line tore at the split. ~keep
    #[test]
    fn partition_lines_at_keeps_a_line_with_a_nested_span_whole() {
        let strategy = XYCutStrategy::new();
        let spans = vec![
            make_span_text(40.0, 300.0, 360.0, 10.0, "long run", 10.0),
            make_span_text(50.0, 300.5, 10.0, 10.0, "nested", 10.0),
            make_span_text(405.0, 300.0, 60.0, 10.0, "tail", 10.0),
        ];
        let indices: Vec<usize> = (0..spans.len()).collect();

        let (left, right) = strategy.partition_lines_at(&spans, &indices, 402.0);

        assert_eq!(left, indices, "the whole line stays together on one side");
        assert!(right.is_empty(), "no span of the line is torn off: {right:?}");
    }

    /// The reporter's layout for GH#2097 at the level the column cut is decided: 13-line
    /// prose columns at x=69 and x=444 whose baselines differ by 1.0pt. The cut must be
    /// taken and each column's lines must land whole on their own side. ~keep
    #[test]
    fn horizontal_split_cuts_prose_columns_whose_baselines_differ_gh2097() {
        let strategy = XYCutStrategy::new();
        let mut spans = Vec::new();
        for row in 0..13 {
            let y = 600.0 - row as f32 * 12.0;
            spans.push(make_span_text(
                69.0,
                y,
                200.0,
                10.0,
                "left prose column line of words",
                10.0,
            ));
            spans.push(make_span_text(
                444.0,
                y + 1.0,
                200.0,
                10.0,
                "right prose column line of words",
                10.0,
            ));
        }
        let indices: Vec<usize> = (0..spans.len()).collect();

        let (left, right) = strategy
            .find_horizontal_split_indexed(&spans, &indices)
            .expect("two prose columns must be cut at the gutter");

        assert_eq!(left, (0..spans.len()).step_by(2).collect::<Vec<_>>(), "left column");
        assert_eq!(right, (1..spans.len()).step_by(2).collect::<Vec<_>>(), "right column");
    }

    /// Short label / value rows whose value sits a hair above the label are a table, not
    /// two columns: cutting between them reads every label before every value. Before the
    /// `x`-ordered line grouping the value sorted first, the pair measured as one line and
    /// the cut was refused; the grouping alone would now accept it. ~keep
    #[test]
    fn horizontal_split_keeps_label_value_rows_together() {
        let strategy = XYCutStrategy::new();
        for value_offset in [0.24f32, 0.6] {
            let mut spans = Vec::new();
            for row in 0..8 {
                let y = 600.0 - row as f32 * 14.0;
                let label_width = if row % 2 == 0 { 70.0 } else { 46.0 };
                spans.push(make_span_text(40.0, y, label_width, 10.0, "Label", 10.0));
                spans.push(make_span_text(220.0, y + value_offset, 70.0, 10.0, "Value", 10.0));
            }
            let indices: Vec<usize> = (0..spans.len()).collect();

            assert!(
                strategy.find_horizontal_split_indexed(&spans, &indices).is_none(),
                "label/value rows with the value {value_offset}pt higher must not be cut into columns"
            );
        }
    }
}
