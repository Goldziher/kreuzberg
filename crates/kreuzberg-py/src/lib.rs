use pyo3::prelude::*;

mod bindings;
mod error;
mod types;

use bindings::{
    cache, chunking, email, excel, html, image_preprocessing, libreoffice, ocr, pandoc, plugins, pptx, structured,
    table, text, text_utils, xml,
};
use types::{
    PyEmailAttachment, PyEmailExtractionResult, PyExcelSheet, PyExcelWorkbook, PyExtractedInlineImage,
    PyHtmlExtractionResult, PyTextExtractionResult, PyXmlExtractionResult,
};

#[pymodule]
fn _internal_bindings(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Email extraction
    m.add_function(wrap_pyfunction!(email::extract_email_content, m)?)?;
    m.add_function(wrap_pyfunction!(email::parse_eml_content, m)?)?;
    m.add_function(wrap_pyfunction!(email::parse_msg_content, m)?)?;
    m.add_function(wrap_pyfunction!(email::build_email_text_output, m)?)?;
    m.add_class::<PyEmailExtractionResult>()?;
    m.add_class::<PyEmailAttachment>()?;

    // Excel extraction
    m.add_function(wrap_pyfunction!(excel::read_excel_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(excel::read_excel_file, m)?)?;
    m.add_class::<PyExcelWorkbook>()?;
    m.add_class::<PyExcelSheet>()?;

    // HTML extraction
    m.add_function(wrap_pyfunction!(html::convert_html_to_markdown, m)?)?;
    m.add_function(wrap_pyfunction!(html::process_html, m)?)?;
    m.add_class::<PyHtmlExtractionResult>()?;
    m.add_class::<PyExtractedInlineImage>()?;

    // PPTX extraction
    m.add_function(wrap_pyfunction!(pptx::extract_pptx_from_path_msgpack, m)?)?;
    m.add_function(wrap_pyfunction!(pptx::extract_pptx_from_bytes_msgpack, m)?)?;

    // Structured data extraction
    m.add_function(wrap_pyfunction!(structured::parse_json_msgpack, m)?)?;
    m.add_function(wrap_pyfunction!(structured::parse_yaml_msgpack, m)?)?;
    m.add_function(wrap_pyfunction!(structured::parse_toml_msgpack, m)?)?;

    // Text extraction
    m.add_function(wrap_pyfunction!(text::parse_text, m)?)?;
    m.add_class::<PyTextExtractionResult>()?;

    // XML extraction
    m.add_function(wrap_pyfunction!(xml::parse_xml, m)?)?;
    m.add_class::<PyXmlExtractionResult>()?;

    // Table utilities
    m.add_function(wrap_pyfunction!(table::table_from_arrow_to_markdown, m)?)?;

    // Pandoc extraction
    m.add_function(wrap_pyfunction!(pandoc::extract_with_pandoc_msgpack, m)?)?;
    m.add_function(wrap_pyfunction!(pandoc::extract_with_pandoc_from_bytes_msgpack, m)?)?;
    m.add_function(wrap_pyfunction!(pandoc::validate_pandoc_version, m)?)?;

    // LibreOffice conversion
    m.add_function(wrap_pyfunction!(libreoffice::check_libreoffice_available, m)?)?;
    m.add_function(wrap_pyfunction!(libreoffice::convert_doc_to_docx_msgpack, m)?)?;
    m.add_function(wrap_pyfunction!(libreoffice::convert_ppt_to_pptx_msgpack, m)?)?;

    // Image preprocessing
    image_preprocessing::register_image_preprocessing_functions(m)?;

    // Text utilities
    text_utils::register_text_utils_functions(m)?;

    // Cache utilities
    cache::register_cache_functions(m)?;

    // OCR
    ocr::register_ocr_functions(m)?;

    // Chunking
    chunking::register_chunking_functions(m)?;

    // Plugin registration
    plugins::register_plugin_functions(m)?;

    Ok(())
}
