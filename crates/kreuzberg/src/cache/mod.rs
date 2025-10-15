use crate::error::{KreuzbergError, Result};
use ahash::AHasher;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_files: usize,
    pub total_size_mb: f64,
    pub available_space_mb: f64,
    pub oldest_file_age_days: f64,
    pub newest_file_age_days: f64,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

struct CacheScanResult {
    stats: CacheStats,
    entries: Vec<CacheEntry>,
}

pub struct GenericCache {
    cache_dir: PathBuf,
    cache_type: String,
    max_age_days: f64,
    max_cache_size_mb: f64,
    min_free_space_mb: f64,
    processing_locks: Arc<Mutex<HashSet<String>>>,
}

impl GenericCache {
    pub fn new(
        cache_type: String,
        cache_dir: Option<String>,
        max_age_days: f64,
        max_cache_size_mb: f64,
        min_free_space_mb: f64,
    ) -> Result<Self> {
        let cache_dir_path = if let Some(dir) = cache_dir {
            PathBuf::from(dir).join(&cache_type)
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(".kreuzberg")
                .join(&cache_type)
        };

        fs::create_dir_all(&cache_dir_path)
            .map_err(|e| KreuzbergError::cache(format!("Failed to create cache directory: {}", e)))?;

        Ok(Self {
            cache_dir: cache_dir_path,
            cache_type,
            max_age_days,
            max_cache_size_mb,
            min_free_space_mb,
            processing_locks: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    fn get_cache_path(&self, cache_key: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.msgpack", cache_key))
    }

    fn get_metadata_path(&self, cache_key: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.meta", cache_key))
    }

    fn is_valid(&self, cache_path: &Path, source_file: Option<&str>) -> bool {
        if !cache_path.exists() {
            return false;
        }

        if let Ok(metadata) = fs::metadata(cache_path)
            && let Ok(modified) = metadata.modified()
            && let Ok(elapsed) = SystemTime::now().duration_since(modified)
        {
            let age_days = elapsed.as_secs() as f64 / (24.0 * 3600.0);
            if age_days > self.max_age_days {
                return false;
            }
        }

        if let Some(source_path) = source_file {
            let meta_path = self.get_metadata_path(cache_path.file_stem().and_then(|s| s.to_str()).unwrap_or(""));

            if meta_path.exists() {
                if let Ok(cached_meta_bytes) = fs::read(&meta_path)
                    && cached_meta_bytes.len() >= 16
                {
                    let cached_size = u64::from_le_bytes([
                        cached_meta_bytes[0],
                        cached_meta_bytes[1],
                        cached_meta_bytes[2],
                        cached_meta_bytes[3],
                        cached_meta_bytes[4],
                        cached_meta_bytes[5],
                        cached_meta_bytes[6],
                        cached_meta_bytes[7],
                    ]);
                    let cached_mtime = u64::from_le_bytes([
                        cached_meta_bytes[8],
                        cached_meta_bytes[9],
                        cached_meta_bytes[10],
                        cached_meta_bytes[11],
                        cached_meta_bytes[12],
                        cached_meta_bytes[13],
                        cached_meta_bytes[14],
                        cached_meta_bytes[15],
                    ]);

                    if let Ok(source_metadata) = fs::metadata(source_path) {
                        let current_size = source_metadata.len();
                        let current_mtime = source_metadata
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0);

                        return cached_size == current_size && cached_mtime == current_mtime;
                    }
                }
                return false;
            }
        }

        true
    }

    fn save_metadata(&self, cache_key: &str, source_file: Option<&str>) {
        if let Some(source_path) = source_file
            && let Ok(metadata) = fs::metadata(source_path)
        {
            let size = metadata.len();
            let mtime = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let mut bytes = Vec::with_capacity(16);
            bytes.extend_from_slice(&size.to_le_bytes());
            bytes.extend_from_slice(&mtime.to_le_bytes());

            let meta_path = self.get_metadata_path(cache_key);
            let _ = fs::write(meta_path, bytes);
        }
    }

    pub fn get(&self, cache_key: &str, source_file: Option<&str>) -> Result<Option<Vec<u8>>> {
        let cache_path = self.get_cache_path(cache_key);

        if !self.is_valid(&cache_path, source_file) {
            return Ok(None);
        }

        match fs::read(&cache_path) {
            Ok(content) => Ok(Some(content)),
            Err(_) => {
                let _ = fs::remove_file(&cache_path);
                let _ = fs::remove_file(self.get_metadata_path(cache_key));
                Ok(None)
            }
        }
    }

    pub fn set(&self, cache_key: &str, data: Vec<u8>, source_file: Option<&str>) -> Result<()> {
        let cache_path = self.get_cache_path(cache_key);

        fs::write(&cache_path, data)
            .map_err(|e| KreuzbergError::cache(format!("Failed to write cache file: {}", e)))?;

        self.save_metadata(cache_key, source_file);

        let mut hasher = AHasher::default();
        cache_key.hash(&mut hasher);
        if hasher.finish() % 100 == 0 {
            let _ = smart_cleanup_cache(
                self.cache_dir.to_str().unwrap_or("."),
                self.max_age_days,
                self.max_cache_size_mb,
                self.min_free_space_mb,
            );
        }

        Ok(())
    }

    pub fn is_processing(&self, cache_key: &str) -> bool {
        let locks = self
            .processing_locks
            .lock()
            .expect("Processing locks mutex poisoned - another thread panicked while processing cache");
        locks.contains(cache_key)
    }

    pub fn mark_processing(&self, cache_key: String) {
        let mut locks = self
            .processing_locks
            .lock()
            .expect("Processing locks mutex poisoned - another thread panicked while processing cache");
        locks.insert(cache_key);
    }

    pub fn mark_complete(&self, cache_key: &str) {
        let mut locks = self
            .processing_locks
            .lock()
            .expect("Processing locks mutex poisoned - another thread panicked while processing cache");
        locks.remove(cache_key);
    }

    pub fn clear(&self) -> Result<(usize, f64)> {
        clear_cache_directory(self.cache_dir.to_str().unwrap_or("."))
    }

    pub fn get_stats(&self) -> Result<CacheStats> {
        get_cache_metadata(self.cache_dir.to_str().unwrap_or("."))
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn cache_type(&self) -> &str {
        &self.cache_type
    }
}

pub fn generate_cache_key(parts: &[(&str, &str)]) -> String {
    if parts.is_empty() {
        return "empty".to_string();
    }

    let mut sorted_parts: Vec<_> = parts.to_vec();
    sorted_parts.sort_by_key(|(k, _)| *k);

    let estimated_size = sorted_parts.iter().map(|(k, v)| k.len() + v.len() + 2).sum::<usize>();
    let mut cache_str = String::with_capacity(estimated_size);

    for (i, (key, val)) in sorted_parts.iter().enumerate() {
        if i > 0 {
            cache_str.push('&');
        }
        cache_str.push_str(&format!("{}={}", key, val));
    }

    let mut hasher = AHasher::default();
    cache_str.hash(&mut hasher);
    let hash = hasher.finish();

    format!("{:032x}", hash)
}

pub fn get_available_disk_space(path: &str) -> Result<f64> {
    #[cfg(unix)]
    {
        let path = Path::new(path);
        let check_path = if path.exists() {
            path
        } else if let Some(parent) = path.parent() {
            parent
        } else {
            Path::new("/")
        };

        use libc::{statvfs, statvfs as statvfs_struct};
        use std::ffi::CString;

        let path_str = check_path
            .to_str()
            .ok_or_else(|| KreuzbergError::validation("Path contains invalid UTF-8".to_string()))?;
        let c_path = CString::new(path_str).map_err(|e| KreuzbergError::validation(format!("Invalid path: {}", e)))?;

        // SAFETY: statvfs is a valid C struct defined by POSIX that can be zero-initialized.
        // All fields are integers (u64, c_ulong) which are safe when zeroed per the C standard.
        let mut stat: statvfs_struct = unsafe { std::mem::zeroed() };

        // SAFETY: c_path is a valid null-terminated C string (CString guarantees this),
        // stat is a valid mutable reference to a properly initialized statvfs struct,
        // and statvfs is a standard POSIX syscall that won't cause UB when called correctly.
        let result = unsafe { statvfs(c_path.as_ptr(), &mut stat) };

        if result == 0 {
            #[allow(clippy::unnecessary_cast)]
            let available_bytes = stat.f_bavail as u64 * stat.f_frsize as u64;
            Ok(available_bytes as f64 / (1024.0 * 1024.0))
        } else {
            eprintln!("Failed to get disk stats for {}: errno {}", path_str, result);
            Ok(10000.0)
        }
    }

    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(10000.0)
    }
}

fn scan_cache_directory(cache_dir: &str) -> Result<CacheScanResult> {
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

    let read_dir =
        fs::read_dir(dir_path).map_err(|e| KreuzbergError::cache(format!("Failed to read cache directory: {}", e)))?;

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

pub fn get_cache_metadata(cache_dir: &str) -> Result<CacheStats> {
    let scan_result = scan_cache_directory(cache_dir)?;
    Ok(scan_result.stats)
}

pub fn cleanup_cache(
    cache_dir: &str,
    max_age_days: f64,
    max_size_mb: f64,
    target_size_ratio: f64,
) -> Result<(usize, f64)> {
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

pub fn smart_cleanup_cache(
    cache_dir: &str,
    max_age_days: f64,
    max_size_mb: f64,
    min_free_space_mb: f64,
) -> Result<(usize, f64)> {
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

pub fn filter_old_cache_entries(cache_times: &[f64], current_time: f64, max_age_seconds: f64) -> Vec<usize> {
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

pub fn sort_cache_by_access_time(mut entries: Vec<(String, f64)>) -> Vec<String> {
    entries.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    entries.into_iter().map(|(key, _)| key).collect()
}

pub fn fast_hash(data: &[u8]) -> u64 {
    let mut hasher = AHasher::default();
    data.hash(&mut hasher);
    hasher.finish()
}

pub fn validate_cache_key(key: &str) -> bool {
    key.len() == 32 && key.chars().all(|c| c.is_ascii_hexdigit())
}

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

pub fn clear_cache_directory(cache_dir: &str) -> Result<(usize, f64)> {
    let dir_path = Path::new(cache_dir);

    if !dir_path.exists() {
        return Ok((0, 0.0));
    }

    let mut removed_count = 0;
    let mut removed_size = 0.0;

    let read_dir =
        fs::read_dir(dir_path).map_err(|e| KreuzbergError::cache(format!("Failed to read cache directory: {}", e)))?;

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

pub fn batch_cleanup_caches(
    cache_dirs: &[&str],
    max_age_days: f64,
    max_size_mb: f64,
    min_free_space_mb: f64,
) -> Result<Vec<(usize, f64)>> {
    cache_dirs
        .iter()
        .map(|dir| smart_cleanup_cache(dir, max_age_days, max_size_mb, min_free_space_mb))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_generate_cache_key_empty() {
        let result = generate_cache_key(&[]);
        assert_eq!(result, "empty");
    }

    #[test]
    fn test_generate_cache_key_consistent() {
        let parts = [("key1", "value1"), ("key2", "value2")];
        let key1 = generate_cache_key(&parts);
        let key2 = generate_cache_key(&parts);
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 32);
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

        let old_indices = filter_old_cache_entries(&cache_times, current_time, max_age);
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

    #[test]
    fn test_generic_cache_new() {
        let temp_dir = tempdir().unwrap();
        let cache = GenericCache::new(
            "test".to_string(),
            Some(temp_dir.path().to_str().unwrap().to_string()),
            30.0,
            500.0,
            1000.0,
        )
        .unwrap();

        assert_eq!(cache.cache_type, "test");
        assert!(cache.cache_dir.exists());
    }

    #[test]
    fn test_generic_cache_get_set() {
        let temp_dir = tempdir().unwrap();
        let cache = GenericCache::new(
            "test".to_string(),
            Some(temp_dir.path().to_str().unwrap().to_string()),
            30.0,
            500.0,
            1000.0,
        )
        .unwrap();

        let cache_key = "test_key";
        let data = b"test data".to_vec();

        cache.set(cache_key, data.clone(), None).unwrap();

        let result = cache.get(cache_key, None).unwrap();
        assert_eq!(result, Some(data));
    }

    #[test]
    fn test_generic_cache_get_miss() {
        let temp_dir = tempdir().unwrap();
        let cache = GenericCache::new(
            "test".to_string(),
            Some(temp_dir.path().to_str().unwrap().to_string()),
            30.0,
            500.0,
            1000.0,
        )
        .unwrap();

        let result = cache.get("nonexistent", None).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_generic_cache_source_file_invalidation() {
        use std::io::Write;
        use std::thread::sleep;
        use std::time::Duration;

        let temp_dir = tempdir().unwrap();
        let cache = GenericCache::new(
            "test".to_string(),
            Some(temp_dir.path().to_str().unwrap().to_string()),
            30.0,
            500.0,
            1000.0,
        )
        .unwrap();

        let source_file = temp_dir.path().join("source.txt");
        let mut f = File::create(&source_file).unwrap();
        f.write_all(b"original content").unwrap();
        drop(f);

        let cache_key = "test_key";
        let data = b"cached data".to_vec();

        cache
            .set(cache_key, data.clone(), Some(source_file.to_str().unwrap()))
            .unwrap();

        let result = cache.get(cache_key, Some(source_file.to_str().unwrap())).unwrap();
        assert_eq!(result, Some(data.clone()));

        sleep(Duration::from_millis(10));
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&source_file)
            .unwrap();
        f.write_all(b"modified content with different size").unwrap();
        drop(f);

        let result = cache.get(cache_key, Some(source_file.to_str().unwrap())).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_generic_cache_processing_locks() {
        let temp_dir = tempdir().unwrap();
        let cache = GenericCache::new(
            "test".to_string(),
            Some(temp_dir.path().to_str().unwrap().to_string()),
            30.0,
            500.0,
            1000.0,
        )
        .unwrap();

        let cache_key = "test_key";

        assert!(!cache.is_processing(cache_key));

        cache.mark_processing(cache_key.to_string());
        assert!(cache.is_processing(cache_key));

        cache.mark_complete(cache_key);
        assert!(!cache.is_processing(cache_key));
    }

    #[test]
    fn test_generic_cache_clear() {
        let temp_dir = tempdir().unwrap();
        let cache = GenericCache::new(
            "test".to_string(),
            Some(temp_dir.path().to_str().unwrap().to_string()),
            30.0,
            500.0,
            1000.0,
        )
        .unwrap();

        cache.set("key1", b"data1".to_vec(), None).unwrap();
        cache.set("key2", b"data2".to_vec(), None).unwrap();

        let (removed, _freed) = cache.clear().unwrap();
        assert_eq!(removed, 2);

        assert_eq!(cache.get("key1", None).unwrap(), None);
        assert_eq!(cache.get("key2", None).unwrap(), None);
    }

    #[test]
    fn test_generic_cache_stats() {
        let temp_dir = tempdir().unwrap();
        let cache = GenericCache::new(
            "test".to_string(),
            Some(temp_dir.path().to_str().unwrap().to_string()),
            30.0,
            500.0,
            1000.0,
        )
        .unwrap();

        cache.set("key1", b"test data 1".to_vec(), None).unwrap();
        cache.set("key2", b"test data 2".to_vec(), None).unwrap();

        let stats = cache.get_stats().unwrap();
        assert_eq!(stats.total_files, 2);
        assert!(stats.total_size_mb > 0.0);
        assert!(stats.available_space_mb > 0.0);
    }

    #[test]
    fn test_generic_cache_expired_entry() {
        use std::io::Write;

        let temp_dir = tempdir().unwrap();
        let cache = GenericCache::new(
            "test".to_string(),
            Some(temp_dir.path().to_str().unwrap().to_string()),
            0.000001,
            500.0,
            1000.0,
        )
        .unwrap();

        let cache_key = "test_key";

        let cache_path = cache.cache_dir.join(format!("{}.msgpack", cache_key));
        let mut f = File::create(&cache_path).unwrap();
        f.write_all(b"test data").unwrap();
        drop(f);

        let old_time = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        filetime::set_file_mtime(&cache_path, filetime::FileTime::from_system_time(old_time)).unwrap();

        let result = cache.get(cache_key, None).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_generic_cache_properties() {
        let temp_dir = tempdir().unwrap();
        let cache = GenericCache::new(
            "test".to_string(),
            Some(temp_dir.path().to_str().unwrap().to_string()),
            30.0,
            500.0,
            1000.0,
        )
        .unwrap();

        assert_eq!(cache.cache_type(), "test");
        assert!(cache.cache_dir().to_string_lossy().contains("test"));
    }
}
