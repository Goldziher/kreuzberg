//! An `auto`-gated native PDF must keep its layout detections for VLM region extraction when a page
//! skips layout inference, in every build with `liter-llm`, not only with `formula-recognition`.
//!
//! Lives under `engine` because the stub model is installed in the private engine pool, which only
//! this module can construct an engine for; the VLM endpoint is a local wiremock stub.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::core::config::acceleration::{AccelerationConfig, ExecutionProviderType};
use crate::core::config::layout::LayoutStrategy;
use crate::core::config::{ExtractionConfig, LayoutDetectionConfig, LlmConfig, OcrConfig, VlmFallbackPolicy};
use crate::layout::{BBox, LayoutClass, LayoutDetection};
use crate::plugins::InternalDocumentExtractor;

/// Keys the stub engine apart from any other pooled engine, so no other test can lease it.
const STUB_CONFIDENCE_THRESHOLD: f32 = 0.37;
const REGION_MARKDOWN: &str = "Stub figure transcription for the gated document.";

/// Reports one picture region on every page it is shown and counts the pages it ran on.
struct PictureRegionModel {
    pages_inferred: Arc<AtomicUsize>,
}

impl LayoutModel for PictureRegionModel {
    fn detect(&mut self, img: &RgbImage) -> Result<Vec<LayoutDetection>, LayoutError> {
        self.pages_inferred.fetch_add(1, Ordering::SeqCst);
        let (width, height) = (img.width() as f32, img.height() as f32);
        Ok(vec![LayoutDetection {
            class_name: LayoutClass::Picture,
            confidence: 0.9,
            bbox: BBox {
                x1: width * 0.1,
                y1: height * 0.1,
                x2: width * 0.9,
                y2: height * 0.5,
            },
        }])
    }

    fn detect_with_threshold(&mut self, img: &RgbImage, _threshold: f32) -> Result<Vec<LayoutDetection>, LayoutError> {
        self.detect(img)
    }

    fn name(&self) -> &str {
        "picture-region-stub"
    }
}

fn layout_config() -> LayoutDetectionConfig {
    LayoutDetectionConfig {
        strategy: LayoutStrategy::Auto,
        confidence_threshold: Some(STUB_CONFIDENCE_THRESHOLD),
        acceleration: Some(AccelerationConfig {
            provider: ExecutionProviderType::Cpu,
            device_id: 0,
        }),
        ..Default::default()
    }
}

/// Pools a stub engine matching `layout`, so the markdown layout pass leases it instead of loading RT-DETR.
fn install_stub_engine(layout: &LayoutDetectionConfig) -> Arc<AtomicUsize> {
    let pages_inferred = Arc::new(AtomicUsize::new(0));
    let engine = LayoutEngine {
        model: Box::new(PictureRegionModel {
            pages_inferred: Arc::clone(&pages_inferred),
        }),
        config: crate::layout::config_from_extraction(layout),
        thread_budget: crate::core::config::concurrency::resolve_thread_budget(None).max(1),
    };
    let lease = crate::layout::CACHED_ENGINE
        .take_or_create_matching(|_| false, || Ok::<_, LayoutError>(engine))
        .expect("the stub engine is pooled");
    drop(lease);
    pages_inferred
}

fn stub_completion(content: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "created": 0,
        "model": "openai/gpt-4o-mini",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": content },
            "finish_reason": "stop"
        }],
        "usage": { "prompt_tokens": 10, "completion_tokens": 10, "total_tokens": 20 }
    })
}

/// Page 1 carries only a raster figure, so the gate runs the model there; page 2 is single
/// column prose the gate skips.
fn figure_then_prose_pdf() -> Vec<u8> {
    let raster = vec![0x60u8; 16 * 16];
    let figure_stream = "q 468 0 0 300 72 420 cm /Im0 Do Q\n".to_string();
    let mut prose_stream = String::from("BT /F1 10 Tf 72 720 Td 14 TL\n");
    for line in 0..45 {
        prose_stream.push_str(&format!(
            "(Line {line} of steady single column prose with enough characters to pass the floor.) Tj T*\n"
        ));
    }
    prose_stream.push_str("ET\n");

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets: Vec<usize> = Vec::new();
    let mut object = |pdf: &mut Vec<u8>, body: &[u8]| {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", offsets.len()).as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    };
    object(&mut pdf, b"<< /Type /Catalog /Pages 2 0 R >>");
    object(&mut pdf, b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>");
    object(
        &mut pdf,
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 5 0 R \
          /Resources << /XObject << /Im0 7 0 R >> >> >>",
    );
    object(
        &mut pdf,
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 6 0 R \
          /Resources << /Font << /F1 8 0 R >> >> >>",
    );
    for stream in [&figure_stream, &prose_stream] {
        object(
            &mut pdf,
            format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len()).as_bytes(),
        );
    }
    let mut image = format!(
        "<< /Type /XObject /Subtype /Image /Width 16 /Height 16 /ColorSpace /DeviceGray \
         /BitsPerComponent 8 /Length {} >>\nstream\n",
        raster.len()
    )
    .into_bytes();
    image.extend_from_slice(&raster);
    image.extend_from_slice(b"\nendstream");
    object(&mut pdf, &image);
    object(
        &mut pdf,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );

    let xref_pos = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f\r\n", offsets.len() + 1).as_bytes());
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n",
            offsets.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

#[tokio::test]
#[serial_test::serial]
async fn auto_gated_native_pdf_keeps_detections_for_vlm_regions() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_completion(REGION_MARKDOWN)))
        .mount(&server)
        .await;
    let layout = layout_config();
    let pages_inferred = install_stub_engine(&layout);
    let config = ExtractionConfig {
        use_layout_for_markdown: true,
        disable_ocr: true,
        layout: Some(layout),
        ocr: Some(OcrConfig {
            vlm_fallback: VlmFallbackPolicy::OnLowQuality { quality_threshold: 0.5 },
            vlm_config: Some(LlmConfig {
                model: "openai/gpt-4o-mini".to_string(),
                api_key: Some("test-key".to_string()),
                base_url: Some(server.uri()),
                timeout_secs: Some(10),
                max_retries: Some(0),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let doc = crate::extractors::PdfExtractor::new()
        .extract_content(&figure_then_prose_pdf(), "application/pdf", &config)
        .await
        .expect("extraction must succeed");

    let Some(crate::types::FormatMetadata::Pdf(pdf_metadata)) = &doc.metadata.format else {
        panic!("a PDF carries PDF metadata");
    };
    assert_eq!(
        pdf_metadata.layout_gated_pages,
        Some(vec![2]),
        "the auto gate must skip the prose page"
    );
    assert_eq!(
        pages_inferred.load(Ordering::SeqCst),
        1,
        "layout inference must run on the figure page only"
    );

    let requests = server.received_requests().await.expect("wiremock records requests");
    assert_eq!(
        requests.len(),
        1,
        "the figure detection must reach one VLM region request"
    );
    let body = String::from_utf8_lossy(&requests[0].body);
    assert!(
        body.contains("data:image/png;base64,"),
        "the region request must carry the cropped figure: {body}"
    );
    assert!(
        doc.elements
            .iter()
            .any(|element| element.page == Some(1) && element.text == REGION_MARKDOWN),
        "the region transcription must be spliced into page 1"
    );
}
