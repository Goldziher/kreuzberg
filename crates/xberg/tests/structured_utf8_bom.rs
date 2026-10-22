//! A UTF-8 byte order mark in front of a JSON, JSON Lines, YAML, TOML or GeoJSON document must not
//! stop the structured parse. `serde_json` and the YAML parser rejected it, so these documents fell
//! back to plain text with a `parse-fallback` warning. ~keep

use xberg::core::config::ExtractionConfig;

mod helpers;
use helpers::extract_bytes_document;

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

#[tokio::test]
async fn should_parse_structured_documents_that_start_with_a_utf8_bom() {
    let cases: [(&str, &[u8]); 5] = [
        ("application/json", br#"{"name": "Ada", "role": "Engineer"}"#),
        ("application/x-ndjson", b"{\"name\": \"Ada\"}\n{\"name\": \"Grace\"}\n"),
        ("application/yaml", b"name: Ada\nrole: Engineer\n"),
        ("application/toml", b"name = \"Ada\"\nrole = \"Engineer\"\n"),
        (
            "application/geo+json",
            br#"{"type": "FeatureCollection", "features": [{"type": "Feature", "properties": {"name": "Depot"}, "geometry": {"type": "Point", "coordinates": [8.5, 47.4]}}]}"#,
        ),
    ];
    let config = ExtractionConfig::default();
    for (mime_type, body) in cases {
        let plain = extract_bytes_document(body, mime_type, &config)
            .await
            .unwrap_or_else(|error| panic!("{mime_type} without a BOM should extract: {error}"));
        let with_bom = extract_bytes_document(&[UTF8_BOM, body].concat(), mime_type, &config)
            .await
            .unwrap_or_else(|error| panic!("{mime_type} with a BOM should extract: {error}"));

        assert!(
            !with_bom
                .processing_warnings
                .iter()
                .any(|warning| warning.source == "parse-fallback"),
            "{mime_type}: a BOM must not send the document to the plain-text fallback, got {:?}",
            with_bom.processing_warnings
        );
        assert_eq!(
            with_bom.content, plain.content,
            "{mime_type}: a BOM must not change the extracted content"
        );
    }
}
