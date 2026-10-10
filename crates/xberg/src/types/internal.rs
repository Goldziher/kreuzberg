//! Internal flat document representation.
//!
//! This module provides the internal DTO that all extractors output. It is a flat,
//! append-only structure optimized for extraction performance. The public
//! [`DocumentStructure`](super::document_structure::DocumentStructure) tree and
//! relationship graph are derived from this in a post-processing step.
//!
//! # Design
//!
//! - **Flat `Vec<InternalElement>`**: Cache-friendly, append-only during extraction
//! - **Relationships stored separately**: Keeps element iteration compact
//! - **Optional container markers**: `ListStart`/`ListEnd` etc. improve tree derivation
//!   when present; depth-based heuristics used as fallback
//! - **OCR elements unified**: OCR text is just another element kind, not a parallel structure
//! - **Blake3 IDs**: Deterministic, collision-resistant identifiers

use std::fmt;

use ahash::AHashMap;
use serde::{Deserialize, Serialize};

use super::document_structure::{ContentLayer, TextAnnotation};
use super::extraction::BoundingBox;
use super::metadata::Metadata;
use super::ocr_elements::{OcrBoundingGeometry, OcrConfidence, OcrElementLevel, OcrRotation};
use super::tables::Table;
use crate::types::ExtractedImage;

const SUPPRESS_IMAGE_OCR_RENDER_ATTRIBUTE: &str = "xberg:internal:suppress-image-ocr-render";

/// Attribute key of an anchor for the OCR text of a picture; the value is the index of the
/// picture in `InternalDocument::images`.
const IMAGE_OCR_TEXT_ANCHOR_ATTRIBUTE: &str = "xberg:internal:image-ocr-text-anchor";

/// Attribute key carrying a `ListItem` element's literal source marker text
/// (e.g. `"B."`, `"(a)"`, `"iv."`), when the extractor recovered one.
///
/// Deliberately a generic attribute rather than a field on
/// [`ElementKind::ListItem`]: `ElementKind` derives `Copy` and is matched *by
/// value* (`match elem.kind { .. }`) throughout the renderers and the
/// structure-tree derivation step. A `String`-bearing field there would
/// silently drop the `Copy` derive and break every one of those call sites
/// across the crate. Routing the label through the existing attribute bag
/// keeps `ElementKind` unchanged and, unlike
/// [`SUPPRESS_IMAGE_OCR_RENDER_ATTRIBUTE`], is intentionally left out of the
/// [`public_attributes`](InternalElement::public_attributes) filter, so it
/// also reaches the public `DocumentStructure` tree via
/// `DocumentNode::attributes` with no change needed in `extraction::derive`.
const LIST_ITEM_SOURCE_LABEL_ATTRIBUTE: &str = "list_marker";

/// Attribute key carrying the measured dominant font size (in points) that the PDF
/// structure pipeline computed, stamped by `pdf::structure::assembly::push_paragraph_element`.
///
/// Internal-only plumbing, unlike [`LIST_ITEM_SOURCE_LABEL_ATTRIBUTE`]: filtered out of
/// [`public_attributes`](InternalElement::public_attributes) alongside
/// [`SUPPRESS_IMAGE_OCR_RENDER_ATTRIBUTE`]. Distinct from the OCR-only, public
/// `HOCR_FONT_SIZE_ATTRIBUTE` (`ocr::hocr_parser`), gated under `feature = "ocr"` and thus
/// unusable from a bare `pdf` build.
const MEASURED_FONT_SIZE_ATTRIBUTE: &str = "xberg:internal:font-size-pt";

pub(crate) const NATIVE_TABLE_GRID_ATTRIBUTE: &str = "xberg:internal:native-table-grid";

#[cfg_attr(alef, alef(skip))]
/// Deterministic element identifier, generated via blake3 hashing.
///
/// Format: `"ie-{12 hex chars}"` (48 bits from blake3, ~281 trillion address space).
/// Same input always produces the same ID, enabling diffing and caching.
///
/// Serializes as a plain string (`"ie-aabbccddeeff"`).
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InternalElementId([u8; 15]);

impl Serialize for InternalElementId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for InternalElementId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        if s.len() != 15 {
            return Err(serde::de::Error::custom(format!(
                "InternalElementId must be 15 bytes, got {}",
                s.len()
            )));
        }
        let mut buf = [0u8; 15];
        buf.copy_from_slice(s.as_bytes());
        Ok(Self(buf))
    }
}

impl InternalElementId {
    /// Generate a deterministic ID from element content.
    ///
    /// Hashes the element kind discriminant, text content, page number, and
    /// positional index using blake3. Takes 48 bits (6 bytes) of the hash.
    pub(crate) fn generate(kind_discriminant: &str, text: &str, page: Option<u32>, index: u32) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(kind_discriminant.as_bytes());
        hasher.update(text.as_bytes());
        hasher.update(&page.unwrap_or(u32::MAX).to_le_bytes());
        hasher.update(&index.to_le_bytes());
        let hash = hasher.finalize();
        let bytes = &hash.as_bytes()[..6];
        let mut buf = [0u8; 15];
        buf[0] = b'i';
        buf[1] = b'e';
        buf[2] = b'-';
        hex::encode_to_slice(bytes, &mut buf[3..]).expect("fixed size");
        Self(buf)
    }

    /// Get the ID as a string slice.
    pub(crate) fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap()
    }
}

impl fmt::Display for InternalElementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for InternalElementId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// One OCR page's authoritative processed-raster coordinate frame (GH#1645).
///
/// Public `OcrElement` geometry stays in the OCR backend's own raster pixel space even
/// after the rest of a page's document is normalized into PDF page points, and
/// `Metadata::additional`'s document-wide `ocr_processed_image_width/height` pair cannot
/// describe differently sized, preprocessed, or rotated pages. This record is the per-page
/// authority a multi-page consumer joins to an `OcrElement` through the element's own
/// `page_number`.
///
/// Crate-private and carried on [`InternalDocument::ocr_coordinate_frame`] with
/// `#[serde(skip)]` so it never crosses the plugin-bridge JSON wire format or gets an alef
/// binding DTO; it is folded into `Metadata::additional` under
/// `ocr_metadata_keys::OCR_PAGE_COORDINATE_FRAMES_METADATA_KEY` once a page's document is
/// final (`extractors::pdf::mod`).
// Gated to match the only code that writes or reads it, in `extractors::pdf::ocr`.
// Ungated, the `ocr`-without-`pdf` feature leg compiles the field with no writer and no
// reader, which `-D warnings` rejects as dead_code — and CI builds that leg. ~keep
#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
// `pub` + alef(skip), not `pub(crate)`: `InternalDocument` is a `pub` struct whose every
// other field is `pub`, and struct-update syntax requires ALL fields to be visible at the
// construction site. One `pub(crate)` field therefore breaks the 12 integration tests that
// build one with `..Default::default()`, since those are separate crates. alef(skip)
// keeps it out of the generated bindings, and `#[serde(skip)]` on the field keeps it off
// the plugin-bridge wire. ~keep
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct OcrPageCoordinateFrame {
    /// 1-based page this frame describes.
    pub page_number: u32,
    /// Processed raster width in pixels.
    pub width: u32,
    /// Processed raster height in pixels.
    pub height: u32,
    /// Always `"pixel"`.
    pub unit: &'static str,
    /// Always `"top_left"`.
    pub origin: &'static str,
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
impl OcrPageCoordinateFrame {
    /// `width`/`height` must already be known non-zero -- callers gate on that before
    /// constructing one, so this never represents a degenerate/fabricated frame.
    pub fn new(page_number: u32, width: u32, height: u32) -> Self {
        Self {
            page_number,
            width,
            height,
            unit: "pixel",
            origin: "top_left",
        }
    }
}

/// One PDF page's raw MediaBox coordinate frame (GH#1653 + GH#1654).
///
/// GH#1653: `origin_x`/`origin_y` are the MediaBox `llx`/`lly`, which can be non-zero and
/// negative -- a consumer that assumes `(0, 0)` mis-places every geometry field the page
/// reports. GH#1654: `clockwise_rotation` is the page `/Rotate`. Both gaps are one record,
/// not two, because a consumer needs the origin and the rotation together to place a page's
/// geometry in display space.
///
/// `width`/`height` are the MediaBox *extent* (`urx - llx`, `ury - lly`) and are deliberately
/// NOT swapped for a 90/270 rotation: this describes raw PDF user space, the space
/// `HierarchicalBlock.bbox` and other segment geometry actually live in
/// (`pdf::structure::types`), not the displayed/rotated frame. This is the opposite of
/// `OcrPageCoordinateFrame` above, which reports the already-rotated processed raster. ~keep
#[cfg(feature = "pdf")]
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub(crate) struct PdfPageCoordinateFrame {
    /// 1-based page this frame describes.
    pub page_number: u32,
    /// MediaBox `llx`.
    pub origin_x: f32,
    /// MediaBox `lly`.
    pub origin_y: f32,
    /// MediaBox extent: `urx - llx`. Not swapped for 90/270 rotation.
    pub width: f32,
    /// MediaBox extent: `ury - lly`. Not swapped for 90/270 rotation.
    pub height: f32,
    /// Always `"point"`.
    pub unit: &'static str,
    /// Always `"bottom_left"`.
    pub origin: &'static str,
    /// The page `/Rotate`, normalized to one of `{0, 90, 180, 270}`.
    pub clockwise_rotation: i32,
}

#[cfg(feature = "pdf")]
impl PdfPageCoordinateFrame {
    /// Returns `None` for a MediaBox that does not yield a usable extent.
    ///
    /// A MediaBox is not guaranteed well-ordered -- the margin filter at this record's own
    /// call site defensively takes `min`/`max` over the same y pair -- so an inverted or
    /// degenerate box produces a zero or negative extent. Publishing that as an authoritative
    /// frame is worse than publishing nothing, so the page is omitted instead, matching the
    /// fail-closed convention `OcrPageCoordinateFrame` uses for invalid raster dimensions. ~keep
    pub fn new(page_number: u32, llx: f32, lly: f32, urx: f32, ury: f32, clockwise_rotation: i32) -> Option<Self> {
        let width = urx - llx;
        let height = ury - lly;
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return None;
        }
        Some(Self {
            page_number,
            origin_x: llx,
            origin_y: lly,
            width,
            height,
            unit: "point",
            origin: "bottom_left",
            clockwise_rotation,
        })
    }
}

#[cfg_attr(alef, alef(skip))]
/// The internal flat document representation.
///
/// All extractors output this structure. It is converted to the public
/// [`ExtractedDocument`](super::extraction::ExtractedDocument) and
/// [`DocumentStructure`](super::document_structure::DocumentStructure) in the pipeline.
///
/// Implements `Serialize`/`Deserialize` so that foreign-language plugin implementations
/// (Python, TypeScript, Ruby, etc.) can construct and return this type via JSON at the
/// FFI/trait-bridge boundary.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InternalDocument {
    /// All elements in reading order. Append-only during extraction.
    pub elements: Vec<InternalElement>,

    /// Relationships between elements (source index → target).
    /// Stored separately from elements for cache-friendly iteration.
    pub relationships: Vec<Relationship>,

    /// Source format identifier (e.g., "pdf", "docx", "html", "markdown").
    pub source_format: String,

    /// Document-level metadata (title, author, dates, etc.).
    pub metadata: Metadata,

    /// Extracted images (binary data). Referenced by index from `ElementKind::Image`.
    pub images: Vec<ExtractedImage>,

    /// Extracted tables (structured data). Referenced by index from `ElementKind::Table`.
    pub tables: Vec<Table>,

    /// Node/edge graphs recovered from vector diagrams in the source.
    ///
    /// Populated by extractors that can read diagram geometry deterministically
    /// (SVG today). The `dot` renderer turns these into Graphviz DOT; every
    /// other renderer ignores them, so the field only ever adds output.
    #[serde(default)]
    pub diagrams: Vec<super::diagram::DiagramGraph>,

    /// URIs/links discovered during extraction (hyperlinks, image refs, citations, etc.).
    pub uris: Vec<super::uri::ExtractedUri>,

    /// Number of URIs [`InternalDocument::push_uri`] refused because `uris` was
    /// already at the per-document `MAX_URIS` cap.
    ///
    /// Not part of the plugin-bridge wire format: a foreign plugin deserializes
    /// straight into `uris` and never goes through the cap, so it has nothing to
    /// report here. `derive_extraction_result` turns a non-zero count into a
    /// `ProcessingWarning` (#76).
    #[serde(skip)]
    pub uris_dropped: usize,

    /// Archive children: fully-extracted results for files within an archive.
    ///
    /// Only populated by archive extractors (ZIP, TAR, 7z, GZIP) when recursive
    /// extraction is enabled. Each entry contains the full `ExtractedDocument` for
    /// a child file that was extracted through the public pipeline.
    pub children: Option<Vec<crate::types::ArchiveEntry>>,

    /// MIME type of the source document (e.g., "application/pdf", "text/html").
    pub mime_type: String,

    /// Non-fatal warnings collected during extraction.
    pub processing_warnings: Vec<crate::types::ProcessingWarning>,

    /// Structured OCR backend failures collected while extracting PDF pages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ocr_page_failures: Vec<crate::types::OcrPageFailure>,

    /// PDF annotations (links, highlights, notes).
    pub annotations: Option<Vec<crate::types::annotations::PdfAnnotation>>,

    /// Pre-built per-page content (set by extractors that track page boundaries natively).
    ///
    /// When populated, `derive_extraction_result` uses this directly instead of
    /// attempting to reconstruct pages from element-level page numbers.
    pub prebuilt_pages: Option<Vec<crate::types::PageContent>>,

    /// Pre-rendered formatted content produced by the extractor itself.
    ///
    /// When an extractor has direct access to high-quality formatted output (e.g.,
    /// html-to-markdown produces GFM markdown), it can store that here to bypass
    /// the lossy InternalDocument → renderer round-trip. `derive_extraction_result`
    /// will use this directly when the requested output format matches
    /// `metadata.output_format`.
    pub pre_rendered_content: Option<String>,

    /// Pre-built OCR element list (set by extractors that have direct access to
    /// bounding-box element data alongside a separately produced coherent text).
    ///
    /// When populated, `derive_extraction_result` uses this directly instead of
    /// reconstructing `OcrElement`s from `OcrText` `InternalElement`s. This lets
    /// the image extractor carry Tesseract/paddle-ocr bounding-box metadata without
    /// injecting raw word tokens into the element list (which would otherwise corrupt
    /// `render_plain` and page content — issue #706).
    pub prebuilt_ocr_elements: Option<Vec<crate::types::ocr_elements::OcrElement>>,

    /// LLM usage records accumulated during extraction (e.g., VLM OCR per page).
    ///
    /// Populated by extractors that call LLM-backed backends (VLM OCR).
    /// `derive_extraction_result` transfers this to `ExtractedDocument.llm_usage`.
    pub llm_usage: Option<Vec<crate::types::LlmUsage>>,

    /// Track-changes revisions embedded in the source document.
    ///
    /// Set by format-specific extractors (DOCX, ODT, …) that parse
    /// change-tracking markup. `derive_extraction_result` transfers this
    /// directly to `ExtractedDocument.revisions`.
    pub revisions: Option<Vec<crate::types::revisions::DocumentRevision>>,

    /// PDF form fields extracted from AcroForm or XFA-based forms.
    ///
    /// Set by the PDF extractor when `pdf_options.extract_form_fields = true`.
    /// `derive_extraction_result` transfers this directly to `ExtractedDocument.form_fields`.
    pub form_fields: Vec<crate::types::PdfFormField>,

    /// Mathematical formulas recognized during layout-guided OCR.
    ///
    /// Set by the OCR pipeline (per-page formulas, renumbered to document pages).
    /// `derive_extraction_result` transfers this directly to `ExtractedDocument.formulas`.
    pub formulas: Vec<crate::types::Formula>,

    /// Formulas that stay inside the text they came from.
    ///
    /// A table cell keeps its equation in the cell, and an HWP sentence keeps
    /// its equation in the sentence, so neither can become an element without
    /// changing that text. They are still formulas of the document.
    ///
    /// Kept apart from `formulas` because that field holds what the OCR
    /// pipeline produced, and `derive_extraction_result` treats an OCR entry as
    /// a second representation of an element. An entry here is not.
    #[serde(skip)]
    pub recorded_formulas: Vec<crate::types::Formula>,

    /// When `true`, image OCR results are rendered as plain text without the
    /// `![...](...)` markdown placeholder. Set by the pipeline from
    /// `ImageExtractionConfig.ocr_text_only`.
    #[serde(skip)]
    pub ocr_text_only: bool,

    /// When `true` and `ocr_text_only` is `false`, append the OCR text after
    /// the image placeholder in the rendered output. Set by the pipeline from
    /// `ImageExtractionConfig.append_ocr_text`.
    #[serde(skip)]
    pub append_ocr_text: bool,

    /// When `true` (the default), Markdown rendering backslash-escapes
    /// CommonMark-significant characters (`_[]()*=-#`) so the output round-trips
    /// safely through a CommonMark parser. When `false`, those escapes are
    /// stripped so prose reads identically to the already-unescaped text used in
    /// table cells. Set by the pipeline from `ExtractionConfig::escape_markdown`.
    #[serde(skip)]
    pub escape_markdown: bool,

    /// ~keep When `true`, Markdown rendering preserves watermark text such as arXiv
    /// identifiers. Set by the pipeline from `ContentFilterConfig::include_watermarks`.
    #[serde(skip)]
    pub include_watermarks: bool,

    /// Page marker format (with `{page_num}` placeholder) when
    /// `PageConfig::insert_page_markers` is enabled, `None` otherwise. Set by
    /// the pipeline. Renderers use it to emit page markers verbatim instead of
    /// escaping or stripping them.
    #[serde(skip)]
    pub page_marker_format: Option<String>,

    /// When `true`, Markdown rendering inserts a `[TABLE:{table_id}]` marker
    /// immediately before each table's rendered Markdown block. Set by the
    /// pipeline from `ExtractionConfig::table_anchors`. Defaults to `false`.
    #[serde(skip)]
    pub table_anchors: bool,

    /// This OCR page's authoritative processed-raster coordinate frame, captured before
    /// the page-local metadata it comes from is discarded (GH#1645). `#[serde(skip)]` for
    /// the same reason as the fields above -- it never crosses the plugin-bridge JSON wire
    /// format -- and is folded into `Metadata::additional` once the page document is final
    /// rather than becoming a new public binding type. `None` for every non-OCR document,
    /// and for an OCR'd page whose render raster was degenerate (0x0).
    #[serde(skip)]
    #[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
    pub ocr_coordinate_frame: Option<OcrPageCoordinateFrame>,

    /// Set by an OCR backend when detected tables claimed every text element of this page
    /// (#1571), so the page text holds only lines the tables already carry. Never crosses the
    /// plugin-bridge JSON wire format.
    #[serde(skip)]
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    pub ocr_text_claimed_by_tables: bool,
}

impl From<crate::types::extraction::ExtractedDocument> for InternalDocument {
    /// Conversion used at FFI/trait-bridge boundaries where a foreign-language plugin
    /// returns the public `ExtractedDocument` shape but the canonical Rust trait
    /// signature requires an `InternalDocument`. The text content is stashed in
    /// `pre_rendered_content` so the pipeline returns it verbatim instead of trying
    /// to re-render from a non-existent element tree.
    ///
    /// Every field with an exact `InternalDocument` destination is carried over. The
    /// conversion is still lossy for the derived-only parts of the public shape — the
    /// flat element list, the relationship graph, and `DocumentStructure` cannot be
    /// reconstructed from `ExtractedDocument`, which is why `pre_rendered_content`
    /// carries the text.
    fn from(result: crate::types::extraction::ExtractedDocument) -> Self {
        let extraction_method = result.extraction_method;
        let mut doc = Self::new(result.mime_type.as_ref());
        doc.mime_type = result.mime_type.into_owned();
        doc.metadata = result.metadata;
        if let Some(extraction_method) = extraction_method {
            doc.metadata.additional.insert(
                std::borrow::Cow::Borrowed("extraction_method"),
                serde_json::Value::String(extraction_method.as_str().to_string()),
            );
        }
        doc.tables = result.tables;
        doc.images = result.images.unwrap_or_default();
        doc.uris = result.uris.unwrap_or_default();
        doc.children = result.children;
        doc.annotations = result.annotations;
        doc.processing_warnings = result.processing_warnings;
        doc.ocr_page_failures = result.ocr_page_failures.unwrap_or_default();
        doc.llm_usage = result.llm_usage;
        doc.prebuilt_pages = result.pages;
        doc.prebuilt_ocr_elements = result.ocr_elements;
        doc.revisions = result.revisions;
        doc.form_fields = result.form_fields;
        doc.formulas = result.formulas;
        doc.pre_rendered_content = if result.content.is_empty() {
            None
        } else {
            Some(result.content)
        };
        doc
    }
}

impl From<InternalDocument> for crate::types::extraction::ExtractedDocument {
    /// Run the canonical derivation pipeline with `OutputFormat::Plain` and no document
    /// structure derivation. Used at FFI/trait-bridge boundaries where the rich
    /// `InternalDocument` must be converted to the public `ExtractedDocument` shape.
    fn from(doc: InternalDocument) -> Self {
        crate::extraction::derive::derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain)
    }
}

impl InternalDocument {
    /// Create a new empty document with the given source format.
    pub fn new(source_format: impl Into<String>) -> Self {
        Self {
            elements: Vec::new(),
            relationships: Vec::new(),
            source_format: source_format.into(),
            metadata: Metadata::default(),
            images: Vec::new(),
            tables: Vec::new(),
            diagrams: Vec::new(),
            uris: Vec::new(),
            uris_dropped: 0,
            children: None,
            mime_type: "application/octet-stream".to_string(),
            processing_warnings: Vec::new(),
            ocr_page_failures: Vec::new(),
            annotations: None,
            prebuilt_pages: None,
            pre_rendered_content: None,
            prebuilt_ocr_elements: None,
            llm_usage: None,
            revisions: None,
            ocr_text_only: false,
            append_ocr_text: false,
            escape_markdown: true,
            include_watermarks: false,
            page_marker_format: None,
            table_anchors: false,
            form_fields: Vec::new(),
            formulas: Vec::new(),
            recorded_formulas: Vec::new(),
            #[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
            ocr_coordinate_frame: None,
            #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
            ocr_text_claimed_by_tables: false,
        }
    }

    /// Push an element and return its index.
    pub fn push_element(&mut self, element: InternalElement) -> u32 {
        let idx = self.elements.len() as u32;
        self.elements.push(element);
        idx
    }

    /// Push a relationship.
    pub fn push_relationship(&mut self, relationship: Relationship) {
        self.relationships.push(relationship);
    }

    /// Push a table and return its index (for use in `ElementKind::Table`).
    pub fn push_table(&mut self, table: Table) -> u32 {
        let idx = self.tables.len() as u32;
        self.tables.push(table);
        idx
    }

    /// Push an image and return its index (for use in `ElementKind::Image`).
    pub fn push_image(&mut self, image: ExtractedImage) -> u32 {
        let idx = self.images.len() as u32;
        self.images.push(image);
        idx
    }

    /// Maximum number of URIs to collect per document (DoS prevention).
    pub(crate) const MAX_URIS: usize = 100_000;

    /// Push a URI discovered during extraction.
    ///
    /// URIs beyond the `MAX_URIS` cap are dropped to prevent unbounded memory
    /// growth, and counted in [`Self::uris_dropped`] so the derivation step can
    /// tell the caller the list was truncated (#76).
    pub fn push_uri(&mut self, uri: super::uri::ExtractedUri) {
        if self.uris.len() < Self::MAX_URIS {
            self.uris.push(uri);
        } else {
            self.uris_dropped += 1;
        }
    }

    /// Concatenate all element text into a single string, separated by newlines.
    #[cfg(all(test, any(feature = "html", feature = "hwpx")))]
    pub(crate) fn content(&self) -> String {
        self.elements
            .iter()
            .map(|e| e.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// A single element in the internal flat document.
///
/// Elements are appended in reading order during extraction. The `depth` field
/// and optional container markers enable tree reconstruction in the derivation step.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InternalElement {
    /// Deterministic identifier.
    pub id: InternalElementId,

    /// What kind of content this element represents.
    pub kind: ElementKind,

    /// Primary text content. Empty for non-text elements (images, page breaks).
    pub text: String,

    /// Nesting depth (0 = root level).
    ///
    /// Extractors set this based on heading level, list indent, blockquote depth, etc.
    /// The tree derivation step uses depth changes to reconstruct parent-child relationships.
    pub depth: u16,

    /// Page number (1-indexed). `None` for non-paginated formats.
    pub page: Option<u32>,

    /// Bounding box in document coordinates.
    pub bbox: Option<BoundingBox>,

    /// Content layer classification (Body, Header, Footer, Footnote).
    pub layer: ContentLayer,

    /// Inline annotations (formatting, links) on this element's text content.
    /// Byte-range based, reuses the existing `TextAnnotation` type.
    pub annotations: Vec<TextAnnotation>,

    /// Format-specific key-value attributes.
    /// Used for CSS classes, LaTeX env names, slide layout names, etc.
    pub attributes: Option<AHashMap<String, String>>,

    /// Optional anchor/key for this element.
    ///
    /// Used by the relationship resolver to match references to targets.
    /// Examples: heading slug `"introduction"`, footnote label `"fn1"`,
    /// citation key `"smith2024"`, figure label `"fig:diagram"`.
    pub anchor: Option<String>,

    /// OCR bounding geometry (rectangle or quadrilateral).
    pub ocr_geometry: Option<OcrBoundingGeometry>,

    /// OCR confidence scores (detection + recognition).
    pub ocr_confidence: Option<OcrConfidence>,

    /// OCR rotation metadata.
    pub ocr_rotation: Option<OcrRotation>,
}

impl InternalElement {
    /// Create a simple text element with minimal fields.
    pub fn text(kind: ElementKind, text: impl Into<String>, depth: u16) -> Self {
        let text = text.into();
        let id = InternalElementId::generate(kind.discriminant(), &text, None, 0);
        Self {
            id,
            kind,
            text,
            depth,
            page: None,
            bbox: None,
            layer: ContentLayer::Body,
            annotations: Vec::new(),
            attributes: None,
            anchor: None,
            ocr_geometry: None,
            ocr_confidence: None,
            ocr_rotation: None,
        }
    }

    /// Set the page number.
    #[cfg(any(
        feature = "ocr",
        feature = "office",
        feature = "pdf",
        paddle_ocr,
        feature = "xml",
        feature = "hwpx",
        feature = "quality",
        feature = "chunking",
        test
    ))]
    #[allow(dead_code)]
    pub(crate) fn with_page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Set the bounding box.
    #[cfg(feature = "office")]
    pub(crate) fn with_bbox(mut self, bbox: BoundingBox) -> Self {
        self.bbox = Some(bbox);
        self
    }

    /// Set the content layer.
    #[cfg(all(
        test,
        any(feature = "ocr", feature = "pdf", paddle_ocr, feature = "xml", feature = "office")
    ))]
    pub(crate) fn with_layer(mut self, layer: ContentLayer) -> Self {
        self.layer = layer;
        self
    }

    /// Set the anchor key.
    #[cfg(test)]
    pub(crate) fn with_anchor(mut self, anchor: impl Into<String>) -> Self {
        self.anchor = Some(anchor.into());
        self
    }

    /// Set attributes.
    #[cfg(any(feature = "xml", feature = "hwpx"))]
    pub(crate) fn with_attributes(mut self, attributes: AHashMap<String, String>) -> Self {
        self.attributes = Some(attributes);
        self
    }

    /// Regenerate the ID with the correct index (call after pushing to the document).
    #[cfg(any(
        feature = "ocr",
        feature = "xml",
        feature = "archives",
        feature = "hwpx",
        // The only bare-`ocr-pipeline` caller lives in `extractors::pdf::ocr`, so gate on
        // pdf+ocr-pipeline. `ocr-wasm` enables ocr-pipeline without pdf and has no caller. ~keep
        all(feature = "pdf", feature = "ocr-pipeline")
    ))]
    pub(crate) fn with_index(mut self, index: u32) -> Self {
        self.id = InternalElementId::generate(self.kind.discriminant(), &self.text, self.page, index);
        self
    }

    /// Mark an image element so whole-page OCR can replace its nested OCR text
    /// without removing the image placeholder or mutating the public image data.
    ///
    /// Only called by the PDF OCR merge planner (`extractors::pdf::ocr`); dead in
    /// builds that enable `ocr`/`ocr-pipeline` without `pdf`. ~keep
    #[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
    pub(crate) fn suppress_image_ocr_rendering(&mut self) {
        self.attributes
            .get_or_insert_with(AHashMap::new)
            .insert(SUPPRESS_IMAGE_OCR_RENDER_ATTRIBUTE.to_string(), "true".to_string());
    }

    /// Whether renderers should include nested OCR text for this image element.
    pub(crate) fn should_render_image_ocr(&self) -> bool {
        !self
            .attributes
            .as_ref()
            .is_some_and(|attributes| attributes.contains_key(SUPPRESS_IMAGE_OCR_RENDER_ATTRIBUTE))
    }

    /// An anchor for the OCR text of the picture `image_index`, where no placeholder is wanted.
    ///
    /// It is an empty paragraph, so it renders as nothing until the pipeline gives it the
    /// text of the picture, or removes it when the picture gave none. An image element in
    /// its place would render as a placeholder wherever the pipeline did not run. ~keep
    #[cfg(all(any(feature = "pdf", feature = "office"), feature = "ocr", feature = "tokio-runtime"))]
    pub(crate) fn image_ocr_text_anchor(image_index: u32, page: Option<u32>) -> Self {
        let mut anchor = Self::text(ElementKind::Paragraph, "", 0);
        anchor.page = page;
        anchor.attributes = Some(AHashMap::from_iter([(
            IMAGE_OCR_TEXT_ANCHOR_ATTRIBUTE.to_string(),
            image_index.to_string(),
        )]));
        anchor
    }

    /// The picture this element is the OCR text anchor of, if it is one.
    #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
    pub(crate) fn image_ocr_text_anchor_index(&self) -> Option<u32> {
        self.attributes
            .as_ref()?
            .get(IMAGE_OCR_TEXT_ANCHOR_ATTRIBUTE)?
            .parse()
            .ok()
    }

    /// Attach a `ListItem` element's literal source marker text (e.g. `"B."`,
    /// `"(a)"`, `"iv."`).
    ///
    /// PDF structure assembly and image layout extraction attach the marker through
    /// `InternalDocumentBuilder::set_list_item_source_label` after removing it from text.
    pub(crate) fn set_list_item_source_label(&mut self, label: impl Into<String>) {
        let label = label.into();
        if label.is_empty() {
            return;
        }
        self.attributes
            .get_or_insert_with(AHashMap::new)
            .insert(LIST_ITEM_SOURCE_LABEL_ATTRIBUTE.to_string(), label);
    }

    /// The literal source list-marker text, if one was captured (see
    /// [`set_list_item_source_label`](Self::set_list_item_source_label)).
    ///
    /// `None` for list items whose marker text was not confidently recovered -- renderers fall back to
    /// `ElementKind::ListItem::ordered`'s synthesized sequence position in
    /// that case, exactly as they did before this attribute existed.
    pub(crate) fn list_item_source_label(&self) -> Option<&str> {
        self.attributes
            .as_ref()?
            .get(LIST_ITEM_SOURCE_LABEL_ATTRIBUTE)
            .map(String::as_str)
    }

    /// Attributes safe to expose through the public document structure.
    pub(crate) fn public_attributes(&self) -> Option<std::collections::HashMap<String, String>> {
        let original = self.attributes.as_ref()?;
        let attributes: std::collections::HashMap<String, String> = original
            .iter()
            .filter(|(key, _)| !is_internal_only_attribute(key))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        if attributes.is_empty() && !original.is_empty() {
            None
        } else {
            Some(attributes)
        }
    }

    /// Record the measured dominant font size (in points) for this element.
    ///
    /// Non-finite or non-positive values are ignored rather than stored, since a
    /// bogus measurement is worse than falling back to the default at read time.
    #[cfg(feature = "pdf")]
    pub(crate) fn set_measured_font_size(&mut self, font_size_pt: f32) {
        if !font_size_pt.is_finite() || font_size_pt <= 0.0 {
            return;
        }
        self.attributes
            .get_or_insert_with(AHashMap::new)
            .insert(MEASURED_FONT_SIZE_ATTRIBUTE.to_string(), font_size_pt.to_string());
    }

    /// The measured font size set via [`set_measured_font_size`](Self::set_measured_font_size).
    ///
    /// The attribute map survives a cache round-trip as plain strings, so the stored value is
    /// treated as untrusted and re-validated as finite and positive before being returned.
    #[cfg(feature = "pdf")]
    pub(crate) fn measured_font_size(&self) -> Option<f32> {
        let raw = self.attributes.as_ref()?.get(MEASURED_FONT_SIZE_ATTRIBUTE)?;
        let value: f32 = raw.parse().ok()?;
        (value.is_finite() && value > 0.0).then_some(value)
    }
}

/// Attribute keys that are internal plumbing and must never reach the public
/// `DocumentNode::attributes` surface.
fn is_internal_only_attribute(key: &str) -> bool {
    key == SUPPRESS_IMAGE_OCR_RENDER_ATTRIBUTE
        || key == IMAGE_OCR_TEXT_ANCHOR_ATTRIBUTE
        || key == MEASURED_FONT_SIZE_ATTRIBUTE
        || key == NATIVE_TABLE_GRID_ATTRIBUTE
}

/// [`InternalElement::list_item_source_label`], for renderers that flatten an
/// element into a `(kind, text, .., attributes)` tuple before dispatch
/// (`rendering::comrak_bridge`) rather than keeping `&InternalElement` around.
pub(crate) fn list_item_source_label_from_attributes(attributes: Option<&AHashMap<String, String>>) -> Option<&str> {
    attributes?.get(LIST_ITEM_SOURCE_LABEL_ATTRIBUTE).map(String::as_str)
}

/// Semantic role of an internal element.
///
/// Superset of [`NodeContent`](super::document_structure::NodeContent) variants
/// plus OCR and container markers.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementKind {
    /// Document title.
    Title,
    /// Section heading with level (1-6).
    Heading {
        /// Heading depth (1 = h1, 2 = h2, …, 6 = h6).
        level: u8,
    },
    /// Body text paragraph.
    Paragraph,
    /// List item. `ordered` indicates numbered vs bulleted.
    ListItem {
        /// `true` for ordered (numbered) lists; `false` for unordered (bullet) lists.
        ordered: bool,
    },
    /// Code block. Language stored in element attributes.
    Code,
    /// Mathematical formula / equation.
    Formula,
    /// Footnote content (the definition, not the reference marker).
    FootnoteDefinition,
    /// Footnote reference marker in body text.
    FootnoteRef,
    /// Comment content (the definition, not the reference marker). See
    /// [`NodeContent::Comment`](super::document_structure::NodeContent::Comment).
    CommentDefinition,
    /// Comment reference marker in body text.
    CommentRef,
    /// Citation or bibliographic reference.
    Citation,
    /// Presentation slide container.
    Slide {
        /// 1-indexed slide number.
        number: u32,
    },
    /// Definition list term.
    DefinitionTerm,
    /// Definition list description.
    DefinitionDescription,
    /// Admonition / callout (note, warning, tip, etc.). Kind stored in attributes.
    Admonition,
    /// Raw block preserved verbatim. Format stored in attributes.
    RawBlock,
    /// Structured metadata block (frontmatter, email headers).
    MetadataBlock,

    /// Start of a list container.
    ListStart {
        /// `true` for ordered (numbered) lists; `false` for unordered (bullet) lists.
        ordered: bool,
    },
    /// End of a list container.
    ListEnd,
    /// Start of a block quote.
    QuoteStart,
    /// End of a block quote.
    QuoteEnd,
    /// Start of a generic group/section.
    GroupStart,
    /// End of a generic group/section.
    GroupEnd,

    /// Table reference. `table_index` is an index into `InternalDocument::tables`.
    Table {
        /// Index into `InternalDocument::tables` for the referenced table.
        table_index: u32,
    },
    /// Image reference. `image_index` is an index into `InternalDocument::images`.
    Image {
        /// Index into `InternalDocument::images` for the referenced image.
        image_index: u32,
    },
    /// Page break marker.
    PageBreak,

    /// OCR-detected text at a given hierarchical level.
    OcrText {
        /// Hierarchical level (word, line, paragraph, block) of this OCR element.
        level: OcrElementLevel,
    },
}

impl ElementKind {
    /// Get a stable string discriminant for ID generation.
    pub(crate) fn discriminant(&self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Heading { .. } => "heading",
            Self::Paragraph => "paragraph",
            Self::ListItem { .. } => "list_item",
            Self::Code => "code",
            Self::Formula => "formula",
            Self::FootnoteDefinition => "footnote_definition",
            Self::FootnoteRef => "footnote_ref",
            Self::CommentDefinition => "comment_definition",
            Self::CommentRef => "comment_ref",
            Self::Citation => "citation",
            Self::Slide { .. } => "slide",
            Self::DefinitionTerm => "definition_term",
            Self::DefinitionDescription => "definition_description",
            Self::Admonition => "admonition",
            Self::RawBlock => "raw_block",
            Self::MetadataBlock => "metadata_block",
            Self::ListStart { .. } => "list_start",
            Self::ListEnd => "list_end",
            Self::QuoteStart => "quote_start",
            Self::QuoteEnd => "quote_end",
            Self::GroupStart => "group_start",
            Self::GroupEnd => "group_end",
            Self::Table { .. } => "table",
            Self::Image { .. } => "image",
            Self::PageBreak => "page_break",
            Self::OcrText { .. } => "ocr_text",
        }
    }

    /// Returns true if this is a container start marker.
    pub(crate) fn is_container_start(&self) -> bool {
        matches!(self, Self::ListStart { .. } | Self::QuoteStart | Self::GroupStart)
    }

    /// Returns true if this is a container end marker.
    pub(crate) fn is_container_end(&self) -> bool {
        matches!(self, Self::ListEnd | Self::QuoteEnd | Self::GroupEnd)
    }

    /// Returns the matching end marker for a container start, if applicable.
    #[cfg(test)]
    pub(crate) fn matching_end(&self) -> Option<ElementKind> {
        match self {
            Self::ListStart { .. } => Some(Self::ListEnd),
            Self::QuoteStart => Some(Self::QuoteEnd),
            Self::GroupStart => Some(Self::GroupEnd),
            _ => None,
        }
    }
}

/// A relationship between two elements in the document.
///
/// During extraction, targets may be unresolved keys (`RelationshipTarget::Key`).
/// The derivation step resolves these to indices using the element anchor index.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    /// Index of the source element in `InternalDocument::elements`.
    pub source: u32,

    /// Target of the relationship (resolved index or unresolved key).
    pub target: RelationshipTarget,

    /// Semantic kind of the relationship.
    pub kind: RelationshipKind,
}

/// Target of a relationship — either a resolved element index or an unresolved key.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RelationshipTarget {
    /// Resolved: index into `InternalDocument::elements`.
    Index(u32),
    /// Unresolved: key to be matched against element anchors during derivation.
    Key(String),
}

pub use super::document_structure::RelationshipKind;

const _: () = {
    #[allow(dead_code)]
    fn assert_send_sync<T: Send + Sync>() {}
    #[allow(dead_code)]
    fn _check() {
        assert_send_sync::<InternalDocument>();
        assert_send_sync::<InternalElement>();
    }
};

#[cfg(test)]
mod tests;
