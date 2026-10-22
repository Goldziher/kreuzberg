//! Markdown extractor with YAML frontmatter support.
//!
//! This extractor provides:
//! - Comprehensive markdown parsing using pulldown-cmark
//! - Complete YAML frontmatter metadata extraction:
//!   - Standard fields: title, author, date, description, keywords
//!   - Extended fields: abstract, subject, category, tags, language, version
//! - Automatic conversion of array fields (keywords, tags) to comma-separated strings
//! - Table extraction as structured data
//! - Heading structure preservation
//! - Code block and link extraction
//! - Data URI image extraction
//!
use super::annotation_utils::adjust_annotations_for_trim;
use super::frontmatter_utils::{
    extract_frontmatter_with_warning, extract_metadata_from_yaml, extract_title_from_content,
};
use super::markdown_utils::{FootnoteRefHolder, PendingFootnoteRefs};
use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::security::SecurityBudget;
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::uri::{ExtractedUri, UriKind, classify_uri};
use crate::types::{Metadata, Table};
use async_trait::async_trait;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
#[cfg(not(feature = "notebook"))]
use std::borrow::Cow;

/// Annotation tracking entry: (kind_tag, byte_start, optional link data).
///
/// kind_tag: 0=bold, 1=italic, 2=strikethrough, 3=code, 4=link, 5=superscript, 6=subscript
type AnnotationEntry = (u8, u32, Option<(String, Option<String>)>);

/// Full pulldown-cmark option set used for every Markdown parse.
///
/// Enables the complete parser feature surface (pandoc/GFM/Quarto supersets): tables,
/// footnotes, strikethrough, task lists, smart punctuation, heading attributes, math,
/// GFM alerts, definition lists, super/subscript, and wikilinks. The metadata-block
/// options are intentionally omitted — YAML/`+++` frontmatter is stripped up-front by
/// [`extract_frontmatter`], so those flags would never fire and would only double-parse.
/// `ENABLE_OLD_FOOTNOTES` is also omitted (deprecated Hoedown semantics that conflict
/// with `ENABLE_FOOTNOTES`).
pub(crate) fn markdown_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_SMART_PUNCTUATION
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_MATH
        | Options::ENABLE_GFM
        | Options::ENABLE_DEFINITION_LIST
        | Options::ENABLE_SUPERSCRIPT
        | Options::ENABLE_SUBSCRIPT
        | Options::ENABLE_WIKILINKS
}

/// Normalize a fenced code-block info string into a bare language token.
///
/// Quarto and R Markdown executable cells use a braced info string that also carries
/// chunk options, e.g. `` ```{python} `` or `` ```{r, echo=FALSE} ``. Strip the braces
/// and any trailing options so the emitted language is the bare kernel name (`python`,
/// `r`). Ordinary fences (`rust`, `python`) pass through unchanged. Returns `None` for an
/// empty or attribute-only info string.
pub(crate) fn normalize_fence_lang(info: &str) -> Option<String> {
    let info = info.trim();
    if info.is_empty() {
        return None;
    }
    let inner = info
        .strip_prefix('{')
        .map_or(info, |rest| rest.strip_suffix('}').unwrap_or(rest));
    let lang = inner
        .split([',', ' ', '\t'])
        .next()
        .unwrap_or("")
        .trim()
        .trim_start_matches('.');
    if lang.is_empty() { None } else { Some(lang.to_string()) }
}

fn classify_markdown_uri(url: &str) -> UriKind {
    if url.starts_with("cite:") {
        UriKind::Citation
    } else {
        classify_uri(url)
    }
}

/// Markdown extractor with metadata and table support.
///
/// Parses markdown documents with YAML frontmatter, extracting:
/// - Metadata from YAML frontmatter
/// - Plain text content
/// - Tables as structured data
/// - Document structure (headings, links, code blocks)
/// - Images from data URIs
#[cfg_attr(alef, alef(skip))]
pub struct MarkdownExtractor;

impl MarkdownExtractor {
    /// Create a new Markdown extractor.
    pub fn new() -> Self {
        Self
    }

    /// Build an `InternalDocument` from pulldown-cmark events and optional YAML frontmatter.
    ///
    /// Kept as a 2-argument function for existing callers (e.g. the Jupyter notebook
    /// extractor's Markdown cell rendering) that have no JSX blocks to record. See
    /// [`Self::build_internal_document_with_jsx`] for the MDX entry point.
    pub(crate) fn build_internal_document(events: &[Event], yaml: &Option<yaml_serde::Value>) -> InternalDocument {
        Self::build_internal_document_with_jsx(events, yaml, &[])
    }

    /// Build an `InternalDocument` from pulldown-cmark events and optional YAML frontmatter.
    ///
    /// This is the single shared event-stream builder for both the Markdown and MDX
    /// extractors (see issue #273: the two used to be a ~470-line copy-paste fork that
    /// drifted, silently dropping math/inline-HTML/superscript/subscript/definition-list
    /// support in `.mdx` files). `raw_jsx_blocks` carries MDX-specific stripped JSX
    /// fragments to be recorded as raw blocks; pass an empty slice for plain Markdown
    /// (that's what [`Self::build_internal_document`] does).
    pub(crate) fn build_internal_document_with_jsx(
        events: &[Event],
        yaml: &Option<yaml_serde::Value>,
        raw_jsx_blocks: &[String],
    ) -> InternalDocument {
        use crate::types::builder;
        use crate::types::document_structure::TextAnnotation;
        let mut b = InternalDocumentBuilder::new("markdown");

        if let Some(yaml_serde::Value::Mapping(map)) = yaml {
            let entries: Vec<(String, String)> = map
                .iter()
                .filter_map(|(k, v)| {
                    let key = k.as_str()?.to_string();
                    let val = match v {
                        yaml_serde::Value::String(s) => s.clone(),
                        other => format!("{other:?}"),
                    };
                    Some((key, val))
                })
                .collect();
            if !entries.is_empty() {
                b.push_metadata_block(&entries, None);
            }
        }

        for jsx in raw_jsx_blocks {
            if !jsx.trim().is_empty() {
                b.push_raw_block("jsx", jsx, None);
            }
        }

        let mut paragraph_text = String::new();
        let mut paragraph_annotations: Vec<TextAnnotation> = Vec::new();
        let mut in_paragraph = false;
        let mut heading_text = String::new();
        let mut heading_annotations: Vec<TextAnnotation> = Vec::new();
        let mut heading_level: u8 = 0;
        let mut in_heading = false;
        let mut code_text = String::new();
        let mut code_lang: Option<String> = None;
        let mut in_code_block = false;
        let mut table_rows: Vec<Vec<String>> = Vec::new();
        let mut current_row: Vec<String> = Vec::new();
        let mut current_cell = String::new();
        let mut in_table_cell = false;
        let mut list_stack: Vec<bool> = Vec::new();
        let mut list_item_text = String::new();
        let mut list_item_annotations: Vec<TextAnnotation> = Vec::new();
        // Depth counter, not a bool: a sublist nested inside a list item closes its own
        // `Item` before the enclosing item closes, and trailing text after the sublist
        // still belongs to the enclosing item. A bool cleared to `false` by the inner
        // `End(Item)` let that trailing text fall through to the paragraph guard below
        // and be emitted as a bare paragraph instead of list-item content (GH#1459).
        let mut in_list_item: usize = 0;
        let mut in_image = false;
        let mut image_alt = String::new();
        let mut image_url: Option<String> = None;
        let mut image_counter: u32 = 0;
        let mut footnote_def_label: Option<String> = None;
        let mut footnote_def_text = String::new();
        let mut in_def_title = false;
        let mut in_def_desc = false;
        let mut def_buf = String::new();
        let mut blockquote_stack: Vec<Option<u32>> = Vec::new();
        let mut pending_anchor: Option<String> = None;
        let mut pending_footnote_refs = PendingFootnoteRefs::default();

        let mut annotation_starts: Vec<AnnotationEntry> = Vec::new();

        /// Get the current length of the active text buffer as u32.
        fn active_text_offset(buf: &str) -> u32 {
            buf.len() as u32
        }

        fn apply_pending_anchor(
            builder: &mut InternalDocumentBuilder,
            pending_anchor: &mut Option<String>,
            index: u32,
        ) {
            if let Some(anchor) = pending_anchor.take() {
                builder.set_anchor(index, anchor);
            }
        }

        // Flush the text of the open list item. A block inside the item (a sublist, a code block,
        // a table, a heading or a display formula) is pushed as it starts or ends, while the
        // item's own text waits for `End(Item)`, so without this flush the block would land
        // before the text of the item that holds it. See GH#1459. ~keep
        macro_rules! flush_list_item_text {
            () => {
                if in_list_item > 0 {
                    let trimmed = list_item_text.trim();
                    if let Some(ordered) = list_stack.last().copied()
                        && !trimmed.is_empty()
                    {
                        let annotations = adjust_annotations_for_trim(
                            std::mem::take(&mut list_item_annotations),
                            &list_item_text,
                            trimmed,
                        );
                        b.push_list_item(trimmed, ordered, annotations, None, None);
                    }
                    pending_footnote_refs.flush(&mut b, FootnoteRefHolder::ListItem(in_list_item));
                    list_item_text.clear();
                    list_item_annotations.clear();
                    annotation_starts.clear();
                }
            };
        }

        for event in events {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    flush_list_item_text!();
                    heading_text.clear();
                    heading_annotations.clear();
                    annotation_starts.clear();
                    heading_level = match *level {
                        pulldown_cmark::HeadingLevel::H1 => 1,
                        pulldown_cmark::HeadingLevel::H2 => 2,
                        pulldown_cmark::HeadingLevel::H3 => 3,
                        pulldown_cmark::HeadingLevel::H4 => 4,
                        pulldown_cmark::HeadingLevel::H5 => 5,
                        pulldown_cmark::HeadingLevel::H6 => 6,
                    };
                    in_heading = true;
                }
                Event::End(TagEnd::Heading(_)) => {
                    in_heading = false;
                    let trimmed = heading_text.trim();
                    if !trimmed.is_empty() {
                        let annotations = adjust_annotations_for_trim(
                            std::mem::take(&mut heading_annotations),
                            &heading_text,
                            trimmed,
                        );
                        let idx = b.push_heading(heading_level, trimmed, None, None);
                        apply_pending_anchor(&mut b, &mut pending_anchor, idx);
                        if !annotations.is_empty() {
                            b.set_annotations(idx, annotations);
                        }
                    }
                    pending_footnote_refs.flush(&mut b, FootnoteRefHolder::Heading);
                    heading_text.clear();
                    heading_annotations.clear();
                }
                Event::Start(Tag::Paragraph) if in_list_item > 0 && !list_item_text.is_empty() => {
                    list_item_text.push_str("\n\n");
                }
                Event::Start(Tag::Paragraph)
                    if !in_heading && in_list_item == 0 && footnote_def_label.is_none() && !in_def_desc =>
                {
                    paragraph_text.clear();
                    paragraph_annotations.clear();
                    in_paragraph = true;
                }
                Event::End(TagEnd::Paragraph) if in_paragraph => {
                    in_paragraph = false;
                    let trimmed = paragraph_text.trim();
                    if !trimmed.is_empty() {
                        let annotations = adjust_annotations_for_trim(
                            std::mem::take(&mut paragraph_annotations),
                            &paragraph_text,
                            trimmed,
                        );
                        let idx = b.push_paragraph(trimmed, annotations, None, None);
                        apply_pending_anchor(&mut b, &mut pending_anchor, idx);
                    }
                    pending_footnote_refs.flush(&mut b, FootnoteRefHolder::Paragraph);
                    paragraph_text.clear();
                    paragraph_annotations.clear();
                }
                Event::Start(Tag::Strong) => {
                    if in_paragraph {
                        annotation_starts.push((0, active_text_offset(&paragraph_text), None));
                    } else if in_heading {
                        annotation_starts.push((0, active_text_offset(&heading_text), None));
                    } else if in_list_item > 0 {
                        annotation_starts.push((0, active_text_offset(&list_item_text), None));
                    }
                }
                Event::End(TagEnd::Strong) => {
                    if let Some(i) = annotation_starts.iter().rposition(|(k, _, _)| *k == 0) {
                        let (_, start, _) = annotation_starts.remove(i);
                        if in_paragraph {
                            let end = active_text_offset(&paragraph_text);
                            if start < end {
                                paragraph_annotations.push(builder::bold(start, end));
                            }
                        } else if in_heading {
                            let end = active_text_offset(&heading_text);
                            if start < end {
                                heading_annotations.push(builder::bold(start, end));
                            }
                        } else if in_list_item > 0 {
                            let end = active_text_offset(&list_item_text);
                            if start < end {
                                list_item_annotations.push(builder::bold(start, end));
                            }
                        }
                    }
                }
                Event::Start(Tag::Emphasis) => {
                    if in_paragraph {
                        annotation_starts.push((1, active_text_offset(&paragraph_text), None));
                    } else if in_heading {
                        annotation_starts.push((1, active_text_offset(&heading_text), None));
                    } else if in_list_item > 0 {
                        annotation_starts.push((1, active_text_offset(&list_item_text), None));
                    }
                }
                Event::End(TagEnd::Emphasis) => {
                    if let Some(i) = annotation_starts.iter().rposition(|(k, _, _)| *k == 1) {
                        let (_, start, _) = annotation_starts.remove(i);
                        if in_paragraph {
                            let end = active_text_offset(&paragraph_text);
                            if start < end {
                                paragraph_annotations.push(builder::italic(start, end));
                            }
                        } else if in_heading {
                            let end = active_text_offset(&heading_text);
                            if start < end {
                                heading_annotations.push(builder::italic(start, end));
                            }
                        } else if in_list_item > 0 {
                            let end = active_text_offset(&list_item_text);
                            if start < end {
                                list_item_annotations.push(builder::italic(start, end));
                            }
                        }
                    }
                }
                Event::Start(Tag::Strikethrough) => {
                    if in_paragraph {
                        annotation_starts.push((2, active_text_offset(&paragraph_text), None));
                    } else if in_heading {
                        annotation_starts.push((2, active_text_offset(&heading_text), None));
                    } else if in_list_item > 0 {
                        annotation_starts.push((2, active_text_offset(&list_item_text), None));
                    }
                }
                Event::End(TagEnd::Strikethrough) => {
                    if let Some(i) = annotation_starts.iter().rposition(|(k, _, _)| *k == 2) {
                        let (_, start, _) = annotation_starts.remove(i);
                        if in_paragraph {
                            let end = active_text_offset(&paragraph_text);
                            if start < end {
                                paragraph_annotations.push(builder::strikethrough(start, end));
                            }
                        } else if in_heading {
                            let end = active_text_offset(&heading_text);
                            if start < end {
                                heading_annotations.push(builder::strikethrough(start, end));
                            }
                        } else if in_list_item > 0 {
                            let end = active_text_offset(&list_item_text);
                            if start < end {
                                list_item_annotations.push(builder::strikethrough(start, end));
                            }
                        }
                    }
                }
                Event::Start(Tag::Superscript) => {
                    if in_paragraph {
                        annotation_starts.push((5, active_text_offset(&paragraph_text), None));
                    } else if in_heading {
                        annotation_starts.push((5, active_text_offset(&heading_text), None));
                    } else if in_list_item > 0 {
                        annotation_starts.push((5, active_text_offset(&list_item_text), None));
                    }
                }
                Event::End(TagEnd::Superscript) => {
                    if let Some(i) = annotation_starts.iter().rposition(|(k, _, _)| *k == 5) {
                        let (_, start, _) = annotation_starts.remove(i);
                        if in_paragraph {
                            let end = active_text_offset(&paragraph_text);
                            if start < end {
                                paragraph_annotations.push(builder::superscript(start, end));
                            }
                        } else if in_heading {
                            let end = active_text_offset(&heading_text);
                            if start < end {
                                heading_annotations.push(builder::superscript(start, end));
                            }
                        } else if in_list_item > 0 {
                            let end = active_text_offset(&list_item_text);
                            if start < end {
                                list_item_annotations.push(builder::superscript(start, end));
                            }
                        }
                    }
                }
                Event::Start(Tag::Subscript) => {
                    if in_paragraph {
                        annotation_starts.push((6, active_text_offset(&paragraph_text), None));
                    } else if in_heading {
                        annotation_starts.push((6, active_text_offset(&heading_text), None));
                    } else if in_list_item > 0 {
                        annotation_starts.push((6, active_text_offset(&list_item_text), None));
                    }
                }
                Event::End(TagEnd::Subscript) => {
                    if let Some(i) = annotation_starts.iter().rposition(|(k, _, _)| *k == 6) {
                        let (_, start, _) = annotation_starts.remove(i);
                        if in_paragraph {
                            let end = active_text_offset(&paragraph_text);
                            if start < end {
                                paragraph_annotations.push(builder::subscript(start, end));
                            }
                        } else if in_heading {
                            let end = active_text_offset(&heading_text);
                            if start < end {
                                heading_annotations.push(builder::subscript(start, end));
                            }
                        } else if in_list_item > 0 {
                            let end = active_text_offset(&list_item_text);
                            if start < end {
                                list_item_annotations.push(builder::subscript(start, end));
                            }
                        }
                    }
                }
                Event::Start(Tag::Link { dest_url, title, .. }) => {
                    let url = dest_url.to_string();
                    let title_opt = if title.is_empty() {
                        None
                    } else {
                        Some(title.to_string())
                    };
                    if in_paragraph {
                        annotation_starts.push((4, active_text_offset(&paragraph_text), Some((url, title_opt))));
                    } else if in_heading {
                        annotation_starts.push((4, active_text_offset(&heading_text), Some((url, title_opt))));
                    } else if in_list_item > 0 {
                        annotation_starts.push((4, active_text_offset(&list_item_text), Some((url, title_opt))));
                    }
                }
                Event::End(TagEnd::Link) => {
                    if let Some(i) = annotation_starts.iter().rposition(|(k, _, _)| *k == 4) {
                        let (_, start, link_data) = annotation_starts.remove(i);
                        if let Some((url, title)) = link_data {
                            let label_text = if in_paragraph {
                                let end = active_text_offset(&paragraph_text);
                                if start < end {
                                    paragraph_annotations.push(builder::link(start, end, &url, title.as_deref()));
                                    Some(paragraph_text[start as usize..end as usize].to_string())
                                } else {
                                    None
                                }
                            } else if in_heading {
                                let end = active_text_offset(&heading_text);
                                if start < end {
                                    heading_annotations.push(builder::link(start, end, &url, title.as_deref()));
                                    Some(heading_text[start as usize..end as usize].to_string())
                                } else {
                                    None
                                }
                            } else if in_list_item > 0 {
                                let end = active_text_offset(&list_item_text);
                                if start < end {
                                    list_item_annotations.push(builder::link(start, end, &url, title.as_deref()));
                                    Some(list_item_text[start as usize..end as usize].to_string())
                                } else {
                                    None
                                }
                            } else {
                                None
                            };
                            if !url.is_empty() {
                                let kind = classify_markdown_uri(&url);
                                b.push_uri(ExtractedUri {
                                    url,
                                    label: label_text.filter(|s| !s.is_empty()),
                                    page: None,
                                    kind,
                                });
                            }
                        }
                    }
                }
                Event::Start(Tag::CodeBlock(pulldown_cmark::CodeBlockKind::Fenced(lang))) => {
                    flush_list_item_text!();
                    code_text.clear();
                    code_lang = normalize_fence_lang(lang);
                    in_code_block = true;
                }
                Event::Start(Tag::CodeBlock(_)) => {
                    flush_list_item_text!();
                    code_text.clear();
                    code_lang = None;
                    in_code_block = true;
                }
                Event::End(TagEnd::CodeBlock) => {
                    in_code_block = false;
                    let trimmed = code_text.trim_end();
                    if !trimmed.is_empty() {
                        let idx = b.push_code(trimmed, code_lang.as_deref(), None, None);
                        apply_pending_anchor(&mut b, &mut pending_anchor, idx);
                    }
                    code_text.clear();
                    code_lang = None;
                }
                Event::Start(Tag::BlockQuote(kind)) => {
                    // GFM alert (`> [!NOTE]`) — pulldown consumes the `[!KIND]` marker into
                    if let Some(alert) = kind {
                        let alert_kind = match alert {
                            pulldown_cmark::BlockQuoteKind::Note => "note",
                            pulldown_cmark::BlockQuoteKind::Tip => "tip",
                            pulldown_cmark::BlockQuoteKind::Important => "important",
                            pulldown_cmark::BlockQuoteKind::Warning => "warning",
                            pulldown_cmark::BlockQuoteKind::Caution => "caution",
                        };
                        let idx = b.push_admonition(alert_kind, None, None);
                        apply_pending_anchor(&mut b, &mut pending_anchor, idx);
                        blockquote_stack.push(Some(idx));
                    } else {
                        b.push_quote_start();
                        blockquote_stack.push(None);
                    }
                }
                Event::End(TagEnd::BlockQuote(_)) => match blockquote_stack.pop() {
                    Some(Some(_)) => {}
                    Some(None) | None => b.push_quote_end(),
                },
                Event::Start(Tag::List(start)) => {
                    // A sublist nests INSIDE its parent item (`Start(Item)` -> ... -> `Start(List)`
                    // -> ... -> `End(List)` -> ... -> `End(Item)`), so the parent's text has already
                    // accumulated in `list_item_text` by the time this sublist starts. Flush it now,
                    // before descending, so the parent lands before its children in document order —
                    // an emit-on-`End(Item)` design would place it after them instead. Flush the WHOLE
                    // buffer (and its annotations), not just the last `Text` event: item text arrives
                    // across multiple events (emphasis/strong/strikethrough/link/code/math/soft-break/
                    // task marker all write into `list_item_text`). `list_stack.last()` is still the
                    // ENCLOSING list here — the sublist itself hasn't been pushed yet. See GH#1459.
                    flush_list_item_text!();
                    let ordered = start.is_some();
                    b.push_list(ordered);
                    list_stack.push(ordered);
                }
                Event::End(TagEnd::List(_)) if list_stack.pop().is_some() => {
                    b.end_list();
                }
                Event::Start(Tag::Item) => {
                    list_item_text.clear();
                    list_item_annotations.clear();
                    annotation_starts.clear();
                    in_list_item += 1;
                }
                Event::End(TagEnd::Item) => {
                    let item_depth = in_list_item;
                    in_list_item = in_list_item.saturating_sub(1);
                    let trimmed = list_item_text.trim();
                    if let Some(ordered) = list_stack.last().copied()
                        && !trimmed.is_empty()
                    {
                        let annotations = adjust_annotations_for_trim(
                            std::mem::take(&mut list_item_annotations),
                            &list_item_text,
                            trimmed,
                        );
                        b.push_list_item(trimmed, ordered, annotations, None, None);
                    }
                    pending_footnote_refs.flush(&mut b, FootnoteRefHolder::ListItem(item_depth));
                    list_item_text.clear();
                    list_item_annotations.clear();
                }
                Event::Start(Tag::Table(_)) => {
                    flush_list_item_text!();
                    table_rows.clear();
                }
                Event::End(TagEnd::Table) => {
                    if !table_rows.is_empty() {
                        let markdown = super::frontmatter_utils::cells_to_markdown(&table_rows);
                        let table = Table {
                            cells: std::mem::take(&mut table_rows),
                            markdown,
                            page_number: 1,
                            bounding_box: None,
                            ..Default::default()
                        };
                        let idx = b.push_table(table, None, None);
                        apply_pending_anchor(&mut b, &mut pending_anchor, idx);
                    }
                    pending_footnote_refs.flush(&mut b, FootnoteRefHolder::Table);
                    table_rows.clear();
                }
                Event::Start(Tag::TableHead | Tag::TableRow) => {
                    current_row.clear();
                }
                Event::End(TagEnd::TableHead | TagEnd::TableRow) if !current_row.is_empty() => {
                    table_rows.push(std::mem::take(&mut current_row));
                }
                Event::Start(Tag::TableCell) => {
                    current_cell.clear();
                    in_table_cell = true;
                }
                Event::End(TagEnd::TableCell) => {
                    in_table_cell = false;
                    current_row.push(current_cell.trim().to_string());
                    current_cell.clear();
                }
                Event::Start(Tag::Image { dest_url, .. }) => {
                    in_image = true;
                    image_alt.clear();
                    image_url = Some(dest_url.to_string());
                }
                Event::End(TagEnd::Image) => {
                    in_image = false;
                    let trimmed = image_alt.trim();
                    let desc = if trimmed.is_empty() { None } else { Some(trimmed) };

                    let url = image_url.take().filter(|u| !u.is_empty());
                    let decoded_image = url
                        .as_deref()
                        .filter(|u| u.starts_with("data:image/"))
                        .and_then(|u| crate::extractors::markdown_utils::decode_data_uri_image(u, image_counter));

                    // Data-URI images are decoded here so the builder can emit an
                    // `ElementKind::Image` whose `image_index` actually resolves in `doc.images`.
                    // Plain-URL images have no bytes to attach, and a placeholder element with an
                    // unresolvable index is silently dropped by every renderer, so their
                    // reference is preserved as visible text instead. ~keep
                    if let Some(mut image) = decoded_image {
                        image_counter += 1;
                        image.description = desc.map(str::to_string);
                        b.push_image(desc, image, None, None);
                    } else {
                        let display = match (&url, desc) {
                            (Some(u), Some(d)) => format!("[Image: {d} ({u})]"),
                            (Some(u), None) => format!("[Image: {u}]"),
                            (None, Some(d)) => format!("[Image: {d}]"),
                            (None, None) => String::new(),
                        };
                        if !display.is_empty() {
                            let idx = b.push_paragraph(&display, vec![], None, None);
                            apply_pending_anchor(&mut b, &mut pending_anchor, idx);
                        }
                    }

                    if let Some(url) = url {
                        b.push_uri(ExtractedUri {
                            url,
                            label: desc.map(str::to_string),
                            page: None,
                            kind: UriKind::Image,
                        });
                    }
                    image_alt.clear();
                }
                Event::Start(Tag::FootnoteDefinition(label)) => {
                    footnote_def_label = Some(label.to_string());
                    footnote_def_text.clear();
                }
                Event::End(TagEnd::FootnoteDefinition) => {
                    if let Some(label) = footnote_def_label.take() {
                        let text = footnote_def_text.trim().to_string();
                        if !text.is_empty() {
                            b.push_footnote_definition(&text, &label, None);
                        }
                    }
                    footnote_def_text.clear();
                }
                Event::Start(Tag::DefinitionListTitle) => {
                    in_def_title = true;
                    def_buf.clear();
                }
                Event::End(TagEnd::DefinitionListTitle) => {
                    in_def_title = false;
                    let trimmed = def_buf.trim();
                    if !trimmed.is_empty() {
                        b.push_definition_term(trimmed, None);
                    }
                    pending_footnote_refs.flush(&mut b, FootnoteRefHolder::DefinitionTerm);
                    def_buf.clear();
                }
                Event::Start(Tag::DefinitionListDefinition) => {
                    in_def_desc = true;
                    def_buf.clear();
                }
                Event::End(TagEnd::DefinitionListDefinition) => {
                    in_def_desc = false;
                    let trimmed = def_buf.trim();
                    if !trimmed.is_empty() {
                        b.push_definition_description(trimmed, None);
                    }
                    pending_footnote_refs.flush(&mut b, FootnoteRefHolder::DefinitionDescription);
                    def_buf.clear();
                }
                Event::Code(s) => {
                    if in_code_block {
                        code_text.push_str(s);
                    } else if in_heading {
                        let start = heading_text.len() as u32;
                        heading_text.push_str(s);
                        let end = heading_text.len() as u32;
                        if start < end {
                            heading_annotations.push(builder::code(start, end));
                        }
                    } else if in_image {
                        image_alt.push_str(s);
                    } else if in_table_cell {
                        current_cell.push_str(s);
                    } else if in_list_item > 0 {
                        let start = list_item_text.len() as u32;
                        list_item_text.push_str(s);
                        let end = list_item_text.len() as u32;
                        if start < end {
                            list_item_annotations.push(builder::code(start, end));
                        }
                    } else if footnote_def_label.is_some() {
                        footnote_def_text.push_str(s);
                    } else if in_def_title || in_def_desc {
                        def_buf.push_str(s);
                    } else if in_paragraph {
                        let start = paragraph_text.len() as u32;
                        paragraph_text.push_str(s);
                        let end = paragraph_text.len() as u32;
                        if start < end {
                            paragraph_annotations.push(builder::code(start, end));
                        }
                    }
                }
                Event::Text(s) => {
                    let text = if let Some((kind, title, remaining)) = super::myst::myst_admonition_metadata(s) {
                        if let Some(Some(index)) = blockquote_stack.last().copied() {
                            b.merge_attribute(index, "kind", kind);
                            if let Some(title) = title {
                                b.merge_attribute(index, "title", title);
                            }
                        }
                        remaining
                    } else {
                        s
                    };
                    if in_code_block {
                        code_text.push_str(text);
                    } else if in_heading {
                        heading_text.push_str(text);
                    } else if in_image {
                        image_alt.push_str(text);
                    } else if in_table_cell {
                        current_cell.push_str(text);
                    } else if in_list_item > 0 {
                        list_item_text.push_str(text);
                    } else if footnote_def_label.is_some() {
                        footnote_def_text.push_str(text);
                    } else if in_def_title || in_def_desc {
                        def_buf.push_str(text);
                    } else if in_paragraph {
                        paragraph_text.push_str(text);
                    }
                }
                Event::InlineMath(s) => {
                    if in_heading {
                        heading_text.push('$');
                        heading_text.push_str(s);
                        heading_text.push('$');
                    } else if in_table_cell {
                        current_cell.push('$');
                        current_cell.push_str(s);
                        current_cell.push('$');
                    } else if in_list_item > 0 {
                        list_item_text.push('$');
                        list_item_text.push_str(s);
                        list_item_text.push('$');
                    } else if footnote_def_label.is_some() {
                        footnote_def_text.push('$');
                        footnote_def_text.push_str(s);
                        footnote_def_text.push('$');
                    } else if in_def_title || in_def_desc {
                        def_buf.push('$');
                        def_buf.push_str(s);
                        def_buf.push('$');
                    } else if in_paragraph {
                        paragraph_text.push('$');
                        paragraph_text.push_str(s);
                        paragraph_text.push('$');
                    }
                }
                Event::DisplayMath(s) => {
                    flush_list_item_text!();
                    let trimmed = s.trim();
                    if !trimmed.is_empty() {
                        let idx = b.push_formula(trimmed, None, None);
                        apply_pending_anchor(&mut b, &mut pending_anchor, idx);
                    }
                }
                Event::SoftBreak | Event::HardBreak => {
                    if in_code_block {
                        code_text.push('\n');
                    } else if in_heading {
                        heading_text.push(' ');
                    } else if in_list_item > 0 {
                        list_item_text.push(' ');
                    } else if footnote_def_label.is_some() {
                        footnote_def_text.push(' ');
                    } else if in_def_title || in_def_desc {
                        def_buf.push(' ');
                    } else if in_paragraph {
                        paragraph_text.push(' ');
                    }
                }
                Event::FootnoteReference(name) => {
                    // The holder is the block whose buffer takes the text around the
                    // reference, in the same order as the `Event::Text` arm.
                    let holder = if in_heading {
                        Some(FootnoteRefHolder::Heading)
                    } else if in_table_cell {
                        Some(FootnoteRefHolder::Table)
                    } else if in_list_item > 0 {
                        Some(FootnoteRefHolder::ListItem(in_list_item))
                    } else if footnote_def_label.is_some() {
                        None
                    } else if in_def_title {
                        Some(FootnoteRefHolder::DefinitionTerm)
                    } else if in_def_desc {
                        Some(FootnoteRefHolder::DefinitionDescription)
                    } else if in_paragraph {
                        Some(FootnoteRefHolder::Paragraph)
                    } else {
                        None
                    };
                    match holder {
                        Some(holder) => pending_footnote_refs.hold(holder, name),
                        None => {
                            b.push_footnote_ref(name, name, None);
                        }
                    }
                }
                Event::InlineHtml(s) => {
                    if let Some(target) = super::myst::myst_target_marker(s) {
                        pending_anchor = Some(target.to_string());
                        continue;
                    }
                    if in_heading {
                        heading_text.push_str(s);
                    } else if in_table_cell {
                        current_cell.push_str(s);
                    } else if in_list_item > 0 {
                        list_item_text.push_str(s);
                    } else if footnote_def_label.is_some() {
                        footnote_def_text.push_str(s);
                    } else if in_def_title || in_def_desc {
                        def_buf.push_str(s);
                    } else if in_paragraph {
                        paragraph_text.push_str(s);
                    }
                }
                // Block-level raw HTML (e.g. a bare `<div>...</div>` between blank lines) is
                // emitted by pulldown-cmark with no enclosing paragraph/heading/etc. buffer open.
                // It used to be silently dropped in that case; it is now recorded as a raw block
                // so callers can recover it. See issue #135.
                Event::Html(s) => {
                    if let Some(target) = super::myst::myst_target_marker(s) {
                        pending_anchor = Some(target.to_string());
                        continue;
                    }
                    if in_heading {
                        heading_text.push_str(s);
                    } else if in_table_cell {
                        current_cell.push_str(s);
                    } else if in_list_item > 0 {
                        list_item_text.push_str(s);
                    } else if footnote_def_label.is_some() {
                        footnote_def_text.push_str(s);
                    } else if in_def_title || in_def_desc {
                        def_buf.push_str(s);
                    } else if in_paragraph {
                        paragraph_text.push_str(s);
                    } else {
                        let trimmed = s.trim();
                        if !trimmed.is_empty() {
                            b.push_raw_block("html", trimmed, None);
                        }
                    }
                }
                Event::TaskListMarker(checked) if in_list_item > 0 => {
                    list_item_text.push_str(if *checked { "[x] " } else { "[ ] " });
                }
                _ => {}
            }
        }
        pending_footnote_refs.flush_all(&mut b);

        b.build()
    }
}

impl Default for MarkdownExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for MarkdownExtractor {
    fn name(&self) -> &str {
        "markdown-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Extracts content from Markdown files with YAML frontmatter and table support"
    }

    fn author(&self) -> &str {
        "Xberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for MarkdownExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        tracing::debug!(format = "markdown", size_bytes = content.len(), "extraction starting");
        let mut budget = SecurityBudget::from_config(config);
        budget.account_text(content.len())?;
        let text = crate::utils::strip_bom(&String::from_utf8_lossy(content)).to_owned();

        let (yaml, remaining_content, frontmatter_warning) = extract_frontmatter_with_warning(&text);

        let mut metadata = if let Some(ref yaml_value) = yaml {
            extract_metadata_from_yaml(yaml_value)
        } else {
            Metadata::default()
        };

        if metadata.title.is_none()
            && let Some(title) = extract_title_from_content(&remaining_content)
        {
            metadata.title = Some(title);
        }

        let text_notebook = super::myst::parse_myst_text_notebook(&text, &mut budget)?;
        #[cfg(feature = "notebook")]
        if let Some(notebook) = text_notebook {
            let mut doc =
                super::jupyter::JupyterExtractor::render_text_notebook(notebook, mime_type, config, &mut budget)?;
            let notebook_additional = std::mem::take(&mut doc.metadata.additional);
            doc.metadata = metadata;
            doc.metadata.additional.extend(notebook_additional);
            doc.processing_warnings.extend(frontmatter_warning);
            return Ok(doc);
        }

        let preprocessed_content = if super::myst::might_contain_myst_syntax(&remaining_content) {
            Some(super::myst::preprocess_myst(&remaining_content, &mut budget)?)
        } else {
            None
        };
        let parser_content = preprocessed_content.as_deref().unwrap_or(&remaining_content);
        let parser = Parser::new_ext(parser_content, markdown_options());
        let events: Vec<Event> = parser.collect();

        // Images (including data-URI decoding) are handled in-line inside
        // `build_internal_document`, which pushes a correctly-indexed image element for each
        // one, so no separate extraction/push pass is needed here.
        let mut doc = Self::build_internal_document(&events, &yaml);
        doc.metadata = metadata;
        #[cfg(not(feature = "notebook"))]
        if let Some(notebook) = text_notebook {
            let cell_metadata = notebook.cell_metadata();
            for (key, value) in notebook.metadata {
                doc.metadata.additional.insert(Cow::Owned(key), value);
            }
            doc.metadata.additional.insert(Cow::Borrowed("cells"), cell_metadata);
        }
        doc.mime_type = mime_type.to_string();
        doc.processing_warnings.extend(frontmatter_warning);

        tracing::debug!(
            element_count = doc.elements.len(),
            format = "markdown",
            "extraction complete"
        );
        Ok(doc)
    }

    async fn extract_path(
        &self,
        path: &std::path::Path,
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        crate::core::path_resolver::extract_file_with_image_resolution(self, path, mime_type, config).await
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "text/markdown",
            "text/x-markdown",
            "text/x-gfm",
            "text/x-commonmark",
            "text/x-markdown-extra",
            "text/x-multimarkdown",
            "text/x-pandoc",
            "text/x-quarto",
            // `application/x-quarto` is declared as an alias of `text/x-quarto` in the
            // static format table (core/mime.rs), so `validate_mime_type` accepts it —
            // but the registry looks extractors up by exact string with no alias
            // resolution, so an unclaimed alias reaches extraction and fails as
            // UnsupportedFormat despite being advertised as supported (#229). ~keep
            "application/x-quarto",
            "text/x-r-markdown",
        ]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::super::frontmatter_utils::{cells_to_markdown, extract_frontmatter, extract_metadata_from_yaml};
    use super::*;
    use crate::types::internal::ElementKind;
    use yaml_serde::Value as YamlValue;

    #[test]
    fn test_can_extract_markdown_mime_types() {
        let extractor = MarkdownExtractor::new();
        let mime_types = extractor.supported_mime_types();

        assert!(mime_types.contains(&"text/markdown"));
        assert!(mime_types.contains(&"text/x-markdown"));
        assert!(mime_types.contains(&"text/x-gfm"));
        assert!(mime_types.contains(&"text/x-commonmark"));
        assert!(mime_types.contains(&"text/x-markdown-extra"));
        assert!(mime_types.contains(&"text/x-multimarkdown"));
        assert!(mime_types.contains(&"text/x-pandoc"));
        assert!(mime_types.contains(&"text/x-quarto"));
        assert!(mime_types.contains(&"text/x-r-markdown"));
    }

    /// For each footnote reference, the kind and text of the element it follows.
    fn blocks_cited_by_footnote_refs(doc: &InternalDocument) -> Vec<(String, ElementKind, String)> {
        let mut cited = Vec::new();
        let mut last: Option<(ElementKind, String)> = None;
        for element in &doc.elements {
            if element.kind == ElementKind::FootnoteRef {
                let (kind, text) = last.clone().expect("a reference follows a block");
                cited.push((element.text.clone(), kind, text));
            } else {
                last = Some((element.kind, element.text.clone()));
            }
        }
        cited
    }

    #[tokio::test]
    async fn test_footnote_ref_follows_its_own_block_when_a_nested_block_closes_first() {
        // A list item and a definition push their text when they close, after the heading,
        // table and blockquote nested in them. Their references still follow them.
        let content = b"- parent[^a]\n\n  # Nested heading\n\n  | x | y |\n  |---|---|\n  | 1 | 2 |\n\n  > quoted\n\n- second[^b]\n\nTerm\n: definition[^c]\n\n    # Definition heading\n\n[^a]: Note a.\n[^b]: Note b.\n[^c]: Note c.\n";
        let doc = MarkdownExtractor::new()
            .extract_content(content, "text/markdown", &ExtractionConfig::default())
            .await
            .expect("extraction should succeed");

        let cited = blocks_cited_by_footnote_refs(&doc);
        assert_eq!(cited.len(), 3, "{cited:?}");
        let (ref name, kind, ref text) = cited[0];
        assert!(
            name == "a" && matches!(kind, ElementKind::ListItem { .. }) && text.starts_with("parent"),
            "{cited:?}"
        );
        let (ref name, kind, ref text) = cited[1];
        assert!(
            name == "b" && matches!(kind, ElementKind::ListItem { .. }) && text == "second",
            "{cited:?}"
        );
        let (ref name, kind, ref text) = cited[2];
        assert!(
            name == "c" && kind == ElementKind::DefinitionDescription && text == "definition",
            "{cited:?}"
        );
    }

    /// Render a document through the same path the extractor's callers use.
    async fn render(content: &[u8]) -> String {
        let doc = MarkdownExtractor::new()
            .extract_content(content, "text/markdown", &ExtractionConfig::default())
            .await
            .expect("extraction should succeed");
        crate::rendering::render_markdown(&doc)
    }

    #[tokio::test]
    async fn test_extract_simple_markdown() {
        let content =
            b"# Header\n\nThis is a paragraph with **bold** and *italic* text.\n\n## Subheading\n\nMore content here.";
        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, remaining) = extract_frontmatter(&text);
        assert!(yaml.is_none());
        assert!(!remaining.is_empty());

        let extracted = render(content).await;

        assert!(extracted.contains("Header"));
        assert!(extracted.contains("This is a paragraph"));
        assert!(extracted.contains("bold"));
        assert!(extracted.contains("italic"));
    }

    #[test]
    fn test_extract_frontmatter_metadata() {
        let content = b"---\ntitle: My Document\nauthor: John Doe\ndate: 2024-01-15\nkeywords: rust, markdown, extraction\ndescription: A test document\n---\n\n# Content\n\nBody text.";

        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml_opt, remaining) = extract_frontmatter(&text);
        assert!(yaml_opt.is_some());
        assert!(remaining.contains("# Content"));

        let yaml = yaml_opt.expect("Should extract YAML frontmatter");
        let metadata = extract_metadata_from_yaml(&yaml);

        assert_eq!(metadata.title.as_deref(), Some("My Document"));
        assert_eq!(metadata.created_by.as_deref(), Some("John Doe"));
        assert_eq!(metadata.created_at, Some("2024-01-15".to_string()));
        assert!(metadata.subject.is_some());
        assert!(
            metadata
                .subject
                .as_ref()
                .expect("Should have subject description")
                .contains("test document")
        );
    }

    #[test]
    fn test_extract_frontmatter_metadata_array_keywords() {
        let content = b"---\ntitle: Document\nkeywords:\n  - rust\n  - markdown\n  - parsing\n---\n\nContent";

        let text = String::from_utf8_lossy(content).into_owned();
        let (yaml_opt, _remaining) = extract_frontmatter(&text);

        assert!(yaml_opt.is_some());
        let yaml = yaml_opt.expect("Should extract YAML frontmatter");
        let metadata = extract_metadata_from_yaml(&yaml);

        let keywords = metadata
            .keywords
            .as_ref()
            .expect("Should extract keywords from metadata");
        assert!(keywords.iter().any(|k| k == "rust"));
        assert!(keywords.iter().any(|k| k == "markdown"));
    }

    #[tokio::test]
    async fn test_extract_tables() {
        let content = b"# Tables Example\n\n| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |\n| Cell 3   | Cell 4   |";

        let extractor = MarkdownExtractor::new();
        let result = extractor
            .extract_content(content, "text/markdown", &ExtractionConfig::default())
            .await
            .expect("Should extract markdown with tables");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        assert!(!result.tables.is_empty());
        let table = &result.tables[0];
        assert!(!table.cells.is_empty());
        assert_eq!(table.cells[0].len(), 2);
        assert!(!table.markdown.is_empty());
    }

    #[test]
    fn test_extract_without_frontmatter() {
        let content = b"# Main Title\n\nSome content\n\nMore text";
        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, remaining) = extract_frontmatter(&text);
        assert!(yaml.is_none());
        assert_eq!(remaining, text);

        let title = extract_title_from_content(&remaining);
        assert_eq!(title, Some("Main Title".to_string()));
    }

    #[tokio::test]
    async fn test_empty_document() {
        let content = b"";
        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, remaining) = extract_frontmatter(&text);
        assert!(yaml.is_none());
        assert!(remaining.is_empty());

        assert!(render(content).await.is_empty());
    }

    #[tokio::test]
    async fn test_whitespace_only_document() {
        let content = b"   \n\n  \n";
        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, _remaining) = extract_frontmatter(&text);
        assert!(yaml.is_none());

        assert!(render(content).await.trim().is_empty());
    }

    #[tokio::test]
    async fn test_unicode_content() {
        let content = "# 日本語のタイトル\n\nこれは日本語の内容です。\n\n## Español\n\nEste es un documento en español.\n\n## Русский\n\nЭто русский текст.".as_bytes();

        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, _remaining) = extract_frontmatter(&text);
        assert!(yaml.is_none());

        let extracted = render(content).await;

        assert!(extracted.contains("日本語"));
        assert!(extracted.contains("Español"));
        assert!(extracted.contains("Русский"));
    }

    #[tokio::test]
    async fn test_full_extraction_with_frontmatter_and_tables() {
        let content = b"---\ntitle: Complete Document\nauthor: Test Author\ndate: 2024-01-20\n---\n\n# Document\n\nIntroduction text.\n\n| Name | Value |\n|------|-------|\n| A    | 1     |\n| B    | 2     |";

        let extractor = MarkdownExtractor::new();
        let result = extractor
            .extract_content(content, "text/x-markdown", &ExtractionConfig::default())
            .await
            .expect("Should extract markdown with frontmatter and tables");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        assert_eq!(result.mime_type, "text/x-markdown");
        assert!(result.content.contains("Introduction text"));
        assert_eq!(result.metadata.title.as_deref(), Some("Complete Document"));
        assert_eq!(result.metadata.created_by.as_deref(), Some("Test Author"));
        assert!(!result.tables.is_empty());
    }

    #[test]
    fn test_plugin_interface() {
        let extractor = MarkdownExtractor::new();
        assert_eq!(extractor.name(), "markdown-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
        assert!(extractor.supported_mime_types().contains(&"text/markdown"));
    }

    #[test]
    fn test_cells_to_markdown() {
        let cells = vec![
            vec!["Header 1".to_string(), "Header 2".to_string()],
            vec!["Data 1".to_string(), "Data 2".to_string()],
            vec!["Data 3".to_string(), "Data 4".to_string()],
        ];

        let markdown = cells_to_markdown(&cells);
        assert!(markdown.contains("Header 1"));
        assert!(markdown.contains("Data 1"));
        assert!(markdown.contains("---"));
        let lines: Vec<&str> = markdown.lines().collect();
        assert!(lines.len() >= 4);
    }

    #[tokio::test]
    async fn test_extract_markdown_with_links() {
        let content = b"# Page\n\nCheck [Google](https://google.com) and [Rust](https://rust-lang.org).";
        let extracted = render(content).await;

        assert!(extracted.contains("Google"));
        assert!(extracted.contains("Rust"));
    }

    #[tokio::test]
    async fn test_extract_markdown_with_code_blocks() {
        let content = b"# Code Example\n\n```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
        let extracted = render(content).await;

        assert!(extracted.contains("main"));
        assert!(extracted.contains("println"));
    }

    #[test]
    fn test_malformed_frontmatter_fallback() {
        let content = b"---\nthis: is: invalid: yaml:\n---\n\nContent here";
        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, _remaining) = extract_frontmatter(&text);
        let _ = yaml;
    }

    #[test]
    fn test_metadata_extraction_completeness() {
        let yaml_str = r#"
title: "Test Document"
author: "Test Author"
date: "2024-01-15"
keywords:
  - rust
  - markdown
  - testing
description: "A test description"
abstract: "Test abstract"
subject: "Test subject"
category: "Documentation"
version: "1.2.3"
language: "en"
tags:
  - tag1
  - tag2
custom_field: "custom_value"
nested:
  organization: "Test Corp"
  contact:
    email: "test@example.com"
"#;

        let yaml: YamlValue = yaml_serde::from_str(yaml_str).expect("Valid YAML");
        let metadata = extract_metadata_from_yaml(&yaml);

        assert_eq!(metadata.created_at, Some("2024-01-15".to_string()));
        assert_eq!(metadata.title.as_deref(), Some("Test Document"));
        assert_eq!(metadata.created_by.as_deref(), Some("Test Author"));

        let keywords = metadata.keywords.as_ref().expect("Should have keywords");
        assert!(keywords.iter().any(|k| k == "rust"));
        assert!(keywords.iter().any(|k| k == "markdown"));

        assert_eq!(metadata.subject, Some("Test subject".to_string()));

        assert_eq!(metadata.abstract_text.as_deref(), Some("Test abstract"));

        assert_eq!(metadata.category.as_deref(), Some("Documentation"));

        let tags = metadata.tags.as_ref().expect("Should have tags");
        assert!(tags.iter().any(|t| t == "tag1"));
        assert!(tags.iter().any(|t| t == "tag2"));

        assert_eq!(metadata.language.as_deref(), Some("en"));

        assert_eq!(metadata.document_version.as_deref(), Some("1.2.3"));

        println!("\nSuccessfully extracted all typed metadata fields");
    }

    #[test]
    fn test_decode_data_uri_png() {
        let png_b64 =
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
        let uri = format!("data:image/png;base64,{png_b64}");

        let image = crate::extractors::markdown_utils::decode_data_uri_image(&uri, 0);
        assert!(image.is_some());
        let img = image.unwrap();
        assert_eq!(img.format.as_ref(), "png");
        assert_eq!(img.image_index, 0);
        assert!(!img.data.is_empty());
    }

    #[test]
    fn test_decode_data_uri_jpeg() {
        let uri = "data:image/jpeg;base64,/9j/4AAQSkZJRg==";

        let image = crate::extractors::markdown_utils::decode_data_uri_image(uri, 3);
        assert!(image.is_some());
        let img = image.unwrap();
        assert_eq!(img.format.as_ref(), "jpeg");
        assert_eq!(img.image_index, 3);
    }

    #[test]
    fn test_decode_data_uri_unsupported_format() {
        let uri = "data:image/tiff;base64,AAAA";
        let image = crate::extractors::markdown_utils::decode_data_uri_image(uri, 0);
        assert!(image.is_none());
    }

    #[test]
    fn test_decode_data_uri_non_base64() {
        let uri = "data:image/png,raw-data-here";
        let image = crate::extractors::markdown_utils::decode_data_uri_image(uri, 0);
        assert!(image.is_none());
    }

    #[test]
    fn test_decode_data_uri_invalid_base64() {
        let uri = "data:image/png;base64,!!!not-valid-base64!!!";
        let image = crate::extractors::markdown_utils::decode_data_uri_image(uri, 0);
        assert!(image.is_none());
    }

    #[test]
    fn test_decode_data_uri_not_data_uri() {
        let uri = "https://example.com/image.png";
        let image = crate::extractors::markdown_utils::decode_data_uri_image(uri, 0);
        assert!(image.is_none());
    }

    #[tokio::test]
    async fn test_http_image_produces_no_extracted_image_bytes() {
        let md = b"# Title\n\n![alt](https://example.com/photo.jpg)\n\nSome text.";

        let doc = MarkdownExtractor::new()
            .extract_content(md, "text/markdown", &ExtractionConfig::default())
            .await
            .expect("extraction should succeed");

        // A remote URL carries no bytes to decode, so nothing lands in `doc.images`; the
        // reference itself is preserved as text (see
        // `test_markdown_http_image_reference_preserved_in_output`).
        assert!(doc.images.is_empty(), "unexpected images: {:?}", doc.images);
    }

    #[tokio::test]
    async fn test_extract_bytes_with_data_uri_image() {
        let png_b64 =
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
        let md = format!("# Doc\n\n![photo](data:image/png;base64,{png_b64})\n\nText.");

        let extractor = MarkdownExtractor::new();
        let result = extractor
            .extract_content(md.as_bytes(), "text/markdown", &ExtractionConfig::default())
            .await
            .expect("Should extract markdown with data URI image");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        assert!(result.images.is_some());
        let imgs = result.images.unwrap();
        assert_eq!(imgs.len(), 1);
        assert_eq!(imgs[0].format.as_ref(), "png");
    }

    /// Regression test: `build_internal_document` used to create an `ElementKind::Image`
    /// placeholder with a sentinel `image_index: u32::MAX` for every image, then a separate pass
    /// in `extract_content` pushed the actual bytes via the raw, non-index-patching
    /// `InternalDocument::push_image`. The placeholder's index was never fixed up, so every
    /// renderer (which looks images up by walking `ElementKind::Image` elements) silently
    /// dropped the image from rendered output even though `doc.images` had the data. Images are
    /// now decoded and pushed in a single step with a correct index.
    #[tokio::test]
    async fn test_markdown_data_uri_image_renders_in_output() {
        let png_b64 =
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
        let md = format!("Intro.\n\n![a photo](data:image/png;base64,{png_b64})\n\nOutro.");

        let extractor = MarkdownExtractor::new();
        let doc = extractor
            .extract_content(md.as_bytes(), "text/markdown", &ExtractionConfig::default())
            .await
            .expect("extraction should succeed");

        assert_eq!(doc.images.len(), 1);
        let image_element_count = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, crate::types::internal::ElementKind::Image { .. }))
            .count();
        assert_eq!(
            image_element_count, 1,
            "expected one Image element in {:?}",
            doc.elements
        );

        let markdown = crate::rendering::render_markdown(&doc);
        assert!(
            markdown.contains("a photo"),
            "image description missing from rendered markdown: {markdown}"
        );
    }

    /// Regression test: a plain (non-data-URI) image reference used to be dropped from rendered
    /// output entirely (see `test_markdown_data_uri_image_renders_in_output`), since no bytes
    /// were ever available to attach to its placeholder element. It is now preserved as visible
    /// text instead of vanishing.
    #[tokio::test]
    async fn test_markdown_http_image_reference_preserved_in_output() {
        let md = "Intro.\n\n![a photo](https://example.com/photo.jpg)\n\nOutro.";

        let extractor = MarkdownExtractor::new();
        let doc = extractor
            .extract_content(md.as_bytes(), "text/markdown", &ExtractionConfig::default())
            .await
            .expect("extraction should succeed");

        let markdown = crate::rendering::render_markdown(&doc);
        assert!(
            markdown.contains("a photo") && markdown.contains("https://example.com/photo.jpg"),
            "image reference missing from rendered markdown: {markdown}"
        );
    }

    #[tokio::test]
    async fn test_extract_bytes_no_images() {
        let md = b"# Simple\n\nJust text, no images.";

        let extractor = MarkdownExtractor::new();
        let result = extractor
            .extract_content(md, "text/markdown", &ExtractionConfig::default())
            .await
            .expect("Should extract markdown without images");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        assert!(result.images.is_none());
    }

    #[tokio::test]
    async fn test_trimmed_paragraph_with_emoji() {
        let md = b"  **bold** \xf0\x9f\x8e\x89 text  ";

        let extractor = MarkdownExtractor::new();
        let result = extractor
            .extract_content(md, "text/markdown", &ExtractionConfig::default())
            .await
            .expect("Should handle emoji in trimmed paragraph");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        assert!(result.content.contains("bold"), "Bold text preserved");
        assert!(result.content.contains("\u{1F389}"), "Emoji preserved after trim");
    }

    #[tokio::test]
    async fn test_cjk_paragraph_with_formatting() {
        let md = "# CJK\n\nこれは**太字**テスト".as_bytes();

        let extractor = MarkdownExtractor::new();
        let result = extractor
            .extract_content(md, "text/markdown", &ExtractionConfig::default())
            .await
            .expect("Should handle CJK with bold formatting");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        assert!(result.content.contains("太字"), "Bold CJK content present");
        assert!(result.content.contains("これは"), "Leading CJK preserved");
    }

    /// Collect `(text, ordered, depth)` for every `ListItem` element, in document order.
    fn list_items(doc: &crate::types::internal::InternalDocument) -> Vec<(String, bool, u16)> {
        use crate::types::internal::ElementKind;
        doc.elements
            .iter()
            .filter_map(|e| match e.kind {
                ElementKind::ListItem { ordered } => Some((e.text.clone(), ordered, e.depth)),
                _ => None,
            })
            .collect()
    }

    #[tokio::test]
    async fn should_preserve_annotations_after_list_item_paragraph_breaks() {
        let doc = MarkdownExtractor::new()
            .extract_content(
                "- **café first**\n\n  *second paragraph*\n".as_bytes(),
                "text/markdown",
                &ExtractionConfig::default(),
            )
            .await
            .expect("extraction should succeed");
        let items: Vec<_> = doc
            .elements
            .iter()
            .filter(|element| matches!(element.kind, ElementKind::ListItem { .. }))
            .collect();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].text, "café first\n\nsecond paragraph");
        let spans: Vec<_> = items[0]
            .annotations
            .iter()
            .map(|annotation| &items[0].text[annotation.start as usize..annotation.end as usize])
            .collect();
        assert_eq!(spans, ["café first", "second paragraph"]);
    }

    fn block_sequence(doc: &crate::types::internal::InternalDocument) -> Vec<String> {
        use crate::types::internal::ElementKind;
        doc.elements
            .iter()
            .filter_map(|e| match e.kind {
                ElementKind::ListItem { .. } => Some(format!("item {}", e.text)),
                ElementKind::Code => Some(format!("code {}", e.text)),
                ElementKind::Heading { .. } => Some(format!("heading {}", e.text)),
                ElementKind::Formula => Some(format!("formula {}", e.text)),
                ElementKind::Table { .. } => Some("table".to_string()),
                _ => None,
            })
            .collect()
    }

    /// A code block, table, heading or display formula inside a list item is pushed when it
    /// starts or ends, while the item's own text waits for `End(Item)`. The block used to land
    /// before the text of the item that holds it. ~keep
    #[tokio::test]
    async fn test_list_item_text_precedes_the_blocks_inside_it() {
        let cases: [(&[u8], &[&str]); 5] = [
            (
                b"1. Install:\n\n   ```\n   pip install x\n   ```\n2. Run it\n",
                &["item Install:", "code pip install x", "item Run it"],
            ),
            (
                b"- item\n\n      indented code\n- next\n",
                &["item item", "code indented code", "item next"],
            ),
            (
                b"- item\n\n  | a | b |\n  |---|---|\n  | one | two |\n- next\n",
                &["item item", "table", "item next"],
            ),
            (
                b"- Steps\n\n  # Heading in item\n- next\n",
                &["item Steps", "heading Heading in item", "item next"],
            ),
            (
                b"- Formula:\n\n  $$x^2$$\n- next\n",
                &["item Formula:", "formula x^2", "item next"],
            ),
        ];
        let extractor = MarkdownExtractor::new();
        for (md, expected) in cases {
            let doc = extractor
                .extract_content(md, "text/markdown", &ExtractionConfig::default())
                .await
                .expect("extraction should succeed");
            assert_eq!(
                block_sequence(&doc),
                expected,
                "{:?}: the item's text must come before its blocks",
                String::from_utf8_lossy(md)
            );
        }
    }

    /// Regression test for GH#1459: a nested list used to silently lose every ancestor
    /// item's text. Only the deepest item ("L3") survived because `Start(Tag::Item)`
    /// unconditionally clears `list_item_text` with no flush of the enclosing item's
    /// buffer at `Start(Tag::List)`.
    ///
    /// Against the unfixed code this assertion fails: `list_items(&doc).len()` is `1`
    /// (only `[("L3", false, 3)]`) instead of the expected 3 levels.
    #[tokio::test]
    async fn test_nested_list_preserves_all_ancestor_text() {
        let md = b"- L1\n  - L2\n    - L3\n";
        let extractor = MarkdownExtractor::new();
        let doc = extractor
            .extract_content(md, "text/markdown", &ExtractionConfig::default())
            .await
            .expect("extraction should succeed");

        let items = list_items(&doc);
        assert_eq!(
            items,
            vec![
                ("L1".to_string(), false, 1),
                ("L2".to_string(), false, 2),
                ("L3".to_string(), false, 3),
            ],
            "all three nesting levels must survive with increasing depth, got {items:?}"
        );
    }

    /// Regression test for GH#1459: the parent item's text can arrive across multiple
    /// pulldown-cmark events (a `Strong` span plus a soft-break-joined continuation line)
    /// before the sublist starts. The flush at `Start(Tag::List)` must carry the whole
    /// accumulated buffer and its annotations, not just the most recent `Text` event.
    ///
    /// Against the unfixed code this assertion fails: `list_items(&doc)` contains only
    /// `[("Child", false, 2)]` — the parent's text and its bold annotation are discarded
    /// entirely when `Start(Tag::Item)` for "Child" clears `list_item_text`.
    #[tokio::test]
    async fn test_nested_list_flushes_full_multi_event_parent_buffer() {
        use crate::types::document_structure::AnnotationKind;

        let md = b"- Parent **bold** line\n  continued\n  - Child\n";
        let extractor = MarkdownExtractor::new();
        let doc = extractor
            .extract_content(md, "text/markdown", &ExtractionConfig::default())
            .await
            .expect("extraction should succeed");

        let items = list_items(&doc);
        assert_eq!(
            items,
            vec![
                ("Parent bold line continued".to_string(), false, 1),
                ("Child".to_string(), false, 2),
            ],
            "the parent's multi-event text (emphasis span + soft-break continuation) \
             must be flushed whole before descending into the sublist, got {items:?}"
        );

        let parent = doc
            .elements
            .iter()
            .find(|e| e.text == "Parent bold line continued")
            .expect("flushed parent item must be present");
        assert_eq!(
            parent.annotations.len(),
            1,
            "the bold annotation on \"bold\" must survive the flush, got {:?}",
            parent.annotations
        );
        let annotation = &parent.annotations[0];
        assert_eq!(annotation.kind, AnnotationKind::Bold);
        assert_eq!(
            &parent.text[annotation.start as usize..annotation.end as usize],
            "bold",
            "annotation byte range must still point at \"bold\" after the flush"
        );
    }

    /// Regression test for GH#1459: trailing text after a sublist used to be emitted as a
    /// bare `Paragraph` element instead of staying list-item content, because
    /// `End(TagEnd::Item)` for the inner item set the old boolean `in_list_item` back to
    /// `false` while the outer item was still open, so the outer item's trailing text hit
    /// the `Paragraph` start guard.
    ///
    /// Against the unfixed code this assertion fails: `list_items(&doc)` contains only
    /// `[("Child", false, 2)]` (the parent's own text is *also* lost, per the flush defect
    /// above), and a `Paragraph` element with text `"Trailing"` exists in `doc.elements`
    /// where none should.
    #[tokio::test]
    async fn test_trailing_text_after_sublist_stays_list_item() {
        use crate::types::internal::ElementKind;

        let md = b"- Parent\n  - Child\n\n  Trailing\n";
        let extractor = MarkdownExtractor::new();
        let doc = extractor
            .extract_content(md, "text/markdown", &ExtractionConfig::default())
            .await
            .expect("extraction should succeed");

        let items = list_items(&doc);
        assert_eq!(
            items,
            vec![
                ("Parent".to_string(), false, 1),
                ("Child".to_string(), false, 2),
                ("Trailing".to_string(), false, 1),
            ],
            "trailing text after the sublist must become a sibling list item at the \
             parent's depth, got {items:?}"
        );

        let stray_paragraph = doc
            .elements
            .iter()
            .any(|e| matches!(e.kind, ElementKind::Paragraph) && e.text == "Trailing");
        assert!(
            !stray_paragraph,
            "trailing list-item text must not be emitted as a bare Paragraph element"
        );
    }

    #[test]
    fn test_normalize_fence_lang_strips_quarto_braces() {
        assert_eq!(normalize_fence_lang("python"), Some("python".to_string()));
        assert_eq!(normalize_fence_lang("{python}"), Some("python".to_string()));
        assert_eq!(normalize_fence_lang("{r, echo=FALSE}"), Some("r".to_string()));
        assert_eq!(
            normalize_fence_lang("{.python .numberLines}"),
            Some("python".to_string())
        );
        assert_eq!(normalize_fence_lang("  rust  "), Some("rust".to_string()));
        assert_eq!(normalize_fence_lang(""), None);
        assert_eq!(normalize_fence_lang("{}"), None);
    }

    #[tokio::test]
    async fn test_quarto_executable_cells_render_as_clean_code() {
        use crate::types::internal::ElementKind;
        let content = b"---\ntitle: Quarto Doc\n---\n\nProse before.\n\n```{python}\nprint(\"hi\")\n```\n\n```{r, echo=FALSE}\nsummary(cars)\n```\n";
        let extractor = MarkdownExtractor::new();
        let doc = extractor
            .extract_content(content, "text/x-quarto", &ExtractionConfig::default())
            .await
            .expect("should extract a Quarto document");

        assert_eq!(doc.metadata.title.as_deref(), Some("Quarto Doc"));
        let code_langs: Vec<Option<String>> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Code))
            .map(|e| e.attributes.as_ref().and_then(|a| a.get("language").cloned()))
            .collect();
        assert_eq!(
            code_langs,
            vec![Some("python".to_string()), Some("r".to_string())],
            "executable cell braces are stripped to bare kernel languages"
        );
        let code_bodies: String = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Code))
            .map(|e| e.text.clone())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(code_bodies.contains("print(\"hi\")"), "python cell body preserved");
        assert!(code_bodies.contains("summary(cars)"), "r cell body preserved");
    }

    #[tokio::test]
    async fn test_pandoc_full_elements_parsed() {
        use crate::AnnotationKind;
        use crate::types::internal::ElementKind;
        let content = concat!(
            "Marker ^sup^ and ~sub~ inline.\n\n",
            "Inline $a^2 + b^2$ stays inline.\n\n",
            "$$\\int_0^1 x\\,dx$$\n\n",
            "Term 1\n: Definition of term 1.\n\n",
            "- [x] done\n- [ ] todo\n\n",
            "Here is a note[^1].\n\n[^1]: The footnote body.\n",
        )
        .as_bytes();
        let extractor = MarkdownExtractor::new();
        let doc = extractor
            .extract_content(content, "text/x-pandoc", &ExtractionConfig::default())
            .await
            .expect("should extract pandoc-flavored markdown");

        assert!(
            doc.elements
                .iter()
                .any(|e| matches!(e.kind, ElementKind::Formula) && e.text.contains("\\int")),
            "display math becomes a Formula element"
        );
        assert!(
            doc.elements
                .iter()
                .any(|e| matches!(e.kind, ElementKind::DefinitionTerm) && e.text.contains("Term 1")),
            "definition term parsed"
        );
        assert!(
            doc.elements.iter().any(
                |e| matches!(e.kind, ElementKind::DefinitionDescription) && e.text.contains("Definition of term 1")
            ),
            "definition description parsed"
        );
        assert!(
            doc.elements
                .iter()
                .any(|e| matches!(e.kind, ElementKind::FootnoteDefinition)),
            "footnote definition parsed"
        );
        let list_text: String = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::ListItem { .. }))
            .map(|e| e.text.clone())
            .collect::<Vec<_>>()
            .join("|");
        assert!(
            list_text.contains("[x]") && list_text.contains("[ ]"),
            "task-list markers rendered: {list_text}"
        );
        assert!(
            doc.elements
                .iter()
                .any(|e| matches!(e.kind, ElementKind::Paragraph) && e.text.contains("$a^2 + b^2$")),
            "inline math preserved with $ delimiters"
        );
        assert!(
            doc.elements
                .iter()
                .any(|e| e.annotations.iter().any(|a| a.kind == AnnotationKind::Superscript)),
            "superscript (^sup^) recorded as an annotation"
        );
        assert!(
            doc.elements
                .iter()
                .any(|e| e.annotations.iter().any(|a| a.kind == AnnotationKind::Subscript)),
            "subscript (~sub~) recorded as an annotation"
        );
    }

    #[tokio::test]
    async fn test_gfm_alert_becomes_admonition() {
        use crate::types::internal::ElementKind;
        let content = b"> [!WARNING]\n> Be careful here.\n";
        let extractor = MarkdownExtractor::new();
        let doc = extractor
            .extract_content(content, "text/x-gfm", &ExtractionConfig::default())
            .await
            .expect("should extract a GFM alert");
        assert!(
            doc.elements.iter().any(|e| matches!(e.kind, ElementKind::Admonition)),
            "GFM alert renders as an admonition rather than losing the [!WARNING] marker"
        );
        assert!(
            doc.elements.iter().any(|e| e.text.contains("Be careful here")),
            "alert body text retained"
        );
    }

    #[tokio::test]
    async fn test_smart_punctuation_rewrites_quotes() {
        let content = "He said \"hello\" -- really.".as_bytes();
        let extractor = MarkdownExtractor::new();
        let doc = extractor
            .extract_content(content, "text/markdown", &ExtractionConfig::default())
            .await
            .expect("should extract");
        let text: String = doc
            .elements
            .iter()
            .map(|e| e.text.clone())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            text.contains('\u{201C}') || text.contains('\u{201D}'),
            "straight quotes become curly: {text}"
        );
        assert!(
            text.contains('\u{2013}') || text.contains('\u{2014}'),
            "-- becomes en/em dash: {text}"
        );
    }
}
