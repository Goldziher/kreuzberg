//! Benchmark harness for comparing document extraction frameworks
//!
//! This crate provides infrastructure for benchmarking Kreuzberg against other
//! document extraction frameworks, measuring performance (throughput, memory, latency)
//! and quality (F1 scores, text accuracy).

pub mod adapter;
pub mod adapters;
pub mod config;
pub mod error;
pub mod fixture;
pub mod monitoring;
pub mod output;
pub mod registry;
pub mod runner;
pub mod types;

pub use adapter::FrameworkAdapter;
pub use adapters::{NativeAdapter, NodeAdapter, PythonAdapter, RubyAdapter};
pub use config::{BenchmarkConfig, BenchmarkMode};
pub use error::{Error, Result};
pub use fixture::{Fixture, FixtureManager};
pub use monitoring::{ResourceMonitor, ResourceSample, ResourceStats};
pub use output::{write_by_extension_analysis, write_json};
pub use registry::AdapterRegistry;
pub use runner::BenchmarkRunner;
pub use types::{BenchmarkResult, FrameworkCapabilities, PdfMetadata};
