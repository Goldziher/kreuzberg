pub mod structured;
pub mod text;

#[cfg(feature = "ocr")]
pub mod image;

#[cfg(feature = "archives")]
pub mod archive;

#[cfg(feature = "email")]
pub mod email;

#[cfg(feature = "excel")]
pub mod excel;

#[cfg(feature = "html")]
pub mod html;

#[cfg(feature = "office")]
pub mod docx;

#[cfg(feature = "office")]
pub mod libreoffice;

#[cfg(feature = "office")]
pub mod office_metadata;

#[cfg(feature = "pandoc-fallback")]
pub mod pandoc;

#[cfg(feature = "office")]
pub mod pptx;

#[cfg(feature = "excel")]
pub mod table;

#[cfg(feature = "xml")]
pub mod xml;

#[cfg(any(feature = "office", feature = "html"))]
pub mod markdown;

pub use structured::{JsonExtractionConfig, StructuredDataResult, parse_json, parse_toml, parse_yaml};
pub use text::parse_text;

#[cfg(feature = "ocr")]
pub use image::{ImageMetadata, extract_image_metadata};

#[cfg(feature = "archives")]
pub use archive::{
    ArchiveEntry, ArchiveMetadata, extract_7z_metadata, extract_7z_text_content, extract_tar_metadata,
    extract_tar_text_content, extract_zip_metadata, extract_zip_text_content,
};

#[cfg(feature = "email")]
pub use email::{build_email_text_output, extract_email_content, parse_eml_content, parse_msg_content};

#[cfg(feature = "excel")]
pub use excel::{excel_to_markdown, read_excel_bytes, read_excel_file};

#[cfg(feature = "html")]
pub use html::{convert_html_to_markdown, process_html};

#[cfg(feature = "office")]
pub use libreoffice::{check_libreoffice_available, convert_doc_to_docx, convert_ppt_to_pptx};

#[cfg(feature = "office")]
pub use office_metadata::{
    CoreProperties, CustomProperties, DocxAppProperties, PptxAppProperties, XlsxAppProperties, extract_core_properties,
    extract_custom_properties, extract_docx_app_properties, extract_pptx_app_properties, extract_xlsx_app_properties,
};

#[cfg(feature = "office")]
pub use pptx::{extract_pptx_from_bytes, extract_pptx_from_path};

#[cfg(feature = "excel")]
pub use table::table_from_arrow_to_markdown;

#[cfg(feature = "xml")]
pub use xml::parse_xml;

#[cfg(any(feature = "office", feature = "html"))]
pub use markdown::cells_to_markdown;
