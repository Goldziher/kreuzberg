//! CFF (Compact Font Format) encoding parser.
//!
//! Parses the built-in encoding table from CFF font programs and resolves
//! PDF content-stream bytes to glyph IDs.
//!
//! # Two byte → GID resolution paths
//!
//! Simple CFF fonts have two potential sources of byte → glyph mapping
//! information, and PDFs decide between them according to ISO 32000-1
//! §9.6.6:
//!
//! 1. **The PDF font dictionary's `/Encoding`** is authoritative for
//!    simple fonts (Type 1 / TrueType / CFF). It supplies byte → glyph
//!    *name*; the CFF Charset then resolves the name → SID → GID.
//!    [`parse_cff_gid_mapping_with_pdf_encoding`] implements this path.
//!
//! 2. **The CFF font program's own Encoding table** (CFF Tech Note #5176
//!    §12) supplies byte → GID directly. This is the *fallback* when no
//!    PDF-level encoding is supplied (e.g. an `Encoding::Identity` caller).
//!    [`parse_cff_gid_mapping`] implements this path.
//!
//! Subsetter-emitted CFF Encoding tables are frequently sparse —
//! some prepress subsetters commonly emit only `0x20 → space` and
//! `0x41 → A` while the Charset enumerates the full subset — so callers
//! that have the PDF `/Encoding` in hand should always go through the
//! `_with_pdf_encoding` entrypoint. The CFF Encoding path is preserved
//! for backwards compatibility and as a final fallback when the PDF side
//! cannot supply a byte → name resolver.
//!
//! Per PDF spec §9.6.6.2, when no `/BaseEncoding` is specified in an
//! encoding dictionary, the implicit base encoding is the font program's
//! built-in encoding — this module also provides [`parse_cff_encoding`]
//! for that legacy lookup.

use std::collections::HashMap;

mod mac_encoding_tables;
use mac_encoding_tables::{mac_roman_byte_to_name, standard_encoding_byte_to_name};
mod sid_names;
use sid_names::sid_to_name;

/// Look up the SID for a standard glyph name by searching the predefined SID table.
fn glyph_name_to_sid(name: &str) -> Option<u16> {
    for sid in 0u16..391 {
        if sid_to_name(sid) == Some(name) {
            return Some(sid);
        }
    }
    None
}

/// Parse a CFF INDEX structure, returning byte slices for each entry.
fn parse_index(data: &[u8], offset: usize) -> Option<(Vec<&[u8]>, usize)> {
    if offset + 2 > data.len() {
        return None;
    }
    let count = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
    if count == 0 {
        return Some((Vec::new(), offset + 2));
    }

    if offset + 3 > data.len() {
        return None;
    }
    let off_size = data[offset + 2] as usize;
    if off_size == 0 || off_size > 4 {
        return None;
    }

    let offset_array_start = offset + 3;
    let offset_array_len = (count + 1) * off_size;
    if offset_array_start + offset_array_len > data.len() {
        return None;
    }

    let mut offsets = Vec::with_capacity(count + 1);
    for i in 0..=count {
        let pos = offset_array_start + i * off_size;
        let mut val: u32 = 0;
        for j in 0..off_size {
            val = (val << 8) | data[pos + j] as u32;
        }
        offsets.push(val as usize);
    }

    let data_start = offset_array_start + offset_array_len;
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let start = data_start + offsets[i] - 1; // CFF offsets are 1-based ~keep
        let end = data_start + offsets[i + 1] - 1;
        if start > data.len() || end > data.len() || start > end {
            return None;
        }
        entries.push(&data[start..end]);
    }

    let next_offset = data_start + offsets[count] - 1;
    Some((entries, next_offset))
}

/// Parse a CFF DICT operand (integer or real).
/// Returns (value, bytes consumed).
fn parse_dict_operand(data: &[u8], pos: usize) -> Option<(i32, usize)> {
    if pos >= data.len() {
        return None;
    }
    let b0 = data[pos] as i32;
    match b0 {
        // Integer: 1 byte ~keep
        32..=246 => Some((b0 - 139, 1)),
        // Integer: 2 bytes ~keep
        247..=250 => {
            if pos + 1 >= data.len() {
                return None;
            }
            let b1 = data[pos + 1] as i32;
            Some(((b0 - 247) * 256 + b1 + 108, 2))
        }
        251..=254 => {
            if pos + 1 >= data.len() {
                return None;
            }
            let b1 = data[pos + 1] as i32;
            Some((-(b0 - 251) * 256 - b1 - 108, 2))
        }
        // Integer: 3 bytes (16-bit) ~keep
        28 => {
            if pos + 2 >= data.len() {
                return None;
            }
            let val = i16::from_be_bytes([data[pos + 1], data[pos + 2]]) as i32;
            Some((val, 3))
        }
        // Integer: 5 bytes (32-bit) ~keep
        29 => {
            if pos + 4 >= data.len() {
                return None;
            }
            let val = i32::from_be_bytes([data[pos + 1], data[pos + 2], data[pos + 3], data[pos + 4]]);
            Some((val, 5))
        }
        // Real number (skip, we only need integers for encoding/charset offsets) ~keep
        30 => {
            let mut i = pos + 1;
            while i < data.len() {
                let nibble1 = (data[i] >> 4) & 0x0F;
                let nibble2 = data[i] & 0x0F;
                if nibble1 == 0x0F || nibble2 == 0x0F {
                    return Some((0, i - pos + 1));
                }
                i += 1;
            }
            None
        }
        _ => None,
    }
}

/// Parse a CFF Top DICT to extract encoding and charset offsets.
fn parse_top_dict(dict_data: &[u8]) -> (i32, i32) {
    let mut encoding_offset: i32 = 0; // Default: StandardEncoding ~keep
    let mut charset_offset: i32 = 0; // Default: ISOAdobe charset ~keep

    let mut pos = 0;
    let mut operand_stack: Vec<i32> = Vec::new();

    while pos < dict_data.len() {
        let b0 = dict_data[pos];
        if b0 <= 21 {
            let op = if b0 == 12 {
                pos += 1;
                if pos >= dict_data.len() {
                    break;
                }
                (12u16 << 8) | dict_data[pos] as u16
            } else {
                b0 as u16
            };

            match op {
                16 => {
                    if let Some(&val) = operand_stack.last() {
                        encoding_offset = val;
                    }
                }
                15 => {
                    if let Some(&val) = operand_stack.last() {
                        charset_offset = val;
                    }
                }
                _ => {}
            }

            operand_stack.clear();
            pos += 1;
        } else if let Some((val, consumed)) = parse_dict_operand(dict_data, pos) {
            operand_stack.push(val);
            pos += consumed;
        } else {
            pos += 1;
        }
    }

    (encoding_offset, charset_offset)
}

/// Parse the CFF Top DICT and surface (CharStrings, Encoding, charset)
/// offsets. CharStrings (op 17) points at the CharStrings INDEX whose count
/// field is the real `nGlyphs` for the font — needed to parse the full
/// Charset for subsets enumerating >256 glyphs.
///
/// Returns `(charstrings_offset, encoding_offset, charset_offset)`.
fn parse_top_dict_with_charstrings(dict_data: &[u8]) -> (i32, i32, i32) {
    let mut charstrings_offset: i32 = 0;
    let mut encoding_offset: i32 = 0;
    let mut charset_offset: i32 = 0;

    let mut pos = 0;
    let mut operand_stack: Vec<i32> = Vec::new();

    while pos < dict_data.len() {
        let b0 = dict_data[pos];
        if b0 <= 21 {
            let op = if b0 == 12 {
                pos += 1;
                if pos >= dict_data.len() {
                    break;
                }
                (12u16 << 8) | dict_data[pos] as u16
            } else {
                b0 as u16
            };

            match op {
                15 => {
                    if let Some(&val) = operand_stack.last() {
                        charset_offset = val;
                    }
                }
                16 => {
                    if let Some(&val) = operand_stack.last() {
                        encoding_offset = val;
                    }
                }
                17 => {
                    if let Some(&val) = operand_stack.last() {
                        charstrings_offset = val;
                    }
                }
                _ => {}
            }

            operand_stack.clear();
            pos += 1;
        } else if let Some((val, consumed)) = parse_dict_operand(dict_data, pos) {
            operand_stack.push(val);
            pos += consumed;
        } else {
            pos += 1;
        }
    }

    (charstrings_offset, encoding_offset, charset_offset)
}

/// Read the 2-byte big-endian `count` field at the start of a CFF INDEX
/// header (CFF spec §5). For the CharStrings INDEX, this count is `nGlyphs`.
///
/// Returns `None` if the header is truncated.
fn read_index_count(data: &[u8], offset: usize) -> Option<u32> {
    if offset + 2 > data.len() {
        return None;
    }
    Some(u16::from_be_bytes([data[offset], data[offset + 1]]) as u32)
}

/// Parse the CFF charset table.
/// Returns GID → SID mapping (GID 0 is always .notdef).
fn parse_charset(data: &[u8], offset: usize, num_glyphs: usize) -> Option<Vec<u16>> {
    if offset >= data.len() {
        return None;
    }

    let mut sids = Vec::with_capacity(num_glyphs);
    sids.push(0); // GID 0 = .notdef (SID 0) ~keep

    let format = data[offset];
    let mut pos = offset + 1;

    match format {
        0 => {
            // Format 0: array of SIDs ~keep
            for _ in 1..num_glyphs {
                if pos + 1 >= data.len() {
                    break;
                }
                let sid = u16::from_be_bytes([data[pos], data[pos + 1]]);
                sids.push(sid);
                pos += 2;
            }
        }
        1 => {
            // Format 1: ranges with 1-byte count ~keep
            while sids.len() < num_glyphs && pos + 2 < data.len() {
                let first_sid = u16::from_be_bytes([data[pos], data[pos + 1]]);
                let n_left = data[pos + 2] as u16;
                pos += 3;
                for i in 0..=n_left {
                    if sids.len() >= num_glyphs {
                        break;
                    }
                    sids.push(first_sid + i);
                }
            }
        }
        2 => {
            // Format 2: ranges with 2-byte count ~keep
            while sids.len() < num_glyphs && pos + 3 < data.len() {
                let first_sid = u16::from_be_bytes([data[pos], data[pos + 1]]);
                let n_left = u16::from_be_bytes([data[pos + 2], data[pos + 3]]);
                pos += 4;
                for i in 0..=n_left {
                    if sids.len() >= num_glyphs {
                        break;
                    }
                    sids.push(first_sid + i);
                }
            }
        }
        _ => return None,
    }

    Some(sids)
}

/// Parse the CFF encoding table.
/// Returns character code → GID mapping.
fn parse_encoding_table(data: &[u8], offset: usize) -> Option<HashMap<u8, u16>> {
    if offset >= data.len() {
        return None;
    }

    let mut code_to_gid = HashMap::new();
    let format = data[offset] & 0x7F; // Bit 7 is supplement flag ~keep
    let has_supplement = (data[offset] & 0x80) != 0;
    let mut pos = offset + 1;

    match format {
        0 => {
            // Format 0: array of codes ~keep
            if pos >= data.len() {
                return None;
            }
            let n_codes = data[pos] as usize;
            pos += 1;
            for gid in 1..=n_codes {
                if pos >= data.len() {
                    break;
                }
                let code = data[pos];
                code_to_gid.insert(code, gid as u16);
                pos += 1;
            }
        }
        1 => {
            // Format 1: ranges ~keep
            if pos >= data.len() {
                return None;
            }
            let n_ranges = data[pos] as usize;
            pos += 1;
            let mut gid: u16 = 1;
            for _ in 0..n_ranges {
                if pos + 1 >= data.len() {
                    break;
                }
                let first = data[pos];
                let n_left = data[pos + 1] as u16;
                pos += 2;
                for i in 0..=n_left {
                    let code = first.wrapping_add(i as u8);
                    code_to_gid.insert(code, gid);
                    gid += 1;
                }
            }
        }
        _ => return None,
    }

    if has_supplement && pos < data.len() {
        let n_sups = data[pos] as usize;
        pos += 1;
        for _ in 0..n_sups {
            if pos + 2 >= data.len() {
                break;
            }
            let code = data[pos];
            let sid = u16::from_be_bytes([data[pos + 1], data[pos + 2]]);
            pos += 3;
            // For supplements, we use SID directly as a pseudo-GID
            // The caller will need to handle this via the charset ~keep
            code_to_gid.insert(code, sid);
        }
    }

    Some(code_to_gid)
}

/// Resolve a glyph name from a SID, using predefined strings and the
/// String INDEX from the CFF font.
fn resolve_glyph_name<'a>(sid: u16, string_index: &'a [&'a [u8]]) -> Option<String> {
    if sid <= 390 {
        sid_to_name(sid).map(|s| s.to_string())
    } else {
        let idx = (sid - 391) as usize;
        if idx < string_index.len() {
            std::str::from_utf8(string_index[idx]).ok().map(|s| s.to_string())
        } else {
            None
        }
    }
}

/// Extract the CFF table from an OpenType (sfnt) wrapper.
/// Returns the CFF data slice if found, or None if the data isn't an sfnt container.
fn extract_cff_from_opentype(data: &[u8]) -> Option<&[u8]> {
    if data.len() < 12 {
        return None;
    }
    let magic = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    // Check for OpenType "OTTO" or TrueType 0x00010000 ~keep
    if magic != 0x4F54544F && magic != 0x00010000 {
        return None;
    }
    let num_tables = u16::from_be_bytes([data[4], data[5]]) as usize;
    // Table directory starts at offset 12 ~keep
    let mut pos = 12;
    for _ in 0..num_tables {
        if pos + 16 > data.len() {
            return None;
        }
        let tag = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        let offset = u32::from_be_bytes([data[pos + 8], data[pos + 9], data[pos + 10], data[pos + 11]]) as usize;
        let length = u32::from_be_bytes([data[pos + 12], data[pos + 13], data[pos + 14], data[pos + 15]]) as usize;
        // CFF tag = 0x43464620 ("CFF ") ~keep
        if tag == 0x43464620 && offset + length <= data.len() {
            return Some(&data[offset..offset + length]);
        }
        pos += 16;
    }
    None
}

/// Resolve the raw CFF byte range from `font_data`, unwrapping an OpenType
/// container if the input isn't already a bare CFF table (`font_data[0] ==
/// 1`). Split out of `parse_cff_encoding` purely to keep that function
/// within the repository's line-length guideline; behavior is unchanged. ~keep
fn resolve_cff_data(font_data: &[u8]) -> Option<&[u8]> {
    if font_data[0] == 1 {
        return Some(font_data);
    }
    if let Some(cff) = extract_cff_from_opentype(font_data) {
        tracing::debug!(
            "Extracted CFF table ({} bytes) from OpenType wrapper ({} bytes)",
            cff.len(),
            font_data.len()
        );
        Some(cff)
    } else {
        crate::extractors::recovery_tally::record(|counts| {
            counts
                .unsupported_cff_versions
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        });
        tracing::trace!("CFF version {} not supported (expected 1)", font_data[0]);
        None
    }
}

/// Build a GID-indexed fallback encoding map for a CFF using
/// StandardEncoding with a custom charset (subset fonts where character
/// codes equal GIDs rather than standard encoding positions). Returns
/// `None` when no charset SID resolves to a mapped Unicode character. Split
/// out of `parse_cff_encoding` purely to keep that function within the
/// repository's line-length guideline; behavior is unchanged. ~keep
fn cff_standard_encoding_charset_fallback(
    cff_data: &[u8],
    charset_offset: i32,
    string_index: &[&[u8]],
) -> Option<HashMap<u8, char>> {
    let num_glyphs = 256usize;
    let charset_sids = parse_charset(cff_data, charset_offset as usize, num_glyphs)?;

    let mut encoding_map = HashMap::new();
    for (gid, &sid) in charset_sids.iter().enumerate() {
        if gid == 0 || gid > 255 {
            continue;
        }
        if let Some(glyph_name) = resolve_glyph_name(sid, string_index)
            && let Some(unicode_char) = super::font_dict::glyph_name_to_unicode(&glyph_name)
        {
            encoding_map.insert(gid as u8, unicode_char);
        }
    }
    if encoding_map.is_empty() {
        None
    } else {
        Some(encoding_map)
    }
}

/// Build the byte→Unicode encoding map from a CFF's own Encoding + Charset
/// tables (the non-predefined-encoding path). Split out of
/// `parse_cff_encoding` purely to keep that function within the
/// repository's line-length guideline; behavior is unchanged. ~keep
fn build_cff_encoding_map(
    cff_data: &[u8],
    encoding_offset: i32,
    charset_offset: i32,
    string_index: &[&[u8]],
) -> Option<HashMap<u8, char>> {
    let code_to_gid = parse_encoding_table(cff_data, encoding_offset as usize)?;

    let max_gid = code_to_gid.values().max().copied().unwrap_or(0) as usize;
    let num_glyphs = max_gid + 10;

    let charset_sids = if charset_offset == 0 {
        (0..num_glyphs as u16).collect()
    } else if charset_offset == 1 || charset_offset == 2 {
        tracing::debug!("CFF uses predefined charset {}", charset_offset);
        return None;
    } else {
        parse_charset(cff_data, charset_offset as usize, num_glyphs)?
    };

    let mut encoding_map = HashMap::new();

    for (&code, &gid) in &code_to_gid {
        let sid = if (gid as usize) < charset_sids.len() {
            charset_sids[gid as usize]
        } else {
            continue;
        };

        if let Some(glyph_name) = resolve_glyph_name(sid, string_index)
            && let Some(unicode_char) = super::font_dict::glyph_name_to_unicode(&glyph_name)
        {
            encoding_map.insert(code, unicode_char);
        }
    }

    if encoding_map.is_empty() {
        None
    } else {
        tracing::debug!(
            "CFF built-in encoding parsed: {} character mappings",
            encoding_map.len()
        );
        Some(encoding_map)
    }
}

/// Extract the built-in encoding from a CFF font program.
///
/// Returns a HashMap mapping character codes (u8) to Unicode characters.
/// This implements the CFF encoding → charset → glyph name → Unicode pipeline.
/// Also handles OpenType-wrapped CFF data (FontFile3 with sfnt container).
pub fn parse_cff_encoding(font_data: &[u8]) -> Option<HashMap<u8, char>> {
    if font_data.len() < 4 {
        return None;
    }

    let cff_data = resolve_cff_data(font_data)?;

    if cff_data.len() < 4 || cff_data[0] != 1 {
        return None;
    }
    let hdr_size = cff_data[2] as usize;

    let (_, after_name) = parse_index(cff_data, hdr_size)?;

    let (top_dicts, after_top_dict) = parse_index(cff_data, after_name)?;
    if top_dicts.is_empty() {
        return None;
    }

    let (string_index, _after_string) = parse_index(cff_data, after_top_dict)?;

    let (encoding_offset, charset_offset) = parse_top_dict(top_dicts[0]);

    if encoding_offset == 1 {
        // ExpertEncoding — rarely used for text ~keep
        tracing::debug!("CFF uses ExpertEncoding (predefined)");
        return None;
    }

    if encoding_offset == 0 {
        // StandardEncoding — for fonts with custom charsets, build a GID-based
        // fallback map. This handles subset fonts where character codes equal
        // GIDs rather than standard encoding positions. ~keep
        if charset_offset > 2 {
            tracing::debug!("CFF StandardEncoding + custom charset; building charset-based map");
            if let Some(encoding_map) = cff_standard_encoding_charset_fallback(cff_data, charset_offset, &string_index)
            {
                tracing::debug!("CFF charset-based fallback: {} character mappings", encoding_map.len());
                return Some(encoding_map);
            }
        }
        tracing::debug!("CFF uses StandardEncoding (predefined)");
        return None;
    }

    build_cff_encoding_map(cff_data, encoding_offset, charset_offset, &string_index)
}

/// Parse a CFF font program and return a byte_code → glyph_id mapping.
/// This allows rendering CFF subset fonts by mapping PDF character codes
/// directly to glyph indices without needing a Unicode cmap.
pub fn parse_cff_gid_mapping(font_data: &[u8]) -> Option<HashMap<u8, u16>> {
    if font_data.len() < 4 {
        return None;
    }

    let cff_data = if font_data[0] != 1 {
        extract_cff_from_opentype(font_data)?
    } else {
        font_data
    };

    if cff_data.len() < 4 || cff_data[0] != 1 {
        return None;
    }
    let hdr_size = cff_data[2] as usize;

    let (_, after_name) = parse_index(cff_data, hdr_size)?;
    let (top_dicts, after_top_dict) = parse_index(cff_data, after_name)?;
    if top_dicts.is_empty() {
        return None;
    }

    let (_string_index, _after_string) = parse_index(cff_data, after_top_dict)?;
    let (encoding_offset, charset_offset) = parse_top_dict(top_dicts[0]);

    if encoding_offset == 0 && charset_offset > 2 {
        // StandardEncoding + custom charset:
        // byte_code → SID (via CFF Standard Encoding) → GID (via charset) ~keep
        let num_glyphs = 256usize;
        if let Some(charset_sids) = parse_charset(cff_data, charset_offset as usize, num_glyphs) {
            let mut sid_to_gid: HashMap<u16, u16> = HashMap::new();
            for (gid, &sid) in charset_sids.iter().enumerate() {
                if gid > 0 {
                    sid_to_gid.entry(sid).or_insert(gid as u16);
                }
            }
            let mut map = HashMap::new();
            for byte_code in 0u16..256 {
                let glyph_name = super::font_dict::FontInfo::gid_to_standard_glyph_name(byte_code);
                if let Some(name) = glyph_name
                    && let Some(sid) = glyph_name_to_sid(name)
                    && let Some(&gid) = sid_to_gid.get(&sid)
                {
                    map.insert(byte_code as u8, gid);
                }
            }
            if !map.is_empty() {
                tracing::debug!("CFF StandardEncoding→charset GID mapping: {} entries", map.len());
                return Some(map);
            }
        }
        return None;
    }

    if encoding_offset <= 1 {
        return None;
    }

    parse_encoding_table(cff_data, encoding_offset as usize)
}

/// Build the byte → GID map for a simple CFF font using the PDF font
/// dictionary's `/Encoding` as the byte → glyph-name source and the CFF
/// Charset as the glyph-name → GID resolver, per ISO 32000-1 §9.6.6.
///
/// This is the correct resolution model for simple Type 1 / TrueType / CFF
/// fonts. The CFF font program's *own* Encoding table is only authoritative
/// when there is no PDF-level encoding to consult (an Identity case).
///
/// In practice the bug this fixes is prepress-tool-authored subset CFFs
/// whose internal Encoding lists only `0x20 → space` and `0x41 → A` while
/// the Charset enumerates the full subset (e.g. `A B C D E F G I K M N O R
/// S U V X g`). The previous resolution consulted the CFF Encoding directly
/// and silently dropped every non-A content byte to `.notdef`, producing
/// bare-A glyphs on every separation plate.
///
/// Returns `None` when the result is empty so callers can fall through to
/// the legacy [`parse_cff_gid_mapping`] for fonts where the PDF-level
/// encoding genuinely cannot resolve any byte (e.g. `Encoding::Identity`
/// on a CIDFont — though those normally short-circuit before reaching
/// this path).
pub fn parse_cff_gid_mapping_with_pdf_encoding(
    font_data: &[u8],
    pdf_encoding: &crate::fonts::font_dict::Encoding,
    differences: &HashMap<u8, String>,
) -> Option<HashMap<u8, u16>> {
    use crate::fonts::font_dict::Encoding;

    if matches!(pdf_encoding, Encoding::Identity) {
        // Caller has no byte→name mapping to supply — fall through to the
        // CFF Encoding-driven legacy path. ~keep
        return parse_cff_gid_mapping(font_data);
    }

    if font_data.len() < 4 {
        return None;
    }
    let cff_data = if font_data[0] != 1 {
        extract_cff_from_opentype(font_data)?
    } else {
        font_data
    };
    if cff_data.len() < 4 || cff_data[0] != 1 {
        return None;
    }
    let hdr_size = cff_data[2] as usize;

    let (_, after_name) = parse_index(cff_data, hdr_size)?;
    let (top_dicts, after_top_dict) = parse_index(cff_data, after_name)?;
    if top_dicts.is_empty() {
        return None;
    }
    let (string_index, _after_string) = parse_index(cff_data, after_top_dict)?;
    let (charstrings_offset, _encoding_offset, charset_offset) = parse_top_dict_with_charstrings(top_dicts[0]);

    // §9.6.6 path: build name→GID from the Charset (which always enumerates
    // every subset glyph), then key bytes through the PDF /Encoding +
    // /Differences.
    //
    // nGlyphs comes from the CharStrings INDEX header (CFF spec §9). Simple
    // fonts only address ≤256 codes via /Encoding, but a subset's GID space
    // can exceed 256 — a /Differences entry pointing at a glyph whose CFF
    // Charset GID is >256 must still resolve. Falls back to 256 if the
    // CharStrings offset is missing or the INDEX header is truncated. ~keep
    let num_glyphs = if charstrings_offset > 0 {
        read_index_count(cff_data, charstrings_offset as usize)
            .map(|n| n as usize)
            .unwrap_or(256)
    } else {
        256
    };

    let charset_sids = if charset_offset > 2 {
        parse_charset(cff_data, charset_offset as usize, num_glyphs)?
    } else {
        // charset_offset 0 or 1 = ISOAdobe / Expert / ExpertSubset
        // predefined charsets. The CFF Standard Encoding + charset path in
        // `parse_cff_gid_mapping` handles these; defer. ~keep
        return parse_cff_gid_mapping(font_data);
    };

    let resolved = resolve_bytes_via_pdf_encoding(&charset_sids, &string_index, pdf_encoding, differences);

    if resolved.is_empty() {
        // PDF /Encoding yielded zero hits against the Charset. Fall back to
        // the CFF Encoding-driven path so we never make a working font worse. ~keep
        parse_cff_gid_mapping(font_data)
    } else {
        Some(resolved)
    }
}

/// Pure-input helper: given a parsed CFF Charset + String INDEX, build the
/// byte → GID map driven by the PDF font dictionary's `/Encoding` and
/// `/Differences`. Split out of [`parse_cff_gid_mapping_with_pdf_encoding`]
/// so the name-resolution logic can be tested without constructing a
/// custom CFF binary.
fn resolve_bytes_via_pdf_encoding(
    charset_sids: &[u16],
    string_index: &[&[u8]],
    pdf_encoding: &crate::fonts::font_dict::Encoding,
    differences: &HashMap<u8, String>,
) -> HashMap<u8, u16> {
    use crate::fonts::font_dict::{Encoding, FontInfo};

    // Glyph name → GID (lowest GID wins on duplicate names — first occurrence
    // in the Charset reflects the subsetter's primary mapping). ~keep
    let mut name_to_gid: HashMap<String, u16> = HashMap::new();
    for (gid, &sid) in charset_sids.iter().enumerate() {
        if gid == 0 {
            continue; // .notdef is implicit and not addressable by name ~keep
        }
        if let Some(name) = resolve_glyph_name(sid, string_index) {
            name_to_gid.entry(name).or_insert(gid as u16);
        }
    }

    let resolve_base_byte = |byte: u8| -> Option<&'static str> {
        match pdf_encoding {
            Encoding::Standard(name) => match name.as_str() {
                "MacRomanEncoding" => mac_roman_byte_to_name(byte),
                "StandardEncoding" => standard_encoding_byte_to_name(byte),
                // WinAnsiEncoding, MacExpertEncoding, PDFDocEncoding, and any
                // unrecognised string fall through to WinAnsi. Mac Expert is
                // a non-text variant; PDF Doc Encoding overlaps WinAnsi. ~keep
                _ => FontInfo::gid_to_standard_glyph_name(byte as u16),
            },
            Encoding::Custom(_) => FontInfo::gid_to_standard_glyph_name(byte as u16),
            Encoding::Identity => None, // handled by the outer guard already ~keep
        }
    };

    let mut out: HashMap<u8, u16> = HashMap::new();
    for byte_code in 0u16..256 {
        let byte = byte_code as u8;

        // §9.6.6: /Differences entries override the base predefined encoding. ~keep
        if let Some(diff_name) = differences.get(&byte)
            && let Some(&gid) = name_to_gid.get(diff_name)
        {
            out.insert(byte, gid);
            continue;
        }

        if let Some(name) = resolve_base_byte(byte)
            && let Some(&gid) = name_to_gid.get(name)
        {
            out.insert(byte, gid);
        }
    }
    out
}

#[cfg(test)]
mod tests;
