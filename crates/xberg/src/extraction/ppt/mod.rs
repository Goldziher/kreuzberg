//! Native PPT (PowerPoint 97-2003) text extraction.
//!
//! Extracts text directly from PowerPoint Binary File Format using OLE/CFB
//! compound document parsing, without requiring LibreOffice.
//!
//! Supports PowerPoint 97, 2000, XP, and 2003 (.ppt) files.

mod embedded;
pub(crate) use embedded::extract_ppt_embedded_objects;

use crate::error::{Result, XbergError};
use crate::types::{ExtractedImage, ProcessingWarning};
use bytes::Bytes;
use std::borrow::Cow;
use std::io::Cursor;

/// Warning source tag for `.ppt` extraction diagnostics (#171 convention).
pub(super) const PPT_WARNING_SOURCE: &str = "ppt";

/// Result of PPT text extraction.
#[cfg_attr(alef, alef(skip))]
pub struct PptExtractionResult {
    /// Full document text: every slide's text joined by double newlines,
    /// kept for diagnostics/plain-text consumers. Slide *structure* (numbers,
    /// per-slide boundaries) must come from `slides`, not from re-splitting
    /// this string (see #1418 -- a slide's own text can itself contain an
    /// internal "\n\n", which makes re-splitting on it ambiguous).
    pub text: String,
    /// Per-slide text, in persist order. `number` is the slide's 1-based
    /// position among `RT_SLIDE` containers as they occur in the
    /// "PowerPoint Document" stream -- the same order `slide_count` counts.
    /// A slide with no text atoms still gets an entry (with an empty
    /// `text`), so slide numbers stay contiguous with the real deck.
    pub slides: Vec<PptSlideText>,
    /// Number of slides found.
    pub slide_count: usize,
    /// Document metadata.
    pub metadata: PptMetadata,
    /// Speaker notes text per slide (if available).
    pub speaker_notes: Vec<String>,
    /// Pictures recovered from the OLE `Pictures` stream (raw
    /// `OfficeArtBlip` payloads). Empty when the deck has no `Pictures`
    /// stream, the stream is empty, or image extraction was not requested.
    /// `page_number` is the slide whose drawing displays the picture, resolved
    /// through `pib` -> `msofbtBSE.foDelay`; it stays `None` for a blip no live
    /// shape references (#1620).
    pub images: Vec<ExtractedImage>,
    /// Non-fatal degradations encountered while extracting (see
    /// `core::diagnostics`). Empty when extraction was complete.
    pub processing_warnings: Vec<ProcessingWarning>,
}

/// One slide's text, numbered by its position in the deck's own persist
/// order rather than by the position of a text block in a joined string.
#[cfg_attr(alef, alef(skip))]
pub struct PptSlideText {
    /// 1-based slide number, as encountered in persist order.
    pub number: u32,
    /// The slide's text (its atoms joined by `\n`). Empty for a slide with
    /// no text.
    pub text: String,
    /// The slide's title, read from the outline collection's own `TextHeaderAtom` (`Title` /
    /// `CenterTitle`) rather than guessed from `text`'s first line (xberg-io/xberg#1635).
    /// `None` when the file states no outline title for this slide -- a deck whose title is
    /// drawn on the canvas and never entered in the outline view still has one, just not
    /// here; the consumer's first-line fallback covers that case.
    pub title: Option<String>,
    /// This slide's speaker notes, resolved via `NotesAtom.slideIdRef` against the live
    /// `SlideListWithText` (xberg-io/xberg#1640) -- not by position among the deck's
    /// non-empty notes pages, which misattributes a note as soon as one slide in between
    /// has none. `None` when the slide has no notes, or the file cannot be read that way.
    pub notes: Option<String>,
    /// The slide's tables, each as rows of cell text, in the order they are drawn. A table's
    /// cells are not also in `text`. Empty for a slide with no table.
    pub tables: Vec<Vec<Vec<String>>>,
}

/// Metadata extracted from PPT files.
#[cfg_attr(alef, alef(skip))]
#[derive(Default)]
pub struct PptMetadata {
    /// Presentation title from the OLE summary information.
    pub title: Option<String>,
    /// Presentation subject from the OLE summary information.
    pub subject: Option<String>,
    /// Original author from the OLE summary information.
    pub author: Option<String>,
    /// Most recent editor from the OLE summary information.
    pub last_author: Option<String>,
}

const RT_TEXT_CHARS_ATOM: u16 = 0x0FA0;
const RT_TEXT_BYTES_ATOM: u16 = 0x0FA8;
/// A single slide's persisted content container (SlideAtom + shapes/text).
/// `SlideListWithText` (0x0FF0), by contrast, is a per-document container of
/// `SlidePersistAtom` entries used for the outline view -- it does not
/// enclose the slides' actual text and does not occur once per slide, so it
/// cannot be used as a slide boundary (#87).
const RT_SLIDE: u16 = 0x03EE;
const RT_MAIN_MASTER: u16 = 0x03F8;
/// Document-level outline collection. A legacy deck keeps a slide's title here rather than in
/// the slide's own drawing, and reading only the drawing lost every such title
/// (xberg-io/xberg#1612). This is NOT a slide boundary -- see
/// `test_extract_texts_segments_on_slide_not_slide_list_with_text`. ~keep
const RT_SLIDE_LIST_WITH_TEXT: u16 = 0x0FF0;
/// Introduces one slide's entries inside [`RT_SLIDE_LIST_WITH_TEXT`]; the nth such atom starts
/// the outline records belonging to the nth slide, which is how outline text is attributed. ~keep
const RT_SLIDE_PERSIST_ATOM: u16 = 0x03F3;
/// Byte offset of `SlidePersistAtom.slideId` within its 20-byte payload, past
/// `persistIdRef` (4), `flags` (4) and `numberTexts` (4) (MS-PPT 2.4.14). What
/// `NotesAtom.slideIdRef` names a notes page's slide by -- a different value than
/// `persistIdRef`, which only orders the persist directory (xberg-io/xberg#1640). ~keep
const SLIDE_PERSIST_ATOM_SLIDE_ID_OFFSET: usize = 12;
const RT_NOTES: u16 = 0x03F0;
/// Inside an [`RT_NOTES`] container, states which slide the notes page belongs to
/// (MS-PPT 2.5.7). ~keep
const RT_NOTES_ATOM: u16 = 0x03F1;
/// `NotesAtom.slideIdRef` sentinel meaning "this is the notes master", not a per-slide
/// notes page. Its placeholder text ("Click to edit Master text styles...") must never
/// reach `speaker_notes` (xberg-io/xberg#1640). ~keep
const NOTES_MASTER_SLIDE_ID_REF: u32 = 0x8000_0000;
/// Precedes a text atom inside [`RT_SLIDE_LIST_WITH_TEXT`] and types it (MS-PPT 2.13.33
/// `TextTypeEnum`); read here only to tell a slide's outline *title* apart from its outline
/// *body* (xberg-io/xberg#1635). ~keep
const RT_TEXT_HEADER_ATOM: u16 = 0x0F9F;
/// `TextHeaderAtom.textType == Title`. ~keep
const TEXT_TYPE_TITLE: u32 = 0;
/// `TextHeaderAtom.textType == CenterTitle` -- a title-slide's centred title placeholder.
/// `5` is `CenterBody` (body text, not a title); confirmed against Apache POI's
/// `TextHeaderAtom` constants, which the MS-PPT spec text alone does not make obvious. ~keep
const TEXT_TYPE_CENTER_TITLE: u32 = 6;

/// `OfficeArtBlip` record types for the raster formats a `Pictures` stream
/// can hold (MS-ODRAW 2.2.23). `RT_BLIP_JPEG_ALT` (0xF02A) is an alternate
/// `recType` documented for JPEG blips written by older Office versions; it
/// uses the same `OfficeArtBlipJPEG` layout as 0xF01D.
const RT_BLIP_JPEG: u16 = 0xF01D;
const RT_BLIP_JPEG_ALT: u16 = 0xF02A;
const RT_BLIP_PNG: u16 = 0xF01E;
const RT_BLIP_DIB: u16 = 0xF01F;

/// `msofbtBSE` -- one per blip in the document's `BStoreContainer`. A shape's `pib`
/// property is a 1-based index into these, in container order (MS-ODRAW 2.2.32). ~keep
const MSOFBT_BSE: u16 = 0xF007;
/// Byte offset of `foDelay` inside an `msofbtBSE`'s content, past `btWin32` (1), `btMacOS`
/// (1), `rgbUid` (16), `tag` (2), `size` (4) and `cRef` (4). `foDelay` is the blip record's
/// own start offset in the `Pictures` stream, which is the key `Pictures` is walked by. ~keep
const BSE_FO_DELAY_OFFSET: usize = 28;

/// `msofbtOPT` and its secondary/tertiary siblings, each a table of `OfficeArtFOPTE`
/// property entries (MS-ODRAW 2.3.1). `pib` is normally in the primary table, but a
/// shape that overflows its property set spills into the other two. ~keep
const MSOFBT_OPT: [u16; 3] = [0xF00B, 0xF121, 0xF122];
/// An `OfficeArtFOPTE` is a 2-byte `opid` followed by a 4-byte value.
const FOPT_ENTRY_LEN: usize = 6;
/// The low 14 bits of `opid` are the property number; bit 14 is `fBid` and bit 15 is
/// `fComplex`, neither of which changes where the entry sits. ~keep
const FOPT_PROPERTY_ID_MASK: u16 = 0x3FFF;
/// `pib` -- the 1-based `BStoreContainer` index of the blip a picture shape displays.
const MSO_PROPERTY_PIB: u16 = 0x0104;

/// Depth cap for the OfficeArt container walk. Real nesting is under a dozen levels; this
/// only stops a hostile self-describing container tree from exhausting the stack. ~keep
const MAX_OFFICE_ART_DEPTH: usize = 16;

/// `UserEditAtom` -- one per save, chained newest-to-oldest through `offsetLastEdit`.
const RT_USER_EDIT_ATOM: u16 = 0x0FF5;
/// `PersistDirectoryAtom` -- maps persist ids to stream offsets for one save.
const RT_PERSIST_DIRECTORY_ATOM: u16 = 0x1772;
/// The deck's own top-level container, holding (among other things) the live
/// `SlideListWithText` (MS-PPT 2.4.1). A `.ppt` stream carries one of these per save;
/// `UserEditAtom.docPersistIdRef` names which one is current (xberg-io/xberg#1639). ~keep
const RT_DOCUMENT: u16 = 0x03E8;

/// Byte offset of `offsetToCurrentEdit` within the `Current User` stream: an 8-byte
/// record header, then `size` (4) and `headerToken` (4) precede it (MS-PPT 2.3.2). ~keep
const CURRENT_USER_OFFSET_TO_CURRENT_EDIT: usize = 16;

/// A `persistId` is the low 20 bits of a `PersistDirectoryEntry`'s first dword; the
/// high 12 bits are `cPersist`, the number of consecutive ids the entry covers
/// (MS-PPT 2.3.6). ~keep
const PERSIST_ID_MASK: u32 = 0x000F_FFFF;
const PERSIST_COUNT_SHIFT: u32 = 20;

/// Upper bound on `UserEditAtom` hops. A `.ppt` accumulates one per save, so a legitimate
/// chain is long but finite; this only exists so a corrupt or hostile `offsetLastEdit`
/// cycle cannot spin forever. Chains longer than this fall back to stream order. ~keep
const MAX_USER_EDIT_CHAIN: usize = 4096;

/// Depth cap for [`slide_persist_ids_in_presentation_order`]'s descent into nested
/// containers. Real decks keep `SlideListWithText` a direct child of `DocumentContainer`;
/// this only stops a hostile self-nesting container from recursing without bound.
const MAX_SLIDE_LIST_SEARCH_DEPTH: usize = 16;

/// Maximum accepted size for a single embedded picture (100 MB), mirroring
/// the DOCX/PPTX image cap (`crate::extraction::docx::MAX_IMAGE_FILE_SIZE`).
/// Bounds allocation from a hostile `recLen` in the untrusted `Pictures`
/// stream.
const MAX_PICTURE_SIZE: usize = 100 * 1024 * 1024;

/// Extract text from PPT bytes.
///
/// Parses the OLE/CFB compound document, reads the "PowerPoint Document" stream,
/// and extracts text from TextCharsAtom and TextBytesAtom records.
///
/// When `include_master_slides` is `true`, master slide content (placeholder text
/// like "Click to edit Master title style") is included instead of being skipped.
#[cfg(test)]
pub(crate) fn extract_ppt_text(content: &[u8]) -> Result<PptExtractionResult> {
    extract_ppt_text_with_options(content, false, true)
}

/// Extract text from PPT bytes with configurable master slide inclusion and
/// image extraction.
///
/// When `include_master_slides` is `true`, `RT_MAIN_MASTER` containers are not
/// skipped, so master slide placeholder text is included in the output.
///
/// When `extract_images` is `true`, the OLE `Pictures` stream (if present)
/// is walked for embedded raster images (#1417).
/// Join every non-empty slide/loose text into the extraction result's flat
/// `text` field. Computed before the synthetic-slide fallback runs, so
/// `loose_texts` is never counted twice (once here, once folded in there).
fn assemble_full_text(slides: &[PptSlideText], loose_texts: &[String]) -> String {
    // A table reads as one line per row, its cells separated by tabs. ~keep
    let tables: Vec<String> = slides
        .iter()
        .map(|s| {
            s.tables
                .iter()
                .map(|table| table.iter().map(|row| row.join("\t")).collect::<Vec<_>>().join("\n"))
                .collect::<Vec<_>>()
                .join("\n\n")
        })
        .collect();
    slides
        .iter()
        .zip(&tables)
        .flat_map(|(s, t)| [s.text.as_str(), t.as_str()])
        .chain(loose_texts.iter().map(String::as_str))
        .filter(|t| !t.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Extract embedded pictures from the `/Pictures` stream, if present and requested.
fn extract_images_if_requested(
    comp: &mut cfb::CompoundFile<Cursor<&[u8]>>,
    ppt_stream: &[u8],
    live_slides: Option<&LiveSlides>,
    extract_images: bool,
    warnings: &mut Vec<ProcessingWarning>,
) -> Vec<ExtractedImage> {
    if !extract_images {
        return Vec::new();
    }
    match read_stream(comp, "/Pictures") {
        Ok(pictures_stream) if !pictures_stream.is_empty() => {
            let slide_numbers = picture_slide_numbers(ppt_stream, live_slides.map(|l| l.offsets.as_slice()));
            extract_pictures_from_stream(&pictures_stream, &slide_numbers, warnings)
        }
        _ => Vec::new(),
    }
}

pub(crate) fn extract_ppt_text_with_options(
    content: &[u8],
    include_master_slides: bool,
    extract_images: bool,
) -> Result<PptExtractionResult> {
    let cursor = Cursor::new(content);
    let mut comp = cfb::CompoundFile::open(cursor)
        .map_err(|e| XbergError::parsing(format!("Failed to open PPT as OLE container: {e}")))?;

    let metadata = extract_ppt_metadata(&mut comp);

    let ppt_stream = read_stream(&mut comp, "/PowerPoint Document")?;
    if ppt_stream.is_empty() {
        return Err(XbergError::parsing("PowerPoint Document stream is empty"));
    }

    let mut processing_warnings = Vec::new();
    // Absent or unreadable `Current User` stream leaves this `None`, and the walk keeps the
    // stream order it always used. See [`live_slide_offsets`] for why a partial answer is
    // deliberately not produced.
    let current_user_stream = read_stream(&mut comp, "/Current User").unwrap_or_default();
    let live_slides = live_slide_offsets(&ppt_stream, &current_user_stream);
    if live_slides.is_none() {
        tracing::debug!(
            target: "xberg::ppt",
            "no live slide list resolved from the persist chain; using stream order"
        );
    }

    let (mut slides, loose_texts, speaker_notes) = extract_texts_from_records(
        &ppt_stream,
        include_master_slides,
        live_slides.as_ref(),
        &mut processing_warnings,
    )?;

    let text = assemble_full_text(&slides, &loose_texts);

    // Defensive fallback for a stream with no `RT_SLIDE` containers at all
    // but with top-level text outside any slide/notes container: surface it
    // as a single synthetic slide rather than dropping it (matches the
    if slides.is_empty() && !loose_texts.is_empty() {
        slides.push(PptSlideText {
            number: 1,
            text: loose_texts.join("\n"),
            title: None,
            notes: None,
            tables: Vec::new(),
        });
    }
    let slide_count = slides.len();

    let images = extract_images_if_requested(
        &mut comp,
        &ppt_stream,
        live_slides.as_ref(),
        extract_images,
        &mut processing_warnings,
    );

    Ok(PptExtractionResult {
        text: text.trim().to_string(),
        slides,
        slide_count,
        metadata,
        speaker_notes,
        images,
        processing_warnings,
    })
}

/// Read a little-endian `u32` at `offset`, or `None` if it does not fit in `data`.
pub(super) fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// The deck's live slides, resolved through the persist chain (#1614, #1639).
pub(super) struct LiveSlides {
    /// Live `Slide` container offsets, presentation order (#1614).
    pub(super) offsets: Vec<usize>,
    /// Absolute stream offset of the live `SlideListWithText` record header itself -- the
    /// only occurrence [`extract_texts_from_records`] may harvest outline text (titles
    /// included) from. An older save's list, or the master/notes lists sharing the same
    /// record type, must not be read as the presentation's outline (#1639). ~keep
    outline_record_offset: usize,
    /// `SlidePersistAtom.slideId` for each live slide, aligned by index with `offsets` --
    /// what `NotesAtom.slideIdRef` names a notes page's slide by (#1640).
    slide_ids: Vec<u32>,
    /// The merged persist directory (persist id -> stream offset, newest save wins), kept
    /// so other persist-referenced records -- an embedded object's `ExOleObjStg`, named by
    /// `ExOleObjAtom.persistIdRef` (GH#1660) -- resolve through the same chain. ~keep
    pub(super) persist: ahash::AHashMap<u32, usize>,
    /// Payload range `[start, end)` of the live `DocumentContainer` in the stream, so a
    /// document-level list (`ExObjList`) is read from the current save, not an older one. ~keep
    pub(super) document_payload: (usize, usize),
}

/// Resolve the live slide containers, in presentation order, as byte offsets into the
/// "PowerPoint Document" stream.
///
/// A `.ppt` stream is append-only across saves: editing a deck writes new copies of the
/// objects that changed and leaves the old ones in place. Walking the stream and calling
/// every `RT_SLIDE` container a slide therefore counts deleted and superseded revisions
/// as slides, and numbers them by byte order rather than by the order they are presented
/// in. The format states both answers explicitly and this reads them:
///
/// ```text
/// Current User stream
///   CurrentUserAtom.offsetToCurrentEdit ─▶ UserEditAtom (0x0FF5)
///                                            .offsetPersistDirectory ─▶ PersistDirectoryAtom (0x1772)
///                                            .offsetLastEdit         ─▶ the previous save's UserEditAtom
///                                            .docPersistIdRef        ─▶ the live DocumentContainer
///
/// DocumentContainer (0x03E8) > SlideListWithText (0x0FF0, recInstance 0)
///   SlidePersistAtom (0x03F3) per slide, in presentation order,
///   each naming the persist id of that slide's Slide container
/// ```
///
/// Persist directories are merged newest-first, so a later save's entry for an id wins
/// and older revisions of the same object are never reachable. The slide list itself is
/// read from the **live** `DocumentContainer` -- resolved the same way, through
/// `docPersistIdRef` on the newest `UserEditAtom` -- not from the first `SlideListWithText`
/// the stream happens to contain, which is the oldest save's (#1639).
///
/// Returns `None` whenever the chain cannot be read in full -- absent `Current User`
/// stream, unparseable atom, offset out of range, an unresolvable `DocumentContainer`, or a
/// slide list naming an id no directory resolves. The caller then keeps stream order, which
/// is what this extractor did unconditionally before. A partial answer would be worse than
/// the old behaviour: it would drop real slides. See GH#1614. ~keep
pub(super) fn live_slide_offsets(ppt_stream: &[u8], current_user_stream: &[u8]) -> Option<LiveSlides> {
    let first_edit = read_u32_le(current_user_stream, CURRENT_USER_OFFSET_TO_CURRENT_EDIT)? as usize;

    // persist id -> stream offset, newest save wins.
    let mut persist: ahash::AHashMap<u32, usize> = ahash::AHashMap::new();
    let mut document_id: Option<u32> = None;
    let mut next_edit = Some(first_edit);
    let mut hops = 0usize;

    while let Some(edit_offset) = next_edit {
        hops += 1;
        if hops > MAX_USER_EDIT_CHAIN {
            return None;
        }
        let header = ppt_stream.get(edit_offset..edit_offset.checked_add(8)?)?;
        if u16::from_le_bytes([header[2], header[3]]) != RT_USER_EDIT_ATOM {
            return None;
        }
        let content = edit_offset + 8;
        let offset_last_edit = read_u32_le(ppt_stream, content + 8)? as usize;
        let offset_persist_directory = read_u32_le(ppt_stream, content + 12)? as usize;
        // `docPersistIdRef`: only the FIRST (newest) edit's value matters -- it names the
        // live `DocumentContainer`, which carries the presentation's own slide list rather
        // than an earlier save's (#1639). ~keep
        document_id.get_or_insert(read_u32_le(ppt_stream, content + 16)?);

        merge_persist_directory(ppt_stream, offset_persist_directory, &mut persist)?;

        // `offsetLastEdit` is 0 at the first save. Anything that does not move BACKWARD is
        // a cycle or corruption, and following it would not terminate. ~keep
        next_edit = match offset_last_edit {
            0 => None,
            previous if previous < edit_offset => Some(previous),
            _ => return None,
        };
    }

    let document_offset = *persist.get(&document_id?)?;
    let document_header = ppt_stream.get(document_offset..document_offset.checked_add(8)?)?;
    if u16::from_le_bytes([document_header[2], document_header[3]]) != RT_DOCUMENT {
        return None;
    }
    let document_len = u32::from_le_bytes([
        document_header[4],
        document_header[5],
        document_header[6],
        document_header[7],
    ]) as usize;
    let document_content_start = document_offset.checked_add(8)?;
    let document_content_end = document_content_start.checked_add(document_len)?;
    let document_payload = ppt_stream.get(document_content_start..document_content_end)?;

    let (relative_outline_offset, entries) = slide_persist_ids_in_presentation_order(document_payload, 0)?;
    if entries.is_empty() {
        return None;
    }
    let outline_record_offset = document_content_start + relative_outline_offset;

    let offsets = entries
        .iter()
        .map(|&(persist_id, _)| {
            let offset = *persist.get(&persist_id)?;
            let header = ppt_stream.get(offset..offset.checked_add(8)?)?;
            (u16::from_le_bytes([header[2], header[3]]) == RT_SLIDE).then_some(offset)
        })
        .collect::<Option<Vec<usize>>>()?;
    let slide_ids = entries.iter().map(|&(_, slide_id)| slide_id).collect();

    Some(LiveSlides {
        offsets,
        outline_record_offset,
        slide_ids,
        persist,
        document_payload: (document_content_start, document_content_end),
    })
}

/// Merge one save's `PersistDirectoryAtom` into `persist` without overwriting an entry a
/// newer save already claimed. Returns `None` if the atom is not where the `UserEditAtom`
/// says it is, or is internally inconsistent.
fn merge_persist_directory(
    ppt_stream: &[u8],
    directory_offset: usize,
    persist: &mut ahash::AHashMap<u32, usize>,
) -> Option<()> {
    let header = ppt_stream.get(directory_offset..directory_offset.checked_add(8)?)?;
    if u16::from_le_bytes([header[2], header[3]]) != RT_PERSIST_DIRECTORY_ATOM {
        return None;
    }
    let length = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
    let start = directory_offset + 8;
    let entries = ppt_stream.get(start..start.checked_add(length)?)?;

    let mut cursor = 0usize;
    while cursor + 4 <= entries.len() {
        let packed = read_u32_le(entries, cursor)?;
        let first_id = packed & PERSIST_ID_MASK;
        let count = (packed >> PERSIST_COUNT_SHIFT) as usize;
        cursor += 4;
        for index in 0..count {
            let offset = read_u32_le(entries, cursor)? as usize;
            cursor += 4;
            // `or_insert` and not `insert`: directories arrive newest-first, so the first
            // writer of an id is the live revision and later (older) saves must not
            // clobber it. ~keep
            persist.entry(first_id + index as u32).or_insert(offset);
        }
    }
    Some(())
}

/// The persist ids named by `container`'s own `SlideListWithText` (`recInstance` 0), in the
/// order the deck presents them, plus that record's own offset relative to `container`.
///
/// `container` is the **live** `DocumentContainer`'s payload, resolved by the caller
/// through the persist chain (#1639) -- so an older save's slide list elsewhere in the
/// stream is never seen. `recInstance` distinguishes the three lists a document can carry
/// -- 0 slides, 1 master, 2 notes -- and only the first is the presentation's slide order.
/// It is the top 12 bits of the record header's first `u16`. ~keep
///
/// A proper depth-first descent (not a linear walk that never returns to a sibling): a
/// `SlideListWithText` not found in one child container is looked for in the next.
///
/// Each entry is `(persistIdRef, slideId)`: `persistIdRef` resolves the `Slide` container's
/// stream offset through the persist directory; `slideId` is what `NotesAtom.slideIdRef`
/// names the slide by, and is a different value (xberg-io/xberg#1640).
fn slide_persist_ids_in_presentation_order(container: &[u8], depth: usize) -> Option<(usize, Vec<(u32, u32)>)> {
    if depth > MAX_SLIDE_LIST_SEARCH_DEPTH {
        return None;
    }
    let mut pos = 0usize;
    while pos + 8 <= container.len() {
        let ver_instance = u16::from_le_bytes([container[pos], container[pos + 1]]);
        let rec_type = u16::from_le_bytes([container[pos + 2], container[pos + 3]]);
        let rec_len = u32::from_le_bytes([
            container[pos + 4],
            container[pos + 5],
            container[pos + 6],
            container[pos + 7],
        ]) as usize;
        let content_start = pos + 8;
        let content_end = content_start.checked_add(rec_len)?;
        if content_end > container.len() {
            return None;
        }

        if rec_type == RT_SLIDE_LIST_WITH_TEXT && (ver_instance >> 4) == 0 {
            let mut entries = Vec::new();
            let mut inner = content_start;
            while inner + 8 <= content_end {
                let inner_type = u16::from_le_bytes([container[inner + 2], container[inner + 3]]);
                let inner_len = u32::from_le_bytes([
                    container[inner + 4],
                    container[inner + 5],
                    container[inner + 6],
                    container[inner + 7],
                ]) as usize;
                if inner_type == RT_SLIDE_PERSIST_ATOM {
                    let persist_id = read_u32_le(container, inner + 8)?;
                    let slide_id = read_u32_le(container, inner + 8 + SLIDE_PERSIST_ATOM_SLIDE_ID_OFFSET)?;
                    entries.push((persist_id, slide_id));
                }
                inner = inner.checked_add(8)?.checked_add(inner_len)?;
            }
            return Some((pos, entries));
        }

        if (ver_instance & 0x000F) == 0x0F
            && let Some((relative, entries)) =
                slide_persist_ids_in_presentation_order(&container[content_start..content_end], depth + 1)
        {
            // `relative` is relative to the nested slice; translate it back into
            // `container`'s own coordinate space so the caller's offset stays meaningful.
            return Some((content_start + relative, entries));
        }

        pos = content_end;
    }
    None
}

/// Parse PowerPoint record headers and extract text atoms.
///
/// Returns `(slides, loose_texts, speaker_notes)`, where `slides` carries
/// one entry per `RT_SLIDE` container in persist order (including empty
/// slides), and `loose_texts` carries text found outside any slide/notes
/// container (rare, but preserved for the `slide_count == 0` fallback).
///
/// When `include_master_slides` is `true`, master slide containers are not
/// skipped, allowing their placeholder text to appear in the output.
/// Push one outline atom's cleaned text into `outline_texts[index]`, and -- when
/// `text_type` names the slide's title (`Title` / `CenterTitle`) and no title has been
/// captured for this slide yet -- also into `outline_titles[index]` (#1635). The first
/// title-typed atom wins: an outline states a slide's title once.
fn record_outline_text(
    outline_texts: &mut [Vec<String>],
    outline_titles: &mut [Option<String>],
    index: usize,
    text_type: Option<u32>,
    cleaned: String,
) {
    if matches!(text_type, Some(TEXT_TYPE_TITLE | TEXT_TYPE_CENTER_TITLE)) && outline_titles[index].is_none() {
        outline_titles[index] = Some(cleaned.clone());
    }
    outline_texts[index].push(cleaned);
}

/// Mutable state [`dispatch_cleaned_text`] routes one decoded text atom into,
/// threaded through as one bundle so the function stays under the
/// parameter-count limit; this type has no callers outside this module. ~keep
struct TextDispatchState<'a> {
    outline_texts: &'a mut Vec<Vec<String>>,
    outline_titles: &'a mut Vec<Option<String>>,
    outline_slide_index: Option<usize>,
    outline_text_type: Option<u32>,
    in_notes: bool,
    current_notes_texts: &'a mut Vec<String>,
    in_slide_text: bool,
    current_slide_texts: &'a mut Vec<String>,
    loose_texts: &'a mut Vec<String>,
}

/// Route one decoded, cleaned text atom (from either `RT_TEXT_CHARS_ATOM` or
/// `RT_TEXT_BYTES_ATOM` -- both dispatch identically once decoded) to the
/// outline collection, notes, current slide, or loose text, exactly as
/// `extract_texts_from_records` did inline before this was split out.
/// A no-op for an empty `cleaned` string.
fn dispatch_cleaned_text(cleaned: String, state: TextDispatchState) {
    if cleaned.is_empty() {
        return;
    }
    if let Some(index) = state.outline_slide_index {
        // Outline text belongs to a slide, not to `loose_texts` -- which is
        // discarded whenever any slide exists, and is where every legacy
        // title used to end up (#1612).
        record_outline_text(
            state.outline_texts,
            state.outline_titles,
            index,
            state.outline_text_type,
            cleaned,
        );
    } else {
        if state.in_notes {
            state.current_notes_texts.push(cleaned.clone());
        }
        if state.in_slide_text {
            state.current_slide_texts.push(cleaned);
        } else if !state.in_notes {
            state.loose_texts.push(cleaned);
        }
    }
}

/// Commit one just-closed `RT_NOTES` container's accumulated text into `notes_by_slide`,
/// keyed by the slide it belongs to (xberg-io/xberg#1640).
///
/// The notes master (`slideIdRef == NOTES_MASTER_SLIDE_ID_REF`) is layout, not content, and
/// is dropped -- the same reason `RT_MAIN_MASTER` is skipped by default. With a resolved
/// persist chain, `current_notes_slide_id` is looked up in `slide_number_by_id`; a notes
/// page whose id names no live slide is a stale revision, like a `Slide` container the live
/// list does not name (#1614), and is dropped too. Without a resolved chain, notes fall back
/// to the deck's own encounter order via `next_positional_slide_number` -- the old, purely
/// positional behaviour, now at least skipping the master.
fn commit_notes(
    current_notes_texts: &mut Vec<String>,
    current_notes_slide_id: Option<u32>,
    slide_number_by_id: &ahash::AHashMap<u32, u32>,
    next_positional_slide_number: &mut u32,
    notes_by_slide: &mut std::collections::BTreeMap<u32, String>,
) {
    if current_notes_texts.is_empty() {
        return;
    }
    let notes_text = current_notes_texts.join("\n");
    current_notes_texts.clear();
    let trimmed = notes_text.trim();
    if trimmed.is_empty() || current_notes_slide_id == Some(NOTES_MASTER_SLIDE_ID_REF) {
        return;
    }
    let slide_number = if slide_number_by_id.is_empty() {
        let number = *next_positional_slide_number;
        *next_positional_slide_number += 1;
        Some(number)
    } else {
        current_notes_slide_id.and_then(|id| slide_number_by_id.get(&id).copied())
    };
    if let Some(number) = slide_number {
        notes_by_slide.insert(number, trimmed.to_string());
    }
}

fn extract_texts_from_records(
    data: &[u8],
    include_master_slides: bool,
    live: Option<&LiveSlides>,
    warnings: &mut Vec<ProcessingWarning>,
) -> Result<(Vec<PptSlideText>, Vec<String>, Vec<String>)> {
    let live_slides = live.map(|l| l.offsets.as_slice());
    // The only `SlideListWithText` occurrence outline text may be harvested from; `None`
    // means the persist chain did not resolve, in which case every occurrence is read, as
    // this extractor always did before #1639.
    let live_outline_offset = live.map(|l| l.outline_record_offset);
    let mut slides: Vec<PptSlideText> = Vec::new();
    let mut loose_texts = Vec::new();
    let mut current_slide_number: u32 = 0;
    let mut pos = 0;
    let mut in_slide_text = false;
    let mut slide_end: Option<usize> = None;
    let mut current_slide_texts: Vec<String> = Vec::new();
    let mut current_slide_tables: Vec<Vec<Vec<String>>> = Vec::new();
    let mut in_notes = false;
    let mut notes_end: Option<usize> = None;
    let mut current_notes_texts: Vec<String> = Vec::new();
    // `NotesAtom.slideIdRef` of the notes container currently open; `None` until its
    // `NotesAtom` (0x03F1) is read (#1640).
    let mut current_notes_slide_id: Option<u32> = None;
    // Speaker notes, keyed by the slide number they belong to -- resolved from
    // `slideIdRef` when a persist chain is available, positional (skipping the master)
    // otherwise (xberg-io/xberg#1640).
    let mut notes_by_slide: std::collections::BTreeMap<u32, String> = std::collections::BTreeMap::new();
    let mut next_positional_slide_number: u32 = 1;
    let slide_number_by_id: ahash::AHashMap<u32, u32> = live
        .map(|l| {
            l.slide_ids
                .iter()
                .enumerate()
                .map(|(i, &id)| (id, i as u32 + 1))
                .collect()
        })
        .unwrap_or_default();
    // Outline text keyed by slide position, harvested from `SlideListWithText` (#1612).
    let mut outline_texts: Vec<Vec<String>> = Vec::new();
    let mut outline_end: Option<usize> = None;
    let mut outline_slide_index: Option<usize> = None;
    // The outline's own title, one atom per slide, typed by the `TextHeaderAtom` that
    // precedes it (#1635) -- `None` where the file's outline states no title.
    let mut outline_titles: Vec<Option<String>> = Vec::new();
    // `TextHeaderAtom.textType` of the outline atom about to follow; reset after every text
    // atom so an untyped run never inherits a stale type from an earlier one. ~keep
    let mut outline_text_type: Option<u32> = None;

    while pos + 8 <= data.len() {
        // A slide/notes container's text only spans its own declared byte
        // range; close it out as soon as the walk passes that range's end,
        // rather than leaving `in_slide_text`/`in_notes` stuck on until the
        // *next* occurrence (which, for `in_slide_text`, previously ran on
        // to the end of the stream -- see `RT_SLIDE` below).
        if let Some(end) = slide_end
            && pos >= end
        {
            // Push even when empty: a slide with no text atoms still exists
            // and must keep its persist-order number (#1418), rather than
            // vanishing and shifting every later slide's number down.
            slides.push(PptSlideText {
                number: current_slide_number,
                text: current_slide_texts.join("\n"),
                title: None,
                notes: None,
                tables: std::mem::take(&mut current_slide_tables),
            });
            current_slide_texts.clear();
            in_slide_text = false;
            slide_end = None;
        }
        if let Some(end) = outline_end
            && pos >= end
        {
            outline_end = None;
            outline_slide_index = None;
        }
        if let Some(end) = notes_end
            && pos >= end
        {
            commit_notes(
                &mut current_notes_texts,
                current_notes_slide_id,
                &slide_number_by_id,
                &mut next_positional_slide_number,
                &mut notes_by_slide,
            );
            current_notes_slide_id = None;
            in_notes = false;
            notes_end = None;
        }

        let rec_ver_instance = u16::from_le_bytes([data[pos], data[pos + 1]]);
        let rec_ver = rec_ver_instance & 0x000F;
        let rec_type = u16::from_le_bytes([data[pos + 2], data[pos + 3]]);
        let rec_len = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]) as usize;

        if rec_len > data.len() - pos {
            crate::core::diagnostics::push_warning(
                warnings,
                PPT_WARNING_SOURCE,
                "Record stream ended with a truncated record; the remaining presentation content was not extracted",
            );
            break;
        }

        let is_container = rec_ver == 0x0F;
        let content_start = pos + 8;
        let content_end = content_start + rec_len;

        match rec_type {
            RT_SLIDE => {
                // With a resolved persist chain, only the containers the live slide list
                // names are slides, and their number is their position in that list. A
                // container the list does not name is a deleted or superseded revision the
                // stream still carries, and must not be extracted or counted (#1614).
                let presentation_number = match live_slides {
                    Some(offsets) => match offsets.iter().position(|&offset| offset == pos) {
                        Some(index) => index as u32 + 1,
                        None => {
                            pos = content_end;
                            continue;
                        }
                    },
                    None => current_slide_number + 1,
                };
                if in_slide_text {
                    slides.push(PptSlideText {
                        number: current_slide_number,
                        text: current_slide_texts.join("\n"),
                        title: None,
                        notes: None,
                        tables: std::mem::take(&mut current_slide_tables),
                    });
                    current_slide_texts.clear();
                }
                current_slide_number = presentation_number;
                in_slide_text = true;
                slide_end = Some(content_end);
                pos += 8;
                continue;
            }
            RT_NOTES => {
                if in_notes {
                    commit_notes(
                        &mut current_notes_texts,
                        current_notes_slide_id,
                        &slide_number_by_id,
                        &mut next_positional_slide_number,
                        &mut notes_by_slide,
                    );
                }
                in_notes = true;
                current_notes_slide_id = None;
                notes_end = Some(content_end);
                pos += 8;
                continue;
            }
            RT_NOTES_ATOM => {
                if in_notes && let Some(bytes) = data.get(content_start..content_start + 4) {
                    current_notes_slide_id = Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
                }
                pos = content_end;
                continue;
            }
            RT_MAIN_MASTER if !include_master_slides => {
                pos = content_end;
                continue;
            }
            RT_SLIDE_LIST_WITH_TEXT => {
                // Only the live document's own list (#1639); a stale save's, or the
                // master/notes lists sharing this record type, are skipped.
                if live_outline_offset.is_none_or(|offset| offset == pos) {
                    outline_end = Some(content_end);
                    outline_slide_index = None;
                }
                pos += 8;
                continue;
            }
            RT_SLIDE_PERSIST_ATOM if outline_end.is_some() => {
                let next = outline_slide_index.map_or(0, |i| i + 1);
                outline_slide_index = Some(next);
                if outline_texts.len() <= next {
                    outline_texts.resize(next + 1, Vec::new());
                    outline_titles.resize(next + 1, None);
                }
                outline_text_type = None;
                pos = content_end;
                continue;
            }
            RT_TEXT_HEADER_ATOM => {
                if outline_slide_index.is_some()
                    && let Some(bytes) = data.get(content_start..content_start + 4)
                {
                    outline_text_type = Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
                }
                pos = content_end;
                continue;
            }
            RT_TEXT_CHARS_ATOM => {
                if content_end <= data.len() {
                    let text_data = &data[content_start..content_end];
                    let chars: Vec<u16> = text_data
                        .chunks_exact(2)
                        .map(|c| u16::from_le_bytes([c[0], c[1]]))
                        .collect();
                    let text = String::from_utf16_lossy(&chars);
                    let cleaned = clean_ppt_text(&text);
                    dispatch_cleaned_text(
                        cleaned,
                        TextDispatchState {
                            outline_texts: &mut outline_texts,
                            outline_titles: &mut outline_titles,
                            outline_slide_index,
                            outline_text_type,
                            in_notes,
                            current_notes_texts: &mut current_notes_texts,
                            in_slide_text,
                            current_slide_texts: &mut current_slide_texts,
                            loose_texts: &mut loose_texts,
                        },
                    );
                    outline_text_type = None;
                }
                pos = content_end;
                continue;
            }
            RT_TEXT_BYTES_ATOM => {
                if content_end <= data.len() {
                    let text_data = &data[content_start..content_end];
                    let text: String = text_data.iter().map(|&b| cp1252_to_char(b)).collect();
                    let cleaned = clean_ppt_text(&text);
                    dispatch_cleaned_text(
                        cleaned,
                        TextDispatchState {
                            outline_texts: &mut outline_texts,
                            outline_titles: &mut outline_titles,
                            outline_slide_index,
                            outline_text_type,
                            in_notes,
                            current_notes_texts: &mut current_notes_texts,
                            in_slide_text,
                            current_slide_texts: &mut current_slide_texts,
                            loose_texts: &mut loose_texts,
                        },
                    );
                    outline_text_type = None;
                }
                pos = content_end;
                continue;
            }
            // A table is a group shape; its cells are read as a grid rather than as loose
            // text, and the walk skips them. A group that is not a table is walked as before.
            MSOFBT_SPGR_CONTAINER if is_container && in_slide_text && !in_notes => {
                if let Some(table) = read_table(data, content_start, content_end) {
                    if !table.is_empty() {
                        current_slide_tables.push(table);
                    }
                    pos = content_end;
                    continue;
                }
            }
            _ => {}
        }

        if is_container {
            pos += 8;
        } else {
            pos = content_end;
        }
    }

    // The stream ended while still inside a slide's declared byte range
    // (e.g. a truncated record broke the walk early): still record it,
    // rather than silently dropping the last slide's text and number.
    if in_slide_text {
        slides.push(PptSlideText {
            number: current_slide_number,
            text: current_slide_texts.join("\n"),
            title: None,
            notes: None,
            tables: std::mem::take(&mut current_slide_tables),
        });
    }

    commit_notes(
        &mut current_notes_texts,
        current_notes_slide_id,
        &slide_number_by_id,
        &mut next_positional_slide_number,
        &mut notes_by_slide,
    );

    // Presentation order, not stream order: with a live slide list the walk visits the
    // containers in whatever order the saves left them, and the number assigned above is
    // the authority. Sorting here also restores the positional contract the outline merge
    // below depends on -- the nth entry of `slides` is the nth slide of the deck (#1614).
    if live_slides.is_some() {
        slides.sort_by_key(|slide| slide.number);
    }

    // Prepend each slide's outline text, skipping any line the slide's own drawing already
    // carries. A title drawn on the canvas appears in both places, and those decks are exactly
    // the ones whose titles already survived -- recovering the outline copy must not double
    // them (#1612). Attribution is positional: the nth `SlidePersistAtom` introduces the nth
    // slide's outline records, matching the persist order `slides` is built in.
    for (index, outline) in outline_texts.iter().enumerate() {
        let Some(slide) = slides.get_mut(index) else {
            continue;
        };
        // The file's own outline title (#1635), not the first-line guess the caller falls
        // back to when this is `None`.
        slide.title = outline_titles.get(index).cloned().flatten();
        let recovered: Vec<&str> = outline
            .iter()
            .map(String::as_str)
            .filter(|line| !slide.text.lines().any(|existing| existing == *line))
            .collect();
        if recovered.is_empty() {
            continue;
        }
        let joined = recovered.join("\n");
        slide.text = if slide.text.is_empty() {
            joined
        } else {
            format!("{joined}\n{}", slide.text)
        };
    }

    // Every non-empty flat entry, in slide-number order -- kept for the metadata array
    // consumers already read (`PptExtractionResult::speaker_notes`); the per-slide
    // attachment below is what fixes the misattribution (#1640).
    let speaker_notes: Vec<String> = notes_by_slide.values().cloned().collect();
    for slide in &mut slides {
        slide.notes = notes_by_slide.remove(&slide.number);
    }

    Ok((slides, loose_texts, speaker_notes))
}

/// One record header from the PowerPoint/OfficeArt record tree, flattened out of its
/// containers so callers can filter by type without re-walking.
pub(super) struct ArtRecord {
    pub(super) rec_instance: u16,
    pub(super) rec_type: u16,
    pub(super) content_start: usize,
    pub(super) content_end: usize,
}

/// Collect every record header in `data[start..end]`, descending into containers.
///
/// A record is a container when `recVer` (the low nibble of the first field) is 0xF; its
/// children are the records inside its own declared byte range. A `recLen` that would run
/// past `end` stops the walk at that level rather than over-reading -- the stream is
/// untrusted, and a truncated tail is the common shape of a salvageable corrupt deck.
pub(super) fn collect_art_records(data: &[u8], start: usize, end: usize, depth: usize, out: &mut Vec<ArtRecord>) {
    if depth > MAX_OFFICE_ART_DEPTH {
        return;
    }
    let mut pos = start;
    while pos + 8 <= end {
        let rec_ver_instance = u16::from_le_bytes([data[pos], data[pos + 1]]);
        let rec_type = u16::from_le_bytes([data[pos + 2], data[pos + 3]]);
        let rec_len = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]) as usize;
        let content_start = pos + 8;
        let Some(content_end) = content_start.checked_add(rec_len).filter(|&candidate| candidate <= end) else {
            return;
        };
        out.push(ArtRecord {
            rec_instance: rec_ver_instance >> 4,
            rec_type,
            content_start,
            content_end,
        });
        if rec_ver_instance & 0x000F == 0x0F {
            collect_art_records(data, content_start, content_end, depth + 1, out);
        }
        pos = content_end;
    }
}

/// The `Pictures`-stream offset of every blip, in `BStoreContainer` order, so that a
/// shape's 1-based `pib` indexes straight into the result.
fn blip_stream_offsets(data: &[u8], records: &[ArtRecord]) -> Vec<u32> {
    records
        .iter()
        .filter(|record| record.rec_type == MSOFBT_BSE)
        .filter_map(|record| {
            let at = record.content_start.checked_add(BSE_FO_DELAY_OFFSET)?;
            (at.checked_add(4)? <= record.content_end).then(|| read_u32_le(data, at))?
        })
        .collect()
}

/// The blip indices (`pib`) referenced by shapes inside one slide's byte range.
///
/// Reading `pib` wherever it appears in the slide's property tables -- rather than only
/// under a shape whose `msofbtSp` says `msosptPictureFrame` -- is deliberate: a picture
/// placed as the fill of an ordinary autoshape carries the same property and displays the
/// same blip on the same slide. ~keep
fn slide_blip_indices(data: &[u8], records: &[ArtRecord], slide_start: usize, slide_end: usize) -> Vec<u32> {
    let mut indices = Vec::new();
    for record in records
        .iter()
        .filter(|record| MSOFBT_OPT.contains(&record.rec_type))
        .filter(|record| record.content_start >= slide_start && record.content_end <= slide_end)
    {
        for entry in 0..record.rec_instance as usize {
            let Some(at) = entry
                .checked_mul(FOPT_ENTRY_LEN)
                .and_then(|o| record.content_start.checked_add(o))
            else {
                break;
            };
            if at + FOPT_ENTRY_LEN > record.content_end {
                break;
            }
            let opid = u16::from_le_bytes([data[at], data[at + 1]]);
            if opid & FOPT_PROPERTY_ID_MASK == MSO_PROPERTY_PIB
                && let Some(pib) = read_u32_le(data, at + 2)
                && pib > 0
            {
                indices.push(pib);
            }
        }
    }
    indices
}

/// Map each blip's `Pictures`-stream offset to the slide that displays it (#1620).
///
/// `Pictures` is a blob store in save order and says nothing about slides; what owns a
/// picture is the drawing, via `pib` -> `msofbtBSE.foDelay` -> that offset. Without this
/// every `.ppt` image came back with no page at all.
///
/// A blip displayed on several slides is recorded against the first, so that a logo
/// repeated through a deck stays one extracted image rather than one per slide. A blip no
/// live shape references is absent from the map and keeps today's behaviour. ~keep
fn picture_slide_numbers(data: &[u8], live_slides: Option<&[usize]>) -> std::collections::HashMap<u32, u32> {
    let mut records = Vec::new();
    collect_art_records(data, 0, data.len(), 0, &mut records);

    let blip_offsets = blip_stream_offsets(data, &records);
    let mut by_offset = std::collections::HashMap::new();
    if blip_offsets.is_empty() {
        return by_offset;
    }

    let slide_ranges: Vec<(usize, usize)> = match live_slides {
        // Presentation order, already resolved from the persist chain (#1614).
        Some(offsets) => offsets
            .iter()
            .filter_map(|&offset| {
                records
                    .iter()
                    .find(|record| record.rec_type == RT_SLIDE && record.content_start == offset + 8)
                    .map(|record| (record.content_start, record.content_end))
            })
            .collect(),
        // Mirrors the text walk's fallback: number the `RT_SLIDE` containers in stream order.
        None => records
            .iter()
            .filter(|record| record.rec_type == RT_SLIDE)
            .map(|record| (record.content_start, record.content_end))
            .collect(),
    };

    for (index, (slide_start, slide_end)) in slide_ranges.iter().enumerate() {
        let slide_number = index as u32 + 1;
        for pib in slide_blip_indices(data, &records, *slide_start, *slide_end) {
            if let Some(&offset) = blip_offsets.get(pib as usize - 1) {
                by_offset.entry(offset).or_insert(slide_number);
            }
        }
    }
    by_offset
}

/// Walk a `Pictures` stream (a flat run of `OfficeArtBlip` records, MS-ODRAW
/// 2.2.23) and emit one `ExtractedImage` per raster blip.
///
/// Only the raster formats stored as `rgbUid` + optional second `rgbUid` +
/// `tag` + `BLIPFileData` are handled: JPEG (0xF01D / the alternate 0xF02A),
/// PNG (0xF01E), and DIB (0xF01F). Metafile blips (EMF/WMF/PICT) use a
/// different, larger header and are not raster images; they are skipped.
///
/// Every length is validated against the remaining buffer before slicing,
/// so a hostile `recLen` can only shrink the walk (skip a record or stop
/// early), never over-read or allocate unboundedly.
/// A blip record's position and header fields, as read from the `Pictures`
/// stream before its picture bytes are decoded. Grouped so
/// [`extract_blip_image`] stays under the parameter-count limit; this type
/// has no callers outside this module. ~keep
struct BlipRecordHeader {
    pos: usize,
    rec_instance: u16,
    rec_len: usize,
    content_start: usize,
    content_end: usize,
}

/// Validate a blip record's declared length against its UID header and the
/// size cap, returning the picture payload length, or `None` if the record
/// should be skipped: too short or oversized (both push a `ProcessingWarning`)
/// or empty (skipped silently), matching the original behaviour.
fn validate_blip_picture_len(
    pos: usize,
    rec_len: usize,
    header_len: usize,
    warnings: &mut Vec<ProcessingWarning>,
) -> Option<usize> {
    if rec_len < header_len {
        crate::core::diagnostics::push_warning(
            warnings,
            PPT_WARNING_SOURCE,
            format!(
                "Blip record at offset {pos} (recLen={rec_len}) is shorter than its UID header \
                 ({header_len} bytes); skipped"
            ),
        );
        return None;
    }

    let picture_len = rec_len - header_len;
    if picture_len == 0 {
        return None;
    }

    if picture_len > MAX_PICTURE_SIZE {
        crate::core::diagnostics::push_warning(
            warnings,
            PPT_WARNING_SOURCE,
            format!(
                "Embedded picture at offset {pos} ({picture_len} bytes) exceeds the \
                 {MAX_PICTURE_SIZE}-byte size cap and was skipped"
            ),
        );
        return None;
    }

    Some(picture_len)
}

/// Decode one blip record's picture bytes into an `ExtractedImage`, or return
/// `None` if the record is malformed/oversized/empty (see
/// [`validate_blip_picture_len`]).
fn extract_blip_image(
    data: &[u8],
    record: BlipRecordHeader,
    format: Cow<'static, str>,
    slide_numbers: &std::collections::HashMap<u32, u32>,
    image_index: u32,
    warnings: &mut Vec<ProcessingWarning>,
) -> Option<ExtractedImage> {
    let BlipRecordHeader {
        pos,
        rec_instance,
        rec_len,
        content_start,
        content_end,
    } = record;

    // One `rgbUid` (16 bytes) + `tag` (1 byte) = 17-byte header, or
    // two `rgbUid`s + `tag` = 33 bytes; per MS-ODRAW 2.2.27-2.2.29
    // the low bit of `recInstance` is what distinguishes the two
    // UID counts for every raster blip type (e.g. JPEG 0x46A vs
    // 0x46B, PNG 0x6E0 vs 0x6E1, DIB 0x7A8 vs 0x7A9).
    let header_len = if rec_instance & 0x1 == 1 { 33 } else { 17 };
    validate_blip_picture_len(pos, rec_len, header_len, warnings)?;

    let picture_start = content_start + header_len;
    let picture_bytes = &data[picture_start..content_end];

    Some(ExtractedImage {
        data: Bytes::copy_from_slice(picture_bytes),
        format,
        image_index,
        // `foDelay` in the BSE names this record's own start offset, so `pos` is the
        // key the drawing refers to it by (#1620). ~keep
        page_number: u32::try_from(pos)
            .ok()
            .and_then(|offset| slide_numbers.get(&offset))
            .copied(),
        width: None,
        height: None,
        colorspace: None,
        bits_per_component: None,
        is_mask: false,
        description: None,
        ocr_result: None,
        bounding_box: None,
        source_path: None,
        image_kind: None,
        kind_confidence: None,
        cluster_id: None,
        caption: None,
        qr_codes: None,
        data_base64: None,
    })
}

fn extract_pictures_from_stream(
    data: &[u8],
    slide_numbers: &std::collections::HashMap<u32, u32>,
    warnings: &mut Vec<ProcessingWarning>,
) -> Vec<ExtractedImage> {
    let mut images = Vec::new();
    let mut pos = 0usize;
    let mut image_index: u32 = 0;

    while pos + 8 <= data.len() {
        let rec_ver_instance = u16::from_le_bytes([data[pos], data[pos + 1]]);
        // recInstance occupies the upper 12 bits of the packed 16-bit field
        // (recVer, the low 4 bits, is checked nowhere here -- MS-ODRAW
        // requires it to be 0 for blips, but a non-zero value doesn't change
        // where the UID/tag/data fields are).
        let rec_instance = rec_ver_instance >> 4;
        let rec_type = u16::from_le_bytes([data[pos + 2], data[pos + 3]]);
        let rec_len = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]) as usize;

        let remaining = data.len() - (pos + 8);
        if rec_len > remaining {
            crate::core::diagnostics::push_warning(
                warnings,
                PPT_WARNING_SOURCE,
                "Pictures stream ended with a truncated record; the remaining embedded images were not extracted",
            );
            break;
        }

        let content_start = pos + 8;
        let content_end = content_start + rec_len;

        let format: Option<Cow<'static, str>> = match rec_type {
            RT_BLIP_JPEG | RT_BLIP_JPEG_ALT => Some(Cow::Borrowed("jpeg")),
            RT_BLIP_PNG => Some(Cow::Borrowed("png")),
            RT_BLIP_DIB => Some(Cow::Borrowed("dib")),
            _ => None,
        };

        let record = BlipRecordHeader {
            pos,
            rec_instance,
            rec_len,
            content_start,
            content_end,
        };
        if let Some(format) = format
            && let Some(image) = extract_blip_image(data, record, format, slide_numbers, image_index, warnings)
        {
            images.push(image);
            image_index += 1;
        }

        pos = content_end;
    }

    images
}

/// `OfficeArtSpgrContainer`: a group of shapes. PowerPoint 97-2003 draws a table as one.
const MSOFBT_SPGR_CONTAINER: u16 = 0xF003;
/// `OfficeArtSpContainer`: one shape.
const MSOFBT_SP_CONTAINER: u16 = 0xF004;
/// `OfficeArtFSP`; its `recInstance` is the shape type.
const MSOFBT_FSP: u16 = 0xF00A;
/// `OfficeArtChildAnchor`: a shape's rectangle (left, top, right, bottom) inside its group.
const MSOFBT_CHILD_ANCHOR: u16 = 0xF00F;
/// `tableProperties`, set on the shape of the group that draws a table.
const MSO_PROPERTY_TABLE_PROPERTIES: u16 = 0x039F;
/// `msosptLine`, the shape type of a table's border lines.
const MSOSPT_LINE: u16 = 20;
/// Bounds the grid of one table read from an untrusted stream.
const MAX_TABLE_GRID_CELLS: usize = 10_000;

/// One table cell: the top-left corner of its rectangle in the group, and its text.
struct TableCell {
    left: i32,
    top: i32,
    text: String,
}

/// Read the table a group shape draws, as rows of cell text; `None` when the group in
/// `data[start..end]` is not a table, or no cell of it can be placed.
///
/// A table is a group whose own shape (its first child) carries `tableProperties`, with one
/// text shape per cell and line shapes for the borders, each placed by its `ChildAnchor`.
/// The cells are not stored in reading order -- a table can be stored bottom-right first --
/// so the grid is rebuilt from the anchors: every distinct left edge starts a column and every
/// distinct top edge a row. A cell spanning several columns or rows is written once at its
/// top-left position and the positions it covers stay empty, as `grid_flatten` lays out a
/// spanned cell for the other formats. Columns and rows empty throughout (such as a narrow
/// spacer column) are left out, so a table with no text reads as empty. ~keep
fn read_table(data: &[u8], start: usize, end: usize) -> Option<Vec<Vec<String>>> {
    let children = child_records(data, start, end);
    let (group_shape, shapes) = children.split_first()?;
    if group_shape.rec_type != MSOFBT_SP_CONTAINER
        || !shape_has_property(data, group_shape, MSO_PROPERTY_TABLE_PROPERTIES)
    {
        return None;
    }
    let mut cells = Vec::new();
    for shape in shapes.iter().filter(|r| r.rec_type == MSOFBT_SP_CONTAINER) {
        let mut records = Vec::new();
        collect_art_records(data, shape.content_start, shape.content_end, 0, &mut records);
        if records
            .iter()
            .any(|r| r.rec_type == MSOFBT_FSP && r.rec_instance == MSOSPT_LINE)
        {
            continue;
        }
        let Some(anchor) = records
            .iter()
            .find(|r| r.rec_type == MSOFBT_CHILD_ANCHOR && r.content_end - r.content_start >= 16)
        else {
            continue;
        };
        let edge = |i: usize| read_u32_le(data, anchor.content_start + 4 * i).map(|v| v as i32);
        let (Some(left), Some(top), Some(right), Some(bottom)) = (edge(0), edge(1), edge(2), edge(3)) else {
            continue;
        };
        if right <= left || bottom <= top {
            continue;
        }
        cells.push(TableCell {
            left,
            top,
            text: table_cell_text(data, &records),
        });
        if cells.len() > MAX_TABLE_GRID_CELLS {
            return None;
        }
    }
    if cells.is_empty() {
        return None;
    }

    let edges = |edge: fn(&TableCell) -> i32| {
        let mut all: Vec<i32> = cells.iter().map(edge).collect();
        all.sort_unstable();
        all.dedup();
        all
    };
    let columns = edges(|c| c.left);
    let rows = edges(|c| c.top);
    if columns.len().checked_mul(rows.len())? > MAX_TABLE_GRID_CELLS {
        return None;
    }
    let mut grid = vec![vec![String::new(); columns.len()]; rows.len()];
    for cell in &cells {
        // The edges are taken from the cells, so both lookups find the cell's own corner.
        if let (Ok(r), Ok(c)) = (rows.binary_search(&cell.top), columns.binary_search(&cell.left))
            && grid[r][c].is_empty()
        {
            grid[r][c].clone_from(&cell.text);
        }
    }
    let kept: Vec<usize> = (0..columns.len())
        .filter(|&c| grid.iter().any(|row| !row[c].is_empty()))
        .collect();
    Some(
        grid.into_iter()
            .filter(|row| row.iter().any(|text| !text.is_empty()))
            .map(|row| kept.iter().map(|&c| row[c].clone()).collect())
            .collect(),
    )
}

/// The records directly inside `data[start..end]`, without descending into containers.
fn child_records(data: &[u8], start: usize, end: usize) -> Vec<ArtRecord> {
    let mut out = Vec::new();
    let mut pos = start;
    while pos + 8 <= end {
        let rec_ver_instance = u16::from_le_bytes([data[pos], data[pos + 1]]);
        let rec_type = u16::from_le_bytes([data[pos + 2], data[pos + 3]]);
        let rec_len = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]) as usize;
        let content_start = pos + 8;
        let Some(content_end) = content_start.checked_add(rec_len).filter(|&candidate| candidate <= end) else {
            break;
        };
        out.push(ArtRecord {
            rec_instance: rec_ver_instance >> 4,
            rec_type,
            content_start,
            content_end,
        });
        pos = content_end;
    }
    out
}

/// Whether a shape's own property tables set `property`.
fn shape_has_property(data: &[u8], shape: &ArtRecord, property: u16) -> bool {
    child_records(data, shape.content_start, shape.content_end)
        .iter()
        .filter(|record| MSOFBT_OPT.contains(&record.rec_type))
        .any(|record| {
            (0..record.rec_instance as usize).any(|entry| {
                let at = record.content_start + entry * FOPT_ENTRY_LEN;
                at + FOPT_ENTRY_LEN <= record.content_end
                    && u16::from_le_bytes([data[at], data[at + 1]]) & FOPT_PROPERTY_ID_MASK == property
            })
        })
}

/// A cell's text: its text atoms, cleaned, on one line.
fn table_cell_text(data: &[u8], records: &[ArtRecord]) -> String {
    let mut parts = Vec::new();
    for record in records {
        let Some(body) = data.get(record.content_start..record.content_end) else {
            continue;
        };
        let text: String = match record.rec_type {
            RT_TEXT_CHARS_ATOM => {
                let units: Vec<u16> = body.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
                String::from_utf16_lossy(&units)
            }
            RT_TEXT_BYTES_ATOM => body.iter().map(|&b| cp1252_to_char(b)).collect(),
            _ => continue,
        };
        parts.push(clean_ppt_text(&text));
    }
    parts.join(" ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Clean PPT text: replace control characters and normalize whitespace.
fn clean_ppt_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());

    for c in text.chars() {
        match c {
            '\r' => result.push('\n'),
            '\x0B' => result.push('\n'),
            c if c < '\x20' && c != '\n' && c != '\t' => {}
            _ => result.push(c),
        }
    }

    let cleaned = result
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");

    let trimmed = cleaned.trim();
    if trimmed.chars().all(|c| c == '*' || c == '\n' || c.is_whitespace()) {
        return String::new();
    }

    cleaned
}

/// Convert CP1252 byte to Unicode char.
fn cp1252_to_char(b: u8) -> char {
    match b {
        0x80 => '\u{20AC}',
        0x82 => '\u{201A}',
        0x83 => '\u{0192}',
        0x84 => '\u{201E}',
        0x85 => '\u{2026}',
        0x86 => '\u{2020}',
        0x87 => '\u{2021}',
        0x88 => '\u{02C6}',
        0x89 => '\u{2030}',
        0x8A => '\u{0160}',
        0x8B => '\u{2039}',
        0x8C => '\u{0152}',
        0x8E => '\u{017D}',
        0x91 => '\u{2018}',
        0x92 => '\u{2019}',
        0x93 => '\u{201C}',
        0x94 => '\u{201D}',
        0x95 => '\u{2022}',
        0x96 => '\u{2013}',
        0x97 => '\u{2014}',
        0x98 => '\u{02DC}',
        0x99 => '\u{2122}',
        0x9A => '\u{0161}',
        0x9B => '\u{203A}',
        0x9C => '\u{0153}',
        0x9E => '\u{017E}',
        0x9F => '\u{0178}',
        b => b as char,
    }
}

/// Read a named stream from the CFB compound file.
pub(super) fn read_stream(comp: &mut cfb::CompoundFile<Cursor<&[u8]>>, name: &str) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut stream = comp
        .open_stream(name)
        .map_err(|e| XbergError::parsing(format!("Failed to open stream '{name}': {e}")))?;
    let mut data = Vec::new();
    stream
        .read_to_end(&mut data)
        .map_err(|e| XbergError::parsing(format!("Failed to read stream '{name}': {e}")))?;
    Ok(data)
}

/// Extract metadata from OLE summary information streams.
fn extract_ppt_metadata(comp: &mut cfb::CompoundFile<Cursor<&[u8]>>) -> PptMetadata {
    let mut meta = PptMetadata::default();

    if let Ok(data) = read_stream(comp, "/\x05SummaryInformation") {
        parse_summary_info(&data, &mut meta);
    }

    meta
}

/// Parse OLE SummaryInformation for PPT metadata.
fn parse_summary_info(data: &[u8], meta: &mut PptMetadata) {
    if data.len() < 48 {
        return;
    }

    let set_offset = u32::from_le_bytes([data[44], data[45], data[46], data[47]]) as usize;

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
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .take_while(|&c| c != 0)
                .collect();
            Some(String::from_utf16_lossy(&chars))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests;
