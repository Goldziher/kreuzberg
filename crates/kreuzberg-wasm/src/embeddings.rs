//! Embedding preset utilities for WASM bindings
//!
//! This module provides functions for accessing and managing text embedding presets
//! in WebAssembly environments. Presets provide pre-configured models optimized for
//! different use cases (fast, balanced, quality, multilingual).

#[cfg(feature = "embeddings")]
use js_sys::Array;
#[cfg(feature = "embeddings")]
use wasm_bindgen::prelude::*;

/// List all available embedding preset names.
///
/// Returns a JavaScript Array of all available preset names. Each preset provides
/// a pre-configured model optimized for specific use cases. Current presets include:
/// - "fast": Quick prototyping with quantized models
/// - "balanced": Production-ready general-purpose embeddings
/// - "quality": High-quality embeddings with larger models
/// - "multilingual": Support for multiple languages
///
/// # JavaScript Parameters
///
/// None
///
/// # Returns
///
/// `string[]` - Array of available preset names
///
/// # Example
///
/// ```javascript
/// import { listEmbeddingPresets } from '@kreuzberg/wasm';
///
/// const presets = listEmbeddingPresets();
/// console.log(presets); // ["fast", "balanced", "quality", "multilingual"]
///
/// for (const preset of presets) {
///   console.log(`Available preset: ${preset}`);
/// }
/// ```
#[cfg(feature = "embeddings")]
#[wasm_bindgen(js_name = listEmbeddingPresets)]
pub fn list_embedding_presets() -> Array {
    let presets = kreuzberg::list_presets();
    let array = Array::new();

    for preset_name in presets {
        array.push(&JsValue::from_str(preset_name));
    }

    array
}

/// Get details about a specific embedding preset.
///
/// Retrieves configuration details for a named embedding preset, including
/// the model information, chunk size, overlap, dimensions, and description.
///
/// Returns None if the preset name is not found.
///
/// # JavaScript Parameters
///
/// * `name: string` - The name of the preset (e.g., "balanced", "fast")
///
/// # Returns
///
/// `object | null` - Preset object with properties or null if not found
///
/// The returned object has the following properties:
/// - `name: string` - Preset name
/// - `chunkSize: number` - Recommended chunk size in characters
/// - `overlap: number` - Recommended overlap between chunks in characters
/// - `dimensions: number` - Embedding vector dimensions
/// - `modelName: string` - Name of the embedding model
/// - `description: string` - Human-readable description of the preset
///
/// # Throws
///
/// Does not throw; returns null for unknown presets.
///
/// # Example
///
/// ```javascript
/// import { getEmbeddingPreset } from '@kreuzberg/wasm';
///
/// // Get balanced preset (general-purpose)
/// const balanced = getEmbeddingPreset('balanced');
/// if (balanced) {
///   console.log(balanced.name);        // "balanced"
///   console.log(balanced.dimensions);  // 768
///   console.log(balanced.chunkSize);   // 1024
///   console.log(balanced.overlap);     // 100
///   console.log(balanced.description); // "Balanced quality and speed..."
/// }
///
/// // Get fast preset (development)
/// const fast = getEmbeddingPreset('fast');
/// if (fast) {
///   console.log(fast.chunkSize);  // 512
///   console.log(fast.dimensions); // 384
/// }
///
/// // Preset not found
/// const unknown = getEmbeddingPreset('nonexistent');
/// console.log(unknown); // null
/// ```
#[cfg(feature = "embeddings")]
#[wasm_bindgen(js_name = getEmbeddingPreset)]
pub fn get_embedding_preset(name: String) -> Option<JsValue> {
    let preset = kreuzberg::get_preset(&name)?;

    // Create a JavaScript object with preset information
    let obj = js_sys::Object::new();

    js_sys::Reflect::set(&obj, &"name".into(), &preset.name.into()).ok()?;
    js_sys::Reflect::set(&obj, &"chunkSize".into(), &preset.chunk_size.into()).ok()?;
    js_sys::Reflect::set(&obj, &"overlap".into(), &preset.overlap.into()).ok()?;
    js_sys::Reflect::set(&obj, &"dimensions".into(), &preset.dimensions.into()).ok()?;

    // Set model name (format the model enum as a string)
    let model_name = format!("{:?}", preset.model);
    js_sys::Reflect::set(&obj, &"modelName".into(), &model_name.into()).ok()?;

    js_sys::Reflect::set(&obj, &"description".into(), &preset.description.into()).ok()?;

    Some(obj.into())
}

#[cfg(all(test, feature = "embeddings"))]
mod tests {
    use super::*;

    #[test]
    fn test_list_embedding_presets() {
        let presets = list_embedding_presets();
        assert!(presets.length() > 0);
    }

    #[test]
    fn test_get_embedding_preset_valid() {
        let preset = get_embedding_preset("balanced".to_string());
        assert!(preset.is_some());
    }

    #[test]
    fn test_get_embedding_preset_invalid() {
        let preset = get_embedding_preset("nonexistent".to_string());
        assert!(preset.is_none());
    }
}
