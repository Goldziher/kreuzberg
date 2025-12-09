//! RTF (Rich Text Format) extractor.
//!
//! Supports: Rich Text Format (.rtf)
//!
//! This native Rust extractor provides text extraction from RTF documents with:
//! - Character encoding support (Windows-1252 for 0x80-0x9F range)
//! - Common RTF control words (paragraph breaks, tabs, bullets, quotes, dashes)
//! - Unicode escape sequences
//! - Image metadata extraction
//! - Whitespace normalization

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::cells_to_markdown;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata, Table};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// Native Rust RTF extractor.
///
/// Extracts text content, metadata, and structure from RTF documents
pub struct RtfExtractor;

impl RtfExtractor {
    /// Create a new RTF extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for RtfExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RtfExtractor {
    fn name(&self) -> &str {
        "rtf-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Extracts content from RTF (Rich Text Format) files with native Rust parsing"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

/// Convert a hex digit character to its numeric value.
///
/// Returns None if the character is not a valid hex digit.
#[inline]
fn hex_digit_to_u8(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some((c as u8) - b'0'),
        'a'..='f' => Some((c as u8) - b'a' + 10),
        'A'..='F' => Some((c as u8) - b'A' + 10),
        _ => None,
    }
}

/// Parse a hex-encoded byte from two characters.
///
/// Returns the decoded byte if both characters are valid hex digits.
#[inline]
fn parse_hex_byte(h1: char, h2: char) -> Option<u8> {
    let high = hex_digit_to_u8(h1)?;
    let low = hex_digit_to_u8(h2)?;
    Some((high << 4) | low)
}

/// Parse an RTF control word and extract its value.
///
/// Returns a tuple of (control_word, optional_numeric_value)
fn parse_rtf_control_word(chars: &mut std::iter::Peekable<std::str::Chars>) -> (String, Option<i32>) {
    let mut word = String::new();
    let mut num_str = String::new();
    let mut is_negative = false;

    while let Some(&c) = chars.peek() {
        if c.is_alphabetic() {
            word.push(c);
            chars.next();
        } else {
            break;
        }
    }

    if let Some(&c) = chars.peek()
        && c == '-'
    {
        is_negative = true;
        chars.next();
    }

    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            num_str.push(c);
            chars.next();
        } else {
            break;
        }
    }

    let num_value = if !num_str.is_empty() {
        let val = num_str.parse::<i32>().unwrap_or(0);
        Some(if is_negative { -val } else { val })
    } else {
        None
    };

    (word, num_value)
}

/// Extract text and image metadata from RTF document.
///
/// This function extracts plain text from an RTF document by:
/// 1. Tokenizing control sequences and text
/// 2. Converting encoded characters to Unicode
/// 3. Extracting text while skipping formatting groups
/// 4. Detecting and extracting image metadata (\pict sections)
/// 5. Normalizing whitespace
fn extract_text_from_rtf(content: &str) -> (String, Vec<Table>) {
    struct TableState {
        rows: Vec<Vec<String>>,
        current_row: Vec<String>,
        current_cell: String,
        in_row: bool,
    }

    fn push_cell(state: &mut TableState) {
        let cell = state.current_cell.trim().to_string();
        state.current_row.push(cell);
        state.current_cell.clear();
    }

    fn push_row(state: &mut TableState) {
        if state.in_row || !state.current_cell.is_empty() {
            push_cell(state);
            state.in_row = false;
        }
        if !state.current_row.is_empty() {
            state.rows.push(state.current_row.clone());
            state.current_row.clear();
        }
    }

    fn finalize_table(state_opt: &mut Option<TableState>, tables: &mut Vec<Table>) {
        if let Some(mut state) = state_opt.take() {
            if state.in_row || !state.current_cell.is_empty() || !state.current_row.is_empty() {
                push_row(&mut state);
            }
            if !state.rows.is_empty() {
                let markdown = cells_to_markdown(&state.rows);
                tables.push(Table {
                    cells: state.rows,
                    markdown,
                    page_number: 1,
                });
            }
        }
    }

    let mut result = String::new();
    let mut chars = content.chars().peekable();
    let mut tables: Vec<Table> = Vec::new();
    let mut table_state: Option<TableState> = None;

    let ensure_table = |table_state: &mut Option<TableState>| {
        if table_state.is_none() {
            *table_state = Some(TableState {
                rows: Vec::new(),
                current_row: Vec::new(),
                current_cell: String::new(),
                in_row: false,
            });
        }
    };

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                if let Some(&next_ch) = chars.peek() {
                    match next_ch {
                        '\\' | '{' | '}' => {
                            chars.next();
                            result.push(next_ch);
                        }
                        '\'' => {
                            chars.next();
                            let hex1 = chars.next();
                            let hex2 = chars.next();
                            if let (Some(h1), Some(h2)) = (hex1, hex2)
                                && let Some(byte) = parse_hex_byte(h1, h2)
                            {
                                let decoded = match byte {
                                    0x80 => '\u{20AC}',
                                    0x81 => '?',
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
                                    0x8D => '?',
                                    0x8E => '\u{017D}',
                                    0x8F => '?',
                                    0x90 => '?',
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
                                    0x9D => '?',
                                    0x9E => '\u{017E}',
                                    0x9F => '\u{0178}',
                                    _ => byte as char,
                                };
                                result.push(decoded);
                                if let Some(state) = table_state.as_mut()
                                    && state.in_row
                                {
                                    state.current_cell.push(decoded);
                                }
                            }
                        }
                        'u' => {
                            chars.next();
                            let mut num_str = String::new();
                            while let Some(&c) = chars.peek() {
                                if c.is_ascii_digit() || c == '-' {
                                    num_str.push(c);
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                            if let Ok(code_num) = num_str.parse::<i32>() {
                                let code_u = if code_num < 0 {
                                    (code_num + 65536) as u32
                                } else {
                                    code_num as u32
                                };
                                if let Some(c) = char::from_u32(code_u) {
                                    result.push(c);
                                    if let Some(state) = table_state.as_mut()
                                        && state.in_row
                                    {
                                        state.current_cell.push(c);
                                    }
                                }
                            }
                        }
                        _ => {
                            let (control_word, _) = parse_rtf_control_word(&mut chars);

                            match control_word.as_str() {
                                "pict" => {
                                    let image_metadata = extract_image_metadata(&mut chars);
                                    if !image_metadata.is_empty() {
                                        result.push('!');
                                        result.push('[');
                                        result.push_str("image");
                                        result.push(']');
                                        result.push('(');
                                        result.push_str(&image_metadata);
                                        result.push(')');
                                        result.push(' ');
                                        if let Some(state) = table_state.as_mut()
                                            && state.in_row
                                        {
                                            state.current_cell.push('!');
                                            state.current_cell.push('[');
                                            state.current_cell.push_str("image");
                                            state.current_cell.push(']');
                                            state.current_cell.push('(');
                                            state.current_cell.push_str(&image_metadata);
                                            state.current_cell.push(')');
                                            state.current_cell.push(' ');
                                        }
                                    }
                                }
                                "par" => {
                                    if table_state.is_some() {
                                        finalize_table(&mut table_state, &mut tables);
                                    }
                                    if !result.is_empty() && !result.ends_with('\n') {
                                        result.push('\n');
                                        result.push('\n');
                                    }
                                }
                                "tab" => {
                                    result.push('\t');
                                    if let Some(state) = table_state.as_mut()
                                        && state.in_row
                                    {
                                        state.current_cell.push('\t');
                                    }
                                }
                                "bullet" => {
                                    result.push('•');
                                }
                                "lquote" => {
                                    result.push('\u{2018}');
                                }
                                "rquote" => {
                                    result.push('\u{2019}');
                                }
                                "ldblquote" => {
                                    result.push('\u{201C}');
                                }
                                "rdblquote" => {
                                    result.push('\u{201D}');
                                }
                                "endash" => {
                                    result.push('\u{2013}');
                                }
                                "emdash" => {
                                    result.push('\u{2014}');
                                }
                                "trowd" => {
                                    ensure_table(&mut table_state);
                                    if let Some(state) = table_state.as_mut() {
                                        if state.in_row {
                                            push_row(state);
                                        }
                                        state.in_row = true;
                                        state.current_cell.clear();
                                        state.current_row.clear();
                                    }
                                    if !result.is_empty() && !result.ends_with('\n') {
                                        result.push('\n');
                                    }
                                    if !result.ends_with('|') {
                                        result.push('|');
                                        result.push(' ');
                                    }
                                }
                                "cell" => {
                                    if !result.ends_with('|') {
                                        if !result.ends_with(' ') && !result.is_empty() {
                                            result.push(' ');
                                        }
                                        result.push('|');
                                    }
                                    if !result.ends_with(' ') {
                                        result.push(' ');
                                    }
                                }
                                "row" => {
                                    ensure_table(&mut table_state);
                                    if let Some(state) = table_state.as_mut()
                                        && (state.in_row || !state.current_cell.is_empty())
                                    {
                                        push_row(state);
                                    }
                                    if !result.ends_with('|') {
                                        result.push('|');
                                    }
                                    if !result.ends_with('\n') {
                                        result.push('\n');
                                    }
                                    if let Some(state) = table_state.as_ref()
                                        && !state.in_row
                                        && !state.rows.is_empty()
                                    {
                                        // We'll finalize once we see content outside the table
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            '{' | '}' => {
                if !result.is_empty() && !result.ends_with(' ') {
                    result.push(' ');
                }
            }
            ' ' | '\t' | '\n' | '\r' => {
                if !result.is_empty() && !result.ends_with(' ') {
                    result.push(' ');
                }
                if let Some(state) = table_state.as_mut()
                    && state.in_row
                    && !state.current_cell.ends_with(' ')
                {
                    state.current_cell.push(' ');
                }
            }
            _ => {
                if let Some(state) = table_state.as_ref()
                    && !state.in_row
                    && !state.rows.is_empty()
                {
                    finalize_table(&mut table_state, &mut tables);
                }
                result.push(ch);
                if let Some(state) = table_state.as_mut()
                    && state.in_row
                {
                    state.current_cell.push(ch);
                }
            }
        }
    }

    if table_state.is_some() {
        finalize_table(&mut table_state, &mut tables);
    }

    (normalize_whitespace(&result), tables)
}

/// Normalize whitespace in a string using a single-pass algorithm.
///
/// Collapses multiple consecutive whitespace characters into single spaces
/// and trims leading/trailing whitespace.
fn normalize_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut last_was_space = false;

    for ch in s.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(ch);
            last_was_space = false;
        }
    }

    result.trim().to_string()
}

/// Parse a `{\\creatim ...}` or `{\\revtim ...}` RTF info block into ISO 8601 format.
fn parse_rtf_datetime(segment: &str) -> Option<String> {
    let mut year: Option<i32> = None;
    let mut month: Option<i32> = None;
    let mut day: Option<i32> = None;
    let mut hour: Option<i32> = None;
    let mut minute: Option<i32> = None;

    let mut chars = segment.chars().peekable();
    while let Some(&ch) = chars.peek() {
        if ch != '\\' {
            chars.next();
            continue;
        }
        chars.next();
        let (word, value) = parse_rtf_control_word(&mut chars);
        if let Some(v) = value {
            match word.as_str() {
                "yr" => year = Some(v),
                "mo" => month = Some(v),
                "dy" => day = Some(v),
                "hr" => hour = Some(v),
                "min" => minute = Some(v),
                _ => {}
            }
        }
    }

    let year = year?;
    let month = month.unwrap_or(1).max(1) as u32;
    let day = day.unwrap_or(1).max(1) as u32;
    let hour = hour.unwrap_or(0).max(0) as u32;
    let minute = minute.unwrap_or(0).max(0) as u32;

    Some(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:00Z",
        year, month, day, hour, minute
    ))
}

/// Extract metadata from the RTF `\\info` block and augment with computed statistics.
fn extract_rtf_metadata(rtf_content: &str, extracted_text: &str) -> HashMap<String, Value> {
    let mut metadata: HashMap<String, Value> = HashMap::new();

    if let Some(start) = rtf_content.find("{\\info") {
        let slice = &rtf_content[start..];
        let mut depth = 0usize;
        let mut end_offset: Option<usize> = None;

        for (idx, ch) in slice.char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    if depth == 0 {
                        end_offset = Some(idx + 1);
                        break;
                    }
                }
                _ => {}
            }
        }

        let info_block = end_offset.map(|end| &slice[..end]).unwrap_or(slice);

        let mut segments: Vec<String> = Vec::new();
        let mut seg_depth = 0usize;
        let mut current = String::new();
        let mut in_segment = false;

        for ch in info_block.chars() {
            if ch == '{' {
                seg_depth += 1;
                if seg_depth == 2 {
                    in_segment = true;
                    current.clear();
                    continue;
                }
            } else if ch == '}' {
                if seg_depth == 2 && in_segment {
                    segments.push(current.clone());
                    in_segment = false;
                }
                seg_depth = seg_depth.saturating_sub(1);
                continue;
            }

            if in_segment {
                current.push(ch);
            }
        }

        for segment in segments {
            if !segment.starts_with('\\') {
                continue;
            }

            let cleaned_segment = if segment.starts_with("\\*\\") {
                segment.replacen("\\*\\", "\\", 1)
            } else {
                segment.clone()
            };

            let mut chars = cleaned_segment.chars().peekable();
            chars.next(); // consume the leading backslash
            let (keyword, numeric) = parse_rtf_control_word(&mut chars);
            let remaining: String = chars.collect();
            let trimmed = remaining.trim();

            match keyword.as_str() {
                "author" => {
                    if !trimmed.is_empty() {
                        let author = trimmed.to_string();
                        metadata.insert("created_by".to_string(), Value::String(author.clone()));
                        metadata.insert("authors".to_string(), Value::Array(vec![Value::String(author)]));
                    }
                }
                "operator" => {
                    if !trimmed.is_empty() {
                        metadata.insert("modified_by".to_string(), Value::String(trimmed.to_string()));
                    }
                }
                "title" => {
                    if !trimmed.is_empty() {
                        metadata.insert("title".to_string(), Value::String(trimmed.to_string()));
                    }
                }
                "subject" => {
                    if !trimmed.is_empty() {
                        metadata.insert("subject".to_string(), Value::String(trimmed.to_string()));
                    }
                }
                "generator" => {
                    if !trimmed.is_empty() {
                        metadata.insert("generator".to_string(), Value::String(trimmed.to_string()));
                    }
                }
                "creatim" => {
                    if let Some(dt) = parse_rtf_datetime(trimmed) {
                        metadata.insert("created_at".to_string(), Value::String(dt));
                    }
                }
                "revtim" => {
                    if let Some(dt) = parse_rtf_datetime(trimmed) {
                        metadata.insert("modified_at".to_string(), Value::String(dt));
                    }
                }
                "version" => {
                    if let Some(val) = numeric.or_else(|| trimmed.parse::<i32>().ok()) {
                        metadata.insert("revision".to_string(), Value::String(val.to_string()));
                    }
                }
                "nofpages" => {
                    if let Some(val) = numeric.or_else(|| trimmed.parse::<i32>().ok()) {
                        metadata.insert("page_count".to_string(), Value::Number(val.into()));
                    }
                }
                "nofwords" => {
                    if let Some(val) = numeric.or_else(|| trimmed.parse::<i32>().ok()) {
                        metadata.insert("word_count".to_string(), Value::Number(val.into()));
                    }
                }
                "nofchars" => {
                    if let Some(val) = numeric.or_else(|| trimmed.parse::<i32>().ok()) {
                        metadata.insert("character_count".to_string(), Value::Number(val.into()));
                    }
                }
                "lines" => {
                    if let Some(val) = numeric.or_else(|| trimmed.parse::<i32>().ok()) {
                        metadata.insert("line_count".to_string(), Value::Number(val.into()));
                    }
                }
                "paragraphs" => {
                    if let Some(val) = numeric.or_else(|| trimmed.parse::<i32>().ok()) {
                        metadata.insert("paragraph_count".to_string(), Value::Number(val.into()));
                    }
                }
                _ => {}
            }
        }
    }

    let cleaned_text = extracted_text.trim();
    if !cleaned_text.is_empty() {
        let word_count = cleaned_text.split_whitespace().count() as i64;
        metadata
            .entry("word_count".to_string())
            .or_insert(Value::Number(word_count.into()));

        let character_count = cleaned_text.chars().count() as i64;
        metadata
            .entry("character_count".to_string())
            .or_insert(Value::Number(character_count.into()));

        let line_count = cleaned_text.lines().count() as i64;
        metadata
            .entry("line_count".to_string())
            .or_insert(Value::Number(line_count.into()));

        let paragraph_count = cleaned_text.split("\n\n").filter(|p| !p.trim().is_empty()).count() as i64;
        metadata
            .entry("paragraph_count".to_string())
            .or_insert(Value::Number(paragraph_count.into()));
    }

    metadata
}

/// Extract image metadata from within a \pict group.
///
/// Looks for image type (jpegblip, pngblip, etc.) and dimensions.
fn extract_image_metadata(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut metadata = String::new();
    let mut image_type: Option<&str> = None;
    let mut width_goal: Option<i32> = None;
    let mut height_goal: Option<i32> = None;
    let mut depth = 0;

    while let Some(&ch) = chars.peek() {
        match ch {
            '{' => {
                depth += 1;
                chars.next();
            }
            '}' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                chars.next();
            }
            '\\' => {
                chars.next();
                let (control_word, value) = parse_rtf_control_word(chars);

                match control_word.as_str() {
                    "jpegblip" => image_type = Some("jpg"),
                    "pngblip" => image_type = Some("png"),
                    "wmetafile" => image_type = Some("wmf"),
                    "dibitmap" => image_type = Some("bmp"),
                    "picwgoal" => width_goal = value,
                    "pichgoal" => height_goal = value,
                    "bin" => break,
                    _ => {}
                }
            }
            ' ' => {
                chars.next();
            }
            _ => {
                chars.next();
            }
        }
    }

    if let Some(itype) = image_type {
        metadata.push_str("image.");
        metadata.push_str(itype);
    }

    if let Some(width) = width_goal {
        let width_inches = f64::from(width) / 1440.0;
        metadata.push_str(&format!(" width=\"{:.1}in\"", width_inches));
    }

    if let Some(height) = height_goal {
        let height_inches = f64::from(height) / 1440.0;
        metadata.push_str(&format!(" height=\"{:.1}in\"", height_inches));
    }

    if metadata.is_empty() {
        metadata.push_str("image.jpg");
    }

    metadata
}

#[async_trait]
impl DocumentExtractor for RtfExtractor {
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, _config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let rtf_content = String::from_utf8_lossy(content);

        let (extracted_text, tables) = extract_text_from_rtf(&rtf_content);
        let metadata_map = extract_rtf_metadata(&rtf_content, &extracted_text);

        Ok(ExtractionResult {
            content: extracted_text,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                additional: metadata_map,
                ..Default::default()
            },
            tables,
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/rtf", "text/rtf"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rtf_extractor_plugin_interface() {
        let extractor = RtfExtractor::new();
        assert_eq!(extractor.name(), "rtf-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert!(extractor.supported_mime_types().contains(&"application/rtf"));
        assert_eq!(extractor.priority(), 50);
    }

    #[test]
    fn test_simple_rtf_extraction() {
        let _extractor = RtfExtractor;
        let rtf_content = r#"{\rtf1 Hello World}"#;
        let (extracted, _) = extract_text_from_rtf(rtf_content);
        assert!(extracted.contains("Hello") || extracted.contains("World"));
    }
}
