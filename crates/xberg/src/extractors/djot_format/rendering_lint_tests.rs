use super::{render_block_to_djot, render_inline_content};
use crate::types::{Attributes, BlockType, FormattedBlock, InlineElement, InlineType};

fn inline(kind: InlineType, text: &str) -> InlineElement {
    InlineElement {
        element_type: kind,
        content: text.into(),
        attributes: None,
        metadata: None,
    }
}

fn block(kind: BlockType, text: &str) -> FormattedBlock {
    FormattedBlock {
        block_type: kind,
        level: None,
        inline_content: vec![inline(InlineType::Text, text)],
        attributes: None,
        language: None,
        code: None,
        children: vec![],
    }
}

#[test]
fn should_preserve_all_inline_delimiters_and_selective_attribute_emission() {
    let kinds = [
        InlineType::Text,
        InlineType::Strong,
        InlineType::Emphasis,
        InlineType::Highlight,
        InlineType::Subscript,
        InlineType::Superscript,
        InlineType::Insert,
        InlineType::Delete,
        InlineType::Code,
        InlineType::Link,
        InlineType::Image,
        InlineType::Span,
        InlineType::Math,
        InlineType::RawInline,
        InlineType::FootnoteRef,
        InlineType::Symbol,
    ];
    let elements = kinds
        .into_iter()
        .map(|kind| {
            let mut element = inline(kind, "x");
            element.attributes = Some(Attributes {
                id: Some("id".into()),
                ..Default::default()
            });
            element.metadata = Some(std::collections::HashMap::from_iter([
                ("href".into(), "https://link".into()),
                ("src".into(), "image.png".into()),
                ("format".into(), "svg".into()),
            ]));
            element
        })
        .collect::<Vec<_>>();
    assert_eq!(elements.len(), 16);
    assert_eq!(
        render_inline_content(&elements),
        concat!(
            "x*x*{#id}_x_{#id}{=x=}{#id}~x~{#id}^x^{#id}{+x+}{#id}{-x-}{#id}",
            "`x`{#id}[x](https://link){#id}![x](image.png){#id}[x]{#id}$x$`x`{=svg}[^x]:x:"
        )
    );
}

#[test]
fn should_preserve_missing_resource_defaults_and_explicit_empty_raw_format() {
    let mut explicit_empty = inline(InlineType::RawInline, "empty");
    explicit_empty.metadata = Some(std::collections::HashMap::from_iter([("format".into(), "".into())]));
    let elements = vec![
        inline(InlineType::Link, ""),
        inline(InlineType::Image, ""),
        inline(InlineType::RawInline, "raw"),
        explicit_empty,
        inline(InlineType::FootnoteRef, ""),
        inline(InlineType::Symbol, ""),
    ];
    assert_eq!(render_inline_content(&elements), "[]()![]()`raw`{=html}`empty`{=}[^]::");
}

#[test]
fn should_preserve_quote_prefixes_and_fixed_child_indentation_inside_nested_lists() {
    let mut quote = block(BlockType::Blockquote, "x");
    quote.attributes = Some(Attributes {
        id: Some("id".into()),
        ..Default::default()
    });
    quote.children.push(block(BlockType::Paragraph, "a\nb"));
    assert_eq!(
        render_block_to_djot(&quote, 1),
        "  {#id}\n  > x\n  > a\n  > b\n  > \n\n"
    );
    let mut list = block(BlockType::OrderedList, "ignored");
    let mut item = block(BlockType::ListItem, "A");
    item.children.push(block(BlockType::Paragraph, "child"));
    list.children.push(item);
    assert_eq!(render_block_to_djot(&list, 2), "    1. A\n  child\n\n\n");
}

#[test]
fn should_distinguish_explicit_code_from_raw_inline_fallback_and_empty_code() {
    let mut code = block(BlockType::CodeBlock, "fallback");
    code.language = Some("rust".into());
    code.attributes = Some(Attributes {
        id: Some("id".into()),
        ..Default::default()
    });
    code.code = Some("a\n\nb\n".into());
    assert_eq!(
        render_block_to_djot(&code, 1),
        "  {#id}\n  ``` rust\n  a\n  \n  b\n  ```\n\n"
    );
    code.code = Some(String::new());
    assert_eq!(render_block_to_djot(&code, 1), "  {#id}\n  ``` rust\n  ```\n\n");
    code.code = None;
    assert_eq!(
        render_block_to_djot(&code, 1),
        "  {#id}\n  ``` rust\n  fallback\n  ```\n\n"
    );
}
