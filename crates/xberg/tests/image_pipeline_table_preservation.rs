#![cfg(all(feature = "pdf", feature = "ocr", feature = "ocr-pipeline"))]

mod helpers;

use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use xberg::ExtractedDocument;
use xberg::core::config::{ExtractionConfig, OcrConfig, OutputFormat, PageConfig};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use xberg::types::Table;

const PROSE: &str = "The inventory record contains the complete approved item counts for this shipment.";
const GFM: &str = "| Code | Count | Status |\n| --- | --- | --- |\n| ITEM2144 | 17 | READY2144 |";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Payload {
    ProseAndTable,
    MarkdownAndTable,
    TableOnly,
    ProseOnly,
    MarkdownTableOnly,
    MarkdownWithTrailingProse,
    MarkdownWithUnmatchedTable,
    MarkdownWithEmptyTableMarkdown,
}

impl Payload {
    fn text(self) -> String {
        match self {
            Self::MarkdownAndTable | Self::MarkdownWithEmptyTableMarkdown => format!("{PROSE}\n\n{GFM}"),
            Self::MarkdownTableOnly => GFM.to_owned(),
            Self::MarkdownWithTrailingProse => {
                format!("{PROSE}\n\n{GFM}\n\nThe unchanged final inventory note remains here.")
            }
            Self::MarkdownWithUnmatchedTable => {
                format!("{PROSE}\n\n{GFM}\n\n| Other | Value |\n| --- | --- |\n| KEEPOTHER2144 | 29 |")
            }
            Self::TableOnly => String::new(),
            Self::ProseAndTable | Self::ProseOnly => PROSE.to_owned(),
        }
    }

    fn tables(self) -> Vec<Table> {
        if self == Self::ProseOnly {
            return Vec::new();
        }
        vec![Table {
            cells: expected_cells(),
            markdown: if self == Self::MarkdownWithEmptyTableMarkdown {
                String::new()
            } else {
                GFM.to_owned()
            },
            page_number: 1,
            ..Default::default()
        }]
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Invocation {
    backend: String,
    text: String,
    table_cells: Vec<Vec<Vec<String>>>,
}

struct FixedBackend {
    name: &'static str,
    payload: Payload,
    invocations: Arc<Mutex<Vec<Invocation>>>,
}

impl Plugin for FixedBackend {
    fn name(&self) -> &str {
        self.name
    }
}

#[async_trait]
impl OcrBackend for FixedBackend {
    async fn process_image(&self, _: &[u8], config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        let mut result = ExtractedDocument::default();
        result.content = self.payload.text();
        result.tables = self.payload.tables();
        self.invocations
            .lock()
            .expect("invocation ledger locks")
            .push(Invocation {
                backend: config.backend.clone(),
                text: result.content.clone(),
                table_cells: result.tables.iter().map(|table| table.cells.clone()).collect(),
            });
        Ok(result)
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
        matches!(
            self.payload,
            Payload::MarkdownAndTable
                | Payload::MarkdownTableOnly
                | Payload::MarkdownWithTrailingProse
                | Payload::MarkdownWithUnmatchedTable
                | Payload::MarkdownWithEmptyTableMarkdown
        )
    }
}

struct BackendGuard(Option<&'static str>);

impl BackendGuard {
    fn unregister(mut self) -> xberg::Result<()> {
        unregister_ocr_backend(self.0.take().expect("owned backend remains registered"))
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

fn expected_cells() -> Vec<Vec<String>> {
    [["Code", "Count", "Status"], ["ITEM2144", "17", "READY2144"]]
        .into_iter()
        .map(|row| row.into_iter().map(str::to_owned).collect())
        .collect()
}

fn png() -> Vec<u8> {
    let picture = image::RgbImage::from_pixel(300, 120, image::Rgb([160, 160, 160]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    picture
        .write_to(&mut bytes, image::ImageFormat::Png)
        .expect("encode synthetic PNG");
    bytes.into_inner()
}

fn config(name: &str) -> ExtractionConfig {
    // ~keep Serde avoids constructing the stage's deprecated Paddle field; an explicit stage
    // routes the image through the shared pipeline instead of the direct-backend fast path.
    let pipeline = serde_json::from_value(serde_json::json!({
        "stages": [{"backend": name, "priority": 100}],
        "quality_thresholds": {"pipeline_min_quality": 0.0}
    }))
    .expect("valid explicit OCR pipeline");
    ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: name.to_owned(),
            pipeline: Some(pipeline),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        output_format: OutputFormat::Markdown,
        use_cache: false,
        ..Default::default()
    }
}

async fn run(name: &'static str, payload: Payload) -> ExtractedDocument {
    let invocations = Arc::new(Mutex::new(Vec::new()));
    register_ocr_backend(Arc::new(FixedBackend {
        name,
        payload,
        invocations: Arc::clone(&invocations),
    }))
    .expect("register unique owned backend");
    let guard = BackendGuard(Some(name));
    let result = helpers::extract_bytes_document(&png(), "image/png", &config(name)).await;
    guard.unregister().expect("remove owned backend");
    let expected = Invocation {
        backend: name.to_owned(),
        text: payload.text(),
        table_cells: payload.tables().into_iter().map(|table| table.cells).collect(),
    };
    assert_eq!(
        invocations.lock().expect("invocation ledger locks").as_slice(),
        &[expected],
        "exactly one selected backend invocation must emit the requested payload"
    );
    result.expect("extract synthetic PNG")
}

fn assert_rendered_once(content: &str, payload: Payload) {
    assert_eq!(
        content.matches("ITEM2144").count(),
        1,
        "table body appears once: {content}"
    );
    assert_eq!(content.matches("READY2144").count(), 1);
    let expected_prose_count = usize::from(!matches!(payload, Payload::TableOnly | Payload::MarkdownTableOnly));
    assert_eq!(content.matches(PROSE).count(), expected_prose_count);
    let lines: Vec<&str> = content.lines().collect();
    let body_index = lines
        .iter()
        .position(|line| line.contains("ITEM2144"))
        .expect("table row renders");
    assert!(body_index >= 2, "table has a header and separator before its body");
    let expected = expected_cells();
    assert_eq!(markdown_row_cells(lines[body_index - 2]), expected[0]);
    assert_eq!(markdown_row_cells(lines[body_index]), expected[1]);
    let separator = markdown_row_cells(lines[body_index - 1]);
    assert_eq!(separator.len(), 3);
    for cell in separator {
        let dashes = cell.trim_matches(':');
        assert!(dashes.len() >= 3 && dashes.chars().all(|character| character == '-'));
    }
}

fn markdown_row_cells(line: &str) -> Vec<String> {
    let row = line.trim();
    assert!(!row.contains("\\|"), "table pipes are not escaped: {row}");
    let cells = row.strip_prefix('|').and_then(|row| row.strip_suffix('|'));
    cells
        .expect("table row has structural outer pipes")
        .split('|')
        .map(|cell| cell.trim().to_owned())
        .collect()
}

fn assert_table_payload_and_rendering(result: ExtractedDocument, payload: Payload) {
    assert_eq!(result.tables.len(), 1, "document table survives pipeline assembly");
    assert_eq!(result.tables[0].cells, expected_cells());
    assert_eq!(result.tables[0].page_number, 1);
    let pages = result.pages.as_ref().expect("page output requested");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].page_number, 1);
    assert_eq!(pages[0].tables.len(), 1);
    assert_eq!(pages[0].tables[0].cells, expected_cells());
    assert_eq!(pages[0].tables[0].page_number, 1);
    assert_rendered_once(&result.content, payload);
    assert_rendered_once(&pages[0].content, payload);
}

#[tokio::test]
async fn should_preserve_standalone_image_pipeline_tables_and_render_them_once() {
    let payload = Payload::ProseAndTable;
    assert_table_payload_and_rendering(run("issue-2144-table-payload", payload).await, payload);
}

#[tokio::test]
async fn should_not_duplicate_pipeline_table_already_present_in_ocr_markdown() {
    let payload = Payload::MarkdownAndTable;
    assert_table_payload_and_rendering(run("issue-2144-gfm-payload", payload).await, payload);
}

#[tokio::test]
async fn should_preserve_pipeline_table_when_backend_text_is_empty() {
    let payload = Payload::TableOnly;
    assert_table_payload_and_rendering(run("issue-2144-table-only", payload).await, payload);
}

#[tokio::test]
async fn should_preserve_text_when_standalone_image_pipeline_has_no_tables() {
    let result = run("issue-2144-empty-control", Payload::ProseOnly).await;
    assert_eq!(result.tables.len(), 0);
    let pages = result.pages.as_ref().expect("page output requested");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].page_number, 1);
    assert_eq!(pages[0].tables.len(), 0);
    assert_eq!(result.content.matches(PROSE).count(), 1);
    assert_eq!(pages[0].content.matches(PROSE).count(), 1);
    assert_eq!(result.content.matches("ITEM2144").count(), 0);
    assert_eq!(pages[0].content.matches("ITEM2144").count(), 0);
}

#[tokio::test]
async fn should_render_table_once_when_the_legacy_markdown_transcript_is_table_only() {
    let payload = Payload::MarkdownTableOnly;
    assert_table_payload_and_rendering(run("ci-legacy-markdown-table-only", payload).await, payload);
}

#[tokio::test]
async fn should_keep_both_prose_paragraphs_when_removing_the_duplicate_legacy_table() {
    let payload = Payload::MarkdownWithTrailingProse;
    let result = run("ci-legacy-markdown-table-prose", payload).await;
    for content in [
        &result.content,
        &result.pages.as_ref().expect("page requested")[0].content,
    ] {
        assert_eq!(
            content
                .matches("The unchanged final inventory note remains here.")
                .count(),
            1
        );
        assert_eq!(content.matches(PROSE).count(), 1);
        assert!(
            content.find(PROSE).expect("first prose")
                < content
                    .find("The unchanged final inventory note remains here.")
                    .expect("last prose")
        );
    }
    assert_table_payload_and_rendering(result, payload);
}

#[tokio::test]
async fn should_keep_unmatched_legacy_markdown_table_content() {
    let payload = Payload::MarkdownWithUnmatchedTable;
    let result = run("ci-legacy-markdown-unmatched-table", payload).await;
    for content in [
        &result.content,
        &result.pages.as_ref().expect("page requested")[0].content,
    ] {
        assert_eq!(content.matches("KEEPOTHER2144").count(), 1);
        assert_eq!(content.matches("Other").count(), 1);
        assert_eq!(content.matches("Value").count(), 1);
        assert_eq!(content.matches("29").count(), 1);
    }
    assert_table_payload_and_rendering(result, payload);
}

#[tokio::test]
async fn should_render_table_cells_once_when_legacy_sidecar_has_no_markdown() {
    let payload = Payload::MarkdownWithEmptyTableMarkdown;
    assert_table_payload_and_rendering(run("ci-legacy-markdown-empty-sidecar", payload).await, payload);
}
