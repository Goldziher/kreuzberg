use html_escape::decode_html_entities;
use mail_parser::MimeHeaders;
use once_cell::sync::Lazy;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Email parsing errors with detailed context
#[derive(Error, Debug)]
pub enum EmailError {
    #[error("Failed to parse email: {message}")]
    ParseError { message: String },
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("MSG parser error: {message}")]
    MsgError { message: String },
    #[error("Unsupported email format: {format}")]
    UnsupportedFormat { format: String },
    #[error("Invalid email content: {message}")]
    InvalidContent { message: String },
    #[error("Attachment processing error: {message}")]
    AttachmentError { message: String },
}

impl From<EmailError> for PyErr {
    fn from(err: EmailError) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(err.to_string())
    }
}

/// Email attachment data transfer object
#[derive(Debug, Clone, Serialize, Deserialize)]
#[pyclass]
pub struct EmailAttachmentDTO {
    #[pyo3(get)]
    pub name: Option<String>,
    #[pyo3(get)]
    pub filename: Option<String>,
    #[pyo3(get)]
    pub mime_type: Option<String>,
    #[pyo3(get)]
    pub size: Option<u64>,
    #[pyo3(get)]
    pub is_image: bool,
    #[pyo3(get)]
    pub data: Option<Vec<u8>>,
}

#[pymethods]
impl EmailAttachmentDTO {
    #[new]
    pub fn new(
        name: Option<String>,
        filename: Option<String>,
        mime_type: Option<String>,
        size: Option<u64>,
        is_image: bool,
        data: Option<Vec<u8>>,
    ) -> Self {
        Self {
            name,
            filename,
            mime_type,
            size,
            is_image,
            data,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "EmailAttachment(filename={:?}, mime_type={:?}, size={:?})",
            self.filename, self.mime_type, self.size
        )
    }
}

/// Email extraction result data transfer object
#[derive(Debug, Clone, Serialize, Deserialize)]
#[pyclass]
pub struct EmailExtractionResultDTO {
    #[pyo3(get)]
    pub subject: Option<String>,
    #[pyo3(get)]
    pub from_email: Option<String>,
    #[pyo3(get)]
    pub to_emails: Vec<String>,
    #[pyo3(get)]
    pub cc_emails: Vec<String>,
    #[pyo3(get)]
    pub bcc_emails: Vec<String>,
    #[pyo3(get)]
    pub date: Option<String>,
    #[pyo3(get)]
    pub message_id: Option<String>,
    #[pyo3(get)]
    pub plain_text: Option<String>,
    #[pyo3(get)]
    pub html_content: Option<String>,
    #[pyo3(get)]
    pub cleaned_text: String,
    #[pyo3(get)]
    pub attachments: Vec<EmailAttachmentDTO>,
    #[pyo3(get)]
    pub metadata: HashMap<String, String>,
}

#[pymethods]
impl EmailExtractionResultDTO {
    #[new]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        subject: Option<String>,
        from_email: Option<String>,
        to_emails: Vec<String>,
        cc_emails: Vec<String>,
        bcc_emails: Vec<String>,
        date: Option<String>,
        message_id: Option<String>,
        plain_text: Option<String>,
        html_content: Option<String>,
        cleaned_text: String,
        attachments: Vec<EmailAttachmentDTO>,
        metadata: HashMap<String, String>,
    ) -> Self {
        Self {
            subject,
            from_email,
            to_emails,
            cc_emails,
            bcc_emails,
            date,
            message_id,
            plain_text,
            html_content,
            cleaned_text,
            attachments,
            metadata,
        }
    }

    pub fn to_dict(&self, py: Python) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);

        dict.set_item("subject", &self.subject)?;
        dict.set_item("from_email", &self.from_email)?;
        dict.set_item("to_emails", &self.to_emails)?;
        dict.set_item("cc_emails", &self.cc_emails)?;
        dict.set_item("bcc_emails", &self.bcc_emails)?;
        dict.set_item("date", &self.date)?;
        dict.set_item("message_id", &self.message_id)?;
        dict.set_item("plain_text", &self.plain_text)?;
        dict.set_item("html_content", &self.html_content)?;
        dict.set_item("cleaned_text", &self.cleaned_text)?;

        let attachments_list = PyList::empty(py);
        for attachment in &self.attachments {
            let att_dict = PyDict::new(py);
            att_dict.set_item("name", &attachment.name)?;
            att_dict.set_item("filename", &attachment.filename)?;
            att_dict.set_item("mime_type", &attachment.mime_type)?;
            att_dict.set_item("size", attachment.size)?;
            att_dict.set_item("is_image", attachment.is_image)?;
            if let Some(data) = &attachment.data {
                att_dict.set_item("data", data.as_slice())?;
            } else {
                att_dict.set_item("data", py.None())?;
            }
            attachments_list.append(att_dict)?;
        }
        dict.set_item("attachments", attachments_list)?;

        let metadata_dict = PyDict::new(py);
        for (key, value) in &self.metadata {
            metadata_dict.set_item(key, value)?;
        }
        dict.set_item("metadata", metadata_dict)?;

        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "EmailResult(subject={:?}, from={:?}, attachments={})",
            self.subject,
            self.from_email,
            self.attachments.len()
        )
    }
}

// Regex patterns for HTML cleaning with proper error handling
static HTML_TAG_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]+>").expect("HTML tag regex pattern is valid"));

static SCRIPT_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)<script[^>]*>.*?</script>").expect("Script regex pattern is valid"));

static STYLE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)<style[^>]*>.*?</style>").expect("Style regex pattern is valid"));

static UNICODE_QUOTES_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[\u{201c}\u{201d}]").expect("Unicode quotes regex pattern is valid"));

static UNICODE_SINGLE_QUOTES_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[\u{2018}\u{2019}]").expect("Unicode single quotes regex pattern is valid"));

static WHITESPACE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").expect("Whitespace regex pattern is valid"));

/// Clean HTML content by removing tags and normalizing text
fn clean_html_content(html: &str) -> String {
    if html.is_empty() {
        return String::new();
    }

    // Remove script and style tags with their content
    let cleaned = SCRIPT_REGEX.replace_all(html, "");
    let cleaned = STYLE_REGEX.replace_all(&cleaned, "");

    // Remove all HTML tags
    let cleaned = HTML_TAG_REGEX.replace_all(&cleaned, "");

    // Decode HTML entities
    let cleaned = decode_html_entities(&cleaned);

    // Normalize Unicode quotes
    let cleaned = UNICODE_QUOTES_REGEX.replace_all(&cleaned, "\"");
    let cleaned = UNICODE_SINGLE_QUOTES_REGEX.replace_all(&cleaned, "'");

    // Normalize whitespace
    let cleaned = WHITESPACE_REGEX.replace_all(&cleaned, " ");

    cleaned.trim().to_string()
}

/// Format email addresses for display
fn format_email_addresses(addresses: &[String]) -> String {
    addresses.join(", ")
}

/// Detect if content is an image based on MIME type
fn is_image_mime_type(mime_type: &str) -> bool {
    mime_type.starts_with("image/")
}

/// Parse MIME type from Content-Type header, handling parameters
fn parse_content_type(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or("application/octet-stream")
        .trim()
        .to_lowercase()
}

/// Build metadata hashmap from extracted fields
#[allow(clippy::too_many_arguments)]
fn build_metadata(
    subject: &Option<String>,
    from_email: &Option<String>,
    to_emails: &[String],
    cc_emails: &[String],
    bcc_emails: &[String],
    date: &Option<String>,
    message_id: &Option<String>,
    attachments: &[EmailAttachmentDTO],
) -> HashMap<String, String> {
    let mut metadata = HashMap::new();

    if let Some(subj) = subject {
        metadata.insert("subject".to_string(), subj.clone());
    }
    if let Some(from) = from_email {
        metadata.insert("email_from".to_string(), from.clone());
    }
    if !to_emails.is_empty() {
        metadata.insert("email_to".to_string(), format_email_addresses(to_emails));
    }
    if !cc_emails.is_empty() {
        metadata.insert("email_cc".to_string(), format_email_addresses(cc_emails));
    }
    if !bcc_emails.is_empty() {
        metadata.insert("email_bcc".to_string(), format_email_addresses(bcc_emails));
    }
    if let Some(dt) = date {
        metadata.insert("date".to_string(), dt.clone());
    }
    if let Some(msg_id) = message_id {
        metadata.insert("message_id".to_string(), msg_id.clone());
    }

    // Add attachment names to metadata
    if !attachments.is_empty() {
        let attachment_names: Vec<String> = attachments
            .iter()
            .filter_map(|att| att.name.as_ref().or(att.filename.as_ref()))
            .cloned()
            .collect();
        if !attachment_names.is_empty() {
            metadata.insert("attachments".to_string(), attachment_names.join(", "));
        }
    }

    metadata
}

/// Parse EML content using mail-parser
fn parse_eml_content(content: &[u8]) -> Result<EmailExtractionResultDTO, EmailError> {
    let message = mail_parser::MessageParser::default()
        .parse(content)
        .ok_or_else(|| EmailError::ParseError {
            message: "Failed to parse EML content - invalid email format".to_string(),
        })?;

    // Extract basic information
    let subject = message.subject().map(|s| s.to_string());
    let message_id = message.message_id().map(|s| s.to_string());

    let from_email = message
        .from()
        .and_then(|from| from.first())
        .and_then(|addr| addr.address())
        .map(|s| s.to_string());

    // Extract recipients with proper error handling
    let to_emails: Vec<String> = message
        .to()
        .map(|to| {
            to.iter()
                .filter_map(|addr| addr.address().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let cc_emails: Vec<String> = message
        .cc()
        .map(|cc| {
            cc.iter()
                .filter_map(|addr| addr.address().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let bcc_emails: Vec<String> = message
        .bcc()
        .map(|bcc| {
            bcc.iter()
                .filter_map(|addr| addr.address().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let date = message.date().map(|d| d.to_rfc3339());

    // Extract body content
    let plain_text = message.body_text(0).map(|s| s.to_string());
    let html_content = message.body_html(0).map(|s| s.to_string());

    // Create cleaned text from available content
    let cleaned_text = if let Some(plain) = &plain_text {
        plain.clone()
    } else if let Some(html) = &html_content {
        clean_html_content(html)
    } else {
        String::new()
    };

    // Extract attachments using mail-parser with proper MIME type detection
    let mut attachments = Vec::new();
    for attachment in message.attachments() {
        let filename = attachment.attachment_name().map(|s| s.to_string());

        // Get proper MIME type from mail-parser
        let mime_type = attachment
            .content_type()
            .map(|ct| {
                let content_type_str = format!("{}/{}", ct.ctype(), ct.subtype().unwrap_or("octet-stream"));
                parse_content_type(&content_type_str)
            })
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let is_image = is_image_mime_type(&mime_type);
        let data = Some(attachment.contents().to_vec());
        let size = data.as_ref().map(|d| d.len() as u64);

        attachments.push(EmailAttachmentDTO {
            name: filename.clone(),
            filename,
            mime_type: Some(mime_type),
            size,
            is_image,
            data,
        });
    }

    // Build metadata
    let metadata = build_metadata(
        &subject,
        &from_email,
        &to_emails,
        &cc_emails,
        &bcc_emails,
        &date,
        &message_id,
        &attachments,
    );

    Ok(EmailExtractionResultDTO {
        subject,
        from_email,
        to_emails,
        cc_emails,
        bcc_emails,
        date,
        message_id,
        plain_text,
        html_content,
        cleaned_text,
        attachments,
        metadata,
    })
}

/// Parse MSG content using msg_parser's from_slice method
/// Attribution: Uses msg_parser library (MIT licensed) by marirs
fn parse_msg_content(content: &[u8]) -> Result<EmailExtractionResultDTO, EmailError> {
    // Use msg_parser's from_slice method which works with byte arrays directly
    let outlook = msg_parser::Outlook::from_slice(content).map_err(|e| EmailError::MsgError {
        message: format!("Failed to parse MSG file: {}", e),
    })?;

    // Extract basic fields
    let subject = Some(outlook.subject.clone());
    let from_email = Some(outlook.sender.email.clone());
    let from_name = if !outlook.sender.name.is_empty() {
        Some(outlook.sender.name.clone())
    } else {
        None
    };

    // Extract recipients
    let to_emails = outlook
        .to
        .iter()
        .map(|p| p.email.clone())
        .filter(|e| !e.is_empty())
        .collect::<Vec<String>>();

    let cc_emails = outlook
        .cc
        .iter()
        .map(|p| p.email.clone())
        .filter(|e| !e.is_empty())
        .collect::<Vec<String>>();

    let bcc_emails = if !outlook.bcc.is_empty() {
        vec![outlook.bcc.clone()]
    } else {
        vec![]
    };

    // Extract date from headers
    let date = if !outlook.headers.date.is_empty() {
        Some(outlook.headers.date.clone())
    } else {
        None
    };

    let message_id = if !outlook.headers.message_id.is_empty() {
        Some(outlook.headers.message_id.clone())
    } else {
        None
    };

    // Extract body content
    let plain_text = if !outlook.body.is_empty() {
        Some(outlook.body.clone())
    } else {
        None
    };

    // MSG files typically store RTF rather than HTML
    // For now, we'll just use the plain text body
    let html_content = None;
    let cleaned_text = plain_text.clone();

    // Extract attachments
    let attachments: Vec<EmailAttachmentDTO> = outlook
        .attachments
        .iter()
        .map(|att| {
            let filename = if !att.file_name.is_empty() {
                Some(att.file_name.clone())
            } else if !att.display_name.is_empty() {
                Some(att.display_name.clone())
            } else {
                Some(format!("attachment{}", att.extension))
            };

            let mime_type = if !att.mime_tag.is_empty() {
                Some(att.mime_tag.clone())
            } else {
                Some("application/octet-stream".to_string())
            };

            let data = if !att.payload.is_empty() {
                // The payload is hex-encoded in msg_parser
                hex::decode(&att.payload).ok()
            } else {
                None
            };

            let size = data.as_ref().map(|d| d.len() as u64);
            let is_image = mime_type.as_ref().map(|m| is_image_mime_type(m)).unwrap_or(false);

            EmailAttachmentDTO {
                name: filename.clone(),
                filename,
                mime_type,
                size,
                is_image,
                data,
            }
        })
        .collect();

    // Build metadata
    let mut metadata = HashMap::new();
    if let Some(ref subj) = subject {
        metadata.insert("subject".to_string(), subj.to_string());
    }
    if let Some(ref from) = from_email {
        metadata.insert("email_from".to_string(), from.to_string());
    }
    if let Some(ref name) = from_name {
        metadata.insert("from_name".to_string(), name.to_string());
    }
    if !to_emails.is_empty() {
        metadata.insert("email_to".to_string(), to_emails.join(", "));
    }
    if !cc_emails.is_empty() {
        metadata.insert("email_cc".to_string(), cc_emails.join(", "));
    }
    if !bcc_emails.is_empty() {
        metadata.insert("email_bcc".to_string(), bcc_emails.join(", "));
    }
    if let Some(ref dt) = date {
        metadata.insert("date".to_string(), dt.to_string());
    }
    if let Some(ref msg_id) = message_id {
        metadata.insert("message_id".to_string(), msg_id.to_string());
    }
    if !attachments.is_empty() {
        let attachment_names: Vec<String> = attachments
            .iter()
            .filter_map(|a| a.filename.as_ref())
            .cloned()
            .collect();
        metadata.insert("attachments".to_string(), attachment_names.join(", "));
    }

    Ok(EmailExtractionResultDTO {
        subject,
        from_email,
        to_emails,
        cc_emails,
        bcc_emails,
        date,
        message_id,
        plain_text,
        html_content,
        cleaned_text: cleaned_text.unwrap_or_default(),
        attachments,
        metadata,
    })
}

/// Extract email content from bytes with format detection
#[pyfunction]
pub fn extract_email_content(data: &[u8], mime_type: &str) -> PyResult<EmailExtractionResultDTO> {
    if data.is_empty() {
        return Err(EmailError::InvalidContent {
            message: "Email content is empty".to_string(),
        }
        .into());
    }

    match mime_type {
        "message/rfc822" | "text/plain" => parse_eml_content(data).map_err(|e| e.into()),
        "application/vnd.ms-outlook" => parse_msg_content(data).map_err(|e| e.into()),
        _ => Err(EmailError::UnsupportedFormat {
            format: mime_type.to_string(),
        }
        .into()),
    }
}

/// Extract email content from EML format
#[pyfunction]
pub fn extract_eml_content(data: &[u8]) -> PyResult<EmailExtractionResultDTO> {
    parse_eml_content(data).map_err(|e| e.into())
}

/// Extract email content from MSG format
#[pyfunction]
pub fn extract_msg_content(data: &[u8]) -> PyResult<EmailExtractionResultDTO> {
    parse_msg_content(data).map_err(|e| e.into())
}

/// Extract email from file path
#[pyfunction]
pub fn extract_email_from_file(file_path: &str) -> PyResult<EmailExtractionResultDTO> {
    let path = Path::new(file_path);

    if !path.exists() {
        return Err(EmailError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("File not found: {}", file_path),
        ))
        .into());
    }

    let content = std::fs::read(path).map_err(EmailError::IoError)?;

    // Determine format from file extension
    let mime_type = match path.extension().and_then(|s| s.to_str()) {
        Some("eml") => "message/rfc822",
        Some("msg") => "application/vnd.ms-outlook",
        _ => "message/rfc822", // Default to EML
    };

    extract_email_content(&content, mime_type)
}

/// Get list of supported email formats
#[pyfunction]
pub fn get_supported_email_formats() -> Vec<String> {
    vec![
        "message/rfc822".to_string(),
        "application/vnd.ms-outlook".to_string(),
        "text/plain".to_string(), // Some EML files are served as text/plain
    ]
}

/// Validate email content format
#[pyfunction]
pub fn validate_email_content(content: &[u8], mime_type: &str) -> bool {
    if content.is_empty() {
        return false;
    }

    match mime_type {
        "message/rfc822" | "text/plain" => mail_parser::MessageParser::default().parse(content).is_some(),
        "application/vnd.ms-outlook" => {
            // For MSG, we currently don't support validation to avoid temp file issues
            // Return false for now - this is safer than potential security vulnerabilities
            false
        }
        _ => false,
    }
}

/// Build formatted text output compatible with Python implementation
#[pyfunction]
pub fn build_email_text_output(result: &EmailExtractionResultDTO) -> String {
    let mut text_parts = Vec::new();

    // Add headers in consistent order
    if let Some(ref subject) = result.subject {
        text_parts.push(format!("Subject: {}", subject));
    }

    if let Some(ref from) = result.from_email {
        text_parts.push(format!("From: {}", from));
    }

    if !result.to_emails.is_empty() {
        text_parts.push(format!("To: {}", format_email_addresses(&result.to_emails)));
    }

    if !result.cc_emails.is_empty() {
        text_parts.push(format!("CC: {}", format_email_addresses(&result.cc_emails)));
    }

    if !result.bcc_emails.is_empty() {
        text_parts.push(format!("BCC: {}", format_email_addresses(&result.bcc_emails)));
    }

    if let Some(ref date) = result.date {
        text_parts.push(format!("Date: {}", date));
    }

    // Add body content
    text_parts.push(result.cleaned_text.clone());

    // Add attachments info
    if !result.attachments.is_empty() {
        let attachment_names: Vec<String> = result
            .attachments
            .iter()
            .filter_map(|att| att.name.as_ref().or(att.filename.as_ref()))
            .cloned()
            .collect();
        if !attachment_names.is_empty() {
            text_parts.push(format!("Attachments: {}", attachment_names.join(", ")));
        }
    }

    text_parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_html_content() {
        let html = r#"
            <html>
                <head><style>body { color: red; }</style></head>
                <body>
                    <script>alert('test');</script>
                    <p>Hello &amp; welcome to our <b>newsletter</b>!</p>
                    <div>Visit our &quot;products&quot; page</div>
                </body>
            </html>
        "#;

        let cleaned = clean_html_content(html);
        assert!(!cleaned.contains("<script>"));
        assert!(!cleaned.contains("<style>"));
        assert!(!cleaned.contains("<p>"));
        assert!(cleaned.contains("Hello & welcome"));
        assert!(cleaned.contains("newsletter"));
        assert!(cleaned.contains("\"products\""));
    }

    #[test]
    fn test_simple_eml_parsing() {
        let eml_content = b"From: test@example.com\r\nTo: recipient@example.com\r\nSubject: Test Email\r\n\r\nThis is a test email body.";

        let result = parse_eml_content(eml_content).unwrap();
        assert_eq!(result.cleaned_text, "This is a test email body.");
        assert_eq!(result.subject, Some("Test Email".to_string()));
        assert_eq!(result.from_email, Some("test@example.com".to_string()));
        assert_eq!(result.to_emails, vec!["recipient@example.com".to_string()]);
    }

    #[test]
    fn test_email_validation() {
        let valid_eml = b"From: test@example.com\r\nSubject: Test\r\n\r\nBody";
        assert!(validate_email_content(valid_eml, "message/rfc822"));

        let invalid_content = b"This is not an email";
        assert!(!validate_email_content(invalid_content, "message/rfc822"));

        // Empty content should be invalid
        assert!(!validate_email_content(b"", "message/rfc822"));
    }

    #[test]
    fn test_supported_formats() {
        let formats = get_supported_email_formats();
        assert!(formats.contains(&"message/rfc822".to_string()));
        assert!(formats.contains(&"application/vnd.ms-outlook".to_string()));
    }

    #[test]
    fn test_mime_type_parsing() {
        assert_eq!(parse_content_type("text/plain"), "text/plain");
        assert_eq!(parse_content_type("text/plain; charset=utf-8"), "text/plain");
        assert_eq!(parse_content_type("image/jpeg; name=test.jpg"), "image/jpeg");
    }

    #[test]
    fn test_is_image_mime_type() {
        assert!(is_image_mime_type("image/jpeg"));
        assert!(is_image_mime_type("image/png"));
        assert!(!is_image_mime_type("text/plain"));
        assert!(!is_image_mime_type("application/pdf"));
    }

    #[test]
    fn test_email_with_attachments() {
        let eml_with_attachments = b"From: test@example.com\r\nTo: recipient@example.com\r\nSubject: Test with attachment\r\nContent-Type: multipart/mixed; boundary=\"boundary123\"\r\n\r\n--boundary123\r\nContent-Type: text/plain\r\n\r\nThis is the email body.\r\n--boundary123\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment; filename=\"test.pdf\"\r\n\r\nPDF content here\r\n--boundary123--";

        let result = parse_eml_content(eml_with_attachments).unwrap();
        assert_eq!(result.subject, Some("Test with attachment".to_string()));
        assert!(!result.attachments.is_empty());
    }

    #[test]
    fn test_error_handling() {
        // Test invalid content
        let result = extract_email_content(b"", "message/rfc822");
        assert!(result.is_err());

        // Test unsupported format
        let result = extract_email_content(b"test", "unsupported/format");
        assert!(result.is_err());
    }
}
