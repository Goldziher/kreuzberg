/// Image compression with JPEG, PNG, and smart format selection
use image::{codecs::jpeg::JpegEncoder, codecs::png::PngEncoder, DynamicImage, ImageEncoder, ImageError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::io::Cursor;

/// Compress image as JPEG with quality control
pub fn compress_jpeg(image: &DynamicImage, quality: u8) -> Result<Vec<u8>, ImageError> {
    let mut cursor = Cursor::new(Vec::new());
    let rgb = image.to_rgb8();

    let encoder = JpegEncoder::new_with_quality(&mut cursor, quality);
    encoder.write_image(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8)?;

    Ok(cursor.into_inner())
}

/// Compress image as PNG with compression level
pub fn compress_png(
    image: &DynamicImage,
    compression_level: image::codecs::png::CompressionType,
) -> Result<Vec<u8>, ImageError> {
    let mut cursor = Cursor::new(Vec::new());
    let rgb = image.to_rgb8();

    let encoder =
        PngEncoder::new_with_quality(&mut cursor, compression_level, image::codecs::png::FilterType::Adaptive);

    encoder.write_image(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8)?;

    Ok(cursor.into_inner())
}

/// Automatic compression with format selection based on image characteristics
pub fn compress_auto(image: &DynamicImage, target_size_kb: Option<u32>) -> Result<(Vec<u8>, String), ImageError> {
    let has_transparency = matches!(image, DynamicImage::ImageRgba8(_) | DynamicImage::ImageLumaA8(_));

    // Analyze image characteristics
    let (width, height) = (image.width(), image.height());
    let total_pixels = width * height;

    // Sample pixels to determine complexity
    let rgb = image.to_rgb8();
    let mut unique_colors = std::collections::HashSet::new();
    let sample_rate = (total_pixels / 10000).max(1);

    for (i, pixel) in rgb.pixels().enumerate() {
        if i % sample_rate as usize == 0 {
            unique_colors.insert((pixel[0], pixel[1], pixel[2]));
            if unique_colors.len() > 1000 {
                break; // Photo-like
            }
        }
    }

    let is_photo = unique_colors.len() > 1000;

    // Choose format and compress
    let (compressed, format) = if has_transparency {
        // Must use PNG for transparency
        let compressed = compress_png(image, image::codecs::png::CompressionType::Default)?;
        (compressed, "PNG")
    } else if is_photo {
        // Use JPEG for photos
        let quality = if let Some(target_kb) = target_size_kb {
            find_optimal_jpeg_quality(image, target_kb)?
        } else {
            85
        };
        let compressed = compress_jpeg(image, quality)?;
        (compressed, "JPEG")
    } else {
        // Try both formats, use smaller
        let png = compress_png(image, image::codecs::png::CompressionType::Best)?;
        let jpeg = compress_jpeg(image, 90)?;

        if png.len() < jpeg.len() {
            (png, "PNG")
        } else {
            (jpeg, "JPEG")
        }
    };

    Ok((compressed, format.to_string()))
}

/// Binary search for optimal JPEG quality to meet size target
fn find_optimal_jpeg_quality(image: &DynamicImage, target_kb: u32) -> Result<u8, ImageError> {
    let target_bytes = target_kb as usize * 1024;
    let mut low = 20u8;
    let mut high = 95u8;
    let mut best_quality = 85u8;
    let mut best_size = usize::MAX;

    for _ in 0..8 {
        let mid = (low + high) / 2;
        let compressed = compress_jpeg(image, mid)?;
        let size = compressed.len();

        if size <= target_bytes {
            if size > best_size || best_size > target_bytes {
                best_quality = mid;
                best_size = size;
            }
            low = mid + 1;
        } else {
            high = mid - 1;
            if best_size > target_bytes {
                best_quality = mid;
                best_size = size;
            }
        }

        if low > high {
            break;
        }
    }

    Ok(best_quality)
}

// Python bindings

/// Compress image as JPEG
#[pyfunction]
#[pyo3(signature = (data, width, height, quality=85))]
pub fn compress_image_jpeg<'py>(
    py: Python<'py>,
    data: &Bound<'py, PyBytes>,
    width: u32,
    height: u32,
    quality: u8,
) -> PyResult<Bound<'py, PyBytes>> {
    use image::RgbImage;

    let bytes = data.as_bytes();
    let img = RgbImage::from_raw(width, height, bytes.to_vec()).ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Invalid image dimensions: {}x{} for {} bytes",
            width,
            height,
            bytes.len()
        ))
    })?;

    let dynamic_img = DynamicImage::ImageRgb8(img);
    let compressed = compress_jpeg(&dynamic_img, quality)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("JPEG compression failed: {}", e)))?;

    Ok(PyBytes::new(py, &compressed))
}

/// Compress image as PNG
#[pyfunction]
#[pyo3(signature = (data, width, height, compression=6))]
pub fn compress_image_png<'py>(
    py: Python<'py>,
    data: &Bound<'py, PyBytes>,
    width: u32,
    height: u32,
    compression: u8,
) -> PyResult<Bound<'py, PyBytes>> {
    use image::RgbImage;

    let compression_level = match compression {
        0..=2 => image::codecs::png::CompressionType::Fast,
        3..=6 => image::codecs::png::CompressionType::Default,
        _ => image::codecs::png::CompressionType::Best,
    };

    let bytes = data.as_bytes();
    let img = RgbImage::from_raw(width, height, bytes.to_vec()).ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Invalid image dimensions: {}x{} for {} bytes",
            width,
            height,
            bytes.len()
        ))
    })?;

    let dynamic_img = DynamicImage::ImageRgb8(img);
    let compressed = compress_png(&dynamic_img, compression_level)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("PNG compression failed: {}", e)))?;

    Ok(PyBytes::new(py, &compressed))
}

/// Automatic image compression with format selection
#[pyfunction]
#[pyo3(signature = (data, width, height, target_size_kb=None))]
pub fn compress_image_auto<'py>(
    py: Python<'py>,
    data: &Bound<'py, PyBytes>,
    width: u32,
    height: u32,
    target_size_kb: Option<u32>,
) -> PyResult<(Bound<'py, PyBytes>, String)> {
    use image::RgbImage;

    let bytes = data.as_bytes();
    let img = RgbImage::from_raw(width, height, bytes.to_vec()).ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Invalid image dimensions: {}x{} for {} bytes",
            width,
            height,
            bytes.len()
        ))
    })?;

    let dynamic_img = DynamicImage::ImageRgb8(img);
    let (compressed, format) = compress_auto(&dynamic_img, target_size_kb)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Auto compression failed: {}", e)))?;

    Ok((PyBytes::new(py, &compressed), format))
}
