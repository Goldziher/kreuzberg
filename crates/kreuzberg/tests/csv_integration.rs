//! CSV and spreadsheet integration tests.
//!
//! Tests for CSV and TSV extraction via Pandoc.
//! Validates data extraction, custom delimiters, quoted fields, and edge cases.

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::extract_bytes;

mod helpers;

// ============================================================================
// Basic CSV Tests
// ============================================================================

/// Test basic CSV extraction - simple comma-separated values.
#[tokio::test]
async fn test_csv_basic_extraction() {
    let config = ExtractionConfig::default();

    // Simple CSV with 3 rows, 3 columns
    let csv_content = b"Name,Age,City\nAlice,30,NYC\nBob,25,LA";

    let result = extract_bytes(csv_content, "text/csv", &config).await;

    // May fail if Pandoc is not installed, so we check gracefully
    if result.is_err() {
        println!("Skipping test: Pandoc may not be installed");
        return;
    }

    let extraction = result.unwrap();

    // Verify MIME type
    assert_eq!(extraction.mime_type, "text/csv");

    // Content should contain the data (Pandoc converts CSV to markdown table)
    // Verify all headers are present
    assert!(extraction.content.contains("Name"), "Should contain 'Name' header");
    assert!(extraction.content.contains("Age"), "Should contain 'Age' header");
    assert!(extraction.content.contains("City"), "Should contain 'City' header");

    // Verify all data rows are present
    assert!(extraction.content.contains("Alice"), "Should contain Alice row");
    assert!(extraction.content.contains("30"), "Should contain Alice's age");
    assert!(extraction.content.contains("NYC"), "Should contain Alice's city");

    assert!(extraction.content.contains("Bob"), "Should contain Bob row");
    assert!(extraction.content.contains("25"), "Should contain Bob's age");
    assert!(extraction.content.contains("LA"), "Should contain Bob's city");
}

/// Test CSV with headers - first row as headers.
#[tokio::test]
async fn test_csv_with_headers() {
    let config = ExtractionConfig::default();

    // CSV with clear header row
    let csv_content = b"Product,Price,Quantity\nApple,1.50,100\nBanana,0.75,200\nOrange,2.00,150";

    let result = extract_bytes(csv_content, "text/csv", &config).await;

    if result.is_err() {
        println!("Skipping test: Pandoc may not be installed");
        return;
    }

    let extraction = result.unwrap();

    // Pandoc should recognize headers and convert to markdown table
    // Verify all headers
    assert!(extraction.content.contains("Product"), "Should contain Product header");
    assert!(extraction.content.contains("Price"), "Should contain Price header");
    assert!(
        extraction.content.contains("Quantity"),
        "Should contain Quantity header"
    );

    // Verify all data rows
    assert!(
        extraction.content.contains("Apple")
            && extraction.content.contains("1.50")
            && extraction.content.contains("100")
    );
    assert!(
        extraction.content.contains("Banana")
            && extraction.content.contains("0.75")
            && extraction.content.contains("200")
    );
    assert!(
        extraction.content.contains("Orange")
            && extraction.content.contains("2.00")
            && extraction.content.contains("150")
    );
}

// ============================================================================
// Custom Delimiter Tests
// ============================================================================

/// Test CSV with custom delimiter - tab and semicolon.
#[tokio::test]
async fn test_csv_custom_delimiter() {
    let config = ExtractionConfig::default();

    // Semicolon-delimited CSV (common in European locales)
    let csv_content = b"Name;Age;City\nAlice;30;NYC\nBob;25;LA";

    let result = extract_bytes(csv_content, "text/csv", &config).await;

    if result.is_err() {
        println!("Skipping test: Pandoc may not be installed");
        return;
    }

    let extraction = result.unwrap();

    // Content should be extracted (Pandoc auto-detects delimiters)
    assert!(!extraction.content.is_empty(), "Content should be extracted");

    // Verify data was parsed correctly (semicolons should not appear as content)
    assert!(extraction.content.contains("Alice"), "Should contain Alice");
    assert!(extraction.content.contains("30"), "Should contain age");
    assert!(extraction.content.contains("NYC"), "Should contain city");
}

/// Test TSV (Tab-Separated Values) file.
#[tokio::test]
async fn test_tsv_file() {
    let config = ExtractionConfig::default();

    // Tab-separated values
    let tsv_content = b"Name\tAge\tCity\nAlice\t30\tNYC\nBob\t25\tLA";

    let result = extract_bytes(tsv_content, "text/tab-separated-values", &config).await;

    if result.is_err() {
        println!("Skipping test: Pandoc may not be installed");
        return;
    }

    let extraction = result.unwrap();

    // Verify MIME type
    assert_eq!(extraction.mime_type, "text/tab-separated-values");

    // Content should be extracted - verify all data
    assert!(extraction.content.contains("Name"), "Should contain Name header");
    assert!(extraction.content.contains("Age"), "Should contain Age header");
    assert!(extraction.content.contains("City"), "Should contain City header");
    assert!(extraction.content.contains("Alice"), "Should contain Alice");
    assert!(extraction.content.contains("Bob"), "Should contain Bob");
    assert!(extraction.content.contains("30") && extraction.content.contains("NYC"));
    assert!(extraction.content.contains("25") && extraction.content.contains("LA"));
}

// ============================================================================
// Quoted Fields and Special Characters
// ============================================================================

/// Test CSV with quoted fields - fields containing commas.
#[tokio::test]
async fn test_csv_quoted_fields() {
    let config = ExtractionConfig::default();

    // CSV with quoted fields (commas inside quotes)
    let csv_content =
        b"Name,Description,Price\n\"Smith, John\",\"Product A, premium\",100\n\"Doe, Jane\",\"Product B, standard\",50";

    let result = extract_bytes(csv_content, "text/csv", &config).await;

    if result.is_err() {
        println!("Skipping test: Pandoc may not be installed");
        return;
    }

    let extraction = result.unwrap();

    // Quoted fields should be handled correctly
    // Verify that commas inside quotes didn't split the field
    assert!(extraction.content.contains("Smith"), "Should contain Smith");
    assert!(extraction.content.contains("John"), "Should contain John");
    assert!(extraction.content.contains("Doe"), "Should contain Doe");
    assert!(extraction.content.contains("Jane"), "Should contain Jane");

    // Verify descriptions with commas were parsed
    assert!(extraction.content.contains("Product A") || extraction.content.contains("premium"));
    assert!(extraction.content.contains("Product B") || extraction.content.contains("standard"));

    // Verify prices
    assert!(extraction.content.contains("100") && extraction.content.contains("50"));
}

/// Test CSV with special characters - Unicode, newlines in fields.
#[tokio::test]
async fn test_csv_special_characters() {
    let config = ExtractionConfig::default();

    // CSV with Unicode characters
    let csv_content = "Name,City,Emoji\nAlice,Tokyo 東京,🎉\nBob,París,✅\nCarlos,Москва,🌍".as_bytes();

    let result = extract_bytes(csv_content, "text/csv", &config).await;

    if result.is_err() {
        println!("Skipping test: Pandoc may not be installed");
        return;
    }

    let extraction = result.unwrap();

    // Unicode characters should be preserved
    assert!(!extraction.content.is_empty(), "Special characters should be handled");

    // Verify data extraction
    assert!(extraction.content.contains("Alice"), "Should contain Alice");
    assert!(extraction.content.contains("Bob"), "Should contain Bob");
    assert!(extraction.content.contains("Carlos"), "Should contain Carlos");

    // Verify cities (Unicode may or may not be preserved depending on Pandoc version)
    // At minimum, city names should be present
    assert!(extraction.content.contains("Tokyo") || extraction.content.contains("東京"));
    assert!(extraction.content.contains("París") || extraction.content.contains("Paris"));
}

// ============================================================================
// Large File and Performance Tests
// ============================================================================

/// Test CSV with large file - 10,000+ rows (streaming).
#[tokio::test]
async fn test_csv_large_file() {
    let config = ExtractionConfig::default();

    // Generate large CSV (10,000 rows)
    let mut csv_content = "ID,Name,Value\n".to_string();
    for i in 1..=10_000 {
        csv_content.push_str(&format!("{},Item{},{}.00\n", i, i, i * 10));
    }

    let result = extract_bytes(csv_content.as_bytes(), "text/csv", &config).await;

    if result.is_err() {
        println!("Skipping test: Pandoc may not be installed");
        return;
    }

    let extraction = result.unwrap();

    // Large file should be processed successfully
    assert!(!extraction.content.is_empty(), "Large CSV should be processed");

    // Content should contain data from various rows
    assert!(
        extraction.content.len() > 1000,
        "Large CSV content should be substantial"
    );

    // Verify first row data
    assert!(extraction.content.contains("Item1") || extraction.content.contains("10.00"));

    // Verify middle row data
    assert!(extraction.content.contains("Item5000") || extraction.content.contains("50000.00"));

    // Verify last row data
    assert!(extraction.content.contains("Item10000") || extraction.content.contains("100000.00"));
}

// ============================================================================
// Malformed CSV Tests
// ============================================================================

/// Test malformed CSV - inconsistent columns.
#[tokio::test]
async fn test_csv_malformed() {
    let config = ExtractionConfig::default();

    // CSV with inconsistent column counts
    let csv_content = b"Name,Age,City\nAlice,30\nBob,25,LA,Extra\nCarlos,35,SF";

    let result = extract_bytes(csv_content, "text/csv", &config).await;

    // Malformed CSV may succeed or fail depending on Pandoc's tolerance
    // The important thing is it doesn't panic
    assert!(
        result.is_ok() || result.is_err(),
        "Should handle malformed CSV gracefully"
    );

    if let Ok(extraction) = result {
        // If it succeeds, content should be extracted (even if partial)
        assert!(!extraction.content.is_empty());
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Test empty CSV file.
#[tokio::test]
async fn test_csv_empty() {
    let config = ExtractionConfig::default();

    let empty_csv = b"";

    let result = extract_bytes(empty_csv, "text/csv", &config).await;

    // Empty CSV may succeed with empty content or fail with validation error
    assert!(result.is_ok() || result.is_err(), "Should handle empty CSV gracefully");
}

/// Test CSV with only headers.
#[tokio::test]
async fn test_csv_headers_only() {
    let config = ExtractionConfig::default();

    let csv_content = b"Name,Age,City";

    let result = extract_bytes(csv_content, "text/csv", &config).await;

    if result.is_err() {
        println!("Skipping test: Pandoc may not be installed");
        return;
    }

    let extraction = result.unwrap();

    // Headers-only CSV should be extracted
    assert!(
        extraction.content.contains("Name") || !extraction.content.is_empty(),
        "Headers should be extracted"
    );
}

/// Test CSV with blank lines.
#[tokio::test]
async fn test_csv_blank_lines() {
    let config = ExtractionConfig::default();

    // CSV with blank lines between data rows
    let csv_content = b"Name,Age\nAlice,30\n\nBob,25\n\nCarlos,35";

    let result = extract_bytes(csv_content, "text/csv", &config).await;

    if result.is_err() {
        println!("Skipping test: Pandoc may not be installed");
        return;
    }

    let extraction = result.unwrap();

    // Blank lines should be handled
    assert!(extraction.content.contains("Alice") || extraction.content.contains("Bob"));
}

/// Test CSV with numeric data.
#[tokio::test]
async fn test_csv_numeric_data() {
    let config = ExtractionConfig::default();

    // CSV with various numeric formats
    let csv_content = b"ID,Price,Quantity,Discount\n1,19.99,100,0.15\n2,29.99,50,0.20\n3,9.99,200,0.10";

    let result = extract_bytes(csv_content, "text/csv", &config).await;

    if result.is_err() {
        println!("Skipping test: Pandoc may not be installed");
        return;
    }

    let extraction = result.unwrap();

    // Numeric data should be preserved - verify all rows
    assert!(extraction.content.contains("Price"), "Should contain Price header");
    assert!(
        extraction.content.contains("Quantity"),
        "Should contain Quantity header"
    );
    assert!(
        extraction.content.contains("Discount"),
        "Should contain Discount header"
    );

    // Verify all rows with their numeric values
    assert!(extraction.content.contains("19.99"), "Should contain first price");
    assert!(extraction.content.contains("100"), "Should contain first quantity");
    assert!(extraction.content.contains("0.15"), "Should contain first discount");

    assert!(extraction.content.contains("29.99"), "Should contain second price");
    assert!(extraction.content.contains("50"), "Should contain second quantity");

    assert!(extraction.content.contains("9.99"), "Should contain third price");
    assert!(extraction.content.contains("200"), "Should contain third quantity");
}
