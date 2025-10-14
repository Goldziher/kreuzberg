use super::error::{PdfError, Result};
use lopdf::{Document, Object};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for PdfMetadata {
    fn default() -> Self {
        Self {
            title: None,
            subject: None,
            authors: None,
            keywords: None,
            created_at: None,
            modified_at: None,
            created_by: None,
            producer: None,
            page_count: None,
            pdf_version: None,
            is_encrypted: None,
            width: None,
            height: None,
            summary: None,
        }
    }
}

pub fn extract_metadata(pdf_bytes: &[u8]) -> Result<PdfMetadata> {
    extract_metadata_with_password(pdf_bytes, None)
}

pub fn extract_metadata_with_password(pdf_bytes: &[u8], password: Option<&str>) -> Result<PdfMetadata> {
    let mut doc = Document::load_mem(pdf_bytes)
        .map_err(|e| PdfError::MetadataExtractionFailed(format!("Failed to load PDF: {}", e)))?;

    if doc.is_encrypted() {
        if let Some(pwd) = password {
            doc.decrypt(pwd).map_err(|_| PdfError::InvalidPassword)?;
        } else {
            return Err(PdfError::PasswordRequired);
        }
    }

    let mut metadata = PdfMetadata::default();

    metadata.pdf_version = Some(doc.version.clone());
    metadata.is_encrypted = Some(doc.is_encrypted());
    metadata.page_count = Some(doc.get_pages().len());

    if let Ok(info_ref) = doc.trailer.get(b"Info").and_then(Object::as_reference) {
        if let Ok(info_dict) = doc.get_dictionary(info_ref) {
            extract_info_dictionary(info_dict, &mut metadata);
        }
    }

    if let Some(page_id) = doc.get_pages().values().next() {
        if let Ok(page_dict) = doc.get_dictionary(*page_id) {
            extract_page_dimensions(page_dict, &mut metadata);
        }
    }

    if metadata.summary.is_none() {
        metadata.summary = Some(generate_summary(&metadata));
    }

    Ok(metadata)
}

pub fn extract_metadata_with_passwords(pdf_bytes: &[u8], passwords: &[&str]) -> Result<PdfMetadata> {
    let mut last_error = None;

    for password in passwords {
        match extract_metadata_with_password(pdf_bytes, Some(password)) {
            Ok(metadata) => return Ok(metadata),
            Err(e) => {
                last_error = Some(e);
                continue;
            }
        }
    }

    if let Some(err) = last_error {
        return Err(err);
    }

    extract_metadata(pdf_bytes)
}

fn extract_info_dictionary(info_dict: &lopdf::Dictionary, metadata: &mut PdfMetadata) {
    if let Ok(title) = info_dict.get(b"Title") {
        if let Ok(title_str) = decode_pdf_string(title) {
            metadata.title = Some(title_str);
        }
    }

    if let Ok(subject) = info_dict.get(b"Subject") {
        if let Ok(subject_str) = decode_pdf_string(subject) {
            metadata.subject = Some(subject_str);
        }
    }

    if let Ok(author) = info_dict.get(b"Author") {
        if let Ok(author_str) = decode_pdf_string(author) {
            let authors = parse_authors(&author_str);
            if !authors.is_empty() {
                metadata.authors = Some(authors);
            }
        }
    }

    if let Ok(keywords) = info_dict.get(b"Keywords") {
        if let Ok(keywords_str) = decode_pdf_string(keywords) {
            let kw_list = parse_keywords(&keywords_str);
            if !kw_list.is_empty() {
                metadata.keywords = Some(kw_list);
            }
        }
    }

    if let Ok(created) = info_dict.get(b"CreationDate") {
        if let Ok(date_str) = decode_pdf_string(created) {
            metadata.created_at = Some(parse_pdf_date(&date_str));
        }
    }

    if let Ok(modified) = info_dict.get(b"ModDate") {
        if let Ok(date_str) = decode_pdf_string(modified) {
            metadata.modified_at = Some(parse_pdf_date(&date_str));
        }
    }

    if let Ok(creator) = info_dict.get(b"Creator") {
        if let Ok(creator_str) = decode_pdf_string(creator) {
            metadata.created_by = Some(creator_str);
        }
    }

    if let Ok(producer) = info_dict.get(b"Producer") {
        if let Ok(producer_str) = decode_pdf_string(producer) {
            metadata.producer = Some(producer_str);
        }
    }
}

fn extract_page_dimensions(page_dict: &lopdf::Dictionary, metadata: &mut PdfMetadata) {
    if let Ok(media_box) = page_dict.get(b"MediaBox").and_then(Object::as_array) {
        if media_box.len() >= 4 {
            let width = match &media_box[2] {
                Object::Integer(i) => Some(*i),
                Object::Real(f) => Some(*f as i64),
                _ => None,
            };
            let height = match &media_box[3] {
                Object::Integer(i) => Some(*i),
                Object::Real(f) => Some(*f as i64),
                _ => None,
            };
            if let (Some(w), Some(h)) = (width, height) {
                metadata.width = Some(w);
                metadata.height = Some(h);
            }
        }
    }
}

fn decode_pdf_string(obj: &Object) -> lopdf::Result<String> {
    match obj {
        Object::String(bytes, _) => {
            if bytes.starts_with(&[0xFE, 0xFF]) {
                let utf16_bytes: Vec<u16> = bytes[2..]
                    .chunks_exact(2)
                    .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                    .collect();
                Ok(String::from_utf16_lossy(&utf16_bytes))
            } else {
                Ok(String::from_utf8_lossy(bytes).to_string())
            }
        }
        Object::Name(bytes) => Ok(String::from_utf8_lossy(bytes).to_string()),
        _ => Err(lopdf::Error::DictType {
            expected: "String or Name",
            found: String::from_utf8_lossy(obj.type_name().unwrap_or(b"Unknown")).to_string(),
        }),
    }
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
    let cleaned = date_str.trim_start_matches("D:");

    if cleaned.len() >= 8 {
        let year = &cleaned[0..4];
        let month = &cleaned[4..6];
        let day = &cleaned[6..8];

        if cleaned.len() >= 14 {
            let hour = &cleaned[8..10];
            let minute = &cleaned[10..12];
            let second = &cleaned[12..14];
            format!("{}-{}-{}T{}:{}:{}Z", year, month, day, hour, minute, second)
        } else {
            format!("{}-{}-{}T00:00:00Z", year, month, day)
        }
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
        let mut metadata = PdfMetadata::default();
        metadata.page_count = Some(10);
        metadata.pdf_version = Some("1.7".to_string());
        metadata.is_encrypted = Some(false);

        let summary = generate_summary(&metadata);
        assert!(summary.contains("10 pages"));
        assert!(summary.contains("1.7"));
        assert!(!summary.contains("encrypted"));
    }

    #[test]
    fn test_generate_summary_single_page() {
        let mut metadata = PdfMetadata::default();
        metadata.page_count = Some(1);

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
