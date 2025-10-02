//! TSV parser for Tesseract TSV output
//!
//! Parses Tesseract's TSV output format to extract word-level information
//! including position, dimensions, and confidence scores.

use super::super::error::OCRError;

/// Represents a word extracted from Tesseract TSV output
#[derive(Debug, Clone, PartialEq)]
pub struct TSVWord {
    pub level: u32,
    pub page_num: u32,
    pub block_num: u32,
    pub par_num: u32,
    pub line_num: u32,
    pub word_num: u32,
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
    pub conf: f64,
    pub text: String,
}

impl TSVWord {
    /// Get the right edge position
    #[allow(dead_code)]
    pub fn right(&self) -> u32 {
        self.left + self.width
    }

    /// Get the bottom edge position
    #[allow(dead_code)]
    pub fn bottom(&self) -> u32 {
        self.top + self.height
    }

    /// Get the vertical center position
    pub fn y_center(&self) -> f64 {
        self.top as f64 + (self.height as f64 / 2.0)
    }

    /// Get the horizontal center position
    #[allow(dead_code)]
    pub fn x_center(&self) -> f64 {
        self.left as f64 + (self.width as f64 / 2.0)
    }
}

/// Extract words from Tesseract TSV output
///
/// Parses TSV data and filters for word-level entries (level 5) with sufficient confidence.
///
/// # Arguments
///
/// * `tsv_data` - Raw TSV output from Tesseract
/// * `min_confidence` - Minimum confidence threshold (0.0-100.0)
///
/// # Returns
///
/// Vector of TSVWord structs representing detected words
pub fn extract_words(tsv_data: &str, min_confidence: f64) -> Result<Vec<TSVWord>, OCRError> {
    let mut words = Vec::new();

    for (line_num, line) in tsv_data.lines().enumerate() {
        // Skip header line
        if line_num == 0 {
            continue;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse TSV line
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 12 {
            continue; // Skip malformed lines
        }

        // Parse level (filter for word level = 5)
        let level = fields[0].parse::<u32>().unwrap_or(0);
        if level != 5 {
            continue;
        }

        // Parse confidence
        let conf = fields[10].parse::<f64>().unwrap_or(-1.0);
        if conf < min_confidence {
            continue;
        }

        // Parse text (skip empty text)
        let text = fields[11].trim();
        if text.is_empty() {
            continue;
        }

        // Parse all fields
        let word = TSVWord {
            level,
            page_num: fields[1].parse().unwrap_or(0),
            block_num: fields[2].parse().unwrap_or(0),
            par_num: fields[3].parse().unwrap_or(0),
            line_num: fields[4].parse().unwrap_or(0),
            word_num: fields[5].parse().unwrap_or(0),
            left: fields[6].parse().unwrap_or(0),
            top: fields[7].parse().unwrap_or(0),
            width: fields[8].parse().unwrap_or(0),
            height: fields[9].parse().unwrap_or(0),
            conf,
            text: text.to_string(),
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

        let words = extract_words(tsv, 0.0).unwrap();
        assert_eq!(words.len(), 2);

        assert_eq!(words[0].text, "Hello");
        assert_eq!(words[0].left, 100);
        assert_eq!(words[0].top, 50);
        assert_eq!(words[0].conf, 95.5);

        assert_eq!(words[1].text, "World");
        assert_eq!(words[1].left, 190);
    }

    #[test]
    fn test_extract_words_confidence_filter() {
        let tsv = r#"level	page_num	block_num	par_num	line_num	word_num	left	top	width	height	conf	text
5	1	0	0	0	0	100	50	80	30	95.5	Hello
5	1	0	0	0	1	190	50	70	30	50.0	World
5	1	0	0	0	2	270	50	60	30	92.3	Test"#;

        let words = extract_words(tsv, 90.0).unwrap();
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

        let words = extract_words(tsv, 0.0).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "Hello");
    }

    #[test]
    fn test_tsv_word_methods() {
        let word = TSVWord {
            level: 5,
            page_num: 1,
            block_num: 0,
            par_num: 0,
            line_num: 0,
            word_num: 0,
            left: 100,
            top: 50,
            width: 80,
            height: 30,
            conf: 95.5,
            text: "Hello".to_string(),
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

        let words = extract_words(tsv, 0.0).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "World");
    }

    #[test]
    fn test_extract_words_malformed() {
        let tsv = r#"level	page_num	block_num
5	1	0	0	0	0	100	50	80	30	95.5	Hello
invalid line
5	1	0	0	0	1	190	50	70	30	92.3	World"#;

        let words = extract_words(tsv, 0.0).unwrap();
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "Hello");
        assert_eq!(words[1].text, "World");
    }
}
