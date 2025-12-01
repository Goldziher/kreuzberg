//! PDF table extraction using pdfium character positions.
//!
//! This module converts pdfium character data to HocrWord format,
//! allowing us to reuse the existing table reconstruction logic.

use super::error::{PdfError, Result};
use crate::ocr::table::HocrWord;
use pdfium_render::prelude::*;

/// Spacing threshold for word boundary detection (in PDF units).
///
/// Characters separated by more than this distance are considered separate words.
const WORD_SPACING_THRESHOLD: f32 = 3.0;

/// Minimum word length for table detection (filter out noise).
const MIN_WORD_LENGTH: usize = 1;

/// Extract words with positions from PDF page for table detection.
///
/// Groups adjacent characters into words based on spacing heuristics,
/// then converts to HocrWord format for table reconstruction.
///
/// # Arguments
///
/// * `page` - PDF page to extract words from
/// * `min_confidence` - Minimum confidence threshold (0.0-100.0). PDF text has high confidence (95.0).
///
/// # Returns
///
/// Vector of HocrWord objects with text and bounding box information.
///
/// # Example
///
/// ```rust,no_run
/// use kreuzberg::pdf::table::extract_words_from_page;
/// use pdfium_render::prelude::*;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let pdfium = Pdfium::default();
/// let document = pdfium.load_pdf_from_file("example.pdf", None)?;
/// let page = document.pages().get(0)?;
/// let words = extract_words_from_page(&page, 90.0)?;
/// # Ok(())
/// # }
/// ```
pub fn extract_words_from_page(page: &PdfPage, min_confidence: f64) -> Result<Vec<HocrWord>> {
    // Get page dimensions for coordinate system
    let page_width = page.width().value as i32;
    let page_height = page.height().value as i32;

    // Get all text from page
    let page_text = page
        .text()
        .map_err(|e| PdfError::TextExtractionFailed(format!("Failed to get page text: {}", e)))?;

    // Extract character-level information
    let chars = page_text.chars();

    // Group characters into words based on spacing
    let words = group_chars_into_words(chars, page_width, page_height, min_confidence)?;

    Ok(words)
}

/// Character with position information extracted from PDF.
#[derive(Debug, Clone)]
struct CharInfo {
    text: char,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

/// Group PDF characters into words based on spacing heuristics.
///
/// Characters are grouped into the same word if they are:
/// 1. On the same horizontal line (similar y-coordinate)
/// 2. Close together horizontally (spacing < WORD_SPACING_THRESHOLD)
///
/// # Arguments
///
/// * `chars` - Iterator of PDF page characters
/// * `page_width` - Page width in PDF units
/// * `page_height` - Page height in PDF units
/// * `min_confidence` - Minimum confidence threshold (PDF text uses 95.0)
fn group_chars_into_words(
    chars: PdfPageTextChars,
    _page_width: i32,
    page_height: i32,
    min_confidence: f64,
) -> Result<Vec<HocrWord>> {
    let mut words: Vec<HocrWord> = Vec::new();
    let mut current_word_chars: Vec<CharInfo> = Vec::new();

    for pdf_char in chars.iter() {
        // Get character bounds (use loose_bounds for table detection)
        let bounds = pdf_char
            .loose_bounds()
            .map_err(|e| PdfError::TextExtractionFailed(format!("Failed to get char bounds: {}", e)))?;

        // Get unicode character (skip if invalid)
        let Some(ch) = pdf_char.unicode_char() else {
            continue;
        };

        // Extract character information
        let char_info = CharInfo {
            text: ch,
            x: bounds.left().value,
            y: bounds.bottom().value, // PDF coordinates: bottom-left origin
            width: bounds.width().value,
            height: bounds.height().value,
        };

        // Skip whitespace characters (they're used for word boundaries)
        if char_info.text.is_whitespace() {
            if !current_word_chars.is_empty() {
                if let Some(word) = finalize_word(&current_word_chars, page_height, min_confidence) {
                    words.push(word);
                }
                current_word_chars.clear();
            }
            continue;
        }

        // Check if this character should start a new word
        if should_start_new_word(&current_word_chars, &char_info) && !current_word_chars.is_empty() {
            if let Some(word) = finalize_word(&current_word_chars, page_height, min_confidence) {
                words.push(word);
            }
            current_word_chars.clear();
        }

        current_word_chars.push(char_info);
    }

    // Finalize last word
    if !current_word_chars.is_empty()
        && let Some(word) = finalize_word(&current_word_chars, page_height, min_confidence)
    {
        words.push(word);
    }

    Ok(words)
}

/// Determine if a new character should start a new word.
///
/// Returns true if the character is far from the previous character
/// (indicating a word boundary) or on a different line.
fn should_start_new_word(current_word_chars: &[CharInfo], new_char: &CharInfo) -> bool {
    if current_word_chars.is_empty() {
        return false;
    }

    let last_char = &current_word_chars[current_word_chars.len() - 1];

    // Check vertical distance (different lines)
    let vertical_distance = (new_char.y - last_char.y).abs();
    if vertical_distance > last_char.height * 0.5 {
        return true;
    }

    // Check horizontal distance (word spacing)
    let horizontal_gap = new_char.x - (last_char.x + last_char.width);
    horizontal_gap > WORD_SPACING_THRESHOLD
}

/// Convert a group of characters into a HocrWord.
///
/// Calculates bounding box and confidence for the word.
/// Returns None if the word doesn't meet minimum criteria.
fn finalize_word(chars: &[CharInfo], page_height: i32, min_confidence: f64) -> Option<HocrWord> {
    if chars.is_empty() {
        return None;
    }

    // Build word text
    let text: String = chars.iter().map(|c| c.text).collect();

    if text.len() < MIN_WORD_LENGTH {
        return None;
    }

    // Calculate bounding box (encompassing all characters)
    let left = chars
        .iter()
        .map(|c| c.x)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);
    let right = chars
        .iter()
        .map(|c| c.x + c.width)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);
    let bottom = chars
        .iter()
        .map(|c| c.y)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);
    let top = chars
        .iter()
        .map(|c| c.y + c.height)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);

    let width = (right - left).round() as i32;
    let height = (top - bottom).round() as i32;

    // Convert PDF coordinates (bottom-left origin) to image coordinates (top-left origin)
    // HocrWord expects top-left origin like images/OCR output
    let top_in_image_coords = (page_height as f32 - top).round() as i32;

    // PDF text has high confidence (no OCR uncertainty)
    let confidence = 95.0;

    // Apply confidence threshold
    if confidence < min_confidence {
        return None;
    }

    Some(HocrWord {
        text,
        left: left.round().max(0.0) as u32,
        top: top_in_image_coords.max(0) as u32,
        width: width.max(0) as u32,
        height: height.max(0) as u32,
        confidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_info_creation() {
        let char_info = CharInfo {
            text: 'A',
            x: 100.0,
            y: 50.0,
            width: 10.0,
            height: 12.0,
        };

        assert_eq!(char_info.text, 'A');
        assert_eq!(char_info.x, 100.0);
        assert_eq!(char_info.width, 10.0);
    }

    #[test]
    fn test_should_start_new_word_empty() {
        let chars: Vec<CharInfo> = vec![];
        let new_char = CharInfo {
            text: 'A',
            x: 100.0,
            y: 50.0,
            width: 10.0,
            height: 12.0,
        };

        assert!(!should_start_new_word(&chars, &new_char));
    }

    #[test]
    fn test_should_start_new_word_spacing() {
        let chars = vec![CharInfo {
            text: 'A',
            x: 100.0,
            y: 50.0,
            width: 10.0,
            height: 12.0,
        }];

        // Close character - same word
        let close_char = CharInfo {
            text: 'B',
            x: 111.0, // 1 unit gap
            y: 50.0,
            width: 10.0,
            height: 12.0,
        };
        assert!(!should_start_new_word(&chars, &close_char));

        // Far character - new word
        let far_char = CharInfo {
            text: 'C',
            x: 120.0, // 10 unit gap (> WORD_SPACING_THRESHOLD)
            y: 50.0,
            width: 10.0,
            height: 12.0,
        };
        assert!(should_start_new_word(&chars, &far_char));
    }

    #[test]
    fn test_should_start_new_word_different_line() {
        let chars = vec![CharInfo {
            text: 'A',
            x: 100.0,
            y: 50.0,
            width: 10.0,
            height: 12.0,
        }];

        // Character on different line
        let new_line_char = CharInfo {
            text: 'B',
            x: 100.0,
            y: 70.0, // Different y
            width: 10.0,
            height: 12.0,
        };
        assert!(should_start_new_word(&chars, &new_line_char));
    }

    #[test]
    fn test_finalize_word_basic() {
        let chars = vec![
            CharInfo {
                text: 'H',
                x: 100.0,
                y: 50.0,
                width: 10.0,
                height: 12.0,
            },
            CharInfo {
                text: 'i',
                x: 110.0,
                y: 50.0,
                width: 8.0,
                height: 12.0,
            },
        ];

        let page_height = 800;
        let word = finalize_word(&chars, page_height, 0.0).unwrap();

        assert_eq!(word.text, "Hi");
        assert_eq!(word.left, 100);
        assert_eq!(word.width, 18); // 110 + 8 - 100
        assert_eq!(word.height, 12);
        assert_eq!(word.confidence, 95.0);
    }

    #[test]
    fn test_finalize_word_empty() {
        let chars: Vec<CharInfo> = vec![];
        let word = finalize_word(&chars, 800, 0.0);
        assert!(word.is_none());
    }

    #[test]
    fn test_finalize_word_confidence_filter() {
        let chars = vec![CharInfo {
            text: 'A',
            x: 100.0,
            y: 50.0,
            width: 10.0,
            height: 12.0,
        }];

        // Low threshold - should pass
        let word = finalize_word(&chars, 800, 90.0);
        assert!(word.is_some());

        // High threshold - should fail
        let word = finalize_word(&chars, 800, 96.0);
        assert!(word.is_none());
    }

    #[test]
    fn test_coordinate_conversion() {
        // Test PDF coordinate (bottom-left origin) to image coordinate (top-left origin)
        let chars = vec![CharInfo {
            text: 'A',
            x: 100.0,
            y: 700.0, // PDF coordinates: bottom-left origin
            width: 10.0,
            height: 12.0,
        }];

        let page_height = 800;
        let word = finalize_word(&chars, page_height, 0.0).unwrap();

        // top_in_image_coords = page_height - (y + height) = 800 - (700 + 12) = 88
        assert_eq!(word.top, 88);
    }

    #[test]
    fn test_word_bounding_box() {
        // Test that bounding box encompasses all characters
        let chars = vec![
            CharInfo {
                text: 'A',
                x: 100.0,
                y: 50.0,
                width: 10.0,
                height: 12.0,
            },
            CharInfo {
                text: 'B',
                x: 110.0,
                y: 51.0, // Slightly different y
                width: 10.0,
                height: 13.0, // Slightly different height
            },
        ];

        let word = finalize_word(&chars, 800, 0.0).unwrap();

        // Left should be minimum x
        assert_eq!(word.left, 100);

        // Width should span from leftmost to rightmost character
        assert_eq!(word.width, 20); // 120 - 100

        // Height should encompass both characters
        // max(y+height) - min(y) = max(51+13, 50+12) - 50 = 64 - 50 = 14
        assert_eq!(word.height, 14);
    }
}
