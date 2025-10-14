pub mod cache;
pub mod error;
pub mod hocr;
pub mod table;
pub mod utils;
pub mod validation;

pub use cache::{OcrCache, OcrCacheStats};
pub use error::OcrError;
pub use hocr::convert_hocr_to_markdown;
pub use table::{HocrWord, extract_words_from_tsv, reconstruct_table, table_to_markdown};
pub use utils::compute_hash;
pub use validation::{validate_language_code, validate_tesseract_version};
