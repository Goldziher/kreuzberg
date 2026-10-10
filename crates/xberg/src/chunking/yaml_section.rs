//! Structured data section-aware chunking (YAML and JSON).
//!
//! Splits YAML and JSON documents by nested keys, creating one chunk per
//! leaf section. Each chunk includes the full key path for context, making it
//! possible for RAG systems to index configuration files at the section level
//! rather than as a single monolithic document.
//!
//! Both formats are parsed into a value tree (`serde_yaml_ng` for YAML,
//! `serde_json` for JSON) and walked with BFS to avoid stack overflow on
//! deeply nested untrusted input (see RUSTSEC-2024-0012).

use std::collections::VecDeque;

use crate::error::Result;
use crate::types::{Chunk, ChunkMetadata, PageBoundary};

use super::boundaries::PageIndex;
use super::config::{ChunkingConfig, ChunkingResult};
use super::section_spans::{Span, json_member_spans, yaml_member_spans};

/// Split YAML or JSON text into per-section chunks.
///
/// Detects JSON (starts with `{` or `[`) and parses with `serde_json`.
/// Otherwise parses with `serde_yaml_ng`. Both are walked with BFS.
///
/// Falls back to plain text chunking if the input cannot be parsed
/// or has no extractable mapping keys.
pub(crate) fn chunk_yaml_by_sections(
    text: &str,
    config: &ChunkingConfig,
    page_boundaries: Option<&[PageBoundary]>,
) -> Result<ChunkingResult> {
    if text.is_empty() {
        return Ok(ChunkingResult {
            chunks: vec![],
            chunk_count: 0,
        });
    }

    let trimmed = text.trim();
    let is_json = trimmed.starts_with('{') || trimmed.starts_with('[');
    let mut sections = if is_json {
        flatten_json(text)?
    } else {
        flatten_yaml(text)?
    };

    if sections.is_empty() {
        return fallback_to_text(text, config);
    }

    assign_source_spans(text, is_json, &mut sections);

    build_chunks_from_sections(&sections, config, page_boundaries)
}

/// Locate each section in the source text, falling back to its nearest located ancestor.
///
/// Sections the scanners cannot place keep a `0..0` range.
fn assign_source_spans(text: &str, is_json: bool, sections: &mut [Section]) {
    let spans = if is_json {
        json_member_spans(text)
    } else {
        let prefixes: Vec<&[String]> = sections
            .iter()
            .flat_map(|section| (1..=section.path.len()).map(|len| &section.path[..len]))
            .collect();
        yaml_member_spans(text, prefixes)
    };
    for section in sections {
        let located: Option<Span> = (1..=section.path.len())
            .rev()
            .find_map(|len| spans.get(&section.path[..len].join(" > ")).copied());
        if let Some((start, end)) = located {
            section.byte_start = start;
            section.byte_end = end;
        }
    }
}

/// Parse JSON and flatten into leaf sections via BFS.
///
/// Uses serde_json for strict JSON parsing, then converts to serde_yaml_ng::Value
/// so both paths share the same BFS walker.
fn flatten_json(text: &str) -> Result<Vec<Section>> {
    let json_val: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return Ok(Vec::new()),
    };
    if !json_val.is_object() {
        return Ok(Vec::new());
    }
    let yaml_str = serde_json::to_string(&json_val).unwrap_or_default();
    let yaml_val: serde_yaml_ng::Value = match serde_yaml_ng::from_str(&yaml_str) {
        Ok(v) => v,
        Err(_) => return Ok(Vec::new()),
    };
    let mapping = match yaml_val.as_mapping() {
        Some(m) => m,
        None => return Ok(Vec::new()),
    };
    Ok(flatten_value_tree(
        mapping
            .iter()
            .filter_map(|(k, v)| Some((k.as_str()?.to_string(), v.clone()))),
    ))
}

/// Parse YAML and flatten into leaf sections via BFS.
fn flatten_yaml(text: &str) -> Result<Vec<Section>> {
    let parsed: serde_yaml_ng::Value = match serde_yaml_ng::from_str(text) {
        Ok(v) => v,
        Err(_) => return Ok(Vec::new()),
    };
    let mapping = match parsed.as_mapping() {
        Some(m) => m,
        None => return Ok(Vec::new()),
    };
    Ok(flatten_value_tree(
        mapping
            .iter()
            .filter_map(|(k, v)| Some((k.as_str()?.to_string(), v.clone()))),
    ))
}

/// BFS walk of a key-value tree, collecting leaf sections.
///
/// Uses a VecDeque work queue instead of recursion to avoid stack overflow
/// on deeply nested input (RUSTSEC-2024-0012).
///
/// A node is a leaf if it is not a mapping/object, or is an empty mapping/object.
fn flatten_value_tree(roots: impl Iterator<Item = (String, serde_yaml_ng::Value)>) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    let mut queue: VecDeque<(Vec<String>, serde_yaml_ng::Value)> = VecDeque::new();

    for (key, val) in roots {
        queue.push_back((vec![key], val));
    }

    while let Some((path, value)) = queue.pop_front() {
        match value {
            serde_yaml_ng::Value::Mapping(map) if !map.is_empty() => {
                for (k, v) in map {
                    if let Some(key_str) = k.as_str() {
                        let mut child_path = path.clone();
                        child_path.push(key_str.to_string());
                        queue.push_back((child_path, v));
                    }
                }
            }
            other => {
                sections.push(Section {
                    key: path.join(" > "),
                    path,
                    value: format_yaml_value(&other),
                    byte_start: 0,
                    byte_end: 0,
                });
            }
        }
    }

    sections
}

/// Format a YAML value as a readable string for chunk content.
fn format_yaml_value(value: &serde_yaml_ng::Value) -> String {
    match value {
        serde_yaml_ng::Value::String(s) => s.clone(),
        serde_yaml_ng::Value::Null => "null".to_string(),
        serde_yaml_ng::Value::Bool(b) => b.to_string(),
        serde_yaml_ng::Value::Number(n) => n.to_string(),
        other => serde_yaml_ng::to_string(other).unwrap_or_default().trim().to_string(),
    }
}

/// Fall back to the plain text chunker.
fn fallback_to_text(text: &str, config: &ChunkingConfig) -> Result<ChunkingResult> {
    let fallback_config = ChunkingConfig {
        chunker_type: super::config::ChunkerType::Text,
        ..config.clone()
    };
    super::core::chunk_text(text, &fallback_config, None)
}

/// Per-section page data derived from `page_boundaries`.
struct SectionPageData {
    first_page: Option<u32>,
    last_page: Option<u32>,
    page_spans: Vec<crate::types::PageSpan>,
}

/// Compute the page data for a section from the pages its source range overlaps.
///
/// A section the scanners could not place has an empty range at offset 0 and resolves to the
/// page containing that offset.
fn section_page_data(section: &Section, pages: Option<&PageIndex<'_>>) -> SectionPageData {
    let Some(pages) = pages else {
        return SectionPageData {
            first_page: None,
            last_page: None,
            page_spans: Vec::new(),
        };
    };
    let end = section.byte_end.max(section.byte_start + 1);
    let (first_page, last_page) = pages.range(section.byte_start, end);
    SectionPageData {
        first_page,
        last_page,
        page_spans: pages.spans(section.byte_start, end),
    }
}

/// Build `ChunkMetadata` for a section (or sub-chunk) from its byte range, token count,
/// and page data.
fn section_chunk_metadata(
    byte_start: usize,
    byte_end: usize,
    token_count: Option<usize>,
    page_data: &SectionPageData,
) -> ChunkMetadata {
    ChunkMetadata {
        byte_start,
        byte_end,
        token_count,
        chunk_index: 0,
        total_chunks: 0,
        first_page: page_data.first_page,
        last_page: page_data.last_page,
        heading_context: None,
        heading_path: Vec::new(),
        image_indices: Vec::new(),
        node_ids: Vec::new(),
        page_spans: page_data.page_spans.clone(),
        classifications: Vec::new(),
    }
}

/// Build a `Chunk` from its content and metadata.
fn build_chunk(content: String, metadata: ChunkMetadata) -> Chunk {
    Chunk {
        content,
        chunk_type: Default::default(),
        embedding: None,
        sparse_embedding: None,
        late_interaction: None,
        metadata,
    }
}

/// Shared logic: convert Sections into Chunks, handling oversized splitting.
fn build_chunks_from_sections(
    sections: &[Section],
    config: &ChunkingConfig,
    page_boundaries: Option<&[PageBoundary]>,
) -> Result<ChunkingResult> {
    let mut chunks: Vec<Chunk> = Vec::new();
    let token_counter = super::builder::resolve_token_counter(&config.sizing);
    let page_index = page_boundaries
        .filter(|b| !b.is_empty())
        .map(PageIndex::new)
        .transpose()?;

    for section in sections {
        let prefix = format!("# {}", section.key);
        let content = format!("{}\n\n{}", prefix, section.value.trim());
        let page_data = section_page_data(section, page_index.as_ref());

        if config.max_characters == 0 || content.len() <= config.max_characters {
            let token_count = token_counter.as_ref().map(|counter| counter(&content));
            let metadata = section_chunk_metadata(section.byte_start, section.byte_end, token_count, &page_data);
            chunks.push(build_chunk(content, metadata));
        } else {
            let sub_result = super::core::chunk_text(
                section.value.trim(),
                &ChunkingConfig {
                    chunker_type: super::config::ChunkerType::Text,
                    ..config.clone()
                },
                None,
            )?;
            for sub_chunk in sub_result.chunks {
                let metadata = section_chunk_metadata(
                    section.byte_start,
                    section.byte_end,
                    sub_chunk.metadata.token_count,
                    &page_data,
                );
                chunks.push(build_chunk(format!("{}\n\n{}", prefix, sub_chunk.content), metadata));
            }
        }
    }

    let total = chunks.len();
    for (i, chunk) in chunks.iter_mut().enumerate() {
        chunk.metadata.chunk_index = i;
        chunk.metadata.total_chunks = total;
    }

    Ok(ChunkingResult {
        chunk_count: total,
        chunks,
    })
}

struct Section {
    key: String,
    /// Key segments from the document root to this section.
    path: Vec<String>,
    value: String,
    byte_start: usize,
    byte_end: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunking::config::ChunkerType;

    fn make_config() -> ChunkingConfig {
        ChunkingConfig {
            max_characters: 10000,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Yaml,
            ..Default::default()
        }
    }

    #[test]
    fn test_simple_yaml() {
        let yaml = "server:\n  host: localhost\n  port: 8080\ndb:\n  name: mydb\n  user: admin";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 4);
        assert!(result.chunks[0].content.contains("# server > host"));
        assert!(result.chunks[0].content.contains("localhost"));
        assert!(result.chunks[1].content.contains("# server > port"));
        assert!(result.chunks[2].content.contains("# db > name"));
        assert!(result.chunks[2].content.contains("mydb"));
        assert!(result.chunks[3].content.contains("# db > user"));
    }

    #[test]
    fn test_nested_yaml() {
        let yaml = "app:\n  name: test\n  config:\n    debug: true\n    log_level: info";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 3);
        assert!(result.chunks[0].content.contains("# app > name"));
        assert!(result.chunks[0].content.contains("test"));
        assert!(result.chunks[1].content.contains("# app > config > debug"));
        assert!(result.chunks[1].content.contains("true"));
        assert!(result.chunks[2].content.contains("# app > config > log_level"));
        assert!(result.chunks[2].content.contains("info"));
    }

    #[test]
    fn test_inline_values() {
        let yaml = "name: myapp\nversion: 1.0\ndescription: A test app";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 3);
        assert!(result.chunks[0].content.contains("# name"));
        assert!(result.chunks[0].content.contains("myapp"));
    }

    #[test]
    fn test_empty_yaml() {
        let result = chunk_yaml_by_sections("", &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 0);
    }

    #[test]
    fn test_comments_and_blanks() {
        let yaml = "# Top comment\n\nserver:\n  port: 8080\n\n# Another comment\nclient:\n  timeout: 30";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 2);
    }

    #[test]
    fn test_chunk_indices() {
        let yaml = "a: 1\nb: 2\nc: 3";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        for (i, chunk) in result.chunks.iter().enumerate() {
            assert_eq!(chunk.metadata.chunk_index, i);
            assert_eq!(chunk.metadata.total_chunks, 3);
        }
    }

    #[test]
    fn test_produces_chunks_for_flat_yaml() {
        let yaml = "first: value\nsecond: value";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 2);
        assert!(result.chunks[0].content.contains("# first"));
        assert!(result.chunks[1].content.contains("# second"));
    }

    #[test]
    fn test_single_scalar() {
        let yaml = "a: 1";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 1);
        assert!(result.chunks[0].content.contains("# a"));
    }

    #[test]
    fn test_array_values() {
        let yaml = "items:\n  - one\n  - two\n  - three\nother: stuff";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 2);
        assert!(result.chunks[0].content.contains("- one"));
    }

    #[test]
    fn test_no_top_level_keys_falls_back() {
        let yaml = "  indented: value\n  another: value";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert!(result.chunk_count >= 1);
    }

    #[test]
    fn test_unicode_keys() {
        let yaml = "\u{540d}\u{524d}: test\n\u{8a2d}\u{5b9a}:\n  key: value";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 2);
    }

    #[test]
    fn test_multiline_string_values() {
        let yaml = "description: |\n  This is a\n  multiline string\nother: value";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 2);
        assert!(result.chunks[0].content.contains("multiline string"));
    }

    #[test]
    fn test_colon_in_value() {
        let yaml = "url: http://example.com:8080\nname: test";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 2);
        assert!(result.chunks[0].content.contains("# url"));
    }

    #[test]
    fn test_windows_line_endings() {
        let yaml = "a: 1\r\nb: 2\r\nc: 3";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 3);
        assert!(result.chunks[0].content.contains("# a"));
        assert!(result.chunks[1].content.contains("# b"));
        assert!(result.chunks[2].content.contains("# c"));
    }

    #[test]
    fn test_oversized_section_split() {
        let long_text = "word ".repeat(500);
        let yaml = format!("description: |\n  {}\nother: ok", long_text.trim());
        let config = ChunkingConfig {
            max_characters: 100,
            ..make_config()
        };
        let result = chunk_yaml_by_sections(&yaml, &config, None).unwrap();
        assert!(result.chunk_count > 2);
    }

    #[test]
    fn test_document_separator() {
        let yaml = "---\nname: test\nversion: 1.0";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert!(result.chunk_count >= 1);
    }

    #[test]
    fn test_single_key() {
        let yaml = "only_key:\n  a: 1\n  b: 2";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 2);
        assert!(result.chunks[0].content.contains("# only_key > a"));
        assert!(result.chunks[1].content.contains("# only_key > b"));
    }

    #[test]
    fn test_flow_style_value() {
        let yaml = "ports: [80, 443]\nname: test";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 2);
        assert!(result.chunks.iter().any(|c| c.content.contains("# ports")));
        assert!(result.chunks.iter().any(|c| c.content.contains("# name")));
    }

    #[test]
    fn test_chunk_indices_after_split() {
        let yaml = "a: 1\nb: 2\nc: 3";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        for (i, chunk) in result.chunks.iter().enumerate() {
            assert_eq!(chunk.metadata.chunk_index, i);
            assert_eq!(chunk.metadata.total_chunks, result.chunk_count);
        }
    }

    #[test]
    fn test_anchor_alias() {
        let yaml = "defaults: &defaults\n  adapter: postgres\nproduction:\n  <<: *defaults\n  host: prod-db";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 3);
    }

    #[test]
    fn test_chunk_metadata_valid() {
        let yaml = "server:\n  host: localhost\n  port: 8080\ndb:\n  name: mydb";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        for chunk in &result.chunks {
            assert!(chunk.metadata.byte_start <= chunk.metadata.byte_end);
            assert!(!chunk.content.is_empty());
        }
    }

    #[test]
    fn test_max_characters_zero_disables_splitting() {
        let long_text = "word ".repeat(500);
        let yaml = format!("big: |\n  {}\nsmall: ok", long_text.trim());
        let config = ChunkingConfig {
            max_characters: 0,
            ..make_config()
        };
        let result = chunk_yaml_by_sections(&yaml, &config, None).unwrap();
        assert_eq!(result.chunk_count, 2);
        assert!(result.chunks.iter().any(|c| c.content.contains("# big")));
        assert!(result.chunks.iter().any(|c| c.content.contains("# small")));
    }

    #[test]
    fn test_nested_yaml_creates_leaf_chunks() {
        let yaml = "database:\n  primary:\n    host: db1\n  replica:\n    host: db2";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 2);
        assert!(result.chunks[0].content.contains("# database > primary > host"));
        assert!(result.chunks[0].content.contains("db1"));
        assert!(result.chunks[1].content.contains("# database > replica > host"));
        assert!(result.chunks[1].content.contains("db2"));
    }

    #[test]
    fn test_deeply_nested_4_levels() {
        let yaml = "a:\n  b:\n    c:\n      d: deep_value";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 1);
        assert!(result.chunks[0].content.contains("# a > b > c > d"));
        assert!(result.chunks[0].content.contains("deep_value"));
    }

    #[test]
    fn test_nested_with_sibling_reset() {
        let yaml = "parent1:\n  child_a: 1\n  child_b: 2\nparent2:\n  child_c: 3\n  child_d: 4";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 4);
        assert!(result.chunks[0].content.contains("# parent1 > child_a"));
        assert!(result.chunks[1].content.contains("# parent1 > child_b"));
        assert!(result.chunks[2].content.contains("# parent2 > child_c"));
        assert!(result.chunks[3].content.contains("# parent2 > child_d"));
        assert!(!result.chunks[2].content.contains("parent1"));
        assert!(!result.chunks[3].content.contains("parent1"));
    }

    #[test]
    fn test_json_object_by_keys() {
        let json = r#"{"server": "localhost", "port": 8080, "debug": true}"#;
        let result = chunk_yaml_by_sections(json, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 3);
        let all_content: String = result
            .chunks
            .iter()
            .map(|c| c.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(all_content.contains("# debug"), "should contain debug key");
        assert!(all_content.contains("# port"), "should contain port key");
        assert!(all_content.contains("# server"), "should contain server key");
        assert!(all_content.contains("localhost"), "should contain localhost value");
    }

    #[test]
    fn test_json_nested_objects() {
        let json = r#"{"database": {"primary": {"host": "db1"}, "replica": {"host": "db2"}}}"#;
        let result = chunk_yaml_by_sections(json, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 2);
        assert!(result.chunks[0].content.contains("# database > primary > host"));
        assert!(result.chunks[0].content.contains("db1"));
        assert!(result.chunks[1].content.contains("# database > replica > host"));
        assert!(result.chunks[1].content.contains("db2"));
    }

    #[test]
    fn test_json_array_root_fallback() {
        let json = r#"[1, 2, 3, 4, 5]"#;
        let result = chunk_yaml_by_sections(json, &make_config(), None).unwrap();
        assert!(result.chunk_count >= 1);
        assert!(!result.chunks[0].content.starts_with("# "));
    }

    #[test]
    fn test_json_minified() {
        let json = r#"{"a":1,"b":{"c":2,"d":3},"e":"hello"}"#;
        let result = chunk_yaml_by_sections(json, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 4);
        assert!(result.chunks[0].content.contains("# a"));
        assert!(result.chunks[1].content.contains("# e"));
        assert!(result.chunks[1].content.contains("hello"));
        assert!(result.chunks[2].content.contains("# b > c"));
        assert!(result.chunks[3].content.contains("# b > d"));
    }

    #[test]
    fn test_json_empty_object() {
        let json = r#"{}"#;
        let result = chunk_yaml_by_sections(json, &make_config(), None).unwrap();
        assert!(result.chunk_count <= 1);
    }

    #[test]
    fn test_json_deeply_nested() {
        let json = r#"{"a": {"b": {"c": {"d": {"e": "deep"}}}}}"#;
        let result = chunk_yaml_by_sections(json, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 1);
        assert!(result.chunks[0].content.contains("# a > b > c > d > e"));
        assert!(result.chunks[0].content.contains("deep"));
    }

    #[test]
    fn test_invalid_yaml_falls_back() {
        let yaml = ":\n  - :\n  invalid:: yaml::: [unterminated";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert!(result.chunk_count > 0);
    }

    #[test]
    fn test_malformed_json_falls_back() {
        let json = r#"{"key": "unterminated"#;
        let result = chunk_yaml_by_sections(json, &make_config(), None).unwrap();
        assert!(result.chunk_count > 0);
    }

    #[test]
    fn test_null_values_handled() {
        let yaml = "present: value\nmissing:\nempty: \"\"";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 3);
        assert!(result.chunks[0].content.contains("# present"));
        assert!(result.chunks[1].content.contains("# missing"));
        assert!(result.chunks[2].content.contains("# empty"));
    }

    #[test]
    fn test_yaml_scalar_root_falls_back() {
        let yaml = "just a plain string";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert!(result.chunk_count >= 1);
        assert!(!result.chunks[0].content.starts_with("# "));
    }

    #[test]
    fn test_yaml_list_root_falls_back() {
        let yaml = "- item1\n- item2\n- item3";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert!(result.chunk_count >= 1);
        assert!(!result.chunks[0].content.starts_with("# "));
    }

    #[test]
    fn test_json_null_value() {
        let json = r#"{"key": null, "other": "value"}"#;
        let result = chunk_yaml_by_sections(json, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 2);
        assert!(result.chunks.iter().any(|c| c.content.contains("# key")));
        assert!(result.chunks.iter().any(|c| c.content.contains("# other")));
    }

    #[test]
    fn test_boolean_leaf_values() {
        let yaml = "debug: true\nverbose: false";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        assert_eq!(result.chunk_count, 2);
        assert!(result.chunks[0].content.contains("true"));
        assert!(result.chunks[1].content.contains("false"));
    }

    #[test]
    fn test_page_bounds_none_without_boundaries() {
        let yaml = "server:\n  host: localhost\n  port: 8080";
        let result = chunk_yaml_by_sections(yaml, &make_config(), None).unwrap();
        for chunk in &result.chunks {
            assert_eq!(chunk.metadata.first_page, None);
            assert_eq!(chunk.metadata.last_page, None);
        }
    }

    #[test]
    fn test_page_bounds_set_from_single_boundary() {
        let yaml = "server:\n  host: localhost\n  port: 8080";
        let boundaries = vec![PageBoundary {
            byte_start: 0,
            byte_end: yaml.len(),
            page_number: 3,
        }];
        let result = chunk_yaml_by_sections(yaml, &make_config(), Some(&boundaries)).unwrap();
        assert_eq!(result.chunk_count, 2);
        for chunk in &result.chunks {
            assert_eq!(chunk.metadata.first_page, Some(3), "first_page should be page 3");
            assert_eq!(chunk.metadata.last_page, Some(3), "last_page should be page 3");
        }
    }

    #[test]
    fn test_page_bounds_set_for_oversized_sub_chunks() {
        let long_text = "word ".repeat(500);
        let yaml = format!("description: |\n  {}\nother: ok", long_text.trim());
        let config = ChunkingConfig {
            max_characters: 100,
            ..make_config()
        };
        let boundaries = vec![PageBoundary {
            byte_start: 0,
            byte_end: yaml.len(),
            page_number: 7,
        }];
        let result = chunk_yaml_by_sections(&yaml, &config, Some(&boundaries)).unwrap();
        assert!(result.chunk_count > 1);
        for chunk in &result.chunks {
            assert_eq!(chunk.metadata.first_page, Some(7));
            assert_eq!(chunk.metadata.last_page, Some(7));
        }
    }

    #[test]
    fn test_empty_boundaries_slice_leaves_pages_none() {
        let yaml = "a: 1\nb: 2";
        let boundaries: Vec<PageBoundary> = vec![];
        let result = chunk_yaml_by_sections(yaml, &make_config(), Some(&boundaries)).unwrap();
        for chunk in &result.chunks {
            assert_eq!(chunk.metadata.first_page, None);
            assert_eq!(chunk.metadata.last_page, None);
        }
    }

    /// Regression test for #255: the single-chunk-per-section path always set
    /// `token_count: None` even with `ChunkSizing::Tokenizer` configured.
    #[cfg(feature = "chunking-tokenizers")]
    #[test]
    fn test_token_count_populated_for_single_chunk_section() {
        use crate::plugins::registry::test_support::TokenizerRegistryGuard;
        use crate::plugins::{Plugin, TokenizerBackend, register_tokenizer_backend};
        use std::sync::Arc;

        struct WordCountTokenizer;
        impl Plugin for WordCountTokenizer {
            fn name(&self) -> &str {
                "chunking-yaml-word-count-tokenizer"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> crate::Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> crate::Result<()> {
                Ok(())
            }
        }
        impl TokenizerBackend for WordCountTokenizer {
            fn count_tokens(&self, text: &str) -> usize {
                text.split_whitespace().count()
            }
        }

        let _guard = TokenizerRegistryGuard::acquire();
        register_tokenizer_backend(Arc::new(WordCountTokenizer)).unwrap();

        let yaml = "a: one two three";
        let config = ChunkingConfig {
            sizing: crate::core::config::ChunkSizing::Tokenizer {
                model: "chunking-yaml-word-count-tokenizer".to_string(),
                cache_dir: None,
            },
            ..make_config()
        };
        let result = chunk_yaml_by_sections(yaml, &config, None).unwrap();

        assert_eq!(result.chunk_count, 1);
        assert!(
            result.chunks[0].metadata.token_count.is_some(),
            "token_count must be populated for a single-chunk section, got None"
        );
    }

    /// Regression test for #255: the oversized-section split path discarded the
    /// delegated `chunk_text` sub-chunks' `token_count`, hardcoding `None` instead of
    /// carrying it through.
    #[cfg(feature = "chunking-tokenizers")]
    #[test]
    fn test_token_count_carried_over_for_oversized_sub_chunks() {
        use crate::plugins::registry::test_support::TokenizerRegistryGuard;
        use crate::plugins::{Plugin, TokenizerBackend, register_tokenizer_backend};
        use std::sync::Arc;

        struct WordCountTokenizer;
        impl Plugin for WordCountTokenizer {
            fn name(&self) -> &str {
                "chunking-yaml-oversized-word-count-tokenizer"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> crate::Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> crate::Result<()> {
                Ok(())
            }
        }
        impl TokenizerBackend for WordCountTokenizer {
            fn count_tokens(&self, text: &str) -> usize {
                text.split_whitespace().count()
            }
        }

        let _guard = TokenizerRegistryGuard::acquire();
        register_tokenizer_backend(Arc::new(WordCountTokenizer)).unwrap();

        let long_text = "word ".repeat(500);
        let yaml = format!("description: |\n  {}\nother: ok", long_text.trim());
        let config = ChunkingConfig {
            max_characters: 100,
            sizing: crate::core::config::ChunkSizing::Tokenizer {
                model: "chunking-yaml-oversized-word-count-tokenizer".to_string(),
                cache_dir: None,
            },
            ..make_config()
        };
        let result = chunk_yaml_by_sections(&yaml, &config, None).unwrap();

        assert!(result.chunk_count > 2, "expected the oversized section to split");
        let description_chunks: Vec<_> = result
            .chunks
            .iter()
            .filter(|c| c.content.contains("# description"))
            .collect();
        assert!(
            !description_chunks.is_empty(),
            "expected at least one chunk from the oversized 'description' section"
        );
        for chunk in description_chunks {
            assert!(
                chunk.metadata.token_count.is_some(),
                "oversized sub-chunk must carry over token_count from the delegated chunk_text call, got None"
            );
        }
    }
}
