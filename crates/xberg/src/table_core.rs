//! Core table reconstruction types and algorithms.
//!
//! This module provides the `HocrWord` type and table reconstruction functions
//! that are shared between the OCR and PDF modules. The algorithms detect
//! column/row structure from word bounding boxes and reconstruct tabular layouts.
//!
//! Originally adapted from the `hocr` module of `html-to-markdown-rs` (removed in v3).

use std::collections::HashSet;

/// Represents a word extracted from hOCR (or any source) with position and confidence information.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub struct HocrWord {
    /// Recognized word text.
    pub text: String,
    /// Left edge of the word bounding box in pixels.
    pub left: u32,
    /// Top edge of the word bounding box in pixels.
    pub top: u32,
    /// Bounding box width in pixels.
    pub width: u32,
    /// Bounding box height in pixels.
    pub height: u32,
    /// OCR confidence score (0.0–100.0).
    pub confidence: f64,
}

impl HocrWord {
    /// Get the right edge position.
    #[cfg(test)]
    #[inline]
    pub(crate) fn right(&self) -> u32 {
        self.left + self.width
    }

    /// Get the bottom edge position.
    #[cfg(test)]
    #[inline]
    pub(crate) fn bottom(&self) -> u32 {
        self.top + self.height
    }

    /// Get the vertical center position.
    #[inline]
    pub(crate) fn y_center(&self) -> f64 {
        self.top as f64 + (self.height as f64 / 2.0)
    }

    /// Get the horizontal center position.
    #[cfg(test)]
    #[inline]
    pub(crate) fn x_center(&self) -> f64 {
        self.left as f64 + (self.width as f64 / 2.0)
    }
}

const NUMERIC_CONTINUATION_GAP_HEIGHT_RATIO: f64 = 0.15;
const NUMERIC_CONTINUATION_ROUNDING_TOLERANCE: u32 = 1;

/// The symbols a table cell prints on their own: a nil dash, a bracket, a currency sign, a percent
/// sign, a footnote asterisk. Alone, each is cell content rather than a stray mark. ~keep
const CELL_SYMBOLS: &[char] = &[
    '-', '\u{2010}', '\u{2011}', '\u{2012}', '\u{2013}', '\u{2014}', '\u{2212}', '(', ')', '[', ']', '{', '}', '$',
    '\u{a2}', '\u{a3}', '\u{a5}', '\u{20ac}', '%', '*',
];

/// Whether `text` is one of [`CELL_SYMBOLS`] standing alone.
pub(crate) fn is_lone_cell_symbol(text: &str) -> bool {
    let mut chars = text.chars();
    matches!((chars.next(), chars.next()), (Some(symbol), None) if CELL_SYMBOLS.contains(&symbol))
}

/// Whether `text` reads as a value: it holds a digit and no letter.
#[cfg(feature = "ocr")]
pub(crate) fn is_value(text: &[char]) -> bool {
    chars_read_as_value(text.iter().copied())
}

/// The same test as `is_value` over a string's characters, without materialising them.
pub(crate) fn text_is_value(text: &str) -> bool {
    chars_read_as_value(text.chars())
}

/// The one implementation behind `is_value` and [`text_is_value`], which differ only in how the
/// caller holds the text — the underscore-mark cut works in `&[char]`, column detection in `&str`.
/// ~keep
fn chars_read_as_value(chars: impl Iterator<Item = char>) -> bool {
    let mut has_digit = false;
    for character in chars {
        if character.is_alphabetic() {
            return false;
        }
        has_digit = has_digit || character.is_ascii_digit();
    }
    has_digit
}

/// Whether `text` prints a value in a table cell: a number by [`text_is_value`], or one of the
/// symbols a cell prints alone ([`is_lone_cell_symbol`]) — the nil dash of an empty amount, or the
/// bracket or currency sign OCR split off the front of one.
///
/// This is the same notion of cell content the shading-mark filter tests for (`is_mark_text`, which
/// rejects a word as a mark when it is either of these), reused rather than restated. ~keep
pub(crate) fn is_cell_value_text(text: &str) -> bool {
    let text = text.trim();
    text_is_value(text) || is_lone_cell_symbol(text)
}

/// One detected column: the x-position reported to callers, and the left edges membership is
/// decided against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ColumnTrack {
    /// Median left edge of the column's tokens, and the leftmost of them once tracks are folded. A
    /// header-only track folded in does not move it: the position is where the data sits, not
    /// where the header text lands, and downstream geometry (the OCR blank-quantity retry's crop
    /// bounds) reads it as the data column's left edge.
    ///
    /// This, and only this, is what [`reconstruct_table_with_columns`] reports: every downstream
    /// consumer of a column position — the OCR blank-quantity retry's crop bounds, the native-PDF
    /// borderless-table whitespace check, `merge_disjoint_numeric_columns` — reads it as a left
    /// edge in page pixels. ~keep
    left: u32,
    /// Median right edge of the column's tokens (the rightmost once folded).
    right: u32,
    /// Whether the column's data tokens are values ([`is_cell_value_text`]) by majority, which is
    /// what makes it eligible to fold into an adjacent value column. Header-band tokens are not
    /// read: a column's header label is text by nature. See [`column_track`].
    right_aligned: bool,
    /// Whether the column holds a token below the header band. A track without one is a header
    /// label on its own: it folds into the value column whose right edge it shares, whatever its
    /// label reads, unless that column has a label of its own in the same row. See
    /// [`fold_right_aligned_tracks`]. ~keep
    has_data: bool,
    /// The header-band rows in which the column holds a token, of every track it was folded from.
    /// Two labels in one row are two columns, so a header-only track never folds into a column
    /// that holds a label in the same row. ~keep
    label_rows: Vec<usize>,
    /// The median left edge of each track this column was folded from, one entry when it was not
    /// folded at all.
    ///
    /// Membership is still decided on a left edge — the nearest of these — rather than on the
    /// merged column's own left edge. Folding is about how many columns the grid has, not about
    /// which column a word belongs to, and the two must be kept apart: a folded column's `left` is
    /// its leftmost track's, so measuring membership against it alone would move every short value
    /// to whichever neighbour's left edge now sits closer, and take a header word with it. Measured
    /// on the GH#1832 fixture, deciding membership on the merged edge split the header `Year 2`
    /// across two cells; deciding it on the merged column's right edge instead did that *and*
    /// shifted every data value one cell right, dropping ground-truth cell agreement from 138 to
    /// 24. Keeping each track's own left edge leaves membership exactly as it was before the
    /// fold. ~keep
    lefts: Vec<u32>,
    /// The rows in which the column holds a number ([`text_is_value`]), of every track it was
    /// folded from. Two numbers in one row are two cells, so two tracks that both hold a number
    /// in the same row never fold.
    number_rows: Vec<usize>,
}

impl ColumnTrack {
    /// A column anchored on `left`, for the direct-assignment test seam that has only a position.
    #[cfg(test)]
    fn left_aligned(left: u32) -> Self {
        Self {
            left,
            right: left,
            right_aligned: false,
            has_data: true,
            label_rows: Vec::new(),
            lefts: vec![left],
            number_rows: Vec::new(),
        }
    }

    /// Distance from `word` to this column: to the nearest of the left edges of the tracks it was
    /// folded from. See [`ColumnTrack::lefts`].
    fn distance_to(&self, word: &HocrWord) -> u32 {
        self.lefts
            .iter()
            .map(|left| left.abs_diff(word.left))
            .min()
            .unwrap_or_else(|| self.left.abs_diff(word.left))
    }
}

/// Detect columns from word x-coordinates, each carrying the left edges membership is decided
/// against and the median left edge callers report as its position.
///
/// Groups words by left edge within `column_threshold`, splits a group that holds two columns
/// (see [`split_row_sharing_groups`]), then folds adjacent value columns whose right edges
/// coincide within the same threshold (see [`fold_right_aligned_tracks`]). `row_positions` are the
/// rows the words were grouped into; the header band at the top is read as
/// [`header_band_row_count`] defines it.
pub(crate) fn detect_columns(words: &[HocrWord], row_positions: &[u32], column_threshold: u32) -> Vec<ColumnTrack> {
    if words.is_empty() {
        return Vec::new();
    }

    let header_rows = header_band_row_count(words, row_positions);
    let groups = cluster_by_edge(words, column_threshold, |word| word.left);
    let groups = split_row_sharing_groups(groups, row_positions, column_threshold);

    let mut columns: Vec<ColumnTrack> = groups
        .iter()
        .map(|group| column_track(group.as_slice(), row_positions, header_rows))
        .collect();
    columns.sort_by_key(|column| column.left);
    fold_right_aligned_tracks(&mut columns, column_threshold);
    columns
}

/// Group `words` in order, each joining the first group whose first word's `edge` lies within
/// `column_threshold` of its own.
fn cluster_by_edge<'a>(
    words: impl IntoIterator<Item = &'a HocrWord>,
    column_threshold: u32,
    edge: impl Fn(&HocrWord) -> u32,
) -> Vec<Vec<&'a HocrWord>> {
    let mut groups: Vec<Vec<&HocrWord>> = Vec::new();
    for word in words {
        match groups.iter_mut().find(|group| {
            group
                .first()
                .is_some_and(|first| edge(word).abs_diff(edge(first)) <= column_threshold)
        }) {
            Some(group) => group.push(word),
            None => groups.push(vec![word]),
        }
    }
    groups
}

fn right_edge(word: &HocrWord) -> u32 {
    word.left.saturating_add(word.width)
}

/// Whether two of `group`'s tokens sit in the same row.
fn shares_a_row(group: &[&HocrWord], row_positions: &[u32]) -> bool {
    let mut seen = vec![false; row_positions.len()];
    group
        .iter()
        .filter_map(|word| find_row_index(row_positions, word))
        .any(|row| {
            let already = seen[row];
            seen[row] = true;
            already
        })
}

/// Split a left-edge group that holds two columns (xberg-io/xberg#1909).
///
/// Tokens are already merged into cells, so two tokens of one row in one group are two cells, and
/// two cells of one row cannot be one column. That happens when a short amount in one column starts
/// within `column_threshold` of a long amount in the next: `7` ends where its column ends, and
/// `40,218,965` starts 47px to its right at 300 dpi. Such a group is split by
/// [`split_by_right_edge`] when that gives each of the two columns its own tokens; otherwise it is
/// kept whole. Membership stays on left edges, as [`ColumnTrack::lefts`] requires: the split only
/// gives each of the two columns its own left edge. ~keep
fn split_row_sharing_groups<'a>(
    groups: Vec<Vec<&'a HocrWord>>,
    row_positions: &[u32],
    column_threshold: u32,
) -> Vec<Vec<&'a HocrWord>> {
    let mut split = Vec::with_capacity(groups.len());
    for group in groups {
        match shares_a_row(&group, row_positions)
            .then(|| split_by_right_edge(&group, row_positions, column_threshold))
            .flatten()
        {
            Some(parts) => split.extend(parts),
            None => split.push(group),
        }
    }
    split
}

/// Count of rows in the header band at the top of the region: row 0, plus every following row up
/// to the first row that holds a number ([`text_is_value`]).
///
/// A scanned header band spans two or three rows: a title row, the column labels, a units line.
/// Reading only row 0 as the header made the labels of every further header row count as data
/// tokens, so a value column under a two-row header never read as values and never folded
/// (xberg-io/xberg#1952). A row with a number in any column is table content, and the band ends
/// there. A region with no number has no value column to fold, and its header is row 0.
///
/// Only the fold test reads the band. The right-edge split ([`split_by_right_edge`]) keeps row 0
/// as its header: when the first number sits far down a text table, the band holds most of the
/// table's rows, and a split that did not read them would leave two close text columns in one
/// cell. ~keep
fn header_band_row_count(words: &[HocrWord], row_positions: &[u32]) -> usize {
    number_rows(words, row_positions)
        .first()
        .map_or(1, |&first_value_row| first_value_row.max(1))
}

/// Split `group` into its header-band tokens (the first `header_rows` rows) and its data tokens.
fn split_header_band<'a>(
    group: &[&'a HocrWord],
    row_positions: &[u32],
    header_rows: usize,
) -> (Vec<&'a HocrWord>, Vec<&'a HocrWord>) {
    group
        .iter()
        .copied()
        .partition(|word| find_row_index(row_positions, word).is_some_and(|row| row < header_rows))
}

/// Re-group `group` by right edge, or `None` unless that gives exactly two independently
/// supported parts with at most one token from each row.
///
/// Only data tokens, below row 0, are re-grouped by right edge. A header label is written from the
/// left edge of its column whatever the alignment of the values under it, so its right edge says
/// nothing about its column: it joins the part whose median left edge is nearest its own. ~keep
fn split_by_right_edge<'a>(
    group: &[&'a HocrWord],
    row_positions: &[u32],
    column_threshold: u32,
) -> Option<Vec<Vec<&'a HocrWord>>> {
    let (header, data) = split_header_band(group, row_positions, 1);
    let mut parts = cluster_by_edge(data, column_threshold, right_edge);
    if parts.len() != 2 {
        return None;
    }
    let lefts: Vec<u32> = parts
        .iter()
        .map(|part| median_of(part.iter().map(|word| word.left).collect()))
        .collect();
    for word in header {
        let nearest = (0..parts.len()).min_by_key(|&index| lefts[index].abs_diff(word.left))?;
        parts[nearest].push(word);
    }
    if parts.iter().any(|part| part.len() < 2) {
        return None;
    }
    (!parts.iter().any(|part| shares_a_row(part, row_positions))).then_some(parts)
}

/// Whether `group` reads as values: its tokens below the header band when it has any, else its
/// header tokens. The band is not read when data rows are present: a column's header label is
/// text by nature. A header-only group reads as values only when its label is one, such as a year.
///
/// The tokens read are values when the values outnumber the other tokens. A column is the kind of
/// cell most of its cells are: on a scan, a value column carries the odd token OCR read from a
/// rule mark or misread as a letter, and one such token must not make the whole column text
/// (xberg-io/xberg#1952). A text column has text in most of its cells and never reads as values,
/// whatever numbers it holds. ~keep
fn reads_as_values(group: &[&HocrWord], row_positions: &[u32], header_rows: usize) -> bool {
    let (header, data) = split_header_band(group, row_positions, header_rows);
    let tokens = if data.is_empty() { header } else { data };
    let values = tokens.iter().filter(|word| is_cell_value_text(&word.text)).count();
    values > tokens.len() - values
}

/// The column one group of tokens forms.
///
/// The fold test reads the data tokens only. A header label sits on the left edge of the values it
/// labels, so it joins one of their left-edge groups, and reading its text as well would stop that
/// group from folding with the rest of its column. A group with no data token is a header-only
/// track (`has_data` false): a label wider than the values under it starts more than the threshold
/// left of them and forms one (xberg-io/xberg#1909). ~keep
fn column_track(group: &[&HocrWord], row_positions: &[u32], header_rows: usize) -> ColumnTrack {
    let left = median_of(group.iter().map(|word| word.left).collect());
    let (header, data) = split_header_band(group, row_positions, header_rows);
    ColumnTrack {
        left,
        right: median_of(group.iter().map(|word| right_edge(word)).collect()),
        right_aligned: reads_as_values(group, row_positions, header_rows),
        has_data: !data.is_empty(),
        label_rows: header
            .iter()
            .filter_map(|word| find_row_index(row_positions, word))
            .collect(),
        lefts: vec![left],
        number_rows: number_rows(group.iter().copied(), row_positions),
    }
}

/// The rows in which `words` hold a number ([`text_is_value`]), sorted and without repeats.
fn number_rows<'a>(words: impl IntoIterator<Item = &'a HocrWord>, row_positions: &[u32]) -> Vec<usize> {
    let mut rows: Vec<usize> = words
        .into_iter()
        .filter(|word| text_is_value(word.text.trim()))
        .filter_map(|word| find_row_index(row_positions, word))
        .collect();
    rows.sort_unstable();
    rows.dedup();
    rows
}

/// Fold adjacent columns that are one right-aligned value column split by digit-count drift
/// (xberg-io/xberg#1886).
///
/// An amount column is right-aligned, so its tokens' left edges spread with digit count: `5`, `73`
/// and `1,234,567` share a right edge and differ on the left by far more than
/// `column_threshold` at 300 dpi. Left-edge grouping alone mints two or three columns from one,
/// and the residue after `merge_disjoint_numeric_columns` re-merges the disjoint cases is lost
/// content: a dropped nil-dash column, a sparse row's interior cells, a value in the label cell.
///
/// Raising `column_threshold` is not the alternative — it merges genuinely narrow neighbouring
/// columns (the GH#1649 `DEPOSIT` case). Both sides must read as values in their data rows (see
/// [`reads_as_values`]), so a text column is never folded. A header-only track folds into a value
/// column whose right edge it shares whatever its label reads: a right-aligned label ends where
/// the values it labels end. Without that, a label wider than its values, such as a period written
/// as letters and digits, keeps a column of its own, and the column's long amounts, whose left
/// edges sit nearer the label's, follow it out of their column (xberg-io/xberg#1909). It does not
/// fold into a value column that has a label of its own in the same row: two labels side by side
/// head two columns, such as an empty note column beside a quantity column, and folding them
/// would put both labels in one cell (see [`ColumnTrack::label_rows`]). A header word clustered
/// with data values does not stop them folding either (see [`column_track`]). Two tracks that
/// each hold a number in one row are two columns, such as a code column next to a quantity
/// column, and never fold (see [`ColumnTrack::number_rows`]). A rule mark or nil dash beside a
/// value does not count: it holds no digit. ~keep
fn fold_right_aligned_tracks(columns: &mut Vec<ColumnTrack>, column_threshold: u32) {
    let mut index = 0;
    while index + 1 < columns.len() {
        let (current, next) = (&columns[index], &columns[index + 1]);
        let values_meet = current.right_aligned && next.right_aligned;
        let label_meets_values = ((!current.has_data && next.right_aligned)
            || (current.right_aligned && !next.has_data))
            && !share_a_label_row(current, next);
        let foldable = (values_meet || label_meets_values)
            && current.right.abs_diff(next.right) <= column_threshold
            && !share_a_number_row(current, next);
        if foldable {
            let next = columns.remove(index + 1);
            let current = &mut columns[index];
            // The folded column is the kind of the track that holds its data, and sits where that
            // data sits (see [`ColumnTrack::left`]). ~keep
            (current.left, current.right_aligned) = match (current.has_data, next.has_data) {
                (true, true) => (current.left.min(next.left), current.right_aligned),
                (true, false) => (current.left, current.right_aligned),
                (false, true) => (next.left, next.right_aligned),
                (false, false) => (current.left.min(next.left), current.right_aligned || next.right_aligned),
            };
            current.right = current.right.max(next.right);
            current.has_data |= next.has_data;
            current.label_rows.extend(next.label_rows);
            current.lefts.extend(next.lefts);
            current.number_rows.extend(next.number_rows);
        } else {
            index += 1;
        }
    }
}

/// Whether `left` and `right` both hold a number in some row.
fn share_a_number_row(left: &ColumnTrack, right: &ColumnTrack) -> bool {
    left.number_rows.iter().any(|row| right.number_rows.contains(row))
}

fn share_a_label_row(left: &ColumnTrack, right: &ColumnTrack) -> bool {
    left.label_rows.iter().any(|row| right.label_rows.contains(row))
}

/// Compute the median word height. Returns 0 for an empty slice.
///
/// Extracted so `detect_rows`'s row-grouping threshold,
/// `group_words_into_cell_tokens`'s cell-merge threshold, and (for OCR
/// callers) `merge_disjoint_numeric_columns`'s column-merge threshold are
/// always computed from the same statistic, rather than duplicating the
/// sort-and-index in more than one place.
pub(crate) fn median_word_height(words: &[HocrWord]) -> u32 {
    median_of(words.iter().map(|w| w.height).collect())
}

/// The median of `values` (the upper one of an even count), or 0 when there are none.
pub(crate) fn median_of(mut values: Vec<u32>) -> u32 {
    values.sort_unstable();
    values.get(values.len() / 2).copied().unwrap_or(0)
}

fn median_row_position(group: &[f64]) -> Option<u32> {
    if group.is_empty() {
        return None;
    }
    let mut sorted = group.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    Some(sorted[sorted.len() / 2] as u32)
}

/// Detect row positions from word y-coordinates.
///
/// Groups representative-height words by their vertical center position and returns the median
/// y-position for each detected row. A taller word seeds a row only when its box does not overlap
/// an existing row band, so a standalone tall heading remains a row while an outlier in a normal
/// row cannot split it. The `row_threshold_ratio` is multiplied by the median word height to
/// determine the grouping threshold.
pub(crate) fn detect_rows(words: &[HocrWord], row_threshold_ratio: f64) -> Vec<u32> {
    if words.is_empty() {
        return Vec::new();
    }

    let median_height = median_word_height(words);
    let row_threshold = (median_height as f64 * row_threshold_ratio) as u32;
    let maximum_seed_height = median_height.saturating_add(median_height / 2);
    let (seed_words, tall_words): (Vec<&HocrWord>, Vec<&HocrWord>) =
        words.iter().partition(|word| word.height <= maximum_seed_height);
    let (seed_words, tall_words) = if seed_words.is_empty() {
        (words.iter().collect(), Vec::new())
    } else {
        (seed_words, tall_words)
    };

    let mut position_groups: Vec<Vec<f64>> = Vec::new();

    for word in seed_words {
        let y_center = word.y_center();

        let mut found_group = false;
        for group in &mut position_groups {
            if let Some(&first_pos) = group.first()
                && (y_center - first_pos).abs() <= row_threshold as f64
            {
                group.push(y_center);
                found_group = true;
                break;
            }
        }

        if !found_group {
            position_groups.push(vec![y_center]);
        }
    }

    let half_height = median_height / 2;
    for word in tall_words {
        let bottom = word.top.saturating_add(word.height);
        let overlaps_seeded_row = position_groups
            .iter()
            .filter_map(|group| median_row_position(group))
            .any(|row_y| {
                let band_top = row_y.saturating_sub(half_height);
                let band_bottom = row_y.saturating_add(half_height);
                bottom.min(band_bottom).saturating_sub(word.top.max(band_top)) > 0
            });
        if !overlaps_seeded_row {
            position_groups.push(vec![word.y_center()]);
        }
    }

    let mut rows: Vec<u32> = position_groups
        .iter()
        .filter_map(|group| median_row_position(group))
        .collect();

    rows.sort_unstable();
    rows
}

/// Find which row a word belongs to based on its y-center.
pub(crate) fn find_row_index(row_positions: &[u32], word: &HocrWord) -> Option<usize> {
    let y_center = word.y_center() as u32;

    row_positions
        .iter()
        .enumerate()
        .min_by_key(|&(_, row_y)| row_y.abs_diff(y_center))
        .map(|(idx, _)| idx)
}

/// Find the row whose median-height band overlaps `word` most, breaking ties by centre distance.
fn find_row_index_by_overlap(row_positions: &[u32], word: &HocrWord, median_height: u32) -> Option<usize> {
    let half_height = median_height / 2;
    let word_bottom = word.top.saturating_add(word.height);
    let word_center = word.y_center() as u32;

    row_positions
        .iter()
        .enumerate()
        .min_by_key(|&(_, row_y)| {
            let band_top = row_y.saturating_sub(half_height);
            let band_bottom = row_y.saturating_add(half_height);
            let overlap = word_bottom.min(band_bottom).saturating_sub(word.top.max(band_top));
            (std::cmp::Reverse(overlap), row_y.abs_diff(word_center))
        })
        .map(|(index, _)| index)
}

/// Find which column a word belongs to, by the nearest left edge of the tracks it was folded from
/// ([`ColumnTrack::distance_to`]).
fn find_column_index(columns: &[ColumnTrack], word: &HocrWord) -> Option<usize> {
    columns
        .iter()
        .enumerate()
        .min_by_key(|&(_, column)| column.distance_to(word))
        .map(|(idx, _)| idx)
}

/// Determine which columns of a table grid contain at least one non-empty
/// cell. Shared by `remove_empty_rows_and_columns` and
/// `reconstruct_table_with_columns`, which both need to drop the same set of
/// all-empty columns — the latter also has to filter a parallel
/// column-position vector by the identical mask so positions keep lining up
/// 1:1 with the grid's surviving columns.
fn non_empty_column_mask(table: &[Vec<String>]) -> Vec<bool> {
    let num_cols = table.first().map_or(0, Vec::len);
    let mut mask = vec![false; num_cols];

    for row in table {
        for (col_idx, cell) in row.iter().enumerate() {
            if !cell.trim().is_empty() {
                mask[col_idx] = true;
            }
        }
    }

    mask
}

/// Remove empty rows and columns from a table grid.
fn remove_empty_rows_and_columns(table: Vec<Vec<String>>) -> Vec<Vec<String>> {
    if table.is_empty() {
        return table;
    }

    let non_empty_cols = non_empty_column_mask(&table);

    table
        .into_iter()
        .filter(|row| row.iter().any(|cell| !cell.trim().is_empty()))
        .map(|row| {
            row.into_iter()
                .enumerate()
                .filter(|(idx, _)| non_empty_cols[*idx])
                .map(|(_, cell)| cell)
                .collect()
        })
        .collect()
}

/// Fraction of the median word height used as the maximum horizontal gap (in
/// pixels) for merging two horizontally-adjacent words within the same
/// detected row into a single cell token before column detection
/// (xberg-io/xberg#688).
///
/// Without this pre-merge, a table cell containing more than one word (e.g.
/// "ABX Air n") puts that cell's second and third words at x-positions that
/// match no genuine column start, so [`detect_columns`] mints a spurious
/// near-empty column per extra word — exactly what the downstream structural
/// validator then rejects, discarding a genuine table entirely.
///
/// 0.6 was chosen because it recovers the exact ground-truth column count
/// (10) on a 215-word-box fixture captured from a real scanned newspaper
/// stock table (ground truth: 16 rows x 10 columns; two side-by-side
/// sub-tables give true column starts at x = {46, 89, 135, 230, 272} and
/// {340, 386, 428, 526, 567}). Measured sensitivity of this factor on that
/// fixture: 0.4 -> 12 columns, 0.6 -> 10 columns (correct), 0.8 -> 9 columns,
/// 1.0 -> 6 columns.
pub(crate) const CELL_MERGE_GAP_HEIGHT_RATIO: f64 = 0.6;

/// Multiplier applied to the normal cell-merge gap when one of the two words being considered
/// is pure punctuation (xberg-io/xberg#1649).
///
/// A lone dash/hyphen glyph rendered as its own OCR word (e.g. the en dash in a date range like
/// "Jan 1 - Jan 31, 2026") is drawn with extra kerning on both sides relative to normal
/// inter-word spacing, so its neighboring gap can land just over `CELL_MERGE_GAP_HEIGHT_RATIO *
/// median height` (measured: a 35px gap against a 34.8px threshold on the reported fixture).
/// Missing that merge leaves the punctuation word to start its own spurious column, which then
/// steals nearby words from the real column during final cell assignment (`find_column_index`
/// picks nearest by raw x-position, not token membership) -- corrupting cells that never
/// contained a multi-word ambiguity in the first place. 2x comfortably covers the measured
/// overshoot without approaching the gaps between genuinely different words (#688's tuned
/// fixture has no punctuation-only tokens, so this multiplier does not affect it). ~keep
const PUNCTUATION_GLUE_GAP_MULTIPLIER: f64 = 2.0;

/// A token/word whose trimmed text is non-empty and entirely ASCII punctuation
/// (a lone "-", ":", "&", etc.), used to widen the cell-merge gap around it (#1649).
fn is_pure_punctuation(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty() && trimmed.chars().all(|ch| ch.is_ascii_punctuation())
}

/// Group horizontally-adjacent words within the same detected row into cell clusters, returning
/// each cluster's merged summary bbox/text (used for column detection, see [`detect_columns`])
/// paired with the original words the cluster contains (xberg-io/xberg#688, xberg-io/xberg#1649).
///
/// Two words in the same row are merged when the horizontal gap between them
/// (the left word's right edge to the right word's left edge) is at most
/// `CELL_MERGE_GAP_HEIGHT_RATIO * median word height`. Rows are the same
/// `row_positions` the caller already computed via `detect_rows` — this does
/// not invent a second row model.
///
/// Column *membership* is decided once per cluster from the cluster's own left edge (its
/// leftmost word), not independently per word: a wide multi-word cell (e.g. a long left-aligned
/// description) has no per-column width bound in [`find_column_index`], only a single
/// representative x-position per column, so a trailing word deep inside such a cell can sit
/// closer by raw x-distance to an unrelated column's anchor than to its own column's anchor.
/// Deciding once per cluster and placing every member word there keeps that trailing word from
/// drifting into a neighboring column at cell-assignment time.
fn group_words_into_cell_tokens<'a>(
    words: &'a [HocrWord],
    row_positions: &[u32],
) -> Vec<(HocrWord, Vec<&'a HocrWord>)> {
    if words.len() <= 1 || row_positions.is_empty() {
        return words.iter().map(|word| (word.clone(), vec![word])).collect();
    }

    let median_height = median_word_height(words);
    let merge_gap = median_height as f64 * CELL_MERGE_GAP_HEIGHT_RATIO;

    let mut rows: Vec<Vec<&HocrWord>> = vec![Vec::new(); row_positions.len()];
    for word in words {
        if let Some(row_index) = find_row_index_by_overlap(row_positions, word, median_height) {
            rows[row_index].push(word);
        }
    }

    for row_words in &mut rows {
        row_words.sort_by_key(|w| w.left);
    }
    let column_edges = repeated_column_edge_words(&rows, median_height);

    let mut groups: Vec<(HocrWord, Vec<&HocrWord>)> = Vec::with_capacity(words.len());
    for row_words in rows {
        groups.extend(merge_row_into_cell_tokens(&row_words, merge_gap, &column_edges));
    }

    groups
}

/// Minimum number of rows whose non-leading words share a left edge before that edge counts as
/// a column boundary the cell merge must not cross (xberg-io/xberg#2140).
const COLUMN_EDGE_MIN_ROWS: usize = 8;

/// Words that start a column the cell merge must not swallow: the non-leading word's left edge
/// (within a fifth of the median height) repeats across at least [`COLUMN_EDGE_MIN_ROWS`] rows,
/// and both the word and its left neighbour vary from row to row in at least half of those rows.
/// Without this, a column sitting closer than the merge gap to its neighbour (`ROW0004` then
/// `21`, 10-16px apart in 32 rows) vanishes into one token and its cells come out empty.
/// Requiring variation on both sides keeps repeated phrases ("Item 1", "12 Jan") merged. ~keep
fn repeated_column_edge_words(rows: &[Vec<&HocrWord>], median_height: u32) -> HashSet<*const HocrWord> {
    let tolerance = median_height / 5;
    let mut candidates: Vec<(u32, usize, &HocrWord, &HocrWord)> = Vec::new();
    for (row_index, row_words) in rows.iter().enumerate() {
        for pair in row_words.windows(2) {
            let (previous, word) = (pair[0], pair[1]);
            if !is_pure_punctuation(&word.text) && !is_pure_punctuation(&previous.text) {
                candidates.push((word.left, row_index, previous, word));
            }
        }
    }
    candidates.sort_by_key(|candidate| candidate.0);

    let mut blocked = HashSet::new();
    let mut start = 0;
    while start < candidates.len() {
        let mut end = start + 1;
        while end < candidates.len() && candidates[end].0 - candidates[end - 1].0 <= tolerance {
            end += 1;
        }
        let cluster = &candidates[start..end];
        let row_count = cluster
            .iter()
            .map(|candidate| candidate.1)
            .collect::<HashSet<_>>()
            .len();
        if row_count >= COLUMN_EDGE_MIN_ROWS {
            let distinct = |texts: Vec<&str>| texts.into_iter().collect::<HashSet<_>>().len();
            let varied_words = distinct(cluster.iter().map(|c| c.3.text.as_str()).collect());
            let varied_neighbours = distinct(cluster.iter().map(|c| c.2.text.as_str()).collect());
            if varied_words * 2 >= cluster.len() && varied_neighbours * 2 >= cluster.len() {
                blocked.extend(cluster.iter().map(|c| std::ptr::from_ref(c.3)));
            }
        }
        start = end;
    }
    blocked
}

/// Decide the maximum horizontal gap allowed for merging `word` into the token immediately
/// preceding it, and whether `word` itself is glued in as a punctuation *connector* (so a
/// following word may in turn glue to it at the widened gap).
///
/// The widened [`PUNCTUATION_GLUE_GAP_MULTIPLIER`] gap applies only when punctuation genuinely
/// bridges two real words, never to an isolated punctuation cell sitting between two column
/// gaps (xberg-io/xberg#1649 review follow-up):
/// - `word` is pure punctuation: bridging requires `next_word` (the word after it, same row) to
///   itself be within the widened gap of `word` -- i.e. the punctuation has a real neighbour on
///   both sides. A lone `-` with nothing close on the far side keeps the normal gap.
/// - `word` is an ordinary word merging into a token that ends in punctuation: only widened when
///   that trailing punctuation was itself glued in as a connector (`previous_was_connector`),
///   never for an isolated leading punctuation cell. ~keep
fn allowed_merge_gap(
    word: &HocrWord,
    next_word: Option<&HocrWord>,
    base_gap: f64,
    previous_was_connector: bool,
) -> (f64, bool) {
    if is_pure_punctuation(&word.text) {
        let bridges_forward = next_word.is_some_and(|next| {
            let gap_to_next = next.left as f64 - (word.left + word.width) as f64;
            gap_to_next <= base_gap * PUNCTUATION_GLUE_GAP_MULTIPLIER
        });
        return if bridges_forward {
            (base_gap * PUNCTUATION_GLUE_GAP_MULTIPLIER, true)
        } else {
            (base_gap, false)
        };
    }

    if previous_was_connector {
        (base_gap * PUNCTUATION_GLUE_GAP_MULTIPLIER, false)
    } else {
        (base_gap, false)
    }
}

/// Merge one row's words (already sorted left-to-right) into cell-token clusters, applying
/// [`allowed_merge_gap`]'s punctuation-connector rule.
fn merge_row_into_cell_tokens<'a>(
    row_words: &[&'a HocrWord],
    merge_gap: f64,
    column_edges: &HashSet<*const HocrWord>,
) -> Vec<(HocrWord, Vec<&'a HocrWord>)> {
    let mut groups: Vec<(HocrWord, Vec<&HocrWord>)> = Vec::new();
    let mut current: Option<(HocrWord, Vec<&HocrWord>)> = None;
    let mut previous_was_connector = false;

    for (index, &word) in row_words.iter().enumerate() {
        let next_word = row_words.get(index + 1).copied();
        current = Some(match current.take() {
            None => (word.clone(), vec![word]),
            Some((mut token, mut members)) => {
                let gap = word.left as f64 - (token.left + token.width) as f64;
                let (allowed_gap, is_connector) = allowed_merge_gap(word, next_word, merge_gap, previous_was_connector);
                if gap <= allowed_gap && !column_edges.contains(&std::ptr::from_ref(word)) {
                    let new_right = (word.left + word.width).max(token.left + token.width);
                    let new_bottom = (word.top + word.height).max(token.top + token.height);
                    token.top = token.top.min(word.top);
                    token.width = new_right.saturating_sub(token.left);
                    token.height = new_bottom.saturating_sub(token.top);
                    token.text.push(' ');
                    token.text.push_str(&word.text);
                    members.push(word);
                    previous_was_connector = is_connector;
                    (token, members)
                } else {
                    groups.push((token, members));
                    previous_was_connector = false;
                    (word.clone(), vec![word])
                }
            }
        });
    }
    if let Some(group) = current {
        groups.push(group);
    }

    groups
}

/// Reconstruct a table grid from words with bounding box positions.
///
/// Takes detected words and reconstructs a 2D table by:
/// 1. Detecting row positions (grouping by y-center within `row_threshold_ratio` * median height)
/// 2. Merging horizontally-adjacent words within the same row into cell tokens
///    (see [`CELL_MERGE_GAP_HEIGHT_RATIO`]) so a multi-word cell does not mint
///    spurious columns
/// 3. Detecting column positions from those tokens (grouping by x-coordinate
///    within `column_threshold`)
/// 4. Assigning the *original* words to cells based on closest row/column
/// 5. Combining words within the same cell
///
/// Returns a `Vec<Vec<String>>` where each inner `Vec` is a row of cell texts.
///
/// Thin wrapper over [`reconstruct_table_with_columns`] for callers that only
/// need the grid; use that function instead when the column x-positions used
/// to build the grid must stay correlated with it (e.g. a caller that later
/// indexes `column_positions[column]` against `grid[row][column]`).
#[cfg(any(feature = "pdf", paddle_ocr, test))]
pub(crate) fn reconstruct_table(
    words: &[HocrWord],
    column_threshold: u32,
    row_threshold_ratio: f64,
) -> Vec<Vec<String>> {
    reconstruct_table_with_columns(words, column_threshold, row_threshold_ratio).0
}

/// Like [`reconstruct_table`], but also returns the column x-positions
/// actually used to build the grid, filtered to the same set of columns that
/// survived empty-column removal — so `result.1[i]` corresponds exactly to
/// `result.0[row][i]` for every row.
///
/// Callers that separately call [`detect_columns`] on the raw input words and
/// then index a returned grid by those positions (e.g. to compare adjacent
/// columns) MUST use this function instead: `reconstruct_table` now detects
/// columns from post-merge cell tokens, not from `words` directly, so a
/// `detect_columns(words, ...)` call sitting next to a plain `reconstruct_table`
/// call no longer corresponds to that grid's column count or positions.
pub(crate) fn reconstruct_table_with_columns(
    words: &[HocrWord],
    column_threshold: u32,
    row_threshold_ratio: f64,
) -> (Vec<Vec<String>>, Vec<u32>) {
    if words.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let row_positions = detect_rows(words, row_threshold_ratio);
    let median_height = median_word_height(words);
    let groups = group_words_into_cell_tokens(words, &row_positions);
    let cell_tokens: Vec<HocrWord> = groups.iter().map(|(token, _)| token.clone()).collect();
    let columns = detect_columns(&cell_tokens, &row_positions, column_threshold);

    if columns.is_empty() || row_positions.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut result = assign_grouped_words_to_cells(&groups, &row_positions, median_height, &columns);
    let mut col_positions: Vec<u32> = columns.iter().map(|column| column.left).collect();
    merge_header_fragments_by_geometry(&mut result, &mut col_positions);

    let non_empty_cols = non_empty_column_mask(&result);
    let kept_col_positions: Vec<u32> = col_positions
        .iter()
        .zip(non_empty_cols.iter())
        .filter(|&(_, &keep)| keep)
        .map(|(&pos, _)| pos)
        .collect();

    (remove_empty_rows_and_columns(result), kept_col_positions)
}

fn is_rtl_letter(character: char) -> bool {
    character.is_alphabetic()
        && matches!(character,
            '\u{0590}'..='\u{08ff}' | '\u{fb1d}'..='\u{fdff}' | '\u{fe70}'..='\u{feff}')
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DirectionVote {
    Rtl,
    Ltr,
    Neutral,
}

fn direction_vote(text: &str) -> DirectionVote {
    if text.chars().any(is_rtl_letter) {
        return DirectionVote::Rtl;
    }
    let has_latin = text.chars().any(|character| character.is_ascii_alphabetic());
    if !has_latin {
        return DirectionVote::Neutral;
    }
    let lower = text.to_ascii_lowercase();
    let identifier_like = text.chars().any(|character| character.is_ascii_digit())
        || text.contains(['/', '\\', '@', '_'])
        || text.contains("://")
        || lower.starts_with("www.")
        || (!text.contains(char::is_whitespace) && text.chars().count() > 20)
        || (text.chars().filter(|character| character.is_ascii_alphabetic()).count() > 1
            && text
                .chars()
                .filter(|character| character.is_ascii_alphabetic())
                .all(|character| character.is_ascii_uppercase()));
    if identifier_like {
        DirectionVote::Neutral
    } else {
        DirectionVote::Ltr
    }
}

pub(crate) fn hocr_words_are_rtl<'a>(words: impl Iterator<Item = &'a HocrWord>) -> bool {
    let mut rtl_count = 0usize;
    let mut ltr_count = 0usize;
    let mut rtl_x = 0u64;
    let mut ltr_x = 0u64;
    for word in words {
        match direction_vote(&word.text) {
            DirectionVote::Rtl => {
                rtl_count += 1;
                rtl_x += word.left as u64;
            }
            DirectionVote::Ltr => {
                ltr_count += 1;
                ltr_x += word.left as u64;
            }
            DirectionVote::Neutral => {}
        }
    }
    if rtl_count != ltr_count {
        return rtl_count > ltr_count;
    }
    rtl_count > 0 && rtl_x * ltr_count as u64 > ltr_x * rtl_count as u64
}

#[cfg(any(feature = "ocr", test))]
pub(crate) fn orient_table_for_reading_order(
    table: &mut [Vec<String>],
    column_positions: &mut [u32],
    words: &[HocrWord],
) {
    if !hocr_words_are_rtl(words.iter()) || !column_positions.windows(2).all(|pair| pair[0] <= pair[1]) {
        return;
    }
    for row in table {
        row.reverse();
    }
    column_positions.reverse();
}

const MIN_HEADER_NEIGHBOR_SUPPORT: usize = 2;

/// A header word can start left of the data values it labels and form a header-only x-track.
/// Attach that fragment to the nearest column supported by multiple data rows, while retaining
/// the correlated x-position vector used by downstream table geometry checks. ~keep
fn merge_header_fragments_by_geometry(table: &mut [Vec<String>], column_positions: &mut Vec<u32>) {
    let column_count = table.first().map_or(0, Vec::len);
    if table.len() < 3 || column_count < 3 || column_positions.len() != column_count {
        return;
    }
    if !has_split_invoice_header(&table[0]) {
        return;
    }

    let support: Vec<usize> = (0..column_count)
        .map(|column| {
            table
                .iter()
                .skip(1)
                .filter(|row| row.get(column).is_some_and(|cell| !cell.trim().is_empty()))
                .count()
        })
        .collect();
    let targets = header_fragment_targets(table, column_positions, &support);
    if targets.iter().all(Option::is_none) {
        return;
    }
    rebuild_table_without_header_fragments(table, column_positions, &targets);
}

fn has_split_invoice_header(header: &[String]) -> bool {
    let has = |expected: &str| header.iter().any(|cell| cell.trim().eq_ignore_ascii_case(expected));
    has("QTY") && ((has("UNIT") && has("PRICE")) || (has("LINE") && has("TOTAL")))
}

fn header_fragment_targets(table: &[Vec<String>], column_positions: &[u32], support: &[usize]) -> Vec<Option<usize>> {
    let column_count = support.len();
    let mut targets = vec![None; column_count];
    let mut left = None;
    for column in 0..column_count {
        targets[column] = left;
        if support[column] >= MIN_HEADER_NEIGHBOR_SUPPORT {
            left = Some(column);
        }
    }
    let mut right = None;
    for column in (0..column_count).rev() {
        if support[column] >= MIN_HEADER_NEIGHBOR_SUPPORT {
            targets[column] = None;
            right = Some(column);
            continue;
        }
        let Some(left) = targets[column] else {
            continue;
        };
        let Some(right) = right else {
            continue;
        };
        if support[column] != 0 || !is_split_invoice_fragment(&table[0][column], &table[0][right]) {
            targets[column] = None;
            continue;
        }
        targets[column] = sufficiently_near_matching_column(column_positions, left, column, right);
    }
    targets
}

fn is_split_invoice_fragment(fragment: &str, destination: &str) -> bool {
    let fragment = fragment.trim();
    let destination = destination.trim();
    (fragment.eq_ignore_ascii_case("UNIT") && destination.eq_ignore_ascii_case("PRICE"))
        || (fragment.eq_ignore_ascii_case("LINE") && destination.eq_ignore_ascii_case("TOTAL"))
}

fn sufficiently_near_matching_column(positions: &[u32], left: usize, fragment: usize, right: usize) -> Option<usize> {
    let span = positions[right].abs_diff(positions[left]);
    let right_distance = positions[fragment].abs_diff(positions[right]);
    (right_distance.saturating_mul(3) < span).then_some(right)
}

fn rebuild_table_without_header_fragments(
    table: &mut [Vec<String>],
    column_positions: &mut Vec<u32>,
    targets: &[Option<usize>],
) {
    let column_count = targets.len();
    let (prefixes, suffixes) = header_fragment_affixes(table, targets);
    let kept_columns = targets.iter().filter(|target| target.is_none()).count();
    for (row_index, row) in table.iter_mut().enumerate() {
        let mut rebuilt = Vec::with_capacity(kept_columns);
        for column in 0..column_count {
            if targets[column].is_some() {
                continue;
            }
            if row_index == 0 {
                rebuilt.push(rebuilt_header(&row[column], &prefixes[column], &suffixes[column]));
            } else {
                rebuilt.push(std::mem::take(&mut row[column]));
            }
        }
        *row = rebuilt;
    }
    let mut position_index = 0usize;
    column_positions.retain(|_| {
        let keep_position = targets[position_index].is_none();
        position_index += 1;
        keep_position
    });
}

fn header_fragment_affixes(table: &[Vec<String>], targets: &[Option<usize>]) -> (Vec<String>, Vec<String>) {
    let column_count = targets.len();
    let mut prefixes = vec![String::new(); column_count];
    let mut suffixes = vec![String::new(); column_count];
    for (column, target) in targets.iter().copied().enumerate() {
        let Some(target) = target else {
            continue;
        };
        let fragment = table[0][column].trim();
        let destination = if target < column {
            &mut suffixes[target]
        } else {
            &mut prefixes[target]
        };
        if !destination.is_empty() {
            destination.push(' ');
        }
        destination.push_str(fragment);
    }
    (prefixes, suffixes)
}

fn rebuilt_header(original: &str, prefix: &str, suffix: &str) -> String {
    let original = original.trim();
    let capacity = prefix.len() + original.len() + suffix.len() + 2;
    let mut header = String::with_capacity(capacity);
    for part in [prefix, original, suffix] {
        if part.is_empty() {
            continue;
        }
        if !header.is_empty() {
            header.push(' ');
        }
        header.push_str(part);
    }
    header
}

/// Fraction of a cell's median word height within which two words belong to the same visual line.
///
/// Sized to sit between the two deltas it must tell apart: a sub/superscript is offset from its
/// base's vertical centre by a fraction of its own height (0.6pt against an 11.59pt line on the
/// GH#1628 reproducer), while a wrapped second line is a full line height away. Shares the shape,
/// but not the value, of [`CELL_MERGE_GAP_HEIGHT_RATIO`], which measures a horizontal gap. ~keep
const CELL_LINE_GROUP_HEIGHT_RATIO: f64 = 0.5;

/// Order one cell's words for joining: top-to-bottom by visual line, left-to-right within a line.
///
/// Arrival order is wrong for a cell containing a sub/superscript (xberg-io/xberg#1628). A
/// sub/superscript is drawn as its own content-stream segment sitting a fraction of a point below
/// the line it annotates, so every reading-order sort upstream of here places it after the whole
/// line rather than beside the symbol it belongs to — `eta_S %` joins as `eta % S`.
///
/// Sorting the cell by `left` alone fixes that and introduces something worse: a cell holding two
/// wrapped lines interleaves them (`Hello world` / `again here` becomes `Hello again world here`),
/// which would scramble every wrapped cell in the corpus rather than only cells with scripts.
/// Grouping into visual lines first separates the two cases, because the vertical deltas differ by
/// an order of magnitude — see [`CELL_LINE_GROUP_HEIGHT_RATIO`]. ~keep
fn order_cell_words_in_reading_order(mut cell_words: Vec<&HocrWord>) -> Vec<&HocrWord> {
    if cell_words.len() <= 1 {
        return cell_words;
    }

    let mut heights: Vec<u32> = cell_words.iter().map(|word| word.height).collect();
    heights.sort_unstable();
    let line_gap = heights[heights.len() / 2] as f64 * CELL_LINE_GROUP_HEIGHT_RATIO;

    cell_words.sort_by(|a, b| a.y_center().total_cmp(&b.y_center()).then_with(|| a.left.cmp(&b.left)));

    let mut lines: Vec<Vec<&HocrWord>> = Vec::new();
    for word in cell_words {
        let same_line = lines.last().is_some_and(|line| {
            let centre = line.iter().map(|w| w.y_center()).sum::<f64>() / line.len() as f64;
            (word.y_center() - centre).abs() <= line_gap
        });
        match lines.last_mut() {
            Some(line) if same_line => line.push(word),
            _ => lines.push(vec![word]),
        }
    }

    for line in &mut lines {
        if hocr_words_are_rtl(line.iter().copied()) {
            line.sort_by_key(|word| std::cmp::Reverse(word.left));
        } else {
            line.sort_by_key(|word| word.left);
        }
    }
    lines.into_iter().flatten().collect()
}

/// Assign each original word independently to its nearest detected row/column and combine
/// same-cell words into space-joined cell text. Retained as a direct-unit-test seam for that
/// per-word assignment and cell-ordering behavior in isolation; the production path
/// (`reconstruct_table_with_columns`) calls [`assign_grouped_words_to_cells`] instead, which
/// assigns a whole merged cell cluster to one column at a time (xberg-io/xberg#1649). ~keep
#[cfg(test)]
fn assign_words_to_cells(words: &[HocrWord], row_positions: &[u32], col_positions: &[u32]) -> Vec<Vec<String>> {
    let columns: Vec<ColumnTrack> = col_positions.iter().copied().map(ColumnTrack::left_aligned).collect();
    let num_rows = row_positions.len();
    let num_cols = columns.len();
    let mut table: Vec<Vec<Vec<&HocrWord>>> = vec![vec![vec![]; num_cols]; num_rows];

    for word in words {
        if let (Some(r), Some(c)) = (find_row_index(row_positions, word), find_column_index(&columns, word))
            && r < num_rows
            && c < num_cols
        {
            table[r][c].push(word);
        }
    }

    finish_cell_assignment(table)
}

/// Like [`assign_words_to_cells`], but decides row/column membership once per merged cell
/// cluster (see [`group_words_into_cell_tokens`]) from the cluster's own summary token, then
/// places every original word the cluster contains into that same cell — instead of resolving
/// each original word's column independently, which lets a multi-word cell's trailing word drift
/// into a neighboring column (xberg-io/xberg#1649).
fn assign_grouped_words_to_cells<'a>(
    groups: &[(HocrWord, Vec<&'a HocrWord>)],
    row_positions: &[u32],
    median_height: u32,
    columns: &[ColumnTrack],
) -> Vec<Vec<String>> {
    let num_rows = row_positions.len();
    let num_cols = columns.len();
    let mut table: Vec<Vec<Vec<&'a HocrWord>>> = vec![vec![vec![]; num_cols]; num_rows];
    let data_supported = columns_with_data_support(groups, row_positions, median_height, columns);

    for (token, members) in groups {
        let Some(row) = find_row_index_by_overlap(row_positions, token, median_height) else {
            continue;
        };
        if row >= num_rows {
            continue;
        }
        if row == 0 {
            // Row 0 is conventionally the header row throughout this module (see
            // `data_support_count`, `table_to_markdown`). Keep a merged header token together when
            // its column has data below it; splitting its words again can move a trailing word to
            // the next header (#1934). A header-only token can genuinely span toward the data
            // column it labels, so retain independent placement for that case and let
            // `merge_header_fragments_by_geometry` reconcile it (#2219). ~keep
            let token_column = find_column_index(columns, token);
            if let Some(col) = token_column.filter(|&col| col < num_cols && data_supported[col]) {
                table[row][col].extend(members.iter().copied());
            } else {
                assign_words_to_row(&mut table[row], members, columns);
            }
        } else if let Some(col) = find_column_index(columns, token)
            && col < num_cols
        {
            table[row][col].extend(members.iter().copied());
        }
    }

    finish_cell_assignment(table)
}

fn columns_with_data_support(
    groups: &[(HocrWord, Vec<&HocrWord>)],
    row_positions: &[u32],
    median_height: u32,
    columns: &[ColumnTrack],
) -> Vec<bool> {
    let mut supported = vec![false; columns.len()];
    for (token, _) in groups {
        if find_row_index_by_overlap(row_positions, token, median_height) != Some(0)
            && let Some(column) = find_column_index(columns, token)
        {
            supported[column] = true;
        }
    }
    supported
}

fn assign_words_to_row<'a>(row: &mut [Vec<&'a HocrWord>], words: &[&'a HocrWord], columns: &[ColumnTrack]) {
    for &word in words {
        if let Some(column) = find_column_index(columns, word) {
            row[column].push(word);
        }
    }
}

/// Shared tail of [`assign_words_to_cells`] and [`assign_grouped_words_to_cells`]: order each
/// cell's collected words and join them into the final cell text.
fn finish_cell_assignment(table: Vec<Vec<Vec<&HocrWord>>>) -> Vec<Vec<String>> {
    table
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|cell_words| {
                    // A "word" whose whole segment held no whitespace keeps that segment's own
                    // trailing space (`split_segment_to_words_lifted` returns it unsplit), which
                    // then stacks on the separator below and renders `Q_HE GJ` as `Q  HE GJ`.
                    // Collapse each word's own whitespace so the join alone owns the spacing. ~keep
                    join_cell_words(&order_cell_words_in_reading_order(cell_words))
                })
                .collect()
        })
        .collect()
}

// ~keep Joining numeric fragments only after column assignment preserves the detector's
// token counts: joining earlier changes dense-table classification and merges real columns (GH#2122).
fn join_cell_words(words: &[&HocrWord]) -> String {
    let mut text = String::new();
    let mut previous = None;
    for word in words {
        let normalized = word.text.split_whitespace().collect::<Vec<_>>().join(" ");
        if normalized.is_empty() {
            previous = None;
            continue;
        }
        if !text.is_empty() && !previous.is_some_and(|prev| is_touching_numeric_continuation(prev, word)) {
            text.push(' ');
        }
        text.push_str(&normalized);
        previous = Some(*word);
    }
    text
}

fn is_touching_numeric_continuation(previous: &HocrWord, next: &HocrWord) -> bool {
    let prefix = previous.text.as_str();
    let suffix = next.text.trim_end().trim_end_matches([')', ']', '%']);
    if !prefix.ends_with(|character: char| character.is_ascii_digit())
        || !prefix
            .chars()
            .all(|character| character.is_ascii_digit() || matches!(character, '.' | ',' | '(' | '[' | '+' | '-' | '−'))
        || !suffix.starts_with(['.', ','])
        || !suffix
            .chars()
            .nth(1)
            .is_some_and(|character| character.is_ascii_digit())
        || !suffix
            .chars()
            .all(|character| character.is_ascii_digit() || matches!(character, '.' | ','))
    {
        return false;
    }

    // ~keep HocrWord rounds PDF coordinates to pixels; allow one rounding pixel at either edge.
    let tolerance = (previous.height.max(next.height) as f64 * NUMERIC_CONTINUATION_GAP_HEIGHT_RATIO)
        .max(NUMERIC_CONTINUATION_ROUNDING_TOLERANCE as f64);
    let gap = next.left as f64 - previous.left as f64 - previous.width as f64;
    previous.top.abs_diff(next.top) <= NUMERIC_CONTINUATION_ROUNDING_TOLERANCE
        && previous.height.abs_diff(next.height) <= NUMERIC_CONTINUATION_ROUNDING_TOLERANCE
        && gap.abs() <= tolerance
}

/// Convert a table grid to markdown format.
///
/// The first row is treated as the header row, with a separator line added after it.
///
/// Delegates to the crate's single table renderer
/// ([`crate::rendering::common::render_table_markdown`]) so a reconstructed
/// OCR/PDF table serialises identically to one from any other source
/// (xberg-io/xberg#220). That renderer also pads every row to the widest row,
/// which this reconstruction path did not do — ragged rows used to emit fewer
/// pipe columns than the header, misaligning the table for downstream parsers.
pub(crate) fn table_to_markdown(table: &[Vec<String>]) -> String {
    crate::rendering::common::render_table_markdown(table)
}

/// Multiple of the page's average word height used as the vertical-gap
/// threshold for splitting table candidate words into separate regions
/// (#177). Normal row spacing inside one table rarely exceeds ~1.5x the
/// average word height, so a wider multiple avoids splitting a single
/// table's own row gaps while still separating genuinely distinct tables
/// (or a table from surrounding prose) on the same page.
//
// This module also compiles for `feature = "pdf"` (shared with the native PDF table path), but
// the only real callers of this constant -- `ocr::processor::execution` (gated `feature = "ocr"`)
// and `paddle_ocr::backend` (gated `paddle_ocr`) -- need neither `pdf` nor each other's gate. A
// gate as wide as the module's left this compiled with zero callers on a `pdf`-only leg. ~keep
#[cfg(any(feature = "ocr", paddle_ocr))]
pub(crate) const TABLE_REGION_GAP_HEIGHT_MULTIPLIER: u32 = 3;

/// Minimum number of words for a spatial region to be treated as a table
/// candidate. Mirrors the previous whole-page threshold so a single small
/// table on an otherwise text-only page is not over-fabricated.
//
// Same reasoning as `TABLE_REGION_GAP_HEIGHT_MULTIPLIER` above: real callers are gated `ocr` or
// `paddle_ocr`, not `pdf`. ~keep
#[cfg(any(feature = "ocr", paddle_ocr))]
pub(crate) const MIN_TABLE_CANDIDATE_WORDS: usize = 6;

#[cfg(any(feature = "ocr", paddle_ocr))]
const MAX_ALIGNED_FRAGMENT_GAP_HEIGHT_MULTIPLIER: u32 = 12;

#[cfg(any(feature = "ocr", paddle_ocr))]
const MAX_ALIGNED_FULL_REGION_GAP_HEIGHT_MULTIPLIER: u32 = 4;

/// Split table-candidate words into vertically separated regions.
///
/// Tesseract's TSV output has no notion of "this is a separate table from
/// that one" — [`reconstruct_table`] previously ran once over every
/// table-confidence word on the page, producing at most one table whose
/// bounding box spanned the union of all such words, even when the page had
/// several independent tables separated by paragraphs of prose (#177).
///
/// This groups words by contiguous vertical extent: a gap between one row's
/// bottom edge and the next word's top edge wider than
/// `TABLE_REGION_GAP_HEIGHT_MULTIPLIER` times the average word height starts
/// a new region. Each region is reconstructed independently, giving each
/// table its own bounding box.
///
// Same reasoning as `TABLE_REGION_GAP_HEIGHT_MULTIPLIER` above: the one real caller,
// `PaddleOcrBackend::build_ocr_tables_from_words` (gated `paddle_ocr`), never needs the wider
// `pdf` gate this module carries. The Tesseract table branch calls
// [`cluster_word_indices_into_table_regions`] instead. ~keep
#[cfg(any(paddle_ocr, all(test, feature = "ocr")))]
pub(crate) fn cluster_words_into_table_regions(words: &[HocrWord]) -> Vec<Vec<HocrWord>> {
    cluster_word_indices_into_table_regions(words)
        .into_iter()
        .map(|region| region.into_iter().map(|index| words[index].clone()).collect())
        .collect()
}

/// Split table-candidate words into vertically separated regions, each given as indices into
/// `words`. The rule is the one `cluster_words_into_table_regions` documents.
// Called by `ocr::processor::execution::perform_ocr`'s table-detection branch (gated
// `feature = "ocr"`) and through the wrapper above (gated `paddle_ocr`). ~keep
#[cfg(any(feature = "ocr", paddle_ocr))]
pub(crate) fn cluster_word_indices_into_table_regions(words: &[HocrWord]) -> Vec<Vec<usize>> {
    if words.is_empty() {
        return Vec::new();
    }

    let mut sorted: Vec<usize> = (0..words.len()).collect();
    sorted.sort_by(|&a, &b| words[a].top.cmp(&words[b].top).then(words[a].left.cmp(&words[b].left)));

    let avg_height: u32 = {
        let total: u32 = words.iter().map(|w| w.height).sum();
        (total / words.len() as u32).max(1)
    };
    let region_gap_threshold = avg_height * TABLE_REGION_GAP_HEIGHT_MULTIPLIER;

    let mut regions: Vec<Vec<usize>> = Vec::new();
    let mut current_region: Vec<usize> = Vec::new();
    let mut current_bottom: u32 = 0;

    for index in sorted {
        let word = &words[index];
        let word_bottom = word.top + word.height;
        let is_new_region =
            !current_region.is_empty() && word.top.saturating_sub(current_bottom) > region_gap_threshold;

        if is_new_region {
            regions.push(std::mem::take(&mut current_region));
            current_bottom = 0;
        }

        current_bottom = current_bottom.max(word_bottom);
        current_region.push(index);
    }
    if !current_region.is_empty() {
        regions.push(current_region);
    }

    merge_aligned_regions(&mut regions, words, avg_height);
    regions
}

/// Attach adjacent table rows when their columns and vertical spacing support continuation.
///
/// A table whose rows are spaced widely enough to exceed the region gap threshold splits into one
/// region per row. A one-row region holds fewer than the minimum words, so the table branch drops
/// it entirely and the row's values are lost (xberg-io/xberg#1957: an invoice's trailing line
/// items). A region below the minimum is not a table on its own. A larger row can also continue a
/// table when three columns align across a short gap; the stricter alignment and distance keep
/// complete nearby tables separate (xberg-io/xberg#2043). ~keep
#[cfg(any(feature = "ocr", paddle_ocr))]
fn merge_aligned_regions(regions: &mut Vec<Vec<usize>>, words: &[HocrWord], tolerance: u32) {
    let mut merged: Vec<Vec<usize>> = Vec::with_capacity(regions.len());
    for region in regions.drain(..) {
        if let Some(previous) = merged.last_mut()
            && regions_continue(previous, &region, words, tolerance)
        {
            previous.extend(region);
            continue;
        }
        merged.push(region);
    }
    // A small leading fragment has no region above it; attach it downward instead.
    let mut index = 0;
    while index + 1 < merged.len() {
        if merged[index].len() < MIN_TABLE_CANDIDATE_WORDS
            && columns_align(&merged[index + 1], &merged[index], words, tolerance, 2)
            && region_gap(&merged[index + 1], &merged[index], words)
                <= tolerance.saturating_mul(MAX_ALIGNED_FRAGMENT_GAP_HEIGHT_MULTIPLIER)
        {
            let fragment = merged.remove(index);
            merged[index].extend(fragment);
            continue;
        }
        index += 1;
    }
    *regions = merged;
}

#[cfg(any(feature = "ocr", paddle_ocr))]
fn regions_continue(first: &[usize], second: &[usize], words: &[HocrWord], tolerance: u32) -> bool {
    let small = second.len() < MIN_TABLE_CANDIDATE_WORDS;
    let min_columns = if small { 2 } else { 3 };
    let max_gap = if small {
        MAX_ALIGNED_FRAGMENT_GAP_HEIGHT_MULTIPLIER
    } else {
        MAX_ALIGNED_FULL_REGION_GAP_HEIGHT_MULTIPLIER
    };
    // ~keep A repeated header can align perfectly with a preceding table across the same short
    // gap. Require two numeric cells in the next region's first row before merging full regions.
    (small || first_row_has_two_values(second, words, tolerance))
        && columns_align(first, second, words, tolerance, min_columns)
        && region_gap(first, second, words) <= tolerance.saturating_mul(max_gap)
}

#[cfg(any(feature = "ocr", paddle_ocr))]
fn first_row_has_two_values(region: &[usize], words: &[HocrWord], tolerance: u32) -> bool {
    let first_top = region.iter().map(|&index| words[index].top).min().unwrap_or(0);
    region
        .iter()
        .filter(|&&index| words[index].top.saturating_sub(first_top) <= tolerance / 2)
        .filter(|&&index| text_is_value(&words[index].text))
        .take(2)
        .count()
        == 2
}

#[cfg(any(feature = "ocr", paddle_ocr))]
fn region_gap(first: &[usize], second: &[usize], words: &[HocrWord]) -> u32 {
    let first_top = first.iter().map(|&index| words[index].top).min().unwrap_or(0);
    let first_bottom = first
        .iter()
        .map(|&index| words[index].top.saturating_add(words[index].height))
        .max()
        .unwrap_or(0);
    let second_top = second.iter().map(|&index| words[index].top).min().unwrap_or(0);
    let second_bottom = second
        .iter()
        .map(|&index| words[index].top.saturating_add(words[index].height))
        .max()
        .unwrap_or(0);
    first_top
        .saturating_sub(second_bottom)
        .max(second_top.saturating_sub(first_bottom))
}

/// Whether enough distinct columns of `fragment` line up with `anchor` within `tolerance` points.
/// A scan's column jitter is a fraction of a word's height, so callers pass the average word height.
#[cfg(any(feature = "ocr", paddle_ocr))]
fn columns_align(anchor: &[usize], fragment: &[usize], words: &[HocrWord], tolerance: u32, minimum: usize) -> bool {
    let mut matched = 0usize;
    let mut seen: Vec<u32> = Vec::new();
    for &fragment_index in fragment {
        let left = words[fragment_index].left;
        if seen.iter().any(|&seen_left| seen_left.abs_diff(left) <= tolerance) {
            continue;
        }
        seen.push(left);
        if anchor
            .iter()
            .any(|&anchor_index| words[anchor_index].left.abs_diff(left) <= tolerance)
        {
            matched += 1;
            if matched >= minimum {
                return true;
            }
        }
    }
    false
}

/// Multiple of the region's median word height used as the maximum gap for merging two
/// adjacent OCR table columns that are very likely one logical column split by word-start
/// drift (xberg-io/xberg#1649).
///
/// Right-aligned numeric columns (amounts, balances) have a left edge that shifts with digit
/// count -- `$1,250.00` starts well left of `$87.32` in the same visual column -- so
/// [`detect_columns`]'s absolute `column_threshold` (tuned for left-aligned label columns, and
/// applied in raster-pixel space after DPI normalization can upscale the page 2x+) can end up
/// splitting one logical column into several. 3.0 mirrors [`TABLE_REGION_GAP_HEIGHT_MULTIPLIER`]'s
/// use of median word height as the scale-invariant unit: measured gaps of 95-120px against a
/// 51px median height (roughly 1.9-2.4x) on the reported fixture merge comfortably under this
/// multiplier, while the gap to a genuinely different column (WITHDRAWAL to DEPOSIT, ~454px,
/// ~8.9x) stays well clear of it.
#[cfg(feature = "ocr")]
pub(crate) const DISJOINT_NUMERIC_COLUMN_MERGE_HEIGHT_MULTIPLIER: f64 = 3.0;

/// A non-empty, trimmed cell that looks like a plain number or currency amount: at least one
/// ASCII digit, and no characters outside a small currency/number charset. Deliberately narrow
/// so it never matches ordinary prose (a lone `-` or `()` alone does not count -- see
/// [`merge_disjoint_numeric_columns`], which relies on this to avoid merging text columns).
#[cfg(feature = "ocr")]
fn looks_like_amount(cell: &str) -> bool {
    let trimmed = cell.trim();
    !trimmed.is_empty()
        && trimmed.chars().any(|ch| ch.is_ascii_digit())
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_digit() || "$,.-()%€£¥+".contains(ch))
}

/// Whether `cell` carries no letter and no digit: empty, or only marks such as `-`, a dash run,
/// `:`, `_`, `|` or `.`. On a scan these are what OCR reads from rules and shaded bands, so the
/// drift-split merge counts such a cell as empty rather than as a value of its own.
#[cfg(feature = "ocr")]
fn has_no_alphanumeric(cell: &str) -> bool {
    !cell.chars().any(char::is_alphanumeric)
}

/// Count of header rows above the data: row 0, plus every following row up to the first row that
/// holds an amount in any column. A scanned header band can leave a stray glyph in a row of its
/// own under the header text, and that row is part of the header, not data.
#[cfg(feature = "ocr")]
fn leading_header_row_count(table: &[Vec<String>]) -> usize {
    table
        .iter()
        .position(|row| row.iter().any(|cell| looks_like_amount(cell)))
        .unwrap_or(table.len())
        .max(1)
}

/// Whether columns `left` and `right` of `table` (the first `header_rows` rows are the header,
/// excluded from this check) are mutually exclusive -- no data row has both populated -- and
/// every populated data cell in either column looks like a number/currency amount. Both
/// conditions must hold for [`merge_disjoint_numeric_columns`] to treat the pair as one logical
/// column split in two. A cell with no letter or digit counts as empty here (see
/// [`has_no_alphanumeric`]), except a lone cell symbol in front of a value in the right track: a
/// `-`, `$` or `(` that OCR split off the front of that value is part of it
/// ([`is_lone_cell_symbol`]), and merging would drop it.
#[cfg(feature = "ocr")]
fn columns_are_disjoint_and_numeric(table: &[Vec<String>], header_rows: usize, left: usize, right: usize) -> bool {
    let mut any_data = false;
    for row in table.iter().skip(header_rows) {
        let (Some(left_cell), Some(right_cell)) = (row.get(left), row.get(right)) else {
            return false;
        };
        let left_empty = has_no_alphanumeric(left_cell);
        let right_empty = has_no_alphanumeric(right_cell);
        match (left_empty, right_empty) {
            (true, true) => {}
            (false, false) => return false,
            (true, false) if is_lone_cell_symbol(left_cell.trim()) => return false,
            (false, true) => {
                if !looks_like_amount(left_cell) {
                    return false;
                }
                any_data = true;
            }
            (true, false) => {
                if !looks_like_amount(right_cell) {
                    return false;
                }
                any_data = true;
            }
        }
    }
    any_data
}

/// Whether at most one of columns `left`/`right` carries its own header label in the first
/// `header_rows` rows of `table`. A header cell with no letter or digit (see
/// [`has_no_alphanumeric`]) is not a label.
///
/// A drift-split numeric column (this function's target) has its header text in only one of the
/// split pieces -- the other piece's header cell is empty, because the source document had one
/// label for the whole logical column. Two genuinely distinct, independently-labeled columns
/// (e.g. a ledger's separately headed "Debit" and "Credit") must never be folded together by
/// [`merge_disjoint_numeric_columns`] even when their data happens to be mutually exclusive per
/// row and their x-positions sit close together -- that shape is a normal compact table layout,
/// not a drift artifact, and this guard keeps it untouched regardless of the gap/exclusivity
/// checks. ~keep
#[cfg(feature = "ocr")]
fn at_most_one_column_has_its_own_header(table: &[Vec<String>], header_rows: usize, left: usize, right: usize) -> bool {
    let header = &table[..header_rows.min(table.len())];
    let labeled = |column: usize| {
        header
            .iter()
            .any(|row| row.get(column).is_some_and(|cell| !has_no_alphanumeric(cell)))
    };
    !(labeled(left) && labeled(right))
}

/// Fold column `right` into column `left` in place, then drop column `right` from every row.
///
/// When only one of a row's two cells carries a letter or digit, that cell wins and the other
/// (empty, or a rule mark such as `-` or `:`) is dropped. Callers only reach here after
/// [`columns_are_disjoint_and_numeric`] and [`at_most_one_column_has_its_own_header`] confirm
/// that no row, header included, has a value on both sides. When neither cell has a letter or
/// digit, their non-empty text joins with a space (order preserved), so a lone `-` nil marker
/// survives.
#[cfg(feature = "ocr")]
fn merge_column_into(table: &mut [Vec<String>], left: usize, right: usize) {
    for row in table.iter_mut() {
        let right_cell = row.remove(right);
        let right_cell = right_cell.trim();
        let left_cell = row[left].trim();
        row[left] = match (has_no_alphanumeric(left_cell), has_no_alphanumeric(right_cell)) {
            (false, true) => left_cell.to_string(),
            (true, false) => right_cell.to_string(),
            _ => [left_cell, right_cell]
                .into_iter()
                .filter(|cell| !cell.is_empty())
                .collect::<Vec<_>>()
                .join(" "),
        };
    }
}

/// Count of data cells (the first `header_rows` rows excluded) in `column` that hold a letter or
/// digit.
#[cfg(feature = "ocr")]
fn data_support_count(table: &[Vec<String>], header_rows: usize, column: usize) -> usize {
    table
        .iter()
        .skip(header_rows)
        .filter(|row| !has_no_alphanumeric(&row[column]))
        .count()
}

/// Merge adjacent OCR table columns that are very likely one logical (typically right-aligned,
/// numeric) column split by word-start drift (xberg-io/xberg#1649): see
/// [`DISJOINT_NUMERIC_COLUMN_MERGE_HEIGHT_MULTIPLIER`] for the distance bound and
/// [`columns_are_disjoint_and_numeric`] for the content/exclusivity bound. Both must hold, so
/// two genuinely distinct columns (different labels, or data that ever coexists in the same
/// row) are left untouched.
///
/// A right-aligned amount column can split into more than two pieces (e.g. a header-only
/// column with no data support at all, plus two data-bearing pieces at different digit-count
/// widths), so this keeps re-checking the same left index against its new right neighbor after
/// each merge rather than advancing past it. Each merge re-anchors `column_positions[column]`
/// on whichever side actually carries more data, not always the geometrically-leftmost side —
/// otherwise a header-only column's position (with zero data support) would stay the anchor and
/// the next real data column could land just outside `max_gap` of it, even though both data
/// pieces are close together.
#[cfg(feature = "ocr")]
pub(crate) fn merge_disjoint_numeric_columns(
    table: &mut [Vec<String>],
    column_positions: &mut Vec<u32>,
    median_height: u32,
) {
    if table.len() < 2 || column_positions.len() < 2 {
        return;
    }
    let max_gap = median_height as f64 * DISJOINT_NUMERIC_COLUMN_MERGE_HEIGHT_MULTIPLIER;
    let header_rows = leading_header_row_count(table);
    let mut column = 0;
    while column + 1 < column_positions.len() {
        let gap = column_positions[column + 1].abs_diff(column_positions[column]) as f64;
        if gap <= max_gap
            && at_most_one_column_has_its_own_header(table, header_rows, column, column + 1)
            && columns_are_disjoint_and_numeric(table, header_rows, column, column + 1)
        {
            if data_support_count(table, header_rows, column + 1) > data_support_count(table, header_rows, column) {
                column_positions[column] = column_positions[column + 1];
            }
            merge_column_into(table, column, column + 1);
            column_positions.remove(column + 1);
        } else {
            column += 1;
        }
    }
}

/// Drop a leading section-caption row that [`cluster_words_into_table_regions`] joined to the
/// same region as a genuine header row (xberg-io/xberg#1649): a section title (e.g.
/// "TRANSACTIONS") sitting close enough above a table's header to share its region has only one
/// populated cell, spanning what OCR resolved as the leftmost column, while a real header row
/// populates most of the grid's columns.
///
/// Deliberately narrow: requires at least 3 columns and 3 rows (so this never fires on a
/// two-row summary card, which has no separate caption to begin with), the first row's only
/// non-empty cell in column 0, and the second row populating at least 3 columns.
#[cfg(feature = "ocr")]
pub(crate) fn drop_leading_caption_row(table: &mut Vec<Vec<String>>) {
    if table.len() < 3 {
        return;
    }
    let column_count = table[0].len();
    if column_count < 3 {
        return;
    }
    let first_non_empty: Vec<usize> = table[0]
        .iter()
        .enumerate()
        .filter(|(_, cell)| !cell.trim().is_empty())
        .map(|(index, _)| index)
        .collect();
    let second_non_empty_count = table[1].iter().filter(|cell| !cell.trim().is_empty()).count();
    if first_non_empty == [0] && second_non_empty_count >= 3 {
        table.remove(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a `HocrWord` with a fixed confidence, for tests that only care
    /// about position/text (matches the `word`/`make_word`/`hocr_word_at`
    /// helper convention used by the other files in this module's blast
    /// radius, e.g. `pdf::native::table` and `paddle_ocr::backend`).
    fn word(text: &str, left: u32, top: u32, width: u32, height: u32) -> HocrWord {
        HocrWord {
            text: text.to_string(),
            left,
            top,
            width,
            height,
            confidence: 95.0,
        }
    }

    #[test]
    fn median_of_takes_the_middle_of_an_odd_count() {
        assert_eq!(median_of(vec![50, 10, 30]), 30);
    }

    #[test]
    fn median_of_takes_the_upper_middle_of_an_even_count() {
        assert_eq!(median_of(vec![40, 10, 30, 20]), 30);
    }

    #[test]
    fn median_of_no_values_is_zero() {
        assert_eq!(median_of(Vec::new()), 0);
        assert_eq!(median_word_height(&[]), 0);
    }

    #[test]
    fn rtl_table_columns_and_cell_words_follow_logical_geometry() {
        let words = vec![
            word("כמות", 100, 0, 40, 20),
            word("קוד", 200, 0, 40, 20),
            word("מצב", 300, 0, 40, 20),
            word("42", 100, 60, 40, 20),
            word("REQ-2026-104", 200, 60, 80, 20),
            word("מוכן", 300, 60, 40, 20),
        ];

        let (mut table, mut positions) = reconstruct_table_with_columns(&words, 20, 0.5);
        orient_table_for_reading_order(&mut table, &mut positions, &words);

        assert_eq!(table[0], ["מצב", "קוד", "כמות"]);
        assert_eq!(table[1], ["מוכן", "REQ-2026-104", "42"]);
        assert!(positions.windows(2).all(|pair| pair[0] > pair[1]));

        let right = word("שלבי", 300, 120, 40, 20);
        let left = word("הבדיקה", 240, 122, 50, 20);
        let ordered = order_cell_words_in_reading_order(vec![&left, &right]);
        assert_eq!(
            ordered.iter().map(|word| word.text.as_str()).collect::<Vec<_>>(),
            ["שלבי", "הבדיקה"]
        );
    }

    #[test]
    fn numeric_and_ltr_tables_keep_left_to_right_geometry() {
        let words = vec![
            word("17", 100, 0, 40, 20),
            word("09", 200, 0, 40, 20),
            word("42", 300, 0, 40, 20),
            word("left", 100, 60, 40, 20),
            word("middle", 200, 60, 50, 20),
            word("right", 300, 60, 40, 20),
        ];

        let (mut table, mut positions) = reconstruct_table_with_columns(&words, 20, 0.5);
        orient_table_for_reading_order(&mut table, &mut positions, &words);

        assert_eq!(table[0], ["17", "09", "42"]);
        assert_eq!(table[1], ["left", "middle", "right"]);
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn code_heavy_rtl_table_ignores_long_latin_identifiers_for_base_direction() {
        let words = vec![
            word("https://example.test/a/very/long/resource-name", 100, 0, 180, 20),
            word("REQ-2026-104-ALPHA", 200, 60, 150, 20),
            word("الحالة", 400, 0, 60, 20),
        ];
        let mut table = vec![vec![
            "https://example.test/a/very/long/resource-name".to_string(),
            "REQ-2026-104-ALPHA".to_string(),
            "الحالة".to_string(),
        ]];
        let mut positions = vec![100, 200, 400];

        orient_table_for_reading_order(&mut table, &mut positions, &words);

        assert_eq!(table[0][0], "الحالة");
        assert_eq!(table[0][2], "https://example.test/a/very/long/resource-name");
        assert_eq!(positions, [400, 200, 100]);
    }

    #[test]
    fn test_detect_rows_zero_height_words_grouped_into_one_row() {
        let words = vec![
            HocrWord {
                text: "A".to_string(),
                left: 0,
                top: 10,
                width: 5,
                height: 0,
                confidence: 0.0,
            },
            HocrWord {
                text: "B".to_string(),
                left: 0,
                top: 10,
                width: 5,
                height: 0,
                confidence: 0.0,
            },
        ];
        let rows = detect_rows(&words, 0.5);
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_nan_safe_sort_does_not_panic() {
        let mut values: Vec<f64> = vec![1.0, f64::NAN, 2.0];
        values.sort_by(|a, b| a.total_cmp(b));
        assert_eq!(values.len(), 3);
        assert!(!values[0].is_nan());
        assert!(!values[1].is_nan());
        assert!(values[2].is_nan(), "NaN sorts last in ascending total_cmp order");
    }

    #[test]
    fn test_hocr_word_methods() {
        let word = HocrWord {
            text: "Hello".to_string(),
            left: 100,
            top: 50,
            width: 80,
            height: 30,
            confidence: 95.5,
        };

        assert_eq!(word.right(), 180);
        assert_eq!(word.bottom(), 80);
        assert_eq!(word.y_center(), 65.0);
        assert_eq!(word.x_center(), 140.0);
    }

    #[test]
    fn test_detect_columns() {
        let words = vec![
            HocrWord {
                text: "A".to_string(),
                left: 100,
                top: 50,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
            HocrWord {
                text: "B".to_string(),
                left: 300,
                top: 50,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
            HocrWord {
                text: "C".to_string(),
                left: 105,
                top: 100,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
            HocrWord {
                text: "D".to_string(),
                left: 295,
                top: 100,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
        ];

        let cols = detect_columns(&words, &detect_rows(&words, 0.5), 20);
        assert_eq!(cols.len(), 2);
    }

    #[test]
    fn test_detect_rows() {
        let words = vec![
            HocrWord {
                text: "A".to_string(),
                left: 100,
                top: 50,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
            HocrWord {
                text: "B".to_string(),
                left: 200,
                top: 52,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
            HocrWord {
                text: "C".to_string(),
                left: 100,
                top: 100,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
        ];

        let rows = detect_rows(&words, 0.5);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn detect_rows_checks_tall_overlap_against_the_order_independent_row_median() {
        let low_center = word("low", 0, 388, 20, 24);
        let median_a = word("median-a", 30, 399, 20, 24);
        let median_b = word("median-b", 60, 399, 20, 24);
        let tall = word("tall", 90, 413, 20, 50);
        let low_first = vec![low_center.clone(), median_a.clone(), median_b.clone(), tall.clone()];
        let median_first = vec![median_a, low_center, median_b, tall];

        assert_eq!(detect_rows(&low_first, 0.5), vec![411]);
        assert_eq!(detect_rows(&median_first, 0.5), vec![411]);
    }

    fn table_with_tall_words(tall_top: u32, tall_height: u32, tall_words_first: bool) -> Vec<HocrWord> {
        let mut words = Vec::new();
        let mut tall_words = Vec::new();

        for row in 0..12 {
            let top = row * 40;
            words.push(word(&format!("Item {row}"), 0, top, 50, 24));
            for column in 1..=6 {
                let amount = word(&format!("{}00", row * 10 + column), column * 100, top, 40, 24);
                if row == 10 && column >= 5 {
                    tall_words.push(word(&amount.text, amount.left, tall_top, amount.width, tall_height));
                } else {
                    words.push(amount);
                }
            }
        }

        if tall_words_first {
            tall_words.extend(words);
            tall_words
        } else {
            words.extend(tall_words);
            words
        }
    }

    #[test]
    fn reconstruct_table_keeps_downward_tall_words_in_their_row() {
        let words = table_with_tall_words(400, 50, false);

        let table = reconstruct_table(&words, 20, 0.5);

        assert_eq!(table.len(), 12);
        assert_eq!(table[10][5], "10500");
        assert_eq!(table[10][6], "10600");
    }

    #[test]
    fn reconstruct_table_keeps_upward_tall_word_in_its_row_when_seen_first() {
        let words = table_with_tall_words(370, 54, true);

        let table = reconstruct_table(&words, 20, 0.5);

        assert_eq!(table.len(), 12);
        assert_eq!(table[10][5], "10500");
        assert_eq!(table[10][6], "10600");
    }

    #[test]
    fn reconstruct_table_keeps_a_standalone_tall_header_row() {
        let words = vec![
            word("Heading", 0, 0, 50, 50),
            word("Total", 100, 0, 40, 50),
            word("First", 0, 100, 50, 24),
            word("100", 100, 100, 40, 24),
            word("Second", 0, 140, 50, 24),
            word("200", 100, 140, 40, 24),
        ];

        let table = reconstruct_table(&words, 20, 0.5);

        assert_eq!(table.len(), 3);
        assert_eq!(table[0], ["Heading", "Total"]);
    }

    #[test]
    fn test_reconstruct_table_basic() {
        let words = vec![
            HocrWord {
                text: "Name".to_string(),
                left: 100,
                top: 50,
                width: 40,
                height: 20,
                confidence: 95.0,
            },
            HocrWord {
                text: "Value".to_string(),
                left: 300,
                top: 50,
                width: 40,
                height: 20,
                confidence: 95.0,
            },
            HocrWord {
                text: "Alice".to_string(),
                left: 100,
                top: 100,
                width: 40,
                height: 20,
                confidence: 95.0,
            },
            HocrWord {
                text: "42".to_string(),
                left: 300,
                top: 100,
                width: 20,
                height: 20,
                confidence: 95.0,
            },
        ];

        let table = reconstruct_table(&words, 20, 0.5);
        assert_eq!(table.len(), 2);
        assert_eq!(table[0].len(), 2);
        assert_eq!(table[0][0], "Name");
        assert_eq!(table[0][1], "Value");
        assert_eq!(table[1][0], "Alice");
        assert_eq!(table[1][1], "42");
    }

    #[test]
    fn test_table_to_markdown_basic() {
        let table = vec![
            vec!["Name".to_string(), "Value".to_string()],
            vec!["Alice".to_string(), "42".to_string()],
        ];

        let md = table_to_markdown(&table);
        assert!(md.contains("| Name | Value |"));
        assert!(md.contains("| --- | --- |"));
        assert!(md.contains("| Alice | 42 |"));
    }

    #[test]
    fn test_table_to_markdown_empty() {
        assert_eq!(table_to_markdown(&[]), String::new());
    }

    #[test]
    fn test_table_to_markdown_escapes_pipes() {
        let table = vec![vec!["Header".to_string()], vec!["a|b".to_string()]];

        let md = table_to_markdown(&table);
        assert!(md.contains("a\\|b"));
    }

    /// Regression test for issue where intra-cell word spacing ("Chose 1")
    /// was incorrectly split into separate columns.
    /// The word "1" should stay in the same cell as "Chose" despite having
    /// a different left position, because they're separated by a small gap.
    #[test]
    fn test_reconstruct_table_intra_cell_word_spacing() {
        let words = vec![
            HocrWord {
                text: "Chose".to_string(),
                left: 57,
                top: 496,
                width: 30,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "Truc".to_string(),
                left: 306,
                top: 496,
                width: 23,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "Chose".to_string(),
                left: 57,
                top: 510,
                width: 28,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "1".to_string(),
                left: 90,
                top: 510,
                width: 6,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "Truc".to_string(),
                left: 306,
                top: 510,
                width: 21,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "1".to_string(),
                left: 332,
                top: 510,
                width: 5,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "Chose".to_string(),
                left: 57,
                top: 524,
                width: 28,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "2".to_string(),
                left: 90,
                top: 524,
                width: 6,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "Truc".to_string(),
                left: 306,
                top: 524,
                width: 21,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "2".to_string(),
                left: 332,
                top: 524,
                width: 5,
                height: 12,
                confidence: 95.0,
            },
        ];

        let table = reconstruct_table(&words, 60, 0.5);

        assert_eq!(table.len(), 3, "Expected 3 rows, got {}", table.len());
        assert_eq!(table[0].len(), 2, "Expected 2 columns in row 0, got {}", table[0].len());

        assert_eq!(table[0][0], "Chose", "Header row 1, col 1");
        assert_eq!(table[0][1], "Truc", "Header row 1, col 2");
        assert_eq!(table[1][0], "Chose 1", "Row 2, col 1 should contain merged text");
        assert_eq!(table[1][1], "Truc 1", "Row 2, col 2 should contain merged text");
        assert_eq!(table[2][0], "Chose 2", "Row 3, col 1 should contain merged text");
        assert_eq!(table[2][1], "Truc 2", "Row 3, col 2 should contain merged text");
    }

    /// Regression/verification test for the claim in xberg-io/xberg#183 that
    /// `reconstruct_table` "drops any word for which `find_row_index` or
    /// `find_column_index` returns `None`".
    ///
    /// That claim does not hold for the current implementation: both
    /// `find_row_index` and `find_column_index` resolve to the *nearest*
    /// row/column via `min_by_key`, which is `Some` for every word whenever
    /// `row_positions`/`col_positions` are non-empty — and `reconstruct_table`
    /// already early-returns before this loop if either is empty. There is no
    /// path through this function that silently discards a word; even a word
    /// wildly outside the detected column/row bands is force-assigned to its
    /// nearest band instead of being dropped. This test proves that with an
    /// outlier word far outside the main table extent: every input word is
    /// still present, exactly once, in the reconstructed table.
    #[test]
    fn test_reconstruct_table_never_drops_words_including_far_outliers() {
        let mut words = vec![
            HocrWord {
                text: "A1".to_string(),
                left: 0,
                top: 0,
                width: 10,
                height: 10,
                confidence: 95.0,
            },
            HocrWord {
                text: "B1".to_string(),
                left: 200,
                top: 0,
                width: 10,
                height: 10,
                confidence: 95.0,
            },
            HocrWord {
                text: "A2".to_string(),
                left: 0,
                top: 200,
                width: 10,
                height: 10,
                confidence: 95.0,
            },
            HocrWord {
                text: "B2".to_string(),
                left: 200,
                top: 200,
                width: 10,
                height: 10,
                confidence: 95.0,
            },
        ];
        // Far outside every detected row/column band and every other word's
        // neighborhood — the scenario #183 claims gets silently dropped.
        words.push(HocrWord {
            text: "Outlier".to_string(),
            left: 50_000,
            top: 50_000,
            width: 10,
            height: 10,
            confidence: 95.0,
        });

        let input_word_count = words.len();
        let table = reconstruct_table(&words, 20, 0.5);

        let output_word_count: usize = table
            .iter()
            .flat_map(|row| row.iter())
            .flat_map(|cell| cell.split_whitespace())
            .count();

        assert_eq!(
            output_word_count, input_word_count,
            "every input word (including the far outlier) must appear exactly once in the output; \
             reconstruct_table's nearest-row/nearest-column assignment never returns None here"
        );

        let all_text: Vec<&str> = table
            .iter()
            .flat_map(|row| row.iter())
            .flat_map(|c| c.split_whitespace())
            .collect();
        assert!(
            all_text.contains(&"Outlier"),
            "the outlier word must not be silently dropped"
        );
    }

    /// Regression test for xberg-io/xberg#688: a table cell containing more
    /// than one word ("Alice Smith") must not mint a spurious extra column
    /// from its second word's x-position.
    ///
    /// TEST HONESTY: without the merge-before-column-detection fix, this
    /// fails at the `table[0].len() == 2` assertion — `detect_columns` sees
    /// "Alice"@100 and "Smith"@145 as two distinct groups (gap 45 > the
    /// column_threshold of 20), producing 3 columns instead of 2. `table[0]`
    /// comes out as `["Name", "", "Value"]` (len 3) instead of `["Name",
    /// "Value"]`.
    #[test]
    fn test_reconstruct_table_multiword_cell_no_spurious_column() {
        let words = vec![
            word("Name", 100, 50, 40, 20),
            word("Value", 300, 50, 40, 20),
            word("Alice", 100, 100, 40, 20),
            word("Smith", 145, 100, 30, 20),
            word("42", 300, 100, 20, 20),
        ];

        let table = reconstruct_table(&words, 20, 0.5);

        assert_eq!(table.len(), 2);
        assert_eq!(
            table[0].len(),
            2,
            "expected 2 columns; without the fix 'Smith' (x=145) mints a spurious 3rd column"
        );
        assert_eq!(table[0], vec!["Name".to_string(), "Value".to_string()]);
        assert_eq!(table[1], vec!["Alice Smith".to_string(), "42".to_string()]);
    }

    /// xberg-io/xberg#1934: column detection treats a close two-word header as one cell token.
    /// Assignment must keep that token together when its first word anchors the supported data
    /// column, even if the second word is geometrically closer to the next column.
    ///
    /// TEST HONESTY: assigning header words independently produces `["Code", "Amount", "Unit",
    /// "cost Total"]`; the second word leaves the cell token and joins the next header. ~keep
    #[test]
    fn issue_1934_two_word_header_stays_in_its_supported_column() {
        let words = vec![
            word("Code", 0, 0, 50, 20),
            word("Amount", 100, 0, 70, 20),
            word("Unit", 200, 0, 70, 20),
            word("cost", 280, 0, 45, 20),
            word("Total", 350, 0, 60, 20),
            word("A", 0, 60, 20, 20),
            word("10", 100, 60, 30, 20),
            word("2.50", 200, 60, 50, 20),
            word("25.00", 350, 60, 60, 20),
            word("B", 0, 120, 20, 20),
            word("20", 100, 120, 30, 20),
            word("3.00", 200, 120, 50, 20),
            word("60.00", 350, 120, 60, 20),
        ];

        let (table, column_positions) = reconstruct_table_with_columns(&words, 50, 0.5);

        assert_eq!(table[0], vec!["Code", "Amount", "Unit cost", "Total"]);
        assert_eq!(table[1], vec!["A", "10", "2.50", "25.00"]);
        assert_eq!(column_positions, vec![0, 100, 200, 350]);
    }

    #[test]
    fn reconstruct_table_attaches_header_fragments_to_nearest_supported_column() {
        let words = vec![
            word("DESCRIPTION", 279, 100, 250, 40),
            word("QTY", 1_699, 100, 90, 40),
            word("UNIT", 2_096, 100, 105, 40),
            word("PRICE", 2_219, 100, 135, 40),
            word("LINE", 2_663, 100, 98, 40),
            word("TOTAL", 2_778, 100, 143, 40),
            word("Espresso Beans", 279, 200, 400, 40),
            word("10", 1_740, 200, 48, 40),
            word("$45.00", 2_206, 200, 150, 40),
            word("$450.00", 2_744, 200, 175, 40),
            word("Cups", 279, 300, 100, 40),
            word("200", 1_709, 300, 75, 40),
            word("$1.20", 2_233, 300, 125, 40),
            word("$240.00", 2_744, 300, 175, 40),
            word("Cleaning Tablets", 279, 400, 360, 40),
            word("$18.50", 2_206, 400, 150, 40),
            word("$92.50", 2_772, 400, 150, 40),
        ];

        let (table, column_positions) = reconstruct_table_with_columns(&words, 50, 0.5);

        assert_eq!(table[0], vec!["DESCRIPTION", "QTY", "UNIT PRICE", "LINE TOTAL"]);
        assert_eq!(table[3], vec!["Cleaning Tablets", "", "$18.50", "$92.50"]);
        // The merged column keeps `detect_columns`' own anchor for the data cluster (2_206, the
        // median of the "$45.00"/"$1.20"/"$18.50" lefts), not the PRICE header word's own left
        // (2_219). `assign_words_to_cells` already used that anchor -- not the header word's
        // position -- to decide every word's column membership, and downstream geometry (the OCR
        // blank-quantity retry's `cell_axis_bounds`) derives an adjacent cell's crop boundary from
        // this same value, so it must track where the data actually sits, not where the header text
        // happened to land. ~keep
        assert_eq!(column_positions, vec![279, 1_709, 2_206, 2_744]);
    }

    #[test]
    fn reconstruct_table_keeps_fragment_far_from_matching_right_column() {
        let words = vec![
            word("DESCRIPTION", 100, 100, 250, 40),
            word("QTY", 1_000, 100, 90, 40),
            word("UNIT", 1_150, 100, 105, 40),
            word("PRICE", 1_500, 100, 135, 40),
            word("Item A", 100, 200, 160, 40),
            word("10", 1_000, 200, 48, 40),
            word("$45.00", 1_500, 200, 150, 40),
            word("Item B", 100, 300, 160, 40),
            word("20", 1_000, 300, 48, 40),
            word("$50.00", 1_500, 300, 150, 40),
        ];

        let (table, column_positions) = reconstruct_table_with_columns(&words, 50, 0.5);

        assert_eq!(table[0], vec!["DESCRIPTION", "QTY", "UNIT", "PRICE"]);
        assert_eq!(column_positions, vec![100, 1_000, 1_150, 1_500]);
    }

    #[test]
    fn reconstruct_table_keeps_far_standalone_header_column() {
        let words = vec![
            word("QTY", 0, 100, 80, 40),
            word("UNIT", 400, 100, 120, 40),
            word("PRICE", 1_000, 100, 100, 40),
            word("1", 0, 200, 30, 40),
            word("$10", 1_000, 200, 60, 40),
            word("2", 0, 300, 30, 40),
            word("$20", 1_000, 300, 60, 40),
        ];

        let (table, column_positions) = reconstruct_table_with_columns(&words, 50, 0.5);

        assert_eq!(table[0], vec!["QTY", "UNIT", "PRICE"]);
        assert_eq!(column_positions, vec![0, 400, 1_000]);
    }

    #[test]
    fn reconstruct_table_keeps_centered_group_header_column() {
        let words = vec![
            word("QTY", 0, 100, 80, 40),
            word("UNIT", 500, 100, 120, 40),
            word("PRICE", 1_000, 100, 100, 40),
            word("1", 0, 200, 30, 40),
            word("$10", 1_000, 200, 60, 40),
            word("2", 0, 300, 30, 40),
            word("$20", 1_000, 300, 60, 40),
        ];

        let (table, column_positions) = reconstruct_table_with_columns(&words, 50, 0.5);

        assert_eq!(table[0], vec!["QTY", "UNIT", "PRICE"]);
        assert_eq!(column_positions, vec![0, 500, 1_000]);
    }

    #[test]
    fn reconstruct_table_keeps_non_header_singleton_column() {
        let words = vec![
            word("QTY", 0, 100, 80, 40),
            word("note", 700, 100, 80, 40),
            word("UNIT", 850, 100, 80, 40),
            word("PRICE", 1_000, 100, 100, 40),
            word("1", 0, 200, 30, 40),
            word("$10", 1_000, 200, 60, 40),
            word("2", 0, 300, 30, 40),
            word("$20", 1_000, 300, 60, 40),
        ];

        let (table, column_positions) = reconstruct_table_with_columns(&words, 50, 0.5);

        assert_eq!(table[0], vec!["QTY", "note", "UNIT PRICE"]);
        assert_eq!(column_positions, vec![0, 700, 1_000]);
    }

    #[test]
    fn reconstruct_table_keeps_unrelated_uppercase_header_column() {
        let words = vec![
            word("QTY", 0, 100, 80, 40),
            word("DISCOUNT", 700, 100, 120, 40),
            word("UNIT", 850, 100, 80, 40),
            word("PRICE", 1_000, 100, 100, 40),
            word("1", 0, 200, 30, 40),
            word("$10", 1_000, 200, 60, 40),
            word("2", 0, 300, 30, 40),
            word("$20", 1_000, 300, 60, 40),
        ];

        let (table, column_positions) = reconstruct_table_with_columns(&words, 50, 0.5);

        assert_eq!(table[0], vec!["QTY", "DISCOUNT", "UNIT PRICE"]);
        assert_eq!(column_positions, vec![0, 700, 1_000]);
    }

    #[test]
    fn reconstruct_table_keeps_every_supported_column() {
        let words = vec![
            word("A", 0, 100, 40, 40),
            word("B", 500, 100, 40, 40),
            word("C", 1_000, 100, 40, 40),
            word("1", 0, 200, 30, 40),
            word("2", 500, 200, 30, 40),
            word("3", 1_000, 200, 30, 40),
            word("4", 0, 300, 30, 40),
            word("5", 500, 300, 30, 40),
            word("6", 1_000, 300, 30, 40),
        ];

        let (table, column_positions) = reconstruct_table_with_columns(&words, 50, 0.5);

        assert_eq!(table[0], vec!["A", "B", "C"]);
        assert_eq!(column_positions, vec![0, 500, 1_000]);
    }

    /// A genuinely separate, closely-spaced column pair ("Wid"/"Zone" at
    /// x=150 and x=180, gap 15px) must survive intact even once a multi-word
    /// cell elsewhere in the same table ("Foo"+"Bar", gap 5px) gets merged.
    ///
    /// TEST HONESTY: without the fix, "Bar" (x=25) mints its own spurious
    /// column, producing 4 columns total ([0, 25, 150, 180] once sorted).
    /// `table[0]` comes out as `["Name", "", "Wid", "Zone"]` (len 4) instead
    /// of `["Name", "Wid", "Zone"]` (len 3), and `table[1]` as `["Foo",
    /// "Bar", "Val1", "Val2"]` instead of `["Foo Bar", "Val1", "Val2"]`.
    #[test]
    fn test_reconstruct_table_close_but_distinct_columns_not_merged() {
        let words = vec![
            word("Name", 0, 0, 20, 20),
            word("Wid", 150, 0, 15, 20),
            word("Zone", 180, 0, 20, 20),
            word("Foo", 0, 40, 20, 20),
            word("Bar", 25, 40, 15, 20),
            word("Val1", 150, 40, 15, 20),
            word("Val2", 180, 40, 20, 20),
        ];

        let table = reconstruct_table(&words, 10, 0.5);

        assert_eq!(table.len(), 2);
        assert_eq!(
            table[0].len(),
            3,
            "Wid/Zone must stay 2 distinct columns, not collapsed by the Foo/Bar merge fix"
        );
        assert_eq!(
            table[0],
            vec!["Name".to_string(), "Wid".to_string(), "Zone".to_string()]
        );
        assert_eq!(
            table[1],
            vec!["Foo Bar".to_string(), "Val1".to_string(), "Val2".to_string()],
            "Foo+Bar merge into one cell; Val1 and Val2 remain distinct cells, not merged together"
        );
    }

    /// Column count must be stable across different `column_threshold`
    /// values once multi-word cells are merged before column detection.
    ///
    /// TEST HONESTY: without the fix, threshold 10 yields 4 columns
    /// ([0, 15, 30, 300], "A"/"B"/"C" each minting a separate column) while
    /// threshold 30 over-merges the spurious columns into the real ones,
    /// yielding only 2 columns ([15, 300]) — i.e. 4 != 2, so the equality
    /// assertion below fails on unfixed code even though 2 happens to match
    /// one of the two unfixed outputs by coincidence.
    #[test]
    fn test_reconstruct_table_column_count_stable_across_thresholds() {
        let words = vec![
            word("H1", 0, 0, 20, 20),
            word("H2", 300, 0, 30, 20),
            word("A", 0, 40, 10, 20),
            word("B", 15, 40, 10, 20),
            word("C", 30, 40, 10, 20),
            word("Val", 300, 40, 20, 20),
        ];

        let table_tight = reconstruct_table(&words, 10, 0.5);
        let table_loose = reconstruct_table(&words, 30, 0.5);

        assert_eq!(table_tight[0].len(), 2);
        assert_eq!(
            table_tight[0].len(),
            table_loose[0].len(),
            "column count must not depend on column_threshold once cells are pre-merged"
        );
        assert_eq!(table_tight, table_loose);
        assert_eq!(table_tight[0], vec!["H1".to_string(), "H2".to_string()]);
        assert_eq!(table_tight[1], vec!["A B C".to_string(), "Val".to_string()]);
    }

    /// Degenerate-input guard: empty input. This early return predates the
    /// fix and behaves identically before and after it — included for
    /// completeness of degenerate-input coverage, not as a fix-detecting
    /// regression test (there is nothing for the fix to change here).
    #[test]
    fn test_reconstruct_table_empty_words_returns_empty_table() {
        let table = reconstruct_table(&[], 20, 0.5);
        assert!(table.is_empty());
    }

    /// Degenerate-input guard: a single word can never trigger the merge
    /// step (nothing to merge with), so this is identical before and after
    /// the fix. It exists to prove `group_words_into_cell_tokens`'s
    /// `words.len() <= 1` guard doesn't panic or drop the word.
    #[test]
    fn test_reconstruct_table_single_word_no_panic() {
        let words = vec![word("Solo", 10, 10, 20, 20)];
        let table = reconstruct_table(&words, 20, 0.5);
        assert_eq!(table, vec![vec!["Solo".to_string()]]);
    }

    /// Degenerate-input guard: zero-height words drive the merge-gap
    /// threshold to exactly 0 (`0.6 * 0`), so only touching/overlapping
    /// words (gap <= 0) merge, and the computation must not panic (no
    /// division by a zero median height, no overflow in the saturating
    /// bbox-union arithmetic).
    ///
    /// TEST HONESTY: without the fix, "X" (x=0..10) and "Y" (x=10..20) —
    /// which touch with a zero-pixel gap — are still 2 separate columns
    /// under raw `detect_columns` (their left edges differ by 10, over the
    /// column_threshold of 5), giving 3 columns (`["X", "Y", "Z"]`) instead
    /// of the correct 2 (`["X Y", "Z"]`).
    #[test]
    fn test_reconstruct_table_zero_height_words_merge_on_touching_gap() {
        let words = vec![word("X", 0, 0, 10, 0), word("Y", 10, 0, 10, 0), word("Z", 50, 0, 10, 0)];

        let table = reconstruct_table(&words, 5, 0.5);

        assert_eq!(table.len(), 1);
        assert_eq!(
            table[0].len(),
            2,
            "X and Y touch with a 0px gap and must merge into one cell"
        );
        assert_eq!(table[0], vec!["X Y".to_string(), "Z".to_string()]);
    }

    #[test]
    fn issue_2122_should_join_overlapping_numeric_runs_after_column_detection() {
        for (prefix, suffix, expected) in [
            ("59", ".804", "59.804"),
            ("59", ".804 ", "59.804"),
            ("59", ",804", "59,804"),
            ("(7", ".812)", "(7.812)"),
            ("1.234", ".567", "1.234.567"),
        ] {
            let words = vec![word(prefix, 100, 100, 10, 8), word(suffix, 109, 100, 20, 8)];
            let table = assign_words_to_cells(&words, &[100], &[100]);
            assert_eq!(table, vec![vec![expected.to_string()]]);
        }
    }

    #[test]
    fn issue_2122_should_preserve_adjacent_financial_columns_when_joining_numbers() {
        let mut words = Vec::new();
        for row in 0..7 {
            for column in 0..9 {
                let left = 100 + column * 40;
                let top = 100 + row * 12;
                words.push(word("59", left, top, 10, 8));
                words.push(word(".804", left + 9, top, 20, 8));
            }
        }
        assert_eq!(words.len(), 126);
        let (table, positions) = reconstruct_table_with_columns(&words, 20, 0.5);
        assert_eq!(positions, (0..9).map(|column| 100 + column * 40).collect::<Vec<_>>());
        assert_eq!(table, vec![vec!["59.804".to_string(); 9]; 7]);
    }

    #[test]
    fn issue_2122_should_preserve_real_spaces_and_non_numeric_boundaries() {
        for (prefix, suffix, left, top, expected) in [
            ("59 ", ".804", 109, 100, "59 .804"),
            ("59", " .804", 109, 100, "59 .804"),
            ("59", ".804", 114, 100, "59 .804"),
            ("59", ".804", 109, 108, "59 .804"),
            ("Item", ".804", 109, 100, "Item .804"),
            ("59", ".note", 109, 100, "59 .note"),
            ("59", "804", 109, 100, "59 804"),
        ] {
            let words = [word(prefix, 100, 100, 10, 8), word(suffix, left, top, 20, 8)];
            let table = finish_cell_assignment(vec![vec![words.iter().collect()]]);
            assert_eq!(table, vec![vec![expected.to_string()]]);
        }
    }

    /// GH#1628: a cell's words must read left-to-right, not in the order they happened to
    /// arrive in `words`.
    ///
    /// A sub/superscript is drawn as its own content-stream segment sitting a fraction of a point
    /// below the line it annotates, so every reading-order sort upstream of here places it after
    /// the whole line rather than beside the symbol it belongs to. By the time words reach this
    /// function the subscript's `left` still says exactly where it goes --- `eta`(206),
    /// `S`(211), `%`(253) --- but slice order says `eta`, `%`, `S`, and the join followed slice
    /// order. The reported output was `E % S` for `eta_S %` and `Q GJ HE` for `Q_HE GJ`.
    ///
    /// Left ordering is the correct rule here regardless of subscripts: every word in one cell
    /// sits in the same row band by construction (`find_row_index` put it there), so `left` is
    /// reading order for that cell.
    #[test]
    fn issue_1628_cell_words_join_in_left_to_right_order() {
        let words = vec![
            word("eta", 206, 100, 7, 12),
            word("%", 253, 100, 6, 12),
            word("S", 211, 104, 3, 7),
        ];

        let table = assign_words_to_cells(&words, &[100], &[206]);

        assert_eq!(
            table,
            vec![vec!["eta S %".to_string()]],
            "a subscript must join between the symbol it annotates and the next unit, not after it"
        );
    }

    /// GH#1628 negative control: a cell holding two visual lines must NOT interleave them.
    ///
    /// This is the failure mode a naive left-only sort introduces, and it is strictly worse than
    /// the bug being fixed: it scrambles every wrapped cell in the corpus, not just cells with
    /// sub/superscripts. Line grouping is what separates the two cases --- a subscript sits a
    /// fraction of its own height off its base's centre, a second line sits a full line height
    /// away.
    #[test]
    fn issue_1628_cell_with_two_visual_lines_does_not_interleave_them() {
        let words = vec![
            word("Hello", 100, 100, 30, 12),
            word("world", 140, 100, 30, 12),
            word("again", 100, 116, 30, 12),
            word("here", 140, 116, 30, 12),
        ];

        let table = assign_words_to_cells(&words, &[106], &[100]);

        assert_eq!(table, vec![vec!["Hello world again here".to_string()]]);
    }

    /// GH#1628: a segment whose text holds no whitespace is returned unsplit, so its word keeps
    /// the segment's own trailing space. Joining those words then stacks that space on the
    /// separator and renders `Q_HE GJ` as `Q  HE GJ`. Each word's own whitespace must collapse so
    /// the join alone owns the spacing.
    #[test]
    fn issue_1628_cell_join_does_not_stack_a_word_s_own_trailing_space() {
        let words = vec![word("Q ", 206, 100, 8, 12), word("HE", 213, 104, 7, 7)];

        let table = assign_words_to_cells(&words, &[100], &[206]);

        assert_eq!(table, vec![vec!["Q HE".to_string()]]);
    }

    /// Degenerate-input guard: every word on one row is a degenerate case
    /// for the per-row bucketing inside `group_words_into_cell_tokens` (a
    /// single bucket holding every word) — must not panic and must still
    /// merge/split correctly within that one row.
    ///
    /// TEST HONESTY: without the fix, "A" (x=0) and "B" (x=15) are 2
    /// separate columns under raw `detect_columns` (gap 15 > the
    /// column_threshold of 5), giving 3 columns (`["A", "B", "C"]`) instead
    /// of the correct 2 (`["A B", "C"]`).
    #[test]
    fn test_reconstruct_table_all_words_one_row() {
        let words = vec![
            word("A", 0, 0, 10, 10),
            word("B", 15, 0, 10, 10),
            word("C", 100, 0, 10, 10),
        ];

        let table = reconstruct_table(&words, 5, 0.5);

        assert_eq!(table.len(), 1);
        assert_eq!(table[0].len(), 2);
        assert_eq!(table[0], vec!["A B".to_string(), "C".to_string()]);
    }

    /// `reconstruct_table_with_columns` must return column positions that
    /// stay index-correlated with the returned grid: `positions[i]`
    /// corresponds to `grid[row][i]` for every row. Callers like
    /// `pdf::native::table::reconstruct_region_table_with_column_gap` rely on
    /// this to index `column_positions[column]` against `grid[row][column]`
    /// (xberg-io/xberg#688 blast-radius fix: `reconstruct_table` now detects
    /// columns from post-merge cell tokens, so a caller's own separate
    /// `detect_columns(words, ...)` call no longer corresponds to the grid
    /// `reconstruct_table` returns — this sibling function is how such a
    /// caller gets a mutually consistent pair instead).
    ///
    /// Note: row/column assignment always resolves to the nearest position
    /// (xberg-io/xberg#183), and every column detect_columns reports is the
    /// exact position of one of its member tokens, so that token's word
    /// always maps back to its own column at distance 0. In practice this
    /// means a detect_columns-derived column can never end up entirely
    /// empty across all rows, so remove_empty_rows_and_columns cannot
    /// actually strip a column reachable through this path. This test
    /// therefore checks the correspondence invariant directly rather than
    /// depending on triggering that specific removal branch.
    #[test]
    fn test_reconstruct_table_with_columns_positions_correlate_with_grid() {
        let words = vec![
            word("Left", 0, 0, 20, 20),
            word("Right", 300, 0, 20, 20),
            word("L2", 0, 40, 20, 20),
            word("R2", 300, 40, 20, 20),
        ];
        let (grid, positions) = reconstruct_table_with_columns(&words, 20, 0.5);

        assert!(!grid.is_empty());
        assert_eq!(
            positions.len(),
            grid[0].len(),
            "column_positions must have exactly one entry per surviving grid column"
        );
        assert_eq!(positions, vec![0, 300]);
        assert_eq!(grid[0], vec!["Left".to_string(), "Right".to_string()]);
        assert_eq!(grid[1], vec!["L2".to_string(), "R2".to_string()]);
    }

    /// xberg-io/xberg#1649: a wide multi-word cell's trailing word must stay in its own cell's
    /// column even when its raw x-position sits numerically closer to a neighboring column's
    /// anchor. Mirrors the reported fixture's "ACH Deposit - EMPLOYER INC" description cell
    /// sitting immediately left of a narrow "WITHDRAWAL" amount column.
    #[test]
    fn issue_1649_wide_description_word_stays_in_its_own_column_not_the_nearest_anchor() {
        // Coordinates measured on the reported fixture's transaction table (xberg-io/xberg#1649):
        // the header row, and the "ACH Deposit - EMPLOYER INC" data row whose trailing word
        // ("INC") sits closer by raw x-distance to the WITHDRAWAL column anchor (2394) than to
        // the DESCRIPTION column anchor (746).
        let words = vec![
            word("DATE", 126, 1292, 152, 43),
            word("DESCRIPTION", 746, 1292, 412, 44),
            word("WITHDRAWAL", 2394, 1292, 418, 43),
            word("2026-01-05", 125, 1673, 350, 49),
            word("ACH", 741, 1671, 139, 52),
            word("Deposit", 908, 1672, 228, 63),
            word("-", 1159, 1699, 20, 7),
            word("EMPLOYER", 1206, 1671, 360, 52),
            word("INC", 1591, 1671, 108, 52),
        ];

        let table = reconstruct_table(&words, 50, 0.5);

        assert_eq!(table[0], vec!["DATE", "DESCRIPTION", "WITHDRAWAL"]);
        assert_eq!(
            table[1],
            vec!["2026-01-05", "ACH Deposit - EMPLOYER INC", ""],
            "INC must join the description cell it visually belongs to, not the far column its \
             own x-position happens to be nearest to"
        );
    }

    /// xberg-io/xberg#1649: a right-aligned amount column split into two x-position buckets by
    /// digit-width drift (one header-only, one holding every value) must be folded back into one
    /// logical column.
    #[cfg(feature = "ocr")]
    #[test]
    fn issue_1649_merge_disjoint_numeric_columns_folds_a_drift_split_column() {
        let mut table = vec![
            vec!["WITHDRAWAL".to_string(), "".to_string(), "DEPOSIT".to_string()],
            vec!["".to_string(), "$1,250.00".to_string(), "".to_string()],
            vec!["".to_string(), "$87.32".to_string(), "".to_string()],
            vec!["".to_string(), "".to_string(), "$500.00".to_string()],
        ];
        let mut positions = vec![100_u32, 120, 600];

        merge_disjoint_numeric_columns(&mut table, &mut positions, 50);

        assert_eq!(positions.len(), 2, "the drift-split pair must fold into one column");
        assert_eq!(table[0], vec!["WITHDRAWAL".to_string(), "DEPOSIT".to_string()]);
        assert_eq!(table[1], vec!["$1,250.00".to_string(), "".to_string()]);
        assert_eq!(table[2], vec!["$87.32".to_string(), "".to_string()]);
        assert_eq!(table[3], vec!["".to_string(), "$500.00".to_string()]);
    }

    /// Negative control for xberg-io/xberg#1649's fix: two independently-labeled, mutually
    /// exclusive numeric columns (a ledger's "Debit" and "Credit") must NOT be folded together
    /// just because they sit close together and never both hold a value on the same row --
    /// that shape is an ordinary compact ledger layout, not a drift-split artifact, and each
    /// column here carries its own header text.
    #[cfg(feature = "ocr")]
    #[test]
    fn issue_1649_merge_disjoint_numeric_columns_leaves_independently_headed_columns_alone() {
        let mut table = vec![
            vec!["Debit".to_string(), "Credit".to_string()],
            vec!["$1,250.00".to_string(), "".to_string()],
            vec!["".to_string(), "$87.32".to_string()],
        ];
        let mut positions = vec![100_u32, 120];

        merge_disjoint_numeric_columns(&mut table, &mut positions, 50);

        assert_eq!(
            positions.len(),
            2,
            "two independently headed columns must not be merged"
        );
        assert_eq!(table[0], vec!["Debit".to_string(), "Credit".to_string()]);
    }

    /// xberg-io/xberg#1649: a section caption ("TRANSACTIONS") that shares a table region with
    /// the real header row must be dropped, leaving the header as row 0.
    #[cfg(feature = "ocr")]
    #[test]
    fn issue_1649_drop_leading_caption_row_removes_a_lone_leading_caption() {
        let mut table = vec![
            vec!["TRANSACTIONS".to_string(), "".to_string(), "".to_string()],
            vec!["DATE".to_string(), "DESCRIPTION".to_string(), "BALANCE".to_string()],
            vec![
                "2026-01-02".to_string(),
                "Opening Balance".to_string(),
                "$10,500.00".to_string(),
            ],
        ];

        drop_leading_caption_row(&mut table);

        assert_eq!(table.len(), 2, "the caption row must be dropped");
        assert_eq!(table[0], vec!["DATE", "DESCRIPTION", "BALANCE"]);
    }

    /// Negative control: a genuine two-row summary card (header + one data row) must never lose
    /// its header row -- there is no separate caption to drop here, only a legitimate header.
    #[cfg(feature = "ocr")]
    #[test]
    fn issue_1649_drop_leading_caption_row_leaves_a_two_row_summary_card_alone() {
        let mut table = vec![
            vec![
                "ACCOUNT TYPE".to_string(),
                "STATEMENT PERIOD".to_string(),
                "CLOSING BALANCE".to_string(),
            ],
            vec![
                "Checking".to_string(),
                "Jan 1 - Jan 31, 2026".to_string(),
                "$12,847.65".to_string(),
            ],
        ];

        drop_leading_caption_row(&mut table);

        assert_eq!(table.len(), 2, "a two-row table has no caption to drop");
        assert_eq!(table[0][0], "ACCOUNT TYPE");
    }

    #[cfg(feature = "ocr")]
    fn grid(rows: &[&[&str]]) -> Vec<Vec<String>> {
        rows.iter()
            .map(|row| row.iter().map(|cell| (*cell).to_string()).collect())
            .collect()
    }

    /// A right-aligned amount column that drift split into two tracks, where OCR also read the
    /// page's rules as cells of their own (`-`, a dash run, `:`) in the right track. A cell with
    /// no letter or digit is not a value, so the pair still merges: the real amount wins where a
    /// row has both, and a lone mark survives where it is the row's only content.
    #[cfg(feature = "ocr")]
    #[test]
    fn split_amount_column_with_rule_marks_is_merged() {
        let mut table = grid(&[
            &["Item", "Column A", "Column B", ""],
            &["Item 1", "1,250", "", "310"],
            &["Item 2", "", "87", "-"],
            &["Item 3", "940", "", "——————"],
            &["Item 4", "", "4,020", ""],
            &["Item 5", "", "", ":"],
        ]);
        let mut positions = vec![100_u32, 400, 700, 740];

        merge_disjoint_numeric_columns(&mut table, &mut positions, 25);

        assert_eq!(positions.len(), 3, "the split amount column must fold into one column");
        assert_eq!(
            table,
            grid(&[
                &["Item", "Column A", "Column B"],
                &["Item 1", "1,250", "310"],
                &["Item 2", "", "87"],
                &["Item 3", "940", "——————"],
                &["Item 4", "", "4,020"],
                &["Item 5", "", ":"],
            ])
        );
    }

    /// The header band above a scanned table can hold rule marks (`~`) and a stray glyph on a row
    /// of its own. A mark with no letter or digit is not a column label, and the stray row sits
    /// above the first amount, so it is header, not data: the split amount column still merges.
    #[cfg(feature = "ocr")]
    #[test]
    fn rule_marks_in_the_header_band_do_not_block_a_split_column_merge() {
        let mut table = grid(&[
            &["Item", "Column B", "~"],
            &["", "a", ""],
            &["Item 1", "1,250", ""],
            &["Item 2", "", "87"],
        ]);
        let mut positions = vec![100_u32, 700, 740];

        merge_disjoint_numeric_columns(&mut table, &mut positions, 25);

        assert_eq!(positions.len(), 2, "the split amount column must fold into one column");
        assert_eq!(table[0], vec!["Item".to_string(), "Column B".to_string()]);
        assert_eq!(table[2], vec!["Item 1".to_string(), "1,250".to_string()]);
        assert_eq!(table[3], vec!["Item 2".to_string(), "87".to_string()]);
    }

    /// Negative control: a data cell with letters in it is still not an amount, so two close
    /// tracks where one holds words stay separate.
    #[cfg(feature = "ocr")]
    #[test]
    fn a_word_cell_in_a_data_row_still_blocks_a_split_column_merge() {
        let mut table = grid(&[
            &["Item", "Column B", ""],
            &["Item 1", "1,250", ""],
            &["Item 2", "", "see note"],
        ]);
        let mut positions = vec![100_u32, 700, 740];

        merge_disjoint_numeric_columns(&mut table, &mut positions, 25);

        assert_eq!(positions.len(), 3, "a track holding words must not be merged");
    }

    /// A sign, currency sign or bracket that OCR split off the front of a short amount lands in the
    /// left track, in front of the amount in the right track. It is part of the value, so the pair
    /// stays apart rather than merge and drop it.
    #[cfg(feature = "ocr")]
    #[test]
    fn a_symbol_in_front_of_a_value_blocks_a_split_column_merge() {
        for symbol in ["-", "$", "("] {
            let mut table = grid(&[
                &["Item", "Column B", ""],
                &["Item 1", "1,250", ""],
                &["Item 2", symbol, "87"],
                &["Item 3", "", "56"],
            ]);
            let mut positions = vec![100_u32, 700, 740];

            merge_disjoint_numeric_columns(&mut table, &mut positions, 25);

            assert_eq!(
                positions.len(),
                3,
                "a {symbol:?} in front of a value must not be merged away"
            );
            assert_eq!(
                table[2],
                vec!["Item 2".to_string(), symbol.to_string(), "87".to_string()]
            );
        }
    }

    /// Rule marks do not count as support: the merged column keeps the position of the track that
    /// holds more values, not the one that holds more marks.
    #[cfg(feature = "ocr")]
    #[test]
    fn rule_marks_do_not_decide_the_position_of_a_merged_column() {
        let mut table = grid(&[
            &["Item", "Column B", ""],
            &["Item 1", "1,250", "-"],
            &["Item 2", "940", "——"],
            &["Item 3", "", "87"],
        ]);
        let mut positions = vec![100_u32, 700, 740];

        merge_disjoint_numeric_columns(&mut table, &mut positions, 25);

        assert_eq!(positions, vec![100, 700]);
    }

    /// A header row that holds a number of its own (a year in the first column) is still the
    /// header: the header band is never shorter than row 0.
    #[cfg(feature = "ocr")]
    #[test]
    fn a_header_row_holding_a_number_is_still_the_header() {
        let mut table = grid(&[
            &["2024", "Column B", ""],
            &["Item 1", "1,250", ""],
            &["Item 2", "", "87"],
        ]);
        let mut positions = vec![100_u32, 700, 740];

        merge_disjoint_numeric_columns(&mut table, &mut positions, 25);

        assert_eq!(positions.len(), 2, "the split amount column must fold into one column");
        assert_eq!(table[0], vec!["2024".to_string(), "Column B".to_string()]);
    }

    /// A number-only row under the header ends the header band, as before: its value is data, and
    /// it folds into the merged column instead of labelling the right track.
    #[cfg(feature = "ocr")]
    #[test]
    fn a_number_only_row_under_the_header_is_data() {
        let mut table = grid(&[
            &["Item", "Column B", ""],
            &["", "", "2024"],
            &["Item 1", "1,250", ""],
            &["Item 2", "", "87"],
        ]);
        let mut positions = vec![100_u32, 700, 740];

        merge_disjoint_numeric_columns(&mut table, &mut positions, 25);

        assert_eq!(positions.len(), 2, "the split amount column must fold into one column");
        assert_eq!(table[1], vec![String::new(), "2024".to_string()]);
    }

    /// End to end through the OCR table cleanup: without the merge, the right track of the split
    /// column is a headerless, mostly empty column and the sparse-column gate drops the whole
    /// table.
    #[cfg(all(feature = "ocr", feature = "pdf"))]
    #[test]
    fn table_with_a_split_amount_column_and_rule_marks_is_kept() {
        let mut table = grid(&[
            &["Item", "Column A", "Column B", "Column C", ""],
            &["Item 1", "2,110", "3,040", "1,250", ""],
            &["Item 2", "2,380", "3,150", "", "87"],
            &["Item 3", "2,470", "3,260", "940", "——"],
            &["Item 4", "2,560", "3,370", "4,020", ""],
            &["Item 5", "2,650", "3,480", "", "56"],
            &["Item 6", "2,740", "3,590", "1,330", ""],
            &["Item 7", "2,830", "3,600", "", "72"],
            &["Item 8", "2,920", "3,710", "2,480", ""],
            &["Item 9", "3,010", "3,820", "1,960", ""],
            &["Item 10", "3,100", "3,930", "3,570", ""],
        ]);
        let mut positions = vec![100_u32, 400, 700, 1000, 1040];

        merge_disjoint_numeric_columns(&mut table, &mut positions, 25);
        let kept = crate::pdf::table_reconstruct::post_process_table(table, false, false)
            .expect("the table must survive the OCR table cleanup");

        assert_eq!(kept[0].len(), 4, "the split column must come back as one column");
        let column_c: Vec<&str> = kept[1..].iter().map(|row| row[3].as_str()).collect();
        assert_eq!(
            column_c,
            [
                "1,250", "87", "940", "4,020", "56", "1,330", "72", "2,480", "1,960", "3,570"
            ]
        );
    }

    /// xberg-io/xberg#1649: a lone punctuation glyph (e.g. the dash in a date range) must merge
    /// into its neighboring cell even when its gap is slightly wider than the normal cell-merge
    /// threshold, so it does not mint a spurious extra column.
    #[test]
    fn issue_1649_punctuation_glyph_merges_across_a_widened_gap() {
        let words = vec![
            word("Jan", 0, 0, 30, 20),
            word("1", 35, 0, 10, 20),
            // Gap from "1"'s right edge (45) to "-"'s left edge (57) is 12px; gap from "-"'s
            // right edge (67) to "Jan"'s left edge (80) is 13px. Median height is 20, so the
            // normal merge_gap (0.6 * 20 = 12) alone would leave the second gap (13) unmerged.
            word("-", 57, 0, 10, 20),
            word("Jan", 80, 0, 30, 20),
            word("31,", 115, 0, 25, 20),
            word("2026", 145, 0, 40, 20),
        ];

        let table = reconstruct_table(&words, 500, 0.5);

        assert_eq!(table.len(), 1);
        assert_eq!(
            table[0],
            vec!["Jan 1 - Jan 31, 2026".to_string()],
            "the dash must glue the date range into a single cell, not split it"
        );
    }

    /// Regression for the punctuation-glue-gap connector rule (xberg-io/xberg#1649 review
    /// follow-up): a lone punctuation cell (e.g. a "-" placeholder for a zero amount) sitting
    /// near a real column boundary must not steal the preceding word into its cell just because
    /// it is punctuation. The widened gap must apply only when the punctuation genuinely
    /// bridges two neighboring words on both sides.
    #[test]
    fn issue_1649_lone_punctuation_cell_keeps_its_own_column() {
        let words = vec![
            word("Debit", 0, 0, 50, 20),
            // Gap from "Debit" (right edge 50) to "-" (left 63) is 13px -- just over the normal
            // merge_gap (0.6 * 20 = 12) but under the punctuation-widened gap (24). Gap from "-"
            // (right edge 73) to "Credit" (left 130) is 57px -- nowhere near even the widened
            // gap, so "-" has no genuine neighbour to bridge and must not glue to "Debit"
            // either. ~keep
            word("-", 63, 0, 10, 20),
            word("Credit", 130, 0, 60, 20),
        ];

        let table = reconstruct_table(&words, 20, 0.5);

        assert_eq!(table.len(), 1);
        assert_eq!(
            table[0],
            vec!["Debit".to_string(), "-".to_string(), "Credit".to_string()],
            "an isolated punctuation cell with no genuine neighbour must not glue to the preceding word"
        );
    }

    /// xberg-io/xberg#1886: a right-aligned amount column's tokens share a right edge and spread on
    /// the left with digit count, so left-edge grouping alone mints more than one column from it --
    /// here `5`/`73`/`-` (lefts 460-490) against `1,234,567` (left 340), 140px apart at a 50px
    /// threshold. Every one of the column's cells must land in one column, including the nil dash.
    ///
    /// TEST HONESTY: without the fold this is a 3-column grid -- `["Alpha", "", "5"]`,
    /// `["Beta", "", "73"]`, `["Gamma", "1,234,567", ""]`, `["Delta", "", "\u{2014}"]` -- and
    /// `column_positions` reads `[0, 340, 480]`.
    #[test]
    fn issue_1886_right_aligned_amount_column_stays_one_column() {
        let words = vec![
            word("Alpha", 0, 0, 100, 30),
            word("5", 480, 0, 20, 30),
            word("Beta", 0, 60, 100, 30),
            word("73", 460, 60, 40, 30),
            word("Gamma", 0, 120, 100, 30),
            word("1,234,567", 340, 120, 160, 30),
            word("Delta", 0, 180, 100, 30),
            word("\u{2014}", 490, 180, 10, 30),
        ];

        let (table, column_positions) = reconstruct_table_with_columns(&words, 50, 0.5);

        assert_eq!(
            table,
            vec![
                vec!["Alpha".to_string(), "5".to_string()],
                vec!["Beta".to_string(), "73".to_string()],
                vec!["Gamma".to_string(), "1,234,567".to_string()],
                vec!["Delta".to_string(), "\u{2014}".to_string()],
            ]
        );
        assert_eq!(
            column_positions,
            vec![0, 340],
            "the folded column keeps a left edge in page pixels, the unit every consumer reads"
        );
    }

    /// The right-edge rule applies to value tokens only (xberg-io/xberg#1886): two left-aligned text
    /// cells of very different widths ("Alpha" 40px, "Alphabetical" 300px) share a left edge and
    /// differ by 260px on the right, so clustering them by their right edge would split one label
    /// column in two.
    #[test]
    fn issue_1886_text_tokens_of_different_widths_keep_clustering_by_their_left_edge() {
        let words = vec![
            word("Alpha", 100, 0, 40, 20),
            word("X", 600, 0, 40, 20),
            word("Alphabetical", 100, 60, 300, 20),
            word("Y", 600, 60, 40, 20),
        ];

        let (table, column_positions) = reconstruct_table_with_columns(&words, 50, 0.5);

        assert_eq!(
            table,
            vec![
                vec!["Alpha".to_string(), "X".to_string()],
                vec!["Alphabetical".to_string(), "Y".to_string()],
            ]
        );
        assert_eq!(column_positions, vec![100, 600]);
    }

    /// Precision guard for the fold (xberg-io/xberg#1886): two genuinely distinct narrow value
    /// columns whose right edges are 60px apart stay separate at a 50px threshold. The fold reuses
    /// `column_threshold` rather than widening it, because widening merges real neighbouring
    /// columns (the GH#1649 `DEPOSIT` case).
    #[test]
    fn issue_1886_value_columns_further_apart_than_the_threshold_are_not_folded() {
        let words = vec![
            word("1", 0, 0, 20, 20),
            word("2", 60, 0, 20, 20),
            word("3", 0, 60, 20, 20),
            word("4", 60, 60, 20, 20),
        ];

        let (table, column_positions) = reconstruct_table_with_columns(&words, 50, 0.5);

        assert_eq!(
            table,
            vec![
                vec!["1".to_string(), "2".to_string()],
                vec!["3".to_string(), "4".to_string()],
            ]
        );
        assert_eq!(column_positions, vec![0, 60]);
    }

    /// xberg-io/xberg#1952: on a scan, one token of a value column is junk -- here the nil dash of
    /// `Delta` read as the letter `a` -- and one junk token must not make the whole track text.
    /// `73`, `a` and `8` share a right edge with `1,234,567`, and values outnumber the junk, so the
    /// column still folds into one.
    ///
    /// TEST HONESTY: when every data token has to be a value, this is a 3-column grid with
    /// `["Gamma", "1,234,567", ""]` and `["Delta", "", "a"]`.
    #[test]
    fn issue_1952_a_junk_token_does_not_stop_a_value_column_folding() {
        let words = vec![
            word("Alpha", 0, 0, 100, 30),
            word("5", 480, 0, 20, 30),
            word("Beta", 0, 60, 100, 30),
            word("73", 460, 60, 40, 30),
            word("Gamma", 0, 120, 100, 30),
            word("1,234,567", 340, 120, 160, 30),
            word("Delta", 0, 180, 100, 30),
            word("a", 485, 180, 15, 30),
            word("Epsilon", 0, 240, 100, 30),
            word("8", 480, 240, 20, 30),
        ];

        let (table, column_positions) = reconstruct_table_with_columns(&words, 50, 0.5);

        assert_eq!(
            table,
            vec![
                vec!["Alpha".to_string(), "5".to_string()],
                vec!["Beta".to_string(), "73".to_string()],
                vec!["Gamma".to_string(), "1,234,567".to_string()],
                vec!["Delta".to_string(), "a".to_string()],
                vec!["Epsilon".to_string(), "8".to_string()],
            ]
        );
        assert_eq!(column_positions, vec![0, 340]);
    }

    /// xberg-io/xberg#1952: a scanned header band spans several rows -- a label, a period, a unit.
    /// Only row 0 was read as the header, so `Year 1` and `(net)` counted as data tokens of the
    /// short track, and with two values against two labels the track never read as values. The
    /// band ends at the first row with a number, so the labels are header and the column folds.
    ///
    /// TEST HONESTY: with row 0 alone as the header, this is a 3-column grid with
    /// `["B", "12,345,678", ""]` in a column of its own.
    #[test]
    fn issue_1952_a_multi_row_header_band_does_not_stop_a_value_column_folding() {
        let words = vec![
            word("Item", 0, 0, 60, 20),
            word("Total", 250, 0, 60, 20),
            word("Year 1", 255, 60, 55, 20),
            word("(net)", 265, 120, 45, 20),
            word("A", 0, 180, 60, 20),
            word("846", 265, 180, 45, 20),
            word("B", 0, 240, 60, 20),
            word("12,345,678", 140, 240, 170, 20),
            word("C", 0, 300, 60, 20),
            word("5", 295, 300, 15, 20),
        ];

        let table = reconstruct_table(&words, 50, 0.5);

        assert_eq!(
            table,
            vec![
                vec!["Item".to_string(), "Total".to_string()],
                vec!["".to_string(), "Year 1".to_string()],
                vec!["".to_string(), "(net)".to_string()],
                vec!["A".to_string(), "846".to_string()],
                vec!["B".to_string(), "12,345,678".to_string()],
                vec!["C".to_string(), "5".to_string()],
            ]
        );
    }

    /// Precision guard for the fold (xberg-io/xberg#1952): a track whose data tokens are mostly
    /// text is a text column, whatever its right edge. `note` and `abc` outnumber `5`, so the
    /// track stays apart from the amount `1,234,567` that shares its right edge.
    #[test]
    fn a_text_column_with_a_coinciding_right_edge_is_not_folded() {
        let words = vec![
            word("Item", 0, 0, 60, 20),
            word("Total", 440, 0, 60, 20),
            word("A", 0, 60, 60, 20),
            word("5", 480, 60, 20, 20),
            word("B", 0, 120, 60, 20),
            word("note", 440, 120, 60, 20),
            word("C", 0, 180, 60, 20),
            word("abc", 455, 180, 45, 20),
            word("D", 0, 240, 60, 20),
            word("1,234,567", 340, 240, 160, 20),
        ];

        let table = reconstruct_table(&words, 50, 0.5);

        assert_eq!(
            table,
            vec![
                vec!["Item".to_string(), "".to_string(), "Total".to_string()],
                vec!["A".to_string(), "".to_string(), "5".to_string()],
                vec!["B".to_string(), "".to_string(), "note".to_string()],
                vec!["C".to_string(), "".to_string(), "abc".to_string()],
                vec!["D".to_string(), "1,234,567".to_string(), "".to_string()],
            ]
        );
    }

    /// Precision guard for the fold (xberg-io/xberg#1952): a track with as many junk tokens as
    /// values has no majority of values, so it stays apart from the amount that shares its right
    /// edge. `5` and `7` tie with `x` and `b`.
    #[test]
    fn a_track_with_as_many_junk_tokens_as_values_is_not_folded() {
        let words = vec![
            word("Item", 0, 0, 60, 20),
            word("Total", 250, 0, 60, 20),
            word("A", 0, 60, 60, 20),
            word("5", 295, 60, 15, 20),
            word("B", 0, 120, 60, 20),
            word("x", 297, 120, 13, 20),
            word("C", 0, 180, 60, 20),
            word("12,345,678", 140, 180, 170, 20),
            word("D", 0, 240, 60, 20),
            word("7", 295, 240, 15, 20),
            word("E", 0, 300, 60, 20),
            word("b", 297, 300, 13, 20),
        ];

        let table = reconstruct_table(&words, 50, 0.5);

        assert_eq!(
            table,
            vec![
                vec!["Item".to_string(), "".to_string(), "Total".to_string()],
                vec!["A".to_string(), "".to_string(), "5".to_string()],
                vec!["B".to_string(), "".to_string(), "x".to_string()],
                vec!["C".to_string(), "12,345,678".to_string(), "".to_string()],
                vec!["D".to_string(), "".to_string(), "7".to_string()],
                vec!["E".to_string(), "".to_string(), "b".to_string()],
            ]
        );
    }

    /// xberg-io/xberg#1952: a number in row 0, such as a year, ends the header band there, but
    /// row 0 stays the header. The label `Total` is not read as a data token of its track, so the
    /// track (`5` alone) reads as values and folds with `12,345,678`.
    ///
    /// TEST HONESTY: when row 0 counts as data, `Total` ties with `5` and the grid has 3 columns.
    #[test]
    fn a_number_in_row_zero_keeps_row_zero_as_the_header() {
        let words = vec![
            word("2024", 0, 0, 60, 20),
            word("Total", 250, 0, 60, 20),
            word("A", 0, 60, 60, 20),
            word("12,345,678", 140, 60, 170, 20),
            word("B", 0, 120, 60, 20),
            word("5", 295, 120, 15, 20),
        ];

        let table = reconstruct_table(&words, 50, 0.5);

        assert_eq!(
            table,
            vec![
                vec!["2024".to_string(), "Total".to_string()],
                vec!["A".to_string(), "12,345,678".to_string()],
                vec!["B".to_string(), "5".to_string()],
            ]
        );
    }

    /// The right-edge split (xberg-io/xberg#1909) reads every row below row 0, whatever the fold's
    /// header band. The only number in this text table sits in its last row, so the band holds
    /// rows 0 to 3; the split must still see them and keep `N` and `Code` apart.
    ///
    /// TEST HONESTY: when the split skips the band, the group has one data token, it does not
    /// split, and the rows read `["N Code", "Qty"]`, `["Al ABCDEFG", ""]`.
    #[test]
    fn two_close_text_columns_split_when_the_first_number_sits_far_down() {
        let words = vec![
            word("N", 0, 0, 15, 20),
            word("Code", 45, 0, 60, 20),
            word("Qty", 400, 0, 40, 20),
            word("Al", 0, 60, 15, 20),
            word("ABCDEFG", 45, 60, 105, 20),
            word("Bo", 0, 120, 15, 20),
            word("HIJKLMN", 45, 120, 105, 20),
            word("Cy", 0, 180, 15, 20),
            word("OPQRSTU", 45, 180, 105, 20),
            word("VWXYZAB", 45, 240, 105, 20),
            word("7", 425, 240, 15, 20),
        ];

        let table = reconstruct_table(&words, 50, 0.5);

        assert_eq!(
            table,
            vec![
                vec!["N".to_string(), "Code".to_string(), "Qty".to_string()],
                vec!["Al".to_string(), "ABCDEFG".to_string(), "".to_string()],
                vec!["Bo".to_string(), "HIJKLMN".to_string(), "".to_string()],
                vec!["Cy".to_string(), "OPQRSTU".to_string(), "".to_string()],
                vec!["".to_string(), "VWXYZAB".to_string(), "7".to_string()],
            ]
        );
    }

    /// A column folded from two tracks holds the numbers of both. `7` and `9` fold with
    /// `12,345,678`, and the quantities `5` and `3` share their rows, so the quantity column stays
    /// apart even though it shares no row with `12,345,678`.
    ///
    /// TEST HONESTY: when the folded column keeps the rows of its first track only, rows 1 and 3
    /// read `["A", "7 5"]` and `["C", "9 3"]`.
    #[test]
    fn a_quantity_column_does_not_fold_into_an_amount_column_folded_from_two_tracks() {
        let words = vec![
            word("Item", 0, 0, 60, 20),
            word("Total", 250, 0, 60, 20),
            word("A", 0, 60, 60, 20),
            word("7", 295, 60, 15, 20),
            word("5", 350, 60, 8, 20),
            word("B", 0, 120, 60, 20),
            word("12,345,678", 140, 120, 170, 20),
            word("C", 0, 180, 60, 20),
            word("9", 295, 180, 15, 20),
            word("3", 350, 180, 8, 20),
        ];

        let table = reconstruct_table(&words, 50, 0.5);

        assert_eq!(
            table,
            vec![
                vec!["Item".to_string(), "Total".to_string(), "".to_string()],
                vec!["A".to_string(), "7".to_string(), "5".to_string()],
                vec!["B".to_string(), "12,345,678".to_string(), "".to_string()],
                vec!["C".to_string(), "9".to_string(), "3".to_string()],
            ]
        );
    }

    /// Two numbers in one row are two cells: a code column whose right edge lies within the
    /// threshold of the quantity column beside it never folds into it, whether its codes are all
    /// numbers or mostly numbers.
    ///
    /// TEST HONESTY: without the number-row check, each row reads `["A", "1010 5"]`: the code and
    /// the quantity share one cell.
    #[test]
    fn a_code_column_does_not_fold_into_the_quantity_column_beside_it() {
        for third_code in ["1030", "10A5"] {
            let words = vec![
                word("Item", 0, 0, 60, 20),
                word("Code", 300, 0, 40, 20),
                word("Qty", 360, 0, 25, 20),
                word("A", 0, 60, 60, 20),
                word("1010", 300, 60, 40, 20),
                word("5", 370, 60, 15, 20),
                word("B", 0, 120, 60, 20),
                word("1020", 300, 120, 40, 20),
                word("7", 370, 120, 15, 20),
                word("C", 0, 180, 60, 20),
                word(third_code, 300, 180, 40, 20),
                word("3", 370, 180, 15, 20),
                word("D", 0, 240, 60, 20),
                word("1040", 300, 240, 40, 20),
                word("9", 370, 240, 15, 20),
            ];

            let table = reconstruct_table(&words, 50, 0.5);

            assert_eq!(
                table,
                vec![
                    vec!["Item".to_string(), "Code".to_string(), "Qty".to_string()],
                    vec!["A".to_string(), "1010".to_string(), "5".to_string()],
                    vec!["B".to_string(), "1020".to_string(), "7".to_string()],
                    vec!["C".to_string(), third_code.to_string(), "3".to_string()],
                    vec!["D".to_string(), "1040".to_string(), "9".to_string()],
                ],
                "third code {third_code}"
            );
        }
    }

    /// xberg-io/xberg#1909: a one-digit amount of one column starts within the threshold of an
    /// eight-digit amount of the next, so one left-edge group holds cells of two columns. Two cells
    /// of one row cannot be one column, so the group splits on its right edges and each amount stays
    /// in its own column.
    ///
    /// TEST HONESTY: without the split, row 0 reads `["A", "", "7 40,218,965"]`: both amounts share
    /// a cell.
    #[test]
    fn issue_1909_amount_columns_whose_left_edges_interleave_stay_apart() {
        let words = vec![
            word("A", 0, 0, 60, 20),
            word("7", 285, 0, 15, 20),
            word("40,218,965", 330, 0, 170, 20),
            word("B", 0, 60, 60, 20),
            word("12,345,678", 130, 60, 170, 20),
            word("4", 485, 60, 15, 20),
            word("C", 0, 120, 60, 20),
            word("5", 285, 120, 15, 20),
            word("17,382,649", 332, 120, 168, 20),
        ];

        let (table, column_positions) = reconstruct_table_with_columns(&words, 50, 0.5);

        assert_eq!(
            table,
            vec![
                vec!["A".to_string(), "7".to_string(), "40,218,965".to_string()],
                vec!["B".to_string(), "12,345,678".to_string(), "4".to_string()],
                vec!["C".to_string(), "5".to_string(), "17,382,649".to_string()],
            ]
        );
        assert_eq!(column_positions, vec![0, 130, 332]);
    }

    #[test]
    fn issue_1769_footer_fragments_do_not_split_a_six_column_table() {
        let columns = [44, 87, 129, 171, 211, 252];
        let mut words = Vec::new();
        for (column, text) in ["Number", "DP", "SP-C", "SP-D", "DP vs SP-C", "DP vs SP-D"]
            .into_iter()
            .enumerate()
        {
            words.push(word(text, columns[column], 0, 24, 6));
        }
        for row in 1..29 {
            let top = row * 12;
            let cells = if row == 1 {
                ["Number", "166/647", "157/647", "324/647", "0.114", "<0.001"]
            } else {
                ["Measure", "10", "20", "30", "0.1", "0.2"]
            };
            for (column, text) in cells.into_iter().enumerate() {
                words.push(word(text, columns[column], top, 24, 6));
            }
        }
        let footer_top = 29 * 12;
        words.push(word("Fisher", 38, footer_top, 10, 6));
        words.push(word("exact test (2x2), and quantitative", 66, footer_top, 107, 6));
        for (column, text) in ["10", "20", "30", "0.1", "0.2"].into_iter().enumerate() {
            words.push(word(text, columns[column + 1], footer_top, 24, 6));
        }

        let (table, column_positions) = reconstruct_table_with_columns(&words, 30, 0.5);

        assert_eq!(table.len(), 30);
        assert_eq!(column_positions.len(), 6);
        assert_eq!(table[1], ["Number", "166/647", "157/647", "324/647", "0.114", "<0.001"]);
    }

    /// xberg-io/xberg#1909: a header label clusters with the values under it by its left edge. Its
    /// text must not stop that group folding with the rest of its right-aligned column.
    ///
    /// TEST HONESTY: when the header word counts against the fold, the grid keeps three columns and
    /// `12,345,678` sits in a column of its own.
    #[test]
    fn issue_1909_a_header_label_does_not_stop_its_value_column_folding() {
        let words = vec![
            word("Item", 0, 0, 60, 20),
            word("Total", 250, 0, 60, 20),
            word("A", 0, 60, 60, 20),
            word("846", 255, 60, 45, 20),
            word("B", 0, 120, 60, 20),
            word("12,345,678", 130, 120, 170, 20),
            word("C", 0, 180, 60, 20),
            word("5", 285, 180, 15, 20),
        ];

        let table = reconstruct_table(&words, 50, 0.5);

        assert_eq!(
            table,
            vec![
                vec!["Item".to_string(), "Total".to_string()],
                vec!["A".to_string(), "846".to_string()],
                vec!["B".to_string(), "12,345,678".to_string()],
                vec!["C".to_string(), "5".to_string()],
            ]
        );
    }

    /// xberg-io/xberg#1909: the split is taken only when every part holds at most one cell per row.
    /// Here `1` and `2` share a row and a right edge, so re-grouping by right edge cannot separate
    /// them, and the group stays one column.
    ///
    /// TEST HONESTY: taking that split anyway yields `[["1", "2"], ["3", ""]]`.
    #[test]
    fn issue_1909_a_split_that_leaves_two_cells_of_one_row_together_is_not_taken() {
        let words = vec![
            word("1", 100, 0, 10, 20),
            word("2", 135, 0, 10, 20),
            word("3", 120, 60, 90, 20),
        ];

        let table = reconstruct_table(&words, 50, 0.5);

        assert_eq!(table, vec![vec!["1 2".to_string()], vec!["3".to_string()]]);
    }

    /// xberg-io/xberg#1909: two left-aligned text columns that start closer than the threshold
    /// share one left-edge group. Two cells of one row cannot be one column whatever their text,
    /// so the group splits on its right edges as a group of amounts does.
    ///
    /// TEST HONESTY: when only a group of values may split, every row reads as one cell, such as
    /// `["Al ABCDEFG"]`.
    #[test]
    fn issue_1909_two_close_text_columns_split_like_two_close_amount_columns() {
        let words = vec![
            word("N", 0, 0, 15, 20),
            word("Code", 45, 0, 60, 20),
            word("Al", 0, 60, 15, 20),
            word("ABCDEFG", 45, 60, 105, 20),
            word("Bo", 0, 120, 15, 20),
            word("HIJKLMN", 45, 120, 105, 20),
            word("Cy", 0, 180, 15, 20),
            word("OPQRSTU", 45, 180, 105, 20),
        ];

        let table = reconstruct_table(&words, 50, 0.5);

        assert_eq!(
            table,
            vec![
                vec!["N".to_string(), "Code".to_string()],
                vec!["Al".to_string(), "ABCDEFG".to_string()],
                vec!["Bo".to_string(), "HIJKLMN".to_string()],
                vec!["Cy".to_string(), "OPQRSTU".to_string()],
            ]
        );
    }

    /// xberg-io/xberg#1909: a header label written from the left edge of its column joins the
    /// left-edge group where a one-digit amount of the column before meets the long amounts of its
    /// own column. The label must not stop that group splitting on its right edges.
    ///
    /// TEST HONESTY: when the header label counts against the split, row 1 reads
    /// `["A", "", "7 40,218,965"]`: both amounts share a cell. When the label is re-grouped by its
    /// right edge with the amounts, it leaves its column, and row 3's amount lands in a fourth
    /// column with row 2's `4`.
    #[test]
    fn issue_1909_a_header_label_does_not_stop_a_row_sharing_value_group_splitting() {
        let words = vec![
            word("Item", 0, 0, 60, 20),
            word("One", 130, 0, 50, 20),
            word("Two", 330, 0, 50, 20),
            word("A", 0, 60, 60, 20),
            word("7", 285, 60, 15, 20),
            word("40,218,965", 330, 60, 170, 20),
            word("B", 0, 120, 60, 20),
            word("12,345,678", 130, 120, 170, 20),
            word("4", 485, 120, 15, 20),
            word("C", 0, 180, 60, 20),
            word("5", 285, 180, 15, 20),
            word("17,382,649", 332, 180, 168, 20),
        ];

        let table = reconstruct_table(&words, 50, 0.5);

        assert_eq!(
            table,
            vec![
                vec!["Item".to_string(), "One".to_string(), "Two".to_string()],
                vec!["A".to_string(), "7".to_string(), "40,218,965".to_string()],
                vec!["B".to_string(), "12,345,678".to_string(), "4".to_string()],
                vec!["C".to_string(), "5".to_string(), "17,382,649".to_string()],
            ]
        );
    }

    /// A header label that reads as text, such as a period written as letters and digits, is often
    /// wider than the amounts under it: it starts more than the threshold left of every amount and
    /// forms a header-only track. Its right edge is the column's right edge, so it folds with the
    /// column whatever its text reads. Without that, the label keeps a column of its own, and the
    /// long amounts, whose left edges sit nearer the label's than the short amount's, follow it.
    ///
    /// TEST HONESTY: without the fix the grid has three value columns for one: the label over the
    /// long amounts, and `5` in a column of its own, `["B", "", "5"]`. ~keep
    #[test]
    fn a_text_header_label_on_its_own_track_folds_with_its_amount_column() {
        let words = vec![
            word("Item", 0, 0, 60, 20),
            word("Period 1", 400, 0, 100, 20),
            word("A", 0, 60, 60, 20),
            word("12,345,678", 330, 60, 170, 20),
            word("B", 0, 120, 60, 20),
            word("5", 485, 120, 15, 20),
            word("C", 0, 180, 60, 20),
            word("40,218,965", 330, 180, 170, 20),
        ];

        let table = reconstruct_table(&words, 50, 0.5);

        assert_eq!(
            table,
            vec![
                vec!["Item".to_string(), "Period 1".to_string()],
                vec!["A".to_string(), "12,345,678".to_string()],
                vec!["B".to_string(), "5".to_string()],
                vec!["C".to_string(), "40,218,965".to_string()],
            ]
        );
    }

    /// Control: a header-only track folds only with a value column whose right edge it shares. A
    /// label whose right edge is its own, here over a column with no data at all, keeps its
    /// column, with or without the fold of a wider label. ~keep
    #[test]
    fn a_header_label_with_its_own_right_edge_keeps_its_column() {
        let words = vec![
            word("Item", 0, 0, 60, 20),
            word("Note", 250, 0, 60, 20),
            word("Amt", 455, 0, 45, 20),
            word("A", 0, 60, 60, 20),
            word("12,345,678", 330, 60, 170, 20),
            word("B", 0, 120, 60, 20),
            word("5", 485, 120, 15, 20),
        ];

        let table = reconstruct_table(&words, 50, 0.5);

        assert_eq!(
            table,
            vec![
                vec!["Item".to_string(), "Note".to_string(), "Amt".to_string()],
                vec!["A".to_string(), String::new(), "12,345,678".to_string()],
                vec!["B".to_string(), String::new(), "5".to_string()],
            ]
        );
    }

    /// A header-only track whose right edge meets a value column's still heads a column of its own
    /// when that value column has a label in the same row: two labels side by side are two
    /// columns. Here an empty `Notes` column ends just left of a narrow `Qty` column, and an empty
    /// `Note` column starts just right of a wide `Amount` column. Each gap is wider than a word
    /// space, so the two labels stay two cell tokens.
    ///
    /// TEST HONESTY: when the fold ignores the value column's own label, each pair shares one
    /// cell, `Notes Qty` and `Amount Note`, and the empty column is gone. ~keep
    #[test]
    fn a_header_only_label_beside_a_labelled_value_column_keeps_its_column() {
        let notes_left_of_quantities = vec![
            word("Item", 0, 0, 60, 20),
            word("Notes", 370, 0, 80, 20),
            word("Qty", 470, 0, 30, 20),
            word("A", 0, 60, 60, 20),
            word("5", 485, 60, 15, 20),
            word("B", 0, 120, 60, 20),
            word("7", 485, 120, 15, 20),
            word("C", 0, 180, 60, 20),
            word("12", 470, 180, 30, 20),
        ];
        let note_right_of_amounts = vec![
            word("Item", 0, 0, 60, 20),
            word("Amount", 380, 0, 120, 20),
            word("Note", 520, 0, 30, 20),
            word("A", 0, 60, 60, 20),
            word("12,345", 400, 60, 100, 20),
            word("B", 0, 120, 60, 20),
            word("40,218", 400, 120, 100, 20),
        ];

        assert_eq!(
            reconstruct_table(&notes_left_of_quantities, 50, 0.5)[0],
            vec!["Item", "Notes", "Qty"]
        );
        assert_eq!(
            reconstruct_table(&note_right_of_amounts, 50, 0.5)[0],
            vec!["Item", "Amount", "Note"]
        );
    }

    /// A header-only track that has folded into its value column still counts as that column's
    /// label. A second header-only label to its right, in the same row, keeps a column of its own:
    /// `Period 1` folds with the amounts it ends with, and `Note` stays a column beside it. The gap
    /// between the two labels is wider than a word space, so they stay two cell tokens.
    ///
    /// TEST HONESTY: when the folded column forgets the label it took in, `Note` folds into it as
    /// well and the header reads `Period 1 Note`. ~keep
    #[test]
    fn a_folded_label_keeps_a_neighbouring_label_in_its_own_column() {
        let words = vec![
            word("Item", 0, 0, 60, 20),
            word("Period 1", 400, 0, 100, 20),
            word("Note", 520, 0, 30, 20),
            word("A", 0, 60, 60, 20),
            word("12,345,678", 330, 60, 170, 20),
            word("B", 0, 120, 60, 20),
            word("1,234", 425, 120, 75, 20),
            word("C", 0, 180, 60, 20),
            word("40,218,965", 330, 180, 170, 20),
        ];

        let table = reconstruct_table(&words, 50, 0.5);

        assert_eq!(
            table,
            vec![
                vec!["Item".to_string(), "Period 1".to_string(), "Note".to_string()],
                vec!["A".to_string(), "12,345,678".to_string(), String::new()],
                vec!["B".to_string(), "1,234".to_string(), String::new()],
                vec!["C".to_string(), "40,218,965".to_string(), String::new()],
            ]
        );
    }

    /// The position a folded column reports is the left edge of its data, whatever folds after the
    /// header-only label. Here a label starts left of the long amounts it heads, folds with them,
    /// and the short amounts then fold in by their right edge: the column stays at the long
    /// amounts' left edge, where the OCR blank-quantity retry crops.
    ///
    /// TEST HONESTY: when the folded column forgets that it holds data, the second fold reads it
    /// as a header-only track and moves the column to the short amounts' left edge, 485. ~keep
    #[test]
    fn a_folded_label_column_keeps_the_position_of_its_long_amounts() {
        let words = vec![
            word("Item", 0, 0, 60, 20),
            word("Period 1", 270, 0, 230, 20),
            word("A", 0, 60, 60, 20),
            word("12,345,678", 330, 60, 170, 20),
            word("B", 0, 120, 60, 20),
            word("5", 485, 120, 15, 20),
            word("C", 0, 180, 60, 20),
            word("40,218,965", 330, 180, 170, 20),
        ];

        let (table, positions) = reconstruct_table_with_columns(&words, 50, 0.5);

        assert_eq!(table[0], vec!["Item".to_string(), "Period 1".to_string()]);
        assert_eq!(positions, vec![0, 330]);
    }

    /// xberg-io/xberg#1909: a header label that reads as a value, such as a year, can start more
    /// than the threshold right of its column's long amounts and form a header-only track. Its right
    /// edge is the column's right edge, so it folds with the column as it did before the fold test
    /// read data tokens only.
    ///
    /// TEST HONESTY: when a header-only track never folds, the year keeps its own column between
    /// the long amounts and the one-digit amount, and every row spreads over four columns, such as
    /// `["B", "", "", "5"]`.
    #[test]
    fn issue_1909_a_value_header_only_track_folds_with_its_amount_column() {
        let words = vec![
            word("Item", 0, 0, 60, 20),
            word("2023", 420, 0, 80, 20),
            word("A", 0, 60, 60, 20),
            word("12,345,678", 330, 60, 170, 20),
            word("B", 0, 120, 60, 20),
            word("5", 485, 120, 15, 20),
            word("C", 0, 180, 60, 20),
            word("40,218,965", 330, 180, 170, 20),
        ];

        let table = reconstruct_table(&words, 50, 0.5);

        assert_eq!(
            table,
            vec![
                vec!["Item".to_string(), "2023".to_string()],
                vec!["A".to_string(), "12,345,678".to_string()],
                vec!["B".to_string(), "5".to_string()],
                vec!["C".to_string(), "40,218,965".to_string()],
            ]
        );
    }
    fn ledger_rows(
        rows: u32,
        gap: u32,
        first: impl Fn(u32) -> String,
        second: impl Fn(u32) -> String,
    ) -> Vec<HocrWord> {
        let mut words = Vec::new();
        for i in 1..=rows {
            let top = 100 + i * 60;
            let first_text = first(i);
            let first_width = 230;
            words.push(word(&first_text, 100, top, first_width, 32));
            words.push(word(&second(i), 100 + first_width + gap, top, 60, 32));
            words.push(word(&format!("{}", 10000 + i * 37), 900, top, 130, 32));
        }
        words
    }

    #[test]
    fn repeated_column_edge_within_merge_gap_stays_a_separate_column() {
        for gap in [10u32, 16] {
            let words = ledger_rows(32, gap, |i| format!("ROW{i:04}"), |i| format!("{}", (i * 7) % 90 + 10));
            let table = reconstruct_table(&words, 20, 0.5);
            assert_eq!(table.len(), 32, "gap {gap}");
            for (index, row) in table.iter().enumerate() {
                let i = index as u32 + 1;
                assert_eq!(
                    row,
                    &vec![
                        format!("ROW{i:04}"),
                        format!("{}", (i * 7) % 90 + 10),
                        format!("{}", 10000 + i * 37)
                    ],
                    "gap {gap} row {i}"
                );
            }
        }
    }

    #[test]
    fn repeated_label_prefix_cell_still_merges() {
        let words = ledger_rows(32, 10, |_| "Item".to_string(), |i| format!("{}", (i * 7) % 90 + 10));
        let table = reconstruct_table(&words, 20, 0.5);
        assert_eq!(table.len(), 32);
        assert_eq!(table[0].len(), 2);
        assert_eq!(table[0][0], format!("Item {}", 7 + 10));
    }

    #[test]
    fn few_aligned_rows_are_not_column_evidence() {
        let words = ledger_rows(4, 10, |i| format!("ROW{i:04}"), |i| format!("{}", (i * 7) % 90 + 10));
        let table = reconstruct_table(&words, 20, 0.5);
        assert_eq!(table[0].len(), 2);
        assert!(table[0][0].starts_with("ROW0001 "));
    }
}
