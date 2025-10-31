mod cjk_utils;
mod config;
mod core;
mod filters;
mod semantic;
mod simd_text;

pub use config::{ReductionLevel, TokenReductionConfig};
pub use core::TokenReducer;

// TODO: reorganize token_reduction - move out of text, and reorganize text properly into utils etc.

/// Reduces token count in text while preserving meaning and structure.
///
/// This function removes stopwords, redundancy, and applies compression techniques
/// based on the specified reduction level. Supports 64 languages with automatic
/// stopword removal and optional semantic clustering.
///
/// # Arguments
///
/// * `text` - The input text to reduce
/// * `config` - Configuration specifying reduction level and options
/// * `language_hint` - Optional ISO 639-3 language code (e.g., "eng", "spa")
///
/// # Returns
///
/// Returns the reduced text with preserved structure (markdown, code blocks).
///
/// # Errors
///
/// Returns an error if the language hint is invalid or stopwords cannot be loaded.
///
/// # Examples
///
/// ```rust
/// use kreuzberg::text::token_reduction::{reduce_tokens, TokenReductionConfig, ReductionLevel};
///
/// let text = "This is a simple example text with some stopwords.";
/// let config = TokenReductionConfig::new(ReductionLevel::Moderate);
/// let reduced = reduce_tokens(text, &config, Some("eng"))?;
/// println!("Reduced: {}", reduced);
/// # Ok::<(), kreuzberg::error::KreuzbergError>(())
/// ```
pub fn reduce_tokens(
    text: &str,
    config: &TokenReductionConfig,
    language_hint: Option<&str>,
) -> crate::error::Result<String> {
    let reducer = TokenReducer::new(config, language_hint)?;
    Ok(reducer.reduce(text))
}

/// Reduces token count for multiple texts efficiently using parallel processing.
///
/// This function processes multiple texts in parallel using Rayon, providing
/// significant performance improvements for batch operations. All texts use the
/// same configuration and language hint for consistency.
///
/// # Arguments
///
/// * `texts` - Slice of text references to reduce
/// * `config` - Configuration specifying reduction level and options
/// * `language_hint` - Optional ISO 639-3 language code (e.g., "eng", "spa")
///
/// # Returns
///
/// Returns a vector of reduced texts in the same order as the input.
///
/// # Errors
///
/// Returns an error if the language hint is invalid or stopwords cannot be loaded.
///
/// # Examples
///
/// ```rust
/// use kreuzberg::text::token_reduction::{batch_reduce_tokens, TokenReductionConfig, ReductionLevel};
///
/// let texts = vec![
///     "This is the first document with some text.",
///     "Here is another document with different content.",
///     "And finally, a third document to process.",
/// ];
/// let config = TokenReductionConfig::new(ReductionLevel::Moderate);
/// let reduced = batch_reduce_tokens(&texts, &config, Some("eng"))?;
/// assert_eq!(reduced.len(), 3);
/// # Ok::<(), kreuzberg::error::KreuzbergError>(())
/// ```
pub fn batch_reduce_tokens(
    texts: &[&str],
    config: &TokenReductionConfig,
    language_hint: Option<&str>,
) -> crate::error::Result<Vec<String>> {
    let reducer = TokenReducer::new(config, language_hint)?;
    Ok(reducer.batch_reduce(texts))
}

/// Calculates detailed statistics comparing original and reduced text.
///
/// Provides comprehensive metrics including reduction percentages and absolute
/// counts for both characters and tokens. Useful for analyzing the effectiveness
/// of token reduction and monitoring compression ratios.
///
/// # Arguments
///
/// * `original` - The original text before reduction
/// * `reduced` - The reduced text after applying token reduction
///
/// # Returns
///
/// Returns a tuple with the following statistics (in order):
/// 1. `char_reduction` (f64) - Character reduction ratio (0.0 to 1.0)
/// 2. `token_reduction` (f64) - Token reduction ratio (0.0 to 1.0)
/// 3. `original_chars` (usize) - Original character count
/// 4. `reduced_chars` (usize) - Reduced character count
/// 5. `original_tokens` (usize) - Original token count (whitespace-delimited)
/// 6. `reduced_tokens` (usize) - Reduced token count (whitespace-delimited)
///
/// # Examples
///
/// ```rust
/// use kreuzberg::text::token_reduction::{reduce_tokens, get_reduction_statistics, TokenReductionConfig, ReductionLevel};
///
/// let original = "This is a simple example text with some stopwords and redundancy.";
/// let config = TokenReductionConfig::new(ReductionLevel::Moderate);
/// let reduced = reduce_tokens(original, &config, Some("eng"))?;
///
/// let (char_ratio, token_ratio, orig_chars, red_chars, orig_tokens, red_tokens) =
///     get_reduction_statistics(original, &reduced);
///
/// println!("Reduced {:.1}% of characters ({} -> {})", char_ratio * 100.0, orig_chars, red_chars);
/// println!("Reduced {:.1}% of tokens ({} -> {})", token_ratio * 100.0, orig_tokens, red_tokens);
/// # Ok::<(), kreuzberg::error::KreuzbergError>(())
/// ```
pub fn get_reduction_statistics(original: &str, reduced: &str) -> (f64, f64, usize, usize, usize, usize) {
    let original_chars = original.chars().count();
    let reduced_chars = reduced.chars().count();
    let original_tokens = original.split_whitespace().count();
    let reduced_tokens = reduced.split_whitespace().count();

    let char_reduction = if original_chars > 0 {
        1.0 - (reduced_chars as f64 / original_chars as f64)
    } else {
        0.0
    };

    let token_reduction = if original_tokens > 0 {
        1.0 - (reduced_tokens as f64 / original_tokens as f64)
    } else {
        0.0
    };

    (
        char_reduction,
        token_reduction,
        original_chars,
        reduced_chars,
        original_tokens,
        reduced_tokens,
    )
}
