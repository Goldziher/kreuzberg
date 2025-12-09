//! OCR backend plugin trait.
//!
//! This module defines the trait for implementing custom OCR backends.

use crate::Result;
use crate::core::config::OcrConfig;
use crate::plugins::Plugin;
use crate::types::ExtractionResult;
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

/// OCR backend types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcrBackendType {
    /// Tesseract OCR (native Rust binding)
    Tesseract,
    /// EasyOCR (Python-based, via FFI)
    EasyOCR,
    /// PaddleOCR (Python-based, via FFI)
    PaddleOCR,
    /// Custom/third-party OCR backend
    Custom,
}

/// Trait for OCR backend plugins.
///
/// Implement this trait to add custom OCR capabilities. OCR backends can be:
/// - Native Rust implementations (like Tesseract)
/// - FFI bridges to Python libraries (like EasyOCR, PaddleOCR)
/// - Cloud-based OCR services (Google Vision, AWS Textract, etc.)
///
/// # Thread Safety
///
/// OCR backends must be thread-safe (`Send + Sync`) to support concurrent processing.
///
/// # Example
///
/// ```rust
/// use kreuzberg::plugins::{Plugin, OcrBackend, OcrBackendType};
/// use kreuzberg::{Result, OcrConfig};
/// use async_trait::async_trait;
/// use std::path::Path;
/// use kreuzberg::types::{ExtractionResult, Metadata};
///
/// struct CustomOcrBackend;
///
/// impl Plugin for CustomOcrBackend {
///     fn name(&self) -> &str { "custom-ocr" }
///     fn version(&self) -> String { "1.0.0".to_string() }
///     fn initialize(&self) -> Result<()> { Ok(()) }
///     fn shutdown(&self) -> Result<()> { Ok(()) }
/// }
///
/// #[async_trait]
/// impl OcrBackend for CustomOcrBackend {
///     async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
///         // Implement OCR logic here
///         Ok(ExtractionResult {
///             content: "Extracted text".to_string(),
///             mime_type: "text/plain".to_string(),
///             metadata: Metadata::default(),
///             tables: vec![],
///             detected_languages: None,
///             chunks: None,
///             images: None,
///         })
///     }
///
///     async fn process_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractionResult> {
///         let bytes = std::fs::read(path)?;
///         self.process_image(&bytes, config).await
///     }
///
///     fn supports_language(&self, lang: &str) -> bool {
///         matches!(lang, "eng" | "deu" | "fra")
///     }
///
///     fn backend_type(&self) -> OcrBackendType {
///         OcrBackendType::Custom
///     }
/// }
/// ```
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait OcrBackend: Plugin {
    /// Process an image and extract text via OCR.
    ///
    /// # Arguments
    ///
    /// * `image_bytes` - Raw image data (JPEG, PNG, TIFF, etc.)
    /// * `config` - OCR configuration (language, PSM mode, etc.)
    ///
    /// # Returns
    ///
    /// An `ExtractionResult` containing the extracted text and metadata.
    ///
    /// # Errors
    ///
    /// - `KreuzbergError::Ocr` - OCR processing failed
    /// - `KreuzbergError::Validation` - Invalid image format or configuration
    /// - `KreuzbergError::Io` - I/O errors (these always bubble up)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use kreuzberg::plugins::{Plugin, OcrBackend};
    /// # use kreuzberg::{Result, OcrConfig};
    /// # use async_trait::async_trait;
    /// # use std::path::Path;
    /// # use kreuzberg::types::{ExtractionResult, Metadata};
    /// # struct MyOcr;
    /// # impl Plugin for MyOcr {
    /// #     fn name(&self) -> &str { "my-ocr" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # use kreuzberg::plugins::OcrBackendType;
    /// # #[async_trait]
    /// # impl OcrBackend for MyOcr {
    /// #     fn supports_language(&self, _: &str) -> bool { true }
    /// #     fn backend_type(&self) -> OcrBackendType { OcrBackendType::Custom }
    /// #     async fn process_file(&self, _: &Path, _: &OcrConfig) -> Result<ExtractionResult> { todo!() }
    /// async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
    ///     // Validate image format
    ///     if image_bytes.is_empty() {
    ///         return Err(kreuzberg::KreuzbergError::Validation {
    ///             message: "Empty image data".to_string(),
    ///             source: None,
    ///         });
    ///     }
    ///
    ///     // Perform OCR processing
    ///     let text = format!("Extracted text in language: {}", config.language);
    ///
    ///     Ok(ExtractionResult {
    ///         content: text,
    ///         mime_type: "text/plain".to_string(),
    ///         metadata: Metadata::default(),
    ///         tables: vec![],
    ///         detected_languages: None,
    ///         chunks: None,
    ///         images: None,
    ///     })
    /// }
    /// # }
    /// ```
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult>;

    /// Process a file and extract text via OCR.
    ///
    /// Default implementation reads the file and calls `process_image`.
    /// Override for custom file handling or optimizations.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the image file
    /// * `config` - OCR configuration
    ///
    /// # Errors
    ///
    /// Same as `process_image`, plus file I/O errors.
    async fn process_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractionResult> {
        use crate::core::io;
        let bytes = io::read_file_async(path).await?;
        self.process_image(&bytes, config).await
    }

    /// Check if this backend supports a given language code.
    ///
    /// # Arguments
    ///
    /// * `lang` - ISO 639-2/3 language code (e.g., "eng", "deu", "fra")
    ///
    /// # Returns
    ///
    /// `true` if the language is supported, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use kreuzberg::plugins::{Plugin, OcrBackend};
    /// # use kreuzberg::Result;
    /// # use async_trait::async_trait;
    /// # use std::path::Path;
    /// # struct MyOcr { languages: Vec<String> }
    /// # impl Plugin for MyOcr {
    /// #     fn name(&self) -> &str { "my-ocr" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # use kreuzberg::plugins::OcrBackendType;
    /// # use kreuzberg::{ExtractionResult, OcrConfig};
    /// # #[async_trait]
    /// # impl OcrBackend for MyOcr {
    /// #     fn backend_type(&self) -> OcrBackendType { OcrBackendType::Custom }
    /// #     async fn process_image(&self, _: &[u8], _: &OcrConfig) -> Result<ExtractionResult> { todo!() }
    /// #     async fn process_file(&self, _: &Path, _: &OcrConfig) -> Result<ExtractionResult> { todo!() }
    /// fn supports_language(&self, lang: &str) -> bool {
    ///     self.languages.contains(&lang.to_string())
    /// }
    /// # }
    /// ```
    fn supports_language(&self, lang: &str) -> bool;

    /// Get the backend type identifier.
    ///
    /// # Returns
    ///
    /// The backend type enum value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use kreuzberg::plugins::{Plugin, OcrBackend, OcrBackendType};
    /// # use kreuzberg::Result;
    /// # use async_trait::async_trait;
    /// # use std::path::Path;
    /// # struct TesseractBackend;
    /// # impl Plugin for TesseractBackend {
    /// #     fn name(&self) -> &str { "tesseract" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # use kreuzberg::{ExtractionResult, OcrConfig};
    /// # #[async_trait]
    /// # impl OcrBackend for TesseractBackend {
    /// #     fn supports_language(&self, _: &str) -> bool { true }
    /// #     async fn process_image(&self, _: &[u8], _: &OcrConfig) -> Result<ExtractionResult> { todo!() }
    /// #     async fn process_file(&self, _: &Path, _: &OcrConfig) -> Result<ExtractionResult> { todo!() }
    /// fn backend_type(&self) -> OcrBackendType {
    ///     OcrBackendType::Tesseract
    /// }
    /// # }
    /// ```
    fn backend_type(&self) -> OcrBackendType;

    /// Optional: Get a list of all supported languages.
    ///
    /// Defaults to empty list. Override to provide comprehensive language support info.
    fn supported_languages(&self) -> Vec<String> {
        vec![]
    }

    /// Optional: Check if the backend supports table detection.
    ///
    /// Defaults to `false`. Override if your backend can detect and extract tables.
    fn supports_table_detection(&self) -> bool {
        false
    }
}

/// Register an OCR backend with the global registry.
///
/// The OCR backend will be registered with its name from the `name()` method
/// and can be used for OCR processing via the extraction pipeline.
///
/// # Arguments
///
/// * `backend` - The OCR backend implementation wrapped in Arc
///
/// # Returns
///
/// - `Ok(())` if registration succeeded
/// - `Err(...)` if validation failed or initialization failed
///
/// # Errors
///
/// - `KreuzbergError::Validation` - Invalid backend name (empty or contains whitespace)
/// - Any error from the backend's `initialize()` method
///
/// # Example
///
/// ```rust
/// use kreuzberg::plugins::{Plugin, OcrBackend, register_ocr_backend, OcrBackendType};
/// use kreuzberg::{Result, OcrConfig};
/// use kreuzberg::types::{ExtractionResult, Metadata};
/// use async_trait::async_trait;
/// use std::sync::Arc;
/// use std::path::Path;
///
/// struct CustomOcr;
///
/// impl Plugin for CustomOcr {
///     fn name(&self) -> &str { "custom-ocr" }
///     fn version(&self) -> String { "1.0.0".to_string() }
///     fn initialize(&self) -> Result<()> { Ok(()) }
///     fn shutdown(&self) -> Result<()> { Ok(()) }
/// }
///
/// #[async_trait]
/// impl OcrBackend for CustomOcr {
///     async fn process_image(&self, _: &[u8], _: &OcrConfig) -> Result<ExtractionResult> {
///         Ok(ExtractionResult {
///             content: "text".to_string(),
///             mime_type: "text/plain".to_string(),
///             metadata: Metadata::default(),
///             tables: vec![],
///             detected_languages: None,
///             chunks: None,
///             images: None,
///         })
///     }
///     fn supports_language(&self, _: &str) -> bool { true }
///     fn backend_type(&self) -> OcrBackendType { OcrBackendType::Custom }
/// }
///
/// # tokio_test::block_on(async {
/// let backend = Arc::new(CustomOcr);
/// register_ocr_backend(backend)?;
/// # Ok::<(), kreuzberg::KreuzbergError>(())
/// # });
/// ```
pub fn register_ocr_backend(backend: Arc<dyn OcrBackend>) -> crate::Result<()> {
    use crate::plugins::registry::get_ocr_backend_registry;

    let registry = get_ocr_backend_registry();
    // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
    let mut registry = registry
        .write()
        .expect("OCR backend registry lock poisoned - critical runtime error");

    registry.register(backend)
}

/// Unregister an OCR backend by name.
///
/// Removes the OCR backend from the global registry and calls its `shutdown()` method.
///
/// # Arguments
///
/// * `name` - Name of the OCR backend to unregister
///
/// # Returns
///
/// - `Ok(())` if the backend was unregistered or didn't exist
/// - `Err(...)` if the shutdown method failed
///
/// # Example
///
/// ```rust
/// use kreuzberg::plugins::unregister_ocr_backend;
///
/// # tokio_test::block_on(async {
/// unregister_ocr_backend("custom-ocr")?;
/// # Ok::<(), kreuzberg::KreuzbergError>(())
/// # });
/// ```
pub fn unregister_ocr_backend(name: &str) -> crate::Result<()> {
    use crate::plugins::registry::get_ocr_backend_registry;

    let registry = get_ocr_backend_registry();
    // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
    let mut registry = registry
        .write()
        .expect("OCR backend registry lock poisoned - critical runtime error");

    registry.remove(name)
}

/// List all registered OCR backends.
///
/// Returns the names of all OCR backends currently registered in the global registry.
///
/// # Returns
///
/// A vector of OCR backend names.
///
/// # Example
///
/// ```rust
/// use kreuzberg::plugins::list_ocr_backends;
///
/// # tokio_test::block_on(async {
/// let backends = list_ocr_backends()?;
/// for name in backends {
///     println!("Registered OCR backend: {}", name);
/// }
/// # Ok::<(), kreuzberg::KreuzbergError>(())
/// # });
/// ```
pub fn list_ocr_backends() -> crate::Result<Vec<String>> {
    use crate::plugins::registry::get_ocr_backend_registry;

    let registry = get_ocr_backend_registry();
    // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
    let registry = registry
        .read()
        .expect("OCR backend registry lock poisoned - critical runtime error");

    Ok(registry.list())
}

/// Clear all OCR backends from the global registry.
///
/// Removes all OCR backends and calls their `shutdown()` methods.
///
/// # Returns
///
/// - `Ok(())` if all backends were cleared successfully
/// - `Err(...)` if any shutdown method failed
///
/// # Example
///
/// ```rust
/// use kreuzberg::plugins::clear_ocr_backends;
///
/// # tokio_test::block_on(async {
/// clear_ocr_backends()?;
/// # Ok::<(), kreuzberg::KreuzbergError>(())
/// # });
/// ```
pub fn clear_ocr_backends() -> crate::Result<()> {
    use crate::plugins::registry::get_ocr_backend_registry;

    let registry = get_ocr_backend_registry();
    // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
    let mut registry = registry
        .write()
        .expect("OCR backend registry lock poisoned - critical runtime error");

    registry.shutdown_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockOcrBackend {
        languages: Vec<String>,
    }

    impl Plugin for MockOcrBackend {
        fn name(&self) -> &str {
            "mock-ocr"
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
    impl OcrBackend for MockOcrBackend {
        async fn process_image(&self, _image_bytes: &[u8], _config: &OcrConfig) -> Result<ExtractionResult> {
            Ok(ExtractionResult {
                content: "Mocked OCR text".to_string(),
                mime_type: "text/plain".to_string(),
                metadata: crate::types::Metadata::default(),
                tables: vec![],
                detected_languages: None,
                chunks: None,
                images: None,
            })
        }

        fn supports_language(&self, lang: &str) -> bool {
            self.languages.iter().any(|l| l == lang)
        }

        fn backend_type(&self) -> OcrBackendType {
            OcrBackendType::Custom
        }

        fn supported_languages(&self) -> Vec<String> {
            self.languages.clone()
        }
    }

    #[tokio::test]
    async fn test_ocr_backend_process_image() {
        let backend = MockOcrBackend {
            languages: vec!["eng".to_string(), "deu".to_string()],
        };

        let config = OcrConfig {
            backend: "mock".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        };

        let result = backend.process_image(b"fake image data", &config).await.unwrap();
        assert_eq!(result.content, "Mocked OCR text");
        assert_eq!(result.mime_type, "text/plain");
    }

    #[test]
    fn test_ocr_backend_supports_language() {
        let backend = MockOcrBackend {
            languages: vec!["eng".to_string(), "deu".to_string()],
        };

        assert!(backend.supports_language("eng"));
        assert!(backend.supports_language("deu"));
        assert!(!backend.supports_language("fra"));
    }

    #[test]
    fn test_ocr_backend_type() {
        let backend = MockOcrBackend {
            languages: vec!["eng".to_string()],
        };

        assert_eq!(backend.backend_type(), OcrBackendType::Custom);
    }

    #[test]
    fn test_ocr_backend_supported_languages() {
        let backend = MockOcrBackend {
            languages: vec!["eng".to_string(), "deu".to_string(), "fra".to_string()],
        };

        let supported = backend.supported_languages();
        assert_eq!(supported.len(), 3);
        assert!(supported.contains(&"eng".to_string()));
        assert!(supported.contains(&"deu".to_string()));
        assert!(supported.contains(&"fra".to_string()));
    }

    #[test]
    fn test_ocr_backend_type_variants() {
        assert_eq!(OcrBackendType::Tesseract, OcrBackendType::Tesseract);
        assert_ne!(OcrBackendType::Tesseract, OcrBackendType::EasyOCR);
        assert_ne!(OcrBackendType::EasyOCR, OcrBackendType::PaddleOCR);
        assert_ne!(OcrBackendType::PaddleOCR, OcrBackendType::Custom);
    }

    #[test]
    fn test_ocr_backend_type_debug() {
        let backend_type = OcrBackendType::Tesseract;
        let debug_str = format!("{:?}", backend_type);
        assert!(debug_str.contains("Tesseract"));
    }

    #[test]
    fn test_ocr_backend_type_clone() {
        let backend_type = OcrBackendType::EasyOCR;
        let cloned = backend_type;
        assert_eq!(backend_type, cloned);
    }

    #[test]
    fn test_ocr_backend_default_table_detection() {
        let backend = MockOcrBackend {
            languages: vec!["eng".to_string()],
        };
        assert!(!backend.supports_table_detection());
    }

    #[tokio::test]
    async fn test_ocr_backend_process_file_default_impl() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let backend = MockOcrBackend {
            languages: vec!["eng".to_string()],
        };

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"fake image data").unwrap();
        let path = temp_file.path();

        let config = OcrConfig {
            backend: "mock".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        };

        let result = backend.process_file(path, &config).await.unwrap();
        assert_eq!(result.content, "Mocked OCR text");
    }

    #[test]
    fn test_ocr_backend_plugin_interface() {
        let backend = MockOcrBackend {
            languages: vec!["eng".to_string()],
        };

        assert_eq!(backend.name(), "mock-ocr");
        assert_eq!(backend.version(), "1.0.0");
        assert!(backend.initialize().is_ok());
        assert!(backend.shutdown().is_ok());
    }

    #[test]
    fn test_ocr_backend_empty_languages() {
        let backend = MockOcrBackend { languages: vec![] };

        let supported = backend.supported_languages();
        assert_eq!(supported.len(), 0);
        assert!(!backend.supports_language("eng"));
    }

    #[tokio::test]
    async fn test_ocr_backend_with_empty_image() {
        let backend = MockOcrBackend {
            languages: vec!["eng".to_string()],
        };

        let config = OcrConfig {
            backend: "mock".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        };

        let result = backend.process_image(b"", &config).await;
        assert!(result.is_ok());
    }
}
