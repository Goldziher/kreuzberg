use super::pipeline::run_ocr_pipeline;
use crate::core::config::{ExtractionConfig, OcrConfig, OcrPipelineConfig, OutputFormat};
use crate::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use crate::types::{ExtractedDocument, Table};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const BACKEND: &str = "legacy-table-raw-transcript-preservation";
const PROSE: &str = "The inventory record contains the complete approved item counts for this shipment.";
const GFM: &str = "| Code | Count | Status |\n| --- | --- | --- |\n| RAW2144 | 17 | READY2144 |";

fn backend_table() -> Table {
    Table {
        cells: [["Code", "Count", "Status"], ["RAW2144", "17", "READY2144"]]
            .into_iter()
            .map(|row| row.into_iter().map(str::to_owned).collect())
            .collect(),
        markdown: GFM.to_owned(),
        page_number: 1,
        table_id: Some("raw-source-table".to_owned()),
        columns: Some(vec![
            "source-code".into(),
            "source-count".into(),
            "source-status".into(),
        ]),
        ..Default::default()
    }
}

struct LegacyBackend(Arc<AtomicUsize>);

impl Plugin for LegacyBackend {
    fn name(&self) -> &str {
        BACKEND
    }
}

#[async_trait::async_trait]
impl OcrBackend for LegacyBackend {
    async fn process_image(&self, _: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        assert_eq!(config.backend, BACKEND);
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(ExtractedDocument {
            content: format!("{PROSE}\n\n{GFM}"),
            tables: vec![backend_table()],
            ..Default::default()
        })
    }

    fn supports_language(&self, _: &str) -> bool {
        true
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }

    fn supports_table_detection(&self) -> bool {
        true
    }

    fn emits_structured_markdown(&self) -> bool {
        true
    }
}

struct BackendGuard(Option<&'static str>);

impl BackendGuard {
    fn unregister(mut self) {
        unregister_ocr_backend(self.0.take().expect("owned backend remains registered")).expect("remove owned backend");
    }
}

impl Drop for BackendGuard {
    fn drop(&mut self) {
        if let Some(name) = self.0.take()
            && let Err(error) = unregister_ocr_backend(name)
        {
            tracing::error!(backend = name, %error, "Test backend cleanup failed while unwinding");
        }
    }
}

fn pipeline_config() -> (ExtractionConfig, OcrPipelineConfig) {
    let pipeline: OcrPipelineConfig = serde_json::from_value(serde_json::json!({
        "stages": [{"backend": BACKEND, "priority": 100}],
        "quality_thresholds": {"pipeline_min_quality": 0.0}
    }))
    .expect("valid pipeline");
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Markdown,
        ocr: Some(OcrConfig {
            backend: BACKEND.into(),
            pipeline: Some(pipeline.clone()),
            ..Default::default()
        }),
        ..Default::default()
    };
    (config, pipeline)
}

#[tokio::test]
async fn should_preserve_raw_markdown_transcripts_while_rendering_the_legacy_table_once() {
    let calls = Arc::new(AtomicUsize::new(0));
    register_ocr_backend(Arc::new(LegacyBackend(Arc::clone(&calls)))).expect("register unique backend");
    let guard = BackendGuard(Some(BACKEND));
    let (config, pipeline) = pipeline_config();
    let images = [image::DynamicImage::new_rgb8(300, 120)];
    let output = run_ocr_pipeline(
        None,
        Some(&images),
        #[cfg(feature = "layout-detection")]
        None,
        &config,
        &pipeline,
        None,
        None,
    )
    .await;
    guard.unregister();
    let (text, tables, _, document, _, page_texts, _, _, _, _) = output.expect("legacy stage accepted");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(text, format!("{PROSE}\n\n{GFM}"));
    assert_eq!(page_texts, vec![text]);
    assert_eq!(tables.len(), 1);
    let expected = backend_table();
    assert_eq!(tables[0].cells, expected.cells);
    assert_eq!(tables[0].markdown, expected.markdown);
    assert_eq!(tables[0].page_number, expected.page_number);
    assert_eq!(tables[0].table_id, expected.table_id);
    assert_eq!(tables[0].columns, expected.columns);
    let markdown = crate::rendering::render_markdown(&document.expect("fallback IR retains the table"));
    assert_eq!(markdown.matches("RAW2144").count(), 1, "{markdown}");
    assert_eq!(markdown.matches(PROSE).count(), 1, "{markdown}");
}
