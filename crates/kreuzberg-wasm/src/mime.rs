//! MIME type utilities for WASM bindings
//!
//! This module provides utilities for MIME type detection, validation, and conversion
//! in WebAssembly environments. Includes functions for detecting MIME types from bytes,
//! file extensions, and normalizing MIME type strings.

use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Detect MIME type from raw file bytes.
///
/// Uses magic byte signatures and content analysis to detect the MIME type of
/// a document from its binary content. Falls back to text detection if binary
/// detection fails.
///
/// # JavaScript Parameters
///
/// * `data: Uint8Array` - The raw file bytes
///
/// # Returns
///
/// `string` - The detected MIME type (e.g., "application/pdf", "image/png")
///
/// # Throws
///
/// Throws an error if MIME type cannot be determined from the content.
///
/// # Example
///
/// ```javascript
/// import { detectMimeFromBytes } from '@kreuzberg/wasm';
/// import { readFileSync } from 'fs';
///
/// const pdfBytes = readFileSync('document.pdf');
/// const mimeType = detectMimeFromBytes(new Uint8Array(pdfBytes));
/// console.log(mimeType); // "application/pdf"
/// ```
#[wasm_bindgen(js_name = detectMimeFromBytes)]
pub fn detect_mime_from_bytes(data: js_sys::Uint8Array) -> Result<String, JsValue> {
    let bytes = data.to_vec();
    kreuzberg::detect_mime_type_from_bytes(&bytes).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get MIME type from file extension.
///
/// Looks up the MIME type associated with a given file extension.
/// Returns None if the extension is not recognized.
///
/// # JavaScript Parameters
///
/// * `extension: string` - The file extension (with or without leading dot)
///
/// # Returns
///
/// `string | null` - The MIME type if found, null otherwise
///
/// # Example
///
/// ```javascript
/// import { getMimeFromExtension } from '@kreuzberg/wasm';
///
/// const pdfMime = getMimeFromExtension('pdf');
/// console.log(pdfMime); // "application/pdf"
///
/// const docMime = getMimeFromExtension('docx');
/// console.log(docMime); // "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
///
/// const unknownMime = getMimeFromExtension('unknown');
/// console.log(unknownMime); // null
/// ```
#[wasm_bindgen(js_name = getMimeFromExtension)]
pub fn get_mime_from_extension(extension: String) -> Option<String> {
    // Remove leading dot if present
    let ext = if let Some(stripped) = extension.strip_prefix('.') {
        stripped
    } else {
        &extension
    };

    // Normalize to lowercase
    let ext_lower = ext.to_lowercase();

    // Try to get MIME type from kreuzberg
    // We need to match against known extensions
    match ext_lower.as_str() {
        "txt" => Some("text/plain".to_string()),
        "md" | "markdown" => Some("text/markdown".to_string()),
        "pdf" => Some("application/pdf".to_string()),
        "html" | "htm" => Some("text/html".to_string()),
        "xlsx" => Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string()),
        "xls" => Some("application/vnd.ms-excel".to_string()),
        "xlsm" => Some("application/vnd.ms-excel.sheet.macroEnabled.12".to_string()),
        "xlsb" => Some("application/vnd.ms-excel.sheet.binary.macroEnabled.12".to_string()),
        "xlam" => Some("application/vnd.ms-excel.addin.macroEnabled.12".to_string()),
        "xla" => Some("application/vnd.ms-excel.template.macroEnabled.12".to_string()),
        "ods" => Some("application/vnd.oasis.opendocument.spreadsheet".to_string()),
        "pptx" => Some("application/vnd.openxmlformats-officedocument.presentationml.presentation".to_string()),
        "ppt" => Some("application/vnd.ms-powerpoint".to_string()),
        "docx" => Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string()),
        "doc" => Some("application/msword".to_string()),
        "odt" => Some("application/vnd.oasis.opendocument.text".to_string()),
        "bmp" => Some("image/bmp".to_string()),
        "gif" => Some("image/gif".to_string()),
        "jpg" | "jpeg" => Some("image/jpeg".to_string()),
        "png" => Some("image/png".to_string()),
        "tiff" | "tif" => Some("image/tiff".to_string()),
        "webp" => Some("image/webp".to_string()),
        "jp2" => Some("image/jp2".to_string()),
        "jpx" => Some("image/jpx".to_string()),
        "jpm" => Some("image/jpm".to_string()),
        "mj2" => Some("image/mj2".to_string()),
        "pnm" => Some("image/x-portable-anymap".to_string()),
        "pbm" => Some("image/x-portable-bitmap".to_string()),
        "pgm" => Some("image/x-portable-graymap".to_string()),
        "ppm" => Some("image/x-portable-pixmap".to_string()),
        "csv" => Some("text/csv".to_string()),
        "tsv" => Some("text/tab-separated-values".to_string()),
        "json" => Some("application/json".to_string()),
        "yaml" | "yml" => Some("application/x-yaml".to_string()),
        "toml" => Some("application/toml".to_string()),
        "xml" => Some("application/xml".to_string()),
        "svg" => Some("image/svg+xml".to_string()),
        "eml" => Some("message/rfc822".to_string()),
        "msg" => Some("application/vnd.ms-outlook".to_string()),
        "zip" => Some("application/zip".to_string()),
        "tar" => Some("application/x-tar".to_string()),
        "gz" => Some("application/gzip".to_string()),
        "tgz" => Some("application/x-tar".to_string()),
        "7z" => Some("application/x-7z-compressed".to_string()),
        "rst" => Some("text/x-rst".to_string()),
        "org" => Some("text/x-org".to_string()),
        "epub" => Some("application/epub+zip".to_string()),
        "rtf" => Some("application/rtf".to_string()),
        "bib" => Some("application/x-bibtex".to_string()),
        "ipynb" => Some("application/x-ipynb+json".to_string()),
        "tex" | "latex" => Some("application/x-latex".to_string()),
        "typst" => Some("application/x-typst".to_string()),
        "commonmark" => Some("text/x-commonmark".to_string()),
        _ => None,
    }
}

/// Get file extensions for a given MIME type.
///
/// Looks up all known file extensions that correspond to the specified MIME type.
/// Returns a JavaScript Array of extension strings (without leading dots).
///
/// # JavaScript Parameters
///
/// * `mimeType: string` - The MIME type to look up (e.g., "application/pdf")
///
/// # Returns
///
/// `string[]` - Array of file extensions for the MIME type
///
/// # Throws
///
/// Throws an error if the MIME type is not recognized.
///
/// # Example
///
/// ```javascript
/// import { getExtensionsForMime } from '@kreuzberg/wasm';
///
/// const pdfExts = getExtensionsForMime('application/pdf');
/// console.log(pdfExts); // ["pdf"]
///
/// const jpegExts = getExtensionsForMime('image/jpeg');
/// console.log(jpegExts); // ["jpg", "jpeg"]
/// ```
#[wasm_bindgen(js_name = getExtensionsForMime)]
pub fn get_extensions_for_mime(mime_type: String) -> Result<Array, JsValue> {
    kreuzberg::get_extensions_for_mime(&mime_type)
        .map_err(|e| JsValue::from_str(&e.to_string()))
        .map(|extensions| {
            let array = Array::new();
            for ext in extensions {
                array.push(&JsValue::from_str(&ext));
            }
            array
        })
}

/// Normalize a MIME type string.
///
/// Normalizes a MIME type by converting to lowercase and removing parameters
/// (e.g., "application/json; charset=utf-8" becomes "application/json").
/// This is useful for consistent MIME type comparison.
///
/// # JavaScript Parameters
///
/// * `mimeType: string` - The MIME type string to normalize
///
/// # Returns
///
/// `string` - The normalized MIME type
///
/// # Example
///
/// ```javascript
/// import { normalizeMimeType } from '@kreuzberg/wasm';
///
/// const normalized1 = normalizeMimeType('Application/JSON');
/// console.log(normalized1); // "application/json"
///
/// const normalized2 = normalizeMimeType('text/html; charset=utf-8');
/// console.log(normalized2); // "text/html"
///
/// const normalized3 = normalizeMimeType('Text/Plain; charset=ISO-8859-1');
/// console.log(normalized3); // "text/plain"
/// ```
#[wasm_bindgen(js_name = normalizeMimeType)]
pub fn normalize_mime_type(mime_type: String) -> String {
    // Convert to lowercase and trim whitespace
    let trimmed = mime_type.trim().to_lowercase();

    // Remove parameters (everything after ';')
    if let Some(semicolon_pos) = trimmed.find(';') {
        trimmed[..semicolon_pos].trim().to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_mime_type_basic() {
        assert_eq!(normalize_mime_type("text/plain".to_string()), "text/plain");
    }

    #[test]
    fn test_normalize_mime_type_uppercase() {
        assert_eq!(normalize_mime_type("Application/JSON".to_string()), "application/json");
    }

    #[test]
    fn test_normalize_mime_type_with_charset() {
        assert_eq!(normalize_mime_type("text/html; charset=utf-8".to_string()), "text/html");
    }

    #[test]
    fn test_get_mime_from_extension_pdf() {
        assert_eq!(
            get_mime_from_extension("pdf".to_string()),
            Some("application/pdf".to_string())
        );
        assert_eq!(
            get_mime_from_extension(".pdf".to_string()),
            Some("application/pdf".to_string())
        );
    }

    #[test]
    fn test_get_mime_from_extension_image() {
        assert_eq!(
            get_mime_from_extension("jpg".to_string()),
            Some("image/jpeg".to_string())
        );
        assert_eq!(
            get_mime_from_extension("jpeg".to_string()),
            Some("image/jpeg".to_string())
        );
        assert_eq!(
            get_mime_from_extension("png".to_string()),
            Some("image/png".to_string())
        );
    }

    #[test]
    fn test_get_mime_from_extension_unknown() {
        assert_eq!(get_mime_from_extension("unknown".to_string()), None);
        assert_eq!(get_mime_from_extension("xyz".to_string()), None);
    }
}
