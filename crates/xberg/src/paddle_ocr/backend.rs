//! PaddleOCR backend implementation.
//!
//! This module implements the `OcrBackend` trait for PaddleOCR using ONNX Runtime.
//! PaddleOCR provides excellent recognition quality, especially for CJK languages.
//!
//! The backend maintains a pool of OCR engines keyed by script family.
//! Each family gets its own lazily-initialized engine with the appropriate
//! recognition model and character dictionary.

use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;
#[cfg(feature = "paddle-ocr-ort")]
use std::cell::RefCell;
use std::collections::VecDeque;
use std::panic::catch_unwind;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// Acceleration/execution-provider hook is ORT-only: the tract backend is CPU-only and
// never reads this thread-local (see `init_engine_tract`, which ignores acceleration
// rather than erroring).
#[cfg(feature = "paddle-ocr-ort")]
thread_local! {
    static PADDLE_TL_ACCEL: RefCell<Option<crate::core::config::acceleration::AccelerationConfig>> = const { RefCell::new(None) };
}

#[cfg(feature = "paddle-ocr-ort")]
fn paddle_accel_builder_fn(
    builder: ort::session::builder::SessionBuilder,
) -> std::result::Result<ort::session::builder::SessionBuilder, ort::Error> {
    let accel = PADDLE_TL_ACCEL.with(|cell| cell.borrow().clone());
    crate::ort_discovery::apply_execution_providers(builder, accel.as_ref())
}

use crate::Result;
use crate::core::config::OcrConfig;
use crate::ocr::conversion::{detailed_text_block_to_elements, elements_to_hocr_words};
use crate::ocr::table::drop_document_elements_claimed_by_tables;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::table_core::{reconstruct_table, table_to_markdown};
#[cfg(test)]
use crate::types::OcrElementLevel;
use crate::types::{ExtractedDocument, FormatMetadata, Metadata, OcrElement, OcrElementConfig, OcrMetadata, Table};

#[cfg(test)]
use super::config::DEFAULT_RECOGNITION_BATCH_SIZE;
use super::config::{MAX_RECOGNITION_BATCH_SIZE, MIN_RECOGNITION_BATCH_SIZE, PaddleInferenceBackend, PaddleOcrConfig};
use super::model_manager::{ModelManager, ResolvedRecModel, SharedModelPaths};
use super::{is_language_supported, language_to_script_family, map_language_code};

use xberg_paddle_ocr::PaddleOcrEngine;

type InitCell<T> = Arc<once_cell::sync::OnceCell<T>>;
type InitPool<T> = Mutex<AHashMap<String, InitCell<T>>>;
const MODEL_CONTEXT_CAPACITY: usize = 4;
const DOCUMENT_ORIENTATION_CACHE_CAPACITY: usize = 4;

struct BoundedInitPoolState<T> {
    entries: AHashMap<String, InitCell<T>>,
    recency: VecDeque<String>,
}

struct BoundedInitPool<T> {
    state: Mutex<BoundedInitPoolState<T>>,
    capacity: usize,
}

impl<T> BoundedInitPool<T> {
    fn new(capacity: usize) -> Self {
        Self {
            state: Mutex::new(BoundedInitPoolState {
                entries: AHashMap::with_capacity(capacity),
                recency: VecDeque::with_capacity(capacity),
            }),
            capacity,
        }
    }

    fn cell(&self, key: &str) -> std::result::Result<Option<InitCell<T>>, String> {
        let mut state = self.state.lock().map_err(|error| error.to_string())?;
        if let Some(cell) = state.entries.get(key).cloned() {
            state.recency.retain(|cached_key| cached_key != key);
            state.recency.push_back(key.to_string());
            return Ok(Some(cell));
        }

        if state.entries.len() == self.capacity {
            let evicted = state.recency.iter().find_map(|cached_key| {
                state
                    .entries
                    .get(cached_key)
                    .filter(|cell| Arc::strong_count(cell) == 1)
                    .map(|_| cached_key.clone())
            });
            let Some(evicted) = evicted else {
                return Ok(None);
            };
            state.recency.retain(|cached_key| cached_key != &evicted);
            state.entries.remove(&evicted);
        }

        let cell = Arc::new(once_cell::sync::OnceCell::new());
        state.entries.insert(key.to_string(), Arc::clone(&cell));
        state.recency.push_back(key.to_string());
        Ok(Some(cell))
    }

    #[cfg(test)]
    fn len(&self) -> std::result::Result<usize, String> {
        self.state
            .lock()
            .map(|state| state.entries.len())
            .map_err(|error| error.to_string())
    }
}

struct PaddleModelContext {
    cache_dir: PathBuf,
    model_manager: ModelManager,
    shared_paths: InitPool<SharedModelPaths>,
    engine_pool: InitPool<Arc<PaddleOcrEngine>>,
    doc_ori_detectors: BoundedInitPool<crate::doc_orientation::DocOrientationDetector>,
}

struct PaddleOcrExecution {
    context: Arc<PaddleModelContext>,
    config: Arc<PaddleOcrConfig>,
    acceleration: Option<crate::core::config::acceleration::AccelerationConfig>,
}

impl PaddleModelContext {
    fn new(cache_dir: PathBuf) -> Self {
        Self {
            model_manager: ModelManager::new(cache_dir.clone()),
            cache_dir,
            shared_paths: Mutex::new(AHashMap::new()),
            engine_pool: Mutex::new(AHashMap::new()),
            doc_ori_detectors: BoundedInitPool::new(DOCUMENT_ORIENTATION_CACHE_CAPACITY),
        }
    }
}

struct PaddleModelContextCache {
    entries: AHashMap<PathBuf, Arc<PaddleModelContext>>,
    recency: VecDeque<PathBuf>,
}

impl PaddleModelContextCache {
    fn new() -> Self {
        Self {
            entries: AHashMap::with_capacity(MODEL_CONTEXT_CAPACITY),
            recency: VecDeque::with_capacity(MODEL_CONTEXT_CAPACITY),
        }
    }

    fn context(&mut self, cache_dir: &Path) -> Option<Arc<PaddleModelContext>> {
        if let Some(context) = self.entries.get(cache_dir).cloned() {
            self.recency.retain(|path| path != cache_dir);
            self.recency.push_back(cache_dir.to_path_buf());
            return Some(context);
        }

        if self.entries.len() == MODEL_CONTEXT_CAPACITY {
            let evicted = self.recency.iter().find_map(|path| {
                self.entries
                    .get(path)
                    .filter(|context| Arc::strong_count(context) == 1)
                    .map(|_| path.clone())
            })?;
            self.recency.retain(|path| path != &evicted);
            self.entries.remove(&evicted);
        }

        let cache_dir = cache_dir.to_path_buf();
        let context = Arc::new(PaddleModelContext::new(cache_dir.clone()));
        self.entries.insert(cache_dir.clone(), Arc::clone(&context));
        self.recency.push_back(cache_dir);
        Some(context)
    }
}

#[cfg(feature = "paddle-ocr-ort")]
struct PaddleAccelerationGuard {
    previous: Option<crate::core::config::acceleration::AccelerationConfig>,
}

#[cfg(feature = "paddle-ocr-ort")]
impl PaddleAccelerationGuard {
    fn set(acceleration: Option<crate::core::config::acceleration::AccelerationConfig>) -> Self {
        let previous = PADDLE_TL_ACCEL.with(|cell| cell.replace(acceleration));
        Self { previous }
    }
}

#[cfg(feature = "paddle-ocr-ort")]
impl Drop for PaddleAccelerationGuard {
    fn drop(&mut self) {
        PADDLE_TL_ACCEL.with(|cell| {
            cell.replace(self.previous.take());
        });
    }
}

fn init_cell_for_key<T>(pool: &InitPool<T>, key: &str) -> std::result::Result<InitCell<T>, String> {
    let mut pool = pool.lock().map_err(|error| error.to_string())?;
    if let Some(cell) = pool.get(key) {
        return Ok(Arc::clone(cell));
    }

    let cell = Arc::new(once_cell::sync::OnceCell::new());
    pool.insert(key.to_string(), Arc::clone(&cell));
    Ok(cell)
}

async fn run_orientation_blocking<T, F>(operation: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|error| crate::XbergError::Plugin {
            message: format!("PaddleOCR document orientation task panicked: {error}"),
            plugin_name: "paddle-ocr".to_string(),
        })?
}

fn engine_pool_key(
    version: &str,
    tier: &str,
    model_key: &str,
    accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
    backend: PaddleInferenceBackend,
) -> String {
    let accel_key = acceleration_pool_key(accel);
    // ~keep A build can compile both `paddle-ocr-ort` and `paddle-ocr-tract` together (native
    // parity benchmarks); fold the resolved backend into the key so the pool never hands
    // back an engine constructed on the other engine.
    let backend_key = match backend {
        PaddleInferenceBackend::Ort => "ort",
        PaddleInferenceBackend::Tract => "tract",
    };
    format!("{version}/{tier}/{model_key}/{accel_key}/{backend_key}")
}

fn acceleration_pool_key(accel: Option<&crate::core::config::acceleration::AccelerationConfig>) -> String {
    use crate::core::config::acceleration::ExecutionProviderType;

    match accel.map(|config| (&config.provider, config.device_id)) {
        Some((ExecutionProviderType::Cuda, device_id)) => format!("cuda:{device_id}"),
        Some((ExecutionProviderType::TensorRt, device_id)) => format!("tensorrt:{device_id}"),
        Some((ExecutionProviderType::CoreMl, _)) => "coreml".to_string(),
        Some((ExecutionProviderType::Auto, _)) => "auto".to_string(),
        Some((ExecutionProviderType::Cpu, _)) | None => "cpu".to_string(),
    }
}

/// Intra-op thread count for PaddleOCR's shared inference session.
///
/// PaddleOCR keeps one session per model per pool key, and `OrtBackend` guards it with a
/// `Mutex` because `ort::Session::run` takes `&mut self` (ort 2.0.0-rc.13). Concurrent page
/// OCR therefore serializes on that mutex, so the session is always exactly one worker and can
/// safely claim the whole process budget — the same shape `layout/engine.rs::from_config` and
/// `inference/ort_backend.rs` already use. The previous hardcoded `1` left it single-core.
///
/// If layout detection and PaddleOCR are ever made to run concurrently (today PaddleOCR's
/// per-page `join_set` in `extractors/pdf/ocr.rs` only reaches this session after layout's own
/// pass), this must go through `resolve_batch_execution_plan` instead, or two full-budget
/// sessions could oversubscribe the process.
///
/// The `tract` backend takes the same `num_thread` parameter but ignores it (`TractBackend::load`).
fn paddle_inference_thread_count() -> usize {
    paddle_session_thread_budget(crate::core::config::concurrency::resolve_thread_budget(None))
}

/// Pure core of [`paddle_inference_thread_count`], split out so the policy is testable
/// without a live ORT session or host-CPU detection.
fn paddle_session_thread_budget(total_budget: usize) -> usize {
    total_budget.max(1)
}

use crate::ocr_metadata_keys::OCR_ORIENTATION_CONFIDENCE_METADATA_KEY as ORIENTATION_CONFIDENCE_METADATA_KEY;
const VERTICAL_TEXT_MIN_ASPECT_RATIO: f32 = 1.5;
const VERTICAL_COLUMN_MIN_OVERLAP_RATIO: f32 = 0.5;

/// Pixel tolerance for grouping word left-edges into the same table column
/// (see [`crate::table_core::detect_columns`]), used by the table-detection loop in
/// [`PaddleOcrBackend::process_image`].
///
/// Measured on the two fixtures this module's table tests use (real PaddleOCR word geometry from
/// `ocr_image.tiff`'s dense numeric stock table and page 1 of `ordinance_2197_scanned.pdf`'s
/// prose): widening this past 20px consistently merges *different* real columns together on the
/// stock table (its side-by-side sub-tables and mixed text/numeric columns sit closer together
/// than a single generic threshold can separate), which lowers the reconstructed grid's numeric
/// cell fraction and makes it *less* likely to clear `post_process_table`'s validation, not more.
/// 20px is kept unchanged from the pre-fix value; the discriminator below, not this threshold, is
/// what tells the resulting grid apart from a fabricated one.
const TABLE_COLUMN_ALIGNMENT_THRESHOLD_PX: u32 = 20;

#[derive(Debug)]
struct RotationOutcome {
    rotated_bytes: Option<Vec<u8>>,
    processed_width: u32,
    processed_height: u32,
    orientation: Option<crate::doc_orientation::OrientationResult>,
}

struct PaddlePageOcr {
    text: String,
    line_elements: Vec<OcrElement>,
    word_elements: Vec<OcrElement>,
    processed_width: u32,
    processed_height: u32,
}

impl RotationOutcome {
    fn unrotated(width: u32, height: u32) -> Self {
        Self {
            rotated_bytes: None,
            processed_width: width,
            processed_height: height,
            orientation: None,
        }
    }

    fn auto_rotated(&self) -> bool {
        self.rotated_bytes.is_some()
    }
}

fn rotate_for_detected_orientation(
    image: &image::RgbImage,
    orientation: crate::doc_orientation::OrientationResult,
) -> Result<RotationOutcome> {
    if orientation.degrees == 0 || orientation.confidence < crate::doc_orientation::MIN_CONFIDENCE {
        return Ok(RotationOutcome {
            rotated_bytes: None,
            processed_width: image.width(),
            processed_height: image.height(),
            orientation: Some(orientation),
        });
    }

    let rotated = match orientation.degrees {
        90 => image::imageops::rotate270(image),
        180 => image::imageops::rotate180(image),
        270 => image::imageops::rotate90(image),
        _ => {
            return Ok(RotationOutcome {
                rotated_bytes: None,
                processed_width: image.width(),
                processed_height: image.height(),
                orientation: Some(orientation),
            });
        }
    };
    let processed_width = rotated.width();
    let processed_height = rotated.height();
    let mut encoded = std::io::Cursor::new(Vec::new());
    rotated
        .write_to(&mut encoded, image::ImageFormat::Png)
        .map_err(|error| crate::XbergError::Ocr {
            message: format!("Failed to encode rotated PaddleOCR image: {error}"),
            source: None,
        })?;

    Ok(RotationOutcome {
        rotated_bytes: Some(encoded.into_inner()),
        processed_width,
        processed_height,
        orientation: Some(orientation),
    })
}

fn image_metadata(outcome: &RotationOutcome) -> AHashMap<Cow<'static, str>, serde_json::Value> {
    let mut additional = AHashMap::new();
    additional.insert(
        Cow::Borrowed(crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY),
        serde_json::Value::Number(outcome.processed_width.into()),
    );
    additional.insert(
        Cow::Borrowed(crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY),
        serde_json::Value::Number(outcome.processed_height.into()),
    );
    if let Some(orientation) = outcome.orientation {
        additional.insert(
            Cow::Borrowed(crate::ocr_metadata_keys::OCR_ORIENTATION_DEGREES_METADATA_KEY),
            serde_json::Value::Number(orientation.degrees.into()),
        );
        additional.insert(
            Cow::Borrowed(ORIENTATION_CONFIDENCE_METADATA_KEY),
            serde_json::json!(orientation.confidence),
        );
    }
    if outcome.auto_rotated() {
        additional.insert(
            Cow::Borrowed(crate::ocr_metadata_keys::OCR_AUTO_ROTATED_METADATA_KEY),
            serde_json::Value::Bool(true),
        );
    }
    additional
}

/// PaddleOCR backend using ONNX Runtime.
///
/// Maintains a pool of OCR engines keyed by script family. Each family has its own
/// recognition model and character dictionary, while detection and classification
/// models are shared across all families.
///
/// # Thread Safety
///
/// The backend is `Send + Sync` and can be used across threads safely via `Arc`.
/// Per-key initialization cells serialize each cold start. Initialized engines
/// can run OCR concurrently without holding the pool lock.
#[cfg_attr(alef, alef(skip))]
pub struct PaddleOcrBackend {
    config: Arc<PaddleOcrConfig>,
    model_contexts: Mutex<PaddleModelContextCache>,
    /// Hardware acceleration configuration for ORT sessions (set at construction).
    /// Per-request acceleration from `OcrConfig.acceleration` takes precedence.
    acceleration: Option<crate::core::config::acceleration::AccelerationConfig>,
}

/// Number of buckets in `DropScoreDiscardStats::histogram`, spanning the score range
/// `[0.0, 1.0]` in equal-width steps (bucket 0 is `[0.0, 0.2)`, ..., bucket 4 is `[0.8, 1.0]`).
const DROP_SCORE_HISTOGRAM_BUCKETS: usize = 5;

/// Instrumentation-only summary of what the `drop_score` filter in `PaddleOcrBackend::perform_ocr`
/// discards.
///
/// This exists to answer a question the removed-gate measurement documented on
/// `PaddleOcrBackend::process_image` (#675) could not: the surviving detections' mean
/// recognition confidence cannot see what `drop_score` (default 0.5) already discarded before
/// those detections ever became elements. Recording it does not change which detections
/// survive `perform_ocr`'s filter — `record` is only ever called for blocks the filter has
/// already decided to drop.
#[derive(Debug, Default, Clone, Copy)]
struct DropScoreDiscardStats {
    discarded_count: usize,
    /// Subset of `discarded_count` whose `text_score` was NaN (excluded from
    /// sum/min/max/histogram since NaN has no ordering).
    nan_count: usize,
    sum: f64,
    min: f32,
    max: f32,
    histogram: [u32; DROP_SCORE_HISTOGRAM_BUCKETS],
}

impl DropScoreDiscardStats {
    /// Record one discarded detection's `text_score`. Must only be called for scores the live
    /// filter has already excluded, and must not itself decide inclusion/exclusion.
    fn record(&mut self, score: f32) {
        self.discarded_count += 1;
        if score.is_nan() {
            self.nan_count += 1;
            return;
        }
        let scored_before = self.discarded_count - self.nan_count == 1;
        self.min = if scored_before { score } else { self.min.min(score) };
        self.max = if scored_before { score } else { self.max.max(score) };
        self.sum += f64::from(score);

        let bucket = ((score.clamp(0.0, 1.0) * DROP_SCORE_HISTOGRAM_BUCKETS as f32) as usize)
            .min(DROP_SCORE_HISTOGRAM_BUCKETS - 1);
        self.histogram[bucket] += 1;
    }

    /// Count of discarded, non-NaN scores the mean/min/max/histogram are computed over.
    fn scored_count(&self) -> usize {
        self.discarded_count - self.nan_count
    }

    /// Mean of discarded, non-NaN scores; `0.0` when there is nothing to average (this is a
    /// "no discards to report" reading, not a measured score of zero).
    fn mean(&self) -> f64 {
        let scored_count = self.scored_count();
        if scored_count == 0 {
            0.0
        } else {
            self.sum / scored_count as f64
        }
    }
}

/// The exact `drop_score` filter predicate from `PaddleOcrBackend::perform_ocr`, factored out
/// so the boundary case (`text_score == drop_score`) is unit-testable without exercising the
/// full detection pipeline. `perform_ocr`'s filter calls this and only this to decide
/// inclusion; `DropScoreDiscardStats::record` never does.
fn passes_drop_score(text_score: f32, drop_score: f32) -> bool {
    text_score >= drop_score && !text_score.is_nan()
}

/// Mean/min/max summary of a `box_score` population (DBNet's detection-region confidence),
/// computed alongside `DropScoreDiscardStats` in `perform_ocr` so it can be compared against
/// `text_score` (CRNN's recognition confidence) page by page.
///
/// This distinction matters because `drop_score` (#675) filters on `text_score`, and
/// `text_score` is the exact signal `PaddleOcrBackend::process_image`'s doc comment already
/// measured as unable to separate plat/drawing noise from real text: on
/// `ordinance_2197_scanned.pdf` every one of the 16 pages' surviving `text_score`-derived mean
/// recognition confidence landed between 0.79 and 0.99, good and bad pages alike, and the
/// worst-case page-level minimum for a *bad* page (0.9546) exceeds several *good* pages' means.
/// `box_score` is a different signal — the detection region's own confidence, gated separately
/// by `det_db_box_thresh` before recognition ever runs — and has never been measured this way.
/// Recording it here changes nothing about which blocks survive; see the identical caveat on
/// `DropScoreDiscardStats`.
#[derive(Debug, Default, Clone, Copy)]
struct BoxScoreSummary {
    count: usize,
    sum: f64,
    min: f32,
    max: f32,
}

impl BoxScoreSummary {
    /// Record one block's `box_score`. NaN scores are counted separately and excluded from
    /// sum/min/max, matching `DropScoreDiscardStats`'s treatment of NaN `text_score`.
    fn record(&mut self, score: f32) {
        if score.is_nan() {
            return;
        }
        let first = self.count == 0;
        self.count += 1;
        self.min = if first { score } else { self.min.min(score) };
        self.max = if first { score } else { self.max.max(score) };
        self.sum += f64::from(score);
    }

    /// Mean of recorded, non-NaN scores; `0.0` when nothing has been recorded (a "no data"
    /// reading, not a measured score of zero).
    fn mean(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }
}

/// Result of clustering, reconstructing, and validating OCR table candidates from a
/// page's word boxes. Factored out of `process_image` so the table-building logic is
/// unit-testable without spinning up a PaddleOCR engine (see `build_ocr_tables_from_words`).
struct BuiltOcrTables {
    tables: Vec<Table>,
    table_count: u32,
    table_rows: Option<u32>,
    table_cols: Option<u32>,
    rejected_candidates: u32,
}

impl PaddleOcrBackend {
    /// Create a new PaddleOCR backend with default configuration.
    pub fn new() -> Result<Self> {
        Self::with_config(PaddleOcrConfig::default())
    }

    /// Create a new PaddleOCR backend with custom configuration.
    pub fn with_config(config: PaddleOcrConfig) -> Result<Self> {
        config.checked_cache_dir()?;
        Ok(Self {
            config: Arc::new(config),
            model_contexts: Mutex::new(PaddleModelContextCache::new()),
            acceleration: None,
        })
    }

    fn model_context(&self, cache_dir: &Path) -> Result<Arc<PaddleModelContext>> {
        let mut contexts = self.model_contexts.lock().map_err(|error| crate::XbergError::Plugin {
            message: format!("Failed to acquire PaddleOCR model context cache: {error}"),
            plugin_name: "paddle-ocr".to_string(),
        })?;
        contexts.context(cache_dir).ok_or_else(|| crate::XbergError::Ocr {
            message: format!(
                "PaddleOCR model context capacity ({MODEL_CONTEXT_CAPACITY} active cache roots) is exhausted; \
                 retry after an active OCR request completes"
            ),
            source: None,
        })
    }

    /// Set hardware acceleration for ORT sessions.
    pub fn with_acceleration(mut self, accel: crate::core::config::acceleration::AccelerationConfig) -> Self {
        self.acceleration = Some(accel);
        self
    }

    /// Get the current acceleration configuration, if any.
    pub fn acceleration(&self) -> Option<&crate::core::config::acceleration::AccelerationConfig> {
        self.acceleration.as_ref()
    }

    /// Resolve effective acceleration: per-request from OcrConfig takes precedence
    /// over the backend-level default.
    fn resolve_acceleration(
        &self,
        request_accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
    ) -> Option<crate::core::config::acceleration::AccelerationConfig> {
        request_accel.cloned().or_else(|| self.acceleration.clone())
    }

    /// Get or initialize shared model paths (det + cls) for the given config's
    /// version and tier.
    ///
    /// Keyed by `"{model_version}/{model_tier}"` so a per-request override
    /// (`OcrConfig.paddle_ocr_config`) resolves a detection model matching its
    /// recognition model rather than the backend default (issue #1279).
    fn get_or_init_shared_paths(
        model_manager: &ModelManager,
        shared_paths: &InitPool<SharedModelPaths>,
        config: &PaddleOcrConfig,
    ) -> Result<SharedModelPaths> {
        let key = format!("{}/{}", config.model_version, config.model_tier);
        let init_cell = init_cell_for_key(shared_paths, &key).map_err(|error| crate::XbergError::Plugin {
            message: format!("Failed to acquire shared paths lock: {error}"),
            plugin_name: "paddle-ocr".to_string(),
        })?;
        init_cell
            .get_or_try_init(|| model_manager.ensure_shared_models_versioned(&config.model_version, &config.model_tier))
            .cloned()
    }

    /// Get or create an OCR engine for the given script family.
    ///
    /// The engine pool is keyed by a composite `"{version}/{tier}/{model_key}/{accel}"` string.
    /// This ensures that:
    /// - Multiple families sharing the same unified model reuse one engine
    /// - Different tiers get different engines (different det model)
    /// - Different acceleration configs get separate engines (CPU vs CUDA)
    async fn get_or_init_engine_for_family(
        context: Arc<PaddleModelContext>,
        family: &str,
        config: Arc<PaddleOcrConfig>,
        accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
    ) -> Result<Arc<PaddleOcrEngine>> {
        let family = family.to_string();
        let accel = accel.cloned();

        // Model I/O, same-key waits, and ORT session construction must stay off async workers. ~keep
        tokio::task::spawn_blocking(move || {
            Self::get_or_init_engine_for_family_blocking(
                &context.model_manager,
                &context.shared_paths,
                &context.engine_pool,
                &family,
                &config,
                accel.as_ref(),
            )
        })
        .await
        .map_err(|error| crate::XbergError::Plugin {
            message: format!("PaddleOCR initialization task panicked: {error}"),
            plugin_name: "paddle-ocr".to_string(),
        })?
    }

    fn get_or_init_engine_for_family_blocking(
        model_manager: &ModelManager,
        shared_paths: &InitPool<SharedModelPaths>,
        engine_pool: &InitPool<Arc<PaddleOcrEngine>>,
        family: &str,
        config: &PaddleOcrConfig,
        accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
    ) -> Result<Arc<PaddleOcrEngine>> {
        let tier = &config.model_tier;
        let version = &config.model_version;
        let backend = Self::effective_backend(config)?;
        let resolved = model_manager.resolve_rec_model_versioned(version, family, tier)?;
        let pool_key = engine_pool_key(version, tier, &resolved.model_key, accel, backend);

        let init_cell = init_cell_for_key(engine_pool, &pool_key).map_err(|error| crate::XbergError::Plugin {
            message: format!("Failed to acquire engine pool lock: {error}"),
            plugin_name: "paddle-ocr".to_string(),
        })?;
        let engine = init_cell.get_or_try_init(|| -> Result<Arc<PaddleOcrEngine>> {
            let shared = Self::get_or_init_shared_paths(model_manager, shared_paths, config)?;
            Self::initialize_engine(family, tier, &resolved, &shared, accel.cloned(), backend)
        })?;

        Ok(Arc::clone(engine))
    }

    /// Resolve which inference engine to use, validating an explicit
    /// `config.inference_backend` request against the compiled features.
    ///
    /// Mirrors `sceptre_ocr`'s `validate_inference_backend` free function in
    /// `crates/xberg/src/sceptre_ocr/mod.rs`: an unset request resolves to the
    /// compiled default (`ort` when `paddle-ocr-ort` is compiled in, else `tract`,
    /// preserving today's behavior exactly); an explicit choice whose feature is not
    /// compiled in is a clear configuration error rather than a silent fallback.
    fn effective_backend(config: &PaddleOcrConfig) -> Result<PaddleInferenceBackend> {
        let requested = config.inference_backend.unwrap_or_else(Self::default_inference_backend);
        match requested {
            PaddleInferenceBackend::Ort if cfg!(feature = "paddle-ocr-ort") => Ok(requested),
            PaddleInferenceBackend::Tract if cfg!(feature = "paddle-ocr-tract") => Ok(requested),
            PaddleInferenceBackend::Ort => Err(crate::XbergError::Ocr {
                message: "PaddleOCR ORT inference is unavailable in this build; enable `paddle-ocr-ort`".to_string(),
                source: None,
            }),
            PaddleInferenceBackend::Tract => Err(crate::XbergError::Ocr {
                message: "PaddleOCR tract inference is unavailable in this build; enable `paddle-ocr-tract`"
                    .to_string(),
                source: None,
            }),
        }
    }

    /// The compile-time default backend when `config.inference_backend` is unset:
    /// `ort` when `paddle-ocr-ort` is compiled in (preserving today's behavior
    /// exactly), otherwise `tract`.
    fn default_inference_backend() -> PaddleInferenceBackend {
        #[cfg(feature = "paddle-ocr-ort")]
        {
            PaddleInferenceBackend::Ort
        }
        #[cfg(not(feature = "paddle-ocr-ort"))]
        {
            PaddleInferenceBackend::Tract
        }
    }

    fn initialize_engine(
        family: &str,
        tier: &str,
        resolved: &ResolvedRecModel,
        shared: &SharedModelPaths,
        accel: Option<crate::core::config::acceleration::AccelerationConfig>,
        backend: PaddleInferenceBackend,
    ) -> Result<Arc<PaddleOcrEngine>> {
        tracing::info!(family, model_key = %resolved.model_key, tier, ?backend, "Initializing PaddleOCR engine");

        let mut ocr_engine = PaddleOcrEngine::new();
        let det_model_path = Self::find_onnx_model(&shared.det_model)?;
        let cls_model_path = Self::find_onnx_model(&shared.cls_model)?;
        let rec_model_path = Self::find_onnx_model(&resolved.model_dir)?;
        let det_model_path = det_model_path.to_str().ok_or_else(|| crate::XbergError::Ocr {
            message: "Invalid detection model path".to_string(),
            source: None,
        })?;
        let cls_model_path = cls_model_path.to_str().ok_or_else(|| crate::XbergError::Ocr {
            message: "Invalid classification model path".to_string(),
            source: None,
        })?;
        let rec_model_path = rec_model_path.to_str().ok_or_else(|| crate::XbergError::Ocr {
            message: "Invalid recognition model path".to_string(),
            source: None,
        })?;
        let dict_path = resolved.dict_file.to_str().ok_or_else(|| crate::XbergError::Ocr {
            message: "Invalid dictionary file path".to_string(),
            source: None,
        })?;

        match backend {
            #[cfg(feature = "paddle-ocr-ort")]
            PaddleInferenceBackend::Ort => Self::init_engine_ort(
                &mut ocr_engine,
                det_model_path,
                cls_model_path,
                rec_model_path,
                dict_path,
                accel,
            )
            .map_err(|error| crate::XbergError::Ocr {
                message: format!(
                    "Failed to initialize PaddleOCR models for {family} ({}) on the ort backend: {error}",
                    resolved.model_key
                ),
                source: None,
            })?,
            #[cfg(feature = "paddle-ocr-tract")]
            PaddleInferenceBackend::Tract => Self::init_engine_tract(
                &mut ocr_engine,
                det_model_path,
                cls_model_path,
                rec_model_path,
                dict_path,
                accel.as_ref(),
            )
            .map_err(|error| crate::XbergError::Ocr {
                message: format!(
                    "Failed to initialize PaddleOCR models for {family} ({}) on the tract backend: {error}",
                    resolved.model_key
                ),
                source: None,
            })?,
            // Unreachable in practice: `effective_backend` already rejects a backend whose
            // feature is not compiled in before `initialize_engine` is ever called. This arm
            // only exists so the match stays exhaustive in a single-engine build, mirroring
            // `xberg_paddle_ocr::inference::load_backend`'s catch-all. ~keep
            #[allow(unreachable_patterns)]
            other => {
                return Err(crate::XbergError::Ocr {
                    message: format!(
                        "PaddleOCR backend {other:?} is not compiled in (enable the matching cargo feature)"
                    ),
                    source: None,
                });
            }
        }

        tracing::info!(family, model_key = %resolved.model_key, "PaddleOCR engine initialized successfully");
        Ok(Arc::new(ocr_engine))
    }

    /// Load models onto the native ONNX Runtime backend, applying the acceleration/EP
    /// hook (see `paddle_accel_builder_fn`) when one is configured.
    #[cfg(feature = "paddle-ocr-ort")]
    fn init_engine_ort(
        ocr_engine: &mut PaddleOcrEngine,
        det_model_path: &str,
        cls_model_path: &str,
        rec_model_path: &str,
        dict_path: &str,
        accel: Option<crate::core::config::acceleration::AccelerationConfig>,
    ) -> std::result::Result<(), xberg_paddle_ocr::OcrError> {
        let _acceleration_guard = PaddleAccelerationGuard::set(accel);
        crate::ort_discovery::ensure_ort_available();
        crate::ort_discovery::ort_environment()?;

        let builder_fn: Option<
            fn(
                ort::session::builder::SessionBuilder,
            ) -> std::result::Result<ort::session::builder::SessionBuilder, ort::Error>,
        > = if PADDLE_TL_ACCEL.with(|cell| cell.borrow().is_some()) {
            Some(paddle_accel_builder_fn)
        } else {
            None
        };

        ocr_engine.init_models_with_dict_custom(
            det_model_path,
            cls_model_path,
            rec_model_path,
            dict_path,
            paddle_inference_thread_count(),
            builder_fn,
        )
    }

    /// Load models onto the pure-Rust tract backend. Tract is CPU-only, so an EP
    /// acceleration request is logged and ignored rather than treated as an error —
    /// mirroring `sceptre_ocr::validate_acceleration`'s CPU-only stance for tract targets.
    ///
    /// Detection needs no shape configuration here. tract cannot shape-infer DBNet with a
    /// symbolic input H/W (the `Resize`-upsampled extent fails to unify against the FPN skip
    /// connection at `Concat`; see the Phase 0 spike note in
    /// `docs-site/src/content/docs/concepts/tract-inference.md`), so `xberg-paddle-ocr` builds
    /// a DBNet plan per page extent and caches it. Pinning to the page's own extent — rather
    /// than padding every page into one fixed canvas — is what keeps tract detection numerically
    /// equal to ORT: DBNet's squeeze-and-excitation blocks average over the whole input, so a
    /// padded canvas shifts the probability map across the entire page. `AngleNet`/`CrnnNet`
    /// stay unpinned; their graphs carry no dimension tract cannot resolve.
    #[cfg(feature = "paddle-ocr-tract")]
    fn init_engine_tract(
        ocr_engine: &mut PaddleOcrEngine,
        det_model_path: &str,
        cls_model_path: &str,
        rec_model_path: &str,
        dict_path: &str,
        accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
    ) -> std::result::Result<(), xberg_paddle_ocr::OcrError> {
        if accel.is_some() {
            tracing::debug!(
                "PaddleOCR tract backend is CPU-only; ignoring the requested hardware acceleration provider"
            );
        }

        // Selected explicitly rather than via `init_models_with_dict`: that entry point takes
        // `xberg_paddle_ocr`'s compile-time default engine, which prefers `ort` whenever
        // `paddle-ocr-ort` is also compiled in. In a dual-engine build (native cross-engine
        // parity) the default would hand this function an `ort` engine, so a caller asking for
        // tract would silently get ORT.
        ocr_engine.init_models_with_dict_on(
            xberg_paddle_ocr::InferenceBackend::Tract,
            det_model_path,
            cls_model_path,
            rec_model_path,
            dict_path,
            paddle_inference_thread_count(),
        )
    }

    /// Find the ONNX model file within a model directory.
    fn find_onnx_model(model_dir: &std::path::Path) -> Result<std::path::PathBuf> {
        if model_dir.is_file() && model_dir.extension().is_some_and(|extension| extension == "onnx") {
            return Ok(model_dir.to_path_buf());
        }
        if !model_dir.exists() {
            return Err(crate::XbergError::Ocr {
                message: format!("Model directory does not exist: {:?}", model_dir),
                source: None,
            });
        }

        let standard_path = model_dir.join("model.onnx");
        if standard_path.exists() {
            return Ok(standard_path);
        }

        let entries = std::fs::read_dir(model_dir).map_err(|e| crate::XbergError::Ocr {
            message: format!("Failed to read model directory {:?}: {}", model_dir, e),
            source: None,
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| crate::XbergError::Ocr {
                message: format!("Failed to read directory entry: {}", e),
                source: None,
            })?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "onnx") {
                return Ok(path);
            }
        }

        Err(crate::XbergError::Ocr {
            message: format!("No ONNX model file found in directory: {:?}", model_dir),
            source: None,
        })
    }

    /// Detect document orientation and rotate if needed.
    ///
    async fn detect_and_rotate(
        image: image::RgbImage,
        context: Arc<PaddleModelContext>,
        acceleration: Option<crate::core::config::acceleration::AccelerationConfig>,
    ) -> Result<RotationOutcome> {
        run_orientation_blocking(move || {
            let detector_cell = context
                .doc_ori_detectors
                .cell(&acceleration_pool_key(acceleration.as_ref()))
                .map_err(|error| crate::XbergError::Plugin {
                    message: format!("Failed to acquire PaddleOCR orientation detector cache: {error}"),
                    plugin_name: "paddle-ocr".to_string(),
                })?
                .ok_or_else(|| crate::XbergError::Ocr {
                    message: format!(
                        "PaddleOCR document orientation cache capacity ({DOCUMENT_ORIENTATION_CACHE_CAPACITY} active \
                         acceleration configurations) is exhausted; retry after an active request completes"
                    ),
                    source: None,
                })?;
            let detector = detector_cell.get_or_try_init(|| {
                Ok::<_, crate::XbergError>(crate::doc_orientation::DocOrientationDetector::with_acceleration(
                    context.cache_dir.clone(),
                    acceleration,
                ))
            })?;

            let orientation = detector.detect(&image)?;
            tracing::debug!(
                degrees = orientation.degrees,
                confidence = orientation.confidence,
                "Document orientation detected for PaddleOCR"
            );
            rotate_for_detected_orientation(&image, orientation)
        })
        .await
    }

    /// Perform OCR on image bytes using the appropriate script family engine.
    async fn do_ocr(
        image_bytes: &[u8],
        language: &str,
        execution: PaddleOcrExecution,
        page_rotation_degrees: u32,
        security_limits: &crate::extractors::security::SecurityLimits,
    ) -> Result<PaddlePageOcr> {
        let PaddleOcrExecution {
            context,
            config,
            acceleration,
        } = execution;
        let family = language_to_script_family(language);
        let engine =
            Self::get_or_init_engine_for_family(context, family, Arc::clone(&config), acceleration.as_ref()).await?;

        let image_bytes_owned = image_bytes.to_vec();
        let security_limits = security_limits.clone();

        let (mut text_blocks, processed_width, processed_height) = tokio::task::spawn_blocking(move || {
            catch_unwind(std::panic::AssertUnwindSafe(|| {
                Self::perform_ocr(&image_bytes_owned, &engine, &config, &security_limits)
            }))
            .map_err(|_| crate::XbergError::Plugin {
                message: "PaddleOCR inference panicked (ONNX Runtime error)".to_string(),
                plugin_name: "paddle-ocr".to_string(),
            })?
        })
        .await
        .map_err(|e| crate::XbergError::Plugin {
            message: format!("PaddleOCR task panicked: {}", e),
            plugin_name: "paddle-ocr".to_string(),
        })??;

        Self::reorder_blocks_for_page_rotation(
            &mut text_blocks,
            page_rotation_degrees,
            processed_width,
            processed_height,
        );
        let vertical_cjk = Self::sort_vertical_cjk_blocks(&mut text_blocks, language);

        let mut line_elements = Vec::with_capacity(text_blocks.len());
        let mut word_elements = Vec::new();
        for block in &text_blocks {
            if let Some(group) = detailed_text_block_to_elements(block, 1)? {
                line_elements.push(group.line);
                word_elements.extend(group.words);
            }
        }

        let text = Self::assemble_block_text(&text_blocks, vertical_cjk);

        Ok(PaddlePageOcr {
            text,
            line_elements,
            word_elements,
            processed_width,
            processed_height,
        })
    }

    fn sort_vertical_cjk_blocks(blocks: &mut [xberg_paddle_ocr::DetailedTextBlock], language: &str) -> bool {
        if !matches!(language, "ch" | "chinese_cht" | "japan" | "korean") || blocks.is_empty() {
            return false;
        }

        let vertical_count = blocks
            .iter()
            .filter(|block| Self::is_vertical_text_block(block))
            .count();
        // Mixed pages need layout-region ordering; never move or fuse their horizontal content. ~keep
        if vertical_count != blocks.len() {
            return false;
        }

        let mut bounds = blocks.iter().filter_map(Self::block_bounds).collect::<Vec<_>>();
        bounds.sort_by_key(|(min_x, _, max_x, _)| std::cmp::Reverse(u64::from(*min_x) + u64::from(*max_x)));

        let mut columns: Vec<(u32, u32)> = Vec::new();
        for (min_x, _, max_x, _) in bounds {
            if let Some((column_min, column_max)) = columns.iter_mut().find(|(column_min, column_max)| {
                Self::ranges_share_vertical_column(min_x, max_x, *column_min, *column_max)
            }) {
                *column_min = (*column_min).min(min_x);
                *column_max = (*column_max).max(max_x);
            } else {
                columns.push((min_x, max_x));
            }
        }

        // Traditional CJK columns read right-to-left; fragments within a column read top-to-bottom. ~keep
        blocks.sort_by_key(|block| {
            let Some((min_x, min_y, max_x, _)) = Self::block_bounds(block) else {
                return (usize::MAX, u32::MAX);
            };
            let column = columns
                .iter()
                .position(|(column_min, column_max)| {
                    Self::ranges_share_vertical_column(min_x, max_x, *column_min, *column_max)
                })
                .unwrap_or(usize::MAX);
            (column, min_y)
        });
        true
    }

    fn assemble_block_text(blocks: &[xberg_paddle_ocr::DetailedTextBlock], compact_vertical: bool) -> String {
        // Detector blocks are visual lines, not paragraphs; Markdown keeps single newlines inside a paragraph. ~keep
        blocks
            .iter()
            .map(|block| block.block.text.as_str())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(if compact_vertical { "" } else { "\n" })
    }

    fn ranges_share_vertical_column(left_min: u32, left_max: u32, right_min: u32, right_max: u32) -> bool {
        let overlap = left_max.min(right_max).saturating_sub(left_min.max(right_min));
        let narrower_width = left_max
            .saturating_sub(left_min)
            .min(right_max.saturating_sub(right_min));
        narrower_width > 0 && overlap as f32 / narrower_width as f32 >= VERTICAL_COLUMN_MIN_OVERLAP_RATIO
    }

    /// Read the page-rotation hint `extractors::pdf::ocr` injects into
    /// `OcrConfig.backend_options` before dispatching a page to this backend (#640).
    ///
    /// This is the PDF page's own `/Rotate` value, not PaddleOCR's own `auto_rotate`
    /// orientation-detection correction (a separate, independent axis handled by
    /// `detect_and_rotate`/`rotate_for_detected_orientation`). Absent or malformed
    /// values are treated as "no rotation" so callers that don't set the hint (direct
    /// image OCR, other backends' tests reusing this config, etc.) are unaffected.
    fn page_rotation_degrees_from_backend_options(config: &OcrConfig) -> u32 {
        config
            .backend_options
            .as_ref()
            .and_then(|opts| opts.get("page_rotation_degrees"))
            .and_then(serde_json::Value::as_u64)
            .map(|degrees| (degrees % 360) as u32)
            .unwrap_or(0)
    }

    /// Read a per-call `SecurityLimits` override out of `backend_options`, if one is present
    /// and parses. Returns `None` (not a default) when there is no override, so callers can
    /// tell "no override" apart from "override says use the default" and fall through to
    /// [`OcrConfig::security_limits`] instead of masking it. ~keep
    fn security_limits_override_from_backend_options(
        config: &OcrConfig,
    ) -> Option<crate::extractors::security::SecurityLimits> {
        config
            .backend_options
            .as_ref()
            .and_then(|opts| opts.get("security_limits"))
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }

    /// Resolve the `SecurityLimits` to apply when decoding an image for this call (GH#1554).
    ///
    /// Precedence, highest first:
    /// 1. An explicit `backend_options["security_limits"]` override for this one call — the
    ///    per-call extension point `security_limits_override_from_backend_options` reads,
    ///    documented alongside `page_rotation_degrees` above.
    /// 2. [`OcrConfig::security_limits`], injected at runtime from the caller's
    ///    `ExtractionConfig::security_limits` (mirrors `OcrConfig::acceleration`) — this is
    ///    what makes a configured, possibly higher, limit actually reach this backend.
    /// 3. `SecurityLimits::default()`, never an unbounded/disabled check.
    fn resolve_security_limits(config: &OcrConfig) -> crate::extractors::security::SecurityLimits {
        Self::security_limits_override_from_backend_options(config)
            .or_else(|| config.security_limits.clone())
            .unwrap_or_default()
    }

    /// Compose the PDF page's `/Rotate` hint (`page_rotation_degrees_from_backend_options`)
    /// with any rotation `detect_and_rotate` already applied to the raster, yielding the
    /// residual rotation `reorder_blocks_for_page_rotation` must correct for.
    ///
    /// `reorder_blocks_for_page_rotation`'s premise is that its blocks are in the *raw*
    /// MediaBox raster frame the `/Rotate` hint describes. When `OcrConfig.auto_rotate` is
    /// enabled, `process_image` may hand `do_ocr` an already-rotated raster instead — in
    /// which case reordering by the raw `/Rotate` value alone double-corrects: once by the
    /// pixel rotation `detect_and_rotate` performed, once by the reorder itself.
    ///
    /// `auto_rotate_applied_degrees` is `Some(orientation.degrees)` when auto-rotation
    /// actually rotated the raster (`RotationOutcome::auto_rotated()` is `true`), or `None`
    /// otherwise — whether because `auto_rotate` is disabled, orientation detection found
    /// nothing to correct (`degrees == 0` or confidence below threshold), or detection
    /// itself failed and fell back to `RotationOutcome::unrotated`. In every `None` case the
    /// raster handed to OCR is still the raw MediaBox raster, so the residual rotation is
    /// `page_rotation_degrees` unchanged — this is what keeps `auto_rotate: false` (and the
    /// detection-failure fallback) byte-identical to the pre-existing behavior.
    ///
    /// When a rotation *was* applied, it was a pixel rotation of `(360 - degrees) % 360`
    /// (see `rotate_for_detected_orientation`'s `rotate90`/`rotate180`/`rotate270` match),
    /// composed onto the raw raster's own `page_rotation_degrees` rotation-away-from-upright.
    /// The new raster's residual rotation-away-from-upright is therefore
    /// `(page_rotation_degrees + (360 - degrees)) % 360`, which is `0` exactly when
    /// orientation detection correctly identified the same rotation the `/Rotate` hint
    /// already describes.
    fn residual_rotation_for_reorder(page_rotation_degrees: u32, auto_rotate_applied_degrees: Option<u32>) -> u32 {
        let page = page_rotation_degrees % 360;
        match auto_rotate_applied_degrees {
            Some(applied_degrees) => (page + 360 - applied_degrees % 360) % 360,
            None => page,
        }
    }

    /// Map a point from the OCR raster's coordinate space back to true reading-order
    /// (display) coordinates, given the correction angle that produced that raster.
    ///
    /// Mirrors `crate::pdf::render::ocr_page_correction_degrees`'s quarter-turn
    /// convention (duplicated as plain arithmetic below so this module has no
    /// dependency on the `pdf` feature): the raster this backend receives was
    /// obtained by rotating the displayed page by `correction_degrees`, so this
    /// applies the same inverse quarter-turn arithmetic used elsewhere in the OCR
    /// pipeline (`extractors::pdf::ocr::inverse_rotate_ocr_point`) to undo it,
    /// without touching any stored bbox — only used to compute a sort key.
    fn reading_order_point(
        x: f32,
        y: f32,
        correction_degrees: u32,
        raster_width: f32,
        raster_height: f32,
    ) -> (f32, f32) {
        match correction_degrees {
            90 => (y, raster_height - x),
            180 => (raster_width - x, raster_height - y),
            270 => (raster_width - y, x),
            _ => (x, y),
        }
    }

    /// Reorder detector blocks into true reading order when the page they were
    /// rendered from carries a `/Rotate` value (#640).
    ///
    /// `normalize_rendered_page_for_ocr` (`crate::pdf::render`) intentionally hands
    /// OCR backends a raster in the PDF page's raw MediaBox orientation rather than
    /// its display orientation, so bbox math elsewhere in the pipeline needs no axis
    /// swap (see the #530 regression test in `crate::pdf::render`). PaddleOCR's
    /// detector still finds and reads the text fine in that orientation — it warps
    /// each detected quad upright before recognition — but its natural block order
    /// is `(y, x)` in *raster* space, which is only true reading order when the page
    /// isn't rotated. On a rotated page that raster-space order groups lines into
    /// several descending runs instead of a single top-to-bottom pass.
    ///
    /// Only the block *order* changes here; every block's stored bbox stays in the
    /// raster space the caller rescales from, matching Tesseract's convention on the
    /// same route.
    fn reorder_blocks_for_page_rotation(
        blocks: &mut [xberg_paddle_ocr::DetailedTextBlock],
        page_rotation_degrees: u32,
        raster_width: u32,
        raster_height: u32,
    ) {
        let correction_degrees = (360 - page_rotation_degrees % 360) % 360;
        if correction_degrees == 0 || raster_width == 0 || raster_height == 0 || blocks.len() < 2 {
            return;
        }
        let (width, height) = (raster_width as f32, raster_height as f32);
        let key = |block: &xberg_paddle_ocr::DetailedTextBlock| {
            Self::block_bounds(block)
                .map(|(min_x, min_y, _, _)| {
                    Self::reading_order_point(min_x as f32, min_y as f32, correction_degrees, width, height)
                })
                .unwrap_or((f32::MAX, f32::MAX))
        };
        blocks.sort_by(|a, b| {
            let (ax, ay) = key(a);
            let (bx, by) = key(b);
            ay.total_cmp(&by).then_with(|| ax.total_cmp(&bx))
        });
    }

    fn is_vertical_text_block(block: &xberg_paddle_ocr::DetailedTextBlock) -> bool {
        let Some((min_x, min_y, max_x, max_y)) = Self::block_bounds(block) else {
            return false;
        };
        let width = max_x.saturating_sub(min_x);
        let height = max_y.saturating_sub(min_y);
        height as f32 >= width as f32 * VERTICAL_TEXT_MIN_ASPECT_RATIO
    }

    fn block_bounds(block: &xberg_paddle_ocr::DetailedTextBlock) -> Option<(u32, u32, u32, u32)> {
        let first = block.block.box_points.first()?;
        Some(block.block.box_points.iter().fold(
            (first.x, first.y, first.x, first.y),
            |(min_x, min_y, max_x, max_y), point| {
                (
                    min_x.min(point.x),
                    min_y.min(point.y),
                    max_x.max(point.x),
                    max_y.max(point.y),
                )
            },
        ))
    }

    /// Clamp the configured recognition batch size to the supported range.
    fn effective_rec_batch_size(config: &PaddleOcrConfig) -> u32 {
        config
            .rec_batch_num
            .clamp(MIN_RECOGNITION_BATCH_SIZE, MAX_RECOGNITION_BATCH_SIZE)
    }

    /// Perform actual OCR inference (runs in blocking context).
    ///
    /// `PaddleOcrEngine::detect` takes `&self`, but that does **not** mean pages OCR
    /// concurrently: each underlying `ort::Session` is still wrapped in a `Mutex`
    /// (`crates/xberg-paddle-ocr/src/inference/ort_backend.rs`) because `ort::Session::run`
    /// requires `&mut self`. Concurrent calls into this function serialize on that mutex —
    /// see `paddle_inference_thread_count` above for why that is handled by widening the
    /// session's own intra-op thread budget instead.
    fn perform_ocr(
        image_bytes: &[u8],
        ocr_engine: &Arc<PaddleOcrEngine>,
        config: &PaddleOcrConfig,
        security_limits: &crate::extractors::security::SecurityLimits,
    ) -> Result<(Vec<xberg_paddle_ocr::DetailedTextBlock>, u32, u32)> {
        let img = crate::extraction::image::load_image_for_ocr(image_bytes, security_limits)
            .map_err(|e| crate::XbergError::Ocr {
                message: e.to_string(),
                source: None,
            })?
            .to_rgb8();
        let processed_width = img.width();
        let processed_height = img.height();

        let padding = config.padding;
        let max_side_len = config.det_limit_side_len;
        let box_score_thresh = config.det_db_box_thresh;
        let box_thresh = config.det_db_thresh;
        let un_clip_ratio = config.det_db_unclip_ratio;
        let do_angle = config.use_angle_cls;
        let most_angle = false;
        let rec_batch_size = Self::effective_rec_batch_size(config);

        let result = ocr_engine
            .detect_detailed_with_rec_batch_size(
                &img,
                padding,
                max_side_len,
                box_score_thresh,
                box_thresh,
                un_clip_ratio,
                do_angle,
                most_angle,
                rec_batch_size,
            )
            .map_err(|e| crate::XbergError::Ocr {
                message: format!("PaddleOCR detection failed: {}", e),
                source: None,
            })?;

        let drop_score = config.drop_score;
        let total_detected = result.text_blocks.len();
        let mut discarded = DropScoreDiscardStats::default();
        let mut kept_box_scores = BoxScoreSummary::default();
        let mut discarded_box_scores = BoxScoreSummary::default();
        let text_blocks: Vec<_> = result
            .text_blocks
            .into_iter()
            .filter(|block| {
                // `passes_drop_score` is the same predicate this replaced (`text_score >=
                // drop_score && !text_score.is_nan()`), factored out so its boundary case is
                // unit-testable. `discarded.record` is a side effect only, called exclusively
                // for blocks this predicate rejects, and never influences the boolean returned.
                let keep = passes_drop_score(block.block.text_score, drop_score);
                if keep {
                    kept_box_scores.record(block.block.box_score);
                } else {
                    discarded.record(block.block.text_score);
                    discarded_box_scores.record(block.block.box_score);
                }
                keep
            })
            .collect();

        // What `drop_score` (default 0.5) discards above, before any of it becomes an element
        // or contributes text — the population the confidence-gate measurement on
        // `process_image` (#675) explicitly could not see, since the surviving detections' mean
        // cannot see what never survived. No page number or path reaches this function
        // (`perform_ocr` takes only `image_bytes`, `ocr_engine`, `config`), so pages are
        // identified by a content hash of the exact bytes OCR'd here; compute the same hash
        // offline over a known page image to line these numbers up against it.
        let page_hash = blake3::hash(image_bytes).to_hex()[..16].to_string();
        tracing::debug!(
            target: "xberg::paddle::confidence",
            page_hash,
            drop_score,
            total_detected,
            kept = text_blocks.len(),
            discarded_count = discarded.discarded_count,
            discarded_nan_count = discarded.nan_count,
            discarded_mean_score = discarded.mean(),
            discarded_min_score = discarded.min,
            discarded_max_score = discarded.max,
            discarded_histogram = ?discarded.histogram,
            // `box_score` is DBNet's detection-region confidence, independent of the `text_score`
            // recognition confidence above — see `BoxScoreSummary`'s doc comment for why this is
            // the untested signal worth sweeping instead of `drop_score` itself.
            kept_box_score_mean = kept_box_scores.mean(),
            kept_box_score_min = kept_box_scores.min,
            kept_box_score_max = kept_box_scores.max,
            discarded_box_score_mean = discarded_box_scores.mean(),
            discarded_box_score_min = discarded_box_scores.min,
            discarded_box_score_max = discarded_box_scores.max,
            "PaddleOCR drop_score discard stats"
        );

        tracing::debug!(text_block_count = text_blocks.len(), "PaddleOCR detection completed");

        Ok((text_blocks, processed_width, processed_height))
    }

    /// Clusters `words` into table-candidate regions, reconstructs a grid for each, validates
    /// it structurally, and builds the `Table` entries `process_image` returns -- including a
    /// real `bounding_box` derived from the region's word extents (#defect-1: previously left
    /// `None`, which meant nothing downstream -- `table_bboxes_by_page` /
    /// `filter_segments_by_table_bboxes` -- could suppress the prose the table duplicates).
    fn build_ocr_tables_from_words(words: &[crate::table_core::HocrWord]) -> BuiltOcrTables {
        let mut tables: Vec<Table> = vec![];
        let mut table_count = 0u32;
        let mut table_rows: Option<u32> = None;
        let mut table_cols: Option<u32> = None;
        let mut rejected_candidates = 0u32;

        for region_words in crate::table_core::cluster_words_into_table_regions(words) {
            if region_words.len() < crate::table_core::MIN_TABLE_CANDIDATE_WORDS {
                continue;
            }

            let cells = reconstruct_table(&region_words, TABLE_COLUMN_ALIGNMENT_THRESHOLD_PX, 0.5);
            if cells.is_empty() || cells[0].is_empty() {
                continue;
            }

            // Pixel-space extents of the words that formed this region, mirroring
            // Tesseract's derivation in `ocr::processor::execution::process_ocr_result`
            // (region_left/top/right/bottom from region_words' min/max). This is the
            // same raster pixel space (x0=left, y0=top, x1=right, y1=bottom, y increasing
            // downward) already used for every other OCR-sourced `BoundingBox` in this
            // file (see the line-element bbox construction in `process_image`).
            let region_left = region_words.iter().map(|w| w.left).min().unwrap_or(0);
            let region_top = region_words.iter().map(|w| w.top).min().unwrap_or(0);
            let region_right = region_words.iter().map(|w| w.left + w.width).max().unwrap_or(0);
            let region_bottom = region_words.iter().map(|w| w.top + w.height).max().unwrap_or(0);

            // PaddleOCR has no per-word table-candidate confidence carve-out the way
            // Tesseract's TSV does, so every recognised word on the page is a clustering
            // candidate (`elements_to_hocr_words` above just filters by OCR confidence, not
            // "is this tabular"). Left unfiltered, a page of ordinary prose reconstructs into
            // one giant sparse grid: each line's words rarely share x-positions with any other
            // line, so `reconstruct_table`'s column detection manufactures one column per word
            // (xberg-io/xberg — measured 36 columns / 390 rows of near-empty cells on a
            // 16-page municipal ordinance with zero real tables). Tesseract's own OCR table
            // path (`ocr::processor::execution::process_ocr_result`) avoids this by running
            // every candidate grid through `pdf::table_reconstruct::post_process_table`, the
            // shared structural validator (column count, cell-content density, prose-length,
            // and column-flow heuristics tuned against the PDF native-text table corpus) —
            // reused here rather than duplicated so both backends reject the same shapes for
            // the same reasons. Only available under the `pdf` feature (same as Tesseract's own
            // gate at `ocr::processor::execution`); without it, table candidates pass through
            // unfiltered, matching Tesseract's degraded behavior in that build.
            #[cfg(feature = "pdf")]
            let cleaned = crate::pdf::table_reconstruct::post_process_table(cells, false, false);
            #[cfg(not(feature = "pdf"))]
            let cleaned = Some(cells);
            let Some(cells) = cleaned else {
                rejected_candidates += 1;
                continue;
            };

            table_count += 1;
            if table_rows.is_none() {
                table_rows = Some(cells.len() as u32);
                table_cols = cells.first().map(|row| row.len() as u32);
            }

            let table_markdown = table_to_markdown(&cells);

            tables.push(Table {
                cells,
                markdown: table_markdown,
                // `process_image` OCRs a single image/page at a time and has no
                // document-level page context to draw from here — Tesseract's own
                // equivalent push site (`ocr::processor::execution::process_ocr_result`)
                // hardcodes the same value for the same reason; callers with real
                // multi-page context (e.g. the PDF mixed route) reassign it afterward.
                page_number: 1,
                bounding_box: Some(crate::types::extraction::BoundingBox {
                    x0: region_left as f64,
                    y0: region_top as f64,
                    x1: region_right as f64,
                    y1: region_bottom as f64,
                }),
                ..Default::default()
            });
        }

        BuiltOcrTables {
            tables,
            table_count,
            table_rows,
            table_cols,
            rejected_candidates,
        }
    }

    fn table_rejection_warning(count: u32) -> Option<crate::types::ProcessingWarning> {
        (count > 0).then(|| crate::types::ProcessingWarning {
            source: Cow::Borrowed("paddle-ocr-table-detection"),
            message: Cow::Owned(format!(
                "PaddleOCR rejected {count} table candidate region(s) during structural validation"
            )),
        })
    }

    /// The page document `process_image` returns: one `OcrText` element per recognised line, less the
    /// lines a detected table carries. Line and table bboxes are both `to_aabb` of the same
    /// recognition pass, so they share one pixel space here.
    fn build_ocr_document(line_elements: &[OcrElement], tables: &[Table]) -> crate::types::internal::InternalDocument {
        use crate::types::extraction::BoundingBox;
        use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
        use crate::types::ocr_elements::OcrElementLevel;

        // PaddleOCR has no paragraph concept of its own (only per-line
        // geometry), so lines are grouped into blocks here and tagged with
        // the same `hocr_block_id` attribute Tesseract's hOCR parser writes,
        // letting `pdf::structure::adapters::ocr_doc_to_paragraphs` merge
        // them with zero changes to its merge logic (#631).
        let block_ids = crate::ocr::conversion::assign_line_block_ids(line_elements);

        let mut doc = InternalDocument::new("pdf");
        for (elem, block_id) in line_elements.iter().zip(block_ids) {
            let (left, top, width, height) = elem.geometry.to_aabb();
            let bbox = BoundingBox {
                x0: left as f64,
                y0: top as f64,
                x1: (left + width) as f64,
                y1: (top + height) as f64,
            };
            let mut ie = InternalElement::text(
                ElementKind::OcrText {
                    level: OcrElementLevel::Line,
                },
                &elem.text,
                0,
            )
            .with_page(elem.page_number);
            ie.bbox = Some(bbox);
            ie.ocr_confidence = Some(elem.confidence.clone());
            ie.ocr_geometry = Some(elem.geometry.clone());
            ie.attributes = Some(
                [(crate::ocr::hocr_parser::HOCR_BLOCK_ID_ATTRIBUTE.to_string(), block_id)]
                    .into_iter()
                    .collect(),
            );
            doc.push_element(ie);
        }
        let claiming_tables = tables
            .iter()
            .filter_map(|table| Some((table.bounding_box?, table.cells.as_slice())));
        drop_document_elements_claimed_by_tables(&mut doc, claiming_tables);
        doc
    }

    fn select_output_elements(
        lines: &[OcrElement],
        words: &[OcrElement],
        config: Option<&OcrElementConfig>,
    ) -> Vec<OcrElement> {
        let Some(config) = config else {
            return Vec::new();
        };
        let elements = lines.iter().chain(words).cloned().collect::<Vec<_>>();
        config.select_elements(&elements)
    }
}

impl Plugin for PaddleOcrBackend {
    fn name(&self) -> &str {
        "paddle-ocr"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl OcrBackend for PaddleOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractedDocument> {
        if image_bytes.is_empty() {
            return Err(crate::XbergError::Validation {
                message: "Empty image data provided to PaddleOCR".to_string(),
                source: None,
            });
        }

        let effective_config: Arc<PaddleOcrConfig> = match crate::core::config::ocr::resolve_paddle_ocr_settings(
            config.paddle_ocr_settings.as_ref(),
            config.paddle_ocr_config.as_ref(),
        )? {
            Some(overridden) => Arc::new(overridden),
            None => Arc::clone(&self.config),
        };
        let cache_dir = effective_config.checked_cache_dir()?;
        let model_context = self.model_context(&cache_dir)?;
        let effective_accel = self.resolve_acceleration(config.acceleration.as_ref());

        let security_limits = Self::resolve_security_limits(config);

        let languages = config.effective_languages();
        let (paddle_lang, mut processing_warnings) = super::select_paddle_language(&languages);

        let mut rotation_outcome = None;
        let ocr_image_bytes: Cow<'_, [u8]> = if config.auto_rotate {
            let decoded_image = crate::extraction::image::load_image_for_ocr(image_bytes, &security_limits)
                .map_err(|error| crate::XbergError::Ocr {
                    message: format!("Failed to decode PaddleOCR image for orientation detection: {error}"),
                    source: None,
                })?
                .to_rgb8();
            let decoded_width = decoded_image.width();
            let decoded_height = decoded_image.height();
            match Self::detect_and_rotate(decoded_image, Arc::clone(&model_context), effective_accel.clone()).await {
                Ok(outcome) => {
                    rotation_outcome = Some(outcome);
                }
                Err(e) => {
                    tracing::warn!("Doc orientation detection failed, proceeding without rotation: {e}");
                    rotation_outcome = Some(RotationOutcome::unrotated(decoded_width, decoded_height));
                }
            }
            match rotation_outcome
                .as_ref()
                .and_then(|outcome| outcome.rotated_bytes.as_deref())
            {
                Some(rotated) => Cow::Borrowed(rotated),
                None => Cow::Borrowed(image_bytes),
            }
        } else {
            Cow::Borrowed(image_bytes)
        };

        let page_rotation_degrees = Self::page_rotation_degrees_from_backend_options(config);
        // `rotation_outcome` is only `Some` while `config.auto_rotate` is true (populated
        // above); when it's `None`, `residual_rotation_for_reorder` falls back to
        // `page_rotation_degrees` unchanged, keeping `auto_rotate: false` byte-identical.
        let auto_rotate_applied_degrees = rotation_outcome
            .as_ref()
            .filter(|outcome| outcome.auto_rotated())
            .and_then(|outcome| outcome.orientation)
            .map(|orientation| orientation.degrees);
        let residual_page_rotation_degrees =
            Self::residual_rotation_for_reorder(page_rotation_degrees, auto_rotate_applied_degrees);

        let PaddlePageOcr {
            text,
            line_elements,
            word_elements,
            processed_width,
            processed_height,
        } = Self::do_ocr(
            &ocr_image_bytes,
            paddle_lang,
            PaddleOcrExecution {
                context: model_context,
                config: Arc::clone(&effective_config),
                acceleration: effective_accel,
            },
            residual_page_rotation_degrees,
            &security_limits,
        )
        .await?;
        let rotation_outcome =
            rotation_outcome.unwrap_or_else(|| RotationOutcome::unrotated(processed_width, processed_height));

        // PaddleOCR pages are deliberately NOT gated on recognition confidence. This logging
        // records the measurement that ruled it out, so the absence does not read as an
        // oversight and the next attempt does not repeat it (#675).
        //
        // Measured on ordinance_2197_scanned.pdf, whose 16 pages include 5 plat/drawing pages
        // that Tesseract drops at mean confidence 36-62 against its threshold of 75: every one
        // of the 16 PaddleOCR pages reports a mean recognition confidence between 0.7909 and
        // 0.9898. The engine is 79-99% confident on pages whose text reads
        // "e, n h me nd me n ae c que by l an pable re, a". The five lowest means are 0.7909,
        // 0.8220, 0.8678, 0.8684 and 0.9298 while the sixth is 0.9393 — a gap of 0.0095 — so no
        // threshold separates the drawings from legitimate text. Minimum confidence is no
        // better: the highest per-page minimum, 0.9546, belongs to a page in the bad set.
        //
        // A different signal is needed. The most promising is what `drop_score` (default 0.5)
        // DISCARDS above, since the surviving detections' mean cannot see it.
        let recognition_confidences: Vec<f64> = line_elements
            .iter()
            .map(|element| element.confidence.recognition)
            .collect();
        if !recognition_confidences.is_empty() {
            let observed = recognition_confidences.iter().sum::<f64>() / recognition_confidences.len() as f64;
            let min = recognition_confidences.iter().copied().fold(f64::INFINITY, f64::min);
            tracing::debug!(
                target: "xberg::paddle::confidence",
                detections = recognition_confidences.len(),
                mean_recognition_confidence = observed,
                min_recognition_confidence = min,
                "PaddleOCR page recognition confidence"
            );
        }

        let text_blocks_count = line_elements.len();

        let mut tables: Vec<Table> = vec![];
        let mut table_count = 0;
        let mut table_rows: Option<u32> = None;
        let mut table_cols: Option<u32> = None;

        if effective_config.enable_table_detection && !line_elements.is_empty() {
            let table_elements = line_elements.iter().chain(&word_elements).cloned().collect::<Vec<_>>();
            let words = elements_to_hocr_words(&table_elements, 0.3);
            let built = Self::build_ocr_tables_from_words(&words);
            tables = built.tables;
            table_count = built.table_count;
            table_rows = built.table_rows;
            table_cols = built.table_cols;
            if let Some(warning) = Self::table_rejection_warning(built.rejected_candidates) {
                processing_warnings.push(warning);
            }
        }

        let ocr_doc = Self::build_ocr_document(&line_elements, &tables);

        tracing::debug!(
            text_blocks = text_blocks_count,
            line_elements = line_elements.len(),
            word_elements = word_elements.len(),
            internal_doc_elements = ocr_doc.elements.len(),
            "PaddleOCR InternalDocument built"
        );

        let metadata = Metadata {
            format: Some(FormatMetadata::Ocr(OcrMetadata {
                language: paddle_lang.to_string(),
                psm: 3,
                output_format: "text".to_string(),
                table_count,
                table_rows,
                table_cols,
            })),
            additional: image_metadata(&rotation_outcome),
            ..Default::default()
        };

        let output_elements =
            Self::select_output_elements(&line_elements, &word_elements, config.element_config.as_ref());
        let ocr_elements_opt = if output_elements.is_empty() {
            None
        } else {
            Some(output_elements)
        };

        Ok(ExtractedDocument {
            content: text,
            mime_type: Cow::Borrowed("text/plain"),
            metadata,
            tables,
            detected_languages: Some(languages),
            ocr_elements: ocr_elements_opt,
            ocr_internal_document: Some(ocr_doc),
            processing_warnings,
            ..Default::default()
        })
    }

    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractedDocument> {
        let bytes = tokio::fs::read(path).await?;
        self.process_image(&bytes, config).await
    }

    fn supports_language(&self, lang: &str) -> bool {
        is_language_supported(lang) || map_language_code(lang).is_some()
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::PaddleOCR
    }

    /// PaddleOCR never derives a page-level aggregate confidence: per-element `text_score`
    /// exists but no page-level number is written.
    fn confidence_semantics(&self) -> crate::plugins::ConfidenceSemantics {
        crate::plugins::ConfidenceSemantics::None
    }

    /// Measured on a `/Rotate 270` scanned ordinance: PaddleOCR warps each detected min-area-rect
    /// quad upright before recognition, so the recognised text is correct, but the block list
    /// stays in raw raster `(y, x)` order — the caller must reorder blocks itself.
    fn page_orientation_handling(&self) -> crate::plugins::PageOrientationHandling {
        crate::plugins::PageOrientationHandling::RecognisesRotatedText
    }

    fn supported_languages(&self) -> Vec<String> {
        super::SUPPORTED_LANGUAGES.iter().map(|s| s.to_string()).collect()
    }

    fn supports_table_detection(&self) -> bool {
        self.config.enable_table_detection
    }

    #[cfg_attr(alef, alef(skip))]
    fn probe(&self, config: &OcrConfig) -> crate::doctor::DoctorCheck {
        use crate::doctor::DoctorCheck;

        let effective_config = match crate::core::config::ocr::resolve_paddle_ocr_settings(
            config.paddle_ocr_settings.as_ref(),
            config.paddle_ocr_config.as_ref(),
        ) {
            Ok(Some(overridden)) => overridden,
            Ok(None) => (*self.config).clone(),
            Err(error) => return DoctorCheck::fail("ocr.paddle-ocr", error.to_string()),
        };

        let languages = config.effective_languages();
        let (paddle_lang, _warnings) = super::select_paddle_language(&languages);
        let family = language_to_script_family(paddle_lang);

        let manager = match effective_config.checked_cache_dir() {
            Ok(cache_dir) => ModelManager::new(cache_dir),
            Err(error) => return DoctorCheck::fail("ocr.paddle-ocr", error.to_string()),
        };
        match manager.check_models_cached(
            &effective_config.model_version,
            family,
            &effective_config.model_tier,
            config.auto_rotate,
        ) {
            Ok(artifacts) => {
                let missing: Vec<&str> = artifacts
                    .iter()
                    .filter(|(_, cached)| !cached)
                    .map(|(label, _)| label.as_str())
                    .collect();
                if missing.is_empty() {
                    DoctorCheck::pass(
                        "ocr.paddle-ocr",
                        format!("all models cached and verified ({family} recognition)"),
                    )
                } else {
                    DoctorCheck::skip(
                        "ocr.paddle-ocr",
                        format!(
                            "models not cached locally: {} (will download on first use)",
                            missing.join(", ")
                        ),
                    )
                }
            }
            Err(e) => DoctorCheck::fail("ocr.paddle-ocr", format!("{e}")),
        }
    }
}

impl Default for PaddleOcrBackend {
    fn default() -> Self {
        Self::with_config(PaddleOcrConfig::default())
            .unwrap_or_else(|e| panic!("Failed to create default PaddleOcrBackend: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    /// `passes_drop_score` is the live predicate `perform_ocr`'s filter calls; this pins its
    /// current boundary behaviour: `text_score == drop_score` is `>=`, so it survives (is kept,
    /// not discarded). This test would fail if that boundary were ever flipped to strict `>`,
    /// which is the only way it can discriminate — the predicate is a one-line direct copy of
    /// the pre-instrumentation filter condition, so there is no separate "old" behaviour to
    /// diverge from today.
    #[test]
    fn passes_drop_score_keeps_the_exact_threshold_score() {
        let drop_score = 0.5_f32;
        assert!(passes_drop_score(drop_score, drop_score));
        assert!(passes_drop_score(drop_score + 0.01, drop_score));
        assert!(!passes_drop_score(drop_score - 0.01, drop_score));
    }

    /// NaN scores are excluded regardless of `drop_score`, matching the pre-instrumentation
    /// `&& !text_score.is_nan()` clause verbatim.
    #[test]
    fn passes_drop_score_rejects_nan_scores() {
        assert!(!passes_drop_score(f32::NAN, 0.5));
        assert!(!passes_drop_score(f32::NAN, 0.0));
    }

    /// Drives `DropScoreDiscardStats::record` with a synthetic set of discarded scores whose
    /// count, mean, min, max, and histogram bucketing are all known by hand, and asserts every
    /// field against the hand-computed value rather than a re-derived formula. This is a new
    /// struct built for this task, so there is no pre-instrumentation behaviour to diverge
    /// from — the test pins the arithmetic (mean = sum/count over non-NaN scores; NaN counted
    /// separately and excluded from sum/min/max/histogram; 5 equal-width buckets over
    /// `[0.0, 1.0]`) rather than catching a regression.
    #[test]
    fn drop_score_discard_stats_computes_exact_summary_from_synthetic_scores() {
        let mut stats = DropScoreDiscardStats::default();
        // Non-NaN discarded scores: 0.05, 0.15, 0.35, 0.49, plus two NaNs.
        for score in [0.05_f32, 0.15, 0.35, 0.49] {
            stats.record(score);
        }
        stats.record(f32::NAN);
        stats.record(f32::NAN);

        assert_eq!(stats.discarded_count, 6);
        assert_eq!(stats.nan_count, 2);
        assert_eq!(stats.scored_count(), 4);
        assert!((stats.mean() - (0.05 + 0.15 + 0.35 + 0.49) / 4.0).abs() < 1e-6);
        assert!((stats.min - 0.05).abs() < 1e-6);
        assert!((stats.max - 0.49).abs() < 1e-6);
        // Bucket width 0.2: [0.0,0.2) gets 0.05 and 0.15; [0.2,0.4) gets 0.35; [0.4,0.6) gets
        // 0.49; the top two buckets stay empty since nothing discarded here reaches 0.6.
        assert_eq!(stats.histogram, [2, 1, 1, 0, 0]);
    }

    /// An empty (no discards) summary reports zeroed fields rather than NaN/undefined values,
    /// so a "nothing discarded" page reads as `discarded_count = 0` with a well-defined mean of
    /// `0.0`, not a division-by-zero artifact.
    #[test]
    fn drop_score_discard_stats_defaults_to_zero_with_no_discards() {
        let stats = DropScoreDiscardStats::default();
        assert_eq!(stats.discarded_count, 0);
        assert_eq!(stats.nan_count, 0);
        assert_eq!(stats.mean(), 0.0);
        assert_eq!(stats.min, 0.0);
        assert_eq!(stats.max, 0.0);
        assert_eq!(stats.histogram, [0, 0, 0, 0, 0]);
    }

    /// A score exactly at a bucket boundary (0.2, 0.4, 0.6, 0.8) must round into the bucket
    /// starting there, not the one below — confirms `(score * buckets) as usize` truncates
    /// rather than rounding-to-nearest, and that a score of exactly `1.0` clamps into the last
    /// bucket instead of indexing one past the end.
    #[test]
    fn drop_score_discard_stats_histogram_boundaries_land_in_the_upper_bucket() {
        let mut stats = DropScoreDiscardStats::default();
        for score in [0.0_f32, 0.2, 0.4, 0.6, 0.8, 1.0] {
            stats.record(score);
        }
        assert_eq!(stats.histogram, [1, 1, 1, 1, 2]);
    }

    /// Drives `BoxScoreSummary::record` with a synthetic set of scores whose count, mean, min,
    /// and max are known by hand. This is a new struct built for this task, so there is no
    /// pre-instrumentation behaviour to diverge from — the test pins the arithmetic (mean =
    /// sum/count over non-NaN scores) rather than catching a regression.
    #[test]
    fn box_score_summary_computes_exact_summary_from_synthetic_scores() {
        let mut stats = BoxScoreSummary::default();
        for score in [0.62_f32, 0.71, 0.88, 0.93] {
            stats.record(score);
        }
        stats.record(f32::NAN);

        assert_eq!(stats.count, 4);
        assert!((stats.mean() - (0.62 + 0.71 + 0.88 + 0.93) / 4.0).abs() < 1e-6);
        assert!((stats.min - 0.62).abs() < 1e-6);
        assert!((stats.max - 0.93).abs() < 1e-6);
    }

    /// An empty (no recorded scores) summary reports zeroed fields rather than NaN/undefined
    /// values, matching `DropScoreDiscardStats::mean`'s "no data" convention.
    #[test]
    fn box_score_summary_defaults_to_zero_with_no_scores() {
        let stats = BoxScoreSummary::default();
        assert_eq!(stats.count, 0);
        assert_eq!(stats.mean(), 0.0);
        assert_eq!(stats.min, 0.0);
        assert_eq!(stats.max, 0.0);
    }

    /// The session must receive the *entire* resolved process budget, not the hardcoded `1`
    /// this replaced. `workers * intra_threads <= budget` still holds because the mutex caps
    /// PaddleOCR to one concurrent worker. Honest caveat: this is a new pure function, so there
    /// is no unfixed version of it to fail against — the assertion's value is pinning the policy
    /// so a future clamp back to `1` (or a multi-session pool) is forced to revisit it.
    #[test]
    fn paddle_session_thread_budget_grants_the_full_resolved_budget() {
        assert_eq!(paddle_session_thread_budget(1), 1);
        assert_eq!(paddle_session_thread_budget(4), 4);
        assert_eq!(paddle_session_thread_budget(8), 8);
    }

    /// `resolve_thread_budget` never returns `0`, but the pure function must not assume it —
    /// `with_intra_threads(0)` would be a session-construction footgun.
    #[test]
    fn paddle_session_thread_budget_floors_at_one() {
        assert_eq!(paddle_session_thread_budget(0), 1);
    }

    const CONCURRENT_INITIALIZER_COUNT: usize = 8;

    #[test]
    fn engine_init_cell_initializes_a_key_once_across_threads() {
        let pool = Arc::new(Mutex::new(AHashMap::new()));
        let start = Arc::new(Barrier::new(CONCURRENT_INITIALIZER_COUNT));
        let initialization_count = Arc::new(AtomicUsize::new(0));

        let workers = (0..CONCURRENT_INITIALIZER_COUNT)
            .map(|_| {
                let pool = Arc::clone(&pool);
                let start = Arc::clone(&start);
                let initialization_count = Arc::clone(&initialization_count);
                thread::spawn(move || {
                    start.wait();
                    let cell = init_cell_for_key(&pool, "shared").expect("pool lock should be available");
                    *cell.get_or_init(|| {
                        initialization_count.fetch_add(1, Ordering::SeqCst);
                        42
                    })
                })
            })
            .collect::<Vec<_>>();

        let results = workers
            .into_iter()
            .map(|worker| worker.join().expect("initializer worker should not panic"))
            .collect::<Vec<_>>();

        assert_eq!(results, vec![42; CONCURRENT_INITIALIZER_COUNT]);
        assert_eq!(initialization_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn engine_init_cell_does_not_hold_pool_lock_during_initialization() {
        let pool = Mutex::new(AHashMap::new());
        let first = init_cell_for_key(&pool, "first").expect("pool lock should be available");

        let value = first.get_or_init(|| {
            let second = init_cell_for_key(&pool, "second").expect("another key should remain accessible");
            assert_eq!(second.set(2), Ok(()));
            1
        });

        assert_eq!(*value, 1);
        assert_eq!(pool.lock().expect("pool lock should be available").len(), 2);
    }

    #[test]
    fn engine_init_cell_retries_after_initialization_failure() {
        let pool = Mutex::new(AHashMap::new());
        let cell = init_cell_for_key(&pool, "retryable").expect("pool lock should be available");
        let initialization_count = AtomicUsize::new(0);

        let first: std::result::Result<&usize, &str> = cell.get_or_try_init(|| {
            initialization_count.fetch_add(1, Ordering::SeqCst);
            Err("initialization failed")
        });
        let second = cell.get_or_try_init(|| {
            initialization_count.fetch_add(1, Ordering::SeqCst);
            Ok::<usize, &str>(42)
        });

        assert_eq!(first, Err("initialization failed"));
        assert_eq!(second, Ok(&42));
        assert_eq!(initialization_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn engine_pool_key_distinguishes_gpu_devices() {
        use crate::core::config::acceleration::{AccelerationConfig, ExecutionProviderType};

        let first_gpu = AccelerationConfig {
            provider: ExecutionProviderType::Cuda,
            device_id: 0,
        };
        let second_gpu = AccelerationConfig {
            provider: ExecutionProviderType::Cuda,
            device_id: 1,
        };

        assert_eq!(
            engine_pool_key("v6", "small", "latin", None, PaddleInferenceBackend::Ort),
            "v6/small/latin/cpu/ort"
        );
        assert_ne!(
            engine_pool_key("v6", "small", "latin", Some(&first_gpu), PaddleInferenceBackend::Ort),
            engine_pool_key("v6", "small", "latin", Some(&second_gpu), PaddleInferenceBackend::Ort)
        );
    }

    #[test]
    fn engine_pool_key_distinguishes_inference_backends() {
        assert_ne!(
            engine_pool_key("v6", "small", "latin", None, PaddleInferenceBackend::Ort),
            engine_pool_key("v6", "small", "latin", None, PaddleInferenceBackend::Tract)
        );
    }

    #[test]
    fn request_cache_roots_use_isolated_model_contexts() {
        let backend = PaddleOcrBackend::new().expect("backend should construct");
        let first = backend
            .model_context(std::path::Path::new("/first/cache"))
            .expect("first model context should resolve");
        let second = backend
            .model_context(std::path::Path::new("/second/cache"))
            .expect("second model context should resolve");
        let first_again = backend
            .model_context(std::path::Path::new("/first/cache"))
            .expect("first model context should be reusable");

        assert_eq!(
            first.model_manager.cache_dir(),
            &std::path::PathBuf::from("/first/cache")
        );
        assert_eq!(
            second.model_manager.cache_dir(),
            &std::path::PathBuf::from("/second/cache")
        );
        assert!(Arc::ptr_eq(&first, &first_again));
        assert!(!Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn backend_rejects_an_empty_explicit_model_cache_root() {
        let error = match PaddleOcrBackend::with_config(PaddleOcrConfig::new("en").with_cache_dir(PathBuf::new())) {
            Ok(_) => panic!("an empty explicit model cache root must be rejected"),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "Validation error: OCR model cache_dir must not be empty"
        );
    }

    #[test]
    fn model_context_cache_evicts_the_least_recently_used_root() {
        let backend = PaddleOcrBackend::new().expect("backend should construct");
        let first = backend
            .model_context(std::path::Path::new("/cache/0"))
            .expect("first context should resolve");
        let first_weak = Arc::downgrade(&first);
        drop(first);
        for index in 1..MODEL_CONTEXT_CAPACITY {
            backend
                .model_context(std::path::Path::new(&format!("/cache/{index}")))
                .expect("context should resolve");
        }
        backend
            .model_context(std::path::Path::new(&format!("/cache/{MODEL_CONTEXT_CAPACITY}")))
            .expect("new context should evict the inactive least-recently-used context");
        assert!(first_weak.upgrade().is_none());
        let first_again = backend
            .model_context(std::path::Path::new("/cache/0"))
            .expect("evicted context should be recreated");

        assert_eq!(first_again.cache_dir, std::path::PathBuf::from("/cache/0"));
        assert_eq!(
            backend
                .model_contexts
                .lock()
                .expect("model context cache should be available")
                .entries
                .len(),
            MODEL_CONTEXT_CAPACITY
        );
    }

    #[test]
    fn model_context_cache_does_not_rebuild_an_active_root() {
        let backend = PaddleOcrBackend::new().expect("backend should construct");
        let active = (0..MODEL_CONTEXT_CAPACITY)
            .map(|index| {
                backend
                    .model_context(std::path::Path::new(&format!("/active/cache/{index}")))
                    .expect("active context should resolve")
            })
            .collect::<Vec<_>>();

        let overflow = match backend.model_context(std::path::Path::new("/active/cache/overflow")) {
            Ok(_) => panic!("a fifth active cache root must be rejected"),
            Err(error) => error,
        };
        assert_eq!(
            overflow.to_string(),
            concat!(
                "OCR error: PaddleOCR model context capacity (4 active cache roots) is exhausted; ",
                "retry after an active OCR request completes"
            )
        );
        let first_again = backend
            .model_context(std::path::Path::new("/active/cache/0"))
            .expect("active context should remain addressable");

        assert!(Arc::ptr_eq(&active[0], &first_again));
    }

    #[test]
    fn model_context_cache_never_exceeds_the_live_context_limit() {
        let backend = PaddleOcrBackend::new().expect("backend should construct");
        let active = (0..MODEL_CONTEXT_CAPACITY)
            .map(|index| {
                backend
                    .model_context(std::path::Path::new(&format!("/bounded/cache/{index}")))
                    .expect("active context should resolve")
            })
            .collect::<Vec<_>>();
        let contexts = active.iter().map(Arc::downgrade).collect::<Vec<_>>();

        assert!(
            backend
                .model_context(std::path::Path::new("/bounded/cache/overflow"))
                .is_err(),
            "a fifth live context must be rejected"
        );

        assert_eq!(
            contexts.iter().filter(|context| context.upgrade().is_some()).count(),
            MODEL_CONTEXT_CAPACITY
        );
    }

    #[test]
    fn concurrent_context_admission_recovers_after_an_active_root_is_released() {
        use std::sync::Barrier;

        let backend = Arc::new(PaddleOcrBackend::new().expect("backend should construct"));
        let start = Arc::new(Barrier::new(MODEL_CONTEXT_CAPACITY + 2));
        let handles = (0..=MODEL_CONTEXT_CAPACITY)
            .map(|index| {
                let backend = Arc::clone(&backend);
                let start = Arc::clone(&start);
                std::thread::spawn(move || {
                    let cache_dir = std::path::PathBuf::from(format!("/concurrent/cache/{index}"));
                    start.wait();
                    let context = backend.model_context(&cache_dir);
                    (cache_dir, context)
                })
            })
            .collect::<Vec<_>>();
        start.wait();

        let mut admitted = Vec::new();
        let mut rejected = None;
        for handle in handles {
            let (cache_dir, context) = handle.join().expect("context request thread should complete");
            match context {
                Ok(context) => admitted.push(context),
                Err(error) => {
                    assert!(rejected.replace((cache_dir, error)).is_none());
                }
            }
        }

        assert_eq!(admitted.len(), MODEL_CONTEXT_CAPACITY);
        let (rejected_root, _) = rejected.expect("exactly one root should be rejected at capacity");
        for context in &admitted {
            let same = backend
                .model_context(&context.cache_dir)
                .expect("an active root must remain addressable");
            assert!(Arc::ptr_eq(context, &same));
        }

        drop(admitted.pop());
        let newly_admitted = backend
            .model_context(&rejected_root)
            .expect("a rejected root should be admitted once one active root is released");
        assert_eq!(newly_admitted.cache_dir, rejected_root);
        assert_eq!(
            backend
                .model_contexts
                .lock()
                .expect("model context cache should be available")
                .entries
                .len(),
            MODEL_CONTEXT_CAPACITY
        );
    }

    #[test]
    fn orientation_pool_key_distinguishes_acceleration_devices() {
        use crate::core::config::acceleration::{AccelerationConfig, ExecutionProviderType};

        let first_gpu = AccelerationConfig {
            provider: ExecutionProviderType::Cuda,
            device_id: 0,
        };
        let second_gpu = AccelerationConfig {
            provider: ExecutionProviderType::Cuda,
            device_id: 1,
        };

        assert_ne!(
            acceleration_pool_key(Some(&first_gpu)),
            acceleration_pool_key(Some(&second_gpu))
        );
        assert_eq!(acceleration_pool_key(None), "cpu");
    }

    #[test]
    fn orientation_detector_cache_reuses_the_same_live_cell_for_a_key() {
        let pool = BoundedInitPool::<usize>::new(2);
        let first = pool
            .cell("cpu")
            .expect("orientation cache lock should be available")
            .expect("first acceleration key should be admitted");
        let same = pool
            .cell("cpu")
            .expect("orientation cache lock should be available")
            .expect("existing acceleration key should remain addressable");

        assert!(Arc::ptr_eq(&first, &same));
        assert_eq!(pool.len().expect("orientation cache lock should be available"), 1);
    }

    #[test]
    fn orientation_detector_cache_recovers_after_an_active_entry_is_released() {
        let pool = BoundedInitPool::<usize>::new(2);
        let first = pool
            .cell("cpu")
            .expect("orientation cache lock should be available")
            .expect("first acceleration key should be admitted");
        let first_weak = Arc::downgrade(&first);
        let second = pool
            .cell("cuda:0")
            .expect("orientation cache lock should be available")
            .expect("second acceleration key should be admitted");

        assert!(
            pool.cell("cuda:1")
                .expect("orientation cache lock should be available")
                .is_none(),
            "a new acceleration key must be rejected while every cached detector is active"
        );
        let second_again = pool
            .cell("cuda:0")
            .expect("orientation cache lock should be available")
            .expect("an active acceleration key should remain addressable");
        assert!(Arc::ptr_eq(&second, &second_again));

        drop(first);
        let third = pool
            .cell("cuda:1")
            .expect("orientation cache lock should be available")
            .expect("a new key should be admitted after an inactive entry can be evicted");

        assert!(first_weak.upgrade().is_none());
        assert_eq!(pool.len().expect("orientation cache lock should be available"), 2);
        assert!(
            Arc::ptr_eq(
                &third,
                &pool
                    .cell("cuda:1")
                    .expect("orientation cache lock should be available")
                    .expect("new key should preserve identity")
            ),
            "an admitted detector cell must not be rebuilt while live"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn document_orientation_work_runs_off_the_tokio_worker() {
        let tokio_worker = std::thread::current().id();

        let orientation_worker = run_orientation_blocking(move || Ok(std::thread::current().id()))
            .await
            .expect("blocking orientation work should complete");

        assert_ne!(tokio_worker, orientation_worker);
    }

    fn detailed_block(text: &str, left: u32, top: u32, width: u32, height: u32) -> xberg_paddle_ocr::DetailedTextBlock {
        xberg_paddle_ocr::DetailedTextBlock {
            block: xberg_paddle_ocr::TextBlock {
                box_points: vec![
                    xberg_paddle_ocr::Point { x: left, y: top },
                    xberg_paddle_ocr::Point {
                        x: left + width,
                        y: top,
                    },
                    xberg_paddle_ocr::Point {
                        x: left + width,
                        y: top + height,
                    },
                    xberg_paddle_ocr::Point {
                        x: left,
                        y: top + height,
                    },
                ],
                box_score: 0.9,
                angle_index: 0,
                angle_score: 1.0,
                text: text.to_string(),
                text_score: 0.9,
            },
            words: Vec::new(),
            line_column_count: 0.0,
            rotation_retained: false,
        }
    }

    fn output_element(text: &str, level: OcrElementLevel, confidence: f64) -> OcrElement {
        OcrElement::new(
            text,
            crate::types::OcrBoundingGeometry::Rectangle {
                left: 0,
                top: 0,
                width: 10,
                height: 10,
            },
            crate::types::OcrConfidence::from_tesseract(confidence * 100.0),
        )
        .with_level(level)
    }

    #[test]
    fn default_paddle_element_granularity_remains_line_only() {
        let lines = [output_element("line", OcrElementLevel::Line, 0.9)];
        let words = [output_element("word", OcrElementLevel::Word, 0.9)];
        let config = OcrElementConfig {
            include_elements: true,
            ..Default::default()
        };

        let selected = PaddleOcrBackend::select_output_elements(&lines, &words, Some(&config));

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].text, "line");
    }

    #[test]
    fn word_granularity_exposes_hierarchy_and_filters_confidence() {
        let lines = [output_element("line", OcrElementLevel::Line, 0.9)];
        let words = [
            output_element("kept", OcrElementLevel::Word, 0.8),
            output_element("dropped", OcrElementLevel::Word, 0.4),
        ];
        let config = OcrElementConfig {
            include_elements: true,
            min_level: OcrElementLevel::Word,
            min_confidence: 0.5,
            build_hierarchy: true,
        };

        let selected = PaddleOcrBackend::select_output_elements(&lines, &words, Some(&config));

        assert_eq!(
            selected.iter().map(|element| element.text.as_str()).collect::<Vec<_>>(),
            ["line", "kept"]
        );
        assert_eq!(selected[1].parent_id.as_deref(), selected[0].element_id());
    }

    #[test]
    fn vertical_japanese_columns_are_ordered_right_to_left() {
        let mut blocks = vec![
            detailed_block("left", 10, 0, 10, 100),
            detailed_block("right", 50, 0, 10, 100),
            detailed_block("middle", 30, 0, 10, 100),
        ];

        let vertical = PaddleOcrBackend::sort_vertical_cjk_blocks(&mut blocks, "japan");

        assert!(vertical);
        assert_eq!(
            blocks.iter().map(|block| block.block.text.as_str()).collect::<Vec<_>>(),
            ["right", "middle", "left"]
        );
        assert_eq!(
            PaddleOcrBackend::assemble_block_text(&blocks, vertical),
            "rightmiddleleft"
        );
    }

    #[test]
    fn vertical_column_fragments_are_ordered_top_to_bottom_despite_x_jitter() {
        let mut blocks = vec![
            detailed_block("left", 10, 0, 10, 100),
            detailed_block("right-bottom", 51, 60, 10, 50),
            detailed_block("right-top", 50, 0, 10, 50),
        ];

        let vertical = PaddleOcrBackend::sort_vertical_cjk_blocks(&mut blocks, "japan");

        assert!(vertical);
        assert_eq!(
            blocks.iter().map(|block| block.block.text.as_str()).collect::<Vec<_>>(),
            ["right-top", "right-bottom", "left"]
        );
    }

    #[test]
    fn horizontal_japanese_lines_keep_detector_order() {
        let mut blocks = vec![
            detailed_block("first", 50, 0, 100, 10),
            detailed_block("second", 10, 20, 100, 10),
        ];

        let vertical = PaddleOcrBackend::sort_vertical_cjk_blocks(&mut blocks, "japan");

        assert!(!vertical);
        assert_eq!(
            blocks.iter().map(|block| block.block.text.as_str()).collect::<Vec<_>>(),
            ["first", "second"]
        );
    }

    /// #640 — on a page rendered from a `/Rotate 270` PDF page, the OCR raster is in
    /// the page's raw MediaBox orientation (`normalize_rendered_page_for_ocr`), so
    /// text that reads top-to-bottom on the page appears as a column running along
    /// the raster's X axis. The detector's own `(y, x)`-in-raster-space order groups
    /// these lines into several descending runs instead of one top-to-bottom pass;
    /// `reorder_blocks_for_page_rotation` must recover the true (monotonic) order.
    ///
    /// This fails against the unfixed code: without a `reorder_blocks_for_page_rotation`
    /// call, blocks are left in the order the detector handed them — raw ascending
    /// raster `min_y` (90, 100, 105, 110), i.e. `["line3", "line1", "line4", "line2"]`
    /// — not the monotonic `["line1", "line2", "line3", "line4"]` asserted below.
    #[test]
    fn reorders_blocks_into_monotonic_order_on_a_rotated_page() {
        // Detector order: ascending raster min_y (90, 100, 105, 110) — not true
        // reading order, which runs by descending raster min_x instead (800, 600,
        // 400, 200) on this /Rotate 270 page.
        let mut blocks = vec![
            detailed_block("line3", 400, 90, 20, 20),
            detailed_block("line1", 800, 100, 20, 20),
            detailed_block("line4", 200, 105, 20, 20),
            detailed_block("line2", 600, 110, 20, 20),
        ];

        PaddleOcrBackend::reorder_blocks_for_page_rotation(&mut blocks, 270, 900, 1000);

        let order: Vec<&str> = blocks.iter().map(|block| block.block.text.as_str()).collect();
        assert_eq!(
            order,
            ["line1", "line2", "line3", "line4"],
            "reading order must be monotonic"
        );
    }

    /// #640 — an unrotated page (`page_rotation_degrees == 0`, the overwhelmingly
    /// common case) must not be touched at all: reordering by a no-op correction
    /// would silently perturb pages that are already correct (guards against
    /// introducing a *second* rotation bug while fixing this one).
    #[test]
    fn leaves_block_order_unchanged_when_page_is_not_rotated() {
        let mut blocks = vec![
            detailed_block("first", 10, 10, 20, 20),
            detailed_block("second", 10, 40, 20, 20),
            detailed_block("third", 10, 70, 20, 20),
        ];
        let original: Vec<String> = blocks.iter().map(|block| block.block.text.clone()).collect();

        PaddleOcrBackend::reorder_blocks_for_page_rotation(&mut blocks, 0, 900, 1000);

        let order: Vec<&str> = blocks.iter().map(|block| block.block.text.as_str()).collect();
        assert_eq!(order, original.iter().map(String::as_str).collect::<Vec<_>>());
    }

    /// #640 — `extractors::pdf::ocr` threads the page's `/Rotate` value through
    /// `OcrConfig.backend_options["page_rotation_degrees"]`; absent (or non-numeric)
    /// values must resolve to "no rotation" so direct image OCR and other backends'
    /// configs reusing this field are unaffected.
    #[test]
    fn reads_page_rotation_hint_from_backend_options() {
        let with_hint = OcrConfig {
            backend_options: Some(serde_json::json!({"page_rotation_degrees": 270})),
            ..Default::default()
        };
        assert_eq!(
            PaddleOcrBackend::page_rotation_degrees_from_backend_options(&with_hint),
            270
        );

        let without_hint = OcrConfig::default();
        assert_eq!(
            PaddleOcrBackend::page_rotation_degrees_from_backend_options(&without_hint),
            0
        );

        let non_numeric = OcrConfig {
            backend_options: Some(serde_json::json!({"page_rotation_degrees": "sideways"})),
            ..Default::default()
        };
        assert_eq!(
            PaddleOcrBackend::page_rotation_degrees_from_backend_options(&non_numeric),
            0
        );
    }

    /// GH#1554: absent or unparseable `backend_options.security_limits` must yield `None`
    /// (no override), not a default-filled `SecurityLimits` — a default masquerading as "no
    /// override" would shadow `OcrConfig::security_limits` in `resolve_security_limits`.
    #[test]
    fn reads_security_limits_override_from_backend_options() {
        let with_override = OcrConfig {
            backend_options: Some(serde_json::json!({"security_limits": {"max_content_size": 200 * 1024 * 1024}})),
            ..Default::default()
        };
        assert_eq!(
            PaddleOcrBackend::security_limits_override_from_backend_options(&with_override)
                .expect("a well-formed override must parse")
                .max_content_size,
            200 * 1024 * 1024
        );

        let without_hint = OcrConfig::default();
        assert!(PaddleOcrBackend::security_limits_override_from_backend_options(&without_hint).is_none());

        let non_object = OcrConfig {
            backend_options: Some(serde_json::json!({"security_limits": "not an object"})),
            ..Default::default()
        };
        assert!(PaddleOcrBackend::security_limits_override_from_backend_options(&non_object).is_none());
    }

    /// GH#1554 regression: `resolve_security_limits` must prefer, in order, a per-call
    /// `backend_options` override, then `OcrConfig::security_limits` (the field
    /// `image_ocr.rs` populates from the caller's `ExtractionConfig::security_limits`),
    /// then `SecurityLimits::default()`. Before this fix, `OcrConfig::security_limits` did
    /// not exist and the backend fell straight to `SecurityLimits::default()` whenever no
    /// `backend_options` override was set — silently ignoring a caller's configured limit.
    #[test]
    fn resolve_security_limits_honors_precedence() {
        let neither = OcrConfig::default();
        assert_eq!(
            PaddleOcrBackend::resolve_security_limits(&neither).max_content_size,
            crate::extractors::security::SecurityLimits::default().max_content_size
        );

        let inherited_only = OcrConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_content_size: 200 * 1024 * 1024,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            PaddleOcrBackend::resolve_security_limits(&inherited_only).max_content_size,
            200 * 1024 * 1024
        );

        let override_beats_inherited = OcrConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_content_size: 200 * 1024 * 1024,
                ..Default::default()
            }),
            backend_options: Some(serde_json::json!({"security_limits": {"max_content_size": 300 * 1024 * 1024}})),
            ..Default::default()
        };
        assert_eq!(
            PaddleOcrBackend::resolve_security_limits(&override_beats_inherited).max_content_size,
            300 * 1024 * 1024
        );
    }

    /// GH#1554 end-to-end regression: a `security_limits` set on `ExtractionConfig` (mirrored
    /// onto `OcrConfig::security_limits` by `image_ocr::process_images_with_ocr`, the same
    /// mechanism `OcrConfig::acceleration` uses) must actually reach PaddleOCR's image decode
    /// and permit a scan `SecurityLimits::default()` would reject. Uses the same 6100x6100
    /// solid-color fixture as `extraction::image`'s regression test: it decodes to
    /// 6100 * 6100 * 3 = 111,630,000 bytes, over the 100 MiB default but under a
    /// caller-configured 200 MiB limit, while the PNG encoding stays a few hundred bytes.
    #[test]
    fn configured_ocr_config_security_limits_permit_scan_default_rejects() {
        use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
        use std::io::Cursor;

        let (width, height) = (6100u32, 6100u32);
        let img: RgbImage = ImageBuffer::from_pixel(width, height, Rgb([200u8, 100, 50]));
        let mut bytes: Vec<u8> = Vec::new();
        img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png).unwrap();

        let default_config = OcrConfig::default();
        let default_limits = PaddleOcrBackend::resolve_security_limits(&default_config);
        let default_error = crate::extraction::image::load_image_for_ocr(&bytes, &default_limits)
            .expect_err("a 111,630,000-byte decode must be refused under the 100 MiB default");
        assert!(matches!(default_error, crate::XbergError::Validation { .. }));

        let configured_config = OcrConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_content_size: 200 * 1024 * 1024,
                ..Default::default()
            }),
            ..Default::default()
        };
        let configured_limits = PaddleOcrBackend::resolve_security_limits(&configured_config);
        let image = crate::extraction::image::load_image_for_ocr(&bytes, &configured_limits)
            .expect("a caller-configured 200 MiB limit inherited onto OcrConfig must permit the same decode");
        assert_eq!((image.width(), image.height()), (width, height));
    }

    /// The defect this guards: `reorder_blocks_for_page_rotation`'s premise is that its
    /// blocks live in the *raw* MediaBox raster the `/Rotate` hint describes. When
    /// `auto_rotate` actually rotated the raster before OCR, reordering by the raw
    /// `/Rotate` value alone double-corrects — once via the pixel rotation already
    /// applied, once via the reorder itself.
    ///
    /// Exhaustive over all four quarter-turns of `page_rotation_degrees` crossed with
    /// "not auto-rotated" and every quarter-turn `detect_and_rotate` could actually apply
    /// (`auto_rotate_applied_degrees`). This fails against the unfixed code path, which
    /// passes the raw `page_rotation_degrees` straight to `do_ocr` regardless of whether
    /// auto-rotation ran — i.e. it never applies the `Some(_)` composition below at all.
    #[test]
    fn residual_rotation_composes_page_rotate_with_applied_auto_rotation() {
        // Not auto-rotated (config off, low-confidence detection, or detection failure
        // falling back to `RotationOutcome::unrotated`): the raster is still the raw
        // MediaBox raster, so the residual must equal `page_rotation_degrees` unchanged.
        for page_rotation_degrees in [0, 90, 180, 270] {
            assert_eq!(
                PaddleOcrBackend::residual_rotation_for_reorder(page_rotation_degrees, None),
                page_rotation_degrees,
                "page_rotation_degrees={page_rotation_degrees} must pass through unchanged when not auto-rotated"
            );
        }

        // Auto-rotated: a detector reading `applied_degrees` on the raw raster caused a
        // pixel rotation of `(360 - applied_degrees) % 360`. When the detector's estimate
        // matches the page's own `/Rotate` value, the raster is now upright and the
        // residual must be 0 — no further reordering.
        for page_rotation_degrees in [0u32, 90, 180, 270] {
            assert_eq!(
                PaddleOcrBackend::residual_rotation_for_reorder(page_rotation_degrees, Some(page_rotation_degrees)),
                0,
                "page_rotation_degrees={page_rotation_degrees} must fully cancel when the detector agrees with it"
            );
        }

        // General composition table: residual = (page_rotation_degrees + 360 -
        // applied_degrees) % 360, for every quarter-turn combination.
        let cases: [(u32, u32, u32); 16] = [
            (0, 90, 270),
            (0, 180, 180),
            (0, 270, 90),
            (90, 90, 0),
            (90, 180, 270),
            (90, 270, 180),
            (180, 90, 90),
            (180, 180, 0),
            (180, 270, 270),
            (270, 90, 180),
            (270, 180, 90),
            (270, 270, 0),
            (0, 0, 0),
            (90, 0, 90),
            (180, 0, 180),
            (270, 0, 270),
        ];
        for (page_rotation_degrees, applied_degrees, expected_residual) in cases {
            assert_eq!(
                PaddleOcrBackend::residual_rotation_for_reorder(page_rotation_degrees, Some(applied_degrees)),
                expected_residual,
                "page_rotation_degrees={page_rotation_degrees}, applied_degrees={applied_degrees}"
            );
        }
    }

    #[test]
    fn horizontal_lines_remain_within_one_markdown_paragraph() {
        let blocks = vec![
            detailed_block("first visual line", 0, 0, 100, 10),
            detailed_block("second visual line", 0, 20, 100, 10),
        ];

        assert_eq!(
            PaddleOcrBackend::assemble_block_text(&blocks, false),
            "first visual line\nsecond visual line"
        );
    }

    #[test]
    fn mixed_japanese_layout_keeps_detector_order_and_separators() {
        let mut blocks = vec![
            detailed_block("title", 0, 0, 100, 10),
            detailed_block("right", 50, 20, 10, 100),
            detailed_block("left", 10, 20, 10, 100),
        ];

        let vertical = PaddleOcrBackend::sort_vertical_cjk_blocks(&mut blocks, "japan");

        assert!(!vertical);
        assert_eq!(
            blocks.iter().map(|block| block.block.text.as_str()).collect::<Vec<_>>(),
            ["title", "right", "left"]
        );
        assert_eq!(
            PaddleOcrBackend::assemble_block_text(&blocks, vertical),
            "title\nright\nleft"
        );
    }

    #[test]
    fn non_cjk_vertical_lines_keep_detector_order() {
        let mut blocks = vec![
            detailed_block("left", 10, 0, 10, 100),
            detailed_block("right", 50, 0, 10, 100),
        ];

        let vertical = PaddleOcrBackend::sort_vertical_cjk_blocks(&mut blocks, "en");

        assert!(!vertical);
        assert_eq!(
            blocks.iter().map(|block| block.block.text.as_str()).collect::<Vec<_>>(),
            ["left", "right"]
        );
    }

    #[test]
    fn test_paddle_ocr_backend_creation() {
        let result = PaddleOcrBackend::new();
        assert!(result.is_ok(), "Failed to create PaddleOCR backend");
    }

    #[test]
    fn test_paddle_ocr_backend_with_config() {
        let config = PaddleOcrConfig::default();
        let result = PaddleOcrBackend::with_config(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_effective_rec_batch_size_enforces_bounds() {
        let cases = [
            (0, MIN_RECOGNITION_BATCH_SIZE),
            (PaddleOcrConfig::default().rec_batch_num, DEFAULT_RECOGNITION_BATCH_SIZE),
            (12, 12),
            (u32::MAX, MAX_RECOGNITION_BATCH_SIZE),
        ];

        for (configured, expected) in cases {
            let config = PaddleOcrConfig {
                rec_batch_num: configured,
                ..Default::default()
            };

            assert_eq!(
                PaddleOcrBackend::effective_rec_batch_size(&config),
                expected,
                "unexpected effective recognition batch size for configured value {configured}"
            );
        }
    }

    #[test]
    fn test_unrotated_image_metadata_uses_original_dimensions() {
        let image = image::RgbImage::new(3, 2);
        let outcome = rotate_for_detected_orientation(
            &image,
            crate::doc_orientation::OrientationResult {
                degrees: 0,
                confidence: 1.0,
            },
        )
        .expect("zero-degree orientation should not require a model or fail");

        assert!(outcome.rotated_bytes.is_none());
        assert_eq!((outcome.processed_width, outcome.processed_height), (3, 2));

        let metadata = image_metadata(&outcome);
        assert_eq!(
            metadata.get(crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY),
            Some(&serde_json::json!(3))
        );
        assert_eq!(
            metadata.get(crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY),
            Some(&serde_json::json!(2))
        );
        assert_eq!(
            metadata.get(crate::ocr_metadata_keys::OCR_ORIENTATION_DEGREES_METADATA_KEY),
            Some(&serde_json::json!(0))
        );
        assert!(!metadata.contains_key(crate::ocr_metadata_keys::OCR_AUTO_ROTATED_METADATA_KEY));
    }

    #[test]
    fn test_rotated_image_metadata_and_geometry_use_corrected_space() {
        let mut image = image::RgbImage::new(3, 2);
        let marker = image::Rgb([17, 31, 47]);
        image.put_pixel(0, 0, marker);

        let outcome = rotate_for_detected_orientation(
            &image,
            crate::doc_orientation::OrientationResult {
                degrees: 90,
                confidence: 1.0,
            },
        )
        .expect("in-memory rotation should succeed");

        assert_eq!((outcome.processed_width, outcome.processed_height), (2, 3));
        let rotated = image::load_from_memory(outcome.rotated_bytes.as_deref().expect("rotation should produce bytes"))
            .expect("rotated PNG should decode")
            .to_rgb8();
        assert_eq!(rotated.dimensions(), (2, 3));
        assert_eq!(*rotated.get_pixel(0, 2), marker);

        let metadata = image_metadata(&outcome);
        assert_eq!(
            metadata.get(crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY),
            Some(&serde_json::json!(2))
        );
        assert_eq!(
            metadata.get(crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY),
            Some(&serde_json::json!(3))
        );
        assert_eq!(
            metadata.get(crate::ocr_metadata_keys::OCR_ORIENTATION_DEGREES_METADATA_KEY),
            Some(&serde_json::json!(90))
        );
        assert_eq!(
            metadata.get(crate::ocr_metadata_keys::OCR_AUTO_ROTATED_METADATA_KEY),
            Some(&serde_json::json!(true))
        );
    }

    #[test]
    fn test_paddle_ocr_language_support_direct() {
        let backend = PaddleOcrBackend::new().unwrap();

        assert!(backend.supports_language("ch"));
        assert!(backend.supports_language("en"));
        assert!(backend.supports_language("japan"));
        assert!(backend.supports_language("korean"));
        assert!(backend.supports_language("french"));
        assert!(backend.supports_language("thai"));
        assert!(backend.supports_language("greek"));
    }

    #[test]
    fn test_paddle_ocr_language_support_mapped() {
        let backend = PaddleOcrBackend::new().unwrap();

        assert!(backend.supports_language("chi_sim"));
        assert!(backend.supports_language("eng"));
        assert!(backend.supports_language("jpn"));
        assert!(backend.supports_language("kor"));
        assert!(backend.supports_language("fra"));
        assert!(backend.supports_language("zho"));
        assert!(backend.supports_language("tha"));
        assert!(backend.supports_language("ell"));
        assert!(backend.supports_language("rus"));
    }

    #[test]
    fn test_paddle_ocr_language_unsupported() {
        let backend = PaddleOcrBackend::new().unwrap();

        assert!(!backend.supports_language("xyz"));
        assert!(!backend.supports_language("invalid"));
    }

    #[test]
    fn test_paddle_ocr_plugin_interface() {
        let backend = PaddleOcrBackend::new().unwrap();

        assert_eq!(backend.name(), "paddle-ocr");
        assert!(!backend.version().is_empty());
        assert!(backend.initialize().is_ok());
        assert!(backend.shutdown().is_ok());
    }

    #[test]
    fn test_paddle_ocr_backend_type() {
        let backend = PaddleOcrBackend::new().unwrap();
        assert_eq!(backend.backend_type(), OcrBackendType::PaddleOCR);
    }

    #[test]
    fn test_paddle_ocr_supported_languages() {
        let backend = PaddleOcrBackend::new().unwrap();
        let languages = backend.supported_languages();

        assert!(!languages.is_empty());
        assert!(languages.contains(&"ch".to_string()));
        assert!(languages.contains(&"en".to_string()));
        assert!(languages.contains(&"thai".to_string()));
        assert!(languages.contains(&"greek".to_string()));
    }

    #[test]
    fn test_paddle_ocr_table_detection_disabled_by_default() {
        let backend = PaddleOcrBackend::new().unwrap();
        assert!(
            !backend.supports_table_detection(),
            "paddle has no table-vs-prose discriminator, so enabling it by default fabricates \
             wide garbage tables out of ordinary prose -- measured 390 fabricated rows on a \
             document containing no tables at all"
        );
    }

    #[test]
    fn test_paddle_ocr_table_detection_enabled() {
        let config = PaddleOcrConfig::default().with_table_detection(true);
        let backend = PaddleOcrBackend::with_config(config).unwrap();
        assert!(backend.supports_table_detection());
    }

    #[test]
    fn test_paddle_ocr_default() {
        let backend = PaddleOcrBackend::default();
        assert_eq!(backend.name(), "paddle-ocr");
    }

    #[tokio::test]
    async fn test_paddle_ocr_process_empty_image() {
        let backend = PaddleOcrBackend::new().unwrap();
        let config = OcrConfig {
            backend: "paddle-ocr".to_string(),
            language: vec!["ch".to_string()],
            ..Default::default()
        };

        let result = backend.process_image(&[], &config).await;
        assert!(result.is_err(), "Should error on empty image");
    }

    #[tokio::test]
    async fn test_paddle_ocr_process_image_rejects_invalid_legacy_config() {
        let backend = PaddleOcrBackend::new().unwrap();
        let config = OcrConfig {
            backend: "paddle-ocr".to_string(),
            auto_rotate: true,
            paddle_ocr_config: Some(serde_json::json!({"det_db_thresh": "not a number"})),
            ..Default::default()
        };

        match backend.process_image(b"not an image", &config).await {
            Err(crate::XbergError::Validation { message, .. }) => assert!(
                message.contains("Failed to deserialize paddle_ocr_config"),
                "the page must reject the legacy override: {message}"
            ),
            other => panic!("expected a paddle_ocr_config validation error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn should_process_unknown_legacy_config_keys_past_backend_validation() {
        let backend = PaddleOcrBackend::new().unwrap();
        let config = OcrConfig {
            backend: "paddle-ocr".to_string(),
            paddle_ocr_config: Some(serde_json::json!({
                "model_tier": "server",
                "vendor_extension": true
            })),
            ..Default::default()
        };

        let error = backend
            .process_image(b"not an image", &config)
            .await
            .expect_err("invalid image bytes must still fail after config parsing");
        if let crate::XbergError::Validation { message, .. } = error {
            assert!(
                !message.contains("Failed to deserialize paddle_ocr_config"),
                "unknown legacy keys must not fail backend config parsing: {message}"
            );
        }
    }

    #[test]
    fn test_paddle_ocr_probe_rejects_invalid_legacy_config() {
        let backend = PaddleOcrBackend::new().unwrap();
        let config = OcrConfig {
            backend: "paddle-ocr".to_string(),
            paddle_ocr_config: Some(serde_json::json!({"use_angle_cls": "yes"})),
            ..Default::default()
        };

        let check = backend.probe(&config);

        assert!(matches!(check.status, crate::doctor::ProbeStatus::Fail), "{check:?}");
        assert!(check.message.contains("Failed to deserialize paddle_ocr_config"));
    }

    #[test]
    fn test_paddle_ocr_probe_uses_the_paddle_ocr_settings_override() {
        let backend = PaddleOcrBackend::new().unwrap();
        let empty_cache = tempfile::tempdir().unwrap();
        let config = OcrConfig {
            backend: "paddle-ocr".to_string(),
            paddle_ocr_settings: Some(PaddleOcrConfig::new("en").with_cache_dir(empty_cache.path().to_path_buf())),
            ..Default::default()
        };

        let check = backend.probe(&config);

        assert!(
            matches!(check.status, crate::doctor::ProbeStatus::Skip),
            "the probe must look for models in the override's cache dir: {check:?}"
        );
    }

    #[test]
    fn test_internal_document_from_text_blocks() {
        use crate::ocr::conversion::text_block_to_element;
        use crate::types::extraction::BoundingBox;
        use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
        use crate::types::ocr_elements::OcrElementLevel;

        let blocks = [
            xberg_paddle_ocr::TextBlock {
                text: "Hello World".to_string(),
                box_points: vec![
                    xberg_paddle_ocr::Point { x: 10, y: 10 },
                    xberg_paddle_ocr::Point { x: 200, y: 10 },
                    xberg_paddle_ocr::Point { x: 200, y: 50 },
                    xberg_paddle_ocr::Point { x: 10, y: 50 },
                ],
                box_score: 0.95,
                text_score: 0.92,
                angle_index: 0,
                angle_score: 0.99,
            },
            xberg_paddle_ocr::TextBlock {
                text: "Second line".to_string(),
                box_points: vec![
                    xberg_paddle_ocr::Point { x: 10, y: 60 },
                    xberg_paddle_ocr::Point { x: 300, y: 60 },
                    xberg_paddle_ocr::Point { x: 300, y: 100 },
                    xberg_paddle_ocr::Point { x: 10, y: 100 },
                ],
                box_score: 0.88,
                text_score: 0.85,
                angle_index: 0,
                angle_score: 0.97,
            },
        ];

        let ocr_elements: Vec<OcrElement> = blocks
            .iter()
            .map(|block| text_block_to_element(block, 1))
            .filter_map(|result| result.transpose())
            .collect::<crate::Result<Vec<_>>>()
            .expect("text_block_to_element should succeed");

        assert_eq!(ocr_elements.len(), 2, "Should produce 2 OcrElements");

        let mut doc = InternalDocument::new("pdf");
        for elem in &ocr_elements {
            let (left, top, width, height) = elem.geometry.to_aabb();
            let bbox = BoundingBox {
                x0: left as f64,
                y0: top as f64,
                x1: (left + width) as f64,
                y1: (top + height) as f64,
            };
            let mut ie = InternalElement::text(
                ElementKind::OcrText {
                    level: OcrElementLevel::Line,
                },
                &elem.text,
                0,
            )
            .with_page(elem.page_number);
            ie.bbox = Some(bbox);
            ie.ocr_confidence = Some(elem.confidence.clone());
            ie.ocr_geometry = Some(elem.geometry.clone());
            doc.push_element(ie);
        }

        for ie in &doc.elements {
            assert!(
                matches!(
                    ie.kind,
                    ElementKind::OcrText {
                        level: OcrElementLevel::Line
                    }
                ),
                "Element kind should be OcrText with Line level"
            );
        }

        let first_bbox = doc.elements[0].bbox.as_ref().expect("First element should have bbox");
        assert_eq!(first_bbox.x0, 10.0, "left should be min x of quad points");
        assert_eq!(first_bbox.y0, 10.0, "top should be min y of quad points");
        assert_eq!(first_bbox.x1, 200.0, "right should be left + width");
        assert_eq!(first_bbox.y1, 50.0, "bottom should be top + height");

        let second_bbox = doc.elements[1].bbox.as_ref().expect("Second element should have bbox");
        assert_eq!(second_bbox.x0, 10.0);
        assert_eq!(second_bbox.y0, 60.0);
        assert_eq!(second_bbox.x1, 300.0);
        assert_eq!(second_bbox.y1, 100.0);

        let first_conf = doc.elements[0]
            .ocr_confidence
            .as_ref()
            .expect("First element should have confidence");
        assert!(
            (first_conf.detection.unwrap() - 0.95).abs() < 1e-6,
            "Detection confidence should be ~0.95, got {}",
            first_conf.detection.unwrap()
        );
        assert!(
            (first_conf.recognition - 0.92).abs() < 1e-6,
            "Recognition confidence should be ~0.92, got {}",
            first_conf.recognition
        );

        assert_eq!(doc.elements[0].page, Some(1));
        assert_eq!(doc.elements[1].page, Some(1));
    }

    #[cfg(feature = "paddle-ocr-ort")]
    #[test]
    fn paddle_acceleration_guard_restores_worker_state() {
        use crate::core::config::AccelerationConfig;

        let cpu_accel = AccelerationConfig {
            provider: crate::core::config::acceleration::ExecutionProviderType::Cpu,
            device_id: 0,
        };
        let cuda_accel = AccelerationConfig {
            provider: crate::core::config::acceleration::ExecutionProviderType::Cuda,
            device_id: 1,
        };
        PADDLE_TL_ACCEL.with(|cell| {
            cell.replace(Some(cpu_accel.clone()));
        });

        {
            let _guard = PaddleAccelerationGuard::set(Some(cuda_accel));
            let provider = PADDLE_TL_ACCEL.with(|cell| cell.borrow().as_ref().map(|config| config.provider.clone()));
            assert_eq!(
                provider,
                Some(crate::core::config::acceleration::ExecutionProviderType::Cuda)
            );
        }

        let restored = PADDLE_TL_ACCEL.with(|cell| cell.borrow().clone());
        assert_eq!(
            restored,
            Some(cpu_accel),
            "blocking-pool threads must not retain another request's acceleration"
        );

        PADDLE_TL_ACCEL.with(|cell| {
            cell.replace(None);
        });
    }

    /// Builds a `HocrWord` at the given pixel geometry, purely for clustering/reconstruction
    /// tests below — text content is irrelevant to region splitting.
    fn hocr_word_at(text: &str, left: u32, top: u32, width: u32, height: u32) -> crate::table_core::HocrWord {
        crate::table_core::HocrWord {
            text: text.to_string(),
            left,
            top,
            width,
            height,
            confidence: 90.0,
        }
    }

    /// Defect regression: the `Table` PaddleOCR emits must carry a real `bounding_box`
    /// derived from the words that formed the table region, not the hardcoded `None` the
    /// unfixed code pushed regardless of geometry.
    ///
    /// Without the fix, `Table.bounding_box` is unconditionally `None`, so this test's
    /// equality assertion fails: `left` is `None` instead of
    /// `Some(BoundingBox { x0: 0.0, y0: 500.0, x1: 240.0, y1: 535.0 })`. That `None` is exactly
    /// why nothing downstream (`pdf::structure::pipeline::table_bboxes_by_page` /
    /// `filter_segments_by_table_bboxes`) can suppress the prose the table duplicates.
    #[test]
    fn build_ocr_tables_from_words_populates_bounding_box_from_region_extents() {
        let words = vec![
            // 2 rows x 3 cols: left=0..240, top=500..535 in pixel space.
            hocr_word_at("B11", 0, 500, 40, 15),
            hocr_word_at("B12", 100, 500, 40, 15),
            hocr_word_at("B13", 200, 500, 40, 15),
            hocr_word_at("B21", 0, 520, 40, 15),
            hocr_word_at("B22", 100, 520, 40, 15),
            hocr_word_at("B23", 200, 520, 40, 15),
        ];

        let built = PaddleOcrBackend::build_ocr_tables_from_words(&words);

        assert_eq!(
            built.tables.len(),
            1,
            "the 3x2 grid should be detected as exactly one table"
        );
        let table = &built.tables[0];
        assert_eq!(
            table.bounding_box,
            Some(crate::types::extraction::BoundingBox {
                x0: 0.0,
                y0: 500.0,
                x1: 240.0,
                y1: 535.0,
            }),
            "bounding_box must be derived from the region's word extents, not left None"
        );
    }

    #[test]
    fn table_rejection_warning_reports_structurally_rejected_candidates() {
        assert!(PaddleOcrBackend::table_rejection_warning(0).is_none());

        let warning = PaddleOcrBackend::table_rejection_warning(2).expect("rejected candidates must be reported");
        assert_eq!(warning.source, "paddle-ocr-table-detection");
        assert_eq!(
            warning.message,
            "PaddleOCR rejected 2 table candidate region(s) during structural validation"
        );
    }

    fn paddle_element_at(text: &str, level: OcrElementLevel, left: u32, top: u32, width: u32) -> OcrElement {
        OcrElement::new(
            text,
            crate::types::OcrBoundingGeometry::Rectangle {
                left,
                top,
                width,
                height: 15,
            },
            crate::types::OcrConfidence::from_paddle(0.9, 0.9),
        )
        .with_level(level)
    }

    /// The lines a detected table carries must not also stay in the page document (#1571), or the
    /// standalone-image route renders every row twice: once per line, then as the table.
    #[test]
    fn build_ocr_document_drops_lines_a_detected_table_carries() {
        let rows = [
            ("Energy", "186", "372"),
            ("Fat", "1,5", "3,0"),
            ("Protein", "3,0", "6,0"),
            ("Salt", "0,1", "0,2"),
        ];
        let mut lines = vec![paddle_element_at("Nutrition facts", OcrElementLevel::Line, 0, 0, 150)];
        let mut words = vec![
            paddle_element_at("Nutrition", OcrElementLevel::Word, 0, 0, 90),
            paddle_element_at("facts", OcrElementLevel::Word, 100, 0, 50),
        ];
        for (row, (label, per_100, per_serving)) in rows.iter().enumerate() {
            let top = 200 + row as u32 * 25;
            let line = format!("{label} {per_100} {per_serving}");
            lines.push(paddle_element_at(&line, OcrElementLevel::Line, 0, top, 340));
            words.push(paddle_element_at(label, OcrElementLevel::Word, 0, top, 80));
            words.push(paddle_element_at(per_100, OcrElementLevel::Word, 200, top, 40));
            words.push(paddle_element_at(per_serving, OcrElementLevel::Word, 300, top, 40));
        }
        let table_elements = lines.iter().chain(&words).cloned().collect::<Vec<_>>();
        let built = PaddleOcrBackend::build_ocr_tables_from_words(&elements_to_hocr_words(&table_elements, 0.3));
        assert_eq!(
            built.tables.len(),
            1,
            "the nutrition grid must be detected as one table"
        );

        let doc = PaddleOcrBackend::build_ocr_document(&lines, &built.tables);

        let texts: Vec<&str> = doc.elements.iter().map(|element| element.text.as_str()).collect();
        assert_eq!(texts, ["Nutrition facts"], "only the line outside the table may remain");
    }

    /// Two tables stacked vertically with a large blank gap between them must become two
    /// separate regions, not one region spanning both.
    ///
    /// Without the fix, paddle's table path called `reconstruct_table` once over every
    /// table-confidence word on the whole page (`table_count` hardcoded to at most 1), so the
    /// words from both tables below would be reconstructed together, and this test's row-count
    /// assertion on the *first* region would fail (it would see all 8 words merged into one
    /// region with 4 rows instead of two 2-row regions).
    #[test]
    fn cluster_words_into_table_regions_splits_two_vertically_separated_tables() {
        let words = vec![
            // Table 1: 2 rows x 2 cols, rows at y=0 and y=20, height 15.
            hocr_word_at("A1", 0, 0, 40, 15),
            hocr_word_at("B1", 100, 0, 40, 15),
            hocr_word_at("A2", 0, 20, 40, 15),
            hocr_word_at("B2", 100, 20, 40, 15),
            // Large vertical gap (well beyond 3x avg word height of 15).
            hocr_word_at("C1", 0, 500, 40, 15),
            hocr_word_at("D1", 100, 500, 40, 15),
            hocr_word_at("C2", 0, 520, 40, 15),
            hocr_word_at("D2", 100, 520, 40, 15),
        ];

        let regions = crate::table_core::cluster_words_into_table_regions(&words);

        assert_eq!(regions.len(), 2, "a wide vertical gap must split into two regions");
        assert_eq!(regions[0].len(), 4, "first region should contain only table 1's words");
        assert_eq!(regions[1].len(), 4, "second region should contain only table 2's words");
    }

    /// Two sub-tables placed side by side, sharing the same row y-positions, must land in a
    /// *single* region (and therefore a single reconstructed table spanning both), because
    /// there is no vertical gap between them for the region split to key off of. This is the
    /// newspaper-stock-table shape from `test_documents/images_extra/ocr_image.tiff`.
    ///
    /// Without the fix (whole-page `reconstruct_table` call, no clustering at all) this case
    /// happened to already work by accident, since one big call also keeps them together — the
    /// case that actually distinguishes "region clustering exists" is the vertically-separated
    /// test above. This test guards against a *wrong* clustering fix that keys off horizontal
    /// position (which would wrongly split side-by-side tables into two regions and defeat the
    /// merged-shared-row reconstruction).
    #[test]
    fn cluster_words_into_table_regions_keeps_side_by_side_tables_in_one_region() {
        let words = vec![
            // Left sub-table columns at x=0, x=60. Right sub-table columns at x=300, x=360.
            hocr_word_at("L1", 0, 0, 40, 15),
            hocr_word_at("L2", 60, 0, 40, 15),
            hocr_word_at("R1", 300, 0, 40, 15),
            hocr_word_at("R2", 360, 0, 40, 15),
            hocr_word_at("L3", 0, 20, 40, 15),
            hocr_word_at("L4", 60, 20, 40, 15),
            hocr_word_at("R3", 300, 20, 40, 15),
            hocr_word_at("R4", 360, 20, 40, 15),
        ];

        let regions = crate::table_core::cluster_words_into_table_regions(&words);

        assert_eq!(
            regions.len(),
            1,
            "side-by-side sub-tables sharing rows must stay in one region"
        );
        assert_eq!(regions[0].len(), 8);
    }

    /// End-to-end reconstruction over the side-by-side case: `reconstruct_table` must detect
    /// all four columns (two per sub-table) and both shared rows, producing one merged table
    /// rather than dropping either sub-table's columns.
    #[test]
    fn reconstruct_table_merges_side_by_side_subtables_onto_shared_rows() {
        let words = vec![
            hocr_word_at("L1", 0, 0, 40, 15),
            hocr_word_at("L2", 60, 0, 40, 15),
            hocr_word_at("R1", 300, 0, 40, 15),
            hocr_word_at("R2", 360, 0, 40, 15),
            hocr_word_at("L3", 0, 20, 40, 15),
            hocr_word_at("L4", 60, 20, 40, 15),
            hocr_word_at("R3", 300, 20, 40, 15),
            hocr_word_at("R4", 360, 20, 40, 15),
        ];

        let regions = crate::table_core::cluster_words_into_table_regions(&words);
        assert_eq!(regions.len(), 1);

        let cells = reconstruct_table(&regions[0], 20, 0.5);
        assert_eq!(cells.len(), 2, "both shared rows should be reconstructed");
        assert_eq!(
            cells[0].len(),
            4,
            "all four columns from both sub-tables should be detected"
        );
    }

    /// A page-scale defect test: two real tables of *differing* shapes (2 cols vs 3 cols), far
    /// apart vertically, plus a small cluster of stray words below the noise threshold. This
    /// exercises the whole loop the fix adds in `process_image` (region splitting + per-region
    /// `MIN_TABLE_CANDIDATE_WORDS` filtering), not just the clustering helper in isolation.
    ///
    /// Without the fix, all three clusters' words would be fed through one whole-page
    /// `reconstruct_table` call and reported as `table_count == 1` with row/col counts describing
    /// a single nonsensical merged grid, rather than 2 real tables and the stray cluster dropped.
    #[test]
    fn table_region_pipeline_reports_two_tables_of_differing_shape_and_drops_noise() {
        let mut words = vec![
            // Table A: 3 rows x 2 cols (6 words, exactly at MIN_TABLE_CANDIDATE_WORDS so it
            // survives the noise gate).
            hocr_word_at("A11", 0, 0, 40, 15),
            hocr_word_at("A12", 100, 0, 40, 15),
            hocr_word_at("A21", 0, 20, 40, 15),
            hocr_word_at("A22", 100, 20, 40, 15),
            hocr_word_at("A31", 0, 40, 40, 15),
            hocr_word_at("A32", 100, 40, 40, 15),
        ];
        // Table B: 2 rows x 3 cols, far below table A.
        words.extend([
            hocr_word_at("B11", 0, 500, 40, 15),
            hocr_word_at("B12", 100, 500, 40, 15),
            hocr_word_at("B13", 200, 500, 40, 15),
            hocr_word_at("B21", 0, 520, 40, 15),
            hocr_word_at("B22", 100, 520, 40, 15),
            hocr_word_at("B23", 200, 520, 40, 15),
        ]);
        // Stray noise cluster: only 2 words, far below both tables, must be dropped by the
        // MIN_TABLE_CANDIDATE_WORDS gate.
        words.extend([
            hocr_word_at("N1", 0, 1000, 40, 15),
            hocr_word_at("N2", 100, 1000, 40, 15),
        ]);

        let mut reconstructed_tables: Vec<Vec<Vec<String>>> = Vec::new();
        for region_words in crate::table_core::cluster_words_into_table_regions(&words) {
            if region_words.len() < crate::table_core::MIN_TABLE_CANDIDATE_WORDS {
                continue;
            }
            let cells = reconstruct_table(&region_words, 20, 0.5);
            if cells.is_empty() || cells[0].is_empty() {
                continue;
            }
            reconstructed_tables.push(cells);
        }

        assert_eq!(
            reconstructed_tables.len(),
            2,
            "exactly two tables should survive: A and B, with the stray pair dropped"
        );
        assert_eq!(reconstructed_tables[0].len(), 3, "table A should have 3 rows");
        assert_eq!(reconstructed_tables[0][0].len(), 2, "table A should have 2 columns");
        assert_eq!(reconstructed_tables[1].len(), 2, "table B should have 2 rows");
        assert_eq!(reconstructed_tables[1][0].len(), 3, "table B should have 3 columns");
    }

    /// Real (text, left, top, width, height) word geometry captured from PaddleOCR word-level
    /// output on page 1 of `test_documents/pdf_scanned/ordinance_2197_scanned.pdf` (a 16-page
    /// municipal ordinance with zero real tables; see
    /// `test_documents/ground_truth/pdf/ordinance_2197.txt`, which has zero pipe rows). All 355
    /// words recognised on the page are included: unfiltered, they cluster into one region (no
    /// gap in a page of continuous body text exceeds the region-split threshold) and
    /// `reconstruct_table` fabricates a 24-row x 36-column grid from it (measured directly from
    /// this fixture), because ordinary paragraph prose has no genuine repeated column structure --
    /// each line's word x-positions rarely coincide with any other line's under the paddle table
    /// path's 20px column threshold.
    #[cfg(feature = "pdf")]
    const PROSE_PAGE_WORDS: &[(&str, u32, u32, u32, u32)] = &[
        ("district", 260, 134, 40, 75),
        ("zonn", 373, 134, 41, 60),
        ("zoning", 403, 134, 39, 70),
        ("comprehensive", 748, 134, 40, 156),
        ("located", 460, 136, 41, 73),
        ("hereby", 547, 136, 42, 67),
        ("and", 839, 136, 31, 38),
        ("SOUTHEAST", 1296, 136, 41, 154),
        ("ordinance:", 175, 137, 33, 109),
        ("Texas.", 347, 137, 34, 67),
        ("Development", 1150, 137, 45, 137),
        ("Parkway", 1181, 137, 40, 86),
        ("AN", 1412, 137, 44, 39),
        ("requested", 865, 138, 33, 93),
        ("public", 953, 138, 33, 66),
        ("City", 1065, 138, 41, 41),
        ("land", 1212, 138, 39, 44),
        ("PLAN", 1326, 138, 41, 65),
        ("A", 430, 141, 41, 12),
        ("THEREFORE:", 725, 141, 33, 152),
        ("PROVIDING", 1385, 142, 40, 143),
        ("DISTRICT", 1357, 144, 39, 120),
        ("attached", 430, 161, 41, 81),
        ("ORDINANCE", 1411, 188, 45, 150),
        ("located", 1212, 190, 39, 70),
        ("Council,", 1065, 194, 41, 82),
        ("dsrt", 373, 209, 41, 74),
        ("hearing", 953, 209, 33, 72),
        ("Se", 285, 210, 44, 26),
        ("declared", 546, 211, 42, 87),
        ("Section1.That", 576, 211, 40, 173),
        ("classification.", 257, 212, 42, 140),
        ("at", 460, 212, 41, 19),
        ("Section", 489, 212, 41, 80),
        ("e,", 977, 214, 44, 39),
        ("Section", 201, 216, 40, 72),
        ("district", 403, 219, 39, 70),
        ("WHEREAS,", 1096, 219, 41, 122),
        ("WHEREAS,", 779, 220, 41, 121),
        ("WHEREAS,", 1239, 221, 41, 123),
        ("WHEREAS,", 1007, 222, 45, 122),
        ("WHEREAS,", 891, 225, 45, 124),
        ("FOR", 1326, 228, 41, 52),
        ("and", 1181, 238, 40, 32),
        ("the", 460, 240, 41, 25),
        ("zoning", 865, 242, 33, 66),
        ("hereto", 430, 257, 41, 60),
        ("n", 977, 258, 44, 17),
        ("on", 285, 268, 44, 19),
        ("southeast", 460, 274, 41, 96),
        ("within", 1211, 274, 40, 64),
        ("TO", 1357, 274, 39, 29),
        ("recommending", 1064, 277, 42, 152),
        ("classifcaon", 372, 284, 42, 135),
        ("(PD)", 1150, 284, 44, 48),
        ("Creek", 1180, 284, 41, 60),
        ("h", 977, 287, 44, 18),
        ("on", 953, 292, 33, 22),
        ("4.", 201, 296, 40, 18),
        ("3.That", 285, 298, 44, 78),
        ("classification", 402, 298, 40, 129),
        ("APPROXIMATELY", 1325, 298, 42, 223),
        ("true", 545, 300, 42, 39),
        ("CORNER", 1296, 300, 41, 106),
        ("2.", 489, 301, 41, 19),
        ("FOR", 1384, 302, 41, 50),
        ("plan", 748, 304, 40, 47),
        ("PLANNED", 1356, 313, 40, 120),
        ("change", 865, 319, 33, 66),
        ("such", 953, 320, 33, 44),
        ("and", 430, 325, 41, 32),
        ("That", 201, 329, 40, 45),
        ("That", 489, 335, 41, 46),
        ("District", 1149, 335, 45, 78),
        ("me", 976, 339, 44, 25),
        ("the", 1211, 339, 39, 31),
        ("the", 1007, 347, 44, 34),
        ("and", 545, 348, 41, 39),
        ("the", 1096, 349, 41, 33),
        ("the", 778, 350, 42, 33),
        ("OF", 1411, 350, 44, 32),
        ("the", 1239, 351, 41, 27),
        ("the", 891, 358, 45, 36),
        ("Bend", 1180, 359, 41, 45),
        ("and", 747, 365, 40, 33),
        ("incorporated", 429, 366, 42, 128),
        ("corner", 460, 372, 41, 67),
        ("requested", 953, 375, 33, 94),
        ("A", 1384, 376, 40, 16),
        ("the", 284, 379, 45, 33),
        ("BE", 662, 379, 41, 31),
        ("the", 200, 382, 41, 32),
        ("nd", 976, 383, 44, 25),
        ("City", 1211, 385, 39, 37),
        ("current", 1238, 386, 42, 74),
        ("THE", 1411, 387, 44, 54),
        ("OF", 637, 389, 33, 32),
        ("the", 489, 390, 41, 33),
        ("City", 1095, 390, 41, 40),
        ("City", 1007, 391, 44, 42),
        ("the", 576, 393, 40, 24),
        ("correct.", 544, 395, 42, 80),
        ("and", 865, 396, 33, 33),
        ("City", 778, 398, 41, 39),
        ("City", 891, 403, 44, 42),
        ("now", 747, 412, 40, 40),
        ("IT", 662, 413, 41, 31),
        ("OF", 1296, 416, 41, 31),
        ("CHANGE", 1384, 416, 40, 103),
        ("Drive,", 1180, 419, 40, 66),
        ("m", 976, 420, 44, 17),
        ("following", 200, 422, 40, 99),
        ("City's", 284, 423, 44, 48),
        ("Final", 1149, 423, 44, 48),
        ("THE", 637, 427, 33, 49),
        ("zoning", 489, 431, 41, 67),
        ("of", 1211, 431, 39, 24),
        ("facts", 575, 433, 41, 46),
        ("nder", 372, 434, 41, 47),
        ("approval", 1064, 437, 41, 88),
        ("Planning", 1095, 438, 41, 88),
        ("the", 865, 440, 33, 27),
        ("of", 459, 441, 41, 18),
        ("Planning", 1006, 442, 45, 86),
        ("to", 402, 443, 40, 18),
        ("DEVELOPMENT", 1355, 443, 40, 198),
        ("CITY", 1411, 446, 44, 61),
        ("ORDAINED", 661, 447, 42, 133),
        ("Council", 777, 453, 42, 80),
        ("Planning", 890, 454, 45, 88),
        ("LAKE", 1295, 457, 41, 65),
        ("Sugar", 1211, 463, 39, 57),
        ("e", 976, 464, 44, 17),
        ("property", 1238, 468, 41, 81),
        ("Lake", 459, 469, 41, 45),
        ("deems", 747, 473, 40, 67),
        ("Planned", 402, 475, 39, 78),
        ("same", 865, 478, 33, 49),
        ("zoning", 953, 480, 33, 60),
        ("official", 284, 482, 44, 70),
        ("the", 372, 482, 41, 33),
        ("Development", 1147, 482, 45, 129),
        ("and", 575, 488, 41, 31),
        ("CITY", 637, 488, 33, 60),
        ("be", 1180, 494, 40, 18),
        ("herein", 429, 502, 41, 60),
        ("ORDINANCE", 1474, 502, 32, 148),
        ("district", 488, 506, 42, 74),
        ("n", 975, 508, 45, 18),
        ("COUNCIL", 1410, 519, 45, 113),
        ("rezoned", 1179, 521, 41, 80),
        ("comprehensive", 371, 523, 41, 149),
        ("Pointe", 459, 524, 41, 60),
        ("recitations", 575, 528, 40, 108),
        ("Land", 1210, 528, 39, 51),
        ("POINTE", 1295, 532, 41, 93),
        ("0.7906", 1325, 532, 41, 79),
        ("is", 865, 533, 33, 22),
        ("of", 1064, 533, 40, 19),
        ("and", 1094, 534, 42, 40),
        ("Exhibits", 199, 536, 41, 85),
        ("ae", 975, 537, 44, 47),
        ("and", 1006, 538, 44, 34),
        ("finds", 777, 542, 41, 46),
        ("OF", 1383, 543, 40, 29),
        ("it", 747, 547, 39, 20),
        ("change;", 953, 551, 33, 78),
        ("owner", 1237, 557, 42, 61),
        ("OF", 637, 559, 33, 33),
        ("and", 890, 559, 44, 36),
        ("herein", 865, 560, 33, 61),
        ("the", 1063, 560, 41, 32),
        ("zoning", 283, 563, 45, 63),
        ("Development", 401, 569, 40, 131),
        ("by", 429, 571, 41, 25),
        ("appropriate", 746, 580, 40, 114),
        ("classification", 488, 582, 41, 136),
        ("Zoning", 1005, 582, 45, 71),
        ("Zoning", 1094, 582, 41, 67),
        ("(the", 1210, 587, 39, 37),
        ("BY", 661, 591, 41, 30),
        ("Parkway", 459, 592, 41, 87),
        ("rezoning", 1063, 594, 41, 87),
        ("ZONING", 1383, 596, 40, 96),
        ("SUGAR", 637, 598, 33, 82),
        ("that", 777, 604, 41, 39),
        ("Zoning", 889, 604, 45, 73),
        ("reference,", 429, 605, 41, 100),
        ("from", 1179, 616, 41, 46),
        ("Plan;", 1147, 621, 44, 48),
        ("has", 1237, 625, 41, 34),
        ("ACRES", 1325, 627, 40, 85),
        ("are", 199, 629, 40, 31),
        ("THE", 661, 632, 41, 51),
        ("incorporated", 865, 632, 33, 126),
        ("c", 975, 633, 44, 18),
        ("and", 953, 634, 33, 39),
        ("PARKWAY", 1295, 635, 41, 126),
        ("map", 283, 636, 44, 41),
        ("\"City\"),", 1210, 639, 39, 76),
        ("set", 574, 644, 41, 33),
        ("OF", 1410, 644, 44, 33),
        ("(PD)", 1355, 645, 39, 55),
        ("the", 776, 652, 42, 32),
        ("NO.", 1474, 661, 32, 47),
        ("Commission", 1004, 663, 45, 123),
        ("Commission", 1093, 665, 42, 121),
        ("requested", 1236, 667, 42, 95),
        ("attached", 198, 669, 41, 85),
        ("que", 974, 677, 45, 62),
        ("Business", 1178, 677, 41, 88),
        ("zoning", 370, 679, 42, 68),
        ("forth", 574, 679, 41, 53),
        ("and", 1147, 679, 44, 34),
        ("be", 283, 680, 44, 27),
        ("THE", 1409, 681, 45, 54),
        ("Commission", 888, 686, 45, 127),
        ("and", 459, 688, 40, 31),
        ("request;", 1063, 689, 40, 87),
        ("LAND,", 637, 691, 33, 82),
        ("CITY", 660, 693, 42, 58),
        ("zoning", 776, 693, 41, 66),
        ("is", 429, 707, 41, 26),
        ("to", 746, 707, 40, 20),
        ("FROM", 1382, 709, 41, 70),
        ("2197", 1474, 709, 32, 53),
        ("amended", 282, 710, 45, 92),
        ("DISTRICT", 1354, 710, 40, 114),
        ("(PD)", 401, 715, 40, 46),
        ("at", 1209, 723, 40, 18),
        ("of", 488, 726, 41, 19),
        ("Creek", 459, 729, 40, 52),
        ("OF", 1325, 730, 40, 30),
        ("changed", 428, 734, 42, 87),
        ("in", 574, 740, 41, 19),
        ("CITY", 1409, 740, 44, 62),
        ("make", 746, 747, 39, 52),
        ("ordiance", 370, 748, 41, 101),
        ("the", 1209, 750, 39, 30),
        ("by", 974, 751, 44, 31),
        ("approximately", 487, 753, 42, 143),
        ("COUNCIL", 660, 761, 41, 113),
        ("and", 865, 764, 33, 38),
        ("the", 574, 768, 41, 32),
        ("hereto", 198, 769, 40, 58),
        ("request", 775, 769, 42, 80),
        ("that", 1236, 769, 41, 41),
        ("AND", 1294, 771, 42, 51),
        ("TEXAS:", 637, 774, 33, 93),
        ("District", 401, 775, 40, 74),
        ("LAND", 1324, 776, 41, 72),
        ("and", 1063, 777, 40, 39),
        ("Office", 1178, 780, 41, 61),
        ("l", 974, 781, 44, 17),
        ("Southeast", 1209, 789, 39, 96),
        ("forwarded", 1093, 795, 41, 101),
        ("Bend", 458, 796, 41, 46),
        ("and", 1004, 796, 44, 34),
        ("to", 282, 805, 44, 19),
        ("preamble", 573, 809, 42, 88),
        ("approximately", 1235, 810, 42, 150),
        ("BUSINESS", 1381, 810, 41, 116),
        ("such", 746, 813, 39, 46),
        ("made", 865, 813, 33, 49),
        ("OF", 1409, 814, 44, 32),
        ("recommended", 887, 823, 45, 142),
        ("from", 428, 830, 41, 39),
        ("an", 974, 832, 44, 25),
        ("CREEK", 1294, 833, 41, 85),
        ("FINAL", 1354, 833, 39, 75),
        ("reflect", 282, 835, 44, 63),
        ("the", 1003, 840, 45, 34),
        ("and", 198, 842, 40, 32),
        ("(B-O)", 1178, 850, 41, 60),
        ("SUGAR", 1408, 850, 45, 91),
        ("Drive,", 458, 851, 41, 59),
        ("of", 370, 857, 41, 19),
        ("complies", 775, 858, 41, 87),
        ("Final", 400, 863, 40, 52),
        ("a", 865, 868, 33, 11),
        ("LOCATED", 1324, 871, 41, 118),
        ("zoning", 745, 872, 40, 72),
        ("the", 370, 877, 41, 33),
        ("City", 1003, 884, 44, 42),
        ("Business", 428, 885, 41, 93),
        ("incorporated", 197, 888, 41, 125),
        ("part", 865, 890, 33, 38),
        ("corner", 1208, 893, 40, 63),
        ("pable", 973, 898, 45, 84),
        ("this", 281, 901, 45, 41),
        ("0.7906", 487, 904, 41, 67),
        ("its", 1093, 905, 41, 25),
        ("of", 573, 906, 41, 26),
        ("Cty", 369, 918, 42, 40),
        ("described", 458, 918, 40, 93),
        ("District", 1177, 919, 41, 74),
        ("Development", 400, 924, 40, 134),
        ("DEVELOPMENT", 1353, 924, 40, 193),
        ("BEND", 1294, 928, 41, 65),
        ("of", 865, 934, 33, 22),
        ("Council", 1002, 935, 45, 79),
        ("final", 1092, 939, 42, 46),
        ("the", 573, 941, 41, 26),
        ("change", 281, 945, 44, 71),
        ("LAND,", 1408, 946, 44, 84),
        ("OFFICE", 1381, 950, 40, 89),
        ("change;", 745, 957, 39, 80),
        ("with", 774, 960, 42, 46),
        ("this", 865, 961, 33, 38),
        ("0.7906", 1235, 961, 41, 68),
        ("of", 1208, 964, 39, 25),
        ("of", 369, 966, 41, 20),
        ("granting", 886, 975, 45, 81),
        ("ordinance", 573, 976, 41, 102),
        ("rezoneord", 78, 982, 33, 56),
        ("Office", 428, 987, 41, 59),
        ("acres", 487, 987, 41, 46),
        ("Sugar", 369, 993, 41, 54),
        ("3/16/20", 69, 994, 23, 44),
        ("r", 973, 994, 44, 17),
        ("report", 1092, 994, 41, 60),
        ("Lake", 1208, 997, 39, 44),
        ("to", 1177, 1002, 41, 18),
        ("DRIVE.", 1294, 1003, 41, 92),
        ("ordinance;", 865, 1005, 33, 110),
        ("in", 458, 1012, 40, 25),
        ("AT", 1324, 1012, 40, 31),
        ("the", 774, 1015, 41, 25),
        ("have", 1002, 1016, 44, 49),
        ("in", 281, 1019, 44, 19),
        ("into", 197, 1022, 40, 38),
        ("TEXAS,", 1408, 1034, 44, 89),
        ("Planned", 1177, 1037, 41, 74),
        ("acres", 1235, 1037, 41, 54),
        ("of", 487, 1041, 41, 26),
        ("Exhibit", 458, 1045, 40, 73),
        ("zoning", 281, 1048, 44, 71),
        ("NOW,", 745, 1049, 39, 73),
        ("Land,", 369, 1055, 41, 60),
        ("(B-O)", 428, 1055, 41, 60),
        ("to", 1092, 1055, 41, 26),
        ("Pointe", 1208, 1055, 39, 64),
        ("City's", 774, 1056, 41, 67),
        ("(B-O)", 1381, 1056, 40, 63),
        ("THE", 1324, 1066, 40, 51),
        ("e,", 973, 1067, 44, 39),
        ("land", 487, 1069, 41, 46),
        ("Plan", 400, 1073, 40, 40),
        ("such", 886, 1074, 45, 44),
        ("this", 197, 1075, 40, 38),
        ("each", 1002, 1075, 44, 42),
        ("are", 573, 1088, 41, 32),
        ("the", 1092, 1090, 41, 32),
        ("of", 1235, 1098, 41, 27),
        ("a", 973, 1105, 44, 17),
    ];

    /// Real (text, left, top, width, height) word geometry captured from PaddleOCR word-level
    /// output on `test_documents/images_extra/ocr_image.tiff` (a scanned newspaper stock table;
    /// see `test_documents/ground_truth/tiff/image_tiff.md`, 16 rows x 10 cols, 154 non-empty
    /// cells, dense and numeric) -- the cleanest genuine-table fixture available. All 215
    /// recognised words are included so the two side-by-side sub-tables reconstruct with their
    /// real column/row structure intact.
    #[cfg(feature = "pdf")]
    const DENSE_TABLE_WORDS: &[(&str, u32, u32, u32, u32)] = &[
        ("Nasdaq", 48, 72, 185, 84),
        ("&", 251, 74, 36, 83),
        ("AMEX", 305, 75, 159, 84),
        ("Stocks", 44, 152, 52, 32),
        ("in", 102, 153, 20, 32),
        ("bold", 128, 153, 37, 33),
        ("rose", 171, 154, 37, 33),
        ("or", 214, 155, 15, 32),
        ("fell", 230, 156, 37, 32),
        ("5%", 267, 156, 22, 33),
        ("or", 295, 157, 21, 32),
        ("more", 322, 158, 43, 32),
        ("USA", 78, 194, 40, 32),
        ("Track", 136, 188, 44, 34),
        ("your", 192, 189, 39, 34),
        ("investments", 237, 190, 111, 35),
        ("with", 360, 193, 38, 33),
        ("our", 404, 194, 26, 33),
        ("continuously", 442, 195, 113, 34),
        ("TODAY", 48, 218, 73, 29),
        ("updated", 135, 215, 73, 33),
        ("stocks.", 215, 216, 63, 33),
        ("Visit", 285, 217, 41, 33),
        ("us", 333, 218, 14, 32),
        ("on", 360, 218, 14, 33),
        ("the", 387, 219, 24, 32),
        ("web", 424, 220, 30, 32),
        ("at", 467, 220, 14, 32),
        (".com", 70, 235, 51, 30),
        ("money.usatoday.com", 137, 238, 197, 37),
        ("52-week", 41, 272, 48, 19),
        ("52-week", 335, 273, 50, 26),
        ("High", 42, 286, 28, 20),
        ("Low", 96, 287, 26, 20),
        ("Stock", 137, 287, 33, 20),
        ("Last", 233, 285, 29, 27),
        ("Change", 267, 287, 41, 28),
        ("High", 338, 293, 24, 18),
        ("Low", 388, 293, 30, 19),
        ("Stock", 433, 294, 30, 18),
        ("Last", 528, 293, 26, 24),
        ("Change", 562, 294, 41, 24),
        ("45.71", 343, 321, 31, 19),
        ("32.50", 386, 322, 33, 19),
        ("Biomet", 429, 319, 51, 25),
        ("36.71", 526, 324, 32, 19),
        ("-0.42", 569, 321, 38, 25),
        ("2.76", 348, 335, 26, 19),
        ("1.20", 392, 336, 25, 19),
        ("Biomira", 428, 334, 61, 25),
        ("BloScrip", 430, 349, 57, 25),
        ("1.46", 534, 341, 23, 15),
        ("+0.03", 571, 340, 33, 18),
        ("9.07", 348, 350, 26, 19),
        ("5.13", 391, 351, 26, 19),
        ("8.05", 533, 355, 26, 16),
        ("+0.34", 567, 352, 39, 22),
        ("9.19", 54, 365, 24, 18),
        ("6.89", 97, 366, 23, 18),
        ("ABX", 136, 363, 29, 24),
        ("Air", 170, 365, 25, 24),
        ("n", 197, 366, 9, 23),
        ("7.52", 234, 365, 27, 23),
        ("-0.10", 272, 366, 38, 24),
        ("68.88", 342, 365, 29, 18),
        ("50.65", 385, 366, 29, 18),
        ("212.25", 334, 376, 37, 24),
        ("131.03", 379, 377, 38, 24),
        ("Biosite", 431, 367, 47, 18),
        ("50.05", 528, 368, 27, 18),
        ("BiotechT", 429, 378, 57, 25),
        ("-4.57", 571, 369, 33, 18),
        ("33.25", 46, 379, 32, 19),
        ("12.40", 89, 380, 33, 19),
        ("ACMoore", 137, 380, 60, 21),
        ("13.58", 232, 383, 28, 19),
        ("-1.57", 272, 384, 36, 19),
        ("204.66", 518, 382, 37, 19),
        ("BirchMt", 428, 393, 58, 25),
        ("gn", 489, 395, 16, 23),
        ("-0.84", 566, 384, 38, 18),
        ("31.38", 48, 394, 30, 18),
        ("13.51", 92, 395, 27, 18),
        ("ADA-ES", 135, 395, 62, 21),
        ("20.96", 231, 398, 31, 19),
        ("+3.16", 275, 399, 33, 18),
        ("8.50", 347, 394, 24, 18),
        ("1.40", 390, 395, 28, 18),
        ("ADC", 136, 407, 28, 24),
        ("Tel", 170, 408, 20, 24),
        ("rs", 193, 409, 16, 23),
        ("Bickbaud", 428, 407, 68, 25),
        ("6.52", 530, 398, 26, 19),
        ("-0.45", 569, 398, 35, 19),
        ("27.14", 48, 409, 30, 18),
        ("12.88", 89, 409, 31, 19),
        ("23.21", 230, 412, 33, 19),
        ("+0.13", 272, 413, 36, 19),
        ("18.21", 341, 408, 32, 19),
        ("10.73", 384, 409, 32, 19),
        ("17.90", 524, 412, 31, 19),
        ("+0.70", 568, 412, 35, 19),
        ("30.40", 45, 424, 33, 19),
        ("16.70", 91, 425, 31, 18),
        ("ADECP", 134, 422, 53, 25),
        ("27.32", 230, 427, 32, 19),
        ("+0.73", 272, 428, 36, 19),
        ("52.73", 341, 425, 31, 16),
        ("13.86", 383, 424, 33, 19),
        ("BluCoat", 428, 422, 57, 25),
        ("AFC", 135, 436, 29, 24),
        ("Ent", 169, 438, 25, 23),
        ("s", 196, 439, 9, 22),
        ("BlueNile", 427, 436, 63, 25),
        ("41.29", 524, 427, 33, 19),
        ("+1.30", 567, 428, 33, 18),
        ("16.45", 46, 438, 32, 19),
        ("10.47", 89, 440, 29, 18),
        ("15.40", 230, 442, 31, 18),
        ("-0.14", 275, 443, 33, 18),
        ("44.35", 343, 440, 29, 15),
        ("24.15", 384, 438, 32, 19),
        ("BobEvn", 426, 451, 57, 25),
        ("40.30", 526, 443, 28, 15),
        ("-1.10", 570, 443, 33, 18),
        ("8.37", 54, 453, 22, 19),
        ("4.50", 95, 454, 25, 19),
        ("ASE", 136, 454, 25, 21),
        ("Tst", 172, 454, 18, 21),
        ("7.76", 235, 456, 27, 19),
        ("+0.40", 272, 457, 35, 19),
        ("26.45", 340, 453, 33, 19),
        ("19.91", 384, 453, 32, 19),
        ("22.99", 523, 456, 33, 19),
        ("19.25", 46, 468, 31, 18),
        ("12.75", 90, 469, 31, 18),
        ("ASM", 134, 466, 29, 24),
        ("Intl", 169, 467, 29, 25),
        ("17.65", 229, 473, 32, 15),
        ("-0.03", 272, 472, 36, 19),
        ("15.94", 340, 469, 32, 15),
        ("6.12", 390, 470, 23, 15),
        ("Bodisen", 429, 466, 49, 25),
        ("n", 486, 469, 7, 23),
        ("Bookham", 429, 480, 60, 25),
        ("15.45", 524, 471, 31, 19),
        ("ASML", 134, 481, 40, 24),
        ("HId", 180, 483, 24, 23),
        ("+0.45", 566, 472, 33, 18),
        ("20.92", 45, 482, 32, 19),
        ("13.94", 87, 483, 32, 19),
        ("21.24", 228, 486, 30, 18),
        ("+0.46", 271, 486, 36, 19),
        ("6.21", 348, 482, 22, 19),
        ("1.56", 388, 482, 27, 19),
        ("5.94", 532, 485, 21, 19),
        ("+0.06", 567, 486, 36, 19),
        ("27.38", 44, 497, 32, 19),
        ("16.39", 87, 498, 31, 19),
        ("ASV", 134, 496, 23, 24),
        ("Inc", 164, 497, 24, 24),
        ("5", 190, 498, 9, 23),
        ("26.76", 228, 501, 30, 18),
        ("+0.14", 273, 501, 31, 19),
        ("11.80", 339, 496, 32, 19),
        ("4.99", 391, 499, 21, 16),
        ("Borland", 428, 497, 54, 21),
        ("10.47", 86, 509, 36, 23),
        ("BostPrv", 428, 509, 57, 25),
        ("6.68", 529, 500, 26, 19),
        ("ATI", 136, 510, 23, 24),
        ("Tech", 167, 511, 27, 24),
        ("31.90", 339, 510, 31, 19),
        ("+0.14", 568, 501, 33, 18),
        ("19.82", 44, 511, 32, 19),
        ("17.89", 227, 516, 32, 17),
        ("+0.68", 270, 516, 37, 18),
        ("21.10", 384, 512, 30, 18),
        ("BttmlnT", 428, 524, 55, 25),
        ("31.18", 522, 515, 35, 18),
        ("ATMI", 133, 525, 40, 24),
        ("Inc", 175, 526, 24, 24),
        ("-0.07", 567, 516, 35, 17),
        ("33.62", 44, 526, 32, 19),
        ("20.53", 87, 527, 32, 19),
        ("29.95", 228, 530, 31, 19),
        ("+1.29", 272, 531, 33, 18),
        ("18.62", 339, 527, 31, 16),
        ("10.01", 382, 528, 30, 16),
        ("BrigExp", 428, 538, 55, 26),
        ("11.53", 522, 529, 35, 18),
        ("+0.20", 564, 530, 38, 18),
        ("39.20", 43, 541, 33, 18),
        ("16.76", 87, 542, 30, 18),
        ("ATP", 133, 542, 26, 20),
        ("O&G", 166, 542, 29, 20),
        ("38.40", 228, 545, 30, 18),
        ("-0.59", 272, 546, 32, 17),
        ("14.68", 338, 542, 32, 15),
        ("7.10", 388, 543, 23, 15),
        ("12.10", 522, 544, 30, 18),
        ("AVI", 135, 554, 25, 25),
        ("Bio", 163, 555, 25, 25),
        ("BrightHrz", 428, 554, 66, 24),
        ("s", 497, 556, 8, 22),
        ("-0.23", 567, 545, 35, 17),
        ("4.24", 49, 556, 26, 19),
        ("1.99", 92, 556, 26, 19),
        ("3.62", 233, 560, 26, 19),
        ("-0.02", 269, 560, 36, 19),
        ("46.72", 338, 555, 32, 19),
        ("26.65", 383, 556, 30, 18),
        ("38.90", 522, 558, 32, 19),
        ("-0.80", 565, 559, 36, 19),
        ("20\u{2013}55", 338, 570, 30, 10),
    ];

    #[cfg(feature = "pdf")]
    fn hocr_words_from(entries: &[(&str, u32, u32, u32, u32)]) -> Vec<crate::table_core::HocrWord> {
        entries
            .iter()
            .map(|&(text, left, top, width, height)| hocr_word_at(text, left, top, width, height))
            .collect()
    }

    /// Reproduces the table-detection loop `process_image` runs once `enable_table_detection`
    /// is on, including the `post_process_table` structural gate the fix adds. Kept separate
    /// from `process_image` itself so the discriminator can be exercised directly against real
    /// fixture geometry without spinning up a PaddleOCR engine.
    #[cfg(feature = "pdf")]
    fn detected_table_count(words: &[crate::table_core::HocrWord]) -> usize {
        let mut count = 0usize;
        for region_words in crate::table_core::cluster_words_into_table_regions(words) {
            if region_words.len() < crate::table_core::MIN_TABLE_CANDIDATE_WORDS {
                continue;
            }
            let cells = reconstruct_table(&region_words, TABLE_COLUMN_ALIGNMENT_THRESHOLD_PX, 0.5);
            if cells.is_empty() || cells[0].is_empty() {
                continue;
            }
            if crate::pdf::table_reconstruct::post_process_table(cells, false, false).is_some() {
                count += 1;
            }
        }
        count
    }

    /// Negative fixture: all 355 words of real ordinary paragraph prose (page 1 of
    /// `ordinance_2197_scanned.pdf`, which has zero tables) must not fabricate a table.
    ///
    /// Fails without the `post_process_table` gate: unfiltered, `cluster_words_into_table_regions`
    /// puts all 355 words in one region (no gap exceeds the region-split threshold on a page of
    /// continuous body text) and `reconstruct_table` manufactures a 24-row x 36-column grid from
    /// it (measured directly from this fixture) -- one fabricated table, not zero.
    #[test]
    #[cfg(feature = "pdf")]
    fn table_discriminator_rejects_real_prose_page() {
        let words = hocr_words_from(PROSE_PAGE_WORDS);

        // The raw pre-discriminator reconstruction is exactly the pathological shape the bug
        // report described (many columns, mostly-empty cells) -- confirming this fixture
        // reproduces the defect rather than accidentally sidestepping it.
        let regions = crate::table_core::cluster_words_into_table_regions(&words);
        assert_eq!(
            regions.len(),
            1,
            "a page of continuous prose has no gap wide enough to split"
        );
        let raw = reconstruct_table(&regions[0], TABLE_COLUMN_ALIGNMENT_THRESHOLD_PX, 0.5);
        assert!(
            raw.first().is_some_and(|row| row.len() >= 20),
            "raw reconstruction should fabricate a wide grid from unaligned prose word x-positions, \
             got {} columns",
            raw.first().map_or(0, Vec::len)
        );

        assert_eq!(
            detected_table_count(&words),
            0,
            "post_process_table must reject the fabricated prose grid"
        );
    }

    /// Positive fixture: all 215 words of a real dense numeric newspaper stock table
    /// (`ocr_image.tiff`, ground truth 16 rows x 10 cols, 154 non-empty cells) must still be
    /// detected as a table once the discriminator is in place.
    #[test]
    #[cfg(feature = "pdf")]
    // Still ignored after the #688 cell-merge fix, but for a NARROWER reason than before.
    // That fix (merging horizontally-adjacent words into cell tokens before column detection)
    // did unblock the tesseract path on this same document: 0 -> 24 pipe rows end to end, and
    // native-corpus table cell recall rose 19.3% -> 30.0% with precision 46.2% -> 49.5%.
    // It does NOT rescue this fixture, because these are paddle word boxes scored at
    // TABLE_COLUMN_ALIGNMENT_THRESHOLD_PX = 20, where tesseract's path uses
    // `table_column_threshold` = 50 and merges more aggressively.
    //
    // The remaining blocker is REGION ISOLATION, not the validator: all 215 words land in one
    // region (largest vertical gap on the page is 0, against a 3 x average-word-height
    // threshold), so the newspaper masthead and a prose sentence are reconstructed together
    // with the table. Measured separately on docling_scanned.pdf, where every region is a
    // whole page of prose and tables only appear once `--layout` supplies real table regions.
    #[ignore = "blocked on table-region isolation, not on the validator: all 215 words cluster \
               into one region so the masthead is reconstructed with the table. Un-ignore when \
               regions are isolated structurally (see #694) rather than by vertical gap."]
    fn table_discriminator_accepts_real_dense_numeric_table() {
        let words = hocr_words_from(DENSE_TABLE_WORDS);

        assert_eq!(
            detected_table_count(&words),
            1,
            "a genuine dense numeric table must survive post_process_table, not be silenced \
             along with the prose case"
        );
    }
}
