//! Markdown table formatting utilities
//!
//! This module provides utilities for converting tabular data into GitHub-Flavored Markdown (GFM) tables.
//! It's used by multiple extractors (DOCX, HTML) that need to represent structured table data in markdown format.

/// Converts a 2D vector of cell strings into a GitHub-Flavored Markdown table.
///
/// # Behavior
///
/// - The first row is treated as the header row
/// - A separator row is inserted after the header
/// - Pipe characters (`|`) in cell content are automatically escaped with backslash
/// - Irregular tables (rows with varying column counts) are padded with empty cells to match the header
/// - Returns an empty string for empty input
///
/// # Arguments
///
/// * `cells` - A slice of vectors representing table rows, where each inner vector contains cell values
///
/// # Returns
///
/// A `String` containing the GFM markdown table representation
///
/// # Examples
///
/// ```
/// # use kreuzberg::extraction::cells_to_markdown;
/// let cells = vec![
///     vec!["Name".to_string(), "Age".to_string()],
///     vec!["Alice".to_string(), "30".to_string()],
///     vec!["Bob".to_string(), "25".to_string()],
/// ];
///
/// let markdown = cells_to_markdown(&cells);
/// assert!(markdown.contains("| Name | Age |"));
/// assert!(markdown.contains("|------|------|"));
/// ```
pub fn cells_to_markdown(cells: &[Vec<String>]) -> String {
    if cells.is_empty() {
        return String::new();
    }

    let mut markdown = String::new();

    // Determine number of columns from first row
    let num_cols = cells.first().map(|r| r.len()).unwrap_or(0);
    if num_cols == 0 {
        return String::new();
    }

    // Header row (first row)
    if let Some(header) = cells.first() {
        markdown.push('|');
        for cell in header {
            markdown.push(' ');
            // Escape pipe characters in cell content
            let escaped = cell.replace('|', "\\|");
            markdown.push_str(&escaped);
            markdown.push_str(" |");
        }
        markdown.push('\n');

        // Separator row
        markdown.push('|');
        for _ in 0..num_cols {
            markdown.push_str("------|");
        }
        markdown.push('\n');
    }

    // Data rows (skip first row as it's the header)
    for row in cells.iter().skip(1) {
        markdown.push('|');
        for (idx, cell) in row.iter().enumerate() {
            if idx >= num_cols {
                break; // Handle irregular tables
            }
            markdown.push(' ');
            // Escape pipe characters in cell content
            let escaped = cell.replace('|', "\\|");
            markdown.push_str(&escaped);
            markdown.push_str(" |");
        }
        // Pad with empty cells if row is shorter than expected
        for _ in row.len()..num_cols {
            markdown.push_str(" |");
        }
        markdown.push('\n');
    }

    markdown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_formatting_from_simple_table() {
        // Given: A simple 2x2 table with header
        let cells = vec![
            vec!["Header1".to_string(), "Header2".to_string()],
            vec!["Row1Col1".to_string(), "Row1Col2".to_string()],
            vec!["Row2Col1".to_string(), "Row2Col2".to_string()],
        ];

        // When: Converting to markdown
        let markdown = cells_to_markdown(&cells);

        // Then: Should produce valid GFM table
        assert!(markdown.contains("| Header1 | Header2 |"));
        assert!(markdown.contains("|------|------|"));
        assert!(markdown.contains("| Row1Col1 | Row1Col2 |"));
        assert!(markdown.contains("| Row2Col1 | Row2Col2 |"));

        // And: Should have proper structure (header, separator, 2 data rows)
        let lines: Vec<&str> = markdown.lines().collect();
        assert_eq!(lines.len(), 4); // header + separator + 2 data rows
    }

    #[test]
    fn test_markdown_handles_empty_input() {
        // Given: Empty cells vector
        let cells: Vec<Vec<String>> = vec![];

        // When: Converting to markdown
        let markdown = cells_to_markdown(&cells);

        // Then: Should return empty string without panicking
        assert_eq!(markdown, "");
    }

    #[test]
    fn test_markdown_escapes_pipe_characters() {
        // Given: Cells containing pipe characters
        let cells = vec![
            vec!["Header".to_string()],
            vec!["Cell with | pipe".to_string()],
        ];

        // When: Converting to markdown
        let markdown = cells_to_markdown(&cells);

        // Then: Pipe characters should be escaped to not break table structure
        assert!(markdown.contains("Cell with \\| pipe"));

        // And: Table structure should still be valid (starts and ends with |)
        for line in markdown.lines() {
            if !line.is_empty() {
                assert!(line.starts_with('|'));
                assert!(line.ends_with('|'));
            }
        }
    }

    #[test]
    fn test_markdown_pads_irregular_tables() {
        // Given: Table with irregular row lengths
        let cells = vec![
            vec!["H1".to_string(), "H2".to_string(), "H3".to_string()],
            vec!["R1C1".to_string(), "R1C2".to_string()], // Missing third column
            vec!["R2C1".to_string(), "R2C2".to_string(), "R2C3".to_string()],
        ];

        // When: Converting to markdown
        let markdown = cells_to_markdown(&cells);

        // Then: Should have 3 columns in all rows (header defines width)
        assert!(markdown.contains("| H1 | H2 | H3 |"));

        // And: Short rows should be padded with empty cells
        assert!(markdown.contains("| R1C1 | R1C2 | |"));

        // And: All rows should have same number of pipes
        let lines: Vec<&str> = markdown.lines().filter(|l| !l.is_empty()).collect();
        let pipe_counts: Vec<usize> = lines
            .iter()
            .map(|line| line.chars().filter(|c| *c == '|').count())
            .collect();
        assert!(pipe_counts.iter().all(|&count| count == pipe_counts[0]));
    }

    #[test]
    fn test_markdown_single_row_table() {
        // Given: Table with only header row (no data)
        let cells = vec![vec!["OnlyHeader".to_string()]];

        // When: Converting to markdown
        let markdown = cells_to_markdown(&cells);

        // Then: Should produce header and separator only
        assert!(markdown.contains("| OnlyHeader |"));
        assert!(markdown.contains("|------|"));

        // And: Should have exactly 2 lines (header + separator)
        let lines: Vec<&str> = markdown.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_markdown_single_column_table() {
        // Given: Table with only one column
        let cells = vec![
            vec!["Header".to_string()],
            vec!["Data1".to_string()],
            vec!["Data2".to_string()],
        ];

        // When: Converting to markdown
        let markdown = cells_to_markdown(&cells);

        // Then: Should produce valid single-column table
        assert!(markdown.contains("| Header |"));
        assert!(markdown.contains("|------|"));
        assert!(markdown.contains("| Data1 |"));
        assert!(markdown.contains("| Data2 |"));
    }

    #[test]
    fn test_markdown_special_characters() {
        // Given: Cells with markdown special characters (but not pipes)
        let cells = vec![
            vec!["*Header*".to_string(), "#Title".to_string()],
            vec!["**Bold**".to_string(), "~~Strike~~".to_string()],
        ];

        // When: Converting to markdown
        let markdown = cells_to_markdown(&cells);

        // Then: Special characters should be preserved (not escaped like pipes)
        assert!(markdown.contains("*Header*"));
        assert!(markdown.contains("#Title"));
        assert!(markdown.contains("**Bold**"));
        assert!(markdown.contains("~~Strike~~"));
    }

    #[test]
    fn test_markdown_unicode_content() {
        // Given: Cells with unicode characters (emoji, accents, etc.)
        let cells = vec![
            vec!["Emoji".to_string(), "Accents".to_string()],
            vec!["🎉 Party".to_string(), "Café".to_string()],
        ];

        // When: Converting to markdown
        let markdown = cells_to_markdown(&cells);

        // Then: Unicode should be preserved
        assert!(markdown.contains("🎉 Party"));
        assert!(markdown.contains("Café"));
    }
}
