//! Core RTF parsing logic.
//!
//! Split by responsibility to keep individual files under the repository's
//! line-count limit: this file holds the types and helpers shared by both
//! passes; [`formatting_extract`] holds the formatting-metadata pass;
//! [`text_extract`] holds the text-extraction pass; [`control_word`] holds
//! its control-word dispatch. ~keep

mod control_word;
mod formatting_extract;
mod group_close;
mod text_extract;

pub(crate) use formatting_extract::{extract_rtf_formatting, spans_to_annotations};
pub(crate) use text_extract::extract_text_from_rtf;

use crate::extractors::rtf::encoding::{fcharset_to_codepage, parse_hex_byte, parse_rtf_control_word};
use crate::extractors::rtf::formatting::map_offset;
use std::collections::HashMap;

/// Metadata for a single paragraph extracted from RTF.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default)]
pub struct ParagraphMeta {
    /// Heading level (1-based): 1 = H1, 2 = H2, etc. 0 = not a heading.
    pub heading_level: u8,
    /// List nesting level (0-based). `None` means not a list item.
    pub list_level: Option<u8>,
    /// List override ID (\lsN). Used to detect list boundaries.
    pub list_id: Option<u16>,
    /// Whether this paragraph is a table placeholder (text is in tables vec).
    pub is_table: bool,
    /// Whether this list item is ordered (numbered/lettered). Detected from
    /// `\listtext` or `\pntext` content. `false` = unordered (bullet).
    pub ordered: bool,
}

/// A formatting span tracked during RTF parsing.
#[derive(Debug, Clone)]
pub struct RtfFormattingSpan {
    /// Byte offset in the output text where this format starts.
    pub start: usize,
    /// Byte offset in the output text where this format ends.
    pub end: usize,
    /// Whether bold was active.
    pub bold: bool,
    /// Whether italic was active.
    pub italic: bool,
    /// Whether underline was active.
    pub underline: bool,
    /// Whether strikethrough was active.
    pub strikethrough: bool,
    /// Color index into the color table (0 = default/auto).
    pub color_index: u16,
}

/// RTF formatting metadata extracted alongside text.
pub struct RtfFormattingData {
    /// Formatting spans corresponding to text regions.
    pub spans: Vec<RtfFormattingSpan>,
    /// Color table entries (index 0 is auto/default).
    pub color_table: Vec<String>,
    /// Header text content (from \header groups).
    pub header_text: Option<String>,
    /// Footer text content (from \footer groups).
    pub footer_text: Option<String>,
    /// Hyperlink spans: (start_byte, end_byte, url).
    pub hyperlinks: Vec<(usize, usize, String)>,
}

/// Tracks formatting state during the text extraction pass so that
/// formatting spans have byte offsets that exactly match the extracted text.
///
/// This is used inside `extract_text_from_rtf` to produce spans whose
/// byte ranges are guaranteed to align with the output text, eliminating
/// the offset-drift bug that occurred when formatting was tracked in a
/// separate pass.
#[derive(Clone, Default)]
struct FmtState {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    color_idx: u16,
}

struct FormattingTracker {
    /// Current formatting state.
    fmt: FmtState,
    /// Stack of formatting states pushed on `{` and popped on `}`.
    fmt_stack: Vec<FmtState>,
    /// Byte offset where the current span started.
    span_start: usize,
    /// Accumulated formatting spans (byte offsets into pre-normalized result).
    spans: Vec<RtfFormattingSpan>,
}

impl FormattingTracker {
    fn new() -> Self {
        Self {
            fmt: FmtState::default(),
            fmt_stack: Vec::new(),
            span_start: 0,
            spans: Vec::new(),
        }
    }

    /// Push current formatting state onto the stack (called on `{`).
    fn push(&mut self) {
        self.fmt_stack.push(self.fmt.clone());
    }

    /// Pop formatting state from the stack (called on `}`).
    /// If formatting changed inside the group, close the current span.
    fn pop(&mut self, text_offset: usize) {
        if let Some(parent) = self.fmt_stack.pop() {
            let changed = self.fmt.bold != parent.bold
                || self.fmt.italic != parent.italic
                || self.fmt.underline != parent.underline
                || self.fmt.strikethrough != parent.strikethrough
                || self.fmt.color_idx != parent.color_idx;
            if changed {
                if text_offset > self.span_start {
                    self.spans.push(RtfFormattingSpan {
                        start: self.span_start,
                        end: text_offset,
                        bold: self.fmt.bold,
                        italic: self.fmt.italic,
                        underline: self.fmt.underline,
                        strikethrough: self.fmt.strikethrough,
                        color_index: self.fmt.color_idx,
                    });
                }
                self.span_start = text_offset;
                self.fmt = parent;
            }
        }
    }

    /// Update a formatting field, closing the current span if the value changed.
    fn update_bold(&mut self, text_offset: usize, val: bool) {
        if val != self.fmt.bold {
            self.close_span(text_offset);
            self.fmt.bold = val;
        }
    }

    fn update_italic(&mut self, text_offset: usize, val: bool) {
        if val != self.fmt.italic {
            self.close_span(text_offset);
            self.fmt.italic = val;
        }
    }

    fn update_underline(&mut self, text_offset: usize, val: bool) {
        if val != self.fmt.underline {
            self.close_span(text_offset);
            self.fmt.underline = val;
        }
    }

    fn update_strikethrough(&mut self, text_offset: usize, val: bool) {
        if val != self.fmt.strikethrough {
            self.close_span(text_offset);
            self.fmt.strikethrough = val;
        }
    }

    fn update_color(&mut self, text_offset: usize, val: u16) {
        if val != self.fmt.color_idx {
            self.close_span(text_offset);
            self.fmt.color_idx = val;
        }
    }

    /// Reset all formatting to default, closing the current span if needed.
    fn reset_all(&mut self, text_offset: usize) {
        if self.fmt.bold || self.fmt.italic || self.fmt.underline || self.fmt.strikethrough || self.fmt.color_idx != 0 {
            self.close_span(text_offset);
            self.fmt = FmtState::default();
        }
    }

    fn close_span(&mut self, text_offset: usize) {
        if text_offset > self.span_start {
            self.spans.push(RtfFormattingSpan {
                start: self.span_start,
                end: text_offset,
                bold: self.fmt.bold,
                italic: self.fmt.italic,
                underline: self.fmt.underline,
                strikethrough: self.fmt.strikethrough,
                color_index: self.fmt.color_idx,
            });
        }
        self.span_start = text_offset;
    }

    /// Close the final span at the end of parsing.
    fn finalize(&mut self, text_offset: usize) {
        if text_offset > self.span_start
            && (self.fmt.bold
                || self.fmt.italic
                || self.fmt.underline
                || self.fmt.strikethrough
                || self.fmt.color_idx != 0)
        {
            self.spans.push(RtfFormattingSpan {
                start: self.span_start,
                end: text_offset,
                bold: self.fmt.bold,
                italic: self.fmt.italic,
                underline: self.fmt.underline,
                strikethrough: self.fmt.strikethrough,
                color_index: self.fmt.color_idx,
            });
        }
    }

    /// Remap all span byte offsets using a normalization mapping.
    fn remap_spans(&mut self, mapping: &[(usize, usize)]) {
        for span in &mut self.spans {
            span.start = map_offset(mapping, span.start);
            span.end = map_offset(mapping, span.end);
        }
        self.spans.retain(|s| s.start < s.end);
    }
}

/// Extract the text of a balanced `{...}` group starting at the beginning of
/// `rest` (which must start with `{`), tracking brace depth so nested groups
/// are not truncated early. Used by [`parse_rtf_color_table`] and
/// [`parse_font_charset_table`] to isolate their destination's body before
/// parsing it -- both previously duplicated this exact brace-depth scan. ~keep
fn extract_balanced_group_body(rest: &str) -> String {
    let mut depth = 0;
    let mut table_content = String::new();
    for ch in rest.chars() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        if depth > 0 {
            table_content.push(ch);
        }
    }
    table_content
}

/// Extract the color table from RTF content.
///
/// Looks for `{\colortbl ...}` and parses semicolon-delimited color entries.
/// Each entry is formatted as `\red{R}\green{G}\blue{B};`.
fn parse_rtf_color_table(content: &str) -> Vec<String> {
    let mut colors = Vec::new();
    let Some(start) = content.find("{\\colortbl") else {
        return colors;
    };
    let table_content = extract_balanced_group_body(&content[start..]);
    let table_body = table_content.strip_prefix("{\\colortbl").unwrap_or(&table_content);

    for entry in table_body.split(';') {
        let entry = entry.trim();
        if entry.is_empty() {
            colors.push(String::new());
            continue;
        }
        let mut r = 0u8;
        let mut g = 0u8;
        let mut b = 0u8;
        for part in entry.split('\\') {
            let part = part.trim();
            if let Some(val) = part.strip_prefix("red") {
                r = val.parse().unwrap_or(0);
            } else if let Some(val) = part.strip_prefix("green") {
                g = val.parse().unwrap_or(0);
            } else if let Some(val) = part.strip_prefix("blue") {
                b = val.parse().unwrap_or(0);
            }
        }
        colors.push(format!("#{r:02x}{g:02x}{b:02x}"));
    }
    colors
}

/// Extract per-font Windows codepages from the RTF font table.
///
/// Looks for `{\fonttbl ...}` (or the ignorable-destination form `{\*\fonttbl ...}`)
/// and parses each `{\fN ... fontname;}` entry, mapping the font id to a codepage
/// derived from `\fcharsetN` (preferred) or a literal `\cpgN` fallback.
///
/// Per the RTF 1.9.1 spec, `\cpgN` on a font entry is ignored when `\fcharsetN` is
/// present — even if the fcharset value itself has no fixed codepage (e.g. Default
/// or Symbol) — so callers should fall further back to `\ansicpg` in that case, not
/// to this font's `\cpgN`. Fonts with neither `\fcharset` nor `\cpg` get no entry.
fn parse_font_charset_table(content: &str) -> HashMap<u16, u32> {
    let mut map = HashMap::new();
    let Some(start) = content.find("{\\*\\fonttbl").or_else(|| content.find("{\\fonttbl")) else {
        return map;
    };
    let table_content = extract_balanced_group_body(&content[start..]);

    let mut chars = table_content.chars().peekable();
    let mut entry_depth: i32 = 0;
    let mut current_font_id: Option<u16> = None;
    let mut current_fcharset: Option<u8> = None;
    let mut current_cpg: Option<u32> = None;

    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                entry_depth += 1;
                if entry_depth == 2 {
                    current_font_id = None;
                    current_fcharset = None;
                    current_cpg = None;
                }
            }
            '}' => {
                entry_depth -= 1;
                if entry_depth == 1
                    && let Some(id) = current_font_id
                {
                    let codepage = if current_fcharset.is_some() {
                        current_fcharset.and_then(fcharset_to_codepage)
                    } else {
                        current_cpg
                    };
                    if let Some(cp) = codepage {
                        map.insert(id, cp);
                    }
                }
            }
            '\\' => {
                if entry_depth < 2 {
                    continue;
                }
                let (word, param) = parse_rtf_control_word(&mut chars);
                match word.as_str() {
                    "f" => {
                        if let Some(val) = param {
                            current_font_id = Some(val.max(0) as u16);
                        }
                    }
                    "fcharset" => {
                        if let Some(val) = param {
                            current_fcharset = Some(val.max(0) as u8);
                        }
                    }
                    "cpg" => {
                        if let Some(val) = param
                            && val > 0
                        {
                            current_cpg = Some(val as u32);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    map
}

/// Resolve the Windows codepage used to decode a `\'hh` hex escape run.
///
/// Priority: the active font's charset (via [`parse_font_charset_table`]) —
/// preferring an explicit `\fN` in the current scope, falling back to the
/// document default font set by `\deffN` — then the active `\ansicpgNNNN`,
/// then RTF's default of 1252.
///
/// `\deffN` is document-global rather than scoped: it is typically declared
/// once, before any nested group has a chance to inherit it, so it is tracked
/// separately from `font_id_stack` rather than written into the stack itself.
#[inline]
fn resolve_decode_codepage(
    font_id_stack: &[Option<u16>],
    default_font_id: Option<u16>,
    font_charsets: &HashMap<u16, u32>,
    ansi_codepage_stack: &[u32],
) -> u32 {
    font_id_stack
        .last()
        .copied()
        .flatten()
        .or(default_font_id)
        .and_then(|id| font_charsets.get(&id).copied())
        .or_else(|| ansi_codepage_stack.last().copied())
        .unwrap_or(1252)
}

/// Close a `\fldinst` (field-instruction) destination if this `}` ends it,
/// recording a pending hyperlink URL when the instruction is a `HYPERLINK`
/// field. Shared by [`extract_rtf_formatting`] and [`extract_text_from_rtf`],
/// which both track field instructions identically. ~keep
fn close_fldinst_group(
    group_depth: i32,
    in_fldinst: &mut bool,
    fldinst_depth: i32,
    fldinst_content: &mut String,
    pending_hyperlink_url: &mut Option<String>,
) {
    if !*in_fldinst || group_depth >= fldinst_depth {
        return;
    }
    *in_fldinst = false;
    let trimmed = fldinst_content.trim();
    if let Some(rest) = trimmed.strip_prefix("HYPERLINK") {
        let url = rest.trim().trim_matches('"').trim().to_string();
        let url = if let Some(bookmark) = url.strip_prefix("\\l ") {
            format!("#{}", bookmark.trim().trim_matches('"'))
        } else if let Some(bookmark) = url.strip_prefix("\\l\"") {
            format!("#{}", bookmark.trim_matches('"'))
        } else {
            url
        };
        if !url.is_empty() {
            *pending_hyperlink_url = Some(url);
        }
    }
    fldinst_content.clear();
}

/// State needed to close a `\fldrslt` (field-result) destination.
struct FldrsltCloseState<'a> {
    in_fldrslt: &'a mut bool,
    fldrslt_depth: i32,
    fldrslt_start: usize,
    pending_hyperlink_url: &'a mut Option<String>,
    hyperlinks: &'a mut Vec<(usize, usize, String)>,
}

/// Close a `\fldrslt` destination if this `}` ends it, recording a hyperlink
/// span when a URL is pending from the sibling `\fldinst`. Shared by
/// [`extract_rtf_formatting`] and [`extract_text_from_rtf`]. ~keep
fn close_fldrslt_group(group_depth: i32, current_offset: usize, state: FldrsltCloseState) {
    if !*state.in_fldrslt || group_depth >= state.fldrslt_depth {
        return;
    }
    *state.in_fldrslt = false;
    if let Some(url) = state.pending_hyperlink_url.take() {
        state.hyperlinks.push((state.fldrslt_start, current_offset, url));
    }
}

/// Consume the next `\'hh` hex escape if it immediately follows the current one.
///
/// Adjacent hex escapes form one multi-byte run that must be decoded together
/// so multi-byte ANSI codepages (e.g. Shift-JIS, GBK) decode correctly. Raw
/// CR/LF between escapes is skipped: RTF readers ignore bare line breaks, and
/// writers wrap lines freely, including between the bytes of one character.
fn consume_adjacent_hex_escape(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<u8> {
    let mut lookahead = chars.clone();
    let mut skipped = 0usize;
    while matches!(lookahead.peek(), Some('\r' | '\n')) {
        lookahead.next();
        skipped += 1;
    }
    if lookahead.next()? != '\\' || lookahead.next()? != '\'' {
        return None;
    }
    let h1 = lookahead.next()?;
    let h2 = lookahead.next()?;
    let byte = parse_hex_byte(h1 as u8, h2 as u8)?;

    for _ in 0..skipped + 4 {
        chars.next();
    }

    Some(byte)
}
