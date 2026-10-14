//! Native ONNX Runtime backend (`ort`).
//!
//! Wraps an `ort` [`Session`] behind the runtime-neutral [`ModelBackend`] seam so the
//! rest of the crate speaks only in [`Tensor`]s; `ort` types are referenced only here
//! and in [`crate::base_net`]'s session-builder customization hook.

use std::sync::Mutex;

use ndarray::{ArrayD, IxDyn};
use ort::session::Session;
use ort::session::builder::{GraphOptimizationLevel, SessionBuilder};
use ort::value::Tensor as OrtTensor;

use super::{ModelBackend, Tensor};
use crate::ocr_error::OcrError;

/// ONNX Runtime session wrapper.
///
/// The session is held behind a [`Mutex`] because `ort`'s `Session::run` needs
/// `&mut self` while [`ModelBackend::run`] takes `&self`; the mutex serializes calls
/// so the backend stays `Send + Sync` without the raw-pointer cast the pre-seam code
/// used to work around this (borrowing `&self` as a mutable session pointer).
pub(crate) struct OrtBackend {
    session: Mutex<Session>,
}

impl OrtBackend {
    /// Build a session from ONNX bytes read into memory.
    ///
    /// Mirrors the pre-seam `BaseNet::init_model_from_memory` exactly, including its
    /// `catch_unwind` guard around session construction (ONNX Runtime's native
    /// library can panic on a malformed model): `builder_fn` fully replaces the
    /// default session configuration when supplied (matching today's
    /// `init_models_custom` behavior); otherwise the session runs at `All` graph
    /// optimization with `num_thread` intra-op threads and one inter-op thread.
    pub(crate) fn load(
        model_bytes: &[u8],
        num_thread: usize,
        builder_fn: Option<fn(SessionBuilder) -> Result<SessionBuilder, ort::Error>>,
    ) -> Result<Self, OcrError> {
        let session = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut builder = Self::session_builder(num_thread, builder_fn)?;
            builder.commit_from_memory(model_bytes).map_err(OcrError::from)
        }))
        .map_err(|_| OcrError::Ort(ort::Error::new("ORT session initialization panicked")))??;
        Ok(Self {
            session: Mutex::new(session),
        })
    }

    fn environment() -> Result<&'static ort::environment::Environment, OcrError> {
        static ENVIRONMENT: std::sync::OnceLock<ort::environment::Environment> = std::sync::OnceLock::new();
        if let Some(environment) = ENVIRONMENT.get() {
            return Ok(environment);
        }
        let environment = ort::init().build()?;
        Ok(ENVIRONMENT.get_or_init(|| environment))
    }

    fn session_builder(
        num_thread: usize,
        builder_fn: Option<fn(SessionBuilder) -> Result<SessionBuilder, ort::Error>>,
    ) -> Result<SessionBuilder, OcrError> {
        let builder = Session::builder(Self::environment()?)?;
        let builder = match builder_fn {
            Some(custom) => custom(builder)?,
            None => builder
                .with_optimization_level(GraphOptimizationLevel::All)
                .map_err(|e| OcrError::Ort(ort::Error::new(e.message())))?
                .with_intra_threads(num_thread)
                .map_err(|e| OcrError::Ort(ort::Error::new(e.message())))?
                .with_inter_threads(1)
                .map_err(|e| OcrError::Ort(ort::Error::new(e.message())))?,
        };

        Ok(builder)
    }
}

impl ModelBackend for OrtBackend {
    fn name(&self) -> &str {
        "ort"
    }

    fn run(&self, input: Tensor) -> Result<Tensor, OcrError> {
        let (shape, data) = input_buffer(input);
        let value = OrtTensor::from_array((shape, data))?;

        // A panic in a prior `run` poisons the mutex; recover the guard so one bad ~keep
        // call does not permanently brick every later inference on this backend. ~keep
        let mut session = self.session.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let outputs = session.run(ort::inputs![value])?;

        let output = outputs
            .values()
            .next()
            .ok_or_else(|| OcrError::inference("ONNX Runtime returned no output tensor"))?;
        let (out_shape, out_data) = output.try_extract_tensor::<f32>()?;
        array_from_output(out_shape, out_data)
    }

    fn custom_metadata(&self, key: &str) -> Option<String> {
        let session = self.session.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        session.metadata().ok()?.custom(key)
    }
}

/// Move a tensor's backing buffer out in row-major order, copy-free when possible.
///
/// A standard-layout tensor whose buffer starts at offset 0 and is sized to the shape
/// has that buffer taken by move (the common case, copy-free). Any other layout -- a
/// nonzero offset, an over-long backing buffer, or a transposed view -- is
/// materialized into a fresh contiguous buffer. Either way the returned [`Vec`]
/// matches the tensor's logical `iter().copied()` order.
fn input_buffer(input: Tensor) -> (Vec<i64>, Vec<f32>) {
    let shape = shape_to_i64(input.shape());
    let element_count: usize = input.shape().iter().product();
    if input.is_standard_layout() {
        let (data, offset) = input.into_raw_vec_and_offset();
        let start = offset.unwrap_or(0);
        if start == 0 && data.len() == element_count {
            return (shape, data);
        }
        return (shape, data.into_iter().skip(start).take(element_count).collect());
    }
    let (data, _) = input.as_standard_layout().into_owned().into_raw_vec_and_offset();
    (shape, data)
}

/// Convert an ndarray shape (`&[usize]`) to the `i64` dims `ort` expects.
fn shape_to_i64(shape: &[usize]) -> Vec<i64> {
    shape.iter().map(|&dim| dim as i64).collect()
}

/// Rebuild an owned [`Tensor`] from an `ort` output shape and its data slice.
///
/// The output shape is preserved exactly; a mismatch between the declared shape and
/// the data length is surfaced as an [`OcrError::Inference`].
fn array_from_output(dims: &[i64], data: &[f32]) -> Result<Tensor, OcrError> {
    if let Some(&negative) = dims.iter().find(|&&dim| dim < 0) {
        return Err(OcrError::inference(format!(
            "ONNX Runtime returned a negative output dimension {negative} in shape {dims:?}"
        )));
    }
    let shape: Vec<usize> = dims.iter().map(|&dim| dim as usize).collect();
    ArrayD::from_shape_vec(IxDyn(&shape), data.to_vec()).map_err(|error| OcrError::Inference {
        message: format!(
            "ONNX Runtime output shape {dims:?} does not match {} elements",
            data.len()
        ),
        source: Some(Box::new(error)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_to_i64_converts_dims() {
        assert_eq!(shape_to_i64(&[1, 3, 64, 128]), vec![1_i64, 3, 64, 128]);
    }

    #[test]
    fn shape_to_i64_handles_empty_shape() {
        assert_eq!(shape_to_i64(&[]), Vec::<i64>::new());
    }

    #[test]
    fn array_from_output_preserves_shape() {
        let data = [1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let array = array_from_output(&[1, 2, 3], &data).expect("shape matches element count");
        assert_eq!(array.shape(), &[1, 2, 3]);
        assert_eq!(array.iter().copied().collect::<Vec<_>>(), data);
    }

    #[test]
    fn array_from_output_errors_on_length_mismatch() {
        let error = array_from_output(&[2, 2], &[1.0_f32, 2.0, 3.0]).expect_err("length mismatch must fail");
        assert!(matches!(error, OcrError::Inference { .. }));
    }

    #[test]
    fn array_from_output_errors_on_negative_dimension() {
        let error = array_from_output(&[-1, 2], &[1.0_f32, 2.0]).expect_err("negative dimension must fail");
        assert!(matches!(error, OcrError::Inference { .. }));
    }

    #[test]
    fn input_buffer_preserves_row_major_order_for_standard_layout() {
        let expected: Vec<f32> = (0..8).map(|value| value as f32).collect();
        let input = ArrayD::from_shape_vec(IxDyn(&[1, 2, 2, 2]), expected.clone()).expect("build the tensor");
        let manual: Vec<f32> = input.iter().copied().collect();

        let (shape, data) = input_buffer(input);

        assert_eq!(shape, vec![1_i64, 2, 2, 2]);
        assert_eq!(data, manual);
        assert_eq!(data, expected);
    }

    #[test]
    fn input_buffer_matches_iter_order_for_non_standard_layout() {
        let base = ArrayD::from_shape_vec(IxDyn(&[2, 3]), (0..6).map(|value| value as f32).collect())
            .expect("build the tensor");
        let transposed = base.t().into_owned();
        assert!(
            !transposed.is_standard_layout(),
            "transpose must be non-standard layout"
        );
        let manual: Vec<f32> = transposed.iter().copied().collect();

        let (shape, data) = input_buffer(transposed);

        assert_eq!(shape, vec![3_i64, 2]);
        assert_eq!(data, manual);
    }

    #[test]
    fn input_buffer_slices_standard_layout_with_nonzero_offset() {
        use ndarray::{Axis, Slice};
        let mut base = ArrayD::from_shape_vec(IxDyn(&[3, 2]), (0..6).map(|value| value as f32).collect())
            .expect("build the tensor");
        base.slice_axis_inplace(Axis(0), Slice::from(1..));
        assert!(base.is_standard_layout(), "the sliced array must stay standard layout");
        let manual: Vec<f32> = base.iter().copied().collect();

        let (shape, data) = input_buffer(base);

        assert_eq!(shape, vec![2_i64, 2]);
        assert_eq!(
            data, manual,
            "must return the logical elements, not the full backing buffer"
        );
    }
}
