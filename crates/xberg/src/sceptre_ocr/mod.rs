//! Sceptre OCR backend.
//!
//! Sceptre runs CRAFT text detection followed by CRNN recognition through ONNX
//! Runtime, the pure-Rust tract engine, or hand-written networks over candle
//! (CPU, Metal, or CUDA). Readers are initialized lazily and cached by their
//! effective model and inference configuration.
//!
//! [`build_document`] applies a document-assembly pass sceptre itself has no
//! concept of: [`assign_sceptre_block_ids`] groups recognized lines into
//! paragraph blocks by geometry, mirroring PaddleOCR's `assign_line_block_ids`.

use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, Mutex, Weak};

use async_trait::async_trait;
use once_cell::sync::OnceCell;
use sceptre::{Backend, Image, Language, OcrResult, ReadOptions, Reader, TextLine};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::core::config::{ExecutionProviderType, OcrConfig};
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::sceptre_languages::{language_group, language_group_name, supported_language_aliases};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::{
    BoundingBox, ExtractedDocument, FormatMetadata, Metadata, OcrBoundingGeometry, OcrConfidence, OcrElement,
    OcrElementLevel, OcrMetadata,
};
use crate::{Result, XbergError};

mod blocks;
use blocks::{HOCR_BLOCK_ID_ATTRIBUTE, assign_sceptre_block_ids};

const BACKEND_NAME: &str = "sceptre";
const DEFAULT_LANGUAGE: &str = "eng";
// The metadata key names come from the ungated `crate::ocr_metadata_keys` rather than
// `crate::ocr`, which is gated on `feature = "ocr"` / `"ocr-wasm"` — neither of which a
// Sceptre-only build implies. ~keep
use crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY as PROCESSED_HEIGHT_KEY;
use crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY as PROCESSED_WIDTH_KEY;
#[cfg(auto_rotate)]
use crate::ocr_metadata_keys::{
    OCR_AUTO_ROTATED_METADATA_KEY as AUTO_ROTATED_KEY,
    OCR_ORIENTATION_CONFIDENCE_METADATA_KEY as ORIENTATION_CONFIDENCE_KEY,
    OCR_ORIENTATION_DEGREES_METADATA_KEY as ORIENTATION_DEGREES_KEY,
};
// Each reader retains multiple ONNX sessions and a worker pool. Four entries
// cover common language/configuration reuse while bounding retained resources. ~keep
const READER_CACHE_CAPACITY: usize = 4;
#[cfg(auto_rotate)]
const ORIENTATION_DETECTOR_CACHE_CAPACITY: usize = 4;
// A transient overflow slot prevents requests from blocking when every cached
// reader is active while still bounding heavyweight reader initialization. ~keep
const READER_OVERFLOW_CAPACITY: usize = 1;
// A Sceptre reader already parallelizes detection and recognition internally.
// Admit one operation at a time so cached readers cannot multiply the process
// thread budget during concurrent extraction. ~keep
const EXECUTION_CAPACITY: usize = 1;

type ReaderCell = Arc<ReaderSlot>;

struct ReaderSlot {
    reader: OnceCell<Reader>,
    _overflow_permit: Option<OwnedSemaphorePermit>,
}

impl ReaderSlot {
    fn cached() -> Self {
        Self {
            reader: OnceCell::new(),
            _overflow_permit: None,
        }
    }

    fn overflow(permit: OwnedSemaphorePermit) -> Self {
        Self {
            reader: OnceCell::new(),
            _overflow_permit: Some(permit),
        }
    }
}

struct ReaderCache {
    entries: HashMap<String, ReaderCell>,
    overflow: HashMap<String, Weak<ReaderSlot>>,
    recency: VecDeque<String>,
}

#[cfg(auto_rotate)]
type OrientationDetectorCell = Arc<crate::doc_orientation::DocOrientationDetector>;

#[cfg(auto_rotate)]
struct OrientationDetectorCache {
    entries: HashMap<String, OrientationDetectorCell>,
    recency: VecDeque<String>,
}

#[cfg(auto_rotate)]
impl OrientationDetectorCache {
    fn new() -> Self {
        Self {
            entries: HashMap::with_capacity(ORIENTATION_DETECTOR_CACHE_CAPACITY),
            recency: VecDeque::with_capacity(ORIENTATION_DETECTOR_CACHE_CAPACITY),
        }
    }

    fn detector(
        &mut self,
        cache_dir: &Path,
        acceleration: Option<crate::core::config::AccelerationConfig>,
    ) -> Result<OrientationDetectorCell> {
        let key = orientation_detector_cache_key(cache_dir, acceleration.as_ref())?;
        if let Some(detector) = self.entries.get(&key).cloned() {
            self.mark_recent(&key);
            return Ok(detector);
        }

        if self.entries.len() == ORIENTATION_DETECTOR_CACHE_CAPACITY && !self.evict_oldest_inactive() {
            return Err(ocr_error(format!(
                "Sceptre orientation detector cache is at its bounded capacity of {ORIENTATION_DETECTOR_CACHE_CAPACITY} active configurations"
            )));
        }

        let detector = Arc::new(crate::doc_orientation::DocOrientationDetector::with_acceleration(
            cache_dir.to_path_buf(),
            acceleration,
        ));
        self.entries.insert(key.clone(), Arc::clone(&detector));
        self.recency.push_back(key);
        Ok(detector)
    }

    fn mark_recent(&mut self, key: &str) {
        if let Some(index) = self.recency.iter().position(|cached_key| cached_key == key) {
            self.recency.remove(index);
        }
        self.recency.push_back(key.to_string());
    }

    fn evict_oldest_inactive(&mut self) -> bool {
        let Some(index) = self.recency.iter().position(|key| {
            self.entries
                .get(key)
                .is_some_and(|detector| Arc::strong_count(detector) == 1)
        }) else {
            return false;
        };
        let Some(key) = self.recency.remove(index) else {
            return false;
        };
        self.entries.remove(&key);
        true
    }
}

impl ReaderCache {
    fn new() -> Self {
        Self {
            entries: HashMap::with_capacity(READER_CACHE_CAPACITY),
            overflow: HashMap::with_capacity(READER_OVERFLOW_CAPACITY),
            recency: VecDeque::with_capacity(READER_CACHE_CAPACITY),
        }
    }

    fn reader_cell(&mut self, key: &str) -> Option<ReaderCell> {
        if let Some(cell) = self.entries.get(key).cloned() {
            self.mark_recent(key);
            return Some(cell);
        }

        self.overflow.retain(|_, cell| cell.strong_count() > 0);
        if let Some(cell) = self.overflow.get(key).and_then(Weak::upgrade) {
            return Some(cell);
        }

        if self.entries.len() == READER_CACHE_CAPACITY && !self.evict_oldest_inactive() {
            return None;
        }

        let cell = Arc::new(ReaderSlot::cached());
        self.entries.insert(key.to_string(), Arc::clone(&cell));
        self.recency.push_back(key.to_string());
        Some(cell)
    }

    fn insert_overflow(&mut self, key: &str, permit: OwnedSemaphorePermit) -> ReaderCell {
        let cell = Arc::new(ReaderSlot::overflow(permit));
        self.overflow.insert(key.to_string(), Arc::downgrade(&cell));
        cell
    }

    fn mark_recent(&mut self, key: &str) {
        if let Some(index) = self.recency.iter().position(|cached_key| cached_key == key) {
            self.recency.remove(index);
        }
        self.recency.push_back(key.to_string());
    }

    fn evict_oldest_inactive(&mut self) -> bool {
        let Some(index) = self
            .recency
            .iter()
            .position(|key| self.entries.get(key).is_some_and(|cell| Arc::strong_count(cell) == 1))
        else {
            return false;
        };
        let Some(key) = self.recency.remove(index) else {
            return false;
        };
        self.entries.remove(&key);
        true
    }
}

/// Sceptre-backed OCR implementation.
///
/// Reader construction resolves model files and creates inference sessions, so
/// it is deferred until the first call for an effective configuration. The
/// initialized reader is reused without holding the cache lock during inference;
/// backend-wide admission prevents cached readers from oversubscribing the host.
#[cfg_attr(alef, alef(skip))]
pub struct SceptreOcrBackend {
    readers: Arc<Mutex<ReaderCache>>,
    overflow_slots: Arc<Semaphore>,
    execution_slots: Arc<Semaphore>,
    #[cfg(auto_rotate)]
    orientation_detectors: Arc<Mutex<OrientationDetectorCache>>,
}

impl SceptreOcrBackend {
    /// Create an empty, lazily initialized Sceptre backend.
    pub fn new() -> Result<Self> {
        Ok(Self {
            readers: Arc::new(Mutex::new(ReaderCache::new())),
            overflow_slots: Arc::new(Semaphore::new(READER_OVERFLOW_CAPACITY)),
            execution_slots: Arc::new(Semaphore::new(EXECUTION_CAPACITY)),
            #[cfg(auto_rotate)]
            orientation_detectors: Arc::new(Mutex::new(OrientationDetectorCache::new())),
        })
    }

    async fn reader_cell(&self, key: &str) -> Result<ReaderCell> {
        if let Some(cell) = self.lock_reader_cache()?.reader_cell(key) {
            return Ok(cell);
        }

        let permit = Arc::clone(&self.overflow_slots)
            .acquire_owned()
            .await
            .map_err(|error| ocr_error(format!("Failed to acquire Sceptre reader admission: {error}")))?;
        let mut readers = self.lock_reader_cache()?;
        if let Some(cell) = readers.reader_cell(key) {
            return Ok(cell);
        }
        Ok(readers.insert_overflow(key, permit))
    }

    fn lock_reader_cache(&self) -> Result<std::sync::MutexGuard<'_, ReaderCache>> {
        self.readers.lock().map_err(|error| XbergError::Plugin {
            message: format!("Failed to acquire Sceptre reader cache: {error}"),
            plugin_name: BACKEND_NAME.to_string(),
        })
    }

    #[cfg(auto_rotate)]
    fn orientation_detector_cell(
        &self,
        cache_dir: &Path,
        acceleration: Option<crate::core::config::AccelerationConfig>,
    ) -> Result<OrientationDetectorCell> {
        self.orientation_detectors
            .lock()
            .map_err(|error| XbergError::Plugin {
                message: format!("Failed to acquire Sceptre orientation detector cache: {error}"),
                plugin_name: BACKEND_NAME.to_string(),
            })?
            .detector(cache_dir, acceleration)
    }

    #[cfg(auto_rotate)]
    fn orientation_detector_for_config(
        &self,
        config: &sceptre::OcrConfig,
        acceleration: Option<crate::core::config::AccelerationConfig>,
    ) -> Result<OrientationDetectorCell> {
        let cache_dir = config
            .model
            .cache_dir
            .clone()
            .unwrap_or_else(crate::doc_orientation::resolve_cache_dir);
        self.orientation_detector_cell(&cache_dir, acceleration)
    }

    #[cfg(auto_rotate)]
    fn rotation_context(&self, config: &OcrConfig, sceptre_config: &sceptre::OcrConfig) -> Result<RotationContext> {
        Ok(RotationContext {
            detector: config
                .auto_rotate
                .then(|| self.orientation_detector_for_config(sceptre_config, config.acceleration.clone()))
                .transpose()?,
            enabled: config.auto_rotate,
        })
    }

    fn effective_config(config: &OcrConfig) -> Result<(sceptre::OcrConfig, Vec<String>)> {
        validate_auto_rotate_support(config)?;
        validate_acceleration(config)?;
        let languages = effective_languages(config);
        let group = resolve_language_group(&languages)?;
        let mut sceptre_config = parse_sceptre_options(config)?;
        sceptre_config.model.cache_dir =
            crate::cache_dir::ocr_model_cache_override(sceptre_config.model.cache_dir.as_deref())?;
        if !has_explicit_model_backend(config) {
            #[cfg(all(feature = "sceptre-ocr-tract", not(feature = "sceptre-ocr-ort")))]
            {
                sceptre_config.model.backend = Backend::Tract;
            }
        }
        validate_inference_backend(sceptre_config.model.backend)?;
        let thread_budget = crate::core::config::concurrency::active_thread_budget();
        sceptre_config.concurrency.max_threads = Some(
            sceptre_config
                .concurrency
                .max_threads
                .unwrap_or(thread_budget)
                .clamp(1, thread_budget),
        );
        sceptre_config.model.languages = vec![group];
        Ok((sceptre_config, languages))
    }

    async fn process_owned(&self, image_bytes: Arc<Vec<u8>>, config: &OcrConfig) -> Result<ExtractedDocument> {
        if image_bytes.is_empty() {
            return Err(XbergError::validation("Cannot run Sceptre OCR on empty image data"));
        }

        let (sceptre_config, languages) = Self::effective_config(config)?;
        let cache_key = reader_cache_key(&sceptre_config)?;
        let reader_cell = self.reader_cell(&cache_key).await?;
        let execution_permit = Arc::clone(&self.execution_slots)
            .acquire_owned()
            .await
            .map_err(|error| ocr_error(format!("Failed to acquire Sceptre execution admission: {error}")))?;
        #[cfg(auto_rotate)]
        let rotation = self.rotation_context(config, &sceptre_config)?;
        #[cfg(not(auto_rotate))]
        let rotation = RotationContext {};

        let output = tokio::task::spawn_blocking(move || {
            let _execution_permit = execution_permit;
            run_blocking(image_bytes.as_slice(), sceptre_config, reader_cell, rotation)
        })
        .await
        .map_err(|error| XbergError::Plugin {
            message: format!("Sceptre OCR blocking task failed: {error}"),
            plugin_name: BACKEND_NAME.to_string(),
        })??;

        Ok(build_document(output, languages, config))
    }
}

impl Default for SceptreOcrBackend {
    fn default() -> Self {
        Self::new().expect("constructing an empty Sceptre backend is infallible")
    }
}

impl Plugin for SceptreOcrBackend {
    fn name(&self) -> &str {
        BACKEND_NAME
    }

    fn version(&self) -> String {
        sceptre::VERSION.to_string()
    }
}

#[async_trait]
impl OcrBackend for SceptreOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractedDocument> {
        self.process_owned(Arc::new(image_bytes.to_vec()), config).await
    }

    async fn process_image_owned(&self, image_bytes: Arc<Vec<u8>>, config: &OcrConfig) -> Result<ExtractedDocument> {
        self.process_owned(image_bytes, config).await
    }

    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractedDocument> {
        let bytes = crate::core::io::read_file_async(path).await?;
        self.process_owned(Arc::new(bytes), config).await
    }

    fn supports_language(&self, language: &str) -> bool {
        language_group(language).is_some()
    }

    fn backend_type(&self) -> OcrBackendType {
        // Built-in OCR stages are selected by registry name; keep the exhaustive
        // v1 enum stable while adding Sceptre in a minor release. ~keep
        OcrBackendType::Custom
    }

    fn supported_languages(&self) -> Vec<String> {
        supported_language_aliases().into_iter().map(str::to_string).collect()
    }

    /// Sceptre's page confidence is EasyOCR's length-penalised `custom_mean` rescaled to
    /// 0-100. It is not comparable to Tesseract's scale: on one document a real text page
    /// scored 39 while a pure line-art drawing scored 74, an inverted ordering relative to
    /// legibility. Never gate on this value.
    fn confidence_semantics(&self) -> crate::plugins::ConfidenceSemantics {
        crate::plugins::ConfidenceSemantics::Uncalibrated
    }

    /// Measured on a `/Rotate 270` scanned ordinance: sceptre produced character garbage on the
    /// sideways raster and only read correctly once the same page was rendered upright first.
    /// This is the same value as the trait default, made explicit here because it is a
    /// measurement, not an unverified inheritance.
    fn page_orientation_handling(&self) -> crate::plugins::PageOrientationHandling {
        crate::plugins::PageOrientationHandling::RequiresUpright
    }
}

struct BlockingOutput {
    result: OcrResult,
    width: u32,
    height: u32,
    #[cfg(auto_rotate)]
    orientation: Option<crate::doc_orientation::OrientationResult>,
    #[cfg(auto_rotate)]
    auto_rotated: bool,
}

struct RotationContext {
    #[cfg(auto_rotate)]
    detector: Option<OrientationDetectorCell>,
    #[cfg(auto_rotate)]
    enabled: bool,
}

fn run_blocking(
    image_bytes: &[u8],
    config: sceptre::OcrConfig,
    reader_cell: ReaderCell,
    rotation: RotationContext,
) -> Result<BlockingOutput> {
    #[cfg(not(auto_rotate))]
    let _ = rotation;
    let decoded = decode_sceptre_image(image_bytes)?;
    #[cfg(auto_rotate)]
    let (image, orientation, was_rotated) = decode_and_rotate(decoded, rotation.enabled, rotation.detector.as_deref())?;
    #[cfg(not(auto_rotate))]
    let image = Image::from_rgb8(decoded.width(), decoded.height(), decoded.into_raw()).map_err(map_sceptre_error)?;
    let width = image.width();
    let height = image.height();
    let reader = reader_cell
        .reader
        .get_or_try_init(|| Reader::builder().config(config).build().map_err(map_sceptre_error))?;
    let result = reader
        .recognize(&image, &ReadOptions::default())
        .map_err(map_sceptre_error)?;
    Ok(BlockingOutput {
        result,
        width,
        height,
        #[cfg(auto_rotate)]
        orientation,
        #[cfg(auto_rotate)]
        auto_rotated: was_rotated,
    })
}

fn decode_sceptre_image(image_bytes: &[u8]) -> Result<image::RgbImage> {
    crate::extraction::image_decode::decode_standard_rgb8_with_default_security_limits(image_bytes).map_err(|error| {
        XbergError::Ocr {
            message: "Sceptre OCR operation failed".to_string(),
            source: Some(Box::new(error)),
        }
    })
}

#[cfg(auto_rotate)]
fn decode_and_rotate(
    decoded: image::RgbImage,
    auto_rotate: bool,
    detector: Option<&crate::doc_orientation::DocOrientationDetector>,
) -> Result<(Image, Option<crate::doc_orientation::OrientationResult>, bool)> {
    if !auto_rotate {
        return Image::from_rgb8(decoded.width(), decoded.height(), decoded.into_raw())
            .map(|image| (image, None, false))
            .map_err(map_sceptre_error);
    }
    let detector = detector.ok_or_else(|| ocr_error("Sceptre orientation detector was not initialized"))?;

    let orientation = match detector.detect(&decoded) {
        Ok(orientation) => orientation,
        Err(error) => {
            tracing::warn!(error = %error, "Sceptre orientation detection failed; using the original image");
            return Image::from_rgb8(decoded.width(), decoded.height(), decoded.into_raw())
                .map(|image| (image, None, false))
                .map_err(map_sceptre_error);
        }
    };
    let should_rotate = orientation.degrees != 0 && orientation.confidence >= crate::doc_orientation::MIN_CONFIDENCE;
    let rotated = if should_rotate {
        rotate_to_upright(&decoded, orientation.degrees)
    } else {
        decoded
    };
    Image::from_rgb8(rotated.width(), rotated.height(), rotated.into_raw())
        .map(|image| (image, Some(orientation), should_rotate))
        .map_err(map_sceptre_error)
}

#[cfg(auto_rotate)]
fn rotate_to_upright(image: &image::RgbImage, degrees: u32) -> image::RgbImage {
    match degrees {
        90 => image::imageops::rotate270(image),
        180 => image::imageops::rotate180(image),
        270 => image::imageops::rotate90(image),
        _ => image.clone(),
    }
}

fn build_document(output: BlockingOutput, languages: Vec<String>, config: &OcrConfig) -> ExtractedDocument {
    let line_elements: Vec<OcrElement> = output.result.lines.iter().map(line_to_element).collect();
    let content = output
        .result
        .lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let internal_document = build_internal_document(&line_elements);
    let ocr_elements = select_output_elements(&line_elements, config);
    let metadata = build_metadata(&languages, &output);

    ExtractedDocument {
        content,
        mime_type: Cow::Borrowed("text/plain"),
        metadata,
        detected_languages: Some(languages),
        ocr_elements,
        ocr_internal_document: Some(internal_document),
        ..Default::default()
    }
}

fn line_to_element(line: &TextLine) -> OcrElement {
    OcrElement {
        text: line.text.clone(),
        geometry: OcrBoundingGeometry::Quadrilateral {
            points: line
                .quad
                .points
                .into_iter()
                .map(|point| (pixel_coordinate(point.x), pixel_coordinate(point.y)).into())
                .collect(),
        },
        confidence: OcrConfidence {
            detection: None,
            recognition: f64::from(line.confidence).clamp(0.0, 1.0),
        },
        level: OcrElementLevel::Line,
        rotation: None,
        page_number: 1,
        parent_id: None,
        backend_metadata: HashMap::new(),
    }
}

fn pixel_coordinate(value: f32) -> u32 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    value.round().min(u32::MAX as f32) as u32
}

fn build_internal_document(elements: &[OcrElement]) -> InternalDocument {
    let mut document = InternalDocument::new("ocr");
    document.mime_type = "text/plain".to_string();
    document.prebuilt_ocr_elements = Some(elements.to_vec());
    let block_ids = assign_sceptre_block_ids(elements);
    for (element, block_id) in elements.iter().zip(block_ids) {
        let mut internal = InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Line,
            },
            &element.text,
            0,
        );
        internal.page = Some(element.page_number);
        internal.bbox = geometry_bounds(&element.geometry);
        internal.ocr_geometry = Some(element.geometry.clone());
        internal.ocr_confidence = Some(element.confidence.clone());
        internal.attributes = Some([(HOCR_BLOCK_ID_ATTRIBUTE.to_string(), block_id)].into_iter().collect());
        document.push_element(internal);
    }
    document
}

fn geometry_bounds(geometry: &OcrBoundingGeometry) -> Option<BoundingBox> {
    let OcrBoundingGeometry::Quadrilateral { points } = geometry else {
        return None;
    };
    let x0 = points.iter().map(|point| point.x).min()?;
    let y0 = points.iter().map(|point| point.y).min()?;
    let x1 = points.iter().map(|point| point.x).max()?;
    let y1 = points.iter().map(|point| point.y).max()?;
    Some(BoundingBox {
        x0: f64::from(x0),
        y0: f64::from(y0),
        x1: f64::from(x1),
        y1: f64::from(y1),
    })
}

fn select_output_elements(elements: &[OcrElement], config: &OcrConfig) -> Option<Vec<OcrElement>> {
    let options = config.element_config.as_ref()?;
    let selected = options.select_elements(elements);
    (!selected.is_empty()).then_some(selected)
}

// Matches the Tesseract backend's low-word-confidence cutoff
// (ocr/processor/execution.rs) so downstream quality gating applies the same
// threshold regardless of which backend produced the metadata. ~keep
const LOW_CONFIDENCE_THRESHOLD_PCT: i64 = 50;

/// Per-line confidence statistics on Tesseract's 0-100 integer percentage
/// scale, so consumers that already gate on Tesseract's `mean_text_conf` /
/// `median_word_conf` / `p10_word_conf` keys (e.g. `extractors/pdf/ocr.rs`,
/// which divides `mean_text_conf` by 100.0) work unchanged against Sceptre.
struct ConfidenceStats {
    mean_pct: i64,
    median_pct: i64,
    p10_pct: i64,
    line_count: usize,
    low_confidence_count: usize,
}

/// Convert a Sceptre line confidence (`0.0..=1.0`) to Tesseract's `0..=100`
/// integer percentage scale.
fn confidence_to_percent(confidence: f32) -> i64 {
    (f64::from(confidence).clamp(0.0, 1.0) * 100.0).round() as i64
}

/// Compute mean/median/p10 confidence and low-confidence-line count from
/// per-line confidence values. Returns `None` when there are no lines, mirroring
/// the Tesseract backend leaving these keys out of `metadata.additional` when it
/// has nothing to report.
fn compute_confidence_stats(lines: &[TextLine]) -> Option<ConfidenceStats> {
    if lines.is_empty() {
        return None;
    }

    let percentages: Vec<i64> = lines
        .iter()
        .map(|line| confidence_to_percent(line.confidence))
        .collect();
    let line_count = percentages.len();
    let low_confidence_count = percentages
        .iter()
        .filter(|&&pct| pct < LOW_CONFIDENCE_THRESHOLD_PCT)
        .count();

    let mean_pct = percentages.iter().sum::<i64>() / line_count as i64;

    let mut sorted = percentages.clone();
    sorted.sort_unstable();
    let median_pct = if line_count.is_multiple_of(2) {
        (sorted[line_count / 2 - 1] + sorted[line_count / 2]) / 2
    } else {
        sorted[line_count / 2]
    };
    let p10_idx = ((line_count as f64 - 1.0) * 0.1).floor() as usize;
    let p10_pct = sorted[p10_idx.min(line_count - 1)];

    Some(ConfidenceStats {
        mean_pct,
        median_pct,
        p10_pct,
        line_count,
        low_confidence_count,
    })
}

/// Publish confidence statistics into `metadata.additional` using the same key
/// names the Tesseract backend uses, so quality-gating code that reads
/// `mean_text_conf` / `median_word_conf` / `p10_word_conf` / `word_count` /
/// `low_conf_word_count` works against Sceptre output too (issue #186). Sceptre
/// has no word-level detections, so line confidence is the finest granularity
/// available and is reported under the same keys.
fn insert_confidence_stats(metadata: &mut Metadata, lines: &[TextLine]) {
    let Some(stats) = compute_confidence_stats(lines) else {
        return;
    };
    metadata
        .additional
        .insert(Cow::Borrowed("mean_text_conf"), serde_json::json!(stats.mean_pct));
    metadata
        .additional
        .insert(Cow::Borrowed("median_word_conf"), serde_json::json!(stats.median_pct));
    metadata
        .additional
        .insert(Cow::Borrowed("p10_word_conf"), serde_json::json!(stats.p10_pct));
    metadata
        .additional
        .insert(Cow::Borrowed("word_count"), serde_json::json!(stats.line_count));
    metadata.additional.insert(
        Cow::Borrowed("low_conf_word_count"),
        serde_json::json!(stats.low_confidence_count),
    );
}

fn build_metadata(languages: &[String], output: &BlockingOutput) -> Metadata {
    let mut metadata = Metadata {
        format: Some(FormatMetadata::Ocr(OcrMetadata {
            language: languages.join("+"),
            // Sceptre has no Tesseract-style Page Segmentation Mode; `psm: 3`
            // (Tesseract's "fully automatic" default) would misrepresent a
            // concept this backend does not have. Leave it at the zero default. ~keep
            output_format: "text".to_string(),
            ..Default::default()
        })),
        ocr_used: true,
        ..Default::default()
    };
    metadata
        .additional
        .insert(Cow::Borrowed(PROCESSED_WIDTH_KEY), serde_json::json!(output.width));
    metadata
        .additional
        .insert(Cow::Borrowed(PROCESSED_HEIGHT_KEY), serde_json::json!(output.height));
    insert_confidence_stats(&mut metadata, &output.result.lines);
    #[cfg(auto_rotate)]
    if let Some(orientation) = output.orientation {
        metadata.additional.insert(
            Cow::Borrowed(ORIENTATION_DEGREES_KEY),
            serde_json::json!(orientation.degrees),
        );
        metadata.additional.insert(
            Cow::Borrowed(ORIENTATION_CONFIDENCE_KEY),
            serde_json::json!(orientation.confidence),
        );
    }
    #[cfg(auto_rotate)]
    if output.auto_rotated {
        metadata
            .additional
            .insert(Cow::Borrowed(AUTO_ROTATED_KEY), serde_json::Value::Bool(true));
    }
    metadata
}

/// `object.get("detection")` is how `sceptre::DetectionConfig::detect_orientation` reaches
/// this backend: any caller can already flip it on today with
/// `backend_options: {"detection": {"detect_orientation": true}}`. Nothing here defaults it
/// to `true`, and that is a deliberate decision (#662), not an oversight to "finish wiring".
///
/// `detect_orientation` is sceptre's own opt-in whole-page rotation guesser: it probes CRAFT
/// at 0/90/180/270 degrees on a reduced canvas, rotates internally for the winning angle, and
/// unrotates the output quads back onto the frame of the `Image` this backend passed in
/// (`sceptre::engine::sceptre_engine`, `detect::orientation::unrotate_corners`) before
/// `TextLine`s are ever returned here. So enabling it introduces no coordinate-frame bug for
/// this crate's `line_to_element` / `build_internal_document` / table geometry — that risk was
/// checked and ruled out, not assumed away.
///
/// The reason to leave it off is that xberg already has two better-targeted fixes for exactly
/// the defect it targets, and stacking a third, weaker one on top adds cost without adding
/// coverage:
///   - For PDF-sourced pages, `extractors::pdf::ocr::upright_raster_for_backend` (#643) rotates
///     the raster to upright using the page's own `/Rotate` value — a known fact, not a guess —
///     whenever this backend's `page_orientation_handling()` reports `RequiresUpright`, and
///     `undo_upright_raster_correction` maps the returned geometry back deterministically. Zero
///     false-positive risk, because there is nothing to infer.
///   - For raw images with no `/Rotate` to consult, `crate::doc_orientation` (a PP-LCNet
///     classifier) already runs ahead of this backend, cross-backend, behind `config.auto_rotate`
///     (`decode_and_rotate` below), with its own coordinate-frame correction in
///     `extractors::pdf::ocr::undo_auto_rotate_document_bboxes`.
///
/// That leaves only images with unknown rotation and `auto_rotate` left off as a case
/// `detect_orientation` could add coverage for — and sceptre's own ADR 0038 keeps the flag
/// `false` by default even after fixing its false-positive rate to 0/23 on its labeled corpus,
/// because it costs four extra CRAFT forward passes on every page (rotated or not) and the
/// validation set is 23 images. Defaulting it on in xberg would pay that unconditional cost
/// for every Sceptre page to cover a narrower gap than it looks like, and inherit a heuristic
/// xberg has not independently validated. Revisit if `crate::doc_orientation` is ever removed,
/// or if a corpus-backed measurement on xberg's own fixtures justifies the cost.
fn parse_sceptre_options(config: &OcrConfig) -> Result<sceptre::OcrConfig> {
    let Some(options) = config.backend_options.as_ref() else {
        return Ok(sceptre::OcrConfig::default());
    };
    let object = options
        .as_object()
        .ok_or_else(|| ocr_error("Invalid `backend_options` configuration: expected an object"))?;
    let mut parsed = sceptre::OcrConfig::default();
    if let Some(value) = object.get("detection") {
        parsed.detection = parse_sceptre_section("detection", value)?;
    }
    if let Some(value) = object.get("recognition") {
        parsed.recognition = parse_sceptre_section("recognition", value)?;
    }
    if let Some(value) = object.get("concurrency") {
        parsed.concurrency = parse_sceptre_section("concurrency", value)?;
    }
    if let Some(value) = object.get("model") {
        parsed.model = parse_sceptre_section("model", value)?;
    }
    Ok(parsed)
}

fn has_explicit_model_backend(config: &OcrConfig) -> bool {
    config
        .backend_options
        .as_ref()
        .and_then(serde_json::Value::as_object)
        .and_then(|options| options.get("model"))
        .and_then(serde_json::Value::as_object)
        .is_some_and(|model| model.contains_key("backend"))
}

fn validate_inference_backend(backend: Backend) -> Result<()> {
    match backend {
        Backend::Ort if cfg!(feature = "sceptre-ocr-ort") => Ok(()),
        Backend::Tract if cfg!(feature = "sceptre-ocr-tract") => Ok(()),
        Backend::Candle if cfg!(feature = "sceptre-ocr-candle") => Ok(()),
        Backend::Ort => Err(ocr_error(
            "Sceptre ORT inference is unavailable in this build; enable `sceptre-ocr-ort`",
        )),
        Backend::Tract => Err(ocr_error(
            "Sceptre tract inference is unavailable in this build; enable `sceptre-ocr-tract`",
        )),
        Backend::Candle => Err(ocr_error(
            "Sceptre candle inference is unavailable in this build; enable `sceptre-ocr-candle`",
        )),
    }
}

fn parse_sceptre_section<T: serde::de::DeserializeOwned>(name: &str, value: &serde_json::Value) -> Result<T> {
    serde_json::from_value(value.clone()).map_err(|error| {
        ocr_error(format!(
            "Invalid Sceptre `backend_options.{name}` configuration: {error}"
        ))
    })
}

fn reader_cache_key(config: &sceptre::OcrConfig) -> Result<String> {
    serde_json::to_string(config)
        .map_err(|error| ocr_error(format!("Failed to serialize effective Sceptre configuration: {error}")))
}

#[cfg(auto_rotate)]
fn orientation_detector_cache_key(
    cache_dir: &Path,
    acceleration: Option<&crate::core::config::AccelerationConfig>,
) -> Result<String> {
    serde_json::to_string(&(cache_dir, acceleration)).map_err(|error| {
        ocr_error(format!(
            "Failed to serialize Sceptre orientation configuration: {error}"
        ))
    })
}

fn effective_languages(config: &OcrConfig) -> Vec<String> {
    let languages: Vec<String> = config
        .language
        .iter()
        .map(|language| language.trim().to_ascii_lowercase())
        .filter(|language| !language.is_empty())
        .collect();
    if languages.is_empty() {
        vec![DEFAULT_LANGUAGE.to_string()]
    } else {
        languages
    }
}

fn resolve_language_group(languages: &[String]) -> Result<Language> {
    let mut selected = Language::English;
    for language in languages {
        let group = language_group(language).ok_or_else(|| {
            ocr_error(format!(
                concat!(
                    "Sceptre does not support OCR language `{}`; choose an English, Latin, ",
                    "simplified Chinese, Japanese, Korean, Telugu, Kannada, or Cyrillic language"
                ),
                language
            ))
        })?;
        if group == Language::English {
            continue;
        }
        if selected != Language::English && selected != group {
            return Err(ocr_error(format!(
                concat!(
                    "Sceptre cannot combine language groups `{}` and `{}` in one reader; ",
                    "use languages from one script family (English may be combined with any one family)"
                ),
                language_group_name(selected),
                language_group_name(group),
            )));
        }
        selected = group;
    }
    Ok(selected)
}

fn validate_acceleration(config: &OcrConfig) -> Result<()> {
    let Some(acceleration) = config.acceleration.as_ref() else {
        return Ok(());
    };
    if acceleration.provider == ExecutionProviderType::Cpu {
        return Ok(());
    }
    Err(ocr_error(format!(
        concat!(
            "Sceptre in Xberg 1.1 is CPU-only; acceleration provider `{:?}` is not supported. ",
            "Set acceleration.provider to `cpu` or omit acceleration"
        ),
        acceleration.provider
    )))
}

fn validate_auto_rotate_support(_config: &OcrConfig) -> Result<()> {
    #[cfg(not(auto_rotate))]
    if _config.auto_rotate {
        return Err(ocr_error(
            "Sceptre auto-rotation is unavailable in this build; enable the `auto-rotate` or `auto-rotate-tract` feature",
        ));
    }
    Ok(())
}

fn map_sceptre_error(error: sceptre::OcrError) -> XbergError {
    XbergError::Ocr {
        message: "Sceptre OCR operation failed".to_string(),
        source: Some(Box::new(error)),
    }
}

fn ocr_error(message: impl Into<String>) -> XbergError {
    XbergError::Ocr {
        message: message.into(),
        source: None,
    }
}

#[cfg(test)]
mod tests;
