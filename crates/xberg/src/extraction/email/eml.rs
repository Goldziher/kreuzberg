use super::*;

pub(crate) struct ParsedEmailContent {
    pub(crate) result: EmailExtractionResult,
    pub(crate) nested_messages: Vec<NestedMessagePayload>,
    /// Embedded `message/rfc822`-equivalent MSG attachments (`attach_method ==
    /// afEmbeddedMessage`), fully parsed. Flat list across all nesting depths.
    pub(crate) nested_embedded_messages: Vec<EmailExtractionResult>,
    /// Non-fatal warnings raised while parsing (e.g. a `.msg`-in-`.msg` chain
    /// that hit the configured nesting-depth cap and was left unextracted).
    pub(crate) warnings: Vec<ProcessingWarning>,
}

pub(crate) struct NestedMessagePayload {
    part_id: usize,
    pub(crate) data: Option<Bytes>,
    pub(crate) size: usize,
}

/// Parse .eml file content (RFC822 format)
pub(crate) fn parse_eml_content(data: &[u8]) -> Result<EmailExtractionResult> {
    Ok(parse_eml_content_internal(data, false, None)?.result)
}

/// Address- and header-derived fields read from a parsed `mail_parser::Message`, before
/// any body text is looked at. Bundled purely to keep [`parse_eml_content_internal`]'s
/// body shorter -- [`build_metadata`] and the final [`EmailExtractionResult`] each take
/// only the fields they need back out.
pub(super) struct EmailAddressFields {
    subject: Option<String>,
    from_email: Option<String>,
    from_name: Option<String>,
    to_emails: Vec<String>,
    cc_emails: Vec<String>,
    bcc_emails: Vec<String>,
    date: Option<String>,
    message_id: Option<String>,
    reply_to: Vec<String>,
    in_reply_to: Vec<String>,
    references: Vec<String>,
}

/// Read the sender's address and display name from the first `From:` mailbox.
pub(super) fn extract_sender_fields(message: &mail_parser::Message<'_>) -> (Option<String>, Option<String>) {
    let sender = message.from().and_then(|from| from.first());
    let from_email = sender.and_then(|address| address.address().map(str::to_string));
    let from_name = sender
        .and_then(|address| address.name().map(str::trim))
        .filter(|name| !name.is_empty())
        .map(str::to_string);
    (from_email, from_name)
}

/// Read the message date, preferring the literal `Date:` header text (see
/// `extract_raw_date_header`) over `mail_parser`'s own parsed value, and discarding a
/// parsed value that round-trips to an obviously-invalid placeholder year.
pub(super) fn extract_email_date_field(message: &mail_parser::Message<'_>, data: &[u8]) -> Option<String> {
    extract_raw_date_header(data).or_else(|| {
        message.date().and_then(|d| {
            let rfc3339 = d.to_rfc3339();
            if rfc3339.starts_with("2000-00") || rfc3339.starts_with("0000-") {
                None
            } else {
                Some(rfc3339)
            }
        })
    })
}

/// Read subject/sender/recipient/date/threading fields from a parsed message and its raw
/// bytes (`data`, needed because `extract_raw_date_header` reads the literal `Date:`
/// header text rather than `mail_parser`'s own parsed value -- see that function).
pub(super) fn extract_email_address_fields(message: &mail_parser::Message<'_>, data: &[u8]) -> EmailAddressFields {
    let subject = message.subject().map(|s| s.to_string());
    let (from_email, from_name) = extract_sender_fields(message);

    let to_emails: Vec<String> = message
        .to()
        .map(|to| {
            to.iter()
                .filter_map(|addr| addr.address().map(|email| email.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let cc_emails: Vec<String> = message
        .cc()
        .map(|cc| {
            cc.iter()
                .filter_map(|addr| addr.address().map(|email| email.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let bcc_emails: Vec<String> = message
        .bcc()
        .map(|bcc| {
            bcc.iter()
                .filter_map(|addr| addr.address().map(|email| email.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let date = extract_email_date_field(message, data);

    let message_id = message.message_id().map(|id| id.to_string());

    let reply_to: Vec<String> = message
        .reply_to()
        .map(|addrs| {
            addrs
                .iter()
                .filter_map(|addr| addr.address().map(|email| email.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let in_reply_to: Vec<String> = message
        .in_reply_to()
        .as_text_list()
        .map(|list| list.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    let references: Vec<String> = message
        .references()
        .as_text_list()
        .map(|list| list.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    EmailAddressFields {
        subject,
        from_email,
        from_name,
        to_emails,
        cc_emails,
        bcc_emails,
        date,
        message_id,
        reply_to,
        in_reply_to,
        references,
    }
}

/// Build the attachment list, resolving each `message/rfc822` nested-message attachment's
/// bytes from `nested_messages` (already capped/collected by
/// [`collect_nested_message_payloads`]) when nesting was requested, or from the part's own
/// raw bytes otherwise.
pub(super) fn build_email_attachments(
    message: &mail_parser::Message<'_>,
    nested_messages: &[NestedMessagePayload],
    include_nested_messages: bool,
) -> Vec<EmailAttachment> {
    let mut attachments = Vec::with_capacity(message.attachments().count().min(20));
    for &part_id in &message.attachments {
        let Some(attachment) = message.parts.get(part_id as usize) else {
            continue;
        };
        let filename = attachment.attachment_name().map(|s| s.to_string());

        let mime_type = if matches!(&attachment.body, mail_parser::PartType::Message(_)) {
            "message/rfc822".to_string()
        } else {
            attachment
                .content_type()
                .map(|ct| {
                    let content_type_str = format!("{}/{}", ct.ctype(), ct.subtype().unwrap_or("octet-stream"));
                    parse_content_type(&content_type_str)
                })
                .unwrap_or_else(|| "application/octet-stream".to_string())
        };

        let (data, size) = match &attachment.body {
            mail_parser::PartType::Message(nested) => {
                let raw_message = nested.raw_message();
                let data = if include_nested_messages {
                    nested_messages
                        .iter()
                        .find(|payload| payload.part_id == part_id as usize)
                        .and_then(|payload| payload.data.clone())
                } else {
                    Some(Bytes::copy_from_slice(raw_message))
                };
                (data, raw_message.len())
            }
            _ => {
                let contents = recover_base64_contents(message, attachment)
                    .map(Cow::Owned)
                    .unwrap_or(Cow::Borrowed(attachment.contents()));
                (Some(Bytes::copy_from_slice(&contents)), contents.len())
            }
        };

        let is_image = is_image_mime_type(&mime_type);

        attachments.push(EmailAttachment {
            name: filename.clone(),
            filename,
            mime_type: Some(mime_type),
            size: Some(size),
            is_image,
            data,
        });
    }
    attachments
}

/// Base64 without padding, keeping the bits of a last, incomplete group.
const LENIENT_BASE64: base64::engine::GeneralPurpose = base64::engine::GeneralPurpose::new(
    &base64::alphabet::STANDARD,
    base64::engine::GeneralPurposeConfig::new()
        .with_decode_padding_mode(base64::engine::DecodePaddingMode::RequireNone)
        .with_decode_allow_trailing_bits(true),
);

/// The bytes of a base64 part that `mail-parser` could not decode.
///
/// `mail-parser` gives up on a base64 body at the first character outside the base64 alphabet,
/// such as a stray `>`, and keeps the still-encoded text as the part's contents: it flags the part
/// `is_encoding_problem` and resets its `encoding` to `None`. That text then came out as the body
/// or as the text of an attachment. RFC 2045, section 6.8, has a decoder ignore those characters,
/// so the text is decoded here without them, up to the first `=`, which the RFC also takes as the
/// end of the data. A last symbol left on its own holds six bits, no whole byte, and is dropped.
///
/// `None` for any other part, including an attached message that `mail-parser` decoded but could
/// not parse: it is flagged too, but keeps its `Base64` encoding and its decoded contents.
///
/// The encoded text is read from `message`, the message that owns `part`, at the part's body
/// offsets. The contents of a text part are already decoded with its charset, so the base64 of a
/// UTF-16 body came back as unrelated characters there and filtered down to nothing.
pub(super) fn recover_base64_contents(
    message: &mail_parser::Message<'_>,
    part: &mail_parser::MessagePart<'_>,
) -> Option<Vec<u8>> {
    use base64::Engine as _;

    let declares_base64 = part
        .content_transfer_encoding()
        .is_some_and(|encoding| encoding.trim().eq_ignore_ascii_case("base64"));
    if !part.is_encoding_problem || part.encoding != mail_parser::Encoding::None || !declares_base64 {
        return None;
    }
    let encoded = message
        .raw_message
        .get(part.offset_body as usize..part.offset_end as usize)?;
    let mut symbols: Vec<u8> = encoded
        .iter()
        .copied()
        .take_while(|&byte| byte != b'=')
        .filter(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/'))
        .collect();
    if symbols.len() % 4 == 1 {
        symbols.pop();
    }
    LENIENT_BASE64.decode(symbols).ok()
}

/// The text of a body part, decoded from base64 by [`recover_base64_contents`] where
/// `mail-parser` could not, in the charset the part declares.
fn recover_base64_text(message: &mail_parser::Message<'_>, part: &mail_parser::MessagePart<'_>) -> Option<String> {
    let bytes = recover_base64_contents(message, part)?;
    let decoder = part
        .content_type()
        .and_then(|content_type| content_type.attribute("charset"))
        .and_then(|charset| mail_parser::decoders::charsets::map::charset_decoder(charset.as_bytes()));
    Some(match decoder {
        Some(decode) => decode(&bytes),
        None => String::from_utf8_lossy(&bytes).into_owned(),
    })
}

/// Fold the threading headers, sender display name, raw headers, and a compact
/// pipe-delimited attachment summary into `metadata`, after [`build_metadata`] has
/// already populated its core fields.
pub(super) fn augment_email_metadata(
    metadata: &mut HashMap<String, String>,
    addresses: &EmailAddressFields,
    raw_headers: &HashMap<String, String>,
    attachments: &[EmailAttachment],
) {
    if let Some(from_name) = &addresses.from_name {
        metadata.insert("from_name".to_string(), from_name.clone());
    }

    if !addresses.reply_to.is_empty() {
        metadata.insert("reply_to".to_string(), addresses.reply_to.join(", "));
    }
    if !addresses.in_reply_to.is_empty() {
        metadata.insert("in_reply_to".to_string(), addresses.in_reply_to.join(", "));
    }
    if !addresses.references.is_empty() {
        metadata.insert("references".to_string(), addresses.references.join(", "));
    }

    for (key, value) in raw_headers {
        metadata.insert(key.clone(), value.clone());
    }

    if !attachments.is_empty() {
        let attachment_details: Vec<String> = attachments
            .iter()
            .map(|att| {
                let name = att.filename.as_deref().or(att.name.as_deref()).unwrap_or("unnamed");
                let mime = att.mime_type.as_deref().unwrap_or("application/octet-stream");
                let size = att.size.unwrap_or(0);
                format!("{}|{}|{}", name, mime, size)
            })
            .collect();
        metadata.insert("attachment_details".to_string(), attachment_details.join("; "));
    }
}

/// Gather the message's plain-text and HTML bodies (each joined from every genuine body
/// part plus any nested message's own bodies), or `None` for a body kind the message
/// carries no genuine part of -- unless `should_treat_as_html`, which forces both to be
/// gathered so [`parse_eml_content_internal`] can decide there whether one substitutes
/// for the other.
pub(super) fn extract_email_bodies(
    message: &mail_parser::Message<'_>,
    should_treat_as_html: bool,
) -> (Option<String>, Option<String>) {
    let has_genuine_text_part = has_genuine_text_body(message);
    let plain_text = if has_genuine_text_part || should_treat_as_html {
        let mut all_text = Vec::new();
        let mut i = 0;
        while let Some(text) = message.body_text(i) {
            let recovered = message
                .text_body
                .get(i)
                .and_then(|&part_id| message.parts.get(part_id as usize))
                .and_then(|part| recover_base64_text(message, part));
            all_text.push(recovered.unwrap_or_else(|| text.to_string()));
            i += 1;
        }
        collect_nested_message_text(message, &mut all_text);
        if all_text.is_empty() {
            None
        } else {
            Some(all_text.join("\n\n"))
        }
    } else {
        None
    };

    let has_genuine_html_part = has_genuine_html_body(message);
    let html_content = if has_genuine_html_part || should_treat_as_html {
        let mut all_html = Vec::new();
        let mut i = 0;
        while let Some(html) = message.body_html(i) {
            all_html.push(html.to_string());
            i += 1;
        }
        collect_nested_message_html(message, &mut all_html);
        if all_html.is_empty() {
            None
        } else {
            Some(all_html.join("\n\n"))
        }
    } else {
        None
    };

    (plain_text, html_content)
}

/// Resolve the final `html_content` (an HTML-typed message with no genuine HTML part
/// falls back to its plain-text body, since that is the only body it has) and the
/// plain-text `content` field derived from whichever body is present, preferring HTML.
pub(super) fn resolve_email_content(
    plain_text: &Option<String>,
    html_content: Option<String>,
    should_treat_as_html: bool,
) -> (Option<String>, String) {
    let html_content = if should_treat_as_html && html_content.is_none() && plain_text.is_some() {
        plain_text.clone()
    } else {
        html_content
    };

    let content = if let Some(html) = &html_content {
        clean_html_content(html)
    } else if let Some(plain) = plain_text {
        plain.clone()
    } else {
        String::new()
    };

    (html_content, content)
}

pub(super) fn parse_eml_content_internal(
    data: &[u8],
    include_nested_messages: bool,
    max_nested_message_bytes: Option<u64>,
) -> Result<ParsedEmailContent> {
    let data = if let Some(transcoded) = maybe_transcode_utf16(data) {
        std::borrow::Cow::Owned(transcoded)
    } else {
        std::borrow::Cow::Borrowed(data)
    };

    let message = mail_parser::MessageParser::default()
        .parse(&data)
        .ok_or_else(|| XbergError::parsing("Failed to parse EML file: invalid email format".to_string()))?;

    let addresses = extract_email_address_fields(&message, &data);
    let raw_headers = extract_raw_headers(&data);

    let should_treat_as_html = message
        .content_type()
        .and_then(|ct| ct.subtype())
        .map(|subtype| subtype.eq_ignore_ascii_case("html"))
        .unwrap_or(false);

    let (plain_text, html_content) = extract_email_bodies(&message, should_treat_as_html);
    let (html_content, content) = resolve_email_content(&plain_text, html_content, should_treat_as_html);

    let nested_messages = if include_nested_messages {
        collect_nested_message_payloads(&message, max_nested_message_bytes)
    } else {
        Vec::new()
    };

    let attachments = build_email_attachments(&message, &nested_messages, include_nested_messages);

    let mut metadata = build_metadata(EmailMetadataFields {
        subject: &addresses.subject,
        from_email: &addresses.from_email,
        to_emails: &addresses.to_emails,
        cc_emails: &addresses.cc_emails,
        bcc_emails: &addresses.bcc_emails,
        date: &addresses.date,
        message_id: &addresses.message_id,
        attachments: &attachments,
    });
    augment_email_metadata(&mut metadata, &addresses, &raw_headers, &attachments);

    let EmailAddressFields {
        subject,
        from_email,
        to_emails,
        cc_emails,
        bcc_emails,
        date,
        message_id,
        ..
    } = addresses;

    Ok(ParsedEmailContent {
        result: EmailExtractionResult {
            subject,
            from_email,
            to_emails,
            cc_emails,
            bcc_emails,
            date,
            message_id,
            plain_text,
            html_content,
            content,
            attachments,
            metadata,
        },
        nested_messages,
        nested_embedded_messages: Vec::new(),
        warnings: Vec::new(),
    })
}

pub(super) fn collect_nested_message_payloads(
    message: &mail_parser::Message<'_>,
    max_nested_message_bytes: Option<u64>,
) -> Vec<NestedMessagePayload> {
    use mail_parser::PartType;

    message
        .parts
        .iter()
        .enumerate()
        .filter_map(|(part_id, part)| match &part.body {
            PartType::Message(nested) if !nested.raw_message().is_empty() => {
                let raw_message = nested.raw_message();
                let data = max_nested_message_bytes
                    .is_none_or(|cap| raw_message.len() as u64 <= cap)
                    .then(|| Bytes::copy_from_slice(raw_message));
                Some(NestedMessagePayload {
                    part_id,
                    data,
                    size: raw_message.len(),
                })
            }
            _ => None,
        })
        .collect()
}

/// Check whether a message has at least one genuine `text/plain` body part.
///
/// `mail-parser`'s `body_text()` auto-converts HTML to plain text using a naive
/// tag-stripper that doesn't remove `<script>` or `<style>` content. This helper
/// inspects the actual `PartType` of each `text_body` entry to determine if a
/// real `text/plain` part exists. For HTML-only messages (all text_body entries
/// are `PartType::Html`), callers should use `clean_html_content()` instead.
pub(super) fn has_genuine_text_body(message: &mail_parser::Message<'_>) -> bool {
    use mail_parser::PartType;
    for &part_id in &message.text_body {
        if let Some(part) = message.parts.get(part_id as usize)
            && matches!(&part.body, PartType::Text(_))
        {
            return true;
        }
    }
    false
}

/// Check whether a message has at least one genuine `text/html` body part.
///
/// `mail-parser`'s `body_html()` auto-converts text/plain bodies into HTML when
/// no real HTML part exists. This helper inspects each `html_body` entry's
/// `PartType` so callers can avoid treating plain-text emails as HTML.
pub(super) fn has_genuine_html_body(message: &mail_parser::Message<'_>) -> bool {
    use mail_parser::PartType;
    for &part_id in &message.html_body {
        if let Some(part) = message.parts.get(part_id as usize)
            && matches!(&part.body, PartType::Html(_))
        {
            return true;
        }
    }
    false
}

/// Recursively collect plain text from nested `message/rfc822` sub-messages.
///
/// In `multipart/digest` emails, each part is itself an RFC822 message stored as
/// `PartType::Message`. The top-level `body_text()` won't return these; we must
/// recurse into the nested `Message` to extract their text bodies.
pub(super) fn collect_nested_message_text(message: &mail_parser::Message<'_>, out: &mut Vec<String>) {
    use mail_parser::PartType;
    for part in &message.parts {
        if let PartType::Message(sub_msg) = &part.body {
            let mut i = 0;
            while let Some(text) = sub_msg.body_text(i) {
                out.push(text.to_string());
                i += 1;
            }
            collect_nested_message_text(sub_msg, out);
        }
    }
}

/// Recursively collect HTML from nested `message/rfc822` sub-messages.
pub(super) fn collect_nested_message_html(message: &mail_parser::Message<'_>, out: &mut Vec<String>) {
    use mail_parser::PartType;
    for part in &message.parts {
        if let PartType::Message(sub_msg) = &part.body {
            let mut i = 0;
            while let Some(html) = sub_msg.body_html(i) {
                out.push(html.to_string());
                i += 1;
            }
            collect_nested_message_html(sub_msg, out);
        }
    }
}

/// Maximum number of bytes scanned for the Date header when no header/body
/// separator is found in the message.
const DATE_HEADER_SCAN_CAP: usize = 8192;

/// Maximum number of bytes scanned for additional raw headers when no
/// header/body separator is found in the message.
const RAW_HEADER_SCAN_CAP: usize = 16384;

/// Find the byte offset of the end of the header section (the blank line
/// separating headers from body), falling back to `cap` bytes if no
/// separator is found.
///
/// Operates purely on bytes so the returned offset is always safe to use as
/// a slice bound on `data` (byte-slice indexing never panics on non-UTF-8
/// boundaries, unlike `&str` indexing).
pub(super) fn find_header_section_end(data: &[u8], cap: usize) -> usize {
    if let Some(pos) = memchr::memmem::find(data, b"\r\n\r\n") {
        return pos;
    }
    if let Some(pos) = memchr::memmem::find(data, b"\n\n") {
        return pos;
    }
    data.len().min(cap)
}

/// Extract the raw Date header value from email bytes.
///
/// Scans for `Date:` in the header section (before the blank line that separates
/// headers from body) and returns the raw value, handling continuation lines.
///
/// The header section is decoded with `String::from_utf8_lossy` scoped to just
/// the header bytes, so a non-UTF-8 body (or a non-UTF-8 byte sequence split by
/// the scan cap) never causes header parsing to be skipped or to panic.
pub(super) fn extract_raw_date_header(data: &[u8]) -> Option<String> {
    let header_end = find_header_section_end(data, DATE_HEADER_SCAN_CAP);
    let headers = String::from_utf8_lossy(&data[..header_end]);
    let headers = headers.as_ref();

    let mut date_value = None;
    for line in headers.lines() {
        if let Some(val) = line.strip_prefix("Date:").or_else(|| line.strip_prefix("date:")) {
            date_value = Some(val.trim().to_string());
        } else if date_value.is_some() && (line.starts_with(' ') || line.starts_with('\t')) {
            if let Some(ref mut dv) = date_value {
                dv.push(' ');
                dv.push_str(line.trim());
            }
        } else if date_value.is_some() {
            break;
        }
    }

    date_value.filter(|s| !s.is_empty())
}

/// Extract additional raw headers from email bytes.
///
/// Scans for Content-Type, MIME-Version, X-Mailer, User-Agent, List-Id,
/// and List-Unsubscribe headers in the header section.
///
/// The header section is decoded with `String::from_utf8_lossy` scoped to just
/// the header bytes, so a non-UTF-8 body (or a non-UTF-8 byte sequence split by
/// the scan cap) never causes header parsing to be skipped or to panic.
pub(super) fn extract_raw_headers(data: &[u8]) -> HashMap<String, String> {
    let mut headers = HashMap::new();

    let header_end = find_header_section_end(data, RAW_HEADER_SCAN_CAP);
    let header_section = String::from_utf8_lossy(&data[..header_end]);
    let header_section = header_section.as_ref();

    let target_headers: &[(&str, &str)] = &[
        ("content-type:", "content_type"),
        ("mime-version:", "mime_version"),
        ("x-mailer:", "x_mailer"),
        ("user-agent:", "user_agent"),
        ("list-id:", "list_id"),
        ("list-unsubscribe:", "list_unsubscribe"),
    ];

    let mut current_key: Option<&str> = None;
    let mut current_value = String::new();

    for line in header_section.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            if current_key.is_some() {
                current_value.push(' ');
                current_value.push_str(line.trim());
            }
            continue;
        }

        if let Some(key) = current_key {
            if !current_value.is_empty() {
                headers.insert(key.to_string(), current_value.clone());
            }
            current_key = None;
            current_value.clear();
        }

        let line_lower = line.to_lowercase();
        for &(prefix, meta_key) in target_headers {
            if line_lower.starts_with(prefix) {
                current_key = Some(meta_key);
                current_value = line[prefix.len()..].trim().to_string();
                break;
            }
        }
    }

    if let Some(key) = current_key
        && !current_value.is_empty()
    {
        headers.insert(key.to_string(), current_value);
    }

    headers
}
