//! Utility functions for LaTeX parsing.
//!
//! This module contains helper functions for text cleaning, brace extraction,
//! and other common operations used throughout the LaTeX parser.

/// Extracts content from within braces for a given command.
///
/// Example: `\title{Hello World}` with command "title" returns "Hello World"
pub(crate) fn extract_braced(text: &str, command: &str) -> Option<String> {
    let pattern = format!("\\{}{{", command);
    if let Some(start) = text.find(&pattern) {
        let after = &text[start + pattern.len()..];
        let mut depth = 1;
        let mut content = String::new();

        for ch in after.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    content.push(ch);
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(clean_text(&content));
                    }
                    content.push(ch);
                }
                _ => content.push(ch),
            }
        }
    }
    None
}

/// Reads braced content from a character iterator.
///
/// Handles nested braces correctly and maintains proper depth tracking.
pub(crate) fn read_braced_from_chars(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<String> {
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }

    if chars.peek() != Some(&'{') {
        return None;
    }
    chars.next();

    let mut content = String::new();
    let mut depth = 1;

    for c in chars.by_ref() {
        match c {
            '{' => {
                depth += 1;
                content.push(c);
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(content);
                }
                content.push(c);
            }
            _ => content.push(c),
        }
    }

    Some(content)
}

/// Extracts environment name from a \begin{} statement.
///
/// Example: `\begin{itemize}` returns "itemize"
/// Also handles `\begin {itemize}` (with space).
pub(crate) fn extract_env_name(line: &str) -> Option<String> {
    let start = line.find("\\begin{").or_else(|| line.find("\\begin {"))?;
    let brace_pos = line[start..].find('{')?;
    let after = &line[start + brace_pos + 1..];
    let end = after.find('}')?;
    Some(after[..end].to_string())
}

/// Cleans LaTeX text by removing escape sequences.
///
/// Handles common LaTeX escape sequences like \\&, \\#, \\\_, etc.
pub(crate) fn clean_text(text: &str) -> String {
    tie_to_nbsp(&resolve_text_symbols(text))
        .replace("\\\\", "\n")
        .replace("\\&", "&")
        .replace("\\#", "#")
        .replace("\\_", "_")
        .replace("\\{", "{")
        .replace("\\}", "}")
        .replace("\\%", "%")
        .trim()
        .to_string()
}

/// The combining mark an accent command puts on its letter, as in `\'e` or `\c{c}`. ~keep
fn accent_mark(command: &str) -> Option<char> {
    Some(match command {
        "`" => '\u{300}',
        "'" => '\u{301}',
        "^" => '\u{302}',
        "~" => '\u{303}',
        "=" => '\u{304}',
        "u" => '\u{306}',
        "." => '\u{307}',
        "\"" => '\u{308}',
        "r" => '\u{30A}',
        "H" => '\u{30B}',
        "v" => '\u{30C}',
        "d" => '\u{323}',
        "c" => '\u{327}',
        "k" => '\u{328}',
        "b" => '\u{331}',
        _ => return None,
    })
}

/// Letters that LaTeX writes as a command, such as `\ss` or `\o`. ~keep
fn letter_command(command: &str) -> Option<char> {
    Some(match command {
        "ss" => 'ß',
        "o" => 'ø',
        "O" => 'Ø',
        "ae" => 'æ',
        "AE" => 'Æ',
        "oe" => 'œ',
        "OE" => 'Œ',
        "aa" => 'å',
        "AA" => 'Å',
        "l" => 'ł',
        "L" => 'Ł',
        "i" => 'ı',
        "j" => 'ȷ',
        _ => return None,
    })
}

/// Bytes of spaces and tabs at the start of `text`, which TeX skips after a command word. ~keep
fn skipped_spaces(text: &str) -> usize {
    text.len() - text.trim_start_matches([' ', '\t']).len()
}

/// The letter an accent applies to, with the bytes it takes: a letter, or `\i` / `\j` for an
/// undotted i or j, bare or alone in braces. Only a few bytes are looked at, so an unclosed
/// brace never makes the scan run to the end of the text. ~keep
fn accent_argument(text: &str) -> Option<(char, usize)> {
    let (inner, braced) = match text.strip_prefix('{') {
        Some(inner) => (inner, true),
        None => (text, false),
    };
    let dotless = [("\\i", 'i'), ("\\j", 'j')].into_iter().find(|(command, _)| {
        inner
            .strip_prefix(command)
            .is_some_and(|after| !after.starts_with(|c: char| c.is_ascii_alphabetic()))
    });
    let (base, mut len) = match dotless {
        Some((command, letter)) => (letter, command.len()),
        None => {
            let letter = inner.chars().next().filter(|c| c.is_alphabetic())?;
            (letter, letter.len_utf8())
        }
    };
    if braced {
        return inner[len..].starts_with('}').then_some((base, len + 2));
    }
    if dotless.is_some() {
        len += skipped_spaces(&inner[len..]);
    }
    Some((base, len))
}

/// The text that an accent command (`\"o`, `\'{e}`, `\c c`, `\'\i`), a letter command (`\ss`,
/// `\o`) or a control space (`\ `) at the start of `text` prints, with the bytes it takes. ~keep
fn symbol_command(text: &str) -> Option<(String, usize)> {
    let after = text.strip_prefix('\\')?;
    if after.starts_with(' ') {
        return Some((" ".to_string(), 2));
    }
    let word_len = after.len() - after.trim_start_matches(|c: char| c.is_ascii_alphabetic()).len();
    let command_len = if word_len > 0 {
        word_len
    } else {
        after.chars().next()?.len_utf8()
    };
    let command = &after[..command_len];
    let after_command = &after[command_len..];
    if let Some(mark) = accent_mark(command) {
        let skip = if word_len > 0 { skipped_spaces(after_command) } else { 0 };
        let (base, len) = accent_argument(&after_command[skip..])?;
        let printed = match unicode_normalization::char::compose(base, mark) {
            Some(composed) => composed.to_string(),
            None => [base, mark].iter().collect(),
        };
        return Some((printed, 1 + command_len + skip + len));
    }
    if word_len == 0 {
        return None;
    }
    let letter = letter_command(command)?;
    let terminator = if after_command.starts_with("{}") {
        2
    } else {
        skipped_spaces(after_command)
    };
    Some((letter.to_string(), 1 + command_len + terminator))
}

fn opening_argument_len(text: &str) -> usize {
    let spaces = text.len() - text.trim_start_matches(char::is_whitespace).len();
    if text[spaces..].starts_with('{') { spaces + 1 } else { 0 }
}

fn math_span_len(text: &str) -> Option<usize> {
    let (open, close) = [("$$", "$$"), ("$", "$"), ("\\[", "\\]"), ("\\(", "\\)")]
        .into_iter()
        .find(|(open, _)| text.starts_with(open))?;
    let mut escaped = false;
    for (index, ch) in text[open.len()..].char_indices() {
        if !escaped && text[open.len() + index..].starts_with(close) {
            return Some(open.len() + index + close.len());
        }
        escaped = ch == '\\' && !escaped;
    }
    Some(text.len())
}

/// Link targets are literal; only the following `\href` label may decode text symbols. ~keep
fn url_span_len(text: &str) -> Option<usize> {
    let command = ["\\url", "\\href"].into_iter().find(|command| {
        text.strip_prefix(command)
            .is_some_and(|after| !after.starts_with(|c: char| c.is_ascii_alphabetic()))
    })?;
    let argument = opening_argument_len(&text[command.len()..]);
    if argument == 0 {
        return None;
    }
    let start = command.len() + argument;
    let mut depth = 1;
    for (index, ch) in text[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            let end = start + index + 1;
            return Some(if command == "\\href" {
                end + opening_argument_len(&text[end..])
            } else {
                end
            });
        }
    }
    Some(text.len())
}

/// Preserve an argument's opening brace so a sole accent does not erase its command syntax. ~keep
fn unchanged_command_len(text: &str) -> usize {
    let after = &text[1..];
    let word_len = after.len() - after.trim_start_matches(|c: char| c.is_ascii_alphabetic()).len();
    let command_len = if word_len > 0 {
        word_len
    } else {
        after.chars().next().map_or(0, char::len_utf8)
    };
    let argument = if word_len > 0 {
        opening_argument_len(&after[command_len..])
    } else {
        0
    };
    1 + command_len + argument
}

/// Standalone symbol groups collapse, while command arguments, math, URLs and verbatim keep
/// the syntax their downstream parsers require. ~keep
pub(crate) fn resolve_text_symbols(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while let Some(offset) = text[index..].find(['\\', '$', '{']) {
        out.push_str(&text[index..index + offset]);
        index += offset;
        let rest = &text[index..];
        if let Some(len) = math_span_len(rest)
            .or_else(|| verb_span_len(rest))
            .or_else(|| url_span_len(rest))
        {
            out.push_str(&rest[..len]);
            index += len;
            continue;
        }
        if let Some(group) = rest.strip_prefix('{') {
            match symbol_command(group) {
                Some((printed, len)) if group[len..].starts_with('}') => {
                    out.push_str(&printed);
                    index += len + 2;
                }
                _ => {
                    out.push('{');
                    index += 1;
                }
            }
            continue;
        }
        if let Some((printed, len)) = symbol_command(rest) {
            out.push_str(&printed);
            index += len;
            continue;
        }
        let len = unchanged_command_len(rest);
        out.push_str(&rest[..len]);
        index += len;
    }
    out.push_str(&text[index..]);
    out
}

/// `\~` is an accent command, so only unescaped ties become no-break spaces. ~keep
fn tie_to_nbsp(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut escaped = false;
    let mut verbatim_end = 0;
    for (index, ch) in text.char_indices() {
        if index < verbatim_end {
            out.push(ch);
            continue;
        }
        if ch == '\\'
            && !escaped
            && let Some(len) = verb_span_len(&text[index..])
        {
            verbatim_end = index + len;
            out.push(ch);
            continue;
        }
        if ch == '~' && !escaped {
            out.push('\u{a0}');
        } else {
            out.push(ch);
        }
        escaped = ch == '\\' && !escaped;
    }
    out
}

/// Match the inline parser's `\verb` delimiter rules so its literal contents survive. ~keep
fn verb_span_len(text: &str) -> Option<usize> {
    let after_verb = text.strip_prefix("\\verb")?;
    let delim = after_verb.chars().next()?;
    if delim.is_alphabetic() || delim == '{' {
        return None;
    }
    let end = after_verb[delim.len_utf8()..].find(delim)?;
    Some("\\verb".len() + 2 * delim.len_utf8() + end)
}

/// Comment parsing state retained across physical source lines. ~keep
#[derive(Default)]
pub(super) struct CommentState {
    escaped: bool,
    url_depth: usize,
}

impl CommentState {
    /// Scan newly appended text only; rescanning the growing paragraph is quadratic. ~keep
    pub(super) fn strip_new_comment(&mut self, text: &mut String, start: usize) -> bool {
        let mut comment_at = None;
        let mut verbatim_end = 0;
        for (relative, ch) in text[start..].char_indices() {
            let index = start + relative;
            if index < verbatim_end {
                continue;
            }
            if ch == '\\'
                && !self.escaped
                && let Some(len) = verb_span_len(&text[index..])
            {
                verbatim_end = index + len;
                continue;
            }
            if self.url_depth > 0 {
                match ch {
                    '{' => self.url_depth += 1,
                    '}' => self.url_depth -= 1,
                    _ => {}
                }
                continue;
            }
            if ch == '{' && (text[..index].ends_with("\\url") || text[..index].ends_with("\\href")) {
                self.url_depth = 1;
                continue;
            }
            if ch == '%' && !self.escaped {
                comment_at = Some(index);
                break;
            }
            self.escaped = ch == '\\' && !self.escaped;
        }
        match comment_at {
            Some(index) => {
                text.truncate(index);
                true
            }
            None => false,
        }
    }
}

/// Returns true if a line inside a table-like environment (`tabular`,
/// `longtable`, `tabularx`, `tabulary`) is structural row/column markup
/// rather than literal cell content.
///
/// Covers `\hline`/booktabs rules, `longtable` page-break markers
/// (`\endhead`, `\endfirsthead`, `\endfoot`, `\endlastfoot`), `\caption{}`
/// and `\label{}` (handled separately by callers that want them), and any
/// nested `\begin{...}`/`\end{...}` boundary line.
pub(crate) fn is_table_structural_line(trimmed: &str) -> bool {
    trimmed.is_empty()
        || trimmed.starts_with("\\hline")
        || trimmed.starts_with("\\toprule")
        || trimmed.starts_with("\\midrule")
        || trimmed.starts_with("\\bottomrule")
        || trimmed.starts_with("\\endhead")
        || trimmed.starts_with("\\endfirsthead")
        || trimmed.starts_with("\\endfoot")
        || trimmed.starts_with("\\endlastfoot")
        || trimmed.starts_with("\\caption")
        || trimmed.starts_with("\\label")
        || trimmed.starts_with("\\begin{")
        || trimmed.starts_with("\\end{")
}

/// Collects content of an environment from begin to end.
///
/// Returns the content and the index of the line after \end{environment}.
/// Handles nested environments of the same type and single-line environments.
pub(crate) fn collect_environment(lines: &[&str], start_idx: usize, env_name: &str) -> (String, usize) {
    let end_marker = format!("\\end{{{}}}", env_name);
    let begin_marker = format!("\\begin{{{}}}", env_name);

    let start_line = lines[start_idx];
    if let Some(begin_pos) = start_line.find(&begin_marker) {
        let after_begin = &start_line[begin_pos + begin_marker.len()..];
        if let Some(end_pos) = after_begin.find(&end_marker) {
            let inner = &after_begin[..end_pos];
            return (inner.to_string(), start_idx + 1);
        }
    }

    let end_marker_space = format!("\\end {{{}}}", env_name);
    let begin_marker_space = format!("\\begin {{{}}}", env_name);

    let mut content = String::new();
    let mut i = start_idx + 1;
    let mut depth: isize = 1;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        depth += trimmed.matches(&begin_marker).count() as isize;
        depth += trimmed.matches(&begin_marker_space).count() as isize;
        depth -= trimmed.matches(&end_marker).count() as isize;
        depth -= trimmed.matches(&end_marker_space).count() as isize;

        if depth <= 0 {
            return (content, i + 1);
        }

        content.push_str(line);
        content.push('\n');
        i += 1;
    }

    (content, i)
}

/// Extracts a section/heading title, handling optional `[short]` arguments.
///
/// Supports `\section{title}`, `\section[short]{title}`, `\section*{title}`, etc.
pub(crate) fn extract_heading_title(line: &str, command: &str) -> Option<String> {
    let prefix = format!("\\{}", command);
    let start = line.find(&prefix)?;
    let after = &line[start + prefix.len()..];

    let rest = if after.starts_with('[') {
        let bracket_end = after.find(']')?;
        &after[bracket_end + 1..]
    } else {
        after
    };

    if !rest.starts_with('{') {
        return None;
    }

    let content = &rest[1..];
    let mut depth = 1;
    let mut result = String::new();

    for ch in content.chars() {
        match ch {
            '{' => {
                depth += 1;
                result.push(ch);
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(clean_text(&result));
                }
                result.push(ch);
            }
            _ => result.push(ch),
        }
    }
    None
}
