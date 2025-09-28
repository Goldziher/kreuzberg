//! PPTX parser configuration

/// Configuration for PPTX parsing
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// Whether to extract images
    pub extract_images: bool,
    /// Whether to include slide comments in output
    pub include_slide_comment: bool,
    /// Maximum cache size for resources (in MB)
    pub max_cache_size_mb: usize,
    /// Maximum number of cached images
    pub max_cached_images: usize,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            extract_images: true,
            include_slide_comment: false,
            max_cache_size_mb: 256,
            max_cached_images: 100,
        }
    }
}
