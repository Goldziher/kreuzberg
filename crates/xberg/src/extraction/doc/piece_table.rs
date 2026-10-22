//! Word97+ piece-table (`PlcPcd`) decoding: FIB field reads, per-piece text
//! decoding, and assembly into the document's subdocument text ranges.

use super::*;

/// Read a little-endian `u16` at `offset`, or an error naming `what` when `data` is too
/// short to contain it.
fn read_u16_checked(data: &[u8], offset: usize, what: &str) -> Result<u16> {
    if data.len() < offset + 2 {
        return Err(XbergError::parsing(format!("FIB too short for {what}")));
    }
    Ok(u16::from_le_bytes([data[offset], data[offset + 1]]))
}

/// Read a little-endian `u32` at `offset`, or an error naming `what` when `data` is too
/// short to contain it.
fn read_u32_checked(data: &[u8], offset: usize, what: &str) -> Result<u32> {
    if data.len() < offset + 4 {
        return Err(XbergError::parsing(format!("FIB too short for {what}")));
    }
    Ok(u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

/// Extract text from Word 97/2000/XP/2003 files using the piece table.
pub(super) fn extract_text_word97(
    word_doc: &[u8],
    table_stream: &[u8],
    warnings: &mut Vec<ProcessingWarning>,
    picture_limits: Option<&crate::extractors::security::SecurityLimits>,
) -> Result<MainText> {
    let fib_base_size = 32;
    let csw_offset = fib_base_size;
    let csw = read_u16_checked(word_doc, csw_offset, "csw")? as usize;
    let rg_w_offset = csw_offset + 2;
    let cslw_offset = rg_w_offset + csw * 2;

    let cslw = read_u16_checked(word_doc, cslw_offset, "cslw")? as usize;
    let rg_lw_offset = cslw_offset + 2;

    let ccp_text_offset = rg_lw_offset + FIB_LW_IDX_CCP_TEXT * 4;
    let ccp_text = read_u32_checked(word_doc, ccp_text_offset, "ccpText")? as usize;

    let subdoc_ranges = SubdocRanges::from_fib(word_doc, rg_lw_offset, ccp_text);
    let mut total_cp = subdoc_ranges.total_cp();
    if total_cp > 0 {
        total_cp += 1;
    }

    let cbrgfclcb_offset = rg_lw_offset + cslw * 4;
    read_u16_checked(word_doc, cbrgfclcb_offset, "cbRgFcLcb")?;
    let rg_fc_lcb_offset = cbrgfclcb_offset + 2;

    let fc_clx_offset = rg_fc_lcb_offset + FIB_FC_LCB_IDX_CLX * 8;
    let lcb_clx_offset = fc_clx_offset + 4;
    let fc_clx = read_u32_checked(word_doc, fc_clx_offset, "fcClx/lcbClx")? as usize;
    let lcb_clx = read_u32_checked(word_doc, lcb_clx_offset, "fcClx/lcbClx")? as usize;

    if fc_clx == 0 || lcb_clx == 0 {
        images::warn_unsupported_fallback(picture_limits.is_some(), warnings, "contiguous-text fallback");
        return extract_text_contiguous(word_doc, ccp_text).map(MainText::text_only);
    }

    if table_stream.len() < fc_clx + lcb_clx {
        return Err(XbergError::parsing("CLX extends beyond table stream"));
    }

    let clx = &table_stream[fc_clx..fc_clx + lcb_clx];

    let mut pos = 0;
    while pos < clx.len() {
        let clxt = clx[pos];
        if clxt == 0x02 {
            pos += 1;
            if pos + 4 > clx.len() {
                return Err(XbergError::parsing("Pcdt truncated at lcb"));
            }
            let _ = u32::from_le_bytes([clx[pos], clx[pos + 1], clx[pos + 2], clx[pos + 3]]) as usize;
            pos += 4;

            let plc_pcd = &clx[pos..];
            let tables = DocTables {
                lists: papx::ListTables::build(word_doc, table_stream, rg_fc_lcb_offset),
                stories: StoryTables::read(word_doc, table_stream, rg_fc_lcb_offset, &subdoc_ranges),
                picture_limits,
            };
            return extract_text_from_piece_table(word_doc, plc_pcd, &subdoc_ranges, total_cp, warnings, &tables);
        } else if clxt == 0x01 {
            pos += 1;
            if pos + 2 > clx.len() {
                break;
            }
            let cb_grpprl = u16::from_le_bytes([clx[pos], clx[pos + 1]]) as usize;
            pos += 2 + cb_grpprl;
        } else {
            break;
        }
    }

    images::warn_unsupported_fallback(picture_limits.is_some(), warnings, "unavailable-piece-table fallback");
    extract_text_fallback(word_doc, ccp_text).map(MainText::text_only)
}

/// Record that a piece's declared byte range runs past the end of the
/// WordDocument stream (#92). The piece table on a malformed or truncated
/// `.doc` can declare an FC/length pair that overruns the stream; previously
/// this was silently clamped (compressed pieces) or dropped entirely
/// (uncompressed pieces), producing a document that looked complete but was
/// truncated.
fn push_piece_overrun_warning(
    warnings: &mut Vec<ProcessingWarning>,
    piece_index: usize,
    declared_end: usize,
    stream_len: usize,
) {
    let message = format!(
        "Piece {piece_index} in the .doc piece table declares a byte range ending at \
         byte {declared_end}, past the end of the WordDocument stream ({stream_len} bytes); \
         the piece's text beyond the stream end was dropped"
    );
    crate::core::diagnostics::push_warning(warnings, DOC_WARNING_SOURCE, message);
}

/// A decoded piece: its characters, plus for each character the byte offset
/// (`FC`) in the WordDocument stream it was read from.
///
/// The two vectors are always the same length. Paragraph properties are
/// FC-addressed while text is CP-addressed, so #1550 needs this pairing to
/// bind a paragraph to its `PAPX`; the uncompressed path can drop a code unit
/// that is not a scalar value, which is why the offsets are recorded during
/// decoding rather than recomputed from an index afterwards.
///
/// `fc_ends` holds the offset one *past* each character, because that is what
/// an FKP's `rgfc` entry stores for a paragraph. Recording the end during
/// decoding avoids re-deriving it as `fc + 1`, which is only correct for
/// compressed pieces -- an uncompressed character is two bytes wide. ~keep
#[derive(Default)]
struct DecodedPiece {
    chars: Vec<char>,
    fc_ends: Vec<u32>,
}

impl DecodedPiece {
    fn len(&self) -> usize {
        self.chars.len()
    }

    fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }
}

/// Decode one piece's characters, recording each character's `FC`.
fn decode_piece_chars(
    word_doc: &[u8],
    fc_raw: u32,
    char_count: usize,
    piece_index: usize,
    warnings: &mut Vec<ProcessingWarning>,
) -> DecodedPiece {
    let is_compressed = (fc_raw & 0x4000_0000) != 0;
    let fc = (fc_raw & 0x3FFF_FFFF) as usize;
    // Compressed (CP1252) pieces address `word_doc` at half the raw FC value;
    // uncompressed (UTF-16LE) pieces address it directly. ~keep
    let byte_offset = if is_compressed { fc / 2 } else { fc };

    let decode_cp1252 = |start: usize, end: usize| -> DecodedPiece {
        if start >= end {
            return DecodedPiece::default();
        }
        DecodedPiece {
            chars: word_doc[start..end].iter().map(|&b| cp1252_to_char(b)).collect(),
            fc_ends: (start..end).map(|fc| (fc + 1) as u32).collect(),
        }
    };

    if is_compressed {
        let end = byte_offset + char_count;
        let available_end = if end > word_doc.len() {
            push_piece_overrun_warning(warnings, piece_index, end, word_doc.len());
            word_doc.len()
        } else {
            end
        };
        decode_cp1252(byte_offset, available_end)
    } else {
        let end = byte_offset + char_count * 2;
        let available_end = if end > word_doc.len() {
            push_piece_overrun_warning(warnings, piece_index, end, word_doc.len());
            byte_offset + ((word_doc.len().saturating_sub(byte_offset)) / 2) * 2
        } else {
            end
        };
        if byte_offset >= available_end {
            DecodedPiece::default()
        } else {
            let mut chars = Vec::new();
            let mut fc_ends = Vec::new();
            for (i, unit) in word_doc[byte_offset..available_end]
                .as_chunks::<2>()
                .0
                .iter()
                .enumerate()
            {
                if let Some(c) = char::from_u32(u16::from_le_bytes([unit[0], unit[1]]) as u32) {
                    chars.push(c);
                    fc_ends.push((byte_offset + i * 2 + 2) as u32);
                }
            }
            DecodedPiece { chars, fc_ends }
        }
    }
}

/// Append the overlap between `[cp_start, cp_start + chars.len())` and `range`
/// to `out`, translating the overlap into an index range on `chars`.
fn append_range_overlap(
    piece: &DecodedPiece,
    cp_start: usize,
    range: SubdocRange,
    out: &mut String,
    out_fcs: Option<&mut Vec<u32>>,
) {
    if range.len() == 0 {
        return;
    }
    let piece_end = cp_start + piece.len();
    let overlap_start = cp_start.max(range.start);
    let overlap_end = piece_end.min(range.end);
    if overlap_start < overlap_end {
        let from = overlap_start - cp_start;
        let to = overlap_end - cp_start;
        out.extend(&piece.chars[from..to]);
        if let Some(out_fcs) = out_fcs {
            out_fcs.extend_from_slice(&piece.fc_ends[from..to]);
        }
    }
}

/// Fields of [`extract_text_from_piece_table`] that [`process_piece`] needs but never
/// mutates, bundled purely to keep that function's parameter list manageable.
struct PieceTableContext<'a> {
    /// Total number of declared pieces (`PlcPcd` entries).
    n: usize,
    plc_size: usize,
    plc_pcd: &'a [u8],
    word_doc: &'a [u8],
    total_cp: usize,
    ranges: &'a SubdocRanges,
}

/// Process one entry of the piece table (`PlcPcd`), appending its decoded characters to
/// the matching subdocument range(s) in `text`. Returns `false` when the outer loop over
/// pieces in [`extract_text_from_piece_table`] must stop (a truncated table, or a piece
/// starting past `total_cp`); `true` otherwise, including when this particular piece
/// contributed nothing.
fn process_piece(
    i: usize,
    ctx: &PieceTableContext,
    warnings: &mut Vec<ProcessingWarning>,
    text: &mut SubdocumentText,
) -> bool {
    let cp_start_off = i * 4;
    let cp_end_off = (i + 1) * 4;
    let pcd_off = (ctx.n + 1) * 4 + i * 8;

    if cp_end_off + 4 > ctx.plc_size || pcd_off + 8 > ctx.plc_size {
        let n = ctx.n;
        crate::core::diagnostics::push_warning(
            warnings,
            DOC_WARNING_SOURCE,
            format!(
                "Piece table truncated after {i} of {n} declared pieces; remaining document text was not extracted"
            ),
        );
        return false;
    }

    let plc_pcd = ctx.plc_pcd;
    let cp_start = u32::from_le_bytes([
        plc_pcd[cp_start_off],
        plc_pcd[cp_start_off + 1],
        plc_pcd[cp_start_off + 2],
        plc_pcd[cp_start_off + 3],
    ]) as usize;

    let cp_end = u32::from_le_bytes([
        plc_pcd[cp_end_off],
        plc_pcd[cp_end_off + 1],
        plc_pcd[cp_end_off + 2],
        plc_pcd[cp_end_off + 3],
    ]) as usize;

    if cp_start >= ctx.total_cp {
        return false;
    }

    let fc_raw = u32::from_le_bytes([
        plc_pcd[pcd_off + 2],
        plc_pcd[pcd_off + 3],
        plc_pcd[pcd_off + 4],
        plc_pcd[pcd_off + 5],
    ]);

    let mut char_count = cp_end.saturating_sub(cp_start);
    if cp_start + char_count > ctx.total_cp {
        char_count = ctx.total_cp.saturating_sub(cp_start);
    }
    if char_count == 0 {
        return true;
    }

    let piece = decode_piece_chars(ctx.word_doc, fc_raw, char_count, i, warnings);
    if piece.is_empty() {
        return true;
    }

    let ranges = ctx.ranges;
    append_range_overlap(
        &piece,
        cp_start,
        ranges.main,
        &mut text.main,
        Some(&mut text.main_fc_ends),
    );
    append_range_overlap(&piece, cp_start, ranges.footnote, &mut text.footnote, None);
    append_range_overlap(&piece, cp_start, ranges.header, &mut text.header, None);
    append_range_overlap(&piece, cp_start, ranges.annotation, &mut text.annotation, None);
    append_range_overlap(&piece, cp_start, ranges.textbox, &mut text.textbox, None);
    true
}

/// Extract text from the piece table (PlcPcd), bucketing each piece's
/// characters into the subdocument range they fall in (#77: previously any
/// piece whose CP range started at or after `ccpText` -- i.e. every
/// footnote, header/footer, comment and text-box piece -- was silently
/// skipped).
fn extract_text_from_piece_table(
    word_doc: &[u8],
    plc_pcd: &[u8],
    ranges: &SubdocRanges,
    total_cp: usize,
    warnings: &mut Vec<ProcessingWarning>,
    tables: &DocTables<'_>,
) -> Result<MainText> {
    let plc_size = plc_pcd.len();
    if plc_size < 16 {
        return Err(XbergError::parsing("PlcPcd too small"));
    }

    let n = (plc_size - 4) / 12;
    if n == 0 {
        return Ok(MainText::text_only(String::new()));
    }

    let mut text = SubdocumentText::default();
    let piece_ctx = PieceTableContext {
        n,
        plc_size,
        plc_pcd,
        word_doc,
        total_cp,
        ranges,
    };

    for i in 0..n {
        if !process_piece(i, &piece_ctx, warnings, &mut text) {
            break;
        }
    }

    if ranges.has_unextracted_subdocument() {
        crate::core::diagnostics::push_warning(
            warnings,
            DOC_WARNING_SOURCE,
            "Document contains endnote and/or header-text-box content that is not extracted",
        );
    }

    let mut content = normalize_doc_text(&text.main);
    for (label, section) in [
        ("Footnotes", &text.footnote),
        ("Headers and Footers", &text.header),
        ("Comments", &text.annotation),
        ("Text Boxes", &text.textbox),
    ] {
        let normalized_section = normalize_doc_text(section);
        if !normalized_section.is_empty() {
            if !content.is_empty() {
                content.push_str("\n\n");
            }
            content.push_str(label);
            content.push_str("\n\n");
            content.push_str(&normalized_section);
        }
    }

    let (paragraphs, picture_positions) =
        split_paragraphs_and_pictures(&text.main, &text.main_fc_ends, &tables.lists, tables.picture_limits)?;
    let unsupported_pictures = text.main.contains('\x08')
        || [&text.footnote, &text.header, &text.annotation, &text.textbox]
            .iter()
            .any(|story| story.contains(['\x01', '\x08']));
    Ok(MainText {
        paragraphs,
        picture_positions,
        unsupported_pictures,
        subdocuments: collect_subdocuments(&text, &tables.stories, warnings),
        content,
    })
}

/// Table-stream structures the piece-table walk resolves text against.
struct DocTables<'a> {
    picture_limits: Option<&'a crate::extractors::security::SecurityLimits>,
    lists: papx::ListTables,
    stories: StoryTables,
}

/// Story boundaries of the subdocuments that hold more than one story, as CPs
/// relative to the subdocument's start. `None` when the table is absent or
/// unusable.
struct StoryTables {
    footnote: Option<Vec<usize>>,
    header: Option<Vec<usize>>,
    annotation: Option<Vec<usize>>,
}

impl StoryTables {
    fn read(word_doc: &[u8], table_stream: &[u8], rg_fc_lcb_offset: usize, ranges: &SubdocRanges) -> Self {
        let read =
            |index, range: SubdocRange| read_story_bounds(word_doc, table_stream, rg_fc_lcb_offset, index, range.len());
        Self {
            footnote: read(FIB_FC_LCB_IDX_PLCFFND_TXT, ranges.footnote),
            header: read(FIB_FC_LCB_IDX_PLCF_HDD, ranges.header)
                .filter(|bounds| header_table_has_whole_sections(bounds)),
            annotation: read(FIB_FC_LCB_IDX_PLCFAND_TXT, ranges.annotation),
        }
    }
}

/// Read a story table (`PlcffndTxt`, `PlcfHdd`, `PlcfandTxt`) into the CP each
/// story starts at, followed by the end of the last story.
///
/// [MS-DOC] gives these tables one CP per story, then the end of the last
/// story, which must equal `ccp - 1`, then a final CP readers must ignore. A
/// table that does not fit the table stream, runs backwards or does not cover
/// the subdocument from CP 0 to `ccp - 1` is treated as absent; splitting by it
/// would silently drop the uncovered text. ~keep
fn read_story_bounds(
    word_doc: &[u8],
    table_stream: &[u8],
    rg_fc_lcb_offset: usize,
    index: usize,
    ccp: usize,
) -> Option<Vec<usize>> {
    let (fc, lcb) = papx::read_fc_lcb(word_doc, rg_fc_lcb_offset, index)?;
    // A single story already takes three 4-byte CPs.
    if lcb < 12 || lcb % 4 != 0 {
        return None;
    }
    let plc = table_stream.get(fc..fc.checked_add(lcb)?)?;
    let mut bounds: Vec<usize> = plc
        .as_chunks::<4>()
        .0
        .iter()
        .map(|cp| u32::from_le_bytes([cp[0], cp[1], cp[2], cp[3]]) as usize)
        .collect();
    bounds.pop();
    let ordered = bounds.windows(2).all(|pair| pair[0] <= pair[1]);
    let covers_subdocument = bounds.first() == Some(&0) && bounds.last() == Some(&ccp.checked_sub(1)?);
    (ordered && covers_subdocument).then_some(bounds)
}

/// The header subdocument opens with six footnote and endnote separator
/// stories, then holds six stories per section: even header, odd header, even
/// footer, odd footer, first-page header, first-page footer ([MS-DOC]
/// `Plcfhdd`). ~keep
const HEADER_SEPARATOR_STORIES: usize = 6;
const HEADER_STORIES_PER_SECTION: usize = 6;

/// Whether a `PlcfHdd` has the shape above: the separator stories and whole
/// sections, at least one. Any other count would file stories under the wrong
/// kind or drop them as separators, so such a table counts as malformed.
fn header_table_has_whole_sections(bounds: &[usize]) -> bool {
    let stories = bounds.len().saturating_sub(1);
    stories > HEADER_SEPARATOR_STORIES
        && (stories - HEADER_SEPARATOR_STORIES).is_multiple_of(HEADER_STORIES_PER_SECTION)
}

/// Whether header story `index` is a header or a footer; `None` for a separator.
fn header_story_kind(index: usize) -> Option<DocSubdocumentKind> {
    match index.checked_sub(HEADER_SEPARATOR_STORIES)? % HEADER_STORIES_PER_SECTION {
        0 | 1 | 4 => Some(DocSubdocumentKind::Header),
        _ => Some(DocSubdocumentKind::Footer),
    }
}

/// Cut a subdocument's text (one char per CP) at `bounds` and normalize each
/// story. A story the piece table did not fully cover comes out short.
fn split_stories(text: &str, bounds: &[usize]) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    bounds
        .windows(2)
        .map(|story| {
            let start = story[0].min(chars.len());
            let end = story[1].min(chars.len());
            normalize_doc_text(&chars[start..end].iter().collect::<String>())
        })
        .collect()
}

/// Split the footnote, header and comment subdocuments into their stories, in
/// the order `content` lists them, and add the text-box subdocument whole.
/// Without a usable story table a subdocument stays one story, with a warning.
fn collect_subdocuments(
    text: &SubdocumentText,
    tables: &StoryTables,
    warnings: &mut Vec<ProcessingWarning>,
) -> Vec<DocSubdocument> {
    let mut subdocuments = Vec::new();
    for (raw, bounds, kind, fallback_kind, fallback_warning) in [
        (
            &text.footnote,
            &tables.footnote,
            DocSubdocumentKind::Footnote,
            DocSubdocumentKind::Footnote,
            "Footnote table (PlcffndTxt) is missing or malformed; all footnote text is reported as one footnote",
        ),
        (
            &text.header,
            &tables.header,
            DocSubdocumentKind::Header,
            DocSubdocumentKind::HeaderFooter,
            "Header table (PlcfHdd) is missing or malformed; all header and footer text is reported as combined header/footer text",
        ),
        (
            &text.annotation,
            &tables.annotation,
            DocSubdocumentKind::Comment,
            DocSubdocumentKind::Comment,
            "Comment table (PlcfandTxt) is missing or malformed; all comment text is reported as one comment",
        ),
    ] {
        let Some(bounds) = bounds else {
            let whole = normalize_doc_text(raw);
            if !whole.is_empty() {
                crate::core::diagnostics::push_warning(warnings, DOC_WARNING_SOURCE, fallback_warning);
                subdocuments.push(DocSubdocument {
                    kind: fallback_kind,
                    text: whole,
                });
            }
            continue;
        };
        for (index, story) in split_stories(raw, bounds).into_iter().enumerate() {
            let story_kind = if kind == DocSubdocumentKind::Header {
                header_story_kind(index)
            } else {
                Some(kind)
            };
            if let Some(kind) = story_kind
                && !story.is_empty()
            {
                subdocuments.push(DocSubdocument { kind, text: story });
            }
        }
    }

    let textbox = normalize_doc_text(&text.textbox);
    if !textbox.is_empty() {
        subdocuments.push(DocSubdocument {
            kind: DocSubdocumentKind::TextBox,
            text: textbox,
        });
    }
    subdocuments
}
