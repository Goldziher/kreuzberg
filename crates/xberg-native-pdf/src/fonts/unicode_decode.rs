//! The glyph decoder shared by the extraction and rendering text paths.
//! (`TjBuffer`'s simple-font fast path and a few extraction loops still
//! decode inline.)
//!
//! `decode_text_to_unicode` and its helpers (`fallback_char_to_unicode`,
//! `get_byte_mode`, `TextCharIter`) used to exist twice — an extraction copy
//! in `extractors/text.rs` and a rendering copy in
//! `rendering/text_rasterizer.rs` — and the copies drifted in both
//! directions: UTF-8 codespace CMaps and `preserve_unmapped_glyphs` existed
//! only in extraction; ligature decomposition and dropped-glyph accounting
//! only in rendering. A Type0 font with a UTF-8 CMap extracted right and
//! rendered garbage. (Extraction ligatures were never lossy — they decompose
//! downstream in `ligature_processor`.) This module is the union; the
//! genuine policy differences are expressed in [`DecodePolicy`], not by
//! forking the decoder.

use std::collections::HashMap;

use crate::fonts::truetype_cmap::TrueTypeCMap;
use crate::fonts::{FontInfo, MappingProvenance};

/// Per-surface decoding policy — the only intended differences between the
/// extraction and rendering paths.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DecodePolicy {
    /// Keep U+FFFD for unmapped codes instead of dropping them.
    /// Extraction honours the `preserve_unmapped_glyphs()` toggle; rendering
    /// always drops (a U+FFFD has no glyph to paint).
    pub preserve_unmapped: bool,
    /// Expand presentation-form ligature code points (fi, fl, ffi, …) into
    /// component letters. The rasterizer needs this so the shaper doesn't
    /// drop the cluster; extraction decomposes downstream in
    /// `ligature_processor`, so it must stay off here or chars would change.
    pub decompose_ligatures: bool,
    /// Emit '?' for codes that are no valid Unicode scalar value
    /// (surrogate-range CIDs, multi-byte codes past U+10FFFF) instead of
    /// dropping them. Extraction has always printed '?' here and its output
    /// bytes are pinned by corpus diffs; rendering keeps this off — there is
    /// no glyph for such a code, so it is dropped and tallied like any other
    /// unmappable code.
    pub question_mark_for_invalid: bool,
}

/// Counts glyphs a decode dropped so the loss is reported, not silent.
#[derive(Debug, Default)]
pub(crate) struct GlyphDropTally {
    count: usize,
    first: Option<(&'static str, u32, u32)>,
}

impl GlyphDropTally {
    pub(crate) fn record(&mut self, reason: &'static str, char_code: u32, gid: u32) {
        self.count += 1;
        self.first.get_or_insert((reason, char_code, gid));
    }

    /// The reason recorded for the first drop, or `None` when nothing
    /// dropped. Callers latch on this so the same broken font reports once
    /// per reason rather than once per text run.
    pub(crate) fn reason(&self) -> Option<&'static str> {
        self.first.map(|(reason, _, _)| reason)
    }

    /// The warning for glyphs a paint path advanced past without drawing, or
    /// `None` when nothing dropped.
    ///
    /// Only the rasterizer reports (extraction's drops are policy-visible via
    /// `preserve_unmapped`). Build this only from a path that painted (or tried to
    /// paint) the recorded glyphs — see [`Self::warning_omitted`] for
    /// decode-stage misses.
    ///
    /// Whether to emit is the caller's call: the rasterizer owns a
    /// page-scoped latch, so this module holds no reporting state of its own.
    pub(crate) fn warning(&self, font_name: &str) -> Option<crate::extractors::warnings::Warning> {
        let (reason, char_code, gid) = self.first?;
        Some(crate::extractors::warnings::Warning {
            category: crate::extractors::warnings::WarningCategory::GlyphDropped,
            page: None,
            message: format!(
                "font '{font_name}' painted nothing for {} glyph(s) while advancing the cursor; \
                 first was code 0x{char_code:X} (glyph {gid}): {reason}. The page renders with \
                 a gap that reads as whitespace downstream. Reported once per font per page.",
                self.count
            ),
            spec_section: Some("9.6.6"),
        })
    }

    /// Decode-stage misses: the code had no Unicode mapping, so its glyph
    /// was omitted from the shaped text — nothing painted, nothing advanced.
    /// Only callers that actually render from the decoded string may build
    /// this; the CID-direct and CJK-substitution paths paint from the raw
    /// codes, and a decode miss says nothing about what they draw.
    pub(crate) fn warning_omitted(&self, font_name: &str) -> Option<crate::extractors::warnings::Warning> {
        let (reason, char_code, _) = self.first?;
        Some(crate::extractors::warnings::Warning {
            category: crate::extractors::warnings::WarningCategory::GlyphDropped,
            page: None,
            message: format!(
                "font '{font_name}' omitted {} glyph(s) from fallback shaping ({reason}); \
                 first was code 0x{char_code:X}. The run paints without these glyphs. \
                 Reported once per font per page.",
                self.count
            ),
            spec_section: Some("9.10.2"),
        })
    }
}

/// Fallback function to map common character codes to Unicode when ToUnicode CMap fails.
///
/// PDF Spec Compliance: ISO 32000-1:2008 Section 9.10.2
/// This function implements Priority 6 (enhanced fallback) after the standard 5-tier
/// encoding system (ToUnicode CMap, predefined encodings, Adobe Glyph List, etc.) fails.
///
/// Multi-tier fallback strategy:
/// 1. Common punctuation and symbols (em dash, en dash, quotes, bullets)
/// 2. Mathematical operators (∂, ∇, ∑, ∏, ∫, √, ∞, ≤, ≥, ≠)
/// 3. Greek letters (α, β, γ, δ, θ, λ, μ, π, σ, ω - both cases)
/// 4. Currency symbols (€, £, ¥, ¢)
/// 5. Direct Unicode (if char_code is in valid Unicode range)
/// 6. Private Use Area visual description (U+E000-U+F8FF)
/// 7. Replacement character "?" as last resort
///
/// # Arguments
/// * `char_code` - 16-bit character code that failed to decode via standard system
///
/// # Returns
/// Best-effort Unicode string representation, or "?" if no mapping possible
pub(crate) fn fallback_char_to_unicode(char_code: u32) -> String {
    if let Some(s) = fallback_typography_symbol(char_code)
        .or_else(|| fallback_math_symbol(char_code))
        .or_else(|| fallback_greek_letter(char_code))
        .or_else(|| fallback_currency_symbol(char_code))
    {
        return s.to_string();
    }

    if let Some(ch) = char::from_u32(char_code) {
        if (0xE000..=0xF8FF).contains(&char_code) {
            tracing::trace!("Private Use Area character: U+{:04X}", char_code);
        }
        ch.to_string()
    } else {
        tracing::trace!("Character code 0x{:04X} is not a valid Unicode code point", char_code);
        "?".to_string()
    }
}

/// Keep one visible placeholder per unmapped Type 3 glyph in extracted text.
/// The raw procedure code is not Unicode, and control-valued codes would
/// otherwise disappear before the fabricated-text gate can count them. ~keep
pub(crate) fn fallback_extraction_char_to_unicode(font: &FontInfo, char_code: u32) -> String {
    if font.subtype == "Type3" && font.best_mapping_provenance() == MappingProvenance::Fallback {
        return "?".to_string();
    }
    fallback_char_to_unicode(char_code)
}

/// Typographic punctuation fallbacks (dashes, quotes, bullet, ellipsis,
/// degree sign). Split out of `fallback_char_to_unicode` purely to keep that
/// function within the repository's line-length guideline; the mapping
/// itself is unchanged. ~keep
fn fallback_typography_symbol(code: u32) -> Option<&'static str> {
    Some(match code {
        0x2014 => "—",
        0x2013 => "–",
        0x2018 => "\u{2018}",
        0x2019 => "\u{2019}",
        0x201C => "\u{201C}",
        0x201D => "\u{201D}",
        0x2022 => "•",
        0x2026 => "…",
        0x00B0 => "°",
        _ => return None,
    })
}

/// Mathematical operator / set-theory / logic symbol fallbacks. Split out of
/// `fallback_char_to_unicode` purely to keep that function within the
/// repository's line-length guideline; the mapping itself is unchanged. ~keep
fn fallback_math_symbol(code: u32) -> Option<&'static str> {
    Some(match code {
        0x00B1 => "±",
        0x00D7 => "×",
        0x00F7 => "÷",
        0x2202 => "∂",
        0x2207 => "∇",
        0x220F => "∏",
        0x2211 => "∑",
        0x221A => "√",
        0x221E => "∞",
        0x2260 => "≠",
        0x2261 => "≡",
        0x2264 => "≤",
        0x2265 => "≥",
        0x222B => "∫",
        0x2248 => "≈",
        0x2282 => "⊂",
        0x2283 => "⊃",
        0x2286 => "⊆",
        0x2287 => "⊇",
        0x2208 => "∈",
        0x2209 => "∉",
        0x2200 => "∀",
        0x2203 => "∃",
        0x2205 => "∅",
        0x2227 => "∧",
        0x2228 => "∨",
        0x00AC => "¬",
        0x2192 => "→",
        0x2190 => "←",
        0x2194 => "↔",
        0x21D2 => "⇒",
        0x21D4 => "⇔",
        _ => return None,
    })
}

/// Greek lower- and upper-case letter fallbacks. Split out of
/// `fallback_char_to_unicode` purely to keep that function within the
/// repository's line-length guideline; the mapping itself is unchanged. ~keep
fn fallback_greek_letter(code: u32) -> Option<&'static str> {
    Some(match code {
        0x03B1 => "α",
        0x03B2 => "β",
        0x03B3 => "γ",
        0x03B4 => "δ",
        0x03B5 => "ε",
        0x03B6 => "ζ",
        0x03B7 => "η",
        0x03B8 => "θ",
        0x03B9 => "ι",
        0x03BA => "κ",
        0x03BB => "λ",
        0x03BC => "μ",
        0x03BD => "ν",
        0x03BE => "ξ",
        0x03BF => "ο",
        0x03C0 => "π",
        0x03C1 => "ρ",
        0x03C2 => "ς",
        0x03C3 => "σ",
        0x03C4 => "τ",
        0x03C5 => "υ",
        0x03C6 => "φ",
        0x03C7 => "χ",
        0x03C8 => "ψ",
        0x03C9 => "ω",

        0x0391 => "Α",
        0x0392 => "Β",
        0x0393 => "Γ",
        0x0394 => "Δ",
        0x0395 => "Ε",
        0x0396 => "Ζ",
        0x0397 => "Η",
        0x0398 => "Θ",
        0x0399 => "Ι",
        0x039A => "Κ",
        0x039B => "Λ",
        0x039C => "Μ",
        0x039D => "Ν",
        0x039E => "Ξ",
        0x039F => "Ο",
        0x03A0 => "Π",
        0x03A1 => "Ρ",
        0x03A3 => "Σ",
        0x03A4 => "Τ",
        0x03A5 => "Υ",
        0x03A6 => "Φ",
        0x03A7 => "Χ",
        0x03A8 => "Ψ",
        0x03A9 => "Ω",
        _ => return None,
    })
}

/// Currency symbol fallbacks. Split out of `fallback_char_to_unicode` purely
/// to keep that function within the repository's line-length guideline; the
/// mapping itself is unchanged. ~keep
fn fallback_currency_symbol(code: u32) -> Option<&'static str> {
    Some(match code {
        0x20AC => "€",
        0x00A3 => "£",
        0x00A5 => "¥",
        0x00A2 => "¢",
        0x20A3 => "₣",
        0x20A4 => "₤",
        0x20A9 => "₩",
        0x20AA => "₪",
        0x20AB => "₫",
        0x20B9 => "₹",
        _ => return None,
    })
}

/// Byte grouping mode for CID font character code decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ByteMode {
    /// Single-byte codes (simple fonts, some predefined CMaps)
    OneByte,
    /// Always 2-byte codes (Identity-H/V, UCS2)
    TwoByte,
    /// Shift-JIS variable-width (1 or 2 bytes depending on lead byte)
    ShiftJIS,
    /// Variable-width codespace declared by an embedded `/Encoding` CMap
    /// stream that mixes byte widths (e.g. 1-byte ASCII plus 2-byte CJK,
    /// GH #1631). Each code's width is resolved per-position via
    /// [`crate::fonts::cid_cmap::CidCMap::code_length`] rather than a single
    /// fixed width for the whole stream — see `get_byte_mode` for when this
    /// is selected over the fixed-width variants above.
    Codespace(std::sync::Arc<crate::fonts::cid_cmap::CidCMap>),
}

/// True when a Type0 font's `/Encoding` is a UTF-8 (variable-width) CMap —
/// `Uni-Utf8-H` (embedded, pdf.js issue18117) or the Adobe predefined
/// `UniGB-UTF8-H` / `UniCNS-UTF8-H` / `UniJIS-UTF8-H` / `UniKS-UTF8-H` family.
/// Such codes are 1–4 bytes and must be segmented by UTF-8 lead-byte rules
/// (see `decode_text_to_unicode`), not the fixed 1/2-byte `ByteMode`. Matching
/// on the CMap name keeps the change isolated to these fonts.
pub(crate) fn font_has_utf8_cmap(font: &FontInfo) -> bool {
    if font.subtype != "Type0" {
        return false;
    }
    if let crate::fonts::Encoding::Standard(name) = &font.encoding {
        let lower = name.to_ascii_lowercase();
        lower.contains("utf8") || lower.contains("utf-8")
    } else {
        false
    }
}

/// Get byte grouping mode for a font.
pub(crate) fn get_byte_mode(font: Option<&FontInfo>) -> ByteMode {
    let Some(font) = font else {
        return ByteMode::OneByte;
    };
    if font.subtype != "Type0" {
        return ByteMode::OneByte;
    }

    if let Some(mode) = byte_mode_from_embedded_codespace(font) {
        return mode;
    }

    // If the ToUnicode CMap declares a 2-byte codespace range, always use
    // TwoByte mode regardless of the encoding name. This handles CJK fonts
    // whose /Encoding name is a custom CMap stream that doesn't match the
    // well-known keyword patterns below (e.g. "H", "V", "UniCNS-H", …).
    // See PDF Spec §9.7.5 — `begincodespacerange` is authoritative. ~keep
    if let Some(ref lazy_cmap) = font.to_unicode
        && lazy_cmap.code_width() == 2
    {
        return ByteMode::TwoByte;
    }

    byte_mode_from_encoding(&font.encoding)
}

/// An embedded `/Encoding` CMap stream's own `begincodespacerange` is the
/// most authoritative segmentation signal there is (ISO 32000-1 §9.7.6.2) —
/// it is the actual PDF-authored declaration of how many bytes each code
/// occupies, not a name heuristic or a proxy from the (conceptually
/// separate) `/ToUnicode` stream. A genuinely mixed-width codespace gets its
/// own per-position mode (GH #1631); a single declared width is just that
/// fixed mode. Returns `None` when no codespace is declared (or there is no
/// embedded CMap at all), so the caller falls through to its other
/// heuristics unchanged. Split out of `get_byte_mode` purely to keep that
/// function within the repository's line-length guideline; behavior is
/// unchanged. ~keep
fn byte_mode_from_embedded_codespace(font: &FontInfo) -> Option<ByteMode> {
    let cid_map = font.embedded_cid_map.as_ref()?;
    match cid_map.codespace_widths().as_slice() {
        [1] => Some(ByteMode::OneByte),
        [2] => Some(ByteMode::TwoByte),
        [] => None,
        _ => Some(ByteMode::Codespace(std::sync::Arc::clone(cid_map))),
    }
}

/// Split out of `get_byte_mode` purely to keep that function within the
/// repository's line-length guideline; behavior is unchanged. ~keep
fn byte_mode_from_encoding(encoding: &crate::fonts::Encoding) -> ByteMode {
    match encoding {
        crate::fonts::Encoding::Identity => ByteMode::TwoByte,
        crate::fonts::Encoding::Standard(name) => byte_mode_from_standard_encoding_name(name),
        _ => ByteMode::OneByte,
    }
}

/// Split out of `get_byte_mode` purely to keep that function within the
/// repository's line-length guideline; behavior is unchanged. ~keep
fn byte_mode_from_standard_encoding_name(name: &str) -> ByteMode {
    if (name.contains("Identity") && !name.contains("OneByteIdentity"))
        || name.contains("UCS2")
        || name.contains("UTF16")
        // CORPUS-3: bare Adobe predefined horizontal/vertical CMaps
        // ("H"/"V", e.g. Adobe-Japan1-H) are 2-byte by definition;
        // without this they were read single-byte → CJK garbage
        // ("あいうえお" → "CACCCECGCI" on noembed-jis7). ~keep
        || name == "H"
        || name == "V"
    {
        ByteMode::TwoByte
    } else if name.contains("RKSJ") {
        ByteMode::ShiftJIS
    } else if name.contains("EUC")
        || name.contains("GBK")
        || name.contains("GBpc")
        || name.contains("GB-")
        || name.contains("CNS")
        || name.contains("B5")
        || name.contains("KSC")
        || name.contains("KSCms")
    {
        ByteMode::TwoByte
    } else {
        ByteMode::OneByte
    }
}

/// Iterator over characters in a PDF string based on font encoding.
pub(crate) struct TextCharIter<'a> {
    bytes: &'a [u8],
    byte_mode: ByteMode,
    index: usize,
}

impl<'a> TextCharIter<'a> {
    pub(crate) fn new(bytes: &'a [u8], font: Option<&FontInfo>) -> Self {
        Self {
            bytes,
            byte_mode: get_byte_mode(font),
            index: 0,
        }
    }
}

impl<'a> Iterator for TextCharIter<'a> {
    type Item = (u16, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.bytes.len() {
            return None;
        }

        let (char_code, bytes_consumed) = match &self.byte_mode {
            ByteMode::TwoByte if self.index + 1 < self.bytes.len() => (
                ((self.bytes[self.index] as u16) << 8) | (self.bytes[self.index + 1] as u16),
                2,
            ),
            ByteMode::ShiftJIS => {
                let b = self.bytes[self.index];
                let is_lead = (0x81..=0x9F).contains(&b) || (0xE0..=0xFC).contains(&b);
                if is_lead && self.index + 1 < self.bytes.len() {
                    (((b as u16) << 8) | (self.bytes[self.index + 1] as u16), 2)
                } else {
                    (b as u16, 1)
                }
            }
            ByteMode::Codespace(cid_map) => {
                let len = cid_map.code_length(self.bytes, self.index).max(1);
                let mut code: u32 = 0;
                for &b in &self.bytes[self.index..(self.index + len).min(self.bytes.len())] {
                    code = (code << 8) | b as u32;
                }
                (u16::try_from(code).unwrap_or(0), len)
            }
            _ => (self.bytes[self.index] as u16, 1),
        };

        self.index += bytes_consumed;
        Some((char_code, bytes_consumed))
    }
}

/// Segment `bytes` into UTF-8-CMap character codes (1–4 bytes each, by
/// lead-byte width; invalid lead bytes consume one byte so the scan can't
/// stall). Both `decode_text_to_unicode` and `char_codes` read this same
/// segmentation, so decoded text and per-code lookups stay aligned.
fn utf8_codes(bytes: &[u8]) -> impl Iterator<Item = u32> + '_ {
    utf8_codes_with_len(bytes).map(|(code, _)| code)
}

/// [`utf8_codes`], paired with the byte count each code consumed. Word
/// spacing needs the count: per ISO 32000-1 §9.3.3 only the single-byte code
/// 32 takes Tw, so a multi-byte code that happens to equal 0x20 must not.
fn utf8_codes_with_len(bytes: &[u8]) -> impl Iterator<Item = (u32, usize)> + '_ {
    let mut i = 0;
    std::iter::from_fn(move || {
        if i >= bytes.len() {
            return None;
        }
        let width = match bytes[i] {
            0x00..=0x7F => 1,
            0xC0..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF7 => 4,
            _ => 1,
        }
        .min(bytes.len() - i);
        let mut code: u32 = 0;
        for &b in &bytes[i..i + width] {
            code = (code << 8) | b as u32;
        }
        i += width;
        Some((code, width))
    })
}

/// The per-character codes `decode_text_to_unicode` decodes `bytes` into,
/// in decode segmentation order. Parallel per-character arrays (the
/// rasterizer's CID/width lookups) must be built from this rather than
/// `TextCharIter`, or a UTF-8 CMap's variable-width segmentation
/// desynchronises the indices. Only the rasterizer builds such arrays, so
/// this is reachable only from a paint path.
pub(crate) fn char_codes(bytes: &[u8], font: &FontInfo) -> Vec<u32> {
    char_codes_with_len(bytes, font)
        .into_iter()
        .map(|(code, _)| code)
        .collect()
}

/// [`char_codes`], paired with the byte count each code consumed, for the
/// advance computation: Tw applies only to the single-byte code 32
/// (ISO 32000-1 §9.3.3), so a 2-byte CID 0x0020 under Identity-H must not
/// take it.
pub(crate) fn char_codes_with_len(bytes: &[u8], font: &FontInfo) -> Vec<(u32, usize)> {
    if font_has_utf8_cmap(font) {
        utf8_codes_with_len(bytes).collect()
    } else {
        TextCharIter::new(bytes, Some(font))
            .map(|(code, nbytes)| (code as u32, nbytes))
            .collect()
    }
}

/// `char_to_unicode` then the shared fallback, except that a code outside
/// the Unicode scalar range only becomes '?' under
/// `DecodePolicy::question_mark_for_invalid` — otherwise it is returned as
/// U+FFFD so the caller's drop/tally branch sees it.
fn resolve_char(font: &FontInfo, code: u32, policy: DecodePolicy) -> String {
    if let Some(char_str) = font.char_to_unicode(code) {
        return char_str;
    }
    if char::from_u32(code).is_none() && !policy.question_mark_for_invalid {
        return "\u{FFFD}".to_string();
    }
    if policy.question_mark_for_invalid {
        fallback_extraction_char_to_unicode(font, code)
    } else {
        fallback_char_to_unicode(code)
    }
}

pub(crate) fn decode_text_to_unicode(
    bytes: &[u8],
    font: Option<&FontInfo>,
    policy: DecodePolicy,
    drops: Option<&mut GlyphDropTally>,
) -> String {
    let raw_result = if let Some(font) = font {
        if font.subtype != "Type0" {
            decode_simple_font_bytes(font, bytes, policy, drops)
        } else if font_has_utf8_cmap(font) {
            decode_utf8_cmap_bytes(font, bytes, policy, drops)
        } else {
            decode_generic_type0_bytes(font, bytes, policy, drops)
        }
    } else {
        decode_latin1_fallback(bytes)
    };

    filter_decoded_text(&raw_result, policy)
}

/// Decode `bytes` for a simple (non-Type0) font via its precomputed
/// byte→char table, falling back to `resolve_char` for unmapped bytes. Split
/// out of `decode_text_to_unicode` purely to keep that function within the
/// repository's line-length guideline; behavior is unchanged. ~keep
fn decode_simple_font_bytes(
    font: &FontInfo,
    bytes: &[u8],
    policy: DecodePolicy,
    mut drops: Option<&mut GlyphDropTally>,
) -> String {
    let mut result = String::new();
    let table = font.get_byte_to_char_table();
    for &byte in bytes {
        let c = table[byte as usize];
        if c != '\0' {
            result.push(c);
        } else {
            let char_str = resolve_char(font, byte as u32, policy);
            if char_str != "\u{FFFD}" || policy.preserve_unmapped {
                result.push_str(&char_str);
            } else if let Some(tally) = drops.as_deref_mut() {
                tally.record("no Unicode mapping", byte as u32, 0);
            }
        }
    }
    result
}

/// Decode `bytes` for a Type0 font whose /Encoding is an embedded CMap with
/// a UTF-8 (variable-width) codespace — e.g. `Uni-Utf8-H` (pdf.js
/// issue18117) and the Adobe predefined `Uni*-UTF8-H` family. Codes are 1–4
/// bytes segmented by UTF-8 lead-byte rules, which exceed the u16 of
/// `TextCharIter`. Segment into u32 codes and resolve via the (present)
/// ToUnicode CMap, which is keyed by the same multi-byte codes. Isolated to
/// UTF-8-CMap fonts: every other font keeps `decode_generic_type0_bytes`
/// unchanged. Split out of `decode_text_to_unicode` purely to keep that
/// function within the repository's line-length guideline; behavior is
/// unchanged. ~keep
fn decode_utf8_cmap_bytes(
    font: &FontInfo,
    bytes: &[u8],
    policy: DecodePolicy,
    mut drops: Option<&mut GlyphDropTally>,
) -> String {
    let mut result = String::new();
    for code in utf8_codes(bytes) {
        let char_str = resolve_char(font, code, policy);
        if char_str != "\u{FFFD}" || policy.preserve_unmapped {
            result.push_str(&char_str);
        } else if let Some(tally) = drops.as_deref_mut() {
            tally.record("no Unicode mapping", code, 0);
        }
    }
    result
}

/// Decode `bytes` for any other Type0 font via `TextCharIter`'s fixed/mixed
/// byte-mode segmentation. Split out of `decode_text_to_unicode` purely to
/// keep that function within the repository's line-length guideline;
/// behavior is unchanged. ~keep
fn decode_generic_type0_bytes(
    font: &FontInfo,
    bytes: &[u8],
    policy: DecodePolicy,
    mut drops: Option<&mut GlyphDropTally>,
) -> String {
    let mut result = String::new();
    for (char_code, _) in TextCharIter::new(bytes, Some(font)) {
        let char_str = resolve_char(font, char_code as u32, policy);

        if char_str != "\u{FFFD}" || policy.preserve_unmapped {
            result.push_str(&char_str);
        } else if let Some(tally) = drops.as_deref_mut() {
            tally.record("no Unicode mapping", char_code as u32, 0);
        }
    }
    result
}

/// No font - fallback to Latin-1 (ISO 8859-1) encoding. Per PDF Spec ISO
/// 32000-1:2008, Section 9.6.6, Latin-1 maps bytes 0x00-0xFF directly to
/// Unicode code points U+0000-U+00FF. Split out of `decode_text_to_unicode`
/// purely to keep that function within the repository's line-length
/// guideline; behavior is unchanged. ~keep
fn decode_latin1_fallback(bytes: &[u8]) -> String {
    crate::extractors::recovery_tally::record(|counts| {
        counts
            .missing_font_decodes
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        counts
            .missing_font_bytes
            .fetch_add(bytes.len() as u64, std::sync::atomic::Ordering::Relaxed);
    });
    tracing::trace!(
        "No font provided for {} bytes, using Latin-1 fallback (PDF spec compliant)",
        bytes.len()
    );
    bytes.iter().map(|&b| char::from(b)).collect()
}

/// Strip control characters (except tab/newline/CR) and optionally
/// decompose ligatures. Split out of `decode_text_to_unicode` purely to keep
/// that function within the repository's line-length guideline; behavior is
/// unchanged. ~keep
fn filter_decoded_text(raw_result: &str, policy: DecodePolicy) -> String {
    let mut filtered = String::with_capacity(raw_result.len());
    for c in raw_result.chars() {
        if c < '\x20' && c != '\t' && c != '\n' && c != '\r' {
            continue;
        }
        if policy.decompose_ligatures
            && let Some(components) = crate::text::ligature_processor::get_ligature_components(c)
        {
            filtered.push_str(components);
            continue;
        }
        filtered.push(c);
    }
    filtered
}

/// Strip a subset-tag prefix from a base font name
/// (e.g., `"QQPMQK+Impact"` → `"Impact"`). Only the spec-shaped form —
/// six uppercase letters and a `+` — is treated as a tag, so fonts whose
/// real name contains `+` are left alone.
pub(crate) fn strip_subset_prefix(name: &str) -> &str {
    if name.len() > 7 && name.as_bytes()[6] == b'+' && name[..6].chars().all(|c| c.is_ascii_uppercase()) {
        &name[7..]
    } else {
        name
    }
}

/// The best available TrueType cmap for each stripped base font name.
///
/// When multiple subset variants of the same font exist (`ABCDEF+Arial`,
/// `GHIJKL+Arial`), pick the cmap with the most glyph mappings — it has the
/// best Unicode coverage. On equal coverage, prefer the lexicographically
/// smallest `base_font` as a deterministic tie-breaker (HashMap iteration
/// order is randomized per-process). Callers donate these cmaps to Type0
/// fonts that lack one (the extraction caller additionally gates on
/// Identity encoding); the write loop stays at the call site
/// because the two font tables store `FontInfo` differently (`Arc` vs
/// owned).
pub(crate) fn best_truetype_cmaps<'a>(
    fonts: impl Iterator<Item = &'a FontInfo>,
) -> HashMap<String, (TrueTypeCMap, String)> {
    let mut best: HashMap<String, (TrueTypeCMap, String)> = HashMap::new();
    for font in fonts {
        if let Some(cmap) = font.truetype_cmap() {
            let stripped = strip_subset_prefix(&font.base_font).to_string();
            let dominated =
                best.get(&stripped)
                    .is_none_or(|(existing, existing_name)| match cmap.len().cmp(&existing.len()) {
                        std::cmp::Ordering::Greater => true,
                        std::cmp::Ordering::Equal => font.base_font < *existing_name,
                        std::cmp::Ordering::Less => false,
                    });
            if dominated {
                best.insert(stripped, (cmap.clone(), font.base_font.clone()));
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::{CIDToGIDMap, Encoding, FontInfo, VerticalMetrics};

    fn utf8_cmap_font() -> FontInfo {
        FontInfo {
            base_font: "TestUtf8CMap".to_string(),
            subtype: "Type0".to_string(),
            encoding: Encoding::Standard("UniFull-UTF8-H".to_string()),
            to_unicode: None,
            font_weight: None,
            flags: None,
            stem_v: None,
            ascent: 0.95,
            descent: -0.35,
            embedded_font_data: None,
            truetype_cmap: std::sync::OnceLock::new(),
            embedded_glyph_names: std::sync::OnceLock::new(),
            is_truetype_font: false,
            widths: None,
            first_char: None,
            last_char: None,
            font_matrix_a: 0.001,
            default_width: 1000.0,
            cid_to_gid_map: Some(CIDToGIDMap::Identity),
            cid_system_info: None,
            cid_font_type: Some("CIDFontType2".to_string()),
            cid_widths: None,
            cid_default_width: 1000.0,
            has_explicit_dw: false,
            cff_gid_map: None,
            multi_char_map: HashMap::new(),
            byte_to_char_table: std::sync::OnceLock::new(),
            type0_unicode_memo: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            byte_to_width_table: std::sync::OnceLock::new(),
            weight_memo: std::sync::OnceLock::new(),
            italic_memo: std::sync::OnceLock::new(),
            std14_memo: std::sync::OnceLock::new(),
            diff_glyph_names: HashMap::new(),
            wmode: 0,
            cid_vertical_metrics: None,
            cid_default_vertical_metrics: VerticalMetrics::SPEC_DEFAULT,
            cjk_substitution: None,
            embedded_cid_map: None,
        }
    }

    /// UTF-8-CMap codes are lead-byte-width segmented; an invalid lead byte
    /// consumes exactly one byte, and a truncated tail clamps rather than
    /// stalling the scan.
    #[test]
    fn utf8_codes_segments_by_lead_byte_width() {
        let bytes = [
            0x41, 0xC3, 0xA9, 0xE4, 0xB8, 0xAD, 0xF0, 0x9F, 0x98, 0x80, 0x80, 0xE4, 0xB8,
        ];
        let codes: Vec<u32> = utf8_codes(&bytes).collect();
        assert_eq!(codes, vec![0x41, 0xC3A9, 0xE4B8AD, 0xF09F_9880, 0x80, 0xE4B8]);
    }

    /// The rasterizer's parallel CID/width arrays must be built from the same
    /// segmentation the decode uses: for a UTF-8 codespace CMap that is
    /// variable-width, not `TextCharIter`'s fixed grouping.
    #[test]
    fn char_codes_uses_decode_segmentation_for_utf8_cmaps() {
        let font = utf8_cmap_font();
        assert!(font_has_utf8_cmap(&font), "fixture font must select the UTF-8 route");
        let bytes = [0x41, 0xC3, 0xA9, 0xE4, 0xB8, 0xAD];
        assert_eq!(char_codes(&bytes, &font), vec![0x41, 0xC3A9, 0xE4B8AD]);
    }

    /// The advance path keys word spacing off the byte count, so the
    /// segmentation must report how many bytes each code consumed. A
    /// multi-byte code equal to 0x20 must be distinguishable from the
    /// single-byte code 32 (ISO 32000-1 §9.3.3).
    #[test]
    fn char_codes_with_len_reports_the_bytes_each_code_consumed() {
        let font = utf8_cmap_font();
        let bytes = [0x41, 0xC3, 0xA9, 0xE4, 0xB8, 0xAD];
        assert_eq!(
            char_codes_with_len(&bytes, &font),
            vec![(0x41, 1), (0xC3A9, 2), (0xE4B8AD, 3)]
        );
    }

    /// A tally with no drops has no warning to report.
    #[test]
    fn empty_tally_builds_no_warning() {
        let tally = GlyphDropTally::default();
        assert!(tally.warning("AnyFont").is_none());
        assert!(tally.warning_omitted("AnyFont").is_none());
        assert!(tally.reason().is_none());
    }
}
