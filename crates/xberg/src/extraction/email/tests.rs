use super::eml::*;
use super::*;

#[test]
fn test_clean_html_content() {
    let html = "<p>Hello <b>World</b></p>";
    let cleaned = clean_html_content(html);
    assert_eq!(cleaned, "Hello World");
}

#[test]
fn test_clean_html_with_whitespace() {
    let html = "<div>  Multiple   \n  spaces  </div>";
    let cleaned = clean_html_content(html);
    assert!(
        cleaned.contains("Multiple") && cleaned.contains("spaces"),
        "Should contain text: {}",
        cleaned
    );
    assert!(
        !cleaned.contains("  "),
        "Should not have consecutive spaces: {}",
        cleaned
    );
}

#[test]
fn test_clean_html_with_script_and_style() {
    let html = r#"
        <html>
            <head><style>body { color: red; }</style></head>
            <body>
                <script>alert('test');</script>
                <p>Hello World</p>
            </body>
        </html>
    "#;
    let cleaned = clean_html_content(html);
    assert!(!cleaned.contains("<script>"));
    assert!(!cleaned.contains("<style>"));
    assert!(cleaned.contains("Hello World"));
}

#[test]
fn test_is_image_mime_type() {
    assert!(is_image_mime_type("image/png"));
    assert!(is_image_mime_type("image/jpeg"));
    assert!(!is_image_mime_type("text/plain"));
    assert!(!is_image_mime_type("application/pdf"));
}

#[test]
fn test_parse_content_type() {
    assert_eq!(parse_content_type("text/plain"), "text/plain");
    assert_eq!(parse_content_type("text/plain; charset=utf-8"), "text/plain");
    assert_eq!(parse_content_type("image/jpeg; name=test.jpg"), "image/jpeg");
    assert_eq!(parse_content_type(""), "application/octet-stream");
}

#[test]
fn test_extract_email_content_empty_data() {
    let result = extract_email_content(b"", "message/rfc822", None);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), XbergError::Validation { .. }));
}

#[test]
fn test_extract_email_content_invalid_mime_type() {
    let result = extract_email_content(b"test", "application/pdf", None);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), XbergError::Validation { .. }));
}

#[test]
fn test_parse_eml_content_invalid() {
    let result = parse_eml_content(b"not an email");
    assert!(result.is_ok());
}

#[test]
fn test_parse_msg_content_invalid() {
    let result = parse_msg_content(b"not a msg file", None);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), XbergError::Parsing { .. }));
}

#[test]
fn test_simple_eml_parsing() {
    let eml_content =
        b"From: test@example.com\r\nTo: recipient@example.com\r\nSubject: Test Email\r\n\r\nThis is a test email body.";

    let result = parse_eml_content(eml_content).unwrap();
    assert_eq!(result.subject, Some("Test Email".to_string()));
    assert_eq!(result.from_email, Some("test@example.com".to_string()));
    assert_eq!(result.to_emails, vec!["recipient@example.com".to_string()]);
    assert_eq!(result.content, "This is a test email body.");
}

#[test]
fn test_eml_sender_display_name_is_preserved_separately() {
    let eml_content = b"From: Alice Example <alice@example.com>\r\nSubject: Sender\r\n\r\nBody";

    let result = parse_eml_content(eml_content).unwrap();

    assert_eq!(result.from_email.as_deref(), Some("alice@example.com"));
    assert_eq!(
        result.metadata.get("from_name").map(String::as_str),
        Some("Alice Example")
    );
}

#[test]
fn test_build_email_text_output_minimal() {
    let result = EmailExtractionResult {
        subject: Some("Test".to_string()),
        from_email: Some("sender@example.com".to_string()),
        to_emails: vec!["recipient@example.com".to_string()],
        cc_emails: vec![],
        bcc_emails: vec![],
        date: None,
        message_id: None,
        plain_text: None,
        html_content: None,
        content: "Hello World".to_string(),
        attachments: vec![],
        metadata: HashMap::new(),
    };

    let output = build_email_text_output(&result);
    assert!(output.contains("Subject: Test"));
    assert!(output.contains("From: sender@example.com"));
    assert!(output.contains("To: recipient@example.com"));
    assert!(output.contains("Hello World"));
}

#[test]
fn test_build_email_text_output_with_attachments() {
    let result = EmailExtractionResult {
        subject: Some("Test".to_string()),
        from_email: Some("sender@example.com".to_string()),
        to_emails: vec!["recipient@example.com".to_string()],
        cc_emails: vec![],
        bcc_emails: vec![],
        date: None,
        message_id: None,
        plain_text: None,
        html_content: None,
        content: "Hello World".to_string(),
        attachments: vec![EmailAttachment {
            name: Some("file.txt".to_string()),
            filename: Some("file.txt".to_string()),
            mime_type: Some("text/plain".to_string()),
            size: Some(1024),
            is_image: false,
            data: None,
        }],
        metadata: HashMap::new(),
    };

    let output = build_email_text_output(&result);
    assert!(!output.contains("Attachments:"));
    assert!(output.contains("Hello World"));
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

    let metadata = build_metadata(EmailMetadataFields {
        subject: &subject,
        from_email: &from_email,
        to_emails: &to_emails,
        cc_emails: &cc_emails,
        bcc_emails: &bcc_emails,
        date: &date,
        message_id: &message_id,
        attachments: &attachments,
    });

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
        EmailAttachment {
            name: Some("file1.pdf".to_string()),
            filename: Some("file1.pdf".to_string()),
            mime_type: Some("application/pdf".to_string()),
            size: Some(1024),
            is_image: false,
            data: None,
        },
        EmailAttachment {
            name: Some("image.png".to_string()),
            filename: Some("image.png".to_string()),
            mime_type: Some("image/png".to_string()),
            size: Some(2048),
            is_image: true,
            data: None,
        },
    ];

    let metadata = build_metadata(EmailMetadataFields {
        subject: &None,
        from_email: &None,
        to_emails: &[],
        cc_emails: &[],
        bcc_emails: &[],
        date: &None,
        message_id: &None,
        attachments: &attachments,
    });

    assert_eq!(metadata.get("attachments"), Some(&"file1.pdf, image.png".to_string()));
}

#[test]
fn test_clean_html_content_empty() {
    let cleaned = clean_html_content("");
    assert_eq!(cleaned, "");
}

#[test]
fn test_clean_html_content_only_tags() {
    let html = "<div><span><p></p></span></div>";
    let cleaned = clean_html_content(html);
    assert_eq!(cleaned, "");
}

#[test]
fn test_clean_html_content_nested_tags() {
    let html = "<div><p>Outer <span>Inner <b>Bold</b></span> Text</p></div>";
    let cleaned = clean_html_content(html);
    assert_eq!(cleaned, "Outer Inner Bold Text");
}

#[test]
fn test_clean_html_content_multiple_scripts() {
    let html = r#"
        <script>function a() {}</script>
        <p>Content</p>
        <script>function b() {}</script>
    "#;
    let cleaned = clean_html_content(html);
    assert!(!cleaned.contains("function"));
    assert!(cleaned.contains("Content"));
}

#[test]
fn test_is_image_mime_type_variants() {
    assert!(is_image_mime_type("image/gif"));
    assert!(is_image_mime_type("image/webp"));
    assert!(is_image_mime_type("image/svg+xml"));
    assert!(!is_image_mime_type("video/mp4"));
    assert!(!is_image_mime_type("audio/mp3"));
}

#[test]
fn test_parse_content_type_with_parameters() {
    assert_eq!(parse_content_type("multipart/mixed; boundary=xyz"), "multipart/mixed");
    assert_eq!(parse_content_type("text/html; charset=UTF-8"), "text/html");
}

#[test]
fn test_parse_content_type_whitespace() {
    assert_eq!(parse_content_type("  text/plain  "), "text/plain");
    assert_eq!(parse_content_type(" text/plain ; charset=utf-8 "), "text/plain");
}

#[test]
fn test_parse_content_type_case_insensitive() {
    assert_eq!(parse_content_type("TEXT/PLAIN"), "text/plain");
    assert_eq!(parse_content_type("Image/JPEG"), "image/jpeg");
}

#[test]
fn test_extract_email_content_mime_variants() {
    let eml_content = b"From: test@example.com\r\n\r\nBody";

    assert!(extract_email_content(eml_content, "message/rfc822", None).is_ok());
    assert!(extract_email_content(eml_content, "text/plain", None).is_ok());
}

#[test]
fn test_simple_eml_with_multiple_recipients() {
    let eml_content = b"From: sender@example.com\r\nTo: r1@example.com, r2@example.com\r\nCc: cc@example.com\r\nBcc: bcc@example.com\r\nSubject: Multi-recipient\r\n\r\nBody";

    let result = parse_eml_content(eml_content).unwrap();
    assert_eq!(result.to_emails.len(), 2);
    assert!(result.to_emails.contains(&"r1@example.com".to_string()));
    assert!(result.to_emails.contains(&"r2@example.com".to_string()));
}

#[test]
fn test_simple_eml_with_html_body() {
    let eml_content = b"From: sender@example.com\r\nTo: recipient@example.com\r\nSubject: HTML Email\r\nContent-Type: text/html\r\n\r\n<html><body><p>HTML Body</p></body></html>";

    let result = parse_eml_content(eml_content).unwrap();
    assert!(!result.content.is_empty());
}

#[test]
fn test_build_email_text_output_with_all_fields() {
    let result = EmailExtractionResult {
        subject: Some("Complete Email".to_string()),
        from_email: Some("sender@example.com".to_string()),
        to_emails: vec!["recipient@example.com".to_string()],
        cc_emails: vec!["cc@example.com".to_string()],
        bcc_emails: vec!["bcc@example.com".to_string()],
        date: Some("2024-01-01T12:00:00Z".to_string()),
        message_id: Some("<msg123@example.com>".to_string()),
        plain_text: Some("Plain text body".to_string()),
        html_content: Some("<html><body>HTML body</body></html>".to_string()),
        content: "Cleaned body text".to_string(),
        attachments: vec![],
        metadata: HashMap::new(),
    };

    let output = build_email_text_output(&result);
    assert!(output.contains("Subject: Complete Email"));
    assert!(output.contains("From: sender@example.com"));
    assert!(output.contains("To: recipient@example.com"));
    assert!(output.contains("CC: cc@example.com"));
    assert!(output.contains("BCC: bcc@example.com"));
    assert!(output.contains("Date: 2024-01-01T12:00:00Z"));
    assert!(output.contains("Cleaned body text"));
}

#[test]
fn test_build_email_text_output_empty_attachments() {
    let result = EmailExtractionResult {
        subject: Some("Test".to_string()),
        from_email: Some("sender@example.com".to_string()),
        to_emails: vec!["recipient@example.com".to_string()],
        cc_emails: vec![],
        bcc_emails: vec![],
        date: None,
        message_id: None,
        plain_text: None,
        html_content: None,
        content: "Body".to_string(),
        attachments: vec![EmailAttachment {
            name: None,
            filename: None,
            mime_type: Some("application/octet-stream".to_string()),
            size: Some(100),
            is_image: false,
            data: None,
        }],
        metadata: HashMap::new(),
    };

    let output = build_email_text_output(&result);
    assert!(output.contains("Body"));
}

#[test]
fn test_build_metadata_empty_fields() {
    let metadata = build_metadata(EmailMetadataFields {
        subject: &None,
        from_email: &None,
        to_emails: &[],
        cc_emails: &[],
        bcc_emails: &[],
        date: &None,
        message_id: &None,
        attachments: &[],
    });
    assert!(metadata.is_empty());
}

#[test]
fn test_build_metadata_partial_fields() {
    let subject = Some("Test".to_string());
    let date = Some("2024-01-01".to_string());

    let metadata = build_metadata(EmailMetadataFields {
        subject: &subject,
        from_email: &None,
        to_emails: &[],
        cc_emails: &[],
        bcc_emails: &[],
        date: &date,
        message_id: &None,
        attachments: &[],
    });

    assert_eq!(metadata.get("subject"), Some(&"Test".to_string()));
    assert_eq!(metadata.get("date"), Some(&"2024-01-01".to_string()));
    assert_eq!(metadata.len(), 2);
}

#[test]
fn test_clean_html_content_case_insensitive_tags() {
    let html = "<SCRIPT>code</SCRIPT><STYLE>css</STYLE><P>Text</P>";
    let cleaned = clean_html_content(html);
    assert!(!cleaned.contains("code"));
    assert!(!cleaned.contains("css"));
    assert!(cleaned.contains("Text"));
}

#[test]
fn test_simple_eml_with_date() {
    let eml_content = b"From: sender@example.com\r\nTo: recipient@example.com\r\nDate: Mon, 1 Jan 2024 12:00:00 +0000\r\nSubject: Test\r\n\r\nBody";

    let result = parse_eml_content(eml_content).unwrap();
    assert!(result.date.is_some());
}

#[test]
fn test_simple_eml_with_message_id() {
    let eml_content = b"From: sender@example.com\r\nTo: recipient@example.com\r\nMessage-ID: <unique@example.com>\r\nSubject: Test\r\n\r\nBody";

    let result = parse_eml_content(eml_content).unwrap();
    assert!(result.message_id.is_some());
}

#[test]
fn test_simple_eml_minimal() {
    let eml_content = b"From: sender@example.com\r\n\r\nMinimal body";

    let result = parse_eml_content(eml_content).unwrap();
    assert_eq!(result.from_email, Some("sender@example.com".to_string()));
    assert_eq!(result.content, "Minimal body");
}

#[test]
fn test_regex_initialization() {
    let _ = html_tag_regex();
    let _ = script_regex();
    let _ = style_regex();
    let _ = whitespace_regex();

    let _ = html_tag_regex();
    let _ = script_regex();
    let _ = style_regex();
    let _ = whitespace_regex();
}

#[test]
fn test_clean_html_content_multiline_script() {
    let html = r#"<html>
<head>
<title>HTML Email</title>
<style>
    body { font-family: Arial, sans-serif; }
    .header { color: blue; }
    .content { margin: 10px; }
</style>
</head>
<body>
<div class="header">
    <h1>Welcome to Our Service</h1>
</div>

<div class="content">
    <p>This email contains <strong>only HTML</strong> content.</p>

    <p>It includes:</p>
    <ul>
        <li>HTML entities: &lt;, &gt;, &amp;, &quot;</li>
        <li>Special characters: €, ©, ®</li>
        <li>Formatting: <em>italic</em>, <strong>bold</strong></li>
    </ul>

    <p>The Rust implementation should clean this HTML and extract meaningful text.</p>

    <script>
        // This script should be removed during HTML cleaning
        alert('Should not appear in extracted text');
    </script>
</div>

<footer>
    <p>&copy; 2024 Example Company</p>
</footer>
</body>
</html>"#;
    let cleaned = clean_html_content(html);
    assert!(
        !cleaned.contains("script should be removed"),
        "Script content leaked into output: {}",
        cleaned
    );
    assert!(
        !cleaned.contains("alert("),
        "Script content leaked into output: {}",
        cleaned
    );
    assert!(cleaned.contains("Welcome to Our Service"));
}

#[test]
fn test_html_only_eml_script_stripping() {
    let eml_data = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test_documents/email/html_only.eml"
    ))
    .expect("html_only.eml should exist");
    let result = parse_eml_content(&eml_data).unwrap();
    assert!(
        !result.content.contains("script should be removed"),
        "Script content leaked into content: {}",
        result.content
    );
    assert!(
        !result.content.contains("alert("),
        "Script content leaked into content: {}",
        result.content
    );
    assert!(
        result.content.contains("Welcome to Our Service"),
        "Expected content missing from content: {}",
        result.content
    );
}

#[test]
fn test_parse_msg_content_invalid_with_fallback() {
    let result = parse_msg_content(b"not a msg file", Some(1251));
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), XbergError::Parsing { .. }));
}

#[test]
fn test_extract_email_content_invalid_codepage_is_silent() {
    let eml = b"From: a@b.com\r\nSubject: Test\r\n\r\nBody";
    let result = extract_email_content(eml, "message/rfc822", Some(99999));
    assert!(result.is_ok());
}

#[test]
fn test_parse_msg_default_unchanged_with_real_fixture() {
    let Some(data) = crate::utils::read_test_fixture("vendored/unstructured/msg/fake-email.msg") else {
        return;
    };
    let result = parse_msg_content(&data, None).unwrap();
    assert!(!result.content.is_empty());
}

#[test]
fn test_parse_msg_reads_binary_html_body_with_attachment() {
    let Some(data) = crate::utils::read_test_fixture("email/msg_with_attachments_alt.msg") else {
        return;
    };

    let result = parse_msg_content(&data, None).unwrap();

    assert_eq!(result.subject.as_deref(), Some("This is the subject"));
    assert_eq!(result.from_email.as_deref(), Some("peterpan@neverland.com"));
    assert_eq!(result.to_emails, vec!["crocodile@neverland.com".to_string()]);
    assert_eq!(result.content, "This is a message");
    assert_eq!(result.html_content.as_deref(), Some("This is a message"));
    assert_eq!(result.attachments.len(), 1);
    assert_eq!(result.attachments[0].filename.as_deref(), Some("canvas.png"));
}

#[test]
fn test_parse_msg_invalid_codepage_falls_back_silently() {
    let Some(data) = crate::utils::read_test_fixture("vendored/unstructured/msg/fake-email.msg") else {
        return;
    };
    let result_invalid = parse_msg_content(&data, Some(99999)).unwrap();
    let result_default = parse_msg_content(&data, None).unwrap();
    assert_eq!(result_invalid.subject, result_default.subject);
    assert_eq!(result_invalid.content, result_default.content);
}

#[test]
fn test_eml_threading_headers() {
    let eml = b"From: alice@example.com\r\n\
        To: bob@example.com\r\n\
        Subject: Re: Thread test\r\n\
        Message-ID: <msg2@example.com>\r\n\
        In-Reply-To: <msg1@example.com>\r\n\
        References: <msg0@example.com> <msg1@example.com>\r\n\
        Reply-To: noreply@example.com\r\n\
        \r\n\
        Reply body";

    let result = parse_eml_content(eml).unwrap();
    assert!(
        result.metadata.contains_key("in_reply_to"),
        "Should extract In-Reply-To header"
    );
    assert!(
        result.metadata.contains_key("references"),
        "Should extract References header"
    );
    assert!(
        result.metadata.contains_key("reply_to"),
        "Should extract Reply-To header"
    );
    assert!(result.metadata.get("reply_to").unwrap().contains("noreply@example.com"));
}

#[test]
fn test_eml_raw_headers() {
    let eml = b"From: alice@example.com\r\n\
        To: bob@example.com\r\n\
        Subject: Header test\r\n\
        Content-Type: text/plain; charset=utf-8\r\n\
        MIME-Version: 1.0\r\n\
        X-Mailer: TestMailer/1.0\r\n\
        List-Id: <test.example.com>\r\n\
        List-Unsubscribe: <mailto:unsub@example.com>\r\n\
        \r\n\
        Body content";

    let result = parse_eml_content(eml).unwrap();
    assert!(
        result.metadata.contains_key("content_type"),
        "Should extract Content-Type"
    );
    assert!(result.metadata.get("content_type").unwrap().contains("text/plain"));
    assert!(
        result.metadata.contains_key("mime_version"),
        "Should extract MIME-Version"
    );
    assert_eq!(result.metadata.get("mime_version").unwrap(), "1.0");
    assert!(result.metadata.contains_key("x_mailer"), "Should extract X-Mailer");
    assert!(result.metadata.get("x_mailer").unwrap().contains("TestMailer"));
    assert!(result.metadata.contains_key("list_id"), "Should extract List-Id");
    assert!(
        result.metadata.contains_key("list_unsubscribe"),
        "Should extract List-Unsubscribe"
    );
}

#[test]
fn test_eml_attachment_details_metadata() {
    let eml = b"From: alice@example.com\r\n\
        To: bob@example.com\r\n\
        Subject: With attachment\r\n\
        MIME-Version: 1.0\r\n\
        Content-Type: multipart/mixed; boundary=\"BOUNDARY\"\r\n\
        \r\n\
        --BOUNDARY\r\n\
        Content-Type: text/plain\r\n\
        \r\n\
        Body text\r\n\
        --BOUNDARY\r\n\
        Content-Type: application/pdf\r\n\
        Content-Disposition: attachment; filename=\"report.pdf\"\r\n\
        \r\n\
        FAKEPDFDATA\r\n\
        --BOUNDARY--";

    let result = parse_eml_content(eml).unwrap();
    assert!(!result.attachments.is_empty(), "Should have attachment");
    assert!(
        result.metadata.contains_key("attachment_details"),
        "Should have attachment_details"
    );
    let details = result.metadata.get("attachment_details").unwrap();
    assert!(details.contains("report.pdf"), "Should contain filename");
    assert!(details.contains("application/pdf"), "Should contain mime type");
}

#[test]
fn test_extract_raw_headers_function() {
    let data = b"From: alice@example.com\r\n\
        Content-Type: multipart/mixed; boundary=foo\r\n\
        MIME-Version: 1.0\r\n\
        X-Mailer: MyApp/2.0\r\n\
        User-Agent: MyAgent/1.0\r\n\
        \r\n\
        Body";

    let headers = extract_raw_headers(data);
    assert_eq!(headers.get("content_type").unwrap(), "multipart/mixed; boundary=foo");
    assert_eq!(headers.get("mime_version").unwrap(), "1.0");
    assert_eq!(headers.get("x_mailer").unwrap(), "MyApp/2.0");
    assert_eq!(headers.get("user_agent").unwrap(), "MyAgent/1.0");
}

#[test]
fn test_find_header_section_end_crlf_separator() {
    let data = b"From: alice@example.com\r\n\r\nBody";
    // The header line is 23 bytes, so the separator STARTS at index 23; both the
    // old windows().position() and memmem::find return the match start, not its end.
    assert_eq!(find_header_section_end(data, 8192), 23);
}

#[test]
fn test_find_header_section_end_lf_only_separator() {
    let data = b"From: alice@example.com\n\nBody";
    assert_eq!(find_header_section_end(data, 8192), 23);
}

#[test]
fn test_find_header_section_end_no_separator_falls_back_to_cap() {
    let data = b"From: alice@example.com, no blank line here";
    assert_eq!(find_header_section_end(data, 10), 10);
}

#[test]
fn test_find_header_section_end_no_separator_shorter_than_cap() {
    let data = b"short";
    assert_eq!(find_header_section_end(data, 8192), data.len());
}

#[test]
fn test_decompress_rtf_compressed_crafted_raw_size_does_not_over_allocate() {
    let mut data = Vec::with_capacity(20);
    data.extend_from_slice(&16u32.to_le_bytes());
    data.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    data.extend_from_slice(&0x75465a4cu32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&[0x00, b'A', b'B', b'C']);

    let result = decompress_rtf_compressed(&data);
    let out = result.expect("should decompress without error");
    assert!(out.len() < 16, "output should be tiny, not a huge allocation");
}

#[test]
fn test_decompress_rtf_compressed_cap_is_hint_only() {
    let payload: &[u8] = &[
        0x00, b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', 0x00, b'I', b'J', b'K', b'L', b'M', b'N', b'O', b'P',
        0x00, b'Q', b'R', b'S', b'T', b'U', b'V', b'W', b'X',
    ];
    let comp_size = (12 + payload.len()) as u32;
    let raw_size = 1u32;
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&comp_size.to_le_bytes());
    data.extend_from_slice(&raw_size.to_le_bytes());
    data.extend_from_slice(&0x75465a4cu32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(payload);

    let out = decompress_rtf_compressed(&data).expect("should decompress");
    assert_eq!(
        out.len(),
        24,
        "Vec must grow past the raw_size=1 hint to hold all 24 bytes"
    );
    assert_eq!(&out[..8], b"ABCDEFGH");
}

#[test]
fn test_decompress_rtf_compressed_too_short() {
    assert!(decompress_rtf_compressed(&[0u8; 10]).is_none());
}

#[test]
fn test_decompress_rtf_compressed_bad_magic() {
    let mut data = [0u8; 16];
    data[8..12].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
    assert!(decompress_rtf_compressed(&data).is_none());
}

#[test]
fn test_decompress_rtf_compressed_uncompressed_magic() {
    let rtf_payload = b"{\\rtf1 Hello}";
    let comp_size = (rtf_payload.len() + 12) as u32;
    let raw_size = rtf_payload.len() as u32;
    let mut data = Vec::new();
    data.extend_from_slice(&comp_size.to_le_bytes());
    data.extend_from_slice(&raw_size.to_le_bytes());
    data.extend_from_slice(&0x414c_454du32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(rtf_payload);

    let result = decompress_rtf_compressed(&data).unwrap();
    assert_eq!(result, rtf_payload);
}

#[test]
fn test_strip_rtf_to_plain_text_basic() {
    let rtf = b"{\\rtf1\\ansi\\deff0 Hello World}";
    let result = strip_rtf_to_plain_text(rtf);
    assert_eq!(result, "Hello World");
}

#[test]
fn test_strip_rtf_to_plain_text_par() {
    let rtf = b"{\\rtf1\\ansi Line 1\\par Line 2}";
    let result = strip_rtf_to_plain_text(rtf);
    assert!(result.contains("Line 1"));
    assert!(result.contains("Line 2"));
    assert!(result.contains('\n'));
}

#[test]
fn test_strip_rtf_to_plain_text_unicode() {
    let rtf = b"{\\rtf1 Price: \\u8364?100}";
    let result = strip_rtf_to_plain_text(rtf);
    assert!(result.contains('\u{20AC}'));
    assert!(result.contains("100"));
}

#[test]
fn test_strip_rtf_to_plain_text_hex_escape() {
    let rtf = b"{\\rtf1 caf\\'e9}";
    let result = strip_rtf_to_plain_text(rtf);
    assert!(result.contains("caf\u{00e9}"));
}

#[test]
fn test_strip_rtf_to_plain_text_skips_fonttbl() {
    let rtf = b"{\\rtf1{\\fonttbl{\\f0 Arial;}}{\\f0 Visible text}}";
    let result = strip_rtf_to_plain_text(rtf);
    assert!(!result.contains("Arial"));
    assert!(result.contains("Visible text"));
}

#[test]
fn test_strip_rtf_to_plain_text_escaped_braces() {
    let rtf = b"{\\rtf1 Open \\{ and close \\}}";
    let result = strip_rtf_to_plain_text(rtf);
    assert!(result.contains("Open {"));
    assert!(result.contains("close }"));
}

#[test]
fn test_strip_rtf_to_plain_text_empty() {
    let rtf = b"{\\rtf1}";
    let result = strip_rtf_to_plain_text(rtf);
    assert!(result.is_empty());
}

#[test]
fn test_maybe_transcode_utf16_robustness() {
    let tiny_ambiguous = b"t\0e\0";
    assert!(maybe_transcode_utf16(tiny_ambiguous).is_none());

    let mut with_bom_le = vec![0xFF, 0xFE];
    with_bom_le.extend_from_slice(&"Test".encode_utf16().flat_map(|u| u.to_le_bytes()).collect::<Vec<u8>>());
    let result = maybe_transcode_utf16(&with_bom_le).expect("Should transcode LE with BOM");
    assert_eq!(String::from_utf8(result).unwrap(), "Test");

    let mut with_bom_be = vec![0xFE, 0xFF];
    with_bom_be.extend_from_slice(&"Test".encode_utf16().flat_map(|u| u.to_be_bytes()).collect::<Vec<u8>>());
    let result = maybe_transcode_utf16(&with_bom_be).expect("Should transcode BE with BOM");
    assert_eq!(String::from_utf8(result).unwrap(), "Test");

    let long_text = "Subject: This is a long enough string to trigger the heuristic.";
    let long_utf16_le: Vec<u8> = long_text.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    let result = maybe_transcode_utf16(&long_utf16_le).expect("Should transcode long non-BOM LE");
    assert_eq!(String::from_utf8(result).unwrap(), long_text);

    let long_utf8 = b"Subject: This is a normal UTF-8 string without any null bytes.";
    assert!(maybe_transcode_utf16(long_utf8).is_none());
}
