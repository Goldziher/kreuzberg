//! High-level layout detection engine.
//!
//! Provides [`LayoutEngine`] as the main entry point for layout detection,
//! with [`LayoutEngineConfig`] for full programmatic control.

use std::path::PathBuf;
use std::time::Instant;

use image::RgbImage;

/// Square model input side the inference workspace estimate is sized from.
///
/// Every built-in backend must fit inside it, which
/// `layout_peak_tests::workspace_side_covers_every_built_in_backend_input`
/// checks against the backends themselves rather than against a copy of their
/// numbers. `CustomModelVariant::Yolox` takes its input dimensions from the
/// caller and is the one backend this ceiling cannot bound: a custom export
/// wider than this under-counts its own workspace.
const LAYOUT_MODEL_MAX_INPUT_SIDE: u32 = 1_280;

/// Bytes per pixel of the NCHW `f32` tensor a preprocessor builds.
///
/// [`crate::layout::preprocessing::preprocess_rescale`] and `preprocess_letterbox`
/// both return an `Array4<f32>` of shape `(1, 3, side, side)`: three channels of
/// four bytes each.
const LAYOUT_MODEL_FLOAT_RGB_BYTES_PER_PIXEL: u64 = 12;

/// Bytes per pixel of the resized RGB8 image held live beside that tensor.
///
/// Both preprocessors resize into an `RgbImage` and read from it while they fill
/// the tensor, so the two buffers peak together rather than in sequence.
const LAYOUT_MODEL_RESIZED_RGB_BYTES_PER_PIXEL: u64 = 3;

/// Model input side of the DocLayNet-trained YOLO export.
#[cfg(feature = "layout-detection")]
const YOLO_DOC_LAY_NET_INPUT_SIDE: u32 = 640;

/// Model input side of the DocStructBench-trained DocLayout-YOLO export, the
/// largest of any built-in backend.
#[cfg(feature = "layout-detection")]
const YOLO_DOC_STRUCT_BENCH_INPUT_SIDE: u32 = 1_024;

use crate::layout::error::LayoutError;
#[cfg(not(target_arch = "wasm32"))]
use crate::layout::model_manager::LayoutModelManager;
use crate::layout::models::LayoutModel;
#[cfg(feature = "layout-detection")]
use crate::layout::models::pp_doclayout_v3::PpDocLayoutV3Model;
use crate::layout::models::rtdetr::RtDetrModel;
#[cfg(feature = "layout-detection")]
use crate::layout::models::yolo::{YoloModel, YoloModelConfig, YoloVariant};
use crate::layout::postprocessing::heuristics;
use crate::layout::types::DetectionResult;
/// Which underlying model architecture to use for layout detection.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq)]
pub enum ModelBackend {
    /// YOLO trained on DocLayNet (11 classes, 640x640 input).
    YoloDocLayNet,
    /// RT-DETR v2 (17 classes, 640x640 input, NMS-free).
    RtDetr,
    /// PP-DocLayout-V3 (25 classes, 800×800 input, PaddleDetection DETR).
    PpDocLayoutV3,
    /// Custom model from a local file path.
    Custom {
        /// Filesystem path to the ONNX model file.
        path: PathBuf,
        /// Model architecture variant for the custom file.
        variant: CustomModelVariant,
    },
}
/// Variant selection for custom model paths, used with [`ModelBackend::Custom`].
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq)]
pub enum CustomModelVariant {
    /// RT-DETR v2 model format.
    RtDetr,
    /// PP-DocLayout-V3 model format.
    PpDocLayoutV3,
    /// YOLO trained on DocLayNet (11 classes).
    YoloDocLayNet,
    /// DocLayout-YOLO trained on DocStructBench (10 classes).
    YoloDocStructBench,
    /// YOLOX with explicit input dimensions.
    Yolox {
        /// Model input width in pixels.
        input_width: u32,
        /// Model input height in pixels.
        input_height: u32,
    },
}
#[cfg_attr(alef, alef(skip))]
/// Full configuration for the layout engine.
///
/// Provides fine-grained control over model selection, thresholds, and
/// postprocessing.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutEngineConfig {
    /// Which model backend to use.
    pub backend: ModelBackend,
    /// Confidence threshold override (None = use model default).
    pub confidence_threshold: Option<f32>,
    /// Whether to apply postprocessing heuristics.
    pub apply_heuristics: bool,
    /// Custom cache directory for model files (None = default).
    pub cache_dir: Option<PathBuf>,
    /// Hardware acceleration for ONNX inference.
    pub acceleration: Option<crate::core::config::acceleration::AccelerationConfig>,
}

impl Default for LayoutEngineConfig {
    fn default() -> Self {
        Self {
            backend: ModelBackend::RtDetr,
            confidence_threshold: None,
            apply_heuristics: true,
            cache_dir: None,
            acceleration: None,
        }
    }
}

/// Granular timing breakdown for a single `detect()` call.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default)]
pub struct DetectTimings {
    /// Time spent in image preprocessing (resize, letterbox, normalize, tensor allocation).
    pub preprocess_ms: f64,
    /// Time for the ONNX `session.run()` call (actual neural network computation).
    pub onnx_ms: f64,
    /// Total time from start of model call to end of raw output decoding.
    pub model_total_ms: f64,
    /// Time spent in postprocessing heuristics (confidence filtering, overlap resolution).
    pub postprocess_ms: f64,
}

/// High-level layout detection engine.
///
/// Wraps model loading, inference, and postprocessing into a single
/// reusable object. Models are downloaded and cached on first use.
#[cfg_attr(alef, alef(skip))]
pub struct LayoutEngine {
    model: Box<dyn LayoutModel>,
    config: LayoutEngineConfig,
    #[cfg(feature = "layout-detection")]
    #[allow(dead_code)]
    thread_budget: usize,
}

impl LayoutEngine {
    #[cfg(feature = "layout-detection")]
    #[allow(dead_code)]
    pub(crate) fn matches_config(&self, config: &LayoutEngineConfig, thread_budget: usize) -> bool {
        self.config == *config && self.thread_budget == thread_budget.max(1)
    }

    /// Create a layout engine from a full config.
    ///
    /// `ModelBackend::RtDetr` and `CustomModelVariant::RtDetr` work on either engine
    /// (ORT-backed `layout-detection` or pure-Rust `layout-tract`). `PpDocLayoutV3` and
    /// every YOLO-based `CustomModelVariant` require the ORT-backed `layout-detection`
    /// feature; under `layout-tract` alone they return a
    /// [`LayoutError::ModelDownload`] explaining why, rather than failing to compile
    /// or panicking.
    ///
    /// Not available on `wasm32`: model resolution goes through
    /// [`LayoutModelManager`], which downloads weights from Hugging Face Hub over
    /// `hf-hub`/`reqwest` — both unavailable on that target. WASM callers construct
    /// a [`LayoutEngine`] from injected model bytes via [`Self::from_rtdetr_bytes`]
    /// instead.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_config(config: LayoutEngineConfig) -> Result<Self, LayoutError> {
        Self::from_config_with_thread_budget(config, crate::core::config::concurrency::resolve_thread_budget(None))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn from_config_with_thread_budget(
        config: LayoutEngineConfig,
        thread_budget: usize,
    ) -> Result<Self, LayoutError> {
        #[cfg(feature = "layout-detection")]
        crate::ort_discovery::ensure_ort_available();
        let thread_budget = thread_budget.max(1);

        let model: Box<dyn LayoutModel> = match &config.backend {
            ModelBackend::YoloDocLayNet => {
                return Err(LayoutError::ModelDownload(
                    "YOLO DocLayNet model is not available for automatic download. \
                     Use ModelBackend::Custom with a local YOLO ONNX file instead."
                        .into(),
                ));
            }
            ModelBackend::RtDetr => {
                let manager = LayoutModelManager::new(config.cache_dir.clone());
                let model_path = manager.ensure_rtdetr_model()?;
                let path_str = model_path.to_string_lossy();
                Box::new(RtDetrModel::from_file(
                    &path_str,
                    config.acceleration.as_ref(),
                    thread_budget,
                )?)
            }
            #[cfg(feature = "layout-detection")]
            ModelBackend::PpDocLayoutV3 => {
                let manager = LayoutModelManager::new(config.cache_dir.clone());
                let model_path = manager.ensure_pp_doclayout_v3_model()?;
                let path_str = model_path.to_string_lossy();
                Box::new(PpDocLayoutV3Model::from_file_with_thread_budget(
                    &path_str,
                    config.acceleration.as_ref(),
                    thread_budget,
                )?)
            }
            #[cfg(not(feature = "layout-detection"))]
            ModelBackend::PpDocLayoutV3 => {
                return Err(LayoutError::ModelDownload(
                    "PP-DocLayout-V3 requires the ORT-backed `layout-detection` feature \
                     (unsupported under the pure-Rust `layout-tract` engine — see \
                     docs-site/src/content/docs/concepts/tract-inference.md)"
                        .into(),
                ));
            }
            ModelBackend::Custom { path, variant } => {
                model_from_custom_variant(path, variant, config.acceleration.as_ref(), thread_budget)?
            }
        };

        Ok(Self {
            model,
            config,
            #[cfg(feature = "layout-detection")]
            thread_budget,
        })
    }

    /// Create a layout engine directly from RT-DETR model bytes already resolved by the caller.
    ///
    /// Bypasses [`LayoutModelManager`] entirely — there is no filesystem path or HTTP
    /// download involved. This is the WASM entry point: the JS host fetches the ONNX
    /// weights (never embedded in the `.wasm` binary) and hands over the bytes, which
    /// flow straight through to the [`crate::inference`] seam's `load_from_memory`.
    /// Only the RT-DETR detection backend is supported this way; `PpDocLayoutV3` and
    /// the YOLO variants require the ORT-backed `layout-detection` feature, which is
    /// not available on `wasm32`.
    pub fn from_rtdetr_bytes(
        rtdetr_bytes: &[u8],
        accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
    ) -> Result<Self, LayoutError> {
        let model: Box<dyn LayoutModel> = Box::new(RtDetrModel::from_bytes(rtdetr_bytes, accel)?);
        Ok(Self {
            model,
            config: LayoutEngineConfig {
                backend: ModelBackend::RtDetr,
                acceleration: accel.cloned(),
                ..LayoutEngineConfig::default()
            },
            #[cfg(feature = "layout-detection")]
            thread_budget: crate::core::config::concurrency::resolve_thread_budget(None),
        })
    }

    /// Run layout detection on an image.
    ///
    /// Returns a [`DetectionResult`] with bounding boxes, classes, and confidence scores.
    /// If `apply_heuristics` is enabled in config, postprocessing is applied automatically.
    pub fn detect(&mut self, img: &RgbImage) -> Result<DetectionResult, LayoutError> {
        self.detect_with_security_limits(img, &crate::extractors::security::SecurityLimits::default())
    }

    pub(crate) fn detect_with_security_limits(
        &mut self,
        img: &RgbImage,
        security_limits: &crate::extractors::security::SecurityLimits,
    ) -> Result<DetectionResult, LayoutError> {
        validate_layout_peak(img, security_limits)
            .map_err(|error| LayoutError::Image(image::ImageError::IoError(std::io::Error::other(error))))?;
        let (result, _timings) = self.detect_timed(img)?;
        for detection in &result.detections {
            tracing::trace!(class = ?detection.class_name, confidence = detection.confidence, "Layout detection result");
        }
        Ok(result)
    }

    /// Decode `image_bytes` and run layout detection.
    ///
    /// A convenience wrapper over [`Self::detect`] for callers that hold encoded
    /// image bytes (PNG/JPEG/…) rather than a decoded [`RgbImage`] — notably the
    /// WASM bridge, which receives image bytes from JS.
    pub fn detect_image_bytes(&mut self, image_bytes: &[u8]) -> Result<DetectionResult, LayoutError> {
        let img = crate::extraction::image_decode::decode_standard_rgb8_with_default_security_limits(image_bytes)
            .map_err(|error| LayoutError::Image(image::ImageError::IoError(std::io::Error::other(error))))?;
        self.detect(&img)
    }

    /// Run layout detection on an image and return granular timing data.
    ///
    /// Identical to [`detect`] but also returns a [`DetectTimings`] breakdown.
    /// Use this when you need per-step profiling (preprocess / onnx / postprocess).
    pub(crate) fn detect_timed(&mut self, img: &RgbImage) -> Result<(DetectionResult, DetectTimings), LayoutError> {
        let model_start = Instant::now();
        let mut detections = if let Some(threshold) = self.config.confidence_threshold {
            self.model.detect_with_threshold(img, threshold)?
        } else {
            self.model.detect(img)?
        };
        let model_total_ms = model_start.elapsed().as_secs_f64() * 1000.0;

        let (preprocess_ms, onnx_ms) = crate::layout::inference_timings::take();

        let page_width = img.width();
        let page_height = img.height();

        let postprocess_start = Instant::now();
        if self.config.apply_heuristics {
            detections = heuristics::apply_heuristics(detections, page_width as f32, page_height as f32);
        }
        let postprocess_ms = postprocess_start.elapsed().as_secs_f64() * 1000.0;

        tracing::info!(
            preprocess_ms,
            onnx_ms,
            model_total_ms,
            postprocess_ms,
            final_detections = detections.len(),
            "Layout engine detect_timed() breakdown"
        );

        let timings = DetectTimings {
            preprocess_ms,
            onnx_ms,
            model_total_ms,
            postprocess_ms,
        };

        Ok((DetectionResult::new(page_width, page_height, detections), timings))
    }

    /// Run layout detection on a batch of images in a single model call.
    ///
    /// Returns one `(DetectionResult, DetectTimings)` tuple per input image.
    /// Postprocessing heuristics are applied per image when enabled in config.
    ///
    /// Timing note: `preprocess_ms` and `onnx_ms` in each `DetectTimings` are the
    /// amortized per-image share of the batch operation (total / N), not independent
    /// per-image measurements.
    #[allow(dead_code)]
    pub(crate) fn detect_batch(
        &mut self,
        images: &[&RgbImage],
    ) -> Result<Vec<(DetectionResult, DetectTimings)>, LayoutError> {
        if images.is_empty() {
            return Ok(Vec::new());
        }

        let model_start = Instant::now();
        let per_image_detections = self.model.detect_batch(images, self.config.confidence_threshold)?;
        let model_total_ms = model_start.elapsed().as_secs_f64() * 1000.0;

        let (preprocess_ms, onnx_ms) = crate::layout::inference_timings::take();

        let postprocess_start = Instant::now();
        let mut results = Vec::with_capacity(images.len());

        for (img, mut detections) in images.iter().zip(per_image_detections) {
            let page_width = img.width();
            let page_height = img.height();

            if self.config.apply_heuristics {
                detections = heuristics::apply_heuristics(detections, page_width as f32, page_height as f32);
            }

            results.push((
                DetectionResult::new(page_width, page_height, detections),
                DetectTimings {
                    preprocess_ms,
                    onnx_ms,
                    model_total_ms,
                    postprocess_ms: 0.0,
                },
            ));
        }

        let postprocess_ms = postprocess_start.elapsed().as_secs_f64() * 1000.0;
        let postprocess_ms_per = postprocess_ms / images.len() as f64;
        for (_, timings) in &mut results {
            timings.postprocess_ms = postprocess_ms_per;
        }

        tracing::info!(
            preprocess_ms,
            onnx_ms,
            model_total_ms,
            postprocess_ms,
            batch_size = images.len(),
            total_detections = results.iter().map(|(r, _)| r.detections.len()).sum::<usize>(),
            "Layout engine detect_batch() breakdown"
        );

        Ok(results)
    }

    /// Benchmark-only access to the internal batched inference path.
    #[cfg(feature = "profiling")]
    #[doc(hidden)]
    pub fn detect_batch_for_benchmark(
        &mut self,
        images: &[&RgbImage],
    ) -> Result<Vec<(DetectionResult, DetectTimings)>, LayoutError> {
        self.detect_batch(images)
    }
}

fn validate_layout_peak(
    image: &RgbImage,
    security_limits: &crate::extractors::security::SecurityLimits,
) -> crate::Result<()> {
    validate_layout_batch_peak(&[image], security_limits)
}

/// Builds the [`LayoutModel`] for a [`ModelBackend::Custom`] path, dispatching on
/// [`CustomModelVariant`]. Split out of [`LayoutEngine::from_config_with_thread_budget`]
/// so that method stays a single per-backend match.
///
/// Carries that method's `wasm32` gate: every variant loads its model from a filesystem
/// path, which `wasm32` has none of -- `RtDetrModel::from_file` does not exist there at all.
/// A helper whose `cfg` is wider than the union of its call sites' is dead code on some leg,
/// and here it was a compile error on the one leg no local gate builds. ~keep
#[cfg(not(target_arch = "wasm32"))]
fn model_from_custom_variant(
    path: &std::path::Path,
    variant: &CustomModelVariant,
    accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
    thread_budget: usize,
) -> Result<Box<dyn LayoutModel>, LayoutError> {
    let path_str = path.to_string_lossy();
    Ok(match variant {
        CustomModelVariant::RtDetr => Box::new(RtDetrModel::from_file(&path_str, accel, thread_budget)?),
        #[cfg(feature = "layout-detection")]
        CustomModelVariant::PpDocLayoutV3 => Box::new(PpDocLayoutV3Model::from_file_with_thread_budget(
            &path_str,
            accel,
            thread_budget,
        )?),
        #[cfg(feature = "layout-detection")]
        CustomModelVariant::YoloDocLayNet => Box::new(YoloModel::from_file(
            &path_str,
            YoloModelConfig::new(
                YoloVariant::DocLayNet,
                YOLO_DOC_LAY_NET_INPUT_SIDE,
                YOLO_DOC_LAY_NET_INPUT_SIDE,
                "Custom-YOLO-DocLayNet",
            ),
            accel,
            thread_budget,
        )?),
        #[cfg(feature = "layout-detection")]
        CustomModelVariant::YoloDocStructBench => Box::new(YoloModel::from_file(
            &path_str,
            YoloModelConfig::new(
                YoloVariant::DocStructBench,
                YOLO_DOC_STRUCT_BENCH_INPUT_SIDE,
                YOLO_DOC_STRUCT_BENCH_INPUT_SIDE,
                "Custom-DocLayout-YOLO",
            ),
            accel,
            thread_budget,
        )?),
        #[cfg(feature = "layout-detection")]
        CustomModelVariant::Yolox {
            input_width,
            input_height,
        } => Box::new(YoloModel::from_file(
            &path_str,
            YoloModelConfig::new(YoloVariant::Yolox, *input_width, *input_height, "Custom-YOLOX"),
            accel,
            thread_budget,
        )?),
        #[cfg(not(feature = "layout-detection"))]
        CustomModelVariant::PpDocLayoutV3
        | CustomModelVariant::YoloDocLayNet
        | CustomModelVariant::YoloDocStructBench
        | CustomModelVariant::Yolox { .. } => {
            return Err(LayoutError::ModelDownload(
                "this custom model variant requires the ORT-backed \
                 `layout-detection` feature (unsupported under the pure-Rust \
                 `layout-tract` engine)"
                    .into(),
            ));
        }
    })
}

fn layout_model_workspace_bytes() -> crate::Result<u64> {
    crate::extraction::image_decode::decoded_byte_count(
        LAYOUT_MODEL_MAX_INPUT_SIDE,
        LAYOUT_MODEL_MAX_INPUT_SIDE,
        LAYOUT_MODEL_FLOAT_RGB_BYTES_PER_PIXEL + LAYOUT_MODEL_RESIZED_RGB_BYTES_PER_PIXEL,
    )
}

pub(crate) fn validate_layout_inference_peak(
    width: u32,
    height: u32,
    current_live_bytes: u64,
    inference_page_count: usize,
    security_limits: &crate::extractors::security::SecurityLimits,
) -> crate::Result<()> {
    let workspace = layout_model_workspace_bytes()?
        .checked_mul(u64::try_from(inference_page_count).unwrap_or(u64::MAX))
        .ok_or_else(|| crate::extraction::image_decode::image_dimension_error(width, height, u64::MAX, u64::MAX))?;
    crate::extraction::image_decode::validate_image_live_bytes(
        width,
        height,
        current_live_bytes,
        workspace,
        security_limits,
    )
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
pub(crate) fn layout_inference_batch_capacity(
    width: u32,
    height: u32,
    current_live_bytes: u64,
    candidate_count: usize,
    security_limits: &crate::extractors::security::SecurityLimits,
) -> crate::Result<usize> {
    if candidate_count == 0 {
        return Ok(0);
    }
    let maximum_live_bytes = u64::try_from(security_limits.max_content_size).unwrap_or(u64::MAX);
    let available = maximum_live_bytes.saturating_sub(current_live_bytes);
    let capacity = usize::try_from(available / layout_model_workspace_bytes()?).unwrap_or(usize::MAX);
    if capacity == 0 {
        validate_layout_inference_peak(width, height, current_live_bytes, 1, security_limits)?;
    }
    Ok(capacity.min(candidate_count).max(1))
}

/// Sum the decoded size of every raster in `images`, reporting an overflow as a dimension
/// error against `width` and `height`. The layout runner's per-chunk accounting and
/// `validate_layout_batch_peak` both call this, so the two agree on what counts as live. ~keep
pub(crate) fn live_raster_bytes<'a>(
    images: impl IntoIterator<Item = &'a RgbImage>,
    width: u32,
    height: u32,
) -> crate::Result<u64> {
    images.into_iter().try_fold(0_u64, |total, image| {
        let bytes = u64::try_from(image.as_raw().len())
            .map_err(|_| crate::extraction::image_decode::image_dimension_error(width, height, u64::MAX, u64::MAX))?;
        total
            .checked_add(bytes)
            .ok_or_else(|| crate::extraction::image_decode::image_dimension_error(width, height, u64::MAX, u64::MAX))
    })
}

pub(crate) fn validate_layout_batch_peak(
    images: &[&RgbImage],
    security_limits: &crate::extractors::security::SecurityLimits,
) -> crate::Result<()> {
    let (width, height) = images.first().map_or((1, 1), |image| image.dimensions());
    let current = live_raster_bytes(images.iter().copied(), width, height)?;
    validate_layout_inference_peak(width, height, current, images.len(), security_limits)
}

#[cfg(all(test, feature = "pdf", feature = "layout-detection"))]
mod layout_peak_tests {
    use super::*;

    #[test]
    fn default_limits_split_eight_letter_pages_into_valid_subbatches() {
        const PAGE_COUNT: usize = 8;
        const PAGE_WIDTH: u32 = 1_275;
        const PAGE_HEIGHT: u32 = 1_650;
        let page_bytes = crate::extraction::image_decode::decoded_byte_count(PAGE_WIDTH, PAGE_HEIGHT, 3)
            .expect("letter raster size");
        let current_live_bytes = page_bytes.checked_mul(PAGE_COUNT as u64).expect("eight page rasters");
        let limits = crate::extractors::security::SecurityLimits::default();

        let capacity =
            layout_inference_batch_capacity(PAGE_WIDTH, PAGE_HEIGHT, current_live_bytes, PAGE_COUNT, &limits)
                .expect("default limits must support at least one inference page");

        assert_eq!(capacity, 2);
        validate_layout_inference_peak(PAGE_WIDTH, PAGE_HEIGHT, current_live_bytes, capacity, &limits)
            .expect("the selected subbatch must fit default limits");
    }

    /// Ties the workspace estimate to the things it estimates, so the three
    /// constants behind it can be checked rather than trusted: the side against
    /// every built-in backend's own input resolution, and the two per-pixel
    /// figures against the buffer types the preprocessors allocate.
    #[test]
    fn workspace_side_covers_every_built_in_backend_input() {
        for (backend, input_side) in [
            ("RT-DETR", crate::layout::models::rtdetr::INPUT_SIZE),
            ("PP-DocLayout-V3", crate::layout::models::pp_doclayout_v3::INPUT_SIZE),
            ("YOLO DocLayNet", YOLO_DOC_LAY_NET_INPUT_SIDE),
            ("DocLayout-YOLO DocStructBench", YOLO_DOC_STRUCT_BENCH_INPUT_SIDE),
        ] {
            assert!(
                input_side <= LAYOUT_MODEL_MAX_INPUT_SIDE,
                "{backend} runs at {input_side} px, above the {LAYOUT_MODEL_MAX_INPUT_SIDE} px workspace estimate"
            );
        }

        assert_eq!(
            LAYOUT_MODEL_FLOAT_RGB_BYTES_PER_PIXEL,
            3 * size_of::<f32>() as u64,
            "the tensor is three f32 channels per pixel"
        );
        assert_eq!(
            LAYOUT_MODEL_RESIZED_RGB_BYTES_PER_PIXEL,
            u64::from(image::ColorType::Rgb8.bytes_per_pixel()),
            "the resized buffer is an RgbImage"
        );
        assert_eq!(
            layout_model_workspace_bytes().expect("workspace estimate must be representable"),
            24_576_000
        );
    }
}

#[cfg(test)]
mod cache_key_tests {
    use super::*;
    use crate::core::config::acceleration::{AccelerationConfig, ExecutionProviderType};

    struct NeverCalledModel;

    impl LayoutModel for NeverCalledModel {
        fn detect(&mut self, _img: &RgbImage) -> Result<Vec<crate::layout::types::LayoutDetection>, LayoutError> {
            panic!("oversized image must be rejected before layout inference")
        }

        fn detect_with_threshold(
            &mut self,
            _img: &RgbImage,
            _threshold: f32,
        ) -> Result<Vec<crate::layout::types::LayoutDetection>, LayoutError> {
            panic!("oversized image must be rejected before layout inference")
        }

        fn name(&self) -> &str {
            "never-called"
        }
    }

    #[test]
    fn detect_image_bytes_rejects_oversized_dimensions_before_inference() {
        let mut engine = LayoutEngine {
            model: Box::new(NeverCalledModel),
            config: LayoutEngineConfig::default(),
            #[cfg(feature = "layout-detection")]
            thread_budget: 1,
        };
        let bytes = crate::extraction::image_decode::bmp_with_declared_dimensions(6000, 6000);

        let error = engine
            .detect_image_bytes(&bytes)
            .expect_err("layout decoding must apply the default security budget");

        assert!(error.to_string().contains("6000x6000"));
        assert!(error.to_string().contains("security_limits.max_content_size"));
    }

    #[test]
    fn engine_config_equality_covers_every_session_and_output_setting() {
        let base = LayoutEngineConfig::default();

        let mut backend = base.clone();
        backend.backend = ModelBackend::YoloDocLayNet;
        assert_ne!(base, backend);

        let mut threshold = base.clone();
        threshold.confidence_threshold = Some(0.75);
        assert_ne!(base, threshold);

        let mut heuristics = base.clone();
        heuristics.apply_heuristics = false;
        assert_ne!(base, heuristics);

        let mut cache_dir = base.clone();
        cache_dir.cache_dir = Some(PathBuf::from("different-cache"));
        assert_ne!(base, cache_dir);

        let mut acceleration = base.clone();
        acceleration.acceleration = Some(AccelerationConfig {
            provider: ExecutionProviderType::Cpu,
            device_id: 1,
        });
        assert_ne!(base, acceleration);
    }
}

#[cfg(test)]
#[cfg(all(
    feature = "pdf",
    feature = "layout-detection",
    feature = "ocr",
    feature = "liter-llm",
    feature = "tokio-runtime"
))]
mod auto_gate_vlm_region_tests;
