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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_batch_mode_not_set_by_default() {
        let result = is_batch_mode();
        assert!(!result, "batch mode should be false by default");
    }

    #[tokio::test]
    async fn test_with_batch_mode_sets_flag() {
        let result = with_batch_mode(async { is_batch_mode() }).await;

        assert!(result, "batch mode should be true inside with_batch_mode");
    }

    #[tokio::test]
    async fn test_batch_mode_scoped_to_future() {
        assert!(!is_batch_mode(), "batch mode should be false before");

        with_batch_mode(async {
            assert!(is_batch_mode(), "batch mode should be true inside");
        })
        .await;

        assert!(!is_batch_mode(), "batch mode should be false after future completes");
    }

    #[tokio::test]
    async fn test_nested_batch_mode_calls() {
        let result = with_batch_mode(async {
            let outer = is_batch_mode();
            let inner = with_batch_mode(async { is_batch_mode() }).await;
            (outer, inner)
        })
        .await;

        assert!(result.0, "outer batch mode should be true");
        assert!(result.1, "inner batch mode should be true");
    }

    #[tokio::test]
    async fn test_batch_mode_unaffected_after_with_batch_mode() {
        with_batch_mode(async {
            assert!(is_batch_mode(), "first call should set batch mode");
        })
        .await;

        assert!(!is_batch_mode(), "batch mode should be false between calls");

        with_batch_mode(async {
            assert!(is_batch_mode(), "second call should set batch mode");
        })
        .await;

        assert!(!is_batch_mode(), "batch mode should be false after all calls");
    }
}
