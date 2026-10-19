use super::*;

#[test]
fn looks_rtl_pure_ascii_is_false() {
    assert!(!looks_rtl("hello world"));
    assert!(!looks_rtl(""));
}

#[test]
fn reverse_keep_numbers_digit_free_is_plain_reversal() {
    // No digits → byte-identical to chars().rev() so digit-free RTL
    // (the corpus-validated path) is untouched. ~keep
    for s in ["الثدييات", "שלום", "ab-cd!", ""] {
        let plain: String = s.chars().rev().collect();
        assert_eq!(reverse_rtl_keep_numbers(s), plain, "changed digit-free {s:?}");
    }
}

#[test]
fn reverse_keep_numbers_preserves_year() {
    // Visual-order Hebrew "ל-2009," → logical "ל-2009," (digits stay 2009,
    // a plain reversal would emit 9002). Visual input is the rendered order. ~keep
    assert_eq!(reverse_rtl_keep_numbers(",2009-ל"), "ל-2009,");
}

#[test]
fn reverse_keep_numbers_keeps_internal_separators() {
    // Thousands / decimal separators between digits stay with the number. ~keep
    assert_eq!(reverse_rtl_keep_numbers(",1,000-ל"), "ל-1,000,");
    assert_eq!(reverse_rtl_keep_numbers("3.14-ל"), "ל-3.14");
}

#[test]
fn reverse_keep_numbers_keeps_a_terminator_with_its_number() {
    // UAX #9 W5: a European terminator touching a European number joins it,
    // so the sign stays on the side of the digits it was drawn on.
    assert_eq!(reverse_rtl_keep_numbers("50%"), "50%");
    assert_eq!(reverse_rtl_keep_numbers("%50"), "%50");
    assert_eq!(reverse_rtl_keep_numbers("$50"), "$50");
    assert_eq!(reverse_rtl_keep_numbers("47.500%-ל"), "ל-47.500%");
}

#[test]
fn reverse_keep_numbers_leaves_a_lone_terminator_alone() {
    // No number to join, so the terminator reverses as an ordinary neutral.
    assert_eq!(reverse_rtl_keep_numbers("%ל"), "ל%");
}

#[test]
fn looks_rtl_arabic_is_true() {
    assert!(looks_rtl("مرحبا"));
    assert!(looks_rtl("year 2024 عام"));
}

#[test]
fn looks_rtl_hebrew_is_true() {
    assert!(looks_rtl("שלום"));
}

#[test]
fn reorder_pure_ltr_is_identity() {
    let s = "Hello, world!";
    assert_eq!(reorder_visual_to_logical(s), s);
}

/// D7-fix documentation — `reorder_visual_to_logical` assumes the
/// input is in *visual* order and converts to logical. PDFs vary:
/// some store visual order (Arabic news papers, certain Acrobat
/// outputs) and some store logical order (most modern publishers,
/// the pdfium hebrew_mirrored.pdf test fixture). Callers MUST
/// know which case they are in. The default markdown converter
/// no longer invokes this function for that reason — see
/// pipeline::converters::markdown.rs RTL emphasis-cleanup block.
/// This test pins the asymmetric behaviour as a contract.
#[test]
fn reorder_is_a_visual_to_logical_converter_not_idempotent() {
    let logical_hebrew = "בנימין";
    let after_first = reorder_visual_to_logical(logical_hebrew);
    assert_ne!(after_first, logical_hebrew);
    let after_second = reorder_visual_to_logical(&after_first);
    assert_eq!(after_second, logical_hebrew);
}

/// D7 RED — A visual-order Arabic line with embedded English
/// numerals must come back in logical order with the numerals
/// preserved in their natural reading direction. Reproduces the
/// `right_to_left_02` fixture pattern.
#[test]
fn reorder_arabic_with_numerals_keeps_digits_logical() {
    let logical = "عام 2024 كان جيدا";
    let result = reorder_visual_to_logical(logical);
    assert!(
        result.contains("2024"),
        "expected `2024` in reordered line, got {:?}",
        result
    );
    assert_eq!(result.chars().count(), logical.chars().count());
}

#[test]
fn paragraph_is_rtl_for_arabic() {
    assert!(paragraph_is_rtl("هذا نص عربي"));
}

#[test]
fn paragraph_is_not_rtl_for_pure_english() {
    assert!(!paragraph_is_rtl("This is English"));
}

/// `looks_rtl` and `crate::text::rtl_detector::is_rtl_text` must
/// agree on every codepoint, since the bidi module delegates to
/// the detector. Pin the parity to catch any future drift in
/// either direction.
#[test]
fn looks_rtl_delegates_to_rtl_detector() {
    for cp in [
        // Edges of every supported block. ~keep
        0x058F, 0x0590, 0x05FF, 0x0600, 0x0633, 0x06FF, 0x0700, 0x074F, 0x0750, 0x077F, 0x0780, 0x08A0, 0x08FF, 0x0900,
        0xFB4F, 0xFB50, 0xFDFF, 0xFE00, 0xFE70, 0xFEFE, 0xFEFF, 0xFF00,
    ] {
        if let Some(c) = char::from_u32(cp) {
            let s = c.to_string();
            let bidi_says = looks_rtl(&s);
            let detector_says = crate::text::rtl_detector::is_rtl_text(cp);
            assert_eq!(
                bidi_says, detector_says,
                "U+{:04X}: looks_rtl={} but rtl_detector::is_rtl_text={}",
                cp, bidi_says, detector_says
            );
        }
    }
}

/// `paragraph_is_rtl` must reflect the *dominant* paragraph
/// direction (per UAX #9 §3.3.1 — the level of the first strong
/// character). A paragraph led by an LTR token but with RTL
/// chars further in (e.g. `Foo بار 1`) is logically LTR and
/// must not report as RTL just because some RTL characters
/// appear later. Earlier impl returned true on any string
/// containing RTL chars, conflating with `looks_rtl`.
#[test]
fn paragraph_is_rtl_respects_dominant_direction() {
    // Dominant LTR (first strong char is Latin) → false. ~keep
    assert!(!paragraph_is_rtl("Foo بار 1"));
    // Dominant RTL (first strong char is Arabic) → true. ~keep
    assert!(paragraph_is_rtl("بار Foo 1"));
}

/// D7 coverage — the looks_rtl quick-check spans every RTL Unicode
/// block we declare support for. Used as the converter's gate, so
/// any block we miss here would entirely bypass the bidi pass for
/// that script.
#[test]
fn looks_rtl_covers_all_supported_blocks() {
    let cases: &[(u32, &str)] = &[
        (0x0590, "Hebrew start"),
        (0x05F4, "Hebrew end-ish"),
        (0x0600, "Arabic start"),
        (0x06FF, "Arabic end"),
        (0x0750, "Arabic Supplement start"),
        (0x077F, "Arabic Supplement end"),
        (0x08A0, "Arabic Extended-A start"),
        (0x08FF, "Arabic Extended-A end"),
        (0xFB50, "Arabic Presentation Forms-A start"),
        (0xFDFF, "Arabic Presentation Forms-A end"),
        (0xFE70, "Arabic Presentation Forms-B start"),
        (0xFEFF, "Arabic Presentation Forms-B end"),
    ];
    for (cp, name) in cases {
        if let Some(c) = char::from_u32(*cp) {
            let s = c.to_string();
            assert!(looks_rtl(&s), "looks_rtl({:?} {}) should be true", s, name);
        }
    }
}

/// D7 negative coverage — characters that LOOK like they could be
/// RTL but are actually neutral or LTR (CJK, math, common
/// punctuation, the BOM area near U+FEFF).
#[test]
fn looks_rtl_rejects_neutral_and_cjk() {
    for s in ["中文", "日本語", "α β γ", "1234567890", "!@#$%^&*()", "café", "naïve"] {
        assert!(!looks_rtl(s), "looks_rtl({:?}) should be false", s);
    }
}

/// D7 coverage — reorder is byte-stable for pure-ASCII strings of
/// many shapes (no RTL means identity).
#[test]
fn reorder_pure_ltr_identity_extras() {
    for s in [
        "",
        "a",
        "Hello, world!",
        "Multi-line\nstays unchanged",
        "Numbers: 1234 5678",
        "Symbols: !@#$%^&*",
        "Whitespace   between   words",
    ] {
        assert_eq!(reorder_visual_to_logical(s), s, "identity broken on {:?}", s);
    }
}

/// D7 coverage — reorder preserves character count and never drops
/// or duplicates content. Property-style spot-check across mixed
/// inputs.
#[test]
fn reorder_preserves_character_count() {
    for s in [
        "عربي",
        "هذا نص عربي للاختبار",
        "year 2024 عام جيد",
        "שלום world",
        "Mixed: عربي + 123 + Latin",
    ] {
        let out = reorder_visual_to_logical(s);
        assert_eq!(
            out.chars().count(),
            s.chars().count(),
            "char count changed: {:?} -> {:?}",
            s,
            out
        );
    }
}

/// D7 coverage — embedded LTR runs (English brand names, codes)
/// inside an Arabic paragraph survive intact in the output. The
/// English token must still be findable as a contiguous substring,
/// not reversed.
#[test]
fn reorder_keeps_embedded_ltr_token_contiguous() {
    let line = "هذا منتج Microsoft الجديد";
    let result = reorder_visual_to_logical(line);
    assert!(
        result.contains("Microsoft"),
        "embedded LTR token reversed: {:?} -> {:?}",
        line,
        result
    );
}

/// D7 coverage — paragraph_is_rtl agrees with looks_rtl on edge
/// cases (empty string, whitespace, mixed-script).
#[test]
fn paragraph_is_rtl_edges() {
    assert!(!paragraph_is_rtl(""));
    assert!(!paragraph_is_rtl("   "));
    assert!(!paragraph_is_rtl("123 456"));
    // Mixed but RTL-dominated. ~keep
    assert!(paragraph_is_rtl("نص with English"));
}

/// The motivating BidiSample case: a confidently-RTL date line that
/// mixes Latin (`april`), European numerals (`1434`/`14`) and an
/// Arabic-Indic numeral run (`٤٣٤١`). The embedded LTR sub-runs must
/// read left-to-right and keep their relative position within the
/// line; char count is preserved (output is a permutation).
#[test]
fn reorder_mixed_rtl_line_date_keeps_ltr_subruns_left_to_right() {
    let line = "14 april 1434 ٤٣٤١";
    let out = reorder_mixed_rtl_line(line);
    // Embedded LTR tokens stay left-to-right (not reversed). ~keep
    assert!(out.contains("1434"), "`1434` reversed/lost: {:?} -> {:?}", line, out);
    assert!(out.contains("april"), "`april` reversed/lost: {:?} -> {:?}", line, out);
    assert!(
        out.contains("14 "),
        "leading `14` reversed/lost: {:?} -> {:?}",
        line,
        out
    );
    // Relative line position preserved: `14` precedes `april`, which
    // precedes `1434`, in the emitted (logical) order. ~keep
    let p14 = out.find("14").expect("14 present");
    let papril = out.find("april").expect("april present");
    let p1434 = out.find("1434").expect("1434 present");
    assert!(p14 < papril && papril < p1434, "LTR sub-run order changed: {:?}", out);
    assert_eq!(
        out.chars().count(),
        line.chars().count(),
        "char count changed: {:?} -> {:?}",
        line,
        out
    );
}

/// A pure-Arabic line (no embedded digit/Latin) hits the "mixed"
/// gate and is returned byte-for-byte identical — pins the
/// no-regression contract for `right_to_left_02` / Hebrew fixtures.
#[test]
fn reorder_mixed_rtl_line_pure_arabic_is_byte_identical() {
    let line = "هذا نص عربي خالص";
    assert_eq!(reorder_mixed_rtl_line(line), line);
}

/// A pure-English line is LTR-dominant (first strong char Latin),
/// fails the RTL gate, and is returned byte-for-byte identical.
#[test]
fn reorder_mixed_rtl_line_pure_english_is_byte_identical() {
    let line = "This is plain English 2024";
    assert_eq!(reorder_mixed_rtl_line(line), line);
}

/// An ambiguous / LTR-first mixed line (first strong char is Latin
/// even though Arabic appears later) is left unchanged — the
/// confidence gate only acts on RTL-dominant lines.
#[test]
fn reorder_mixed_rtl_line_ltr_first_is_unchanged() {
    let line = "Invoice رقم 123";
    assert_eq!(reorder_mixed_rtl_line(line), line);
}

/// Char count is preserved across a spread of mixed RTL inputs
/// (property-style spot check) — output is always a permutation.
#[test]
fn reorder_mixed_rtl_line_preserves_char_count() {
    for s in [
        "14 april 1434 ٤٣٤١",
        "هذا منتج Microsoft الجديد",
        "عام 2024 كان جيدا",
        "السعر 99 دولار",
    ] {
        let out = reorder_mixed_rtl_line(s);
        assert_eq!(
            out.chars().count(),
            s.chars().count(),
            "char count changed: {:?} -> {:?}",
            s,
            out
        );
    }
}

#[test]
fn detect_visual_run_short_run_is_ambiguous() {
    // < 4 RTL letters → not enough signal. ~keep
    let three_chars = [('ק', 0.0), ('ר', 6.0), ('ח', 12.0)];
    assert_eq!(detect_visual_order_run(&three_chars), RunOrder::Ambiguous);
}

#[test]
fn detect_visual_run_hebrew_visual_order() {
    // Hebrew word "מקלדת" (keyboard, 5 letters) emitted in visual
    // order: leftmost glyph first in stream, ascending x. ~keep
    let visual = [('מ', 0.0), ('ק', 6.0), ('ל', 12.0), ('ד', 18.0), ('ת', 24.0)];
    assert_eq!(detect_visual_order_run(&visual), RunOrder::Visual);
}

#[test]
fn detect_visual_run_hebrew_logical_order() {
    // Same letters, logical order: rightmost glyph first in stream
    // (descending x — the PDF producer ran its own bidi pass before
    // drawing). ~keep
    let logical = [('מ', 24.0), ('ק', 18.0), ('ל', 12.0), ('ד', 6.0), ('ת', 0.0)];
    assert_eq!(detect_visual_order_run(&logical), RunOrder::Logical);
}

#[test]
fn detect_visual_run_arabic_main_block_visual() {
    // Arabic main block (U+0600-U+06FF), no Presentation Forms.
    // Ascending x → Visual. ~keep
    let visual = [('ع', 0.0), ('ر', 7.0), ('ب', 14.0), ('ي', 21.0)];
    assert_eq!(detect_visual_order_run(&visual), RunOrder::Visual);
}

#[test]
fn detect_visual_run_presentation_forms_bails_out() {
    // Arabic Presentation Forms-B in the run — Pass 0 owns this.
    // The geometric detector must bail rather than double-process. ~keep
    let with_pfs = [
        ('\u{FE80}', 0.0), // Hamza isolated form ~keep
        ('\u{FE91}', 7.0), // Beh initial form ~keep
        ('\u{FE9A}', 14.0),
        ('\u{FEAB}', 21.0),
    ];
    assert_eq!(detect_visual_order_run(&with_pfs), RunOrder::Ambiguous);
}

#[test]
fn detect_visual_run_ties_are_ambiguous() {
    // All chars at the same x (degenerate). No monotonicity signal. ~keep
    let ties = [('ק', 5.0), ('ר', 5.0), ('ח', 5.0), ('ל', 5.0)];
    assert_eq!(detect_visual_order_run(&ties), RunOrder::Ambiguous);
}

#[test]
fn detect_visual_run_mixed_signal_is_ambiguous() {
    // 4 RTL letters: 1 ascending pair, 2 descending pairs. With
    // only 3 monotonic pairs (asc=1, desc=2, total=3), neither
    // direction reaches the 90 % floor → Ambiguous. ~keep
    let mixed = [('ק', 0.0), ('ר', 6.0), ('ח', 3.0), ('ל', 1.0)];
    assert_eq!(detect_visual_order_run(&mixed), RunOrder::Ambiguous);
}

#[test]
fn detect_visual_run_ignores_non_rtl_chars() {
    // Embedded LTR digit ("2024") between Hebrew letters — filtered
    // out before the monotonicity check. Hebrew chars still need
    // to be ≥4 and monotonic. ~keep
    let with_digit = [('ק', 0.0), ('ר', 6.0), ('2', 12.0), ('ח', 18.0), ('ל', 24.0)];
    assert_eq!(detect_visual_order_run(&with_digit), RunOrder::Visual);
}

#[test]
fn detect_visual_run_kerning_tolerance() {
    // Tiny x differences within 0.5pt → treated as ties; can't
    // be the dominant signal on their own. Four pairs where dx
    // ≈ 0.3pt → all ties → Ambiguous. ~keep
    let kerning_noise = [('ק', 0.0), ('ר', 0.3), ('ח', 0.6), ('ל', 0.9), ('מ', 1.2)];
    assert_eq!(detect_visual_order_run(&kerning_noise), RunOrder::Ambiguous);
}

#[test]
fn wrap_rtl_isolates_pure_ltr_is_identity() {
    // Pure-LTR English in an LTR block — nothing to wrap, byte-
    // identical output. This is the no-regression contract: LTR-
    // only documents must not gain any markers anywhere. ~keep
    for s in [
        "",
        "Hello, world!",
        "The article is about greetings, page 42.",
        "Multiple\nlines\nstay clean",
        "Numbers 123 and punctuation: !?.,;",
    ] {
        assert_eq!(wrap_rtl_isolates(s, false), s, "pure-LTR identity broken on {:?}", s);
    }
}

#[test]
fn wrap_rtl_isolates_rtl_run_in_ltr_block_gets_rli_pdi() {
    // Hebrew phrase embedded in English — expect U+2067 (RLI)
    // before the Hebrew run and U+2069 (PDI) after it. The
    // canonical example from the release plan. ~keep
    let line = "The article שלום עולם is greetings.";
    let out = wrap_rtl_isolates(line, false);
    assert!(out.contains('\u{2067}'), "RLI missing in {:?}", out);
    assert!(out.contains('\u{2069}'), "PDI missing in {:?}", out);
    // No LRI (we're in an LTR block — LTR runs need no marker). ~keep
    assert!(!out.contains('\u{2066}'), "unexpected LRI in {:?}", out);
    let rli_idx = out.find('\u{2067}').expect("RLI present");
    let pdi_idx = out.find('\u{2069}').expect("PDI present");
    assert!(rli_idx < pdi_idx, "RLI must precede PDI in {:?}", out);
}

#[test]
fn wrap_rtl_isolates_ltr_run_in_rtl_block_gets_lri_pdi() {
    // English brand name embedded in a Hebrew sentence — expect
    // U+2066 (LRI) before the English run and U+2069 (PDI) after. ~keep
    let line = "הספר Microsoft חדש";
    let out = wrap_rtl_isolates(line, true);
    assert!(out.contains('\u{2066}'), "LRI missing in {:?}", out);
    assert!(out.contains('\u{2069}'), "PDI missing in {:?}", out);
    // RLI must NOT appear — we're in an RTL block, RTL runs are
    // unmarked. ~keep
    assert!(!out.contains('\u{2067}'), "unexpected RLI in {:?}", out);
    let lri_idx = out.find('\u{2066}').expect("LRI present");
    let pdi_idx = out.find('\u{2069}').expect("PDI present");
    assert!(lri_idx < pdi_idx, "LRI must precede PDI in {:?}", out);
}

#[test]
fn wrap_rtl_isolates_pure_rtl_in_rtl_block_is_identity() {
    // All-Hebrew line in an RTL block — no LTR runs to isolate,
    // byte-identical output. ~keep
    let line = "שלום עולם";
    assert_eq!(wrap_rtl_isolates(line, true), line);
}

#[test]
fn wrap_rtl_isolates_no_double_wrap_on_repeated_runs() {
    // Two separate Hebrew runs in one English line — each wrapped
    // independently with its own RLI/PDI pair. ~keep
    let line = "First שלום middle עולם last";
    let out = wrap_rtl_isolates(line, false);
    let rli_count = out.chars().filter(|&c| c == '\u{2067}').count();
    let pdi_count = out.chars().filter(|&c| c == '\u{2069}').count();
    assert_eq!(rli_count, 2, "expected 2 RLIs in {:?}", out);
    assert_eq!(pdi_count, 2, "expected 2 PDIs in {:?}", out);
}

#[test]
fn wrap_rtl_isolates_preserves_char_count_modulo_markers() {
    // The wrapped output must contain every original char exactly
    // once — markers are additive, never destructive. ~keep
    let line = "abc שלום def";
    let out = wrap_rtl_isolates(line, false);
    let stripped: String = out
        .chars()
        .filter(|c| !matches!(*c, '\u{2066}' | '\u{2067}' | '\u{2068}' | '\u{2069}'))
        .collect();
    assert_eq!(stripped, line);
}
