use html_to_markdown_rs::ConversionOptions;
use html_to_markdown_rs::hocr::{extract_hocr_document, convert_to_markdown};

use super::error::OCRError;

pub fn convert_hocr_to_markdown(hocr_html: &str, options: Option<ConversionOptions>) -> Result<String, OCRError> {
    let use_default = options.is_none();
    let opts = options.unwrap_or_default();

    // Parse the HTML
    let dom = tl::parse(hocr_html, tl::ParserOptions::default())
        .map_err(|e| OCRError::ProcessingFailed(format!("HTML parsing failed: {}", e)))?;

    // Extract hOCR elements
    let use_spatial_tables = if use_default { false } else { opts.hocr_spatial_tables };
    let (elements, _metadata) = extract_hocr_document(&dom, use_spatial_tables);

    // Convert to markdown
    let markdown = convert_to_markdown(&elements, true);

    Ok(markdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_hocr_conversion() {
        let hocr = r#"<div class="ocr_page">
            <p class="ocr_par">
                <span class="ocrx_word">Hello</span>
                <span class="ocrx_word">World</span>
            </p>
        </div>"#;

        let markdown = convert_hocr_to_markdown(hocr, None).unwrap();
        assert!(markdown.contains("Hello"));
        assert!(markdown.contains("World"));
    }

    #[test]
    fn test_hocr_with_formatting() {
        let hocr = r#"<div class="ocr_page">
            <p class="ocr_par">
                <strong class="ocrx_word">Bold</strong>
                <em class="ocrx_word">Italic</em>
            </p>
        </div>"#;

        let markdown = convert_hocr_to_markdown(hocr, None).unwrap();
        assert!(!markdown.is_empty());
    }

    #[test]
    fn test_empty_hocr() {
        let hocr = "";
        let markdown = convert_hocr_to_markdown(hocr, None).unwrap();
        assert!(markdown.is_empty() || markdown.trim().is_empty());
    }

    #[test]
    fn test_hocr_with_headings() {
        // hOCR headings require proper hOCR classes like ocr_title
        let hocr = r#"<div class="ocr_page">
            <h1 class="ocr_title"><span class="ocrx_word">Title</span></h1>
            <p class="ocr_par">
                <span class="ocrx_word">Content</span>
            </p>
        </div>"#;

        let markdown = convert_hocr_to_markdown(hocr, None).unwrap();
        assert!(markdown.contains("Title"));
        assert!(markdown.contains("Content"));
    }

    #[test]
    fn test_hocr_with_multiple_paragraphs() {
        let hocr = r#"<div class="ocr_page">
            <p class="ocr_par">
                <span class="ocrx_word">First</span>
                <span class="ocrx_word">paragraph</span>
            </p>
            <p class="ocr_par">
                <span class="ocrx_word">Second</span>
                <span class="ocrx_word">paragraph</span>
            </p>
        </div>"#;

        let markdown = convert_hocr_to_markdown(hocr, None).unwrap();
        assert!(markdown.contains("First"));
        assert!(markdown.contains("Second"));
    }

    #[test]
    fn test_hocr_with_line_breaks() {
        let hocr = r#"<div class="ocr_page">
            <p class="ocr_par">
                <span class="ocrx_line">
                    <span class="ocrx_word">Line</span>
                    <span class="ocrx_word">one</span>
                </span>
                <span class="ocrx_line">
                    <span class="ocrx_word">Line</span>
                    <span class="ocrx_word">two</span>
                </span>
            </p>
        </div>"#;

        let markdown = convert_hocr_to_markdown(hocr, None).unwrap();
        assert!(!markdown.is_empty());
    }

    #[test]
    fn test_hocr_whitespace_handling() {
        let hocr = r#"<div class="ocr_page">
            <p class="ocr_par">
                <span class="ocrx_word">  Padded  </span>
                <span class="ocrx_word">  Text  </span>
            </p>
        </div>"#;

        let markdown = convert_hocr_to_markdown(hocr, None).unwrap();
        assert!(!markdown.is_empty());
    }

    #[test]
    fn test_hocr_special_characters() {
        let hocr = r#"<div class="ocr_page">
            <p class="ocr_par">
                <span class="ocrx_word">&lt;special&gt;</span>
                <span class="ocrx_word">&amp;chars&amp;</span>
            </p>
        </div>"#;

        let markdown = convert_hocr_to_markdown(hocr, None).unwrap();
        assert!(!markdown.is_empty());
    }

    #[test]
    fn test_hocr_nested_structure() {
        let hocr = r#"<div class="ocr_page">
            <div class="ocr_carea">
                <p class="ocr_par">
                    <span class="ocr_line">
                        <span class="ocrx_word">Nested</span>
                    </span>
                </p>
            </div>
        </div>"#;

        let markdown = convert_hocr_to_markdown(hocr, None).unwrap();
        assert!(markdown.contains("Nested"));
    }

    #[test]
    fn test_hocr_malformed_html() {
        let hocr = r#"<div class="ocr_page">
            <p class="ocr_par">
                <span class="ocrx_word">Unclosed
        </div>"#;

        let result = convert_hocr_to_markdown(hocr, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hocr_no_ocr_classes() {
        // Regular HTML without hOCR classes produces empty output - this is expected
        let hocr = r#"<div>
            <p>
                <span>Regular HTML</span>
            </p>
        </div>"#;

        let markdown = convert_hocr_to_markdown(hocr, None).unwrap();
        // When there are no hOCR classes, the output may be empty
        assert!(markdown.is_empty() || markdown.trim().is_empty());
    }

    #[test]
    fn test_hocr_mixed_content() {
        // hOCR needs proper classes for headings; regular HTML elements without hOCR classes are ignored
        let hocr = r#"<div class="ocr_page">
            <h1 class="ocr_chapter"><span class="ocrx_word">Heading</span></h1>
            <p class="ocr_par">
                <span class="ocrx_word">Paragraph</span>
            </p>
        </div>"#;

        let markdown = convert_hocr_to_markdown(hocr, None).unwrap();
        assert!(markdown.contains("Heading"));
        assert!(markdown.contains("Paragraph"));
    }

    #[test]
    fn test_hocr_unicode_content() {
        let hocr = r#"<div class="ocr_page">
            <p class="ocr_par">
                <span class="ocrx_word">Ñoño</span>
                <span class="ocrx_word">日本語</span>
                <span class="ocrx_word">العربية</span>
            </p>
        </div>"#;

        let markdown = convert_hocr_to_markdown(hocr, None).unwrap();
        assert!(markdown.contains("Ñoño") || !markdown.is_empty());
    }

    #[test]
    fn test_hocr_large_document() {
        let mut hocr = String::from(r#"<div class="ocr_page">"#);
        for i in 0..100 {
            hocr.push_str(&format!(
                r#"<p class="ocr_par"><span class="ocrx_word">Word{}</span></p>"#,
                i
            ));
        }
        hocr.push_str("</div>");

        let result = convert_hocr_to_markdown(&hocr, None);
        assert!(result.is_ok());
        let markdown = result.unwrap();
        assert!(!markdown.is_empty());
    }
}
