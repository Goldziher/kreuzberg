//! Comprehensive tests for DocBook extractor supporting both 4.x and 5.x versions.

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::plugins::{DocumentExtractor, Plugin};
use std::path::PathBuf;

/// Helper to get absolute path to test documents
fn test_file_path(filename: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test_documents")
        .join("docbook")
        .join(filename)
}

/// DocBook 4.x extractor test helper
async fn extract_docbook4_file(filename: &str) -> kreuzberg::Result<kreuzberg::types::ExtractionResult> {
    let extractor = kreuzberg::extractors::DocbookExtractor::new();
    let path = test_file_path(filename);
    let config = ExtractionConfig::default();
    extractor.extract_file(&path, "application/docbook+xml", &config).await
}

/// DocBook 5.x extractor test helper
async fn extract_docbook5_file(filename: &str) -> kreuzberg::Result<kreuzberg::types::ExtractionResult> {
    let extractor = kreuzberg::extractors::DocbookExtractor::new();
    let path = test_file_path(filename);
    let config = ExtractionConfig::default();
    extractor.extract_file(&path, "application/docbook+xml", &config).await
}

/// Helper to extract bytes directly
async fn extract_docbook_bytes(
    content: &[u8],
    mime_type: &str,
) -> kreuzberg::Result<kreuzberg::types::ExtractionResult> {
    let extractor = kreuzberg::extractors::DocbookExtractor::new();
    let config = ExtractionConfig::default();
    extractor.extract_bytes(content, mime_type, &config).await
}

#[test]
fn test_docbook_extractor_plugin_interface() {
    let extractor = kreuzberg::extractors::DocbookExtractor::new();
    assert_eq!(extractor.name(), "docbook-extractor");
    assert!(extractor.initialize().is_ok());
    assert!(extractor.shutdown().is_ok());
}

#[test]
fn test_docbook_extractor_supported_mime_types() {
    let extractor = kreuzberg::extractors::DocbookExtractor::new();
    let mime_types = extractor.supported_mime_types();
    assert!(mime_types.contains(&"application/docbook+xml"));
    assert!(mime_types.contains(&"text/docbook"));
}

#[test]
fn test_docbook_extractor_priority() {
    let extractor = kreuzberg::extractors::DocbookExtractor::new();
    assert_eq!(extractor.priority(), 50);
}

#[tokio::test]
async fn test_docbook4_chapter_extraction() {
    let result = extract_docbook4_file("docbook-chapter.docbook").await;
    assert!(result.is_ok(), "Failed to extract DocBook 4 chapter");

    let result = result.unwrap();
    assert!(!result.content.is_empty(), "Content should not be empty");
    assert!(
        result.content.contains("Test Chapter"),
        "Content should contain chapter title"
    );
    assert!(
        result.content.contains("Like a Sect1"),
        "Content should contain section titles"
    );
}

#[tokio::test]
async fn test_docbook5_reader_extraction() {
    let result = extract_docbook5_file("docbook-reader.docbook").await;
    assert!(result.is_ok(), "Failed to extract DocBook 5 file");

    let result = result.unwrap();
    assert!(!result.content.is_empty(), "Content should not be empty");
    assert!(
        result.content.contains("Pandoc Test Suite"),
        "Content should contain article title"
    );
}

#[tokio::test]
async fn test_docbook_xref_extraction() {
    let result = extract_docbook4_file("docbook-xref.docbook").await;
    assert!(result.is_ok(), "Failed to extract DocBook with xref elements");

    let result = result.unwrap();
    assert!(!result.content.is_empty(), "Content should not be empty");
    assert!(
        result.content.contains("An Example Book"),
        "Content should contain book title"
    );
    assert!(
        result.content.contains("XRef Samples"),
        "Content should contain xref chapter"
    );
}

#[tokio::test]
async fn test_docbook_tables_extraction() {
    let result = extract_docbook4_file("tables.docbook4").await;
    assert!(result.is_ok(), "Failed to extract DocBook with tables");

    let result = result.unwrap();
    assert!(!result.content.is_empty(), "Content should not be empty");
    assert!(!result.tables.is_empty(), "Should extract tables from DocBook");
}

#[tokio::test]
async fn test_docbook5_tables_extraction() {
    let result = extract_docbook5_file("tables.docbook5").await;
    assert!(result.is_ok(), "Failed to extract DocBook 5 with tables");

    let result = result.unwrap();
    assert!(!result.content.is_empty(), "Content should not be empty");
    assert!(!result.tables.is_empty(), "Should extract tables from DocBook 5");
}

#[tokio::test]
async fn test_docbook_metadata_extraction() {
    let result = extract_docbook5_file("docbook-reader.docbook").await;
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(!result.content.is_empty());
}

#[tokio::test]
async fn test_docbook_section_hierarchy() {
    let result = extract_docbook4_file("docbook-chapter.docbook").await;
    assert!(result.is_ok());

    let result = result.unwrap();
    let content = &result.content;

    assert!(content.contains("Like a Sect1"));
    assert!(content.contains("Like a Sect2"));
    assert!(content.contains("Like a Sect3"));
    assert!(content.contains("Like a Sect4"));
}

#[tokio::test]
async fn test_docbook_paragraph_extraction() {
    let result = extract_docbook4_file("docbook-chapter.docbook").await;
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(
        result.content.contains("This chapter uses recursive sections"),
        "Should extract paragraph content"
    );
}

#[tokio::test]
async fn test_docbook_paragraph_content() {
    let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE article PUBLIC "-//OASIS//DTD DocBook XML V4.4//EN"
"http://www.oasis-open.org/docbook/xml/4.4/docbookx.dtd">
<article>
  <title>Test Article</title>
  <para>This is a test paragraph.</para>
  <para>This is another paragraph with <emphasis>emphasized</emphasis> text.</para>
</article>"#;

    let result = extract_docbook_bytes(docbook.as_bytes(), "application/docbook+xml").await;
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.content.contains("Test Article"));
    assert!(result.content.contains("This is a test paragraph"));
    assert!(result.content.contains("another paragraph"));
}

#[tokio::test]
async fn test_docbook_code_block_extraction() {
    let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE article PUBLIC "-//OASIS//DTD DocBook XML V4.4//EN"
"http://www.oasis-open.org/docbook/xml/4.4/docbookx.dtd">
<article>
  <para>Here is code:</para>
  <programlisting>
def hello():
    print("world")
  </programlisting>
</article>"#;

    let result = extract_docbook_bytes(docbook.as_bytes(), "application/docbook+xml").await;
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.content.contains("def hello"));
    assert!(result.content.contains("print"));
}

#[tokio::test]
async fn test_docbook_mixed_content() {
    let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE book PUBLIC "-//OASIS//DTD DocBook XML V4.4//EN"
"http://www.oasis-open.org/docbook/xml/4.4/docbookx.dtd">
<book>
  <title>Test Book</title>
  <chapter>
    <title>Chapter 1</title>
    <section>
      <title>Section 1.1</title>
      <para>Paragraph in section.</para>
    </section>
  </chapter>
</book>"#;

    let result = extract_docbook_bytes(docbook.as_bytes(), "application/docbook+xml").await;
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.content.contains("Test Book"));
    assert!(result.content.contains("Chapter 1"));
    assert!(result.content.contains("Section 1.1"));
    assert!(result.content.contains("Paragraph in section"));
}

#[tokio::test]
async fn test_docbook_namespaced_5x_parsing() {
    let docbook5 = r#"<?xml version="1.0" encoding="UTF-8"?>
<article xmlns="http://docbook.org/ns/docbook">
  <info>
    <title>DocBook 5 Article</title>
    <author>
      <personname>
        <firstname>John</firstname>
        <surname>Doe</surname>
      </personname>
    </author>
    <date>2024-01-01</date>
  </info>
  <section>
    <title>Introduction</title>
    <para>Welcome to DocBook 5.</para>
  </section>
</article>"#;

    let result = extract_docbook_bytes(docbook5.as_bytes(), "application/docbook+xml").await;
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.content.contains("DocBook 5 Article"));
    assert!(result.content.contains("Welcome to DocBook 5"));
}

#[tokio::test]
async fn test_docbook_link_handling() {
    let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE article PUBLIC "-//OASIS//DTD DocBook XML V4.4//EN"
"http://www.oasis-open.org/docbook/xml/4.4/docbookx.dtd">
<article>
  <title>Links Test</title>
  <para>See <link xlink:href="http://example.com">example site</link>.</para>
</article>"#;

    let result = extract_docbook_bytes(docbook.as_bytes(), "application/docbook+xml").await;
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.content.contains("example"));
}

#[tokio::test]
async fn test_docbook_mime_type_detection() {
    let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE article PUBLIC "-//OASIS//DTD DocBook XML V4.4//EN"
"http://www.oasis-open.org/docbook/xml/4.4/docbookx.dtd">
<article>
  <title>Test</title>
</article>"#;

    let result1 = extract_docbook_bytes(docbook.as_bytes(), "application/docbook+xml").await;
    assert!(result1.is_ok());

    let result2 = extract_docbook_bytes(docbook.as_bytes(), "text/docbook").await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_docbook_empty_sections() {
    let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE article PUBLIC "-//OASIS//DTD DocBook XML V4.4//EN"
"http://www.oasis-open.org/docbook/xml/4.4/docbookx.dtd">
<article>
  <title>Empty Sections</title>
  <section>
    <title>Empty Section</title>
  </section>
  <section>
    <title>Section with Content</title>
    <para>Content here</para>
  </section>
</article>"#;

    let result = extract_docbook_bytes(docbook.as_bytes(), "application/docbook+xml").await;
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.content.contains("Empty Section"));
    assert!(result.content.contains("Section with Content"));
    assert!(result.content.contains("Content here"));
}

#[tokio::test]
async fn test_docbook_itemized_list() {
    let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE article PUBLIC "-//OASIS//DTD DocBook XML V4.4//EN"
"http://www.oasis-open.org/docbook/xml/4.4/docbookx.dtd">
<article>
  <title>List Test</title>
  <itemizedlist>
    <listitem>
      <para>First item</para>
    </listitem>
    <listitem>
      <para>Second item</para>
    </listitem>
    <listitem>
      <para>Third item</para>
    </listitem>
  </itemizedlist>
</article>"#;

    let result = extract_docbook_bytes(docbook.as_bytes(), "application/docbook+xml").await;
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.content.contains("First item"));
    assert!(result.content.contains("Second item"));
    assert!(result.content.contains("Third item"));
    assert!(result.content.contains("- "), "Should contain bullet points");
}

#[tokio::test]
async fn test_docbook_ordered_list() {
    let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE article PUBLIC "-//OASIS//DTD DocBook XML V4.4//EN"
"http://www.oasis-open.org/docbook/xml/4.4/docbookx.dtd">
<article>
  <title>Ordered List Test</title>
  <orderedlist>
    <listitem>
      <para>First step</para>
    </listitem>
    <listitem>
      <para>Second step</para>
    </listitem>
    <listitem>
      <para>Third step</para>
    </listitem>
  </orderedlist>
</article>"#;

    let result = extract_docbook_bytes(docbook.as_bytes(), "application/docbook+xml").await;
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.content.contains("First step"));
    assert!(result.content.contains("Second step"));
    assert!(result.content.contains("Third step"));
    assert!(result.content.contains("1. "), "Should contain numbered list");
}

#[tokio::test]
async fn test_docbook_blockquote() {
    let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE article PUBLIC "-//OASIS//DTD DocBook XML V4.4//EN"
"http://www.oasis-open.org/docbook/xml/4.4/docbookx.dtd">
<article>
  <title>Blockquote Test</title>
  <blockquote>
    <para>This is a quoted passage.</para>
  </blockquote>
</article>"#;

    let result = extract_docbook_bytes(docbook.as_bytes(), "application/docbook+xml").await;
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.content.contains("quoted passage"));
    assert!(result.content.contains("> "), "Should contain blockquote marker");
}

#[tokio::test]
async fn test_docbook_figure() {
    let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE article PUBLIC "-//OASIS//DTD DocBook XML V4.4//EN"
"http://www.oasis-open.org/docbook/xml/4.4/docbookx.dtd">
<article>
  <title>Figure Test</title>
  <figure>
    <title>Sample Figure</title>
    <para>This is a figure description.</para>
  </figure>
</article>"#;

    let result = extract_docbook_bytes(docbook.as_bytes(), "application/docbook+xml").await;
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.content.contains("Figure"));
}

#[tokio::test]
async fn test_docbook_footnote() {
    let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE article PUBLIC "-//OASIS//DTD DocBook XML V4.4//EN"
"http://www.oasis-open.org/docbook/xml/4.4/docbookx.dtd">
<article>
  <title>Footnote Test</title>
  <para>Here is some text with a footnote<footnote><para>This is the footnote content</para></footnote>.</para>
</article>"#;

    let result = extract_docbook_bytes(docbook.as_bytes(), "application/docbook+xml").await;
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.content.contains("text with a footnote"));
    assert!(result.content.contains("footnote content"));
}

#[tokio::test]
async fn test_docbook_mixed_content_with_lists() {
    let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE article PUBLIC "-//OASIS//DTD DocBook XML V4.4//EN"
"http://www.oasis-open.org/docbook/xml/4.4/docbookx.dtd">
<article>
  <title>Mixed Content</title>
  <para>Introduction paragraph.</para>
  <itemizedlist>
    <listitem>
      <para>List item 1</para>
    </listitem>
    <listitem>
      <para>List item 2</para>
    </listitem>
  </itemizedlist>
  <para>Conclusion paragraph.</para>
  <programlisting>
code example
  </programlisting>
</article>"#;

    let result = extract_docbook_bytes(docbook.as_bytes(), "application/docbook+xml").await;
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.content.contains("Introduction paragraph"));
    assert!(result.content.contains("List item 1"));
    assert!(result.content.contains("List item 2"));
    assert!(result.content.contains("Conclusion paragraph"));
    assert!(result.content.contains("code example"));
}

#[tokio::test]
async fn test_docbook_namespaced_lists() {
    let docbook5 = r#"<?xml version="1.0" encoding="UTF-8"?>
<article xmlns="http://docbook.org/ns/docbook">
  <info>
    <title>Lists in DocBook 5</title>
  </info>
  <itemizedlist>
    <listitem>
      <para>Namespaced item 1</para>
    </listitem>
    <listitem>
      <para>Namespaced item 2</para>
    </listitem>
  </itemizedlist>
</article>"#;

    let result = extract_docbook_bytes(docbook5.as_bytes(), "application/docbook+xml").await;
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.content.contains("Namespaced item 1"));
    assert!(result.content.contains("Namespaced item 2"));
    assert!(result.content.contains("- "));
}
