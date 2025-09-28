/// High-performance Excel processing using Calamine
///
/// This module provides direct Excel reading and processing in Rust,
/// replacing the python-calamine bridge for 5-10x performance improvement.
use calamine::{Data, Range, Reader, open_workbook_auto};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::io::Cursor;
use std::path::Path;

/// Represents extracted Excel workbook DTO (Data Transfer Object)
#[pyclass]
#[derive(Debug, Clone)]
pub struct ExcelWorkbookDTO {
    #[pyo3(get)]
    pub sheets: Vec<ExcelSheetDTO>,
    #[pyo3(get)]
    pub metadata: HashMap<String, String>,
}

/// Represents a single Excel sheet DTO (Data Transfer Object)
#[pyclass]
#[derive(Debug, Clone)]
pub struct ExcelSheetDTO {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub markdown: String,
    #[pyo3(get)]
    pub row_count: usize,
    #[pyo3(get)]
    pub col_count: usize,
    #[pyo3(get)]
    pub cell_count: usize,
}

/// Read Excel file from path
#[pyfunction]
pub fn read_excel_file(file_path: &str) -> PyResult<ExcelWorkbookDTO> {
    let workbook = open_workbook_auto(Path::new(file_path))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;

    process_workbook(workbook)
}

/// Read Excel file from bytes
#[pyfunction]
pub fn read_excel_bytes(data: &Bound<'_, PyBytes>, file_extension: &str) -> PyResult<ExcelWorkbookDTO> {
    let bytes = data.as_bytes();
    let cursor = Cursor::new(bytes);

    match file_extension.to_lowercase().as_str() {
        ".xlsx" | ".xlsm" | ".xlam" | ".xltm" => {
            let workbook =
                calamine::Xlsx::new(cursor).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
            process_workbook(workbook)
        }
        ".xls" | ".xla" => {
            let workbook =
                calamine::Xls::new(cursor).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
            process_workbook(workbook)
        }
        ".xlsb" => {
            let workbook =
                calamine::Xlsb::new(cursor).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
            process_workbook(workbook)
        }
        ".ods" => {
            let workbook =
                calamine::Ods::new(cursor).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
            process_workbook(workbook)
        }
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Unsupported file extension: {}",
            file_extension
        ))),
    }
}

/// Process any workbook type that implements Reader trait
fn process_workbook<RS, R>(mut workbook: R) -> PyResult<ExcelWorkbookDTO>
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

    Ok(ExcelWorkbookDTO { sheets, metadata })
}

/// Process a single sheet to markdown with optimized memory usage
#[inline]
fn process_sheet(name: &str, range: &Range<Data>) -> ExcelSheetDTO {
    let (rows, cols) = range.get_size();
    let cell_count = range.used_cells().count();

    let estimated_capacity = 50 + (cols * 20) + (rows * cols * 12);

    let markdown = if rows == 0 || cols == 0 {
        format!("## {}\n\n*Empty sheet*", name)
    } else {
        generate_markdown_from_range_optimized(name, range, estimated_capacity)
    };

    ExcelSheetDTO {
        name: name.to_owned(),
        markdown,
        row_count: rows,
        col_count: cols,
        cell_count,
    }
}

/// Optimized markdown generation with minimal allocations
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

/// High-performance cell formatting that writes directly to string buffer
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

/// Optimized markdown escaping that writes directly to buffer
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

/// Extract metadata efficiently with minimal allocations
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

/// Convert Excel workbook to single markdown string
#[pyfunction]
pub fn excel_to_markdown(file_path: &str) -> PyResult<String> {
    let workbook = read_excel_file(file_path)?;

    let total_capacity: usize = workbook.sheets.iter().map(|sheet| sheet.markdown.len() + 2).sum();

    let mut result = String::with_capacity(total_capacity);

    for (i, sheet) in workbook.sheets.iter().enumerate() {
        if i > 0 {
            result.push_str("\n\n");
        }
        let sheet_content = sheet.markdown.trim_end();
        result.push_str(sheet_content);
    }

    Ok(result)
}

/// Benchmark function for performance testing
#[pyfunction]
pub fn benchmark_excel_reading(file_path: &str, iterations: usize) -> PyResult<f64> {
    use std::time::Instant;

    let mut total_time = 0.0;

    for _ in 0..iterations {
        let start = Instant::now();
        let _ = read_excel_file(file_path)?;
        total_time += start.elapsed().as_secs_f64();
    }

    Ok(total_time / iterations as f64)
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
        assert_eq!(buffer, "3.14159");

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
}
