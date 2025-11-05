use super::super::error::OcrError;
use super::super::utils::{TSV_MIN_FIELDS, TSV_WORD_LEVEL};
use html_to_markdown_rs::hocr::HocrWord;

/// Extract words from Tesseract TSV output and convert to HocrWord format
///
/// This parses Tesseract's TSV format (level, page_num, block_num, ...) and
/// converts it to the HocrWord format used by html-to-markdown-rs for table reconstruction.
pub fn extract_words_from_tsv(tsv_data: &str, min_confidence: f64) -> Result<Vec<HocrWord>, OcrError> {
    let mut words = Vec::new();

    for (line_num, line) in tsv_data.lines().enumerate() {
        if line_num == 0 {
            continue;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < TSV_MIN_FIELDS {
            continue;
        }

        let level = fields[0].parse::<u32>().unwrap_or(0);
        if level != TSV_WORD_LEVEL {
            continue;
        }

        let conf = fields[10].parse::<f64>().unwrap_or(-1.0);
        if conf < min_confidence {
            continue;
        }

        let text = fields[11].trim();
        if text.is_empty() {
            continue;
        }

        let word = HocrWord {
            text: text.to_string(),
            left: fields[6].parse().unwrap_or(0),
            top: fields[7].parse().unwrap_or(0),
            width: fields[8].parse().unwrap_or(0),
            height: fields[9].parse().unwrap_or(0),
            confidence: conf,
        };

        words.push(word);
    }

    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_words_basic() {
        let tsv = r#"level	page_num	block_num	par_num	line_num	word_num	left	top	width	height	conf	text
5	1	0	0	0	0	100	50	80	30	95.5	Hello
5	1	0	0	0	1	190	50	70	30	92.3	World"#;

        let words = extract_words_from_tsv(tsv, 0.0).unwrap();
        assert_eq!(words.len(), 2);

        assert_eq!(words[0].text, "Hello");
        assert_eq!(words[0].left, 100);
        assert_eq!(words[0].top, 50);
        assert_eq!(words[0].confidence, 95.5);

        assert_eq!(words[1].text, "World");
        assert_eq!(words[1].left, 190);
    }

    #[test]
    fn test_extract_words_confidence_filter() {
        let tsv = r#"level	page_num	block_num	par_num	line_num	word_num	left	top	width	height	conf	text
5	1	0	0	0	0	100	50	80	30	95.5	Hello
5	1	0	0	0	1	190	50	70	30	50.0	World
5	1	0	0	0	2	270	50	60	30	92.3	Test"#;

        let words = extract_words_from_tsv(tsv, 90.0).unwrap();
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "Hello");
        assert_eq!(words[1].text, "Test");
    }

    #[test]
    fn test_extract_words_level_filter() {
        let tsv = r#"level	page_num	block_num	par_num	line_num	word_num	left	top	width	height	conf	text
3	1	0	0	0	0	100	50	80	30	95.5	Paragraph
5	1	0	0	0	0	100	50	80	30	95.5	Hello
4	1	0	0	0	1	190	50	70	30	92.3	Line"#;

        let words = extract_words_from_tsv(tsv, 0.0).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "Hello");
    }

    #[test]
    fn test_hocr_word_methods() {
        let word = HocrWord {
            text: "Hello".to_string(),
            left: 100,
            top: 50,
            width: 80,
            height: 30,
            confidence: 95.5,
        };

        assert_eq!(word.right(), 180);
        assert_eq!(word.bottom(), 80);
        assert_eq!(word.y_center(), 65.0);
        assert_eq!(word.x_center(), 140.0);
    }

    #[test]
    fn test_extract_words_empty_text() {
        let tsv = r#"level	page_num	block_num	par_num	line_num	word_num	left	top	width	height	conf	text
5	1	0	0	0	0	100	50	80	30	95.5
5	1	0	0	0	1	190	50	70	30	92.3	World"#;

        let words = extract_words_from_tsv(tsv, 0.0).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "World");
    }

    #[test]
    fn test_extract_words_malformed() {
        let tsv = r#"level	page_num	block_num
5	1	0	0	0	0	100	50	80	30	95.5	Hello
invalid line
5	1	0	0	0	1	190	50	70	30	92.3	World"#;

        let words = extract_words_from_tsv(tsv, 0.0).unwrap();
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "Hello");
        assert_eq!(words[1].text, "World");
    }
}
