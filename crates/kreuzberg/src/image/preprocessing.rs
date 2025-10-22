use crate::error::{KreuzbergError, Result};
use crate::types::{ExtractionConfig, ImagePreprocessingMetadata};
use image::{DynamicImage, ImageBuffer, Rgb};

use super::dpi::calculate_smart_dpi;
use super::resize::resize_image;

const PDF_POINTS_PER_INCH: f64 = 72.0;

/// Result of image normalization
pub struct NormalizeResult {
    /// Processed RGB image data (height * width * 3 bytes)
    pub rgb_data: Vec<u8>,
    /// Image dimensions (width, height)
    pub dimensions: (usize, usize),
    /// Preprocessing metadata
    pub metadata: ImagePreprocessingMetadata,
}

/// Normalize image DPI based on extraction configuration
///
/// # Arguments
/// * `rgb_data` - RGB image data as a flat Vec<u8> (height * width * 3 bytes, row-major)
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `config` - Extraction configuration containing DPI settings
/// * `current_dpi` - Optional current DPI of the image (defaults to 72 if None)
///
/// # Returns
/// * `NormalizeResult` containing processed image data and metadata
pub fn normalize_image_dpi(
    rgb_data: &[u8],
    width: usize,
    height: usize,
    config: &ExtractionConfig,
    current_dpi: Option<f64>,
) -> Result<NormalizeResult> {
    if width > 65536 || height > 65536 {
        return Err(KreuzbergError::validation(format!(
            "Image dimensions {}x{} exceed maximum 65536x65536",
            width, height
        )));
    }

    let expected_size = height * width * 3;
    if rgb_data.len() != expected_size {
        return Err(KreuzbergError::validation(format!(
            "RGB data size {} does not match expected size {} for {}x{} image",
            rgb_data.len(),
            expected_size,
            width,
            height
        )));
    }

    let current_dpi = current_dpi.unwrap_or(PDF_POINTS_PER_INCH);
    let original_dpi = (current_dpi, current_dpi);
    let max_memory_mb = 2048.0;

    let (target_dpi, auto_adjusted, calculated_dpi) =
        calculate_target_dpi(width as u32, height as u32, current_dpi, config, max_memory_mb);

    let scale_factor = f64::from(target_dpi) / current_dpi;

    if !needs_resize(width as u32, height as u32, scale_factor, config) {
        return Ok(create_skip_result(
            rgb_data.to_vec(),
            width,
            height,
            original_dpi,
            config,
            target_dpi,
            scale_factor,
            auto_adjusted,
            calculated_dpi,
        ));
    }

    let (new_width, new_height, final_scale, dimension_clamped) =
        calculate_new_dimensions(width as u32, height as u32, scale_factor, config);

    perform_resize(
        rgb_data,
        width as u32,
        height as u32,
        new_width,
        new_height,
        final_scale,
        original_dpi,
        target_dpi,
        auto_adjusted,
        dimension_clamped,
        calculated_dpi,
        config,
    )
}

/// Calculate target DPI based on configuration
fn calculate_target_dpi(
    width: u32,
    height: u32,
    current_dpi: f64,
    config: &ExtractionConfig,
    max_memory_mb: f64,
) -> (i32, bool, Option<i32>) {
    if config.auto_adjust_dpi {
        let approx_width_points = f64::from(width) * PDF_POINTS_PER_INCH / current_dpi;
        let approx_height_points = f64::from(height) * PDF_POINTS_PER_INCH / current_dpi;

        let optimal_dpi = calculate_smart_dpi(
            approx_width_points,
            approx_height_points,
            config.target_dpi,
            config.max_image_dimension,
            max_memory_mb,
        );

        (optimal_dpi, optimal_dpi != config.target_dpi, Some(optimal_dpi))
    } else {
        (config.target_dpi, false, None)
    }
}

/// Check if resize is needed
fn needs_resize(width: u32, height: u32, scale_factor: f64, config: &ExtractionConfig) -> bool {
    let max_dimension = width.max(height);
    let exceeds_max = i32::try_from(max_dimension).map_or(true, |dim| dim > config.max_image_dimension);

    (scale_factor - 1.0).abs() >= 0.05 || exceeds_max
}

/// Calculate new dimensions after scaling
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn calculate_new_dimensions(
    original_width: u32,
    original_height: u32,
    scale_factor: f64,
    config: &ExtractionConfig,
) -> (u32, u32, f64, bool) {
    let mut new_width = (f64::from(original_width) * scale_factor).round() as u32;
    let mut new_height = (f64::from(original_height) * scale_factor).round() as u32;
    let mut final_scale = scale_factor;
    let mut dimension_clamped = false;

    let max_new_dimension = new_width.max(new_height);
    if let Ok(max_dim_i32) = i32::try_from(max_new_dimension)
        && max_dim_i32 > config.max_image_dimension
    {
        let dimension_scale = f64::from(config.max_image_dimension) / f64::from(max_new_dimension);
        new_width = (f64::from(new_width) * dimension_scale).round() as u32;
        new_height = (f64::from(new_height) * dimension_scale).round() as u32;
        final_scale *= dimension_scale;
        dimension_clamped = true;
    }

    (new_width, new_height, final_scale, dimension_clamped)
}

/// Create result when resize is skipped
#[allow(clippy::too_many_arguments)]
fn create_skip_result(
    rgb_data: Vec<u8>,
    width: usize,
    height: usize,
    original_dpi: (f64, f64),
    config: &ExtractionConfig,
    target_dpi: i32,
    scale_factor: f64,
    auto_adjusted: bool,
    calculated_dpi: Option<i32>,
) -> NormalizeResult {
    NormalizeResult {
        rgb_data,
        dimensions: (width, height),
        metadata: ImagePreprocessingMetadata {
            original_dimensions: (width, height),
            original_dpi,
            target_dpi: config.target_dpi,
            scale_factor,
            auto_adjusted,
            final_dpi: target_dpi,
            new_dimensions: None,
            resample_method: "NONE".to_string(),
            dimension_clamped: false,
            calculated_dpi,
            skipped_resize: true,
            resize_error: None,
        },
    }
}

/// Perform the actual resize operation
#[allow(clippy::too_many_arguments)]
fn perform_resize(
    rgb_data: &[u8],
    original_width: u32,
    original_height: u32,
    new_width: u32,
    new_height: u32,
    final_scale: f64,
    original_dpi: (f64, f64),
    target_dpi: i32,
    auto_adjusted: bool,
    dimension_clamped: bool,
    calculated_dpi: Option<i32>,
    config: &ExtractionConfig,
) -> Result<NormalizeResult> {
    let img_buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(original_width, original_height, rgb_data.to_vec())
        .ok_or_else(|| {
            KreuzbergError::parsing(format!(
                "Failed to create image buffer from {}x{} RGB data",
                original_width, original_height
            ))
        })?;

    let image = DynamicImage::ImageRgb8(img_buffer);

    let resized = resize_image(&image, new_width, new_height, final_scale)?;

    let rgb_image = resized.to_rgb8();
    let result_rgb_data = rgb_image.into_raw();

    let metadata = ImagePreprocessingMetadata {
        original_dimensions: (original_width as usize, original_height as usize),
        original_dpi,
        target_dpi: config.target_dpi,
        scale_factor: final_scale,
        auto_adjusted,
        final_dpi: target_dpi,
        new_dimensions: Some((new_width as usize, new_height as usize)),
        resample_method: if final_scale < 1.0 { "LANCZOS3" } else { "CATMULLROM" }.to_string(),
        dimension_clamped,
        calculated_dpi,
        skipped_resize: false,
        resize_error: None,
    };

    Ok(NormalizeResult {
        rgb_data: result_rgb_data,
        dimensions: (new_width as usize, new_height as usize),
        metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_rgb_data(width: usize, height: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(width * height * 3);
        for _ in 0..width * height {
            data.push(255);
            data.push(0);
            data.push(0);
        }
        data
    }

    #[test]
    fn test_normalize_image_dpi_skip_resize() {
        let config = ExtractionConfig {
            target_dpi: 72,
            max_image_dimension: 4096,
            auto_adjust_dpi: false,
            min_dpi: 72,
            max_dpi: 600,
        };

        let rgb_data = create_test_rgb_data(100, 100);
        let result = normalize_image_dpi(&rgb_data, 100, 100, &config, Some(72.0));

        assert!(result.is_ok());
        let normalized = result.unwrap();
        assert_eq!(normalized.dimensions, (100, 100));
        assert!(normalized.metadata.skipped_resize);
    }

    #[test]
    fn test_normalize_image_dpi_upscale() {
        let config = ExtractionConfig {
            target_dpi: 300,
            max_image_dimension: 4096,
            auto_adjust_dpi: false,
            min_dpi: 72,
            max_dpi: 600,
        };

        let rgb_data = create_test_rgb_data(100, 100);
        let result = normalize_image_dpi(&rgb_data, 100, 100, &config, Some(72.0));

        assert!(result.is_ok());
        let normalized = result.unwrap();
        assert!(!normalized.metadata.skipped_resize);
        assert!(normalized.dimensions.0 > 100);
        assert!(normalized.dimensions.1 > 100);
    }

    #[test]
    fn test_normalize_image_dpi_downscale() {
        let config = ExtractionConfig {
            target_dpi: 72,
            max_image_dimension: 4096,
            auto_adjust_dpi: false,
            min_dpi: 72,
            max_dpi: 600,
        };

        let rgb_data = create_test_rgb_data(1000, 1000);
        let result = normalize_image_dpi(&rgb_data, 1000, 1000, &config, Some(300.0));

        assert!(result.is_ok());
        let normalized = result.unwrap();
        assert!(!normalized.metadata.skipped_resize);
        assert!(normalized.dimensions.0 < 1000);
        assert!(normalized.dimensions.1 < 1000);
    }

    #[test]
    fn test_normalize_image_dpi_dimension_clamp() {
        let config = ExtractionConfig {
            target_dpi: 300,
            max_image_dimension: 500,
            auto_adjust_dpi: false,
            min_dpi: 72,
            max_dpi: 600,
        };

        let rgb_data = create_test_rgb_data(1000, 1000);
        let result = normalize_image_dpi(&rgb_data, 1000, 1000, &config, Some(300.0));

        assert!(result.is_ok());
        let normalized = result.unwrap();
        assert!(normalized.metadata.dimension_clamped);
        assert!(normalized.dimensions.0 <= 500);
        assert!(normalized.dimensions.1 <= 500);
    }

    #[test]
    fn test_normalize_image_dpi_auto_adjust() {
        let config = ExtractionConfig {
            target_dpi: 300,
            max_image_dimension: 4096,
            auto_adjust_dpi: true,
            min_dpi: 72,
            max_dpi: 600,
        };

        let rgb_data = create_test_rgb_data(100, 100);
        let result = normalize_image_dpi(&rgb_data, 100, 100, &config, Some(72.0));

        assert!(result.is_ok());
        let normalized = result.unwrap();
        assert!(normalized.metadata.calculated_dpi.is_some());
    }

    #[test]
    fn test_normalize_image_dpi_invalid_dimensions() {
        let config = ExtractionConfig::default();
        let rgb_data = create_test_rgb_data(100, 100);

        let result = normalize_image_dpi(&rgb_data, 100000, 100000, &config, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_image_dpi_invalid_data_size() {
        let config = ExtractionConfig::default();
        let rgb_data = vec![0u8; 100];

        let result = normalize_image_dpi(&rgb_data, 100, 100, &config, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_needs_resize_threshold() {
        let config = ExtractionConfig {
            target_dpi: 300,
            max_image_dimension: 4096,
            auto_adjust_dpi: false,
            min_dpi: 72,
            max_dpi: 600,
        };

        assert!(!needs_resize(100, 100, 1.02, &config));

        assert!(needs_resize(100, 100, 1.10, &config));
    }

    #[test]
    fn test_calculate_new_dimensions_no_clamp() {
        let config = ExtractionConfig::default();

        let (new_w, new_h, scale, clamped) = calculate_new_dimensions(100, 100, 2.0, &config);

        assert_eq!(new_w, 200);
        assert_eq!(new_h, 200);
        assert!((scale - 2.0).abs() < 0.01);
        assert!(!clamped);
    }

    #[test]
    fn test_calculate_new_dimensions_with_clamp() {
        let config = ExtractionConfig {
            target_dpi: 300,
            max_image_dimension: 100,
            auto_adjust_dpi: false,
            min_dpi: 72,
            max_dpi: 600,
        };

        let (new_w, new_h, _scale, clamped) = calculate_new_dimensions(100, 100, 2.0, &config);

        assert!(new_w <= 100);
        assert!(new_h <= 100);
        assert!(clamped);
    }
}
