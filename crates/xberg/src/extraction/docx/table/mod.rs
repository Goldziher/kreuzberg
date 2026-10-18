//! DOCX table property types and parsing for Word document extraction.
//!
//! This module provides comprehensive support for parsing table-level, row-level, and cell-level
//! properties from OOXML `<w:tblPr>`, `<w:trPr>`, and `<w:tcPr>` elements using streaming XML parsing.

use crate::extractors::security::{SecurityBudget, SecurityError};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use serde::{Deserialize, Serialize};

/// Table-level properties from `<w:tblPr>`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(alef, alef(skip))]
pub struct TableProperties {
    /// Style ID of the table style applied to this table.
    pub style_id: Option<String>,
    /// Total table width specification.
    pub width: Option<TableWidth>,
    /// Horizontal alignment: `"left"`, `"center"`, or `"right"`.
    pub alignment: Option<String>,
    /// Table layout algorithm: `"fixed"` or `"autofit"`.
    pub layout: Option<String>,
    /// Conditional formatting flags for header/banded rows/columns.
    pub look: Option<TableLook>,
    /// Table outer and inner border definitions.
    pub borders: Option<TableBorders>,
    /// Default cell margins applied to all cells in the table.
    pub cell_margins: Option<CellMargins>,
    /// Table indentation from the leading margin.
    pub indent: Option<TableWidth>,
    /// Table caption text (from `<w:tblCaption>`).
    pub caption: Option<String>,
}

/// Width specification used for tables and cells.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableWidth {
    /// Numeric width value; interpretation depends on `width_type`.
    pub value: i32,
    /// Width unit: `"dxa"` (twips), `"pct"` (50ths of a percent), `"auto"`, or `"nil"`.
    pub width_type: String,
}

/// Table look bitmask/flags controlling conditional formatting bands.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(alef, alef(skip))]
pub struct TableLook {
    /// Apply first-row conditional formatting.
    pub first_row: bool,
    /// Apply last-row conditional formatting.
    pub last_row: bool,
    /// Apply first-column conditional formatting.
    pub first_column: bool,
    /// Apply last-column conditional formatting.
    pub last_column: bool,
    /// Suppress horizontal banding.
    pub no_h_band: bool,
    /// Suppress vertical banding.
    pub no_v_band: bool,
}

/// Borders for a table (6 borders: top, bottom, left, right, insideH, insideV).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(alef, alef(skip))]
pub struct TableBorders {
    /// Top outer border.
    pub top: Option<BorderStyle>,
    /// Bottom outer border.
    pub bottom: Option<BorderStyle>,
    /// Left outer border.
    pub left: Option<BorderStyle>,
    /// Right outer border.
    pub right: Option<BorderStyle>,
    /// Horizontal inner borders between rows.
    pub inside_h: Option<BorderStyle>,
    /// Vertical inner borders between columns.
    pub inside_v: Option<BorderStyle>,
}

/// A single border specification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(alef, alef(skip))]
pub struct BorderStyle {
    /// Border line style: `"single"`, `"double"`, `"dashed"`, `"dotted"`, `"none"`, etc.
    pub style: String,
    /// Border thickness in eighths of a point.
    pub size: Option<i32>,
    /// Border color as a hex RGB string (e.g. `"2F5496"`) or `"auto"`.
    pub color: Option<String>,
    /// Spacing between the border and the cell contents in points.
    pub space: Option<i32>,
}

/// Cell margins (used for both table-level defaults and per-cell overrides).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(alef, alef(skip))]
pub struct CellMargins {
    /// Top cell margin in twips.
    pub top: Option<i32>,
    /// Bottom cell margin in twips.
    pub bottom: Option<i32>,
    /// Left cell margin in twips.
    pub left: Option<i32>,
    /// Right cell margin in twips.
    pub right: Option<i32>,
}

/// Row-level properties from `<w:trPr>`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(alef, alef(skip))]
pub struct RowProperties {
    /// Row height in twips.
    pub height: Option<i32>,
    /// Height rule: `"auto"`, `"atLeast"`, or `"exact"`.
    pub height_rule: Option<String>,
    /// Whether this row acts as a repeating table header.
    pub is_header: bool,
    /// Whether this row may be split across a page break.
    pub cant_split: bool,
}

/// Grid columns a row leaves empty before its first cell (`w:gridBefore`) and after its
/// last (`w:gridAfter`). Kept out of [`RowProperties`] so its public shape stays as is;
/// the parser turns these into padding cells.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct RowGridSkips {
    pub(crate) before: u32,
    pub(crate) after: u32,
}

/// Cell-level properties from `<w:tcPr>`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(alef, alef(skip))]
pub struct CellProperties {
    /// Cell width specification.
    pub width: Option<TableWidth>,
    /// Number of grid columns this cell spans (default 1).
    pub grid_span: Option<u32>,
    /// Vertical merge state for this cell.
    pub v_merge: Option<VerticalMerge>,
    /// Per-cell border overrides.
    pub borders: Option<CellBorders>,
    /// Cell background shading.
    pub shading: Option<CellShading>,
    /// Per-cell margin overrides.
    pub margins: Option<CellMargins>,
    /// Vertical text alignment: `"top"`, `"center"`, or `"bottom"`.
    pub vertical_align: Option<String>,
    /// Text direction: `"lrTb"`, `"tbRl"`, or `"btLr"`.
    pub text_direction: Option<String>,
    /// Whether cell content wraps across lines.
    pub no_wrap: bool,
}

/// Vertical merge state for a table cell.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerticalMerge {
    /// This cell starts a new vertically-merged group.
    Restart,
    /// This cell continues a vertically-merged group started above.
    Continue,
}

/// Per-cell borders (4 sides).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(alef, alef(skip))]
pub struct CellBorders {
    /// Top cell border.
    pub top: Option<BorderStyle>,
    /// Bottom cell border.
    pub bottom: Option<BorderStyle>,
    /// Left (start in LTR) cell border.
    pub left: Option<BorderStyle>,
    /// Right (end in LTR) cell border.
    pub right: Option<BorderStyle>,
}

/// Cell shading/background fill.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(alef, alef(skip))]
pub struct CellShading {
    /// Background fill color as hex RGB or `"auto"`.
    pub fill: Option<String>,
    /// Pattern foreground color.
    pub color: Option<String>,
    /// Pattern type: `"clear"`, `"solid"`, `"pct10"`, etc.
    pub val: Option<String>,
}

/// Column widths from `<w:tblGrid>`.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TableGrid {
    /// Ordered list of column widths in twips.
    pub columns: Vec<i32>,
}

/// Parse table-level properties from streaming XML reader.
///
/// Expects the reader to be positioned just after the `<w:tblPr>` start tag.
/// Reads all child elements until the matching `</w:tblPr>` end tag.
///
/// Threads `budget` through every event this function (and the delegating
/// helpers it calls) reads, so nesting inside `w:tblPr` is measured against
/// the caller's depth cap instead of passing through unaccounted (GH#384).
/// Apply a `<w:tblPr>` child property whose value comes entirely from the element's own
/// attributes -- handled identically whether the element arrives as `Event::Start` or
/// `Event::Empty`. `tblBorders` and `tblCellMar` nest their own children instead and are
/// handled by the caller.
fn apply_simple_table_property(props: &mut TableProperties, e: &BytesStart) {
    match e.local_name().as_ref() {
        "tblStyle" => props.style_id = get_attribute(e, "val"),
        "tblW" => props.width = parse_width_element(e),
        "jc" => props.alignment = get_attribute(e, "val"),
        "tblLayout" => props.layout = get_attribute(e, "type"),
        "tblLook" => props.look = Some(parse_table_look(e)),
        "tblInd" => props.indent = parse_width_element(e),
        "tblCaption" => props.caption = get_attribute(e, "val"),
        _ => {}
    }
}

pub(crate) fn parse_table_properties(
    reader: &mut Reader<&[u8]>,
    budget: &mut SecurityBudget,
) -> Result<TableProperties, SecurityError> {
    let mut props = TableProperties::default();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                budget.enter()?;
                match e.local_name().as_ref() {
                    "tblBorders" => {
                        props.borders = Some(parse_table_borders(reader));
                        // `parse_table_borders` reads through its own `</w:tblBorders>`
                        // without touching `budget`; refund the enter above or every
                        // `tblBorders` child leaks one depth level.
                        budget.leave();
                    }
                    "tblCellMar" => {
                        props.cell_margins = Some(parse_cell_margins_element(reader));
                        // Same as `tblBorders`: consumes its own end tag.
                        budget.leave();
                    }
                    _ => apply_simple_table_property(&mut props, &e),
                }
                buf.clear();
            }
            Ok(Event::Empty(e)) => {
                match e.local_name().as_ref() {
                    "tblBorders" => props.borders = Some(TableBorders::default()),
                    "tblCellMar" => props.cell_margins = Some(CellMargins::default()),
                    _ => apply_simple_table_property(&mut props, &e),
                }
                buf.clear();
            }
            Ok(Event::End(e)) => {
                budget.leave();
                if e.local_name().as_ref() == "tblPr" {
                    break;
                }
                buf.clear();
            }
            Ok(Event::Eof) => break,
            _ => {
                buf.clear();
            }
        }
    }

    Ok(props)
}

/// Parse row-level properties from streaming XML reader, with the row's grid skips.
///
/// Expects the reader to be positioned just after the `<w:trPr>` start tag.
///
/// Threads `budget` through every event so nesting inside `w:trPr` is
/// measured against the caller's depth cap (GH#384).
pub(crate) fn parse_row_properties(
    reader: &mut Reader<&[u8]>,
    budget: &mut SecurityBudget,
) -> Result<(RowProperties, RowGridSkips), SecurityError> {
    let mut props = RowProperties::default();
    let mut skips = RowGridSkips::default();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                budget.enter()?;
                let local_name = e.local_name();
                match local_name.as_ref() {
                    "trHeight" => {
                        props.height = get_attribute_int(&e, "val");
                        props.height_rule = get_attribute(&e, "hRule");
                    }
                    "tblHeader" => {
                        props.is_header = is_toggle_on(&e);
                    }
                    "cantSplit" => {
                        props.cant_split = is_toggle_on(&e);
                    }
                    "gridBefore" => skips.before = validate_grid_span(get_attribute_u32(&e, "val")).unwrap_or(0),
                    "gridAfter" => skips.after = validate_grid_span(get_attribute_u32(&e, "val")).unwrap_or(0),
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::Empty(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    "trHeight" => {
                        props.height = get_attribute_int(&e, "val");
                        props.height_rule = get_attribute(&e, "hRule");
                    }
                    "tblHeader" => {
                        props.is_header = is_toggle_on(&e);
                    }
                    "cantSplit" => {
                        props.cant_split = is_toggle_on(&e);
                    }
                    "gridBefore" => skips.before = validate_grid_span(get_attribute_u32(&e, "val")).unwrap_or(0),
                    "gridAfter" => skips.after = validate_grid_span(get_attribute_u32(&e, "val")).unwrap_or(0),
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::End(e)) => {
                budget.leave();
                if e.local_name().as_ref() == "trPr" {
                    break;
                }
                buf.clear();
            }
            Ok(Event::Eof) => break,
            _ => {
                buf.clear();
            }
        }
    }

    Ok((props, skips))
}

/// Maximum plausible `w:gridSpan` value for a single table cell.
///
/// Word's UI caps a table at 63 columns; this ceiling is set well above that (1,024) so
/// wide, tooling-generated tables (spreadsheet exports and similar) still round-trip,
/// while a value that could not correspond to any real document is rejected. Every unit
/// of `gridSpan` becomes one cloned copy of the cell's text at each of the four
/// consumption sites (`extractors/docx.rs` x2, `extraction/docx/parser.rs` x2), so an
/// unclamped `w:val="4294967295"` would attempt roughly 4 billion `String` clones.
const MAX_CELL_GRID_SPAN: u32 = 1024;

/// Reject a `w:gridSpan` value outside the plausible `1..=MAX_CELL_GRID_SPAN` range.
/// A row's `w:gridBefore`/`w:gridAfter` column counts go through the same check.
///
/// Rather than silently truncating a corrupt value down into range (which would
/// fabricate a plausible-looking but wrong column count), an out-of-range span is
/// dropped back to `None`, so the cell falls back to its unspanned default of 1 column
/// (`unwrap_or(1)` at every consumption site). This mirrors `parse_width_element`'s
/// existing convention for this module: an unparseable/invalid property value degrades
/// to "not specified" rather than erroring the whole document.
fn validate_grid_span(span: Option<u32>) -> Option<u32> {
    span.filter(|&value| (1..=MAX_CELL_GRID_SPAN).contains(&value))
}

/// Parse cell-level properties from streaming XML reader.
///
/// Expects the reader to be positioned just after the `<w:tcPr>` start tag.
///
/// Threads `budget` through every event this function (and the delegating
/// helpers it calls) reads, so nesting inside `w:tcPr` is measured against
/// the caller's depth cap instead of passing through unaccounted (GH#384).
/// Apply a `<w:tcPr>` child property whose value comes entirely from the element's own
/// attributes -- handled identically whether the element arrives as `Event::Start` or
/// `Event::Empty`. `tcBorders` and `tcMar` nest their own children instead and are handled
/// by the caller.
fn apply_simple_cell_property(props: &mut CellProperties, e: &BytesStart) {
    match e.local_name().as_ref() {
        "tcW" => props.width = parse_width_element(e),
        "gridSpan" => props.grid_span = validate_grid_span(get_attribute_u32(e, "val")),
        "vMerge" => props.v_merge = Some(parse_vmerge(e)),
        "shd" => props.shading = Some(parse_cell_shading(e)),
        "vAlign" => props.vertical_align = get_attribute(e, "val"),
        "textDirection" => props.text_direction = get_attribute(e, "val"),
        "noWrap" => props.no_wrap = is_toggle_on(e),
        _ => {}
    }
}

pub(crate) fn parse_cell_properties(
    reader: &mut Reader<&[u8]>,
    budget: &mut SecurityBudget,
) -> Result<CellProperties, SecurityError> {
    let mut props = CellProperties::default();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                budget.enter()?;
                match e.local_name().as_ref() {
                    "tcBorders" => {
                        props.borders = Some(parse_cell_borders(reader));
                        // `parse_cell_borders` reads through its own `</w:tcBorders>`
                        // without touching `budget`; refund the enter above.
                        budget.leave();
                    }
                    "tcMar" => {
                        props.margins = Some(parse_cell_margins_element(reader));
                        // `parse_cell_margins_element` reads through its own
                        // `</w:tcMar>` without touching `budget`; refund the enter above.
                        budget.leave();
                    }
                    _ => apply_simple_cell_property(&mut props, &e),
                }
                buf.clear();
            }
            Ok(Event::Empty(e)) => {
                match e.local_name().as_ref() {
                    "tcBorders" => props.borders = Some(CellBorders::default()),
                    "tcMar" => props.margins = Some(CellMargins::default()),
                    _ => apply_simple_cell_property(&mut props, &e),
                }
                buf.clear();
            }
            Ok(Event::End(e)) => {
                budget.leave();
                if e.local_name().as_ref() == "tcPr" {
                    break;
                }
                buf.clear();
            }
            Ok(Event::Eof) => break,
            _ => {
                buf.clear();
            }
        }
    }

    Ok(props)
}

/// Parse table grid (column widths) from streaming XML reader.
///
/// Expects the reader to be positioned just after the `<w:tblGrid>` start tag.
///
/// Threads `budget` through every event so nesting inside `w:tblGrid` is
/// measured against the caller's depth cap (GH#384).
pub(crate) fn parse_table_grid(
    reader: &mut Reader<&[u8]>,
    budget: &mut SecurityBudget,
) -> Result<TableGrid, SecurityError> {
    let mut grid = TableGrid::default();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                budget.enter()?;
                if e.local_name().as_ref() == "gridCol"
                    && let Some(width) = get_attribute_int(&e, "w")
                {
                    grid.columns.push(width);
                }
                buf.clear();
            }
            Ok(Event::Empty(e)) => {
                if e.local_name().as_ref() == "gridCol"
                    && let Some(width) = get_attribute_int(&e, "w")
                {
                    grid.columns.push(width);
                }
                buf.clear();
            }
            Ok(Event::End(e)) => {
                budget.leave();
                if e.local_name().as_ref() == "tblGrid" {
                    break;
                }
                buf.clear();
            }
            Ok(Event::Eof) => break,
            _ => {
                buf.clear();
            }
        }
    }

    Ok(grid)
}

/// Helper: Check if an OOXML on/off toggle element is enabled.
/// Handles `<w:foo/>` (on), `<w:foo w:val="1"/>` (on), `<w:foo w:val="0"/>` (off),
/// `<w:foo w:val="true"/>` (on), `<w:foo w:val="false"/>` (off).
fn is_toggle_on(e: &BytesStart) -> bool {
    !matches!(
        get_attribute(e, "val").as_deref(),
        Some("0") | Some("false") | Some("off")
    )
}

/// Helper: Parse border element from attributes.
fn parse_border_element(e: &BytesStart) -> BorderStyle {
    BorderStyle {
        style: get_attribute(e, "val").unwrap_or_default(),
        size: get_attribute_int(e, "sz"),
        color: get_attribute(e, "color"),
        space: get_attribute_int(e, "space"),
    }
}

/// Helper: Parse width element from attributes.
fn parse_width_element(e: &BytesStart) -> Option<TableWidth> {
    get_attribute_int(e, "w").map(|value| TableWidth {
        value,
        width_type: get_attribute(e, "type").unwrap_or_default(),
    })
}

/// Helper: Parse table look from attributes.
/// Handles both OOXML 2012+ individual boolean attributes and legacy hex bitmask.
fn parse_table_look(e: &BytesStart) -> TableLook {
    let mut look = TableLook::default();

    let has_individual = get_attribute(e, "firstRow").is_some()
        || get_attribute(e, "lastRow").is_some()
        || get_attribute(e, "firstColumn").is_some()
        || get_attribute(e, "lastColumn").is_some()
        || get_attribute(e, "noHBand").is_some()
        || get_attribute(e, "noVBand").is_some();

    if has_individual {
        look.first_row = get_attribute(e, "firstRow").as_deref() == Some("1");
        look.last_row = get_attribute(e, "lastRow").as_deref() == Some("1");
        look.first_column = get_attribute(e, "firstColumn").as_deref() == Some("1");
        look.last_column = get_attribute(e, "lastColumn").as_deref() == Some("1");
        look.no_h_band = get_attribute(e, "noHBand").as_deref() == Some("1");
        look.no_v_band = get_attribute(e, "noVBand").as_deref() == Some("1");
    } else if let Some(val_str) = get_attribute(e, "val")
        && let Ok(mask) = i32::from_str_radix(&val_str, 16)
    {
        look.first_row = (mask & 0x0020) != 0;
        look.last_row = (mask & 0x0040) != 0;
        look.first_column = (mask & 0x0080) != 0;
        look.last_column = (mask & 0x0100) != 0;
        look.no_h_band = (mask & 0x0200) != 0;
        look.no_v_band = (mask & 0x0400) != 0;
    }

    look
}

/// Helper: Parse vertical merge state.
fn parse_vmerge(e: &BytesStart) -> VerticalMerge {
    match get_attribute(e, "val") {
        Some(val) if val == "restart" => VerticalMerge::Restart,
        _ => VerticalMerge::Continue,
    }
}

/// Helper: Parse cell shading from attributes.
fn parse_cell_shading(e: &BytesStart) -> CellShading {
    CellShading {
        fill: get_attribute(e, "fill"),
        color: get_attribute(e, "color"),
        val: get_attribute(e, "val"),
    }
}

/// Helper: Parse table borders container element.
fn parse_table_borders(reader: &mut Reader<&[u8]>) -> TableBorders {
    let mut borders = TableBorders::default();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    "top" => {
                        borders.top = Some(parse_border_element(&e));
                    }
                    "bottom" => {
                        borders.bottom = Some(parse_border_element(&e));
                    }
                    "left" | "start" => {
                        borders.left = Some(parse_border_element(&e));
                    }
                    "right" | "end" => {
                        borders.right = Some(parse_border_element(&e));
                    }
                    "insideH" => {
                        borders.inside_h = Some(parse_border_element(&e));
                    }
                    "insideV" => {
                        borders.inside_v = Some(parse_border_element(&e));
                    }
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::Empty(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    "top" => {
                        borders.top = Some(parse_border_element(&e));
                    }
                    "bottom" => {
                        borders.bottom = Some(parse_border_element(&e));
                    }
                    "left" | "start" => {
                        borders.left = Some(parse_border_element(&e));
                    }
                    "right" | "end" => {
                        borders.right = Some(parse_border_element(&e));
                    }
                    "insideH" => {
                        borders.inside_h = Some(parse_border_element(&e));
                    }
                    "insideV" => {
                        borders.inside_v = Some(parse_border_element(&e));
                    }
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == "tblBorders" {
                    break;
                }
                buf.clear();
            }
            Ok(Event::Eof) => break,
            _ => {
                buf.clear();
            }
        }
    }

    borders
}

/// Helper: Parse cell borders container element.
fn parse_cell_borders(reader: &mut Reader<&[u8]>) -> CellBorders {
    let mut borders = CellBorders::default();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    "top" => {
                        borders.top = Some(parse_border_element(&e));
                    }
                    "bottom" => {
                        borders.bottom = Some(parse_border_element(&e));
                    }
                    "left" | "start" => {
                        borders.left = Some(parse_border_element(&e));
                    }
                    "right" | "end" => {
                        borders.right = Some(parse_border_element(&e));
                    }
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::Empty(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    "top" => {
                        borders.top = Some(parse_border_element(&e));
                    }
                    "bottom" => {
                        borders.bottom = Some(parse_border_element(&e));
                    }
                    "left" | "start" => {
                        borders.left = Some(parse_border_element(&e));
                    }
                    "right" | "end" => {
                        borders.right = Some(parse_border_element(&e));
                    }
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == "tcBorders" {
                    break;
                }
                buf.clear();
            }
            Ok(Event::Eof) => break,
            _ => {
                buf.clear();
            }
        }
    }

    borders
}

/// Helper: Parse cell margins container element.
fn parse_cell_margins_element(reader: &mut Reader<&[u8]>) -> CellMargins {
    let mut margins = CellMargins::default();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    "top" => {
                        margins.top = get_attribute_int(&e, "w");
                    }
                    "bottom" => {
                        margins.bottom = get_attribute_int(&e, "w");
                    }
                    "left" | "start" => {
                        margins.left = get_attribute_int(&e, "w");
                    }
                    "right" | "end" => {
                        margins.right = get_attribute_int(&e, "w");
                    }
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::Empty(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    "top" => {
                        margins.top = get_attribute_int(&e, "w");
                    }
                    "bottom" => {
                        margins.bottom = get_attribute_int(&e, "w");
                    }
                    "left" | "start" => {
                        margins.left = get_attribute_int(&e, "w");
                    }
                    "right" | "end" => {
                        margins.right = get_attribute_int(&e, "w");
                    }
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == "tblCellMar" || e.local_name().as_ref() == "tcMar" {
                    break;
                }
                buf.clear();
            }
            Ok(Event::Eof) => break,
            _ => {
                buf.clear();
            }
        }
    }

    margins
}

/// Helper: Extract string attribute value.
/// Uses local_name() to handle namespace-prefixed attributes (e.g., `w:val` matches `val`).
fn get_attribute(e: &BytesStart, key: &str) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|attr| attr.key.local_name().as_ref() == key)
        .and_then(|attr| {
            let raw = attr.value.as_ref();
            quick_xml::escape::unescape(raw).ok().map(|s| s.into_owned())
        })
}

/// Helper: Extract and parse integer attribute value.
fn get_attribute_int(e: &BytesStart, key: &str) -> Option<i32> {
    get_attribute(e, key).and_then(|s| s.parse().ok())
}

/// Helper: Extract and parse unsigned integer attribute value.
fn get_attribute_u32(e: &BytesStart, key: &str) -> Option<u32> {
    get_attribute(e, key).and_then(|s| s.parse().ok())
}

#[cfg(test)]
mod tests;
