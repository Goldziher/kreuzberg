//! Djot content rendering to djot markup.
//!
//! Converts DjotContent structures back to valid djot markup with full formatting preservation.

use super::attributes::render_attributes;
use crate::types::{BlockType, FormattedBlock, InlineElement, InlineType};

/// Render a single block to djot markup.
pub(crate) fn render_block_to_djot(block: &FormattedBlock, indent_level: usize) -> String {
    let mut output = String::new();
    let indent = "  ".repeat(indent_level);
    let attrs = block.attributes.as_ref().map(render_attributes).unwrap_or_default();
    match block.block_type {
        BlockType::Heading | BlockType::Paragraph => render_text_block(block, &indent, &attrs, &mut output),
        BlockType::CodeBlock => render_code_block(block, &indent, &attrs, &mut output),
        BlockType::Blockquote => render_quote_block(block, &indent, &attrs, &mut output),
        BlockType::BulletList | BlockType::OrderedList | BlockType::TaskList => {
            render_list_block(block, &indent, &attrs, &mut output)
        }
        BlockType::ListItem => {
            output.push_str(&indent);
            output.push_str(&render_inline_content(&block.inline_content));
            output.push('\n');
            render_children(block, indent_level + 1, &mut output);
        }
        BlockType::DefinitionList | BlockType::Section => {
            append_block_attributes(&indent, &attrs, &mut output);
            render_children(block, indent_level, &mut output);
            if block.block_type == BlockType::DefinitionList {
                output.push('\n');
            }
        }
        BlockType::DefinitionTerm | BlockType::DefinitionDescription => {
            output.push_str(&indent);
            if block.block_type == BlockType::DefinitionDescription {
                output.push_str(": ");
            }
            output.push_str(&render_inline_content(&block.inline_content));
            output.push('\n');
        }
        BlockType::Div => render_div_block(block, indent_level, &indent, &attrs, &mut output),
        BlockType::ThematicBreak => {
            output.push_str(&indent);
            output.push_str("---\n\n");
        }
        BlockType::RawBlock | BlockType::MathDisplay => render_raw_block(block, &indent, &mut output),
    }
    output
}

fn append_block_attributes(indent: &str, attrs: &str, output: &mut String) {
    if !attrs.is_empty() {
        output.push_str(indent);
        output.push_str(attrs);
        output.push('\n');
    }
}

fn render_children(block: &FormattedBlock, indent_level: usize, output: &mut String) {
    for child in &block.children {
        output.push_str(&render_block_to_djot(child, indent_level));
    }
}

fn render_text_block(block: &FormattedBlock, indent: &str, attrs: &str, output: &mut String) {
    output.push_str(indent);
    if block.block_type == BlockType::Heading {
        output.push_str(&"#".repeat(block.level.unwrap_or(1)));
        output.push(' ');
    }
    output.push_str(&render_inline_content(&block.inline_content));
    if !attrs.is_empty() {
        output.push(' ');
        output.push_str(attrs);
    }
    output.push_str("\n\n");
}

fn render_code_block(block: &FormattedBlock, indent: &str, attrs: &str, output: &mut String) {
    append_block_attributes(indent, attrs, output);
    output.push_str(indent);
    output.push_str("```");
    if let Some(language) = block.language.as_deref() {
        output.push(' ');
        output.push_str(language);
    }
    output.push('\n');
    if let Some(code) = block.code.as_deref() {
        for line in code.lines() {
            output.push_str(indent);
            output.push_str(line);
            output.push('\n');
        }
    } else {
        append_inline_source(block, indent, output);
    }
    output.push_str(indent);
    output.push_str("```\n\n");
}

fn append_inline_source(block: &FormattedBlock, indent: &str, output: &mut String) {
    for element in &block.inline_content {
        output.push_str(indent);
        output.push_str(&element.content);
        output.push('\n');
    }
}

fn render_quote_block(block: &FormattedBlock, indent: &str, attrs: &str, output: &mut String) {
    append_block_attributes(indent, attrs, output);
    output.push_str(indent);
    output.push_str("> ");
    output.push_str(&render_inline_content(&block.inline_content));
    output.push('\n');
    for child in &block.children {
        for line in render_block_to_djot(child, 0).lines() {
            output.push_str(indent);
            output.push_str("> ");
            output.push_str(line);
            output.push('\n');
        }
    }
    output.push('\n');
}

fn render_list_block(block: &FormattedBlock, indent: &str, attrs: &str, output: &mut String) {
    append_block_attributes(indent, attrs, output);
    for (index, child) in block.children.iter().enumerate() {
        if block.block_type == BlockType::OrderedList {
            let marker = format!("{}. ", index + 1);
            output.push_str(&render_list_item(child, indent, &marker));
        } else {
            let marker = if block.block_type == BlockType::BulletList {
                "- "
            } else {
                "- [ ] "
            };
            output.push_str(&render_list_item(child, indent, marker));
        }
    }
    output.push('\n');
}

fn render_div_block(block: &FormattedBlock, indent_level: usize, indent: &str, attrs: &str, output: &mut String) {
    output.push_str(indent);
    output.push_str(":::");
    if !attrs.is_empty() {
        output.push(' ');
        output.push_str(attrs);
    }
    output.push('\n');
    render_children(block, indent_level, output);
    output.push_str(indent);
    output.push_str(":::\n\n");
}

fn render_raw_block(block: &FormattedBlock, indent: &str, output: &mut String) {
    output.push_str(indent);
    if block.block_type == BlockType::RawBlock {
        output.push_str("```");
        if let Some(language) = block.language.as_deref() {
            output.push('=');
            output.push_str(language);
        }
    } else {
        output.push_str("$$");
    }
    output.push('\n');
    append_inline_source(block, indent, output);
    output.push_str(indent);
    output.push_str(if block.block_type == BlockType::RawBlock {
        "```\n\n"
    } else {
        "$$\n\n"
    });
}

/// Render a list item with the given marker.
pub(crate) fn render_list_item(item: &FormattedBlock, indent: &str, marker: &str) -> String {
    let mut output = String::new();
    output.push_str(indent);
    output.push_str(marker);
    output.push_str(&render_inline_content(&item.inline_content));
    output.push('\n');
    for child in &item.children {
        output.push_str(&render_block_to_djot(child, 1));
    }
    output
}

/// Render inline content to djot markup.
pub(crate) fn render_inline_content(elements: &[InlineElement]) -> String {
    let mut output = String::new();
    for element in elements {
        render_inline_element(element, &mut output);
    }
    output
}

fn render_inline_element(element: &InlineElement, output: &mut String) {
    let attrs = element.attributes.as_ref().map(render_attributes).unwrap_or_default();
    let (opening, closing) = match element.element_type {
        InlineType::Text => {
            output.push_str(&element.content);
            return;
        }
        InlineType::Link | InlineType::Image => {
            render_inline_link(element, &attrs, output);
            return;
        }
        InlineType::RawInline => {
            render_raw_inline(element, output);
            return;
        }
        InlineType::Strong => ("*", "*"),
        InlineType::Emphasis => ("_", "_"),
        InlineType::Highlight => ("{=", "=}"),
        InlineType::Subscript => ("~", "~"),
        InlineType::Superscript => ("^", "^"),
        InlineType::Insert => ("{+", "+}"),
        InlineType::Delete => ("{-", "-}"),
        InlineType::Code => ("`", "`"),
        InlineType::Span => ("[", "]"),
        InlineType::Math => ("$", "$"),
        InlineType::FootnoteRef => ("[^", "]"),
        InlineType::Symbol => (":", ":"),
    };
    output.push_str(opening);
    output.push_str(&element.content);
    output.push_str(closing);
    if !matches!(
        element.element_type,
        InlineType::Math | InlineType::FootnoteRef | InlineType::Symbol
    ) {
        output.push_str(&attrs);
    }
}

fn render_inline_link(element: &InlineElement, attrs: &str, output: &mut String) {
    let is_image = element.element_type == InlineType::Image;
    let key = if is_image { "src" } else { "href" };
    let target = element
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.get(key))
        .map(String::as_str)
        .unwrap_or("");
    output.push_str(if is_image { "![" } else { "[" });
    output.push_str(&element.content);
    output.push_str("](");
    output.push_str(target);
    output.push(')');
    output.push_str(attrs);
}

fn render_raw_inline(element: &InlineElement, output: &mut String) {
    let format = element
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.get("format"))
        .map(String::as_str)
        .unwrap_or("html");
    output.push('`');
    output.push_str(&element.content);
    output.push_str("`{=");
    output.push_str(format);
    output.push('}');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DjotContent, Metadata};

    #[test]
    fn test_render_heading() {
        let content = DjotContent {
            plain_text: "Test Heading".to_string(),
            blocks: vec![FormattedBlock {
                block_type: BlockType::Heading,
                level: Some(1),
                inline_content: vec![InlineElement {
                    element_type: InlineType::Text,
                    content: "Test Heading".to_string(),
                    attributes: None,
                    metadata: None,
                }],
                attributes: None,
                language: None,
                code: None,
                children: vec![],
            }],
            metadata: Metadata::default(),
            tables: vec![],
            images: vec![],
            links: vec![],
            footnotes: vec![],
            attributes: Default::default(),
        };

        let mut output = String::new();
        for block in &content.blocks {
            output.push_str(&render_block_to_djot(block, 0));
        }

        assert!(output.contains("# Test Heading"));
    }

    #[test]
    fn test_render_paragraph() {
        let content = DjotContent {
            plain_text: "Test paragraph".to_string(),
            blocks: vec![FormattedBlock {
                block_type: BlockType::Paragraph,
                level: None,
                inline_content: vec![InlineElement {
                    element_type: InlineType::Text,
                    content: "Test paragraph".to_string(),
                    attributes: None,
                    metadata: None,
                }],
                attributes: None,
                language: None,
                code: None,
                children: vec![],
            }],
            metadata: Metadata::default(),
            tables: vec![],
            images: vec![],
            links: vec![],
            footnotes: vec![],
            attributes: Default::default(),
        };

        let mut output = String::new();
        for block in &content.blocks {
            output.push_str(&render_block_to_djot(block, 0));
        }

        assert!(output.contains("Test paragraph"));
    }
}

#[cfg(test)]
#[path = "rendering_lint_tests.rs"]
mod lint_tests;
