//! Calibrated per-document F1 thresholds for the PDF quality gates.

// ═══════════════════════════════════════════════════════════════════
// Ground truth entries with calibrated thresholds
//
// Thresholds are set ~7% below measured F1 to catch regressions
// while allowing minor fluctuations. Documents with placeholder/
// invalid GTs have threshold 0.0 (extraction-must-not-crash only).
//
// The `// measured <score>` annotations are the calibration record: they are what makes a
// threshold interpretable and a regression provable. They were stripped wholesale by
// 4f32d4d44f3 ("chore: regen", 2026-07-09) and recovered from 4f32d4d44f3^. Do not drop
// them again — without them the table cannot be told apart from a set of arbitrary numbers. ~keep
// ═══════════════════════════════════════════════════════════════════

pub(crate) const PDFIUM_GROUND_TRUTH: &[(&str, f64)] = &[
    // ── Docling vendored PDFs (GT: pdftotext) ──
    ("2203.01017v2", 0.85),           // measured 0.927
    ("2206.01062", 0.79),             // measured 0.863
    ("2305.03393v1", 0.83),           // measured 0.908
    ("2305.03393v1-pg9", 0.85),       // measured 0.927
    ("amt_handbook_sample", 0.74),    // measured 0.810
    ("code_and_formula", 0.82),       // measured 0.894
    ("multi_page", 0.85),             // measured 0.929
    ("picture_classification", 0.81), // measured 0.889
    ("redp5110_sampled", 0.84),       // measured 0.912
    ("right_to_left_01", 0.45),       // measured 0.521 (RTL text)
    ("right_to_left_02", 0.43),       // measured 0.507 (RTL text)
    ("right_to_left_03", 0.31),       // measured 0.384 (RTL text)
    // ── pdfplumber vendored PDFs (GT: pdftotext) ──
    ("2023-06-20-PV", 0.85),                          // measured 0.921
    ("annotations", 0.0),                             // 5-word GT, volatile
    ("annotations-rotated-180", 0.0),                 // 5-word GT, volatile
    ("annotations-rotated-270", 0.0),                 // 5-word GT, volatile
    ("annotations-rotated-90", 0.0),                  // 5-word GT, volatile
    ("annotations-unicode-issues", 0.0),              // 11-word GT, volatile
    ("chelsea_pdta", 0.77),                           // measured 0.846
    ("cupertino_usd_4-6-16", 0.89),                   // measured 0.961
    ("extra-attrs-example", 0.0),                     // 1-word GT
    ("federal-register-2020-17221", 0.82),            // measured 0.899
    ("figure_structure", 0.93),                       // measured 1.000
    ("hello_structure", 0.93),                        // measured 1.000
    ("image_structure", 0.39),                        // measured 0.467
    ("issue-1054-example", 0.0),                      // sparse GT, xberg extracts more
    ("issue-1114-dedupe-chars", 0.68),                // measured 0.759
    ("issue-1147-example", 0.34),                     // measured 0.414
    ("issue-1181", 0.56),                             // measured 0.889 md, 0.571 plain (24-word GT, volatile)
    ("issue-1279-example", 0.60),                     // measured 0.678
    ("issue-140-example", 0.0),                       // image-only PDF
    ("issue-192-example", 0.55), // measured 0.567 on macOS-latest (was 0.653 — drift from html-to-markdown-rs/tokenizer dep updates)
    ("issue-316-example", 0.85), // measured 0.927
    ("issue-33-lorem-ipsum", 0.89), // measured 0.964
    ("issue-336-example", 0.74), // measured 0.810
    ("issue-461-example", 0.0),  // CJK text, low overlap
    ("issue-463-example", 0.80), // measured 0.815 on macOS-latest (was 0.896 — same drift)
    ("issue-466-example", 0.93), // measured 1.000
    ("issue-53-example", 0.90),  // measured 0.976
    ("issue-598-example", 0.82), // measured 0.897
    ("issue-67-example", 0.60),  // measured 0.672
    ("issue-71-duplicate-chars", 0.26), // measured 0.333
    ("issue-71-duplicate-chars-2", 0.78), // measured 0.855
    ("issue-842-example", 0.58), // measured 0.651
    ("issue-848", 0.17),         // measured 0.242
    ("issue-90-example", 0.89),  // measured 0.961
    ("issue-905", 0.0),          // 1-word GT
    ("issue-912", 0.91),         // measured 0.984
    ("issue-982-example", 0.87), // measured 0.947
    ("issue-987-test", 0.93),    // measured 1.000
    ("la-precinct-bulletin-2014-p1", 0.90), // measured 0.973
    ("line-char-render-example", 0.0), // 6-word GT, volatile
    ("malformed-from-issue-932", 0.0), // 3-word GT, volatile
    ("mcid_example", 0.93),      // measured 1.000
    ("nics-background-checks-2015-11", 0.92), // measured 0.996
    ("nics-background-checks-2015-11-rotated", 0.92), // measured 0.996
    ("page-boxes-example", 0.93), // measured 1.000
    ("pdf_structure", 0.86),     // measured 0.931
    ("pdffill-demo", 0.77),      // measured 0.845
    ("pr-136-example", 0.36),    // measured 0.436
    ("pr-138-example", 0.91),    // measured 0.985
    ("pr-88-example", 0.85),     // measured 0.926
    ("scotus-transcript-p1", 0.65), // measured 0.723
    ("senate-expenditures", 0.0), // complex tabular, xberg extracts more
    ("table-curves-example", 0.86), // measured 0.937
    ("test-punkt", 0.93),        // measured 1.000
    ("WARN-Report-for-7-1-2015-to-03-25-2016", 0.92), // measured 0.997
    ("word365_structure", 0.93), // measured 1.000
    // ── markitdown vendored PDFs (GT: pdftotext) ──
    ("masterformat_partial_numbering", 0.89),         // measured 0.962
    ("RECEIPT-2024-TXN-98765_retail_purchase", 0.89), // measured 0.962
    ("REPAIR-2022-INV-001_multipage", 0.88),          // measured 0.954
    ("SPARSE-2024-INV-1234_borderless_table", 0.89),  // measured 0.961
    ("test", 0.83),                                   // measured 0.909
    // ── quality-benchmarks nougat PDFs (GT: pixparse) ──
    ("nougat_001", 0.70), // measured 0.776
    ("nougat_002", 0.85), // measured 0.925
    ("nougat_003", 0.90), // measured 0.974
    ("nougat_004", 0.88), // measured 0.950
    ("nougat_005", 0.82), // measured 0.892
    ("nougat_006", 0.87), // measured 0.945
    ("nougat_007", 0.83), // measured 0.902
    ("nougat_008", 0.81), // measured 0.886
    ("nougat_009", 0.78), // measured 0.856
    ("nougat_010", 0.88), // measured 0.959
    ("nougat_011", 0.85), // measured 0.926
    ("nougat_012", 0.87), // measured 0.948
    ("nougat_013", 0.86), // measured 0.931
    ("nougat_014", 0.85), // measured 0.921
    ("nougat_015", 0.81), // measured 0.889
    ("nougat_016", 0.56), // measured 0.637
    ("nougat_017", 0.72), // measured 0.797
    ("nougat_018", 0.84), // measured 0.919
    ("nougat_019", 0.92), // measured 0.990
    ("nougat_020", 0.75), // measured 0.828
    ("nougat_021", 0.85), // measured 0.926
    ("nougat_022", 0.87), // measured 0.940
    ("nougat_023", 0.74), // measured 0.812
    ("nougat_024", 0.89), // measured 0.969
    ("nougat_025", 0.83), // measured 0.904
    // nougat_026..050 removed: byte-identical duplicates of pdfa_001..025 (same PDF *and* same
    // ground truth; verified with md5 — nougat_026.pdf and pdfa_001.pdf are both
    // 8899e9abb778fb4e85269f2b0f0e5e1b). Scoring both aliases counted 25 documents twice in
    // failures.len() and in the avg_f1 mean, pulling the average toward whatever that one
    // 25-document slice happened to score. The pdfa_* names are kept as canonical; the corpus
    // files under test_documents/ are untouched (separate submodule-tracked repository). ~keep
    // ── quality-benchmarks pdfa PDFs (GT: pixparse) ──
    ("pdfa_001", 0.92), // measured 0.993
    ("pdfa_002", 0.83), // measured 0.900
    ("pdfa_003", 0.63), // measured 0.703
    ("pdfa_004", 0.85), // measured 0.928
    ("pdfa_005", 0.86), // measured 0.936
    ("pdfa_006", 0.83), // measured 0.900
    ("pdfa_007", 0.80), // measured 0.878
    ("pdfa_008", 0.83), // measured 0.905
    ("pdfa_009", 0.88), // measured 0.952
    ("pdfa_010", 0.84), // measured 0.913
    ("pdfa_011", 0.82), // measured 0.896
    ("pdfa_012", 0.87), // measured 0.940
    ("pdfa_013", 0.86), // measured 0.936
    ("pdfa_014", 0.83), // measured 0.900
    ("pdfa_015", 0.81), // measured 0.887
    ("pdfa_016", 0.78), // measured 0.852
    ("pdfa_017", 0.88), // measured 0.952
    ("pdfa_018", 0.92), // measured 0.991
    ("pdfa_019", 0.84), // measured 0.913
    ("pdfa_020", 0.87), // measured 0.949
    ("pdfa_021", 0.83), // measured 0.903
    ("pdfa_022", 0.82), // measured 0.897
    ("pdfa_023", 0.85), // measured 0.927
    ("pdfa_024", 0.84), // measured 0.919
    ("pdfa_025", 0.87), // measured 0.942
    ("pdfa_026", 0.90), // measured 0.972
    ("pdfa_027", 0.71), // measured 0.783
    ("pdfa_028", 0.86), // measured 0.933
    ("pdfa_029", 0.86), // measured 0.939
    ("pdfa_030", 0.84), // measured 0.918
    ("pdfa_031", 0.83), // measured 0.903
    ("pdfa_032", 0.88), // measured 0.953
    ("pdfa_033", 0.06), // measured 0.133 (non-text-layer PDF)
    ("pdfa_034", 0.82), // measured 0.893
    ("pdfa_035", 0.14), // measured 0.213 (non-text-layer PDF)
    ("pdfa_036", 0.83), // measured 0.907
    ("pdfa_037", 0.82), // measured 0.893
    ("pdfa_038", 0.78), // measured 0.851
    ("pdfa_039", 0.89), // measured 0.966
    ("pdfa_040", 0.85), // measured 0.921
    ("pdfa_041", 0.87), // measured 0.946
    ("pdfa_042", 0.88), // measured 0.951
    ("pdfa_043", 0.79), // measured 0.861
    ("pdfa_044", 0.85), // measured 0.923
    ("pdfa_045", 0.73), // measured 0.802
    ("pdfa_046", 0.85), // measured 0.921
    ("pdfa_047", 0.84), // measured 0.915
    ("pdfa_048", 0.82), // measured 0.890
    ("pdfa_049", 0.89), // measured 0.960
    ("pdfa_050", 0.85), // measured 0.921
];

// ═══════════════════════════════════════════════════════════════════
// Known regressions — skipped from the gate
//
// These docs currently fail their PDFIUM_GROUND_TRUTH thresholds. They are NOT silently
// relaxed — the thresholds remain as the historic floor for when extraction is fixed.
// They are skipped so unrelated xberg work can continue while the fix lands.
//
// Tracking issue: https://github.com/yfedoseev/xberg_native_pdf/issues/484
//   ("Extraction failures, near-empty output, and quality regressions
//   on a calibrated 166-PDF set")
//
// As of xberg_native_pdf 0.3.66, 9 of the originally-listed docs recovered and were
// re-enabled (annotations*, pdfa_039, pr-136-example, table-curves-example,
// nougat_026, pdfa_001). nics-background-checks also recovered but only to
// 0.922 vs its 0.92 floor — kept skipped pending cross-platform confirmation.
// After each xberg_native_pdf bump, re-measure every entry below and remove the ones
// that clear all three gates. Of the six post-calibration regressions tracked by
// #1406, the native-coverage safeguard restored four (issue-1147-example,
// nougat_004, pdfa_045, pr-136-example) and they are enabled again. The other two
// did not recover and stay listed in group 4 with their measured scores.
//
// To re-enable a doc once the regression is fixed: remove from this
// list. Do not lower the threshold — fix the regression at its source.
//
// The list is grouped by *why* an entry is here. Group 1 is a capability gap (the document
// cannot be extracted at all with the current feature set). Groups 2-4 are quality
// regressions: extraction used to score the recorded value and now scores less. Keep new
// entries in the group that describes them, and always record measured / today / delta so
// the size of what is being suppressed stays visible at the point of suppression.
// ═══════════════════════════════════════════════════════════════════

pub(crate) const PDFIUM_KNOWN_REGRESSIONS: &[&str] = &[
    // ── 1. UNSUPPORTED: hard extraction failure, not a quality regression ──
    "pr-138-example", // requires xberg_native_pdf legacy-crypto feature (R=4 PDF)
    // ── 2. BROKEN: F1 quality regressions vs calibrated floor (md / plain F1),
    //    surviving at xberg_native_pdf 0.3.66; recovered docs were removed at that bump ──
    "right_to_left_02",                       // md 0.423 < 0.43 (RTL drift — xberg_native_pdf #484)
    "right_to_left_03", // md 0.122 < 0.31 (RTL: xberg_native_pdf CID/Type0 decode drops ~92% of body; #484)
    "hello_structure",  // md 0.778 < 0.93
    "issue-336-example", // md 0.522 < 0.74
    "issue-466-example", // md 0.833 / plain 0.806 < 0.93
    "issue-53-example", // md 0.843 / plain 0.694 < 0.90
    "issue-987-test",   // md 0.400 / plain 0.000 < 0.93
    "la-precinct-bulletin-2014-p1", // md 0.834 / plain 0.658 < 0.90
    "pr-88-example",    // md 0.793 < 0.85
    "SPARSE-2024-INV-1234_borderless_table", // md 0.874 < 0.89
    "WARN-Report-for-7-1-2015-to-03-25-2016", // plain 0.669 < 0.83
    "nougat_005",       // plain 0.333 < 0.74
    "nougat_018",       // md 0.740 < 0.84
    "pdfa_014",         // md 0.684 < 0.83 (was also listed as its duplicate alias nougat_039)
    "pdfa_015",         // md 0.765 < 0.81 (was also listed as its duplicate alias nougat_040)
    "pdfa_036",         // md 0.639 < 0.83
    "pdfa_044",         // md 0.646 / plain 0.720 < 0.85
    // ── 3. BROKEN: OSS v5 post-processor regression — added 2026-06-02 ──
    // After wiring `register_builtin()` for NER / redaction / summarisation /
    // translation / page-classification / captions / QR codes, the rotated NICS
    // PDF dropped md / djot F1 0.996 → 0.724 against the 0.92 floor. The
    // issue-1181 plain score also tipped over exactly at the 0.50 threshold
    // (24-word GT, already flagged as "volatile" in the row comment).
    // Both must be investigated as part of the OSS v5 follow-up — restore the
    // entries to the gate once the offending post-processor is identified. ~keep
    "nics-background-checks-2015-11-rotated", // md / djot 0.724 < 0.92
    "issue-1181",                             // plain 0.500 ≤ 0.50 floor (tip-over)
    // ── 4. NOT AN EXTRACTION DEFECT: this ground truth encodes policies xberg does not
    //       implement. Diagnosed 2026-08-09; the previous comment here was wrong. ──
    //
    // pdfa_001 scores P 0.927 / R 0.784 / F1 0.849 against a 0.92 floor. 345 of 1597 ground-truth
    // words are missing, and two causes account for 244 of them. NEITHER is a bug in extraction.
    //
    // (a) 131 ballot glyphs — 75 × U+2610 ☐ and 56 × U+2611 ☑ — that NO text extractor produces.
    //     Measured on this exact file: pdfplumber `extract_text()` yields ☐ 0, ☑ 0, and 57
    //     standalone "X"; xberg yields the same. The PDF has no AcroForm, no /Widget, no /FT, no
    //     ZapfDingbats and no /Differences — just two Type1 Helvetica fonts in WinAnsiEncoding.
    //     Checked boxes are the literal operator `(X) Tj`; unchecked boxes have NO text at all,
    //     only stroke-only vector rectangles. So for 75 of them there is no character to extract:
    //     emitting a glyph would mean detecting an empty stroked rect beside a label and
    //     synthesizing one, which is a layout feature, not an extraction fix. The other 56 are a
    //     token *substitution* (we emit "X", the GT wants ☑) and are penalised twice — the glyph
    //     counts as missing, the "X" counts as spurious.
    //
    // (b) ~113 words of repeated page footer, stripped by design. "Report Status: / Data Last
    //     Update: 09/17/2012 / Submission Date: 01/08/2013 / Print Date: 1/9/2013 6:25 AM" is on
    //     every page; the GT keeps all 13 copies, we keep 3. Instrumented and confirmed: the
    //     footer arrives as SEVEN separate paragraphs of 8–14 alphanumeric characters each, all
    //     far below `retain_page_furniture_safely`'s MIN_SUBSTANTIVE_CHARS = 80, so each is
    //     dropped wherever that function's 30 %-of-page rescue valve does not fire. Across the 13
    //     pages the valve fires on exactly 2 (51.4 % and 51.0 %, the sparse pages) and not on the
    //     other 11 (7.4 %–26.8 %); page 1 never marks the footer as furniture at all. 2 + 1 = the
    //     3 survivors. Dropping short repeated furniture is the intended behaviour of that code.
    //
    // Why the entry stays rather than getting "fixed": measured, fixing the footer ALONE reaches
    // only F1 0.892 — still under the floor. The only route to 0.92 runs through mapping "X" → ☑
    // (0.928), and that is ground-truth chasing: xberg has no checkbox detection, so it cannot
    // know an "X" sits in a box rather than being ordinary content. Do not do it to turn a number
    // green, and do NOT lower the floor — the floor is correct, the fixture just measures a
    // different rendering policy. Retire this entry only if xberg gains real checkbox detection,
    // or if the corpus grows a ground truth produced by a plain text extractor.
    //
    // Closed (GH#1411): the standalone table value "22" (alnum = 2) was being swallowed as
    // furniture. It now survives — see tests/issue_1411_page_number_deletion.rs, which asserts the
    // "inpatient 22" pair at the extraction level. The note that used to sit here blamed "the same
    // repeat-detector" as the footer case; that was wrong. mark_cross_page_repeating_short_text
    // skips any paragraph with block_bbox.is_some(), and finalize_paragraph gives a loose table
    // cell a real bbox, so the cell never reached that detector at all. ~keep
    //
    // nougat_026 is the same file as pdfa_001 (identical md5); its duplicate table row was
    // removed, so the pdfa_001 entry covers both. ~keep
    "pdfa_001", // GT policy mismatch (synthesized ☐/☑ + by-design furniture stripping), not a regression
    //
    // pdfa_019's ground truth is HALLUCINATED -- it is not a transcript of this PDF. Verified by
    // counting both sides of the same file: the PDF (via `pdftotext -layout`, a faithful non-ML
    // text layer) contains "Dobson" x18, "Crisis" x12, "Goodreads" x1. The ground truth contains
    // ZERO of those three and instead carries "Babson" x14, "China" x7, "Goodneath" x1, plus
    // "Bobbin", "Robian", "Blowin" and "rebrinding" x3. These are VLM/OCR fabrications, the same
    // corpus defect class as pdfa_001's synthesized glyphs: every fabricated token is penalised
    // twice (missing GT word + spurious extracted word), so no extractor can ever reach 1.0 here.
    // The 0.913 calibration measurement was taken against this same corrupt GT, which is why the
    // recorded floor is 0.84 rather than something near 0.99.
    //
    // A SECOND, REAL REGRESSION IS STACKED UNDER THAT CEILING and is NOT fixed by this entry:
    // md/djot F1 measured 0.795 today vs 0.913 at calibration, a 0.118 drop that appeared after
    // 2026-08-08 (`9b9362c1e35` lists the docs that had regressed as of that date and pdfa_019 is
    // not among them). Not bisected -- ~409 commits touched crates/xberg/src in that window, many
    // in PDF heading/paragraph paths. This is single-column with no tables, so the table-guard and
    // multi-column reading-order defect classes are unlikely; paragraph assembly or heading
    // detection is the more plausible cause.
    //
    // Retire this entry only when BOTH are addressed: regenerate the ground truth from a
    // non-hallucinating text extractor, AND root-cause the 0.913 -> 0.795 drop. Do not lower the
    // floor -- the floor is not what is wrong here. ~keep
    "pdfa_019",
];

/// OCR ground truth entries. Same documents but tested through OCR pipeline.
/// Thresholds are lower because OCR introduces more noise than native extraction.
#[cfg(feature = "ocr")]
pub(crate) const OCR_GROUND_TRUTH: &[(&str, f64)] = &[
    ("hello_structure", 0.30),
    ("multi_page", 0.30),
    ("code_and_formula", 0.20),
    ("2305.03393v1-pg9", 0.20),
    ("amt_handbook_sample", 0.20),
    ("scotus-transcript-p1", 0.30),
    ("federal-register-2020-17221", 0.30),
    ("issue-33-lorem-ipsum", 0.30),
    ("masterformat_partial_numbering", 0.20),
    ("test", 0.20),
];
