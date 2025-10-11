use super::super::error::OCRError;
use super::detection::{detect_columns, detect_rows};
use super::tsv_parser::TSVWord;

pub fn reconstruct_table(
    words: &[TSVWord],
    column_threshold: u32,
    row_threshold_ratio: f64,
) -> Result<Vec<Vec<String>>, OCRError> {
    if words.is_empty() {
        return Ok(Vec::new());
    }

    let col_positions = detect_columns(words, column_threshold);
    let row_positions = detect_rows(words, row_threshold_ratio);

    if col_positions.is_empty() || row_positions.is_empty() {
        return Ok(Vec::new());
    }

    let num_rows = row_positions.len();
    let num_cols = col_positions.len();
    let mut table: Vec<Vec<Vec<String>>> = vec![vec![vec![]; num_cols]; num_rows];

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

    let cleaned = remove_empty_rows_and_columns(result);

    Ok(cleaned)
}

fn find_row_index(row_positions: &[u32], word: &TSVWord) -> Option<usize> {
    let y_center = word.y_center() as u32;

    row_positions
        .iter()
        .enumerate()
        .min_by_key(|&(_, row_y)| row_y.abs_diff(y_center))
        .map(|(idx, _)| idx)
}

fn find_column_index(col_positions: &[u32], word: &TSVWord) -> Option<usize> {
    let x_pos = word.left;

    col_positions
        .iter()
        .enumerate()
        .min_by_key(|&(_, col_x)| col_x.abs_diff(x_pos))
        .map(|(idx, _)| idx)
}

fn remove_empty_rows_and_columns(table: Vec<Vec<String>>) -> Vec<Vec<String>> {
    if table.is_empty() {
        return table;
    }

    let non_empty_rows: Vec<Vec<String>> = table
        .into_iter()
        .filter(|row| row.iter().any(|cell| !cell.trim().is_empty()))
        .collect();

    if non_empty_rows.is_empty() {
        return Vec::new();
    }

    let num_cols = non_empty_rows[0].len();

    let non_empty_col_indices: Vec<usize> = (0..num_cols)
        .filter(|&col_idx| {
            non_empty_rows
                .iter()
                .any(|row| row.get(col_idx).is_some_and(|cell| !cell.trim().is_empty()))
        })
        .collect();

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
    use crate::ocr::table::test_helpers::create_test_word;

    #[test]
    fn test_reconstruct_simple_table() {
        let words = vec![
            create_test_word(100, 50, 80, 30, "A1"),
            create_test_word(300, 50, 70, 30, "B1"),
            create_test_word(100, 90, 75, 30, "A2"),
            create_test_word(300, 90, 75, 30, "B2"),
        ];

        let table = reconstruct_table(&words, 50, 0.5).unwrap();

        assert_eq!(table.len(), 2);
        assert_eq!(table[0].len(), 2);

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
