use std::hash::Hash;

use ahash::AHasher;

pub const MINIMAL_SUPPORTED_TESSERACT_VERSION: u32 = 5;

#[cfg(test)]
pub const HASH_LENGTH_CHARS: usize = 16;

pub const TSV_MIN_FIELDS: usize = 12;
pub const TSV_WORD_LEVEL: u32 = 5;

pub const MIN_COLUMN_WIDTH: usize = 3;

pub fn compute_hash<T: Hash>(data: &T) -> String {
    use std::hash::Hasher;

    let mut hasher = AHasher::default();
    data.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash_deterministic() {
        let data1 = vec![1, 2, 3, 4, 5];
        let data2 = vec![1, 2, 3, 4, 5];

        assert_eq!(compute_hash(&data1), compute_hash(&data2));
    }

    #[test]
    fn test_compute_hash_different_inputs() {
        let data1 = vec![1, 2, 3];
        let data2 = vec![3, 2, 1];

        assert_ne!(compute_hash(&data1), compute_hash(&data2));
    }

    #[test]
    fn test_compute_hash_length() {
        let data = "test string";
        let hash = compute_hash(&data);

        assert_eq!(hash.len(), HASH_LENGTH_CHARS);
    }

    #[test]
    fn test_compute_hash_string() {
        let s1 = "hello world";
        let s2 = "hello world";

        assert_eq!(compute_hash(&s1), compute_hash(&s2));
    }
}
