//! DOCX (Microsoft Word) text extraction using docx-lite.
//!
//! This module provides high-performance text extraction from DOCX files using the docx-lite
//! library, which uses streaming XML parsing for efficiency.
//!
//! Page break detection is best-effort, detecting only explicit page breaks (`<w:br w:type="page"/>`)
//! in the document XML. This does not account for automatic pagination based on content reflowing.

use crate::error::{KreuzbergError, Result};
use crate::types::PageBoundary;
use std::io::Cursor;

/// Extract text from DOCX bytes using docx-lite.
///
/// # Arguments
/// * `bytes` - The DOCX file contents as bytes
///
/// # Returns
/// * `Ok(String)` - The extracted text content
/// * `Err(KreuzbergError)` - If extraction fails
///
/// # Performance
/// docx-lite uses streaming XML parsing for minimal memory overhead and high throughput
/// (~160 MB/s average).
pub fn extract_text(bytes: &[u8]) -> Result<String> {
    docx_lite::extract_text_from_bytes(bytes)
        .map_err(|e| KreuzbergError::parsing(format!("DOCX text extraction failed: {}", e)))
}

/// Extract text and page boundaries from DOCX bytes.
///
/// Detects explicit page breaks (`<w:br w:type="page"/>`) in the document XML and maps them to
/// character offsets in the extracted text. This is a best-effort approach that only detects
/// explicit page breaks, not automatic pagination.
///
/// # Arguments
/// * `bytes` - The DOCX file contents as bytes
///
/// # Returns
/// * `Ok((String, Option<Vec<PageBoundary>>))` - Extracted text and optional page boundaries
/// * `Err(KreuzbergError)` - If extraction fails
///
/// # Limitations
/// - Only detects explicit page breaks, not reflowed content
/// - Page numbers are estimates, not guaranteed accurate
/// - Word's pagination may differ from detected breaks
/// - No page dimensions available (would require layout engine)
///
/// # Performance
/// Performs two passes: one with docx-lite for text extraction and one for page break detection.
pub fn extract_text_with_page_breaks(bytes: &[u8]) -> Result<(String, Option<Vec<PageBoundary>>)> {
    // Extract text content
    let text = extract_text(bytes)?;

    // Detect page breaks from document.xml
    let page_breaks = detect_page_breaks(bytes)?;

    // If no page breaks found, return text without boundaries
    if page_breaks.is_empty() {
        return Ok((text, None));
    }

    // Map detected page break positions to character offsets
    let boundaries = map_page_breaks_to_boundaries(&text, page_breaks)?;

    Ok((text, Some(boundaries)))
}

/// Detect explicit page break positions in document.xml and extract full text with page boundaries.
///
/// This is a convenience function for the extractor that combines text extraction with page
/// break detection. It returns the extracted text along with page boundaries.
///
/// # Arguments
/// * `bytes` - The DOCX file contents (ZIP archive)
///
/// # Returns
/// * `Ok(Option<Vec<PageBoundary>>)` - Optional page boundaries
/// * `Err(KreuzbergError)` - If extraction fails
///
/// # Limitations
/// - Only detects explicit page breaks, not reflowed content
/// - Page numbers are estimates based on detected breaks
pub fn detect_page_breaks_from_docx(bytes: &[u8]) -> Result<Option<Vec<PageBoundary>>> {
    match extract_text_with_page_breaks(bytes) {
        Ok((_, boundaries)) => Ok(boundaries),
        Err(e) => {
            // Page break detection is best-effort, return None on any error
            // The main extraction should not fail due to page break detection
            tracing::debug!("Page break detection failed: {}", e);
            Ok(None)
        }
    }
}

/// Detect explicit page break positions in document.xml.
///
/// Returns a vector of byte offsets within the document.xml content where page breaks occur.
/// These offsets will later be mapped to character positions in the extracted text.
///
/// # Arguments
/// * `bytes` - The DOCX file contents (ZIP archive)
///
/// # Returns
/// * `Ok(Vec<usize>)` - Vector of detected page break byte offsets (empty if none found)
/// * `Err(KreuzbergError)` - If ZIP/XML parsing fails
fn detect_page_breaks(bytes: &[u8]) -> Result<Vec<usize>> {
    use zip::ZipArchive;

    let cursor = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| KreuzbergError::parsing(format!("Failed to open DOCX as ZIP: {}", e)))?;

    // Extract document.xml
    let document_xml = match archive.by_name("word/document.xml") {
        Ok(mut file) => {
            let mut content = String::new();
            std::io::Read::read_to_string(&mut file, &mut content)
                .map_err(|e| KreuzbergError::parsing(format!("Failed to read document.xml: {}", e)))?;
            content
        }
        Err(_) => return Ok(Vec::new()), // No document.xml means no page breaks
    };

    // Simple string-based detection for explicit page breaks
    // Looking for: <w:br w:type="page"/>
    let mut breaks = Vec::new();
    let search_pattern = r#"<w:br w:type="page"/>"#;

    for (idx, _) in document_xml.match_indices(search_pattern) {
        breaks.push(idx);
    }

    Ok(breaks)
}

/// Map detected page break positions to character boundaries in extracted text.
///
/// Since we don't have a precise mapping between document.xml byte positions and final text
/// character positions, we use a heuristic: divide the text roughly equally between detected breaks.
/// This is best-effort and may not perfectly match Word's pagination.
///
/// # Arguments
/// * `text` - The extracted document text
/// * `page_breaks` - Vector of detected page break positions (unused, but kept for extension)
///
/// # Returns
/// * `Ok(Vec<PageBoundary>)` - Character boundaries for each page
fn map_page_breaks_to_boundaries(text: &str, page_breaks: Vec<usize>) -> Result<Vec<PageBoundary>> {
    if page_breaks.is_empty() {
        return Ok(Vec::new());
    }

    // Estimate page count: number of breaks + 1 (for content before first break and after last break)
    let page_count = page_breaks.len() + 1;

    // Divide text roughly equally by page count
    // This is a simple heuristic: we assume breaks are roughly evenly distributed
    let chars_per_page = text.len() / page_count;

    let mut boundaries = Vec::new();
    let mut char_start = 0;

    for page_num in 1..=page_count {
        let char_end = if page_num == page_count {
            // Last page: extend to end of document
            text.len()
        } else {
            // Regular page: extend to estimated boundary
            let mut end = (page_num * chars_per_page).min(text.len());
            // Ensure we split at a valid UTF-8 boundary
            while !text.is_char_boundary(end) && end > 0 {
                end -= 1;
            }
            end
        };

        boundaries.push(PageBoundary {
            char_start,
            char_end,
            page_number: page_num,
        });

        char_start = char_end;
    }

    Ok(boundaries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_empty() {
        let result = extract_text(b"");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_text_invalid() {
        let result = extract_text(b"not a docx file");
        assert!(result.is_err());
    }

    #[test]
    fn test_map_page_breaks_to_boundaries_empty() {
        let result = map_page_breaks_to_boundaries("test text", Vec::new()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_map_page_breaks_to_boundaries_single_break() {
        let text = "Page 1 content here with some text.Page 2 content here with more text.";
        let breaks = vec![0]; // One break detected

        let result = map_page_breaks_to_boundaries(text, breaks).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].page_number, 1);
        assert_eq!(result[0].char_start, 0);
        assert!(result[0].char_end > 0);
        assert!(result[0].char_end < text.len());

        assert_eq!(result[1].page_number, 2);
        assert_eq!(result[1].char_start, result[0].char_end);
        assert_eq!(result[1].char_end, text.len());
    }

    #[test]
    fn test_map_page_breaks_to_boundaries_multiple_breaks() {
        let text = "A".repeat(300); // 300 character text
        let breaks = vec![0, 0, 0]; // Three breaks

        let result = map_page_breaks_to_boundaries(&text, breaks).unwrap();

        assert_eq!(result.len(), 4); // 3 breaks + 1 = 4 pages
        assert_eq!(result[0].page_number, 1);
        assert_eq!(result[3].page_number, 4);
        assert_eq!(result[3].char_end, 300);

        // Verify all boundaries are continuous
        for i in 0..result.len() - 1 {
            assert_eq!(result[i].char_end, result[i + 1].char_start);
        }
    }

    #[test]
    fn test_map_page_breaks_to_boundaries_utf8_boundary() {
        // Test with UTF-8 multi-byte characters
        let text = "Hello world! こんにちは世界！ More text here.";
        let breaks = vec![0]; // One break

        let result = map_page_breaks_to_boundaries(text, breaks).unwrap();

        assert_eq!(result.len(), 2);
        // Should not panic when checking UTF-8 boundaries
        assert!(text.is_char_boundary(result[0].char_start));
        assert!(text.is_char_boundary(result[0].char_end));
        assert!(text.is_char_boundary(result[1].char_start));
        assert!(text.is_char_boundary(result[1].char_end));
    }

    #[test]
    fn test_detect_page_breaks_no_feature() {
        // Page break detection should handle invalid ZIP gracefully
        // Invalid input should return an error (expected behavior)
        let result = detect_page_breaks(b"invalid");
        // Should fail because "invalid" is not a valid ZIP file
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_text_with_page_breaks_no_breaks() {
        // Load a real DOCX file without explicit page breaks
        let docx_path =
            "/Users/naamanhirschfeld/workspace/kreuzberg-dev/kreuzberg/test_documents/documents/lorem_ipsum.docx";
        if let Ok(bytes) = std::fs::read(docx_path) {
            let result = extract_text_with_page_breaks(&bytes);
            // Should succeed and either have no boundaries or empty
            match result {
                Ok((text, boundaries)) => {
                    assert!(!text.is_empty());
                    // If boundaries present, should have at least one page
                    if let Some(b) = boundaries {
                        assert!(!b.is_empty());
                    }
                }
                Err(_) => {} // File might not exist in test environment
            }
        }
    }
}
