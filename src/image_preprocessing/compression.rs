use image::{DynamicImage, ImageEncoder, ImageError, codecs::jpeg::JpegEncoder, codecs::png::PngEncoder};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::io::Cursor;

pub fn compress_jpeg(image: &DynamicImage, quality: u8) -> Result<Vec<u8>, ImageError> {
    let mut cursor = Cursor::new(Vec::new());
    let rgb = image.to_rgb8();

    let encoder = JpegEncoder::new_with_quality(&mut cursor, quality);
    encoder.write_image(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8)?;

    Ok(cursor.into_inner())
}

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

pub fn compress_auto(image: &DynamicImage, target_size_kb: Option<u32>) -> Result<(Vec<u8>, String), ImageError> {
    let has_transparency = matches!(image, DynamicImage::ImageRgba8(_) | DynamicImage::ImageLumaA8(_));

    let (width, height) = (image.width(), image.height());
    let total_pixels = width * height;

    let rgb = image.to_rgb8();
    let mut unique_colors = std::collections::HashSet::new();
    let sample_rate = (total_pixels / 10000).max(1);

    for (i, pixel) in rgb.pixels().enumerate() {
        if i % sample_rate as usize == 0 {
            unique_colors.insert((pixel[0], pixel[1], pixel[2]));
            if unique_colors.len() > 1000 {
                break;
            }
        }
    }

    let is_photo = unique_colors.len() > 1000;

    let (compressed, format) = if has_transparency {
        let compressed = compress_png(image, image::codecs::png::CompressionType::Default)?;
        (compressed, "PNG")
    } else if is_photo {
        let quality = if let Some(target_kb) = target_size_kb {
            find_optimal_jpeg_quality(image, target_kb)?
        } else {
            85
        };
        let compressed = compress_jpeg(image, quality)?;
        (compressed, "JPEG")
    } else {
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

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb, Rgba};

    fn create_photo_image() -> DynamicImage {
        let img = ImageBuffer::from_fn(100, 100, |x, y| {
            Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
        });
        DynamicImage::ImageRgb8(img)
    }

    fn create_simple_image() -> DynamicImage {
        let img = ImageBuffer::from_fn(100, 100, |_, _| Rgb([255u8, 0u8, 0u8]));
        DynamicImage::ImageRgb8(img)
    }

    fn create_transparent_image() -> DynamicImage {
        let img = ImageBuffer::from_fn(100, 100, |_, _| Rgba([255u8, 0u8, 0u8, 128u8]));
        DynamicImage::ImageRgba8(img)
    }

    #[test]
    fn test_compress_jpeg_quality_85() {
        let img = create_photo_image();
        let result = compress_jpeg(&img, 85);
        assert!(result.is_ok());
        let compressed = result.unwrap();
        assert!(!compressed.is_empty());
    }

    #[test]
    fn test_compress_jpeg_quality_ranges() {
        let img = create_photo_image();

        let high_quality = compress_jpeg(&img, 95).unwrap();
        let mid_quality = compress_jpeg(&img, 85).unwrap();
        let low_quality = compress_jpeg(&img, 50).unwrap();

        assert!(high_quality.len() > mid_quality.len());
        assert!(mid_quality.len() > low_quality.len());
    }

    #[test]
    fn test_compress_png_default() {
        let img = create_simple_image();
        let result = compress_png(&img, image::codecs::png::CompressionType::Default);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_compress_png_compression_levels() {
        let img = create_photo_image();

        let fast = compress_png(&img, image::codecs::png::CompressionType::Fast).unwrap();
        let default = compress_png(&img, image::codecs::png::CompressionType::Default).unwrap();
        let best = compress_png(&img, image::codecs::png::CompressionType::Best).unwrap();

        assert!(best.len() <= default.len());
        assert!(default.len() <= fast.len());
    }

    #[test]
    fn test_compress_auto_transparent_uses_png() {
        let img = create_transparent_image();
        let (compressed, format) = compress_auto(&img, None).unwrap();
        assert_eq!(format, "PNG");
        assert!(!compressed.is_empty());
    }

    #[test]
    fn test_compress_auto_photo_uses_jpeg() {
        let img = create_photo_image();
        let (compressed, format) = compress_auto(&img, None).unwrap();
        assert_eq!(format, "JPEG");
        assert!(!compressed.is_empty());
    }

    #[test]
    fn test_compress_auto_simple_graphics() {
        let img = create_simple_image();
        let (compressed, format) = compress_auto(&img, None).unwrap();
        assert!(format == "PNG" || format == "JPEG");
        assert!(!compressed.is_empty());
    }

    #[test]
    fn test_compress_auto_with_target_size() {
        let img = create_photo_image();
        let (compressed, format) = compress_auto(&img, Some(50)).unwrap();
        assert_eq!(format, "JPEG");

        let size_kb = compressed.len() / 1024;
        assert!(size_kb <= 55);
    }

    #[test]
    fn test_find_optimal_jpeg_quality() {
        let img = create_photo_image();
        let quality = find_optimal_jpeg_quality(&img, 100).unwrap();
        assert!(quality >= 20);
        assert!(quality <= 95);

        let compressed = compress_jpeg(&img, quality).unwrap();
        let size_kb = compressed.len() / 1024;
        assert!(size_kb <= 105);
    }

    #[test]
    fn test_find_optimal_jpeg_quality_very_small_target() {
        let img = create_photo_image();
        let quality = find_optimal_jpeg_quality(&img, 5).unwrap();
        assert!(quality >= 20);
        assert!(quality <= 95);
    }

    #[test]
    fn test_find_optimal_jpeg_quality_large_target() {
        let img = create_simple_image();
        let quality = find_optimal_jpeg_quality(&img, 1000).unwrap();
        assert!(quality >= 20);
        assert!(quality <= 95);
    }

    #[test]
    fn test_compress_jpeg_small_image() {
        let img = ImageBuffer::from_fn(10, 10, |_, _| Rgb([128u8, 128u8, 128u8]));
        let dynamic_img = DynamicImage::ImageRgb8(img);
        let result = compress_jpeg(&dynamic_img, 85);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_compress_png_small_image() {
        let img = ImageBuffer::from_fn(10, 10, |_, _| Rgb([255u8, 0u8, 0u8]));
        let dynamic_img = DynamicImage::ImageRgb8(img);
        let result = compress_png(&dynamic_img, image::codecs::png::CompressionType::Default);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_roundtrip_jpeg() {
        let img = create_photo_image();
        let compressed = compress_jpeg(&img, 85).unwrap();
        let loaded = image::load_from_memory(&compressed).unwrap();
        assert_eq!(img.width(), loaded.width());
        assert_eq!(img.height(), loaded.height());
    }

    #[test]
    fn test_roundtrip_png() {
        let img = create_simple_image();
        let compressed = compress_png(&img, image::codecs::png::CompressionType::Default).unwrap();
        let loaded = image::load_from_memory(&compressed).unwrap();
        assert_eq!(img.width(), loaded.width());
        assert_eq!(img.height(), loaded.height());
    }
}
