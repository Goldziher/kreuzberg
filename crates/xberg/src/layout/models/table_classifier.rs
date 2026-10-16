//! PP-LCNet table classifier for wired vs wireless table detection.
//!
//! Classifies a cropped table image as either wired (bordered) or wireless
//! (borderless), routing to the appropriate SLANeXT model variant.
//!
//! Model: PP-LCNet_x1_0_table_cls
//! Input:  `x` shape `[batch, 3, 224, 224]` f32 (fixed size, ImageNet normalization)
//! Output: `fetch_name_0` shape `[batch, 2]` f32 — [wired_score, wireless_score]

use std::path::Path;

use image::RgbImage;
use ndarray::Array4;

use crate::inference::{InferenceSession, InferenceTensor, default_backend};
use crate::layout::error::LayoutError;

/// PP-LCNet fixed input dimensions.
const INPUT_SIZE: u32 = 224;

/// ImageNet normalization mean, applied in BGR channel order.
///
/// PaddleOCR uses OpenCV (BGR) convention: B=0.485, G=0.456, R=0.406.
const IMAGENET_MEAN_BGR: [f32; 3] = [0.485, 0.456, 0.406];

/// ImageNet normalization std, applied in BGR channel order.
const IMAGENET_STD_BGR: [f32; 3] = [0.229, 0.224, 0.225];

/// Table type classification result.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TableType {
    /// Bordered table with visible gridlines.
    Wired,
    /// Borderless table without visible gridlines.
    Wireless,
}

impl TableType {
    /// Human-readable label (`"wired"` or `"wireless"`).
    pub fn name(&self) -> &'static str {
        match self {
            Self::Wired => "wired",
            Self::Wireless => "wireless",
        }
    }
}

/// PP-LCNet table classifier model.
#[cfg_attr(alef, alef(skip))]
pub struct TableClassifier {
    session: Box<dyn InferenceSession>,
    input_name: String,
}

impl TableClassifier {
    /// Load the table classifier ONNX model from a file path.
    ///
    /// The session (with its `GraphOptimizationLevel::All` config, thread budget,
    /// execution-provider selection, and CPU-only fallback) is built by the
    /// [`crate::inference`] seam's default backend, so this works on either engine:
    /// the ORT-backed `layout-detection` feature or the pure-Rust `layout-tract` variant.
    pub fn from_file(
        path: &str,
        accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
    ) -> Result<Self, LayoutError> {
        Self::from_file_with_thread_budget(
            path,
            accel,
            crate::core::config::concurrency::resolve_thread_budget(None),
        )
    }

    pub(crate) fn from_file_with_thread_budget(
        path: &str,
        accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
        thread_budget: usize,
    ) -> Result<Self, LayoutError> {
        let session = default_backend()
            .load_with_thread_budget(Path::new(path), accel, thread_budget)
            .map_err(|e| LayoutError::Inference(e.to_string()))?;
        let input_name = session
            .input_names()
            .first()
            .cloned()
            .ok_or_else(|| LayoutError::InvalidOutput("table classifier model has no inputs".to_string()))?;
        Ok(Self { session, input_name })
    }

    /// Load the table classifier ONNX model from an in-memory byte buffer.
    ///
    /// Used where there is no filesystem path to read from, e.g. WASM builds where
    /// the JS host fetches and hands over the model weights directly. Uses the same
    /// engine-neutral [`crate::inference`] seam as [`Self::from_file`].
    pub fn from_bytes(
        model_bytes: &[u8],
        accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
    ) -> Result<Self, LayoutError> {
        let session = default_backend()
            .load_from_memory(model_bytes, accel)
            .map_err(|e| LayoutError::Inference(e.to_string()))?;
        let input_name = session
            .input_names()
            .first()
            .cloned()
            .ok_or_else(|| LayoutError::InvalidOutput("table classifier model has no inputs".to_string()))?;
        Ok(Self { session, input_name })
    }

    /// Classify a cropped table image as wired or wireless.
    pub fn classify(&mut self, table_img: &RgbImage) -> Result<TableType, LayoutError> {
        tracing::trace!(
            input_width = table_img.width(),
            input_height = table_img.height(),
            "TableClassifier: starting classification"
        );

        let input_tensor = preprocess_lcnet(table_img)?;

        let inference_start = std::time::Instant::now();
        let outputs = self
            .session
            .run(vec![(
                self.input_name.clone(),
                InferenceTensor::F32(input_tensor.into_dyn()),
            )])
            .map_err(|e| LayoutError::Inference(e.to_string()))?;
        let inference_ms = inference_start.elapsed().as_secs_f64() * 1000.0;

        tracing::trace!(
            inference_ms = format!("{:.1}", inference_ms),
            "TableClassifier: inference complete"
        );

        for (_name, value) in &outputs {
            if let Some(array) = value.as_f32() {
                let data = array.as_slice().unwrap_or(&[]);
                if data.len() >= 2 {
                    let raw_wired = data[0];
                    let raw_wireless = data[1];

                    let max_val = raw_wired.max(raw_wireless);
                    let exp_wired = (raw_wired - max_val).exp();
                    let exp_wireless = (raw_wireless - max_val).exp();
                    let sum_exp = exp_wired + exp_wireless;
                    let prob_wired = exp_wired / sum_exp;
                    let prob_wireless = exp_wireless / sum_exp;

                    let result = if prob_wired >= prob_wireless {
                        TableType::Wired
                    } else {
                        TableType::Wireless
                    };
                    tracing::debug!(
                        raw_wired = format!("{:.3}", raw_wired),
                        raw_wireless = format!("{:.3}", raw_wireless),
                        prob_wired = format!("{:.3}", prob_wired),
                        prob_wireless = format!("{:.3}", prob_wireless),
                        result = result.name(),
                        "Table classification result"
                    );
                    return Ok(result);
                }
            }
        }

        tracing::warn!("TableClassifier: could not parse output, defaulting to wireless");
        Ok(TableType::Wireless)
    }
}

/// Minimum edge length before center-crop.
const MIN_EDGE: u32 = 256;

/// Preprocess an image for PP-LCNet table classifier.
///
/// Matches MinerU's `paddle_table_cls.py` preprocessing exactly:
/// 1. Resize so shortest edge = 256 (aspect-preserving)
/// 2. Center-crop to 224×224
/// 3. Normalize in BGR channel order (PaddleOCR convention)
/// 4. Layout: NCHW `[1, 3, 224, 224]` f32
fn preprocess_lcnet(img: &RgbImage) -> Result<Array4<f32>, LayoutError> {
    let orig_w = img.width();
    let orig_h = img.height();

    let scale = MIN_EDGE as f32 / orig_w.min(orig_h) as f32;
    let new_w = (orig_w as f32 * scale).round().max(1.0) as u32;
    let new_h = (orig_h as f32 * scale).round().max(1.0) as u32;

    let resized = image::imageops::resize(img, new_w, new_h, image::imageops::FilterType::Triangle);

    let crop_x = (new_w.saturating_sub(INPUT_SIZE)) / 2;
    let crop_y = (new_h.saturating_sub(INPUT_SIZE)) / 2;
    let crop_w = INPUT_SIZE.min(new_w);
    let crop_h = INPUT_SIZE.min(new_h);
    let cropped = image::imageops::crop_imm(&resized, crop_x, crop_y, crop_w, crop_h).to_image();

    let w = INPUT_SIZE as usize;
    let h = INPUT_SIZE as usize;
    let hw = h * w;

    const INV_255: f32 = 1.0 / 255.0;
    let alpha_b = INV_255 / IMAGENET_STD_BGR[0];
    let alpha_g = INV_255 / IMAGENET_STD_BGR[1];
    let alpha_r = INV_255 / IMAGENET_STD_BGR[2];
    let beta_b = -IMAGENET_MEAN_BGR[0] / IMAGENET_STD_BGR[0];
    let beta_g = -IMAGENET_MEAN_BGR[1] / IMAGENET_STD_BGR[1];
    let beta_r = -IMAGENET_MEAN_BGR[2] / IMAGENET_STD_BGR[2];

    let mut data = vec![0.0f32; 3 * hw];
    let pixels = cropped.as_raw();

    for (i, pixel) in pixels.as_chunks::<3>().0.iter().enumerate() {
        let r = pixel[0] as f32;
        let g = pixel[1] as f32;
        let b = pixel[2] as f32;
        data[i] = b * alpha_b + beta_b;
        data[hw + i] = g * alpha_g + beta_g;
        data[2 * hw + i] = r * alpha_r + beta_r;
    }

    Array4::from_shape_vec((1, 3, h, w), data)
        .map_err(|e| LayoutError::InvalidOutput(format!("preprocess_lcnet shape mismatch: {e}")))
}
