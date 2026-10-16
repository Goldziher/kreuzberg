use super::msg::direct_child_storage_paths;
use super::*;

/// Read a raw CFB stream by path; returns `None` for missing or empty streams.
pub(super) fn read_msg_stream<F: std::io::Read + std::io::Seek>(
    comp: &mut cfb::CompoundFile<F>,
    path: &str,
) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut stream = comp.open_stream(path).ok()?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).ok()?;
    if buf.is_empty() { None } else { Some(buf) }
}

/// Read a PT_LONG (0x0003) integer property from a message-level
/// `__properties_version1.0` stream.
///
/// `message_root` is `""` for the top-level MSG message (32-byte property
/// stream header per MS-OXMSG 2.4.3), or the CFB storage path of an embedded
/// message (24-byte header per MS-OXMSG 2.4.3, since the top-level's leading
/// 8 reserved bytes are omitted for embedded messages). Use
/// [`read_msg_attach_or_recip_int_prop`] instead for attachment- or
/// recipient-level properties, whose header is always 8 bytes.
pub(super) fn read_msg_int_prop<F: std::io::Read + std::io::Seek>(
    comp: &mut cfb::CompoundFile<F>,
    message_root: &str,
    prop_id: u16,
) -> Option<u32> {
    use std::io::Read;

    let props_path = format!("{message_root}/__properties_version1.0");
    let mut stream = comp.open_stream(&props_path).ok()?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).ok()?;

    const TOP_LEVEL_HEADER_SIZE: usize = 32;
    const EMBEDDED_MESSAGE_HEADER_SIZE: usize = 24;
    let header_size: usize = if message_root.is_empty() {
        TOP_LEVEL_HEADER_SIZE
    } else {
        EMBEDDED_MESSAGE_HEADER_SIZE
    };
    let mut offset = header_size;

    while offset + 16 <= buf.len() {
        let ptype = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        let pid = u16::from_le_bytes([buf[offset + 2], buf[offset + 3]]);

        if pid == prop_id && ptype == 0x0003 {
            return Some(u32::from_le_bytes(buf[offset + 8..offset + 12].try_into().ok()?));
        }
        offset += 16;
    }
    None
}

/// Read a PT_LONG (0x0003) integer property from an attachment's or
/// recipient's `__properties_version1.0` stream, whose header is always 8
/// bytes (MS-OXMSG 2.4.4/2.4.5) regardless of whether the parent message is
/// the top-level message or an embedded message.
pub(super) fn read_msg_attach_or_recip_int_prop<F: std::io::Read + std::io::Seek>(
    comp: &mut cfb::CompoundFile<F>,
    base: &str,
    prop_id: u16,
) -> Option<u32> {
    use std::io::Read;

    const ATTACH_OR_RECIP_HEADER_SIZE: usize = 8;

    let props_path = format!("{base}/__properties_version1.0");
    let mut stream = comp.open_stream(&props_path).ok()?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).ok()?;

    let mut offset = ATTACH_OR_RECIP_HEADER_SIZE;
    while offset + 16 <= buf.len() {
        let ptype = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        let pid = u16::from_le_bytes([buf[offset + 2], buf[offset + 3]]);

        if pid == prop_id && ptype == 0x0003 {
            return Some(u32::from_le_bytes(buf[offset + 8..offset + 12].try_into().ok()?));
        }
        offset += 16;
    }
    None
}

/// Read a MAPI string property (tries PT_UNICODE then PT_STRING8).
///
/// `codepage` is the Windows code page to use when decoding PT_STRING8 bytes.
/// Pass `None` to fall back to windows-1252 (safe default for legacy MSG files).
pub(super) fn read_msg_string_prop<F: std::io::Read + std::io::Seek>(
    comp: &mut cfb::CompoundFile<F>,
    base: &str,
    prop_id: u16,
    codepage: Option<u32>,
) -> Option<String> {
    let unicode_path = format!("{base}/__substg1.0_{prop_id:04X}001F");
    if let Some(buf) = read_msg_stream(comp, &unicode_path) {
        return Some(decode_utf16le_bytes(&buf));
    }
    let ansi_path = format!("{base}/__substg1.0_{prop_id:04X}001E");
    read_msg_stream(comp, &ansi_path).map(|buf| {
        let encoding = codepage
            .map(encoding_for_windows_codepage)
            .unwrap_or(encoding_rs::WINDOWS_1252);
        let (decoded, _, _) = encoding.decode(&buf);
        decoded.trim_end_matches('\0').to_string()
    })
}

/// Read the PidTagHtml property, whose canonical MAPI type is PT_BINARY.
///
/// Some producers use a string-typed property instead, so retain those probes
/// before falling back to the standard binary stream. ~keep
pub(super) fn read_msg_html_prop<F: std::io::Read + std::io::Seek>(
    comp: &mut cfb::CompoundFile<F>,
    base: &str,
    codepage: Option<u32>,
) -> Option<String> {
    read_msg_string_prop(comp, base, PID_TAG_HTML, codepage).or_else(|| {
        let path = format!("{base}/__substg1.0_{PID_TAG_HTML:04X}0102");
        let buf = read_msg_stream(comp, &path)?;
        let decoded = if let Some(codepage) = codepage {
            let (text, _, _) = encoding_for_windows_codepage(codepage).decode(&buf);
            text.trim_end_matches('\0').to_string()
        } else {
            match utf8_validation::from_utf8(&buf) {
                Ok(text) => text.trim_start_matches('\u{FEFF}').trim_end_matches('\0').to_string(),
                Err(_) => {
                    let encoding = encoding_rs::WINDOWS_1252;
                    let (text, _, _) = encoding.decode(&buf);
                    text.trim_end_matches('\0').to_string()
                }
            }
        };
        Some(decoded)
    })
}

/// Decode UTF-16LE bytes to a String, stripping trailing NUL chars.
pub(super) fn decode_utf16le_bytes(data: &[u8]) -> String {
    let u16s: Vec<u16> = data
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16_lossy(&u16s).trim_end_matches('\0').to_string()
}

/// Read a PT_SYSTIME (FILETIME) property from the __properties_version1.0 stream
/// and convert it to an ISO 8601 date string.
///
/// FILETIME is a 64-bit value representing 100-nanosecond intervals since 1601-01-01.
pub(super) fn read_msg_filetime_prop<F: std::io::Read + std::io::Seek>(
    comp: &mut cfb::CompoundFile<F>,
    base: &str,
    prop_id: u16,
) -> Option<String> {
    use std::io::Read;

    let props_path = format!("{base}/__properties_version1.0");
    let mut stream = comp.open_stream(&props_path).ok()?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).ok()?;

    const TOP_LEVEL_HEADER_SIZE: usize = 32;
    const EMBEDDED_MESSAGE_HEADER_SIZE: usize = 24;
    let header_size: usize = if base.is_empty() {
        TOP_LEVEL_HEADER_SIZE
    } else {
        EMBEDDED_MESSAGE_HEADER_SIZE
    };
    let mut offset = header_size;

    while offset + 16 <= buf.len() {
        let ptype = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        let pid = u16::from_le_bytes([buf[offset + 2], buf[offset + 3]]);

        if pid == prop_id && ptype == 0x0040 {
            let filetime = u64::from_le_bytes(buf[offset + 8..offset + 16].try_into().ok()?);
            return filetime_to_iso8601(filetime);
        }
        offset += 16;
    }
    None
}

/// Convert a Windows FILETIME (100-ns intervals since 1601-01-01) to ISO 8601.
pub(super) fn filetime_to_iso8601(filetime: u64) -> Option<String> {
    const EPOCH_DIFF: u64 = 116_444_736_000_000_000;
    if filetime < EPOCH_DIFF {
        return None;
    }
    let hundred_ns = filetime - EPOCH_DIFF;
    let secs = (hundred_ns / 10_000_000) as i64;
    let nanos = ((hundred_ns % 10_000_000) * 100) as u32;

    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let (hour, min, sec) = (time_of_day / 3600, (time_of_day % 3600) / 60, time_of_day % 60);

    let z = days_since_epoch + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    if nanos == 0 {
        Some(format!("{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{sec:02}+00:00"))
    } else {
        let frac = nanos / 1_000_000;
        Some(format!(
            "{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{sec:02}.{frac:03}+00:00"
        ))
    }
}

/// Read recipients from MSG __recip_version1.0_#XXXXXXXX substorages.
///
/// Returns (to, cc, bcc) vectors. Each entry is formatted as `"Name" <email>` or just `email`.
pub(super) fn read_msg_recipients<F: std::io::Read + std::io::Seek>(
    comp: &mut cfb::CompoundFile<F>,
    message_root: &str,
    codepage: Option<u32>,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let recip_paths = direct_child_storage_paths(comp, message_root, "__recip_version1.0_");

    let mut to_emails = Vec::new();
    let mut cc_emails = Vec::new();
    let mut bcc_emails = Vec::new();

    for path in &recip_paths {
        let display_name = read_msg_string_prop(comp, path, 0x3001, codepage);
        let email_addr = read_msg_string_prop(comp, path, 0x39FE, codepage)
            .or_else(|| read_msg_string_prop(comp, path, 0x3003, codepage))
            .filter(|s| !s.is_empty());

        let formatted = match (&display_name, &email_addr) {
            (Some(name), Some(email)) if !name.is_empty() && name != email => {
                format!("\"{}\" <{}>", name, email)
            }
            (_, Some(email)) => email.clone(),
            (Some(name), None) if !name.is_empty() => name.clone(),
            _ => continue,
        };

        let recip_type = read_msg_recip_type(comp, path);
        match recip_type {
            1 => to_emails.push(formatted),
            2 => cc_emails.push(formatted),
            3 => bcc_emails.push(formatted),
            _ => to_emails.push(formatted),
        }
    }

    (to_emails, cc_emails, bcc_emails)
}

/// Read PR_RECIPIENT_TYPE (0x0C15) from a recipient's __properties_version1.0 stream.
/// Returns 1 (To), 2 (CC), 3 (BCC), or 0 if not found.
pub(super) fn read_msg_recip_type<F: std::io::Read + std::io::Seek>(
    comp: &mut cfb::CompoundFile<F>,
    base: &str,
) -> u32 {
    use std::io::Read;

    let props_path = format!("{base}/__properties_version1.0");
    let mut stream = match comp.open_stream(&props_path) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let mut buf = Vec::new();
    if stream.read_to_end(&mut buf).is_err() {
        return 0;
    }

    let mut offset = 8;
    while offset + 16 <= buf.len() {
        let ptype = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        let pid = u16::from_le_bytes([buf[offset + 2], buf[offset + 3]]);

        if pid == 0x0C15 && ptype == 0x0003 {
            return u32::from_le_bytes([buf[offset + 8], buf[offset + 9], buf[offset + 10], buf[offset + 11]]);
        }
        offset += 16;
    }
    0
}
