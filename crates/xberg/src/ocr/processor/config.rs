//! Configuration hashing and Tesseract variable management.
//!
//! This module handles configuration hashing for caching and
//! setting Tesseract variables.

use crate::ocr::error::OcrError;
use crate::ocr::types::TesseractConfig;
use xberg_tesseract::TesseractAPI;

const TESSERACT_RESULT_SCHEMA_VERSION: u8 = 15;

/// Compute a deterministic hash of the OCR configuration.
///
/// This hash is used as part of the cache key to ensure different
/// configurations produce different cached results.
///
/// # Arguments
///
/// * `config` - Configuration to hash
/// * `resolved_tessdata_path` - The tessdata directory OCR will actually run against, as
///   returned by [`super::validation::resolve_tessdata_path`]. Hashing the resolved directory
///   rather than only `config.tessdata_path` (the optional override) is required: without it,
///   two calls that resolve to different directories through `TESSDATA_PREFIX` or another
///   fallback in the search chain — with no explicit `OcrConfig.tessdata_path` set — hash
///   identically and share a cache entry even though a different tessdata model produced the
///   cached text (#1787).
///
/// # Returns
///
/// Hexadecimal string representation of the configuration hash
pub(super) fn hash_config(config: &TesseractConfig, resolved_tessdata_path: &str) -> String {
    hash_config_for_schema(config, resolved_tessdata_path, TESSERACT_RESULT_SCHEMA_VERSION)
}

/// Fold the caller's `security_limits` into an OCR cache key.
///
/// These bound the image decode inside `perform_ocr`, which runs only on a cache MISS -- so
/// without them in the key, a request carrying a strict limit is served the result of an earlier
/// permissive request and the limit never applies. That is both a silent policy bypass and the
/// cause of a flaky `issue_1651_ocr_security_limits`: which of its sibling tests populated the
/// entry first decided whether the limit was enforced. ~keep
fn hash_security_limits(hasher: &mut blake3::Hasher, limits: Option<&crate::extractors::security::SecurityLimits>) {
    let Some(limits) = limits else {
        hasher.update(&[0]);
        return;
    };
    hasher.update(&[1]);
    for value in [
        limits.max_archive_size,
        limits.max_compression_ratio,
        limits.max_files_in_archive,
        limits.max_nesting_depth,
        limits.max_entity_length,
        limits.max_content_size,
        limits.max_iterations,
        limits.max_xml_depth,
        limits.max_table_cells,
    ] {
        hasher.update(&value.to_le_bytes());
    }
    match limits.max_pages {
        Some(pages) => {
            hasher.update(&[1]);
            hasher.update(&pages.to_le_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
}

fn hash_config_for_schema(config: &TesseractConfig, resolved_tessdata_path: &str, result_schema_version: u8) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[result_schema_version]);
    hash_bytes(&mut hasher, config.language.as_bytes());
    hasher.update(&config.psm.to_le_bytes());
    hasher.update(&config.oem.to_le_bytes());
    hasher.update(&config.min_confidence.to_bits().to_le_bytes());
    hash_bytes(&mut hasher, config.output_format.as_bytes());
    hash_preprocessing(&mut hasher, config);
    hasher.update(&[config.enable_table_detection as u8]);
    hasher.update(&config.table_min_confidence.to_bits().to_le_bytes());
    hasher.update(&config.table_column_threshold.to_le_bytes());
    hasher.update(&config.table_row_threshold_ratio.to_bits().to_le_bytes());
    hash_tesseract_variables(&mut hasher, config);
    hash_security_limits(&mut hasher, config.security_limits.as_ref());
    hash_source_context(&mut hasher, config, resolved_tessdata_path);

    let hash = hasher.finalize();
    hex::encode(&hash.as_bytes()[..16])
}

fn hash_preprocessing(hasher: &mut blake3::Hasher, config: &TesseractConfig) {
    match config.preprocessing.as_ref() {
        Some(preprocessing) => {
            hasher.update(&[1]);
            hasher.update(&preprocessing.target_dpi.to_le_bytes());
            hasher.update(&[
                preprocessing.auto_rotate as u8,
                preprocessing.deskew as u8,
                preprocessing.denoise as u8,
                preprocessing.contrast_enhance as u8,
                preprocessing.invert_colors as u8,
                // GH#1785 added this and it rewrites `gray` before binarization, so it changes
                // the raster OCR reads. Omitting it here would serve an un-normalized result to
                // a caller that asked for normalization -- the same defect as #687's
                // `hocr_font_info`, and it would make the feature silently inert whenever an
                // entry already existed. ~keep
                preprocessing.normalize_shaded_rows as u8,
            ]);
            hash_bytes(hasher, preprocessing.binarization_method.as_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
}

/// Hash the exact ordered variable set applied to Tesseract so cache identity cannot drift from
/// engine configuration as it did for `hocr_font_info` in #687. ~keep
fn hash_tesseract_variables(hasher: &mut blake3::Hasher, config: &TesseractConfig) {
    for (name, value) in tesseract_variable_set(config) {
        hash_bytes(hasher, name.as_bytes());
        hash_bytes(hasher, value.as_bytes());
    }
}

/// Hash fields that select the input raster, model directory, or page-stamped output. ~keep
fn hash_source_context(hasher: &mut blake3::Hasher, config: &TesseractConfig, resolved_tessdata_path: &str) {
    hasher.update(&[config.auto_rotate as u8]);
    match config.source_dpi {
        Some(dpi) => {
            hasher.update(&[1]);
            hasher.update(&dpi.to_bits().to_le_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
    hasher.update(&[config.known_full_page_scan as u8]);
    hash_bytes(hasher, resolved_tessdata_path.as_bytes());
    hasher.update(&config.page_number.to_le_bytes());
}

fn hash_bytes(hasher: &mut blake3::Hasher, value: &[u8]) {
    hasher.update(&(value.len() as u64).to_le_bytes());
    hasher.update(value);
}

/// Apply Tesseract configuration variables to the API.
///
/// Sets all the advanced Tesseract variables from the configuration.
///
/// # Arguments
///
/// * `api` - Tesseract API instance
/// * `config` - Configuration with variables to apply
///
/// # Returns
///
/// `Ok(())` if all variables were set successfully, otherwise an error
pub(super) fn apply_tesseract_variables(api: &TesseractAPI, config: &TesseractConfig) -> Result<(), OcrError> {
    for (name, value) in tesseract_variable_set(config) {
        api.set_variable(name, &value)
            .map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set {name}: {e}")))?;
    }

    Ok(())
}

/// The full, ordered set of Tesseract engine variables applied for `config`.
///
/// This is the single source of truth consumed by both [`apply_tesseract_variables`] (which
/// sets them on the engine) and [`hash_config`] (which folds them into the OCR cache key). A
/// variable that changes hOCR/TSV serialization must be added here, and only here — adding it
/// directly inside `apply_tesseract_variables` instead (as `hocr_font_info` originally was)
/// reproduces #687: the engine's behaviour changes but the cache key does not, so every
/// previously-cached entry keeps being served as if nothing happened.
///
/// Sorted by variable name so the result — and therefore the cache key — does not depend on
/// insertion order, in case a future change builds this list from an unordered source (e.g. a
/// `HashMap`) instead of literal pushes.
fn tesseract_variable_set(config: &TesseractConfig) -> Vec<(&'static str, String)> {
    let mut variables = vec![
        (
            "classify_use_pre_adapted_templates",
            config.classify_use_pre_adapted_templates.to_string(),
        ),
        ("language_model_ngram_on", config.language_model_ngram_on.to_string()),
        (
            "tessedit_dont_blkrej_good_wds",
            config.tessedit_dont_blkrej_good_wds.to_string(),
        ),
        (
            "tessedit_dont_rowrej_good_wds",
            config.tessedit_dont_rowrej_good_wds.to_string(),
        ),
        (
            "tessedit_enable_dict_correction",
            config.tessedit_enable_dict_correction.to_string(),
        ),
        ("tessedit_char_whitelist", config.tessedit_char_whitelist.clone()),
        ("tessedit_char_blacklist", config.tessedit_char_blacklist.clone()),
        (
            "tessedit_use_primary_params_model",
            config.tessedit_use_primary_params_model.to_string(),
        ),
        (
            "textord_space_size_is_variable",
            config.textord_space_size_is_variable.to_string(),
        ),
        ("thresholding_method", config.thresholding_method.to_string()),
        // Tesseract emits `x_fsize`/`x_font`/`x_bold`/`x_italic` on `ocrx_word` spans only when
        // this variable is on, and it defaults to off. Without it the hOCR parser never sees a
        // font size, so every OCR paragraph falls back to a single constant and heading
        // clustering has no signal. Safe to enable unconditionally: it changes hOCR
        // serialization only, never the recognized text. Unconditional — no `TesseractConfig`
        // field of its own — which is exactly why it must live in this shared list rather than
        // as an inline `set_variable` call: nothing else would ever notice it changed.
        ("hocr_font_info", "1".to_string()),
    ];
    variables.sort_by(|a, b| a.0.cmp(b.0));
    variables
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_TESSDATA_PATH: &str = "/test/tessdata/eng";

    fn create_test_config() -> TesseractConfig {
        TesseractConfig {
            output_format: "text".to_string(),
            enable_table_detection: false,
            use_cache: false,
            ..TesseractConfig::default()
        }
    }

    #[test]
    fn test_hash_config_deterministic() {
        let config = create_test_config();

        let hash1 = hash_config(&config, TEST_TESSDATA_PATH);
        let hash2 = hash_config(&config, TEST_TESSDATA_PATH);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 32);
    }

    /// `source_dpi` changes the raster the preprocessor produces and the `scan_res` Tesseract is
    /// given, so it must be part of the cache identity. Two pages of a mixed-size document can
    /// otherwise share a key and be served each other's result — the same failure mode as #687.
    ///
    /// Fails on unfixed code: `TesseractConfig` has no `source_dpi` field, so this does not
    /// compile. Adding the field but not hashing it makes it compile and fail on the first
    /// assertion, since all three hashes would then be identical.
    #[test]
    fn should_distinguish_cache_keys_by_source_dpi() {
        let unknown = create_test_config();
        let at_150 = TesseractConfig {
            source_dpi: Some(150.0),
            ..create_test_config()
        };
        let at_300 = TesseractConfig {
            source_dpi: Some(300.0),
            ..create_test_config()
        };

        assert_ne!(
            hash_config(&unknown, TEST_TESSDATA_PATH),
            hash_config(&at_150, TEST_TESSDATA_PATH),
            "a known source DPI must not collide with the unknown/72-assumption case"
        );
        assert_ne!(
            hash_config(&at_150, TEST_TESSDATA_PATH),
            hash_config(&at_300, TEST_TESSDATA_PATH),
            "two different known source DPIs must not collide"
        );
    }

    /// `page_number` is stamped onto every returned element/table/`OcrElement`, so it must be
    /// part of the cache identity the same way `source_dpi` is — two calls with byte-identical
    /// images but different declared page numbers must not share a cache entry.
    ///
    /// Fails on unfixed code: `TesseractConfig` has no `page_number` field, so this does not
    /// compile. Adding the field but not hashing it makes it compile and fail on the
    /// assertion, since both hashes would then be identical.
    #[test]
    fn should_distinguish_cache_keys_by_page_number() {
        let page_one = create_test_config();
        let page_two = TesseractConfig {
            page_number: 2,
            ..create_test_config()
        };

        assert_ne!(
            hash_config(&page_one, TEST_TESSDATA_PATH),
            hash_config(&page_two, TEST_TESSDATA_PATH),
            "two different declared page numbers must not collide"
        );
    }

    /// #1787 part 2: the cache key must depend on the tessdata directory OCR actually runs
    /// against, not only the optional `TesseractConfig.tessdata_path` override. Two calls with
    /// byte-identical `TesseractConfig` but resolved against different tessdata directories
    /// (e.g. one via `TESSDATA_PREFIX=fast`, one via `TESSDATA_PREFIX=best`) must not share a
    /// cache entry, or the second model's request is silently served the first model's text.
    ///
    /// Fails on unfixed code: `hash_config` takes a single argument, so this does not compile.
    #[test]
    fn should_distinguish_cache_keys_by_resolved_tessdata_directory() {
        let config = create_test_config();

        assert_ne!(
            hash_config(&config, "/tessdata/fast"),
            hash_config(&config, "/tessdata/best"),
            "two different resolved tessdata directories must not collide, \
             or a comparison across models silently reads the first model's cached text"
        );
    }

    /// Negative control for the fix above: the resolved-directory hashing must not become so
    /// specific that two calls resolving to the SAME directory stop sharing a cache entry. An
    /// over-eager fix (e.g. hashing a directory listing, an inode, or a timestamp instead of the
    /// resolved path string) would make this fail while the positive test above still passes.
    #[test]
    fn same_resolved_tessdata_directory_still_shares_a_cache_key() {
        let config = create_test_config();

        assert_eq!(
            hash_config(&config, "/tessdata/fast"),
            hash_config(&config, "/tessdata/fast"),
            "two calls resolving to the same tessdata directory must still hit the same cache entry"
        );
    }

    /// GH#1785 added `normalize_shaded_rows`, which rewrites the grayscale raster before
    /// binarization and so changes the text OCR returns. It was not in the cache key, which made
    /// the whole feature silently inert whenever an entry for the same image already existed:
    /// turning it on served the un-normalized result back.
    #[test]
    fn should_distinguish_cache_keys_by_normalize_shaded_rows() {
        let mut off = create_test_config();
        off.preprocessing = Some(crate::types::ImagePreprocessingConfig {
            normalize_shaded_rows: false,
            ..Default::default()
        });
        let mut on = off.clone();
        on.preprocessing = Some(crate::types::ImagePreprocessingConfig {
            normalize_shaded_rows: true,
            ..Default::default()
        });

        assert_ne!(
            hash_config(&off, TEST_TESSDATA_PATH),
            hash_config(&on, TEST_TESSDATA_PATH),
            "normalize_shaded_rows changes the raster OCR reads, so it must change the cache key"
        );
    }

    /// `security_limits` bounds the decode inside `perform_ocr`, which runs only on a cache MISS.
    /// Leaving it out of the key let a request carrying a strict limit be served the result an
    /// earlier permissive request had cached, so the limit never applied.
    #[test]
    fn should_distinguish_cache_keys_by_security_limits() {
        let permissive = create_test_config();
        let mut strict = permissive.clone();
        strict.security_limits = Some(crate::extractors::security::SecurityLimits {
            max_content_size: 1,
            ..Default::default()
        });

        assert_ne!(
            hash_config(&permissive, TEST_TESSDATA_PATH),
            hash_config(&strict, TEST_TESSDATA_PATH),
            "a configured security limit must not reuse an unrestricted run's cached result"
        );
    }

    /// Negative control for the two tests above: the added fields must not be hashed so loosely
    /// that two identical configurations stop sharing an entry, which would disable the cache.
    #[test]
    fn identical_limits_and_preprocessing_still_share_a_cache_key() {
        let mut config = create_test_config();
        config.security_limits = Some(crate::extractors::security::SecurityLimits {
            max_content_size: 4096,
            ..Default::default()
        });
        config.preprocessing = Some(crate::types::ImagePreprocessingConfig {
            normalize_shaded_rows: true,
            ..Default::default()
        });

        assert_eq!(
            hash_config(&config, TEST_TESSDATA_PATH),
            hash_config(&config.clone(), TEST_TESSDATA_PATH),
            "two identical configurations must still hit the same cache entry"
        );
    }

    #[test]
    fn test_hash_config_frames_result_schema_version() {
        let config = create_test_config();

        // ~keep This pins the CURRENT version, and the number is meant to move: every bump has to
        // come here and be justified, which is what stops a change to the cached computation from
        // silently reusing entries computed the old way. Version 15 honors the requested OCR
        // engine mode during native initialization; older entries may contain output from
        // automatic mode even when the caller explicitly requested a different engine.
        assert_eq!(
            TESSERACT_RESULT_SCHEMA_VERSION, 15,
            "a bump must be deliberate: state here which change made an old entry wrong"
        );
        assert_ne!(
            hash_config_for_schema(&config, TEST_TESSDATA_PATH, 1),
            hash_config_for_schema(&config, TEST_TESSDATA_PATH, 2)
        );
        assert_ne!(
            hash_config(&config, TEST_TESSDATA_PATH),
            hash_config_for_schema(&config, TEST_TESSDATA_PATH, TESSERACT_RESULT_SCHEMA_VERSION - 1)
        );
    }

    #[test]
    fn test_hash_config_different_languages() {
        let mut config1 = create_test_config();
        config1.language = "eng".to_string();

        let mut config2 = create_test_config();
        config2.language = "fra".to_string();

        let hash1 = hash_config(&config1, TEST_TESSDATA_PATH);
        let hash2 = hash_config(&config2, TEST_TESSDATA_PATH);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_different_psm() {
        let mut config1 = create_test_config();
        config1.psm = 3;

        let mut config2 = create_test_config();
        config2.psm = 6;

        let hash1 = hash_config(&config1, TEST_TESSDATA_PATH);
        let hash2 = hash_config(&config2, TEST_TESSDATA_PATH);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_different_output_format() {
        let mut config1 = create_test_config();
        config1.output_format = "text".to_string();

        let mut config2 = create_test_config();
        config2.output_format = "markdown".to_string();

        let hash1 = hash_config(&config1, TEST_TESSDATA_PATH);
        let hash2 = hash_config(&config2, TEST_TESSDATA_PATH);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_table_detection_flag() {
        let mut config1 = create_test_config();
        config1.enable_table_detection = false;

        let mut config2 = create_test_config();
        config2.enable_table_detection = true;

        let hash1 = hash_config(&config1, TEST_TESSDATA_PATH);
        let hash2 = hash_config(&config2, TEST_TESSDATA_PATH);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_whitelist() {
        let mut config1 = create_test_config();
        config1.tessedit_char_whitelist = "".to_string();

        let mut config2 = create_test_config();
        config2.tessedit_char_whitelist = "0123456789".to_string();

        let hash1 = hash_config(&config1, TEST_TESSDATA_PATH);
        let hash2 = hash_config(&config2, TEST_TESSDATA_PATH);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_blacklist() {
        let config1 = create_test_config();
        let mut config2 = create_test_config();
        config2.tessedit_char_blacklist = "abc".to_string();

        assert_ne!(
            hash_config(&config1, TEST_TESSDATA_PATH),
            hash_config(&config2, TEST_TESSDATA_PATH)
        );
    }

    #[test]
    fn test_hash_config_frames_whitelist_and_blacklist() {
        let mut config1 = create_test_config();
        config1.tessedit_char_whitelist = "ab".to_string();
        config1.tessedit_char_blacklist = "c".to_string();
        let mut config2 = create_test_config();
        config2.tessedit_char_whitelist = "a".to_string();
        config2.tessedit_char_blacklist = "bc".to_string();

        assert_ne!(
            hash_config(&config1, TEST_TESSDATA_PATH),
            hash_config(&config2, TEST_TESSDATA_PATH)
        );
    }

    /// Regression test for the OCR structure defect: without `hocr_font_info`
    /// enabled, Tesseract's hOCR output never carries `x_fsize` on any word, so
    /// `resolve_ocr_font_size_pt` (`crate::pdf::structure::adapters`) can never
    /// read a real per-block font size and every heading/body paragraph collapses
    /// to the same fallback value.
    ///
    /// Uses a real (non-mocked) `TesseractAPI`. `init("", "eng")` relies on Tesseract's
    /// own compiled-in default tessdata location rather than the crate's resolver
    /// (`resolve_tessdata_path_in`, which real jobs and `query_available_languages` both
    /// reach), so it gracefully skips instead of asserting when that default has no
    /// "eng" data in this environment.
    ///
    /// Before the fix, `apply_tesseract_variables` never called
    /// `set_variable("hocr_font_info", ...)`, so this reads back Tesseract's own
    /// default (`false`) and the assertion fails.
    #[test]
    fn test_apply_tesseract_variables_enables_hocr_font_info() {
        let api = match xberg_tesseract::TesseractAPI::new() {
            Ok(api) => api,
            Err(_) => return, // no Tesseract/Leptonica available in this environment
        };
        if api.init("", "eng").is_err() {
            return; // no "eng" tessdata available in this environment
        }

        let config = create_test_config();
        apply_tesseract_variables(&api, &config).expect("apply_tesseract_variables should succeed");

        assert_eq!(
            api.get_bool_variable("hocr_font_info").ok(),
            Some(true),
            "hocr_font_info must be enabled so hOCR word spans carry x_fsize/x_font"
        );
    }

    /// GH#1784: `thresholding_method` was documented as "use adaptive thresholding" while
    /// being a `bool`, and Tesseract's `thresholding_method` engine variable is an
    /// `INT_MEMBER` parsed with `stream >> intval` — the literal strings `"true"`/`"false"`
    /// xberg used to send never parsed as an integer, so the engine variable stayed at
    /// Tesseract's own default (Otsu, `0`) no matter what the caller configured.
    ///
    /// Uses a real (non-mocked) `TesseractAPI`, with no `init()` call: `thresholding_method`
    /// is an `INT_MEMBER` populated by the `TessBaseAPI` constructor itself (see
    /// `get_variable_accessors_return_the_stored_value` in `xberg-tesseract/src/api.rs`), so
    /// reading it back needs no tessdata and cannot be skipped by environment.
    ///
    /// Fails on the pre-fix code (where this field is a `bool` sent via `.to_string()` as the
    /// literal `"true"`): the engine variable then reads back `Some(0)`, not `Some(1)`,
    /// proving the value never reached Tesseract as a usable integer — not merely that our
    /// own struct field held the caller's choice.
    #[test]
    fn should_reach_tesseract_as_an_integer_leptonica_otsu_value_not_the_literal_string_true() {
        let api = xberg_tesseract::TesseractAPI::new().expect("create engine");

        let mut config = create_test_config();
        config.thresholding_method = 1;
        apply_tesseract_variables(&api, &config).expect("apply_tesseract_variables should succeed");

        assert_eq!(
            api.get_int_variable("thresholding_method").ok(),
            Some(1),
            "thresholding_method: 1 must select Tesseract's LeptonicaOtsu method as an integer \
             the engine actually parses"
        );
    }

    /// Negative control for GH#1784: when nothing is configured, the effective Tesseract
    /// method must remain Otsu (`0`) — the regression risk of the fix is a default that
    /// silently changes for every caller who never touched this field.
    #[test]
    fn should_default_to_otsu_when_thresholding_method_is_unconfigured() {
        let api = xberg_tesseract::TesseractAPI::new().expect("create engine");

        let config = create_test_config();
        apply_tesseract_variables(&api, &config).expect("apply_tesseract_variables should succeed");

        assert_eq!(
            api.get_int_variable("thresholding_method").ok(),
            Some(0),
            "an unconfigured thresholding_method must leave Tesseract at Otsu (0), unchanged \
             from before this fix"
        );
    }

    #[test]
    fn test_character_variables_include_empty_resets() {
        let mut configured = create_test_config();
        configured.tessedit_char_whitelist = "0123456789".to_string();
        configured.tessedit_char_blacklist = "abc".to_string();
        let empty = create_test_config();

        let configured_set = tesseract_variable_set(&configured);
        let empty_set = tesseract_variable_set(&empty);
        let value_of = |set: &[(&str, String)], name: &str| {
            set.iter()
                .find(|(n, _)| *n == name)
                .unwrap_or_else(|| panic!("{name} missing from tesseract_variable_set"))
                .1
                .clone()
        };

        assert_eq!(value_of(&configured_set, "tessedit_char_whitelist"), "0123456789");
        assert_eq!(value_of(&configured_set, "tessedit_char_blacklist"), "abc");
        assert_eq!(value_of(&empty_set, "tessedit_char_whitelist"), "");
        assert_eq!(value_of(&empty_set, "tessedit_char_blacklist"), "");
    }

    // ---------------------------------------------------------------------
    // #687 — the OCR cache key does not cover the Tesseract engine variables
    // `apply_tesseract_variables` applies, so a build that changes them (e.g.
    // enabling `hocr_font_info` in 57e414a6db) keeps serving stale entries.
    // ---------------------------------------------------------------------

    /// Every variable `apply_tesseract_variables` sets on the engine must also move the
    /// cache key. This is the invariant #687 violated: `hocr_font_info` was applied
    /// inline, the key never saw it, and 306 pre-fix entries kept being served after
    /// 57e414a6db flipped it — so `x_fsize` stayed absent and the font-size proxy kept
    /// supplying garbage.
    ///
    /// Asserted two ways, because the variable set has two kinds of entry:
    /// - every entry BACKED BY A CONFIG FIELD: flipping the field must change the hash;
    /// - the UNCONDITIONAL entries (today just `hocr_font_info`), which no config value
    ///   can vary: assert their presence, so removing one from the shared list — the
    ///   move that reintroduces this bug — fails here.
    ///
    /// Against the unfixed code `hash_config` hand-copied the same fields, so the
    /// flip assertions pass; it is the `hocr_font_info` presence assertion that fails,
    /// because `tesseract_variable_set` does not exist there at all.
    #[test]
    fn every_applied_tesseract_variable_moves_the_cache_key() {
        let baseline = create_test_config();
        let baseline_hash = hash_config(&baseline, TEST_TESSDATA_PATH);

        let flips = cache_key_flip_cases();

        for (name, flip) in &flips {
            let mut mutated = baseline.clone();
            flip(&mut mutated);
            assert_ne!(
                hash_config(&mutated, TEST_TESSDATA_PATH),
                baseline_hash,
                "changing the config field behind the `{name}` engine variable must change the \
                 OCR cache key, or a run with a different value is served the previous result"
            );
        }

        let names: Vec<&str> = tesseract_variable_set(&baseline)
            .iter()
            .map(|(name, _)| *name)
            .collect();
        for (name, _) in &flips {
            assert!(
                names.contains(name),
                "`{name}` is hashed but no longer applied to the engine, so the two have drifted"
            );
        }
        assert!(
            names.contains(&"hocr_font_info"),
            "hocr_font_info must stay in the shared variable set: it is applied unconditionally, \
             so the set is the only thing that can carry it into the cache key (#687)"
        );
    }

    /// One `(engine variable name, config-field mutation)` pair per config field that backs
    /// a variable `apply_tesseract_variables` sets on the engine.
    #[allow(clippy::type_complexity)]
    fn cache_key_flip_cases() -> Vec<(&'static str, Box<dyn Fn(&mut TesseractConfig)>)> {
        vec![
            (
                "classify_use_pre_adapted_templates",
                Box::new(|c: &mut TesseractConfig| {
                    c.classify_use_pre_adapted_templates = !c.classify_use_pre_adapted_templates
                }),
            ),
            (
                "language_model_ngram_on",
                Box::new(|c: &mut TesseractConfig| c.language_model_ngram_on = !c.language_model_ngram_on),
            ),
            (
                "tessedit_dont_blkrej_good_wds",
                Box::new(|c: &mut TesseractConfig| c.tessedit_dont_blkrej_good_wds = !c.tessedit_dont_blkrej_good_wds),
            ),
            (
                "tessedit_dont_rowrej_good_wds",
                Box::new(|c: &mut TesseractConfig| c.tessedit_dont_rowrej_good_wds = !c.tessedit_dont_rowrej_good_wds),
            ),
            (
                "tessedit_enable_dict_correction",
                Box::new(|c: &mut TesseractConfig| {
                    c.tessedit_enable_dict_correction = !c.tessedit_enable_dict_correction
                }),
            ),
            (
                "tessedit_char_whitelist",
                Box::new(|c: &mut TesseractConfig| c.tessedit_char_whitelist = "0123456789".to_string()),
            ),
            (
                "tessedit_char_blacklist",
                Box::new(|c: &mut TesseractConfig| c.tessedit_char_blacklist = "|~".to_string()),
            ),
            (
                "tessedit_use_primary_params_model",
                Box::new(|c: &mut TesseractConfig| {
                    c.tessedit_use_primary_params_model = !c.tessedit_use_primary_params_model
                }),
            ),
            (
                "textord_space_size_is_variable",
                Box::new(|c: &mut TesseractConfig| {
                    c.textord_space_size_is_variable = !c.textord_space_size_is_variable
                }),
            ),
            (
                "thresholding_method",
                Box::new(|c: &mut TesseractConfig| c.thresholding_method = (c.thresholding_method + 1) % 3),
            ),
        ]
    }

    /// `tesseract_variable_set` must be deterministic and sorted by name, so the
    /// cache key it feeds into can never depend on insertion/iteration order
    /// (e.g. if a future change built it from a `HashMap`). This function does
    /// not exist before this change, so it fails to compile against the unfixed
    /// code — there is no pre-fix equivalent to run it against.
    #[test]
    fn tesseract_variable_set_is_stable_and_sorted_across_repeated_calls() {
        let config = create_test_config();

        let first = tesseract_variable_set(&config);
        let second = tesseract_variable_set(&config);
        assert_eq!(
            first, second,
            "the variable set must be deterministic so the cache key derived from it is too"
        );

        let names: Vec<&str> = first.iter().map(|(name, _)| *name).collect();
        let mut sorted_names = names.clone();
        sorted_names.sort_unstable();
        assert_eq!(names, sorted_names, "the variable set must be returned in sorted order");
    }
}
