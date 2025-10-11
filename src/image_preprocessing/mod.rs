mod compression;
mod config;
mod conversions;
mod dpi;
mod image_io;
mod metadata;
mod resize;

pub use compression::{compress_image_auto, compress_image_jpeg, compress_image_png};
pub use config::ExtractionConfigDTO;
pub use conversions::{
    convert_format, load_image_as_numpy, rgb_to_grayscale, rgb_to_rgba, rgba_to_rgb, save_numpy_as_image,
};
pub use dpi::calculate_optimal_dpi;
pub use image_io::{detect_image_format, load_image, save_image};
pub use metadata::ImagePreprocessingMetadataDTO;

use crate::error_utils::errors;
use ndarray::ArrayView3;
use numpy::{PyArray3, PyArrayMethods};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use self::dpi::calculate_smart_dpi;
use self::resize::{image_to_numpy, numpy_to_image, resize_image};

#[derive(Debug, Clone, Copy)]
struct ImageDimensions {
    width: u32,
    height: u32,
}

impl ImageDimensions {
    fn from_array_dims(width: usize, height: usize) -> PyResult<Self> {
        let width =
            u32::try_from(width).map_err(|_| errors::out_of_range("Width", &width, &0, &(u32::MAX as usize)))?;
        let height =
            u32::try_from(height).map_err(|_| errors::out_of_range("Height", &height, &0, &(u32::MAX as usize)))?;

        Ok(Self { width, height })
    }

    #[inline]
    fn validate_bounds(&self, max_dimension: u32) -> PyResult<()> {
        if self.width > max_dimension || self.height > max_dimension {
            return Err(errors::out_of_range(
                "Image dimensions",
                &format!("{}x{}", self.width, self.height),
                &"1x1",
                &format!("{}x{}", max_dimension, max_dimension),
            ));
        }
        Ok(())
    }
}

const PDF_POINTS_PER_INCH: f64 = 72.0;

#[pyfunction]
#[pyo3(signature = (image_array, config, dpi_info=None))]
#[allow(clippy::too_many_lines)]
pub fn normalize_image_dpi<'py>(
    py: Python<'py>,
    image_array: &Bound<'py, PyArray3<u8>>,
    config: &ExtractionConfigDTO,
    dpi_info: Option<&Bound<'py, PyDict>>,
) -> PyResult<(Bound<'py, PyArray3<u8>>, ImagePreprocessingMetadataDTO)> {
    let array_view = unsafe { image_array.as_array() };
    let (height, width, _channels) = array_view.dim();

    let dims = ImageDimensions::from_array_dims(width, height)?;
    dims.validate_bounds(65536)?;

    let (original_width, original_height) = (dims.width, dims.height);

    let current_dpi = extract_dpi_from_dict(dpi_info);
    let original_dpi = (current_dpi, current_dpi);

    let max_memory_mb = 2048.0;

    let (target_dpi, auto_adjusted, calculated_dpi) =
        calculate_target_dpi(original_width, original_height, current_dpi, config, max_memory_mb);

    let scale_factor = f64::from(target_dpi) / current_dpi;

    if !needs_resize(original_width, original_height, scale_factor, config) {
        return Ok(create_skip_result(
            image_array.clone(),
            original_width,
            original_height,
            original_dpi,
            config,
            target_dpi,
            scale_factor,
            auto_adjusted,
            calculated_dpi,
        ));
    }

    let (new_width, new_height, final_scale, dimension_clamped) =
        calculate_new_dimensions(original_width, original_height, scale_factor, config);

    perform_resize(
        py,
        array_view,
        original_width,
        original_height,
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

fn extract_dpi_from_dict(dpi_info: Option<&Bound<'_, PyDict>>) -> f64 {
    dpi_info.map_or(PDF_POINTS_PER_INCH, |dpi_dict| {
        dpi_dict
            .get_item("dpi")
            .ok()
            .flatten()
            .and_then(|val| val.extract::<f64>().ok())
            .unwrap_or(PDF_POINTS_PER_INCH)
    })
}

fn calculate_target_dpi(
    width: u32,
    height: u32,
    current_dpi: f64,
    config: &ExtractionConfigDTO,
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

fn needs_resize(width: u32, height: u32, scale_factor: f64, config: &ExtractionConfigDTO) -> bool {
    let max_dimension = width.max(height);
    let exceeds_max = i32::try_from(max_dimension).map_or(true, |dim| dim > config.max_image_dimension);

    (scale_factor - 1.0).abs() >= 0.05 || exceeds_max
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn calculate_new_dimensions(
    original_width: u32,
    original_height: u32,
    scale_factor: f64,
    config: &ExtractionConfigDTO,
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

#[allow(clippy::too_many_arguments)]
fn create_skip_result<'py>(
    image_array: Bound<'py, PyArray3<u8>>,
    width: u32,
    height: u32,
    original_dpi: (f64, f64),
    config: &ExtractionConfigDTO,
    target_dpi: i32,
    scale_factor: f64,
    auto_adjusted: bool,
    calculated_dpi: Option<i32>,
) -> (Bound<'py, PyArray3<u8>>, ImagePreprocessingMetadataDTO) {
    (
        image_array,
        ImagePreprocessingMetadataDTO {
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
    )
}

#[allow(clippy::too_many_arguments)]
fn perform_resize<'py>(
    py: Python<'py>,
    array_view: ArrayView3<'_, u8>,
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
    config: &ExtractionConfigDTO,
) -> PyResult<(Bound<'py, PyArray3<u8>>, ImagePreprocessingMetadataDTO)> {
    let image = numpy_to_image(array_view)?;

    let resized = resize_image(&image, new_width, new_height, final_scale)?;

    let result_array = image_to_numpy(py, &resized);

    let metadata = ImagePreprocessingMetadataDTO {
        original_dimensions: (original_width, original_height),
        original_dpi,
        target_dpi: config.target_dpi,
        scale_factor: final_scale,
        auto_adjusted,
        final_dpi: target_dpi,
        new_dimensions: Some((new_width, new_height)),
        resample_method: if final_scale < 1.0 { "LANCZOS3" } else { "CATMULLROM" }.to_string(),
        dimension_clamped,
        calculated_dpi,
        skipped_resize: false,
        resize_error: None,
    };

    Ok((result_array, metadata))
}

#[pyfunction]
pub fn batch_normalize_images<'py>(
    py: Python<'py>,
    images: Vec<Bound<'py, PyArray3<u8>>>,
    config: &ExtractionConfigDTO,
) -> PyResult<Vec<(Bound<'py, PyArray3<u8>>, ImagePreprocessingMetadataDTO)>> {
    images
        .into_iter()
        .map(|img| normalize_image_dpi(py, &img, config, None))
        .collect()
}
