```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_custom_extractor() {
        let extractor = CustomJsonExtractor;

        let json_data = br#"{"message": "Hello, world!"}"#;
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(json_data, "application/json", &config)
            .await
            .expect("Extraction failed");

        assert!(result.content.contains("Hello, world!"));
        assert_eq!(result.mime_type, "application/json");
    }
}
```
