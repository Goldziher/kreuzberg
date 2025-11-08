use std::ops::RangeInclusive;

/// CJK text tokenizer for token reduction.
///
/// This tokenizer uses bigram (2-character) tokenization for CJK text,
/// which is appropriate for token reduction where we want to preserve
/// meaning while reducing token count.
///
/// # Unicode Range Coverage
///
/// **Currently covers:** CJK Unified Ideographs (U+4E00-U+9FFF)
/// - Covers ~20,992 common Chinese/Japanese Kanji characters
/// - Sufficient for token reduction purposes with Chinese and Japanese text
///
/// **Intentionally excluded:**
/// - Hiragana (U+3040-U+309F): Japanese phonetic script
/// - Katakana (U+30A0-U+30FF): Japanese phonetic script
/// - Hangul (U+AC00-U+D7AF): Korean alphabet
///
/// These exclusions are intentional for token reduction. Hiragana and Katakana
/// are typically tokenized with whitespace, and Hangul has different tokenization
/// requirements. If broader CJK support is needed, consider expanding the range
/// or using language-specific tokenizers.
pub struct CjkTokenizer {
    cjk_range: RangeInclusive<u32>,
}

impl CjkTokenizer {
    pub fn new() -> Self {
        Self {
            cjk_range: 0x4E00..=0x9FFF,
        }
    }

    /// Checks if a character is a CJK Unified Ideograph (U+4E00-U+9FFF).
    ///
    /// Returns true for Chinese characters and Japanese Kanji, false for
    /// Hiragana, Katakana, Hangul, and non-CJK characters.
    #[inline]
    pub fn is_cjk_char(&self, c: char) -> bool {
        self.cjk_range.contains(&(c as u32))
    }

    #[inline]
    pub fn has_cjk(&self, text: &str) -> bool {
        text.chars().any(|c| self.is_cjk_char(c))
    }

    pub fn tokenize_cjk_string(&self, text: &str) -> Vec<String> {
        let chars: Vec<char> = text.chars().collect();
        self.tokenize_cjk_chars(&chars)
    }

    pub fn tokenize_cjk_chars(&self, chars: &[char]) -> Vec<String> {
        chars
            .chunks(2)
            .map(|chunk| {
                if chunk.len() == 2 {
                    format!("{}{}", chunk[0], chunk[1])
                } else {
                    chunk[0].to_string()
                }
            })
            .collect()
    }

    pub fn tokenize_mixed_text(&self, text: &str) -> Vec<String> {
        let whitespace_tokens: Vec<&str> = text.split_whitespace().collect();

        if whitespace_tokens.is_empty() {
            return if text.is_empty() {
                vec![]
            } else {
                vec![text.to_string()]
            };
        }

        if whitespace_tokens.len() == 1 {
            let token = whitespace_tokens[0];
            return if self.has_cjk(token) {
                self.tokenize_cjk_string(token)
            } else {
                vec![token.to_string()]
            };
        }

        let mut all_tokens = Vec::new();
        for token in whitespace_tokens {
            if self.has_cjk(token) {
                all_tokens.extend(self.tokenize_cjk_string(token));
            } else {
                all_tokens.push(token.to_string());
            }
        }
        all_tokens
    }
}

impl Default for CjkTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cjk_char() {
        let tokenizer = CjkTokenizer::new();

        assert!(tokenizer.is_cjk_char('中'));
        assert!(tokenizer.is_cjk_char('国'));
        assert!(tokenizer.is_cjk_char('日'));
        assert!(tokenizer.is_cjk_char('本'));

        assert!(!tokenizer.is_cjk_char('a'));
        assert!(!tokenizer.is_cjk_char('Z'));
        assert!(!tokenizer.is_cjk_char('1'));
        assert!(!tokenizer.is_cjk_char(' '));
    }

    #[test]
    fn test_has_cjk() {
        let tokenizer = CjkTokenizer::new();

        assert!(tokenizer.has_cjk("这是中文"));
        assert!(tokenizer.has_cjk("mixed 中文 text"));
        assert!(tokenizer.has_cjk("日本語"));

        assert!(!tokenizer.has_cjk("English text"));
        assert!(!tokenizer.has_cjk("12345"));
        assert!(!tokenizer.has_cjk(""));
    }

    #[test]
    fn test_tokenize_cjk_string() {
        let tokenizer = CjkTokenizer::new();

        let tokens = tokenizer.tokenize_cjk_string("中国人");
        assert_eq!(tokens, vec!["中国", "人"]);

        let tokens = tokenizer.tokenize_cjk_string("四个字");
        assert_eq!(tokens, vec!["四个", "字"]);
    }

    #[test]
    fn test_tokenize_mixed_text() {
        let tokenizer = CjkTokenizer::new();

        let tokens = tokenizer.tokenize_mixed_text("hello world");
        assert_eq!(tokens, vec!["hello", "world"]);

        let tokens = tokenizer.tokenize_mixed_text("中国");
        assert_eq!(tokens, vec!["中国"]);

        let tokens = tokenizer.tokenize_mixed_text("hello 中国 world");
        assert_eq!(tokens, vec!["hello", "中国", "world"]);

        let tokens = tokenizer.tokenize_mixed_text("学习 machine learning 技术");
        assert_eq!(tokens, vec!["学习", "machine", "learning", "技术"]);
    }
}
