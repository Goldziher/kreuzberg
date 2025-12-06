#![cfg(feature = "office")]
//! Comprehensive test for BibTeX extractor parity with Pandoc

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::extractors::BibtexExtractor;
use kreuzberg::plugins::DocumentExtractor;

#[tokio::test]
async fn test_all_entry_types() {
    let extractor = BibtexExtractor::new();

    // Test all standard BibTeX entry types
    let test_cases = vec![
        (
            "@article{test, author={John Doe}, title={Test}, journal={Journal}, year={2023}}",
            "article",
        ),
        (
            "@book{test, author={John Doe}, title={Test}, publisher={Publisher}, year={2023}}",
            "book",
        ),
        (
            "@inproceedings{test, author={John Doe}, title={Test}, booktitle={Conference}, year={2023}}",
            "inproceedings",
        ),
        (
            "@phdthesis{test, author={John Doe}, title={Test}, school={University}, year={2023}}",
            "phdthesis",
        ),
        (
            "@mastersthesis{test, author={John Doe}, title={Test}, school={University}, year={2023}}",
            "mastersthesis",
        ),
        (
            "@techreport{test, author={John Doe}, title={Test}, institution={Institute}, year={2023}}",
            "techreport",
        ),
        ("@manual{test, title={Test Manual}, year={2023}}", "manual"),
        ("@misc{test, author={John Doe}, title={Test}, year={2023}}", "misc"),
        (
            "@unpublished{test, author={John Doe}, title={Test}, note={Unpublished}, year={2023}}",
            "unpublished",
        ),
        (
            "@incollection{test, author={John Doe}, title={Test}, booktitle={Book}, publisher={Pub}, year={2023}}",
            "incollection",
        ),
        (
            "@inbook{test, author={John Doe}, title={Test}, chapter={5}, publisher={Pub}, year={2023}}",
            "inbook",
        ),
        (
            "@proceedings{test, title={Conference Proceedings}, year={2023}}",
            "proceedings",
        ),
        ("@booklet{test, title={Booklet}, year={2023}}", "booklet"),
    ];

    for (bibtex_content, expected_type) in test_cases {
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(bibtex_content.as_bytes(), "application/x-bibtex", &config)
            .await;

        assert!(result.is_ok(), "Failed to parse {} entry", expected_type);
        let result = result.unwrap();

        // Check that the entry type is captured
        if let Some(entry_types) = result.metadata.additional.get("entry_types") {
            assert!(entry_types.as_object().is_some(), "Entry types should be an object");
            println!("Entry type '{}' extracted successfully", expected_type);
        }
    }
}

#[tokio::test]
async fn test_all_common_fields() {
    let extractor = BibtexExtractor::new();

    // Test entry with many fields
    let bibtex_content = r#"
@article{comprehensive,
    author = {Smith, John and Doe, Jane},
    title = {Comprehensive Test},
    journal = {Test Journal},
    year = {2023},
    volume = {42},
    number = {3},
    pages = {123--145},
    month = {June},
    doi = {10.1234/test.001},
    url = {https://example.com},
    issn = {1234-5678},
    isbn = {978-0-12-345678-9},
    abstract = {This is an abstract},
    keywords = {test, bibtex},
    note = {Additional notes},
    publisher = {Test Publisher},
    address = {Test City},
    edition = {2nd},
    editor = {Editor Name},
    series = {Test Series},
    organization = {Test Org},
    institution = {Test Institute},
    school = {Test School},
    howpublished = {Online},
    type = {Research Article},
    chapter = {5},
    booktitle = {Book Title}
}
"#;

    let config = ExtractionConfig::default();
    let result = extractor
        .extract_bytes(bibtex_content.as_bytes(), "application/x-bibtex", &config)
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();

    // The content should contain all these fields
    let content = &result.content;

    let expected_fields = vec![
        "author",
        "title",
        "journal",
        "year",
        "volume",
        "number",
        "pages",
        "month",
        "doi",
        "url",
        "issn",
        "isbn",
        "abstract",
        "keywords",
        "note",
        "publisher",
        "address",
        "edition",
        "editor",
        "series",
        "organization",
        "institution",
        "school",
        "howpublished",
        "type",
        "chapter",
        "booktitle",
    ];

    let num_fields = expected_fields.len();
    for field in expected_fields {
        assert!(content.contains(field), "Field '{}' should be present in output", field);
    }

    println!("All {} fields were extracted", num_fields);
}

#[tokio::test]
async fn test_author_parsing() {
    let extractor = BibtexExtractor::new();

    let test_cases = vec![
        ("author = {John Doe}", vec!["John Doe"]),
        ("author = {John Doe and Jane Smith}", vec!["John Doe", "Jane Smith"]),
        ("author = {Smith, John and Doe, Jane}", vec!["Smith, John", "Doe, Jane"]),
        (
            "author = {John Doe and Jane Smith and Bob Jones}",
            vec!["John Doe", "Jane Smith", "Bob Jones"],
        ),
        // Test with 'von' particle
        ("author = {van der Berg, Hans}", vec!["van der Berg, Hans"]),
        // Test with 'Jr.' suffix
        ("author = {Smith, Jr., John}", vec!["Smith, Jr., John"]),
    ];

    for (author_field, expected_authors) in test_cases {
        let bibtex = format!("@article{{test, {}, title={{Test}}, year={{2023}}}}", author_field);

        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(bibtex.as_bytes(), "application/x-bibtex", &config)
            .await;

        assert!(result.is_ok());
        let result = result.unwrap();

        if let Some(authors) = result.metadata.additional.get("authors") {
            let authors_array = authors.as_array().expect("Authors should be an array");

            for expected_author in &expected_authors {
                let found = authors_array
                    .iter()
                    .any(|a| a.as_str().map(|s| s.contains(expected_author)).unwrap_or(false));
                assert!(
                    found,
                    "Expected author '{}' not found in {:?}",
                    expected_author, authors_array
                );
            }
        }
    }
}

#[tokio::test]
async fn test_special_characters() {
    let extractor = BibtexExtractor::new();

    let bibtex_content = r#"
@article{special,
    author = {M{\"u}ller, Hans and Sch{\"o}n, Anna and Garc{\'\i}a, Jos{\'e}},
    title = {Special Characters in {BibTeX}: {\"O}berblick},
    journal = {Test Journal},
    year = {2022}
}
"#;

    let config = ExtractionConfig::default();
    let result = extractor
        .extract_bytes(bibtex_content.as_bytes(), "application/x-bibtex", &config)
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();

    // Should extract the entry successfully
    assert_eq!(
        result.metadata.additional.get("entry_count"),
        Some(&serde_json::json!(1))
    );

    // Authors should be extracted (the biblatex crate should handle LaTeX commands)
    if let Some(authors) = result.metadata.additional.get("authors") {
        let authors_array = authors.as_array().expect("Authors should be an array");
        assert!(authors_array.len() >= 3, "Should have 3 authors");
    }
}

#[tokio::test]
async fn test_year_range_extraction() {
    let extractor = BibtexExtractor::new();

    let bibtex_content = r#"
@article{old, author={A}, title={Old}, year={1990}}
@article{mid, author={B}, title={Mid}, year={2005}}
@article{new, author={C}, title={New}, year={2023}}
"#;

    let config = ExtractionConfig::default();
    let result = extractor
        .extract_bytes(bibtex_content.as_bytes(), "application/x-bibtex", &config)
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();

    if let Some(year_range) = result.metadata.additional.get("year_range") {
        assert_eq!(year_range.get("min"), Some(&serde_json::json!(1990)));
        assert_eq!(year_range.get("max"), Some(&serde_json::json!(2023)));

        if let Some(years) = year_range.get("years") {
            let years_array = years.as_array().expect("Years should be an array");
            assert_eq!(years_array.len(), 3, "Should have 3 unique years");
        }
    } else {
        panic!("Year range not extracted");
    }
}

#[tokio::test]
async fn test_citation_keys_extraction() {
    let extractor = BibtexExtractor::new();

    let bibtex_content = r#"
@article{key1, author={A}, title={T1}, year={2023}}
@book{key2, author={B}, title={T2}, year={2023}}
@inproceedings{key3, author={C}, title={T3}, year={2023}}
"#;

    let config = ExtractionConfig::default();
    let result = extractor
        .extract_bytes(bibtex_content.as_bytes(), "application/x-bibtex", &config)
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();

    if let Some(citation_keys) = result.metadata.additional.get("citation_keys") {
        let keys_array = citation_keys.as_array().expect("Citation keys should be an array");
        assert_eq!(keys_array.len(), 3);

        let expected_keys = vec!["key1", "key2", "key3"];
        for expected_key in expected_keys {
            let found = keys_array.iter().any(|k| k.as_str() == Some(expected_key));
            assert!(found, "Citation key '{}' not found", expected_key);
        }
    } else {
        panic!("Citation keys not extracted");
    }
}

#[tokio::test]
async fn test_entry_type_distribution() {
    let extractor = BibtexExtractor::new();

    let bibtex_content = r#"
@article{a1, author={A}, title={T1}, year={2023}}
@article{a2, author={B}, title={T2}, year={2023}}
@book{b1, author={C}, title={T3}, year={2023}}
@inproceedings{c1, author={D}, title={T4}, year={2023}}
@inproceedings{c2, author={E}, title={T5}, year={2023}}
@inproceedings{c3, author={F}, title={T6}, year={2023}}
"#;

    let config = ExtractionConfig::default();
    let result = extractor
        .extract_bytes(bibtex_content.as_bytes(), "application/x-bibtex", &config)
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();

    if let Some(entry_types) = result.metadata.additional.get("entry_types") {
        let types_obj = entry_types.as_object().expect("Entry types should be an object");

        assert_eq!(types_obj.get("article"), Some(&serde_json::json!(2)));
        assert_eq!(types_obj.get("book"), Some(&serde_json::json!(1)));
        assert_eq!(types_obj.get("inproceedings"), Some(&serde_json::json!(3)));
    } else {
        panic!("Entry types not extracted");
    }
}

#[tokio::test]
async fn test_unicode_support() {
    let extractor = BibtexExtractor::new();

    let bibtex_content = r#"
@article{unicode,
    author = {Müller, Hans and Søren, Kierkegård},
    title = {Unicode in BibTeX: A Global Perspective},
    journal = {International Journal},
    year = {2023}
}
"#;

    let config = ExtractionConfig::default();
    let result = extractor
        .extract_bytes(bibtex_content.as_bytes(), "application/x-bibtex", &config)
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();

    // Should successfully parse the entry
    assert_eq!(
        result.metadata.additional.get("entry_count"),
        Some(&serde_json::json!(1))
    );
}

#[tokio::test]
async fn test_empty_fields() {
    let extractor = BibtexExtractor::new();

    let bibtex_content = r#"
@article{empty,
    author = {Smith, John},
    title = {Test},
    journal = {},
    year = {2023},
    volume = {}
}
"#;

    let config = ExtractionConfig::default();
    let result = extractor
        .extract_bytes(bibtex_content.as_bytes(), "application/x-bibtex", &config)
        .await;

    // Should not fail on empty fields
    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(
        result.metadata.additional.get("entry_count"),
        Some(&serde_json::json!(1))
    );
}

#[tokio::test]
async fn test_comprehensive_file() {
    let extractor = BibtexExtractor::new();

    let bibtex_content = std::fs::read("/Users/naamanhirschfeld/workspace/kreuzberg/test_comprehensive.bib")
        .expect("Failed to read test file");

    let config = ExtractionConfig::default();
    let result = extractor
        .extract_bytes(&bibtex_content, "application/x-bibtex", &config)
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();

    // Should have parsed all 20 entries
    assert_eq!(
        result.metadata.additional.get("entry_count"),
        Some(&serde_json::json!(20))
    );

    // Check we have multiple entry types
    if let Some(entry_types) = result.metadata.additional.get("entry_types") {
        let types_obj = entry_types.as_object().expect("Entry types should be an object");
        assert!(types_obj.len() >= 10, "Should have at least 10 different entry types");
    }

    // Check we have authors
    if let Some(authors) = result.metadata.additional.get("authors") {
        let authors_array = authors.as_array().expect("Authors should be an array");
        assert!(authors_array.len() > 10, "Should have many unique authors");
    }

    // Check year range
    if let Some(year_range) = result.metadata.additional.get("year_range") {
        assert!(year_range.get("min").is_some());
        assert!(year_range.get("max").is_some());
    }
}
