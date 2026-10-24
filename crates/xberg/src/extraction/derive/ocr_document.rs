//! ~keep Validate a backend's public structure before it enters the OCR composition pipeline.

use crate::extractors::security::SecurityLimits;
use crate::types::ExtractedDocument;
use crate::types::document_structure::{DocumentNode, DocumentStructure, NodeContent, TextAnnotation};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement, Relationship, RelationshipTarget};

#[derive(Debug)]
struct InvalidPublicOcrDocument;

impl std::fmt::Display for InvalidPublicOcrDocument {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("public OCR document validation")
    }
}

impl std::error::Error for InvalidPublicOcrDocument {}

fn invalid_document(message: &str) -> crate::XbergError {
    crate::XbergError::Validation {
        message: format!("Invalid structured OCR document: {message}"),
        source: Some(Box::new(InvalidPublicOcrDocument)),
    }
}

/// ~keep Only invalid public contributions abort stage fallback; ordinary backend
/// validation errors retain the pipeline's existing backend-failure policy.
#[cfg(feature = "pdf")]
pub(crate) fn is_invalid_public_ocr_document(error: &crate::XbergError) -> bool {
    matches!(error, crate::XbergError::Validation { source: Some(source), .. } if source.is::<InvalidPublicOcrDocument>())
}

/// ~keep Explicit nodes are adopted without interpreting a string-only transcript as markup.
pub(crate) fn adopt_public_ocr_document(result: &mut ExtractedDocument, limits: &SecurityLimits) -> crate::Result<()> {
    let Some(structure) = result.document.as_ref() else {
        return Ok(());
    };
    validate_payload_fields(result)?;
    if structure.nodes.is_empty() && !result.content.trim().is_empty() {
        return Err(invalid_document(
            "a nonempty transcript requires a nonempty structured contribution",
        ));
    }
    validate_metadata(result, structure, limits)?;
    validate_structure(structure)?;
    validate_serialized_payload_size(result, limits)?;
    let mut doc = InternalDocument::new(structure.source_format.as_deref().unwrap_or("ocr"));
    for node in &structure.nodes {
        doc.push_element(convert_node(node)?);
    }
    for relationship in &structure.relationships {
        if relationship.source.0 as usize >= doc.elements.len() || relationship.target.0 as usize >= doc.elements.len()
        {
            return Err(invalid_document("relationship references a missing node"));
        }
        doc.relationships.push(Relationship {
            source: relationship.source.0,
            target: RelationshipTarget::Index(relationship.target.0),
            kind: relationship.kind,
        });
    }
    result.ocr_internal_document = Some(doc);
    Ok(())
}

fn validate_metadata(
    result: &ExtractedDocument,
    structure: &DocumentStructure,
    limits: &SecurityLimits,
) -> crate::Result<()> {
    let mut metadata = MetadataTraversal::new(limits);
    if structure.nodes.len() > u32::MAX as usize {
        return Err(invalid_document("node count exceeds supported index range"));
    }
    metadata.charge(structure.nodes.len())?;
    metadata.charge(structure.relationships.len())?;
    for node in &structure.nodes {
        metadata.charge(node.annotations.len())?;
    }
    metadata.json_values(
        result
            .metadata
            .additional
            .values()
            .chain(result.metadata.json_schema.iter()),
    )?;
    #[cfg(feature = "office")]
    if let Some(crate::types::FormatMetadata::Docx(docx)) = result.metadata.format.as_ref()
        && let Some(properties) = docx.custom_properties.as_ref()
    {
        metadata.json_values(properties.values())?;
    }
    if let Some(elements) = result.ocr_elements.as_ref() {
        metadata.charge(elements.len())?;
        for element in elements {
            metadata.json_values(element.backend_metadata.values())?;
        }
    }
    #[cfg(feature = "tree-sitter")]
    if let Some(crate::types::FormatMetadata::Code(code)) = result.metadata.format.as_ref()
        && let Some(data) = code.data.as_ref()
    {
        metadata.enqueue(MetadataNode::Code(data), 1)?;
    }
    metadata.validate()?;
    Ok(())
}

fn validate_structure(structure: &DocumentStructure) -> crate::Result<()> {
    // ~keep Flatness is checked before tree membership validation, which can be quadratic on wide trees.
    if structure
        .nodes
        .iter()
        .any(|node| node.parent.is_some() || !node.children.is_empty())
    {
        return Err(invalid_document("nested OCR nodes are not supported"));
    }
    structure.validate().map_err(|message| invalid_document(&message))
}

fn validate_serialized_payload_size(result: &ExtractedDocument, limits: &SecurityLimits) -> crate::Result<()> {
    struct Budget(usize);
    impl std::io::Write for Budget {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_sub(bytes.len())
                .ok_or_else(|| std::io::Error::other("content limit"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(&mut Budget(limits.max_content_size), result)
        .map_err(|_| invalid_document("payload exceeds content limit or cannot be serialized"))
}

fn node_kind_and_text(content: &NodeContent) -> crate::Result<(ElementKind, &str)> {
    Ok(match content {
        NodeContent::Title { text } => (ElementKind::Title, text),
        NodeContent::Heading { level, text } if (1..=6).contains(level) => {
            (ElementKind::Heading { level: *level }, text)
        }
        NodeContent::Paragraph { text } => (ElementKind::Paragraph, text),
        NodeContent::Code { text, .. } => (ElementKind::Code, text),
        NodeContent::Formula { text } => (ElementKind::Formula, text),
        NodeContent::Footnote { text } => (ElementKind::FootnoteDefinition, text),
        NodeContent::Comment { text } => (ElementKind::CommentDefinition, text),
        NodeContent::Citation { text, .. } => (ElementKind::Citation, text),
        NodeContent::RawBlock { format, content } if format == "html" => (ElementKind::RawBlock, content),
        _ => return Err(invalid_document("unsupported OCR node content")),
    })
}

fn validate_node(node: &DocumentNode) -> crate::Result<()> {
    if node
        .attributes
        .as_ref()
        .is_some_and(|attributes| attributes.keys().any(|key| key.starts_with("xberg:internal:")))
    {
        return Err(invalid_document("reserved internal node attributes"));
    }
    if node.page.is_some_and(|page| page != 1) || node.page_end.is_some_and(|page| page != 1) {
        return Err(invalid_document("a backend result must describe one page"));
    }
    if let Some(bbox) = node.bbox
        && (![bbox.x0, bbox.y0, bbox.x1, bbox.y1]
            .iter()
            // ~keep Pixel magnitudes are bounded before a later raster/page scale can amplify them.
            .all(|value| value.is_finite() && value.abs() <= f64::from(u32::MAX))
            || bbox.x0 > bbox.x1
            || bbox.y0 > bbox.y1)
    {
        return Err(invalid_document("invalid node bounding box"));
    }
    Ok(())
}

fn validate_annotations(annotations: &[TextAnnotation], text: &str) -> crate::Result<()> {
    for annotation in annotations {
        let start = annotation.start as usize;
        let end = annotation.end as usize;
        if start > end || end > text.len() || !text.is_char_boundary(start) || !text.is_char_boundary(end) {
            return Err(invalid_document("annotation range is not a valid UTF-8 text span"));
        }
    }
    Ok(())
}

fn convert_node(node: &DocumentNode) -> crate::Result<InternalElement> {
    validate_node(node)?;
    let (kind, text) = node_kind_and_text(&node.content)?;
    validate_annotations(&node.annotations, text)?;
    let mut element = InternalElement::text(kind, text, 0);
    element.page = node.page;
    element.layer = node.content_layer;
    element.bbox = node.bbox;
    element.annotations = node.annotations.clone();
    element.attributes = node
        .attributes
        .as_ref()
        .map(|attributes| attributes.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
    let attributes = element.attributes.get_or_insert_with(Default::default);
    match &node.content {
        NodeContent::RawBlock { format, .. } => {
            attributes.insert("format".into(), format.clone());
        }
        NodeContent::Code {
            language: Some(language),
            ..
        } => {
            attributes.insert("language".into(), language.clone());
        }
        NodeContent::Citation { key, .. } => {
            element.anchor = Some(key.clone());
        }
        _ => {}
    }
    Ok(element)
}

fn validate_payload_fields(result: &ExtractedDocument) -> crate::Result<()> {
    if !result.tables.is_empty()
        || result.images.as_ref().is_some_and(|images| !images.is_empty())
        || result.uris.as_ref().is_some_and(|uris| !uris.is_empty())
    {
        return Err(invalid_document("unsupported OCR asset references"));
    }
    if has_document_outputs(result) || has_enrichment_outputs(result) {
        return Err(invalid_document(
            "unsupported derived OCR output or document-level annotations",
        ));
    }
    #[cfg(feature = "tree-sitter")]
    if result.code_intelligence.is_some() {
        return Err(invalid_document("unsupported derived OCR code intelligence"));
    }
    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    if result.extracted_keywords.is_some() {
        return Err(invalid_document("unsupported derived OCR keywords"));
    }
    #[cfg(feature = "heuristics")]
    if result.extraction_confidence.is_some() {
        return Err(invalid_document("unsupported derived OCR confidence"));
    }
    Ok(())
}

fn has_document_outputs(result: &ExtractedDocument) -> bool {
    result.pages.is_some()
        || result.chunks.is_some()
        || result.elements.is_some()
        || result.djot_content.is_some()
        || result.annotations.is_some()
        || result.children.is_some()
        || result.revisions.is_some()
        || !result.form_fields.is_empty()
}

fn has_enrichment_outputs(result: &ExtractedDocument) -> bool {
    result.structured_output.is_some()
        || result.entities.is_some()
        || result.summary.is_some()
        || result.translation.is_some()
        || result.page_classifications.is_some()
        || result.redaction_report.is_some()
        || result.formatted_content.is_some()
        || result.detected_languages.is_some()
        || result.detected_language_confidences.is_some()
        || result.quality_score.is_some()
        || result.ocr_page_failures.is_some()
}

enum MetadataNode<'a> {
    Json(&'a serde_json::Value),
    #[cfg(feature = "tree-sitter")]
    Code(&'a crate::types::metadata::CodeDataNode),
}

/// ~keep One contribution-wide budget covers structural work, OCR elements and every
/// accepted recursive JSON/code-data node. Roots have logical depth one. Charge
/// before enqueueing so neither breadth nor a new root can reset/exceed the budget.
struct MetadataTraversal<'a> {
    pending: Vec<(MetadataNode<'a>, usize)>,
    remaining: usize,
    max_depth: usize,
}

impl<'a> MetadataTraversal<'a> {
    fn new(limits: &SecurityLimits) -> Self {
        Self {
            pending: Vec::new(),
            remaining: limits.max_iterations,
            max_depth: limits.max_nesting_depth,
        }
    }

    fn charge(&mut self, count: usize) -> crate::Result<()> {
        self.remaining = self
            .remaining
            .checked_sub(count)
            .ok_or_else(|| invalid_document("metadata or structure exceeds security work limit"))?;
        Ok(())
    }

    fn enqueue(&mut self, node: MetadataNode<'a>, depth: usize) -> crate::Result<()> {
        if depth > self.max_depth {
            return Err(invalid_document("metadata exceeds security nesting limit"));
        }
        self.charge(1)?;
        self.pending.push((node, depth));
        Ok(())
    }

    fn json_values(&mut self, values: impl Iterator<Item = &'a serde_json::Value>) -> crate::Result<()> {
        for value in values {
            self.enqueue(MetadataNode::Json(value), 1)?;
        }
        Ok(())
    }

    fn validate(mut self) -> crate::Result<()> {
        while let Some((node, depth)) = self.pending.pop() {
            let child_depth = depth
                .checked_add(1)
                .ok_or_else(|| invalid_document("metadata exceeds security nesting limit"))?;
            match node {
                MetadataNode::Json(serde_json::Value::Array(values)) => {
                    for value in values {
                        self.enqueue(MetadataNode::Json(value), child_depth)?;
                    }
                }
                MetadataNode::Json(serde_json::Value::Object(values)) => {
                    for value in values.values() {
                        self.enqueue(MetadataNode::Json(value), child_depth)?;
                    }
                }
                MetadataNode::Json(_) => {}
                #[cfg(feature = "tree-sitter")]
                MetadataNode::Code(node) => {
                    for child in &node.children {
                        self.enqueue(MetadataNode::Code(child), child_depth)?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(all(test, feature = "tree-sitter"))]
mod recursive_metadata_tests {
    use super::*;

    #[test]
    fn recursive_metadata_code_shallow_is_preserved_at_adoption() {
        let mut result = ExtractedDocument {
            content: "Text".into(),
            document: Some(
                serde_json::from_value(
                    serde_json::json!({"nodes": [{"content": {"node_type": "paragraph", "text": "Text"}}]}),
                )
                .unwrap(),
            ),
            ..Default::default()
        };
        result.metadata.format = Some(crate::types::FormatMetadata::Code(
            crate::types::metadata::CodeMetadata {
                chunks: vec![],
                data: Some(crate::types::metadata::CodeDataNode {
                    kind: crate::types::metadata::CodeDataNodeKind::Sequence,
                    key: None,
                    value: Some("leaf".into()),
                    attributes: vec![],
                    children: vec![],
                    byte_start: 0,
                    byte_end: 1,
                }),
            },
        ));
        let before = serde_json::to_value(&result.metadata.format).unwrap();
        let limits = SecurityLimits {
            max_nesting_depth: 1,
            max_iterations: 2,
            ..Default::default()
        };
        adopt_public_ocr_document(&mut result, &limits).unwrap();
        assert_eq!(serde_json::to_value(&result.metadata.format).unwrap(), before);
        assert!(result.ocr_internal_document.is_some());
    }
}
