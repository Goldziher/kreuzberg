use fast_image_resize::{images::Image as FirImage, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::{DynamicImage, ImageBuffer, Rgb};
use ndarray::{Array3, ArrayView3};
use numpy::{PyArray3, PyArrayMethods, ToPyArray};
use pyo3::prelude::*;
use pyo3::types::PyDict;

const PDF_POINTS_PER_INCH: f64 = 72.0;

/// Image preprocessing metadata matching Python's ImagePreprocessingMetadata
#[pyclass]
#[derive(Debug, Clone)]
pub struct ImagePreprocessingMetadata {
    #[pyo3(get)]
    pub original_dimensions: (u32, u32),
    #[pyo3(get)]
    pub original_dpi: (f64, f64),
    #[pyo3(get)]
    pub target_dpi: i32,
    #[pyo3(get)]
    pub scale_factor: f64,
    #[pyo3(get)]
    pub auto_adjusted: bool,
    #[pyo3(get)]
    pub final_dpi: i32,
    #[pyo3(get)]
    pub new_dimensions: Option<(u32, u32)>,
    #[pyo3(get)]
    pub resample_method: String,
    #[pyo3(get)]
    pub dimension_clamped: bool,
    #[pyo3(get)]
    pub calculated_dpi: Option<i32>,
    #[pyo3(get)]
    pub skipped_resize: bool,
    #[pyo3(get)]
    pub resize_error: Option<String>,
}

/// Configuration for image extraction matching Python's ExtractionConfig
#[pyclass]
#[derive(Debug, Clone)]
pub struct ExtractionConfig {
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
impl ExtractionConfig {
    #[new]
    #[pyo3(signature = (target_dpi=300, max_image_dimension=4096, auto_adjust_dpi=false, min_dpi=72, max_dpi=600))]
    fn new(target_dpi: i32, max_image_dimension: i32, auto_adjust_dpi: bool, min_dpi: i32, max_dpi: i32) -> Self {
        ExtractionConfig {
            target_dpi,
            max_image_dimension,
            auto_adjust_dpi,
            min_dpi,
            max_dpi,
        }
    }
}

/// Calculate optimal DPI based on page dimensions and constraints
#[allow(dead_code)]
fn calculate_optimal_dpi(
    page_width: f64,
    page_height: f64,
    target_dpi: i32,
    max_dimension: i32,
    min_dpi: i32,
    max_dpi: i32,
) -> i32 {
    let width_inches = page_width / PDF_POINTS_PER_INCH;
    let height_inches = page_height / PDF_POINTS_PER_INCH;

    let target_width_pixels = (width_inches * target_dpi as f64) as i32;
    let target_height_pixels = (height_inches * target_dpi as f64) as i32;

    let max_pixel_dimension = target_width_pixels.max(target_height_pixels);

    if max_pixel_dimension <= max_dimension {
        return min_dpi.max(target_dpi.min(max_dpi));
    }

    let max_dpi_for_width = if width_inches > 0.0 {
        (max_dimension as f64 / width_inches) as i32
    } else {
        max_dpi
    };

    let max_dpi_for_height = if height_inches > 0.0 {
        (max_dimension as f64 / height_inches) as i32
    } else {
        max_dpi
    };

    let constrained_dpi = max_dpi_for_width.min(max_dpi_for_height);
    min_dpi.max(constrained_dpi.min(max_dpi))
}

/// Calculate smart DPI that respects memory constraints
fn calculate_smart_dpi(
    page_width: f64,
    page_height: f64,
    target_dpi: i32,
    max_dimension: i32,
    max_memory_mb: f64,
) -> i32 {
    let width_inches = page_width / PDF_POINTS_PER_INCH;
    let height_inches = page_height / PDF_POINTS_PER_INCH;

    // Calculate what DPI would fit in memory
    let max_pixels = (max_memory_mb * 1024.0 * 1024.0 / 3.0).sqrt() as i32;

    let max_dpi_for_memory_width = if width_inches > 0.0 {
        (max_pixels as f64 / width_inches) as i32
    } else {
        target_dpi
    };

    let max_dpi_for_memory_height = if height_inches > 0.0 {
        (max_pixels as f64 / height_inches) as i32
    } else {
        target_dpi
    };

    let memory_constrained_dpi = max_dpi_for_memory_width.min(max_dpi_for_memory_height);

    // Also respect dimension constraint
    let target_width_pixels = (width_inches * target_dpi as f64) as i32;
    let target_height_pixels = (height_inches * target_dpi as f64) as i32;
    let max_pixel_dimension = target_width_pixels.max(target_height_pixels);

    let dimension_constrained_dpi = if max_pixel_dimension > max_dimension {
        let max_dpi_for_width = if width_inches > 0.0 {
            (max_dimension as f64 / width_inches) as i32
        } else {
            target_dpi
        };
        let max_dpi_for_height = if height_inches > 0.0 {
            (max_dimension as f64 / height_inches) as i32
        } else {
            target_dpi
        };
        max_dpi_for_width.min(max_dpi_for_height)
    } else {
        target_dpi
    };

    // Use the most restrictive constraint
    let final_dpi = target_dpi.min(memory_constrained_dpi).min(dimension_constrained_dpi);
    final_dpi.max(72) // Never go below 72 DPI
}

/// Convert numpy array to Rust image for processing
fn numpy_to_image(array: ArrayView3<u8>) -> Result<DynamicImage, String> {
    let (height, width, channels) = array.dim();

    if channels != 3 {
        return Err(format!("Expected 3 channels (RGB), got {}", channels));
    }

    let mut img = ImageBuffer::new(width as u32, height as u32);

    for y in 0..height {
        for x in 0..width {
            let r = array[(y, x, 0)];
            let g = array[(y, x, 1)];
            let b = array[(y, x, 2)];
            img.put_pixel(x as u32, y as u32, Rgb([r, g, b]));
        }
    }

    Ok(DynamicImage::ImageRgb8(img))
}

/// Convert Rust image back to numpy array
fn image_to_numpy<'py>(py: Python<'py>, image: &DynamicImage) -> Result<Bound<'py, PyArray3<u8>>, String> {
    let rgb_image = image.to_rgb8();
    let (width, height) = rgb_image.dimensions();

    let mut array = Array3::<u8>::zeros((height as usize, width as usize, 3));

    for y in 0..height {
        for x in 0..width {
            let pixel = rgb_image.get_pixel(x, y);
            array[(y as usize, x as usize, 0)] = pixel[0];
            array[(y as usize, x as usize, 1)] = pixel[1];
            array[(y as usize, x as usize, 2)] = pixel[2];
        }
    }

    Ok(array.to_pyarray(py))
}

/// High-performance image resize using fast_image_resize
fn resize_image_fast(
    image: &DynamicImage,
    new_width: u32,
    new_height: u32,
    scale_factor: f64,
) -> Result<DynamicImage, String> {
    let rgb_image = image.to_rgb8();
    let (width, height) = rgb_image.dimensions();

    // Create source image for fast_image_resize
    let src_image = FirImage::from_vec_u8(width, height, rgb_image.into_raw(), PixelType::U8x3)
        .map_err(|e| format!("Failed to create source image: {:?}", e))?;

    // Create destination image
    let mut dst_image = FirImage::new(new_width, new_height, PixelType::U8x3);

    // Choose algorithm based on scale factor
    let algorithm = if scale_factor < 1.0 {
        ResizeAlg::Convolution(fast_image_resize::FilterType::Lanczos3) // High quality downsampling
    } else {
        ResizeAlg::Convolution(fast_image_resize::FilterType::CatmullRom) // Good for upsampling
    };

    // Create resizer and perform resize
    let mut resizer = Resizer::new();
    resizer
        .resize(&src_image, &mut dst_image, &ResizeOptions::new().resize_alg(algorithm))
        .map_err(|e| format!("Resize failed: {:?}", e))?;

    // Convert back to DynamicImage
    let buffer = dst_image.into_vec();
    let img_buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(new_width, new_height, buffer)
        .ok_or_else(|| "Failed to create image buffer".to_string())?;

    Ok(DynamicImage::ImageRgb8(img_buffer))
}

/// Main image preprocessing function - Rust implementation
#[pyfunction]
#[pyo3(signature = (image_array, config, dpi_info=None))]
pub fn normalize_image_dpi_rust<'py>(
    py: Python<'py>,
    image_array: &Bound<'py, PyArray3<u8>>,
    config: &ExtractionConfig,
    dpi_info: Option<&Bound<'py, PyDict>>,
) -> PyResult<(Bound<'py, PyArray3<u8>>, ImagePreprocessingMetadata)> {
    // Get array dimensions
    let array_view = unsafe { image_array.as_array() };
    let (height, width, _channels) = array_view.dim();
    let original_width = width as u32;
    let original_height = height as u32;

    // Extract DPI from dict if provided, otherwise use defaults
    let current_dpi = if let Some(dpi_dict) = dpi_info {
        if let Ok(Some(dpi_val)) = dpi_dict.get_item("dpi") {
            dpi_val.extract::<f64>().unwrap_or(PDF_POINTS_PER_INCH)
        } else {
            PDF_POINTS_PER_INCH
        }
    } else {
        PDF_POINTS_PER_INCH
    };

    let original_dpi = (current_dpi, current_dpi);

    // Memory limits (matching Python implementation)
    let max_memory_mb = 2048.0; // 2GB cap

    // Calculate target DPI
    let (target_dpi, auto_adjusted, calculated_dpi) = if config.auto_adjust_dpi {
        let approx_width_points = original_width as f64 * PDF_POINTS_PER_INCH / current_dpi;
        let approx_height_points = original_height as f64 * PDF_POINTS_PER_INCH / current_dpi;

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
    };

    let scale_factor = target_dpi as f64 / current_dpi;

    // Check if resize is needed
    let max_current_dimension = original_width.max(original_height) as i32;
    let needs_resize = (scale_factor - 1.0).abs() >= 0.05 || max_current_dimension > config.max_image_dimension;

    if !needs_resize {
        // Return original array with metadata
        return Ok((
            image_array.clone(),
            ImagePreprocessingMetadata {
                original_dimensions: (original_width, original_height),
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
        ));
    }

    // Calculate new dimensions
    let mut new_width = (original_width as f64 * scale_factor) as u32;
    let mut new_height = (original_height as f64 * scale_factor) as u32;
    let mut final_scale = scale_factor;
    let mut dimension_clamped = false;

    // Apply dimension constraints
    let max_new_dimension = new_width.max(new_height) as i32;
    if max_new_dimension > config.max_image_dimension {
        let dimension_scale = config.max_image_dimension as f64 / max_new_dimension as f64;
        new_width = (new_width as f64 * dimension_scale) as u32;
        new_height = (new_height as f64 * dimension_scale) as u32;
        final_scale *= dimension_scale;
        dimension_clamped = true;
    }

    // Convert numpy array to image
    let image = numpy_to_image(array_view).map_err(PyErr::new::<pyo3::exceptions::PyValueError, _>)?;

    // Perform resize using fast_image_resize
    let resized = resize_image_fast(&image, new_width, new_height, final_scale)
        .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)?;

    // Convert back to numpy array
    let result_array = image_to_numpy(py, &resized).map_err(PyErr::new::<pyo3::exceptions::PyValueError, _>)?;

    let metadata = ImagePreprocessingMetadata {
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

/// Batch processing function for multiple images
#[pyfunction]
pub fn batch_normalize_images_rust<'py>(
    py: Python<'py>,
    images: Vec<Bound<'py, PyArray3<u8>>>,
    config: &ExtractionConfig,
) -> PyResult<Vec<(Bound<'py, PyArray3<u8>>, ImagePreprocessingMetadata)>> {
    // Process images sequentially for now (PyO3 GIL limitations with parallel processing)
    let mut results = Vec::new();
    for img in images.iter() {
        let result = normalize_image_dpi_rust(py, img, config, None)?;
        results.push(result);
    }
    Ok(results)
}
