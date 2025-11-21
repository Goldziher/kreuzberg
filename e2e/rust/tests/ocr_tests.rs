// Auto-generated tests for ocr fixtures.
#![allow(clippy::too_many_lines)]
use e2e_rust::{assertions, resolve_document};
use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::KreuzbergError;

#[test]
fn test_ocr_image_hello_world() {
    // PNG image with visible English text for OCR validation.

    let document_path = resolve_document("images/test_hello_world.png");
    if !document_path.exists() {
        println!("Skipping ocr_image_hello_world: missing document at {}", document_path.display());
        return;
    }
    let config: ExtractionConfig = serde_json::from_str(r#"{
  "force_ocr": true,
  "ocr": {
    "backend": "tesseract",
    "language": "eng"
  }
}"#)
        .expect("Fixture config should deserialize");

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(KreuzbergError::MissingDependency(dep)) => {
            println!("Skipping ocr_image_hello_world: missing dependency {dep}", dep=dep);
            return;
        },
        Err(KreuzbergError::UnsupportedFormat(fmt)) => {
            println!("Skipping ocr_image_hello_world: unsupported format {fmt} (requires optional tool)", fmt=fmt);
            return;
        },
        Err(err) => panic!("Extraction failed for ocr_image_hello_world: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["image/png"]);
    assertions::assert_min_content_length(&result, 5);
    assertions::assert_content_contains_any(&result, &["hello", "world"]);
}

#[test]
fn test_ocr_image_no_text() {
    // Image with no text to ensure OCR handles empty results gracefully.

    let document_path = resolve_document("images/flower_no_text.jpg");
    if !document_path.exists() {
        println!("Skipping ocr_image_no_text: missing document at {}", document_path.display());
        return;
    }
    let config: ExtractionConfig = serde_json::from_str(r#"{
  "force_ocr": true,
  "ocr": {
    "backend": "tesseract",
    "language": "eng"
  }
}"#)
        .expect("Fixture config should deserialize");

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(KreuzbergError::MissingDependency(dep)) => {
            println!("Skipping ocr_image_no_text: missing dependency {dep}", dep=dep);
            return;
        },
        Err(KreuzbergError::UnsupportedFormat(fmt)) => {
            println!("Skipping ocr_image_no_text: unsupported format {fmt} (requires optional tool)", fmt=fmt);
            return;
        },
        Err(err) => panic!("Extraction failed for ocr_image_no_text: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["image/jpeg"]);
    assertions::assert_max_content_length(&result, 200);
}

#[test]
fn test_ocr_pdf_image_only_german() {
    // Image-only German PDF requiring OCR to extract text.

    let document_path = resolve_document("pdfs/image_only_german_pdf.pdf");
    if !document_path.exists() {
        println!("Skipping ocr_pdf_image_only_german: missing document at {}", document_path.display());
        return;
    }
    let config: ExtractionConfig = serde_json::from_str(r#"{
  "force_ocr": true,
  "ocr": {
    "backend": "tesseract",
    "language": "eng"
  }
}"#)
        .expect("Fixture config should deserialize");

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(KreuzbergError::MissingDependency(dep)) => {
            println!("Skipping ocr_pdf_image_only_german: missing dependency {dep}", dep=dep);
            return;
        },
        Err(KreuzbergError::UnsupportedFormat(fmt)) => {
            println!("Skipping ocr_pdf_image_only_german: unsupported format {fmt} (requires optional tool)", fmt=fmt);
            return;
        },
        Err(err) => panic!("Extraction failed for ocr_pdf_image_only_german: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 20);
    assertions::assert_metadata_expectation(&result, "format_type", &serde_json::json!({"eq":"pdf"}));
}

#[test]
fn test_ocr_pdf_rotated_90() {
    // Rotated page PDF requiring OCR to verify orientation handling.

    let document_path = resolve_document("pdfs/ocr_test_rotated_90.pdf");
    if !document_path.exists() {
        println!("Skipping ocr_pdf_rotated_90: missing document at {}", document_path.display());
        return;
    }
    let config: ExtractionConfig = serde_json::from_str(r#"{
  "force_ocr": true,
  "ocr": {
    "backend": "tesseract",
    "language": "eng"
  }
}"#)
        .expect("Fixture config should deserialize");

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(KreuzbergError::MissingDependency(dep)) => {
            println!("Skipping ocr_pdf_rotated_90: missing dependency {dep}", dep=dep);
            return;
        },
        Err(KreuzbergError::UnsupportedFormat(fmt)) => {
            println!("Skipping ocr_pdf_rotated_90: unsupported format {fmt} (requires optional tool)", fmt=fmt);
            return;
        },
        Err(err) => panic!("Extraction failed for ocr_pdf_rotated_90: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 10);
}

#[test]
fn test_ocr_pdf_tesseract() {
    // Scanned PDF requires OCR to extract text.

    let document_path = resolve_document("pdfs/ocr_test.pdf");
    if !document_path.exists() {
        println!("Skipping ocr_pdf_tesseract: missing document at {}", document_path.display());
        return;
    }
    let config: ExtractionConfig = serde_json::from_str(r#"{
  "force_ocr": true,
  "ocr": {
    "backend": "tesseract",
    "language": "eng"
  }
}"#)
        .expect("Fixture config should deserialize");

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(KreuzbergError::MissingDependency(dep)) => {
            println!("Skipping ocr_pdf_tesseract: missing dependency {dep}", dep=dep);
            return;
        },
        Err(KreuzbergError::UnsupportedFormat(fmt)) => {
            println!("Skipping ocr_pdf_tesseract: unsupported format {fmt} (requires optional tool)", fmt=fmt);
            return;
        },
        Err(err) => panic!("Extraction failed for ocr_pdf_tesseract: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 20);
    assertions::assert_content_contains_any(&result, &["Docling", "Markdown", "JSON"]);
}

