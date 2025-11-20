//! Plugin system for extending Kreuzberg functionality.
//!
//! The plugin system provides a trait-based architecture that allows extending
//! Kreuzberg with custom extractors, OCR backends, post-processors, and validators.
//!
//! # Plugin Types
//!
//! - [`Plugin`] - Base trait that all plugins must implement
//! - [`OcrBackend`] - OCR processing plugins
//! - [`DocumentExtractor`] - Document format extraction plugins
//! - [`PostProcessor`] - Content post-processing plugins
//! - [`Validator`] - Validation plugins
//!
//! # Language Support
//!
//! Plugins can be implemented in:
//! - **Rust** (native, highest performance)
//! - **Python** (via PyO3 FFI bridge)
//! - **Node.js** (future - via napi-rs FFI bridge)
//!
//! # Lifecycle Pattern
//!
//! Plugins are stored in `Arc<dyn Trait>` for thread-safe shared access:
//!
//! ```rust
//! use kreuzberg::plugins::{Plugin, DocumentExtractor};
//! use kreuzberg::plugins::registry::get_document_extractor_registry;
//! use std::sync::Arc;
//!
//! # struct MyExtractor;
//! # use kreuzberg::types::{ExtractionResult, Metadata};
//! # impl kreuzberg::plugins::Plugin for MyExtractor {
//! #     fn name(&self) -> &str { "my" }
//! #     fn version(&self) -> String { "1.0.0".to_string() }
//! #     fn initialize(&self) -> kreuzberg::Result<()> { Ok(()) }
//! #     fn shutdown(&self) -> kreuzberg::Result<()> { Ok(()) }
//! # }
//! # #[async_trait::async_trait]
//! # impl DocumentExtractor for MyExtractor {
//! #     async fn extract_bytes(&self, _: &[u8], _: &str, _: &kreuzberg::ExtractionConfig)
//! #         -> kreuzberg::Result<ExtractionResult> {
//! #         Ok(ExtractionResult {
//! #             content: String::new(),
//! #             mime_type: String::new(),
//! #             metadata: Metadata::default(),
//! #             tables: vec![],
//! #             detected_languages: None,
//! #             chunks: None,
//! #             images: None,
//! #         })
//! #     }
//! #     async fn extract_file(&self, _: &std::path::Path, _: &str, _: &kreuzberg::ExtractionConfig)
//! #         -> kreuzberg::Result<ExtractionResult> {
//! #         Ok(ExtractionResult {
//! #             content: String::new(),
//! #             mime_type: String::new(),
//! #             metadata: Metadata::default(),
//! #             tables: vec![],
//! #             detected_languages: None,
//! #             chunks: None,
//! #             images: None,
//! #         })
//! #     }
//! #     fn supported_mime_types(&self) -> &[&str] { &[] }
//! #     fn priority(&self) -> i32 { 50 }
//! # }
//! // 1. Create plugin instance
//! let plugin = MyExtractor;
//!
//! // 2. Wrap in Arc for registration
//! let plugin = Arc::new(plugin);
//!
//! // 3. Register with registry (calls initialize internally)
//! let registry = get_document_extractor_registry();
//! let mut registry = registry.write().unwrap();
//! registry.register(plugin)?;
//! # Ok::<(), kreuzberg::KreuzbergError>(())
//! ```
//!
//! # Example: Custom Document Extractor
//!
//! ```rust
//! use kreuzberg::plugins::{Plugin, DocumentExtractor};
//! use kreuzberg::{Result, ExtractionConfig};
//! use kreuzberg::types::{ExtractionResult, Metadata};
//! use async_trait::async_trait;
//! use std::path::Path;
//!
//! struct CustomJsonExtractor;
//!
//! impl Plugin for CustomJsonExtractor {
//!     fn name(&self) -> &str { "custom-json-extractor" }
//!     fn version(&self) -> String { "1.0.0".to_string() }
//!     fn initialize(&self) -> Result<()> {
//!         println!("JSON extractor initialized");
//!         Ok(())
//!     }
//!     fn shutdown(&self) -> Result<()> {
//!         println!("JSON extractor shutdown");
//!         Ok(())
//!     }
//! }
//!
//! #[async_trait]
//! impl DocumentExtractor for CustomJsonExtractor {
//!     async fn extract_bytes(&self, content: &[u8], _mime_type: &str, _config: &ExtractionConfig)
//!         -> Result<ExtractionResult> {
//!         // Parse JSON and extract all string values
//!         let json: serde_json::Value = serde_json::from_slice(content)?;
//!         let extracted_text = extract_strings_from_json(&json);
//!
//!         let mut metadata = Metadata::default();
//!         metadata.additional.insert("extracted_fields".to_string(), serde_json::json!(true));
//!
//!         Ok(ExtractionResult {
//!             content: extracted_text,
//!             mime_type: "application/json".to_string(),
//!             metadata,
//!             tables: vec![],
//!             detected_languages: None,
//!             chunks: None,
//!             images: None,
//!         })
//!     }
//!
//!     async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig)
//!         -> Result<ExtractionResult> {
//!         // Read file and delegate to extract_bytes
//!         let content = tokio::fs::read(path).await?;
//!         self.extract_bytes(&content, mime_type, config).await
//!     }
//!
//!     fn supported_mime_types(&self) -> &[&str] {
//!         &["application/json", "text/json"]
//!     }
//!
//!     fn priority(&self) -> i32 { 50 } // Default priority
//! }
//!
//! fn extract_strings_from_json(value: &serde_json::Value) -> String {
//!     match value {
//!         serde_json::Value::String(s) => format!("{}\n", s),
//!         serde_json::Value::Array(arr) => {
//!             arr.iter().map(extract_strings_from_json).collect()
//!         }
//!         serde_json::Value::Object(obj) => {
//!             obj.values().map(extract_strings_from_json).collect()
//!         }
//!         _ => String::new(),
//!     }
//! }
//! ```
//!
//! # Safety and Threading
//!
//! **CRITICAL**: All plugins must be `Send + Sync` because they are:
//! - Stored in `Arc<dyn Trait>` for shared ownership
//! - Accessed concurrently from multiple threads
//! - Called with `&self` (shared references)
//!
//! **Interior Mutability Pattern**:
//! Since plugins receive `&self` (not `&mut self`), use these for mutable state:
//! - `Mutex<T>` - Exclusive access, blocking
//! - `RwLock<T>` - Shared read, exclusive write
//! - `AtomicBool` / `AtomicU64` - Lock-free primitives
//! - `OnceCell<T>` - One-time initialization
//!
//! ```rust
//! use kreuzberg::plugins::Plugin;
//! use std::sync::Mutex;
//!
//! struct StatefulPlugin {
//!     // Use interior mutability for state
//!     call_count: std::sync::atomic::AtomicU64,
//!     cache: Mutex<Option<Vec<String>>>,
//! }
//!
//! impl Plugin for StatefulPlugin {
//!     fn name(&self) -> &str { "stateful-plugin" }
//!     fn version(&self) -> String { "1.0.0".to_string() }
//!
//!     fn initialize(&self) -> kreuzberg::Result<()> {
//!         // Modify through interior mutability
//!         let mut cache = self.cache.lock().unwrap();
//!         *cache = Some(vec!["initialized".to_string()]);
//!         Ok(())
//!     }
//!
//!     fn shutdown(&self) -> kreuzberg::Result<()> {
//!         self.call_count.store(0, std::sync::atomic::Ordering::Release);
//!         Ok(())
//!     }
//! }
//! ```

mod extractor;
mod ocr;
mod processor;
pub mod registry;
mod traits;
mod validator;

pub use extractor::{DocumentExtractor, clear_extractors, list_extractors, register_extractor, unregister_extractor};
pub use ocr::{
    OcrBackend, OcrBackendType, clear_ocr_backends, list_ocr_backends, register_ocr_backend,
    unregister_ocr_backend,
};
pub use processor::{PostProcessor, ProcessingStage, list_post_processors};
pub use traits::Plugin;
pub use validator::{Validator, clear_validators, list_validators, register_validator, unregister_validator};
