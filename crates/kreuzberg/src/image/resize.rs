use crate::error::{KreuzbergError, Result};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer, images::Image as FirImage};
use image::{DynamicImage, ImageBuffer, Rgb};

/// Resize an image using fast_image_resize with appropriate algorithm based on scale factor
pub fn resize_image(image: &DynamicImage, new_width: u32, new_height: u32, scale_factor: f64) -> Result<DynamicImage> {
    let rgb_image = image.to_rgb8();
    let (width, height) = rgb_image.dimensions();

    let src_image = FirImage::from_vec_u8(width, height, rgb_image.into_raw(), PixelType::U8x3)
        .map_err(|e| KreuzbergError::Parsing(format!("Failed to create source image: {e:?}")))?;

    let mut dst_image = FirImage::new(new_width, new_height, PixelType::U8x3);

    // Choose algorithm based on scale factor
    let algorithm = if scale_factor < 1.0 {
        // Downscaling: use Lanczos3 for better quality
        ResizeAlg::Convolution(FilterType::Lanczos3)
    } else {
        // Upscaling: use CatmullRom for smoother results
        ResizeAlg::Convolution(FilterType::CatmullRom)
    };

    let mut resizer = Resizer::new();
    resizer
        .resize(&src_image, &mut dst_image, &ResizeOptions::new().resize_alg(algorithm))
        .map_err(|e| KreuzbergError::Parsing(format!("Resize failed: {e:?}")))?;

    let buffer = dst_image.into_vec();
    let img_buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(new_width, new_height, buffer)
        .ok_or_else(|| KreuzbergError::Parsing("Failed to create image buffer".to_string()))?;

    Ok(DynamicImage::ImageRgb8(img_buffer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    fn create_test_image() -> DynamicImage {
        let mut img = ImageBuffer::new(100, 100);
        for y in 0..100 {
            for x in 0..100 {
                img.put_pixel(x, y, Rgb([255u8, 0u8, 0u8]));
            }
        }
        DynamicImage::ImageRgb8(img)
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
