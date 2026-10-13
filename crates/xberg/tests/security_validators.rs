//! Hostile-input integration tests for the security validators wired into
//! extractors. Each test feeds a synthesised attack payload through the
//! public `extract_bytes_document_blocking` entry point and asserts the extraction fails
//! with `XbergError::Security`.
//!
//! The validators (`StringGrowthValidator`, `IterationValidator`,
//! `DepthValidator`, `EntityValidator`, `TableValidator`) are crate-private
//! helpers — the assertion target is the unified `Security` error variant
//! that bindings observe. All tests use `ExtractionConfig::security_limits`
//! to dial limits down so we can verify a bounded payload fires the cap
//! deterministically (rather than waiting for OOM at the production default).
//!
//! Coverage is bounded by what's actually wired today. This file grows as
//! more extractors get the `SecurityBudget` parameter threaded through their
//! parser loops.

mod helpers;
use helpers::extract_bytes_document_blocking;

use xberg::core::config::ExtractionConfig;
use xberg::extractors::security::SecurityLimits;

/// Build a `SecurityLimits` with everything dialled to a small cap so a
/// bounded hostile fixture can deterministically trip exactly one validator.
fn tight_limits() -> SecurityLimits {
    SecurityLimits {
        max_archive_size: 500 * 1024 * 1024,
        max_compression_ratio: 100,
        max_files_in_archive: 10_000,
        max_nesting_depth: 16,
        max_entity_length: 32,
        max_content_size: 4 * 1024,
        max_iterations: 10_000,
        max_xml_depth: 16,
        max_table_cells: 64,
        // No fixture in this file is a PDF, so the page cap is left at the
        // production default rather than dialled down like the others.
        max_pages: None,
    }
}

fn config_with_tight_limits() -> ExtractionConfig {
    ExtractionConfig {
        security_limits: Some(tight_limits()),
        ..ExtractionConfig::default()
    }
}

/// Plain XML body: 1000 nested `<a>` elements should trip `max_xml_depth = 16`.
#[test]
fn xml_depth_bomb_fires_security_error() {
    let mut payload = String::from("<root>");
    for _ in 0..1000 {
        payload.push_str("<a>");
    }
    payload.push_str("leaf");
    for _ in 0..1000 {
        payload.push_str("</a>");
    }
    payload.push_str("</root>");

    let cfg = config_with_tight_limits();
    let err = extract_bytes_document_blocking(payload.as_bytes(), "application/xml", &cfg)
        .expect_err("hostile XML must not extract successfully");

    assert!(
        matches!(err, xberg::XbergError::Security { .. }),
        "expected Security error, got {:?}",
        err
    );
    assert!(
        err.to_string().to_lowercase().contains("nesting"),
        "expected nesting-too-deep message, got {}",
        err
    );
}

/// XML body with one element whose name expands beyond `max_entity_length = 32`
/// once read as text content. Tests that text-content entities are checked.
#[test]
fn xml_oversize_text_fires_security_error() {
    let huge_text = "x".repeat(64);
    let payload = format!("<root>{}</root>", huge_text);

    let cfg = ExtractionConfig {
        strict_parse: true,
        ..config_with_tight_limits()
    };
    let err = extract_bytes_document_blocking(payload.as_bytes(), "application/xml", &cfg)
        .expect_err("oversize text must not extract successfully");

    assert!(
        matches!(err, xberg::XbergError::Security { .. }),
        "expected Security error, got {:?}",
        err
    );
    assert!(
        err.to_string().to_lowercase().contains("entity"),
        "expected entity-too-long message, got {}",
        err
    );
}

/// XML body with an attribute value longer than `max_entity_length = 32`.
#[test]
fn xml_oversize_attribute_fires_security_error() {
    let huge_attr = "v".repeat(128);
    let payload = format!("<root attr=\"{}\">ok</root>", huge_attr);

    let cfg = ExtractionConfig {
        strict_parse: true,
        ..config_with_tight_limits()
    };
    let err = extract_bytes_document_blocking(payload.as_bytes(), "application/xml", &cfg)
        .expect_err("oversize attribute must not extract successfully");

    assert!(
        matches!(err, xberg::XbergError::Security { .. }),
        "expected Security error, got {:?}",
        err
    );
    assert!(
        err.to_string().to_lowercase().contains("entity"),
        "expected entity-too-long message, got {}",
        err
    );
}

/// XML body whose accumulated text emit exceeds `max_content_size = 4 KiB`.
/// Each text node alone fits under `max_entity_length`, but the running total
/// trips `StringGrowthValidator`.
#[test]
fn xml_string_growth_fires_security_error() {
    let mut payload = String::from("<root>");
    for _ in 0..256 {
        payload.push_str("<n>");
        payload.push_str(&"x".repeat(30));
        payload.push_str("</n>");
    }
    payload.push_str("</root>");

    let cfg = config_with_tight_limits();
    let err = extract_bytes_document_blocking(payload.as_bytes(), "application/xml", &cfg)
        .expect_err("oversized cumulative content must not extract successfully");

    assert!(
        matches!(err, xberg::XbergError::Security { .. }),
        "expected Security error, got {:?}",
        err
    );
    assert!(
        err.to_string().to_lowercase().contains("content"),
        "expected content-too-large message, got {}",
        err
    );
}

/// XML body with more events than `max_iterations = 10_000`. Each empty element
/// is one Start + one End event, so 6 000 empties = 12 000 events.
#[test]
fn xml_iteration_bomb_fires_security_error() {
    let mut payload = String::from("<root>");
    for _ in 0..15_000 {
        payload.push_str("<n/>");
    }
    payload.push_str("</root>");

    let cfg = config_with_tight_limits();
    let err = extract_bytes_document_blocking(payload.as_bytes(), "application/xml", &cfg)
        .expect_err("oversized iteration count must not extract successfully");

    assert!(
        matches!(err, xberg::XbergError::Security { .. }),
        "expected Security error, got {:?}",
        err
    );
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("iteration") || msg.contains("content"),
        "expected iteration-or-content error, got {}",
        err
    );
}

/// Sanity check: a benign small XML document under all caps extracts
/// successfully with the same `tight_limits()` configuration.
#[test]
fn xml_benign_input_extracts_successfully() {
    let payload = "<root><greeting>hello world</greeting></root>";
    let cfg = config_with_tight_limits();
    let result = extract_bytes_document_blocking(payload.as_bytes(), "application/xml", &cfg)
        .expect("benign XML must extract under the same tight limits");
    assert!(
        result.content.contains("hello world"),
        "expected greeting in content, got: {}",
        result.content
    );
}
