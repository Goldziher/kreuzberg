// Quality processing modules (feature-gated)
#[cfg(feature = "quality")]
pub mod quality;

#[cfg(feature = "quality")]
pub mod string_utils;

#[cfg(feature = "quality")]
pub mod token_reduction;

// Re-exports (feature-gated)
#[cfg(feature = "quality")]
pub use quality::{calculate_quality_score, clean_extracted_text, normalize_spaces};

#[cfg(feature = "quality")]
pub use string_utils::{calculate_text_confidence, fix_mojibake, get_encoding_cache_key, safe_decode};

#[cfg(feature = "quality")]
pub use token_reduction::{
    ReductionLevel, TokenReductionConfig, batch_reduce_tokens, get_reduction_statistics, reduce_tokens,
};
