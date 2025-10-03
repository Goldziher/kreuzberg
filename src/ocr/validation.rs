//! Validation functions for Tesseract configuration

use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::process::Command;
use std::sync::Mutex;

const MINIMAL_SUPPORTED_TESSERACT_VERSION: u32 = 5;

/// Supported Tesseract language codes (177 languages)
static TESSERACT_SUPPORTED_LANGUAGE_CODES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "afr", "amh", "ara", "asm", "aze", "aze_cyrl", "bel", "ben", "bod", "bos", "bre", "bul", "cat", "ceb", "ces",
        "chi_sim", "chi_tra", "chr", "cos", "cym", "dan", "dan_frak", "deu", "deu_frak", "deu_latf", "dzo", "ell",
        "eng", "enm", "epo", "equ", "est", "eus", "fao", "fas", "fil", "fin", "fra", "frk", "frm", "fry", "gla", "gle",
        "glg", "grc", "guj", "hat", "heb", "hin", "hrv", "hun", "hye", "iku", "ind", "isl", "ita", "ita_old", "jav",
        "jpn", "kan", "kat", "kat_old", "kaz", "khm", "kir", "kmr", "kor", "kor_vert", "kur", "lao", "lat", "lav",
        "lit", "ltz", "mal", "mar", "mkd", "mlt", "mon", "mri", "msa", "mya", "nep", "nld", "nor", "oci", "ori", "osd",
        "pan", "pol", "por", "pus", "que", "ron", "rus", "san", "sin", "slk", "slk_frak", "slv", "snd", "spa",
        "spa_old", "sqi", "srp", "srp_latn", "sun", "swa", "swe", "syr", "tam", "tat", "tel", "tgk", "tgl", "tha",
        "tir", "ton", "tur", "uig", "ukr", "urd", "uzb", "uzb_cyrl", "vie", "yid", "yor",
    ]
    .into_iter()
    .collect()
});

/// Cached Tesseract version check result
static VERSION_CHECKED: Lazy<Mutex<Option<bool>>> = Lazy::new(|| Mutex::new(None));

/// Validate Tesseract language code(s)
///
/// Supports single language codes (e.g., "eng") and multi-language codes (e.g., "eng+deu")
/// Returns the lowercase normalized version of the language code
#[pyo3::pyfunction]
pub fn validate_language_code(lang: &str) -> pyo3::PyResult<String> {
    if lang.is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err("Language code cannot be empty"));
    }

    // Handle multi-language codes (e.g., "eng+deu")
    let codes: Vec<&str> = lang.split('+').collect();
    let mut normalized_codes = Vec::new();

    for code in &codes {
        let normalized = code.trim().to_lowercase();

        if !TESSERACT_SUPPORTED_LANGUAGE_CODES.contains(normalized.as_str()) {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Language code '{}' is not supported by Tesseract",
                code
            )));
        }

        normalized_codes.push(normalized);
    }

    // Return lowercase normalized version
    Ok(normalized_codes.join("+"))
}

/// Validate Tesseract version
///
/// Checks that Tesseract version is >= 5. Result is cached after first check.
#[pyo3::pyfunction]
pub fn validate_tesseract_version() -> pyo3::PyResult<()> {
    // Check cache first
    {
        let checked = VERSION_CHECKED.lock().unwrap();
        if let Some(is_valid) = *checked {
            return if is_valid {
                Ok(())
            } else {
                Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Tesseract version unknown is not supported. Minimum version is {}",
                    MINIMAL_SUPPORTED_TESSERACT_VERSION
                )))
            };
        }
    }

    // Run tesseract --version
    let output = Command::new("tesseract")
        .arg("--version")
        .output()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to execute tesseract: {}", e)))?;

    let version_output = String::from_utf8_lossy(&output.stdout);
    let version_str = version_output
        .lines()
        .next()
        .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("Could not read tesseract version"))?;

    // Parse version number (format: "tesseract 5.3.0")
    let version = version_str
        .split_whitespace()
        .nth(1)
        .and_then(|v| v.split('.').next())
        .and_then(|v| v.parse::<u32>().ok())
        .ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Could not parse tesseract version: {}", version_str))
        })?;

    // Validate version
    let is_valid = version >= MINIMAL_SUPPORTED_TESSERACT_VERSION;

    // Cache result
    {
        let mut checked = VERSION_CHECKED.lock().unwrap();
        *checked = Some(is_valid);
    }

    if is_valid {
        Ok(())
    } else {
        Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
            "Tesseract version {} is not supported. Minimum version is {}",
            version, MINIMAL_SUPPORTED_TESSERACT_VERSION
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_language_code_single() {
        assert!(validate_language_code("eng").is_ok());
        assert!(validate_language_code("deu").is_ok());
        assert!(validate_language_code("fra").is_ok());
    }

    #[test]
    fn test_validate_language_code_multi() {
        assert!(validate_language_code("eng+deu").is_ok());
        assert!(validate_language_code("eng+deu+fra").is_ok());
    }

    #[test]
    fn test_validate_language_code_invalid() {
        assert!(validate_language_code("invalid_lang").is_err());
        assert!(validate_language_code("").is_err());
    }

    #[test]
    fn test_validate_language_code_case_insensitive() {
        assert!(validate_language_code("ENG").is_ok());
        assert!(validate_language_code("Eng").is_ok());
    }

    #[test]
    fn test_validate_tesseract_version() {
        // This test requires tesseract to be installed
        // We can't test the actual version without tesseract installed
        let _result = validate_tesseract_version();
        // Just ensure it doesn't panic
    }
}
