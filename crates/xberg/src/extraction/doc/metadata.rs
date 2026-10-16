//! OLE summary-information (metadata) parsing for legacy `.doc` files.

use std::io::Cursor;

use super::DocMetadata;
use super::read_stream;

/// Extract metadata from OLE summary information streams.
pub(super) fn extract_doc_metadata(comp: &mut cfb::CompoundFile<Cursor<&[u8]>>) -> DocMetadata {
    let mut meta = DocMetadata::default();

    if let Ok(data) = read_stream(comp, "/\x05SummaryInformation") {
        parse_summary_info(&data, &mut meta);
    }

    if let Ok(data) = read_stream(comp, "/\x05DocumentSummaryInformation") {
        parse_doc_summary_info(&data, &mut meta);
    }

    meta
}

/// Parse OLE SummaryInformation property set.
fn parse_summary_info(data: &[u8], meta: &mut DocMetadata) {
    if data.len() < 28 {
        return;
    }

    let offset = 24;
    if data.len() < offset + 4 {
        return;
    }

    let num_sets = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]) as usize;
    if num_sets == 0 {
        return;
    }

    if data.len() < 48 {
        return;
    }
    let set_offset = u32::from_le_bytes([data[44], data[45], data[46], data[47]]) as usize;

    parse_property_set(data, set_offset, meta, false);
}

/// Parse OLE DocumentSummaryInformation property set.
fn parse_doc_summary_info(data: &[u8], meta: &mut DocMetadata) {
    if data.len() < 48 {
        return;
    }

    let set_offset = u32::from_le_bytes([data[44], data[45], data[46], data[47]]) as usize;

    parse_property_set(data, set_offset, meta, true);
}

/// Parse a single property set from OLE property data.
fn parse_property_set(data: &[u8], set_offset: usize, meta: &mut DocMetadata, _is_doc_summary: bool) {
    if set_offset + 8 > data.len() {
        return;
    }

    let num_props = u32::from_le_bytes([
        data[set_offset + 4],
        data[set_offset + 5],
        data[set_offset + 6],
        data[set_offset + 7],
    ]) as usize;

    let props_start = set_offset + 8;

    for i in 0..num_props {
        let entry_offset = props_start + i * 8;
        if entry_offset + 8 > data.len() {
            break;
        }

        let prop_id = u32::from_le_bytes([
            data[entry_offset],
            data[entry_offset + 1],
            data[entry_offset + 2],
            data[entry_offset + 3],
        ]);
        let prop_offset = u32::from_le_bytes([
            data[entry_offset + 4],
            data[entry_offset + 5],
            data[entry_offset + 6],
            data[entry_offset + 7],
        ]) as usize;

        let abs_offset = set_offset + prop_offset;
        if abs_offset + 8 > data.len() {
            continue;
        }

        if let Some(value) = read_property_value(data, abs_offset) {
            match prop_id {
                2 => meta.title = Some(value),
                3 => meta.subject = Some(value),
                4 => meta.author = Some(value),
                8 => meta.last_author = Some(value),
                9 => meta.revision_number = Some(value),
                _ => {}
            }
        }
    }
}

/// Read a property value from an OLE property entry.
fn read_property_value(data: &[u8], offset: usize) -> Option<String> {
    if offset + 8 > data.len() {
        return None;
    }

    let vt_type = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);

    match vt_type {
        30 => {
            let len =
                u32::from_le_bytes([data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]]) as usize;
            if len == 0 || offset + 8 + len > data.len() {
                return None;
            }
            let bytes = &data[offset + 8..offset + 8 + len];
            let trimmed = bytes.iter().take_while(|&&b| b != 0).copied().collect::<Vec<_>>();
            Some(String::from_utf8_lossy(&trimmed).to_string())
        }
        31 => {
            let len =
                u32::from_le_bytes([data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]]) as usize;
            if len == 0 || offset + 8 + len * 2 > data.len() {
                return None;
            }
            let bytes = &data[offset + 8..offset + 8 + len * 2];
            let chars: Vec<u16> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .take_while(|&c| c != 0)
                .collect();
            Some(String::from_utf16_lossy(&chars))
        }
        _ => None,
    }
}
