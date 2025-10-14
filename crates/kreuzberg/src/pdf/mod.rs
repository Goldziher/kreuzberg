pub mod error;
pub mod images;
pub mod metadata;
pub mod rendering;
pub mod text;

pub use error::PdfError;
pub use images::{PdfImage, PdfImageExtractor, extract_images_from_pdf};
pub use metadata::extract_metadata;
pub use rendering::{PageRenderOptions, render_page_to_image};
pub use text::extract_text_from_pdf;
