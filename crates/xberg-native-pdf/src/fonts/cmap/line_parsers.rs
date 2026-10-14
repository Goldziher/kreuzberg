//! Line-level parsers for the `begincodespacerange`, `beginbfchar`, `beginbfrange` and
//! `beginnotdefrange` sections of a ToUnicode CMap.
//!
//! Split out of `cmap.rs` purely for file size (GH#1567); the parsers are unchanged. ~keep

use super::{decode_utf16_surrogate_pair, parse_escape_sequence};
use regex::Regex;

/// Parse a `begincodespacerange` line and return the maximum code byte-width found.
///
/// Each entry is a pair of hex strings: `<lo> <hi>`.  The number of hex digits
/// in each string determines the byte width of the character codes:
/// - 2 hex digits  → 1-byte code  (e.g. `<00> <FF>`)
/// - 4 hex digits  → 2-byte code  (e.g. `<0000> <FFFF>`)
///
/// Returns 1 if the line does not contain a valid codespace pair, or 2 if at
/// least one 2-byte (4-hex-digit) entry is found.
pub(super) fn parse_codespacerange_line_width(line: &str) -> u8 {
    static RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| Regex::new(r"<([^>]*)>\s*<([^>]*)>").unwrap());

    let mut max_width: u8 = 1;
    for caps in RE.captures_iter(line) {
        let lo_hex = caps[1].trim().replace(char::is_whitespace, "");
        let hi_hex = caps[2].trim().replace(char::is_whitespace, "");
        if lo_hex.len() >= 4 || hi_hex.len() >= 4 {
            max_width = 2;
        }
    }
    max_width
}

/// Returns `true` if every byte of `s` is an ASCII hex digit (`0-9`, `a-f`, `A-F`),
/// and `s` is non-empty.
///
/// PDF ToUnicode CMap hex strings (`<...>`) are defined to contain only ASCII
/// hex digits (Adobe CMap & CIDFont Files Spec §7, ISO 32000-1 §9.10.3). The
/// capture regexes used throughout this module (`<([^>]*)>`) match any byte
/// sequence between angle brackets, so a raw stream containing invalid UTF-8
/// — decoded upstream via `String::from_utf8_lossy`, which substitutes each
/// invalid byte sequence with the 3-byte U+FFFD replacement character — can
/// hand a capture that is not what it looks like: a length check such as
/// `dst_hex.len() > 8` counts BYTES, not hex digits, so a captured string
/// that mixes ASCII bytes with one U+FFFD can satisfy that check while the
/// `step_by(4)` loop it guards then slices `&dst_hex[i..i+4]` at an offset
/// landing inside the middle of the replacement character, panicking with
/// "byte index N is not a char boundary". Validating ASCII-hex-only at the
/// capture boundary, before any length check or slicing, guarantees every
/// subsequent byte offset is also a char boundary (every ASCII byte is one
/// char), and rejects the corrupted capture the same way any other malformed
/// hex is already rejected. ~keep
fn is_ascii_hex_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Parse a bfchar line, returning all `<src> <dst>` pairs found on the line.
///
/// Example: `<0041> <0041>` maps character code 0x41 to Unicode U+0041.
/// Example: `<0003> <00410042>` maps character code 0x03 to Unicode "AB" (multi-char mapping).
/// Example: `<01> <0041> <02> <0042>` maps two character codes on one line.
///
/// Supports multiple pairs per line, hex code points, ligatures, escape sequences,
/// and flexible whitespace inside angle brackets.
pub(super) fn parse_bfchar_line(line: &str) -> Vec<(u32, String)> {
    static RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| Regex::new(r"<([^>]*)>\s*<([^>]*)>").unwrap());

    let mut results = Vec::new();

    for caps in RE.captures_iter(line) {
        let parsed = (|| -> Option<(u32, String)> {
            let src_str = caps[1].trim().replace(char::is_whitespace, "");
            let src = u32::from_str_radix(&src_str, 16).ok()?;

            let dst_str = caps[2].trim();

            let dst = if let Some(escape) = parse_escape_sequence(&format!("<{}>", dst_str)) {
                escape
            } else {
                let dst_hex = dst_str.replace(char::is_whitespace, "");
                // Reject a corrupted capture (e.g. a lossy-decoded U+FFFD from
                // invalid UTF-8 in the raw stream) before any length check or
                // byte-index slice below can misread its byte count as a hex
                // digit count. See `is_ascii_hex_digits`. ~keep
                if !is_ascii_hex_digits(&dst_hex) {
                    return None;
                }

                if dst_hex.len() <= 4 {
                    let dst_code = u32::from_str_radix(&dst_hex, 16).ok()?;
                    char::from_u32(dst_code)?.to_string()
                } else if dst_hex.len() <= 6 {
                    // 5-6 hex digits: direct supplementary Unicode code point (e.g., 020BB7 = U+20BB7)
                    // ~keep
                    let dst_code = u32::from_str_radix(&dst_hex, 16).ok()?;
                    char::from_u32(dst_code)?.to_string()
                } else if dst_hex.len() == 8 {
                    let dst_code = u32::from_str_radix(&dst_hex, 16).ok()?;
                    if let Some(decoded) = decode_utf16_surrogate_pair(dst_code) {
                        decoded
                    } else {
                        let mut result = String::new();
                        if let Ok(code1) = u32::from_str_radix(&dst_hex[0..4], 16)
                            && let Some(ch) = char::from_u32(code1)
                        {
                            result.push(ch);
                        }
                        if let Ok(code2) = u32::from_str_radix(&dst_hex[4..8], 16)
                            && let Some(ch) = char::from_u32(code2)
                        {
                            result.push(ch);
                        }
                        if result.is_empty() {
                            return None;
                        }
                        result
                    }
                } else {
                    let mut result = String::new();
                    for i in (0..dst_hex.len()).step_by(4) {
                        let end = (i + 4).min(dst_hex.len());
                        if let Ok(code) = u32::from_str_radix(&dst_hex[i..end], 16)
                            && let Some(ch) = char::from_u32(code)
                        {
                            result.push(ch);
                        }
                    }
                    if result.is_empty() {
                        return None;
                    }
                    result
                }
            };

            Some((src, dst))
        })();

        if let Some(pair) = parsed {
            results.push(pair);
        }
    }

    results
}

/// Parse a bfrange line: `<start> <end> <dst>`
///
/// Example: `<0020> <007E> <0020>` maps codes 0x20-0x7E to Unicode U+0020-U+007E.
///
/// There are two formats:
/// 1. `<start> <end> <dst>` - Sequential mapping starting at dst
/// 2. `<start> <end> [<dst1> <dst2> ...]` - Array of individual destinations
///
/// This function supports both formats and flexible whitespace within angle brackets.
pub(super) fn parse_bfrange_line(line: &str) -> Option<Vec<(u32, String)>> {
    static RE_SEQ: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"<([^>]*)>\s*<([^>]*)>\s*<([^>]*)>").unwrap());
    static RE_ARRAY: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"<([^>]*)>\s*<([^>]*)>\s*\[((?:\s*<[^>]+>\s*)+)\]").unwrap());

    if let Some(caps) = RE_ARRAY.captures(line) {
        return parse_bfrange_array_form(&caps);
    }

    if let Some(caps) = RE_SEQ.captures(line) {
        return parse_bfrange_sequential_form(&caps);
    }

    None
}

/// Parse the `<start> <end> [<dst1> <dst2> ...]` bfrange form: an explicit
/// array of individual destinations, one per code in `start..=end`. Split
/// out of `parse_bfrange_line` purely to keep that function within the
/// repository's line-length guideline; behavior, `?`-propagated abort
/// semantics, and per-entry `continue`-skip semantics are all unchanged. ~keep
fn parse_bfrange_array_form(caps: &regex::Captures) -> Option<Vec<(u32, String)>> {
    let start_str = caps[1].trim().replace(char::is_whitespace, "");
    let end_str = caps[2].trim().replace(char::is_whitespace, "");
    let start = u32::from_str_radix(&start_str, 16).ok()?;
    let end = u32::from_str_radix(&end_str, 16).ok()?;
    let array_str = &caps[3];

    static RE_HEX: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| Regex::new(r"<([^>]*)>").unwrap());
    // Drop any array entry that is not pure ASCII hex (e.g. a lossy-decoded
    // U+FFFD from invalid UTF-8 in the raw stream) before it can reach the
    // byte-index slices below. See `is_ascii_hex_digits`. A dropped entry
    // shrinks `dst_hexes` below `range_size`, which the existing
    // size-mismatch warning already surfaces. ~keep
    let dst_hexes: Vec<String> = RE_HEX
        .captures_iter(array_str)
        .filter_map(|cap| {
            let s = cap.get(1).unwrap().as_str().trim().replace(char::is_whitespace, "");
            if is_ascii_hex_digits(&s) { Some(s) } else { None }
        })
        .collect();

    let mut result = Vec::new();

    // `start`/`end` are attacker-controlled hex from the font's ToUnicode stream, so a
    // reversed range (`<0100> <0000>`) or `end == u32::MAX` makes the naive
    // `(end - start + 1)` overflow: a panic under `overflow-checks` (debug and test), a
    // silent wrap to a bogus count in release, which this crate's profile does not
    // enable. The sequential branch and `parse_notdefrange_line` clamp with
    // `end.saturating_sub(start).min(10000)`; here `range_size` is an entry COUNT rather
    // than a 0-based loop offset, hence the `saturating_add(1)`, and their `.min(10000)`
    // is unnecessary because this value only bounds a `.take()` over `dst_hexes` (already
    // sized by the literal array in the stream) instead of driving iteration. ~keep
    let range_size = end.saturating_sub(start).saturating_add(1) as usize;

    // SPEC VALIDATION: PDF Spec ISO 32000-1:2008, Section 9.10.3
    // The array must have exactly (end - start + 1) entries.
    // Current behavior (lenient): Use what's available, ignore extras/missing.
    // Proper strict mode: Should fail if array size doesn't match range_size. ~keep
    if dst_hexes.len() != range_size {
        tracing::warn!(
            "ToUnicode bfrange array size mismatch: expected {} entries for range 0x{:X}-0x{:X}, got {}",
            range_size,
            start,
            end,
            dst_hexes.len()
        );
    }

    for (i, dst_hex) in dst_hexes.iter().take(range_size).enumerate() {
        let src = start + i as u32;

        let Some(dst) = decode_bfrange_array_entry(dst_hex)? else {
            continue;
        };

        result.push((src, dst));
    }
    Some(result)
}

/// Decode one bfrange array entry (a single `<dstN>` hex string) to its
/// Unicode string. The outer `Option` (propagated with `?` by the caller)
/// is this entry's *abort-the-whole-line* signal — a malformed hex digit
/// sequence, matching the original inline `.ok()?` sites. The inner
/// `Option` is this entry's *skip-just-this-entry* signal — a
/// well-formed-hex but otherwise unusable value, matching the original
/// inline `continue` sites. Split out of `parse_bfrange_array_form` purely
/// to keep nesting within the repository's guideline; behavior is
/// unchanged. ~keep
fn decode_bfrange_array_entry(dst_hex: &str) -> Option<Option<String>> {
    if dst_hex.len() <= 4 {
        let dst_code = u32::from_str_radix(dst_hex, 16).ok()?;
        return Some(Some(char::from_u32(dst_code)?.to_string()));
    }
    if dst_hex.len() <= 6 {
        let dst_code = u32::from_str_radix(dst_hex, 16).ok()?;
        return Some(char::from_u32(dst_code).map(|ch| ch.to_string()));
    }
    if dst_hex.len() == 8 {
        let dst_code = u32::from_str_radix(dst_hex, 16).ok()?;
        if let Some(decoded) = decode_utf16_surrogate_pair(dst_code) {
            return Some(Some(decoded));
        }
        let mut unicode_string = String::new();
        if let Ok(code) = u32::from_str_radix(&dst_hex[0..4], 16)
            && let Some(ch) = char::from_u32(code)
        {
            unicode_string.push(ch);
        }
        if let Ok(code) = u32::from_str_radix(&dst_hex[4..8], 16)
            && let Some(ch) = char::from_u32(code)
        {
            unicode_string.push(ch);
        }
        return Some(if unicode_string.is_empty() {
            None
        } else {
            Some(unicode_string)
        });
    }

    let mut unicode_string = String::new();
    for chunk_start in (0..dst_hex.len()).step_by(4) {
        let chunk_end = (chunk_start + 4).min(dst_hex.len());
        if let Ok(code) = u32::from_str_radix(&dst_hex[chunk_start..chunk_end], 16)
            && let Some(ch) = char::from_u32(code)
        {
            unicode_string.push(ch);
        }
    }
    Some(if unicode_string.is_empty() {
        None
    } else {
        Some(unicode_string)
    })
}

/// Parse the `<start> <end> <dst>` bfrange form: a sequential mapping
/// starting at `dst` and incrementing per code in `start..=end`. Split out
/// of `parse_bfrange_line` purely to keep that function within the
/// repository's line-length guideline; behavior is unchanged. ~keep
fn parse_bfrange_sequential_form(caps: &regex::Captures) -> Option<Vec<(u32, String)>> {
    let start_str = caps[1].trim().replace(char::is_whitespace, "");
    let end_str = caps[2].trim().replace(char::is_whitespace, "");
    let dst_start_str = caps[3].trim().replace(char::is_whitespace, "");
    let start = u32::from_str_radix(&start_str, 16).ok()?;
    let end = u32::from_str_radix(&end_str, 16).ok()?;
    let dst_start = u32::from_str_radix(&dst_start_str, 16).ok()?;

    let mut result = Vec::new();
    let range_size = end.saturating_sub(start).min(10000);

    // For surrogate pair destinations (8 hex digits), decode to Unicode code point
    // first, then increment the code point. Naively incrementing the raw u32 would
    // overflow across the low surrogate boundary (0xDFFF → 0xE000). ~keep
    let base_codepoint = if dst_start > 0xFFFF {
        if let Some(decoded) = decode_utf16_surrogate_pair(dst_start) {
            decoded.chars().next().map(|c| c as u32)
        } else {
            Some(dst_start)
        }
    } else {
        Some(dst_start)
    };

    if let Some(base_cp) = base_codepoint {
        for i in 0..=range_size {
            let src = start.wrapping_add(i);
            let cp = base_cp.wrapping_add(i);
            if let Some(ch) = char::from_u32(cp) {
                result.push((src, ch.to_string()));
            }
        }
    }
    Some(result)
}

/// Parse a notdefrange line: `<start> <end> <dst>`
///
/// Phase 4.1 addition: Support for beginnotdefrange sections
///
/// Example: `<0000> <0040> <FFFD>` maps codes 0x0000-0x0040 to U+FFFD (replacement character)
/// for unmapped character codes (fallback/notdef handling).
///
/// Unlike bfrange, notdefrange only supports the sequential format (not arrays).
/// Notdefrange mappings are applied only to codes not already mapped by bfchar/bfrange.
pub(super) fn parse_notdefrange_line(line: &str) -> Option<Vec<(u32, String)>> {
    static RE_SEQ: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"<([^>]*)>\s*<([^>]*)>\s*<([^>]*)>").unwrap());

    if let Some(caps) = RE_SEQ.captures(line) {
        let start_str = caps[1].trim().replace(char::is_whitespace, "");
        let end_str = caps[2].trim().replace(char::is_whitespace, "");
        let dst_str = caps[3].trim();

        let start = u32::from_str_radix(&start_str, 16).ok()?;
        let end = u32::from_str_radix(&end_str, 16).ok()?;

        let dst = if let Some(escape) = parse_escape_sequence(&format!("<{}>", dst_str)) {
            escape
        } else {
            let dst_hex = dst_str.replace(char::is_whitespace, "");
            let dst_code = u32::from_str_radix(&dst_hex, 16).ok()?;
            if dst_code > 0xFFFF {
                decode_utf16_surrogate_pair(dst_code).or_else(|| char::from_u32(dst_code).map(|ch| ch.to_string()))?
            } else {
                char::from_u32(dst_code)?.to_string()
            }
        };

        let mut result = Vec::new();
        let range_size = end.saturating_sub(start).min(10000);
        for i in 0..=range_size {
            let src = start.wrapping_add(i);
            result.push((src, dst.clone()));
        }
        return Some(result);
    }

    None
}
