//! PowerPoint presentation extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::security::SecurityBudget;
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::metadata::Metadata;
use crate::types::uri::ExtractedUri;
use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;
use std::path::Path;
#[cfg_attr(alef, alef(skip))]
/// PowerPoint presentation extractor.
///
/// Supports: .pptx, .pptm, .ppsx
pub struct PptxExtractor;

struct PptxMarkdownContext<'a> {
    forms: &'a [(String, String)],
    formulas: &'a [(String, bool)],
    plain_output: bool,
}

impl Default for PptxExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PptxExtractor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl PptxExtractor {
    /// Build an `InternalDocument` from PPTX extracted text.
    ///
    /// Parses each archive-derived slide independently so page metadata never
    /// depends on headings or marker-like user text.
    ///
    /// Try to strip an ordered-list prefix like `1. `, `2. `, `10. ` from a line.
    /// Returns the remaining text after the prefix, or `None` if the line does not
    /// start with a `<digits>. ` pattern.
    fn strip_ordered_prefix(line: &str) -> Option<&str> {
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == 0 || i + 2 > bytes.len() {
            return None;
        }
        if bytes[i] == b'.' && bytes[i + 1] == b' ' {
            Some(&line[i + 2..])
        } else {
            None
        }
    }

    /// Build the delimited forms of a deck's math runs, longest first.
    ///
    /// A math run reaches markdown text as `$$latex$$` (display) or `$latex$`
    /// (inline). Matching the exact strings the OMML converter produced keeps
    /// author text that merely holds a `$` out of the formula list.
    fn math_forms(formulas: &[(String, bool)]) -> Vec<(String, String)> {
        let mut forms: Vec<(String, String)> = formulas
            .iter()
            .map(|(latex, is_display)| {
                let delimiter = if *is_display { "$$" } else { "$" };
                (format!("{delimiter}{latex}{delimiter}"), latex.clone())
            })
            .collect();
        forms.sort_by_key(|form| std::cmp::Reverse(form.0.len()));
        forms
    }

    /// Pull the math spans out of one line of extracted text.
    ///
    /// Returns the line without its math and the LaTeX of every span removed, in
    /// the order the spans appeared.
    ///
    /// Plain text carries no delimiters, so there a line that is exactly one
    /// formula's LaTeX becomes that formula. This covers the equation shape a
    /// slide deck usually holds. Math mixed into a line of plain text stays in
    /// the line, because the LaTeX and the words around it are the same
    /// characters. Markdown text keeps its delimiters, so both shapes of math
    /// come out of it.
    fn split_line_math<'a>(
        line: &'a str,
        forms: &[(String, String)],
        formulas: &[(String, bool)],
        plain_output: bool,
    ) -> (Cow<'a, str>, Vec<String>) {
        if formulas.is_empty() {
            return (Cow::Borrowed(line), Vec::new());
        }
        if plain_output {
            let trimmed = line.trim();
            return match formulas.iter().find(|(latex, _)| latex.as_str() == trimmed) {
                Some((latex, _)) => (Cow::Borrowed(""), vec![latex.clone()]),
                None => (Cow::Borrowed(line), Vec::new()),
            };
        }
        if !line.contains('$') {
            return (Cow::Borrowed(line), Vec::new());
        }

        let mut rest = line;
        let mut text = String::new();
        let mut found: Vec<String> = Vec::new();
        loop {
            let earliest = forms
                .iter()
                .filter_map(|form| rest.find(form.0.as_str()).map(|pos| (pos, form)))
                .min_by_key(|(pos, _)| *pos);
            let Some((pos, form)) = earliest else {
                break;
            };
            text.push_str(&rest[..pos]);
            found.push(form.1.clone());
            rest = &rest[pos + form.0.len()..];
        }
        if found.is_empty() {
            return (Cow::Borrowed(line), Vec::new());
        }
        text.push_str(rest);

        (Cow::Owned(text.split_whitespace().collect::<Vec<_>>().join(" ")), found)
    }

    /// Emit a formula element per LaTeX string, in order.
    fn push_line_formulas(
        builder: &mut InternalDocumentBuilder,
        formulas: &[String],
        slide_num: u32,
        budget: &mut SecurityBudget,
    ) -> Result<()> {
        for latex in formulas {
            budget.account_text(latex.len())?;
            builder.push_formula(latex, Some(slide_num), None);
        }
        Ok(())
    }

    /// Add the OCR text anchor of a picture that gets no placeholder.
    ///
    /// Only the pipeline step that follows OCR of the embedded pictures turns an anchor into
    /// text, so a build without that step adds none. ~keep
    #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
    fn push_image_ocr_text_anchor(builder: &mut InternalDocumentBuilder, image_index: u32, slide_number: u32) {
        builder.push_element(crate::types::internal::InternalElement::image_ocr_text_anchor(
            image_index,
            Some(slide_number),
        ));
    }

    #[cfg(not(all(feature = "ocr", feature = "tokio-runtime")))]
    fn push_image_ocr_text_anchor(_builder: &mut InternalDocumentBuilder, _image_index: u32, _slide_number: u32) {}

    fn build_internal_document(
        slide_contents: &[crate::extraction::pptx::PptxInternalSlide],
        slide_count: u32,
        formulas: &[(String, bool)],
        plain_output: bool,
        ocr_text_anchors: bool,
        budget: &mut SecurityBudget,
    ) -> Result<InternalDocument> {
        let mut builder = InternalDocumentBuilder::new("pptx");
        let mut saw_title = false;
        let forms = Self::math_forms(formulas);
        let markdown_context = PptxMarkdownContext {
            forms: &forms,
            formulas,
            plain_output,
        };

        for slide in slide_contents {
            for element in &slide.elements {
                match element {
                    crate::extraction::pptx::PptxInternalSlideElement::Markdown(content) => {
                        Self::push_markdown_content(
                            &mut builder,
                            content,
                            slide.slide_number,
                            &markdown_context,
                            &mut saw_title,
                            budget,
                        )?;
                    }
                    crate::extraction::pptx::PptxInternalSlideElement::Image {
                        alt_text,
                        target,
                        image_index,
                    } => {
                        budget.step()?;
                        if let Some(image_index) = image_index {
                            budget.account_text(alt_text.len())?;
                            let element = crate::types::internal::InternalElement::text(
                                crate::types::internal::ElementKind::Image {
                                    image_index: *image_index,
                                },
                                alt_text,
                                0,
                            )
                            .with_page(slide.slide_number);
                            builder.push_element(element);
                        } else {
                            let placeholder = format!("![{alt_text}]({target})");
                            budget.account_text(placeholder.len())?;
                            builder.push_paragraph(&placeholder, vec![], Some(slide.slide_number), None);
                        }
                    }
                    crate::extraction::pptx::PptxInternalSlideElement::ImageOcrTextAnchor { image_index } => {
                        budget.step()?;
                        if ocr_text_anchors {
                            Self::push_image_ocr_text_anchor(&mut builder, *image_index, slide.slide_number);
                        }
                    }
                }
            }
        }

        // Preserve the legacy all-untitled output shape: it contains one Slide
        // sentinel, while titled decks do not gain new thematic-break/JSON nodes.
        if !saw_title && slide_count > 0 {
            builder.push_slide(1, None, Some(1));
        }

        Ok(builder.build())
    }

    fn push_markdown_content(
        builder: &mut InternalDocumentBuilder,
        content: &str,
        slide_num: u32,
        context: &PptxMarkdownContext<'_>,
        saw_title: &mut bool,
        budget: &mut SecurityBudget,
    ) -> Result<()> {
        let mut in_notes = false;
        for block in content.split("\n\n") {
            budget.step()?;
            let trimmed = block.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with("### Notes:") || trimmed == "Notes:" {
                in_notes = true;
                continue;
            }
            if let Some(title_text) = trimmed.strip_prefix("# ") {
                in_notes = false;
                *saw_title = true;
                let (title_text, title_formulas) =
                    Self::split_line_math(title_text, context.forms, context.formulas, context.plain_output);
                Self::push_line_formulas(builder, &title_formulas, slide_num, budget)?;
                let title = title_text.trim();
                if !title.is_empty() {
                    budget.account_text(title.len())?;
                    builder.push_heading(2, title, Some(slide_num), None);
                }
                continue;
            }
            if in_notes {
                in_notes = false;
            }
            if trimmed.starts_with('|') {
                let cells = Self::parse_markdown_table(trimmed);
                if !cells.is_empty() {
                    builder.push_table_from_cells(&cells, Some(slide_num), None);
                }
                continue;
            }

            let mut in_list: Option<bool> = None;
            for line in trimmed.lines() {
                let (line_text, line_formulas) =
                    Self::split_line_math(line, context.forms, context.formulas, context.plain_output);
                let lt = line_text.trim();
                if lt.is_empty() && line_formulas.is_empty() {
                    if in_list.is_some() {
                        builder.end_list();
                        in_list = None;
                    }
                    continue;
                }
                let list_match = if let Some(item_text) = lt.strip_prefix("- ") {
                    Some((false, item_text))
                } else {
                    Self::strip_ordered_prefix(lt).map(|item_text| (true, item_text))
                };
                if let Some((ordered, item_text)) = list_match {
                    match in_list {
                        Some(previous) if previous != ordered => {
                            builder.end_list();
                            builder.push_list(ordered);
                            in_list = Some(ordered);
                        }
                        None => {
                            builder.push_list(ordered);
                            in_list = Some(ordered);
                        }
                        _ => {}
                    }
                    Self::push_line_formulas(builder, &line_formulas, slide_num, budget)?;
                    budget.account_text(item_text.len())?;
                    builder.push_list_item(item_text, ordered, vec![], Some(slide_num), None);
                } else {
                    if in_list.is_some() {
                        builder.end_list();
                        in_list = None;
                    }
                    Self::push_line_formulas(builder, &line_formulas, slide_num, budget)?;
                    if !lt.is_empty() {
                        budget.account_text(lt.len())?;
                        builder.push_paragraph(lt, vec![], Some(slide_num), None);
                    }
                }
            }
            if in_list.is_some() {
                builder.end_list();
            }
        }
        Ok(())
    }

    /// Parse a markdown table block into a 2D cell grid.
    fn parse_markdown_table(table_text: &str) -> Vec<Vec<String>> {
        let mut cells = Vec::new();
        for line in table_text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.contains("---") {
                continue;
            }
            let row: Vec<String> = trimmed
                .trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_string())
                .collect();
            if !row.is_empty() {
                cells.push(row);
            }
        }
        cells
    }
}

impl PptxExtractor {
    /// Build an InternalDocument from a PptxExtractionResult, mapping office
    /// metadata to standard `Metadata` struct fields.
    ///
    /// `budget` is threaded into the internal document builder to enforce
    /// hostile-input limits on the extracted content.
    ///
    /// `ocr_text_anchors` is `ExtractionConfig::runs_ocr_on_embedded_images`, the one OCR
    /// gate: a picture with no placeholder gets an anchor for its OCR text only when the
    /// pictures go to OCR. ~keep
    fn build_document_from_result(
        pptx_internal: crate::extraction::pptx::PptxInternalExtraction,
        mime_type: &str,
        extract_images: bool,
        ocr_text_anchors: bool,
        budget: &mut SecurityBudget,
    ) -> Result<InternalDocument> {
        let crate::extraction::pptx::PptxInternalExtraction {
            result: pptx_result,
            slide_contents,
            formulas,
            plain_output,
        } = pptx_internal;
        let mut additional: AHashMap<Cow<'static, str>, serde_json::Value> = AHashMap::new();

        let mut pptx_metadata = pptx_result.metadata;
        pptx_metadata.image_count = Some(pptx_result.image_count as u32);
        pptx_metadata.table_count = Some(pptx_result.table_count as u32);

        let office_meta = &pptx_result.office_metadata;
        let title = office_meta.get("title").cloned();
        let subject = office_meta.get("subject").cloned();
        let created_by = office_meta.get("created_by").cloned();
        let modified_by = office_meta.get("modified_by").cloned();
        let created_at = office_meta.get("created_at").cloned();
        let modified_at = office_meta.get("modified_at").cloned();
        let authors = office_meta.get("author").map(|a| vec![a.clone()]);
        let keywords = office_meta.get("keywords").map(|k| {
            k.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        });

        for (key, value) in &pptx_result.office_metadata {
            match key.as_str() {
                "title" | "subject" | "created_by" | "modified_by" | "created_at" | "modified_at" | "author"
                | "keywords" => {}
                "slide_count" => {}
                "notes_count" | "hidden_slides" => {
                    let json_value = value
                        .parse::<u64>()
                        .map(|n| serde_json::Value::Number(n.into()))
                        .unwrap_or_else(|_| serde_json::json!(value));
                    additional.insert(Cow::Owned(key.clone()), json_value);
                }
                _ => {
                    additional.insert(Cow::Owned(key.clone()), serde_json::json!(value));
                }
            }
        }

        let mut doc = Self::build_internal_document(
            &slide_contents,
            pptx_result.slide_count as u32,
            &formulas,
            plain_output,
            ocr_text_anchors,
            budget,
        )?;
        doc.mime_type = mime_type.to_string();

        let mut metadata = Metadata {
            title,
            subject,
            authors,
            keywords,
            created_at,
            modified_at,
            created_by,
            modified_by,
            format: Some(crate::types::FormatMetadata::Pptx(pptx_metadata)),
            additional,
            ..Default::default()
        };

        if let Some(page_structure) = pptx_result.page_structure {
            metadata.pages = Some(page_structure);
        }

        doc.metadata = metadata;

        for hyperlink in pptx_result.hyperlinks {
            doc.push_uri(ExtractedUri::hyperlink(&hyperlink.url, hyperlink.label));
        }

        doc.prebuilt_pages = pptx_result.page_contents;

        doc.revisions = pptx_result.revisions;

        if extract_images {
            doc.images = pptx_result.images;
        }

        Ok(doc)
    }
}

impl Plugin for PptxExtractor {
    fn name(&self) -> &str {
        "pptx-extractor"
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
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for PptxExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        tracing::debug!(format = "pptx", size_bytes = content.len(), "extraction starting");
        let extract_images = config.needs_image_data();
        let inject_placeholders = config
            .images
            .as_ref()
            .map(|img| img.inject_placeholders)
            .unwrap_or(true);
        let plain = matches!(config.output_format, crate::core::config::OutputFormat::Plain);
        let security_limits = config.security_limits.clone().unwrap_or_default();
        let max_pages = security_limits.max_pages;

        let mut pptx_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();

        let pptx_internal = {
            #[cfg(feature = "tokio-runtime")]
            {
                if crate::core::batch_mode::is_batch_mode() {
                    if config.cancel_token.as_ref().map(|t| t.is_cancelled()).unwrap_or(false) {
                        return Err(crate::error::XbergError::Cancelled);
                    }
                    let content_owned = content.to_vec();
                    let options = crate::extraction::pptx::PptxExtractionOptions {
                        extract_images,
                        page_config: config.pages.clone(),
                        plain,
                        include_structure: false,
                        inject_placeholders,
                        security_limits: security_limits.clone(),
                        max_pages,
                    };
                    let span = tracing::Span::current();
                    let (result, warnings) = tokio::task::spawn_blocking(move || {
                        let _guard = span.entered();
                        let mut warnings = Vec::new();
                        let result = crate::extraction::pptx::extract_pptx_from_bytes_with_slide_contents(
                            &content_owned,
                            &options,
                            &mut warnings,
                        );
                        (result, warnings)
                    })
                    .await
                    .map_err(|e| crate::error::XbergError::parsing(format!("PPTX extraction task failed: {}", e)))?;
                    pptx_warnings = warnings;
                    result?
                } else {
                    let options = crate::extraction::pptx::PptxExtractionOptions {
                        extract_images,
                        page_config: config.pages.clone(),
                        plain,
                        include_structure: false,
                        inject_placeholders,
                        security_limits: security_limits.clone(),
                        max_pages,
                    };
                    crate::extraction::pptx::extract_pptx_from_bytes_with_slide_contents(
                        content,
                        &options,
                        &mut pptx_warnings,
                    )?
                }
            }

            #[cfg(not(feature = "tokio-runtime"))]
            {
                let options = crate::extraction::pptx::PptxExtractionOptions {
                    extract_images,
                    page_config: config.pages.clone(),
                    plain,
                    include_structure: false,
                    inject_placeholders,
                    security_limits: security_limits.clone(),
                    max_pages,
                };
                crate::extraction::pptx::extract_pptx_from_bytes_with_slide_contents(
                    content,
                    &options,
                    &mut pptx_warnings,
                )?
            }
        };

        let mut budget = SecurityBudget::from_config(config);
        let mut doc = Self::build_document_from_result(
            pptx_internal,
            mime_type,
            extract_images,
            config.runs_ocr_on_embedded_images(),
            &mut budget,
        )?;
        doc.processing_warnings.extend(pptx_warnings);

        if config.max_archive_depth > 0 {
            let (children, embed_warnings) = crate::extraction::ooxml_embedded::extract_ooxml_embedded_objects(
                content,
                "ppt/embeddings/",
                "pptx",
                config,
            )
            .await;
            if !children.is_empty() {
                doc.children = Some(children);
            }
            doc.processing_warnings.extend(embed_warnings);
        }

        tracing::debug!(
            element_count = doc.elements.len(),
            format = "pptx",
            "extraction complete"
        );
        Ok(doc)
    }

    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, path, config),
        fields(
            extractor.name = self.name(),
        )
    ))]
    async fn extract_path(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        let path_str = path
            .to_str()
            .ok_or_else(|| crate::XbergError::validation("Invalid file path".to_string()))?;

        let extract_images = config.needs_image_data();
        let inject_placeholders = config
            .images
            .as_ref()
            .map(|img| img.inject_placeholders)
            .unwrap_or(true);
        let plain = matches!(config.output_format, crate::core::config::OutputFormat::Plain);
        let security_limits = config.security_limits.clone().unwrap_or_default();

        let options = crate::extraction::pptx::PptxExtractionOptions {
            extract_images,
            page_config: config.pages.clone(),
            plain,
            include_structure: false,
            inject_placeholders,
            security_limits: security_limits.clone(),
            max_pages: security_limits.max_pages,
        };
        let mut pptx_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();
        let pptx_internal = crate::extraction::pptx::extract_pptx_from_path_with_slide_contents(
            path_str,
            &options,
            &mut pptx_warnings,
        )?;

        let mut budget = SecurityBudget::from_config(config);
        let mut doc = Self::build_document_from_result(
            pptx_internal,
            mime_type,
            extract_images,
            config.runs_ocr_on_embedded_images(),
            &mut budget,
        )?;
        doc.processing_warnings.extend(pptx_warnings);
        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            "application/vnd.ms-powerpoint.presentation.macroEnabled.12",
            "application/vnd.openxmlformats-officedocument.presentationml.slideshow",
            "application/vnd.openxmlformats-officedocument.presentationml.template",
            "application/vnd.ms-powerpoint.template.macroEnabled.12",
        ]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn markdown_slides(contents: Vec<(u32, String)>) -> Vec<crate::extraction::pptx::PptxInternalSlide> {
        contents
            .into_iter()
            .map(|(slide_number, markdown)| crate::extraction::pptx::PptxInternalSlide {
                slide_number,
                elements: vec![crate::extraction::pptx::PptxInternalSlideElement::Markdown(markdown)],
            })
            .collect()
    }

    /// REV-CB regression for GH#1687 (shares the GH#1662/GH#1686 fix): an OCR-only
    /// `ExtractionConfig` (no `images.extract_images`, no captioning, no QR codes) must
    /// still read a slide's embedded raster image bytes out of the PPTX archive, not
    /// skip it. `needs_image_data` gained the OCR disjunct that makes this true (#1662);
    /// PPTX shares that predicate with DOCX and HTML through
    /// `PptxExtractor::extract_content`'s `config.needs_image_data()` call, but until now
    /// nothing exercised that call site directly for PPTX. Before the fix, `extract_images`
    /// stayed `false` for an OCR-only config, so the slide-image loop in
    /// `extraction::pptx::extract_pptx_from_bytes_with_slide_contents` never ran at all and
    /// `doc.images` stayed empty, silently, with no warning.
    #[tokio::test]
    async fn test_pptx_ocr_only_config_reads_real_embedded_image_bytes() {
        use crate::core::config::ExtractionConfig;
        use crate::plugins::InternalDocumentExtractor;

        let payload = "PNGPAYLOAD".repeat(64);
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
    <p:cSld><p:spTree>
        <p:pic>
            <p:nvPicPr><p:cNvPr id="2" name="Picture 1"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
            <p:blipFill><a:blip r:embed="rId2"/></p:blipFill>
            <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm></p:spPr>
        </p:pic>
    </p:spTree></p:cSld>
</p:sld>"#;
        let slide_rels_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
</Relationships>"#;

        let pptx = crate::extraction::pptx::tests::build_single_slide_pptx(
            slide_xml,
            Some(slide_rels_xml),
            &[("ppt/media/image1.png", payload.as_bytes())],
        );

        let config = ExtractionConfig {
            ocr: Some(crate::core::config::OcrConfig::default()),
            output_format: crate::core::config::OutputFormat::Markdown,
            ..Default::default()
        };

        let extractor = PptxExtractor::new();
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let internal_doc = extractor
            .extract_content(&pptx, mime, &config)
            .await
            .expect("a pptx with one embedded image must extract");

        assert_eq!(internal_doc.images.len(), 1, "the single picture must yield one image");
        assert_eq!(
            internal_doc.images[0].data.as_ref(),
            payload.as_bytes(),
            "an OCR-only config must still read the real embedded-image bytes, not skip the image entirely"
        );
        assert!(internal_doc.elements.iter().any(|element| {
            matches!(
                element.kind,
                crate::types::internal::ElementKind::Image { image_index: 0 }
            )
        }));
    }

    #[tokio::test]
    async fn test_image_placeholders_follow_visual_order_not_xml_order() {
        use crate::core::config::{ExtractionConfig, ImageExtractionConfig, OutputFormat};
        use crate::plugins::InternalDocumentExtractor;
        use crate::types::internal::ElementKind;

        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
    <p:cSld><p:spTree>
        <p:pic>
            <p:nvPicPr><p:cNvPr id="2" name="Bottom" descr="Bottom alt"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
            <p:blipFill><a:blip r:embed="rId2"/></p:blipFill>
            <p:spPr><a:xfrm><a:off x="0" y="2000000"/><a:ext cx="1000000" cy="1000000"/></a:xfrm></p:spPr>
        </p:pic>
        <p:pic>
            <p:nvPicPr><p:cNvPr id="3" name="Top" descr="Top alt"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
            <p:blipFill><a:blip r:embed="rId3"/></p:blipFill>
            <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm></p:spPr>
        </p:pic>
    </p:spTree></p:cSld>
</p:sld>"#;
        let slide_rels_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/bottom.png"/>
    <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/top.png"/>
</Relationships>"#;
        let bottom_bytes = b"bottom image bytes";
        let top_bytes = b"top image bytes";
        let pptx = crate::extraction::pptx::tests::build_single_slide_pptx(
            slide_xml,
            Some(slide_rels_xml),
            &[("ppt/media/bottom.png", bottom_bytes), ("ppt/media/top.png", top_bytes)],
        );
        let config = ExtractionConfig {
            images: Some(ImageExtractionConfig::default()),
            output_format: OutputFormat::Markdown,
            ..Default::default()
        };

        let extractor = PptxExtractor::new();
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let mut document = extractor
            .extract_content(&pptx, mime, &config)
            .await
            .expect("two-image PPTX should extract");

        assert_eq!(document.images.len(), 2);
        assert_eq!(document.images[0].data.as_ref(), top_bytes);
        assert_eq!(document.images[0].description.as_deref(), Some("Top alt"));
        assert_eq!(document.images[1].data.as_ref(), bottom_bytes);
        assert_eq!(document.images[1].description.as_deref(), Some("Bottom alt"));
        let image_indices: Vec<u32> = document
            .elements
            .iter()
            .filter_map(|element| match element.kind {
                ElementKind::Image { image_index } => Some(image_index),
                _ => None,
            })
            .collect();
        assert_eq!(image_indices, vec![0, 1]);

        document.images[0].description = Some("Top caption".to_string());
        document.images[1].description = Some("Bottom caption".to_string());
        let markdown = crate::rendering::render_markdown(&document);
        let top = markdown.find("Top caption").expect("top image caption should render");
        let bottom = markdown
            .find("Bottom caption")
            .expect("bottom image caption should render");
        assert!(top < bottom, "captions must stay attached in visual order: {markdown}");
    }

    /// A slide with math: the LaTeX must reach `ExtractedDocument.formulas`, not
    /// only the text. The deck holds display math in its own shape, inline math
    /// beside text, and a `$` amount that is not math at all.
    #[tokio::test]
    async fn test_slide_math_populates_formulas() {
        use crate::core::config::ExtractionConfig;
        use crate::extraction::derive::derive_extraction_result;
        use crate::plugins::InternalDocumentExtractor;

        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
       xmlns:a14="http://schemas.microsoft.com/office/drawing/2010/main"
       xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
    <p:cSld><p:spTree>
        <p:sp><p:txBody>
            <a:p><a:r><a:t>Budget is $5 per unit</a:t></a:r></a:p>
        </p:txBody></p:sp>
        <p:sp><p:txBody>
            <a:p>
                <mc:AlternateContent>
                    <mc:Choice Requires="a14"><a14:m>
                        <m:oMathPara><m:oMath><m:sSup>
                            <m:e><m:r><m:t>x</m:t></m:r></m:e>
                            <m:sup><m:r><m:t>2</m:t></m:r></m:sup>
                        </m:sSup></m:oMath></m:oMathPara>
                    </a14:m></mc:Choice>
                    <mc:Fallback><a:r><a:t>[equation]</a:t></a:r></mc:Fallback>
                </mc:AlternateContent>
            </a:p>
            <a:p>
                <a:r><a:t>Rate </a:t></a:r>
                <mc:AlternateContent>
                    <mc:Choice Requires="a14"><a14:m>
                        <m:oMath><m:r><m:t>a</m:t></m:r></m:oMath>
                    </a14:m></mc:Choice>
                    <mc:Fallback><a:r><a:t>[a]</a:t></a:r></mc:Fallback>
                </mc:AlternateContent>
                <a:r><a:t> per hour</a:t></a:r>
            </a:p>
        </p:txBody></p:sp>
    </p:spTree></p:cSld>
</p:sld>"#;

        let pptx = crate::extraction::pptx::tests::build_single_slide_pptx(slide_xml, None, &[]);
        let extractor = PptxExtractor::new();
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let config = ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Markdown,
            ..Default::default()
        };
        let internal_doc = extractor
            .extract_content(&pptx, mime, &config)
            .await
            .expect("extraction failed");
        let result = derive_extraction_result(internal_doc, false, crate::core::config::OutputFormat::Markdown);

        let latex: Vec<&str> = result.formulas.iter().map(|f| f.latex.as_str()).collect();
        assert_eq!(latex, vec!["x^{2}", "a"], "both math runs reach formulas");
        assert!(
            result.content.contains("Budget is $5 per unit"),
            "a dollar amount stays text, got: {:?}",
            result.content
        );
        assert!(
            result.content.contains("Rate per hour"),
            "the text around inline math survives, got: {:?}",
            result.content
        );
    }

    /// A deck written by a tool other than PowerPoint puts the math straight
    /// into the paragraph, with no `a14:m` wrapper and no `mc:AlternateContent`.
    #[tokio::test]
    async fn test_bare_omml_in_a_paragraph_populates_formulas() {
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
    <p:cSld><p:spTree>
        <p:sp><p:txBody>
            <a:p>
                <m:oMathPara><m:oMath><m:sSup>
                    <m:e><m:r><m:t>y</m:t></m:r></m:e>
                    <m:sup><m:r><m:t>3</m:t></m:r></m:sup>
                </m:sSup></m:oMath></m:oMathPara>
            </a:p>
        </p:txBody></p:sp>
    </p:spTree></p:cSld>
</p:sld>"#;

        assert_eq!(slide_formulas(slide_xml).await, vec!["y^{3}"]);
    }

    /// PowerPoint writes the equation as `a14:m` in its 2010 drawing namespace,
    /// with no compatibility wrapper. A real deck extracted to nothing before.
    #[tokio::test]
    async fn test_drawing_extension_math_without_a_compatibility_wrapper() {
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a14="http://schemas.microsoft.com/office/drawing/2010/main"
       xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
    <p:cSld><p:spTree>
        <p:sp><p:txBody>
            <a:p><a14:m>
                <m:oMathPara><m:oMath><m:sSup>
                    <m:e><m:r><m:t>e</m:t></m:r></m:e>
                    <m:sup><m:r><m:t>x</m:t></m:r></m:sup>
                </m:sSup></m:oMath></m:oMathPara>
            </a14:m></a:p>
        </p:txBody></p:sp>
    </p:spTree></p:cSld>
</p:sld>"#;

        assert_eq!(slide_formulas(slide_xml).await, vec!["e^{x}"]);
    }

    /// Bare inline math sits beside the words of its sentence. The equation
    /// becomes a formula and the words keep their spacing.
    #[tokio::test]
    async fn test_bare_inline_omml_leaves_the_sentence_intact() {
        use crate::core::config::ExtractionConfig;
        use crate::extraction::derive::derive_extraction_result;
        use crate::plugins::InternalDocumentExtractor;

        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
    <p:cSld><p:spTree>
        <p:sp><p:txBody>
            <a:p>
                <a:r><a:t>Speed </a:t></a:r>
                <m:oMath><m:r><m:t>v</m:t></m:r></m:oMath>
                <a:r><a:t> in metres</a:t></a:r>
            </a:p>
        </p:txBody></p:sp>
    </p:spTree></p:cSld>
</p:sld>"#;

        let pptx = crate::extraction::pptx::tests::build_single_slide_pptx(slide_xml, None, &[]);
        let extractor = PptxExtractor::new();
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let config = ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Markdown,
            ..Default::default()
        };
        let internal_doc = extractor
            .extract_content(&pptx, mime, &config)
            .await
            .expect("extraction failed");
        let result = derive_extraction_result(internal_doc, false, crate::core::config::OutputFormat::Markdown);

        let latex: Vec<&str> = result.formulas.iter().map(|f| f.latex.as_str()).collect();
        assert_eq!(latex, vec!["v"], "the inline equation becomes a formula");
        assert!(
            result.content.contains("Speed in metres"),
            "the sentence keeps one space where the equation left it, got: {:?}",
            result.content
        );
    }

    /// Extract one slide and return the LaTeX of every formula it yields.
    async fn slide_formulas(slide_xml: &str) -> Vec<String> {
        use crate::core::config::ExtractionConfig;
        use crate::extraction::derive::derive_extraction_result;
        use crate::plugins::InternalDocumentExtractor;

        let pptx = crate::extraction::pptx::tests::build_single_slide_pptx(slide_xml, None, &[]);
        let extractor = PptxExtractor::new();
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let config = ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Markdown,
            ..Default::default()
        };
        let internal_doc = extractor
            .extract_content(&pptx, mime, &config)
            .await
            .expect("extraction failed");
        derive_extraction_result(internal_doc, false, crate::core::config::OutputFormat::Markdown)
            .formulas
            .iter()
            .map(|f| f.latex.clone())
            .collect()
    }

    /// Plain output carries no math delimiters, so a shape that holds nothing but
    /// math is still recognized by its exact LaTeX. Math mixed into a line of text
    /// stays in that line.
    #[tokio::test]
    async fn test_standalone_slide_math_populates_formulas_in_plain_output() {
        use crate::core::config::ExtractionConfig;
        use crate::extraction::derive::derive_extraction_result;
        use crate::plugins::InternalDocumentExtractor;

        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
       xmlns:a14="http://schemas.microsoft.com/office/drawing/2010/main"
       xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
    <p:cSld><p:spTree>
        <p:sp><p:txBody>
            <a:p><a:r><a:t>Energy of a body at rest</a:t></a:r></a:p>
        </p:txBody></p:sp>
        <p:sp><p:txBody>
            <a:p>
                <mc:AlternateContent>
                    <mc:Choice Requires="a14"><a14:m>
                        <m:oMathPara><m:oMath><m:sSup>
                            <m:e><m:r><m:t>x</m:t></m:r></m:e>
                            <m:sup><m:r><m:t>2</m:t></m:r></m:sup>
                        </m:sSup></m:oMath></m:oMathPara>
                    </a14:m></mc:Choice>
                    <mc:Fallback><a:r><a:t>[equation]</a:t></a:r></mc:Fallback>
                </mc:AlternateContent>
            </a:p>
        </p:txBody></p:sp>
    </p:spTree></p:cSld>
</p:sld>"#;

        let pptx = crate::extraction::pptx::tests::build_single_slide_pptx(slide_xml, None, &[]);
        let extractor = PptxExtractor::new();
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let internal_doc = extractor
            .extract_content(&pptx, mime, &ExtractionConfig::default())
            .await
            .expect("extraction failed");
        let result = derive_extraction_result(internal_doc, false, crate::core::config::OutputFormat::Plain);

        let latex: Vec<&str> = result.formulas.iter().map(|f| f.latex.as_str()).collect();
        assert_eq!(latex, vec!["x^{2}"]);
    }

    #[test]
    fn test_split_line_math_pulls_delimited_spans() {
        let formulas = vec![("x^{2}".to_string(), true), ("a".to_string(), false)];
        let forms = PptxExtractor::math_forms(&formulas);

        let (text, found) = PptxExtractor::split_line_math("Rate $a$ per hour", &forms, &formulas, false);
        assert_eq!(text, "Rate per hour");
        assert_eq!(found, vec!["a".to_string()]);

        let (text, found) = PptxExtractor::split_line_math("$$x^{2}$$", &forms, &formulas, false);
        assert_eq!(text, "");
        assert_eq!(found, vec!["x^{2}".to_string()]);
    }

    #[test]
    fn test_split_line_math_keeps_plain_dollar_text() {
        let formulas = vec![("a".to_string(), false)];
        let forms = PptxExtractor::math_forms(&formulas);

        let (text, found) = PptxExtractor::split_line_math("Budget is $5 per unit", &forms, &formulas, false);
        assert_eq!(text, "Budget is $5 per unit");
        assert!(found.is_empty(), "author text with a dollar sign is not math");
    }

    #[test]
    fn test_split_line_math_matches_undelimited_plain_output() {
        let formulas = vec![("x^{2}".to_string(), true)];
        let forms = PptxExtractor::math_forms(&formulas);

        let (text, found) = PptxExtractor::split_line_math("x^{2}", &forms, &formulas, true);
        assert_eq!(text, "");
        assert_eq!(found, vec!["x^{2}".to_string()], "plain output carries no delimiters");
    }

    /// Markdown text keeps its delimiters, so a line that merely repeats a
    /// formula's characters is author text, not math.
    #[test]
    fn test_split_line_math_leaves_undelimited_line_in_markdown_text() {
        let formulas = vec![("n".to_string(), false)];
        let forms = PptxExtractor::math_forms(&formulas);

        let (text, found) = PptxExtractor::split_line_math("n", &forms, &formulas, false);
        assert_eq!(text, "n");
        assert!(found.is_empty());
    }

    /// Math inside a bulleted line becomes its own element, and the bullet keeps
    /// its words.
    #[test]
    fn test_build_internal_document_lifts_list_item_and_title_math() {
        use crate::types::internal::ElementKind;

        let content = "# Growth is $$g^{2}$$\n\n- Rate $r$ per year\n- Plain bullet\n";
        let formulas = vec![("g^{2}".to_string(), true), ("r".to_string(), false)];
        let mut budget = SecurityBudget::with_defaults();
        let slides = markdown_slides(vec![(1, content.to_string())]);
        let doc = PptxExtractor::build_internal_document(&slides, 1, &formulas, false, false, &mut budget).unwrap();

        let math: Vec<&str> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Formula))
            .map(|e| e.text.as_str())
            .collect();
        assert_eq!(math, vec!["g^{2}", "r"], "title and list-item math both emit");

        let items: Vec<&str> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::ListItem { .. }))
            .map(|e| e.text.as_str())
            .collect();
        assert_eq!(items, vec!["Rate per year", "Plain bullet"]);

        let headings: Vec<&str> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Heading { .. }))
            .map(|e| e.text.as_str())
            .collect();
        assert_eq!(headings, vec!["Growth is"], "the heading keeps its words");
    }

    #[test]
    fn test_build_internal_document_keeps_image_placeholder_linked_to_extracted_image() {
        use crate::types::internal::ElementKind;

        let slide_contents = vec![crate::extraction::pptx::PptxInternalSlide {
            slide_number: 1,
            elements: vec![
                crate::extraction::pptx::PptxInternalSlideElement::Markdown("Text before.".to_string()),
                crate::extraction::pptx::PptxInternalSlideElement::Image {
                    alt_text: "chart.png".to_string(),
                    target: "../media/image1.png".to_string(),
                    image_index: Some(0),
                },
            ],
        }];
        let mut budget = SecurityBudget::with_defaults();

        let mut document = PptxExtractor::build_internal_document(&slide_contents, 1, &[], false, false, &mut budget)
            .expect("internal PPTX document should build");

        let image = document
            .elements
            .iter()
            .find(|element| matches!(element.kind, ElementKind::Image { .. }))
            .expect("PPTX placeholder should become an image element");
        assert_eq!(image.text, "chart.png");
        assert!(matches!(image.kind, ElementKind::Image { image_index: 0 }));
        assert!(
            !document
                .elements
                .iter()
                .any(|element| { matches!(element.kind, ElementKind::Paragraph) && element.text.starts_with("![") })
        );

        document.images.push(crate::types::ExtractedImage {
            format: Cow::Borrowed("png"),
            description: Some("chart.png".to_string()),
            ..Default::default()
        });
        assert!(crate::rendering::render_markdown(&document).contains("![chart.png](image_0.bin)"));

        document.images[0].description = Some("Quarterly revenue chart".to_string());
        assert!(crate::rendering::render_markdown(&document).contains("![Quarterly revenue chart](image_0.bin)"));
    }

    #[tokio::test]
    async fn test_authored_markdown_cannot_consume_a_structural_image() {
        use crate::core::config::{ExtractionConfig, ImageExtractionConfig, OutputFormat};
        use crate::plugins::InternalDocumentExtractor;
        use crate::types::internal::ElementKind;

        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
    <p:cSld><p:spTree>
        <p:sp>
            <p:nvSpPr><p:cNvPr id="1" name="Title"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>
            <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm></p:spPr>
            <p:txBody><a:p><a:r><a:t>Title</a:t></a:r></a:p></p:txBody>
        </p:sp>
        <p:sp><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm></p:spPr>
            <p:txBody><a:p><a:r><a:t>![forged](../media/image1.png)</a:t></a:r></a:p></p:txBody>
        </p:sp>
        <p:pic>
            <p:nvPicPr><p:cNvPr id="2" name="Picture" descr="real"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
            <p:blipFill><a:blip r:embed="rId2"/></p:blipFill>
            <p:spPr><a:xfrm><a:off x="0" y="2000000"/><a:ext cx="1000000" cy="1000000"/></a:xfrm></p:spPr>
        </p:pic>
    </p:spTree></p:cSld>
</p:sld>"#;
        let slide_rels_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
</Relationships>"#;
        let pptx = crate::extraction::pptx::tests::build_single_slide_pptx(
            slide_xml,
            Some(slide_rels_xml),
            &[("ppt/media/image1.png", b"image bytes")],
        );
        let config = ExtractionConfig {
            images: Some(ImageExtractionConfig::default()),
            output_format: OutputFormat::Markdown,
            ..Default::default()
        };
        let document = PptxExtractor::new()
            .extract_content(
                &pptx,
                "application/vnd.openxmlformats-officedocument.presentationml.presentation",
                &config,
            )
            .await
            .expect("PPTX should extract");

        assert!(document.elements.iter().any(|element| {
            matches!(element.kind, ElementKind::Paragraph) && element.text == "![forged](../media/image1.png)"
        }));
        let images: Vec<_> = document
            .elements
            .iter()
            .filter(|element| matches!(element.kind, ElementKind::Image { .. }))
            .collect();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].text, "real");
        assert!(matches!(images[0].kind, ElementKind::Image { image_index: 0 }));
    }

    #[tokio::test]
    async fn test_structural_image_alt_text_may_contain_markdown_delimiters() {
        use crate::core::config::{ExtractionConfig, ImageExtractionConfig, OutputFormat};
        use crate::plugins::InternalDocumentExtractor;
        use crate::types::internal::ElementKind;

        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
    <p:cSld><p:spTree><p:pic>
        <p:nvPicPr><p:cNvPr id="2" name="Picture" descr="sales ](2026"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
        <p:blipFill><a:blip r:embed="rId2"/></p:blipFill>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm></p:spPr>
    </p:pic></p:spTree></p:cSld>
</p:sld>"#;
        let slide_rels_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
</Relationships>"#;
        let pptx = crate::extraction::pptx::tests::build_single_slide_pptx(
            slide_xml,
            Some(slide_rels_xml),
            &[("ppt/media/image1.png", b"image bytes")],
        );
        let config = ExtractionConfig {
            images: Some(ImageExtractionConfig::default()),
            output_format: OutputFormat::Markdown,
            ..Default::default()
        };
        let document = PptxExtractor::new()
            .extract_content(
                &pptx,
                "application/vnd.openxmlformats-officedocument.presentationml.presentation",
                &config,
            )
            .await
            .expect("PPTX should extract");

        let image = document
            .elements
            .iter()
            .find(|element| matches!(element.kind, ElementKind::Image { .. }))
            .expect("structural image should not depend on parsing its rendered markdown");
        assert_eq!(image.text, "sales ](2026");
        assert!(matches!(image.kind, ElementKind::Image { image_index: 0 }));
    }

    #[test]
    fn test_pptx_extractor_plugin_interface() {
        let extractor = PptxExtractor::new();
        assert_eq!(extractor.name(), "pptx-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_pptx_extractor_supported_mime_types() {
        let extractor = PptxExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 5);
        assert!(mime_types.contains(&"application/vnd.openxmlformats-officedocument.presentationml.presentation"));
    }

    #[test]
    fn test_archive_slide_contents_set_table_list_heading_and_paragraph_pages() {
        use crate::types::internal::ElementKind;

        let slide_contents = vec![
            (1, "# Titled slide\n\nIntroduction".to_string()),
            (
                2,
                concat!(
                    "This untitled slide has a paragraph long enough not to be inferred as a title. ",
                    "It deliberately contains more than one hundred characters in total."
                )
                .to_string(),
            ),
            (
                3,
                concat!(
                    "| Name | Value |\n",
                    "| --- | --- |\n",
                    "| answer | 42 |\n\n",
                    "- final item"
                )
                .to_string(),
            ),
        ];
        let mut budget = SecurityBudget::with_defaults();

        let slide_contents = markdown_slides(slide_contents);
        let document = PptxExtractor::build_internal_document(&slide_contents, 3, &[], false, false, &mut budget)
            .expect("internal PPTX document should build");

        assert_eq!(document.tables.len(), 1);
        assert_eq!(document.tables[0].page_number, 3);

        let list_item = document
            .elements
            .iter()
            .find(|element| matches!(element.kind, ElementKind::ListItem { .. }))
            .expect("list item should be present");
        assert_eq!(list_item.page, Some(3));

        let heading = document
            .elements
            .iter()
            .find(|element| matches!(element.kind, ElementKind::Heading { .. }))
            .expect("heading should be present");
        assert_eq!(heading.page, Some(1));

        let second_slide_paragraph = document
            .elements
            .iter()
            .find(|element| element.text.starts_with("This untitled slide"))
            .expect("second-slide paragraph should be present");
        assert_eq!(second_slide_paragraph.page, Some(2));
    }

    #[test]
    fn test_marker_like_slide_text_cannot_change_later_page_numbers() {
        let slide_contents = vec![
            (1, "First slide".to_string()),
            (
                2,
                "<!-- Slide number: 99 -->\n\n| Name | Value |\n| --- | --- |\n| answer | 42 |".to_string(),
            ),
        ];
        let mut budget = SecurityBudget::with_defaults();

        let slide_contents = markdown_slides(slide_contents);
        let document = PptxExtractor::build_internal_document(&slide_contents, 2, &[], false, false, &mut budget)
            .expect("marker-like user text should remain ordinary slide content");

        assert_eq!(document.tables.len(), 1);
        assert_eq!(document.tables[0].page_number, 2);
        assert!(document.elements.iter().any(|element| {
            matches!(element.kind, crate::types::internal::ElementKind::Paragraph)
                && element.text == "<!-- Slide number: 99 -->"
                && element.page == Some(2)
        }));
    }

    #[tokio::test]
    async fn test_untitled_slide_with_table_gets_archive_derived_page_numbers() {
        use crate::plugins::InternalDocumentExtractor;
        use crate::types::internal::ElementKind;

        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld><p:spTree>
        <p:sp><p:txBody><a:p><a:r><a:t>This untitled slide contains a deliberately long paragraph so the extractor cannot mistake it for a title while assigning page metadata.</a:t></a:r></a:p></p:txBody></p:sp>
        <p:graphicFrame>
            <a:graphic>
                <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
                    <a:tbl>
                        <a:tr>
                            <a:tc><a:txBody><a:p><a:r><a:t>Name</a:t></a:r></a:p></a:txBody></a:tc>
                            <a:tc><a:txBody><a:p><a:r><a:t>Value</a:t></a:r></a:p></a:txBody></a:tc>
                        </a:tr>
                        <a:tr>
                            <a:tc><a:txBody><a:p><a:r><a:t>answer</a:t></a:r></a:p></a:txBody></a:tc>
                            <a:tc><a:txBody><a:p><a:r><a:t>42</a:t></a:r></a:p></a:txBody></a:tc>
                        </a:tr>
                    </a:tbl>
                </a:graphicData>
            </a:graphic>
        </p:graphicFrame>
    </p:spTree></p:cSld>
</p:sld>"#;
        let pptx = crate::extraction::pptx::tests::build_single_slide_pptx(slide_xml, None, &[]);
        let extractor = PptxExtractor::new();
        let config = ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Markdown,
            ..Default::default()
        };
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";

        let document = extractor
            .extract_content(&pptx, mime, &config)
            .await
            .expect("real PPTX extraction should succeed");

        assert_eq!(document.tables.len(), 1);
        assert_eq!(document.tables[0].page_number, 1);
        assert!(document.elements.iter().any(|element| {
            matches!(element.kind, ElementKind::Paragraph)
                && element.text.starts_with("This untitled slide")
                && element.page == Some(1)
        }));
        assert!(
            document
                .elements
                .iter()
                .any(|element| { matches!(element.kind, ElementKind::Slide { number: 1 }) && element.page == Some(1) })
        );
    }

    /// Full round-trip through PptxExtractor::extract_bytes → derive_extraction_result →
    /// ExtractedDocument.pages, asserting that speaker_notes and section_name are present.
    #[tokio::test]
    async fn test_extract_bytes_populates_speaker_notes_and_section_name() {
        use crate::core::config::{ExtractionConfig, PageConfig};
        use crate::extraction::derive::derive_extraction_result;
        use crate::plugins::InternalDocumentExtractor;

        let pptx = crate::extraction::pptx::tests::create_pptx_with_sections_and_notes(
            &[
                ("Title", Some("Intro notes.")),
                ("Body", Some("Body notes.")),
                ("End", None),
            ],
            &[("Chapter 1", &[1, 2]), ("Chapter 2", &[3])],
        );

        let extractor = PptxExtractor::new();
        let config = ExtractionConfig {
            pages: Some(PageConfig::default()),
            ..Default::default()
        };
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let internal_doc = extractor
            .extract_content(&pptx, mime, &config)
            .await
            .expect("extraction failed");
        let result = derive_extraction_result(internal_doc, true, crate::core::config::OutputFormat::Plain);

        assert!(
            !result.content.contains("<!-- Slide number:"),
            "internal slide markers must not leak into rendered output"
        );
        let pages = result.pages.as_ref().expect("pages should be populated");
        assert_eq!(pages.len(), 3);

        assert_eq!(pages[0].speaker_notes.as_deref(), Some("Intro notes."));
        assert_eq!(pages[0].section_name.as_deref(), Some("Chapter 1"));

        assert_eq!(pages[1].speaker_notes.as_deref(), Some("Body notes."));
        assert_eq!(pages[1].section_name.as_deref(), Some("Chapter 1"));

        assert!(pages[2].speaker_notes.is_none());
        assert_eq!(pages[2].section_name.as_deref(), Some("Chapter 2"));
    }

    /// GH#639: PPTX had no top-level archive entry-count check at all, so this fails
    /// against the unfixed code regardless of the configured limit (it never raises).
    #[tokio::test]
    async fn test_pptx_extract_content_honours_configured_archive_entry_limit() {
        use crate::core::config::ExtractionConfig;

        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>Hello</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld>
</p:sld>"#;
        let extra_parts: Vec<(String, Vec<u8>)> = (0..5)
            .map(|i| (format!("ppt/extra_{}.xml", i), b"<x/>".to_vec()))
            .collect();
        let extra_refs: Vec<(&str, &[u8])> = extra_parts.iter().map(|(p, d)| (p.as_str(), d.as_slice())).collect();
        let pptx = crate::extraction::pptx::tests::build_single_slide_pptx(slide_xml, None, &extra_refs);

        let extractor = PptxExtractor::new();
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let config = ExtractionConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_files_in_archive: 3,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extractor.extract_content(&pptx, mime, &config).await;
        assert!(
            result.is_err(),
            "an archive with more entries than the configured max_files_in_archive must be rejected"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains('3'),
            "error should mention the configured limit (3), got: {}",
            err_msg
        );
    }

    /// Sibling of the rejection test above: the same archive shape, but under a
    /// configured limit that comfortably fits it, must still extract successfully.
    #[tokio::test]
    async fn test_pptx_extract_content_succeeds_under_configured_archive_entry_limit() {
        use crate::core::config::ExtractionConfig;

        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>Hello</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld>
</p:sld>"#;
        let extra_parts: Vec<(String, Vec<u8>)> = (0..5)
            .map(|i| (format!("ppt/extra_{}.xml", i), b"<x/>".to_vec()))
            .collect();
        let extra_refs: Vec<(&str, &[u8])> = extra_parts.iter().map(|(p, d)| (p.as_str(), d.as_slice())).collect();
        let pptx = crate::extraction::pptx::tests::build_single_slide_pptx(slide_xml, None, &extra_refs);

        let extractor = PptxExtractor::new();
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let config = ExtractionConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_files_in_archive: 50,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extractor.extract_content(&pptx, mime, &config).await;
        assert!(
            result.is_ok(),
            "an archive within the configured max_files_in_archive must extract successfully: {:?}",
            result.err()
        );
    }

    /// A normal presentation with no `security_limits` override must still extract
    /// successfully under the default `SecurityLimits::max_files_in_archive`.
    #[tokio::test]
    async fn test_pptx_extract_content_succeeds_under_default_archive_entry_limit() {
        use crate::core::config::ExtractionConfig;

        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>Default</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld>
</p:sld>"#;
        let pptx = crate::extraction::pptx::tests::build_single_slide_pptx(slide_xml, None, &[]);

        let extractor = PptxExtractor::new();
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let config = ExtractionConfig::default();

        let result = extractor.extract_content(&pptx, mime, &config).await;
        assert!(
            result.is_ok(),
            "a normal presentation must extract under the default archive entry limit: {:?}",
            result.err()
        );
    }

    /// Gap: PPTX had `check_entry_count` (file count only) but no `ZipBombValidator`
    /// at all, so nothing ever checked aggregate declared uncompressed size or
    /// compression ratio -- unlike ODT/ODP (see `extractors::odt`/`extractors::odp`).
    /// Against unfixed code this test fails: `PptxContainer::open`/`from_bytes` never
    /// call `ZipBombValidator::validate`, so a highly compressible member sails
    /// through regardless of `max_compression_ratio`, and `extract_content` returns
    /// `Ok`.
    #[tokio::test]
    async fn test_pptx_extract_content_rejects_high_compression_ratio_archive() {
        use crate::core::config::ExtractionConfig;

        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>Hello</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld>
</p:sld>"#;
        // 64 KiB of a single repeated byte, deflated: compresses far past any sane
        // ratio in a few hundred bytes, so the archive stays tiny while the ratio
        // comparison still fires.
        let bomb_payload = vec![0u8; 64 * 1024];
        let pptx = crate::extraction::pptx::tests::build_single_slide_pptx(
            slide_xml,
            None,
            &[("ppt/media/bomb.bin", &bomb_payload)],
        );

        let extractor = PptxExtractor::new();
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let config = ExtractionConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_compression_ratio: 5,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extractor.extract_content(&pptx, mime, &config).await;
        let err = result.expect_err("a highly compressible member must be rejected under a low ratio limit");
        assert!(
            matches!(err, crate::error::XbergError::Security { .. }),
            "expected XbergError::Security, got: {err:?}"
        );
        assert!(
            err.to_string().to_lowercase().contains("ratio") || err.to_string().contains("ZIP bomb"),
            "error should name the ratio violation, got: {err}"
        );
    }

    /// Sibling of the ratio test above, bounding aggregate declared uncompressed
    /// size instead. Against unfixed code this also fails (no `max_archive_size`
    /// check existed for PPTX at all).
    #[tokio::test]
    async fn test_pptx_extract_content_rejects_archive_exceeding_max_size() {
        use crate::core::config::ExtractionConfig;

        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>Hello</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld>
</p:sld>"#;
        // Incompressible-ish payload (pseudo-random via a simple LCG) so the entry's
        // declared uncompressed size is what trips the limit, not the ratio check.
        let mut state: u32 = 0x1234_5678;
        let payload: Vec<u8> = (0..8192)
            .map(|_| {
                state = state.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                (state >> 16) as u8
            })
            .collect();
        let extra_parts: [(&str, &[u8]); 1] = [("ppt/media/big.bin", &payload)];
        let pptx = crate::extraction::pptx::tests::build_single_slide_pptx(slide_xml, None, &extra_parts);

        let extractor = PptxExtractor::new();
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let config = ExtractionConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_archive_size: 1024,
                max_compression_ratio: usize::MAX,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extractor.extract_content(&pptx, mime, &config).await;
        let err = result.expect_err("an archive declaring more bytes than max_archive_size must be rejected");
        assert!(
            matches!(err, crate::error::XbergError::Security { .. }),
            "expected XbergError::Security, got: {err:?}"
        );
    }

    /// Positive control for both tests above: a validator that rejects everything
    /// would pass the negative tests too, so this proves an ordinary presentation's
    /// exact text still extracts under the default `SecurityLimits`.
    #[tokio::test]
    async fn test_pptx_extract_content_positive_control_exact_text_under_default_limits() {
        use crate::core::config::ExtractionConfig;
        use crate::extraction::derive::derive_extraction_result;

        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>The quick brown fox jumps over the lazy dog.</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld>
</p:sld>"#;
        let pptx = crate::extraction::pptx::tests::build_single_slide_pptx(slide_xml, None, &[]);

        let extractor = PptxExtractor::new();
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let config = ExtractionConfig::default();

        let internal_doc = extractor
            .extract_content(&pptx, mime, &config)
            .await
            .expect("an ordinary presentation must extract under the default security limits");
        let result = derive_extraction_result(internal_doc, false, config.output_format);
        assert!(
            result.content.contains("The quick brown fox jumps over the lazy dog."),
            "extracted text must contain the source run's exact text, got: {:?}",
            result.content
        );
    }

    /// #1451: `max_pages` must reject a presentation once its slide count is known,
    /// before any per-slide work (text rendering, chart/diagram resolution) begins.
    /// Against unfixed code `PptxExtractionOptions` has no `max_pages` field, so this
    /// fails to compile; once the field exists but nothing reads it,
    /// `extract_content` would return `Ok` with 3 slides instead of the expected
    /// `SecurityError::TooManyPages`.
    #[tokio::test]
    async fn test_pptx_extract_content_rejects_presentation_exceeding_max_pages() {
        use crate::core::config::ExtractionConfig;

        let pptx = crate::extraction::pptx::tests::create_test_pptx_bytes(vec!["Slide 1", "Slide 2", "Slide 3"]);
        let extractor = PptxExtractor::new();
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let config = ExtractionConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_pages: Some(2),
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extractor.extract_content(&pptx, mime, &config).await;
        let error = result.expect_err("a presentation with more slides than max_pages must be rejected");
        let message = error.to_string();
        assert!(
            message.contains("too many pages") || message.contains("max_pages"),
            "error must name the limit that was hit: {message}"
        );
    }

    /// A presentation exactly at the configured `max_pages` ceiling must extract in
    /// full -- a limit that rejects the boundary case too is not the fix #1451 asked
    /// for.
    #[tokio::test]
    async fn test_pptx_extract_content_succeeds_when_slide_count_is_at_max_pages() {
        use crate::core::config::ExtractionConfig;

        let pptx = crate::extraction::pptx::tests::create_test_pptx_bytes(vec!["Slide 1", "Slide 2", "Slide 3"]);
        let extractor = PptxExtractor::new();
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let config = ExtractionConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_pages: Some(3),
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extractor.extract_content(&pptx, mime, &config).await;
        let doc = result.expect("a presentation exactly at max_pages must extract fully, not be rejected");
        assert!(
            !doc.elements.is_empty(),
            "extraction at the boundary must still produce content, not an empty truncated result"
        );
    }

    /// The default `SecurityLimits` (no override) must extract a multi-slide
    /// presentation exactly as before #1451: `max_pages` defaulting to anything
    /// other than `None` (unlimited) would silently start rejecting existing
    /// callers' presentations.
    #[tokio::test]
    async fn test_pptx_extract_content_succeeds_with_default_max_pages() {
        use crate::core::config::ExtractionConfig;

        let pptx = crate::extraction::pptx::tests::create_test_pptx_bytes(vec!["Slide 1", "Slide 2", "Slide 3"]);
        let extractor = PptxExtractor::new();
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let config = ExtractionConfig::default();

        let result = extractor.extract_content(&pptx, mime, &config).await;
        assert!(
            result.is_ok(),
            "default security limits must not reject a normal multi-slide presentation: {:?}",
            result.err()
        );
    }

    /// With no `security_limits` override the container must still enforce the default
    /// `SecurityLimits::max_files_in_archive`: "unset" means the default ceiling, not "no
    /// ceiling". One entry past that default must be rejected.
    #[tokio::test]
    async fn test_pptx_extract_content_rejects_archive_over_default_entry_limit() {
        use crate::core::config::ExtractionConfig;

        let default_limit = crate::extractors::security::SecurityLimits::default().max_files_in_archive;
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
    <p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>Hello</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld>
</p:sld>"#;
        // The builder adds its own fixed parts, so this alone already exceeds the ceiling.
        let extra_parts: Vec<(String, Vec<u8>)> = (0..=default_limit)
            .map(|i| (format!("ppt/extra_{}.xml", i), b"<x/>".to_vec()))
            .collect();
        let extra_refs: Vec<(&str, &[u8])> = extra_parts.iter().map(|(p, d)| (p.as_str(), d.as_slice())).collect();
        let pptx = crate::extraction::pptx::tests::build_single_slide_pptx(slide_xml, None, &extra_refs);

        let extractor = PptxExtractor::new();
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let config = ExtractionConfig::default();
        assert!(
            config.security_limits.is_none(),
            "this test must exercise the unset fallback, not an explicit limit"
        );

        let result = extractor.extract_content(&pptx, mime, &config).await;
        assert!(
            result.is_err(),
            "an archive over the default max_files_in_archive must be rejected when no limit is configured"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains(&default_limit.to_string()),
            "error should mention the default limit ({default_limit}), got: {err_msg}"
        );
    }

    /// A picture with no placeholder keeps an anchor for its OCR text, and only when the
    /// embedded pictures go to OCR.
    #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
    #[tokio::test]
    async fn test_plain_picture_gets_an_ocr_text_anchor_only_when_embedded_image_ocr_runs() {
        use crate::core::config::{ExtractionConfig, ImageExtractionConfig, OcrConfig, OutputFormat};
        use crate::plugins::InternalDocumentExtractor;

        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
    <p:cSld><p:spTree>
        <p:sp><p:txBody><a:p><a:r><a:t>Stock list</a:t></a:r></a:p></p:txBody></p:sp>
        <p:pic>
            <p:nvPicPr><p:cNvPr id="2" name="Picture 1"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
            <p:blipFill><a:blip r:embed="rId2"/></p:blipFill>
            <p:spPr><a:xfrm><a:off x="0" y="2000000"/><a:ext cx="1000000" cy="1000000"/></a:xfrm></p:spPr>
        </p:pic>
    </p:spTree></p:cSld>
</p:sld>"#;
        let slide_rels_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
</Relationships>"#;
        let pptx = crate::extraction::pptx::tests::build_single_slide_pptx(
            slide_xml,
            Some(slide_rels_xml),
            &[("ppt/media/image1.png", b"picture bytes")],
        );
        let mime = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let anchors = |document: &InternalDocument| -> Vec<(Option<u32>, Option<u32>)> {
            document
                .elements
                .iter()
                .filter_map(|element| {
                    element
                        .image_ocr_text_anchor_index()
                        .map(|image_index| (Some(image_index), element.page))
                })
                .collect()
        };

        let with_ocr = ExtractionConfig {
            ocr: Some(OcrConfig::default()),
            output_format: OutputFormat::Plain,
            ..Default::default()
        };
        let document = PptxExtractor::new()
            .extract_content(&pptx, mime, &with_ocr)
            .await
            .expect("the deck extracts");
        assert_eq!(document.images.len(), 1);
        assert_eq!(anchors(&document), vec![(Some(0), Some(1))]);

        let without_ocr = ExtractionConfig {
            images: Some(ImageExtractionConfig::default()),
            output_format: OutputFormat::Plain,
            ..Default::default()
        };
        let reference = PptxExtractor::new()
            .extract_content(&pptx, mime, &without_ocr)
            .await
            .expect("the deck extracts");
        assert_eq!(reference.images.len(), 1, "the picture is read without OCR too");
        assert_eq!(anchors(&reference), vec![]);
        assert_eq!(reference.elements.len() + 1, document.elements.len());
    }

    /// A picture with no placeholder is one step of the iteration limit, the same as a
    /// picture with a placeholder, also when it gets no anchor.
    #[test]
    fn test_a_picture_with_no_placeholder_counts_against_the_iteration_limit() {
        let slide_contents = vec![crate::extraction::pptx::PptxInternalSlide {
            slide_number: 1,
            elements: vec![
                crate::extraction::pptx::PptxInternalSlideElement::ImageOcrTextAnchor { image_index: 0 },
                crate::extraction::pptx::PptxInternalSlideElement::ImageOcrTextAnchor { image_index: 1 },
            ],
        }];
        let budget_of = |max_iterations: usize| {
            SecurityBudget::from_limits(&crate::extractors::security::SecurityLimits {
                max_iterations,
                ..Default::default()
            })
        };

        let mut budget = budget_of(2);
        let document = PptxExtractor::build_internal_document(&slide_contents, 1, &[], true, false, &mut budget)
            .expect("two pictures fit a limit of two steps");
        assert_eq!(document.images.len(), 0);

        let mut budget = budget_of(1);
        let error = PptxExtractor::build_internal_document(&slide_contents, 1, &[], true, false, &mut budget)
            .expect_err("the second picture is over a limit of one step")
            .to_string();
        assert!(error.contains("Too many iterations"), "{error}");
    }
}
