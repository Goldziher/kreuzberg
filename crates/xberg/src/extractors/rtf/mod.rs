//! RTF (Rich Text Format) extractor.
//!
//! Supports: Rich Text Format (.rtf)
//!
//! This native Rust extractor provides text extraction from RTF documents with:
//! - Character encoding support (Windows-1252 for 0x80-0x9F range)
//! - Common RTF control words (paragraph breaks, tabs, bullets, quotes, dashes)
//! - Unicode escape sequences
//! - Image metadata extraction
//! - Whitespace normalization

mod encoding;
mod formatting;
mod images;
mod metadata;
mod parser;
mod tables;

pub use formatting::normalize_whitespace;
pub(crate) use metadata::extract_rtf_metadata;
pub(crate) use parser::{extract_rtf_formatting, extract_text_from_rtf, spans_to_annotations};

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::security::SecurityBudget;
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::types::ExtractedImage;
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::metadata::Metadata;
use crate::types::uri::ExtractedUri;
use async_trait::async_trait;
use bytes::Bytes;
use std::borrow::Cow;

fn expand_binary_payloads<'a>(content: &'a [u8], budget: &mut SecurityBudget) -> Result<Cow<'a, [u8]>> {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut output: Option<Vec<u8>> = None;
    let mut copied_until = 0usize;
    let mut index = 0usize;
    while index + 4 <= content.len() {
        if &content[index..index + 4] != b"\\bin" {
            index += 1;
            continue;
        }
        let preceding_backslashes = content[..index].iter().rev().take_while(|byte| **byte == b'\\').count();
        if preceding_backslashes % 2 == 1 {
            index += 1;
            continue;
        }

        let mut cursor = index + 4;
        let digit_start = cursor;
        while cursor < content.len() && content[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor == digit_start {
            index += 1;
            continue;
        }
        let Ok(count_text) = std::str::from_utf8(&content[digit_start..cursor]) else {
            index += 1;
            continue;
        };
        let Ok(count) = count_text.parse::<usize>() else {
            index += 1;
            continue;
        };
        if content.get(cursor) == Some(&b' ') {
            cursor += 1;
        }
        let Some(payload_end) = cursor.checked_add(count).filter(|end| *end <= content.len()) else {
            index += 1;
            continue;
        };

        budget.account_text(count)?;
        let output = output.get_or_insert_with(|| Vec::with_capacity(content.len().saturating_add(count)));
        output.extend_from_slice(&content[copied_until..index]);
        output.push(b' ');
        for byte in &content[cursor..payload_end] {
            output.push(HEX[(byte >> 4) as usize]);
            output.push(HEX[(byte & 0x0f) as usize]);
        }
        copied_until = payload_end;
        index = payload_end;
    }

    Ok(match output {
        Some(mut output) => {
            output.extend_from_slice(&content[copied_until..]);
            Cow::Owned(output)
        }
        None => Cow::Borrowed(content),
    })
}

/// `ProcessingWarning::source` for every warning this extractor emits (#171).
const RTF_WARNING_SOURCE: &str = "rtf";

/// Native Rust RTF extractor.
///
/// Extracts text content, metadata, and structure from RTF documents
#[cfg_attr(alef, alef(skip))]
pub struct RtfExtractor;

impl RtfExtractor {
    /// Create a new RTF extractor.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for RtfExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl RtfExtractor {
    /// Build an `InternalDocument` from RTF content.
    ///
    /// Extracts paragraphs (split on double-newlines), tables, formatting
    /// annotations, and header/footer content with appropriate content layers.
    fn build_internal_document(rtf_content: &str, plain: bool) -> InternalDocument {
        use crate::types::document_structure::ContentLayer;

        let (extracted_text, tables, rtf_images, para_metas, mut formatting) =
            extract_text_from_rtf(rtf_content, plain);
        let legacy_formatting = extract_rtf_formatting(rtf_content);
        formatting.header_text = legacy_formatting.header_text;
        formatting.footer_text = legacy_formatting.footer_text;

        let mut builder = InternalDocumentBuilder::new("rtf");

        for (_start, _end, url) in &formatting.hyperlinks {
            if !url.is_empty() {
                let label = extracted_text.get(*_start..*_end).map(|s| s.to_string());
                builder.push_uri(ExtractedUri::hyperlink(url, label));
            }
        }

        let mut table_idx = 0;
        let mut meta_idx = 0;
        let mut byte_offset = 0usize;
        let mut in_table_rows = false;

        let mut in_list = false;
        let mut list_id: Option<u16> = None;
        let mut list_depth: u8 = 0;

        for paragraph in extracted_text.split("\n\n") {
            let trimmed = paragraph.trim();
            if trimmed.is_empty() {
                byte_offset += paragraph.len() + 2;
                meta_idx += 1;
                continue;
            }

            let meta = para_metas.get(meta_idx).cloned().unwrap_or_default();
            meta_idx += 1;

            if meta.is_table {
                if in_list {
                    for _ in 0..=list_depth {
                        builder.end_list();
                    }
                    in_list = false;
                    list_id = None;
                    list_depth = 0;
                }
                in_table_rows = true;
                byte_offset += paragraph.len() + 2;
                continue;
            }

            if in_table_rows {
                in_table_rows = false;
                if table_idx < tables.len() {
                    builder.push_table_from_cells(&tables[table_idx].cells, None, None);
                    table_idx += 1;
                }
            }

            let lines: Vec<&str> = trimmed.lines().collect();
            let is_table_like = lines.len() >= 2 && lines.iter().all(|l| l.contains('|'));
            if is_table_like && table_idx < tables.len() {
                if in_list {
                    for _ in 0..=list_depth {
                        builder.end_list();
                    }
                    in_list = false;
                    list_id = None;
                    list_depth = 0;
                }
                builder.push_table_from_cells(&tables[table_idx].cells, None, None);
                table_idx += 1;
                byte_offset += paragraph.len() + 2;
                continue;
            }

            let trim_offset = byte_offset + (paragraph.len() - paragraph.trim_start().len());
            let annotations = spans_to_annotations(trim_offset, trim_offset + trimmed.len(), &formatting);

            if meta.heading_level > 0 && meta.heading_level <= 6 {
                if in_list {
                    for _ in 0..=list_depth {
                        builder.end_list();
                    }
                    in_list = false;
                    list_id = None;
                    list_depth = 0;
                }
                builder.push_heading(meta.heading_level, trimmed, None, None);
            } else if let Some(level) = meta.list_level {
                let new_list_id = meta.list_id;
                let ordered = meta.ordered;

                if !in_list || list_id != new_list_id {
                    if in_list {
                        for _ in 0..=list_depth {
                            builder.end_list();
                        }
                    }
                    builder.push_list(ordered);
                    for _ in 0..level {
                        builder.push_list(ordered);
                    }
                    in_list = true;
                    list_id = new_list_id;
                    list_depth = level;
                } else if level > list_depth {
                    for _ in list_depth..level {
                        builder.push_list(ordered);
                    }
                    list_depth = level;
                } else if level < list_depth {
                    for _ in level..list_depth {
                        builder.end_list();
                    }
                    list_depth = level;
                }

                builder.push_list_item(trimmed, ordered, annotations, None, None);
            } else {
                if in_list {
                    for _ in 0..=list_depth {
                        builder.end_list();
                    }
                    in_list = false;
                    list_id = None;
                    list_depth = 0;
                }
                builder.push_paragraph(trimmed, annotations, None, None);
            }
            byte_offset += paragraph.len() + 2;
        }

        if in_list {
            for _ in 0..=list_depth {
                builder.end_list();
            }
        }

        if in_table_rows && table_idx < tables.len() {
            builder.push_table_from_cells(&tables[table_idx].cells, None, None);
            table_idx += 1;
        }

        while table_idx < tables.len() {
            builder.push_table_from_cells(&tables[table_idx].cells, None, None);
            table_idx += 1;
        }

        for (i, rtf_img) in rtf_images.into_iter().enumerate() {
            let (image_kind, kind_confidence) =
                crate::extraction::image_kind::classify(crate::extraction::image_kind::ImageClassifyInput {
                    bytes: &rtf_img.data,
                    format: rtf_img.format,
                    width: None,
                    height: None,
                    colorspace: None,
                    bits_per_component: None,
                    is_mask: false,
                });

            let image = ExtractedImage {
                data: Bytes::from(rtf_img.data),
                format: Cow::Borrowed(rtf_img.format),
                image_index: i as u32,
                page_number: None,
                width: None,
                height: None,
                colorspace: None,
                bits_per_component: None,
                is_mask: false,
                description: Some("image".to_string()),
                ocr_result: None,
                bounding_box: None,
                source_path: None,
                image_kind: Some(image_kind),
                kind_confidence: Some(kind_confidence),
                cluster_id: None,
                caption: None,
                qr_codes: None,
                data_base64: None,
            };
            builder.push_image(None, image, None, None);
        }

        if let Some(ref header) = formatting.header_text {
            let idx = builder.push_paragraph(header, vec![], None, None);
            builder.set_layer(idx, ContentLayer::Header);
        }
        if let Some(ref footer) = formatting.footer_text {
            let idx = builder.push_paragraph(footer, vec![], None, None);
            builder.set_layer(idx, ContentLayer::Footer);
        }

        builder.build()
    }
}

impl Plugin for RtfExtractor {
    fn name(&self) -> &str {
        "rtf-extractor"
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
        "Extracts content from RTF (Rich Text Format) files with native Rust parsing"
    }

    fn author(&self) -> &str {
        "Xberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for RtfExtractor {
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        tracing::debug!(format = "rtf", size_bytes = content.len(), "extraction starting");
        let mut budget = SecurityBudget::from_config(config);
        budget.account_text(content.len())?;
        let normalized_content = expand_binary_payloads(content, &mut budget)?;
        let rtf_content = String::from_utf8_lossy(&normalized_content);
        let plain = true;

        let (extracted_text, _tables, _images, _metas, _formatting_data) = extract_text_from_rtf(&rtf_content, plain);
        let mut metadata_map = extract_rtf_metadata(&rtf_content, &extracted_text);

        let title = metadata_map
            .remove(&Cow::Borrowed("title"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let subject = metadata_map
            .remove(&Cow::Borrowed("subject"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let authors = metadata_map.remove(&Cow::Borrowed("authors")).and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        });
        let created_by = metadata_map
            .remove(&Cow::Borrowed("created_by"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let modified_by = metadata_map
            .remove(&Cow::Borrowed("modified_by"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let created_at = metadata_map
            .remove(&Cow::Borrowed("created_at"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let modified_at = metadata_map
            .remove(&Cow::Borrowed("modified_at"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        let mut doc = Self::build_internal_document(&rtf_content, plain);

        // `from_utf8_lossy` above already replaced every undecodable byte with U+FFFD.
        // RTF producers routinely emit raw 8-bit bytes instead of `\'hh` escapes, so this
        // is a real and common way for characters to disappear without any error (#171).
        if std::str::from_utf8(content).is_err() {
            crate::core::diagnostics::push_lossy_decode_warning(
                &mut doc.processing_warnings,
                RTF_WARNING_SOURCE,
                "RTF source",
            );
        }

        if let Some(ref filter) = config.content_filter {
            use crate::types::document_structure::ContentLayer;
            doc.elements.retain(|elem| match elem.layer {
                ContentLayer::Header => filter.include_headers,
                ContentLayer::Footer => filter.include_footers,
                _ => true,
            });
        }

        doc.mime_type = mime_type.to_string();
        doc.metadata = Metadata {
            title,
            subject,
            authors,
            created_by,
            modified_by,
            created_at,
            modified_at,
            additional: metadata_map,
            ..Default::default()
        };

        tracing::debug!(
            element_count = doc.elements.len(),
            format = "rtf",
            "extraction complete"
        );
        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/rtf", "text/rtf"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests;
