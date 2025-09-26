use crate::error_utils::{IntoKreuzbergError, errors};
use ahash::AHasher;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Cache statistics structure
#[pyclass]
#[derive(Debug, Clone)]
pub struct CacheStats {
    #[pyo3(get)]
    pub total_files: usize,
    #[pyo3(get)]
    pub total_size_mb: f64,
    #[pyo3(get)]
    pub available_space_mb: f64,
    #[pyo3(get)]
    pub oldest_file_age_days: f64,
    #[pyo3(get)]
    pub newest_file_age_days: f64,
}

/// Cache entry metadata
#[derive(Debug, Clone)]
struct CacheEntry {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

/// Cache scan result combining stats and entries
struct CacheScanResult {
    stats: CacheStats,
    entries: Vec<CacheEntry>,
}

/// Helper to format key-value pairs efficiently
#[inline]
fn format_cache_part(key: &str, val: &Bound<'_, PyAny>) -> String {
    if let Ok(s) = val.extract::<String>() {
        format!("{}={}", key, s)
    } else if let Ok(i) = val.extract::<i64>() {
        format!("{}={}", key, i)
    } else if let Ok(f) = val.extract::<f64>() {
        format!("{}={}", key, f)
    } else if let Ok(b) = val.extract::<bool>() {
        format!("{}={}", key, b)
    } else if let Ok(bytes) = val.downcast::<PyBytes>() {
        format!("{}=bytes:{}", key, bytes.len().unwrap_or(0))
    } else {
        let type_name = val.get_type().name().map_or("unknown".to_string(), |n| n.to_string());
        format!("{}={}:{}", key, type_name, val)
    }
}

/// Generate cache key from kwargs dictionary
#[pyfunction]
#[pyo3(signature = (**kwargs))]
pub fn generate_cache_key(kwargs: Option<&Bound<'_, PyDict>>) -> String {
    if kwargs.is_none_or(|d| d.len() == 0) {
        return "empty".to_string();
    }

    let dict = kwargs.unwrap();
    let mut parts: Vec<String> = Vec::with_capacity(dict.len());

    let mut keys: Vec<String> = Vec::new();
    for key in dict.keys() {
        if let Ok(key_str) = key.extract::<String>() {
            keys.push(key_str);
        }
    }
    keys.sort();

    for key in keys {
        if let Ok(Some(val)) = dict.get_item(&key) {
            parts.push(format_cache_part(&key, &val));
        }
    }

    let estimated_size = parts.iter().map(|p| p.len()).sum::<usize>() + parts.len();
    let mut cache_str = String::with_capacity(estimated_size);
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            cache_str.push('&');
        }
        cache_str.push_str(part);
    }

    let mut hasher = AHasher::default();
    cache_str.hash(&mut hasher);
    let hash = hasher.finish();

    format!("{:032x}", hash)
}

/// Batch cache key generation for multiple items
#[pyfunction]
pub fn batch_generate_cache_keys(items: &Bound<'_, PyList>) -> PyResult<Vec<String>> {
    let mut results = Vec::with_capacity(items.len());

    for item in items.iter() {
        if let Ok(dict) = item.downcast::<PyDict>() {
            results.push(generate_cache_key(Some(dict)));
        } else {
            results.push("invalid".to_string());
        }
    }

    Ok(results)
}

/// Get available disk space for a given path
#[pyfunction]
pub fn get_available_disk_space(path: &str) -> PyResult<f64> {
    let path = Path::new(path);

    let check_path = if path.exists() {
        path
    } else if let Some(parent) = path.parent() {
        parent
    } else {
        Path::new("/")
    };

    #[cfg(unix)]
    {
        use libc::{statvfs, statvfs as statvfs_struct};
        use std::ffi::CString;

        let path_str = check_path
            .to_str()
            .ok_or_else(|| errors::value_error("Path error", "contains invalid UTF-8"))?;
        let c_path = CString::new(path_str)?;
        let mut stat: statvfs_struct = unsafe { std::mem::zeroed() };

        let result = unsafe { statvfs(c_path.as_ptr(), &mut stat) };

        if result == 0 {
            let available_bytes = stat.f_bavail as u64 * stat.f_frsize;
            Ok(available_bytes as f64 / (1024.0 * 1024.0))
        } else {
            eprintln!("Failed to get disk stats for {}: errno {}", path_str, result);
            Ok(10000.0)
        }
    }

    #[cfg(not(unix))]
    {
        Ok(10000.0)
    }
}

/// Scan cache directory and collect metadata efficiently
/// This combines the functionality of get_cache_metadata and entry collection
fn scan_cache_directory(cache_dir: &str) -> PyResult<CacheScanResult> {
    let dir_path = Path::new(cache_dir);

    if !dir_path.exists() {
        return Ok(CacheScanResult {
            stats: CacheStats {
                total_files: 0,
                total_size_mb: 0.0,
                available_space_mb: get_available_disk_space(cache_dir)?,
                oldest_file_age_days: 0.0,
                newest_file_age_days: 0.0,
            },
            entries: Vec::new(),
        });
    }

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as f64;

    let read_dir = fs::read_dir(dir_path).into_io_error("Failed to read cache directory")?;

    let mut total_size = 0u64;
    let mut oldest_age = 0.0f64;
    let mut newest_age = f64::INFINITY;
    let mut entries = Vec::new();

    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Error reading cache entry: {}", e);
                continue;
            }
        };

        let metadata = match entry.metadata() {
            Ok(m) if m.is_file() => m,
            _ => continue,
        };

        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("msgpack") {
            continue;
        }

        let modified = match metadata.modified() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Error getting modification time for {:?}: {}", path, e);
                continue;
            }
        };

        let size = metadata.len();
        total_size += size;

        if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
            let age_days = (current_time - duration.as_secs() as f64) / (24.0 * 3600.0);
            oldest_age = oldest_age.max(age_days);
            newest_age = newest_age.min(age_days);
        }

        entries.push(CacheEntry { path, size, modified });
    }

    if entries.is_empty() {
        oldest_age = 0.0;
        newest_age = 0.0;
    }

    Ok(CacheScanResult {
        stats: CacheStats {
            total_files: entries.len(),
            total_size_mb: total_size as f64 / (1024.0 * 1024.0),
            available_space_mb: get_available_disk_space(cache_dir)?,
            oldest_file_age_days: oldest_age,
            newest_file_age_days: newest_age,
        },
        entries,
    })
}

/// Calculate total cache size and get file metadata
#[pyfunction]
pub fn get_cache_metadata(cache_dir: &str) -> PyResult<CacheStats> {
    let scan_result = scan_cache_directory(cache_dir)?;
    Ok(scan_result.stats)
}

/// Clean up old cache entries based on age and size limits
#[pyfunction]
pub fn cleanup_cache(
    cache_dir: &str,
    max_age_days: f64,
    max_size_mb: f64,
    target_size_ratio: f64,
) -> PyResult<(usize, f64)> {
    let scan_result = scan_cache_directory(cache_dir)?;

    if scan_result.entries.is_empty() {
        return Ok((0, 0.0));
    }

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as f64;
    let max_age_seconds = max_age_days * 24.0 * 3600.0;

    let mut removed_count = 0;
    let mut removed_size = 0.0;
    let mut remaining_entries = Vec::new();
    let mut total_remaining_size = 0u64;

    for entry in scan_result.entries {
        if let Ok(age) = entry.modified.duration_since(UNIX_EPOCH) {
            let age_seconds = current_time - age.as_secs() as f64;
            if age_seconds > max_age_seconds {
                match fs::remove_file(&entry.path) {
                    Ok(_) => {
                        removed_count += 1;
                        removed_size += entry.size as f64 / (1024.0 * 1024.0);
                    }
                    Err(e) => {
                        eprintln!("Failed to remove {:?}: {}", entry.path, e);
                    }
                }
            } else {
                total_remaining_size += entry.size;
                remaining_entries.push(entry);
            }
        }
    }

    let mut total_size_mb = total_remaining_size as f64 / (1024.0 * 1024.0);

    if total_size_mb > max_size_mb {
        remaining_entries.sort_by_key(|e| e.modified);

        let target_size = max_size_mb * target_size_ratio;

        for entry in remaining_entries {
            if total_size_mb <= target_size {
                break;
            }

            match fs::remove_file(&entry.path) {
                Ok(_) => {
                    let size_mb = entry.size as f64 / (1024.0 * 1024.0);
                    removed_count += 1;
                    removed_size += size_mb;
                    total_size_mb -= size_mb;
                }
                Err(e) => {
                    eprintln!("Failed to remove {:?}: {}", entry.path, e);
                }
            }
        }
    }

    Ok((removed_count, removed_size))
}

/// Smart cache cleanup based on available disk space
#[pyfunction]
pub fn smart_cleanup_cache(
    cache_dir: &str,
    max_age_days: f64,
    max_size_mb: f64,
    min_free_space_mb: f64,
) -> PyResult<(usize, f64)> {
    let stats = get_cache_metadata(cache_dir)?;

    let needs_cleanup = stats.available_space_mb < min_free_space_mb
        || stats.total_size_mb > max_size_mb
        || stats.oldest_file_age_days > max_age_days;

    if !needs_cleanup {
        return Ok((0, 0.0));
    }

    let target_ratio = if stats.available_space_mb < min_free_space_mb {
        0.5
    } else {
        0.8
    };

    cleanup_cache(cache_dir, max_age_days, max_size_mb, target_ratio)
}

/// Filter old cache entries
#[pyfunction]
pub fn filter_old_cache_entries(cache_times: Vec<f64>, current_time: f64, max_age_seconds: f64) -> Vec<usize> {
    cache_times
        .iter()
        .enumerate()
        .filter_map(|(idx, &time)| {
            if current_time - time > max_age_seconds {
                Some(idx)
            } else {
                None
            }
        })
        .collect()
}

/// Sort cache entries by access time for LRU eviction
#[pyfunction]
pub fn sort_cache_by_access_time(mut entries: Vec<(String, f64)>) -> Vec<String> {
    entries.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    entries.into_iter().map(|(key, _)| key).collect()
}

/// Fast hash function for deduplication
#[pyfunction]
pub fn fast_hash(data: &[u8]) -> u64 {
    let mut hasher = AHasher::default();
    data.hash(&mut hasher);
    hasher.finish()
}

/// Concurrent safe cache key validation
#[pyfunction]
pub fn validate_cache_key(key: &str) -> bool {
    key.len() == 32 && key.chars().all(|c| c.is_ascii_hexdigit())
}

/// Check if cache path exists and is valid
#[pyfunction]
pub fn is_cache_valid(cache_path: &str, max_age_days: f64) -> bool {
    let path = Path::new(cache_path);

    if !path.exists() {
        return false;
    }

    match fs::metadata(path) {
        Ok(metadata) => match metadata.modified() {
            Ok(modified) => match SystemTime::now().duration_since(modified) {
                Ok(elapsed) => {
                    let age_days = elapsed.as_secs() as f64 / (24.0 * 3600.0);
                    age_days <= max_age_days
                }
                Err(_) => false,
            },
            Err(_) => false,
        },
        Err(_) => false,
    }
}

/// Clear entire cache directory
#[pyfunction]
pub fn clear_cache_directory(cache_dir: &str) -> PyResult<(usize, f64)> {
    let dir_path = Path::new(cache_dir);

    if !dir_path.exists() {
        return Ok((0, 0.0));
    }

    let mut removed_count = 0;
    let mut removed_size = 0.0;

    let read_dir = fs::read_dir(dir_path)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(format!("Failed to read cache directory: {}", e)))?;

    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Error reading entry: {}", e);
                continue;
            }
        };

        let metadata = match entry.metadata() {
            Ok(m) if m.is_file() => m,
            _ => continue,
        };

        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("msgpack") {
            continue;
        }

        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
        match fs::remove_file(&path) {
            Ok(_) => {
                removed_count += 1;
                removed_size += size_mb;
            }
            Err(e) => {
                eprintln!("Failed to remove {:?}: {}", path, e);
            }
        }
    }

    Ok((removed_count, removed_size))
}

/// Batch cleanup for multiple cache directories
#[pyfunction]
pub fn batch_cleanup_caches(
    cache_dirs: Vec<String>,
    max_age_days: f64,
    max_size_mb: f64,
    min_free_space_mb: f64,
) -> PyResult<Vec<(usize, f64)>> {
    cache_dirs
        .into_iter()
        .map(|dir| smart_cleanup_cache(&dir, max_age_days, max_size_mb, min_free_space_mb))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_generate_cache_key_empty() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|_py| {
            let result = generate_cache_key(None);
            assert_eq!(result, "empty");
        });
    }

    #[test]
    fn test_generate_cache_key_consistent() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            use pyo3::types::IntoPyDict;
            let dict = [("key1", "value1"), ("key2", "value2")].into_py_dict(py).unwrap();
            let key1 = generate_cache_key(Some(&dict));
            let key2 = generate_cache_key(Some(&dict));
            assert_eq!(key1, key2);
            assert_eq!(key1.len(), 32);
        });
    }

    #[test]
    fn test_validate_cache_key() {
        assert!(validate_cache_key("0123456789abcdef0123456789abcdef"));
        assert!(!validate_cache_key("invalid_key"));
        assert!(!validate_cache_key("0123456789abcdef"));
        assert!(!validate_cache_key("0123456789abcdef0123456789abcdef0"));
    }

    #[test]
    fn test_fast_hash() {
        let data1 = b"test data";
        let data2 = b"test data";
        let data3 = b"different data";

        assert_eq!(fast_hash(data1), fast_hash(data2));
        assert_ne!(fast_hash(data1), fast_hash(data3));
    }

    #[test]
    fn test_filter_old_cache_entries() {
        let cache_times = vec![100.0, 200.0, 300.0, 400.0];
        let current_time = 500.0;
        let max_age = 200.0;

        let old_indices = filter_old_cache_entries(cache_times, current_time, max_age);
        assert_eq!(old_indices, vec![0, 1]);
    }

    #[test]
    fn test_sort_cache_by_access_time() {
        let entries = vec![
            ("key3".to_string(), 300.0),
            ("key1".to_string(), 100.0),
            ("key2".to_string(), 200.0),
        ];

        let sorted = sort_cache_by_access_time(entries);
        assert_eq!(sorted, vec!["key1", "key2", "key3"]);
    }

    #[test]
    fn test_sort_cache_with_nan() {
        let entries = vec![
            ("key1".to_string(), 100.0),
            ("key2".to_string(), f64::NAN),
            ("key3".to_string(), 200.0),
        ];

        let sorted = sort_cache_by_access_time(entries);
        assert_eq!(sorted.len(), 3);
    }

    #[test]
    fn test_cache_metadata() {
        let temp_dir = tempdir().unwrap();
        let cache_dir = temp_dir.path().to_str().unwrap();

        let file1 = temp_dir.path().join("test1.msgpack");
        let file2 = temp_dir.path().join("test2.msgpack");
        File::create(&file1).unwrap();
        File::create(&file2).unwrap();

        let stats = get_cache_metadata(cache_dir).unwrap();
        assert_eq!(stats.total_files, 2);
        assert!(stats.available_space_mb > 0.0);
    }

    #[test]
    fn test_cleanup_cache() {
        use std::io::Write;

        let temp_dir = tempdir().unwrap();
        let cache_dir = temp_dir.path().to_str().unwrap();

        let file1 = temp_dir.path().join("old.msgpack");
        let mut f = File::create(&file1).unwrap();
        f.write_all(b"test data for cleanup").unwrap();
        drop(f);

        let (removed_count, _) = cleanup_cache(cache_dir, 1000.0, 0.000001, 0.8).unwrap();
        assert_eq!(removed_count, 1);
        assert!(!file1.exists());
    }

    #[test]
    fn test_is_cache_valid() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.msgpack");
        File::create(&file_path).unwrap();

        let path_str = file_path.to_str().unwrap();

        assert!(is_cache_valid(path_str, 1.0));

        assert!(!is_cache_valid("/nonexistent/path", 1.0));
    }
}
