//! GH#2100 / GH#2101: text-like formats degrade to plain text with a warning instead of failing,
//! unless `strict_parse` is set.

mod helpers;

use helpers::{extract_bytes_document, extract_uri_document};
use xberg::{ExtractionConfig, SecurityLimits, XbergError};

const HELM_TEMPLATE: &[u8] = b"{{- if .Values.enabled }}\napiVersion: apps/v1\nkind: StatefulSet\nmetadata:\n  name: {{ include \"chart.fullname\" . }}\n{{- end }}\n";

fn strict() -> ExtractionConfig {
    ExtractionConfig {
        strict_parse: true,
        ..Default::default()
    }
}

fn has_fallback_warning(doc: &xberg::ExtractedDocument, needle: &str) -> bool {
    doc.processing_warnings
        .iter()
        .any(|w| w.source == "parse-fallback" && w.message.contains(needle))
}

#[tokio::test]
async fn should_fall_back_to_plain_text_for_helm_template_yaml() {
    let doc = extract_bytes_document(HELM_TEMPLATE, "application/x-yaml", &ExtractionConfig::default())
        .await
        .expect("lenient extraction must succeed");
    assert!(doc.content.contains("kind: StatefulSet"), "got: {}", doc.content);
    assert!(
        has_fallback_warning(&doc, "application/x-yaml"),
        "{:?}",
        doc.processing_warnings
    );
}

#[tokio::test]
async fn should_fall_back_for_yaml_read_from_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("statefulset.yaml");
    std::fs::write(&path, HELM_TEMPLATE).unwrap();
    let doc = extract_uri_document(&path, Some("application/x-yaml"), &ExtractionConfig::default())
        .await
        .expect("lenient file extraction must succeed");
    assert!(doc.content.contains("kind: StatefulSet"), "got: {}", doc.content);
    assert!(has_fallback_warning(&doc, "application/x-yaml"));

    let err = extract_uri_document(&path, Some("application/x-yaml"), &strict())
        .await
        .expect_err("strict file extraction must fail");
    assert!(matches!(err, XbergError::Parsing { .. }), "got: {err:?}");
}

#[tokio::test]
async fn should_fall_back_to_plain_text_for_invalid_json() {
    let doc = extract_bytes_document(b"{\"a\": 1,, nope", "application/json", &ExtractionConfig::default())
        .await
        .expect("lenient extraction must succeed");
    assert!(doc.content.contains("nope"), "got: {}", doc.content);
    assert!(has_fallback_warning(&doc, "application/json"));
}

#[tokio::test]
async fn should_fall_back_to_plain_text_for_invalid_toml() {
    let doc = extract_bytes_document(
        b"[section\nkey = = 1\nhello = world",
        "application/toml",
        &ExtractionConfig::default(),
    )
    .await
    .expect("lenient extraction must succeed");
    assert!(doc.content.contains("hello = world"), "got: {}", doc.content);
    assert!(has_fallback_warning(&doc, "application/toml"));
}

#[tokio::test]
async fn should_error_on_unparseable_yaml_json_toml_when_strict() {
    for (bytes, mime) in [
        (HELM_TEMPLATE, "application/x-yaml"),
        (&b"{\"a\": 1,, nope"[..], "application/json"),
        (&b"[section\nkey = = 1"[..], "application/toml"),
    ] {
        let err = extract_bytes_document(bytes, mime, &strict())
            .await
            .expect_err("strict extraction must fail");
        assert!(matches!(err, XbergError::Parsing { .. }), "{mime}: {err:?}");
    }
}

#[tokio::test]
async fn should_fall_back_for_csv_over_max_table_cells() {
    let csv = b"a,b,c\n1,2,3\n4,5,6\n7,8,9\n";
    let config = ExtractionConfig {
        security_limits: Some(SecurityLimits {
            max_table_cells: 5,
            ..Default::default()
        }),
        ..Default::default()
    };
    let doc = extract_bytes_document(csv, "text/csv", &config)
        .await
        .expect("lenient extraction must succeed");
    assert!(doc.content.contains("4,5,6"), "got: {}", doc.content);
    assert!(
        has_fallback_warning(&doc, "max_table_cells"),
        "{:?}",
        doc.processing_warnings
    );

    let strict_config = ExtractionConfig {
        strict_parse: true,
        ..config
    };
    let err = extract_bytes_document(csv, "text/csv", &strict_config)
        .await
        .expect_err("strict extraction must fail");
    assert!(matches!(err, XbergError::Security { .. }), "got: {err:?}");
}

#[cfg(feature = "xml")]
mod xml {
    use super::*;

    const UNTERMINATED_XML: &[u8] = b"<root><a>text</root";

    #[tokio::test]
    async fn should_fall_back_to_plain_text_for_invalid_xml() {
        let doc = extract_bytes_document(UNTERMINATED_XML, "application/xml", &ExtractionConfig::default())
            .await
            .expect("lenient extraction must succeed");
        assert!(doc.content.contains("<a>text</root"), "got: {}", doc.content);
        assert!(
            has_fallback_warning(&doc, "application/xml"),
            "{:?}",
            doc.processing_warnings
        );

        let err = extract_bytes_document(UNTERMINATED_XML, "application/xml", &strict())
            .await
            .expect_err("strict extraction must fail");
        assert!(matches!(err, XbergError::Parsing { .. }), "got: {err:?}");
    }

    fn svg_with_long_text(len: usize) -> Vec<u8> {
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\"><text>{}</text></svg>",
            "x".repeat(len)
        )
        .into_bytes()
    }

    fn small_entity_limit() -> ExtractionConfig {
        ExtractionConfig {
            security_limits: Some(SecurityLimits {
                max_entity_length: 64,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn should_fall_back_for_svg_with_over_limit_entity() {
        let svg = svg_with_long_text(500);
        let doc = extract_bytes_document(&svg, "image/svg+xml", &small_entity_limit())
            .await
            .expect("lenient extraction must succeed");
        assert!(doc.content.contains("xxxxxxxx"), "got: {}", doc.content);
        assert!(
            has_fallback_warning(&doc, "max_entity_length"),
            "{:?}",
            doc.processing_warnings
        );

        let err = extract_bytes_document(
            &svg,
            "image/svg+xml",
            &ExtractionConfig {
                strict_parse: true,
                ..small_entity_limit()
            },
        )
        .await
        .expect_err("strict extraction must fail");
        assert!(matches!(err, XbergError::Security { .. }), "got: {err:?}");
    }
}

#[tokio::test]
async fn should_truncate_fallback_content_on_a_char_boundary_and_warn() {
    let body = "é: {{ x }}\n".repeat(50);
    let config = ExtractionConfig {
        security_limits: Some(SecurityLimits {
            max_content_size: 101,
            ..Default::default()
        }),
        ..Default::default()
    };
    let doc = extract_bytes_document(body.as_bytes(), "application/x-yaml", &config)
        .await
        .expect("lenient extraction must succeed");
    assert!(doc.content.len() <= 101, "len {}", doc.content.len());
    assert!(!doc.content.contains('\u{FFFD}'), "cut inside a character");
    assert!(
        has_fallback_warning(&doc, "max_content_size"),
        "{:?}",
        doc.processing_warnings
    );
}
