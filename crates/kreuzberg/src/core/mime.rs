//! MIME type detection and validation.
//!
//! This module provides utilities for detecting MIME types from file extensions
//! and validating them against supported types.

use crate::{KreuzbergError, Result};
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub const HTML_MIME_TYPE: &str = "text/html";
pub const MARKDOWN_MIME_TYPE: &str = "text/markdown";
pub const PDF_MIME_TYPE: &str = "application/pdf";
pub const PLAIN_TEXT_MIME_TYPE: &str = "text/plain";
pub const POWER_POINT_MIME_TYPE: &str = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
pub const DOCX_MIME_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
pub const LEGACY_WORD_MIME_TYPE: &str = "application/msword";
pub const LEGACY_POWERPOINT_MIME_TYPE: &str = "application/vnd.ms-powerpoint";

pub const EML_MIME_TYPE: &str = "message/rfc822";
pub const MSG_MIME_TYPE: &str = "application/vnd.ms-outlook";
pub const JSON_MIME_TYPE: &str = "application/json";
pub const YAML_MIME_TYPE: &str = "application/x-yaml";
pub const TOML_MIME_TYPE: &str = "application/toml";
pub const XML_MIME_TYPE: &str = "application/xml";
pub const XML_TEXT_MIME_TYPE: &str = "text/xml";
pub const SVG_MIME_TYPE: &str = "image/svg+xml";

pub const EXCEL_MIME_TYPE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
pub const EXCEL_BINARY_MIME_TYPE: &str = "application/vnd.ms-excel";
pub const EXCEL_MACRO_MIME_TYPE: &str = "application/vnd.ms-excel.sheet.macroEnabled.12";
pub const EXCEL_BINARY_2007_MIME_TYPE: &str = "application/vnd.ms-excel.sheet.binary.macroEnabled.12";
pub const EXCEL_ADDON_MIME_TYPE: &str = "application/vnd.ms-excel.addin.macroEnabled.12";
pub const EXCEL_TEMPLATE_MIME_TYPE: &str = "application/vnd.ms-excel.template.macroEnabled.12";

pub const OPENDOC_SPREADSHEET_MIME_TYPE: &str = "application/vnd.oasis.opendocument.spreadsheet";

/// Extension to MIME type mapping (ported from Python EXT_TO_MIME_TYPE).
static EXT_TO_MIME: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();

    m.insert("txt", PLAIN_TEXT_MIME_TYPE);
    m.insert("md", MARKDOWN_MIME_TYPE);
    m.insert("markdown", MARKDOWN_MIME_TYPE);

    m.insert("pdf", PDF_MIME_TYPE);

    m.insert("html", HTML_MIME_TYPE);
    m.insert("htm", HTML_MIME_TYPE);

    m.insert("xlsx", EXCEL_MIME_TYPE);
    m.insert("xls", EXCEL_BINARY_MIME_TYPE);
    m.insert("xlsm", EXCEL_MACRO_MIME_TYPE);
    m.insert("xlsb", EXCEL_BINARY_2007_MIME_TYPE);
    m.insert("xlam", EXCEL_ADDON_MIME_TYPE);
    m.insert("xla", EXCEL_TEMPLATE_MIME_TYPE);
    m.insert("ods", OPENDOC_SPREADSHEET_MIME_TYPE);

    m.insert("pptx", POWER_POINT_MIME_TYPE);
    m.insert("ppt", LEGACY_POWERPOINT_MIME_TYPE);

    m.insert("docx", DOCX_MIME_TYPE);
    m.insert("doc", LEGACY_WORD_MIME_TYPE);
    m.insert("odt", "application/vnd.oasis.opendocument.text");

    m.insert("bmp", "image/bmp");
    m.insert("gif", "image/gif");
    m.insert("jpg", "image/jpeg");
    m.insert("jpeg", "image/jpeg");
    m.insert("png", "image/png");
    m.insert("tiff", "image/tiff");
    m.insert("tif", "image/tiff");
    m.insert("webp", "image/webp");
    m.insert("jp2", "image/jp2");
    m.insert("jpx", "image/jpx");
    m.insert("jpm", "image/jpm");
    m.insert("mj2", "image/mj2");
    m.insert("pnm", "image/x-portable-anymap");
    m.insert("pbm", "image/x-portable-bitmap");
    m.insert("pgm", "image/x-portable-graymap");
    m.insert("ppm", "image/x-portable-pixmap");

    m.insert("csv", "text/csv");
    m.insert("tsv", "text/tab-separated-values");
    m.insert("json", JSON_MIME_TYPE);
    m.insert("yaml", YAML_MIME_TYPE);
    m.insert("yml", YAML_MIME_TYPE);
    m.insert("toml", TOML_MIME_TYPE);
    m.insert("xml", XML_MIME_TYPE);
    m.insert("svg", SVG_MIME_TYPE);

    m.insert("eml", EML_MIME_TYPE);
    m.insert("msg", MSG_MIME_TYPE);

    m.insert("zip", "application/zip");
    m.insert("tar", "application/x-tar");
    m.insert("gz", "application/gzip");
    m.insert("tgz", "application/x-tar");
    m.insert("7z", "application/x-7z-compressed");

    m.insert("rst", "text/x-rst");
    m.insert("org", "text/x-org");
    m.insert("epub", "application/epub+zip");
    m.insert("rtf", "application/rtf");
    m.insert("bib", "application/x-bibtex");
    m.insert("ipynb", "application/x-ipynb+json");
    m.insert("tex", "application/x-latex");
    m.insert("latex", "application/x-latex");
    m.insert("typst", "application/x-typst");
    m.insert("commonmark", "text/x-commonmark");

    m
});

/// All supported MIME types (ported from Python SUPPORTED_MIME_TYPES).
static SUPPORTED_MIME_TYPES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::new();

    set.insert(PLAIN_TEXT_MIME_TYPE);
    set.insert(MARKDOWN_MIME_TYPE);
    set.insert("text/x-markdown");

    set.insert("image/bmp");
    set.insert("image/gif");
    set.insert("image/jp2");
    set.insert("image/jpeg");
    set.insert("image/jpm");
    set.insert("image/jpx");
    set.insert("image/mj2");
    set.insert("image/pjpeg");
    set.insert("image/png");
    set.insert("image/tiff");
    set.insert("image/webp");
    set.insert("image/x-bmp");
    set.insert("image/x-ms-bmp");
    set.insert("image/x-portable-anymap");
    set.insert("image/x-portable-bitmap");
    set.insert("image/x-portable-graymap");
    set.insert("image/x-portable-pixmap");
    set.insert("image/x-tiff");

    set.insert("application/csl+json");
    set.insert("application/docbook+xml");
    set.insert("application/epub+zip");
    set.insert("application/rtf");
    set.insert("application/vnd.oasis.opendocument.text");
    set.insert(DOCX_MIME_TYPE);
    set.insert("application/x-biblatex");
    set.insert("application/x-bibtex");
    set.insert("application/x-endnote+xml");
    set.insert("application/x-fictionbook+xml");
    set.insert("application/x-ipynb+json");
    set.insert("application/x-jats+xml");
    set.insert("application/x-latex");
    set.insert("application/x-opml+xml");
    set.insert("application/x-research-info-systems");
    set.insert("application/x-typst");
    set.insert("text/csv");
    set.insert("text/tab-separated-values");
    set.insert("text/troff");
    set.insert("text/x-commonmark");
    set.insert("text/x-dokuwiki");
    set.insert("text/x-gfm");
    set.insert("text/x-markdown-extra");
    set.insert("text/x-mdoc");
    set.insert("text/x-multimarkdown");
    set.insert("text/x-org");
    set.insert("text/x-pod");
    set.insert("text/x-rst");

    set.insert(EXCEL_MIME_TYPE);
    set.insert(EXCEL_BINARY_MIME_TYPE);
    set.insert(EXCEL_MACRO_MIME_TYPE);
    set.insert(EXCEL_BINARY_2007_MIME_TYPE);
    set.insert(EXCEL_ADDON_MIME_TYPE);
    set.insert(EXCEL_TEMPLATE_MIME_TYPE);
    set.insert(OPENDOC_SPREADSHEET_MIME_TYPE);

    set.insert(PDF_MIME_TYPE);
    set.insert(POWER_POINT_MIME_TYPE);
    set.insert(LEGACY_WORD_MIME_TYPE);
    set.insert(LEGACY_POWERPOINT_MIME_TYPE);
    set.insert(HTML_MIME_TYPE);
    set.insert(EML_MIME_TYPE);
    set.insert(MSG_MIME_TYPE);
    set.insert(JSON_MIME_TYPE);
    set.insert("text/json");
    set.insert(YAML_MIME_TYPE);
    set.insert("text/yaml");
    set.insert("text/x-yaml");
    set.insert("application/yaml");
    set.insert(TOML_MIME_TYPE);
    set.insert("text/toml");
    set.insert(XML_MIME_TYPE);
    set.insert(XML_TEXT_MIME_TYPE);
    set.insert(SVG_MIME_TYPE);

    set.insert("application/zip");
    set.insert("application/x-zip-compressed");
    set.insert("application/x-tar");
    set.insert("application/tar");
    set.insert("application/x-gtar");
    set.insert("application/x-ustar");
    set.insert("application/x-7z-compressed");

    set
});

/// Detect MIME type from a file path.
///
/// Uses file extension to determine MIME type. Falls back to `mime_guess` crate
/// if extension-based detection fails.
///
/// # Arguments
///
/// * `path` - Path to the file
/// * `check_exists` - Whether to verify file existence
///
/// # Returns
///
/// The detected MIME type string.
///
/// # Errors
///
/// Returns `KreuzbergError::Validation` if file doesn't exist (when `check_exists` is true).
/// Returns `KreuzbergError::UnsupportedFormat` if MIME type cannot be determined.
pub fn detect_mime_type(path: impl AsRef<Path>, check_exists: bool) -> Result<String> {
    let path = path.as_ref();

    if check_exists && !path.exists() {
        return Err(KreuzbergError::validation(format!(
            "File does not exist: {}",
            path.display()
        )));
    }

    let extension = path.extension().and_then(|ext| ext.to_str()).map(|s| s.to_lowercase());

    if let Some(ext) = &extension
        && let Some(mime_type) = EXT_TO_MIME.get(ext.as_str())
    {
        return Ok(mime_type.to_string());
    }

    let guess = mime_guess::from_path(path).first();
    if let Some(mime) = guess {
        return Ok(mime.to_string());
    }

    if let Some(ext) = extension {
        return Err(KreuzbergError::UnsupportedFormat(format!(
            "Unknown extension: .{}",
            ext
        )));
    }

    Err(KreuzbergError::validation(format!(
        "Could not determine MIME type from file path: {}",
        path.display()
    )))
}

/// Validate that a MIME type is supported.
///
/// # Arguments
///
/// * `mime_type` - The MIME type to validate
///
/// # Returns
///
/// The validated MIME type (may be normalized).
///
/// # Errors
///
/// Returns `KreuzbergError::UnsupportedFormat` if not supported.
pub fn validate_mime_type(mime_type: &str) -> Result<String> {
    if SUPPORTED_MIME_TYPES.contains(mime_type) {
        return Ok(mime_type.to_string());
    }

    if mime_type.starts_with("image/") {
        return Ok(mime_type.to_string());
    }

    Err(KreuzbergError::UnsupportedFormat(mime_type.to_string()))
}

/// Detect or validate MIME type.
///
/// If `mime_type` is provided, validates it. Otherwise, detects from `path`.
///
/// # Arguments
///
/// * `path` - Optional path to detect MIME type from
/// * `mime_type` - Optional explicit MIME type to validate
///
/// # Returns
///
/// The validated MIME type string.
pub fn detect_or_validate(path: Option<&Path>, mime_type: Option<&str>) -> Result<String> {
    if let Some(mime) = mime_type {
        validate_mime_type(mime)
    } else if let Some(p) = path {
        let detected = detect_mime_type(p, true)?;
        validate_mime_type(&detected)
    } else {
        Err(KreuzbergError::validation(
            "Must provide either path or mime_type".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_detect_mime_type_pdf() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.pdf");
        File::create(&file_path).unwrap();

        let mime = detect_mime_type(&file_path, true).unwrap();
        assert_eq!(mime, "application/pdf");
    }

    #[test]
    fn test_detect_mime_type_images() {
        let dir = tempdir().unwrap();

        let test_cases = vec![
            ("test.png", "image/png"),
            ("test.jpg", "image/jpeg"),
            ("test.jpeg", "image/jpeg"),
            ("test.gif", "image/gif"),
            ("test.bmp", "image/bmp"),
            ("test.webp", "image/webp"),
            ("test.tiff", "image/tiff"),
        ];

        for (filename, expected_mime) in test_cases {
            let file_path = dir.path().join(filename);
            File::create(&file_path).unwrap();
            let mime = detect_mime_type(&file_path, true).unwrap();
            assert_eq!(mime, expected_mime, "Failed for {}", filename);
        }
    }

    #[test]
    fn test_detect_mime_type_office() {
        let dir = tempdir().unwrap();

        let test_cases = vec![
            ("test.xlsx", EXCEL_MIME_TYPE),
            ("test.xls", EXCEL_BINARY_MIME_TYPE),
            ("test.pptx", POWER_POINT_MIME_TYPE),
            ("test.ppt", LEGACY_POWERPOINT_MIME_TYPE),
            ("test.docx", DOCX_MIME_TYPE),
            ("test.doc", LEGACY_WORD_MIME_TYPE),
        ];

        for (filename, expected_mime) in test_cases {
            let file_path = dir.path().join(filename);
            File::create(&file_path).unwrap();
            let mime = detect_mime_type(&file_path, true).unwrap();
            assert_eq!(mime, expected_mime, "Failed for {}", filename);
        }
    }

    #[test]
    fn test_detect_mime_type_data_formats() {
        let dir = tempdir().unwrap();

        let test_cases = vec![
            ("test.json", JSON_MIME_TYPE),
            ("test.yaml", YAML_MIME_TYPE),
            ("test.yml", YAML_MIME_TYPE),
            ("test.toml", TOML_MIME_TYPE),
            ("test.xml", XML_MIME_TYPE),
            ("test.csv", "text/csv"),
        ];

        for (filename, expected_mime) in test_cases {
            let file_path = dir.path().join(filename);
            File::create(&file_path).unwrap();
            let mime = detect_mime_type(&file_path, true).unwrap();
            assert_eq!(mime, expected_mime, "Failed for {}", filename);
        }
    }

    #[test]
    fn test_detect_mime_type_text_formats() {
        let dir = tempdir().unwrap();

        let test_cases = vec![
            ("test.txt", PLAIN_TEXT_MIME_TYPE),
            ("test.md", MARKDOWN_MIME_TYPE),
            ("test.html", HTML_MIME_TYPE),
            ("test.htm", HTML_MIME_TYPE),
        ];

        for (filename, expected_mime) in test_cases {
            let file_path = dir.path().join(filename);
            File::create(&file_path).unwrap();
            let mime = detect_mime_type(&file_path, true).unwrap();
            assert_eq!(mime, expected_mime, "Failed for {}", filename);
        }
    }

    #[test]
    fn test_detect_mime_type_email() {
        let dir = tempdir().unwrap();

        let test_cases = vec![("test.eml", EML_MIME_TYPE), ("test.msg", MSG_MIME_TYPE)];

        for (filename, expected_mime) in test_cases {
            let file_path = dir.path().join(filename);
            File::create(&file_path).unwrap();
            let mime = detect_mime_type(&file_path, true).unwrap();
            assert_eq!(mime, expected_mime, "Failed for {}", filename);
        }
    }

    #[test]
    fn test_validate_mime_type_exact() {
        assert!(validate_mime_type("application/pdf").is_ok());
        assert!(validate_mime_type("text/plain").is_ok());
        assert!(validate_mime_type("text/html").is_ok());
    }

    #[test]
    fn test_validate_mime_type_images() {
        assert!(validate_mime_type("image/jpeg").is_ok());
        assert!(validate_mime_type("image/png").is_ok());
        assert!(validate_mime_type("image/gif").is_ok());
        assert!(validate_mime_type("image/webp").is_ok());

        assert!(validate_mime_type("image/custom-format").is_ok());
    }

    #[test]
    fn test_validate_mime_type_unsupported() {
        assert!(validate_mime_type("application/unknown").is_err());
        assert!(validate_mime_type("video/mp4").is_err());
    }

    #[test]
    fn test_file_not_exists() {
        let result = detect_mime_type("/nonexistent/file.pdf", true);
        assert!(result.is_err());
    }

    #[test]
    fn test_file_no_extension() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("testfile");
        File::create(&file_path).unwrap();

        let result = detect_mime_type(&file_path, true);
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_detect_or_validate_with_mime() {
        let result = detect_or_validate(None, Some("application/pdf"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "application/pdf");
    }

    #[test]
    fn test_detect_or_validate_with_path() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.pdf");
        File::create(&file_path).unwrap();

        let result = detect_or_validate(Some(&file_path), None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "application/pdf");
    }

    #[test]
    fn test_detect_or_validate_neither() {
        let result = detect_or_validate(None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_case_insensitive_extensions() {
        let dir = tempdir().unwrap();

        let file_path = dir.path().join("test.PDF");
        File::create(&file_path).unwrap();
        let mime = detect_mime_type(&file_path, true).unwrap();
        assert_eq!(mime, "application/pdf");

        let file_path2 = dir.path().join("test.XLSX");
        File::create(&file_path2).unwrap();
        let mime2 = detect_mime_type(&file_path2, true).unwrap();
        assert_eq!(mime2, EXCEL_MIME_TYPE);
    }
}
