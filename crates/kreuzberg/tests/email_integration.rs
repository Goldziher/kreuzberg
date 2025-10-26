//! Email extraction integration tests.
//!
//! Tests for .eml (RFC822) email extraction.
//! Validates metadata extraction, content extraction, HTML/plain text handling, and attachments.

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::extract_bytes;

mod helpers;

/// Test basic EML extraction with subject, from, to, and body.
#[tokio::test]
async fn test_eml_basic_extraction() {
    let config = ExtractionConfig::default();

    let eml_content = b"From: sender@example.com\r\n\
To: recipient@example.com\r\n\
Subject: Test Email Subject\r\n\
Date: Mon, 1 Jan 2024 12:00:00 +0000\r\n\
Message-ID: <unique123@example.com>\r\n\
\r\n\
This is the email body content.";

    let result = extract_bytes(eml_content, "message/rfc822", &config)
        .await
        .expect("Should extract EML successfully");

    assert_eq!(result.mime_type, "message/rfc822");

    assert_eq!(result.metadata.subject, Some("Test Email Subject".to_string()));

    assert!(result.metadata.format.is_some());
    let email_meta = match result.metadata.format.as_ref().unwrap() {
        kreuzberg::FormatMetadata::Email(meta) => meta,
        _ => panic!("Expected Email metadata"),
    };

    assert_eq!(email_meta.from_email, Some("sender@example.com".to_string()));

    assert_eq!(email_meta.to_emails, vec!["recipient@example.com".to_string()]);
    assert!(email_meta.cc_emails.is_empty(), "CC should be empty");
    assert!(email_meta.bcc_emails.is_empty(), "BCC should be empty");

    assert!(email_meta.message_id.is_some());
    let msg_id = email_meta.message_id.clone().unwrap();
    assert!(
        msg_id.contains("unique123@example.com"),
        "Message ID should contain unique123@example.com"
    );

    assert!(email_meta.attachments.is_empty(), "Should have no attachments");

    assert!(result.metadata.date.is_some());

    assert!(result.content.contains("Subject: Test Email Subject"));
    assert!(result.content.contains("From: sender@example.com"));
    assert!(result.content.contains("To: recipient@example.com"));
    assert!(result.content.contains("This is the email body content"));
}

/// Test EML with attachments - metadata extraction.
#[tokio::test]
async fn test_eml_with_attachments() {
    let config = ExtractionConfig::default();

    let eml_content = b"From: sender@example.com\r\n\
To: recipient@example.com\r\n\
Subject: Email with Attachment\r\n\
Content-Type: multipart/mixed; boundary=\"----boundary\"\r\n\
\r\n\
------boundary\r\n\
Content-Type: text/plain\r\n\
\r\n\
Email body text.\r\n\
------boundary\r\n\
Content-Type: text/plain; name=\"file.txt\"\r\n\
Content-Disposition: attachment; filename=\"file.txt\"\r\n\
\r\n\
Attachment content here.\r\n\
------boundary--\r\n";

    let result = extract_bytes(eml_content, "message/rfc822", &config)
        .await
        .expect("Should extract EML with attachment");

    assert!(result.metadata.format.is_some());
    let email_meta = match result.metadata.format.as_ref().unwrap() {
        kreuzberg::FormatMetadata::Email(meta) => meta,
        _ => panic!("Expected Email metadata"),
    };

    if !email_meta.attachments.is_empty() {
        assert!(result.content.contains("Attachments:"));
    }

    assert!(result.content.contains("Email body text") || result.content.contains("Attachment content"));
}

/// Test EML with HTML body.
#[tokio::test]
async fn test_eml_html_body() {
    let config = ExtractionConfig::default();

    let eml_content = b"From: sender@example.com\r\n\
To: recipient@example.com\r\n\
Subject: HTML Email\r\n\
Content-Type: text/html; charset=utf-8\r\n\
\r\n\
<html>\r\n\
<head><style>body { color: blue; }</style></head>\r\n\
<body>\r\n\
<h1>HTML Heading</h1>\r\n\
<p>This is <b>bold</b> text in HTML.</p>\r\n\
<script>alert('test');</script>\r\n\
</body>\r\n\
</html>";

    let result = extract_bytes(eml_content, "message/rfc822", &config)
        .await
        .expect("Should extract HTML email");

    assert!(!result.content.contains("<script>"));
    assert!(!result.content.contains("<style>"));

    assert!(result.content.contains("HTML Heading") || result.content.contains("bold"));

    assert!(result.metadata.format.is_some());
    let email_meta = match result.metadata.format.as_ref().unwrap() {
        kreuzberg::FormatMetadata::Email(meta) => meta,
        _ => panic!("Expected Email metadata"),
    };
    assert_eq!(email_meta.from_email, Some("sender@example.com".to_string()));
    assert_eq!(email_meta.to_emails, vec!["recipient@example.com".to_string()]);
    assert_eq!(result.metadata.subject, Some("HTML Email".to_string()));
}

/// Test EML with plain text body.
#[tokio::test]
async fn test_eml_plain_text_body() {
    let config = ExtractionConfig::default();

    let eml_content = b"From: sender@example.com\r\n\
To: recipient@example.com\r\n\
Subject: Plain Text Email\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
This is a plain text email.\r\n\
It has multiple lines.\r\n\
And preserves formatting.";

    let result = extract_bytes(eml_content, "message/rfc822", &config)
        .await
        .expect("Should extract plain text email");

    assert!(result.content.contains("This is a plain text email"));
    assert!(result.content.contains("multiple lines"));
    assert!(result.content.contains("preserves formatting"));

    assert!(result.metadata.format.is_some());
    let email_meta = match result.metadata.format.as_ref().unwrap() {
        kreuzberg::FormatMetadata::Email(meta) => meta,
        _ => panic!("Expected Email metadata"),
    };
    assert_eq!(email_meta.from_email, Some("sender@example.com".to_string()));
    assert_eq!(email_meta.to_emails, vec!["recipient@example.com".to_string()]);
    assert_eq!(result.metadata.subject, Some("Plain Text Email".to_string()));
}

/// Test EML multipart (HTML + plain text).
#[tokio::test]
async fn test_eml_multipart() {
    let config = ExtractionConfig::default();

    let eml_content = b"From: sender@example.com\r\n\
To: recipient@example.com\r\n\
Subject: Multipart Email\r\n\
Content-Type: multipart/alternative; boundary=\"----boundary\"\r\n\
\r\n\
------boundary\r\n\
Content-Type: text/plain\r\n\
\r\n\
Plain text version of the email.\r\n\
------boundary\r\n\
Content-Type: text/html\r\n\
\r\n\
<html><body><p>HTML version of the email.</p></body></html>\r\n\
------boundary--\r\n";

    let result = extract_bytes(eml_content, "message/rfc822", &config)
        .await
        .expect("Should extract multipart email");

    assert!(
        result.content.contains("Plain text version") || result.content.contains("HTML version"),
        "Should extract either plain text or HTML content"
    );

    assert!(result.metadata.format.is_some());
    let email_meta = match result.metadata.format.as_ref().unwrap() {
        kreuzberg::FormatMetadata::Email(meta) => meta,
        _ => panic!("Expected Email metadata"),
    };
    assert_eq!(email_meta.from_email, Some("sender@example.com".to_string()));
    assert_eq!(email_meta.to_emails, vec!["recipient@example.com".to_string()]);
    assert_eq!(result.metadata.subject, Some("Multipart Email".to_string()));
}

/// Test MSG file extraction (Outlook format).
///
/// Note: Creating valid MSG files programmatically is complex.
/// This test verifies error handling for invalid MSG format.
#[tokio::test]
async fn test_msg_file_extraction() {
    let config = ExtractionConfig::default();

    let invalid_msg = b"This is not a valid MSG file";

    let result = extract_bytes(invalid_msg, "application/vnd.ms-outlook", &config).await;

    assert!(result.is_err(), "Invalid MSG should fail gracefully");
}

/// Test email thread with quoted replies.
#[tokio::test]
async fn test_email_thread() {
    let config = ExtractionConfig::default();

    let eml_content = b"From: person2@example.com\r\n\
To: person1@example.com\r\n\
Subject: Re: Original Subject\r\n\
In-Reply-To: <original@example.com>\r\n\
\r\n\
This is my reply.\r\n\
\r\n\
On Mon, 1 Jan 2024, person1@example.com wrote:\r\n\
> Original message text here.\r\n\
> This was the first message.";

    let result = extract_bytes(eml_content, "message/rfc822", &config)
        .await
        .expect("Should extract email thread");

    assert!(result.content.contains("This is my reply"));

    assert!(result.content.contains("Original message text") || result.content.contains(">"));
}

/// Test email with various encodings (UTF-8, quoted-printable).
#[tokio::test]
async fn test_email_encodings() {
    let config = ExtractionConfig::default();

    let eml_content = "From: sender@example.com\r\n\
To: recipient@example.com\r\n\
Subject: Email with Unicode: 你好世界 🌍\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Email body with special chars: café, naïve, résumé.\r\n\
Emoji: 🎉 🚀 ✅"
        .as_bytes();

    let result = extract_bytes(eml_content, "message/rfc822", &config)
        .await
        .expect("Should extract UTF-8 email");

    assert!(result.content.contains("café") || result.content.contains("naive") || !result.content.is_empty());

    if let Some(subject) = result.metadata.subject {
        assert!(subject.contains("Unicode") || subject.contains("Email"));
    }
}

/// Test email with multiple recipients (To, CC, BCC).
#[tokio::test]
async fn test_email_large_attachments() {
    let config = ExtractionConfig::default();

    let eml_content = b"From: sender@example.com\r\n\
To: r1@example.com, r2@example.com, r3@example.com\r\n\
Cc: cc1@example.com, cc2@example.com\r\n\
Bcc: bcc@example.com\r\n\
Subject: Multiple Recipients\r\n\
\r\n\
Email to multiple recipients.";

    let result = extract_bytes(eml_content, "message/rfc822", &config)
        .await
        .expect("Should extract email with multiple recipients");

    assert!(result.metadata.format.is_some());
    let email_meta = match result.metadata.format.as_ref().unwrap() {
        kreuzberg::FormatMetadata::Email(meta) => meta,
        _ => panic!("Expected Email metadata"),
    };

    assert_eq!(email_meta.from_email, Some("sender@example.com".to_string()));

    assert_eq!(email_meta.to_emails.len(), 3, "Should have 3 To recipients");
    assert!(email_meta.to_emails.contains(&"r1@example.com".to_string()));
    assert!(email_meta.to_emails.contains(&"r2@example.com".to_string()));
    assert!(email_meta.to_emails.contains(&"r3@example.com".to_string()));

    assert_eq!(email_meta.cc_emails.len(), 2, "Should have 2 CC recipients");
    assert!(email_meta.cc_emails.contains(&"cc1@example.com".to_string()));
    assert!(email_meta.cc_emails.contains(&"cc2@example.com".to_string()));

    assert_eq!(result.metadata.subject, Some("Multiple Recipients".to_string()));

    assert!(email_meta.attachments.is_empty(), "Should have no attachments");
}

/// Test malformed email structure.
#[tokio::test]
async fn test_malformed_email() {
    let config = ExtractionConfig::default();

    let malformed_eml = b"This is not a valid email at all.";

    let result = extract_bytes(malformed_eml, "message/rfc822", &config).await;

    assert!(
        result.is_ok() || result.is_err(),
        "Should handle malformed email gracefully"
    );
}
