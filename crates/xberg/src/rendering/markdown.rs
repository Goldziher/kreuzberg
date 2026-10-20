//! Render an `InternalDocument` to GFM-compliant Markdown via comrak.

use comrak::nodes::{AstNode, NodeValue};
use comrak::{Arena, Options, format_commonmark};
use std::borrow::Cow;
use std::sync::LazyLock;

use crate::types::annotations::PdfAnnotation;
use crate::types::internal::InternalDocument;

use super::common::{annotation_display_text, annotation_type_label};
use super::comrak_bridge::{build_comrak_ast, keep_code_literal};

/// Single-pass replacement of multiple two-char escape sequences of the form `\X`
/// where X is one of `_`, `[`, `]`, `(`, `)`.
///
/// Returns [`Cow::Borrowed`] when no replacement occurs (zero allocation).
/// Returns [`Cow::Owned`] with one pre-sized allocation when any hit is found.
fn unescape_backslash_sequences<'a>(input: &'a str, targets: &[char]) -> Cow<'a, str> {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;

    let first_hit = loop {
        if i + 1 >= len {
            return Cow::Borrowed(input);
        }
        if bytes[i] == b'\\' {
            let next = bytes[i + 1] as char;
            if targets.contains(&next) {
                break i;
            }
        }
        i += 1;
    };

    let mut out = String::with_capacity(input.len());
    out.push_str(&input[..first_hit]);
    i = first_hit;

    while i < len {
        if i + 1 < len && bytes[i] == b'\\' {
            let next = bytes[i + 1] as char;
            if targets.contains(&next) {
                out.push(next);
                i += 2;
                continue;
            }
        }
        let c = input[i..].chars().next().expect("valid UTF-8");
        out.push(c);
        i += c.len_utf8();
    }

    Cow::Owned(out)
}

/// ~keep Write tabs in Text as spaces, not comrak's `&#9;`; `normalize_text` drops other controls.
fn replace_text_tabs<'a>(root: &'a AstNode<'a>) {
    for node in root.descendants() {
        if let NodeValue::Text(text) = &mut node.data.borrow_mut().value
            && text.contains('\t')
        {
            *text = Cow::Owned(text.replace('\t', " "));
        }
    }
}

/// Collapse runs of three or more consecutive newlines down to exactly two
/// using a single pass over the string.
///
/// Returns [`Cow::Borrowed`] when no triple-newline sequence is found.
fn collapse_excess_newlines(input: &str) -> Cow<'_, str> {
    if !input.contains("\n\n\n") {
        return Cow::Borrowed(input);
    }

    let mut out = String::with_capacity(input.len());
    let mut newline_run = 0usize;

    for c in input.chars() {
        if c == '\n' {
            newline_run += 1;
            if newline_run <= 2 {
                out.push('\n');
            }
        } else {
            newline_run = 0;
            out.push(c);
        }
    }

    Cow::Owned(out)
}

static ARXIV_WATERMARK_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"(?:\s+\S+(?:\s+\S+){0,8})?\s*arXiv:\d{4}\.\d{4,5}(?:v\d+)?(?:\s*\[[\w.-]+\])?\s*(?:\d{1,2}\s+\w+\s+\d{4})?",
    )
    .expect("static regex compiles")
});

/// Render an `InternalDocument` to GFM Markdown.
///
/// Whether the output backslash-escapes CommonMark-significant leading characters
/// (`-`, `#`, plus the always-unescaped `_[]()*=`) is controlled by
/// `doc.escape_markdown`, which the pipeline sets from
/// [`ExtractionConfig::escape_markdown`](crate::core::config::ExtractionConfig::escape_markdown).
/// Defaults to `true` (escaped), preserving prior behavior.
pub(crate) fn render_markdown(doc: &InternalDocument) -> String {
    tracing::debug!(element_count = doc.elements.len(), "markdown rendering starting");
    let arena = Arena::new();
    let root = build_comrak_ast(doc, &arena);
    replace_text_tabs(root);
    if !doc.include_watermarks {
        strip_arxiv_watermark_noise(root);
    }
    keep_code_literal(root, &arena);

    if root.first_child().is_none() {
        tracing::debug!("markdown rendering: empty AST, returning empty string");
        return finalize_annotations_appendix(doc.annotations.as_deref());
    }

    let mut options = comrak_options();
    options.render.width = 0;

    let mut output = String::new();
    format_commonmark(root, &options, &mut output).expect("comrak formatting should not fail");

    output = filter_page_marker_comments(output, doc.page_marker_format.as_deref());

    output = unescape_markdown_output(output, doc.escape_markdown);

    if let Cow::Owned(s) = collapse_excess_newlines(&output) {
        output = s;
    }

    output = append_annotations_section(output, doc.annotations.as_deref());

    let trimmed_len = output.trim_end().len();
    if trimmed_len == 0 {
        return String::new();
    }
    output.truncate(trimmed_len);
    output.push('\n');
    tracing::debug!(output_length = output.len(), "markdown rendering complete");
    output
}

/// Drop HTML comment lines from rendered output, except page-marker comments
/// that match `page_marker_format` (when configured).
fn filter_page_marker_comments(output: String, page_marker_format: Option<&str>) -> String {
    if !output.contains("<!--") {
        return output;
    }

    let marker_re = page_marker_format.map(crate::core::config::page::marker_line_regex);
    output
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("<!--")
                || !trimmed.ends_with("-->")
                || marker_re.as_ref().is_some_and(|re| re.is_match(trimmed))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Apply the `escape_markdown`-dependent unescape pass to rendered output.
fn unescape_markdown_output(mut output: String, escape_markdown: bool) -> String {
    if escape_markdown {
        const UNESCAPE_TARGETS: &[char] = &['_', '[', ']', '(', ')', '*', '='];
        let cow = unescape_backslash_sequences(&output, UNESCAPE_TARGETS);
        if let Cow::Owned(s) = cow {
            output = s;
        }

        if output.contains("\\*") || output.contains("\\#") {
            output = output
                .lines()
                .map(|line| {
                    let trimmed = line.trim_start();
                    if trimmed.starts_with("\\* ") || trimmed.starts_with("\\#.") || trimmed.starts_with("\\#\\.") {
                        line.replacen("\\*", "*", 1).replacen("\\#", "#", 1)
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
        }
    } else {
        const UNESCAPE_TARGETS: &[char] = &['_', '[', ']', '(', ')', '*', '=', '-', '#'];
        let cow = unescape_backslash_sequences(&output, UNESCAPE_TARGETS);
        if let Cow::Owned(s) = cow {
            output = s;
        }
    }
    output
}

/// Append the `## Annotations` section to rendered output, when present.
fn append_annotations_section(mut output: String, annotations: Option<&[PdfAnnotation]>) -> String {
    if let Some(annotations) = annotations {
        let block = render_annotations_markdown(annotations);
        if !block.is_empty() {
            if !output.trim_end().is_empty() {
                output.push_str("\n\n");
            }
            output.push_str(&block);
        }
    }
    output
}

/// Render the document-level PDF annotations (issue #63) as a Markdown
/// section: a `## Annotations` heading followed by one bullet per annotation.
///
/// Returns an empty string when `annotations` is empty, so callers can append
/// unconditionally without introducing spurious blank sections.
fn render_annotations_markdown(annotations: &[PdfAnnotation]) -> String {
    if annotations.is_empty() {
        return String::new();
    }

    let mut out = String::from("## Annotations\n\n");
    for annotation in annotations {
        out.push_str("- **");
        out.push_str(annotation_type_label(annotation.annotation_type));
        out.push_str("** (page ");
        out.push_str(&annotation.page_number.to_string());
        out.push(')');

        if let Some(author) = annotation.author.as_deref().filter(|s| !s.is_empty()) {
            out.push_str(" by ");
            out.push_str(author);
        }

        if let Some(text) = annotation_display_text(annotation) {
            out.push_str(": ");
            out.push_str(text);
        }

        out.push('\n');
    }
    out
}

/// Build the final annotations-only output when the document body itself
/// renders empty (e.g. a scanned page whose only content is a highlight).
fn finalize_annotations_appendix(annotations: Option<&[PdfAnnotation]>) -> String {
    let mut block = annotations.map(render_annotations_markdown).unwrap_or_default();
    let trimmed_len = block.trim_end().len();
    if trimmed_len == 0 {
        return String::new();
    }
    block.truncate(trimmed_len);
    block.push('\n');
    block
}

/// Strip arXiv watermark noise from the document's prose.
///
/// LaTeX-generated PDFs often have a rotated sidebar with the arXiv identifier
/// that the PDF extractor concatenates with body text. This strips patterns like:
/// "Title N arXiv:NNNN.NNNNNvN [cat.SC] DD Mon YYYY" from the first pages.
/// Only `Text` nodes are searched, so a removal can neither reach into code nor
/// join two blocks.
fn strip_arxiv_watermark_noise<'a>(root: &'a AstNode<'a>) {
    let mut searched = 0;
    for node in root.descendants() {
        if searched >= 6000 {
            return;
        }
        let mut data = node.data.borrow_mut();
        let NodeValue::Text(text) = &mut data.value else {
            continue;
        };
        let search_limit = text.floor_char_boundary(text.len().min(6000 - searched));
        searched += text.len();
        let Some(m) = ARXIV_WATERMARK_REGEX.find(&text[..search_limit]) else {
            continue;
        };
        let range = m.range();
        let is_at_paragraph_boundary =
            text[..range.start].ends_with('.') || (range.end == text.len() && ends_its_block(node));
        if is_at_paragraph_boundary {
            tracing::trace!(
                stripped = %text[range.clone()].chars().take(80).collect::<String>(),
                "stripping arXiv watermark from markdown output"
            );
            text.to_mut().replace_range(range, "");
        }
        return;
    }
}

fn ends_its_block<'a>(node: &'a AstNode<'a>) -> bool {
    node.next_sibling().is_none()
        && node
            .parent()
            .is_none_or(|parent| parent.data.borrow().value.block() || ends_its_block(parent))
}

/// Shared comrak options with all GFM extensions enabled.
pub(crate) fn comrak_options<'a>() -> Options<'a> {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.footnotes = true;
    options.extension.description_lists = true;
    options.extension.math_dollars = true;
    // Parse-only: the DOCX and PDF writers then read `$`-adjacent code as math, as `render_markdown` writes it.
    options.extension.math_code = true;
    options.extension.underline = true;
    options.extension.subscript = true;
    options.extension.superscript = true;
    options.extension.highlight = true;
    options.extension.alerts = true;
    options.render.prefer_fenced = true;
    options
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::internal_builder::InternalDocumentBuilder;

    /// #### FAILS against unfixed code
    /// `InternalElement::set_list_item_source_label` does not exist on unfixed
    /// code, so this test does not compile without the fix -- the compatibility
    /// story an additive attribute (rather than a field on the `Copy`
    /// `ElementKind` enum) is built around. Once compiled, it proves the
    /// renderer half of the ordinal-discarding defect: a captured literal
    /// source marker ("B.", from Exhibit B of the ground-truth ordinance) must
    /// reach the rendered markdown verbatim, not get replaced by comrak's
    /// synthesized sequential "1.". This constructs the element directly and
    /// sets the label by hand because the PDF pipeline call site that would
    /// capture it (`pdf::structure::assembly::push_paragraph_element`) is
    /// outside this task's file scope -- see the task report for that half.
    #[cfg(feature = "pdf")]
    #[test]
    fn render_markdown_preserves_a_literal_list_item_source_label() {
        use crate::types::internal::{ElementKind, InternalElement};

        let mut doc = InternalDocument::new("pdf");
        doc.push_element(InternalElement::text(ElementKind::ListStart { ordered: true }, "", 0));
        let mut item = InternalElement::text(
            ElementKind::ListItem { ordered: true },
            "General Provisions, Definitions, and Exhibits.",
            1,
        );
        item.set_list_item_source_label("B.");
        doc.push_element(item);
        doc.push_element(InternalElement::text(ElementKind::ListEnd, "", 0));

        let rendered = render_markdown(&doc);

        // CommonMark's ordered marker is an auto-incrementing decimal and cannot express
        // a lettered label like "B.", so an item carrying a source label is rendered as a
        // bullet with the label as leading text. Two requirements, both non-negotiable:
        // the label survives (unfixed code drops it), and no synthesized ordinal appears
        // beside it (a "1." next to "B." renumbers a document cross-referenced by clause).
        assert!(
            rendered.contains("- B. General Provisions, Definitions, and Exhibits."),
            "expected the literal source label \"B.\" on a bullet, got: {rendered}"
        );
        assert!(
            !rendered.contains("1."),
            "a synthesized ordinal must not compete with the document's own label, got: {rendered}"
        );
    }

    /// Issue #1292: by default, prose containing a leading `#06-18`-style token
    /// keeps its backslash escapes so the markdown round-trips safely through a
    /// CommonMark parser. This must remain the behavior when `escape_markdown`
    /// is left at its default (`true`).
    #[test]
    fn render_markdown_default_escapes_leading_hash_and_dash_in_prose() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph("#06-18 widget replacement", vec![], None, None);
        let doc = b.build();
        assert!(doc.escape_markdown, "escape_markdown must default to true");

        let rendered = render_markdown(&doc);
        assert!(
            rendered.contains("\\#06-18"),
            "expected default rendering to keep the backslash escape: {rendered}"
        );
    }

    /// Issue #1292: a bare "- clause" paragraph must still render with its
    /// leading dash escaped by default, otherwise a CommonMark parser would
    /// reinterpret it as a list item.
    #[test]
    fn render_markdown_default_escapes_leading_dash_in_prose() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph("- clause applies here", vec![], None, None);
        let doc = b.build();

        let rendered = render_markdown(&doc);
        assert!(
            rendered.contains("\\- clause"),
            "expected default rendering to escape a leading dash: {rendered}"
        );
    }

    /// Issue #1292: `escape_markdown = false` strips the `#`/`-` escapes so prose
    /// reads identically to the (always-clean) table cell text.
    #[test]
    fn render_markdown_escape_markdown_false_yields_clean_prose() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph("#06-18 widget replacement", vec![], None, None);
        b.push_paragraph("- clause applies here", vec![], None, None);
        let mut doc = b.build();
        doc.escape_markdown = false;

        let rendered = render_markdown(&doc);
        assert!(
            rendered.contains("#06-18 widget replacement"),
            "expected clean, unescaped hash: {rendered}"
        );
        assert!(!rendered.contains("\\#06-18"), "escape must be stripped: {rendered}");
        assert!(
            rendered.contains("- clause applies here"),
            "expected clean, unescaped dash: {rendered}"
        );
        assert!(!rendered.contains("\\- clause"), "escape must be stripped: {rendered}");
    }

    /// Issue #1292: with `escape_markdown = false`, the same literal text rendered
    /// in prose and inside a table cell must be byte-identical, since table cells
    /// are never escaped.
    #[test]
    fn render_markdown_escape_markdown_false_matches_table_cell_rendering() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph("#06-18 widget replacement", vec![], None, None);
        b.push_table_from_cells(
            &[
                vec!["Part".to_string(), "Description".to_string()],
                vec!["#06-18".to_string(), "Widget".to_string()],
            ],
            None,
            None,
        );
        let mut doc = b.build();
        doc.escape_markdown = false;

        let rendered = render_markdown(&doc);
        assert!(
            rendered.contains("#06-18 widget replacement"),
            "prose must be clean: {rendered}"
        );
        assert!(
            rendered.contains("| #06-18 "),
            "table cell must remain clean and unescaped: {rendered}"
        );
        assert!(
            !rendered.contains("\\#06-18"),
            "no escaped variant should appear anywhere: {rendered}"
        );
    }

    /// Issue #1297: by default (`table_anchors = false`), rendered markdown
    /// contains no `[TABLE:...]` marker, so existing output stays unchanged.
    #[test]
    fn render_markdown_default_has_no_table_anchor() {
        let mut b = InternalDocumentBuilder::new("test");
        let idx = b.push_table_from_cells(&[vec!["A".to_string(), "B".to_string()]], None, None);
        let mut doc = b.build();
        doc.tables[idx as usize].table_id = Some("table-1".to_string());
        assert!(!doc.table_anchors, "table_anchors must default to false");

        let rendered = render_markdown(&doc);
        assert!(
            !rendered.contains("[TABLE:"),
            "no anchor should appear when table_anchors is disabled: {rendered}"
        );
    }

    /// Issue #1297: with `table_anchors = true`, rendered markdown contains a
    /// `[TABLE:{table_id}]` marker immediately before the table's markdown.
    #[test]
    fn render_markdown_table_anchors_enabled_emits_marker() {
        let mut b = InternalDocumentBuilder::new("test");
        let idx = b.push_table_from_cells(
            &[
                vec!["Name".to_string(), "Age".to_string()],
                vec!["Alice".to_string(), "30".to_string()],
            ],
            None,
            None,
        );
        let mut doc = b.build();
        doc.tables[idx as usize].table_id = Some("table-1".to_string());
        doc.table_anchors = true;

        let rendered = render_markdown(&doc);
        assert!(
            rendered.contains("[TABLE:table-1]"),
            "expected the table anchor marker: {rendered}"
        );

        let anchor_pos = rendered.find("[TABLE:table-1]").unwrap();
        let table_pos = rendered.find("| Name").unwrap();
        assert!(
            anchor_pos < table_pos,
            "anchor must precede the table markdown: {rendered}"
        );
    }

    /// Issue #1297: when a table has no `table_id`, no anchor is emitted even
    /// with `table_anchors = true` (nothing to reference).
    #[test]
    fn render_markdown_table_anchors_enabled_without_table_id_emits_no_marker() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_table_from_cells(&[vec!["A".to_string(), "B".to_string()]], None, None);
        let mut doc = b.build();
        doc.table_anchors = true;

        let rendered = render_markdown(&doc);
        assert!(
            !rendered.contains("[TABLE:"),
            "no anchor should appear without a table_id: {rendered}"
        );
    }

    #[test]
    fn render_markdown_strips_arxiv_watermark_by_default() {
        let mut builder = InternalDocumentBuilder::new("pdf");
        builder.push_paragraph(
            "Research title 7 arXiv:2401.12345v2 [cs.CL] 9 Jan 2024",
            vec![],
            None,
            None,
        );
        let document = builder.build();

        let rendered = render_markdown(&document);

        assert!(!rendered.contains("arXiv:2401.12345v2"));
    }

    #[test]
    fn render_markdown_preserves_arxiv_watermark_when_enabled() {
        let mut builder = InternalDocumentBuilder::new("pdf");
        builder.push_paragraph(
            "Research title 7 arXiv:2401.12345v2 [cs.CL] 9 Jan 2024",
            vec![],
            None,
            None,
        );
        let mut document = builder.build();
        document.include_watermarks = true;

        let rendered = render_markdown(&document);

        assert!(rendered.contains("arXiv:2401.12345v2 [cs.CL] 9 Jan 2024"));
    }

    #[test]
    fn unescape_backslash_sequences_empty_input_returns_borrowed() {
        let result = unescape_backslash_sequences("", &['_']);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, "");
    }

    #[test]
    fn unescape_backslash_sequences_no_targets_returns_borrowed() {
        let input = "hello world no escapes here";
        let result = unescape_backslash_sequences(input, &['_', '[', ']', '(', ')']);
        assert!(
            matches!(result, Cow::Borrowed(_)),
            "expected Cow::Borrowed when no target sequence present"
        );
        assert_eq!(result, input);
    }

    #[test]
    fn unescape_backslash_sequences_single_hit_returns_owned_correct_content() {
        let result = unescape_backslash_sequences("hello\\_world", &['_']);
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result, "hello_world");
    }

    #[test]
    fn unescape_backslash_sequences_multiple_targets_all_replaced() {
        let input = "\\[link\\](url\\) and \\[another\\]";
        let result = unescape_backslash_sequences(input, &['[', ']', '(', ')']);
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result, "[link](url) and [another]");
    }

    #[test]
    fn unescape_backslash_sequences_backslash_not_followed_by_target_is_kept() {
        let input = "foo\\nbar";
        let result = unescape_backslash_sequences(input, &['_']);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, "foo\\nbar");
    }

    #[test]
    fn unescape_backslash_sequences_roundtrip_vs_chained_replace() {
        let input = "a\\_b\\[c\\]d\\(e\\)f";
        let expected = input
            .replace("\\_", "_")
            .replace("\\[", "[")
            .replace("\\]", "]")
            .replace("\\(", "(")
            .replace("\\)", ")");
        let result = unescape_backslash_sequences(input, &['_', '[', ']', '(', ')']);
        assert_eq!(result.as_ref(), expected.as_str());
    }

    #[test]
    fn collapse_excess_newlines_empty_returns_borrowed() {
        let result = collapse_excess_newlines("");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn collapse_excess_newlines_no_triple_newline_returns_borrowed() {
        let input = "line1\n\nline2\n";
        let result = collapse_excess_newlines(input);
        assert!(
            matches!(result, Cow::Borrowed(_)),
            "expected Cow::Borrowed when no \\n\\n\\n present"
        );
        assert_eq!(result, input);
    }

    #[test]
    fn collapse_excess_newlines_triple_newline_collapsed_to_double() {
        let result = collapse_excess_newlines("a\n\n\nb");
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result, "a\n\nb");
    }

    #[test]
    fn collapse_excess_newlines_many_newlines_collapsed() {
        let result = collapse_excess_newlines("a\n\n\n\n\n\nb");
        assert_eq!(result, "a\n\nb");
    }

    #[test]
    fn collapse_excess_newlines_equivalent_to_while_replace() {
        let input = "p1\n\n\n\np2\n\n\np3\n\n";
        let mut expected = input.to_string();
        while expected.contains("\n\n\n") {
            expected = expected.replace("\n\n\n", "\n\n");
        }
        let result = collapse_excess_newlines(input);
        assert_eq!(result.as_ref(), expected.as_str());
    }
}
