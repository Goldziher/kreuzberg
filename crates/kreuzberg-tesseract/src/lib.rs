#![cfg_attr(not(feature = "build-tesseract"), allow(unused_variables, dead_code))]
#![allow(clippy::arc_with_non_send_sync)]
#![allow(clippy::missing_transmute_annotations)]
#![allow(clippy::type_complexity)]
#![allow(clippy::new_without_default)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![allow(clippy::cmp_null)]

//! # tesseract-rs
//!
//! `tesseract-rs` provides safe Rust bindings for Tesseract OCR with built-in compilation
//! of Tesseract and Leptonica libraries. This crate aims to make OCR functionality
//! easily accessible in Rust projects while handling the complexity of interfacing
//! with the underlying C++ libraries.
//!
//! ## Usage
//!
//! Here's a basic example of how to use `tesseract-rs`:
//!
//! ```rust
//! use std::path::PathBuf;
//! use std::error::Error;
//! use kreuzberg_tesseract::TesseractAPI;
//!
//! fn get_default_tessdata_dir() -> PathBuf {
//!     if cfg!(target_os = "macos") {
//!         let home_dir = std::env::var("HOME").expect("HOME environment variable not set");
//!         PathBuf::from(home_dir)
//!             .join("Library")
//!             .join("Application Support")
//!             .join("tesseract-rs")
//!             .join("tessdata")
//!     } else if cfg!(target_os = "linux") {
//!         let home_dir = std::env::var("HOME").expect("HOME environment variable not set");
//!         PathBuf::from(home_dir)
//!             .join(".tesseract-rs")
//!             .join("tessdata")
//!     } else if cfg!(target_os = "windows") {
//!         PathBuf::from(std::env::var("APPDATA").expect("APPDATA environment variable not set"))
//!             .join("tesseract-rs")
//!             .join("tessdata")
//!     } else {
//!         panic!("Unsupported operating system");
//!     }
//! }
//!
//! fn get_tessdata_dir() -> PathBuf {
//!     match std::env::var("TESSDATA_PREFIX") {
//!         Ok(dir) => {
//!             let path = PathBuf::from(dir);
//!             println!("Using TESSDATA_PREFIX directory: {:?}", path);
//!             path
//!         }
//!         Err(_) => {
//!             let default_dir = get_default_tessdata_dir();
//!             println!(
//!                 "TESSDATA_PREFIX not set, using default directory: {:?}",
//!                 default_dir
//!             );
//!             default_dir
//!         }
//!     }
//! }
//!
//! fn main() -> Result<(), Box<dyn Error>> {
//!     let api = TesseractAPI::new();
//!
//!     // Get tessdata directory (uses default location or TESSDATA_PREFIX if set)
//!     let tessdata_dir = get_tessdata_dir();
//!     api.init(tessdata_dir.to_str().unwrap(), "eng")?;
//!
//!     let width = 24;
//!     let height = 24;
//!     let bytes_per_pixel = 1;
//!     let bytes_per_line = width * bytes_per_pixel;
//!
//!     // Initialize image data with all white pixels
//!     let mut image_data = vec![255u8; width * height];
//!
//!     // Draw number 9 with clearer distinction
//!     for y in 4..19 {
//!         for x in 7..17 {
//!             // Top bar
//!             if y == 4 && x >= 8 && x <= 15 {
//!                 image_data[y * width + x] = 0;
//!             }
//!             // Top curve left side
//!             if y >= 4 && y <= 10 && x == 7 {
//!                 image_data[y * width + x] = 0;
//!             }
//!             // Top curve right side
//!             if y >= 4 && y <= 11 && x == 16 {
//!                 image_data[y * width + x] = 0;
//!             }
//!             // Middle bar
//!             if y == 11 && x >= 8 && x <= 15 {
//!                 image_data[y * width + x] = 0;
//!             }
//!             // Bottom right vertical line
//!             if y >= 11 && y <= 18 && x == 16 {
//!                 image_data[y * width + x] = 0;
//!             }
//!             // Bottom bar
//!             if y == 18 && x >= 8 && x <= 15 {
//!                 image_data[y * width + x] = 0;
//!             }
//!         }
//!     }
//!
//!     // Set the image data
//!     api.set_image(&image_data, width.try_into().unwrap(), height.try_into().unwrap(), bytes_per_pixel.try_into().unwrap(), bytes_per_line.try_into().unwrap())?;
//!
//!     // Set whitelist for digits only
//!     api.set_variable("tessedit_char_whitelist", "0123456789")?;
//!
//!     // Set PSM mode to single character
//!     api.set_variable("tessedit_pageseg_mode", "10")?;
//!
//!     // Get the recognized text
//!     let text = api.get_utf8_text()?;
//!     println!("Recognized text: {}", text.trim());
//!
//!     Ok(())
//! }
//! ```
pub use error::{Result, TesseractError};
mod error;
mod page_iterator;
pub use page_iterator::PageIterator;
mod result_iterator;
pub use result_iterator::ResultIterator;
mod choice_iterator;
pub use choice_iterator::ChoiceIterator;
mod monitor;
pub use monitor::TessMonitor;
mod result_renderer;
pub use result_renderer::TessResultRenderer;
mod mutable_iterator;
pub use mutable_iterator::MutableIterator;
mod enums;
pub use enums::{
    TessOrientation, TessPageIteratorLevel, TessPageSegMode, TessPolyBlockType, TessTextlineOrder, TessWritingDirection,
};
mod api;
pub use api::TesseractAPI;
