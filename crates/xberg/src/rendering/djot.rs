//! Render an `InternalDocument` to Djot markup.

use crate::types::annotations::PdfAnnotation;
use crate::types::document_structure::AnnotationKind;
use crate::types::internal::{ElementKind, InternalDocument};

use super::common::{
    FootnoteCollector, NestingKind, RenderState, annotated_math, annotation_display_text, annotation_type_label,
    ensure_trailing_newline, finalize_output, get_admonition_kind, get_admonition_title, get_language,
    handle_container_end, is_body_element, is_container_end, normalize_inline_text, parse_metadata_entries,
    push_with_bq, render_annotated_text_with_plain, render_nested_annotated_text, render_table_djot,
    valid_math_annotation,
};

/// Render an `InternalDocument` to Djot markup.
pub(crate) fn render_djot(doc: &InternalDocument) -> String {
    let footnotes = FootnoteCollector::new(doc);
    let mut state = RenderState::default();
    let mut out = String::with_capacity(doc.elements.len() * 80);
    // Where the text of the last block written ends, while that block can take an inline
    // footnote marker. A `FootnoteRef` follows the block that cites it.
    let mut inline_block_end: Option<usize> = None;

    for (i, elem) in doc.elements.iter().enumerate() {
        if !is_body_element(elem) {
            continue;
        }

        if is_container_end(elem) {
            handle_container_end(&elem.kind, &mut state);
            continue;
        }

        state.pop_to_depth(elem.depth);

        let bq_depth = state.blockquote_depth();

        match elem.kind {
            ElementKind::Title => {
                let text = render_djot_annotated(&elem.text, &elem.annotations);
                let block = format!("# {}\n\n", text);
                push_with_bq(&mut out, &block, bq_depth);
            }
            ElementKind::Heading { level } => {
                let hashes = "#".repeat(level as usize);
                let text = render_djot_annotated(&elem.text, &elem.annotations);
                let block = format!("{} {}\n\n", hashes, text);
                push_with_bq(&mut out, &block, bq_depth);
            }
            ElementKind::Paragraph => {
                let text = render_djot_annotated(&elem.text, &elem.annotations);
                let block = format!("{}\n\n", text);
                push_with_bq(&mut out, &block, bq_depth);
            }
            ElementKind::ListItem { ordered } => {
                let list_depth = state.list_depth();
                let indent = "  ".repeat(list_depth.saturating_sub(1));
                let text = render_djot_annotated(&elem.text, &elem.annotations);
                let source_label = elem.list_item_source_label().filter(|label| !label.is_empty());
                let mut block = String::with_capacity(indent.len() + text.len() + 8);
                block.push_str(&indent);
                // A literal source marker (e.g. "B.", "(a)") cannot be expressed by Djot's
                // auto-incrementing decimal marker syntax either, so it falls back to a
                // bullet with the label written out as visible text, same trade-off as the
                // markdown renderer (comrak_bridge) makes. `next_list_number` is still
                // called (and its result discarded) so a sibling item without a label keeps
                // counting from the position it would have held either way.
                if ordered {
                    let n = state.next_list_number();
                    match source_label {
                        Some(label) => {
                            block.push_str("- ");
                            block.push_str(label);
                            block.push(' ');
                        }
                        None => {
                            block.push_str(&n.to_string());
                            block.push_str(". ");
                        }
                    }
                } else if let Some(label) = source_label {
                    block.push_str("- ");
                    block.push_str(label);
                    block.push(' ');
                } else {
                    block.push_str("- ");
                };
                block.push_str(&text);
                block.push('\n');
                push_with_bq(&mut out, &block, bq_depth);
            }
            ElementKind::Code => {
                let lang = get_language(elem).unwrap_or("");
                let mut block = format!("```{}\n{}", lang, elem.text);
                if !elem.text.ends_with('\n') {
                    block.push('\n');
                }
                block.push_str("```\n\n");
                push_with_bq(&mut out, &block, bq_depth);
            }
            ElementKind::Formula => {
                let mut block = format!("$$\n{}", elem.text);
                if !elem.text.ends_with('\n') {
                    block.push('\n');
                }
                block.push_str("$$\n\n");
                push_with_bq(&mut out, &block, bq_depth);
            }
            ElementKind::Table { table_index } => {
                if let Some(table) = doc.tables.get(table_index as usize) {
                    if doc.table_anchors
                        && let Some(table_id) = table.table_id.as_deref()
                    {
                        let anchor = format!("[TABLE:{table_id}]\n\n");
                        push_with_bq(&mut out, &anchor, bq_depth);
                    }
                    let table_str = if !table.cells.is_empty() {
                        render_table_djot(&table.cells)
                    } else {
                        table.markdown.clone()
                    };
                    if !table_str.trim().is_empty() {
                        let block = format!("{}\n", table_str);
                        push_with_bq(&mut out, &block, bq_depth);
                    }
                }
            }
            ElementKind::Image { image_index } => {
                let image = doc.images.get(image_index as usize);
                let desc = image
                    .and_then(|img| img.description.as_deref())
                    .unwrap_or(elem.text.as_str());

                if image.is_none() && desc.is_empty() {
                    continue;
                }

                let url = image
                    .and_then(|img| {
                        if !img.data.is_empty() {
                            Some(format!("image_{}.{}", image_index, img.format))
                        } else {
                            img.source_path.clone()
                        }
                    })
                    .unwrap_or_default();
                let block = format!("![{}]({})\n\n", desc, url);
                push_with_bq(&mut out, &block, bq_depth);

                if elem.should_render_image_ocr()
                    && let Some(ocr_result) = image.and_then(|img| img.ocr_result.as_ref())
                    && !ocr_result.content.is_empty()
                {
                    let block = format!("{}\n\n", ocr_result.content);
                    push_with_bq(&mut out, &block, bq_depth);
                }
            }
            ElementKind::FootnoteRef => {
                if let Some(n) = footnotes.ref_number(i as u32) {
                    let marker = format!("[^{n}]");
                    // Appended to the end of `out`, the marker started the next block instead.
                    match inline_block_end {
                        Some(end) => {
                            out.insert_str(end, &marker);
                            inline_block_end = Some(end + marker.len());
                        }
                        None => {
                            push_with_bq(&mut out, &format!("{marker}\n\n"), bq_depth);
                            inline_block_end = Some(block_text_end(&out, bq_depth));
                        }
                    }
                }
            }
            ElementKind::FootnoteDefinition => {}
            ElementKind::CommentRef => {}
            ElementKind::CommentDefinition => {}
            ElementKind::Citation => {}
            ElementKind::PageBreak => {}
            ElementKind::Slide { number: _ } => {
                if elem.text.is_empty() {
                    push_with_bq(&mut out, "\n---\n\n", bq_depth);
                } else {
                    let text = render_djot_annotated(&elem.text, &elem.annotations);
                    let mut block = String::with_capacity(12 + text.len());
                    block.push_str("\n---\n\n## ");
                    block.push_str(&text);
                    block.push_str("\n\n");
                    push_with_bq(&mut out, &block, bq_depth);
                }
            }
            ElementKind::DefinitionTerm => {
                let text = render_djot_annotated(&elem.text, &elem.annotations);
                let block = format!("{}\n", text);
                push_with_bq(&mut out, &block, bq_depth);
            }
            ElementKind::DefinitionDescription => {
                let text = render_djot_annotated(&elem.text, &elem.annotations);
                let block = format!(": {}\n\n", text);
                push_with_bq(&mut out, &block, bq_depth);
            }
            ElementKind::Admonition => {
                let kind = get_admonition_kind(elem);
                let title = get_admonition_title(elem);
                let text = render_djot_annotated(&elem.text, &elem.annotations);

                let mut block = String::with_capacity(kind.len() + text.len() + 32);
                block.push_str("::: ");
                block.push_str(kind);
                block.push('\n');
                if let Some(t) = title {
                    block.push_str("**");
                    block.push_str(t);
                    block.push_str("**\n\n");
                }
                if !text.is_empty() {
                    block.push_str(&text);
                    block.push('\n');
                }
                block.push_str(":::\n\n");
                push_with_bq(&mut out, &block, bq_depth);
            }
            ElementKind::RawBlock => {
                let mut block = elem.text.clone();
                ensure_trailing_newline(&mut block);
                block.push('\n');
                push_with_bq(&mut out, &block, bq_depth);
            }
            ElementKind::MetadataBlock => {
                let entries = parse_metadata_entries(&elem.text);
                let mut block = String::new();
                for (key, value) in &entries {
                    block.push('*');
                    block.push_str(key);
                    block.push_str("*: ");
                    block.push_str(value);
                    block.push('\n');
                }
                if entries.is_empty() && !elem.text.is_empty() {
                    block.push_str(&elem.text);
                    ensure_trailing_newline(&mut block);
                }
                block.push('\n');
                push_with_bq(&mut out, &block, bq_depth);
            }
            ElementKind::OcrText { .. } => {
                let text = render_djot_annotated(&elem.text, &elem.annotations);
                let block = format!("{}\n\n", text);
                push_with_bq(&mut out, &block, bq_depth);
            }
            ElementKind::ListStart { ordered } => {
                state.push_container(NestingKind::List { ordered, item_count: 0 }, elem.depth);
            }
            ElementKind::ListEnd => {}
            ElementKind::QuoteStart => {
                state.push_container(NestingKind::BlockQuote, elem.depth);
            }
            ElementKind::QuoteEnd => {}
            ElementKind::GroupStart => {
                state.push_container(NestingKind::Group, elem.depth);
            }
            ElementKind::GroupEnd => {}
        }

        inline_block_end = match elem.kind {
            ElementKind::Title
            | ElementKind::Heading { .. }
            | ElementKind::Paragraph
            | ElementKind::ListItem { .. }
            | ElementKind::DefinitionTerm
            | ElementKind::DefinitionDescription
            | ElementKind::OcrText { .. }
                if !elem.text.is_empty() =>
            {
                Some(block_text_end(&out, bq_depth))
            }
            // Nothing of these is written in the flow, so the last block stays the last one.
            ElementKind::FootnoteRef
            | ElementKind::FootnoteDefinition
            | ElementKind::CommentRef
            | ElementKind::CommentDefinition
            | ElementKind::Citation
            | ElementKind::PageBreak => inline_block_end,
            _ => None,
        };
    }

    let defs = footnotes.definitions();
    if !defs.is_empty() {
        out.push('\n');
        for entry in defs {
            out.push_str("[^");
            out.push_str(&entry.number.to_string());
            out.push_str("]: ");
            out.push_str(&entry.text);
            out.push_str("\n\n");
        }
    }

    for elem in &doc.elements {
        if elem.kind == ElementKind::Citation {
            let key = elem.anchor.as_deref().unwrap_or("?");
            out.push_str("[^");
            out.push_str(key);
            out.push_str("]: ");
            out.push_str(&elem.text);
            out.push_str("\n\n");
        }
    }

    // Comment definitions (#300) aren't tracked by `FootnoteCollector`, so surface
    // them the same way `Citation` is above — keyed by anchor rather than assigned
    // a sequential number — instead of silently dropping the comment body.
    for elem in &doc.elements {
        if elem.kind == ElementKind::CommentDefinition {
            let key = elem.anchor.as_deref().unwrap_or("?");
            out.push_str("[^");
            out.push_str(key);
            out.push_str("]: ");
            out.push_str(&elem.text);
            out.push_str("\n\n");
        }
    }

    if let Some(annotations) = doc.annotations.as_deref() {
        let block = render_annotations_djot(annotations);
        if !block.is_empty() {
            if !out.trim_end().is_empty() {
                out.push('\n');
            }
            out.push_str(&block);
        }
    }

    finalize_output(out)
}

/// Render the document-level PDF annotations (issue #63) as a Djot section:
/// a `## Annotations` heading followed by one bullet per annotation.
///
/// Returns an empty string when `annotations` is empty.
fn render_annotations_djot(annotations: &[PdfAnnotation]) -> String {
    if annotations.is_empty() {
        return String::new();
    }

    let mut out = String::from("## Annotations\n\n");
    for annotation in annotations {
        out.push_str("- _");
        out.push_str(annotation_type_label(annotation.annotation_type));
        out.push_str("_ (page ");
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
    out.push('\n');
    out
}

/// Render text with djot inline annotations, normalizing inline text.
///
/// Plain text segments and annotated spans are normalized (whitespace collapsed,
/// newlines replaced, control characters stripped) to match the markdown renderer.
/// Code spans are left un-normalized to preserve literal content.
/// Byte offset just past the text of the last block in `out`: before the newlines that close
/// it and, inside a blockquote, before the quoted blank line (`> `) that follows it.
fn block_text_end(out: &str, bq_depth: usize) -> usize {
    let mut end = out.trim_end_matches('\n').len();
    if bq_depth == 0 {
        return end;
    }
    let prefix = "> ".repeat(bq_depth);
    loop {
        let line_start = out[..end].rfind('\n').map_or(0, |i| i + 1);
        if out[line_start..end] != prefix {
            return end;
        }
        end = out[..line_start].trim_end_matches('\n').len();
    }
}

fn render_djot_annotated(text: &str, annotations: &[crate::types::document_structure::TextAnnotation]) -> String {
    if annotations
        .iter()
        .any(|annotation| valid_math_annotation(text, annotation))
    {
        return render_nested_annotated_text(
            text,
            annotations,
            &|span, kind| {
                if let Some((literal, display)) = annotated_math(span, kind) {
                    Some(render_djot_math(literal, display))
                } else if matches!(kind, AnnotationKind::Code) {
                    Some(render_djot_span(span, kind, false))
                } else {
                    None
                }
            },
            &|span, kind| render_djot_span(span, kind, false),
            &|span| normalize_inline_text(span).into_owned(),
        );
    }
    render_annotated_text_with_plain(
        text,
        annotations,
        |span, kind| render_djot_span(span, kind, true),
        |span| normalize_inline_text(span).into_owned(),
    )
}

fn render_djot_span(span: &str, kind: &AnnotationKind, normalize: bool) -> String {
    let normalized = if normalize {
        normalize_inline_text(span)
    } else {
        std::borrow::Cow::Borrowed(span)
    };
    match kind {
        AnnotationKind::Bold => format!("*{}*", normalized),
        AnnotationKind::Italic => format!("_{}_", normalized),
        AnnotationKind::Code => format!("`{}`", span),
        AnnotationKind::Strikethrough => format!("{{-{}-}}", normalized),
        AnnotationKind::Underline => format!("[{}]{{.underline}}", normalized),
        AnnotationKind::Subscript => format!("~{}~", normalized),
        AnnotationKind::Superscript => format!("^{}^", normalized),
        AnnotationKind::Highlight => format!("{{={}=}}", normalized),
        AnnotationKind::Link { url, title } => {
            if let Some(title) = title {
                format!("[{}]({} \"{}\")", normalized, url, title)
            } else {
                format!("[{}]({})", normalized, url)
            }
        }
        AnnotationKind::Color { .. } | AnnotationKind::FontSize { .. } | AnnotationKind::Custom { .. } => {
            normalized.into_owned()
        }
    }
}

fn render_djot_math(literal: &str, display: bool) -> String {
    let width = literal
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0)
        + 1;
    let fence = "`".repeat(width);
    let before = if literal.starts_with('`') { " " } else { "" };
    let after = if literal.ends_with('`') { " " } else { "" };
    let dollar = if display { "$$" } else { "$" };
    format!("{dollar}{fence}{before}{literal}{after}{fence}")
}

#[cfg(test)]
mod tests;
