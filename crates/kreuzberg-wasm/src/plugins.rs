//! Plugin management for WASM bindings
//!
//! This module provides functions to register, unregister, and manage custom plugins
//! for document extraction, post-processing, validation, and OCR.
//!
//! All plugin functions operate on plugin objects passed from JavaScript that implement
//! the appropriate plugin protocol.
//!
//! # Threading Model and Safety
//!
//! This module uses `unsafe` to implement Send/Sync for types containing JavaScript
//! values (JsValue). Safety depends on the threading mode:
//!
//! ## Single-threaded Mode (Default)
//! - WASM execution is single-threaded by default
//! - All `unsafe impl Send/Sync` are unconditionally sound
//! - Only the main JS thread can access JsValue objects
//!
//! ## Multi-threaded Mode (After `initThreadPool()`)
//! - Multiple Web Workers exist with separate JS contexts
//! - JsValue access from wrong context = undefined behavior
//! - **CRITICAL**: Plugin callbacks MUST ONLY execute on main thread
//! - Currently no compile-time or runtime enforcement mechanism exists
//!
//! ## Known Limitation
//! The type system does not prevent JsValue access from worker threads.
//! When multi-threading is enabled, users must ensure plugin callbacks
//! are never invoked from worker contexts. Future work should implement
//! message-passing for sound multi-threading support.

#[allow(unused_imports)]
use async_trait::async_trait;
#[allow(unused_imports)]
use js_sys::{Promise, Reflect};
use kreuzberg::plugins::{OcrBackend, Plugin, PostProcessor, ProcessingStage, Validator};
#[allow(unused_imports)]
use kreuzberg::{ExtractionConfig, ExtractionResult, KreuzbergError, OcrConfig};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use wasm_bindgen::prelude::*;
#[allow(unused_imports)]
use wasm_bindgen_futures::JsFuture;

// ============================================================================
// LOCK POISONING RECOVERY HELPERS
// ============================================================================

/// Attempt to acquire a write lock with detailed error context and poisoning recovery.
///
/// When lock poisoning occurs, this function:
/// 1. Extracts the inner guard (which may still be usable)
/// 2. Logs a warning to the browser console about the poisoning
/// 3. Returns the guard for recovery attempts
///
/// # Context Provided
/// - Which registry failed (POST_PROCESSORS, VALIDATORS, or OCR_BACKENDS)
/// - Clear indication that the data may be in an inconsistent state
fn acquire_write_lock<'a, T>(registry: &'a RwLock<T>, registry_name: &str) -> Result<RwLockWriteGuard<'a, T>, String> {
    match registry.write() {
        Ok(guard) => Ok(guard),
        Err(poison) => {
            // SAFETY: We're extracting the guard from the poison error. The guard contains
            // the data even though the lock was poisoned. This is safe as long as the lock
            // holder didn't corrupt the data before panicking.
            let guard = poison.into_inner();
            web_sys::console::warn_1(
                &format!(
                    "WARN: {} registry write lock was poisoned but recovered; data may be in inconsistent state",
                    registry_name
                )
                .into(),
            );
            Ok(guard)
        }
    }
}

/// Attempt to acquire a read lock with detailed error context about poisoning.
///
/// For read operations, we cannot safely recover from poisoning since we can't verify
/// the data is uncorrupted. Returns a contextual error message indicating:
/// - Which registry failed
/// - That the lock is poisoned
/// - A hint that a previous operation may have panicked
fn acquire_read_lock<'a, T>(registry: &'a RwLock<T>, registry_name: &str) -> Result<RwLockReadGuard<'a, T>, String> {
    registry.read().map_err(|_| {
        format!(
            "Failed to acquire {} registry read lock: lock poisoned (possible panic in previous operation)",
            registry_name
        )
    })
}

// ============================================================================
// HELPER: MakeSend wrapper for non-Send futures in WASM
// ============================================================================

/// Wrapper to make a non-Send future Send in WASM (single-threaded environment).
///
/// Safety depends on whether multi-threading is enabled via `initThreadPool()`:
/// - Single-threaded (default): Safe to make any future Send
/// - Multi-threaded: Only safe for futures that never access JsValue from workers
#[allow(dead_code)]
struct MakeSend<F>(F);

impl<F: Future + Unpin> Future for MakeSend<F> {
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx)
    }
}

/// SAFETY: This wrapper makes futures Send by assuming they execute in
/// controlled contexts. Safety depends on usage:
///
/// **Single-threaded mode (default)**: Unconditionally safe. WASM execution
/// is single-threaded, preventing concurrent access.
///
/// **Multi-threaded mode (when `initThreadPool()` called)**: CONDITIONALLY SAFE.
/// Plugin callbacks MUST NOT be invoked from worker threads. Currently relies
/// on runtime ensuring plugins only execute on main thread. This is not
/// enforced by the type system.
///
/// **Current safety guarantee**: As long as plugin callbacks are only invoked
/// from the main thread (not from rayon workers), no data races can occur.
///
/// **Future improvement**: Replace with message-passing architecture for
/// sound multi-threading (see issue #TODO).
///
/// Used with `#[async_trait(?Send)]` to allow JavaScript async callbacks.
unsafe impl<F> Send for MakeSend<F> {}

/// SAFETY: Safe for the same conditional reasons as Send.
///
/// Single-threaded: Unconditionally safe.
/// Multi-threaded: Conditional on main-thread-only execution (same as Send above).
unsafe impl<F> Sync for MakeSend<F> {}

/// Newtype wrapper around JsValue that implements Send/Sync for use in Rust async contexts.
///
/// JsValue is normally !Send and !Sync because JavaScript objects can only be accessed
/// from the JavaScript engine's context. Safety depends on the threading mode.
#[derive(Clone)]
struct SendableJsValue(#[allow(dead_code)] JsValue);

/// SAFETY: JsValue is normally !Send/!Sync because it contains pointers to
/// JavaScript objects that are only valid in a specific JS context.
///
/// **Single-threaded mode**: Unconditionally safe - only one JS context exists.
///
/// **Multi-threaded mode (rayon enabled)**: CONDITIONALLY SAFE.
/// Safe ONLY if JsValue is never accessed from worker threads.
/// Current code assumes all JsValue access happens on main thread.
///
/// **Risk**: If a worker thread accesses JsValue, undefined behavior occurs.
/// No compile-time or runtime enforcement exists.
///
/// **Mitigation**: Plugin registry methods should document that callbacks
/// must not be invoked from worker contexts.
unsafe impl Send for SendableJsValue {}

/// SAFETY: Safe for the same conditional reasons as Send.
///
/// Single-threaded: Unconditionally safe.
/// Multi-threaded: Conditional on main-thread-only access (same as Send above).
unsafe impl Sync for SendableJsValue {}

// ============================================================================
// POST-PROCESSOR WRAPPER AND FUNCTIONS
// ============================================================================

/// Wrapper that makes a JavaScript PostProcessor object usable from Rust.
struct JsPostProcessorWrapper {
    name: String,
    #[allow(dead_code)]
    js_obj: SendableJsValue,
    stage: ProcessingStage,
}

impl JsPostProcessorWrapper {
    /// Create a new wrapper from a JS object
    fn new(js_obj: JsValue, name: String, stage: ProcessingStage) -> Self {
        Self {
            js_obj: SendableJsValue(js_obj),
            name,
            stage,
        }
    }
}

impl Plugin for JsPostProcessorWrapper {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> kreuzberg::Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> kreuzberg::Result<()> {
        Ok(())
    }
}

#[cfg(not(test))]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl PostProcessor for JsPostProcessorWrapper {
    async fn process(&self, result: &mut ExtractionResult, _config: &ExtractionConfig) -> kreuzberg::Result<()> {
        // Convert Rust result to JSON
        let json_input = serde_json::to_string(&*result).map_err(|e| KreuzbergError::Plugin {
            message: format!("Failed to serialize extraction result: {}", e),
            plugin_name: self.name.clone(),
        })?;

        // Call the JS process function
        let json_input_copy = json_input.clone();
        let name_copy = self.name.clone();

        // Wrap the JS interaction to handle non-Send futures
        let promise_val = {
            let process_fn = Reflect::get(&self.js_obj.0, &JsValue::from_str("process"))
                .map_err(|_| KreuzbergError::Plugin {
                    message: format!("PostProcessor '{}' missing 'process' method", self.name),
                    plugin_name: self.name.clone(),
                })?
                .dyn_into::<js_sys::Function>()
                .map_err(|_| KreuzbergError::Plugin {
                    message: format!("PostProcessor '{}' process is not a function", self.name),
                    plugin_name: self.name.clone(),
                })?;

            let promise_val = process_fn
                .call1(&self.js_obj.0, &JsValue::from_str(&json_input_copy))
                .map_err(|e| KreuzbergError::Plugin {
                    message: format!("PostProcessor '{}' process call failed: {:?}", name_copy, e),
                    plugin_name: name_copy.clone(),
                })?;

            SendableJsValue(promise_val)
        };

        // Wait for the promise - wrap in MakeSend for WASM compatibility
        let promise = Promise::resolve(&promise_val.0);
        let result_val = MakeSend(JsFuture::from(promise))
            .await
            .map_err(|e| KreuzbergError::Plugin {
                message: format!("PostProcessor '{}' promise failed: {:?}", self.name, e),
                plugin_name: self.name.clone(),
            })?;

        let json_output = result_val.as_string().ok_or_else(|| KreuzbergError::Plugin {
            message: format!("PostProcessor '{}' returned non-string result", self.name),
            plugin_name: self.name.clone(),
        })?;

        // Parse the result back
        let updated: ExtractionResult = serde_json::from_str(&json_output).map_err(|e| KreuzbergError::Plugin {
            message: format!("Failed to deserialize PostProcessor result: {}", e),
            plugin_name: self.name.clone(),
        })?;

        *result = updated;
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        self.stage
    }
}

#[cfg(test)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl PostProcessor for JsPostProcessorWrapper {
    async fn process(&self, _result: &mut ExtractionResult, _config: &ExtractionConfig) -> kreuzberg::Result<()> {
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        self.stage
    }
}

/// Register a custom post-processor.
///
/// # Arguments
///
/// * `processor` - JavaScript object implementing the PostProcessorProtocol interface:
///   - `name(): string` - Unique processor name
///   - `process(jsonString: string): Promise<string>` - Process function that takes JSON input
///   - `processingStage(): "early" | "middle" | "late"` - Optional processing stage (defaults to "middle")
///
/// # Returns
///
/// Ok if registration succeeds, Err with description if it fails.
///
/// # Example
///
/// ```javascript
/// registerPostProcessor({
///   name: () => "my-post-processor",
///   processingStage: () => "middle",
///   process: async (jsonString) => {
///     const result = JSON.parse(jsonString);
///     // Process the extraction result
///     result.metadata.processed_by = "my-post-processor";
///     return JSON.stringify(result);
///   }
/// });
/// ```
#[wasm_bindgen]
pub fn register_post_processor(processor: JsValue) -> Result<(), JsValue> {
    // Validate required methods
    let name_fn =
        Reflect::get(&processor, &JsValue::from_str("name")).map_err(|e| format!("Missing 'name' method: {:?}", e))?;

    let process_fn = Reflect::get(&processor, &JsValue::from_str("process"))
        .map_err(|e| format!("Missing 'process' method: {:?}", e))?;

    if !name_fn.is_function() || !process_fn.is_function() {
        return Err(JsValue::from_str("name and process must be functions"));
    }

    // Get the name
    let name_fn = name_fn
        .dyn_into::<js_sys::Function>()
        .map_err(|_| "Failed to convert name to function")?;
    let name = name_fn
        .call0(&processor)
        .map_err(|e| format!("Failed to call name(): {:?}", e))?
        .as_string()
        .ok_or("name() must return a string")?;

    if name.is_empty() {
        return Err(JsValue::from_str("Processor name cannot be empty"));
    }

    // Get the processing stage
    let stage = if let Ok(stage_fn) = Reflect::get(&processor, &JsValue::from_str("processingStage")) {
        if stage_fn.is_function() {
            let stage_fn = stage_fn
                .dyn_into::<js_sys::Function>()
                .map_err(|_| "Failed to convert processingStage to function")?;
            let stage_str = stage_fn
                .call0(&processor)
                .map_err(|e| format!("Failed to call processingStage(): {:?}", e))?
                .as_string()
                .unwrap_or_else(|| "middle".to_string());

            match stage_str.to_lowercase().as_str() {
                "early" => ProcessingStage::Early,
                "late" => ProcessingStage::Late,
                _ => ProcessingStage::Middle,
            }
        } else {
            ProcessingStage::Middle
        }
    } else {
        ProcessingStage::Middle
    };

    // Create wrapper and register
    let wrapper = JsPostProcessorWrapper::new(processor, name.clone(), stage);
    let registry = kreuzberg::plugins::registry::get_post_processor_registry();
    let mut registry = acquire_write_lock(&registry, "POST_PROCESSORS").map_err(|e| JsValue::from_str(&e))?;

    registry
        .register(Arc::new(wrapper), 0)
        .map_err(|e| JsValue::from_str(&format!("Registration failed: {}", e)))
}

/// Unregister a post-processor by name.
///
/// # Arguments
///
/// * `name` - Name of the post-processor to unregister
///
/// # Returns
///
/// Ok if unregistration succeeds, Err if the processor is not found or other error occurs.
///
/// # Example
///
/// ```javascript
/// unregisterPostProcessor("my-post-processor");
/// ```
#[wasm_bindgen]
pub fn unregister_post_processor(name: String) -> Result<(), JsValue> {
    let registry = kreuzberg::plugins::registry::get_post_processor_registry();
    let mut registry = acquire_write_lock(&registry, "POST_PROCESSORS").map_err(|e| JsValue::from_str(&e))?;

    registry
        .remove(&name)
        .map_err(|e| JsValue::from_str(&format!("Unregistration failed: {}", e)))
}

/// Clear all registered post-processors.
///
/// # Returns
///
/// Ok if clearing succeeds, Err if an error occurs.
///
/// # Example
///
/// ```javascript
/// clearPostProcessors();
/// ```
#[wasm_bindgen]
pub fn clear_post_processors() -> Result<(), JsValue> {
    let registry = kreuzberg::plugins::registry::get_post_processor_registry();
    let mut registry = acquire_write_lock(&registry, "POST_PROCESSORS").map_err(|e| JsValue::from_str(&e))?;

    let names = registry.list();
    for name in names {
        registry
            .remove(&name)
            .map_err(|e| JsValue::from_str(&format!("Failed to remove post-processor: {}", e)))?;
    }

    Ok(())
}

/// List all registered post-processor names.
///
/// # Returns
///
/// Array of post-processor names, or Err if an error occurs.
///
/// # Example
///
/// ```javascript
/// const processors = listPostProcessors();
/// console.log(processors); // ["my-post-processor", ...]
/// ```
#[wasm_bindgen]
pub fn list_post_processors() -> Result<js_sys::Array, JsValue> {
    let registry = kreuzberg::plugins::registry::get_post_processor_registry();
    let registry = acquire_read_lock(&registry, "POST_PROCESSORS").map_err(|e| JsValue::from_str(&e))?;

    let names = registry.list();
    let arr = js_sys::Array::new();
    for name in names {
        arr.push(&JsValue::from_str(&name));
    }

    Ok(arr)
}

// ============================================================================
// VALIDATOR WRAPPER AND FUNCTIONS
// ============================================================================

/// Wrapper that makes a JavaScript Validator object usable from Rust.
struct JsValidatorWrapper {
    name: String,
    #[allow(dead_code)]
    js_obj: SendableJsValue,
    #[allow(dead_code)]
    priority: i32,
}

impl JsValidatorWrapper {
    /// Create a new wrapper from a JS object
    fn new(js_obj: JsValue, name: String, priority: i32) -> Self {
        Self {
            js_obj: SendableJsValue(js_obj),
            name,
            priority,
        }
    }
}

impl Plugin for JsValidatorWrapper {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> kreuzberg::Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> kreuzberg::Result<()> {
        Ok(())
    }
}

#[cfg(not(test))]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Validator for JsValidatorWrapper {
    async fn validate(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> kreuzberg::Result<()> {
        // Convert Rust result to JSON
        let json_input = serde_json::to_string(result).map_err(|e| KreuzbergError::Plugin {
            message: format!("Failed to serialize extraction result: {}", e),
            plugin_name: self.name.clone(),
        })?;

        // Wrap the JS interaction to handle non-Send futures
        let promise_val = {
            let validate_fn = Reflect::get(&self.js_obj.0, &JsValue::from_str("validate"))
                .map_err(|_| KreuzbergError::Plugin {
                    message: format!("Validator '{}' missing 'validate' method", self.name),
                    plugin_name: self.name.clone(),
                })?
                .dyn_into::<js_sys::Function>()
                .map_err(|_| KreuzbergError::Plugin {
                    message: format!("Validator '{}' validate is not a function", self.name),
                    plugin_name: self.name.clone(),
                })?;

            let promise_val = validate_fn
                .call1(&self.js_obj.0, &JsValue::from_str(&json_input))
                .map_err(|e| KreuzbergError::Plugin {
                    message: format!("Validator '{}' validate call failed: {:?}", self.name, e),
                    plugin_name: self.name.clone(),
                })?;

            SendableJsValue(promise_val)
        };

        // Wait for the promise - wrap in MakeSend for WASM compatibility
        let promise = Promise::resolve(&promise_val.0);
        let result_val = MakeSend(JsFuture::from(promise)).await.map_err(|e| {
            let err_msg = format!("{:?}", e);
            if err_msg.contains("ValidationError") || err_msg.contains("validation") {
                KreuzbergError::Validation {
                    message: err_msg,
                    source: None,
                }
            } else {
                KreuzbergError::Plugin {
                    message: format!("Validator '{}' promise failed: {}", self.name, err_msg),
                    plugin_name: self.name.clone(),
                }
            }
        })?;

        // Check if result is an error or success
        // Empty string or undefined = success, any error message = validation failure
        if let Some(error_msg) = result_val.as_string()
            && !error_msg.is_empty()
        {
            return Err(KreuzbergError::Validation {
                message: error_msg,
                source: None,
            });
        }

        Ok(())
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

#[cfg(test)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Validator for JsValidatorWrapper {
    async fn validate(&self, _result: &ExtractionResult, _config: &ExtractionConfig) -> kreuzberg::Result<()> {
        Ok(())
    }
}

/// Register a custom validator.
///
/// # Arguments
///
/// * `validator` - JavaScript object implementing the ValidatorProtocol interface:
///   - `name(): string` - Unique validator name
///   - `validate(jsonString: string): Promise<string>` - Validation function returning empty string on success, error message on failure
///   - `priority(): number` - Optional priority (defaults to 50, higher runs first)
///
/// # Returns
///
/// Ok if registration succeeds, Err with description if it fails.
///
/// # Example
///
/// ```javascript
/// registerValidator({
///   name: () => "min-content-length",
///   priority: () => 100,
///   validate: async (jsonString) => {
///     const result = JSON.parse(jsonString);
///     if (result.content.length < 100) {
///       return "Content too short"; // Validation failure
///     }
///     return ""; // Success
///   }
/// });
/// ```
#[wasm_bindgen]
pub fn register_validator(validator: JsValue) -> Result<(), JsValue> {
    // Validate required methods
    let name_fn =
        Reflect::get(&validator, &JsValue::from_str("name")).map_err(|e| format!("Missing 'name' method: {:?}", e))?;

    let validate_fn = Reflect::get(&validator, &JsValue::from_str("validate"))
        .map_err(|e| format!("Missing 'validate' method: {:?}", e))?;

    if !name_fn.is_function() || !validate_fn.is_function() {
        return Err(JsValue::from_str("name and validate must be functions"));
    }

    // Get the name
    let name_fn = name_fn
        .dyn_into::<js_sys::Function>()
        .map_err(|_| "Failed to convert name to function")?;
    let name = name_fn
        .call0(&validator)
        .map_err(|e| format!("Failed to call name(): {:?}", e))?
        .as_string()
        .ok_or("name() must return a string")?;

    if name.is_empty() {
        return Err(JsValue::from_str("Validator name cannot be empty"));
    }

    // Get the priority
    let priority = if let Ok(priority_fn) = Reflect::get(&validator, &JsValue::from_str("priority")) {
        if priority_fn.is_function() {
            let priority_fn = priority_fn
                .dyn_into::<js_sys::Function>()
                .map_err(|_| "Failed to convert priority to function")?;
            priority_fn
                .call0(&validator)
                .map_err(|e| format!("Failed to call priority(): {:?}", e))?
                .as_f64()
                .map(|n| n as i32)
                .unwrap_or(50)
        } else {
            50
        }
    } else {
        50
    };

    // Create wrapper and register
    let wrapper = JsValidatorWrapper::new(validator, name.clone(), priority);
    let registry = kreuzberg::plugins::registry::get_validator_registry();
    let mut registry = acquire_write_lock(&registry, "VALIDATORS").map_err(|e| JsValue::from_str(&e))?;

    registry
        .register(Arc::new(wrapper))
        .map_err(|e| JsValue::from_str(&format!("Registration failed: {}", e)))
}

/// Unregister a validator by name.
///
/// # Arguments
///
/// * `name` - Name of the validator to unregister
///
/// # Returns
///
/// Ok if unregistration succeeds, Err if the validator is not found or other error occurs.
///
/// # Example
///
/// ```javascript
/// unregisterValidator("min-content-length");
/// ```
#[wasm_bindgen]
pub fn unregister_validator(name: String) -> Result<(), JsValue> {
    let registry = kreuzberg::plugins::registry::get_validator_registry();
    let mut registry = acquire_write_lock(&registry, "VALIDATORS").map_err(|e| JsValue::from_str(&e))?;

    registry
        .remove(&name)
        .map_err(|e| JsValue::from_str(&format!("Unregistration failed: {}", e)))
}

/// Clear all registered validators.
///
/// # Returns
///
/// Ok if clearing succeeds, Err if an error occurs.
///
/// # Example
///
/// ```javascript
/// clearValidators();
/// ```
#[wasm_bindgen]
pub fn clear_validators() -> Result<(), JsValue> {
    let registry = kreuzberg::plugins::registry::get_validator_registry();
    let mut registry = acquire_write_lock(&registry, "VALIDATORS").map_err(|e| JsValue::from_str(&e))?;

    let names = registry.list();
    for name in names {
        registry
            .remove(&name)
            .map_err(|e| JsValue::from_str(&format!("Failed to remove validator: {}", e)))?;
    }

    Ok(())
}

/// List all registered validator names.
///
/// # Returns
///
/// Array of validator names, or Err if an error occurs.
///
/// # Example
///
/// ```javascript
/// const validators = listValidators();
/// console.log(validators); // ["min-content-length", ...]
/// ```
#[wasm_bindgen]
pub fn list_validators() -> Result<js_sys::Array, JsValue> {
    let registry = kreuzberg::plugins::registry::get_validator_registry();
    let registry = acquire_read_lock(&registry, "VALIDATORS").map_err(|e| JsValue::from_str(&e))?;

    let names = registry.list();
    let arr = js_sys::Array::new();
    for name in names {
        arr.push(&JsValue::from_str(&name));
    }

    Ok(arr)
}

// ============================================================================
// OCR BACKEND WRAPPER AND FUNCTIONS
// ============================================================================

/// Wrapper that makes a JavaScript OcrBackend object usable from Rust.
struct JsOcrBackendWrapper {
    name: String,
    #[allow(dead_code)]
    js_obj: SendableJsValue,
    #[allow(dead_code)]
    supported_languages: Vec<String>,
}

impl JsOcrBackendWrapper {
    /// Create a new wrapper from a JS object
    fn new(js_obj: JsValue, name: String, supported_languages: Vec<String>) -> Self {
        Self {
            js_obj: SendableJsValue(js_obj),
            name,
            supported_languages,
        }
    }
}

impl Plugin for JsOcrBackendWrapper {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> kreuzberg::Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> kreuzberg::Result<()> {
        Ok(())
    }
}

#[cfg(not(test))]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl OcrBackend for JsOcrBackendWrapper {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> kreuzberg::Result<ExtractionResult> {
        // Encode image as base64
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(image_bytes);

        // Wrap the JS interaction to handle non-Send futures
        let promise_val = {
            let process_fn = Reflect::get(&self.js_obj.0, &JsValue::from_str("processImage"))
                .map_err(|_| KreuzbergError::Ocr {
                    message: format!("OCR backend '{}' missing 'processImage' method", self.name),
                    source: None,
                })?
                .dyn_into::<js_sys::Function>()
                .map_err(|_| KreuzbergError::Ocr {
                    message: format!("OCR backend '{}' processImage is not a function", self.name),
                    source: None,
                })?;

            let language = config.language.clone();
            let promise_val = process_fn
                .call2(
                    &self.js_obj.0,
                    &JsValue::from_str(&encoded),
                    &JsValue::from_str(&language),
                )
                .map_err(|e| KreuzbergError::Ocr {
                    message: format!("OCR backend '{}' processImage call failed: {:?}", self.name, e),
                    source: None,
                })?;

            SendableJsValue(promise_val)
        };

        // Wait for the promise - wrap in MakeSend for WASM compatibility
        let promise = Promise::resolve(&promise_val.0);
        let result_val = MakeSend(JsFuture::from(promise))
            .await
            .map_err(|e| KreuzbergError::Ocr {
                message: format!("OCR backend '{}' promise failed: {:?}", self.name, e),
                source: None,
            })?;

        let json_output = result_val.as_string().ok_or_else(|| KreuzbergError::Ocr {
            message: format!("OCR backend '{}' returned non-string result", self.name),
            source: None,
        })?;

        // Parse the JSON result
        let result: serde_json::Value = serde_json::from_str(&json_output).map_err(|e| KreuzbergError::Ocr {
            message: format!("Failed to parse OCR result: {}", e),
            source: None,
        })?;

        // Extract fields from the result
        let content = result
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| KreuzbergError::Ocr {
                message: format!("OCR backend '{}' result missing 'content' field", self.name),
                source: None,
            })?
            .to_string();

        let mime_type = result
            .get("mime_type")
            .and_then(|v| v.as_str())
            .unwrap_or("text/plain")
            .to_string();

        let metadata = result
            .get("metadata")
            .cloned()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        let metadata: kreuzberg::types::Metadata =
            serde_json::from_value(metadata).map_err(|e| KreuzbergError::Ocr {
                message: format!("Failed to parse OCR metadata: {}", e),
                source: None,
            })?;

        let tables = result
            .get("tables")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| serde_json::from_value::<kreuzberg::Table>(t.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();

        Ok(ExtractionResult {
            content,
            mime_type,
            metadata,
            tables,
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
        })
    }

    async fn process_file(&self, path: &std::path::Path, config: &OcrConfig) -> kreuzberg::Result<ExtractionResult> {
        use kreuzberg::core::io;
        let bytes = io::read_file_sync(path)?;
        self.process_image(&bytes, config).await
    }

    fn supports_language(&self, lang: &str) -> bool {
        self.supported_languages.iter().any(|l| l == lang)
    }

    fn backend_type(&self) -> kreuzberg::plugins::OcrBackendType {
        kreuzberg::plugins::OcrBackendType::Custom
    }

    fn supported_languages(&self) -> Vec<String> {
        self.supported_languages.clone()
    }
}

#[cfg(test)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl OcrBackend for JsOcrBackendWrapper {
    async fn process_image(&self, _image_bytes: &[u8], _config: &OcrConfig) -> kreuzberg::Result<ExtractionResult> {
        Ok(ExtractionResult {
            content: String::new(),
            mime_type: "image/jpeg".to_string(),
            metadata: Default::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
        })
    }

    async fn process_file(&self, _path: &std::path::Path, _config: &OcrConfig) -> kreuzberg::Result<ExtractionResult> {
        Ok(ExtractionResult {
            content: String::new(),
            mime_type: "image/jpeg".to_string(),
            metadata: Default::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
        })
    }

    fn supports_language(&self, _lang: &str) -> bool {
        false
    }

    fn backend_type(&self) -> kreuzberg::plugins::OcrBackendType {
        kreuzberg::plugins::OcrBackendType::Custom
    }

    fn supported_languages(&self) -> Vec<String> {
        vec![]
    }
}

/// Register a custom OCR backend.
///
/// # Arguments
///
/// * `backend` - JavaScript object implementing the OcrBackendProtocol interface:
///   - `name(): string` - Unique backend name
///   - `supportedLanguages(): string[]` - Array of language codes the backend supports
///   - `processImage(imageBase64: string, language: string): Promise<string>` - Process image and return JSON result
///
/// # Returns
///
/// Ok if registration succeeds, Err with description if it fails.
///
/// # Example
///
/// ```javascript
/// registerOcrBackend({
///   name: () => "custom-ocr",
///   supportedLanguages: () => ["en", "es", "fr"],
///   processImage: async (imageBase64, language) => {
///     const buffer = Buffer.from(imageBase64, "base64");
///     // Process image with custom OCR engine
///     const text = await customOcrEngine.recognize(buffer, language);
///     return JSON.stringify({
///       content: text,
///       mime_type: "text/plain",
///       metadata: {}
///     });
///   }
/// });
/// ```
#[wasm_bindgen]
pub fn register_ocr_backend(backend: JsValue) -> Result<(), JsValue> {
    // Validate required methods
    let name_fn =
        Reflect::get(&backend, &JsValue::from_str("name")).map_err(|e| format!("Missing 'name' method: {:?}", e))?;

    let langs_fn = Reflect::get(&backend, &JsValue::from_str("supportedLanguages"))
        .map_err(|e| format!("Missing 'supportedLanguages' method: {:?}", e))?;

    let process_fn = Reflect::get(&backend, &JsValue::from_str("processImage"))
        .map_err(|e| format!("Missing 'processImage' method: {:?}", e))?;

    if !name_fn.is_function() || !langs_fn.is_function() || !process_fn.is_function() {
        return Err(JsValue::from_str(
            "name, supportedLanguages, and processImage must be functions",
        ));
    }

    // Get the name
    let name_fn = name_fn
        .dyn_into::<js_sys::Function>()
        .map_err(|_| "Failed to convert name to function")?;
    let name = name_fn
        .call0(&backend)
        .map_err(|e| format!("Failed to call name(): {:?}", e))?
        .as_string()
        .ok_or("name() must return a string")?;

    if name.is_empty() {
        return Err(JsValue::from_str("OCR backend name cannot be empty"));
    }

    // Get supported languages
    let langs_fn = langs_fn
        .dyn_into::<js_sys::Function>()
        .map_err(|_| "Failed to convert supportedLanguages to function")?;
    let langs_val = langs_fn
        .call0(&backend)
        .map_err(|e| format!("Failed to call supportedLanguages(): {:?}", e))?;

    let langs_array = js_sys::Array::from(&langs_val);
    let mut supported_languages = Vec::new();
    for i in 0..langs_array.length() {
        if let Some(lang) = langs_array.get(i).as_string() {
            supported_languages.push(lang);
        }
    }

    if supported_languages.is_empty() {
        return Err(JsValue::from_str("OCR backend must support at least one language"));
    }

    // Create wrapper and register
    let wrapper = JsOcrBackendWrapper::new(backend, name.clone(), supported_languages);
    let registry = kreuzberg::plugins::registry::get_ocr_backend_registry();
    let mut registry = acquire_write_lock(&registry, "OCR_BACKENDS").map_err(|e| JsValue::from_str(&e))?;

    registry
        .register(Arc::new(wrapper))
        .map_err(|e| JsValue::from_str(&format!("Registration failed: {}", e)))
}

/// Unregister an OCR backend by name.
///
/// # Arguments
///
/// * `name` - Name of the OCR backend to unregister
///
/// # Returns
///
/// Ok if unregistration succeeds, Err if the backend is not found or other error occurs.
///
/// # Example
///
/// ```javascript
/// unregisterOcrBackend("custom-ocr");
/// ```
#[wasm_bindgen]
pub fn unregister_ocr_backend(name: String) -> Result<(), JsValue> {
    let registry = kreuzberg::plugins::registry::get_ocr_backend_registry();
    let mut registry = acquire_write_lock(&registry, "OCR_BACKENDS").map_err(|e| JsValue::from_str(&e))?;

    registry
        .remove(&name)
        .map_err(|e| JsValue::from_str(&format!("Unregistration failed: {}", e)))
}

/// Clear all registered OCR backends.
///
/// # Returns
///
/// Ok if clearing succeeds, Err if an error occurs.
///
/// # Example
///
/// ```javascript
/// clearOcrBackends();
/// ```
#[wasm_bindgen]
pub fn clear_ocr_backends() -> Result<(), JsValue> {
    let registry = kreuzberg::plugins::registry::get_ocr_backend_registry();
    let mut registry = acquire_write_lock(&registry, "OCR_BACKENDS").map_err(|e| JsValue::from_str(&e))?;

    let names = registry.list();
    for name in names {
        registry
            .remove(&name)
            .map_err(|e| JsValue::from_str(&format!("Failed to remove OCR backend: {}", e)))?;
    }

    Ok(())
}

/// List all registered OCR backend names.
///
/// # Returns
///
/// Array of OCR backend names, or Err if an error occurs.
///
/// # Example
///
/// ```javascript
/// const backends = listOcrBackends();
/// console.log(backends); // ["tesseract", "custom-ocr", ...]
/// ```
#[wasm_bindgen]
pub fn list_ocr_backends() -> Result<js_sys::Array, JsValue> {
    let registry = kreuzberg::plugins::registry::get_ocr_backend_registry();
    let registry = acquire_read_lock(&registry, "OCR_BACKENDS").map_err(|e| JsValue::from_str(&e))?;

    let names = registry.list();
    let arr = js_sys::Array::new();
    for name in names {
        arr.push(&JsValue::from_str(&name));
    }

    Ok(arr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    // Helper to create a mock processor JS object
    fn create_mock_processor(name: &str) -> Result<JsValue, String> {
        let obj = js_sys::Object::new();

        Reflect::set(
            &obj,
            &JsValue::from_str("name"),
            &js_sys::Function::new_with_args("", &format!("return '{}'", name)),
        )
        .map_err(|_| "Failed to set name method".to_string())?;

        Reflect::set(
            &obj,
            &JsValue::from_str("process"),
            &js_sys::Function::new_with_args("json", "return Promise.resolve(json)"),
        )
        .map_err(|_| "Failed to set process method".to_string())?;

        Reflect::set(
            &obj,
            &JsValue::from_str("processingStage"),
            &js_sys::Function::new_with_args("", "return 'middle'"),
        )
        .map_err(|_| "Failed to set processingStage method".to_string())?;

        Ok(JsValue::from(obj))
    }

    // Helper to create a mock validator JS object
    fn create_mock_validator(name: &str) -> Result<JsValue, String> {
        let obj = js_sys::Object::new();

        Reflect::set(
            &obj,
            &JsValue::from_str("name"),
            &js_sys::Function::new_with_args("", &format!("return '{}'", name)),
        )
        .map_err(|_| "Failed to set name method".to_string())?;

        Reflect::set(
            &obj,
            &JsValue::from_str("validate"),
            &js_sys::Function::new_with_args("json", "return Promise.resolve('')"),
        )
        .map_err(|_| "Failed to set validate method".to_string())?;

        Reflect::set(
            &obj,
            &JsValue::from_str("priority"),
            &js_sys::Function::new_with_args("", "return 50"),
        )
        .map_err(|_| "Failed to set priority method".to_string())?;

        Ok(JsValue::from(obj))
    }

    // Helper to create a mock OCR backend JS object
    fn create_mock_ocr_backend(name: &str) -> Result<JsValue, String> {
        let obj = js_sys::Object::new();

        Reflect::set(
            &obj,
            &JsValue::from_str("name"),
            &js_sys::Function::new_with_args("", &format!("return '{}'", name)),
        )
        .map_err(|_| "Failed to set name method".to_string())?;

        Reflect::set(
            &obj,
            &JsValue::from_str("supportedLanguages"),
            &js_sys::Function::new_with_args("", "return ['en']"),
        )
        .map_err(|_| "Failed to set supportedLanguages method".to_string())?;

        Reflect::set(
            &obj,
            &JsValue::from_str("processImage"),
            &js_sys::Function::new_with_args(
                "imageBase64,language",
                "return Promise.resolve('{\"content\":\"test\"}')",
            ),
        )
        .map_err(|_| "Failed to set processImage method".to_string())?;

        Ok(JsValue::from(obj))
    }

    #[wasm_bindgen_test]
    fn test_register_post_processor_valid_processor_succeeds() {
        clear_post_processors().ok();
        let processor = create_mock_processor("test-processor").expect("Failed to create mock processor");

        let result = register_post_processor(processor);

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_register_post_processor_missing_name_fails() {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("process"),
            &js_sys::Function::new_with_args("json", "return Promise.resolve(json)"),
        )
        .ok();

        let result = register_post_processor(JsValue::from(obj));

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_register_post_processor_missing_process_fails() {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("name"),
            &js_sys::Function::new_with_args("", "return 'test'"),
        )
        .ok();

        let result = register_post_processor(JsValue::from(obj));

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_register_post_processor_empty_name_fails() {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("name"),
            &js_sys::Function::new_with_args("", "return ''"),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("process"),
            &js_sys::Function::new_with_args("json", "return Promise.resolve(json)"),
        )
        .ok();

        let result = register_post_processor(JsValue::from(obj));

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_unregister_post_processor_registered_processor_succeeds() {
        clear_post_processors().ok();
        let processor = create_mock_processor("test-processor").expect("Failed to create mock processor");
        register_post_processor(processor).ok();

        let result = unregister_post_processor("test-processor".to_string());

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_unregister_post_processor_unregistered_processor_fails() {
        clear_post_processors().ok();

        let result = unregister_post_processor("nonexistent".to_string());

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_clear_post_processors_removes_all() {
        clear_post_processors().ok();
        let processor1 = create_mock_processor("processor1").expect("Failed to create mock processor 1");
        let processor2 = create_mock_processor("processor2").expect("Failed to create mock processor 2");
        register_post_processor(processor1).ok();
        register_post_processor(processor2).ok();

        let result = clear_post_processors();

        assert!(result.is_ok());
        let list = list_post_processors().unwrap_or_else(|_| js_sys::Array::new());
        assert_eq!(list.length(), 0);
    }

    #[wasm_bindgen_test]
    fn test_list_post_processors_returns_array() {
        clear_post_processors().ok();

        let result = list_post_processors();

        assert!(result.is_ok());
        let arr = result.unwrap();
        assert!(arr.is_array());
    }

    #[wasm_bindgen_test]
    fn test_list_post_processors_after_register_contains_name() {
        clear_post_processors().ok();
        let processor = create_mock_processor("test-processor").expect("Failed to create mock processor");
        register_post_processor(processor).ok();

        let result = list_post_processors();

        assert!(result.is_ok());
        let arr = result.unwrap();
        assert!(arr.length() > 0);
    }

    #[wasm_bindgen_test]
    fn test_register_validator_valid_validator_succeeds() {
        clear_validators().ok();
        let validator = create_mock_validator("test-validator").expect("Failed to create mock validator");

        let result = register_validator(validator);

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_register_validator_missing_name_fails() {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("validate"),
            &js_sys::Function::new_with_args("json", "return Promise.resolve('')"),
        )
        .ok();

        let result = register_validator(JsValue::from(obj));

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_register_validator_missing_validate_fails() {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("name"),
            &js_sys::Function::new_with_args("", "return 'test'"),
        )
        .ok();

        let result = register_validator(JsValue::from(obj));

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_register_validator_empty_name_fails() {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("name"),
            &js_sys::Function::new_with_args("", "return ''"),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("validate"),
            &js_sys::Function::new_with_args("json", "return Promise.resolve('')"),
        )
        .ok();

        let result = register_validator(JsValue::from(obj));

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_unregister_validator_registered_validator_succeeds() {
        clear_validators().ok();
        let validator = create_mock_validator("test-validator").expect("Failed to create mock validator");
        register_validator(validator).ok();

        let result = unregister_validator("test-validator".to_string());

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_unregister_validator_unregistered_validator_fails() {
        clear_validators().ok();

        let result = unregister_validator("nonexistent".to_string());

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_clear_validators_removes_all() {
        clear_validators().ok();
        let validator1 = create_mock_validator("validator1").expect("Failed to create mock validator 1");
        let validator2 = create_mock_validator("validator2").expect("Failed to create mock validator 2");
        register_validator(validator1).ok();
        register_validator(validator2).ok();

        let result = clear_validators();

        assert!(result.is_ok());
        let list = list_validators().unwrap_or_else(|_| js_sys::Array::new());
        assert_eq!(list.length(), 0);
    }

    #[wasm_bindgen_test]
    fn test_list_validators_returns_array() {
        clear_validators().ok();

        let result = list_validators();

        assert!(result.is_ok());
        let arr = result.unwrap();
        assert!(arr.is_array());
    }

    #[wasm_bindgen_test]
    fn test_list_validators_after_register_contains_name() {
        clear_validators().ok();
        let validator = create_mock_validator("test-validator").expect("Failed to create mock validator");
        register_validator(validator).ok();

        let result = list_validators();

        assert!(result.is_ok());
        let arr = result.unwrap();
        assert!(arr.length() > 0);
    }

    #[wasm_bindgen_test]
    fn test_register_ocr_backend_valid_backend_succeeds() {
        clear_ocr_backends().ok();
        let backend = create_mock_ocr_backend("test-backend").expect("Failed to create mock OCR backend");

        let result = register_ocr_backend(backend);

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_register_ocr_backend_missing_name_fails() {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("supportedLanguages"),
            &js_sys::Function::new_with_args("", "return ['en']"),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("processImage"),
            &js_sys::Function::new_with_args("image,lang", "return Promise.resolve('')"),
        )
        .ok();

        let result = register_ocr_backend(JsValue::from(obj));

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_register_ocr_backend_missing_supported_languages_fails() {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("name"),
            &js_sys::Function::new_with_args("", "return 'test'"),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("processImage"),
            &js_sys::Function::new_with_args("image,lang", "return Promise.resolve('')"),
        )
        .ok();

        let result = register_ocr_backend(JsValue::from(obj));

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_register_ocr_backend_missing_process_image_fails() {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("name"),
            &js_sys::Function::new_with_args("", "return 'test'"),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("supportedLanguages"),
            &js_sys::Function::new_with_args("", "return ['en']"),
        )
        .ok();

        let result = register_ocr_backend(JsValue::from(obj));

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_register_ocr_backend_empty_languages_fails() {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("name"),
            &js_sys::Function::new_with_args("", "return 'test'"),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("supportedLanguages"),
            &js_sys::Function::new_with_args("", "return []"),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("processImage"),
            &js_sys::Function::new_with_args("image,lang", "return Promise.resolve('')"),
        )
        .ok();

        let result = register_ocr_backend(JsValue::from(obj));

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_unregister_ocr_backend_registered_backend_succeeds() {
        clear_ocr_backends().ok();
        let backend = create_mock_ocr_backend("test-backend").expect("Failed to create mock OCR backend");
        register_ocr_backend(backend).ok();

        let result = unregister_ocr_backend("test-backend".to_string());

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_unregister_ocr_backend_unregistered_backend_fails() {
        clear_ocr_backends().ok();

        let result = unregister_ocr_backend("nonexistent".to_string());

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_clear_ocr_backends_removes_all() {
        clear_ocr_backends().ok();
        let backend1 = create_mock_ocr_backend("backend1").expect("Failed to create mock OCR backend 1");
        let backend2 = create_mock_ocr_backend("backend2").expect("Failed to create mock OCR backend 2");
        register_ocr_backend(backend1).ok();
        register_ocr_backend(backend2).ok();

        let result = clear_ocr_backends();

        assert!(result.is_ok());
        let list = list_ocr_backends().unwrap_or_else(|_| js_sys::Array::new());
        assert_eq!(list.length(), 0);
    }

    #[wasm_bindgen_test]
    fn test_list_ocr_backends_returns_array() {
        clear_ocr_backends().ok();

        let result = list_ocr_backends();

        assert!(result.is_ok());
        let arr = result.unwrap();
        assert!(arr.is_array());
    }

    #[wasm_bindgen_test]
    fn test_list_ocr_backends_after_register_contains_name() {
        clear_ocr_backends().ok();
        let backend = create_mock_ocr_backend("test-backend").expect("Failed to create mock OCR backend");
        register_ocr_backend(backend).ok();

        let result = list_ocr_backends();

        assert!(result.is_ok());
        let arr = result.unwrap();
        assert!(arr.length() > 0);
    }

    #[wasm_bindgen_test]
    fn test_js_post_processor_wrapper_implements_plugin() {
        let processor = create_mock_processor("test").expect("Failed to create mock processor");
        let wrapper = JsPostProcessorWrapper::new(processor, "test".to_string(), ProcessingStage::Middle);

        assert_eq!(wrapper.name(), "test");
        assert_eq!(wrapper.version(), "1.0.0");
        assert!(wrapper.initialize().is_ok());
        assert!(wrapper.shutdown().is_ok());
    }

    #[wasm_bindgen_test]
    fn test_js_validator_wrapper_implements_plugin() {
        let validator = create_mock_validator("test").expect("Failed to create mock validator");
        let wrapper = JsValidatorWrapper::new(validator, "test".to_string(), 50);

        assert_eq!(wrapper.name(), "test");
        assert_eq!(wrapper.version(), "1.0.0");
        assert_eq!(wrapper.priority(), 50);
        assert!(wrapper.initialize().is_ok());
        assert!(wrapper.shutdown().is_ok());
    }

    #[wasm_bindgen_test]
    fn test_js_ocr_backend_wrapper_implements_plugin() {
        let backend = create_mock_ocr_backend("test").expect("Failed to create mock OCR backend");
        let wrapper = JsOcrBackendWrapper::new(backend, "test".to_string(), vec!["en".to_string()]);

        assert_eq!(wrapper.name(), "test");
        assert_eq!(wrapper.version(), "1.0.0");
        assert!(wrapper.initialize().is_ok());
        assert!(wrapper.shutdown().is_ok());
        assert_eq!(wrapper.supported_languages().len(), 1);
        assert!(wrapper.supports_language("en"));
        assert!(!wrapper.supports_language("fr"));
    }

    #[wasm_bindgen_test]
    fn test_register_multiple_post_processors() {
        clear_post_processors().ok();
        let p1 = create_mock_processor("proc1").expect("Failed to create mock processor 1");
        let p2 = create_mock_processor("proc2").expect("Failed to create mock processor 2");
        let p3 = create_mock_processor("proc3").expect("Failed to create mock processor 3");

        assert!(register_post_processor(p1).is_ok());
        assert!(register_post_processor(p2).is_ok());
        assert!(register_post_processor(p3).is_ok());

        let list = list_post_processors().unwrap();
        assert!(list.length() >= 3);
    }

    #[wasm_bindgen_test]
    fn test_register_multiple_validators() {
        clear_validators().ok();
        let v1 = create_mock_validator("val1").expect("Failed to create mock validator 1");
        let v2 = create_mock_validator("val2").expect("Failed to create mock validator 2");

        assert!(register_validator(v1).is_ok());
        assert!(register_validator(v2).is_ok());

        let list = list_validators().unwrap();
        assert!(list.length() >= 2);
    }

    #[wasm_bindgen_test]
    fn test_register_multiple_ocr_backends() {
        clear_ocr_backends().ok();
        let b1 = create_mock_ocr_backend("backend1").expect("Failed to create mock OCR backend 1");
        let b2 = create_mock_ocr_backend("backend2").expect("Failed to create mock OCR backend 2");

        assert!(register_ocr_backend(b1).is_ok());
        assert!(register_ocr_backend(b2).is_ok());

        let list = list_ocr_backends().unwrap();
        assert!(list.length() >= 2);
    }

    #[wasm_bindgen_test]
    fn test_processor_processing_stage_early() {
        let processor = create_mock_processor("test").expect("Failed to create mock processor");
        let wrapper = JsPostProcessorWrapper::new(processor, "test".to_string(), ProcessingStage::Early);

        assert_eq!(wrapper.processing_stage(), ProcessingStage::Early);
    }

    #[wasm_bindgen_test]
    fn test_processor_processing_stage_late() {
        let processor = create_mock_processor("test").expect("Failed to create mock processor");
        let wrapper = JsPostProcessorWrapper::new(processor, "test".to_string(), ProcessingStage::Late);

        assert_eq!(wrapper.processing_stage(), ProcessingStage::Late);
    }
}
