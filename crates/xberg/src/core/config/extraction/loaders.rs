//! Configuration file loading.
//!
//! This module provides methods for loading extraction configuration from
//! TOML, YAML, and JSON files.
//!
//! Loading here is entirely generic (`toml`/`yaml_serde`/`serde_json` deserializing
//! straight into `ExtractionConfig`). New `#[serde(default)]` fields on nested config
//! types are picked up automatically with no change required in this file.

use crate::{Result, XbergError};
use std::path::Path;

use super::core::ExtractionConfig;

impl ExtractionConfig {
    /// Load configuration from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns `XbergError::Validation` if file doesn't exist or is invalid TOML.
    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| XbergError::validation(format!("Failed to read config file {}: {}", path.display(), e)))?;
        let config: Self = toml::from_str(&content)
            .map_err(|e| XbergError::validation(format!("Invalid TOML in {}: {}", path.display(), e)))?;
        config.validate()?;
        Ok(config)
    }

    /// Load configuration from a YAML file.
    pub fn from_yaml_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| XbergError::validation(format!("Failed to read config file {}: {}", path.display(), e)))?;
        let config: Self = yaml_serde::from_str(&content)
            .map_err(|e| XbergError::validation(format!("Invalid YAML in {}: {}", path.display(), e)))?;
        config.validate()?;
        Ok(config)
    }

    /// Load configuration from a JSON file.
    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| XbergError::validation(format!("Failed to read config file {}: {}", path.display(), e)))?;
        let config: Self = serde_json::from_str(&content)
            .map_err(|e| XbergError::validation(format!("Invalid JSON in {}: {}", path.display(), e)))?;
        config.validate()?;
        Ok(config)
    }

    /// Load configuration from a file, auto-detecting format by extension.
    ///
    /// Supported formats: `.toml`, `.yaml`, `.yml`, `.json`.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let extension = path.extension().and_then(|ext| ext.to_str()).ok_or_else(|| {
            XbergError::validation(format!(
                "Cannot determine file format: no extension found in {}",
                path.display()
            ))
        })?;

        match extension.to_lowercase().as_str() {
            "toml" => Self::from_toml_file(path),
            "yaml" | "yml" => Self::from_yaml_file(path),
            "json" => Self::from_json_file(path),
            other => Err(XbergError::validation(format!(
                "Unsupported config file format: .{}. Supported formats: .toml, .yaml, .json",
                other
            ))),
        }
    }

    /// Discover configuration file.
    ///
    /// Searches for `xberg.{toml,yaml,yml,json}` in the current directory and its
    /// parents. If no project-local config is found, falls back to a per-user global config in
    /// the platform config directory: `xberg/xberg.{toml,yaml,yml,json}` under
    /// `dirs::config_dir()` — i.e. `$XDG_CONFIG_HOME` (or `~/.config`) on Linux,
    /// `~/Library/Application Support` on macOS, `%APPDATA%` on Windows.
    pub fn discover() -> Result<Option<Self>> {
        let current = std::env::current_dir().map_err(crate::XbergError::from)?;
        if let Some(config) = Self::find_config_in_ancestors(&current)? {
            return Ok(Some(config));
        }

        if let Some(config_dir) = dirs::config_dir()
            && let Some(config) = Self::find_config_in_dir(&config_dir.join("xberg"))?
        {
            return Ok(Some(config));
        }

        Ok(None)
    }

    fn find_config_in_ancestors(start: &Path) -> Result<Option<Self>> {
        for directory in start.ancestors() {
            if let Some(config) = Self::find_config_in_dir(directory)? {
                return Ok(Some(config));
            }
        }
        Ok(None)
    }

    /// Load the first `xberg.{toml,yaml,yml,json}` present in `dir`, if any.
    ///
    /// Extensions are probed in a fixed order so discovery is deterministic when
    /// multiple config files coexist in the same directory.
    fn find_config_in_dir(dir: &Path) -> Result<Option<Self>> {
        const CONFIG_BASENAMES: [&str; 4] = ["xberg.toml", "xberg.yaml", "xberg.yml", "xberg.json"];

        for basename in CONFIG_BASENAMES {
            let candidate = dir.join(basename);
            if candidate.exists() {
                return Ok(Some(Self::from_file(candidate)?));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_config_in_ancestors_loads_each_supported_format() {
        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("project/src");
        std::fs::create_dir_all(&nested).unwrap();

        for (basename, contents) in [
            ("xberg.toml", "include_document_structure = true\n"),
            ("xberg.yaml", "include_document_structure: true\n"),
            ("xberg.yml", "include_document_structure: true\n"),
            ("xberg.json", "{\"include_document_structure\":true}\n"),
        ] {
            let path = root.path().join(basename);
            std::fs::write(&path, contents).unwrap();
            let discovered = ExtractionConfig::find_config_in_ancestors(&nested)
                .unwrap()
                .unwrap_or_else(|| panic!("{basename} must be discovered in a project ancestor"));
            assert!(
                discovered.include_document_structure,
                "{basename} must be parsed rather than replaced with the default config"
            );
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn find_config_in_ancestors_prefers_nearest_directory() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        let nested = project.join("src");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(root.path().join("xberg.toml"), "not valid toml = [").unwrap();
        std::fs::write(project.join("xberg.json"), "{}").unwrap();

        assert!(
            ExtractionConfig::find_config_in_ancestors(&nested).unwrap().is_some(),
            "the nearest ancestor config must win before a higher TOML config"
        );
    }

    #[test]
    fn find_config_in_dir_returns_none_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let found = ExtractionConfig::find_config_in_dir(dir.path()).unwrap();
        assert!(found.is_none(), "empty dir must yield no config");
    }

    #[test]
    fn find_config_in_dir_loads_yaml_and_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("xberg.json"), "{}").unwrap();
        assert!(
            ExtractionConfig::find_config_in_dir(dir.path()).unwrap().is_some(),
            "xberg.json must be discovered"
        );

        std::fs::remove_file(dir.path().join("xberg.json")).unwrap();
        std::fs::write(dir.path().join("xberg.yaml"), "use_cache: true\n").unwrap();
        assert!(
            ExtractionConfig::find_config_in_dir(dir.path()).unwrap().is_some(),
            "xberg.yaml must be discovered"
        );
    }

    #[test]
    fn find_config_in_dir_prefers_toml_over_other_formats() {
        let dir = tempfile::tempdir().unwrap();
        // A valid TOML file and a deliberately invalid JSON file coexist. TOML is
        // probed first, so discovery must succeed without touching the JSON. ~keep
        std::fs::write(dir.path().join("xberg.toml"), "use_cache = true\n").unwrap();
        std::fs::write(dir.path().join("xberg.json"), "not valid json").unwrap();

        let found = ExtractionConfig::find_config_in_dir(dir.path()).unwrap();
        assert!(found.is_some(), "xberg.toml must win over xberg.json");
    }

    #[test]
    fn from_toml_file_rejects_unknown_nested_fields() {
        let cases = [
            (
                "max_archive_bytes",
                "[security_limits]\nmax_archive_bytes = 1024\n",
                true,
            ),
            ("backnd", "[ocr]\nbacknd = \"tesseract\"\n", true),
            (
                "psmm",
                "[ocr]\nbackend = \"tesseract\"\n[ocr.tesseract_config]\npsmm = 6\n",
                true,
            ),
            (
                "deskww",
                "[ocr]\nbackend = \"tesseract\"\n[ocr.tesseract_config.preprocessing]\ndeskww = true\n",
                true,
            ),
            (
                "min_confidence",
                "[ocr_strategy]\nmode = \"auto\"\nmin_confidence = 0.95\n",
                true,
            ),
            (
                "quality_threshold",
                "[ocr]\n[ocr.vlm_fallback]\nmode = \"disabled\"\nquality_threshold = 0.8\n",
                true,
            ),
            (
                "pdf_backend",
                "[pdf_options]\npdf_backend = \"native\"\n",
                cfg!(feature = "pdf"),
            ),
        ];

        for (unknown_field, source, enabled) in cases {
            if !enabled {
                continue;
            }
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("xberg.toml");
            std::fs::write(&path, source).unwrap();

            let error = ExtractionConfig::from_toml_file(&path)
                .expect_err("an unknown nested field must make config loading fail");
            let message = error.to_string();
            assert!(
                message.contains("Invalid TOML"),
                "wrong error for {unknown_field}: {message}"
            );
            assert!(
                message.contains(unknown_field),
                "error must name {unknown_field}: {message}"
            );
        }
    }
}
