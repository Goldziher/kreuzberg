//! DOCX output (#1942): `OutputFormat::Custom("docx")` returns a base64-encoded `.docx`
//! package in `content`.
//!
//! The package is built from the Markdown rendering after every post-processor has run,
//! so redaction rewrites the text before any of it reaches the OOXML parts.

#![cfg(all(feature = "office", feature = "tokio-runtime"))]

use base64::Engine as _;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OutputFormat, extract};

const DOCX_MIME_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

fn docx() -> OutputFormat {
    OutputFormat::Custom("docx".to_string())
}

async fn extract_one(bytes: &[u8], mime_type: &str, config: &ExtractionConfig) -> ExtractedDocument {
    extract(ExtractInput::from_bytes(bytes, mime_type, None), config)
        .await
        .expect("extraction should succeed")
        .results
        .into_iter()
        .next()
        .expect("one input yields one result")
}

fn decode_package(result: &ExtractedDocument) -> Vec<u8> {
    assert_eq!(result.metadata.output_format.as_deref(), Some("docx"));
    base64::engine::general_purpose::STANDARD
        .decode(&result.content)
        .expect("DOCX content should be base64")
}

fn config(output_format: OutputFormat) -> ExtractionConfig {
    ExtractionConfig {
        output_format,
        use_cache: false,
        ..Default::default()
    }
}

const REPORT: &str = "# Quarterly report

Revenue grew **12%** in *Q3*, see [the dashboard](https://example.com/q3).

## Actions

1. Hire two engineers
2. Close the audit
   - evidence pack
   - sign-off

Regional totals:

| Region | Revenue |
| --- | --- |
| North | 120 |
| South | 95 |
";

#[tokio::test]
#[serial_test::serial]
async fn docx_output_round_trips_through_the_docx_extractor() {
    let source = extract_one(REPORT.as_bytes(), "text/markdown", &config(OutputFormat::Markdown)).await;
    let rendered = extract_one(REPORT.as_bytes(), "text/markdown", &config(docx())).await;
    let package = decode_package(&rendered);

    let reread = extract_one(&package, DOCX_MIME_TYPE, &config(OutputFormat::Markdown)).await;
    assert_eq!(reread.content, source.content);
}

/// The PDF extractor recovers headings only when the output format asks for structure.
#[cfg(feature = "pdf")]
#[tokio::test]
#[serial_test::serial]
async fn a_pdf_converted_to_docx_keeps_the_headings_markdown_output_has() {
    fn headings(markdown: &str) -> Vec<&str> {
        markdown.lines().filter(|line| line.starts_with('#')).collect()
    }

    let path = format!("{}/../../test_documents/pdf/338298584.pdf", env!("CARGO_MANIFEST_DIR"));
    let pdf = std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}; fetch the corpus first"));
    let markdown = extract_one(&pdf, "application/pdf", &config(OutputFormat::Markdown)).await;
    let rendered = extract_one(&pdf, "application/pdf", &config(docx())).await;
    let reread = extract_one(
        &decode_package(&rendered),
        DOCX_MIME_TYPE,
        &config(OutputFormat::Markdown),
    )
    .await;

    assert!(!headings(&markdown.content).is_empty(), "{}", markdown.content);
    assert_eq!(headings(&reread.content), headings(&markdown.content));
}

#[cfg(feature = "redaction")]
mod redaction {
    use std::io::{Cursor, Read};

    use super::*;
    use xberg::core::config::redaction::{RedactionConfig, RedactionTerm};
    use xberg::types::redaction::RedactionStrategy;

    fn redacting(output_format: OutputFormat, terms: &[&str]) -> ExtractionConfig {
        ExtractionConfig {
            redaction: Some(RedactionConfig {
                strategy: RedactionStrategy::Mask,
                custom_terms: terms
                    .iter()
                    .map(|term| RedactionTerm::labeled("person", *term))
                    .collect(),
                ..RedactionConfig::default()
            }),
            ..config(output_format)
        }
    }

    /// Every part of the package, decompressed.
    fn package_parts(package: &[u8]) -> Vec<(String, String)> {
        let mut archive = zip::ZipArchive::new(Cursor::new(package)).expect("the package should be a zip archive");
        (0..archive.len())
            .map(|index| {
                let mut entry = archive.by_index(index).expect("every entry should be readable");
                let mut xml = String::new();
                entry.read_to_string(&mut xml).expect("every part should be UTF-8");
                (entry.name().expect("every entry name should decode").into_owned(), xml)
            })
            .collect()
    }

    const PERSONNEL: &str = "# Review for Jane Doe

Contact Jane Doe through [her page](https://example.com/jdoe).

- Owner: Jane Doe

| Name | Role |
| --- | --- |
| Jane Doe | Lead |
";

    #[tokio::test]
    #[serial_test::serial]
    async fn redacted_terms_are_absent_from_every_part_of_the_package() {
        let result = extract_one(
            PERSONNEL.as_bytes(),
            "text/markdown",
            &redacting(docx(), &["Jane Doe", "jdoe"]),
        )
        .await;
        let report = result.redaction_report.as_ref().expect("a redaction report");
        assert!(report.total_redacted >= 6, "{report:?}");

        for (name, xml) in package_parts(&decode_package(&result)) {
            let lowered = xml.to_lowercase();
            assert!(!lowered.contains("jane doe"), "{name} leaks a redacted term:\n{xml}");
            assert!(!lowered.contains("jdoe"), "{name} leaks a redacted term:\n{xml}");
        }

        let reread = extract_one(&decode_package(&result), DOCX_MIME_TYPE, &config(OutputFormat::Plain)).await;
        assert!(reread.content.contains("Contact"), "{}", reread.content);
        assert!(reread.content.contains("Lead"), "{}", reread.content);
    }

    /// An archive member runs its own pipeline and is packaged before the parent's
    /// redaction pass walks into it. `UEsDB` is how every base64-encoded zip archive
    /// begins, so a parent pass that rewrote the member's `content` would corrupt it.
    #[cfg(feature = "archives")]
    #[tokio::test]
    #[serial_test::serial]
    async fn archive_members_keep_intact_packages_when_the_parent_is_redacted() {
        use std::io::Write;

        let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
        archive
            .start_file("member.md", zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"# Member\n\nJane Doe signed off.\n").unwrap();
        let bytes = archive.finish().unwrap().into_inner();

        let result = extract_one(&bytes, "application/zip", &redacting(docx(), &["Jane Doe", "UEsDB"])).await;
        let children = result.children.as_ref().expect("the archive member is extracted");
        let member = children
            .iter()
            .find(|child| child.path == "member.md")
            .expect("member.md is a child");

        let parts = package_parts(&decode_package(&member.result));
        let document = parts
            .iter()
            .find(|(name, _)| name == "word/document.xml")
            .map(|(_, xml)| xml)
            .expect("the member package has a document part");
        assert!(document.contains("signed off."), "{document}");
        assert!(!document.to_lowercase().contains("jane doe"), "{document}");
    }
}

/// A post-processor that rewrites `content` without the rendering makes the pipeline
/// discard the rendering (#331). DOCX must then report the plain-text fallback like any
/// other format, not package text that no processor saw.
mod fallback {
    use super::*;
    use async_trait::async_trait;
    use xberg::plugins::Plugin;
    use xberg::{PostProcessor, ProcessingStage, Result, register_post_processor, unregister_post_processor};

    const PROCESSOR_NAME: &str = "docx-fallback-rewriter";

    struct ContentOnlyRewriter;

    impl Plugin for ContentOnlyRewriter {
        fn name(&self) -> &str {
            PROCESSOR_NAME
        }
    }

    #[async_trait]
    impl PostProcessor for ContentOnlyRewriter {
        async fn process(&self, result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
            result.content = result.content.replace("SECRET-42", "[gone]");
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Late
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn a_discarded_rendering_falls_back_to_plain_text_instead_of_a_package() {
        register_post_processor(std::sync::Arc::new(ContentOnlyRewriter)).expect("register the processor");
        let result = extract_one(b"Invoice SECRET-42 is overdue.", "text/plain", &config(docx())).await;
        unregister_post_processor(PROCESSOR_NAME).expect("unregister the processor");

        assert_eq!(result.metadata.output_format.as_deref(), Some("plain"));
        assert!(
            result.content.contains("Invoice [gone] is overdue."),
            "{}",
            result.content
        );
        assert!(
            result
                .processing_warnings
                .iter()
                .any(|warning| warning.source == "output_format"),
            "{:?}",
            result.processing_warnings
        );
    }
}
