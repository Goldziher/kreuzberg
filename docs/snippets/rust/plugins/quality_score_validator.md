```rust
#[async_trait]
impl Validator for QualityValidator {
    async fn validate(
        &self,
        result: &ExtractionResult,
        _config: &ExtractionConfig,
    ) -> Result<()> {
        let score = result.metadata
            .additional
            .get("quality_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        if score < 0.5 {
            return Err(KreuzbergError::validation(format!(
                "Quality score too low: {:.2} < 0.50",
                score
            )));
        }

        Ok(())
    }
}
```
