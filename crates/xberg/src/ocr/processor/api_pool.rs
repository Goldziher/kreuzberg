//! Bounded, keyed ownership pool for native Tesseract handles.

use std::ops::Deref;
use std::sync::{Arc, Condvar, Mutex};

use crate::ocr::error::OcrError;
use xberg_tesseract::TesseractAPI;

/// Recognition sessions this process runs at once.
///
/// Two limiters enforce it: the admission semaphore in
/// [`crate::ocr::tesseract_backend::TesseractBackend`], which holds async callers
/// back before they reach a blocking thread, and the capacity of the handle pool
/// below. They read the number here rather than each deriving it, and thread
/// pool initialization fixes it before either can be built, so no ordering
/// between the two can give them different limits.
///
/// `ConcurrencyConfig::max_concurrent_ocr` sets it; see
/// [`crate::core::config::concurrency::resolve_recognition_concurrency`] for the default.
pub(crate) fn tesseract_api_capacity() -> usize {
    // No cache here. `recognition_concurrency` is already a single atomic load of
    // a value only pool initialization writes, and a cache in front of it would
    // pin whatever this module happened to read first. ~keep
    crate::core::config::concurrency::recognition_concurrency()
}

#[derive(Clone, PartialEq, Eq)]
struct ApiKey {
    tessdata_path: String,
    language: String,
    oem: u8,
}

struct PoolState<K, V> {
    checked_out: usize,
    idle: Vec<(K, V)>,
}

struct ResourcePool<K, V> {
    capacity: usize,
    state: Mutex<PoolState<K, V>>,
    available: Condvar,
}

impl<K: PartialEq, V> ResourcePool<K, V> {
    fn new(capacity: usize) -> Arc<Self> {
        assert!(capacity > 0, "resource pool capacity must be positive");
        Arc::new(Self {
            capacity,
            state: Mutex::new(PoolState {
                checked_out: 0,
                idle: Vec::with_capacity(capacity),
            }),
            available: Condvar::new(),
        })
    }

    fn checkout<E, F>(self: &Arc<Self>, key: K, constructor: F) -> Result<ResourceLease<K, V>, E>
    where
        F: FnOnce() -> Result<V, E>,
    {
        let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            if let Some(index) = state.idle.iter().position(|(idle_key, _)| idle_key == &key) {
                let value = state.idle.swap_remove(index).1;
                state.checked_out += 1;
                return Ok(ResourceLease::new(Arc::clone(self), key, value));
            }

            let total = state.checked_out + state.idle.len();
            if total < self.capacity || !state.idle.is_empty() {
                let stale = state.idle.pop();
                state.checked_out += 1;
                drop(state);
                drop(stale);

                let reservation = ConstructionReservation::new(Arc::clone(self));
                let value = constructor()?;
                return Ok(reservation.finish(key, value));
            }

            state = self
                .available
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    fn release(&self, key: K, value: Option<V>) {
        let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        debug_assert!(state.checked_out > 0, "released resource was not checked out");
        state.checked_out -= 1;
        if let Some(value) = value {
            state.idle.push((key, value));
        }
        drop(state);
        self.available.notify_one();
    }

    #[cfg(test)]
    fn capacity(&self) -> usize {
        self.capacity
    }

    #[cfg(test)]
    fn counts(&self) -> (usize, usize) {
        let state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        (state.checked_out, state.idle.len())
    }
}

struct ConstructionReservation<K: PartialEq, V> {
    pool: Arc<ResourcePool<K, V>>,
    active: bool,
}

impl<K: PartialEq, V> ConstructionReservation<K, V> {
    fn new(pool: Arc<ResourcePool<K, V>>) -> Self {
        Self { pool, active: true }
    }

    fn finish(mut self, key: K, value: V) -> ResourceLease<K, V> {
        self.active = false;
        ResourceLease::new(Arc::clone(&self.pool), key, value)
    }
}

impl<K: PartialEq, V> Drop for ConstructionReservation<K, V> {
    fn drop(&mut self) {
        if self.active {
            let mut state = self
                .pool
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            debug_assert!(state.checked_out > 0, "construction reservation was not active");
            state.checked_out -= 1;
            drop(state);
            self.pool.available.notify_one();
        }
    }
}

struct ResourceLease<K: PartialEq, V> {
    pool: Arc<ResourcePool<K, V>>,
    key: Option<K>,
    value: Option<V>,
}

impl<K: PartialEq, V> ResourceLease<K, V> {
    fn new(pool: Arc<ResourcePool<K, V>>, key: K, value: V) -> Self {
        Self {
            pool,
            key: Some(key),
            value: Some(value),
        }
    }

    #[cfg(test)]
    fn discard(mut self) {
        self.release_with(|_| false);
    }

    fn release_with<F>(&mut self, reset: F)
    where
        F: FnOnce(&V) -> bool,
    {
        let Some(key) = self.key.take() else {
            return;
        };
        let value = self.value.take().expect("resource lease consumed prematurely");
        let mut reservation = ReleaseReservation::new(Arc::clone(&self.pool), key, value);
        let retain = reset(reservation.value());
        reservation.finish(retain);
    }
}

impl<K: PartialEq, V> Deref for ResourceLease<K, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.value.as_ref().expect("resource lease consumed prematurely")
    }
}

impl<K: PartialEq, V> Drop for ResourceLease<K, V> {
    fn drop(&mut self) {
        self.release_with(|_| true);
    }
}

struct ReleaseReservation<K: PartialEq, V> {
    pool: Arc<ResourcePool<K, V>>,
    key: Option<K>,
    value: Option<V>,
}

impl<K: PartialEq, V> ReleaseReservation<K, V> {
    fn new(pool: Arc<ResourcePool<K, V>>, key: K, value: V) -> Self {
        Self {
            pool,
            key: Some(key),
            value: Some(value),
        }
    }

    fn value(&self) -> &V {
        self.value.as_ref().expect("release reservation consumed prematurely")
    }

    fn finish(&mut self, retain: bool) {
        let key = self.key.take().expect("release reservation key consumed prematurely");
        let value = if retain {
            self.value.take()
        } else {
            drop(self.value.take());
            None
        };
        self.pool.release(key, value);
    }
}

impl<K: PartialEq, V> Drop for ReleaseReservation<K, V> {
    fn drop(&mut self) {
        if let Some(key) = self.key.take() {
            drop(self.value.take());
            self.pool.release(key, None);
        }
    }
}

pub(super) struct TesseractApiPool {
    resources: Arc<ResourcePool<ApiKey, TesseractAPI>>,
}

impl TesseractApiPool {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self {
            resources: ResourcePool::new(tesseract_api_capacity()),
        })
    }

    #[cfg(test)]
    pub(super) fn capacity(&self) -> usize {
        self.resources.capacity()
    }

    pub(super) fn checkout(
        self: &Arc<Self>,
        tessdata_path: &str,
        language: &str,
        oem: u8,
    ) -> Result<TesseractApiLease, OcrError> {
        crate::core::config_validation::validate_tesseract_oem(i32::from(oem))
            .map_err(|error| OcrError::InvalidConfiguration(error.to_string()))?;
        let key = ApiKey {
            tessdata_path: tessdata_path.to_string(),
            language: language.to_string(),
            oem,
        };
        let lease = self.resources.checkout(key, || {
            let api = TesseractAPI::new().map_err(|error| {
                OcrError::TesseractInitializationFailed(format!("Failed to allocate Tesseract engine: {error}"))
            })?;
            api.init_2(tessdata_path, language, i32::from(oem)).map_err(|error| {
                OcrError::TesseractInitializationFailed(format!(
                    "Failed to initialize language '{language}' with OCR engine mode {oem}: {error}"
                ))
            })?;
            api.clear().map_err(|error| {
                OcrError::ProcessingFailed(format!("Failed to clear newly initialized Tesseract API: {error}"))
            })?;
            Ok(api)
        })?;
        Ok(TesseractApiLease { inner: lease })
    }
}

pub(super) struct TesseractApiLease {
    inner: ResourceLease<ApiKey, TesseractAPI>,
}

impl Deref for TesseractApiLease {
    type Target = TesseractAPI;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Drop for TesseractApiLease {
    fn drop(&mut self) {
        self.inner.release_with(|api| match api.clear() {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    "Failed to clear Tesseract API before returning it to the idle pool; discarding handle"
                );
                false
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    #[test]
    fn should_keep_requested_engine_modes_separate_in_pool_keys() {
        let pool = ResourcePool::new(1);
        let key = |oem| ApiKey {
            tessdata_path: "models".into(),
            language: "eng".into(),
            oem,
        };
        drop(pool.checkout::<(), _>(key(1), || Ok(1)).unwrap());
        let reused = pool
            .checkout::<(), _>(key(1), || panic!("matching mode must reuse its handle"))
            .unwrap();
        assert_eq!(*reused, 1);
        drop(reused);
        let automatic = pool.checkout::<(), _>(key(3), || Ok(3)).unwrap();
        assert_eq!(*automatic, 3);
        assert_eq!(pool.counts(), (1, 0));
    }

    #[cfg(feature = "bundle-tessdata-eng")]
    fn bundled_tessdata() -> tempfile::TempDir {
        let directory = tempfile::tempdir().expect("temporary native model directory");
        std::fs::write(
            directory.path().join("eng.traineddata"),
            xberg_tesseract::bundled_eng_traineddata().expect("bundled English model"),
        )
        .expect("materialize bundled English model");
        directory
    }

    #[test]
    #[cfg(feature = "bundle-tessdata-eng")]
    fn should_release_native_capacity_after_unavailable_engine_initialization() {
        let directory = bundled_tessdata();
        let path = directory.path().to_str().expect("UTF-8 native model path");
        let pool = Arc::new(TesseractApiPool {
            resources: ResourcePool::new(1),
        });
        for unavailable in [0, 2] {
            let lstm = pool.checkout(path, "eng", 1).expect("LSTM mode initializes");
            assert_eq!(lstm.get_int_variable("tessedit_ocr_engine_mode").unwrap(), 1);
            drop(lstm);
            assert_eq!(pool.resources.counts(), (0, 1));
            assert!(pool.checkout(path, "eng", unavailable).is_err());
            assert_eq!(pool.resources.counts(), (0, 0));
        }
        let automatic = pool
            .checkout(path, "eng", 3)
            .expect("default mode initializes after failure");
        assert_eq!(automatic.get_int_variable("tessedit_ocr_engine_mode").unwrap(), 1);
        drop(automatic);
        assert_eq!(pool.resources.counts(), (0, 1));
    }

    #[test]
    fn should_reject_invalid_engine_mode_before_native_initialization() {
        let pool = Arc::new(TesseractApiPool {
            resources: ResourcePool::new(1),
        });
        assert!(matches!(
            pool.checkout("", "eng", 4),
            Err(OcrError::InvalidConfiguration(_))
        ));
        assert_eq!(pool.resources.counts(), (0, 0));
    }

    #[test]
    fn total_checked_out_and_idle_never_exceeds_capacity() {
        let pool = ResourcePool::new(4);
        let peak = Arc::new(AtomicUsize::new(0));
        let mut threads = Vec::new();
        for key in 0..12 {
            let pool = Arc::clone(&pool);
            let peak = Arc::clone(&peak);
            threads.push(thread::spawn(move || {
                let lease = pool.checkout::<(), _>(key, || Ok(key)).unwrap();
                let (checked_out, idle) = pool.counts();
                peak.fetch_max(checked_out + idle, Ordering::SeqCst);
                thread::yield_now();
                drop(lease);
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(peak.load(Ordering::SeqCst) <= 4);
        let (checked_out, idle) = pool.counts();
        assert_eq!(checked_out, 0);
        assert!(idle <= 4);
    }

    #[test]
    fn key_switch_evicts_an_idle_mismatch() {
        let pool = ResourcePool::new(1);
        drop(pool.checkout::<(), _>("eng", || Ok(1)).unwrap());
        let german = pool.checkout::<(), _>("deu", || Ok(2)).unwrap();
        assert_eq!(*german, 2);
        let (checked_out, idle) = pool.counts();
        assert_eq!((checked_out, idle), (1, 0));
    }

    #[test]
    fn constructor_failure_releases_reservation() {
        let pool = ResourcePool::<&str, usize>::new(1);
        assert!(pool.checkout("eng", || Err::<usize, _>("failed")).is_err());
        assert_eq!(pool.counts(), (0, 0));
        assert!(pool.checkout::<(), _>("eng", || Ok(1)).is_ok());
    }

    #[test]
    fn discarded_resource_releases_capacity_without_reuse() {
        let pool = ResourcePool::new(1);
        let lease = pool.checkout::<(), _>("eng", || Ok(1)).unwrap();
        lease.discard();
        assert_eq!(pool.counts(), (0, 0));

        let replacement = pool.checkout::<(), _>("eng", || Ok(2)).unwrap();
        assert_eq!(*replacement, 2);
    }

    #[test]
    fn constructor_panic_releases_reservation() {
        let pool = ResourcePool::<&str, usize>::new(1);
        let result = std::panic::catch_unwind({
            let pool = Arc::clone(&pool);
            move || {
                let _ = pool.checkout::<(), _>("eng", || panic!("constructor panic"));
            }
        });
        assert!(result.is_err());
        assert_eq!(pool.counts(), (0, 0));
        assert!(pool.checkout::<(), _>("eng", || Ok(1)).is_ok());
    }

    #[test]
    fn successful_release_reset_retains_resource() {
        let pool = ResourcePool::new(1);
        let reset_count = AtomicUsize::new(0);
        let mut lease = pool.checkout::<(), _>("eng", || Ok(1)).unwrap();
        lease.release_with(|value| {
            assert_eq!(*value, 1);
            reset_count.fetch_add(1, Ordering::SeqCst);
            true
        });

        assert_eq!(reset_count.load(Ordering::SeqCst), 1);
        assert_eq!(pool.counts(), (0, 1));
        let reused = pool.checkout::<(), _>("eng", || Ok(2)).unwrap();
        assert_eq!(*reused, 1);
    }

    #[test]
    fn failed_release_reset_discards_resource() {
        let pool = ResourcePool::new(1);
        let mut lease = pool.checkout::<(), _>("eng", || Ok(1)).unwrap();
        lease.release_with(|_| false);

        assert_eq!(pool.counts(), (0, 0));
        let replacement = pool.checkout::<(), _>("eng", || Ok(2)).unwrap();
        assert_eq!(*replacement, 2);
    }

    #[test]
    fn failed_release_reset_drops_resource_before_reopening_capacity() {
        struct DropCounter(Arc<AtomicUsize>);

        impl Drop for DropCounter {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let pool = ResourcePool::new(1);
        let drops = Arc::new(AtomicUsize::new(0));
        let mut lease = pool
            .checkout::<(), _>("eng", || Ok(DropCounter(Arc::clone(&drops))))
            .unwrap();
        lease.release_with(|_| false);

        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(pool.counts(), (0, 0));
    }

    #[test]
    fn release_reset_panic_discards_resource_and_releases_capacity() {
        let pool = ResourcePool::new(1);
        let result = std::panic::catch_unwind({
            let pool = Arc::clone(&pool);
            move || {
                let mut lease = pool.checkout::<(), _>("eng", || Ok(1)).unwrap();
                lease.release_with(|_| panic!("reset panic"));
            }
        });

        assert!(result.is_err());
        assert_eq!(pool.counts(), (0, 0));
        assert!(pool.checkout::<(), _>("eng", || Ok(2)).is_ok());
    }
}
