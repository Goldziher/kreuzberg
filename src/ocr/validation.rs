use std::collections::HashSet;
use std::process::Command;
use std::sync::Mutex;

use once_cell::sync::Lazy;

use super::utils::MINIMAL_SUPPORTED_TESSERACT_VERSION;

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

static VERSION_CHECKED: Lazy<Mutex<Option<bool>>> = Lazy::new(|| Mutex::new(None));

#[pyo3::pyfunction]
pub fn validate_language_code(lang: &str) -> pyo3::PyResult<String> {
    if lang.is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err("Language code cannot be empty"));
    }

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

    Ok(normalized_codes.join("+"))
}

#[pyo3::pyfunction]
pub fn validate_tesseract_version() -> pyo3::PyResult<()> {
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

    let output = Command::new("tesseract")
        .arg("--version")
        .output()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to execute tesseract: {}", e)))?;

    let version_output = String::from_utf8_lossy(&output.stdout);
    let version_str = version_output
        .lines()
        .next()
        .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("Could not read tesseract version"))?;

    let version = version_str
        .split_whitespace()
        .nth(1)
        .and_then(|v| v.split('.').next())
        .and_then(|v| v.parse::<u32>().ok())
        .ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Could not parse tesseract version: {}", version_str))
        })?;

    let is_valid = version >= MINIMAL_SUPPORTED_TESSERACT_VERSION;

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
        let _result = validate_tesseract_version();
    }

    #[test]
    fn test_validate_language_code_multi_with_spaces() {
        let result = validate_language_code("eng + deu").unwrap();
        assert_eq!(result, "eng+deu");
    }

    #[test]
    fn test_validate_language_code_multi_invalid_one() {
        let result = validate_language_code("eng+invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not supported"));
    }

    #[test]
    fn test_validate_language_code_normalization() {
        let result = validate_language_code("ENG+DEU+FRA").unwrap();
        assert_eq!(result, "eng+deu+fra");
    }

    #[test]
    fn test_validate_language_code_all_supported() {
        let test_langs = [
            "eng", "deu", "fra", "spa", "ita", "por", "jpn", "chi_sim", "chi_tra", "ara",
        ];
        for lang in &test_langs {
            assert!(
                validate_language_code(lang).is_ok(),
                "Language {} should be valid",
                lang
            );
        }
    }

    #[test]
    fn test_validate_language_code_edge_cases() {
        assert!(validate_language_code("osd").is_ok());
        assert!(validate_language_code("equ").is_ok());
        assert!(validate_language_code("aze_cyrl").is_ok());
        assert!(validate_language_code("chi_sim+chi_tra").is_ok());
    }

    #[test]
    fn test_validate_language_code_invalid_multi() {
        assert!(validate_language_code("eng+xyz+deu").is_err());
        assert!(validate_language_code("xxx+yyy").is_err());
    }

    #[test]
    fn test_language_code_constants() {
        assert!(TESSERACT_SUPPORTED_LANGUAGE_CODES.contains("eng"));
        assert!(TESSERACT_SUPPORTED_LANGUAGE_CODES.contains("jpn"));
        assert!(!TESSERACT_SUPPORTED_LANGUAGE_CODES.contains("invalid"));
        assert!(TESSERACT_SUPPORTED_LANGUAGE_CODES.len() > 100);
    }

    #[test]
    fn test_validate_language_code_trimming() {
        let result = validate_language_code("  eng  ").unwrap();
        assert_eq!(result, "eng");

        let result = validate_language_code("eng + deu + fra").unwrap();
        assert_eq!(result, "eng+deu+fra");
    }
}
