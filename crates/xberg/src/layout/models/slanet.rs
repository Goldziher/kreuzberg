//! SLANeXT table structure recognition model.
//!
//! Takes a cropped table image and outputs HTML structure tokens with cell
//! bounding box polygons for table reconstruction.
//!
//! SLANeXT is PaddlePaddle's sequence-to-sequence table structure recognition
//! model. Two variants exist: wired (bordered tables) and wireless (borderless).
//!
//! Model: SLANeXT (PaddleOCR table structure recognition)
//! Input:  `x` shape `[batch, 3, 512, 512]` f32 (fixed size, ImageNet normalization)
//! Output 0: `fetch_name_0` shape `[batch, seq_len, 8]` f32 — cell polygon bboxes (4 corners × 2 coords)
//! Output 1: `fetch_name_1` shape `[batch, seq_len, 50]` f32 — structure token logits (50-token vocab)
//!
//! Token vocabulary (50 tokens):
//! Index 0: "sos" (start of sequence)
//! Index 1-48: HTML structure tokens from table_structure_dict_ch.txt
//! Index 49: "eos" (end of sequence)

use image::RgbImage;
use ndarray::Array4;
use ort::{inputs, session::Session, value::Tensor};

use crate::layout::error::LayoutError;
use crate::layout::session::build_session;

/// SLANeXT fixed input dimensions.
const INPUT_SIZE: u32 = 512;

/// ImageNet normalization mean, applied in BGR channel order.
///
/// PaddleOCR uses OpenCV (BGR) convention: these values are applied as
/// B=0.485, G=0.456, R=0.406 — matching PaddleOCR's `cv2.split()` order.
const IMAGENET_MEAN_BGR: [f32; 3] = [0.485, 0.456, 0.406];

/// ImageNet normalization std, applied in BGR channel order.
const IMAGENET_STD_BGR: [f32; 3] = [0.229, 0.224, 0.225];

/// Vocabulary size for structure token logits.
const VOCAB_SIZE: usize = 50;

/// Index of the "eos" token in the vocabulary.
const EOS_TOKEN_IDX: usize = 49;

/// Index of the "sos" token in the vocabulary.
const SOS_TOKEN_IDX: usize = 0;

/// HTML structure token dictionary (indices 1..=48).
/// Maps vocabulary index to HTML structure token string.
///
/// Source: PaddleOCR table_structure_dict_ch.txt with "sos" prepended and "eos" appended.
const TOKEN_DICT: [&str; VOCAB_SIZE] = [
    "sos",
    "<thead>",
    "</thead>",
    "<tbody>",
    "</tbody>",
    "<tr>",
    "</tr>",
    "<td>",
    "<td",
    ">",
    "</td>",
    " colspan=\"2\"",
    " colspan=\"3\"",
    " colspan=\"4\"",
    " colspan=\"5\"",
    " colspan=\"6\"",
    " colspan=\"7\"",
    " colspan=\"8\"",
    " colspan=\"9\"",
    " colspan=\"10\"",
    " colspan=\"11\"",
    " colspan=\"12\"",
    " colspan=\"13\"",
    " colspan=\"14\"",
    " colspan=\"15\"",
    " colspan=\"16\"",
    " colspan=\"17\"",
    " colspan=\"18\"",
    " colspan=\"19\"",
    " colspan=\"20\"",
    " rowspan=\"2\"",
    " rowspan=\"3\"",
    " rowspan=\"4\"",
    " rowspan=\"5\"",
    " rowspan=\"6\"",
    " rowspan=\"7\"",
    " rowspan=\"8\"",
    " rowspan=\"9\"",
    " rowspan=\"10\"",
    " rowspan=\"11\"",
    " rowspan=\"12\"",
    " rowspan=\"13\"",
    " rowspan=\"14\"",
    " rowspan=\"15\"",
    " rowspan=\"16\"",
    " rowspan=\"17\"",
    " rowspan=\"18\"",
    " rowspan=\"19\"",
    " rowspan=\"20\"",
    "eos",
];

#[cfg_attr(alef, alef(skip))]
/// A single cell detected by SLANeXT.
#[derive(Debug, Clone)]
pub struct SlanetCell {
    /// Bounding box polygon in image pixel coordinates.
    /// Format: [x1, y1, x2, y2, x3, y3, x4, y4] (4 corners, clockwise from top-left).
    pub polygon: [f32; 8],
    /// Axis-aligned bounding box derived from polygon: [left, top, right, bottom].
    pub bbox: [f32; 4],
    /// Row index in the table (0-based).
    pub row: usize,
    /// Column index within the row (0-based).
    pub col: usize,
}
#[cfg_attr(alef, alef(skip))]
/// SLANeXT recognition result for a single table image.
#[derive(Debug, Clone)]
pub struct SlanetResult {
    /// Detected cells with bounding boxes and grid positions.
    pub cells: Vec<SlanetCell>,
    /// Number of rows in the table.
    pub num_rows: usize,
    /// Maximum number of columns across all rows.
    pub num_cols: usize,
    /// Average structure prediction confidence.
    pub confidence: f32,
    /// Raw HTML structure tokens (for debugging).
    pub structure_tokens: Vec<&'static str>,
}

#[cfg_attr(alef, alef(skip))]
/// SLANeXT table structure recognition model.
///
/// Wraps an ORT session for SLANeXT ONNX model and provides preprocessing,
/// inference, and post-processing in a single `recognize` call.
pub struct SlanetModel {
    session: Session,
    input_name: String,
}

impl SlanetModel {
    /// Load a SLANeXT ONNX model from a file path.
    pub(crate) fn from_file(
        path: &str,
        accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
        thread_budget: usize,
    ) -> Result<Self, LayoutError> {
        let thread_budget = thread_budget.max(1);
        let session = match build_session(path, accel, thread_budget) {
            Ok(s) => s,
            Err(first_err) => {
                tracing::warn!("SLANeXT: platform EP failed ({first_err}), retrying with CPU-only");
                match Self::build_cpu_session(path, thread_budget) {
                    Ok(s) => s,
                    Err(cpu_err) => {
                        tracing::warn!("SLANeXT: CPU-only also failed: {cpu_err}");
                        return Err(cpu_err);
                    }
                }
            }
        };
        let input_name = session.inputs()[0].name().to_string();
        Ok(Self { session, input_name })
    }

    /// Build a CPU-only ORT session (no CoreML/CUDA).
    fn build_cpu_session(path: &str, thread_budget: usize) -> Result<Session, LayoutError> {
        use ort::session::builder::GraphOptimizationLevel;
        let mut builder = Session::builder(crate::ort_discovery::ort_environment().map_err(LayoutError::Ort)?)?
            .with_optimization_level(GraphOptimizationLevel::All)
            .map_err(|e| LayoutError::Ort(ort::Error::new(e.message())))?
            .with_intra_threads(thread_budget)
            .map_err(|e| LayoutError::Ort(ort::Error::new(e.message())))?
            .with_inter_threads(1)
            .map_err(|e| LayoutError::Ort(ort::Error::new(e.message())))?;
        Ok(builder.commit_from_file(path)?)
    }

    /// Recognize table structure from a cropped table image.
    ///
    /// Returns a [`SlanetResult`] with detected cells, grid dimensions,
    /// and structure tokens.
    pub(crate) fn recognize(&mut self, table_img: &RgbImage) -> Result<SlanetResult, LayoutError> {
        let orig_w = table_img.width() as f32;
        let orig_h = table_img.height() as f32;

        tracing::trace!(
            input_width = table_img.width(),
            input_height = table_img.height(),
            "SLANeXT: starting recognition"
        );

        let preprocess_start = std::time::Instant::now();
        let input_tensor = preprocess_slanet(table_img);
        let preprocess_ms = preprocess_start.elapsed().as_secs_f64() * 1000.0;
        let tensor = Tensor::from_array(input_tensor)?;

        tracing::trace!(
            preprocess_ms = format!("{:.1}", preprocess_ms),
            "SLANeXT: preprocessing complete"
        );

        let inference_start = std::time::Instant::now();
        let outputs = self.session.run(inputs![
            self.input_name.as_str() => tensor
        ])?;
        let inference_ms = inference_start.elapsed().as_secs_f64() * 1000.0;

        tracing::trace!(
            inference_ms = format!("{:.1}", inference_ms),
            "SLANeXT: ONNX inference complete"
        );

        let mut float_outputs: Vec<(Vec<usize>, Vec<f32>)> = Vec::new();
        for (_name, value) in outputs.iter() {
            if let Ok(view) = value.try_extract_tensor::<f32>() {
                let shape: Vec<usize> = view.0.iter().map(|&d| d as usize).collect();
                let data: Vec<f32> = view.1.to_vec();
                float_outputs.push((shape, data));
            }
        }

        if float_outputs.len() < 2 {
            return Err(LayoutError::InvalidOutput(format!(
                "SLANeXT expected 2 float outputs, got {}",
                float_outputs.len()
            )));
        }

        let (bbox_shape, bbox_data, logits_shape, logits_data) = if float_outputs[0].0.last() == Some(&8) {
            let (bs, bd) = float_outputs.remove(0);
            let (ls, ld) = float_outputs.remove(0);
            (bs, bd, ls, ld)
        } else {
            let (ls, ld) = float_outputs.remove(0);
            let (bs, bd) = float_outputs.remove(0);
            (bs, bd, ls, ld)
        };

        let seq_len = logits_shape.get(1).copied().unwrap_or(0);
        let vocab = logits_shape.last().copied().unwrap_or(0);

        tracing::trace!(
            bbox_shape = ?bbox_shape,
            logits_shape = ?logits_shape,
            seq_len,
            vocab,
            "SLANeXT: output tensor shapes"
        );

        if seq_len == 0 || vocab < VOCAB_SIZE {
            return Ok(SlanetResult {
                cells: Vec::new(),
                num_rows: 0,
                num_cols: 0,
                confidence: 0.0,
                structure_tokens: Vec::new(),
            });
        }

        let mut tokens: Vec<&'static str> = Vec::new();
        let mut token_entries: Vec<(usize, usize)> = Vec::new();
        let mut scores: Vec<f32> = Vec::new();

        for t in 0..seq_len {
            let offset = t * vocab;
            let logits_slice = &logits_data[offset..offset + vocab];

            let (idx, score) = argmax_with_score(logits_slice);

            if t > 0 && idx == EOS_TOKEN_IDX {
                break;
            }
            if idx == SOS_TOKEN_IDX {
                continue;
            }

            if idx < VOCAB_SIZE {
                tokens.push(TOKEN_DICT[idx]);
                token_entries.push((idx, t));
                scores.push(score);
            }
        }

        let confidence = if scores.is_empty() {
            0.0
        } else {
            scores.iter().sum::<f32>() / scores.len() as f32
        };

        tracing::trace!(
            token_count = tokens.len(),
            tokens_preview = ?tokens.iter().take(40).collect::<Vec<_>>(),
            "SLANeXT: decoded structure tokens"
        );

        let mut cells = Vec::new();
        let mut current_row: usize = 0;
        let mut current_col: usize = 0;
        let mut max_cols: usize = 0;
        let mut in_td = false;

        for &(idx, seq_pos) in &token_entries {
            let token = TOKEN_DICT[idx];
            match token {
                "<tr>" => {
                    if current_row > 0 || current_col > 0 {
                        max_cols = max_cols.max(current_col);
                        current_row += 1;
                    }
                    current_col = 0;
                }
                "</tr>" => {
                    max_cols = max_cols.max(current_col);
                }
                "<td>" | "<td" => {
                    let bbox_offset = seq_pos * 8;
                    if bbox_offset + 8 <= bbox_data.len() {
                        let polygon = [
                            bbox_data[bbox_offset],
                            bbox_data[bbox_offset + 1],
                            bbox_data[bbox_offset + 2],
                            bbox_data[bbox_offset + 3],
                            bbox_data[bbox_offset + 4],
                            bbox_data[bbox_offset + 5],
                            bbox_data[bbox_offset + 6],
                            bbox_data[bbox_offset + 7],
                        ];

                        let mut pixel_polygon = polygon;
                        pixel_polygon[0] = polygon[0] * orig_w;
                        pixel_polygon[2] = polygon[2] * orig_w;
                        pixel_polygon[4] = polygon[4] * orig_w;
                        pixel_polygon[6] = polygon[6] * orig_w;
                        pixel_polygon[1] = polygon[1] * orig_h;
                        pixel_polygon[3] = polygon[3] * orig_h;
                        pixel_polygon[5] = polygon[5] * orig_h;
                        pixel_polygon[7] = polygon[7] * orig_h;

                        for i in (0..8).step_by(2) {
                            pixel_polygon[i] = pixel_polygon[i].clamp(0.0, orig_w);
                        }
                        for i in (1..8).step_by(2) {
                            pixel_polygon[i] = pixel_polygon[i].clamp(0.0, orig_h);
                        }

                        let left = pixel_polygon[0]
                            .min(pixel_polygon[2])
                            .min(pixel_polygon[4])
                            .min(pixel_polygon[6]);
                        let top = pixel_polygon[1]
                            .min(pixel_polygon[3])
                            .min(pixel_polygon[5])
                            .min(pixel_polygon[7]);
                        let right = pixel_polygon[0]
                            .max(pixel_polygon[2])
                            .max(pixel_polygon[4])
                            .max(pixel_polygon[6]);
                        let bottom = pixel_polygon[1]
                            .max(pixel_polygon[3])
                            .max(pixel_polygon[5])
                            .max(pixel_polygon[7]);

                        tracing::trace!(
                            seq_pos,
                            row = current_row,
                            col = current_col,
                            raw_bbox = ?(polygon[0], polygon[1], polygon[2], polygon[3]),
                            pixel_bbox = ?(left, top, right, bottom),
                            "SLANeXT: cell bbox extracted"
                        );

                        cells.push(SlanetCell {
                            polygon: pixel_polygon,
                            bbox: [left, top, right, bottom],
                            row: current_row,
                            col: current_col,
                        });
                    }
                    if token == "<td>" {
                        current_col += 1;
                    } else {
                        in_td = true;
                    }
                }
                ">" if in_td => {
                    current_col += 1;
                    in_td = false;
                }
                "</td>" => {}
                _ => {}
            }
        }
        max_cols = max_cols.max(current_col);
        let num_rows = if max_cols > 0 { current_row + 1 } else { 0 };

        tracing::debug!(
            num_cells = cells.len(),
            num_rows,
            num_cols = max_cols,
            confidence = format!("{:.3}", confidence),
            tokens = tokens.len(),
            "SLANeXT inference result"
        );

        Ok(SlanetResult {
            cells,
            num_rows,
            num_cols: max_cols,
            confidence,
            structure_tokens: tokens,
        })
    }
}

/// Preprocess an image for SLANeXT inference.
///
/// Pipeline:
/// 1. Aspect-preserving resize to fit within 512×512
/// 2. Pad to exactly 512×512 with zeros (letterbox)
/// 3. Normalize: ImageNet mean/std in RGB channel order
/// 4. Layout: NCHW `[1, 3, 512, 512]` f32
///
/// Returns `tensor` — the preprocessed NCHW input for ONNX inference.
/// Bbox decode uses original image dimensions directly (not ratios).
fn preprocess_slanet(img: &RgbImage) -> Array4<f32> {
    let orig_w = img.width();
    let orig_h = img.height();

    let scale = (INPUT_SIZE as f32 / orig_w as f32).min(INPUT_SIZE as f32 / orig_h as f32);
    let new_w = (orig_w as f32 * scale).round().max(1.0) as u32;
    let new_h = (orig_h as f32 * scale).round().max(1.0) as u32;

    let resized = image::imageops::resize(img, new_w, new_h, image::imageops::FilterType::Triangle);

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
    for i in 0..hw {
        data[i] = beta_b;
        data[hw + i] = beta_g;
        data[2 * hw + i] = beta_r;
    }

    let resized_pixels = resized.as_raw();
    let rw = new_w as usize;
    let rh = new_h as usize;

    for y in 0..rh {
        let row_start = y * rw * 3;
        let dst_row_start = y * w;
        for x in 0..rw {
            let src_idx = row_start + x * 3;
            let dst_idx = dst_row_start + x;
            let r = resized_pixels[src_idx] as f32;
            let g = resized_pixels[src_idx + 1] as f32;
            let b = resized_pixels[src_idx + 2] as f32;
            data[dst_idx] = b * alpha_b + beta_b;
            data[hw + dst_idx] = g * alpha_g + beta_g;
            data[2 * hw + dst_idx] = r * alpha_r + beta_r;
        }
    }

    Array4::from_shape_vec((1, 3, h, w), data).expect("shape mismatch in preprocess_slanet")
}

/// Argmax with maximum score (softmax not needed — we just want the index).
fn argmax_with_score(logits: &[f32]) -> (usize, f32) {
    let mut max_idx = 0;
    let mut max_val = f32::NEG_INFINITY;
    for (i, &v) in logits.iter().enumerate() {
        if v > max_val {
            max_val = v;
            max_idx = i;
        }
    }
    let max_logit = max_val;
    let sum_exp: f32 = logits.iter().map(|&v| (v - max_logit).exp()).sum();
    let prob = 1.0 / sum_exp;
    (max_idx, prob)
}
