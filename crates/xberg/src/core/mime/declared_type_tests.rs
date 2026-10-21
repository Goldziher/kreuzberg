use super::*;
use crate::core::config::{ExtractionConfig, MimeDetectionPolicy};

const TEXT: &[u8] = b"hello world\n";
const BIBTEX: &[u8] = b"@article{routing, title={Generic MIME routing}, author={Example, Test}, year={2026}}";
const POLICIES: [MimeDetectionPolicy; 3] = [
    MimeDetectionPolicy::PreferContent,
    MimeDetectionPolicy::TrustExtension,
    MimeDetectionPolicy::ContentOnly,
];

fn config() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        ..Default::default()
    }
}

async fn assert_generic_detection(declared: &str) {
    let input = crate::ExtractInput::from_bytes(TEXT.to_vec(), declared, None);
    let output = crate::extract(input, &config()).await.unwrap();
    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].mime_type, PLAIN_TEXT_MIME_TYPE);
    assert_eq!(output.results[0].content.trim(), "hello world");
    assert_byte_detection(declared);
    assert_file_detection(declared);
    assert_owned_detection(declared).await;
    #[cfg(feature = "office")]
    assert_bibtex_extraction(declared).await;
}

fn assert_byte_detection(declared: &str) {
    for policy in POLICIES {
        assert_eq!(
            detect_or_validate_bytes(TEXT, None, Some(declared), policy).unwrap(),
            PLAIN_TEXT_MIME_TYPE
        );
        let expected = if policy == MimeDetectionPolicy::ContentOnly {
            PLAIN_TEXT_MIME_TYPE
        } else {
            "application/x-bibtex"
        };
        assert_eq!(
            detect_or_validate_bytes(BIBTEX, Some("routing.bib"), Some(declared), policy).unwrap(),
            expected
        );
    }
}

fn assert_file_detection(declared: &str) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("routing.bib");
    std::fs::write(&path, BIBTEX).unwrap();
    for policy in POLICIES {
        let mut file = std::fs::File::open(&path).unwrap();
        let expected = if policy == MimeDetectionPolicy::ContentOnly {
            PLAIN_TEXT_MIME_TYPE
        } else {
            "application/x-bibtex"
        };
        assert_eq!(
            detect_or_validate_file(&path, &mut file, Some(declared), policy).unwrap(),
            expected
        );
    }
}

async fn assert_owned_detection(declared: &str) {
    for policy in POLICIES {
        for filename in [None, Some("routing.bib".to_string())] {
            let expected = if filename.is_none() || policy == MimeDetectionPolicy::ContentOnly {
                PLAIN_TEXT_MIME_TYPE
            } else {
                "application/x-bibtex"
            };
            let (content, detected) =
                resolve_owned_bytes_mime(BIBTEX.to_vec(), filename, Some(declared.to_string()), policy)
                    .await
                    .unwrap();
            assert_eq!(content, BIBTEX);
            assert_eq!(detected, expected);
        }
    }
}

#[cfg(feature = "office")]
async fn assert_bibtex_extraction(declared: &str) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("routing.bib");
    std::fs::write(&path, BIBTEX).unwrap();
    let mut input = crate::ExtractInput::from_uri(path.to_string_lossy().into_owned());
    input.mime_type = Some(declared.to_string());
    #[cfg(feature = "tokio-runtime")]
    {
        let output = crate::extract(input, &config()).await.unwrap();
        assert_eq!(output.results.len(), 1);
        assert_eq!(output.results[0].mime_type, "application/x-bibtex");
        assert!(output.results[0].content.contains("Generic MIME routing"));
    }
    #[cfg(not(feature = "tokio-runtime"))]
    assert!(matches!(
        crate::extract(input, &config()).await,
        Err(XbergError::Other(message)) if message == "Path extraction requires the tokio-runtime feature"
    ));
    let input = crate::ExtractInput::from_bytes(BIBTEX.to_vec(), declared, Some("routing.bib".to_string()));
    let output = crate::extract(input, &config()).await.unwrap();
    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].mime_type, "application/x-bibtex");
    assert!(output.results[0].content.contains("Generic MIME routing"));
}

#[tokio::test]
async fn should_keep_specific_declared_types_authoritative() {
    for policy in POLICIES {
        let result = detect_or_validate_bytes(TEXT, Some("routing.bib"), Some("application/json"), policy);
        assert_eq!(result.unwrap(), JSON_MIME_TYPE);
        let (_, detected) = resolve_owned_bytes_mime(
            TEXT.to_vec(),
            Some("routing.bib".to_string()),
            Some("application/json".to_string()),
            policy,
        )
        .await
        .unwrap();
        assert_eq!(detected, JSON_MIME_TYPE);
    }
}

#[tokio::test]
async fn should_accept_parameters_on_specific_declared_types() {
    for declared in [" Text/Plain; charset=utf-8 ", "text/plain; charset="] {
        assert!(declared.trim().parse::<mime::Mime>().is_ok());
        assert_eq!(validate_mime_type(declared).unwrap(), PLAIN_TEXT_MIME_TYPE);
        for policy in POLICIES {
            assert_eq!(
                detect_or_validate_bytes(TEXT, Some("routing.bib"), Some(declared), policy).unwrap(),
                PLAIN_TEXT_MIME_TYPE
            );
        }
        let input = crate::ExtractInput::from_bytes(TEXT.to_vec(), declared, Some("routing.bib".to_string()));
        let output = crate::extract(input, &config()).await.unwrap();
        assert_eq!(output.results[0].mime_type, PLAIN_TEXT_MIME_TYPE);
        assert_eq!(output.results[0].content.trim(), "hello world");
    }
}

async fn assert_declared_types_rejected(declared: &[&str]) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("routing.txt");
    std::fs::write(&path, TEXT).unwrap();
    for declared in declared {
        assert!(matches!(
            validate_mime_type(declared),
            Err(XbergError::UnsupportedFormat(_))
        ));
        for policy in POLICIES {
            let mut file = std::fs::File::open(&path).unwrap();
            assert!(matches!(
                detect_or_validate_file(&path, &mut file, Some(declared), policy),
                Err(XbergError::UnsupportedFormat(_))
            ));
            assert!(matches!(
                detect_or_validate_bytes(TEXT, None, Some(declared), policy),
                Err(XbergError::UnsupportedFormat(_))
            ));
            assert!(matches!(
                resolve_owned_bytes_mime(TEXT.to_vec(), None, Some((*declared).to_string()), policy).await,
                Err(XbergError::UnsupportedFormat(_))
            ));
        }
    }
}

#[tokio::test]
async fn should_reject_malformed_declared_types_instead_of_detecting() {
    let declared = [
        "not a MIME",
        "application/octet-stream; charset=\"unterminated",
        "application/octet-stream; charset",
        "application/octet-stream garbage",
    ];
    for value in &declared {
        assert!(value.trim().parse::<mime::Mime>().is_err(), "parsed {value:?}");
    }
    assert_declared_types_rejected(&declared).await;
}

#[tokio::test]
async fn should_not_treat_other_unsupported_types_as_generic() {
    assert_declared_types_rejected(&[
        "binary/octet-stream",
        "application/x-octet-stream",
        "application/unknown",
        "application/x-not-supported",
    ])
    .await;
}

#[test]
fn should_preserve_preexisting_alias_return_values() {
    for alias in ["text/x-tex", "text/x-python", "text/x-asciidoc", "application/docx"] {
        assert_eq!(validate_mime_type(alias).unwrap(), alias);
    }
}

#[test]
fn should_canonicalize_registered_client_mime_aliases() {
    for (alias, canonical) in [
        ("application/x-tex", "application/x-latex"),
        ("application/x-sh", "text/x-source-code"),
        ("text/x-sh", "text/x-source-code"),
        ("text/x-shellscript", "text/x-source-code"),
        ("application/vnd.apple.pages", "application/x-iwork-pages-sffpages"),
        (
            "application/vnd.apple.numbers",
            "application/x-iwork-numbers-sffnumbers",
        ),
        ("application/vnd.apple.keynote", "application/x-iwork-keynote-sffkey"),
    ] {
        assert_eq!(validate_mime_type(alias).unwrap(), canonical);
    }
}

#[tokio::test]
async fn should_detect_octet_stream_with_bare() {
    assert_generic_detection("application/octet-stream").await;
}

#[tokio::test]
async fn should_detect_octet_stream_with_parameters() {
    assert_generic_detection("application/octet-stream; charset=utf-8").await;
    assert_generic_detection("application/octet-stream; charset=").await;
}

#[tokio::test]
async fn should_detect_octet_stream_with_uppercase() {
    assert_generic_detection("APPLICATION/OCTET-STREAM").await;
}

#[tokio::test]
async fn should_detect_octet_stream_with_surrounding_whitespace() {
    assert_generic_detection(" application/octet-stream ").await;
}

#[tokio::test]
async fn should_detect_octet_stream_with_mixed_case_and_parameters() {
    assert_generic_detection(" \tApplication/Octet-Stream; charset=\"utf-8\" \t").await;
}

#[cfg(all(feature = "office", feature = "iwork", feature = "tree-sitter"))]
mod aliases {
    use super::*;
    use std::io::Write;

    const SAMPLE_TEXT: &str = "A small MIME routing sample.";

    fn alias_config() -> ExtractionConfig {
        ExtractionConfig {
            tree_sitter: Some(crate::TreeSitterConfig {
                enabled: false,
                ..Default::default()
            }),
            ..config()
        }
    }

    async fn assert_alias_extracts(alias: &str, canonical: &str, content: Vec<u8>, expected: &str) {
        let baseline = crate::extract(
            crate::ExtractInput::from_bytes(content.clone(), canonical, None),
            &alias_config(),
        )
        .await
        .unwrap();
        assert_eq!(baseline.results.len(), 1);
        assert_eq!(baseline.results[0].mime_type, canonical);
        assert!(baseline.results[0].content.contains(expected));
        assert_eq!(validate_mime_type(alias).unwrap(), canonical);
        let output = crate::extract(crate::ExtractInput::from_bytes(content, alias, None), &alias_config())
            .await
            .unwrap();
        assert_eq!(output.results.len(), 1);
        assert_eq!(output.results[0].mime_type, canonical);
        assert_eq!(output.results[0].content, baseline.results[0].content);
        assert!(output.results[0].content.contains(expected));
    }

    fn iwork_sample(member: &str) -> Vec<u8> {
        const TEXT_FIELD: u8 = 0x1a;
        const UNCOMPRESSED_FRAME: u8 = 1;
        let text_length = u8::try_from(SAMPLE_TEXT.len()).unwrap();
        let mut payload = vec![TEXT_FIELD, text_length];
        payload.extend_from_slice(SAMPLE_TEXT.as_bytes());
        let payload_length = u32::try_from(payload.len()).unwrap().to_le_bytes();
        let mut frame = vec![UNCOMPRESSED_FRAME];
        frame.extend_from_slice(&payload_length[..3]);
        frame.extend_from_slice(&payload);
        let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);
        archive.start_file(member, options).unwrap();
        archive.write_all(&frame).unwrap();
        archive.finish().unwrap().into_inner()
    }

    #[tokio::test]
    async fn should_extract_registered_latex_alias() {
        assert_alias_extracts(
            "application/x-tex",
            "application/x-latex",
            b"\\documentclass{article}\n\\begin{document}\nA small MIME routing sample.\n\\end{document}\n".to_vec(),
            SAMPLE_TEXT,
        )
        .await;
    }

    #[tokio::test]
    async fn should_extract_registered_application_shell_alias() {
        assert_alias_extracts(
            "application/x-sh",
            "text/x-source-code",
            b"#!/usr/bin/env bash\necho hello\n".to_vec(),
            "echo hello",
        )
        .await;
    }

    #[tokio::test]
    async fn should_extract_registered_text_shell_alias() {
        assert_alias_extracts(
            "text/x-sh",
            "text/x-source-code",
            b"#!/usr/bin/env bash\necho hello\n".to_vec(),
            "echo hello",
        )
        .await;
    }

    #[tokio::test]
    async fn should_extract_registered_shellscript_alias() {
        assert_alias_extracts(
            "text/x-shellscript",
            "text/x-source-code",
            b"#!/usr/bin/env bash\necho hello\n".to_vec(),
            "echo hello",
        )
        .await;
    }

    #[tokio::test]
    async fn should_extract_registered_keynote_alias() {
        assert_alias_extracts(
            "application/vnd.apple.keynote",
            "application/x-iwork-keynote-sffkey",
            iwork_sample("Index/Slide-1.iwa"),
            SAMPLE_TEXT,
        )
        .await;
    }

    #[tokio::test]
    async fn should_extract_registered_numbers_alias() {
        assert_alias_extracts(
            "application/vnd.apple.numbers",
            "application/x-iwork-numbers-sffnumbers",
            iwork_sample("Index/Document.iwa"),
            SAMPLE_TEXT,
        )
        .await;
    }

    #[tokio::test]
    async fn should_extract_registered_pages_alias() {
        assert_alias_extracts(
            "application/vnd.apple.pages",
            "application/x-iwork-pages-sffpages",
            iwork_sample("Index/Document.iwa"),
            SAMPLE_TEXT,
        )
        .await;
    }
}
