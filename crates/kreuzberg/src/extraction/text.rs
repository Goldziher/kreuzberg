use once_cell::sync::Lazy;
use regex::Regex;

use crate::error::Result;
use crate::types::TextExtractionResult;

static MARKDOWN_HEADER: Lazy<Regex> = Lazy::new(|| Regex::new(r"^#{1,6}\s*(.+)$").unwrap());
static MARKDOWN_LINK: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap());
static CODE_BLOCK_DELIMITER: Lazy<Regex> = Lazy::new(|| Regex::new(r"^```(\w*)[\r]?$").unwrap());

pub fn parse_text(text_bytes: &[u8], is_markdown: bool) -> Result<TextExtractionResult> {
    // Decode bytes to UTF-8, using lossy conversion if needed
    let text = String::from_utf8_lossy(text_bytes).into_owned();

    let mut line_count = 0;
    let mut word_count = 0;
    let character_count = text.len();

    let mut headers = Vec::new();
    let mut links = Vec::new();
    let mut code_blocks = Vec::new();
    let mut in_code_block = false;
    let mut current_code_lang = String::new();
    let mut current_code = String::new();

    for line in text.lines() {
        line_count += 1;
        word_count += line.split_whitespace().count();

        if !is_markdown {
            continue;
        }

        if CODE_BLOCK_DELIMITER.is_match(line) {
            if in_code_block {
                code_blocks.push((
                    if current_code_lang.is_empty() {
                        "plain".to_string()
                    } else {
                        current_code_lang.clone()
                    },
                    current_code.trim_end().to_string(),
                ));
                current_code.clear();
                current_code_lang.clear();
                in_code_block = false;
            } else {
                if let Some(caps) = CODE_BLOCK_DELIMITER.captures(line) {
                    current_code_lang = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                }
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            current_code.push_str(line);
            current_code.push('\n');
            continue;
        }

        if let Some(caps) = MARKDOWN_HEADER.captures(line)
            && let Some(header) = caps.get(1)
        {
            headers.push(header.as_str().to_string());
        }

        for caps in MARKDOWN_LINK.captures_iter(line) {
            if let (Some(text), Some(url)) = (caps.get(1), caps.get(2)) {
                links.push((text.as_str().to_string(), url.as_str().to_string()));
            }
        }
    }

    Ok(TextExtractionResult {
        content: text,
        line_count,
        word_count,
        character_count,
        headers: if headers.is_empty() { None } else { Some(headers) },
        links: if links.is_empty() { None } else { Some(links) },
        code_blocks: if code_blocks.is_empty() {
            None
        } else {
            Some(code_blocks)
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text_basic() {
        let text = b"Hello, World!\nThis is a test.\nThird line here.";
        let result = parse_text(text, false).unwrap();
        assert_eq!(result.content, "Hello, World!\nThis is a test.\nThird line here.");
        assert_eq!(result.line_count, 3);
        assert_eq!(result.word_count, 9);
        assert_eq!(result.character_count, result.content.len());
        assert!(result.headers.is_none());
        assert!(result.links.is_none());
        assert!(result.code_blocks.is_none());
    }

    #[test]
    fn test_plain_text_empty() {
        let text = b"";
        let result = parse_text(text, false).unwrap();
        assert_eq!(result.content, "");
        assert_eq!(result.line_count, 0);
        assert_eq!(result.word_count, 0);
        assert_eq!(result.character_count, 0);
    }

    #[test]
    fn test_markdown_headers() {
        let text = b"# Header 1\n## Header 2\n### Header 3\n#NoSpace\n## Multiple  spaces";
        let result = parse_text(text, true).unwrap();
        assert_eq!(result.line_count, 5);
        let headers = result.headers.unwrap();
        assert_eq!(headers.len(), 5);
        assert!(headers.contains(&"Header 1".to_string()));
        assert!(headers.contains(&"Header 2".to_string()));
        assert!(headers.contains(&"Header 3".to_string()));
        assert!(headers.contains(&"NoSpace".to_string()));
        assert!(headers.contains(&"Multiple  spaces".to_string()));
    }

    #[test]
    fn test_markdown_links() {
        let text =
            b"Check [Google](https://google.com) and [GitHub](https://github.com).\n[Another](https://example.com)";
        let result = parse_text(text, true).unwrap();
        let links = result.links.unwrap();
        assert_eq!(links.len(), 3);
        assert!(links.contains(&("Google".to_string(), "https://google.com".to_string())));
        assert!(links.contains(&("GitHub".to_string(), "https://github.com".to_string())));
        assert!(links.contains(&("Another".to_string(), "https://example.com".to_string())));
    }

    #[test]
    fn test_markdown_code_blocks() {
        let text = b"```python\ndef hello():\n    print(\"Hello\")\n```\n\n```javascript\nconsole.log(\"Hi\");\n```\n\n```\nplain code\n```";
        let result = parse_text(text, true).unwrap();
        let code_blocks = result.code_blocks.unwrap();
        assert_eq!(code_blocks.len(), 3);

        let python_block = code_blocks.iter().find(|(lang, _)| lang == "python").unwrap();
        assert!(python_block.1.contains("def hello()"));

        let js_block = code_blocks.iter().find(|(lang, _)| lang == "javascript").unwrap();
        assert!(js_block.1.contains("console.log"));

        let plain_block = code_blocks.iter().find(|(lang, _)| lang == "plain").unwrap();
        assert!(plain_block.1.contains("plain code"));
    }

    #[test]
    fn test_markdown_code_blocks_crlf() {
        let text = b"```python\r\ndef hello():\r\n    print(\"Hello\")\r\n```\r\n";
        let result = parse_text(text, true).unwrap();
        let code_blocks = result.code_blocks.unwrap();
        assert_eq!(code_blocks.len(), 1);
        assert_eq!(code_blocks[0].0, "python");
        assert!(code_blocks[0].1.contains("def hello()"));
    }

    #[test]
    fn test_markdown_complex() {
        let text = b"# Documentation\n\n## Overview\nThis is a [test](https://example.com).\n\n```python\nx = 42\n```\n\n## Another\nMore [links](https://test.com).";
        let result = parse_text(text, true).unwrap();
        assert!(result.line_count > 0);
        assert!(result.word_count > 0);

        let headers = result.headers.unwrap();
        assert_eq!(headers.len(), 3);

        let links = result.links.unwrap();
        assert_eq!(links.len(), 2);

        let code_blocks = result.code_blocks.unwrap();
        assert_eq!(code_blocks.len(), 1);
    }

    #[test]
    fn test_unicode_content() {
        let text = "Hello 世界 🌍\nUnicode test".as_bytes();
        let result = parse_text(text, false).unwrap();
        assert!(result.content.contains("世界"));
        assert!(result.content.contains("🌍"));
        assert_eq!(result.line_count, 2);
    }

    #[test]
    fn test_word_count_accuracy() {
        let text = b"One two three four five.\nSix seven eight.\nNine.";
        let result = parse_text(text, false).unwrap();
        assert_eq!(result.line_count, 3);
        assert_eq!(result.word_count, 9);
    }

    #[test]
    fn test_headers_not_in_code_blocks() {
        let text = b"# Real Header\n```\n# Not a header\n```\n## Another Real";
        let result = parse_text(text, true).unwrap();
        let headers = result.headers.unwrap();
        assert_eq!(headers.len(), 2);
        assert!(headers.contains(&"Real Header".to_string()));
        assert!(headers.contains(&"Another Real".to_string()));
        assert!(!headers.iter().any(|h| h.contains("Not a header")));
    }

    #[test]
    fn test_links_not_in_code_blocks() {
        let text = b"[Real Link](https://real.com)\n```\n[Not Link](https://fake.com)\n```";
        let result = parse_text(text, true).unwrap();
        let links = result.links.unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].0, "Real Link");
        assert_eq!(links[0].1, "https://real.com");
    }

    #[test]
    fn test_empty_code_block_language() {
        let text = b"```\ncode without language\n```";
        let result = parse_text(text, true).unwrap();
        let code_blocks = result.code_blocks.unwrap();
        assert_eq!(code_blocks.len(), 1);
        assert_eq!(code_blocks[0].0, "plain");
    }

    #[test]
    fn test_large_text_streaming() {
        let large_text = "Line\n".repeat(10000);
        let result = parse_text(large_text.as_bytes(), false).unwrap();
        assert_eq!(result.line_count, 10000);
        assert_eq!(result.word_count, 10000);
    }
}
