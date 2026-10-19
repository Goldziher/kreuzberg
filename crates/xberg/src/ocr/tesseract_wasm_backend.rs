//! WASM-compatible Tesseract OCR backend.
//!
//! Drives the Tesseract+Leptonica WASI build (provided by `xberg-tesseract`'s
//! `build-tesseract-wasm` feature) via in-memory tessdata, so OCR works on
//! `wasm32-unknown-unknown` with no filesystem and no JavaScript dependencies.
//!
//! Tessdata bytes can come from two sources, in priority order:
//! 1. `OcrConfig::tessdata_bytes` — caller-supplied per-language map.
//! 2. The `bundle-tessdata-eng` feature on `xberg-tesseract`, which embeds
//!    the English `eng.traineddata` (~4 MB, tessdata_fast) into the WASM
//!    binary at compile time.
//!
//! Without either, this backend returns a `MissingDependency` error explaining
//! how to provide tessdata.

use crate::Result;
use crate::core::config::OcrConfig;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::{ExtractedDocument, FormatMetadata, Metadata, OcrElement, OcrMetadata};
use async_trait::async_trait;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;
use std::sync::Mutex;
use xberg_tesseract::{Pix, TessMonitor, TessPageSegMode, TesseractAPI, WordData};

/// Default OCR engine mode: LSTM only (mode 1). Matches the `OEM_LSTM_ONLY`
/// constant from Tesseract's `tesseract/publictypes.h`. LSTM is the only
/// recognition engine compiled into our WASI Tesseract build.
const OEM_LSTM_ONLY: i32 = 1;

/// Bounded default page segmentation mode used when `OcrConfig::tesseract_config`
/// carries no explicit (or an out-of-range) PSM.
///
/// `PSM_AUTO` (3) — Tesseract's own library default — is known to hang or
/// abort inside the WASI-compiled Tesseract build (issue #855: "Tesseract
/// PSM_AUTO hangs 60-90s in WASM build"), surfacing to callers as an
/// uncatchable wasm `unreachable` trap. `PSM_SINGLE_BLOCK` treats the whole
/// image as one block of text, which is safe in the WASM build and matches
/// `TesseractConfig::default()`'s wasm32-specific `psm: 6`.
const DEFAULT_WASM_PSM: TessPageSegMode = TessPageSegMode::PSM_SINGLE_BLOCK;

/// Recognition deadline, in milliseconds, enforced via `TessMonitor`.
///
/// Bounds worst-case recognition time for a pathological image so a stuck
/// recognition run fails gracefully instead of hanging indefinitely. Kept
/// well under typical caller-side timeouts (e.g. the 30s WASM smoke-test
/// limit that exposed issue #855).
const RECOGNITION_DEADLINE_MS: i32 = 15_000;
const MAX_WASM_OCR_IMAGE_DIMENSION: u32 = 4_096;
const MAX_WASM_OCR_IMAGE_PIXELS: u64 = MAX_WASM_OCR_IMAGE_DIMENSION as u64 * MAX_WASM_OCR_IMAGE_DIMENSION as u64;
const MAX_WASM_OCR_DECODE_ALLOCATION_BYTES: u64 = 128 * 1024 * 1024;

/// WASM-compatible Tesseract OCR backend.
#[cfg_attr(alef, alef(skip))]
pub struct TesseractWasmBackend {
    /// Process-local tessdata cache, keyed by language code.
    tessdata_cache: Mutex<HashMap<String, Vec<u8>>>,
}

impl TesseractWasmBackend {
    /// Create a new Tesseract WASM backend.
    pub(crate) fn new() -> Result<Self> {
        Ok(Self {
            tessdata_cache: Mutex::new(HashMap::new()),
        })
    }

    /// Resolve tessdata bytes for a language, consulting the cache, the
    /// supplied OcrConfig, and the optional bundled-eng compile-time blob.
    fn resolve_tessdata(&self, language: &str, config: &OcrConfig) -> Result<Vec<u8>> {
        if let Ok(cache) = self.tessdata_cache.lock()
            && let Some(cached) = cache.get(language)
        {
            return Ok(cached.clone());
        }

        if let Some(ref user_supplied) = config.tessdata_bytes
            && let Some(bytes) = user_supplied.get(language)
        {
            self.cache_tessdata(language, bytes.clone());
            return Ok(bytes.clone());
        }

        if language == "eng"
            && let Some(bundled) = bundled_eng_traineddata()
        {
            self.cache_tessdata(language, bundled.to_vec());
            return Ok(bundled.to_vec());
        }

        Err(crate::XbergError::MissingDependency(format!(
            "Tesseract tessdata for language '{language}' not available on WASM. \
             Provide bytes via OcrConfig::tessdata_bytes, or build with the \
             'bundle-tessdata-eng' feature for English."
        )))
    }

    fn cache_tessdata(&self, language: &str, bytes: Vec<u8>) {
        if let Ok(mut cache) = self.tessdata_cache.lock() {
            cache.insert(language.to_string(), bytes);
        }
    }
}

impl Plugin for TesseractWasmBackend {
    fn name(&self) -> &str {
        "tesseract"
    }

    fn version(&self) -> String {
        TesseractAPI::version()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Inherits the `RequiresUpright` default for `page_orientation_handling`, the same value the native
/// `TesseractBackend` declares (#2172).
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl OcrBackend for TesseractWasmBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractedDocument> {
        if image_bytes.is_empty() {
            return Err(crate::XbergError::Validation {
                message: "OCR input image is empty".to_string(),
                source: None,
            });
        }

        // Reconciled with `tesseract_config.language` the same way the native backend does
        // (#1572), so the two Tesseract backends agree on which field wins.
        let languages = config.effective_tesseract_language();
        let language = languages[0].clone();
        if languages.len() > 1 {
            tracing::warn!(
                requested = ?languages,
                used = %language,
                "WASM Tesseract backend recognizes a single language per call; using the primary language"
            );
        }
        let tessdata = self.resolve_tessdata(&language, config)?;
        let pix = prepare_ocr_pix(image_bytes, config)?;
        let psm_mode = resolve_psm(config);
        let include_elements = config
            .element_config
            .as_ref()
            .is_some_and(|options| options.include_elements);
        let recognition = run_tesseract_recognition(&tessdata, &language, psm_mode, &pix, include_elements)?;
        let ocr_elements = select_output_ocr_elements(recognition.words, config);

        let metadata = Metadata {
            format: Some(FormatMetadata::Ocr(OcrMetadata {
                language: language.clone(),
                psm: psm_mode as i32,
                output_format: "text".to_string(),
                table_count: 0,
                table_rows: None,
                table_cols: None,
            })),
            ..Default::default()
        };

        Ok(ExtractedDocument {
            content: recognition.text,
            mime_type: Cow::Borrowed("text/plain"),
            metadata,
            ocr_elements,
            ..Default::default()
        })
    }

    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractedDocument> {
        let bytes = std::fs::read(path).map_err(crate::XbergError::from)?;
        self.process_image(&bytes, config).await
    }

    fn supports_language(&self, _lang: &str) -> bool {
        true
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Tesseract
    }

    /// The WASM Tesseract backend reports no page-level confidence.
    fn confidence_semantics(&self) -> crate::plugins::ConfidenceSemantics {
        crate::plugins::ConfidenceSemantics::None
    }

    // Rotation handling stays on the trait's `RequiresUpright` default, matching the native backend. ~keep
}

/// Returns the compile-time-bundled English tessdata when the
/// `xberg-tesseract/bundle-tessdata-eng` feature is on, otherwise `None`.
fn bundled_eng_traineddata() -> Option<&'static [u8]> {
    xberg_tesseract::bundled_eng_traineddata()
}

/// Resolves the page segmentation mode to use for a recognition call.
///
/// Respects `config.tesseract_config.psm` when it is explicitly set (not just when
/// `tesseract_config` itself is present — #1573) and maps to a valid `TessPageSegMode`.
/// Falls back to [`DEFAULT_WASM_PSM`] — never to Tesseract's own `PSM_AUTO` default —
/// when `psm` is unset or carries an out-of-range value, so callers can never end up
/// hitting the PSM_AUTO hang described in issue #855 by omission.
fn resolve_psm(config: &OcrConfig) -> TessPageSegMode {
    config
        .tesseract_config
        .as_ref()
        .and_then(|c| c.psm)
        .and_then(TessPageSegMode::try_from_int)
        .unwrap_or(DEFAULT_WASM_PSM)
}

fn resolve_preprocessing(config: &OcrConfig) -> Option<&crate::types::ImagePreprocessingConfig> {
    config
        .tesseract_config
        .as_ref()
        .and_then(|tesseract| tesseract.preprocessing.as_ref())
}

fn decode_wasm_ocr_image(image_bytes: &[u8]) -> Result<image::DynamicImage> {
    let limits = wasm_ocr_decode_limits();
    let mut reader = image::ImageReader::new(Cursor::new(image_bytes))
        .with_guessed_format()
        .map_err(ocr_image_decode_error)?;
    reader.limits(limits.clone());
    let format = reader.format().ok_or_else(|| crate::XbergError::Validation {
        message: "OCR input image format could not be determined".to_string(),
        source: None,
    })?;
    let dimensions = reader.into_dimensions().map_err(wasm_ocr_image_error)?;
    validate_wasm_ocr_dimensions(dimensions.0, dimensions.1)?;

    let mut reader = image::ImageReader::with_format(Cursor::new(image_bytes), format);
    reader.limits(limits);
    reader.decode().map_err(wasm_ocr_image_error)
}

/// Decodes `image_bytes` into a Leptonica [`Pix`] and applies the configured
/// preprocessing, if any. Split out of [`TesseractWasmBackend::process_image`]
/// purely to shorten that method.
fn prepare_ocr_pix(image_bytes: &[u8], config: &OcrConfig) -> Result<Pix> {
    let img = decode_wasm_ocr_image(image_bytes)?;
    let rgb = img.into_rgb8();
    let (width, height) = rgb.dimensions();
    let pix = Pix::from_raw_rgb(rgb.as_raw(), width, height).map_err(|e| crate::XbergError::Ocr {
        message: format!("Failed to create Leptonica Pix from image: {e}"),
        source: Some(Box::new(e)),
    })?;
    drop(rgb);
    match resolve_preprocessing(config) {
        Some(preprocessing) => {
            crate::ocr::preprocessing::preprocess_pix(pix, preprocessing).map_err(|error| crate::XbergError::Ocr {
                message: format!("Failed to preprocess image for OCR: {error}"),
                source: Some(Box::new(error)),
            })
        }
        None => Ok(pix),
    }
}

struct WasmRecognition {
    text: String,
    words: Option<Vec<WordData>>,
}

/// Runs recognition and optionally reads the recognized words and their geometry. ~keep
fn run_tesseract_recognition(
    tessdata: &[u8],
    language: &str,
    psm_mode: TessPageSegMode,
    pix: &Pix,
    include_elements: bool,
) -> Result<WasmRecognition> {
    let api = TesseractAPI::new().map_err(|e| crate::XbergError::Ocr {
        message: format!("Failed to create Tesseract API handle: {e}"),
        source: Some(Box::new(e)),
    })?;

    api.init_5(tessdata, tessdata.len() as i32, language, OEM_LSTM_ONLY, &[])
        .map_err(|e| crate::XbergError::Ocr {
            message: format!("Failed to init Tesseract with bundled tessdata: {e}"),
            source: Some(Box::new(e)),
        })?;

    api.set_page_seg_mode(psm_mode).map_err(|e| crate::XbergError::Ocr {
        message: format!("Failed to set Tesseract page segmentation mode: {e}"),
        source: Some(Box::new(e)),
    })?;

    api.set_image_2(pix.as_ptr()).map_err(|e| crate::XbergError::Ocr {
        message: format!("Failed to set image on Tesseract API: {e}"),
        source: Some(Box::new(e)),
    })?;

    let monitor = TessMonitor::new();
    monitor
        .set_deadline(RECOGNITION_DEADLINE_MS)
        .map_err(|e| crate::XbergError::Ocr {
            message: format!("Failed to configure Tesseract recognition deadline: {e}"),
            source: Some(Box::new(e)),
        })?;
    api.recognize_with_monitor(&monitor)
        .map_err(|e| crate::XbergError::Ocr {
            message: format!("Tesseract recognition failed or exceeded its deadline: {e}"),
            source: Some(Box::new(e)),
        })?;
    let text = api.get_utf8_text().map_err(|e| crate::XbergError::Ocr {
        message: format!("Failed to read Tesseract text output: {e}"),
        source: Some(Box::new(e)),
    })?;
    let words = include_elements
        .then(|| {
            api.get_iterator()
                .and_then(|iterator| iterator.extract_all_words())
                .map(|outcome| {
                    if outcome.skipped > 0 {
                        tracing::warn!(skipped = outcome.skipped, "WASM Tesseract skipped malformed OCR words");
                    }
                    outcome.words
                })
        })
        .transpose()
        .map_err(|e| crate::XbergError::Ocr {
            message: format!("Failed to read Tesseract word elements: {e}"),
            source: Some(Box::new(e)),
        })?;

    Ok(WasmRecognition { text, words })
}

fn select_output_ocr_elements(words: Option<Vec<WordData>>, config: &OcrConfig) -> Option<Vec<OcrElement>> {
    let options = config.element_config.as_ref()?;
    let elements = words
        .unwrap_or_default()
        .iter()
        .map(|word| crate::ocr::conversion::iterator_word_to_element(word, None, None, 1))
        .collect::<Vec<_>>();
    let selected = options.select_elements(&elements);
    (!selected.is_empty()).then_some(selected)
}

fn wasm_ocr_decode_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_WASM_OCR_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_WASM_OCR_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_WASM_OCR_DECODE_ALLOCATION_BYTES);
    limits
}

fn validate_wasm_ocr_dimensions(width: u32, height: u32) -> Result<()> {
    let pixels = u64::from(width) * u64::from(height);
    if width == 0
        || height == 0
        || width > MAX_WASM_OCR_IMAGE_DIMENSION
        || height > MAX_WASM_OCR_IMAGE_DIMENSION
        || pixels > MAX_WASM_OCR_IMAGE_PIXELS
    {
        return Err(crate::XbergError::Validation {
            message: format!(
                "OCR input dimensions {width}x{height} exceed the WebAssembly limit of \
                 {MAX_WASM_OCR_IMAGE_DIMENSION}x{MAX_WASM_OCR_IMAGE_DIMENSION}"
            ),
            source: None,
        });
    }
    Ok(())
}

fn ocr_image_decode_error(error: impl std::error::Error + Send + Sync + 'static) -> crate::XbergError {
    crate::XbergError::Ocr {
        message: format!("Failed to decode image for OCR: {error}"),
        source: Some(Box::new(error)),
    }
}

fn wasm_ocr_image_error(error: image::ImageError) -> crate::XbergError {
    if matches!(error, image::ImageError::Limits(_)) {
        crate::XbergError::Validation {
            message: "OCR input image exceeds WebAssembly decoding limits".to_string(),
            source: Some(Box::new(error)),
        }
    } else {
        ocr_image_decode_error(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OcrBoundingGeometry, OcrElementConfig, OcrElementLevel};

    /// This is a native-compilable test of the PSM-selection logic only.
    /// `TesseractWasmBackend::process_image` itself drives the WASI-compiled
    /// Tesseract engine, which requires a wasm32 build with the
    /// `build-tesseract-wasm` toolchain and is not exercised by `cargo test`
    /// on this target; the deadline/monitor wiring around `recognize()` is
    /// therefore not covered by an automated test here.
    #[test]
    fn should_use_default_wasm_psm_when_config_has_no_tesseract_config() {
        let config = OcrConfig {
            tesseract_config: None,
            ..Default::default()
        };

        assert_eq!(resolve_psm(&config), DEFAULT_WASM_PSM);
        assert_eq!(DEFAULT_WASM_PSM, TessPageSegMode::PSM_SINGLE_BLOCK);
    }

    #[test]
    fn should_never_resolve_to_psm_auto_when_config_has_no_tesseract_config() {
        let config = OcrConfig {
            tesseract_config: None,
            ..Default::default()
        };

        assert_ne!(resolve_psm(&config), TessPageSegMode::PSM_AUTO);
    }

    #[test]
    fn should_resolve_wasm_preprocessing_from_tesseract_config() {
        let preprocessing = crate::types::ImagePreprocessingConfig {
            denoise: true,
            ..Default::default()
        };
        let config = OcrConfig {
            tesseract_config: Some(crate::types::TesseractConfig {
                preprocessing: Some(preprocessing),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert!(resolve_preprocessing(&config).is_some_and(|resolved| resolved.denoise));
    }

    #[test]
    fn should_reject_wasm_ocr_images_over_the_dimension_budget() {
        let image = image::DynamicImage::ImageRgb8(image::RgbImage::new(MAX_WASM_OCR_IMAGE_DIMENSION + 1, 1));
        let mut encoded = Cursor::new(Vec::new());
        image.write_to(&mut encoded, image::ImageFormat::Png).unwrap();

        let result = decode_wasm_ocr_image(encoded.get_ref());

        assert!(matches!(result, Err(crate::XbergError::Validation { .. })));
    }

    #[test]
    fn should_accept_wasm_ocr_images_within_the_pixel_budget() {
        assert!(validate_wasm_ocr_dimensions(MAX_WASM_OCR_IMAGE_DIMENSION, MAX_WASM_OCR_IMAGE_DIMENSION).is_ok());
    }

    #[test]
    fn should_respect_explicit_psm_from_tesseract_config() {
        let config = OcrConfig {
            tesseract_config: Some(crate::types::TesseractConfig {
                psm: Some(7),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(resolve_psm(&config), TessPageSegMode::PSM_SINGLE_LINE);
    }

    #[test]
    fn should_respect_explicit_psm_auto_when_caller_opts_in() {
        let config = OcrConfig {
            tesseract_config: Some(crate::types::TesseractConfig {
                psm: Some(3),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(resolve_psm(&config), TessPageSegMode::PSM_AUTO);
    }

    #[test]
    fn should_fall_back_to_default_wasm_psm_for_out_of_range_psm_value() {
        let config = OcrConfig {
            tesseract_config: Some(crate::types::TesseractConfig {
                psm: Some(255),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(resolve_psm(&config), DEFAULT_WASM_PSM);
    }

    // Regression test for #1573: a `TesseractConfig` present for a reason unrelated to
    // `psm` must still resolve to the platform default PSM, exactly as if the struct
    // were absent — the field's absence, not the struct's, is what should matter.
    #[test]
    fn should_fall_back_to_default_wasm_psm_when_tesseract_config_present_but_psm_unset() {
        let config = OcrConfig {
            tesseract_config: Some(crate::types::TesseractConfig {
                enable_table_detection: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(resolve_psm(&config), DEFAULT_WASM_PSM);
    }

    #[test]
    fn should_return_selected_word_elements_with_geometry_and_confidence() {
        let config = OcrConfig {
            element_config: Some(OcrElementConfig {
                include_elements: true,
                min_level: OcrElementLevel::Word,
                min_confidence: 0.8,
                build_hierarchy: false,
            }),
            ..Default::default()
        };
        let words = vec![
            WordData {
                text: "Hello".to_string(),
                left: 12,
                top: 8,
                right: 42,
                bottom: 24,
                confidence: 95.0,
                ..Default::default()
            },
            WordData {
                text: "noise".to_string(),
                confidence: 40.0,
                ..Default::default()
            },
        ];

        let elements = select_output_ocr_elements(Some(words), &config).expect("word element");

        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].text, "Hello");
        assert_eq!(elements[0].level, OcrElementLevel::Word);
        assert_eq!(elements[0].confidence.recognition, 0.95);
        assert_eq!(
            elements[0].geometry,
            OcrBoundingGeometry::Rectangle {
                left: 12,
                top: 8,
                width: 30,
                height: 16,
            }
        );
    }
}
