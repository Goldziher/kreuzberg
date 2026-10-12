//! Plain-text recovery for text-like documents whose structured parse fails (GH#2100, GH#2101).
//!
//! Both extraction entry points (bytes and file) route a failed extractor call through
//! [`recovery_reason`]; when it answers, [`extract_plain_text`] re-reads the raw content as plain
//! text. Plain text never expands entities or builds tables, so the security limits that tripped
//! are not weakened by recovering this way.

use crate::Result;
use crate::XbergError;
use crate::core::config::ExtractionConfig;
use crate::core::mime::{KML_MIME_TYPE, ODG_FLAT_MIME_TYPE};
use crate::extractors::security::{SecurityError, SecurityLimits};
use crate::plugins::InternalDocumentExtractor;
use crate::types::ExtractedDocument;

/// `ProcessingWarning::source` for every warning recorded here. Part of the public result surface.
const WARNING_SOURCE: &str = "parse-fallback";

const STRUCTURED_MIMES: &[&str] = &[
    "application/json",
    "text/json",
    "application/csl+json",
    "application/geo+json",
    "application/vnd.geo+json",
    "application/x-ndjson",
    "application/jsonl",
    "application/x-jsonlines",
    "application/yaml",
    "application/x-yaml",
    "text/yaml",
    "text/x-yaml",
    "application/toml",
    "text/toml",
];

const XML_MIMES: &[&str] = &[
    "application/xml",
    "text/xml",
    KML_MIME_TYPE,
    "image/svg+xml",
    "application/x-endnote+xml",
    ODG_FLAT_MIME_TYPE,
];

const CSV_MIMES: &[&str] = &["text/csv", "text/tab-separated-values"];

fn is_parse_recoverable(mime_type: &str) -> bool {
    STRUCTURED_MIMES.contains(&mime_type) || XML_MIMES.contains(&mime_type)
}

fn is_limit_recoverable(mime_type: &str) -> bool {
    is_parse_recoverable(mime_type) || CSV_MIMES.contains(&mime_type)
}

/// Why `error` from the extractor for `mime_type` may be recovered as plain text, as the
/// warning message to attach. `None` means the error must be returned unchanged.
pub(super) fn recovery_reason(error: &XbergError, mime_type: &str, config: &ExtractionConfig) -> Option<String> {
    if config.strict_parse {
        return None;
    }
    match error {
        XbergError::Parsing { message, .. } if is_parse_recoverable(mime_type) => Some(format!(
            "structured parsing of {mime_type} failed ({message}); the content was extracted as plain text"
        )),
        XbergError::Security { source, .. } if is_limit_recoverable(mime_type) => {
            let security = source.as_deref()?.downcast_ref::<SecurityError>()?;
            let limit = match security {
                SecurityError::EntityTooLong { .. } => "security_limits.max_entity_length",
                SecurityError::TooManyCells { .. } => "security_limits.max_table_cells",
                _ => return None,
            };
            Some(format!(
                "{security}; `{limit}` tripped, so the content was extracted as plain text"
            ))
        }
        _ => None,
    }
}

fn max_content_size(config: &ExtractionConfig) -> usize {
    config.security_limits.as_ref().map_or_else(
        || SecurityLimits::default().max_content_size,
        |limits| limits.max_content_size,
    )
}

/// The most raw bytes recovery will read from a file: one past the content budget, so an
/// over-budget file is detected without being read in full.
pub(super) fn read_budget(config: &ExtractionConfig) -> u64 {
    (max_content_size(config) as u64).saturating_add(1)
}

fn truncate_on_char_boundary(content: &[u8], max: usize) -> &[u8] {
    let cut = &content[..max];
    match std::str::from_utf8(cut) {
        Err(error) if error.error_len().is_none() => &cut[..error.valid_up_to()],
        _ => cut,
    }
}

/// Re-extract `content` as plain text, recording `reason` (and any truncation to
/// `security_limits.max_content_size`) in the result's warnings.
pub(super) async fn extract_plain_text(
    content: &[u8],
    mime_type: &str,
    config: &ExtractionConfig,
    reason: &str,
) -> Result<ExtractedDocument> {
    let max = max_content_size(config);
    let truncated = content.len() > max;
    let content = if truncated {
        truncate_on_char_boundary(content, max)
    } else {
        content
    };

    let mut doc = crate::extractors::text::PlainTextExtractor::new()
        .extract_content(content, mime_type, config)
        .await?;
    crate::core::diagnostics::push_warning(&mut doc.processing_warnings, WARNING_SOURCE, reason);
    if truncated {
        crate::core::diagnostics::push_warning(
            &mut doc.processing_warnings,
            WARNING_SOURCE,
            format!("the plain-text content was truncated to {max} bytes (`security_limits.max_content_size`)"),
        );
    }
    super::file::ensure_builtin_extraction_method(&mut doc, true);
    Box::pin(crate::core::pipeline::run_pipeline(doc, config)).await
}
