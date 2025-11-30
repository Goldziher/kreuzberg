#![cfg(feature = "otel")]

use std::sync::{Arc, Mutex};
use tracing::Subscriber;
use tracing::span::{Attributes, Id};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::Layer;
use tracing_subscriber::registry::LookupSpan;

/// Simple span name collector for testing.
///
/// This layer collects span names as they are created to verify
/// that instrumentation is working correctly.
struct SpanCollector {
    spans: Arc<Mutex<Vec<String>>>,
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for SpanCollector {
    fn on_new_span(&self, attrs: &Attributes<'_>, _id: &Id, _ctx: Context<'_, S>) {
        self.spans.lock().unwrap().push(attrs.metadata().name().to_string());
    }
}

#[tokio::test]
async fn test_cache_instrumentation() {
    use kreuzberg::cache::GenericCache;
    use tempfile::tempdir;

    let spans = Arc::new(Mutex::new(Vec::new()));
    let collector = SpanCollector { spans: spans.clone() };

    let subscriber = tracing_subscriber::registry().with(collector);
    let _guard = tracing::subscriber::set_default(subscriber);

    let temp_dir = tempdir().unwrap();
    let cache = GenericCache::new(
        "test".to_string(),
        Some(temp_dir.path().to_str().unwrap().to_string()),
        30.0,
        500.0,
        1000.0,
    )
    .unwrap();

    // Test set operation
    cache.set("test_key", b"test data".to_vec(), None).unwrap();

    // Test get operation
    let _ = cache.get("test_key", None).unwrap();

    let span_names = spans.lock().unwrap();
    assert!(span_names.contains(&"set".to_string()), "Expected 'set' span");
    assert!(span_names.contains(&"get".to_string()), "Expected 'get' span");
}

#[cfg(feature = "ocr")]
#[tokio::test]
async fn test_ocr_instrumentation() {
    use kreuzberg::ocr::processor::OcrProcessor;
    use kreuzberg::ocr::types::TesseractConfig;
    use tempfile::tempdir;

    let spans = Arc::new(Mutex::new(Vec::new()));
    let collector = SpanCollector { spans: spans.clone() };

    let subscriber = tracing_subscriber::registry().with(collector);
    let _guard = tracing::subscriber::set_default(subscriber);

    let temp_dir = tempdir().unwrap();
    let processor = OcrProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

    // Create a simple test image (1x1 white pixel PNG)
    let mut test_image = Vec::new();
    let img = image::ImageBuffer::from_fn(1, 1, |_, _| image::Rgb([255u8, 255u8, 255u8]));
    img.write_to(&mut std::io::Cursor::new(&mut test_image), image::ImageFormat::Png).unwrap();

    let config = TesseractConfig {
        output_format: "text".to_string(),
        use_cache: false,
        ..TesseractConfig::default()
    };

    // This may fail if Tesseract is not installed, but we're testing for span creation
    let _ = processor.process_image(&test_image, &config);

    let span_names = spans.lock().unwrap();
    assert!(span_names.contains(&"process_image".to_string()), "Expected 'process_image' span");
}

#[tokio::test]
async fn test_registry_instrumentation() {
    use kreuzberg::plugins::registry::DocumentExtractorRegistry;

    let spans = Arc::new(Mutex::new(Vec::new()));
    let collector = SpanCollector { spans: spans.clone() };

    let subscriber = tracing_subscriber::registry().with(collector);
    let _guard = tracing::subscriber::set_default(subscriber);

    let registry = DocumentExtractorRegistry::new();

    // Try to get an extractor (will fail but should create a span)
    let _ = registry.get("application/pdf");

    let span_names = spans.lock().unwrap();
    assert!(span_names.contains(&"get".to_string()), "Expected 'get' span from registry");
}

#[cfg(all(feature = "pdf", feature = "office"))]
#[tokio::test]
async fn test_span_hierarchy() {
    use kreuzberg::core::config::ExtractionConfig;
    use kreuzberg::core::extractor::extract_bytes;
    use std::fs;

    let spans = Arc::new(Mutex::new(Vec::new()));
    let collector = SpanCollector { spans: spans.clone() };

    let subscriber = tracing_subscriber::registry().with(collector);
    let _guard = tracing::subscriber::set_default(subscriber);

    // Create a simple text file to extract
    let test_content = b"Hello, World!";
    let config = ExtractionConfig::default();

    let _ = extract_bytes(test_content, "text/plain", &config).await;

    let span_names = spans.lock().unwrap();
    // Should have extract_bytes span
    assert!(span_names.contains(&"extract_bytes".to_string()), "Expected 'extract_bytes' span");
}

#[test]
fn test_span_collector_creation() {
    let spans = Arc::new(Mutex::new(Vec::new()));
    let _collector = SpanCollector { spans };
    // Just verify we can create the collector
}
