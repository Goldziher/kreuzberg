mod cjk_utils;
mod config;
mod core;
mod filters;
mod semantic;
mod simd_text;

pub use config::{ReductionLevel, TokenReductionConfig};
pub use core::TokenReducer;

// TODO: reorganize token_reduction - move out of text, and reorganize text properly into utils etc.

pub fn reduce_tokens(
    text: &str,
    config: &TokenReductionConfig,
    language_hint: Option<&str>,
) -> crate::error::Result<String> {
    let reducer = TokenReducer::new(config, language_hint)?;
    Ok(reducer.reduce(text))
}

pub fn batch_reduce_tokens(
    texts: &[&str],
    config: &TokenReductionConfig,
    language_hint: Option<&str>,
) -> crate::error::Result<Vec<String>> {
    let reducer = TokenReducer::new(config, language_hint)?;
    Ok(reducer.batch_reduce(texts))
}

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
