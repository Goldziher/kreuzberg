//! The text-extraction pass: [`extract_text_from_rtf`] and RTF control-word
//! dispatch (`handle_control_word` and its per-word helpers).

use super::control_word::{ControlWordCtx, handle_control_word};
use super::group_close::{close_annotation_group, close_footnote_group, close_listtext_group, close_shptxt_group};
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

#[derive(Default)]
struct TextOutput {
    result: String,
    table_state: Option<TableState>,
    tables: Vec<Table>,
    images: Vec<RtfImage>,
    plain: bool,
}

#[derive(Default)]
struct TextScope {
    group_has_text: Vec<bool>,
    uc_stack: Vec<u8>,
    ansi_codepage_stack: Vec<u32>,
    pending_boundary_space: bool,
    hidden_stack: Vec<bool>,
    font_id_stack: Vec<Option<u16>>,
    // \deff is document-wide; active font and codepage inherit group scopes. ~keep
    default_font_id: Option<u16>,
    font_charsets: HashMap<u16, u32>,
    group_depth: i32,
    skip_depth: i32,
    expect_destination: bool,
    ignorable_pending: bool,
    picture_groups: Vec<PictureGroupState>,
    pending_primary: Option<PendingPrimary>,
}

#[derive(Default)]
struct ParagraphState {
    cur_heading_level: u8,
    cur_list_level: Option<u8>,
    cur_list_id: Option<u16>,
    cur_ordered: bool,
    para_metas: Vec<ParagraphMeta>,
    para_meta_emitted: bool,
}

#[derive(Default)]
struct FieldState {
    in_fldinst: bool,
    fldinst_depth: i32,
    fldinst_content: String,
    in_fldrslt: bool,
    fldrslt_depth: i32,
    fldrslt_start: usize,
    pending_hyperlink_url: Option<String>,
    hyperlinks: Vec<(usize, usize, String)>,
}

#[derive(Default)]
struct CapturedText {
    active: bool,
    depth: i32,
    buffer: String,
    items: Vec<String>,
}

impl CapturedText {
    fn start(&mut self, depth: i32) {
        self.active = true;
        self.depth = depth;
        self.buffer.clear();
    }
}

/// Buffered content survives ignorable ancestors and is appended after the body (#86). ~keep
#[derive(Default)]
struct TextDestinations {
    listtext: CapturedText,
    footnote: CapturedText,
    shptxt: CapturedText,
    annotation: CapturedText,
    footnote_count: usize,
    pending_atnid: Option<i32>,
}

struct TextParser {
    output: TextOutput,
    scope: TextScope,
    paragraph: ParagraphState,
    fields: FieldState,
    destinations: TextDestinations,
    fmt_tracker: FormattingTracker,
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

/// Settle `pending_primary` when a picture group's closing `}` is reached.
fn close_picture_group(
    picture_group: PictureGroupState,
    group_depth: i32,
    image_count: usize,
    pending_primary: &mut Option<PendingPrimary>,
) {
    match picture_group.wrapper {
        Some(PictureWrapper::Primary) => {
            *pending_primary = Some(PendingPrimary {
                parent_depth: group_depth,
                has_image: image_count > picture_group.image_start,
            });
        }
        Some(PictureWrapper::Fallback) => *pending_primary = None,
        None => {
            if pending_primary.is_some_and(|primary| primary.parent_depth >= group_depth) {
                *pending_primary = None;
            }
        }
    }
}

impl TextParser {
    fn new(content: &str, plain: bool) -> Self {
        Self {
            output: TextOutput {
                plain,
                ..Default::default()
            },
            scope: TextScope {
                font_charsets: parse_font_charset_table(content),
                uc_stack: vec![1],
                hidden_stack: vec![false],
                ansi_codepage_stack: vec![1252],
                font_id_stack: vec![None],
                picture_groups: vec![PictureGroupState::default()],
                ..Default::default()
            },
            paragraph: ParagraphState::default(),
            fields: FieldState::default(),
            destinations: TextDestinations::default(),
            fmt_tracker: FormattingTracker::new(),
        }
    }

    fn parse(&mut self, content: &str) {
        let mut chars = content.chars().peekable();
        while let Some(character) = chars.next() {
            match character {
                '{' => self.open_group(),
                '}' => self.close_group(),
                '\\' => self.handle_escape(&mut chars),
                '\n' | '\r' => {}
                ' ' | '\t' => self.handle_whitespace(),
                _ => self.handle_text(character),
            }
        }
    }

    fn open_group(&mut self) {
        self.scope.group_depth += 1;
        self.scope.expect_destination = true;
        self.scope.group_has_text.push(false);
        let role = self
            .scope
            .picture_groups
            .last()
            .map(|group| group.role)
            .unwrap_or_default();
        self.scope.picture_groups.push(PictureGroupState {
            role,
            ..PictureGroupState::default()
        });
        let uc = self.scope.uc_stack.last().copied().unwrap_or(1);
        self.scope.uc_stack.push(uc);
        let hidden = self.scope.hidden_stack.last().copied().unwrap_or(false);
        self.scope.hidden_stack.push(hidden);
        let codepage = self.scope.ansi_codepage_stack.last().copied().unwrap_or(1252);
        self.scope.ansi_codepage_stack.push(codepage);
        let font = self.scope.font_id_stack.last().copied().flatten();
        self.scope.font_id_stack.push(font);
        self.fmt_tracker.push();
        self.scope.pending_boundary_space = false;
    }

    fn close_group(&mut self) {
        self.scope.group_depth -= 1;
        self.scope.expect_destination = false;
        self.scope.ignorable_pending = false;
        if let Some(picture_group) = self.scope.picture_groups.pop() {
            close_picture_group(
                picture_group,
                self.scope.group_depth,
                self.output.images.len(),
                &mut self.scope.pending_primary,
            );
        }
        self.fmt_tracker.pop(self.output.result.len());
        pop_scope(&mut self.scope.uc_stack);
        pop_scope(&mut self.scope.hidden_stack);
        pop_scope(&mut self.scope.ansi_codepage_stack);
        pop_scope(&mut self.scope.font_id_stack);
        if self.scope.skip_depth > 0 && self.scope.group_depth < self.scope.skip_depth {
            self.scope.skip_depth = 0;
        }
        let listtext = &mut self.destinations.listtext;
        close_listtext_group(
            self.scope.group_depth,
            &mut listtext.active,
            listtext.depth,
            &mut listtext.buffer,
            &mut self.paragraph.cur_ordered,
        );
        self.close_fields();
        self.close_captures();
        let produced_text = self.scope.group_has_text.pop().unwrap_or(false);
        if produced_text && self.scope.skip_depth == 0 {
            self.scope.pending_boundary_space = true;
        }
    }

    fn close_fields(&mut self) {
        close_fldinst_group(
            self.scope.group_depth,
            &mut self.fields.in_fldinst,
            self.fields.fldinst_depth,
            &mut self.fields.fldinst_content,
            &mut self.fields.pending_hyperlink_url,
        );
        close_fldrslt_group(
            self.scope.group_depth,
            self.output.result.len(),
            FldrsltCloseState {
                in_fldrslt: &mut self.fields.in_fldrslt,
                fldrslt_depth: self.fields.fldrslt_depth,
                fldrslt_start: self.fields.fldrslt_start,
                pending_hyperlink_url: &mut self.fields.pending_hyperlink_url,
                hyperlinks: &mut self.fields.hyperlinks,
            },
        );
    }

    fn close_captures(&mut self) {
        let footnote = &mut self.destinations.footnote;
        close_footnote_group(
            self.scope.group_depth,
            &mut footnote.active,
            footnote.depth,
            &mut footnote.buffer,
            &mut footnote.items,
        );
        let shptxt = &mut self.destinations.shptxt;
        close_shptxt_group(
            self.scope.group_depth,
            &mut shptxt.active,
            shptxt.depth,
            &mut shptxt.buffer,
            &mut shptxt.items,
        );
        let annotation = &mut self.destinations.annotation;
        close_annotation_group(
            self.scope.group_depth,
            &mut annotation.active,
            annotation.depth,
            &mut annotation.buffer,
            &mut annotation.items,
            &mut self.destinations.pending_atnid,
        );
    }

    fn capture_auxiliary_character(&mut self, character: char) {
        for capture in [
            &mut self.destinations.footnote,
            &mut self.destinations.shptxt,
            &mut self.destinations.annotation,
        ] {
            if capture.active {
                capture.buffer.push(character);
            }
        }
    }

    fn handle_whitespace(&mut self) {
        if self.fields.in_fldinst {
            self.fields.fldinst_content.push(' ');
        }
        self.capture_auxiliary_character(' ');
        if self.scope.skip_depth > 0
            || self.destinations.footnote.active
            || self.destinations.shptxt.active
            || self.destinations.annotation.active
        {
            return;
        }
        if let Some(state) = self.output.table_state.as_mut()
            && state.in_row
        {
            if !state.current_cell.ends_with(' ') {
                state.current_cell.push(' ');
            }
        } else if !self.output.result.is_empty()
            && !self.output.result.ends_with(' ')
            && !self.output.result.ends_with('\n')
        {
            self.output.result.push(' ');
            if let Some(flag) = self.scope.group_has_text.last_mut() {
                *flag = true;
            }
        }
    }

    fn handle_text(&mut self, character: char) {
        self.scope.expect_destination = false;
        self.scope.pending_primary = None;
        if self.fields.in_fldinst {
            self.fields.fldinst_content.push(character);
        }
        self.capture_auxiliary_character(character);
        if self.destinations.listtext.active {
            self.destinations.listtext.buffer.push(character);
        }
        if self.scope.skip_depth > 0 || self.scope.hidden_stack.last().copied().unwrap_or(false) {
            return;
        }
        if let Some(state) = self.output.table_state.as_ref()
            && !state.in_row
            && !state.rows.is_empty()
        {
            self.finalize_table();
        }
        if let Some(state) = self.output.table_state.as_mut()
            && state.in_row
        {
            state.current_cell.push(character);
            return;
        }
        if self.scope.pending_boundary_space
            && !self.output.result.is_empty()
            && !self.output.result.ends_with(' ')
            && !self.output.result.ends_with('\n')
        {
            self.output.result.push(' ');
        }
        self.scope.pending_boundary_space = false;
        self.paragraph.para_meta_emitted = false;
        self.output.result.push(character);
        if let Some(flag) = self.scope.group_has_text.last_mut() {
            *flag = true;
        }
    }

    fn finalize_table(&mut self) {
        finalize_table(&mut self.output.table_state, &mut self.output.tables, self.output.plain);
    }

    fn complete_paragraph_metadata(&mut self, normalized: &str) {
        if normalized.trim_end().is_empty() {
            return;
        }
        let count = normalized
            .split("\n\n")
            .filter(|paragraph| !paragraph.trim().is_empty())
            .count();
        while self.paragraph.para_metas.len() < count {
            self.paragraph.para_metas.push(ParagraphMeta {
                heading_level: self.paragraph.cur_heading_level,
                list_level: self.paragraph.cur_list_level,
                list_id: self.paragraph.cur_list_id,
                is_table: false,
                ordered: self.paragraph.cur_ordered,
            });
        }
    }

    fn append_captured_text(&self, result: &mut String) {
        if !self.destinations.footnote.items.is_empty() {
            ensure_capture_separator(result);
            for (index, note) in self.destinations.footnote.items.iter().enumerate() {
                result.push_str(&format!("[^{}]: {}", index + 1, note.trim()));
                result.push_str("\n\n");
            }
        }
        for capture in [&self.destinations.shptxt, &self.destinations.annotation] {
            if capture.items.is_empty() {
                continue;
            }
            ensure_capture_separator(result);
            for item in &capture.items {
                result.push_str(item);
                result.push_str("\n\n");
            }
        }
    }

    fn finish(mut self, content: &str) -> (String, Vec<Table>, Vec<RtfImage>, Vec<ParagraphMeta>, RtfFormattingData) {
        if self.output.table_state.is_some() {
            self.finalize_table();
        }
        self.fmt_tracker.finalize(self.output.result.len());
        let (normalized, mapping) = normalize_whitespace_with_mapping(&self.output.result);
        self.complete_paragraph_metadata(&normalized);
        let mut final_result = normalized;
        self.append_captured_text(&mut final_result);
        self.fmt_tracker.remap_spans(&mapping);
        for link in &mut self.fields.hyperlinks {
            link.0 = map_offset(&mapping, link.0);
            link.1 = map_offset(&mapping, link.1);
        }
        self.fields.hyperlinks.retain(|link| link.0 < link.1);
        let formatting = RtfFormattingData {
            spans: self.fmt_tracker.spans,
            color_table: parse_rtf_color_table(content),
            header_text: None,
            footer_text: None,
            hyperlinks: self.fields.hyperlinks,
        };
        (
            final_result,
            self.output.tables,
            self.output.images,
            self.paragraph.para_metas,
            formatting,
        )
    }

    fn control_word_ctx<'a>(
        &'a mut self,
        finalize_table: &'a dyn Fn(&mut Option<TableState>, &mut Vec<Table>),
    ) -> ControlWordCtx<'a> {
        ControlWordCtx {
            result: &mut self.output.result,
            table_state: &mut self.output.table_state,
            tables: &mut self.output.tables,
            images: &mut self.output.images,
            suppress_picture: matches!(
                self.scope.picture_groups.last().map(|group| group.role),
                Some(PictureRole::Fallback { suppress: true })
            ),
            ensure_table: &ensure_table,
            finalize_table,
            plain: self.output.plain,
            group_has_text: &mut self.scope.group_has_text,
            cur_heading_level: &mut self.paragraph.cur_heading_level,
            cur_list_level: &mut self.paragraph.cur_list_level,
            cur_list_id: &mut self.paragraph.cur_list_id,
            cur_ordered: &mut self.paragraph.cur_ordered,
            para_metas: &mut self.paragraph.para_metas,
            para_meta_emitted: &mut self.paragraph.para_meta_emitted,
            uc_stack: &mut self.scope.uc_stack,
            ansi_codepage_stack: &mut self.scope.ansi_codepage_stack,
            footnote_count: &mut self.destinations.footnote_count,
            pending_boundary_space: &mut self.scope.pending_boundary_space,
            hidden_stack: &mut self.scope.hidden_stack,
            fmt_tracker: &mut self.fmt_tracker,
            font_id_stack: &mut self.scope.font_id_stack,
            default_font_id: &mut self.scope.default_font_id,
        }
    }

    fn dispatch_control_word(&mut self, word: &str, param: Option<i32>, chars: &mut Peekable<Chars<'_>>) {
        let plain = self.output.plain;
        let finalize = |state: &mut Option<TableState>, tables: &mut Vec<Table>| finalize_table(state, tables, plain);
        let mut context = self.control_word_ctx(&finalize);
        handle_control_word(word, param, chars, &mut context);
    }

    fn handle_escape(&mut self, chars: &mut Peekable<Chars<'_>>) {
        let Some(&next_ch) = chars.peek() else {
            return;
        };
        match next_ch {
            '\n' | '\r' => self.handle_line_break(next_ch, chars),
            '\\' | '{' | '}' => self.handle_literal(next_ch, chars),
            // Control symbols: a non-breaking space and a non-breaking hyphen are
            // text, an optional hyphen only marks where a word may break.
            '~' => self.handle_literal('\u{a0}', chars),
            '_' => self.handle_literal('\u{2011}', chars),
            '-' => {
                chars.next();
                self.scope.expect_destination = false;
            }
            '\'' => self.handle_hex_escape(chars),
            '*' => {
                chars.next();
                self.scope.ignorable_pending = true;
            }
            _ => self.handle_control_word(chars),
        }
    }

    fn handle_line_break(&mut self, next_ch: char, chars: &mut Peekable<Chars<'_>>) {
        chars.next();
        if next_ch == '\r' && matches!(chars.peek(), Some(&'\n')) {
            chars.next();
        }
        self.scope.expect_destination = false;
        if self.scope.skip_depth > 0 {
            return;
        }
        self.dispatch_control_word("par", None, chars);
    }

    fn handle_literal(&mut self, next_ch: char, chars: &mut Peekable<Chars<'_>>) {
        chars.next();
        self.scope.pending_primary = None;
        self.scope.expect_destination = false;
        if self.fields.in_fldinst {
            self.fields.fldinst_content.push(next_ch);
        }
        self.capture_auxiliary_character(next_ch);
        if self.scope.skip_depth > 0 || self.scope.hidden_stack.last().copied().unwrap_or(false) {
            return;
        }
        // Inside a table row the character belongs to the current cell, as a
        // hex-escaped one does.
        if let Some(state) = self.output.table_state.as_mut()
            && state.in_row
        {
            state.current_cell.push(next_ch);
            return;
        }
        if self.scope.pending_boundary_space
            && !self.output.result.is_empty()
            && !self.output.result.ends_with(' ')
            && !self.output.result.ends_with('\n')
        {
            self.output.result.push(' ');
        }
        self.scope.pending_boundary_space = false;
        self.paragraph.para_meta_emitted = false;
        self.output.result.push(next_ch);
        if let Some(flag) = self.scope.group_has_text.last_mut() {
            *flag = true;
        }
    }

    fn handle_hex_escape(&mut self, chars: &mut Peekable<Chars<'_>>) {
        chars.next();
        self.scope.pending_primary = None;
        self.scope.expect_destination = false;
        let bytes = parse_hex_escape_bytes(chars);
        if (self.destinations.footnote.active || self.destinations.shptxt.active || self.destinations.annotation.active)
            && bytes.is_some()
        {
            self.append_hex_to_destinations(bytes.as_deref().unwrap_or_default());
        }
        if self.scope.skip_depth > 0 || self.scope.hidden_stack.last().copied().unwrap_or(false) {
            return;
        }
        let Some(bytes) = bytes.as_deref() else {
            return;
        };
        let decoded = self.decode_ansi(bytes);
        if let Some(state) = self.output.table_state.as_mut()
            && state.in_row
        {
            state.current_cell.push_str(&decoded);
            return;
        }
        if self.scope.pending_boundary_space
            && !self.output.result.is_empty()
            && !self.output.result.ends_with(' ')
            && !self.output.result.ends_with('\n')
        {
            self.output.result.push(' ');
        }
        self.scope.pending_boundary_space = false;
        self.paragraph.para_meta_emitted = false;
        self.output.result.push_str(&decoded);
        if let Some(flag) = self.scope.group_has_text.last_mut() {
            *flag = true;
        }
    }

    fn decode_ansi(&self, bytes: &[u8]) -> String {
        let codepage = resolve_decode_codepage(
            &self.scope.font_id_stack,
            self.scope.default_font_id,
            &self.scope.font_charsets,
            &self.scope.ansi_codepage_stack,
        );
        decode_ansi_bytes(bytes, codepage)
    }

    fn append_hex_to_destinations(&mut self, bytes: &[u8]) {
        let decoded = self.decode_ansi(bytes);
        if self.destinations.footnote.active {
            self.destinations.footnote.buffer.push_str(&decoded);
        }
        if self.destinations.shptxt.active {
            self.destinations.shptxt.buffer.push_str(&decoded);
        }
        if self.destinations.annotation.active {
            self.destinations.annotation.buffer.push_str(&decoded);
        }
    }

    fn handle_control_word(&mut self, chars: &mut Peekable<Chars<'_>>) {
        let (control_word, param) = parse_rtf_control_word(chars);
        if self.scope.expect_destination || self.scope.ignorable_pending {
            self.scope.expect_destination = false;
            if self.handle_destination(&control_word, param) {
                return;
            }
        }
        if self.scope.skip_depth > 0 {
            self.handle_skipped_control_word(&control_word, param, chars);
            return;
        }
        self.scope.pending_primary = None;
        self.dispatch_control_word(&control_word, param, chars);
    }

    fn handle_picture_destination(&mut self, word: &str) -> bool {
        match word {
            "shppict" => {
                self.scope.pending_primary = None;
                if let Some(group) = self.scope.picture_groups.last_mut() {
                    group.role = PictureRole::Primary;
                    group.wrapper = Some(PictureWrapper::Primary);
                    group.image_start = self.output.images.len();
                }
            }
            "nonshppict" => {
                let parent_depth = self.scope.group_depth - 1;
                let suppress = self
                    .scope
                    .pending_primary
                    .take()
                    .is_some_and(|primary| primary.parent_depth == parent_depth && primary.has_image);
                if let Some(group) = self.scope.picture_groups.last_mut() {
                    group.role = PictureRole::Fallback { suppress };
                    group.wrapper = Some(PictureWrapper::Fallback);
                    group.image_start = self.output.images.len();
                }
            }
            _ => return false,
        }
        self.scope.ignorable_pending = false;
        true
    }

    fn handle_destination(&mut self, control_word: &str, param: Option<i32>) -> bool {
        if self.handle_picture_destination(control_word) {
            return true;
        }
        self.scope.pending_primary = None;
        if self.scope.ignorable_pending {
            self.scope.ignorable_pending = false;
            if self.handle_ignorable_destination(control_word, param) {
                return true;
            }
        }
        match control_word {
            "listtext" | "pntext" => {
                self.destinations.listtext.start(self.scope.group_depth);
                self.start_skip();
            }
            "fldinst" => {
                self.fields.in_fldinst = true;
                self.fields.fldinst_depth = self.scope.group_depth;
                self.start_skip();
            }
            "fldrslt" => {
                self.fields.in_fldrslt = true;
                self.fields.fldrslt_depth = self.scope.group_depth;
                self.fields.fldrslt_start = self.output.result.len();
            }
            "footnote" => {
                self.destinations.footnote.start(self.scope.group_depth);
                self.start_skip();
            }
            "shptxt" => {
                self.destinations.shptxt.start(self.scope.group_depth);
                self.start_skip();
            }
            "annotation" => {
                self.destinations.annotation.start(self.scope.group_depth);
                self.start_skip();
            }
            "atnid" => {
                if let Some(id) = param {
                    self.destinations.pending_atnid = Some(id);
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
                self.fields.in_fldinst = true;
                self.fields.fldinst_depth = self.scope.group_depth;
            }
            "listtext" | "pntext" => {
                self.destinations.listtext.start(self.scope.group_depth);
            }
            "atnid" => {
                if let Some(id) = param {
                    self.destinations.pending_atnid = Some(id);
                }
            }
            "shppict" => return false,
            _ => {}
        }
        self.start_skip();
        true
    }

    fn start_skip(&mut self) {
        if self.scope.skip_depth == 0 {
            self.scope.skip_depth = self.scope.group_depth;
        }
    }

    fn handle_skipped_control_word(&mut self, control_word: &str, param: Option<i32>, chars: &mut Peekable<Chars<'_>>) {
        match control_word {
            "uc" => {
                if let Some(value) = param
                    && let Some(uc) = self.scope.uc_stack.last_mut()
                {
                    *uc = value.max(0) as u8;
                }
            }
            "ansicpg" => {
                if let Some(value) = param
                    && value > 0
                    && let Some(codepage) = self.scope.ansi_codepage_stack.last_mut()
                {
                    *codepage = value as u32;
                }
            }
            "f" => {
                if let Some(value) = param
                    && let Some(font_id) = self.scope.font_id_stack.last_mut()
                {
                    *font_id = Some(value.max(0) as u16);
                }
            }
            "deff" => {
                if let Some(value) = param {
                    self.scope.default_font_id = Some(value.max(0) as u16);
                }
            }
            "u" if self.destinations.footnote.active
                || self.destinations.shptxt.active
                || self.destinations.annotation.active =>
            {
                self.append_skipped_unicode(param, chars);
            }
            "par" | "line"
                if self.destinations.footnote.active
                    || self.destinations.shptxt.active
                    || self.destinations.annotation.active =>
            {
                self.capture_auxiliary_character(' ');
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
            self.capture_auxiliary_character(character);
        }
        let uc_count = self.scope.uc_stack.last().copied().unwrap_or(1);
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

fn pop_scope<T>(stack: &mut Vec<T>) {
    if stack.len() > 1 {
        stack.pop();
    }
}

fn ensure_table(table_state: &mut Option<TableState>) {
    if table_state.is_none() {
        *table_state = Some(TableState::new());
    }
}

fn finalize_table(state: &mut Option<TableState>, tables: &mut Vec<Table>, plain: bool) {
    if let Some(state) = state.take()
        && let Some(table) = state.finalize_with_format(plain)
    {
        tables.push(table);
    }
}

fn ensure_capture_separator(result: &mut String) {
    if !result.ends_with('\n') {
        result.push_str("\n\n");
    }
}

/// Extract normalized text, table/image data and byte-aligned formatting from RTF. ~keep
pub(crate) fn extract_text_from_rtf(
    content: &str,
    plain: bool,
) -> (String, Vec<Table>, Vec<RtfImage>, Vec<ParagraphMeta>, RtfFormattingData) {
    let mut parser = TextParser::new(content, plain);
    parser.parse(content);
    parser.finish(content)
}

#[cfg(test)]
#[path = "text_extract_tests.rs"]
mod issue_86_destination_tests;

#[cfg(test)]
#[path = "text_extract_lint_tests.rs"]
mod lint_tests;
