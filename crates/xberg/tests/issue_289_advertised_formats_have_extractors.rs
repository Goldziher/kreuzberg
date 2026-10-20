//! Regression test for internal issue #289 (GitHub xberg-io/xberg#1387).
//!
//! # The defect class
//!
//! `crates/xberg/src/core/mime.rs` holds `static FORMATS` — the ungated catalogue of every
//! document format the codebase knows how to *describe*. Every extractor that turns one of
//! those formats into text, by contrast, is `#[cfg(feature = "…")]`-gated in
//! `crates/xberg/src/extractors/mod.rs::register_default_extractors`. Nothing structurally ties
//! the two together, so a build whose Cargo feature set omits an extractor still advertises the
//! format and then rejects it with `XbergError::UnsupportedFormat`.
//!
//! That is exactly what shipped in 1.0.13/1.0.14: `crates/xberg-ffi/Cargo.toml`'s
//! `cfg(not(any(android, ios, windows, macos-x86_64)))` dependency — the block that builds the
//! native libraries for linux-x64, linux-aarch64 and macos-arm64 — lost its baked `"full"`
//! feature and was left with a hand-enumerated list that omitted `excel`, `hwp`, `hwpx`,
//! `iwork`, `wordperfect`, `mdx`, `xml` and `qr-codes`. Reporters saw `.xlsx` listed by
//! `list_supported_formats()` and rejected by `extract()` in the same process.
//!
//! # Why this test is gated on `full` rather than made feature-aware
//!
//! The constraint is that the catalogue and the registry must agree *in whatever build is
//! running*. Two shapes satisfy that:
//!
//! 1. Gate the whole test on `feature = "full"` and assert the complete catalogue resolves.
//! 2. Run in every build and skip a format when its gating Cargo feature is genuinely off.
//!
//! Shape 2 is rejected deliberately. It needs a hand-maintained map from each of the ~90
//! catalogue entries to the `cfg!(feature = "…")` that gates its extractor — a second,
//! independent copy of the same truth, kept in sync by hand. That copy can drift out of step
//! with `register_default_extractors`, and when it does the test would *skip* the format it was
//! supposed to guard, passing while the defect is live. Encoding the bug's own failure mode
//! (two hand-maintained lists silently diverging) into its regression test is not acceptable.
//!
//! Shape 1 has no such copy: under `full` every format extractor is compiled in by definition,
//! so any declared canonical MIME or alias that does not resolve to a registered extractor is a
//! catalogue lie — either a `FORMATS` entry added without an extractor, or an aggregate feature
//! that stopped implying one. The cost is that narrow builds are not covered here; those are
//! covered from the other side by `list_supported_formats()`'s registry intersection (issue
//! #308) and, for the FFI manifest specifically, by
//! `crates/xberg/tests/issue_1387_xlsx_ffi_feature_regression.rs`.
//!
//! # Relationship to the in-crate alias guard
//!
//! `core::mime`'s test module has `every_declared_alias_resolves_to_the_same_extractor_as_its_
//! canonical_mime`, which checks alias → canonical routing and deliberately *skips* any format
//! whose canonical MIME has no extractor. This test is the complementary half: canonical →
//! registered. Neither subsumes the other.
//!
//! The catalogue is read out of the `mime.rs` source because `FORMATS` is a private `static`
//! with no public enumeration API, and `list_supported_formats()` cannot stand in for it — that
//! function already filters itself by the very registry lookup under test, so asserting against
//! its output would be tautological.
//!
//! This test only reads the process-global extractor registry; it never registers, clears or
//! otherwise mutates it, so it is safe alongside tests that exercise registry lifecycle.
#![cfg(feature = "full")]

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use xberg::core::mime::detect_mime_type;
use xberg::extractors::ensure_initialized;
use xberg::plugins::registry::get_document_extractor_registry;

/// Opening line of the catalogue table in `core/mime.rs`.
const FORMATS_TABLE_HEADER: &str = "static FORMATS: &[FormatEntry] = &[";

/// Closing line of the catalogue table in `core/mime.rs`.
const FORMATS_TABLE_FOOTER: &str = "\n];";

/// Lower bound on how many catalogue entries carry at least one file extension. Guards against a
/// parser regression silently reducing the table to a handful of entries (or none), which would
/// let `every_catalogued_format_resolves_to_a_registered_extractor` pass vacuously. Chosen well
/// below the current count (93 at the time of writing) so adding or removing a format never
/// trips it.
const MIN_ENTRIES_WITH_EXTENSIONS: usize = 80;

const EXPECTED_FORMAT_COUNT: usize = 107;
const EXPECTED_EXTENSION_COUNT: usize = 141;
const EXPECTED_ALIAS_COUNT: usize = 58;

const MIME_ONLY_FORMATS: &[&str] = &[
    "text/x-gfm",
    "text/x-markdown-extra",
    "text/x-multimarkdown",
    "text/x-pandoc",
    "application/csl+json",
    "text/x-source-code",
];

/// The formats whose extractors were compiled out of the shipped 1.0.13/1.0.14 native libraries.
/// Pinned by extension so the parser is proven to reach the specific rows this defect was about,
/// spread across the whole table rather than clustered at its start.
const REGRESSION_EXTENSIONS: &[&str] = &[
    "xlsx", "xls", "ods", "hwp", "hwpx", "pages", "numbers", "key", "mdx", "xml", "wpd",
];

/// One row of `core/mime.rs`'s `FORMATS` table.
struct CatalogueEntry {
    extensions: Vec<String>,
    mime_type: String,
    aliases: Vec<String>,
}

/// Reads the `core/mime.rs` source that owns the format catalogue.
fn mime_source() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/core/mime.rs");
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

/// Collects every double-quoted string literal in `line`, in order.
fn string_literals(line: &str) -> Vec<String> {
    let mut literals = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find('"') {
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('"') else {
            break;
        };
        literals.push(after_open[..close].to_owned());
        rest = &after_open[close + 1..];
    }
    literals
}

fn array_literals<'a>(first_line: &str, lines: &mut impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut literals = string_literals(first_line);
    if first_line.contains(']') {
        return literals;
    }
    for line in lines.by_ref() {
        literals.extend(string_literals(line));
        if line.contains(']') {
            return literals;
        }
    }
    panic!("unterminated catalogue array: {first_line}");
}

/// Maps `const NAME: &str = "value";` declarations to their values.
///
/// Four `FORMATS` rows (`odt`, `odp`, `ods`, `wpd`) name a constant instead of inlining the MIME
/// string, so the parser has to resolve them or it would drop those rows.
fn string_constants(source: &str) -> Vec<(String, String)> {
    source
        .lines()
        .filter_map(|line| {
            let declaration = line.trim();
            let after_const = declaration
                .strip_prefix("pub(crate) const ")
                .or_else(|| declaration.strip_prefix("pub const "))
                .or_else(|| declaration.strip_prefix("const "))?;
            let (name, tail) = after_const.split_once(": &str = ")?;
            let value = string_literals(tail).into_iter().next()?;
            Some((name.to_owned(), value))
        })
        .collect()
}

/// Isolates the body of the `FORMATS` table from the surrounding source.
fn formats_table(source: &str) -> &str {
    let after_header = source
        .split_once(FORMATS_TABLE_HEADER)
        .unwrap_or_else(|| panic!("`{FORMATS_TABLE_HEADER}` not found in core/mime.rs"))
        .1;
    after_header
        .split_once(FORMATS_TABLE_FOOTER)
        .unwrap_or_else(|| panic!("unterminated `FORMATS` table in core/mime.rs"))
        .0
}

/// Parses every `FormatEntry` row of the catalogue.
///
/// Line-oriented rather than regex-based, and cross-checked against the raw `FormatEntry {`
/// count by [`the_parser_reads_every_row_of_the_format_table`], so a formatting change that this
/// parser cannot handle fails loudly instead of quietly shrinking the table under test.
fn parse_catalogue(source: &str) -> Vec<CatalogueEntry> {
    let constants = string_constants(source);
    let mut entries = Vec::new();
    let mut extensions: Vec<String> = Vec::new();
    let mut mime_type: Option<String> = None;
    let mut aliases: Vec<String> = Vec::new();
    let mut inside_entry = false;

    let mut lines = formats_table(source).lines();
    while let Some(line) = lines.next() {
        let line = line.trim();
        if line.starts_with("FormatEntry {") {
            inside_entry = true;
            extensions.clear();
            mime_type = None;
            aliases.clear();
        } else if !inside_entry {
            continue;
        } else if let Some(tail) = line.strip_prefix("extensions:") {
            extensions = array_literals(tail, &mut lines);
        } else if let Some(tail) = line.strip_prefix("mime_type:") {
            let value = tail.trim().trim_end_matches(',');
            mime_type = Some(if value.starts_with('"') {
                string_literals(value)
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| panic!("malformed mime_type literal: {line}"))
            } else {
                constants
                    .iter()
                    .find(|(name, _)| name.as_str() == value)
                    .map(|(_, resolved)| resolved.clone())
                    .unwrap_or_else(|| panic!("mime_type constant `{value}` is not declared in core/mime.rs"))
            });
        } else if let Some(tail) = line.strip_prefix("aliases:") {
            aliases = array_literals(tail, &mut lines);
        } else if line == "}," {
            let resolved = mime_type
                .take()
                .unwrap_or_else(|| panic!("FormatEntry row without a mime_type in core/mime.rs"));
            entries.push(CatalogueEntry {
                extensions: std::mem::take(&mut extensions),
                mime_type: resolved,
                aliases: std::mem::take(&mut aliases),
            });
            inside_entry = false;
        }
    }

    entries
}

#[test]
fn catalogue_parser_reads_multiline_extensions_and_aliases() {
    let source = r#"
static FORMATS: &[FormatEntry] = &[
    FormatEntry {
        extensions: &[
            "py",
            "r",
        ],
        mime_type: "text/x-source-code",
        aliases: &[
            "text/x-python",
            "text/x-r-source",
            "text/x-julia",
            "text/vnd.graphviz",
            "text/x-plantuml",
        ],
    },
];
"#;
    let catalogue = parse_catalogue(source);

    assert_eq!(catalogue.len(), 1);
    assert_eq!(catalogue[0].extensions, ["py", "r"]);
    assert_eq!(catalogue[0].mime_type, "text/x-source-code");
    assert_eq!(
        catalogue[0].aliases,
        [
            "text/x-python",
            "text/x-r-source",
            "text/x-julia",
            "text/vnd.graphviz",
            "text/x-plantuml",
        ]
    );
}

/// Every canonical MIME and alias in the catalogue must resolve to the same extractor.
///
/// A failure names every offending format, not just the first, so a feature-list regression that
/// drops several extractors at once is diagnosable in one run.
#[test]
fn every_catalogued_format_resolves_to_a_registered_extractor() {
    ensure_initialized().expect("built-in extractor registration should succeed");

    let source = mime_source();
    let catalogue = parse_catalogue(&source);
    let registry = get_document_extractor_registry();
    let registry_guard = registry.read();

    let mut unextractable = Vec::new();
    for entry in &catalogue {
        let canonical = match registry_guard.get(&entry.mime_type) {
            Ok(extractor) => extractor,
            Err(_) => {
                unextractable.push(format!("{} (.{})", entry.mime_type, entry.extensions.join(", .")));
                continue;
            }
        };
        for alias in &entry.aliases {
            match registry_guard.get(alias) {
                Ok(extractor) if extractor.name() == canonical.name() => {}
                Ok(extractor) => unextractable.push(format!(
                    "{alias} routes to {}, not canonical extractor {}",
                    extractor.name(),
                    canonical.name()
                )),
                Err(_) => unextractable.push(format!("{alias} (alias of {})", entry.mime_type)),
            }
        }
    }
    unextractable.sort();

    assert!(
        unextractable.is_empty(),
        "core/mime.rs advertises {} format(s) with no registered extractor in this build, so \
         extract() rejects them as UnsupportedFormat while the catalogue claims support \
         (issue #289 / GH#1387):\n  {}",
        unextractable.len(),
        unextractable.join("\n  ")
    );
}

/// The parser must see every row of the table.
///
/// Without this, a `FORMATS` formatting change the parser cannot follow would shrink the table
/// under test and let the guard above pass while advertising unextractable formats — a test that
/// passes when the code is broken.
#[test]
fn the_parser_reads_every_row_of_the_format_table() {
    let source = mime_source();
    let table = formats_table(&source);
    let catalogue = parse_catalogue(&source);

    assert_eq!(
        catalogue.len(),
        table.matches("FormatEntry {").count(),
        "the catalogue parser dropped rows of core/mime.rs's FORMATS table; \
         every guard in this file would be weakened by the difference"
    );

    assert_eq!(
        catalogue.len(),
        EXPECTED_FORMAT_COUNT,
        "the audited registry should contain exactly {EXPECTED_FORMAT_COUNT} formats"
    );

    let with_extensions = catalogue.iter().filter(|entry| !entry.extensions.is_empty()).count();
    assert!(
        with_extensions >= MIN_ENTRIES_WITH_EXTENSIONS,
        "only {with_extensions} catalogue entries carry a file extension, expected at least \
         {MIN_ENTRIES_WITH_EXTENSIONS}; the parser is not reading the real table"
    );

    let extensions: HashSet<&str> = catalogue
        .iter()
        .flat_map(|entry| entry.extensions.iter().map(String::as_str))
        .collect();
    assert_eq!(
        extensions.len(),
        EXPECTED_EXTENSION_COUNT,
        "the audited registry should contain exactly {EXPECTED_EXTENSION_COUNT} unique extensions"
    );
    assert_eq!(
        catalogue.iter().map(|entry| entry.aliases.len()).sum::<usize>(),
        EXPECTED_ALIAS_COUNT,
        "the audited registry should contain exactly {EXPECTED_ALIAS_COUNT} MIME aliases"
    );

    let mime_only: Vec<&str> = catalogue
        .iter()
        .filter(|entry| entry.extensions.is_empty())
        .map(|entry| entry.mime_type.as_str())
        .collect();
    assert_eq!(mime_only, MIME_ONLY_FORMATS);
    let unseen: Vec<&str> = REGRESSION_EXTENSIONS
        .iter()
        .copied()
        .filter(|extension| !extensions.contains(extension))
        .collect();

    assert!(
        unseen.is_empty(),
        "the parser did not reach the catalogue rows this regression is about: {unseen:?}"
    );
}

#[test]
fn every_catalogued_extension_is_unique_and_detected_case_insensitively() {
    let source = mime_source();
    let catalogue = parse_catalogue(&source);
    let mut owners = std::collections::HashMap::new();
    let mut checked = 0;

    for entry in &catalogue {
        for extension in &entry.extensions {
            assert_eq!(
                owners.insert(extension.as_str(), entry.mime_type.as_str()),
                None,
                "extension .{extension} has more than one canonical MIME owner"
            );
            for candidate in [extension.clone(), extension.to_ascii_uppercase()] {
                assert_eq!(
                    detect_mime_type(format!("audit.{candidate}"), false).unwrap(),
                    entry.mime_type,
                    "extension .{candidate} should resolve to {}",
                    entry.mime_type
                );
            }
            checked += 1;
        }
    }

    assert_eq!(
        checked, EXPECTED_EXTENSION_COUNT,
        "the detection matrix must not pass vacuously"
    );
}

#[test]
fn every_catalogued_mime_name_has_one_owner() {
    let source = mime_source();
    let catalogue = parse_catalogue(&source);
    let mut owners = std::collections::HashMap::new();
    let mut checked = 0;

    for entry in &catalogue {
        for mime_type in std::iter::once(&entry.mime_type).chain(&entry.aliases) {
            assert_eq!(
                owners.insert(mime_type.as_str(), entry.mime_type.as_str()),
                None,
                "MIME name {mime_type} has more than one canonical owner"
            );
            checked += 1;
        }
    }

    assert_eq!(
        checked,
        EXPECTED_FORMAT_COUNT + EXPECTED_ALIAS_COUNT,
        "the MIME ownership matrix must not pass vacuously"
    );
}
