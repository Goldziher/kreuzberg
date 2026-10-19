//! Native Tesseract OCR backend.
//!
//! This module provides the native Tesseract backend that implements the OcrBackend
//! trait, bridging the plugin system with the low-level OcrProcessor.

use crate::Result;
use crate::core::config::OcrConfig;
use crate::ocr::processor::OcrProcessor;
use crate::ocr::processor::validation::TessdataEnv;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractedDocument;
use ahash::AHashMap;
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const USE_CACHE_BACKEND_OPTION: &str = "use_cache";

/// Memo key for the available-language probe: `(probe languages, tessdata_path override,
/// TESSDATA_PREFIX, XBERG_CACHE_DIR)`.
///
/// Every component is something tessdata resolution actually reads, so two calls differing in any
/// one of them can resolve different directories and must not serve each other's answer. Keying on
/// the override alone (GH#1857's fix) let two calls under different environments, or for different
/// languages, collide. GH#1891. ~keep
type LanguagesMemoKey = (Vec<String>, Option<PathBuf>, Option<String>, Option<PathBuf>);

fn select_output_ocr_elements(
    elements: Option<Vec<crate::types::OcrElement>>,
    config: &OcrConfig,
) -> Option<Vec<crate::types::OcrElement>> {
    let options = config.element_config.as_ref()?;
    let selected = options.select_elements(elements.as_deref().unwrap_or_default());
    (!selected.is_empty()).then_some(selected)
}

use crate::ocr::types::TesseractConfig as InternalTesseractConfig;

/// Native Tesseract OCR backend.
///
/// This backend wraps the OcrProcessor and implements the OcrBackend trait,
/// allowing it to be used through the plugin system.
///
/// # Thread Safety
///
/// Uses Arc for shared ownership and is thread-safe (Send + Sync).
///
/// # Lazy Initialization
///
/// The native Tesseract/Leptonica FFI handle is allocated on first use,
/// not at backend construction. This allows the registry to be built without
/// triggering expensive native initialization.
#[cfg_attr(alef, alef(skip))]
pub struct TesseractBackend {
    processor: OnceCell<Arc<OcrProcessor>>,
    // Keyed by everything the answer depends on rather than a single cell: the language list is a
    // property of the resolved tessdata directory, and one cell would serve whichever caller asked
    // first to every later one. See GH#1857 for the override half and [`LanguagesMemoKey`] for why
    // the key is not the override alone. ~keep
    available_languages: RwLock<AHashMap<LanguagesMemoKey, Arc<[String]>>>,
    #[cfg(not(target_arch = "wasm32"))]
    concurrency: OnceCell<Arc<tokio::sync::Semaphore>>,
}

impl TesseractBackend {
    /// Create a new Tesseract backend wrapper (infallible).
    ///
    /// The actual FFI handle is allocated lazily on first use via
    /// `processor()`.
    pub(crate) fn new() -> Self {
        Self {
            processor: OnceCell::new(),
            available_languages: RwLock::new(AHashMap::new()),
            #[cfg(not(target_arch = "wasm32"))]
            concurrency: OnceCell::new(),
        }
    }

    /// The semaphore that holds async callers back to the recognition limit.
    ///
    /// Built on first use, like the processor above: the registry constructs
    /// this backend before any extraction resolves a thread budget, and sizing
    /// the semaphore here keeps it from fixing the limit while the configured
    /// one is still unknown.
    #[cfg(not(target_arch = "wasm32"))]
    fn concurrency(&self) -> &Arc<tokio::sync::Semaphore> {
        self.concurrency.get_or_init(|| {
            Arc::new(tokio::sync::Semaphore::new(
                crate::ocr::processor::tesseract_api_capacity(),
            ))
        })
    }

    /// Get or initialize the Tesseract processor.
    ///
    /// Allocates the native FFI handle on first call; subsequent calls reuse
    /// the cached processor.
    fn processor(&self) -> Result<&Arc<OcrProcessor>> {
        self.processor.get_or_try_init(|| {
            OcrProcessor::new(None)
                .map(Arc::new)
                .map_err(|e| crate::XbergError::Ocr {
                    message: format!("Failed to create Tesseract processor: {}", e),
                    source: Some(Box::new(e)),
                })
        })
    }

    #[cfg(test)]
    pub(crate) fn processor_is_initialized(&self) -> bool {
        self.processor.get().is_some()
    }

    /// Convert OcrConfig to internal TesseractConfig.
    ///
    /// Uses tesseract_config from OcrConfig if provided, otherwise uses defaults
    /// with the language from OcrConfig. Multi-language configs are joined with "+".
    fn config_to_tesseract(&self, config: &OcrConfig) -> InternalTesseractConfig {
        let mut internal = match &config.tesseract_config {
            Some(tess_config) => InternalTesseractConfig::from(tess_config),
            None => InternalTesseractConfig::default(),
        };
        // `TesseractConfig::from` above takes its language from `tess_config.language`, which
        // silently discards `OcrConfig.language` (#1572). Reconcile the two through the shared
        // rule so an `OcrConfig(language=..., tesseract_config=...)` caller is not OCR'd in
        // English regardless of which field they set.
        internal.language = config.effective_tesseract_language().join("+");
        if internal.language.trim().is_empty() {
            internal.language = crate::core::config::ocr::DEFAULT_OCR_LANGUAGE.to_string();
        }
        if config.auto_rotate {
            internal.auto_rotate = true;
        }
        // GH#1651: without this the caller's limits die here -- `TesseractConfig` is the only
        // channel from `OcrConfig` down to the decode, because `OcrBackend::process_image`
        // receives no `ExtractionConfig`. ~keep
        internal.security_limits = config.security_limits.clone();
        internal.tessdata_path = config.tessdata_path.clone();
        internal.source_dpi = Self::source_dpi_from_backend_options(config);
        internal.known_full_page_scan = Self::known_full_page_scan_from_backend_options(config);
        if let Some(use_cache) = Self::use_cache_from_backend_options(config) {
            internal.use_cache = use_cache;
        }
        internal
    }

    /// Read the per-call result-cache override from `backend_options`.
    ///
    /// This keeps callers that only need a cold Tesseract invocation from
    /// materializing `tesseract_config`, which would make automatic whole-image
    /// PSM selection and its sparse-image fallback look explicitly configured.
    fn use_cache_from_backend_options(config: &OcrConfig) -> Option<bool> {
        config
            .backend_options
            .as_ref()
            .and_then(|options| options.get(USE_CACHE_BACKEND_OPTION))
            .and_then(serde_json::Value::as_bool)
    }

    /// Read the per-call source-resolution hint out of `backend_options`.
    ///
    /// Mirrors `PaddleOcrBackend::page_rotation_degrees_from_backend_options`: an absent,
    /// malformed, or non-positive value is treated as "unknown" rather than as an error, so
    /// callers that never set the hint (standalone image OCR, other backends' tests reusing this
    /// config) keep the historical 72-DPI assumption and nothing about their behaviour changes.
    ///
    /// Non-finite and non-positive values are rejected because they would propagate into the
    /// `target_dpi / source_dpi` scale factor as a NaN or a negative resize.
    fn source_dpi_from_backend_options(config: &OcrConfig) -> Option<f64> {
        // Delegates rather than repeating the read: the extractor boundary resolves the same
        // override before resizing (GH#1630), and two readers of one option that validate it
        // independently are the drift shape GH#1621 was caused by. ~keep
        crate::extraction::image::explicit_source_dpi_from_ocr_config(config)
    }

    /// Read the PDF OCR route's scan-detection signal out of `backend_options` (GH#1894).
    ///
    /// An absent or non-boolean value is "not a known scan", matching every non-PDF caller
    /// (standalone image OCR, direct API callers), which never stamp this key and must keep
    /// deciding preprocessing from the pixel-brightness heuristic.
    fn known_full_page_scan_from_backend_options(config: &OcrConfig) -> bool {
        config
            .backend_options
            .as_ref()
            .and_then(|options| options.get(crate::core::config::ocr::KNOWN_FULL_PAGE_SCAN_BACKEND_OPTION))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    }

    /// Get cached available languages, lazily querying Tesseract if needed.
    ///
    /// `languages` are the languages the caller's config asks for, `override_path` is
    /// `OcrConfig.tessdata_path` (or `None`). Both, plus the two environment values on
    /// `tessdata_env`, take part in tessdata resolution, so all four make up the memo key -- see
    /// [`LanguagesMemoKey`]. Falls back to a hardcoded language list if dynamic querying fails.
    ///
    /// ~keep The key carries the languages the probe will actually resolve with, not the caller's
    /// raw request: every language no searched directory holds collapses onto the same English
    /// datapath and therefore the same answer, so a caller walking a long language list through
    /// `supports_language` allocates one Tesseract engine rather than one per language.
    fn get_cached_languages(
        &self,
        languages: &[String],
        override_path: Option<&Path>,
        tessdata_env: &TessdataEnv,
    ) -> Arc<[String]> {
        let probe_languages = Self::probe_languages(languages, override_path, tessdata_env);
        let key = Self::languages_memo_key(&probe_languages, override_path, tessdata_env);
        if let Some(cached) = self.available_languages.read().get(&key) {
            return Arc::clone(cached);
        }
        // The write lock is deliberately not held across the query below: it allocates a native
        // Tesseract engine and can trigger a language-pack download, and blocking every other
        // caller of this backend on that is worse than a concurrent miss querying the same
        // directory twice. The first writer wins; both answers enumerate the same directory. ~keep
        let available: Arc<[String]> =
            match Self::query_available_languages(&probe_languages, override_path, tessdata_env) {
                Ok(available) => Arc::from(available),
                Err(_) => Arc::from(Self::fallback_languages()),
            };
        Arc::clone(self.available_languages.write().entry(key).or_insert(available))
    }

    /// Build the memo key for one probe. `languages` are the probe languages
    /// [`Self::probe_languages`] selected, not the caller's raw request. See [`LanguagesMemoKey`].
    fn languages_memo_key(
        languages: &[String],
        override_path: Option<&Path>,
        tessdata_env: &TessdataEnv,
    ) -> LanguagesMemoKey {
        (
            languages.to_vec(),
            override_path.map(Path::to_path_buf),
            tessdata_env.tessdata_prefix.clone(),
            tessdata_env.cache_dir.clone(),
        )
    }

    /// The languages to resolve the probe's tessdata directory with.
    ///
    /// The caller's own languages when some already-present directory holds them all, and
    /// [`crate::core::config::ocr::DEFAULT_OCR_LANGUAGE`] otherwise. The fallback is what keeps
    /// this probe from resolving a directory it would first have to download into: a caller asking
    /// about a language this machine does not have gets the historical English-datapath
    /// enumeration, which answers "no" without touching the network. GH#1891. ~keep
    fn probe_languages(languages: &[String], override_path: Option<&Path>, tessdata_env: &TessdataEnv) -> Vec<String> {
        let resolvable = !languages.is_empty()
            && crate::ocr::processor::validation::existing_tessdata_dir_for(languages, override_path, tessdata_env)
                .is_some();
        if resolvable {
            languages.to_vec()
        } else {
            vec![crate::core::config::ocr::DEFAULT_OCR_LANGUAGE.to_string()]
        }
    }

    /// Query available languages from the Tesseract API.
    ///
    /// Creates a temporary Tesseract API instance, initializes it against the same tessdata
    /// directory a real job asking for `probe_languages` would resolve, and enumerates that
    /// directory.
    ///
    /// `probe_languages` must already have been through [`Self::probe_languages`]: that is what
    /// keeps a language no searched directory holds from sending resolution down its
    /// download-and-materialize path.
    ///
    /// # Returns
    ///
    /// Returns a vector of available language codes, or an error if querying fails.
    fn query_available_languages(
        probe_languages: &[String],
        override_path: Option<&Path>,
        tessdata_env: &TessdataEnv,
    ) -> Result<Vec<String>> {
        // An empty datapath here used to hand libtesseract its own compiled-in default,
        // which appends an extra `tessdata` directory level that the real OCR job's
        // resolver (`resolve_tessdata_path`) never adds. That mismatch made this probe
        // fail and log a misleading "couldn't load any languages" error on a layout where
        // every real job already succeeds. Resolving through the same function the job
        // uses keeps the two in agreement. See GH#1671.
        //
        // `override_path` is threaded through as the same first argument a real job passes
        // (`processor/execution.rs`'s `config.tessdata_path.as_deref()`); a hardcoded `None`
        // here searched a different chain than the job, so this probe could deny a language the
        // job loads without trouble -- the agreement GH#1671 established, broken for callers with
        // a configured `tessdata_path`. See GH#1857.
        //
        // The languages are threaded through for the same reason the override is, and it is the
        // same agreement: `tessdata_search_dirs` accepts a candidate directory only when
        // `all_languages_exist` holds for the languages it is given, so a hardcoded `eng` here
        // rejected a `tessdata_path` holding only the caller's own language, fell through to
        // another directory, and denied a language every real job with that config loads. See
        // GH#1891. `probe_languages` is what keeps the fallback from downloading. ~keep
        let tessdata_path =
            crate::ocr::processor::validation::resolve_tessdata_path_in(probe_languages, override_path, tessdata_env)
                .map_err(|e| crate::XbergError::Ocr {
                message: format!("Failed to resolve tessdata path for language query: {}", e),
                source: Some(Box::new(e)),
            })?;

        // ~keep Initialize with a language the resolved directory is known to hold, not a
        // hardcoded `eng`: `resolve_tessdata_path_in` guarantees that only for the languages it
        // was asked about, so on a datapath holding just the caller's language `init` with `eng`
        // fails and the probe falls back to the hardcoded list.
        let init_language = probe_languages
            .first()
            .map_or(crate::core::config::ocr::DEFAULT_OCR_LANGUAGE, String::as_str);

        let api = xberg_tesseract::TesseractAPI::new().map_err(|e| crate::XbergError::Ocr {
            message: format!("Failed to allocate Tesseract engine: {}", e),
            source: Some(Box::new(e)),
        })?;
        api.init(&tessdata_path, init_language)
            .map_err(|e| crate::XbergError::Ocr {
                message: format!("Failed to initialize Tesseract for language query: {}", e),
                source: Some(Box::new(e)),
            })?;

        api.get_available_languages().map_err(|e| crate::XbergError::Ocr {
            message: format!("Failed to query available Tesseract languages: {}", e),
            source: Some(Box::new(e)),
        })
    }

    /// Fallback list of supported languages (hardcoded list).
    ///
    /// Used when dynamic language querying fails, ensuring the backend
    /// always has a sensible default set of languages.
    fn fallback_languages() -> Vec<String> {
        vec![
            "eng", "deu", "fra", "spa", "ita", "por", "rus", "chi_sim", "chi_tra", "jpn", "jpn_vert", "kor", "ara",
            "hin", "ben", "tha", "vie", "heb", "tur", "pol", "nld", "swe", "dan", "fin", "nor", "ces", "hun", "ron",
            "ukr", "bul", "hrv", "srp", "slk", "slv", "lit", "lav", "est",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }
}

impl Default for TesseractBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a backend-internal [`crate::types::OcrTable`] into the public
/// [`crate::types::Table`], assigning a deterministic `table_id`.
///
/// `index` is the table's 0-based position among the tables returned for this
/// single OCR call (i.e. document push order for this call), so the id is
/// `"table-{index + 1}"` — never derived from randomness or wall-clock time,
/// so the same input always produces the same id. See
/// `crate::types::Table::table_id` for the shared scheme doc.
fn convert_ocr_table(index: usize, table: crate::types::OcrTable) -> crate::types::Table {
    let bounding_box = table.bounding_box.map(|bbox| crate::types::BoundingBox {
        x0: bbox.left as f64,
        y0: bbox.top as f64,
        x1: bbox.right as f64,
        y1: bbox.bottom as f64,
    });
    let columns = table.cells.first().cloned();
    crate::types::Table {
        cells: table.cells,
        markdown: table.markdown,
        page_number: table.page_number,
        bounding_box,
        table_id: Some(format!("table-{}", index + 1)),
        columns,
        cell_styles: Vec::new(),
    }
}

impl Plugin for TesseractBackend {
    fn name(&self) -> &str {
        "tesseract"
    }

    fn version(&self) -> String {
        xberg_tesseract::TesseractAPI::version()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        if let Some(processor) = self.processor.get() {
            processor.clear_cache().map_err(|e| crate::XbergError::Plugin {
                message: format!("Failed to clear Tesseract cache: {}", e),
                plugin_name: "tesseract".to_string(),
            })
        } else {
            Ok(())
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl OcrBackend for TesseractBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractedDocument> {
        self.process_image_owned(Arc::new(image_bytes.to_vec()), config).await
    }

    async fn process_image_owned(&self, image_bytes: Arc<Vec<u8>>, config: &OcrConfig) -> Result<ExtractedDocument> {
        let tess_config = self.config_to_tesseract(config);
        let tess_config_clone = tess_config.clone();
        let output_format = config.output_format.clone();

        let processor = Arc::clone(self.processor()?);

        #[cfg(not(target_arch = "wasm32"))]
        let permit = Arc::clone(self.concurrency())
            .acquire_owned()
            .await
            .map_err(|error| crate::XbergError::Ocr {
                message: format!("Tesseract concurrency limiter closed unexpectedly: {error}"),
                source: None,
            })?;

        let operation = move || {
            #[cfg(not(target_arch = "wasm32"))]
            let _permit = permit;
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match output_format {
                Some(fmt) => processor.process_image_with_format(image_bytes.as_slice(), &tess_config_clone, fmt),
                None => processor.process_image(image_bytes.as_slice(), &tess_config_clone),
            }))
            .unwrap_or_else(|_| {
                Err(crate::ocr::error::OcrError::ProcessingFailed(
                    "Tesseract/Leptonica foreign exception caught".to_string(),
                ))
            })
        };
        #[cfg(not(target_arch = "wasm32"))]
        let ocr_result = tokio::task::spawn_blocking(operation)
            .await
            .map_err(|e| crate::XbergError::Plugin {
                message: format!("Tesseract task panicked or caught foreign exception: {}", e),
                plugin_name: "tesseract".to_string(),
            })?;
        #[cfg(target_arch = "wasm32")]
        let ocr_result = operation();
        let mut ocr_result = ocr_result.map_err(|e| crate::XbergError::Ocr {
            message: format!("Tesseract OCR failed: {}", e),
            source: Some(Box::new(e)),
        })?;
        normalize_vertical_cjk_result(&mut ocr_result, &tess_config.language, &tess_config.output_format);

        let resolved_language = ocr_result
            .metadata
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or(&tess_config.language)
            .to_string();

        let pre_formatted = extract_pre_formatted_metadata(&mut ocr_result.metadata);
        let image_preprocessing = extract_image_preprocessing_metadata(&mut ocr_result.metadata);

        let processing_warnings = warnings_from_ocr_metadata(&ocr_result.metadata);
        strip_ocr_scratch_metadata_keys(&mut ocr_result.metadata);
        let ocr_elements = select_output_ocr_elements(ocr_result.ocr_elements.take(), config);

        let mut additional = AHashMap::new();
        for (key, value) in ocr_result.metadata {
            additional.insert(Cow::Owned(key), value);
        }

        let metadata = crate::types::Metadata {
            format: Some(crate::types::FormatMetadata::Ocr(crate::types::OcrMetadata {
                language: resolved_language,
                psm: tess_config.psm as i32,
                output_format: tess_config.output_format.clone(),
                table_count: ocr_result.tables.len() as u32,
                table_rows: ocr_result.tables.first().map(|t| t.cells.len() as u32),
                table_cols: ocr_result
                    .tables
                    .first()
                    .and_then(|t| t.cells.first().map(|row| row.len() as u32)),
            })),
            output_format: pre_formatted,
            image_preprocessing,
            additional,
            ..Default::default()
        };

        Ok(ExtractedDocument {
            content: ocr_result.content,
            mime_type: ocr_result.mime_type.into(),
            metadata,
            tables: ocr_result
                .tables
                .into_iter()
                .enumerate()
                .map(|(index, t)| convert_ocr_table(index, t))
                .collect(),
            ocr_elements,
            ocr_internal_document: ocr_result.internal_document,
            processing_warnings,
            ..Default::default()
        })
    }

    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractedDocument> {
        let tess_config = self.config_to_tesseract(config);
        let tess_config_clone = tess_config.clone();
        let output_format = config.output_format.clone();

        let processor = Arc::clone(self.processor()?);
        let path_str = path.to_string_lossy().to_string();

        #[cfg(not(target_arch = "wasm32"))]
        let permit = Arc::clone(self.concurrency())
            .acquire_owned()
            .await
            .map_err(|error| crate::XbergError::Ocr {
                message: format!("Tesseract concurrency limiter closed unexpectedly: {error}"),
                source: None,
            })?;

        let operation = move || {
            #[cfg(not(target_arch = "wasm32"))]
            let _permit = permit;
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match output_format {
                Some(fmt) => processor.process_image_file_with_format(&path_str, &tess_config_clone, fmt),
                None => processor.process_image_file(&path_str, &tess_config_clone),
            }))
            .unwrap_or_else(|_| {
                Err(crate::ocr::error::OcrError::ProcessingFailed(
                    "Tesseract/Leptonica foreign exception caught".to_string(),
                ))
            })
        };
        #[cfg(not(target_arch = "wasm32"))]
        let ocr_result = tokio::task::spawn_blocking(operation)
            .await
            .map_err(|e| crate::XbergError::Plugin {
                message: format!("Tesseract task panicked or caught foreign exception: {}", e),
                plugin_name: "tesseract".to_string(),
            })?;
        #[cfg(target_arch = "wasm32")]
        let ocr_result = operation();
        let mut ocr_result = ocr_result.map_err(|e| crate::XbergError::Ocr {
            message: format!("Tesseract OCR failed: {}", e),
            source: Some(Box::new(e)),
        })?;
        normalize_vertical_cjk_result(&mut ocr_result, &tess_config.language, &tess_config.output_format);

        let resolved_language = ocr_result
            .metadata
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or(&tess_config.language)
            .to_string();

        let pre_formatted = extract_pre_formatted_metadata(&mut ocr_result.metadata);
        let image_preprocessing = extract_image_preprocessing_metadata(&mut ocr_result.metadata);

        let processing_warnings = warnings_from_ocr_metadata(&ocr_result.metadata);
        strip_ocr_scratch_metadata_keys(&mut ocr_result.metadata);
        let ocr_elements = select_output_ocr_elements(ocr_result.ocr_elements.take(), config);

        let mut additional = AHashMap::new();
        for (key, value) in ocr_result.metadata {
            additional.insert(Cow::Owned(key), value);
        }

        let metadata = crate::types::Metadata {
            format: Some(crate::types::FormatMetadata::Ocr(crate::types::OcrMetadata {
                language: resolved_language,
                psm: tess_config.psm as i32,
                output_format: tess_config.output_format.clone(),
                table_count: ocr_result.tables.len() as u32,
                table_rows: ocr_result.tables.first().map(|t| t.cells.len() as u32),
                table_cols: ocr_result
                    .tables
                    .first()
                    .and_then(|t| t.cells.first().map(|row| row.len() as u32)),
            })),
            output_format: pre_formatted,
            image_preprocessing,
            additional,
            ..Default::default()
        };

        Ok(ExtractedDocument {
            content: ocr_result.content,
            mime_type: ocr_result.mime_type.into(),
            metadata,
            tables: ocr_result
                .tables
                .into_iter()
                .enumerate()
                .map(|(index, t)| convert_ocr_table(index, t))
                .collect(),
            ocr_elements,
            ocr_internal_document: ocr_result.internal_document,
            processing_warnings,
            ..Default::default()
        })
    }

    /// ~keep Probes for `lang` itself, not for English: with no config there is no other statement
    /// of intent, and a datapath holding only `lang` is exactly the layout GH#1891 is about. The
    /// answer is still read off the enumerated directory, so an absent language still reads `false`
    /// -- `probe_languages` declines to resolve one that is nowhere on disk.
    fn supports_language(&self, lang: &str) -> bool {
        self.get_cached_languages(&[lang.to_string()], None, &TessdataEnv::from_process())
            .iter()
            .any(|language| language == lang)
    }

    /// ~keep Probes with the config's own effective languages, which is what a real job with this
    /// config resolves with (`processor/execution.rs`); a hardcoded `eng` made this deny a language
    /// the job loads without trouble whenever `tessdata_path` held only that language. GH#1891.
    fn supports_language_for(&self, config: &OcrConfig, language: &str) -> bool {
        self.get_cached_languages(
            &config.effective_tesseract_language(),
            config.tessdata_path.as_deref(),
            &TessdataEnv::from_process(),
        )
        .iter()
        .any(|candidate| candidate == language)
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Tesseract
    }

    /// ~keep No config and no language to ask about, so this keeps probing the English datapath:
    /// there is nothing else to resolve with. `supported_languages_for` is the accurate answer for
    /// a caller that has a config.
    fn supported_languages(&self) -> Vec<String> {
        self.get_cached_languages(
            &[crate::core::config::ocr::DEFAULT_OCR_LANGUAGE.to_string()],
            None,
            &TessdataEnv::from_process(),
        )
        .to_vec()
    }

    fn supported_languages_for(&self, config: &OcrConfig) -> Vec<String> {
        self.get_cached_languages(
            &config.effective_tesseract_language(),
            config.tessdata_path.as_deref(),
            &TessdataEnv::from_process(),
        )
        .to_vec()
    }

    fn supports_table_detection(&self) -> bool {
        true
    }

    /// Tesseract's mean per-word classifier confidence, 0-100, is validated to track
    /// legibility: on a scanned ordinance, prose pages scored 89-95 and pure line-art
    /// drawings scored 36-62. It is safe to use as an absolute quality gate.
    fn confidence_semantics(&self) -> crate::plugins::ConfidenceSemantics {
        crate::plugins::ConfidenceSemantics::Legibility { scale_max: 100.0 }
    }

    /// The raster the pipeline hands a backend is in the page's raw MediaBox axes, which for a
    /// `/Rotate`d page is sideways. Default PSM 3 does not compensate for that (a display-rotated
    /// bid table scored 0.35 page confidence sideways and 0.87 upright, #2172), and the
    /// `auto_rotate` detector is optional, so the upright-raster route must always apply. ~keep
    fn page_orientation_handling(&self) -> crate::plugins::PageOrientationHandling {
        crate::plugins::PageOrientationHandling::RequiresUpright
    }

    #[cfg_attr(alef, alef(skip))]
    fn probe(&self, config: &OcrConfig) -> crate::doctor::DoctorCheck {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = config;
            crate::doctor::DoctorCheck::skip("ocr.tesseract", "tessdata probe is not available on wasm32")
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            probe_tessdata(config)
        }
    }
}

/// Check-only tessdata probe: mirrors the runtime resolution chain without
/// materializing or downloading language packs.
///
/// The runtime requires ONE directory containing every requested language
/// (`resolve_tessdata_path`); the probe applies the same rule, then
/// distinguishes "known language, would download on first use" (skip) from
/// "unknown language code, download would also fail" (fail).
#[cfg(not(target_arch = "wasm32"))]
fn probe_tessdata(config: &OcrConfig) -> crate::doctor::DoctorCheck {
    let dirs = crate::ocr::processor::validation::tessdata_search_dirs(
        config.tessdata_path.as_deref(),
        &TessdataEnv::from_process(),
    );
    probe_tessdata_in_dirs(config, &dirs)
}

#[cfg(not(target_arch = "wasm32"))]
fn probe_tessdata_in_dirs(config: &OcrConfig, dirs: &[String]) -> crate::doctor::DoctorCheck {
    use crate::doctor::DoctorCheck;
    use crate::ocr::validation::TESSERACT_SUPPORTED_LANGUAGE_CODES;

    let version = xberg_tesseract::TesseractAPI::version();
    let languages = config.effective_languages();

    if let Some(dir) = dirs.iter().find(|dir| {
        languages
            .iter()
            .all(|lang| std::path::Path::new(dir).join(format!("{lang}.traineddata")).exists())
    }) {
        return DoctorCheck::pass(
            "ocr.tesseract",
            format!(
                "tesseract {version}; tessdata for {} language(s) at {dir}",
                languages.len()
            ),
        );
    }

    let missing: Vec<&str> = languages
        .iter()
        .map(String::as_str)
        .filter(|lang| {
            !dirs
                .iter()
                .any(|dir| std::path::Path::new(dir).join(format!("{lang}.traineddata")).exists())
        })
        .collect();

    let (unknown, downloadable): (Vec<&str>, Vec<&str>) = missing
        .iter()
        .copied()
        .partition(|lang| !TESSERACT_SUPPORTED_LANGUAGE_CODES.contains(lang));

    if !unknown.is_empty() {
        return DoctorCheck::fail(
            "ocr.tesseract",
            format!(
                "unknown language code(s): {} (no traineddata exists)",
                unknown.join(", ")
            ),
        );
    }

    DoctorCheck::skip(
        "ocr.tesseract",
        format!(
            "tessdata for [{}] not found locally (will download on first use)",
            downloadable.join(", ")
        ),
    )
}

fn normalize_vertical_cjk_result(result: &mut crate::types::OcrExtractionResult, language: &str, output_format: &str) {
    if matches!(output_format, "hocr" | "tsv")
        || !language
            .split('+')
            .any(|code| code.to_ascii_lowercase().ends_with("_vert"))
    {
        return;
    }
    result.content = compact_cjk_horizontal_spacing(&result.content);
    for table in &mut result.tables {
        for row in &mut table.cells {
            for cell in row {
                *cell = compact_cjk_horizontal_spacing(cell);
            }
        }
        table.markdown = compact_cjk_horizontal_spacing(&table.markdown);
    }
    if let Some(document) = result.internal_document.as_mut() {
        for (index, element) in document.elements.iter_mut().enumerate() {
            element.text = compact_cjk_horizontal_spacing(&element.text);
            element.id = crate::types::internal::InternalElementId::generate(
                element.kind.discriminant(),
                &element.text,
                element.page,
                index as u32,
            );
        }
    }
}

/// Metadata key `perform_ocr` sets when the Tesseract result iterator dropped
/// one or more words (null pointer, invalid parameter, or invalid UTF-8)
/// rather than including them in `ocr_elements`. Mirrors the literal written
/// in `ocr::processor::execution::insert_word_iterator_skipped_count_metadata`.
const WORD_ITERATOR_SKIPPED_COUNT_METADATA_KEY: &str = "word_iterator_skipped_count";

/// Metadata key `perform_ocr` sets when `auto_rotate` was requested but the
/// `auto-rotate` build feature is not compiled in, so orientation detection
/// never ran. Mirrors the literal written in `ocr::processor::execution::perform_ocr`.
const AUTO_ROTATE_UNAVAILABLE_METADATA_KEY: &str = "auto_rotate_unavailable";

/// Metadata key carrying the exact number of hOCR lines removed by dictionary filtering. ~keep
const DICTIONARY_FILTERED_LINE_COUNT_METADATA_KEY: &str = "dictionary_filtered_line_count";

/// Metadata key `perform_ocr` sets when it rebuilds `content` with inline
/// table markdown at each table's original vertical position (see
/// `perform_ocr`'s `build_content_with_inline_tables` step). Promoted to
/// `Metadata::output_format` by [`extract_pre_formatted_metadata`] rather
/// than left as a raw `Metadata::additional` entry (#354).
const PRE_FORMATTED_METADATA_KEY: &str = "pre_formatted";

/// Pull the `pre_formatted` marker out of an `OcrExtractionResult`'s metadata
/// map, returning its string value if present.
///
/// Uses `.remove()`, not `.get()`, so the key never also survives into the
/// user-visible `Metadata::additional` map once the remaining metadata is
/// copied wholesale (#354). Extracted as its own pure function, mirroring
/// `warnings_from_ocr_metadata`, so the removal is unit-testable without a
/// live Tesseract API instance.
fn extract_pre_formatted_metadata(
    metadata: &mut std::collections::HashMap<String, serde_json::Value>,
) -> Option<String> {
    metadata
        .remove(PRE_FORMATTED_METADATA_KEY)
        .and_then(|v| v.as_str().map(str::to_string))
}

fn extract_image_preprocessing_metadata(
    metadata: &mut std::collections::HashMap<String, serde_json::Value>,
) -> Option<crate::types::ImagePreprocessingMetadata> {
    let value = metadata.remove(crate::ocr_metadata_keys::OCR_IMAGE_PREPROCESSING_METADATA_KEY)?;
    match serde_json::from_value(value) {
        Ok(metadata) => Some(metadata),
        Err(error) => {
            tracing::warn!(%error, "discarding invalid OCR image preprocessing metadata");
            None
        }
    }
}

/// Turn the OCR backend's metadata side-channel into the `ProcessingWarning`s a
/// caller of `ExtractedDocument` actually sees (#309).
///
/// `OcrExtractionResult` has no dedicated warnings field of its own -- adding
/// one would be binding-visible drift across every language binding, since the
/// struct carries no `alef(skip)`. `perform_ocr` instead records data-loss
/// signals into its existing free-form `metadata` map, the same precedent set
/// by the `pre_formatted` and `word_iterator_skipped_count` keys. This function
/// is the other half: it reads those same keys back out here, where the map is
/// still available (before it is drained into `Metadata::additional`), and
/// turns them into warnings on the returned `ExtractedDocument`.
///
/// Extracted as its own pure function so the metadata-to-warning mapping is
/// unit-testable without a live Tesseract API instance.
fn warnings_from_ocr_metadata(
    metadata: &std::collections::HashMap<String, serde_json::Value>,
) -> Vec<crate::types::ProcessingWarning> {
    let mut warnings = Vec::new();

    if let Some(skipped) = metadata
        .get(WORD_ITERATOR_SKIPPED_COUNT_METADATA_KEY)
        .and_then(serde_json::Value::as_u64)
        && skipped > 0
    {
        crate::core::diagnostics::push_warning(
            &mut warnings,
            "tesseract",
            format!(
                "The Tesseract result iterator failed to extract {skipped} word(s) from this image \
                 (null pointer, invalid parameter, or invalid UTF-8); those words are missing from \
                 the OCR output"
            ),
        );
    }

    if metadata
        .get(AUTO_ROTATE_UNAVAILABLE_METADATA_KEY)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        crate::core::diagnostics::push_warning(
            &mut warnings,
            "tesseract",
            "auto_rotate was requested but this build does not include the `auto-rotate` feature; \
             the image was OCR'd without orientation detection or correction",
        );
    }

    if let Some(filtered_lines) = metadata
        .get(DICTIONARY_FILTERED_LINE_COUNT_METADATA_KEY)
        .and_then(serde_json::Value::as_u64)
        && filtered_lines > 0
    {
        crate::core::diagnostics::push_warning(
            &mut warnings,
            "tesseract",
            format!(
                "Tesseract removed {filtered_lines} OCR line(s) because their dictionary-checkable words were mostly \
                 not real words"
            ),
        );
    }

    warnings
}

/// Remove the OCR pipeline's internal scratch metadata keys before the
/// remaining map is copied wholesale into the user-visible
/// `Metadata::additional` (#354).
///
/// `WORD_ITERATOR_SKIPPED_COUNT_METADATA_KEY` and
/// `AUTO_ROTATE_UNAVAILABLE_METADATA_KEY` are plumbing for
/// `warnings_from_ocr_metadata` (call that first -- it reads these keys back
/// out before this function removes them) and have no meaning as document
/// metadata. `pre_formatted` is handled separately by
/// [`extract_pre_formatted_metadata`] because it is promoted to
/// `Metadata::output_format`, not dropped.
///
/// Extracted as its own pure function, mirroring `warnings_from_ocr_metadata`,
/// so the filtering is unit-testable without a live Tesseract API instance.
fn strip_ocr_scratch_metadata_keys(metadata: &mut std::collections::HashMap<String, serde_json::Value>) {
    metadata.remove(WORD_ITERATOR_SKIPPED_COUNT_METADATA_KEY);
    metadata.remove(AUTO_ROTATE_UNAVAILABLE_METADATA_KEY);
    metadata.remove(DICTIONARY_FILTERED_LINE_COUNT_METADATA_KEY);
}

fn compact_cjk_horizontal_spacing(text: &str) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(text.len());
    let mut index = 0;
    while index < chars.len() {
        if !matches!(chars[index], ' ' | '\t') {
            output.push(chars[index]);
            index += 1;
            continue;
        }

        let whitespace_start = index;
        while index < chars.len() && matches!(chars[index], ' ' | '\t') {
            index += 1;
        }
        let joins_cjk = output.chars().next_back().is_some_and(is_compact_cjk_char)
            && chars.get(index).copied().is_some_and(is_compact_cjk_char);
        if !joins_cjk {
            output.extend(chars[whitespace_start..index].iter());
        }
    }
    output
}

fn is_compact_cjk_char(character: char) -> bool {
    matches!(
        character as u32,
        0x2E80..=0x30FF | 0x31F0..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF
            | 0x20000..=0x2FA1F
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The probe's historical request: English only.
    fn eng() -> Vec<String> {
        vec!["eng".to_string()]
    }

    /// GH#1891. A `tessdata_path` holding only the caller's own language must satisfy the probe,
    /// because it satisfies a real job: `tessdata_search_dirs` accepts a candidate directory only
    /// when `all_languages_exist` holds for the languages it is given, so the hardcoded `eng` the
    /// probe used to pass rejected the override and fell through to the rest of the chain.
    ///
    /// Asserted on the resolution step rather than through `query_available_languages`, so it needs
    /// no Tesseract engine and no real model bytes -- `all_languages_exist` is a file-existence
    /// check, which is exactly the step that was being handed the wrong languages. Nothing reads or
    /// writes the process environment, so this needs no `#[serial]`.
    #[test]
    fn the_probe_resolves_a_tessdata_path_holding_only_the_requested_language() {
        let root = tempfile::tempdir().expect("must create a temp dir for the fixture");
        let override_dir = root.path().join("tess-deu");
        let prefix_dir = root.path().join("prefix");
        std::fs::create_dir_all(&override_dir).expect("must create the override directory");
        std::fs::create_dir_all(&prefix_dir).expect("must create the prefix directory");
        std::fs::write(override_dir.join("deu.traineddata"), b"not-a-real-model").expect("must write deu.traineddata");
        std::fs::write(prefix_dir.join("eng.traineddata"), b"not-a-real-model").expect("must write eng.traineddata");

        let tessdata_env = TessdataEnv {
            tessdata_prefix: Some(prefix_dir.to_string_lossy().into_owned()),
            cache_dir: None,
        };
        let requested = vec!["deu".to_string()];

        assert_eq!(
            TesseractBackend::probe_languages(&requested, Some(&override_dir), &tessdata_env),
            requested,
            "the probe must ask for the caller's own languages when a searched directory holds them"
        );
        assert_eq!(
            crate::ocr::processor::validation::existing_tessdata_dir_for(
                &requested,
                Some(&override_dir),
                &tessdata_env
            )
            .as_deref(),
            override_dir.to_str(),
            "and must resolve the configured tessdata_path, exactly as a real job with this config \
             does -- asking for `eng` resolved the TESSDATA_PREFIX directory instead"
        );
    }

    /// GH#1891. A language nowhere on disk must not make the probe resolve a directory it would
    /// first have to create and download into: `supports_language_for` is a question about this
    /// machine, and answering it must not reach the network.
    #[test]
    fn the_probe_falls_back_to_english_for_a_language_no_searched_directory_holds() {
        let root = tempfile::tempdir().expect("must create a temp dir for the fixture");
        let prefix_dir = root.path().join("prefix");
        std::fs::create_dir_all(&prefix_dir).expect("must create the prefix directory");
        std::fs::write(prefix_dir.join("eng.traineddata"), b"not-a-real-model").expect("must write eng.traineddata");

        let tessdata_env = TessdataEnv {
            tessdata_prefix: Some(prefix_dir.to_string_lossy().into_owned()),
            cache_dir: None,
        };

        assert_eq!(
            TesseractBackend::probe_languages(&["zzz_absent".to_string()], None, &tessdata_env),
            eng(),
            "an absent language must fall back to English rather than being resolved by download"
        );
        assert_eq!(
            TesseractBackend::probe_languages(&[], None, &tessdata_env),
            eng(),
            "and an empty request must too: `all_languages_exist` rejects every directory for an \
             empty language list, which would send resolution down the same download path"
        );
    }

    /// GH#1891, the memo half. The key must cover every input tessdata resolution reads, not the
    /// `tessdata_path` override alone (GH#1857's fix): `TESSDATA_PREFIX` and `XBERG_CACHE_DIR` both
    /// feed `tessdata_search_dirs`, and the probe languages decide which candidate is accepted.
    /// Keyed on the key builder rather than on two live probes so it needs no Tesseract engine --
    /// with no engine both probes return the same hardcoded fallback list, which cannot distinguish
    /// a correct key from a colliding one.
    #[test]
    fn the_language_memo_key_covers_every_input_resolution_reads() {
        let override_path = std::path::Path::new("/opt/tess-deu");
        let env_a = TessdataEnv {
            tessdata_prefix: Some("/a/tessdata".to_string()),
            cache_dir: None,
        };
        let env_b = TessdataEnv {
            tessdata_prefix: Some("/b/tessdata".to_string()),
            cache_dir: None,
        };
        let env_c = TessdataEnv {
            tessdata_prefix: Some("/a/tessdata".to_string()),
            cache_dir: Some(std::path::PathBuf::from("/cache")),
        };

        let baseline = TesseractBackend::languages_memo_key(&eng(), Some(override_path), &env_a);

        assert_eq!(
            baseline,
            TesseractBackend::languages_memo_key(&eng(), Some(override_path), &env_a),
            "the same inputs must produce the same key, or nothing is ever memoized"
        );
        assert_ne!(
            baseline,
            TesseractBackend::languages_memo_key(&eng(), None, &env_a),
            "the tessdata_path override must take part in the key (GH#1857)"
        );
        assert_ne!(
            baseline,
            TesseractBackend::languages_memo_key(&eng(), Some(override_path), &env_b),
            "TESSDATA_PREFIX must take part in the key: it is a search directory of its own"
        );
        assert_ne!(
            baseline,
            TesseractBackend::languages_memo_key(&eng(), Some(override_path), &env_c),
            "XBERG_CACHE_DIR must take part in the key: it contributes two search directories"
        );
        assert_ne!(
            baseline,
            TesseractBackend::languages_memo_key(&["deu".to_string()], Some(override_path), &env_a),
            "and the probe languages must too: they decide which candidate directory is accepted"
        );
    }

    // Needs real, loadable eng.traineddata with no network fetch to distinguish a
    // resolved-directory probe from a silent fallback; `bundle-tessdata-eng` is the
    // only feature that gives this test that without hitting the network at run time.
    // The fix itself (resolving the datapath the same way the real job does) does not
    // need this feature; it is only how this test gets deterministic fixture bytes.
    #[cfg(feature = "bundle-tessdata-eng")]
    #[test]
    fn query_available_languages_resolves_the_same_tessdata_directory_the_real_job_uses() {
        let temp_dir = tempfile::tempdir().expect("must create a temp dir for the fixture");
        let tessdata_dir = temp_dir.path().join("tessdata");
        std::fs::create_dir_all(&tessdata_dir).expect("must create the fixture tessdata dir");

        let eng_bytes = xberg_tesseract::bundled_eng_traineddata()
            .expect("this build must carry bundled eng.traineddata for the test to be meaningful");
        std::fs::write(tessdata_dir.join("eng.traineddata"), eng_bytes).expect("must write eng.traineddata");
        // A file only a real directory scan of THIS fixture would report; the hardcoded
        // fallback list can never contain it, so its presence proves the probe actually
        // looked at the tessdata directory the real OCR job resolves.
        std::fs::write(tessdata_dir.join("zzz_probe_marker.traineddata"), b"not-a-real-model")
            .expect("must write the marker file");

        // `TESSDATA_PREFIX` ranks above `XBERG_CACHE_DIR`, and CI's unit-test runner
        // (scripts/lib/tessdata.sh::setup_tessdata) exports it before `cargo test` starts.
        // Passing no prefix makes the probe reach this fixture without changing the
        // process environment that parallel tests read. ~keep
        let tessdata_env = TessdataEnv {
            tessdata_prefix: None,
            cache_dir: Some(temp_dir.path().to_path_buf()),
        };
        let languages = TesseractBackend::query_available_languages(&eng(), None, &tessdata_env)
            .expect("the probe must resolve the fixture tessdata directory and read its languages");

        assert!(
            languages.iter().any(|lang| lang == "zzz_probe_marker"),
            "the language-availability probe must scan the same tessdata directory the real \
             OCR job resolves (XBERG_CACHE_DIR/tessdata here), not silently fall back to the \
             hardcoded language list; got: {languages:?}"
        );
    }

    /// Real `eng.traineddata` bytes without the `bundle-tessdata-eng` feature.
    ///
    /// `xberg-tesseract`'s build script downloads and caches real `eng.traineddata` into
    /// its own `OUT_DIR` unconditionally (`crates/xberg-tesseract/build.rs`), regardless of
    /// `bundle-tessdata-eng`; that feature only controls whether the bytes are ALSO embedded
    /// into the binary via `include_bytes!`. Building `xberg` with `pdf`/`ocr` builds
    /// `xberg-tesseract` first, so its `OUT_DIR` is a sibling of this crate's own `OUT_DIR`
    /// under `target/<profile>/build/` by the time this test runs. Returns `None`, rather
    /// than panicking, when that layout assumption does not hold, so the test can skip
    /// cleanly instead of failing for an environment reason unrelated to the fix.
    fn real_eng_traineddata_bytes_from_sibling_build_dir() -> Option<Vec<u8>> {
        let this_out_dir = std::path::PathBuf::from(env!("OUT_DIR"));
        let build_dir = this_out_dir.parent()?.parent()?;
        let mut entries: Vec<_> = std::fs::read_dir(build_dir).ok()?.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let name = entry.file_name();
            if !name.to_string_lossy().starts_with("xberg-tesseract-") {
                continue;
            }
            let candidate = entry.path().join("out").join("eng.traineddata");
            if let Ok(bytes) = std::fs::read(&candidate) {
                return Some(bytes);
            }
        }
        None
    }

    // The regression test above only runs under `bundle-tessdata-eng`, which is never
    // enabled outside wasm32 builds (`crates/xberg-tesseract/Cargo.toml`), so it protects
    // nothing in the feature set that actually ships (`pdf,ocr`, no bundle gate). This is
    // the same assertion against the same realistic `XBERG_CACHE_DIR/tessdata` fixture, but
    // sourcing real `eng.traineddata` bytes the way described above instead of through the
    // bundled feature, so it runs and protects the shipping path. See GH#1671.
    #[test]
    fn query_available_languages_resolves_the_same_tessdata_directory_the_real_job_uses_under_pdf_ocr() {
        let Some(eng_bytes) = real_eng_traineddata_bytes_from_sibling_build_dir() else {
            eprintln!(
                "skipping: no real eng.traineddata found in a sibling xberg-tesseract build \
                 OUT_DIR; this environment did not build xberg-tesseract the way this test expects"
            );
            return;
        };

        let temp_dir = tempfile::tempdir().expect("must create a temp dir for the fixture");
        let tessdata_dir = temp_dir.path().join("tessdata");
        std::fs::create_dir_all(&tessdata_dir).expect("must create the fixture tessdata dir");

        std::fs::write(tessdata_dir.join("eng.traineddata"), &eng_bytes).expect("must write eng.traineddata");
        // A file only a real directory scan of THIS fixture would report; the hardcoded
        // fallback list can never contain it, so its presence proves the probe actually
        // looked at the tessdata directory the real OCR job resolves.
        std::fs::write(tessdata_dir.join("zzz_probe_marker.traineddata"), b"not-a-real-model")
            .expect("must write the marker file");

        // See the sibling `bundle-tessdata-eng` test above for why no prefix is passed. ~keep
        let tessdata_env = TessdataEnv {
            tessdata_prefix: None,
            cache_dir: Some(temp_dir.path().to_path_buf()),
        };
        let languages = TesseractBackend::query_available_languages(&eng(), None, &tessdata_env)
            .expect("the probe must resolve the fixture tessdata directory and read its languages");

        assert!(
            languages.iter().any(|lang| lang == "zzz_probe_marker"),
            "the language-availability probe must scan the same tessdata directory the real \
             OCR job resolves (XBERG_CACHE_DIR/tessdata here), not silently fall back to the \
             hardcoded language list; got: {languages:?}"
        );
    }

    /// GH#1857: the probe must search `OcrConfig.tessdata_path` first, exactly as a real job does
    /// (`processor/execution.rs`'s `config.tessdata_path.as_deref()`). Both candidate directories
    /// hold loadable `eng.traineddata`, so the resolver genuinely chooses between them. Nothing
    /// reads or writes the process environment: the `TESSDATA_PREFIX` value is an argument on
    /// `TessdataEnv` (GH#1846), so this needs no `#[serial]`.
    #[test]
    fn query_available_languages_prefers_the_configured_tessdata_path_over_tessdata_prefix() {
        let Some(eng_bytes) = real_eng_traineddata_bytes_from_sibling_build_dir() else {
            eprintln!(
                "skipping: no real eng.traineddata in a sibling xberg-tesseract build OUT_DIR; \
                 this environment did not build xberg-tesseract the way this test expects"
            );
            return;
        };

        let root = tempfile::tempdir().expect("must create a temp dir for the fixture");
        let override_dir = root.path().join("override");
        let prefix_dir = root.path().join("prefix");
        for (dir, marker) in [
            (&override_dir, "zzz_override_marker"),
            (&prefix_dir, "zzz_prefix_marker"),
        ] {
            std::fs::create_dir_all(dir).expect("must create the fixture directory");
            std::fs::write(dir.join("eng.traineddata"), &eng_bytes).expect("must write eng.traineddata");
            std::fs::write(dir.join(format!("{marker}.traineddata")), b"not-a-real-model")
                .expect("must write the marker file");
        }

        let tessdata_env = TessdataEnv {
            tessdata_prefix: Some(prefix_dir.to_string_lossy().into_owned()),
            cache_dir: None,
        };
        let languages = TesseractBackend::query_available_languages(&eng(), Some(&override_dir), &tessdata_env)
            .expect("the probe must resolve the configured tessdata directory");

        assert!(
            languages.iter().any(|language| language == "zzz_override_marker"),
            "the probe must enumerate the configured tessdata_path; got: {languages:?}"
        );
        assert!(
            !languages.iter().any(|language| language == "zzz_prefix_marker"),
            "the configured tessdata_path outranks TESSDATA_PREFIX, as it does for a real job; \
             got: {languages:?}"
        );
    }

    /// The language memo must be keyed by the override. A single cell would serve whichever
    /// caller asked first to every later one, which is the second half of GH#1857 and is
    /// invisible to the test above (that one never populates the memo twice).
    #[test]
    fn the_language_memo_is_keyed_by_the_configured_tessdata_path() {
        let Some(eng_bytes) = real_eng_traineddata_bytes_from_sibling_build_dir() else {
            eprintln!("skipping: no real eng.traineddata in a sibling xberg-tesseract build OUT_DIR");
            return;
        };

        let root = tempfile::tempdir().expect("must create a temp dir for the fixture");
        let override_dir = root.path().join("override");
        let prefix_dir = root.path().join("prefix");
        for (dir, marker) in [
            (&override_dir, "zzz_override_marker"),
            (&prefix_dir, "zzz_prefix_marker"),
        ] {
            std::fs::create_dir_all(dir).expect("must create the fixture directory");
            std::fs::write(dir.join("eng.traineddata"), &eng_bytes).expect("must write eng.traineddata");
            std::fs::write(dir.join(format!("{marker}.traineddata")), b"not-a-real-model")
                .expect("must write the marker file");
        }

        let tessdata_env = TessdataEnv {
            tessdata_prefix: Some(prefix_dir.to_string_lossy().into_owned()),
            cache_dir: None,
        };
        let backend = TesseractBackend::new();

        let with_override = backend.get_cached_languages(&eng(), Some(&override_dir), &tessdata_env);
        let without_override = backend.get_cached_languages(&eng(), None, &tessdata_env);

        assert!(
            with_override.iter().any(|language| language == "zzz_override_marker"),
            "the override key must enumerate the override directory; got: {with_override:?}"
        );
        assert!(
            without_override.iter().any(|language| language == "zzz_prefix_marker"),
            "the no-override key must enumerate TESSDATA_PREFIX; got: {without_override:?}"
        );
        assert!(
            !without_override
                .iter()
                .any(|language| language == "zzz_override_marker"),
            "a single memo cell would have replayed the override's list here; got: {without_override:?}"
        );
    }

    /// The production call sites read `TESSDATA_PREFIX` and `XBERG_CACHE_DIR` from the
    /// process environment. The variables are set only on a child run of this test binary,
    /// so no thread in this process sees the environment change.
    #[test]
    fn production_call_sites_read_tessdata_prefix_and_xberg_cache_dir() {
        let Some(eng_bytes) = real_eng_traineddata_bytes_from_sibling_build_dir() else {
            eprintln!(
                "skipping: no real eng.traineddata found in a sibling xberg-tesseract build \
                 OUT_DIR; this environment did not build xberg-tesseract the way this test expects"
            );
            return;
        };

        let root = tempfile::tempdir().expect("must create a temp dir for the fixture");
        let prefix_dir = root.path().join("prefix");
        let cache_tessdata_dir = root.path().join("cache").join("tessdata");
        std::fs::create_dir_all(&prefix_dir).expect("must create the prefix fixture dir");
        std::fs::create_dir_all(&cache_tessdata_dir).expect("must create the cache fixture dir");
        std::fs::write(prefix_dir.join("zzz_prefix_marker.traineddata"), b"not-a-real-model")
            .expect("must write the prefix marker");
        std::fs::write(cache_tessdata_dir.join("eng.traineddata"), &eng_bytes).expect("must write eng.traineddata");
        std::fs::write(
            cache_tessdata_dir.join("zzz_cache_marker.traineddata"),
            b"not-a-real-model",
        )
        .expect("must write the cache marker");

        let child = "ocr::tesseract_backend::tests::production_call_sites_read_tessdata_env_child";
        let status = std::process::Command::new(std::env::current_exe().expect("must find the test binary"))
            .arg("--exact")
            .arg(child)
            .arg("--ignored")
            .arg("--nocapture")
            .env("XBERG_TESSDATA_ENV_TEST_ROOT", root.path())
            .env("TESSDATA_PREFIX", &prefix_dir)
            .env("XBERG_CACHE_DIR", root.path().join("cache"))
            .status()
            .expect("must launch the isolated tessdata environment test");
        assert!(status.success(), "isolated test {child} failed with {status}");
    }

    #[test]
    #[ignore = "run in an isolated subprocess by production_call_sites_read_tessdata_prefix_and_xberg_cache_dir"]
    fn production_call_sites_read_tessdata_env_child() {
        let root =
            std::path::PathBuf::from(std::env::var_os("XBERG_TESSDATA_ENV_TEST_ROOT").expect("test fixture root"));
        let prefix_dir = root.join("prefix").to_string_lossy().into_owned();
        let cache_dir = root.join("cache");
        let cache_tessdata_dir = cache_dir.join("tessdata").to_string_lossy().into_owned();

        let resolve = |language: &str| {
            crate::ocr::processor::validation::resolve_tessdata_path(&[language.to_string()], None)
                .expect("the job resolver must find the fixture language")
        };
        assert_eq!(
            resolve("zzz_prefix_marker"),
            prefix_dir,
            "job resolver, TESSDATA_PREFIX"
        );
        assert_eq!(
            resolve("zzz_cache_marker"),
            cache_tessdata_dir,
            "job resolver, XBERG_CACHE_DIR"
        );
        assert_eq!(
            crate::cache_dir::resolve_cache_base(),
            cache_dir,
            "cache base, XBERG_CACHE_DIR"
        );

        let backend = TesseractBackend::new();
        let languages = backend.supported_languages();
        assert!(
            languages.iter().any(|lang| lang == "zzz_cache_marker"),
            "the language probe must scan XBERG_CACHE_DIR/tessdata; got: {languages:?}"
        );

        for (language, dir) in [
            ("zzz_prefix_marker", &prefix_dir),
            ("zzz_cache_marker", &cache_tessdata_dir),
        ] {
            let config = OcrConfig {
                language: vec![language.to_string()],
                ..OcrConfig::default()
            };
            let check = backend.probe(&config);
            assert!(
                check.message.ends_with(&format!(" at {dir}")),
                "the doctor probe must find {language} in {dir}; got: {check:?}"
            );
        }
    }

    #[test]
    fn vertical_cjk_spacing_removes_only_inter_character_horizontal_space() {
        assert_eq!(
            compact_cjk_horizontal_spacing("元 来 日 本 語 は 漢文 に 倣い 、 API 文書 。\n次 行"),
            "元来日本語は漢文に倣い、 API 文書。\n次行"
        );
    }

    #[test]
    fn vertical_cjk_spacing_preserves_latin_and_paragraph_whitespace() {
        assert_eq!(
            compact_cjk_horizontal_spacing("API 仕様\tversion 2\n\n次段落"),
            "API 仕様\tversion 2\n\n次段落"
        );
    }

    /// Both limiters on the recognition path must take the same number. Each
    /// used to read its own copy of a compile-time four, so raising one alone
    /// left recognition exactly as wide as before.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn recognition_limiters_share_one_capacity() {
        let capacity = crate::ocr::processor::tesseract_api_capacity();
        let cache_dir = tempfile::TempDir::new().expect("failed to create a cache directory");
        let processor = crate::ocr::processor::OcrProcessor::new(Some(cache_dir.path().to_path_buf()))
            .expect("failed to create the OCR processor");

        assert_eq!(
            processor.api_pool_capacity(),
            capacity,
            "the Tesseract handle pool must take the shared recognition capacity"
        );
        assert_eq!(
            TesseractBackend::new().concurrency().available_permits(),
            capacity,
            "the admission semaphore must take the shared recognition capacity"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_tesseract_backend_limits_concurrent_calls() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let backend = Arc::new(TesseractBackend::new());
        let capacity = crate::ocr::processor::tesseract_api_capacity();
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();
        // Oversubscribe the limit rather than a fixed count, which only reached
        // the limit while it was the compile-time four.
        for _ in 0..capacity * 3 {
            let backend = Arc::clone(&backend);
            let active = Arc::clone(&active);
            let peak = Arc::clone(&peak);
            tasks.push(tokio::spawn(async move {
                let _permit = backend.concurrency().acquire().await.unwrap();
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(current, Ordering::SeqCst);
                tokio::task::yield_now().await;
                active.fetch_sub(1, Ordering::SeqCst);
            }));
        }
        for task in tasks {
            task.await.unwrap();
        }
        assert_eq!(peak.load(Ordering::SeqCst), capacity);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_tesseract_permit_outlives_cancelled_async_caller() {
        let backend = Arc::new(TesseractBackend::new());
        let reserved = Arc::clone(backend.concurrency())
            .acquire_many_owned((crate::ocr::processor::tesseract_api_capacity() - 1) as u32)
            .await
            .unwrap();
        let rendezvous = Arc::new(std::sync::Barrier::new(2));
        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn({
            let semaphore = Arc::clone(backend.concurrency());
            let rendezvous = Arc::clone(&rendezvous);
            async move {
                let permit = semaphore.acquire_owned().await.unwrap();
                tokio::task::spawn_blocking(move || {
                    {
                        let _permit = permit;
                        rendezvous.wait();
                        rendezvous.wait();
                    }
                    let _ = done_tx.send(());
                })
                .await
                .unwrap();
            }
        });

        rendezvous.wait();
        task.abort();
        assert!(Arc::clone(backend.concurrency()).try_acquire_owned().is_err());
        rendezvous.wait();
        done_rx.await.unwrap();
        drop(reserved);

        assert_eq!(
            backend.concurrency().available_permits(),
            crate::ocr::processor::tesseract_api_capacity()
        );
    }

    #[test]
    fn test_tesseract_backend_creation() {
        let backend = TesseractBackend::new();
        assert!(!backend.processor_is_initialized());
    }

    #[test]
    fn test_tesseract_backend_plugin_interface() {
        let backend = TesseractBackend::new();
        assert_eq!(backend.name(), "tesseract");
        assert!(!backend.version().is_empty());
        assert!(backend.initialize().is_ok());
    }

    #[test]
    fn test_tesseract_backend_type() {
        let backend = TesseractBackend::new();
        assert_eq!(backend.backend_type(), OcrBackendType::Tesseract);
    }

    #[test]
    fn test_tesseract_backend_supports_language() {
        let backend = TesseractBackend::new();
        assert!(backend.supports_language("eng"));
        assert!(!backend.supports_language("xyz"));
        assert!(!backend.supports_language("invalid"));
    }

    #[test]
    fn test_tesseract_backend_supports_table_detection() {
        let backend = TesseractBackend::new();
        assert!(backend.supports_table_detection());
    }

    #[test]
    fn test_tesseract_backend_supported_languages() {
        let backend = TesseractBackend::new();
        let languages = backend.supported_languages();
        assert!(languages.contains(&"eng".to_string()));
        assert!(!languages.is_empty());
    }

    #[test]
    fn test_fallback_languages_include_vertical_japanese() {
        assert!(
            TesseractBackend::fallback_languages()
                .iter()
                .any(|language| language == "jpn_vert")
        );
    }

    /// Issue #181: OCR-produced tables must carry a deterministic `table_id`,
    /// `columns`, and `bounding_box` — not `..Default::default()` blanks.
    #[test]
    fn convert_ocr_table_assigns_sequential_ids_columns_and_bounding_box() {
        let first = crate::types::OcrTable {
            cells: vec![
                vec!["Name".to_string(), "Age".to_string()],
                vec!["Alice".to_string(), "30".to_string()],
            ],
            markdown: "| Name | Age |\n|---|---|\n| Alice | 30 |".to_string(),
            page_number: 1,
            bounding_box: Some(crate::types::OcrTableBoundingBox {
                left: 10,
                top: 20,
                right: 110,
                bottom: 220,
            }),
        };
        let second = crate::types::OcrTable {
            cells: vec![vec!["X".to_string()]],
            markdown: "| X |".to_string(),
            page_number: 2,
            bounding_box: None,
        };

        let converted_first = convert_ocr_table(0, first);
        let converted_second = convert_ocr_table(1, second);

        assert_eq!(converted_first.table_id.as_deref(), Some("table-1"));
        assert_eq!(
            converted_first.columns,
            Some(vec!["Name".to_string(), "Age".to_string()])
        );
        let bbox = converted_first.bounding_box.expect("bounding box must be populated");
        assert_eq!(bbox.x0, 10.0);
        assert_eq!(bbox.y0, 20.0);
        assert_eq!(bbox.x1, 110.0);
        assert_eq!(bbox.y1, 220.0);

        assert_eq!(converted_second.table_id.as_deref(), Some("table-2"));
        assert_eq!(converted_second.columns, Some(vec!["X".to_string()]));
        assert!(converted_second.bounding_box.is_none());
    }

    #[test]
    fn test_config_to_tesseract_with_none() {
        let backend = TesseractBackend::new();
        let ocr_config = OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["deu".to_string()],
            ..Default::default()
        };

        let tess_config = backend.config_to_tesseract(&ocr_config);
        assert_eq!(tess_config.language, "deu");
        assert_eq!(tess_config.psm, InternalTesseractConfig::default().psm);
    }

    #[test]
    fn test_config_to_tesseract_with_some() {
        let backend = TesseractBackend::new();
        let custom_tess_config = crate::types::TesseractConfig {
            language: vec!["fra".to_string()],
            psm: Some(6),
            enable_table_detection: true,
            ..Default::default()
        };

        let ocr_config = OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            tesseract_config: Some(custom_tess_config),
            ..Default::default()
        };

        let tess_config = backend.config_to_tesseract(&ocr_config);
        assert_eq!(tess_config.language, "fra");
        assert_eq!(tess_config.psm, 6);
        assert!(tess_config.enable_table_detection);
    }

    /// #1572: supplying ANY `TesseractConfig` used to discard `OcrConfig.language` entirely,
    /// because the `Some` arm read the public struct's own `language` field -- which defaults to
    /// `["eng"]`, so a German document silently OCR'd in English. `["eng"]` is not empty, so the
    /// existing empty-string guard never caught it. The neighbouring test above covers the
    /// opposite precedence (an explicitly-set `tesseract_config.language` still wins); this one
    /// pins the reported case, where only `OcrConfig.language` was set. ~keep
    #[test]
    fn config_to_tesseract_keeps_ocr_config_language_when_tesseract_config_is_default() {
        let backend = TesseractBackend::new();
        let ocr_config = OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["deu".to_string()],
            tesseract_config: Some(crate::types::TesseractConfig::default()),
            ..Default::default()
        };

        let tess_config = backend.config_to_tesseract(&ocr_config);
        assert_eq!(
            tess_config.language, "deu",
            "OcrConfig.language must survive a default TesseractConfig"
        );
    }

    /// #1572, multi-language form: the join must use the configured list, not fall back to "eng".
    #[test]
    fn config_to_tesseract_joins_multiple_ocr_config_languages_with_a_default_tesseract_config() {
        let backend = TesseractBackend::new();
        let ocr_config = OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["deu".to_string(), "fra".to_string()],
            tesseract_config: Some(crate::types::TesseractConfig::default()),
            ..Default::default()
        };

        assert_eq!(backend.config_to_tesseract(&ocr_config).language, "deu+fra");
    }

    /// The `source_dpi` hint the PDF OCR route stamps per page must survive the crossing into
    /// the internal config — that is the whole delivery path, and it needs no change to the
    /// `OcrBackend` trait because `backend_options` is already a documented per-call channel.
    ///
    /// Fails on unfixed code: `InternalTesseractConfig` has no `source_dpi` field, so this does
    /// not compile. There is no expression of the value to assert against at all.
    #[test]
    fn should_read_source_dpi_hint_from_backend_options() {
        let backend = TesseractBackend::new();
        let ocr_config = OcrConfig {
            backend: "tesseract".to_string(),
            backend_options: Some(serde_json::json!({ "source_dpi": 150.0 })),
            ..Default::default()
        };

        assert_eq!(backend.config_to_tesseract(&ocr_config).source_dpi, Some(150.0));
    }

    #[test]
    fn should_read_result_cache_override_from_backend_options() {
        let backend = TesseractBackend::new();
        let ocr_config = OcrConfig {
            backend: "tesseract".to_string(),
            backend_options: Some(serde_json::json!({ "use_cache": false })),
            ..Default::default()
        };

        assert!(!backend.config_to_tesseract(&ocr_config).use_cache);
        assert!(ocr_config.tesseract_config.is_none());
    }

    #[test]
    fn should_ignore_malformed_result_cache_override() {
        let backend = TesseractBackend::new();
        let ocr_config = OcrConfig {
            backend: "tesseract".to_string(),
            backend_options: Some(serde_json::json!({ "use_cache": "false" })),
            ..Default::default()
        };

        assert_eq!(
            backend.config_to_tesseract(&ocr_config).use_cache,
            InternalTesseractConfig::default().use_cache
        );
    }

    /// Callers that do not know their image's resolution — standalone image OCR, plugin callers
    /// handed arbitrary bytes — must keep reaching the historical 72-DPI assumption rather than
    /// being given a fabricated value.
    ///
    /// Fails on unfixed code by not compiling; behaviourally this is the pre-existing contract,
    /// pinned so the new hint cannot silently displace it.
    #[test]
    fn should_leave_source_dpi_unknown_when_no_hint_is_supplied() {
        let backend = TesseractBackend::new();
        let ocr_config = OcrConfig {
            backend: "tesseract".to_string(),
            ..Default::default()
        };

        assert_eq!(backend.config_to_tesseract(&ocr_config).source_dpi, None);
    }

    /// A malformed or impossible hint is "unknown", not an error and not a poisoned scale
    /// factor: a zero or negative DPI would make the resize factor infinite or negative, and a
    /// string would silently become `None` anyway.
    ///
    /// Fails on unfixed code by not compiling.
    #[test]
    fn should_reject_non_positive_or_malformed_source_dpi_hints() {
        let backend = TesseractBackend::new();
        for hint in [
            serde_json::json!({ "source_dpi": 0.0 }),
            serde_json::json!({ "source_dpi": -150.0 }),
            serde_json::json!({ "source_dpi": "150" }),
        ] {
            let ocr_config = OcrConfig {
                backend: "tesseract".to_string(),
                backend_options: Some(hint.clone()),
                ..Default::default()
            };

            assert_eq!(
                backend.config_to_tesseract(&ocr_config).source_dpi,
                None,
                "hint {hint} must be treated as unknown"
            );
        }
    }

    /// GH#1894: the PDF OCR route's own scan-detection signal must reach the internal config so
    /// `prepare_ocr_image` can bypass the pixel-brightness heuristic for a page it already knows
    /// is a whole-page scan.
    ///
    /// Negative control (delete the `backend_options` line, leaving `..Default::default()`):
    /// fails with `assertion failed: backend.config_to_tesseract(&ocr_config).known_full_page_scan`
    /// — proving this test actually reads the hint rather than the field's own default.
    #[test]
    fn should_read_known_full_page_scan_hint_from_backend_options() {
        let backend = TesseractBackend::new();
        let ocr_config = OcrConfig {
            backend: "tesseract".to_string(),
            backend_options: Some(serde_json::json!({ "known_full_page_scan": true })),
            ..Default::default()
        };

        assert!(backend.config_to_tesseract(&ocr_config).known_full_page_scan);
    }

    /// Callers that never stamp the hint — standalone image OCR, plugin callers, direct API
    /// callers — must keep the pre-GH#1894 behaviour of judging every image by pixel brightness.
    ///
    /// Fails on unfixed code by not compiling (the field does not exist).
    #[test]
    fn should_default_known_full_page_scan_to_false_when_no_hint_is_supplied() {
        let backend = TesseractBackend::new();
        let ocr_config = OcrConfig {
            backend: "tesseract".to_string(),
            ..Default::default()
        };

        assert!(!backend.config_to_tesseract(&ocr_config).known_full_page_scan);
    }

    #[test]
    fn test_config_to_tesseract_defaults_empty_language_to_eng() {
        let backend = TesseractBackend::new();

        let ocr_config = OcrConfig {
            backend: "tesseract".to_string(),
            language: vec![],
            ..Default::default()
        };
        assert_eq!(backend.config_to_tesseract(&ocr_config).language, "eng");

        let ocr_config_with_tess = OcrConfig {
            backend: "tesseract".to_string(),
            language: vec![],
            tesseract_config: Some(crate::types::TesseractConfig {
                language: vec![],
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(backend.config_to_tesseract(&ocr_config_with_tess).language, "eng");
    }

    #[test]
    fn test_tesseract_backend_default() {
        let backend = TesseractBackend::default();
        assert_eq!(backend.name(), "tesseract");
    }

    #[test]
    fn test_config_conversion_with_new_fields() {
        let backend = TesseractBackend::new();

        let preprocessing = crate::types::ImagePreprocessingConfig {
            target_dpi: 600,
            auto_rotate: false,
            deskew: true,
            denoise: true,
            contrast_enhance: true,
            binarization_method: "adaptive".to_string(),
            invert_colors: false,
            normalize_shaded_rows: false,
        };

        let custom_tess_config = crate::types::TesseractConfig {
            language: vec!["eng".to_string()],
            psm: Some(6),
            output_format: "markdown".to_string(),
            oem: 1,
            min_confidence: 80.0,
            preprocessing: Some(preprocessing.clone()),
            tessedit_char_blacklist: "!@#$".to_string(),
            ..Default::default()
        };

        let ocr_config = OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            tesseract_config: Some(custom_tess_config),
            ..Default::default()
        };

        let tess_config = backend.config_to_tesseract(&ocr_config);

        assert_eq!(tess_config.oem, 1);
        assert_eq!(tess_config.min_confidence, 80.0);
        assert_eq!(tess_config.tessedit_char_blacklist, "!@#$");

        assert!(tess_config.preprocessing.is_some());
        let preproc = tess_config.preprocessing.unwrap();
        assert_eq!(preproc.target_dpi, 600);
        assert!(!preproc.auto_rotate);
        assert!(preproc.deskew);
        assert!(preproc.denoise);
        assert!(preproc.contrast_enhance);
        assert_eq!(preproc.binarization_method, "adaptive");
        assert!(!preproc.invert_colors);
    }

    #[test]
    fn test_convert_config_type_conversions() {
        let public_config = crate::types::TesseractConfig {
            language: vec!["eng".to_string()],
            psm: Some(6),
            oem: 3,
            table_column_threshold: 100,
            ..Default::default()
        };

        let internal_config = InternalTesseractConfig::from(&public_config);

        assert_eq!(internal_config.psm, 6u8);
        assert_eq!(internal_config.oem, 3u8);
        assert_eq!(internal_config.table_column_threshold, 100u32);
    }

    /// #309: a positive `word_iterator_skipped_count` must surface as a
    /// `tesseract`-sourced warning naming the exact word count lost, not a
    /// generic "something was dropped" message.
    #[test]
    fn warnings_from_ocr_metadata_flags_dropped_words_with_exact_count() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            WORD_ITERATOR_SKIPPED_COUNT_METADATA_KEY.to_string(),
            serde_json::Value::Number(2.into()),
        );

        let warnings = warnings_from_ocr_metadata(&metadata);

        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].source, "tesseract");
        assert!(
            warnings[0].message.contains("2 word(s)"),
            "message must name the exact skipped count: {}",
            warnings[0].message
        );
    }

    #[test]
    fn warnings_from_ocr_metadata_flags_dictionary_filtered_lines_with_exact_count() {
        let metadata = std::collections::HashMap::from([(
            "dictionary_filtered_line_count".to_string(),
            serde_json::Value::Number(2.into()),
        )]);

        let warnings = warnings_from_ocr_metadata(&metadata);

        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].source, "tesseract");
        assert_eq!(
            warnings[0].message,
            "Tesseract removed 2 OCR line(s) because their dictionary-checkable words were mostly not real words"
        );
    }

    /// A `word_iterator_skipped_count` of exactly zero must not produce a
    /// warning -- the metadata-insertion side already omits the key in that
    /// case, but this guards the consumption side independently (#309).
    #[test]
    fn warnings_from_ocr_metadata_ignores_zero_skipped_count() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            WORD_ITERATOR_SKIPPED_COUNT_METADATA_KEY.to_string(),
            serde_json::Value::Number(0.into()),
        );

        assert!(warnings_from_ocr_metadata(&metadata).is_empty());
    }

    /// #309: an `auto_rotate_unavailable` metadata flag must surface as a
    /// `tesseract`-sourced warning naming the `auto-rotate` feature, so a user
    /// who asked for rotation correction learns it never ran.
    #[test]
    fn warnings_from_ocr_metadata_flags_auto_rotate_unavailable() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            AUTO_ROTATE_UNAVAILABLE_METADATA_KEY.to_string(),
            serde_json::Value::Bool(true),
        );

        let warnings = warnings_from_ocr_metadata(&metadata);

        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].source, "tesseract");
        assert!(
            warnings[0].message.contains("auto-rotate"),
            "message must name the missing feature: {}",
            warnings[0].message
        );
    }

    /// A clean extraction (neither metadata key present) must produce no
    /// warnings at all -- the whole point of the deduped, source-scoped
    /// warning convention is silence on the happy path (#309).
    #[test]
    fn warnings_from_ocr_metadata_is_silent_on_clean_extraction() {
        let metadata = std::collections::HashMap::new();
        assert!(warnings_from_ocr_metadata(&metadata).is_empty());
    }

    /// Both signals can fire together (a garbled page that also requested
    /// rotation on a build without the feature); both warnings must be kept,
    /// not just the first one found (#309).
    #[test]
    fn warnings_from_ocr_metadata_keeps_both_warnings_when_both_signals_fire() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            WORD_ITERATOR_SKIPPED_COUNT_METADATA_KEY.to_string(),
            serde_json::Value::Number(1.into()),
        );
        metadata.insert(
            AUTO_ROTATE_UNAVAILABLE_METADATA_KEY.to_string(),
            serde_json::Value::Bool(true),
        );
        assert_eq!(warnings_from_ocr_metadata(&metadata).len(), 2);
    }

    /// Round-trip test for the metadata -> warnings propagation path: the
    /// on-disk OCR cache (`ocr/cache.rs`) serializes `OcrExtractionResult`,
    /// including this `metadata` map, with `rmp_serde::to_vec_named` and
    /// decodes it back on a cache hit. This proves the two side-channel keys
    /// survive that exact round-trip and still produce the same warnings, so
    /// a cache hit surfaces the same `ProcessingWarning`s as a cache miss
    /// (#309).
    #[test]
    fn warnings_from_ocr_metadata_survives_msgpack_cache_round_trip() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            WORD_ITERATOR_SKIPPED_COUNT_METADATA_KEY.to_string(),
            serde_json::Value::Number(5.into()),
        );
        metadata.insert(
            AUTO_ROTATE_UNAVAILABLE_METADATA_KEY.to_string(),
            serde_json::Value::Bool(true),
        );
        metadata.insert(
            DICTIONARY_FILTERED_LINE_COUNT_METADATA_KEY.to_string(),
            serde_json::Value::Number(3.into()),
        );

        let serialized = rmp_serde::to_vec_named(&metadata).expect("metadata must serialize for the OCR cache");
        let round_tripped: std::collections::HashMap<String, serde_json::Value> =
            rmp_serde::from_slice(&serialized).expect("metadata must deserialize from the OCR cache");

        let before = warnings_from_ocr_metadata(&metadata);
        let after = warnings_from_ocr_metadata(&round_tripped);
        assert_eq!(before.len(), 3);
        assert_eq!(after.len(), 3);
        for (before_warning, after_warning) in before.iter().zip(&after) {
            assert_eq!(before_warning.source, after_warning.source);
            assert_eq!(before_warning.message, after_warning.message);
        }
    }

    /// #354: `word_iterator_skipped_count` is pipeline plumbing consumed by
    /// `warnings_from_ocr_metadata` -- it must not survive into the
    /// user-visible metadata map that becomes `Metadata::additional`.
    #[test]
    fn strip_ocr_scratch_metadata_keys_removes_word_iterator_skipped_count() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            WORD_ITERATOR_SKIPPED_COUNT_METADATA_KEY.to_string(),
            serde_json::Value::Number(2.into()),
        );

        strip_ocr_scratch_metadata_keys(&mut metadata);

        assert!(!metadata.contains_key(WORD_ITERATOR_SKIPPED_COUNT_METADATA_KEY));
    }

    /// #354: `auto_rotate_unavailable` is pipeline plumbing consumed by
    /// `warnings_from_ocr_metadata` -- it must not survive into the
    /// user-visible metadata map that becomes `Metadata::additional`.
    #[test]
    fn strip_ocr_scratch_metadata_keys_removes_auto_rotate_unavailable() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            AUTO_ROTATE_UNAVAILABLE_METADATA_KEY.to_string(),
            serde_json::Value::Bool(true),
        );

        strip_ocr_scratch_metadata_keys(&mut metadata);

        assert!(!metadata.contains_key(AUTO_ROTATE_UNAVAILABLE_METADATA_KEY));
    }

    /// #354: `pre_formatted` is promoted to `Metadata::output_format`, so it
    /// must be removed from the metadata map (not merely read), or it would
    /// also leak into `Metadata::additional` once the remaining map is
    /// copied wholesale.
    #[test]
    fn extract_pre_formatted_metadata_removes_key_and_returns_value() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            PRE_FORMATTED_METADATA_KEY.to_string(),
            serde_json::Value::String("markdown".to_string()),
        );

        let extracted = extract_pre_formatted_metadata(&mut metadata);

        assert_eq!(extracted.as_deref(), Some("markdown"));
        assert!(!metadata.contains_key(PRE_FORMATTED_METADATA_KEY));
    }

    #[test]
    fn image_preprocessing_metadata_is_promoted_to_the_typed_document_field() {
        let mut metadata = std::collections::HashMap::from([
            (
                crate::ocr_metadata_keys::OCR_IMAGE_PREPROCESSING_METADATA_KEY.to_string(),
                serde_json::json!({
                    "original_dimensions": [4, 4],
                    "original_dpi": [72.0, 72.0],
                    "target_dpi": 300,
                    "scale_factor": 0.5,
                    "auto_adjusted": false,
                    "final_dpi": 36,
                    "new_dimensions": [2, 2],
                    "resample_method": "LANCZOS3",
                    "dimension_clamped": true,
                    "calculated_dpi": null,
                    "skipped_resize": false,
                    "resize_error": null
                }),
            ),
            ("mean_text_conf".to_string(), serde_json::json!(98.5)),
        ]);

        let promoted =
            extract_image_preprocessing_metadata(&mut metadata).expect("valid preprocessing metadata must be promoted");

        assert_eq!(promoted.original_dimensions.width, 4);
        assert_eq!(promoted.original_dimensions.height, 4);
        assert_eq!(
            promoted.new_dimensions.as_ref().map(|dimensions| dimensions.width),
            Some(2)
        );
        assert_eq!(
            promoted.new_dimensions.as_ref().map(|dimensions| dimensions.height),
            Some(2)
        );
        assert!(promoted.dimension_clamped);
        assert!(!metadata.contains_key(crate::ocr_metadata_keys::OCR_IMAGE_PREPROCESSING_METADATA_KEY));
        assert_eq!(metadata.get("mean_text_conf"), Some(&serde_json::json!(98.5)));
    }

    /// #354 must not over-fire: genuine document metadata that happens to
    /// share the map with the scratch keys has to survive the filter intact,
    /// both in value and key set, so callers still see it in
    /// `Metadata::additional`.
    #[test]
    fn strip_ocr_scratch_metadata_keys_preserves_genuine_user_metadata() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("language".to_string(), serde_json::Value::String("eng".to_string()));
        metadata.insert("mean_text_conf".to_string(), serde_json::Value::Number(87.into()));
        metadata.insert(
            WORD_ITERATOR_SKIPPED_COUNT_METADATA_KEY.to_string(),
            serde_json::Value::Number(1.into()),
        );
        metadata.insert(
            AUTO_ROTATE_UNAVAILABLE_METADATA_KEY.to_string(),
            serde_json::Value::Bool(true),
        );
        metadata.insert(
            DICTIONARY_FILTERED_LINE_COUNT_METADATA_KEY.to_string(),
            serde_json::Value::Number(1.into()),
        );

        strip_ocr_scratch_metadata_keys(&mut metadata);

        assert_eq!(metadata.len(), 2);
        assert_eq!(
            metadata.get("language"),
            Some(&serde_json::Value::String("eng".to_string()))
        );
        assert_eq!(
            metadata.get("mean_text_conf"),
            Some(&serde_json::Value::Number(87.into()))
        );
    }

    #[test]
    fn tesseract_backend_does_not_eagerly_allocate_processor() {
        let backend = TesseractBackend::new();
        assert!(
            !backend.processor_is_initialized(),
            "TesseractBackend::new() should not eagerly allocate the processor"
        );
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod probe_tests {
    use super::*;
    use crate::doctor::ProbeStatus;

    fn config_with_tessdata(path: &Path, languages: &[&str]) -> OcrConfig {
        OcrConfig {
            tessdata_path: Some(path.to_path_buf()),
            language: languages.iter().map(|l| l.to_string()).collect(),
            ..OcrConfig::default()
        }
    }

    fn probe_with_dirs(config: &OcrConfig, dirs: &[&Path]) -> crate::doctor::DoctorCheck {
        let dirs: Vec<String> = dirs.iter().map(|d| d.to_string_lossy().into_owned()).collect();
        probe_tessdata_in_dirs(config, &dirs)
    }

    #[test]
    fn probe_passes_when_all_languages_present() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("eng.traineddata"), b"fake").unwrap();
        let check = probe_with_dirs(&config_with_tessdata(dir.path(), &["eng"]), &[dir.path()]);
        assert_eq!(check.status, ProbeStatus::Pass);
        assert!(check.message.contains(dir.path().to_str().unwrap()));
    }

    #[test]
    fn probe_skips_downloadable_language() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("eng.traineddata"), b"fake").unwrap();
        let check = probe_with_dirs(&config_with_tessdata(dir.path(), &["eng", "deu"]), &[dir.path()]);
        assert_eq!(check.status, ProbeStatus::Skip);
        assert!(check.message.contains("deu"));
    }

    #[test]
    fn probe_fails_on_unknown_language_code() {
        let dir = tempfile::TempDir::new().unwrap();
        let check = probe_with_dirs(&config_with_tessdata(dir.path(), &["xx9"]), &[dir.path()]);
        assert_eq!(check.status, ProbeStatus::Fail);
        assert!(check.message.contains("xx9"));
    }
}
