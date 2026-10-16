//! XML extraction functions.
//!
//! Provides memory-efficient streaming XML parsing using `quick-xml`. Can handle
//! multi-GB XML files without loading them entirely into memory.
//!
//! # Features
//!
//! - **Streaming parser**: Processes XML files in constant memory
//! - **Element tracking**: Counts total elements and unique element names
//! - **Hierarchical text extraction**: Preserves document structure through indentation
//! - **Whitespace handling**: Optional whitespace preservation
//!
//! # Example
//!
//! ```ignore
//! use xberg::extraction::xml::parse_xml;
//! use xberg::extractors::security::SecurityLimits;
//!
//! # fn example() -> xberg::Result<()> {
//! let xml = b"<root><item>Hello</item><item>World</item></root>";
//! let limits = SecurityLimits::default();
//! let result = parse_xml(xml, false, &limits)?; // false = trim whitespace
//!
//! // Content preserves element hierarchy through indentation
//! assert!(result.content.contains("item\n  Hello"));
//! assert!(result.content.contains("item\n  World"));
//! assert_eq!(result.element_count, 3);
//! # Ok(())
//! # }
//! ```
use crate::error::{Result, XbergError};
use crate::extractors::security::{SecurityBudget, SecurityLimits};
use crate::types::XmlExtractionResult;
use crate::utils::xml_utils::EntityReader;
use ahash::AHashSet;
use quick_xml::events::Event;
use std::borrow::Cow;

/// SVG text-bearing elements whose text content should be extracted.
const SVG_TEXT_ELEMENTS: &[&str] = &["text", "tspan", "title", "desc", "textPath", "altGlyph"];

/// Parse XML with optional SVG mode.
///
/// In SVG mode, only text from SVG text-bearing elements (`<text>`, `<tspan>`,
/// `<title>`, `<desc>`, `<textPath>`) is extracted, without element name prefixes.
/// Attribute values are also omitted in SVG mode.
///
/// Security limits are enforced on every event loop iteration, element depth,
/// attribute values, and cumulative text output via `SecurityBudget`.
pub(crate) fn parse_xml_svg(
    xml_bytes: &[u8],
    preserve_whitespace: bool,
    limits: &SecurityLimits,
) -> Result<XmlExtractionResult> {
    parse_xml_inner(xml_bytes, preserve_whitespace, true, limits)
}

pub(crate) fn parse_xml(
    xml_bytes: &[u8],
    preserve_whitespace: bool,
    limits: &SecurityLimits,
) -> Result<XmlExtractionResult> {
    parse_xml_inner(xml_bytes, preserve_whitespace, false, limits)
}

fn parse_xml_inner(
    xml_bytes: &[u8],
    preserve_whitespace: bool,
    svg_mode: bool,
    limits: &SecurityLimits,
) -> Result<XmlExtractionResult> {
    let effective_bytes = normalize_to_utf8(xml_bytes)?;

    // No reader-level trim_text: EntityReader coalesces text fragments around
    // entity references, and trimming fragments first would corrupt spacing.
    // Text is trimmed below (when `preserve_whitespace` is off), after coalescing. ~keep
    let mut reader = EntityReader::from_bytes(&effective_bytes);
    reader.config_mut().check_end_names = false;

    let mut budget = SecurityBudget::from_limits(limits);
    let mut state = XmlParseState {
        content: String::new(),
        element_count: 0,
        unique_elements_set: AHashSet::new(),
        element_stack: Vec::new(),
        had_depth1_element: false,
    };

    loop {
        budget.step()?;
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(event) => process_xml_event(event, preserve_whitespace, svg_mode, &mut budget, &mut state)?,
            Err(e) => {
                return Err(XbergError::parsing(format!(
                    "XML parsing error at position {}: {}",
                    reader.buffer_position(),
                    e
                )));
            }
        }
    }

    let content = state.content.trim().to_string();
    let mut unique_elements: Vec<String> = state.unique_elements_set.into_iter().collect();
    unique_elements.sort();

    Ok(XmlExtractionResult {
        content,
        element_count: state.element_count,
        unique_elements,
    })
}

/// Mutable accumulator threaded through `process_xml_event` for one `parse_xml_inner` run.
struct XmlParseState {
    content: String,
    element_count: usize,
    unique_elements_set: AHashSet<String>,
    element_stack: Vec<String>,
    had_depth1_element: bool,
}

/// Apply one non-EOF `quick_xml` event to `state`. `Event::Eof` is handled by the
/// caller's loop, not here.
fn process_xml_event(
    event: Event,
    preserve_whitespace: bool,
    svg_mode: bool,
    budget: &mut SecurityBudget,
    state: &mut XmlParseState,
) -> Result<()> {
    match event {
        Event::Start(e) => {
            budget.enter()?;
            let name_owned = record_open_tag(&e, svg_mode, budget, state)?;
            state.element_stack.push(name_owned);
        }
        Event::Empty(e) => {
            record_open_tag(&e, svg_mode, budget, state)?;
        }
        Event::End(_e) => {
            budget.leave();
            state.element_stack.pop();
        }
        Event::Text(e) => {
            let text_cow: Cow<str> = std::borrow::Cow::Borrowed(e.as_ref());
            let trimmed = if preserve_whitespace {
                text_cow.to_string()
            } else {
                text_cow.trim().to_string()
            };

            if !trimmed.is_empty() {
                budget.check_entity(&trimmed)?;
                budget.account_text(trimmed.len())?;
                if svg_mode {
                    append_svg_text(&mut state.content, &trimmed, &state.element_stack);
                } else {
                    write_text_line(&mut state.content, &trimmed, state.element_stack.len());
                }
            }
        }
        Event::CData(e) => {
            let should_process = !svg_mode
                || state
                    .element_stack
                    .iter()
                    .any(|name| SVG_TEXT_ELEMENTS.contains(&name.as_str()));
            if !should_process {
                return Ok(());
            }

            let text_cow: Cow<str> = std::borrow::Cow::Borrowed(e.as_ref());
            budget.check_entity(&text_cow)?;
            budget.account_text(text_cow.len())?;
            write_text_line(&mut state.content, &text_cow, state.element_stack.len());
        }
        _ => {}
    }
    Ok(())
}

/// Normalize `xml_bytes` to UTF-8, transcoding BOM-marked UTF-16 or otherwise
/// non-UTF-8 input; already-UTF-8 input passes through unchanged.
fn normalize_to_utf8(xml_bytes: &[u8]) -> Result<Cow<'_, [u8]>> {
    if xml_bytes.len() >= 2 && xml_bytes[0] == 0xFF && xml_bytes[1] == 0xFE {
        Ok(Cow::Owned(decode_utf16_to_utf8(xml_bytes, false)?))
    } else if xml_bytes.len() >= 2 && xml_bytes[0] == 0xFE && xml_bytes[1] == 0xFF {
        Ok(Cow::Owned(decode_utf16_to_utf8(xml_bytes, true)?))
    } else if std::str::from_utf8(xml_bytes).is_err() {
        // quick-xml validates UTF-8 as it parses, so anything that is not already
        // UTF-8 has to be transcoded here or the whole document fails to read.
        // Before quick-xml 0.42 the reader handed back raw bytes and each caller
        // decoded lossily, which is the graceful degradation #395 requires: a
        // mis-encoded document must still extract, not vanish. ~keep
        Ok(Cow::Owned(
            crate::utils::decode_with_provenance(xml_bytes, None).text.into_bytes(),
        ))
    } else {
        Ok(Cow::Borrowed(xml_bytes))
    }
}

/// Record a `Start` or `Empty` tag's name/count/attributes into `state`, returning
/// the owned element name so `Start` can push it onto the element stack afterward
/// (the one difference between the two event arms).
fn record_open_tag(
    e: &quick_xml::events::BytesStart,
    svg_mode: bool,
    budget: &mut SecurityBudget,
    state: &mut XmlParseState,
) -> Result<String> {
    let name_owned = e.name().as_ref().to_string();
    state.element_count += 1;
    state.unique_elements_set.insert(name_owned.clone());

    if !svg_mode {
        write_open_tag_line(
            &mut state.content,
            e,
            &name_owned,
            state.element_stack.len(),
            budget,
            &mut state.had_depth1_element,
        )?;
    }

    Ok(name_owned)
}

/// Check attribute budget and emit an indented element-open line for a `Start` or
/// `Empty` tag. Shared by both event arms in `parse_xml_inner`, which are identical
/// apart from whether the tag is pushed onto the element stack afterward.
fn write_open_tag_line(
    content: &mut String,
    e: &quick_xml::events::BytesStart,
    name_owned: &str,
    depth: usize,
    budget: &mut SecurityBudget,
    had_depth1_element: &mut bool,
) -> Result<()> {
    for attr in e.attributes().flatten() {
        let key: Cow<str> = std::borrow::Cow::Borrowed(attr.key.as_ref());
        let val: Cow<str> = std::borrow::Cow::Borrowed(attr.value.as_ref());
        budget.check_attr(&key, &val)?;
    }
    let label = format_element_label(name_owned, e.attributes());
    write_element_line(content, &label, depth, had_depth1_element);
    Ok(())
}

/// In SVG mode, append `trimmed` to `content` when the current element stack is
/// inside an SVG text-bearing element, joining with a space where needed.
fn append_svg_text(content: &mut String, trimmed: &str, element_stack: &[String]) {
    let in_text_element = element_stack
        .iter()
        .any(|name| SVG_TEXT_ELEMENTS.contains(&name.as_str()));
    if in_text_element {
        if !content.is_empty() && !content.ends_with('\n') && !content.ends_with(' ') {
            content.push(' ');
        }
        content.push_str(trimmed);
    }
}

/// Decode UTF-16 bytes (with BOM) to UTF-8 bytes.
fn decode_utf16_to_utf8(data: &[u8], big_endian: bool) -> Result<Vec<u8>> {
    let data = &data[2..];
    let even_len = data.len() & !1;
    let data = &data[..even_len];

    let u16_iter = data.as_chunks::<2>().0.iter().map(|chunk| {
        if big_endian {
            u16::from_be_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_le_bytes([chunk[0], chunk[1]])
        }
    });

    let text: String = char::decode_utf16(u16_iter)
        .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect();

    Ok(text.into_bytes())
}

/// Build an element label with non-namespace attributes inline.
fn format_element_label(name: &str, attrs: quick_xml::events::attributes::Attributes) -> String {
    let attr_parts: Vec<String> = attrs
        .flatten()
        .filter_map(|attr| {
            let key: Cow<str> = std::borrow::Cow::Borrowed(attr.key.as_ref());
            if key.starts_with("xmlns") {
                return None;
            }
            let val: Cow<str> = std::borrow::Cow::Borrowed(attr.value.as_ref());
            let trimmed = val.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(format!("{}: {}", key, trimmed))
            }
        })
        .collect();
    if attr_parts.is_empty() {
        name.to_string()
    } else {
        format!("{} ({})", name, attr_parts.join(", "))
    }
}

/// Write an indented element label, with blank lines between depth-1 siblings.
fn write_element_line(content: &mut String, label: &str, depth: usize, had_depth1: &mut bool) {
    match depth {
        0 => {
            content.push_str(label);
            content.push('\n');
        }
        1 => {
            if *had_depth1 {
                content.push('\n');
            }
            content.push_str(label);
            content.push('\n');
            *had_depth1 = true;
        }
        _ => {
            for _ in 0..depth - 1 {
                content.push_str("  ");
            }
            content.push_str(label);
            content.push('\n');
        }
    }
}

/// Write indented text content under the current element.
fn write_text_line(content: &mut String, text: &str, stack_len: usize) {
    let indent = if stack_len == 0 {
        0
    } else {
        stack_len.saturating_sub(1).max(1)
    };
    for _ in 0..indent {
        content.push_str("  ");
    }
    content.push_str(text);
    content.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_limits() -> SecurityLimits {
        SecurityLimits::default()
    }

    #[test]
    fn test_simple_xml() {
        let xml = b"<root><item>Hello</item><item>World</item></root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert!(result.content.contains("item\n  Hello"));
        assert!(result.content.contains("item\n  World"));
        assert_eq!(result.element_count, 3);
        assert!(result.unique_elements.contains(&"root".to_string()));
        assert!(result.unique_elements.contains(&"item".to_string()));
        assert_eq!(result.unique_elements.len(), 2);
    }

    #[test]
    fn test_xml_with_cdata() {
        let xml = b"<root><![CDATA[Special <characters> & data]]></root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert!(result.content.contains("Special <characters> & data"));
        assert_eq!(result.element_count, 1);
    }

    #[test]
    fn test_malformed_xml_lenient() {
        let xml = b"<root><item>Unclosed<item2>Content</root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert!(!result.content.is_empty());
        assert!(result.content.contains("Content"));
    }

    #[test]
    fn test_empty_xml() {
        let xml = b"<root></root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert_eq!(result.content, "root");
        assert_eq!(result.element_count, 1);
        assert_eq!(result.unique_elements.len(), 1);
    }

    #[test]
    fn test_whitespace_handling() {
        let xml = b"<root>  <item>  Text  </item>  </root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert!(result.content.contains("item\n  Text"));
    }

    #[test]
    fn test_preserve_whitespace() {
        let xml = b"<root>  Text with   spaces  </root>";
        let result_trimmed = parse_xml(xml, false, &default_limits()).unwrap();
        let result_preserved = parse_xml(xml, true, &default_limits()).unwrap();
        assert!(result_trimmed.content.contains("Text with   spaces"));
        assert!(result_preserved.content.len() >= result_trimmed.content.len());
    }

    #[test]
    fn test_element_counting() {
        let xml = b"<root><a/><b/><c/><b/><d/></root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert_eq!(result.element_count, 6);
        assert_eq!(result.unique_elements.len(), 5);
        assert!(result.unique_elements.contains(&"b".to_string()));
    }

    #[test]
    fn test_xml_with_attributes() {
        let xml = br#"<root id="1"><item type="test">Content</item></root>"#;
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert!(result.content.contains("item (type: test)\n  Content"));
        assert_eq!(result.element_count, 2);
    }

    #[test]
    fn test_xml_with_namespaces() {
        let xml = b"<ns:root xmlns:ns=\"http://example.com\"><ns:item>Text</ns:item></ns:root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert!(result.content.contains("Text"));
        assert!(result.element_count >= 2);
    }

    #[test]
    fn test_xml_with_comments() {
        let xml = b"<root><!-- Comment --><item>Text</item></root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert!(result.content.contains("item\n  Text"));
        assert_eq!(result.element_count, 2);
    }

    #[test]
    fn test_xml_with_processing_instructions() {
        let xml = b"<?xml version=\"1.0\"?><root><item>Text</item></root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert!(result.content.contains("item\n  Text"));
        assert_eq!(result.element_count, 2);
    }

    #[test]
    fn test_xml_with_mixed_content() {
        let xml = b"<root>Text before<item>nested</item>Text after</root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert!(result.content.contains("Text before"));
        assert!(result.content.contains("item\n  nested"));
        assert!(result.content.contains("Text after"));
    }

    #[test]
    fn test_xml_empty_bytes() {
        let xml = b"";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert_eq!(result.content, "");
        assert_eq!(result.element_count, 0);
        assert!(result.unique_elements.is_empty());
    }

    #[test]
    fn test_xml_only_whitespace() {
        let xml = b"   \n\t  ";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert_eq!(result.content, "");
        assert_eq!(result.element_count, 0);
    }

    #[test]
    fn test_xml_with_nested_elements() {
        let xml = b"<root><parent><child><grandchild>Deep</grandchild></child></parent></root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert!(result.content.contains("    grandchild\n      Deep"));
        assert_eq!(result.element_count, 4);
        assert_eq!(result.unique_elements.len(), 4);
    }

    #[test]
    fn test_xml_with_special_characters() {
        let xml = b"<root>&lt;&gt;&amp;&quot;&apos;</root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert!(result.element_count >= 1);
    }

    #[test]
    fn test_xml_self_closing_tags() {
        let xml = b"<root><item1/><item2/><item3/></root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert_eq!(result.element_count, 4);
        assert_eq!(result.unique_elements.len(), 4);
    }

    #[test]
    fn test_xml_multiple_text_nodes() {
        let xml = b"<root>First<a/>Second<b/>Third</root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert!(result.content.contains("First"));
        assert!(result.content.contains("Second"));
        assert!(result.content.contains("Third"));
    }

    #[test]
    fn test_xml_with_newlines() {
        let xml = b"<root>\n  <item>\n    Text\n  </item>\n</root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert!(result.content.contains("item\n  Text"));
    }

    #[test]
    fn test_xml_large_cdata() {
        let large_text = "A".repeat(10000);
        let xml = format!("<root><![CDATA[{}]]></root>", large_text);
        let result = parse_xml(xml.as_bytes(), false, &default_limits()).unwrap();
        assert!(result.content.contains(&large_text));
    }

    #[test]
    fn test_xml_unique_elements_sorted() {
        let xml = b"<root><z/><a/><m/><b/></root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        let expected = vec!["a", "b", "m", "root", "z"];
        assert_eq!(result.unique_elements, expected);
    }

    #[test]
    fn test_xml_result_structure() {
        let xml = b"<root><item>Test</item></root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();

        assert!(!result.content.is_empty());
        assert!(result.element_count > 0);
        assert!(!result.unique_elements.is_empty());
    }

    #[test]
    fn test_xml_with_multiple_cdata_sections() {
        let xml = b"<root><![CDATA[First]]>Text<![CDATA[Second]]></root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert!(result.content.contains("First"));
        assert!(result.content.contains("Text"));
        assert!(result.content.contains("Second"));
    }

    #[test]
    fn test_xml_preserve_whitespace_flag() {
        let xml = b"<root>  A  B  </root>";
        let without_preserve = parse_xml(xml, false, &default_limits()).unwrap();
        let with_preserve = parse_xml(xml, true, &default_limits()).unwrap();

        assert!(!without_preserve.content.starts_with(' '));

        assert!(with_preserve.content.len() >= without_preserve.content.len());
    }

    #[test]
    fn test_xml_element_count_accuracy() {
        let xml = b"<root><a><b><c/></b></a><d/></root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert_eq!(result.element_count, 5);
    }

    #[test]
    fn test_xml_with_invalid_utf8() {
        let xml = b"<root><item>Valid text \xFF invalid</item></root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert!(result.content.contains("Valid text"));
        assert_eq!(result.element_count, 2);
    }

    #[test]
    fn test_xml_cdata_with_invalid_utf8() {
        let xml = b"<root><![CDATA[Text \xFF more text]]></root>";
        let result = parse_xml(xml, false, &default_limits()).unwrap();
        assert!(result.content.contains("Text"));
        assert!(result.content.contains("more text"));
        assert_eq!(result.element_count, 1);
    }

    #[test]
    fn test_xml_element_name_with_invalid_utf8() {
        let xml = b"<root><item\xFF>Content</item\xFF></root>";
        let result = parse_xml(xml, false, &default_limits());
        let _ = result;
    }

    #[test]
    fn test_utf16_le_xml() {
        let mut xml = vec![0xFF, 0xFE];
        for c in "<r>A</r>".encode_utf16() {
            xml.extend_from_slice(&c.to_le_bytes());
        }
        let result = parse_xml(&xml, false, &default_limits()).unwrap();
        assert!(result.content.contains("A"));
    }

    #[test]
    fn test_utf16_be_xml() {
        let mut xml = vec![0xFE, 0xFF];
        for c in "<r>B</r>".encode_utf16() {
            xml.extend_from_slice(&c.to_be_bytes());
        }
        let result = parse_xml(&xml, false, &default_limits()).unwrap();
        assert!(result.content.contains("B"));
    }

    #[test]
    fn test_utf16_odd_byte_count_truncates_gracefully() {
        let mut xml = vec![0xFF, 0xFE];
        for c in "<r>X</r>".encode_utf16() {
            xml.extend_from_slice(&c.to_le_bytes());
        }
        xml.push(0x0A);
        let result = parse_xml(&xml, false, &default_limits()).unwrap();
        assert!(result.content.contains("X"));
    }

    #[test]
    fn test_svg_script_cdata_excluded() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg">
            <script><![CDATA[ var x = 1; alert("hello"); ]]></script>
            <text>Hello</text>
            <title>My Title</title>
        </svg>"#;
        let result = parse_xml_svg(svg, false, &default_limits()).unwrap();
        assert!(
            !result.content.contains("var x"),
            "script CDATA should not appear in SVG output"
        );
        assert!(
            !result.content.contains("alert"),
            "script CDATA should not appear in SVG output"
        );
        assert!(
            result.content.contains("Hello"),
            "text element content should be included"
        );
        assert!(
            result.content.contains("My Title"),
            "title element content should be included"
        );
    }

    #[test]
    fn test_svg_style_cdata_excluded() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg">
            <style type="text/css"><![CDATA[ .cls { fill: red; } ]]></style>
            <text>Visible</text>
        </svg>"#;
        let result = parse_xml_svg(svg, false, &default_limits()).unwrap();
        assert!(
            !result.content.contains("fill"),
            "style CDATA should not appear in SVG output"
        );
        assert!(
            !result.content.contains(".cls"),
            "style CDATA should not appear in SVG output"
        );
        assert!(
            result.content.contains("Visible"),
            "text element content should be included"
        );
    }

    #[test]
    fn test_svg_text_elements_included() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg">
            <title>Chart Title</title>
            <desc>A description</desc>
            <text x="10" y="20">Label</text>
            <text><tspan>Span text</tspan></text>
            <rect width="100" height="50"/>
        </svg>"#;
        let result = parse_xml_svg(svg, false, &default_limits()).unwrap();
        assert!(result.content.contains("Chart Title"), "title text should be included");
        assert!(result.content.contains("A description"), "desc text should be included");
        assert!(
            result.content.contains("Label"),
            "text element content should be included"
        );
        assert!(result.content.contains("Span text"), "tspan content should be included");
    }

    #[test]
    fn test_svg_cdata_in_text_element_included() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg">
            <text><![CDATA[CDATA in text]]></text>
        </svg>"#;
        let result = parse_xml_svg(svg, false, &default_limits()).unwrap();
        assert!(
            result.content.contains("CDATA in text"),
            "CDATA inside text element should be included"
        );
    }

    #[test]
    fn test_utf16_factbook_file() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/vendored/unstructured/xml/factbook-utf-16.xml");
        if path.exists() {
            let xml = std::fs::read(&path).unwrap();
            let result = parse_xml(&xml, false, &default_limits()).unwrap();
            assert!(
                !result.content.is_empty(),
                "factbook-utf-16.xml should produce non-empty content"
            );
            assert!(result.content.contains("United States"));
            assert!(result.content.contains("Canada"));
        }
    }
}
