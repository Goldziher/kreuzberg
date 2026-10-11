//! The text-extraction pass: [`extract_text_from_rtf`] and RTF control-word
//! dispatch (`handle_control_word` and its per-word helpers).

use super::control_word::{ControlWordCtx, handle_control_word};
use super::{
    FldrsltCloseState, FormattingTracker, ParagraphMeta, RtfFormattingData, close_fldinst_group, close_fldrslt_group,
    consume_adjacent_hex_escape, parse_font_charset_table, parse_rtf_color_table, resolve_decode_codepage,
};
use crate::extractors::rtf::encoding::{decode_ansi_bytes, parse_hex_byte, parse_rtf_control_word};
use crate::extractors::rtf::formatting::{map_offset, normalize_whitespace_with_mapping};
use crate::extractors::rtf::images::RtfImage;
use crate::extractors::rtf::tables::TableState;
use crate::types::Table;
use std::collections::HashMap;
use std::iter::Peekable;
use std::str::Chars;

/// Known RTF destination groups whose content should be skipped entirely.
///
/// These are groups that start with a control word and contain metadata,
/// font tables, style sheets, or binary data — not document body text.
///
/// Note: `field` and `fldinst` are NOT in this list — they are handled
/// specially so that hyperlink text (`\fldrslt`) is extracted.
const SKIP_DESTINATIONS: &[&str] = &[
    "fonttbl",
    "colortbl",
    "stylesheet",
    "info",
    "listtable",
    "listoverridetable",
    "generator",
    "filetbl",
    "revtbl",
    "rsidtbl",
    "xmlnstbl",
    "mmathPr",
    "themedata",
    "colorschememapping",
    "datastore",
    "latentstyles",
    "datafield",
    "objdata",
    "objclass",
    "panose",
    "bkmkstart",
    "bkmkend",
    "wgrffmtfilter",
    "fcharset",
    "pgdsctbl",
];

/// Close a `\listtext`/`\pntext` group if this `}` ends it, marking the
/// current list item as ordered when its buffered text looks like a numbered
/// or lettered marker (e.g. `1.` or `a)`). ~keep
fn close_listtext_group(
    group_depth: i32,
    in_listtext: &mut bool,
    listtext_depth: i32,
    listtext_buf: &mut String,
    cur_ordered: &mut bool,
) {
    if !*in_listtext || group_depth >= listtext_depth {
        return;
    }
    *in_listtext = false;
    let lt = listtext_buf.trim();
    let is_ordered = lt
        .strip_suffix('.')
        .or_else(|| lt.strip_suffix(')'))
        .is_some_and(|prefix| {
            let p = prefix.trim();
            if p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty() {
                return true;
            }
            if p.chars().all(|c| c.is_ascii_alphabetic()) && !p.is_empty() {
                return true;
            }
            false
        });
    if is_ordered {
        *cur_ordered = true;
    }
    listtext_buf.clear();
}

/// Close a `\footnote` destination if this `}` ends it, recording the
/// buffered note text.
fn close_footnote_group(
    group_depth: i32,
    in_footnote: &mut bool,
    footnote_depth: i32,
    footnote_buf: &mut String,
    footnotes: &mut Vec<String>,
) {
    if !*in_footnote || group_depth >= footnote_depth {
        return;
    }
    *in_footnote = false;
    let note = footnote_buf.trim().to_string();
    if !note.is_empty() {
        footnotes.push(note);
    }
    footnote_buf.clear();
}

/// Close a `\shptxt` (drawing-object/text-box text) destination if this `}`
/// ends it, recording the buffered text-box text.
fn close_shptxt_group(
    group_depth: i32,
    in_shptxt: &mut bool,
    shptxt_depth: i32,
    shptxt_buf: &mut String,
    text_boxes: &mut Vec<String>,
) {
    if !*in_shptxt || group_depth >= shptxt_depth {
        return;
    }
    *in_shptxt = false;
    let text_box = shptxt_buf.trim().to_string();
    if !text_box.is_empty() {
        text_boxes.push(text_box);
    }
    shptxt_buf.clear();
}

/// Close an `\annotation` (Word comment) destination if this `}` ends it,
/// labeling the comment with its `\atnid` (if any) or a running counter.
fn close_annotation_group(
    group_depth: i32,
    in_annotation: &mut bool,
    annotation_depth: i32,
    annotation_buf: &mut String,
    comments: &mut Vec<String>,
    pending_atnid: &mut Option<i32>,
) {
    if !*in_annotation || group_depth >= annotation_depth {
        return;
    }
    *in_annotation = false;
    let comment = annotation_buf.trim().to_string();
    if !comment.is_empty() {
        let label = pending_atnid
            .take()
            .map(|id| id.to_string())
            .unwrap_or_else(|| (comments.len() + 1).to_string());
        comments.push(format!("[Comment {label}]: {comment}"));
    } else {
        *pending_atnid = None;
    }
    annotation_buf.clear();
}

struct TextEscapeCtx<'a> {
    result: &'a mut String,
    table_state: &'a mut Option<TableState>,
    tables: &'a mut Vec<Table>,
    images: &'a mut Vec<RtfImage>,
    ensure_table: &'a dyn Fn(&mut Option<TableState>),
    finalize_table: &'a dyn Fn(&mut Option<TableState>, &mut Vec<Table>),
    plain: bool,
    group_has_text: &'a mut Vec<bool>,
    cur_heading_level: &'a mut u8,
    cur_list_level: &'a mut Option<u8>,
    cur_list_id: &'a mut Option<u16>,
    cur_ordered: &'a mut bool,
    para_metas: &'a mut Vec<ParagraphMeta>,
    para_meta_emitted: &'a mut bool,
    uc_stack: &'a mut Vec<u8>,
    ansi_codepage_stack: &'a mut Vec<u32>,
    footnote_count: &'a mut usize,
    pending_boundary_space: &'a mut bool,
    hidden_stack: &'a mut Vec<bool>,
    fmt_tracker: &'a mut FormattingTracker,
    font_id_stack: &'a mut Vec<Option<u16>>,
    default_font_id: &'a mut Option<u16>,
    font_charsets: &'a HashMap<u16, u32>,
    group_depth: i32,
    skip_depth: &'a mut i32,
    expect_destination: &'a mut bool,
    ignorable_pending: &'a mut bool,
    in_fldinst: &'a mut bool,
    fldinst_depth: &'a mut i32,
    fldinst_content: &'a mut String,
    in_fldrslt: &'a mut bool,
    fldrslt_depth: &'a mut i32,
    fldrslt_start: &'a mut usize,
    in_listtext: &'a mut bool,
    listtext_depth: &'a mut i32,
    listtext_buf: &'a mut String,
    in_footnote: &'a mut bool,
    footnote_depth: &'a mut i32,
    footnote_buf: &'a mut String,
    in_shptxt: &'a mut bool,
    shptxt_depth: &'a mut i32,
    shptxt_buf: &'a mut String,
    in_annotation: &'a mut bool,
    annotation_depth: &'a mut i32,
    annotation_buf: &'a mut String,
    pending_atnid: &'a mut Option<i32>,
    picture_groups: &'a mut Vec<PictureGroupState>,
    pending_primary: &'a mut Option<PendingPrimary>,
}

#[derive(Clone, Copy, Default)]
enum PictureRole {
    #[default]
    Direct,
    Primary,
    Fallback {
        suppress: bool,
    },
}

#[derive(Clone, Copy, Default)]
struct PictureGroupState {
    role: PictureRole,
    wrapper: Option<PictureWrapper>,
    image_start: usize,
}

#[derive(Clone, Copy)]
enum PictureWrapper {
    Primary,
    Fallback,
}

#[derive(Clone, Copy)]
struct PendingPrimary {
    parent_depth: i32,
    has_image: bool,
}

impl TextEscapeCtx<'_> {
    fn control_word_ctx(&mut self) -> ControlWordCtx<'_> {
        ControlWordCtx {
            result: self.result,
            table_state: self.table_state,
            tables: self.tables,
            images: self.images,
            suppress_picture: matches!(
                self.picture_groups.last().map(|group| group.role),
                Some(PictureRole::Fallback { suppress: true })
            ),
            ensure_table: self.ensure_table,
            finalize_table: self.finalize_table,
            plain: self.plain,
            group_has_text: self.group_has_text,
            cur_heading_level: self.cur_heading_level,
            cur_list_level: self.cur_list_level,
            cur_list_id: self.cur_list_id,
            cur_ordered: self.cur_ordered,
            para_metas: self.para_metas,
            para_meta_emitted: self.para_meta_emitted,
            uc_stack: self.uc_stack,
            ansi_codepage_stack: self.ansi_codepage_stack,
            footnote_count: self.footnote_count,
            pending_boundary_space: self.pending_boundary_space,
            hidden_stack: self.hidden_stack,
            fmt_tracker: self.fmt_tracker,
            font_id_stack: self.font_id_stack,
            default_font_id: self.default_font_id,
        }
    }

    fn handle(&mut self, chars: &mut Peekable<Chars<'_>>) {
        let Some(&next_ch) = chars.peek() else {
            return;
        };
        match next_ch {
            '\n' | '\r' => self.handle_line_break(next_ch, chars),
            '\\' | '{' | '}' => self.handle_literal(next_ch, chars),
            '\'' => self.handle_hex_escape(chars),
            '*' => {
                chars.next();
                *self.ignorable_pending = true;
            }
            _ => self.handle_control_word(chars),
        }
    }

    fn handle_line_break(&mut self, next_ch: char, chars: &mut Peekable<Chars<'_>>) {
        chars.next();
        if next_ch == '\r' && matches!(chars.peek(), Some(&'\n')) {
            chars.next();
        }
        *self.expect_destination = false;
        if *self.skip_depth > 0 {
            return;
        }
        let mut ctx = self.control_word_ctx();
        handle_control_word("par", None, chars, &mut ctx);
    }

    fn handle_literal(&mut self, next_ch: char, chars: &mut Peekable<Chars<'_>>) {
        chars.next();
        *self.pending_primary = None;
        *self.expect_destination = false;
        if *self.in_fldinst {
            self.fldinst_content.push(next_ch);
        }
        if *self.in_footnote {
            self.footnote_buf.push(next_ch);
        }
        if *self.in_shptxt {
            self.shptxt_buf.push(next_ch);
        }
        if *self.in_annotation {
            self.annotation_buf.push(next_ch);
        }
        if *self.skip_depth > 0 || self.hidden_stack.last().copied().unwrap_or(false) {
            return;
        }
        if *self.pending_boundary_space
            && !self.result.is_empty()
            && !self.result.ends_with(' ')
            && !self.result.ends_with('\n')
        {
            self.result.push(' ');
        }
        *self.pending_boundary_space = false;
        *self.para_meta_emitted = false;
        self.result.push(next_ch);
        if let Some(flag) = self.group_has_text.last_mut() {
            *flag = true;
        }
    }

    fn handle_hex_escape(&mut self, chars: &mut Peekable<Chars<'_>>) {
        chars.next();
        *self.pending_primary = None;
        *self.expect_destination = false;
        let bytes = parse_hex_escape_bytes(chars);
        if (*self.in_footnote || *self.in_shptxt || *self.in_annotation) && bytes.is_some() {
            self.append_hex_to_destinations(bytes.as_deref().unwrap_or_default());
        }
        if *self.skip_depth > 0 || self.hidden_stack.last().copied().unwrap_or(false) {
            return;
        }
        let Some(bytes) = bytes.as_deref() else {
            return;
        };
        let decoded = self.decode_ansi(bytes);
        if let Some(state) = self.table_state.as_mut()
            && state.in_row
        {
            state.current_cell.push_str(&decoded);
            return;
        }
        if *self.pending_boundary_space
            && !self.result.is_empty()
            && !self.result.ends_with(' ')
            && !self.result.ends_with('\n')
        {
            self.result.push(' ');
        }
        *self.pending_boundary_space = false;
        *self.para_meta_emitted = false;
        self.result.push_str(&decoded);
        if let Some(flag) = self.group_has_text.last_mut() {
            *flag = true;
        }
    }

    fn decode_ansi(&self, bytes: &[u8]) -> String {
        let codepage = resolve_decode_codepage(
            self.font_id_stack,
            *self.default_font_id,
            self.font_charsets,
            self.ansi_codepage_stack,
        );
        decode_ansi_bytes(bytes, codepage)
    }

    fn append_hex_to_destinations(&mut self, bytes: &[u8]) {
        let decoded = self.decode_ansi(bytes);
        if *self.in_footnote {
            self.footnote_buf.push_str(&decoded);
        }
        if *self.in_shptxt {
            self.shptxt_buf.push_str(&decoded);
        }
        if *self.in_annotation {
            self.annotation_buf.push_str(&decoded);
        }
    }

    fn handle_control_word(&mut self, chars: &mut Peekable<Chars<'_>>) {
        let (control_word, param) = parse_rtf_control_word(chars);
        if *self.expect_destination || *self.ignorable_pending {
            *self.expect_destination = false;
            if self.handle_destination(&control_word, param) {
                return;
            }
        }
        if *self.skip_depth > 0 {
            self.handle_skipped_control_word(&control_word, param, chars);
            return;
        }
        *self.pending_primary = None;
        let mut ctx = self.control_word_ctx();
        handle_control_word(&control_word, param, chars, &mut ctx);
    }

    fn handle_destination(&mut self, control_word: &str, param: Option<i32>) -> bool {
        if control_word == "shppict" {
            *self.pending_primary = None;
            if let Some(group) = self.picture_groups.last_mut() {
                group.role = PictureRole::Primary;
                group.wrapper = Some(PictureWrapper::Primary);
                group.image_start = self.images.len();
            }
            *self.ignorable_pending = false;
            return true;
        }
        if control_word == "nonshppict" {
            let parent_depth = self.group_depth - 1;
            let suppress = self
                .pending_primary
                .take()
                .is_some_and(|primary| primary.parent_depth == parent_depth && primary.has_image);
            if let Some(group) = self.picture_groups.last_mut() {
                group.role = PictureRole::Fallback { suppress };
                group.wrapper = Some(PictureWrapper::Fallback);
                group.image_start = self.images.len();
            }
            *self.ignorable_pending = false;
            return true;
        }
        *self.pending_primary = None;
        if *self.ignorable_pending {
            *self.ignorable_pending = false;
            if self.handle_ignorable_destination(control_word, param) {
                return true;
            }
        }
        match control_word {
            "listtext" | "pntext" => {
                *self.in_listtext = true;
                *self.listtext_depth = self.group_depth;
                self.listtext_buf.clear();
                self.start_skip();
            }
            "fldinst" => {
                *self.in_fldinst = true;
                *self.fldinst_depth = self.group_depth;
                self.start_skip();
            }
            "fldrslt" => {
                *self.in_fldrslt = true;
                *self.fldrslt_depth = self.group_depth;
                *self.fldrslt_start = self.result.len();
            }
            "footnote" => {
                *self.in_footnote = true;
                *self.footnote_depth = self.group_depth;
                self.footnote_buf.clear();
                self.start_skip();
            }
            "shptxt" => {
                *self.in_shptxt = true;
                *self.shptxt_depth = self.group_depth;
                self.shptxt_buf.clear();
                self.start_skip();
            }
            "annotation" => {
                *self.in_annotation = true;
                *self.annotation_depth = self.group_depth;
                self.annotation_buf.clear();
                self.start_skip();
            }
            "atnid" => {
                if let Some(id) = param {
                    *self.pending_atnid = Some(id);
                }
                self.start_skip();
            }
            word if SKIP_DESTINATIONS.contains(&word) => self.start_skip(),
            _ => return false,
        }
        true
    }

    fn handle_ignorable_destination(&mut self, control_word: &str, param: Option<i32>) -> bool {
        match control_word {
            "fldinst" => {
                *self.in_fldinst = true;
                *self.fldinst_depth = self.group_depth;
            }
            "listtext" | "pntext" => {
                *self.in_listtext = true;
                *self.listtext_depth = self.group_depth;
                self.listtext_buf.clear();
            }
            "atnid" => {
                if let Some(id) = param {
                    *self.pending_atnid = Some(id);
                }
            }
            "shppict" => return false,
            _ => {}
        }
        self.start_skip();
        true
    }

    fn start_skip(&mut self) {
        if *self.skip_depth == 0 {
            *self.skip_depth = self.group_depth;
        }
    }

    fn handle_skipped_control_word(&mut self, control_word: &str, param: Option<i32>, chars: &mut Peekable<Chars<'_>>) {
        match control_word {
            "uc" => {
                if let Some(value) = param
                    && let Some(uc) = self.uc_stack.last_mut()
                {
                    *uc = value.max(0) as u8;
                }
            }
            "ansicpg" => {
                if let Some(value) = param
                    && value > 0
                    && let Some(codepage) = self.ansi_codepage_stack.last_mut()
                {
                    *codepage = value as u32;
                }
            }
            "f" => {
                if let Some(value) = param
                    && let Some(font_id) = self.font_id_stack.last_mut()
                {
                    *font_id = Some(value.max(0) as u16);
                }
            }
            "deff" => {
                if let Some(value) = param {
                    *self.default_font_id = Some(value.max(0) as u16);
                }
            }
            "u" if *self.in_footnote || *self.in_shptxt || *self.in_annotation => {
                self.append_skipped_unicode(param, chars);
            }
            "par" | "line" if *self.in_footnote || *self.in_shptxt || *self.in_annotation => {
                if *self.in_footnote {
                    self.footnote_buf.push(' ');
                }
                if *self.in_shptxt {
                    self.shptxt_buf.push(' ');
                }
                if *self.in_annotation {
                    self.annotation_buf.push(' ');
                }
            }
            _ => {}
        }
    }

    fn append_skipped_unicode(&mut self, param: Option<i32>, chars: &mut Peekable<Chars<'_>>) {
        let Some(code_num) = param else {
            return;
        };
        let code_u = if code_num < 0 {
            (code_num + 65536) as u32
        } else {
            code_num as u32
        };
        if let Some(character) = char::from_u32(code_u) {
            if *self.in_footnote {
                self.footnote_buf.push(character);
            }
            if *self.in_shptxt {
                self.shptxt_buf.push(character);
            }
            if *self.in_annotation {
                self.annotation_buf.push(character);
            }
        }
        let uc_count = self.uc_stack.last().copied().unwrap_or(1);
        for _ in 0..uc_count {
            if chars.peek().is_some_and(|next| !matches!(next, '\\' | '{' | '}')) {
                chars.next();
            }
        }
    }
}

fn parse_hex_escape_bytes(chars: &mut Peekable<Chars<'_>>) -> Option<Vec<u8>> {
    let (Some(hex1), Some(hex2)) = (chars.next(), chars.next()) else {
        return None;
    };
    let byte = parse_hex_byte(hex1 as u8, hex2 as u8)?;
    let mut bytes = vec![byte];
    while let Some(next_byte) = consume_adjacent_hex_escape(chars) {
        bytes.push(next_byte);
    }
    Some(bytes)
}

/// Extract text and image metadata from RTF document.
///
/// This function extracts plain text from an RTF document by:
/// 1. Tracking group nesting depth with a state stack
/// 2. Skipping known destination groups (fonttbl, stylesheet, info, etc.)
/// 3. Skipping `{\*\...}` ignorable destination groups
/// 4. Converting encoded characters to Unicode
/// 5. Extracting text while skipping formatting groups
/// 6. Detecting and extracting image metadata (\pict sections)
/// 7. Normalizing whitespace
pub(crate) fn extract_text_from_rtf(
    content: &str,
    plain: bool,
) -> (String, Vec<Table>, Vec<RtfImage>, Vec<ParagraphMeta>, RtfFormattingData) {
    let color_table = parse_rtf_color_table(content);
    let font_charsets = parse_font_charset_table(content);
    let mut fmt_tracker = FormattingTracker::new();

    let mut result = String::new();
    let mut chars = content.chars().peekable();
    let mut tables: Vec<Table> = Vec::new();
    let mut images: Vec<RtfImage> = Vec::new();
    let mut table_state: Option<TableState> = None;

    let mut para_metas: Vec<ParagraphMeta> = Vec::new();
    let mut cur_heading_level: u8 = 0;
    let mut cur_list_level: Option<u8> = None;
    let mut cur_list_id: Option<u16> = None;
    let mut in_listtext = false;
    let mut listtext_depth: i32 = 0;
    let mut listtext_buf = String::new();
    let mut cur_ordered = false;
    let mut para_meta_emitted = false;

    let mut uc_stack: Vec<u8> = vec![1];

    let mut in_fldinst = false;
    let mut fldinst_depth: i32 = 0;
    let mut fldinst_content = String::new();
    let mut in_fldrslt = false;
    let mut fldrslt_depth: i32 = 0;
    let mut fldrslt_start: usize = 0;
    let mut pending_hyperlink_url: Option<String> = None;
    let mut hyperlinks: Vec<(usize, usize, String)> = Vec::new();

    let mut in_footnote = false;
    let mut footnote_depth: i32 = 0;
    let mut footnote_buf = String::new();
    let mut footnote_count: usize = 0;
    let mut footnotes: Vec<String> = Vec::new();

    // `\shptxt` (drawing-object / text-box text) and `\annotation` (comment
    // text) are ordinary content destinations, but real producers nest them
    // inside an *ignorable* ancestor (`{\*\shp{\*\shpinst{...{\shptxt ...}`
    // for text boxes) that this parser otherwise skips wholesale. Buffering
    // their content unconditionally -- the same trick `footnote_buf` uses --
    // lets them survive even while nested under an active `skip_depth` (#86). ~keep
    let mut in_shptxt = false;
    let mut shptxt_depth: i32 = 0;
    let mut shptxt_buf = String::new();
    let mut text_boxes: Vec<String> = Vec::new();

    let mut in_annotation = false;
    let mut annotation_depth: i32 = 0;
    let mut annotation_buf = String::new();
    let mut comments: Vec<String> = Vec::new();
    // Set by `\atnid` (the comment's numeric id, always written as an
    // ignorable `{\*\atnid N}` sibling of `\annotation`) and consumed when
    // the enclosing `\annotation` group closes.
    let mut pending_atnid: Option<i32> = None;

    let mut group_depth: i32 = 0;
    let mut skip_depth: i32 = 0;

    let mut ignorable_pending = false;
    let mut expect_destination = false;

    let mut group_has_text: Vec<bool> = Vec::new();
    let mut picture_groups = vec![PictureGroupState::default()];
    let mut pending_primary = None;

    let mut pending_boundary_space = false;

    let mut hidden_stack: Vec<bool> = vec![false];

    // ANSI codepage for \'hh escapes. RTF defaults to Windows-1252 unless
    // overridden by \ansicpgNNNN. Scoped like other document properties.
    let mut ansi_codepage_stack: Vec<u32> = vec![1252];

    // Active font id for \'hh escapes, set by \fN / \deffN. Used to look up a
    // per-font codepage in `font_charsets` (from \fcharsetN), which takes
    // priority over `ansi_codepage_stack`. Scoped like other document properties.
    let mut font_id_stack: Vec<Option<u16>> = vec![None];
    // Document default font set by \deffN. Unlike font_id_stack, this is not
    // scoped: \deff is typically declared once, before any nested group could
    // have inherited it, so it's tracked separately and consulted only when no
    // scope has set an explicit \fN. See `resolve_decode_codepage`.
    let mut default_font_id: Option<u16> = None;

    let ensure_table = |table_state: &mut Option<TableState>| {
        if table_state.is_none() {
            *table_state = Some(TableState::new());
        }
    };

    let finalize_table = move |state_opt: &mut Option<TableState>, tables: &mut Vec<Table>| {
        if let Some(state) = state_opt.take()
            && let Some(table) = state.finalize_with_format(plain)
        {
            tables.push(table);
        }
    };

    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                group_depth += 1;
                expect_destination = true;
                group_has_text.push(false);
                let role = picture_groups.last().map(|group| group.role).unwrap_or_default();
                picture_groups.push(PictureGroupState {
                    role,
                    ..PictureGroupState::default()
                });
                let current_uc = uc_stack.last().copied().unwrap_or(1);
                uc_stack.push(current_uc);
                let current_hidden = hidden_stack.last().copied().unwrap_or(false);
                hidden_stack.push(current_hidden);
                let current_codepage = ansi_codepage_stack.last().copied().unwrap_or(1252);
                ansi_codepage_stack.push(current_codepage);
                let current_font = font_id_stack.last().copied().flatten();
                font_id_stack.push(current_font);
                fmt_tracker.push();
                pending_boundary_space = false;
            }
            '}' => {
                group_depth -= 1;
                expect_destination = false;
                ignorable_pending = false;
                if let Some(picture_group) = picture_groups.pop() {
                    match picture_group.wrapper {
                        Some(PictureWrapper::Primary) => {
                            pending_primary = Some(PendingPrimary {
                                parent_depth: group_depth,
                                has_image: images.len() > picture_group.image_start,
                            });
                        }
                        Some(PictureWrapper::Fallback) => pending_primary = None,
                        None => {
                            if pending_primary.is_some_and(|primary| primary.parent_depth >= group_depth) {
                                pending_primary = None;
                            }
                        }
                    }
                }
                fmt_tracker.pop(result.len());
                if uc_stack.len() > 1 {
                    uc_stack.pop();
                }
                if hidden_stack.len() > 1 {
                    hidden_stack.pop();
                }
                if ansi_codepage_stack.len() > 1 {
                    ansi_codepage_stack.pop();
                }
                if font_id_stack.len() > 1 {
                    font_id_stack.pop();
                }
                if skip_depth > 0 && group_depth < skip_depth {
                    skip_depth = 0;
                }
                close_listtext_group(
                    group_depth,
                    &mut in_listtext,
                    listtext_depth,
                    &mut listtext_buf,
                    &mut cur_ordered,
                );
                close_fldinst_group(
                    group_depth,
                    &mut in_fldinst,
                    fldinst_depth,
                    &mut fldinst_content,
                    &mut pending_hyperlink_url,
                );
                close_fldrslt_group(
                    group_depth,
                    result.len(),
                    FldrsltCloseState {
                        in_fldrslt: &mut in_fldrslt,
                        fldrslt_depth,
                        fldrslt_start,
                        pending_hyperlink_url: &mut pending_hyperlink_url,
                        hyperlinks: &mut hyperlinks,
                    },
                );
                close_footnote_group(
                    group_depth,
                    &mut in_footnote,
                    footnote_depth,
                    &mut footnote_buf,
                    &mut footnotes,
                );
                close_shptxt_group(
                    group_depth,
                    &mut in_shptxt,
                    shptxt_depth,
                    &mut shptxt_buf,
                    &mut text_boxes,
                );
                close_annotation_group(
                    group_depth,
                    &mut in_annotation,
                    annotation_depth,
                    &mut annotation_buf,
                    &mut comments,
                    &mut pending_atnid,
                );
                let produced_text = group_has_text.pop().unwrap_or(false);
                if produced_text && skip_depth == 0 {
                    pending_boundary_space = true;
                }
            }
            '\\' => {
                let mut escape_ctx = TextEscapeCtx {
                    result: &mut result,
                    table_state: &mut table_state,
                    tables: &mut tables,
                    images: &mut images,
                    ensure_table: &ensure_table,
                    finalize_table: &finalize_table,
                    plain,
                    group_has_text: &mut group_has_text,
                    cur_heading_level: &mut cur_heading_level,
                    cur_list_level: &mut cur_list_level,
                    cur_list_id: &mut cur_list_id,
                    cur_ordered: &mut cur_ordered,
                    para_metas: &mut para_metas,
                    para_meta_emitted: &mut para_meta_emitted,
                    uc_stack: &mut uc_stack,
                    ansi_codepage_stack: &mut ansi_codepage_stack,
                    footnote_count: &mut footnote_count,
                    pending_boundary_space: &mut pending_boundary_space,
                    hidden_stack: &mut hidden_stack,
                    fmt_tracker: &mut fmt_tracker,
                    font_id_stack: &mut font_id_stack,
                    default_font_id: &mut default_font_id,
                    font_charsets: &font_charsets,
                    group_depth,
                    skip_depth: &mut skip_depth,
                    expect_destination: &mut expect_destination,
                    ignorable_pending: &mut ignorable_pending,
                    in_fldinst: &mut in_fldinst,
                    fldinst_depth: &mut fldinst_depth,
                    fldinst_content: &mut fldinst_content,
                    in_fldrslt: &mut in_fldrslt,
                    fldrslt_depth: &mut fldrslt_depth,
                    fldrslt_start: &mut fldrslt_start,
                    in_listtext: &mut in_listtext,
                    listtext_depth: &mut listtext_depth,
                    listtext_buf: &mut listtext_buf,
                    in_footnote: &mut in_footnote,
                    footnote_depth: &mut footnote_depth,
                    footnote_buf: &mut footnote_buf,
                    in_shptxt: &mut in_shptxt,
                    shptxt_depth: &mut shptxt_depth,
                    shptxt_buf: &mut shptxt_buf,
                    in_annotation: &mut in_annotation,
                    annotation_depth: &mut annotation_depth,
                    annotation_buf: &mut annotation_buf,
                    pending_atnid: &mut pending_atnid,
                    picture_groups: &mut picture_groups,
                    pending_primary: &mut pending_primary,
                };
                escape_ctx.handle(&mut chars);
            }
            '\n' | '\r' => {}
            ' ' | '\t' => {
                if in_fldinst {
                    fldinst_content.push(' ');
                }
                if in_footnote {
                    footnote_buf.push(' ');
                }
                if in_shptxt {
                    shptxt_buf.push(' ');
                }
                if in_annotation {
                    annotation_buf.push(' ');
                }
                if skip_depth > 0 && !in_footnote && !in_shptxt && !in_annotation {
                    continue;
                }
                if in_footnote || in_shptxt || in_annotation {
                    continue;
                }
                if let Some(state) = table_state.as_mut()
                    && state.in_row
                {
                    if !state.current_cell.ends_with(' ') {
                        state.current_cell.push(' ');
                    }
                } else if !result.is_empty() && !result.ends_with(' ') && !result.ends_with('\n') {
                    result.push(' ');
                    if let Some(flag) = group_has_text.last_mut() {
                        *flag = true;
                    }
                }
            }
            _ => {
                expect_destination = false;
                pending_primary = None;
                if in_fldinst {
                    fldinst_content.push(ch);
                }
                if in_footnote {
                    footnote_buf.push(ch);
                }
                if in_shptxt {
                    shptxt_buf.push(ch);
                }
                if in_annotation {
                    annotation_buf.push(ch);
                }
                if in_listtext {
                    listtext_buf.push(ch);
                }
                if skip_depth > 0 {
                    continue;
                }
                if hidden_stack.last().copied().unwrap_or(false) {
                    continue;
                }
                if let Some(state) = table_state.as_ref()
                    && !state.in_row
                    && !state.rows.is_empty()
                {
                    finalize_table(&mut table_state, &mut tables);
                }
                if let Some(state) = table_state.as_mut()
                    && state.in_row
                {
                    state.current_cell.push(ch);
                } else {
                    if pending_boundary_space && !result.is_empty() && !result.ends_with(' ') && !result.ends_with('\n')
                    {
                        result.push(' ');
                    }
                    pending_boundary_space = false;
                    para_meta_emitted = false;
                    result.push(ch);
                    if let Some(flag) = group_has_text.last_mut() {
                        *flag = true;
                    }
                }
            }
        }
    }

    if table_state.is_some() {
        finalize_table(&mut table_state, &mut tables);
    }

    fmt_tracker.finalize(result.len());

    let (normalized, mapping) = normalize_whitespace_with_mapping(&result);
    let final_text = normalized.trim_end();
    if !final_text.is_empty() {
        let para_count = normalized.split("\n\n").filter(|p| !p.trim().is_empty()).count();
        while para_metas.len() < para_count {
            para_metas.push(ParagraphMeta {
                heading_level: cur_heading_level,
                list_level: cur_list_level,
                list_id: cur_list_id,
                is_table: false,
                ordered: cur_ordered,
            });
        }
    }

    let mut final_result = normalized;
    if !footnotes.is_empty() {
        if !final_result.ends_with('\n') {
            final_result.push('\n');
            final_result.push('\n');
        }
        for (i, note) in footnotes.iter().enumerate() {
            final_result.push_str(&format!("[^{}]: {}", i + 1, note.trim()));
            final_result.push('\n');
            final_result.push('\n');
        }
    }

    if !text_boxes.is_empty() {
        if !final_result.ends_with('\n') {
            final_result.push('\n');
            final_result.push('\n');
        }
        for text_box in &text_boxes {
            final_result.push_str(text_box);
            final_result.push('\n');
            final_result.push('\n');
        }
    }

    if !comments.is_empty() {
        if !final_result.ends_with('\n') {
            final_result.push('\n');
            final_result.push('\n');
        }
        for comment in &comments {
            final_result.push_str(comment);
            final_result.push('\n');
            final_result.push('\n');
        }
    }

    fmt_tracker.remap_spans(&mapping);

    for link in &mut hyperlinks {
        link.0 = map_offset(&mapping, link.0);
        link.1 = map_offset(&mapping, link.1);
    }
    hyperlinks.retain(|l| l.0 < l.1);

    let formatting_data = RtfFormattingData {
        spans: fmt_tracker.spans,
        color_table,
        header_text: None,
        footer_text: None,
        hyperlinks,
    };

    (final_result, tables, images, para_metas, formatting_data)
}

#[cfg(test)]
#[path = "text_extract_tests.rs"]
mod issue_86_destination_tests;
