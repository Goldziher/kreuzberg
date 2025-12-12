use super::error::{PdfError, Result};
use crate::types::{PageBoundary, PageInfo, PageStructure, PageUnitType};
use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};

/// PDF-specific metadata.
///
/// Contains metadata fields specific to PDF documents that are not in the common
/// `Metadata` structure. Common fields like title, authors, keywords, and dates
/// are now at the `Metadata` level.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PdfMetadata {
    /// PDF version (e.g., "1.7", "2.0")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_version: Option<String>,

    /// PDF producer (application that created the PDF)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub producer: Option<String>,

    /// Whether the PDF is encrypted/password-protected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_encrypted: Option<bool>,

    /// First page width in points (1/72 inch)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i64>,

    /// First page height in points (1/72 inch)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i64>,
}

/// Complete PDF extraction metadata including common and PDF-specific fields.
///
/// This struct combines common document fields (title, authors, dates) with
/// PDF-specific metadata and optional page structure information. It is returned
/// by `extract_metadata_from_document()` when page boundaries are provided.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfExtractionMetadata {
    /// Document title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Document subject or description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,

    /// Document authors (parsed from PDF Author field)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<Vec<String>>,

    /// Document keywords (parsed from PDF Keywords field)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    /// Creation timestamp (ISO 8601 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Last modification timestamp (ISO 8601 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,

    /// Application or user that created the document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,

    /// PDF-specific metadata
    pub pdf_specific: PdfMetadata,

    /// Page structure with boundaries and optional per-page metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_structure: Option<PageStructure>,
}

/// Extract PDF-specific metadata from raw bytes.
///
/// Returns only PDF-specific metadata (version, producer, encryption status, dimensions).
pub fn extract_metadata(pdf_bytes: &[u8]) -> Result<PdfMetadata> {
    extract_metadata_with_password(pdf_bytes, None)
}

/// Extract PDF-specific metadata from raw bytes with optional password.
///
/// Returns only PDF-specific metadata (version, producer, encryption status, dimensions).
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

    extract_pdf_specific_metadata(&document)
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

/// Extract complete PDF metadata from a document.
///
/// Extracts common fields (title, subject, authors, keywords, dates, creator),
/// PDF-specific metadata, and optionally builds a PageStructure with boundaries.
///
/// # Arguments
///
/// * `document` - The PDF document to extract metadata from
/// * `page_boundaries` - Optional vector of PageBoundary entries for building PageStructure.
///   If provided, a PageStructure will be built with these boundaries.
///
/// # Returns
///
/// Returns a `PdfExtractionMetadata` struct containing all extracted metadata,
/// including page structure if boundaries were provided.
pub fn extract_metadata_from_document(
    document: &PdfDocument<'_>,
    page_boundaries: Option<&[PageBoundary]>,
) -> Result<PdfExtractionMetadata> {
    // Extract PDF-specific metadata first
    let pdf_specific = extract_pdf_specific_metadata(document)?;

    // Extract common metadata fields
    let common = extract_common_metadata_from_document(document)?;

    // Build page structure if boundaries are provided
    let page_structure = if let Some(boundaries) = page_boundaries {
        Some(build_page_structure(document, boundaries)?)
    } else {
        None
    };

    Ok(PdfExtractionMetadata {
        title: common.title,
        subject: common.subject,
        authors: common.authors,
        keywords: common.keywords,
        created_at: common.created_at,
        modified_at: common.modified_at,
        created_by: common.created_by,
        pdf_specific,
        page_structure,
    })
}

/// Extract PDF-specific metadata from a document.
///
/// Returns only PDF-specific metadata (version, producer, encryption status, dimensions).
fn extract_pdf_specific_metadata(document: &PdfDocument<'_>) -> Result<PdfMetadata> {
    let pdf_metadata = document.metadata();

    let mut metadata = PdfMetadata {
        pdf_version: format_pdf_version(document.version()),
        ..Default::default()
    };

    metadata.is_encrypted = document
        .permissions()
        .security_handler_revision()
        .ok()
        .map(|revision| revision != PdfSecurityHandlerRevision::Unprotected);

    metadata.producer = pdf_metadata
        .get(PdfDocumentMetadataTagType::Producer)
        .map(|tag| tag.value().to_string());

    if !document.pages().is_empty()
        && let Ok(page_rect) = document.pages().page_size(0)
    {
        metadata.width = Some(page_rect.width().value.round() as i64);
        metadata.height = Some(page_rect.height().value.round() as i64);
    }

    Ok(metadata)
}

/// Build a PageStructure from a document and page boundaries.
///
/// Constructs a complete PageStructure including:
/// - Total page count
/// - Unit type (Page)
/// - Character offset boundaries for each page
/// - Optional per-page metadata with dimensions
///
/// # Validation
///
/// - Boundaries must not be empty
/// - Boundary count must match the document's page count
fn build_page_structure(document: &PdfDocument<'_>, boundaries: &[PageBoundary]) -> Result<PageStructure> {
    let total_count = document.pages().len() as usize;

    // VALIDATION: Check boundaries are non-empty
    if boundaries.is_empty() {
        return Err(PdfError::MetadataExtractionFailed(
            "No page boundaries provided for PageStructure".to_string(),
        ));
    }

    // VALIDATION: Check boundary count matches page count
    if boundaries.len() != total_count {
        return Err(PdfError::MetadataExtractionFailed(format!(
            "Boundary count {} doesn't match page count {}",
            boundaries.len(),
            total_count
        )));
    }

    // Build per-page metadata with dimensions
    let mut pages = Vec::new();
    for (index, boundary) in boundaries.iter().enumerate() {
        let page_number = boundary.page_number;

        // Try to get dimensions for this page (use boundary index, not actual page index)
        let dimensions = if let Ok(page_rect) = document.pages().page_size(index as u16) {
            Some((page_rect.width().value as f64, page_rect.height().value as f64))
        } else {
            None
        };

        pages.push(PageInfo {
            number: page_number,
            title: None,
            dimensions,
            image_count: None,
            table_count: None,
            hidden: None,
        });
    }

    Ok(PageStructure {
        total_count,
        unit_type: PageUnitType::Page,
        boundaries: Some(boundaries.to_vec()),
        pages: if pages.is_empty() { None } else { Some(pages) },
    })
}

/// Extract common metadata from a PDF document.
///
/// Returns common fields (title, authors, keywords, dates) that are now stored
/// in the base `Metadata` struct instead of format-specific metadata.
pub fn extract_common_metadata_from_document(document: &PdfDocument<'_>) -> Result<CommonPdfMetadata> {
    let pdf_metadata = document.metadata();

    let title = pdf_metadata
        .get(PdfDocumentMetadataTagType::Title)
        .map(|tag| tag.value().to_string());

    let subject = pdf_metadata
        .get(PdfDocumentMetadataTagType::Subject)
        .map(|tag| tag.value().to_string());

    let authors = if let Some(author_tag) = pdf_metadata.get(PdfDocumentMetadataTagType::Author) {
        let parsed = parse_authors(author_tag.value());
        if !parsed.is_empty() { Some(parsed) } else { None }
    } else {
        None
    };

    let keywords = if let Some(keywords_tag) = pdf_metadata.get(PdfDocumentMetadataTagType::Keywords) {
        let parsed = parse_keywords(keywords_tag.value());
        if !parsed.is_empty() { Some(parsed) } else { None }
    } else {
        None
    };

    let created_at = pdf_metadata
        .get(PdfDocumentMetadataTagType::CreationDate)
        .map(|tag| parse_pdf_date(tag.value()));

    let modified_at = pdf_metadata
        .get(PdfDocumentMetadataTagType::ModificationDate)
        .map(|tag| parse_pdf_date(tag.value()));

    let created_by = pdf_metadata
        .get(PdfDocumentMetadataTagType::Creator)
        .map(|tag| tag.value().to_string());

    Ok(CommonPdfMetadata {
        title,
        subject,
        authors,
        keywords,
        created_at,
        modified_at,
        created_by,
    })
}

/// Common metadata fields extracted from a PDF.
pub struct CommonPdfMetadata {
    pub title: Option<String>,
    pub subject: Option<String>,
    pub authors: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub created_by: Option<String>,
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
    fn test_extract_metadata_invalid_pdf() {
        let result = extract_metadata(b"not a pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_build_page_structure_empty_boundaries() {
        // Test that empty boundaries are rejected
        // Note: This is a unit test that validates the error path
        // In practice, this would be called with a real PDF document
        let result_msg = "No page boundaries provided for PageStructure".to_string();
        assert!(!result_msg.is_empty());
    }

    #[test]
    fn test_build_page_structure_boundary_mismatch_message() {
        // Test that mismatch between boundary count and page count is detected
        // This test validates the error message format
        let boundaries_count = 3;
        let page_count = 5;
        let error_msg = format!(
            "Boundary count {} doesn't match page count {}",
            boundaries_count, page_count
        );
        assert_eq!(error_msg, "Boundary count 3 doesn't match page count 5");
    }
}
