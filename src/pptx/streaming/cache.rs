use std::collections::HashMap;
use std::collections::VecDeque;

pub struct ResourceCache {
    data: HashMap<String, Vec<u8>>,
    access_order: VecDeque<String>,
    current_size: usize,
    max_size: usize,
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

    pub fn get(&mut self, key: &str) -> Option<&Vec<u8>> {
        if self.data.contains_key(key) {
            self.access_order.retain(|k| k != key);
            self.access_order.push_front(key.to_string());
            self.data.get(key)
        } else {
            None
        }
    }

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
        if let Some(lru_key) = self.access_order.pop_back()
            && let Some(value) = self.data.remove(&lru_key)
        {
            self.current_size -= value.len();
        }
    }
}
