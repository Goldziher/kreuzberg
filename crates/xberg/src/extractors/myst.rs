use super::frontmatter_utils::extract_frontmatter_with_warning;
use crate::Result;
use crate::extractors::security::SecurityBudget;
use serde_json::{Map, Value, json};

const ADMONITION_METADATA_PREFIX: &str = "\u{e000}xberg-myst-admonition:";
const ADMONITION_METADATA_SEPARATOR: char = '\u{1f}';
const MARKER_SUFFIX: char = '\u{e001}';
const TARGET_METADATA_PREFIX: &str = "<!--xberg-myst-target:";
const TARGET_METADATA_SUFFIX: &str = "-->";
const MAX_MYST_DIRECTIVE_DEPTH: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TextNotebookCellType {
    Markdown,
    Code,
}

impl TextNotebookCellType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Code => "code",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TextNotebookCell {
    pub(crate) cell_type: TextNotebookCellType,
    pub(crate) source: String,
    pub(crate) language: Option<String>,
    pub(crate) tags: Vec<String>,
}

impl TextNotebookCell {
    fn new(cell_type: TextNotebookCellType) -> Self {
        Self {
            cell_type,
            source: String::new(),
            language: None,
            tags: Vec::new(),
        }
    }

    #[cfg(not(feature = "notebook"))]
    pub(crate) fn metadata(&self, index: usize) -> Value {
        let mut metadata = Map::new();
        metadata.insert("index".into(), json!(index));
        metadata.insert("cell_type".into(), json!(self.cell_type.as_str()));
        if let Some(language) = &self.language {
            metadata.insert("language".into(), json!(language));
        }
        if !self.tags.is_empty() {
            metadata.insert("tags".into(), json!(self.tags));
        }
        Value::Object(metadata)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TextNotebook {
    pub(crate) metadata: Map<String, Value>,
    pub(crate) cells: Vec<TextNotebookCell>,
}

impl TextNotebook {
    #[cfg(not(feature = "notebook"))]
    pub(crate) fn cell_metadata(&self) -> Value {
        Value::Array(
            self.cells
                .iter()
                .enumerate()
                .map(|(index, cell)| cell.metadata(index))
                .collect(),
        )
    }
}

#[cfg(all(feature = "notebook", feature = "tree-sitter"))]
pub(crate) fn parse_text_notebook(content: &str, budget: &mut SecurityBudget) -> Result<Option<TextNotebook>> {
    if !might_be_jupytext_notebook(content) {
        return Ok(None);
    }
    parse_jupytext_notebook(content, budget)
}

pub(crate) fn parse_myst_text_notebook(content: &str, budget: &mut SecurityBudget) -> Result<Option<TextNotebook>> {
    if !might_be_myst_notebook(content) {
        return Ok(None);
    }
    let (yaml, remaining, _) = extract_frontmatter_with_warning(content);
    parse_myst_text_notebook_parts(yaml.as_ref(), &remaining, budget)
}

fn parse_myst_text_notebook_parts(
    yaml: Option<&yaml_serde::Value>,
    remaining: &str,
    budget: &mut SecurityBudget,
) -> Result<Option<TextNotebook>> {
    let Some(yaml) = yaml else {
        return Ok(None);
    };
    if yaml.get("kernelspec").and_then(yaml_serde::Value::as_mapping).is_none()
        || !contains_code_cell(remaining, budget)?
    {
        return Ok(None);
    }
    let Some(metadata) = yaml_to_json_map(yaml) else {
        return Ok(None);
    };
    let cells = parse_myst_cells(remaining, budget)?;
    Ok(Some(TextNotebook { metadata, cells }))
}

pub(crate) fn might_contain_myst_syntax(content: &str) -> bool {
    content.contains(":::{")
        || content.contains("```{")
        || content.contains("{ref}`")
        || content.contains("{cite}`")
        || content.contains(")=")
}

pub(crate) fn preprocess_myst(content: &str, budget: &mut SecurityBudget) -> Result<String> {
    preprocess_myst_with_depth(content, 0, budget)
}

fn preprocess_myst_with_depth(content: &str, depth: usize, budget: &mut SecurityBudget) -> Result<String> {
    if depth > MAX_MYST_DIRECTIVE_DEPTH {
        return Err(crate::XbergError::security(format!(
            "MyST directive nesting exceeds the maximum of {MAX_MYST_DIRECTIVE_DEPTH} levels"
        )));
    }
    let lines: Vec<&str> = content.lines().collect();
    let mut output = String::with_capacity(content.len());
    let mut index = 0;
    while index < lines.len() {
        budget.step()?;
        let line = lines[index];
        let trimmed = line.trim();
        if let Some(directive) = parse_colon_directive(trimmed).filter(is_supported_directive) {
            index = render_colon_directive(&lines, index, directive, depth, &mut output, budget)?;
        } else if let Some((kind, argument)) = parse_fenced_directive(trimmed).filter(is_supported_fenced_directive) {
            index = render_fenced_directive(&lines, index, kind, argument, depth, &mut output, budget)?;
        } else if is_ordinary_fence(trimmed) {
            index = copy_ordinary_fence(&lines, index, &mut output, budget)?;
        } else if let Some(target) = standalone_target(trimmed) {
            render_target_marker(target, &mut output, budget)?;
            index += 1;
        } else {
            let rendered = replace_myst_roles(line, budget)?;
            append(&mut output, &rendered, budget)?;
            append(&mut output, "\n", budget)?;
            index += 1;
        }
    }
    Ok(output)
}

#[derive(Clone, Copy)]
struct ColonDirective<'a> {
    fence_length: usize,
    kind: &'a str,
    argument: &'a str,
}

fn parse_colon_directive(line: &str) -> Option<ColonDirective<'_>> {
    let fence_length = line.bytes().take_while(|byte| *byte == b':').count();
    if fence_length < 3 {
        return None;
    }
    let header = line.get(fence_length..)?.strip_prefix('{')?;
    let (kind, argument) = header.split_once('}')?;
    Some(ColonDirective {
        fence_length,
        kind: kind.trim(),
        argument: argument.trim(),
    })
}

fn render_colon_directive(
    lines: &[&str],
    start: usize,
    directive: ColonDirective<'_>,
    depth: usize,
    output: &mut String,
    budget: &mut SecurityBudget,
) -> Result<usize> {
    let end = find_colon_directive_end(lines, start + 1, directive.fence_length, budget)?;
    let (options, body_start) = parse_directive_options(lines, start + 1, end, budget)?;
    render_directive(
        &lines[body_start..end],
        directive.kind,
        directive.argument,
        &options,
        depth,
        output,
        budget,
    )?;
    Ok(end.saturating_add(1))
}

fn find_colon_directive_end(
    lines: &[&str],
    start: usize,
    fence_length: usize,
    budget: &mut SecurityBudget,
) -> Result<usize> {
    let mut depth = 1usize;
    for (offset, line) in lines[start..].iter().enumerate() {
        budget.step()?;
        let trimmed = line.trim();
        if parse_colon_directive(trimmed).is_some_and(|directive| directive.fence_length == fence_length) {
            depth += 1;
        } else if trimmed.len() == fence_length && trimmed.bytes().all(|byte| byte == b':') {
            depth -= 1;
            if depth == 0 {
                return Ok(start + offset);
            }
        }
    }
    Ok(lines.len())
}

fn parse_fenced_directive(line: &str) -> Option<(&str, &str)> {
    let header = line.strip_prefix("```{")?;
    let (kind, argument) = header.split_once('}')?;
    Some((kind.trim(), argument.trim()))
}

fn render_fenced_directive(
    lines: &[&str],
    start: usize,
    kind: &str,
    argument: &str,
    depth: usize,
    output: &mut String,
    budget: &mut SecurityBudget,
) -> Result<usize> {
    let end = find_closing_line(lines, start + 1, "```", budget)?;
    let (options, body_start) = parse_directive_options(lines, start + 1, end, budget)?;
    render_directive(&lines[body_start..end], kind, argument, &options, depth, output, budget)?;
    Ok(end.saturating_add(1))
}

fn render_directive(
    lines: &[&str],
    kind: &str,
    argument: &str,
    options: &Map<String, Value>,
    depth: usize,
    output: &mut String,
    budget: &mut SecurityBudget,
) -> Result<()> {
    match kind {
        "math" => render_math_directive(lines, options, output, budget),
        "code-cell" | "code-block" => render_code_directive(lines, argument, options, output, budget),
        "image" | "figure" => render_image_directive(lines, kind, argument, options, depth, output, budget),
        "table" => render_table_directive(lines, argument, options, depth, output, budget),
        _ => render_admonition_directive(lines, kind, argument, options, depth, output, budget),
    }
}

fn is_supported_fenced_directive((kind, _): &(&str, &str)) -> bool {
    is_supported_directive(&ColonDirective {
        fence_length: 3,
        kind,
        argument: "",
    })
}

fn is_supported_directive(directive: &ColonDirective<'_>) -> bool {
    matches!(
        directive.kind,
        "admonition"
            | "attention"
            | "caution"
            | "danger"
            | "error"
            | "hint"
            | "important"
            | "note"
            | "seealso"
            | "tip"
            | "warning"
            | "math"
            | "code-cell"
            | "code-block"
            | "image"
            | "figure"
            | "table"
    )
}

fn parse_directive_options(
    lines: &[&str],
    start: usize,
    end: usize,
    budget: &mut SecurityBudget,
) -> Result<(Map<String, Value>, usize)> {
    let mut options = Map::new();
    let mut index = start;
    while index < end {
        budget.step()?;
        let line = lines[index].trim();
        let Some(option) = line.strip_prefix(':') else {
            break;
        };
        let Some((name, value)) = option.split_once(':') else {
            break;
        };
        let name = name.trim();
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            break;
        }
        options.insert(name.to_string(), json!(value.trim()));
        index += 1;
    }
    Ok((options, index))
}

fn render_math_directive(
    lines: &[&str],
    options: &Map<String, Value>,
    output: &mut String,
    budget: &mut SecurityBudget,
) -> Result<()> {
    if let Some(label) = option(options, "label").or_else(|| option(options, "name")) {
        render_target_marker(label, output, budget)?;
    }
    append(output, "$$", budget)?;
    append_joined_lines(output, lines, budget)?;
    append(output, "$$\n\n", budget)
}

fn render_code_directive(
    lines: &[&str],
    language: &str,
    options: &Map<String, Value>,
    output: &mut String,
    budget: &mut SecurityBudget,
) -> Result<()> {
    if let Some(caption) = option(options, "caption") {
        append(output, caption, budget)?;
        append(output, "\n\n", budget)?;
    }
    if let Some(name) = option(options, "name").or_else(|| option(options, "label")) {
        render_target_marker(name, output, budget)?;
    }
    append(output, "```", budget)?;
    append(output, language, budget)?;
    append(output, "\n", budget)?;
    for line in lines {
        budget.step()?;
        append(output, line, budget)?;
        append(output, "\n", budget)?;
    }
    append(output, "```\n\n", budget)
}

fn render_image_directive(
    lines: &[&str],
    kind: &str,
    source: &str,
    options: &Map<String, Value>,
    depth: usize,
    output: &mut String,
    budget: &mut SecurityBudget,
) -> Result<()> {
    if let Some(name) = option(options, "name") {
        render_target_marker(name, output, budget)?;
    }
    let alt = option(options, "alt").unwrap_or(source);
    append(output, "![", budget)?;
    append(output, alt, budget)?;
    append(output, "](", budget)?;
    append(output, source, budget)?;
    append(output, ")\n\n", budget)?;
    if kind == "figure" && !lines.is_empty() {
        let body = preprocess_nested(lines, depth, budget)?;
        append(output, &body, budget)?;
        append(output, "\n", budget)?;
    }
    Ok(())
}

fn render_table_directive(
    lines: &[&str],
    title: &str,
    options: &Map<String, Value>,
    depth: usize,
    output: &mut String,
    budget: &mut SecurityBudget,
) -> Result<()> {
    if !title.is_empty() {
        append(output, title, budget)?;
        append(output, "\n\n", budget)?;
    }
    if let Some(name) = option(options, "name") {
        render_target_marker(name, output, budget)?;
    }
    let body = preprocess_nested(lines, depth, budget)?;
    append(output, &body, budget)?;
    append(output, "\n", budget)
}

fn render_admonition_directive(
    lines: &[&str],
    kind: &str,
    argument: &str,
    options: &Map<String, Value>,
    depth: usize,
    output: &mut String,
    budget: &mut SecurityBudget,
) -> Result<()> {
    let semantic_kind = if kind == "admonition" { "note" } else { kind };
    let title = option(options, "title").unwrap_or(argument);
    let rendered_title = if title.is_empty() {
        admonition_display_title(semantic_kind)
    } else {
        title
    };
    if let Some(name) = option(options, "name").or_else(|| option(options, "label")) {
        render_target_marker(name, output, budget)?;
    }
    append(output, "> [!", budget)?;
    append(output, gfm_admonition_kind(semantic_kind), budget)?;
    append(output, "]\n> ", budget)?;
    append(output, ADMONITION_METADATA_PREFIX, budget)?;
    append(output, semantic_kind, budget)?;
    append_char(output, ADMONITION_METADATA_SEPARATOR, budget)?;
    if !rendered_title.contains(ADMONITION_METADATA_SEPARATOR) && !rendered_title.contains(MARKER_SUFFIX) {
        append(output, rendered_title, budget)?;
    }
    append_char(output, MARKER_SUFFIX, budget)?;
    append(output, "\n>\n", budget)?;
    let body = preprocess_nested(lines, depth, budget)?;
    for line in body.lines() {
        budget.step()?;
        append(output, "> ", budget)?;
        append(output, line, budget)?;
        append(output, "\n", budget)?;
    }
    append(output, "\n", budget)
}

fn admonition_display_title(kind: &str) -> &str {
    match kind {
        "attention" => "Attention",
        "caution" => "Caution",
        "danger" => "Danger",
        "error" => "Error",
        "hint" => "Hint",
        "important" => "Important",
        "seealso" => "See also",
        "tip" => "Tip",
        "warning" => "Warning",
        _ => "Note",
    }
}

fn gfm_admonition_kind(kind: &str) -> &str {
    match kind {
        "tip" | "hint" => "TIP",
        "important" => "IMPORTANT",
        "warning" => "WARNING",
        "caution" | "danger" | "error" => "CAUTION",
        _ => "NOTE",
    }
}

fn option<'a>(options: &'a Map<String, Value>, name: &str) -> Option<&'a str> {
    options
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

fn is_ordinary_fence(line: &str) -> bool {
    line.starts_with("```") || line.starts_with("~~~")
}

fn copy_ordinary_fence(
    lines: &[&str],
    start: usize,
    output: &mut String,
    budget: &mut SecurityBudget,
) -> Result<usize> {
    let delimiter = if lines[start].trim().starts_with("~~~") {
        "~~~"
    } else {
        "```"
    };
    let end = find_closing_line(lines, start + 1, delimiter, budget)?;
    for line in &lines[start..end.min(lines.len().saturating_sub(1)).saturating_add(1)] {
        budget.step()?;
        append(output, line, budget)?;
        append(output, "\n", budget)?;
    }
    Ok(end.saturating_add(1))
}

fn find_closing_line(lines: &[&str], start: usize, delimiter: &str, budget: &mut SecurityBudget) -> Result<usize> {
    if start >= lines.len() {
        return Ok(lines.len());
    }
    for (offset, line) in lines[start..].iter().enumerate() {
        budget.step()?;
        if line.trim() == delimiter {
            return Ok(start + offset);
        }
    }
    Ok(lines.len())
}

fn replace_myst_roles(line: &str, budget: &mut SecurityBudget) -> Result<String> {
    let mut output = String::with_capacity(line.len());
    let mut cursor = 0;
    while let Some(relative_offset) = line[cursor..].find('{') {
        budget.step()?;
        let offset = cursor + relative_offset;
        append(&mut output, &line[cursor..offset], budget)?;
        let remaining = &line[offset..];
        let (role_length, role_kind) = if remaining.starts_with("{ref}`") {
            ("{ref}`".len(), "ref")
        } else if remaining.starts_with("{cite}`") {
            ("{cite}`".len(), "cite")
        } else {
            append(&mut output, "{", budget)?;
            cursor = offset + 1;
            continue;
        };
        let role_value = &remaining[role_length..];
        let Some(end) = role_value.find('`') else {
            append(&mut output, remaining, budget)?;
            return Ok(output);
        };
        let role_value = &role_value[..end];
        let (label, target) = role_label_and_target(role_value);
        append(&mut output, "[", budget)?;
        append(&mut output, label, budget)?;
        append(&mut output, "](", budget)?;
        if role_kind == "ref" {
            append(&mut output, "#", budget)?;
        } else {
            append(&mut output, "cite:", budget)?;
        }
        append(&mut output, target, budget)?;
        append(&mut output, ")", budget)?;
        cursor = offset + role_length + end + 1;
    }
    append(&mut output, &line[cursor..], budget)?;
    Ok(output)
}

fn role_label_and_target(value: &str) -> (&str, &str) {
    let value = value.trim();
    if let Some(without_suffix) = value.strip_suffix('>')
        && let Some((label, target)) = without_suffix.rsplit_once('<')
    {
        return (label.trim(), target.trim());
    }
    (value, value)
}

fn standalone_target(line: &str) -> Option<&str> {
    let target = line.strip_prefix('(')?.strip_suffix(")=")?.trim();
    is_valid_target(target).then_some(target)
}

fn render_target_marker(target: &str, output: &mut String, budget: &mut SecurityBudget) -> Result<()> {
    if !is_valid_target(target) {
        return Ok(());
    }
    append(output, TARGET_METADATA_PREFIX, budget)?;
    append(output, target, budget)?;
    append(output, TARGET_METADATA_SUFFIX, budget)?;
    append(output, "\n\n", budget)
}

pub(crate) fn myst_target_marker(value: &str) -> Option<&str> {
    let target = value
        .trim()
        .strip_prefix(TARGET_METADATA_PREFIX)?
        .strip_suffix(TARGET_METADATA_SUFFIX)?;
    is_valid_target(target).then_some(target)
}

fn is_valid_target(target: &str) -> bool {
    !target.is_empty()
        && target
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

pub(crate) fn myst_admonition_metadata(value: &str) -> Option<(&str, Option<&str>, &str)> {
    let metadata = value.strip_prefix(ADMONITION_METADATA_PREFIX)?;
    let end = metadata.find(MARKER_SUFFIX)?;
    let (kind, title) = metadata[..end].split_once(ADMONITION_METADATA_SEPARATOR)?;
    let title = (!title.is_empty()).then_some(title);
    Some((kind, title, &metadata[end + MARKER_SUFFIX.len_utf8()..]))
}

fn might_be_myst_notebook(content: &str) -> bool {
    content.starts_with("---") && content.contains("```{code-cell}")
}

fn contains_code_cell(content: &str, budget: &mut SecurityBudget) -> Result<bool> {
    for line in content.lines() {
        budget.step()?;
        if parse_fenced_directive(line.trim()).is_some_and(|(kind, _)| kind == "code-cell") {
            return Ok(true);
        }
    }
    Ok(false)
}

fn yaml_to_json_map(yaml: &yaml_serde::Value) -> Option<Map<String, Value>> {
    serde_json::to_value(yaml).ok()?.as_object().cloned()
}

fn parse_myst_cells(content: &str, budget: &mut SecurityBudget) -> Result<Vec<TextNotebookCell>> {
    let lines: Vec<&str> = content.lines().collect();
    let mut cells = Vec::new();
    let mut markdown_start = 0;
    let mut index = 0;
    while index < lines.len() {
        budget.step()?;
        let Some((kind, language)) = parse_fenced_directive(lines[index].trim()) else {
            index += 1;
            continue;
        };
        if kind != "code-cell" {
            index += 1;
            continue;
        }
        push_markdown_cell(&lines[markdown_start..index], &mut cells, budget)?;
        let end = find_closing_line(&lines, index + 1, "```", budget)?;
        let (options, body_start) = parse_directive_options(&lines, index + 1, end, budget)?;
        let mut cell = TextNotebookCell::new(TextNotebookCellType::Code);
        cell.language = (!language.is_empty()).then(|| language.to_string());
        cell.tags = option_tags(&options, budget)?;
        cell.source = joined_lines(&lines[body_start..end], budget)?;
        cells.push(cell);
        index = end.saturating_add(1);
        markdown_start = index;
    }
    if markdown_start < lines.len() {
        push_markdown_cell(&lines[markdown_start..], &mut cells, budget)?;
    }
    Ok(cells)
}

fn push_markdown_cell(lines: &[&str], cells: &mut Vec<TextNotebookCell>, budget: &mut SecurityBudget) -> Result<()> {
    let source = joined_lines(lines, budget)?;
    if source.trim().is_empty() {
        return Ok(());
    }
    let mut cell = TextNotebookCell::new(TextNotebookCellType::Markdown);
    cell.source = source;
    cells.push(cell);
    Ok(())
}

fn option_tags(options: &Map<String, Value>, budget: &mut SecurityBudget) -> Result<Vec<String>> {
    match options.get("tags").and_then(Value::as_str) {
        Some(value) => parse_tags(value, budget),
        None => Ok(Vec::new()),
    }
}

fn parse_tags(value: &str, budget: &mut SecurityBudget) -> Result<Vec<String>> {
    let mut tags = Vec::new();
    for tag in value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|tag| tag.trim().trim_matches(['\'', '"']))
        .filter(|tag| !tag.is_empty())
    {
        budget.step()?;
        budget.account_text(tag.len())?;
        tags.push(tag.to_string());
    }
    Ok(tags)
}

#[cfg(all(feature = "notebook", feature = "tree-sitter"))]
fn might_be_jupytext_notebook(content: &str) -> bool {
    content.lines().any(|line| {
        comment_payload(line).is_some_and(|(_, payload)| {
            let payload = payload.trim_start();
            payload.starts_with("%%") || payload.contains("format_name: light")
        })
    })
}

#[cfg(all(feature = "notebook", feature = "tree-sitter"))]
fn parse_jupytext_notebook(content: &str, budget: &mut SecurityBudget) -> Result<Option<TextNotebook>> {
    let lines: Vec<&str> = content.lines().collect();
    let header = parse_commented_header(&lines, budget)?;
    let start = header.as_ref().map_or(0, |header| header.end);
    let metadata = header.as_ref().map_or_else(Map::new, |header| header.metadata.clone());
    let format = notebook_format(&metadata);
    if format == Some("light") {
        return Ok(parse_light_cells(&lines[start..], budget)?.map(|cells| TextNotebook { metadata, cells }));
    }
    Ok(parse_percent_cells(&lines[start..], budget)?.map(|cells| TextNotebook { metadata, cells }))
}

#[cfg(all(feature = "notebook", feature = "tree-sitter"))]
struct CommentedHeader {
    metadata: Map<String, Value>,
    end: usize,
}

#[cfg(all(feature = "notebook", feature = "tree-sitter"))]
fn parse_commented_header(lines: &[&str], budget: &mut SecurityBudget) -> Result<Option<CommentedHeader>> {
    let Some(start) = lines.iter().position(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    let Some((prefix, payload)) = comment_payload(lines[start]) else {
        return Ok(None);
    };
    if payload.trim() != "---" {
        return Ok(None);
    }
    let mut yaml_lines = Vec::new();
    for (offset, line) in lines[start + 1..].iter().enumerate() {
        budget.step()?;
        let Some((line_prefix, payload)) = comment_payload(line) else {
            return Ok(None);
        };
        if line_prefix != prefix {
            return Ok(None);
        }
        if payload.trim() == "---" {
            let yaml_source = joined_lines(&yaml_lines, budget)?;
            let Some(yaml) = yaml_serde::from_str::<yaml_serde::Value>(&yaml_source).ok() else {
                return Ok(None);
            };
            let Some(metadata) = yaml_to_json_map(&yaml) else {
                return Ok(None);
            };
            return Ok(Some(CommentedHeader {
                metadata,
                end: start + offset + 2,
            }));
        }
        yaml_lines.push(payload);
    }
    Ok(None)
}

#[cfg(all(feature = "notebook", feature = "tree-sitter"))]
fn comment_payload(line: &str) -> Option<(&'static str, &str)> {
    let trimmed = line.trim_start();
    for prefix in ["//", "#", "--", ";"] {
        if let Some(payload) = trimmed.strip_prefix(prefix) {
            return Some((prefix, payload.strip_prefix(' ').unwrap_or(payload)));
        }
    }
    None
}

#[cfg(all(feature = "notebook", feature = "tree-sitter"))]
fn notebook_format(metadata: &Map<String, Value>) -> Option<&str> {
    metadata
        .get("jupyter")
        .and_then(|value| value.get("jupytext"))
        .or_else(|| metadata.get("jupytext"))?
        .get("text_representation")?
        .get("format_name")?
        .as_str()
}

#[cfg(all(feature = "notebook", feature = "tree-sitter"))]
fn parse_percent_cells(lines: &[&str], budget: &mut SecurityBudget) -> Result<Option<Vec<TextNotebookCell>>> {
    let mut cells = Vec::new();
    let mut current: Option<(TextNotebookCell, &'static str)> = None;
    for line in lines {
        budget.step()?;
        if let Some((prefix, marker)) = notebook_marker(line, "%%") {
            if let Some((cell, _)) = current.take() {
                push_nonempty_cell(cell, &mut cells);
            }
            current = Some((cell_from_marker(marker, budget)?, prefix));
        } else if let Some((cell, prefix)) = current.as_mut() {
            append_cell_line(cell, line, prefix, budget)?;
        }
    }
    if let Some((cell, _)) = current {
        push_nonempty_cell(cell, &mut cells);
    }
    Ok((!cells.is_empty()).then_some(cells))
}

#[cfg(all(feature = "notebook", feature = "tree-sitter"))]
fn parse_light_cells(lines: &[&str], budget: &mut SecurityBudget) -> Result<Option<Vec<TextNotebookCell>>> {
    let mut cells = Vec::new();
    let mut current: Option<(TextNotebookCell, &'static str)> = None;
    for line in lines {
        budget.step()?;
        if let Some((prefix, marker)) = notebook_marker(line, "+") {
            if let Some((cell, _)) = current.take() {
                push_nonempty_cell(cell, &mut cells);
            }
            current = Some((cell_from_marker(marker, budget)?, prefix));
        } else if notebook_marker(line, "-").is_some() {
            if let Some((cell, _)) = current.take() {
                push_nonempty_cell(cell, &mut cells);
            }
        } else if let Some((cell, prefix)) = current.as_mut() {
            append_cell_line(cell, line, prefix, budget)?;
        }
    }
    if let Some((cell, _)) = current {
        push_nonempty_cell(cell, &mut cells);
    }
    Ok((!cells.is_empty()).then_some(cells))
}

#[cfg(all(feature = "notebook", feature = "tree-sitter"))]
fn notebook_marker<'a>(line: &'a str, marker: &str) -> Option<(&'static str, &'a str)> {
    let (prefix, payload) = comment_payload(line)?;
    let rest = payload.trim_start().strip_prefix(marker)?;
    let marker_metadata = rest.trim();
    if !marker_metadata.is_empty()
        && !marker_metadata.starts_with('[')
        && !marker_metadata.starts_with('{')
        && !marker_metadata.starts_with("tags=")
    {
        return None;
    }
    Some((prefix, marker_metadata))
}

#[cfg(all(feature = "notebook", feature = "tree-sitter"))]
fn cell_from_marker(marker: &str, budget: &mut SecurityBudget) -> Result<TextNotebookCell> {
    let cell_type = if marker.starts_with("[markdown]") {
        TextNotebookCellType::Markdown
    } else {
        TextNotebookCellType::Code
    };
    let mut cell = TextNotebookCell::new(cell_type);
    if let Some(tags) = marker.split_once("tags=").map(|(_, tags)| tags) {
        cell.tags = parse_tags(tags.trim_matches(['{', '}']), budget)?;
    }
    Ok(cell)
}

#[cfg(all(feature = "notebook", feature = "tree-sitter"))]
fn append_cell_line(cell: &mut TextNotebookCell, line: &str, prefix: &str, budget: &mut SecurityBudget) -> Result<()> {
    let rendered = if cell.cell_type == TextNotebookCellType::Markdown {
        comment_payload(line)
            .filter(|(line_prefix, _)| *line_prefix == prefix)
            .map_or(line, |(_, payload)| payload)
    } else {
        line
    };
    append(&mut cell.source, rendered, budget)?;
    append(&mut cell.source, "\n", budget)?;
    Ok(())
}

#[cfg(all(feature = "notebook", feature = "tree-sitter"))]
fn push_nonempty_cell(mut cell: TextNotebookCell, cells: &mut Vec<TextNotebookCell>) {
    let trimmed_length = cell.source.trim_end_matches('\n').len();
    cell.source.truncate(trimmed_length);
    if !cell.source.trim().is_empty() {
        cells.push(cell);
    }
}

fn preprocess_nested(lines: &[&str], depth: usize, budget: &mut SecurityBudget) -> Result<String> {
    budget.enter()?;
    let result = (|| {
        let source = joined_lines(lines, budget)?;
        preprocess_myst_with_depth(&source, depth + 1, budget)
    })();
    budget.leave();
    result
}

fn joined_lines(lines: &[&str], budget: &mut SecurityBudget) -> Result<String> {
    let mut output = String::new();
    append_joined_lines(&mut output, lines, budget)?;
    Ok(output)
}

fn append_joined_lines(output: &mut String, lines: &[&str], budget: &mut SecurityBudget) -> Result<()> {
    for (index, line) in lines.iter().enumerate() {
        budget.step()?;
        if index > 0 {
            append(output, "\n", budget)?;
        }
        append(output, line, budget)?;
    }
    Ok(())
}

fn append(output: &mut String, value: &str, budget: &mut SecurityBudget) -> Result<()> {
    budget.account_text(value.len())?;
    output.push_str(value);
    Ok(())
}

fn append_char(output: &mut String, value: char, budget: &mut SecurityBudget) -> Result<()> {
    let mut buffer = [0; 4];
    append(output, value.encode_utf8(&mut buffer), budget)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SecurityLimits;

    fn test_budget() -> SecurityBudget {
        SecurityBudget::from_limits(&SecurityLimits::default())
    }

    #[test]
    fn should_reject_generated_myst_content_beyond_growth_limit() {
        let limits = SecurityLimits {
            max_content_size: 8,
            ..Default::default()
        };
        let mut budget = SecurityBudget::from_limits(&limits);
        let error = preprocess_myst(":::{note}\nexpanded\n:::", &mut budget)
            .expect_err("rendered admonition must respect the caller's growth limit");
        assert!(matches!(error, crate::XbergError::Security { .. }));
    }

    #[test]
    fn should_preprocess_supported_myst_constructs() {
        let content = ":::{warning}\nTake care.\n:::\n\n```{math}\n:label: eq-one\nx = 1\n```\nSee {ref}`eq-one`.";
        let rendered = preprocess_myst(content, &mut test_budget()).expect("supported MyST should preprocess");
        assert_eq!(
            ["Warning", "Take care.", "x = 1", "eq-one"].map(|expected| rendered.contains(expected)),
            [true; 4]
        );
        assert_eq!(
            [":::", ":label:", "```{math}", "{ref}"].map(|syntax| rendered.contains(syntax)),
            [false; 4]
        );
    }

    #[test]
    fn should_require_strong_myst_notebook_detection() {
        let ordinary = "# Report\n\n```{code-cell} python\nprint(1)\n```";
        assert_eq!(
            parse_myst_text_notebook(ordinary, &mut test_budget()).expect("detection should succeed"),
            None
        );
    }

    #[test]
    fn should_parse_myst_notebook_cells_and_metadata() {
        let content = "---\ntitle: Report\nkernelspec:\n  language: python\n---\n# Analysis\n\n```{code-cell} python\n:tags: [parameters]\nx = 1\n```";
        let notebook = parse_myst_text_notebook(content, &mut test_budget())
            .expect("parsing should succeed")
            .expect("frontmatter and code-cell identify a MyST notebook");
        assert_eq!(notebook.metadata["title"], "Report");
        assert_eq!(notebook.cells.len(), 2);
        assert_eq!(notebook.cells[0].cell_type, TextNotebookCellType::Markdown);
        assert_eq!(notebook.cells[1].cell_type, TextNotebookCellType::Code);
        assert_eq!(notebook.cells[1].tags, ["parameters"]);
    }

    #[test]
    #[cfg(all(feature = "notebook", feature = "tree-sitter"))]
    fn should_parse_percent_cells_without_header() {
        let content = "# %% [markdown]\n# # Report\n# %% tags=[parameters]\nx = 1\n";
        let notebook = parse_text_notebook(content, &mut test_budget())
            .expect("parsing should succeed")
            .expect("percent markers identify a notebook");
        assert_eq!(notebook.cells.len(), 2);
        assert_eq!(notebook.cells[0].source, "# Report");
        assert_eq!(notebook.cells[1].tags, ["parameters"]);
    }

    #[test]
    #[cfg(all(feature = "notebook", feature = "tree-sitter"))]
    fn should_not_treat_marker_free_source_as_notebook() {
        assert_eq!(
            parse_text_notebook("# ordinary source\nx = 1\n", &mut test_budget()).expect("detection should succeed"),
            None
        );
    }

    #[test]
    #[cfg(all(feature = "notebook", feature = "tree-sitter"))]
    fn should_require_light_header() {
        let content = "# + [markdown]\n# Ordinary comment\n# -\nx = 1\n";
        assert_eq!(
            parse_text_notebook(content, &mut test_budget()).expect("detection should succeed"),
            None
        );
    }

    #[test]
    #[cfg(all(feature = "notebook", feature = "tree-sitter"))]
    fn should_parse_light_cells_with_header() {
        let content = "# ---\n# jupyter:\n#   jupytext:\n#     text_representation:\n#       format_name: light\n# kernelspec:\n#   language: python\n# ---\n# + [markdown]\n# # Report\n# -\n# +\nx = 1\n# -\n";
        let notebook = parse_text_notebook(content, &mut test_budget())
            .expect("parsing should succeed")
            .expect("light header identifies a notebook");
        assert_eq!(notebook.cells.len(), 2);
        assert_eq!(notebook.cells[0].source, "# Report");
        assert_eq!(notebook.cells[1].source, "x = 1");
        assert_eq!(notebook.metadata["kernelspec"]["language"], "python");
    }

    #[test]
    fn should_bound_deeply_nested_directive_preprocessing() {
        let deepest = MAX_MYST_DIRECTIVE_DEPTH + 16;
        let mut content = String::new();
        for fence_length in (3..deepest + 3).rev() {
            content.push_str(&format!("{}{{note}}\n", ":".repeat(fence_length)));
        }
        content.push_str("bounded body\n");
        for fence_length in 3..deepest + 3 {
            content.push_str(&format!("{}\n", ":".repeat(fence_length)));
        }
        let error = preprocess_myst(&content, &mut test_budget())
            .expect_err("directives beyond the named recursion cap must be rejected");
        assert!(matches!(error, crate::XbergError::Security { .. }));
    }

    #[test]
    fn should_allow_exact_named_directive_depth_limit() {
        let deepest = MAX_MYST_DIRECTIVE_DEPTH;
        let mut content = String::new();
        for fence_length in (3..deepest + 3).rev() {
            content.push_str(&format!("{}{{note}}\n", ":".repeat(fence_length)));
        }
        content.push_str("boundary body\n");
        for fence_length in 3..deepest + 3 {
            content.push_str(&format!("{}\n", ":".repeat(fence_length)));
        }
        let rendered =
            preprocess_myst(&content, &mut test_budget()).expect("the named depth cap itself should remain valid");
        assert!(rendered.contains("boundary body"));
    }

    #[test]
    fn should_reject_unsafe_target_marker_values() {
        let mut rendered = String::new();
        render_target_marker("unsafe--><script>", &mut rendered, &mut test_budget())
            .expect("invalid target should be ignored safely");
        assert_eq!(rendered, "");
        assert_eq!(myst_target_marker("<!--xberg-myst-target:unsafe--><script>-->"), None);
    }

    #[test]
    fn should_place_table_target_after_caption() {
        let content = "```{table} Results\n:name: table-results\n| A |\n| - |\n| 1 |\n```";
        let rendered = preprocess_myst(content, &mut test_budget()).expect("table should preprocess");
        let caption = rendered.find("Results").expect("table caption");
        let target = rendered.find(TARGET_METADATA_PREFIX).expect("table target");
        let table = rendered.find("| A |").expect("table body");
        assert!(caption < target && target < table);
    }

    #[test]
    fn should_preserve_named_code_and_admonition_targets() {
        let content =
            ":::{note}\n:name: guidance\nBody.\n:::\n\n```{code-block} rust\n:label: example\nfn main() {}\n```";
        let rendered = preprocess_myst(content, &mut test_budget()).expect("named directives should preprocess");
        let guidance = rendered
            .find("<!--xberg-myst-target:guidance-->")
            .expect("admonition target marker");
        let admonition = rendered.find("> [!NOTE]").expect("rendered admonition");
        let example = rendered
            .find("<!--xberg-myst-target:example-->")
            .expect("code target marker");
        let code = rendered.find("```rust").expect("rendered code block");
        assert!(guidance < admonition && example < code);
    }
}
