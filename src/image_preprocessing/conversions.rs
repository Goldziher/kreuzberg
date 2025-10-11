use image::{DynamicImage, GrayImage, RgbImage, RgbaImage};
use ndarray::{Axis, s};
use numpy::{PyArray3, PyArrayMethods, PyReadonlyArray3};
use pyo3::prelude::*;

#[pyfunction]
pub fn rgb_to_grayscale<'py>(
    py: Python<'py>,
    rgb_array: PyReadonlyArray3<'py, u8>,
) -> PyResult<Bound<'py, PyArray3<u8>>> {
    let rgb = rgb_array.as_array();
    let (height, width, channels) = rgb.dim();

    if channels != 3 {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Expected 3 channels (RGB), got {}",
            channels
        )));
    }

    let gray_array = PyArray3::<u8>::zeros(py, (height, width, 1), false);
    let gray_slice = unsafe { gray_array.as_slice_mut()? };

    const R_COEFF: u32 = 54;
    const G_COEFF: u32 = 183;
    const B_COEFF: u32 = 19;

    rgb.axis_iter(Axis(0))
        .zip(gray_slice.chunks_exact_mut(width))
        .for_each(|(row, gray_row)| {
            for x in 0..width {
                let r = row[[x, 0]] as u32;
                let g = row[[x, 1]] as u32;
                let b = row[[x, 2]] as u32;
                gray_row[x] = ((r * R_COEFF + g * G_COEFF + b * B_COEFF) >> 8) as u8;
            }
        });

    Ok(gray_array)
}

#[pyfunction]
#[pyo3(signature = (rgb_array, alpha=255))]
pub fn rgb_to_rgba<'py>(
    py: Python<'py>,
    rgb_array: PyReadonlyArray3<'py, u8>,
    alpha: u8,
) -> PyResult<Bound<'py, PyArray3<u8>>> {
    let rgb = rgb_array.as_array();
    let (height, width, channels) = rgb.dim();

    if channels != 3 {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Expected 3 channels (RGB), got {}",
            channels
        )));
    }

    let rgba_array = PyArray3::<u8>::zeros(py, (height, width, 4), false);
    let rgba_slice = unsafe { rgba_array.as_slice_mut()? };

    for y in 0..height {
        let rgb_row = &rgb.slice(s![y, .., ..]);
        let rgba_row = &mut rgba_slice[y * width * 4..(y + 1) * width * 4];

        for x in 0..width {
            let dst_idx = x * 4;
            rgba_row[dst_idx] = rgb_row[[x, 0]];
            rgba_row[dst_idx + 1] = rgb_row[[x, 1]];
            rgba_row[dst_idx + 2] = rgb_row[[x, 2]];
            rgba_row[dst_idx + 3] = alpha;
        }
    }

    Ok(rgba_array)
}

#[pyfunction]
pub fn rgba_to_rgb<'py>(py: Python<'py>, rgba_array: PyReadonlyArray3<'py, u8>) -> PyResult<Bound<'py, PyArray3<u8>>> {
    let rgba = rgba_array.as_array();
    let (height, width, channels) = rgba.dim();

    if channels != 4 {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Expected 4 channels (RGBA), got {}",
            channels
        )));
    }

    let rgb_array = PyArray3::<u8>::zeros(py, (height, width, 3), false);
    let rgb_slice = unsafe { rgb_array.as_slice_mut()? };

    rgba.axis_iter(Axis(0)).enumerate().for_each(|(y, row)| {
        let rgb_row = &mut rgb_slice[y * width * 3..(y + 1) * width * 3];
        for x in 0..width {
            let dst_idx = x * 3;
            rgb_row[dst_idx] = row[[x, 0]];
            rgb_row[dst_idx + 1] = row[[x, 1]];
            rgb_row[dst_idx + 2] = row[[x, 2]];
        }
    });

    Ok(rgb_array)
}

#[pyfunction]
pub fn convert_format<'py>(
    py: Python<'py>,
    array: PyReadonlyArray3<'py, u8>,
    to_format: &str,
) -> PyResult<Bound<'py, PyArray3<u8>>> {
    let input = array.as_array();
    let (height, width, channels) = input.dim();

    match (channels, to_format.to_uppercase().as_str()) {
        (3, "RGBA") => rgb_to_rgba(py, array, 255),
        (4, "RGB") => rgba_to_rgb(py, array),
        (3, "L") | (3, "GRAY") | (3, "GRAYSCALE") => rgb_to_grayscale(py, array),
        (1, "RGB") => {
            let rgb_array = PyArray3::<u8>::zeros(py, (height, width, 3), false);
            let rgb_slice = unsafe { rgb_array.as_slice_mut()? };

            for y in 0..height {
                for x in 0..width {
                    let gray = input[[y, x, 0]];
                    let idx = (y * width + x) * 3;
                    rgb_slice[idx] = gray;
                    rgb_slice[idx + 1] = gray;
                    rgb_slice[idx + 2] = gray;
                }
            }

            Ok(rgb_array)
        }
        (1, "RGBA") => {
            let rgba_array = PyArray3::<u8>::zeros(py, (height, width, 4), false);
            let rgba_slice = unsafe { rgba_array.as_slice_mut()? };

            for y in 0..height {
                for x in 0..width {
                    let gray = input[[y, x, 0]];
                    let idx = (y * width + x) * 4;
                    rgba_slice[idx] = gray;
                    rgba_slice[idx + 1] = gray;
                    rgba_slice[idx + 2] = gray;
                    rgba_slice[idx + 3] = 255;
                }
            }

            Ok(rgba_array)
        }
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Unsupported conversion: {} channels to {}",
            channels, to_format
        ))),
    }
}

#[pyfunction]
pub fn load_image_as_numpy<'py>(py: Python<'py>, data: &[u8]) -> PyResult<Bound<'py, PyArray3<u8>>> {
    let img = image::load_from_memory(data)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to load image: {}", e)))?;

    let rgb = img.to_rgb8();
    let (width, height) = rgb.dimensions();

    let raw_vec = rgb.into_raw();

    let array = PyArray3::zeros(py, (height as usize, width as usize, 3), false);
    unsafe {
        let slice = array.as_slice_mut()?;
        slice.copy_from_slice(&raw_vec);
    }

    Ok(array)
}

#[pyfunction]
pub fn save_numpy_as_image<'py>(_py: Python<'py>, array: PyReadonlyArray3<'py, u8>, format: &str) -> PyResult<Vec<u8>> {
    let arr = array.as_array();
    let (height, width, channels) = arr.dim();

    let img = match channels {
        3 => {
            let mut buffer = Vec::with_capacity(height * width * 3);
            for y in 0..height {
                for x in 0..width {
                    buffer.push(arr[[y, x, 0]]);
                    buffer.push(arr[[y, x, 1]]);
                    buffer.push(arr[[y, x, 2]]);
                }
            }

            RgbImage::from_raw(width as u32, height as u32, buffer).map(DynamicImage::ImageRgb8)
        }
        4 => {
            let mut buffer = Vec::with_capacity(height * width * 4);
            for y in 0..height {
                for x in 0..width {
                    buffer.push(arr[[y, x, 0]]);
                    buffer.push(arr[[y, x, 1]]);
                    buffer.push(arr[[y, x, 2]]);
                    buffer.push(arr[[y, x, 3]]);
                }
            }

            RgbaImage::from_raw(width as u32, height as u32, buffer).map(DynamicImage::ImageRgba8)
        }
        1 => {
            let mut buffer = Vec::with_capacity(height * width);
            for y in 0..height {
                for x in 0..width {
                    buffer.push(arr[[y, x, 0]]);
                }
            }

            GrayImage::from_raw(width as u32, height as u32, buffer).map(DynamicImage::ImageLuma8)
        }
        _ => None,
    }
    .ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Invalid image: {}x{}x{} dimensions",
            height, width, channels
        ))
    })?;

    let mut cursor = std::io::Cursor::new(Vec::new());
    let format_enum = match format.to_uppercase().as_str() {
        "PNG" => image::ImageFormat::Png,
        "JPEG" | "JPG" => image::ImageFormat::Jpeg,
        "BMP" => image::ImageFormat::Bmp,
        "WEBP" => image::ImageFormat::WebP,
        "TIFF" | "TIF" => image::ImageFormat::Tiff,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unsupported image format: {}",
                format
            )));
        }
    };

    img.write_to(&mut cursor, format_enum).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to encode image as {}: {}", format, e))
    })?;

    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use ndarray::Array3;

    fn create_rgb_array() -> Array3<u8> {
        let mut arr = Array3::zeros((10, 10, 3));
        for y in 0..10 {
            for x in 0..10 {
                arr[[y, x, 0]] = 255;
                arr[[y, x, 1]] = 128;
                arr[[y, x, 2]] = 64;
            }
        }
        arr
    }

    fn create_rgba_array() -> Array3<u8> {
        let mut arr = Array3::zeros((10, 10, 4));
        for y in 0..10 {
            for x in 0..10 {
                arr[[y, x, 0]] = 255;
                arr[[y, x, 1]] = 128;
                arr[[y, x, 2]] = 64;
                arr[[y, x, 3]] = 255;
            }
        }
        arr
    }

    fn create_grayscale_array() -> Array3<u8> {
        let mut arr = Array3::zeros((10, 10, 1));
        for y in 0..10 {
            for x in 0..10 {
                arr[[y, x, 0]] = 128;
            }
        }
        arr
    }

    #[test]
    fn test_rgb_to_grayscale_conversion() {
        let rgb = create_rgb_array();

        Python::initialize();
        Python::attach(|py| {
            let rgb_array = PyArray3::from_array(py, &rgb);
            let readonly = rgb_array.readonly();
            let result = rgb_to_grayscale(py, readonly).unwrap();

            let gray = unsafe { result.as_array() };
            assert_eq!(gray.dim(), (10, 10, 1));

            let expected = (255u32 * 54 + 128 * 183 + 64 * 19) >> 8;
            assert_eq!(gray[[0, 0, 0]] as u32, expected);
        });
    }

    #[test]
    fn test_rgb_to_grayscale_wrong_channels() {
        let rgba = create_rgba_array();

        Python::initialize();
        Python::attach(|py| {
            let rgba_array = PyArray3::from_array(py, &rgba);
            let result = rgb_to_grayscale(py, rgba_array.readonly());
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_rgb_to_rgba_default_alpha() {
        let rgb = create_rgb_array();

        Python::initialize();
        Python::attach(|py| {
            let rgb_array = PyArray3::from_array(py, &rgb);
            let result = rgb_to_rgba(py, rgb_array.readonly(), 255).unwrap();

            let rgba = unsafe { result.as_array() };
            assert_eq!(rgba.dim(), (10, 10, 4));
            assert_eq!(rgba[[0, 0, 0]], 255);
            assert_eq!(rgba[[0, 0, 1]], 128);
            assert_eq!(rgba[[0, 0, 2]], 64);
            assert_eq!(rgba[[0, 0, 3]], 255);
        });
    }

    #[test]
    fn test_rgb_to_rgba_custom_alpha() {
        let rgb = create_rgb_array();

        Python::initialize();
        Python::attach(|py| {
            let rgb_array = PyArray3::from_array(py, &rgb);
            let result = rgb_to_rgba(py, rgb_array.readonly(), 128).unwrap();

            let rgba = unsafe { result.as_array() };
            assert_eq!(rgba[[0, 0, 3]], 128);
        });
    }

    #[test]
    fn test_rgb_to_rgba_wrong_channels() {
        let rgba = create_rgba_array();

        Python::initialize();
        Python::attach(|py| {
            let rgba_array = PyArray3::from_array(py, &rgba);
            let result = rgb_to_rgba(py, rgba_array.readonly(), 255);
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_rgba_to_rgb_conversion() {
        let rgba = create_rgba_array();

        Python::initialize();
        Python::attach(|py| {
            let rgba_array = PyArray3::from_array(py, &rgba);
            let result = rgba_to_rgb(py, rgba_array.readonly()).unwrap();

            let rgb = unsafe { result.as_array() };
            assert_eq!(rgb.dim(), (10, 10, 3));
            assert_eq!(rgb[[0, 0, 0]], 255);
            assert_eq!(rgb[[0, 0, 1]], 128);
            assert_eq!(rgb[[0, 0, 2]], 64);
        });
    }

    #[test]
    fn test_rgba_to_rgb_wrong_channels() {
        let rgb = create_rgb_array();

        Python::initialize();
        Python::attach(|py| {
            let rgb_array = PyArray3::from_array(py, &rgb);
            let result = rgba_to_rgb(py, rgb_array.readonly());
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_convert_format_rgb_to_rgba() {
        let rgb = create_rgb_array();

        Python::initialize();
        Python::attach(|py| {
            let rgb_array = PyArray3::from_array(py, &rgb);
            let result = convert_format(py, rgb_array.readonly(), "RGBA").unwrap();

            let rgba = unsafe { result.as_array() };
            assert_eq!(rgba.dim(), (10, 10, 4));
        });
    }

    #[test]
    fn test_convert_format_rgba_to_rgb() {
        let rgba = create_rgba_array();

        Python::initialize();
        Python::attach(|py| {
            let rgba_array = PyArray3::from_array(py, &rgba);
            let result = convert_format(py, rgba_array.readonly(), "RGB").unwrap();

            let rgb = unsafe { result.as_array() };
            assert_eq!(rgb.dim(), (10, 10, 3));
        });
    }

    #[test]
    fn test_convert_format_rgb_to_grayscale() {
        let rgb = create_rgb_array();

        Python::initialize();
        Python::attach(|py| {
            let rgb_array = PyArray3::from_array(py, &rgb);
            let result = convert_format(py, rgb_array.readonly(), "GRAY").unwrap();

            let gray = unsafe { result.as_array() };
            assert_eq!(gray.dim(), (10, 10, 1));
        });
    }

    #[test]
    fn test_convert_format_grayscale_to_rgb() {
        let gray = create_grayscale_array();

        Python::initialize();
        Python::attach(|py| {
            let gray_array = PyArray3::from_array(py, &gray);
            let result = convert_format(py, gray_array.readonly(), "RGB").unwrap();

            let rgb = unsafe { result.as_array() };
            assert_eq!(rgb.dim(), (10, 10, 3));
            assert_eq!(rgb[[0, 0, 0]], 128);
            assert_eq!(rgb[[0, 0, 1]], 128);
            assert_eq!(rgb[[0, 0, 2]], 128);
        });
    }

    #[test]
    fn test_convert_format_grayscale_to_rgba() {
        let gray = create_grayscale_array();

        Python::initialize();
        Python::attach(|py| {
            let gray_array = PyArray3::from_array(py, &gray);
            let result = convert_format(py, gray_array.readonly(), "RGBA").unwrap();

            let rgba = unsafe { result.as_array() };
            assert_eq!(rgba.dim(), (10, 10, 4));
            assert_eq!(rgba[[0, 0, 3]], 255);
        });
    }

    #[test]
    fn test_convert_format_unsupported() {
        let rgb = create_rgb_array();

        Python::initialize();
        Python::attach(|py| {
            let rgb_array = PyArray3::from_array(py, &rgb);
            let result = convert_format(py, rgb_array.readonly(), "CMYK");
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_load_image_as_numpy() {
        let img = ImageBuffer::from_fn(50, 50, |_, _| Rgb([255u8, 0u8, 0u8]));
        let dynamic_img = DynamicImage::ImageRgb8(img);
        let mut bytes = Vec::new();
        dynamic_img
            .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();

        Python::initialize();
        Python::attach(|py| {
            let result = load_image_as_numpy(py, &bytes).unwrap();
            let arr = unsafe { result.as_array() };
            assert_eq!(arr.dim(), (50, 50, 3));
        });
    }

    #[test]
    fn test_load_image_as_numpy_invalid() {
        let invalid_bytes = vec![0u8; 100];

        Python::initialize();
        Python::attach(|py| {
            let result = load_image_as_numpy(py, &invalid_bytes);
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_save_numpy_as_image_png() {
        let rgb = create_rgb_array();

        Python::initialize();
        Python::attach(|py| {
            let rgb_array = PyArray3::from_array(py, &rgb);
            let result = save_numpy_as_image(py, rgb_array.readonly(), "PNG").unwrap();
            assert!(!result.is_empty());

            let loaded = image::load_from_memory(&result).unwrap();
            assert_eq!(loaded.width(), 10);
            assert_eq!(loaded.height(), 10);
        });
    }

    #[test]
    fn test_save_numpy_as_image_jpeg() {
        let rgb = create_rgb_array();

        Python::initialize();
        Python::attach(|py| {
            let rgb_array = PyArray3::from_array(py, &rgb);
            let result = save_numpy_as_image(py, rgb_array.readonly(), "JPEG").unwrap();
            assert!(!result.is_empty());
        });
    }

    #[test]
    fn test_save_numpy_as_image_rgba() {
        let rgba = create_rgba_array();

        Python::initialize();
        Python::attach(|py| {
            let rgba_array = PyArray3::from_array(py, &rgba);
            let result = save_numpy_as_image(py, rgba_array.readonly(), "PNG").unwrap();
            assert!(!result.is_empty());
        });
    }

    #[test]
    fn test_save_numpy_as_image_grayscale() {
        let gray = create_grayscale_array();

        Python::initialize();
        Python::attach(|py| {
            let gray_array = PyArray3::from_array(py, &gray);
            let result = save_numpy_as_image(py, gray_array.readonly(), "PNG").unwrap();
            assert!(!result.is_empty());
        });
    }

    #[test]
    fn test_save_numpy_as_image_unsupported_format() {
        let rgb = create_rgb_array();

        Python::initialize();
        Python::attach(|py| {
            let rgb_array = PyArray3::from_array(py, &rgb);
            let result = save_numpy_as_image(py, rgb_array.readonly(), "XYZ");
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_roundtrip_load_save_numpy() {
        let original = create_rgb_array();

        Python::initialize();
        Python::attach(|py| {
            let original_array = PyArray3::from_array(py, &original);
            let bytes = save_numpy_as_image(py, original_array.readonly(), "PNG").unwrap();
            let loaded = load_image_as_numpy(py, &bytes).unwrap();
            let loaded_arr = unsafe { loaded.as_array() };

            assert_eq!(original.dim(), loaded_arr.dim());
        });
    }
}
