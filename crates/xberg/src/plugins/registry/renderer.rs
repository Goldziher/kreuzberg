//! Renderer registry.

use crate::plugins::{InternalRenderer, Plugin, Renderer};
use crate::types::internal::InternalDocument;
use crate::{Result, XbergError};
use ahash::AHashMap;
use std::sync::Arc;

#[derive(Clone)]
struct RegisteredRenderer {
    renderer: Arc<dyn Renderer>,
    internal: Option<Arc<dyn InternalRenderer>>,
}

impl RegisteredRenderer {
    fn public(renderer: Arc<dyn Renderer>) -> Self {
        Self {
            renderer,
            internal: None,
        }
    }

    fn internal<T>(renderer: Arc<T>) -> Self
    where
        T: InternalRenderer + 'static,
    {
        Self {
            renderer: renderer.clone(),
            internal: Some(renderer),
        }
    }

    fn render(&self, doc: &InternalDocument) -> Result<String> {
        if let Some(internal) = &self.internal {
            return internal.render(doc);
        }

        // Public renderers only ever see `ExtractedDocument`. The plain `From` conversion
        // derives without document structure and never fills in `elements`, which strips
        // every layout signal a renderer needs. Derive the structure tree explicitly and
        // attach both the element list and the source document so a foreign renderer can
        // do layout-aware rendering. The renderer's output is the `content` the caller
        // gets, so the pages of this document are built as for an output that is not plain.
        let mut result = crate::extraction::derive::derive_renderer_input(doc.clone());
        result.internal_document = Some(doc.clone());
        result.elements = Some(crate::extraction::transform::transform_extraction_result_to_elements(
            &result,
        ));
        self.renderer.render_result(&result)
    }
}

/// Built-in Markdown renderer.
struct MarkdownRenderer;

impl Plugin for MarkdownRenderer {
    fn name(&self) -> &str {
        "markdown"
    }
}

impl InternalRenderer for MarkdownRenderer {
    fn render(&self, doc: &InternalDocument) -> Result<String> {
        Ok(crate::rendering::render_markdown(doc))
    }
}

/// Built-in HTML renderer.
struct HtmlRenderer;

impl Plugin for HtmlRenderer {
    fn name(&self) -> &str {
        "html"
    }
}

impl InternalRenderer for HtmlRenderer {
    fn render(&self, doc: &InternalDocument) -> Result<String> {
        Ok(crate::rendering::render_html(doc))
    }
}

/// Built-in Djot renderer.
struct DjotRenderer;

impl Plugin for DjotRenderer {
    fn name(&self) -> &str {
        "djot"
    }
}

impl InternalRenderer for DjotRenderer {
    fn render(&self, doc: &InternalDocument) -> Result<String> {
        Ok(crate::rendering::render_djot(doc))
    }
}

/// Built-in Docling DocTags renderer.
struct DocTagsRenderer;

impl Plugin for DocTagsRenderer {
    fn name(&self) -> &str {
        "doctags"
    }
}

impl InternalRenderer for DocTagsRenderer {
    fn render(&self, doc: &InternalDocument) -> Result<String> {
        Ok(crate::rendering::render_doctags(doc))
    }
}

/// Built-in Graphviz DOT renderer.
struct DotRenderer;

impl Plugin for DotRenderer {
    fn name(&self) -> &str {
        "dot"
    }
}

impl InternalRenderer for DotRenderer {
    fn render(&self, doc: &InternalDocument) -> Result<String> {
        Ok(crate::rendering::render_dot(doc))
    }
}

// ~keep Liveness is configuration-dependent: `DocxRenderer` needs `office`, `PdfRenderer`
// needs `pdf`, and the redaction engine's nested-document check needs `redaction`, so
// `-D dead-code` fires on a leg without them. A hand-kept union-of-consumers `cfg` is what
// drifted and failed the 1.3.0 publish (GH#1951).
/// Name of the built-in DOCX renderer, reached through `OutputFormat::Custom`.
#[allow(dead_code)]
pub(crate) const DOCX_RENDERER_NAME: &str = "docx";

/// Name of the built-in PDF renderer, reached through `OutputFormat::Custom`.
#[allow(dead_code)]
pub(crate) const PDF_RENDERER_NAME: &str = "pdf";

/// How the base64 encoding of a zip archive begins: its first local file header, `PK\x03\x04`.
#[allow(dead_code)]
const BASE64_ZIP_PREFIX: &str = "UEsDB";

/// How the base64 encoding of a PDF file begins: its `%PDF-` header.
#[allow(dead_code)]
const BASE64_PDF_PREFIX: &str = "JVBERi0";

/// Whether a finished document's `content` is a base64-encoded binary document (DOCX or
/// PDF) rather than text that can still be rewritten.
///
/// Both the recorded format and the bytes have to agree, so text that merely claims the
/// format is still treated as text.
#[allow(dead_code)]
pub(crate) fn holds_encoded_package(output_format: Option<&str>, content: &str) -> bool {
    let prefix = match output_format {
        Some(DOCX_RENDERER_NAME) => BASE64_ZIP_PREFIX,
        Some(PDF_RENDERER_NAME) => BASE64_PDF_PREFIX,
        _ => return false,
    };
    content.starts_with(prefix)
}

/// Whether the built-in renderer `name` builds its output from the Markdown rendering.
///
/// An extractor that recovers headings and tables only for markup output formats has to
/// recover them for these too, or the output carries none.
#[allow(dead_code)]
pub(crate) fn renders_from_markdown(name: &str) -> bool {
    (cfg!(feature = "office") && name == DOCX_RENDERER_NAME) || (cfg!(feature = "pdf") && name == PDF_RENDERER_NAME)
}

/// Built-in DOCX renderer.
///
/// Renders Markdown, which the redaction processor rewrites like any other text output,
/// then packages it in [`InternalRenderer::finish`] as base64-encoded `.docx` bytes.
#[cfg(feature = "office")]
struct DocxRenderer;

#[cfg(feature = "office")]
impl Plugin for DocxRenderer {
    fn name(&self) -> &str {
        DOCX_RENDERER_NAME
    }
}

#[cfg(feature = "office")]
impl InternalRenderer for DocxRenderer {
    fn render(&self, doc: &InternalDocument) -> Result<String> {
        Ok(crate::rendering::render_markdown(doc))
    }

    fn finish(&self, rendered: String) -> Result<String> {
        use base64::Engine as _;
        let package = crate::rendering::render_docx(&rendered)?;
        Ok(base64::engine::general_purpose::STANDARD.encode(package))
    }
}

/// Built-in PDF renderer.
///
/// Renders Markdown, which the redaction processor rewrites like any other text output,
/// then lays it out in [`InternalRenderer::finish`] as a base64-encoded PDF file.
#[cfg(feature = "pdf")]
struct PdfRenderer;

#[cfg(feature = "pdf")]
impl Plugin for PdfRenderer {
    fn name(&self) -> &str {
        PDF_RENDERER_NAME
    }
}

#[cfg(feature = "pdf")]
impl InternalRenderer for PdfRenderer {
    fn render(&self, doc: &InternalDocument) -> Result<String> {
        Ok(crate::rendering::render_markdown(doc))
    }

    fn finish(&self, rendered: String) -> Result<String> {
        use base64::Engine as _;
        let document = crate::rendering::render_pdf(&rendered)?;
        Ok(base64::engine::general_purpose::STANDARD.encode(document))
    }
}

/// Built-in plain text renderer.
struct PlainRenderer;

impl Plugin for PlainRenderer {
    fn name(&self) -> &str {
        "plain"
    }
}

impl InternalRenderer for PlainRenderer {
    fn render(&self, doc: &InternalDocument) -> Result<String> {
        Ok(crate::rendering::render_plain(doc))
    }
}

/// Built-in renderer names registered by [`RendererRegistry::register_builtins`].
///
/// Used by [`RendererRegistry::is_missing_builtin_renderer`] to detect when
/// [`clear`](RendererRegistry::clear) has stripped a built-in that the `Custom`
/// output-format dispatch path (`extraction::derive::derive_extraction_result`) looks up
/// by name — including `"dot"`, which is reachable only through `Custom` since
/// `OutputFormat::from_str` never maps a string directly to a dedicated variant for it. ~keep
const BUILTIN_RENDERER_NAMES: &[&str] = &[
    "markdown",
    "html",
    "djot",
    "doctags",
    "dot",
    #[cfg(feature = "office")]
    DOCX_RENDERER_NAME,
    #[cfg(feature = "pdf")]
    PDF_RENDERER_NAME,
    "plain",
];

/// Registry for document renderer plugins.
///
/// Manages renderers that convert internal pipeline documents to output format strings.
///
/// # Thread Safety
///
/// The registry is thread-safe and can be accessed concurrently from multiple threads.
///
/// # Example
///
/// ```rust,no_run
/// use xberg::plugins::registry::RendererRegistry;
/// use std::sync::Arc;
///
/// let registry = RendererRegistry::new();
/// let available = registry.list();
/// // Built-in renderers: "markdown", "html", "djot", "doctags", "dot", "docx", "pdf", "plain"
/// ```
#[cfg_attr(alef, alef(skip))]
pub struct RendererRegistry {
    renderers: AHashMap<String, RegisteredRenderer>,
}

impl RendererRegistry {
    /// Create a new renderer registry with built-in renderers.
    ///
    /// Registers the following built-in renderers:
    /// - `markdown` — GFM Markdown (via comrak)
    /// - `html` — HTML5 (via comrak)
    /// - `djot` — Djot markup
    /// - `doctags` — Docling DocTags (tables as OTSL)
    /// - `dot` — Graphviz DOT (diagrams recovered from vector sources)
    /// - `docx` — Office Open XML, base64-encoded (with the `office` feature)
    /// - `pdf` — PDF, base64-encoded (with the `pdf` feature)
    /// - `plain` — Plain text (no formatting)
    pub fn new() -> Self {
        let mut registry = Self {
            renderers: AHashMap::new(),
        };

        registry.register_builtins();
        registry
    }

    /// Create a new empty renderer registry without built-in renderers.
    ///
    /// Useful for testing or when you want full control over renderer registration.
    pub fn new_empty() -> Self {
        Self {
            renderers: AHashMap::new(),
        }
    }

    /// Register built-in renderers.
    fn register_builtins(&mut self) {
        self.renderers.insert(
            "markdown".to_string(),
            RegisteredRenderer::internal(Arc::new(MarkdownRenderer)),
        );
        self.renderers
            .insert("html".to_string(), RegisteredRenderer::internal(Arc::new(HtmlRenderer)));
        self.renderers
            .insert("djot".to_string(), RegisteredRenderer::internal(Arc::new(DjotRenderer)));
        self.renderers.insert(
            "doctags".to_string(),
            RegisteredRenderer::internal(Arc::new(DocTagsRenderer)),
        );
        self.renderers
            .insert("dot".to_string(), RegisteredRenderer::internal(Arc::new(DotRenderer)));
        #[cfg(feature = "office")]
        self.renderers.insert(
            DOCX_RENDERER_NAME.to_string(),
            RegisteredRenderer::internal(Arc::new(DocxRenderer)),
        );
        #[cfg(feature = "pdf")]
        self.renderers.insert(
            PDF_RENDERER_NAME.to_string(),
            RegisteredRenderer::internal(Arc::new(PdfRenderer)),
        );
        self.renderers.insert(
            "plain".to_string(),
            RegisteredRenderer::internal(Arc::new(PlainRenderer)),
        );
    }

    /// Register a renderer.
    ///
    /// # Arguments
    ///
    /// * `renderer` - The renderer to register
    ///
    /// # Returns
    ///
    /// - `Ok(())` if registration succeeded
    /// - `Err(...)` if the renderer name is invalid
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use xberg::plugins::registry::RendererRegistry;
    /// # use std::sync::Arc;
    /// let mut registry = RendererRegistry::new();
    /// // let renderer = Arc::new(MyRenderer);
    /// // registry.register(renderer)?;
    /// # Ok::<(), xberg::XbergError>(())
    /// ```
    pub fn register(&mut self, renderer: Arc<dyn Renderer>) -> Result<()> {
        let name = renderer.name().to_string();

        super::validate_plugin_name(&name)?;

        self.renderers.insert(name, RegisteredRenderer::public(renderer));
        Ok(())
    }

    /// Render a document using the named renderer.
    ///
    /// Convenience method that looks up the renderer by name and renders the document.
    ///
    /// # Arguments
    ///
    /// * `name` - Renderer name (e.g., "markdown", "html")
    /// * `doc` - The internal document to render
    ///
    /// # Returns
    ///
    /// The rendered output string, or an error if the renderer is not found or rendering fails.
    pub(crate) fn render(&self, name: &str, doc: &InternalDocument) -> Result<String> {
        self.renderers
            .get(name)
            .ok_or_else(|| XbergError::Plugin {
                message: format!("Renderer '{}' not registered", name),
                plugin_name: name.to_string(),
            })?
            .render(doc)
    }

    /// Finish a rendering that [`render`](Self::render) produced, once post-processing
    /// has run on it.
    ///
    /// Only a built-in renderer has a finishing step. A public renderer's output, and the
    /// output of a name that is not registered, is returned unchanged.
    pub(crate) fn finish(&self, name: &str, rendered: String) -> Result<String> {
        match self.renderers.get(name).and_then(|entry| entry.internal.as_ref()) {
            Some(internal) => internal.finish(rendered),
            None => Ok(rendered),
        }
    }

    /// List all registered renderer names.
    pub fn list(&self) -> Vec<String> {
        self.renderers.keys().cloned().collect()
    }

    /// Remove a renderer from the registry, calling its `shutdown()` method.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        if let Some(renderer) = self.renderers.remove(name) {
            renderer.renderer.shutdown()?;
        }
        Ok(())
    }

    /// Clear all renderers from the registry.
    ///
    /// Removes every renderer, including the built-in defaults. After calling
    /// this the registry is empty; re-register renderers as needed.
    pub fn clear_all(&mut self) -> Result<()> {
        let names: Vec<_> = self.renderers.keys().cloned().collect();
        for name in names {
            self.remove(&name)?;
        }
        Ok(())
    }

    /// Drain the registry. Alias for `clear_all` used by alef trait-bridge codegen.
    pub fn clear(&mut self) -> Result<()> {
        self.clear_all()
    }

    /// Clear all renderers and re-register the built-in defaults.
    #[cfg(test)]
    pub(crate) fn reset_to_defaults(&mut self) -> Result<()> {
        self.renderers.clear();
        self.register_builtins();
        Ok(())
    }

    /// True when the registry is missing one or more built-in renderers and should be
    /// re-seeded.
    ///
    /// [`clear`](Self::clear) empties the registry, including the built-ins, leaving the
    /// `Custom` output-format dispatch path with no renderer to look up by name.
    pub(crate) fn is_missing_builtin_renderer(&self) -> bool {
        BUILTIN_RENDERER_NAMES
            .iter()
            .any(|name| !self.renderers.contains_key(*name))
    }

    /// Non-destructively re-seed the built-in renderers when any are missing.
    ///
    /// Keeps any user-registered renderers; a no-op when every built-in is already
    /// present. Mirrors `OcrBackendRegistry::ensure_defaults` for the OCR backend
    /// registry: used by the self-healing dispatch path so a registry emptied via
    /// [`clear`](Self::clear) (e.g. by a sibling test or a plugin lifecycle reset) heals
    /// itself before the next `Custom`-format render.
    pub(crate) fn ensure_defaults(&mut self) {
        if self.is_missing_builtin_renderer() {
            self.register_builtins();
        }
    }
}

impl Default for RendererRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockRenderer {
        format_name: String,
    }

    impl Plugin for MockRenderer {
        fn name(&self) -> &str {
            &self.format_name
        }
    }

    impl InternalRenderer for MockRenderer {
        fn render(&self, doc: &InternalDocument) -> Result<String> {
            Ok(format!("mock-rendered-{}-elements", doc.elements.len()))
        }
    }

    #[test]
    fn test_renderer_registry_new_has_builtins() {
        let registry = RendererRegistry::new();
        let names = registry.list();
        assert!(names.contains(&"markdown".to_string()));
        assert!(names.contains(&"html".to_string()));
        assert!(names.contains(&"djot".to_string()));
        assert!(names.contains(&"doctags".to_string()));
        assert!(names.contains(&"plain".to_string()));
    }

    #[test]
    fn test_renderer_registry_new_empty() {
        let registry = RendererRegistry::new_empty();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_renderer_registry_register_and_get() {
        let mut registry = RendererRegistry::new_empty();

        let renderer = Arc::new(MockRenderer {
            format_name: "test-format".to_string(),
        });

        registry.register(renderer).unwrap();

        assert!(registry.list().contains(&"test-format".to_string()));
    }

    #[test]
    fn test_renderer_registry_get_missing() {
        let registry = RendererRegistry::new_empty();
        assert!(!registry.list().contains(&"nonexistent".to_string()));
    }

    #[test]
    fn test_renderer_registry_render_convenience() {
        let registry = RendererRegistry::new();
        let doc = InternalDocument::new("text/plain");

        let result = registry.render("plain", &doc);
        assert!(result.is_ok());
    }

    #[test]
    fn test_renderer_registry_render_missing() {
        let registry = RendererRegistry::new_empty();
        let doc = InternalDocument::new("text/plain");

        let result = registry.render("nonexistent", &doc);
        assert!(result.is_err());
    }

    #[test]
    fn test_renderer_registry_remove() {
        let mut registry = RendererRegistry::new_empty();
        let renderer = Arc::new(MockRenderer {
            format_name: "to-remove".to_string(),
        });
        registry.register(renderer).unwrap();

        registry.remove("to-remove").unwrap();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_renderer_registry_remove_nonexistent_returns_ok() {
        let mut registry = RendererRegistry::new_empty();
        let result = registry.remove("nonexistent-renderer");
        assert!(result.is_ok());
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_renderer_registry_clear_all_drops_builtins() {
        let mut registry = RendererRegistry::new();
        assert!(!registry.list().is_empty());

        registry.clear_all().unwrap();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_renderer_registry_clear_alias_matches_clear_all() {
        let mut registry = RendererRegistry::new();
        let custom = Arc::new(MockRenderer {
            format_name: "custom".to_string(),
        });
        registry.register(custom).unwrap();

        registry.clear().unwrap();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_renderer_registry_is_missing_builtin_renderer_after_clear() {
        let mut registry = RendererRegistry::new();
        assert!(!registry.is_missing_builtin_renderer());

        registry.clear().unwrap();
        assert!(registry.is_missing_builtin_renderer());
    }

    #[test]
    fn ensure_defaults_reseeds_builtins_after_clear_and_keeps_user_renderer() {
        let mut registry = RendererRegistry::new();
        registry.clear().unwrap();
        let custom = Arc::new(MockRenderer {
            format_name: "custom-after-clear".to_string(),
        });
        registry.register(custom).unwrap();
        assert!(
            registry.is_missing_builtin_renderer(),
            "precondition: built-ins absent after clear"
        );

        registry.ensure_defaults();

        assert!(
            !registry.is_missing_builtin_renderer(),
            "ensure_defaults should re-seed every built-in renderer"
        );
        assert!(
            registry.list().contains(&"custom-after-clear".to_string()),
            "ensure_defaults must be non-destructive: the user renderer is kept"
        );
    }

    #[test]
    fn test_renderer_registry_reset_to_defaults() {
        let mut registry = RendererRegistry::new();
        let custom = Arc::new(MockRenderer {
            format_name: "custom".to_string(),
        });
        registry.register(custom).unwrap();
        assert!(registry.list().contains(&"custom".to_string()));

        registry.reset_to_defaults().unwrap();
        assert!(!registry.list().contains(&"custom".to_string()));
        assert!(registry.list().contains(&"markdown".to_string()));
    }

    #[test]
    fn test_renderer_registry_invalid_name_empty() {
        let mut registry = RendererRegistry::new_empty();
        let renderer = Arc::new(MockRenderer {
            format_name: "".to_string(),
        });

        let result = registry.register(renderer);
        assert!(matches!(result, Err(XbergError::Validation { .. })));
    }

    #[test]
    fn test_renderer_registry_invalid_name_with_spaces() {
        let mut registry = RendererRegistry::new_empty();
        let renderer = Arc::new(MockRenderer {
            format_name: "invalid format".to_string(),
        });

        let result = registry.register(renderer);
        assert!(matches!(result, Err(XbergError::Validation { .. })));
    }

    #[test]
    fn test_renderer_registry_builtin_markdown_renders() {
        let registry = RendererRegistry::new();
        let doc = InternalDocument::new("text/plain");

        let result = registry.render("markdown", &doc).unwrap();
        let _ = result;
    }

    #[test]
    fn test_renderer_registry_builtin_html_renders() {
        let registry = RendererRegistry::new();
        let doc = InternalDocument::new("text/plain");

        let result = registry.render("html", &doc).unwrap();
        let _ = result;
    }

    #[test]
    fn test_renderer_registry_builtin_djot_renders() {
        let registry = RendererRegistry::new();
        let doc = InternalDocument::new("text/plain");

        let result = registry.render("djot", &doc).unwrap();
        let _ = result;
    }

    #[test]
    fn test_renderer_registry_builtin_doctags_renders() {
        let registry = RendererRegistry::new();
        let doc = InternalDocument::new("text/plain");

        let result = registry.render("doctags", &doc).unwrap();
        assert_eq!(result, "<doctag></doctag>");
    }

    #[test]
    fn test_renderer_registry_builtin_plain_renders() {
        let registry = RendererRegistry::new();
        let doc = InternalDocument::new("text/plain");

        let result = registry.render("plain", &doc).unwrap();
        let _ = result;
    }

    #[cfg(feature = "office")]
    #[test]
    fn test_renderer_registry_builtin_docx_finishes_into_a_base64_package() {
        let registry = RendererRegistry::new();
        let rendered = registry.render("docx", &InternalDocument::new("text/plain")).unwrap();
        let finished = registry.finish("docx", rendered).unwrap();

        use base64::Engine as _;
        let package = base64::engine::general_purpose::STANDARD.decode(&finished).unwrap();
        assert!(package.starts_with(b"PK\x03\x04"), "not a zip archive");
        assert!(holds_encoded_package(Some("docx"), &finished));
    }

    #[test]
    fn test_renderer_registry_finish_leaves_text_formats_unchanged() {
        let registry = RendererRegistry::new();
        assert_eq!(registry.finish("markdown", "# Title".to_string()).unwrap(), "# Title");
        assert_eq!(registry.finish("unregistered", "text".to_string()).unwrap(), "text");
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn test_renderer_registry_builtin_pdf_finishes_into_a_base64_document() {
        let registry = RendererRegistry::new();
        let rendered = registry.render("pdf", &InternalDocument::new("text/plain")).unwrap();
        let finished = registry.finish("pdf", rendered).unwrap();

        use base64::Engine as _;
        let document = base64::engine::general_purpose::STANDARD.decode(&finished).unwrap();
        assert!(document.starts_with(b"%PDF-"), "not a PDF file");
        assert!(holds_encoded_package(Some("pdf"), &finished));
    }

    #[test]
    fn test_text_that_only_claims_the_docx_format_is_not_an_encoded_package() {
        assert!(!holds_encoded_package(Some("docx"), "Jane Doe signed off."));
        assert!(!holds_encoded_package(Some("markdown"), "UEsDBBQAAAAIAAAAIQ"));
    }

    #[test]
    fn test_text_that_only_claims_the_pdf_format_is_not_an_encoded_package() {
        assert!(!holds_encoded_package(Some("pdf"), "Jane Doe signed off."));
        assert!(!holds_encoded_package(Some("pdf"), "UEsDBBQAAAAIAAAAIQ"));
        assert!(!holds_encoded_package(Some("docx"), "JVBERi0xLjcKJcK1wrY"));
        assert!(holds_encoded_package(Some("pdf"), "JVBERi0xLjcKJcK1wrY"));
    }
}
