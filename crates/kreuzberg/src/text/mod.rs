pub mod quality;
pub mod string_utils;
pub mod token_reduction;

pub use quality::{calculate_quality_score, clean_extracted_text, normalize_spaces};
pub use string_utils::{calculate_text_confidence, fix_mojibake, get_encoding_cache_key, safe_decode};
pub use token_reduction::{
    ReductionLevel, TokenReductionConfig, batch_reduce_tokens, get_reduction_statistics, reduce_tokens,
};
