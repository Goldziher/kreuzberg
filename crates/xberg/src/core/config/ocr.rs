//! OCR configuration.
//!
//! Defines OCR-specific configuration including backend selection, language settings,
//! Tesseract-specific parameters, quality thresholds, and multi-backend pipeline config.

use serde::{Deserialize, Deserializer, Serialize};
use std::path::PathBuf;

use super::formats::OutputFormat;
use crate::core::config_validation::validate_ocr_backend;
use crate::error::XbergError;
use crate::types::OcrElementConfig;

/// Deserialize a language field that accepts either a string or a list of strings.
///
/// This helper enables backward compatibility: old configs with `language: "eng"`
/// deserialize to `Vec<String>` via coercion, while new configs with
/// `language: ["eng", "deu"]` deserialize directly.
fn deserialize_languages<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let value: serde_json::Value = serde_json::Value::deserialize(deserializer)?;

    match value {
        serde_json::Value::String(s) => {
            if s.contains('+') {
                Ok(s.split('+').map(|l| l.to_string()).collect())
            } else {
                Ok(vec![s])
            }
        }
        serde_json::Value::Array(arr) => arr
            .into_iter()
            .map(|v| {
                v.as_str()
                    .map(String::from)
                    .ok_or_else(|| Error::custom("each language must be a string"))
            })
            .collect(),
        _ => Err(Error::custom(
            "language must be a string (e.g., \"eng\") or an array of strings (e.g., [\"eng\", \"deu\"])",
        )),
    }
}

/// Deserialize an optional language field (for pipeline stages).
fn deserialize_optional_languages<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;

    match value {
        None => Ok(None),
        Some(serde_json::Value::String(s)) => {
            if s.contains('+') {
                Ok(Some(s.split('+').map(|l| l.to_string()).collect()))
            } else {
                Ok(Some(vec![s]))
            }
        }
        Some(serde_json::Value::Array(arr)) => {
            let langs: Result<Vec<String>, D::Error> = arr
                .into_iter()
                .map(|v| {
                    v.as_str()
                        .map(String::from)
                        .ok_or_else(|| Error::custom("each language must be a string"))
                })
                .collect();
            langs.map(Some)
        }
        Some(_) => Err(Error::custom("language must be a string or an array of strings")),
    }
}

/// Quality thresholds for OCR fallback decisions and pipeline quality gating.
///
/// Fields default to conservative extraction behavior. Suspected OCR recognition noise is
/// reported but retained unless destructive filtering is explicitly enabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OcrQualityThresholds {
    /// Minimum total non-whitespace characters to consider text substantive.
    #[serde(default = "default_min_total_non_whitespace")]
    pub min_total_non_whitespace: usize,

    /// Minimum non-whitespace characters per page on average.
    #[serde(default = "default_min_non_whitespace_per_page")]
    pub min_non_whitespace_per_page: f64,

    /// Minimum character count for a word to be "meaningful".
    #[serde(default = "default_min_meaningful_word_len")]
    pub min_meaningful_word_len: usize,

    /// Minimum count of meaningful words before text is accepted.
    #[serde(default = "default_min_meaningful_words")]
    pub min_meaningful_words: usize,

    /// Minimum alphanumeric ratio (non-whitespace chars that are alphanumeric).
    #[serde(default = "default_min_alnum_ratio")]
    pub min_alnum_ratio: f64,

    /// Minimum Unicode replacement characters (U+FFFD) to trigger OCR fallback.
    #[serde(default = "default_min_garbage_chars")]
    pub min_garbage_chars: usize,

    /// Maximum fraction of short (1-2 char) words before text is considered fragmented.
    #[serde(default = "default_max_fragmented_word_ratio")]
    pub max_fragmented_word_ratio: f64,

    /// Critical fragmentation threshold — triggers OCR regardless of meaningful words.
    /// Normal English text has ~20-30% short words. 80%+ is definitive garbage.
    #[serde(default = "default_critical_fragmented_word_ratio")]
    pub critical_fragmented_word_ratio: f64,

    /// Maximum fraction of short (1-2 char) words an *OCR result* may carry before the page
    /// is reported as suspected recognition noise.
    ///
    /// This is a different decision, and a different operating point, from
    /// [`Self::max_fragmented_word_ratio`] / [`Self::critical_fragmented_word_ratio`]: those
    /// ask "is the native text bad enough that we should OCR this page?", and are tuned to
    /// only fire on text that is already definitively broken (0.6 / 0.8). Here we are judging
    /// what OCR *produced*, where the failure mode is an engine run over line art — a scanned
    /// plat, an engineering drawing, a signature flourish — returning confident-looking
    /// strings that are not words.
    ///
    /// Measured over the 16 pages of a recorded municipal ordinance (13 prose pages, 3 scanned
    /// survey drawings): prose ran 0.04-0.28, the drawings 0.42-0.47. The default sits in that
    /// gap with margin on both sides. By default, crossing the threshold emits a processing
    /// warning but retains the recognized text. Set [`Self::discard_suspected_ocr_noise`] to
    /// `true` to restore destructive filtering.
    #[serde(default = "default_max_ocr_output_fragmented_word_ratio")]
    pub max_ocr_output_fragmented_word_ratio: f64,

    /// Minimum mean OCR confidence (0-100) below which a page is reported as suspected noise.
    ///
    /// This is the engine's own uncertainty about what it read, and it is a far sharper
    /// instrument than any statistic derived from the output text. Measured per page over a
    /// recorded municipal ordinance (10 prose pages, 6 scanned survey/architectural drawings)
    /// with Tesseract 5.5.3:
    ///
    /// ```text
    /// prose      86.3 - 95.3
    /// drawings   18.5 - 64.3
    /// ```
    ///
    /// The default sits in that gap with ~11 points of margin on each side. Compare the
    /// short-word ratio, which separated the same two groups by 0.09 on a 0-1 scale.
    ///
    /// A backend that reports no confidence (no `mean_text_conf` in its result metadata)
    /// skips this check entirely and falls back to
    /// [`Self::max_ocr_output_fragmented_word_ratio`]. By default, the signal emits a warning
    /// without discarding content. Set to 0.0 to disable the signal.
    #[serde(default = "default_min_ocr_mean_confidence")]
    pub min_ocr_mean_confidence: f64,

    /// Minimum word count before [`Self::max_ocr_output_fragmented_word_ratio`] may report a
    /// page. Short pages (a signature block, an exhibit title) are legitimately dominated by
    /// short words, so the ratio is not meaningful on them and the signal stays disabled.
    #[serde(default = "default_min_words_for_ocr_output_check")]
    pub min_words_for_ocr_output_check: usize,

    /// Maximum fraction of a Tesseract page's dictionary-checkable words that
    /// `TesseractAPI::is_valid_word` may reject before the page is treated as recognition
    /// noise, supplementing (never replacing) [`Self::max_ocr_output_fragmented_word_ratio`].
    ///
    /// Tesseract-only: other backends never report this ratio (see
    /// `dictionary_invalid_word_ratio` in `ocr::processor::execution`), so this threshold is
    /// simply never consulted for them.
    ///
    /// Calibration owed, same as `ocr::types::TesseractConfig::min_confidence`:
    /// unlike [`Self::max_ocr_output_fragmented_word_ratio`] (measured across a recorded
    /// ordinance's prose vs. drawing pages), this ratio has not yet had a page-level
    /// measurement run over a labeled corpus. The default therefore disables the check
    /// entirely (`1.01`, above the `[0.0, 1.0]` range a ratio can reach) rather than guess an
    /// operating point. Do not lower this without running that measurement first.
    #[serde(default = "default_max_ocr_output_dict_invalid_word_ratio")]
    pub max_ocr_output_dict_invalid_word_ratio: f64,

    /// Discard non-empty OCR text when any configured recognition-noise signal fires.
    ///
    /// Defaults to `false`: suspected noise is retained and surfaced through
    /// `processing_warnings`. Set to `true` to preserve the legacy destructive behavior.
    #[serde(default)]
    pub discard_suspected_ocr_noise: bool,

    /// Minimum average word length. Below this with enough words indicates garbled extraction.
    #[serde(default = "default_min_avg_word_length")]
    pub min_avg_word_length: f64,

    /// Minimum word count before average word length check applies.
    #[serde(default = "default_min_words_for_avg_length_check")]
    pub min_words_for_avg_length_check: usize,

    /// Minimum consecutive word repetition ratio to detect column scrambling.
    #[serde(default = "default_min_consecutive_repeat_ratio")]
    pub min_consecutive_repeat_ratio: f64,

    /// Minimum word count before consecutive repetition check is applied.
    #[serde(default = "default_min_words_for_repeat_check")]
    pub min_words_for_repeat_check: usize,

    /// Minimum character count for "substantive markdown" OCR skip gate.
    #[serde(default = "default_substantive_min_chars")]
    pub substantive_min_chars: usize,

    /// Minimum character count for "non-text content" OCR skip gate.
    #[serde(default = "default_non_text_min_chars")]
    pub non_text_min_chars: usize,

    /// Alphanumeric+whitespace ratio threshold for skip decisions.
    #[serde(default = "default_alnum_ws_ratio_threshold")]
    pub alnum_ws_ratio_threshold: f64,

    /// Minimum quality score (0.0-1.0) for a pipeline stage result to be accepted.
    /// If the result from a backend scores below this, try the next backend.
    #[serde(default = "default_pipeline_min_quality")]
    pub pipeline_min_quality: f64,

    /// Minimum fraction of non-whitespace characters that are undecodable
    /// (Unicode Private Use Area, replacement characters, or non-whitespace
    /// control characters) before a page's text layer is treated as
    /// unreadable and routed to OCR (issue #1254). Gated by
    /// `min_total_non_whitespace` so short snippets with a stray symbol or
    /// two do not trip this check.
    #[serde(default = "default_min_undecodable_ratio")]
    pub min_undecodable_ratio: f64,

    /// Whether to route a page to OCR when xberg_native_pdf reports that a high
    /// fraction of its text was fabricated rather than read from the file
    /// (`MappingProvenance::Fallback`, xberg_native_pdf 0.3.75+, issue #1254). This is
    /// a direct fact from the extractor's ISO 32000-1 §9.10.2 mapping cascade,
    /// distinct from the character-heuristic proxy behind `min_undecodable_ratio`.
    /// Defaults to `true`.
    #[serde(default = "default_enable_provenance_ocr_routing")]
    pub enable_provenance_ocr_routing: bool,

    /// Minimum fraction of a page's non-whitespace characters with
    /// `MappingProvenance::Fallback` provenance before the page is treated as
    /// having a fabricated text layer and routed to OCR (issue #1254). Gated by
    /// `min_total_non_whitespace` so a short page with a stray fallback
    /// character cannot trip it. Only used when `enable_provenance_ocr_routing`
    /// is `true`.
    #[serde(default = "default_min_provenance_fallback_ratio")]
    pub min_provenance_fallback_ratio: f64,

    /// Whether to route a page to OCR when its decoded text does not read as any real,
    /// detectable language (issue #1696). Unlike `enable_provenance_ocr_routing`, this
    /// catches a `/ToUnicode` CMap that resolves every glyph to *a* character, but
    /// consistently the WRONG one (e.g. a ROT-shifted mapping) -- text that is structurally
    /// indistinguishable from real prose to every character-shape check, including the
    /// provenance signal, because the mapping tier really is file-backed. Defaults to `true`.
    #[serde(default = "default_enable_plausibility_ocr_routing")]
    pub enable_plausibility_ocr_routing: bool,

    /// Minimum fraction of a page's language-detection chunks that whatlang classifies as
    /// reliable (`Info::is_reliable()`) before the page is trusted as legible (issue #1696).
    /// Below this AND below the mean-confidence guard together, the page's text layer is
    /// treated as implausible and routed to OCR. Only used when
    /// `enable_plausibility_ocr_routing` is `true`.
    #[serde(default = "default_min_reliable_language_chunk_ratio")]
    pub min_reliable_language_chunk_ratio: f64,
}

const DISABLED_DICTIONARY_INVALID_WORD_RATIO: f64 = 1.01;

impl OcrQualityThresholds {
    fn validate(&self, path: &str) -> Result<(), XbergError> {
        let unit_ratios = [
            ("min_alnum_ratio", self.min_alnum_ratio),
            ("max_fragmented_word_ratio", self.max_fragmented_word_ratio),
            ("critical_fragmented_word_ratio", self.critical_fragmented_word_ratio),
            (
                "max_ocr_output_fragmented_word_ratio",
                self.max_ocr_output_fragmented_word_ratio,
            ),
            ("min_consecutive_repeat_ratio", self.min_consecutive_repeat_ratio),
            ("alnum_ws_ratio_threshold", self.alnum_ws_ratio_threshold),
            ("pipeline_min_quality", self.pipeline_min_quality),
            ("min_undecodable_ratio", self.min_undecodable_ratio),
            ("min_provenance_fallback_ratio", self.min_provenance_fallback_ratio),
            (
                "min_reliable_language_chunk_ratio",
                self.min_reliable_language_chunk_ratio,
            ),
        ];
        for (field, value) in unit_ratios {
            validate_quality_value(path, field, value, 0.0, 1.0)?;
        }

        validate_quality_value(
            path,
            "min_ocr_mean_confidence",
            self.min_ocr_mean_confidence,
            0.0,
            100.0,
        )?;
        validate_nonnegative_quality_value(path, "min_non_whitespace_per_page", self.min_non_whitespace_per_page)?;
        validate_nonnegative_quality_value(path, "min_avg_word_length", self.min_avg_word_length)?;

        if self.max_ocr_output_dict_invalid_word_ratio != DISABLED_DICTIONARY_INVALID_WORD_RATIO {
            validate_quality_value(
                path,
                "max_ocr_output_dict_invalid_word_ratio",
                self.max_ocr_output_dict_invalid_word_ratio,
                0.0,
                1.0,
            )?;
        }
        Ok(())
    }
}

fn validate_quality_value(path: &str, field: &str, value: f64, min: f64, max: f64) -> Result<(), XbergError> {
    if value.is_finite() && (min..=max).contains(&value) {
        return Ok(());
    }
    Err(XbergError::validation(format!(
        "{path}.{field} must be a finite value between {min} and {max}, got {value}"
    )))
}

fn validate_nonnegative_quality_value(path: &str, field: &str, value: f64) -> Result<(), XbergError> {
    if value.is_finite() && value >= 0.0 {
        return Ok(());
    }
    Err(XbergError::validation(format!(
        "{path}.{field} must be a finite nonnegative value, got {value}"
    )))
}

impl Default for OcrQualityThresholds {
    fn default() -> Self {
        Self {
            min_total_non_whitespace: 64,
            min_non_whitespace_per_page: 32.0,
            min_meaningful_word_len: 4,
            min_meaningful_words: 3,
            min_alnum_ratio: 0.3,
            min_garbage_chars: 5,
            max_fragmented_word_ratio: 0.6,
            critical_fragmented_word_ratio: 0.80,
            min_ocr_mean_confidence: default_min_ocr_mean_confidence(),
            max_ocr_output_fragmented_word_ratio: default_max_ocr_output_fragmented_word_ratio(),
            min_words_for_ocr_output_check: default_min_words_for_ocr_output_check(),
            max_ocr_output_dict_invalid_word_ratio: default_max_ocr_output_dict_invalid_word_ratio(),
            discard_suspected_ocr_noise: false,
            min_avg_word_length: 2.0,
            min_words_for_avg_length_check: 50,
            min_consecutive_repeat_ratio: 0.08,
            min_words_for_repeat_check: 50,
            substantive_min_chars: 100,
            non_text_min_chars: 20,
            alnum_ws_ratio_threshold: 0.4,
            pipeline_min_quality: 0.5,
            min_undecodable_ratio: default_min_undecodable_ratio(),
            enable_provenance_ocr_routing: default_enable_provenance_ocr_routing(),
            min_provenance_fallback_ratio: default_min_provenance_fallback_ratio(),
            enable_plausibility_ocr_routing: default_enable_plausibility_ocr_routing(),
            min_reliable_language_chunk_ratio: default_min_reliable_language_chunk_ratio(),
        }
    }
}

fn default_min_total_non_whitespace() -> usize {
    64
}
fn default_min_non_whitespace_per_page() -> f64 {
    32.0
}
fn default_min_meaningful_word_len() -> usize {
    4
}
fn default_min_meaningful_words() -> usize {
    3
}
fn default_min_alnum_ratio() -> f64 {
    0.3
}
fn default_min_garbage_chars() -> usize {
    5
}
fn default_max_fragmented_word_ratio() -> f64 {
    0.6
}
fn default_critical_fragmented_word_ratio() -> f64 {
    0.80
}
fn default_min_ocr_mean_confidence() -> f64 {
    75.0
}
fn default_max_ocr_output_fragmented_word_ratio() -> f64 {
    0.35
}
fn default_min_words_for_ocr_output_check() -> usize {
    20
}
/// Disables the dictionary-invalid-word-ratio veto by default: `1.01` is above `[0.0, 1.0]`,
/// the range the ratio can actually reach, so it can never fire until calibrated and lowered.
fn default_max_ocr_output_dict_invalid_word_ratio() -> f64 {
    DISABLED_DICTIONARY_INVALID_WORD_RATIO
}
fn default_min_avg_word_length() -> f64 {
    2.0
}
fn default_min_words_for_avg_length_check() -> usize {
    50
}
fn default_min_consecutive_repeat_ratio() -> f64 {
    0.08
}
fn default_min_words_for_repeat_check() -> usize {
    50
}
fn default_substantive_min_chars() -> usize {
    100
}
fn default_non_text_min_chars() -> usize {
    20
}
fn default_alnum_ws_ratio_threshold() -> f64 {
    0.4
}
fn default_pipeline_min_quality() -> f64 {
    0.5
}
/// Pages at or above this fraction of undecodable (PUA/replacement/control-garbage)
/// characters are treated as having an unreadable text layer (issue #1254).
fn default_min_undecodable_ratio() -> f64 {
    0.5
}
/// Provenance-based fabricated-text OCR routing is on by default: it is a
/// direct fact from xberg_native_pdf 0.3.75+, not a heuristic, so false positives are
/// rare (issue #1254).
fn default_enable_provenance_ocr_routing() -> bool {
    true
}
/// Pages at or above this fraction of `MappingProvenance::Fallback` characters
/// are treated as having a fabricated text layer (issue #1254).
fn default_min_provenance_fallback_ratio() -> f64 {
    0.5
}
/// Language/dictionary-plausibility OCR routing is on by default: false positives are rare
/// because the check abstains (`NotEvaluated`) on any page without enough prose to judge, and
/// requires both a low reliable-chunk ratio AND a low mean confidence to flag one (issue
/// #1696).
fn default_enable_plausibility_ocr_routing() -> bool {
    true
}
/// Calibrated in `tools/benchmark-harness/scripts/plausibility_calibrate.py` against the
/// `wrong_mapping` corpus (100% recall target) and the `native-clean`/table/formula cohorts
/// (false-positive ceilings); see that script's own doc comment for the measured numbers
/// (issue #1696).
fn default_min_reliable_language_chunk_ratio() -> f64 {
    0.10
}

/// A single backend stage in the OCR pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OcrPipelineStage {
    /// Backend name: "tesseract", "paddleocr", "paddle-ocr", "sceptre", "vlm", or a custom registered name.
    /// Sceptre uses ONNX Runtime on desktop/server and tract on Android/iOS; browser WebAssembly has a separate
    /// byte-fed engine because the normal async OCR registry assumes native model storage.
    pub backend: String,

    /// Priority weight (higher = tried first). Stages are sorted by priority descending.
    #[serde(default = "default_priority")]
    pub priority: u32,

    /// Language override for this stage (None = use parent OcrConfig.language).
    ///
    /// A list is the canonical form and the only form accepted by the binding
    /// object APIs: `["eng", "deu"]`. When deserializing from a config file,
    /// JSON body, or the REST/MCP API, a single string is also accepted,
    /// either as one code ("eng") or "+"-joined ("eng+deu").
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_languages"
    )]
    pub language: Option<Vec<String>>,

    /// Tesseract-specific config override for this stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tesseract_config: Option<crate::types::TesseractConfig>,

    /// Legacy PaddleOCR config JSON for this stage.
    ///
    /// Deprecated since 1.3.2 and planned for removal in 2.0. Use
    /// [`Self::paddle_ocr_settings`] for typed access.
    #[deprecated(since = "1.3.2", note = "use paddle_ocr_settings; removal planned for 2.0")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paddle_ocr_config: Option<serde_json::Value>,

    /// Typed PaddleOCR config for this stage.
    ///
    /// When both PaddleOCR fields are set, this typed field takes precedence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paddle_ocr_settings: Option<crate::paddle_ocr::PaddleOcrConfig>,

    /// VLM config override for this pipeline stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vlm_config: Option<super::llm::LlmConfig>,

    /// Arbitrary per-call options passed through to the backend unchanged.
    ///
    /// Backends that support runtime tuning (mode switching, preprocessing
    /// flags, inference parameters, etc.) read this value and deserialize
    /// the keys they care about. Keys unknown to the backend are silently
    /// ignored, so options from different backends can coexist in the same
    /// config without conflict.
    ///
    /// Example (custom backend):
    /// ```json
    /// { "mode": "fast", "enable_layout": true }
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_options: Option<serde_json::Value>,
}

/// [`OcrConfig::backend_options`] key carrying the true resolution, in DPI, of the raster in
/// this one call.
///
/// Stamped per page by the PDF OCR route, which is the only caller that can know it: it
/// rendered the page itself and can divide the raster's pixel width by the MediaBox width (see
/// `crate::pdf::render::rendered_page_dpi`). Deliberately absent for callers that genuinely do
/// not know the resolution of the bytes they hold — the standalone image extractor, plugin
/// callers passing arbitrary user images — so those have the resample sized from the text in the
/// raster (`image::preprocessing::unknown_raster_dpi`), and 72 DPI where no text can be measured.
///
/// This is per *page*, not per document: the OCR route clones the config for each page (the
/// same mechanism `page_rotation_degrees` uses), so a document mixing page sizes, or one page
/// whose DPI `choose_safe_dpi` reduced, still reports its own value.
///
/// Declared here rather than as a literal at each end so the producer
/// (`extractors::pdf::ocr::ocr_config_with_page_rotation_hint`) and the consumer
/// (`ocr::tesseract_backend::TesseractBackend::config_to_tesseract`) cannot drift apart.
///
/// Both users are feature-gated (`ocr`/`ocr-pipeline`, and `pdf` for the producer), so builds
/// that compile neither leave this unreferenced rather than unused-by-mistake.
#[allow(dead_code)]
pub(crate) const SOURCE_DPI_BACKEND_OPTION: &str = "source_dpi";

/// [`OcrConfig::backend_options`] key carrying a page's PDF `/Rotate` value, in degrees.
///
/// Stamped per page by the PDF OCR route alongside [`SOURCE_DPI_BACKEND_OPTION`] (see
/// `crate::extractors::pdf::ocr::pipeline::ocr_config_with_page_rotation_hint`). Declared here
/// for the same reason: so the producer and any consumer cannot drift apart on the key's spelling.
#[allow(dead_code)]
pub(crate) const PAGE_ROTATION_DEGREES_BACKEND_OPTION: &str = "page_rotation_degrees";

/// [`OcrConfig::backend_options`] key carrying whether the PDF OCR route's scan-detection density
/// check classified this page as a whole-page raster scan.
///
/// Stamped per page alongside [`SOURCE_DPI_BACKEND_OPTION`] (see
/// `crate::extractors::pdf::ocr::pipeline::ocr_config_with_page_rotation_hint`), which is the only
/// caller that can know it: it already runs `crate::pdf::scan_detect::full_page_raster_density`
/// per page to decide the default Tesseract PSM. `config_to_tesseract` reads this to make a known
/// scan page take Tesseract's default OCR preprocessing (resample, binarisation, deskew)
/// unconditionally, however dark the raster is -- the pixel-brightness heuristic
/// (`should_apply_default_preprocessing`) is tuned for a caller with no other signal and misses a
/// scanned page with shaded rows or a grey background (GH#1894). A bare image handed to the
/// standalone image extractor carries no such signal and keeps the pixel test.
///
/// Declared here rather than as a literal at each end so the producer and the consumer
/// (`ocr::tesseract_backend::TesseractBackend::config_to_tesseract`) cannot drift apart.
#[allow(dead_code)]
pub(crate) const KNOWN_FULL_PAGE_SCAN_BACKEND_OPTION: &str = "known_full_page_scan";

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
pub(crate) fn is_strong_rtl_tesseract_language(language: &str) -> bool {
    language.split('+').any(|language| {
        matches!(
            language.trim().to_ascii_lowercase().as_str(),
            "ara"
                | "aze_ara"
                | "ckb"
                | "div"
                | "fas"
                | "heb"
                | "kir_ara"
                | "pus"
                | "snd"
                | "uig"
                | "urd"
                | "uzb_ara"
                | "yid"
        )
    })
}

fn default_priority() -> u32 {
    100
}

/// Selection policy for a pipeline's best-effort fallback: which stage's result to
/// return when no stage clears `quality_thresholds.pipeline_min_quality`.
///
/// Internal (`pub(crate)`) rather than a field on [`OcrPipelineConfig`]: it is derived
/// from the [`OcrConfig`] that produced the pipeline (see [`OcrConfig::pipeline_selection`]),
/// so keeping it off the binding-facing config avoids a public API break and a full
/// binding regeneration for a behavior-only fix (#1341).
#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum OcrPipelineSelection {
    /// Return the highest-scoring stage result (default).
    ///
    /// Correct for pipelines where every stage is an independent attempt at the
    /// same OCR problem, so the surface quality score is a reasonable proxy for
    /// which result to keep.
    #[default]
    HighestScore,

    /// Return the last stage that produced non-empty text, regardless of score.
    ///
    /// Intended for fallback pipelines (e.g. `vlm_fallback`) where a later stage
    /// only runs because the earlier stage(s) were judged inadequate — so a later
    /// non-empty result is a deliberate override, not noise, even when it scores
    /// lower than an earlier stage's output (#1341). An empty result never
    /// overwrites a prior non-empty one, nor does a non-empty result that is a
    /// materially worse replacement for an already-dense incumbent — a one-sided
    /// quality guard, not a reversion to score-based selection (F46; see
    /// `should_replace_best_effort_result` in `extractors::pdf::ocr`).
    PreferLastNonEmpty,
}

/// Multi-backend OCR pipeline with quality-based fallback.
///
/// Backends are tried in priority order (highest first). After each backend
/// produces output, quality is evaluated. If it meets `quality_thresholds.pipeline_min_quality`,
/// the result is accepted. Otherwise the next backend is tried; if none clears the
/// threshold, an internal selection policy derived from the `OcrConfig` decides which
/// stage's result is returned as the best effort (`vlm_fallback` pipelines prefer their
/// last non-empty stage; explicit and classical pipelines stay score-based).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OcrPipelineConfig {
    /// Ordered list of backends to try. Sorted by priority (descending) at runtime.
    pub stages: Vec<OcrPipelineStage>,

    /// Quality thresholds for deciding whether to accept a result or try the next backend.
    #[serde(default)]
    pub quality_thresholds: OcrQualityThresholds,
}

/// Policy controlling when VLM (Vision Language Model) OCR is used as a fallback.
///
/// This knob is syntactic sugar over the explicit [`OcrPipelineConfig`] stage
/// ordering. When `vlm_fallback` is set and `pipeline` is `None`, an equivalent
/// pipeline is synthesised at extraction time:
///
/// - [`VlmFallbackPolicy::Disabled`] — no synthesis; single-backend mode (default).
/// - [`VlmFallbackPolicy::OnLowQuality`] — tries the classical backend first; if the
///   result scores below `quality_threshold`, tries VLM.
/// - [`VlmFallbackPolicy::Always`] — skips the classical backend and sends every page
///   to the VLM.
///
/// When [`OcrConfig::pipeline`] is explicitly set, `vlm_fallback` is ignored — the
/// explicit pipeline takes precedence.
///
/// # Errors
///
/// Both `OnLowQuality` and `Always` require [`OcrConfig::vlm_config`] to be `Some`.
/// Constructing an [`OcrConfig`] with one of these policies but no `vlm_config` is
/// detected by `OcrConfig::validate` and will surface as a
/// [`crate::XbergError::Validation`] error at extraction time, not a panic.
///
/// # Example
///
/// ```rust
/// use xberg::{OcrConfig, VlmFallbackPolicy, LlmConfig};
///
/// # fn example() -> xberg::Result<()> {
/// let config = OcrConfig {
///     vlm_fallback: VlmFallbackPolicy::OnLowQuality { quality_threshold: 0.6 },
///     vlm_config: Some(LlmConfig {
///         model: "openai/gpt-4o-mini".to_string(),
///         ..Default::default()
///     }),
///     ..Default::default()
/// };
///
/// // Threshold calibration is deferred to the Stage 0 benchmark harness.
/// assert!(matches!(config.vlm_fallback, VlmFallbackPolicy::OnLowQuality { .. }));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum VlmFallbackPolicy {
    /// No VLM fallback (default). Behaves identically to the pre-policy single-backend mode.
    #[default]
    Disabled,

    /// Try the classical OCR backend first. If the quality score is below
    /// `quality_threshold`, send the page to the VLM.
    ///
    /// `quality_threshold` is in the `[0.0, 1.0]` range, but it is **not** the same
    /// quantity reported on [`crate::types::page::PageOcrConfidence::score`] (GH#1584).
    /// The accept decision blends text-shape quality with confidence, weighted 0.7/0.3
    /// (`extractors::pdf::ocr::pipeline_stage_score`) -- when the backend's confidence is on
    /// a known scale it contributes only 30% of the compared score, so a page can clear this
    /// threshold on clean-looking text even while its own `PageOcrConfidence.score` reads
    /// below it. Do not calibrate this value by reading `PageOcrConfidence.score` off a
    /// sample page and expecting an equal `quality_threshold` to reproduce the same
    /// accept/reject outcome. A value of `0.5` is a reasonable starting point; calibrate with
    /// the Stage 0 benchmark harness.
    OnLowQuality {
        /// Minimum acceptable quality score from the classical backend. Pages scoring below
        /// this are retried with VLM -- see this variant's doc comment for what "scoring"
        /// means here.
        quality_threshold: f64,
    },

    /// Skip the classical OCR backend entirely. Every page is sent to the VLM.
    Always,
}

#[derive(Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
enum VlmFallbackPolicyWire {
    Disabled {},
    OnLowQuality { quality_threshold: f64 },
    Always {},
}

impl<'de> Deserialize<'de> for VlmFallbackPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(match VlmFallbackPolicyWire::deserialize(deserializer)? {
            VlmFallbackPolicyWire::Disabled {} => Self::Disabled,
            VlmFallbackPolicyWire::OnLowQuality { quality_threshold } => Self::OnLowQuality { quality_threshold },
            VlmFallbackPolicyWire::Always {} => Self::Always,
        })
    }
}

/// Default confidence a page must reach before [`OcrStrategy::ScannedPages`] OCRs it.
///
/// A slide with a full-bleed background image scores `0.50`, so a threshold of
/// `0.50` or lower also sends such slides to OCR.
pub const DEFAULT_SCANNED_MIN_CONFIDENCE: f64 = 0.70;

/// Which pages of a PDF get OCR'd when neither `force_ocr` nor `force_ocr_pages` applies.
///
/// # Examples
///
/// ```
/// use xberg::{ExtractionConfig, OcrStrategy};
///
/// // OCR pages that look like scans; keep native text everywhere else.
/// let config = ExtractionConfig {
///     ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.7 },
///     ..Default::default()
/// };
/// assert!(matches!(config.ocr_strategy, OcrStrategy::ScannedPages { .. }));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum OcrStrategy {
    /// OCR only when the native text layer fails a quality check (default).
    ///
    /// A scanner's invisible OCR sidecar passes that check, so scanned pages
    /// carrying one are extracted natively. Use [`OcrStrategy::ScannedPages`]
    /// to OCR them instead.
    #[default]
    Auto,

    /// Additionally OCR every page that looks like a scan.
    ///
    /// Pages are graded on raster coverage, whether the text layer is invisible
    /// or absent, the image codec, and the producer. Pages at or above
    /// `min_confidence` are OCR'd; the rest keep native text and still go through
    /// the `Auto` quality check.
    ///
    /// Detects that a text layer came from a scanner, not whether it is accurate,
    /// so a page carrying a good sidecar is OCR'd too.
    ScannedPages {
        /// Minimum scan confidence, in `[0.0, 1.0]`. Configuration validation rejects
        /// non-finite or out-of-range values. See [`DEFAULT_SCANNED_MIN_CONFIDENCE`]
        /// for how to pick one.
        min_confidence: f64,
    },
}

#[derive(Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
enum OcrStrategyWire {
    Auto {},
    ScannedPages { min_confidence: f64 },
}

impl<'de> Deserialize<'de> for OcrStrategy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(match OcrStrategyWire::deserialize(deserializer)? {
            OcrStrategyWire::Auto {} => Self::Auto,
            OcrStrategyWire::ScannedPages { min_confidence } => Self::ScannedPages { min_confidence },
        })
    }
}

impl OcrStrategy {
    pub(crate) fn validate(&self) -> Result<(), XbergError> {
        if let Self::ScannedPages { min_confidence } = self {
            validate_quality_value("ocr_strategy", "min_confidence", *min_confidence, 0.0, 1.0)?;
        }
        Ok(())
    }

    /// Confidence a page must reach to count as a scan.
    ///
    /// [`OcrStrategy::Auto`] uses the default: it does not select pages by scan
    /// confidence, but `scanned_pages` metadata still needs a threshold. Finite direct
    /// values are defensively clamped to `[0.0, 1.0]`; non-finite values use the default.
    #[must_use]
    pub fn effective_min_confidence(&self) -> f64 {
        match self {
            Self::Auto => DEFAULT_SCANNED_MIN_CONFIDENCE,
            Self::ScannedPages { min_confidence } if min_confidence.is_finite() => min_confidence.clamp(0.0, 1.0),
            Self::ScannedPages { .. } => DEFAULT_SCANNED_MIN_CONFIDENCE,
        }
    }
}

/// OCR configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OcrConfig {
    /// Whether OCR is enabled.
    ///
    /// Setting `enabled: false` is a shorthand for `disable_ocr: true` on the parent
    /// [`ExtractionConfig`](crate::core::config::ExtractionConfig). Images return
    /// metadata only; PDFs use native text extraction without OCR fallback.
    ///
    /// Defaults to `true`. When `false`, all other OCR settings are ignored.
    #[serde(default = "default_ocr_enabled")]
    pub enabled: bool,

    /// OCR backend: tesseract, paddleocr, paddle-ocr, sceptre, or vlm.
    /// Sceptre uses ONNX Runtime on desktop/server and tract on supported mobile builds.
    /// Browser WebAssembly uses the separate byte-fed Sceptre worker API.
    #[serde(default = "default_tesseract_backend")]
    pub backend: String,

    /// Language code(s) for OCR recognition. Defaults to `["eng"]`. For Tesseract,
    /// languages are joined with "+".
    ///
    /// A list is the canonical form and the only form accepted by the binding
    /// object APIs (Python, Node, PHP, WASM, etc.): `["eng", "deu"]`. When
    /// deserializing from a config file, JSON body, or the REST/MCP API, a
    /// single string is also accepted, either as one code ("eng") or
    /// "+"-joined ("eng+deu").
    ///
    /// The four candle-based backends also use this list to decide which scripts are
    /// plausible in their output, dropping a line written in an unconfigured script.
    #[serde(default = "default_eng", deserialize_with = "deserialize_languages")]
    pub language: Vec<String>,

    /// Tesseract-specific configuration (optional)
    #[serde(default)]
    pub tesseract_config: Option<crate::types::TesseractConfig>,

    /// Output format for OCR results (optional, for format conversion)
    #[serde(default)]
    pub output_format: Option<OutputFormat>,

    /// Legacy PaddleOCR configuration JSON.
    ///
    /// Deprecated since 1.3.2 and planned for removal in 2.0. Use
    /// [`Self::paddle_ocr_settings`] for typed access.
    #[deprecated(since = "1.3.2", note = "use paddle_ocr_settings; removal planned for 2.0")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paddle_ocr_config: Option<serde_json::Value>,

    /// Typed PaddleOCR-specific configuration (optional).
    ///
    /// Any `PaddleOcrConfig` field can be set here; unset fields keep their defaults. In a config
    /// file, a JSON config, or a REST or MCP request body, an unknown key is a configuration error.
    /// A typed object built in a binding has only the fields it declares, so it ignores any other
    /// property, as every other typed config does. The most common overrides are `model_version`
    /// (`"pp-ocrv6"` default / `"pp-ocrv5"`) and `model_tier`. In TOML:
    ///
    /// ```toml
    /// [ocr.paddle_ocr_settings]
    /// model_version = "pp-ocrv5"
    /// model_tier = "server"
    /// ```
    ///
    /// The `XBERG_OCR_MODEL_VERSION` / `XBERG_OCR_MODEL_TIER` environment variables set the same two
    /// keys for env-configured servers (issue #1279).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paddle_ocr_settings: Option<crate::paddle_ocr::PaddleOcrConfig>,

    /// Arbitrary per-call options passed through to the backend unchanged.
    ///
    /// Custom OCR backends and built-in backends that support runtime tuning
    /// can read this value and deserialize the keys they care about. Keys
    /// unknown to the backend are silently ignored.
    ///
    /// This is the recommended extension point for per-call parameters that
    /// are not covered by the typed fields above (e.g. mode switching,
    /// preprocessing flags, inference batch size).
    ///
    /// **Scope:** when `pipeline` is `None`, this value is propagated to the
    /// primary stage of the auto-constructed pipeline. When `pipeline` is
    /// explicitly set, this field has **no effect** — the caller must set
    /// `OcrPipelineStage.backend_options` directly on the relevant stage(s)
    /// instead.
    ///
    /// Example:
    /// ```json
    /// { "mode": "fast", "enable_layout": true, "timeout_ms": 5000 }
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_options: Option<serde_json::Value>,

    /// OCR element extraction configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element_config: Option<OcrElementConfig>,

    /// Quality thresholds for the native-text-to-OCR fallback decision.
    /// When None, uses compiled defaults (matching previous hardcoded behavior).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_thresholds: Option<OcrQualityThresholds>,

    /// Multi-backend OCR pipeline configuration. When set, enables weighted
    /// fallback across multiple OCR backends based on output quality.
    /// When None, uses the single `backend` field (same as today).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pipeline: Option<OcrPipelineConfig>,

    /// Enable automatic page rotation based on orientation detection.
    ///
    /// When enabled, page orientation (0/90/180/270 degrees) is detected with an
    /// ONNX PP-LCNet document-orientation classifier — NOT Tesseract's own
    /// `DetectOrientationScript()`/OSD, which this crate does not call. If the
    /// page is rotated with high confidence, the image is corrected before
    /// recognition. Applies to every OCR backend; the tesseract-only
    /// `TesseractConfig::preprocessing` field of the same name is OR'd in on
    /// top of this one and affects tesseract alone.
    ///
    /// Independent of a page's PDF `/Rotate` entry, which is handled separately
    /// on every OCR route regardless of this setting.
    #[serde(default)]
    pub auto_rotate: bool,

    /// Ergonomic VLM fallback policy.
    ///
    /// When set to anything other than [`VlmFallbackPolicy::Disabled`] and
    /// [`OcrConfig::pipeline`] is `None`, a multi-stage pipeline is synthesised
    /// automatically:
    ///
    /// - [`VlmFallbackPolicy::OnLowQuality`] → `[classical_stage, vlm_stage]` with the
    ///   `quality_threshold` mapped onto [`OcrQualityThresholds::pipeline_min_quality`].
    /// - [`VlmFallbackPolicy::Always`] → `[vlm_stage]` only.
    ///
    /// Requires [`OcrConfig::vlm_config`] to be `Some` when not `Disabled`.
    /// When [`OcrConfig::pipeline`] is explicitly set, this field is ignored.
    #[serde(default)]
    pub vlm_fallback: VlmFallbackPolicy,

    /// VLM (Vision Language Model) OCR configuration.
    ///
    /// Required when `backend` is `"vlm"` or when `vlm_fallback` is not
    /// [`VlmFallbackPolicy::Disabled`]. Uses liter-llm to send page images to a
    /// vision model for text extraction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vlm_config: Option<super::llm::LlmConfig>,

    /// Custom Jinja2 prompt template for VLM OCR.
    ///
    /// When `None`, uses the default template. Available variables:
    /// - `{{ language }}` — The document language code (e.g., "eng", "deu").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vlm_prompt: Option<String>,

    /// Hardware acceleration for ONNX Runtime models (e.g. PaddleOCR, layout detection).
    ///
    /// Not user-configurable via config files — injected at runtime from
    /// `ExtractionConfig::acceleration` before each `process_image` call.
    #[serde(skip)]
    pub acceleration: Option<super::acceleration::AccelerationConfig>,

    /// Security limits applied when decoding raw image bytes for OCR (GH#1554).
    ///
    /// Not user-configurable via config files — injected at runtime from
    /// `ExtractionConfig::security_limits` before each `process_image` call, the same
    /// pattern [`Self::acceleration`] uses. `ExtractionConfig::security_limits` remains the
    /// source of truth and overwrites this field whenever it carries a value; a value set
    /// here directly is honoured only when `ExtractionConfig` carries none, so the two
    /// cannot drift while a directly-set limit is no longer silently discarded (GH#1651).
    /// A backend consulting a
    /// `backend_options` override for this call may still let that override win, but in
    /// the absence of one this field is what backends should fall back to instead of
    /// `SecurityLimits::default()`. `None` means "use `SecurityLimits::default()`", never
    /// "disable the check". ~keep
    #[serde(skip)]
    pub security_limits: Option<crate::extractors::security::SecurityLimits>,

    /// Caller-supplied Tesseract `traineddata` bytes per language code.
    ///
    /// Primary use case is the WASM build, which has no filesystem and cannot
    /// download tessdata at runtime. Native builds typically rely on
    /// `TessdataManager` and ignore this field. When present, the WASM
    /// Tesseract backend prefers these bytes over its compile-time-bundled
    /// English data.
    ///
    /// Skipped by serde to keep config files small — supply via the typed API
    /// at runtime.
    #[serde(skip)]
    pub tessdata_bytes: Option<std::collections::HashMap<String, Vec<u8>>>,

    /// Runtime override for tessdata directory path.
    ///
    /// When set, uses this path as the highest-priority tessdata location,
    /// bypassing environment variables and cache directories. Useful for
    /// embedding pre-installed tessdata in applications. When `None`, uses
    /// the standard resolution chain: TESSDATA_PREFIX env, cache dir, system paths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tessdata_path: Option<PathBuf>,

    /// Repair OCR tokens that are clearly numeric but mis-punctuated: a dropped thousands
    /// separator, a decimal point misread for a grouping comma, or one number split into two
    /// tokens at a rendering gap (GH#1789).
    ///
    /// Defaults to `false`. Unlike the always-on list-marker repair
    /// (`crate::extractors::pdf::ocr::scoring::repair_ocr_list_markers`), this repair has no
    /// table-column context available at the point OCR text comes back as a flat string, so it
    /// cannot tell `"1.234,56"` (European) from `"1,234.56"` (US) apart on its own -- it always
    /// assumes the US/UK convention (comma groups, period decimals).
    ///
    /// The rules read only the punctuation and digit counts, never whether a token is an amount,
    /// so each of these is rewritten too (GH#1836). Enable it only for documents whose numbers are
    /// amounts, and which use that convention:
    ///
    /// - A bare 4-9 digit integer becomes grouped, whatever it denotes: `"Year 2019"` ->
    ///   `"Year 2,019"`, and likewise a ZIP code, a page number or a part number. Only a literal
    ///   `"FY "` prefix is exempt.
    /// - A US-convention decimal with exactly three fraction digits and at most three integer
    ///   digits becomes an amount: `"0.125"` -> `"0,125"`. Four fraction digits (`"0.7906"`) or a
    ///   trailing `%` are left alone.
    /// - A lone digit followed by a single space and an already-grouped number is joined, which is
    ///   indistinguishable from two adjacent table cells: `"5 12,000"` -> `"512,000"`.
    ///
    /// The repair is skipped entirely when `tesseract_config.output_format` is `"hocr"` or
    /// `"tsv"`, because it would re-punctuate the bare coordinate integers in that markup.
    #[serde(default)]
    pub numeric_repair: bool,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: default_tesseract_backend(),
            language: vec!["eng".to_string()],
            tesseract_config: None,
            output_format: None,
            paddle_ocr_config: None,
            paddle_ocr_settings: None,
            backend_options: None,
            element_config: None,
            quality_thresholds: None,
            pipeline: None,
            auto_rotate: false,
            vlm_fallback: VlmFallbackPolicy::Disabled,
            vlm_config: None,
            vlm_prompt: None,
            acceleration: None,
            security_limits: None,
            tessdata_bytes: None,
            tessdata_path: None,
            numeric_repair: false,
        }
    }
}

impl OcrConfig {
    #[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
    pub(crate) fn select_public_elements(
        &self,
        elements: Option<Vec<crate::types::OcrElement>>,
    ) -> Option<Vec<crate::types::OcrElement>> {
        let element_config = self.element_config.as_ref()?;
        let selected = element_config.select_elements(elements.as_deref().unwrap_or_default());
        (!selected.is_empty()).then_some(selected)
    }

    #[cfg(any(feature = "ocr", all(feature = "pdf", feature = "ocr-pipeline")))]
    pub(crate) fn apply_public_element_policy(&self, document: &mut crate::types::ExtractedDocument) {
        document.ocr_elements = self.select_public_elements(document.ocr_elements.take());
    }

    fn validate_quality_thresholds(&self) -> Result<(), XbergError> {
        if let Some(thresholds) = self.quality_thresholds.as_ref() {
            thresholds.validate("ocr.quality_thresholds")?;
        }
        if let Some(pipeline) = self.pipeline.as_ref() {
            pipeline
                .quality_thresholds
                .validate("ocr.pipeline.quality_thresholds")?;
        }
        Ok(())
    }

    /// Validates that the configured backend is supported.
    ///
    /// This method checks that the backend name is one of the supported OCR backends:
    /// - tesseract
    /// - paddleocr
    /// - paddle-ocr
    /// - sceptre
    /// - vlm
    ///
    /// Typos in backend names are caught at configuration validation time, not at runtime.
    /// Also validates pipeline stage backends when a pipeline is configured.
    ///
    /// Also validates, for the backend and each pipeline stage, the options only that backend
    /// parses: a compiled candle backend's `backend_options` and the legacy PaddleOCR JSON.
    ///
    /// Also validates every non-blank entry of `language` (and, per pipeline stage, its
    /// `language` override) as an ISO 639 code, and the `vlm_fallback` quality threshold as a
    /// `[0.0, 1.0]` confidence value. OCR quality thresholds are validated against their
    /// documented numeric domains. Blank language entries are tolerated here (they are
    /// filtered out by [`Self::effective_languages`] and fall back to the default language),
    /// but a non-blank, unrecognized code is rejected.
    ///
    /// When `vlm_fallback` is not `Disabled` and no explicit `pipeline` is set,
    /// `vlm_config` must be `Some`. A missing `vlm_config` in that case is a
    /// configuration error detected here, not at runtime.
    pub(crate) fn validate(&self) -> Result<(), XbergError> {
        validate_ocr_backend(&self.backend)?;
        crate::core::config_validation::validate_vlm_backend_config(&self.backend, self.vlm_config.as_ref())?;
        validate_languages(&self.language)?;
        if let VlmFallbackPolicy::OnLowQuality { quality_threshold } = &self.vlm_fallback {
            crate::core::config_validation::validate_confidence(*quality_threshold)?;
        }
        if let Some(element_config) = self.element_config.as_ref()
            && (!element_config.min_confidence.is_finite() || !(0.0..=1.0).contains(&element_config.min_confidence))
        {
            return Err(XbergError::validation(format!(
                "ocr.element_config.min_confidence must be a finite value between 0.0 and 1.0, got {}",
                element_config.min_confidence
            )));
        }
        self.validate_quality_thresholds()?;
        validate_tesseract_tuning(self.tesseract_config.as_ref())?;
        validate_backend_owned_options(
            &self.backend,
            self.backend_options.as_ref(),
            self.paddle_ocr_config.as_ref(),
            self.paddle_ocr_settings.as_ref(),
        )?;
        if let Some(ref pipeline) = self.pipeline {
            for stage in &pipeline.stages {
                validate_ocr_backend(&stage.backend)?;
                crate::core::config_validation::validate_vlm_backend_config(&stage.backend, stage.vlm_config.as_ref())?;
                if let Some(ref languages) = stage.language {
                    validate_languages(languages)?;
                }
                validate_tesseract_tuning(stage.tesseract_config.as_ref())?;
                validate_backend_owned_options(
                    &stage.backend,
                    stage.backend_options.as_ref(),
                    stage.paddle_ocr_config.as_ref(),
                    stage.paddle_ocr_settings.as_ref(),
                )?;
            }
        } else if self.vlm_fallback != VlmFallbackPolicy::Disabled && self.vlm_config.is_none() {
            return Err(XbergError::validation(
                "vlm_fallback is set but vlm_config is missing; \
                 provide an LlmConfig with the model and API key"
                    .to_string(),
            ));
        }
        Ok(())
    }

    /// Effective OCR languages, with the documented default applied.
    ///
    /// Blank entries are dropped and an empty list falls back to
    /// [`DEFAULT_OCR_LANGUAGE`]. OCR backends call this instead of reading
    /// [`language`](Self::language) directly so an unset language never reaches
    /// an engine as an empty language string — which Tesseract would otherwise
    /// try to load as a language pack named `""`, surfacing as a confusing
    /// "Failed to download language pack ''" error.
    #[cfg(any(
        feature = "ocr",
        feature = "ocr-wasm",
        feature = "ocr-pipeline",
        paddle_ocr,
        all(feature = "liter-llm", not(target_arch = "wasm32")),
    ))]
    pub(crate) fn effective_languages(&self) -> Vec<String> {
        let langs: Vec<String> = self
            .language
            .iter()
            .map(|lang| lang.trim())
            .filter(|lang| !lang.is_empty())
            .map(str::to_string)
            .collect();
        if langs.is_empty() {
            vec![DEFAULT_OCR_LANGUAGE.to_string()]
        } else {
            langs
        }
    }

    /// Resolve the language Tesseract should use, reconciling [`Self::language`] with
    /// `tesseract_config.language` (#1572).
    ///
    /// Both fields default to `["eng"]`, so neither can represent "unset" on its own. An
    /// explicit (non-default) [`Self::language`] always wins, matching the convention already
    /// used to decide whether to propagate a language into a synthesised pipeline stage (see
    /// [`Self::effective_pipeline`]). Otherwise a non-empty `tesseract_config.language` wins,
    /// so a caller who only configures `TesseractConfig` still gets their language. Both
    /// native (`ocr::tesseract_backend`) and WASM (`ocr::tesseract_wasm_backend`) Tesseract
    /// backends call this so they agree on which field wins.
    #[cfg(any(
        feature = "ocr",
        feature = "ocr-wasm",
        feature = "ocr-pipeline",
        paddle_ocr,
        all(feature = "liter-llm", not(target_arch = "wasm32")),
    ))]
    pub(crate) fn effective_tesseract_language(&self) -> Vec<String> {
        if self.language != [DEFAULT_OCR_LANGUAGE.to_string()] {
            return self.effective_languages();
        }
        match &self.tesseract_config {
            Some(tess) if !tess.language.is_empty() => tess.language.clone(),
            _ => self.effective_languages(),
        }
    }

    /// Returns the effective quality thresholds, using configured values or defaults.
    #[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
    pub(crate) fn effective_thresholds(&self) -> OcrQualityThresholds {
        self.quality_thresholds.clone().unwrap_or_default()
    }

    /// Returns the effective pipeline config.
    ///
    /// Priority order:
    /// 1. If `pipeline` is explicitly set, return it unchanged.
    /// 2. If `vlm_fallback` is `OnLowQuality` or `Always` (and `pipeline` is
    ///    `None`), synthesise a pipeline from the policy:
    ///    - `OnLowQuality { quality_threshold }` → `[classical_stage @ 100, vlm_stage @ 50]`
    ///      with `quality_thresholds.pipeline_min_quality = quality_threshold`.
    ///    - `Always` → `[vlm_stage @ 100]` only (no classical stage).
    ///      Returns `None` if `vlm_config` is not set (misconfiguration; surfaces at
    ///      call-time as a logged warning rather than a panic — [`validate`] catches it
    ///      at config-load time).
    /// 3. If `paddle-ocr` is compiled in and the backend is the default (tesseract),
    ///    auto-constructs `[tesseract @ 100, paddleocr @ 50]`.
    /// 4. Otherwise returns `None` (single-backend mode).
    ///
    /// Explicit non-default backend selections are honored as-is — a silent
    /// paddleocr fallback would mask errors from the chosen backend.
    #[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
    pub(crate) fn effective_pipeline(&self) -> Option<OcrPipelineConfig> {
        if self.pipeline.is_some() {
            return self.pipeline.clone();
        }

        match &self.vlm_fallback {
            VlmFallbackPolicy::OnLowQuality { quality_threshold } => {
                let Some(vlm_cfg) = self.vlm_config.clone() else {
                    tracing::warn!(
                        "vlm_fallback=OnLowQuality is set but vlm_config is missing; \
                         falling through to single-backend mode"
                    );
                    return self.effective_pipeline_classical();
                };
                let mut thresholds = self.effective_thresholds();
                thresholds.pipeline_min_quality = *quality_threshold;
                let classical_stage = OcrPipelineStage {
                    backend: self.backend.clone(),
                    priority: 100,
                    language: if self.language.len() == 1 && self.language[0] == "eng" {
                        None
                    } else {
                        Some(self.language.clone())
                    },
                    tesseract_config: self.tesseract_config.clone(),
                    paddle_ocr_config: None,
                    paddle_ocr_settings: None,
                    vlm_config: None,
                    backend_options: self.backend_options.clone(),
                };
                let vlm_stage = OcrPipelineStage {
                    backend: "vlm".to_string(),
                    priority: 50,
                    language: None,
                    tesseract_config: None,
                    paddle_ocr_config: None,
                    paddle_ocr_settings: None,
                    vlm_config: Some(vlm_cfg),
                    backend_options: None,
                };
                return Some(OcrPipelineConfig {
                    stages: vec![classical_stage, vlm_stage],
                    quality_thresholds: thresholds,
                });
            }
            VlmFallbackPolicy::Always => {
                let Some(vlm_cfg) = self.vlm_config.clone() else {
                    tracing::warn!(
                        "vlm_fallback=Always is set but vlm_config is missing; \
                         falling through to single-backend mode"
                    );
                    return self.effective_pipeline_classical();
                };
                let vlm_stage = OcrPipelineStage {
                    backend: "vlm".to_string(),
                    priority: 100,
                    language: None,
                    tesseract_config: None,
                    paddle_ocr_config: None,
                    paddle_ocr_settings: None,
                    vlm_config: Some(vlm_cfg),
                    backend_options: None,
                };
                return Some(OcrPipelineConfig {
                    stages: vec![vlm_stage],
                    quality_thresholds: self.effective_thresholds(),
                });
            }
            VlmFallbackPolicy::Disabled => {}
        }

        self.effective_pipeline_classical()
    }

    /// Classical pipeline synthesis: paddle-ocr auto-fallback or `None`.
    ///
    /// Extracted so the vlm_fallback paths can fall through cleanly without
    /// duplicating the paddle-ocr conditional compilation block.
    #[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
    fn effective_pipeline_classical(&self) -> Option<OcrPipelineConfig> {
        #[cfg(paddle_ocr)]
        {
            if self.backend != default_tesseract_backend() {
                return None;
            }

            let stages = vec![
                OcrPipelineStage {
                    backend: self.backend.clone(),
                    priority: 100,
                    language: if self.language.len() == 1 && self.language[0] == "eng" {
                        None
                    } else {
                        Some(self.language.clone())
                    },
                    tesseract_config: self.tesseract_config.clone(),
                    paddle_ocr_config: None,
                    paddle_ocr_settings: None,
                    vlm_config: self.vlm_config.clone(),
                    backend_options: self.backend_options.clone(),
                },
                OcrPipelineStage {
                    backend: "paddleocr".to_string(),
                    priority: 50,
                    language: None,
                    tesseract_config: None,
                    paddle_ocr_config: self.paddle_ocr_config.clone(),
                    paddle_ocr_settings: self.paddle_ocr_settings.clone(),
                    vlm_config: None,
                    backend_options: None,
                },
            ];
            Some(OcrPipelineConfig {
                stages,
                quality_thresholds: self.effective_thresholds(),
            })
        }

        #[cfg(not(paddle_ocr))]
        {
            None
        }
    }

    /// Best-effort selection policy for the pipeline this config produces.
    ///
    /// A `vlm_fallback` policy synthesises a fallback pipeline whose later VLM stage
    /// is a deliberate override of an inadequate earlier stage, so its best-effort
    /// result should be the last non-empty stage rather than the highest-scoring one
    /// (#1341). Explicit pipelines and the classical paddle auto-fallback keep the
    /// score-based selection. Mirrors the branch structure of [`Self::effective_pipeline`].
    #[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
    pub(crate) fn pipeline_selection(&self) -> OcrPipelineSelection {
        // Explicit pipelines keep score-based selection regardless of vlm_fallback
        // (they take precedence in `effective_pipeline`).
        if self.pipeline.is_some() {
            return OcrPipelineSelection::HighestScore;
        }
        // A synthesised vlm_fallback pipeline only adds its VLM stage when `vlm_config`
        // is present; without it, `effective_pipeline` falls through to the classical
        // (score-based) pipeline, so match that outcome here.
        if self.vlm_fallback != VlmFallbackPolicy::Disabled && self.vlm_config.is_some() {
            OcrPipelineSelection::PreferLastNonEmpty
        } else {
            OcrPipelineSelection::HighestScore
        }
    }
}

/// Resolve the additive typed PaddleOCR field before the legacy JSON field so callers can migrate
/// without changing the old field's public type. ~keep
pub(crate) fn resolve_paddle_ocr_settings(
    paddle_ocr_settings: Option<&crate::paddle_ocr::PaddleOcrConfig>,
    paddle_ocr_config: Option<&serde_json::Value>,
) -> Result<Option<crate::paddle_ocr::PaddleOcrConfig>, XbergError> {
    if let Some(settings) = paddle_ocr_settings {
        return Ok(Some(settings.clone()));
    }
    paddle_ocr_config
        .map(|value| {
            crate::paddle_ocr::PaddleOcrConfig::from_legacy_value(value).map_err(|error| XbergError::Validation {
                message: format!("Failed to deserialize paddle_ocr_config: {error}"),
                source: None,
            })
        })
        .transpose()
}

/// Validate each non-blank entry of a configured language list as an ISO 639 code.
///
/// Blank entries are skipped rather than rejected: [`OcrConfig::effective_languages`] already
/// filters them out and falls back to the default language, so an empty string is not itself a
/// configuration error — but a non-blank, unrecognized code (e.g. a typo) is.
fn validate_languages(languages: &[String]) -> Result<(), XbergError> {
    for language in languages
        .iter()
        .map(|language| language.trim())
        .filter(|l| !l.is_empty())
    {
        crate::core::config_validation::validate_language_code(language)?;
    }
    Ok(())
}

/// Validate the Tesseract tuning knobs that are plain integers and strings rather than enums.
///
/// `psm`, `oem`, `thresholding_method` and `binarization_method` are the only OCR fields whose
/// type does not already constrain them at deserialization time, so an out-of-range mode
/// reaches the backend and fails there — far from the config that caused it. Validating here
/// keeps the error next to the input.
fn validate_tesseract_tuning(tesseract_config: Option<&crate::types::TesseractConfig>) -> Result<(), XbergError> {
    let Some(tesseract_config) = tesseract_config else {
        return Ok(());
    };
    if let Some(psm) = tesseract_config.psm {
        crate::core::config_validation::validate_tesseract_psm(psm)?;
    }
    crate::core::config_validation::validate_tesseract_oem(tesseract_config.oem)?;
    crate::core::config_validation::validate_tesseract_thresholding_method(tesseract_config.thresholding_method)?;
    if let Some(ref preprocessing) = tesseract_config.preprocessing {
        crate::core::config_validation::validate_image_preprocessing_config(preprocessing)?;
    }
    Ok(())
}

/// Validate the settings that only a backend parses: a candle backend's `backend_options`.
///
/// This runs the same check the backend runs on each page, so an invalid value fails the
/// extraction before any page runs. The automatic OCR route keeps native text when a page
/// fails, so a value first rejected on a page would surface only as a warning. ~keep
#[cfg_attr(
    not(all(
        paddle_ocr,
        any(
            feature = "candle-trocr",
            feature = "candle-paddleocr-vl",
            all(
                not(target_arch = "wasm32"),
                any(feature = "candle-glm-ocr", feature = "candle-deepseek-ocr")
            )
        )
    )),
    allow(unused_variables)
)]
fn validate_backend_owned_options(
    backend: &str,
    backend_options: Option<&serde_json::Value>,
    paddle_ocr_config: Option<&serde_json::Value>,
    paddle_ocr_settings: Option<&crate::paddle_ocr::PaddleOcrConfig>,
) -> Result<(), XbergError> {
    #[cfg(any(
        feature = "candle-trocr",
        feature = "candle-paddleocr-vl",
        all(
            not(target_arch = "wasm32"),
            any(feature = "candle-glm-ocr", feature = "candle-deepseek-ocr")
        )
    ))]
    crate::candle_ocr::validate_backend_options(backend, backend_options)?;
    #[cfg(paddle_ocr)]
    resolve_paddle_ocr_settings(paddle_ocr_settings, paddle_ocr_config)?;
    Ok(())
}

fn default_ocr_enabled() -> bool {
    true
}

fn default_tesseract_backend() -> String {
    "tesseract".to_string()
}

/// Default OCR language (Tesseract/ISO 639 naming): English.
///
/// Single source of truth for the empty-language fallback shared by every OCR
/// backend. Use [`OcrConfig::effective_languages`] rather than this constant
/// directly when a defaulted language list is needed.
pub(crate) const DEFAULT_OCR_LANGUAGE: &str = "eng";

fn default_eng() -> Vec<String> {
    vec![DEFAULT_OCR_LANGUAGE.to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tesseract_config_with(psm: i32, oem: i32) -> crate::types::TesseractConfig {
        crate::types::TesseractConfig {
            psm: Some(psm),
            oem,
            ..Default::default()
        }
    }

    #[test]
    fn should_accept_ocr_config_when_tesseract_psm_and_oem_are_in_range() {
        let config = OcrConfig {
            tesseract_config: Some(tesseract_config_with(6, 1)),
            ..Default::default()
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn should_accept_ocr_config_when_tesseract_psm_is_unset() {
        let config = OcrConfig {
            tesseract_config: Some(crate::types::TesseractConfig {
                psm: None,
                oem: 1,
                ..Default::default()
            }),
            ..Default::default()
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn should_reject_ocr_config_when_tesseract_psm_is_above_range() {
        let config = OcrConfig {
            tesseract_config: Some(tesseract_config_with(14, 1)),
            ..Default::default()
        };

        let message = config
            .validate()
            .expect_err("psm 14 is out of the 1-13 range")
            .to_string();
        assert!(
            message.contains("PSM"),
            "error should name the PSM field; got: {message}"
        );
    }

    /// GH#1586: PSM 0 is `PSM_OSD_ONLY`, so Tesseract recognises no characters and the
    /// extraction completed successfully with an empty document. Rejecting it here is the
    /// only place a caller finds out before losing the content. ~keep
    #[test]
    fn should_reject_ocr_config_when_tesseract_psm_is_osd_only() {
        let config = OcrConfig {
            tesseract_config: Some(tesseract_config_with(0, 1)),
            ..Default::default()
        };

        let message = config
            .validate()
            .expect_err("psm 0 recognises no text and must not be accepted")
            .to_string();
        assert!(
            message.contains("PSM 0") || message.contains("PSM value '0'"),
            "error should name PSM 0 specifically; got: {message}"
        );
        assert!(
            message.contains("orientation"),
            "error should explain that PSM 0 is orientation detection only; got: {message}"
        );
    }

    #[test]
    fn should_reject_ocr_config_when_tesseract_oem_is_above_range() {
        let config = OcrConfig {
            tesseract_config: Some(tesseract_config_with(6, 4)),
            ..Default::default()
        };

        let message = config
            .validate()
            .expect_err("oem 4 is out of the 0-3 range")
            .to_string();
        assert!(
            message.contains("OEM"),
            "error should name the OEM field; got: {message}"
        );
    }

    #[test]
    fn should_reject_ocr_config_when_binarization_method_is_unknown() {
        let mut tesseract_config = tesseract_config_with(6, 1);
        tesseract_config.preprocessing = Some(crate::types::ImagePreprocessingConfig {
            binarization_method: "not-a-method".to_string(),
            ..Default::default()
        });
        let config = OcrConfig {
            tesseract_config: Some(tesseract_config),
            ..Default::default()
        };

        let message = config
            .validate()
            .expect_err("an unknown binarization method must be rejected")
            .to_string();
        assert!(
            message.contains("binarization"),
            "error should name the binarization method; got: {message}"
        );
    }

    #[test]
    fn should_reject_ocr_config_when_deskew_has_no_binarization() {
        let mut tesseract_config = tesseract_config_with(6, 1);
        tesseract_config.preprocessing = Some(crate::types::ImagePreprocessingConfig {
            deskew: true,
            binarization_method: "none".to_string(),
            ..Default::default()
        });
        let config = OcrConfig {
            tesseract_config: Some(tesseract_config),
            ..Default::default()
        };

        let message = config
            .validate()
            .expect_err("grayscale-preserving deskew is unavailable")
            .to_string();

        assert_eq!(
            message,
            "Validation error: deskew must be false when binarization_method is none or off"
        );
    }

    #[test]
    fn test_ocr_config_default() {
        let config = OcrConfig::default();
        assert_eq!(config.backend, "tesseract");
        assert_eq!(config.language, vec!["eng".to_string()]);
        assert!(config.tesseract_config.is_none());
        assert!(config.output_format.is_none());
    }

    #[test]
    fn test_ocr_config_with_tesseract() {
        let config = OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["fra".to_string()],
            ..Default::default()
        };
        assert_eq!(config.backend, "tesseract");
        assert_eq!(config.language, vec!["fra".to_string()]);
    }

    #[test]
    fn test_ocr_config_with_multiple_languages() {
        let config = OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string(), "deu".to_string()],
            ..Default::default()
        };
        assert_eq!(config.backend, "tesseract");
        assert_eq!(config.language, vec!["eng".to_string(), "deu".to_string()]);
    }

    /// GH#1621: `fao` (Faroese) is a real Tesseract-supported ISO 639-3 code, but
    /// `OcrConfig::validate` rejected it because the general validator's allowlist
    /// (`core::config_validation::sections::VALID_LANGUAGE_CODES`) omitted it while the
    /// Tesseract-specific list (`ocr::validation::TESSERACT_SUPPORTED_LANGUAGE_CODES`)
    /// already carried it. ~keep
    #[test]
    fn should_accept_fao_for_tesseract_backend_regression_gh_1621() {
        let config = OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["fao".to_string()],
            ..Default::default()
        };
        let result = config.validate();
        assert!(
            result.is_ok(),
            "expected 'fao' to be accepted as a valid Tesseract language code, got {:?}",
            result
        );
    }

    #[test]
    fn test_language_deserialization_single_string() {
        let json = r#"{"language": "eng"}"#;
        let config: OcrConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.language, vec!["eng".to_string()]);
    }

    #[test]
    fn test_language_deserialization_array() {
        let json = r#"{"language": ["eng", "deu", "fra"]}"#;
        let config: OcrConfig = serde_json::from_str(json).unwrap();
        assert_eq!(
            config.language,
            vec!["eng".to_string(), "deu".to_string(), "fra".to_string()]
        );
    }

    #[test]
    fn test_language_deserialization_tesseract_format() {
        let json = r#"{"language": "eng+deu"}"#;
        let config: OcrConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.language, vec!["eng".to_string(), "deu".to_string()]);
    }

    #[test]
    fn test_validate_tesseract_backend() {
        let config = OcrConfig {
            backend: "tesseract".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_unknown_backend_rejected() {
        let config = OcrConfig {
            backend: "unsupported-ocr".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid OCR backend"));
    }

    #[test]
    fn test_validate_paddleocr_backend() {
        let config = OcrConfig {
            backend: "paddleocr".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_legacy_paddle_ocr_config_remains_json_passthrough() {
        let config: OcrConfig = serde_json::from_value(serde_json::json!({
            "backend": "paddle-ocr",
            "paddle_ocr_config": {"legacy_extension_key": true},
        }))
        .unwrap();
        assert_eq!(
            config.paddle_ocr_config,
            Some(serde_json::json!({"legacy_extension_key": true}))
        );
    }

    #[test]
    fn test_paddle_ocr_settings_deserializes_to_typed_struct() {
        let config: OcrConfig = serde_json::from_value(serde_json::json!({
            "backend": "paddle-ocr",
            "paddle_ocr_settings": {"use_angle_cls": true, "enable_table_detection": true},
        }))
        .unwrap();
        let paddle = config
            .paddle_ocr_settings
            .expect("paddle_ocr_settings must deserialize");
        assert!(paddle.use_angle_cls);
        assert!(paddle.enable_table_detection);
        let defaults = crate::paddle_ocr::PaddleOcrConfig::default();
        assert_eq!(paddle.model_tier, defaults.model_tier);
        assert_eq!(paddle.model_version, defaults.model_version);
        assert_eq!(paddle.det_db_box_thresh, defaults.det_db_box_thresh);
    }

    #[test]
    fn test_legacy_paddle_ocr_config_resolves_to_typed_settings() {
        let legacy = serde_json::json!({"model_tier": "server", "enable_table_detection": true});
        let resolved = resolve_paddle_ocr_settings(None, Some(&legacy))
            .unwrap()
            .expect("legacy JSON must resolve");

        assert_eq!(resolved.model_tier, "server");
        assert!(resolved.enable_table_detection);
    }

    #[test]
    fn issue_2010_legacy_paddle_ocr_config_accepts_known_camel_case_keys() {
        let legacy = serde_json::json!({
            "useAngleCls": true,
            "enableTableDetection": true,
            "modelTier": "server",
            "detDbBoxThresh": 0.97,
        });
        let resolved = resolve_paddle_ocr_settings(None, Some(&legacy))
            .unwrap()
            .expect("known legacy camelCase settings must resolve");

        assert!(resolved.use_angle_cls);
        assert!(resolved.enable_table_detection);
        assert_eq!(resolved.model_tier, "server");
        assert!((resolved.det_db_box_thresh - 0.97).abs() < f32::EPSILON);
    }

    #[test]
    fn legacy_paddle_ocr_config_prefers_existing_snake_case_over_alias() {
        let legacy = serde_json::json!({
            "enable_table_detection": false,
            "enableTableDetection": true,
        });
        let resolved = resolve_paddle_ocr_settings(None, Some(&legacy))
            .unwrap()
            .expect("legacy settings must resolve");

        assert!(!resolved.enable_table_detection);
    }

    #[test]
    fn should_ignore_unknown_legacy_paddle_ocr_config_keys() {
        let legacy = serde_json::json!({
            "model_tier": "server",
            "enable_table_detection": true,
            "vendor_extension": {"enabled": true}
        });
        let resolved = resolve_paddle_ocr_settings(None, Some(&legacy))
            .expect("unknown legacy keys must be ignored")
            .expect("legacy settings must resolve");

        assert_eq!(resolved.model_tier, "server");
        assert!(resolved.enable_table_detection);
    }

    #[test]
    fn should_reject_invalid_known_legacy_paddle_ocr_config_values() {
        let legacy = serde_json::json!({
            "det_db_thresh": "not a number",
            "vendor_extension": true
        });
        let error =
            resolve_paddle_ocr_settings(None, Some(&legacy)).expect_err("invalid known legacy values must still fail");

        assert!(
            matches!(&error, XbergError::Validation { .. }),
            "known legacy type errors must remain validation errors: {error}"
        );
        assert!(error.to_string().contains("expected f32"), "{error}");
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn should_validate_legacy_paddle_ocr_config_with_unknown_extension_keys() {
        let config = OcrConfig {
            backend: "paddle-ocr".to_string(),
            paddle_ocr_config: Some(serde_json::json!({
                "model_tier": "server",
                "vendor_extension": true
            })),
            ..Default::default()
        };

        config
            .validate()
            .expect("unknown legacy extension keys must not fail validation");
    }

    #[test]
    fn test_typed_paddle_ocr_settings_take_precedence_over_legacy_json() {
        let legacy = serde_json::json!({"model_tier": 42});
        let typed = crate::paddle_ocr::PaddleOcrConfig::default().with_model_tier("server");
        let resolved = resolve_paddle_ocr_settings(Some(&typed), Some(&legacy))
            .unwrap()
            .expect("typed settings must resolve");

        assert_eq!(resolved.model_tier, "server");
    }

    #[test]
    fn test_paddle_ocr_settings_unknown_key_fails_deserialization() {
        let err = serde_json::from_value::<OcrConfig>(serde_json::json!({
            "paddle_ocr_settings": {"useAngleCls": true},
        }))
        .expect_err("an unknown paddle_ocr_settings key must fail deserialization");
        assert!(err.to_string().contains("useAngleCls"), "{err}");
    }

    #[test]
    fn test_pipeline_stage_paddle_ocr_settings_unknown_key_fails_deserialization() {
        let err = serde_json::from_value::<OcrPipelineStage>(serde_json::json!({
            "backend": "paddleocr",
            "paddle_ocr_settings": {"bogus": 1},
        }))
        .expect_err("an unknown pipeline-stage paddle_ocr_settings key must fail deserialization");
        assert!(err.to_string().contains("bogus"), "{err}");
    }

    #[test]
    fn test_paddle_ocr_settings_deserializes_from_toml() {
        let config: OcrConfig = toml::from_str(
            r#"
backend = "paddle-ocr"

[paddle_ocr_settings]
model_version = "pp-ocrv5"
model_tier = "server"
"#,
        )
        .unwrap();
        let paddle = config
            .paddle_ocr_settings
            .expect("paddle_ocr_settings must deserialize");
        assert_eq!(paddle.model_version, "pp-ocrv5");
        assert_eq!(paddle.model_tier, "server");
        assert!(!paddle.enable_table_detection);
    }

    #[test]
    fn test_validate_invalid_backend_typo() {
        let config = OcrConfig {
            backend: "tesseract_typo".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid OCR backend"));
    }

    #[test]
    fn test_validate_invalid_backend_completely_wrong() {
        let config = OcrConfig {
            backend: "ocr_lib".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid OCR backend") || err_msg.contains("Valid options are"));
    }

    #[test]
    fn test_validate_default_backend() {
        let config = OcrConfig::default();
        assert!(config.validate().is_ok());
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_effective_languages_defaults_and_filters() {
        let empty = OcrConfig {
            language: vec![],
            ..Default::default()
        };
        assert_eq!(empty.effective_languages(), vec!["eng".to_string()]);

        let blank = OcrConfig {
            language: vec![String::new(), "   ".to_string()],
            ..Default::default()
        };
        assert_eq!(blank.effective_languages(), vec!["eng".to_string()]);

        let mixed = OcrConfig {
            language: vec!["eng".to_string(), " ".to_string(), " deu".to_string()],
            ..Default::default()
        };
        assert_eq!(mixed.effective_languages(), vec!["eng".to_string(), "deu".to_string()]);
    }

    #[cfg(all(feature = "ocr", feature = "pdf"))]
    #[test]
    fn test_effective_pipeline_explicit_pipeline_returned_unchanged() {
        let explicit_pipeline = OcrPipelineConfig {
            stages: vec![OcrPipelineStage {
                backend: "paddleocr".to_string(),
                priority: 200,
                language: Some(vec!["fra".to_string()]),
                tesseract_config: None,
                paddle_ocr_config: None,
                paddle_ocr_settings: None,
                vlm_config: None,
                backend_options: None,
            }],
            quality_thresholds: OcrQualityThresholds::default(),
        };
        let config = OcrConfig {
            pipeline: Some(explicit_pipeline.clone()),
            ..Default::default()
        };
        let result = config.effective_pipeline().unwrap();
        assert_eq!(result.stages.len(), 1);
        assert_eq!(result.stages[0].backend, "paddleocr");
        assert_eq!(result.stages[0].priority, 200);
        assert_eq!(result.stages[0].language, Some(vec!["fra".to_string()]));
        assert_eq!(
            config.pipeline_selection(),
            OcrPipelineSelection::HighestScore,
            "an explicit pipeline must keep score-based selection"
        );
    }

    #[cfg(all(feature = "ocr", feature = "pdf"))]
    #[test]
    fn test_effective_pipeline_explicit_paddleocr_no_autofallback() {
        let config = OcrConfig {
            backend: "paddleocr".to_string(),
            ..Default::default()
        };
        assert!(config.effective_pipeline().is_none());
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_effective_pipeline_unknown_backend_rejected_by_validation() {
        let config = OcrConfig {
            backend: "unsupported-ocr".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid OCR backend"));
    }

    #[cfg(all(feature = "ocr", feature = "pdf"))]
    #[test]
    fn test_effective_pipeline_default_tesseract_backend() {
        let config = OcrConfig::default();
        let result = config.effective_pipeline();
        #[cfg(paddle_ocr)]
        {
            let pipeline = result.unwrap();
            assert_eq!(pipeline.stages.len(), 2);
            assert_eq!(pipeline.stages[0].backend, "tesseract");
            assert_eq!(pipeline.stages[0].priority, 100);
            assert_eq!(pipeline.stages[1].backend, "paddleocr");
            assert_eq!(pipeline.stages[1].priority, 50);
        }
        #[cfg(not(paddle_ocr))]
        {
            assert!(result.is_none());
        }
    }

    #[cfg(all(feature = "ocr", paddle_ocr, feature = "pdf"))]
    #[test]
    fn should_keep_synthesized_paddle_fallback_on_its_safe_table_default() {
        let pipeline = OcrConfig::default()
            .effective_pipeline()
            .expect("paddle-ocr feature must produce a pipeline");

        assert!(pipeline.stages[1].paddle_ocr_config.is_none());
        assert!(pipeline.stages[1].paddle_ocr_settings.is_none());
    }

    #[cfg(all(feature = "ocr", paddle_ocr, feature = "pdf"))]
    #[test]
    fn should_not_copy_tesseract_table_intent_into_synthesized_paddle_fallback() {
        let config = OcrConfig {
            tesseract_config: Some(crate::types::TesseractConfig {
                enable_table_detection: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        let pipeline = config
            .effective_pipeline()
            .expect("paddle-ocr feature must produce a pipeline");

        assert_eq!(pipeline.stages[1].paddle_ocr_config, None);
        assert_eq!(pipeline.stages[1].paddle_ocr_settings, None);
    }

    #[cfg(all(feature = "ocr", paddle_ocr, feature = "pdf"))]
    #[test]
    fn should_preserve_explicit_legacy_paddle_table_setting_in_synthesized_fallback() {
        let paddle_ocr_config = serde_json::json!({
            "enable_table_detection": false,
            "model_tier": "server"
        });
        let config = OcrConfig {
            paddle_ocr_config: Some(paddle_ocr_config.clone()),
            ..Default::default()
        };
        let pipeline = config
            .effective_pipeline()
            .expect("paddle-ocr feature must produce a pipeline");

        assert_eq!(pipeline.stages[1].paddle_ocr_config, Some(paddle_ocr_config));
        assert_eq!(pipeline.stages[1].paddle_ocr_settings, None);
    }

    #[cfg(all(feature = "ocr", paddle_ocr, feature = "pdf"))]
    #[test]
    fn should_preserve_typed_paddle_model_settings_without_enabling_tables() {
        let paddle_ocr_settings = crate::paddle_ocr::PaddleOcrConfig::default().with_model_tier("server");
        let config = OcrConfig {
            paddle_ocr_settings: Some(paddle_ocr_settings.clone()),
            ..Default::default()
        };
        let pipeline = config
            .effective_pipeline()
            .expect("paddle-ocr feature must produce a pipeline");

        assert_eq!(pipeline.stages[1].paddle_ocr_config, None);
        assert_eq!(pipeline.stages[1].paddle_ocr_settings, Some(paddle_ocr_settings));
        assert!(
            !pipeline.stages[1]
                .paddle_ocr_settings
                .as_ref()
                .expect("typed settings must remain present")
                .enable_table_detection
        );
    }

    #[cfg(all(feature = "ocr", paddle_ocr, feature = "pdf"))]
    #[test]
    fn should_preserve_explicit_typed_paddle_table_setting_in_synthesized_fallback() {
        let paddle_ocr_settings = crate::paddle_ocr::PaddleOcrConfig::default().with_table_detection(true);
        let config = OcrConfig {
            paddle_ocr_settings: Some(paddle_ocr_settings.clone()),
            ..Default::default()
        };
        let pipeline = config
            .effective_pipeline()
            .expect("paddle-ocr feature must produce a pipeline");

        assert_eq!(pipeline.stages[1].paddle_ocr_config, None);
        assert_eq!(pipeline.stages[1].paddle_ocr_settings, Some(paddle_ocr_settings));
    }

    #[cfg(all(feature = "ocr", feature = "pdf"))]
    #[test]
    fn test_effective_thresholds_custom_vs_default() {
        let custom = OcrQualityThresholds {
            min_total_non_whitespace: 128,
            min_meaningful_words: 10,
            ..Default::default()
        };
        let config_custom = OcrConfig {
            quality_thresholds: Some(custom.clone()),
            ..Default::default()
        };
        let eff = config_custom.effective_thresholds();
        assert_eq!(eff.min_total_non_whitespace, 128);
        assert_eq!(eff.min_meaningful_words, 10);

        let config_default = OcrConfig::default();
        let eff_default = config_default.effective_thresholds();
        assert_eq!(eff_default.min_total_non_whitespace, 64);
        assert_eq!(eff_default.min_meaningful_words, 3);
    }

    #[test]
    fn test_vlm_fallback_policy_default_is_disabled() {
        let config = OcrConfig::default();
        assert_eq!(config.vlm_fallback, VlmFallbackPolicy::Disabled);
    }

    #[test]
    fn auto_strategy_reports_the_default_scan_threshold() {
        assert_eq!(
            OcrStrategy::Auto.effective_min_confidence(),
            DEFAULT_SCANNED_MIN_CONFIDENCE
        );
    }

    #[test]
    fn scanned_pages_strategy_clamps_its_threshold_into_the_unit_interval() {
        assert_eq!(
            OcrStrategy::ScannedPages { min_confidence: -0.5 }.effective_min_confidence(),
            0.0
        );
        assert_eq!(
            OcrStrategy::ScannedPages { min_confidence: 1.5 }.effective_min_confidence(),
            1.0
        );
        assert_eq!(
            OcrStrategy::ScannedPages { min_confidence: 0.42 }.effective_min_confidence(),
            0.42
        );
    }

    #[test]
    fn scanned_pages_strategy_uses_the_default_for_non_finite_direct_values() {
        for min_confidence in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                OcrStrategy::ScannedPages { min_confidence }.effective_min_confidence(),
                DEFAULT_SCANNED_MIN_CONFIDENCE
            );
        }
    }

    #[test]
    fn should_reject_invalid_typed_scanned_pages_confidence_at_the_public_boundary() {
        for min_confidence in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
            let config = crate::ExtractionConfig {
                ocr_strategy: OcrStrategy::ScannedPages { min_confidence },
                ..Default::default()
            };
            let error = config.validate().unwrap_err();
            assert!(
                error.to_string().contains("ocr_strategy.min_confidence"),
                "unexpected error for {min_confidence}: {error}"
            );
        }
    }

    #[test]
    fn should_reject_invalid_json_scanned_pages_confidence_at_the_public_boundary() {
        let json = r#"{"ocr_strategy":{"mode":"scanned_pages","min_confidence":1.1}}"#;
        let config: crate::ExtractionConfig = serde_json::from_str(json).unwrap();
        let error = config.validate().unwrap_err();
        assert!(error.to_string().contains("ocr_strategy.min_confidence"));
    }

    #[test]
    fn test_vlm_fallback_disabled_validate_ok_without_vlm_config() {
        let config = OcrConfig {
            vlm_fallback: VlmFallbackPolicy::Disabled,
            vlm_config: None,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_vlm_fallback_on_low_quality_missing_vlm_config_validate_err() {
        let config = OcrConfig {
            vlm_fallback: VlmFallbackPolicy::OnLowQuality { quality_threshold: 0.6 },
            vlm_config: None,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("vlm_config"),
            "error must mention vlm_config; got: {err_msg}"
        );
    }

    #[test]
    fn test_vlm_fallback_always_missing_vlm_config_validate_err() {
        let config = OcrConfig {
            vlm_fallback: VlmFallbackPolicy::Always,
            vlm_config: None,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("vlm_config"),
            "error must mention vlm_config; got: {err_msg}"
        );
    }

    #[cfg(all(feature = "ocr", feature = "pdf"))]
    fn explicit_low_quality_vlm_config(vlm_cfg: super::super::llm::LlmConfig) -> OcrConfig {
        OcrConfig {
            pipeline: Some(OcrPipelineConfig {
                stages: vec![
                    OcrPipelineStage {
                        backend: "tesseract".to_string(),
                        priority: 100,
                        language: None,
                        tesseract_config: None,
                        paddle_ocr_config: None,
                        paddle_ocr_settings: None,
                        vlm_config: None,
                        backend_options: None,
                    },
                    OcrPipelineStage {
                        backend: "vlm".to_string(),
                        priority: 50,
                        language: None,
                        tesseract_config: None,
                        paddle_ocr_config: None,
                        paddle_ocr_settings: None,
                        vlm_config: Some(vlm_cfg),
                        backend_options: None,
                    },
                ],
                quality_thresholds: OcrQualityThresholds {
                    pipeline_min_quality: 0.6,
                    ..Default::default()
                },
            }),
            ..Default::default()
        }
    }

    #[cfg(all(feature = "ocr", feature = "pdf"))]
    fn assert_vlm_pipeline_stages_match(synthesised: &OcrPipelineConfig, hand_written: &OcrPipelineConfig) {
        assert_eq!(
            synthesised.stages.len(),
            hand_written.stages.len(),
            "stage count mismatch"
        );

        assert_eq!(synthesised.stages[0].backend, hand_written.stages[0].backend);
        assert_eq!(synthesised.stages[0].priority, hand_written.stages[0].priority);

        assert_eq!(synthesised.stages[1].backend, hand_written.stages[1].backend);
        assert_eq!(synthesised.stages[1].priority, hand_written.stages[1].priority);
        let s_vlm = synthesised.stages[1]
            .vlm_config
            .as_ref()
            .expect("synthesised stage 1 must have vlm_config");
        let h_vlm = hand_written.stages[1]
            .vlm_config
            .as_ref()
            .expect("hand-written stage 1 must have vlm_config");
        assert_eq!(s_vlm.model, h_vlm.model);
    }

    /// `OnLowQuality` synthesises a two-stage pipeline equivalent to what a caller
    /// would write by hand with an explicit `OcrPipelineConfig`.
    #[cfg(all(feature = "ocr", feature = "pdf"))]
    #[test]
    fn test_vlm_fallback_on_low_quality_synthesises_two_stage_pipeline() {
        use super::super::llm::LlmConfig;

        let vlm_cfg = LlmConfig {
            model: "openai/gpt-4o-mini".to_string(),
            base_url: Some("http://localhost:9999".to_string()),
            ..Default::default()
        };

        let config = OcrConfig {
            backend: "tesseract".to_string(),
            vlm_fallback: VlmFallbackPolicy::OnLowQuality { quality_threshold: 0.6 },
            vlm_config: Some(vlm_cfg.clone()),
            ..Default::default()
        };

        let explicit = explicit_low_quality_vlm_config(vlm_cfg);

        let synthesised = config.effective_pipeline().expect("must synthesise a pipeline");
        let hand_written = explicit
            .effective_pipeline()
            .expect("explicit pipeline must be returned");

        assert_vlm_pipeline_stages_match(&synthesised, &hand_written);

        assert!(
            (synthesised.quality_thresholds.pipeline_min_quality - 0.6).abs() < f64::EPSILON,
            "threshold must be 0.6, got {}",
            synthesised.quality_thresholds.pipeline_min_quality
        );
        assert!((hand_written.quality_thresholds.pipeline_min_quality - 0.6).abs() < f64::EPSILON,);
        assert_eq!(
            config.pipeline_selection(),
            OcrPipelineSelection::PreferLastNonEmpty,
            "OnLowQuality synthesis must prefer the last non-empty stage as its best-effort fallback"
        );
    }

    /// `Always` synthesises a single-stage VLM-only pipeline.
    #[cfg(all(feature = "ocr", feature = "pdf"))]
    #[test]
    fn test_vlm_fallback_always_synthesises_single_stage_vlm_pipeline() {
        use super::super::llm::LlmConfig;

        let vlm_cfg = LlmConfig {
            model: "anthropic/claude-sonnet-4-20250514".to_string(),
            ..Default::default()
        };
        let config = OcrConfig {
            vlm_fallback: VlmFallbackPolicy::Always,
            vlm_config: Some(vlm_cfg),
            ..Default::default()
        };

        let pipeline = config.effective_pipeline().expect("Always must produce a pipeline");
        assert_eq!(pipeline.stages.len(), 1, "Always must produce exactly one stage");
        assert_eq!(pipeline.stages[0].backend, "vlm");
        assert_eq!(pipeline.stages[0].priority, 100);
        assert!(pipeline.stages[0].vlm_config.is_some());
        assert_eq!(
            config.pipeline_selection(),
            OcrPipelineSelection::PreferLastNonEmpty,
            "Always synthesis must prefer the last non-empty stage as its best-effort fallback"
        );
    }

    /// `Disabled` with no explicit pipeline produces no synthesised pipeline
    /// (consistent with pre-policy single-backend mode).
    #[cfg(all(feature = "ocr", feature = "pdf"))]
    #[test]
    fn test_vlm_fallback_disabled_no_synthesis() {
        let config = OcrConfig {
            backend: "paddleocr".to_string(),
            vlm_fallback: VlmFallbackPolicy::Disabled,
            vlm_config: None,
            ..Default::default()
        };
        assert!(config.effective_pipeline().is_none());
    }

    /// Explicit pipeline always wins over vlm_fallback (regression guard).
    #[cfg(all(feature = "ocr", feature = "pdf"))]
    #[test]
    fn test_explicit_pipeline_wins_over_vlm_fallback() {
        use super::super::llm::LlmConfig;

        let explicit = OcrPipelineConfig {
            stages: vec![OcrPipelineStage {
                backend: "paddleocr".to_string(),
                priority: 99,
                language: None,
                tesseract_config: None,
                paddle_ocr_config: None,
                paddle_ocr_settings: None,
                vlm_config: None,
                backend_options: None,
            }],
            quality_thresholds: OcrQualityThresholds::default(),
        };
        let config = OcrConfig {
            pipeline: Some(explicit),
            vlm_fallback: VlmFallbackPolicy::Always,
            vlm_config: Some(LlmConfig {
                model: "openai/gpt-4o".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let pipeline = config.effective_pipeline().expect("explicit pipeline must be returned");
        assert_eq!(pipeline.stages.len(), 1, "explicit pipeline must win");
        assert_eq!(pipeline.stages[0].backend, "paddleocr", "explicit pipeline must win");
    }

    #[test]
    fn test_vlm_fallback_policy_serde_roundtrip_disabled() {
        let policy = VlmFallbackPolicy::Disabled;
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: VlmFallbackPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, VlmFallbackPolicy::Disabled);
    }

    #[test]
    fn test_vlm_fallback_policy_serde_roundtrip_on_low_quality() {
        let policy = VlmFallbackPolicy::OnLowQuality {
            quality_threshold: 0.42,
        };
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: VlmFallbackPolicy = serde_json::from_str(&json).unwrap();
        match deserialized {
            VlmFallbackPolicy::OnLowQuality { quality_threshold } => {
                assert!((quality_threshold - 0.42).abs() < f64::EPSILON);
            }
            other => panic!("expected OnLowQuality, got {other:?}"),
        }
    }

    #[test]
    fn test_vlm_fallback_policy_serde_roundtrip_always() {
        let policy = VlmFallbackPolicy::Always;
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: VlmFallbackPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, VlmFallbackPolicy::Always);
    }

    #[test]
    fn test_ocr_config_vlm_fallback_omitted_when_disabled_default() {
        let config = OcrConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: OcrConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.vlm_fallback, VlmFallbackPolicy::Disabled);
    }

    #[test]
    fn test_pipeline_config_serde_roundtrip() {
        let pipeline = OcrPipelineConfig {
            stages: vec![
                OcrPipelineStage {
                    backend: "tesseract".to_string(),
                    priority: 100,
                    language: Some(vec!["eng".to_string()]),
                    tesseract_config: None,
                    paddle_ocr_config: None,
                    paddle_ocr_settings: None,
                    vlm_config: None,
                    backend_options: None,
                },
                OcrPipelineStage {
                    backend: "paddleocr".to_string(),
                    priority: 50,
                    language: None,
                    tesseract_config: None,
                    paddle_ocr_config: None,
                    paddle_ocr_settings: Some(crate::paddle_ocr::PaddleOcrConfig::new("en").with_model_tier("server")),
                    vlm_config: None,
                    backend_options: None,
                },
            ],
            quality_thresholds: OcrQualityThresholds::default(),
        };
        let json = serde_json::to_string(&pipeline).unwrap();
        let deserialized: OcrPipelineConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.stages.len(), 2);
        assert_eq!(deserialized.stages[0].backend, "tesseract");
        assert_eq!(deserialized.stages[0].priority, 100);
        assert_eq!(deserialized.stages[1].backend, "paddleocr");
        assert_eq!(deserialized.stages[1].priority, 50);
        assert_eq!(
            deserialized.stages[1].paddle_ocr_settings, pipeline.stages[1].paddle_ocr_settings,
            "typed paddle_ocr_settings must survive a JSON round trip"
        );
    }

    /// `pipeline_selection` derives the best-effort policy from the `OcrConfig` rather
    /// than a serialized pipeline field (#1341): score-based by default and for explicit
    /// pipelines, last-non-empty only for a genuinely-synthesised vlm_fallback pipeline.
    #[cfg(all(feature = "ocr", feature = "pdf"))]
    #[test]
    fn test_pipeline_selection_derivation() {
        use super::super::llm::LlmConfig;

        // Default (classical / no fallback): score-based.
        assert_eq!(
            OcrConfig::default().pipeline_selection(),
            OcrPipelineSelection::HighestScore
        );

        // vlm_fallback requested but no vlm_config: falls through to classical, score-based.
        let missing_vlm = OcrConfig {
            vlm_fallback: VlmFallbackPolicy::Always,
            vlm_config: None,
            ..Default::default()
        };
        assert_eq!(missing_vlm.pipeline_selection(), OcrPipelineSelection::HighestScore);

        // vlm_fallback with vlm_config present: synthesised fallback prefers last non-empty.
        let with_vlm = OcrConfig {
            vlm_fallback: VlmFallbackPolicy::Always,
            vlm_config: Some(LlmConfig {
                model: "openai/gpt-4o-mini".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(with_vlm.pipeline_selection(), OcrPipelineSelection::PreferLastNonEmpty);
    }

    #[test]
    fn test_pipeline_stage_deserialization_missing_optional_fields() {
        let json = r#"{"backend": "tesseract"}"#;
        let stage: OcrPipelineStage = serde_json::from_str(json).unwrap();
        assert_eq!(stage.backend, "tesseract");
        assert_eq!(stage.priority, 100);
        assert!(stage.language.is_none());
        assert!(stage.tesseract_config.is_none());
        assert!(stage.paddle_ocr_config.is_none());
        assert!(stage.paddle_ocr_settings.is_none());
    }

    #[test]
    fn test_pipeline_stage_language_deserialization_single() {
        let json = r#"{"backend": "tesseract", "language": "eng"}"#;
        let stage: OcrPipelineStage = serde_json::from_str(json).unwrap();
        assert_eq!(stage.language, Some(vec!["eng".to_string()]));
    }

    #[test]
    fn test_pipeline_stage_language_deserialization_array() {
        let json = r#"{"backend": "tesseract", "language": ["eng", "deu"]}"#;
        let stage: OcrPipelineStage = serde_json::from_str(json).unwrap();
        assert_eq!(stage.language, Some(vec!["eng".to_string(), "deu".to_string()]));
    }

    #[test]
    fn test_pipeline_stage_default_priority_is_100() {
        let json = r#"{"backend": "paddleocr"}"#;
        let stage: OcrPipelineStage = serde_json::from_str(json).unwrap();
        assert_eq!(stage.priority, 100);
    }

    #[test]
    fn test_ocr_config_deserialization_missing_optional_fields() {
        let json = r#"{}"#;
        let config: OcrConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.backend, "tesseract");
        assert_eq!(config.language, vec!["eng".to_string()]);
        assert!(config.pipeline.is_none());
        assert!(config.quality_thresholds.is_none());
        assert!(config.element_config.is_none());
    }

    #[test]
    fn test_quality_thresholds_deserialization_partial() {
        let json = r#"{"min_total_non_whitespace": 256}"#;
        let thresholds: OcrQualityThresholds = serde_json::from_str(json).unwrap();
        assert_eq!(thresholds.min_total_non_whitespace, 256);
        assert_eq!(thresholds.min_meaningful_words, 3);
        assert_eq!(thresholds.min_garbage_chars, 5);
        assert!((thresholds.pipeline_min_quality - 0.5).abs() < f64::EPSILON);
        assert!(!thresholds.discard_suspected_ocr_noise);
    }

    #[test]
    fn destructive_ocr_noise_filter_defaults_disabled() {
        assert!(!OcrQualityThresholds::default().discard_suspected_ocr_noise);
    }

    #[test]
    fn should_roundtrip_destructive_ocr_noise_filter_opt_in() {
        let thresholds: OcrQualityThresholds = serde_json::from_str(r#"{"discard_suspected_ocr_noise":true}"#).unwrap();
        assert!(thresholds.discard_suspected_ocr_noise);

        let serialized = serde_json::to_string(&thresholds).unwrap();
        let roundtripped: OcrQualityThresholds = serde_json::from_str(&serialized).unwrap();
        assert!(roundtripped.discard_suspected_ocr_noise);
    }

    fn config_with_quality_thresholds(thresholds: OcrQualityThresholds) -> crate::ExtractionConfig {
        crate::ExtractionConfig {
            ocr: Some(OcrConfig {
                quality_thresholds: Some(thresholds),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn assert_invalid_quality_ratio(field: &str, mutate: fn(&mut OcrQualityThresholds)) {
        let mut thresholds = OcrQualityThresholds::default();
        mutate(&mut thresholds);
        let error = config_with_quality_thresholds(thresholds).validate().unwrap_err();
        assert!(
            error.to_string().contains(field),
            "unexpected error for {field}: {error}"
        );
    }

    #[test]
    fn should_reject_invalid_typed_ocr_quality_ratios() {
        assert_invalid_quality_ratio("min_alnum_ratio", |value| value.min_alnum_ratio = -0.1);
        assert_invalid_quality_ratio("max_fragmented_word_ratio", |value| {
            value.max_fragmented_word_ratio = 1.1;
        });
        assert_invalid_quality_ratio("critical_fragmented_word_ratio", |value| {
            value.critical_fragmented_word_ratio = f64::NAN;
        });
        assert_invalid_quality_ratio("max_ocr_output_fragmented_word_ratio", |value| {
            value.max_ocr_output_fragmented_word_ratio = f64::INFINITY;
        });
        assert_invalid_quality_ratio("min_consecutive_repeat_ratio", |value| {
            value.min_consecutive_repeat_ratio = -0.1;
        });
        assert_invalid_quality_ratio("alnum_ws_ratio_threshold", |value| {
            value.alnum_ws_ratio_threshold = 1.1;
        });
        assert_invalid_quality_ratio("min_undecodable_ratio", |value| {
            value.min_undecodable_ratio = -0.1;
        });
        assert_invalid_quality_ratio("min_provenance_fallback_ratio", |value| {
            value.min_provenance_fallback_ratio = 1.1;
        });
    }

    #[test]
    fn should_reject_invalid_typed_ocr_quality_scalars() {
        let mut thresholds = OcrQualityThresholds {
            min_non_whitespace_per_page: -0.1,
            ..Default::default()
        };
        assert!(config_with_quality_thresholds(thresholds.clone()).validate().is_err());

        thresholds.min_non_whitespace_per_page = 1.0;
        thresholds.min_avg_word_length = f64::INFINITY;
        assert!(config_with_quality_thresholds(thresholds.clone()).validate().is_err());

        thresholds.min_avg_word_length = 1.0;
        thresholds.min_ocr_mean_confidence = 100.1;
        assert!(config_with_quality_thresholds(thresholds.clone()).validate().is_err());

        thresholds.min_ocr_mean_confidence = 50.0;
        thresholds.pipeline_min_quality = f64::NAN;
        assert!(config_with_quality_thresholds(thresholds).validate().is_err());
    }

    #[test]
    fn should_validate_json_ocr_quality_thresholds_at_the_public_config_boundary() {
        let invalid_json = r#"{"ocr":{"quality_thresholds":{"pipeline_min_quality":1.1}}}"#;
        let invalid: crate::ExtractionConfig = serde_json::from_str(invalid_json).unwrap();
        let error = invalid.validate().unwrap_err();
        assert!(
            error
                .to_string()
                .contains("ocr.quality_thresholds.pipeline_min_quality")
        );

        let sentinel_json = r#"{"ocr":{"quality_thresholds":{"max_ocr_output_dict_invalid_word_ratio":1.01}}}"#;
        let sentinel: crate::ExtractionConfig = serde_json::from_str(sentinel_json).unwrap();
        sentinel.validate().unwrap();

        let negative_count = r#"{"ocr":{"quality_thresholds":{"min_meaningful_words":-1}}}"#;
        assert!(serde_json::from_str::<crate::ExtractionConfig>(negative_count).is_err());
    }

    #[test]
    fn should_reject_invalid_explicit_pipeline_quality_thresholds() {
        let json = r#"{
            "ocr": {
                "pipeline": {
                    "stages": [],
                    "quality_thresholds": {"max_ocr_output_dict_invalid_word_ratio": 1.001}
                }
            }
        }"#;
        let config: crate::ExtractionConfig = serde_json::from_str(json).unwrap();
        let error = config.validate().unwrap_err();
        assert!(error.to_string().contains("max_ocr_output_dict_invalid_word_ratio"));
    }

    #[test]
    fn test_provenance_ocr_routing_defaults_enabled_with_half_ratio() {
        let thresholds = OcrQualityThresholds::default();
        assert!(thresholds.enable_provenance_ocr_routing);
        assert!((thresholds.min_provenance_fallback_ratio - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_provenance_ocr_routing_deserializes_overrides() {
        let json = r#"{"enable_provenance_ocr_routing": false, "min_provenance_fallback_ratio": 0.75}"#;
        let thresholds: OcrQualityThresholds = serde_json::from_str(json).unwrap();
        assert!(!thresholds.enable_provenance_ocr_routing);
        assert!((thresholds.min_provenance_fallback_ratio - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_plausibility_ocr_routing_defaults_enabled_with_tenth_ratio() {
        let thresholds = OcrQualityThresholds::default();
        assert!(thresholds.enable_plausibility_ocr_routing);
        assert!((thresholds.min_reliable_language_chunk_ratio - 0.10).abs() < f64::EPSILON);
    }

    #[test]
    fn test_plausibility_ocr_routing_deserializes_overrides() {
        let json = r#"{"enable_plausibility_ocr_routing": false, "min_reliable_language_chunk_ratio": 0.25}"#;
        let thresholds: OcrQualityThresholds = serde_json::from_str(json).unwrap();
        assert!(!thresholds.enable_plausibility_ocr_routing);
        assert!((thresholds.min_reliable_language_chunk_ratio - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn should_reject_out_of_range_min_reliable_language_chunk_ratio() {
        let json = r#"{"ocr": {"quality_thresholds": {"min_reliable_language_chunk_ratio": 1.5}}}"#;
        let config: crate::ExtractionConfig = serde_json::from_str(json).unwrap();
        let error = config.validate().unwrap_err();
        assert!(error.to_string().contains("min_reliable_language_chunk_ratio"));
    }

    #[test]
    fn test_validate_catches_invalid_pipeline_stage_backend() {
        let config = OcrConfig {
            pipeline: Some(OcrPipelineConfig {
                stages: vec![
                    OcrPipelineStage {
                        backend: "tesseract".to_string(),
                        priority: 100,
                        language: None,
                        tesseract_config: None,
                        paddle_ocr_config: None,
                        paddle_ocr_settings: None,
                        vlm_config: None,
                        backend_options: None,
                    },
                    OcrPipelineStage {
                        backend: "invalid_backend".to_string(),
                        priority: 50,
                        language: None,
                        tesseract_config: None,
                        paddle_ocr_config: None,
                        paddle_ocr_settings: None,
                        vlm_config: None,
                        backend_options: None,
                    },
                ],
                quality_thresholds: OcrQualityThresholds::default(),
            }),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err(), "Should catch invalid backend in pipeline stages");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid OCR backend") || err_msg.contains("invalid_backend"));
    }

    #[test]
    fn test_validate_passes_with_valid_pipeline_stages() {
        let config = OcrConfig {
            pipeline: Some(OcrPipelineConfig {
                stages: vec![
                    OcrPipelineStage {
                        backend: "tesseract".to_string(),
                        priority: 100,
                        language: None,
                        tesseract_config: None,
                        paddle_ocr_config: None,
                        paddle_ocr_settings: None,
                        vlm_config: None,
                        backend_options: None,
                    },
                    OcrPipelineStage {
                        backend: "paddleocr".to_string(),
                        priority: 50,
                        language: None,
                        tesseract_config: None,
                        paddle_ocr_config: None,
                        paddle_ocr_settings: None,
                        vlm_config: None,
                        backend_options: None,
                    },
                ],
                quality_thresholds: OcrQualityThresholds::default(),
            }),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_ocr_config_backend_options_default_is_none() {
        let config = OcrConfig::default();
        assert!(config.backend_options.is_none());
    }

    #[test]
    fn test_ocr_config_backend_options_serde_roundtrip() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"mode": "fast", "threshold": 0.8, "enable_layout": true})),
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: OcrConfig = serde_json::from_str(&json).unwrap();
        let opts = deserialized.backend_options.unwrap();
        assert_eq!(opts["mode"], "fast");
        assert!((opts["threshold"].as_f64().unwrap() - 0.8).abs() < f64::EPSILON);
        assert_eq!(opts["enable_layout"], true);
    }

    #[test]
    fn test_ocr_config_backend_options_omitted_when_none() {
        let config = OcrConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(
            !json.contains("backend_options"),
            "backend_options must be omitted when None"
        );
    }

    #[test]
    fn test_pipeline_stage_backend_options_serde_roundtrip() {
        let stage = OcrPipelineStage {
            backend: "custom".to_string(),
            priority: 80,
            language: None,
            tesseract_config: None,
            paddle_ocr_config: None,
            paddle_ocr_settings: None,
            vlm_config: None,
            backend_options: Some(serde_json::json!({"batch_size": 4, "device": "cpu"})),
        };
        let json = serde_json::to_string(&stage).unwrap();
        let deserialized: OcrPipelineStage = serde_json::from_str(&json).unwrap();
        let opts = deserialized.backend_options.unwrap();
        assert_eq!(opts["batch_size"], 4);
        assert_eq!(opts["device"], "cpu");
    }

    #[test]
    fn test_pipeline_stage_backend_options_omitted_when_none() {
        let stage = OcrPipelineStage {
            backend: "tesseract".to_string(),
            priority: 100,
            language: None,
            tesseract_config: None,
            paddle_ocr_config: None,
            paddle_ocr_settings: None,
            vlm_config: None,
            backend_options: None,
        };
        let json = serde_json::to_string(&stage).unwrap();
        assert!(
            !json.contains("backend_options"),
            "backend_options must be omitted when None"
        );
    }

    #[cfg(all(feature = "ocr", paddle_ocr, feature = "pdf"))]
    #[test]
    fn test_effective_pipeline_propagates_backend_options_to_primary_stage() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"mode": "fast"})),
            ..Default::default()
        };
        let pipeline = config
            .effective_pipeline()
            .expect("paddle-ocr feature must produce a pipeline");
        assert_eq!(pipeline.stages.len(), 2);

        let primary = &pipeline.stages[0];
        assert_eq!(primary.backend, "tesseract");
        let opts = primary
            .backend_options
            .as_ref()
            .expect("primary stage must carry backend_options");
        assert_eq!(opts["mode"], "fast");

        let fallback = &pipeline.stages[1];
        assert_eq!(fallback.backend, "paddleocr");
        assert!(
            fallback.backend_options.is_none(),
            "paddleocr stage must not inherit backend_options"
        );
    }

    #[cfg(all(feature = "ocr", feature = "pdf"))]
    #[test]
    fn test_explicit_pipeline_ignores_top_level_backend_options() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"mode": "fast"})),
            pipeline: Some(OcrPipelineConfig {
                stages: vec![OcrPipelineStage {
                    backend: "tesseract".to_string(),
                    priority: 100,
                    language: None,
                    tesseract_config: None,
                    paddle_ocr_config: None,
                    paddle_ocr_settings: None,
                    vlm_config: None,
                    backend_options: None,
                }],
                quality_thresholds: OcrQualityThresholds::default(),
            }),
            ..Default::default()
        };
        let pipeline = config
            .effective_pipeline()
            .expect("explicit pipeline must be returned as-is");
        assert_eq!(pipeline.stages.len(), 1);
        assert!(
            pipeline.stages[0].backend_options.is_none(),
            "top-level backend_options must not be injected into an explicit pipeline stage"
        );
    }

    #[cfg(all(feature = "ocr", feature = "pdf"))]
    #[test]
    fn test_stage_level_backend_options_preserved_in_explicit_pipeline() {
        let stage_opts = serde_json::json!({"device": "gpu", "batch": 8});
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({"mode": "fast"})),
            pipeline: Some(OcrPipelineConfig {
                stages: vec![OcrPipelineStage {
                    backend: "custom".to_string(),
                    priority: 100,
                    language: None,
                    tesseract_config: None,
                    paddle_ocr_config: None,
                    paddle_ocr_settings: None,
                    vlm_config: None,
                    backend_options: Some(stage_opts.clone()),
                }],
                quality_thresholds: OcrQualityThresholds::default(),
            }),
            ..Default::default()
        };
        let pipeline = config
            .effective_pipeline()
            .expect("explicit pipeline must be returned as-is");
        let returned_opts = pipeline.stages[0]
            .backend_options
            .as_ref()
            .expect("stage-level backend_options must be preserved");
        assert_eq!(returned_opts["device"], "gpu");
        assert_eq!(returned_opts["batch"], 8);
    }
}
#[test]
fn tagged_ocr_policies_reject_fields_for_unit_variants() {
    let strategy = r#"{"mode":"auto","min_confidence":0.95}"#;
    assert!(serde_json::from_str::<OcrStrategy>(strategy).is_err());

    let fallback = r#"{"mode":"disabled","quality_threshold":0.8}"#;
    assert!(serde_json::from_str::<VlmFallbackPolicy>(fallback).is_err());
}
