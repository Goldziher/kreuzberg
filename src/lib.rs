use pyo3::prelude::*;

mod cache;
mod chunking;
mod common;
mod email;
mod error_utils;
mod excel;
mod html;
mod image_preprocessing;
mod ocr;
mod pptx;
mod quality;
mod string_utils;
mod table_processing;
mod text;
mod token_reduction;
mod xml;

use cache::{
    CacheStats, GenericCache, batch_cleanup_caches, batch_generate_cache_keys, cleanup_cache, clear_cache_directory,
    fast_hash, filter_old_cache_entries, generate_cache_key, get_available_disk_space, get_cache_metadata,
    is_cache_valid, smart_cleanup_cache, sort_cache_by_access_time, validate_cache_key,
};
use chunking::register_chunking;
use email::{
    EmailAttachmentDTO, EmailExtractionResultDTO, build_email_text_output, extract_email_content,
    extract_email_from_file, extract_eml_content, extract_msg_content, get_supported_email_formats,
    validate_email_content,
};
use excel::{
    ExcelSheetDTO, ExcelWorkbookDTO, benchmark_excel_reading, excel_to_markdown, read_excel_bytes, read_excel_file,
};
use html::{convert_html_to_markdown, process_html};
use image_preprocessing::{
    ExtractionConfigDTO, ImagePreprocessingMetadataDTO, batch_normalize_images, calculate_optimal_dpi,
    compress_image_auto, compress_image_jpeg, compress_image_png, convert_format, detect_image_format, load_image,
    load_image_as_numpy, normalize_image_dpi, rgb_to_grayscale, rgb_to_rgba, rgba_to_rgb, save_image,
    save_numpy_as_image,
};
use ocr::{
    BatchItemResult, ExtractionResultDTO, OCRCacheStats, OCRProcessor, PSMMode, TableDTO, TesseractConfigDTO,
    validate_language_code, validate_tesseract_version,
};
use pptx::extractor::PptxExtractorDTO;
use pptx::streaming::extractor::StreamingPptxExtractorDTO;
use pptx::types::{PptxExtractionResultDTO, PptxMetadataDTO};
use quality::{calculate_quality_score, clean_extracted_text, normalize_spaces};
use string_utils::{batch_process_texts, calculate_text_confidence, fix_mojibake, get_encoding_cache_key, safe_decode};
use table_processing::table_from_arrow_to_markdown;
use token_reduction::{
    ReductionLevelDTO, TokenReductionConfigDTO, batch_reduce_tokens, get_reduction_statistics, reduce_tokens,
};

#[pymodule]
fn _internal_bindings(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(calculate_quality_score, m)?)?;
    m.add_function(wrap_pyfunction!(clean_extracted_text, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_spaces, m)?)?;

    m.add_function(wrap_pyfunction!(safe_decode, m)?)?;
    m.add_function(wrap_pyfunction!(batch_process_texts, m)?)?;
    m.add_function(wrap_pyfunction!(calculate_text_confidence, m)?)?;
    m.add_function(wrap_pyfunction!(fix_mojibake, m)?)?;
    m.add_function(wrap_pyfunction!(get_encoding_cache_key, m)?)?;

    m.add_function(wrap_pyfunction!(normalize_image_dpi, m)?)?;
    m.add_function(wrap_pyfunction!(batch_normalize_images, m)?)?;
    m.add_function(wrap_pyfunction!(calculate_optimal_dpi, m)?)?;
    m.add_class::<ImagePreprocessingMetadataDTO>()?;
    m.add_class::<ExtractionConfigDTO>()?;

    m.add_function(wrap_pyfunction!(load_image, m)?)?;
    m.add_function(wrap_pyfunction!(save_image, m)?)?;
    m.add_function(wrap_pyfunction!(detect_image_format, m)?)?;
    m.add_function(wrap_pyfunction!(load_image_as_numpy, m)?)?;
    m.add_function(wrap_pyfunction!(save_numpy_as_image, m)?)?;

    m.add_function(wrap_pyfunction!(compress_image_jpeg, m)?)?;
    m.add_function(wrap_pyfunction!(compress_image_png, m)?)?;
    m.add_function(wrap_pyfunction!(compress_image_auto, m)?)?;

    m.add_function(wrap_pyfunction!(rgb_to_grayscale, m)?)?;
    m.add_function(wrap_pyfunction!(rgb_to_rgba, m)?)?;
    m.add_function(wrap_pyfunction!(rgba_to_rgb, m)?)?;
    m.add_function(wrap_pyfunction!(convert_format, m)?)?;

    m.add_function(wrap_pyfunction!(reduce_tokens, m)?)?;
    m.add_function(wrap_pyfunction!(batch_reduce_tokens, m)?)?;
    m.add_function(wrap_pyfunction!(get_reduction_statistics, m)?)?;
    m.add_class::<TokenReductionConfigDTO>()?;
    m.add_class::<ReductionLevelDTO>()?;

    m.add_function(wrap_pyfunction!(generate_cache_key, m)?)?;
    m.add_function(wrap_pyfunction!(batch_generate_cache_keys, m)?)?;
    m.add_function(wrap_pyfunction!(fast_hash, m)?)?;
    m.add_function(wrap_pyfunction!(validate_cache_key, m)?)?;
    m.add_function(wrap_pyfunction!(filter_old_cache_entries, m)?)?;
    m.add_function(wrap_pyfunction!(sort_cache_by_access_time, m)?)?;
    m.add_function(wrap_pyfunction!(get_available_disk_space, m)?)?;
    m.add_function(wrap_pyfunction!(get_cache_metadata, m)?)?;
    m.add_function(wrap_pyfunction!(cleanup_cache, m)?)?;
    m.add_function(wrap_pyfunction!(smart_cleanup_cache, m)?)?;
    m.add_function(wrap_pyfunction!(is_cache_valid, m)?)?;
    m.add_function(wrap_pyfunction!(clear_cache_directory, m)?)?;
    m.add_function(wrap_pyfunction!(batch_cleanup_caches, m)?)?;
    m.add_class::<CacheStats>()?;
    m.add_class::<GenericCache>()?;

    m.add_function(wrap_pyfunction!(table_from_arrow_to_markdown, m)?)?;
    m.add_function(wrap_pyfunction!(convert_html_to_markdown, m)?)?;
    m.add_function(wrap_pyfunction!(process_html, m)?)?;

    m.add_function(wrap_pyfunction!(read_excel_file, m)?)?;
    m.add_function(wrap_pyfunction!(read_excel_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(excel_to_markdown, m)?)?;
    m.add_function(wrap_pyfunction!(benchmark_excel_reading, m)?)?;
    m.add_class::<ExcelWorkbookDTO>()?;
    m.add_class::<ExcelSheetDTO>()?;

    m.add_class::<PptxExtractorDTO>()?;
    m.add_class::<StreamingPptxExtractorDTO>()?;
    m.add_class::<PptxExtractionResultDTO>()?;
    m.add_class::<PptxMetadataDTO>()?;

    m.add_function(wrap_pyfunction!(extract_email_content, m)?)?;
    m.add_function(wrap_pyfunction!(extract_eml_content, m)?)?;
    m.add_function(wrap_pyfunction!(extract_msg_content, m)?)?;
    m.add_function(wrap_pyfunction!(extract_email_from_file, m)?)?;
    m.add_function(wrap_pyfunction!(get_supported_email_formats, m)?)?;
    m.add_function(wrap_pyfunction!(validate_email_content, m)?)?;
    m.add_function(wrap_pyfunction!(build_email_text_output, m)?)?;
    m.add_class::<EmailExtractionResultDTO>()?;
    m.add_class::<EmailAttachmentDTO>()?;

    m.add_function(wrap_pyfunction!(validate_language_code, m)?)?;
    m.add_function(wrap_pyfunction!(validate_tesseract_version, m)?)?;
    m.add_class::<PSMMode>()?;
    m.add_class::<TesseractConfigDTO>()?;
    m.add_class::<ExtractionResultDTO>()?;
    m.add_class::<TableDTO>()?;
    m.add_class::<BatchItemResult>()?;
    m.add_class::<OCRProcessor>()?;
    m.add_class::<OCRCacheStats>()?;

    register_chunking(m)?;

    xml::register_xml_functions(m)?;
    text::register_text_functions(m)?;

    Ok(())
}
