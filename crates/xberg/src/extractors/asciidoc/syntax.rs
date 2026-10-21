//! Pure line- and span-level AsciiDoc syntax helpers used by the block parser.

use super::ADMONITION_LABELS;
use crate::types::document_structure::AnnotationKind;
use ahash::AHashMap;

/// Minimum run length for a block delimiter line (`----`, `....`, `====`, `____`).
const MIN_DELIMITER_RUN: usize = 4;

/// Deepest section level AsciiDoc defines (`====== ` → level 6).
const MAX_SECTION_LEVEL: usize = 6;

/// Characters that may appear in an AsciiDoc table cell specifier (`2+|`, `.3+|`, `a|`).
const CELL_SPEC_CHARS: [char; 15] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '+', '*', '.', '<', '>',
];

/// Which notation a math macro or block carries.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum MathNotation {
    Latex,
    AsciiMath,
}

/// Parse an inline math macro: `latexmath:[...]`, `asciimath:[...]`, or
/// `stem:[...]`, whose notation the document's `:stem:` attribute selects.
///
/// Returns the consumed length, the macro's content, and its notation. The
/// content may hold nested brackets, so the scan tracks depth.
pub(super) fn parse_math_macro(text: &str, stem: MathNotation) -> Option<(usize, String, MathNotation)> {
    let (name, notation) = ["latexmath", "asciimath", "stem"]
        .iter()
        .find(|name| text.starts_with(&format!("{name}:[")))
        .map(|name| {
            let notation = match *name {
                "latexmath" => MathNotation::Latex,
                "asciimath" => MathNotation::AsciiMath,
                _ => stem,
            };
            (*name, notation)
        })?;

    let open = name.len() + 2;
    let mut depth = 1usize;
    for (offset, ch) in text[open..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some((open + offset + 1, text[open..open + offset].to_string(), notation));
                }
            }
            _ => {}
        }
    }
    None
}

/// Build a one-entry attribute map.
pub(super) fn single_attribute(key: &str, value: &str) -> AHashMap<String, String> {
    let mut map = AHashMap::new();
    map.insert(key.to_string(), value.to_string());
    map
}

/// `true` when `line` is a run of at least [`MIN_DELIMITER_RUN`] copies of `ch`.
pub(super) fn is_delimiter(line: &str, ch: char) -> bool {
    line.len() >= MIN_DELIMITER_RUN && line.chars().all(|c| c == ch)
}

/// Section level for a `== Heading` line: the number of leading `=` characters.
pub(super) fn section_level(line: &str) -> Option<usize> {
    let equals = line.chars().take_while(|c| *c == '=').count();
    if equals == 0 || equals > MAX_SECTION_LEVEL {
        return None;
    }
    let rest = &line[equals..];
    if rest.starts_with(' ') && !rest.trim().is_empty() {
        Some(equals)
    } else {
        None
    }
}

/// Parse a `:name: value` document attribute entry.
pub(super) fn parse_attribute_entry(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix(':')?;
    let (name, value) = rest.split_once(':')?;
    if name.is_empty() || name.contains(char::is_whitespace) {
        return None;
    }
    Some((name.trim_matches('!').to_string(), value.trim().to_string()))
}

/// Split a `[source,rust]` / `[cols="2*",options="header"]` block attribute line.
pub(super) fn parse_block_attributes(line: &str) -> Vec<String> {
    let inner = line.trim_start_matches('[').trim_end_matches(']');
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in inner.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    parts.push(current.trim().to_string());
    parts
}

/// Parse a `.Block title` line. `. item` (ordered list) and `..` are not titles.
pub(super) fn parse_block_title(line: &str) -> Option<&str> {
    let rest = line.strip_prefix('.')?;
    let first = rest.chars().next()?;
    if first.is_whitespace() || first == '.' {
        return None;
    }
    Some(rest)
}

/// Split `NOTE: body` into its label and body.
pub(super) fn split_inline_admonition(line: &str) -> Option<(&'static str, &str)> {
    ADMONITION_LABELS.iter().find_map(|label| {
        line.strip_prefix(label)
            .and_then(|rest| rest.strip_prefix(": "))
            .map(|body| (*label, body.trim()))
    })
}

/// List marker depth (1-based) and orderedness for `* `, `** `, `- `, `. `, `.. `.
pub(super) fn list_marker(line: &str) -> Option<(usize, bool)> {
    let first = line.chars().next()?;
    let ordered = match first {
        '*' => false,
        '.' => true,
        '-' => {
            return line.strip_prefix("- ").map(|_| (1, false));
        }
        _ => return None,
    };
    let depth = line.chars().take_while(|c| *c == first).count();
    let rest = &line[depth..];
    if depth > 0 && rest.starts_with(' ') && !rest.trim().is_empty() {
        Some((depth, ordered))
    } else {
        None
    }
}

/// Count declared table columns from a `cols` spec such as `">,<,^,"` or `"3*"`.
pub(super) fn count_columns(spec: &str) -> usize {
    spec.split(',')
        .map(|item| {
            let item = item.trim();
            match item.split_once('*') {
                Some((repeat, _)) => repeat.trim().parse::<usize>().unwrap_or(1),
                None => 1,
            }
        })
        .sum()
}

/// Split a `|a |b |c` table row into cells, discarding cell specifiers such as `2+`.
pub(super) fn split_table_row(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    for fragment in line.split('|').skip(1) {
        let trimmed = fragment.trim();
        let is_spec = !trimmed.is_empty()
            && trimmed.len() <= 4
            && trimmed.chars().all(|c| CELL_SPEC_CHARS.contains(&c) || c == 'a');
        if is_spec && trimmed.chars().any(|c| c == '+' || c == '*') {
            continue;
        }
        cells.push(trimmed.to_string());
    }
    cells
}

/// Parse `link:target[text]`, `https://url[text]` or a bare `https://url` at `rest`'s start.
///
/// Returns the consumed byte length, the display text and the target URL.
pub(super) fn parse_link_macro(rest: &str) -> Option<(usize, String, String)> {
    const SCHEMES: [&str; 4] = ["link:", "https://", "http://", "mailto:"];
    let scheme = SCHEMES.iter().find(|s| rest.starts_with(**s))?;
    let after_scheme = &rest[scheme.len()..];
    let target_end = after_scheme
        .find(|c: char| c.is_whitespace() || c == '[')
        .unwrap_or(after_scheme.len());
    let target = &after_scheme[..target_end];
    if target.is_empty() {
        return None;
    }
    let url = if *scheme == "link:" {
        target.to_string()
    } else {
        format!("{scheme}{target}")
    };

    let remainder = &after_scheme[target_end..];
    if let Some(inner) = remainder.strip_prefix('[')
        && let Some(close) = inner.find(']')
    {
        let display = inner[..close].trim();
        let display = if display.is_empty() {
            url.clone()
        } else {
            display.to_string()
        };
        return Some((scheme.len() + target_end + close + 2, display, url));
    }
    if *scheme == "link:" {
        // `link:` without a `[...]` target block is not a link macro.
        return None;
    }
    // Bare URL with no macro brackets: display the URL itself. Trailing sentence
    // punctuation is not part of the link.
    let url = url.trim_end_matches(['.', ',', ';', ':', ')']).to_string();
    if url.len() <= scheme.len() {
        return None;
    }
    Some((url.len(), url.clone(), url))
}

// An xref needs an opening bracket before a closing one; either bound alone permits failed suffix rescans. ~keep
pub(super) fn macro_delimiters(text: &str) -> (Option<usize>, Option<usize>, Option<usize>) {
    let bracket_close = text.rfind(']');
    let xref_open = bracket_close.and_then(|close| text[..close].rfind('['));
    (bracket_close, text.rfind(">>"), xref_open)
}

pub(super) fn can_close_macro(
    rest: &str,
    offset: usize,
    (bracket_close, reference_close, xref_open): (Option<usize>, Option<usize>, Option<usize>),
) -> bool {
    if rest.starts_with("+++") {
        return true;
    }
    let last_close = if rest.starts_with("<<") {
        reference_close
    } else if rest.starts_with("xref:") {
        xref_open
    } else {
        bracket_close
    };
    last_close.is_some_and(|close| close > offset)
}

pub(super) fn parse_inline_macro(
    rest: &str,
    at_boundary: bool,
    offset: usize,
    last_delimiters: (Option<usize>, Option<usize>, Option<usize>),
) -> Option<(usize, String)> {
    // The `pass:` and `xref:` macros open a word, so `bypass:[x]` stays literal. ~keep
    let macro_allowed = at_boundary || !(rest.starts_with("pass:") || rest.starts_with("xref:"));
    if !macro_allowed || !can_close_macro(rest, offset, last_delimiters) {
        return None;
    }
    parse_passthrough(rest)
        .map(|passthrough| (passthrough.consumed, passthrough.text))
        .or_else(|| parse_cross_reference(rest))
}

/// Parse a cross reference at `rest`'s start, `<<id>>`, `<<id,text>>` or `xref:id[text]`,
/// into the bytes consumed and the text it shows: its own text, or the target id without one.
pub(super) fn parse_cross_reference(rest: &str) -> Option<(usize, String)> {
    let (target, text, consumed) = if let Some(inner) = rest.strip_prefix("<<") {
        let (target_end, boundary) = inner.char_indices().find(|&(index, character)| {
            character == ',' || character.is_whitespace() || inner[index..].starts_with(">>")
        })?;
        if target_end == 0 || boundary.is_whitespace() {
            return None;
        }
        let close = inner.find(">>")?;
        let (target, text) = inner[..close].split_once(',').unwrap_or((&inner[..close], ""));
        (target, text, close + 4)
    } else {
        let inner = rest.strip_prefix("xref:")?;
        // The id runs up to `[` and holds no whitespace ~keep
        let open = inner.find(|c: char| c == '[' || c.is_whitespace())?;
        if open == 0 {
            return None;
        }
        let after_open = inner[open..].strip_prefix('[')?;
        let close = after_open.find(']')?;
        (
            &inner[..open],
            &after_open[..close],
            "xref:".len() + open + 1 + close + 1,
        )
    };
    if target.is_empty() || target.contains(char::is_whitespace) {
        return None;
    }
    let text = text.trim();
    Some((consumed, if text.is_empty() { target } else { text }.to_string()))
}

/// An inline passthrough at the start of some text: `pass:[text]` or `+++text+++`.
pub(super) struct Passthrough {
    /// Bytes the macro takes up, delimiters included
    pub(super) consumed: usize,
    /// The text, as written
    pub(super) text: String,
    /// Whether the macro asks for attribute references to be replaced (`pass:a[...]`)
    pub(super) substitutes_attributes: bool,
}

/// Parse an inline passthrough at `rest`'s start, `pass:[text]` or `+++text+++`, whose text
/// is kept as written.
pub(super) fn parse_passthrough(rest: &str) -> Option<Passthrough> {
    if let Some(inner) = rest.strip_prefix("+++") {
        let close = inner.find("+++")?;
        return Some(Passthrough {
            consumed: close + 6,
            text: inner[..close].to_string(),
            substitutes_attributes: false,
        });
    }
    let inner = rest.strip_prefix("pass:")?;
    // Only substitution names such as `q` or `q,a` may sit between `pass:` and `[` ~keep
    let open = inner.find(|c: char| !(c.is_ascii_alphabetic() || c == ','))?;
    let after_open = inner[open..].strip_prefix('[')?;
    let close = after_open.find(']')?;
    let subs = &inner[..open];
    Some(Passthrough {
        consumed: "pass:".len() + open + 1 + close + 1,
        text: after_open[..close].to_string(),
        substitutes_attributes: subs.split(',').any(|sub| sub == "a" || sub == "attributes"),
    })
}

/// Parse a constrained `*strong*`, `_emphasis_` or `` `mono` `` span at `rest`'s start.
pub(super) fn parse_constrained_span(rest: &str) -> Option<(usize, String, AnnotationKind)> {
    let marker = rest.chars().next()?;
    let kind = match marker {
        '*' => AnnotationKind::Bold,
        '_' => AnnotationKind::Italic,
        '`' => AnnotationKind::Code,
        _ => return None,
    };
    let after = &rest[marker.len_utf8()..];
    if after.starts_with(char::is_whitespace) {
        return None;
    }
    let close = after.find(marker)?;
    let inner = &after[..close];
    if inner.is_empty() || inner.ends_with(char::is_whitespace) {
        return None;
    }
    Some((marker.len_utf8() * 2 + close, inner.to_string(), kind))
}
