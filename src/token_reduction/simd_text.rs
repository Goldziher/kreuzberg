use memchr::memchr3;

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
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn normalize_whitespace(&self, text: &str) -> String {
        let bytes = text.as_bytes();
        let mut result = Vec::with_capacity(text.len());
        let mut i = 0;
        let mut in_whitespace = false;

        while i + self.chunk_size <= bytes.len() {
            let chunk = &bytes[i..i + self.chunk_size];

            if let Some(ws_pos) = self.find_whitespace_simd(chunk) {
                let prefix_end = i + ws_pos;
                if !in_whitespace && i < prefix_end {
                    result.extend_from_slice(&bytes[i..prefix_end]);
                }

                i = self.skip_whitespace_sequence(bytes, prefix_end);
                if !in_whitespace {
                    result.push(b' ');
                }
                in_whitespace = true;
            } else {
                if !in_whitespace {
                    result.extend_from_slice(chunk);
                }
                i += self.chunk_size;
                in_whitespace = false;
            }
        }

        while i < bytes.len() {
            let byte = bytes[i];
            if self.is_whitespace(byte) {
                if !in_whitespace {
                    result.push(b' ');
                }
                in_whitespace = true;
            } else {
                result.push(byte);
                in_whitespace = false;
            }
            i += 1;
        }

        String::from_utf8(result).unwrap_or_else(|_| text.to_string())
    }

    pub fn clean_punctuation(&self, text: &str) -> String {
        let bytes = text.as_bytes();
        let mut result = Vec::with_capacity(text.len());
        let mut i = 0;

        while i < bytes.len() {
            let byte = bytes[i];

            if self.is_repeated_punctuation(byte) {
                let sequence_end = self.find_complex_punctuation_sequence_end(bytes, i);

                if sequence_end - i > 1 {
                    let sequence_slice = &bytes[i..sequence_end];
                    let has_mixed_punctuation = sequence_slice.windows(2).any(|pair| pair[0] != pair[1]);

                    if has_mixed_punctuation {
                        match byte {
                            b'!' | b'?' => result.push(byte),
                            b'.' => result.push(b'.'),
                            b',' => result.push(b','),
                            _ => result.push(byte),
                        }
                    } else {
                        match byte {
                            b'!' | b'?' => result.push(byte),
                            b'.' => result.push(b'.'),
                            b',' => result.push(b','),
                            _ => result.push(byte),
                        }
                    }
                } else {
                    result.push(byte);
                }

                i = sequence_end;
            } else {
                result.push(byte);
                i += 1;
            }
        }

        String::from_utf8(result).unwrap_or_else(|_| text.to_string())
    }

    #[allow(dead_code)]
    #[inline]
    fn find_whitespace_simd(&self, chunk: &[u8]) -> Option<usize> {
        memchr3(b' ', b'\t', b'\n', chunk)
    }

    #[allow(dead_code)]
    #[inline]
    fn is_whitespace(&self, byte: u8) -> bool {
        matches!(byte, b' ' | b'\t' | b'\n' | b'\r')
    }

    #[inline]
    fn is_repeated_punctuation(&self, byte: u8) -> bool {
        matches!(byte, b'!' | b'?' | b'.' | b',')
    }

    #[allow(dead_code)]
    fn skip_whitespace_sequence(&self, bytes: &[u8], start: usize) -> usize {
        let mut i = start;
        while i < bytes.len() && self.is_whitespace(bytes[i]) {
            i += 1;
        }
        i
    }

    fn find_complex_punctuation_sequence_end(&self, bytes: &[u8], start: usize) -> usize {
        let mut i = start + 1;

        while i < bytes.len() && self.is_repeated_punctuation(bytes[i]) {
            i += 1;
        }

        i
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
            let search_start = (end.saturating_sub(200)).max(start);
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
    fn test_normalize_whitespace() {
        let processor = SimdTextProcessor::new();
        let input = "Hello   \t\n  world!";
        let result = processor.normalize_whitespace(input);
        assert_eq!(result, "Hello world!");
    }

    #[test]
    fn test_clean_punctuation() {
        let processor = SimdTextProcessor::new();
        let input = "What???!!! Really... Yes,,,";
        let result = processor.clean_punctuation(input);
        assert_eq!(result, "What? Really. Yes,");
    }

    #[test]
    fn test_chunk_text() {
        let text = "This is a test. Another sentence! And one more? Final statement.";
        let chunks = chunk_text_for_parallel(text, 2);
        assert!(chunks.len() <= 2);
        assert_eq!(chunks.join(""), text);
    }
}
