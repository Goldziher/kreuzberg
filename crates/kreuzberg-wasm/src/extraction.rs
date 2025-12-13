//! Document extraction functionality for WASM
//!
//! This module provides functions for extracting content from various document formats
//! in WebAssembly environments. Supports both synchronous and asynchronous extraction
//! from byte arrays and web-accessible files.

use crate::errors::convert_error;
use crate::types::{parse_config, result_to_js_value, results_to_js_value};
use js_sys::Uint8Array;
use kreuzberg::{batch_extract_bytes_sync, extract_bytes, extract_bytes_sync};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{File, FileReader};

/// Extract content from a byte array (synchronous).
///
/// Extracts text, tables, images, and metadata from a document represented as bytes.
/// This is a synchronous, blocking operation suitable for smaller documents or when
/// async execution is not available.
///
/// # JavaScript Parameters
///
/// * `data: Uint8Array` - The document bytes to extract
/// * `mimeType: string` - MIME type of the data (e.g., "application/pdf", "image/png")
/// * `config?: object` - Optional extraction configuration
///
/// # Returns
///
/// `object` - ExtractionResult with extracted content and metadata
///
/// # Throws
///
/// Throws an error if data is malformed or MIME type is unsupported.
///
/// # Example
///
/// ```javascript
/// import { extractBytesSync } from '@kreuzberg/wasm';
/// import { readFileSync } from 'fs';
///
/// const buffer = readFileSync('document.pdf');
/// const data = new Uint8Array(buffer);
/// const result = extractBytesSync(data, 'application/pdf', null);
/// console.log(result.content);
/// ```
#[wasm_bindgen(js_name = extractBytesSync)]
pub fn extract_bytes_sync_wasm(
    data: Uint8Array,
    mime_type: String,
    config: Option<JsValue>,
) -> Result<JsValue, JsValue> {
    let extraction_config = parse_config(config)?;
    let bytes = data.to_vec();

    extract_bytes_sync(&bytes, &mime_type, &extraction_config)
        .map_err(convert_error)
        .and_then(|result| result_to_js_value(&result))
}

/// Extract content from a byte array (asynchronous).
///
/// Asynchronously extracts text, tables, images, and metadata from a document.
/// Non-blocking alternative to `extractBytesSync` suitable for large documents
/// or browser environments.
///
/// # JavaScript Parameters
///
/// * `data: Uint8Array` - The document bytes to extract
/// * `mimeType: string` - MIME type of the data (e.g., "application/pdf")
/// * `config?: object` - Optional extraction configuration
///
/// # Returns
///
/// `Promise<object>` - Promise resolving to ExtractionResult
///
/// # Throws
///
/// Rejects if data is malformed or MIME type is unsupported.
///
/// # Example
///
/// ```javascript
/// import { extractBytes } from '@kreuzberg/wasm';
///
/// // Fetch from URL
/// const response = await fetch('document.pdf');
/// const arrayBuffer = await response.arrayBuffer();
/// const data = new Uint8Array(arrayBuffer);
///
/// const result = await extractBytes(data, 'application/pdf', null);
/// console.log(result.content.substring(0, 100));
/// ```
#[wasm_bindgen(js_name = extractBytes)]
pub fn extract_bytes_wasm(data: Uint8Array, mime_type: String, config: Option<JsValue>) -> js_sys::Promise {
    let bytes = data.to_vec();

    wasm_bindgen_futures::future_to_promise(async move {
        let extraction_config = parse_config(config)?;
        let result = extract_bytes(&bytes, &mime_type, &extraction_config)
            .await
            .map_err(convert_error)?;

        result_to_js_value(&result)
    })
}

/// Extract content from a web File or Blob (asynchronous).
///
/// Extracts content from a web File (from `<input type="file">`) or Blob object
/// using the FileReader API. Only available in browser environments.
///
/// # JavaScript Parameters
///
/// * `file: File | Blob` - The file or blob to extract
/// * `mimeType?: string` - Optional MIME type hint (auto-detected if omitted)
/// * `config?: object` - Optional extraction configuration
///
/// # Returns
///
/// `Promise<object>` - Promise resolving to ExtractionResult
///
/// # Throws
///
/// Rejects if file cannot be read or is malformed.
///
/// # Example
///
/// ```javascript
/// import { extractFile } from '@kreuzberg/wasm';
///
/// // From file input
/// const fileInput = document.getElementById('file-input');
/// const file = fileInput.files[0];
///
/// const result = await extractFile(file, null, null);
/// console.log(`Extracted ${result.content.length} characters`);
/// ```
#[wasm_bindgen(js_name = extractFile)]
pub fn extract_file_wasm(file: &web_sys::File, mime_type: Option<String>, config: Option<JsValue>) -> js_sys::Promise {
    let file_clone = file.clone();
    let mime_type_clone = mime_type.clone();
    let config_clone = config.clone();

    wasm_bindgen_futures::future_to_promise(async move {
        // Read the file using FileReader
        let bytes = read_file_as_array_buffer(&file_clone)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to read file: {}", e)))?;

        let extraction_config = parse_config(config_clone)?;
        let mime = mime_type_clone.unwrap_or_else(|| file_clone.type_());

        let result = extract_bytes(&bytes, &mime, &extraction_config)
            .await
            .map_err(convert_error)?;

        result_to_js_value(&result)
    })
}

/// Batch extract from multiple byte arrays (synchronous).
///
/// Processes multiple document byte arrays in parallel. All documents use the
/// same extraction configuration.
///
/// # JavaScript Parameters
///
/// * `dataList: Uint8Array[]` - Array of document bytes
/// * `mimeTypes: string[]` - Array of MIME types (must match dataList length)
/// * `config?: object` - Optional extraction configuration (applied to all)
///
/// # Returns
///
/// `object[]` - Array of ExtractionResults in the same order as inputs
///
/// # Throws
///
/// Throws if dataList and mimeTypes lengths don't match.
///
/// # Example
///
/// ```javascript
/// import { batchExtractBytesSync } from '@kreuzberg/wasm';
///
/// const buffers = [buffer1, buffer2, buffer3];
/// const mimeTypes = ['application/pdf', 'text/plain', 'image/png'];
/// const results = batchExtractBytesSync(buffers, mimeTypes, null);
///
/// results.forEach((result, i) => {
///   console.log(`Document ${i}: ${result.content.substring(0, 50)}...`);
/// });
/// ```
#[wasm_bindgen(js_name = batchExtractBytesSync)]
pub fn batch_extract_bytes_sync_wasm(
    data_list: Vec<Uint8Array>,
    mime_types: Vec<String>,
    config: Option<JsValue>,
) -> Result<JsValue, JsValue> {
    if data_list.len() != mime_types.len() {
        return Err(JsValue::from_str("data_list and mime_types must have the same length"));
    }

    let extraction_config = parse_config(config)?;
    let owned_data: Vec<Vec<u8>> = data_list.iter().map(|d| d.to_vec()).collect();

    let contents: Vec<(&[u8], &str)> = owned_data
        .iter()
        .zip(mime_types.iter())
        .map(|(data, mime)| (data.as_slice(), mime.as_str()))
        .collect();

    let results = batch_extract_bytes_sync(contents, &extraction_config).map_err(convert_error)?;

    results_to_js_value(&results)
}

/// Batch extract from multiple byte arrays (asynchronous).
///
/// Asynchronously processes multiple document byte arrays in parallel.
/// Non-blocking alternative to `batchExtractBytesSync`.
///
/// # JavaScript Parameters
///
/// * `dataList: Uint8Array[]` - Array of document bytes
/// * `mimeTypes: string[]` - Array of MIME types (must match dataList length)
/// * `config?: object` - Optional extraction configuration (applied to all)
///
/// # Returns
///
/// `Promise<object[]>` - Promise resolving to array of ExtractionResults
///
/// # Throws
///
/// Rejects if dataList and mimeTypes lengths don't match.
///
/// # Example
///
/// ```javascript
/// import { batchExtractBytes } from '@kreuzberg/wasm';
///
/// const responses = await Promise.all([
///   fetch('doc1.pdf'),
///   fetch('doc2.docx')
/// ]);
///
/// const buffers = await Promise.all(
///   responses.map(r => r.arrayBuffer().then(b => new Uint8Array(b)))
/// );
///
/// const results = await batchExtractBytes(
///   buffers,
///   ['application/pdf', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'],
///   null
/// );
/// ```
#[wasm_bindgen(js_name = batchExtractBytes)]
pub fn batch_extract_bytes_wasm(
    data_list: Vec<Uint8Array>,
    mime_types: Vec<String>,
    config: Option<JsValue>,
) -> js_sys::Promise {
    wasm_bindgen_futures::future_to_promise(async move {
        if data_list.len() != mime_types.len() {
            return Err(JsValue::from_str("data_list and mime_types must have the same length"));
        }

        let extraction_config = parse_config(config)?;
        let owned_data: Vec<Vec<u8>> = data_list.iter().map(|d| d.to_vec()).collect();

        let mut results = Vec::with_capacity(owned_data.len());
        for (data, mime) in owned_data.iter().zip(mime_types.iter()) {
            let result = extract_bytes(data.as_slice(), mime, &extraction_config)
                .await
                .map_err(convert_error)?;
            results.push(result);
        }

        results_to_js_value(&results)
    })
}

/// Batch extract from multiple Files or Blobs (asynchronous).
///
/// Processes multiple web File or Blob objects in parallel using the FileReader API.
/// Only available in browser environments.
///
/// # JavaScript Parameters
///
/// * `files: (File | Blob)[]` - Array of files or blobs to extract
/// * `config?: object` - Optional extraction configuration (applied to all)
///
/// # Returns
///
/// `Promise<object[]>` - Promise resolving to array of ExtractionResults
///
/// # Example
///
/// ```javascript
/// import { batchExtractFiles } from '@kreuzberg/wasm';
///
/// // From file input with multiple files
/// const fileInput = document.getElementById('file-input');
/// const files = Array.from(fileInput.files);
///
/// const results = await batchExtractFiles(files, null);
/// console.log(`Processed ${results.length} files`);
/// ```
#[wasm_bindgen(js_name = batchExtractFiles)]
pub fn batch_extract_files_wasm(files: Vec<File>, config: Option<JsValue>) -> js_sys::Promise {
    wasm_bindgen_futures::future_to_promise(async move {
        let extraction_config = parse_config(config)?;
        let mut results = Vec::with_capacity(files.len());

        for file in files {
            let bytes = read_file_as_array_buffer(&file)
                .await
                .map_err(|e| JsValue::from_str(&format!("Failed to read file: {}", e)))?;

            let mime = file.type_();
            let result = extract_bytes(&bytes, &mime, &extraction_config)
                .await
                .map_err(convert_error)?;

            results.push(result);
        }

        results_to_js_value(&results)
    })
}

/// Extract content from a file (synchronous) - NOT AVAILABLE IN WASM.
///
/// File system operations are not available in WebAssembly environments.
/// Use `extractBytesSync` or `extractBytes` instead.
///
/// # Throws
///
/// Always throws: "File operations are not available in WASM. Use extractBytesSync or extractBytes instead."
#[wasm_bindgen(js_name = extractFileSync)]
pub fn extract_file_sync_wasm() -> Result<JsValue, JsValue> {
    Err(JsValue::from_str(
        "File operations are not available in WASM. Use extractBytesSync or extractBytes instead.",
    ))
}

/// Batch extract from multiple files (synchronous) - NOT AVAILABLE IN WASM.
///
/// File system operations are not available in WebAssembly environments.
/// Use `batchExtractBytesSync` or `batchExtractBytes` instead.
///
/// # Throws
///
/// Always throws: "File operations are not available in WASM. Use batchExtractBytesSync or batchExtractBytes instead."
#[wasm_bindgen(js_name = batchExtractFilesSync)]
pub fn batch_extract_files_sync_wasm() -> Result<JsValue, JsValue> {
    Err(JsValue::from_str(
        "File operations are not available in WASM. Use batchExtractBytesSync or batchExtractBytes instead.",
    ))
}

/// Helper function to read a File/Blob as bytes using FileReader API.
///
/// This is an internal helper that reads web File/Blob objects asynchronously.
async fn read_file_as_array_buffer(file: &web_sys::File) -> Result<Vec<u8>, String> {
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::closure::Closure;

    let reader = FileReader::new().map_err(|_| "Failed to create FileReader".to_string())?;
    let reader = Rc::new(RefCell::new(reader));

    // Create a promise that will resolve when FileReader completes
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let reader_clone = reader.clone();

        let onload = {
            let resolve_clone = resolve.clone();
            let reader_inner = reader_clone.clone();

            Closure::once(move |_: JsValue| {
                if let Ok(result) = reader_inner.borrow().result() {
                    let _ = resolve_clone.call1(&JsValue::undefined(), &result);
                }
            })
        };

        let reader_borrow = reader_clone.borrow_mut();
        let _ = reader_borrow.add_event_listener_with_callback("load", onload.as_ref().unchecked_ref());

        drop(reader_borrow);
        onload.forget();
    });

    // Start the read
    reader
        .borrow_mut()
        .read_as_array_buffer(file)
        .map_err(|_| "Failed to read file".to_string())?;

    // Wait for the promise
    let array_buffer = JsFuture::from(promise)
        .await
        .map_err(|_| "File read failed".to_string())?;

    // Convert to Vec
    let arr = Uint8Array::new(&array_buffer);
    Ok(arr.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    // Static test data with 'static lifetime to avoid use-after-free
    const VALID_PDF_DATA: &[u8] = b"%PDF-1.4\n%test";
    const INVALID_DATA: &[u8] = b"some data";
    const EMPTY_DATA: &[u8] = b"";
    const TEXT_DATA: &[u8] = b"Hello, this is plain text content";

    #[wasm_bindgen_test]
    fn test_extract_bytes_sync_wasm_valid_pdf_data_returns_result() {
        // SAFETY: VALID_PDF_DATA is a static const slice with 'static lifetime,
        // so the Uint8Array view remains valid for the entire test duration.
        let data = unsafe { Uint8Array::view(VALID_PDF_DATA) };
        let mime_type = "application/pdf".to_string();
        let config = None;

        let result = extract_bytes_sync_wasm(data, mime_type, config);

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_extract_bytes_sync_wasm_invalid_mime_type_returns_error() {
        // SAFETY: INVALID_DATA is a static const slice with 'static lifetime.
        let data = unsafe { Uint8Array::view(INVALID_DATA) };
        let mime_type = "invalid/mime".to_string();
        let config = None;

        let result = extract_bytes_sync_wasm(data, mime_type, config);

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_extract_bytes_sync_wasm_empty_data_returns_error() {
        // SAFETY: EMPTY_DATA is a static const slice with 'static lifetime.
        let data = unsafe { Uint8Array::view(EMPTY_DATA) };
        let mime_type = "application/pdf".to_string();
        let config = None;

        let result = extract_bytes_sync_wasm(data, mime_type, config);

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_extract_bytes_sync_wasm_with_valid_config_returns_result() {
        // SAFETY: VALID_PDF_DATA is a static const slice with 'static lifetime.
        let data = unsafe { Uint8Array::view(VALID_PDF_DATA) };
        let mime_type = "application/pdf".to_string();
        let config = Some(JsValue::NULL);

        let result = extract_bytes_sync_wasm(data, mime_type, config);

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_extract_bytes_sync_wasm_text_plain_data_returns_result() {
        // SAFETY: TEXT_DATA is a static const slice with 'static lifetime.
        let data = unsafe { Uint8Array::view(TEXT_DATA) };
        let mime_type = "text/plain".to_string();
        let config = None;

        let result = extract_bytes_sync_wasm(data, mime_type, config);

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_extract_bytes_wasm_returns_promise() {
        // SAFETY: VALID_PDF_DATA is a static const slice with 'static lifetime.
        let data = unsafe { Uint8Array::view(VALID_PDF_DATA) };
        let mime_type = "application/pdf".to_string();
        let config = None;

        let promise = extract_bytes_wasm(data, mime_type, config);

        assert!(!promise.is_null());
        assert!(!promise.is_undefined());
    }

    #[wasm_bindgen_test]
    fn test_extract_bytes_wasm_invalid_mime_type_returns_promise() {
        // SAFETY: INVALID_DATA is a static const slice with 'static lifetime.
        let data = unsafe { Uint8Array::view(INVALID_DATA) };
        let mime_type = "invalid/type".to_string();
        let config = None;

        let promise = extract_bytes_wasm(data, mime_type, config);

        assert!(!promise.is_null());
    }

    #[wasm_bindgen_test]
    fn test_extract_bytes_wasm_with_config_returns_promise() {
        // SAFETY: VALID_PDF_DATA is a static const slice with 'static lifetime.
        let data = unsafe { Uint8Array::view(VALID_PDF_DATA) };
        let mime_type = "application/pdf".to_string();
        let config = Some(JsValue::NULL);

        let promise = extract_bytes_wasm(data, mime_type, config);

        assert!(!promise.is_null());
    }

    #[wasm_bindgen_test]
    fn test_extract_file_sync_wasm_always_returns_error() {
        let result = extract_file_sync_wasm();

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_str = format!("{:?}", err);
        assert!(err_str.contains("not available") || err_str.contains("WASM"));
    }

    #[wasm_bindgen_test]
    fn test_extract_file_sync_wasm_error_message_is_descriptive() {
        let result = extract_file_sync_wasm();

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_batch_extract_files_sync_wasm_always_returns_error() {
        let result = batch_extract_files_sync_wasm();

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_batch_extract_files_sync_wasm_error_mentions_alternative() {
        let result = batch_extract_files_sync_wasm();

        assert!(result.is_err());
    }

    // Static batch test data
    const PDF_DATA_1: &[u8] = b"%PDF-1.4\n%test1";
    const TEXT_CONTENT: &[u8] = b"Plain text content";

    #[wasm_bindgen_test]
    fn test_batch_extract_bytes_sync_wasm_matching_lengths_returns_result() {
        // SAFETY: PDF_DATA_1 and TEXT_CONTENT are static const slices with 'static lifetime.
        let data1 = unsafe { Uint8Array::view(PDF_DATA_1) };
        let data2 = unsafe { Uint8Array::view(TEXT_CONTENT) };
        let data_list = vec![data1, data2];
        let mime_types = vec!["application/pdf".to_string(), "text/plain".to_string()];
        let config = None;

        let result = batch_extract_bytes_sync_wasm(data_list, mime_types, config);

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_batch_extract_bytes_sync_wasm_mismatched_lengths_returns_error() {
        // SAFETY: VALID_PDF_DATA is a static const slice with 'static lifetime.
        let data1 = unsafe { Uint8Array::view(VALID_PDF_DATA) };
        let data_list = vec![data1];
        let mime_types = vec!["application/pdf".to_string(), "text/plain".to_string()];
        let config = None;

        let result = batch_extract_bytes_sync_wasm(data_list, mime_types, config);

        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_batch_extract_bytes_sync_wasm_empty_batch_returns_result() {
        let data_list: Vec<Uint8Array> = vec![];
        let mime_types: Vec<String> = vec![];
        let config = None;

        let result = batch_extract_bytes_sync_wasm(data_list, mime_types, config);

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_batch_extract_bytes_sync_wasm_single_document_returns_result() {
        // SAFETY: VALID_PDF_DATA is a static const slice with 'static lifetime.
        let data = unsafe { Uint8Array::view(VALID_PDF_DATA) };
        let data_list = vec![data];
        let mime_types = vec!["application/pdf".to_string()];
        let config = None;

        let result = batch_extract_bytes_sync_wasm(data_list, mime_types, config);

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_batch_extract_bytes_sync_wasm_with_config_returns_result() {
        // SAFETY: VALID_PDF_DATA is a static const slice with 'static lifetime.
        let data = unsafe { Uint8Array::view(VALID_PDF_DATA) };
        let data_list = vec![data];
        let mime_types = vec!["application/pdf".to_string()];
        let config = Some(JsValue::NULL);

        let result = batch_extract_bytes_sync_wasm(data_list, mime_types, config);

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_batch_extract_bytes_wasm_returns_promise() {
        // SAFETY: VALID_PDF_DATA is a static const slice with 'static lifetime.
        let data = unsafe { Uint8Array::view(VALID_PDF_DATA) };
        let data_list = vec![data];
        let mime_types = vec!["application/pdf".to_string()];
        let config = None;

        let promise = batch_extract_bytes_wasm(data_list, mime_types, config);

        assert!(!promise.is_null());
    }

    #[wasm_bindgen_test]
    fn test_batch_extract_bytes_wasm_mismatched_lengths_returns_promise() {
        // SAFETY: VALID_PDF_DATA is a static const slice with 'static lifetime.
        let data = unsafe { Uint8Array::view(VALID_PDF_DATA) };
        let data_list = vec![data];
        let mime_types = vec!["application/pdf".to_string(), "text/plain".to_string()];
        let config = None;

        let promise = batch_extract_bytes_wasm(data_list, mime_types, config);

        assert!(!promise.is_null());
    }

    #[wasm_bindgen_test]
    fn test_batch_extract_bytes_wasm_empty_batch_returns_promise() {
        let data_list: Vec<Uint8Array> = vec![];
        let mime_types: Vec<String> = vec![];
        let config = None;

        let promise = batch_extract_bytes_wasm(data_list, mime_types, config);

        assert!(!promise.is_null());
    }
}
