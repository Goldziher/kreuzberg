use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)]
pub struct ExtractionConfigDTO {
    #[pyo3(get, set)]
    pub target_dpi: i32,
    #[pyo3(get, set)]
    pub max_image_dimension: i32,
    #[pyo3(get, set)]
    pub auto_adjust_dpi: bool,
    #[pyo3(get, set)]
    pub min_dpi: i32,
    #[pyo3(get, set)]
    pub max_dpi: i32,
}

#[pymethods]
impl ExtractionConfigDTO {
    #[new]
    #[pyo3(signature = (target_dpi=300, max_image_dimension=4096, auto_adjust_dpi=false, min_dpi=72, max_dpi=600))]
    #[must_use]
    pub const fn new(
        target_dpi: i32,
        max_image_dimension: i32,
        auto_adjust_dpi: bool,
        min_dpi: i32,
        max_dpi: i32,
    ) -> Self {
        Self {
            target_dpi,
            max_image_dimension,
            auto_adjust_dpi,
            min_dpi,
            max_dpi,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_new_with_defaults() {
        let config = ExtractionConfigDTO::new(300, 4096, false, 72, 600);
        assert_eq!(config.target_dpi, 300);
        assert_eq!(config.max_image_dimension, 4096);
        assert!(!config.auto_adjust_dpi);
        assert_eq!(config.min_dpi, 72);
        assert_eq!(config.max_dpi, 600);
    }

    #[test]
    fn test_config_new_with_custom_values() {
        let config = ExtractionConfigDTO::new(150, 2048, true, 96, 400);
        assert_eq!(config.target_dpi, 150);
        assert_eq!(config.max_image_dimension, 2048);
        assert!(config.auto_adjust_dpi);
        assert_eq!(config.min_dpi, 96);
        assert_eq!(config.max_dpi, 400);
    }

    #[test]
    fn test_config_clone() {
        let config1 = ExtractionConfigDTO::new(300, 4096, false, 72, 600);
        let config2 = config1.clone();
        assert_eq!(config1.target_dpi, config2.target_dpi);
        assert_eq!(config1.max_image_dimension, config2.max_image_dimension);
    }
}
