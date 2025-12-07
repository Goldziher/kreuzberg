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

    while let Some(&c) = chars.peek() {
        if c.is_alphabetic() {
            word.push(c);
            chars.next();
        } else {
            break;
        }
    }

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
                if let Some(&next_ch) = chars.peek() {
                    match next_ch {
                        '\\' | '{' | '}' => {
                            chars.next();
                            result.push(next_ch);
                        }
                        '\'' => {
                            chars.next();
                            let hex1 = chars.next();
                            let hex2 = chars.next();
                            if let (Some(h1), Some(h2)) = (hex1, hex2)
                                && let Some(byte) = parse_hex_byte(h1, h2)
                            {
                                let decoded = match byte {
                                    0x80 => '\u{20AC}',
                                    0x81 => '?',
                                    0x82 => '\u{201A}',
                                    0x83 => '\u{0192}',
                                    0x84 => '\u{201E}',
                                    0x85 => '\u{2026}',
                                    0x86 => '\u{2020}',
                                    0x87 => '\u{2021}',
                                    0x88 => '\u{02C6}',
                                    0x89 => '\u{2030}',
                                    0x8A => '\u{0160}',
                                    0x8B => '\u{2039}',
                                    0x8C => '\u{0152}',
                                    0x8D => '?',
                                    0x8E => '\u{017D}',
                                    0x8F => '?',
                                    0x90 => '?',
                                    0x91 => '\u{2018}',
                                    0x92 => '\u{2019}',
                                    0x93 => '\u{201C}',
                                    0x94 => '\u{201D}',
                                    0x95 => '\u{2022}',
                                    0x96 => '\u{2013}',
                                    0x97 => '\u{2014}',
                                    0x98 => '\u{02DC}',
                                    0x99 => '\u{2122}',
                                    0x9A => '\u{0161}',
                                    0x9B => '\u{203A}',
                                    0x9C => '\u{0153}',
                                    0x9D => '?',
                                    0x9E => '\u{017E}',
                                    0x9F => '\u{0178}',
                                    _ => byte as char,
                                };
                                result.push(decoded);
                            }
                        }
                        'u' => {
                            chars.next();
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
                            let (control_word, _) = parse_rtf_control_word(&mut chars);

                            match control_word.as_str() {
                                "pict" => {
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
                                    if !result.is_empty() && !result.ends_with('\n') {
                                        result.push('\n');
                                        result.push('\n');
                                    }
                                }
                                "tab" => {
                                    result.push('\t');
                                }
                                "bullet" => {
                                    result.push('•');
                                }
                                "lquote" => {
                                    result.push('\u{2018}');
                                }
                                "rquote" => {
                                    result.push('\u{2019}');
                                }
                                "ldblquote" => {
                                    result.push('\u{201C}');
                                }
                                "rdblquote" => {
                                    result.push('\u{201D}');
                                }
                                "endash" => {
                                    result.push('\u{2013}');
                                }
                                "emdash" => {
                                    result.push('\u{2014}');
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            '{' | '}' => {
                if !result.is_empty() && !result.ends_with(' ') {
                    result.push(' ');
                }
            }
            ' ' | '\t' | '\n' | '\r' => {
                if !result.is_empty() && !result.ends_with(' ') {
                    result.push(' ');
                }
            }
            _ => {
                result.push(ch);
            }
        }
    }

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
                chars.next();
                let (control_word, value) = parse_rtf_control_word(chars);

                match control_word.as_str() {
                    "jpegblip" => image_type = Some("jpg"),
                    "pngblip" => image_type = Some("png"),
                    "wmetafile" => image_type = Some("wmf"),
                    "dibitmap" => image_type = Some("bmp"),
                    "picwgoal" => width_goal = value,
                    "pichgoal" => height_goal = value,
                    "bin" => break,
                    _ => {}
                }
            }
            ' ' => {
                chars.next();
            }
            _ => {
                chars.next();
            }
        }
    }

    if let Some(itype) = image_type {
        metadata.push_str("image.");
        metadata.push_str(itype);
    }

    if let Some(width) = width_goal {
        let width_inches = f64::from(width) / 1440.0;
        metadata.push_str(&format!(" width=\"{:.1}in\"", width_inches));
    }

    if let Some(height) = height_goal {
        let height_inches = f64::from(height) / 1440.0;
        metadata.push_str(&format!(" height=\"{:.1}in\"", height_inches));
    }

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
        let rtf_content = String::from_utf8_lossy(content);

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
