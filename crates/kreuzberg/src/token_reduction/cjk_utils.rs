use std::ops::RangeInclusive;

pub struct CjkTokenizer {
    cjk_range: RangeInclusive<u32>,
}

impl CjkTokenizer {
    pub fn new() -> Self {
        Self {
            cjk_range: 0x4E00..=0x9FFF,
        }
    }

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
