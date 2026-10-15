//! The formatting-metadata pass: [`extract_rtf_formatting`] and the
//! span-to-annotation conversion consumed by callers of the text-extraction
//! pass in [`super::text_extract`].

use super::{
    FldrsltCloseState, RtfFormattingData, RtfFormattingSpan, close_fldinst_group, close_fldrslt_group,
    consume_adjacent_hex_escape, parse_font_charset_table, parse_rtf_color_table, resolve_decode_codepage,
};
use crate::extractors::rtf::encoding::{decode_ansi_bytes, parse_hex_byte, parse_rtf_control_word};
use crate::types::TextAnnotation;
use crate::types::document_structure::AnnotationKind;
use std::collections::HashMap;
use std::iter::Peekable;
use std::str::Chars;

#[derive(Clone, Default, PartialEq, Eq)]
struct FmtState {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    color_idx: u16,
}

#[derive(Default)]
struct FormattingSpans {
    spans: Vec<RtfFormattingSpan>,
    span_start: usize,
    text_offset: usize,
    fmt: FmtState,
    stack: Vec<FmtState>,
}

impl FormattingSpans {
    fn close_span(&mut self) {
        if self.text_offset > self.span_start {
            self.spans.push(RtfFormattingSpan {
                start: self.span_start,
                end: self.text_offset,
                bold: self.fmt.bold,
                italic: self.fmt.italic,
                underline: self.fmt.underline,
                strikethrough: self.fmt.strikethrough,
                color_index: self.fmt.color_idx,
            });
        }
        self.span_start = self.text_offset;
    }

    fn restore(&mut self) {
        if let Some(parent) = self.stack.pop()
            && self.fmt != parent
        {
            self.close_span();
            self.fmt = parent;
        }
    }
}

#[derive(Default)]
struct FormattingScope {
    font_charsets: HashMap<u16, u32>,
    ansi_codepage_stack: Vec<u32>,
    font_id_stack: Vec<Option<u16>>,
    default_font_id: Option<u16>,
    group_has_text: Vec<bool>,
    pending_boundary_space: bool,
    expect_destination: bool,
    ignorable_pending: bool,
    group_depth: i32,
    skip_depth: i32,
}

#[derive(Default)]
struct FieldCapture {
    in_fldinst: bool,
    fldinst_depth: i32,
    fldinst_content: String,
    in_fldrslt: bool,
    fldrslt_depth: i32,
    fldrslt_start: usize,
    pending_hyperlink_url: Option<String>,
    hyperlinks: Vec<(usize, usize, String)>,
}

impl FieldCapture {
    fn close_group(&mut self, group_depth: i32, text_offset: usize) {
        close_fldinst_group(
            group_depth,
            &mut self.in_fldinst,
            self.fldinst_depth,
            &mut self.fldinst_content,
            &mut self.pending_hyperlink_url,
        );
        close_fldrslt_group(
            group_depth,
            text_offset,
            FldrsltCloseState {
                in_fldrslt: &mut self.in_fldrslt,
                fldrslt_depth: self.fldrslt_depth,
                fldrslt_start: self.fldrslt_start,
                pending_hyperlink_url: &mut self.pending_hyperlink_url,
                hyperlinks: &mut self.hyperlinks,
            },
        );
    }
}

#[derive(Default)]
struct TextCapture {
    active: bool,
    depth: i32,
    text: String,
}

#[derive(Default)]
struct FormattingParser {
    formatting: FormattingSpans,
    scope: FormattingScope,
    fields: FieldCapture,
    header: TextCapture,
    footer: TextCapture,
}

const SKIP_DESTINATIONS: &[&str] = &[
    "fonttbl",
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
    "colortbl",
    "pict",
];

impl FormattingParser {
    fn new(content: &str) -> Self {
        Self {
            scope: FormattingScope {
                font_charsets: parse_font_charset_table(content),
                ansi_codepage_stack: vec![1252],
                font_id_stack: vec![None],
                ..Default::default()
            },
            ..Default::default()
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
        if self.formatting.fmt != FmtState::default() {
            self.formatting.close_span();
        }
    }

    fn open_group(&mut self) {
        self.scope.group_depth += 1;
        self.scope.expect_destination = true;
        self.formatting.stack.push(self.formatting.fmt.clone());
        self.scope.group_has_text.push(false);
        self.scope.pending_boundary_space = false;
        let codepage = self.scope.ansi_codepage_stack.last().copied().unwrap_or(1252);
        self.scope.ansi_codepage_stack.push(codepage);
        let font = self.scope.font_id_stack.last().copied().flatten();
        self.scope.font_id_stack.push(font);
    }

    fn close_group(&mut self) {
        self.scope.group_depth -= 1;
        self.scope.expect_destination = false;
        self.scope.ignorable_pending = false;
        if self.scope.ansi_codepage_stack.len() > 1 {
            self.scope.ansi_codepage_stack.pop();
        }
        if self.scope.font_id_stack.len() > 1 {
            self.scope.font_id_stack.pop();
        }
        self.formatting.restore();
        if self.scope.skip_depth > 0 && self.scope.group_depth < self.scope.skip_depth {
            self.scope.skip_depth = 0;
        }
        if self.header.active && self.scope.group_depth < self.header.depth {
            self.header.active = false;
        }
        if self.footer.active && self.scope.group_depth < self.footer.depth {
            self.footer.active = false;
        }
        self.fields
            .close_group(self.scope.group_depth, self.formatting.text_offset);
        let produced_text = self.scope.group_has_text.pop().unwrap_or(false);
        if produced_text && self.scope.skip_depth == 0 {
            self.scope.pending_boundary_space = true;
        }
    }

    fn handle_whitespace(&mut self) {
        if self.fields.in_fldinst {
            self.fields.fldinst_content.push(' ');
        }
        if self.scope.skip_depth > 0 {
            return;
        }
        if self.formatting.text_offset > 0 {
            self.formatting.text_offset += 1;
            if let Some(flag) = self.scope.group_has_text.last_mut() {
                *flag = true;
            }
        }
    }

    fn handle_text(&mut self, character: char) {
        if self.fields.in_fldinst {
            self.fields.fldinst_content.push(character);
            return;
        }
        if self.scope.skip_depth > 0 {
            return;
        }
        self.add_boundary_space();
        self.formatting.text_offset += character.len_utf8();
        if let Some(flag) = self.scope.group_has_text.last_mut() {
            *flag = true;
        }
        if self.header.active {
            self.header.text.push(character);
        }
        if self.footer.active {
            self.footer.text.push(character);
        }
    }

    fn handle_escape(&mut self, chars: &mut Peekable<Chars<'_>>) {
        let Some(&next_ch) = chars.peek() else {
            return;
        };
        match next_ch {
            '\\' | '{' | '}' => self.handle_literal(next_ch, chars),
            '\'' => self.handle_hex_escape(chars),
            '*' => {
                chars.next();
                self.scope.ignorable_pending = true;
            }
            _ => self.handle_control_word(chars),
        }
    }

    fn handle_literal(&mut self, next_ch: char, chars: &mut Peekable<Chars<'_>>) {
        chars.next();
        self.scope.expect_destination = false;
        if self.fields.in_fldinst {
            self.fields.fldinst_content.push(next_ch);
        }
        if self.scope.skip_depth > 0 {
            return;
        }
        self.add_boundary_space();
        self.formatting.text_offset += next_ch.len_utf8();
        if let Some(flag) = self.scope.group_has_text.last_mut() {
            *flag = true;
        }
        if self.header.active {
            self.header.text.push(next_ch);
        }
        if self.footer.active {
            self.footer.text.push(next_ch);
        }
    }

    fn handle_hex_escape(&mut self, chars: &mut Peekable<Chars<'_>>) {
        chars.next();
        self.scope.expect_destination = false;
        let bytes = parse_hex_escape_bytes(chars);
        if self.scope.skip_depth > 0 {
            return;
        }
        let Some(bytes) = bytes.as_deref() else {
            return;
        };
        let codepage = resolve_decode_codepage(
            &self.scope.font_id_stack,
            self.scope.default_font_id,
            &self.scope.font_charsets,
            &self.scope.ansi_codepage_stack,
        );
        let decoded = decode_ansi_bytes(bytes, codepage);
        self.add_boundary_space();
        self.formatting.text_offset += decoded.len();
        if let Some(flag) = self.scope.group_has_text.last_mut() {
            *flag = true;
        }
    }

    fn add_boundary_space(&mut self) {
        if self.scope.pending_boundary_space && self.formatting.text_offset > 0 {
            self.formatting.text_offset += 1;
        }
        self.scope.pending_boundary_space = false;
    }

    fn handle_control_word(&mut self, chars: &mut Peekable<Chars<'_>>) {
        let (word, param) = parse_rtf_control_word(chars);
        if self.scope.expect_destination || self.scope.ignorable_pending {
            self.scope.expect_destination = false;
            if self.handle_destination(&word) {
                return;
            }
        }
        if self.fields.in_fldinst {
            self.fields.fldinst_content.push_str(&word);
        }
        self.update_scope(&word, param);
        if self.scope.skip_depth > 0 {
            return;
        }
        if self.update_format(&word, param) {
            return;
        }
        self.emit_control_word(&word, param, chars);
    }

    fn handle_destination(&mut self, word: &str) -> bool {
        if self.scope.ignorable_pending {
            self.scope.ignorable_pending = false;
            if word == "fldinst" {
                self.fields.in_fldinst = true;
                self.fields.fldinst_depth = self.scope.group_depth;
            }
            self.start_skip();
            return true;
        }
        match word {
            "fldinst" => {
                self.fields.in_fldinst = true;
                self.fields.fldinst_depth = self.scope.group_depth;
                self.start_skip();
            }
            "fldrslt" => {
                self.fields.in_fldrslt = true;
                self.fields.fldrslt_depth = self.scope.group_depth;
                self.fields.fldrslt_start = self.formatting.text_offset;
            }
            destination if SKIP_DESTINATIONS.contains(&destination) => self.start_skip(),
            _ => return false,
        }
        true
    }

    fn start_skip(&mut self) {
        if self.scope.skip_depth == 0 {
            self.scope.skip_depth = self.scope.group_depth;
        }
    }

    fn update_scope(&mut self, word: &str, param: Option<i32>) {
        match word {
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
            _ => {}
        }
    }

    fn update_format(&mut self, word: &str, param: Option<i32>) -> bool {
        let mut next = self.formatting.fmt.clone();
        match word {
            "b" => next.bold = param.unwrap_or(1) != 0,
            "i" => next.italic = param.unwrap_or(1) != 0,
            "ul" => next.underline = param.unwrap_or(1) != 0,
            "ulnone" => next.underline = false,
            "strike" => next.strikethrough = param.unwrap_or(1) != 0,
            "cf" => next.color_idx = param.unwrap_or(0) as u16,
            "plain" => next = FmtState::default(),
            _ => return false,
        }
        if self.formatting.fmt != next {
            self.formatting.close_span();
            self.formatting.fmt = next;
        }
        true
    }

    fn emit_control_word(&mut self, word: &str, param: Option<i32>, chars: &mut Peekable<Chars<'_>>) {
        match word {
            "header" | "headerl" | "headerr" | "headerf" => {
                self.header.active = true;
                self.header.depth = self.scope.group_depth;
            }
            "footer" | "footerl" | "footerr" | "footerf" => {
                self.footer.active = true;
                self.footer.depth = self.scope.group_depth;
            }
            "par" | "line" => {
                self.formatting.text_offset += 1;
                if self.header.active {
                    self.header.text.push('\n');
                }
                if self.footer.active {
                    self.footer.text.push('\n');
                }
            }
            "tab" => self.formatting.text_offset += 1,
            "bullet" => self.formatting.text_offset += '\u{2022}'.len_utf8(),
            "lquote" => self.formatting.text_offset += '\u{2018}'.len_utf8(),
            "rquote" => self.formatting.text_offset += '\u{2019}'.len_utf8(),
            "ldblquote" => self.formatting.text_offset += '\u{201C}'.len_utf8(),
            "rdblquote" => self.formatting.text_offset += '\u{201D}'.len_utf8(),
            "endash" => self.formatting.text_offset += '\u{2013}'.len_utf8(),
            "emdash" => self.formatting.text_offset += '\u{2014}'.len_utf8(),
            "u" => self.emit_unicode(param, chars),
            _ => {}
        }
    }

    fn emit_unicode(&mut self, param: Option<i32>, chars: &mut Peekable<Chars<'_>>) {
        if let Some(code_num) = param {
            let code_u = if code_num < 0 {
                (code_num + 65536) as u32
            } else {
                code_num as u32
            };
            if let Some(character) = char::from_u32(code_u) {
                self.formatting.text_offset += character.len_utf8();
                if self.header.active {
                    self.header.text.push(character);
                }
                if self.footer.active {
                    self.footer.text.push(character);
                }
            }
        }
        if chars.peek().is_some_and(|next| !matches!(next, '\\' | '{' | '}')) {
            chars.next();
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

/// Extract formatting metadata from RTF content.
///
/// Counts UTF-8 bytes with the same scoped codepages as the text pass. ~keep
pub(crate) fn extract_rtf_formatting(content: &str) -> RtfFormattingData {
    let mut parser = FormattingParser::new(content);
    parser.parse(content);
    RtfFormattingData {
        spans: parser.formatting.spans,
        color_table: parse_rtf_color_table(content),
        header_text: nonempty_text(&parser.header.text),
        footer_text: nonempty_text(&parser.footer.text),
        hyperlinks: parser.fields.hyperlinks,
    }
}

fn nonempty_text(text: &str) -> Option<String> {
    let text = text.trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// Convert formatting spans overlapping a paragraph into paragraph-relative annotations. ~keep
pub(crate) fn spans_to_annotations(
    para_start: usize,
    para_end: usize,
    formatting: &RtfFormattingData,
) -> Vec<TextAnnotation> {
    let mut annotations = Vec::new();
    for span in &formatting.spans {
        if span.end <= para_start || span.start >= para_end {
            continue;
        }
        let start = span.start.max(para_start) - para_start;
        let end = span.end.min(para_end) - para_start;
        if start < end {
            append_span_annotations(
                &mut annotations,
                span,
                start as u32,
                end as u32,
                &formatting.color_table,
            );
        }
    }
    for (link_start, link_end, url) in &formatting.hyperlinks {
        if *link_end <= para_start || *link_start >= para_end {
            continue;
        }
        let start = (link_start.max(&para_start) - para_start) as u32;
        let end = (link_end.min(&para_end) - para_start) as u32;
        if start < end {
            annotations.push(TextAnnotation {
                start,
                end,
                kind: AnnotationKind::Link {
                    url: url.clone(),
                    title: None,
                },
            });
        }
    }
    annotations
}

fn append_span_annotations(
    annotations: &mut Vec<TextAnnotation>,
    span: &RtfFormattingSpan,
    start: u32,
    end: u32,
    color_table: &[String],
) {
    let attributes = [
        (span.bold, AnnotationKind::Bold),
        (span.italic, AnnotationKind::Italic),
        (span.underline, AnnotationKind::Underline),
        (span.strikethrough, AnnotationKind::Strikethrough),
    ];
    for (active, kind) in attributes {
        if active {
            annotations.push(TextAnnotation { start, end, kind });
        }
    }
    if span.color_index > 0
        && let Some(color) = color_table.get(span.color_index as usize)
        && !color.is_empty()
        && color != "#000000"
    {
        annotations.push(TextAnnotation {
            start,
            end,
            kind: AnnotationKind::Color { value: color.clone() },
        });
    }
}

#[cfg(test)]
#[path = "formatting_extract_tests.rs"]
mod tests;
