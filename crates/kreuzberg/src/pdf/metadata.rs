use super::error::{PdfError, Result};
use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PdfMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub producer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_encrypted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

pub fn extract_metadata(pdf_bytes: &[u8]) -> Result<PdfMetadata> {
    extract_metadata_with_password(pdf_bytes, None)
}

pub fn extract_metadata_with_password(pdf_bytes: &[u8], password: Option<&str>) -> Result<PdfMetadata> {
    let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
        .or_else(|_| Pdfium::bind_to_system_library())
        .map_err(|e| PdfError::MetadataExtractionFailed(format!("Failed to initialize Pdfium: {}", e)))?;

    let pdfium = Pdfium::new(bindings);

    let document = pdfium.load_pdf_from_byte_slice(pdf_bytes, password).map_err(|e| {
        let err_msg = e.to_string();
        if (err_msg.contains("password") || err_msg.contains("Password")) && password.is_some() {
            PdfError::InvalidPassword
        } else if err_msg.contains("password") || err_msg.contains("Password") {
            PdfError::PasswordRequired
        } else {
            PdfError::MetadataExtractionFailed(err_msg)
        }
    })?;

    extract_metadata_from_document(&document)
}

pub fn extract_metadata_with_passwords(pdf_bytes: &[u8], passwords: &[&str]) -> Result<PdfMetadata> {
    let mut last_error = None;

    for password in passwords {
        match extract_metadata_with_password(pdf_bytes, Some(password)) {
            Ok(metadata) => return Ok(metadata),
            Err(err) => {
                last_error = Some(err);
                continue;
            }
        }
    }

    if let Some(err) = last_error {
        return Err(err);
    }

    extract_metadata(pdf_bytes)
}

pub(crate) fn extract_metadata_from_document(document: &PdfDocument<'_>) -> Result<PdfMetadata> {
    let pdf_metadata = document.metadata();

    let mut metadata = PdfMetadata {
        pdf_version: format_pdf_version(document.version()),
        ..Default::default()
    };
    metadata.page_count = Some(document.pages().len() as usize);
    metadata.is_encrypted = document
        .permissions()
        .security_handler_revision()
        .ok()
        .map(|revision| revision != PdfSecurityHandlerRevision::Unprotected);

    metadata.title = pdf_metadata
        .get(PdfDocumentMetadataTagType::Title)
        .map(|tag| tag.value().to_string());

    metadata.subject = pdf_metadata
        .get(PdfDocumentMetadataTagType::Subject)
        .map(|tag| tag.value().to_string());

    if let Some(author_tag) = pdf_metadata.get(PdfDocumentMetadataTagType::Author) {
        let authors = parse_authors(author_tag.value());
        if !authors.is_empty() {
            metadata.authors = Some(authors);
        }
    }

    if let Some(keywords_tag) = pdf_metadata.get(PdfDocumentMetadataTagType::Keywords) {
        let keywords = parse_keywords(keywords_tag.value());
        if !keywords.is_empty() {
            metadata.keywords = Some(keywords);
        }
    }

    if let Some(created_tag) = pdf_metadata.get(PdfDocumentMetadataTagType::CreationDate) {
        metadata.created_at = Some(parse_pdf_date(created_tag.value()));
    }

    if let Some(modified_tag) = pdf_metadata.get(PdfDocumentMetadataTagType::ModificationDate) {
        metadata.modified_at = Some(parse_pdf_date(modified_tag.value()));
    }

    metadata.created_by = pdf_metadata
        .get(PdfDocumentMetadataTagType::Creator)
        .map(|tag| tag.value().to_string());

    metadata.producer = pdf_metadata
        .get(PdfDocumentMetadataTagType::Producer)
        .map(|tag| tag.value().to_string());

    if !document.pages().is_empty()
        && let Ok(page_rect) = document.pages().page_size(0)
    {
        metadata.width = Some(page_rect.width().value.round() as i64);
        metadata.height = Some(page_rect.height().value.round() as i64);
    }

    if metadata.summary.is_none() {
        metadata.summary = Some(generate_summary(&metadata));
    }

    Ok(metadata)
}

fn parse_authors(author_str: &str) -> Vec<String> {
    let author_str = author_str.replace(" and ", ", ");
    let mut authors = Vec::new();

    for segment in author_str.split(';') {
        for author in segment.split(',') {
            let trimmed = author.trim();
            if !trimmed.is_empty() {
                authors.push(trimmed.to_string());
            }
        }
    }

    authors
}

fn parse_keywords(keywords_str: &str) -> Vec<String> {
    keywords_str
        .replace(';', ",")
        .split(',')
        .filter_map(|k| {
            let trimmed = k.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}

fn parse_pdf_date(date_str: &str) -> String {
    let cleaned = date_str.trim();

    if cleaned.starts_with("D:") && cleaned.len() >= 10 {
        let year = &cleaned[2..6];
        let month = &cleaned[6..8];
        let day = &cleaned[8..10];

        if cleaned.len() >= 16 {
            let hour = &cleaned[10..12];
            let minute = &cleaned[12..14];
            let second = &cleaned[14..16];
            format!("{}-{}-{}T{}:{}:{}Z", year, month, day, hour, minute, second)
        } else if cleaned.len() >= 14 {
            let hour = &cleaned[10..12];
            let minute = &cleaned[12..14];
            format!("{}-{}-{}T{}:{}:00Z", year, month, day, hour, minute)
        } else {
            format!("{}-{}-{}T00:00:00Z", year, month, day)
        }
    } else if cleaned.len() >= 8 {
        let year = &cleaned[0..4];
        let month = &cleaned[4..6];
        let day = &cleaned[6..8];
        format!("{}-{}-{}T00:00:00Z", year, month, day)
    } else {
        date_str.to_string()
    }
}

fn generate_summary(metadata: &PdfMetadata) -> String {
    let mut parts = Vec::new();

    if let Some(page_count) = metadata.page_count {
        let plural = if page_count != 1 { "s" } else { "" };
        parts.push(format!("PDF document with {} page{}.", page_count, plural));
    }

    if let Some(ref version) = metadata.pdf_version {
        parts.push(format!("PDF version {}.", version));
    }

    if metadata.is_encrypted == Some(true) {
        parts.push("Document is encrypted.".to_string());
    }

    parts.join(" ")
}

fn format_pdf_version(version: PdfDocumentVersion) -> Option<String> {
    match version {
        PdfDocumentVersion::Unset => None,
        PdfDocumentVersion::Pdf1_0 => Some("1.0".to_string()),
        PdfDocumentVersion::Pdf1_1 => Some("1.1".to_string()),
        PdfDocumentVersion::Pdf1_2 => Some("1.2".to_string()),
        PdfDocumentVersion::Pdf1_3 => Some("1.3".to_string()),
        PdfDocumentVersion::Pdf1_4 => Some("1.4".to_string()),
        PdfDocumentVersion::Pdf1_5 => Some("1.5".to_string()),
        PdfDocumentVersion::Pdf1_6 => Some("1.6".to_string()),
        PdfDocumentVersion::Pdf1_7 => Some("1.7".to_string()),
        PdfDocumentVersion::Pdf2_0 => Some("2.0".to_string()),
        PdfDocumentVersion::Other(value) => {
            if value >= 10 {
                Some(format!("{}.{}", value / 10, value % 10))
            } else {
                Some(value.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_authors_single() {
        let authors = parse_authors("John Doe");
        assert_eq!(authors, vec!["John Doe"]);
    }

    #[test]
    fn test_parse_authors_multiple_comma() {
        let authors = parse_authors("John Doe, Jane Smith");
        assert_eq!(authors, vec!["John Doe", "Jane Smith"]);
    }

    #[test]
    fn test_parse_authors_multiple_and() {
        let authors = parse_authors("John Doe and Jane Smith");
        assert_eq!(authors, vec!["John Doe", "Jane Smith"]);
    }

    #[test]
    fn test_parse_authors_semicolon() {
        let authors = parse_authors("John Doe;Jane Smith");
        assert_eq!(authors, vec!["John Doe", "Jane Smith"]);
    }

    #[test]
    fn test_parse_keywords() {
        let keywords = parse_keywords("pdf, document, test");
        assert_eq!(keywords, vec!["pdf", "document", "test"]);
    }

    #[test]
    fn test_parse_keywords_semicolon() {
        let keywords = parse_keywords("pdf;document;test");
        assert_eq!(keywords, vec!["pdf", "document", "test"]);
    }

    #[test]
    fn test_parse_keywords_empty() {
        let keywords = parse_keywords("");
        assert!(keywords.is_empty());
    }

    #[test]
    fn test_parse_pdf_date_full() {
        let date = parse_pdf_date("D:20230115123045");
        assert_eq!(date, "2023-01-15T12:30:45Z");
    }

    #[test]
    fn test_parse_pdf_date_no_time() {
        let date = parse_pdf_date("D:20230115");
        assert_eq!(date, "2023-01-15T00:00:00Z");
    }

    #[test]
    fn test_parse_pdf_date_no_prefix() {
        let date = parse_pdf_date("20230115");
        assert_eq!(date, "2023-01-15T00:00:00Z");
    }

    #[test]
    fn test_generate_summary() {
        let metadata = PdfMetadata {
            page_count: Some(10),
            pdf_version: Some("1.7".to_string()),
            is_encrypted: Some(false),
            ..Default::default()
        };

        let summary = generate_summary(&metadata);
        assert!(summary.contains("10 pages"));
        assert!(summary.contains("1.7"));
        assert!(!summary.contains("encrypted"));
    }

    #[test]
    fn test_generate_summary_single_page() {
        let metadata = PdfMetadata {
            page_count: Some(1),
            ..Default::default()
        };

        let summary = generate_summary(&metadata);
        assert!(summary.contains("1 page."));
        assert!(!summary.contains("pages"));
    }

    #[test]
    fn test_extract_metadata_invalid_pdf() {
        let result = extract_metadata(b"not a pdf");
        assert!(result.is_err());
    }
}
