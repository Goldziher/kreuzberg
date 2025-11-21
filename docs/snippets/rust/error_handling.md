```rust
async fn extract_bytes(
    &self,
    content: &[u8],
    mime_type: &str,
    config: &ExtractionConfig,
) -> Result<ExtractionResult> {
    // Validate inputs
    if content.is_empty() {
        return Err(KreuzbergError::validation("Empty content"));
    }

    // Handle errors with context
    let parsed = parse_content(content)
        .map_err(|e| KreuzbergError::parsing(
            format!("Failed to parse {}: {}", mime_type, e)
        ))?;

    Ok(result)
}
```
