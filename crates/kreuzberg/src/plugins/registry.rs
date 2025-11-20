//! Plugin registration and discovery.
//!
//! This module provides registries for managing plugins of different types.
//! Each plugin type (OcrBackend, DocumentExtractor, etc.) has its own registry
//! with type-safe registration and lookup.

use crate::plugins::{DocumentExtractor, OcrBackend, PostProcessor, ProcessingStage, Validator};
use crate::{KreuzbergError, Result};
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};

/// Validate a plugin name before registration.
///
/// # Rules
///
/// - Name cannot be empty
/// - Name cannot contain whitespace
/// - Name should follow kebab-case convention (lowercase with hyphens)
///
/// # Errors
///
/// Returns `ValidationError` if the name is invalid.
fn validate_plugin_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(KreuzbergError::Validation {
            message: "Plugin name cannot be empty".to_string(),
            source: None,
        });
    }

    if name.contains(char::is_whitespace) {
        return Err(KreuzbergError::Validation {
            message: format!("Plugin name '{}' cannot contain whitespace", name),
            source: None,
        });
    }

    Ok(())
}

/// Registry for OCR backend plugins.
///
/// Manages OCR backends with backend type and language-based selection.
///
/// # Thread Safety
///
/// The registry is thread-safe and can be accessed concurrently from multiple threads.
///
/// # Example
///
/// ```rust,no_run
/// use kreuzberg::plugins::registry::OcrBackendRegistry;
/// use std::sync::Arc;
///
/// let registry = OcrBackendRegistry::new();
/// // Register OCR backends
/// // registry.register(Arc::new(TesseractBackend::new()));
/// ```
pub struct OcrBackendRegistry {
    backends: HashMap<String, Arc<dyn OcrBackend>>,
}

impl OcrBackendRegistry {
    /// Create a new OCR backend registry with default backends.
    ///
    /// Registers the Tesseract backend by default if the "ocr" feature is enabled.
    pub fn new() -> Self {
        #[cfg(feature = "ocr")]
        let mut registry = Self {
            backends: HashMap::new(),
        };

        #[cfg(not(feature = "ocr"))]
        let registry = Self {
            backends: HashMap::new(),
        };

        #[cfg(feature = "ocr")]
        {
            use crate::ocr::tesseract_backend::TesseractBackend;
            if let Ok(backend) = TesseractBackend::new() {
                let _ = registry.register(Arc::new(backend));
            }
        }

        registry
    }

    /// Create a new empty OCR backend registry without default backends.
    ///
    /// This is useful for testing or when you want full control over backend registration.
    pub fn new_empty() -> Self {
        Self {
            backends: HashMap::new(),
        }
    }

    /// Register an OCR backend.
    ///
    /// # Arguments
    ///
    /// * `backend` - The OCR backend to register
    ///
    /// # Returns
    ///
    /// - `Ok(())` if registration succeeded
    /// - `Err(...)` if initialization failed
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use kreuzberg::plugins::registry::OcrBackendRegistry;
    /// # use std::sync::Arc;
    /// let mut registry = OcrBackendRegistry::new();
    /// // let backend = Arc::new(MyOcrBackend::new());
    /// // registry.register(backend)?;
    /// # Ok::<(), kreuzberg::KreuzbergError>(())
    /// ```
    pub fn register(&mut self, backend: Arc<dyn OcrBackend>) -> Result<()> {
        let name = backend.name().to_string();

        validate_plugin_name(&name)?;

        backend.initialize()?;

        self.backends.insert(name, backend);
        Ok(())
    }

    /// Get an OCR backend by name.
    ///
    /// # Arguments
    ///
    /// * `name` - Backend name
    ///
    /// # Returns
    ///
    /// The backend if found, or an error if not registered.
    pub fn get(&self, name: &str) -> Result<Arc<dyn OcrBackend>> {
        self.backends.get(name).cloned().ok_or_else(|| KreuzbergError::Plugin {
            message: format!("OCR backend '{}' not registered", name),
            plugin_name: name.to_string(),
        })
    }

    /// Get an OCR backend that supports a specific language.
    ///
    /// Returns the first backend that supports the language.
    ///
    /// # Arguments
    ///
    /// * `language` - Language code (e.g., "eng", "deu")
    ///
    /// # Returns
    ///
    /// The first backend supporting the language, or an error if none found.
    pub fn get_for_language(&self, language: &str) -> Result<Arc<dyn OcrBackend>> {
        self.backends
            .values()
            .find(|backend| backend.supports_language(language))
            .cloned()
            .ok_or_else(|| KreuzbergError::Plugin {
                message: format!("No OCR backend supports language '{}'", language),
                plugin_name: language.to_string(),
            })
    }

    /// List all registered backend names.
    pub fn list(&self) -> Vec<String> {
        self.backends.keys().cloned().collect()
    }

    /// Remove a backend from the registry.
    ///
    /// Calls `shutdown()` on the backend before removing.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        if let Some(backend) = self.backends.remove(name) {
            backend.shutdown()?;
        }
        Ok(())
    }

    /// Shutdown all backends and clear the registry.
    pub fn shutdown_all(&mut self) -> Result<()> {
        let names: Vec<_> = self.backends.keys().cloned().collect();
        for name in names {
            self.remove(&name)?;
        }
        Ok(())
    }
}

impl Default for OcrBackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry for document extractor plugins.
///
/// Manages extractors with MIME type and priority-based selection.
///
/// # Thread Safety
///
/// The registry is thread-safe and can be accessed concurrently from multiple threads.
pub struct DocumentExtractorRegistry {
    extractors: HashMap<String, BTreeMap<i32, Arc<dyn DocumentExtractor>>>,
    name_index: HashMap<String, Vec<(String, i32)>>,
}

impl DocumentExtractorRegistry {
    /// Create a new empty extractor registry.
    pub fn new() -> Self {
        Self {
            extractors: HashMap::new(),
            name_index: HashMap::new(),
        }
    }

    /// Register a document extractor.
    ///
    /// The extractor is registered for all MIME types it supports.
    ///
    /// # Arguments
    ///
    /// * `extractor` - The extractor to register
    ///
    /// # Returns
    ///
    /// - `Ok(())` if registration succeeded
    /// - `Err(...)` if initialization failed
    pub fn register(&mut self, extractor: Arc<dyn DocumentExtractor>) -> Result<()> {
        let name = extractor.name().to_string();
        let priority = extractor.priority();
        let mime_types: Vec<String> = extractor.supported_mime_types().iter().map(|s| s.to_string()).collect();

        validate_plugin_name(&name)?;

        extractor.initialize()?;

        let mut index_entries = Vec::new();

        for mime_type in &mime_types {
            self.extractors
                .entry(mime_type.clone())
                .or_default()
                .insert(priority, Arc::clone(&extractor));
            index_entries.push((mime_type.clone(), priority));
        }

        self.name_index.insert(name, index_entries);

        Ok(())
    }

    /// Get the highest priority extractor for a MIME type.
    ///
    /// # Arguments
    ///
    /// * `mime_type` - MIME type to look up
    ///
    /// # Returns
    ///
    /// The highest priority extractor, or an error if none found.
    pub fn get(&self, mime_type: &str) -> Result<Arc<dyn DocumentExtractor>> {
        if let Some(priority_map) = self.extractors.get(mime_type)
            && let Some((_priority, extractor)) = priority_map.iter().next_back()
        {
            return Ok(Arc::clone(extractor));
        }

        let mut best_match: Option<(i32, Arc<dyn DocumentExtractor>)> = None;

        for (registered_mime, priority_map) in &self.extractors {
            if registered_mime.ends_with("/*") {
                let prefix = &registered_mime[..registered_mime.len() - 1];
                if mime_type.starts_with(prefix)
                    && let Some((_priority, extractor)) = priority_map.iter().next_back()
                {
                    let priority = extractor.priority();
                    match &best_match {
                        None => best_match = Some((priority, Arc::clone(extractor))),
                        Some((current_priority, _)) => {
                            if priority > *current_priority {
                                best_match = Some((priority, Arc::clone(extractor)));
                            }
                        }
                    }
                }
            }
        }

        if let Some((_priority, extractor)) = best_match {
            return Ok(extractor);
        }

        Err(KreuzbergError::UnsupportedFormat(mime_type.to_string()))
    }

    /// List all registered extractors.
    pub fn list(&self) -> Vec<String> {
        self.name_index.keys().cloned().collect()
    }

    /// Remove an extractor from the registry.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        let index_entries = match self.name_index.remove(name) {
            Some(entries) => entries,
            None => return Ok(()),
        };

        let mut extractor_to_shutdown: Option<Arc<dyn DocumentExtractor>> = None;

        for (mime_type, priority) in index_entries {
            if let Some(priority_map) = self.extractors.get_mut(&mime_type) {
                if let Some(extractor) = priority_map.remove(&priority)
                    && extractor_to_shutdown.is_none()
                {
                    extractor_to_shutdown = Some(extractor);
                }

                if priority_map.is_empty() {
                    self.extractors.remove(&mime_type);
                }
            }
        }

        if let Some(extractor) = extractor_to_shutdown {
            extractor.shutdown()?;
        }

        Ok(())
    }

    /// Shutdown all extractors and clear the registry.
    pub fn shutdown_all(&mut self) -> Result<()> {
        let names = self.list();
        for name in names {
            self.remove(&name)?;
        }
        Ok(())
    }
}

impl Default for DocumentExtractorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry for post-processor plugins.
///
/// Manages post-processors organized by processing stage.
pub struct PostProcessorRegistry {
    processors: HashMap<ProcessingStage, BTreeMap<i32, Vec<Arc<dyn PostProcessor>>>>,
    name_index: HashMap<String, (ProcessingStage, i32)>,
}

impl PostProcessorRegistry {
    /// Create a new empty post-processor registry.
    pub fn new() -> Self {
        Self {
            processors: HashMap::new(),
            name_index: HashMap::new(),
        }
    }

    /// Register a post-processor.
    ///
    /// # Arguments
    ///
    /// * `processor` - The post-processor to register
    /// * `priority` - Execution priority (higher = runs first within stage)
    pub fn register(&mut self, processor: Arc<dyn PostProcessor>, priority: i32) -> Result<()> {
        let name = processor.name().to_string();
        let stage = processor.processing_stage();

        validate_plugin_name(&name)?;

        processor.initialize()?;

        if self.name_index.contains_key(&name) {
            self.remove(&name)?;
        }

        self.processors
            .entry(stage)
            .or_default()
            .entry(priority)
            .or_default()
            .push(Arc::clone(&processor));

        self.name_index.insert(name, (stage, priority));

        Ok(())
    }

    /// Get all processors for a specific stage, in priority order.
    ///
    /// # Arguments
    ///
    /// * `stage` - The processing stage
    ///
    /// # Returns
    ///
    /// Vector of processors in priority order (highest first).
    pub fn get_for_stage(&self, stage: ProcessingStage) -> Vec<Arc<dyn PostProcessor>> {
        let mut result = Vec::new();

        if let Some(priority_map) = self.processors.get(&stage) {
            for (_priority, processors) in priority_map.iter().rev() {
                for processor in processors {
                    result.push(Arc::clone(processor));
                }
            }
        }

        result
    }

    /// List all registered processor names.
    pub fn list(&self) -> Vec<String> {
        self.name_index.keys().cloned().collect()
    }

    /// Remove a processor from the registry.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        let (stage, priority) = match self.name_index.remove(name) {
            Some(location) => location,
            None => return Ok(()),
        };

        let processor_to_shutdown = if let Some(priority_map) = self.processors.get_mut(&stage) {
            let processor = priority_map.get_mut(&priority).and_then(|processors| {
                processors
                    .iter()
                    .position(|p| p.name() == name)
                    .map(|pos| processors.remove(pos))
            });

            if let Some(processors) = priority_map.get(&priority)
                && processors.is_empty()
            {
                priority_map.remove(&priority);
            }

            if priority_map.is_empty() {
                self.processors.remove(&stage);
            }
            processor
        } else {
            None
        };

        if let Some(processor) = processor_to_shutdown {
            processor.shutdown()?;
        }

        Ok(())
    }

    /// Shutdown all processors and clear the registry.
    pub fn shutdown_all(&mut self) -> Result<()> {
        let names = self.list();
        for name in names {
            self.remove(&name)?;
        }
        Ok(())
    }
}

impl Default for PostProcessorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry for validator plugins.
///
/// Manages validators with priority-based execution order.
pub struct ValidatorRegistry {
    validators: BTreeMap<i32, IndexMap<String, Arc<dyn Validator>>>,
}

impl ValidatorRegistry {
    /// Create a new empty validator registry.
    pub fn new() -> Self {
        Self {
            validators: BTreeMap::new(),
        }
    }

    /// Register a validator.
    ///
    /// # Arguments
    ///
    /// * `validator` - The validator to register
    pub fn register(&mut self, validator: Arc<dyn Validator>) -> Result<()> {
        let name = validator.name().to_string();
        let priority = validator.priority();

        validate_plugin_name(&name)?;

        validator.initialize()?;

        self.validators.entry(priority).or_default().insert(name, validator);

        Ok(())
    }

    /// Get all validators in priority order.
    ///
    /// # Returns
    ///
    /// Vector of validators in priority order (highest first).
    pub fn get_all(&self) -> Vec<Arc<dyn Validator>> {
        let mut result = Vec::new();

        for (_priority, validators) in self.validators.iter().rev() {
            for validator in validators.values() {
                result.push(Arc::clone(validator));
            }
        }

        result
    }

    /// List all registered validator names.
    pub fn list(&self) -> Vec<String> {
        let mut names = std::collections::HashSet::new();
        for validators in self.validators.values() {
            names.extend(validators.keys().cloned());
        }
        names.into_iter().collect()
    }

    /// Remove a validator from the registry.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        let mut validator_to_shutdown: Option<Arc<dyn Validator>> = None;

        for validators in self.validators.values_mut() {
            if let Some(validator) = validators.shift_remove(name)
                && validator_to_shutdown.is_none()
            {
                validator_to_shutdown = Some(validator);
            }
        }

        if let Some(validator) = validator_to_shutdown {
            validator.shutdown()?;
        }

        self.validators.retain(|_, validators| !validators.is_empty());

        Ok(())
    }

    /// Shutdown all validators and clear the registry.
    pub fn shutdown_all(&mut self) -> Result<()> {
        let names = self.list();
        for name in names {
            self.remove(&name)?;
        }
        Ok(())
    }
}

impl Default for ValidatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global OCR backend registry singleton.
pub static OCR_BACKEND_REGISTRY: Lazy<Arc<RwLock<OcrBackendRegistry>>> =
    Lazy::new(|| Arc::new(RwLock::new(OcrBackendRegistry::new())));

/// Global document extractor registry singleton.
pub static DOCUMENT_EXTRACTOR_REGISTRY: Lazy<Arc<RwLock<DocumentExtractorRegistry>>> =
    Lazy::new(|| Arc::new(RwLock::new(DocumentExtractorRegistry::new())));

/// Global post-processor registry singleton.
pub static POST_PROCESSOR_REGISTRY: Lazy<Arc<RwLock<PostProcessorRegistry>>> =
    Lazy::new(|| Arc::new(RwLock::new(PostProcessorRegistry::new())));

/// Global validator registry singleton.
pub static VALIDATOR_REGISTRY: Lazy<Arc<RwLock<ValidatorRegistry>>> =
    Lazy::new(|| Arc::new(RwLock::new(ValidatorRegistry::new())));

/// Get the global OCR backend registry.
pub fn get_ocr_backend_registry() -> Arc<RwLock<OcrBackendRegistry>> {
    OCR_BACKEND_REGISTRY.clone()
}

/// Get the global document extractor registry.
pub fn get_document_extractor_registry() -> Arc<RwLock<DocumentExtractorRegistry>> {
    DOCUMENT_EXTRACTOR_REGISTRY.clone()
}

/// Get the global post-processor registry.
pub fn get_post_processor_registry() -> Arc<RwLock<PostProcessorRegistry>> {
    POST_PROCESSOR_REGISTRY.clone()
}

/// Get the global validator registry.
pub fn get_validator_registry() -> Arc<RwLock<ValidatorRegistry>> {
    VALIDATOR_REGISTRY.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{ExtractionConfig, OcrConfig};
    use crate::plugins::{Plugin, PostProcessor, ProcessingStage, Validator};
    use crate::types::ExtractionResult;
    use async_trait::async_trait;

    struct MockOcrBackend {
        name: String,
        languages: Vec<String>,
    }

    impl Plugin for MockOcrBackend {
        fn name(&self) -> &str {
            &self.name
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
        async fn process_image(&self, _: &[u8], _: &OcrConfig) -> Result<ExtractionResult> {
            Ok(ExtractionResult {
                content: "test".to_string(),
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

        fn backend_type(&self) -> crate::plugins::ocr::OcrBackendType {
            crate::plugins::ocr::OcrBackendType::Custom
        }
    }

    struct MockExtractor {
        name: String,
        mime_types: &'static [&'static str],
        priority: i32,
    }

    impl Plugin for MockExtractor {
        fn name(&self) -> &str {
            &self.name
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
        async fn extract_bytes(&self, _: &[u8], _: &str, _: &ExtractionConfig) -> Result<ExtractionResult> {
            Ok(ExtractionResult {
                content: "test".to_string(),
                mime_type: "text/plain".to_string(),
                metadata: crate::types::Metadata::default(),
                tables: vec![],
                detected_languages: None,
                chunks: None,
                images: None,
            })
        }

        fn supported_mime_types(&self) -> &[&str] {
            self.mime_types
        }

        fn priority(&self) -> i32 {
            self.priority
        }
    }

    struct MockPostProcessor {
        name: String,
        stage: ProcessingStage,
    }

    impl Plugin for MockPostProcessor {
        fn name(&self) -> &str {
            &self.name
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
        async fn process(&self, _result: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> {
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            self.stage
        }
    }

    struct MockValidator {
        name: String,
        priority: i32,
    }

    impl Plugin for MockValidator {
        fn name(&self) -> &str {
            &self.name
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
        async fn validate(&self, _: &ExtractionResult, _: &ExtractionConfig) -> Result<()> {
            Ok(())
        }

        fn priority(&self) -> i32 {
            self.priority
        }
    }

    #[test]
    fn test_ocr_backend_registry() {
        let mut registry = OcrBackendRegistry::new_empty();

        let backend = Arc::new(MockOcrBackend {
            name: "test-ocr".to_string(),
            languages: vec!["eng".to_string(), "deu".to_string()],
        });

        registry.register(backend).unwrap();

        let retrieved = registry.get("test-ocr").unwrap();
        assert_eq!(retrieved.name(), "test-ocr");

        let eng_backend = registry.get_for_language("eng").unwrap();
        assert_eq!(eng_backend.name(), "test-ocr");

        let names = registry.list();
        assert_eq!(names.len(), 1);
        assert!(names.contains(&"test-ocr".to_string()));
    }

    #[test]
    fn test_post_processor_registry() {
        let mut registry = PostProcessorRegistry::new();

        let early = Arc::new(MockPostProcessor {
            name: "early-processor".to_string(),
            stage: ProcessingStage::Early,
        });

        let middle = Arc::new(MockPostProcessor {
            name: "middle-processor".to_string(),
            stage: ProcessingStage::Middle,
        });

        registry.register(early, 100).unwrap();
        registry.register(middle, 50).unwrap();

        let early_processors = registry.get_for_stage(ProcessingStage::Early);
        assert_eq!(early_processors.len(), 1);
        assert_eq!(early_processors[0].name(), "early-processor");

        let middle_processors = registry.get_for_stage(ProcessingStage::Middle);
        assert_eq!(middle_processors.len(), 1);

        let names = registry.list();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_validator_registry() {
        let mut registry = ValidatorRegistry::new();

        let high_priority = Arc::new(MockValidator {
            name: "high-priority".to_string(),
            priority: 100,
        });

        let low_priority = Arc::new(MockValidator {
            name: "low-priority".to_string(),
            priority: 10,
        });

        registry.register(high_priority).unwrap();
        registry.register(low_priority).unwrap();

        let validators = registry.get_all();
        assert_eq!(validators.len(), 2);
        assert_eq!(validators[0].name(), "high-priority");
        assert_eq!(validators[1].name(), "low-priority");
    }

    #[test]
    fn test_document_extractor_registry_exact_match() {
        let mut registry = DocumentExtractorRegistry::new();

        let extractor = Arc::new(MockExtractor {
            name: "pdf-extractor".to_string(),
            mime_types: &["application/pdf"],
            priority: 100,
        });

        registry.register(extractor).unwrap();

        let retrieved = registry.get("application/pdf").unwrap();
        assert_eq!(retrieved.name(), "pdf-extractor");

        let names = registry.list();
        assert_eq!(names.len(), 1);
        assert!(names.contains(&"pdf-extractor".to_string()));
    }

    #[test]
    fn test_document_extractor_registry_prefix_match() {
        let mut registry = DocumentExtractorRegistry::new();

        let image_extractor = Arc::new(MockExtractor {
            name: "image-extractor".to_string(),
            mime_types: &["image/*"],
            priority: 50,
        });

        registry.register(image_extractor).unwrap();

        let retrieved = registry.get("image/png").unwrap();
        assert_eq!(retrieved.name(), "image-extractor");

        let retrieved_jpg = registry.get("image/jpeg").unwrap();
        assert_eq!(retrieved_jpg.name(), "image-extractor");
    }

    #[test]
    fn test_document_extractor_registry_priority() {
        let mut registry = DocumentExtractorRegistry::new();

        let low_priority = Arc::new(MockExtractor {
            name: "low-priority-pdf".to_string(),
            mime_types: &["application/pdf"],
            priority: 10,
        });

        let high_priority = Arc::new(MockExtractor {
            name: "high-priority-pdf".to_string(),
            mime_types: &["application/pdf"],
            priority: 100,
        });

        registry.register(low_priority).unwrap();
        registry.register(high_priority).unwrap();

        let retrieved = registry.get("application/pdf").unwrap();
        assert_eq!(retrieved.name(), "high-priority-pdf");
    }

    #[test]
    fn test_document_extractor_registry_not_found() {
        let registry = DocumentExtractorRegistry::new();

        let result = registry.get("application/unknown");
        assert!(matches!(result, Err(KreuzbergError::UnsupportedFormat(_))));
    }

    #[test]
    fn test_document_extractor_registry_remove() {
        let mut registry = DocumentExtractorRegistry::new();

        let extractor = Arc::new(MockExtractor {
            name: "test-extractor".to_string(),
            mime_types: &["text/plain"],
            priority: 50,
        });

        registry.register(extractor).unwrap();
        assert!(registry.get("text/plain").is_ok());

        registry.remove("test-extractor").unwrap();
        assert!(registry.get("text/plain").is_err());
    }

    #[test]
    fn test_document_extractor_registry_shutdown_all() {
        let mut registry = DocumentExtractorRegistry::new();

        let extractor1 = Arc::new(MockExtractor {
            name: "extractor1".to_string(),
            mime_types: &["text/plain"],
            priority: 50,
        });

        let extractor2 = Arc::new(MockExtractor {
            name: "extractor2".to_string(),
            mime_types: &["application/pdf"],
            priority: 50,
        });

        registry.register(extractor1).unwrap();
        registry.register(extractor2).unwrap();

        assert_eq!(registry.list().len(), 2);

        registry.shutdown_all().unwrap();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_ocr_backend_registry_not_found() {
        let registry = OcrBackendRegistry::new();

        let result = registry.get("nonexistent");
        assert!(matches!(result, Err(KreuzbergError::Plugin { .. })));
    }

    #[test]
    fn test_ocr_backend_registry_language_not_found() {
        let registry = OcrBackendRegistry::new();

        let result = registry.get_for_language("xyz");
        assert!(matches!(result, Err(KreuzbergError::Plugin { .. })));
    }

    #[test]
    fn test_ocr_backend_registry_remove() {
        let mut registry = OcrBackendRegistry::new();

        let backend = Arc::new(MockOcrBackend {
            name: "test-ocr".to_string(),
            languages: vec!["eng".to_string()],
        });

        registry.register(backend).unwrap();
        assert!(registry.get("test-ocr").is_ok());

        registry.remove("test-ocr").unwrap();
        assert!(registry.get("test-ocr").is_err());
    }

    #[test]
    fn test_post_processor_registry_remove() {
        let mut registry = PostProcessorRegistry::new();

        let processor = Arc::new(MockPostProcessor {
            name: "test-processor".to_string(),
            stage: ProcessingStage::Early,
        });

        registry.register(processor, 50).unwrap();
        assert_eq!(registry.get_for_stage(ProcessingStage::Early).len(), 1);

        registry.remove("test-processor").unwrap();
        assert_eq!(registry.get_for_stage(ProcessingStage::Early).len(), 0);
    }

    #[test]
    fn test_validator_registry_remove() {
        let mut registry = ValidatorRegistry::new();

        let validator = Arc::new(MockValidator {
            name: "test-validator".to_string(),
            priority: 50,
        });

        registry.register(validator).unwrap();
        assert_eq!(registry.get_all().len(), 1);

        registry.remove("test-validator").unwrap();
        assert_eq!(registry.get_all().len(), 0);
    }

    #[test]
    fn test_global_registry_access() {
        let ocr_registry = get_ocr_backend_registry();
        let _ = ocr_registry
            .read()
            .expect("Failed to acquire read lock on OCR registry in test")
            .list();

        let extractor_registry = get_document_extractor_registry();
        let _ = extractor_registry
            .read()
            .expect("Failed to acquire read lock on extractor registry in test")
            .list();

        let processor_registry = get_post_processor_registry();
        let _ = processor_registry
            .read()
            .expect("Failed to acquire read lock on processor registry in test")
            .list();

        let validator_registry = get_validator_registry();
        let _ = validator_registry
            .read()
            .expect("Failed to acquire read lock on validator registry in test")
            .list();
    }

    #[test]
    fn test_ocr_backend_registry_shutdown_all() {
        let mut registry = OcrBackendRegistry::new();
        let baseline = registry.list().len();

        let backend1 = Arc::new(MockOcrBackend {
            name: "backend1".to_string(),
            languages: vec!["eng".to_string()],
        });

        let backend2 = Arc::new(MockOcrBackend {
            name: "backend2".to_string(),
            languages: vec!["deu".to_string()],
        });

        registry.register(backend1).unwrap();
        registry.register(backend2).unwrap();

        assert_eq!(registry.list().len(), baseline + 2);

        registry.shutdown_all().unwrap();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_post_processor_registry_shutdown_all() {
        let mut registry = PostProcessorRegistry::new();

        let early = Arc::new(MockPostProcessor {
            name: "early".to_string(),
            stage: ProcessingStage::Early,
        });

        let late = Arc::new(MockPostProcessor {
            name: "late".to_string(),
            stage: ProcessingStage::Late,
        });

        registry.register(early, 100).unwrap();
        registry.register(late, 50).unwrap();

        assert_eq!(registry.list().len(), 2);

        registry.shutdown_all().unwrap();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_validator_registry_shutdown_all() {
        let mut registry = ValidatorRegistry::new();

        let validator1 = Arc::new(MockValidator {
            name: "validator1".to_string(),
            priority: 100,
        });

        let validator2 = Arc::new(MockValidator {
            name: "validator2".to_string(),
            priority: 50,
        });

        registry.register(validator1).unwrap();
        registry.register(validator2).unwrap();

        assert_eq!(registry.get_all().len(), 2);

        registry.shutdown_all().unwrap();
        assert_eq!(registry.get_all().len(), 0);
    }

    #[test]
    fn test_document_extractor_registry_multiple_mime_types() {
        let mut registry = DocumentExtractorRegistry::new();

        let multi_extractor = Arc::new(MockExtractor {
            name: "multi-extractor".to_string(),
            mime_types: &["text/plain", "text/markdown", "text/html"],
            priority: 50,
        });

        registry.register(multi_extractor).unwrap();

        assert_eq!(registry.get("text/plain").unwrap().name(), "multi-extractor");
        assert_eq!(registry.get("text/markdown").unwrap().name(), "multi-extractor");
        assert_eq!(registry.get("text/html").unwrap().name(), "multi-extractor");
    }

    #[test]
    fn test_post_processor_registry_priority_order() {
        let mut registry = PostProcessorRegistry::new();

        let low = Arc::new(MockPostProcessor {
            name: "low-priority".to_string(),
            stage: ProcessingStage::Early,
        });

        let high = Arc::new(MockPostProcessor {
            name: "high-priority".to_string(),
            stage: ProcessingStage::Early,
        });

        registry.register(low, 10).unwrap();
        registry.register(high, 100).unwrap();

        let processors = registry.get_for_stage(ProcessingStage::Early);
        assert_eq!(processors.len(), 2);
        assert_eq!(processors[0].name(), "high-priority");
        assert_eq!(processors[1].name(), "low-priority");
    }

    #[test]
    fn test_post_processor_registry_empty_stage() {
        let registry = PostProcessorRegistry::new();

        let processors = registry.get_for_stage(ProcessingStage::Late);
        assert_eq!(processors.len(), 0);
    }

    #[test]
    fn test_ocr_backend_registry_default() {
        let registry = OcrBackendRegistry::default();
        #[cfg(feature = "ocr")]
        assert!(
            !registry.list().is_empty(),
            "expected at least one default OCR backend when the 'ocr' feature is enabled"
        );
        #[cfg(not(feature = "ocr"))]
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_document_extractor_registry_default() {
        let registry = DocumentExtractorRegistry::default();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_post_processor_registry_default() {
        let registry = PostProcessorRegistry::default();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_validator_registry_default() {
        let registry = ValidatorRegistry::default();
        assert_eq!(registry.get_all().len(), 0);
    }

    #[test]
    fn test_document_extractor_registry_exact_over_prefix() {
        let mut registry = DocumentExtractorRegistry::new();

        let prefix_extractor = Arc::new(MockExtractor {
            name: "prefix-extractor".to_string(),
            mime_types: &["image/*"],
            priority: 100,
        });

        let exact_extractor = Arc::new(MockExtractor {
            name: "exact-extractor".to_string(),
            mime_types: &["image/png"],
            priority: 50,
        });

        registry.register(prefix_extractor).unwrap();
        registry.register(exact_extractor).unwrap();

        let retrieved = registry.get("image/png").unwrap();
        assert_eq!(retrieved.name(), "exact-extractor");

        let retrieved_jpg = registry.get("image/jpeg").unwrap();
        assert_eq!(retrieved_jpg.name(), "prefix-extractor");
    }

    #[test]
    fn test_ocr_backend_registry_invalid_name_empty() {
        let mut registry = OcrBackendRegistry::new();

        let backend = Arc::new(MockOcrBackend {
            name: "".to_string(),
            languages: vec!["eng".to_string()],
        });

        let result = registry.register(backend);
        assert!(matches!(result, Err(KreuzbergError::Validation { .. })));
    }

    #[test]
    fn test_ocr_backend_registry_invalid_name_whitespace() {
        let mut registry = OcrBackendRegistry::new();

        let backend = Arc::new(MockOcrBackend {
            name: "my ocr backend".to_string(),
            languages: vec!["eng".to_string()],
        });

        let result = registry.register(backend);
        assert!(matches!(result, Err(KreuzbergError::Validation { .. })));
    }

    #[test]
    fn test_document_extractor_registry_invalid_name_empty() {
        let mut registry = DocumentExtractorRegistry::new();

        let extractor = Arc::new(MockExtractor {
            name: "".to_string(),
            mime_types: &["text/plain"],
            priority: 50,
        });

        let result = registry.register(extractor);
        assert!(matches!(result, Err(KreuzbergError::Validation { .. })));
    }

    #[test]
    fn test_document_extractor_registry_invalid_name_whitespace() {
        let mut registry = DocumentExtractorRegistry::new();

        let extractor = Arc::new(MockExtractor {
            name: "my extractor".to_string(),
            mime_types: &["text/plain"],
            priority: 50,
        });

        let result = registry.register(extractor);
        assert!(matches!(result, Err(KreuzbergError::Validation { .. })));
    }

    #[test]
    fn test_post_processor_registry_invalid_name_empty() {
        let mut registry = PostProcessorRegistry::new();

        let processor = Arc::new(MockPostProcessor {
            name: "".to_string(),
            stage: ProcessingStage::Early,
        });

        let result = registry.register(processor, 50);
        assert!(matches!(result, Err(KreuzbergError::Validation { .. })));
    }

    #[test]
    fn test_post_processor_registry_invalid_name_whitespace() {
        let mut registry = PostProcessorRegistry::new();

        let processor = Arc::new(MockPostProcessor {
            name: "my processor".to_string(),
            stage: ProcessingStage::Early,
        });

        let result = registry.register(processor, 50);
        assert!(matches!(result, Err(KreuzbergError::Validation { .. })));
    }

    #[test]
    fn test_validator_registry_invalid_name_empty() {
        let mut registry = ValidatorRegistry::new();

        let validator = Arc::new(MockValidator {
            name: "".to_string(),
            priority: 50,
        });

        let result = registry.register(validator);
        assert!(matches!(result, Err(KreuzbergError::Validation { .. })));
    }

    #[test]
    fn test_validator_registry_invalid_name_whitespace() {
        let mut registry = ValidatorRegistry::new();

        let validator = Arc::new(MockValidator {
            name: "my validator".to_string(),
            priority: 50,
        });

        let result = registry.register(validator);
        assert!(matches!(result, Err(KreuzbergError::Validation { .. })));
    }
}
