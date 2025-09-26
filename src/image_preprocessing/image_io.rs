/// Image I/O operations - load, save, format detection
use image::{DynamicImage, ImageFormat, ImageResult};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::io::Cursor;

/// Detect image format from bytes
pub fn detect_format(bytes: &[u8]) -> Option<ImageFormat> {
    image::guess_format(bytes).ok()
}

/// Load image from bytes with format detection
pub fn load_image_from_bytes(bytes: &[u8]) -> ImageResult<DynamicImage> {
    if let Some(format) = detect_format(bytes) {
        image::load_from_memory_with_format(bytes, format)
    } else {
        image::load_from_memory(bytes)
    }
}

/// Save image to bytes in specified format
pub fn save_image_to_bytes(image: &DynamicImage, format: ImageFormat) -> ImageResult<Vec<u8>> {
    let mut cursor = Cursor::new(Vec::new());
    image.write_to(&mut cursor, format)?;
    Ok(cursor.into_inner())
}

/// Load image from bytes
#[pyfunction]
pub fn load_image<'py>(_py: Python<'py>, data: &Bound<'py, PyBytes>) -> PyResult<(Vec<u8>, u32, u32, String)> {
    let bytes = data.as_bytes();

    let image = load_image_from_bytes(bytes)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to load image: {}", e)))?;

    let width = image.width();
    let height = image.height();
    let format = match image.color() {
        image::ColorType::L8 => "L",
        image::ColorType::La8 => "LA",
        image::ColorType::Rgb8 => "RGB",
        image::ColorType::Rgba8 => "RGBA",
        _ => "UNKNOWN",
    }
    .to_string();

    let rgb_image = image.to_rgb8();
    let raw_bytes = rgb_image.into_raw();

    Ok((raw_bytes, width, height, format))
}

/// Save image to bytes
#[pyfunction]
pub fn save_image<'py>(
    py: Python<'py>,
    data: Vec<u8>,
    width: u32,
    height: u32,
    format_str: &str,
) -> PyResult<Bound<'py, PyBytes>> {
    use image::RgbImage;

    let image = RgbImage::from_raw(width, height, data).ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid image dimensions: {}x{}", width, height))
    })?;

    let dynamic_image = DynamicImage::ImageRgb8(image);

    let format = match format_str.to_lowercase().as_str() {
        "png" => ImageFormat::Png,
        "jpeg" | "jpg" => ImageFormat::Jpeg,
        "webp" => ImageFormat::WebP,
        "bmp" => ImageFormat::Bmp,
        "tiff" | "tif" => ImageFormat::Tiff,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unsupported format: {}",
                format_str
            )))
        }
    };

    let bytes = save_image_to_bytes(&dynamic_image, format)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to save image: {}", e)))?;

    Ok(PyBytes::new(py, &bytes))
}

/// Detect image format from bytes
#[pyfunction]
pub fn detect_image_format<'py>(data: &Bound<'py, PyBytes>) -> PyResult<String> {
    let bytes = data.as_bytes();

    let format = detect_format(bytes).ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("Unable to detect image format from provided bytes")
    })?;

    let format_str = match format {
        ImageFormat::Png => "PNG",
        ImageFormat::Jpeg => "JPEG",
        ImageFormat::Gif => "GIF",
        ImageFormat::WebP => "WEBP",
        ImageFormat::Bmp => "BMP",
        ImageFormat::Tiff => "TIFF",
        ImageFormat::Tga => "TGA",
        ImageFormat::Dds => "DDS",
        ImageFormat::Ico => "ICO",
        _ => "UNKNOWN",
    };

    Ok(format_str.to_string())
}
