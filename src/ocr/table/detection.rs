use super::tsv_parser::TSVWord;

pub fn detect_columns(words: &[TSVWord], column_threshold: u32) -> Vec<u32> {
    if words.is_empty() {
        return Vec::new();
    }

    let mut position_groups: Vec<Vec<u32>> = Vec::new();

    for word in words {
        let x_pos = word.left;

        let mut found_group = false;
        for group in &mut position_groups {
            if let Some(&first_pos) = group.first()
                && x_pos.abs_diff(first_pos) <= column_threshold
            {
                group.push(x_pos);
                found_group = true;
                break;
            }
        }

        if !found_group {
            position_groups.push(vec![x_pos]);
        }
    }

    let mut columns: Vec<u32> = position_groups
        .iter()
        .filter(|group| !group.is_empty())
        .map(|group| {
            let mut positions = group.clone();
            let mid = positions.len() / 2;
            positions.select_nth_unstable(mid);
            positions[mid]
        })
        .collect();

    columns.sort_unstable();
    columns
}

pub fn detect_rows(words: &[TSVWord], row_threshold_ratio: f64) -> Vec<u32> {
    if words.is_empty() {
        return Vec::new();
    }

    let mut heights: Vec<u32> = words.iter().map(|w| w.height).collect();
    heights.sort_unstable();
    let median_height = heights[heights.len() / 2];
    let row_threshold = (median_height as f64 * row_threshold_ratio) as u32;

    let mut position_groups: Vec<Vec<f64>> = Vec::new();

    for word in words {
        let y_center = word.y_center();

        let mut found_group = false;
        for group in &mut position_groups {
            if let Some(&first_pos) = group.first()
                && (y_center - first_pos).abs() <= row_threshold as f64
            {
                group.push(y_center);
                found_group = true;
                break;
            }
        }

        if !found_group {
            position_groups.push(vec![y_center]);
        }
    }

    let mut rows: Vec<u32> = position_groups
        .iter()
        .filter(|group| !group.is_empty())
        .map(|group| {
            let mut positions = group.clone();
            let mid = positions.len() / 2;
            positions.select_nth_unstable_by(mid, |a, b| a.partial_cmp(b).unwrap());
            positions[mid] as u32
        })
        .collect();

    rows.sort_unstable();
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocr::table::test_helpers::create_test_word;

    #[test]
    fn test_detect_columns_simple() {
        let words = vec![
            create_test_word(100, 50, 80, 30, "A1"),
            create_test_word(105, 90, 75, 30, "A2"),
            create_test_word(300, 50, 70, 30, "B1"),
            create_test_word(295, 90, 75, 30, "B2"),
        ];

        let columns = detect_columns(&words, 20);
        assert_eq!(columns.len(), 2);
        assert!(columns[0] < 120);
        assert!(columns[1] > 280);
    }

    #[test]
    fn test_detect_columns_single() {
        let words = vec![
            create_test_word(100, 50, 80, 30, "A1"),
            create_test_word(105, 90, 75, 30, "A2"),
            create_test_word(110, 130, 70, 30, "A3"),
        ];

        let columns = detect_columns(&words, 20);
        assert_eq!(columns.len(), 1);
    }

    #[test]
    fn test_detect_columns_empty() {
        let words: Vec<TSVWord> = vec![];
        let columns = detect_columns(&words, 20);
        assert_eq!(columns.len(), 0);
    }

    #[test]
    fn test_detect_rows_simple() {
        let words = vec![
            create_test_word(100, 50, 80, 30, "A1"),
            create_test_word(300, 52, 70, 30, "B1"),
            create_test_word(100, 90, 75, 30, "A2"),
            create_test_word(300, 88, 75, 30, "B2"),
        ];

        let rows = detect_rows(&words, 0.5);
        assert_eq!(rows.len(), 2);
        assert!(rows[0] < 70);
        assert!(rows[1] > 80);
    }

    #[test]
    fn test_detect_rows_single() {
        let words = vec![
            create_test_word(100, 50, 80, 30, "A1"),
            create_test_word(300, 52, 70, 30, "B1"),
            create_test_word(500, 48, 75, 30, "C1"),
        ];

        let rows = detect_rows(&words, 0.5);
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_detect_rows_empty() {
        let words: Vec<TSVWord> = vec![];
        let rows = detect_rows(&words, 0.5);
        assert_eq!(rows.len(), 0);
    }

    #[test]
    fn test_detect_columns_sorted() {
        let words = vec![
            create_test_word(500, 50, 80, 30, "C1"),
            create_test_word(100, 50, 80, 30, "A1"),
            create_test_word(300, 50, 70, 30, "B1"),
        ];

        let columns = detect_columns(&words, 20);
        assert_eq!(columns.len(), 3);
        assert!(columns[0] < columns[1]);
        assert!(columns[1] < columns[2]);
    }

    #[test]
    fn test_detect_rows_sorted() {
        let words = vec![
            create_test_word(100, 130, 80, 30, "A3"),
            create_test_word(100, 50, 80, 30, "A1"),
            create_test_word(100, 90, 80, 30, "A2"),
        ];

        let rows = detect_rows(&words, 0.5);
        assert_eq!(rows.len(), 3);
        assert!(rows[0] < rows[1]);
        assert!(rows[1] < rows[2]);
    }
}
