//! Cache-key version tag folded into every on-disk cache key.
//!
//! A cached extraction result is only valid for the crate version, cache
//! schema, and *build* that produced it. Neither the content hash (bytes of
//! the input) nor the config hash (the `ExtractionConfig`) changes when
//! *extraction behaviour* changes — a fixed extractor, a reordered pipeline
//! stage, a new post-processor — so without this tag an entry written before
//! the fix is served forever, and the bug it encodes outlives its own fix.
//!
//! The tag mixes in four things:
//! - `CARGO_PKG_VERSION` — the crate version.
//! - [`CACHE_SCHEMA_VERSION`] — bumped by hand when a behaviour change is
//!   deliberately recognised as cache-affecting (see its own doc comment).
//! - `XBERG_BUILD_ID` — a best-effort per-build identifier baked in by
//!   `build.rs` (see that file's `build_id` for the full derivation and its
//!   git-SHA / dirty-tree / env-override precedence, and for the cases —
//!   Docker builds, crates.io/docs.rs tarball builds — where it is `""`).
//!   This is the safety net *under* [`CACHE_SCHEMA_VERSION`], not a
//!   replacement for it: most behaviour changes are never recognised as
//!   cache-affecting and so never get a manual schema bump, but they usually
//!   do land as a distinct commit, which `XBERG_BUILD_ID` catches without
//!   anyone having to remember.
//! - The debug/release profile bit (`cfg!(debug_assertions)`) — a debug and
//!   a release binary built from the identical commit must not silently
//!   share entries if they ever diverge in output; the bit costs nothing to
//!   include even when they don't.
//!
//! When `XBERG_BUILD_ID` is `""` (no git metadata and no override reached
//! the build), that component stops distinguishing anything: the tag's only
//! remaining discriminating power is crate version + schema version, plus
//! the profile bit above (new here regardless of `XBERG_BUILD_ID`) — the
//! same discriminating power the tag had before `XBERG_BUILD_ID` existed.
//! Note this changes the tag's *value*, not just what it discriminates on:
//! deploying this change invalidates every entry already on disk, the same
//! one-time effect as a [`CACHE_SCHEMA_VERSION`] bump.
//!
//! Every cache key is therefore prefixed with this tag. Changing any of the
//! four inputs makes all previously written entries unreachable; they age
//! out through the normal cleanup pass.

/// Generation counter for cached extraction results.
///
/// Bump this whenever extraction output can change for an unchanged input and
/// an unchanged `ExtractionConfig` — that is, whenever a fix or a behaviour
/// change would otherwise be masked by an entry written before it landed.
/// Bumping it invalidates every existing cache entry process-wide.
/// Bumped for #687: the OCR cache key did not cover the Tesseract engine variables
/// `apply_tesseract_variables` applies (`crates/xberg/src/ocr/processor/config.rs`), so
/// entries written before `hocr_font_info` was enabled in 57e414a6db kept being served
/// after it landed, serving stale font-size data. `hash_config` now folds those variables
/// into its own hash, but this bump is still needed to invalidate the entries that were
/// already on disk before that fix.
///
/// Bumped for #783: `hocr_parser::parse_hocr_to_internal_document_with_dictionary_filter`
/// now drops per-line dictionary-invalid noise from the `InternalDocument`/`content` a
/// Tesseract "markdown"-format OCR call produces, for an unchanged image hash and config
/// hash (`hash_config` never covered this -- it is not a Tesseract engine variable, just a
/// call-site decision in `ocr::processor::execution::perform_ocr`). A prior attempt at
/// this exact fix (reverted as `29738a1f29`) was measured byte-identical with its new
/// threshold on vs. off. Its confirmed cause was representation drift -- it stripped only
/// `page_texts` while rendering reads paragraphs -- not this constant. But that commit
/// also failed to bump it, so a stale entry would have hidden a working fix just as
/// effectively, and the two are indistinguishable after the fact. Any A/B measuring this
/// fix MUST run against a cold cache -- see `ocr::cache::OcrCache`.
///
/// Bumped for the PDF OCR source-DPI fix: `extractors::pdf::ocr` now tells the Tesseract
/// backend the true resolution of each rendered page (`SOURCE_DPI_BACKEND_OPTION`), so
/// `image::preprocessing::normalize_image_dpi_owned` stops assuming 72 DPI for rasters
/// rendered at 150. Every PDF OCR page therefore produces a differently sized raster, a
/// different Tesseract `scan_res`, and different output for an unchanged input and config.
/// `hash_config` now folds `TesseractConfig::source_dpi` in, but that only separates future
/// entries from each other; the entries already on disk were written under the 72 assumption
/// with a key that cannot distinguish them, and this bump is what makes them unreachable. Any
/// A/B measuring this fix MUST run against a cold cache.
///
/// Bumped for the OCR page-number fix: `perform_ocr` (`ocr::processor::execution`) stamped
/// every returned element, table, and `OcrElement` with `1` regardless of which page of the
/// source document the image actually was, because Tesseract numbers every single-image call's
/// hOCR/TSV/iterator page as `0`/`1` internally (see `TesseractConfig::page_number`'s doc
/// comment). `hash_config` now folds `page_number` in, but that only separates future entries
/// from each other; entries already on disk were written with the always-`1` page baked into
/// their cached content and a key that cannot distinguish them, and this bump is what makes
/// them unreachable.
///
/// Bumped for built-in extractor provenance: successful built-in extraction now defaults a
/// missing or unrecognized `extraction_method` to `native`. Cached `ExtractedDocument` values
/// are returned before extractor dispatch, so schema-6 entries could otherwise retain `None`.
///
/// Bumped for plugin lifecycle generations: extraction cache entries now wrap the result with
/// the post-processor and validator registry generations that produced it. ~keep
/// Invalidate cached false EXIF warnings in builds without a Git identity. ~keep
pub(crate) const CACHE_SCHEMA_VERSION: u32 = 9;

/// Number of hex characters in the cache version tag.
const VERSION_TAG_HEX_LEN: usize = 8;

/// Return the process-wide cache-key version tag as 8 hex characters.
///
/// Stable for the lifetime of a process: the same binary always produces the
/// same tag, because every input is a compile-time constant (`env!` values
/// and `cfg!(debug_assertions)` are baked in at compilation, not read at
/// runtime). Two builds that differ in crate version, [`CACHE_SCHEMA_VERSION`],
/// debug/release profile, or `XBERG_BUILD_ID` (see `build.rs`'s `build_id`)
/// produce different tags. When `XBERG_BUILD_ID` is `""` on both builds
/// (neither reached git metadata nor an override — e.g. two crates.io/docs.rs
/// tarball builds of the same version), those two builds still collide on
/// that input; see the module doc for why that particular case is considered
/// correct rather than a gap.
pub(crate) fn cache_version_tag() -> &'static str {
    static TAG: std::sync::OnceLock<String> = std::sync::OnceLock::new();

    TAG.get_or_init(|| {
        let mut hasher = blake3::Hasher::new();
        hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
        hasher.update(b"\x00");
        hasher.update(&CACHE_SCHEMA_VERSION.to_le_bytes());
        hasher.update(b"\x00");
        hasher.update(env!("XBERG_BUILD_ID").as_bytes());
        hasher.update(b"\x00");
        hasher.update(&[cfg!(debug_assertions) as u8]);
        hex::encode(&hasher.finalize().as_bytes()[..VERSION_TAG_HEX_LEN / 2])
    })
    .as_str()
}

/// Prefix `cache_key` with the cache-key version tag (see [`cache_version_tag`]).
///
/// The result stays a single safe filename component: the tag is hex, and the
/// separator is `-`, so `Path::file_stem` on `<tag>-<key>.msgpack` round-trips
/// back to the versioned key.
pub(crate) fn versioned_cache_key(cache_key: &str) -> String {
    format!("{}-{}", cache_version_tag(), cache_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_is_eight_lowercase_hex_characters() {
        let tag = cache_version_tag();
        assert_eq!(tag.len(), VERSION_TAG_HEX_LEN, "tag was {tag:?}");
        assert!(
            tag.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "tag was {tag:?}"
        );
    }

    #[test]
    fn tag_is_stable_across_calls() {
        assert_eq!(cache_version_tag(), cache_version_tag());
    }

    #[test]
    fn versioned_key_prefixes_the_tag_and_preserves_the_key() {
        let versioned = versioned_cache_key("deadbeefdeadbeefdeadbeefdeadbeef");
        assert_eq!(
            versioned,
            format!("{}-deadbeefdeadbeefdeadbeefdeadbeef", cache_version_tag())
        );
        assert!(versioned.ends_with("-deadbeefdeadbeefdeadbeefdeadbeef"));
    }

    #[test]
    fn versioned_key_is_deterministic_for_equal_keys_and_distinct_for_different_keys() {
        assert_eq!(versioned_cache_key("aaaa"), versioned_cache_key("aaaa"));
        assert_ne!(versioned_cache_key("aaaa"), versioned_cache_key("bbbb"));
    }

    #[test]
    fn versioned_key_stays_a_single_path_component() {
        let versioned = versioned_cache_key("deadbeef");
        assert!(!versioned.contains('/'), "{versioned}");
        assert!(!versioned.contains('\\'), "{versioned}");
        assert_eq!(
            std::path::Path::new(&format!("{versioned}.msgpack"))
                .file_stem()
                .and_then(|s| s.to_str()),
            Some(versioned.as_str()),
            "file_stem must round-trip back to the versioned key"
        );
    }

    /// Recomputes the tag the way [`cache_version_tag`] does, but with every input
    /// parameterised so the tests below can vary one field at a time.
    ///
    /// Kept in lock-step with `cache_version_tag`'s hasher calls deliberately, not
    /// derived from it, so a future edit to one and not the other is caught by
    /// `helper_matches_the_real_derivation_for_this_process` failing rather than
    /// both drifting together unnoticed.
    fn tag_for(crate_version: &str, schema_version: u32, build_id: &str, debug_profile: bool) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(crate_version.as_bytes());
        hasher.update(b"\x00");
        hasher.update(&schema_version.to_le_bytes());
        hasher.update(b"\x00");
        hasher.update(build_id.as_bytes());
        hasher.update(b"\x00");
        hasher.update(&[debug_profile as u8]);
        hex::encode(&hasher.finalize().as_bytes()[..VERSION_TAG_HEX_LEN / 2])
    }

    /// Confirms `tag_for` above is not a hand-derivation that has drifted from the
    /// real one: fed this process's own compile-time constants, it must reproduce
    /// exactly what `cache_version_tag()` returns.
    #[test]
    fn helper_matches_the_real_derivation_for_this_process() {
        let reproduced = tag_for(
            env!("CARGO_PKG_VERSION"),
            CACHE_SCHEMA_VERSION,
            env!("XBERG_BUILD_ID"),
            cfg!(debug_assertions),
        );
        assert_eq!(reproduced, cache_version_tag());
    }

    /// Guards the whole point of #206: a different schema generation must not be
    /// able to collide with the current one.
    #[test]
    fn bumping_the_schema_version_changes_the_tag() {
        let current = tag_for(env!("CARGO_PKG_VERSION"), CACHE_SCHEMA_VERSION, "", false);
        let bumped = tag_for(env!("CARGO_PKG_VERSION"), CACHE_SCHEMA_VERSION + 1, "", false);
        assert_ne!(current, bumped, "a schema bump must invalidate old entries");
    }

    /// Guards a plain crate-version bump the same way.
    #[test]
    fn bumping_the_crate_version_changes_the_tag() {
        let current = tag_for(env!("CARGO_PKG_VERSION"), CACHE_SCHEMA_VERSION, "", false);
        let other = tag_for("0.0.0-test", CACHE_SCHEMA_VERSION, "", false);
        assert_ne!(current, other, "a crate version bump must invalidate old entries");
    }

    /// ★ Scope of this test and the two below: they prove the *mixing formula* in
    /// [`cache_version_tag`] is sensitive to `XBERG_BUILD_ID` and to the
    /// debug/release profile bit — i.e. if `build.rs` ever does produce two
    /// different `XBERG_BUILD_ID` values, or two builds ever do differ in profile,
    /// the resulting tags are guaranteed to differ too.
    ///
    /// They do NOT prove that two separately compiled binaries actually receive
    /// different `XBERG_BUILD_ID` values in practice. That is a property of
    /// `build.rs`'s `build_id` function plus the git/CI state at build time — a
    /// single `cargo test` process only ever observes its own one compiled value
    /// of `env!("XBERG_BUILD_ID")`, so no in-process test can exercise "two builds"
    /// for real. Verifying that requires building twice (e.g. once on a commit,
    /// once after an uncommitted edit, or once with `XBERG_BUILD_ID` set and once
    /// without) and diffing the resulting tags or on-disk cache filenames — left
    /// for a manual check, not simulated here.
    #[test]
    fn different_build_ids_produce_different_tags() {
        let a = tag_for(env!("CARGO_PKG_VERSION"), CACHE_SCHEMA_VERSION, "sha-aaaaaaaa", false);
        let b = tag_for(env!("CARGO_PKG_VERSION"), CACHE_SCHEMA_VERSION, "sha-bbbbbbbb", false);
        assert_ne!(a, b, "two builds with different build ids must not share cache entries");
    }

    /// See the scope note on `different_build_ids_produce_different_tags` — same
    /// caveat applies: this proves the formula reacts to the profile bit, not that
    /// a debug and a release binary observably diverge in output.
    #[test]
    fn debug_and_release_profiles_produce_different_tags_for_the_same_build_id() {
        let debug = tag_for(env!("CARGO_PKG_VERSION"), CACHE_SCHEMA_VERSION, "sha-cccccccc", true);
        let release = tag_for(env!("CARGO_PKG_VERSION"), CACHE_SCHEMA_VERSION, "sha-cccccccc", false);
        assert_ne!(
            debug, release,
            "a debug and a release binary at the same commit must not share cache entries"
        );
    }

    // Two builds that both land on the empty-`XBERG_BUILD_ID` fallback (no git
    // metadata, no override — a Docker build context per `.dockerignore`, or a
    // crates.io/docs.rs tarball build) collide on that field by construction:
    // `tag_for` is a pure function of its four arguments, so equal arguments
    // always give equal output. That collision is the deliberate, documented
    // degradation described in the module doc, not a bug this suite needs to
    // re-prove — asserting `tag_for(v, s, "", false) == tag_for(v, s, "", false)`
    // would only restate that a function is deterministic. What the module doc
    // promises instead is that this case is *safe* to collide on: crates.io
    // tarballs are immutable per version, so two builds landing here really do
    // share identical source. That claim is about crates.io's guarantees, not
    // this hasher, and isn't something a unit test in this file can check.
}
