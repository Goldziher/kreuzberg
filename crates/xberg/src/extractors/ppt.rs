//! Native PPT extractor for PowerPoint 97-2003 binary format.
//!
//! Extracts text directly from OLE/CFB compound documents without LibreOffice.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::core::mime::LEGACY_POWERPOINT_MIME_TYPE;
use crate::extraction::ppt::PptSlideText;
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::types::ExtractedImage;
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::{Metadata, PageInfo, PageStructure, PageUnitType};
use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;
#[cfg_attr(alef, alef(skip))]
/// Native PPT extractor using OLE/CFB parsing.
///
/// This extractor handles PowerPoint 97-2003 binary (.ppt) files without
/// requiring LibreOffice, providing ~50x faster extraction.
pub struct PptExtractor;

impl PptExtractor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for PptExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Join a title's outline paragraphs -- kept as `\n` inside `PptSlideText::title`, one
/// `\r` paragraph mark per break -- into the single line a `Slide` node's title is
/// displayed as (xberg-io/xberg#1635): `"Special Databases:\nREACTIONS"` becomes
/// `"Special Databases: REACTIONS"`, not two lines that read as two different slides.
fn join_title_paragraphs(title: &str) -> String {
    title
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// How many of `body`'s leading lines, joined by a single space and trimmed, equal
/// `title`. The outline merge always prepends a recovered title at the very front of the
/// slide's text, and a title the drawing already carried is normally its first shape too --
/// so this looks from the front, and `0` means the title's lines were not found there.
fn leading_lines_matching(body: &str, title: &str) -> usize {
    if title.is_empty() {
        return 0;
    }
    let mut joined = String::new();
    for (index, line) in body.lines().enumerate() {
        if !joined.is_empty() {
            joined.push(' ');
        }
        joined.push_str(line.trim());
        if joined == title {
            return index + 1;
        }
    }
    0
}

/// Run the blocking PPT/OLE extraction, offloading to `spawn_blocking` under batch mode
/// (tokio-runtime feature) so it does not block the async runtime; otherwise runs inline.
async fn run_ppt_extraction(
    content: &[u8],
    config: &ExtractionConfig,
    include_master_slides: bool,
    extract_images: bool,
) -> Result<crate::extraction::ppt::PptExtractionResult> {
    #[cfg(feature = "tokio-runtime")]
    if crate::core::batch_mode::is_batch_mode() {
        if config.cancel_token.as_ref().map(|t| t.is_cancelled()).unwrap_or(false) {
            return Err(crate::error::XbergError::Cancelled);
        }
        let content_owned = content.to_vec();
        let span = tracing::Span::current();
        tokio::task::spawn_blocking(move || -> crate::error::Result<_> {
            let _guard = span.entered();
            crate::extraction::ppt::extract_ppt_text_with_options(&content_owned, include_master_slides, extract_images)
        })
        .await
        .map_err(|e| crate::error::XbergError::parsing(format!("PPT extraction task failed: {e}")))?
    } else {
        crate::extraction::ppt::extract_ppt_text_with_options(content, include_master_slides, extract_images)
    }

    #[cfg(not(feature = "tokio-runtime"))]
    {
        if config.cancel_token.as_ref().map(|t| t.is_cancelled()).unwrap_or(false) {
            return Err(crate::error::XbergError::Cancelled);
        }
        crate::extraction::ppt::extract_ppt_text_with_options(content, include_master_slides, extract_images)
    }
}

/// Build the `metadata.additional` map for `slide_count`, `extraction_method`, and
/// `speaker_notes` (when present).
fn build_ppt_metadata_map(
    slide_count: usize,
    speaker_notes: &[String],
) -> AHashMap<Cow<'static, str>, serde_json::Value> {
    let mut metadata_map = AHashMap::new();

    metadata_map.insert(
        Cow::Borrowed("slide_count"),
        serde_json::Value::Number(slide_count.into()),
    );
    metadata_map.insert(
        Cow::Borrowed("extraction_method"),
        serde_json::Value::String("native_ole".to_string()),
    );

    if !speaker_notes.is_empty() {
        metadata_map.insert(
            Cow::Borrowed("speaker_notes"),
            serde_json::Value::Array(
                speaker_notes
                    .iter()
                    .map(|n| serde_json::Value::String(n.clone()))
                    .collect(),
            ),
        );
    }

    metadata_map
}

/// Build the deck's `PageStructure` (one `PageInfo` per slide), or `None` when there are no
/// slides.
fn build_ppt_page_structure(slide_count: usize) -> Option<PageStructure> {
    if slide_count == 0 {
        return None;
    }
    Some(PageStructure {
        total_count: slide_count as u32,
        unit_type: PageUnitType::Slide,
        boundaries: None,
        pages: Some(
            (1..=slide_count)
                .map(|num| PageInfo {
                    number: num as u32,
                    title: None,
                    dimensions: None,
                    image_count: None,
                    table_count: None,
                    hidden: None,
                    is_blank: None,
                    has_vector_graphics: false,
                })
                .collect(),
        ),
    })
}

impl PptExtractor {
    /// Recursively extract the deck's embedded OLE objects into `children` (GH#1660),
    /// mirroring `extraction::ooxml_embedded::extract_ooxml_embedded_objects`: one
    /// `ArchiveEntry` per object the matching legacy extractor can read, a warning per
    /// object it cannot. The entry path carries the displaying slide when a shape
    /// references the object (`slide2/oleObject1.bin`).
    async fn extract_embedded_objects(
        content: &[u8],
        config: &ExtractionConfig,
    ) -> (Vec<crate::types::ArchiveEntry>, Vec<crate::types::ProcessingWarning>) {
        const SOURCE: &str = "ppt_embedded_objects";
        let security_limits = config.security_limits.clone().unwrap_or_default();
        let max_object_bytes = config
            .max_embedded_file_bytes
            .unwrap_or(security_limits.max_archive_size as u64) as usize;

        let (mut objects, mut warnings) =
            match crate::extraction::ppt::extract_ppt_embedded_objects(content, max_object_bytes) {
                Ok(parsed) => parsed,
                Err(e) => {
                    return (
                        Vec::new(),
                        vec![crate::types::ProcessingWarning {
                            source: Cow::Borrowed(SOURCE),
                            message: Cow::Owned(format!("Failed to read embedded objects: {e}")),
                        }],
                    );
                }
            };
        if objects.len() > security_limits.max_files_in_archive {
            let skipped = objects.len() - security_limits.max_files_in_archive;
            warnings.push(crate::types::ProcessingWarning {
                source: Cow::Borrowed(SOURCE),
                message: Cow::Owned(format!(
                    "Skipped {skipped} embedded object(s): max_files_in_archive ({}) reached",
                    security_limits.max_files_in_archive
                )),
            });
            objects.truncate(security_limits.max_files_in_archive);
        }

        let mut child_config = config.clone();
        child_config.max_archive_depth = config.max_archive_depth.saturating_sub(1);

        let mut children = Vec::new();
        for object in objects {
            let path = match object.slide_number {
                Some(slide) => format!("slide{slide}/oleObject{}.bin", object.ex_obj_id),
                None => format!("oleObject{}.bin", object.ex_obj_id),
            };
            let Some((inner_bytes, inner_mime)) =
                crate::extraction::ooxml_embedded::extract_ole_embedded_object(&object.data)
            else {
                warnings.push(crate::types::ProcessingWarning {
                    source: Cow::Borrowed(SOURCE),
                    message: Cow::Owned(format!(
                        "Skipped embedded object '{path}': format identification not supported"
                    )),
                });
                continue;
            };
            match crate::core::extractor::extract_bytes(&inner_bytes, &inner_mime, &child_config).await {
                Ok(result) => children.push(crate::types::ArchiveEntry {
                    path,
                    mime_type: inner_mime,
                    result: Box::new(result),
                }),
                Err(e) => warnings.push(crate::types::ProcessingWarning {
                    source: Cow::Borrowed(SOURCE),
                    message: Cow::Owned(format!("Failed to extract embedded object '{path}': {e}")),
                }),
            }
        }
        (children, warnings)
    }

    /// Build an `InternalDocument` from PPT extracted slides and embedded images.
    ///
    /// `slides` carries the deck's real per-slide structure (persist order and numbering,
    /// from `extraction::ppt::extract_texts_from_records`) -- slide numbers are read from
    /// that structure, never re-derived by splitting rendered text (#1418), and each
    /// slide's speaker notes come from `PptSlideText::notes`, resolved by `slideIdRef`
    /// rather than by position among the deck's non-empty notes pages (#1640).
    fn build_internal_document(slides: &[PptSlideText], images: &[ExtractedImage]) -> InternalDocument {
        let mut builder = InternalDocumentBuilder::new("ppt");

        for slide in slides.iter() {
            push_deck_slide(&mut builder, slide, images);
        }

        // A blip no live shape referenced resolves to no slide (#1620); it is still
        // extracted, appended here as it always was, rather than dropped. ~keep
        for image in images.iter().filter(|image| image.page_number.is_none()) {
            builder.push_image(None, image.clone(), None, None);
        }

        builder.build()
    }
}

/// Resolve a slide's title and how many of its (already-trimmed) body's leading lines that
/// title accounts for, given the file's own outline title, if any, and whether the slide has
/// a table.
fn resolve_slide_title(file_title: Option<&str>, trimmed: &str, has_table: bool) -> (Option<String>, usize) {
    match file_title {
        Some(file_title) => {
            let joined = join_title_paragraphs(file_title);
            let matched = leading_lines_matching(trimmed, &joined);
            (Some(joined), matched)
        }
        None => {
            let mut lines = trimmed.lines();
            let first_line = lines.next().unwrap_or("");
            // A table's cells are not in `trimmed`, so a drawn title over a table is the only
            // line there; it is still the slide's title, as it was when the cells were lines. ~keep
            if !first_line.is_empty() && first_line.len() <= 80 && (has_table || lines.clone().next().is_some()) {
                (Some(first_line.to_string()), 1)
            } else {
                (None, 0)
            }
        }
    }
}

/// Push a slide's body as one or more paragraphs, skipping the leading lines the title
/// already accounts for.
fn push_slide_body(builder: &mut InternalDocumentBuilder, trimmed: &str, title: &Option<String>, header_lines: usize) {
    if trimmed.is_empty() {
        return;
    }

    if title.is_some() && header_lines > 0 {
        // Skip the lines the title already accounts for -- whether the outline
        // merge prepended them or the drawing already carried them -- so the
        // title text is never also emitted as a body paragraph.
        for line in trimmed.lines().skip(header_lines) {
            let lt = line.trim();
            if !lt.is_empty() {
                builder.push_paragraph(lt, vec![], None, None);
            }
        }
        return;
    }

    builder.push_paragraph(trimmed, vec![], None, None);
}

/// Push one slide's title, body, images, and speaker notes onto `builder`.
fn push_deck_slide(builder: &mut InternalDocumentBuilder, slide: &PptSlideText, images: &[ExtractedImage]) {
    let trimmed = slide.text.trim();
    // The file's own outline title (#1635) wins when it states one; the first-line
    // guess below is the fallback for a deck whose title is drawn on the canvas and
    // never entered in the outline view, which has no outline title to read at all.
    let (title, header_lines) = resolve_slide_title(slide.title.as_deref(), trimmed, !slide.tables.is_empty());
    builder.push_slide(slide.number, title.as_deref(), None);

    push_slide_body(builder, trimmed, &title, header_lines);

    // Each table follows the slide's other text and carries the slide's page, as a PPTX
    // table does. ~keep
    for table in &slide.tables {
        builder.push_table_from_cells(table, Some(slide.number), None);
    }

    // Inside the slide loop, so an image node sits on the slide that displays it
    // rather than behind the last one -- which is what made a picture-only slide
    // read as an empty slide (#1620). ~keep
    for image in images.iter().filter(|image| image.page_number == Some(slide.number)) {
        builder.push_image(None, image.clone(), image.page_number, None);
    }

    if let Some(notes) = slide.notes.as_deref()
        && !notes.is_empty()
    {
        let key = format!("slide-{}-notes", slide.number);
        builder.push_footnote_definition(notes, &key, None);
    }
}

impl Plugin for PptExtractor {
    fn name(&self) -> &str {
        "ppt-extractor"
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
        "Native PPT text extraction via OLE/CFB parsing"
    }

    fn author(&self) -> &str {
        "Xberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for PptExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let include_master_slides = config.content_filter.as_ref().is_some_and(|f| f.include_headers);
        let extract_images = config.needs_image_data();

        let result = run_ppt_extraction(content, config, include_master_slides, extract_images).await?;

        let meta_title = result.metadata.title;
        let meta_subject = result.metadata.subject;

        let (meta_authors, meta_created_by) = if let Some(author) = result.metadata.author {
            (Some(vec![author.clone()]), Some(author))
        } else {
            (None, None)
        };

        let meta_modified_by = result.metadata.last_author;

        let metadata_map = build_ppt_metadata_map(result.slide_count, &result.speaker_notes);
        let page_structure = build_ppt_page_structure(result.slide_count);

        let mut doc = Self::build_internal_document(&result.slides, &result.images);
        doc.mime_type = mime_type.to_string();
        doc.processing_warnings.extend(result.processing_warnings);

        // GH#1660: the legacy counterpart of the `ppt/embeddings/` recursion the `.pptx`
        // extractor does -- a Word or Excel table inserted as an object lives in an
        // `ExOleObjStg` and reached the output in no form. ~keep
        if config.max_archive_depth > 0 {
            let (children, embed_warnings) = Self::extract_embedded_objects(content, config).await;
            if !children.is_empty() {
                doc.children = Some(children);
            }
            doc.processing_warnings.extend(embed_warnings);
        }
        doc.metadata = Metadata {
            title: meta_title,
            subject: meta_subject,
            authors: meta_authors,
            created_by: meta_created_by,
            modified_by: meta_modified_by,
            pages: page_structure,
            additional: metadata_map,
            ..Default::default()
        };

        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[LEGACY_POWERPOINT_MIME_TYPE]
    }

    fn priority(&self) -> i32 {
        60
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::internal::ElementKind;

    fn image_on(page: Option<u32>, index: u32) -> ExtractedImage {
        ExtractedImage {
            data: bytes::Bytes::from_static(b"png"),
            format: std::borrow::Cow::Borrowed("png"),
            image_index: index,
            page_number: page,
            ..Default::default()
        }
    }

    /// #1620: an image node must sit on the slide that displays it. Emitting every image
    /// after the slide loop put each one behind the last slide, which made a slide whose
    /// only content is a picture read as an empty slide.
    #[test]
    fn should_emit_each_image_node_on_its_own_slide() {
        let slides = vec![
            PptSlideText {
                number: 1,
                text: "First".to_string(),
                title: None,
                notes: None,
                tables: Vec::new(),
            },
            // Picture-only: no text at all, so before the fix this slide had no content.
            PptSlideText {
                number: 2,
                text: String::new(),
                title: None,
                notes: None,
                tables: Vec::new(),
            },
            PptSlideText {
                number: 3,
                text: "Third".to_string(),
                title: None,
                notes: None,
                tables: Vec::new(),
            },
        ];
        let images = vec![image_on(Some(2), 0), image_on(None, 1)];

        let document = PptExtractor::build_internal_document(&slides, &images);

        let order: Vec<String> = document
            .elements
            .iter()
            .filter_map(|element| match &element.kind {
                ElementKind::Slide { number } => Some(format!("slide{number}")),
                ElementKind::Image { .. } => Some("image".to_string()),
                _ => None,
            })
            .collect();

        assert_eq!(
            order,
            vec!["slide1", "slide2", "image", "slide3", "image"],
            "the referenced image belongs to slide 2; the unreferenced one still trails the deck"
        );
    }

    #[tokio::test]
    async fn test_ppt_extractor_plugin_interface() {
        let extractor = PptExtractor::new();
        assert_eq!(extractor.name(), "ppt-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 60);
        assert_eq!(extractor.supported_mime_types(), &["application/vnd.ms-powerpoint"]);
    }

    #[tokio::test]
    async fn test_ppt_extractor_initialize_shutdown() {
        let extractor = PptExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[tokio::test]
    async fn test_ppt_extractor_real_file() {
        let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/ppt/simple.ppt");
        if !test_file.exists() {
            return;
        }
        let content = std::fs::read(&test_file).expect("Failed to read test PPT");
        let extractor = PptExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_content(&content, "application/vnd.ms-powerpoint", &config)
            .await
            .expect("PPT extraction failed");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);
        assert!(!result.content.is_empty(), "Should extract text from PPT");
        assert_eq!(&*result.mime_type, "application/vnd.ms-powerpoint");
    }

    #[tokio::test]
    async fn test_ppt_document_structure_slides() {
        let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/ppt/simple.ppt");
        if !test_file.exists() {
            return;
        }
        let content = std::fs::read(&test_file).expect("Failed to read test PPT");
        let extractor = PptExtractor::new();
        let config = ExtractionConfig {
            include_document_structure: true,
            ..Default::default()
        };
        let result = extractor
            .extract_content(&content, "application/vnd.ms-powerpoint", &config)
            .await
            .expect("PPT extraction failed");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);
        assert!(result.document.is_some(), "Should produce document structure for PPT");
        let doc = result.document.unwrap();
        let has_slide = doc
            .nodes
            .iter()
            .any(|n| matches!(n.content, crate::types::document_structure::NodeContent::Slide { .. }));
        assert!(has_slide, "PPT should produce Slide nodes in document structure");
    }

    #[tokio::test]
    async fn test_ppt_slide_count_metadata() {
        let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/ppt/simple.ppt");
        if !test_file.exists() {
            return;
        }
        let content = std::fs::read(&test_file).expect("Failed to read test PPT");
        let extractor = PptExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_content(&content, "application/vnd.ms-powerpoint", &config)
            .await
            .expect("PPT extraction failed");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);
        assert!(
            result.metadata.additional.contains_key("slide_count"),
            "Should have slide_count metadata"
        );
        let slide_count = result.metadata.additional.get("slide_count").unwrap();
        assert!(slide_count.as_u64().unwrap_or(0) > 0, "Slide count should be > 0");
    }

    /// PPT speaker notes go to `metadata.additional["speaker_notes"]` as a JSON array,
    /// NOT to `PageContent.speaker_notes`.  The legacy binary format does not support
    /// per-slide `PageContent` objects, so `page_contents` is always `None` for PPT
    /// regardless of whether `page_config` is set.
    #[tokio::test]
    async fn test_ppt_speaker_notes_in_metadata_not_page_contents() {
        let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/ppt/simple.ppt");
        if !test_file.exists() {
            return;
        }
        let content = std::fs::read(&test_file).expect("Failed to read test PPT");
        let extractor = PptExtractor::new();
        let config = ExtractionConfig {
            pages: Some(crate::core::config::PageConfig::default()),
            ..Default::default()
        };
        let result = extractor
            .extract_content(&content, "application/vnd.ms-powerpoint", &config)
            .await
            .expect("PPT extraction failed");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);
        assert!(
            result.pages.is_none(),
            "PPT should not produce pages; speaker notes are in metadata.additional"
        );
        if let Some(notes) = result.metadata.additional.get("speaker_notes") {
            assert!(notes.is_array(), "PPT speaker_notes in metadata should be a JSON array");
        }
    }

    /// Slide-element `text` (the title, for `ElementKind::Slide`) for slide `number`, or
    /// `None` if no such slide element exists.
    fn slide_element_title(doc: &crate::types::internal::InternalDocument, number: u32) -> Option<String> {
        doc.elements.iter().find_map(|e| match &e.kind {
            ElementKind::Slide { number: n } if *n == number => Some(e.text.clone()),
            _ => None,
        })
    }

    /// Paragraph texts, in document order.
    fn paragraph_texts(doc: &crate::types::internal::InternalDocument) -> Vec<String> {
        doc.elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Paragraph))
            .map(|e| e.text.clone())
            .collect()
    }

    /// xberg-io/xberg#1635: a slide whose only text is its title -- a picture, a diagram, a
    /// section divider -- previously guessed `title: None` because the first-line heuristic
    /// required a *second* line to trust the first as a title. The file's own outline title
    /// carries no such requirement.
    #[test]
    fn should_use_the_outline_title_when_the_slide_has_no_other_text() {
        let slides = vec![PptSlideText {
            number: 1,
            text: "Section Divider".to_string(),
            title: Some("Section Divider".to_string()),
            notes: None,
            tables: Vec::new(),
        }];
        let doc = PptExtractor::build_internal_document(&slides, &[]);

        assert_eq!(slide_element_title(&doc, 1), Some("Section Divider".to_string()));
        assert!(
            paragraph_texts(&doc).is_empty(),
            "a title-only slide must not also get a body paragraph repeating the title"
        );
    }

    /// xberg-io/xberg#1635: a title stored as two outline paragraphs (one PowerPoint `\r`
    /// break) must reach the node whole, space-joined -- not truncated to its first line
    /// the way the old first-line heuristic cut `"Special Databases:\nREACTIONS"` down to
    /// `"Special Databases:"`.
    #[test]
    fn should_join_a_two_paragraph_outline_title_instead_of_truncating_it() {
        let slides = vec![PptSlideText {
            number: 1,
            text: "Special Databases:\nREACTIONS\nBody bullet one".to_string(),
            title: Some("Special Databases:\nREACTIONS".to_string()),
            notes: None,
            tables: Vec::new(),
        }];
        let doc = PptExtractor::build_internal_document(&slides, &[]);

        assert_eq!(
            slide_element_title(&doc, 1),
            Some("Special Databases: REACTIONS".to_string())
        );
        assert_eq!(
            paragraph_texts(&doc),
            vec!["Body bullet one".to_string()],
            "only the body bullet remains a paragraph; the title's two lines must not appear there too"
        );
    }

    /// xberg-io/xberg#1635: the old heuristic dropped a title longer than 80 characters
    /// outright (`title: None`). An outline title has no such length cap.
    #[test]
    fn should_use_an_outline_title_longer_than_eighty_characters() {
        let long_title = "A section title that runs well past the eighty character heuristic cutoff used before";
        assert!(long_title.len() > 80, "fixture must exercise the old length cutoff");
        let slides = vec![PptSlideText {
            number: 1,
            text: format!("{long_title}\nBody text"),
            title: Some(long_title.to_string()),
            notes: None,
            tables: Vec::new(),
        }];
        let doc = PptExtractor::build_internal_document(&slides, &[]);

        assert_eq!(slide_element_title(&doc, 1), Some(long_title.to_string()));
        assert_eq!(paragraph_texts(&doc), vec!["Body text".to_string()]);
    }

    /// A deck whose title is drawn on the canvas and never entered in the outline view has
    /// no outline title to read (`PptSlideText::title` is `None`); the first-line heuristic
    /// must still apply exactly as before (xberg-io/xberg#1635 fallback contract).
    #[test]
    fn should_fall_back_to_the_first_line_heuristic_without_an_outline_title() {
        let slides = vec![PptSlideText {
            number: 1,
            text: "Drawn Title\nDrawn body".to_string(),
            title: None,
            notes: None,
            tables: Vec::new(),
        }];
        let doc = PptExtractor::build_internal_document(&slides, &[]);

        assert_eq!(slide_element_title(&doc, 1), Some("Drawn Title".to_string()));
        assert_eq!(paragraph_texts(&doc), vec!["Drawn body".to_string()]);
    }

    /// A table's cells are not in the slide text, so a drawn title over a table is the
    /// slide's only line; it must still be taken as the title, as it was when the cells
    /// were read as lines after it.
    #[test]
    fn should_take_the_only_line_over_a_table_as_the_title() {
        let slides = vec![PptSlideText {
            number: 1,
            text: "Cases by region".to_string(),
            title: None,
            notes: None,
            tables: vec![vec![
                vec!["Region".to_string(), "Q1".to_string()],
                vec!["North".to_string(), "27,039".to_string()],
            ]],
        }];
        let doc = PptExtractor::build_internal_document(&slides, &[]);

        assert_eq!(slide_element_title(&doc, 1), Some("Cases by region".to_string()));
        assert!(paragraph_texts(&doc).is_empty(), "{:?}", paragraph_texts(&doc));
        assert_eq!(doc.tables.len(), 1);
        assert_eq!(doc.tables[0].cells[1], vec!["North", "27,039"]);
    }

    /// #1418 root-cause regression at the consumer side: `build_internal_document`
    /// must trust the structured `slides` list, never re-split a slide's own
    /// text on `"\n\n"`. A single slide whose text happens to contain an
    /// internal blank line must still produce exactly one `Slide` element.
    #[test]
    fn should_produce_one_slide_element_when_slide_text_contains_internal_blank_line() {
        let slides = vec![PptSlideText {
            number: 1,
            text: "Title\n\nBody".to_string(),
            title: None,
            notes: None,
            tables: Vec::new(),
        }];
        let doc = PptExtractor::build_internal_document(&slides, &[]);

        let slide_numbers: Vec<u32> = doc
            .elements
            .iter()
            .filter_map(|e| match &e.kind {
                crate::types::internal::ElementKind::Slide { number } => Some(*number),
                _ => None,
            })
            .collect();

        assert_eq!(
            slide_numbers,
            vec![1],
            "one Slide entry must produce exactly one Slide element, however its text is shaped"
        );
    }

    /// #1418: a slide with no text atoms must still get a `Slide` element
    /// carrying its real persist-order number, not be dropped (which would
    /// shift every later slide's number down).
    #[test]
    fn should_number_slide_elements_by_persist_order_including_an_empty_middle_slide() {
        let slides = vec![
            PptSlideText {
                number: 1,
                text: "Slide One".to_string(),
                title: None,
                notes: None,
                tables: Vec::new(),
            },
            PptSlideText {
                number: 2,
                text: String::new(),
                title: None,
                notes: None,
                tables: Vec::new(),
            },
            PptSlideText {
                number: 3,
                text: "Slide Three".to_string(),
                title: None,
                notes: None,
                tables: Vec::new(),
            },
        ];
        let doc = PptExtractor::build_internal_document(&slides, &[]);

        let slide_numbers: Vec<u32> = doc
            .elements
            .iter()
            .filter_map(|e| match &e.kind {
                crate::types::internal::ElementKind::Slide { number } => Some(*number),
                _ => None,
            })
            .collect();

        assert_eq!(slide_numbers, vec![1, 2, 3]);
    }

    /// #1417: images recovered from the `Pictures` stream must be attached
    /// to the document (`InternalDocument::images`), not silently discarded.
    #[test]
    fn should_attach_images_to_internal_document_when_images_are_present() {
        let slides = vec![PptSlideText {
            number: 1,
            text: "Slide One".to_string(),
            title: None,
            notes: None,
            tables: Vec::new(),
        }];
        let image = ExtractedImage {
            data: bytes::Bytes::from_static(b"\xFF\xD8\xFFfake-jpeg"),
            format: Cow::Borrowed("jpeg"),
            image_index: 0,
            page_number: None,
            width: None,
            height: None,
            colorspace: None,
            bits_per_component: None,
            is_mask: false,
            description: None,
            ocr_result: None,
            bounding_box: None,
            source_path: None,
            image_kind: None,
            kind_confidence: None,
            cluster_id: None,
            caption: None,
            qr_codes: None,
            data_base64: None,
        };
        let doc = PptExtractor::build_internal_document(&slides, std::slice::from_ref(&image));

        assert_eq!(doc.images.len(), 1);
        assert_eq!(doc.images[0].format, "jpeg");
        assert_eq!(&doc.images[0].data[..], b"\xFF\xD8\xFFfake-jpeg");
        let has_image_element = doc
            .elements
            .iter()
            .any(|e| matches!(&e.kind, crate::types::internal::ElementKind::Image { image_index: 0 }));
        assert!(has_image_element, "an Image element must reference the pushed image");
    }

    /// #87/#1418 end-to-end: `simple.ppt` has exactly two `Slide` (0x03EE)
    /// containers (see `extraction::ppt::tests::test_extract_ppt_real_file_reports_two_slides`).
    /// The document structure produced through the real extraction pipeline
    /// must report exactly slide numbers `[1, 2]`, not ordinals derived from
    /// re-splitting joined text.
    #[tokio::test]
    async fn should_report_exact_contiguous_slide_numbers_for_real_ppt_file() {
        let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/ppt/simple.ppt");
        if !test_file.exists() {
            return;
        }
        let content = std::fs::read(&test_file).expect("Failed to read test PPT");
        let extractor = PptExtractor::new();
        let config = ExtractionConfig::default();
        let doc = extractor
            .extract_content(&content, "application/vnd.ms-powerpoint", &config)
            .await
            .expect("PPT extraction failed");

        let slide_numbers: Vec<u32> = doc
            .elements
            .iter()
            .filter_map(|e| match &e.kind {
                crate::types::internal::ElementKind::Slide { number } => Some(*number),
                _ => None,
            })
            .collect();

        assert_eq!(
            slide_numbers,
            vec![1, 2],
            "simple.ppt has exactly two Slide containers, numbered 1 and 2"
        );
    }
}
