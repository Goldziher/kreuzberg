use super::*;

#[test]
fn test_rejoin_hyphenated_words_basic() {
    let input = "modali-\nties";
    let output = TextPostProcessor::rejoin_hyphenated_words(input);
    assert_eq!(output, "modalities");
}

#[test]
fn test_rejoin_hyphenated_words_soft_hyphen() {
    let input = "phenomenon\u{00AD}\nnon";
    let output = TextPostProcessor::rejoin_hyphenated_words(input);
    assert_eq!(output, "phenomenonnon");
}

#[test]
fn test_rejoin_hyphenated_words_with_context() {
    let input = "This is modali-\nties are important";
    let output = TextPostProcessor::rejoin_hyphenated_words(input);
    assert_eq!(output, "This is modalities are important");
}

#[test]
fn test_rejoin_hyphenated_words_preserves_actual_hyphens() {
    let input = "well-designed\nβ-VAE";
    let output = TextPostProcessor::rejoin_hyphenated_words(input);
    // "β" is uppercase, so no rejoin ~keep
    assert_eq!(output, "well-designed\nβ-VAE");
}

#[test]
fn test_rejoin_hyphenated_words_uppercase_start() {
    let input = "test-\nAnother";
    let output = TextPostProcessor::rejoin_hyphenated_words(input);
    assert_eq!(output, "test-\nAnother");
}

#[test]
fn test_rejoin_hyphenated_words_multiple() {
    let input = "phenom-\nenal\n\nmodali-\nties";
    let output = TextPostProcessor::rejoin_hyphenated_words(input);
    assert_eq!(output, "phenomenal\n\nmodalities");
}

#[test]
fn test_rejoin_hard_hyphen_requires_lowercase_both_sides() {
    assert_eq!(
        TextPostProcessor::rejoin_hyphenated_words("modali-\nties"),
        "modalities"
    );
    assert_eq!(TextPostProcessor::rejoin_hyphenated_words("COVID-\n19"), "COVID-\n19");
    assert_eq!(
        TextPostProcessor::rejoin_hyphenated_words("well-\nKnown"),
        "well-\nKnown"
    );
    assert_eq!(
        TextPostProcessor::rejoin_hyphenated_words("modali\u{00AD}\nties"),
        "modalities"
    );
}

#[test]
fn test_rejoin_hyphenated_words_no_hyphens() {
    let input = "No hyphens here\nJust normal text";
    let output = TextPostProcessor::rejoin_hyphenated_words(input);
    assert_eq!(output, input);
}

#[test]
fn test_normalize_whitespace_basic() {
    let input = "The  quick   brown  fox";
    let output = TextPostProcessor::normalize_whitespace(input);
    assert_eq!(output, "The quick brown fox");
}

#[test]
fn test_normalize_whitespace_multiline() {
    let input = "Line  one\nLine  two";
    let output = TextPostProcessor::normalize_whitespace(input);
    assert_eq!(output, "Line one\nLine two");
}

#[test]
fn test_normalize_whitespace_preserves_indentation() {
    let input = "  indented   text";
    let output = TextPostProcessor::normalize_whitespace(input);
    assert!(output.starts_with("  "));
}

#[test]
fn test_normalize_whitespace_email_with_dots() {
    let input = "marlene. mayer@tum. de";
    let output = TextPostProcessor::normalize_whitespace(input);
    assert_eq!(output, "marlene. mayer@tum. de");
}

#[test]
fn test_normalize_whitespace_no_changes_needed() {
    let input = "The quick brown fox";
    let output = TextPostProcessor::normalize_whitespace(input);
    assert_eq!(output, input);
}

#[test]
fn test_process_combined() {
    let input = "modali-\nties  are  important";
    let output = TextPostProcessor::process(input);
    assert_eq!(output, "modalities are important");
}

#[test]
fn test_rejoin_hyphenated_words_with_whitespace_after_hyphen() {
    let input = "test-  \nable";
    let output = TextPostProcessor::rejoin_hyphenated_words(input);
    // trim_start removes the spaces, so "able" starts with lowercase 'a' ~keep
    assert_eq!(output, "testable");
}

#[test]
fn test_normalize_whitespace_empty_string() {
    let input = "";
    let output = TextPostProcessor::normalize_whitespace(input);
    assert_eq!(output, "");
}

#[test]
fn test_rejoin_hyphenated_words_end_of_text() {
    let input = "test-";
    let output = TextPostProcessor::rejoin_hyphenated_words(input);
    assert_eq!(output, "test-");
}

#[test]
fn test_ensure_special_char_spacing_greek_letter_spacing() {
    let input = "computeβVAE";
    let output = TextPostProcessor::ensure_special_char_spacing(input);
    assert!(output.contains(" β "));
}

#[test]
fn test_ensure_special_char_spacing_greek_after_word() {
    let input = "modelβ-VAE";
    let output = TextPostProcessor::ensure_special_char_spacing(input);
    // Space before β, but keep - for hyphen ~keep
    assert!(output.contains(" β-"));
}

#[test]
fn test_ensure_special_char_spacing_multiple_greek_letters() {
    let input = "αβγ";
    let output = TextPostProcessor::ensure_special_char_spacing(input);
    assert!(!output.is_empty());
}

#[test]
fn test_ensure_special_char_spacing_math_symbols() {
    let input = "compute∑x";
    let output = TextPostProcessor::ensure_special_char_spacing(input);
    assert!(output.contains(" ∑ "));
}

#[test]
fn test_ensure_special_char_spacing_preserves_existing_spaces() {
    let input = "compute α VAE";
    let output = TextPostProcessor::ensure_special_char_spacing(input);
    assert!(output.contains("compute α VAE") || output.contains("compute  α  VAE"));
}

#[test]
fn test_ensure_special_char_spacing_parenthesis_handling() {
    let input = "(α)";
    let output = TextPostProcessor::ensure_special_char_spacing(input);
    assert_eq!(output, "(α)");
}

#[test]
fn test_ensure_special_char_spacing_punctuation_after() {
    let input = "variableα,";
    let output = TextPostProcessor::ensure_special_char_spacing(input);
    assert!(output.contains("α,"));
}

#[test]
fn test_ensure_special_char_spacing_hyphen_preservation() {
    let input = "β-VAE";
    let output = TextPostProcessor::ensure_special_char_spacing(input);
    assert!(output.contains("β-"));
}

#[test]
fn test_ensure_special_char_spacing_empty_string() {
    let input = "";
    let output = TextPostProcessor::ensure_special_char_spacing(input);
    assert_eq!(output, "");
}

#[test]
fn test_ensure_special_char_spacing_no_special_chars() {
    let input = "regular text";
    let output = TextPostProcessor::ensure_special_char_spacing(input);
    assert_eq!(output, "regular text");
}

#[test]
fn test_process_full_pipeline_with_special_chars() {
    let input = "modali-\nties  α  VAE";
    let output = TextPostProcessor::process(input);
    assert!(output.contains("modalities"));
    assert!(output.contains("α"));
}

#[test]
fn test_repair_ligatures_ff() {
    // ! → ff ~keep
    assert_eq!(TextPostProcessor::repair_ligatures("di!erent"), "different");
    assert_eq!(TextPostProcessor::repair_ligatures("e!ect"), "effect");
}

#[test]
fn test_repair_ligatures_ffi() {
    // " → ffi (between letters) ~keep
    assert_eq!(TextPostProcessor::repair_ligatures("o\"ces"), "offices");
    assert_eq!(TextPostProcessor::repair_ligatures("e\"cient"), "efficient");
}

#[test]
fn test_repair_ligatures_fi() {
    // # → fi ~keep
    assert_eq!(TextPostProcessor::repair_ligatures("#nancial"), "#nancial");
    assert_eq!(TextPostProcessor::repair_ligatures("de#ne"), "define");
    assert_eq!(TextPostProcessor::repair_ligatures("bene#t"), "benefit");
}

#[test]
fn test_repair_ligatures_fl() {
    // $ → fl ~keep
    assert_eq!(TextPostProcessor::repair_ligatures("$oor"), "$oor");
    assert_eq!(TextPostProcessor::repair_ligatures("re$ect"), "reflect");
}

#[test]
fn test_repair_ligatures_ffl() {
    // % → ffl ~keep
    assert_eq!(TextPostProcessor::repair_ligatures("ba%e"), "baffle");
    assert_eq!(TextPostProcessor::repair_ligatures("ra%e"), "raffle");
}

#[test]
fn test_repair_ligatures_preserves_punctuation() {
    // ! at end of sentence — not a ligature ~keep
    assert_eq!(TextPostProcessor::repair_ligatures("Hello!"), "Hello!");
    // ! at start of word — not a ligature ~keep
    assert_eq!(TextPostProcessor::repair_ligatures("!important"), "!important");
    // " as quote at word boundary ~keep
    assert_eq!(
        TextPostProcessor::repair_ligatures("He said \"hello\""),
        "He said \"hello\""
    );
    // # at start ~keep
    assert_eq!(TextPostProcessor::repair_ligatures("#hashtag"), "#hashtag");
    // $ as currency ~keep
    assert_eq!(TextPostProcessor::repair_ligatures("$100"), "$100");
    // % as percent ~keep
    assert_eq!(TextPostProcessor::repair_ligatures("100%"), "100%");
}

#[test]
fn test_repair_ligatures_multiple() {
    assert_eq!(
        TextPostProcessor::repair_ligatures("the di!erent o\"ces"),
        "the different offices"
    );
}

#[test]
fn test_repair_ligatures_empty() {
    assert_eq!(TextPostProcessor::repair_ligatures(""), "");
}

#[test]
fn test_repair_ligatures_no_changes() {
    let input = "normal text without broken ligatures";
    assert_eq!(TextPostProcessor::repair_ligatures(input), input);
}

#[test]
fn test_is_special_character_greek_letters() {
    assert!(TextPostProcessor::is_special_character('α'));
    assert!(TextPostProcessor::is_special_character('β'));
    assert!(TextPostProcessor::is_special_character('γ'));
    assert!(TextPostProcessor::is_special_character('Ω'));
}

#[test]
fn test_is_special_character_math_symbols() {
    assert!(TextPostProcessor::is_special_character('∑'));
    assert!(TextPostProcessor::is_special_character('∫'));
    assert!(TextPostProcessor::is_special_character('∞'));
}

#[test]
fn test_is_special_character_regular_chars() {
    assert!(!TextPostProcessor::is_special_character('a'));
    assert!(!TextPostProcessor::is_special_character('1'));
    assert!(!TextPostProcessor::is_special_character(' '));
}

#[test]
fn test_space_before_special_bracket() {
    assert!(TextPostProcessor::is_space_before_special('('));
    assert!(TextPostProcessor::is_space_before_special('['));
    assert!(TextPostProcessor::is_space_before_special('{'));
}

#[test]
fn test_space_after_special_punctuation() {
    assert!(TextPostProcessor::is_space_after_special(','));
    assert!(TextPostProcessor::is_space_after_special('.'));
    assert!(TextPostProcessor::is_space_after_special(')'));
}

#[test]
fn test_normalize_leader_dots_basic() {
    assert_eq!(
        TextPostProcessor::normalize_leader_dots("Introduction .................. 5"),
        "Introduction ... 5"
    );
}

#[test]
fn test_normalize_leader_dots_multiple_lines() {
    let input = "Chapter 1.......10\nChapter 2.......25\nChapter 3.......40";
    let output = TextPostProcessor::normalize_leader_dots(input);
    assert_eq!(output, "Chapter 1 ... 10\nChapter 2 ... 25\nChapter 3 ... 40");
}

#[test]
fn test_normalize_leader_dots_short_preserved() {
    assert_eq!(
        TextPostProcessor::normalize_leader_dots("e.g. this is normal"),
        "e.g. this is normal"
    );
    assert_eq!(
        TextPostProcessor::normalize_leader_dots("wait for it..."),
        "wait for it..."
    );
}

#[test]
fn test_normalize_leader_dots_unicode() {
    assert_eq!(
        TextPostProcessor::normalize_leader_dots("Section 1 ···················· 5"),
        "Section 1 ... 5"
    );
    assert_eq!(
        TextPostProcessor::normalize_leader_dots("Section 1 ․․․․․․․․ 5"),
        "Section 1 ... 5"
    );
}

#[test]
fn test_normalize_leader_dots_empty() {
    assert_eq!(TextPostProcessor::normalize_leader_dots(""), "");
}

#[test]
fn test_normalize_leader_dots_no_trailing_content() {
    assert_eq!(
        TextPostProcessor::normalize_leader_dots("Section 1 ............"),
        "Section 1 ..."
    );
}

#[test]
fn test_normalize_leader_dots_preserves_version_numbers() {
    assert_eq!(
        TextPostProcessor::normalize_leader_dots("Version 1.2.3 is released"),
        "Version 1.2.3 is released"
    );
}

#[test]
fn test_process_pipeline_includes_leader_dots() {
    let input = "Chapter 1 .................. 5";
    let output = TextPostProcessor::process(input);
    assert!(output.contains("..."));
    assert!(!output.contains(".................."));
}

#[test]
fn test_normalize_unicode_spaces_hair_space() {
    // U+200A (HAIR SPACE) used as word separator in justified PDFs — must become U+0020 ~keep
    let input = "The\u{200A}\u{200A}\u{200A}\u{200A}K2\u{200A}\u{200A}\u{200A}\u{200A}Australian";
    let output = TextPostProcessor::process(input);
    assert_eq!(output, "The K2 Australian");
}

#[test]
fn test_normalize_unicode_spaces_zero_width() {
    // U+200B (ZERO WIDTH SPACE) should be removed entirely ~keep
    let input = "word\u{200B}boundary";
    let output = TextPostProcessor::process(input);
    assert_eq!(output, "wordboundary");
}

#[test]
fn test_normalize_unicode_spaces_range() {
    // All typographic spaces U+2000–U+200A should collapse to single space ~keep
    let input = "a\u{2000}b\u{2003}c\u{2009}d\u{200A}e";
    let output = TextPostProcessor::process(input);
    assert_eq!(output, "a b c d e");
}

#[test]
fn test_normalize_unicode_spaces_narrow_no_break() {
    // U+202F (NARROW NO-BREAK SPACE) → U+0020 ~keep
    let input = "100\u{202F}km/h";
    let output = TextPostProcessor::process(input);
    assert_eq!(output, "100 km/h");
}

// === regression tests ===
//
// Per the release goal, every fix needs at least one
// test that fails on the broken output and passes on
// the fix. These tests assert the repair-pass behaviour directly
// on the v0.3.54-shaped input strings (taken verbatim from each
// issue's "Actual" output) and verify the post-processed result
// matches the issue's "Expected" output. ~keep

/// Latin ligatures from pdfTeX-typeset PDFs come out as
/// component letters separated by spaces. The post-processing
/// concatenates the three space-separated tokens (prefix +
/// ligature + suffix) back into one word. Examples from the
/// issue body: `differ` → `di ff er`, `affects` → `a ff ects`,
/// `reflects` → `re fl ects`, `affixes` → `af fi xes`.
#[test]
fn ligature_three_token_split_concatenated() {
    assert_eq!(
        TextPostProcessor::repair_ligature_intra_space("di ff er and"),
        "differ and",
    );
    assert_eq!(
        TextPostProcessor::repair_ligature_intra_space("the a ff ects"),
        "the affects",
    );
    assert_eq!(TextPostProcessor::repair_ligature_intra_space("re fl ects"), "reflects",);
    assert_eq!(TextPostProcessor::repair_ligature_intra_space("af fi xes"), "affixes",);
}

#[test]
fn ligature_ffi_swallowed_char_not_recoverable() {
    // Honest limitation: output `di ff cult` from a `/ffi`
    // ligature has lost the `i`; post-processing concatenates
    // `ff` and `cult` but the `i` is gone. Proper root-cause fix
    // at AGL expansion site. ~keep
    assert_eq!(TextPostProcessor::repair_ligature_intra_space("di ff cult"), "diffcult",);
}

#[test]
fn ligature_embedded_in_token_not_repaired() {
    // The `Bara ffe` pattern (where `ffe` is `ff`+`e` in one
    // token) is NOT caught by the regex because the ligature is
    // embedded in surrounding text rather than space-isolated.
    // Honest limitation: regex post-processing requires the
    // space-isolated three-token shape. ~keep
    assert_eq!(
        TextPostProcessor::repair_ligature_intra_space("Bara ffe and"),
        "Bara ffe and",
    );
}

#[test]
fn ligature_repair_idempotent_on_correct_text() {
    let correct = "The difficult question of efficient algorithms remained unsolved.";
    assert_eq!(TextPostProcessor::repair_ligature_intra_space(correct), correct,);
}

/// Combining diacritics are emitted as separate glyphs
/// adjacent to the base letter (`´E`, `Universit e´`,
/// `Sup erieure,´`). Verify the `compose_combining_marks`
/// pass joins them via NFC-equivalent precomposed codepoints.
#[test]
fn combining_acute_mark_before_base_composes() {
    // pdfTeX emits the ACUTE ACCENT (U+00B4) BEFORE the base E,
    // producing `´E`. Should become `É`. ~keep
    let input = "2 \u{00B4}Ecole Normale";
    let expected = "2 École Normale";
    assert_eq!(TextPostProcessor::compose_combining_marks(input), expected);
}

#[test]
fn combining_acute_mark_after_base_composes() {
    // The other ordering: base letter BEFORE the standalone acute.
    // `e´` → `é`. From the issue body: `Universit e´` → `Université`
    // and `Sup erieure,´` → `Supérieure,`. ~keep
    let input = "Universit e\u{00B4} de Lyon";
    let expected = "Université de Lyon";
    assert_eq!(TextPostProcessor::compose_combining_marks(input), expected);
}

#[test]
fn combining_full_diacritic_set_composes() {
    assert_eq!(TextPostProcessor::compose_combining_marks("caf\u{00B4}e"), "café",);
    assert_eq!(TextPostProcessor::compose_combining_marks("a\u{0060}"), "à",);
    assert_eq!(TextPostProcessor::compose_combining_marks("\u{005E}etre"), "être",);
    assert_eq!(TextPostProcessor::compose_combining_marks("c\u{00B8}a"), "ça",);
    assert_eq!(TextPostProcessor::compose_combining_marks("man\u{007E}ana"), "mañana",);
    assert_eq!(TextPostProcessor::compose_combining_marks("u\u{00A8}ber"), "über",);
}

#[test]
fn combining_marks_no_op_on_plain_ascii() {
    let input = "Plain ASCII text with no diacritics.";
    assert_eq!(TextPostProcessor::compose_combining_marks(input), input,);
}

/// Monospace code listings emit one show-text op per glyph,
/// producing intra-token whitespace around punctuation
/// (`function add (a , b ) {`). Verify the
/// `repair_monospace_punctuation_spacing` pass removes the
/// spurious spaces inside code-shaped lines.
#[test]
fn monospace_function_call_spacing_repaired() {
    let actual_v0_3_54 = "function add (a , b ) {\n  return a + b ;\n}";
    let expected = "function add(a, b) {\n  return a + b;\n}";
    assert_eq!(
        TextPostProcessor::repair_monospace_punctuation_spacing(actual_v0_3_54),
        expected,
    );
}

#[test]
fn monospace_method_chain_spacing_repaired() {
    let actual = "function f() { console . log ( add (3 , 5)) ; }";
    let out = TextPostProcessor::repair_monospace_punctuation_spacing(actual);
    // Conservative repair: removes pre-punctuation space
    // post-open-paren space; idempotent on already-correct. ~keep
    assert!(out.contains("(3,"));
    assert!(out.contains("add(3"));
    assert!(!out.contains(" )"));
}

#[test]
fn monospace_repair_skips_prose_lines() {
    // Prose without code keywords should NOT be touched (the
    // heuristic only fires on lines containing both code
    // punctuation AND code keywords). ~keep
    let prose = "The function of the brain is to process information.";
    assert_eq!(TextPostProcessor::repair_monospace_punctuation_spacing(prose), prose,);
}

/// Missing space at run/font boundary. The
/// `repair_run_boundary_space` regex catches case-change boundaries
/// (`theEditor` → `the Editor`) but cannot detect lowercase-to-
/// lowercase merges (`Astrophysicsmanuscript`) — those need the
/// root-cause threshold fix.
#[test]
fn run_boundary_case_change_inserts_space() {
    assert_eq!(
        TextPostProcessor::repair_run_boundary_space("Letter to theEditor"),
        "Letter to the Editor",
    );
    assert_eq!(
        TextPostProcessor::repair_run_boundary_space("andSwift search begins"),
        "and Swift search begins",
    );
}

#[test]
fn run_boundary_repair_skips_camelcase_in_code() {
    // CamelCase identifiers inside code-shaped lines must NOT be
    // split (the heuristic only fires on prose-shaped lines). ~keep
    let code = "let map = HashMap::new();";
    assert_eq!(TextPostProcessor::repair_run_boundary_space(code), code,);
}
