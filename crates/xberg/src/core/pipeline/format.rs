//! Output format conversion for extraction results.
//!
//! This module handles the final step of output format application: swapping
//! pre-rendered content into the result and recording format metadata.
//!
//! The heavy rendering work (Markdown, Djot, HTML) is now done earlier in the
//! pipeline inside `derive_extraction_result`, which populates
//! `ExtractedDocument::formatted_content`. This function simply swaps that
//! pre-rendered content into the `content` field after post-processors have
//! operated on the plain-text version.

use crate::Result;
use crate::core::config::OutputFormat;
use crate::types::ExtractedDocument;
#[cfg(test)]
use std::borrow::Cow;

/// Apply output format conversion to the extraction result.
///
/// Records the output format in metadata and swaps in pre-rendered content
/// (produced during `derive_extraction_result`) if available.
///
/// This runs as the final pipeline step, after post-processors have operated
/// on the plain-text `content` field.
///
/// # Arguments
///
/// * `result` - The extraction result to modify
/// * `output_format` - The desired output format
#[cfg_attr(alef, alef(skip))]
pub fn apply_output_format(result: ExtractedDocument, output_format: OutputFormat) -> ExtractedDocument {
    let mut result = result;

    // #208: a `Custom(name)` format with no matching renderer plugin falls back to
    // plain text during derivation (`extraction::derive::derive_extraction_result`),
    // leaving `formatted_content` unset. That is the only way a `Custom` format can
    // reach this function with no pre-rendered content — every successful custom
    // render always produces `Some(_)`. Detect that fallback here so metadata does
    // not claim a format that was never actually produced. ~keep
    let custom_fallback_to_plain =
        matches!(output_format, OutputFormat::Custom(_)) && result.formatted_content.is_none();

    let format_name = match output_format {
        OutputFormat::Plain => "plain",
        OutputFormat::Markdown => "markdown",
        OutputFormat::Djot => "djot",
        OutputFormat::Html => "html",
        OutputFormat::Json => "json",
        OutputFormat::DocTags => "doctags",
        OutputFormat::Custom(ref name) => {
            if custom_fallback_to_plain {
                "plain"
            } else {
                name.as_str()
            }
        }
    };
    result.metadata.output_format = Some(format_name.to_string());

    if let Some(formatted) = result.formatted_content.take() {
        result.content = formatted;
        reconcile_boundaries_with_rendering(&mut result);
    }
    result
}

/// Page byte ranges were measured on the plain text, so swapping in a rendering leaves them
/// pointing into another string (#2166). Re-locate each page's rendered text in the final
/// content; when that cannot be verified, omit the ranges rather than publish wrong ones.
fn reconcile_boundaries_with_rendering(result: &mut ExtractedDocument) {
    let has_boundaries = result
        .metadata
        .pages
        .as_ref()
        .is_some_and(|structure| structure.boundaries.is_some());
    if !has_boundaries {
        return;
    }
    if crate::extraction::derive::reconcile_page_boundaries(
        &mut result.metadata,
        &result.content,
        result.pages.as_deref(),
    ) {
        return;
    }
    if let Some(structure) = result.metadata.pages.as_mut() {
        structure.boundaries = None;
    }
    crate::core::diagnostics::push_warning(
        &mut result.processing_warnings,
        "page_boundaries",
        "Page byte ranges were measured on the plain text and could not be mapped onto the \
         requested output format, so they were omitted",
    );
}

/// Hand the rendering in `content` to its renderer's finishing step.
///
/// Runs after every other pipeline step. A binary format such as DOCX renders Markdown,
/// so the post-processors rewrite it as text, and only becomes bytes (base64 in
/// `content`) here. A `Custom` format that fell back to plain text is left alone, since
/// `apply_output_format` then records `"plain"` rather than the requested name.
pub(crate) fn finish_output_format(result: &mut ExtractedDocument, output_format: &OutputFormat) -> Result<()> {
    let OutputFormat::Custom(name) = output_format else {
        return Ok(());
    };
    if result.metadata.output_format.as_deref() != Some(name.as_str()) {
        return Ok(());
    }

    crate::plugins::ensure_renderers_initialized();
    let registry = crate::plugins::registry::get_renderer_registry();
    let registry = registry.read();
    let rendered = std::mem::take(&mut result.content);
    result.content = registry.finish(name, rendered)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Metadata;

    #[test]
    fn test_apply_output_format_plain() {
        let result = ExtractedDocument {
            content: "Hello World".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Plain);

        assert_eq!(result.content, "Hello World");
        assert_eq!(result.metadata.output_format, Some("plain".to_string()));
    }

    #[test]
    fn test_apply_output_format_markdown_no_prerender() {
        let result = ExtractedDocument {
            content: "Hello World".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Markdown);

        assert_eq!(result.content, "Hello World");
        assert_eq!(result.metadata.output_format, Some("markdown".to_string()));
    }

    fn two_page_result(plain: &str, formatted: &str, rendered_pages: [&str; 2]) -> ExtractedDocument {
        use crate::types::{PageBoundary, PageContent, PageStructure, PageUnitType};
        let page = |page_number: u32, content: &str| -> PageContent {
            serde_json::from_value(serde_json::json!({"page_number": page_number, "content": content})).unwrap()
        };
        let mut result = ExtractedDocument {
            content: plain.to_string(),
            formatted_content: Some(formatted.to_string()),
            pages: Some(vec![page(1, rendered_pages[0]), page(2, rendered_pages[1])]),
            ..Default::default()
        };
        result.metadata.pages = Some(PageStructure {
            total_count: 2,
            unit_type: PageUnitType::Page,
            boundaries: Some(vec![
                PageBoundary {
                    byte_start: 0,
                    byte_end: 5,
                    page_number: 1,
                },
                PageBoundary {
                    byte_start: 7,
                    byte_end: 12,
                    page_number: 2,
                },
            ]),
            pages: None,
        });
        result
    }

    #[test]
    fn issue_2166_should_relocate_page_boundaries_in_the_rendered_content() {
        let markdown = "# Page one body\n\n| a | b |\n\n# Page two body";
        let result = two_page_result(
            "Page1\n\nPage2",
            markdown,
            ["# Page one body\n\n| a | b |", "# Page two body"],
        );

        let result = apply_output_format(result, OutputFormat::Markdown);

        let boundaries = result.metadata.pages.unwrap().boundaries.unwrap();
        let slices: Vec<_> = boundaries
            .iter()
            .map(|b| &result.content[b.byte_start..b.byte_end])
            .collect();
        assert_eq!(slices, ["# Page one body\n\n| a | b |", "# Page two body"]);
        assert!(result.processing_warnings.is_empty());
    }

    #[test]
    fn issue_2166_should_omit_page_boundaries_that_cannot_be_verified() {
        let result = two_page_result("Page1\n\nPage2", "- one\n- two\n- three", ["1. one", "1. two"]);

        let result = apply_output_format(result, OutputFormat::Markdown);

        assert_eq!(result.content, "- one\n- two\n- three");
        assert!(result.metadata.pages.unwrap().boundaries.is_none());
        assert!(result.processing_warnings.iter().any(|w| w.source == "page_boundaries"));
    }

    #[test]
    fn test_apply_output_format_swaps_formatted_content() {
        let result = ExtractedDocument {
            content: "plain text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            formatted_content: Some("# Heading\n\nFormatted markdown".to_string()),
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Markdown);

        assert_eq!(result.content, "# Heading\n\nFormatted markdown");
        assert!(result.formatted_content.is_none(), "formatted_content should be taken");
        assert_eq!(result.metadata.output_format, Some("markdown".to_string()));
    }

    #[test]
    fn test_apply_output_format_html_with_prerender() {
        let result = ExtractedDocument {
            content: "plain text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            formatted_content: Some("<p>Hello World</p>".to_string()),
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Html);

        assert_eq!(result.content, "<p>Hello World</p>");
        assert_eq!(result.metadata.output_format, Some("html".to_string()));
    }

    #[test]
    fn test_apply_output_format_djot_with_prerender() {
        let result = ExtractedDocument {
            content: "plain text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            formatted_content: Some("# Djot heading".to_string()),
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Djot);

        assert_eq!(result.content, "# Djot heading");
        assert_eq!(result.metadata.output_format, Some("djot".to_string()));
    }

    /// `DocTags` must get its own "doctags" metadata label, not be mislabeled by
    /// the `Custom` fallback logic — it is a first-class variant with a renderer
    /// that always exists, unlike `Custom`, which can legitimately have none.
    #[test]
    fn should_label_doctags_metadata_with_its_own_format_name() {
        let result = ExtractedDocument {
            content: "plain text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            formatted_content: Some("<doctag><text>Hello</text></doctag>".to_string()),
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::DocTags);

        assert_eq!(result.content, "<doctag><text>Hello</text></doctag>");
        assert_eq!(result.metadata.output_format, Some("doctags".to_string()));
    }

    #[test]
    fn test_apply_output_format_preserves_metadata() {
        use ahash::AHashMap;
        let mut additional = AHashMap::new();
        additional.insert(Cow::Borrowed("custom_key"), serde_json::json!("custom_value"));
        let metadata = Metadata {
            title: Some("Test Title".to_string()),
            additional,
            ..Default::default()
        };

        let result = ExtractedDocument {
            content: "Hello World".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            metadata,
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Markdown);

        assert_eq!(result.metadata.title, Some("Test Title".to_string()));
        assert_eq!(
            result.metadata.additional.get("custom_key"),
            Some(&serde_json::json!("custom_value"))
        );
    }

    #[test]
    fn test_apply_output_format_preserves_tables() {
        use crate::types::Table;

        let table = Table {
            cells: vec![vec!["A".to_string(), "B".to_string()]],
            markdown: "| A | B |".to_string(),
            page_number: 1,
            bounding_box: None,
            ..Default::default()
        };

        let result = ExtractedDocument {
            content: "Hello World".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            tables: vec![table],
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Html);

        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].cells[0][0], "A");
    }

    /// #208: an unknown/renderer-less `Custom` format must not be mislabeled in
    /// metadata as the requested (unproduced) format. `formatted_content` being
    /// `None` for a `Custom` format only ever happens via the derivation
    /// fallback-to-plain path, so metadata must say "plain", not the typo'd name.
    #[test]
    fn test_apply_output_format_custom_without_renderer_reports_plain_not_the_requested_name() {
        let result = ExtractedDocument {
            content: "plain text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            formatted_content: None,
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Custom("markdwon".to_string()));

        assert_eq!(result.content, "plain text");
        assert_eq!(
            result.metadata.output_format,
            Some("plain".to_string()),
            "metadata must not claim the requested custom format was produced when it was not"
        );
    }

    /// A `Custom` format that *did* render successfully must still be labelled
    /// with the requested name, not overridden to "plain".
    #[test]
    fn test_apply_output_format_custom_with_renderer_reports_the_requested_name() {
        let result = ExtractedDocument {
            content: "plain text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            formatted_content: Some("<custom/>".to_string()),
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Custom("my-xml".to_string()));

        assert_eq!(result.content, "<custom/>");
        assert_eq!(result.metadata.output_format, Some("my-xml".to_string()));
    }

    #[test]
    fn test_apply_output_format_sets_typed_field() {
        let result = ExtractedDocument {
            content: "test".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Djot);

        assert_eq!(result.metadata.output_format, Some("djot".to_string()));
    }
}
