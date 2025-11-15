//! Internal batch mode tracking using tokio task-local storage.
//!
//! This module provides a way to track whether we're in batch processing mode
//! without exposing it in the public API. Extractors check this flag to decide
//! whether to use `spawn_blocking` for CPU-intensive work.

use std::cell::Cell;
use tokio::task_local;

task_local! {
    /// Task-local flag indicating batch processing mode.
    ///
    /// When true, extractors use `spawn_blocking` for CPU-intensive work to enable
    /// parallelism. When false (single-file mode), extractors run directly to avoid
    /// spawn overhead.
    static BATCH_MODE: Cell<bool>;
}

/// Check if we're currently in batch processing mode.
///
/// Returns `false` if the task-local is not set (single-file mode).
#[allow(dead_code)]
pub fn is_batch_mode() -> bool {
    BATCH_MODE.try_with(|cell| cell.get()).unwrap_or(false)
}

/// Run a future with batch mode enabled.
///
/// This sets the task-local BATCH_MODE flag for the duration of the future.
pub async fn with_batch_mode<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    BATCH_MODE.scope(Cell::new(true), future).await
}
