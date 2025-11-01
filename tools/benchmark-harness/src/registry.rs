//! Adapter registry for managing framework adapters
//!
//! The registry provides a central place to register and retrieve adapters
//! for different extraction frameworks.

use crate::Error;
use crate::adapter::FrameworkAdapter;
use ahash::AHashMap;
use std::sync::Arc;

/// Registry for framework adapters
///
/// Stores adapters by name and provides lookup and iteration capabilities.
pub struct AdapterRegistry {
    adapters: AHashMap<String, Arc<dyn FrameworkAdapter>>,
}

impl AdapterRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            adapters: AHashMap::new(),
        }
    }

    /// Register an adapter
    ///
    /// # Arguments
    /// * `adapter` - The adapter to register
    ///
    /// # Returns
    /// * `Ok(())` - Adapter registered successfully
    /// * `Err(Error::Config)` - Adapter with same name already exists
    pub fn register(&mut self, adapter: Arc<dyn FrameworkAdapter>) -> crate::Result<()> {
        let name = adapter.name().to_string();

        if self.adapters.contains_key(&name) {
            return Err(Error::Config(format!("Adapter '{}' is already registered", name)));
        }

        self.adapters.insert(name, adapter);
        Ok(())
    }

    /// Get an adapter by name
    ///
    /// # Arguments
    /// * `name` - The adapter name
    ///
    /// # Returns
    /// * `Some(Arc<dyn FrameworkAdapter>)` - Adapter found
    /// * `None` - No adapter with that name
    pub fn get(&self, name: &str) -> Option<Arc<dyn FrameworkAdapter>> {
        self.adapters.get(name).cloned()
    }

    /// Check if an adapter is registered
    pub fn contains(&self, name: &str) -> bool {
        self.adapters.contains_key(name)
    }

    /// Get all registered adapter names
    pub fn adapter_names(&self) -> Vec<String> {
        self.adapters.keys().cloned().collect()
    }

    /// Get all registered adapters
    pub fn adapters(&self) -> Vec<Arc<dyn FrameworkAdapter>> {
        self.adapters.values().cloned().collect()
    }

    /// Get the number of registered adapters
    pub fn len(&self) -> usize {
        self.adapters.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }

    /// Remove an adapter by name
    ///
    /// # Returns
    /// * `Some(Arc<dyn FrameworkAdapter>)` - The removed adapter
    /// * `None` - No adapter with that name
    pub fn remove(&mut self, name: &str) -> Option<Arc<dyn FrameworkAdapter>> {
        self.adapters.remove(name)
    }

    /// Clear all adapters
    pub fn clear(&mut self) {
        self.adapters.clear();
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::NativeAdapter;

    #[test]
    fn test_registry_creation() {
        let registry = AdapterRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_register_adapter() {
        let mut registry = AdapterRegistry::new();
        let adapter = Arc::new(NativeAdapter::new()) as Arc<dyn FrameworkAdapter>;

        registry.register(adapter).unwrap();
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("kreuzberg-native"));
    }

    #[test]
    fn test_duplicate_registration_fails() {
        let mut registry = AdapterRegistry::new();
        let adapter1 = Arc::new(NativeAdapter::new()) as Arc<dyn FrameworkAdapter>;
        let adapter2 = Arc::new(NativeAdapter::new()) as Arc<dyn FrameworkAdapter>;

        registry.register(adapter1).unwrap();
        let result = registry.register(adapter2);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_adapter() {
        let mut registry = AdapterRegistry::new();
        let adapter = Arc::new(NativeAdapter::new()) as Arc<dyn FrameworkAdapter>;

        registry.register(adapter).unwrap();

        let retrieved = registry.get("kreuzberg-native");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "kreuzberg-native");
    }

    #[test]
    fn test_adapter_names() {
        let mut registry = AdapterRegistry::new();
        let adapter = Arc::new(NativeAdapter::new()) as Arc<dyn FrameworkAdapter>;

        registry.register(adapter).unwrap();

        let names = registry.adapter_names();
        assert_eq!(names.len(), 1);
        assert!(names.contains(&"kreuzberg-native".to_string()));
    }

    #[test]
    fn test_remove_adapter() {
        let mut registry = AdapterRegistry::new();
        let adapter = Arc::new(NativeAdapter::new()) as Arc<dyn FrameworkAdapter>;

        registry.register(adapter).unwrap();
        assert_eq!(registry.len(), 1);

        let removed = registry.remove("kreuzberg-native");
        assert!(removed.is_some());
        assert_eq!(registry.len(), 0);
    }
}
