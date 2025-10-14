use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};

#[pyclass]
#[derive(Debug, Clone)]
pub struct CacheStatsDTO {
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

impl From<kreuzberg::cache::CacheStats> for CacheStatsDTO {
    fn from(stats: kreuzberg::cache::CacheStats) -> Self {
        Self {
            total_files: stats.total_files,
            total_size_mb: stats.total_size_mb,
            available_space_mb: stats.available_space_mb,
            oldest_file_age_days: stats.oldest_file_age_days,
            newest_file_age_days: stats.newest_file_age_days,
        }
    }
}

#[pyclass(name = "GenericCache")]
pub struct GenericCacheDTO {
    inner: kreuzberg::cache::GenericCache,
}

#[pymethods]
impl GenericCacheDTO {
    #[new]
    #[pyo3(signature = (cache_type, cache_dir=None, max_age_days=30.0, max_cache_size_mb=500.0, min_free_space_mb=1000.0))]
    pub fn new(
        cache_type: String,
        cache_dir: Option<String>,
        max_age_days: f64,
        max_cache_size_mb: f64,
        min_free_space_mb: f64,
    ) -> PyResult<Self> {
        let inner = kreuzberg::cache::GenericCache::new(
            cache_type,
            cache_dir,
            max_age_days,
            max_cache_size_mb,
            min_free_space_mb,
        )
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Cache creation error: {}", e)))?;

        Ok(Self { inner })
    }

    #[pyo3(signature = (cache_key, source_file=None))]
    pub fn get(&self, cache_key: String, source_file: Option<String>) -> PyResult<Option<Vec<u8>>> {
        self.inner
            .get(&cache_key, source_file.as_deref())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Cache get error: {}", e)))
    }

    #[pyo3(signature = (cache_key, data, source_file=None))]
    pub fn set(&self, cache_key: String, data: Vec<u8>, source_file: Option<String>) -> PyResult<()> {
        self.inner
            .set(&cache_key, data, source_file.as_deref())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Cache set error: {}", e)))
    }

    pub fn is_processing(&self, cache_key: String) -> bool {
        self.inner.is_processing(&cache_key)
    }

    pub fn mark_processing(&self, cache_key: String) {
        self.inner.mark_processing(cache_key);
    }

    pub fn mark_complete(&self, cache_key: String) {
        self.inner.mark_complete(&cache_key);
    }

    pub fn clear(&self) -> PyResult<(usize, f64)> {
        self.inner
            .clear()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Cache clear error: {}", e)))
    }

    pub fn get_stats(&self) -> PyResult<CacheStatsDTO> {
        let stats = self
            .inner
            .get_stats()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Cache stats error: {}", e)))?;
        Ok(CacheStatsDTO::from(stats))
    }

    #[getter]
    pub fn cache_dir(&self) -> String {
        self.inner.cache_dir().to_string_lossy().to_string()
    }

    #[getter]
    pub fn cache_type_name(&self) -> String {
        self.inner.cache_type().to_string()
    }
}

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

#[pyfunction]
#[pyo3(signature = (**kwargs))]
pub fn generate_cache_key(kwargs: Option<&Bound<'_, PyDict>>) -> String {
    if kwargs.is_none_or(|d| d.len() == 0) {
        return "empty".to_string();
    }

    let dict = kwargs.unwrap();
    let mut parts: Vec<(&str, String)> = Vec::with_capacity(dict.len());

    let mut keys: Vec<String> = Vec::new();
    for key in dict.keys() {
        if let Ok(key_str) = key.extract::<String>() {
            keys.push(key_str);
        }
    }
    keys.sort();

    for key in &keys {
        if let Ok(Some(val)) = dict.get_item(key) {
            let formatted = format_cache_part(key, &val);
            parts.push((key, formatted));
        }
    }

    let parts_slice: Vec<(&str, &str)> = parts.iter().map(|(k, v)| (*k, v.as_ref())).collect();
    kreuzberg::cache::generate_cache_key(&parts_slice)
}

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

#[pyfunction]
pub fn get_available_disk_space(path: &str) -> PyResult<f64> {
    kreuzberg::cache::get_available_disk_space(path)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Disk space check error: {}", e)))
}

#[pyfunction]
pub fn get_cache_metadata(cache_dir: &str) -> PyResult<CacheStatsDTO> {
    let stats = kreuzberg::cache::get_cache_metadata(cache_dir)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Cache metadata error: {}", e)))?;
    Ok(CacheStatsDTO::from(stats))
}

#[pyfunction]
pub fn cleanup_cache(
    cache_dir: &str,
    max_age_days: f64,
    max_size_mb: f64,
    target_size_ratio: f64,
) -> PyResult<(usize, f64)> {
    kreuzberg::cache::cleanup_cache(cache_dir, max_age_days, max_size_mb, target_size_ratio)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Cache cleanup error: {}", e)))
}

#[pyfunction]
pub fn smart_cleanup_cache(
    cache_dir: &str,
    max_age_days: f64,
    max_size_mb: f64,
    min_free_space_mb: f64,
) -> PyResult<(usize, f64)> {
    kreuzberg::cache::smart_cleanup_cache(cache_dir, max_age_days, max_size_mb, min_free_space_mb)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Cache cleanup error: {}", e)))
}

#[pyfunction]
pub fn filter_old_cache_entries(cache_times: Vec<f64>, current_time: f64, max_age_seconds: f64) -> Vec<usize> {
    kreuzberg::cache::filter_old_cache_entries(&cache_times, current_time, max_age_seconds)
}

#[pyfunction]
pub fn sort_cache_by_access_time(entries: Vec<(String, f64)>) -> Vec<String> {
    kreuzberg::cache::sort_cache_by_access_time(entries)
}

#[pyfunction]
pub fn fast_hash(data: &[u8]) -> u64 {
    kreuzberg::cache::fast_hash(data)
}

#[pyfunction]
pub fn validate_cache_key(key: &str) -> bool {
    kreuzberg::cache::validate_cache_key(key)
}

#[pyfunction]
pub fn is_cache_valid(cache_path: &str, max_age_days: f64) -> bool {
    kreuzberg::cache::is_cache_valid(cache_path, max_age_days)
}

#[pyfunction]
pub fn clear_cache_directory(cache_dir: &str) -> PyResult<(usize, f64)> {
    kreuzberg::cache::clear_cache_directory(cache_dir)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Cache clear error: {}", e)))
}

#[pyfunction]
pub fn batch_cleanup_caches(
    cache_dirs: Vec<String>,
    max_age_days: f64,
    max_size_mb: f64,
    min_free_space_mb: f64,
) -> PyResult<Vec<(usize, f64)>> {
    let cache_dirs_refs: Vec<&str> = cache_dirs.iter().map(|s| s.as_str()).collect();
    kreuzberg::cache::batch_cleanup_caches(&cache_dirs_refs, max_age_days, max_size_mb, min_free_space_mb)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Batch cleanup error: {}", e)))
}

pub fn register_cache_functions(m: &Bound<'_, pyo3::types::PyModule>) -> PyResult<()> {
    m.add_class::<CacheStatsDTO>()?;
    m.add_class::<GenericCacheDTO>()?;

    m.add_function(wrap_pyfunction!(generate_cache_key, m)?)?;
    m.add_function(wrap_pyfunction!(batch_generate_cache_keys, m)?)?;
    m.add_function(wrap_pyfunction!(get_available_disk_space, m)?)?;
    m.add_function(wrap_pyfunction!(get_cache_metadata, m)?)?;
    m.add_function(wrap_pyfunction!(cleanup_cache, m)?)?;
    m.add_function(wrap_pyfunction!(smart_cleanup_cache, m)?)?;
    m.add_function(wrap_pyfunction!(filter_old_cache_entries, m)?)?;
    m.add_function(wrap_pyfunction!(sort_cache_by_access_time, m)?)?;
    m.add_function(wrap_pyfunction!(fast_hash, m)?)?;
    m.add_function(wrap_pyfunction!(validate_cache_key, m)?)?;
    m.add_function(wrap_pyfunction!(is_cache_valid, m)?)?;
    m.add_function(wrap_pyfunction!(clear_cache_directory, m)?)?;
    m.add_function(wrap_pyfunction!(batch_cleanup_caches, m)?)?;

    Ok(())
}
