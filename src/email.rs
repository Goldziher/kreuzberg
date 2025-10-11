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

fn clean_html_content(html: &str) -> String {
    if html.is_empty() {
        return String::new();
    }

    let cleaned = SCRIPT_REGEX.replace_all(html, "");
    let cleaned = STYLE_REGEX.replace_all(&cleaned, "");

    let cleaned = HTML_TAG_REGEX.replace_all(&cleaned, "");

    let cleaned = decode_html_entities(&cleaned);

    let cleaned = UNICODE_QUOTES_REGEX.replace_all(&cleaned, "\"");
    let cleaned = UNICODE_SINGLE_QUOTES_REGEX.replace_all(&cleaned, "'");

    let cleaned = WHITESPACE_REGEX.replace_all(&cleaned, " ");

    cleaned.trim().to_string()
}

fn format_email_addresses(addresses: &[String]) -> String {
    addresses.join(", ")
}

fn is_image_mime_type(mime_type: &str) -> bool {
    mime_type.starts_with("image/")
}

fn parse_content_type(content_type: &str) -> String {
    let trimmed = content_type.trim();
    if trimmed.is_empty() {
        return "application/octet-stream".to_string();
    }
    trimmed
        .split(';')
        .next()
        .unwrap_or("application/octet-stream")
        .trim()
        .to_lowercase()
}

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

fn parse_eml_content(content: &[u8]) -> Result<EmailExtractionResultDTO, EmailError> {
    let message = mail_parser::MessageParser::default()
        .parse(content)
        .ok_or_else(|| EmailError::ParseError {
            message: "Failed to parse EML content - invalid email format".to_string(),
        })?;

    let subject = message.subject().map(|s| s.to_string());
    let message_id = message.message_id().map(|s| s.to_string());

    let from_email = message
        .from()
        .and_then(|from| from.first())
        .and_then(|addr| addr.address())
        .map(|s| s.to_string());

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

    let plain_text = message.body_text(0).map(|s| s.to_string());
    let html_content = message.body_html(0).map(|s| s.to_string());

    let cleaned_text = if let Some(plain) = &plain_text {
        plain.clone()
    } else if let Some(html) = &html_content {
        clean_html_content(html)
    } else {
        String::new()
    };

    let mut attachments = Vec::new();
    for attachment in message.attachments() {
        let filename = attachment.attachment_name().map(|s| s.to_string());

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

fn parse_msg_content(content: &[u8]) -> Result<EmailExtractionResultDTO, EmailError> {
    let outlook = msg_parser::Outlook::from_slice(content).map_err(|e| EmailError::MsgError {
        message: format!("Failed to parse MSG file: {}", e),
    })?;

    let subject = Some(outlook.subject.clone());
    let from_email = Some(outlook.sender.email.clone());
    let from_name = if !outlook.sender.name.is_empty() {
        Some(outlook.sender.name.clone())
    } else {
        None
    };

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

    let plain_text = if !outlook.body.is_empty() {
        Some(outlook.body.clone())
    } else {
        None
    };

    let html_content = None;
    let cleaned_text = plain_text.clone();

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

#[pyfunction]
pub fn extract_eml_content(data: &[u8]) -> PyResult<EmailExtractionResultDTO> {
    parse_eml_content(data).map_err(|e| e.into())
}

#[pyfunction]
pub fn extract_msg_content(data: &[u8]) -> PyResult<EmailExtractionResultDTO> {
    parse_msg_content(data).map_err(|e| e.into())
}

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

    let mime_type = match path.extension().and_then(|s| s.to_str()) {
        Some("eml") => "message/rfc822",
        Some("msg") => "application/vnd.ms-outlook",
        _ => "message/rfc822",
    };

    extract_email_content(&content, mime_type)
}

#[pyfunction]
pub fn get_supported_email_formats() -> Vec<String> {
    vec![
        "message/rfc822".to_string(),
        "application/vnd.ms-outlook".to_string(),
        "text/plain".to_string(),
    ]
}

#[pyfunction]
pub fn validate_email_content(content: &[u8], mime_type: &str) -> bool {
    if content.is_empty() {
        return false;
    }

    match mime_type {
        "message/rfc822" | "text/plain" => mail_parser::MessageParser::default().parse(content).is_some(),
        "application/vnd.ms-outlook" => false,
        _ => false,
    }
}

#[pyfunction]
pub fn build_email_text_output(result: &EmailExtractionResultDTO) -> String {
    let mut text_parts = Vec::new();

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

    text_parts.push(result.cleaned_text.clone());

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
        let result = extract_email_content(b"", "message/rfc822");
        assert!(result.is_err());

        let result = extract_email_content(b"test", "unsupported/format");
        assert!(result.is_err());
    }

    #[test]
    fn test_format_email_addresses() {
        let addresses = vec![
            "user1@example.com".to_string(),
            "user2@example.com".to_string(),
            "user3@example.com".to_string(),
        ];
        assert_eq!(
            format_email_addresses(&addresses),
            "user1@example.com, user2@example.com, user3@example.com"
        );

        let empty: Vec<String> = vec![];
        assert_eq!(format_email_addresses(&empty), "");

        let single = vec!["single@example.com".to_string()];
        assert_eq!(format_email_addresses(&single), "single@example.com");
    }

    #[test]
    fn test_build_metadata() {
        let subject = Some("Test Subject".to_string());
        let from_email = Some("sender@example.com".to_string());
        let to_emails = vec!["recipient@example.com".to_string()];
        let cc_emails = vec!["cc@example.com".to_string()];
        let bcc_emails = vec!["bcc@example.com".to_string()];
        let date = Some("2024-01-01T12:00:00Z".to_string());
        let message_id = Some("<abc123@example.com>".to_string());
        let attachments = vec![];

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

        assert_eq!(metadata.get("subject"), Some(&"Test Subject".to_string()));
        assert_eq!(metadata.get("email_from"), Some(&"sender@example.com".to_string()));
        assert_eq!(metadata.get("email_to"), Some(&"recipient@example.com".to_string()));
        assert_eq!(metadata.get("email_cc"), Some(&"cc@example.com".to_string()));
        assert_eq!(metadata.get("email_bcc"), Some(&"bcc@example.com".to_string()));
        assert_eq!(metadata.get("date"), Some(&"2024-01-01T12:00:00Z".to_string()));
        assert_eq!(metadata.get("message_id"), Some(&"<abc123@example.com>".to_string()));
    }

    #[test]
    fn test_build_metadata_with_attachments() {
        let attachments = vec![
            EmailAttachmentDTO {
                name: Some("file1.pdf".to_string()),
                filename: Some("file1.pdf".to_string()),
                mime_type: Some("application/pdf".to_string()),
                size: Some(1024),
                is_image: false,
                data: None,
            },
            EmailAttachmentDTO {
                name: Some("image.png".to_string()),
                filename: Some("image.png".to_string()),
                mime_type: Some("image/png".to_string()),
                size: Some(2048),
                is_image: true,
                data: None,
            },
        ];

        let metadata = build_metadata(&None, &None, &[], &[], &[], &None, &None, &attachments);

        assert_eq!(metadata.get("attachments"), Some(&"file1.pdf, image.png".to_string()));
    }

    #[test]
    fn test_build_email_text_output() {
        let result = EmailExtractionResultDTO {
            subject: Some("Test Subject".to_string()),
            from_email: Some("sender@example.com".to_string()),
            to_emails: vec!["recipient@example.com".to_string()],
            cc_emails: vec![],
            bcc_emails: vec![],
            date: Some("2024-01-01T12:00:00Z".to_string()),
            message_id: Some("<abc123@example.com>".to_string()),
            plain_text: Some("This is the email body.".to_string()),
            html_content: None,
            cleaned_text: "This is the email body.".to_string(),
            attachments: vec![],
            metadata: HashMap::new(),
        };

        let output = build_email_text_output(&result);

        assert!(output.contains("Subject: Test Subject"));
        assert!(output.contains("From: sender@example.com"));
        assert!(output.contains("To: recipient@example.com"));
        assert!(output.contains("Date: 2024-01-01T12:00:00Z"));
        assert!(output.contains("This is the email body."));
    }

    #[test]
    fn test_build_email_text_output_with_attachments() {
        let attachments = vec![EmailAttachmentDTO {
            name: Some("file.pdf".to_string()),
            filename: Some("file.pdf".to_string()),
            mime_type: Some("application/pdf".to_string()),
            size: Some(1024),
            is_image: false,
            data: None,
        }];

        let result = EmailExtractionResultDTO {
            subject: Some("Test".to_string()),
            from_email: Some("sender@example.com".to_string()),
            to_emails: vec!["recipient@example.com".to_string()],
            cc_emails: vec![],
            bcc_emails: vec![],
            date: None,
            message_id: None,
            plain_text: Some("Body".to_string()),
            html_content: None,
            cleaned_text: "Body".to_string(),
            attachments,
            metadata: HashMap::new(),
        };

        let output = build_email_text_output(&result);

        assert!(output.contains("Attachments: file.pdf"));
    }

    #[test]
    fn test_build_email_text_output_minimal() {
        let result = EmailExtractionResultDTO {
            subject: None,
            from_email: None,
            to_emails: vec![],
            cc_emails: vec![],
            bcc_emails: vec![],
            date: None,
            message_id: None,
            plain_text: None,
            html_content: None,
            cleaned_text: "Just content".to_string(),
            attachments: vec![],
            metadata: HashMap::new(),
        };

        let output = build_email_text_output(&result);

        assert!(output.contains("Just content"));
        assert!(!output.contains("Subject:"));
        assert!(!output.contains("From:"));
    }

    #[test]
    fn test_extract_eml_from_file() {
        use std::fs::File;
        use std::io::Write;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.eml");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "From: test@example.com\r\nSubject: Test\r\n\r\nTest body").unwrap();

        let result = extract_email_from_file(file_path.to_str().unwrap()).unwrap();
        assert_eq!(result.subject, Some("Test".to_string()));
        assert_eq!(result.from_email, Some("test@example.com".to_string()));
        assert_eq!(result.cleaned_text, "Test body\n");
    }

    #[test]
    fn test_parse_content_type_edge_cases() {
        assert_eq!(parse_content_type(""), "application/octet-stream");
        assert_eq!(parse_content_type("text/plain"), "text/plain");
        assert_eq!(
            parse_content_type("text/plain; charset=utf-8; format=flowed"),
            "text/plain"
        );
        assert_eq!(parse_content_type("IMAGE/JPEG"), "image/jpeg");
    }

    #[test]
    fn test_clean_html_content_edge_cases() {
        assert_eq!(clean_html_content(""), "");
        assert_eq!(clean_html_content("plain text"), "plain text");
        assert_eq!(clean_html_content("<p>&lt;tag&gt;</p>"), "<tag>");
    }

    #[test]
    fn test_parse_eml_content_complex() {
        let eml_content = b"From: sender@example.com\r\n\
To: recipient1@example.com, recipient2@example.com\r\n\
Cc: cc@example.com\r\n\
Subject: Complex Email\r\n\
Date: Wed, 15 Mar 2024 14:30:00 +0000\r\n\
Content-Type: text/html; charset=utf-8\r\n\
\r\n\
<html><body><p>HTML <strong>content</strong></p></body></html>";

        let result = parse_eml_content(eml_content).unwrap();
        assert_eq!(result.subject, Some("Complex Email".to_string()));
        assert_eq!(result.from_email, Some("sender@example.com".to_string()));
        assert_eq!(result.to_emails.len(), 2);
        assert_eq!(result.cc_emails, vec!["cc@example.com".to_string()]);
        assert!(result.cleaned_text.contains("HTML content"));
    }

    #[test]
    fn test_parse_eml_content_plain_text() {
        let eml_content = b"From: alice@example.com\r\n\
To: bob@example.com\r\n\
Subject: Plain Text Email\r\n\
Content-Type: text/plain\r\n\
\r\n\
This is plain text content.";

        let result = parse_eml_content(eml_content).unwrap();
        assert_eq!(result.subject, Some("Plain Text Email".to_string()));
        assert_eq!(result.from_email, Some("alice@example.com".to_string()));
        assert_eq!(result.cleaned_text, "This is plain text content.");
    }

    #[test]
    fn test_parse_eml_content_empty() {
        let empty_content = b"";
        let result = parse_eml_content(empty_content);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_eml_content_wrapper() {
        let eml_content = b"From: test@example.com\r\n\
Subject: Wrapper Test\r\n\
\r\n\
Test body";

        let result = extract_eml_content(eml_content);
        assert!(result.is_ok());
        let dto = result.unwrap();
        assert_eq!(dto.subject, Some("Wrapper Test".to_string()));
    }

    #[test]
    fn test_extract_email_content_with_eml_mime() {
        let eml_content = b"From: test@example.com\r\n\
Subject: MIME Test\r\n\
\r\n\
Body content";

        let result = extract_email_content(eml_content, "message/rfc822");
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_email_content_unsupported_mime() {
        let content = b"some content";
        let result = extract_email_content(content, "application/pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_email_content_eml() {
        let eml_content = b"From: test@example.com\r\nSubject: Test\r\n\r\nBody";
        assert!(validate_email_content(eml_content, "message/rfc822"));
    }

    #[test]
    fn test_validate_email_content_msg() {
        let msg_content = b"some msg content";
        assert!(!validate_email_content(msg_content, "application/vnd.ms-outlook"));
    }

    #[test]
    fn test_validate_email_content_unsupported() {
        assert!(!validate_email_content(b"test", "application/pdf"));
    }

    #[test]
    fn test_validate_email_content_empty() {
        assert!(!validate_email_content(b"", "message/rfc822"));
    }

    #[test]
    fn test_parse_eml_with_multipart() {
        let eml_content = b"From: multipart@example.com\r\n\
To: recipient@example.com\r\n\
Subject: Multipart Email\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/alternative; boundary=\"boundary123\"\r\n\
\r\n\
--boundary123\r\n\
Content-Type: text/plain\r\n\
\r\n\
Plain text part\r\n\
--boundary123\r\n\
Content-Type: text/html\r\n\
\r\n\
<html><body>HTML part</body></html>\r\n\
--boundary123--";

        let result = parse_eml_content(eml_content).unwrap();
        assert_eq!(result.subject, Some("Multipart Email".to_string()));
        assert!(!result.cleaned_text.is_empty());
    }

    #[test]
    fn test_build_metadata_comprehensive() {
        let subject = Some("Test Subject".to_string());
        let from = Some("from@example.com".to_string());
        let to = vec!["to1@example.com".to_string(), "to2@example.com".to_string()];
        let cc = vec!["cc@example.com".to_string()];
        let bcc = vec!["bcc@example.com".to_string()];
        let date = Some("2024-01-01".to_string());
        let msg_id = Some("<msg123>".to_string());
        let attachments = vec![];

        let metadata = build_metadata(&subject, &from, &to, &cc, &bcc, &date, &msg_id, &attachments);

        assert_eq!(metadata.get("subject"), Some(&"Test Subject".to_string()));
        assert_eq!(metadata.get("email_from"), Some(&"from@example.com".to_string()));
        assert_eq!(
            metadata.get("email_to"),
            Some(&"to1@example.com, to2@example.com".to_string())
        );
        assert_eq!(metadata.get("email_cc"), Some(&"cc@example.com".to_string()));
        assert_eq!(metadata.get("email_bcc"), Some(&"bcc@example.com".to_string()));
        assert_eq!(metadata.get("date"), Some(&"2024-01-01".to_string()));
        assert_eq!(metadata.get("message_id"), Some(&"<msg123>".to_string()));
    }

    #[test]
    fn test_parse_eml_with_attachment() {
        let eml_content = b"From: sender@example.com\r\n\
To: recipient@example.com\r\n\
Subject: Email with Attachment\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=\"boundary456\"\r\n\
\r\n\
--boundary456\r\n\
Content-Type: text/plain\r\n\
\r\n\
Email body with attachment\r\n\
--boundary456\r\n\
Content-Type: text/plain; name=\"test.txt\"\r\n\
Content-Disposition: attachment; filename=\"test.txt\"\r\n\
\r\n\
Attachment content\r\n\
--boundary456--";

        let result = parse_eml_content(eml_content).unwrap();
        assert_eq!(result.subject, Some("Email with Attachment".to_string()));
        assert!(!result.attachments.is_empty());
    }

    #[test]
    fn test_parse_eml_html_only() {
        let eml_content = b"From: html@example.com\r\n\
To: recipient@example.com\r\n\
Subject: HTML Only Email\r\n\
Content-Type: text/html\r\n\
\r\n\
<html><body><p>Only <b>HTML</b> content here</p></body></html>";

        let result = parse_eml_content(eml_content).unwrap();
        assert_eq!(result.subject, Some("HTML Only Email".to_string()));
        assert!(result.html_content.is_some());
        assert!(result.cleaned_text.contains("HTML content"));
    }

    #[test]
    fn test_extract_msg_content_invalid() {
        let invalid_msg = b"not a valid MSG file";
        let result = extract_msg_content(invalid_msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_eml_no_body() {
        let eml_content = b"From: nobodytest@example.com\r\n\
To: recipient@example.com\r\n\
Subject: No Body\r\n\
\r\n";

        let result = parse_eml_content(eml_content).unwrap();
        assert_eq!(result.subject, Some("No Body".to_string()));
        assert_eq!(result.cleaned_text, "");
    }

    #[test]
    fn test_parse_eml_with_bcc() {
        let eml_content = b"From: sender@example.com\r\n\
To: to@example.com\r\n\
Bcc: bcc@example.com\r\n\
Subject: BCC Test\r\n\
\r\n\
Body";

        let result = parse_eml_content(eml_content).unwrap();
        assert!(!result.bcc_emails.is_empty());
    }

    #[test]
    fn test_extract_email_content_msg_mime() {
        let invalid_msg = b"not valid";
        let result = extract_email_content(invalid_msg, "application/vnd.ms-outlook");
        assert!(result.is_err());
    }

    #[test]
    fn test_real_simple_eml() {
        use std::fs;
        let path = "test_documents/email/sample_email.eml";
        if let Ok(content) = fs::read(path) {
            let result = extract_eml_content(&content);
            assert!(result.is_ok());
            let dto = result.unwrap();
            assert!(dto.subject.is_some());
        }
    }

    #[test]
    fn test_real_html_only_eml() {
        use std::fs;
        let path = "test_documents/email/html_only.eml";
        if let Ok(content) = fs::read(path) {
            let result = parse_eml_content(&content);
            assert!(result.is_ok());
            let dto = result.unwrap();
            assert!(dto.html_content.is_some());
            assert!(!dto.cleaned_text.is_empty());
        }
    }

    #[test]
    fn test_real_multipart_eml() {
        use std::fs;
        let path = "test_documents/email/multipart_email.eml";
        if let Ok(content) = fs::read(path) {
            let result = parse_eml_content(&content);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_real_complex_headers_eml() {
        use std::fs;
        let path = "test_documents/email/complex_headers.eml";
        if let Ok(content) = fs::read(path) {
            let result = parse_eml_content(&content);
            assert!(result.is_ok());
            let dto = result.unwrap();
            assert!(dto.subject.is_some());
        }
    }

    #[test]
    fn test_real_eml_with_pdf_attachment() {
        use std::fs;
        let path = "test_documents/email/eml/with_attachments/mailgun_pdf_attachment.eml";
        if let Ok(content) = fs::read(path) {
            let result = parse_eml_content(&content);
            assert!(result.is_ok());
            let dto = result.unwrap();
            assert!(!dto.attachments.is_empty());
            assert!(dto.metadata.contains_key("attachments"));
        }
    }

    #[test]
    fn test_real_eml_with_png_attachment() {
        use std::fs;
        let path = "test_documents/email/eml/with_attachments/thunderbird_png_attachment.eml";
        if let Ok(content) = fs::read(path) {
            let result = parse_eml_content(&content);
            assert!(result.is_ok());
            let dto = result.unwrap();
            if !dto.attachments.is_empty() {
                let has_image = dto.attachments.iter().any(|a| a.is_image);
                assert!(has_image);
            }
        }
    }

    #[test]
    fn test_real_simple_msg() {
        use std::fs;
        let path = "test_documents/email/msg/simple/simple_msg.msg";
        if let Ok(content) = fs::read(path) {
            let result = parse_msg_content(&content);
            assert!(result.is_ok());
            let dto = result.unwrap();
            assert!(dto.subject.is_some() || dto.from_email.is_some());
        }
    }

    #[test]
    fn test_real_msg_with_attachments() {
        use std::fs;
        let path = "test_documents/email/msg/with_attachments/msg_with_png_attachment.msg";
        if let Ok(content) = fs::read(path) {
            let result = parse_msg_content(&content);
            assert!(result.is_ok());
            let dto = result.unwrap();
            if !dto.attachments.is_empty() {
                assert!(dto.metadata.contains_key("attachments"));
            }
        }
    }

    #[test]
    fn test_real_fake_email_msg() {
        use std::fs;
        let path = "test_documents/email/fake_email.msg";
        if let Ok(content) = fs::read(path) {
            let result = extract_msg_content(&content);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_real_msg_attachment_variant() {
        use std::fs;
        let path = "test_documents/email/fake_email_attachment.msg";
        if let Ok(content) = fs::read(path) {
            let result = parse_msg_content(&content);
            assert!(result.is_ok());
            let dto = result.unwrap();
            if !dto.attachments.is_empty() {
                let att = &dto.attachments[0];
                assert!(att.filename.is_some() || att.name.is_some());
                assert!(att.mime_type.is_some());
            }
        }
    }

    #[test]
    fn test_extract_email_from_file_real() {
        let path = "test_documents/email/sample_email.eml";
        let result = extract_email_from_file(path);
        assert!(result.is_ok());
        let dto = result.unwrap();
        assert!(dto.subject.is_some());
    }

    #[test]
    fn test_extract_email_from_file_nonexistent() {
        let result = extract_email_from_file("/nonexistent/file.eml");
        assert!(result.is_err());
    }

    #[test]
    fn test_real_plain_text_only_eml() {
        use std::fs;
        let path = "test_documents/email/eml/simple/plain_text_only.eml";
        if let Ok(content) = fs::read(path) {
            let result = parse_eml_content(&content);
            assert!(result.is_ok());
            let dto = result.unwrap();
            assert!(dto.plain_text.is_some());
            assert!(!dto.cleaned_text.is_empty());
        }
    }

    #[test]
    fn test_real_mixed_content_types() {
        use std::fs;
        let path = "test_documents/email/eml/with_attachments/mixed_content_types.eml";
        if let Ok(content) = fs::read(path) {
            let result = parse_eml_content(&content);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_extract_email_from_file_msg() {
        let path = "test_documents/email/fake_email.msg";
        let result = extract_email_from_file(path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_email_from_file_unknown_extension() {
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.unknown");
        fs::write(&file_path, b"From: test@example.com\r\nSubject: Test\r\n\r\nBody").unwrap();

        let result = extract_email_from_file(file_path.to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_msg_parsing_with_empty_fields() {
        use std::fs;
        let paths = [
            "test_documents/email/msg/simple/simple_msg.msg",
            "test_documents/email/msg/simple/simple_msg_alt.msg",
        ];

        for path in &paths {
            if let Ok(content) = fs::read(path) {
                let result = parse_msg_content(&content);
                if let Ok(dto) = result {
                    if dto.subject.is_some() {
                        assert!(dto.metadata.contains_key("subject"));
                    }
                    if dto.from_email.is_some() {
                        assert!(dto.metadata.contains_key("email_from"));
                    }
                }
            }
        }
    }

    #[test]
    fn test_msg_with_multiple_attachment_types() {
        use std::fs;
        let path = "test_documents/email/msg/with_attachments/msg_with_attachments_alt.msg";
        if let Ok(content) = fs::read(path) {
            let result = parse_msg_content(&content);
            if let Ok(dto) = result {
                for att in &dto.attachments {
                    assert!(att.mime_type.is_some());
                    assert!(att.filename.is_some() || att.name.is_some());
                }
            }
        }
    }

    #[test]
    fn test_eml_attachment_content_type_parsing() {
        let eml_content = b"From: sender@example.com\r\n\
To: recipient@example.com\r\n\
Subject: Test Attachment Content-Type\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=\"boundary789\"\r\n\
\r\n\
--boundary789\r\n\
Content-Type: text/plain\r\n\
\r\n\
Body\r\n\
--boundary789\r\n\
Content-Type: image/png; name=\"image.png\"\r\n\
Content-Disposition: attachment; filename=\"image.png\"\r\n\
\r\n\
PNG data here\r\n\
--boundary789--";

        let result = parse_eml_content(eml_content).unwrap();
        if !result.attachments.is_empty() {
            let img_att = result.attachments.iter().find(|a| a.is_image);
            if let Some(att) = img_att {
                assert!(att.mime_type.as_ref().unwrap().contains("image"));
            }
        }
    }

    #[test]
    fn test_msg_metadata_with_all_fields() {
        use std::fs;
        let path = "test_documents/email/fake_email_attachment.msg";
        if let Ok(content) = fs::read(path) {
            let result = parse_msg_content(&content);
            if let Ok(dto) = result {
                let keys: Vec<&str> = dto.metadata.keys().map(|s| s.as_str()).collect();
                assert!(!keys.is_empty());
            }
        }
    }

    #[test]
    fn test_eml_with_no_date() {
        let eml_content = b"From: sender@example.com\r\n\
To: recipient@example.com\r\n\
Subject: No Date Field\r\n\
\r\n\
Body content";

        let result = parse_eml_content(eml_content).unwrap();
        assert!(result.date.is_none() || result.date.as_ref().map(|d| d.is_empty()).unwrap_or(true));
    }

    #[test]
    fn test_eml_with_no_message_id() {
        let eml_content = b"From: sender@example.com\r\n\
To: recipient@example.com\r\n\
Subject: No Message-ID\r\n\
\r\n\
Body";

        let result = parse_eml_content(eml_content).unwrap();
        assert!(result.message_id.is_none());
    }
}
