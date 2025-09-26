//! Bounded resource cache for memory-efficient processing

use std::collections::HashMap;
use std::collections::VecDeque;

/// LRU cache for image and resource data with size limits
pub struct ResourceCache {
    /// Cached resources
    data: HashMap<String, Vec<u8>>,
    /// Access order for LRU eviction
    access_order: VecDeque<String>,
    /// Current total size in bytes
    current_size: usize,
    /// Maximum size in bytes
    max_size: usize,
    /// Maximum number of items
    max_items: usize,
}

impl ResourceCache {
    pub fn new(max_size_mb: usize, max_items: usize) -> Self {
        Self {
            data: HashMap::new(),
            access_order: VecDeque::new(),
            current_size: 0,
            max_size: max_size_mb * 1024 * 1024,
            max_items,
        }
    }

    /// Get resource data, marking it as recently accessed
    pub fn get(&mut self, key: &str) -> Option<&Vec<u8>> {
        if self.data.contains_key(key) {
            self.access_order.retain(|k| k != key);
            self.access_order.push_front(key.to_string());
            self.data.get(key)
        } else {
            None
        }
    }

    /// Insert resource data, potentially evicting old items
    pub fn insert(&mut self, key: String, value: Vec<u8>) {
        let value_size = value.len();

        if let Some(old_value) = self.data.remove(&key) {
            self.current_size -= old_value.len();
            self.access_order.retain(|k| k != &key);
        }

        while (self.current_size + value_size > self.max_size || self.data.len() >= self.max_items)
            && !self.access_order.is_empty()
        {
            self.evict_lru();
        }

        self.data.insert(key.clone(), value);
        self.access_order.push_front(key);
        self.current_size += value_size;
    }

    fn evict_lru(&mut self) {
        if let Some(lru_key) = self.access_order.pop_back() {
            if let Some(value) = self.data.remove(&lru_key) {
                self.current_size -= value.len();
            }
        }
    }
}
