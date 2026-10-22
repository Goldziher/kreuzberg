use super::*;

const TEXT: &[u8] = b"hello world\n";
const GENERIC: [&str; 6] = [
    "application/octet-stream",
    "application/octet-stream; charset=utf-8",
    "Application/Octet-Stream",
    " application/octet-stream ",
    " Application/Octet-Stream; charset=\"utf-8\" ",
    "application/octet-stream; name=sample",
];
const REJECTED: [&str; 5] = [
    "application/octet-stream; charset=\"unterminated",
    "application/octet-stream; =bad",
    "application/unknown",
    "binary/octet-stream",
    "application/x-octet-stream",
];

fn config() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        ..Default::default()
    }
}

fn assert_text(document: &ExtractedDocument) {
    assert_eq!(document.mime_type, "text/plain");
    assert_eq!(document.content.trim(), "hello world");
}

#[tokio::test]
async fn should_detect_generic_declarations_in_extract_bytes() {
    for declared in GENERIC {
        assert_text(&extract_bytes(TEXT, declared, &config()).await.unwrap());
    }
}

#[tokio::test]
async fn should_preserve_explicit_validation_in_extract_bytes() {
    assert_text(
        &extract_bytes(TEXT, " Text/Plain; charset=utf-8 ", &config())
            .await
            .unwrap(),
    );
    for declared in REJECTED {
        assert!(
            matches!(
                extract_bytes(TEXT, declared, &config()).await,
                Err(crate::XbergError::UnsupportedFormat(_))
            ),
            "accepted {declared:?}"
        );
    }
    assert!(extract_bytes(TEXT, "application/pdf", &config()).await.is_err());
}

#[cfg(feature = "tower-service")]
async fn service_extract(declared: &str) -> crate::Result<ExtractedDocument> {
    use crate::service::{ExtractionRequest, ExtractionService};
    use tower::ServiceExt;
    ExtractionService::new()
        .oneshot(ExtractionRequest::bytes(TEXT, declared, config()))
        .await
}

#[cfg(feature = "tower-service")]
#[tokio::test]
async fn should_detect_generic_declarations_in_tower_bytes_service() {
    for declared in GENERIC {
        assert_text(&service_extract(declared).await.unwrap());
    }
}

#[cfg(feature = "tower-service")]
#[tokio::test]
async fn should_preserve_explicit_validation_in_tower_bytes_service() {
    assert_text(&service_extract("Text/Plain; charset=utf-8").await.unwrap());
    for declared in REJECTED {
        assert!(
            matches!(
                service_extract(declared).await,
                Err(crate::XbergError::UnsupportedFormat(_))
            ),
            "accepted {declared:?}"
        );
    }
    assert!(service_extract("application/pdf").await.is_err());
}

#[tokio::test]
async fn should_extract_shell_aliases_with_available_source_extractor() {
    let script = b"#!/usr/bin/env bash\nset -eu\necho hello\n";
    #[cfg(not(feature = "tree-sitter"))]
    let configuration = config();
    #[cfg(feature = "tree-sitter")]
    let configuration = ExtractionConfig {
        tree_sitter: Some(crate::core::config::TreeSitterConfig {
            enabled: false,
            ..Default::default()
        }),
        ..config()
    };
    for declared in [
        "application/x-sh",
        "text/x-sh",
        "text/x-shellscript",
        "text/x-source-code",
    ] {
        let input = crate::ExtractInput::from_bytes(script.to_vec(), declared, None);
        let output = crate::extract(input, &configuration).await.unwrap();
        assert_eq!(output.results.len(), 1);
        let document = &output.results[0];
        assert_eq!(document.mime_type, "text/x-source-code");
        assert_eq!(document.content.trim(), std::str::from_utf8(script).unwrap().trim());
        #[cfg(not(feature = "tree-sitter"))]
        assert!(matches!(
            document.metadata.format,
            Some(crate::types::FormatMetadata::Text(_))
        ));
        #[cfg(feature = "tree-sitter")]
        assert!(matches!(
            document.metadata.format,
            Some(crate::types::FormatMetadata::Code(_))
        ));
    }
}

#[cfg(not(feature = "tree-sitter"))]
#[tokio::test]
async fn should_preserve_legacy_source_alias_feature_requirement() {
    let declared = "text/x-python";
    assert_eq!(crate::core::mime::validate_mime_type(declared).unwrap(), declared);
    let input = crate::ExtractInput::from_bytes(b"print('hello')\n".to_vec(), declared, None);
    let error = crate::extract(input, &config()).await.unwrap_err();
    assert!(matches!(error, crate::XbergError::UnsupportedFormat(ref mime) if mime == declared));
}
