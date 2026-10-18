//! Span-mode GLiNER ONNX inference.
//!
//! This crate vendors the span-mode preprocessing and decoding path from the
//! `gline-rs` project and replaces its pipeline wrapper with direct `ort`
//! session management.

#![deny(clippy::print_stdout, clippy::print_stderr)]
#![cfg_attr(test, allow(clippy::print_stdout, clippy::print_stderr))]

#[cfg(feature = "candle")]
pub mod candle;
mod config;
pub mod decode;
#[cfg(feature = "ort-backend")]
mod engine;
mod error;
mod input;
#[cfg(feature = "ort-backend")]
mod preprocess;
#[cfg(feature = "ort-backend")]
mod session;
// Only used by the ORT engines (engine.rs, preprocess.rs); dead weight
// without ort-backend. ~keep
#[cfg(feature = "ort-backend")]
mod splitter;
#[cfg(feature = "ort-backend")]
mod tensor;
#[cfg(feature = "ort-backend")]
mod tokenizer;
// The V2 (GLiNER2) pipeline is only reachable through the ONNX engine or
// the candle backend; without either there is no consumer for it. ~keep
#[cfg(any(feature = "ort-backend", feature = "candle"))]
mod v2;

pub use config::{Parameters, RuntimeConfig};
pub use decode::{Span, SpanOutput};
#[cfg(feature = "ort-backend")]
pub use engine::Gliner;
pub use error::{GlinerError, Result};
#[cfg(feature = "ort-backend")]
pub use input::TextInput;
pub use input::Token;
#[cfg(feature = "ort-backend")]
pub use session::{INPUT_NAMES, OUTPUT_NAMES, adopt_environment as adopt_ort_environment};
#[cfg(feature = "ort-backend")]
pub use v2::engine::Gliner2;
#[cfg(feature = "ort-backend")]
pub use v2::session::{INPUT_NAMES_V2, OUTPUT_NAMES_V2};

// The V2 prompt-encoding surface is shared with the `candle` module but is
// not part of the crate's public API. ~keep
#[cfg(feature = "candle")]
pub(crate) use v2::preprocess::{V2Encoded, encode_v2};
#[cfg(feature = "candle")]
pub(crate) use v2::splitter::V2Splitter;
#[cfg(feature = "candle")]
pub(crate) use v2::tokenizer::V2Tokenizer;

#[cfg(feature = "ort-backend")]
pub(crate) use decode::EntityContext;
#[cfg(feature = "ort-backend")]
pub(crate) use preprocess::EncodedInput;

#[cfg(all(test, feature = "ort-backend"))]
mod tests;
