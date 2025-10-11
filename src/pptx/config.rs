#[derive(Debug, Clone)]
pub struct ParserConfig {
    pub extract_images: bool,
    pub include_slide_comment: bool,
    pub max_cache_size_mb: usize,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_config_default() {
        let config = ParserConfig::default();
        assert!(config.extract_images);
        assert!(!config.include_slide_comment);
        assert_eq!(config.max_cache_size_mb, 256);
        assert_eq!(config.max_cached_images, 100);
    }

    #[test]
    fn test_parser_config_custom() {
        let config = ParserConfig {
            extract_images: false,
            include_slide_comment: true,
            max_cache_size_mb: 512,
            max_cached_images: 200,
        };
        assert!(!config.extract_images);
        assert!(config.include_slide_comment);
        assert_eq!(config.max_cache_size_mb, 512);
        assert_eq!(config.max_cached_images, 200);
    }

    #[test]
    fn test_parser_config_clone() {
        let config1 = ParserConfig::default();
        let config2 = config1.clone();
        assert_eq!(config1.extract_images, config2.extract_images);
        assert_eq!(config1.include_slide_comment, config2.include_slide_comment);
        assert_eq!(config1.max_cache_size_mb, config2.max_cache_size_mb);
        assert_eq!(config1.max_cached_images, config2.max_cached_images);
    }
}
