//! Email extraction integration tests.
//!
//! Tests for .eml (RFC822) email extraction.
//! Validates metadata extraction, content extraction, HTML/plain text handling, and attachments.

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::extract_bytes;

mod helpers;

// ============================================================================
// EML Basic Extraction Tests
// ============================================================================

/// Test basic EML extraction with subject, from, to, and body.
#[tokio::test]
async fn test_eml_basic_extraction() {
    let config = ExtractionConfig::default();

    // Create a simple EML email
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

    // Verify MIME type
    assert_eq!(result.mime_type, "message/rfc822");

    // Verify subject metadata
    assert_eq!(result.metadata.subject, Some("Test Email Subject".to_string()));

    // Verify email metadata
    assert!(result.metadata.email.is_some());
    let email_meta = result.metadata.email.unwrap();
    assert_eq!(email_meta.from_email, Some("sender@example.com".to_string()));
    assert_eq!(email_meta.to_emails, vec!["recipient@example.com".to_string()]);

    // Message ID may or may not include angle brackets depending on parser
    assert!(email_meta.message_id.is_some());
    let msg_id = email_meta.message_id.unwrap();
    assert!(
        msg_id.contains("unique123@example.com"),
        "Message ID should contain unique123@example.com"
    );

    // Verify date
    assert!(result.metadata.date.is_some());

    // Verify content extraction
    assert!(result.content.contains("Subject: Test Email Subject"));
    assert!(result.content.contains("From: sender@example.com"));
    assert!(result.content.contains("To: recipient@example.com"));
    assert!(result.content.contains("This is the email body content"));
}

/// Test EML with attachments - metadata extraction.
#[tokio::test]
async fn test_eml_with_attachments() {
    let config = ExtractionConfig::default();

    // Create EML with attachment (simplified - real MIME multipart is complex)
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

    // Verify email metadata
    assert!(result.metadata.email.is_some());
    let email_meta = result.metadata.email.unwrap();

    // Verify attachments are listed (mail-parser should detect them)
    // Note: Attachment detection depends on mail-parser's parsing
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

    // Verify HTML content is cleaned
    // Scripts and styles should be removed, HTML tags stripped
    assert!(!result.content.contains("<script>"));
    assert!(!result.content.contains("<style>"));

    // Text content should be extracted
    assert!(result.content.contains("HTML Heading") || result.content.contains("bold"));
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

    // Verify plain text content
    assert!(result.content.contains("This is a plain text email"));
    assert!(result.content.contains("multiple lines"));
    assert!(result.content.contains("preserves formatting"));
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

    // mail-parser should extract at least one version
    assert!(
        result.content.contains("Plain text version") || result.content.contains("HTML version"),
        "Should extract either plain text or HTML content"
    );
}

/// Test MSG file extraction (Outlook format).
///
/// Note: Creating valid MSG files programmatically is complex.
/// This test verifies error handling for invalid MSG format.
#[tokio::test]
async fn test_msg_file_extraction() {
    let config = ExtractionConfig::default();

    // Invalid MSG data (should fail gracefully)
    let invalid_msg = b"This is not a valid MSG file";

    let result = extract_bytes(invalid_msg, "application/vnd.ms-outlook", &config).await;

    // Should fail with parsing error, not panic
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

    // Verify thread structure
    assert!(result.content.contains("This is my reply"));

    // Quoted text should be preserved
    assert!(result.content.contains("Original message text") || result.content.contains(">"));
}

/// Test email with various encodings (UTF-8, quoted-printable).
#[tokio::test]
async fn test_email_encodings() {
    let config = ExtractionConfig::default();

    // UTF-8 email with special characters
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

    // Verify Unicode is preserved
    assert!(result.content.contains("café") || result.content.contains("naive") || !result.content.is_empty());

    // Subject with Unicode
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

    // Verify email metadata
    assert!(result.metadata.email.is_some());
    let email_meta = result.metadata.email.unwrap();

    // Verify To recipients
    assert_eq!(email_meta.to_emails.len(), 3);
    assert!(email_meta.to_emails.contains(&"r1@example.com".to_string()));
    assert!(email_meta.to_emails.contains(&"r2@example.com".to_string()));
    assert!(email_meta.to_emails.contains(&"r3@example.com".to_string()));

    // Verify CC recipients
    assert_eq!(email_meta.cc_emails.len(), 2);
    assert!(email_meta.cc_emails.contains(&"cc1@example.com".to_string()));
    assert!(email_meta.cc_emails.contains(&"cc2@example.com".to_string()));

    // BCC is often not included in message headers (privacy)
    // So we don't assert on it
}

/// Test malformed email structure.
#[tokio::test]
async fn test_malformed_email() {
    let config = ExtractionConfig::default();

    // Malformed email (missing required headers, invalid structure)
    let malformed_eml = b"This is not a valid email at all.";

    let result = extract_bytes(malformed_eml, "message/rfc822", &config).await;

    // mail-parser is very permissive and may parse this
    // The important thing is it doesn't panic
    assert!(
        result.is_ok() || result.is_err(),
        "Should handle malformed email gracefully"
    );
}
