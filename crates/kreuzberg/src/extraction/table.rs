//! Table conversion utilities.
//!
//! This module provides functions for converting tabular data between different formats.
//! Currently supports converting Apache Arrow IPC format to Markdown tables using Polars.
//!
//! # Features
//!
//! - **Arrow IPC parsing**: Read tables from Arrow IPC binary format
//! - **Markdown generation**: Convert DataFrames to clean Markdown tables
//! - **Type-safe**: Handles all Polars data types safely
//! - **Empty table handling**: Gracefully handles empty DataFrames
//!
//! # Supported Conversions
//!
//! - Arrow IPC → Markdown table
//! - Polars DataFrame → Markdown table
//!
//! # Example
//!
//! ```rust,no_run
//! use kreuzberg::extraction::table::table_from_arrow_to_markdown;
//!
//! # fn example() -> kreuzberg::Result<()> {
//! // Convert Arrow IPC bytes to Markdown
//! let arrow_bytes = vec![/* Arrow IPC data */];
//! let markdown = table_from_arrow_to_markdown(&arrow_bytes)?;
//!
//! println!("Markdown table:\n{}", markdown);
//! # Ok(())
//! # }
//! ```
//!
//! # Output Format
//!
//! The generated Markdown follows GitHub Flavored Markdown table syntax:
//! ```markdown
//! | Column1 | Column2 | Column3 |
//! |------|------|------|
//! | value1 | value2 | value3 |
//! | value4 | value5 | value6 |
//! ```

use crate::error::{KreuzbergError, Result};
use polars::prelude::*;
use std::io::Cursor;

/// Convert Arrow IPC bytes to markdown table format
pub fn table_from_arrow_to_markdown(arrow_bytes: &[u8]) -> Result<String> {
    let cursor = Cursor::new(arrow_bytes);
    let df = IpcReader::new(cursor)
        .finish()
        .map_err(|e| KreuzbergError::parsing(format!("Failed to read Arrow IPC data: {}", e)))?;

    dataframe_to_markdown(&df)
}

/// Convert a Polars DataFrame to markdown table format
fn dataframe_to_markdown(df: &DataFrame) -> Result<String> {
    if df.is_empty() {
        return Ok(String::new());
    }

    let mut markdown = String::new();

    markdown.push_str("| ");
    for col_name in df.get_column_names() {
        markdown.push_str(col_name);
        markdown.push_str(" | ");
    }
    markdown.push('\n');

    markdown.push('|');
    for _ in 0..df.width() {
        markdown.push_str("------|");
    }
    markdown.push('\n');

    for row_idx in 0..df.height() {
        markdown.push_str("| ");
        for col in df.get_columns() {
            let series = col.as_materialized_series();
            let value = format_cell_value(series, row_idx)?;
            markdown.push_str(&value);
            markdown.push_str(" | ");
        }
        markdown.push('\n');
    }

    Ok(markdown)
}

fn format_cell_value(series: &Series, idx: usize) -> Result<String> {
    let is_null_array = series.is_null();
    if is_null_array.get(idx).unwrap_or(false) {
        return Ok(String::new());
    }

    let value_str = match series.dtype() {
        DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64 => {
            let casted = series
                .cast(&DataType::Int64)
                .map_err(|e| KreuzbergError::parsing(format!("Failed to cast to i64: {}", e)))?;
            casted
                .i64()
                .map_err(|e| KreuzbergError::parsing(format!("Failed to get i64 value: {}", e)))?
                .get(idx)
                .map(|v| v.to_string())
                .unwrap_or_default()
        }
        DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64 => {
            let casted = series
                .cast(&DataType::UInt64)
                .map_err(|e| KreuzbergError::parsing(format!("Failed to cast to u64: {}", e)))?;
            casted
                .u64()
                .map_err(|e| KreuzbergError::parsing(format!("Failed to get u64 value: {}", e)))?
                .get(idx)
                .map(|v| v.to_string())
                .unwrap_or_default()
        }
        DataType::Float32 | DataType::Float64 => {
            let casted = series
                .cast(&DataType::Float64)
                .map_err(|e| KreuzbergError::parsing(format!("Failed to cast to f64: {}", e)))?;
            casted
                .f64()
                .map_err(|e| KreuzbergError::parsing(format!("Failed to get f64 value: {}", e)))?
                .get(idx)
                .map(|v| format!("{:.2}", v))
                .unwrap_or_default()
        }
        DataType::Boolean => series
            .bool()
            .map_err(|e| KreuzbergError::parsing(format!("Failed to get bool value: {}", e)))?
            .get(idx)
            .map(|v| v.to_string())
            .unwrap_or_default(),
        DataType::String => series
            .str()
            .map_err(|e| KreuzbergError::parsing(format!("Failed to get string value: {}", e)))?
            .get(idx)
            .map(|v| v.to_string())
            .unwrap_or_default(),
        _ => {
            format!("{:?}", series.get(idx))
        }
    };

    Ok(value_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn create_test_dataframe() -> DataFrame {
        df!(
            "name" => &["Alice", "Bob", "Charlie"],
            "age" => &[30, 25, 35],
            "score" => &[95.5, 87.3, 92.1]
        )
        .unwrap()
    }

    fn dataframe_to_arrow_bytes(df: &DataFrame) -> Vec<u8> {
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);
        let mut df_mut = df.clone();
        IpcWriter::new(&mut cursor).finish(&mut df_mut).unwrap();
        buffer
    }

    #[test]
    fn test_dataframe_to_markdown_basic() {
        let df = create_test_dataframe();
        let markdown = dataframe_to_markdown(&df).unwrap();

        assert!(markdown.contains("| name | age | score |"));
        assert!(markdown.contains("|------|------|------|"));
        assert!(markdown.contains("| Alice | 30 | 95.50 |"));
        assert!(markdown.contains("| Bob | 25 | 87.30 |"));
        assert!(markdown.contains("| Charlie | 35 | 92.10 |"));
    }

    #[test]
    fn test_table_from_arrow_to_markdown() {
        let df = create_test_dataframe();
        let arrow_bytes = dataframe_to_arrow_bytes(&df);

        let markdown = table_from_arrow_to_markdown(&arrow_bytes).unwrap();

        assert!(markdown.contains("| name | age | score |"));
        assert!(markdown.contains("| Alice | 30 | 95.50 |"));
        assert!(markdown.contains("| Bob | 25 | 87.30 |"));
        assert!(markdown.contains("| Charlie | 35 | 92.10 |"));
    }

    #[test]
    fn test_empty_dataframe() {
        let df = df!("col1" => Vec::<i32>::new()).unwrap();
        let markdown = dataframe_to_markdown(&df).unwrap();
        assert_eq!(markdown, "");
    }

    #[test]
    fn test_dataframe_with_nulls() {
        let s1 = Series::new("name".into(), &["Alice", "Bob", "Charlie"]);
        let s2 = Series::new("value".into(), &[Some(1), None, Some(3)]);
        let df = DataFrame::new(vec![s1.into(), s2.into()]).unwrap();

        let markdown = dataframe_to_markdown(&df).unwrap();

        assert!(markdown.contains("| name | value |"));
        assert!(markdown.contains("| Alice | 1 |"));
        assert!(markdown.contains("| Bob |  |"));
        assert!(markdown.contains("| Charlie | 3 |"));
    }

    #[test]
    fn test_dataframe_with_booleans() {
        let df = df!(
            "name" => &["Alice", "Bob"],
            "active" => &[true, false]
        )
        .unwrap();

        let markdown = dataframe_to_markdown(&df).unwrap();

        assert!(markdown.contains("| name | active |"));
        assert!(markdown.contains("| Alice | true |"));
        assert!(markdown.contains("| Bob | false |"));
    }

    #[test]
    fn test_dataframe_with_integers() {
        let df = df!(
            "id" => &[1i64, 2i64, 3i64],
            "count" => &[100u64, 200u64, 300u64]
        )
        .unwrap();

        let markdown = dataframe_to_markdown(&df).unwrap();

        assert!(markdown.contains("| id | count |"));
        assert!(markdown.contains("| 1 | 100 |"));
        assert!(markdown.contains("| 2 | 200 |"));
        assert!(markdown.contains("| 3 | 300 |"));
    }

    #[test]
    fn test_single_column_dataframe() {
        let df = df!("name" => &["Alice", "Bob", "Charlie"]).unwrap();
        let markdown = dataframe_to_markdown(&df).unwrap();

        assert!(markdown.contains("| name |"));
        assert!(markdown.contains("|------|"));
        assert!(markdown.contains("| Alice |"));
        assert!(markdown.contains("| Bob |"));
        assert!(markdown.contains("| Charlie |"));
    }

    #[test]
    fn test_single_row_dataframe() {
        let df = df!(
            "name" => &["Alice"],
            "age" => &[30]
        )
        .unwrap();

        let markdown = dataframe_to_markdown(&df).unwrap();

        assert!(markdown.contains("| name | age |"));
        assert!(markdown.contains("| Alice | 30 |"));
    }

    #[test]
    fn test_arrow_bytes_roundtrip() {
        let original_df = df!(
            "col1" => &[1, 2, 3],
            "col2" => &["a", "b", "c"]
        )
        .unwrap();

        let arrow_bytes = dataframe_to_arrow_bytes(&original_df);
        let markdown = table_from_arrow_to_markdown(&arrow_bytes).unwrap();

        assert!(markdown.contains("| col1 | col2 |"));
        assert!(markdown.contains("| 1 | a |"));
        assert!(markdown.contains("| 2 | b |"));
        assert!(markdown.contains("| 3 | c |"));
    }

    #[test]
    fn test_invalid_arrow_bytes() {
        let invalid_bytes = vec![0u8; 10];
        let result = table_from_arrow_to_markdown(&invalid_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_float_formatting() {
        let df = df!(
            "value" => &[1.234, 5.678, 9.012]
        )
        .unwrap();

        let markdown = dataframe_to_markdown(&df).unwrap();

        assert!(markdown.contains("| 1.23 |"));
        assert!(markdown.contains("| 5.68 |"));
        assert!(markdown.contains("| 9.01 |"));
    }

    #[test]
    fn test_special_characters_in_strings() {
        let df = df!(
            "text" => &["Hello | World", "A & B", "C > D"]
        )
        .unwrap();

        let markdown = dataframe_to_markdown(&df).unwrap();

        assert!(markdown.contains("| Hello | World |"));
        assert!(markdown.contains("| A & B |"));
        assert!(markdown.contains("| C > D |"));
    }
}
