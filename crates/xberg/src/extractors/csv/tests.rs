use super::*;

#[test]
fn test_parse_csv_simple() {
    let rows = parse_csv("a,b,c\n1,2,3\n", ',');
    assert_eq!(rows, vec![vec!["a", "b", "c"], vec!["1", "2", "3"]]);
}

#[test]
fn test_parse_csv_quoted() {
    let rows = parse_csv("\"hello, world\",b,c\n", ',');
    assert_eq!(rows, vec![vec!["hello, world", "b", "c"]]);
}

#[test]
fn test_parse_csv_escaped_quotes() {
    let rows = parse_csv("\"say \"\"hello\"\"\",b\n", ',');
    assert_eq!(rows, vec![vec!["say \"hello\"", "b"]]);
}

#[test]
fn test_parse_tsv() {
    let rows = parse_csv("a\tb\tc\n1\t2\t3\n", '\t');
    assert_eq!(rows, vec![vec!["a", "b", "c"], vec!["1", "2", "3"]]);
}

#[test]
fn test_parse_csv_crlf() {
    let rows = parse_csv("a,b\r\n1,2\r\n", ',');
    assert_eq!(rows, vec![vec!["a", "b"], vec!["1", "2"]]);
}

#[test]
fn test_parse_csv_empty_fields() {
    let rows = parse_csv("a,,c\n", ',');
    assert_eq!(rows, vec![vec!["a", "", "c"]]);
}

#[test]
fn test_build_markdown_table() {
    let rows = vec![
        vec!["Name".to_string(), "Age".to_string()],
        vec!["Alice".to_string(), "30".to_string()],
    ];
    let md = build_markdown_table(&rows, true);
    assert!(md.contains("| Name | Age |"));
    assert!(md.contains("| --- | --- |"));
    assert!(md.contains("| Alice | 30 |"));
}

#[test]
fn should_build_markdown_with_empty_header_for_headerless_rows() {
    let rows = vec![
        vec!["Alice".to_string(), "NYC".to_string()],
        vec!["Bob".to_string(), "LA".to_string()],
    ];

    let markdown = build_markdown_table(&rows, false);

    assert!(markdown.starts_with("|  |  |\n| --- | --- |\n"));
    assert!(markdown.contains("| Alice | NYC |"));
    assert!(markdown.contains("| Bob | LA |"));
}

#[tokio::test]
async fn test_csv_extractor_plugin_interface() {
    let extractor = CsvExtractor::new();
    assert_eq!(extractor.name(), "csv-extractor");
    assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
    assert_eq!(extractor.priority(), 60);
    assert_eq!(
        extractor.supported_mime_types(),
        &["text/csv", "text/tab-separated-values"]
    );
}

#[tokio::test]
async fn test_csv_extractor_output() {
    let extractor = CsvExtractor::new();
    let config = ExtractionConfig::default();
    let csv_data = b"Name,Age,City\nAlice,30,NYC\nBob,25,LA\n";

    let result = extractor
        .extract_content(csv_data, "text/csv", &config)
        .await
        .expect("CSV extraction should succeed");

    assert!(!result.tables.is_empty());
    assert!(matches!(
        result.elements.as_slice(),
        [crate::types::internal::InternalElement {
            kind: crate::types::internal::ElementKind::Table { table_index: 0 },
            ..
        }]
    ));
    let markdown = crate::rendering::render_markdown(&result);
    assert!(markdown.contains("| Name | Age | City |"));
    assert!(markdown.contains("| Alice | 30 | NYC |"));
    assert!(!markdown.contains("Row 1:"));

    let plain = crate::rendering::render_plain(&result);
    assert_eq!(plain, "Name Age City\nAlice 30 NYC\nBob 25 LA");
    assert!(!plain.contains('|'));

    if let Some(FormatMetadata::Csv(csv_meta)) = &result.metadata.format {
        assert!(csv_meta.has_header);
    } else {
        panic!("Expected FormatMetadata::Csv");
    }
}

#[tokio::test]
async fn should_render_headerless_csv_without_promoting_first_data_row() {
    // A numeric first row is unambiguously data, so it stays headerless and
    // the first row is not promoted into the header. (An all-text first row
    // is treated as a header instead — see xberg-io/xberg#1369.)
    let extractor = CsvExtractor::new();
    let config = ExtractionConfig::default();
    let csv_data = b"1,2,3\n4,5,6\n";

    let result = extractor
        .extract_content(csv_data, "text/csv", &config)
        .await
        .expect("CSV extraction should succeed");

    let markdown = crate::rendering::render_markdown(&result);
    assert!(markdown.starts_with("|  |  |  |\n| --- | --- | --- |\n"));
    assert!(markdown.contains("| 1 | 2 | 3 |"));
    assert!(markdown.contains("| 4 | 5 | 6 |"));

    let plain = crate::rendering::render_plain(&result);
    assert_eq!(plain, "1 2 3\n4 5 6");
}

#[tokio::test]
async fn test_csv_extractor_quoted_fields() {
    let extractor = CsvExtractor::new();
    let config = ExtractionConfig::default();
    let csv_data = b"Name,Description\n\"Smith, John\",\"Has a comma, inside\"\n";

    let result = extractor
        .extract_content(csv_data, "text/csv", &config)
        .await
        .expect("CSV extraction with quoted fields should succeed");

    assert!(!result.tables.is_empty());
}

#[test]
fn test_detect_delimiter_comma() {
    assert_eq!(detect_delimiter("a,b,c\n1,2,3\n4,5,6"), ',');
}

#[test]
fn test_detect_delimiter_semicolon() {
    assert_eq!(detect_delimiter("a;b;c\n1;2;3\n4;5;6"), ';');
}

#[test]
fn test_detect_delimiter_pipe() {
    assert_eq!(detect_delimiter("a|b|c\n1|2|3\n4|5|6"), '|');
}

#[test]
fn test_detect_delimiter_tab() {
    assert_eq!(detect_delimiter("a\tb\tc\n1\t2\t3\n4\t5\t6"), '\t');
}

#[test]
fn test_detect_delimiter_semicolons_with_commas_in_values() {
    assert_eq!(
        detect_delimiter("\"last, first\";age;city\n\"doe, john\";30;NYC\n\"smith, jane\";25;LA"),
        ';'
    );
}

#[test]
fn test_decode_csv_bytes_shift_jis() {
    let shift_jis_data = vec![
        0x96u8, 0xbc, 0x91, 0x4f, 0x2c, 0x94, 0x4e, 0x97, 0xee, 0x2c, 0x8f, 0x5a, 0x8f, 0x8a,
    ];

    let decoded = decode_csv_bytes(&shift_jis_data);

    assert!(decoded.contains("名前"), "Should contain '名前' (Name)");
    assert!(decoded.contains("年齢"), "Should contain '年齢' (Age)");
    assert!(decoded.contains("住所"), "Should contain '住所' (Address)");

    assert!(
        !decoded.contains("□"),
        "Should not contain mojibake replacement characters"
    );
    assert!(
        !decoded.contains("\u{FFFD}"),
        "Should not contain Unicode replacement characters"
    );
}

#[test]
fn test_decode_csv_bytes_utf8() {
    let utf8_data = "名前,年齢,住所".as_bytes();
    let decoded = decode_csv_bytes(utf8_data);
    assert_eq!(decoded, "名前,年齢,住所");
}

#[test]
fn test_detect_header_with_numeric_data() {
    let rows = vec![
        vec!["Name".to_string(), "Age".to_string(), "Score".to_string()],
        vec!["Alice".to_string(), "30".to_string(), "95.5".to_string()],
        vec!["Bob".to_string(), "25".to_string(), "88.0".to_string()],
    ];
    assert!(detect_header(&rows), "Should detect header when data rows have numbers");
}

#[test]
fn test_detect_header_all_text() {
    let rows = vec![
        vec!["Name".to_string(), "City".to_string()],
        vec!["Alice".to_string(), "NYC".to_string()],
        vec!["Bob".to_string(), "LA".to_string()],
    ];
    assert!(
        detect_header(&rows),
        "an all-text first row is the header by CSV convention, not a blank synthetic header (#1369)"
    );
}

#[test]
fn all_text_csv_renders_first_row_as_header_not_blank() {
    // Regression for xberg-io/xberg#1369: an all-text table must render its
    // first row as the header, not a synthetic blank header with the real
    // header pushed down into the data.
    let rows = vec![
        vec!["Name".to_string(), "City".to_string()],
        vec!["Alice".to_string(), "NYC".to_string()],
        vec!["Bob".to_string(), "LA".to_string()],
    ];
    let has_header = detect_header(&rows);
    let markdown = build_markdown_table(&rows, has_header);

    assert!(has_header);
    assert!(
        !markdown.contains("|  |  |"),
        "must not emit a blank synthetic header row"
    );
    assert!(markdown.starts_with("| Name | City |\n| --- | --- |\n"));
    assert!(markdown.contains("| Alice | NYC |"));
}

#[test]
fn test_detect_header_numeric_first_row() {
    let rows = vec![
        vec!["1".to_string(), "2".to_string(), "3".to_string()],
        vec!["4".to_string(), "5".to_string(), "6".to_string()],
    ];
    assert!(
        !detect_header(&rows),
        "Should not detect header when first row has numbers"
    );
}

#[test]
fn nan_inf_are_not_numeric() {
    assert!(!is_csv_number("NaN"));
    assert!(!is_csv_number("inf"));
    assert!(!is_csv_number("-Infinity"));
    assert!(!is_csv_number("nan"));
    assert!(is_csv_number("42"));
    assert!(is_csv_number("-3.14"));
    assert!(is_csv_number("1e6"));
}

#[test]
fn header_row_of_nan_inf_labels_still_detected_as_header() {
    let rows = vec![
        vec!["NaN".to_string(), "inf".to_string(), "label".to_string()],
        vec!["1".to_string(), "2".to_string(), "x".to_string()],
    ];
    assert!(
        detect_header(&rows),
        "header of NaN/inf/label words must be treated as a header, not numeric data"
    );
}

#[test]
fn test_infer_column_types_basic() {
    let rows = vec![
        vec!["Name".to_string(), "Age".to_string(), "Date".to_string()],
        vec!["Alice".to_string(), "30".to_string(), "2024-01-15".to_string()],
        vec!["Bob".to_string(), "25".to_string(), "2024-02-20".to_string()],
    ];
    let types = infer_column_types(&rows, true);
    assert_eq!(types.len(), 3);
    assert_eq!(types[0], "text");
    assert_eq!(types[1], "numeric");
    assert_eq!(types[2], "date");
}

#[tokio::test]
async fn test_csv_extractor_header_detection_metadata() {
    let extractor = CsvExtractor::new();
    let config = ExtractionConfig::default();
    let csv_data = b"Name,Age,City\nAlice,30,NYC\nBob,25,LA\n";

    let result = extractor.extract_content(csv_data, "text/csv", &config).await.unwrap();

    if let Some(FormatMetadata::Csv(csv_meta)) = &result.metadata.format {
        assert!(csv_meta.has_header);
        assert!(csv_meta.column_types.is_some(), "Should have column_types metadata");
    } else {
        panic!("Expected FormatMetadata::Csv");
    }
}

#[tokio::test]
async fn test_csv_extractor_real_file() {
    let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/csv/data_table.csv");
    if !test_file.exists() {
        return;
    }
    let content = std::fs::read(&test_file).expect("Failed to read test CSV");
    let extractor = CsvExtractor::new();
    let config = ExtractionConfig::default();
    let result = extractor.extract_content(&content, "text/csv", &config).await.unwrap();

    assert!(!result.tables.is_empty());
}

#[tokio::test]
async fn plain_comma_csv_parses_identically_with_no_csv_config_set() {
    // Regression guard: introducing `ExtractionConfig::csv` must not change
    // default behavior when it is left `None`. ~keep
    let extractor = CsvExtractor::new();
    let config = ExtractionConfig::default();
    assert!(config.csv.is_none());
    let csv_data = b"Name,Age,City\nAlice,30,NYC\nBob,25,LA\n";

    let result = extractor
        .extract_content(csv_data, "text/csv", &config)
        .await
        .expect("CSV extraction should succeed");

    assert_eq!(
        result.tables[0].cells,
        vec![
            vec!["Name".to_string(), "Age".to_string(), "City".to_string()],
            vec!["Alice".to_string(), "30".to_string(), "NYC".to_string()],
            vec!["Bob".to_string(), "25".to_string(), "LA".to_string()],
        ]
    );

    let plain = crate::rendering::render_plain(&result);
    assert_eq!(plain, "Name Age City\nAlice 30 NYC\nBob 25 LA");
}

#[tokio::test]
async fn configured_semicolon_delimiter_is_used_instead_of_auto_detection() {
    let extractor = CsvExtractor::new();
    let config = ExtractionConfig {
        csv: Some(crate::core::config::CsvConfig {
            delimiter: Some(";".to_string()),
            comment_prefixes: vec![],
        }),
        ..Default::default()
    };
    // A single-row, single-delimiter-occurrence sample defeats consistency-based
    // auto-detection (`detect_delimiter` needs >= 2 rows to score a candidate),
    // so this only parses correctly when the configured delimiter is honored. ~keep
    let csv_data = b"Name;Age;City\nAlice;30;NYC\n";

    let result = extractor
        .extract_content(csv_data, "text/csv", &config)
        .await
        .expect("CSV extraction with configured delimiter should succeed");

    assert_eq!(
        result.tables[0].cells,
        vec![
            vec!["Name".to_string(), "Age".to_string(), "City".to_string()],
            vec!["Alice".to_string(), "30".to_string(), "NYC".to_string()],
        ]
    );

    if let Some(FormatMetadata::Csv(csv_meta)) = &result.metadata.format {
        assert_eq!(csv_meta.delimiter.as_deref(), Some(";"));
    } else {
        panic!("Expected FormatMetadata::Csv");
    }
}

#[tokio::test]
async fn configured_comment_prefix_skips_matching_lines() {
    let extractor = CsvExtractor::new();
    let config = ExtractionConfig {
        csv: Some(crate::core::config::CsvConfig {
            delimiter: None,
            comment_prefixes: vec!["#".to_string()],
        }),
        ..Default::default()
    };
    let csv_data = b"# this is a comment\nName,Age,City\n# another comment\nAlice,30,NYC\nBob,25,LA\n";

    let result = extractor
        .extract_content(csv_data, "text/csv", &config)
        .await
        .expect("CSV extraction with comment prefix should succeed");

    assert_eq!(
        result.tables[0].cells,
        vec![
            vec!["Name".to_string(), "Age".to_string(), "City".to_string()],
            vec!["Alice".to_string(), "30".to_string(), "NYC".to_string()],
            vec!["Bob".to_string(), "25".to_string(), "LA".to_string()],
        ]
    );

    let plain = crate::rendering::render_plain(&result);
    assert_eq!(plain, "Name Age City\nAlice 30 NYC\nBob 25 LA");
    assert!(!plain.contains('#'));
}

#[test]
fn strip_comment_lines_is_a_no_op_when_no_prefixes_are_configured() {
    let text = "a,b\n#c,d\n";
    assert_eq!(strip_comment_lines(text, &[]), Cow::Borrowed(text));
}

#[test]
fn strip_comment_lines_drops_lines_whose_trimmed_start_matches_a_prefix() {
    let text = "# header comment\na,b,c\n  # indented comment\n1,2,3\n";
    let filtered = strip_comment_lines(text, &["#".to_string()]);
    assert_eq!(filtered, "a,b,c\n1,2,3\n");
}

/// A cell's `|` used to go into the table's Markdown as written, which added a column
/// boundary to its row. It is escaped as `\|`, as every other table renders it.
#[tokio::test]
async fn should_escape_a_pipe_in_the_table_markdown() {
    let extractor = CsvExtractor::new();
    let config = ExtractionConfig::default();
    let csv_data = b"Name,Note\nAlice,\"a | b\"\n";

    let result = extractor
        .extract_content(csv_data, "text/csv", &config)
        .await
        .expect("CSV extraction should succeed");

    assert_eq!(
        result.tables[0].markdown,
        "| Name | Note |\n| --- | --- |\n| Alice | a \\| b |\n"
    );
    assert!(
        result.processing_warnings.is_empty(),
        "an escaped cell needs no warning, got {:?}",
        result.processing_warnings
    );
    assert_eq!(
        result.tables[0].cells,
        vec![
            vec!["Name".to_string(), "Note".to_string()],
            vec!["Alice".to_string(), "a | b".to_string()],
        ]
    );
}

/// A cell with an embedded newline (RFC 4180 permits one inside a quoted field) used to
/// split its row in the table's Markdown. The break is rendered as `<br>`, as every other
/// table renders it, so the row stays one line.
#[tokio::test]
async fn should_keep_a_cell_with_an_embedded_newline_on_one_table_row() {
    let extractor = CsvExtractor::new();
    let config = ExtractionConfig::default();
    let csv_data = b"Name,Note\nAlice,\"line one\nline two\"\n";

    let result = extractor
        .extract_content(csv_data, "text/csv", &config)
        .await
        .expect("CSV extraction should succeed");

    assert_eq!(
        result.tables[0].markdown,
        "| Name | Note |\n| --- | --- |\n| Alice | line one<br>line two |\n"
    );
    assert!(result.processing_warnings.is_empty());
}

/// An ordinary CSV file with no pipes or embedded newlines in any cell must not warn.
#[tokio::test]
async fn plain_csv_with_no_pipes_or_newlines_produces_zero_warnings() {
    let extractor = CsvExtractor::new();
    let config = ExtractionConfig::default();
    let csv_data = b"Name,Age,City\nAlice,30,NYC\nBob,25,LA\n";

    let result = extractor
        .extract_content(csv_data, "text/csv", &config)
        .await
        .expect("CSV extraction should succeed");

    assert!(
        result.processing_warnings.is_empty(),
        "an ordinary CSV file must not warn, got {:?}",
        result.processing_warnings
    );
}
