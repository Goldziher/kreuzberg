//! Source byte ranges for the sections `yaml_section` derives from parsed YAML and JSON.
//!
//! `serde_yaml_ng` and `serde_json::Value` carry no source positions, so the ranges are
//! recovered from the text: a small string-aware scanner for JSON and an indentation-based
//! line scan for YAML. A range runs from the start of a member's key to the end of its value.
//! Both lookups return `None` for members they cannot place (anchors, complex keys, flow-style
//! mappings), and callers fall back to the nearest enclosing member.

use ahash::AHashMap;

/// Byte range of a located member.
pub(crate) type Span = (usize, usize);

/// Spans of every object member reachable through objects only, keyed by `" > "`-joined path.
///
/// Members nested under arrays are not reported, matching the sections `yaml_section` emits.
pub(crate) fn json_member_spans(text: &str) -> AHashMap<String, Span> {
    struct Frame {
        is_object: bool,
        key: Option<String>,
        key_start: usize,
    }

    fn value_done(stack: &mut [Frame], spans: &mut AHashMap<String, Span>, end: usize) {
        let Some(top) = stack.last() else { return };
        if !top.is_object || top.key.is_none() || stack.iter().any(|frame| !frame.is_object) {
            return;
        }
        let path = stack
            .iter()
            .filter_map(|frame| frame.key.as_deref())
            .collect::<Vec<_>>()
            .join(" > ");
        let key_start = top.key_start;
        spans.entry(path).or_insert((key_start, end));
        if let Some(top) = stack.last_mut() {
            top.key = None;
        }
    }

    let bytes = text.as_bytes();
    let mut spans = AHashMap::new();
    let mut stack: Vec<Frame> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\r' | b'\n' | b',' | b':' => i += 1,
            open @ (b'{' | b'[') => {
                stack.push(Frame {
                    is_object: open == b'{',
                    key: None,
                    key_start: 0,
                });
                i += 1;
            }
            b'}' | b']' => {
                stack.pop();
                i += 1;
                value_done(&mut stack, &mut spans, i);
            }
            b'"' => {
                let end = json_string_end(bytes, i);
                match stack.last_mut() {
                    Some(frame) if frame.is_object && frame.key.is_none() => {
                        frame.key = serde_json::from_str::<String>(&text[i..end]).ok();
                        frame.key_start = i;
                    }
                    _ => value_done(&mut stack, &mut spans, end),
                }
                i = end;
            }
            _ => {
                while i < bytes.len() && !matches!(bytes[i], b',' | b'}' | b']' | b' ' | b'\t' | b'\r' | b'\n') {
                    i += 1;
                }
                value_done(&mut stack, &mut spans, i);
            }
        }
    }
    spans
}

/// Index one past the closing quote of the JSON string opening at `start`.
fn json_string_end(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return i + 1,
            _ => i += 1,
        }
    }
    bytes.len()
}

struct KeyLine {
    indent: usize,
    key: String,
    key_start: usize,
}

struct Lines<'a> {
    text: &'a str,
    /// `(line_start, line_end_excluding_terminator)` for every line.
    ranges: Vec<(usize, usize)>,
    keys: Vec<Option<KeyLine>>,
    /// Indentation of each non-blank, non-comment line.
    indents: Vec<Option<usize>>,
}

impl<'a> Lines<'a> {
    fn new(text: &'a str) -> Self {
        let mut ranges = Vec::new();
        let mut start = 0;
        for line in text.split_inclusive('\n') {
            let end = start + line.trim_end_matches(['\n', '\r']).len();
            ranges.push((start, end));
            start += line.len();
        }
        let mut keys = Vec::with_capacity(ranges.len());
        let mut indents = Vec::with_capacity(ranges.len());
        for &(line_start, line_end) in &ranges {
            let line = &text[line_start..line_end];
            let content = line.trim_start_matches([' ', '\t']);
            let indent = line.len() - content.len();
            let skipped = content.is_empty() || content.starts_with('#');
            indents.push((!skipped).then_some(indent));
            keys.push(if skipped {
                None
            } else {
                parse_key(content, line_start + indent, indent)
            });
        }
        Self {
            text,
            ranges,
            keys,
            indents,
        }
    }

    /// Exclusive end line of the block opened by the line at `index` with the given indent.
    fn block_end(&self, index: usize, indent: usize) -> usize {
        let mut end = index + 1;
        for next in index + 1..self.ranges.len() {
            let Some(next_indent) = self.indents[next] else {
                continue;
            };
            let line = &self.text[self.ranges[next].0..self.ranges[next].1];
            let is_sequence_entry = line.trim_start().starts_with("- ") || line.trim() == "-";
            if next_indent > indent || (next_indent == indent && is_sequence_entry) {
                end = next + 1;
            } else {
                break;
            }
        }
        end
    }
}

/// `key:` or a quoted `"key":` at the start of `content`, if `content` is a mapping entry.
fn parse_key(content: &str, key_start: usize, indent: usize) -> Option<KeyLine> {
    if content.starts_with("- ") || content == "-" || content.starts_with("---") || content.starts_with("...") {
        return None;
    }
    let key = match content.as_bytes().first()? {
        quote @ (b'"' | b'\'') => {
            let close = content[1..].find(*quote as char)? + 1;
            let after = content[close + 1..].trim_start_matches([' ', '\t']);
            if !(after.starts_with(':') && (after.len() == 1 || after[1..].starts_with([' ', '\t']))) {
                return None;
            }
            content[1..close].to_string()
        }
        _ => {
            let mut separator = None;
            for (i, c) in content.char_indices() {
                if c == ':' && content[i + 1..].chars().next().is_none_or(|n| n == ' ' || n == '\t') {
                    separator = Some(i);
                    break;
                }
            }
            content[..separator?].trim_end().to_string()
        }
    };
    Some(KeyLine { indent, key, key_start })
}

/// Spans of the YAML mapping entries addressed by each `" > "`-joined path in `paths`.
///
/// Returns an entry only for paths whose every segment is found as a mapping key at the
/// shallowest indentation inside its parent's block.
pub(crate) fn yaml_member_spans<'p>(
    text: &str,
    paths: impl IntoIterator<Item = &'p [String]>,
) -> AHashMap<String, Span> {
    let lines = Lines::new(text);
    let mut spans = AHashMap::new();
    for path in paths {
        if let Some(span) = locate(&lines, path) {
            spans.insert(path.join(" > "), span);
        }
    }
    spans
}

fn locate(lines: &Lines<'_>, path: &[String]) -> Option<Span> {
    let mut range = 0..lines.ranges.len();
    let mut found: Option<(usize, usize)> = None;
    for segment in path {
        let child_indent = range
            .clone()
            .filter_map(|i| lines.keys[i].as_ref().map(|key| key.indent))
            .min()?;
        let index = range.clone().find(|&i| {
            lines.keys[i]
                .as_ref()
                .is_some_and(|key| key.indent == child_indent && key.key == *segment)
        })?;
        let key = lines.keys[index].as_ref()?;
        let block_end = lines.block_end(index, key.indent);
        found = Some((index, block_end));
        range = index + 1..block_end;
    }
    let (index, block_end) = found?;
    let key = lines.keys[index].as_ref()?;
    let last_content = (index..block_end).rev().find(|&i| lines.indents[i].is_some())?;
    Some((key.key_start, lines.ranges[last_content].1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| (*part).to_string()).collect()
    }

    #[test]
    fn json_members_cover_key_through_value_at_every_object_depth() {
        let text = "{\n  \"a\": 1,\n  \"b\": {\"c\": \"x,y\", \"d\": [1, {\"e\": 2}]},\n  \"f\": null\n}";
        let spans = json_member_spans(text);
        assert_eq!(&text[spans["a"].0..spans["a"].1], "\"a\": 1");
        assert_eq!(
            &text[spans["b"].0..spans["b"].1],
            "\"b\": {\"c\": \"x,y\", \"d\": [1, {\"e\": 2}]}"
        );
        assert_eq!(&text[spans["b > c"].0..spans["b > c"].1], "\"c\": \"x,y\"");
        assert_eq!(&text[spans["b > d"].0..spans["b > d"].1], "\"d\": [1, {\"e\": 2}]");
        assert_eq!(&text[spans["f"].0..spans["f"].1], "\"f\": null");
        assert!(
            !spans.contains_key("b > d > e"),
            "members under arrays are not sections"
        );
    }

    #[test]
    fn json_keys_with_escapes_and_braces_in_strings_are_located() {
        let text = r#"{"we\"ird": "}{", "next": 1}"#;
        let spans = json_member_spans(text);
        assert_eq!(&text[spans["we\"ird"].0..spans["we\"ird"].1], r#""we\"ird": "}{""#);
        assert_eq!(&text[spans["next"].0..spans["next"].1], "\"next\": 1");
    }

    #[test]
    fn yaml_members_span_their_whole_block() {
        let text = "# header\nname: demo\nserver:\n  host: a\n  ports:\n    - 1\n    - 2\n\n  tls: true\nlist:\n- x\n- y\nlast: end\n";
        let paths = [
            path(&["name"]),
            path(&["server"]),
            path(&["server", "ports"]),
            path(&["server", "tls"]),
            path(&["list"]),
            path(&["last"]),
        ];
        let spans = yaml_member_spans(text, paths.iter().map(Vec::as_slice));
        let slice = |key: &str| &text[spans[key].0..spans[key].1];
        assert_eq!(slice("name"), "name: demo");
        assert_eq!(
            slice("server"),
            "server:\n  host: a\n  ports:\n    - 1\n    - 2\n\n  tls: true"
        );
        assert_eq!(slice("server > ports"), "ports:\n    - 1\n    - 2");
        assert_eq!(slice("server > tls"), "tls: true");
        assert_eq!(slice("list"), "list:\n- x\n- y");
        assert_eq!(slice("last"), "last: end");
    }

    #[test]
    fn yaml_quoted_keys_and_colons_in_values_are_handled() {
        let text = "\"a b\": 'x: y'\nurl: http://h:80/p\n";
        let paths = [path(&["a b"]), path(&["url"])];
        let spans = yaml_member_spans(text, paths.iter().map(Vec::as_slice));
        assert_eq!(&text[spans["a b"].0..spans["a b"].1], "\"a b\": 'x: y'");
        assert_eq!(&text[spans["url"].0..spans["url"].1], "url: http://h:80/p");
    }

    #[test]
    fn yaml_unlocatable_paths_are_omitted() {
        let text = "a: 1\n";
        let paths = [path(&["missing"]), path(&["a", "child"])];
        assert!(yaml_member_spans(text, paths.iter().map(Vec::as_slice)).is_empty());
    }
}
