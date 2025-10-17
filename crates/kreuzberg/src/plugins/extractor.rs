//! Document extractor plugin trait.
//!
//! This module defines the trait for implementing custom document extractors.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::Plugin;
use crate::types::ExtractionResult;
use async_trait::async_trait;
use std::path::Path;

/// Trait for document extractor plugins.
///
/// Implement this trait to add support for new document formats or to override
/// built-in extraction behavior with custom logic.
///
/// # Priority System
///
/// When multiple extractors support the same MIME type, the registry selects
/// the extractor with the highest priority value. Use this to:
/// - Override built-in extractors (priority > 50)
/// - Provide fallback extractors (priority < 50)
/// - Implement specialized extractors for specific use cases
///
/// Default priority is 50.
///
/// # Thread Safety
///
/// Extractors must be thread-safe (`Send + Sync`) to support concurrent extraction.
///
/// # Example
///
/// ```rust
/// use kreuzberg::plugins::{Plugin, DocumentExtractor};
/// use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
/// use async_trait::async_trait;
/// use std::path::Path;
/// use std::collections::HashMap;
///
/// /// Custom PDF extractor with premium features
/// struct PremiumPdfExtractor;
///
/// impl Plugin for PremiumPdfExtractor {
///     fn name(&self) -> &str { "premium-pdf" }
///     fn version(&self) -> String { "2.0.0".to_string() }
///     fn initialize(&self) -> Result<()> { Ok(()) }
///     fn shutdown(&self) -> Result<()> { Ok(()) }
/// }
///
/// #[async_trait]
/// impl DocumentExtractor for PremiumPdfExtractor {
///     async fn extract_bytes(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig)
///         -> Result<ExtractionResult> {
///         // Premium extraction logic with better accuracy
///         Ok(ExtractionResult {
///             content: "Premium extracted content".to_string(),
///             mime_type: mime_type.to_string(),
///             metadata: HashMap::new(),
///             tables: vec![],
///             detected_languages: None,
///         })
///     }
///
///     async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig)
///         -> Result<ExtractionResult> {
///         let bytes = std::fs::read(path)?;
///         self.extract_bytes(&bytes, mime_type, config).await
///     }
///
///     fn supported_mime_types(&self) -> &[&str] {
///         &["application/pdf"]
///     }
///
///     fn priority(&self) -> i32 {
///         100  // Higher than default (50) - will be preferred
///     }
/// }
/// ```
#[async_trait]
pub trait DocumentExtractor: Plugin {
    /// Extract content from a byte array.
    ///
    /// This is the core extraction method that processes in-memory document data.
    ///
    /// # Arguments
    ///
    /// * `content` - Raw document bytes
    /// * `mime_type` - MIME type of the document (already validated)
    /// * `config` - Extraction configuration
    ///
    /// # Returns
    ///
    /// An `ExtractionResult` containing the extracted content, metadata, and tables.
    ///
    /// # Errors
    ///
    /// - `KreuzbergError::Parsing` - Document parsing failed
    /// - `KreuzbergError::Validation` - Invalid document structure
    /// - `KreuzbergError::Io` - I/O errors (these always bubble up)
    /// - `KreuzbergError::MissingDependency` - Required dependency not available
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use kreuzberg::plugins::{Plugin, DocumentExtractor};
    /// # use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
    /// # use async_trait::async_trait;
    /// # use std::path::Path;
    /// # struct MyExtractor;
    /// # impl Plugin for MyExtractor {
    /// #     fn name(&self) -> &str { "my-extractor" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # #[async_trait]
    /// # impl DocumentExtractor for MyExtractor {
    /// #     fn supported_mime_types(&self) -> &[&str] { &["text/plain"] }
    /// #     fn priority(&self) -> i32 { 50 }
    /// #     async fn extract_file(&self, _: &Path, _: &str, _: &ExtractionConfig) -> Result<ExtractionResult> { todo!() }
    /// async fn extract_bytes(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig)
    ///     -> Result<ExtractionResult> {
    ///     // Parse document
    ///     let text = String::from_utf8_lossy(content).to_string();
    ///
    ///     // Extract metadata
    ///     let mut metadata = std::collections::HashMap::new();
    ///     metadata.insert("byte_count".to_string(), serde_json::json!(content.len()));
    ///
    ///     Ok(ExtractionResult {
    ///         content: text,
    ///         mime_type: mime_type.to_string(),
    ///         metadata,
    ///         tables: vec![],
    ///             detected_languages: None,
    ///     })
    /// }
    /// # }
    /// ```
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult>;

    /// Extract content from a file.
    ///
    /// Default implementation reads the file and calls `extract_bytes`.
    /// Override for custom file handling, streaming, or memory optimizations.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the document file
    /// * `mime_type` - MIME type of the document (already validated)
    /// * `config` - Extraction configuration
    ///
    /// # Errors
    ///
    /// Same as `extract_bytes`, plus file I/O errors.
    ///
    /// # Example - Custom File Handling
    ///
    /// ```rust,no_run
    /// # use kreuzberg::plugins::{Plugin, DocumentExtractor};
    /// # use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
    /// # use async_trait::async_trait;
    /// # use std::path::Path;
    /// # struct StreamingExtractor;
    /// # impl Plugin for StreamingExtractor {
    /// #     fn name(&self) -> &str { "streaming" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # #[async_trait]
    /// # impl DocumentExtractor for StreamingExtractor {
    /// #     fn supported_mime_types(&self) -> &[&str] { &["text/plain"] }
    /// #     fn priority(&self) -> i32 { 50 }
    /// #     async fn extract_bytes(&self, _: &[u8], _: &str, _: &ExtractionConfig) -> Result<ExtractionResult> { todo!() }
    /// /// Override for memory-efficient streaming extraction
    /// async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig)
    ///     -> Result<ExtractionResult> {
    ///     // Stream large files instead of loading entirely into memory
    ///     let mut content = String::new();
    ///
    ///     // Use buffered reader for streaming
    ///     use std::io::{BufRead, BufReader};
    ///     let file = std::fs::File::open(path)?;
    ///     let reader = BufReader::new(file);
    ///
    ///     for line in reader.lines() {
    ///         content.push_str(&line?);
    ///         content.push('\n');
    ///     }
    ///
    ///     Ok(ExtractionResult {
    ///         content,
    ///         mime_type: mime_type.to_string(),
    ///         metadata: std::collections::HashMap::new(),
    ///         tables: vec![],
    ///             detected_languages: None,
    ///     })
    /// }
    /// # }
    /// ```
    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        use crate::core::io;
        let bytes = io::read_file_async(path).await?;
        self.extract_bytes(&bytes, mime_type, config).await
    }

    /// Get the list of MIME types supported by this extractor.
    ///
    /// Can include exact MIME types and prefix patterns:
    /// - Exact: `"application/pdf"`, `"text/plain"`
    /// - Prefix: `"image/*"` (matches any image type)
    ///
    /// # Returns
    ///
    /// A slice of MIME type strings.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use kreuzberg::plugins::{Plugin, DocumentExtractor};
    /// # use kreuzberg::Result;
    /// # use async_trait::async_trait;
    /// # use std::path::Path;
    /// # struct MultiFormatExtractor;
    /// # impl Plugin for MultiFormatExtractor {
    /// #     fn name(&self) -> &str { "multi-format" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # use kreuzberg::{ExtractionResult, ExtractionConfig};
    /// # #[async_trait]
    /// # impl DocumentExtractor for MultiFormatExtractor {
    /// #     fn priority(&self) -> i32 { 50 }
    /// #     async fn extract_bytes(&self, _: &[u8], _: &str, _: &ExtractionConfig) -> Result<ExtractionResult> { todo!() }
    /// #     async fn extract_file(&self, _: &Path, _: &str, _: &ExtractionConfig) -> Result<ExtractionResult> { todo!() }
    /// fn supported_mime_types(&self) -> &[&str] {
    ///     &[
    ///         "text/plain",
    ///         "text/markdown",
    ///         "application/json",
    ///         "application/xml",
    ///         "text/html",
    ///     ]
    /// }
    /// # }
    /// ```
    fn supported_mime_types(&self) -> &[&str];

    /// Get the priority of this extractor.
    ///
    /// Higher priority extractors are preferred when multiple extractors
    /// support the same MIME type.
    ///
    /// # Priority Guidelines
    ///
    /// - **0-25**: Fallback/low-quality extractors
    /// - **26-49**: Alternative extractors
    /// - **50**: Default priority (built-in extractors)
    /// - **51-75**: Premium/enhanced extractors
    /// - **76-100**: Specialized/high-priority extractors
    ///
    /// # Returns
    ///
    /// Priority value (default: 50)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use kreuzberg::plugins::{Plugin, DocumentExtractor};
    /// # use kreuzberg::Result;
    /// # use async_trait::async_trait;
    /// # use std::path::Path;
    /// # struct FallbackExtractor;
    /// # impl Plugin for FallbackExtractor {
    /// #     fn name(&self) -> &str { "fallback" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # use kreuzberg::{ExtractionResult, ExtractionConfig};
    /// # #[async_trait]
    /// # impl DocumentExtractor for FallbackExtractor {
    /// #     fn supported_mime_types(&self) -> &[&str] { &["text/plain"] }
    /// #     async fn extract_bytes(&self, _: &[u8], _: &str, _: &ExtractionConfig) -> Result<ExtractionResult> { todo!() }
    /// #     async fn extract_file(&self, _: &Path, _: &str, _: &ExtractionConfig) -> Result<ExtractionResult> { todo!() }
    /// fn priority(&self) -> i32 {
    ///     10  // Low priority - only used as fallback
    /// }
    /// # }
    /// ```
    fn priority(&self) -> i32 {
        50 // Default priority for extractors
    }

    /// Optional: Check if this extractor can handle a specific file.
    ///
    /// Allows for more sophisticated detection beyond MIME types.
    /// Defaults to `true` (rely on MIME type matching).
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to check
    /// * `mime_type` - Detected MIME type
    ///
    /// # Returns
    ///
    /// `true` if the extractor can handle this file, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use kreuzberg::plugins::{Plugin, DocumentExtractor};
    /// # use kreuzberg::Result;
    /// # use async_trait::async_trait;
    /// # use std::path::Path;
    /// # struct SmartExtractor;
    /// # impl Plugin for SmartExtractor {
    /// #     fn name(&self) -> &str { "smart" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # use kreuzberg::{ExtractionResult, ExtractionConfig};
    /// # #[async_trait]
    /// # impl DocumentExtractor for SmartExtractor {
    /// #     fn supported_mime_types(&self) -> &[&str] { &["application/pdf"] }
    /// #     fn priority(&self) -> i32 { 50 }
    /// #     async fn extract_bytes(&self, _: &[u8], _: &str, _: &ExtractionConfig) -> Result<ExtractionResult> { todo!() }
    /// #     async fn extract_file(&self, _: &Path, _: &str, _: &ExtractionConfig) -> Result<ExtractionResult> { todo!() }
    /// /// Only handle PDFs that are searchable (have text layer)
    /// fn can_handle(&self, path: &Path, mime_type: &str) -> bool {
    ///     if mime_type != "application/pdf" {
    ///         return false;
    ///     }
    ///
    ///     // Check if PDF has text layer (simplified example)
    ///     // In real implementation, analyze PDF structure here
    ///     let _ = path; // Use path for PDF analysis
    ///     true  // Simplified - always accept
    /// }
    /// # }
    /// ```
    fn can_handle(&self, _path: &Path, _mime_type: &str) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MockExtractor {
        mime_types: Vec<&'static str>,
        priority: i32,
    }

    impl Plugin for MockExtractor {
        fn name(&self) -> &str {
            "mock-extractor"
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
    impl DocumentExtractor for MockExtractor {
        async fn extract_bytes(
            &self,
            content: &[u8],
            mime_type: &str,
            _config: &ExtractionConfig,
        ) -> Result<ExtractionResult> {
            Ok(ExtractionResult {
                content: String::from_utf8_lossy(content).to_string(),
                mime_type: mime_type.to_string(),
                metadata: HashMap::new(),
                tables: vec![],
                detected_languages: None,
            })
        }

        fn supported_mime_types(&self) -> &[&str] {
            &self.mime_types
        }

        fn priority(&self) -> i32 {
            self.priority
        }
    }

    #[tokio::test]
    async fn test_document_extractor_extract_bytes() {
        let extractor = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 50,
        };

        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(b"test content", "text/plain", &config)
            .await
            .unwrap();

        assert_eq!(result.content, "test content");
        assert_eq!(result.mime_type, "text/plain");
    }

    #[test]
    fn test_document_extractor_supported_mime_types() {
        let extractor = MockExtractor {
            mime_types: vec!["text/plain", "text/markdown"],
            priority: 50,
        };

        let supported = extractor.supported_mime_types();
        assert_eq!(supported.len(), 2);
        assert!(supported.contains(&"text/plain"));
        assert!(supported.contains(&"text/markdown"));
    }

    #[test]
    fn test_document_extractor_priority() {
        let low_priority = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 10,
        };

        let high_priority = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 100,
        };

        assert_eq!(low_priority.priority(), 10);
        assert_eq!(high_priority.priority(), 100);
    }

    #[test]
    fn test_document_extractor_can_handle_default() {
        let extractor = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 50,
        };

        use std::path::PathBuf;
        let path = PathBuf::from("test.txt");

        // Default implementation always returns true
        assert!(extractor.can_handle(&path, "text/plain"));
        assert!(extractor.can_handle(&path, "application/pdf")); // Even for unsupported types
    }

    #[tokio::test]
    async fn test_document_extractor_extract_file_default_impl() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let extractor = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 50,
        };

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"file content").unwrap();
        let path = temp_file.path();

        let config = ExtractionConfig::default();
        let result = extractor.extract_file(path, "text/plain", &config).await.unwrap();

        assert_eq!(result.content, "file content");
        assert_eq!(result.mime_type, "text/plain");
    }

    #[tokio::test]
    async fn test_document_extractor_empty_content() {
        let extractor = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 50,
        };

        let config = ExtractionConfig::default();
        let result = extractor.extract_bytes(b"", "text/plain", &config).await.unwrap();

        assert_eq!(result.content, "");
        assert_eq!(result.mime_type, "text/plain");
    }

    #[tokio::test]
    async fn test_document_extractor_non_utf8_content() {
        let extractor = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 50,
        };

        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(&invalid_utf8, "text/plain", &config)
            .await
            .unwrap();

        // Should handle non-UTF8 gracefully using lossy conversion
        assert!(!result.content.is_empty());
    }

    #[test]
    fn test_document_extractor_plugin_interface() {
        let extractor = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 50,
        };

        // Test Plugin trait methods
        assert_eq!(extractor.name(), "mock-extractor");
        assert_eq!(extractor.version(), "1.0.0");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_document_extractor_default_priority() {
        // Create a minimal extractor that uses default priority
        struct DefaultPriorityExtractor;

        impl Plugin for DefaultPriorityExtractor {
            fn name(&self) -> &str {
                "default"
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
        impl DocumentExtractor for DefaultPriorityExtractor {
            async fn extract_bytes(
                &self,
                _content: &[u8],
                _mime_type: &str,
                _config: &ExtractionConfig,
            ) -> Result<ExtractionResult> {
                Ok(ExtractionResult {
                    content: String::new(),
                    mime_type: String::new(),
                    metadata: HashMap::new(),
                    tables: vec![],
                    detected_languages: None,
                })
            }

            fn supported_mime_types(&self) -> &[&str] {
                &["text/plain"]
            }
            // Uses default priority() implementation
        }

        let extractor = DefaultPriorityExtractor;
        assert_eq!(extractor.priority(), 50);
    }

    #[test]
    fn test_document_extractor_empty_mime_types() {
        let extractor = MockExtractor {
            mime_types: vec![],
            priority: 50,
        };

        assert_eq!(extractor.supported_mime_types().len(), 0);
    }

    #[test]
    fn test_document_extractor_wildcard_mime_types() {
        let extractor = MockExtractor {
            mime_types: vec!["text/*", "image/*"],
            priority: 50,
        };

        let supported = extractor.supported_mime_types();
        assert_eq!(supported.len(), 2);
        assert!(supported.contains(&"text/*"));
        assert!(supported.contains(&"image/*"));
    }

    #[test]
    fn test_document_extractor_priority_ranges() {
        // Test different priority ranges
        let fallback = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 10,
        };
        let alternative = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 40,
        };
        let default = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 50,
        };
        let premium = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 75,
        };
        let specialized = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 100,
        };

        assert!(fallback.priority() < alternative.priority());
        assert!(alternative.priority() < default.priority());
        assert!(default.priority() < premium.priority());
        assert!(premium.priority() < specialized.priority());
    }

    #[tokio::test]
    async fn test_document_extractor_preserves_mime_type() {
        let extractor = MockExtractor {
            mime_types: vec!["application/json"],
            priority: 50,
        };

        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(b"{\"key\":\"value\"}", "application/json", &config)
            .await
            .unwrap();

        assert_eq!(result.mime_type, "application/json");
    }
}
