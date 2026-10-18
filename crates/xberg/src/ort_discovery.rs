//! ONNX Runtime library auto-discovery and execution provider configuration.
//!
//! Scans common installation paths and sets `ORT_DYLIB_PATH` so the `ort` crate
//! can find `libonnxruntime` via `dlopen`. Called once at init time.
//!
//! Also provides `apply_execution_providers` for configuring GPU acceleration
//! on ORT session builders across all subsystems (layout, embeddings, OCR, etc.).

#[cfg(not(feature = "ort-bundled"))]
use std::sync::Once;

#[cfg(not(feature = "ort-bundled"))]
static ORT_INIT: Once = Once::new();

const XBERG_ORT_EP_ENV_VAR: &str = "XBERG_ORT_EP";

pub(crate) fn parse_execution_provider_override(
    value: &str,
) -> Option<crate::core::config::acceleration::ExecutionProviderType> {
    use crate::core::config::acceleration::ExecutionProviderType;

    match value.trim().to_ascii_lowercase().as_str() {
        "cpu" => Some(ExecutionProviderType::Cpu),
        "coreml" => Some(ExecutionProviderType::CoreMl),
        "cuda" => Some(ExecutionProviderType::Cuda),
        "tensorrt" => Some(ExecutionProviderType::TensorRt),
        "auto" => Some(ExecutionProviderType::Auto),
        _ => None,
    }
}

// Deliberately ungated (unlike `apply_execution_providers`, which has `ort::` types in its
// signature): every ORT-capable subsystem calls this independently under its own feature
// gate, and those gates are numerous and not owned by this module. A feature combination
// that compiles no ORT-capable subsystem at all (e.g. `paddle-ocr-tract` alone, with no
// `layout-detection`/`embeddings`/`auto-rotate`/etc.) legitimately calls neither function;
// that is not a bug in either the caller or here. ~keep
#[allow(dead_code)]
pub(crate) fn execution_provider_override() -> Option<crate::core::config::acceleration::ExecutionProviderType> {
    std::env::var(XBERG_ORT_EP_ENV_VAR)
        .ok()
        .and_then(|value| parse_execution_provider_override(&value))
}

/// Ensure ONNX Runtime is discoverable. Safe to call multiple times (no-op after first).
///
/// When the `ort-bundled` feature is enabled the ORT binaries are embedded via the
/// official Microsoft release and no system library search is needed.
#[allow(dead_code)]
pub(crate) fn ensure_ort_available() {
    #[cfg(feature = "ort-bundled")]
    {
        tracing::debug!("ONNX Runtime is bundled; skipping system library discovery");
    }

    #[cfg(not(feature = "ort-bundled"))]
    ORT_INIT.call_once(|| {
        if let Err(msg) = try_discover_ort() {
            tracing::warn!("ONNX Runtime not found: {msg}");
        }
    });
}

#[cfg(not(feature = "ort-bundled"))]
fn try_discover_ort() -> Result<(), &'static str> {
    if let Ok(path) = std::env::var("ORT_DYLIB_PATH")
        && std::path::Path::new(&path).exists()
    {
        return Ok(());
    }

    let candidates: &[&str] = platform_candidates();

    for path in candidates {
        if std::path::Path::new(path).exists() {
            #[allow(unsafe_code)]
            unsafe {
                std::env::set_var("ORT_DYLIB_PATH", path);
            }
            tracing::debug!("Auto-discovered ONNX Runtime at {path}");
            return Ok(());
        }
    }

    Err("ONNX Runtime library not found in common installation paths")
}

#[cfg(all(not(feature = "ort-bundled"), target_os = "macos"))]
fn platform_candidates() -> &'static [&'static str] {
    &[
        "/opt/homebrew/lib/libonnxruntime.dylib",
        "/usr/local/lib/libonnxruntime.dylib",
    ]
}

#[cfg(all(not(feature = "ort-bundled"), target_os = "linux"))]
fn platform_candidates() -> &'static [&'static str] {
    &[
        "/usr/lib/libonnxruntime.so",
        "/usr/local/lib/libonnxruntime.so",
        "/usr/lib/x86_64-linux-gnu/libonnxruntime.so",
        "/usr/lib/aarch64-linux-gnu/libonnxruntime.so",
    ]
}

#[cfg(all(not(feature = "ort-bundled"), target_os = "windows"))]
fn platform_candidates() -> &'static [&'static str] {
    &[
        "C:\\Program Files\\onnxruntime\\bin\\onnxruntime.dll",
        "C:\\Windows\\System32\\onnxruntime.dll",
    ]
}

#[cfg(all(
    not(feature = "ort-bundled"),
    not(any(target_os = "macos", target_os = "linux", target_os = "windows"))
))]
fn platform_candidates() -> &'static [&'static str] {
    &[]
}

/// Outcome of deciding how to register a GPU execution provider, given whether it was
/// explicitly requested and whether ONNX Runtime reports (compile-time) support for it.
///
/// `is_available()` on an [`ort::ep::ExecutionProvider`] only reports that ONNX Runtime was
/// *compiled with* the provider — it does not mean the provider can actually load at session
/// creation (e.g. missing CUDA runtime libraries). `RegisterStrict` accounts for that gap: the
/// caller asked for this provider by name, so a registration failure must be reported, not
/// swallowed by `ort`'s default `fail_silently` policy.
// See the `#[allow(dead_code)]` rationale on `execution_provider_override` above: under the
// crate's default features (`tokio-runtime`, `simd-utf8`) none of the feature gates on
// `apply_execution_providers` are enabled, so this type has no non-test caller in that build.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GpuEpOutcome {
    /// Register the provider and require registration to succeed; a load failure at session
    /// creation must surface as an error, since the caller explicitly asked for this provider.
    RegisterStrict,
    /// Register the provider on a best-effort basis; a load failure falls back to CPU silently
    /// (`ort`'s default policy). Only reachable for the unspecified/`auto` path.
    RegisterBestEffort,
    /// Do not attempt registration: the caller explicitly requested a provider ONNX Runtime was
    /// not compiled to support.
    ErrorUnavailable,
    /// Do not attempt registration: the provider is unavailable and nothing explicit was
    /// requested, so falling back to CPU is the documented, silent `auto` behavior.
    SkipToCpu,
}

// Pure and hardware-independent by design: `is_available` is a plain `bool` rather than a call
// into `ort`'s FFI, so tests can exercise every (explicit, is_available) combination without
// depending on what accelerators happen to be present on the machine running the test. Kept
// ungated (see `execution_provider_override` above) so it — and its tests — compile and run
// even in feature combinations that build no ORT-capable subsystem. ~keep
#[allow(dead_code)]
pub(crate) fn decide_gpu_ep_outcome(explicit: bool, is_available: bool) -> GpuEpOutcome {
    match (explicit, is_available) {
        (true, true) => GpuEpOutcome::RegisterStrict,
        (true, false) => GpuEpOutcome::ErrorUnavailable,
        (false, true) => GpuEpOutcome::RegisterBestEffort,
        (false, false) => GpuEpOutcome::SkipToCpu,
    }
}

/// Build the error message for a GPU execution provider that was explicitly requested but is
/// not available in the loaded ONNX Runtime. Names both the provider and a concrete suggestion
/// so the caller can act on it immediately, per the error-context convention (operation,
/// input, root cause, suggestion).
#[allow(dead_code)] // see `decide_gpu_ep_outcome` above
pub(crate) fn unavailable_gpu_ep_error_message(provider_name: &str, suggestion: &str) -> String {
    format!(
        "{provider_name} execution provider requested but not available in the loaded ONNX \
         Runtime. {suggestion}"
    )
}

/// Resolve the execution provider a session build will attempt for `accel`: the `XBERG_ORT_EP`
/// env override first, then `accel`'s configured provider, then
/// [`crate::core::config::acceleration::ExecutionProviderType::Auto`].
/// Mirrors the resolution `apply_execution_providers` performs internally; factored out so
/// callers that need this decision *before* invoking `apply_execution_providers` — e.g. deciding
/// whether a failed session build may retry CPU-only, see `OrtBackend::build_session` in
/// `inference::ort_backend` — do not re-derive it independently and risk it drifting out of sync.
// See the `#[allow(dead_code)]` rationale on `execution_provider_override` above.
#[allow(dead_code)]
pub(crate) fn resolve_execution_provider(
    accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
) -> crate::core::config::acceleration::ExecutionProviderType {
    use crate::core::config::acceleration::ExecutionProviderType;

    execution_provider_override()
        .unwrap_or_else(|| accel.map(|a| a.provider.clone()).unwrap_or(ExecutionProviderType::Auto))
}

/// Whether `accel` (plus any `XBERG_ORT_EP` override) resolves to an explicitly-requested
/// execution provider, as opposed to
/// [`crate::core::config::acceleration::ExecutionProviderType::Auto`]. `apply_execution_providers`
/// hard-errors on an explicit request it cannot satisfy rather than falling back to CPU — callers
/// that build a session must not swallow that error into a silent CPU retry.
#[allow(dead_code)]
pub(crate) fn is_explicit_provider_request(
    accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
) -> bool {
    resolve_execution_provider(accel) != crate::core::config::acceleration::ExecutionProviderType::Auto
}

/// The process-wide ONNX Runtime environment every session builder in this crate shares; ort
/// rc.14 takes it as an argument and ONNX Runtime requires it to outlive every session.
/// The `xberg-paddle-ocr` and `xberg-gliner` session builders are handed this same
/// environment when it is created, so call this before they load a model. ~keep
#[cfg(any(
    feature = "layout-detection",
    feature = "embeddings",
    feature = "paddle-ocr-ort",
    feature = "auto-rotate",
    feature = "reranker",
    feature = "onnx-runtime",
    feature = "transcription",
    feature = "ner-onnx"
))]
pub(crate) fn ort_environment() -> Result<&'static ort::environment::Environment, ort::Error> {
    static ENVIRONMENT: std::sync::OnceLock<ort::environment::Environment> = std::sync::OnceLock::new();
    static INIT: std::sync::Mutex<()> = std::sync::Mutex::new(());
    if let Some(environment) = ENVIRONMENT.get() {
        return Ok(environment);
    }
    // ONNX Runtime allows one environment per process, so a second concurrent `build()` fails
    // rather than duplicating work: serialize the first creation. ~keep
    let _init = INIT.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(environment) = ENVIRONMENT.get() {
        return Ok(environment);
    }
    let environment = ort::init().build()?;
    let environment = ENVIRONMENT.get_or_init(|| environment);
    #[cfg(feature = "paddle-ocr-ort")]
    if !xberg_paddle_ocr::adopt_ort_environment(environment) {
        tracing::warn!("PaddleOCR already holds its own ONNX Runtime environment");
    }
    #[cfg(feature = "ner-onnx")]
    if !xberg_gliner::adopt_ort_environment(environment) {
        tracing::warn!("GLiNER already holds its own ONNX Runtime environment");
    }
    Ok(environment)
}

/// Apply execution providers to an ORT session builder based on [`AccelerationConfig`].
///
/// Shared by all ORT consumers (layout detection, embeddings, PaddleOCR, doc orientation).
///
/// When a GPU provider is **explicitly requested** (e.g. `cuda`, `tensorrt`) this function
/// returns an error with an actionable message if either (a) ONNX Runtime was not compiled
/// with that provider, or (b) the provider fails to load at session-creation time (e.g. a
/// missing CUDA runtime library) — `is_available()` alone cannot detect the second case, so
/// explicit registrations are additionally marked `error_on_failure()`. When `auto` is used,
/// unavailable or unloadable GPU providers fall back to CPU silently, with an info-level log.
///
/// [`AccelerationConfig`]: crate::core::config::acceleration::AccelerationConfig
#[cfg(any(
    feature = "layout-detection",
    feature = "embeddings",
    feature = "paddle-ocr-ort",
    feature = "auto-rotate",
    feature = "reranker",
    feature = "onnx-runtime",
    feature = "transcription"
))]
pub(crate) fn apply_execution_providers(
    builder: ort::session::builder::SessionBuilder,
    accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
) -> Result<ort::session::builder::SessionBuilder, ort::Error> {
    use crate::core::config::acceleration::ExecutionProviderType;

    let provider = resolve_execution_provider(accel);
    // Only read by the CUDA/TensorRT EP arms, which are cfg-gated behind their respective
    // Cargo features; unused without at least one of them enabled.
    #[cfg_attr(not(any(feature = "cuda", feature = "tensorrt")), allow(unused_variables))]
    let device_id = accel.map(|a| a.device_id).unwrap_or(0);

    let builder = match provider {
        ExecutionProviderType::Cpu => {
            tracing::debug!("ORT session: CPU execution provider (explicit)");
            builder
        }
        #[cfg(target_os = "macos")]
        ExecutionProviderType::CoreMl => register_coreml_strict(builder)?,
        #[cfg(not(target_os = "macos"))]
        ExecutionProviderType::CoreMl => {
            return Err(ort::Error::new(
                "CoreML execution provider requested but this build target is not macOS. \
                 CoreML is only available on macOS.",
            ));
        }
        #[cfg(feature = "cuda")]
        ExecutionProviderType::Cuda => register_cuda_strict(builder, device_id)?,
        #[cfg(not(feature = "cuda"))]
        ExecutionProviderType::Cuda => {
            return Err(ort::Error::new(
                "CUDA execution provider requested but this build was compiled without CUDA \
                 support; rebuild with the `cuda` feature.",
            ));
        }
        #[cfg(feature = "tensorrt")]
        ExecutionProviderType::TensorRt => register_tensorrt_strict(builder, device_id)?,
        #[cfg(not(feature = "tensorrt"))]
        ExecutionProviderType::TensorRt => {
            return Err(ort::Error::new(
                "TensorRT execution provider requested but this build was compiled without \
                 TensorRT support; rebuild with the `tensorrt` feature.",
            ));
        }
        ExecutionProviderType::Auto => register_auto_providers(builder)?,
    };

    Ok(builder)
}

/// Build the CoreML execution provider, honouring the `XBERG_COREML_FORMAT` and
/// `XBERG_COREML_UNITS` environment overrides. An unrecognised value is warned about and
/// ignored rather than failing session creation. ~keep
#[cfg(any(
    feature = "layout-detection",
    feature = "embeddings",
    feature = "paddle-ocr-ort",
    feature = "auto-rotate",
    feature = "reranker",
    feature = "onnx-runtime",
    feature = "transcription"
))]
#[cfg(target_os = "macos")]
fn build_coreml_ep() -> ort::ep::CoreML {
    use ort::ep::coreml::{ComputeUnits, ModelFormat};
    let mut ep = ort::ep::CoreML::default();
    if let Ok(fmt) = std::env::var("XBERG_COREML_FORMAT") {
        match fmt.trim().to_ascii_lowercase().as_str() {
            "mlprogram" => ep = ep.with_model_format(ModelFormat::MLProgram),
            "neuralnetwork" | "nn" => ep = ep.with_model_format(ModelFormat::NeuralNetwork),
            other => tracing::warn!(value = other, "ignoring unknown XBERG_COREML_FORMAT"),
        }
    }
    if let Ok(units) = std::env::var("XBERG_COREML_UNITS") {
        match units.trim().to_ascii_lowercase().as_str() {
            "all" => ep = ep.with_compute_units(ComputeUnits::All),
            "cpu_and_ne" => ep = ep.with_compute_units(ComputeUnits::CPUAndNeuralEngine),
            "cpu_and_gpu" => ep = ep.with_compute_units(ComputeUnits::CPUAndGPU),
            "cpu_only" => ep = ep.with_compute_units(ComputeUnits::CPUOnly),
            other => tracing::warn!(value = other, "ignoring unknown XBERG_COREML_UNITS"),
        }
    }
    ep
}

/// Register CoreML as an explicitly requested provider: unavailable or unloadable is an error,
/// never a silent CPU fallback. ~keep
#[cfg(any(
    feature = "layout-detection",
    feature = "embeddings",
    feature = "paddle-ocr-ort",
    feature = "auto-rotate",
    feature = "reranker",
    feature = "onnx-runtime",
    feature = "transcription"
))]
#[cfg(target_os = "macos")]
fn register_coreml_strict(
    builder: ort::session::builder::SessionBuilder,
) -> Result<ort::session::builder::SessionBuilder, ort::Error> {
    use ort::ep::ExecutionProvider;

    let ep = build_coreml_ep();
    match decide_gpu_ep_outcome(
        true,
        <ort::ep::CoreML as ExecutionProvider>::is_available().unwrap_or(false),
    ) {
        GpuEpOutcome::RegisterStrict => {
            tracing::info!("ORT session: CoreML execution provider available, using GPU");
            builder
                .with_execution_providers([ep.build().error_on_failure()])
                .map_err(|e| ort::Error::new(e.message()))
        }
        _ => Err(ort::Error::new(unavailable_gpu_ep_error_message(
            "CoreML",
            "Set ORT_DYLIB_PATH to an ONNX Runtime build that includes CoreML support.",
        ))),
    }
}

/// Register CUDA as an explicitly requested provider: unavailable or unloadable is an error,
/// never a silent CPU fallback. ~keep
#[cfg(any(
    feature = "layout-detection",
    feature = "embeddings",
    feature = "paddle-ocr-ort",
    feature = "auto-rotate",
    feature = "reranker",
    feature = "onnx-runtime",
    feature = "transcription"
))]
#[cfg(feature = "cuda")]
fn register_cuda_strict(
    builder: ort::session::builder::SessionBuilder,
    device_id: u32,
) -> Result<ort::session::builder::SessionBuilder, ort::Error> {
    use ort::ep::ExecutionProvider;

    let ep = ort::ep::CUDA::default().with_device_id(device_id as i32);
    match decide_gpu_ep_outcome(
        true,
        <ort::ep::CUDA as ExecutionProvider>::is_available().unwrap_or(false),
    ) {
        GpuEpOutcome::RegisterStrict => {
            tracing::info!(device_id, "ORT session: CUDA execution provider available, using GPU");
            builder
                .with_execution_providers([ep.build().error_on_failure()])
                .map_err(|e| ort::Error::new(e.message()))
        }
        _ => Err(ort::Error::new(unavailable_gpu_ep_error_message(
            "CUDA",
            "Install a CUDA-enabled ONNX Runtime and set ORT_DYLIB_PATH to point at it \
             (see https://github.com/microsoft/onnxruntime/releases).",
        ))),
    }
}

/// Register TensorRT as an explicitly requested provider: unavailable or unloadable is an error,
/// never a silent CPU fallback. ~keep
#[cfg(any(
    feature = "layout-detection",
    feature = "embeddings",
    feature = "paddle-ocr-ort",
    feature = "auto-rotate",
    feature = "reranker",
    feature = "onnx-runtime",
    feature = "transcription"
))]
#[cfg(feature = "tensorrt")]
fn register_tensorrt_strict(
    builder: ort::session::builder::SessionBuilder,
    device_id: u32,
) -> Result<ort::session::builder::SessionBuilder, ort::Error> {
    use ort::ep::ExecutionProvider;

    let ep = ort::ep::TensorRT::default().with_device_id(device_id as i32);
    match decide_gpu_ep_outcome(
        true,
        <ort::ep::TensorRT as ExecutionProvider>::is_available().unwrap_or(false),
    ) {
        GpuEpOutcome::RegisterStrict => {
            tracing::info!(
                device_id,
                "ORT session: TensorRT execution provider available, using GPU"
            );
            builder
                .with_execution_providers([ep.build().error_on_failure()])
                .map_err(|e| ort::Error::new(e.message()))
        }
        _ => Err(ort::Error::new(unavailable_gpu_ep_error_message(
            "TensorRT",
            "Install a TensorRT-enabled ONNX Runtime and set ORT_DYLIB_PATH to point at it \
             (see https://github.com/microsoft/onnxruntime/releases).",
        ))),
    }
}

/// `auto`: try the platform GPU provider best-effort and fall back to CPU with an info-level log
/// when it is unavailable or fails to load. ~keep
#[cfg(any(
    feature = "layout-detection",
    feature = "embeddings",
    feature = "paddle-ocr-ort",
    feature = "auto-rotate",
    feature = "reranker",
    feature = "onnx-runtime",
    feature = "transcription"
))]
fn register_auto_providers(
    builder: ort::session::builder::SessionBuilder,
) -> Result<ort::session::builder::SessionBuilder, ort::Error> {
    #[cfg(any(target_os = "macos", all(target_os = "linux", feature = "cuda")))]
    use ort::ep::ExecutionProvider;

    #[cfg(target_os = "macos")]
    let builder = {
        let ep = build_coreml_ep();
        match decide_gpu_ep_outcome(
            false,
            <ort::ep::CoreML as ExecutionProvider>::is_available().unwrap_or(false),
        ) {
            GpuEpOutcome::RegisterBestEffort => {
                tracing::info!("ORT session: auto — CoreML available, using GPU");
                builder
                    .with_execution_providers([ep.build()])
                    .map_err(|e| ort::Error::new(e.message()))?
            }
            _ => {
                tracing::info!("ORT session: auto — CoreML not available, using CPU");
                builder
            }
        }
    };
    #[cfg(all(target_os = "linux", feature = "cuda"))]
    let builder = {
        let ep = ort::ep::CUDA::default();
        match decide_gpu_ep_outcome(
            false,
            <ort::ep::CUDA as ExecutionProvider>::is_available().unwrap_or(false),
        ) {
            GpuEpOutcome::RegisterBestEffort => {
                tracing::info!("ORT session: auto — CUDA available, using GPU");
                builder
                    .with_execution_providers([ep.build()])
                    .map_err(|e| ort::Error::new(e.message()))?
            }
            _ => {
                tracing::info!(
                    "ORT session: auto — CUDA not available, using CPU. \
                     For GPU support, set ORT_DYLIB_PATH to a CUDA-enabled ONNX Runtime."
                );
                builder
            }
        }
    };
    #[cfg(all(target_os = "linux", not(feature = "cuda")))]
    let builder = {
        tracing::debug!("ORT session: auto — using CPU. Rebuild with the `cuda` feature for GPU support.");
        builder
    };
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let builder = {
        tracing::debug!("ORT session: auto — no platform GPU EP, using CPU");
        builder
    };

    Ok(builder)
}

#[cfg(test)]
mod tests {
    use super::{
        GpuEpOutcome, decide_gpu_ep_outcome, is_explicit_provider_request, parse_execution_provider_override,
        resolve_execution_provider, unavailable_gpu_ep_error_message,
    };
    use crate::core::config::acceleration::{AccelerationConfig, ExecutionProviderType};

    // These tests exercise `decide_gpu_ep_outcome` with an injected `is_available` bool rather
    // than a real `ort::ep::ExecutionProvider::is_available()` call. Real EP availability
    // varies by machine (GPU box vs CI runner), so asserting against actual hardware would be
    // flaky in one direction or the other; the decision logic itself has no such dependency.

    #[test]
    fn explicit_request_for_unavailable_provider_errors_instead_of_falling_back() {
        // Fails against the unfixed code: before this change, an explicit GPU request that
        // fails `is_available()` returned an error already (that part was correct) but nothing
        // distinguished "explicit + unavailable" from "explicit + available-but-fails-to-load"
        // — the latter fell back to CPU silently because `error_on_failure()` was never called.
        // This test pins the first half of the decision the fix makes explicit and reusable.
        assert_eq!(decide_gpu_ep_outcome(true, false), GpuEpOutcome::ErrorUnavailable);
    }

    #[test]
    fn explicit_request_for_available_provider_is_registered_strictly() {
        // Fails against the unfixed code: `decide_gpu_ep_outcome` does not exist prior to this
        // fix, and the production arms it now drives called `.build()` without
        // `.error_on_failure()`, so an available-but-unloadable provider fell back to CPU
        // silently instead of surfacing the registration failure. `RegisterStrict` is the
        // outcome that carries `error_on_failure()` into the actual EP dispatch (see the CUDA,
        // TensorRT, and CoreML arms of `apply_execution_providers`).
        assert_eq!(decide_gpu_ep_outcome(true, true), GpuEpOutcome::RegisterStrict);
    }

    #[test]
    fn unspecified_provider_falls_back_to_cpu_silently_regardless_of_availability() {
        // Control: the legitimate `auto` path must never error and must never request strict
        // registration, whether or not the provider happens to be available. Fails against the
        // unfixed code because `decide_gpu_ep_outcome` does not exist; once it exists, this
        // guards against a future change accidentally tightening `auto` to `error_on_failure`.
        assert_eq!(decide_gpu_ep_outcome(false, false), GpuEpOutcome::SkipToCpu);
        assert_eq!(decide_gpu_ep_outcome(false, true), GpuEpOutcome::RegisterBestEffort);
        for is_available in [false, true] {
            let outcome = decide_gpu_ep_outcome(false, is_available);
            assert_ne!(outcome, GpuEpOutcome::ErrorUnavailable, "auto must never error");
            assert_ne!(outcome, GpuEpOutcome::RegisterStrict, "auto must never require success");
        }
    }

    #[test]
    fn unavailable_gpu_ep_error_names_the_provider_and_the_suggestion() {
        // Fails against the unfixed code: `unavailable_gpu_ep_error_message` does not exist
        // prior to this fix. It pins the constraint that the error names both the EP that was
        // requested and why it is unavailable, as used by the CUDA, TensorRT, and CoreML arms.
        let message = unavailable_gpu_ep_error_message("CUDA", "Install a CUDA-enabled ONNX Runtime.");
        assert!(
            message.contains("CUDA execution provider requested"),
            "message: {message:?}"
        );
        assert!(
            message.contains("not available in the loaded ONNX Runtime"),
            "message: {message:?}"
        );
        assert!(
            message.contains("Install a CUDA-enabled ONNX Runtime."),
            "message: {message:?}"
        );
    }

    #[test]
    fn unavailable_gpu_ep_error_distinguishes_providers() {
        let cuda = unavailable_gpu_ep_error_message("CUDA", "suggestion");
        let tensorrt = unavailable_gpu_ep_error_message("TensorRT", "suggestion");
        let coreml = unavailable_gpu_ep_error_message("CoreML", "suggestion");
        assert_ne!(cuda, tensorrt);
        assert_ne!(cuda, coreml);
        assert_ne!(tensorrt, coreml);
    }

    #[test]
    fn blank_execution_provider_overrides_are_absent() {
        for value in ["", " ", "\t\r\n"] {
            assert_eq!(parse_execution_provider_override(value), None, "value: {value:?}");
        }
    }

    #[test]
    fn unrecognized_execution_provider_overrides_are_absent() {
        for value in ["invalid", "gpu", "core-ml", "cpu,cuda"] {
            assert_eq!(parse_execution_provider_override(value), None, "value: {value:?}");
        }
    }

    #[test]
    fn explicit_gpu_providers_resolve_to_explicit_request() {
        for provider in [
            ExecutionProviderType::CoreMl,
            ExecutionProviderType::Cuda,
            ExecutionProviderType::TensorRt,
        ] {
            let accel = AccelerationConfig { provider, device_id: 0 };
            assert_eq!(resolve_execution_provider(Some(&accel)), accel.provider);
            assert!(
                is_explicit_provider_request(Some(&accel)),
                "provider {:?} must resolve as explicit",
                accel.provider
            );
        }
    }

    #[test]
    fn auto_provider_and_absent_config_resolve_to_not_explicit() {
        // Fails against the unfixed code: prior to this fix there was no way to distinguish an
        // auto-detected provider from an explicitly-requested one outside of
        // `apply_execution_providers` itself, which is exactly why `OrtBackend::build_session`
        // retried CPU-only on ANY session-build error, including an explicit request that
        // `apply_execution_providers` deliberately hard-errored on (commit b0444fffdd).
        let auto = AccelerationConfig {
            provider: ExecutionProviderType::Auto,
            device_id: 0,
        };
        assert_eq!(resolve_execution_provider(Some(&auto)), ExecutionProviderType::Auto);
        assert!(!is_explicit_provider_request(Some(&auto)));
        assert_eq!(resolve_execution_provider(None), ExecutionProviderType::Auto);
        assert!(!is_explicit_provider_request(None));
    }

    #[test]
    fn recognized_execution_provider_overrides_are_parsed() {
        for (value, expected) in [
            ("cpu", ExecutionProviderType::Cpu),
            (" COREML ", ExecutionProviderType::CoreMl),
            ("cuda", ExecutionProviderType::Cuda),
            ("TensorRT", ExecutionProviderType::TensorRt),
            ("auto", ExecutionProviderType::Auto),
        ] {
            assert_eq!(
                parse_execution_provider_override(value),
                Some(expected),
                "value: {value:?}"
            );
        }
    }
}

#[cfg(all(test, any(feature = "paddle-ocr-ort", feature = "ner-onnx")))]
mod shared_environment_tests {
    fn field(number: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![(number << 3) | 2, payload.len() as u8];
        out.extend_from_slice(payload);
        out
    }

    fn varint_field(number: u8, value: u8) -> Vec<u8> {
        vec![number << 3, value]
    }

    fn value_info(name: &str) -> Vec<u8> {
        let dim = field(1, &varint_field(1, 1));
        let tensor = [varint_field(1, 1), field(2, &dim)].concat();
        let type_proto = field(1, &tensor);
        [field(1, name.as_bytes()), field(2, &type_proto)].concat()
    }

    /// A one-node `Identity` ONNX model: loads under any ONNX Runtime without model assets.
    fn identity_model() -> Vec<u8> {
        let node = [field(1, b"x"), field(2, b"y"), field(4, b"Identity")].concat();
        let graph = [
            field(1, &node),
            field(2, b"g"),
            field(11, &value_info("x")),
            field(12, &value_info("y")),
        ]
        .concat();
        [varint_field(1, 8), field(7, &graph), field(8, &varint_field(2, 13))].concat()
    }

    #[test]
    fn repeated_calls_return_the_same_environment() {
        let first = super::ort_environment().expect("environment");
        let second = super::ort_environment().expect("second call must not rebuild");
        assert!(std::ptr::eq(first, second));
    }

    #[cfg(feature = "paddle-ocr-ort")]
    #[test]
    fn paddle_loads_under_the_shared_environment() {
        use xberg_paddle_ocr::base_net::BaseNet;
        super::ort_environment().expect("environment");
        let mut net = xberg_paddle_ocr::db_net::DbNet::new();
        // A PaddleOCR-created environment would fail here: ort allows one per process.
        net.init_model_from_memory_with_ort_builder(&identity_model(), 1, None)
            .expect("paddle must reuse the shared environment");
        assert!(!xberg_paddle_ocr::adopt_ort_environment(
            super::ort_environment().expect("environment")
        ));
    }

    #[cfg(feature = "ner-onnx")]
    #[test]
    fn gliner_loads_under_the_shared_environment() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tokenizer = dir.path().join("tokenizer.json");
        std::fs::write(
            &tokenizer,
            r#"{"version":"1.0","model":{"type":"WordLevel","vocab":{"[UNK]":0},"unk_token":"[UNK]"}}"#,
        )
        .expect("write tokenizer");
        let model = dir.path().join("model.onnx");
        std::fs::write(&model, identity_model()).expect("write model");
        super::ort_environment().expect("environment");
        let result = xberg_gliner::Gliner::with_runtime(
            xberg_gliner::Parameters::default(),
            xberg_gliner::RuntimeConfig::default(),
            &tokenizer,
            &model,
        );
        // The identity model has no GLiNER inputs, so loading must get past session creation and
        // stop at schema validation; an `Ort` error here means GLiNER built its own environment.
        assert!(
            matches!(result, Err(xberg_gliner::GlinerError::UnexpectedModelSchema { .. })),
            "unexpected result: {:?}",
            result.err()
        );
    }
}
