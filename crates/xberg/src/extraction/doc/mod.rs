//! Native DOC (Word 97-2003) text extraction.
//!
//! Extracts text directly from Word Binary File Format using OLE/CFB
//! compound document parsing, without requiring LibreOffice.
//!
//! Supports Word 97, 2000, XP, and 2003 (.doc) files.

mod papx;

use crate::error::{Result, XbergError};
use crate::types::ProcessingWarning;
use std::io::Cursor;

/// Warning source tag for `.doc` extraction diagnostics (#171 convention).
const DOC_WARNING_SOURCE: &str = "doc";

/// One main-document paragraph, carrying the list binding Word gave it.
///
/// #1550: the extractor previously returned text only, so a paragraph Word
/// numbers automatically was indistinguishable from prose. This carries
/// membership and depth; the number Word paints is deliberately not resolved
/// (that needs `PlfLst`/`PlfLfo` and a counter walk). ~keep
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct DocParagraph {
    pub content: String,
    /// `Some(..)` when the paragraph is bound to an automatic list. `None`
    /// means ordinary prose.
    pub list: Option<DocListMembership>,
    /// `Some(level)` when the paragraph's style resolves to `heading 1`..
    /// `heading 9`, directly or through its base style.
    pub heading_level: Option<u8>,
    /// `Some(..)` when the paragraph is in a table.
    #[serde(default)]
    pub table: Option<DocTableMark>,
}

/// Where a table paragraph falls in its table.
///
/// Word ends the last paragraph of each cell with a cell mark instead of a
/// paragraph mark, and ends each row with a paragraph holding only a row
/// mark. A paragraph that is neither is one of several in its cell. A nested
/// table's paragraphs end with paragraph marks, so its text stays in the cell
/// of the outer table that holds it. ~keep
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct DocTableMark {
    /// The paragraph ends a cell.
    pub cell_end: bool,
    /// The paragraph ends a row.
    pub row_end: bool,
}

/// What the Word97+ text path produces: the assembled text plus the paragraph
/// structure behind it.
struct MainText {
    content: String,
    paragraphs: Vec<DocParagraph>,
    subdocuments: Vec<DocSubdocument>,
}

impl MainText {
    /// A document whose paragraph properties were not read -- Word 6/95, or
    /// the contiguous fallback. Callers fall back to `content`.
    fn text_only(content: String) -> Self {
        Self {
            content,
            paragraphs: Vec::new(),
            subdocuments: Vec::new(),
        }
    }
}

/// One header, footer, footnote, comment or the text-box subdocument.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct DocSubdocument {
    pub kind: DocSubdocumentKind,
    /// Normalized text, paragraphs separated by `\n`.
    pub text: String,
}

/// Where a [`DocSubdocument`] comes from. Headers and footers share one
/// subdocument; `PlcfHdd` tells them apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum DocSubdocumentKind {
    Header,
    Footer,
    HeaderFooter,
    Footnote,
    Comment,
    TextBox,
}

/// A paragraph's place in an automatic list.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub(crate) struct DocListMembership {
    /// Zero-based nesting depth (`ilvl`).
    pub level: u8,
    /// Whether the level paints numbers rather than bullets, resolved from the
    /// list tables' `nfc`.
    pub ordered: bool,
}

/// Result of DOC text extraction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct DocExtractionResult {
    /// Extracted text content. Aliased as `text` for back-compat.
    pub content: String,
    /// Document metadata.
    pub metadata: DocMetadata,
    /// Non-fatal degradations encountered while extracting (see
    /// `core::diagnostics`). Empty when extraction was complete.
    pub processing_warnings: Vec<ProcessingWarning>,
    /// Main-document paragraphs. Empty when the document took a path that
    /// carries no paragraph properties (Word 6/95, or the contiguous
    /// fallback), in which case callers keep using `content`. ~keep
    pub paragraphs: Vec<DocParagraph>,
    /// Header, footer, footnote, comment and text-box text, one entry per
    /// story or note. `content` carries the same text as labelled sections;
    /// `paragraphs` does not. Empty on the paths that carry no paragraph
    /// properties (Word 6/95, the contiguous fallback). On the piece-table
    /// path it can be non-empty while `paragraphs` is empty: a document whose
    /// only text is a header or footer.
    pub subdocuments: Vec<DocSubdocument>,
}

/// Metadata extracted from DOC files.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct DocMetadata {
    pub title: Option<String>,
    pub subject: Option<String>,
    pub author: Option<String>,
    pub last_author: Option<String>,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub revision_number: Option<String>,
}

/// Extract text from DOC bytes.
///
/// Parses the OLE/CFB compound document, reads the FIB (File Information Block),
/// and extracts text from the piece table.
pub(crate) fn extract_doc_text(content: &[u8]) -> Result<DocExtractionResult> {
    let cursor = Cursor::new(content);
    let mut comp = cfb::CompoundFile::open(cursor)
        .map_err(|e| XbergError::parsing(format!("Failed to open DOC as OLE container: {e}")))?;

    let metadata = extract_doc_metadata(&mut comp);

    let word_doc = read_stream(&mut comp, "/WordDocument")?;
    if word_doc.len() < 12 {
        return Err(XbergError::parsing("WordDocument stream too short"));
    }

    let w_ident = u16::from_le_bytes([word_doc[0], word_doc[1]]);
    if !matches!(w_ident, 0xA5DC | 0xA5EC) {
        return Err(XbergError::parsing(format!(
            "Invalid DOC magic number: 0x{w_ident:04X}, expected 0xA5DC or 0xA5EC"
        )));
    }

    let n_fib = u16::from_le_bytes([word_doc[2], word_doc[3]]);
    let flags_a = u16::from_le_bytes([word_doc[0x0A], word_doc[0x0B]]);
    if w_ident == 0xA5DC || (0x0065..=0x0068).contains(&n_fib) {
        if flags_a & 0x0004 != 0 {
            return Err(XbergError::parsing(
                "Fast-saved Word 6/95 documents are not supported; save the document without fast-save history and retry",
            ));
        }
        return extract_text_word6(&word_doc).map(|text| DocExtractionResult {
            content: text,
            metadata,
            processing_warnings: Vec::new(),
            paragraphs: Vec::new(),
            subdocuments: Vec::new(),
        });
    }
    let use_1table = (flags_a & 0x0200) != 0;

    let table_stream_name = if use_1table { "/1Table" } else { "/0Table" };

    let table_stream = read_stream(&mut comp, table_stream_name)?;

    let mut processing_warnings = Vec::new();

    extract_text_word97(&word_doc, &table_stream, &mut processing_warnings).map(|main| DocExtractionResult {
        content: main.content,
        metadata,
        processing_warnings,
        paragraphs: main.paragraphs,
        subdocuments: main.subdocuments,
    })
}

/// Index of `ccpText` (main document CP count) in the FIB's `FibRgLw97`
/// long-word array.
const FIB_LW_IDX_CCP_TEXT: usize = 3;

/// Index of the `fcClx`/`lcbClx` pair in the FIB's `FibRgFcLcb97` array.
///
/// [MS-DOC] 2.5.5 orders the array `fcStshfOrig`(0) ... `fcSttbfAssoc`(32),
/// `fcClx`(33). This read used index 66 (`fcBkdFtnOldOld`, an obsolete field
/// Word writes as zero) until #1551, so `fc_clx == 0` held for every real
/// document and the piece table was never walked -- the whole `Clx` path was
/// unreachable at runtime and only the contiguous fallback ever ran. ~keep
const FIB_FC_LCB_IDX_CLX: usize = 33;
/// Index of the `fcPlcffndTxt`/`lcbPlcffndTxt` pair: where each footnote starts
/// in the footnote subdocument.
const FIB_FC_LCB_IDX_PLCFFND_TXT: usize = 3;
/// Index of the `fcPlcfandTxt`/`lcbPlcfandTxt` pair: where each comment starts.
const FIB_FC_LCB_IDX_PLCFAND_TXT: usize = 5;
/// Index of the `fcPlcfHdd`/`lcbPlcfHdd` pair: where each header and footer
/// story starts in the header subdocument.
const FIB_FC_LCB_IDX_PLCF_HDD: usize = 11;
/// Index of `ccpFtn` (footnote subdocument CP count) in the FIB's `FibRgLw97`
/// long-word array.
const FIB_LW_IDX_CCP_FTN: usize = 4;
/// Index of `ccpHdd` (header/footer subdocument CP count).
const FIB_LW_IDX_CCP_HDD: usize = 5;
// Index 6 (`ccpMcr`) is a deprecated macro-subdocument CP count that the spec
// requires readers to ignore; it does not occupy space in the piece-table CP
// range and is intentionally not modeled here.
/// Index of `ccpAtn` (annotation/comment subdocument CP count).
const FIB_LW_IDX_CCP_ATN: usize = 7;
/// Index of `ccpEdn` (endnote subdocument CP count).
const FIB_LW_IDX_CCP_EDN: usize = 8;
/// Index of `ccpTxbx` (main-document text-box subdocument CP count).
const FIB_LW_IDX_CCP_TXBX: usize = 9;
/// Index of `ccpHdrTxbx` (header text-box subdocument CP count).
const FIB_LW_IDX_CCP_HDR_TXBX: usize = 10;

/// Read one `u32` field from the FIB's `FibRgLw97` long-word array.
///
/// Returns 0 when the FIB is too short to contain the field, matching this
/// module's existing "degrade gracefully" behavior for the base FIB reads.
fn read_lw_field(word_doc: &[u8], rg_lw_offset: usize, index: usize) -> usize {
    let off = rg_lw_offset + index * 4;
    if word_doc.len() < off + 4 {
        return 0;
    }
    u32::from_le_bytes([word_doc[off], word_doc[off + 1], word_doc[off + 2], word_doc[off + 3]]) as usize
}

/// A named half-open range in the piece table's CP (character position) space.
///
/// Word lays the main document body and its subdocuments (footnotes,
/// headers/footers, comments, text boxes, ...) out as consecutive CP ranges
/// sharing a single piece table (MS-DOC 2.4.2 "Retrieving Text").
#[derive(Debug, Clone, Copy)]
struct SubdocRange {
    start: usize,
    end: usize,
}

impl SubdocRange {
    fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

/// The CP-space subdocument ranges this extractor recognizes and extracts,
/// plus whether any *unrecognized* subdocument (endnotes, header text boxes)
/// consumes CPs -- tracked only so callers can warn that that content was not
/// extracted (#77 covers footnotes/headers/comments/text boxes, not endnotes
/// or header text boxes).
struct SubdocRanges {
    main: SubdocRange,
    footnote: SubdocRange,
    header: SubdocRange,
    annotation: SubdocRange,
    /// Endnotes are not extracted (out of #77's stated scope), but their CP
    /// span must still be accounted for so later subdocuments (text boxes)
    /// resolve to the correct offsets.
    endnote: SubdocRange,
    textbox: SubdocRange,
    /// Header text boxes are not extracted (out of #77's stated scope); kept
    /// only to compute `total_cp` correctly and to detect that content was
    /// left out (see `has_unextracted_subdocument`).
    header_textbox: SubdocRange,
}

impl SubdocRanges {
    /// Derive the CP-space layout from the FIB's `ccp*` fields.
    ///
    /// Order matches the FIB's `FibRgLw97` field declaration order, which is
    /// also the documents' physical CP-space layout (MS-DOC 2.4.2): main
    /// document, footnotes, headers/footers, comments, endnotes, text boxes,
    /// header text boxes. `ccpMcr` (the deprecated macro subdocument) is
    /// skipped: it does not occupy CP space.
    fn from_fib(word_doc: &[u8], rg_lw_offset: usize, ccp_text: usize) -> Self {
        let ccp_ftn = read_lw_field(word_doc, rg_lw_offset, FIB_LW_IDX_CCP_FTN);
        let ccp_hdd = read_lw_field(word_doc, rg_lw_offset, FIB_LW_IDX_CCP_HDD);
        let ccp_atn = read_lw_field(word_doc, rg_lw_offset, FIB_LW_IDX_CCP_ATN);
        let ccp_edn = read_lw_field(word_doc, rg_lw_offset, FIB_LW_IDX_CCP_EDN);
        let ccp_txbx = read_lw_field(word_doc, rg_lw_offset, FIB_LW_IDX_CCP_TXBX);
        let ccp_hdr_txbx = read_lw_field(word_doc, rg_lw_offset, FIB_LW_IDX_CCP_HDR_TXBX);

        let main = SubdocRange {
            start: 0,
            end: ccp_text,
        };
        let footnote = SubdocRange {
            start: main.end,
            end: main.end + ccp_ftn,
        };
        let header = SubdocRange {
            start: footnote.end,
            end: footnote.end + ccp_hdd,
        };
        let annotation = SubdocRange {
            start: header.end,
            end: header.end + ccp_atn,
        };
        let endnote = SubdocRange {
            start: annotation.end,
            end: annotation.end + ccp_edn,
        };
        let textbox = SubdocRange {
            start: endnote.end,
            end: endnote.end + ccp_txbx,
        };
        let header_textbox = SubdocRange {
            start: textbox.end,
            end: textbox.end + ccp_hdr_txbx,
        };

        Self {
            main,
            footnote,
            header,
            annotation,
            endnote,
            textbox,
            header_textbox,
        }
    }

    /// Whether the document has endnote or header-text-box content that this
    /// extractor does not extract.
    fn has_unextracted_subdocument(&self) -> bool {
        self.endnote.len() > 0 || self.header_textbox.len() > 0
    }

    /// Total CP-space size across every subdocument, including the ones this
    /// extractor does not (yet) extract -- needed so the piece-table walk
    /// doesn't stop before consuming pieces that belong to earlier
    /// subdocuments just because a later one is unaccounted for.
    fn total_cp(&self) -> usize {
        self.header_textbox.end
    }
}

/// Text collected from each recognized subdocument during a single pass over
/// the piece table.
#[derive(Default)]
struct SubdocumentText {
    main: String,
    /// Byte offset one past each character in `main`, same length as its char
    /// count. Only the main document needs it: #1550 binds paragraph
    /// properties for body text, and subdocument paragraphs carry no list
    /// numbering a reader would see. ~keep
    main_fc_ends: Vec<u32>,
    footnote: String,
    header: String,
    annotation: String,
    textbox: String,
}

mod piece_table;
use piece_table::extract_text_word97;

/// Word's paragraph mark. Splitting the raw main text on it gives the
/// document's own paragraph granularity, which is finer than the blank-line
/// chunking `content` consumers use. ~keep
const PARAGRAPH_MARK: char = '\r';

/// Word's cell mark, which ends a cell's last paragraph and a table row.
const CELL_MARK: char = '\x07';

/// Split the raw main text into paragraphs and attach each one's list binding.
///
/// Operates on the *raw* text so that character positions still line up with
/// `main_fcs`; each paragraph's text is normalized individually afterwards.
/// `content` is still produced by normalizing the whole main text in one pass,
/// so it stays byte-identical to what this module produced before #1550.
///
/// Fields are matched over the whole text too, before it is split: a field
/// can open before a paragraph mark and close after it -- Word ends a
/// hyperlink laid over a whole paragraph after that paragraph's mark, and a
/// `TOC` result holds every entry's paragraph -- and on its own the paragraph
/// holding the begin would see it unterminated and keep the instruction as
/// text.
///
/// A cell mark ends a paragraph too when the paragraph is in a table, so each
/// cell is its own paragraph instead of the row's cells running together. ~keep
fn split_main_paragraphs(main: &str, main_fc_ends: &[u32], list_tables: &papx::ListTables) -> Vec<DocParagraph> {
    let markup = field_markup_mask(main);
    let mut paragraphs = Vec::new();
    let mut raw = String::new();
    let mut after_last_mark = false;
    let mut char_count = 0usize;

    for (i, (c, is_markup)) in main.chars().zip(markup).enumerate() {
        char_count = i + 1;
        if c == PARAGRAPH_MARK || c == CELL_MARK {
            let mark_fc_end = main_fc_ends.get(i).copied();
            let table = mark_fc_end.and_then(|fc_end| list_tables.table_for_paragraph_end(fc_end));
            if c == PARAGRAPH_MARK || table.is_some() {
                let table = table.map(|table| DocTableMark {
                    cell_end: c == CELL_MARK && !table.row_end,
                    row_end: c == CELL_MARK && table.row_end,
                });
                push_paragraph(&mut paragraphs, &raw, mark_fc_end, table, list_tables);
                raw.clear();
                after_last_mark = false;
                continue;
            }
        }
        after_last_mark = true;
        if !is_markup {
            raw.push(c);
        }
    }

    if after_last_mark {
        // A final run with no paragraph mark still has properties keyed on the
        // FC one past its last character.
        let last_fc = main_fc_ends.get(char_count.saturating_sub(1)).copied();
        push_paragraph(&mut paragraphs, &raw, last_fc, None, list_tables);
    }

    paragraphs
}

/// Normalize one paragraph's raw text, field markup already taken out, and
/// record it when it survives. A table paragraph always does: an empty cell
/// still holds its column, and a row's end mark is empty.
fn push_paragraph(
    out: &mut Vec<DocParagraph>,
    raw: &str,
    mark_fc_end: Option<u32>,
    table: Option<DocTableMark>,
    list_tables: &papx::ListTables,
) {
    let content = normalize_doc_text(raw);
    if content.is_empty() && table.is_none() {
        return;
    }
    // Word keys a paragraph's PAPX on the FC one past its paragraph mark,
    // which is exactly what `fc_ends` recorded for that character.
    let list = mark_fc_end
        .and_then(|fc_end| list_tables.membership_for_paragraph_end(fc_end))
        .map(|(level, ordered)| DocListMembership { level, ordered });
    let heading_level = mark_fc_end.and_then(|fc_end| list_tables.heading_level_for_paragraph_end(fc_end));
    out.push(DocParagraph {
        content,
        list,
        heading_level,
        table,
    });
}

/// Extract text from a "simple" DOC file where text is stored contiguously.
///
/// When fcClx=0, the text is stored at offset `fcMin` in the WordDocument stream
/// as either CP1252 (compressed) or UTF-16LE (uncompressed).
fn extract_text_contiguous(word_doc: &[u8], ccp_text: usize) -> Result<String> {
    if word_doc.len() < 0x20 {
        return extract_text_fallback(word_doc, ccp_text);
    }

    let fc_min = u32::from_le_bytes([word_doc[0x18], word_doc[0x19], word_doc[0x1A], word_doc[0x1B]]) as usize;
    let fc_mac = u32::from_le_bytes([word_doc[0x1C], word_doc[0x1D], word_doc[0x1E], word_doc[0x1F]]) as usize;

    if fc_min == 0 || fc_min >= word_doc.len() {
        return extract_text_fallback(word_doc, ccp_text);
    }

    let data_len = fc_mac.saturating_sub(fc_min).min(word_doc.len() - fc_min);
    if data_len == 0 {
        return extract_text_fallback(word_doc, ccp_text);
    }

    let text_data = &word_doc[fc_min..fc_min + data_len];

    let null_count = text_data.iter().filter(|&&b| b == 0).count();
    let is_unicode = data_len >= ccp_text * 2 || null_count > data_len / 4;

    let text = if is_unicode {
        let chars: Vec<u16> = text_data
            .as_chunks::<2>()
            .0
            .iter()
            .take(ccp_text)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&chars)
    } else {
        text_data.iter().take(ccp_text).map(|&b| cp1252_to_char(b)).collect()
    };

    let normalized = normalize_doc_text(&text);
    if normalized.is_empty() {
        return extract_text_fallback(word_doc, ccp_text);
    }

    Ok(normalized)
}

/// Fallback text extraction for when the piece table is unavailable.
///
/// Attempts to extract readable text from the WordDocument stream directly.
fn extract_text_fallback(word_doc: &[u8], _ccp_text: usize) -> Result<String> {
    let mut result = String::new();
    let mut text_run = String::new();

    for &b in word_doc.iter().skip(256) {
        if b == 0x0D || b == 0x0A || b == 0x09 || (0x20..=0xFE).contains(&b) {
            text_run.push(cp1252_to_char(b));
        } else if !text_run.is_empty() {
            if text_run.len() >= 3 {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(&text_run);
            }
            text_run.clear();
        }
    }

    if text_run.len() >= 3 {
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(&text_run);
    }

    if result.is_empty() {
        return Err(XbergError::parsing("No text content found in DOC file"));
    }

    Ok(normalize_doc_text(&result))
}

/// Extract text from Word 6/95 files.
///
/// Word 6/95 has a simpler format where text starts at a known offset.
fn extract_text_word6(word_doc: &[u8]) -> Result<String> {
    if word_doc.len() < 0x50 {
        return Err(XbergError::parsing("Word 6/95 file too short"));
    }

    let ccp_text = u32::from_le_bytes([word_doc[0x34], word_doc[0x35], word_doc[0x36], word_doc[0x37]]) as usize;

    let fc_min = u32::from_le_bytes([word_doc[0x18], word_doc[0x19], word_doc[0x1A], word_doc[0x1B]]) as usize;

    let Some(fc_mac) = fc_min.checked_add(ccp_text) else {
        return Err(XbergError::parsing(
            "Word 6/95 text range overflows the document stream",
        ));
    };
    if fc_mac > word_doc.len() {
        return extract_text_fallback(word_doc, ccp_text);
    }

    let text_bytes = &word_doc[fc_min..fc_mac];
    let mut result = String::with_capacity(ccp_text);

    for &b in text_bytes {
        result.push(cp1252_to_char(b));
    }

    Ok(normalize_doc_text(&result))
}

/// Word field BEGIN marker. Text from here to [`FIELD_SEPARATOR`] is the field
/// *instruction* (`HYPERLINK "…"`, `PAGEREF _Toc1 \h`, `TOC \o "1-3"`), i.e. markup.
const FIELD_BEGIN: char = '\x13';

/// Word field SEPARATOR marker. Text from here to [`FIELD_END`] is the field
/// *result* — the only part a reader sees, and the only part that is document text.
const FIELD_SEPARATOR: char = '\x14';

/// Word field END marker.
const FIELD_END: char = '\x15';

/// Word non-breaking hyphen (`0x1E` in the binary text stream).
///
/// This is a *visible* character — the reader sees a hyphen; the only thing
/// "non-breaking" suppresses is a line break at that position. Dropping it
/// welds the two halves of a compound together (`twenty-one` → `twentyone`),
/// which corrupts the word rather than merely losing formatting.
///
/// Emitted as U+2011 NON-BREAKING HYPHEN rather than ASCII `-` to match the
/// DOCX parser, which maps `w:noBreakHyphen` — the same character in the modern
/// serialization of the same Word document model — to U+2011 (#224). The same
/// document saved as `.doc` and as `.docx` must extract to the same text.
const NON_BREAKING_HYPHEN: char = '\u{2011}';

/// Record, for each [`FIELD_BEGIN`] in `text` (in order of occurrence), whether it
/// has a matching [`FIELD_END`].
///
/// Fields nest — a `TOC` result is full of `PAGEREF` fields — so matching is
/// innermost-first via a stack. Begins left on the stack at end of input are
/// unterminated and reported as `false`.
///
/// An unterminated begin is deliberately *not* treated as opening a suppression
/// region by the caller: doing so would swallow the entire remainder of a document
/// whose stream happens to carry one stray `0x13`. Degrading to the historical
/// behaviour (the instruction leaks) is far cheaper than losing the document tail.
fn scan_field_begin_termination(text: &str) -> Vec<bool> {
    let mut terminated: Vec<bool> = Vec::new();
    let mut open: Vec<usize> = Vec::new();

    for c in text.chars() {
        match c {
            FIELD_BEGIN => {
                terminated.push(false);
                open.push(terminated.len() - 1);
            }
            FIELD_END => {
                // A stray END with nothing open is ignored rather than panicking:
                // this is user-supplied binary content.
                if let Some(index) = open.pop() {
                    terminated[index] = true;
                }
            }
            _ => {}
        }
    }

    terminated
}

/// Normalize extracted DOC text: strip field instructions, convert special
/// characters and clean up whitespace.
fn normalize_doc_text(text: &str) -> String {
    let stripped = strip_doc_field_instructions(text);
    collapse_excess_newlines(&stripped).trim().to_string()
}

/// Mark, for each character of `text`, whether it is field markup rather than
/// document text: a field marker, or part of a field instruction.
///
/// Field handling (#1460): the text between [`FIELD_BEGIN`] and [`FIELD_SEPARATOR`]
/// is the field instruction and is markup, not document text, so it is dropped;
/// the result between [`FIELD_SEPARATOR`] and [`FIELD_END`] is kept. A terminated
/// field with no separator has no result and therefore contributes nothing.
fn field_markup_mask(text: &str) -> Vec<bool> {
    let mut markup = Vec::with_capacity(text.len());

    let begin_terminated = scan_field_begin_termination(text);
    let mut begin_ordinal = 0usize;
    // One entry per open field; `true` while that field is still in its
    // instruction part.
    let mut field_stack: Vec<bool> = Vec::new();
    // Count of `field_stack` entries still in their instruction part. Text is
    // suppressed whenever this is non-zero, which is what makes nesting work:
    // a `PAGEREF` inside a `TOC` instruction stays suppressed even after the
    // inner field reaches its own separator.
    let mut instruction_depth = 0usize;

    for c in text.chars() {
        match c {
            FIELD_BEGIN => {
                let terminated = begin_terminated.get(begin_ordinal).copied().unwrap_or(false);
                begin_ordinal += 1;
                if terminated {
                    field_stack.push(true);
                    instruction_depth += 1;
                }
                markup.push(true);
            }
            FIELD_SEPARATOR => {
                if let Some(in_instruction) = field_stack.last_mut()
                    && *in_instruction
                {
                    *in_instruction = false;
                    instruction_depth -= 1;
                }
                markup.push(true);
            }
            FIELD_END => {
                if let Some(in_instruction) = field_stack.pop()
                    && in_instruction
                {
                    instruction_depth -= 1;
                }
                markup.push(true);
            }
            _ => markup.push(instruction_depth > 0),
        }
    }

    markup
}

/// Strip field instructions and convert DOC's special control characters to their
/// plain-text equivalents (see [`field_markup_mask`] for what a field leaves).
fn strip_doc_field_instructions(text: &str) -> String {
    let mut result = String::with_capacity(text.len());

    for (c, is_markup) in text.chars().zip(field_markup_mask(text)) {
        if is_markup {
            continue;
        }

        match c {
            '\r' => result.push('\n'),
            '\x07' => result.push('\t'),
            '\x0B' => result.push('\n'),
            '\x0C' => result.push('\n'),
            '\x01' | '\x08' => {}
            '\x1E' => result.push(NON_BREAKING_HYPHEN),
            // `0x1F` is the *optional* (soft) hyphen: invisible unless the line
            // happens to break there, so discarding it is correct and must stay
            // that way — emitting it would insert a hyphen the reader never saw.
            c if c < '\x20' && c != '\n' && c != '\t' => {}
            _ => result.push(c),
        }
    }

    result
}

/// Collapse three or more consecutive newlines down to two (a single blank line).
fn collapse_excess_newlines(text: &str) -> String {
    let mut prev_newline = false;
    let mut prev_prev_newline = false;
    let mut cleaned = String::with_capacity(text.len());

    for c in text.chars() {
        if c == '\n' {
            if prev_prev_newline && prev_newline {
                continue;
            }
            prev_prev_newline = prev_newline;
            prev_newline = true;
        } else {
            prev_prev_newline = false;
            prev_newline = false;
        }
        cleaned.push(c);
    }

    cleaned
}

/// Convert CP1252 byte to Unicode char.
fn cp1252_to_char(b: u8) -> char {
    match b {
        0x80 => '\u{20AC}',
        0x82 => '\u{201A}',
        0x83 => '\u{0192}',
        0x84 => '\u{201E}',
        0x85 => '\u{2026}',
        0x86 => '\u{2020}',
        0x87 => '\u{2021}',
        0x88 => '\u{02C6}',
        0x89 => '\u{2030}',
        0x8A => '\u{0160}',
        0x8B => '\u{2039}',
        0x8C => '\u{0152}',
        0x8E => '\u{017D}',
        0x91 => '\u{2018}',
        0x92 => '\u{2019}',
        0x93 => '\u{201C}',
        0x94 => '\u{201D}',
        0x95 => '\u{2022}',
        0x96 => '\u{2013}',
        0x97 => '\u{2014}',
        0x98 => '\u{02DC}',
        0x99 => '\u{2122}',
        0x9A => '\u{0161}',
        0x9B => '\u{203A}',
        0x9C => '\u{0153}',
        0x9E => '\u{017E}',
        0x9F => '\u{0178}',
        b => b as char,
    }
}

/// Read a named stream from the CFB compound file.
fn read_stream(comp: &mut cfb::CompoundFile<Cursor<&[u8]>>, name: &str) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut stream = comp
        .open_stream(name)
        .map_err(|e| XbergError::parsing(format!("Failed to open stream '{name}': {e}")))?;
    let mut data = Vec::new();
    stream
        .read_to_end(&mut data)
        .map_err(|e| XbergError::parsing(format!("Failed to read stream '{name}': {e}")))?;
    Ok(data)
}

mod metadata;
use metadata::extract_doc_metadata;

#[cfg(test)]
pub(crate) mod tests;
