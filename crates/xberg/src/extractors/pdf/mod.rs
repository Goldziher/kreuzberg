//! PDF document extractor.
//!
//! Provides extraction of text, metadata, tables, and images from PDF documents
//! using xberg_native_pdf (pure Rust). Supports both native text extraction and OCR fallback.

mod extraction;
#[cfg(feature = "layout-detection")]
mod layout_hints;
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
mod layout_runner;
pub(crate) mod ocr;
mod pages;
#[cfg(all(feature = "pdf", feature = "pdf-pdfium"))]
mod pdfium_engine;
#[cfg(feature = "layout-detection")]
pub(crate) mod reading_order;
#[cfg(all(feature = "liter-llm", feature = "layout-detection"))]
mod region_vlm;
#[cfg(feature = "pdf")]
pub(crate) mod rotation;

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::{ExtractionMethod, Metadata};
use async_trait::async_trait;
#[cfg(feature = "tokio-runtime")]
use std::path::Path;

use extraction::extract_all_from_native_document;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn extraction_method_after_mixed_ocr(replacements: &ahash::AHashMap<u32, String>) -> ExtractionMethod {
    if replacements.is_empty() {
        ExtractionMethod::Native
    } else {
        ExtractionMethod::Mixed
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn boundaries_for_ocr_output(
    extraction_method: ExtractionMethod,
    text: &str,
    native_boundaries: Option<&[crate::types::PageBoundary]>,
    replacements: Option<&ahash::AHashMap<u32, String>>,
    ocr_page_texts: Option<&[String]>,
) -> Option<Vec<crate::types::PageBoundary>> {
    match extraction_method {
        ExtractionMethod::Native => native_boundaries.map(<[_]>::to_vec),
        ExtractionMethod::Mixed => native_boundaries.map(|boundaries| {
            replacements.map_or_else(
                || boundaries.to_vec(),
                |accepted| ocr::boundaries_after_replacements(boundaries, accepted),
            )
        }),
        ExtractionMethod::Ocr => exact_boundaries_for_page_texts(text, ocr_page_texts?),
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn exact_boundaries_for_page_texts(text: &str, page_texts: &[String]) -> Option<Vec<crate::types::PageBoundary>> {
    let mut boundaries = Vec::with_capacity(page_texts.len());
    let mut search_offset = 0usize;

    for (index, page_text) in page_texts.iter().enumerate() {
        let byte_start = if page_text.is_empty() {
            search_offset
        } else {
            search_offset + text.get(search_offset..)?.find(page_text)?
        };
        let byte_end = byte_start + page_text.len();
        boundaries.push(crate::types::PageBoundary {
            byte_start,
            byte_end,
            page_number: (index + 1) as u32,
        });
        search_offset = byte_end;
    }

    Some(boundaries)
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn accepted_mixed_ocr_elements(
    structured_pages: &ahash::AHashMap<u32, InternalDocument>,
) -> Vec<crate::types::OcrElement> {
    let mut page_numbers = structured_pages.keys().copied().collect::<Vec<_>>();
    page_numbers.sort_unstable();
    page_numbers
        .into_iter()
        .filter_map(|page_number| structured_pages.get(&page_number))
        .filter_map(|page| page.prebuilt_ocr_elements.as_ref())
        .flatten()
        .cloned()
        .collect()
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn accepted_mixed_ocr_tables(structured_pages: &ahash::AHashMap<u32, InternalDocument>) -> Vec<crate::types::Table> {
    let mut page_numbers = structured_pages.keys().copied().collect::<Vec<_>>();
    page_numbers.sort_unstable();
    page_numbers
        .into_iter()
        .filter_map(|page_number| structured_pages.get(&page_number))
        .flat_map(|page| page.tables.iter())
        .cloned()
        .collect()
}

/// One coordinate frame per OCR page that has both a captured frame (GH#1645) and public
/// elements -- a page whose elements were all filtered out (margins, quality gate) has
/// nothing for a consumer to join the frame to, so it is omitted rather than emitted with an
/// empty element set. Sorted by page number for a deterministic, reader-facing array; the
/// backing `structured_pages` is an `AHashMap`, whose iteration order is unspecified.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn accepted_mixed_ocr_coordinate_frames(
    structured_pages: &ahash::AHashMap<u32, InternalDocument>,
) -> Vec<crate::types::internal::OcrPageCoordinateFrame> {
    let mut page_numbers = structured_pages.keys().copied().collect::<Vec<_>>();
    page_numbers.sort_unstable();
    page_numbers
        .into_iter()
        .filter_map(|page_number| structured_pages.get(&page_number))
        .filter(|page| {
            page.prebuilt_ocr_elements
                .as_ref()
                .is_some_and(|elements| !elements.is_empty())
        })
        .filter_map(|page| page.ocr_coordinate_frame)
        .collect()
}

#[cfg(feature = "pdf")]
const PDF_OUTLINES_MARKER: &[u8] = b"/Outlines";
#[cfg(feature = "pdf")]
const PDF_PREVIOUS_XREF_MARKER: &[u8] = b"/Prev";

#[cfg(feature = "pdf")]
fn contains_pdf_marker(content: &[u8], marker: &[u8]) -> bool {
    memchr::memmem::find(content, marker).is_some()
}

#[cfg(feature = "pdf")]
fn catalog_needs_lopdf_compatibility_pass(catalog: Option<&xberg_native_pdf::object::Object>) -> bool {
    let Some(catalog) = catalog else {
        return true;
    };
    let Some(dictionary) = catalog.as_dict() else {
        return true;
    };
    dictionary.contains_key("Outlines")
}

#[cfg(feature = "pdf")]
fn raw_pdf_needs_lopdf_compatibility_pass(content: &[u8]) -> bool {
    contains_pdf_marker(content, PDF_PREVIOUS_XREF_MARKER) || contains_pdf_marker(content, PDF_OUTLINES_MARKER)
}

/// Reject a document whose page count exceeds `security_limits.max_pages` before any
/// per-page work (layout detection, OCR, rendering) begins (#1451).
///
/// Counts pages with `xberg_native_pdf` first and falls back to `lopdf` when that cannot
/// open the document at all.
///
/// The fallback exists because the `xberg_native_pdf` open is the sole gate on this cap, and
/// `raw_pdf_needs_lopdf_compatibility_pass` below documents that some structures only
/// `lopdf` can walk — so without it, a document `xberg_native_pdf` rejects skips the cap
/// entirely and is then handed to an extraction path that may still recover it. That
/// is the shape a hostile input would take against a security limit.
///
/// Measured, so the comment does not outlive the fact: over the 488 PDFs in
/// `test_documents/`, exactly one defeats the `xberg_native_pdf` count — `corrupt_truncated.pdf`,
/// which `lopdf` also cannot read and which fails extraction anyway. Encrypted documents
/// are *not* among the failures: a PDF's page tree is structure, not string or stream
/// data, so `xberg_native_pdf` counts an AES-256 document's pages without ever authenticating.
/// No password handling is needed here, and adding it would buy nothing.
///
/// A document neither parser can count is let through rather than rejected. `max_pages`
/// is opt-in (`SecurityLimits` defaults it to `None`, the fast path below), and
/// failing closed on "cannot tell" would turn a parse failure into a spurious
/// `TooManyPages`, masking the real error on documents the extraction path may still
/// recover.
#[cfg(feature = "pdf")]
fn enforce_page_limit(content: &[u8], config: &ExtractionConfig) -> Result<()> {
    let max_pages = config.security_limits.as_ref().and_then(|limits| limits.max_pages);
    let Some(max_pages) = max_pages else {
        return Ok(());
    };

    let Some(page_count) = native_page_count(content).or_else(|| lopdf_page_count(content)) else {
        return Ok(());
    };

    Ok(crate::extractors::security::enforce_page_count(
        page_count,
        Some(max_pages),
    )?)
}

/// Whether a failed OCR fallback has left nothing at all to return.
///
/// The automatic fallback exists to replace native text that scored below the OCR trigger, so on
/// failure, returning that native text with a warning is right whenever there is some. When there
/// is none, "success with empty content" is indistinguishable from a legitimately empty PDF: the
/// caller cannot tell that OCR was required and produced nothing, because `quality_score` is
/// documented as not a completeness signal and `ProcessingWarning` is free text.
///
/// `run_ocr_pipeline_for_page` already builds a typed error for exactly this case, and its comment
/// states the intent — "Degrading a per-page failure to a warning must not turn a wholesale OCR
/// failure into a silently empty document". That guarantee was defeated one frame up, in the
/// automatic-fallback arms of `extract_core_native`. The `force_ocr` path never had the bug: it
/// propagates the identical error with `?`.
///
/// Whitespace-only counts as nothing: a document yielding `"\n\n  "` is as total a loss as `""`.
/// ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn failed_ocr_fallback_is_total_loss(native_text: &str) -> bool {
    native_text.trim().is_empty()
}

#[cfg(feature = "ocr-pipeline")]
use crate::plugins::registry::automatic_ocr_backend_is_registered;

/// Page count via `xberg_native_pdf`. `None` when it cannot open or count the document,
/// in which case the caller falls back to [`lopdf_page_count`].
#[cfg(feature = "pdf")]
fn native_page_count(content: &[u8]) -> Option<usize> {
    xberg_native_pdf::PdfDocument::from_bytes(content.to_vec())
        .ok()?
        .page_count()
        .ok()
}

/// Page count via `lopdf`, for documents `xberg_native_pdf` could not open or count.
#[cfg(feature = "pdf")]
fn lopdf_page_count(content: &[u8]) -> Option<usize> {
    let document = lopdf::Document::load_mem(content).ok()?;
    Some(document.get_pages().len())
}

#[cfg(feature = "pdf")]
fn parsed_pdf_needs_lopdf_compatibility_pass(document: &crate::pdf::native::NativeDocument) -> bool {
    match document.doc.catalog() {
        Ok(catalog) => catalog_needs_lopdf_compatibility_pass(Some(&catalog)),
        Err(error) => {
            tracing::debug!(
                error = %error,
                "xberg_native_pdf catalog inspection failed; retaining lopdf compatibility pass"
            );
            catalog_needs_lopdf_compatibility_pass(None)
        }
    }
}

#[cfg(feature = "pdf")]
fn extract_lopdf_compatibility_data(
    content: &[u8],
) -> (
    Vec<crate::pdf::bookmarks::PdfOutlineEntry>,
    Vec<crate::types::ExtractedUri>,
    Option<Vec<crate::types::DocumentRevision>>,
) {
    match lopdf::Document::load_mem(content) {
        Ok(lopdf_doc) => {
            let entries = crate::pdf::bookmarks::extract_outline_entries(&lopdf_doc);
            let uris = crate::pdf::bookmarks::extract_bookmarks_from_entries(&entries);
            let revisions = crate::pdf::xref_revisions::extract_pdf_xref_revisions(content, &lopdf_doc);
            (entries, uris, revisions)
        }
        Err(_) => (Vec::new(), Vec::new(), None),
    }
}

#[cfg(feature = "pdf")]
struct NativeBlockingOutput {
    extraction: extraction::PdfExtractionPhaseResult,
    bookmark_uris: Vec<crate::types::ExtractedUri>,
    pdf_revisions: Option<Vec<crate::types::DocumentRevision>>,
    diagrams: Vec<crate::types::diagram::DiagramGraph>,
    #[cfg(feature = "layout-detection")]
    layout_images: Option<Vec<image::RgbImage>>,
    #[cfg(feature = "layout-detection")]
    layout_hints: Option<Vec<Vec<crate::pdf::structure::types::LayoutHint>>>,
    #[cfg(feature = "layout-detection")]
    layout_acceleration_override: Option<crate::core::config::acceleration::AccelerationConfig>,
}

#[cfg(feature = "pdf")]
fn ensure_pdf_not_cancelled(config: &ExtractionConfig) -> Result<()> {
    if config
        .cancel_token
        .as_ref()
        .is_some_and(crate::cancellation::CancellationToken::is_cancelled)
    {
        return Err(crate::XbergError::Cancelled);
    }
    Ok(())
}

#[cfg(all(feature = "pdf", feature = "tokio-runtime", not(target_arch = "wasm32")))]
async fn spawn_pdf_blocking<F, T>(work: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| crate::XbergError::Other(format!("native PDF extraction task failed: {error}")))?
}

#[cfg(all(feature = "pdf", any(not(feature = "tokio-runtime"), target_arch = "wasm32")))]
async fn spawn_pdf_blocking<F, T>(work: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    work()
}

#[cfg(feature = "pdf")]
fn extract_native_blocking(
    content: Vec<u8>,
    config: ExtractionConfig,
    #[cfg(feature = "layout-detection")] layout_images: Option<Vec<image::RgbImage>>,
    #[cfg(feature = "layout-detection")] layout_results: Option<Vec<crate::pdf::structure::types::PageLayoutResult>>,
    #[cfg(feature = "layout-detection")] layout_hints: Option<Vec<Vec<crate::pdf::structure::types::LayoutHint>>>,
    #[cfg(feature = "layout-detection")] layout_acceleration_override: Option<
        crate::core::config::acceleration::AccelerationConfig,
    >,
) -> Result<NativeBlockingOutput> {
    ensure_pdf_not_cancelled(&config)?;
    let passwords = config
        .pdf_options
        .as_ref()
        .and_then(|options| options.passwords.as_deref())
        .unwrap_or(&[]);
    let raw_compatibility_signal = raw_pdf_needs_lopdf_compatibility_pass(&content);
    let compatibility_data = raw_compatibility_signal.then(|| extract_lopdf_compatibility_data(&content));
    let mut native_document = crate::pdf::native::NativeDocument::open_bytes_with_passwords(&content, passwords)?;

    let compatibility_data = match compatibility_data {
        Some(data) => Some(data),
        None if parsed_pdf_needs_lopdf_compatibility_pass(&native_document) => {
            drop(native_document);
            let data = extract_lopdf_compatibility_data(&content);
            native_document = crate::pdf::native::NativeDocument::open_bytes_with_passwords(&content, passwords)?;
            Some(data)
        }
        None => None,
    };
    let (outline_entries, bookmark_uris, pdf_revisions) = compatibility_data.unwrap_or_default();
    ensure_pdf_not_cancelled(&config)?;
    let diagrams = if wants_dot_output(&config) {
        crate::extraction::diagram::pdf::recover(&mut native_document)
    } else {
        Vec::new()
    };
    let extraction = extract_all_from_native_document(
        native_document,
        &config,
        &outline_entries,
        #[cfg(feature = "layout-detection")]
        layout_hints.as_deref(),
        #[cfg(not(feature = "layout-detection"))]
        None,
        #[cfg(feature = "layout-detection")]
        layout_images.as_deref(),
        #[cfg(not(feature = "layout-detection"))]
        None,
        #[cfg(feature = "layout-detection")]
        layout_results.as_deref(),
        #[cfg(not(feature = "layout-detection"))]
        None,
        #[cfg(feature = "layout-detection")]
        layout_acceleration_override.as_ref(),
    )?;
    ensure_pdf_not_cancelled(&config)?;

    Ok(NativeBlockingOutput {
        extraction,
        bookmark_uris,
        pdf_revisions,
        diagrams,
        #[cfg(feature = "layout-detection")]
        layout_images,
        #[cfg(feature = "layout-detection")]
        layout_hints,
        #[cfg(feature = "layout-detection")]
        layout_acceleration_override,
    })
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PdfDocumentOrigin {
    Native,
    Mixed,
    Ocr,
}

/// Flat paragraph document for PDF text that arrived without structure.
///
/// `text` is whichever of native or OCR text won the extraction-method decision, and
/// neither is guaranteed LF-only: `crate::pdf::text::fix_pdf_control_chars` explicitly
/// whitelists `\r` as a character to preserve, and the VLM OCR backend returns model
/// markdown verbatim from an HTTP response. Normalize before splitting (#316).
/// `boundaries` must describe `text` itself, not some earlier revision of it -- on the
/// mixed path that means the output of `ocr::boundaries_after_replacements`, since OCR
/// replacement shifts every later offset. When supplied, each paragraph is tagged with
/// the page its start offset falls in.
///
/// Tagging matters well beyond bookkeeping: `ExtractedDocument.pages` is derived from
/// `element.page` (`extraction::derive::build_pages`), so a flat document with no page
/// tags yields `pages: None` -- indistinguishable at the call site from "this document
/// has no pages", and a silent shape change between the native and OCR paths for the
/// same input. Every arm that falls back here previously lost per-page access entirely. ~keep
fn flat_pdf_document(
    text: &str,
    mime_type: &str,
    boundaries: Option<&[crate::types::PageBoundary]>,
) -> InternalDocument {
    let mut doc = InternalDocument::new("pdf");
    doc.mime_type = mime_type.to_string();
    let normalized = crate::extraction::transform::normalize_line_endings(text);

    // Normalization rewrites CRLF to LF, which shifts offsets; boundaries are only
    // usable when it changed nothing. Falling back to untagged paragraphs is the
    // pre-existing behaviour, so a CRLF document is no worse off than before.
    let usable: Option<&[crate::types::PageBoundary]> = boundaries.filter(|bounds| {
        normalized.len() == text.len()
            && !bounds.is_empty()
            && bounds
                .iter()
                .all(|b| b.byte_start <= b.byte_end && b.byte_end <= normalized.len())
    });

    let page_at = |offset: usize| -> Option<u32> {
        usable?
            .iter()
            .find(|b| offset >= b.byte_start && offset < b.byte_end)
            .map(|b| b.page_number)
    };

    let mut cursor = 0usize;
    for raw in normalized.split("\n\n") {
        let offset = cursor;
        cursor += raw.len() + 2; // the split consumed the two-byte separator
        let paragraph = raw.trim();
        if paragraph.is_empty() {
            continue;
        }
        // Offset of the trimmed text, so a paragraph sitting just after a page break is
        // attributed to the page its visible content is on rather than the previous one.
        let text_offset = offset + (raw.len() - raw.trim_start().len());
        let element = InternalElement::text(ElementKind::Paragraph, paragraph, 0);
        doc.push_element(match page_at(text_offset) {
            Some(page) => element.with_page(page),
            None => element,
        });
    }
    doc
}

#[cfg(feature = "pdf")]
fn apply_extracted_pdf_metadata(
    metadata: &mut Metadata,
    extracted: crate::pdf::metadata::PdfExtractionMetadata,
    enabled: bool,
) {
    if !enabled {
        return;
    }

    metadata.title = extracted.title;
    metadata.subject = extracted.subject;
    metadata.authors = extracted.authors;
    metadata.keywords = extracted.keywords;
    metadata.created_at = extracted.created_at;
    metadata.modified_at = extracted.modified_at;
    metadata.created_by = extracted.created_by;
    metadata.pages = extracted.page_structure;
    metadata.format = Some(crate::types::FormatMetadata::Pdf(extracted.pdf_specific));
}

const MIN_NATIVE_TOKENS_FOR_STRUCTURE_COVERAGE: usize = 20;
const MIN_STRUCTURED_NATIVE_TOKEN_COVERAGE: f64 = 0.70;

fn normalized_pdf_tokens(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split_whitespace()
        .map(|word| {
            word.trim_matches(|character: char| character.is_ascii_punctuation())
                .to_lowercase()
        })
        .filter(|word| !word.is_empty())
}

fn structured_native_token_coverage(document: &InternalDocument, native_text: &str) -> Option<f64> {
    let native_tokens = normalized_pdf_tokens(native_text).collect::<Vec<_>>();
    if native_tokens.len() < MIN_NATIVE_TOKENS_FOR_STRUCTURE_COVERAGE {
        return None;
    }
    let native_token_count = native_tokens.len();

    let mut represented = ahash::AHashMap::<String, usize>::new();
    for token in document
        .elements
        .iter()
        .flat_map(|element| normalized_pdf_tokens(&element.text))
        .chain(
            document
                .tables
                .iter()
                .flat_map(|table| table.cells.iter().flatten())
                .flat_map(|cell| normalized_pdf_tokens(cell)),
        )
    {
        *represented.entry(token).or_default() += 1;
    }

    let mut matched = 0usize;
    for token in native_tokens {
        if let Some(remaining) = represented.get_mut(&token)
            && *remaining > 0
        {
            *remaining -= 1;
            matched += 1;
        }
    }
    Some(matched as f64 / native_token_count as f64)
}

fn select_native_pdf_document(
    text: &str,
    mime_type: &str,
    pre_rendered_doc: Option<InternalDocument>,
    boundaries: Option<&[crate::types::PageBoundary]>,
) -> (InternalDocument, bool) {
    let Some(mut document) = pre_rendered_doc else {
        return (flat_pdf_document(text, mime_type, boundaries), false);
    };
    document.mime_type = mime_type.to_string();

    let coverage = structured_native_token_coverage(&document, text);
    if coverage.is_none_or(|coverage| coverage >= MIN_STRUCTURED_NATIVE_TOKEN_COVERAGE) {
        return (document, true);
    }

    tracing::warn!(
        coverage = coverage.unwrap_or_default(),
        minimum_coverage = MIN_STRUCTURED_NATIVE_TOKEN_COVERAGE,
        "PDF structure omitted substantial native text; using complete native text"
    );
    (flat_pdf_document(text, mime_type, boundaries), false)
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[allow(clippy::too_many_arguments)]
fn select_pdf_document(
    extraction_method: ExtractionMethod,
    text: &str,
    mime_type: &str,
    pre_rendered_doc: Option<InternalDocument>,
    ocr_internal_doc: Option<InternalDocument>,
    ocr_results: Option<&ahash::AHashMap<u32, String>>,
    structured_ocr_pages: Option<&ahash::AHashMap<u32, InternalDocument>>,
    boundaries: Option<&[crate::types::PageBoundary]>,
    output_format: &crate::core::config::OutputFormat,
) -> (InternalDocument, PdfDocumentOrigin, bool) {
    let (mut doc, origin, structured) = match extraction_method {
        ExtractionMethod::Native => {
            let (document, structured) = select_native_pdf_document(text, mime_type, pre_rendered_doc, boundaries);
            (document, PdfDocumentOrigin::Native, structured)
        }
        ExtractionMethod::Mixed => {
            // A native document with no text layer at all (a fully scanned PDF) makes
            // `pre_rendered_doc` `None`: `extract_document_structure_from_segments` ran
            // over zero segments and its caller (`extraction.rs`) collapses an empty
            // result to `None` rather than an empty `InternalDocument`. That used to fall
            // straight through to `flat_pdf_document`, which only ever emits `Paragraph`
            // elements from `text` -- discarding every heading/list-item the document-global
            // OCR heuristic (`ocr::heuristically_restructured_ocr_pages`) recovered into
            // `structured_ocr_pages`, because nothing here ever looked at that map. `text`
            // has already had the OCR replacements applied (see `apply_ocr_page_replacements`
            // upstream) so `flat_pdf_document` still seeds the right per-page paragraphs for
            // every page, OCR'd or not; the merge below then replaces the OCR'd pages'
            // flattened paragraphs with their structured elements, exactly like the
            // `Some(pre_rendered_doc)` branch already does for a native document.
            //
            // Gated on `output_format != Plain`: `render_plain` renders straight off
            // `doc.elements`, so merging in `structured_ocr_pages` (paragraph-segmented
            // per-page OCR documents, not the raw line-broken text `flat_pdf_document`
            // splits on) would change Plain-format bytes for a document that previously
            // never took this branch's merge path. Plain output for this `None` case
            // must stay exactly what `flat_pdf_document(text, ..)` alone produced before
            // this fix.
            let had_native_document = pre_rendered_doc.is_some();
            let mut doc = pre_rendered_doc.unwrap_or_else(|| flat_pdf_document(text, mime_type, boundaries));
            let should_merge = had_native_document || *output_format != crate::core::config::OutputFormat::Plain;
            let merged_ocr_content = if should_merge && let Some(results) = ocr_results {
                if let Some(structured_pages) = structured_ocr_pages {
                    ocr::merge_structured_ocr_pages_into_internal_document(&mut doc, results, structured_pages);
                } else {
                    ocr::merge_ocr_pages_into_internal_document(&mut doc, results);
                }
                true
            } else {
                false
            };
            (doc, PdfDocumentOrigin::Mixed, had_native_document || merged_ocr_content)
        }
        ExtractionMethod::Ocr => match ocr_internal_doc {
            Some(doc) => (doc, PdfDocumentOrigin::Ocr, true),
            None => (
                flat_pdf_document(text, mime_type, boundaries),
                PdfDocumentOrigin::Ocr,
                false,
            ),
        },
    };
    doc.mime_type = mime_type.to_string();
    (doc, origin, structured)
}

fn attach_unrepresented_tables(doc: &mut InternalDocument, tables: Vec<crate::types::Table>) {
    if doc.tables.is_empty() {
        for table in tables {
            doc.push_table(table);
        }
    }
}

/// Share of a table's cell tokens that must already be in the element stream
/// before the table counts as represented.
const MIN_TABLE_TOKEN_REPRESENTATION: f64 = 0.90;
/// Cell-token count below which the containment check abstains: on a handful of
/// tokens an incidental overlap with unrelated prose is likely, and injecting a
/// duplicate is cheaper than dropping a real table.
const MIN_TABLE_TOKENS_FOR_CONTAINMENT: usize = 8;

/// Tokens the element stream already renders, as a consumable multiset.
fn element_token_multiset(doc: &InternalDocument) -> ahash::AHashMap<String, usize> {
    let mut represented = ahash::AHashMap::<String, usize>::new();
    for token in doc
        .elements
        .iter()
        .flat_map(|element| normalized_pdf_tokens(&element.text))
    {
        *represented.entry(token).or_default() += 1;
    }
    represented
}

/// Whether `table`'s cell text is already carried by the element stream.
///
/// Compares on `normalized_pdf_tokens`, so whitespace, line wrapping and edge
/// punctuation do not matter — a table reconstructed from prose lines that are
/// still in the stream verbatim reads as represented. Accounting is multiset
/// based like [`structured_native_token_coverage`]: each text occurrence backs at
/// most one cell occurrence, and the occurrences are consumed only when the table
/// is judged represented, so one table cannot mask the next.
///
/// Both bailouts err toward returning `false` (inject), because a duplicated
/// table costs precision while a dropped one costs content.
fn table_is_represented(table: &crate::types::Table, represented: &mut ahash::AHashMap<String, usize>) -> bool {
    let mut table_tokens = ahash::AHashMap::<String, usize>::new();
    let mut table_token_count = 0usize;
    for token in table
        .cells
        .iter()
        .flatten()
        .flat_map(|cell| normalized_pdf_tokens(cell))
    {
        *table_tokens.entry(token).or_default() += 1;
        table_token_count += 1;
    }
    if table_token_count < MIN_TABLE_TOKENS_FOR_CONTAINMENT {
        return false;
    }

    let matched: usize = table_tokens
        .iter()
        .map(|(token, count)| (*count).min(represented.get(token).copied().unwrap_or_default()))
        .sum();
    if (matched as f64 / table_token_count as f64) < MIN_TABLE_TOKEN_REPRESENTATION {
        return false;
    }

    for (token, count) in table_tokens {
        if let Some(remaining) = represented.get_mut(&token) {
            *remaining = remaining.saturating_sub(count);
        }
    }
    true
}

fn inject_unrepresented_table_elements(doc: &mut InternalDocument, allow_injection: bool) {
    if !allow_injection
        || doc
            .elements
            .iter()
            .any(|element| matches!(element.kind, ElementKind::Table { .. }))
    {
        return;
    }

    // The `Table`-element guard above only ever fires on the structured path;
    // `flat_pdf_document` builds nothing but `Paragraph`s, so on the flat path
    // every detected table was injected on top of native text that already
    // contained the words it was reconstructed from — the same content rendered
    // twice (pdfa_031). Inject only the tables whose cells are genuinely absent. ~keep
    let mut represented = element_token_multiset(doc);
    let unrepresented = (0..doc.tables.len() as u32)
        .filter(|table_index| !table_is_represented(&doc.tables[*table_index as usize], &mut represented))
        .collect::<Vec<_>>();
    for table_index in unrepresented {
        doc.push_element(InternalElement::text(ElementKind::Table { table_index }, "", 0));
    }
}

/// Surface filled AcroForm/XFA field values in rendered content (issue #64).
///
/// `doc.form_fields` already reaches the typed `ExtractedDocument.form_fields`
/// API, but nothing renders it into `content`. For the plain-text path this is
/// usually harmless: `native::text`'s `append_missing_widget_values` already
/// splices Widget `/V` values into the flat native text before it is chopped
/// into `Paragraph` elements. But the *structured* path (Markdown/HTML/Djot,
/// built from `pdf::native::hierarchy`'s span segments) never sees that
/// splice — a filled, non-flattened form renders with none of its entered
/// values.
///
/// Follows `inject_unrepresented_table_elements`'s pattern: push one
/// `ElementKind::Paragraph` per field that has a non-empty value and isn't
/// already present verbatim somewhere in the document (the containment check
/// is what keeps the plain-text path, which already has the value, from
/// getting a duplicate).
fn inject_unrepresented_form_field_elements(doc: &mut InternalDocument, form_fields: &[crate::types::PdfFormField]) {
    for field in form_fields {
        let Some(value) = field.value.as_ref().filter(|v| !v.is_empty()) else {
            continue;
        };
        if doc.elements.iter().any(|element| element.text.contains(value.as_str())) {
            continue;
        }
        let display_name = if field.full_name.is_empty() {
            field.name.as_str()
        } else {
            field.full_name.as_str()
        };
        doc.push_element(InternalElement::text(
            ElementKind::Paragraph,
            format!("{display_name}: {value}"),
            0,
        ));
    }
}

/// Compute language/dictionary-implausible pages (issue #1696), record them on
/// `pdf_metadata.pdf_specific.implausible_text_pages`, and merge them into
/// `pdf_metadata.pdf_specific.scanned_pages` so `OcrStrategy::ScannedPages` (via
/// `scanned_pages_to_ocr`, which reads `scanned_pages`) picks the signal up for free --
/// mirroring how `native::metadata::ocr_routing_pages` already merges `fabricated_text_pages`
/// into the same field for the provenance signal.
///
/// Runs unconditionally (using the default `OcrQualityThresholds` when `config.ocr` is `None`)
/// so `ScannedPages` and the metadata field both see the signal regardless of whether the
/// caller configured OCR explicitly. Whenever `Auto` will not act on the signal, a deduped
/// warning is pushed instead, keeping the defect visible rather than silently discarded; when
/// it will, the warning is suppressed, since it would otherwise tell the caller the pages were
/// dropped on the floor while `Auto` was in fact about to route them. That "will it run"
/// question is answered by `near_empty_ocr_fallback_applies` and nothing else -- see the
/// comment on the warning below for why a second derivation of it is not acceptable. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn record_implausible_text_pages(
    config: &ExtractionConfig,
    pdf_metadata: &mut crate::pdf::metadata::PdfExtractionMetadata,
    native_text: &str,
    boundaries: Option<&[crate::types::PageBoundary]>,
    warnings: &mut Vec<crate::types::ProcessingWarning>,
) {
    let default_ocr_config = crate::core::config::OcrConfig::default();
    let ocr_config = config.ocr.as_ref().unwrap_or(&default_ocr_config);
    let thresholds = ocr_config.effective_thresholds();

    let scan = ocr::scan_text_plausibility(
        native_text,
        boundaries,
        pdf_metadata.pdf_specific.page_count,
        &thresholds,
    );

    pdf_metadata.pdf_specific.implausible_text_pages = thresholds
        .enable_plausibility_ocr_routing
        .then(|| scan.implausible.clone());

    // An abstention is not a pass. Say so, or an empty `implausible_text_pages` reads as a
    // clean bill of health on a document the check never got to look at (issue #1709). Forced
    // OCR is the exception: it discards the native layer, so the caller never receives the text
    // this notice is about and has already taken the action it would advise. ~keep
    if !config.force_ocr && scan.judged == 0 && !scan.unjudged.is_empty() {
        crate::core::diagnostics::push_warning_deduped(
            warnings,
            crate::types::ProcessingWarning {
                source: std::borrow::Cow::Borrowed("ocr"),
                message: std::borrow::Cow::Owned(format!(
                    "No page of this document holds enough prose to read, so the language check \
                     for a wrong glyph-to-Unicode mapping could not judge any of the {} page(s) \
                     it examined (issue #1709). An empty `implausible_text_pages` reports here \
                     that the check did not run, not that the text layer is correct. If the text \
                     looks wrong, extract the document again with OCR forced.",
                    scan.unjudged.len()
                )),
            },
        );
    }

    let implausible_pages = scan.implausible;
    if implausible_pages.is_empty() {
        return;
    }

    let mut merged = pdf_metadata.pdf_specific.scanned_pages.clone().unwrap_or_default();
    merged.extend(implausible_pages.iter().copied());
    merged.sort_unstable();
    merged.dedup();
    pdf_metadata.pdf_specific.scanned_pages = Some(merged);

    // GH#1890: this must call the routing predicate, never re-derive it. The earlier
    // `config.ocr.is_none() && ocr_near_empty_fallback != Some(true)` agreed with
    // `near_empty_ocr_fallback_applies` only until GH#1752 added `Some(false)`: with an `ocr`
    // block and the fallback switched off, the `else if` in `extract_core_native` that gates the
    // routing does not run AND that condition suppressed this warning, so the caller got garbage
    // text with no diagnostic at all. `Some(true)` with no `ocr` block and no registered automatic
    // backend diverged the same way. ~keep
    if !near_empty_ocr_fallback_applies(config, native_text) {
        crate::core::diagnostics::push_warning_deduped(
            warnings,
            crate::types::ProcessingWarning {
                source: std::borrow::Cow::Borrowed("ocr"),
                message: std::borrow::Cow::Owned(format!(
                    "Page(s) {implausible_pages:?} do not read as any real detectable language, \
                     suggesting a wrong glyph-to-Unicode mapping (issue #1696), but the automatic \
                     OCR fallback does not run for this extraction, so they were not routed to \
                     OCR. Set `ocr`, and leave `ocr_near_empty_fallback` unset or set it to \
                     `true`, to route these pages to OCR."
                )),
            },
        );
    }
}

/// Whether `OcrStrategy::ScannedPages` folds the per-page text-quality gate into its page
/// selection, on top of the pages scan detection flagged.
///
/// `ExtractionConfig::ocr_scanned_page_quality_gate` is the caller's explicit answer;
/// `None` derives it from whether an `ocr` block is present, which is what this condition
/// was before GH#1752 gave the behaviour a setting of its own. `Some(true)` without an `ocr`
/// block also requires a registered automatic backend, mirroring the sibling
/// `near_empty_ocr_fallback_applies`'s own `Some(true)` arm (its doc comment states the same
/// requirement) -- without this, a whole-document failure inside the gate selected every page
/// for OCR with nothing able to run it, turning a would-be native-text degradation into a
/// hard extraction failure. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn scanned_page_quality_gate_enabled(config: &ExtractionConfig) -> bool {
    config
        .ocr_scanned_page_quality_gate
        .map(|on| on && (config.ocr.is_some() || automatic_ocr_backend_is_registered()))
        .unwrap_or(config.ocr.is_some())
}

/// Whether the `Auto` near-empty fallback branch runs for this document.
///
/// `ExtractionConfig::ocr_near_empty_fallback` is the caller's explicit answer:
///
/// - `Some(true)` runs the branch whenever there is something to run it with, so the
///   text-quality gate inside it -- not the much narrower "no text at all" test -- decides.
///   An automatic OCR backend must be registered, exactly as #1338's carve-out requires.
/// - `Some(false)` never runs it, even with an explicit `ocr` block.
/// - `None` derives the answer the way this condition did before GH#1752 gave the behaviour
///   a setting of its own: always with an `ocr` block, and otherwise only for genuinely
///   absent native text, so a legitimately sparse PDF stays native under a default config.
///   ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn near_empty_ocr_fallback_applies(config: &ExtractionConfig, native_text: &str) -> bool {
    match config.ocr_near_empty_fallback {
        Some(false) => false,
        Some(true) => config.ocr.is_some() || automatic_ocr_backend_is_registered(),
        None => config.ocr.is_some() || (native_text.trim().is_empty() && automatic_ocr_backend_is_registered()),
    }
}

/// Pages to OCR under `OcrStrategy::ScannedPages`, 1-indexed.
///
/// The union of detected scans and pages failing the text-quality gate, so never
/// a subset of what `Auto` would OCR -- unless the gate is off, in which case only
/// detected scans are selected. See [`scanned_page_quality_gate_enabled`].
///
/// `None` means fall through to the `Auto` gate: wrong strategy, no page
/// qualifies, or the gate wants the whole document rather than a page subset.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn scanned_pages_to_ocr(
    config: &ExtractionConfig,
    pdf_metadata: &crate::pdf::metadata::PdfExtractionMetadata,
    native_text: &str,
    boundaries: Option<&[crate::types::PageBoundary]>,
) -> Option<Vec<u32>> {
    use crate::core::config::OcrStrategy;

    if !matches!(config.ocr_strategy, OcrStrategy::ScannedPages { .. }) {
        return None;
    }

    let mut pages = pdf_metadata.pdf_specific.scanned_pages.clone()?;

    if scanned_page_quality_gate_enabled(config) {
        // Thresholds come from the caller's `ocr` block when there is one; the gate can now
        // be switched on without one, and then reads the same defaults every other
        // no-explicit-block OCR route reads. ~keep
        let default_ocr_config = crate::core::config::OcrConfig::default();
        let ocr_config = config.ocr.as_ref().unwrap_or(&default_ocr_config);
        let decision = ocr::evaluate_per_page_ocr(
            native_text,
            boundaries,
            pdf_metadata.pdf_specific.page_count,
            &ocr_config.effective_thresholds(),
        );
        if decision.whole_doc_failure {
            // A whole-document failure is itself a `ScannedPages` signal (see
            // discarding the signal and relying on the caller's `Auto` fallthrough.
            let page_count = pdf_metadata.pdf_specific.page_count.unwrap_or(0);
            return if page_count == 0 {
                None
            } else {
                Some((1..=page_count).collect())
            };
        }
        pages.extend(decision.failing_pages);
    }

    pages.sort_unstable();
    pages.dedup();

    if pages.is_empty() { None } else { Some(pages) }
}
use pages::{assign_hierarchy_to_pages, assign_tables_and_images_to_pages};

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn replace_tables_with_ocr_output(tables: &mut Vec<crate::types::Table>, mut ocr_tables: Vec<crate::types::Table>) {
    if ocr_tables.is_empty() {
        return;
    }

    ocr_tables.sort_by_key(|table| table.page_number);
    *tables = ocr_tables;
}

/// Merge OCR tables into the mixed path's table list, replacing only the tables of pages OCR
/// itself produced a table for.
///
/// Unlike [`replace_tables_with_ocr_output`], which is right for full-document OCR (every page
/// went through OCR, so there is no native page left to preserve), the mixed path sends only
/// SOME pages to OCR. Before this, the mixed path called `replace_tables_with_ocr_output` too,
/// which replaced the WHOLE document's table list with only the OCR pages' tables: on a long
/// document where a handful of scanned pages produced even one OCR table, every native table on
/// every page OCR never touched was silently dropped (GH#1670). A page whose OCR pass found no
/// table keeps its native tables untouched, the same as `replace_tables_with_ocr_output` does
/// when it finds none anywhere.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn merge_mixed_ocr_tables(
    tables: &mut Vec<crate::types::Table>,
    structured_pages: &ahash::AHashMap<u32, InternalDocument>,
) {
    let mut ocr_tables = accepted_mixed_ocr_tables(structured_pages);
    if ocr_tables.is_empty() {
        return;
    }

    ocr_tables.sort_by_key(|table| table.page_number);
    let ocr_pages: std::collections::HashSet<u32> = ocr_tables.iter().map(|table| table.page_number).collect();
    tables.retain(|table| !ocr_pages.contains(&table.page_number));
    tables.extend(ocr_tables);
    tables.sort_by_key(|table| table.page_number);
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-pipeline")))]
fn prepare_ocr_layout_inputs(
    images: Vec<image::RgbImage>,
    mut detections: Vec<crate::layout::DetectionResult>,
) -> (Vec<image::DynamicImage>, Vec<crate::layout::DetectionResult>) {
    if detections.len() != images.len() {
        tracing::warn!(
            images = images.len(),
            detections = detections.len(),
            "OCR layout input cardinality mismatch; discarding detections while reusing page rasters"
        );
        detections = images
            .iter()
            .map(|image| crate::layout::DetectionResult {
                page_width: image.width(),
                page_height: image.height(),
                detections: Vec::new(),
            })
            .collect();
    } else {
        for (page_index, (image, detection)) in images.iter().zip(&mut detections).enumerate() {
            if detection.page_width != image.width() || detection.page_height != image.height() {
                tracing::warn!(
                    page = page_index + 1,
                    image_width = image.width(),
                    image_height = image.height(),
                    detection_width = detection.page_width,
                    detection_height = detection.page_height,
                    "OCR layout dimensions mismatch; discarding detections for this page"
                );
                *detection = crate::layout::DetectionResult {
                    page_width: image.width(),
                    page_height: image.height(),
                    detections: Vec::new(),
                };
            }
        }
    }

    let images = images.into_iter().map(image::DynamicImage::ImageRgb8).collect();
    (images, detections)
}

/// Auto layout gate audit trail: 1-indexed skipped pages and per-page
/// decision reasons, both `None` unless the `Auto` gate ran.
///
/// Unused only when both OCR features and the pdf + layout-detection pair
/// are off; the allow is scoped wider because cfg algebra for "either user
/// present" is not worth the noise.
#[cfg_attr(not(any(feature = "ocr", feature = "ocr-pipeline")), allow(dead_code))]
type OcrLayoutGateDecisions = (Option<Vec<u32>>, Option<Vec<String>>);

/// Whether the markdown layout pass's rasters may be reused as OCR input.
///
/// Under `LayoutStrategy::Auto` with `SkipRender`, gate-skipped pages carry
/// 64x64 white placeholders. Rotated pages also remain in the display-space
/// coordinate frame required by Markdown layout. Reuse is therefore allowed
/// only when every page ran the model and no page has an effective `/Rotate`;
/// otherwise OCR reruns layout with normalized rasters. ~keep
#[cfg(all(
    feature = "pdf",
    feature = "layout-detection",
    any(feature = "ocr", feature = "ocr-pipeline")
))]
fn markdown_layout_reusable_for_ocr(
    decisions: Option<&[crate::pdf::layout_gate::PageGateDecision]>,
    page_rotations: &[u32],
) -> bool {
    decisions.is_none_or(|decisions| decisions.iter().all(|decision| decision.run_layout))
        && page_rotations.iter().all(|rotation| rotation.is_multiple_of(360))
}

/// Convert gate decisions into the metadata representation.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn layout_gate_metadata(decisions: Option<&[crate::pdf::layout_gate::PageGateDecision]>) -> OcrLayoutGateDecisions {
    let Some(decisions) = decisions else {
        return (None, None);
    };
    let gated = decisions
        .iter()
        .enumerate()
        .filter(|(_, decision)| !decision.run_layout)
        .map(|(page_index, _)| page_index as u32 + 1)
        .collect();
    let reasons = decisions
        .iter()
        .map(|decision| decision.reason.wire_name().to_string())
        .collect();
    (Some(gated), Some(reasons))
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-pipeline")))]
fn config_with_layout_acceleration_override(
    config: &ExtractionConfig,
    acceleration_override: Option<crate::core::config::acceleration::AccelerationConfig>,
) -> std::borrow::Cow<'_, ExtractionConfig> {
    let Some(acceleration) = acceleration_override else {
        return std::borrow::Cow::Borrowed(config);
    };

    let mut effective = config.clone();
    if let Some(layout) = effective.layout.as_mut() {
        layout.acceleration = Some(acceleration);
    } else {
        effective.acceleration = Some(acceleration);
    }
    std::borrow::Cow::Owned(effective)
}

/// Run OCR with optional layout detection on PDF bytes.
///
/// Reuses detections from native extraction when available. Otherwise, when
/// layout detection is configured, it runs a soft-failing layout pass before
/// OCR. Layout failures are logged and OCR continues without layout hints.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
async fn run_ocr_with_layout(
    content: &[u8],
    config: &ExtractionConfig,
    path: Option<&std::path::Path>,
    page_ocr_hints: Option<ocr::PageOcrHints>,
    #[cfg(feature = "layout-detection")] precomputed_layout_images: Option<Vec<image::RgbImage>>,
    #[cfg(feature = "layout-detection")] precomputed_layout_detections: Option<Vec<crate::layout::DetectionResult>>,
    #[cfg(feature = "layout-detection")] precomputed_layout_acceleration_override: Option<
        crate::core::config::acceleration::AccelerationConfig,
    >,
) -> ocr::OcrRunResult<(
    String,
    Vec<crate::types::Table>,
    Vec<crate::types::OcrElement>,
    Option<crate::types::internal::InternalDocument>,
    Vec<crate::types::LlmUsage>,
    Vec<String>,
    Option<Vec<crate::types::ExtractedImage>>,
    Vec<crate::types::Formula>,
    ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata>,
    ahash::AHashMap<u32, crate::types::page::PageOcrConfidence>,
    OcrLayoutGateDecisions,
    Option<crate::types::ProcessingWarning>,
    Vec<crate::types::ProcessingWarning>,
)> {
    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    let mut layout_warning = None;
    #[cfg(not(all(feature = "pdf", feature = "layout-detection")))]
    let layout_warning = None;

    // `xberg_native_pdf` glyph-drop warnings captured while the layout pass rendered
    // its pages (#353); populated only on the layout-detection path, since
    // that is the only render call site routed through `spawn_blocking`.
    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    let mut layout_glyph_drop_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();
    #[cfg(not(all(feature = "pdf", feature = "layout-detection")))]
    let layout_glyph_drop_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();

    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    let mut ocr_layout_gate_decisions: OcrLayoutGateDecisions = (None, None);
    #[cfg(not(all(feature = "pdf", feature = "layout-detection")))]
    let ocr_layout_gate_decisions: OcrLayoutGateDecisions = (None, None);

    #[cfg(feature = "layout-detection")]
    let mut layout_acceleration_override = precomputed_layout_acceleration_override;

    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    let owned_layout = if precomputed_layout_detections.is_none() || precomputed_layout_images.is_none() {
        if let Some(layout_config) = config.resolved_layout_config() {
            let thread_budget = crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());
            let default_security_limits = crate::extractors::security::SecurityLimits::default();
            let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
            match layout_runner::run_layout_for_ocr(
                content,
                layout_config.as_ref(),
                thread_budget,
                security_limits,
                config.images.as_ref(),
            )
            .await
            {
                Ok((
                    layout_runner::LayoutAttempt {
                        output:
                            layout_runner::LayoutRunOutput {
                                data: Some(layout),
                                gate_decisions,
                            },
                        acceleration_override,
                        warning,
                    },
                    glyph_drop_warnings,
                )) => {
                    layout_acceleration_override = acceleration_override;
                    layout_warning = warning;
                    layout_glyph_drop_warnings = glyph_drop_warnings;
                    ocr_layout_gate_decisions = layout_gate_metadata(gate_decisions.as_deref());
                    Some(layout)
                }
                Ok((
                    layout_runner::LayoutAttempt {
                        output:
                            layout_runner::LayoutRunOutput {
                                data: None,
                                gate_decisions,
                            },
                        acceleration_override: _,
                        warning,
                    },
                    glyph_drop_warnings,
                )) => {
                    layout_warning = warning;
                    layout_glyph_drop_warnings = glyph_drop_warnings;
                    tracing::info!("OCR layout: auto gate skipped every page, continuing without layout assembly");
                    ocr_layout_gate_decisions = layout_gate_metadata(gate_decisions.as_deref());
                    None
                }
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        "OCR layout detection failed; continuing without layout assembly"
                    );
                    layout_warning = Some(layout_runner::layout_failure_warning(&error));
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    let layout_inputs = match (precomputed_layout_images, precomputed_layout_detections) {
        (Some(images), Some(detections)) => Some((images, detections)),
        _ => owned_layout.map(|(images, _, _, detections)| (images, detections)),
    };
    #[cfg(all(not(feature = "pdf"), feature = "layout-detection"))]
    let layout_inputs = precomputed_layout_images.zip(precomputed_layout_detections);

    #[cfg(feature = "layout-detection")]
    let prepared_layout_inputs =
        layout_inputs.map(|(images, detections)| prepare_ocr_layout_inputs(images, detections));
    #[cfg(feature = "layout-detection")]
    let ocr_images = prepared_layout_inputs.as_ref().map(|(images, _)| images.as_slice());
    #[cfg(feature = "layout-detection")]
    let layout_detections = prepared_layout_inputs
        .as_ref()
        .map(|(_, detections)| detections.as_slice());

    #[cfg(feature = "layout-detection")]
    let effective_config = config_with_layout_acceleration_override(config, layout_acceleration_override);
    #[cfg(feature = "layout-detection")]
    let config = effective_config.as_ref();

    let default_ocr_config = crate::core::config::OcrConfig::default();
    let ocr_config = config.ocr.as_ref().unwrap_or(&default_ocr_config);

    if let Some(pipeline) = ocr_config.effective_pipeline() {
        #[cfg(paddle_ocr)]
        // The synthesized classical pipeline is an automatic page-quality fallback. Running
        // it here as one document lets a good page's score hide a bad page, so use the same
        // page-local runner as selected-page OCR. Explicit pipelines retain their configured
        // document scope because their stages may intentionally process the document. ~keep
        if ocr_config.pipeline.is_none()
            && ocr_config.vlm_fallback == crate::core::config::VlmFallbackPolicy::Disabled
            && ocr_config.backend == "tesseract"
        {
            let (text, tables, elements, document, usage, page_texts, rasters, formulas, preprocessing, confidence) =
                Box::pin(ocr::extract_full_document_ocr_pipeline_per_page(
                    content,
                    config,
                    path,
                    page_ocr_hints,
                    #[cfg(feature = "layout-detection")]
                    prepared_layout_inputs,
                ))
                .await?;
            return Ok((
                text,
                tables,
                elements,
                document,
                usage,
                page_texts,
                rasters,
                formulas,
                preprocessing,
                confidence,
                ocr_layout_gate_decisions,
                layout_warning,
                layout_glyph_drop_warnings,
            ));
        }

        let (
            mut text,
            mut ocr_tables,
            ocr_elements,
            mut pipeline_doc,
            llm_usage,
            ocr_pts,
            pipeline_rasters,
            pipeline_formulas,
            preprocessing,
            pipeline_ocr_confidence,
        ) = Box::pin(ocr::run_ocr_pipeline(
            Some(content),
            #[cfg(feature = "layout-detection")]
            ocr_images,
            #[cfg(not(feature = "layout-detection"))]
            None,
            #[cfg(feature = "layout-detection")]
            layout_detections,
            config,
            &pipeline,
            path,
            page_ocr_hints,
        ))
        .await?;
        // GH#1892: the whole-document route repaired nothing, so `numeric_repair` was honoured for
        // `force_ocr_pages` and silently ignored for `force_ocr` under one identical flag. Placed
        // ahead of formula recognition on purpose -- see the function's own doc comment. ~keep
        if ocr::numeric_repair_enabled(config) {
            ocr::apply_numeric_repair_to_whole_document_ocr(&mut text, pipeline_doc.as_mut(), &mut ocr_tables);
        }
        #[cfg(feature = "formula-recognition")]
        let (mut pipeline_doc, mut pipeline_formulas) = (pipeline_doc, pipeline_formulas);
        #[cfg(feature = "formula-recognition")]
        if let (Some((images, detections)), Some(layout)) = (prepared_layout_inputs.as_ref(), config.layout.as_ref()) {
            recognize_pdf_formula_regions(
                pipeline_doc.as_mut(),
                &mut pipeline_formulas,
                images,
                detections,
                layout,
                content,
            )
            .await;
        }
        return Ok((
            text,
            ocr_tables,
            ocr_elements,
            pipeline_doc,
            llm_usage,
            ocr_pts,
            pipeline_rasters,
            pipeline_formulas,
            preprocessing,
            pipeline_ocr_confidence,
            ocr_layout_gate_decisions,
            layout_warning,
            layout_glyph_drop_warnings,
        ));
    }

    let (
        mut text,
        _mean_conf,
        mut ocr_tables,
        ocr_elements,
        mut ocr_doc,
        llm_usage,
        ocr_pts,
        ocr_rasters,
        formulas,
        preprocessing,
        ocr_confidence,
    ) = Box::pin(ocr::extract_with_ocr_with_page_hints(
        Some(content),
        #[cfg(feature = "layout-detection")]
        ocr_images,
        #[cfg(not(feature = "layout-detection"))]
        None,
        #[cfg(feature = "layout-detection")]
        layout_detections,
        config,
        path,
        page_ocr_hints,
    ))
    .await?;
    // GH#1892: same repair as the pipeline branch above -- this is the other half of the
    // whole-document route, the one `force_ocr` and the near-empty `Auto` fallback reach. ~keep
    if ocr::numeric_repair_enabled(config) {
        ocr::apply_numeric_repair_to_whole_document_ocr(&mut text, ocr_doc.as_mut(), &mut ocr_tables);
    }
    #[cfg(feature = "formula-recognition")]
    let (mut ocr_doc, mut formulas) = (ocr_doc, formulas);
    #[cfg(feature = "formula-recognition")]
    if let (Some((images, detections)), Some(layout)) = (prepared_layout_inputs.as_ref(), config.layout.as_ref()) {
        recognize_pdf_formula_regions(ocr_doc.as_mut(), &mut formulas, images, detections, layout, content).await;
    }
    Ok((
        text,
        ocr_tables,
        ocr_elements,
        ocr_doc,
        llm_usage,
        ocr_pts,
        ocr_rasters,
        formulas,
        preprocessing,
        ocr_confidence,
        ocr_layout_gate_decisions,
        layout_warning,
        layout_glyph_drop_warnings,
    ))
}

/// The formula detections of one page, sorted by raw `y1` then `x1`.
///
/// This comparator deliberately matches the order the OCR side channel emits
/// formulas in (`glm_ocr_backend::process_paired` sorts its regions the same
/// way) — the pairing below is positional, so both sides must sort
/// identically. It is NOT the row-banded reading order the image extractor
/// uses for element layout; do not "unify" them without changing the
/// pairing to a geometric match.
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn formula_regions_in_reading_order(
    detection: &crate::layout::DetectionResult,
) -> Vec<&crate::layout::LayoutDetection> {
    let mut regions: Vec<&crate::layout::LayoutDetection> = detection
        .detections
        .iter()
        .filter(|d| matches!(d.class_name, crate::layout::LayoutClass::Formula))
        .collect();
    regions.sort_by(|a, b| a.bbox.y1.total_cmp(&b.bbox.y1).then(a.bbox.x1.total_cmp(&b.bbox.x1)));
    regions
}

#[cfg(all(test, feature = "formula-recognition", feature = "layout-detection"))]
mod formula_region_tests {
    use super::formula_regions_in_reading_order;
    use crate::layout::{BBox, DetectionResult, LayoutClass, LayoutDetection};

    fn det(class: LayoutClass, x1: f32, y1: f32) -> LayoutDetection {
        LayoutDetection {
            class_name: class,
            confidence: 0.9,
            bbox: BBox {
                x1,
                y1,
                x2: x1 + 10.0,
                y2: y1 + 10.0,
            },
        }
    }

    #[test]
    fn orders_formulas_top_to_bottom_then_left_to_right() {
        let page = DetectionResult {
            page_width: 100,
            page_height: 100,
            detections: vec![
                det(LayoutClass::Formula, 50.0, 40.0),
                det(LayoutClass::Text, 0.0, 0.0),
                det(LayoutClass::Formula, 10.0, 40.0),
                det(LayoutClass::Formula, 10.0, 5.0),
            ],
        };
        let ordered = formula_regions_in_reading_order(&page);
        let coords: Vec<(f32, f32)> = ordered.iter().map(|d| (d.bbox.y1, d.bbox.x1)).collect();
        assert_eq!(coords, vec![(5.0, 10.0), (40.0, 10.0), (40.0, 50.0)]);
    }

    #[test]
    fn pages_without_formulas_yield_no_regions() {
        let page = DetectionResult {
            page_width: 100,
            page_height: 100,
            detections: vec![det(LayoutClass::Text, 0.0, 0.0), det(LayoutClass::Table, 5.0, 5.0)],
        };
        assert!(formula_regions_in_reading_order(&page).is_empty());
    }

    fn pt_box(x0: f64, y0: f64, x1: f64, y1: f64) -> crate::types::BoundingBox {
        crate::types::BoundingBox { x0, y0, x1, y1 }
    }

    #[test]
    fn matching_counts_pair_positionally() {
        let assigned = super::assign_formula_slots(2, &[None, None], None);
        assert_eq!(assigned, vec![Some(0), Some(1)]);
    }

    #[test]
    fn mismatched_counts_pair_by_overlap_and_leave_the_rest_unassigned() {
        // Three regions, two slots; slots overlap regions 0 and 2.
        let regions = [
            pt_box(10.0, 100.0, 200.0, 130.0),
            pt_box(10.0, 50.0, 200.0, 80.0),
            pt_box(10.0, 10.0, 200.0, 40.0),
        ];
        let slots = [
            Some(pt_box(12.0, 12.0, 190.0, 38.0)),
            Some(pt_box(12.0, 102.0, 190.0, 128.0)),
        ];
        let assigned = super::assign_formula_slots(3, &slots, Some(&regions));
        assert_eq!(assigned, vec![Some(1), None, Some(0)]);
    }

    #[test]
    fn mismatched_counts_without_geometry_assign_nothing() {
        let slots = [Some(pt_box(0.0, 0.0, 10.0, 10.0))];
        let assigned = super::assign_formula_slots(3, &slots, None);
        assert_eq!(assigned, vec![None, None, None]);
    }

    #[test]
    fn slots_without_bboxes_stay_unassigned_on_mismatch() {
        let regions = [pt_box(0.0, 0.0, 100.0, 30.0)];
        let assigned = super::assign_formula_slots(1, &[None, None], Some(&regions));
        assert_eq!(assigned, vec![None]);
    }

    #[test]
    fn synthesis_needs_geometry_when_the_page_carries_formula_text() {
        // No geometry, no text: a scanned page OCR'd to plain text synthesizes.
        assert!(super::formula_synthesis_allowed(false, 0, 0));
        // No geometry, text present: synthesis would duplicate it.
        assert!(!super::formula_synthesis_allowed(false, 1, 0));
        assert!(!super::formula_synthesis_allowed(false, 0, 2));
        // With geometry, unpaired regions synthesize regardless of text.
        assert!(super::formula_synthesis_allowed(true, 3, 1));
    }

    #[test]
    fn overlap_ratio_is_intersection_over_smaller_area() {
        let big = pt_box(0.0, 0.0, 100.0, 100.0);
        let small = pt_box(50.0, 50.0, 150.0, 150.0);
        assert!((super::bbox_overlap_ratio(&big, &small) - 0.25).abs() < 1e-9);
        let contained = pt_box(10.0, 10.0, 20.0, 20.0);
        assert!((super::bbox_overlap_ratio(&big, &contained) - 1.0).abs() < 1e-9);
        assert_eq!(
            super::bbox_overlap_ratio(&big, &pt_box(200.0, 200.0, 300.0, 300.0)),
            0.0
        );
    }
}

/// A page raster the formula recognizer can crop, however the flow stores it.
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
trait AsPageRgb {
    fn page_rgb(&self) -> std::borrow::Cow<'_, image::RgbImage>;
}
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
impl AsPageRgb for image::DynamicImage {
    fn page_rgb(&self) -> std::borrow::Cow<'_, image::RgbImage> {
        std::borrow::Cow::Owned(self.to_rgb8())
    }
}
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
impl AsPageRgb for image::RgbImage {
    fn page_rgb(&self) -> std::borrow::Cow<'_, image::RgbImage> {
        std::borrow::Cow::Borrowed(self)
    }
}

/// Intersection area over the smaller box's area, in a shared coordinate
/// space. 0.0 when either box is degenerate.
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn bbox_overlap_ratio(a: &crate::types::BoundingBox, b: &crate::types::BoundingBox) -> f64 {
    let ix = (a.x1.min(b.x1) - a.x0.max(b.x0)).max(0.0);
    let iy = (a.y1.min(b.y1) - a.y0.max(b.y0)).max(0.0);
    let min_area = ((a.x1 - a.x0) * (a.y1 - a.y0)).min((b.x1 - b.x0) * (b.y1 - b.y0));
    if min_area <= 0.0 { 0.0 } else { (ix * iy) / min_area }
}

/// A detection must cover a slot's box (or vice versa) by at least this
/// fraction of the smaller area to pair with it geometrically.
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
const FORMULA_PAIR_MIN_OVERLAP: f64 = 0.5;

/// Assign each detected region to at most one formula slot on one side
/// (side-channel formulas or formula elements).
///
/// When the counts match, pairing is positional in reading order — the
/// OCR side channel emits its formulas in the same sort, and that invariant
/// must hold (see [`formula_regions_in_reading_order`]). When the counts
/// differ, each region pairs with the unused slot whose bounding box it
/// overlaps most (both in PDF points); regions and slots without a
/// sufficient overlap stay unassigned. Without region geometry
/// (`regions_pts` is `None`), a count mismatch assigns nothing.
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn assign_formula_slots(
    region_count: usize,
    slot_bboxes: &[Option<crate::types::BoundingBox>],
    regions_pts: Option<&[crate::types::BoundingBox]>,
) -> Vec<Option<usize>> {
    if slot_bboxes.len() == region_count {
        return (0..region_count).map(Some).collect();
    }
    let Some(regions_pts) = regions_pts else {
        return vec![None; region_count];
    };
    let mut used = vec![false; slot_bboxes.len()];
    regions_pts
        .iter()
        .map(|region| {
            let mut best: Option<(usize, f64)> = None;
            for (index, slot) in slot_bboxes.iter().enumerate() {
                if used[index] {
                    continue;
                }
                let Some(slot) = slot else { continue };
                let overlap = bbox_overlap_ratio(region, slot);
                if overlap >= FORMULA_PAIR_MIN_OVERLAP && best.is_none_or(|(_, b)| overlap > b) {
                    best = Some((index, overlap));
                }
            }
            best.map(|(index, _)| {
                used[index] = true;
                index
            })
        })
        .collect()
}

/// Whether an unpaired detection may become a new formula on this page.
///
/// Without region geometry, a count mismatch cannot pair regions with the
/// existing formula text, so a synthesized formula would duplicate one the
/// page already carries. Synthesis is safe when geometry exists, or when the
/// page carries no formula text at all (then there is nothing to duplicate).
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn formula_synthesis_allowed(has_region_geometry: bool, formula_slot_count: usize, element_slot_count: usize) -> bool {
    has_region_geometry || (formula_slot_count == 0 && element_slot_count == 0)
}

/// Per-page MediaBox sizes in points, for mapping detection bboxes into PDF
/// point space. `None` when the document cannot be opened (for example an
/// encrypted PDF); synthesized formulas then keep pixel coordinates, the
/// documented fallback for pages whose geometry is unavailable.
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
fn pdf_page_sizes_pt(content: &[u8], page_count: usize) -> Option<Vec<(f32, f32)>> {
    let doc = xberg_native_pdf::PdfDocument::from_bytes(content.to_vec()).ok()?;
    Some(
        (0..page_count)
            .map(|index| crate::pdf::render::get_page_dimensions_pt(&doc, index))
            .collect(),
    )
}

/// Recognize layout-detected formula regions on rendered PDF pages.
///
/// Each detected region pairs with the formula text the extraction side
/// already produced (side-channel formulas and formula elements), per page:
/// positionally in reading order when the counts match, by bounding-box
/// overlap in PDF points when they differ. Recognized LaTeX replaces the
/// text of paired slots. A region paired with nothing — for example on a
/// scanned page whose OCR backend emits plain text only — becomes a new
/// formula with its bbox in PDF points. Failures keep the existing text.
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
async fn recognize_pdf_formula_regions(
    ocr_doc: Option<&mut crate::types::internal::InternalDocument>,
    formulas: &mut Vec<crate::types::Formula>,
    images: &[impl AsPageRgb],
    detections: &[crate::layout::DetectionResult],
    layout: &crate::core::config::LayoutDetectionConfig,
    content: &[u8],
) {
    if layout.formula_model.is_none() {
        return;
    }
    let mut element_slots: Vec<Vec<(Option<crate::types::BoundingBox>, &mut String)>> =
        (0..images.len()).map(|_| Vec::new()).collect();
    if let Some(doc) = ocr_doc {
        for element in doc.elements.iter_mut() {
            if matches!(element.kind, crate::types::internal::ElementKind::Formula)
                && let Some(page) = element.page
                && let Some(slot) = element_slots.get_mut((page as usize).saturating_sub(1))
            {
                slot.push((element.bbox, &mut element.text));
            }
        }
    }
    // Page MediaBox sizes, parsed once on first need (geometric pairing or a
    // synthesized bbox); fully positional pages never pay for the parse.
    let mut page_sizes_pt: Option<Option<Vec<(f32, f32)>>> = None;
    for (page_idx, (image, detection)) in images.iter().zip(detections).enumerate() {
        let regions = formula_regions_in_reading_order(detection);
        let page_number = (page_idx + 1) as u32;
        if regions.is_empty() {
            tracing::debug!(
                page = page_number,
                detections = detection.detections.len(),
                "layout detected no formula regions on this page; formula recognition skipped"
            );
            continue;
        }

        let mut synthesized: Vec<crate::types::Formula> = Vec::new();
        {
            let mut formula_slots: Vec<&mut crate::types::Formula> =
                formulas.iter_mut().filter(|f| f.page == Some(page_number)).collect();
            let elements = element_slots
                .get_mut(page_idx)
                .map(Vec::as_mut_slice)
                .unwrap_or(&mut []);

            let rgb = image.page_rgb();
            let rgb: &image::RgbImage = &rgb;

            let need_geometry = formula_slots.len() != regions.len() || elements.len() != regions.len();
            let page_size = if need_geometry {
                page_sizes_pt
                    .get_or_insert_with(|| pdf_page_sizes_pt(content, images.len()))
                    .as_ref()
                    .and_then(|sizes| sizes.get(page_idx).copied())
            } else {
                None
            };
            let regions_pts: Option<Vec<crate::types::BoundingBox>> = page_size.map(|(page_w_pt, page_h_pt)| {
                regions
                    .iter()
                    .map(|region| {
                        let pixel = crate::types::BoundingBox {
                            x0: f64::from(region.bbox.x1),
                            y0: f64::from(region.bbox.y1),
                            x1: f64::from(region.bbox.x2),
                            y1: f64::from(region.bbox.y2),
                        };
                        crate::pdf::render::pixel_bbox_to_pdf_points(
                            pixel,
                            rgb.width(),
                            rgb.height(),
                            page_w_pt,
                            page_h_pt,
                        )
                    })
                    .collect()
            });
            if need_geometry && (!formula_slots.is_empty() || !elements.is_empty()) {
                if regions_pts.is_some() {
                    tracing::warn!(
                        page = page_number,
                        detections = regions.len(),
                        formulas = formula_slots.len(),
                        elements = elements.len(),
                        "formula counts differ from detections; pairing regions by bounding-box overlap"
                    );
                } else {
                    tracing::warn!(
                        page = page_number,
                        detections = regions.len(),
                        formulas = formula_slots.len(),
                        elements = elements.len(),
                        "formula counts differ from detections and the page geometry is unavailable; keeping OCR text on this page"
                    );
                }
            }
            let allow_synthesis = formula_synthesis_allowed(regions_pts.is_some(), formula_slots.len(), elements.len());
            let formula_boxes: Vec<Option<crate::types::BoundingBox>> = formula_slots.iter().map(|f| f.bbox).collect();
            let element_boxes: Vec<Option<crate::types::BoundingBox>> =
                elements.iter().map(|(bbox, _)| *bbox).collect();
            let formula_assign = assign_formula_slots(regions.len(), &formula_boxes, regions_pts.as_deref());
            let element_assign = assign_formula_slots(regions.len(), &element_boxes, regions_pts.as_deref());

            for (region_idx, region) in regions.iter().enumerate() {
                if formula_assign[region_idx].is_none() && element_assign[region_idx].is_none() && !allow_synthesis {
                    continue;
                }
                let Some((x, y, w, h)) = region.bbox.clamp_to_image(rgb.width(), rgb.height()) else {
                    continue;
                };
                let crop = image::imageops::crop_imm(rgb, x, y, w, h).to_image();
                match crate::formula_recognition::recognize_crop_blocking(crop, layout.acceleration.clone()).await {
                    Ok(Some(latex)) => {
                        let mut replaced = false;
                        if let Some(slot) = formula_assign[region_idx].and_then(|i| formula_slots.get_mut(i)) {
                            slot.latex = latex.clone();
                            replaced = true;
                        }
                        if let Some(entry) = element_assign[region_idx].and_then(|i| elements.get_mut(i)) {
                            *entry.1 = latex.clone();
                            replaced = true;
                        }
                        if !replaced && allow_synthesis {
                            let bbox = regions_pts.as_ref().map(|pts| pts[region_idx]).unwrap_or_else(|| {
                                crate::types::BoundingBox {
                                    x0: f64::from(x),
                                    y0: f64::from(y),
                                    x1: f64::from(x + w),
                                    y1: f64::from(y + h),
                                }
                            });
                            synthesized.push(crate::types::Formula {
                                latex,
                                bbox: Some(bbox),
                                page: Some(page_number),
                            });
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        tracing::warn!(error = %error, page = page_number, "pdf formula recognition failed; keeping OCR text");
                    }
                }
            }
        }
        if !synthesized.is_empty() {
            tracing::info!(
                page = page_number,
                formulas = synthesized.len(),
                "recognized formulas from layout regions with no extracted formula text"
            );
            formulas.extend(synthesized);
        }
    }
}

/// Whether the caller actually asked for the recovered diagram, i.e. the
/// request's output format resolves to the `dot` renderer.
///
/// `OutputFormat::Custom` is how any renderer, built-in or registered, is
/// selected (see `plugins::registry::RendererRegistry`), so matching the
/// renderer name here is the same test `derive_extraction_result` uses to
/// decide which renderer runs — not a looser proxy for it.
#[cfg(feature = "pdf")]
fn wants_dot_output(config: &ExtractionConfig) -> bool {
    matches!(&config.output_format, crate::core::config::OutputFormat::Custom(name) if name == "dot")
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn attach_pdf_preprocessing_metadata(
    pages: &mut Option<Vec<crate::types::PageContent>>,
    by_page: &ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata>,
) -> Option<crate::types::ImagePreprocessingMetadata> {
    if let Some(pages) = pages {
        for page in pages {
            page.image_preprocessing = by_page.get(&page.page_number).cloned();
        }
    }

    by_page
        .iter()
        .min_by_key(|(page_number, _)| *page_number)
        .map(|(_, metadata)| metadata.clone())
}

/// Attach each OCR'd page's confidence summary to its [`crate::types::PageContent`] (#1568).
///
/// `by_page` is keyed by 1-based page number and only holds pages an OCR route actually ran,
/// so a natively extracted page keeps `ocr_confidence: None` -- absence means "not OCR'd",
/// distinct from an entry whose `score` is `None` because the backend has no calibrated
/// legibility scale. Unlike `attach_pdf_preprocessing_metadata` there is no document-level
/// aggregate: averaging per-page confidences across pages OCR'd by different backends (the
/// pipeline route's stages) would compare incomparable scales. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn attach_pdf_ocr_confidence(
    pages: &mut Option<Vec<crate::types::PageContent>>,
    by_page: &ahash::AHashMap<u32, crate::types::page::PageOcrConfidence>,
) {
    if let Some(pages) = pages {
        for page in pages {
            page.ocr_confidence = by_page.get(&page.page_number).cloned();
        }
    }
}

/// Replace a page's `content` with OCR text and recompute `is_blank` from it.
///
/// With `keep_native_content`, the text layer that `content` held moves to `native_content`
/// when it is not blank and differs from the OCR text. The same rule applies on every OCR route.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn replace_page_content_with_ocr_text(page: &mut crate::types::PageContent, ocr_text: &str, keep_native_content: bool) {
    let ocr_content = crate::pdf::text::fix_pdf_control_chars(ocr_text).into_owned();
    let native = std::mem::replace(&mut page.content, ocr_content);
    let keep = keep_native_content && !native.trim().is_empty() && native != page.content;
    page.native_content = keep.then_some(native);
    page.is_blank = Some(crate::extraction::blank_detection::is_page_text_blank(&page.content));
}

/// Write the OCR text of each page into the page that has the same position.
///
/// One OCR text for a document of several pages is the text of the whole document: it goes to
/// the first page, and the content of every later page is replaced with empty text.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn replace_pages_with_ocr_page_texts(
    pages: &mut [crate::types::PageContent],
    ocr_page_texts: &[String],
    keep_native_content: bool,
) {
    for (page, text) in pages.iter_mut().zip(ocr_page_texts) {
        replace_page_content_with_ocr_text(page, text, keep_native_content);
    }

    if ocr_page_texts.len() == 1 {
        for page in pages.iter_mut().skip(1) {
            replace_page_content_with_ocr_text(page, "", keep_native_content);
        }
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn automatic_unmapped_text_hints(metadata: &crate::pdf::metadata::PdfExtractionMetadata) -> Option<ocr::PageOcrHints> {
    let pages = metadata.pdf_specific.fabricated_text_pages.as_ref()?;
    (!pages.is_empty()).then(|| ocr::PageOcrHints {
        source_dpi: None,
        known_full_page_scan: false,
        single_block_pages: Some(std::sync::Arc::new(pages.iter().copied().collect())),
    })
}

/// PDF document extractor using xberg_native_pdf.
#[cfg_attr(alef, alef(skip))]
pub struct PdfExtractor;

impl Default for PdfExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfExtractor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Plugin for PdfExtractor {
    fn name(&self) -> &str {
        "pdf-extractor"
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
impl InternalDocumentExtractor for PdfExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        self.extract_core(content, mime_type, config, None).await
    }

    #[cfg(feature = "tokio-runtime")]
    async fn extract_path(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        #[cfg(feature = "pdf")]
        crate::pdf::native_text::set_current_pdf_path(Some(path.to_path_buf()));
        // Async on native (non-blocking tokio::fs); sync fallback on wasm32 where tokio's `fs`
        // feature is unavailable. See `core::io::read_file_async`. ~keep
        let bytes = crate::core::io::read_file_async(path).await?;
        let result = self.extract_core(&bytes, mime_type, config, Some(path)).await;
        #[cfg(feature = "pdf")]
        crate::pdf::native_text::set_current_pdf_path(None);
        result
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/pdf"]
    }
}

impl PdfExtractor {
    /// Core extraction logic shared between extract_bytes and extract_file.
    ///
    /// Accepts an optional `path` which is passed to OCR backends to allow
    /// direct document-level processing (bypassing page rendering).
    ///
    /// This is the dispatch seam for `PdfConfig::backend` (#702): the single place
    /// that routes a PDF extraction request to a backend implementation. Adding a
    /// real `PdfBackend::Pdfium` engine means adding a branch here, not touching
    /// `extract_content`/`extract_path` or any caller. The dispatch is enforced here
    /// -- in the core crate, below every binding -- rather than only in
    /// `xberg-cli`'s `ExtractionOverrides::validate`, because any caller that builds
    /// `ExtractionConfig` directly (library use, the API/MCP servers, or a language
    /// binding) sets `PdfConfig::backend` without ever going through CLI validation.
    /// Without an enforcement point here, such a caller selecting
    /// `PdfBackend::Pdfium` would have silently gotten `xberg_native_pdf` output mislabeled
    /// as pdfium.
    async fn extract_core(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
        path: Option<&std::path::Path>,
    ) -> Result<InternalDocument> {
        tracing::debug!(format = "pdf", size_bytes = content.len(), "extraction starting");
        #[cfg(feature = "pdf")]
        {
            if let Some(options) = config.pdf_options.as_ref() {
                options.validate()?;
            }
            let backend = config
                .pdf_options
                .as_ref()
                .map(|options| options.backend)
                .unwrap_or_default();
            match backend {
                crate::core::config::PdfBackend::Native => {
                    self.extract_core_native(content, mime_type, config, path).await
                }
                crate::core::config::PdfBackend::Pdfium => {
                    self.extract_core_pdfium(content, mime_type, config, path).await
                }
            }
        }
        #[cfg(not(feature = "pdf"))]
        self.extract_core_native(content, mime_type, config, path).await
    }

    /// Dispatch target for `PdfBackend::Pdfium`.
    ///
    /// Without the `pdf-pdfium` Cargo feature, `crates/xberg-pdfium-render` is not
    /// even compiled in, so this fails loudly rather than falling back to
    /// `extract_core_native`: silently reusing `xberg_native_pdf` output under a caller's
    /// `PdfBackend::Pdfium` selection would produce a document the caller believes
    /// came from pdfium, which is worse than an error.
    #[cfg(all(feature = "pdf", not(feature = "pdf-pdfium")))]
    async fn extract_core_pdfium(
        &self,
        _content: &[u8],
        _mime_type: &str,
        _config: &ExtractionConfig,
        _path: Option<&std::path::Path>,
    ) -> Result<InternalDocument> {
        Err(crate::XbergError::validation(
            "PDF extraction requested backend 'pdfium', but this build was compiled \
             without the 'pdf-pdfium' Cargo feature, so no pdfium extraction engine is \
             available. Use the default 'native' backend instead, or rebuild with \
             --features pdf-pdfium to enable pdfium extraction.",
        ))
    }

    /// Dispatch target for `PdfBackend::Pdfium` when the `pdf-pdfium` feature is
    /// enabled (#702).
    ///
    /// Delegates to [`pdfium_engine::extract`], a deliberately smaller engine than
    /// `extract_core_native`: see that module's doc comment for exactly what it
    /// extracts and what it does not (no tables, images, annotations, form fields,
    /// embedded files, or OCR fallback). Binding to the pdfium shared library is a
    /// runtime concern -- see `pdfium_engine::bind_once` -- so this can still fail
    /// with an actionable `XbergError::MissingDependency` even though the feature
    /// compiled cleanly.
    #[cfg(all(feature = "pdf", feature = "pdf-pdfium"))]
    async fn extract_core_pdfium(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
        _path: Option<&std::path::Path>,
    ) -> Result<InternalDocument> {
        enforce_page_limit(content, config)?;
        pdfium_engine::extract(content, mime_type, config).await
    }

    /// Core extraction via the xberg_native_pdf backend.
    ///
    /// Runs text + metadata, tables, and annotation extraction through the native
    /// modules, then builds an `InternalDocument` using the same post-processing
    /// pipeline (OCR evaluation, page assembly, image extraction, bookmarks, etc.).
    #[cfg(feature = "pdf")]
    async fn extract_core_native(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
        path: Option<&std::path::Path>,
    ) -> Result<InternalDocument> {
        let _ = &path;

        enforce_page_limit(content, config)?;

        #[cfg(all(feature = "pdf", feature = "layout-detection"))]
        #[allow(unused_mut, unused_variables)]
        let (
            mut markdown_layout_images,
            markdown_layout_results,
            markdown_layout_hints,
            mut markdown_layout_detections,
            markdown_layout_gate_decisions,
            markdown_layout_warning,
            mut markdown_layout_acceleration_override,
            markdown_layout_glyph_drop_warnings,
        ) = layout_runner::maybe_run_layout_for_markdown(content, config).await;

        let blocking_content = content.to_vec();
        let blocking_config = config.clone();
        let native_output = spawn_pdf_blocking(move || {
            extract_native_blocking(
                blocking_content,
                blocking_config,
                #[cfg(feature = "layout-detection")]
                markdown_layout_images,
                #[cfg(feature = "layout-detection")]
                markdown_layout_results,
                #[cfg(feature = "layout-detection")]
                markdown_layout_hints,
                #[cfg(feature = "layout-detection")]
                markdown_layout_acceleration_override,
            )
        })
        .await?;
        let bookmark_uris = native_output.bookmark_uris;
        let pdf_revisions = native_output.pdf_revisions;
        let diagrams = native_output.diagrams;
        #[cfg(feature = "layout-detection")]
        let mut markdown_layout_images = native_output.layout_images;
        #[cfg(all(feature = "layout-detection", feature = "liter-llm"))]
        let markdown_layout_hints = native_output.layout_hints;
        #[cfg(all(feature = "layout-detection", not(feature = "liter-llm")))]
        let _markdown_layout_hints = native_output.layout_hints;
        #[cfg(feature = "layout-detection")]
        let mut markdown_layout_acceleration_override = native_output.layout_acceleration_override;

        #[allow(unused_variables, unused_mut)]
        let (
            mut pdf_metadata,
            native_text,
            mut tables,
            mut page_contents,
            mut boundaries,
            pre_rendered_doc,
            _has_font_encoding_issues,
            annotation_text_fallback,
            pdf_annotations,
            mut extracted_images,
            pdf_form_fields,
            mut pdf_extraction_warnings,
            pdf_page_labels,
            pdf_page_coordinate_frames,
        ) = native_output.extraction;

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if config.pdf_options.as_ref().is_some_and(|p| p.ocr_inline_images)
            && let Some(ref mut imgs) = extracted_images
            && !imgs.is_empty()
        {
            let default_ocr_config;
            let ocr_config = match config.ocr.as_ref() {
                Some(c) => c,
                None => {
                    default_ocr_config = crate::core::config::OcrConfig::default();
                    &default_ocr_config
                }
            };
            crate::plugins::ensure_ocr_backends_initialized();
            let backend = {
                let registry = crate::plugins::registry::get_ocr_backend_registry();
                registry.read().get(&ocr_config.backend)?
            };
            let mut ocr_config_with_format = ocr_config.clone();
            ocr_config_with_format.output_format = Some(config.output_format.clone());
            // GH#1651: this embedded-image route never carried the caller's decode limits,
            // so every decode ran under `SecurityLimits::default()`. Conditional so a limit
            // set directly on `OcrConfig` is not replaced by `None`. ~keep
            if let Some(limits) = config.security_limits.clone() {
                ocr_config_with_format.security_limits = Some(limits);
            }
            for img in imgs.iter_mut() {
                if config.cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                    break;
                }
                match backend.process_image(&img.data, &ocr_config_with_format).await {
                    Ok(mut ocr_result) => {
                        ocr_config.apply_public_element_policy(&mut ocr_result);
                        img.ocr_result = Some(Box::new(ocr_result));
                    }
                    Err(e) => {
                        tracing::warn!(
                            page = img.page_number,
                            image_index = img.image_index,
                            error = %e,
                            "inline image OCR failed; image returned without OCR result"
                        );
                    }
                }
            }
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_tables: Vec<crate::types::Table> = Vec::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_elements: Vec<crate::types::OcrElement> = Vec::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_coordinate_frames: Vec<crate::types::internal::OcrPageCoordinateFrame> = Vec::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_internal_doc: Option<InternalDocument> = None;
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_llm_usage: Vec<crate::types::LlmUsage> = Vec::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_page_texts: Option<Vec<String>> = None;
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_results_map: Option<ahash::AHashMap<u32, String>> = None;
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut structured_ocr_pages: Option<ahash::AHashMap<u32, InternalDocument>> = None;
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_page_rasters: Option<Vec<crate::types::ExtractedImage>> = None;
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_formulas: Vec<crate::types::Formula> = Vec::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_preprocessing_by_page: ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata> =
            ahash::AHashMap::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_confidence_by_page: ahash::AHashMap<u32, crate::types::page::PageOcrConfidence> =
            ahash::AHashMap::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_fallback_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_page_failures: Vec<crate::types::OcrPageFailure> = Vec::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        #[allow(unused_assignments)]
        let mut ocr_layout_gate_audit: OcrLayoutGateDecisions = (None, None);

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        record_implausible_text_pages(
            config,
            &mut pdf_metadata,
            &native_text,
            boundaries.as_deref(),
            &mut ocr_fallback_warnings,
        );

        #[cfg(all(
            feature = "pdf",
            feature = "layout-detection",
            any(feature = "ocr", feature = "ocr-pipeline")
        ))]
        {
            // These values exist only with layout detection; keep the reuse gate in the same cfg block. ~keep
            let markdown_page_rotations = markdown_layout_images
                .as_ref()
                .map(|images| crate::pdf::render::get_page_rotations_from_bytes(content, images.len()))
                .unwrap_or_default();
            if !markdown_layout_reusable_for_ocr(markdown_layout_gate_decisions.as_deref(), &markdown_page_rotations) {
                markdown_layout_images = None;
                markdown_layout_detections = None;
                markdown_layout_acceleration_override = None;
            }
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let (mut text, extraction_method) = if config.effective_disable_ocr() {
            (native_text, ExtractionMethod::Native)
        } else if config.force_ocr {
            let (
                ocr_text,
                ocr_tbls,
                ocr_elems,
                ocr_doc,
                llm_usage,
                ocr_pts,
                ocr_rstrs,
                formulas,
                preprocessing,
                page_ocr_confidence,
                gate_audit,
                layout_warning,
                layout_glyph_drop_warnings,
            ) = run_ocr_with_layout(
                content,
                config,
                path,
                None,
                #[cfg(feature = "layout-detection")]
                markdown_layout_images.take(),
                #[cfg(feature = "layout-detection")]
                markdown_layout_detections.take(),
                #[cfg(feature = "layout-detection")]
                markdown_layout_acceleration_override.take(),
            )
            .await?;
            if let Some(warning) = layout_warning {
                crate::core::diagnostics::push_warning_deduped(&mut ocr_fallback_warnings, warning);
            }
            for warning in layout_glyph_drop_warnings {
                crate::core::diagnostics::push_warning_deduped(&mut ocr_fallback_warnings, warning);
            }
            ocr_layout_gate_audit = gate_audit;
            ocr_tables = ocr_tbls;
            ocr_elements = ocr_elems;
            ocr_internal_doc = ocr_doc;
            ocr_llm_usage = llm_usage;
            ocr_page_texts = Some(ocr_pts);
            ocr_page_rasters = ocr_rstrs;
            ocr_formulas = formulas;
            ocr_preprocessing_by_page = preprocessing;
            ocr_confidence_by_page = page_ocr_confidence;
            (ocr_text, ExtractionMethod::Ocr)
        } else if let Some(ref ocr_pages) = config.force_ocr_pages {
            if !ocr_pages.is_empty() {
                if let Some(ref bounds) = boundaries {
                    if !bounds.is_empty() {
                        let (
                            mixed,
                            results_map,
                            mixed_structured_pages,
                            mixed_llm_usage,
                            mixed_rstrs,
                            mixed_formulas,
                            mixed_preprocessing,
                            mixed_ocr_confidence,
                            mixed_warnings,
                            mixed_page_failures,
                        ) = ocr::extract_mixed_ocr_native_with_single_block_pages(
                            &native_text,
                            bounds,
                            ocr_pages,
                            &[],
                            content,
                            config,
                            path,
                        )
                        .await?;
                        let extraction_method = extraction_method_after_mixed_ocr(&results_map);
                        ocr_llm_usage = mixed_llm_usage;
                        ocr_results_map = Some(results_map);
                        structured_ocr_pages = Some(mixed_structured_pages);
                        ocr_page_rasters = mixed_rstrs;
                        if !mixed_formulas.is_empty() {
                            ocr_formulas = mixed_formulas;
                        }
                        ocr_preprocessing_by_page.extend(mixed_preprocessing);
                        ocr_confidence_by_page.extend(mixed_ocr_confidence);
                        ocr_page_failures.extend(mixed_page_failures);
                        ocr_fallback_warnings.extend(mixed_warnings);
                        (mixed, extraction_method)
                    } else {
                        tracing::warn!("force_ocr_pages set but no page boundaries available; using native text");
                        (native_text, ExtractionMethod::Native)
                    }
                } else {
                    tracing::warn!("force_ocr_pages set but no page boundaries available; using native text");
                    (native_text, ExtractionMethod::Native)
                }
            } else {
                (native_text, ExtractionMethod::Native)
            }
        } else if let Some(scanned_pages) =
            scanned_pages_to_ocr(config, &pdf_metadata, &native_text, boundaries.as_deref())
                .filter(|_| config.ocr.is_some() || automatic_ocr_backend_is_registered())
        {
            // A scanner's invisible sidecar passes the gate below, so detected
            // pages are selected before it runs. ~keep
            if let Some(ref bounds) = boundaries
                && !bounds.is_empty()
            {
                // This OCR run is AUTOMATIC -- scanned-page detection asked for it, the
                // caller did not -- so a failure here must degrade to the native text rather
                // than abort, exactly as the quality-gate fallback below already does. With
                // `ocr-pipeline` enabled and no backend registered (a legitimate feature
                // selection: `ocr` implies `ocr-pipeline`, not the reverse) the `?` that used
                // to be here failed an ordinary PDF extraction that never requested OCR. The
                // EXPLICIT sites (`force_ocr_pages` above, `ocr_inline_images`) keep their
                // hard error, because there the caller asked for something the build cannot
                // do. See GH#1610. ~keep
                match ocr::extract_mixed_ocr_native_with_single_block_pages(
                    &native_text,
                    bounds,
                    &scanned_pages,
                    pdf_metadata
                        .pdf_specific
                        .fabricated_text_pages
                        .as_deref()
                        .unwrap_or(&[]),
                    content,
                    config,
                    path,
                )
                .await
                {
                    Ok((
                        mixed,
                        results_map,
                        mixed_structured_pages,
                        mixed_llm_usage,
                        mixed_rstrs,
                        mixed_formulas,
                        mixed_preprocessing,
                        mixed_ocr_confidence,
                        mixed_warnings,
                        mixed_page_failures,
                    )) => {
                        // `Mixed` must mean "OCR contributed text", not "OCR was attempted". When
                        // every candidate page was rejected (blank render, failed decode, empty
                        // backend output) nothing was replaced and the result IS the native text --
                        // reporting `Mixed` there tells a caller the document was OCR'd when it was
                        // not, which is how a silent whole-document OCR failure reads as success. ~keep
                        let mixed_method = extraction_method_after_mixed_ocr(&results_map);
                        let ocr_contributed = mixed_method == ExtractionMethod::Mixed;
                        if !ocr_contributed {
                            tracing::warn!(
                                candidate_pages = scanned_pages.len(),
                                "OCR was attempted on every detected scanned page but no page produced usable \
                                 text; reporting the native extraction method rather than `mixed`"
                            );
                        }
                        ocr_llm_usage = mixed_llm_usage;
                        ocr_results_map = Some(results_map);
                        structured_ocr_pages = Some(mixed_structured_pages);
                        ocr_page_rasters = mixed_rstrs;
                        if !mixed_formulas.is_empty() {
                            ocr_formulas = mixed_formulas;
                        }
                        ocr_preprocessing_by_page.extend(mixed_preprocessing);
                        ocr_confidence_by_page.extend(mixed_ocr_confidence);
                        ocr_page_failures.extend(mixed_page_failures);
                        ocr_fallback_warnings.extend(mixed_warnings);
                        if ocr_contributed {
                            (mixed, mixed_method)
                        } else {
                            (mixed, ExtractionMethod::Native)
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            candidate_pages = ?scanned_pages,
                            "Automatic OCR of detected scanned pages failed; using native text extraction result"
                        );
                        if failed_ocr_fallback_is_total_loss(&native_text) {
                            return Err(e);
                        }
                        ocr_page_failures.extend(ocr::route_level_ocr_page_failures(
                            &scanned_pages,
                            &e,
                            &native_text,
                            bounds,
                        ));
                        ocr_fallback_warnings.push(crate::types::ProcessingWarning {
                            source: std::borrow::Cow::Borrowed("ocr"),
                            message: std::borrow::Cow::Owned(format!(
                                "Automatic OCR of detected scanned pages {scanned_pages:?} failed ({e}); those \
                                 pages retain their native text, which may be empty or incomplete."
                            )),
                        });
                        (native_text, ExtractionMethod::Native)
                    }
                }
            } else {
                tracing::warn!("scanned pages detected but no page boundaries available; using native text");
                (native_text, ExtractionMethod::Native)
            }
        } else if near_empty_ocr_fallback_applies(config, &native_text) {
            // Under `Auto`, a PDF with NO native text at all (a scan / missing text layer)
            // must reach OCR even when no explicit `ocr` config was given — the default
            // `ocr: None` ("OCR disabled") otherwise silently discarded the detected scan
            // and returned an empty result (#1338). This is gated on genuinely-absent text
            // a legitimately sparse/short PDF must stay native under a default config, and
            // heuristic quality failures still require an explicit `ocr` config to trigger
            // OCR. An explicit `ocr` config keeps its full per-page gate behavior.
            // `ocr_near_empty_fallback` overrides that derivation in either direction
            // (GH#1752); see `near_empty_ocr_fallback_applies`. ~keep
            let default_ocr_config = crate::core::config::OcrConfig::default();
            let ocr_config = config.ocr.as_ref().unwrap_or(&default_ocr_config);
            let thresholds = ocr_config.effective_thresholds();
            let mut decision = ocr::evaluate_per_page_ocr(
                &native_text,
                boundaries.as_deref(),
                pdf_metadata.pdf_specific.page_count,
                &thresholds,
            );

            // `NativeTextStats`' character-class checks cannot see a font whose mapping
            // legitimately (per the §9.10.2 cascade) resolves to the wrong-but-ordinary
            // letters and punctuation (issue #1667): the text is structurally clean. The
            // provenance signal computed above (issue #1254) is a fact about how the text
            // was derived, not a shape guess, and `ScannedPages` already routes it via
            // `scanned_pages_to_ocr`; `Auto` must consult it too rather than silently
            // discarding it. ~keep
            let has_boundaries = boundaries.as_deref().is_some_and(|b| !b.is_empty());
            ocr::apply_flagged_pages(
                &mut decision,
                pdf_metadata
                    .pdf_specific
                    .fabricated_text_pages
                    .as_deref()
                    .unwrap_or(&[]),
                has_boundaries,
                pdf_metadata.pdf_specific.page_count,
            );

            // A `/ToUnicode` CMap that resolves every glyph to *a* character, but
            // consistently the WRONG one, is file-backed exactly like a correct mapping --
            // `fabricated_text_pages`' provenance check cannot see it either (issue #1696).
            // `record_implausible_text_pages` already computed and merged this into
            // `scanned_pages` above; `Auto` reads it here the same way it reads the
            // provenance signal, so both share one union step. ~keep
            ocr::apply_flagged_pages(
                &mut decision,
                pdf_metadata
                    .pdf_specific
                    .implausible_text_pages
                    .as_deref()
                    .unwrap_or(&[]),
                has_boundaries,
                pdf_metadata.pdf_specific.page_count,
            );

            tracing::debug!(
                target: "xberg::pdf::ocr",
                fallback = decision.fallback,
                non_whitespace = decision.stats.non_whitespace,
                alnum = decision.stats.alnum,
                meaningful_words = decision.stats.meaningful_words,
                avg_non_whitespace = decision.avg_non_whitespace,
                avg_alnum = decision.avg_alnum,
                alnum_ratio = decision.stats.alnum_ratio,
                fragmented_word_ratio = decision.stats.fragmented_word_ratio,
                avg_word_length = decision.stats.avg_word_length,
                word_count = decision.stats.word_count,
                consecutive_repeat_ratio = decision.stats.consecutive_repeat_ratio,
                "per-page OCR gate decision",
            );

            let total_chars = native_text.chars().count();
            let alnum_ws_chars = native_text
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                .count();
            let alnum_ws_ratio = if total_chars > 0 {
                alnum_ws_chars as f64 / total_chars as f64
            } else {
                1.0
            };

            match ocr::evaluate_ocr_skip_gate(
                pre_rendered_doc.is_some(),
                total_chars,
                alnum_ws_ratio,
                &decision,
                &thresholds,
            ) {
                ocr::OcrGateOutcome::SkipNonText => {
                    tracing::debug!(
                        alnum_ws_ratio,
                        total_chars,
                        alnum_ws_chars,
                        "Skipping OCR: content is non-textual and pre-rendered structured doc available"
                    );
                    (native_text, ExtractionMethod::Native)
                }
                ocr::OcrGateOutcome::SkipSubstantive => {
                    tracing::debug!(
                        total_chars,
                        alnum_ws_ratio,
                        ocr_fallback = decision.fallback,
                        "Skipping OCR: pre-rendered structured doc available with substantive native text"
                    );
                    (native_text, ExtractionMethod::Native)
                }
                ocr::OcrGateOutcome::RunFallback => {
                    // `images.run_ocr_on_images` controls whether extracted images are
                    // separately OCR'd (`core/pipeline/mod.rs`); it does not gate this
                    // document-level page OCR fallback. Reading it here made any
                    // `ImageExtractionConfig` (whose `run_ocr_on_images` defaults to `true`)
                    // silently suppress page OCR on scanned PDFs, leaving `pages[].content`
                    // empty (#1576). `RunFallbackOnPages` below has never read this field;
                    // this now matches it. ~keep
                    match run_ocr_with_layout(
                        content,
                        config,
                        path,
                        automatic_unmapped_text_hints(&pdf_metadata),
                        #[cfg(feature = "layout-detection")]
                        markdown_layout_images.take(),
                        #[cfg(feature = "layout-detection")]
                        markdown_layout_detections.take(),
                        #[cfg(feature = "layout-detection")]
                        markdown_layout_acceleration_override.take(),
                    )
                    .await
                    {
                        Ok((
                            ocr_text,
                            ocr_tbls,
                            ocr_elems,
                            ocr_doc,
                            llm_usage,
                            ocr_pts,
                            ocr_rstrs,
                            formulas,
                            preprocessing,
                            page_ocr_confidence,
                            gate_audit,
                            layout_warning,
                            layout_glyph_drop_warnings,
                        )) => {
                            if let Some(warning) = layout_warning {
                                crate::core::diagnostics::push_warning_deduped(&mut ocr_fallback_warnings, warning);
                            }
                            for warning in layout_glyph_drop_warnings {
                                crate::core::diagnostics::push_warning_deduped(&mut ocr_fallback_warnings, warning);
                            }
                            ocr_layout_gate_audit = gate_audit;
                            ocr_tables = ocr_tbls;
                            ocr_elements = ocr_elems;
                            ocr_internal_doc = ocr_doc;
                            ocr_llm_usage = llm_usage;
                            ocr_page_texts = Some(ocr_pts);
                            ocr_page_rasters = ocr_rstrs;
                            ocr_formulas = formulas;
                            ocr_preprocessing_by_page = preprocessing;
                            ocr_confidence_by_page = page_ocr_confidence;
                            (ocr_text, ExtractionMethod::Ocr)
                        }
                        Err(failed_run) => {
                            tracing::warn!(
                                error = %failed_run.error,
                                "OCR fallback failed; using native text extraction result"
                            );
                            if failed_ocr_fallback_is_total_loss(&native_text) {
                                return Err(failed_run.error);
                            }
                            // ~keep A cancelled run did not fail to read its pages, so it adds no
                            // record. The token decides: a pipeline run reports a cancellation as
                            // the failure of each of its stages, not as the cancellation error.
                            if ensure_pdf_not_cancelled(config).is_ok() {
                                ocr_page_failures.extend(ocr::failed_run_page_failures(
                                    &failed_run,
                                    pdf_metadata.pdf_specific.page_count.unwrap_or(0),
                                    &native_text,
                                    boundaries.as_deref().unwrap_or(&[]),
                                ));
                            }
                            let e = &failed_run.error;
                            ocr_fallback_warnings.push(crate::types::ProcessingWarning {
                                source: std::borrow::Cow::Borrowed("ocr"),
                                message: std::borrow::Cow::Owned(format!(
                                    "OCR fallback failed ({e}); returning native text that was below the \
                                         quality threshold which triggered OCR. Extracted content may be empty \
                                         or incomplete."
                                )),
                            });
                            (native_text, ExtractionMethod::Native)
                        }
                    }
                }
                ocr::OcrGateOutcome::RunFallbackOnPages(pages) => match boundaries.as_deref() {
                    Some(bounds) if !bounds.is_empty() => {
                        match ocr::extract_mixed_ocr_native_with_single_block_pages(
                            &native_text,
                            bounds,
                            &pages,
                            pdf_metadata
                                .pdf_specific
                                .fabricated_text_pages
                                .as_deref()
                                .unwrap_or(&[]),
                            content,
                            config,
                            path,
                        )
                        .await
                        {
                            Ok((
                                mixed,
                                results_map,
                                mixed_structured_pages,
                                mixed_llm_usage,
                                mixed_rstrs,
                                mixed_formulas,
                                mixed_preprocessing,
                                mixed_ocr_confidence,
                                mixed_warnings,
                                mixed_page_failures,
                            )) => {
                                let extraction_method = extraction_method_after_mixed_ocr(&results_map);
                                ocr_llm_usage = mixed_llm_usage;
                                ocr_results_map = Some(results_map);
                                structured_ocr_pages = Some(mixed_structured_pages);
                                ocr_page_rasters = mixed_rstrs;
                                if !mixed_formulas.is_empty() {
                                    ocr_formulas = mixed_formulas;
                                }
                                ocr_preprocessing_by_page.extend(mixed_preprocessing);
                                ocr_confidence_by_page.extend(mixed_ocr_confidence);
                                ocr_page_failures.extend(mixed_page_failures);
                                ocr_fallback_warnings.extend(mixed_warnings);
                                (mixed, extraction_method)
                            }
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    failing_pages = ?pages,
                                    "Targeted OCR fallback failed; using native text extraction result"
                                );
                                if failed_ocr_fallback_is_total_loss(&native_text) {
                                    return Err(e);
                                }
                                ocr_page_failures.extend(ocr::route_level_ocr_page_failures(
                                    &pages,
                                    &e,
                                    &native_text,
                                    bounds,
                                ));
                                ocr_fallback_warnings.push(crate::types::ProcessingWarning {
                                    source: std::borrow::Cow::Borrowed("ocr"),
                                    message: std::borrow::Cow::Owned(format!(
                                        "Targeted OCR fallback failed ({e}) for pages {pages:?}; those pages \
                                         retain native text that was below the OCR-trigger quality threshold \
                                         and may be empty or incomplete."
                                    )),
                                });
                                (native_text, ExtractionMethod::Native)
                            }
                        }
                    }
                    _ => {
                        tracing::warn!(
                            failing_pages = ?pages,
                            "Targeted OCR requested but no page boundaries available; using native text"
                        );
                        if failed_ocr_fallback_is_total_loss(&native_text) {
                            return Err(crate::XbergError::Plugin {
                                message: format!(
                                    "Targeted OCR was required for pages {pages:?} but no page boundaries \
                                     were available, and no native text could be recovered"
                                ),
                                plugin_name: "ocr".to_string(),
                            });
                        }
                        ocr_fallback_warnings.push(crate::types::ProcessingWarning {
                            source: std::borrow::Cow::Borrowed("ocr"),
                            message: std::borrow::Cow::Owned(format!(
                                "Targeted OCR was required for pages {pages:?} but no page boundaries were \
                                 available; those pages retain native text that was below the OCR-trigger \
                                 quality threshold and may be empty or incomplete."
                            )),
                        });
                        (native_text, ExtractionMethod::Native)
                    }
                },
                ocr::OcrGateOutcome::UseNative => (native_text, ExtractionMethod::Native),
            }
        } else {
            (native_text, ExtractionMethod::Native)
        };

        #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
        let (mut text, extraction_method) = (native_text, ExtractionMethod::Native);

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        // Full-document OCR is authoritative for tables when it produced
        // them. The structured OCR document already contains the same table
        // values, so later document assembly must not inject them a second
        // time. ~keep
        replace_tables_with_ocr_output(&mut tables, ocr_tables);

        let (images, image_fallback_warning): (
            Option<Vec<crate::types::ExtractedImage>>,
            Option<crate::types::ProcessingWarning>,
        ) = (extracted_images, None);

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        {
            let keep_native_content = config.pages.as_ref().is_some_and(|pages| pages.keep_native_content);

            if let Some(pts) = ocr_page_texts.as_ref() {
                if let Some(ref mut pages) = page_contents {
                    replace_pages_with_ocr_page_texts(pages, pts, keep_native_content);
                } else {
                    page_contents = Some(
                        pts.iter()
                            .enumerate()
                            .map(|(i, text)| {
                                let content = crate::pdf::text::fix_pdf_control_chars(text).into_owned();
                                let is_blank = Some(crate::extraction::blank_detection::is_page_text_blank(&content));
                                crate::types::PageContent {
                                    page_number: (i + 1) as u32,
                                    content,
                                    tables: Vec::new(),
                                    image_indices: vec![],
                                    image_preprocessing: None,
                                    hierarchy: None,
                                    is_blank,
                                    layout_regions: None,
                                    speaker_notes: None,
                                    section_name: None,
                                    sheet_name: None,
                                    ocr_confidence: None,
                                    native_content: None,
                                }
                            })
                            .collect(),
                    );
                }
            }

            if let Some(results_map) = ocr_results_map.as_ref()
                && let Some(ref mut pages) = page_contents
            {
                for page in pages.iter_mut() {
                    if let Some(ocr_text) = results_map.get(&page.page_number) {
                        replace_page_content_with_ocr_text(page, ocr_text, keep_native_content);
                    }
                }
            }

            if let Some(ref content_pages) = page_contents
                && let Some(ref mut page_structure) = pdf_metadata.page_structure
                && let Some(ref mut info_pages) = page_structure.pages
            {
                for info in info_pages.iter_mut() {
                    if let Some(content_page) = content_pages.iter().find(|p| p.page_number == info.number) {
                        info.is_blank = content_page.is_blank;
                    }
                }
            }
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let image_preprocessing = attach_pdf_preprocessing_metadata(&mut page_contents, &ocr_preprocessing_by_page);
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        attach_pdf_ocr_confidence(&mut page_contents, &ocr_confidence_by_page);
        #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
        let image_preprocessing = None;

        #[cfg_attr(not(any(feature = "ocr", feature = "ocr-pipeline")), allow(unused_variables))]
        let annotation_fallback_applied = annotation_text_fallback.is_some();
        if let Some(annotation_text_fallback) = annotation_text_fallback.as_ref() {
            extraction::apply_annotation_text_fallback(
                annotation_text_fallback,
                #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
                ocr_page_texts.as_deref(),
                #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
                None,
                #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
                ocr_results_map.as_ref(),
                #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
                None,
                #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
                (extraction_method == ExtractionMethod::Ocr),
                #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
                false,
                extraction::AnnotationFallbackTarget {
                    text: &mut text,
                    page_contents: &mut page_contents,
                    boundaries: &mut boundaries,
                    pdf_metadata: &mut pdf_metadata,
                    page_config: config.pages.as_ref(),
                },
            );
        }

        let mut final_pages =
            assign_tables_and_images_to_pages(page_contents, &tables, images.as_deref().unwrap_or(&[]));

        let pre_formatted_output: Option<String> = None;

        let used_ocr = extraction_method.used_ocr();

        // `boundaries` index the NATIVE text; on the mixed path the OCR replacements have
        // already shifted every later offset, so re-map before handing them to the document
        // selector or page tagging would attribute content to the wrong page. ~keep
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let selector_boundaries = if annotation_fallback_applied {
            boundaries.clone()
        } else {
            boundaries_for_ocr_output(
                extraction_method,
                &text,
                boundaries.as_deref(),
                ocr_results_map.as_ref(),
                ocr_page_texts.as_deref(),
            )
        };
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if let Some(ref mut page_structure) = pdf_metadata.page_structure {
            page_structure.boundaries = selector_boundaries.clone();
        }
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let (mut doc, document_origin, document_is_structured) = select_pdf_document(
            extraction_method,
            &text,
            mime_type,
            pre_rendered_doc,
            ocr_internal_doc.take(),
            ocr_results_map.as_ref(),
            structured_ocr_pages.as_ref(),
            selector_boundaries.as_deref(),
            &config.output_format,
        );
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if extraction_method == ExtractionMethod::Mixed
            && let Some(ref accepted_pages) = structured_ocr_pages
        {
            ocr_elements = accepted_mixed_ocr_elements(accepted_pages);
            merge_mixed_ocr_tables(&mut tables, accepted_pages);
            ocr_coordinate_frames = accepted_mixed_ocr_coordinate_frames(accepted_pages);
        }
        #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
        let (mut doc, document_is_structured) =
            select_native_pdf_document(&text, mime_type, pre_rendered_doc, boundaries.as_deref());
        // #1575: `doc.metadata` is fully replaced by a fresh `Metadata { .. }` literal below
        // (native/mixed extraction never populates `doc.metadata` this early, so this is a
        // no-op for them), which would otherwise discard the OCR pipeline's `psm`/`language`/
        // `tesseract_dict_invalid_word_ratio` keys set on `ocr_internal_doc.metadata.additional`
        // (`extract_with_ocr_for_page`) before they ever reached a caller. Taken here and
        // merged back in after that literal is built. ~keep
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let ocr_backend_additional_metadata = std::mem::take(&mut doc.metadata.additional);
        if let Some(annotation_text_fallback) = annotation_text_fallback.as_ref() {
            extraction::append_annotation_fallback_elements(annotation_text_fallback, &mut doc);
        }
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        tracing::debug!(?document_origin, document_is_structured, "selected PDF document origin");

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        doc.processing_warnings.append(&mut ocr_fallback_warnings);
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        doc.ocr_page_failures.append(&mut ocr_page_failures);

        doc.processing_warnings.append(&mut pdf_extraction_warnings);

        // #340: drain any xberg_native_pdf glyph-drop warnings captured while this
        // document's pages were rendered on *this* thread (OCR rasterization
        // and any other call routed through `render_page_capturing_glyph_drops`
        // in `crate::pdf::render` that runs inline on the extracting task's
        // thread). The drain is unconditionally cheap and correct even when
        // nothing was ever captured: `install_pdf_render_diagnostics` is
        // opt-in and only an embedding application calls it (a library must
        // not seize the process-global `log` backend on its own — see
        // `pdf::render` for why), so this buffer stays empty for every caller
        // that has not opted in and this loop is a no-op `Vec::is_empty`
        // check. Deduped because a multi-page document can hit the identical
        // xberg_native_pdf cause on many pages.
        //
        // Layout-detection rasterization runs inside `tokio::task::spawn_blocking`
        // on a different OS thread, so this drain never sees those warnings —
        // `layout_runner::run_layout_for_pdf_pages_async` drains them itself
        // from inside that closure and threads them back through its return
        // value instead (#353); they are merged below via
        // `markdown_layout_glyph_drop_warnings` (markdown path) and were
        // already folded into `ocr_fallback_warnings` above (OCR path). ~keep
        for pdf_render_warning in crate::pdf::render::take_xberg_native_pdf_render_warnings() {
            crate::core::diagnostics::push_warning_deduped(&mut doc.processing_warnings, pdf_render_warning);
        }

        // Surface a hard layout-inference failure (e.g. CoreML kernel error) so
        // degraded no-layout output is never silent to the caller (#1344). Runs
        // whenever layout-detection is on, independent of OCR.
        #[cfg(all(feature = "pdf", feature = "layout-detection"))]
        if let Some(warning) = markdown_layout_warning {
            crate::core::diagnostics::push_warning_deduped(&mut doc.processing_warnings, warning);
        }

        // #353: merge `xberg_native_pdf` glyph-drop warnings captured while the
        // markdown layout pass rendered its pages off-thread inside
        // `spawn_blocking` (see the drain comment above).
        #[cfg(all(feature = "pdf", feature = "layout-detection"))]
        for warning in markdown_layout_glyph_drop_warnings {
            crate::core::diagnostics::push_warning_deduped(&mut doc.processing_warnings, warning);
        }

        // Record the auto layout gate's audit trail from whichever pass ran
        // it. Compiled whenever `pdf` is on so `ocr_layout_gate_audit` keeps
        // a reader in profiles without layout-detection (it is always
        // `(None, None)` there and the fields stay `None`). ~keep
        {
            #[cfg(feature = "layout-detection")]
            #[allow(unused_mut)]
            let (mut gated_pages, mut gate_reasons) = layout_gate_metadata(markdown_layout_gate_decisions.as_deref());
            #[cfg(not(feature = "layout-detection"))]
            #[allow(unused_mut)]
            let (mut gated_pages, mut gate_reasons): OcrLayoutGateDecisions = (None, None);
            #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
            if gated_pages.is_none() {
                let (ocr_gated_pages, ocr_gate_reasons) = ocr_layout_gate_audit;
                gated_pages = ocr_gated_pages;
                gate_reasons = ocr_gate_reasons;
            }
            pdf_metadata.pdf_specific.layout_gated_pages = gated_pages;
            pdf_metadata.pdf_specific.layout_gate_reasons = gate_reasons;
        }

        let extract_pdf_metadata = config
            .pdf_options
            .as_ref()
            .is_none_or(|options| options.extract_metadata);
        doc.metadata = Metadata {
            output_format: pre_formatted_output,
            image_preprocessing,
            ocr_used: used_ocr,
            ..Default::default()
        };
        apply_extracted_pdf_metadata(&mut doc.metadata, pdf_metadata, extract_pdf_metadata);
        doc.metadata.additional.insert(
            std::borrow::Cow::Borrowed("extraction_method"),
            serde_json::Value::String(extraction_method.as_str().to_string()),
        );
        // #1575: restore the OCR-pipeline keys taken above, now that the fresh `Metadata`
        // literal they would otherwise have been lost to already exists.
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        doc.metadata.additional.extend(ocr_backend_additional_metadata);

        // GH#1645: one authoritative processed-raster coordinate frame per OCR page with
        // public elements, so a multi-page consumer can normalize `OcrElement` geometry
        // without relying on the single document-wide `ocr_processed_image_width/height`
        // pair, which cannot describe differently sized, preprocessed, or rotated pages.
        // Rides in `additional` rather than a new public binding type, same as the
        // `page_labels` key below (issue #66).
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if !ocr_coordinate_frames.is_empty() {
            doc.metadata.additional.insert(
                std::borrow::Cow::Borrowed(crate::ocr_metadata_keys::OCR_PAGE_COORDINATE_FRAMES_METADATA_KEY),
                serde_json::json!(ocr_coordinate_frames),
            );
        }

        // Issue #66: `/PageLabels` — one display label per page, index-aligned
        // with `pdf_metadata.page_structure`/`PageBoundary::page_number`.
        // `PdfMetadata` is an alef-listed type (no new public fields), so this
        // rides in `additional` instead. ~keep
        if extract_pdf_metadata && let Some(labels) = pdf_page_labels {
            doc.metadata
                .additional
                .insert(std::borrow::Cow::Borrowed("page_labels"), serde_json::json!(labels));
        }

        // Issue #64: surface filled field values in rendered content for
        // output shapes (Markdown/HTML/Djot) that the plain-text widget
        // splice in `native::text` never touches.
        inject_unrepresented_form_field_elements(&mut doc, &pdf_form_fields);
        doc.form_fields = pdf_form_fields;

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let allow_table_injection = (!document_is_structured
            && (config.output_format != crate::core::config::OutputFormat::Plain
                || document_origin == PdfDocumentOrigin::Ocr))
            || (document_origin == PdfDocumentOrigin::Ocr && doc.tables.is_empty());
        #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
        let allow_table_injection =
            document_is_structured || config.output_format != crate::core::config::OutputFormat::Plain;
        attach_unrepresented_tables(&mut doc, tables);
        inject_unrepresented_table_elements(&mut doc, allow_table_injection);

        if let Some(imgs) = images {
            // The OCR path has its own guarded injection block below (see the `#[cfg(feature = "ocr")]`
            let image_anchor = extraction::image_anchor(config);
            let document_has_image_elements = doc.elements.iter().any(|element| {
                #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
                if element.image_ocr_text_anchor_index().is_some() {
                    return true;
                }
                matches!(element.kind, ElementKind::Image { .. })
            });
            if !document_has_image_elements && image_anchor != extraction::ImageAnchor::None {
                for (idx, img) in imgs.iter().enumerate() {
                    // Page OCR already read the picture as part of the page, so the picture's
                    // own OCR text is not rendered there: no anchor, and a placeholder without
                    // the text. The OCR merge of a structured document does the same for the
                    // pages it replaces. ~keep
                    #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
                    let page_text_is_from_ocr = extraction_method == ExtractionMethod::Ocr
                        || img.page_number.is_some_and(|page| {
                            ocr_results_map
                                .as_ref()
                                .is_some_and(|results| results.contains_key(&page))
                        });
                    #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
                    if image_anchor == extraction::ImageAnchor::OcrTextOnly {
                        if !page_text_is_from_ocr {
                            doc.push_element(InternalElement::image_ocr_text_anchor(idx as u32, img.page_number));
                        }
                        continue;
                    }
                    let mut elem = InternalElement::text(
                        ElementKind::Image {
                            image_index: idx as u32,
                        },
                        "",
                        0,
                    );
                    elem.page = img.page_number;
                    #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
                    if page_text_is_from_ocr {
                        elem.suppress_image_ocr_rendering();
                    }
                    doc.push_element(elem);
                }
            }
            doc.images = imgs;
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let ocr_rasters_bypass = ocr_page_rasters.is_none();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if let Some(rasters) = ocr_page_rasters {
            let base_idx = doc.images.len() as u32;
            for (offset, mut raster) in rasters.into_iter().enumerate() {
                raster.image_index = base_idx + offset as u32;
                doc.images.push(raster);
            }
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if used_ocr
            && ocr_rasters_bypass
            && extraction_method == ExtractionMethod::Ocr
            && config.images.as_ref().is_some_and(|c| c.include_page_rasters)
        {
            doc.processing_warnings.push(crate::types::ProcessingWarning {
                source: std::borrow::Cow::Borrowed("page_rasters"),
                message: std::borrow::Cow::Borrowed(
                    "include_page_rasters is set but no page rasters were produced; \
                     the active OCR backend used document-level processing without per-page rendering",
                ),
            });
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if used_ocr && !doc.images.is_empty() {
            let images_enabled = config.images.as_ref().map(|c| c.extract_images).unwrap_or(false)
                || config.pdf_options.as_ref().map(|p| p.extract_images).unwrap_or(false);
            if images_enabled && config.images.as_ref().map(|c| c.inject_placeholders).unwrap_or(false) {
                let referenced_images: std::collections::HashSet<u32> = doc
                    .elements
                    .iter()
                    .filter_map(|element| match element.kind {
                        ElementKind::Image { image_index } => Some(image_index),
                        _ => None,
                    })
                    .collect();
                let elems: Vec<InternalElement> = doc
                    .images
                    .iter()
                    .filter(|image| !referenced_images.contains(&image.image_index))
                    .map(|img| {
                        let elem = InternalElement::text(
                            ElementKind::Image {
                                image_index: img.image_index,
                            },
                            "",
                            0,
                        );
                        if let Some(page) = img.page_number {
                            elem.with_page(page)
                        } else {
                            elem
                        }
                    })
                    .collect();
                for elem in elems {
                    doc.push_element(elem);
                }
            }
        }

        if let Some(warning) = image_fallback_warning {
            doc.processing_warnings.push(warning);
        }
        doc.annotations = pdf_annotations;

        {
            use crate::types::annotations::PdfAnnotationType;
            use crate::types::uri::{ExtractedUri, UriKind};

            let uris: Vec<ExtractedUri> = doc
                .annotations
                .as_ref()
                .map(|annotations| {
                    annotations
                        .iter()
                        .filter(|a| a.annotation_type == PdfAnnotationType::Link)
                        .filter_map(|a| {
                            a.content.as_ref().map(|url| {
                                let kind = if url.starts_with('#') {
                                    UriKind::Anchor
                                } else if url.starts_with("mailto:") {
                                    UriKind::Email
                                } else {
                                    UriKind::Hyperlink
                                };
                                ExtractedUri {
                                    url: url.clone(),
                                    label: Some(url.clone()),
                                    page: Some(a.page_number),
                                    kind,
                                }
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            for uri in uris {
                doc.push_uri(uri);
            }
        }

        #[cfg(feature = "pdf")]
        {
            for uri in bookmark_uris {
                doc.push_uri(uri);
            }
            doc.revisions = pdf_revisions;
        }

        #[cfg(all(feature = "pdf", feature = "tokio-runtime"))]
        {
            let (embedded_children, embedded_warnings) =
                crate::pdf::embedded_files::extract_and_process_embedded_files(content, config).await;
            if !embedded_children.is_empty() {
                match doc.children {
                    Some(ref mut existing) => existing.extend(embedded_children),
                    None => doc.children = Some(embedded_children),
                }
            }
            for warning in embedded_warnings {
                doc.processing_warnings.push(warning);
            }
        }

        if let Some(ref mut pages) = final_pages {
            assign_hierarchy_to_pages(pages, &doc);
        }

        // GH#1653 + GH#1654: one raw-MediaBox coordinate frame per page that ended up with
        // hierarchy blocks, now that `assign_hierarchy_to_pages` above has populated
        // `PageContent::hierarchy`. One shared record covers both issues: a consumer needs
        // the MediaBox origin (which can be non-zero and negative) and the page rotation
        // together to place a page's raw-space geometry. Rides in `additional` rather than a
        // new public binding type, same as the `page_labels` key below (issue #66) and
        // `ocr_page_coordinate_frames` above (GH#1645). ~keep
        if let Some(ref pages) = final_pages {
            let pages_with_hierarchy: std::collections::HashSet<u32> = pages
                .iter()
                .filter(|page| {
                    page.hierarchy
                        .as_ref()
                        .is_some_and(|hierarchy| !hierarchy.blocks.is_empty())
                })
                .map(|page| page.page_number)
                .collect();
            let pdf_page_coordinate_frames: Vec<_> = pdf_page_coordinate_frames
                .into_iter()
                .filter(|frame| pages_with_hierarchy.contains(&frame.page_number))
                .collect();
            if !pdf_page_coordinate_frames.is_empty() {
                doc.metadata.additional.insert(
                    std::borrow::Cow::Borrowed("pdf_page_coordinate_frames"),
                    serde_json::json!(pdf_page_coordinate_frames),
                );
            }
        }

        doc.prebuilt_pages = final_pages;

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if !ocr_elements.is_empty() {
            doc.prebuilt_ocr_elements = Some(ocr_elements);
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if !ocr_formulas.is_empty() {
            doc.formulas = ocr_formulas;
        }

        // Native pages with layout hints carry formula elements holding the
        // region's glyph text; a configured model replaces it with LaTeX.
        #[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
        if let (Some(images), Some(detections), Some(layout)) = (
            markdown_layout_images.as_ref(),
            markdown_layout_detections.as_ref(),
            config.layout.as_ref(),
        ) {
            let mut side_channel = std::mem::take(&mut doc.formulas);
            recognize_pdf_formula_regions(Some(&mut doc), &mut side_channel, images, detections, layout, content).await;
            doc.formulas = side_channel;
        }

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if !ocr_llm_usage.is_empty() {
            doc.llm_usage = Some(ocr_llm_usage);
        }

        doc.diagrams = diagrams;

        tracing::debug!(
            elements = doc.elements.len(),
            tables = doc.tables.len(),
            has_pages = doc.prebuilt_pages.is_some(),
            diagrams = doc.diagrams.len(),
            "InternalDocument finalized (native path)"
        );

        #[cfg(all(feature = "liter-llm", feature = "layout-detection"))]
        {
            let vlm_enabled = config
                .ocr
                .as_ref()
                .map(|o| o.vlm_fallback != crate::core::config::VlmFallbackPolicy::Disabled && o.vlm_config.is_some())
                .unwrap_or(false);

            if vlm_enabled
                && let (Some(layout_images), Some(hints)) =
                    (markdown_layout_images.as_deref(), markdown_layout_hints.as_deref())
            {
                let vlm_cfg = config
                    .ocr
                    .as_ref()
                    .and_then(|o| o.vlm_config.as_ref())
                    .expect("vlm_config checked above");

                let region_results = region_vlm::extract_vlm_regions(layout_images, hints, vlm_cfg).await;
                if !region_results.is_empty() {
                    tracing::debug!(
                        count = region_results.len(),
                        "injecting VLM region results into document"
                    );
                    region_vlm::inject_region_results(&mut doc, region_results);
                }
            }
        }

        {
            let mut budget = crate::extractors::security::SecurityBudget::from_config(config);
            for elem in &doc.elements {
                budget.account_text(elem.text.len())?;
            }
        }

        Ok(doc)
    }

    /// Fallback extraction path when pdf feature is not enabled.
    #[cfg(not(feature = "pdf"))]
    async fn extract_core_native(
        &self,
        _content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
        _path: Option<&std::path::Path>,
    ) -> Result<InternalDocument> {
        let doc = InternalDocument::new(mime_type);
        Ok(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "ocr")]
    use crate::core::config::OcrQualityThresholds;
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    use serial_test::serial;

    #[cfg(all(feature = "pdf", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    #[tokio::test(flavor = "current_thread")]
    async fn native_pdf_blocking_phase_does_not_starve_the_timeout_driver() {
        let work = spawn_pdf_blocking(|| {
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok::<_, crate::XbergError>(())
        });

        let result = tokio::time::timeout(std::time::Duration::from_millis(10), work).await;

        assert!(
            result.is_err(),
            "the timeout must fire while native PDF work is still running"
        );
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn native_pdf_blocking_phase_stops_when_cancelled() {
        let token = crate::cancellation::CancellationToken::new();
        token.cancel();
        let config = ExtractionConfig {
            cancel_token: Some(token),
            ..Default::default()
        };

        let error = ensure_pdf_not_cancelled(&config).expect_err("cancelled native PDF work must stop");

        assert!(matches!(error, crate::XbergError::Cancelled));
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn should_suppress_extracted_pdf_metadata_when_disabled() {
        let mut metadata = Metadata {
            output_format: Some("markdown".to_string()),
            ..Default::default()
        };
        let extracted = crate::pdf::metadata::PdfExtractionMetadata {
            title: Some("Document title".to_string()),
            subject: Some("Document subject".to_string()),
            authors: Some(vec!["Author".to_string()]),
            keywords: Some(vec!["keyword".to_string()]),
            created_at: Some("2026-01-01".to_string()),
            modified_at: Some("2026-01-02".to_string()),
            created_by: Some("Producer".to_string()),
            pdf_specific: crate::pdf::metadata::PdfMetadata {
                page_count: Some(2),
                ..Default::default()
            },
            page_structure: Some(crate::types::PageStructure {
                total_count: 2,
                unit_type: crate::types::PageUnitType::Page,
                boundaries: None,
                pages: None,
            }),
        };

        apply_extracted_pdf_metadata(&mut metadata, extracted, false);

        assert_eq!(metadata.output_format.as_deref(), Some("markdown"));
        assert!(metadata.title.is_none());
        assert!(metadata.subject.is_none());
        assert!(metadata.authors.is_none());
        assert!(metadata.keywords.is_none());
        assert!(metadata.created_at.is_none());
        assert!(metadata.modified_at.is_none());
        assert!(metadata.created_by.is_none());
        assert!(metadata.pages.is_none());
        assert!(metadata.format.is_none());
    }

    #[cfg(feature = "pdf")]
    #[tokio::test]
    async fn should_validate_direct_pdf_config_before_opening_document() {
        let config = ExtractionConfig {
            pdf_options: Some(crate::core::config::PdfConfig {
                top_margin_fraction: Some(f32::NAN),
                ..Default::default()
            }),
            ..Default::default()
        };

        let error = PdfExtractor::new()
            .extract_content(b"not a PDF", "application/pdf", &config)
            .await
            .expect_err("invalid direct PDF config must fail before parsing");

        assert!(matches!(error, crate::XbergError::Validation { .. }));
        assert!(error.to_string().contains("top_margin_fraction"));
    }

    fn coverage_native_text() -> String {
        (0..MIN_NATIVE_TOKENS_FOR_STRUCTURE_COVERAGE)
            .map(|index| format!("token{index}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// `ExtractedDocument.pages` is derived from `element.page`, so a flat document with no
    /// page tags yields `pages: None` -- which a caller cannot distinguish from "this
    /// document has no pages". Reported against a 16-page scanned PDF where native mode
    /// returned 16 pages and enabling OCR returned `None`, so `list(doc.pages)` raised
    /// TypeError on the very mode that exists to give per-page OCR routing.
    #[test]
    fn should_tag_flat_document_paragraphs_with_their_page_numbers() {
        let text = "Page one body.\n\nPage two body.\n\nPage three body.";
        let first = text.find("Page two").expect("page two");
        let second = text.find("Page three").expect("page three");
        let boundaries = vec![
            crate::types::PageBoundary {
                byte_start: 0,
                byte_end: first,
                page_number: 1,
            },
            crate::types::PageBoundary {
                byte_start: first,
                byte_end: second,
                page_number: 2,
            },
            crate::types::PageBoundary {
                byte_start: second,
                byte_end: text.len(),
                page_number: 3,
            },
        ];

        let doc = flat_pdf_document(text, "application/pdf", Some(&boundaries));

        assert_eq!(doc.elements.len(), 3, "one paragraph per page");
        assert_eq!(
            doc.elements.iter().map(|e| e.page).collect::<Vec<_>>(),
            vec![Some(1), Some(2), Some(3)],
            "every paragraph must carry the page its content is on"
        );
    }

    /// Without boundaries the pre-existing untagged behaviour stands, so a caller that has
    /// no page information is no worse off than before.
    #[test]
    fn should_leave_flat_document_untagged_when_no_boundaries_are_available() {
        let doc = flat_pdf_document("Only body.\n\nSecond para.", "application/pdf", None);

        assert_eq!(doc.elements.len(), 2);
        assert!(
            doc.elements.iter().all(|e| e.page.is_none()),
            "no boundaries means no page attribution, not a guessed one"
        );
    }

    #[test]
    fn should_fall_back_to_native_text_when_structure_drops_substantial_content() {
        let native_text = coverage_native_text();
        let represented = native_text.split_whitespace().take(13).collect::<Vec<_>>().join(" ");
        let mut structured = InternalDocument::new("pdf");
        structured.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, represented, 0));

        let (selected, is_structured) =
            select_native_pdf_document(&native_text, "application/pdf", Some(structured), None);

        assert!(!is_structured);
        assert_eq!(selected.elements.len(), 1);
        assert_eq!(selected.elements[0].text, native_text);
    }

    #[test]
    fn should_keep_structure_when_native_token_coverage_meets_floor() {
        let native_text = coverage_native_text();
        let represented = native_text.split_whitespace().take(14).collect::<Vec<_>>().join(" ");
        let mut structured = InternalDocument::new("pdf");
        structured.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, represented, 0));

        let (selected, is_structured) =
            select_native_pdf_document(&native_text, "application/pdf", Some(structured), None);

        assert!(is_structured);
        assert!(matches!(selected.elements[0].kind, ElementKind::Heading { level: 1 }));
    }

    #[test]
    fn should_count_table_cells_as_structured_native_text() {
        let native_text = coverage_native_text();
        let mut structured = InternalDocument::new("pdf");
        structured.tables.push(crate::types::Table {
            cells: vec![native_text.split_whitespace().map(str::to_string).collect()],
            ..Default::default()
        });

        let (_, is_structured) = select_native_pdf_document(&native_text, "application/pdf", Some(structured), None);

        assert!(is_structured);
    }

    #[test]
    fn should_not_apply_structure_coverage_gate_to_trivial_native_text() {
        let mut structured = InternalDocument::new("pdf");
        structured.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Title", 0));

        let (selected, is_structured) =
            select_native_pdf_document("Title and body", "application/pdf", Some(structured), None);

        assert!(is_structured);
        assert!(matches!(selected.elements[0].kind, ElementKind::Heading { level: 1 }));
    }

    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    fn gate_decision(run_layout: bool) -> crate::pdf::layout_gate::PageGateDecision {
        crate::pdf::layout_gate::PageGateDecision {
            run_layout,
            reason: if run_layout {
                crate::pdf::layout_gate::GateReason::MultiColumn
            } else {
                crate::pdf::layout_gate::GateReason::PlainText
            },
        }
    }

    /// Gate-skipped pages carry placeholder rasters; any skip must refuse
    /// raster reuse so OCR never reads a blank square.
    #[cfg(all(
        feature = "pdf",
        feature = "layout-detection",
        any(feature = "ocr", feature = "ocr-pipeline")
    ))]
    #[test]
    fn markdown_layout_rasters_are_reusable_only_when_ungated_and_unrotated() {
        assert!(markdown_layout_reusable_for_ocr(None, &[]));
        assert!(markdown_layout_reusable_for_ocr(
            Some(&[gate_decision(true), gate_decision(true)]),
            &[0, 0]
        ));
        assert!(!markdown_layout_reusable_for_ocr(
            Some(&[gate_decision(true), gate_decision(false)]),
            &[0, 0]
        ));
        for rotation in [90, 180, 270] {
            assert!(!markdown_layout_reusable_for_ocr(None, &[rotation]));
        }
    }

    #[cfg(all(
        feature = "pdf",
        feature = "layout-detection",
        any(feature = "ocr", feature = "ocr-pipeline")
    ))]
    #[test]
    fn cpu_layout_retry_override_controls_downstream_ocr_models() {
        use crate::core::config::{
            acceleration::{AccelerationConfig, ExecutionProviderType},
            layout::LayoutDetectionConfig,
        };

        let config = ExtractionConfig {
            layout: Some(LayoutDetectionConfig {
                acceleration: Some(AccelerationConfig {
                    provider: ExecutionProviderType::CoreMl,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let effective = config_with_layout_acceleration_override(
            &config,
            Some(AccelerationConfig {
                provider: ExecutionProviderType::Cpu,
                ..Default::default()
            }),
        );

        assert_eq!(
            effective
                .resolved_layout_acceleration()
                .map(|acceleration| acceleration.provider.clone()),
            Some(ExecutionProviderType::Cpu)
        );
        assert_eq!(
            config
                .resolved_layout_acceleration()
                .map(|acceleration| acceleration.provider.clone()),
            Some(ExecutionProviderType::CoreMl)
        );
    }

    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    #[test]
    fn layout_gate_metadata_reports_one_indexed_skipped_pages_and_all_reasons() {
        let decisions = [gate_decision(true), gate_decision(false), gate_decision(false)];
        let (gated, reasons) = layout_gate_metadata(Some(&decisions));
        assert_eq!(gated, Some(vec![2, 3]));
        assert_eq!(
            reasons,
            Some(vec![
                "multi_column".to_string(),
                "plain_text".to_string(),
                "plain_text".to_string()
            ])
        );

        assert_eq!(layout_gate_metadata(None), (None, None));
    }

    #[cfg(feature = "pdf")]
    fn catalog(
        entries: impl IntoIterator<Item = (&'static str, xberg_native_pdf::object::Object)>,
    ) -> xberg_native_pdf::object::Object {
        xberg_native_pdf::object::Object::Dictionary(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        )
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn lopdf_compatibility_pass_skips_pdf_without_relevant_signals() {
        let catalog = catalog([]);

        assert!(!raw_pdf_needs_lopdf_compatibility_pass(
            b"%PDF-1.7\nordinary content\n%%EOF",
        ));
        assert!(!catalog_needs_lopdf_compatibility_pass(Some(&catalog)));
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn lopdf_compatibility_pass_keeps_raw_pdf_signals() {
        let catalog = catalog([]);

        assert!(raw_pdf_needs_lopdf_compatibility_pass(
            b"%PDF-1.7\ntrailer << /Prev 42 >>",
        ));
        assert!(raw_pdf_needs_lopdf_compatibility_pass(
            b"%PDF-1.7\n<< /Outlines 7 0 R >>",
        ));
        assert!(!catalog_needs_lopdf_compatibility_pass(Some(&catalog)));
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn lopdf_compatibility_pass_keeps_parsed_catalog_outline() {
        let catalog = catalog([("Outlines", xberg_native_pdf::object::Object::Null)]);

        assert!(catalog_needs_lopdf_compatibility_pass(Some(&catalog)));
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn lopdf_compatibility_pass_fails_open_without_catalog() {
        assert!(catalog_needs_lopdf_compatibility_pass(None));
    }

    #[cfg(feature = "pdf")]
    fn pdf_test_document(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../test_documents/pdf/{name}"))
    }

    #[cfg(feature = "pdf")]
    fn extraction_method(result: &crate::types::ExtractedDocument) -> Option<ExtractionMethod> {
        result.extraction_method
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn should_report_native_when_mixed_ocr_accepts_no_replacements() {
        let replacements = ahash::AHashMap::new();

        assert_eq!(
            extraction_method_after_mixed_ocr(&replacements),
            ExtractionMethod::Native
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn should_report_mixed_when_ocr_replacement_survives() {
        let replacements = ahash::AHashMap::from([(2, "authoritative OCR text".to_string())]);

        assert_eq!(
            extraction_method_after_mixed_ocr(&replacements),
            ExtractionMethod::Mixed
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn should_build_full_ocr_boundaries_against_exact_output_text() {
        let page_texts = vec![
            "first page  ".to_string(),
            "repeated page".to_string(),
            "repeated page".to_string(),
        ];
        let text = "<page 1>first page  \n\n<page 2>repeated page\n\n<page 3>repeated page";

        let boundaries = boundaries_for_ocr_output(ExtractionMethod::Ocr, text, None, None, Some(&page_texts))
            .expect("full OCR page texts should produce exact boundaries");

        assert_eq!(boundaries.len(), 3);
        for (boundary, expected) in boundaries.iter().zip(&page_texts) {
            assert_eq!(
                &text[boundary.byte_start..boundary.byte_end],
                expected,
                "each boundary must slice the exact OCR page text"
            );
        }
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn should_collect_requested_elements_when_plain_mixed_output_skips_structure_merge() {
        let mut page = InternalDocument::new("pdf");
        page.prebuilt_ocr_elements = Some(vec![crate::types::OcrElement {
            text: "accepted OCR element".to_string(),
            page_number: 1,
            ..Default::default()
        }]);
        let structured_pages = ahash::AHashMap::from([(1, page)]);
        let replacements = ahash::AHashMap::from([(1, "accepted OCR text".to_string())]);

        let (plain, _, _) = select_pdf_document(
            ExtractionMethod::Mixed,
            "accepted OCR text",
            "application/pdf",
            None,
            None,
            Some(&replacements),
            Some(&structured_pages),
            None,
            &crate::core::config::OutputFormat::Plain,
        );
        assert!(plain.prebuilt_ocr_elements.is_none());

        let elements = accepted_mixed_ocr_elements(&structured_pages);
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].text, "accepted OCR element");
        assert_eq!(elements[0].page_number, 1);
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn should_preserve_tables_without_injecting_text_when_plain_mixed_skips_structure_merge() {
        let table_markdown = "| hidden OCR table |";
        let mut page = InternalDocument::new("pdf");
        page.tables.push(crate::types::Table {
            markdown: table_markdown.to_string(),
            page_number: 2,
            ..Default::default()
        });
        let structured_pages = ahash::AHashMap::from([(2, page)]);
        let replacements = ahash::AHashMap::from([(2, "accepted OCR text".to_string())]);

        let (mut plain, _, _) = select_pdf_document(
            ExtractionMethod::Mixed,
            "accepted OCR text",
            "application/pdf",
            None,
            None,
            Some(&replacements),
            Some(&structured_pages),
            None,
            &crate::core::config::OutputFormat::Plain,
        );
        assert!(plain.tables.is_empty());

        let tables = accepted_mixed_ocr_tables(&structured_pages);
        attach_unrepresented_tables(&mut plain, tables);
        inject_unrepresented_table_elements(&mut plain, false);

        assert_eq!(plain.tables.len(), 1);
        assert_eq!(plain.tables[0].page_number, 2);
        assert_eq!(plain.tables[0].markdown, table_markdown);
        assert!(!crate::rendering::render_plain(&plain).contains(table_markdown));
    }

    #[cfg(all(feature = "pdf", feature = "ocr", feature = "chunking"))]
    fn assert_occurs_once(haystack: &str, needle: &str, projection: &str) {
        assert_eq!(
            haystack.matches(needle).count(),
            1,
            "mixed OCR text must occur exactly once in {projection}: {haystack}"
        );
    }

    /// A single letter-sized page with an empty `Contents` stream -- no native text at all,
    /// so the OCR fallback decision always flags it as a whole-document failure and the gate
    /// deterministically returns `OcrGateOutcome::RunFallback`.
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    fn blank_letter_page_pdf() -> Vec<u8> {
        use lopdf::{Document, Object, Stream, dictionary};

        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();
        let content_id = document.add_object(Stream::new(dictionary! {}, Vec::new()));
        let page_id = document.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Resources" => dictionary! {},
            "Contents" => content_id,
        });
        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);

        let mut bytes = Vec::new();
        document.save_to(&mut bytes).expect("fixture PDF must serialize");
        bytes
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    fn mixed_native_and_scanned_pdf() -> Vec<u8> {
        use lopdf::content::{Content, Operation};
        use lopdf::{Document, Object, Stream, dictionary};

        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();
        let font_id = document.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });

        let native_content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 18.into()]),
                Operation::new("Td", vec![72.into(), 720.into()]),
                Operation::new(
                    "Tj",
                    vec![Object::string_literal(
                        "Issue 1281 native text remains on page one with enough meaningful words to pass the automatic per-page quality gate and preserve mixed routing.",
                    )],
                ),
                Operation::new("ET", vec![]),
            ],
        };
        let native_content_id = document.add_object(Stream::new(
            dictionary! {},
            native_content.encode().expect("native PDF content must encode"),
        ));
        let native_page_id = document.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => native_content_id,
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });

        let image_id = document.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => 1,
                "Height" => 1,
                "ColorSpace" => "DeviceGray",
                "BitsPerComponent" => 8,
            },
            vec![0],
        ));
        let scanned_content = Content {
            operations: vec![
                Operation::new("q", vec![]),
                Operation::new(
                    "cm",
                    vec![612.into(), 0.into(), 0.into(), 792.into(), 0.into(), 0.into()],
                ),
                Operation::new("Do", vec![Object::Name(b"Scan".to_vec())]),
                Operation::new("Q", vec![]),
            ],
        };
        let scanned_content_id = document.add_object(Stream::new(
            dictionary! {},
            scanned_content.encode().expect("scanned PDF content must encode"),
        ));
        let scanned_page_id = document.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => scanned_content_id,
            "Resources" => dictionary! { "XObject" => dictionary! { "Scan" => image_id } },
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });

        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![native_page_id.into(), scanned_page_id.into()],
                "Count" => 2,
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);

        let mut bytes = Vec::new();
        document.save_to(&mut bytes).expect("mixed PDF fixture must serialize");
        bytes
    }

    /// A PDF of `page_count` pages, each a full-page image `XObject` with no text layer, the
    /// same scanned-page shape `mixed_native_and_scanned_pdf` uses for its one scanned page,
    /// repeated. Letter `MediaBox` on every page (612x792pt) so the render batch peak is
    /// uniform and predictable at the default 150 dpi: about 20.3MB per page.
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    fn all_scanned_pages_pdf(page_count: u32) -> Vec<u8> {
        use lopdf::content::{Content, Operation};
        use lopdf::{Document, Object, Stream, dictionary};

        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();

        let image_id = document.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => 1,
                "Height" => 1,
                "ColorSpace" => "DeviceGray",
                "BitsPerComponent" => 8,
            },
            vec![0],
        ));

        let mut page_ids = Vec::new();
        for _ in 0..page_count {
            let scanned_content = Content {
                operations: vec![
                    Operation::new("q", vec![]),
                    Operation::new(
                        "cm",
                        vec![612.into(), 0.into(), 0.into(), 792.into(), 0.into(), 0.into()],
                    ),
                    Operation::new("Do", vec![Object::Name(b"Scan".to_vec())]),
                    Operation::new("Q", vec![]),
                ],
            };
            let content_id = document.add_object(Stream::new(
                dictionary! {},
                scanned_content.encode().expect("scanned page content must encode"),
            ));
            let page_id = document.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
                "Resources" => dictionary! { "XObject" => dictionary! { "Scan" => image_id } },
                "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            });
            page_ids.push(page_id.into());
        }

        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => page_ids,
                "Count" => i64::from(page_count),
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);

        let mut bytes = Vec::new();
        document
            .save_to(&mut bytes)
            .expect("all-scanned PDF fixture must serialize");
        bytes
    }

    /// A single-page PDF whose only font is `Type0`/`Identity-H` with no `/CIDSystemInfo`
    /// ordering on its descendant `CIDFontType2`, and, when `with_tounicode` is `false`, no
    /// `/ToUnicode` either -- so xberg_native_pdf's §9.10.2 mapping cascade has no route to
    /// Unicode and falls back to a CID-as-Unicode echo (`MappingProvenance::Fallback`, issue
    /// #1254's signal, same font shape as that issue's own reproducer). CIDs are chosen equal
    /// to the Unicode codepoints of `text` (big-endian, two bytes each), so the echoed native
    /// text reads as ordinary ASCII -- structurally indistinguishable from real prose by every
    /// character-class heuristic, which is the shape of issue #1667's defect. No embedded font
    /// program: the mapping decision does not depend on one.
    ///
    /// When `with_tounicode` is `true`, an identity `/ToUnicode` CMap over printable ASCII
    /// (§9.10.3) is attached, which resolves `MappingProvenance::ToUnicode` -- a mapping tier
    /// the font's own file data backs, not a fallback echo -- so the page must NOT be routed
    /// to OCR under `Auto` (issue #1696's false-positive control: the fix must not treat every
    /// Type0 font as suspect, only ones with no usable mapping tier at all).
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    fn identity_h_mapping_pdf(text: &str, with_tounicode: bool) -> Vec<u8> {
        use lopdf::content::{Content, Operation};
        use lopdf::{Document, Object, Stream, dictionary};

        let cid_bytes: Vec<u8> = text.chars().flat_map(|c| (c as u32 as u16).to_be_bytes()).collect();

        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();

        let descriptor_id = document.add_object(dictionary! {
            "Type" => "FontDescriptor",
            "FontName" => "Synth+Fallback",
            "Flags" => 4,
            "FontBBox" => vec![0.into(), (-200).into(), 1000.into(), 900.into()],
            "ItalicAngle" => 0,
            "Ascent" => 800,
            "Descent" => (-200),
            "CapHeight" => 700,
            "StemV" => 80,
        });
        let descendant_id = document.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "CIDFontType2",
            "BaseFont" => "Synth+Fallback",
            "CIDSystemInfo" => dictionary! {
                "Registry" => Object::string_literal("Adobe"),
                "Ordering" => Object::string_literal("Identity"),
                "Supplement" => 0,
            },
            "FontDescriptor" => descriptor_id,
            "CIDToGIDMap" => "Identity",
            "DW" => 600,
        });

        // Identity mapping over printable ASCII (0x20-0x7E), which is the range `text`'s
        // codepoints are drawn from in every caller. Sequential `beginbfrange` form (ISO
        // 32000-1:2008 §9.10.3): a single dst start value, code and dst increment together. ~keep
        const TO_UNICODE_CMAP: &[u8] = b"/CIDInit /ProcSet findresource begin\n\
            12 dict begin\n\
            begincmap\n\
            /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
            /CMapName /Adobe-Identity-UCS def\n\
            1 begincodespacerange\n\
            <0000> <FFFF>\n\
            endcodespacerange\n\
            1 beginbfrange\n\
            <0020> <007E> <0020>\n\
            endbfrange\n\
            endcmap\n\
            CMapName currentdict /CMap defineresource pop\n\
            end\n\
            end";
        let to_unicode_id =
            with_tounicode.then(|| document.add_object(Stream::new(dictionary! {}, TO_UNICODE_CMAP.to_vec())));

        let mut font_dict = dictionary! {
            "Type" => "Font",
            "Subtype" => "Type0",
            "BaseFont" => "Synth+Fallback",
            "Encoding" => "Identity-H",
            "DescendantFonts" => vec![descendant_id.into()],
            // Deliberately no /ToUnicode when `with_tounicode` is false: severs every route to
            // Unicode (issue #1254).
        };
        if let Some(id) = to_unicode_id {
            font_dict.set("ToUnicode", id);
        }
        let font_id = document.add_object(font_dict);

        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 18.into()]),
                Operation::new("Td", vec![72.into(), 720.into()]),
                Operation::new("Tj", vec![Object::string_literal(cid_bytes)]),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = document.add_object(Stream::new(
            dictionary! {},
            content
                .encode()
                .expect("fabricated-provenance fixture content must encode"),
        ));
        let page_id = document.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });

        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);

        let mut bytes = Vec::new();
        document
            .save_to(&mut bytes)
            .expect("fabricated-provenance fixture must serialize");
        bytes
    }

    /// A single-page PDF, modeled on [`identity_h_mapping_pdf`], whose `/ToUnicode` CMap
    /// resolves every CID not to itself but to a ROT-`shift`ed letter (`a`-`z`/`A`-`Z` wrap
    /// within their own case; every other character passes through unshifted). Unlike
    /// [`identity_h_mapping_pdf`]'s `with_tounicode: false` case, this font DOES carry a
    /// `/ToUnicode` CMap, so provenance resolves `MappingProvenance::ToUnicode` -- a real,
    /// file-backed mapping tier -- not `Fallback`. Issue #1667's provenance signal must
    /// therefore NOT fire on this fixture; only issue #1696's language-plausibility detector
    /// can catch a `shift != 0` page, which is exactly the gap this builder exists to exercise.
    ///
    /// CIDs are chosen equal to the Unicode codepoints of `text`, mirroring
    /// [`identity_h_mapping_pdf`], so `shift: 0` is a structurally identical no-op control that
    /// must extract `text` verbatim.
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    fn shifted_to_unicode_pdf(text: &str, shift: u8) -> Vec<u8> {
        use lopdf::content::{Content, Operation};
        use lopdf::{Document, Object, Stream, dictionary};

        fn rot(c: char, shift: u8) -> char {
            let shift = shift % 26;
            if c.is_ascii_lowercase() {
                ((((c as u8 - b'a') + shift) % 26) + b'a') as char
            } else if c.is_ascii_uppercase() {
                ((((c as u8 - b'A') + shift) % 26) + b'A') as char
            } else {
                c
            }
        }

        let cid_bytes: Vec<u8> = text.chars().flat_map(|c| (c as u32 as u16).to_be_bytes()).collect();

        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();

        let descriptor_id = document.add_object(dictionary! {
            "Type" => "FontDescriptor",
            "FontName" => "Synth+Shifted",
            "Flags" => 4,
            "FontBBox" => vec![0.into(), (-200).into(), 1000.into(), 900.into()],
            "ItalicAngle" => 0,
            "Ascent" => 800,
            "Descent" => (-200),
            "CapHeight" => 700,
            "StemV" => 80,
        });
        let descendant_id = document.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "CIDFontType2",
            "BaseFont" => "Synth+Shifted",
            "CIDSystemInfo" => dictionary! {
                "Registry" => Object::string_literal("Adobe"),
                "Ordering" => Object::string_literal("Identity"),
                "Supplement" => 0,
            },
            "FontDescriptor" => descriptor_id,
            "CIDToGIDMap" => "Identity",
            "DW" => 600,
        });

        // Per-character `bfchar` entries rather than a `bfrange`: a ROT shift is not a fixed
        // offset once case wraps (`z` -> `a`, `Z` -> `A`), so each distinct source character
        // needs its own explicit mapping entry.
        let mut distinct: Vec<char> = text.chars().collect();
        distinct.sort_unstable();
        distinct.dedup();
        let mut bfchar_body = String::new();
        for c in &distinct {
            let mapped = rot(*c, shift);
            bfchar_body.push_str(&format!("<{:04X}> <{:04X}>\n", *c as u32, mapped as u32));
        }
        let to_unicode_cmap = format!(
            "/CIDInit /ProcSet findresource begin\n\
             12 dict begin\n\
             begincmap\n\
             /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
             /CMapName /Adobe-Identity-UCS def\n\
             1 begincodespacerange\n\
             <0000> <FFFF>\n\
             endcodespacerange\n\
             {} beginbfchar\n\
             {}\
             endbfchar\n\
             endcmap\n\
             CMapName currentdict /CMap defineresource pop\n\
             end\n\
             end",
            distinct.len(),
            bfchar_body
        );
        let to_unicode_id = document.add_object(Stream::new(dictionary! {}, to_unicode_cmap.into_bytes()));

        let font_dict = dictionary! {
            "Type" => "Font",
            "Subtype" => "Type0",
            "BaseFont" => "Synth+Shifted",
            "Encoding" => "Identity-H",
            "DescendantFonts" => vec![descendant_id.into()],
            "ToUnicode" => to_unicode_id,
        };
        let font_id = document.add_object(font_dict);

        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 18.into()]),
                Operation::new("Td", vec![72.into(), 720.into()]),
                Operation::new("Tj", vec![Object::string_literal(cid_bytes)]),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = document.add_object(Stream::new(
            dictionary! {},
            content.encode().expect("shifted-mapping fixture content must encode"),
        ));
        let page_id = document.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });

        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);

        let mut bytes = Vec::new();
        document
            .save_to(&mut bytes)
            .expect("shifted-mapping fixture must serialize");
        bytes
    }

    /// Genuine English prose, single line, well over 600 characters -- three full
    /// `language_detection::CHUNK_SIZE` chunks -- so `shifted_to_unicode_pdf`'s language-
    /// plausibility detector has enough prose to render a verdict on (issue #1696). Public
    /// domain (US Declaration of Independence, opening), pure ASCII letters/spaces so it
    /// round-trips through a two-byte CID exactly.
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    const WRONG_MAPPING_PROSE: &str = "When in the course of human events it becomes necessary \
        for one people to dissolve the political bands which have connected them with another \
        and to assume among the powers of the earth the separate and equal station to which the \
        laws of nature and of natures god entitle them a decent respect to the opinions of \
        mankind requires that they should declare the causes which impel them to the separation \
        we hold these truths to be self evident that all men are created equal that they are \
        endowed by their creator with certain unalienable rights that among these are life \
        liberty and the pursuit of happiness that to secure these rights governments are \
        instituted among men deriving their just powers from the consent of the governed";

    /// ROT-`shift` over ASCII letters only, mirroring `shifted_to_unicode_pdf`'s own mapping so
    /// the assertions below can predict the extracted text without re-deriving it.
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    fn rot_ascii_letters(text: &str, shift: u8) -> String {
        let shift = shift % 26;
        text.chars()
            .map(|c| {
                if c.is_ascii_lowercase() {
                    ((((c as u8 - b'a') + shift) % 26) + b'a') as char
                } else if c.is_ascii_uppercase() {
                    ((((c as u8 - b'A') + shift) % 26) + b'A') as char
                } else {
                    c
                }
            })
            .collect()
    }

    /// xberg#1696: a `/ToUnicode` CMap that resolves every glyph to a real, file-backed
    /// character, but consistently the WRONG one (a ROT-3 shift), reads as ordinary,
    /// structurally-clean prose to every character-shape heuristic -- including issue #1667's
    /// provenance check, since the mapping tier really is `ToUnicode`, not `Fallback`. Only the
    /// language/dictionary-plausibility signal can see that the decoded text is not English (or
    /// any real language) at all.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_auto_strategy_routes_wrong_mapping_page_to_ocr() {
        use crate::core::config::{OcrConfig, PageConfig};

        const OCR_TEXT: &str = "issue sixteen ninety six wrong mapping ocr replacement text";
        let _backend = register_mock_ocr_backend("pdf-1696-auto-wrong-mapping-routing", OCR_TEXT);
        let shifted = rot_ascii_letters(WRONG_MAPPING_PROSE, 3);

        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "pdf-1696-auto-wrong-mapping-routing".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            use_cache: false,
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(
                &shifted_to_unicode_pdf(WRONG_MAPPING_PROSE, 3),
                "application/pdf",
                &config,
            )
            .await
            .expect("wrong-mapping PDF extraction should succeed");
        let derived = crate::extraction::derive::derive_extraction_result(
            internal,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        assert!(
            !derived.content.contains(&shifted),
            "the wrong-mapped native text must not survive to the final content: {:?}",
            derived.content
        );
        assert_eq!(
            derived.content.matches(OCR_TEXT).count(),
            1,
            "Auto-routed OCR content must occur exactly once: {:?}",
            derived.content
        );
        assert_eq!(
            derived.extraction_method,
            Some(ExtractionMethod::Ocr),
            "Auto strategy must record extraction_method: ocr for a wrong-mapped page"
        );

        let implausible_text_pages = derived.metadata.format.as_ref().and_then(|format| match format {
            crate::types::FormatMetadata::Pdf(pdf) => pdf.implausible_text_pages.clone(),
            _ => None,
        });
        assert_eq!(
            implausible_text_pages,
            Some(vec![1]),
            "the wrong-mapped page must be listed in implausible_text_pages"
        );
    }

    /// `shift: 0` control for the test above: a real `/ToUnicode` CMap whose mapping happens to
    /// be the identity (equivalent to a correctly authored font) must never be routed to OCR --
    /// the language-plausibility signal must not treat every `/ToUnicode`-bearing font as
    /// suspect, only ones whose decoded text does not read as a real language.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_auto_strategy_keeps_native_text_for_identity_to_unicode() {
        use crate::core::config::{OcrConfig, PageConfig};

        const OCR_TEXT: &str = "mock ocr text that must never appear in a clean extraction";
        let _backend = register_mock_ocr_backend("pdf-1696-identity-false-positive", OCR_TEXT);

        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "pdf-1696-identity-false-positive".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            use_cache: false,
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(
                &shifted_to_unicode_pdf(WRONG_MAPPING_PROSE, 0),
                "application/pdf",
                &config,
            )
            .await
            .expect("identity-mapped PDF extraction should succeed");
        let derived = crate::extraction::derive::derive_extraction_result(
            internal,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        assert!(
            derived.content.contains(WRONG_MAPPING_PROSE),
            "genuinely mapped native text must survive to the final content: {:?}",
            derived.content
        );
        assert!(
            !derived.content.contains(OCR_TEXT),
            "a page whose ToUnicode mapping happens to be identity must not be routed to OCR: {:?}",
            derived.content
        );
        assert_eq!(
            derived.extraction_method,
            Some(ExtractionMethod::Native),
            "Auto strategy must keep extraction_method: native for a genuinely legible page"
        );

        let implausible_text_pages = derived.metadata.format.as_ref().and_then(|format| match format {
            crate::types::FormatMetadata::Pdf(pdf) => pdf.implausible_text_pages.clone(),
            _ => None,
        });
        assert_eq!(
            implausible_text_pages,
            Some(Vec::new()),
            "a page that reads as real English must not be listed as implausible"
        );
    }

    /// Table rows, not sentences: the line carries five alphabetic words but is over a third
    /// ASCII digits, so the prose gate's digit-ratio bound rejects all of it. The mapping is
    /// genuine and the text is correct; the document simply holds no prose for a language check
    /// to read. This is the shape of an invoice, a form or an agenda packet -- exactly the
    /// documents people scan (issue #1709). ~keep
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    fn no_prose_table_text() -> String {
        (1..=25)
            .map(|row| format!("Item {row:04} Qty 12 Unit 45.00 Tax 3.75 Total 48.75"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Issue #1709: the language-plausibility check abstains on a page with too little prose to
    /// judge, and an abstention is invisible. `implausible_text_pages: []` means both "every
    /// page was checked and is fine" and "no page could be checked at all", so a caller holding
    /// a document whose text layer decodes to the wrong letters cannot tell it from a clean one.
    ///
    /// The document here is legitimate and correctly mapped, so it must stay native and must
    /// flag no page. What must change is that the caller can see the check did not run.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_document_without_prose_reports_that_plausibility_was_not_evaluated() {
        use crate::core::config::{OcrConfig, PageConfig};

        const OCR_TEXT: &str = "mock ocr text that must never appear for a legitimate table";
        let _backend = register_mock_ocr_backend("pdf-1709-no-prose-abstention", OCR_TEXT);
        let table_text = no_prose_table_text();

        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "pdf-1709-no-prose-abstention".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            use_cache: false,
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(&shifted_to_unicode_pdf(&table_text, 0), "application/pdf", &config)
            .await
            .expect("table-only PDF extraction should succeed");
        let derived = crate::extraction::derive::derive_extraction_result(
            internal,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        assert_eq!(
            derived.extraction_method,
            Some(ExtractionMethod::Native),
            "a legitimate table document must stay native: {:?}",
            derived.extraction_method
        );
        assert!(
            !derived.content.contains(OCR_TEXT),
            "a legitimate table document must not be routed to OCR: {:?}",
            derived.content
        );

        let implausible_text_pages = derived.metadata.format.as_ref().and_then(|format| match format {
            crate::types::FormatMetadata::Pdf(pdf) => pdf.implausible_text_pages.clone(),
            _ => None,
        });
        assert_eq!(
            implausible_text_pages,
            Some(Vec::new()),
            "no page of a legitimate table document may be flagged as implausible"
        );

        let warnings = &derived.processing_warnings;
        assert!(
            warnings.iter().any(|warning| warning.message.contains("1709")),
            "a document the plausibility check could not judge on any page must say so, so that \
             an empty implausible_text_pages is not read as a clean bill of health: {warnings:?}"
        );
    }

    /// Forced OCR discards the native text layer, so the caller never receives the text the
    /// plausibility check abstained on. Reporting the abstention there would describe content
    /// nobody got and would close by advising the very thing the caller already did.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_forced_ocr_does_not_report_a_plausibility_abstention() {
        use crate::core::config::{OcrConfig, PageConfig};

        const OCR_TEXT: &str = "forced ocr replacement text for the table only fixture of issue \
            one thousand seven hundred and nine with plenty of additional alphanumeric content \
            so the native alnum retention guard keeps this replacement in place";
        let _backend = register_mock_ocr_backend("pdf-1709-forced-ocr-abstention", OCR_TEXT);
        let table_text = no_prose_table_text();

        let config = ExtractionConfig {
            force_ocr: true,
            ocr: Some(OcrConfig {
                backend: "pdf-1709-forced-ocr-abstention".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            use_cache: false,
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(&shifted_to_unicode_pdf(&table_text, 0), "application/pdf", &config)
            .await
            .expect("forced-OCR extraction of a table-only PDF should succeed");
        let derived = crate::extraction::derive::derive_extraction_result(
            internal,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        assert_eq!(
            derived.extraction_method,
            Some(ExtractionMethod::Ocr),
            "force_ocr must produce OCR text: {:?}",
            derived.extraction_method
        );

        let warnings = &derived.processing_warnings;
        assert!(
            !warnings.iter().any(|warning| warning.message.contains("1709")),
            "a caller who forced OCR must not be told the native layer could not be checked: \
             {warnings:?}"
        );
    }

    /// xberg#1338's "explicit OCR config" rule stays intact for the plausibility signal too: a
    /// wrong-mapped page must not be silently, automatically OCR'd when the caller never
    /// configured `ocr`. The defect must instead be surfaced as a warning, and the page's
    /// native (implausible) text is what the caller gets back.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_wrong_mapping_page_without_ocr_config_warns_and_keeps_native() {
        use crate::core::config::PageConfig;

        let shifted = rot_ascii_letters(WRONG_MAPPING_PROSE, 3);

        let config = ExtractionConfig {
            ocr: None,
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            use_cache: false,
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(
                &shifted_to_unicode_pdf(WRONG_MAPPING_PROSE, 3),
                "application/pdf",
                &config,
            )
            .await
            .expect("wrong-mapping PDF extraction without an ocr config should still succeed");
        let derived = crate::extraction::derive::derive_extraction_result(
            internal,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        assert!(
            derived.content.contains(&shifted),
            "without an explicit ocr config the wrong-mapped native text must be kept: {:?}",
            derived.content
        );
        assert_eq!(
            derived.extraction_method,
            Some(ExtractionMethod::Native),
            "Auto strategy must not silently OCR a page with no explicit ocr config"
        );

        let warnings = &derived.processing_warnings;
        assert!(
            warnings.iter().any(|warning| warning.message.contains("1696")),
            "the defect must be surfaced as a warning naming issue #1696 when no ocr config is \
             present: {warnings:?}"
        );

        let implausible_text_pages = derived.metadata.format.as_ref().and_then(|format| match format {
            crate::types::FormatMetadata::Pdf(pdf) => pdf.implausible_text_pages.clone(),
            _ => None,
        });
        assert_eq!(
            implausible_text_pages,
            Some(vec![1]),
            "the metadata must still record the implausible page even though it was not OCR'd"
        );
    }

    /// The opt-in `OcrStrategy::ScannedPages` strategy reads `scanned_pages`, which
    /// `record_implausible_text_pages` merges the language-plausibility signal into --
    /// `scanned_pages_to_ocr` must therefore pick up a wrong-mapped page without needing its
    /// own separate consultation of `implausible_text_pages`.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_scanned_pages_strategy_ocrs_implausible_pages() {
        use crate::core::config::{OcrConfig, OcrStrategy, PageConfig};

        // Long enough that its alnum count clears `MIN_OCR_NATIVE_ALNUM_RETENTION_RATIO`
        // against `WRONG_MAPPING_PROSE`'s ~600 native alnum characters: that unrelated guard
        // (`ocr::document`, #1678-era) rejects an OCR replacement that "recovers" less than
        // half of a page the character-shape gate still calls structurally healthy -- which a
        // wrong-but-ordinary-looking ROT-3 mapping does, despite being unreadable. A short mock
        // string would be rejected by that guard before this test ever exercises the routing
        // decision it is meant to check. ~keep
        const OCR_TEXT: &str = "scanned pages strategy ocr replacement text for issue one thousand \
            six hundred ninety six demonstrating that the language plausibility signal correctly \
            routed this wrong mapped page through the mixed native and ocr merge path under the \
            configured scanned pages strategy and confirming the ocr backend actually produced \
            this replacement content for the page under test with plenty of additional \
            alphanumeric characters to clear the native alnum retention guard";
        let _backend = register_mock_ocr_backend("pdf-1696-scanned-pages-strategy", OCR_TEXT);

        let config = ExtractionConfig {
            ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
            ocr: Some(OcrConfig {
                backend: "pdf-1696-scanned-pages-strategy".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            use_cache: false,
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(
                &shifted_to_unicode_pdf(WRONG_MAPPING_PROSE, 3),
                "application/pdf",
                &config,
            )
            .await
            .expect("wrong-mapping PDF extraction under ScannedPages should succeed");
        let derived = crate::extraction::derive::derive_extraction_result(
            internal,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        assert_eq!(
            derived.content.matches(OCR_TEXT).count(),
            1,
            "ScannedPages must route the implausible page to OCR via the merged scanned_pages \
             field: {:?}",
            derived.content
        );
        // The `ScannedPages` strategy always routes through the mixed OCR/native merge path
        // (`extract_mixed_ocr_native`), which reports `Mixed` whenever any page was replaced --
        // never `Ocr`, which is reserved for the whole-document `force_ocr`/`Auto` fallback
        // path. See `test_scanned_page_strategy_automatically_routes_only_the_scan_to_ocr` for
        // the same assertion on an unrelated fixture. ~keep
        assert_eq!(derived.extraction_method, Some(ExtractionMethod::Mixed));
    }

    /// A single page with a non-origin, negative-origin `MediaBox [10 -100 622 692]` (GH#1653)
    /// and two font sizes -- a 24pt heading line and a 10pt body paragraph, both at known raw
    /// user-space positions -- so hierarchy clustering assigns a heading level to the first and
    /// leaves the second as body text. `rotate` optionally sets `/Rotate` on the page (GH#1654).
    #[cfg(feature = "pdf")]
    fn coordinate_frame_test_pdf(rotate: Option<i32>) -> Vec<u8> {
        use lopdf::content::{Content, Operation};
        use lopdf::{Document, Object, Stream, dictionary};

        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();
        let font_id = document.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });

        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 24.into()]),
                Operation::new("Td", vec![82.into(), 500.into()]),
                Operation::new("Tj", vec![Object::string_literal("Coordinate Frame Heading")]),
                Operation::new("ET", vec![]),
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 10.into()]),
                Operation::new("Td", vec![82.into(), 460.into()]),
                Operation::new(
                    "Tj",
                    vec![Object::string_literal(
                        "This is a body paragraph with enough ordinary words to cluster as body text below the heading.",
                    )],
                ),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = document.add_object(Stream::new(
            dictionary! {},
            content.encode().expect("coordinate frame fixture content must encode"),
        ));

        let mut page_dict = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
            "MediaBox" => vec![10.into(), (-100).into(), 622.into(), 692.into()],
        };
        if let Some(rotate) = rotate {
            page_dict.set("Rotate", rotate);
        }
        let page_id = document.add_object(page_dict);

        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);

        let mut bytes = Vec::new();
        document
            .save_to(&mut bytes)
            .expect("coordinate frame fixture PDF must serialize");
        bytes
    }

    #[cfg(feature = "pdf")]
    fn coordinate_frame_extraction_config() -> ExtractionConfig {
        use crate::core::config::{HierarchyConfig, PdfConfig};

        ExtractionConfig {
            pdf_options: Some(PdfConfig {
                hierarchy: Some(HierarchyConfig {
                    enabled: true,
                    ..HierarchyConfig::default()
                }),
                ..PdfConfig::default()
            }),
            ..ExtractionConfig::default()
        }
    }

    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn pdf_page_coordinate_frame_reports_raw_media_box_origin_and_unswapped_extent() {
        let content = coordinate_frame_test_pdf(None);
        let config = coordinate_frame_extraction_config();
        let extractor = PdfExtractor::new();
        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("coordinate frame fixture must extract");

        let frames = result
            .metadata
            .additional
            .get("pdf_page_coordinate_frames")
            .expect("pdf_page_coordinate_frames must be present when a page has hierarchy blocks")
            .as_array()
            .expect("pdf_page_coordinate_frames must be a JSON array");
        assert_eq!(frames.len(), 1, "exactly one page has hierarchy blocks");

        let frame = &frames[0];
        assert_eq!(frame["page_number"], serde_json::json!(1));
        assert_eq!(frame["origin_x"], serde_json::json!(10.0));
        assert_eq!(frame["origin_y"], serde_json::json!(-100.0));
        assert_eq!(frame["width"], serde_json::json!(612.0));
        assert_eq!(frame["height"], serde_json::json!(792.0));
        assert_eq!(frame["unit"], serde_json::json!("point"));
        assert_eq!(frame["origin"], serde_json::json!("bottom_left"));
        assert_eq!(frame["clockwise_rotation"], serde_json::json!(0));
    }

    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn pdf_page_coordinate_frame_reports_rotation_without_swapping_extent() {
        let content = coordinate_frame_test_pdf(Some(90));
        let config = coordinate_frame_extraction_config();
        let extractor = PdfExtractor::new();
        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("rotated coordinate frame fixture must extract");

        let frames = result
            .metadata
            .additional
            .get("pdf_page_coordinate_frames")
            .expect("pdf_page_coordinate_frames must be present when a page has hierarchy blocks")
            .as_array()
            .expect("pdf_page_coordinate_frames must be a JSON array");
        assert_eq!(frames.len(), 1);

        let frame = &frames[0];
        assert_eq!(frame["clockwise_rotation"], serde_json::json!(90));
        // Deliberately un-swapped: this describes raw PDF user space, not the displayed frame.
        assert_eq!(frame["width"], serde_json::json!(612.0));
        assert_eq!(frame["height"], serde_json::json!(792.0));
    }

    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn pdf_page_coordinate_frame_omits_page_with_malformed_rotation() {
        let content = coordinate_frame_test_pdf(Some(135));
        let config = coordinate_frame_extraction_config();
        let extractor = PdfExtractor::new();
        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("malformed-rotation fixture must still extract successfully");

        assert!(
            !result.elements.is_empty(),
            "extraction must otherwise be unaffected by the malformed /Rotate"
        );

        match result.metadata.additional.get("pdf_page_coordinate_frames") {
            None => {}
            Some(value) => {
                let frames = value.as_array().expect("pdf_page_coordinate_frames must be an array");
                assert!(
                    frames.is_empty(),
                    "page with malformed /Rotate must be omitted entirely, got {frames:?}"
                );
            }
        }
    }

    #[test]
    fn flat_plain_text_retains_table_asset_without_duplicate_rendering() {
        const TABLE_TEXT: &str = "Account balance 42";

        let mut doc = InternalDocument::new("pdf");
        doc.push_element(InternalElement::text(ElementKind::Paragraph, TABLE_TEXT, 0));
        let table = crate::types::Table {
            cells: vec![vec!["Account balance".to_string(), "42".to_string()]],
            markdown: "| Account balance | 42 |".to_string(),
            page_number: 1,
            bounding_box: None,
            ..Default::default()
        };

        attach_unrepresented_tables(&mut doc, vec![table]);
        inject_unrepresented_table_elements(&mut doc, false);

        assert_eq!(doc.tables.len(), 1, "table assets must remain available to callers");
        assert!(
            !doc.elements
                .iter()
                .any(|element| matches!(element.kind, ElementKind::Table { .. })),
            "flat plain text must not append a second renderable copy of native table text"
        );

        let result =
            crate::extraction::derive::derive_extraction_result(doc, true, crate::core::config::OutputFormat::Plain);
        assert_eq!(result.content.matches(TABLE_TEXT).count(), 1);
        assert_eq!(result.tables.len(), 1);
    }

    #[test]
    fn table_element_injection_remains_available_for_structured_output() {
        let mut doc = InternalDocument::new("pdf");
        let table = crate::types::Table {
            cells: vec![vec!["Heading".to_string(), "Value".to_string()]],
            markdown: "| Heading | Value |".to_string(),
            page_number: 1,
            bounding_box: None,
            ..Default::default()
        };

        attach_unrepresented_tables(&mut doc, vec![table]);
        inject_unrepresented_table_elements(&mut doc, true);

        assert_eq!(doc.tables.len(), 1);
        assert_eq!(
            doc.elements
                .iter()
                .filter(|element| matches!(element.kind, ElementKind::Table { .. }))
                .count(),
            1,
            "structured and OCR paths must still be able to render attached table assets"
        );
    }

    /// pdfa_031: the structure-coverage safeguard falls back to `flat_pdf_document`,
    /// which carries no `Table` element, so a table reconstructed from prose that is
    /// still in the flat text used to be rendered a second time.
    #[test]
    fn structured_output_skips_tables_already_present_in_the_element_stream() {
        let mut doc = InternalDocument::new("pdf");
        doc.push_element(InternalElement::text(
            ElementKind::Paragraph,
            "Persons committed to the custody of a sheriff shall be confined\nin the facilities designated by law.",
            0,
        ));
        let table = crate::types::Table {
            cells: vec![
                vec![
                    "Persons committed to the custody".to_string(),
                    "of a sheriff".to_string(),
                ],
                vec![
                    "shall be confined in the facilities".to_string(),
                    "designated by law".to_string(),
                ],
            ],
            markdown: "| Persons committed to the custody | of a sheriff |".to_string(),
            page_number: 1,
            bounding_box: None,
            ..Default::default()
        };

        attach_unrepresented_tables(&mut doc, vec![table]);
        inject_unrepresented_table_elements(&mut doc, true);

        assert_eq!(doc.tables.len(), 1, "table assets must remain available to callers");
        assert!(
            !doc.elements
                .iter()
                .any(|element| matches!(element.kind, ElementKind::Table { .. })),
            "a table whose cells are already rendered must not be injected a second time"
        );
    }

    #[test]
    fn structured_output_still_injects_tables_missing_from_the_element_stream() {
        let mut doc = InternalDocument::new("pdf");
        doc.push_element(InternalElement::text(
            ElementKind::Paragraph,
            "Narrative prose that shares none of the tabulated content.",
            0,
        ));
        let table = crate::types::Table {
            cells: vec![
                vec!["Region".to_string(), "Revenue".to_string(), "Growth".to_string()],
                vec!["North".to_string(), "1200".to_string(), "4 percent".to_string()],
                vec!["South".to_string(), "980".to_string(), "7 percent".to_string()],
            ],
            markdown: "| Region | Revenue | Growth |".to_string(),
            page_number: 1,
            bounding_box: None,
            ..Default::default()
        };

        attach_unrepresented_tables(&mut doc, vec![table]);
        inject_unrepresented_table_elements(&mut doc, true);

        assert_eq!(
            doc.elements
                .iter()
                .filter(|element| matches!(element.kind, ElementKind::Table { .. }))
                .count(),
            1,
            "dropped table content must still be recovered by injection"
        );
    }

    /// The containment check abstains below `MIN_TABLE_TOKENS_FOR_CONTAINMENT`, so a
    /// short table cannot be suppressed by an incidental token overlap.
    #[test]
    fn short_tables_are_injected_even_when_their_tokens_appear_in_the_text() {
        let mut doc = InternalDocument::new("pdf");
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "total 42 balance", 0));
        let table = crate::types::Table {
            cells: vec![vec!["Total".to_string(), "42".to_string()]],
            markdown: "| Total | 42 |".to_string(),
            page_number: 1,
            bounding_box: None,
            ..Default::default()
        };

        attach_unrepresented_tables(&mut doc, vec![table]);
        inject_unrepresented_table_elements(&mut doc, true);

        assert_eq!(
            doc.elements
                .iter()
                .filter(|element| matches!(element.kind, ElementKind::Table { .. }))
                .count(),
            1
        );
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn ocr_tables_replace_native_tables_and_are_sorted() {
        let table = |markdown: &str, page_number| crate::types::Table {
            cells: Vec::new(),
            markdown: markdown.to_string(),
            page_number,
            bounding_box: None,
            ..Default::default()
        };
        let mut tables = vec![table("native", 1)];

        replace_tables_with_ocr_output(&mut tables, vec![table("ocr-page-2", 2), table("ocr-page-1", 1)]);

        assert_eq!(tables.len(), 2);
        assert_eq!(tables[0].markdown, "ocr-page-1");
        assert_eq!(tables[1].markdown, "ocr-page-2");
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn empty_ocr_tables_preserve_native_tables() {
        let mut tables = vec![crate::types::Table {
            cells: Vec::new(),
            markdown: "native".to_string(),
            page_number: 1,
            bounding_box: None,
            ..Default::default()
        }];

        replace_tables_with_ocr_output(&mut tables, Vec::new());

        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].markdown, "native");
    }

    /// GH#1670: a mixed-path document where only page 2 goes to OCR must keep the native
    /// tables of every other page. The bug replaced the whole document's table list with only
    /// page 2's OCR table, dropping pages 1 and 3 entirely.
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn mixed_ocr_tables_replace_only_the_pages_ocr_produced_a_table_for() {
        let native_table = |page_number| crate::types::Table {
            cells: Vec::new(),
            markdown: format!("native-page-{page_number}"),
            page_number,
            bounding_box: None,
            ..Default::default()
        };
        let mut tables = vec![native_table(1), native_table(2), native_table(3)];

        let mut ocr_page = InternalDocument::new("pdf");
        ocr_page.tables.push(crate::types::Table {
            cells: Vec::new(),
            markdown: "ocr-page-2".to_string(),
            page_number: 2,
            bounding_box: None,
            ..Default::default()
        });
        let structured_pages = ahash::AHashMap::from([(2, ocr_page)]);

        merge_mixed_ocr_tables(&mut tables, &structured_pages);

        assert_eq!(
            tables.len(),
            3,
            "pages 1 and 3 were never sent to OCR; their native tables must survive"
        );
        assert_eq!(tables[0].markdown, "native-page-1");
        assert_eq!(
            tables[1].markdown, "ocr-page-2",
            "page 2 went to OCR; its native table is superseded"
        );
        assert_eq!(tables[2].markdown, "native-page-3");
    }

    /// A page that goes to OCR but whose OCR pass finds no table must keep its native table,
    /// the same rule `replace_tables_with_ocr_output` applies when OCR finds nothing anywhere.
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn a_page_ocr_ran_on_but_found_no_table_for_keeps_its_native_table() {
        let mut tables = vec![crate::types::Table {
            cells: Vec::new(),
            markdown: "native-page-1".to_string(),
            page_number: 1,
            bounding_box: None,
            ..Default::default()
        }];

        // Page 1 went to OCR (it is a key in structured_pages) but produced no table there.
        let structured_pages = ahash::AHashMap::from([(1, InternalDocument::new("pdf"))]);

        merge_mixed_ocr_tables(&mut tables, &structured_pages);

        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].markdown, "native-page-1");
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn full_ocr_flat_fallback_never_selects_stale_native_document() {
        let mut native = InternalDocument::new("pdf");
        native.push_element(InternalElement::text(ElementKind::Paragraph, "stale native content", 0));

        let (doc, origin, structured) = select_pdf_document(
            ExtractionMethod::Ocr,
            "authoritative OCR content",
            "application/pdf",
            Some(native),
            None,
            None,
            None,
            None,
            &crate::core::config::OutputFormat::Markdown,
        );

        assert_eq!(origin, PdfDocumentOrigin::Ocr);
        assert!(!structured);
        assert!(
            doc.elements
                .iter()
                .any(|element| element.text == "authoritative OCR content")
        );
        assert!(
            !doc.elements
                .iter()
                .any(|element| element.text == "stale native content")
        );
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn mixed_origin_replaces_only_targeted_structured_page() {
        let mut native = InternalDocument::new("pdf");
        let table = crate::types::Table {
            cells: vec![vec!["native table".to_string()]],
            markdown: "| native table |".to_string(),
            page_number: 2,
            bounding_box: None,
            ..Default::default()
        };
        native.tables.push(table.clone());
        native.push_element(InternalElement::text(ElementKind::Paragraph, "native page one", 0).with_page(1));
        native.push_element(InternalElement::text(ElementKind::PageBreak, "", 0));
        native.push_element(InternalElement::text(ElementKind::Paragraph, "stale page two", 0).with_page(2));
        native.push_element(InternalElement::text(ElementKind::Table { table_index: 0 }, "", 0).with_page(2));
        let mut results = ahash::AHashMap::new();
        results.insert(2, "OCR page two".to_string());

        let (mut doc, origin, structured) = select_pdf_document(
            ExtractionMethod::Mixed,
            "flat text must not be selected",
            "application/pdf",
            Some(native),
            None,
            Some(&results),
            None,
            None,
            &crate::core::config::OutputFormat::Markdown,
        );

        assert_eq!(origin, PdfDocumentOrigin::Mixed);
        assert!(structured);
        assert!(doc.elements.iter().any(|element| element.text == "native page one"));
        assert!(doc.elements.iter().any(|element| element.text == "OCR page two"));
        assert!(!doc.elements.iter().any(|element| element.text == "stale page two"));
        attach_unrepresented_tables(&mut doc, vec![table]);
        inject_unrepresented_table_elements(&mut doc, false);
        assert_eq!(doc.tables.len(), 1, "structured table data must remain available");
        assert!(
            !doc.elements
                .iter()
                .any(|element| matches!(element.kind, ElementKind::Table { .. })),
            "target-page table markup must not be re-injected after OCR replacement"
        );
    }

    /// A fully scanned PDF has no native text layer at all, so
    /// `extract_document_structure_from_segments` (native pipeline) runs over zero
    /// segments, produces zero elements, and its caller collapses that to
    /// `pre_rendered_doc: None` (see `extraction.rs`'s `Ok(_) => None` arm). Before this
    /// fix, `ExtractionMethod::Mixed`'s `None` branch ignored `structured_ocr_pages`
    /// entirely and returned `flat_pdf_document`, which only ever emits `Paragraph`
    /// elements -- discarding every heading the document-global OCR heuristic recovered
    /// (matches the `ordinance_2197_scanned.pdf` field observation: `blocks_to_paragraphs`
    /// found ~71 headings, but the returned document, and its rendered markdown, had zero).
    /// Fails on unfixed code: `doc.elements` has no `Heading` (only the flat `Paragraph`
    /// from `text`), and the rendered markdown has no `#` line.
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn mixed_without_native_document_still_renders_ocr_structure_as_markdown() {
        let mut results = ahash::AHashMap::new();
        results.insert(1u32, "ANNUAL REPORT OVERVIEW\n\nReport body text.".to_string());

        let mut structured_page = InternalDocument::new("pdf");
        structured_page.push_element(
            InternalElement::text(ElementKind::Heading { level: 1 }, "ANNUAL REPORT OVERVIEW", 0).with_page(1),
        );
        structured_page
            .push_element(InternalElement::text(ElementKind::Paragraph, "Report body text.", 0).with_page(1));
        let mut structured_pages = ahash::AHashMap::new();
        structured_pages.insert(1u32, structured_page);

        let text = "ANNUAL REPORT OVERVIEW\n\nReport body text.";
        let boundaries = vec![crate::types::PageBoundary {
            byte_start: 0,
            byte_end: text.len(),
            page_number: 1,
        }];

        let (doc, origin, structured) = select_pdf_document(
            ExtractionMethod::Mixed,
            text,
            "application/pdf",
            None, // no native document -- the fully-scanned-PDF case
            None,
            Some(&results),
            Some(&structured_pages),
            Some(&boundaries),
            &crate::core::config::OutputFormat::Markdown,
        );

        assert_eq!(origin, PdfDocumentOrigin::Mixed);
        assert!(
            structured,
            "a document assembled from OCR-recovered structure must be reported as structured"
        );
        assert!(
            doc.elements
                .iter()
                .any(|element| matches!(element.kind, ElementKind::Heading { .. })),
            "expected a Heading element to survive the merge when there is no native document; \
             got element kinds: {:?}",
            doc.elements.iter().map(|element| &element.kind).collect::<Vec<_>>()
        );

        let markdown = crate::rendering::render_markdown(&doc);
        assert!(
            markdown.contains("# ANNUAL REPORT OVERVIEW"),
            "OCR document-global heading structure must reach the rendered markdown even when \
             native xberg_native_pdf extraction produced no structured document: {markdown:?}"
        );
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn full_ocr_structured_without_ocr_tables_injects_native_fallback_once() {
        let ocr_doc = InternalDocument::new("pdf");
        let table = crate::types::Table {
            cells: vec![vec!["native fallback".to_string()]],
            markdown: "| native fallback |".to_string(),
            page_number: 1,
            bounding_box: None,
            ..Default::default()
        };
        let (mut doc, origin, structured) = select_pdf_document(
            ExtractionMethod::Ocr,
            "OCR content",
            "application/pdf",
            None,
            Some(ocr_doc),
            None,
            None,
            None,
            &crate::core::config::OutputFormat::Markdown,
        );
        let allow_injection = !structured || (origin == PdfDocumentOrigin::Ocr && doc.tables.is_empty());

        attach_unrepresented_tables(&mut doc, vec![table]);
        inject_unrepresented_table_elements(&mut doc, allow_injection);

        assert_eq!(doc.tables.len(), 1);
        assert_eq!(
            doc.elements
                .iter()
                .filter(|element| matches!(element.kind, ElementKind::Table { .. }))
                .count(),
            1
        );
    }

    #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-pipeline")))]
    #[test]
    fn preparing_ocr_layout_inputs_transfers_raster_ownership() {
        let image = image::RgbImage::from_pixel(4, 3, image::Rgb([1, 2, 3]));
        let original_pixels = image.as_ptr();
        let detections = vec![crate::layout::DetectionResult {
            page_width: 4,
            page_height: 3,
            detections: Vec::new(),
        }];

        let (images, detections) = prepare_ocr_layout_inputs(vec![image], detections);

        let image::DynamicImage::ImageRgb8(transferred) = &images[0] else {
            panic!("RGB raster must retain its storage type");
        };
        assert_eq!(transferred.as_ptr(), original_pixels);
        assert_eq!((detections[0].page_width, detections[0].page_height), (4, 3));
    }

    #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-pipeline")))]
    #[test]
    fn preparing_ocr_layout_inputs_discards_only_mismatched_page_detections() {
        let images = vec![image::RgbImage::new(4, 3), image::RgbImage::new(8, 6)];
        let detections = vec![
            crate::layout::DetectionResult {
                page_width: 4,
                page_height: 3,
                detections: Vec::new(),
            },
            crate::layout::DetectionResult {
                page_width: 16,
                page_height: 12,
                detections: Vec::new(),
            },
        ];

        let (_, detections) = prepare_ocr_layout_inputs(images, detections);

        assert_eq!((detections[0].page_width, detections[0].page_height), (4, 3));
        assert_eq!((detections[1].page_width, detections[1].page_height), (8, 6));
        assert!(detections[1].detections.is_empty());
    }

    #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-pipeline")))]
    #[test]
    fn preparing_ocr_layout_inputs_repairs_detection_cardinality() {
        let images = vec![image::RgbImage::new(4, 3), image::RgbImage::new(8, 6)];

        let (_, detections) = prepare_ocr_layout_inputs(images, Vec::new());

        assert_eq!(detections.len(), 2);
        assert_eq!((detections[0].page_width, detections[0].page_height), (4, 3));
        assert_eq!((detections[1].page_width, detections[1].page_height), (8, 6));
    }

    #[cfg(feature = "ocr")]
    fn mk_decision(fallback: bool, whole_doc_failure: bool, failing_pages: Vec<u32>) -> ocr::OcrFallbackDecision {
        ocr::OcrFallbackDecision {
            stats: ocr::NativeTextStats::default(),
            avg_non_whitespace: 0.0,
            avg_alnum: 0.0,
            fallback,
            failing_pages,
            whole_doc_failure,
        }
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    struct MockPdfOcrBackend {
        name: &'static str,
        content: &'static str,
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    impl crate::plugins::Plugin for MockPdfOcrBackend {
        fn name(&self) -> &str {
            self.name
        }

        fn version(&self) -> String {
            "1.0.0".to_string()
        }

        fn initialize(&self) -> crate::Result<()> {
            Ok(())
        }

        fn shutdown(&self) -> crate::Result<()> {
            Ok(())
        }
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[async_trait::async_trait]
    impl crate::plugins::OcrBackend for MockPdfOcrBackend {
        fn backend_type(&self) -> crate::plugins::OcrBackendType {
            crate::plugins::OcrBackendType::Custom
        }

        fn supports_language(&self, _lang: &str) -> bool {
            true
        }

        async fn process_image(
            &self,
            _image_bytes: &[u8],
            _config: &crate::core::config::OcrConfig,
        ) -> crate::Result<crate::types::ExtractedDocument> {
            Ok(crate::types::ExtractedDocument {
                content: self.content.to_string(),
                mime_type: std::borrow::Cow::Borrowed("text/plain"),
                ..Default::default()
            })
        }
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    struct RegisteredOcrBackendGuard {
        name: &'static str,
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    impl Drop for RegisteredOcrBackendGuard {
        fn drop(&mut self) {
            let _ = crate::plugins::unregister_ocr_backend(self.name);
        }
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    fn register_mock_ocr_backend(name: &'static str, content: &'static str) -> RegisteredOcrBackendGuard {
        crate::plugins::register_ocr_backend(std::sync::Arc::new(MockPdfOcrBackend { name, content })).unwrap();
        RegisteredOcrBackendGuard { name }
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    struct FailingPdfOcrBackend {
        name: &'static str,
        message: &'static str,
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    impl crate::plugins::Plugin for FailingPdfOcrBackend {
        fn name(&self) -> &str {
            self.name
        }

        fn version(&self) -> String {
            "1.0.0".to_string()
        }

        fn initialize(&self) -> crate::Result<()> {
            Ok(())
        }

        fn shutdown(&self) -> crate::Result<()> {
            Ok(())
        }
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[async_trait::async_trait]
    impl crate::plugins::OcrBackend for FailingPdfOcrBackend {
        fn backend_type(&self) -> crate::plugins::OcrBackendType {
            crate::plugins::OcrBackendType::Custom
        }

        fn supports_language(&self, _lang: &str) -> bool {
            true
        }

        async fn process_image(
            &self,
            _image_bytes: &[u8],
            _config: &crate::core::config::OcrConfig,
        ) -> crate::Result<crate::types::ExtractedDocument> {
            Err(crate::XbergError::Plugin {
                message: self.message.to_string(),
                plugin_name: self.name.to_string(),
            })
        }
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    fn register_failing_ocr_backend(name: &'static str, message: &'static str) -> RegisteredOcrBackendGuard {
        crate::plugins::register_ocr_backend(std::sync::Arc::new(FailingPdfOcrBackend { name, message })).unwrap();
        RegisteredOcrBackendGuard { name }
    }

    /// A backend that reports the two metadata keys `PageContent.ocr_confidence` is built from
    /// (`mean_text_conf`, `word_count`) on a declared, calibrated scale -- the shape a real
    /// Tesseract page arrives in, without needing tessdata on the machine running the test.
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    struct ConfidenceReportingOcrBackend {
        name: &'static str,
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    const CONFIDENCE_MOCK_TEXT: &str = "Ordinance number twenty seventeen authorizes the municipal drainage \
                                        improvement program described throughout this recorded document";
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    const CONFIDENCE_MOCK_MEAN_TEXT_CONF: i64 = 87;
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    const CONFIDENCE_MOCK_WORD_COUNT: u32 = 16;
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    const CONFIDENCE_MOCK_SCALE_MAX: f64 = 100.0;

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    impl crate::plugins::Plugin for ConfidenceReportingOcrBackend {
        fn name(&self) -> &str {
            self.name
        }

        fn version(&self) -> String {
            "1.0.0".to_string()
        }

        fn initialize(&self) -> crate::Result<()> {
            Ok(())
        }

        fn shutdown(&self) -> crate::Result<()> {
            Ok(())
        }
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[async_trait::async_trait]
    impl crate::plugins::OcrBackend for ConfidenceReportingOcrBackend {
        fn backend_type(&self) -> crate::plugins::OcrBackendType {
            crate::plugins::OcrBackendType::Custom
        }

        fn supports_language(&self, _lang: &str) -> bool {
            true
        }

        fn confidence_semantics(&self) -> crate::plugins::ConfidenceSemantics {
            crate::plugins::ConfidenceSemantics::Legibility {
                scale_max: CONFIDENCE_MOCK_SCALE_MAX,
            }
        }

        async fn process_image(
            &self,
            _image_bytes: &[u8],
            _config: &crate::core::config::OcrConfig,
        ) -> crate::Result<crate::types::ExtractedDocument> {
            let mut document = crate::types::ExtractedDocument {
                content: CONFIDENCE_MOCK_TEXT.to_string(),
                mime_type: std::borrow::Cow::Borrowed("text/plain"),
                ..Default::default()
            };
            document.metadata.additional.insert(
                std::borrow::Cow::Borrowed("mean_text_conf"),
                serde_json::Value::from(CONFIDENCE_MOCK_MEAN_TEXT_CONF),
            );
            document.metadata.additional.insert(
                std::borrow::Cow::Borrowed("word_count"),
                serde_json::Value::from(CONFIDENCE_MOCK_WORD_COUNT),
            );
            Ok(document)
        }
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    fn register_confidence_reporting_ocr_backend(name: &'static str) -> RegisteredOcrBackendGuard {
        crate::plugins::register_ocr_backend(std::sync::Arc::new(ConfidenceReportingOcrBackend { name })).unwrap();
        RegisteredOcrBackendGuard { name }
    }

    /// Git-tracked, one-page scanned fixture with no native text layer. Embedded rather than
    /// read at runtime so a missing file is a compile error, not a silently skipped test. ~keep
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    const SCANNED_HELLO_PDF: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ocr/scanned_hello.pdf"
    ));

    /// #1568 -- the force-OCR route (`config.force_ocr` -> `run_ocr_with_layout` ->
    /// `extract_with_ocr`). Fails on unfixed code: `ocr_confidence` was `None` on every page
    /// because the per-page confidence the OCR loop already computes was never returned.
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[tokio::test]
    #[serial]
    async fn force_ocr_route_attaches_page_ocr_confidence() {
        use crate::core::config::{OcrConfig, PageConfig};

        const BACKEND_NAME: &str = "pdf-page-confidence-force-ocr-mock";
        let _backend = register_confidence_reporting_ocr_backend(BACKEND_NAME);

        let config = ExtractionConfig {
            use_cache: false,
            force_ocr: true,
            ocr: Some(OcrConfig {
                backend: BACKEND_NAME.to_string(),
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(SCANNED_HELLO_PDF, "application/pdf", &config)
            .await
            .expect("force-OCR extraction of the scanned fixture must succeed");

        let pages = internal
            .prebuilt_pages
            .as_ref()
            .expect("extract_pages must produce page contents");
        assert_eq!(pages.len(), 1, "the fixture is a single-page scan: {pages:?}");
        let confidence = pages[0]
            .ocr_confidence
            .as_ref()
            .expect("an OCR'd page must carry its confidence summary");
        assert_eq!(confidence.backend, BACKEND_NAME);
        assert_eq!(confidence.word_count, CONFIDENCE_MOCK_WORD_COUNT);
        assert_eq!(
            confidence.score,
            Some(CONFIDENCE_MOCK_MEAN_TEXT_CONF as f64 / CONFIDENCE_MOCK_SCALE_MAX),
            "a calibrated backend's raw confidence must be normalized by its own scale"
        );
    }

    /// GH#1892: `numeric_repair` on the whole-document route (`force_ocr` -> `run_ocr_with_layout`
    /// -> `extract_with_ocr`), which had no repair call at all while the `force_ocr_pages` route
    /// repaired from the same flag.
    ///
    /// ~keep This asserts the *wiring*, which the unit tests on
    /// `apply_numeric_repair_to_whole_document_ocr` cannot: those call the helper directly and pass
    /// identically with both call sites deleted. The `numeric_repair: false` control is what makes
    /// the positive assertion mean something -- without it, a fixture whose number the repair never
    /// touched would read the same as a working repair.
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[tokio::test]
    #[serial]
    async fn force_ocr_route_applies_numeric_repair() {
        use crate::core::config::OcrConfig;

        const BACKEND_NAME: &str = "pdf-1892-force-ocr-numeric-repair";
        const OCR_TEXT: &str = "Total operating revenue 1172 dollars";
        let _backend = register_mock_ocr_backend(BACKEND_NAME, OCR_TEXT);

        let config_for = |numeric_repair: bool| ExtractionConfig {
            use_cache: false,
            force_ocr: true,
            ocr: Some(OcrConfig {
                backend: BACKEND_NAME.to_string(),
                numeric_repair,
                ..Default::default()
            }),
            ..Default::default()
        };

        let plain_content = |config: ExtractionConfig| async move {
            let internal = PdfExtractor::new()
                .extract_content(SCANNED_HELLO_PDF, "application/pdf", &config)
                .await
                .expect("force-OCR extraction of the scanned fixture must succeed");
            crate::extraction::derive::derive_extraction_result(
                internal,
                false,
                crate::core::config::OutputFormat::Plain,
            )
            .content
        };

        let unrepaired = plain_content(config_for(false)).await;
        let repaired = plain_content(config_for(true)).await;

        assert!(
            unrepaired.contains("1172"),
            "control: with the flag off the mock's bare number must survive, or this test measures \
             nothing: {unrepaired:?}"
        );
        assert!(
            repaired.contains("1,172"),
            "force_ocr takes the whole-document route, so `numeric_repair` must reach it: {repaired:?}"
        );
        assert!(
            !repaired.contains("1172"),
            "and the bare form must be replaced, not merely accompanied: {repaired:?}"
        );
    }

    /// xberg#1665: the render batch peak scales with the configured thread budget alone, with
    /// no notion of `security_limits.max_content_size`. A 4-page batch of Letter (612x792pt)
    /// pages at the default 150 dpi estimates to about 4 x 20.3MB = 81MB; with
    /// `max_content_size` set to 50MiB (well above any one page, but below the whole batch),
    /// the pre-fix code renders and validates the FULL 4-page batch and rejects it outright.
    /// `force_ocr_pages` is the explicit-request route (`extract_mixed_ocr_native`), which
    /// keeps a real validation failure a hard error rather than a silent native-text fallback
    /// (see the `~keep` comment on its call site), so on the base tree this call returns `Err`.
    /// The batch is no longer measured against that limit at all: each page's own peak is
    /// checked on its own, and 20.3MB clears a 50MiB limit, so all four pages reach OCR in one
    /// batch and the thread budget stops being the reason OCR turns off.
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[tokio::test]
    #[serial]
    async fn large_thread_budget_does_not_turn_off_ocr_at_a_fixed_content_limit() {
        use crate::core::config::{ConcurrencyConfig, OcrConfig, PageConfig};
        use crate::extractors::security::SecurityLimits;

        const OCR_TEXT: &str = "issue sixteen sixty five recovered scanned page text";
        let _backend = register_mock_ocr_backend("pdf-1665-batch-peak-thread-budget", OCR_TEXT);

        let config = ExtractionConfig {
            concurrency: Some(ConcurrencyConfig {
                max_threads: Some(4),
                max_concurrent_ocr: None,
            }),
            force_ocr_pages: Some(vec![1, 2, 3, 4]),
            security_limits: Some(SecurityLimits {
                max_content_size: 50 * 1024 * 1024,
                ..Default::default()
            }),
            ocr: Some(OcrConfig {
                backend: "pdf-1665-batch-peak-thread-budget".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            use_cache: false,
            ..Default::default()
        };

        let result = PdfExtractor::new()
            .extract_content(&all_scanned_pages_pdf(4), "application/pdf", &config)
            .await;

        let internal = result.expect(
            "a thread budget wider than the content limit must still complete: the batch must \
             shrink, not reject every scanned page",
        );
        let pages = internal
            .prebuilt_pages
            .as_ref()
            .expect("extract_pages must produce page contents");
        assert_eq!(pages.len(), 4, "the fixture has four pages: {pages:?}");
        for page in pages {
            assert!(
                page.content.contains(OCR_TEXT),
                "page {} must carry OCR'd text, not an empty native fallback: {:?}",
                page.page_number,
                page.content
            );
        }
    }

    /// #1568 -- the mixed / scanned-pages route (`config.force_ocr_pages` ->
    /// `extract_mixed_ocr_native`). Covered separately from the force-OCR route above because
    /// the two dispatch through entirely different code paths, and a fix to one leaves the
    /// other silently reporting `None`. Also pins the "absent means not OCR'd" contract: the
    /// natively extracted page must keep `ocr_confidence: None`.
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[tokio::test]
    #[serial]
    async fn scanned_pages_route_attaches_page_ocr_confidence_only_to_ocred_pages() {
        use crate::core::config::{OcrConfig, PageConfig};

        const BACKEND_NAME: &str = "pdf-page-confidence-mixed-mock";
        let _backend = register_confidence_reporting_ocr_backend(BACKEND_NAME);

        let config = ExtractionConfig {
            use_cache: false,
            force_ocr_pages: Some(vec![2]),
            ocr: Some(OcrConfig {
                backend: BACKEND_NAME.to_string(),
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(&mixed_native_and_scanned_pdf(), "application/pdf", &config)
            .await
            .expect("targeted OCR of page 2 must succeed");

        let pages = internal
            .prebuilt_pages
            .as_ref()
            .expect("extract_pages must produce page contents");
        assert_eq!(pages.len(), 2, "the fixture has two pages: {pages:?}");
        assert!(
            pages[0].ocr_confidence.is_none(),
            "page 1 was extracted natively and must not carry a fabricated confidence: {:?}",
            pages[0].ocr_confidence
        );
        let confidence = pages[1]
            .ocr_confidence
            .as_ref()
            .expect("the OCR'd page must carry its confidence summary");
        assert_eq!(confidence.backend, BACKEND_NAME);
        assert_eq!(confidence.word_count, CONFIDENCE_MOCK_WORD_COUNT);
        assert_eq!(
            confidence.score,
            Some(CONFIDENCE_MOCK_MEAN_TEXT_CONF as f64 / CONFIDENCE_MOCK_SCALE_MAX)
        );
    }

    #[test]
    fn test_pdf_extractor_plugin_interface() {
        let extractor = PdfExtractor::new();
        assert_eq!(extractor.name(), "pdf-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    /// The condition under which a failed automatic OCR fallback must propagate instead of
    /// returning an empty success. Observed on a real scanned document whose pages exceeded
    /// `security_limits.max_content_size`: extraction returned `ok`, empty content,
    /// `extraction_method: native`, `ocr_used: false`, `quality_score: 0.0`, and the caller had
    /// no way to distinguish that from a genuinely empty PDF. ~keep
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn a_failed_ocr_fallback_with_no_native_text_is_a_total_loss() {
        assert!(
            failed_ocr_fallback_is_total_loss(""),
            "no native text at all is a total loss"
        );
        assert!(
            failed_ocr_fallback_is_total_loss("\n\n  \t "),
            "whitespace-only native text is as total a loss as none: it carries no content"
        );
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn a_failed_ocr_fallback_with_native_text_still_returns_that_text() {
        assert!(
            !failed_ocr_fallback_is_total_loss("Ordinance No. 2024-17"),
            "native text below the OCR trigger is still worth returning with a warning"
        );
        assert!(
            !failed_ocr_fallback_is_total_loss("  x  "),
            "a single meaningful character is not a total loss"
        );
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[tokio::test]
    #[serial]
    async fn automatic_ocr_failure_with_no_native_text_returns_an_error() {
        const BACKEND_NAME: &str = "pdf-total-loss-ocr-failure";
        const FAILURE: &str = "deliberate total-loss OCR failure";
        let _backend = register_failing_ocr_backend(BACKEND_NAME, FAILURE);
        let content = crate::pdf::render::build_minimal_pdf_with_mediabox(612.0, 792.0);
        let config = ExtractionConfig {
            use_cache: false,
            ocr: Some(crate::core::config::OcrConfig {
                backend: BACKEND_NAME.to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let error = PdfExtractor::new()
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect_err("a failed automatic OCR fallback with no native text must not report success");

        match error {
            crate::XbergError::Plugin { plugin_name, message } => {
                assert_eq!(plugin_name, "ocr");
                assert!(
                    message.contains(FAILURE),
                    "the backend failure must propagate, got: {message}"
                );
            }
            other => panic!("the typed OCR failure must propagate, got: {other}"),
        }
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[tokio::test]
    #[serial]
    async fn automatic_ocr_failure_with_native_text_returns_native_content_and_warning() {
        use crate::core::config::{OcrConfig, PageConfig};

        const BACKEND_NAME: &str = "pdf-native-fallback-ocr-failure";
        const FAILURE: &str = "deliberate recoverable OCR failure";
        const NATIVE_TEXT: &str = "Issue 1281 native text remains on page one";
        let _backend = register_failing_ocr_backend(BACKEND_NAME, FAILURE);
        let config = ExtractionConfig {
            use_cache: false,
            ocr: Some(OcrConfig {
                backend: BACKEND_NAME.to_string(),
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(&mixed_native_and_scanned_pdf(), "application/pdf", &config)
            .await
            .expect("native text must remain available when targeted OCR fails");

        assert!(
            internal.content().contains(NATIVE_TEXT),
            "the native page text must survive: {}",
            internal.content()
        );
        assert_eq!(internal.ocr_page_failures.len(), 1);
        assert_eq!(internal.ocr_page_failures[0].page, 2);
        assert!(internal.ocr_page_failures[0].error.contains(FAILURE));
        assert!(!internal.ocr_page_failures[0].recovered);
        let warnings = internal
            .processing_warnings
            .iter()
            .filter(|warning| warning.source == "ocr")
            .collect::<Vec<_>>();
        // Three distinct OCR-source warnings belong here, not duplicates: one reports that
        // targeted OCR itself failed for page 2, one records the XObject retry that preceded
        // that failure, and the last says the language-plausibility check (issue #1709) could
        // not judge that page's retained native text at all. Assert on each warning's content
        // rather than a bare count, so a real regression in any one fails loudly. ~keep
        let fallback_failure_warnings = warnings
            .iter()
            .filter(|warning| warning.message.contains(FAILURE))
            .collect::<Vec<_>>();
        assert_eq!(
            fallback_failure_warnings.len(),
            1,
            "expected exactly one OCR fallback-failure warning: {warnings:?}"
        );

        let plausibility_abstention_warnings = warnings
            .iter()
            .filter(|warning| warning.message.contains("could not judge"))
            .collect::<Vec<_>>();
        assert_eq!(
            plausibility_abstention_warnings.len(),
            1,
            "expected exactly one plausibility-check abstention warning: {warnings:?}"
        );

        let xobject_retry_warnings = warnings
            .iter()
            .filter(|warning| {
                warning.message.contains("Page 2 contains 1 image XObject(s)")
                    && warning.message.contains("retried on the embedded image bytes")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            xobject_retry_warnings.len(),
            1,
            "expected exactly one page-2 XObject retry warning: {warnings:?}"
        );

        assert_eq!(
            warnings.len(),
            fallback_failure_warnings.len() + plausibility_abstention_warnings.len() + xobject_retry_warnings.len(),
            "unexpected extra OCR-source warning(s): {warnings:?}"
        );

        let result = crate::extraction::derive::derive_extraction_result(
            internal,
            true,
            crate::core::config::OutputFormat::Plain,
        );
        assert_eq!(extraction_method(&result), Some(ExtractionMethod::Native));
        assert!(!result.metadata.ocr_used);
    }

    #[test]
    fn test_pdf_extractor_supported_mime_types() {
        let extractor = PdfExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 1);
        assert!(mime_types.contains(&"application/pdf"));
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_should_fallback_to_ocr_for_empty_text() {
        assert!(ocr::evaluate_native_text_for_ocr("", Some(1), &OcrQualityThresholds::default()).fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_should_not_fallback_for_meaningful_text() {
        let sample = "This page has searchable vector text and should avoid OCR.";
        assert!(!ocr::evaluate_native_text_for_ocr(sample, Some(1), &OcrQualityThresholds::default()).fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_should_fallback_for_punctuation_only_text() {
        let sample = " . , ; : -- -- ";
        assert!(ocr::evaluate_native_text_for_ocr(sample, Some(2), &OcrQualityThresholds::default()).fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_no_boundaries_falls_back_to_whole_doc() {
        let text = "This document has enough meaningful words for evaluation purposes here.";
        let decision = ocr::evaluate_per_page_ocr(text, None, Some(1), &OcrQualityThresholds::default());
        assert!(!decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_empty_boundaries_falls_back_to_whole_doc() {
        let text = "This document has enough meaningful words for evaluation purposes here.";
        let decision = ocr::evaluate_per_page_ocr(text, Some(&[]), Some(1), &OcrQualityThresholds::default());
        assert!(!decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_all_pages_good() {
        use crate::types::PageBoundary;

        let page1 = "This first page has plenty of meaningful searchable text content here.";
        let page2 = "This second page also has plenty of meaningful searchable text content.";
        let text = format!("{}{}", page1, page2);
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: page1.len(),
                page_number: 1,
            },
            PageBoundary {
                byte_start: page1.len(),
                byte_end: text.len(),
                page_number: 2,
            },
        ];

        let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2), &OcrQualityThresholds::default());
        assert!(!decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_one_bad_page_triggers_fallback() {
        use crate::types::PageBoundary;

        let good_page = "This page has plenty of meaningful searchable text content for extraction.";
        let bad_page = " . ; ";
        let text = format!("{}{}", good_page, bad_page);
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: good_page.len(),
                page_number: 1,
            },
            PageBoundary {
                byte_start: good_page.len(),
                byte_end: text.len(),
                page_number: 2,
            },
        ];

        let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2), &OcrQualityThresholds::default());
        assert!(decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_empty_page_triggers_fallback() {
        use crate::types::PageBoundary;

        let good_page = "This page has plenty of meaningful searchable text content for extraction.";
        let empty_page = "";
        let text = format!("{}{}", good_page, empty_page);
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: good_page.len(),
                page_number: 1,
            },
            PageBoundary {
                byte_start: good_page.len(),
                byte_end: text.len(),
                page_number: 2,
            },
        ];

        let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2), &OcrQualityThresholds::default());
        assert!(decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_preserves_document_stats_on_fallback() {
        use crate::types::PageBoundary;

        let good_page = "This page has plenty of meaningful searchable text content for extraction.";
        let bad_page = " . ; ";
        let text = format!("{}{}", good_page, bad_page);
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: good_page.len(),
                page_number: 1,
            },
            PageBoundary {
                byte_start: good_page.len(),
                byte_end: text.len(),
                page_number: 2,
            },
        ];

        let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2), &OcrQualityThresholds::default());
        assert!(decision.fallback);
        assert!(decision.stats.non_whitespace > 0);
        assert!(decision.stats.meaningful_words > 0);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_invalid_boundaries_skipped() {
        use crate::types::PageBoundary;

        let text = "This page has plenty of meaningful searchable text content for extraction.";
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: text.len(),
                page_number: 1,
            },
            PageBoundary {
                byte_start: 999,
                byte_end: 9999,
                page_number: 2,
            },
        ];

        let decision = ocr::evaluate_per_page_ocr(text, Some(&boundaries), Some(1), &OcrQualityThresholds::default());
        assert!(!decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_multi_page_correct_page_count() {
        let text = "ab cd ef";
        let decision_wrong = ocr::evaluate_native_text_for_ocr(text, None, &OcrQualityThresholds::default());
        let decision_correct = ocr::evaluate_native_text_for_ocr(text, Some(20), &OcrQualityThresholds::default());
        assert!(
            decision_correct.avg_non_whitespace < decision_wrong.avg_non_whitespace,
            "Correct page count should produce lower per-page averages"
        );
    }

    /// #1336 bug 2: `ScannedPages` must honor its own whole-document-failure
    /// signal (OCR every page) rather than discarding it and relying on the
    /// caller's fallthrough to the `Auto` gate.
    #[cfg(feature = "ocr")]
    #[test]
    fn test_scanned_pages_to_ocr_returns_full_page_set_on_whole_doc_failure() {
        use crate::core::config::{OcrConfig, OcrStrategy};
        use crate::pdf::metadata::{PdfExtractionMetadata, PdfMetadata};

        let config = ExtractionConfig {
            ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
            ocr: Some(OcrConfig::default()),
            ..Default::default()
        };

        let pdf_metadata = PdfExtractionMetadata {
            title: None,
            subject: None,
            authors: None,
            keywords: None,
            created_at: None,
            modified_at: None,
            created_by: None,
            pdf_specific: PdfMetadata {
                page_count: Some(3),
                scanned_pages: Some(Vec::new()),
                ..Default::default()
            },
            page_structure: None,
        };

        // Empty native text everywhere triggers `whole_doc_failure` in the per-page gate.
        let pages = scanned_pages_to_ocr(&config, &pdf_metadata, "", None);

        assert_eq!(
            pages,
            Some(vec![1, 2, 3]),
            "whole-doc failure under ScannedPages must OCR every page, not discard the signal"
        );
    }

    /// Whole-doc failure with an unknown page count must not fabricate a page
    /// range; falling through to `None` (the `Auto` gate) is the safe default.
    #[cfg(feature = "ocr")]
    #[test]
    fn test_scanned_pages_to_ocr_falls_through_when_page_count_unknown() {
        use crate::core::config::{OcrConfig, OcrStrategy};
        use crate::pdf::metadata::{PdfExtractionMetadata, PdfMetadata};

        let config = ExtractionConfig {
            ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
            ocr: Some(OcrConfig::default()),
            ..Default::default()
        };

        let pdf_metadata = PdfExtractionMetadata {
            title: None,
            subject: None,
            authors: None,
            keywords: None,
            created_at: None,
            modified_at: None,
            created_by: None,
            pdf_specific: PdfMetadata {
                page_count: None,
                scanned_pages: Some(Vec::new()),
                ..Default::default()
            },
            page_structure: None,
        };

        let pages = scanned_pages_to_ocr(&config, &pdf_metadata, "", None);

        assert_eq!(pages, None);
    }

    /// Metadata for a `ScannedPages` routing test: `page_count` pages, of which
    /// `scanned_pages` were flagged by scan detection.
    #[cfg(feature = "ocr")]
    fn scanned_pages_metadata(
        page_count: Option<u32>,
        scanned_pages: Vec<u32>,
    ) -> crate::pdf::metadata::PdfExtractionMetadata {
        use crate::pdf::metadata::{PdfExtractionMetadata, PdfMetadata};

        PdfExtractionMetadata {
            title: None,
            subject: None,
            authors: None,
            keywords: None,
            created_at: None,
            modified_at: None,
            created_by: None,
            pdf_specific: PdfMetadata {
                page_count,
                scanned_pages: Some(scanned_pages),
                ..Default::default()
            },
            page_structure: None,
        }
    }

    /// GH#1752. Leaving `ocr_scanned_page_quality_gate` unset must reproduce the derived
    /// answer for every shape of `ocr` block.
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn scanned_page_quality_gate_unset_reproduces_the_presence_of_an_ocr_block() {
        use crate::core::config::OcrConfig;

        for ocr in [None, Some(OcrConfig::default())] {
            let config = ExtractionConfig {
                ocr: ocr.clone(),
                ..Default::default()
            };
            assert_eq!(
                config.ocr_scanned_page_quality_gate, None,
                "the setting must default to None"
            );
            assert_eq!(
                scanned_page_quality_gate_enabled(&config),
                ocr.is_some(),
                "with the setting unset the gate must follow the presence of an `ocr` block"
            );
        }
    }

    /// The same input the whole-document-failure test uses, but with the gate explicitly off:
    /// only the pages scan detection flagged may be selected. This is the setting that avoids
    /// paying for recognition across a whole mixed document.
    #[cfg(feature = "ocr")]
    #[test]
    fn scanned_page_quality_gate_off_selects_detected_scans_only_despite_an_ocr_block() {
        use crate::core::config::{OcrConfig, OcrStrategy};

        let config = ExtractionConfig {
            ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
            ocr: Some(OcrConfig::default()),
            ocr_scanned_page_quality_gate: Some(false),
            ..Default::default()
        };
        let pdf_metadata = scanned_pages_metadata(Some(3), vec![2]);

        // Empty native text would trigger `whole_doc_failure` and select all three pages if
        // the gate were running. ~keep
        assert_eq!(
            scanned_pages_to_ocr(&config, &pdf_metadata, "", None),
            Some(vec![2]),
            "with the gate off only detected scans may be selected"
        );
    }

    /// The mirror image: the gate on with no `ocr` block must behave exactly like the gate on
    /// with one, reading `OcrConfig::default()`'s thresholds.
    #[cfg(feature = "ocr")]
    #[test]
    fn scanned_page_quality_gate_on_without_an_ocr_block_matches_the_gate_with_one() {
        use crate::core::config::{OcrConfig, OcrStrategy};

        let pdf_metadata = scanned_pages_metadata(Some(3), Vec::new());

        let with_block = ExtractionConfig {
            ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
            ocr: Some(OcrConfig::default()),
            ..Default::default()
        };
        let without_block = ExtractionConfig {
            ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
            ocr: None,
            ocr_scanned_page_quality_gate: Some(true),
            ..Default::default()
        };

        assert_eq!(
            scanned_pages_to_ocr(&without_block, &pdf_metadata, "", None),
            Some(vec![1, 2, 3]),
            "the gate must run on default thresholds when switched on without an `ocr` block"
        );
        assert_eq!(
            scanned_pages_to_ocr(&without_block, &pdf_metadata, "", None),
            scanned_pages_to_ocr(&with_block, &pdf_metadata, "", None),
            "and must reach the same answer as the same gate with a default `ocr` block"
        );
    }

    /// The pre-GH#1752 pin: with the setting unset and no `ocr` block, `ScannedPages` selects
    /// detected scans only, even on input that would fail the gate outright.
    #[cfg(feature = "ocr")]
    #[test]
    fn scanned_page_quality_gate_unset_without_an_ocr_block_selects_detected_scans_only() {
        use crate::core::config::OcrStrategy;

        let config = ExtractionConfig {
            ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
            ocr: None,
            ..Default::default()
        };

        assert_eq!(
            scanned_pages_to_ocr(&config, &scanned_pages_metadata(Some(3), vec![2]), "", None),
            Some(vec![2]),
        );
        assert_eq!(
            scanned_pages_to_ocr(&config, &scanned_pages_metadata(Some(3), Vec::new()), "", None),
            None,
            "no detected scan and no gate means nothing to select"
        );
    }

    /// GH#1752 F1: mirrors `near_empty_fallback_unset_reproduces_the_1338_carve_out`'s oracle
    /// pattern below, which already compares `near_empty_ocr_fallback_applies`'s `Some(true)`
    /// arm against `automatic_ocr_backend_is_registered()` directly rather than forcing a
    /// particular build's registry state. Switched on without an `ocr` block,
    /// `scanned_page_quality_gate_enabled` must require a registered automatic backend the
    /// same way; before a fix it returned `true` unconditionally, which let
    /// `scanned_pages_to_ocr` enter the per-page gate (and its `whole_doc_failure` branch,
    /// selecting every page) with no backend able to run OCR on any of them.
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn scanned_page_quality_gate_on_without_an_ocr_block_requires_a_registered_backend() {
        let config = ExtractionConfig {
            ocr: None,
            ocr_scanned_page_quality_gate: Some(true),
            ..Default::default()
        };
        assert_eq!(
            scanned_page_quality_gate_enabled(&config),
            automatic_ocr_backend_is_registered(),
            "switched on without an `ocr` block, the gate must require a registered automatic \
             backend, exactly like its sibling `near_empty_ocr_fallback_applies`'s `Some(true)` arm"
        );
    }

    /// GH#1752 / #1338. `ocr_near_empty_fallback` unset must reproduce the derived condition:
    /// always with an `ocr` block, and without one only for genuinely absent native text.
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn near_empty_fallback_unset_reproduces_the_1338_carve_out() {
        use crate::core::config::OcrConfig;

        let with_block = ExtractionConfig {
            ocr: Some(OcrConfig::default()),
            ..Default::default()
        };
        let without_block = ExtractionConfig::default();

        assert_eq!(with_block.ocr_near_empty_fallback, None);
        assert_eq!(without_block.ocr_near_empty_fallback, None);

        assert!(
            near_empty_ocr_fallback_applies(&with_block, "a page with plenty of real native text"),
            "an explicit `ocr` block keeps its full per-page gate behaviour"
        );
        assert!(
            !near_empty_ocr_fallback_applies(&without_block, "a page with plenty of real native text"),
            "without a block, a legitimately sparse or short PDF must stay native (#1338)"
        );
        assert_eq!(
            near_empty_ocr_fallback_applies(&without_block, "   \n  "),
            automatic_ocr_backend_is_registered(),
            "without a block, genuinely absent native text reaches OCR iff a backend is registered"
        );
    }

    /// Switched on, the fallback stops requiring *completely* empty text -- that is the whole
    /// point: a scanned page carrying only a page label or scanner stamp must reach OCR. It
    /// still consults the backend registry exactly as the derived condition does.
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn near_empty_fallback_on_without_an_ocr_block_drops_the_empty_text_requirement() {
        let opted_in = ExtractionConfig {
            ocr: None,
            ocr_near_empty_fallback: Some(true),
            ..Default::default()
        };
        let derived = ExtractionConfig::default();

        assert_eq!(
            near_empty_ocr_fallback_applies(&opted_in, "Page 3 of 412"),
            automatic_ocr_backend_is_registered(),
            "the stamp-only page must now be judged by the quality gate, not by the empty-text test"
        );
        assert_eq!(
            near_empty_ocr_fallback_applies(&opted_in, "Page 3 of 412"),
            near_empty_ocr_fallback_applies(&derived, ""),
            "and the backend requirement is unchanged: only the text test widened"
        );
    }

    /// Switched off, the fallback never runs, even with an explicit `ocr` block.
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn near_empty_fallback_off_suppresses_the_branch_even_with_an_ocr_block() {
        use crate::core::config::OcrConfig;

        let config = ExtractionConfig {
            ocr: Some(OcrConfig::default()),
            ocr_near_empty_fallback: Some(false),
            ..Default::default()
        };

        assert!(!near_empty_ocr_fallback_applies(&config, ""));
        assert!(!near_empty_ocr_fallback_applies(&config, "real native text"));
    }

    /// GH#1890. `record_implausible_text_pages` warns that implausible pages were not routed to
    /// OCR; the `else if` in `extract_core_native` decides whether they actually are. Those
    /// are one fact, and the warning used to re-derive it, which made them disagree for two
    /// configurations. This is an oracle test, not a table of expectations: it compares the
    /// observable warning against `near_empty_ocr_fallback_applies` itself, so any future
    /// divergence fails here regardless of which side moved. `#[serial]` because the predicate's
    /// `Some(true)`-without-a-block arm consults the process-global OCR backend registry that the
    /// mock-backend tests in this module mutate. ~keep
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[test]
    #[serial]
    fn the_unrouted_implausible_page_warning_tracks_the_routing_predicate() {
        use crate::core::config::OcrConfig;

        let implausible = rot_ascii_letters(WRONG_MAPPING_PROSE, 3);
        let boundaries = vec![crate::types::PageBoundary {
            page_number: 1,
            byte_start: 0,
            byte_end: implausible.len(),
        }];

        for ocr_block in [None, Some(OcrConfig::default())] {
            for near_empty_fallback in [None, Some(true), Some(false)] {
                let config = ExtractionConfig {
                    ocr: ocr_block.clone(),
                    ocr_near_empty_fallback: near_empty_fallback,
                    ..Default::default()
                };
                let mut pdf_metadata = scanned_pages_metadata(Some(1), Vec::new());
                let mut warnings = Vec::new();

                let branch_runs = near_empty_ocr_fallback_applies(&config, &implausible);
                record_implausible_text_pages(
                    &config,
                    &mut pdf_metadata,
                    &implausible,
                    Some(&boundaries),
                    &mut warnings,
                );

                assert_eq!(
                    pdf_metadata.pdf_specific.implausible_text_pages,
                    Some(vec![1]),
                    "the ROT-3 page must be flagged as implausible, or this case asserts nothing \
                     (ocr_block: {}, ocr_near_empty_fallback: {near_empty_fallback:?})",
                    ocr_block.is_some()
                );

                let warned = warnings
                    .iter()
                    .any(|warning| warning.message.contains("do not read as any real detectable language"));
                assert_eq!(
                    warned,
                    !branch_runs,
                    "the not-routed-to-OCR warning must be emitted exactly when the automatic OCR \
                     branch will not run, and suppressed exactly when it will (ocr_block: {}, \
                     ocr_near_empty_fallback: {near_empty_fallback:?}, branch_runs: {branch_runs})",
                    ocr_block.is_some()
                );
            }
        }
    }

    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn test_pdf_batch_mode_validates_page_config_enabled() {
        use crate::core::config::PageConfig;

        let extractor = PdfExtractor::new();

        let config = ExtractionConfig {
            pages: Some(PageConfig {
                extract_pages: true,
                insert_page_markers: false,
                marker_format: "<!-- PAGE {page_num} -->".to_string(),
                keep_native_content: false,
            }),
            ..Default::default()
        };

        let pdf_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/google_doc_document.pdf");
        if let Ok(content) = std::fs::read(pdf_path) {
            let result = extractor.extract_content(&content, "application/pdf", &config).await;
            assert!(
                result.is_ok(),
                "Failed to extract PDF with page config: {:?}",
                result.err()
            );

            let extraction_result = result.unwrap();
            let extraction_result = crate::extraction::derive::derive_extraction_result(
                extraction_result,
                true,
                crate::core::config::OutputFormat::Plain,
            );
            assert!(
                !extraction_result.content.is_empty(),
                "Content should be extracted from PDF"
            );
        }
    }

    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn test_pdf_batch_mode_derives_pages_with_page_config_absent() {
        let extractor = PdfExtractor::new();
        let config = ExtractionConfig::default();

        let pdf_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/google_doc_document.pdf");
        if let Ok(content) = std::fs::read(pdf_path) {
            let result = extractor.extract_content(&content, "application/pdf", &config).await;
            assert!(
                result.is_ok(),
                "Failed to extract PDF without page config: {:?}",
                result.err()
            );

            let extraction_result = result.unwrap();
            let extraction_result = crate::extraction::derive::derive_extraction_result(
                extraction_result,
                true,
                crate::core::config::OutputFormat::Plain,
            );
            assert!(
                extraction_result.pages.is_some(),
                "Pages should always be derived: the default config leaves annotation extraction \
                 off, which forces per-page tracking for the annotation fallback regardless of an \
                 explicit pages config"
            );
        }
    }

    /// GH CI E2E `test_pdf_hierarchy_config`: `pdf_options.hierarchy.enabled` used to be a silent
    /// no-op. A `PageHierarchy` can only hang off a `PageContent`, and `page_contents` is produced
    /// only when `pages.extract_pages` is set -- which `PageConfig::default()` leaves `false` and
    /// nothing in the hierarchy config points at. Headings were detected, then dropped.
    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn test_pdf_hierarchy_enabled_populates_page_hierarchy_without_explicit_page_config() {
        use crate::core::config::{HierarchyConfig, PdfConfig};

        let pdf_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/pdf/embedded_images_tables.pdf");
        let Ok(content) = std::fs::read(pdf_path) else {
            return;
        };

        let config = ExtractionConfig {
            pdf_options: Some(PdfConfig {
                hierarchy: Some(HierarchyConfig {
                    enabled: true,
                    ..HierarchyConfig::default()
                }),
                ..PdfConfig::default()
            }),
            ..ExtractionConfig::default()
        };

        let extractor = PdfExtractor::new();
        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("hierarchy extraction should succeed");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        let pages = result.pages.as_ref().expect("hierarchy request must produce pages");
        let hierarchy = pages[0]
            .hierarchy
            .as_ref()
            .expect("pages[0].hierarchy must be populated when hierarchy.enabled is true");
        assert!(
            !hierarchy.blocks.is_empty(),
            "hierarchy must carry at least one block; got block_count={}",
            hierarchy.block_count
        );
    }

    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn test_pdf_page_marker_validation() {
        use crate::core::config::PageConfig;

        let extractor = PdfExtractor::new();

        let config = ExtractionConfig {
            pages: Some(PageConfig {
                extract_pages: true,
                insert_page_markers: true,
                marker_format: "\n\n<!-- PAGE {page_num} -->\n\n".to_string(),
                keep_native_content: false,
            }),
            ..Default::default()
        };

        let pdf_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/multi_page.pdf");
        if let Ok(content) = std::fs::read(pdf_path) {
            let result = extractor.extract_content(&content, "application/pdf", &config).await;
            assert!(
                result.is_ok(),
                "Failed to extract PDF with page markers: {:?}",
                result.err()
            );

            let extraction_result = result.unwrap();
            let extraction_result = crate::extraction::derive::derive_extraction_result(
                extraction_result,
                true,
                crate::core::config::OutputFormat::Plain,
            );
            let marker_placeholder = "<!-- PAGE ";
            if extraction_result.content.len() > 100 {
                assert!(
                    extraction_result.content.contains(marker_placeholder),
                    "Page markers should be inserted when configured and document has multiple pages"
                );
            }
        }
    }

    /// #1451: `max_pages` must reject a document once its page count is known, before
    /// per-page work (OCR/layout/rendering) starts. Against the unfixed extractor there is
    /// no `max_pages` field to set, so this test fails to compile; once the field exists but
    /// enforcement is missing, `extract_content` would return `Ok` here instead of the
    /// expected `SecurityError::TooManyPages`.
    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn should_reject_document_exceeding_max_pages() {
        let pdf_path = pdf_test_document("multi_page.pdf");
        let Ok(content) = std::fs::read(&pdf_path) else {
            return;
        };
        let real_page_count = xberg_native_pdf::PdfDocument::from_bytes(content.clone())
            .expect("fixture must parse")
            .page_count()
            .expect("fixture must expose a page count");
        assert!(
            real_page_count > 1,
            "fixture must have more than one page to exercise the limit"
        );

        let extractor = PdfExtractor::new();
        let config = ExtractionConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_pages: Some(real_page_count - 1),
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extractor.extract_content(&content, "application/pdf", &config).await;
        let error = result.expect_err("a document over max_pages must be rejected");
        let message = error.to_string();
        assert!(
            message.contains("too many pages") || message.contains("max_pages"),
            "error must name the limit that was hit: {message}"
        );
    }

    /// A base64-encoded, 3-page, AES-256 (R6) encrypted PDF whose user password is the
    /// obviously-fake literal `xberg-test-fake-password-1451`. No corpus fixture requires a
    /// real user password to open: every encrypted PDF under `test_documents/pdf/` (including
    /// `password_protected.pdf`, despite the name) authenticates with the empty password and
    /// only restricts copy/edit permissions, so none of them exercise the password-required
    /// path this test targets. Built with `qpdf --encrypt ... 256` from a minimal 3-page
    /// document and verified with `qpdf --check` (fails with no password or the wrong one,
    /// succeeds and reports 3 pages with the correct one) before being embedded here.
    #[cfg(feature = "pdf")]
    const ENCRYPTED_THREE_PAGE_PDF_BASE64: &str = concat!(
        "JVBERi0xLjcKJb/3ov4KMSAwIG9iago8PCAvRXh0ZW5zaW9ucyA8PCAvQURCRSA8PCAvQmFzZVZlcnNpb24gLzEuNyAv",
        "RXh0ZW5zaW9uTGV2ZWwgOCA+PiA+PiAvUGFnZXMgMiAwIFIgL1R5cGUgL0NhdGFsb2cgPj4KZW5kb2JqCjIgMCBvYmoK",
        "PDwgL0NvdW50IDMgL0tpZHMgWyAzIDAgUiA0IDAgUiA1IDAgUiBdIC9UeXBlIC9QYWdlcyA+PgplbmRvYmoKMyAwIG9i",
        "ago8PCAvQ29udGVudHMgNiAwIFIgL01lZGlhQm94IFsgMCAwIDIwMCAyMDAgXSAvUGFyZW50IDIgMCBSIC9SZXNvdXJj",
        "ZXMgPDwgL0ZvbnQgPDwgL0YxIDcgMCBSID4+ID4+IC9UeXBlIC9QYWdlID4+CmVuZG9iago0IDAgb2JqCjw8IC9Db250",
        "ZW50cyA4IDAgUiAvTWVkaWFCb3ggWyAwIDAgMjAwIDIwMCBdIC9QYXJlbnQgMiAwIFIgL1Jlc291cmNlcyA8PCAvRm9u",
        "dCA8PCAvRjEgNyAwIFIgPj4gPj4gL1R5cGUgL1BhZ2UgPj4KZW5kb2JqCjUgMCBvYmoKPDwgL0NvbnRlbnRzIDkgMCBS",
        "IC9NZWRpYUJveCBbIDAgMCAyMDAgMjAwIF0gL1BhcmVudCAyIDAgUiAvUmVzb3VyY2VzIDw8IC9Gb250IDw8IC9GMSA3",
        "IDAgUiA+PiA+PiAvVHlwZSAvUGFnZSA+PgplbmRvYmoKNiAwIG9iago8PCAvRmlsdGVyIC9GbGF0ZURlY29kZSAvTGVu",
        "Z3RoIDgwID4+CnN0cmVhbQq/uprLnpDr1L1WYuji7aoo8FC/AmNCG9HIRQLXttVpBBc7g+Wbs7gcxVkaZNEjLHKAT0E4",
        "zK38JxdAJjfL7XkpkCbptuaCZCGvvfQ0dyrxlWVuZHN0cmVhbQplbmRvYmoKNyAwIG9iago8PCAvQmFzZUZvbnQgL0hl",
        "bHZldGljYSAvU3VidHlwZSAvVHlwZTEgL1R5cGUgL0ZvbnQgPj4KZW5kb2JqCjggMCBvYmoKPDwgL0ZpbHRlciAvRmxh",
        "dGVEZWNvZGUgL0xlbmd0aCA4MCA+PgpzdHJlYW0K4JE0iPOQNpJRjCvrfuVuZ8G+7e3bbkuci4qETQEnA0RfeOqEZe5q",
        "L81EqqU/h7+KsfI+uoIT6tBO4uAdnj/i0054F0Q6VoQWb0PkpOiMLy1lbmRzdHJlYW0KZW5kb2JqCjkgMCBvYmoKPDwg",
        "L0ZpbHRlciAvRmxhdGVEZWNvZGUgL0xlbmd0aCA4MCA+PgpzdHJlYW0KKqDRR85TmYv59t1N7OdBCttbZfgU8lsnHRFY",
        "MueyVRSwbBDvnujXiRLztgcPFiS4XexkTW/ikEXnPHq9uFq1VDpPVQNpaRQlZKf9xlAUveVlbmRzdHJlYW0KZW5kb2Jq",
        "CjEwIDAgb2JqCjw8IC9DRiA8PCAvU3RkQ0YgPDwgL0F1dGhFdmVudCAvRG9jT3BlbiAvQ0ZNIC9BRVNWMyAvTGVuZ3Ro",
        "IDMyID4+ID4+IC9GaWx0ZXIgL1N0YW5kYXJkIC9MZW5ndGggMjU2IC9PIDw0ZmQzOTFhNjY0MzdlZjFkYzEwNDMxOGVj",
        "OTZhNDY1OTM5OWNhMzY2YTY1MTBmZjhkNTI1N2FjMmJiOGRmMGNkMGRiZjkzYTc4MDgzYTkzOGRhYzBlMjcwMzliMTMw",
        "ZmM+IC9PRSA8NDkwZTZlNzI0OGM2NmZkMjZjOTdjMzE3MWVlNjVlNjc0Yjk1YWM2ZWI1MTI1ZjVlZmVkMDBmOTI3NmQw",
        "Y2YxMj4gL1AgLTQgL1Blcm1zIDw1NmI5YWRkMWJiMWI2YjUwZGYyMjg3NWM1Njc5YTJmMz4gL1IgNiAvU3RtRiAvU3Rk",
        "Q0YgL1N0ckYgL1N0ZENGIC9VIDw0NTNlNjI2Y2JmZTI1MGE5N2FmNDllZWEzMmYwOWFmOWMyZDk4Y2MzMWM2ZGQ1NDdj",
        "NjkwYzU1NTMwZDhjN2ViYjA2MzdlN2JjZmZmMDdkZThmYjc2NmFmNWQ4ZDM1NWY+IC9VRSA8MTA0YWEyZjAzNDBiMTEw",
        "ZGEzZTFmMGQ0ODQ3NmQyOWMyZTExNmYxOTRlZTBjMjFmYWFiMzkzNDFiZmU2NjMyZT4gL1YgNSA+PgplbmRvYmoKeHJl",
        "ZgowIDExCjAwMDAwMDAwMDAgNjU1MzUgZiAKMDAwMDAwMDAxNSAwMDAwMCBuIAowMDAwMDAwMTMwIDAwMDAwIG4gCjAw",
        "MDAwMDAyMDEgMDAwMDAgbiAKMDAwMDAwMDMyOSAwMDAwMCBuIAowMDAwMDAwNDU3IDAwMDAwIG4gCjAwMDAwMDA1ODUg",
        "MDAwMDAgbiAKMDAwMDAwMDczNSAwMDAwMCBuIAowMDAwMDAwODA1IDAwMDAwIG4gCjAwMDAwMDA5NTUgMDAwMDAgbiAK",
        "MDAwMDAwMTEwNSAwMDAwMCBuIAp0cmFpbGVyIDw8IC9Sb290IDEgMCBSIC9TaXplIDExIC9JRCBbPGU4ODhiODFiZjdj",
        "YzRhODE3NzQ3YmM4ZDZlZTgxYzgxPjxmYzI0MzUwMTY1NzA4NjdhNDYzNTZiYTFiYWE5ZGVhND5dIC9FbmNyeXB0IDEw",
        "IDAgUiA+PgpzdGFydHhyZWYKMTY1MwolJUVPRgo=",
    );

    #[cfg(feature = "pdf")]
    const ENCRYPTED_THREE_PAGE_PDF_PASSWORD: &str = "xberg-test-fake-password-1451";

    /// An obviously-fake literal that is not this fixture's password, and is not empty.
    #[cfg(feature = "pdf")]
    const WRONG_PDF_PASSWORD: &str = "xberg-test-fake-wrong-password-1879";

    /// `PdfDocument::authenticate` reports a rejected password as `Ok(false)`, not as an `Err`,
    /// so `open_pdf_document` -- which mapped the error and dropped the bool -- accepted every
    /// wrong password. That is why no `pdf_page_count` fixture could prove a binding forwards
    /// its password argument: the page tree reads unauthenticated, and authentication itself
    /// never failed either. The extraction path (`open_bytes_with_passwords`) always read the
    /// bool and is unaffected. GH#1879.
    #[test]
    #[cfg(feature = "pdf")]
    fn should_reject_a_wrong_pdf_password_rather_than_ignore_it() {
        use base64::Engine as _;

        let content = base64::engine::general_purpose::STANDARD
            .decode(ENCRYPTED_THREE_PAGE_PDF_BASE64)
            .expect("fixture must be valid base64");

        let error = crate::pdf_page_count(&content, Some(WRONG_PDF_PASSWORD))
            .expect_err("a wrong non-empty user password must not authenticate");
        assert!(
            error.to_string().contains("authenticate"),
            "the error must name authentication as the cause: {error}"
        );

        assert_eq!(
            crate::pdf_page_count(&content, Some(ENCRYPTED_THREE_PAGE_PDF_PASSWORD))
                .expect("the correct user password must authenticate"),
            3,
            "the correct password must still open the document"
        );
        assert_eq!(
            crate::pdf_page_count(&content, None).expect("an unauthenticated page-tree read must still succeed"),
            3,
            "the page tree is structure, not encrypted data, so omitting the password is not an error"
        );
    }

    /// The corpus fixture whose user password is empty is still the right document for a
    /// binding-level wrong-password test: the empty password authenticates, and any non-empty
    /// password is rejected. So the e2e fixture needs no new corpus object. GH#1879.
    #[test]
    #[cfg(feature = "pdf")]
    fn should_reject_a_non_empty_password_on_a_pdf_whose_user_password_is_empty() {
        let Some(content) = crate::utils::read_test_fixture("pdf/password_protected.pdf") else {
            return;
        };

        assert_eq!(
            crate::pdf_page_count(&content, None).expect("an encrypted page tree reads without a password"),
            2
        );
        assert_eq!(
            crate::pdf_page_count(&content, Some("")).expect("the empty user password authenticates"),
            2
        );
        let error = crate::pdf_page_count(&content, Some(WRONG_PDF_PASSWORD))
            .expect_err("a non-empty password is neither the user nor the owner password");
        assert!(
            error.to_string().contains("authenticate"),
            "the error must name authentication as the cause: {error}"
        );
    }

    /// An encrypted document is subject to `max_pages` like any other. This is a
    /// characterization test, NOT a regression test — it passes against the pre-fix code
    /// too, and it is kept because the behaviour is worth pinning, not because it catches
    /// anything.
    ///
    /// It was originally written believing `xberg_native_pdf` refuses to count an unauthenticated
    /// document's pages, so an encrypted PDF would skip the cap. That is false, and measuring
    /// it is what settled the design: a PDF's page tree is structure, not string or stream
    /// data, so `xberg_native_pdf::PdfDocument::from_bytes(..).page_count()` returns 3 for this
    /// AES-256/R6 fixture with no password at all. A sweep of all 488 PDFs in
    /// `test_documents/` found exactly one document the unauthenticated count cannot handle
    /// (`corrupt_truncated.pdf`), which `lopdf` also cannot read and which fails extraction
    /// anyway. So `enforce_page_limit` needs no password handling.
    ///
    /// The fixture: 3 pages, AES-256/R6, built with `qpdf --encrypt ... 256` and verified
    /// with `qpdf --check` before embedding. Its password is the obviously-fake literal
    /// `xberg-test-fake-password-1451`. No corpus fixture requires a real user password —
    /// every encrypted PDF under `test_documents/pdf/` (including `password_protected.pdf`,
    /// despite the name) opens with the empty password and only restricts copy/edit.
    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn encrypted_document_over_max_pages_is_rejected_like_any_other() {
        use crate::core::config::pdf::PdfConfig;
        use base64::Engine as _;

        let content = base64::engine::general_purpose::STANDARD
            .decode(ENCRYPTED_THREE_PAGE_PDF_BASE64)
            .expect("fixture must be valid base64");

        let extractor = PdfExtractor::new();
        let config = ExtractionConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_pages: Some(2),
                ..Default::default()
            }),
            pdf_options: Some(PdfConfig {
                passwords: Some(vec![ENCRYPTED_THREE_PAGE_PDF_PASSWORD.to_string()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extractor.extract_content(&content, "application/pdf", &config).await;
        let error = result.expect_err("an encrypted document over max_pages must be rejected like any other");
        let message = error.to_string();
        assert!(
            message.contains("too many pages") || message.contains("max_pages"),
            "error must name the limit that was hit: {message}"
        );
    }

    /// A document exactly at the configured `max_pages` ceiling must extract in full — a
    /// limit that rejects the boundary case too is not the fix the issue asked for.
    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn should_extract_fully_when_page_count_is_at_max_pages() {
        let pdf_path = pdf_test_document("multi_page.pdf");
        let Ok(content) = std::fs::read(&pdf_path) else {
            return;
        };
        let real_page_count = xberg_native_pdf::PdfDocument::from_bytes(content.clone())
            .expect("fixture must parse")
            .page_count()
            .expect("fixture must expose a page count");

        let extractor = PdfExtractor::new();
        let config = ExtractionConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_pages: Some(real_page_count),
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extractor.extract_content(&content, "application/pdf", &config).await;
        let doc = result.expect("a document exactly at max_pages must extract fully, not be rejected");
        assert!(
            !doc.elements.is_empty(),
            "extraction at the boundary must still produce content, not an empty truncated result"
        );
    }

    /// The default `SecurityLimits` (no override) must extract a multi-page document exactly
    /// as before #1451: `max_pages` defaulting to anything other than `None` (unlimited) would
    /// silently start rejecting existing callers' documents.
    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn should_extract_normally_with_default_security_limits() {
        let pdf_path = pdf_test_document("multi_page.pdf");
        let Ok(content) = std::fs::read(&pdf_path) else {
            return;
        };

        let extractor = PdfExtractor::new();
        let config = ExtractionConfig::default();

        let result = extractor.extract_content(&content, "application/pdf", &config).await;
        assert!(
            result.is_ok(),
            "default security limits must not reject a normal multi-page document: {:?}",
            result.err()
        );
    }

    /// Proves the `PdfBackend::Pdfium` dispatch branch in `extract_core` (#702):
    /// selecting pdfium must fail with a specific, actionable error naming the
    /// backend and telling the caller what to do, rather than the native parser's
    /// generic "corrupt file" error. Content is deliberately not a valid PDF --
    /// dispatch must happen before any parsing, so a real pdfium rejection never
    /// reaches the parser at all.
    ///
    /// Fails against unfixed code: `extract_core` unconditionally calls
    /// `extract_core_native` and never reads `PdfConfig::backend`, so `content`
    /// gets handed straight to `xberg_native_pdf`. The native parser rejects the garbage
    /// bytes with an `XbergError::Parsing` whose message describes a corrupt/invalid
    /// PDF and never contains the word "pdfium" -- the
    /// `message.contains("pdfium")` assertion below fails.
    ///
    /// Gated on `not(feature = "pdf-pdfium")`: this exercises the *stub* rejection
    /// wording. With `pdf-pdfium` enabled the real engine runs instead (see
    /// `pdfium_engine`), whose error text this assertion does not describe.
    #[tokio::test]
    #[cfg(all(feature = "pdf", not(feature = "pdf-pdfium")))]
    async fn pdfium_backend_is_rejected_with_actionable_error_not_generic_parse_failure() {
        let extractor = PdfExtractor::new();
        let mut config = ExtractionConfig::default();
        config.pdf_options = Some(crate::core::config::PdfConfig {
            backend: crate::core::config::PdfBackend::Pdfium,
            ..Default::default()
        });

        let result = extractor
            .extract_content(b"not a real pdf", "application/pdf", &config)
            .await;

        let error = result.expect_err("selecting pdfium must fail, not silently extract via xberg_native_pdf");
        let message = error.to_string();
        assert!(
            message.contains("pdfium"),
            "error must name the requested backend 'pdfium', got: {message}"
        );
        assert!(
            message.contains("native"),
            "error must tell the caller what to do (use native instead), got: {message}"
        );
    }

    /// Proves there is no silent fallback (#702's core danger): given a document
    /// `xberg_native_pdf` can extract perfectly well, selecting `PdfBackend::Pdfium` must
    /// still fail rather than quietly returning `xberg_native_pdf`'s output mislabeled as
    /// pdfium. Uses a real, valid multi-page PDF specifically to rule out "it failed
    /// because the input was bad" as an explanation for the error.
    ///
    /// Fails against unfixed code: `extract_core` ignores `PdfConfig::backend`
    /// entirely and always calls `extract_core_native`, so `result.is_ok()` is `true`
    /// pre-fix (the exact silent-fallback failure mode #702 exists to close) instead
    /// of the `false` asserted here.
    ///
    /// Gated on `not(feature = "pdf-pdfium")`: with the real engine enabled, a valid
    /// document legitimately extracting via pdfium is the whole point (see
    /// `pdfium_engine_serializes_concurrent_extractions_when_the_library_is_available` below), not
    /// a silent fallback -- this test's premise (pdfium selection must always fail)
    /// only holds for the stub.
    #[tokio::test]
    #[cfg(all(feature = "pdf", not(feature = "pdf-pdfium")))]
    async fn pdfium_backend_never_silently_falls_back_to_xberg_native_pdf_output() {
        let pdf_path = pdf_test_document("multi_page.pdf");
        let Ok(content) = std::fs::read(&pdf_path) else {
            return;
        };

        let extractor = PdfExtractor::new();
        let mut config = ExtractionConfig::default();
        config.pdf_options = Some(crate::core::config::PdfConfig {
            backend: crate::core::config::PdfBackend::Pdfium,
            ..Default::default()
        });

        let result = extractor.extract_content(&content, "application/pdf", &config).await;
        assert!(
            result.is_err(),
            "pdfium selected on a document xberg_native_pdf could extract fine must still error, \
             not silently return xberg_native_pdf's output under the caller's pdfium selection"
        );
    }

    /// Proves the `pdf-pdfium` engine (#702) actually extracts text and metadata
    /// through pdfium rather than reusing `xberg_native_pdf` output. `multi_page.pdf` is a
    /// 5-page document whose first page's text is known ahead of time (verified
    /// with `pdftotext` against the fixture). Four requests run concurrently to
    /// regression-test serialization of Pdfium's process-global C API.
    ///
    /// Requires a real `libpdfium` on the system library search path (or
    /// `PDFIUM_DYNAMIC_LIB_PATH` pointing at one) -- this repository provisions
    /// neither today, so this test skips (rather than fails) when
    /// `pdfium_engine::bind_once` cannot load the library. That is an environment
    /// gap, not a code defect; asserting the error is specifically
    /// `XbergError::MissingDependency` (rather than string-matching the message)
    /// still proves the failure came from library loading and not from the stub
    /// rejection or some other error being swallowed.
    ///
    /// Fails against unfixed code: the stub `extract_core_pdfium` unconditionally
    /// returns `XbergError::Validation` (never attempting to load pdfium), so on a
    /// machine without `libpdfium` installed the `matches!(err,
    /// XbergError::MissingDependency(_))` assertion below fails -- the error variant
    /// is `Validation`, not `MissingDependency` -- and on a machine *with*
    /// `libpdfium` installed the stub still returns `Validation` unconditionally, so
    /// `result.is_ok()` never becomes `true` and the `.expect`-equivalent match's
    /// `Ok` arm is never reached; either way the real extraction assertions further
    /// down never run against unfixed code.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "pdf-pdfium"))]
    async fn pdfium_engine_serializes_concurrent_extractions_when_the_library_is_available() {
        // Skip-on-absence is right for a developer without the bucket-fetched corpus, but it must
        // never be how CI passes. `XBERG_REQUIRE_PDFIUM` (set by the `pdfium-engine` job) turns
        // every skip in this test into a failure, following the same truthy-env convention as
        // `XBERG_REQUIRE_MODELS` in tests/candle_backends.rs. Without it this test reports success
        // whenever libpdfium cannot be loaded -- which is exactly how the engine ended up with no
        // executed coverage at all. ~keep
        let require_pdfium = matches!(
            std::env::var("XBERG_REQUIRE_PDFIUM").as_deref(),
            Ok("1" | "true" | "yes")
        );

        let pdf_path = pdf_test_document("multi_page.pdf");
        let content = match std::fs::read(&pdf_path) {
            Ok(content) => content,
            Err(err) => {
                assert!(
                    !require_pdfium,
                    "XBERG_REQUIRE_PDFIUM is set but the fixture at {} could not be read ({err}); \
                     a fixture-fetch failure is a checkout bug, never a reason to report success",
                    pdf_path.display()
                );
                return;
            }
        };

        let extractor = PdfExtractor::new();
        let mut config = ExtractionConfig::default();
        config.pdf_options = Some(crate::core::config::PdfConfig {
            backend: crate::core::config::PdfBackend::Pdfium,
            ..Default::default()
        });

        let (first, second, third, fourth) = tokio::join!(
            extractor.extract_content(&content, "application/pdf", &config),
            extractor.extract_content(&content, "application/pdf", &config),
            extractor.extract_content(&content, "application/pdf", &config),
            extractor.extract_content(&content, "application/pdf", &config),
        );
        let results = [first, second, third, fourth];

        if let Some(err) = results.iter().find_map(|result| result.as_ref().err()) {
            if matches!(err, crate::XbergError::MissingDependency(_)) {
                assert!(
                    !require_pdfium,
                    "XBERG_REQUIRE_PDFIUM is set, so a real libpdfium was supposed to be loadable, \
                     but binding failed: {err:?}. Returning here would report success without ever \
                     exercising the engine."
                );
                return;
            }

            panic!("concurrent pdfium extraction failed: {err:?}");
        }

        for document in results.into_iter().flatten() {
            let result = crate::extraction::derive::derive_extraction_result(
                document,
                true,
                crate::core::config::OutputFormat::Plain,
            );

            assert!(
                result.content.contains("The Evolution of the Word Processor"),
                "expected the fixture's real first-page heading in pdfium output, got: {:?}",
                result.content
            );
            assert!(
                result.content.contains("Christopher Latham Sholes"),
                "expected real body text from the fixture in pdfium output, got: {:?}",
                result.content
            );

            let page_count = result.metadata.format.as_ref().and_then(|format| match format {
                crate::types::FormatMetadata::Pdf(pdf) => pdf.page_count,
                _ => None,
            });
            assert_eq!(page_count, Some(5), "multi_page.pdf has 5 pages");
        }
    }

    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn test_pdf_exposes_native_extraction_method() {
        let extractor = PdfExtractor::new();
        let config = ExtractionConfig::default();
        let pdf_path = pdf_test_document("google_doc_document.pdf");

        if let Ok(content) = std::fs::read(pdf_path) {
            let result = extractor
                .extract_content(&content, "application/pdf", &config)
                .await
                .expect("native PDF extraction should succeed");
            let result = crate::extraction::derive::derive_extraction_result(
                result,
                true,
                crate::core::config::OutputFormat::Plain,
            );

            assert_eq!(extraction_method(&result), Some(ExtractionMethod::Native));
        }
    }

    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_pdf_exposes_ocr_extraction_method() {
        use crate::core::config::OcrConfig;

        let _backend = register_mock_ocr_backend("pdf-extraction-method-ocr", "mock OCR text");
        let extractor = PdfExtractor::new();
        let config = ExtractionConfig {
            force_ocr: true,
            ocr: Some(OcrConfig {
                backend: "pdf-extraction-method-ocr".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        };
        let pdf_path = pdf_test_document("multi_page.pdf");

        if let Ok(content) = std::fs::read(pdf_path) {
            let result = extractor
                .extract_content(&content, "application/pdf", &config)
                .await
                .expect("forced OCR extraction should succeed");
            let result = crate::extraction::derive::derive_extraction_result(
                result,
                true,
                crate::core::config::OutputFormat::Plain,
            );

            assert_eq!(extraction_method(&result), Some(ExtractionMethod::Ocr));
        }
    }

    /// #1575: a backend warning and its `additional` metadata (psm/language) must survive the
    /// whole `PdfExtractor::extract_content` call, not just `extract_with_ocr_for_page`'s own
    /// return value. `doc.metadata` is fully replaced by a fresh `Metadata { .. }` literal
    /// partway through this extractor (see the `#1575` comment above that assignment) --
    /// without the take-then-restore fix there, this end-to-end test would see an empty
    /// `metadata.additional` even though the lower-level pipeline test already passes.
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    struct WarningMetadataPdfOcrBackend {
        name: &'static str,
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    impl crate::plugins::Plugin for WarningMetadataPdfOcrBackend {
        fn name(&self) -> &str {
            self.name
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> crate::Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> crate::Result<()> {
            Ok(())
        }
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[async_trait::async_trait]
    impl crate::plugins::OcrBackend for WarningMetadataPdfOcrBackend {
        fn backend_type(&self) -> crate::plugins::OcrBackendType {
            crate::plugins::OcrBackendType::Custom
        }
        fn supports_language(&self, _lang: &str) -> bool {
            true
        }
        async fn process_image(
            &self,
            _image_bytes: &[u8],
            _config: &crate::core::config::OcrConfig,
        ) -> crate::Result<crate::types::ExtractedDocument> {
            let mut additional = ahash::AHashMap::new();
            additional.insert(std::borrow::Cow::Borrowed("psm"), serde_json::json!("11"));
            additional.insert(std::borrow::Cow::Borrowed("language"), serde_json::json!("eng"));
            Ok(crate::types::ExtractedDocument {
                content: "mock OCR text".to_string(),
                metadata: crate::types::Metadata {
                    additional,
                    ..Default::default()
                },
                processing_warnings: vec![crate::types::ProcessingWarning {
                    source: std::borrow::Cow::Borrowed("tesseract"),
                    message: std::borrow::Cow::Borrowed(
                        "Tesseract removed 1 OCR line(s) because their dictionary-check ratio exceeded 0.60.",
                    ),
                }],
                ..Default::default()
            })
        }
    }

    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_pdf_ocr_route_surfaces_backend_warning_and_metadata() {
        const BACKEND_NAME: &str = "pdf-warning-metadata-ocr";
        crate::plugins::register_ocr_backend(std::sync::Arc::new(WarningMetadataPdfOcrBackend { name: BACKEND_NAME }))
            .unwrap();
        let _guard = RegisteredOcrBackendGuard { name: BACKEND_NAME };

        let extractor = PdfExtractor::new();
        let config = ExtractionConfig {
            force_ocr: true,
            ocr: Some(crate::core::config::OcrConfig {
                backend: BACKEND_NAME.to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        };
        let pdf_path = pdf_test_document("multi_page.pdf");
        let content = std::fs::read(pdf_path).expect("fixture must be present");

        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("forced OCR extraction should succeed");

        assert_eq!(
            result.processing_warnings.len(),
            1,
            "expected exactly the backend's own warning: {:?}",
            result.processing_warnings
        );
        assert_eq!(result.processing_warnings[0].source, "tesseract");
        assert_eq!(
            result.processing_warnings[0].message,
            "Tesseract removed 1 OCR line(s) because their dictionary-check ratio exceeded 0.60."
        );
        assert_eq!(
            result.metadata.additional.get("psm"),
            Some(&serde_json::json!("11")),
            "psm must survive the `doc.metadata = Metadata {{ .. }}` reassignment: {:?}",
            result.metadata.additional
        );
        assert_eq!(
            result.metadata.additional.get("language"),
            Some(&serde_json::json!("eng")),
            "language must survive the `doc.metadata = Metadata {{ .. }}` reassignment: {:?}",
            result.metadata.additional
        );
    }

    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    /// #1576: any `ImageExtractionConfig` -- default fields included -- must not suppress
    /// document-level page OCR on a scanned PDF. Before the fix, `ImageExtractionConfig::
    /// default()` (`run_ocr_on_images: true`) left `pages[0].content` empty because the
    /// `RunFallback` gate read `run_ocr_on_images` as "skip page OCR", even though that field
    /// only controls whether separately extracted images get their own OCR pass. ~keep
    async fn images_config_does_not_suppress_scanned_page_ocr() {
        use crate::core::config::{ImageExtractionConfig, OcrConfig, PageConfig};

        const BACKEND_NAME: &str = "pdf-1576-images-config-does-not-suppress-ocr";
        const RECOVERED_TEXT: &str = "recovered scan text";
        let _backend = register_mock_ocr_backend(BACKEND_NAME, RECOVERED_TEXT);
        // A single blank page (no `Contents` text) has empty native text, which the OCR
        // fallback decision (`ocr::scoring::OcrFallbackDecision`) always flags as a whole-
        // document failure -- deterministically exercising `OcrGateOutcome::RunFallback`,
        // unlike `force_ocr` which bypasses this gate entirely (see the comment on
        // `failed_ocr_fallback_is_total_loss` above).
        let content = blank_letter_page_pdf();

        let base_config = ExtractionConfig {
            use_cache: false,
            ocr: Some(OcrConfig {
                backend: BACKEND_NAME.to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                insert_page_markers: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        for (label, images) in [
            ("no images config", None),
            (
                "ImageExtractionConfig::default()",
                Some(ImageExtractionConfig::default()),
            ),
        ] {
            let config = ExtractionConfig {
                images,
                ..base_config.clone()
            };
            let internal = PdfExtractor::new()
                .extract_content(&content, "application/pdf", &config)
                .await
                .unwrap_or_else(|e| panic!("{label}: scanned-page OCR extraction should succeed: {e}"));
            let result = crate::extraction::derive::derive_extraction_result(
                internal,
                true,
                crate::core::config::OutputFormat::Plain,
            );
            let pages = result
                .pages
                .as_ref()
                .unwrap_or_else(|| panic!("{label}: pages must be present"));
            assert!(!pages.is_empty(), "{label}: at least one page must be present");
            assert!(
                pages[0].content.contains(RECOVERED_TEXT),
                "{label}: pages[0].content must carry the OCR text, got {:?}",
                pages[0].content
            );
            assert_eq!(
                extraction_method(&result),
                Some(ExtractionMethod::Ocr),
                "{label}: a fully scanned page must report ExtractionMethod::Ocr"
            );
        }
    }

    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn should_report_native_when_forced_page_ocr_is_rejected() {
        use crate::core::config::{OcrConfig, PageConfig};

        let _backend = register_mock_ocr_backend("pdf-rejected-mixed-method", "a");
        let content = std::fs::read(pdf_test_document("multi_page.pdf"))
            .expect("mixed extraction-method fixture must be available");
        let config = ExtractionConfig {
            force_ocr_pages: Some(vec![1]),
            use_cache: false,
            ocr: Some(OcrConfig {
                backend: "pdf-rejected-mixed-method".to_string(),
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("rejected page OCR should preserve the native result");
        let result = crate::extraction::derive::derive_extraction_result(
            internal,
            true,
            crate::core::config::OutputFormat::Plain,
        );

        assert_eq!(extraction_method(&result), Some(ExtractionMethod::Native));
        assert!(!result.metadata.ocr_used);
    }

    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr", feature = "chunking"))]
    #[serial]
    async fn test_mixed_pdf_ocr_survives_full_postprocessing() {
        use crate::core::config::{ChunkingConfig, OcrConfig, PageConfig};

        // Must recover at least MIN_OCR_NATIVE_ALNUM_RETENTION_RATIO (0.5) of page one's 1897
        // alphanumeric characters, or `accepted_ocr_page_replacements` vetoes the replacement as
        // destructive (`ocr/scoring.rs`, guard added by 10680fca4e6) and the page silently stays
        // native. This text carries 1151, a ratio of 0.61. A short marker string cannot exercise
        // this test's subject at all -- it is rejected before any substitution happens. ~keep
        const OCR_TEXT: &str = "Issue 1281 authoritative OCR replacement on page one. The Evolution of the Word \
            Processor. The concept of the word processor predates modern computers and has evolved through several \
            technological milestones. Pre-Digital Era, nineteenth to early twentieth century. The origins of word \
            processing can be traced back to the invention of the typewriter in the mid nineteenth century. Patented \
            in 1868 by Christopher Latham Sholes, the typewriter revolutionized written communication. It enabled \
            people to produce legible, professional documents far more efficiently than handwriting ever allowed. \
            During this period the term word processing did not yet exist, but the typewriter laid the groundwork. \
            Later advances such as carbon paper for duplicates and the electric typewriter introduced by IBM in 1935 \
            improved speed. Together these refinements steadily increased the convenience and reliability of everyday \
            document creation. Mechanical composition gave way to electronic storage as magnetic media made revision \
            practical for the first time. Dedicated word processing machines briefly dominated offices before general \
            purpose computers absorbed the role. By the late twentieth century the printed page had become an artifact \
            of software rather than of mechanism. That transition is the through line connecting every milestone \
            described on the remainder of this page.";
        const RETAINED_PAGE_TWO_MARKER: &str = "Other notable software from this era included WordPerfect";
        let _backend = register_mock_ocr_backend("pdf-extraction-method-mixed", OCR_TEXT);
        let extractor = PdfExtractor::new();
        let pdf_path = pdf_test_document("multi_page.pdf");
        let content = std::fs::read(pdf_path).expect("mixed OCR fixture must be available");
        let native_config = ExtractionConfig {
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        let native = extractor
            .extract_content(&content, "application/pdf", &native_config)
            .await
            .expect("native PDF extraction should succeed");
        let native =
            crate::extraction::derive::derive_extraction_result(native, true, crate::core::config::OutputFormat::Plain);
        let native_pages = native.pages.expect("native extraction must expose pages");
        let stale_page_one = native_pages[0].content.clone();

        let config = ExtractionConfig {
            force_ocr_pages: Some(vec![1]),
            output_format: crate::core::config::OutputFormat::Markdown,
            include_document_structure: true,
            use_cache: false,
            ocr: Some(OcrConfig {
                backend: "pdf-extraction-method-mixed".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            chunking: Some(ChunkingConfig {
                max_characters: OCR_TEXT.chars().count() + 1,
                overlap: 0,
                ..Default::default()
            }),
            ..Default::default()
        };
        let internal = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("mixed OCR/native extraction should succeed");
        let derived = crate::extraction::derive::derive_extraction_result(
            internal.clone(),
            true,
            crate::core::config::OutputFormat::Markdown,
        );
        // Substitution happens inside `extract_content`, not in `run_pipeline`, so the OCR text is
        // already in place here; the assertions after `run_pipeline` cover the later stages
        // (chunking, document structure) rather than the substitution itself. The old form of this
        // check sliced `derived.content` with `metadata.pages.boundaries`, which index the raw
        // concatenated text and not the rendered string -- see `core::pipeline::features`. ~keep
        let derived_pages = derived.pages.as_ref().expect("derived extraction must expose pages");
        assert!(
            derived_pages[0].content.contains(OCR_TEXT),
            "page one must carry the accepted OCR replacement"
        );
        assert!(
            derived_pages[1].content.contains(RETAINED_PAGE_TWO_MARKER),
            "page-two derived content must retain native text"
        );
        assert_occurs_once(&derived.content, OCR_TEXT, "derived plain content");
        assert_occurs_once(
            derived
                .formatted_content
                .as_deref()
                .expect("derived Markdown output must exist"),
            OCR_TEXT,
            "derived Markdown output",
        );
        let result = crate::core::pipeline::run_pipeline(internal, &config)
            .await
            .expect("mixed OCR post-processing should succeed");

        assert_eq!(extraction_method(&result), Some(ExtractionMethod::Mixed));
        assert_occurs_once(&result.content, OCR_TEXT, "post-processed Markdown content");
        assert!(!result.content.contains(&stale_page_one));
        assert!(result.content.contains(RETAINED_PAGE_TWO_MARKER));

        let pages = result.pages.as_ref().expect("mixed extraction must expose pages");
        assert_occurs_once(&pages[0].content, OCR_TEXT, "page one");
        assert!(!pages[0].content.contains(&stale_page_one));
        assert_occurs_once(&pages[1].content, RETAINED_PAGE_TWO_MARKER, "retained page two");
        assert!(!pages[1].content.contains(OCR_TEXT));

        let document = result.document.as_ref().expect("document structure must be derived");
        let ocr_nodes: Vec<_> = document
            .nodes
            .iter()
            .filter(|node| node.content.text().is_some_and(|text| text.contains(OCR_TEXT)))
            .collect();
        assert_eq!(ocr_nodes.len(), 1, "document structure must not duplicate OCR text");
        assert_eq!(ocr_nodes[0].page, Some(1));
        assert!(
            !document
                .nodes
                .iter()
                .any(|node| node.content.text().is_some_and(|text| text.contains(&stale_page_one)))
        );

        let chunks = result.chunks.as_ref().expect("chunking must run");
        let ocr_chunks: Vec<_> = chunks.iter().filter(|chunk| chunk.content.contains(OCR_TEXT)).collect();
        assert_eq!(ocr_chunks.len(), 1, "chunks must not duplicate OCR text");
        assert_eq!(ocr_chunks[0].metadata.first_page, Some(1));
        assert_eq!(ocr_chunks[0].metadata.last_page, Some(1));
    }

    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr", feature = "chunking"))]
    #[serial]
    async fn test_scanned_page_strategy_automatically_routes_only_the_scan_to_ocr() {
        use crate::core::config::{ChunkingConfig, OcrConfig, OcrStrategy, PageConfig};

        const NATIVE_TEXT: &str = "Issue 1281 native text remains on page one";
        const OCR_TEXT: &str = "Issue 1281 automatic OCR replacement on page two.";
        let _backend = register_mock_ocr_backend("pdf-automatic-mixed-routing", OCR_TEXT);
        let config = ExtractionConfig {
            ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
            output_format: crate::core::config::OutputFormat::Markdown,
            include_document_structure: true,
            use_cache: false,
            ocr: Some(OcrConfig {
                backend: "pdf-automatic-mixed-routing".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            chunking: Some(ChunkingConfig {
                max_characters: OCR_TEXT.chars().count() + 1,
                overlap: 0,
                ..Default::default()
            }),
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(&mixed_native_and_scanned_pdf(), "application/pdf", &config)
            .await
            .expect("automatic mixed PDF extraction should succeed");
        let derived = crate::extraction::derive::derive_extraction_result(
            internal.clone(),
            true,
            crate::core::config::OutputFormat::Markdown,
        );
        assert_occurs_once(&derived.content, OCR_TEXT, "automatic derived content");
        assert_occurs_once(
            derived
                .formatted_content
                .as_deref()
                .expect("automatic derived Markdown output must exist"),
            OCR_TEXT,
            "automatic derived Markdown output",
        );
        let result = crate::core::pipeline::run_pipeline(internal, &config)
            .await
            .expect("automatic mixed PDF post-processing should succeed");

        assert_eq!(extraction_method(&result), Some(ExtractionMethod::Mixed));
        assert_occurs_once(&result.content, NATIVE_TEXT, "automatic mixed Markdown content");
        assert_occurs_once(&result.content, OCR_TEXT, "automatic mixed Markdown content");
        let pages = result
            .pages
            .as_ref()
            .expect("automatic mixed extraction must expose pages");
        assert_eq!(pages.len(), 2);
        assert!(pages[0].content.contains(NATIVE_TEXT));
        assert!(!pages[0].content.contains(OCR_TEXT));
        assert_occurs_once(&pages[1].content, OCR_TEXT, "automatically OCR'd page two");
        assert!(!pages[1].content.contains(NATIVE_TEXT));

        let document = result.document.as_ref().expect("document structure must be derived");
        let ocr_nodes: Vec<_> = document
            .nodes
            .iter()
            .filter(|node| node.content.text().is_some_and(|text| text.contains(OCR_TEXT)))
            .collect();
        assert_eq!(ocr_nodes.len(), 1);
        assert_eq!(ocr_nodes[0].page, Some(2));

        let chunks = result.chunks.as_ref().expect("automatic mixed chunking must run");
        let ocr_chunks: Vec<_> = chunks.iter().filter(|chunk| chunk.content.contains(OCR_TEXT)).collect();
        assert_eq!(ocr_chunks.len(), 1, "automatic chunks must not duplicate OCR text");
        assert_eq!(ocr_chunks[0].metadata.first_page, Some(2));
        assert_eq!(ocr_chunks[0].metadata.last_page, Some(2));
    }

    /// GH#2064: when the automatic OCR of the detected scanned pages fails as a whole, each
    /// selected page gets a record beside the route-level warning.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn route_level_failure_records_each_scanned_page() {
        use crate::core::config::{OcrConfig, OcrStrategy};

        const NATIVE_TEXT: &str = "Issue 1281 native text remains on page one";
        let config = ExtractionConfig {
            ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
            use_cache: false,
            ocr: Some(OcrConfig {
                backend: "pdf-2064-backend-that-is-not-registered".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(&mixed_native_and_scanned_pdf(), "application/pdf", &config)
            .await
            .expect("a failed automatic OCR run must keep the native text");
        let result = crate::extraction::derive::derive_extraction_result(
            internal,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        assert!(
            result.content.contains(NATIVE_TEXT),
            "the native page must survive: {:?}",
            result.content
        );
        let route_warning = result
            .processing_warnings
            .iter()
            .find(|warning| {
                warning
                    .message
                    .starts_with("Automatic OCR of detected scanned pages [2] failed (")
            })
            .unwrap_or_else(|| {
                panic!(
                    "the route-level warning must be present: {:?}",
                    result.processing_warnings
                )
            });
        let failures = result.ocr_page_failures.as_deref().unwrap_or_default();
        assert_eq!(failures.len(), 1, "only the scanned page is recorded: {failures:?}");
        let failure = &failures[0];
        assert_eq!(failure.page, 2);
        assert!(!failure.recovered, "the scanned page has no native text to keep");
        assert_eq!(
            route_warning.message,
            format!(
                "Automatic OCR of detected scanned pages [2] failed ({}); those pages retain their native text, \
                 which may be empty or incomplete.",
                failure.error
            ),
            "the record carries the same error as the warning"
        );
    }

    /// GH#2064: the same records when the targeted fallback of the default `Auto` strategy fails
    /// as a whole.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn targeted_fallback_failure_records_each_selected_page() {
        use crate::core::config::OcrConfig;

        const NATIVE_TEXT: &str = "Issue 1281 native text remains on page one";
        let config = ExtractionConfig {
            use_cache: false,
            ocr: Some(OcrConfig {
                backend: "pdf-2064-backend-that-is-not-registered".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(&mixed_native_and_scanned_pdf(), "application/pdf", &config)
            .await
            .expect("a failed targeted OCR fallback must keep the native text");
        let result = crate::extraction::derive::derive_extraction_result(
            internal,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        assert!(
            result.content.contains(NATIVE_TEXT),
            "the native page must survive: {:?}",
            result.content
        );
        let route_warning = result
            .processing_warnings
            .iter()
            .find(|warning| warning.message.starts_with("Targeted OCR fallback failed ("))
            .unwrap_or_else(|| {
                panic!(
                    "the route-level warning must be present: {:?}",
                    result.processing_warnings
                )
            });
        let failures = result.ocr_page_failures.as_deref().unwrap_or_default();
        assert_eq!(failures.len(), 1, "only the selected page is recorded: {failures:?}");
        let failure = &failures[0];
        assert_eq!(failure.page, 2);
        assert!(!failure.recovered, "the selected page has no native text to keep");
        assert_eq!(
            route_warning.message,
            format!(
                "Targeted OCR fallback failed ({}) for pages [2]; those pages retain native text that was below the \
                 OCR-trigger quality threshold and may be empty or incomplete.",
                failure.error
            ),
            "the record carries the same error as the warning"
        );
    }

    /// xberg#1667: a native text layer produced by a fabricated character mapping
    /// (`MappingProvenance::Fallback`) reads as ordinary, structurally-clean ASCII prose, so
    /// `NativeTextStats`' character-class checks pass it at `quality_score: 1.0` and OCR never
    /// fires -- under the DEFAULT `OcrStrategy::Auto`, not the opt-in `ScannedPages` strategy the
    /// existing scanned-page-routing tests use. The fix reads the fabricated-provenance signal
    /// (already computed for issue #1254) from the `Auto` gate too, so this page must route to
    /// OCR even though every character-shape heuristic would call it clean.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_auto_strategy_routes_fabricated_provenance_page_to_ocr() {
        use crate::core::config::{OcrConfig, PageConfig};

        const OCR_TEXT: &str = "issue sixteen sixty seven automatic ocr replacement text";
        let _backend = register_mock_ocr_backend("pdf-1667-auto-provenance-routing", OCR_TEXT);

        // Echoes back as ordinary ASCII prose (CID == Unicode codepoint under the
        // CID-as-Unicode fallback), well past every `OcrQualityThresholds` default: > 64
        // non-whitespace chars, several words at or above the 4-char "meaningful" length, no
        // fragmentation, no repeats. A clean-looking page, on paper.
        const FABRICATED_NATIVE_TEXT: &str = "synthetic fabricated text used only to confirm that automatic routing reaches the configured ocr backend correctly";

        let config = ExtractionConfig {
            // `OcrStrategy::Auto` is `#[default]`; left unset deliberately, this is the path
            // every caller who does not opt into `ScannedPages` takes.
            ocr: Some(OcrConfig {
                backend: "pdf-1667-auto-provenance-routing".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            use_cache: false,
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(
                &identity_h_mapping_pdf(FABRICATED_NATIVE_TEXT, false),
                "application/pdf",
                &config,
            )
            .await
            .expect("fabricated-provenance PDF extraction should succeed");
        let derived = crate::extraction::derive::derive_extraction_result(
            internal,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        assert!(
            !derived.content.contains(FABRICATED_NATIVE_TEXT),
            "the fabricated native text must not survive to the final content: {:?}",
            derived.content
        );
        assert_eq!(
            derived.content.matches(OCR_TEXT).count(),
            1,
            "Auto-routed OCR content must occur exactly once: {:?}",
            derived.content
        );
        assert_eq!(
            derived.extraction_method,
            Some(ExtractionMethod::Ocr),
            "Auto strategy must record extraction_method: ocr for a fabricated-provenance page"
        );
    }

    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn fabricated_mapping_page_records_effective_block_psm_in_page_metadata() {
        use crate::core::config::{OcrConfig, PageConfig};

        const FABRICATED_NATIVE_TEXT: &str = "synthetic fabricated text used to verify the per page segmentation mode recorded after automatic ocr routing";
        let config = ExtractionConfig {
            ocr: Some(OcrConfig::default()),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            use_cache: false,
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(
                &identity_h_mapping_pdf(FABRICATED_NATIVE_TEXT, false),
                "application/pdf",
                &config,
            )
            .await
            .expect("fabricated-provenance PDF extraction should succeed");
        assert_eq!(
            internal
                .metadata
                .additional
                .get(crate::ocr_metadata_keys::OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY),
            Some(&serde_json::json!([{ "page_number": 1, "psm": 6 }]))
        );
    }

    /// `identity_h_mapping_pdf`'s unmapped text drawn invisibly over an inset scan: a text
    /// raster covering half the page, like a scanned sheet painted with margins, under an
    /// unreadable text layer.
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    fn inset_scan_with_unmapped_text_layer_pdf(text: &str) -> Vec<u8> {
        use lopdf::{Document, Object, Stream, dictionary};

        let mut document =
            Document::load_mem(&identity_h_mapping_pdf(text, false)).expect("fabricated-provenance fixture must load");
        let page_id = *document.get_pages().get(&1).expect("fixture has one page");
        let scan = image::load_from_memory(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/ocr/lone_quantity_invoice.png"
        )))
        .expect("scan fixture must decode")
        .resize(1275, 1650, image::imageops::FilterType::Triangle)
        .to_luma8();
        let (width, height) = scan.dimensions();
        let image_id = document.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => width,
                "Height" => height,
                "ColorSpace" => "DeviceGray",
                "BitsPerComponent" => 8,
            },
            scan.into_raw(),
        ));

        // 432 x 559 pt on a Letter page is half its area: an inset scan, not a figure.
        let mut content = b"q 432 0 0 559 90 116 cm /Scan Do Q\n3 Tr\n".to_vec();
        content.extend(document.get_page_content(page_id));
        document
            .change_page_content(page_id, content)
            .expect("replace fixture page content");
        document
            .get_object_mut(page_id)
            .and_then(Object::as_dict_mut)
            .expect("page dictionary")
            .get_mut(b"Resources")
            .and_then(Object::as_dict_mut)
            .expect("page resources")
            .set("XObject", dictionary! { "Scan" => image_id });

        let mut bytes = Vec::new();
        document.save_to(&mut bytes).expect("fixture PDF must serialize");
        bytes
    }

    /// Block mode is for a page whose unmapped text is what OCR reads. A scan whose text layer
    /// is unmapped is still a scan, and keeps the segmentation mode a scan gets: block mode
    /// loses the table reconstruction on a scanned table.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn scanned_page_with_unmapped_text_layer_keeps_the_scan_segmentation_mode() {
        use crate::core::config::{OcrConfig, PageConfig};

        const FABRICATED_NATIVE_TEXT: &str = "synthetic fabricated text layer drawn invisibly over an inset scan so the page is routed by its unmapped characters";
        let config = ExtractionConfig {
            ocr: Some(OcrConfig::default()),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            use_cache: false,
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(
                &inset_scan_with_unmapped_text_layer_pdf(FABRICATED_NATIVE_TEXT),
                "application/pdf",
                &config,
            )
            .await
            .expect("inset scan with an unmapped text layer should extract");
        let fabricated_text_pages = internal.metadata.format.as_ref().and_then(|format| match format {
            crate::types::FormatMetadata::Pdf(pdf) => pdf.fabricated_text_pages.clone(),
            _ => None,
        });
        assert_eq!(
            fabricated_text_pages,
            Some(vec![1]),
            "the fixture must be routed by its unmapped text layer"
        );
        let entries = internal
            .metadata
            .additional
            .get(crate::ocr_metadata_keys::OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY)
            .and_then(serde_json::Value::as_array)
            .expect("the OCR'd page must record its effective segmentation mode");

        assert_eq!(entries.len(), 1, "one OCR'd page: {entries:?}");
        assert_eq!(entries[0]["page_number"], serde_json::json!(1));
        assert_ne!(
            entries[0]["psm"],
            serde_json::json!(6),
            "a scan must not take block mode because its text layer is unmapped"
        );
    }

    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn explicit_force_ocr_does_not_apply_the_automatic_block_mode() {
        use crate::core::config::{OcrConfig, PageConfig};

        const FABRICATED_NATIVE_TEXT: &str = "synthetic fabricated text used to verify explicit whole document ocr retains the caller controlled segmentation mode";
        let config = ExtractionConfig {
            force_ocr: true,
            ocr: Some(OcrConfig::default()),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            use_cache: false,
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(
                &identity_h_mapping_pdf(FABRICATED_NATIVE_TEXT, false),
                "application/pdf",
                &config,
            )
            .await
            .expect("explicit whole-document OCR should succeed");
        let entries = internal
            .metadata
            .additional
            .get(crate::ocr_metadata_keys::OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY)
            .and_then(serde_json::Value::as_array);

        assert!(entries.is_none_or(|entries| entries.iter().all(|entry| entry["psm"] != serde_json::json!(6))));
    }

    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn explicit_force_ocr_pages_does_not_apply_the_automatic_block_mode() {
        use crate::core::config::{OcrConfig, PageConfig};

        const FABRICATED_NATIVE_TEXT: &str = "synthetic fabricated text used to verify explicit selected page ocr retains the caller controlled segmentation mode";
        let config = ExtractionConfig {
            force_ocr_pages: Some(vec![1]),
            ocr: Some(OcrConfig::default()),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            use_cache: false,
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(
                &identity_h_mapping_pdf(FABRICATED_NATIVE_TEXT, false),
                "application/pdf",
                &config,
            )
            .await
            .expect("explicit selected-page OCR should succeed");
        let entries = internal
            .metadata
            .additional
            .get(crate::ocr_metadata_keys::OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY)
            .and_then(serde_json::Value::as_array);

        assert!(entries.is_none_or(|entries| entries.iter().all(|entry| entry["psm"] != serde_json::json!(6))));
    }

    /// xberg#1696's false-positive control: a Type0/Identity-H font that DOES carry a
    /// `/ToUnicode` CMap resolves `MappingProvenance::ToUnicode`, never `Fallback`, so
    /// `apply_flagged_pages` (issue #1667's fix) must not route it to OCR --
    /// the fix targets fonts with no usable mapping tier at all, not every Type0 font.
    /// Reported alongside #1667's own test since both extract the same fixture shape through
    /// the real `run.rs`/`font_dict.rs` provenance path, differing only in `/ToUnicode`
    /// presence.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_auto_strategy_keeps_native_text_when_page_carries_tounicode_map() {
        use crate::core::config::{OcrConfig, PageConfig};

        const OCR_TEXT: &str = "mock ocr text that must never appear in a clean extraction";
        let _backend = register_mock_ocr_backend("pdf-1696-tounicode-false-positive", OCR_TEXT);

        // > 64 non-whitespace chars (`OcrQualityThresholds::min_total_non_whitespace`'s
        // default): a shorter fixture never reaches the fabricated-ratio check at all,
        // which would make the mutation-testing control below pass for the wrong reason. ~keep
        const NATIVE_TEXT: &str = "genuinely mapped native text read from the ToUnicode character map identity range covering printable ascii glyphs correctly";

        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "pdf-1696-tounicode-false-positive".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            use_cache: false,
            ..Default::default()
        };

        let internal = PdfExtractor::new()
            .extract_content(&identity_h_mapping_pdf(NATIVE_TEXT, true), "application/pdf", &config)
            .await
            .expect("ToUnicode-mapped PDF extraction should succeed");
        let derived = crate::extraction::derive::derive_extraction_result(
            internal,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        assert!(
            derived.content.contains(NATIVE_TEXT),
            "genuinely mapped native text must survive to the final content: {:?}",
            derived.content
        );
        assert!(
            !derived.content.contains(OCR_TEXT),
            "a page with a real ToUnicode mapping must not be routed to OCR: {:?}",
            derived.content
        );
        assert_eq!(
            derived.extraction_method,
            Some(ExtractionMethod::Native),
            "Auto strategy must keep extraction_method: native for a genuinely mapped page"
        );

        let fabricated_text_pages = derived.metadata.format.as_ref().and_then(|format| match format {
            crate::types::FormatMetadata::Pdf(pdf) => pdf.fabricated_text_pages.clone(),
            _ => None,
        });
        assert_eq!(
            fabricated_text_pages,
            Some(Vec::new()),
            "a page whose mapping resolves via ToUnicode must not be listed as fabricated"
        );
    }

    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    async fn test_pdf_force_ocr_without_ocr_config() {
        use crate::core::config::ExtractionConfig;

        let extractor = PdfExtractor::new();

        let config = ExtractionConfig {
            force_ocr: true,
            ocr: None,
            ..Default::default()
        };

        let pdf_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/multi_page.pdf");
        if let Ok(content) = std::fs::read(pdf_path) {
            let result = extractor.extract_content(&content, "application/pdf", &config).await;

            if let Err(e) = result {
                assert!(
                    !e.to_string().contains("OCR config required for force_ocr"),
                    "Should not require manual OCR config when force_ocr is true"
                );
            }
        }
    }

    /// Verifies that per-page OCR text segments correctly override native page
    /// content in each `PageContent` entry (#928).
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[tokio::test]
    async fn test_ocr_page_texts_override_native_page_content() {
        use crate::core::config::OcrConfig;
        use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
        use crate::types::ExtractedDocument;
        use std::sync::Arc;

        struct PerPageMockBackend;

        #[async_trait::async_trait]
        impl OcrBackend for PerPageMockBackend {
            fn backend_type(&self) -> OcrBackendType {
                OcrBackendType::Custom
            }
            fn supports_language(&self, _: &str) -> bool {
                true
            }
            async fn process_image(&self, data: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
                let width = image::load_from_memory(data)
                    .expect("mock input must be an image")
                    .width();
                let target_dpi = if width == 1 { 150 } else { 300 };
                Ok(ExtractedDocument {
                    content: format!("ocr-page-{width}"),
                    metadata: crate::types::Metadata {
                        image_preprocessing: Some(crate::types::ImagePreprocessingMetadata {
                            original_dimensions: (100, 200).into(),
                            original_dpi: (72.0, 72.0).into(),
                            target_dpi,
                            scale_factor: 1.0,
                            auto_adjusted: false,
                            final_dpi: target_dpi,
                            new_dimensions: None,
                            resample_method: "LANCZOS3".to_string(),
                            dimension_clamped: false,
                            calculated_dpi: None,
                            skipped_resize: true,
                            resize_error: None,
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                })
            }
            fn supports_document_processing(&self) -> bool {
                false
            }
        }

        impl Plugin for PerPageMockBackend {
            fn name(&self) -> &str {
                "per-page-ocr-mock-928"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> crate::Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> crate::Result<()> {
                Ok(())
            }
        }

        crate::plugins::register_ocr_backend(Arc::new(PerPageMockBackend)).unwrap();

        use image::ImageEncoder as _;
        let make_png = |width| {
            let img = image::DynamicImage::new_rgb8(width, 1);
            let rgb = img.to_rgb8();
            let (w, h) = rgb.dimensions();
            let mut buf = std::io::Cursor::new(Vec::new());
            image::codecs::png::PngEncoder::new(&mut buf)
                .write_image(&rgb, w, h, image::ColorType::Rgb8.into())
                .unwrap();
            image::load_from_memory(&buf.into_inner()).unwrap()
        };
        let images = vec![make_png(1), make_png(2)];

        let config = crate::core::config::ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "per-page-ocr-mock-928".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = super::ocr::extract_with_ocr(
            None,
            Some(&images),
            #[cfg(feature = "layout-detection")]
            None,
            &config,
            None,
        )
        .await;

        crate::plugins::unregister_ocr_backend("per-page-ocr-mock-928").unwrap();

        let (_text, _conf, _tables, _elems, _doc, _llm, page_texts, _rasters, _formulas, preprocessing, _) =
            result.expect("extract_with_ocr should succeed");

        assert_eq!(page_texts.len(), 2, "expected one entry per page");
        assert!(page_texts[0].starts_with("ocr-page-"), "page 0 should have OCR text");
        assert!(page_texts[1].starts_with("ocr-page-"), "page 1 should have OCR text");
        assert_ne!(page_texts[0], page_texts[1], "each page should get unique OCR text");
        let mut target_dpis = preprocessing
            .into_iter()
            .map(|(page, metadata)| (page, metadata.target_dpi))
            .collect::<Vec<_>>();
        target_dpis.sort_unstable();
        assert_eq!(target_dpis, vec![(1, 150), (2, 300)]);
    }

    /// Verifies that when a VLM returns a single string for a multi-page PDF,
    /// the guard clears stale native text on secondary pages (#928).
    #[cfg(feature = "ocr")]
    #[test]
    fn test_vlm_single_string_guard_clears_secondary_pages() {
        use crate::types::PageContent;

        let vlm_text = "whole-doc VLM summary".to_string();
        let pts = vec![vlm_text.clone()];

        let mut pages: Vec<PageContent> = (1u32..=3u32)
            .map(|n| PageContent {
                page_number: n,
                content: format!("native page {n}"),
                tables: Vec::new(),
                image_indices: Vec::new(),
                image_preprocessing: None,
                hierarchy: None,
                is_blank: None,
                layout_regions: None,
                speaker_notes: None,
                section_name: None,
                sheet_name: None,
                ocr_confidence: None,
                native_content: None,
            })
            .collect();

        replace_pages_with_ocr_page_texts(&mut pages, &pts, false);

        assert_eq!(pages[0].content, vlm_text, "page 1 should carry the VLM text");
        assert!(pages[1].content.is_empty(), "page 2 must be cleared by VLM guard");
        assert!(pages[2].content.is_empty(), "page 3 must be cleared by VLM guard");
        assert_eq!(
            pages[0].is_blank,
            Some(false),
            "page 1 has VLM content so must not be blank"
        );
        assert_eq!(pages[1].is_blank, Some(true), "page 2 was cleared so must be blank");
        assert_eq!(pages[2].is_blank, Some(true), "page 3 was cleared so must be blank");
    }

    #[cfg(feature = "ocr")]
    fn text_layer_page(page_number: u32, content: &str) -> crate::types::PageContent {
        crate::types::PageContent {
            page_number,
            content: content.to_string(),
            tables: Vec::new(),
            image_indices: Vec::new(),
            image_preprocessing: None,
            hierarchy: None,
            is_blank: None,
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
            ocr_confidence: None,
            native_content: None,
        }
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn ocr_text_replaces_the_text_layer_and_keeps_it_only_when_asked() {
        let mut kept = text_layer_page(1, "Ref KX-204");
        replace_page_content_with_ocr_text(&mut kept, "Ref KX-2O4", true);
        assert_eq!(kept.content, "Ref KX-2O4");
        assert_eq!(kept.native_content.as_deref(), Some("Ref KX-204"));
        assert_eq!(kept.is_blank, Some(false));

        let mut dropped = text_layer_page(1, "Ref KX-204");
        replace_page_content_with_ocr_text(&mut dropped, "Ref KX-2O4", false);
        assert_eq!(dropped.content, "Ref KX-2O4");
        assert_eq!(dropped.native_content, None);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn ocr_text_equal_to_the_text_layer_is_not_stored_twice() {
        let mut same = text_layer_page(1, "Ref KX-204");
        replace_page_content_with_ocr_text(&mut same, "Ref KX-204", true);
        assert_eq!(same.content, "Ref KX-204");
        assert_eq!(same.native_content, None);

        let mut different = text_layer_page(1, "Ref KX-204");
        replace_page_content_with_ocr_text(&mut different, "Ref KX-204 ", true);
        assert_eq!(different.native_content.as_deref(), Some("Ref KX-204"));
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn a_blank_text_layer_is_not_kept() {
        for blank in ["", " \n\t "] {
            let mut page = text_layer_page(1, blank);
            replace_page_content_with_ocr_text(&mut page, "scanned words", true);
            assert_eq!(page.content, "scanned words");
            assert_eq!(page.native_content, None, "text layer {blank:?}");
        }

        let mut one_character = text_layer_page(1, "7");
        replace_page_content_with_ocr_text(&mut one_character, "scanned words", true);
        assert_eq!(one_character.native_content.as_deref(), Some("7"));
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn pages_cleared_by_a_single_ocr_text_keep_their_text_layer() {
        let pts = vec!["whole document text".to_string()];
        let mut pages: Vec<_> = (1u32..=3u32)
            .map(|n| text_layer_page(n, &format!("native page {n}")))
            .collect();

        replace_pages_with_ocr_page_texts(&mut pages, &pts, true);

        assert_eq!(pages[0].content, "whole document text");
        assert_eq!(pages[0].native_content.as_deref(), Some("native page 1"));
        for (page, native) in pages
            .iter()
            .zip(["native page 1", "native page 2", "native page 3"])
            .skip(1)
        {
            assert!(page.content.is_empty(), "page {} must be cleared", page.page_number);
            assert_eq!(page.is_blank, Some(true));
            assert_eq!(page.native_content.as_deref(), Some(native));
        }

        let mut not_kept: Vec<_> = (1u32..=2u32)
            .map(|n| text_layer_page(n, &format!("native page {n}")))
            .collect();
        replace_pages_with_ocr_page_texts(&mut not_kept, &pts, false);
        assert!(not_kept[1].content.is_empty());
        assert_eq!(not_kept[1].native_content, None);
    }

    /// Regression for #1095: when OCR texts are written into existing PageContent entries,
    /// is_blank must be recalculated from the new content, not left stale from native extraction.
    ///
    /// Simulates scanned PDF pages (all blank natively) receiving OCR content.
    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_page_texts_update_is_blank_on_existing_pages() {
        use crate::types::PageContent;

        let pts = vec!["page one content".to_string(), "page two content".to_string()];

        let mut pages: Vec<PageContent> = (1u32..=2u32)
            .map(|n| PageContent {
                page_number: n,
                content: String::new(),
                tables: Vec::new(),
                image_indices: Vec::new(),
                image_preprocessing: None,
                hierarchy: None,
                is_blank: Some(true),
                layout_regions: None,
                speaker_notes: None,
                section_name: None,
                sheet_name: None,
                ocr_confidence: None,
                native_content: None,
            })
            .collect();

        replace_pages_with_ocr_page_texts(&mut pages, &pts, false);

        assert_eq!(
            pages[0].is_blank,
            Some(false),
            "page with OCR content must not be blank"
        );
        assert_eq!(
            pages[1].is_blank,
            Some(false),
            "page with OCR content must not be blank"
        );
    }

    /// Regression for #1095: pages built from OCR texts when no native pages exist
    /// must have is_blank derived from the OCR content, not left as None.
    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_scratch_pages_is_blank_set_from_content() {
        use crate::extraction::blank_detection::is_page_text_blank;
        use crate::types::PageContent;

        let pts = vec!["substantial ocr content".to_string(), String::new()];

        let page_contents: Vec<PageContent> = pts
            .into_iter()
            .enumerate()
            .map(|(i, text)| {
                let content = crate::pdf::text::fix_pdf_control_chars(&text).into_owned();
                let is_blank = Some(is_page_text_blank(&content));
                PageContent {
                    page_number: (i + 1) as u32,
                    content,
                    tables: Vec::new(),
                    image_indices: vec![],
                    image_preprocessing: None,
                    hierarchy: None,
                    is_blank,
                    layout_regions: None,
                    speaker_notes: None,
                    section_name: None,
                    sheet_name: None,
                    ocr_confidence: None,
                    native_content: None,
                }
            })
            .collect();

        assert_eq!(
            page_contents[0].is_blank,
            Some(false),
            "page with content must not be blank"
        );
        assert_eq!(
            page_contents[1].is_blank,
            Some(true),
            "page with empty content must be blank"
        );
    }

    /// Integration regression for #1095: force_ocr on a scanned (non-searchable) PDF with
    /// extract_pages=true must produce pages where is_blank reflects OCR content, not stale
    /// native-extraction state.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_force_ocr_sets_is_blank_from_ocr_content() {
        use crate::core::config::{OcrConfig, PageConfig};

        let _backend = register_mock_ocr_backend("is-blank-fix-1095", "extracted ocr text content here");
        let extractor = PdfExtractor::new();
        let config = ExtractionConfig {
            force_ocr: true,
            ocr: Some(OcrConfig {
                backend: "is-blank-fix-1095".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let pdf_path = pdf_test_document("non_searchable.pdf");
        let content = std::fs::read(&pdf_path).unwrap_or_else(|e| panic!("non_searchable.pdf must be readable: {e}"));
        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("extraction should succeed");
        let result = crate::extraction::derive::derive_extraction_result(
            result,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        let pages = result.pages.expect("pages must be present when extract_pages=true");
        assert!(!pages.is_empty(), "must have at least one page");
        for page in &pages {
            assert_eq!(
                page.is_blank,
                Some(false),
                "page {} has OCR content, is_blank must be Some(false) (issue #1095)",
                page.page_number
            );
        }
    }

    /// Regression for #1293: `metadata.pages.pages[].is_blank` must agree with the
    /// top-level `pages[].is_blank` for OCR'd pages. Previously `PageInfo.is_blank`
    /// was computed once from native content in `build_page_structure` and never
    /// refreshed after OCR text overwrote `PageContent`, so a scanned page with no
    /// native text but real OCR text reported `is_blank=true` in metadata while the
    /// top-level page reported `is_blank=false`.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_metadata_page_info_is_blank_matches_page_content_after_ocr() {
        use crate::core::config::{OcrConfig, PageConfig};

        let _backend = register_mock_ocr_backend("is-blank-metadata-fix-1293", "extracted ocr text content here");
        let extractor = PdfExtractor::new();
        let config = ExtractionConfig {
            force_ocr: true,
            ocr: Some(OcrConfig {
                backend: "is-blank-metadata-fix-1293".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let pdf_path = pdf_test_document("non_searchable.pdf");
        let content = std::fs::read(&pdf_path).unwrap_or_else(|e| panic!("non_searchable.pdf must be readable: {e}"));
        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("extraction should succeed");
        let result = crate::extraction::derive::derive_extraction_result(
            result,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        let pages = result
            .pages
            .clone()
            .expect("pages must be present when extract_pages=true");
        let metadata_pages = result
            .metadata
            .pages
            .as_ref()
            .and_then(|structure| structure.pages.as_ref())
            .expect("metadata.pages.pages must be present when extract_pages=true");

        assert!(!pages.is_empty(), "must have at least one page");
        assert_eq!(
            pages.len(),
            metadata_pages.len(),
            "page counts must match between the two sources"
        );

        for page in &pages {
            let info = metadata_pages
                .iter()
                .find(|p| p.number == page.page_number)
                .unwrap_or_else(|| panic!("metadata.pages.pages missing entry for page {}", page.page_number));
            assert_eq!(
                info.is_blank, page.is_blank,
                "page {}: metadata.pages.pages[].is_blank ({:?}) must match top-level pages[].is_blank ({:?}) (issue #1293)",
                page.page_number, info.is_blank, page.is_blank
            );
            assert_eq!(
                info.is_blank,
                Some(false),
                "page {} has OCR content, metadata is_blank must be Some(false) (issue #1293)",
                page.page_number
            );
        }
    }

    /// ocr_inline_images=true on a text-only PDF (no embedded images) must succeed
    /// and return an empty images list, not panic or error.
    #[tokio::test]
    // `ocr_inline_images` is rejected by config validation unless an OCR BACKEND is
    // compiled in, so the `pdf`-only feature leg cannot run this test -- gating it on
    // `pdf` alone left that leg permanently red for a reason unrelated to PDF.
    // `ocr`, not `ocr-pipeline`: `ocr = ["ocr-pipeline", ...]`, so the pipeline can be
    // enabled with no backend registered, which fails one step later instead. ~keep
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    async fn test_pdf_ocr_inline_images_no_images_in_document() {
        use crate::core::config::ExtractionConfig;
        use crate::core::config::pdf::PdfConfig;

        let extractor = PdfExtractor::new();

        let pdf_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/code_and_formula.pdf");

        if !pdf_path.exists() {
            panic!("missing test fixture: {pdf_path:?}");
        }

        let content = std::fs::read(pdf_path).expect("Failed to read PDF");

        let config = ExtractionConfig {
            pdf_options: Some(PdfConfig {
                ocr_inline_images: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("Extraction should succeed even when there are no images to OCR");

        for img in &result.images {
            assert!(img.ocr_result.is_none(), "text-only PDF should produce no OCR results");
        }
    }

    /// ocr_inline_images=true with config.ocr=None must use TesseractConfig::default()
    /// and not panic.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    async fn test_pdf_ocr_inline_images_no_ocr_config() {
        use crate::core::config::ExtractionConfig;
        use crate::core::config::pdf::PdfConfig;

        let extractor = PdfExtractor::new();

        let pdf_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/pdf/embedded_images_tables.pdf");

        assert!(
            pdf_path.exists(),
            "missing test fixture: {pdf_path:?} — add embedded_images_tables.pdf to test_documents/pdf/"
        );

        let content = std::fs::read(pdf_path).expect("Failed to read PDF");

        let config = ExtractionConfig {
            ocr: None,
            pdf_options: Some(PdfConfig {
                ocr_inline_images: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let _result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("Extraction with ocr=None and ocr_inline_images=true must not panic");
    }

    /// Regression for issue #917: a mixed document with good aggregate text but a
    /// scanned page must still reach OCR. Before the fix, `has_substantive_doc=true`
    /// alone suppressed OCR even when `decision.fallback=true`.
    ///
    /// Tests `evaluate_ocr_skip_gate` directly — the function that the production
    /// path delegates to — so that reverting `&& !decision_fallback` breaks this test.
    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_gate_runs_ocr_when_substantive_doc_but_fallback_needed() {
        let thresholds = OcrQualityThresholds::default();

        let outcome = ocr::evaluate_ocr_skip_gate(true, 500, 0.9, &mk_decision(true, true, vec![]), &thresholds);
        assert_eq!(
            outcome,
            ocr::OcrGateOutcome::RunFallback,
            "substantive doc must not suppress OCR when per-page fallback is needed (issue #917)"
        );
    }

    /// Counterpart: when no per-page fallback is needed, a substantive doc correctly
    /// skips OCR. Ensures the fix doesn't over-correct and always run OCR.
    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_gate_skips_when_substantive_doc_and_no_fallback() {
        let thresholds = OcrQualityThresholds::default();

        let outcome = ocr::evaluate_ocr_skip_gate(true, 500, 0.9, &mk_decision(false, false, vec![]), &thresholds);
        assert_eq!(
            outcome,
            ocr::OcrGateOutcome::SkipSubstantive,
            "OCR should be skipped when doc is substantive and no per-page fallback is needed"
        );
    }

    /// Regression for #987: image placeholders must appear in Markdown output when
    /// `force_ocr` is used and `config.images.inject_placeholders = true`.
    ///
    /// Uses a mock OCR backend so the test is independent of tessdata availability.
    /// Image elements are appended after text on the OCR path; positional interleaving
    /// is a known follow-up (tracked separately).
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_inject_placeholders_present_on_force_ocr_path() {
        use crate::core::config::{ImageExtractionConfig, OcrConfig, OutputFormat};

        let _backend = register_mock_ocr_backend("inject-placeholder-ocr", "mock page text");
        let extractor = PdfExtractor::new();

        let pdf_path = pdf_test_document("embedded_images_tables.pdf");
        assert!(
            pdf_path.exists(),
            "missing test fixture: {pdf_path:?} — add embedded_images_tables.pdf to test_documents/pdf/"
        );
        let content = std::fs::read(&pdf_path).expect("failed to read embedded_images_tables.pdf");

        let config = crate::core::config::ExtractionConfig {
            output_format: OutputFormat::Markdown,
            force_ocr: true,
            ocr: Some(OcrConfig {
                backend: "inject-placeholder-ocr".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            images: Some(ImageExtractionConfig {
                extract_images: true,
                inject_placeholders: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("force_ocr extraction with images should succeed");

        let result = crate::extraction::derive::derive_extraction_result(result, true, OutputFormat::Markdown);

        if result.images.as_ref().is_some_and(|imgs| !imgs.is_empty()) {
            assert!(
                result.formatted_content.as_deref().unwrap_or("").contains("![") || result.content.contains("!["),
                "Markdown must contain image placeholders on the force_ocr path when inject_placeholders=true"
            );
        }
    }

    /// Verifies that `inject_placeholders` defaults to false when only
    /// `pdf_options.extract_images` is set and `config.images` is absent,
    /// so callers who never touched `config.images` do not get unexpected placeholders.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_inject_placeholders_absent_when_only_pdf_options_set() {
        use crate::core::config::{OcrConfig, OutputFormat, pdf::PdfConfig};

        let _backend = register_mock_ocr_backend("inject-placeholder-absent-ocr", "mock page text");
        let extractor = PdfExtractor::new();

        let pdf_path = pdf_test_document("embedded_images_tables.pdf");
        assert!(
            pdf_path.exists(),
            "missing test fixture: {pdf_path:?} — add embedded_images_tables.pdf to test_documents/pdf/"
        );
        let content = std::fs::read(&pdf_path).expect("failed to read embedded_images_tables.pdf");

        let config = crate::core::config::ExtractionConfig {
            output_format: OutputFormat::Markdown,
            force_ocr: true,
            ocr: Some(OcrConfig {
                backend: "inject-placeholder-absent-ocr".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            pdf_options: Some(PdfConfig {
                extract_images: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("force_ocr extraction with pdf_options should succeed");

        let result = crate::extraction::derive::derive_extraction_result(result, true, OutputFormat::Markdown);

        let markdown = result.formatted_content.as_deref().unwrap_or(&result.content);
        assert!(
            !markdown.contains("!["),
            "Markdown must NOT contain image placeholders when config.images is absent (inject_placeholders defaults to false)"
        );
    }

    /// GH#1668 end to end: `include_document_structure` alone, with `output_format`
    /// left at its `Plain` default and no hierarchy config, must trigger the
    /// structured native pass. Before the fix, this config silently took the flat
    /// path (`flat_pdf_document`, paragraph-only elements), so the structure tree
    /// this flag asks for came back holding nothing but `paragraph` nodes even
    /// though `embedded_images_tables.pdf` has a native table `counts.tables`
    /// reports correctly either way.
    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn test_include_document_structure_alone_triggers_the_structured_path() {
        use crate::core::config::OutputFormat;

        let extractor = PdfExtractor::new();
        let pdf_path = pdf_test_document("embedded_images_tables.pdf");
        assert!(
            pdf_path.exists(),
            "missing test fixture: {pdf_path:?} — add embedded_images_tables.pdf to test_documents/pdf/"
        );
        let content = std::fs::read(&pdf_path).expect("failed to read embedded_images_tables.pdf");

        let config = crate::core::config::ExtractionConfig {
            include_document_structure: true,
            ..Default::default()
        };

        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("native extraction with include_document_structure should succeed");

        let result = crate::extraction::derive::derive_extraction_result(result, true, OutputFormat::Plain);

        let structure = result
            .document
            .as_ref()
            .expect("include_document_structure=true must populate the document field");
        assert!(
            structure.node_types.iter().any(|kind| kind != "paragraph"),
            "structure must hold a non-paragraph node kind on a document with a native table; got {:?}",
            structure.node_types
        );
    }

    /// GH#1668 review follow-up: `test_include_document_structure_alone_triggers_the_structured_path`
    /// only asserts `node_types`, never `content`, so it cannot catch a change that keeps the
    /// structure tree correct while reverting the plain-text rendering the changelog also
    /// promises. `mod.rs`'s `select_pdf_document` Mixed-branch merge already gates a similar
    /// merge on `output_format != Plain` (see the comment there); a later change of that shape
    /// applied to the native path would silently revert this fix without failing the existing
    /// test. This test pins the plain-text side directly: with `include_document_structure` set,
    /// `Plain` output must still come from the structured document, so each table row renders as
    /// its own line (`render_table_plain` is `row.join(" ")` per row), and that output must differ
    /// from the flag left off.
    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn plain_output_with_document_structure_renders_each_table_row_on_its_own_line() {
        use crate::core::config::OutputFormat;

        let extractor = PdfExtractor::new();
        let pdf_path = pdf_test_document("embedded_images_tables.pdf");
        assert!(
            pdf_path.exists(),
            "missing test fixture: {pdf_path:?} — add embedded_images_tables.pdf to test_documents/pdf/"
        );
        let content = std::fs::read(&pdf_path).expect("failed to read embedded_images_tables.pdf");

        let flat_config = crate::core::config::ExtractionConfig {
            include_document_structure: false,
            ..Default::default()
        };
        let flat_result = extractor
            .extract_content(&content, "application/pdf", &flat_config)
            .await
            .expect("native extraction without include_document_structure should succeed");
        let flat_result = crate::extraction::derive::derive_extraction_result(flat_result, true, OutputFormat::Plain);

        let structured_config = crate::core::config::ExtractionConfig {
            include_document_structure: true,
            ..Default::default()
        };
        let structured_result = extractor
            .extract_content(&content, "application/pdf", &structured_config)
            .await
            .expect("native extraction with include_document_structure should succeed");
        let structured_result =
            crate::extraction::derive::derive_extraction_result(structured_result, true, OutputFormat::Plain);

        assert_eq!(
            flat_result.tables.len(),
            1,
            "fixture must carry exactly one native table with include_document_structure off"
        );
        assert_eq!(
            structured_result.tables.len(),
            1,
            "fixture must carry exactly one native table with include_document_structure on"
        );

        let table = &structured_result.tables[0];
        for row in &table.cells {
            let rendered_row = row.join(" ");
            assert!(
                structured_result.content.lines().any(|line| line == rendered_row),
                "expected table row {rendered_row:?} to render as its own line in the structured Plain content; got {:?}",
                structured_result.content
            );
        }

        assert_ne!(
            flat_result.content, structured_result.content,
            "include_document_structure must change the rendered Plain content for a document with a native table"
        );
    }

    /// Regression for #1355: when `force_ocr` renders a page blank (as xberg_native_pdf does
    /// for a page whose only visible content is an image XObject it silently failed to
    /// decode) but the page actually carries image XObjects, OCR must be retried on the
    /// embedded image bytes and the recovery must be surfaced as a `ProcessingWarning`.
    ///
    /// The mock backend returns an empty string for its first invocation (standing in
    /// for the blank whole-page render) and real text for every subsequent call. Because
    /// `embedded_images_tables.pdf` has exactly one page, the whole-page OCR call is
    /// always the first call the backend receives, so the fallback calls that follow in
    /// the same per-page loop deterministically observe call index >= 1.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_force_ocr_image_xobject_fallback_recovers_blank_page_and_warns() {
        use crate::core::config::OcrConfig;
        use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
        use crate::types::ExtractedDocument;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        const BACKEND_NAME: &str = "blank-then-image-fallback-mock-1355";

        struct BlankThenImageMockBackend {
            call_count: AtomicUsize,
        }

        #[async_trait::async_trait]
        impl OcrBackend for BlankThenImageMockBackend {
            fn backend_type(&self) -> OcrBackendType {
                OcrBackendType::Custom
            }
            fn supports_language(&self, _lang: &str) -> bool {
                true
            }
            async fn process_image(
                &self,
                _image_bytes: &[u8],
                _config: &OcrConfig,
            ) -> crate::Result<ExtractedDocument> {
                let call_index = self.call_count.fetch_add(1, Ordering::SeqCst);
                let content = if call_index == 0 {
                    String::new()
                } else {
                    "IMG_TEXT".to_string()
                };
                Ok(ExtractedDocument {
                    content,
                    ..Default::default()
                })
            }
            fn supports_document_processing(&self) -> bool {
                false
            }
        }

        impl Plugin for BlankThenImageMockBackend {
            fn name(&self) -> &str {
                BACKEND_NAME
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> crate::Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> crate::Result<()> {
                Ok(())
            }
        }

        crate::plugins::register_ocr_backend(Arc::new(BlankThenImageMockBackend {
            call_count: AtomicUsize::new(0),
        }))
        .unwrap();
        let _guard = RegisteredOcrBackendGuard { name: BACKEND_NAME };

        let extractor = PdfExtractor::new();
        let pdf_path = pdf_test_document("embedded_images_tables.pdf");
        assert!(
            pdf_path.exists(),
            "missing test fixture: {pdf_path:?} — add embedded_images_tables.pdf to test_documents/pdf/"
        );
        let content = std::fs::read(&pdf_path).expect("failed to read embedded_images_tables.pdf");

        let config = crate::core::config::ExtractionConfig {
            force_ocr: true,
            ocr: Some(OcrConfig {
                backend: BACKEND_NAME.to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("force_ocr extraction should succeed");
        let result = crate::extraction::derive::derive_extraction_result(
            result,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        assert!(
            !crate::extraction::blank_detection::is_page_text_blank(&result.content),
            "page content must be recovered from the embedded-image OCR fallback, not left blank; got {:?}",
            result.content
        );
        assert!(
            result.content.contains("IMG_TEXT"),
            "recovered content must come from the per-image fallback OCR call; got {:?}",
            result.content
        );
        assert!(
            result
                .processing_warnings
                .iter()
                .any(|w| w.source == "ocr" && w.message.contains("image XObject")),
            "expected a ProcessingWarning with source 'ocr' mentioning 'image XObject'; got: {:?}",
            result.processing_warnings
        );
    }

    /// False-positive guard for #1355: when the whole-page OCR result is not blank, the
    /// image-XObject fallback must never fire, even on a page that has real embedded
    /// images (`embedded_images_tables.pdf`) — no fallback warning, and page text
    /// unchanged from whatever the (mock) OCR backend returned.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_force_ocr_image_xobject_fallback_does_not_fire_on_non_blank_page() {
        use crate::core::config::OcrConfig;

        const BACKEND_NAME: &str = "non-blank-no-fallback-mock-1355";
        const SENTINEL: &str = "genuine OCR text, not blank";

        let _backend = register_mock_ocr_backend(BACKEND_NAME, SENTINEL);

        let extractor = PdfExtractor::new();
        let pdf_path = pdf_test_document("embedded_images_tables.pdf");
        let content = std::fs::read(&pdf_path).expect("failed to read embedded_images_tables.pdf");

        let config = crate::core::config::ExtractionConfig {
            force_ocr: true,
            ocr: Some(OcrConfig {
                backend: BACKEND_NAME.to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("force_ocr extraction should succeed");
        let result = crate::extraction::derive::derive_extraction_result(
            result,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        // The fixture also carries real tables that get merged into the final content
        // independently of OCR (unrelated to this fix), so assert the OCR-produced text
        // is present unchanged rather than an exact match on the whole document body.
        assert!(
            result.content.starts_with(SENTINEL),
            "non-blank OCR output must pass through unchanged when the fallback never fires; got {:?}",
            result.content
        );
        assert!(
            !result.content.contains("IMG_TEXT"),
            "the image fallback must never run when the whole-page OCR result was not blank; got {:?}",
            result.content
        );
        assert!(
            !result
                .processing_warnings
                .iter()
                .any(|w| w.message.contains("image XObject")),
            "no image-fallback warning must be added when the whole-page OCR result was not blank; got: {:?}",
            result.processing_warnings
        );
    }

    /// Fallback must win over the non-text skip even with a pre-rendered document. ~keep
    /// Otherwise scanned ToC-like pages are suppressed. ~keep
    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_gate_fallback_overrides_non_textual_skip() {
        let thresholds = OcrQualityThresholds::default();

        let outcome = ocr::evaluate_ocr_skip_gate(true, 500, 0.1, &mk_decision(true, false, vec![]), &thresholds);
        assert_eq!(
            outcome,
            ocr::OcrGateOutcome::RunFallback,
            "fallback must route to OCR rather than being suppressed as non-text"
        );

        let control = ocr::evaluate_ocr_skip_gate(true, 500, 0.1, &mk_decision(false, false, vec![]), &thresholds);
        assert_eq!(
            control,
            ocr::OcrGateOutcome::SkipNonText,
            "genuinely non-textual content without a fallback decision must still skip OCR"
        );
    }

    /// Hybrid PDF: per-page check fires on specific pages while the whole-document
    /// quality check passes. Gate must route to RunFallbackOnPages, not RunFallback.
    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_gate_targets_specific_pages_on_hybrid_pdf() {
        let thresholds = OcrQualityThresholds::default();

        let outcome = ocr::evaluate_ocr_skip_gate(false, 500, 0.9, &mk_decision(true, false, vec![3, 7]), &thresholds);
        assert_eq!(
            outcome,
            ocr::OcrGateOutcome::RunFallbackOnPages(vec![3, 7]),
            "hybrid PDF with specific failing pages must route to targeted OCR"
        );
    }

    /// Whole-document failure with no per-page list → full document OCR (existing behaviour).
    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_gate_full_document_when_whole_doc_failure() {
        let thresholds = OcrQualityThresholds::default();
        let outcome = ocr::evaluate_ocr_skip_gate(false, 500, 0.9, &mk_decision(true, true, vec![]), &thresholds);
        assert_eq!(outcome, ocr::OcrGateOutcome::RunFallback);
    }

    /// Edge case: whole-doc failure is true AND per-page list is populated.
    /// Whole-doc failure dominates (the document is fundamentally bad).
    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_gate_whole_doc_failure_dominates_per_page_list() {
        let thresholds = OcrQualityThresholds::default();
        let outcome =
            ocr::evaluate_ocr_skip_gate(false, 500, 0.9, &mk_decision(true, true, vec![1, 2, 3]), &thresholds);
        assert_eq!(
            outcome,
            ocr::OcrGateOutcome::RunFallback,
            "whole-doc failure must trigger full OCR even when per-page list is populated"
        );
    }

    /// evaluate_per_page_ocr must collect ALL failing pages, not short-circuit on the first.
    /// This is the core regression the original implementation had.
    #[cfg(feature = "ocr")]
    #[test]
    fn test_evaluate_per_page_ocr_collects_all_failing_pages() {
        use crate::types::PageBoundary;

        let good = "This page has plenty of meaningful searchable text content for extraction.";
        let bad = " . ; ";
        let text = format!("{}{}{}{}", good, bad, good, bad);
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: good.len(),
                page_number: 1,
            },
            PageBoundary {
                byte_start: good.len(),
                byte_end: good.len() + bad.len(),
                page_number: 2,
            },
            PageBoundary {
                byte_start: good.len() + bad.len(),
                byte_end: 2 * good.len() + bad.len(),
                page_number: 3,
            },
            PageBoundary {
                byte_start: 2 * good.len() + bad.len(),
                byte_end: text.len(),
                page_number: 4,
            },
        ];

        let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(4), &OcrQualityThresholds::default());
        assert!(decision.fallback);
        assert_eq!(
            decision.failing_pages,
            vec![2, 4],
            "all failing pages must be collected, not just the first"
        );
        assert!(
            !decision.whole_doc_failure,
            "whole-doc check should pass when half the document has good text"
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_keeps_good_page_when_document_average_fails() {
        use crate::types::PageBoundary;

        let good = "Native words remain searchable with the page table.";
        let mut boundaries = vec![PageBoundary {
            byte_start: 0,
            byte_end: good.len(),
            page_number: 1,
        }];
        boundaries.extend((2..=10).map(|page_number| PageBoundary {
            byte_start: good.len(),
            byte_end: good.len(),
            page_number,
        }));

        let decision = ocr::evaluate_per_page_ocr(good, Some(&boundaries), Some(10), &OcrQualityThresholds::default());

        assert_eq!(decision.failing_pages, (2..=10).collect::<Vec<_>>());
        assert!(!decision.whole_doc_failure);
        assert_eq!(
            ocr::evaluate_ocr_skip_gate(false, good.len(), 0.1, &decision, &OcrQualityThresholds::default()),
            ocr::OcrGateOutcome::RunFallbackOnPages((2..=10).collect())
        );
    }

    /// When every page fails the per-page quality check, the gate must route to
    /// RunFallback (ExtractionMethod::Ocr), not RunFallbackOnPages (ExtractionMethod::Mixed).
    /// A document where every page needs OCR is not a mixed document.
    #[cfg(feature = "ocr")]
    #[test]
    fn test_all_pages_failing_routes_to_run_fallback_not_mixed() {
        use crate::types::PageBoundary;

        let bad = " . ; ";
        let text = format!("{}{}", bad, bad);
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: bad.len(),
                page_number: 1,
            },
            PageBoundary {
                byte_start: bad.len(),
                byte_end: text.len(),
                page_number: 2,
            },
        ];

        let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2), &OcrQualityThresholds::default());
        assert!(decision.fallback);
        assert_eq!(decision.failing_pages, vec![1, 2]);
        assert!(
            decision.whole_doc_failure,
            "all pages failing must set whole_doc_failure so gate routes to RunFallback"
        );

        let outcome = ocr::evaluate_ocr_skip_gate(false, text.len(), 0.1, &decision, &OcrQualityThresholds::default());
        assert_eq!(
            outcome,
            ocr::OcrGateOutcome::RunFallback,
            "all-pages-failing must produce RunFallback (Ocr), not RunFallbackOnPages (Mixed)"
        );
    }

    /// Mock backend that records the OcrConfig it receives so tests can assert
    /// that fields like output_format are forwarded correctly.
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    struct ConfigCapturingBackend {
        name: &'static str,
        sentinel: &'static str,
        received_config: std::sync::Arc<std::sync::Mutex<Option<crate::core::config::OcrConfig>>>,
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    impl crate::plugins::Plugin for ConfigCapturingBackend {
        fn name(&self) -> &str {
            self.name
        }
        fn version(&self) -> String {
            "0.0.0".to_string()
        }
        fn initialize(&self) -> crate::Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> crate::Result<()> {
            Ok(())
        }
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[async_trait::async_trait]
    impl crate::plugins::OcrBackend for ConfigCapturingBackend {
        fn backend_type(&self) -> crate::plugins::OcrBackendType {
            crate::plugins::OcrBackendType::Custom
        }
        fn supports_language(&self, _: &str) -> bool {
            true
        }
        async fn process_image(
            &self,
            _image_bytes: &[u8],
            config: &crate::core::config::OcrConfig,
        ) -> crate::Result<crate::types::ExtractedDocument> {
            *self.received_config.lock().unwrap() = Some(config.clone());
            Ok(crate::types::ExtractedDocument {
                content: self.sentinel.to_string(),
                mime_type: std::borrow::Cow::Borrowed("text/plain"),
                ocr_elements: Some(vec![crate::types::OcrElement {
                    text: "backend element".to_string(),
                    page_number: 1,
                    ..Default::default()
                }]),
                ..Default::default()
            })
        }
    }

    /// Regression for #1088: ocr_inline_images must call the backend named in
    /// OcrConfig.backend, not always Tesseract via OcrProcessor.
    ///
    /// Uses the existing register_mock_ocr_backend helper. The sentinel string
    /// appearing in img.ocr_result.content proves which backend ran — no separate
    /// AtomicBool needed.
    ///
    /// Fixture note: with_images.pdf is used here (not embedded_images_tables.pdf)
    /// because xberg_native_pdf reliably extracts its single raster XObject.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_ocr_inline_images_uses_configured_backend() {
        const BACKEND_NAME: &str = "mock-inline-ocr-routing-1088";
        const SENTINEL: &str = "__inline_ocr_sentinel_1088__";
        let _guard = register_mock_ocr_backend(BACKEND_NAME, SENTINEL);

        let pdf_path = pdf_test_document("with_images.pdf");
        assert!(pdf_path.exists(), "missing test fixture: {pdf_path:?}");
        let content = std::fs::read(&pdf_path).expect("read fixture");

        let config = crate::core::config::ExtractionConfig {
            ocr: Some(crate::core::config::OcrConfig {
                backend: BACKEND_NAME.to_string(),
                ..Default::default()
            }),
            pdf_options: Some(crate::core::config::pdf::PdfConfig {
                ocr_inline_images: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = PdfExtractor::new()
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("extraction must not fail");

        assert!(
            !result.images.is_empty(),
            "with_images.pdf must yield at least one embedded image; \
             fixture may need replacing if xberg_native_pdf no longer extracts from it"
        );

        let images_with_ocr: Vec<_> = result.images.iter().filter(|img| img.ocr_result.is_some()).collect();

        assert!(
            !images_with_ocr.is_empty(),
            "at least one image must have an ocr_result when ocr_inline_images=true"
        );

        for img in &images_with_ocr {
            let nested = img.ocr_result.as_ref().unwrap();
            let content = nested.content.as_str();
            assert!(
                content.contains(SENTINEL),
                "ocr_result content '{content}' does not contain sentinel — \
                 backend routing is still going through hardcoded Tesseract"
            );
            assert!(
                nested.ocr_elements.is_none(),
                "a custom inline-image backend must not bypass the caller's absent element policy"
            );
        }
    }

    /// Verifies that the extraction-level output_format is forwarded to the backend
    /// via OcrConfig.output_format. This mirrors the standalone image extractor
    /// (image.rs) and allows backends that produce format-aware output (e.g. Markdown
    /// table rendering) to behave correctly for inline PDF images.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_ocr_inline_images_forwards_output_format() {
        use std::sync::{Arc, Mutex};
        const BACKEND_NAME: &str = "mock-inline-ocr-format-1088";
        const SENTINEL: &str = "__format_sentinel_1088__";

        let received_config = Arc::new(Mutex::new(None));
        let backend = Arc::new(ConfigCapturingBackend {
            name: BACKEND_NAME,
            sentinel: SENTINEL,
            received_config: Arc::clone(&received_config),
        });
        crate::plugins::register_ocr_backend(backend).unwrap();
        let _guard = RegisteredOcrBackendGuard { name: BACKEND_NAME };

        let pdf_path = pdf_test_document("with_images.pdf");
        assert!(pdf_path.exists(), "missing test fixture: {pdf_path:?}");
        let content = std::fs::read(&pdf_path).expect("read fixture");

        let config = crate::core::config::ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Markdown,
            ocr: Some(crate::core::config::OcrConfig {
                backend: BACKEND_NAME.to_string(),
                ..Default::default()
            }),
            pdf_options: Some(crate::core::config::pdf::PdfConfig {
                ocr_inline_images: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = PdfExtractor::new()
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("extraction must not fail");

        if result.images.is_empty() {
            panic!("with_images.pdf must yield images; fixture may need replacing");
        }

        let captured = received_config.lock().unwrap();
        let captured_config = captured
            .as_ref()
            .expect("backend was never called — no images were processed");
        assert_eq!(
            captured_config.output_format,
            Some(crate::core::config::OutputFormat::Markdown),
            "output_format was not forwarded to the inline-image OCR backend"
        );
    }

    /// When ocr_inline_images is false the mock backend must NOT be called even
    /// though it is registered as the configured backend.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_ocr_inline_images_disabled_does_not_call_backend() {
        const BACKEND_NAME: &str = "mock-inline-ocr-disabled-1088";
        let _guard = register_mock_ocr_backend(BACKEND_NAME, "should-never-appear");

        let pdf_path = pdf_test_document("with_images.pdf");
        assert!(pdf_path.exists(), "missing test fixture: {pdf_path:?}");
        let content = std::fs::read(&pdf_path).expect("read fixture");

        let config = crate::core::config::ExtractionConfig {
            ocr: Some(crate::core::config::OcrConfig {
                backend: BACKEND_NAME.to_string(),
                ..Default::default()
            }),
            pdf_options: Some(crate::core::config::pdf::PdfConfig {
                ocr_inline_images: false,
                extract_images: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = PdfExtractor::new()
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("extraction must not fail");

        assert!(
            !result.images.is_empty(),
            "with_images.pdf must yield images; fixture may need replacing"
        );
        for img in &result.images {
            assert!(
                img.ocr_result.is_none(),
                "img {} on page {:?} has ocr_result even though ocr_inline_images=false",
                img.image_index,
                img.page_number,
            );
        }
    }

    /// Tests form field extraction from a PDF.
    /// Uses an existing test PDF rather than creating one programmatically.
    /// This is a simple smoke test to verify that form field extraction works
    /// and doesn't panic or crash the extraction pipeline.
    /// Path to the vendored fillable-form fixture (AcroForm with text, button,
    /// and choice fields).
    #[cfg(feature = "pdf")]
    fn form_test_document() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/vendored/pdfium-render/form-test.pdf")
    }

    /// End-to-end: a real AcroForm PDF yields populated, correctly-typed
    /// `form_fields` on the extractor's `InternalDocument` carrier.
    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn test_form_field_extraction_enabled() {
        let content = std::fs::read(form_test_document()).expect("read form-test.pdf fixture");

        let config = crate::core::config::ExtractionConfig {
            pdf_options: Some(crate::core::config::pdf::PdfConfig {
                extract_form_fields: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let internal_doc = PdfExtractor::new()
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("extraction must not fail");

        assert!(
            !internal_doc.form_fields.is_empty(),
            "AcroForm PDF must yield form fields, got none"
        );
        assert!(
            internal_doc.form_fields.iter().all(|f| !f.full_name.is_empty()),
            "every extracted field must have a full_name"
        );
        assert!(
            internal_doc
                .form_fields
                .iter()
                .any(|f| f.field_type == crate::types::FormFieldType::Text),
            "fixture has text fields; at least one must map to FormFieldType::Text"
        );
    }

    /// With `extract_form_fields = false`, the same AcroForm PDF yields no fields.
    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn test_form_field_extraction_disabled() {
        let content = std::fs::read(form_test_document()).expect("read form-test.pdf fixture");

        let config = crate::core::config::ExtractionConfig {
            pdf_options: Some(crate::core::config::pdf::PdfConfig {
                extract_form_fields: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        let internal_doc = PdfExtractor::new()
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("extraction must not fail");

        assert!(
            internal_doc.form_fields.is_empty(),
            "form fields must be empty when extract_form_fields is disabled"
        );
    }

    /// Tests that form fields are properly extracted and carried through the pipeline
    /// using the InternalDocument.form_fields carrier (not metadata.additional).
    /// This test manually constructs an InternalDocument with form_fields to verify
    /// the carrier pattern works end-to-end, without relying on a complex PDF fixture.
    #[test]
    #[cfg(feature = "pdf")]
    fn test_form_fields_carrier_via_internal_document() {
        let mut doc = InternalDocument::new("pdf");
        doc.mime_type = "application/pdf".to_string();

        doc.form_fields = vec![crate::types::PdfFormField {
            name: "full_name".to_string(),
            full_name: "form.full_name".to_string(),
            field_type: crate::types::FormFieldType::Text,
            value: Some("Ada Lovelace".to_string()),
            default_value: Some("Default Name".to_string()),
            flags: 0,
            page: None,
            bbox: None,
            max_length: None,
            tooltip: None,
        }];

        assert_eq!(doc.form_fields.len(), 1);
        let field = &doc.form_fields[0];
        assert_eq!(field.name, "full_name");
        assert_eq!(field.value, Some("Ada Lovelace".to_string()));
        assert_eq!(field.field_type, crate::types::FormFieldType::Text);

        assert!(
            doc.metadata.additional.get("_pdf_form_fields").is_none(),
            "metadata.additional should not contain _pdf_form_fields (leak check)"
        );

        let result =
            crate::extraction::derive::derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);

        assert_eq!(result.form_fields.len(), 1);
        assert_eq!(result.form_fields[0].name, "full_name");
        assert_eq!(result.form_fields[0].value, Some("Ada Lovelace".to_string()));
    }

    /// Tests that with form_fields extraction disabled, the result contains no form fields.
    #[test]
    #[cfg(feature = "pdf")]
    fn test_form_fields_disabled_no_leak() {
        let mut doc = InternalDocument::new("pdf");
        doc.mime_type = "application/pdf".to_string();

        assert!(doc.form_fields.is_empty());

        assert!(doc.metadata.additional.get("_pdf_form_fields").is_none());

        let result =
            crate::extraction::derive::derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);

        assert!(result.form_fields.is_empty());
    }

    /// Two stroked boxes joined by a line that ends in a filled arrowhead, with a
    /// label centred in each box. Mirrors the fixture in
    /// `tests/diagram_dot_pdf.rs`, kept local so this unit test can inspect
    /// `InternalDocument::diagrams` directly rather than through the renderer.
    #[cfg(feature = "pdf")]
    fn two_boxes_and_an_arrow_pdf() -> Vec<u8> {
        use lopdf::{Document, Object, Stream, dictionary};

        let content = b"1 w 0 0 0 RG
10 150 80 30 re S
10 20 80 30 re S
50 150 m 50 56 l S
46 56 m 54 56 l 50 50 l h f
BT /F1 12 Tf 30 160 Td (Alpha) Tj ET
BT /F1 12 Tf 30 30 Td (Beta) Tj ET
"
        .to_vec();

        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();
        let font_id = document.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let content_id = document.add_object(Stream::new(dictionary! {}, content));
        let page_id = document.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()],
        });
        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);

        let mut bytes = Vec::new();
        document.save_to(&mut bytes).expect("fixture PDF must save");
        bytes
    }

    /// #579 defect: recovery ran on every PDF regardless of the requested
    /// output format, so an ordinary document paid for a second full text
    /// pass on any page that happened to look graph-shaped. `diagrams` is
    /// internal, so this has to be a unit test here rather than in the
    /// integration suite, which can only see the rendered `content` string —
    /// and `content` looks identical whether recovery ran and was discarded
    /// or never ran at all, so it cannot tell the two apart.
    #[cfg(feature = "pdf")]
    #[tokio::test]
    async fn recovery_is_skipped_unless_dot_output_is_requested() {
        let bytes = two_boxes_and_an_arrow_pdf();
        let extractor = PdfExtractor::new();

        let plain_doc = extractor
            .extract_content(&bytes, "application/pdf", &ExtractionConfig::default())
            .await
            .expect("plain extraction must succeed");
        assert_eq!(
            plain_doc.diagrams.len(),
            0,
            "diagram recovery must not run for non-dot output"
        );

        let dot_config = ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Custom("dot".to_string()),
            ..Default::default()
        };
        let dot_doc = extractor
            .extract_content(&bytes, "application/pdf", &dot_config)
            .await
            .expect("dot extraction must succeed");
        assert_eq!(dot_doc.diagrams.len(), 1, "diagram recovery must run for dot output");
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    #[test]
    fn should_attach_distinct_preprocessing_metadata_to_each_pdf_page() {
        fn preprocessing(target_dpi: i32, final_dpi: i32) -> crate::types::ImagePreprocessingMetadata {
            crate::types::ImagePreprocessingMetadata {
                original_dimensions: (100, 200).into(),
                original_dpi: (72.0, 72.0).into(),
                target_dpi,
                scale_factor: 1.0,
                auto_adjusted: false,
                final_dpi,
                new_dimensions: None,
                resample_method: "LANCZOS3".to_string(),
                dimension_clamped: false,
                calculated_dpi: None,
                skipped_resize: true,
                resize_error: None,
            }
        }

        let mut pages = Some(vec![
            crate::types::PageContent {
                page_number: 1,
                content: "first".to_string(),
                tables: Vec::new(),
                image_indices: Vec::new(),
                image_preprocessing: None,
                hierarchy: None,
                is_blank: Some(false),
                layout_regions: None,
                speaker_notes: None,
                section_name: None,
                sheet_name: None,
                ocr_confidence: None,
                native_content: None,
            },
            crate::types::PageContent {
                page_number: 2,
                content: "second".to_string(),
                tables: Vec::new(),
                image_indices: Vec::new(),
                image_preprocessing: None,
                hierarchy: None,
                is_blank: Some(false),
                layout_regions: None,
                speaker_notes: None,
                section_name: None,
                sheet_name: None,
                ocr_confidence: None,
                native_content: None,
            },
        ]);
        let by_page = ahash::AHashMap::from([(1, preprocessing(150, 150)), (2, preprocessing(300, 300))]);
        let root_metadata = super::attach_pdf_preprocessing_metadata(&mut pages, &by_page);

        let pages = pages.expect("PDF OCR pages must remain present");
        assert_eq!(
            pages[0]
                .image_preprocessing
                .as_ref()
                .map(|metadata| metadata.target_dpi),
            Some(150)
        );
        assert_eq!(
            pages[1]
                .image_preprocessing
                .as_ref()
                .map(|metadata| metadata.target_dpi),
            Some(300)
        );
        assert_eq!(
            root_metadata.as_ref().map(|metadata| metadata.target_dpi),
            Some(150),
            "the singular compatibility field must deterministically report the first preprocessed page"
        );
    }

    /// A minimal 2-page PDF carrying real text content, for tests that only need page
    /// structure and must not depend on the `test_documents/` corpus (see `read_test_fixture`).
    #[cfg(feature = "pdf")]
    fn build_two_page_text_pdf() -> Vec<u8> {
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets: Vec<usize> = Vec::new();

        offsets.push(pdf.len());
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        offsets.push(pdf.len());
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n");

        offsets.push(pdf.len());
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
              /Contents 5 0 R /Resources << /Font << /F1 7 0 R >> >> >>\nendobj\n",
        );

        offsets.push(pdf.len());
        pdf.extend_from_slice(
            b"4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
              /Contents 6 0 R /Resources << /Font << /F1 7 0 R >> >> >>\nendobj\n",
        );

        let stream1 = "BT /F1 12 Tf 72 700 Td (Page one has real text content here.) Tj ET\n";
        offsets.push(pdf.len());
        pdf.extend_from_slice(
            format!(
                "5 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
                stream1.len(),
                stream1
            )
            .as_bytes(),
        );

        let stream2 = "BT /F1 12 Tf 72 700 Td (Page two has different text content here.) Tj ET\n";
        offsets.push(pdf.len());
        pdf.extend_from_slice(
            format!(
                "6 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
                stream2.len(),
                stream2
            )
            .as_bytes(),
        );

        offsets.push(pdf.len());
        pdf.extend_from_slice(
            b"7 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
              /Encoding /WinAnsiEncoding >>\nendobj\n",
        );

        let xref_pos = pdf.len();
        let total_objs = offsets.len() + 1;
        pdf.extend_from_slice(format!("xref\n0 {}\n", total_objs).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f\r\n");
        for &off in &offsets {
            pdf.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                total_objs, xref_pos
            )
            .as_bytes(),
        );
        pdf
    }

    /// GH#1752: `metadata.pages` must be populated when `ocr_near_empty_fallback = Some(true)`
    /// is set with no `ocr` block, at parity with what an `ocr` block alone already produced.
    ///
    /// Before the fix, `page_structure_was_implicitly_tracked` in `extraction.rs` discarded the
    /// page structure whenever `config.ocr` was `None`, regardless of the two settings GH#1752
    /// gave the near-empty-fallback and scanned-page-quality-gate behaviours. Both configs here
    /// set `pdf_options: None` and `pages: None` so `force_annotation_page_tracking` is `true`,
    /// the only condition under which this site is reachable. Config X (an `ocr` block) is
    /// checked first as a control -- asserting it is `Some` before checking config Y rules out a
    /// fixture that simply never produces page structure at all, which would make the Y
    /// assertion pass vacuously.
    #[cfg(feature = "pdf")]
    #[tokio::test]
    async fn metadata_pages_populated_for_near_empty_fallback_without_ocr_block() {
        use crate::core::config::OcrConfig;

        let pdf = build_two_page_text_pdf();
        let extractor = PdfExtractor::new();

        let config_x = ExtractionConfig {
            ocr: Some(OcrConfig::default()),
            pdf_options: None,
            pages: None,
            ..ExtractionConfig::default()
        };
        let result_x = extractor
            .extract_content(&pdf, "application/pdf", &config_x)
            .await
            .expect("extraction with an `ocr` block must succeed");
        let result_x = crate::extraction::derive::derive_extraction_result(
            result_x,
            false,
            crate::core::config::OutputFormat::Plain,
        );
        let pages_x = result_x
            .metadata
            .pages
            .as_ref()
            .expect("control: an `ocr` block alone must populate metadata.pages");

        let config_y = ExtractionConfig {
            ocr: None,
            ocr_near_empty_fallback: Some(true),
            pdf_options: None,
            pages: None,
            ..ExtractionConfig::default()
        };
        let result_y = extractor
            .extract_content(&pdf, "application/pdf", &config_y)
            .await
            .expect("extraction with ocr_near_empty_fallback must succeed");
        let result_y = crate::extraction::derive::derive_extraction_result(
            result_y,
            false,
            crate::core::config::OutputFormat::Plain,
        );
        let pages_y = result_y.metadata.pages.as_ref().expect(
            "ocr_near_empty_fallback = Some(true) without an `ocr` block must also populate \
             metadata.pages (GH#1752)",
        );

        assert_eq!(
            pages_y.total_count, pages_x.total_count,
            "page counts must match between the two configurations"
        );
    }
}
