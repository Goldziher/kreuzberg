//! The [`ProgressSink`] seam: a sink for coarse extraction progress events.
//!
//! The in-core default is [`NoopProgressSink`], which discards every event —
//! exactly the behavior xberg exhibits today, where the extraction path emits
//! no progress. Alternative sinks (a channel, a logger, a metrics bridge)
//! implement this trait and are injected via
//! [`EngineBuilder::with_progress_sink`](super::super::EngineBuilder::with_progress_sink).

/// A coarse progress event emitted during extraction.
///
/// Intentionally minimal: a stage label plus optional detail and completion
/// fraction. Richer event shapes are layered on by the sink implementation.
#[derive(Debug, Clone)]
pub struct ProgressEvent {
    /// Short, stable identifier for the current stage (e.g. `"download"`).
    pub stage: String,
    /// Optional human-readable detail for this event.
    pub message: Option<String>,
    /// Optional completion fraction in `0.0..=1.0`, when known.
    pub fraction: Option<f64>,
}

/// A sink for [`ProgressEvent`]s emitted during extraction.
///
/// # Thread safety
///
/// Implementations are `Send + Sync + 'static` and held behind
/// `Arc<dyn ProgressSink>`; they may be called concurrently.
pub trait ProgressSink: Send + Sync + 'static {
    /// Record a progress event. Implementations must not block.
    fn emit(&self, event: ProgressEvent);

    /// Record completion of one OCR page.
    fn emit_ocr_page(&self, page: usize, total: usize, completed: usize, backend: &str, _input_index: Option<usize>) {
        self.emit(ProgressEvent {
            stage: "ocr_page".to_string(),
            message: Some(format!("page {page} of {total} ({backend})")),
            fraction: (total > 0).then_some(completed as f64 / total as f64),
        });
    }
}

/// In-core default: a sink that discards every event.
///
/// This reproduces today's behavior exactly — the default extraction path emits
/// no progress, so [`emit`](ProgressSink::emit) is inert.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopProgressSink;

impl ProgressSink for NoopProgressSink {
    fn emit(&self, _event: ProgressEvent) {}
}

// ~keep Liveness is configuration-dependent: the only consumers are compiled out on
// narrow feature legs, so `-D dead-code` fires there and nowhere else. A hand-kept
// union-of-consumers `cfg` is what drifted here and failed the 1.3.0 publish (GH#1951).
// The fields are written by `scope_progress` (live under `tokio-runtime` alone) but read
// only by `ProgressContext::emit_ocr_page`, reachable solely from the PDF OCR pipeline.
#[cfg(feature = "tokio-runtime")]
#[derive(Clone)]
#[allow(dead_code)]
struct ProgressContext {
    sink: std::sync::Arc<dyn ProgressSink>,
    input_index: Option<usize>,
    completed_pages: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<usize>>>,
}

#[cfg(feature = "tokio-runtime")]
#[allow(dead_code)]
impl ProgressContext {
    fn emit_ocr_page(&self, page: usize, total: usize, backend: &str) {
        let Ok(mut completed_pages) = self.completed_pages.lock() else {
            tracing::warn!("progress completion counter lock was poisoned; ignoring event");
            return;
        };
        completed_pages.insert(page);
        self.sink
            .emit_ocr_page(page, total, completed_pages.len(), backend, self.input_index);
    }
}

#[cfg(feature = "tokio-runtime")]
tokio::task_local! {
    static CURRENT_PROGRESS: ProgressContext;
}

#[cfg(feature = "tokio-runtime")]
pub(crate) async fn scope_progress<F, T>(
    sink: std::sync::Arc<dyn ProgressSink>,
    input_index: Option<usize>,
    future: F,
) -> T
where
    F: std::future::Future<Output = T>,
{
    CURRENT_PROGRESS
        .scope(
            ProgressContext {
                sink,
                input_index,
                completed_pages: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
            },
            future,
        )
        .await
}

#[cfg(feature = "tokio-runtime")]
#[allow(dead_code)]
pub(crate) fn inherit_progress<F>(future: F) -> impl std::future::Future<Output = F::Output>
where
    F: std::future::Future,
{
    use tracing::Instrument as _;

    let context = CURRENT_PROGRESS.try_with(Clone::clone).ok();
    // ~keep OCR page futures are large; bound stack use when task-local scope takes ownership (#2131).
    let future = Box::pin(future);
    async move {
        match context {
            Some(context) => CURRENT_PROGRESS.scope(context, future).await,
            None => future.await,
        }
    }
    .in_current_span()
}

#[cfg(feature = "tokio-runtime")]
#[allow(dead_code)]
pub(crate) fn emit_ocr_page(page: usize, total: usize, backend: &str) {
    let _ = CURRENT_PROGRESS.try_with(|context| context.emit_ocr_page(page, total, backend));
}

#[cfg(not(feature = "tokio-runtime"))]
#[allow(dead_code)]
pub(crate) fn emit_ocr_page(_page: usize, _total: usize, _backend: &str) {}

#[cfg(all(test, feature = "tokio-runtime"))]
mod tests {
    use std::sync::{Arc, Condvar, Mutex};

    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct RecordedPage {
        page: usize,
        total: usize,
        completed: usize,
        backend: String,
        input_index: Option<usize>,
    }

    #[derive(Default)]
    struct RecordingSink {
        events: Mutex<Vec<ProgressEvent>>,
        pages: Mutex<Vec<RecordedPage>>,
    }

    impl ProgressSink for RecordingSink {
        fn emit(&self, event: ProgressEvent) {
            self.events.lock().expect("progress sink mutex poisoned").push(event);
        }

        fn emit_ocr_page(
            &self,
            page: usize,
            total: usize,
            completed: usize,
            backend: &str,
            input_index: Option<usize>,
        ) {
            self.pages
                .lock()
                .expect("progress sink mutex poisoned")
                .push(RecordedPage {
                    page,
                    total,
                    completed,
                    backend: backend.to_string(),
                    input_index,
                });
        }
    }

    #[tokio::test]
    async fn should_emit_numbered_ocr_page_progress_in_current_extraction_context() {
        let sink = Arc::new(RecordingSink::default());

        scope_progress(sink.clone(), Some(3), async {
            emit_ocr_page(7, 12, "tesseract");
            tokio::spawn(inherit_progress(async {
                emit_ocr_page(7, 12, "paddleocr");
                emit_ocr_page(8, 12, "paddleocr");
            }))
            .await
            .expect("inherited progress task should finish");
        })
        .await;

        let pages = sink.pages.lock().expect("progress sink mutex poisoned");
        assert_eq!(
            *pages,
            vec![
                RecordedPage {
                    page: 7,
                    total: 12,
                    completed: 1,
                    backend: "tesseract".to_string(),
                    input_index: Some(3),
                },
                RecordedPage {
                    page: 7,
                    total: 12,
                    completed: 1,
                    backend: "paddleocr".to_string(),
                    input_index: Some(3),
                },
                RecordedPage {
                    page: 8,
                    total: 12,
                    completed: 2,
                    backend: "paddleocr".to_string(),
                    input_index: Some(3),
                },
            ]
        );
    }

    #[derive(Default)]
    struct BlockingFirstSink {
        completions: Mutex<Vec<usize>>,
        first_entered: (Mutex<bool>, Condvar),
        release_first: (Mutex<bool>, Condvar),
    }

    impl ProgressSink for BlockingFirstSink {
        fn emit(&self, _event: ProgressEvent) {}

        fn emit_ocr_page(
            &self,
            _page: usize,
            _total: usize,
            completed: usize,
            _backend: &str,
            _input_index: Option<usize>,
        ) {
            if completed == 1 {
                let (entered, entered_signal) = &self.first_entered;
                *entered.lock().expect("entered mutex poisoned") = true;
                entered_signal.notify_one();

                let (released, release_signal) = &self.release_first;
                let mut released = released.lock().expect("release mutex poisoned");
                while !*released {
                    released = release_signal.wait(released).expect("release mutex poisoned");
                }
            }
            self.completions
                .lock()
                .expect("completions mutex poisoned")
                .push(completed);
        }
    }

    #[test]
    fn should_serialize_completion_number_and_delivery() {
        let sink = Arc::new(BlockingFirstSink::default());
        let context = ProgressContext {
            sink: sink.clone(),
            input_index: Some(5),
            completed_pages: Arc::new(Mutex::new(std::collections::HashSet::new())),
        };
        let first_context = context.clone();
        let first = std::thread::spawn(move || first_context.emit_ocr_page(1, 2, "tesseract"));

        let (entered, entered_signal) = &sink.first_entered;
        let mut entered = entered.lock().expect("entered mutex poisoned");
        while !*entered {
            entered = entered_signal.wait(entered).expect("entered mutex poisoned");
        }
        drop(entered);

        let second_context = context.clone();
        let (second_started, second_started_rx) = std::sync::mpsc::sync_channel(0);
        let second = std::thread::spawn(move || {
            second_started.send(()).expect("second emitter should signal");
            second_context.emit_ocr_page(2, 2, "tesseract");
        });
        second_started_rx.recv().expect("second emitter should start");
        std::thread::sleep(std::time::Duration::from_millis(50));
        let (released, release_signal) = &sink.release_first;
        *released.lock().expect("release mutex poisoned") = true;
        release_signal.notify_one();

        first.join().expect("first emitter should finish");
        second.join().expect("second emitter should finish");
        assert_eq!(
            *sink.completions.lock().expect("completions mutex poisoned"),
            vec![1, 2]
        );
    }
}
