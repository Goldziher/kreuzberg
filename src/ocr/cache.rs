//! OCR cache integration
//!
//! Integrates with the existing Rust cache layer to provide fast caching of OCR results.

use std::fs;
use std::path::PathBuf;

use pyo3::prelude::*;

use super::error::OCRError;
use super::types::ExtractionResultDTO;
use super::utils::compute_hash;

/// OCR Cache manager
pub struct OCRCache {
    cache_dir: PathBuf,
}

impl OCRCache {
    /// Create a new OCR cache with the specified directory
    pub fn new(cache_dir: Option<PathBuf>) -> Result<Self, OCRError> {
        let cache_dir = cache_dir.unwrap_or_else(|| {
            let mut path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            path.push(".kreuzberg");
            path.push("ocr");
            path
        });

        // Create cache directory if it doesn't exist
        fs::create_dir_all(&cache_dir)
            .map_err(|e| OCRError::CacheError(format!("Failed to create cache directory: {}", e)))?;

        Ok(Self { cache_dir })
    }

    /// Get cache path for a given cache key
    fn get_cache_path(&self, cache_key: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.msgpack", cache_key))
    }

    /// Get cached OCR result
    ///
    /// Returns None if not found or if cache is invalid/expired
    pub fn get_cached_result(
        &self,
        image_hash: &str,
        backend: &str,
        config: &str,
    ) -> Result<Option<ExtractionResultDTO>, OCRError> {
        // Generate cache key using same logic as Python
        let cache_key = self.generate_cache_key(image_hash, backend, config);
        let cache_path = self.get_cache_path(&cache_key);

        if !cache_path.exists() {
            return Ok(None);
        }

        // Read cached data
        let cached_bytes =
            fs::read(&cache_path).map_err(|e| OCRError::CacheError(format!("Failed to read cache file: {}", e)))?;

        // Deserialize from msgpack
        let result: ExtractionResultDTO = rmp_serde::from_slice(&cached_bytes)
            .map_err(|e| OCRError::CacheError(format!("Failed to deserialize cache: {}", e)))?;

        Ok(Some(result))
    }

    /// Set cached OCR result
    pub fn set_cached_result(
        &self,
        image_hash: &str,
        backend: &str,
        config: &str,
        result: &ExtractionResultDTO,
    ) -> Result<(), OCRError> {
        // Generate cache key
        let cache_key = self.generate_cache_key(image_hash, backend, config);
        let cache_path = self.get_cache_path(&cache_key);

        // Serialize to msgpack
        let serialized = rmp_serde::to_vec(result)
            .map_err(|e| OCRError::CacheError(format!("Failed to serialize result: {}", e)))?;

        // Write to cache file
        fs::write(&cache_path, serialized)
            .map_err(|e| OCRError::CacheError(format!("Failed to write cache file: {}", e)))?;

        Ok(())
    }

    /// Generate cache key from components
    fn generate_cache_key(&self, image_hash: &str, backend: &str, config: &str) -> String {
        // Create a deterministic string for hashing
        let cache_string = format!(
            "image_hash={}&ocr_backend={}&ocr_config={}",
            image_hash, backend, config
        );

        compute_hash(&cache_string)
    }

    /// Clear all cached OCR results
    pub fn clear(&self) -> Result<(), OCRError> {
        if !self.cache_dir.exists() {
            return Ok(());
        }

        // Remove all .msgpack files
        let entries = fs::read_dir(&self.cache_dir)
            .map_err(|e| OCRError::CacheError(format!("Failed to read cache directory: {}", e)))?;

        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension()
                && ext == "msgpack"
            {
                let _ = fs::remove_file(entry.path());
            }
        }

        Ok(())
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> Result<OCRCacheStats, OCRError> {
        if !self.cache_dir.exists() {
            return Ok(OCRCacheStats::default());
        }

        let entries = fs::read_dir(&self.cache_dir)
            .map_err(|e| OCRError::CacheError(format!("Failed to read cache directory: {}", e)))?;

        let mut total_files = 0;
        let mut total_size_bytes = 0u64;

        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension()
                && ext == "msgpack"
            {
                total_files += 1;
                if let Ok(metadata) = entry.metadata() {
                    total_size_bytes += metadata.len();
                }
            }
        }

        Ok(OCRCacheStats {
            total_files,
            total_size_mb: total_size_bytes as f64 / 1024.0 / 1024.0,
        })
    }
}

/// OCR Cache statistics
#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct OCRCacheStats {
    #[pyo3(get)]
    pub total_files: usize,
    #[pyo3(get)]
    pub total_size_mb: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn test_cache_get_set() {
        let temp_dir = tempdir().unwrap();
        let cache = OCRCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let result = ExtractionResultDTO {
            content: "Test OCR result".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: Vec::new(),
        };

        // Set cache
        cache.set_cached_result("abc123", "tesseract", "eng", &result).unwrap();

        // Get cache
        let cached = cache.get_cached_result("abc123", "tesseract", "eng").unwrap();

        assert!(cached.is_some());
        assert_eq!(cached.unwrap().content, "Test OCR result");
    }

    #[test]
    fn test_cache_miss() {
        let temp_dir = tempdir().unwrap();
        let cache = OCRCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let cached = cache.get_cached_result("nonexistent", "tesseract", "eng").unwrap();

        assert!(cached.is_none());
    }

    #[test]
    fn test_cache_clear() {
        let temp_dir = tempdir().unwrap();
        let cache = OCRCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let result = ExtractionResultDTO {
            content: "Test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: Vec::new(),
        };

        cache.set_cached_result("test", "tesseract", "eng", &result).unwrap();

        cache.clear().unwrap();

        let cached = cache.get_cached_result("test", "tesseract", "eng").unwrap();
        assert!(cached.is_none());
    }

    #[test]
    fn test_cache_stats() {
        let temp_dir = tempdir().unwrap();
        let cache = OCRCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let stats = cache.get_stats().unwrap();
        assert_eq!(stats.total_files, 0);

        let result = ExtractionResultDTO {
            content: "Test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: Vec::new(),
        };

        cache.set_cached_result("test", "tesseract", "eng", &result).unwrap();

        let stats = cache.get_stats().unwrap();
        assert_eq!(stats.total_files, 1);
        assert!(stats.total_size_mb > 0.0);
    }

    #[test]
    fn test_cache_key_generation_deterministic() {
        let temp_dir = tempdir().unwrap();
        let cache = OCRCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let key1 = cache.generate_cache_key("abc123", "tesseract", "eng");
        let key2 = cache.generate_cache_key("abc123", "tesseract", "eng");

        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 16);
    }

    #[test]
    fn test_cache_key_generation_different_inputs() {
        let temp_dir = tempdir().unwrap();
        let cache = OCRCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let key1 = cache.generate_cache_key("abc123", "tesseract", "eng");
        let key2 = cache.generate_cache_key("def456", "tesseract", "eng");
        let key3 = cache.generate_cache_key("abc123", "easyocr", "eng");
        let key4 = cache.generate_cache_key("abc123", "tesseract", "fra");

        assert_ne!(key1, key2);
        assert_ne!(key1, key3);
        assert_ne!(key1, key4);
    }

    #[test]
    fn test_cache_multiple_entries() {
        let temp_dir = tempdir().unwrap();
        let cache = OCRCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let result1 = ExtractionResultDTO {
            content: "First".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: Vec::new(),
        };

        let result2 = ExtractionResultDTO {
            content: "Second".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: Vec::new(),
        };

        cache.set_cached_result("hash1", "tesseract", "eng", &result1).unwrap();
        cache.set_cached_result("hash2", "tesseract", "eng", &result2).unwrap();

        let stats = cache.get_stats().unwrap();
        assert_eq!(stats.total_files, 2);

        let retrieved1 = cache.get_cached_result("hash1", "tesseract", "eng").unwrap();
        let retrieved2 = cache.get_cached_result("hash2", "tesseract", "eng").unwrap();

        assert_eq!(retrieved1.unwrap().content, "First");
        assert_eq!(retrieved2.unwrap().content, "Second");
    }

    #[test]
    fn test_cache_overwrite() {
        let temp_dir = tempdir().unwrap();
        let cache = OCRCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let result1 = ExtractionResultDTO {
            content: "Original".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: Vec::new(),
        };

        let result2 = ExtractionResultDTO {
            content: "Updated".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: Vec::new(),
        };

        cache.set_cached_result("test", "tesseract", "eng", &result1).unwrap();
        cache.set_cached_result("test", "tesseract", "eng", &result2).unwrap();

        let retrieved = cache.get_cached_result("test", "tesseract", "eng").unwrap();
        assert_eq!(retrieved.unwrap().content, "Updated");

        let stats = cache.get_stats().unwrap();
        assert_eq!(stats.total_files, 1);
    }

    #[test]
    fn test_cache_with_tables() {
        let temp_dir = tempdir().unwrap();
        let cache = OCRCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

        use crate::ocr::types::TableDTO;

        let table = TableDTO {
            cells: vec![vec!["A".to_string(), "B".to_string()]],
            markdown: "| A | B |".to_string(),
            page_number: 0,
        };

        let result = ExtractionResultDTO {
            content: "Content with table".to_string(),
            mime_type: "text/markdown".to_string(),
            metadata: HashMap::new(),
            tables: vec![table],
        };

        cache.set_cached_result("test", "tesseract", "eng", &result).unwrap();

        let retrieved = cache.get_cached_result("test", "tesseract", "eng").unwrap().unwrap();
        assert_eq!(retrieved.tables.len(), 1);
        assert_eq!(retrieved.tables[0].cells[0][0], "A");
    }

    #[test]
    fn test_cache_with_metadata() {
        let temp_dir = tempdir().unwrap();
        let cache = OCRCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let mut metadata = HashMap::new();
        metadata.insert("language".to_string(), "eng".to_string());
        metadata.insert("confidence".to_string(), "95.5".to_string());

        let result = ExtractionResultDTO {
            content: "Content".to_string(),
            mime_type: "text/plain".to_string(),
            metadata,
            tables: Vec::new(),
        };

        cache.set_cached_result("test", "tesseract", "eng", &result).unwrap();

        let retrieved = cache.get_cached_result("test", "tesseract", "eng").unwrap().unwrap();
        assert_eq!(retrieved.metadata.get("language").unwrap(), "eng");
        assert_eq!(retrieved.metadata.get("confidence").unwrap(), "95.5");
    }

    #[test]
    fn test_cache_clear_selective() {
        let temp_dir = tempdir().unwrap();
        let cache = OCRCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let result = ExtractionResultDTO {
            content: "Test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: Vec::new(),
        };

        cache.set_cached_result("test1", "tesseract", "eng", &result).unwrap();
        cache.set_cached_result("test2", "tesseract", "eng", &result).unwrap();

        std::fs::write(temp_dir.path().join("other.txt"), "not a msgpack file").unwrap();

        cache.clear().unwrap();

        assert!(cache.get_cached_result("test1", "tesseract", "eng").unwrap().is_none());
        assert!(cache.get_cached_result("test2", "tesseract", "eng").unwrap().is_none());

        assert!(temp_dir.path().join("other.txt").exists());
    }

    #[test]
    fn test_cache_stats_nonexistent_dir() {
        let temp_dir = tempdir().unwrap();
        let cache_path = temp_dir.path().join("nonexistent");
        let cache = OCRCache { cache_dir: cache_path };

        let stats = cache.get_stats().unwrap();
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.total_size_mb, 0.0);
    }

    #[test]
    fn test_cache_clear_nonexistent_dir() {
        let temp_dir = tempdir().unwrap();
        let cache_path = temp_dir.path().join("nonexistent");
        let cache = OCRCache { cache_dir: cache_path };

        assert!(cache.clear().is_ok());
    }

    #[test]
    fn test_cache_get_path() {
        let temp_dir = tempdir().unwrap();
        let cache = OCRCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let path = cache.get_cache_path("abc123");

        assert!(path.to_string_lossy().contains("abc123.msgpack"));
        assert!(path.parent().unwrap() == temp_dir.path());
    }

    #[test]
    fn test_cache_stats_default() {
        let stats = OCRCacheStats::default();
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.total_size_mb, 0.0);
    }

    #[test]
    fn test_cache_empty_content() {
        let temp_dir = tempdir().unwrap();
        let cache = OCRCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let result = ExtractionResultDTO {
            content: String::new(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: Vec::new(),
        };

        cache.set_cached_result("empty", "tesseract", "eng", &result).unwrap();

        let retrieved = cache.get_cached_result("empty", "tesseract", "eng").unwrap();
        assert_eq!(retrieved.unwrap().content, "");
    }

    #[test]
    fn test_cache_large_content() {
        let temp_dir = tempdir().unwrap();
        let cache = OCRCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let large_content = "x".repeat(10_000);

        let result = ExtractionResultDTO {
            content: large_content.clone(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: Vec::new(),
        };

        cache.set_cached_result("large", "tesseract", "eng", &result).unwrap();

        let retrieved = cache.get_cached_result("large", "tesseract", "eng").unwrap();
        assert_eq!(retrieved.unwrap().content.len(), 10_000);
    }
}
