#![doc = include_str!("../README.md")]
#![allow(clippy::doc_nested_refdefs)]
#![allow(dead_code)]
#![allow(deprecated)]

mod bindgen {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]

    #[cfg(pdfium_bindings_generated)]
    include!(concat!(env!("OUT_DIR"), "/pdfium_7678.rs"));
    #[cfg(not(pdfium_bindings_generated))]
    include!("bindgen/pdfium_7678.rs");

    pub(crate) type size_t = usize;
}

mod bindings;
mod error;
mod font_provider;
mod pdf;
mod pdfium;
mod utils;

/// A prelude for conveniently importing public `pdfium-render` definitions.
///
/// Only re-exports types actually used by the xberg crate.
/// Removed modules: form fields (field/), appearance_mode, annotation subtypes
/// popup/redacted/widget/xfa_widget/variable_text. Form highlighting functions
/// (highlight_*_form_fields) removed from PdfRenderConfig.
///
/// Usage:
/// ```
/// use pdfium_render::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        bindings::*,
        error::*,
        font_provider::FontDescriptor,
        pdf::bitmap::*,
        pdf::color::*,
        pdf::color_space::*,
        pdf::document::fonts::*,
        pdf::document::metadata::*,
        pdf::document::page::annotation::attachment_points::*,
        pdf::document::page::annotation::circle::*,
        pdf::document::page::annotation::free_text::*,
        pdf::document::page::annotation::highlight::*,
        pdf::document::page::annotation::ink::*,
        pdf::document::page::annotation::link::*,
        pdf::document::page::annotation::objects::*,
        pdf::document::page::annotation::square::*,
        pdf::document::page::annotation::squiggly::*,
        pdf::document::page::annotation::stamp::*,
        pdf::document::page::annotation::strikeout::*,
        pdf::document::page::annotation::text::*,
        pdf::document::page::annotation::underline::*,
        pdf::document::page::annotation::unsupported::*,
        pdf::document::page::annotation::{PdfPageAnnotation, PdfPageAnnotationCommon, PdfPageAnnotationType},
        pdf::document::page::annotations::*,
        pdf::document::page::boundaries::*,
        pdf::document::page::extraction::*,
        pdf::document::page::links::*,
        pdf::document::page::object::content_mark::*,
        pdf::document::page::object::content_marks::*,
        pdf::document::page::object::group::*,
        pdf::document::page::object::image::*,
        pdf::document::page::object::path::*,
        pdf::document::page::object::shading::*,
        pdf::document::page::object::text::*,
        pdf::document::page::object::unsupported::*,
        pdf::document::page::object::x_object_form::*,
        pdf::document::page::object::{
            PdfPageObject, PdfPageObjectBlendMode, PdfPageObjectCommon, PdfPageObjectLineCap, PdfPageObjectLineJoin,
            PdfPageObjectType,
        },
        pdf::document::page::objects::common::*,
        pdf::document::page::objects::*,
        pdf::document::page::paragraph::*,
        pdf::document::page::render_config::*,
        pdf::document::page::size::*,
        pdf::document::page::struct_element::*,
        pdf::document::page::struct_tree::*,
        pdf::document::page::text::char::*,
        pdf::document::page::text::chars::*,
        pdf::document::page::text::search::*,
        pdf::document::page::text::segment::*,
        pdf::document::page::text::segments::*,
        pdf::document::page::text::*,
        pdf::document::page::{PdfPage, PdfPageContentRegenerationStrategy, PdfPageOrientation, PdfPageRenderRotation},
        pdf::document::pages::*,
        pdf::document::permissions::*,
        pdf::document::{PdfDocument, PdfDocumentVersion},
        pdf::font::glyph::*,
        pdf::font::glyphs::*,
        pdf::font::*,
        pdf::link::*,
        pdf::matrix::*,
        pdf::path::clip_path::*,
        pdf::path::segment::*,
        pdf::path::segments::*,
        pdf::points::*,
        pdf::quad_points::*,
        pdf::rect::*,
        pdfium::*,
    };
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::utils::test::{test_bind_to_pdfium, test_fixture_path};
    use image_025::ImageFormat;
    use std::fs::File;
    use std::path::Path;

    #[test]
    #[cfg(not(pdfium_use_static))]
    fn test_readme_example() -> Result<(), PdfiumError> {
        fn export_pdf_to_jpegs(path: &impl AsRef<Path>, password: Option<&str>) -> Result<(), PdfiumError> {
            let pdfium = Pdfium;

            let document = pdfium.load_pdf_from_file(path, password)?;

            let render_config = PdfRenderConfig::new()
                .set_target_width(2000)
                .set_maximum_height(2000)
                .rotate_if_landscape(PdfPageRenderRotation::Degrees90, true);

            for (index, page) in document.pages().iter().enumerate() {
                page.render_with_config(&render_config)?
                    .as_image()?
                    .into_rgb8()
                    .save_with_format(format!("test-page-{}.jpg", index), ImageFormat::Jpeg)
                    .map_err(|_| PdfiumError::ImageError)?;
            }

            Ok(())
        }

        export_pdf_to_jpegs(&test_fixture_path("export-test.pdf"), None)
    }

    #[test]
    #[cfg(not(pdfium_use_static))]
    fn test_dynamic_bindings() -> Result<(), PdfiumError> {
        let pdfium = Pdfium;

        let document = pdfium.load_pdf_from_file(&test_fixture_path("form-test.pdf"), None)?;

        let render_config = PdfRenderConfig::new()
            .set_target_width(2000)
            .set_maximum_height(2000)
            .rotate_if_landscape(PdfPageRenderRotation::Degrees90, true)
            .render_form_data(true)
            .render_annotations(true);

        for (index, page) in document.pages().iter().enumerate() {
            let result = page
                .render_with_config(&render_config)?
                .as_image()?
                .into_rgb8()
                .save_with_format(format!("form-test-page-{}.jpg", index), ImageFormat::Jpeg);

            assert!(result.is_ok());
        }

        Ok(())
    }

    #[test]
    #[cfg(pdfium_use_static)]
    fn test_static_bindings() {
        Pdfium::bind_to_statically_linked_library().unwrap();
    }

    #[test]
    fn test_reader_lifetime() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        let filenames = ["form-test.pdf", "annotations-test.pdf"];

        for filename in filenames {
            let path = test_fixture_path(filename);
            let page_count = {
                let reader = File::open(&path).map_err(PdfiumError::IoError)?;

                let document = pdfium.load_pdf_from_reader(reader, None)?;

                document.pages().len()
            };

            println!("{} has {} pages", path.display(), page_count);
        }

        Ok(())
    }

    #[test]
    #[cfg(not(pdfium_use_static))]
    fn test_custom_font_paths_with_text_rendering() -> Result<(), PdfiumError> {
        let config = PdfiumConfig::new().set_user_font_paths(vec!["/usr/share/fonts/truetype/".to_string()]);

        let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library());

        match bindings {
            Ok(bindings) => {
                let pdfium = Pdfium::new_with_config(bindings, &config);

                let mut document = pdfium.create_new_pdf()?;
                let mut page = document.pages_mut().create_page_at_end(PdfPagePaperSize::a4())?;

                let font = document.fonts_mut().helvetica();
                let _text_obj = page.objects_mut().create_text_object(
                    PdfPoints::new(100.0),
                    PdfPoints::new(700.0),
                    "Testing custom font paths",
                    font,
                    PdfPoints::new(12.0),
                )?;

                assert!(page.objects().iter().count() > 0);

                Ok(())
            }
            Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => Ok(()),
            Err(e) => Err(e),
        }
    }

    #[test]
    #[cfg(not(pdfium_use_static))]
    fn test_empty_font_paths() -> Result<(), PdfiumError> {
        let config = PdfiumConfig::new();

        let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library());

        match bindings {
            Ok(bindings) => {
                let pdfium = Pdfium::new_with_config(bindings, &config);
                let document = pdfium.create_new_pdf()?;
                assert_eq!(document.pages().len(), 0);
                Ok(())
            }
            Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => Ok(()),
            Err(e) => Err(e),
        }
    }

    #[test]
    #[cfg(not(pdfium_use_static))]
    fn test_font_paths_with_null_bytes() -> Result<(), PdfiumError> {
        let config = PdfiumConfig::new().set_user_font_paths(vec![
            "/usr/share\0/fonts".to_string(),
            "/usr/share/fonts/truetype/".to_string(),
        ]);

        let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library());

        match bindings {
            Ok(bindings) => {
                let pdfium = Pdfium::new_with_config(bindings, &config);
                let document = pdfium.create_new_pdf()?;
                assert_eq!(document.pages().len(), 0);
                Ok(())
            }
            Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => Ok(()),
            Err(e) => Err(e),
        }
    }

    #[test]
    #[cfg(not(pdfium_use_static))]
    fn test_font_paths_nonexistent() -> Result<(), PdfiumError> {
        let config = PdfiumConfig::new().set_user_font_paths(vec![
            "/this/path/does/not/exist".to_string(),
            "/another/fake/path".to_string(),
        ]);

        let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library());

        match bindings {
            Ok(bindings) => {
                let pdfium = Pdfium::new_with_config(bindings, &config);
                let document = pdfium.create_new_pdf()?;
                assert_eq!(document.pages().len(), 0);
                Ok(())
            }
            Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => Ok(()),
            Err(e) => Err(e),
        }
    }

    #[test]
    #[cfg(not(pdfium_use_static))]
    fn test_default_config_uses_simple_initialization() -> Result<(), PdfiumError> {
        let config = PdfiumConfig::new();

        let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library());

        match bindings {
            Ok(bindings) => {
                let pdfium = Pdfium::new_with_config(bindings, &config);
                let document = pdfium.create_new_pdf()?;
                assert_eq!(document.pages().len(), 0);
                Ok(())
            }
            Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => Ok(()),
            Err(e) => Err(e),
        }
    }
}
