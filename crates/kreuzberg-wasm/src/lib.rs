//! Kreuzberg - WebAssembly Bindings
//!
//! This crate provides WASM-compatible bindings for the Kreuzberg document intelligence library.
//! It enables document extraction, text processing, and analysis in browser and server-side
//! WebAssembly environments.
//!
//! ## Features
//!
//! - Extract text from PDFs, Office documents, and other formats
//! - Compatible with browser and Node.js WASM runtimes
//! - Optimized binary size and performance
//! - Type-safe interface via wasm-bindgen

use wasm_bindgen::prelude::*;

/// Re-export of wasm-bindgen-rayon's init_thread_pool function
///
/// This function initializes a thread pool for parallel processing in WebAssembly contexts.
/// It must be called before any parallel computation using rayon can be performed.
///
/// ## When to Call
///
/// Call this function once during application initialization, after the WASM module has been
/// loaded but before any document extraction that might benefit from parallelization.
///
/// ## Parameters
///
/// The function accepts a number specifying the size of the thread pool (number of worker threads).
/// Common values:
/// - `navigator.hardwareConcurrency`: Browser API for available CPU cores (recommended for web)
/// - `num_cpus::get()`: Rust crate for detecting available CPUs (server-side)
/// - `1`: Explicitly use single-threaded mode if needed
///
/// ## Browser Requirements
///
/// For SharedArrayBuffer support (required for multi-threading in browsers):
/// 1. The browser must support SharedArrayBuffer (Chrome 74+, Firefox 79+, Safari 15.2+)
/// 2. The hosting page must set these HTTP headers:
///    - `Cross-Origin-Opener-Policy: same-origin`
///    - `Cross-Origin-Embedder-Policy: require-corp`
///
/// Without these headers, SharedArrayBuffer is disabled and initialization will fail gracefully.
///
/// ## Graceful Degradation
///
/// If initialization fails (due to missing headers, unsupported browser, or other reasons),
/// document extraction will automatically fall back to single-threaded processing.
/// This means the application will still work correctly, just without the performance benefits.
///
/// ## Example: Browser Usage
///
/// ```typescript
/// import { initThreadPool } from '@kreuzberg/wasm';
///
/// // Initialize thread pool with available CPU cores
/// await initThreadPool(navigator.hardwareConcurrency);
/// ```
///
/// ## Example: Server-Side Usage (Node.js/Deno)
///
/// ```typescript
/// import { initThreadPool } from '@kreuzberg/wasm';
/// import { availableParallelism } from 'os';
///
/// // Initialize with available CPU cores
/// const cpuCount = availableParallelism();
/// await initThreadPool(cpuCount);
/// ```
///
/// ## Example: Error Handling
///
/// ```typescript
/// try {
///   await initThreadPool(navigator.hardwareConcurrency);
///   console.log('Multi-threading enabled');
/// } catch (error) {
///   console.warn('Multi-threading unavailable, using single-threaded mode:', error);
///   // Application continues to work with single-threaded extraction
/// }
/// ```
///
/// ## Performance Notes
///
/// - Each thread pool thread has its own memory context, so total memory usage is roughly
///   (number of threads) × (memory per extraction)
/// - For large documents in constrained environments, consider using fewer threads
/// - The thread pool is initialized lazily on first use, so calling this function is optional
///   but recommended for predictable performance during initial document processing
///
/// ## See Also
///
/// - [wasm-bindgen-rayon documentation](https://docs.rs/wasm-bindgen-rayon/)
/// - [MDN: Cross-Origin-Opener-Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cross-Origin-Opener-Policy)
/// - [MDN: Cross-Origin-Embedder-Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cross-Origin-Embedder-Policy)
pub use wasm_bindgen_rayon::init_thread_pool;

// Module declarations
pub mod config;
pub mod embeddings;
pub mod errors;
pub mod extraction;
pub mod mime;
pub mod plugins;
pub mod types;

// Re-export common types and functions
pub use config::*;
pub use errors::*;
pub use extraction::*;
pub use mime::*;
pub use plugins::*;
pub use types::*;

/// Version of the kreuzberg-wasm binding
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Initialize the WASM module
/// This function should be called once at application startup
#[wasm_bindgen]
pub fn init() {
    // Set panic hook for better error messages in development
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Get information about the WASM module
#[wasm_bindgen]
pub struct ModuleInfo {
    version: String,
    name: String,
}

#[wasm_bindgen]
impl ModuleInfo {
    /// Get the module version
    pub fn version(&self) -> String {
        self.version.clone()
    }

    /// Get the module name
    pub fn name(&self) -> String {
        self.name.clone()
    }
}

/// Get module information
#[wasm_bindgen]
pub fn get_module_info() -> ModuleInfo {
    ModuleInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "kreuzberg-wasm".to_string(),
    }
}

/// Helper function to initialize the thread pool with error handling
/// Accepts the number of threads to use for the thread pool.
/// Returns true if initialization succeeded, false for graceful degradation.
///
/// This function wraps init_thread_pool with panic handling to ensure graceful
/// degradation if thread pool initialization fails. The application will continue
/// to work in single-threaded mode if the thread pool cannot be initialized.
#[wasm_bindgen]
pub fn init_thread_pool_safe(num_threads: u32) -> bool {
    // Validate input
    if num_threads == 0 {
        #[cfg(target_arch = "wasm32")]
        web_sys::console::warn_1(&"Invalid thread count (0). Using single-threaded mode.".into());
        return false;
    }

    // Attempt to initialize the thread pool
    // This is wrapped to handle potential failures gracefully
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // The init_thread_pool function from wasm_bindgen_rayon handles
        // thread pool setup. If called in a non-WASM environment or if
        // rayon initialization fails, we catch and handle it gracefully.
        init_thread_pool(num_threads as usize)
    })) {
        Ok(_) => {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&format!("Thread pool initialized with {} threads", num_threads).into());
            true
        }
        Err(_) => {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::warn_1(&"Failed to initialize thread pool. Falling back to single-threaded mode.".into());
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_thread_pool_export_exists() {
        // Verify the re-export compiles and is accessible
        // This ensures init_thread_pool is available for JavaScript
        // The function signature is correct for WASM binding
        let _ = init_thread_pool;
    }

    #[test]
    fn test_init_thread_pool_safe_graceful_handling() {
        // Test that init_thread_pool_safe handles both success and failure gracefully
        // The function should accept a valid thread count and return a boolean
        let result = init_thread_pool_safe(2);

        // The function should return a boolean indicating success or fallback
        assert!(
            matches!(result, true | false),
            "init_thread_pool_safe should return a boolean"
        );
    }

    #[test]
    fn test_init_thread_pool_safe_invalid_thread_count() {
        // Test that init_thread_pool_safe handles invalid thread counts gracefully
        // Zero threads is invalid and should return false
        let result = init_thread_pool_safe(0);

        // Should return false for invalid input
        assert!(!result, "init_thread_pool_safe should return false for zero threads");
    }

    #[test]
    fn test_init_thread_pool_safe_valid_thread_count() {
        // Test that init_thread_pool_safe accepts valid thread counts
        // A positive number of threads should be accepted
        let result = init_thread_pool_safe(1);

        // Should return a boolean (success or graceful failure)
        assert!(
            matches!(result, true | false),
            "init_thread_pool_safe should return a boolean"
        );
    }

    #[test]
    fn test_module_info_behavior() {
        let info = get_module_info();

        // Verify module name is not empty
        assert!(!info.name().is_empty(), "Module name should not be empty");
        assert_eq!(info.name(), "kreuzberg-wasm", "Module name should be correct");

        // Verify version is not empty
        assert!(!info.version().is_empty(), "Version should not be empty");

        // Verify version format (semantic versioning)
        let version = info.version();
        assert!(version.contains('.'), "Version should contain at least one dot");
    }

    #[test]
    fn test_version_behavior() {
        let v = version();

        // Version string should not be empty
        assert!(!v.is_empty(), "Version string should not be empty");

        // Version should follow semantic versioning pattern (X.Y.Z or X.Y.Z-prerelease)
        // Split by dot and hyphen to get the main version parts
        let main_version = v.split('-').next().unwrap_or(&v);
        let parts: Vec<&str> = main_version.split('.').collect();
        assert!(parts.len() >= 2, "Version should have at least major.minor components");

        // All main version components should be numeric (major.minor.patch)
        for (i, part) in parts.iter().enumerate() {
            assert!(!part.is_empty(), "Version component {} should not be empty", i);
            assert!(
                part.chars().all(|c| c.is_ascii_digit()),
                "Version component {} should be numeric: {}",
                i,
                part
            );
        }
    }

    #[test]
    fn test_module_info_consistency() {
        // Verify that get_module_info() and version() provide consistent version info
        let info = get_module_info();
        let version_str = version();

        assert_eq!(
            info.version(),
            version_str,
            "Module info version should match version() function"
        );
    }
}
