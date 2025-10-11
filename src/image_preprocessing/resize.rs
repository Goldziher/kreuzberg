use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer, images::Image as FirImage};
use image::{DynamicImage, ImageBuffer, Rgb};
use ndarray::{Array3, ArrayView3};
use numpy::{PyArray3, ToPyArray};
use pyo3::prelude::*;

pub fn numpy_to_image(array: ArrayView3<'_, u8>) -> PyResult<DynamicImage> {
    let (height, width, channels) = array.dim();

    if channels != 3 {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Expected 3 channels (RGB), got {channels}"
        )));
    }

    let width = u32::try_from(width)
        .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Image width exceeds u32 limit"))?;
    let height = u32::try_from(height)
        .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Image height exceeds u32 limit"))?;

    let mut img = ImageBuffer::new(width, height);

    for (y, row) in array.outer_iter().enumerate() {
        for (x, pixel) in row.outer_iter().enumerate() {
            #[allow(clippy::many_single_char_names)]
            let (r, g, b) = (pixel[0], pixel[1], pixel[2]);

            let x = u32::try_from(x).unwrap_or(u32::MAX);
            let y = u32::try_from(y).unwrap_or(u32::MAX);

            if x < width && y < height {
                img.put_pixel(x, y, Rgb([r, g, b]));
            }
        }
    }

    Ok(DynamicImage::ImageRgb8(img))
}

pub fn image_to_numpy<'py>(py: Python<'py>, image: &DynamicImage) -> Bound<'py, PyArray3<u8>> {
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

    array.to_pyarray(py)
}

pub fn resize_image(
    image: &DynamicImage,
    new_width: u32,
    new_height: u32,
    scale_factor: f64,
) -> PyResult<DynamicImage> {
    let rgb_image = image.to_rgb8();
    let (width, height) = rgb_image.dimensions();

    let src_image = FirImage::from_vec_u8(width, height, rgb_image.into_raw(), PixelType::U8x3).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to create source image: {e:?}"))
    })?;

    let mut dst_image = FirImage::new(new_width, new_height, PixelType::U8x3);

    let algorithm = if scale_factor < 1.0 {
        ResizeAlg::Convolution(FilterType::Lanczos3)
    } else {
        ResizeAlg::Convolution(FilterType::CatmullRom)
    };

    let mut resizer = Resizer::new();
    resizer
        .resize(&src_image, &mut dst_image, &ResizeOptions::new().resize_alg(algorithm))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Resize failed: {e:?}")))?;

    let buffer = dst_image.into_vec();
    let img_buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(new_width, new_height, buffer)
        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to create image buffer"))?;

    Ok(DynamicImage::ImageRgb8(img_buffer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;
    use ndarray::Array3;

    fn create_test_image() -> DynamicImage {
        let mut img = ImageBuffer::new(100, 100);
        for y in 0..100 {
            for x in 0..100 {
                img.put_pixel(x, y, Rgb([255u8, 0u8, 0u8]));
            }
        }
        DynamicImage::ImageRgb8(img)
    }

    fn create_test_array() -> Array3<u8> {
        Array3::from_elem((100, 100, 3), 255)
    }

    #[test]
    fn test_numpy_to_image() {
        let array = create_test_array();
        let result = numpy_to_image(array.view());
        assert!(result.is_ok());
        let img = result.unwrap();
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 100);
    }

    #[test]
    fn test_numpy_to_image_invalid_channels() {
        let array = Array3::from_elem((100, 100, 4), 255);
        let result = numpy_to_image(array.view());
        assert!(result.is_err());
    }

    #[test]
    fn test_resize_image_downscale() {
        let img = create_test_image();
        let result = resize_image(&img, 50, 50, 0.5);
        assert!(result.is_ok());
        let resized = result.unwrap();
        assert_eq!(resized.width(), 50);
        assert_eq!(resized.height(), 50);
    }

    #[test]
    fn test_resize_image_upscale() {
        let img = create_test_image();
        let result = resize_image(&img, 200, 200, 2.0);
        assert!(result.is_ok());
        let resized = result.unwrap();
        assert_eq!(resized.width(), 200);
        assert_eq!(resized.height(), 200);
    }

    #[test]
    fn test_resize_image_no_scale() {
        let img = create_test_image();
        let result = resize_image(&img, 100, 100, 1.0);
        assert!(result.is_ok());
        let resized = result.unwrap();
        assert_eq!(resized.width(), 100);
        assert_eq!(resized.height(), 100);
    }

    #[test]
    fn test_numpy_to_image_dimension_overflow() {
        let large_width = u32::MAX as usize + 1;
        let array = Array3::<u8>::zeros((0, 0, 3))
            .into_shape_with_order((0, large_width, 3))
            .expect("reshape zero-sized array");
        let result = numpy_to_image(array.view());
        assert!(result.is_err());
    }

    #[test]
    fn test_resize_preserves_aspect_ratio() {
        let img = create_test_image();
        let result = resize_image(&img, 50, 50, 0.5);
        assert!(result.is_ok());
        let resized = result.unwrap();

        let original_aspect = img.width() as f64 / img.height() as f64;
        let resized_aspect = resized.width() as f64 / resized.height() as f64;
        assert!((original_aspect - resized_aspect).abs() < 0.01);
    }
}
