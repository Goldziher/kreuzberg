use image::{DynamicImage, ImageFormat, ImageResult};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::io::Cursor;

pub fn detect_format(bytes: &[u8]) -> Option<ImageFormat> {
    image::guess_format(bytes).ok()
}

pub fn load_image_from_bytes(bytes: &[u8]) -> ImageResult<DynamicImage> {
    if let Some(format) = detect_format(bytes) {
        image::load_from_memory_with_format(bytes, format)
    } else {
        image::load_from_memory(bytes)
    }
}

pub fn save_image_to_bytes(image: &DynamicImage, format: ImageFormat) -> ImageResult<Vec<u8>> {
    let mut cursor = Cursor::new(Vec::new());
    image.write_to(&mut cursor, format)?;
    Ok(cursor.into_inner())
}

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
            )));
        }
    };

    let bytes = save_image_to_bytes(&dynamic_image, format)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to save image: {}", e)))?;

    Ok(PyBytes::new(py, &bytes))
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    fn create_test_png() -> Vec<u8> {
        let img = ImageBuffer::from_fn(10, 10, |_, _| Rgb([255u8, 0u8, 0u8]));
        let dynamic_img = DynamicImage::ImageRgb8(img);
        let mut bytes = Vec::new();
        dynamic_img
            .write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        bytes
    }

    fn create_test_jpeg() -> Vec<u8> {
        let img = ImageBuffer::from_fn(10, 10, |_, _| Rgb([0u8, 255u8, 0u8]));
        let dynamic_img = DynamicImage::ImageRgb8(img);
        let mut bytes = Vec::new();
        dynamic_img
            .write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Jpeg)
            .unwrap();
        bytes
    }

    #[test]
    fn test_detect_format_png() {
        let png_bytes = create_test_png();
        let format = detect_format(&png_bytes);
        assert!(format.is_some());
        assert_eq!(format.unwrap(), ImageFormat::Png);
    }

    #[test]
    fn test_detect_format_jpeg() {
        let jpeg_bytes = create_test_jpeg();
        let format = detect_format(&jpeg_bytes);
        assert!(format.is_some());
        assert_eq!(format.unwrap(), ImageFormat::Jpeg);
    }

    #[test]
    fn test_detect_format_invalid() {
        let invalid_bytes = vec![0u8; 100];
        let format = detect_format(&invalid_bytes);
        assert!(format.is_none());
    }

    #[test]
    fn test_load_image_from_bytes_png() {
        let png_bytes = create_test_png();
        let result = load_image_from_bytes(&png_bytes);
        assert!(result.is_ok());
        let img = result.unwrap();
        assert_eq!(img.width(), 10);
        assert_eq!(img.height(), 10);
    }

    #[test]
    fn test_load_image_from_bytes_jpeg() {
        let jpeg_bytes = create_test_jpeg();
        let result = load_image_from_bytes(&jpeg_bytes);
        assert!(result.is_ok());
        let img = result.unwrap();
        assert_eq!(img.width(), 10);
        assert_eq!(img.height(), 10);
    }

    #[test]
    fn test_load_image_from_bytes_invalid() {
        let invalid_bytes = vec![0u8; 100];
        let result = load_image_from_bytes(&invalid_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_save_image_to_bytes_png() {
        let img = ImageBuffer::from_fn(10, 10, |_, _| Rgb([255u8, 0u8, 0u8]));
        let dynamic_img = DynamicImage::ImageRgb8(img);
        let result = save_image_to_bytes(&dynamic_img, ImageFormat::Png);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_save_image_to_bytes_jpeg() {
        let img = ImageBuffer::from_fn(10, 10, |_, _| Rgb([255u8, 0u8, 0u8]));
        let dynamic_img = DynamicImage::ImageRgb8(img);
        let result = save_image_to_bytes(&dynamic_img, ImageFormat::Jpeg);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_save_image_to_bytes_webp() {
        let img = ImageBuffer::from_fn(10, 10, |_, _| Rgb([255u8, 0u8, 0u8]));
        let dynamic_img = DynamicImage::ImageRgb8(img);
        let result = save_image_to_bytes(&dynamic_img, ImageFormat::WebP);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_roundtrip_png() {
        let img = ImageBuffer::from_fn(50, 50, |x, y| {
            Rgb([(x % 255) as u8, (y % 255) as u8, ((x + y) % 255) as u8])
        });
        let dynamic_img = DynamicImage::ImageRgb8(img);

        let bytes = save_image_to_bytes(&dynamic_img, ImageFormat::Png).unwrap();
        let loaded = load_image_from_bytes(&bytes).unwrap();

        assert_eq!(dynamic_img.width(), loaded.width());
        assert_eq!(dynamic_img.height(), loaded.height());
    }

    #[test]
    fn test_roundtrip_jpeg() {
        let img = ImageBuffer::from_fn(50, 50, |_, _| Rgb([128u8, 128u8, 128u8]));
        let dynamic_img = DynamicImage::ImageRgb8(img);

        let bytes = save_image_to_bytes(&dynamic_img, ImageFormat::Jpeg).unwrap();
        let loaded = load_image_from_bytes(&bytes).unwrap();

        assert_eq!(dynamic_img.width(), loaded.width());
        assert_eq!(dynamic_img.height(), loaded.height());
    }
}
