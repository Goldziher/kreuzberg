//! OCR backend plugin trait.
//!
//! This module defines the trait for implementing custom OCR backends.

use crate::Result;
use crate::core::config::OcrConfig;
use crate::plugins::Plugin;
use crate::types::ExtractedDocument;
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

#[cfg(not(feature = "tokio-runtime"))]
use crate::XbergError;

/// OCR backend types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize)]
pub enum OcrBackendType {
    /// Tesseract OCR (native Rust binding)
    #[default]
    Tesseract,
    /// PaddleOCR (Python-based, via FFI)
    PaddleOCR,
    /// Candle-based VLM OCR (TrOCR, PaddleOCR-VL).
    Candle,
    /// Name-selected built-in or third-party OCR backend.
    Custom,
}

/// How a backend's reported page-level confidence must be interpreted.
///
/// Backend confidence scores are not interchangeable. Tesseract's mean word confidence is a
/// classifier score validated to track legibility on a 0-100 scale. Sceptre (EasyOCR-based)
/// reports a length-penalised `custom_mean` that is rescaled into the same 0-100 range but is
/// *not* comparable — its ordering can be inverted relative to legibility (a dense prose page
/// can score lower than a nearly-blank one). A page-rejection gate calibrated on Tesseract's
/// scale was once applied unconditionally to sceptre's output and rejected every page of a
/// 16-page document, emptying it. This descriptor exists so gating code can ask a backend what
/// its number means instead of assuming.
// `Default` is `Uncalibrated`, matching `OcrBackend::confidence_semantics`'s trait default
// exactly. It reinforces that invariant rather than competing with it: the one value it is
// never safe to fall back to is `Legibility`, which would let an undeclared backend inherit
// Tesseract's gate threshold. `serde` and `Default` are required because the `Legibility`
// variant carries a payload. The payload is NOT what forces the JSON marshalling, though: the
// generated bridge marshals every trait method's return value that way, which is why the
// unit-only `PageOrientationHandling` below needs exactly the same derives. ~keep
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Deserialize, serde::Serialize)]
pub enum ConfidenceSemantics {
    /// Validated to track legibility on a known scale — usable as an absolute quality gate.
    Legibility {
        /// The upper bound of the reported confidence scale (e.g. `100.0` for Tesseract).
        scale_max: f64,
    },
    /// A number is reported, but it is not validated to correlate with legibility.
    /// Never gate on it.
    #[default]
    Uncalibrated,
    /// No page-level confidence is reported at all.
    None,
}

/// How a backend copes with a page raster whose text is not upright.
///
/// Rotated-page handling is a backend capability, not a universal guarantee. An A/B run this
/// session against `/Rotate 270` scanned pages showed the three handled cases genuinely differ:
/// Tesseract reconstructs correct reading order on a sideways raster outright; PaddleOCR
/// recognises the rotated text correctly (it warps each detected quad upright before running
/// recognition) but leaves its block list in raw raster `(y, x)` order, so the caller must
/// reorder; sceptre produces character garbage on the same sideways raster and only reads
/// correctly once the page is rendered upright first. A caller that skips an upright-render step
/// for a backend that actually needs one gets silent garbage, not an error.
///
/// # Only one variant is discriminated (#657)
///
/// Read this before "simplifying" the type. There is exactly one decision point in the
/// codebase that inspects this value: `upright_raster_for_backend`
/// (`crate::extractors::pdf::ocr`), which tests `orientation_handling != RequiresUpright` and
/// otherwise does nothing. Every other mention forwards the value to that test. So, *to that
/// codepath*, `SelfCorrecting` and `RecognisesRotatedText` are behaviourally identical — the
/// enum is a boolean at the point of use, and the three variants describe measured backend
/// behaviour rather than three dispatch paths.
///
/// `RecognisesRotatedText`'s actual remedy is not this enum. The block-order fix is the
/// `backend_options["page_rotation_degrees"]` hint injected by
/// `ocr_config_with_page_rotation_hint` (`crate::extractors::pdf::ocr`) **unconditionally, for
/// every backend**, which `PaddleOcrBackend::process_image` reads back
/// (`page_rotation_degrees_from_backend_options` -> `residual_rotation_for_reorder` ->
/// `reorder_blocks_for_page_rotation`, `crate::paddle_ocr::backend`) and applies internally.
/// Declaring `RecognisesRotatedText` therefore changes nothing on its own; a backend in that
/// class must also read the hint. Conversely, gating that hint on this enum would remove a
/// field from Tesseract's `OcrConfig` and hence from the OCR cache key
/// (`hash(image + language + config)`), invalidating every cached page — do not do it without
/// its own A/B.
///
/// # PDF-route-only
///
/// Only the PDF OCR routes call `OcrBackend::page_orientation_handling` (the `--force-ocr`
/// route via `extract_with_ocr` and the scanned-pages route via `extract_mixed_ocr_native`).
/// The raw-image route (`crate::extractors::image`) never calls it: there is no `/Rotate` to
/// consult, and orientation there is handled by the PP-LCNet document-orientation classifier
/// (`crate::doc_orientation`) gated on `OcrConfig::auto_rotate`.
///
/// # Cost of the default
///
/// The trait default is `RequiresUpright` (deliberately the least capable option, see
/// [`OcrBackend::page_orientation_handling`]). A backend that does not declare therefore pays,
/// on every page with `/Rotate != 0`, a re-encode plus rotation of the raster in
/// `upright_raster_for_backend` and a bounding-box round-trip back through
/// `undo_upright_raster_correction`. That is the safe direction to be wrong in, but it is not
/// free, and for a backend that never got measured it is not known to be necessary either.
// `Default` and `serde` are required by the generated FFI bridge, which marshals the return value
// of EVERY `OcrBackend` trait method through JSON -- see `XbergOcrBackendBridge` in
// `crates/xberg-ffi/src/lib.rs`, where `page_orientation_handling` does
// `serde_json::from_str::<PageOrientationHandling>(&json)` and falls back to `Default::default()`
// on an uninitialised vtable slot, a failing host callback, or a null result. Whether a variant
// carries a payload makes no difference to that. `RequiresUpright` is the default so it matches
// `OcrBackend::page_orientation_handling`'s trait default exactly. ~keep
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize)]
pub enum PageOrientationHandling {
    /// Reconstructs reading order regardless of page rotation — safe to hand a
    /// raster in any orientation.
    SelfCorrecting,
    /// Recognises rotated text correctly but emits blocks in raw raster order,
    /// so the caller must reorder.
    RecognisesRotatedText,
    /// Requires an upright raster; rotated text produces garbage.
    #[default]
    RequiresUpright,
}

/// Trait for OCR backend plugins.
///
/// Implement this trait to add custom OCR capabilities. OCR backends can be:
/// - Native Rust implementations (like Tesseract)
/// - FFI bridges to external libraries (like PaddleOCR)
/// - Cloud-based OCR services (Google Vision, AWS Textract, etc.)
///
/// # Thread Safety
///
/// OCR backends must be thread-safe (`Send + Sync`) to support concurrent processing.
///
/// # Example
///
/// ```rust
/// use xberg::plugins::{Plugin, OcrBackend, OcrBackendType};
/// use xberg::{Result, OcrConfig};
/// use async_trait::async_trait;
/// use std::borrow::Cow;
/// use std::path::Path;
/// use xberg::types::{ExtractedDocument, Metadata};
///
/// struct CustomOcrBackend;
///
/// impl Plugin for CustomOcrBackend {
///     fn name(&self) -> &str { "custom-ocr" }
///     fn version(&self) -> String { "1.0.0".to_string() }
///     fn initialize(&self) -> Result<()> { Ok(()) }
///     fn shutdown(&self) -> Result<()> { Ok(()) }
/// }
///
/// #[async_trait]
/// impl OcrBackend for CustomOcrBackend {
///     async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractedDocument> {
///         // Implement OCR logic here
///         let mut document = ExtractedDocument::default();
///         document.content = "Extracted text".to_string();
///         document.mime_type = Cow::Borrowed("text/plain");
///         Ok(document)
///     }
///
///     async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractedDocument> {
///         let bytes = std::fs::read(path)?;
///         self.process_image(&bytes, config).await
///     }
///
///     fn supports_language(&self, lang: &str) -> bool {
///         matches!(lang, "eng" | "deu" | "fra")
///     }
///
///     fn backend_type(&self) -> OcrBackendType {
///         OcrBackendType::Custom
///     }
/// }
/// ```
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait OcrBackend: Plugin {
    /// Process an image and extract text via OCR.
    ///
    /// # Arguments
    ///
    /// * `image_bytes` - Raw image data (JPEG, PNG, TIFF, etc.)
    /// * `config` - OCR configuration (language, PSM mode, etc.)
    ///
    /// # Returns
    ///
    /// An `ExtractedDocument` containing the extracted text and metadata.
    ///
    /// ~keep Image and PDF page OCR also accept an explicit `document` contribution:
    /// a flat, ordered list of `Title`, `Heading` (levels 1–6), `Paragraph`,
    /// `Code`, `Formula`, `Footnote`, `Comment`, `Citation` and HTML `RawBlock`
    /// nodes. Keep `content` as the page transcript: the existing quality and
    /// whole-page replacement decisions still use it. Literal markup in ordinary
    /// paragraph text is not promoted to raw HTML.
    /// HTML raw blocks are emitted verbatim: supply trusted markup, not an
    /// ordinary OCR transcript relabeled as HTML.
    /// Unsupported nested/container, table-grid and image nodes fail with
    /// `Validation`, rather than flattening.
    /// For string-only output leave `document` unset; a nonempty transcript with
    /// an empty explicit structure is invalid, not a request to flatten it.
    ///
    /// Node annotation ranges must be UTF-8 byte boundaries; relationship indices
    /// must reference nodes in this contribution. Optional boxes are top-left OCR
    /// raster coordinates (the processed dimensions in backend metadata, when
    /// supplied); absent boxes/confidence are not fabricated. PDF owners convert
    /// the boxes to their page coordinate frame. The image extractor retains its
    /// own source format and source-image reference, not `document.source_format`.
    /// Node IDs, node types and result counts are derived by the host extractor.
    ///
    /// Structured contributions do not accept backend table/image/URI inventories,
    /// document-level annotations, pre-rendered output or derived enrichment/page
    /// fields. Leave those fields unset; they fail explicitly instead of being
    /// discarded. Inline node annotations, `metadata`, `processing_warnings`,
    /// `ocr_elements`, `formulas` and `llm_usage` remain subject to their existing
    /// extractor aggregation/selection policies. A selected whole-page contribution
    /// replaces superseded native flow, not retained native asset inventories.
    /// Security limits bound serialized payload size, recursive metadata nesting and
    /// node/annotation/reference work. These checks apply only when `document` is present; string-only results
    /// and built-in private hOCR results retain their existing contract.
    ///
    /// # Errors
    ///
    /// - `XbergError::Ocr` - OCR processing failed
    /// - `XbergError::Validation` - Invalid image format or configuration
    /// - `XbergError::Io` - I/O errors (these always bubble up)
    ///
    /// # Reading `backend_options`
    ///
    /// Backends that support runtime tuning can read `config.backend_options` and
    /// deserialize only the keys they care about. Unknown keys are silently ignored,
    /// so multiple backends can coexist in a pipeline without key conflicts.
    ///
    /// ```rust
    /// # use xberg::plugins::{Plugin, OcrBackend};
    /// # use xberg::{Result, OcrConfig};
    /// # use async_trait::async_trait;
    /// # use std::borrow::Cow;
    /// # use std::path::Path;
    /// # use xberg::types::{ExtractedDocument, Metadata};
    /// # struct MyOcr;
    /// # impl Plugin for MyOcr {
    /// #     fn name(&self) -> &str { "my-ocr" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # use xberg::plugins::OcrBackendType;
    /// # #[async_trait]
    /// # impl OcrBackend for MyOcr {
    /// #     fn supports_language(&self, _: &str) -> bool { true }
    /// #     fn backend_type(&self) -> OcrBackendType { OcrBackendType::Custom }
    /// #     async fn process_image_file(&self, _: &Path, _: &OcrConfig) -> Result<ExtractedDocument> {
    /// #         Ok(ExtractedDocument::default())
    /// #     }
    /// async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractedDocument> {
    ///     // Read backend-specific options; unknown keys are silently ignored.
    ///     let fast_mode = config.backend_options
    ///         .as_ref()
    ///         .and_then(|v| v.get("mode"))
    ///         .and_then(|v| v.as_str())
    ///         .map(|s| s == "fast")
    ///         .unwrap_or(false);
    ///
    ///     if image_bytes.is_empty() {
    ///         return Err(xberg::XbergError::Validation {
    ///             message: "Empty image data".to_string(),
    ///             source: None,
    ///         });
    ///     }
    ///
    ///     let text = if fast_mode {
    ///         "Fast OCR result".to_string()
    ///     } else {
    ///         format!("Extracted text in language: {:?}", config.language)
    ///     };
    ///
    ///     let mut document = ExtractedDocument::default();
    ///     document.content = text;
    ///     document.mime_type = Cow::Borrowed("text/plain");
    ///     Ok(document)
    /// }
    /// # }
    /// ```
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractedDocument>;

    /// Process an owned image buffer and extract text via OCR.
    ///
    /// The default implementation delegates to [`Self::process_image`]. Backends
    /// that hand work to an owned blocking task can override this method to avoid
    /// copying the image buffer.
    ///
    /// Excluded from the polyglot binding surface: it is an owned-buffer perf
    /// override whose `Arc<Vec<u8>>` parameter has no binding representation, and
    /// foreign backends satisfy the trait through [`Self::process_image`] via this
    /// default delegation.
    #[cfg_attr(alef, alef(skip))]
    async fn process_image_owned(&self, image_bytes: Arc<Vec<u8>>, config: &OcrConfig) -> Result<ExtractedDocument> {
        self.process_image(image_bytes.as_slice(), config).await
    }

    /// Process a file and extract text via OCR.
    ///
    /// Default implementation reads the file and calls `process_image`.
    /// Override for custom file handling or optimizations.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the image file
    /// * `config` - OCR configuration
    ///
    /// # Errors
    ///
    /// Same as `process_image`, plus file I/O errors.
    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractedDocument> {
        #[cfg(feature = "tokio-runtime")]
        {
            use crate::core::io;
            let bytes = io::read_file_async(path).await?;
            self.process_image(&bytes, config).await
        }
        #[cfg(not(feature = "tokio-runtime"))]
        {
            let _ = (path, config);
            Err(XbergError::Other(
                "File-based OCR processing requires the tokio-runtime feature".to_string(),
            ))
        }
    }

    /// Check if this backend supports a given language code.
    ///
    /// # Arguments
    ///
    /// * `lang` - ISO 639-2/3 language code (e.g., "eng", "deu", "fra")
    ///
    /// # Returns
    ///
    /// `true` if the language is supported, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use xberg::plugins::{Plugin, OcrBackend};
    /// # use xberg::Result;
    /// # use async_trait::async_trait;
    /// # use std::path::Path;
    /// # struct MyOcr { languages: Vec<String> }
    /// # impl Plugin for MyOcr {
    /// #     fn name(&self) -> &str { "my-ocr" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # use xberg::plugins::OcrBackendType;
    /// # use xberg::{ExtractedDocument, OcrConfig};
    /// # #[async_trait]
    /// # impl OcrBackend for MyOcr {
    /// #     fn backend_type(&self) -> OcrBackendType { OcrBackendType::Custom }
    /// #     async fn process_image(&self, _: &[u8], _: &OcrConfig) -> Result<ExtractedDocument> {
    /// #         Ok(ExtractedDocument::default())
    /// #     }
    /// #     async fn process_image_file(&self, _: &Path, _: &OcrConfig) -> Result<ExtractedDocument> {
    /// #         Ok(ExtractedDocument::default())
    /// #     }
    /// fn supports_language(&self, lang: &str) -> bool {
    ///     self.languages.contains(&lang.to_string())
    /// }
    /// # }
    /// ```
    fn supports_language(&self, lang: &str) -> bool;

    /// Get the backend type identifier.
    ///
    /// # Returns
    ///
    /// The backend type enum value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use xberg::plugins::{Plugin, OcrBackend, OcrBackendType};
    /// # use xberg::Result;
    /// # use async_trait::async_trait;
    /// # use std::path::Path;
    /// # struct TesseractBackend;
    /// # impl Plugin for TesseractBackend {
    /// #     fn name(&self) -> &str { "tesseract" }
    /// #     fn version(&self) -> String { "1.0.0".to_string() }
    /// #     fn initialize(&self) -> Result<()> { Ok(()) }
    /// #     fn shutdown(&self) -> Result<()> { Ok(()) }
    /// # }
    /// # use xberg::{ExtractedDocument, OcrConfig};
    /// # #[async_trait]
    /// # impl OcrBackend for TesseractBackend {
    /// #     fn supports_language(&self, _: &str) -> bool { true }
    /// #     async fn process_image(&self, _: &[u8], _: &OcrConfig) -> Result<ExtractedDocument> {
    /// #         Ok(ExtractedDocument::default())
    /// #     }
    /// #     async fn process_image_file(&self, _: &Path, _: &OcrConfig) -> Result<ExtractedDocument> {
    /// #         Ok(ExtractedDocument::default())
    /// #     }
    /// fn backend_type(&self) -> OcrBackendType {
    ///     OcrBackendType::Tesseract
    /// }
    /// # }
    /// ```
    fn backend_type(&self) -> OcrBackendType;

    /// Optional: Get a list of all supported languages.
    ///
    /// Defaults to empty list. Override to provide comprehensive language support info.
    fn supported_languages(&self) -> Vec<String> {
        vec![]
    }

    /// Optional: languages usable under `config`, defaulting to ignoring it. GH#1857. ~keep
    fn supported_languages_for(&self, config: &OcrConfig) -> Vec<String> {
        let _ = config;
        self.supported_languages()
    }

    /// Optional: whether `language` is supported under `config`. GH#1857.
    fn supports_language_for(&self, config: &OcrConfig, language: &str) -> bool {
        let _ = config;
        self.supports_language(language)
    }

    /// Optional: Check if the backend supports table detection.
    ///
    /// Defaults to `false`. Override if your backend can detect and extract tables.
    fn supports_table_detection(&self) -> bool {
        false
    }

    /// Check if the backend supports direct document-level processing (e.g. for PDFs).
    ///
    /// Defaults to `false`. Override if the backend has optimized document processing.
    /// PDF extraction uses this optimized path only when both effective page margins
    /// are zero; nonzero margins require per-page image processing so geometry can be
    /// filtered correctly.
    fn supports_document_processing(&self) -> bool {
        false
    }

    /// Declare that this backend emits structured markdown directly (tables, headings, lists)
    /// and downstream layout reconstruction should be skipped.
    ///
    /// Defaults to `false` — classical OCR backends (Tesseract, PaddleOCR classical) return
    /// plain text per detected region. End-to-end VLM backends (PaddleOCR-VL, GOT-OCR 2.0)
    /// emit markdown in one forward pass and should override this to `true`.
    fn emits_structured_markdown(&self) -> bool {
        false
    }

    /// Declare how this backend's reported page-level confidence must be interpreted.
    ///
    /// Defaults to [`ConfidenceSemantics::Uncalibrated`]. This default is deliberately the
    /// least trusting option, not [`ConfidenceSemantics::Legibility`]: a new backend that
    /// reports *some* confidence number is not thereby safe to gate on, and defaulting to
    /// `Legibility` would let the next backend silently inherit a threshold calibrated for a
    /// different backend's scale — exactly the failure this type exists to prevent (see the
    /// type's doc comment). Override this only after validating that the reported number
    /// tracks legibility on a known scale.
    fn confidence_semantics(&self) -> ConfidenceSemantics {
        ConfidenceSemantics::Uncalibrated
    }

    /// Declare how this backend copes with a page raster whose text is not upright.
    ///
    /// Defaults to [`PageOrientationHandling::RequiresUpright`]. This default is deliberately
    /// the least capable option, not [`PageOrientationHandling::SelfCorrecting`]: a new backend
    /// must not silently inherit Tesseract's ability to reconstruct reading order on a rotated
    /// raster and then quietly emit garbage the first time it faces one (see the type's doc
    /// comment for the measured A/B behind this). Override this only after validating the
    /// backend's actual behaviour on a rotated page.
    fn page_orientation_handling(&self) -> PageOrientationHandling {
        PageOrientationHandling::RequiresUpright
    }

    /// Process a document file directly via OCR.
    ///
    /// Only called if `supports_document_processing` returns `true`.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the document file (e.g. .pdf)
    /// * `config` - OCR configuration
    async fn process_document(&self, _path: &Path, _config: &OcrConfig) -> Result<ExtractedDocument> {
        Err(crate::XbergError::Other(
            "Document-level OCR processing not supported by this backend".to_string(),
        ))
    }

    /// Optional: Probe whether this backend will actually execute on this host
    /// with the given configuration.
    ///
    /// Used by `doctor` diagnostics. Implementations must not download models
    /// or make billable API calls; anything not yet local is reported as
    /// `ProbeStatus::Skip`. Defaults to `Skip` so custom backends need no
    /// changes; implement it to give users real environment diagnostics.
    ///
    /// Excluded from the polyglot binding surface: doctor results are produced
    /// by the generated `doctor()` function, not per-backend calls.
    #[cfg_attr(alef, alef(skip))]
    fn probe(&self, _config: &OcrConfig) -> crate::doctor::DoctorCheck {
        crate::doctor::DoctorCheck::skip(self.name(), "no probe implemented for this backend")
    }
}

/// Register an OCR backend with the global registry.
///
/// The OCR backend will be registered with its name from the `name()` method
/// and can be used for OCR processing via the extraction pipeline.
///
/// # Arguments
///
/// * `backend` - The OCR backend implementation wrapped in Arc
///
/// # Returns
///
/// - `Ok(())` if registration succeeded
/// - `Err(...)` if validation failed or initialization failed
///
/// # Errors
///
/// - `XbergError::Validation` - Invalid backend name (empty or contains whitespace)
/// - Any error from the backend's `initialize()` method
///
/// # Example
///
/// ```rust
/// use xberg::plugins::{Plugin, OcrBackend, register_ocr_backend, OcrBackendType};
/// use xberg::{Result, OcrConfig};
/// use xberg::types::{ExtractedDocument, Metadata};
/// use async_trait::async_trait;
/// use std::borrow::Cow;
/// use std::sync::Arc;
/// use std::path::Path;
///
/// struct CustomOcr;
///
/// impl Plugin for CustomOcr {
///     fn name(&self) -> &str { "custom-ocr" }
///     fn version(&self) -> String { "1.0.0".to_string() }
///     fn initialize(&self) -> Result<()> { Ok(()) }
///     fn shutdown(&self) -> Result<()> { Ok(()) }
/// }
///
/// #[async_trait]
/// impl OcrBackend for CustomOcr {
///     async fn process_image(&self, _: &[u8], _: &OcrConfig) -> Result<ExtractedDocument> {
///         let mut document = ExtractedDocument::default();
///         document.content = "text".to_string();
///         document.mime_type = Cow::Borrowed("text/plain");
///         Ok(document)
///     }
///     fn supports_language(&self, _: &str) -> bool { true }
///     fn backend_type(&self) -> OcrBackendType { OcrBackendType::Custom }
/// }
///
/// # tokio_test::block_on(async {
/// let backend = Arc::new(CustomOcr);
/// register_ocr_backend(backend)?;
/// # Ok::<(), xberg::XbergError>(())
/// # });
/// ```
#[cfg_attr(alef, alef(skip))]
pub fn register_ocr_backend(backend: Arc<dyn OcrBackend>) -> crate::Result<()> {
    use crate::plugins::registry::get_ocr_backend_registry;

    let registry = get_ocr_backend_registry();
    let mut registry = registry.write();

    registry.register(backend)
}

/// Unregister an OCR backend by name.
///
/// Removes the OCR backend from the global registry and calls its `shutdown()` method.
///
/// # Arguments
///
/// * `name` - Name of the OCR backend to unregister
///
/// # Returns
///
/// - `Ok(())` if the backend was unregistered or didn't exist
/// - `Err(...)` if the shutdown method failed
///
/// # Example
///
/// ```rust
/// use xberg::plugins::unregister_ocr_backend;
///
/// # tokio_test::block_on(async {
/// unregister_ocr_backend("custom-ocr")?;
/// # Ok::<(), xberg::XbergError>(())
/// # });
/// ```
#[cfg_attr(alef, alef(skip))]
pub fn unregister_ocr_backend(name: &str) -> crate::Result<()> {
    use crate::plugins::registry::get_ocr_backend_registry;

    let registry = get_ocr_backend_registry();
    let mut registry = registry.write();

    registry.remove(name)
}

/// List all registered OCR backends.
///
/// Returns the names of all OCR backends currently registered in the global registry.
///
/// # Returns
///
/// A vector of OCR backend names.
///
/// # Example
///
/// ```rust
/// use xberg::plugins::list_ocr_backends;
///
/// # tokio_test::block_on(async {
/// let backends = list_ocr_backends()?;
/// for name in backends {
///     println!("Registered OCR backend: {}", name);
/// }
/// # Ok::<(), xberg::XbergError>(())
/// # });
/// ```
pub fn list_ocr_backends() -> crate::Result<Vec<String>> {
    use crate::plugins::registry::get_ocr_backend_registry;

    let registry = get_ocr_backend_registry();
    let registry = registry.read();

    Ok(registry.list())
}

/// A registered OCR backend's declared name and language capabilities.
///
/// Returned by [`list_ocr_backend_capabilities`]. See that function's documentation for the
/// determinism guarantees and the important caveat about what an empty `supported_languages`
/// means.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct OcrBackendCapabilities {
    /// The backend's registered name, as returned by [`list_ocr_backends`].
    pub name: String,
    /// The languages this backend declares support for, via
    /// [`OcrBackend::supported_languages`].
    ///
    /// An empty list means the backend does not enumerate its languages — it is *not* a
    /// statement that the backend supports no languages. [`OcrBackend::supported_languages`]
    /// is a defaulted trait method that returns `vec![]`, and not every backend overrides it:
    /// the VLM backend (`llm::vlm_ocr::VlmOcrBackend`) accepts every language via
    /// `supports_language` while inheriting the empty default here. Use
    /// [`ocr_backend_supports_language`] to decide whether one specific language is usable —
    /// never infer "unsupported" from an empty list.
    ///
    /// The order of this list is preserved exactly as the backend reported it and is *not*
    /// re-sorted. Tesseract's order comes from enumerating installed tessdata files; PaddleOCR's
    /// comes from its own `SUPPORTED_LANGUAGES` constant. Re-sorting would disagree with the
    /// precedence each backend's own `supports_language` implementation uses internally.
    ///
    /// This is the no-override answer ([`list_ocr_backend_capabilities`]). Use
    /// [`list_ocr_backend_capabilities_for`] when the caller sets `OcrConfig.tessdata_path`: for
    /// Tesseract the language list is a property of the resolved tessdata directory, and the
    /// no-override chain can name a different directory than the one a job using that config
    /// will load from. See GH#1857.
    pub supported_languages: Vec<String>,
}

/// List every registered OCR backend's name alongside its declared supported languages.
///
/// This is the capability-enumeration counterpart to [`list_ocr_backends`]: where that function
/// exposes only backend names, this exposes each backend's `supported_languages()` too, so a
/// consumer (for example, a job-acceptance gate) does not need to hardcode a second list of
/// backend languages.
///
/// # Determinism
///
/// The returned vector is sorted by `name`, regardless of registration order or the order
/// reported by the underlying registry. `supported_languages` within each entry is **not**
/// sorted — see [`OcrBackendCapabilities::supported_languages`] for why.
///
/// # Cost
///
/// Calling this is not free for every backend. In particular, `TesseractBackend`'s
/// `supported_languages()` allocates a Tesseract API and initializes it against the same
/// tessdata directory a real OCR job resolves (`resolve_tessdata_path`), the first time it
/// is called, to enumerate installed tessdata languages; subsequent calls are served from a
/// cache.
///
/// # Errors
///
/// Returns an error only if the registry lock cannot be acquired in the current environment.
///
/// # Example
///
/// ```rust
/// use xberg::plugins::list_ocr_backend_capabilities;
///
/// # tokio_test::block_on(async {
/// for capability in list_ocr_backend_capabilities()? {
///     println!("{}: {:?}", capability.name, capability.supported_languages);
/// }
/// # Ok::<(), xberg::XbergError>(())
/// # });
/// ```
pub fn list_ocr_backend_capabilities() -> crate::Result<Vec<OcrBackendCapabilities>> {
    list_ocr_backend_capabilities_for(&OcrConfig::default())
}

/// [`list_ocr_backend_capabilities`], reporting each backend's languages under `config`.
///
/// Tesseract's language list is a property of the tessdata directory it resolves, and
/// `config.tessdata_path` is the first entry of that search chain. The config-less form always
/// answers for the no-override chain, which can be a different directory than the one a job using
/// `config` will load from. Use this form when `config.tessdata_path` is set. See GH#1857.
pub fn list_ocr_backend_capabilities_for(config: &OcrConfig) -> crate::Result<Vec<OcrBackendCapabilities>> {
    use crate::plugins::registry::get_ocr_backend_registry;

    let registry = get_ocr_backend_registry();
    let registry = registry.read();

    Ok(capabilities_from_snapshot(registry.registered_snapshot(), config))
}

/// Pure mapping from a registry snapshot to sorted capability records.
///
/// Split out from [`list_ocr_backend_capabilities`] so tests can exercise the mapping and the
/// sort-by-name contract with local mock backends, without mutating the process-global OCR
/// registry (which other test modules also read and write concurrently).
fn capabilities_from_snapshot(
    registered: Vec<(String, Arc<dyn OcrBackend>)>,
    config: &OcrConfig,
) -> Vec<OcrBackendCapabilities> {
    let mut capabilities: Vec<OcrBackendCapabilities> = registered
        .into_iter()
        .map(|(name, backend)| OcrBackendCapabilities {
            name,
            supported_languages: backend.supported_languages_for(config),
        })
        .collect();
    capabilities.sort_unstable_by(|left, right| left.name.cmp(&right.name));
    capabilities
}

/// Check whether a specific registered OCR backend supports a language.
///
/// Delegates to the named backend's own [`OcrBackend::supports_language`], which is the correct
/// per-language decision — do not infer support (or its absence) from whether
/// [`list_ocr_backend_capabilities`] reports an empty `supported_languages` list for that
/// backend, since an empty list can mean "does not enumerate" rather than "supports nothing"
/// (see [`OcrBackendCapabilities::supported_languages`]).
///
/// # Arguments
///
/// * `backend` - Name of a registered OCR backend, as returned by [`list_ocr_backends`]. Lookup
///   is case-insensitive and resolves the same `paddleocr` alias as backend dispatch.
/// * `language` - Language code to check (e.g. `"eng"`, `"deu"`).
///
/// # Errors
///
/// Returns an error if no backend with that name (or alias) is registered.
///
/// # Example
///
/// ```rust
/// use xberg::plugins::ocr_backend_supports_language;
///
/// # tokio_test::block_on(async {
/// let supported = ocr_backend_supports_language("tesseract", "eng")?;
/// # Ok::<(), xberg::XbergError>(())
/// # });
/// ```
pub fn ocr_backend_supports_language(backend: &str, language: &str) -> crate::Result<bool> {
    ocr_backend_supports_language_for(backend, language, &OcrConfig::default())
}

/// [`ocr_backend_supports_language`], answering under `config`.
///
/// Use this, not the config-less form, when the caller sets `OcrConfig.tessdata_path`: the
/// config-less form checks the no-override search chain and can deny a language the job would
/// load without trouble. See GH#1857.
pub fn ocr_backend_supports_language_for(backend: &str, language: &str, config: &OcrConfig) -> crate::Result<bool> {
    use crate::plugins::registry::get_ocr_backend_registry;

    let registry = get_ocr_backend_registry();
    let registry = registry.read();
    let registered = registry.registered_snapshot();

    let canonical = crate::plugins::registry::canonical_ocr_backend_name(backend);

    registered
        .iter()
        .find(|(name, _)| name.as_str() == backend)
        .or_else(|| registered.iter().find(|(name, _)| name.as_str() == canonical.as_str()))
        .map(|(_, instance)| instance.supports_language_for(config, language))
        .ok_or_else(|| crate::XbergError::Plugin {
            message: format!(
                "OCR backend '{backend}' not registered. Available backends: {:?}",
                registered.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>()
            ),
            plugin_name: backend.to_string(),
        })
}

/// Clear all OCR backends from the global registry.
///
/// Removes all OCR backends and calls their `shutdown()` methods.
///
/// # Returns
///
/// - `Ok(())` if all backends were cleared successfully
/// - `Err(...)` if any shutdown method failed
///
/// # Example
///
/// ```rust
/// use xberg::plugins::clear_ocr_backends;
///
/// # tokio_test::block_on(async {
/// clear_ocr_backends()?;
/// # Ok::<(), xberg::XbergError>(())
/// # });
/// ```
pub fn clear_ocr_backends() -> crate::Result<()> {
    use crate::plugins::registry::get_ocr_backend_registry;

    let registry = get_ocr_backend_registry();
    let mut registry = registry.write();

    registry.shutdown_all()
}

/// Ensure the global OCR backend registry has its built-in backends registered.
///
/// The global registry is seeded with the built-in backends (Tesseract,
/// PaddleOCR, VLM — gated by feature flags) when it is first constructed.
/// However, [`clear_ocr_backends`] empties the registry, leaving subsequent
/// OCR operations with no backend to dispatch to.
///
/// This function is the self-healing counterpart, mirroring
/// `crate::extractors::ensure_initialized` for the document extractor registry:
/// it re-registers the built-in backends whenever the built-in default is
/// missing so that callers always see a usable registry. It re-seeds not only
/// when the registry is empty but also when it is non-empty yet missing the
/// built-in default (e.g. after [`clear_ocr_backends`] followed by registering
/// a *different* backend) — the plain "empty" check would leave default-config
/// OCR without a backend. Re-seeding is non-destructive (user-registered
/// backends are kept) and cheap to invoke before every OCR dispatch.
#[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
pub(crate) fn ensure_ocr_backends_initialized() {
    use crate::plugins::registry::get_ocr_backend_registry;

    let registry = get_ocr_backend_registry();

    {
        let registry = registry.read();
        if !registry.is_missing_default_backend() {
            return;
        }
    }

    registry.write().ensure_defaults();
}

#[cfg(test)]
mod tests;
