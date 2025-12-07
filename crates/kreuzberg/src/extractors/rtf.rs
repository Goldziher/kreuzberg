//! RTF (Rich Text Format) extractor.
//!
//! Supports: Rich Text Format (.rtf)
//!
//! This native Rust extractor provides text extraction from RTF documents with:
//! - Character encoding support (Windows-1252 for 0x80-0x9F range)
//! - Common RTF control words (paragraph breaks, tabs, bullets, quotes, dashes)
//! - Unicode escape sequences
//! - Image metadata extraction
//! - Whitespace normalization

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;

/// Native Rust RTF extractor.
///
/// Extracts text content, metadata, and structure from RTF documents
pub struct RtfExtractor;

impl RtfExtractor {
    /// Create a new RTF extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for RtfExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RtfExtractor {
    fn name(&self) -> &str {
        "rtf-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Extracts content from RTF (Rich Text Format) files with native Rust parsing"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

/// Convert a hex digit character to its numeric value.
///
/// Returns None if the character is not a valid hex digit.
#[inline]
fn hex_digit_to_u8(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some((c as u8) - b'0'),
        'a'..='f' => Some((c as u8) - b'a' + 10),
        'A'..='F' => Some((c as u8) - b'A' + 10),
        _ => None,
    }
}

/// Parse a hex-encoded byte from two characters.
///
/// Returns the decoded byte if both characters are valid hex digits.
#[inline]
fn parse_hex_byte(h1: char, h2: char) -> Option<u8> {
    let high = hex_digit_to_u8(h1)?;
    let low = hex_digit_to_u8(h2)?;
    Some((high << 4) | low)
}

/// Parse an RTF control word and extract its value.
///
/// Returns a tuple of (control_word, optional_numeric_value)
fn parse_rtf_control_word(chars: &mut std::iter::Peekable<std::str::Chars>) -> (String, Option<i32>) {
    let mut word = String::new();
    let mut num_str = String::new();
    let mut is_negative = false;

    // Read the control word
    while let Some(&c) = chars.peek() {
        if c.is_alphabetic() {
            word.push(c);
            chars.next();
        } else {
            break;
        }
    }

    // Read optional numeric parameter
    if let Some(&c) = chars.peek()
        && c == '-'
    {
        is_negative = true;
        chars.next();
    }

    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            num_str.push(c);
            chars.next();
        } else {
            break;
        }
    }

    let num_value = if !num_str.is_empty() {
        let val = num_str.parse::<i32>().unwrap_or(0);
        Some(if is_negative { -val } else { val })
    } else {
        None
    };

    (word, num_value)
}

/// Extract text and image metadata from RTF document.
///
/// This function extracts plain text from an RTF document by:
/// 1. Tokenizing control sequences and text
/// 2. Converting encoded characters to Unicode
/// 3. Extracting text while skipping formatting groups
/// 4. Detecting and extracting image metadata (\pict sections)
/// 5. Normalizing whitespace
fn extract_text_from_rtf(content: &str) -> String {
    let mut result = String::new();
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                // Handle RTF control sequences
                if let Some(&next_ch) = chars.peek() {
                    match next_ch {
                        '\\' | '{' | '}' => {
                            // Escaped character
                            chars.next();
                            result.push(next_ch);
                        }
                        '\'' => {
                            // Hex-encoded character like \'e9
                            chars.next(); // consume '
                            let hex1 = chars.next();
                            let hex2 = chars.next();
                            if let (Some(h1), Some(h2)) = (hex1, hex2)
                                && let Some(byte) = parse_hex_byte(h1, h2)
                            {
                                // Use Windows-1252 decoding for bytes 0x80-0x9F
                                // For other bytes, treat as direct character mapping
                                let decoded = match byte {
                                    0x80 => '\u{20AC}', // Euro sign
                                    0x81 => '?',        // Undefined
                                    0x82 => '\u{201A}', // Single low quote
                                    0x83 => '\u{0192}', // Florin
                                    0x84 => '\u{201E}', // Double low quote
                                    0x85 => '\u{2026}', // Ellipsis
                                    0x86 => '\u{2020}', // Dagger
                                    0x87 => '\u{2021}', // Double dagger
                                    0x88 => '\u{02C6}', // Caret
                                    0x89 => '\u{2030}', // Per mille
                                    0x8A => '\u{0160}', // S caron
                                    0x8B => '\u{2039}', // Single angle quote left
                                    0x8C => '\u{0152}', // OE ligature
                                    0x8D => '?',        // Undefined
                                    0x8E => '\u{017D}', // Z caron
                                    0x8F => '?',        // Undefined
                                    0x90 => '?',        // Undefined
                                    0x91 => '\u{2018}', // Left single quote
                                    0x92 => '\u{2019}', // Right single quote
                                    0x93 => '\u{201C}', // Left double quote
                                    0x94 => '\u{201D}', // Right double quote
                                    0x95 => '\u{2022}', // Bullet
                                    0x96 => '\u{2013}', // En dash
                                    0x97 => '\u{2014}', // Em dash
                                    0x98 => '\u{02DC}', // Tilde
                                    0x99 => '\u{2122}', // Trademark
                                    0x9A => '\u{0161}', // s caron
                                    0x9B => '\u{203A}', // Single angle quote right
                                    0x9C => '\u{0153}', // oe ligature
                                    0x9D => '?',        // Undefined
                                    0x9E => '\u{017E}', // z caron
                                    0x9F => '\u{0178}', // Y diaeresis
                                    _ => byte as char,  // Latin-1 for 0x00-0x7F and 0xA0-0xFF
                                };
                                result.push(decoded);
                            }
                        }
                        'u' => {
                            // Unicode escape like \uXXXX
                            chars.next(); // consume 'u'
                            let mut num_str = String::new();
                            while let Some(&c) = chars.peek() {
                                if c.is_ascii_digit() || c == '-' {
                                    num_str.push(c);
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                            if let Ok(code_num) = num_str.parse::<i32>() {
                                let code_u = if code_num < 0 {
                                    (code_num + 65536) as u32
                                } else {
                                    code_num as u32
                                };
                                if let Some(c) = char::from_u32(code_u) {
                                    result.push(c);
                                }
                            }
                        }
                        _ => {
                            // Regular control word - parse and check special control words
                            let (control_word, _) = parse_rtf_control_word(&mut chars);

                            match control_word.as_str() {
                                "pict" => {
                                    // Found an image! Extract image metadata
                                    let image_metadata = extract_image_metadata(&mut chars);
                                    if !image_metadata.is_empty() {
                                        result.push('!');
                                        result.push('[');
                                        result.push_str("image");
                                        result.push(']');
                                        result.push('(');
                                        result.push_str(&image_metadata);
                                        result.push(')');
                                        result.push(' ');
                                    }
                                }
                                "par" => {
                                    // Paragraph break
                                    if !result.is_empty() && !result.ends_with('\n') {
                                        result.push('\n');
                                        result.push('\n');
                                    }
                                }
                                "tab" => {
                                    // Tab character
                                    result.push('\t');
                                }
                                "bullet" => {
                                    // Bullet character
                                    result.push('•');
                                }
                                "lquote" => {
                                    // Left single quote
                                    result.push('\u{2018}');
                                }
                                "rquote" => {
                                    // Right single quote
                                    result.push('\u{2019}');
                                }
                                "ldblquote" => {
                                    // Left double quote
                                    result.push('\u{201C}');
                                }
                                "rdblquote" => {
                                    // Right double quote
                                    result.push('\u{201D}');
                                }
                                "endash" => {
                                    // En dash
                                    result.push('\u{2013}');
                                }
                                "emdash" => {
                                    // Em dash
                                    result.push('\u{2014}');
                                }
                                _ => {
                                    // Unknown control word - skip it
                                }
                            }
                        }
                    }
                }
            }
            '{' | '}' => {
                // Group delimiters - just add space
                if !result.is_empty() && !result.ends_with(' ') {
                    result.push(' ');
                }
            }
            ' ' | '\t' | '\n' | '\r' => {
                // Whitespace
                if !result.is_empty() && !result.ends_with(' ') {
                    result.push(' ');
                }
            }
            _ => {
                // Regular character
                result.push(ch);
            }
        }
    }

    // Clean up whitespace using single-pass algorithm
    normalize_whitespace(&result)
}

/// Normalize whitespace in a string using a single-pass algorithm.
///
/// Collapses multiple consecutive whitespace characters into single spaces
/// and trims leading/trailing whitespace.
fn normalize_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut last_was_space = false;

    for ch in s.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(ch);
            last_was_space = false;
        }
    }

    result.trim().to_string()
}

/// Extract image metadata from within a \pict group.
///
/// Looks for image type (jpegblip, pngblip, etc.) and dimensions.
fn extract_image_metadata(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut metadata = String::new();
    let mut image_type: Option<&str> = None;
    let mut width_goal: Option<i32> = None;
    let mut height_goal: Option<i32> = None;
    let mut depth = 0;

    // Scan for image metadata control words
    while let Some(&ch) = chars.peek() {
        match ch {
            '{' => {
                depth += 1;
                chars.next();
            }
            '}' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                chars.next();
            }
            '\\' => {
                chars.next(); // consume backslash
                let (control_word, value) = parse_rtf_control_word(chars);

                // Check for image type
                match control_word.as_str() {
                    "jpegblip" => image_type = Some("jpg"),
                    "pngblip" => image_type = Some("png"),
                    "wmetafile" => image_type = Some("wmf"),
                    "dibitmap" => image_type = Some("bmp"),
                    "picwgoal" => width_goal = value,
                    "pichgoal" => height_goal = value,
                    "bin" => break, // End of control words, rest is binary data
                    _ => {}
                }
            }
            ' ' => {
                chars.next();
            }
            _ => {
                // Skip other characters (like binary data)
                chars.next();
            }
        }
    }

    // Build metadata string
    if let Some(itype) = image_type {
        metadata.push_str("image.");
        metadata.push_str(itype);
    }

    // Add dimensions if available
    // Goal dimensions are in twips, 1 inch = 1440 twips
    if let Some(width) = width_goal {
        let width_inches = f64::from(width) / 1440.0;
        metadata.push_str(&format!(" width=\"{:.1}in\"", width_inches));
    }

    if let Some(height) = height_goal {
        let height_inches = f64::from(height) / 1440.0;
        metadata.push_str(&format!(" height=\"{:.1}in\"", height_inches));
    }

    // If no metadata found, just return a generic image reference
    if metadata.is_empty() {
        metadata.push_str("image.jpg");
    }

    metadata
}

#[async_trait]
impl DocumentExtractor for RtfExtractor {
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, _config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        // Convert bytes to string for RTF processing
        let rtf_content = String::from_utf8_lossy(content);

        // Extract text from RTF
        let extracted_text = extract_text_from_rtf(&rtf_content);

        Ok(ExtractionResult {
            content: extracted_text,
            mime_type: mime_type.to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/rtf", "text/rtf"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rtf_extractor_plugin_interface() {
        let extractor = RtfExtractor::new();
        assert_eq!(extractor.name(), "rtf-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert!(extractor.supported_mime_types().contains(&"application/rtf"));
        assert_eq!(extractor.priority(), 50);
    }

    #[test]
    fn test_simple_rtf_extraction() {
        let _extractor = RtfExtractor;
        let rtf_content = r#"{\rtf1 Hello World}"#;
        let extracted = extract_text_from_rtf(rtf_content);
        assert!(extracted.contains("Hello") || extracted.contains("World"));
    }
}
