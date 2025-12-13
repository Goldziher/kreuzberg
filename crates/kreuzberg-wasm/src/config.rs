//! Configuration utilities for WASM bindings
//!
//! This module provides functions for loading and parsing extraction configuration
//! from various formats (TOML, YAML, JSON) in WebAssembly environments.
//! Note: Configuration discovery (file system access) is not available in WASM.

use kreuzberg::ExtractionConfig;
use wasm_bindgen::prelude::*;

/// Load configuration from a string in the specified format.
///
/// Parses configuration content from TOML, YAML, or JSON formats and returns
/// a JavaScript object representing the ExtractionConfig. This is the primary
/// way to load configuration in WebAssembly environments since file system
/// access is not available.
///
/// # JavaScript Parameters
///
/// * `content: string` - The configuration content as a string
/// * `format: string` - The format of the content: "toml", "yaml", or "json"
///
/// # Returns
///
/// `object` - JavaScript object representing the ExtractionConfig
///
/// # Throws
///
/// Throws an error if:
/// - The content is invalid for the specified format
/// - The format is not one of "toml", "yaml", or "json"
/// - Required configuration fields are missing or invalid
///
/// # Example
///
/// ```javascript
/// import { loadConfigFromString } from '@kreuzberg/wasm';
///
/// // Load from TOML string
/// const tomlConfig = `
/// use_cache = true
/// enable_quality_processing = true
/// `;
/// const config1 = loadConfigFromString(tomlConfig, 'toml');
/// console.log(config1.use_cache); // true
///
/// // Load from YAML string
/// const yamlConfig = `
/// use_cache: true
/// enable_quality_processing: true
/// `;
/// const config2 = loadConfigFromString(yamlConfig, 'yaml');
///
/// // Load from JSON string
/// const jsonConfig = `{"use_cache": true, "enable_quality_processing": true}`;
/// const config3 = loadConfigFromString(jsonConfig, 'json');
/// ```
#[wasm_bindgen(js_name = loadConfigFromString)]
pub fn load_config_from_string(content: String, format: String) -> Result<JsValue, JsValue> {
    let format_lower = format.to_lowercase();

    let config: ExtractionConfig = match format_lower.as_str() {
        "toml" => toml::from_str(&content).map_err(|e| JsValue::from_str(&format!("Failed to parse TOML: {}", e)))?,

        "yaml" | "yml" => {
            serde_yaml::from_str(&content).map_err(|e| JsValue::from_str(&format!("Failed to parse YAML: {}", e)))?
        }

        "json" => {
            serde_json::from_str(&content).map_err(|e| JsValue::from_str(&format!("Failed to parse JSON: {}", e)))?
        }

        _ => return Err(JsValue::from_str("Unsupported format. Use 'toml', 'yaml', or 'json'.")),
    };

    // Serialize the config back to JsValue
    serde_wasm_bindgen::to_value(&config)
        .map_err(|e| JsValue::from_str(&format!("Failed to convert config to JS value: {}", e)))
}

/// Discover configuration file in the project hierarchy.
///
/// In WebAssembly environments, configuration discovery is not available because
/// there is no file system access. This function always returns an error with a
/// descriptive message directing users to use `loadConfigFromString()` instead.
///
/// # JavaScript Parameters
///
/// None
///
/// # Returns
///
/// Never returns successfully.
///
/// # Throws
///
/// Always throws an error with message:
/// "discoverConfig is not available in WebAssembly (no file system access). Use loadConfigFromString() instead."
///
/// # Example
///
/// ```javascript
/// import { discoverConfig } from '@kreuzberg/wasm';
///
/// try {
///   const config = discoverConfig();
/// } catch (e) {
///   console.error(e.message);
///   // "discoverConfig is not available in WebAssembly (no file system access).
///   // Use loadConfigFromString() instead."
/// }
/// ```
#[wasm_bindgen(js_name = discoverConfig)]
pub fn discover_config() -> Result<JsValue, JsValue> {
    Err(JsValue::from_str(
        "discoverConfig is not available in WebAssembly (no file system access). Use loadConfigFromString() instead.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config_from_json_string() {
        let json_config = r#"{"use_cache": true, "enable_quality_processing": true}"#;
        let result = load_config_from_string(json_config.to_string(), "json".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_config_from_toml_string() {
        let toml_config = r#"use_cache = true
enable_quality_processing = true"#;
        let result = load_config_from_string(toml_config.to_string(), "toml".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_config_from_yaml_string() {
        let yaml_config = r#"use_cache: true
enable_quality_processing: true"#;
        let result = load_config_from_string(yaml_config.to_string(), "yaml".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_config_invalid_json() {
        let invalid_json = "{ invalid json }";
        let result = load_config_from_string(invalid_json.to_string(), "json".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_unsupported_format() {
        let config = "use_cache = true";
        let result = load_config_from_string(config.to_string(), "xml".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_discover_config_not_available() {
        let result = discover_config();
        assert!(result.is_err());
    }
}
