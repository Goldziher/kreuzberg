//! Tests for the Excel (.xlsx) extraction module, split out of `excel.rs` to keep
//! that file under the line-count limit.

use super::*;

mod datetime;

/// Permissive `SecurityLimits` for tests that only care about the entry-count
/// ceiling and want the default ratio/size limits (which any small test fixture
/// passes comfortably).
fn test_limits(max_files_in_archive: usize) -> SecurityLimits {
    SecurityLimits {
        max_files_in_archive,
        ..Default::default()
    }
}

/// Build an OLE workbook followed by a ZIP central directory whose local-file
/// header is corrupt. ZIP discovery accepts the central directory, then security
/// accounting fails when it seeks back to the bogus local header. This mirrors
/// the false ZIP detection in #1938 while keeping the outer CFB independently
/// valid. ~keep
#[cfg(feature = "office")]
fn legacy_xls_with_misleading_embedded_zip() -> Vec<u8> {
    use std::io::{Seek as _, Write as _};

    let mut compound = cfb::CompoundFile::create(Cursor::new(Vec::new())).expect("create OLE workbook");
    compound
        .create_stream("/Workbook")
        .expect("create Workbook stream")
        .write_all(b"synthetic workbook stream")
        .expect("write Workbook stream");
    let ole = compound.into_inner().into_inner();

    let mut cursor = Cursor::new(ole);
    cursor
        .seek(std::io::SeekFrom::End(0))
        .expect("seek after OLE container");
    let mut zip = zip::ZipWriter::new(cursor);
    zip.start_file("drs/shapexml.xml", zip::write::SimpleFileOptions::default())
        .expect("start embedded ZIP member");
    zip.write_all(b"<shape/>").expect("write embedded ZIP member");
    let mut bytes = zip.finish().expect("finish embedded ZIP").into_inner();

    let local_header = bytes
        .windows(4)
        .rposition(|window| window == b"PK\x03\x04")
        .expect("embedded ZIP local header");
    bytes[local_header] = b'X';
    bytes
}

#[test]
#[cfg(feature = "office")]
fn should_not_validate_ole_xls_as_zip_at_bytes_boundary() {
    let bytes = legacy_xls_with_misleading_embedded_zip();
    let error = read_excel_bytes(&bytes, ".XLS", &test_limits(10_000))
        .expect_err("synthetic workbook payload is intentionally not valid BIFF");

    assert!(
        !error.to_string().contains("Archive entry"),
        "the OLE workbook must reach the XLS parser instead of ZIP validation: {error}"
    );
}

#[test]
#[cfg(feature = "office")]
fn should_not_validate_ole_xls_as_zip_at_file_boundary() {
    let directory = tempfile::tempdir().expect("create temporary directory");
    let path = directory.path().join("legacy.XLS");
    std::fs::write(&path, legacy_xls_with_misleading_embedded_zip()).expect("write synthetic XLS");

    let error = read_excel_file(path.to_str().expect("UTF-8 path"), &test_limits(10_000))
        .expect_err("synthetic workbook payload is intentionally not valid BIFF");

    assert!(
        !error.to_string().contains("Archive entry"),
        "the OLE workbook must reach the XLS parser instead of ZIP validation: {error}"
    );
}

#[test]
#[cfg(feature = "office")]
fn should_keep_zip_validation_for_zip_backed_spreadsheets() {
    let bytes = legacy_xls_with_misleading_embedded_zip();
    let error = read_excel_bytes(&bytes, ".xlsx", &test_limits(10_000))
        .expect_err("corrupt ZIP local header must fail security accounting");

    assert!(
        error.to_string().contains("Archive entry"),
        "ZIP-backed spreadsheets must retain ZIP security validation: {error}"
    );
}

/// Regression test for #102: office metadata was computed only for the OOXML
/// spreadsheet extensions, so an `.ods` reached `ExcelWorkbook` with an empty
/// metadata map — no title, no author, no dates — even though ODT and ODP read
/// the very same `meta.xml` through `extract_odt_properties`.
///
/// The expectations are read straight out of the fixture's own `meta.xml`.
#[cfg(feature = "office")]
#[test]
fn should_read_ods_document_metadata_from_meta_xml() {
    let path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/data_formats/test_01.ods");
    if !path.exists() {
        println!("Skipping: test document not found at {}", path.display());
        return;
    }
    let bytes = std::fs::read(&path).expect("read fixture");

    let (workbook, _warnings) =
        read_excel_bytes(&bytes, ".ods", &test_limits(10_000)).expect("ODS extraction should succeed");

    assert_eq!(
        workbook.metadata.get("creator").map(String::as_str),
        Some("Peter Staar"),
        "meta:initial-creator must map to the document author; got {:?}",
        workbook.metadata
    );
    assert_eq!(
        workbook.metadata.get("modified_by").map(String::as_str),
        Some("Peter Staar"),
        "dc:creator is ODF's last-modifier"
    );
    assert_eq!(
        workbook.metadata.get("created_at").map(String::as_str),
        Some("2024-11-16T05:17:41")
    );
    assert_eq!(
        workbook.metadata.get("modified_at").map(String::as_str),
        Some("2025-01-24T13:18:51")
    );
}

#[test]
fn test_format_cell_to_string_basic() {
    assert_eq!(format_cell_to_string(&Data::Empty), "");
    assert_eq!(format_cell_to_string(&Data::String("test".to_owned())), "test");
    assert_eq!(format_cell_to_string(&Data::Float(42.0)), "42");
    assert_eq!(format_cell_to_string(&Data::Int(100)), "100");
    assert_eq!(format_cell_to_string(&Data::Bool(true)), "true");
}

#[test]
fn test_escape_markdown_into() {
    let mut buffer = String::with_capacity(50);

    escape_markdown_into(&mut buffer, "normal text");
    assert_eq!(buffer, "normal text");

    buffer.clear();
    escape_markdown_into(&mut buffer, "text|with|pipes");
    assert_eq!(buffer, "text\\|with\\|pipes");

    buffer.clear();
    escape_markdown_into(&mut buffer, "back\\slash");
    assert_eq!(buffer, "back\\\\slash");
}

#[test]
fn test_capacity_optimization() {
    let buffer = String::with_capacity(100);
    assert!(buffer.capacity() >= 100);
}

#[test]
fn test_format_cell_value_datetime() {
    use calamine::{ExcelDateTime, ExcelDateTimeType};
    let dt = Data::DateTime(ExcelDateTime::new(49353.5, ExcelDateTimeType::DateTime, false));
    let result = format_cell_to_string(&dt);
    assert!(!result.is_empty());
    assert!(result.contains('-'), "Expected datetime string, got: {}", result);
}

#[test]
fn test_format_cell_value_error() {
    use calamine::CellErrorType;
    let result = format_cell_to_string(&Data::Error(CellErrorType::Div0));
    assert!(result.contains("#ERR"));
}

#[test]
fn test_format_cell_value_datetime_iso() {
    let result = format_cell_to_string(&Data::DateTimeIso("2024-01-01T10:30:00".to_owned()));
    assert_eq!(result, "2024-01-01T10:30:00");
}

#[test]
fn test_format_cell_value_duration_iso() {
    let result = format_cell_to_string(&Data::DurationIso("PT1H30M".to_owned()));
    assert_eq!(result, "DURATION: PT1H30M");
}

#[test]
fn test_escape_markdown_combined() {
    let mut buffer = String::new();
    escape_markdown_into(&mut buffer, "text|with|pipes\\and\\slashes");
    assert_eq!(buffer, "text\\|with\\|pipes\\\\and\\\\slashes");
}

#[test]
fn test_escape_markdown_no_special_chars() {
    let mut buffer = String::new();
    escape_markdown_into(&mut buffer, "plain text");
    assert_eq!(buffer, "plain text");
}

/// xberg-io/xberg#163: a spreadsheet cell may contain a hard line break
/// (Alt+Enter). Emitted raw it ends the markdown table row, so the tail of
/// the cell becomes a malformed extra row.
#[test]
fn should_replace_cell_line_break_with_break_tag() {
    let mut buffer = String::new();
    escape_markdown_into(&mut buffer, "line1\nline2");
    assert_eq!(buffer, "line1<br>line2");
}

#[test]
fn should_collapse_cell_crlf_to_a_single_break_tag() {
    let mut buffer = String::new();
    escape_markdown_into(&mut buffer, "line1\r\nline2");
    assert_eq!(buffer, "line1<br>line2");
}

#[test]
fn should_replace_lone_carriage_return_with_break_tag() {
    let mut buffer = String::new();
    escape_markdown_into(&mut buffer, "line1\rline2");
    assert_eq!(buffer, "line1<br>line2");
}

#[test]
fn should_escape_pipes_backslashes_and_line_breaks_together() {
    let mut buffer = String::new();
    escape_markdown_into(&mut buffer, "a|b\\c\nd");
    assert_eq!(buffer, "a\\|b\\\\c<br>d");
}

#[test]
fn test_process_sheet_empty() {
    let range: Range<Data> = Range::empty();
    let mut warnings = Vec::new();
    let sheet = process_sheet("EmptySheet", &range, &mut warnings);

    assert_eq!(sheet.name, "EmptySheet");
    assert_eq!(sheet.row_count, 0);
    assert_eq!(sheet.col_count, 0);
    assert_eq!(sheet.cell_count, 0);
    assert!(sheet.markdown.contains("Empty sheet"));
}

#[test]
fn test_process_sheet_single_cell() {
    let mut range: Range<Data> = Range::new((0, 0), (0, 0));
    range.set_value((0, 0), Data::String("Single Cell".to_owned()));

    let mut warnings = Vec::new();
    let sheet = process_sheet("Sheet1", &range, &mut warnings);

    assert_eq!(sheet.name, "Sheet1");
    assert_eq!(sheet.row_count, 1);
    assert_eq!(sheet.col_count, 1);
    assert_eq!(sheet.cell_count, 1);
    assert!(sheet.markdown.contains("Single Cell"));
}

#[test]
fn test_process_sheet_with_data() {
    let mut range: Range<Data> = Range::new((0, 0), (2, 1));
    range.set_value((0, 0), Data::String("Name".to_owned()));
    range.set_value((0, 1), Data::String("Age".to_owned()));
    range.set_value((1, 0), Data::String("Alice".to_owned()));
    range.set_value((1, 1), Data::Int(30));
    range.set_value((2, 0), Data::String("Bob".to_owned()));
    range.set_value((2, 1), Data::Int(25));

    let mut warnings = Vec::new();
    let sheet = process_sheet("People", &range, &mut warnings);

    assert_eq!(sheet.name, "People");
    assert_eq!(sheet.row_count, 3);
    assert_eq!(sheet.col_count, 2);
    assert!(sheet.markdown.contains("Name"));
    assert!(sheet.markdown.contains("Age"));
    assert!(sheet.markdown.contains("Alice"));
    assert!(sheet.markdown.contains("30"));
}

#[test]
fn test_generate_markdown_and_cells_empty() {
    let range: Range<Data> = Range::empty();
    let mut warnings = Vec::new();
    let (markdown, cells) = generate_markdown_and_cells("Test", &range, 100, &mut warnings);

    assert!(markdown.contains("## Test"));
    assert!(cells.is_empty());
}

#[test]
fn test_generate_markdown_and_cells_with_data() {
    let mut range: Range<Data> = Range::new((0, 0), (1, 2));
    range.set_value((0, 0), Data::String("Col1".to_owned()));
    range.set_value((0, 1), Data::String("Col2".to_owned()));
    range.set_value((0, 2), Data::String("Col3".to_owned()));
    range.set_value((1, 0), Data::String("A".to_owned()));
    range.set_value((1, 1), Data::String("B".to_owned()));
    range.set_value((1, 2), Data::String("C".to_owned()));

    let mut warnings = Vec::new();
    let (markdown, cells) = generate_markdown_and_cells("Sheet1", &range, 200, &mut warnings);

    assert!(markdown.contains("## Sheet1"));
    assert!(markdown.contains("Col1"));
    assert!(markdown.contains("---"));
    assert_eq!(cells.len(), 2);
}

#[test]
fn test_generate_markdown_and_cells_sparse() {
    let mut range: Range<Data> = Range::new((0, 0), (2, 2));
    range.set_value((0, 0), Data::String("A".to_owned()));
    range.set_value((0, 1), Data::String("B".to_owned()));
    range.set_value((0, 2), Data::String("C".to_owned()));
    range.set_value((1, 0), Data::String("X".to_owned()));
    range.set_value((1, 2), Data::String("Z".to_owned()));

    let mut warnings = Vec::new();
    let (markdown, cells) = generate_markdown_and_cells("Sparse", &range, 200, &mut warnings);

    assert!(markdown.contains("X"));
    assert!(markdown.contains("Z"));
    assert_eq!(cells.len(), 3);
}

#[test]
fn test_format_cell_value_float_integer() {
    let result = format_cell_to_string(&Data::Float(100.0));
    assert_eq!(result, "100");
}

#[test]
fn test_format_cell_value_float_decimal() {
    let result = format_cell_to_string(&Data::Float(12.3456));
    assert_eq!(result, "12.3456");
}

#[test]
fn test_format_cell_value_float_drops_binary_noise_past_fifteen_digits() {
    // `=0.1+0.2` and `=100*1.19` as Excel stores their results
    assert_eq!(format_cell_to_string(&Data::Float(0.1 + 0.2)), "0.3");
    assert_eq!(format_cell_to_string(&Data::Float(100.0 * 1.19)), "119");
    assert_eq!(format_cell_to_string(&Data::Float(1.0 / 3.0)), "0.333333333333333");
    assert_eq!(format_cell_to_string(&Data::Float(-2.675)), "-2.675");
    assert_eq!(format_cell_to_string(&Data::Float(1e21)), "1000000000000000000000");
}

#[test]
fn test_format_cell_value_float_keeps_the_finite_limits_finite() {
    // 15 significant digits of either limit round past it, which parses as infinity
    assert_eq!(format_cell_to_string(&Data::Float(f64::MAX)), f64::MAX.to_string());
    assert_eq!(format_cell_to_string(&Data::Float(f64::MIN)), f64::MIN.to_string());
}

#[test]
fn test_format_cell_value_bool_false() {
    let result = format_cell_to_string(&Data::Bool(false));
    assert_eq!(result, "false");
}

#[test]
fn test_format_cell_escape_pipe() {
    let mut buffer = String::new();
    escape_markdown_into(&mut buffer, "value|with|pipes");
    assert_eq!(buffer, "value\\|with\\|pipes");
}

#[test]
fn test_format_cell_escape_backslash() {
    let mut buffer = String::new();
    escape_markdown_into(&mut buffer, "path\\to\\file");
    assert_eq!(buffer, "path\\\\to\\\\file");
}

#[test]
fn test_markdown_table_structure() {
    let mut range: Range<Data> = Range::new((0, 0), (2, 1));
    range.set_value((0, 0), Data::String("H1".to_owned()));
    range.set_value((0, 1), Data::String("H2".to_owned()));
    range.set_value((1, 0), Data::String("A".to_owned()));
    range.set_value((1, 1), Data::String("B".to_owned()));

    let mut warnings = Vec::new();
    let (markdown, _cells) = generate_markdown_and_cells("Test", &range, 100, &mut warnings);

    let lines: Vec<&str> = markdown.lines().collect();
    assert!(lines[0].contains("## Test"));
    assert!(lines[2].starts_with("| "));
    assert!(lines[3].contains("---"));
    assert!(lines[4].starts_with("| "));
}

#[test]
fn test_process_sheet_metadata() {
    let mut range: Range<Data> = Range::new((0, 0), (9, 4));
    for row in 0..10 {
        for col in 0..5 {
            range.set_value((row, col), Data::String(format!("R{}C{}", row, col)));
        }
    }

    let mut warnings = Vec::new();
    let sheet = process_sheet("Data", &range, &mut warnings);

    assert_eq!(sheet.row_count, 10);
    assert_eq!(sheet.col_count, 5);
    assert_eq!(sheet.cell_count, 50);
}

fn make_xlsx_with_worksheet(worksheet_xml: &str) -> Vec<u8> {
    use std::io::Write;
    use zip::write::{SimpleFileOptions, ZipWriter};

    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default();

        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Override PartName="/xl/workbook.xml"
ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml"
ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
</Types>"#,
        )
        .unwrap();

        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument"
Target="xl/workbook.xml"/>
</Relationships>"#,
        )
        .unwrap();

        zip.start_file("xl/workbook.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
      xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>
</workbook>"#,
        )
        .unwrap();

        zip.start_file("xl/_rels/workbook.xml.rels", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet"
Target="worksheets/sheet1.xml"/>
</Relationships>"#,
        )
        .unwrap();

        zip.start_file("xl/worksheets/sheet1.xml", options).unwrap();
        zip.write_all(worksheet_xml.as_bytes()).unwrap();
        zip.finish().unwrap();
    }
    buffer
}

#[test]
fn should_preserve_dense_xlsx_output_after_position_only_preflight() {
    let bytes = make_xlsx_with_worksheet(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <dimension ref="A1:B2"/>
  <sheetData>
<row r="1">
  <c r="A1" t="inlineStr"><is><t>Header</t></is></c>
  <c r="B1" t="inlineStr"><is><t>Value</t></is></c>
</row>
<row r="2">
  <c r="A2" t="inlineStr"><is><t>Alpha</t></is></c>
  <c r="B2" t="inlineStr"><is><t>Beta</t></is></c>
</row>
  </sheetData>
</worksheet>"#,
    );
    let mut workbook = calamine::Xlsx::new(Cursor::new(bytes.as_slice())).unwrap();

    assert_eq!(
        classify_xlsx_sheet_shape(&mut workbook, "Sheet1").unwrap(),
        XlsxSheetShape::Dense
    );
    let mut warnings = Vec::new();
    let sheet = process_xlsx_sheet_safe(&mut workbook, "Sheet1", &mut warnings).unwrap();

    assert_eq!(sheet.row_count, 2);
    assert_eq!(sheet.col_count, 2);
    assert_eq!(sheet.cell_count, 4);
    assert_eq!(
        sheet.table_cells,
        Some(vec![
            vec!["Header".to_string(), "Value".to_string()],
            vec!["Alpha".to_string(), "Beta".to_string()],
        ])
    );
    assert_eq!(
        sheet.markdown,
        "## Sheet1\n\n| Header | Value |\n| --- | --- |\n| Alpha | Beta |\n"
    );
}

#[test]
fn should_preserve_pathological_sparse_xlsx_output_after_reopening_reader() {
    let bytes = make_xlsx_with_worksheet(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <dimension ref="A1:XFD1048575"/>
  <sheetData>
<row r="1"><c r="A1" t="inlineStr"><is><t>Start</t></is></c></row>
<row r="1048575"><c r="XFD1048575" t="inlineStr"><is><t>End</t></is></c></row>
  </sheetData>
</worksheet>"#,
    );
    let mut workbook = calamine::Xlsx::new(Cursor::new(bytes.as_slice())).unwrap();

    assert_eq!(
        classify_xlsx_sheet_shape(&mut workbook, "Sheet1").unwrap(),
        XlsxSheetShape::Sparse
    );
    let mut warnings = Vec::new();
    let sheet = process_xlsx_sheet_safe(&mut workbook, "Sheet1", &mut warnings).unwrap();

    assert_eq!(sheet.row_count, 1_048_575);
    assert_eq!(sheet.col_count, 16_384);
    assert_eq!(sheet.cell_count, 2);
    assert_eq!(
        sheet.table_cells,
        Some(vec![
            vec!["Start".to_string(), String::new()],
            vec![String::new(), "End".to_string()],
        ])
    );
    assert_eq!(sheet.markdown, "## Sheet1\n\n| Start |  |\n| --- | --- |\n|  | End |\n");
}

mod revisions;
mod spreadsheet_compatibility;

/// Build a minimal in-memory `.xlsx` zip from a caller-supplied `<workbook>`
/// inner body (the `<sheets>`/`<definedNames>` elements) and workbook-rels
/// relationships, plus any additional zip parts (worksheet XML, worksheet
/// relationships, comment parts). `[Content_Types].xml`/`_rels/.rels` mirror
/// `make_xlsx_with_worksheet`'s already-proven-working shape.
fn make_xlsx(workbook_body: &str, workbook_rels_body: &str, extra_parts: &[(&str, Vec<u8>)]) -> Vec<u8> {
    use std::io::Write;
    use zip::write::{SimpleFileOptions, ZipWriter};

    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default();

        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Override PartName="/xl/workbook.xml"
ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml"
ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
</Types>"#,
        )
        .unwrap();

        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument"
Target="xl/workbook.xml"/>
</Relationships>"#,
        )
        .unwrap();

        zip.start_file("xl/workbook.xml", options).unwrap();
        let workbook_xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" \
             xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\
             {workbook_body}</workbook>"
        );
        zip.write_all(workbook_xml.as_bytes()).unwrap();

        zip.start_file("xl/_rels/workbook.xml.rels", options).unwrap();
        let rels_xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
             {workbook_rels_body}</Relationships>"
        );
        zip.write_all(rels_xml.as_bytes()).unwrap();

        for (path, data) in extra_parts {
            zip.start_file(*path, options).unwrap();
            zip.write_all(data).unwrap();
        }

        zip.finish().unwrap();
    }
    buffer
}

const WORKSHEET_REL: &str = r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>"#;

/// xberg-io/xberg#84: a sheet declared in `workbook.xml`/`workbook.xml.rels`
/// whose worksheet part is missing from the zip must not silently vanish —
/// the workbook must come back with the readable sheets plus a warning
/// naming the one that failed.
#[test]
fn should_warn_when_xlsx_sheet_part_is_missing() {
    let sheet1_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <dimension ref="A1"/>
  <sheetData>
<row r="1"><c r="A1" t="inlineStr"><is><t>Present</t></is></c></row>
  </sheetData>
</worksheet>"#;

    let bytes = make_xlsx(
        r#"<sheets><sheet name="Good" sheetId="1" r:id="rId1"/><sheet name="Missing" sheetId="2" r:id="rId2"/></sheets>"#,
        &format!(
            "{WORKSHEET_REL}\
             <Relationship Id=\"rId2\" \
             Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" \
             Target=\"worksheets/sheet2.xml\"/>"
        ),
        &[("xl/worksheets/sheet1.xml", sheet1_xml.to_vec())],
    );

    let (workbook, warnings) = read_excel_bytes(&bytes, ".xlsx", &test_limits(10_000))
        .expect("a workbook with one missing sheet part must still parse");

    assert_eq!(workbook.sheets.len(), 1, "only the readable sheet must be kept");
    assert_eq!(workbook.sheets[0].name, "Good");

    let sheet_warnings: Vec<_> = warnings.iter().filter(|w| w.source == "excel").collect();
    assert_eq!(
        sheet_warnings.len(),
        1,
        "expected exactly one warning for the unreadable sheet, got: {warnings:?}"
    );
    assert!(
        sheet_warnings[0].message.contains("Missing"),
        "warning must name the failed sheet: {}",
        sheet_warnings[0].message
    );
}

/// xberg-io/xberg#222: a sheet whose declared dimensions vastly exceed its
/// actual data is skipped to avoid an OOM allocation; that skip must be
/// visible to the caller as a warning naming the sheet and the cap, not
/// only as text buried in the sheet's own markdown.
#[test]
fn should_warn_when_sheet_declared_dimensions_greatly_exceed_actual_data() {
    let sheet1_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <dimension ref="A1:A100002"/>
  <sheetData>
<row r="1"><c r="A1" t="inlineStr"><is><t>Start</t></is></c></row>
<row r="100002"><c r="A100002" t="inlineStr"><is><t>End</t></is></c></row>
  </sheetData>
</worksheet>"#;

    let bytes = make_xlsx(
        r#"<sheets><sheet name="Huge" sheetId="1" r:id="rId1"/></sheets>"#,
        WORKSHEET_REL,
        &[("xl/worksheets/sheet1.xml", sheet1_xml.to_vec())],
    );

    let (workbook, warnings) = read_excel_bytes(&bytes, ".xlsx", &test_limits(10_000)).expect("workbook must parse");

    assert_eq!(workbook.sheets.len(), 1);
    assert!(
        workbook.sheets[0].markdown.contains("Skipping to prevent OOM"),
        "sheet content must note the OOM-avoidance skip: {}",
        workbook.sheets[0].markdown
    );

    let truncation_warnings: Vec<_> = warnings
        .iter()
        .filter(|w| w.source == "excel" && w.message.contains("declared"))
        .collect();
    assert_eq!(
        truncation_warnings.len(),
        1,
        "expected exactly one 'declared dimensions' warning, got: {warnings:?}"
    );
    assert!(truncation_warnings[0].message.contains("Huge"));
    assert!(truncation_warnings[0].message.contains("100002"));
}

/// xberg-io/xberg#222: a sparse sheet whose non-empty rows/columns exceed the
/// internal display cap must warn, naming the sheet and the actual vs.
/// displayed size, in addition to the existing in-markdown truncation notice.
#[test]
fn should_warn_when_sparse_sheet_output_is_truncated() {
    let cells: Vec<((u32, u32), Data)> = (0..1500u32)
        .map(|row| ((row, 0u32), Data::String(format!("v{row}"))))
        .collect();

    let mut warnings = Vec::new();
    let bounding_box = SparseSheetBoundingBox {
        row_min: 0,
        row_max: 1499,
        col_min: 0,
        col_max: 0,
    };
    let sheet = process_sparse_sheet_from_cells("Big", cells, bounding_box, &mut warnings)
        .expect("sparse sheet construction must succeed");

    assert_eq!(sheet.row_count, 1500);
    assert!(
        sheet.markdown.contains("Truncated: showing 1000x1"),
        "markdown must retain the existing truncation notice: {}",
        sheet.markdown
    );

    let truncation_warnings: Vec<_> = warnings.iter().filter(|w| w.source == "excel").collect();
    assert_eq!(
        truncation_warnings.len(),
        1,
        "expected exactly one truncation warning, got: {warnings:?}"
    );
    assert!(truncation_warnings[0].message.contains("Big"));
    assert!(truncation_warnings[0].message.contains("1000x1"));
}

/// xberg-io/xberg#119: a hidden or very-hidden sheet must be flagged in
/// workbook metadata so a caller can tell it apart from a visible sheet;
/// its content is still extracted, not dropped.
#[test]
fn should_record_hidden_sheets_in_workbook_metadata() {
    let sheet1_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <dimension ref="A1"/>
  <sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Visible</t></is></c></row></sheetData>
</worksheet>"#;
    let sheet2_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <dimension ref="A1"/>
  <sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Secret</t></is></c></row></sheetData>
</worksheet>"#;

    let bytes = make_xlsx(
        r#"<sheets><sheet name="Visible" sheetId="1" r:id="rId1"/><sheet name="Hidden" sheetId="2" state="hidden" r:id="rId2"/></sheets>"#,
        &format!(
            "{WORKSHEET_REL}\
             <Relationship Id=\"rId2\" \
             Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" \
             Target=\"worksheets/sheet2.xml\"/>"
        ),
        &[
            ("xl/worksheets/sheet1.xml", sheet1_xml.to_vec()),
            ("xl/worksheets/sheet2.xml", sheet2_xml.to_vec()),
        ],
    );

    let (workbook, _warnings) = read_excel_bytes(&bytes, ".xlsx", &test_limits(10_000)).expect("workbook must parse");

    assert_eq!(workbook.sheets.len(), 2, "hidden sheet content must still be extracted");
    assert_eq!(
        workbook.metadata.get("hidden_sheets").map(String::as_str),
        Some("Hidden")
    );
    let hidden_sheet = workbook.sheets.iter().find(|s| s.name == "Hidden").unwrap();
    assert!(hidden_sheet.markdown.contains("Secret"));
}

/// xberg-io/xberg#89 / #119: non-empty cell formulas must be recoverable from
/// the workbook, not silently discarded once calamine resolves them to a
/// cached value.
#[test]
fn should_record_sheet_formulas_in_workbook_metadata() {
    let sheet1_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <dimension ref="A1:B1"/>
  <sheetData>
<row r="1">
  <c r="A1"><f>SUM(B1:B1)</f><v>5</v></c>
  <c r="B1"><v>5</v></c>
</row>
  </sheetData>
</worksheet>"#;

    let bytes = make_xlsx(
        r#"<sheets><sheet name="Calc" sheetId="1" r:id="rId1"/></sheets>"#,
        WORKSHEET_REL,
        &[("xl/worksheets/sheet1.xml", sheet1_xml.to_vec())],
    );

    let (workbook, _warnings) = read_excel_bytes(&bytes, ".xlsx", &test_limits(10_000)).expect("workbook must parse");

    assert_eq!(
        workbook.metadata.get("formulas_Calc").map(String::as_str),
        Some("A1=SUM(B1:B1)")
    );
}

/// xberg-io/xberg#89: cell hyperlinks must be recoverable, not dropped —
/// calamine 0.36 exposes them via `hyperlinks_by_sheet_name`, superseding the
/// "not accessible through the crate" limitation this module used to document.
#[test]
fn should_record_sheet_hyperlinks_in_workbook_metadata() {
    let sheet1_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <dimension ref="A1"/>
  <sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Visit</t></is></c></row></sheetData>
  <hyperlinks><hyperlink ref="A1" r:id="rId1"/></hyperlinks>
</worksheet>"#;
    let sheet1_rels = br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink"
Target="https://example.com/" TargetMode="External"/>
</Relationships>"#;

    let bytes = make_xlsx(
        r#"<sheets><sheet name="Links" sheetId="1" r:id="rId1"/></sheets>"#,
        WORKSHEET_REL,
        &[
            ("xl/worksheets/sheet1.xml", sheet1_xml.to_vec()),
            ("xl/worksheets/_rels/sheet1.xml.rels", sheet1_rels.to_vec()),
        ],
    );

    let (workbook, _warnings) = read_excel_bytes(&bytes, ".xlsx", &test_limits(10_000)).expect("workbook must parse");

    assert_eq!(
        workbook.metadata.get("hyperlinks_Links").map(String::as_str),
        Some("A1=https://example.com/")
    );
}

/// xberg-io/xberg#89: workbook-level defined names must be recoverable.
#[test]
fn should_record_defined_names_in_workbook_metadata() {
    let sheet1_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <dimension ref="A1"/>
  <sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Value</t></is></c></row></sheetData>
</worksheet>"#;

    let bytes = make_xlsx(
        r#"<sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets><definedNames><definedName name="MyRange">Sheet1!$A$1</definedName></definedNames>"#,
        WORKSHEET_REL,
        &[("xl/worksheets/sheet1.xml", sheet1_xml.to_vec())],
    );

    let (workbook, _warnings) = read_excel_bytes(&bytes, ".xlsx", &test_limits(10_000)).expect("workbook must parse");

    assert_eq!(
        workbook.metadata.get("defined_names").map(String::as_str),
        Some("MyRange=Sheet1!$A$1")
    );
}

/// xberg-io/xberg#89: a `<comment>` element's rich-text runs must be
/// concatenated into one string, keyed by cell reference.
#[test]
fn should_parse_comment_text_and_cell_ref_from_comments_xml() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<comments xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <authors><author>Alice</author></authors>
  <commentList>
<comment ref="B2" authorId="0"><text><r><t>Needs review</t></r></text></comment>
  </commentList>
</comments>"#;

    let entries = parse_comments_xml(xml).expect("comments.xml must parse");
    assert_eq!(entries, vec!["B2: Needs review".to_string()]);
}

/// xberg-io/xberg#89: comments must be surfaced end-to-end through
/// `read_excel_bytes`, not just parsed in isolation.
#[test]
fn should_surface_comments_in_full_xlsx_extraction() {
    let sheet1_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <dimension ref="A1"/>
  <sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Value</t></is></c></row></sheetData>
</worksheet>"#;
    let comments_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<comments xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <authors><author>Alice</author></authors>
  <commentList>
<comment ref="A1" authorId="0"><text><r><t>Check this</t></r></text></comment>
  </commentList>
</comments>"#;

    let bytes = make_xlsx(
        r#"<sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>"#,
        WORKSHEET_REL,
        &[
            ("xl/worksheets/sheet1.xml", sheet1_xml.to_vec()),
            ("xl/comments1.xml", comments_xml.to_vec()),
        ],
    );

    let (workbook, _warnings) = read_excel_bytes(&bytes, ".xlsx", &test_limits(10_000)).expect("workbook must parse");

    assert_eq!(
        workbook.metadata.get("comments").map(String::as_str),
        Some("A1: Check this")
    );
}

/// Sibling of `test_revisions_read_is_bounded_for_oversized_member` for the
/// `xl/comments*.xml` read path. `extract_xlsx_comments_from_archive` only lists
/// entry names before reading -- it never looks at declared sizes -- so a single
/// `xl/comments1.xml` member is the only thing that can bound its own decompressed
/// length. The comment element sits after a `MAX_EXCEL_ZIP_MEMBER_SIZE`-sized XML
/// comment: without `Read::take` on the entry, the whole member is read and the
/// comment text is found; with the cap, the read truncates mid-comment, parsing
/// fails, and the part contributes nothing.
#[test]
fn test_comments_read_is_bounded_for_oversized_member() {
    let padding_len = MAX_EXCEL_ZIP_MEMBER_SIZE as usize + 4096;
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <comments xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\n\
         <authors><author>Alice</author></authors>\n<commentList>\n<!--",
    );
    xml.push_str(&"x".repeat(padding_len));
    xml.push_str(
        "-->\n<comment ref=\"Z99\" authorId=\"0\"><text><r><t>Late comment</t></r></text></comment>\n\
         </commentList>\n</comments>",
    );

    let mut buffer = Vec::new();
    {
        use std::io::Write as _;
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut buffer));
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("xl/comments1.xml", opts).unwrap();
        zip.write_all(xml.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    let result = extract_xlsx_comments_from_bytes(&buffer);
    assert!(
        result.is_none(),
        "a comment sitting after MAX_EXCEL_ZIP_MEMBER_SIZE must never be reached; an \
         unbounded read would find 'Late comment' and return Some(..) instead of None, got {result:?}"
    );
}

/// xberg-io/xberg#103: `process_named_sheets` (the xls/xlsb/ods worksheet-range
/// loop shared via `process_workbook`) must warn on a sheet name the backend
/// cannot resolve, instead of silently dropping it. Uses a real on-disk `.xls`
/// fixture plus one name that does not exist in it, so the `Err` this exercises
/// is genuine `calamine::Reader::worksheet_range` behavior, not test-only
/// scaffolding.
#[test]
fn should_warn_when_named_sheet_range_cannot_be_read() {
    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/xls/test_excel.xls");
    let Ok(bytes) = std::fs::read(&fixture_path) else {
        eprintln!("skipping: fixture not present at {fixture_path:?}");
        return;
    };

    let mut workbook =
        calamine::Xls::new(Cursor::new(bytes.as_slice())).expect("fixture must parse as a valid XLS workbook");
    let real_names = Reader::sheet_names(&workbook);
    assert!(!real_names.is_empty(), "fixture must have at least one real sheet");

    let mut names = real_names.clone();
    names.push("Definitely-Not-A-Real-Sheet".to_owned());

    let mut warnings = Vec::new();
    let (sheets, _extra_metadata) = process_named_sheets(&mut workbook, &names, &mut warnings);

    assert_eq!(
        sheets.len(),
        real_names.len(),
        "only the real sheets should be processed"
    );
    assert_eq!(
        warnings.len(),
        1,
        "expected exactly one warning for the unresolvable sheet name, got: {warnings:?}"
    );
    assert_eq!(warnings[0].source, "excel");
    assert!(
        warnings[0].message.contains("Definitely-Not-A-Real-Sheet"),
        "warning must name the failed sheet: {}",
        warnings[0].message
    );
}
