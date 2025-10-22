//! Keyword extraction module.
//!
//! Provides unified keyword extraction interface supporting multiple algorithms:
//! - YAKE (Yet Another Keyword Extractor) - statistical approach
//! - RAKE (Rapid Automatic Keyword Extraction) - co-occurrence based
//!
//! # Feature Flags
//!
//! - `keywords-yake`: Enable YAKE algorithm
//! - `keywords-rake`: Enable RAKE algorithm
//! - `keywords`: Enable both algorithms (default in `full` feature)
//!
//! # Examples
//!
//! ```rust,no_run
//! # use kreuzberg::keywords::{extract_keywords, KeywordConfig};
//! let text = "Rust is a systems programming language focused on safety and performance.";
//!
//! // Use default algorithm (YAKE if available)
//! let config = KeywordConfig::default();
//! let keywords = extract_keywords(text, &config).unwrap();
//!
//! for keyword in keywords {
//!     println!("{}: {:.3}", keyword.text, keyword.score);
//! }
//! ```
//!
//! ```rust,no_run
//! # #[cfg(feature = "keywords-rake")]
//! # {
//! # use kreuzberg::keywords::{extract_keywords, KeywordConfig};
//! // Use RAKE algorithm explicitly
//! let text = "Machine learning models require large datasets.";
//! let config = KeywordConfig::rake()
//!     .with_max_keywords(5)
//!     .with_min_score(0.3);
//!
//! let keywords = extract_keywords(text, &config).unwrap();
//! # }
//! ```

use crate::Result;
use crate::plugins::registry::get_post_processor_registry;
use once_cell::sync::Lazy;
use std::sync::Arc;

pub mod config;
pub mod processor;
pub mod types;

#[cfg(feature = "keywords-yake")]
mod yake;

#[cfg(feature = "keywords-rake")]
mod rake;

pub use config::KeywordConfig;
pub use processor::KeywordExtractor;

#[cfg(feature = "keywords-rake")]
pub use config::RakeParams;

#[cfg(feature = "keywords-yake")]
pub use config::YakeParams;
pub use types::{Keyword, KeywordAlgorithm};

/// Extract keywords from text using the specified algorithm.
///
/// This is the unified entry point for keyword extraction. The algorithm
/// used is determined by `config.algorithm`.
///
/// # Arguments
///
/// * `text` - The text to extract keywords from
/// * `config` - Keyword extraction configuration
///
/// # Returns
///
/// A vector of keywords sorted by relevance (highest score first).
///
/// # Errors
///
/// Returns an error if:
/// - The specified algorithm feature is not enabled
/// - Keyword extraction fails
///
/// # Examples
///
/// ```rust,no_run
/// # use kreuzberg::keywords::{extract_keywords, KeywordConfig};
/// let text = "Document intelligence with Rust provides memory safety.";
/// let config = KeywordConfig::default()
///     .with_max_keywords(10)
///     .with_language("en");
///
/// let keywords = extract_keywords(text, &config)?;
///
/// for keyword in keywords {
///     println!("{}: {:.3}", keyword.text, keyword.score);
/// }
/// # Ok::<(), kreuzberg::KreuzbergError>(())
/// ```
pub fn extract_keywords(text: &str, config: &KeywordConfig) -> Result<Vec<Keyword>> {
    match config.algorithm {
        #[cfg(feature = "keywords-yake")]
        KeywordAlgorithm::Yake => yake::extract_keywords_yake(text, config),

        #[cfg(feature = "keywords-rake")]
        KeywordAlgorithm::Rake => rake::extract_keywords_rake(text, config),

        #[cfg(not(any(feature = "keywords-yake", feature = "keywords-rake")))]
        _ => Err(crate::KreuzbergError::Other(
            "No keyword extraction algorithm feature enabled".to_string(),
        )),
    }
}

/// Lazy-initialized flag that ensures keyword processor is registered exactly once.
///
/// This static is accessed on first use to automatically register the
/// keyword extraction processor with the plugin registry.
static PROCESSOR_INITIALIZED: Lazy<Result<()>> = Lazy::new(register_keyword_processor);

/// Ensure the keyword processor is registered.
///
/// This function is called automatically when needed.
/// It's safe to call multiple times - registration only happens once.
pub fn ensure_initialized() -> Result<()> {
    PROCESSOR_INITIALIZED
        .as_ref()
        .map(|_| ())
        .map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("Failed to register keyword processor: {}", e),
            plugin_name: "keyword-extraction".to_string(),
        })
}

/// Register the keyword extraction processor with the global registry.
///
/// This function should be called once at application startup to register
/// the keyword extraction post-processor.
///
/// **Note:** This is called automatically on first use.
/// Explicit calling is optional.
///
/// # Example
///
/// ```rust
/// # #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
/// use kreuzberg::keywords::register_keyword_processor;
///
/// # #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
/// # fn main() -> kreuzberg::Result<()> {
/// register_keyword_processor()?;
/// # Ok(())
/// # }
/// # #[cfg(not(any(feature = "keywords-yake", feature = "keywords-rake")))]
/// # fn main() {}
/// ```
pub fn register_keyword_processor() -> Result<()> {
    let registry = get_post_processor_registry();
    let mut registry = registry
        .write()
        .map_err(|e| crate::KreuzbergError::Other(format!("Post-processor registry lock poisoned: {}", e)))?;

    registry.register(Arc::new(KeywordExtractor), 50)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_keywords_default_algorithm() {
        let text = "Rust programming language provides memory safety and performance.";
        let config = KeywordConfig::default();

        let keywords = extract_keywords(text, &config).unwrap();

        assert!(!keywords.is_empty(), "Should extract keywords");
        assert!(keywords.len() <= config.max_keywords);
    }

    #[cfg(feature = "keywords-yake")]
    #[test]
    fn test_extract_keywords_yake() {
        let text = "Natural language processing using Rust is efficient and safe.";
        let config = KeywordConfig::yake();

        let keywords = extract_keywords(text, &config).unwrap();

        assert!(!keywords.is_empty());
        assert_eq!(keywords[0].algorithm, KeywordAlgorithm::Yake);
    }

    #[cfg(feature = "keywords-rake")]
    #[test]
    fn test_extract_keywords_rake() {
        let text = "Natural language processing using Rust is efficient and safe.";
        let config = KeywordConfig::rake();

        let keywords = extract_keywords(text, &config).unwrap();

        assert!(!keywords.is_empty());
        assert_eq!(keywords[0].algorithm, KeywordAlgorithm::Rake);
    }

    #[cfg(all(feature = "keywords-yake", feature = "keywords-rake"))]
    #[test]
    fn test_compare_algorithms() {
        let text = "Machine learning and artificial intelligence are transforming technology. \
                    Deep learning models require substantial computational resources.";

        let yake_config = KeywordConfig::yake().with_max_keywords(5);
        let yake_keywords = extract_keywords(text, &yake_config).unwrap();

        let rake_config = KeywordConfig::rake().with_max_keywords(5);
        let rake_keywords = extract_keywords(text, &rake_config).unwrap();

        assert!(!yake_keywords.is_empty());
        assert!(!rake_keywords.is_empty());

        assert!(yake_keywords.iter().all(|k| k.algorithm == KeywordAlgorithm::Yake));
        assert!(rake_keywords.iter().all(|k| k.algorithm == KeywordAlgorithm::Rake));

        println!(
            "YAKE keywords: {:?}",
            yake_keywords.iter().map(|k| &k.text).collect::<Vec<_>>()
        );
        println!(
            "RAKE keywords: {:?}",
            rake_keywords.iter().map(|k| &k.text).collect::<Vec<_>>()
        );
    }
}
