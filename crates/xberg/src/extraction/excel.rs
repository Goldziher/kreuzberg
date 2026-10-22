//! Excel and spreadsheet extraction functions.
//!
//! This module provides Excel file parsing using the `calamine` library.
//! Supports both modern Office Open XML formats (.xlsx, .xlsm, .xlam, .xltm, .xlsb)
//! and legacy binary formats (.xls, .xla), as well as OpenDocument spreadsheets (.ods).
//!
//! # Features
//!
//! - **Multiple formats**: XLSX, XLSM, XLS, XLSB, ODS
//! - **Sheet extraction**: Reads all sheets from workbook
//! - **Markdown conversion**: Converts spreadsheet data to Markdown tables
//! - **Office metadata**: Extracts core properties, custom properties (when `office` feature enabled)
//! - **Error handling**: Distinguishes between format errors and true I/O errors
//!
//! # Example
//!
//! Not run as a doctest: `pub(crate)`, so it is unreachable from a downstream crate.
//!
//! ```ignore
//! use xberg::extraction::excel::read_excel_file;
//!
//! # fn example() -> xberg::Result<()> {
//! let (workbook, _warnings) = read_excel_file("data.xlsx", &Default::default())?;
//!
//! println!("Sheet count: {}", workbook.sheets.len());
//! for sheet in &workbook.sheets {
//!     println!("Sheet: {}", sheet.name);
//! }
//! # Ok(())
//! # }
//! ```
use calamine::{Data, DataRef, Range, Reader, SheetVisible, open_workbook_auto};
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::io::{Cursor, Read, Seek};
use std::path::Path;

use crate::core::diagnostics::push_warning;
use crate::error::{Result, XbergError};
use crate::extraction::capacity;
use crate::extractors::security::SecurityLimits;
use crate::types::revisions::{DocumentRevision, RevisionDelta, RevisionKind};
use crate::types::{ExcelSheet, ExcelWorkbook, ProcessingWarning};

/// Maximum number of cells in a Range's bounding box before we consider it pathological.
/// This threshold is set to prevent OOM when processing files with sparse data at extreme
/// positions (e.g., Excel Solver files that have cells at A1 and XFD1048575).
///
/// 100 million cells at ~64 bytes each = ~6.4 GB, which is a reasonable upper limit.
const MAX_BOUNDING_BOX_CELLS: u64 = 100_000_000;

/// Maximum number of formula/hyperlink/comment entries recorded per sheet in
/// `ExcelWorkbook::metadata` before the list is truncated. Keeps the metadata
/// map bounded for a pathological sheet with tens of thousands of formulas or
/// hyperlinks (xberg-io/xberg#89); this is a defensive cap, not a
/// user-configurable limit.
const MAX_METADATA_ENTRIES_PER_SHEET: usize = 200;

/// Maximum bytes read from a single auxiliary ZIP member inside an XLSX/XLSM/XLTM
/// archive (`xl/revisions/revisionHeaders.xml`, `xl/comments*.xml`).
///
/// The pinned `zip` crate (`zip-2.4.2/src/read.rs:444`) wraps a decompressed entry as
/// `Crc32Reader::new(Decompressor::new(..), crc32, ..)` with no `Take` on the
/// decompressed side -- only the *compressed* stream is bounded, and the CRC is
/// verified at EOF, i.e. after the whole entry is already in memory. `validate_zip_container`
/// (above) only inspects the central directory's *declared* entry count, aggregate size, and
/// compression ratio, so nothing there bounds what a single crafted member actually expands to
/// when read.
/// Bounding every `read_to_end` call with `Read::take` (the same pattern as
/// `docx::MAX_UNCOMPRESSED_FILE_SIZE` and `hwpx::MAX_HWPX_MEMBER_SIZE`) is what actually
/// caps memory here.
const MAX_EXCEL_ZIP_MEMBER_SIZE: u64 = 100 * 1024 * 1024;

#[cfg(feature = "office")]
use crate::extraction::office_metadata::{
    app_properties::{DOC_SECURITY_KEY, decode_doc_security_flags},
    extract_core_properties, extract_custom_properties, extract_xlsx_app_properties,
};
#[cfg(feature = "office")]
use serde_json::Value;

/// Office-metadata, revision-header, and cell-comment extraction, split out to keep this file
/// under the line-count limit.
mod package_metadata;
#[cfg(test)]
use package_metadata::parse_comments_xml;
#[cfg(feature = "office")]
use package_metadata::{
    extract_ods_office_metadata_from_bytes, extract_ods_office_metadata_from_file,
    extract_xlsx_office_metadata_from_bytes, extract_xlsx_office_metadata_from_file,
};
use package_metadata::{
    extract_xlsx_comments_from_bytes, extract_xlsx_comments_from_file, extract_xlsx_revisions_from_bytes,
    extract_xlsx_revisions_from_file,
};

mod datetime;

/// Format dispatch and the `read_excel_file`/`read_excel_bytes` entry points, split out to
/// keep this file under the line-count limit.
mod open;
pub(crate) use open::{read_excel_bytes, read_excel_file};

/// Result of reading a spreadsheet: the parsed workbook plus any
/// [`ProcessingWarning`]s accumulated while reading it (a sheet that failed
/// to parse, a worksheet range that could not be read, or output truncated
/// against an internal safety cap). See the module doc on
/// `core::diagnostics` for the warning convention this follows.
pub(crate) type ExcelReadResult = (ExcelWorkbook, Vec<ProcessingWarning>);

/// Process XLSX workbooks with special handling for pathological sparse files.
///
/// This function uses calamine's `worksheet_cells_reader()` API to detect sheets with
/// extreme bounding boxes BEFORE allocating memory for the full Range. This prevents
/// OOM when processing files like Excel Solver files that have cells at both A1 and
/// XFD1048575, creating a bounding box of ~17 billion cells.
fn process_xlsx_workbook<RS: Read + Seek>(
    mut workbook: calamine::Xlsx<RS>,
    office_metadata: Option<HashMap<String, String>>,
    warnings: &mut Vec<ProcessingWarning>,
) -> Result<ExcelWorkbook> {
    let sheet_names = workbook.sheet_names();
    let mut sheets = Vec::with_capacity(sheet_names.len());
    let mut extra_metadata: HashMap<String, String> = HashMap::new();

    for name in &sheet_names {
        match process_xlsx_sheet_safe(&mut workbook, name, warnings) {
            Ok(sheet) => sheets.push(sheet),
            Err(e) => {
                tracing::warn!("Failed to process sheet '{}': {}", name, e);
                push_warning(
                    warnings,
                    "excel",
                    format!("Sheet '{name}' could not be processed and was skipped ({e})"),
                );
            }
        }

        if let Some(formulas) = collect_sheet_formulas(&mut workbook, name) {
            extra_metadata.insert(format!("formulas_{name}"), formulas);
        }
        if let Some(hyperlinks) = collect_sheet_hyperlinks(&mut workbook, name) {
            extra_metadata.insert(format!("hyperlinks_{name}"), hyperlinks);
        }
    }

    let mut metadata = extract_metadata(&workbook, &sheet_names, office_metadata);
    metadata.extend(extra_metadata);
    Ok(ExcelWorkbook {
        sheets,
        metadata,
        revisions: None,
    })
}

/// Process a single XLSX sheet safely by pre-checking the bounding box.
///
/// This function streams cells to compute the actual bounding box without allocating
/// a full Range, then only creates the Range if the bounding box is within safe limits.
fn process_xlsx_sheet_safe<RS: Read + Seek>(
    workbook: &mut calamine::Xlsx<RS>,
    sheet_name: &str,
    warnings: &mut Vec<ProcessingWarning>,
) -> Result<ExcelSheet> {
    match classify_xlsx_sheet_shape(workbook, sheet_name)? {
        XlsxSheetShape::Empty => return Ok(empty_excel_sheet(sheet_name)),
        XlsxSheetShape::Dense => {}
        XlsxSheetShape::Sparse => {
            let (cells, row_min, row_max, col_min, col_max) = collect_xlsx_sheet_cells(workbook, sheet_name)?;
            if cells.is_empty() {
                return Ok(empty_excel_sheet(sheet_name));
            }
            let bounding_box = SparseSheetBoundingBox {
                row_min,
                row_max,
                col_min,
                col_max,
            };
            return process_sparse_sheet_from_cells(sheet_name, cells, bounding_box, warnings);
        }
    }

    let range = workbook
        .worksheet_range(sheet_name)
        .map_err(|e| XbergError::parsing(format!("Failed to parse sheet '{}': {}", sheet_name, e)))?;

    Ok(process_sheet(sheet_name, &range, warnings))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum XlsxSheetShape {
    Empty,
    Dense,
    Sparse,
}

fn classify_xlsx_sheet_shape<RS: Read + Seek>(
    workbook: &mut calamine::Xlsx<RS>,
    sheet_name: &str,
) -> Result<XlsxSheetShape> {
    let mut cell_reader = workbook
        .worksheet_cells_reader(sheet_name)
        .map_err(|e| XbergError::parsing(format!("Failed to read sheet '{}': {}", sheet_name, e)))?;
    let mut bounds: Option<(u32, u32, u32, u32)> = None;

    while let Ok(Some(cell)) = cell_reader.next_cell() {
        let (row, col) = cell.get_position();
        let (row_min, row_max, col_min, col_max) = bounds.get_or_insert((row, row, col, col));
        *row_min = (*row_min).min(row);
        *row_max = (*row_max).max(row);
        *col_min = (*col_min).min(col);
        *col_max = (*col_max).max(col);

        let row_count = u64::from(*row_max - *row_min + 1);
        let col_count = u64::from(*col_max - *col_min + 1);
        if row_count.saturating_mul(col_count) > MAX_BOUNDING_BOX_CELLS {
            return Ok(XlsxSheetShape::Sparse);
        }
    }

    Ok(if bounds.is_some() {
        XlsxSheetShape::Dense
    } else {
        XlsxSheetShape::Empty
    })
}

type XlsxOwnedCells = (Vec<((u32, u32), Data)>, u32, u32, u32, u32);

fn collect_xlsx_sheet_cells<RS: Read + Seek>(
    workbook: &mut calamine::Xlsx<RS>,
    sheet_name: &str,
) -> Result<XlsxOwnedCells> {
    let mut cell_reader = workbook
        .worksheet_cells_reader(sheet_name)
        .map_err(|e| XbergError::parsing(format!("Failed to read sheet '{}': {}", sheet_name, e)))?;
    let mut cells = Vec::new();
    let mut row_min = u32::MAX;
    let mut row_max = 0;
    let mut col_min = u32::MAX;
    let mut col_max = 0;

    while let Ok(Some(cell)) = cell_reader.next_cell() {
        let (row, col) = cell.get_position();
        row_min = row_min.min(row);
        row_max = row_max.max(row);
        col_min = col_min.min(col);
        col_max = col_max.max(col);
        cells.push(((row, col), owned_xlsx_cell_value(cell.get_value())));
    }

    Ok((cells, row_min, row_max, col_min, col_max))
}

fn owned_xlsx_cell_value(value: &DataRef<'_>) -> Data {
    match value {
        DataRef::Empty => Data::Empty,
        DataRef::String(value) => Data::String(value.clone()),
        DataRef::SharedString(value) => Data::String(value.to_string()),
        DataRef::Float(value) => Data::Float(*value),
        DataRef::Int(value) => Data::Int(*value),
        DataRef::Bool(value) => Data::Bool(*value),
        DataRef::DateTime(value) => Data::DateTime(*value),
        DataRef::DateTimeIso(value) => Data::DateTimeIso(value.clone()),
        DataRef::DurationIso(value) => Data::DurationIso(value.clone()),
        DataRef::Error(value) => Data::Error(value.clone()),
    }
}

fn empty_excel_sheet(sheet_name: &str) -> ExcelSheet {
    ExcelSheet {
        name: sheet_name.to_owned(),
        markdown: format!("## {}\n\n*Empty sheet*", sheet_name),
        row_count: 0,
        col_count: 0,
        cell_count: 0,
        table_cells: None,
    }
}

/// A sparse sheet's cell bounding box: the inclusive min/max row and column indices spanned
/// by its non-empty cells, as streamed by `collect_xlsx_sheet_cells`. Grouped into one type
/// so [`process_sparse_sheet_from_cells`] takes a bounded parameter count — the four values
/// are always read and passed together, never independently. ~keep
#[derive(Clone, Copy)]
struct SparseSheetBoundingBox {
    row_min: u32,
    row_max: u32,
    col_min: u32,
    col_max: u32,
}

/// The deduplicated, sorted non-empty columns/rows of a sparse sheet, plus a lookup map from
/// `(row, col)` to cell value — built once from the sheet's collected cells and shared by the
/// header row, body rows, and empty-sheet check in [`process_sparse_sheet_from_cells`].
struct SparseSheetIndex<'a> {
    cols: Vec<u32>,
    rows: Vec<u32>,
    cell_map: HashMap<(u32, u32), &'a Data>,
}

/// Build a [`SparseSheetIndex`] from a sparse sheet's collected cells, dropping empty cells
/// (they contribute to neither the column/row sets nor the lookup map).
fn index_sparse_sheet_cells(cells: &[((u32, u32), Data)]) -> SparseSheetIndex<'_> {
    let mut col_set = std::collections::BTreeSet::new();
    let mut row_set = std::collections::BTreeSet::new();
    let mut cell_map: HashMap<(u32, u32), &Data> = HashMap::with_capacity(cells.len());

    for ((row, col), data) in cells {
        if !matches!(data, Data::Empty) {
            col_set.insert(*col);
            row_set.insert(*row);
            cell_map.insert((*row, *col), data);
        }
    }

    SparseSheetIndex {
        cols: col_set.into_iter().collect(),
        rows: row_set.into_iter().collect(),
        cell_map,
    }
}

/// Write one markdown pipe-table row for `row` across `display_cols` (looking each cell up in
/// `cell_map`, defaulting to empty when absent), appending it to `markdown`, and return the
/// row's cell strings for `table_cells`. Shared by the header row and every body row.
fn write_sparse_sheet_row(
    markdown: &mut String,
    cell_map: &HashMap<(u32, u32), &Data>,
    row: u32,
    display_cols: &[u32],
) -> Vec<String> {
    let mut row_cells = Vec::with_capacity(display_cols.len());
    markdown.push_str("| ");
    for (i, &col) in display_cols.iter().enumerate() {
        if i > 0 {
            markdown.push_str(" | ");
        }
        let cell_str = cell_map
            .get(&(row, col))
            .map(|d| format_cell_to_string(d))
            .unwrap_or_default();
        escape_markdown_into(markdown, &cell_str);
        row_cells.push(cell_str);
    }
    markdown.push_str(" |\n");
    row_cells
}

/// A sparse sheet's actual non-empty row/column counts alongside how many of each were
/// actually displayed (after the `MAX_OUTPUT_ROWS`/`MAX_OUTPUT_COLS` caps), grouped into one
/// type so [`note_sparse_sheet_truncation`] takes a bounded parameter count.
struct SparseSheetDisplayCounts {
    total_rows: usize,
    total_cols: usize,
    display_rows: usize,
    display_cols: usize,
}

/// Append an in-markdown truncation notice and push a matching `ProcessingWarning`, if and
/// only if `counts` shows fewer rows or columns were displayed than the sheet actually has.
fn note_sparse_sheet_truncation(
    sheet_name: &str,
    counts: SparseSheetDisplayCounts,
    markdown: &mut String,
    warnings: &mut Vec<ProcessingWarning>,
) {
    let SparseSheetDisplayCounts {
        total_rows,
        total_cols,
        display_rows,
        display_cols,
    } = counts;
    if total_rows <= display_rows && total_cols <= display_cols {
        return;
    }

    write!(
        markdown,
        "\n*Truncated: showing {display_rows}x{display_cols} of {total_rows}x{total_cols} cells*\n"
    )
    .expect("write to String cannot fail");

    push_warning(
        warnings,
        "excel",
        format!(
            "Sheet '{sheet_name}' output truncated to {display_rows}x{display_cols} of \
             {total_rows}x{total_cols} non-empty rows/columns (internal sparse-sheet output cap)"
        ),
    );
}

/// Process a sparse sheet directly from collected cells without creating a full Range.
///
/// This is used when the bounding box would exceed MAX_BOUNDING_BOX_CELLS.
/// Instead of creating a dense Range, we generate a markdown pipe table from the sparse cells.
fn process_sparse_sheet_from_cells(
    sheet_name: &str,
    cells: Vec<((u32, u32), Data)>,
    bounding_box: SparseSheetBoundingBox,
    warnings: &mut Vec<ProcessingWarning>,
) -> Result<ExcelSheet> {
    let SparseSheetBoundingBox {
        row_min,
        row_max,
        col_min,
        col_max,
    } = bounding_box;
    let cell_count = cells.len();
    let bb_rows = (row_max - row_min + 1) as usize;
    let bb_cols = (col_max - col_min + 1) as usize;

    let SparseSheetIndex { cols, rows, cell_map } = index_sparse_sheet_cells(&cells);

    if cols.is_empty() || rows.is_empty() {
        let markdown = format!("## {}\n\n*Empty sheet*", sheet_name);
        return Ok(ExcelSheet {
            name: sheet_name.to_owned(),
            markdown,
            row_count: bb_rows,
            col_count: bb_cols,
            cell_count,
            table_cells: None,
        });
    }

    const MAX_OUTPUT_ROWS: usize = 1000;
    const MAX_OUTPUT_COLS: usize = 50;
    let display_rows = rows.len().min(MAX_OUTPUT_ROWS);
    let display_cols = cols.len().min(MAX_OUTPUT_COLS);
    let display_col_ids = &cols[..display_cols];

    let mut markdown = String::with_capacity(500 + cell_count * 20);
    let mut table_cells: Vec<Vec<String>> = Vec::with_capacity(display_rows + 1);

    write!(markdown, "## {}\n\n", sheet_name).expect("write to String cannot fail");

    table_cells.push(write_sparse_sheet_row(
        &mut markdown,
        &cell_map,
        rows[0],
        display_col_ids,
    ));

    markdown.push_str("| ");
    for i in 0..display_cols {
        if i > 0 {
            markdown.push_str(" | ");
        }
        markdown.push_str("---");
    }
    markdown.push_str(" |\n");

    for &row in rows.iter().skip(1).take(display_rows - 1) {
        table_cells.push(write_sparse_sheet_row(&mut markdown, &cell_map, row, display_col_ids));
    }

    let display_counts = SparseSheetDisplayCounts {
        total_rows: rows.len(),
        total_cols: cols.len(),
        display_rows,
        display_cols,
    };
    note_sparse_sheet_truncation(sheet_name, display_counts, &mut markdown, warnings);

    Ok(ExcelSheet {
        name: sheet_name.to_owned(),
        markdown,
        row_count: bb_rows,
        col_count: bb_cols,
        cell_count,
        table_cells: Some(table_cells),
    })
}

fn process_workbook<RS, R>(
    mut workbook: R,
    office_metadata: Option<HashMap<String, String>>,
    warnings: &mut Vec<ProcessingWarning>,
) -> Result<ExcelWorkbook>
where
    RS: std::io::Read + std::io::Seek,
    R: Reader<RS>,
    // Forwarded from `process_named_sheets`, which names the cause in its warning.
    R::Error: std::fmt::Display,
{
    let sheet_names = workbook.sheet_names();
    let (sheets, extra_metadata) = process_named_sheets(&mut workbook, &sheet_names, warnings);

    let mut metadata = extract_metadata(&workbook, &sheet_names, office_metadata);
    metadata.extend(extra_metadata);

    Ok(ExcelWorkbook {
        sheets,
        metadata,
        revisions: None,
    })
}

/// Read each named sheet's range and formulas from a generic calamine `Reader`
/// (xls/xlsb/ods — the XLSX path has its own bounding-box-aware
/// `process_xlsx_sheet_safe`). Split out from [`process_workbook`] so a sheet
/// name that the backend cannot resolve to a range can be exercised directly
/// in tests without needing a corrupt on-disk fixture (xberg-io/xberg#103):
/// pass a real, open workbook plus a sheet-name list containing a name the
/// backend doesn't recognize, and the failure is real `calamine::Reader`
/// behavior, not test-only scaffolding.
///
/// A worksheet range that fails to read is skipped with a warning naming the
/// sheet instead of silently dropped, mirroring the XLSX-side handling in
/// [`process_xlsx_workbook`].
fn process_named_sheets<RS, R>(
    workbook: &mut R,
    sheet_names: &[String],
    warnings: &mut Vec<ProcessingWarning>,
) -> (Vec<ExcelSheet>, HashMap<String, String>)
where
    RS: std::io::Read + std::io::Seek,
    R: Reader<RS>,
    // The warning names the failure cause, so the reader's error type has to be
    // renderable. Every concrete calamine reader (Xlsx, Xls, Ods) satisfies this.
    R::Error: std::fmt::Display,
{
    let mut sheets = Vec::with_capacity(sheet_names.len());
    let mut extra_metadata: HashMap<String, String> = HashMap::new();

    for name in sheet_names {
        match workbook.worksheet_range(name) {
            Ok(range) => sheets.push(process_sheet(name, &range, warnings)),
            Err(e) => {
                tracing::warn!("Failed to read worksheet range '{}': {}", name, e);
                push_warning(
                    warnings,
                    "excel",
                    format!("Sheet '{name}' worksheet range could not be read and was skipped ({e})"),
                );
            }
        }

        if let Some(formulas) = collect_sheet_formulas(workbook, name) {
            extra_metadata.insert(format!("formulas_{name}"), formulas);
        }
    }

    (sheets, extra_metadata)
}

#[inline]
fn process_sheet(name: &str, range: &Range<Data>, warnings: &mut Vec<ProcessingWarning>) -> ExcelSheet {
    let (rows, cols) = range.get_size();
    let cell_count = range.used_cells().count();

    let estimated_capacity = 50 + (cols * 20) + (cell_count * 12);

    if rows == 0 || cols == 0 {
        let markdown = format!("## {}\n\n*Empty sheet*", name);
        ExcelSheet {
            name: name.to_owned(),
            markdown,
            row_count: rows,
            col_count: cols,
            cell_count,
            table_cells: None,
        }
    } else {
        let (markdown, table_cells) = generate_markdown_and_cells(name, range, estimated_capacity, warnings);
        ExcelSheet {
            name: name.to_owned(),
            markdown,
            row_count: rows,
            col_count: cols,
            cell_count,
            table_cells: Some(table_cells),
        }
    }
}

/// Minimum populated-cell count below which a sheet declaring an implausibly large row count
/// is treated as sparse-but-huge rather than genuinely large, and skipped (see
/// [`skip_if_extreme_sparse_dimensions`]).
const MIN_ACTUAL_CELLS_FOR_EXTREME_SHEET: usize = 10_000;

/// Check whether `range` declares far more rows than [`MAX_BOUNDING_BOX_CELLS`]-scale content
/// could plausibly need, with almost none of them actually populated — a shape that would
/// otherwise force an oversized allocation for almost no real content. Returns the sheet's
/// placeholder markdown/cells and records a matching warning when it should be skipped;
/// `None` when normal processing should proceed.
fn skip_if_extreme_sparse_dimensions(
    range: &Range<Data>,
    sheet_name: &str,
    warnings: &mut Vec<ProcessingWarning>,
) -> Option<(String, Vec<Vec<String>>)> {
    const MAX_REASONABLE_ROWS: usize = 100_000;

    let (declared_rows, _declared_cols) = range.get_size();
    if declared_rows <= MAX_REASONABLE_ROWS {
        return None;
    }

    let actual_cell_count = range.used_cells().count();
    if actual_cell_count >= MIN_ACTUAL_CELLS_FOR_EXTREME_SHEET {
        return None;
    }

    let result_capacity = 100 + sheet_name.len();
    let mut result = String::with_capacity(result_capacity);
    write!(
        result,
        "## {}\n\n*Sheet has extreme declared dimensions ({} rows) with minimal actual data ({} cells). Skipping to prevent OOM.*",
        sheet_name, declared_rows, actual_cell_count
    ).unwrap();
    push_warning(
        warnings,
        "excel",
        format!(
            "Sheet '{sheet_name}' declared {declared_rows} rows but only {actual_cell_count} cells \
             contain data; sheet content was skipped to avoid excessive memory use"
        ),
    );
    Some((result, Vec::new()))
}

/// Estimate the markdown-string capacity to pre-allocate for a dense sheet table: an exact
/// estimate from the header/body shape, the range's own general table-size hint
/// (`capacity::estimate_table_markdown_capacity`), and the caller-provided reuse hint —
/// whichever is largest, so growth reallocation stays rare.
fn estimate_dense_sheet_capacity(sheet_name: &str, row_count: usize, header_len: usize, caller_hint: usize) -> usize {
    let table_capacity = capacity::estimate_table_markdown_capacity(row_count, header_len);

    let mut exact_size = 16 + sheet_name.len();
    exact_size += 2 + (header_len * 2);
    exact_size += header_len * 10;
    exact_size += 5 + (header_len * 5);
    exact_size += (row_count - 1) * (5 + header_len * 15);

    exact_size.max(table_capacity).max(caller_hint)
}

/// Generate both markdown and extracted cells in a single pass.
///
/// This function produces both the markdown representation and the structured
/// cell data simultaneously, avoiding the expensive markdown re-parsing that
/// was previously done in `sheets_to_tables()`.
///
/// Returns (markdown, table_cells) where table_cells is a 2D vector of strings.
fn generate_markdown_and_cells(
    sheet_name: &str,
    range: &Range<Data>,
    capacity: usize,
    warnings: &mut Vec<ProcessingWarning>,
) -> (String, Vec<Vec<String>>) {
    if let Some(skipped) = skip_if_extreme_sparse_dimensions(range, sheet_name, warnings) {
        return skipped;
    }

    let rows: Vec<_> = range.rows().collect();
    if rows.is_empty() {
        let result_capacity = 50 + sheet_name.len();
        let mut result = String::with_capacity(result_capacity);
        write!(result, "## {}\n\n*No data*", sheet_name).unwrap();
        return (result, Vec::new());
    }

    let header = &rows[0];
    let header_len = header.len();
    let row_count = rows.len();

    let mut markdown = String::with_capacity(estimate_dense_sheet_capacity(
        sheet_name, row_count, header_len, capacity,
    ));
    let mut cells: Vec<Vec<String>> = Vec::with_capacity(row_count);

    write!(markdown, "## {}\n\n", sheet_name).unwrap();

    let mut header_cells = Vec::with_capacity(header_len);
    markdown.push_str("| ");
    for (i, cell) in header.iter().enumerate() {
        if i > 0 {
            markdown.push_str(" | ");
        }
        let cell_str = format_cell_to_string(cell);
        escape_markdown_into(&mut markdown, &cell_str);
        header_cells.push(cell_str);
    }
    markdown.push_str(" |\n");
    cells.push(header_cells);

    markdown.push_str("| ");
    for i in 0..header_len {
        if i > 0 {
            markdown.push_str(" | ");
        }
        markdown.push_str("---");
    }
    markdown.push_str(" |\n");

    for row in rows.iter().skip(1) {
        let mut row_cells = Vec::with_capacity(header_len);
        markdown.push_str("| ");
        for i in 0..header_len {
            if i > 0 {
                markdown.push_str(" | ");
            }
            let cell_str = if let Some(cell) = row.get(i) {
                let cell_str = format_cell_to_string(cell);
                escape_markdown_into(&mut markdown, &cell_str);
                cell_str
            } else {
                String::new()
            };
            row_cells.push(cell_str);
        }
        markdown.push_str(" |\n");
        cells.push(row_cells);
    }

    (markdown, cells)
}

/// Convert a Data cell to its string representation.
///
/// This helper function is shared between markdown generation and cell extraction
/// to ensure byte-identical output.
///
/// Float values that are whole numbers (e.g. 1.0, 42.0) are formatted without the
/// trailing decimal point (e.g. "1", "42") so that numeric ground-truth comparisons
/// produce correct F1 scores; see [`format_excel_float`].
#[inline]
fn format_cell_to_string(data: &Data) -> String {
    match data {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => format_excel_float(*f),
        Data::Int(i) => format!("{}", i),
        Data::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Data::DateTime(dt) => datetime::format_excel_datetime(dt),
        Data::Error(e) => format!("#ERR: {:?}", e),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => format!("DURATION: {}", s),
    }
}

/// Format a float with at most the 15 significant digits Excel keeps.
///
/// A formula result is the double the calculation produced, with its binary noise past
/// those digits: `=0.1+0.2` is `0.30000000000000004` and `=100*1.19` is
/// `118.99999999999999`, which Excel shows as `0.3` and `119`. Rounding to 15 significant
/// digits before printing the shortest form drops that noise and keeps every digit a typed
/// value can have. Rust's `{}` already prints a whole number without a trailing decimal point.
fn format_excel_float(value: f64) -> String {
    if !value.is_finite() {
        return format!("{value}");
    }
    let rounded = format!("{value:.14e}").parse::<f64>().unwrap_or(value);
    format!("{rounded}")
}

/// Stand-in for a line break inside a spreadsheet cell (Alt+Enter). A raw
/// newline would end the markdown table row, splitting one cell's content
/// across two rows (xberg-io/xberg#163).
const CELL_LINE_BREAK: &str = "<br>";

/// Push `s` into `buffer`, escaping everything that would let a spreadsheet cell
/// break out of its markdown table cell.
///
/// Deliberately *not* routed through the shared `rendering::common` table
/// renderer: this path streams a whole sheet (heading, table, truncation notice)
/// into one buffer rather than rendering a `&[Vec<String>]` grid, and
/// spreadsheet text is literal, so a `\` typed into a cell is doubled to survive
/// markdown unescaping. The shared renderer must not double backslashes because
/// its inputs (DOCX, HTML, PDF) already carry markdown-significant text.
///
/// Escapes `|`, doubles `\`, and turns any line break into [`CELL_LINE_BREAK`].
#[inline]
fn escape_markdown_into(buffer: &mut String, s: &str) {
    if !s.bytes().any(|b| matches!(b, b'|' | b'\\' | b'\n' | b'\r')) {
        buffer.push_str(s);
        return;
    }
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '|' => buffer.push_str("\\|"),
            '\\' => buffer.push_str("\\\\"),
            '\r' => {
                // Consume the LF of a CRLF pair so it yields one break, not two.
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                buffer.push_str(CELL_LINE_BREAK);
            }
            '\n' => buffer.push_str(CELL_LINE_BREAK),
            _ => buffer.push(ch),
        }
    }
}

/// Render a zero-based column index as spreadsheet column letters, e.g.
/// `0 -> "A"`, `25 -> "Z"`, `26 -> "AA"`.
fn column_letters(mut col: u32) -> String {
    let mut letters = Vec::new();
    loop {
        let rem = (col % 26) as u8;
        letters.push(b'A' + rem);
        if col < 26 {
            break;
        }
        col = col / 26 - 1;
    }
    letters.reverse();
    // SAFETY-free: every byte pushed above is in b'A'..=b'Z', which is valid UTF-8. ~keep
    String::from_utf8(letters).unwrap_or_default()
}

/// Render a zero-based `(row, col)` position as an A1-style cell reference
/// relative to the enumerated range (matching the convention already used by
/// `extractors::excel::scan_for_dde_warnings`, which reports positions
/// relative to the extracted `table_cells` grid rather than absolute sheet
/// coordinates).
fn cell_reference(row: u32, col: u32) -> String {
    format!("{}{}", column_letters(col), row + 1)
}

/// Collect non-empty formulas from a sheet as `"<cell_ref>=<formula>"` entries
/// joined by `"; "`. Works across every calamine `Reader` (xlsx/xls/xlsb/ods)
/// since `worksheet_formula` is part of the shared trait. Returns `None` when
/// the sheet has no formulas or the backend cannot report them.
fn collect_sheet_formulas<RS, R>(workbook: &mut R, sheet_name: &str) -> Option<String>
where
    RS: Read + Seek,
    R: Reader<RS>,
{
    let range = workbook.worksheet_formula(sheet_name).ok()?;
    let mut entries = Vec::new();

    'outer: for (row_idx, row) in range.rows().enumerate() {
        for (col_idx, formula) in row.iter().enumerate() {
            if formula.is_empty() {
                continue;
            }
            entries.push(format!(
                "{}={}",
                cell_reference(row_idx as u32, col_idx as u32),
                formula
            ));
            if entries.len() >= MAX_METADATA_ENTRIES_PER_SHEET {
                break 'outer;
            }
        }
    }

    if entries.is_empty() {
        None
    } else {
        Some(entries.join("; "))
    }
}

/// Collect hyperlinks from an XLSX sheet as `"<anchor>=<target>"` entries
/// joined by `"; "`. XLSX-only: hyperlink relationships are not part of the
/// shared calamine `Reader` trait. Returns `None` when the sheet has no
/// hyperlinks or the relationships cannot be read.
fn collect_sheet_hyperlinks<RS: Read + Seek>(workbook: &mut calamine::Xlsx<RS>, sheet_name: &str) -> Option<String> {
    let hyperlinks = workbook.hyperlinks_by_sheet_name(sheet_name).ok()?;
    if hyperlinks.is_empty() {
        return None;
    }

    let entries: Vec<String> = hyperlinks
        .iter()
        .take(MAX_METADATA_ENTRIES_PER_SHEET)
        .map(|link| {
            let target = link.target.as_deref().or(link.location.as_deref()).unwrap_or("");
            let anchor = cell_reference(link.range.start.0, link.range.start.1);
            format!("{anchor}={target}")
        })
        .collect();

    Some(entries.join("; "))
}

fn extract_metadata<RS, R>(
    workbook: &R,
    sheet_names: &[String],
    office_metadata: Option<HashMap<String, String>>,
) -> HashMap<String, String>
where
    RS: std::io::Read + std::io::Seek,
    R: Reader<RS>,
{
    let mut metadata = HashMap::with_capacity(4);

    let sheet_count = sheet_names.len();
    metadata.insert("sheet_count".to_owned(), sheet_count.to_string());

    let sheet_names_str = if sheet_count <= 5 {
        sheet_names.join(", ")
    } else {
        let mut result = String::with_capacity(100);
        for (i, name) in sheet_names.iter().take(5).enumerate() {
            if i > 0 {
                result.push_str(", ");
            }
            result.push_str(name);
        }
        write!(result, ", ... ({} total)", sheet_count).unwrap();
        result
    };
    metadata.insert("sheet_names".to_owned(), sheet_names_str);

    let _workbook_metadata = workbook.metadata();

    let defined_names = workbook.defined_names();
    if !defined_names.is_empty() {
        let joined = defined_names
            .iter()
            .map(|(name, formula)| format!("{name}={formula}"))
            .collect::<Vec<_>>()
            .join("; ");
        metadata.insert("defined_names".to_owned(), joined);
    }

    let hidden_sheets: Vec<&str> = workbook
        .sheets_metadata()
        .iter()
        .filter(|sheet| !matches!(sheet.visible, SheetVisible::Visible))
        .map(|sheet| sheet.name.as_str())
        .collect();
    if !hidden_sheets.is_empty() {
        metadata.insert("hidden_sheets".to_owned(), hidden_sheets.join(", "));
    }

    if let Some(office_meta) = office_metadata {
        for (key, value) in office_meta {
            metadata.insert(key, value);
        }
    }

    metadata
}

/// Convert an Excel workbook to plain text (space-separated cells, one row per line).
///
/// Each sheet is separated by a blank line. Sheet names are included as headers.
/// This produces text suitable for quality scoring against ground truth.
#[cfg_attr(alef, alef(skip))]
pub fn excel_to_text(workbook: &ExcelWorkbook) -> String {
    let mut result = String::new();

    for (i, sheet) in workbook.sheets.iter().enumerate() {
        if i > 0 {
            result.push_str("\n\n");
        }

        if let Some(cells) = &sheet.table_cells {
            for (row_idx, row) in cells.iter().enumerate() {
                if row_idx > 0 {
                    result.push('\n');
                }
                let line: String = row
                    .iter()
                    .map(|cell| cell.trim())
                    .filter(|cell| !cell.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                result.push_str(&line);
            }
        }
    }

    result
}

/// Render all sheets in an [`ExcelWorkbook`] as a single Markdown string.
///
/// Sheets are separated by double newlines; each sheet's pre-rendered markdown
/// is trimmed of trailing whitespace before joining.
#[cfg_attr(alef, alef(skip))]
pub fn excel_to_markdown(workbook: &ExcelWorkbook) -> String {
    let total_capacity: usize = workbook.sheets.iter().map(|sheet| sheet.markdown.len() + 2).sum();

    let mut result = String::with_capacity(total_capacity);

    for (i, sheet) in workbook.sheets.iter().enumerate() {
        if i > 0 {
            result.push_str("\n\n");
        }
        let sheet_content = sheet.markdown.trim_end();
        result.push_str(sheet_content);
    }

    result
}

#[cfg(test)]
mod tests;
