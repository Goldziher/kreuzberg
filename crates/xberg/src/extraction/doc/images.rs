//! Live inline-picture references in Word's CHPX/PICF records. ~keep

use super::{DOC_WARNING_SOURCE, DocExtractionResult, DocParagraph};
use crate::error::{Result, XbergError};
use crate::extractors::security::{SecurityBudget, SecurityLimits};
use crate::types::{ExtractedImage, ProcessingWarning};
use std::io::{Cursor, Read, Seek, SeekFrom};

// [MS-DOC] CHPX/PICF and [MS-ODRAW] OfficeArt record identifiers. ~keep
const CHPX_FIB_PAIR: usize = 12;
const FKP_PAGE_BYTES: usize = 512;
const FKP_PAGE_MASK: u32 = 0x003f_ffff;
const PICF_HEADER_BYTES: usize = 68;
const MM_SHAPE: u16 = 100;
const MM_SHAPEFILE: u16 = 102;
const SPRM_C_SPEC: u16 = 0x0855;
const SPRM_C_DATA: u16 = 0x0806;
const SPRM_C_PIC_LOCATION: u16 = 0x6a03;
const OFFICE_ART_HEADER_BYTES: usize = 8;
const OFFICE_ART_CONTAINER_VERSION: u16 = 15;
const OFFICE_ART_BSE: u16 = 0xf007;
const OFFICE_ART_JPEG: u16 = 0xf01d;
const OFFICE_ART_JPEG_ALT: u16 = 0xf02a;
const OFFICE_ART_PNG: u16 = 0xf01e;
const BSE_HEADER_BYTES: usize = 36;
const BSE_NAME_LENGTH_OFFSET: usize = 33;
const BLIP_SINGLE_UID_BYTES: usize = 17;
const BLIP_DOUBLE_UID_BYTES: usize = 33;

pub(crate) struct DocWithImages {
    pub text: DocExtractionResult,
    pub pictures: Vec<DocPicture>,
}

pub(crate) struct DocPicture {
    pub paragraph_index: usize,
    pub text_offset: usize,
    pub standalone: bool,
    pub image: ExtractedImage,
}

pub(super) struct PicturePosition {
    fc_end: u32,
    paragraph_index: usize,
    text_offset: usize,
    standalone: bool,
    supported: bool,
}

pub(super) fn record_position(
    pending: &mut Vec<(u32, usize)>,
    fc_end: Option<u32>,
    raw_offset: usize,
    limits: Option<&SecurityLimits>,
    count: &mut usize,
) -> Result<()> {
    let Some(limits) = limits else { return Ok(()) };
    if *count >= limits.max_files_in_archive {
        return Err(XbergError::validation(
            "DOC picture reference count exceeds the configured limit",
        ));
    }
    *count += 1;
    if let Some(fc_end) = fc_end {
        pending.push((fc_end, raw_offset));
    }
    Ok(())
}

pub(super) fn place_pictures(
    pictures: &mut Vec<PicturePosition>,
    pending: &mut Vec<(u32, usize)>,
    raw: &str,
    paragraph_index: usize,
    paragraph: Option<&DocParagraph>,
    supported: bool,
) {
    if pending.is_empty() {
        return;
    }
    map_picture_offsets(raw, pending);
    for (fc_end, text_offset) in pending.drain(..) {
        pictures.push(PicturePosition {
            fc_end,
            paragraph_index,
            text_offset,
            standalone: paragraph.is_none(),
            supported,
        });
    }
}

fn map_picture_offsets(raw: &str, pending: &mut [(u32, usize)]) {
    let mut normalized = String::new();
    let mut picture = 0;
    let mut newlines = 0;
    for (offset, character) in raw.char_indices() {
        if pending.get(picture).is_some_and(|(_, position)| *position == offset) {
            pending[picture].1 = normalized.len();
            picture += 1;
        }
        if let Some(character) = super::normalized_doc_character(character) {
            newlines = if character == '\n' { newlines + 1 } else { 0 };
            if newlines <= 2 {
                normalized.push(character);
            }
        }
    }
    let leading = normalized.len() - normalized.trim_start().len();
    let length = normalized.trim().len();
    for (_, offset) in pending {
        *offset = offset.saturating_sub(leading).min(length);
    }
}

pub(super) fn warn_unsupported_fallback(requested: bool, warnings: &mut Vec<ProcessingWarning>, reason: &str) {
    if requested {
        warn(
            warnings,
            &format!("Embedded pictures in DOC {reason} are not extracted"),
        );
    }
}

pub(super) fn warn(warnings: &mut Vec<ProcessingWarning>, message: &str) {
    crate::core::diagnostics::push_warning(warnings, DOC_WARNING_SOURCE, message);
}

fn word(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        data.get(offset..offset.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn dword(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        data.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn validation(error: crate::extractors::security::SecurityError) -> XbergError {
    XbergError::validation(error.to_string())
}

pub(super) fn extract_pictures(
    compound: &mut cfb::CompoundFile<Cursor<&[u8]>>,
    word_doc: &[u8],
    table: &[u8],
    positions: &[PicturePosition],
    limits: &SecurityLimits,
    warnings: &mut Vec<ProcessingWarning>,
) -> Result<Vec<DocPicture>> {
    let mut pictures = Vec::new();
    let mut budget = SecurityBudget::from_limits(limits);
    for position in positions {
        budget.step().map_err(validation)?;
        if !position.supported {
            warn(
                warnings,
                "Pictures in DOC table cells or automatic lists are not extracted",
            );
            continue;
        }
        let Some(offset) = picture_offset(word_doc, table, position.fc_end, &mut budget)? else {
            warn(
                warnings,
                "DOC picture has no supported live character-property reference; skipped",
            );
            continue;
        };
        if pictures.len() >= limits.max_files_in_archive {
            return Err(XbergError::validation(
                "DOC embedded picture count exceeds the configured limit",
            ));
        }
        match read_picture(compound, offset, limits, &mut budget) {
            Ok(Some(mut image)) => {
                image.image_index = pictures.len() as u32;
                pictures.push(DocPicture {
                    paragraph_index: position.paragraph_index,
                    text_offset: position.text_offset,
                    standalone: position.standalone,
                    image,
                });
            }
            Ok(None) => warn(
                warnings,
                "DOC embedded picture is malformed or uses an unsupported image format; skipped",
            ),
            Err(error) => return Err(error),
        }
    }
    Ok(pictures)
}

fn chpx_table<'a>(word_doc: &[u8], table: &'a [u8]) -> Option<&'a [u8]> {
    let cslw_offset = 34 + usize::from(word(word_doc, 32)?) * 2;
    let pairs = cslw_offset + 4 + usize::from(word(word_doc, cslw_offset)?) * 4;
    let pair = pairs + CHPX_FIB_PAIR * 8;
    let start = dword(word_doc, pair)? as usize;
    let length = dword(word_doc, pair + 4)? as usize;
    table.get(start..start.checked_add(length)?)
}

fn picture_offset(word_doc: &[u8], table: &[u8], fc_end: u32, budget: &mut SecurityBudget) -> Result<Option<u32>> {
    let Some(bte) = chpx_table(word_doc, table).filter(|bytes| bytes.len() >= 12 && (bytes.len() - 4) % 8 == 0) else {
        return Ok(None);
    };
    let count = (bte.len() - 4) / 8;
    for index in 0..count {
        budget.step().map_err(validation)?;
        if dword(bte, index * 4) >= dword(bte, (index + 1) * 4) {
            return Ok(None);
        }
    }
    for index in 0..count {
        budget.step().map_err(validation)?;
        let (Some(first), Some(last)) = (dword(bte, index * 4), dword(bte, (index + 1) * 4)) else {
            return Ok(None);
        };
        if first >= last {
            return Ok(None);
        }
        if first >= fc_end || fc_end > last {
            continue;
        }
        let Some(page) = dword(bte, 4 * (count + 1 + index)) else {
            break;
        };
        let start = (page & FKP_PAGE_MASK) as usize * FKP_PAGE_BYTES;
        let Some(fkp) = word_doc.get(start..start + FKP_PAGE_BYTES) else {
            continue;
        };
        if let Some(offset) = fkp_picture_offset(fkp, fc_end, budget)? {
            return Ok(Some(offset));
        }
    }
    Ok(None)
}

fn fkp_picture_offset(fkp: &[u8], fc_end: u32, budget: &mut SecurityBudget) -> Result<Option<u32>> {
    let count = usize::from(fkp[FKP_PAGE_BYTES - 1]);
    if 4 * (count + 1) + count > FKP_PAGE_BYTES - 1 {
        return Ok(None);
    }
    for index in 0..count {
        budget.step().map_err(validation)?;
        let (Some(start), Some(end)) = (dword(fkp, index * 4), dword(fkp, (index + 1) * 4)) else {
            break;
        };
        if start >= fc_end || fc_end > end {
            continue;
        }
        let offset = usize::from(fkp[4 * (count + 1) + index]) * 2;
        if offset == 0 {
            return Ok(None);
        }
        let Some(&length) = fkp.get(offset) else {
            return Ok(None);
        };
        let Some(properties) = fkp.get(offset + 1..offset + 1 + usize::from(length)) else {
            return Ok(None);
        };
        return Ok(pic_location(properties));
    }
    Ok(None)
}

fn pic_location(properties: &[u8]) -> Option<u32> {
    let mut position = 0;
    let (mut special, mut binary, mut location) = (false, false, None);
    while let Some(sprm) = word(properties, position) {
        position += 2;
        let length = match sprm >> 13 {
            0 | 1 => 1,
            2 | 4 | 5 => 2,
            3 => 4,
            7 => 3,
            _ => {
                let length = usize::from(*properties.get(position)?);
                position += 1;
                length
            }
        };
        let operand = properties.get(position..position.checked_add(length)?)?;
        match sprm {
            SPRM_C_SPEC => special = operand[0] == 1,
            SPRM_C_DATA => binary = operand[0] != 0,
            SPRM_C_PIC_LOCATION => location = dword(operand, 0).filter(|offset| *offset <= i32::MAX as u32),
            _ => {}
        }
        position += length;
    }
    location.filter(|_| special && !binary && position == properties.len())
}

fn read_picture(
    compound: &mut cfb::CompoundFile<Cursor<&[u8]>>,
    offset: u32,
    limits: &SecurityLimits,
    budget: &mut SecurityBudget,
) -> Result<Option<ExtractedImage>> {
    let Ok(mut stream) = compound.open_stream("/Data") else {
        return Ok(None);
    };
    let Ok(stream_len) = stream.seek(SeekFrom::End(0)) else {
        return Ok(None);
    };
    if u64::from(offset) + 6 > stream_len || stream.seek(SeekFrom::Start(u64::from(offset))).is_err() {
        return Ok(None);
    }
    let mut header = [0; 6];
    if stream.read_exact(&mut header).is_err() {
        return Ok(None);
    }
    let length = dword(&header, 0).unwrap_or(0) as usize;
    let header_length = usize::from(word(&header, 4).unwrap_or(0));
    if header_length < PICF_HEADER_BYTES || length < header_length || u64::from(offset) + length as u64 > stream_len {
        return Ok(None);
    }
    if length > limits.max_content_size {
        return Err(XbergError::validation(
            "DOC picture exceeds the configured content limit",
        ));
    }
    budget.account_text(length).map_err(validation)?;
    let mut data = vec![0; length];
    data[..6].copy_from_slice(&header);
    if stream.read_exact(&mut data[6..]).is_err() {
        return Ok(None);
    }
    let start = match word(&data, 6) {
        Some(MM_SHAPE) => header_length,
        Some(MM_SHAPEFILE) => match data.get(header_length) {
            Some(length) => header_length + 1 + usize::from(*length),
            None => return Ok(None),
        },
        _ => return Ok(None),
    };
    match data.get(start..) {
        Some(records) => raster_record(records, budget),
        None => Ok(None),
    }
}

fn raster_record(data: &[u8], budget: &mut SecurityBudget) -> Result<Option<ExtractedImage>> {
    let mut position = 0;
    while position < data.len() {
        budget.step().map_err(validation)?;
        let (Some(packed), Some(kind), Some(length)) = (
            word(data, position),
            word(data, position + 2),
            dword(data, position + 4),
        ) else {
            return Ok(None);
        };
        let start = position + OFFICE_ART_HEADER_BYTES;
        let Some(end) = start.checked_add(length as usize).filter(|end| *end <= data.len()) else {
            return Ok(None);
        };
        let payload = &data[start..end];
        if packed & OFFICE_ART_CONTAINER_VERSION == OFFICE_ART_CONTAINER_VERSION || kind == OFFICE_ART_BSE {
            let children = if kind == OFFICE_ART_BSE {
                let Some(name_length) = payload.get(BSE_NAME_LENGTH_OFFSET) else {
                    return Ok(None);
                };
                payload.get(BSE_HEADER_BYTES + usize::from(*name_length)..)
            } else {
                Some(payload)
            };
            budget.enter().map_err(validation)?;
            let result = match children {
                Some(children) => raster_record(children, budget)?,
                None => None,
            };
            budget.leave();
            if result.is_some() {
                return Ok(result);
            }
        } else if let Some(format) = match kind {
            OFFICE_ART_JPEG | OFFICE_ART_JPEG_ALT => Some("jpeg"),
            OFFICE_ART_PNG => Some("png"),
            _ => None,
        } {
            let header = if (packed >> 4) & 1 == 1 {
                BLIP_DOUBLE_UID_BYTES
            } else {
                BLIP_SINGLE_UID_BYTES
            };
            let Some(bytes) = payload.get(header..).filter(|bytes| !bytes.is_empty()) else {
                return Ok(None);
            };
            return Ok(Some(ExtractedImage {
                data: bytes::Bytes::copy_from_slice(bytes),
                format: format.into(),
                ..Default::default()
            }));
        }
        position = end;
    }
    Ok(None)
}
