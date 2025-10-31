use memchr::{memchr, memchr3};

pub struct SimdTextProcessor {
    #[allow(dead_code)]
    chunk_size: usize,
}

impl Default for SimdTextProcessor {
    fn default() -> Self {
        Self { chunk_size: 64 }
    }
}

impl SimdTextProcessor {
    // TODO: we should use SIMD acceleration not only in token reduction, but also in other text processing, lets see if we can generalize this - inspect for refactor
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clean_punctuation(&self, text: &str) -> String {
        let bytes = text.as_bytes();
        let mut result = Vec::with_capacity(text.len());
        let mut i = 0;
        let len = bytes.len();

        while i < len {
            match self.find_next_punctuation(bytes, i) {
                Some(offset) => {
                    if offset > 0 {
                        result.extend_from_slice(&bytes[i..i + offset]);
                        i += offset;
                    }
                }
                None => {
                    result.extend_from_slice(&bytes[i..]);
                    break;
                }
            }

            if i >= len {
                break;
            }

            let sequence_end = self.find_complex_punctuation_sequence_end(bytes, i);
            let byte = bytes[i];

            result.push(match byte {
                b'!' | b'?' => byte,
                b'.' => b'.',
                b',' => b',',
                _ => byte,
            });

            i = sequence_end;
        }

        // Fallback to original text if UTF-8 conversion fails - this is safe because input is valid UTF-8 ~keep
        // Punctuation removal should preserve UTF-8 validity, but fallback ensures robustness
        String::from_utf8(result).unwrap_or_else(|_| text.to_string())
    }

    #[inline]
    fn is_repeated_punctuation(&self, byte: u8) -> bool {
        matches!(byte, b'!' | b'?' | b'.' | b',')
    }

    fn find_complex_punctuation_sequence_end(&self, bytes: &[u8], start: usize) -> usize {
        let mut i = start + 1;

        while i < bytes.len() && self.is_repeated_punctuation(bytes[i]) {
            i += 1;
        }

        i
    }

    #[inline]
    fn find_next_punctuation(&self, bytes: &[u8], start: usize) -> Option<usize> {
        let search = &bytes[start..];
        let primary = memchr3(b'!', b'?', b'.', search);
        let comma = memchr(b',', search);

        match (primary, comma) {
            (Some(p), Some(c)) => Some(p.min(c)),
            (Some(p), None) => Some(p),
            (None, Some(c)) => Some(c),
            (None, None) => None,
        }
    }
}

pub fn chunk_text_for_parallel(text: &str, target_chunks: usize) -> Vec<&str> {
    if text.len() < 1000 || target_chunks <= 1 {
        return vec![text];
    }

    let approximate_chunk_size = text.len() / target_chunks;
    let mut chunks = Vec::with_capacity(target_chunks);
    let mut start = 0;

    while start < text.len() {
        let mut end = (start + approximate_chunk_size).min(text.len());

        if end < text.len() {
            let search_start = end.saturating_sub(200).max(start);
            let search_end = (end + 200).min(text.len());

            if let Some(boundary_pos) = memchr3(b'.', b'!', b'?', &text.as_bytes()[search_start..search_end]) {
                let actual_pos = search_start + boundary_pos + 1;
                if actual_pos > start && actual_pos < text.len() {
                    end = actual_pos;
                }
            }
        }

        chunks.push(&text[start..end]);
        start = end;
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_punctuation() {
        let processor = SimdTextProcessor::new();
        let input = "What???!!! Really... Yes,,,";
        let result = processor.clean_punctuation(input);
        assert_eq!(result, "What? Really. Yes,");
    }

    #[test]
    fn test_clean_punctuation_large_ascii_block() {
        let processor = SimdTextProcessor::new();
        let mut input = "a".repeat(1024);
        input.push_str("!!!");
        let expected = format!("{}!", "a".repeat(1024));

        let result = processor.clean_punctuation(&input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_chunk_text() {
        let text = "This is a test. Another sentence! And one more? Final statement.";
        let chunks = chunk_text_for_parallel(text, 2);
        assert!(chunks.len() <= 2);
        assert_eq!(chunks.join(""), text);
    }
}
