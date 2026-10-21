use super::*;

#[test]
fn test_cp1252_to_char_ascii() {
    assert_eq!(cp1252_to_char(b'A'), 'A');
    assert_eq!(cp1252_to_char(b' '), ' ');
    assert_eq!(cp1252_to_char(b'\n'), '\n');
}

#[test]
fn test_cp1252_to_char_special() {
    assert_eq!(cp1252_to_char(0x80), '\u{20AC}');
    assert_eq!(cp1252_to_char(0x93), '\u{201C}');
    assert_eq!(cp1252_to_char(0x94), '\u{201D}');
    assert_eq!(cp1252_to_char(0x96), '\u{2013}');
}

#[test]
fn test_normalize_doc_text() {
    assert_eq!(normalize_doc_text("Hello\rWorld"), "Hello\nWorld");
    assert_eq!(normalize_doc_text("A\x07B"), "A\tB");
    assert_eq!(normalize_doc_text("A\x0BB"), "A\nB");
    assert_eq!(normalize_doc_text("A\n\n\n\nB"), "A\n\nB");
}

#[test]
fn test_normalize_doc_text_field_codes() {
    // The instruction between BEGIN and SEPARATOR is markup; only the result survives.
    assert_eq!(normalize_doc_text("A\x13FIELD\x14result\x15B"), "AresultB");
}

#[test]
fn should_drop_hyperlink_instruction_and_keep_result_text() {
    let text = "See \x13 HYPERLINK \"http://example.com/spec\" \\o \"Spec\" \x14the specification\x15 for details.";
    assert_eq!(
        normalize_doc_text(text),
        "See the specification for details.",
        "HYPERLINK instruction must not appear in extracted text"
    );
}

#[test]
fn should_strip_nested_pageref_fields_inside_a_toc_field() {
    // A TOC field whose result contains PAGEREF fields, exactly as Word writes it.
    let text = concat!(
        "\x13 TOC \\o \"1-3\" \\h \\z \\u \x14",
        "\x13 PAGEREF _Toc101 \\h \x141\x15\tIntroduction\n",
        "\x13 PAGEREF _Toc102 \\h \x142\x15\tMethods\n",
        "\x15",
        "Body text."
    );
    assert_eq!(
        normalize_doc_text(text),
        "1\tIntroduction\n2\tMethods\nBody text.",
        "nested PAGEREF/TOC instructions must be stripped without corrupting the result"
    );
}

#[test]
fn should_keep_text_after_an_unterminated_field_begin() {
    // BEGIN with no END at all: treated as inert so the document tail is never lost.
    let text = "Intro.\n\x13PAGEREF _Toc1 \\h \x14";
    assert_eq!(
        normalize_doc_text(text),
        "Intro.\nPAGEREF _Toc1 \\h",
        "an unterminated field must degrade, not swallow the rest of the document"
    );
}

#[test]
fn should_ignore_a_stray_field_end_without_a_begin() {
    assert_eq!(normalize_doc_text("Before\x15After"), "BeforeAfter");
    assert_eq!(
        normalize_doc_text("\x15\x13 SEQ Figure \\* ARABIC \x147\x15\x15Tail"),
        "7Tail",
        "unbalanced END markers must not underflow the field stack"
    );
}

#[test]
fn should_emit_nothing_for_a_terminated_field_without_a_separator() {
    // BEGIN..END with no SEPARATOR: the field has no result, so there is
    // nothing for a reader to see and nothing to emit.
    assert_eq!(
        normalize_doc_text("A\x13 SEQ Figure \\* MERGEFORMAT \x15B"),
        "AB",
        "a resultless field must contribute no text"
    );
}

#[test]
fn should_keep_non_breaking_hyphen_as_a_visible_character() {
    // 0x1E is a hyphen the reader SEES; dropping it welds the compound together.
    assert_eq!(
        normalize_doc_text("Section twenty\x1Eone of the sub\x1Esection"),
        "Section twenty\u{2011}one of the sub\u{2011}section",
        "the non-breaking hyphen is visible text and must not be discarded"
    );
}

#[test]
fn should_keep_non_breaking_hyphen_but_drop_optional_hyphen() {
    // The two are one byte apart and must stay on opposite sides of the line:
    // 0x1E is always rendered, 0x1F only when the line breaks there.
    assert_eq!(
        normalize_doc_text("self\x1Econtained extra\x1Fordinary"),
        "self\u{2011}contained extraordinary",
        "0x1E must survive as U+2011 while 0x1F stays discarded"
    );
}

#[test]
fn should_keep_non_breaking_hyphen_inside_a_field_result() {
    // Field-code stripping runs before character mapping; a cross-reference
    // result such as a clause number must keep its hyphen.
    assert_eq!(
        normalize_doc_text("See \x13 REF _Ref1 \\h \x14clause 3\x1E4\x15."),
        "See clause 3\u{2011}4.",
        "hyphen mapping must apply to text kept from a field result"
    );
}

#[test]
fn test_extract_doc_real_file() {
    let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_documents/vendored/unstructured/doc/simple.doc");
    if !test_file.exists() {
        return;
    }
    let content = std::fs::read(&test_file).expect("Failed to read test DOC");
    let result = extract_doc_text(&content).expect("Failed to extract DOC text");
    assert!(!result.content.is_empty(), "DOC extraction should produce text");
}

#[test]
fn test_extract_doc_fake_file() {
    let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_documents/vendored/unstructured/doc/fake.doc");
    if !test_file.exists() {
        return;
    }
    let content = std::fs::read(&test_file).expect("Failed to read test DOC");
    let result = extract_doc_text(&content).expect("Failed to extract DOC text");
    assert!(!result.content.is_empty(), "DOC extraction should produce text");
}

#[test]
fn test_extract_doc_invalid_magic() {
    let result = extract_doc_text(b"not a doc file");
    assert!(result.is_err());
}

// --- Synthetic `.doc` byte-fixture helpers for issue #77 / #92 ---
//
// No vendored fixture under `test_documents/` has non-empty
// ccpFtn/ccpAtn or a deliberately-overrunning piece, so these build a
// minimal OLE compound file directly, matching the exact FIB layout this
// module reads (fib_base_size=32, csw=14, cslw=22; see
// `extract_text_word97`). ~keep

const TEST_CSW: usize = 14;
const TEST_CSLW: usize = 22;
const TEST_FIB_BASE: usize = 32;

fn write_u16(buf: &mut [u8], offset: usize, val: u16) {
    buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
}

fn write_u32(buf: &mut [u8], offset: usize, val: u32) {
    buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
}

/// `rg_lw_offset` for the layout built by `build_fib`.
fn test_rg_lw_offset() -> usize {
    let csw_offset = TEST_FIB_BASE;
    let rg_w_offset = csw_offset + 2;
    let cslw_offset = rg_w_offset + TEST_CSW * 2;
    cslw_offset + 2
}

/// `rg_fc_lcb_offset` for the layout built by `build_fib`.
fn test_rg_fc_lcb_offset() -> usize {
    let cbrgfclcb_offset = test_rg_lw_offset() + TEST_CSLW * 4;
    cbrgfclcb_offset + 2
}

/// `fc_clx_offset` for the layout built by `build_fib` (`lcb_clx_offset`
/// is always `fc_clx_offset + 4`).
fn test_fc_clx_offset() -> usize {
    test_rg_fc_lcb_offset() + FIB_FC_LCB_IDX_CLX * 8
}

/// Build a `len`-byte WordDocument-stream FIB header with the given
/// `ccp*` fields set. `len` must be large enough to hold the header
/// (at least `test_fc_clx_offset() + 8`) plus any text placed after it.
fn build_fib(len: usize, ccp_text: u32, ccp_ftn: u32, ccp_atn: u32, ccp_txbx: u32) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    write_u16(&mut buf, 0, 0xA5EC); // wIdent
    write_u16(&mut buf, 2, 0x00C1); // Word 97 nFib ~keep
    write_u16(&mut buf, 0x0A, 0x0200); // fWhichTblStm: use 1Table
    write_u16(&mut buf, TEST_FIB_BASE, TEST_CSW as u16);
    let cslw_offset = TEST_FIB_BASE + 2 + TEST_CSW * 2;
    write_u16(&mut buf, cslw_offset, TEST_CSLW as u16);
    let rg_lw_offset = test_rg_lw_offset();
    write_u32(&mut buf, rg_lw_offset + FIB_LW_IDX_CCP_TEXT * 4, ccp_text);
    write_u32(&mut buf, rg_lw_offset + FIB_LW_IDX_CCP_FTN * 4, ccp_ftn);
    write_u32(&mut buf, rg_lw_offset + FIB_LW_IDX_CCP_ATN * 4, ccp_atn);
    write_u32(&mut buf, rg_lw_offset + FIB_LW_IDX_CCP_TXBX * 4, ccp_txbx);
    buf
}

struct TestPiece {
    cp_start: u32,
    cp_end: u32,
    fc_raw: u32,
}

/// Build a `PlcPcd` (piece table) from a run of contiguous pieces.
fn build_plc_pcd(pieces: &[TestPiece]) -> Vec<u8> {
    let mut buf = Vec::new();
    for p in pieces {
        buf.extend_from_slice(&p.cp_start.to_le_bytes());
    }
    buf.extend_from_slice(&pieces.last().expect("at least one piece").cp_end.to_le_bytes());
    for p in pieces {
        buf.extend_from_slice(&[0u8, 0u8]);
        buf.extend_from_slice(&p.fc_raw.to_le_bytes());
        buf.extend_from_slice(&[0u8, 0u8]);
    }
    buf
}

/// Wire a piece table's `fcClx`/`lcbClx` into `word_doc` and return the
/// matching `1Table`-stream bytes.
fn build_table_stream(word_doc: &mut [u8], plc_pcd: &[u8]) -> Vec<u8> {
    const FC_CLX: u32 = 8;
    let mut clx = vec![0x02u8]; // Pcdt marker
    clx.extend_from_slice(&0u32.to_le_bytes()); // lcb (unused by the reader)
    clx.extend_from_slice(plc_pcd);

    let fc_clx_offset = test_fc_clx_offset();
    write_u32(word_doc, fc_clx_offset, FC_CLX);
    write_u32(word_doc, fc_clx_offset + 4, clx.len() as u32);

    let mut table_stream = vec![0u8; FC_CLX as usize];
    table_stream.extend_from_slice(&clx);
    table_stream
}

/// A compressed (CP1252, 1 byte/char) FC pointing at `byte_offset` in the
/// WordDocument stream.
fn compressed_fc(byte_offset: u32) -> u32 {
    0x4000_0000 | (byte_offset * 2)
}

/// Assemble a minimal `.doc` OLE container from prebuilt streams.
fn build_doc_ole(word_doc: &[u8], table_stream: &[u8]) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut comp = cfb::CompoundFile::create(cursor).expect("create CFB container");
    {
        let mut stream = comp.create_stream("/WordDocument").expect("create WordDocument stream");
        std::io::Write::write_all(&mut stream, word_doc).expect("write WordDocument stream");
    }
    {
        let mut stream = comp.create_stream("/1Table").expect("create 1Table stream");
        std::io::Write::write_all(&mut stream, table_stream).expect("write 1Table stream");
    }
    comp.into_inner().into_inner()
}

fn build_legacy_doc_ole(w_ident: u16, n_fib: u16, flags: u16, text: &str) -> Vec<u8> {
    const TEXT_OFFSET: usize = 0x80;

    let mut word_doc = vec![0u8; TEXT_OFFSET + text.len()];
    write_u16(&mut word_doc, 0, w_ident);
    write_u16(&mut word_doc, 2, n_fib);
    write_u16(&mut word_doc, 0x0A, flags);
    write_u32(&mut word_doc, 0x18, TEXT_OFFSET as u32);
    write_u32(&mut word_doc, 0x34, text.len() as u32);
    write_u32(&mut word_doc, 0x4C, 0xFFFF_FFFF);
    word_doc[TEXT_OFFSET..].copy_from_slice(text.as_bytes());

    let cursor = Cursor::new(Vec::new());
    let mut compound = cfb::CompoundFile::create(cursor).expect("create Word 6 CFB container");
    let mut stream = compound
        .create_stream("/WordDocument")
        .expect("create WordDocument stream");
    std::io::Write::write_all(&mut stream, &word_doc).expect("write WordDocument stream");
    drop(stream);
    compound.into_inner().into_inner()
}

#[test]
fn should_extract_word_6_document_without_a_table_stream() {
    const TEXT: &str = "Legacy Word six document";
    let bytes = build_legacy_doc_ole(0xA5DC, 0x0065, 0, TEXT);

    let result = extract_doc_text(&bytes).expect("Word 6 extraction should succeed");

    assert_eq!(result.content, TEXT);
    assert!(result.paragraphs.is_empty());
}

#[test]
fn should_route_a5ec_word_7_header_to_legacy_extraction() {
    const TEXT: &str = "Legacy Word seven document";
    let bytes = build_legacy_doc_ole(0xA5EC, 0x0068, 0, TEXT);

    let result = extract_doc_text(&bytes).expect("Word 7 extraction should succeed");

    assert_eq!(result.content, TEXT);
    assert!(result.paragraphs.is_empty());
}

#[test]
fn should_report_fast_saved_legacy_doc_as_unsupported() {
    let bytes = build_legacy_doc_ole(0xA5DC, 0x0065, 0x0004, "Fast-saved legacy document");

    let error = extract_doc_text(&bytes).expect_err("fast-saved legacy DOC is not implemented");

    assert!(
        error
            .to_string()
            .contains("Fast-saved Word 6/95 documents are not supported"),
        "error should identify the unsupported legacy variant: {error}"
    );
}

/// #77: footnotes, headers, comments and text boxes live in subdocument
/// CP ranges addressed by the FIB's `ccpFtn`/`ccpHdd`/`ccpAtn`/`ccpTxbx`
/// fields. Previously any piece whose CP range started at or after
/// `ccpText` was silently skipped, so this content never appeared.
#[test]
fn test_extract_doc_includes_footnote_and_comment_subdocuments() {
    let main_text = b"Hello";
    // A note ends in a paragraph mark, and its subdocument in one more that
    // belongs to no note, which is where the note tables must end ([MS-DOC]).
    let footnote_text = b"Note one\r\r";
    let comment_text = b"See me\r\r";

    let ccp_text = main_text.len() as u32;
    let ccp_ftn = footnote_text.len() as u32;
    let ccp_atn = comment_text.len() as u32;

    let word_doc_len = 2048;
    let mut word_doc = build_fib(word_doc_len, ccp_text, ccp_ftn, ccp_atn, 0);

    let main_offset = 900usize;
    let footnote_offset = 950usize;
    let comment_offset = 1000usize;
    word_doc[main_offset..main_offset + main_text.len()].copy_from_slice(main_text);
    word_doc[footnote_offset..footnote_offset + footnote_text.len()].copy_from_slice(footnote_text);
    word_doc[comment_offset..comment_offset + comment_text.len()].copy_from_slice(comment_text);

    let pieces = vec![
        TestPiece {
            cp_start: 0,
            cp_end: ccp_text,
            fc_raw: compressed_fc(main_offset as u32),
        },
        TestPiece {
            cp_start: ccp_text,
            cp_end: ccp_text + ccp_ftn,
            fc_raw: compressed_fc(footnote_offset as u32),
        },
        TestPiece {
            cp_start: ccp_text + ccp_ftn,
            cp_end: ccp_text + ccp_ftn + ccp_atn,
            fc_raw: compressed_fc(comment_offset as u32),
        },
    ];
    let plc_pcd = build_plc_pcd(&pieces);
    let mut table_stream = build_table_stream(&mut word_doc, &plc_pcd);
    // [MS-DOC] requires the note tables whenever there are notes.
    for (pair, note) in [
        (MS_DOC_SPEC_PLCFFND_TXT_PAIR, "Note one\r"),
        (MS_DOC_SPEC_PLCFAND_TXT_PAIR, "See me\r"),
    ] {
        write_story_plc(&mut word_doc, &mut table_stream, pair, &[note], PlcKind::Valid);
    }
    let doc_bytes = build_doc_ole(&word_doc, &table_stream);

    let result = extract_doc_text(&doc_bytes).expect("DOC extraction should succeed");

    assert_eq!(result.content, "Hello\n\nFootnotes\n\nNote one\n\nComments\n\nSee me");
    assert!(
        result.processing_warnings.is_empty(),
        "a complete, well-formed document should not warn: {:?}",
        result.processing_warnings
    );
}

/// #92: a piece table entry that declares a byte range past the end of
/// the WordDocument stream must be reported, not silently clamped or
/// dropped.
#[test]
fn test_extract_doc_warns_when_piece_range_overruns_stream() {
    let ccp_text = 10u32;
    let word_doc_len = 700usize;
    let mut word_doc = build_fib(word_doc_len, ccp_text, 0, 0, 0);

    // Only 3 bytes are actually available at this offset; the piece
    // claims 10 compressed (1 byte/char) characters.
    let byte_offset = (word_doc_len - 3) as u32;
    word_doc[word_doc_len - 3..word_doc_len].copy_from_slice(b"Hi!");

    let pieces = vec![TestPiece {
        cp_start: 0,
        cp_end: ccp_text,
        fc_raw: compressed_fc(byte_offset),
    }];
    let plc_pcd = build_plc_pcd(&pieces);
    let table_stream = build_table_stream(&mut word_doc, &plc_pcd);
    let doc_bytes = build_doc_ole(&word_doc, &table_stream);

    let result = extract_doc_text(&doc_bytes).expect("DOC extraction should succeed despite the overrun");

    assert_eq!(
        result.content, "Hi!",
        "should keep the bytes that ARE available, dropping only the overrun tail"
    );
    assert_eq!(result.processing_warnings.len(), 1);
    assert_eq!(result.processing_warnings[0].source, "doc");
    assert!(
        result.processing_warnings[0]
            .message
            .contains("past the end of the WordDocument stream"),
        "warning should name the overrun: {:?}",
        result.processing_warnings[0].message
    );
}

/// #1551: `fcClx` was read from `FibRgFcLcb97` pair 66 (`fcBkdFtnOldOld`,
/// an obsolete field Word writes as zero) instead of pair 33. `fc_clx == 0`
/// therefore held for every real document, the piece table was never walked,
/// and extraction silently fell back to reading `reserved5`/`reserved6` at
/// `0x18`/`0x1C` -- bytes [MS-DOC] says a reader must ignore.
///
/// The pair index is written here as a literal rather than through
/// [`FIB_FC_LCB_IDX_CLX`], deliberately. Both the reader and `build_fib`'s
/// helper use that constant, so a test that positioned the `Clx` through the
/// helper would move with a regression and stay green -- which is exactly why
/// the original defect survived a suite that already covered the piece table.
/// Pinning 33 independently is what makes this guard able to fail. ~keep
#[test]
fn fc_clx_is_read_at_ms_doc_pair_33_not_the_obsolete_pair_66() {
    const MS_DOC_SPEC_FC_CLX_PAIR: usize = 33;
    const OBSOLETE_PAIR_THE_READER_USED_TO_USE: usize = 66;
    const TEXT: &str = "lorem ipsum dolor sit amet";
    /// Placed where the contiguous fallback looks, so the two paths cannot be
    /// confused for one another: whichever string comes back names the path
    /// that ran. ~keep
    const FALLBACK_DECOY: &str = "FALLBACK DECOY TEXT NOT THE DOCUMENT BODY";
    const TEXT_OFFSET: usize = 2048;
    const DECOY_OFFSET: usize = 1536;

    let mut word_doc = build_fib(TEXT_OFFSET + TEXT.len(), TEXT.len() as u32, 0, 0, 0);
    word_doc[TEXT_OFFSET..TEXT_OFFSET + TEXT.len()].copy_from_slice(TEXT.as_bytes());
    word_doc[DECOY_OFFSET..DECOY_OFFSET + FALLBACK_DECOY.len()].copy_from_slice(FALLBACK_DECOY.as_bytes());

    // reserved5/reserved6 -- what the fallback reads as fcMin/fcMac.
    write_u32(&mut word_doc, 0x18, DECOY_OFFSET as u32);
    write_u32(&mut word_doc, 0x1C, (DECOY_OFFSET + FALLBACK_DECOY.len()) as u32);

    let plc_pcd = build_plc_pcd(&[TestPiece {
        cp_start: 0,
        cp_end: TEXT.len() as u32,
        fc_raw: compressed_fc(TEXT_OFFSET as u32),
    }]);

    const FC_CLX: u32 = 8;
    let mut clx = vec![0x02u8];
    clx.extend_from_slice(&0u32.to_le_bytes());
    clx.extend_from_slice(&plc_pcd);

    let rg_fc_lcb_offset = test_rg_lw_offset() + TEST_CSLW * 4 + 2;
    let spec_pair = rg_fc_lcb_offset + MS_DOC_SPEC_FC_CLX_PAIR * 8;
    write_u32(&mut word_doc, spec_pair, FC_CLX);
    write_u32(&mut word_doc, spec_pair + 4, clx.len() as u32);

    let obsolete_pair = rg_fc_lcb_offset + OBSOLETE_PAIR_THE_READER_USED_TO_USE * 8;
    assert_eq!(
        u32::from_le_bytes(word_doc[obsolete_pair..obsolete_pair + 4].try_into().expect("4 bytes")),
        0,
        "pair 66 must stay zero -- it is what every real document holds, and the defect \
         was invisible precisely because reading it yields 0"
    );

    let mut table_stream = vec![0u8; FC_CLX as usize];
    table_stream.extend_from_slice(&clx);

    let doc_bytes = build_doc_ole(&word_doc, &table_stream);
    let result = extract_doc_text(&doc_bytes).expect("DOC extraction should succeed");

    assert_eq!(
        result.content, TEXT,
        "text must come from the piece table at pair 33; got {:?}",
        result.content
    );
    assert!(
        !result.content.contains("FALLBACK DECOY"),
        "the contiguous fallback ran, so fcClx read as 0: {:?}",
        result.content
    );
}

/// Build a `.doc` whose body is `text` in one uncompressed (UTF-16LE) piece.
fn build_utf16_doc(text: &str) -> Vec<u8> {
    const TEXT_OFFSET: usize = 900;
    let units: Vec<u16> = text.encode_utf16().collect();

    let mut word_doc = build_fib(TEXT_OFFSET + units.len() * 2, units.len() as u32, 0, 0, 0);
    for (i, unit) in units.iter().enumerate() {
        write_u16(&mut word_doc, TEXT_OFFSET + i * 2, *unit);
    }
    let plc_pcd = build_plc_pcd(&[TestPiece {
        cp_start: 0,
        cp_end: units.len() as u32,
        fc_raw: TEXT_OFFSET as u32,
    }]);
    let table_stream = build_table_stream(&mut word_doc, &plc_pcd);
    build_doc_ole(&word_doc, &table_stream)
}

/// #2073: an uncompressed piece in which over a quarter of the characters were
/// CJK ideographs was re-decoded as cp1252, so Chinese came out as mojibake.
#[test]
fn should_keep_chinese_text_in_an_uncompressed_piece() {
    const TEXT: &str = "居中的大标题";

    let result = extract_doc_text(&build_utf16_doc(TEXT)).expect("DOC extraction should succeed");

    assert_eq!(result.content, TEXT);
}

/// #2073: the cp1252 re-decode read one byte per CP, half of a UTF-16 piece, so
/// mostly Latin text with one Chinese sentence (28% ideographs) was cut off.
#[test]
fn should_keep_a_mixed_latin_and_chinese_piece_whole() {
    const TEXT: &str = "Det här är svensk text. 这是一个中文句子用于测试混合语言。 Sista meningen.";

    let result = extract_doc_text(&build_utf16_doc(TEXT)).expect("DOC extraction should succeed");

    assert_eq!(result.content, TEXT);
}

// --- Synthetic `.doc` with header/footer, footnote and comment stories (#2054) ---
//
// For what a real file cannot show: absent or malformed story tables, and
// separator stories that carry text.
//
// `ccpHdd` text is split into stories by `PlcfHdd` ([MS-DOC] `Plcfhdd`):
// six separator stories, then six per section (even header, odd header, even
// footer, odd footer, first-page header, first-page footer). The aCP array holds
// one CP per story plus two: the second-to-last ends the last story and equals
// `ccpHdd - 1`, the last is undefined and ignored. The header document ends in
// one extra paragraph mark that belongs to no story. `PlcffndTxt` (footnotes)
// and `PlcfandTxt` (comments) follow the same rule against `ccpFtn`/`ccpAtn`:
// one CP per note plus two. A footnote's text starts with the reference
// character U+0002, a comment's with U+0005. ~keep

/// `FibRgFcLcb97` pair indices of `fcPlcffndTxt`, `fcPlcfandTxt` and
/// `fcPlcfHdd`. Written as literals here, not through reader constants, so a
/// wrong index in the reader cannot move the fixture with it (see #1551
/// above). ~keep
const MS_DOC_SPEC_PLCFFND_TXT_PAIR: usize = 3;
const MS_DOC_SPEC_PLCFAND_TXT_PAIR: usize = 5;
const MS_DOC_SPEC_PLCF_HDD_PAIR: usize = 11;

/// The six separator stories every header document starts with. They carry
/// recognisable text here (Word writes control characters) so a reader that
/// emits them is detectable.
pub(crate) const SEPARATOR_STORIES: [&str; 6] = [
    "Footnote separator\r",
    "Footnote continuation separator\r",
    "Footnote continuation notice\r",
    "Endnote separator\r",
    "Endnote continuation separator\r",
    "Endnote continuation notice\r",
];

/// One section's six stories, in [MS-DOC] order.
const ONE_SECTION_STORIES: [&str; 6] = [
    "Even page header\r",
    "Odd page header\r",
    "Even page footer\r",
    "Odd page footer\r",
    "First page header\r",
    "First page footer\r",
];

/// Separator stories followed by one section's six stories.
pub(crate) fn header_doc_stories() -> Vec<&'static str> {
    [SEPARATOR_STORIES, ONE_SECTION_STORIES].concat()
}

/// How [`build_synthetic_doc`] writes a story-splitting PLC.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum PlcKind {
    Valid,
    /// `fc`/`lcb` are zero although the subdocument is not empty.
    Absent,
    /// `fc` points past the end of the table stream.
    OutOfRange,
    /// Lists the separator stories and one story of a section, so the count
    /// is not six per section. Readable, but not a `PlcfHdd` shape.
    PartialSection,
    /// Leaves out the last story, so the table ends before `ccp - 1`.
    MissingLastStory,
}

/// Text for each CP range of a synthetic Word 97 document. Every string is
/// written verbatim, so callers include reference characters and paragraph
/// marks. An empty slice means the subdocument does not exist.
pub(crate) struct SyntheticDoc<'a> {
    pub body: &'a str,
    /// One string per footnote.
    pub footnotes: &'a [&'a str],
    pub header_stories: &'a [&'a str],
    /// One string per comment.
    pub comments: &'a [&'a str],
    pub plcf_hdd: PlcKind,
    /// How both `PlcffndTxt` and `PlcfandTxt` are written.
    pub note_plcs: PlcKind,
}

/// CP count of a subdocument made of `stories`: their text plus the one final
/// paragraph mark that belongs to no story.
fn subdocument_cp_count(stories: &[&str]) -> usize {
    if stories.is_empty() {
        0
    } else {
        stories.concat().len() + 1
    }
}

/// Append a story-splitting PLC to the table stream and point the FIB pair at it.
fn write_story_plc(
    word_doc: &mut [u8],
    table_stream: &mut Vec<u8>,
    pair_index: usize,
    stories: &[&str],
    kind: PlcKind,
) {
    let (mut fc, mut lcb) = (0u32, 0u32);
    let stories = match kind {
        PlcKind::PartialSection => &stories[..stories.len().min(SEPARATOR_STORIES.len() + 1)],
        PlcKind::MissingLastStory => &stories[..stories.len().saturating_sub(1)],
        _ => stories,
    };
    if !stories.is_empty() && kind != PlcKind::Absent {
        let mut cps = Vec::new();
        let mut cp = 0u32;
        for story in stories {
            cps.push(cp);
            cp += story.len() as u32;
        }
        cps.push(cp); // ccp - 1
        cps.push(cp + 1); // undefined, ignored
        fc = table_stream.len() as u32;
        lcb = (cps.len() * 4) as u32;
        for cp in &cps {
            table_stream.extend_from_slice(&cp.to_le_bytes());
        }
        if kind == PlcKind::OutOfRange {
            fc += 0x1_0000;
        }
    }
    let pair = test_rg_fc_lcb_offset() + pair_index * 8;
    write_u32(word_doc, pair, fc);
    write_u32(word_doc, pair + 4, lcb);
}

/// Build a `.doc` whose CP space is body, footnotes, header document and
/// comments, all in one piece.
pub(crate) fn build_synthetic_doc(spec: &SyntheticDoc) -> Vec<u8> {
    const TEXT_OFFSET: usize = 900;
    const CB_RG_FC_LCB_97: u16 = 93;

    let mut text = String::from(spec.body);
    for stories in [spec.footnotes, spec.header_stories, spec.comments] {
        text.push_str(&stories.concat());
        if !stories.is_empty() {
            text.push('\r');
        }
    }
    assert!(text.is_ascii(), "the piece is written as one byte per character");

    let mut word_doc = build_fib(
        TEXT_OFFSET + text.len() + 16,
        spec.body.len() as u32,
        subdocument_cp_count(spec.footnotes) as u32,
        subdocument_cp_count(spec.comments) as u32,
        0,
    );
    write_u32(
        &mut word_doc,
        test_rg_lw_offset() + FIB_LW_IDX_CCP_HDD * 4,
        subdocument_cp_count(spec.header_stories) as u32,
    );
    write_u16(&mut word_doc, test_rg_lw_offset() + TEST_CSLW * 4, CB_RG_FC_LCB_97);
    word_doc[TEXT_OFFSET..TEXT_OFFSET + text.len()].copy_from_slice(text.as_bytes());

    let plc_pcd = build_plc_pcd(&[TestPiece {
        cp_start: 0,
        cp_end: text.len() as u32,
        fc_raw: compressed_fc(TEXT_OFFSET as u32),
    }]);
    let mut table_stream = build_table_stream(&mut word_doc, &plc_pcd);
    for (pair, stories, kind) in [
        (MS_DOC_SPEC_PLCFFND_TXT_PAIR, spec.footnotes, spec.note_plcs),
        (MS_DOC_SPEC_PLCFAND_TXT_PAIR, spec.comments, spec.note_plcs),
        (MS_DOC_SPEC_PLCF_HDD_PAIR, spec.header_stories, spec.plcf_hdd),
    ] {
        write_story_plc(&mut word_doc, &mut table_stream, pair, stories, kind);
    }

    build_doc_ole(&word_doc, &table_stream)
}

fn paragraph_texts(result: &DocExtractionResult) -> Vec<&str> {
    result.paragraphs.iter().map(|p| p.content.as_str()).collect()
}

/// Word ends a hyperlink laid over a whole paragraph after that paragraph's
/// mark, so the paragraph holding the field's begin does not hold its end.
#[test]
fn should_drop_a_hyperlink_instruction_whose_field_ends_in_the_next_paragraph() {
    let text = "Rules\r\x13 HYPERLINK \"http://example.com/rules\" \x01\x14The rules (2023)\r\x15Next paragraph.\r";

    let result = extract_doc_text(&build_utf16_doc(text)).expect("DOC extraction should succeed");

    assert_eq!(
        paragraph_texts(&result),
        ["Rules", "The rules (2023)", "Next paragraph."]
    );
    assert!(!result.content.contains("HYPERLINK"), "{}", result.content);
}

/// A `TOC` field's result is one paragraph per entry, so its instruction opens
/// in the first entry's paragraph and the field ends after the last one.
#[test]
fn should_drop_a_toc_instruction_from_the_paragraphs_it_spans() {
    let text = concat!(
        "\x13 TOC \\o \"1-3\" \\h \\z \\u \x14",
        "\x13 PAGEREF _Toc101 \\h \x141\x15\tIntroduction\r",
        "\x13 PAGEREF _Toc102 \\h \x142\x15\tMethods\r",
        "\x15Body text.\r"
    );

    let result = extract_doc_text(&build_utf16_doc(text)).expect("DOC extraction should succeed");

    assert_eq!(
        paragraph_texts(&result),
        ["1\tIntroduction", "2\tMethods", "Body text."]
    );
}

/// An unterminated begin still degrades instead of swallowing the paragraphs
/// after it.
#[test]
fn should_keep_the_paragraphs_after_an_unterminated_field_begin() {
    let text = "Intro.\r\x13PAGEREF _Toc1 \\h \x14Tail\rLast.\r";

    let result = extract_doc_text(&build_utf16_doc(text)).expect("DOC extraction should succeed");

    assert_eq!(paragraph_texts(&result), ["Intro.", "PAGEREF _Toc1 \\h Tail", "Last."]);
}

/// Build a `.doc` whose body is `text` in one uncompressed piece, with one
/// `PapxFkp` page giving each run of characters (`start..end`, char indices,
/// together covering `text`) the paragraph-property grpprl beside it.
pub(crate) fn build_utf16_doc_with_papx(text: &str, runs: &[(std::ops::Range<usize>, &[u8])]) -> Vec<u8> {
    const TEXT_OFFSET: usize = 900;
    let units: Vec<u16> = text.encode_utf16().collect();
    let fc = |char_index: usize| (TEXT_OFFSET + char_index * 2) as u32;
    let page_offset = (TEXT_OFFSET + units.len() * 2).div_ceil(512) * 512;

    let mut word_doc = build_fib(page_offset + 512, units.len() as u32, 0, 0, 0);
    for (i, unit) in units.iter().enumerate() {
        write_u16(&mut word_doc, TEXT_OFFSET + i * 2, *unit);
    }

    // PapxFkp: crun+1 FCs, crun 13-byte BxPap entries, then the PAPXs, each ~keep
    // a cb byte and 2*cb-1 bytes of istd (0) plus grpprl; crun in the last byte. ~keep
    let page = &mut word_doc[page_offset..page_offset + 512];
    let crun = runs.len();
    let mut papx_at = 256usize;
    for (i, (range, grpprl)) in runs.iter().enumerate() {
        page[i * 4..i * 4 + 4].copy_from_slice(&fc(range.start).to_le_bytes());
        page[(i + 1) * 4..(i + 2) * 4].copy_from_slice(&fc(range.end).to_le_bytes());
        page[(crun + 1) * 4 + i * 13] = (papx_at / 2) as u8;
        let cb = (2 + grpprl.len() + 1).div_ceil(2);
        page[papx_at] = cb as u8;
        page[papx_at + 3..papx_at + 3 + grpprl.len()].copy_from_slice(grpprl);
        papx_at += (1 + cb * 2 - 1).div_ceil(2) * 2;
    }
    page[511] = crun as u8;

    let plc_pcd = build_plc_pcd(&[TestPiece {
        cp_start: 0,
        cp_end: units.len() as u32,
        fc_raw: TEXT_OFFSET as u32,
    }]);
    let mut table_stream = build_table_stream(&mut word_doc, &plc_pcd);
    // PlcfBtePapx: the FCs the one page covers, then its page number. ~keep
    let plcf_at = table_stream.len();
    table_stream.extend_from_slice(&fc(0).to_le_bytes());
    table_stream.extend_from_slice(&fc(units.len()).to_le_bytes());
    table_stream.extend_from_slice(&((page_offset / 512) as u32).to_le_bytes());
    let pair = test_rg_fc_lcb_offset() + 13 * 8;
    write_u32(&mut word_doc, pair, plcf_at as u32);
    write_u32(&mut word_doc, pair + 4, 12);

    build_doc_ole(&word_doc, &table_stream)
}

const IN_TABLE: &[u8] = &[0x16, 0x24, 0x01];
const ROW_END: &[u8] = &[0x16, 0x24, 0x01, 0x17, 0x24, 0x01];

fn table_marks(result: &DocExtractionResult) -> Vec<(&str, Option<(bool, bool)>)> {
    result
        .paragraphs
        .iter()
        .map(|p| (p.content.as_str(), p.table.map(|t| (t.cell_end, t.row_end))))
        .collect()
}

/// A table's cell marks end paragraphs, so each cell is a paragraph of its own
/// and each row ends with an empty row-end paragraph. The cells of a row share
/// one FKP run, so membership is looked up by range.
#[test]
fn should_split_table_cells_and_rows_at_their_marks() {
    let text = "Intro\rA1\x07B1\x07\x07A2\x07\x07\x07After\r";
    let runs: &[(std::ops::Range<usize>, &[u8])] = &[
        (0..6, &[]),
        (6..12, IN_TABLE),
        (12..13, ROW_END),
        (13..17, IN_TABLE),
        (17..18, ROW_END),
        (18..24, &[]),
    ];

    let result = extract_doc_text(&build_utf16_doc_with_papx(text, runs)).expect("DOC extraction should succeed");

    let cell = Some((true, false));
    let row = Some((false, true));
    assert_eq!(
        table_marks(&result),
        [
            ("Intro", None),
            ("A1", cell),
            ("B1", cell),
            ("", row),
            ("A2", cell),
            ("", cell),
            ("", row),
            ("After", None),
        ]
    );
}

/// A cell of several paragraphs: all but the last end with paragraph marks.
#[test]
fn should_keep_the_paragraphs_of_a_cell_in_the_table() {
    let text = "x\ry\x07\x07";
    let runs: &[(std::ops::Range<usize>, &[u8])] = &[(0..4, IN_TABLE), (4..5, ROW_END)];

    let result = extract_doc_text(&build_utf16_doc_with_papx(text, runs)).expect("DOC extraction should succeed");

    assert_eq!(
        table_marks(&result),
        [
            ("x", Some((false, false))),
            ("y", Some((true, false))),
            ("", Some((false, true)))
        ]
    );
}

/// A cell mark in a paragraph that is not in a table does not end it.
#[test]
fn should_leave_a_cell_mark_outside_a_table_as_a_tab() {
    let text = "A\x07B\r";
    let runs: &[(std::ops::Range<usize>, &[u8])] = &[(0..4, &[])];

    let result = extract_doc_text(&build_utf16_doc_with_papx(text, runs)).expect("DOC extraction should succeed");

    assert_eq!(table_marks(&result), [("A\tB", None)]);
}
