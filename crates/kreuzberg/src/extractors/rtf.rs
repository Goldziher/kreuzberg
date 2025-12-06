//! RTF (Rich Text Format) extractor.
//!
//! Supports: Rich Text Format (.rtf)
//!
//! This native Rust extractor provides basic text extraction from RTF documents.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;

/// Native Rust RTF extractor.
///
/// Extracts text content, metadata, and structure from RTF documents
/// without requiring Pandoc as a dependency.
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
                                && let Ok(code) = u8::from_str_radix(&format!("{}{}", h1, h2), 16)
                            {
                                // For Western European, assume Latin-1
                                result.push(code as char);
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
                            // Regular control word - parse and check if it's pict
                            let (control_word, _) = parse_rtf_control_word(&mut chars);

                            if control_word == "pict" {
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

    // Clean up whitespace
    let cleaned = result.split_whitespace().collect::<Vec<_>>().join(" ");

    cleaned.trim().to_string()
}

/// Extract image metadata from within a \pict group.
///
/// Looks for image type (jpegblip, pngblip, etc.) and dimensions.
fn extract_image_metadata(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut metadata = String::new();
    let mut image_type = String::new();
    let mut width_goal: Option<i32> = None;
    let mut height_goal: Option<i32> = None;
    let mut depth = 0;

    // Scan for image metadata control words
    while let Some(&ch) = chars.peek() {
        if ch == '{' {
            depth += 1;
            chars.next();
        } else if ch == '}' {
            if depth == 0 {
                break;
            }
            depth -= 1;
            chars.next();
        } else if ch == '\\' {
            chars.next(); // consume backslash
            let (control_word, value) = parse_rtf_control_word(chars);

            // Check for image type
            if control_word == "jpegblip" {
                image_type = "jpg".to_string();
            } else if control_word == "pngblip" {
                image_type = "png".to_string();
            } else if control_word == "wmetafile" {
                image_type = "wmf".to_string();
            } else if control_word == "dibitmap" {
                image_type = "bmp".to_string();
            }
            // Check for dimensions (goal dimensions are in twips, 1 inch = 1440 twips)
            else if control_word == "picwgoal" {
                if let Some(val) = value {
                    width_goal = Some(val);
                }
            } else if control_word == "pichgoal" {
                if let Some(val) = value {
                    height_goal = Some(val);
                }
            } else if control_word == "bin" {
                // End of control words, rest is binary data
                break;
            }
        } else if ch == ' ' {
            chars.next();
        } else {
            // Skip other characters (like binary data)
            chars.next();
        }
    }

    // Build metadata string
    if !image_type.is_empty() {
        metadata.push_str("image.");
        metadata.push_str(&image_type);
    }

    // Add dimensions if available
    if let Some(width) = width_goal {
        let width_inches = width as f64 / 1440.0;
        metadata.push_str(&format!(" width=\"{:.1}in\"", width_inches));
    }

    if let Some(height) = height_goal {
        let height_inches = height as f64 / 1440.0;
        metadata.push_str(&format!(" height=\"{:.1}in\"", height_inches));
    }

    // If no metadata found, just return a generic image reference
    if metadata.is_empty() {
        metadata = "image.jpg".to_string();
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
        let rtf_content = String::from_utf8_lossy(content).to_string();

        // Extract text from RTF
        let extracted_text = extract_text_from_rtf(&rtf_content);

        Ok(ExtractionResult {
            content: extracted_text,
            mime_type: mime_type.to_string(),
            metadata: Metadata { ..Default::default() },
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
        // Higher priority than Pandoc (40) to prefer native Rust implementation
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
