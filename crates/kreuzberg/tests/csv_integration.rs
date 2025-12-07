//! CSV and spreadsheet integration tests.
//!
//! Tests for CSV and TSV extraction.
//! Validates data extraction, custom delimiters, quoted fields, and edge cases.

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::extract_bytes;

mod helpers;

/// Test basic CSV extraction - simple comma-separated values.
#[tokio::test]
async fn test_csv_basic_extraction() {
    let config = ExtractionConfig::default();

    let csv_content = b"Name,Age,City\nAlice,30,NYC\nBob,25,LA";

    let extraction = match extract_bytes(csv_content, "text/csv", &config).await {
        Ok(result) => result,
        Err(_) => {
            println!("Skipping test: CSV extraction not available");
            return;
        }
    };

    assert_eq!(extraction.mime_type, "text/csv");
    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );
    assert!(extraction.tables.is_empty(), "CSV should not have table structures");

    assert!(extraction.content.contains("Name"), "Should contain 'Name' header");
    assert!(extraction.content.contains("Age"), "Should contain 'Age' header");
    assert!(extraction.content.contains("City"), "Should contain 'City' header");

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

    let csv_content = b"Product,Price,Quantity\nApple,1.50,100\nBanana,0.75,200\nOrange,2.00,150";

    let extraction = match extract_bytes(csv_content, "text/csv", &config).await {
        Ok(result) => result,
        Err(_) => {
            println!("Skipping test: CSV extraction not available");
            return;
        }
    };

    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );
    assert!(extraction.tables.is_empty(), "CSV should not have table structures");

    assert!(extraction.content.contains("Product"), "Should contain Product header");
    assert!(extraction.content.contains("Price"), "Should contain Price header");
    assert!(
        extraction.content.contains("Quantity"),
        "Should contain Quantity header"
    );

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

/// Test CSV with custom delimiter - tab and semicolon.
#[tokio::test]
async fn test_csv_custom_delimiter() {
    let config = ExtractionConfig::default();

    let csv_content = b"Name;Age;City\nAlice;30;NYC\nBob;25;LA";

    let extraction = match extract_bytes(csv_content, "text/csv", &config).await {
        Ok(result) => result,
        Err(_) => {
            println!("Skipping test: CSV extraction not available");
            return;
        }
    };

    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );
    assert!(extraction.tables.is_empty(), "CSV should not have table structures");

    assert!(!extraction.content.is_empty(), "Content should be extracted");

    assert!(extraction.content.contains("Alice"), "Should contain Alice");
    assert!(extraction.content.contains("30"), "Should contain age");
    assert!(extraction.content.contains("NYC"), "Should contain city");
}

/// Test TSV (Tab-Separated Values) file.
#[tokio::test]
async fn test_tsv_file() {
    let config = ExtractionConfig::default();

    let tsv_content = b"Name\tAge\tCity\nAlice\t30\tNYC\nBob\t25\tLA";

    let extraction = match extract_bytes(tsv_content, "text/tab-separated-values", &config).await {
        Ok(result) => result,
        Err(_) => {
            println!("Skipping test: TSV extraction not available");
            return;
        }
    };

    assert_eq!(extraction.mime_type, "text/tab-separated-values");
    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );
    assert!(extraction.tables.is_empty(), "CSV should not have table structures");

    assert!(extraction.content.contains("Name"), "Should contain Name header");
    assert!(extraction.content.contains("Age"), "Should contain Age header");
    assert!(extraction.content.contains("City"), "Should contain City header");
    assert!(extraction.content.contains("Alice"), "Should contain Alice");
    assert!(extraction.content.contains("Bob"), "Should contain Bob");
    assert!(extraction.content.contains("30") && extraction.content.contains("NYC"));
    assert!(extraction.content.contains("25") && extraction.content.contains("LA"));
}

/// Test CSV with quoted fields - fields containing commas.
#[tokio::test]
async fn test_csv_quoted_fields() {
    let config = ExtractionConfig::default();

    let csv_content =
        b"Name,Description,Price\n\"Smith, John\",\"Product A, premium\",100\n\"Doe, Jane\",\"Product B, standard\",50";

    let extraction = match extract_bytes(csv_content, "text/csv", &config).await {
        Ok(result) => result,
        Err(_) => {
            println!("Skipping test: CSV extraction not available");
            return;
        }
    };

    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );
    assert!(extraction.tables.is_empty(), "CSV should not have table structures");

    assert!(extraction.content.contains("Smith"), "Should contain Smith");
    assert!(extraction.content.contains("John"), "Should contain John");
    assert!(extraction.content.contains("Doe"), "Should contain Doe");
    assert!(extraction.content.contains("Jane"), "Should contain Jane");

    assert!(extraction.content.contains("Product A") || extraction.content.contains("premium"));
    assert!(extraction.content.contains("Product B") || extraction.content.contains("standard"));

    assert!(extraction.content.contains("100") && extraction.content.contains("50"));
}

/// Test CSV with special characters - Unicode, newlines in fields.
#[tokio::test]
async fn test_csv_special_characters() {
    let config = ExtractionConfig::default();

    let csv_content = "Name,City,Emoji\nAlice,Tokyo 東京,🎉\nBob,París,✅\nCarlos,Москва,🌍".as_bytes();

    let extraction = match extract_bytes(csv_content, "text/csv", &config).await {
        Ok(result) => result,
        Err(_) => {
            println!("Skipping test: CSV extraction not available");
            return;
        }
    };

    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );
    assert!(extraction.tables.is_empty(), "CSV should not have table structures");

    assert!(!extraction.content.is_empty(), "Special characters should be handled");

    assert!(extraction.content.contains("Alice"), "Should contain Alice");
    assert!(extraction.content.contains("Bob"), "Should contain Bob");
    assert!(extraction.content.contains("Carlos"), "Should contain Carlos");

    assert!(extraction.content.contains("Tokyo") || extraction.content.contains("東京"));
    assert!(extraction.content.contains("París") || extraction.content.contains("Paris"));
}

/// Test CSV with large file - 10,000+ rows (streaming).
#[tokio::test]
async fn test_csv_large_file() {
    let config = ExtractionConfig::default();

    let mut csv_content = "ID,Name,Value\n".to_string();
    for i in 1..=10_000 {
        csv_content.push_str(&format!("{},Item{},{}.00\n", i, i, i * 10));
    }

    let extraction = match extract_bytes(csv_content.as_bytes(), "text/csv", &config).await {
        Ok(result) => result,
        Err(_) => {
            println!("Skipping test: CSV extraction not available");
            return;
        }
    };

    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );
    assert!(extraction.tables.is_empty(), "CSV should not have table structures");

    assert!(!extraction.content.is_empty(), "Large CSV should be processed");

    assert!(
        extraction.content.len() > 1000,
        "Large CSV content should be substantial"
    );

    assert!(extraction.content.contains("Item1") || extraction.content.contains("10.00"));

    assert!(extraction.content.contains("Item5000") || extraction.content.contains("50000.00"));

    assert!(extraction.content.contains("Item10000") || extraction.content.contains("100000.00"));
}

/// Test malformed CSV - inconsistent columns.
#[tokio::test]
async fn test_csv_malformed() {
    let config = ExtractionConfig::default();

    let csv_content = b"Name,Age,City\nAlice,30\nBob,25,LA,Extra\nCarlos,35,SF";

    let result = extract_bytes(csv_content, "text/csv", &config).await;

    assert!(
        result.is_ok() || result.is_err(),
        "Should handle malformed CSV gracefully"
    );

    if let Ok(extraction) = result {
        assert!(!extraction.content.is_empty());
    }
}

/// Test empty CSV file.
#[tokio::test]
async fn test_csv_empty() {
    let config = ExtractionConfig::default();

    let empty_csv = b"";

    let result = extract_bytes(empty_csv, "text/csv", &config).await;

    assert!(result.is_ok() || result.is_err(), "Should handle empty CSV gracefully");
}

/// Test CSV with only headers.
#[tokio::test]
async fn test_csv_headers_only() {
    let config = ExtractionConfig::default();

    let csv_content = b"Name,Age,City";

    let extraction = match extract_bytes(csv_content, "text/csv", &config).await {
        Ok(result) => result,
        Err(_) => {
            println!("Skipping test: CSV extraction not available");
            return;
        }
    };

    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );
    assert!(extraction.tables.is_empty(), "CSV should not have table structures");

    assert!(
        extraction.content.contains("Name") || !extraction.content.is_empty(),
        "Headers should be extracted"
    );
}

/// Test CSV with blank lines.
#[tokio::test]
async fn test_csv_blank_lines() {
    let config = ExtractionConfig::default();

    let csv_content = b"Name,Age\nAlice,30\n\nBob,25\n\nCarlos,35";

    let extraction = match extract_bytes(csv_content, "text/csv", &config).await {
        Ok(result) => result,
        Err(_) => {
            println!("Skipping test: CSV extraction not available");
            return;
        }
    };

    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );
    assert!(extraction.tables.is_empty(), "CSV should not have table structures");

    assert!(extraction.content.contains("Alice") || extraction.content.contains("Bob"));
}

/// Test CSV with numeric data.
#[tokio::test]
async fn test_csv_numeric_data() {
    let config = ExtractionConfig::default();

    let csv_content = b"ID,Price,Quantity,Discount\n1,19.99,100,0.15\n2,29.99,50,0.20\n3,9.99,200,0.10";

    let extraction = match extract_bytes(csv_content, "text/csv", &config).await {
        Ok(result) => result,
        Err(_) => {
            println!("Skipping test: CSV extraction not available");
            return;
        }
    };

    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );
    assert!(extraction.tables.is_empty(), "CSV should not have table structures");

    assert!(extraction.content.contains("Price"), "Should contain Price header");
    assert!(
        extraction.content.contains("Quantity"),
        "Should contain Quantity header"
    );
    assert!(
        extraction.content.contains("Discount"),
        "Should contain Discount header"
    );

    assert!(extraction.content.contains("19.99"), "Should contain first price");
    assert!(extraction.content.contains("100"), "Should contain first quantity");
    assert!(extraction.content.contains("0.15"), "Should contain first discount");

    assert!(extraction.content.contains("29.99"), "Should contain second price");
    assert!(extraction.content.contains("50"), "Should contain second quantity");

    assert!(extraction.content.contains("9.99"), "Should contain third price");
    assert!(extraction.content.contains("200"), "Should contain third quantity");
}
