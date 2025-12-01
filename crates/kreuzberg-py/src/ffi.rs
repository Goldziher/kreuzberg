//! FFI bindings for error context retrieval.
//!
//! This module provides stubs for error code and panic context functions.
//! The actual FFI error context is only available through the C FFI library,
//! which is accessed by non-Rust code (Go, Java, C#, etc).
//!
//! For Python bindings, the ErrorCode enum and PanicContext dataclass
//! are provided in the Python exceptions module for programmatic use.

/// Get the last error code from the FFI layer.
///
/// Returns 0 (Success) - actual error codes are only available through
/// the C FFI library and are thread-local in the FFI layer.
///
/// This function exists for API completeness and future extension
/// when FFI layer integration is available.
pub fn get_last_error_code() -> i32 {
    // TODO: Link to kreuzberg-ffi when available in py bindings
    // For now, return Success code (0)
    0
}

/// Stub panic context function.
///
/// Returns None since panic context is not available in the Rust bindings.
/// Panic context is only available through the C FFI layer.
///
/// This function exists for API completeness and future extension.
pub fn get_last_panic_context() -> Option<String> {
    // TODO: Link to kreuzberg-ffi when available in py bindings
    None
}
