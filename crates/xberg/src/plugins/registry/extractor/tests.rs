use super::*;
use crate::core::config::{ExtractInput, ExtractionConfig};
use crate::plugins::{DocumentExtractor, InternalDocumentExtractor, Plugin};
use crate::types::{ExtractedDocument, ExtractionMethod};

use async_trait::async_trait;

struct MockExtractor {
    name: String,
    mime_types: &'static [&'static str],
    priority: i32,
}

struct PublicMockExtractor {
    name: &'static str,
    extraction_method: Option<ExtractionMethod>,
}

impl Plugin for PublicMockExtractor {
    fn name(&self) -> &str {
        self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }
}

#[async_trait]
impl DocumentExtractor for PublicMockExtractor {
    async fn extract(&self, _input: ExtractInput, _config: &ExtractionConfig) -> Result<ExtractedDocument> {
        Ok(ExtractedDocument {
            mime_type: "application/x-public".into(),
            extraction_method: self.extraction_method,
            ..Default::default()
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-public"]
    }
}

impl Plugin for MockExtractor {
    fn name(&self) -> &str {
        &self.name
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl InternalDocumentExtractor for MockExtractor {
    async fn extract_content(
        &self,
        _: &[u8],
        _: &str,
        _: &ExtractionConfig,
    ) -> Result<crate::types::internal::InternalDocument> {
        Ok(crate::types::internal::InternalDocument::new("mock"))
    }

    fn supported_mime_types(&self) -> &[&str] {
        self.mime_types
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

#[test]
fn test_document_extractor_registry_exact_match() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractor = Arc::new(MockExtractor {
        name: "pdf-extractor".to_string(),
        mime_types: &["application/pdf"],
        priority: 100,
    });

    registry.register(extractor).unwrap();

    let retrieved = registry.get("application/pdf").unwrap();
    assert_eq!(retrieved.name(), "pdf-extractor");

    let names = registry.list();
    assert_eq!(names.len(), 1);
    assert!(names.contains(&"pdf-extractor".to_string()));
}

#[test]
fn should_distinguish_builtin_extractors_from_public_plugins() {
    let builtin = Arc::new(MockExtractor {
        name: "builtin".to_string(),
        mime_types: &["application/x-builtin"],
        priority: 100,
    });
    let plugin = Arc::new(MockExtractor {
        name: "plugin".to_string(),
        mime_types: &["application/x-plugin"],
        priority: 100,
    });
    let mut registry = DocumentExtractorRegistry::new();
    registry.register_internal(builtin).unwrap();
    registry.register(plugin).unwrap();

    assert!(registry.get_registered("application/x-builtin").unwrap().is_builtin());
    assert!(!registry.get_registered("application/x-plugin").unwrap().is_builtin());
}

#[tokio::test]
async fn should_preserve_explicit_public_plugin_provenance_for_bytes_and_paths() {
    let registered = RegisteredDocumentExtractor::public(Arc::new(PublicMockExtractor {
        name: "public-explicit",
        extraction_method: Some(ExtractionMethod::Ocr),
    }));
    let config = ExtractionConfig::default();

    let bytes = registered
        .extract_content_inner(b"content", "application/x-public", &config)
        .await
        .unwrap();
    let path = registered
        .extract_path_inner(Path::new("unused"), "application/x-public", &config)
        .await
        .unwrap();

    for document in [bytes, path] {
        assert_eq!(
            document.metadata.additional.get("extraction_method"),
            Some(&serde_json::Value::String("ocr".to_string()))
        );
    }
}

#[tokio::test]
async fn should_leave_unspecified_public_plugin_provenance_absent_for_bytes_and_paths() {
    let registered = RegisteredDocumentExtractor::public(Arc::new(PublicMockExtractor {
        name: "public-unspecified",
        extraction_method: None,
    }));
    let config = ExtractionConfig::default();

    let bytes = registered
        .extract_content_inner(b"content", "application/x-public", &config)
        .await
        .unwrap();
    let path = registered
        .extract_path_inner(Path::new("unused"), "application/x-public", &config)
        .await
        .unwrap();

    for document in [bytes, path] {
        assert!(!document.metadata.additional.contains_key("extraction_method"));
    }
}

#[test]
fn test_document_extractor_registry_prefix_match() {
    let mut registry = DocumentExtractorRegistry::new();

    let image_extractor = Arc::new(MockExtractor {
        name: "image-extractor".to_string(),
        mime_types: &["image/*"],
        priority: 50,
    });

    registry.register(image_extractor).unwrap();

    let retrieved = registry.get("image/png").unwrap();
    assert_eq!(retrieved.name(), "image-extractor");

    let retrieved_jpg = registry.get("image/jpeg").unwrap();
    assert_eq!(retrieved_jpg.name(), "image-extractor");
}

#[test]
fn test_document_extractor_registry_priority() {
    let mut registry = DocumentExtractorRegistry::new();

    let low_priority = Arc::new(MockExtractor {
        name: "low-priority-pdf".to_string(),
        mime_types: &["application/pdf"],
        priority: 10,
    });

    let high_priority = Arc::new(MockExtractor {
        name: "high-priority-pdf".to_string(),
        mime_types: &["application/pdf"],
        priority: 100,
    });

    registry.register(low_priority).unwrap();
    registry.register(high_priority).unwrap();

    let retrieved = registry.get("application/pdf").unwrap();
    assert_eq!(retrieved.name(), "high-priority-pdf");
}

#[test]
fn test_document_extractor_registry_not_found() {
    let registry = DocumentExtractorRegistry::new();

    let result = registry.get("application/unknown");
    assert!(matches!(result, Err(XbergError::UnsupportedFormat(_))));
}

#[test]
fn test_document_extractor_registry_remove() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractor = Arc::new(MockExtractor {
        name: "test-extractor".to_string(),
        mime_types: &["text/plain"],
        priority: 50,
    });

    registry.register(extractor).unwrap();
    assert!(registry.get("text/plain").is_ok());

    registry.remove("test-extractor").unwrap();
    assert!(registry.get("text/plain").is_err());
}

#[test]
fn test_document_extractor_registry_shutdown_all() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractor1 = Arc::new(MockExtractor {
        name: "extractor1".to_string(),
        mime_types: &["text/plain"],
        priority: 50,
    });

    let extractor2 = Arc::new(MockExtractor {
        name: "extractor2".to_string(),
        mime_types: &["application/pdf"],
        priority: 50,
    });

    registry.register(extractor1).unwrap();
    registry.register(extractor2).unwrap();

    assert_eq!(registry.list().len(), 2);

    registry.shutdown_all().unwrap();
    assert_eq!(registry.list().len(), 0);
}

#[test]
fn test_document_extractor_registry_default() {
    let registry = DocumentExtractorRegistry::default();
    assert_eq!(registry.list().len(), 0);
}

#[test]
fn test_document_extractor_registry_exact_over_prefix() {
    let mut registry = DocumentExtractorRegistry::new();

    let prefix_extractor = Arc::new(MockExtractor {
        name: "prefix-extractor".to_string(),
        mime_types: &["image/*"],
        priority: 100,
    });

    let exact_extractor = Arc::new(MockExtractor {
        name: "exact-extractor".to_string(),
        mime_types: &["image/png"],
        priority: 50,
    });

    registry.register(prefix_extractor).unwrap();
    registry.register(exact_extractor).unwrap();

    let retrieved = registry.get("image/png").unwrap();
    assert_eq!(retrieved.name(), "exact-extractor");

    let retrieved_jpg = registry.get("image/jpeg").unwrap();
    assert_eq!(retrieved_jpg.name(), "prefix-extractor");
}

#[test]
fn test_document_extractor_registry_invalid_name_empty() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractor = Arc::new(MockExtractor {
        name: "".to_string(),
        mime_types: &["text/plain"],
        priority: 50,
    });

    let result = registry.register(extractor);
    assert!(matches!(result, Err(XbergError::Validation { .. })));
}

#[test]
fn test_document_extractor_registry_invalid_name_whitespace() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractor = Arc::new(MockExtractor {
        name: "my extractor".to_string(),
        mime_types: &["text/plain"],
        priority: 50,
    });

    let result = registry.register(extractor);
    assert!(matches!(result, Err(XbergError::Validation { .. })));
}

#[test]
fn test_document_extractor_registry_multiple_mime_types() {
    let mut registry = DocumentExtractorRegistry::new();

    let multi_extractor = Arc::new(MockExtractor {
        name: "multi-extractor".to_string(),
        mime_types: &["text/plain", "text/markdown", "text/html"],
        priority: 50,
    });

    registry.register(multi_extractor).unwrap();

    assert_eq!(registry.get("text/plain").unwrap().name(), "multi-extractor");
    assert_eq!(registry.get("text/markdown").unwrap().name(), "multi-extractor");
    assert_eq!(registry.get("text/html").unwrap().name(), "multi-extractor");
}

struct FailingExtractor {
    name: String,
    fail_on_init: bool,
}

impl Plugin for FailingExtractor {
    fn name(&self) -> &str {
        &self.name
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> Result<()> {
        if self.fail_on_init {
            Err(XbergError::Plugin {
                message: "Extractor initialization failed".to_string(),
                plugin_name: self.name.clone(),
            })
        } else {
            Ok(())
        }
    }
    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl InternalDocumentExtractor for FailingExtractor {
    async fn extract_content(
        &self,
        _: &[u8],
        _: &str,
        _: &ExtractionConfig,
    ) -> Result<crate::types::internal::InternalDocument> {
        Ok(crate::types::internal::InternalDocument::new("mock"))
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/plain"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[test]
fn test_document_extractor_initialization_failure_logs_error() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractor = Arc::new(FailingExtractor {
        name: "failing-extractor".to_string(),
        fail_on_init: true,
    });

    let result = registry.register(extractor);
    assert!(result.is_err());
    assert_eq!(registry.list().len(), 0);
}

#[test]
fn test_document_extractor_invalid_name_empty_logs_warning() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractor = Arc::new(MockExtractor {
        name: "".to_string(),
        mime_types: &["text/plain"],
        priority: 50,
    });

    let result = registry.register(extractor);
    assert!(matches!(result, Err(XbergError::Validation { .. })));
}

#[test]
fn test_document_extractor_invalid_name_with_spaces_logs_warning() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractor = Arc::new(MockExtractor {
        name: "invalid extractor".to_string(),
        mime_types: &["text/plain"],
        priority: 50,
    });

    let result = registry.register(extractor);
    assert!(matches!(result, Err(XbergError::Validation { .. })));
}

#[test]
fn test_document_extractor_successful_registration_logs_debug() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractor = Arc::new(MockExtractor {
        name: "valid-pdf-extractor".to_string(),
        mime_types: &["application/pdf"],
        priority: 100,
    });

    let result = registry.register(extractor);
    assert!(result.is_ok());
    assert_eq!(registry.list().len(), 1);
}

#[test]
fn test_document_extractor_remove_nonexistent_logs_debug() {
    let mut registry = DocumentExtractorRegistry::new();

    let result = registry.remove("nonexistent-extractor");
    assert!(result.is_ok());
    assert_eq!(registry.list().len(), 0);
}

#[test]
fn test_document_extractor_shutdown_empty_registry() {
    let mut registry = DocumentExtractorRegistry::new();
    let result = registry.shutdown_all();
    assert!(result.is_ok());
    assert_eq!(registry.list().len(), 0);
}

#[test]
fn test_document_extractor_shutdown_with_multiple_extractors() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractor1 = Arc::new(MockExtractor {
        name: "extractor1".to_string(),
        mime_types: &["text/plain"],
        priority: 50,
    });

    let extractor2 = Arc::new(MockExtractor {
        name: "extractor2".to_string(),
        mime_types: &["application/pdf"],
        priority: 100,
    });

    let extractor3 = Arc::new(MockExtractor {
        name: "extractor3".to_string(),
        mime_types: &["image/png"],
        priority: 75,
    });

    registry.register(extractor1).unwrap();
    registry.register(extractor2).unwrap();
    registry.register(extractor3).unwrap();

    assert_eq!(registry.list().len(), 3);

    registry.shutdown_all().unwrap();
    assert_eq!(registry.list().len(), 0);
}

/// #216: two different extractors claiming the same MIME type at the same
/// priority must not silently overwrite one another with no trace — the
/// later registration should win the slot (documented, not accidental) and
/// the collision must be observable, not merely inferred from which one
/// answers `get()`.
#[test]
fn test_document_extractor_priority_collision_last_registration_wins_slot() {
    let mut registry = DocumentExtractorRegistry::new();

    let first = Arc::new(MockExtractor {
        name: "collider-a".to_string(),
        mime_types: &["application/x-collision"],
        priority: 50,
    });
    let second = Arc::new(MockExtractor {
        name: "collider-b".to_string(),
        mime_types: &["application/x-collision"],
        priority: 50,
    });

    registry.register(first).unwrap();
    registry.register(second).unwrap();

    // "collider-a" had only this one (MIME type, priority) slot, and "collider-b"
    // took it over entirely, so "collider-a" is left with nothing registered at
    // all — `list()` must not keep it around as an unreachable zombie entry.
    let names = registry.list();
    assert_eq!(names, vec!["collider-b".to_string()]);

    // The later registration answers the shared slot.
    assert_eq!(registry.get("application/x-collision").unwrap().name(), "collider-b");
}

/// #216: re-registering under a name that is already registered for a
/// *different* MIME type must not orphan the old MIME entry. Before the
/// fix, `name_index.insert` overwrote the old index entries wholesale, so
/// `remove("dup-extractor")` could never reach "text/plain" again and the
/// stale registration stayed in `get()` forever.
#[test]
fn test_document_extractor_duplicate_name_reregistration_does_not_orphan_old_mime() {
    let mut registry = DocumentExtractorRegistry::new();

    let first = Arc::new(MockExtractor {
        name: "dup-extractor".to_string(),
        mime_types: &["text/plain"],
        priority: 50,
    });
    registry.register(first).unwrap();
    assert!(registry.get("text/plain").is_ok());

    let second = Arc::new(MockExtractor {
        name: "dup-extractor".to_string(),
        mime_types: &["application/pdf"],
        priority: 50,
    });
    registry.register(second).unwrap();

    // The re-registration only claims application/pdf; text/plain must be
    // fully released, not orphaned as an unreachable zombie entry.
    assert!(registry.get("text/plain").is_err());
    assert_eq!(registry.get("application/pdf").unwrap().name(), "dup-extractor");

    // remove() must now be able to fully clean up the current registration.
    registry.remove("dup-extractor").unwrap();
    assert!(registry.get("application/pdf").is_err());
    assert_eq!(registry.list().len(), 0);
}

struct CanHandleExtractor {
    name: String,
    mime_types: &'static [&'static str],
    priority: i32,
    can_handle_result: bool,
}

impl Plugin for CanHandleExtractor {
    fn name(&self) -> &str {
        &self.name
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl InternalDocumentExtractor for CanHandleExtractor {
    async fn extract_content(
        &self,
        _: &[u8],
        _: &str,
        _: &ExtractionConfig,
    ) -> Result<crate::types::internal::InternalDocument> {
        Ok(crate::types::internal::InternalDocument::new("mock"))
    }

    fn supported_mime_types(&self) -> &[&str] {
        self.mime_types
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn can_handle(&self, _path: &std::path::Path, _mime_type: &str) -> bool {
        self.can_handle_result
    }
}

/// #217: `get_candidates` must return every registered match for a MIME type
/// in priority order (highest first), so a caller can fall back when the
/// first one fails.
#[test]
fn test_get_candidates_orders_by_priority_descending() {
    let mut registry = DocumentExtractorRegistry::new();
    registry
        .register(Arc::new(MockExtractor {
            name: "low".to_string(),
            mime_types: &["application/pdf"],
            priority: 10,
        }))
        .unwrap();
    registry
        .register(Arc::new(MockExtractor {
            name: "high".to_string(),
            mime_types: &["application/pdf"],
            priority: 90,
        }))
        .unwrap();
    registry
        .register(Arc::new(MockExtractor {
            name: "mid".to_string(),
            mime_types: &["application/pdf"],
            priority: 50,
        }))
        .unwrap();

    let candidates = registry.get_candidates(std::path::Path::new("f.pdf"), "application/pdf");
    let names: Vec<&str> = candidates.iter().map(|c| c.plugin().name()).collect();
    assert_eq!(names, vec!["high", "mid", "low"]);
}

/// #217: an extractor whose `can_handle` declines this specific file must be
/// excluded from the candidate list entirely, even though it declares the
/// MIME type — it must never be "selected and left to fail".
#[test]
fn test_get_candidates_excludes_extractor_that_declines_can_handle() {
    let mut registry = DocumentExtractorRegistry::new();
    registry
        .register(Arc::new(CanHandleExtractor {
            name: "picky".to_string(),
            mime_types: &["application/pdf"],
            priority: 90,
            can_handle_result: false,
        }))
        .unwrap();
    registry
        .register(Arc::new(MockExtractor {
            name: "generic".to_string(),
            mime_types: &["application/pdf"],
            priority: 50,
        }))
        .unwrap();

    let candidates = registry.get_candidates(std::path::Path::new("f.pdf"), "application/pdf");
    let names: Vec<&str> = candidates.iter().map(|c| c.plugin().name()).collect();
    assert_eq!(
        names,
        vec!["generic"],
        "the can_handle=false extractor must be excluded, not just deprioritized"
    );
}

#[test]
fn test_get_candidates_empty_when_nothing_registered() {
    let registry = DocumentExtractorRegistry::new();
    assert!(
        registry
            .get_candidates(std::path::Path::new("f.pdf"), "application/pdf")
            .is_empty()
    );
}

#[test]
fn test_document_extractor_priority_ordering_complex() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractors = vec![
        (
            Arc::new(MockExtractor {
                name: "priority-1".to_string(),
                mime_types: &["application/pdf"],
                priority: 1,
            }),
            1,
        ),
        (
            Arc::new(MockExtractor {
                name: "priority-100".to_string(),
                mime_types: &["application/pdf"],
                priority: 100,
            }),
            100,
        ),
        (
            Arc::new(MockExtractor {
                name: "priority-50".to_string(),
                mime_types: &["application/pdf"],
                priority: 50,
            }),
            50,
        ),
    ];

    for (extractor, _priority) in &extractors {
        registry.register(extractor.clone()).unwrap();
    }

    let retrieved = registry.get("application/pdf").unwrap();
    assert_eq!(retrieved.name(), "priority-100");
}
