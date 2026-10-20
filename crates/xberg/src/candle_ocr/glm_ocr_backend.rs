//! GLM-OCR backend plugin for the Xberg OCR pipeline.
//!
//! This module wraps the candle-based GLM-OCR engine in the `OcrBackend` trait,
//! making it available to the extraction pipeline.
//!
//! # Engine pool design
//!
//! The pool key is `(DevicePreference, DType)` — NOT including the task.
//! All five GLM-OCR tasks (`Ocr`, `Table`, `Formula`, `Chart`, `Caption`) differ
//! only in the prompt prefix fed to the decoder. The model weights are identical,
//! so a single engine instance handles every task via `process_image_with_task`.
//! This avoids loading the ~900 MB safetensors five times in paired mode.

use async_trait::async_trait;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use parking_lot::Mutex;

use crate::Result;
use crate::candle_ocr::config::{
    GlmOcrBackendOptions, GlmOcrLayoutMode, GlmOcrTaskKind, parse_backend_options, validate_optional_non_empty,
};
use crate::core::config::OcrConfig;
use crate::engine_cache::EngineCache;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractedDocument;
use xberg_candle_ocr::CandleOcrError;
use xberg_candle_ocr::DType;
use xberg_candle_ocr::DevicePreference;
use xberg_candle_ocr::models::GlmOcrEngine;
use xberg_candle_ocr::models::GlmOcrTask;

/// How the backend dispatches inference across a page image.
///
/// `WholePage` passes the raw page bytes to the engine as a single call — fast
/// and simple. `Paired` (compiled only when `layout-detection` is enabled) runs
/// PP-DocLayout-V3 first, crops individual regions, and dispatches each crop to
/// the task that best matches the detected layout class, merging results in
/// reading order.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Feed the whole page to the model with the backend's default task.
    WholePage,
    /// Detect layout regions first; dispatch each crop to the matching task.
    ///
    /// Only available when the `layout-detection` feature is enabled.
    #[cfg(feature = "layout-detection")]
    Paired,
}

// compiled in, WholePage otherwise. #[derive(Default)] cannot express this.
#[allow(clippy::derivable_impls)]
impl Default for LayoutMode {
    fn default() -> Self {
        #[cfg(feature = "layout-detection")]
        {
            LayoutMode::Paired
        }
        #[cfg(not(feature = "layout-detection"))]
        {
            LayoutMode::WholePage
        }
    }
}

/// Key for the GLM-OCR engine cache: `(DevicePreference, DType, cache_dir, revision)`.
type EngineKey = (DevicePreference, DType, PathBuf, String);

/// Process-wide engine cache keyed by `(DevicePreference, DType, cache_dir, revision)`.
///
/// A single engine instance handles all tasks (OCR / Table / Formula / Chart /
/// Caption) via `process_image_with_task`, so the key does not include the
/// task. Two callers requesting the same device+dtype but different tasks will
/// share one engine and avoid loading weights twice.
static ENGINE_POOL: LazyLock<EngineCache<EngineKey, GlmOcrEngine>> = LazyLock::new(EngineCache::unbounded);

/// Key for the layout model cache: `(model_path, device_preference)`.
#[cfg(feature = "layout-detection")]
type LayoutKey = (String, DevicePreference);

/// Process-wide layout model cache keyed by `(model_path, device_preference)`.
///
/// Caches loaded `PpDocLayoutV3Model` instances by their file path and device preference
/// to avoid reloading the expensive ONNX model on each `process_paired` invocation.
/// Two callers requesting the same model path with the same device will share one instance.
/// Different devices get separate model instances due to device-specific optimizations.
/// The model is wrapped in Mutex (not RwLock) since `detect` takes `&mut self`.
///
/// Only available when `layout-detection` is enabled.
#[cfg(feature = "layout-detection")]
static LAYOUT_POOL: LazyLock<
    EngineCache<LayoutKey, Mutex<crate::layout::models::pp_doclayout_v3::PpDocLayoutV3Model>>,
> = LazyLock::new(EngineCache::unbounded);

/// Process-wide cache of the resolved, verified PP-DocLayout-V3 model path.
///
/// Resolving the model runs a SHA-256 over all 131,731,131 bytes of it. Paired
/// mode resolves the model to feed [`LAYOUT_POOL`] its key, and paired mode runs
/// once per page, so the hash was charged to every page of a document: a 22-page
/// document hashed about 2.9 GB and a 731-page document about 96 GB. The path and
/// its verdict are the same for every page, so both are computed once per process
/// (GH#1718).
///
/// Keyed by the cache directory, so a process that changes where Hugging Face
/// caches models resolves again. The verification itself is unchanged — only how
/// often it runs.
///
/// Only available when `layout-detection` is enabled.
#[cfg(feature = "layout-detection")]
static LAYOUT_MODEL_PATH_POOL: LazyLock<EngineCache<PathBuf, PathBuf>> = LazyLock::new(EngineCache::unbounded);

/// Get or build a cached value, delegating to [`EngineCache::get_or_try_init`].
///
/// The cache stays locked for the whole call, so a second caller for the same
/// key waits for the first load instead of building a duplicate engine. That
/// lock covers the WHOLE cache, not only `key`, so it also serialises loads of
/// different keys while either is in flight; that is coarser than a per-key
/// single flight, and it is the convention `EngineCache` already uses for
/// embeddings, reranking, late interaction and sparse embeddings.
///
/// # Errors
/// Propagates errors from the `init` closure. A failed `init` does not insert
/// anything, so the next caller for the same key retries rather than reusing
/// a poisoned entry.
#[inline]
fn pool_get_or_init<K, V, E>(
    cache: &EngineCache<K, V>,
    key: K,
    init: impl FnOnce() -> std::result::Result<V, E>,
) -> std::result::Result<Arc<V>, E>
where
    K: std::hash::Hash + Eq + Clone,
{
    cache.get_or_try_init(key, init)
}

/// Return a cached engine for `(preference, dtype)`, initialising one on first use.
///
/// Uses the generic [`pool_get_or_init`] helper to ensure two callers with the same
/// `(preference, dtype)` receive the same Arc instance.
fn get_or_init_engine(
    preference: DevicePreference,
    dtype: DType,
    cache_dir: PathBuf,
    revision: String,
) -> crate::Result<Arc<GlmOcrEngine>> {
    let key = (preference, dtype, cache_dir.clone(), revision.clone());

    pool_get_or_init(&ENGINE_POOL, key, || {
        let device = preference.select().map_err(|e| crate::XbergError::Ocr {
            message: format!("Failed to select compute device: {e}"),
            source: Some(Box::new(e)),
        })?;

        tracing::info!(
            preference = ?preference,
            ?dtype,
            "Initialising GLM-OCR engine (cold start)"
        );
        GlmOcrEngine::new_with_hf(GlmOcrTask::default(), device, dtype, Some(&cache_dir), Some(&revision)).map_err(
            |e| crate::XbergError::Ocr {
                message: format!("GLM-OCR engine initialisation failed: {e}"),
                source: Some(Box::new(e)),
            },
        )
    })
}

/// Return a cached layout model for the given path and device, initialising one on first use.
///
/// Uses the generic [`pool_get_or_init`] helper to ensure two callers with the same
/// path and device preference receive the same Arc instance. The model is wrapped in Mutex
/// (not RwLock) since `detect` takes `&mut self` and requires exclusive access.
///
/// Only available when `layout-detection` is enabled.
#[cfg(feature = "layout-detection")]
fn get_or_init_layout_model(
    model_path: &Path,
    device: DevicePreference,
) -> crate::Result<Arc<Mutex<crate::layout::models::pp_doclayout_v3::PpDocLayoutV3Model>>> {
    use crate::layout::models::pp_doclayout_v3::PpDocLayoutV3Model;

    let model_path_str = model_path
        .to_str()
        .ok_or_else(|| crate::XbergError::Ocr {
            message: format!("Model path contains invalid UTF-8: {}", model_path.display()),
            source: None,
        })?
        .to_string();

    let key = (model_path_str.clone(), device);

    pool_get_or_init(&LAYOUT_POOL, key, || {
        tracing::info!(
            path = model_path_str.as_str(),
            ?device,
            "Initialising PP-DocLayout-V3 model (cold start)"
        );
        PpDocLayoutV3Model::from_file(&model_path_str, None)
            .map_err(|e| crate::XbergError::Ocr {
                message: format!("PP-DocLayout-V3 model initialisation failed: {e}"),
                source: Some(Box::new(e)),
            })
            .map(Mutex::new)
    })
}

/// Resolve and SHA-256 verify PP-DocLayout-V3, once per process per cache directory.
///
/// A failed resolve is not cached, so the next page retries rather than reusing a
/// poisoned entry — see [`pool_get_or_init`].
///
/// Only available when `layout-detection` is enabled.
#[cfg(feature = "layout-detection")]
fn ensure_layout_model_path() -> crate::Result<Arc<PathBuf>> {
    use crate::layout::LayoutModelManager;

    pool_get_or_init(&LAYOUT_MODEL_PATH_POOL, hf_hub::resolve_cache_dir(), || {
        tracing::info!("Resolving and verifying PP-DocLayout-V3 (cold start)");
        LayoutModelManager::new(None)
            .ensure_pp_doclayout_v3_model()
            .map_err(|e| crate::XbergError::Ocr {
                message: format!("GLM-OCR paired: layout model unavailable: {e}"),
                source: Some(Box::new(e)),
            })
    })
}

/// Parse and check the GLM-OCR `backend_options`.
///
/// Configuration validation and every page both call this, so an invalid option
/// fails before any page runs. ~keep
pub(crate) fn check_backend_options(value: Option<&serde_json::Value>) -> Result<GlmOcrBackendOptions> {
    let options: GlmOcrBackendOptions = parse_backend_options(value, "candle-glm-ocr")?;
    validate_optional_non_empty(options.cache_dir.as_deref(), "candle-glm-ocr", "cache_dir")?;
    Ok(options)
}

/// Options parsed from backend-specific configuration.
///
/// Extracted from [`OcrConfig.backend_options`] to make GLM-OCR configuration
/// available to both the constructor and the runtime processing paths.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
struct GlmOcrOptions {
    task: GlmOcrTask,
    device: DevicePreference,
    layout_mode: LayoutMode,
    enable_chart_understanding: bool,
    cache_dir: Option<PathBuf>,
}

/// Map a layout detection class to the GLM-OCR task best suited for that region.
///
/// Picture and Chart both go to `Caption` since the model has no standalone
/// chart-region task; caption elicits a descriptive output from the VLM.
/// However, Chart is routed to the Chart task when chart understanding is enabled;
/// otherwise it falls back to Caption for all images.
/// Header and footer are treated as plain `Ocr`.
#[cfg(feature = "layout-detection")]
fn task_for_label(label: crate::layout::LayoutClass, enable_chart_understanding: bool) -> GlmOcrTask {
    use crate::layout::LayoutClass;
    match label {
        LayoutClass::Table => GlmOcrTask::Table,
        LayoutClass::Formula => GlmOcrTask::Formula,
        LayoutClass::Chart => {
            if enable_chart_understanding {
                GlmOcrTask::Chart
            } else {
                GlmOcrTask::Caption
            }
        }
        LayoutClass::Picture => GlmOcrTask::Caption,
        LayoutClass::Text
        | LayoutClass::Title
        | LayoutClass::SectionHeader
        | LayoutClass::Caption
        | LayoutClass::ListItem
        | LayoutClass::Footnote
        | LayoutClass::PageHeader
        | LayoutClass::PageFooter
        | LayoutClass::DocumentIndex
        | LayoutClass::Code
        | LayoutClass::CheckboxSelected
        | LayoutClass::CheckboxUnselected
        | LayoutClass::Form
        | LayoutClass::KeyValueRegion => GlmOcrTask::Ocr,
    }
}

/// Wrap a raw GLM-OCR model output string according to its task.
///
/// Table output is left as-is (the model already emits GFM table syntax).
/// Formula output is wrapped in display-math `$$ … $$` fences.
/// Chart output is fenced in a JSON code block.
/// All other tasks return the text verbatim.
#[cfg(feature = "layout-detection")]
/// Strip leading and trailing `$$` delimiters and surrounding whitespace from a string.
///
/// Used to normalize formula content that may have been wrapped by the model.
/// Delegates to the shared math-delimiter stripper, which removes one pair of
/// `$$..$$`, `\[..\]`, or `$..$` delimiters and trims the result.
fn strip_formula_delimiters(content: &str) -> String {
    crate::extraction::derive::strip_math_delimiters(content).to_string()
}

fn wrap_output(task: GlmOcrTask, content: &str) -> String {
    match task {
        GlmOcrTask::Table => content.to_string(),
        GlmOcrTask::Formula => format!("$$\n{}\n$$", content.trim()),
        GlmOcrTask::Chart => format!("```json\n{}\n```", content.trim()),
        GlmOcrTask::Ocr | GlmOcrTask::Caption => content.to_string(),
    }
}

/// Whether a detected layout region is a table, i.e. its bounding box must be
/// carried through to the corresponding `Table` entry in
/// `ExtractedDocument::tables` (issue #187).
#[cfg(feature = "layout-detection")]
fn is_table_region(class: crate::layout::LayoutClass) -> bool {
    class == crate::layout::LayoutClass::Table
}

/// Attach detection-time bounding boxes to the `Table` entries `extract_gfm_tables`
/// parsed out of the assembled markdown, correlating purely by position: the Nth
/// `Table`-class detection with non-empty OCR output corresponds to the Nth GFM
/// table block that appears in `content`, since `process_paired` appends region
/// output to `content` in the same reading order it iterates detections. Extra
/// entries on either side (a table detection whose output failed to parse as GFM,
/// or vice versa) are left without a match rather than mis-attributed.
#[cfg(feature = "layout-detection")]
fn merge_table_bounding_boxes(
    tables: &mut [crate::types::Table],
    detection_bboxes: &[crate::types::extraction::BoundingBox],
) {
    for (table, bbox) in tables.iter_mut().zip(detection_bboxes.iter()) {
        table.bounding_box = Some(*bbox);
    }
}

/// Canonical markdown checkbox marker for a checkbox-class layout detection.
///
/// OCR cannot reliably recover checked-vs-unchecked state from glyph
/// recognition on a tiny checkbox crop (issue #190) — the layout detector has
/// already computed the state, so it is emitted directly instead of routing
/// the crop through the model. Returns `None` for every other layout class.
#[cfg(feature = "layout-detection")]
fn checkbox_marker_for_class(class: crate::layout::LayoutClass) -> Option<&'static str> {
    use crate::layout::LayoutClass;
    match class {
        LayoutClass::CheckboxSelected => Some("[x]"),
        LayoutClass::CheckboxUnselected => Some("[ ]"),
        _ => None,
    }
}

/// GLM-OCR backend using candle transformers.
///
/// A compact vision-language model (0.9 B) for full-page document parsing.
/// Supports text recognition, tables, formulas, charts, and image captioning
/// through a unified interface with markdown output.
///
/// # Constructor notes
///
/// `GlmOcrBackend::new(task, layout_mode)` stores the default task and layout
/// mode. `dtype` defaults to `F32` — the only dtype validated during the smoke
/// test. Use `GlmOcrBackend::with_dtype` to override.
///
/// # Configuration
///
/// GLM-OCR accepts backend options for task, device, and layout mode selection:
/// ```json
/// {
///   "task": "ocr",
///   "device": "auto",
///   "layout_mode": "whole_page",
///   "enable_chart_understanding": false,
///   "cache_dir": "/path/to/huggingface/cache"
/// }
/// ```
///
/// - `task` (string): `"ocr"` (default), `"table"`, `"formula"`, `"chart"`, `"caption"`
/// - `device` (string): `"auto"`, `"cpu"`, `"cuda"`, `"metal"`
/// - `layout_mode` (string): `"whole_page"` (default), `"paired"` (requires `layout-detection` feature)
/// - `enable_chart_understanding` (boolean): route chart regions to chart understanding
/// - `cache_dir` (string, optional): explicit Hugging Face Hub cache root
#[cfg_attr(alef, alef(skip))]
pub struct GlmOcrBackend {
    default_task: GlmOcrTask,
    layout_mode: LayoutMode,
    dtype: DType,
}

impl GlmOcrBackend {
    /// Create a new GLM-OCR backend.
    ///
    /// `dtype` defaults to `F32`. Use [`GlmOcrBackend::with_dtype`] to change it.
    pub fn new(default_task: GlmOcrTask, layout_mode: LayoutMode) -> Self {
        Self {
            default_task,
            layout_mode,
            dtype: DType::F32,
        }
    }

    /// Override the floating-point precision used by the candle engine.
    pub fn with_dtype(mut self, dtype: DType) -> Self {
        self.dtype = dtype;
        self
    }

    /// Parse backend options to extract GLM-OCR-specific configuration.
    ///
    /// Device selection is delegated to [`crate::candle_ocr::resolve_device_preference`]
    /// so the central `AccelerationConfig` is honoured.
    ///
    /// Supports the following backend options (as serde_json values):
    /// - `task` (string): `"ocr"`, `"table"`, `"formula"`, `"chart"`, `"caption"` (default: `"ocr"`)
    /// - `layout_mode` (string): `"whole_page"`, `"paired"` (default: platform-dependent)
    /// - `enable_chart_understanding` (bool): route detected charts to chart task (default: `false`)
    /// - `cache_dir` (string, optional): explicit Hugging Face Hub cache root.
    ///
    /// Unlike [`crate::candle_ocr::PaddleOcrVlBackend`] and
    /// [`crate::candle_ocr::TrocrBackend`], GLM-OCR has no `model_id` option — weights
    /// always come from the single, checksum-pinned `zai-org/GLM-OCR` repository, so an
    /// `hf_revision`/`revision` override could never resolve to different, still-valid
    /// content. Deliberately not exposed: every value other than the internal pin would
    /// fail `GlmOcrEngine::new_with_hf`'s revision check, and the pin itself is already the
    /// default. See the engine-level pinning check for the (retained) defensive guard.
    fn parse_options(&self, config: &OcrConfig) -> Result<GlmOcrOptions> {
        let options = check_backend_options(config.backend_options.as_ref())?;
        let task = match options.task {
            None => self.default_task,
            Some(GlmOcrTaskKind::Ocr) => GlmOcrTask::Ocr,
            Some(GlmOcrTaskKind::Table) => GlmOcrTask::Table,
            Some(GlmOcrTaskKind::Formula) => GlmOcrTask::Formula,
            Some(GlmOcrTaskKind::Chart) => GlmOcrTask::Chart,
            Some(GlmOcrTaskKind::Caption) => GlmOcrTask::Caption,
        };
        let layout_mode = match options.layout_mode {
            None => self.layout_mode,
            Some(GlmOcrLayoutMode::WholePage) => LayoutMode::WholePage,
            #[cfg(feature = "layout-detection")]
            Some(GlmOcrLayoutMode::Paired) => LayoutMode::Paired,
            #[cfg(not(feature = "layout-detection"))]
            Some(GlmOcrLayoutMode::Paired) => {
                return Err(crate::XbergError::validation(
                    "invalid candle-glm-ocr backend_options.layout_mode: paired requires the layout-detection feature"
                        .to_string(),
                ));
            }
        };
        Ok(GlmOcrOptions {
            task,
            device: super::resolve_device_preference(config, options.device),
            layout_mode,
            enable_chart_understanding: options.enable_chart_understanding.unwrap_or(false),
            cache_dir: options.cache_dir.map(PathBuf::from),
        })
    }
}

impl Plugin for GlmOcrBackend {
    fn name(&self) -> &str {
        "candle-glm-ocr"
    }

    fn version(&self) -> String {
        "0.1.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        tracing::debug!(
            task = %self.default_task,
            "Initializing GLM-OCR backend"
        );
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Inherits the `RequiresUpright` default for `page_orientation_handling` — unmeasured, not validated (#657).
#[async_trait]
impl OcrBackend for GlmOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractedDocument> {
        let opts = self.parse_options(config)?;

        if image_bytes.is_empty() {
            return Err(crate::XbergError::Validation {
                message: "Empty image data provided to GLM-OCR".to_string(),
                source: None,
            });
        }

        let image_bytes_owned = image_bytes.to_vec();
        let dtype = self.dtype;
        let cache_dir = opts.cache_dir.unwrap_or_else(hf_hub::resolve_cache_dir);
        let revision = GlmOcrEngine::revision().to_string();

        let (content, formulas, table_bboxes) = match opts.layout_mode {
            LayoutMode::WholePage => {
                let task = opts.task;
                let device = opts.device;
                crate::extraction::image_decode::validate_standard_image_with_default_security_limits(
                    &image_bytes_owned,
                )?;
                let content = tokio::task::spawn_blocking(move || {
                    let engine = get_or_init_engine(device, dtype, cache_dir, revision)?;
                    let output = engine.process_image_with_task(&image_bytes_owned, task).map_err(|e| {
                        crate::XbergError::Ocr {
                            message: format!("GLM-OCR inference failed: {e}"),
                            source: Some(Box::new(e)),
                        }
                    })?;
                    Ok::<String, crate::XbergError>(output.content)
                })
                .await
                .map_err(|e| crate::XbergError::Ocr {
                    message: format!("GLM-OCR task execution failed: {e}"),
                    source: None,
                })??;
                (content, Vec::new(), Vec::new())
            }

            #[cfg(feature = "layout-detection")]
            LayoutMode::Paired => {
                let enable_chart_understanding = opts.enable_chart_understanding;
                process_paired(
                    image_bytes_owned,
                    opts.device,
                    dtype,
                    enable_chart_understanding,
                    cache_dir,
                    revision,
                )
                .await?
            }
        };

        // A `Paired` layout mixes per-region tasks (table/formula/chart regions
        // legitimately emit markup alongside OCR'd text regions), so the bare-LaTeX rule
        // in `filter_implausible_lines` is disabled for it; only a `WholePage` call with
        // the default OCR task is a plain-text task (GH#1676). ~keep
        let plain_text_task = matches!(opts.layout_mode, LayoutMode::WholePage) && opts.task == GlmOcrTask::Ocr;

        let mut document = super::ocr_result::build_ocr_document(
            content,
            formulas,
            image_bytes,
            config,
            super::ocr_result::OcrDocumentContext {
                mime_type: Cow::Borrowed("text/markdown"),
                backend_name: "candle-glm-ocr",
                plain_text_task,
            },
        );
        #[cfg(feature = "layout-detection")]
        merge_table_bounding_boxes(&mut document.tables, &table_bboxes);
        #[cfg(not(feature = "layout-detection"))]
        let _ = table_bboxes;
        Ok(document)
    }

    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractedDocument> {
        let bytes = crate::core::io::read_file_async(path).await?;
        self.process_image(&bytes, config).await
    }

    fn supports_language(&self, _lang: &str) -> bool {
        true
    }

    fn supported_languages(&self) -> Vec<String> {
        vec![
            "eng", "en", "zho", "zh", "jpn", "ja", "kor", "ko", "fra", "fr", "deu", "de", "spa", "es", "ita", "it",
            "por", "pt", "rus", "ru", "ara", "ar", "hin", "hi", "tha", "th", "vie", "vi",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Candle
    }

    fn emits_structured_markdown(&self) -> bool {
        true
    }

    /// GLM-OCR reports no page-level confidence.
    fn confidence_semantics(&self) -> crate::plugins::ConfidenceSemantics {
        crate::plugins::ConfidenceSemantics::None
    }

    // Rotation handling has not been measured for this backend; it stays on the trait's
    // `RequiresUpright` default.
}

#[cfg(feature = "layout-detection")]
type PairedResult = (
    String,
    Vec<crate::types::Formula>,
    Vec<crate::types::extraction::BoundingBox>,
);

#[cfg(feature = "layout-detection")]
struct PairedPage {
    parts: Vec<String>,
    formulas: Vec<crate::types::Formula>,
    table_bboxes: Vec<crate::types::extraction::BoundingBox>,
}

#[cfg(feature = "layout-detection")]
impl PairedPage {
    fn new(region_count: usize) -> Self {
        Self {
            parts: Vec::with_capacity(region_count),
            formulas: Vec::new(),
            table_bboxes: Vec::new(),
        }
    }

    fn push_output(&mut self, detection: &crate::layout::LayoutDetection, task: GlmOcrTask, content: &str) {
        let bbox = &detection.bbox;
        let region_bbox = crate::types::extraction::BoundingBox {
            x0: bbox.x1 as f64,
            y0: bbox.y1 as f64,
            x1: bbox.x2 as f64,
            y1: bbox.y2 as f64,
        };
        let latex_clean = if detection.class_name == crate::layout::LayoutClass::Formula {
            strip_formula_delimiters(content)
        } else {
            content.to_string()
        };
        let wrapped = wrap_output(task, &latex_clean);
        if detection.class_name == crate::layout::LayoutClass::Formula && !latex_clean.is_empty() {
            self.formulas.push(crate::types::Formula {
                latex: latex_clean,
                bbox: Some(region_bbox),
                // The caller renumbers to the real document page. ~keep
                page: Some(1),
            });
        } else if is_table_region(detection.class_name) && !content.trim().is_empty() {
            self.table_bboxes.push(region_bbox);
        }
        self.parts.push(wrapped);
    }

    fn finish(self) -> PairedResult {
        (self.parts.join("\n\n"), self.formulas, self.table_bboxes)
    }
}

#[cfg(feature = "layout-detection")]
fn detect_paired_regions(
    image_bytes: &[u8],
    device: DevicePreference,
    limits: &crate::extractors::security::SecurityLimits,
) -> crate::Result<(image::RgbImage, Vec<crate::layout::LayoutDetection>)> {
    use crate::layout::models::LayoutModel;

    let img = crate::extraction::image_decode::decode_standard_rgb8_with_default_security_limits(image_bytes).map_err(
        |error| crate::XbergError::Ocr {
            message: format!("GLM-OCR paired: image decode failed: {error}"),
            source: Some(Box::new(error)),
        },
    )?;
    crate::layout::engine::validate_layout_batch_peak(&[&img], limits)?;
    let model_path = ensure_layout_model_path()?;
    let layout_model = get_or_init_layout_model(model_path.as_path(), device).map_err(|e| crate::XbergError::Ocr {
        message: format!("GLM-OCR paired: layout detection init failed: {e}"),
        source: Some(Box::new(e)),
    })?;
    let detections = layout_model.lock().detect(&img).map_err(|e| crate::XbergError::Ocr {
        message: format!("GLM-OCR paired: layout detection failed: {e}"),
        source: Some(Box::new(e)),
    })?;
    let mut sorted = detections;
    sorted.sort_by(|a, b| a.bbox.y1.total_cmp(&b.bbox.y1).then(a.bbox.x1.total_cmp(&b.bbox.x1)));
    Ok((img, sorted))
}

#[cfg(feature = "layout-detection")]
fn encode_paired_crop(
    img: &image::RgbImage,
    bbox: &crate::layout::BBox,
    limits: &crate::extractors::security::SecurityLimits,
) -> crate::Result<Vec<u8>> {
    const CROP_PNG_ENCODE_BYTES_PER_PIXEL: u64 = 4;
    const CROP_PNG_ENCODE_FIXED_BYTES: u64 = 256 * 1024;

    let img_width = img.width();
    let img_height = img.height();
    let x = (bbox.x1.max(0.0) as u32).min(img_width.saturating_sub(1));
    let y = (bbox.y1.max(0.0) as u32).min(img_height.saturating_sub(1));
    let w = ((bbox.x2 - bbox.x1).max(1.0) as u32).min(img_width - x);
    let h = ((bbox.y2 - bbox.y1).max(1.0) as u32).min(img_height - y);
    let current_bytes = u64::try_from(img.as_raw().len()).map_err(|_| {
        crate::extraction::image_decode::image_dimension_error(img_width, img_height, u64::MAX, u64::MAX)
    })?;
    let crop_and_encode_bytes =
        crate::extraction::image_decode::decoded_byte_count(w, h, 3 + CROP_PNG_ENCODE_BYTES_PER_PIXEL)?
            .checked_add(CROP_PNG_ENCODE_FIXED_BYTES)
            .ok_or_else(|| {
                crate::extraction::image_decode::image_dimension_error(img_width, img_height, u64::MAX, u64::MAX)
            })?;
    crate::extraction::image_decode::validate_image_live_bytes(
        img_width,
        img_height,
        current_bytes,
        crop_and_encode_bytes,
        limits,
    )?;
    let crop = image::imageops::crop_imm(img, x, y, w, h).to_image();
    let mut crop_bytes: Vec<u8> = Vec::new();
    crop.write_to(&mut std::io::Cursor::new(&mut crop_bytes), image::ImageFormat::Png)
        .map_err(|e| crate::XbergError::Ocr {
            message: format!("GLM-OCR paired: crop encode failed: {e}"),
            source: Some(Box::new(e)),
        })?;
    Ok(crop_bytes)
}

#[cfg(feature = "layout-detection")]
fn infer_paired_region(
    engine: &GlmOcrEngine,
    crop_bytes: &[u8],
    task: GlmOcrTask,
    detection: &crate::layout::LayoutDetection,
) -> crate::Result<Option<String>> {
    let bbox = &detection.bbox;
    // Distinguishes region-overlap duplication from in-region repetition (GH#1675). ~keep
    tracing::debug!(
        class = ?detection.class_name,
        bbox = ?bbox,
        "GLM-OCR paired: processing region"
    );
    match engine.process_image_with_task(crop_bytes, task) {
        Ok(output) => Ok(Some(output.content)),
        Err(CandleOcrError::UnsupportedConfig(ref msg)) => {
            tracing::warn!(
                class = ?detection.class_name,
                bbox = ?bbox,
                reason = %msg,
                "GLM-OCR paired: skipping region (unsupported config)"
            );
            Ok(None)
        }
        Err(e) => Err(crate::XbergError::Ocr {
            message: format!("GLM-OCR paired: region inference failed: {e}"),
            source: Some(Box::new(e)),
        }),
    }
}

/// Paired-mode dispatch returns Markdown, formula metadata and table bounds.
/// Formula page numbers are filled in by the caller. ~keep
#[cfg(feature = "layout-detection")]
async fn process_paired(
    image_bytes: Vec<u8>,
    device: DevicePreference,
    dtype: DType,
    enable_chart_understanding: bool,
    cache_dir: PathBuf,
    revision: String,
) -> crate::Result<PairedResult> {
    tokio::task::spawn_blocking(move || -> crate::Result<PairedResult> {
        let security_limits = crate::extractors::security::SecurityLimits::default();
        let (img, sorted) = detect_paired_regions(&image_bytes, device, &security_limits)?;
        let engine = get_or_init_engine(device, dtype, cache_dir, revision)?;
        if sorted.is_empty() {
            tracing::debug!("GLM-OCR paired: no layout regions detected, falling back to whole-page inference");
            let output = engine
                .process_image_with_task(&image_bytes, GlmOcrTask::Ocr)
                .map_err(|e| crate::XbergError::Ocr {
                    message: format!("GLM-OCR paired (fallback): whole-page inference failed: {e}"),
                    source: Some(Box::new(e)),
                })?;
            return Ok((output.content, Vec::new(), Vec::new()));
        }
        let mut page = PairedPage::new(sorted.len());
        for detection in &sorted {
            // The layout detector owns checkbox state; tiny-crop OCR cannot recover it (#190). ~keep
            if let Some(marker) = checkbox_marker_for_class(detection.class_name) {
                page.parts.push(marker.to_string());
                continue;
            }
            let crop_bytes = encode_paired_crop(&img, &detection.bbox, &security_limits)?;
            let region_task = task_for_label(detection.class_name, enable_chart_understanding);
            if let Some(content) = infer_paired_region(&engine, &crop_bytes, region_task, detection)? {
                page.push_output(detection, region_task, &content);
            }
        }
        Ok(page.finish())
    })
    .await
    .map_err(|e| crate::XbergError::Ocr {
        message: format!("GLM-OCR paired task execution failed: {e}"),
        source: Some(Box::new(e)),
    })?
}

#[cfg(test)]
mod tests;
