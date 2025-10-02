//! Markdown table generation from reconstructed tables

/// Convert 2D table to markdown format
///
/// Generates a markdown table with proper formatting, including:
/// - Header row (first row)
/// - Separator row with dashes
/// - Data rows
/// - Proper cell padding and alignment
///
/// # Arguments
///
/// * `table` - 2D vector representing the table (rows × columns)
///
/// # Returns
///
/// Markdown-formatted table string
pub fn table_to_markdown(table: &[Vec<String>]) -> String {
    if table.is_empty() {
        return String::new();
    }

    let num_rows = table.len();
    if num_rows == 0 {
        return String::new();
    }

    let num_cols = table[0].len();
    if num_cols == 0 {
        return String::new();
    }

    // Calculate column widths for proper alignment
    let col_widths = calculate_column_widths(table);

    let mut result = Vec::new();

    for (row_idx, row) in table.iter().enumerate() {
        // Build row
        let mut row_parts = vec![];
        for (col_idx, cell) in row.iter().enumerate() {
            let width = col_widths.get(col_idx).copied().unwrap_or(0);
            row_parts.push(pad_cell(cell, width));
        }

        result.push(format!("| {} |", row_parts.join(" | ")));

        // Add separator after header (first row)
        if row_idx == 0 {
            let separator_parts: Vec<String> = col_widths.iter().map(|&width| "-".repeat(width.max(3))).collect();
            result.push(format!("| {} |", separator_parts.join(" | ")));
        }
    }

    result.join("\n")
}

/// Calculate the maximum width needed for each column
fn calculate_column_widths(table: &[Vec<String>]) -> Vec<usize> {
    if table.is_empty() {
        return Vec::new();
    }

    let num_cols = table[0].len();
    let mut widths = vec![0; num_cols];

    for row in table {
        for (col_idx, cell) in row.iter().enumerate() {
            if col_idx < widths.len() {
                widths[col_idx] = widths[col_idx].max(cell.len());
            }
        }
    }

    // Minimum width of 3 for readability
    widths.iter_mut().for_each(|w| *w = (*w).max(3));

    widths
}

/// Pad cell content to specified width
fn pad_cell(content: &str, width: usize) -> String {
    if content.len() >= width {
        content.to_string()
    } else {
        let padding = width - content.len();
        format!("{}{}", content, " ".repeat(padding))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_to_markdown_simple() {
        let table = vec![
            vec!["Name".to_string(), "Age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
            vec!["Bob".to_string(), "25".to_string()],
        ];

        let markdown = table_to_markdown(&table);

        assert!(markdown.contains("| Name  | Age |"));
        assert!(markdown.contains("| Alice | 30  |"));
        assert!(markdown.contains("| Bob   | 25  |"));
        assert!(markdown.contains("| ----- | --- |"));
    }

    #[test]
    fn test_table_to_markdown_empty() {
        let table: Vec<Vec<String>> = vec![];
        let markdown = table_to_markdown(&table);
        assert_eq!(markdown, "");
    }

    #[test]
    fn test_table_to_markdown_single_cell() {
        let table = vec![vec!["Single".to_string()]];
        let markdown = table_to_markdown(&table);

        assert!(markdown.contains("| Single |"));
        assert!(markdown.contains("| ------ |"));
    }

    #[test]
    fn test_table_to_markdown_varying_widths() {
        let table = vec![
            vec!["Short".to_string(), "Very Long Header".to_string()],
            vec!["A".to_string(), "B".to_string()],
        ];

        let markdown = table_to_markdown(&table);

        assert!(markdown.contains("| Short | Very Long Header |"));
        assert!(markdown.contains("| A     | B                |"));
    }

    #[test]
    fn test_calculate_column_widths() {
        let table = vec![
            vec!["Name".to_string(), "Age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
            vec!["Bob".to_string(), "25".to_string()],
        ];

        let widths = calculate_column_widths(&table);

        assert_eq!(widths.len(), 2);
        assert_eq!(widths[0], 5); // "Alice" is longest (5 chars)
        assert_eq!(widths[1], 3); // "Age" is longest (3 chars), minimum width
    }

    #[test]
    fn test_calculate_column_widths_empty() {
        let table: Vec<Vec<String>> = vec![];
        let widths = calculate_column_widths(&table);
        assert_eq!(widths.len(), 0);
    }

    #[test]
    fn test_pad_cell() {
        assert_eq!(pad_cell("test", 10), "test      ");
        assert_eq!(pad_cell("test", 4), "test");
        assert_eq!(pad_cell("test", 2), "test");
        assert_eq!(pad_cell("", 5), "     ");
    }

    #[test]
    fn test_table_to_markdown_with_empty_cells() {
        let table = vec![
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
            vec!["1".to_string(), "".to_string(), "3".to_string()],
            vec!["4".to_string(), "5".to_string(), "".to_string()],
        ];

        let markdown = table_to_markdown(&table);

        assert!(markdown.contains("| A   | B   | C   |"));
        assert!(markdown.contains("| 1   |     | 3   |"));
        assert!(markdown.contains("| 4   | 5   |     |"));
    }

    #[test]
    fn test_table_to_markdown_minimum_width() {
        let table = vec![
            vec!["A".to_string(), "B".to_string()],
            vec!["1".to_string(), "2".to_string()],
        ];

        let markdown = table_to_markdown(&table);

        // Even single character cells should have minimum width of 3
        assert!(markdown.contains("| A   | B   |"));
        assert!(markdown.contains("| --- | --- |"));
    }
}
