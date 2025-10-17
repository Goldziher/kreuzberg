//! Plugin registration and discovery.
//!
//! This module provides registries for managing plugins of different types.
//! Each plugin type (OcrBackend, DocumentExtractor, etc.) has its own registry
//! with type-safe registration and lookup.

use crate::plugins::{DocumentExtractor, OcrBackend, PostProcessor, ProcessingStage, Validator};
use crate::{KreuzbergError, Result};
use once_cell::sync::Lazy;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};

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
    /// Create a new empty OCR backend registry.
    pub fn new() -> Self {
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

        // Initialize the backend
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
            // Shutdown the backend
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
#[allow(clippy::type_complexity)]
pub struct DocumentExtractorRegistry {
    // Maps MIME type -> (priority -> extractor_name -> extractor)
    extractors: HashMap<String, BTreeMap<i32, HashMap<String, Arc<dyn DocumentExtractor>>>>,
}

impl DocumentExtractorRegistry {
    /// Create a new empty extractor registry.
    pub fn new() -> Self {
        Self {
            extractors: HashMap::new(),
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

        // Initialize the extractor
        extractor.initialize()?;

        for mime_type in mime_types {
            self.extractors
                .entry(mime_type)
                .or_default()
                .entry(priority)
                .or_default()
                .insert(name.clone(), Arc::clone(&extractor));
        }

        // Invalidate thread-local caches since registry changed
        crate::core::extractor::invalidate_extractor_cache();

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
        // Try exact match first
        if let Some(priority_map) = self.extractors.get(mime_type)
            // Get highest priority (last in BTreeMap)
            && let Some((_priority, extractors)) = priority_map.iter().next_back()
            && let Some((_name, extractor)) = extractors.iter().next()
        {
            return Ok(Arc::clone(extractor));
        }

        // Try prefix match (e.g., "image/*")
        let mut best_match: Option<(i32, Arc<dyn DocumentExtractor>)> = None;

        for (registered_mime, priority_map) in &self.extractors {
            if registered_mime.ends_with("/*") {
                let prefix = &registered_mime[..registered_mime.len() - 1];
                if mime_type.starts_with(prefix)
                    && let Some((_priority, extractors)) = priority_map.iter().next_back()
                    && let Some((_, extractor)) = extractors.iter().next()
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
        let mut names = std::collections::HashSet::new();
        for priority_map in self.extractors.values() {
            for extractors in priority_map.values() {
                names.extend(extractors.keys().cloned());
            }
        }
        names.into_iter().collect()
    }

    /// Remove an extractor from the registry.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        let mut extractor_to_shutdown: Option<Arc<dyn DocumentExtractor>> = None;

        for priority_map in self.extractors.values_mut() {
            for extractors in priority_map.values_mut() {
                if let Some(extractor) = extractors.remove(name) {
                    // Store first instance to shutdown (all are clones of same Arc)
                    if extractor_to_shutdown.is_none() {
                        extractor_to_shutdown = Some(extractor);
                    }
                }
            }
        }

        // Shutdown the extractor once
        if let Some(extractor) = extractor_to_shutdown {
            extractor.shutdown()?;

            // Invalidate thread-local caches since registry changed
            crate::core::extractor::invalidate_extractor_cache();
        }

        // Clean up empty maps
        self.extractors.retain(|_, priority_map| {
            priority_map.retain(|_, extractors| !extractors.is_empty());
            !priority_map.is_empty()
        });

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
#[allow(clippy::type_complexity)]
pub struct PostProcessorRegistry {
    // Maps stage -> priority -> processor_name -> processor
    processors: HashMap<ProcessingStage, BTreeMap<i32, HashMap<String, Arc<dyn PostProcessor>>>>,
}

impl PostProcessorRegistry {
    /// Create a new empty post-processor registry.
    pub fn new() -> Self {
        Self {
            processors: HashMap::new(),
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

        processor.initialize()?;

        self.processors
            .entry(stage)
            .or_default()
            .entry(priority)
            .or_default()
            .insert(name, processor);

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
            // Iterate in reverse order (highest priority first)
            for (_priority, processors) in priority_map.iter().rev() {
                for processor in processors.values() {
                    result.push(Arc::clone(processor));
                }
            }
        }

        result
    }

    /// List all registered processor names.
    pub fn list(&self) -> Vec<String> {
        let mut names = std::collections::HashSet::new();
        for priority_map in self.processors.values() {
            for processors in priority_map.values() {
                names.extend(processors.keys().cloned());
            }
        }
        names.into_iter().collect()
    }

    /// Remove a processor from the registry.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        let mut processor_to_shutdown: Option<Arc<dyn PostProcessor>> = None;

        for priority_map in self.processors.values_mut() {
            for processors in priority_map.values_mut() {
                if let Some(processor) = processors.remove(name) {
                    // Store first instance to shutdown
                    if processor_to_shutdown.is_none() {
                        processor_to_shutdown = Some(processor);
                    }
                }
            }
        }

        // Shutdown the processor once
        if let Some(processor) = processor_to_shutdown {
            processor.shutdown()?;
        }

        // Clean up empty maps
        self.processors.retain(|_, priority_map| {
            priority_map.retain(|_, processors| !processors.is_empty());
            !priority_map.is_empty()
        });

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
    // Maps priority -> validator_name -> validator
    validators: BTreeMap<i32, HashMap<String, Arc<dyn Validator>>>,
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

        // Iterate in reverse order (highest priority first)
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
            if let Some(validator) = validators.remove(name) {
                // Store first instance to shutdown
                if validator_to_shutdown.is_none() {
                    validator_to_shutdown = Some(validator);
                }
            }
        }

        // Shutdown the validator once
        if let Some(validator) = validator_to_shutdown {
            validator.shutdown()?;
        }

        // Clean up empty maps
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
    use std::collections::HashMap;

    // Mock implementations for testing
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
                metadata: HashMap::new(),
                tables: vec![],
                detected_languages: None,
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
                metadata: HashMap::new(),
                tables: vec![],
                detected_languages: None,
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
        let mut registry = OcrBackendRegistry::new();

        let backend = Arc::new(MockOcrBackend {
            name: "test-ocr".to_string(),
            languages: vec!["eng".to_string(), "deu".to_string()],
        });

        registry.register(backend).unwrap();

        // Test get by name
        let retrieved = registry.get("test-ocr").unwrap();
        assert_eq!(retrieved.name(), "test-ocr");

        // Test get by language
        let eng_backend = registry.get_for_language("eng").unwrap();
        assert_eq!(eng_backend.name(), "test-ocr");

        // Test list
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

        // Test get by stage
        let early_processors = registry.get_for_stage(ProcessingStage::Early);
        assert_eq!(early_processors.len(), 1);
        assert_eq!(early_processors[0].name(), "early-processor");

        let middle_processors = registry.get_for_stage(ProcessingStage::Middle);
        assert_eq!(middle_processors.len(), 1);

        // Test list
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

        // Test get_all returns in priority order
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

        // Test exact MIME type match
        let retrieved = registry.get("application/pdf").unwrap();
        assert_eq!(retrieved.name(), "pdf-extractor");

        // Test list
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

        // Test prefix match
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

        // Should return highest priority extractor
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
        // Test that global registries can be accessed without panicking
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

        assert_eq!(registry.list().len(), 2);

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

        // All MIME types should work
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

        // Register in reverse order
        registry.register(low, 10).unwrap();
        registry.register(high, 100).unwrap();

        // Should return in priority order (highest first)
        let processors = registry.get_for_stage(ProcessingStage::Early);
        assert_eq!(processors.len(), 2);
        assert_eq!(processors[0].name(), "high-priority");
        assert_eq!(processors[1].name(), "low-priority");
    }

    #[test]
    fn test_post_processor_registry_empty_stage() {
        let registry = PostProcessorRegistry::new();

        // No processors for this stage
        let processors = registry.get_for_stage(ProcessingStage::Late);
        assert_eq!(processors.len(), 0);
    }

    #[test]
    fn test_ocr_backend_registry_default() {
        let registry = OcrBackendRegistry::default();
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

        // Register prefix matcher
        let prefix_extractor = Arc::new(MockExtractor {
            name: "prefix-extractor".to_string(),
            mime_types: &["image/*"],
            priority: 100,
        });

        // Register exact matcher with lower priority
        let exact_extractor = Arc::new(MockExtractor {
            name: "exact-extractor".to_string(),
            mime_types: &["image/png"],
            priority: 50,
        });

        registry.register(prefix_extractor).unwrap();
        registry.register(exact_extractor).unwrap();

        // Exact match should be preferred over prefix match
        let retrieved = registry.get("image/png").unwrap();
        assert_eq!(retrieved.name(), "exact-extractor");

        // Other image types should use prefix match
        let retrieved_jpg = registry.get("image/jpeg").unwrap();
        assert_eq!(retrieved_jpg.name(), "prefix-extractor");
    }
}
