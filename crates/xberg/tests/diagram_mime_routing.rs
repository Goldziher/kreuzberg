#![cfg(all(feature = "tree-sitter", feature = "office"))]

use xberg::{ExtractInput, ExtractInputKind, ExtractionConfig, TreeSitterConfig, extract};

const GRAPHVIZ: &str = "digraph example { first -> second; }\n";
const PLANTUML: &str = "@startuml\nAlice -> Bob: Hello\n@enduml\n";

fn config() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        tree_sitter: Some(TreeSitterConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn assert_source(input: ExtractInput, expected_content: &str) {
    let output = extract(input, &config())
        .await
        .expect("diagram extraction should succeed");
    assert_eq!(output.summary.errors, 0, "{:?}", output.errors);
    assert_eq!(output.results.len(), 1);
    let result = &output.results[0];
    assert_eq!(result.mime_type, "text/x-source-code");
    assert_eq!(result.content.trim(), expected_content.trim());
}

async fn assert_source_file(filename: &str, source: &str) {
    let directory = tempfile::tempdir().expect("create temporary diagram directory");
    let path = directory.path().join(filename);
    std::fs::write(&path, source).expect("write source diagram");
    assert_source(ExtractInput::from_uri(path.to_string_lossy()), source).await;
}

async fn assert_source_file_with_mime(mime_type: &str, source: &str) {
    let directory = tempfile::tempdir().expect("create temporary diagram directory");
    let path = directory.path().join("diagram.txt");
    std::fs::write(&path, source).expect("write source diagram");
    let mut input = ExtractInput::from_uri(path.to_string_lossy());
    input.mime_type = Some(mime_type.to_string());
    assert_source(input, source).await;
}

#[tokio::test]
async fn should_extract_graphviz_gv_as_source() {
    assert_source_file("example.gv", GRAPHVIZ).await;
}

#[tokio::test]
async fn should_extract_graphviz_dot_as_source() {
    assert_source_file("example.dot", GRAPHVIZ).await;
}

#[tokio::test]
async fn should_extract_named_graphviz_dot_bytes_as_source() {
    let input = ExtractInput {
        kind: ExtractInputKind::Bytes,
        bytes: Some(GRAPHVIZ.as_bytes().to_vec()),
        filename: Some("example.DOT".to_string()),
        ..Default::default()
    };
    assert_source(input, GRAPHVIZ).await;
}

#[tokio::test]
async fn should_extract_graphviz_mime_as_source_without_filename() {
    assert_source(
        ExtractInput::from_bytes(GRAPHVIZ.as_bytes().to_vec(), "text/vnd.graphviz", None),
        GRAPHVIZ,
    )
    .await;
    assert_source_file_with_mime("text/vnd.graphviz", GRAPHVIZ).await;
}

#[tokio::test]
async fn should_extract_plantuml_puml_as_source() {
    assert_source_file("example.puml", PLANTUML).await;
}

#[tokio::test]
async fn should_extract_plantuml_mime_as_source_without_filename() {
    assert_source(
        ExtractInput::from_bytes(PLANTUML.as_bytes().to_vec(), "text/x-plantuml", None),
        PLANTUML,
    )
    .await;
    assert_source_file_with_mime("text/x-plantuml", PLANTUML).await;
}

#[tokio::test]
async fn should_preserve_ole_word_template_extraction() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/doc/unit_test_lists.doc");
    let bytes = std::fs::read(&fixture).expect("materialize the existing DOC corpus fixture before running this test");
    assert!(bytes.starts_with(&[0xd0, 0xcf, 0x11, 0xe0]));
    let directory = tempfile::tempdir().expect("create temporary template directory");
    let path = directory.path().join("template.dot");
    std::fs::write(&path, bytes).expect("write Word template");
    let output = extract(ExtractInput::from_uri(path.to_string_lossy()), &config())
        .await
        .expect("Word template extraction should succeed");
    assert_eq!(output.summary.errors, 0, "{:?}", output.errors);
    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].mime_type, "application/msword");
    assert!(!output.results[0].content.trim().is_empty());
}
