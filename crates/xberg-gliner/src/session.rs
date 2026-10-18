use std::collections::HashSet;
use std::path::Path;

use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;

use crate::{GlinerError, Result, RuntimeConfig};

/// GLiNER tensor input names expected by span-mode ONNX exports.
pub const INPUT_NAMES: [&str; 6] = [
    "input_ids",
    "attention_mask",
    "words_mask",
    "text_lengths",
    "span_idx",
    "span_mask",
];

/// GLiNER tensor output names expected by span-mode ONNX exports.
pub const OUTPUT_NAMES: [&str; 1] = ["logits"];

pub(crate) const TENSOR_INPUT_IDS: &str = "input_ids";
pub(crate) const TENSOR_ATTENTION_MASK: &str = "attention_mask";
pub(crate) const TENSOR_WORD_MASK: &str = "words_mask";
pub(crate) const TENSOR_TEXT_LENGTHS: &str = "text_lengths";
pub(crate) const TENSOR_SPAN_IDX: &str = "span_idx";
pub(crate) const TENSOR_SPAN_MASK: &str = "span_mask";
pub(crate) const TENSOR_LOGITS: &str = "logits";

static ENVIRONMENT: std::sync::OnceLock<ort::environment::Environment> = std::sync::OnceLock::new();
static ENVIRONMENT_INIT: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Make this crate build its sessions under `environment` instead of creating its own.
///
/// ONNX Runtime permits one environment per process, so a host that also runs other `ort`
/// consumers must hand them all the same one. Returns `false` when this crate already holds an
/// environment (its own, or an earlier adoption), in which case `environment` is not used.
/// Call before the first model load.
pub fn adopt_environment(environment: &ort::environment::Environment) -> bool {
    ENVIRONMENT.set(environment.clone()).is_ok()
}

/// ONNX Runtime must keep one environment alive for the life of every session (ort rc.14 takes
/// it as an argument). ~keep
fn environment() -> Result<&'static ort::environment::Environment> {
    if let Some(environment) = ENVIRONMENT.get() {
        return Ok(environment);
    }
    // A second concurrent `build()` fails ("only one environment is allowed per process")
    // instead of duplicating work, so the first creation is serialized. ~keep
    let _init = ENVIRONMENT_INIT.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(environment) = ENVIRONMENT.get() {
        return Ok(environment);
    }
    let environment = ort::init().build()?;
    Ok(ENVIRONMENT.get_or_init(|| environment))
}

pub(crate) fn build_session<P: AsRef<Path>>(model_path: P, runtime: &RuntimeConfig) -> Result<Session> {
    let session = Session::builder(environment()?)?
        .with_optimization_level(GraphOptimizationLevel::All)
        .map_err(|error| GlinerError::Ort(ort::Error::from(error)))?
        .with_intra_threads(runtime.intra_threads)
        .map_err(|error| GlinerError::Ort(ort::Error::from(error)))?
        .with_inter_threads(1)
        .map_err(|error| GlinerError::Ort(ort::Error::from(error)))?
        .commit_from_file(model_path)?;
    Ok(session)
}

pub(crate) fn validate_session_schema(session: &Session) -> Result<()> {
    let inputs = session
        .inputs()
        .iter()
        .map(|input| input.name().to_string())
        .collect::<Vec<_>>();
    validate_schema_names("input", &INPUT_NAMES, &inputs)?;

    let outputs = session
        .outputs()
        .iter()
        .map(|output| output.name().to_string())
        .collect::<Vec<_>>();
    validate_schema_names("output", &OUTPUT_NAMES, &outputs)
}

pub(crate) fn validate_schema_names(kind: &'static str, expected: &[&'static str], actual: &[String]) -> Result<()> {
    let expected = expected.iter().copied().collect::<HashSet<_>>();
    let actual = actual.iter().map(String::as_str).collect::<HashSet<_>>();
    if expected == actual {
        return Ok(());
    }
    let mut actual = actual.into_iter().map(str::to_string).collect::<Vec<_>>();
    actual.sort_unstable();
    Err(GlinerError::UnexpectedModelSchema {
        kind,
        expected: expected.into_iter().collect(),
        actual,
    })
}
