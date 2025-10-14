use calamine::{Data, Range, Reader, open_workbook_auto};
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::io::Cursor;
use std::path::Path;

use crate::error::{KreuzbergError, Result};
use crate::types::{ExcelSheet, ExcelWorkbook};

pub fn read_excel_file(file_path: &str) -> Result<ExcelWorkbook> {
    let workbook = open_workbook_auto(Path::new(file_path))
        .map_err(|e| KreuzbergError::Parsing(format!("Failed to open Excel file: {}", e)))?;

    process_workbook(workbook)
}

pub fn read_excel_bytes(data: &[u8], file_extension: &str) -> Result<ExcelWorkbook> {
    let cursor = Cursor::new(data);

    match file_extension.to_lowercase().as_str() {
        ".xlsx" | ".xlsm" | ".xlam" | ".xltm" => {
            let workbook = calamine::Xlsx::new(cursor)
                .map_err(|e| KreuzbergError::Parsing(format!("Failed to parse XLSX: {}", e)))?;
            process_workbook(workbook)
        }
        ".xls" | ".xla" => {
            let workbook = calamine::Xls::new(cursor)
                .map_err(|e| KreuzbergError::Parsing(format!("Failed to parse XLS: {}", e)))?;
            process_workbook(workbook)
        }
        ".xlsb" => {
            let workbook = calamine::Xlsb::new(cursor)
                .map_err(|e| KreuzbergError::Parsing(format!("Failed to parse XLSB: {}", e)))?;
            process_workbook(workbook)
        }
        ".ods" => {
            let workbook = calamine::Ods::new(cursor)
                .map_err(|e| KreuzbergError::Parsing(format!("Failed to parse ODS: {}", e)))?;
            process_workbook(workbook)
        }
        _ => Err(KreuzbergError::Parsing(format!(
            "Unsupported file extension: {}",
            file_extension
        ))),
    }
}

fn process_workbook<RS, R>(mut workbook: R) -> Result<ExcelWorkbook>
where
    RS: std::io::Read + std::io::Seek,
    R: Reader<RS>,
{
    let sheet_names = workbook.sheet_names();

    let mut sheets = Vec::with_capacity(sheet_names.len());

    for name in &sheet_names {
        if let Ok(range) = workbook.worksheet_range(name) {
            sheets.push(process_sheet(name, &range));
        }
    }

    let metadata = extract_metadata(&workbook, &sheet_names);

    Ok(ExcelWorkbook { sheets, metadata })
}

#[inline]
fn process_sheet(name: &str, range: &Range<Data>) -> ExcelSheet {
    let (rows, cols) = range.get_size();
    let cell_count = range.used_cells().count();

    let estimated_capacity = 50 + (cols * 20) + (rows * cols * 12);

    let markdown = if rows == 0 || cols == 0 {
        format!("## {}\n\n*Empty sheet*", name)
    } else {
        generate_markdown_from_range_optimized(name, range, estimated_capacity)
    };

    ExcelSheet {
        name: name.to_owned(),
        markdown,
        row_count: rows,
        col_count: cols,
        cell_count,
    }
}

fn generate_markdown_from_range_optimized(sheet_name: &str, range: &Range<Data>, capacity: usize) -> String {
    let mut result = String::with_capacity(capacity);

    write!(result, "## {}\n\n", sheet_name).unwrap();

    let rows: Vec<_> = range.rows().collect();
    if rows.is_empty() {
        result.push_str("*No data*");
        return result;
    }

    let header = &rows[0];
    let header_len = header.len();

    result.push_str("| ");
    for (i, cell) in header.iter().enumerate() {
        if i > 0 {
            result.push_str(" | ");
        }
        format_cell_value_into(&mut result, cell);
    }
    result.push_str(" |\n");

    result.push_str("| ");
    for i in 0..header_len {
        if i > 0 {
            result.push_str(" | ");
        }
        result.push_str("---");
    }
    result.push_str(" |\n");

    for row in rows.iter().skip(1) {
        result.push_str("| ");
        for i in 0..header_len {
            if i > 0 {
                result.push_str(" | ");
            }
            if let Some(cell) = row.get(i) {
                format_cell_value_into(&mut result, cell);
            }
        }
        result.push_str(" |\n");
    }

    result
}

#[inline]
fn format_cell_value_into(buffer: &mut String, data: &Data) {
    match data {
        Data::Empty => {}
        Data::String(s) => {
            if s.contains('|') || s.contains('\\') {
                escape_markdown_into(buffer, s);
            } else {
                buffer.push_str(s);
            }
        }
        Data::Float(f) => {
            if f.fract() == 0.0 {
                write!(buffer, "{:.1}", f).unwrap();
            } else {
                write!(buffer, "{}", f).unwrap();
            }
        }
        Data::Int(i) => {
            write!(buffer, "{}", i).unwrap();
        }
        Data::Bool(b) => {
            buffer.push_str(if *b { "true" } else { "false" });
        }
        Data::DateTime(dt) => {
            if let Some(datetime) = dt.as_datetime() {
                write!(buffer, "{}", datetime.format("%Y-%m-%d %H:%M:%S")).unwrap();
            } else {
                write!(buffer, "{:?}", dt).unwrap();
            }
        }
        Data::Error(e) => {
            write!(buffer, "#ERR: {:?}", e).unwrap();
        }
        Data::DateTimeIso(s) => {
            buffer.push_str(s);
        }
        Data::DurationIso(s) => {
            buffer.push_str("DURATION: ");
            buffer.push_str(s);
        }
    }
}

#[inline]
fn escape_markdown_into(buffer: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '|' => buffer.push_str("\\|"),
            '\\' => buffer.push_str("\\\\"),
            _ => buffer.push(ch),
        }
    }
}

fn extract_metadata<RS, R>(workbook: &R, sheet_names: &[String]) -> HashMap<String, String>
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

    metadata
}

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
mod tests {
    use super::*;

    #[test]
    fn test_format_cell_value_into() {
        let mut buffer = String::with_capacity(100);

        format_cell_value_into(&mut buffer, &Data::Empty);
        assert_eq!(buffer, "");

        buffer.clear();
        format_cell_value_into(&mut buffer, &Data::String("test".to_owned()));
        assert_eq!(buffer, "test");

        buffer.clear();
        format_cell_value_into(&mut buffer, &Data::Float(42.0));
        assert_eq!(buffer, "42.0");

        buffer.clear();
        format_cell_value_into(&mut buffer, &Data::Float(std::f64::consts::PI));
        assert_eq!(buffer, "3.141592653589793");

        buffer.clear();
        format_cell_value_into(&mut buffer, &Data::Int(100));
        assert_eq!(buffer, "100");

        buffer.clear();
        format_cell_value_into(&mut buffer, &Data::Bool(true));
        assert_eq!(buffer, "true");
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
        let mut buffer = String::with_capacity(100);
        format_cell_value_into(&mut buffer, &Data::String("test".to_owned()));

        assert!(buffer.capacity() >= 100);
    }

    #[test]
    fn test_format_cell_value_datetime() {
        use calamine::{ExcelDateTime, ExcelDateTimeType};
        let mut buffer = String::new();

        let dt = Data::DateTime(ExcelDateTime::new(49353.5, ExcelDateTimeType::DateTime, false));
        format_cell_value_into(&mut buffer, &dt);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_format_cell_value_error() {
        use calamine::CellErrorType;
        let mut buffer = String::new();

        format_cell_value_into(&mut buffer, &Data::Error(CellErrorType::Div0));
        assert!(buffer.contains("#ERR"));
    }

    #[test]
    fn test_format_cell_value_datetime_iso() {
        let mut buffer = String::new();
        format_cell_value_into(&mut buffer, &Data::DateTimeIso("2024-01-01T10:30:00".to_owned()));
        assert_eq!(buffer, "2024-01-01T10:30:00");
    }

    #[test]
    fn test_format_cell_value_duration_iso() {
        let mut buffer = String::new();
        format_cell_value_into(&mut buffer, &Data::DurationIso("PT1H30M".to_owned()));
        assert_eq!(buffer, "DURATION: PT1H30M");
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

    #[test]
    fn test_process_sheet_empty() {
        let range: Range<Data> = Range::empty();
        let sheet = process_sheet("EmptySheet", &range);

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

        let sheet = process_sheet("Sheet1", &range);

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

        let sheet = process_sheet("People", &range);

        assert_eq!(sheet.name, "People");
        assert_eq!(sheet.row_count, 3);
        assert_eq!(sheet.col_count, 2);
        assert!(sheet.markdown.contains("Name"));
        assert!(sheet.markdown.contains("Age"));
        assert!(sheet.markdown.contains("Alice"));
        assert!(sheet.markdown.contains("30"));
    }

    #[test]
    fn test_generate_markdown_empty_range() {
        let range: Range<Data> = Range::new((0, 0), (0, 0));
        let markdown = generate_markdown_from_range_optimized("Test", &range, 100);

        assert!(markdown.contains("## Test"));
        assert!(markdown.contains("|"));
    }

    #[test]
    fn test_generate_markdown_with_headers() {
        let mut range: Range<Data> = Range::new((0, 0), (1, 2));
        range.set_value((0, 0), Data::String("Col1".to_owned()));
        range.set_value((0, 1), Data::String("Col2".to_owned()));
        range.set_value((0, 2), Data::String("Col3".to_owned()));
        range.set_value((1, 0), Data::String("A".to_owned()));
        range.set_value((1, 1), Data::String("B".to_owned()));
        range.set_value((1, 2), Data::String("C".to_owned()));

        let markdown = generate_markdown_from_range_optimized("Sheet1", &range, 200);

        assert!(markdown.contains("## Sheet1"));
        assert!(markdown.contains("Col1"));
        assert!(markdown.contains("Col2"));
        assert!(markdown.contains("Col3"));
        assert!(markdown.contains("---"));
        assert!(markdown.contains("A"));
        assert!(markdown.contains("B"));
        assert!(markdown.contains("C"));
    }

    #[test]
    fn test_generate_markdown_sparse_data() {
        let mut range: Range<Data> = Range::new((0, 0), (2, 2));
        range.set_value((0, 0), Data::String("A".to_owned()));
        range.set_value((0, 1), Data::String("B".to_owned()));
        range.set_value((0, 2), Data::String("C".to_owned()));
        range.set_value((1, 0), Data::String("X".to_owned()));
        range.set_value((1, 2), Data::String("Z".to_owned()));

        let markdown = generate_markdown_from_range_optimized("Sparse", &range, 200);

        assert!(markdown.contains("X"));
        assert!(markdown.contains("Z"));
        let lines: Vec<&str> = markdown.lines().collect();
        assert!(lines.iter().any(|line| line.contains("|  |") || line.contains("| |")));
    }

    #[test]
    fn test_format_cell_value_float_integer() {
        let mut buffer = String::new();
        format_cell_value_into(&mut buffer, &Data::Float(100.0));
        assert_eq!(buffer, "100.0");
    }

    #[test]
    fn test_format_cell_value_float_decimal() {
        let mut buffer = String::new();
        format_cell_value_into(&mut buffer, &Data::Float(12.3456));
        assert_eq!(buffer, "12.3456");
    }

    #[test]
    fn test_format_cell_value_bool_false() {
        let mut buffer = String::new();
        format_cell_value_into(&mut buffer, &Data::Bool(false));
        assert_eq!(buffer, "false");
    }

    #[test]
    fn test_format_cell_value_string_with_pipe() {
        let mut buffer = String::new();
        format_cell_value_into(&mut buffer, &Data::String("value|with|pipes".to_owned()));
        assert_eq!(buffer, "value\\|with\\|pipes");
    }

    #[test]
    fn test_format_cell_value_string_with_backslash() {
        let mut buffer = String::new();
        format_cell_value_into(&mut buffer, &Data::String("path\\to\\file".to_owned()));
        assert_eq!(buffer, "path\\\\to\\\\file");
    }

    #[test]
    fn test_markdown_table_structure() {
        let mut range: Range<Data> = Range::new((0, 0), (2, 1));
        range.set_value((0, 0), Data::String("H1".to_owned()));
        range.set_value((0, 1), Data::String("H2".to_owned()));
        range.set_value((1, 0), Data::String("A".to_owned()));
        range.set_value((1, 1), Data::String("B".to_owned()));

        let markdown = generate_markdown_from_range_optimized("Test", &range, 100);

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

        let sheet = process_sheet("Data", &range);

        assert_eq!(sheet.row_count, 10);
        assert_eq!(sheet.col_count, 5);
        assert_eq!(sheet.cell_count, 50);
    }
}
