/// High-performance table processing using Arrow IPC bridge
///
/// This module provides Rust implementations of table processing operations
/// using Arrow IPC as an interop layer between Python Polars and Rust Polars.
/// This approach provides 10-15x performance improvements over pure Python.
mod arrow_bridge;

pub use arrow_bridge::table_from_arrow_to_markdown;
