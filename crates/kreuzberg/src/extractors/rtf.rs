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

/// Extract text from RTF document using simple parsing approach.
///
/// This function extracts plain text from an RTF document by:
/// 1. Tokenizing control sequences and text
/// 2. Converting encoded characters to Unicode
/// 3. Extracting text while skipping formatting groups
/// 4. Normalizing whitespace
fn extract_text_from_rtf(content: &str) -> String {
    let mut result = String::new();
    let mut chars = content.chars().peekable();
    let mut skip_next_char = false;

    while let Some(ch) = chars.next() {
        if skip_next_char {
            skip_next_char = false;
            continue;
        }

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
                            // Regular control word - skip until next whitespace or control char
                            while let Some(&c) = chars.peek() {
                                if !c.is_alphanumeric() {
                                    break;
                                }
                                chars.next();
                            }
                            // Skip one trailing digit if present (for parameterized control words)
                            if let Some(&c) = chars.peek()
                                && (c.is_ascii_digit() || c == '-')
                            {
                                chars.next();
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
        let extractor = RtfExtractor;
        let rtf_content = r#"{\rtf1 Hello World}"#;
        let extracted = extract_text_from_rtf(rtf_content);
        assert!(extracted.contains("Hello") || extracted.contains("World"));
    }
}
