/// Image format conversions with zero-copy NumPy integration
use image::{DynamicImage, GrayImage, RgbImage, RgbaImage};
use ndarray::{Axis, s};
use numpy::{PyArray3, PyArrayMethods, PyReadonlyArray3};
use pyo3::prelude::*;

/// Convert RGB to grayscale using ITU-R BT.709 coefficients
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

/// Convert RGB to RGBA with specified alpha value
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

/// Convert RGBA to RGB by dropping alpha channel
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

/// Convert between image formats
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

/// Load image from bytes directly into NumPy array
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

/// Save NumPy array as image bytes
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
