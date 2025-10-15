//! Extractor registry for MIME type routing.
//!
//! This module provides a thread-safe registry that maps MIME types to extractors.
//! It supports:
//! - Exact MIME type matching
//! - Prefix matching (e.g., "image/*")
//! - Priority-based selection for overlapping MIME types
//! - Thread-safe registration and lookup

use crate::{KreuzbergError, Result};
use once_cell::sync::Lazy;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};

/// Default priority for extractors when not specified.
pub const DEFAULT_PRIORITY: i32 = 50;

/// Global extractor registry singleton.
///
/// This is initialized lazily and provides thread-safe access to the extractor registry.
pub static REGISTRY: Lazy<Arc<RwLock<ExtractorRegistry>>> =
    Lazy::new(|| Arc::new(RwLock::new(ExtractorRegistry::new())));

/// Registry for document extractors.
///
/// The registry maps MIME types to extractors and manages priority-based selection
/// when multiple extractors support the same MIME type.
///
/// ## Priority System
///
/// - Higher priority values take precedence (e.g., 100 > 50)
/// - Default priority is 50
/// - Built-in extractors use priorities 0-100
/// - Custom extractors can use any priority value
#[derive(Debug)]
pub struct ExtractorRegistry {
    // TODO: Replace String with actual extractor trait objects when plugin system is ready
    // Maps MIME type -> (priority -> extractor_name)
    extractors: HashMap<String, BTreeMap<i32, String>>,
}

impl ExtractorRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            extractors: HashMap::new(),
        }
    }

    /// Register an extractor for one or more MIME types with default priority.
    ///
    /// # Arguments
    ///
    /// * `mime_types` - Slice of MIME types this extractor supports
    /// * `extractor_name` - Name/identifier of the extractor
    ///
    /// # Example
    ///
    /// ```rust
    /// use kreuzberg::core::registry::ExtractorRegistry;
    ///
    /// let mut registry = ExtractorRegistry::new();
    /// registry.register(&["application/pdf"], "PDFExtractor");
    /// ```
    pub fn register(&mut self, mime_types: &[&str], extractor_name: &str) {
        self.register_with_priority(mime_types, extractor_name, DEFAULT_PRIORITY);
    }

    /// Register an extractor for one or more MIME types with a specific priority.
    ///
    /// # Arguments
    ///
    /// * `mime_types` - Slice of MIME types this extractor supports
    /// * `extractor_name` - Name/identifier of the extractor
    /// * `priority` - Priority value (higher = higher priority)
    ///
    /// # Example
    ///
    /// ```rust
    /// use kreuzberg::core::registry::ExtractorRegistry;
    ///
    /// let mut registry = ExtractorRegistry::new();
    /// // Register a high-priority PDF extractor
    /// registry.register_with_priority(&["application/pdf"], "CustomPDFExtractor", 100);
    /// // Register a fallback PDF extractor
    /// registry.register_with_priority(&["application/pdf"], "FallbackPDFExtractor", 10);
    /// ```
    pub fn register_with_priority(&mut self, mime_types: &[&str], extractor_name: &str, priority: i32) {
        for mime_type in mime_types {
            self.extractors
                .entry(mime_type.to_string())
                .or_default()
                .insert(priority, extractor_name.to_string());
        }
    }

    /// Get an extractor for a given MIME type.
    ///
    /// Tries exact match first, then prefix matching.
    /// When multiple extractors are registered for the same MIME type,
    /// returns the one with the highest priority.
    ///
    /// # Arguments
    ///
    /// * `mime_type` - The MIME type to look up
    ///
    /// # Returns
    ///
    /// The extractor name with the highest priority.
    ///
    /// # Errors
    ///
    /// Returns `KreuzbergError::UnsupportedFormat` if no extractor found.
    pub fn get(&self, mime_type: &str) -> Result<String> {
        // Try exact match first
        if let Some(priority_map) = self.extractors.get(mime_type) {
            // Get the highest priority extractor (BTreeMap iterates in ascending order)
            if let Some((_priority, extractor)) = priority_map.iter().next_back() {
                return Ok(extractor.clone());
            }
        }

        // Try prefix match (e.g., "image/jpeg" matches "image/*")
        let mut best_match: Option<(i32, String)> = None;

        for (registered_mime, priority_map) in &self.extractors {
            if registered_mime.ends_with("/*") {
                let prefix = &registered_mime[..registered_mime.len() - 1];
                if mime_type.starts_with(prefix) {
                    // Get the highest priority extractor for this prefix
                    if let Some((priority, extractor)) = priority_map.iter().next_back() {
                        match &best_match {
                            None => {
                                best_match = Some((*priority, extractor.clone()));
                            }
                            Some((current_priority, _)) => {
                                if priority > current_priority {
                                    best_match = Some((*priority, extractor.clone()));
                                }
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

    /// Remove an extractor from the registry.
    ///
    /// Removes all registrations of the specified extractor across all MIME types
    /// and priorities.
    ///
    /// # Arguments
    ///
    /// * `extractor_name` - Name of the extractor to remove
    pub fn remove(&mut self, extractor_name: &str) {
        // Remove the extractor from all priority maps
        for priority_map in self.extractors.values_mut() {
            priority_map.retain(|_, name| name != extractor_name);
        }

        // Clean up empty MIME type entries
        self.extractors.retain(|_, priority_map| !priority_map.is_empty());
    }

    /// Remove a specific extractor registration for a MIME type and priority.
    ///
    /// # Arguments
    ///
    /// * `mime_type` - The MIME type to remove the extractor from
    /// * `extractor_name` - Name of the extractor to remove
    /// * `priority` - Priority value to remove (if None, removes all priorities)
    pub fn remove_registration(&mut self, mime_type: &str, extractor_name: &str, priority: Option<i32>) {
        if let Some(priority_map) = self.extractors.get_mut(mime_type) {
            if let Some(p) = priority {
                // Remove specific priority
                if priority_map.get(&p) == Some(&extractor_name.to_string()) {
                    priority_map.remove(&p);
                }
            } else {
                // Remove all priorities for this extractor
                priority_map.retain(|_, name| name != extractor_name);
            }

            // Clean up if empty
            if priority_map.is_empty() {
                self.extractors.remove(mime_type);
            }
        }
    }

    /// Check if a MIME type is supported.
    pub fn supports(&self, mime_type: &str) -> bool {
        self.get(mime_type).is_ok()
    }

    /// Get all registered MIME types.
    pub fn mime_types(&self) -> Vec<String> {
        self.extractors.keys().cloned().collect()
    }
}

impl Default for ExtractorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the global registry instance.
///
/// # Example
///
/// ```rust
/// use kreuzberg::core::registry::get_registry;
///
/// let registry = get_registry();
/// let registry_read = registry.read().unwrap();
/// let supported_types = registry_read.mime_types();
/// ```
pub fn get_registry() -> Arc<RwLock<ExtractorRegistry>> {
    REGISTRY.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get() {
        let mut registry = ExtractorRegistry::new();
        registry.register(&["application/pdf"], "PDFExtractor");

        assert_eq!(registry.get("application/pdf").unwrap(), "PDFExtractor");
    }

    #[test]
    fn test_priority_based_selection() {
        let mut registry = ExtractorRegistry::new();

        // Register multiple extractors for the same MIME type with different priorities
        registry.register_with_priority(&["application/pdf"], "LowPriorityExtractor", 10);
        registry.register_with_priority(&["application/pdf"], "HighPriorityExtractor", 100);
        registry.register_with_priority(&["application/pdf"], "MediumPriorityExtractor", 50);

        // Should return the highest priority extractor
        assert_eq!(registry.get("application/pdf").unwrap(), "HighPriorityExtractor");
    }

    #[test]
    fn test_default_priority() {
        let mut registry = ExtractorRegistry::new();

        // Register with explicit priority
        registry.register_with_priority(&["application/pdf"], "LowPriorityExtractor", 10);
        // Register with default priority (should be 50)
        registry.register(&["application/pdf"], "DefaultPriorityExtractor");

        // Default priority (50) should win over low priority (10)
        assert_eq!(registry.get("application/pdf").unwrap(), "DefaultPriorityExtractor");
    }

    #[test]
    fn test_prefix_matching() {
        let mut registry = ExtractorRegistry::new();
        registry.register(&["image/*"], "ImageExtractor");

        assert_eq!(registry.get("image/jpeg").unwrap(), "ImageExtractor");
        assert_eq!(registry.get("image/png").unwrap(), "ImageExtractor");
    }

    #[test]
    fn test_prefix_matching_with_priority() {
        let mut registry = ExtractorRegistry::new();

        // Register prefix matchers with different priorities
        registry.register_with_priority(&["image/*"], "DefaultImageExtractor", 50);
        registry.register_with_priority(&["image/*"], "PremiumImageExtractor", 100);

        // Should return highest priority prefix matcher
        assert_eq!(registry.get("image/jpeg").unwrap(), "PremiumImageExtractor");
    }

    #[test]
    fn test_exact_match_precedence_over_prefix() {
        let mut registry = ExtractorRegistry::new();

        // Register prefix matcher
        registry.register_with_priority(&["image/*"], "GenericImageExtractor", 100);
        // Register exact match with lower priority
        registry.register_with_priority(&["image/jpeg"], "JPEGExtractor", 50);

        // Exact match should take precedence even with lower priority
        assert_eq!(registry.get("image/jpeg").unwrap(), "JPEGExtractor");
    }

    #[test]
    fn test_unsupported_mime_type() {
        let registry = ExtractorRegistry::new();
        assert!(registry.get("application/unknown").is_err());
    }

    #[test]
    fn test_remove() {
        let mut registry = ExtractorRegistry::new();
        registry.register(&["application/pdf"], "PDFExtractor");
        registry.remove("PDFExtractor");
        assert!(registry.get("application/pdf").is_err());
    }

    #[test]
    fn test_remove_with_multiple_priorities() {
        let mut registry = ExtractorRegistry::new();

        // Register same extractor with multiple priorities
        registry.register_with_priority(&["application/pdf"], "PDFExtractor", 10);
        registry.register_with_priority(&["application/pdf"], "PDFExtractor", 50);
        registry.register_with_priority(&["text/plain"], "PDFExtractor", 100);

        // Remove should clear all registrations
        registry.remove("PDFExtractor");

        assert!(registry.get("application/pdf").is_err());
        assert!(registry.get("text/plain").is_err());
    }

    #[test]
    fn test_remove_registration_specific() {
        let mut registry = ExtractorRegistry::new();

        // Register multiple extractors
        registry.register_with_priority(&["application/pdf"], "ExtractorA", 50);
        registry.register_with_priority(&["application/pdf"], "ExtractorB", 100);

        // Remove specific registration
        registry.remove_registration("application/pdf", "ExtractorB", Some(100));

        // ExtractorA should still be available
        assert_eq!(registry.get("application/pdf").unwrap(), "ExtractorA");
    }

    #[test]
    fn test_remove_registration_all_priorities() {
        let mut registry = ExtractorRegistry::new();

        // Register same extractor with multiple priorities
        registry.register_with_priority(&["application/pdf"], "ExtractorA", 10);
        registry.register_with_priority(&["application/pdf"], "ExtractorA", 50);
        registry.register_with_priority(&["application/pdf"], "ExtractorB", 100);

        // Remove all priorities for ExtractorA
        registry.remove_registration("application/pdf", "ExtractorA", None);

        // ExtractorB should still be available
        assert_eq!(registry.get("application/pdf").unwrap(), "ExtractorB");
    }

    #[test]
    fn test_supports() {
        let mut registry = ExtractorRegistry::new();
        registry.register(&["text/plain"], "TextExtractor");
        assert!(registry.supports("text/plain"));
        assert!(!registry.supports("text/html"));
    }

    #[test]
    fn test_mime_types() {
        let mut registry = ExtractorRegistry::new();
        registry.register(&["application/pdf", "text/plain"], "Extractor1");
        registry.register(&["image/png"], "Extractor2");

        let mime_types = registry.mime_types();
        assert_eq!(mime_types.len(), 3);
        assert!(mime_types.contains(&"application/pdf".to_string()));
        assert!(mime_types.contains(&"text/plain".to_string()));
        assert!(mime_types.contains(&"image/png".to_string()));
    }
}
