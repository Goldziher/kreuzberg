//! Paragraph properties (`PAPX`) for legacy `.doc`, limited to list and table
//! membership.
//!
//! [MS-DOC] stores paragraph formatting in `PAPX` grpprls held in 512-byte
//! `PapxFkp` pages, indexed by byte offset (`FC`) through the FIB's
//! `PlcfBtePapx`. This module reads only what #1550 needs -- whether a
//! paragraph is bound to an automatic list (`sprmPIlfo`) and at what depth
//! (`sprmPIlvl`) -- and whether it sits in a table (`sprmPFInTable`) or ends
//! a table row (`sprmPFTtp`). Resolving the *number* Word paints requires
//! `PlfLst` / `PlfLfo` and a counter walk, which is deliberately out of scope.

use ahash::AHashMap;

/// `FibRgFcLcb97` pair holding `fcPlcfBtePapx`/`lcbPlcfBtePapx`.
const FIB_FC_LCB_IDX_PLCF_BTE_PAPX: usize = 13;

/// `FibRgFcLcb97` pair holding `fcStshf` -- the style sheet.
const FIB_FC_LCB_IDX_STSHF: usize = 1;

/// `STD.istdBase` value meaning "this style has no base style".
const ISTD_NO_BASE: u16 = 0x0FFF;

/// Built-in style identifiers 1..=9 are `heading 1`..`heading 9`; [MS-DOC]
/// reserves the low `sti` range for fixed built-in styles, so this mapping is
/// specified rather than conventional. ~keep
const STI_HEADING_MIN: u16 = 1;
const STI_HEADING_MAX: u16 = 9;

/// Bound on how far a style's base chain is followed. Chains are shallow in
/// practice; the bound exists so a malformed self- or cycle-referencing sheet
/// cannot spin. ~keep
const MAX_STYLE_BASE_DEPTH: usize = 8;

/// `FibRgFcLcb97` pair holding `fcPlfLst` -- the list definition table.
const FIB_FC_LCB_IDX_PLF_LST: usize = 73;

/// `FibRgFcLcb97` pair holding `fcPlfLfo` -- the list format overrides that
/// `ilfo` indexes into.
const FIB_FC_LCB_IDX_PLF_LFO: usize = 74;

/// `LSTF` record size in the `PlfLst` array.
const LSTF_LEN: usize = 28;

/// `LFO` record size in the `PlfLfo` array.
const LFO_LEN: usize = 16;

/// `LVLF` header size, before the two grpprls and the `Xst` that follow it.
const LVLF_LEN: usize = 28;

/// A list level with `nfc` 23 paints a bullet glyph rather than a number;
/// 255 paints nothing. Everything else is an ordered numbering scheme
/// ([MS-DOC] 2.9.131 `MSONFC`). ~keep
const NFC_BULLET: u8 = 23;
const NFC_NONE: u8 = 255;

/// Number of `LVL` records a non-simple list carries, one per outline level.
const LVL_COUNT_MULTILEVEL: usize = 9;

/// `sprmPIlfo` -- the 1-based list index binding a paragraph to a list. Zero
/// means the paragraph is not in a list. `spra` = 2, so a 2-byte operand.
const SPRM_P_ILFO: u16 = 0x460B;

/// `sprmPIlvl` -- the zero-based nesting depth within that list. `spra` = 1,
/// so a 1-byte operand.
const SPRM_P_ILVL: u16 = 0x260A;

/// `sprmPFInTable` -- the paragraph is in a table cell. `spra` = 1.
const SPRM_P_F_IN_TABLE: u16 = 0x2416;

/// `sprmPFTtp` -- the paragraph is the table terminating paragraph that ends
/// a row: it holds only the row's end mark. `spra` = 1.
const SPRM_P_F_TTP: u16 = 0x2417;

/// `sprmTDefTable` and `sprmTDefTable10` -- a row's cell layout, the sprms
/// whose length takes two bytes. A row's terminating paragraph carries the
/// row's table sprms, so its PAPX has one.
const SPRM_T_DEF_TABLE: u16 = 0xD608;
const SPRM_T_DEF_TABLE_10: u16 = 0xD606;

/// `sprmPChgTabs` -- tab stops changed; a length of 255 starts its long form.
const SPRM_P_CHG_TABS: u16 = 0xC615;

/// Every `PapxFkp` is exactly one 512-byte page, and its entry count lives in
/// the final byte. ~keep
const FKP_PAGE_LEN: usize = 512;

/// `BxPap` is 13 bytes in the Word 97 FKP layout: a 1-byte word offset
/// followed by 12 reserved bytes. ~keep
const BX_PAP_LEN: usize = 13;

/// A paragraph's binding to an automatic list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ListBinding {
    /// 1-based index into the list-format overrides. Never zero: a zero
    /// `ilfo` means "not in a list" and yields `None` rather than a binding.
    pub ilfo: u16,
    /// Zero-based nesting depth. Absent `sprmPIlvl` means level 0.
    pub ilvl: u8,
}

/// Operand length in bytes for a sprm, from its `spra` field (bits 13-15).
///
/// `spra` 6 is variable-length, where the first operand byte is the count;
/// returning `None` lets the caller read it. Any other value is a malformed
/// sprm and stops the walk rather than guessing a stride.
fn sprm_operand_len(sprm: u16) -> Option<usize> {
    match sprm >> 13 {
        0 | 1 => Some(1),
        2 | 4 | 5 => Some(2),
        3 => Some(4),
        7 => Some(3),
        _ => None,
    }
}

/// Call `visit` with each sprm in a `grpprl` and its operand, stopping at the
/// first one whose length cannot be read.
fn for_each_sprm(grpprl: &[u8], mut visit: impl FnMut(u16, &[u8])) {
    let mut pos = 0usize;

    while pos + 2 <= grpprl.len() {
        let sprm = u16::from_le_bytes([grpprl[pos], grpprl[pos + 1]]);
        pos += 2;

        let len = match sprm {
            // sprmTDefTable and sprmTDefTable10 hold a 2-byte length, plus one.
            SPRM_T_DEF_TABLE | SPRM_T_DEF_TABLE_10 => match grpprl.get(pos..pos + 2) {
                Some(cb) => usize::from(u16::from_le_bytes([cb[0], cb[1]])) + 1,
                None => break,
            },
            // sprmPChgTabs with a length byte of 255 is sized by its counts:
            // tabs deleted (4 bytes each), then tabs added (3 bytes each).
            SPRM_P_CHG_TABS if grpprl.get(pos) == Some(&255) => {
                let Some(&deleted) = grpprl.get(pos + 1) else { break };
                let Some(&added) = grpprl.get(pos + 2 + 4 * usize::from(deleted)) else {
                    break;
                };
                3 + 4 * usize::from(deleted) + 3 * usize::from(added)
            }
            _ => match sprm_operand_len(sprm) {
                Some(len) => len,
                // spra 6: variable, first operand byte is the length.
                None if sprm >> 13 == 6 => match grpprl.get(pos) {
                    Some(&cb) => usize::from(cb) + 1,
                    None => break,
                },
                None => break,
            },
        };

        let operand = match grpprl.get(pos..pos + len) {
            Some(operand) => operand,
            None => break,
        };
        pos += len;

        visit(sprm, operand);
    }
}

/// Scan a `grpprl` for the two sprms that describe list membership.
fn list_binding_from_grpprl(grpprl: &[u8]) -> Option<ListBinding> {
    let mut ilfo: Option<u16> = None;
    let mut ilvl: u8 = 0;

    for_each_sprm(grpprl, |sprm, operand| match sprm {
        SPRM_P_ILFO if operand.len() >= 2 => {
            ilfo = Some(u16::from_le_bytes([operand[0], operand[1]]));
        }
        SPRM_P_ILVL if !operand.is_empty() => ilvl = operand[0],
        _ => {}
    });

    match ilfo {
        Some(ilfo) if ilfo != 0 => Some(ListBinding { ilfo, ilvl }),
        _ => None,
    }
}

/// A paragraph's place in a table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TableParagraph {
    /// The paragraph is the row's terminating paragraph (`sprmPFTtp`).
    pub row_end: bool,
}

/// Scan a `grpprl` for table membership; `None` when the paragraph is not in a
/// table.
fn table_paragraph_from_grpprl(grpprl: &[u8]) -> Option<TableParagraph> {
    let mut in_table = false;
    let mut row_end = false;

    for_each_sprm(grpprl, |sprm, operand| match sprm {
        SPRM_P_F_IN_TABLE => in_table = operand.first().is_some_and(|&v| v != 0),
        SPRM_P_F_TTP => row_end = operand.first().is_some_and(|&v| v != 0),
        _ => {}
    });

    // A row's terminating paragraph is in the table whether or not it also
    // says so.
    (in_table || row_end).then_some(TableParagraph { row_end })
}

/// Extract one `BxPap` entry's `GrpPrlAndIstd`: the paragraph's style index
/// and the grpprl that overrides it.
///
/// `bOffset` is a *word* offset into the page; zero means the paragraph has no
/// `PAPX` at all. Measured across every `.doc` available, no paragraph is in
/// that state -- but it is returned as `None` rather than defaulted, so a
/// document that does have one is not silently attributed style 0. ~keep
fn style_and_grpprl_at(page: &[u8], b_offset: u8) -> Option<(u16, &[u8])> {
    if b_offset == 0 {
        return None;
    }
    let at = usize::from(b_offset) * 2;
    let cb = *page.get(at)?;
    let (start, len) = if cb != 0 {
        (at + 1, usize::from(cb) * 2 - 1)
    } else {
        let cb2 = *page.get(at + 1)?;
        (at + 2, usize::from(cb2) * 2)
    };
    let run = page.get(start..start + len)?;
    let istd = u16::from_le_bytes([*run.first()?, *run.get(1)?]);
    Some((istd, run.get(2..)?))
}

/// Whether a list level paints numbers or bullets, resolved through
/// `ilfo` -> `LFO` -> `lsid` -> `LSTF` -> `LVL[ilvl]` -> `nfc`.
///
/// `ElementKind::ListItem` requires an `ordered` flag, so this is not optional
/// detail: emitting every list-bound paragraph as ordered would mislabel every
/// bulleted list. Resolving `nfc` is a lookup, distinct from the counter walk
/// that would be needed to paint the number itself -- which stays out of
/// scope. ~keep
#[derive(Default)]
pub(super) struct ListFormats {
    /// `(ilfo, ilvl)` -> ordered.
    ordered_by_level: AHashMap<(u16, u8), bool>,
}

impl ListFormats {
    pub(super) fn build(table_stream: &[u8], word_doc: &[u8], rg_fc_lcb_offset: usize) -> Self {
        let mut formats = Self::default();

        let Some((lst_fc, lst_lcb)) = read_fc_lcb(word_doc, rg_fc_lcb_offset, FIB_FC_LCB_IDX_PLF_LST) else {
            return formats;
        };
        let Some((lfo_fc, lfo_lcb)) = read_fc_lcb(word_doc, rg_fc_lcb_offset, FIB_FC_LCB_IDX_PLF_LFO) else {
            return formats;
        };
        // `lcbPlfLst` covers only the `cLst` count and the `LSTF` array; the
        // variable-length `LVL` blocks follow immediately *after* it in the
        // table stream. Slicing to `lcb` cuts every `LVL` off, which yields an
        // empty format map that `is_ordered`'s default then silently papers
        // over -- a bulleted list would report as ordered. ~keep
        let (Some(plf_lst), Some(plf_lfo)) = (table_stream.get(lst_fc..), table_stream.get(lfo_fc..lfo_fc + lfo_lcb))
        else {
            return formats;
        };

        let nfc_by_lsid = parse_plf_lst(plf_lst, lst_lcb);

        // PlfLfo: cLfo as u32, then that many 16-byte LFO records whose first
        // field is the lsid of the list they override. `ilfo` is 1-based.
        if plf_lfo.len() < 4 {
            return formats;
        }
        let c_lfo = u32::from_le_bytes([plf_lfo[0], plf_lfo[1], plf_lfo[2], plf_lfo[3]]) as usize;
        for i in 0..c_lfo {
            let at = 4 + i * LFO_LEN;
            let Some(record) = plf_lfo.get(at..at + LFO_LEN) else {
                break;
            };
            let lsid = u32::from_le_bytes([record[0], record[1], record[2], record[3]]);
            let Some(levels) = nfc_by_lsid.get(&lsid) else {
                continue;
            };
            let ilfo = u16::try_from(i + 1).unwrap_or(u16::MAX);
            for (ilvl, nfc) in levels.iter().enumerate() {
                let ilvl = u8::try_from(ilvl).unwrap_or(u8::MAX);
                formats
                    .ordered_by_level
                    .insert((ilfo, ilvl), *nfc != NFC_BULLET && *nfc != NFC_NONE);
            }
        }

        formats
    }

    /// Whether the level paints an ordered number. Unknown levels default to
    /// ordered, matching the automatic numbering this issue is about; a
    /// bulleted list whose tables could not be read is the rarer case.
    pub(super) fn is_ordered(&self, ilfo: u16, ilvl: u8) -> bool {
        self.ordered_by_level.get(&(ilfo, ilvl)).copied().unwrap_or(true)
    }
}

/// Read one `FibRgFcLcb97` pair, rejecting an absent or empty structure.
pub(super) fn read_fc_lcb(word_doc: &[u8], rg_fc_lcb_offset: usize, index: usize) -> Option<(usize, usize)> {
    let at = rg_fc_lcb_offset + index * 8;
    let bytes = word_doc.get(at..at + 8)?;
    let fc = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    let lcb = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    (lcb > 0).then_some((fc, lcb))
}

/// Map each list's `lsid` to its per-level `nfc` values.
///
/// `PlfLst` is `cLst` as a 16-bit count, then that many fixed-size `LSTF`
/// records, then the variable-length `LVL` blocks for each list in the same
/// order -- one for a simple list, nine otherwise. The `LVL`s must be walked
/// in sequence because each one's length depends on its own header.
fn parse_plf_lst(plf_lst: &[u8], lcb: usize) -> AHashMap<u32, Vec<u8>> {
    let mut nfc_by_lsid = AHashMap::new();
    if plf_lst.len() < 2 {
        return nfc_by_lsid;
    }
    let c_lst = usize::from(u16::from_le_bytes([plf_lst[0], plf_lst[1]]));
    // The declared lcb must actually hold the LSTF array it claims to.
    if 2 + c_lst * LSTF_LEN > lcb {
        return nfc_by_lsid;
    }

    let mut lists: Vec<(u32, usize)> = Vec::with_capacity(c_lst);
    for i in 0..c_lst {
        let at = 2 + i * LSTF_LEN;
        let Some(lstf) = plf_lst.get(at..at + LSTF_LEN) else {
            return nfc_by_lsid;
        };
        let lsid = u32::from_le_bytes([lstf[0], lstf[1], lstf[2], lstf[3]]);
        let f_simple_list = lstf[26] & 0x01 != 0;
        lists.push((lsid, if f_simple_list { 1 } else { LVL_COUNT_MULTILEVEL }));
    }

    let mut pos = 2 + c_lst * LSTF_LEN;
    for (lsid, level_count) in lists {
        let mut nfcs = Vec::with_capacity(level_count);
        for _ in 0..level_count {
            let Some(lvlf) = plf_lst.get(pos..pos + LVLF_LEN) else {
                return nfc_by_lsid;
            };
            nfcs.push(lvlf[4]);
            let cb_grpprl_chpx = usize::from(lvlf[24]);
            let cb_grpprl_papx = usize::from(lvlf[25]);
            pos += LVLF_LEN + cb_grpprl_papx + cb_grpprl_chpx;
            // Xst: a 16-bit character count followed by that many UTF-16 units.
            let Some(cch) = plf_lst.get(pos..pos + 2) else {
                return nfc_by_lsid;
            };
            pos += 2 + usize::from(u16::from_le_bytes([cch[0], cch[1]])) * 2;
        }
        nfc_by_lsid.insert(lsid, nfcs);
    }

    nfc_by_lsid
}

/// The document's style sheet, reduced to the one question this module asks:
/// does a given `istd` denote a heading, and at what level.
///
/// Resolution follows `istdBase`, so a custom style derived from a built-in
/// heading resolves too. That case is real and not hypothetical -- a corpus
/// document carries `istd` 97 = `TOC Heading`, whose `sti` is 46 but whose
/// base is `heading 1`. Checking `sti` 1..=9 alone would miss it. ~keep
#[derive(Default)]
pub(super) struct StyleSheet {
    /// Indexed by `istd`; `None` for an absent or unparseable entry.
    styles: Vec<Option<StyleEntry>>,
}

#[derive(Clone, Copy)]
struct StyleEntry {
    sti: u16,
    istd_base: u16,
}

impl StyleSheet {
    fn build(word_doc: &[u8], table_stream: &[u8], rg_fc_lcb_offset: usize) -> Self {
        let mut sheet = Self::default();
        let Some((fc, lcb)) = read_fc_lcb(word_doc, rg_fc_lcb_offset, FIB_FC_LCB_IDX_STSHF) else {
            return sheet;
        };
        let Some(stsh) = table_stream.get(fc..fc + lcb) else {
            return sheet;
        };
        // STSH: cbStshi, then STSHI (whose first field is cstd), then one
        // LPStd per style -- a 16-bit length followed by that many bytes.
        let Some(cb_stshi) = stsh.get(..2) else {
            return sheet;
        };
        let cb_stshi = usize::from(u16::from_le_bytes([cb_stshi[0], cb_stshi[1]]));
        let Some(cstd) = stsh.get(2..4) else {
            return sheet;
        };
        let cstd = usize::from(u16::from_le_bytes([cstd[0], cstd[1]]));

        let mut pos = 2 + cb_stshi;
        for _ in 0..cstd {
            let Some(raw) = stsh.get(pos..pos + 2) else {
                break;
            };
            let cb_std = usize::from(u16::from_le_bytes([raw[0], raw[1]]));
            pos += 2;
            if cb_std == 0 {
                // An empty slot: the istd exists but names no style.
                sheet.styles.push(None);
                continue;
            }
            let Some(std) = stsh.get(pos..pos + cb_std) else {
                break;
            };
            pos += cb_std;
            // STDFBase's first two 16-bit words: sti in the low 12 bits of the
            // first, istdBase in the high 12 bits of the second.
            sheet.styles.push(match (std.get(..2), std.get(2..4)) {
                (Some(first), Some(second)) => Some(StyleEntry {
                    sti: u16::from_le_bytes([first[0], first[1]]) & 0x0FFF,
                    istd_base: (u16::from_le_bytes([second[0], second[1]]) >> 4) & 0x0FFF,
                }),
                _ => None,
            });
        }

        sheet
    }

    /// Heading level for a paragraph style, or `None` if it is not a heading.
    pub(super) fn heading_level(&self, istd: u16) -> Option<u8> {
        let mut current = istd;
        for _ in 0..MAX_STYLE_BASE_DEPTH {
            let entry = (*self.styles.get(usize::from(current))?)?;
            if (STI_HEADING_MIN..=STI_HEADING_MAX).contains(&entry.sti) {
                return u8::try_from(entry.sti).ok();
            }
            if entry.istd_base == ISTD_NO_BASE || entry.istd_base == current {
                return None;
            }
            current = entry.istd_base;
        }
        None
    }
}

/// List bindings for a document, keyed by the byte offset (`FC`) of each
/// paragraph mark.
#[derive(Default)]
pub(super) struct ParagraphListIndex {
    by_end_fc: AHashMap<u32, ListBinding>,
    /// Paragraph style index (`istd`), for every paragraph that carries a
    /// `PAPX` -- not only list-bound ones.
    style_by_end_fc: AHashMap<u32, u16>,
    /// FKP runs whose paragraphs sit in a table, as `(first FC, FC one past
    /// the run, membership)`, sorted by their end. Looked up by range rather
    /// than by the run's end: one run covers every consecutive paragraph that
    /// shares a `PAPX`, and the cells of a row usually do. ~keep
    table_runs: Vec<(u32, u32, TableParagraph)>,
}

impl ParagraphListIndex {
    /// Build the index from the `PlcfBtePapx` plex and the FKP pages it names.
    ///
    /// Returns an empty index rather than an error whenever the structures are
    /// absent or malformed: list membership is additive, and a document whose
    /// paragraph properties cannot be read must still yield its text.
    pub(super) fn build(word_doc: &[u8], table_stream: &[u8], rg_fc_lcb_offset: usize) -> Self {
        let mut index = Self::default();

        let pair = rg_fc_lcb_offset + FIB_FC_LCB_IDX_PLCF_BTE_PAPX * 8;
        let Some(fc_bytes) = word_doc.get(pair..pair + 8) else {
            return index;
        };
        let fc = u32::from_le_bytes([fc_bytes[0], fc_bytes[1], fc_bytes[2], fc_bytes[3]]) as usize;
        let lcb = u32::from_le_bytes([fc_bytes[4], fc_bytes[5], fc_bytes[6], fc_bytes[7]]) as usize;
        if lcb < 4 {
            return index;
        }
        let Some(plex) = table_stream.get(fc..fc + lcb) else {
            return index;
        };

        // PlcfBtePapx: (n+1) FCs then n 4-byte page numbers, whose low 22 bits
        // are the FKP page index. ~keep
        let n = (lcb - 4) / 8;
        for i in 0..n {
            let pn_at = (n + 1) * 4 + i * 4;
            let Some(raw) = plex.get(pn_at..pn_at + 4) else {
                break;
            };
            let pn = (u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) & 0x003F_FFFF) as usize;
            index.absorb_fkp_page(word_doc, pn);
        }
        index.table_runs.sort_unstable_by_key(|&(_, end, _)| end);

        index
    }

    fn absorb_fkp_page(&mut self, word_doc: &[u8], pn: usize) {
        let Some(page) = word_doc.get(pn * FKP_PAGE_LEN..(pn + 1) * FKP_PAGE_LEN) else {
            return;
        };
        let crun = usize::from(page[FKP_PAGE_LEN - 1]);
        if crun == 0 {
            return;
        }
        // rgfc holds crun+1 FCs, then rgbx holds crun BxPap entries.
        let rgbx_at = (crun + 1) * 4;
        if rgbx_at + crun * BX_PAP_LEN > FKP_PAGE_LEN - 1 {
            return;
        }

        for i in 0..crun {
            let start_fc_at = i * 4;
            let end_fc_at = (i + 1) * 4;
            let (Some(start), Some(raw)) = (page.get(start_fc_at..end_fc_at), page.get(end_fc_at..end_fc_at + 4))
            else {
                break;
            };
            let start_fc = u32::from_le_bytes([start[0], start[1], start[2], start[3]]);
            let end_fc = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);

            let b_offset = page[rgbx_at + i * BX_PAP_LEN];
            let Some((istd, grpprl)) = style_and_grpprl_at(page, b_offset) else {
                continue;
            };
            // The FKP's rgfc entry i+1 is the FC one past the paragraph's last
            // character, i.e. just past its paragraph mark.
            self.style_by_end_fc.insert(end_fc, istd);
            if let Some(binding) = list_binding_from_grpprl(grpprl) {
                self.by_end_fc.insert(end_fc, binding);
            }
            if let Some(table) = table_paragraph_from_grpprl(grpprl)
                && start_fc < end_fc
            {
                self.table_runs.push((start_fc, end_fc, table));
            }
        }
    }

    /// Table membership of the paragraph whose mark ends at `end_fc`: the run
    /// that holds the mark's last byte.
    fn table_for_paragraph_end(&self, end_fc: u32) -> Option<TableParagraph> {
        let at = self.table_runs.partition_point(|&(_, end, _)| end < end_fc);
        let &(start, _, table) = self.table_runs.get(at)?;
        (start < end_fc).then_some(table)
    }

    /// Look up the binding for a paragraph whose mark ends at `end_fc`.
    pub(super) fn binding_for_paragraph_end(&self, end_fc: u32) -> Option<ListBinding> {
        self.by_end_fc.get(&end_fc).copied()
    }

    /// Look up the style index for a paragraph whose mark ends at `end_fc`.
    fn style_for_paragraph_end(&self, end_fc: u32) -> Option<u16> {
        self.style_by_end_fc.get(&end_fc).copied()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.by_end_fc.len()
    }
}

/// The two list structures a paragraph needs: which paragraphs are bound to a
/// list, and whether each level paints numbers or bullets. Bundled so the
/// piece-table walk does not thread two references through every frame.
pub(super) struct ListTables {
    pub index: ParagraphListIndex,
    pub formats: ListFormats,
    pub styles: StyleSheet,
}

impl ListTables {
    pub(super) fn build(word_doc: &[u8], table_stream: &[u8], rg_fc_lcb_offset: usize) -> Self {
        Self {
            index: ParagraphListIndex::build(word_doc, table_stream, rg_fc_lcb_offset),
            formats: ListFormats::build(table_stream, word_doc, rg_fc_lcb_offset),
            styles: StyleSheet::build(word_doc, table_stream, rg_fc_lcb_offset),
        }
    }

    /// Declared heading level for the paragraph whose mark ends at `end_fc`.
    pub(super) fn heading_level_for_paragraph_end(&self, end_fc: u32) -> Option<u8> {
        self.styles.heading_level(self.index.style_for_paragraph_end(end_fc)?)
    }

    /// Table membership for the paragraph whose mark ends at `end_fc`.
    pub(super) fn table_for_paragraph_end(&self, end_fc: u32) -> Option<TableParagraph> {
        self.index.table_for_paragraph_end(end_fc)
    }

    /// Membership for the paragraph whose mark ends at `end_fc`.
    pub(super) fn membership_for_paragraph_end(&self, end_fc: u32) -> Option<(u8, bool)> {
        self.index
            .binding_for_paragraph_end(end_fc)
            .map(|binding| (binding.ilvl, self.formats.is_ordered(binding.ilfo, binding.ilvl)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprm_operand_len_covers_every_documented_spra() {
        assert_eq!(sprm_operand_len(SPRM_P_ILVL), Some(1), "sprmPIlvl is spra 1");
        assert_eq!(sprm_operand_len(SPRM_P_ILFO), Some(2), "sprmPIlfo is spra 2");
        assert_eq!(sprm_operand_len(0x6000), Some(4), "spra 3 is a 4-byte operand");
        assert_eq!(sprm_operand_len(0xE000), Some(3), "spra 7 is a 3-byte operand");
        assert_eq!(sprm_operand_len(0xC000), None, "spra 6 is variable-length");
    }

    #[test]
    fn table_membership_is_read_from_in_table_and_ttp() {
        assert_eq!(table_paragraph_from_grpprl(&[0x03, 0x24, 0x01]), None, "sprmPJc alone");
        assert_eq!(
            table_paragraph_from_grpprl(&[0x16, 0x24, 0x01]),
            Some(TableParagraph { row_end: false })
        );
        assert_eq!(
            table_paragraph_from_grpprl(&[0x16, 0x24, 0x01, 0x17, 0x24, 0x01]),
            Some(TableParagraph { row_end: true })
        );
        assert_eq!(table_paragraph_from_grpprl(&[0x16, 0x24, 0x00]), None, "fInTable off");
    }

    #[test]
    fn table_membership_is_read_after_a_table_definition() {
        // sprmTDefTable first, as long as a row of 12 cells makes it: its
        // 2-byte length (the operand's size minus one) must be read as such
        // for the flags after it to be found.
        let mut grpprl = vec![0x08, 0xD6, 0x01, 0x01];
        grpprl.extend_from_slice(&[0; 256]);
        grpprl.extend_from_slice(&[0x16, 0x24, 0x01, 0x17, 0x24, 0x01]);
        assert_eq!(
            table_paragraph_from_grpprl(&grpprl),
            Some(TableParagraph { row_end: true })
        );

        // sprmPChgTabs in its long form: one tab deleted, two added.
        let mut grpprl = vec![0x15, 0xC6, 255, 1, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0];
        grpprl.extend_from_slice(&[0x16, 0x24, 0x01]);
        assert_eq!(
            table_paragraph_from_grpprl(&grpprl),
            Some(TableParagraph { row_end: false })
        );
    }

    #[test]
    fn table_runs_are_looked_up_by_the_range_that_holds_the_mark() {
        let index = ParagraphListIndex {
            table_runs: vec![
                (100, 120, TableParagraph { row_end: false }),
                (120, 122, TableParagraph { row_end: true }),
            ],
            ..Default::default()
        };
        assert_eq!(
            index.table_for_paragraph_end(106),
            Some(TableParagraph { row_end: false }),
            "inside a run"
        );
        assert_eq!(
            index.table_for_paragraph_end(120),
            Some(TableParagraph { row_end: false }),
            "a run's last byte"
        );
        assert_eq!(
            index.table_for_paragraph_end(122),
            Some(TableParagraph { row_end: true })
        );
        assert_eq!(index.table_for_paragraph_end(100), None, "before the first run");
        assert_eq!(index.table_for_paragraph_end(130), None, "after the last run");
    }

    #[test]
    fn grpprl_without_ilfo_is_not_a_list_paragraph() {
        // sprmPJc (0x2403), spra 1: a paragraph property that is not a list binding.
        let grpprl = [0x03, 0x24, 0x01];
        assert_eq!(list_binding_from_grpprl(&grpprl), None);
    }

    #[test]
    fn ilfo_zero_means_not_in_a_list_rather_than_list_zero() {
        let mut grpprl = vec![0x0B, 0x46];
        grpprl.extend_from_slice(&0u16.to_le_bytes());
        assert_eq!(
            list_binding_from_grpprl(&grpprl),
            None,
            "ilfo == 0 is Word's encoding for 'no list', not a binding to list 0"
        );
    }

    #[test]
    fn ilfo_and_ilvl_are_read_together() {
        let mut grpprl = vec![0x0A, 0x26, 0x02]; // sprmPIlvl = 2
        grpprl.extend_from_slice(&[0x0B, 0x46]); // sprmPIlfo
        grpprl.extend_from_slice(&9u16.to_le_bytes());
        assert_eq!(
            list_binding_from_grpprl(&grpprl),
            Some(ListBinding { ilfo: 9, ilvl: 2 })
        );
    }

    #[test]
    fn ilvl_defaults_to_zero_when_only_ilfo_is_present() {
        let mut grpprl = vec![0x0B, 0x46];
        grpprl.extend_from_slice(&3u16.to_le_bytes());
        assert_eq!(
            list_binding_from_grpprl(&grpprl),
            Some(ListBinding { ilfo: 3, ilvl: 0 })
        );
    }

    #[test]
    fn an_unknown_sprm_is_skipped_by_its_spra_stride_not_by_guessing() {
        // A 4-byte-operand sprm (spra 3) ahead of the binding: reading its
        // length wrong would desynchronize the walk and lose the ilfo behind it.
        let mut grpprl = vec![0x00, 0x60, 0xDE, 0xAD, 0xBE, 0xEF];
        grpprl.extend_from_slice(&[0x0B, 0x46]);
        grpprl.extend_from_slice(&7u16.to_le_bytes());
        assert_eq!(
            list_binding_from_grpprl(&grpprl),
            Some(ListBinding { ilfo: 7, ilvl: 0 }),
            "a preceding 4-byte sprm must be stepped over exactly"
        );
    }

    #[test]
    fn a_truncated_operand_stops_the_walk_instead_of_reading_past_the_end() {
        let grpprl = [0x0B, 0x46, 0x01]; // sprmPIlfo declaring 2 bytes, only 1 present
        assert_eq!(list_binding_from_grpprl(&grpprl), None);
    }

    #[test]
    fn a_zero_boffset_paragraph_has_no_papx() {
        let page = vec![0u8; FKP_PAGE_LEN];
        assert!(
            style_and_grpprl_at(&page, 0).is_none(),
            "bOffset 0 means the paragraph has no PAPX at all"
        );
    }

    fn sheet(entries: &[(u16, u16)]) -> StyleSheet {
        StyleSheet {
            styles: entries
                .iter()
                .map(|&(sti, istd_base)| Some(StyleEntry { sti, istd_base }))
                .collect(),
        }
    }

    #[test]
    fn a_built_in_heading_style_resolves_to_its_level() {
        let styles = sheet(&[(0, ISTD_NO_BASE), (1, ISTD_NO_BASE), (3, ISTD_NO_BASE)]);
        assert_eq!(styles.heading_level(1), Some(1));
        assert_eq!(styles.heading_level(2), Some(3), "istd 2 carries sti 3 here");
        assert_eq!(styles.heading_level(0), None, "sti 0 is Normal");
    }

    /// A custom style derived from a heading is still a heading. `TOC Heading`
    /// in a corpus document is exactly this shape -- `sti` 46, base `heading
    /// 1` -- so checking `sti` 1..=9 alone would miss it. No corpus document
    /// *applies* such a style, which is why this case is synthesized rather
    /// than asserted against a fixture. ~keep
    #[test]
    fn a_custom_style_based_on_a_heading_resolves_through_its_base() {
        let styles = sheet(&[(0, ISTD_NO_BASE), (1, ISTD_NO_BASE), (46, 1)]);
        assert_eq!(styles.heading_level(2), Some(1), "sti 46 based on heading 1");
    }

    #[test]
    fn a_base_chain_that_cycles_terminates_instead_of_spinning() {
        // Two styles naming each other as base, and one naming itself.
        let styles = sheet(&[(40, 1), (41, 0), (42, 2)]);
        assert_eq!(styles.heading_level(0), None);
        assert_eq!(styles.heading_level(2), None, "self-referencing base must terminate");
    }

    #[test]
    fn an_istd_past_the_end_of_the_sheet_is_not_a_heading() {
        let styles = sheet(&[(1, ISTD_NO_BASE)]);
        assert_eq!(
            styles.heading_level(99),
            None,
            "out-of-range istd must not panic or match"
        );
    }

    #[test]
    fn build_returns_an_empty_index_rather_than_failing_on_a_missing_plex() {
        let index = ParagraphListIndex::build(&[], &[], 0);
        assert_eq!(index.len(), 0, "text extraction must survive unreadable properties");
    }

    /// Reads a real document rather than a synthesized FKP. Both matter: the
    /// synthetic cases above pin the sprm decoding, but #1551 is the standing
    /// proof that a suite built only from this module's own assumptions will
    /// agree with them even when they are wrong about the format.
    ///
    /// `unit_test_lists.doc` has been in the corpus exercising nothing --
    /// until `fcClx` was fixed the extractor could not see a list at all.
    /// The expected counts were derived by an independent parser written
    /// against [MS-DOC], not by recording what this code produces. ~keep
    #[test]
    fn real_corpus_document_yields_its_known_list_paragraph_count() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/doc/unit_test_lists.doc");
        assert!(
            path.exists(),
            "corpus fixture missing at {}; fetch test_documents rather than skipping -- \
             a skipped test and a passing test are indistinguishable in a summary",
            path.display()
        );

        let content = std::fs::read(&path).expect("read corpus fixture");
        let mut comp = cfb::CompoundFile::open(std::io::Cursor::new(content.as_slice())).expect("open OLE");
        let word_doc = read_test_stream(&mut comp, "/WordDocument");
        let flags_a = u16::from_le_bytes([word_doc[0x0A], word_doc[0x0B]]);
        let table_name = if (flags_a & 0x0200) != 0 { "/1Table" } else { "/0Table" };
        let table_stream = read_test_stream(&mut comp, table_name);

        let csw = usize::from(u16::from_le_bytes([word_doc[32], word_doc[33]]));
        let cslw_offset = 34 + csw * 2;
        let cslw = usize::from(u16::from_le_bytes([word_doc[cslw_offset], word_doc[cslw_offset + 1]]));
        let rg_fc_lcb_offset = cslw_offset + 2 + cslw * 4 + 2;

        let index = ParagraphListIndex::build(&word_doc, &table_stream, rg_fc_lcb_offset);

        assert_eq!(
            index.len(),
            25,
            "expected 25 list-bound paragraphs in unit_test_lists.doc (ilfo 1 at ilvl 0/1/2 \
             and ilfo 2 at ilvl 0); got {}",
            index.len()
        );
    }

    fn read_test_stream(comp: &mut cfb::CompoundFile<std::io::Cursor<&[u8]>>, name: &str) -> Vec<u8> {
        use std::io::Read;
        let mut stream = comp.open_stream(name).expect("open stream");
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).expect("read stream");
        buf
    }
}
