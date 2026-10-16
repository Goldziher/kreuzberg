//! Structured document model types.
//!
//! This module provides a hierarchical, tree-based representation of document content.
//! It uses a flat `Vec<DocumentNode>` with index-based parent/child references for
//! efficient traversal and compact serialization.
//!
//! # Design
//!
//! - **Flat array storage**: All nodes stored in `Vec<DocumentNode>` in reading order
//! - **Index-based references**: `NodeIndex(u32)` for parent/child links
//! - **Tagged enum content**: `NodeContent` with `#[serde(tag = "node_type")]`
//! - **Content layer classification**: Each node tagged as Body, Header, Footer, or Footnote
//! - **Deterministic IDs**: `NodeId` generated from content hash for diffing/caching

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::extraction::BoundingBox;

/// Newtype for node indices into the `DocumentStructure::nodes` array.
///
/// Uses `u32` for cross-platform consistency (WASM is 32-bit) and to avoid
/// confusion with page numbers or other `usize` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "api", schema(value_type = u32))]
pub struct NodeIndex(pub u32);

/// Deterministic node identifier.
///
/// Generated from a hash of `node_type + text + page`. The same document
/// always produces the same IDs, making them useful for diffing, caching,
/// and external references. Wraps a `String` (public field, mirroring
/// [`NodeIndex`]'s wrapper pattern) so bindings can treat it as a plain
/// newtype rather than requiring a lossy fallback conversion.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "api", schema(value_type = String))]
pub struct NodeId(pub String);

impl NodeId {
    /// Generate a deterministic `NodeId` from node content.
    ///
    /// Uses wrapping multiplication hashing on the node type discriminant,
    /// text content, page number, and node index to produce a stable hex identifier.
    /// The index parameter ensures uniqueness for duplicate content on the same page.
    ///
    /// # Parameters
    ///
    /// - `node_type`: The node type discriminant (e.g., "paragraph", "heading")
    /// - `text`: The text content of the node
    /// - `page`: The page number (None becomes u64::MAX for hashing)
    /// - `index`: The position of this node in the document's nodes array
    pub(crate) fn generate(node_type: &str, text: &str, page: Option<u32>, index: u32) -> Self {
        let type_hash = node_type
            .bytes()
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));

        let text_hash = text
            .bytes()
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));

        let page_hash = page.map(|p| p as u64).unwrap_or(u64::MAX);

        let combined = type_hash
            .wrapping_mul(65599)
            .wrapping_add(text_hash)
            .wrapping_mul(65599)
            .wrapping_add(page_hash)
            .wrapping_mul(65599)
            .wrapping_add(index as u64);

        Self(format!("node-{:x}", combined))
    }
}

impl AsRef<str> for NodeId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Top-level structured document representation.
///
/// A flat array of nodes with index-based parent/child references forming a tree.
/// Root-level nodes have `parent: None`. Use `body_roots()` and `furniture_roots()`
/// to iterate over top-level content by layer.
///
/// # Validation
///
/// Call `validate()` after construction to verify all node indices are in bounds
/// and parent-child relationships are bidirectionally consistent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "api", schema(no_recursion))]
pub struct DocumentStructure {
    /// All nodes in document/reading order.
    pub nodes: Vec<DocumentNode>,

    /// Origin format identifier (e.g. "docx", "pptx", "html", "pdf").
    ///
    /// Allows renderers to apply format-aware heuristics when converting
    /// the document tree to output formats.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_format: Option<String>,

    /// Resolved relationships between nodes (footnote refs, citations, anchor links, etc.).
    ///
    /// Populated during derivation from the internal document representation.
    /// Empty when no relationships are detected.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub relationships: Vec<DocumentRelationship>,

    /// Sorted, deduplicated list of node type names present in this document.
    ///
    /// Each value is the snake_case `node_type` tag of the corresponding
    /// [`NodeContent`] variant (e.g. `"paragraph"`, `"heading"`, `"table"`, …).
    ///
    /// Computed from `nodes` via [`DocumentStructure::finalize_node_types`].
    /// Empty until that method is called (internal construction paths call it
    /// at the end of derivation).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub node_types: Vec<String>,
}

impl DocumentStructure {
    /// Compute and populate the `node_types` field from the current `nodes`.
    ///
    /// Call this after all nodes have been added to the structure. Internal
    /// construction paths (builder, derivation) call this automatically.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use xberg::types::document_structure::{DocumentStructure, DocumentNode, NodeContent};
    ///
    /// let mut structure = DocumentStructure {
    ///     nodes: vec![DocumentNode {
    ///         id: String::new(),
    ///         content: NodeContent::Paragraph { text: "Hello".into() },
    ///         parent: None,
    ///         children: vec![],
    ///         content_layer: Default::default(),
    ///         page: None,
    ///         page_end: None,
    ///         bbox: None,
    ///         annotations: vec![],
    ///         attributes: None,
    ///     }],
    ///     source_format: None,
    ///     relationships: vec![],
    ///     node_types: vec![],
    /// };
    /// structure.finalize_node_types();
    /// assert!(structure.node_types.contains(&"paragraph".to_string()));
    /// ```
    pub fn finalize_node_types(&mut self) {
        let mut types: Vec<&'static str> = self.nodes.iter().map(|n| n.content.node_type_name()).collect();
        types.sort_unstable();
        types.dedup();
        self.node_types = types.into_iter().map(|s| s.to_string()).collect();
    }

    /// Maps a document node to its byte-offset span within a document's
    /// rendered text content (the string that chunking operates on, e.g.
    /// Markdown or plain-text output).
    ///
    /// **Deprecated since 1.1.0; scheduled for removal in 2.0.**
    ///
    /// Nodes carry position information for the *source* document (`page`,
    /// `bbox`) but not offsets into *rendered* output — rendering is a
    /// separate step (format-specific renderers under `core/pipeline`) that
    /// does not currently track which byte ranges of its output came from
    /// which `NodeIndex`. Always returns `None`.
    ///
    /// This is the seam `ChunkMetadata::node_ids` population (tracked under #1296) is expected
    /// to build on; implementing it requires either threading node provenance through the
    /// renderers, or re-deriving node spans by re-scanning rendered output for node text with
    /// page/order disambiguation.
    ///
    /// `ChunkMetadata::page_spans` (#1295) does not depend on this method: it derives its page
    /// numbers from the existing byte-range-to-page boundary mapping (the same one used for
    /// `first_page`/`last_page`, see `chunking::boundaries::calculate_page_spans`) and fills in
    /// bounding boxes via a page-scoped textual containment check against node text (see
    /// `chunking::page_spans::populate_page_span_bboxes`), without requiring an exact node ->
    /// byte-offset mapping.
    ///
    /// # Parameters
    ///
    /// - `node_index`: index into [`DocumentStructure::nodes`].
    ///
    /// Not bound to language bindings (`alef(skip)`): the tuple return type
    /// is not FFI-friendly, and the method is a placeholder with no behavior
    /// to expose yet. A binding-facing surface can be added once #1296
    /// implements real offset resolution.
    #[deprecated(since = "1.1.0", note = "Always returns None; scheduled for removal in 2.0")]
    #[cfg_attr(alef, alef(skip))]
    #[must_use]
    pub fn node_rendered_offset(&self, _node_index: NodeIndex) -> Option<(usize, usize)> {
        // TODO(#1296): implement real node -> rendered-offset mapping.
        None
    }
}

/// A resolved relationship between two nodes in the document tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct DocumentRelationship {
    /// Source node index (the referencing node).
    pub source: NodeIndex,
    /// Target node index (the referenced node).
    pub target: NodeIndex,
    /// Semantic kind of the relationship.
    pub kind: RelationshipKind,
}

/// Semantic kind of a relationship between document elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum RelationshipKind {
    /// Footnote marker -> footnote definition.
    FootnoteReference,
    /// Citation marker -> bibliography entry.
    CitationReference,
    /// Internal anchor link (`#id`) -> target heading/element.
    InternalLink,
    /// Caption paragraph -> figure/table it describes.
    Caption,
    /// Label -> labeled element (HTML `<label for>`, LaTeX `\label{}`).
    Label,
    /// TOC entry -> target section.
    TocEntry,
    /// Cross-reference (LaTeX `\ref{}`, DOCX cross-reference field).
    CrossReference,
}

/// A single node in the document tree.
///
/// Each node has deterministic `id`, typed `content`, optional `parent`/`children`
/// for tree structure, and metadata like page number, bounding box, and content layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct DocumentNode {
    /// Deterministic identifier (hash of node type + text + page + position).
    ///
    /// Stable and unique within a single extraction response: every internal
    /// construction path threads the node's position (its index in
    /// `DocumentStructure::nodes`) into the hash, so identical
    /// `(node_type, text, page)` tuples at different positions never collide.
    /// Always serialised — `ChunkMetadata::node_ids` references it to join
    /// chunks back to the nodes they were derived from.
    /// `#[serde(default)]` covers the missing-field case on inbound JSON
    /// (e.g. documents serialised before this field existed).
    #[serde(default)]
    pub id: String,

    /// Node content — tagged enum, type-specific data only.
    pub content: NodeContent,

    /// Parent node index (`None` = root-level node).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "api", schema(nullable = false))]
    pub parent: Option<NodeIndex>,

    /// Child node indices in reading order.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<NodeIndex>,

    /// Content layer classification.
    ///
    /// Always serialised — Kotlin-Android (and any other typed binding) treats
    /// the field as non-nullable, so omitting it from the JSON wire would
    /// break consumer deserialisation.  `#[serde(default)]` covers the
    /// missing-field case on inbound JSON.
    #[serde(default)]
    pub content_layer: ContentLayer,

    /// Page number where this node starts (1-indexed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,

    /// Page number where this node ends (for multi-page tables/sections).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_end: Option<u32>,

    /// Bounding box in document coordinates.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "api", schema(nullable = false))]
    pub bbox: Option<BoundingBox>,

    /// Inline annotations (formatting, links) on this node's text content.
    ///
    /// Only meaningful for text-carrying nodes; empty for containers.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub annotations: Vec<TextAnnotation>,

    /// Format-specific key-value attributes.
    ///
    /// Extensible bag for miscellaneous data without a dedicated typed field: CSS classes,
    /// LaTeX environment names, Excel cell formulas, slide layout names, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<HashMap<String, String>>,
}

/// Content layer classification for document nodes.
///
/// Replaces separate body/furniture arrays with per-node granularity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ContentLayer {
    /// Main document body content.
    #[default]
    Body,
    /// Page/section header (running header).
    Header,
    /// Page/section footer (running footer).
    Footer,
    /// Footnote content.
    Footnote,
}

/// Tagged enum for node content. Each variant carries only type-specific data.
///
/// Uses `#[serde(tag = "node_type")]` to avoid "type" keyword collision in
/// Go/Java/TypeScript bindings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(tag = "node_type", rename_all = "snake_case")]
pub enum NodeContent {
    /// Document title.
    Title {
        /// The title text content.
        text: String,
    },

    /// Section heading with level (1-6).
    Heading {
        /// Heading depth (1 = h1, 2 = h2, …, 6 = h6).
        level: u8,
        /// The heading text content.
        text: String,
    },

    /// Body text paragraph.
    Paragraph {
        /// The paragraph text content.
        text: String,
    },

    /// List container — children are `ListItem` nodes.
    List {
        /// `true` for ordered (numbered) lists; `false` for unordered (bullet) lists.
        ordered: bool,
    },

    /// Individual list item.
    ListItem {
        /// The list item text content.
        text: String,
    },

    /// Table with structured cell grid.
    Table {
        /// Structured grid of table cells.
        grid: TableGrid,
    },

    /// Image reference.
    Image {
        /// Optional alt text or caption describing the image.
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// Index into the parent `ExtractedDocument::images` list.
        #[serde(skip_serializing_if = "Option::is_none")]
        image_index: Option<u32>,
        /// Source URL or path of the image (from `<img src="...">` or `![](src)`).
        #[serde(skip_serializing_if = "Option::is_none")]
        src: Option<String>,
    },

    /// Code block.
    Code {
        /// The source code text content.
        text: String,
        /// Programming language identifier (e.g. `"rust"`, `"python"`).
        #[serde(skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },

    /// Block quote — container, children carry the quoted content.
    Quote,

    /// Mathematical formula / equation.
    Formula {
        /// The formula source text (LaTeX or plain mathematical notation).
        text: String,
    },

    /// Footnote reference content.
    Footnote {
        /// The footnote body text.
        text: String,
    },

    /// Reviewer/editor comment content (e.g. DOCX comments).
    ///
    /// Distinct from [`NodeContent::Footnote`] (xberg-io/xberg#300): comments and
    /// footnotes both reach the internal document via a marker/definition pair, but
    /// a consumer needs to tell a reviewer comment apart from an authored footnote.
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    Comment {
        /// The comment body text.
        text: String,
    },

    /// Logical grouping container (section, key-value area).
    ///
    /// `heading_level` + `heading_text` capture the section heading directly
    /// rather than relying on a first-child positional convention.
    Group {
        /// Optional display label for the group (e.g. section name).
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        /// Heading level of the section heading that opened this group (1-6).
        #[serde(skip_serializing_if = "Option::is_none")]
        heading_level: Option<u8>,
        /// Text of the section heading that opened this group.
        #[serde(skip_serializing_if = "Option::is_none")]
        heading_text: Option<String>,
    },

    /// Page break marker.
    PageBreak,

    /// Presentation slide container — children are the slide's content nodes.
    Slide {
        /// 1-indexed slide number.
        number: u32,
        /// Slide title text, if present.
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },

    /// Definition list container — children are `DefinitionItem` nodes.
    DefinitionList,

    /// Individual definition list entry with term and definition.
    DefinitionItem {
        /// The term being defined.
        term: String,
        /// The definition or description of the term.
        definition: String,
    },

    /// Citation or bibliographic reference.
    Citation {
        /// Citation key (e.g. BibTeX key or reference ID).
        key: String,
        /// Formatted citation text as it appears in the document.
        text: String,
    },

    /// Admonition / callout container (note, warning, tip, etc.).
    ///
    /// Children carry the admonition body content.
    Admonition {
        /// Kind of admonition (e.g. "note", "warning", "tip", "danger").
        kind: String,
        /// Optional explicit title overriding the default kind label.
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },

    /// Raw block preserved verbatim from the source format.
    ///
    /// Used for content that cannot be mapped to a semantic node type
    /// (e.g. JSX in MDX, raw LaTeX in markdown, embedded HTML).
    RawBlock {
        /// Source format identifier (e.g. "html", "latex", "jsx").
        format: String,
        /// Verbatim source content in the specified format.
        content: String,
    },

    /// Structured metadata block (email headers, YAML frontmatter, etc.).
    MetadataBlock {
        /// Key-value pairs extracted from the metadata block.
        entries: Vec<super::metadata::KeyValueAttribute>,
    },
}

impl NodeContent {
    /// Return the snake_case type name for this node content variant.
    ///
    /// Matches the `node_type` serde tag value (i.e. `rename_all = "snake_case"`).
    pub fn node_type_name(&self) -> &'static str {
        match self {
            Self::Title { .. } => "title",
            Self::Heading { .. } => "heading",
            Self::Paragraph { .. } => "paragraph",
            Self::List { .. } => "list",
            Self::ListItem { .. } => "list_item",
            Self::Table { .. } => "table",
            Self::Image { .. } => "image",
            Self::Code { .. } => "code",
            Self::Quote => "quote",
            Self::Formula { .. } => "formula",
            Self::Footnote { .. } => "footnote",
            Self::Comment { .. } => "comment",
            Self::Group { .. } => "group",
            Self::PageBreak => "page_break",
            Self::Slide { .. } => "slide",
            Self::DefinitionList => "definition_list",
            Self::DefinitionItem { .. } => "definition_item",
            Self::Citation { .. } => "citation",
            Self::Admonition { .. } => "admonition",
            Self::RawBlock { .. } => "raw_block",
            Self::MetadataBlock { .. } => "metadata_block",
        }
    }
}

/// Structured table grid with cell-level metadata.
///
/// Stores row/column dimensions and a flat list of cells with position info.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct TableGrid {
    /// Number of rows in the table.
    pub rows: u32,
    /// Number of columns in the table.
    pub cols: u32,
    /// All cells in row-major order.
    pub cells: Vec<GridCell>,
}

/// Individual grid cell with position and span metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct GridCell {
    /// Cell text content.
    pub content: String,
    /// Zero-indexed row position.
    pub row: u32,
    /// Zero-indexed column position.
    pub col: u32,
    /// Number of rows this cell spans.
    #[serde(default = "default_span")]
    pub row_span: u32,
    /// Number of columns this cell spans.
    #[serde(default = "default_span")]
    pub col_span: u32,
    /// Whether this is a header cell.
    #[serde(default)]
    pub is_header: bool,
    /// Bounding box for this cell (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "api", schema(nullable = false))]
    pub bbox: Option<BoundingBox>,
    /// Outline level (1-6) of the heading style this cell's text carries, when it has one.
    ///
    /// A DOCX banner row -- row 0, one cell spanning the grid, styled `Heading1`..`Heading6` --
    /// is what Word's navigation pane and a `TOC` field treat as the document's outline, but as
    /// a table cell it reached consumers as anonymous text (GH#1587). `content` is deliberately
    /// left as the bare cell text: prefixing it with `#` would put a markdown heading inside a
    /// table cell, which is invalid where it lands and changes text every existing consumer
    /// already reads. This field is the signal instead, so a caller can decide for itself
    /// whether a `heading 2` in a banner row is a section title or a column label. ~keep
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading_level: Option<u8>,
    /// Human-readable name of the paragraph style applied to this cell's text (`heading 2`).
    ///
    /// Carries the style even when it resolves to no outline level, so a caller can key on a
    /// named style this crate does not map to a heading. See [`GridCell::heading_level`]. ~keep
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<String>,
}

fn default_span() -> u32 {
    1
}

/// Inline text annotation — byte-range based formatting and links.
///
/// Annotations reference byte offsets into the node's text content,
/// enabling precise identification of formatted regions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct TextAnnotation {
    /// Start byte offset in the node's text content (inclusive).
    pub start: u32,
    /// End byte offset in the node's text content (exclusive).
    pub end: u32,
    /// Annotation type.
    pub kind: AnnotationKind,
}

/// Types of inline text annotations.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(tag = "annotation_type", rename_all = "snake_case")]
pub enum AnnotationKind {
    /// Bold (strong) text formatting.
    #[default]
    Bold,
    /// Italic (emphasis) text formatting.
    Italic,
    /// Underlined text.
    Underline,
    /// Strikethrough text.
    Strikethrough,
    /// Inline code span.
    Code,
    /// Subscript text.
    Subscript,
    /// Superscript text.
    Superscript,
    /// Hyperlink annotation.
    Link {
        /// Hyperlink target URL.
        url: String,
        /// Optional link title attribute.
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    /// Highlighted text (PDF highlights, HTML `<mark>`).
    Highlight,
    /// Text color (CSS-compatible value, e.g. "#ff0000", "red").
    Color {
        /// CSS-compatible color value (e.g. `"#ff0000"`, `"red"`).
        value: String,
    },
    /// Font size with units (e.g. "12pt", "1.2em", "16px").
    FontSize {
        /// Font size including unit (e.g. `"12pt"`, `"1.2em"`, `"16px"`).
        value: String,
    },
    /// Extensible annotation for format-specific styling.
    Custom {
        /// Name of the custom annotation kind.
        name: String,
        /// Optional value or parameter for the annotation.
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
    },
}

impl AnnotationKind {
    /// Mark a LaTeX equation without introducing a new enum variant.
    ///
    /// The annotation's byte range includes its `$...$` delimiters, or `$$...$$`
    /// when `display` is true. Text remains available unchanged to plain-text,
    /// chunking, and search consumers. Only explicitly annotated spans are math.
    ///
    /// This uses the reserved `Custom` name `xberg.math` and value `inline` or
    /// `display`: `{"annotation_type":"custom","name":"xberg.math","value":"inline"}`.
    /// Existing bindings can construct and inspect that same Custom value.
    pub fn math(display: bool) -> Self {
        Self::Custom {
            name: "xberg.math".to_string(),
            value: Some(if display { "display" } else { "inline" }.to_string()),
        }
    }

    /// Return the display mode of an explicitly marked math annotation.
    ///
    /// Only the exact reserved name and values produced by [`Self::math`] are
    /// recognized. Other custom annotations return `None`.
    pub fn math_display(&self) -> Option<bool> {
        match self {
            Self::Custom { name, value } if name == "xberg.math" => match value.as_deref() {
                Some("inline") => Some(false),
                Some("display") => Some(true),
                _ => None,
            },
            _ => None,
        }
    }
}

/// Convert PDF hierarchy's `(f32, f32, f32, f32)` bounding box to canonical `BoundingBox`.
///
/// The tuple order is `(left, top, right, bottom)` matching the PDF coordinate convention.
impl From<(f32, f32, f32, f32)> for BoundingBox {
    fn from((left, top, right, bottom): (f32, f32, f32, f32)) -> Self {
        BoundingBox {
            x0: left as f64,
            y0: top as f64,
            x1: right as f64,
            y1: bottom as f64,
        }
    }
}

impl Default for NodeContent {
    fn default() -> Self {
        NodeContent::Paragraph { text: String::new() }
    }
}

impl NodeContent {
    /// Get the primary text content of this node, if it carries text.
    ///
    /// Text-carrying nodes: `Title`, `Heading`, `Paragraph`, `ListItem`, `Code`,
    /// `Formula`, `Footnote`, `Comment`, `Citation` (returns text), `RawBlock` (returns
    /// content), `DefinitionItem` (returns term only, not definition).
    ///
    /// Container/marker nodes return `None`: `List`, `Quote`, `Group`, `PageBreak`,
    /// `Slide`, `DefinitionList`, `Admonition`, `MetadataBlock`.
    pub fn text(&self) -> Option<&str> {
        match self {
            NodeContent::Title { text }
            | NodeContent::Heading { text, .. }
            | NodeContent::Paragraph { text }
            | NodeContent::ListItem { text }
            | NodeContent::Code { text, .. }
            | NodeContent::Formula { text }
            | NodeContent::Footnote { text }
            | NodeContent::Comment { text }
            | NodeContent::Citation { text, .. }
            | NodeContent::RawBlock { content: text, .. } => Some(text),
            NodeContent::DefinitionItem { term, .. } => Some(term),
            NodeContent::Table { .. }
            | NodeContent::Image { .. }
            | NodeContent::List { .. }
            | NodeContent::Quote
            | NodeContent::Group { .. }
            | NodeContent::PageBreak
            | NodeContent::Slide { .. }
            | NodeContent::DefinitionList
            | NodeContent::Admonition { .. }
            | NodeContent::MetadataBlock { .. } => None,
        }
    }

    /// Invoke `redact` on every text-bearing field of this variant, in place.
    ///
    /// A `&mut` companion to [`Self::text`], but exhaustive rather than
    /// primary-field-only: it also covers secondary text fields `text()` does not
    /// surface (`Group::heading_text`, `Slide::title`, `Image::description`,
    /// `DefinitionItem::definition`, `Admonition::title`), plus per-cell table text
    /// and metadata-block values. Without this, a redaction pass over
    /// [`super::extraction::ExtractedDocument::content`] leaves the structured
    /// `document` tree holding the original text verbatim (xberg-io/xberg#298).
    ///
    /// Container/marker nodes with no text of their own — `List`, `Quote`,
    /// `PageBreak`, `DefinitionList` — are no-ops.
    /// Gated to match its callers (`text::redaction` and `text::translation::fields`,
    /// each gated on its own feature) — without this, a build with both off trips
    /// `dead_code`, which CI escalates via `-D warnings`. Any new caller must add its
    /// feature here, or the walker silently compiles out and that caller becomes a
    /// no-op rather than failing to build (#254).
    #[cfg(any(feature = "redaction", feature = "translation"))]
    pub(crate) fn for_each_text_field_mut(&mut self, mut redact: impl FnMut(&mut String)) {
        match self {
            NodeContent::Title { text }
            | NodeContent::Heading { text, .. }
            | NodeContent::Paragraph { text }
            | NodeContent::ListItem { text }
            | NodeContent::Code { text, .. }
            | NodeContent::Formula { text }
            | NodeContent::Footnote { text }
            | NodeContent::Comment { text }
            | NodeContent::Citation { text, .. }
            | NodeContent::RawBlock { content: text, .. } => redact(text),
            NodeContent::DefinitionItem { term, definition } => {
                redact(term);
                redact(definition);
            }
            NodeContent::Group { heading_text, .. } => {
                if let Some(text) = heading_text.as_mut() {
                    redact(text);
                }
            }
            NodeContent::Slide { title, .. } => {
                if let Some(text) = title.as_mut() {
                    redact(text);
                }
            }
            NodeContent::Image { description, .. } => {
                if let Some(text) = description.as_mut() {
                    redact(text);
                }
            }
            NodeContent::Admonition { title, .. } => {
                if let Some(text) = title.as_mut() {
                    redact(text);
                }
            }
            NodeContent::Table { grid } => {
                for cell in grid.cells.iter_mut() {
                    redact(&mut cell.content);
                }
            }
            NodeContent::MetadataBlock { entries } => {
                for attribute in entries.iter_mut() {
                    redact(&mut attribute.value);
                }
            }
            NodeContent::List { .. } | NodeContent::Quote | NodeContent::PageBreak | NodeContent::DefinitionList => {}
        }
    }

    /// Get the serde tag discriminant string for this variant.
    pub fn node_type_str(&self) -> &'static str {
        match self {
            NodeContent::Title { .. } => "title",
            NodeContent::Heading { .. } => "heading",
            NodeContent::Paragraph { .. } => "paragraph",
            NodeContent::List { .. } => "list",
            NodeContent::ListItem { .. } => "list_item",
            NodeContent::Table { .. } => "table",
            NodeContent::Image { .. } => "image",
            NodeContent::Code { .. } => "code",
            NodeContent::Quote => "quote",
            NodeContent::Formula { .. } => "formula",
            NodeContent::Footnote { .. } => "footnote",
            NodeContent::Comment { .. } => "comment",
            NodeContent::Group { .. } => "group",
            NodeContent::PageBreak => "page_break",
            NodeContent::Slide { .. } => "slide",
            NodeContent::DefinitionList => "definition_list",
            NodeContent::DefinitionItem { .. } => "definition_item",
            NodeContent::Citation { .. } => "citation",
            NodeContent::Admonition { .. } => "admonition",
            NodeContent::RawBlock { .. } => "raw_block",
            NodeContent::MetadataBlock { .. } => "metadata_block",
        }
    }
}

impl DocumentStructure {
    /// Create an empty `DocumentStructure`.
    pub(crate) fn new() -> Self {
        Self {
            nodes: Vec::new(),
            source_format: None,
            relationships: Vec::new(),
            node_types: Vec::new(),
        }
    }

    /// Create a `DocumentStructure` with pre-allocated capacity.
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(capacity),
            source_format: None,
            relationships: Vec::new(),
            node_types: Vec::new(),
        }
    }

    /// Push a node and return its `NodeIndex`.
    pub(crate) fn push_node(&mut self, node: DocumentNode) -> NodeIndex {
        let idx = NodeIndex(self.nodes.len() as u32);
        self.nodes.push(node);
        idx
    }

    /// Add a child to an existing parent node.
    ///
    /// Updates both the parent's `children` list and the child's `parent` field.
    ///
    /// # Panics
    ///
    /// Panics if either index is out of bounds.
    pub(crate) fn add_child(&mut self, parent: NodeIndex, child: NodeIndex) {
        self.nodes[parent.0 as usize].children.push(child);
        self.nodes[child.0 as usize].parent = Some(parent);
    }

    /// Validate all node indices are in bounds and parent-child relationships
    /// are bidirectionally consistent.
    ///
    /// # Errors
    ///
    /// Returns a descriptive error string if validation fails.
    pub(crate) fn validate(&self) -> std::result::Result<(), String> {
        let len = self.nodes.len() as u32;

        for (i, node) in self.nodes.iter().enumerate() {
            let idx = i as u32;

            if let Some(parent) = node.parent {
                if parent.0 >= len {
                    return Err(format!(
                        "Node {} has parent index {} which is out of bounds (len={})",
                        idx, parent.0, len
                    ));
                }
                if !self.nodes[parent.0 as usize].children.contains(&NodeIndex(idx)) {
                    return Err(format!(
                        "Node {} claims parent {}, but parent's children list does not contain {}",
                        idx, parent.0, idx
                    ));
                }
            }

            for child in &node.children {
                if child.0 >= len {
                    return Err(format!(
                        "Node {} has child index {} which is out of bounds (len={})",
                        idx, child.0, len
                    ));
                }
                if self.nodes[child.0 as usize].parent != Some(NodeIndex(idx)) {
                    return Err(format!(
                        "Node {} lists child {}, but child's parent is {:?} instead of {}",
                        idx, child.0, self.nodes[child.0 as usize].parent, idx
                    ));
                }
            }
        }

        Ok(())
    }

    /// Iterate over root-level body nodes (content_layer == Body, parent == None).
    #[cfg(test)]
    #[cfg_attr(alef, alef(skip))]
    pub(crate) fn body_roots(&self) -> impl Iterator<Item = (NodeIndex, &DocumentNode)> {
        self.nodes.iter().enumerate().filter_map(|(i, node)| {
            if node.parent.is_none() && node.content_layer == ContentLayer::Body {
                Some((NodeIndex(i as u32), node))
            } else {
                None
            }
        })
    }

    /// Iterate over root-level furniture nodes (non-Body content_layer, parent == None).
    #[cfg(test)]
    #[cfg_attr(alef, alef(skip))]
    pub(crate) fn furniture_roots(&self) -> impl Iterator<Item = (NodeIndex, &DocumentNode)> {
        self.nodes.iter().enumerate().filter_map(|(i, node)| {
            if node.parent.is_none() && node.content_layer != ContentLayer::Body {
                Some((NodeIndex(i as u32), node))
            } else {
                None
            }
        })
    }

    /// Get the total number of nodes.
    pub(crate) fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the document structure is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for DocumentStructure {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
