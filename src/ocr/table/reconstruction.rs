//! Table reconstruction from detected words and structure

use super::super::error::OCRError;
use super::detection::{detect_columns, detect_rows};
use super::tsv_parser::TSVWord;

/// Reconstruct table structure from words
///
/// Takes detected words and reconstructs a 2D table by:
/// 1. Detecting column and row positions
/// 2. Assigning words to cells based on position
/// 3. Combining words within the same cell
///
/// # Arguments
///
/// * `words` - Vector of TSVWord structs
/// * `column_threshold` - Maximum horizontal distance to group words into same column
/// * `row_threshold_ratio` - Ratio of median height to use as row grouping threshold
///
/// # Returns
///
/// 2D vector representing the table (rows × columns)
pub fn reconstruct_table(
    words: &[TSVWord],
    column_threshold: u32,
    row_threshold_ratio: f64,
) -> Result<Vec<Vec<String>>, OCRError> {
    if words.is_empty() {
        return Ok(Vec::new());
    }

    // Detect table structure
    let col_positions = detect_columns(words, column_threshold);
    let row_positions = detect_rows(words, row_threshold_ratio);

    if col_positions.is_empty() || row_positions.is_empty() {
        return Ok(Vec::new());
    }

    // Initialize table grid
    let num_rows = row_positions.len();
    let num_cols = col_positions.len();
    let mut table: Vec<Vec<Vec<String>>> = vec![vec![vec![]; num_cols]; num_rows];

    // Assign words to cells
    for word in words {
        let row_idx = find_row_index(&row_positions, word);
        let col_idx = find_column_index(&col_positions, word);

        if let (Some(r), Some(c)) = (row_idx, col_idx)
            && r < num_rows
            && c < num_cols
        {
            table[r][c].push(word.text.clone());
        }
    }

    // Combine words within cells
    let result: Vec<Vec<String>> = table
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|cell_words| {
                    if cell_words.is_empty() {
                        String::new()
                    } else {
                        cell_words.join(" ")
                    }
                })
                .collect()
        })
        .collect();

    // Remove empty rows and columns
    let cleaned = remove_empty_rows_and_columns(result);

    Ok(cleaned)
}

/// Find which row a word belongs to based on its y-center
fn find_row_index(row_positions: &[u32], word: &TSVWord) -> Option<usize> {
    let y_center = word.y_center() as u32;

    // Find closest row
    row_positions
        .iter()
        .enumerate()
        .min_by_key(|&(_, row_y)| row_y.abs_diff(y_center))
        .map(|(idx, _)| idx)
}

/// Find which column a word belongs to based on its x-position
fn find_column_index(col_positions: &[u32], word: &TSVWord) -> Option<usize> {
    let x_pos = word.left;

    // Find closest column
    col_positions
        .iter()
        .enumerate()
        .min_by_key(|&(_, col_x)| col_x.abs_diff(x_pos))
        .map(|(idx, _)| idx)
}

/// Remove rows and columns that are entirely empty
fn remove_empty_rows_and_columns(table: Vec<Vec<String>>) -> Vec<Vec<String>> {
    if table.is_empty() {
        return table;
    }

    // Find non-empty rows
    let non_empty_rows: Vec<Vec<String>> = table
        .into_iter()
        .filter(|row| row.iter().any(|cell| !cell.trim().is_empty()))
        .collect();

    if non_empty_rows.is_empty() {
        return Vec::new();
    }

    let num_cols = non_empty_rows[0].len();

    // Find non-empty column indices
    let non_empty_col_indices: Vec<usize> = (0..num_cols)
        .filter(|&col_idx| {
            non_empty_rows
                .iter()
                .any(|row| row.get(col_idx).is_some_and(|cell| !cell.trim().is_empty()))
        })
        .collect();

    // Keep only non-empty columns
    non_empty_rows
        .into_iter()
        .map(|row| {
            non_empty_col_indices
                .iter()
                .filter_map(|&idx| row.get(idx).cloned())
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_word(left: u32, top: u32, width: u32, height: u32, text: &str) -> TSVWord {
        TSVWord {
            level: 5,
            page_num: 1,
            block_num: 0,
            par_num: 0,
            line_num: 0,
            word_num: 0,
            left,
            top,
            width,
            height,
            conf: 95.0,
            text: text.to_string(),
        }
    }

    #[test]
    fn test_reconstruct_simple_table() {
        let words = vec![
            // Row 1
            create_test_word(100, 50, 80, 30, "A1"),
            create_test_word(300, 50, 70, 30, "B1"),
            // Row 2
            create_test_word(100, 90, 75, 30, "A2"),
            create_test_word(300, 90, 75, 30, "B2"),
        ];

        let table = reconstruct_table(&words, 50, 0.5).unwrap();

        assert_eq!(table.len(), 2); // 2 rows
        assert_eq!(table[0].len(), 2); // 2 columns

        assert_eq!(table[0][0], "A1");
        assert_eq!(table[0][1], "B1");
        assert_eq!(table[1][0], "A2");
        assert_eq!(table[1][1], "B2");
    }

    #[test]
    fn test_reconstruct_empty() {
        let words: Vec<TSVWord> = vec![];
        let table = reconstruct_table(&words, 50, 0.5).unwrap();
        assert!(table.is_empty());
    }

    #[test]
    fn test_reconstruct_with_empty_cells() {
        let words = vec![
            create_test_word(100, 50, 80, 30, "A1"),
            // B1 is empty
            create_test_word(100, 90, 75, 30, "A2"),
            create_test_word(300, 90, 75, 30, "B2"),
        ];

        let table = reconstruct_table(&words, 50, 0.5).unwrap();

        assert_eq!(table.len(), 2);
        assert_eq!(table[0].len(), 2);

        assert_eq!(table[0][0], "A1");
        assert_eq!(table[0][1], "");
        assert_eq!(table[1][0], "A2");
        assert_eq!(table[1][1], "B2");
    }

    #[test]
    fn test_reconstruct_multi_word_cells() {
        let words = vec![
            create_test_word(100, 50, 40, 30, "Hello"),
            create_test_word(150, 50, 50, 30, "World"),
            create_test_word(300, 50, 40, 30, "Test"),
        ];

        // Use column_threshold of 60 to group "Hello" and "World" together
        let table = reconstruct_table(&words, 60, 0.5).unwrap();

        assert_eq!(table.len(), 1);
        assert_eq!(table[0].len(), 2);

        assert_eq!(table[0][0], "Hello World");
        assert_eq!(table[0][1], "Test");
    }

    #[test]
    fn test_remove_empty_rows() {
        let table = vec![
            vec!["A1".to_string(), "B1".to_string()],
            vec!["".to_string(), "".to_string()],
            vec!["A3".to_string(), "B3".to_string()],
        ];

        let cleaned = remove_empty_rows_and_columns(table);

        assert_eq!(cleaned.len(), 2);
        assert_eq!(cleaned[0][0], "A1");
        assert_eq!(cleaned[1][0], "A3");
    }

    #[test]
    fn test_remove_empty_columns() {
        let table = vec![
            vec!["A1".to_string(), "".to_string(), "C1".to_string()],
            vec!["A2".to_string(), "".to_string(), "C2".to_string()],
        ];

        let cleaned = remove_empty_rows_and_columns(table);

        assert_eq!(cleaned[0].len(), 2);
        assert_eq!(cleaned[0][0], "A1");
        assert_eq!(cleaned[0][1], "C1");
        assert_eq!(cleaned[1][0], "A2");
        assert_eq!(cleaned[1][1], "C2");
    }

    #[test]
    fn test_remove_all_empty() {
        let table = vec![
            vec!["".to_string(), "".to_string()],
            vec!["".to_string(), "".to_string()],
        ];

        let cleaned = remove_empty_rows_and_columns(table);
        assert!(cleaned.is_empty());
    }
}
