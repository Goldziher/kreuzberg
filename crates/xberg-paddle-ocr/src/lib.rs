//! # xberg-paddle-ocr
//!
//! PaddleOCR via ONNX Runtime for Xberg - high-performance text detection and recognition.
//!
//! This crate is vendored from [paddle-ocr-rs](https://github.com/mg-chao/paddle-ocr-rs)
//! by mg-chao, with modifications for Xberg integration.
//!
//! ## ONNX Runtime Requirement
//!
//! Requires **ONNX Runtime 1.24+** at runtime.
//!
//! ## Original License
//!
//! The original paddle-ocr-rs is licensed under Apache-2.0.
//! This vendored version is relicensed to MIT with the original author's copyright retained.

#![deny(clippy::print_stdout, clippy::print_stderr)]
#![cfg_attr(test, allow(clippy::print_stdout, clippy::print_stderr))]
#![allow(clippy::too_many_arguments)]
#![allow(missing_docs)]

#[cfg(not(any(feature = "ort", feature = "tract")))]
compile_error!(
    "xberg-paddle-ocr requires at least one inference backend feature: enable `ort` (native ONNX \
     Runtime) or `tract` (pure-Rust ONNX, for wasm32 / the Android x86_64 emulator)."
);

pub mod angle_net;
pub mod base_net;
pub(crate) mod constants;
pub mod crnn_net;
pub mod db_net;
pub(crate) mod inference;
pub mod ocr_error;
pub mod ocr_lite;
pub mod ocr_result;
pub mod ocr_utils;
pub mod scale_param;

/// Which concrete ONNX engine a model is loaded onto.
///
/// Re-exported so callers can pin an engine per load instead of taking
/// `inference::default_backend`'s compile-time choice, which always prefers `ort` when the
/// `ort` feature is on and so makes `tract` unreachable in a dual-engine build.
pub use inference::Backend as InferenceBackend;
#[cfg(feature = "ort")]
pub use inference::ort_backend::adopt_environment as adopt_ort_environment;
pub use ocr_error::OcrError;
pub use ocr_lite::PaddleOcrEngine;
pub use ocr_result::{
    Angle, DetailedOcrResult, DetailedTextBlock, DetailedTextLine, OcrResult, Point, RecognizedWord, TextBlock,
    TextBox, TextLine, WordBlock,
};

/// Backward-compatible name for [`PaddleOcrEngine`].
#[deprecated(note = "use PaddleOcrEngine")]
pub type OcrLite = PaddleOcrEngine;
