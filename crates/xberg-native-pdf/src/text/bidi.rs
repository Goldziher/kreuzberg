//! Unicode Bidirectional Algorithm (UAX #9) helpers for PDF text
//! extraction.
//!
//! Extracted PDF text can contain Arabic and Hebrew runs in either
//! *visual order* (typical of older Acrobat outputs and a few
//! tagged-PDF flows) or *logical order* (the common case for tools
//! that explicitly post-process to Unicode logical order, including
//! the pdfium `hebrew_mirrored.pdf` test fixture). The PDF
//! specification does not constrain which order a producer chooses;
//! callers must know which case they have before reordering.
//!
//! This module is a thin wrapper around the `unicode-bidi` crate
//! (UAX #9 implementation). It exposes the operations the converters
//! actually need:
//! - `looks_rtl(text)` — quick yes/no check for whether `text` contains
//!   any RTL characters worth running the bidi algorithm against.
//! - `reorder_visual_to_logical(text)` — given a single visual-order
//!   line, returns the logical-order string with embedded LTR runs
//!   (numerals, English words) preserved in their natural reading
//!   direction. **Caller is responsible for knowing the input is in
//!   visual order.** The default markdown converter does NOT call
//!   this for that reason.
//! - `paragraph_is_rtl(text)` — dominant paragraph direction per UAX
//!   #9 §3.3.1 (level of the first strong character).
//!
//! Background: the `right_to_left_02` fixture is an
//! Arabic government document where xberg-native-pdf previously inserted
//! spurious `**bold**` markers around individual letters because
//! contextual glyph forms (initial / medial / final shapes) flipped
//! the font-weight detector. The markdown converter strips those
//! markers (see `pipeline::converters::markdown::strip_inline_emphasis_in_rtl`)
//! while leaving order alone.

#![forbid(unsafe_code)]

use unicode_bidi::{BidiInfo, Level};

/// Cheap pre-check: does `text` look like it contains any RTL
/// characters? Used by the converter to skip the bidi pass entirely
/// for pure-LTR pages (the common case).
///
/// Delegates to `crate::text::rtl_detector::is_rtl_text` so the
/// authoritative list of supported RTL Unicode ranges (Hebrew,
/// Arabic main, Arabic Supplement, Arabic Extended-A, Arabic
/// Presentation Forms-A and -B) lives in exactly one place. A
/// previous inline copy of those ranges in this module risked
/// silent drift when one was updated and the other was not.
pub fn looks_rtl(text: &str) -> bool {
    text.chars().any(|c| crate::text::rtl_detector::is_rtl_text(c as u32))
}

/// Reorder a single line of visual-order text into logical order using
/// UAX #9. Returns the original string when no RTL characters are
/// present (fast path).
///
/// Per UAX #9 §3.3.4 (Reordering), embedded LTR runs (digits, Latin
/// words) inside an RTL paragraph are kept in their natural left-to-
/// right direction; only the surrounding RTL runs are reversed to
/// match the paragraph direction.
pub fn reorder_visual_to_logical(text: &str) -> String {
    if !looks_rtl(text) {
        return text.to_string();
    }
    // Default paragraph direction left to UAX #9 to infer from the
    // first strong character; this matches what PDF readers (and
    // pdftotext) do for mixed-direction lines. ~keep
    let info = BidiInfo::new(text, None);
    if info.paragraphs.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    for para in &info.paragraphs {
        let line_range = para.range.clone();
        let line = info.reorder_line(para, line_range);
        out.push_str(&line);
    }
    out
}

/// Whether the *dominant* paragraph direction of `text` is RTL,
/// computed per UAX #9 §3.3.1 from the level of the first strong
/// character in the first paragraph. Mixed-direction strings whose
/// first strong char is LTR (e.g. an English label followed by an
/// Arabic value) report as LTR even though they contain RTL chars.
pub fn paragraph_is_rtl(text: &str) -> bool {
    if !looks_rtl(text) {
        return false;
    }
    let info = BidiInfo::new(text, None);
    info.paragraphs.first().map(|p| p.level.is_rtl()).unwrap_or(false)
}

/// Is `c` a digit that participates as an embedded left-to-right
/// sub-run inside an RTL line — either a European digit (`0`–`9`,
/// ASCII U+0030..U+0039) or an Arabic-Indic / Extended Arabic-Indic
/// digit (U+0660..U+0669, U+06F0..U+06F9)? Even in an RTL paragraph
/// these read left-to-right (UAX #9 §3.3.3 W2 + §3.3.4 L1/L2): the
/// digit *sequence* keeps ascending order.
fn is_bidi_digit(c: char) -> bool {
    let cp = c as u32;
    c.is_ascii_digit()
        || (0x0660..=0x0669).contains(&cp) // Arabic-Indic ~keep
        || (0x06F0..=0x06F9).contains(&cp) // Extended Arabic-Indic ~keep
}

/// Is `c` a Latin letter (the other source of an embedded LTR sub-run
/// inside an RTL line)? ASCII fast path plus the Latin-1 / Latin
/// Extended ranges that cover accented Latin (e.g. `é`, `ï`).
fn is_latin_letter(c: char) -> bool {
    if c.is_ascii_alphabetic() {
        return true;
    }
    let cp = c as u32;
    c.is_alphabetic()
        && ((0x00C0..=0x024F).contains(&cp) // Latin-1 Supp + Latin Extended-A/B ~keep
            || (0x1E00..=0x1EFF).contains(&cp)) // Latin Extended Additional ~keep
}

/// Whole-line UAX #9 §3.3.4 pass for a *confidently RTL* line that
/// also contains embedded LTR material (European / Arabic-Indic
/// numerals and/or Latin words) — e.g. the date `14 april 1434 ٤٣٤١`.
///
/// **Contract — the input is already in logical order.** The page-text
/// path has *already* produced logical-order codepoints upstream
/// (per-run visual/logical detection + the existing `.chars().rev()`
/// span passes), so this function must **not** re-reverse the RTL runs
/// — doing so would invert previously-correct output. Instead it treats
/// the line as logical order under an RTL paragraph level and applies
/// only the L1/L2 part of §3.3.4 that the per-run passes cannot
/// express: each maximal embedded **even-level (LTR)** sub-run (digits
/// and/or Latin letters, plus the neutral spaces resolved into that
/// level) is ordered left-to-right, while the already-logical
/// **odd-level (RTL)** runs stay exactly where they are.
///
/// # Gating (no-regression contract)
///
/// Returns `line` byte-for-byte unchanged unless **both**:
/// 1. [`paragraph_is_rtl`] — the first strong char is RTL (UAX #9
///    §3.3.1), so the line is confidently RTL-dominant; ambiguous or
///    LTR-first lines are left alone.
/// 2. The line contains at least one bidi digit or Latin letter (the
///    *mixed* condition) — pure-RTL lines have no embedded LTR sublevel
///    to fix and are returned identical, preserving the existing
///    `right_to_left_02` / Hebrew fixtures.
///
/// Character count is always preserved (the output is a permutation of
/// the input chars; no glyph is dropped, duplicated, or substituted).
pub(crate) fn reorder_mixed_rtl_line(line: &str) -> String {
    if !paragraph_is_rtl(line) {
        return line.to_string();
    }
    let has_embedded_ltr = line.chars().any(|c| is_bidi_digit(c) || is_latin_letter(c));
    if !has_embedded_ltr {
        return line.to_string();
    }

    // Resolve per-char embedding levels under an explicit RTL paragraph
    // base. `Some(Level::rtl())` pins the paragraph direction so digits
    // and Latin words next to Arabic resolve to an *even* (LTR) level
    // and the Arabic/Hebrew resolves to an *odd* (RTL) level — exactly
    // the §3.3.4 levels we need, without `reorder_line`'s full
    // logical→visual flip (which would re-reverse our already-logical
    // RTL runs). ~keep
    let info = BidiInfo::new(line, Some(Level::rtl()));
    let chars: Vec<char> = line.chars().collect();
    // `levels` is indexed by UTF-8 byte offset; map it to char indices. ~keep
    if info.levels.len() != line.len() {
        // Defensive: shape mismatch — leave the line untouched. ~keep
        return line.to_string();
    }
    let mut char_levels: Vec<Level> = Vec::with_capacity(chars.len());
    {
        let mut byte = 0usize;
        for c in &chars {
            char_levels.push(info.levels[byte]);
            byte += c.len_utf8();
        }
    }

    // Walk the line; keep odd-level (RTL) chars fixed in place, and for
    // each maximal even-level (LTR) sub-run order it strictly
    // left-to-right by logical index (ascending). Because the input is
    // already logical, a correctly-emitted LTR sub-run is already
    // ascending and this is a no-op for it; a sub-run an upstream pass
    // accidentally left in RTL-visual order is straightened here. RTL
    // runs are emitted verbatim, so already-logical RTL order is never
    // disturbed. ~keep
    let mut out = String::with_capacity(line.len());
    let mut i = 0usize;
    while i < chars.len() {
        if char_levels[i].is_rtl() {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        // Maximal even-level (LTR) sub-run [i, j). ~keep
        let mut j = i;
        while j < chars.len() && char_levels[j].is_ltr() {
            j += 1;
        }
        for &c in &chars[i..j] {
            out.push(c);
        }
        i = j;
    }
    out
}

/// Verdict of the geometric visual-vs-logical detector.
///
/// Returned by [`detect_visual_order_run`] for a contiguous RTL run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunOrder {
    /// The PDF content stream emitted the run in **visual order** —
    /// glyphs were drawn left-to-right in user space even though the
    /// script reads right-to-left. The caller should apply UAX #9
    /// reordering ([`reorder_visual_to_logical`]) — or the simpler
    /// per-run `.chars().rev()` reversal — to produce logical-order
    /// codepoints for downstream RAG / search / display consumers.
    Visual,
    /// The PDF content stream emitted the run in **logical order**.
    /// Chars are placed right-to-left in user space (because the
    /// producer ran its own bidi pass before drawing), so the
    /// extracted codepoint sequence already matches reading order.
    /// The caller must NOT reorder — doing so would invert the run
    /// and break previously-correct output. The pdfium
    /// `hebrew_mirrored.pdf` test fixture is the canonical example.
    Logical,
    /// Insufficient signal to decide — sparse positions, ties,
    /// mixed direction, or the run is too short. The caller's safe
    /// default is to leave the run alone (the historical behaviour).
    Ambiguous,
}

/// Geometric visual-vs-logical detector for a single RTL run.
///
/// Closes the long-standing Hebrew gap captured in
/// `pipeline/converters/markdown.rs:1798-1812`: the bidi machinery
/// is already wired (UAX #9 via `unicode-bidi`, [`reorder_visual_to_logical`])
/// but the markdown converter explicitly does *not* call it because
/// some PDFs store text in visual order and some in logical order,
/// and "without a reliable way to detect which order the source uses
/// we drop the reorder step." This function is that reliable way.
///
/// # Inputs
///
/// `chars_with_x` — a slice of `(codepoint, x_origin_in_user_space)`
/// pairs for the characters that make up the run, in **content-stream
/// order** (i.e. the order the PDF's `Tj`/`TJ` operator emitted them).
/// The `x_origin` is the *user-space* x-coordinate where each glyph
/// was drawn — after `Tm` (text matrix) and `CTM` (current
/// transformation matrix) have been applied. Callers that have only
/// text-space coordinates must transform first; the detector relies
/// on monotonicity in the page's visible coordinate system.
///
/// Whitespace, diacritics, and presentation forms are filtered out
/// before the monotonicity check (they're noise for direction
/// detection).
///
/// # Algorithm
///
/// 1. Require **≥ 4 RTL letters** in the run. Short runs are noise.
/// 2. Bail with [`RunOrder::Ambiguous`] if the run contains any
///    **Arabic Presentation Forms** (U+FB50-U+FDFF, U+FE70-U+FEFF).
///    Those are already handled by the existing Pass 0 of
///    `document::PdfDocument::reverse_rtl_visual_order_runs`, and
///    second-guessing it here would risk double-reversal.
/// 3. Compare adjacent x-coordinates with a `0.5pt` kerning
///    tolerance:
///    - **ascending** (chars placed left-to-right) → visual signal.
///    - **descending** (chars placed right-to-left) → logical signal.
///    - **tie** (within 0.5pt) → no signal for this pair.
/// 4. Require **≥ 90 % monotonicity** (`asc / total > 0.9` or
///    `desc / total > 0.9`) to return [`RunOrder::Visual`] or
///    [`RunOrder::Logical`]. Below threshold → [`RunOrder::Ambiguous`].
///
/// The 90 % floor is deliberately strict: the cost of an unwarranted
/// reversal (logical PDF → visual output) is higher than the cost of
/// a missed reversal (visual PDF → uncorrected output). When in
/// doubt, leave the run alone.
///
/// # Why X-monotonicity is the right signal
///
/// PDF content streams emit glyphs in the order they're drawn, with
/// absolute positions from `Tm` * `CTM` + offset. A visual-order
/// producer (legacy Acrobat, hand-shaped Arabic, the Magic Palace
/// Eilat hotel PDF) draws Hebrew left-to-right in
/// user space even though the script reads right-to-left — so the
/// first codepoint in the stream has the smallest x. A logical-order
/// producer (modern Word with bidi pass, the pdfium
/// `hebrew_mirrored.pdf` test fixture) draws Hebrew right-to-left,
/// so the first codepoint has the largest x. The geometric direction
/// is observable and unambiguous — confirmed by a W3C / PDFuzz /
/// library-by-library survey of visual-vs-logical detection approaches
/// across implementations.
pub(crate) fn detect_visual_order_run(chars_with_x: &[(char, f32)]) -> RunOrder {
    // Arabic Presentation Forms presence → Pass 0 owns this run.
    // Check against the *original* input so PF chars block us even
    // when the letter filter below would strip them. ~keep
    if chars_with_x.iter().any(|(c, _)| {
        let cp = *c as u32;
        (0xFB50..=0xFDFF).contains(&cp) || (0xFE70..=0xFEFF).contains(&cp)
    }) {
        return RunOrder::Ambiguous;
    }

    // Filter: keep RTL **letters** only. `is_rtl_text` matches the
    // whole Arabic/Hebrew script range and so would let diacritics and
    // presentation forms count toward the ≥4 threshold and skew the
    // monotonicity numerator — neither is direction signal. Explicit
    // letter checks match the documented algorithm. ~keep
    use crate::text::rtl_detector::{is_arabic_letter, is_hebrew_letter};
    let rtl: Vec<(char, f32)> = chars_with_x
        .iter()
        .copied()
        .filter(|(c, _)| {
            let cp = *c as u32;
            is_arabic_letter(cp) || is_hebrew_letter(cp)
        })
        .collect();

    if rtl.len() < 4 {
        return RunOrder::Ambiguous;
    }

    const KERN_TOL: f32 = 0.5; // points ~keep
    let mut asc: usize = 0;
    let mut desc: usize = 0;
    for w in rtl.windows(2) {
        let (_, x0) = w[0];
        let (_, x1) = w[1];
        let dx = x1 - x0;
        if dx > KERN_TOL {
            asc += 1;
        } else if dx < -KERN_TOL {
            desc += 1;
        }
        // |dx| <= KERN_TOL → tie, no contribution to either count. ~keep
    }
    let total = asc + desc;
    if total == 0 {
        // All ties — degenerate, no signal. ~keep
        return RunOrder::Ambiguous;
    }
    // 90 % monotonicity floor — strict-on-purpose so we never reorder
    // a logical-order PDF on a noisy signal.
    // Express as integer math: 10 * asc > 9 * total ↔ asc / total > 0.9. ~keep
    if 10 * asc > 9 * total {
        return RunOrder::Visual;
    }
    if 10 * desc > 9 * total {
        return RunOrder::Logical;
    }
    RunOrder::Ambiguous
}

/// Single decision point for "given this [`RunOrder`] verdict (from
/// [`detect_visual_order_run`]), should this flushed RTL run be reversed
/// to logical order?" — shared by every `Tj`/`TJ` buffer-flush site
/// (`flush_tj_buffer`, `flush_tj_span_buffer`, `cluster_to_span` in
/// `extractors/text.rs`).
///
/// Before this existed, each flush site independently re-implemented the
/// identical three-way `match` on the verdict — and one site
/// (`flush_tj_buffer`, the default `WordBoundaryMode::Tiebreaker` path)
/// never called the geometric detector at all, only its coarse fallback.
/// That fallback (`accumulated_width > 0.0`, a sum of *only positive*
/// glyph advance widths — TJ kerning offsets never subtract from it) is
/// true for nearly every non-empty RTL buffer, so that site was
/// unconditionally reversing every RTL run it flushed rather than
/// detecting direction at all (the invisible-OCR mirroring regression:
/// an already-logical-order OCR word got wrongly flipped, and the
/// flipped-but-still-pure-Hebrew word
/// then had its span order *also* reversed by
/// `document::PdfDocument::reverse_rtl_visual_order_runs`, compounding
/// into a full mirror image of the correct line).
///
/// Callers remain responsible for deciding whether they have usable
/// per-character geometry to hand `detect_visual_order_run` (that gating
/// differs slightly per site — e.g. `cluster_to_span` tolerates a
/// `chars_with_x` shorter than the decoded text on ligature expansion)
/// and for computing `coarse_visual_order_heuristic`, the best coarse
/// direction signal available (e.g. "buffer's net horizontal advance is
/// positive" / "last glyph's x > first glyph's x") for the genuinely
/// [`RunOrder::Ambiguous`] case (short runs, sparse geometry).
///
/// `is_invisible_render_mode` — was this run drawn with text render mode
/// `3`/`7` (ISO 32000-1 §9.3.6, "neither fill nor stroke" — the mode
/// OCR-sandwich text layers use so the recognized text is searchable but
/// the original scan stays the only visible thing)? Both `verdict` and
/// `coarse_visual_order_heuristic` are built on the same premise —
/// glyphs placed at *ascending* x are visual order, *descending* x are
/// logical order — which only holds when the producer has
/// rendering-correctness pressure to mirror already-logical RTL content
/// so it *looks* right on the page. Invisible text has no such pressure:
/// a competent OCR engine (the modern default — Tesseract's LSTM model,
/// cloud OCR APIs) outputs correct per-word logical Unicode but has no
/// reason to also mirror the invisible glyph *positions*, so it places
/// them at plain ascending x — geometrically identical to genuinely
/// visual-order content. Neither `verdict` nor the coarse fallback can
/// tell the two apart in that case (the invisible-OCR mirroring
/// regression: this was exactly why wiring the geometric detector into
/// every flush site alone didn't fix the bug — an already-correct OCR
/// word was still being flipped). When
/// `true`, skip both signals entirely and trust the extracted content
/// order as-is — the modern-OCR prior plus this crate's own stated
/// design principle (`detect_visual_order_run`'s doc: "the cost of an
/// unwarranted reversal is higher than the cost of a missed reversal")
/// both favor not reversing when the signal is this unreliable.
/// Cross-word reading order for invisible OCR lines is still corrected
/// separately, by the span-position-based reading-order sort and
/// `document::PdfDocument::reverse_rtl_visual_order_runs` Pass 0.5, which
/// this function has no bearing on.
///
/// Known accepted trade-off: an invisible OCR layer from a genuinely
/// non-language-aware/legacy engine that emits *visual*-order Unicode
/// will no longer get auto-reversed — today it coincidentally does, for
/// the same ascending-x-is-uninformative reason behind the invisible-OCR
/// mirroring regression described above. This is
/// a deliberate choice favoring the now-dominant case.
pub(crate) fn apply_rtl_verdict(
    text: &str,
    verdict: RunOrder,
    coarse_visual_order_heuristic: bool,
    is_invisible_render_mode: bool,
) -> String {
    if is_invisible_render_mode {
        return text.to_string();
    }
    match verdict {
        RunOrder::Visual => reverse_rtl_keep_numbers(text),
        RunOrder::Logical => text.to_string(),
        RunOrder::Ambiguous => {
            if coarse_visual_order_heuristic {
                reverse_rtl_keep_numbers(text)
            } else {
                text.to_string()
            }
        }
    }
}

/// Unicode bidi-isolation markers (UAX #9 §2.4).
///
/// These four code points isolate a directional run from the
/// surrounding paragraph, preventing the Unicode Bidirectional
/// Algorithm from re-ordering neutral characters (parentheses, commas,
/// spaces) across the boundary.
///
/// Crate-internal only: not part of the public Rust API and explicitly
/// excluded from the generated C header (`pub(crate)` prevents
/// cbindgen from re-emitting these as `#define` macros in
/// any FFI-facing binding).
pub(crate) mod isolation {
    /// U+2066 LEFT-TO-RIGHT ISOLATE — wraps an LTR run inside an RTL
    /// paragraph (e.g. an English brand name embedded in Hebrew prose).
    pub(crate) const LRI: char = '\u{2066}';
    /// U+2067 RIGHT-TO-LEFT ISOLATE — wraps an RTL run inside an LTR
    /// paragraph (e.g. a Hebrew phrase embedded in English prose).
    pub(crate) const RLI: char = '\u{2067}';
    /// U+2068 FIRST STRONG ISOLATE — wraps an ambiguous run whose
    /// direction is inferred from its first strong character (UAX #9
    /// §2.4.2). Used when neither side is confidently RTL or LTR.
    #[allow(dead_code)]
    pub(crate) const FSI: char = '\u{2068}';
    /// U+2069 POP DIRECTIONAL ISOLATE — closes the innermost open
    /// isolate (LRI / RLI / FSI).
    pub(crate) const PDI: char = '\u{2069}';
}

/// Per-char strong-direction classification used by
/// [`wrap_rtl_isolates`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharDir {
    /// Strong RTL letter (Hebrew, Arabic, Arabic Supplement,
    /// Arabic Extended-A, Arabic Presentation Forms).
    Rtl,
    /// Strong LTR letter (Latin, Greek, Cyrillic, CJK, etc.).
    Ltr,
    /// Neutral / weak — whitespace, digits, punctuation, ASCII
    /// numerals. Inherits direction from surrounding strong chars.
    Neutral,
}

fn classify(c: char) -> CharDir {
    let cp = c as u32;
    if crate::text::rtl_detector::is_rtl_text(cp) {
        return CharDir::Rtl;
    }
    if c.is_alphabetic() {
        return CharDir::Ltr;
    }
    CharDir::Neutral
}

/// Wrap directional runs in `text` with Unicode bidi-isolation
/// markers (UAX #9 §2.4) so that surrounding paragraph context cannot
/// re-order neutral characters across the run boundary.
///
/// The function scans `text` once, grouping contiguous chars by their
/// strong direction (RTL / LTR / Neutral; neutrals are absorbed into
/// the surrounding strong run). When a run's direction differs from
/// `block_is_rtl`, the run is wrapped with the appropriate isolate
/// markers:
///
/// - `block_is_rtl == false`: RTL runs wrapped with `U+2067` (RLI) …
///   `U+2069` (PDI). LTR runs left bare (they match the block
///   direction).
/// - `block_is_rtl == true`: LTR runs wrapped with `U+2066` (LRI) …
///   `U+2069` (PDI). RTL runs left bare.
///
/// Pure-direction strings (all chars match the block direction, or
/// the string has no strong chars at all) are returned untouched. The
/// caller may safely call this on every markdown span — the cost on a
/// pure-LTR English string is one strong-char scan with no
/// allocation.
///
/// This is the markdown-emission-side companion to
/// `detect_visual_order_run` (private). The detector decides which content-
/// stream runs to re-order at extraction time so the output text is
/// in logical order; this function decides which logical-order runs
/// to isolate at markdown-emission time so that downstream UAX #9
/// renderers (Pandoc, GitHub, VS Code preview, Obsidian) don't
/// re-shuffle neutrals across the boundary.
///
/// Markdown output only — `extract_text` and other plain-text
/// converters MUST NOT call this. Plain-text consumers do not honour
/// UAX #9 and would render the markers as literal garbage.
pub fn wrap_rtl_isolates(text: &str, block_is_rtl: bool) -> String {
    if text.is_empty() {
        return String::new();
    }
    // Fast path: no RTL chars at all and block is LTR → no wrapping
    // possible. Same on the symmetric side. This keeps pure-LTR
    // documents byte-identical to the pre-fix output. ~keep
    let has_rtl = looks_rtl(text);
    if !block_is_rtl && !has_rtl {
        return text.to_string();
    }
    let has_ltr = text.chars().any(|c| classify(c) == CharDir::Ltr);
    if block_is_rtl && !has_ltr {
        return text.to_string();
    }

    // Build runs: contiguous chars with same strong direction.
    // Neutrals attach to the previous strong run; if a neutral leads
    // the string, it attaches to the first strong run that follows. ~keep
    let chars: Vec<char> = text.chars().collect();
    let mut runs: Vec<(CharDir, Vec<char>)> = Vec::new();
    let mut pending_neutrals: Vec<char> = Vec::new();
    for c in chars {
        let dir = classify(c);
        match dir {
            CharDir::Neutral => {
                if let Some(last) = runs.last_mut() {
                    last.1.push(c);
                } else {
                    pending_neutrals.push(c);
                }
            }
            CharDir::Rtl | CharDir::Ltr => {
                if let Some(last) = runs.last_mut()
                    && last.0 == dir
                {
                    last.1.push(c);
                    continue;
                }
                let mut buf = std::mem::take(&mut pending_neutrals);
                buf.push(c);
                runs.push((dir, buf));
            }
        }
    }
    // Trailing-only-neutrals input (no strong chars at all) — return
    // as-is; nothing to isolate. ~keep
    if runs.is_empty() {
        return text.to_string();
    }
    // If pending_neutrals was never absorbed (only happens when the
    // text starts with neutrals AND has no strong chars at all, which
    // is already handled above) — fold them back into the first run
    // for safety. ~keep
    if !pending_neutrals.is_empty() {
        let mut tail = pending_neutrals;
        runs[0].1.append(&mut tail);
    }

    let mut out = String::with_capacity(text.len() + runs.len() * 6);
    for (dir, run_chars) in runs {
        let run_text: String = run_chars.into_iter().collect();
        match (block_is_rtl, dir) {
            (false, CharDir::Rtl) => {
                out.push(isolation::RLI);
                out.push_str(&run_text);
                out.push(isolation::PDI);
            }
            (true, CharDir::Ltr) => {
                out.push(isolation::LRI);
                out.push_str(&run_text);
                out.push(isolation::PDI);
            }
            _ => {
                out.push_str(&run_text);
            }
        }
    }
    out
}

/// Reverse a visual-order RTL run to logical order while keeping embedded
/// number sequences in their natural left-to-right order — UAX #9 rule L2: a
/// run of digits forms an even (LTR) embedding level inside RTL text and is
/// therefore not mirrored when the surrounding RTL is reversed. A separator
/// (`.` `,` `:` and the Arabic decimal/thousands separators) is treated as part
/// of the number only when it sits between two digits, so `1,000` and `3.14`
/// stay intact while a trailing comma reverses as an ordinary neutral. A
/// European terminator — `%`, a currency sign, a degree sign — joins the number
/// whenever it touches one (UAX #9 rule W5), so `50%` keeps its sign on the
/// side the reader expects instead of reversing away from the digits.
///
/// For any run containing no digits this is byte-identical to a plain
/// `chars().rev().collect()`, so the digit-free RTL case (the corpus-validated
/// common path) is unchanged; only digit-bearing runs — where a whole-string
/// reversal would wrongly emit `2009` as `9002` — are corrected.
pub fn reverse_rtl_keep_numbers(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let is_digit = |c: char| {
        c.is_ascii_digit()
            || ('\u{0660}'..='\u{0669}').contains(&c) // Arabic-Indic ~keep
            || ('\u{06F0}'..='\u{06F9}').contains(&c) // Extended Arabic-Indic ~keep
    };
    let is_sep = |c: char| matches!(c, '.' | ',' | ':' | '\u{066B}' | '\u{066C}');
    // Mark which positions belong to a maximal number run (digit (sep digit)*). ~keep
    let mut in_num = vec![false; n];
    let mut i = 0;
    while i < n {
        if is_digit(chars[i]) {
            let start = i;
            let mut j = i + 1;
            loop {
                if j < n && is_digit(chars[j]) {
                    j += 1;
                } else if j + 1 < n && is_sep(chars[j]) && is_digit(chars[j + 1]) {
                    j += 2;
                } else {
                    break;
                }
            }
            for slot in in_num.iter_mut().take(j).skip(start) {
                *slot = true;
            }
            i = j;
        } else {
            i += 1;
        }
    }
    // UAX #9 W5: a sequence of European terminators adjacent to a European
    // number becomes part of that number, so a sign touching the digits travels
    // with them instead of reversing away as a neutral. The class comes from
    // the `unicode-bidi` tables this module wraps rather than a second local
    // list of sign characters.
    let is_terminator = |c: char| unicode_bidi::bidi_class(c) == unicode_bidi::BidiClass::ET;
    let mut i = 0;
    while i < n {
        if !in_num[i] {
            i += 1;
            continue;
        }
        let mut before = i;
        while before > 0 && is_terminator(chars[before - 1]) {
            before -= 1;
            in_num[before] = true;
        }
        while i < n && in_num[i] {
            i += 1;
        }
        while i < n && is_terminator(chars[i]) {
            in_num[i] = true;
            i += 1;
        }
    }
    // Walk right-to-left, emitting number runs forward and every other char
    // reversed (identical to a plain reversal when no number run is present). ~keep
    let mut out: Vec<char> = Vec::with_capacity(n);
    let mut i = n;
    while i > 0 {
        i -= 1;
        if in_num[i] {
            let end = i + 1;
            while i > 0 && in_num[i - 1] {
                i -= 1;
            }
            out.extend_from_slice(&chars[i..end]);
        } else {
            out.push(chars[i]);
        }
    }
    out.into_iter().collect()
}

#[cfg(test)]
mod tests;
