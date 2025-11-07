//! Structured extraction orchestrator binding
//!
//! Provides an async Python-facing function that:
//! - Calls a user-supplied Python async/sync callable to obtain JSON bytes
//! - Validates the JSON against a provided JSON Schema
//! - Retries with appended validation feedback when configured

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyModule, PyString};

/// Extract structured data using a user-supplied Python callable and validate against JSON Schema.
///
/// Args:
/// - images: List of image bytes to provide to the extractor
/// - prompt: Instructional prompt to guide the extractor
/// - extractor: Python callable taking `(images: list[bytes], prompt: str)` and returning `bytes | str` (JSON)
/// - schema_json: JSON Schema (string) to validate the extractor's output
/// - max_retries: Number of validation retries to attempt (default: 2)
/// - include_error_in_retry: Whether to append validation errors to the prompt for retries (default: True)
///
/// Returns:
/// - Awaitable that resolves to `bytes` containing valid JSON matching the schema
///
/// Raises:
/// - ValidationError: If `schema_json` is invalid
/// - ExtractionValidationError: If output fails schema validation after all retries
#[pyfunction]
#[pyo3(signature = (images, prompt, extractor, schema_json, max_retries=2, include_error_in_retry=true))]
pub fn extract_structured_json<'py>(
    py: Python<'py>,
    images: Vec<Vec<u8>>,
    prompt: String,
    extractor: Py<PyAny>,
    schema_json: String,
    max_retries: usize,
    include_error_in_retry: bool,
) -> PyResult<Bound<'py, PyAny>> {
    // Parse and compile JSON Schema upfront
    let schema_value: serde_json::Value = serde_json::from_str(&schema_json).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!(
            "Invalid schema_json (must be valid JSON Schema): {}",
            e
        ))
    })?;

    let compiled = jsonschema::JSONSchema::compile(&schema_value).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!(
            "Failed to compile JSON Schema: {}",
            e
        ))
    })?;

    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let mut current_prompt = prompt.clone();
        let mut last_error_messages: Vec<String> = Vec::new();

        for attempt in 0..=max_retries {
            // Clone for move into blocking task
            let images_clone = images.clone();
            let extractor_obj = Python::attach(|py| extractor.clone_ref(py));
            let prompt_for_call = current_prompt.clone();

            let call_res = tokio::task::spawn_blocking(move || {
                Python::attach(|py| {
                    let obj = extractor_obj.bind(py);

                    // Build Python list of bytes for images
                    let py_list = PyList::empty(py);
                    for img in images_clone.iter() {
                        let py_bytes = PyBytes::new(py, img);
                        py_list.append(py_bytes)?;
                    }

                    // Call extractor(images, prompt)
                    let py_res = obj.call1((py_list, prompt_for_call))?;

                    // Normalize return to bytes
                    if let Ok(py_bytes) = py_res.extract::<&PyBytes>() {
                        Ok(py_bytes.as_bytes().to_vec())
                    } else if let Ok(py_str) = py_res.downcast::<PyString>() {
                        Ok(py_str.to_str()?.as_bytes().to_vec())
                    } else {
                        Err(pyo3::exceptions::PyTypeError::new_err(
                            "Extractor must return bytes or str containing JSON",
                        ))
                    }
                })
            })
            .await
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Extractor task failed to run: {:?}",
                    e
                ))
            })??;

            // Validate JSON output against schema
            match serde_json::from_slice::<serde_json::Value>(&call_res) {
                Ok(value) => {
                    let result = compiled.validate(&value);
                    if result.is_ok() {
                        return Python::attach(|py| {
                            Ok(PyBytes::new_bound(py, &call_res).into_any())
                        });
                    } else {
                        last_error_messages.clear();
                        for err in result.err().unwrap() {
                            last_error_messages.push(format!("{}", err));
                        }
                    }
                }
                Err(err) => {
                    last_error_messages.clear();
                    last_error_messages.push(format!("Invalid JSON output: {}", err));
                }
            }

            // Prepare retry with feedback
            if attempt < max_retries && include_error_in_retry {
                let feedback = format!(
                    "\n\n[System] The previous JSON did not match the required schema.\nErrors:\n- {}\nPlease regenerate JSON strictly matching the schema.",
                    last_error_messages.join("\n- ")
                );
                current_prompt.push_str(&feedback);
            }
        }

        // Exhausted retries: raise ExtractionValidationError with context
        Python::attach(|py| {
            let exc_mod = PyModule::import_bound(py, "kreuzberg.exceptions")?;
            let klass = exc_mod.getattr("ExtractionValidationError")?;
            let kwargs = PyDict::new(py);

            let errs_list = PyList::empty(py);
            for m in &last_error_messages {
                errs_list.append(m)?;
            }

            let context = PyDict::new(py);
            context.set_item("attempts", max_retries + 1)?;
            context.set_item("errors", errs_list)?;
            kwargs.set_item("context", context)?;

            let msg = format!(
                "Structured extraction failed schema validation after {} attempt(s)",
                max_retries + 1
            );
            let instance = klass.call((msg,), Some(&kwargs))?;
            Err(PyErr::from_instance(instance))
        })
    })
}