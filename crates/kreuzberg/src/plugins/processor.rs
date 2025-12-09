//! Post-processor plugin trait.
//!
//! This module defines traits for implementing custom post-processing logic.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::Plugin;
use crate::types::ExtractionResult;
use async_trait::async_trait;

/// Processing stages for post-processors.
///
/// Post-processors are executed in stage order (Early → Middle → Late).
/// Use stages to control the order of post-processing operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProcessingStage {
    /// Early stage - foundational processing.
    ///
    /// Use for:
    /// - Language detection
    /// - Character encoding normalization
    /// - Entity extraction (NER)
    /// - Text quality scoring
    Early,

    /// Middle stage - content transformation.
    ///
    /// Use for:
    /// - Keyword extraction
    /// - Token reduction
    /// - Text summarization
    /// - Semantic analysis
    Middle,

    /// Late stage - final enrichment.
    ///
    /// Use for:
    /// - Custom user hooks
    /// - Analytics/logging
    /// - Final validation
    /// - Output formatting
    Late,
}

/// Trait for post-processor plugins.
///
/// Post-processors transform or enrich extraction results after the initial
/// extraction is complete. They can:
/// - Clean and normalize text
/// - Add metadata (language, keywords, entities)
/// - Split content into chunks
/// - Score quality
/// - Apply custom transformations
///
/// # Processing Order
///
/// Post-processors are executed in stage order:
/// 1. **Early** - Language detection, entity extraction
/// 2. **Middle** - Keyword extraction, token reduction
/// 3. **Late** - Custom hooks, final validation
///
/// Within each stage, processors are executed in registration order.
///
/// # Error Handling
///
/// Post-processor errors are non-fatal by default - they're captured in metadata
/// and execution continues. To make errors fatal, return an error from `process()`.
///
/// # Thread Safety
///
/// Post-processors must be thread-safe (`Send + Sync`).
///
/// # Example
///
/// ```rust
/// use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
/// use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
/// use async_trait::async_trait;
///
/// /// Add word count metadata to extraction results
/// struct WordCountProcessor;
///
/// impl Plugin for WordCountProcessor {
///     fn name(&self) -> &str { "word-count" }
///     fn version(&self) -> String { "1.0.0".to_string() }
///     fn initialize(&self) -> Result<()> { Ok(()) }
///     fn shutdown(&self) -> Result<()> { Ok(()) }
/// }
///
/// #[async_trait]
/// impl PostProcessor for WordCountProcessor {
///     async fn process(&self, result: &mut ExtractionResult, config: &ExtractionConfig)
///         -> Result<()> {
///         // Count words
///         let word_count = result.content.split_whitespace().count();
///
///         // Add to metadata
///         result.metadata.additional.insert("word_count".to_string(), serde_json::json!(word_count));
///
///         Ok(())
///     }
///
///     fn processing_stage(&self) -> ProcessingStage {
///         ProcessingStage::Early
///     }
/// }
/// ```
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait PostProcessor: Plugin {
    /// Process an extraction result.
    ///
    /// Transform or enrich the extraction result. Can modify:
    /// - `content` - The extracted text
    /// - `metadata` - Add or update metadata fields
    /// - `tables` - Modify or enhance table data
    ///
    /// # Arguments
    ///
    /// * `result` - Mutable reference to the extraction result to process
    /// * `config` - Extraction configuration
    ///
    /// # Returns
    ///
    /// `Ok(())` if processing succeeded, `Err(...)` for fatal failures.
    ///
    /// # Errors
    ///
    /// Return errors for fatal processing failures. Non-fatal errors should be
    /// captured in metadata directly on the result.
    ///
    /// # Performance
    ///
    /// This signature avoids unnecessary cloning of large extraction results by
    /// taking a mutable reference instead of ownership. Processors modify the
    /// result in place.
    ///
    /// # Example - Language Detection
    ///
    /// ```rust
    /// # use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
    /// # use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
    /// # use async_trait::async_trait;
    /// # struct LanguageDetector;
    /// # impl Plugin for LanguageDetector {
    /// #     fn name(&self) -> &str { "language-detector" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # #[async_trait]
    /// # impl PostProcessor for LanguageDetector {
    /// #     fn processing_stage(&self) -> ProcessingStage { ProcessingStage::Early }
    /// async fn process(&self, result: &mut ExtractionResult, config: &ExtractionConfig)
    ///     -> Result<()> {
    ///     // Detect language (simplified - use real detection library in practice)
    ///     let language = "en"; // Placeholder detection
    ///
    ///     // Add to metadata
    ///     result.metadata.additional.insert("detected_language".to_string(), serde_json::json!(language));
    ///
    ///     Ok(())
    /// }
    /// # }
    /// ```
    ///
    /// # Example - Text Cleaning
    ///
    /// ```rust
    /// # use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
    /// # use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
    /// # use async_trait::async_trait;
    /// # struct TextCleaner;
    /// # impl Plugin for TextCleaner {
    /// #     fn name(&self) -> &str { "text-cleaner" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # #[async_trait]
    /// # impl PostProcessor for TextCleaner {
    /// #     fn processing_stage(&self) -> ProcessingStage { ProcessingStage::Middle }
    /// async fn process(&self, result: &mut ExtractionResult, config: &ExtractionConfig)
    ///     -> Result<()> {
    ///     // Remove excessive whitespace
    ///     result.content = result
    ///         .content
    ///         .split_whitespace()
    ///         .collect::<Vec<_>>()
    ///         .join(" ");
    ///
    ///     Ok(())
    /// }
    /// # }
    /// ```
    async fn process(&self, result: &mut ExtractionResult, config: &ExtractionConfig) -> Result<()>;

    /// Get the processing stage for this post-processor.
    ///
    /// Determines when this processor runs in the pipeline.
    ///
    /// # Returns
    ///
    /// The `ProcessingStage` (Early, Middle, or Late).
    ///
    /// # Example
    ///
    /// ```rust
    /// # use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
    /// # use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
    /// # use async_trait::async_trait;
    /// # struct MyProcessor;
    /// # impl Plugin for MyProcessor {
    /// #     fn name(&self) -> &str { "my-processor" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # #[async_trait]
    /// # impl PostProcessor for MyProcessor {
    /// #     async fn process(&self, result: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> { Ok(()) }
    /// fn processing_stage(&self) -> ProcessingStage {
    ///     ProcessingStage::Early  // Run before other processors
    /// }
    /// # }
    /// ```
    fn processing_stage(&self) -> ProcessingStage;

    /// Optional: Check if this processor should run for a given result.
    ///
    /// Allows conditional processing based on MIME type, metadata, or content.
    /// Defaults to `true` (always run).
    ///
    /// # Arguments
    ///
    /// * `result` - The extraction result to check
    /// * `config` - Extraction configuration
    ///
    /// # Returns
    ///
    /// `true` if the processor should run, `false` to skip.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
    /// # use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
    /// # use async_trait::async_trait;
    /// # struct PdfOnlyProcessor;
    /// # impl Plugin for PdfOnlyProcessor {
    /// #     fn name(&self) -> &str { "pdf-only" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # #[async_trait]
    /// # impl PostProcessor for PdfOnlyProcessor {
    /// #     fn processing_stage(&self) -> ProcessingStage { ProcessingStage::Middle }
    /// #     async fn process(&self, result: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> { Ok(()) }
    /// /// Only process PDF documents
    /// fn should_process(&self, result: &ExtractionResult, config: &ExtractionConfig) -> bool {
    ///     result.mime_type == "application/pdf"
    /// }
    /// # }
    /// ```
    fn should_process(&self, _result: &ExtractionResult, _config: &ExtractionConfig) -> bool {
        true
    }

    /// Optional: Estimate processing time in milliseconds.
    ///
    /// Used for logging and debugging. Defaults to 0 (unknown).
    ///
    /// # Arguments
    ///
    /// * `result` - The extraction result to estimate for
    ///
    /// # Returns
    ///
    /// Estimated processing time in milliseconds.
    fn estimated_duration_ms(&self, _result: &ExtractionResult) -> u64 {
        0
    }
}

/// List all registered post-processor names.
///
/// Returns a vector of all post-processor names currently registered in the
/// global registry.
///
/// # Returns
///
/// - `Ok(Vec<String>)` - Vector of post-processor names
/// - `Err(...)` if the registry lock is poisoned
///
/// # Example
///
/// ```rust
/// use kreuzberg::plugins::list_post_processors;
///
/// # tokio_test::block_on(async {
/// let processors = list_post_processors()?;
/// for name in processors {
///     println!("Registered post-processor: {}", name);
/// }
/// # Ok::<(), kreuzberg::KreuzbergError>(())
/// # });
/// ```
pub fn list_post_processors() -> crate::Result<Vec<String>> {
    use crate::plugins::registry::get_post_processor_registry;

    let registry = get_post_processor_registry();
    let registry = registry
        .read()
        .expect("~keep Failed to acquire read lock on post-processor registry"); // ~keep

    Ok(registry.list())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MockPostProcessor {
        stage: ProcessingStage,
    }

    impl Plugin for MockPostProcessor {
        fn name(&self) -> &str {
            "mock-processor"
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
    impl PostProcessor for MockPostProcessor {
        async fn process(&self, result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
            result
                .metadata
                .additional
                .insert("processed_by".to_string(), serde_json::json!(self.name()));
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            self.stage
        }
    }

    #[tokio::test]
    async fn test_post_processor_process() {
        let processor = MockPostProcessor {
            stage: ProcessingStage::Early,
        };

        let mut result = ExtractionResult {
            content: "test content".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: crate::types::Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        let config = ExtractionConfig::default();
        processor.process(&mut result, &config).await.unwrap();

        assert_eq!(result.content, "test content");
        assert_eq!(
            result.metadata.additional.get("processed_by").unwrap(),
            &serde_json::json!("mock-processor")
        );
    }

    #[test]
    fn test_processing_stage_order() {
        assert!(ProcessingStage::Early < ProcessingStage::Middle);
        assert!(ProcessingStage::Middle < ProcessingStage::Late);
    }

    #[test]
    fn test_post_processor_stage() {
        let early = MockPostProcessor {
            stage: ProcessingStage::Early,
        };
        let middle = MockPostProcessor {
            stage: ProcessingStage::Middle,
        };
        let late = MockPostProcessor {
            stage: ProcessingStage::Late,
        };

        assert_eq!(early.processing_stage(), ProcessingStage::Early);
        assert_eq!(middle.processing_stage(), ProcessingStage::Middle);
        assert_eq!(late.processing_stage(), ProcessingStage::Late);
    }

    #[test]
    fn test_post_processor_should_process_default() {
        let processor = MockPostProcessor {
            stage: ProcessingStage::Early,
        };

        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: crate::types::Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        let config = ExtractionConfig::default();

        assert!(processor.should_process(&result, &config));
    }

    #[test]
    fn test_processing_stage_equality() {
        assert_eq!(ProcessingStage::Early, ProcessingStage::Early);
        assert_ne!(ProcessingStage::Early, ProcessingStage::Middle);
        assert_ne!(ProcessingStage::Middle, ProcessingStage::Late);
    }

    #[test]
    fn test_processing_stage_clone() {
        let stage = ProcessingStage::Middle;
        let cloned = stage;
        assert_eq!(stage, cloned);
    }

    #[test]
    fn test_processing_stage_debug() {
        let stage = ProcessingStage::Early;
        let debug_str = format!("{:?}", stage);
        assert!(debug_str.contains("Early"));
    }

    #[test]
    fn test_processing_stage_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ProcessingStage::Early);
        set.insert(ProcessingStage::Middle);
        set.insert(ProcessingStage::Late);

        assert_eq!(set.len(), 3);
        assert!(set.contains(&ProcessingStage::Early));
    }

    #[tokio::test]
    async fn test_post_processor_plugin_interface() {
        let processor = MockPostProcessor {
            stage: ProcessingStage::Middle,
        };

        assert_eq!(processor.name(), "mock-processor");
        assert_eq!(processor.version(), "1.0.0");
        assert!(processor.initialize().is_ok());
        assert!(processor.shutdown().is_ok());
    }

    #[tokio::test]
    async fn test_post_processor_empty_content() {
        let processor = MockPostProcessor {
            stage: ProcessingStage::Early,
        };

        let mut result = ExtractionResult {
            content: String::new(),
            mime_type: "text/plain".to_string(),
            metadata: crate::types::Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        let config = ExtractionConfig::default();
        processor.process(&mut result, &config).await.unwrap();

        assert_eq!(result.content, "");
        assert!(result.metadata.additional.contains_key("processed_by"));
    }

    #[tokio::test]
    async fn test_post_processor_preserves_metadata() {
        let processor = MockPostProcessor {
            stage: ProcessingStage::Early,
        };

        let mut additional = HashMap::new();
        additional.insert("existing_key".to_string(), serde_json::json!("existing_value"));

        let mut result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: crate::types::Metadata {
                additional,
                ..Default::default()
            },
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        let config = ExtractionConfig::default();
        processor.process(&mut result, &config).await.unwrap();

        assert_eq!(
            result.metadata.additional.get("existing_key").unwrap(),
            &serde_json::json!("existing_value")
        );
        assert!(result.metadata.additional.contains_key("processed_by"));
    }

    #[test]
    fn test_post_processor_estimated_duration_default() {
        let processor = MockPostProcessor {
            stage: ProcessingStage::Early,
        };

        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: crate::types::Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        assert_eq!(processor.estimated_duration_ms(&result), 0);
    }

    #[test]
    fn test_post_processor_should_process_conditional() {
        struct PdfOnlyProcessor;

        impl Plugin for PdfOnlyProcessor {
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
        impl PostProcessor for PdfOnlyProcessor {
            async fn process(&self, _result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
                Ok(())
            }

            fn processing_stage(&self) -> ProcessingStage {
                ProcessingStage::Middle
            }

            fn should_process(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> bool {
                result.mime_type == "application/pdf"
            }
        }

        let processor = PdfOnlyProcessor;
        let config = ExtractionConfig::default();

        let pdf_result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "application/pdf".to_string(),
            metadata: crate::types::Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        let txt_result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: crate::types::Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        assert!(processor.should_process(&pdf_result, &config));
        assert!(!processor.should_process(&txt_result, &config));
    }

    #[tokio::test]
    async fn test_post_processor_preserves_tables() {
        use crate::types::Table;

        let processor = MockPostProcessor {
            stage: ProcessingStage::Early,
        };

        let table = Table {
            cells: vec![vec!["A".to_string(), "B".to_string()]],
            markdown: "| A | B |".to_string(),
            page_number: 0,
        };

        let mut result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: crate::types::Metadata::default(),
            tables: vec![table],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        let config = ExtractionConfig::default();
        processor.process(&mut result, &config).await.unwrap();

        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].cells.len(), 1);
    }
}
