//! BibTeX bibliography extractor.
//!
//! Extracts and parses BibTeX bibliography files, providing structured access
//! to bibliography entries, metadata, and author information.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};

#[cfg(feature = "office")]
use biblatex::{Bibliography, ChunksExt};

/// BibTeX bibliography extractor.
///
/// Parses BibTeX files and extracts structured bibliography data including
/// entries, authors, publication years, and entry type distribution.
pub struct BibtexExtractor;

impl BibtexExtractor {
    /// Create a new BibTeX extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for BibtexExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for BibtexExtractor {
    fn name(&self) -> &str {
        "bibtex-extractor"
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
        "Extracts and parses BibTeX bibliography files with structured metadata"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg(feature = "office")]
#[async_trait]
impl DocumentExtractor for BibtexExtractor {
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
        let bibtex_str = String::from_utf8_lossy(content);

        let mut entries_vec = Vec::new();
        let mut authors_set = HashSet::new();
        let mut years_set = HashSet::new();
        let mut entry_types_map = HashMap::new();
        let mut formatted_entries = String::new();

        match Bibliography::parse(&bibtex_str) {
            Ok(bib) => {
                for entry in bib.iter() {
                    let key = entry.key.clone();
                    let entry_type = entry.entry_type.clone();

                    formatted_entries.push_str(&format!("@{} {{\n", entry_type));
                    formatted_entries.push_str(&format!("  key = {},\n", key));

                    for (field_name, field_chunks) in &entry.fields {
                        let field_text = field_chunks.format_verbatim();
                        formatted_entries.push_str(&format!("  {} = {},\n", field_name, field_text));

                        if field_name.to_lowercase() == "author" {
                            let authors_text = field_chunks.format_verbatim();
                            for author in authors_text.split(" and ") {
                                let trimmed_author = author.trim().to_string();
                                if !trimmed_author.is_empty() {
                                    authors_set.insert(trimmed_author);
                                }
                            }
                        }

                        if field_name.to_lowercase() == "year" {
                            let year_str = field_chunks.format_verbatim();
                            if let Ok(year) = year_str.parse::<u32>() {
                                years_set.insert(year);
                            }
                        }
                    }

                    formatted_entries.push_str("}\n\n");

                    *entry_types_map
                        .entry(entry_type.to_string().to_lowercase())
                        .or_insert(0) += 1;

                    entries_vec.push(key);
                }
            }
            Err(_err) => {
                #[cfg(feature = "otel")]
                tracing::warn!("BibTeX parsing failed, returning raw content: {}", _err);
                formatted_entries = bibtex_str.to_string();
            }
        }

        let mut additional = HashMap::new();

        additional.insert("entry_count".to_string(), serde_json::json!(entries_vec.len()));

        let mut authors_list: Vec<String> = authors_set.into_iter().collect();
        authors_list.sort();
        additional.insert("authors".to_string(), serde_json::json!(authors_list));

        if !years_set.is_empty() {
            let min_year = years_set.iter().min().copied().unwrap_or(0);
            let max_year = years_set.iter().max().copied().unwrap_or(0);
            additional.insert(
                "year_range".to_string(),
                serde_json::json!({
                    "min": min_year,
                    "max": max_year,
                    "years": years_set.into_iter().collect::<Vec<_>>()
                }),
            );
        }

        if !entry_types_map.is_empty() {
            let mut entry_types_json = serde_json::json!({});
            for (entry_type, count) in entry_types_map {
                entry_types_json[entry_type] = serde_json::json!(count);
            }
            additional.insert("entry_types".to_string(), entry_types_json);
        }

        additional.insert("citation_keys".to_string(), serde_json::json!(entries_vec));

        Ok(ExtractionResult {
            content: formatted_entries,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                additional,
                ..Default::default()
            },
            pages: None,
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-bibtex", "text/x-bibtex"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(all(test, feature = "office"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_can_extract_bibtex_mime_types() {
        let extractor = BibtexExtractor::new();
        let supported = extractor.supported_mime_types();

        assert!(supported.contains(&"application/x-bibtex"));
        assert!(supported.contains(&"text/x-bibtex"));
        assert_eq!(supported.len(), 2);
    }

    #[tokio::test]
    async fn test_extract_simple_bibtex() {
        let extractor = BibtexExtractor::new();
        let bibtex_content = br#"@article{key2023,
    title = {Sample Title},
    author = {John Doe},
    year = {2023}
}"#;

        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(bibtex_content, "application/x-bibtex", &config)
            .await;

        assert!(result.is_ok());
        let result = result.expect("Should extract valid BibTeX entry");

        assert!(result.content.contains("@article"));
        assert!(result.content.contains("key2023"));
        assert!(result.content.contains("Sample Title"));

        let metadata = &result.metadata;
        assert_eq!(metadata.additional.get("entry_count"), Some(&serde_json::json!(1)));
    }

    #[tokio::test]
    async fn test_extract_multiple_entries() {
        let extractor = BibtexExtractor::new();
        let bibtex_content = br#"@article{first2020,
    title = {First Paper},
    author = {Author One},
    year = {2020},
    journal = {Test Journal}
}

@book{second2021,
    title = {Test Book},
    author = {Author Two},
    year = {2021},
    publisher = {Test Publisher}
}

@inproceedings{third2022,
    title = {Conference Paper},
    author = {Author Three},
    year = {2022}
}"#;

        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(bibtex_content, "application/x-bibtex", &config)
            .await;

        assert!(result.is_ok());
        let result = result.expect("Should extract valid BibTeX entries");

        let metadata = &result.metadata;

        assert_eq!(metadata.additional.get("entry_count"), Some(&serde_json::json!(3)));

        if let Some(keys) = metadata.additional.get("citation_keys")
            && let Some(keys_array) = keys.as_array()
        {
            assert_eq!(keys_array.len(), 3);
        }

        if let Some(types) = metadata.additional.get("entry_types") {
            assert!(types.get("article").is_some());
            assert!(types.get("book").is_some());
            assert!(types.get("inproceedings").is_some());
        }
    }

    #[tokio::test]
    async fn test_extract_article_entry() {
        let extractor = BibtexExtractor::new();
        let bibtex_content = br#"@article{einstein1905,
    author = {Albert Einstein},
    title = {On the Electrodynamics of Moving Bodies},
    journal = {Annalen der Physik},
    year = {1905},
    volume = {17},
    pages = {891-921}
}"#;

        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(bibtex_content, "application/x-bibtex", &config)
            .await;

        assert!(result.is_ok());
        let result = result.expect("Should extract valid article entry");

        assert!(result.content.contains("@article"));
        assert!(result.content.contains("einstein1905"));
        assert!(result.content.contains("On the Electrodynamics of Moving Bodies"));
        assert!(result.content.contains("Annalen der Physik"));

        let metadata = &result.metadata;
        if let Some(authors) = metadata.additional.get("authors")
            && let Some(authors_array) = authors.as_array()
        {
            assert!(!authors_array.is_empty());
            assert!(authors_array[0].as_str().unwrap_or("").contains("Einstein"));
        }
    }

    #[tokio::test]
    async fn test_extract_book_entry() {
        let extractor = BibtexExtractor::new();
        let bibtex_content = br#"@book{knuth1984,
    author = {Donald E. Knuth},
    title = {The TeXbook},
    publisher = {Addison-Wesley},
    year = {1984}
}"#;

        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(bibtex_content, "application/x-bibtex", &config)
            .await;

        assert!(result.is_ok());
        let result = result.expect("Should extract valid book entry");

        assert!(result.content.contains("@book"));
        assert!(result.content.contains("knuth1984"));
        assert!(result.content.contains("The TeXbook"));

        let metadata = &result.metadata;
        assert_eq!(metadata.additional.get("entry_count"), Some(&serde_json::json!(1)));

        if let Some(year_range) = metadata.additional.get("year_range") {
            assert_eq!(year_range.get("min"), Some(&serde_json::json!(1984)));
            assert_eq!(year_range.get("max"), Some(&serde_json::json!(1984)));
        }
    }

    #[tokio::test]
    async fn test_extract_metadata() {
        let extractor = BibtexExtractor::new();
        let bibtex_content = br#"@article{paper1,
    author = {Alice Smith and Bob Jones},
    title = {Title 1},
    year = {2020}
}

@article{paper2,
    author = {Charlie Brown},
    title = {Title 2},
    year = {2021}
}

@book{book1,
    author = {David Lee},
    title = {Book Title},
    year = {2019}
}"#;

        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(bibtex_content, "application/x-bibtex", &config)
            .await;

        assert!(result.is_ok());
        let result = result.expect("Should extract valid metadata");
        let metadata = &result.metadata;

        assert_eq!(metadata.additional.get("entry_count"), Some(&serde_json::json!(3)));

        if let Some(authors) = metadata.additional.get("authors")
            && let Some(authors_array) = authors.as_array()
        {
            assert!(authors_array.len() >= 4);
        }

        if let Some(year_range) = metadata.additional.get("year_range") {
            assert_eq!(year_range.get("min"), Some(&serde_json::json!(2019)));
            assert_eq!(year_range.get("max"), Some(&serde_json::json!(2021)));
        }

        if let Some(types) = metadata.additional.get("entry_types") {
            assert_eq!(types.get("article"), Some(&serde_json::json!(2)));
            assert_eq!(types.get("book"), Some(&serde_json::json!(1)));
        }
    }

    #[tokio::test]
    async fn test_empty_bibliography() {
        let extractor = BibtexExtractor::new();
        let bibtex_content = b"";

        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(bibtex_content, "application/x-bibtex", &config)
            .await;

        assert!(result.is_ok());
        let result = result.expect("Should extract empty bibliography");
        let metadata = &result.metadata;

        assert_eq!(metadata.additional.get("entry_count"), Some(&serde_json::json!(0)));
    }

    #[tokio::test]
    async fn test_malformed_entry() {
        let extractor = BibtexExtractor::new();
        let bibtex_content = br#"@article{incomplete
    title = {Missing fields}

Some random text that's not valid BibTeX"#;

        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(bibtex_content, "application/x-bibtex", &config)
            .await;

        assert!(result.is_ok());
        let result = result.expect("Should extract malformed entry as raw content");

        assert!(!result.content.is_empty());
    }

    #[tokio::test]
    async fn test_multiple_authors_extraction() {
        let extractor = BibtexExtractor::new();
        let bibtex_content = br#"@article{collab2022,
    author = {First Author and Second Author and Third Author},
    title = {Collaborative Work},
    year = {2022}
}"#;

        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(bibtex_content, "application/x-bibtex", &config)
            .await;

        assert!(result.is_ok());
        let result = result.expect("Should extract multiple authors");
        let metadata = &result.metadata;

        if let Some(authors) = metadata.additional.get("authors")
            && let Some(authors_array) = authors.as_array()
        {
            assert!(authors_array.len() >= 3);
        }
    }

    #[tokio::test]
    async fn test_bibtex_extractor_plugin_interface() {
        let extractor = BibtexExtractor::new();
        assert_eq!(extractor.name(), "bibtex-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
        assert!(!extractor.supported_mime_types().is_empty());
    }

    #[test]
    fn test_bibtex_extractor_default() {
        let extractor = BibtexExtractor;
        assert_eq!(extractor.name(), "bibtex-extractor");
    }

    #[tokio::test]
    async fn test_bibtex_extractor_initialize_shutdown() {
        let extractor = BibtexExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }
}
