//! ONNX Runtime implementation of the inference seam — the default engine on
//! native builds.
//!
//! [`OrtBackend`] folds the session-build recipe shared across xberg's ORT
//! consumers into one place: `GraphOptimizationLevel::All`, an intra-op thread
//! budget from the concurrency config, a single inter-op thread, and the
//! execution provider selected by [`crate::ort_discovery::apply_execution_providers`],
//! with a CPU-only retry when the platform EP fails to build — but only for an
//! auto-detected provider; an explicitly-requested provider that fails to build
//! surfaces its error instead, since `apply_execution_providers` deliberately
//! hard-errors in that case. [`OrtSession`] wraps the resulting
//! `ort::session::Session` and converts tensors at the boundary.
//!

use std::borrow::Cow;
use std::path::Path;

use ort::session::builder::GraphOptimizationLevel;
use ort::session::{Session, SessionInputValue};
use ort::value::{Tensor, TensorElementType, Value};

use crate::core::config::acceleration::AccelerationConfig;

use super::backend::{InferenceBackend, InferenceError, InferenceSession};
use super::tensor::InferenceTensor;

/// The ONNX Runtime inference backend.
///
/// Zero-sized; construct with [`OrtBackend::new`].
pub struct OrtBackend;

impl OrtBackend {
    /// Create a new ONNX Runtime backend.
    pub fn new() -> Self {
        Self
    }

    /// Build a session from `source`, applying the standard xberg configuration.
    /// When `with_eps` is false the platform execution providers are skipped
    /// (CPU-only), used for the fallback retry.
    fn commit(
        source: ModelSource<'_>,
        accel: Option<&AccelerationConfig>,
        thread_budget: usize,
        with_eps: bool,
    ) -> Result<Session, InferenceError> {
        (|| -> Result<Session, ort::Error> {
            let mut builder = Session::builder(crate::ort_discovery::ort_environment()?)?
                .with_optimization_level(GraphOptimizationLevel::All)
                .map_err(|e| ort::Error::new(e.message()))?
                .with_intra_threads(thread_budget)
                .map_err(|e| ort::Error::new(e.message()))?
                .with_inter_threads(1)
                .map_err(|e| ort::Error::new(e.message()))?;
            if with_eps {
                builder = crate::ort_discovery::apply_execution_providers(builder, accel)?;
            }
            match source {
                ModelSource::File(path) => builder.commit_from_file(path),
                ModelSource::Memory(bytes) => builder.commit_from_memory(bytes),
            }
        })()
        .map_err(|e| InferenceError::Load(e.to_string()))
    }

    /// Build a session from `source` with the EP-then-CPU-fallback retry, and wrap
    /// it behind the neutral session trait. Shared by [`load`](InferenceBackend::load)
    /// and [`load_from_memory`](InferenceBackend::load_from_memory).
    fn build_session(
        source: ModelSource<'_>,
        accel: Option<&AccelerationConfig>,
        thread_budget: usize,
    ) -> Result<Box<dyn InferenceSession>, InferenceError> {
        crate::ort_discovery::ensure_ort_available();
        let thread_budget = thread_budget.max(1);

        let session = match Self::commit(source, accel, thread_budget, true) {
            Ok(session) => session,
            Err(first_err) => {
                let explicit_provider_request = crate::ort_discovery::is_explicit_provider_request(accel);
                if !should_retry_on_cpu(explicit_provider_request) {
                    // The caller explicitly asked for this provider (e.g. `cuda`, `tensorrt`);
                    // `apply_execution_providers` deliberately hard-errors in that case rather
                    // than falling back, so that error must reach the caller unchanged instead
                    // of being swallowed into a silent CPU session.
                    return Err(first_err);
                }
                tracing::warn!("OrtBackend: platform EP build failed ({first_err}), retrying CPU-only");
                Self::commit(source, accel, thread_budget, false)?
            }
        };

        let input_names = session.inputs().iter().map(|i| i.name().to_string()).collect();
        Ok(Box::new(OrtSession { session, input_names }))
    }
}

/// Whether a session build that failed with the platform execution provider may retry
/// CPU-only. Retrying is the desired best-effort behavior for an auto-detected provider that
/// failed to build, but must NOT happen when the provider was explicitly requested: in that
/// case `ort_discovery::apply_execution_providers` deliberately hard-errors, and retrying would
/// silently swallow that error into a CPU session the caller never asked for.
///
/// Deciding this on `explicit_provider_request` alone (never on the error's message) keeps the
/// decision independent of `ort`'s error text, which is not a stable contract to match on.
fn should_retry_on_cpu(explicit_provider_request: bool) -> bool {
    !explicit_provider_request
}

/// Where an ORT session's graph is read from — a cached file or an in-memory buffer.
#[derive(Clone, Copy)]
enum ModelSource<'a> {
    File(&'a Path),
    Memory(&'a [u8]),
}

impl Default for OrtBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl InferenceBackend for OrtBackend {
    fn load(
        &self,
        model_path: &Path,
        accel: Option<&AccelerationConfig>,
    ) -> Result<Box<dyn InferenceSession>, InferenceError> {
        Self::build_session(
            ModelSource::File(model_path),
            accel,
            crate::core::config::concurrency::resolve_thread_budget(None),
        )
    }

    fn load_with_thread_budget(
        &self,
        model_path: &Path,
        accel: Option<&AccelerationConfig>,
        thread_budget: usize,
    ) -> Result<Box<dyn InferenceSession>, InferenceError> {
        Self::build_session(ModelSource::File(model_path), accel, thread_budget)
    }

    fn load_from_memory(
        &self,
        model_bytes: &[u8],
        accel: Option<&AccelerationConfig>,
    ) -> Result<Box<dyn InferenceSession>, InferenceError> {
        Self::build_session(
            ModelSource::Memory(model_bytes),
            accel,
            crate::core::config::concurrency::resolve_thread_budget(None),
        )
    }
}

/// An ONNX Runtime session behind the neutral [`InferenceSession`] trait.
pub struct OrtSession {
    session: Session,
    input_names: Vec<String>,
}

impl InferenceSession for OrtSession {
    fn run(&self, inputs: Vec<(String, InferenceTensor)>) -> Result<Vec<(String, InferenceTensor)>, InferenceError> {
        let ort_inputs: Vec<(Cow<'static, str>, SessionInputValue<'static>)> = inputs
            .into_iter()
            .map(|(name, tensor)| Ok((Cow::Owned(name), tensor_to_input(tensor)?)))
            .collect::<Result<_, InferenceError>>()?;

        // SAFETY: `ort::session::Session::run` takes `&mut self`, but ONNX
        #[allow(unsafe_code)]
        let outputs = unsafe {
            let session_ptr = &self.session as *const Session as *mut Session;
            (*session_ptr).run(ort_inputs)
        }
        .map_err(|e| InferenceError::Run(e.to_string()))?;

        let mut result = Vec::with_capacity(outputs.len());
        for (name, value) in outputs.iter() {
            result.push((name.to_string(), value_to_tensor(&value)?));
        }
        Ok(result)
    }

    fn input_names(&self) -> &[String] {
        &self.input_names
    }
}

/// Convert a neutral input tensor into an owned ORT session input value.
fn tensor_to_input(tensor: InferenceTensor) -> Result<SessionInputValue<'static>, InferenceError> {
    let tensor_err = |e: ort::Error| InferenceError::Tensor(e.to_string());
    let value: SessionInputValue<'static> = match tensor {
        InferenceTensor::F32(array) => Tensor::from_array(array).map_err(tensor_err)?.into(),
        InferenceTensor::I64(array) => Tensor::from_array(array).map_err(tensor_err)?.into(),
        InferenceTensor::I32(array) => Tensor::from_array(array).map_err(tensor_err)?.into(),
        InferenceTensor::U8(array) => Tensor::from_array(array).map_err(tensor_err)?.into(),
        InferenceTensor::Bool(array) => Tensor::from_array(array).map_err(tensor_err)?.into(),
    };
    Ok(value)
}

/// Convert an ORT output value into a neutral tensor.
fn value_to_tensor(value: &Value) -> Result<InferenceTensor, InferenceError> {
    let element_type = value
        .dtype()
        .tensor_type()
        .ok_or_else(|| InferenceError::Tensor("output value is not a tensor".to_string()))?;

    fn extract_err(kind: &str) -> impl Fn(ort::Error) -> InferenceError + '_ {
        move |e| InferenceError::Tensor(format!("extracting {kind} output: {e}"))
    }

    let tensor = match element_type {
        TensorElementType::Float32 => {
            InferenceTensor::F32(value.try_extract_array::<f32>().map_err(extract_err("f32"))?.to_owned())
        }
        TensorElementType::Int64 => {
            InferenceTensor::I64(value.try_extract_array::<i64>().map_err(extract_err("i64"))?.to_owned())
        }
        TensorElementType::Int32 => {
            InferenceTensor::I32(value.try_extract_array::<i32>().map_err(extract_err("i32"))?.to_owned())
        }
        TensorElementType::Uint8 => {
            InferenceTensor::U8(value.try_extract_array::<u8>().map_err(extract_err("u8"))?.to_owned())
        }
        TensorElementType::Bool => InferenceTensor::Bool(
            value
                .try_extract_array::<bool>()
                .map_err(extract_err("bool"))?
                .to_owned(),
        ),
        other => {
            return Err(InferenceError::Tensor(format!(
                "unsupported output element type {other:?}"
            )));
        }
    };
    Ok(tensor)
}

#[cfg(test)]
mod tests {
    use ndarray::ArrayD;

    use super::*;

    // These exercise `should_retry_on_cpu`, the pure decision `build_session` makes when the
    // platform EP build fails — not `build_session` itself, which needs a real ORT session and
    // model to construct. The four `decide_gpu_ep_outcome` tests in `ort_discovery` call that
    // pure helper directly too, and passed unmodified with the swallow-bug fully present,
    // because nothing in `build_session` ever consulted them: `build_session` retried
    // CPU-only on ANY `Self::commit(..., true)` error, explicit or not. `should_retry_on_cpu`
    // is the decision that closes that gap; these tests pin it directly.

    #[test]
    fn should_retry_on_cpu_allows_retry_for_auto_detected_provider() {
        assert!(should_retry_on_cpu(false));
    }

    #[test]
    fn should_retry_on_cpu_forbids_retry_for_explicit_provider_request() {
        // Fails against the unfixed code: `should_retry_on_cpu` does not exist prior to this
        // fix, and `build_session` retried CPU-only unconditionally — including when the
        // caller explicitly requested a GPU provider that `apply_execution_providers`
        // deliberately hard-errored on (commit b0444fffdd). That swallowed the explicit
        // request's error into a silent CPU fallback behind only a `tracing::warn!`, which is
        // exactly the HIGH-severity regression this fix closes.
        assert!(!should_retry_on_cpu(true));
    }

    #[test]
    fn f32_input_conversion_preserves_shape_and_data() {
        let array = ArrayD::from_shape_vec(vec![1, 2, 2], vec![1.0f32, 2.0, 3.0, 4.0]).unwrap();
        let input = tensor_to_input(InferenceTensor::F32(array.clone())).unwrap();
        let (shape, data) = input.try_extract_tensor::<f32>().unwrap();
        assert_eq!(shape.to_vec(), vec![1_i64, 2, 2]);
        assert_eq!(data, array.as_slice().unwrap());
    }

    #[test]
    fn value_to_tensor_roundtrips_f32() {
        let array = ArrayD::from_shape_vec(vec![2, 2], vec![5.0f32, 6.0, 7.0, 8.0]).unwrap();
        let value = Tensor::from_array(array.clone()).unwrap().into_dyn();
        assert_eq!(value_to_tensor(&value).unwrap().as_f32().unwrap(), &array);
    }

    #[test]
    fn value_to_tensor_preserves_i64_dtype() {
        let array = ArrayD::from_shape_vec(vec![3], vec![10i64, 20, 30]).unwrap();
        let value = Tensor::from_array(array.clone()).unwrap().into_dyn();
        match value_to_tensor(&value).unwrap() {
            InferenceTensor::I64(extracted) => assert_eq!(extracted, array),
            other => panic!("expected I64 tensor, got {other:?}"),
        }
    }
}
