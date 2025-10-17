//! Validator plugin trait.
//!
//! This module defines the trait for implementing custom validation logic.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::Plugin;
use crate::types::ExtractionResult;
use async_trait::async_trait;

/// Trait for validator plugins.
///
/// Validators check extraction results for quality, completeness, or correctness.
/// Unlike post-processors, validator errors **fail fast** - if a validator returns
/// an error, the extraction fails immediately.
///
/// # Use Cases
///
/// - **Quality Gates**: Ensure extracted content meets minimum quality standards
/// - **Compliance**: Verify content meets regulatory requirements
/// - **Content Filtering**: Reject documents containing unwanted content
/// - **Format Validation**: Verify extracted content structure
/// - **Security Checks**: Scan for malicious content
///
/// # Error Handling
///
/// Validator errors are **fatal** - they cause the extraction to fail and bubble up
/// to the caller. Use validators for hard requirements that must be met.
///
/// For non-fatal checks, use post-processors instead.
///
/// # Thread Safety
///
/// Validators must be thread-safe (`Send + Sync`).
///
/// # Example
///
/// ```rust
/// use kreuzberg::plugins::{Plugin, Validator};
/// use kreuzberg::{Result, ExtractionResult, ExtractionConfig, KreuzbergError};
/// use async_trait::async_trait;
///
/// /// Validate that extracted content has minimum length
/// struct MinimumLengthValidator {
///     min_length: usize,
/// }
///
/// impl Plugin for MinimumLengthValidator {
///     fn name(&self) -> &str { "min-length-validator" }
///     fn version(&self) -> String { "1.0.0".to_string() }
///     fn initialize(&self) -> Result<()> { Ok(()) }
///     fn shutdown(&self) -> Result<()> { Ok(()) }
/// }
///
/// #[async_trait]
/// impl Validator for MinimumLengthValidator {
///     async fn validate(&self, result: &ExtractionResult, config: &ExtractionConfig)
///         -> Result<()> {
///         if result.content.len() < self.min_length {
///             return Err(KreuzbergError::validation(format!(
///                 "Content too short: {} < {} characters",
///                 result.content.len(),
///                 self.min_length
///             )));
///         }
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Validator: Plugin {
    /// Validate an extraction result.
    ///
    /// Check the extraction result and return `Ok(())` if valid, or an error
    /// if validation fails.
    ///
    /// # Arguments
    ///
    /// * `result` - The extraction result to validate
    /// * `config` - Extraction configuration
    ///
    /// # Returns
    ///
    /// - `Ok(())` if validation passes
    /// - `Err(...)` if validation fails (extraction will fail)
    ///
    /// # Errors
    ///
    /// - `KreuzbergError::Validation` - Validation failed
    /// - Any other error type appropriate for the failure
    ///
    /// # Example - Content Length Validation
    ///
    /// ```rust
    /// # use kreuzberg::plugins::{Plugin, Validator};
    /// # use kreuzberg::{Result, ExtractionResult, ExtractionConfig, KreuzbergError};
    /// # use async_trait::async_trait;
    /// # struct ContentLengthValidator { min: usize, max: usize }
    /// # impl Plugin for ContentLengthValidator {
    /// #     fn name(&self) -> &str { "length-validator" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # #[async_trait]
    /// # impl Validator for ContentLengthValidator {
    /// async fn validate(&self, result: &ExtractionResult, config: &ExtractionConfig)
    ///     -> Result<()> {
    ///     let length = result.content.len();
    ///
    ///     if length < self.min {
    ///         return Err(KreuzbergError::validation(format!(
    ///             "Content too short: {} < {} characters",
    ///             length, self.min
    ///         )));
    ///     }
    ///
    ///     if length > self.max {
    ///         return Err(KreuzbergError::validation(format!(
    ///             "Content too long: {} > {} characters",
    ///             length, self.max
    ///         )));
    ///     }
    ///
    ///     Ok(())
    /// }
    /// # }
    /// ```
    ///
    /// # Example - Quality Score Validation
    ///
    /// ```rust
    /// # use kreuzberg::plugins::{Plugin, Validator};
    /// # use kreuzberg::{Result, ExtractionResult, ExtractionConfig, KreuzbergError};
    /// # use async_trait::async_trait;
    /// # struct QualityValidator { min_score: f64 }
    /// # impl Plugin for QualityValidator {
    /// #     fn name(&self) -> &str { "quality-validator" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # #[async_trait]
    /// # impl Validator for QualityValidator {
    /// async fn validate(&self, result: &ExtractionResult, config: &ExtractionConfig)
    ///     -> Result<()> {
    ///     // Check if quality_score exists in metadata
    ///     let score = result.metadata
    ///         .get("quality_score")
    ///         .and_then(|v| v.as_f64())
    ///         .unwrap_or(0.0);
    ///
    ///     if score < self.min_score {
    ///         return Err(KreuzbergError::validation(format!(
    ///             "Quality score too low: {} < {}",
    ///             score, self.min_score
    ///         )));
    ///     }
    ///
    ///     Ok(())
    /// }
    /// # }
    /// ```
    ///
    /// # Example - Security Validation
    ///
    /// ```rust
    /// # use kreuzberg::plugins::{Plugin, Validator};
    /// # use kreuzberg::{Result, ExtractionResult, ExtractionConfig, KreuzbergError};
    /// # use async_trait::async_trait;
    /// # struct SecurityValidator { blocked_patterns: Vec<String> }
    /// # impl Plugin for SecurityValidator {
    /// #     fn name(&self) -> &str { "security-validator" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # #[async_trait]
    /// # impl Validator for SecurityValidator {
    /// async fn validate(&self, result: &ExtractionResult, config: &ExtractionConfig)
    ///     -> Result<()> {
    ///     // Check for blocked patterns
    ///     for pattern in &self.blocked_patterns {
    ///         if result.content.contains(pattern) {
    ///             return Err(KreuzbergError::validation(format!(
    ///                 "Content contains blocked pattern: {}",
    ///                 pattern
    ///             )));
    ///         }
    ///     }
    ///
    ///     Ok(())
    /// }
    /// # }
    /// ```
    async fn validate(&self, result: &ExtractionResult, config: &ExtractionConfig) -> Result<()>;

    /// Optional: Check if this validator should run for a given result.
    ///
    /// Allows conditional validation based on MIME type, metadata, or content.
    /// Defaults to `true` (always run).
    ///
    /// # Arguments
    ///
    /// * `result` - The extraction result to check
    /// * `config` - Extraction configuration
    ///
    /// # Returns
    ///
    /// `true` if the validator should run, `false` to skip.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use kreuzberg::plugins::{Plugin, Validator};
    /// # use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
    /// # use async_trait::async_trait;
    /// # struct PdfValidator;
    /// # impl Plugin for PdfValidator {
    /// #     fn name(&self) -> &str { "pdf-validator" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # #[async_trait]
    /// # impl Validator for PdfValidator {
    /// #     async fn validate(&self, _: &ExtractionResult, _: &ExtractionConfig) -> Result<()> { Ok(()) }
    /// /// Only validate PDF documents
    /// fn should_validate(&self, result: &ExtractionResult, config: &ExtractionConfig) -> bool {
    ///     result.mime_type == "application/pdf"
    /// }
    /// # }
    /// ```
    fn should_validate(&self, _result: &ExtractionResult, _config: &ExtractionConfig) -> bool {
        true
    }

    /// Optional: Get the validation priority.
    ///
    /// Higher priority validators run first. Useful for ordering validation checks
    /// (e.g., run cheap validations before expensive ones).
    ///
    /// Default priority is 50.
    ///
    /// # Returns
    ///
    /// Priority value (higher = runs earlier).
    ///
    /// # Example
    ///
    /// ```rust
    /// # use kreuzberg::plugins::{Plugin, Validator};
    /// # use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
    /// # use async_trait::async_trait;
    /// # struct FastValidator;
    /// # impl Plugin for FastValidator {
    /// #     fn name(&self) -> &str { "fast-validator" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # #[async_trait]
    /// # impl Validator for FastValidator {
    /// #     async fn validate(&self, _: &ExtractionResult, _: &ExtractionConfig) -> Result<()> { Ok(()) }
    /// /// Run this validator first (it's fast)
    /// fn priority(&self) -> i32 {
    ///     100
    /// }
    /// # }
    /// ```
    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KreuzbergError;
    use std::collections::HashMap;

    struct MockValidator {
        should_fail: bool,
    }

    impl Plugin for MockValidator {
        fn name(&self) -> &str {
            "mock-validator"
        }

        fn version(&self) -> String {
            "1.0.0".to_string()
        }

        fn initialize(&self) -> Result<()> {
            Ok(())
        }

        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl Validator for MockValidator {
        async fn validate(&self, _result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
            if self.should_fail {
                Err(KreuzbergError::validation("Validation failed".to_string()))
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_validator_success() {
        let validator = MockValidator { should_fail: false };

        let result = ExtractionResult {
            content: "test content".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: vec![],
            detected_languages: None,
        };

        let config = ExtractionConfig::default();
        assert!(validator.validate(&result, &config).await.is_ok());
    }

    #[tokio::test]
    async fn test_validator_failure() {
        let validator = MockValidator { should_fail: true };

        let result = ExtractionResult {
            content: "test content".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: vec![],
            detected_languages: None,
        };

        let config = ExtractionConfig::default();
        let validation_result = validator.validate(&result, &config).await;

        assert!(matches!(validation_result, Err(KreuzbergError::Validation { .. })));
    }

    #[test]
    fn test_validator_should_validate_default() {
        let validator = MockValidator { should_fail: false };

        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: vec![],
            detected_languages: None,
        };

        let config = ExtractionConfig::default();

        // Default implementation always returns true
        assert!(validator.should_validate(&result, &config));
    }

    #[test]
    fn test_validator_priority_default() {
        let validator = MockValidator { should_fail: false };
        assert_eq!(validator.priority(), 50);
    }

    #[tokio::test]
    async fn test_validator_plugin_interface() {
        let validator = MockValidator { should_fail: false };

        // Test Plugin trait methods
        assert_eq!(validator.name(), "mock-validator");
        assert_eq!(validator.version(), "1.0.0");
        assert!(validator.initialize().is_ok());
        assert!(validator.shutdown().is_ok());
    }

    #[tokio::test]
    async fn test_validator_empty_content() {
        let validator = MockValidator { should_fail: false };

        let result = ExtractionResult {
            content: String::new(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: vec![],
            detected_languages: None,
        };

        let config = ExtractionConfig::default();
        assert!(validator.validate(&result, &config).await.is_ok());
    }

    #[test]
    fn test_validator_should_validate_conditional() {
        // Create a validator that only validates PDFs
        struct PdfOnlyValidator;

        impl Plugin for PdfOnlyValidator {
            fn name(&self) -> &str {
                "pdf-only"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> Result<()> {
                Ok(())
            }
        }

        #[async_trait]
        impl Validator for PdfOnlyValidator {
            async fn validate(&self, _result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
                Ok(())
            }

            fn should_validate(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> bool {
                result.mime_type == "application/pdf"
            }
        }

        let validator = PdfOnlyValidator;
        let config = ExtractionConfig::default();

        let pdf_result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "application/pdf".to_string(),
            metadata: HashMap::new(),
            tables: vec![],
            detected_languages: None,
        };

        let txt_result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: vec![],
            detected_languages: None,
        };

        assert!(validator.should_validate(&pdf_result, &config));
        assert!(!validator.should_validate(&txt_result, &config));
    }

    #[test]
    fn test_validator_priority_ranges() {
        // Test different priority ranges
        struct HighPriorityValidator;
        struct LowPriorityValidator;

        impl Plugin for HighPriorityValidator {
            fn name(&self) -> &str {
                "high-priority"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> Result<()> {
                Ok(())
            }
        }

        impl Plugin for LowPriorityValidator {
            fn name(&self) -> &str {
                "low-priority"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> Result<()> {
                Ok(())
            }
        }

        #[async_trait]
        impl Validator for HighPriorityValidator {
            async fn validate(&self, _result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
                Ok(())
            }

            fn priority(&self) -> i32 {
                100
            }
        }

        #[async_trait]
        impl Validator for LowPriorityValidator {
            async fn validate(&self, _result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
                Ok(())
            }

            fn priority(&self) -> i32 {
                10
            }
        }

        let high = HighPriorityValidator;
        let low = LowPriorityValidator;

        assert_eq!(high.priority(), 100);
        assert_eq!(low.priority(), 10);
        assert!(high.priority() > low.priority());
    }

    #[tokio::test]
    async fn test_validator_error_message() {
        let validator = MockValidator { should_fail: true };

        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: vec![],
            detected_languages: None,
        };

        let config = ExtractionConfig::default();
        let err = validator.validate(&result, &config).await.unwrap_err();

        match err {
            KreuzbergError::Validation { message: msg, .. } => {
                assert_eq!(msg, "Validation failed");
            }
            _ => panic!("Expected Validation error"),
        }
    }

    #[tokio::test]
    async fn test_validator_with_metadata() {
        let validator = MockValidator { should_fail: false };

        let mut metadata = HashMap::new();
        metadata.insert("quality_score".to_string(), serde_json::json!(0.95));

        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata,
            tables: vec![],
            detected_languages: None,
        };

        let config = ExtractionConfig::default();
        assert!(validator.validate(&result, &config).await.is_ok());
    }

    #[tokio::test]
    async fn test_validator_with_tables() {
        use crate::types::Table;

        let validator = MockValidator { should_fail: false };

        let table = Table {
            cells: vec![vec!["A".to_string(), "B".to_string()]],
            markdown: "| A | B |".to_string(),
            page_number: 0,
        };

        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: vec![table],
            detected_languages: None,
        };

        let config = ExtractionConfig::default();
        assert!(validator.validate(&result, &config).await.is_ok());
    }

    #[tokio::test]
    async fn test_validator_different_mime_types() {
        let validator = MockValidator { should_fail: false };
        let config = ExtractionConfig::default();

        let mime_types = vec![
            "text/plain",
            "application/pdf",
            "application/json",
            "text/html",
            "image/png",
        ];

        for mime_type in mime_types {
            let result = ExtractionResult {
                content: "test".to_string(),
                mime_type: mime_type.to_string(),
                metadata: HashMap::new(),
                tables: vec![],
                detected_languages: None,
            };

            assert!(validator.validate(&result, &config).await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_validator_long_content() {
        let validator = MockValidator { should_fail: false };

        let result = ExtractionResult {
            content: "test content ".repeat(10000),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: vec![],
            detected_languages: None,
        };

        let config = ExtractionConfig::default();
        assert!(validator.validate(&result, &config).await.is_ok());
    }
}
