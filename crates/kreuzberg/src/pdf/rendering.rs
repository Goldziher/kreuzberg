use super::error::{PdfError, Result};
use image::DynamicImage;
use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};

const PDF_POINTS_PER_INCH: f64 = 72.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRenderOptions {
    pub target_dpi: i32,
    pub max_image_dimension: i32,
    pub auto_adjust_dpi: bool,
    pub min_dpi: i32,
    pub max_dpi: i32,
}

impl Default for PageRenderOptions {
    fn default() -> Self {
        Self {
            target_dpi: 300,
            max_image_dimension: 65536,
            auto_adjust_dpi: true,
            min_dpi: 72,
            max_dpi: 600,
        }
    }
}

pub struct PdfRenderer {
    pdfium: Pdfium,
}

impl PdfRenderer {
    pub fn new() -> Result<Self> {
        let binding = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library())
            .map_err(|e| PdfError::RenderingFailed(format!("Failed to initialize Pdfium: {}", e)))?;

        let pdfium = Pdfium::new(binding);
        Ok(Self { pdfium })
    }

    pub fn render_page_to_image(
        &self,
        pdf_bytes: &[u8],
        page_index: usize,
        options: &PageRenderOptions,
    ) -> Result<DynamicImage> {
        self.render_page_to_image_with_password(pdf_bytes, page_index, options, None)
    }

    pub fn render_page_to_image_with_password(
        &self,
        pdf_bytes: &[u8],
        page_index: usize,
        options: &PageRenderOptions,
        password: Option<&str>,
    ) -> Result<DynamicImage> {
        let document = self.pdfium.load_pdf_from_byte_slice(pdf_bytes, password).map_err(|e| {
            let err_msg = e.to_string();
            if (err_msg.contains("password") || err_msg.contains("Password")) && password.is_some() {
                PdfError::InvalidPassword
            } else if err_msg.contains("password") || err_msg.contains("Password") {
                PdfError::PasswordRequired
            } else {
                PdfError::InvalidPdf(err_msg)
            }
        })?;

        let page = document
            .pages()
            .get(page_index as u16)
            .map_err(|_| PdfError::PageNotFound(page_index))?;

        let width_points = page.width().value;
        let height_points = page.height().value;

        let dpi = if options.auto_adjust_dpi {
            calculate_optimal_dpi(
                width_points as f64,
                height_points as f64,
                options.target_dpi,
                options.max_image_dimension,
                options.min_dpi,
                options.max_dpi,
            )
        } else {
            options.target_dpi
        };

        let scale = dpi as f64 / PDF_POINTS_PER_INCH;

        let config = PdfRenderConfig::new()
            .set_target_width(((width_points * scale as f32) as i32).max(1))
            .set_target_height(((height_points * scale as f32) as i32).max(1))
            .rotate_if_landscape(PdfPageRenderRotation::None, false);

        let bitmap = page
            .render_with_config(&config)
            .map_err(|e| PdfError::RenderingFailed(format!("Failed to render page: {}", e)))?;

        let image = bitmap.as_image().into_rgb8();

        Ok(DynamicImage::ImageRgb8(image))
    }

    pub fn render_all_pages(&self, pdf_bytes: &[u8], options: &PageRenderOptions) -> Result<Vec<DynamicImage>> {
        self.render_all_pages_with_password(pdf_bytes, options, None)
    }

    pub fn render_all_pages_with_password(
        &self,
        pdf_bytes: &[u8],
        options: &PageRenderOptions,
        password: Option<&str>,
    ) -> Result<Vec<DynamicImage>> {
        let document = self.pdfium.load_pdf_from_byte_slice(pdf_bytes, password).map_err(|e| {
            let err_msg = e.to_string();
            if (err_msg.contains("password") || err_msg.contains("Password")) && password.is_some() {
                PdfError::InvalidPassword
            } else if err_msg.contains("password") || err_msg.contains("Password") {
                PdfError::PasswordRequired
            } else {
                PdfError::InvalidPdf(err_msg)
            }
        })?;

        let page_count = document.pages().len() as usize;
        let mut images = Vec::with_capacity(page_count);

        for page_index in 0..page_count {
            let image = self.render_page_to_image_with_password(pdf_bytes, page_index, options, password)?;
            images.push(image);
        }

        Ok(images)
    }
}

pub fn render_page_to_image(pdf_bytes: &[u8], page_index: usize, options: &PageRenderOptions) -> Result<DynamicImage> {
    let renderer = PdfRenderer::new()?;
    renderer.render_page_to_image(pdf_bytes, page_index, options)
}

#[allow(clippy::too_many_arguments)]
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

    let width_at_target = (width_inches * target_dpi as f64) as i32;
    let height_at_target = (height_inches * target_dpi as f64) as i32;

    if width_at_target <= max_dimension && height_at_target <= max_dimension {
        return target_dpi.clamp(min_dpi, max_dpi);
    }

    let width_limited_dpi = (max_dimension as f64 / width_inches) as i32;
    let height_limited_dpi = (max_dimension as f64 / height_inches) as i32;

    width_limited_dpi.min(height_limited_dpi).clamp(min_dpi, max_dpi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let result = PdfRenderer::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_invalid_pdf() {
        let renderer = PdfRenderer::new().unwrap();
        let options = PageRenderOptions::default();
        let result = renderer.render_page_to_image(b"not a pdf", 0, &options);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PdfError::InvalidPdf(_)));
    }

    #[test]
    fn test_render_page_not_found() {
        let renderer = PdfRenderer::new().unwrap();
        let options = PageRenderOptions::default();
        let minimal_pdf = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n";
        let result = renderer.render_page_to_image(minimal_pdf, 999, &options);

        if let Err(err) = result {
            assert!(matches!(
                err,
                PdfError::PageNotFound(_) | PdfError::InvalidPdf(_) | PdfError::PasswordRequired
            ));
        }
    }

    #[test]
    fn test_calculate_optimal_dpi_within_limits() {
        let dpi = calculate_optimal_dpi(612.0, 792.0, 300, 65536, 72, 600);
        assert!((72..=600).contains(&dpi));
    }

    #[test]
    fn test_calculate_optimal_dpi_oversized_page() {
        let dpi = calculate_optimal_dpi(10000.0, 10000.0, 300, 4096, 72, 600);
        assert!(dpi >= 72);
        assert!(dpi < 300);
    }

    #[test]
    fn test_calculate_optimal_dpi_min_clamp() {
        let dpi = calculate_optimal_dpi(100.0, 100.0, 10, 65536, 72, 600);
        assert_eq!(dpi, 72);
    }

    #[test]
    fn test_calculate_optimal_dpi_max_clamp() {
        let dpi = calculate_optimal_dpi(100.0, 100.0, 1000, 65536, 72, 600);
        assert_eq!(dpi, 600);
    }

    #[test]
    fn test_page_render_options_default() {
        let options = PageRenderOptions::default();
        assert_eq!(options.target_dpi, 300);
        assert_eq!(options.max_image_dimension, 65536);
        assert!(options.auto_adjust_dpi);
        assert_eq!(options.min_dpi, 72);
        assert_eq!(options.max_dpi, 600);
    }

    #[test]
    fn test_renderer_size() {
        assert!(size_of::<PdfRenderer>() > 0);
    }

    #[test]
    fn test_render_all_pages_empty_pdf() {
        let renderer = PdfRenderer::new().unwrap();
        let options = PageRenderOptions::default();
        let result = renderer.render_all_pages(b"not a pdf", &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_render_page_with_password_none() {
        let renderer = PdfRenderer::new().unwrap();
        let options = PageRenderOptions::default();
        let result = renderer.render_page_to_image_with_password(b"not a pdf", 0, &options, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_render_all_pages_with_password_none() {
        let renderer = PdfRenderer::new().unwrap();
        let options = PageRenderOptions::default();
        let result = renderer.render_all_pages_with_password(b"not a pdf", &options, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_render_page_to_image_function() {
        let options = PageRenderOptions::default();
        let result = render_page_to_image(b"not a pdf", 0, &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_optimal_dpi_tall_page() {
        let dpi = calculate_optimal_dpi(612.0, 10000.0, 300, 4096, 72, 600);
        assert!((72..=600).contains(&dpi));
    }

    #[test]
    fn test_calculate_optimal_dpi_wide_page() {
        let dpi = calculate_optimal_dpi(10000.0, 612.0, 300, 4096, 72, 600);
        assert!((72..=600).contains(&dpi));
    }

    #[test]
    fn test_calculate_optimal_dpi_square_page() {
        let dpi = calculate_optimal_dpi(1000.0, 1000.0, 300, 65536, 72, 600);
        assert!((72..=600).contains(&dpi));
    }

    #[test]
    fn test_calculate_optimal_dpi_tiny_page() {
        let dpi = calculate_optimal_dpi(72.0, 72.0, 300, 65536, 72, 600);
        assert_eq!(dpi, 300);
    }

    #[test]
    fn test_calculate_optimal_dpi_target_equals_max() {
        let dpi = calculate_optimal_dpi(612.0, 792.0, 600, 65536, 72, 600);
        assert_eq!(dpi, 600);
    }

    #[test]
    fn test_calculate_optimal_dpi_target_equals_min() {
        let dpi = calculate_optimal_dpi(612.0, 792.0, 72, 65536, 72, 600);
        assert_eq!(dpi, 72);
    }

    #[test]
    fn test_calculate_optimal_dpi_exactly_at_limit() {
        let page_size = 65536.0 / 300.0 * PDF_POINTS_PER_INCH;
        let dpi = calculate_optimal_dpi(page_size, page_size, 300, 65536, 72, 600);
        assert!((72..=600).contains(&dpi));
    }

    #[test]
    fn test_page_render_options_custom() {
        let options = PageRenderOptions {
            target_dpi: 150,
            max_image_dimension: 8192,
            auto_adjust_dpi: false,
            min_dpi: 50,
            max_dpi: 400,
        };

        assert_eq!(options.target_dpi, 150);
        assert_eq!(options.max_image_dimension, 8192);
        assert!(!options.auto_adjust_dpi);
        assert_eq!(options.min_dpi, 50);
        assert_eq!(options.max_dpi, 400);
    }

    #[test]
    fn test_page_render_options_clone() {
        let options1 = PageRenderOptions::default();
        let options2 = options1.clone();

        assert_eq!(options1.target_dpi, options2.target_dpi);
        assert_eq!(options1.max_image_dimension, options2.max_image_dimension);
        assert_eq!(options1.auto_adjust_dpi, options2.auto_adjust_dpi);
    }

    #[test]
    fn test_pdf_points_per_inch_constant() {
        assert_eq!(PDF_POINTS_PER_INCH, 72.0);
    }

    #[test]
    fn test_render_empty_bytes() {
        let renderer = PdfRenderer::new().unwrap();
        let options = PageRenderOptions::default();
        let result = renderer.render_page_to_image(&[], 0, &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_optimal_dpi_zero_target() {
        let dpi = calculate_optimal_dpi(612.0, 792.0, 0, 65536, 72, 600);
        assert_eq!(dpi, 72);
    }

    #[test]
    fn test_calculate_optimal_dpi_negative_target() {
        let dpi = calculate_optimal_dpi(612.0, 792.0, -100, 65536, 72, 600);
        assert_eq!(dpi, 72);
    }
}
