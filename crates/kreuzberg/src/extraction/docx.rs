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
    let text = extract_text(bytes)?;

    let page_breaks = detect_page_breaks(bytes)?;

    if page_breaks.is_empty() {
        return Ok((text, None));
    }

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

    let document_xml = match archive.by_name("word/document.xml") {
        Ok(mut file) => {
            let mut content = String::new();
            std::io::Read::read_to_string(&mut file, &mut content)
                .map_err(|e| KreuzbergError::parsing(format!("Failed to read document.xml: {}", e)))?;
            content
        }
        Err(_) => return Ok(Vec::new()),
    };

    let mut breaks = Vec::new();
    let search_pattern = r#"<w:br w:type="page"/>"#;

    for (idx, _) in document_xml.match_indices(search_pattern) {
        breaks.push(idx);
    }

    Ok(breaks)
}

/// Map detected page break positions to byte boundaries in extracted text.
///
/// Since we don't have a precise mapping between document.xml byte positions and final text
/// character positions, we use a heuristic: divide the text roughly equally between detected breaks.
/// This is best-effort and may not perfectly match Word's pagination.
///
/// # LIMITATION
/// This is a best-effort heuristic that distributes content evenly across detected page breaks.
/// It does not account for actual page layout, varying page sizes, or Word's pagination logic.
/// Use with caution. The function correctly handles multibyte UTF-8 characters (emoji, CJK, etc.)
/// by working with character indices rather than byte indices.
///
/// # Arguments
/// * `text` - The extracted document text
/// * `page_breaks` - Vector of detected page break positions (unused, but kept for extension)
///
/// # Returns
/// * `Ok(Vec<PageBoundary>)` - Byte boundaries for each page
fn map_page_breaks_to_boundaries(text: &str, page_breaks: Vec<usize>) -> Result<Vec<PageBoundary>> {
    if page_breaks.is_empty() {
        return Ok(Vec::new());
    }

    let page_count = page_breaks.len() + 1;

    let char_count = text.chars().count();
    let chars_per_page = char_count / page_count;

    let mut boundaries = Vec::new();
    let mut byte_offset = 0;

    for page_num in 1..=page_count {
        let start = byte_offset;

        let end = if page_num == page_count {
            text.len()
        } else {
            let remaining = &text[byte_offset..];
            let chars_to_skip = chars_per_page;
            byte_offset
                + remaining
                    .chars()
                    .take(chars_to_skip)
                    .map(|c| c.len_utf8())
                    .sum::<usize>()
        };

        byte_offset = end;

        boundaries.push(PageBoundary {
            byte_start: start,
            byte_end: end,
            page_number: page_num,
        });
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
        let breaks = vec![0];

        let result = map_page_breaks_to_boundaries(text, breaks).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].page_number, 1);
        assert_eq!(result[0].byte_start, 0);
        assert!(result[0].byte_end > 0);
        assert!(result[0].byte_end < text.len());

        assert_eq!(result[1].page_number, 2);
        assert_eq!(result[1].byte_start, result[0].byte_end);
        assert_eq!(result[1].byte_end, text.len());
    }

    #[test]
    fn test_map_page_breaks_to_boundaries_multiple_breaks() {
        let text = "A".repeat(300);
        let breaks = vec![0, 0, 0];

        let result = map_page_breaks_to_boundaries(&text, breaks).unwrap();

        assert_eq!(result.len(), 4);
        assert_eq!(result[0].page_number, 1);
        assert_eq!(result[3].page_number, 4);
        assert_eq!(result[3].byte_end, text.len());

        for i in 0..result.len() - 1 {
            assert_eq!(result[i].byte_end, result[i + 1].byte_start);
        }
    }

    #[test]
    fn test_map_page_breaks_to_boundaries_utf8_boundary() {
        let text = "Hello world! こんにちは世界！ More text here.";
        let breaks = vec![0];

        let result = map_page_breaks_to_boundaries(text, breaks).unwrap();

        assert_eq!(result.len(), 2);
        assert!(text.is_char_boundary(result[0].byte_start));
        assert!(text.is_char_boundary(result[0].byte_end));
        assert!(text.is_char_boundary(result[1].byte_start));
        assert!(text.is_char_boundary(result[1].byte_end));
    }

    #[test]
    fn test_docx_page_breaks_with_emoji() {
        let text = "Hello 😀 World 🌍 Foo 🎉 Bar";
        let breaks = vec![0, 0];

        let result = map_page_breaks_to_boundaries(text, breaks).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].page_number, 1);
        assert_eq!(result[1].page_number, 2);
        assert_eq!(result[2].page_number, 3);

        for boundary in &result {
            assert!(
                text.is_char_boundary(boundary.byte_start),
                "byte_start {} is not a valid UTF-8 boundary",
                boundary.byte_start
            );
            assert!(
                text.is_char_boundary(boundary.byte_end),
                "byte_end {} is not a valid UTF-8 boundary",
                boundary.byte_end
            );
        }

        assert_eq!(result[0].byte_start, 0);
        assert_eq!(result[0].byte_end, result[1].byte_start);
        assert_eq!(result[1].byte_end, result[2].byte_start);
        assert_eq!(result[2].byte_end, text.len());

        let reconstructed = format!(
            "{}{}{}",
            &text[result[0].byte_start..result[0].byte_end],
            &text[result[1].byte_start..result[1].byte_end],
            &text[result[2].byte_start..result[2].byte_end]
        );
        assert_eq!(reconstructed, text);
    }

    #[test]
    fn test_docx_page_breaks_with_cjk() {
        let text = "你好世界你好世界你好世界你好世界";
        let breaks = vec![0];

        let result = map_page_breaks_to_boundaries(text, breaks).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].page_number, 1);
        assert_eq!(result[1].page_number, 2);

        for boundary in &result {
            assert!(
                text.is_char_boundary(boundary.byte_start),
                "byte_start {} is not a valid UTF-8 boundary",
                boundary.byte_start
            );
            assert!(
                text.is_char_boundary(boundary.byte_end),
                "byte_end {} is not a valid UTF-8 boundary",
                boundary.byte_end
            );
        }

        assert_eq!(result[0].byte_start, 0);
        assert_eq!(result[0].byte_end, result[1].byte_start);
        assert_eq!(result[1].byte_end, text.len());

        let reconstructed = format!(
            "{}{}",
            &text[result[0].byte_start..result[0].byte_end],
            &text[result[1].byte_start..result[1].byte_end]
        );
        assert_eq!(reconstructed, text);
    }

    #[test]
    fn test_docx_page_breaks_multibyte_utf8() {
        let text = "ASCII 😀 中文 hello 🎉 world 日本語";
        let breaks = vec![0, 0];

        let result = map_page_breaks_to_boundaries(text, breaks).unwrap();

        assert_eq!(result.len(), 3);

        for boundary in &result {
            assert!(
                text.is_char_boundary(boundary.byte_start),
                "byte_start {} is not a valid UTF-8 boundary",
                boundary.byte_start
            );
            assert!(
                text.is_char_boundary(boundary.byte_end),
                "byte_end {} is not a valid UTF-8 boundary",
                boundary.byte_end
            );
        }

        assert_eq!(result[0].byte_start, 0);
        for i in 0..result.len() - 1 {
            assert_eq!(
                result[i].byte_end,
                result[i + 1].byte_start,
                "Gap or overlap between page {} and {}",
                i + 1,
                i + 2
            );
        }
        assert_eq!(
            result[result.len() - 1].byte_end,
            text.len(),
            "Last page does not end at text boundary"
        );

        let mut reconstructed = String::new();
        for boundary in &result {
            reconstructed.push_str(&text[boundary.byte_start..boundary.byte_end]);
        }
        assert_eq!(reconstructed, text);
    }

    #[test]
    fn test_detect_page_breaks_no_feature() {
        let result = detect_page_breaks(b"invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_text_with_page_breaks_no_breaks() {
        let docx_path =
            "/Users/naamanhirschfeld/workspace/kreuzberg-dev/kreuzberg/test_documents/documents/lorem_ipsum.docx";
        if let Ok(bytes) = std::fs::read(docx_path) {
            let result = extract_text_with_page_breaks(&bytes);
            if let Ok((text, boundaries)) = result {
                assert!(!text.is_empty());
                if let Some(b) = boundaries {
                    assert!(!b.is_empty());
                }
            }
        }
    }
}
