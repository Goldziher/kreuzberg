//! PPTX container and ZIP archive management.
//!
//! This module handles opening PPTX files, reading files from the ZIP archive,
//! finding slide paths, and iterating through slides.

use std::collections::HashMap;
use std::io::{Cursor, Read, Seek};
use std::path::Path;
use zip::ZipArchive;

use super::elements::{Slide, SlideElement};
use super::image_handling::get_full_image_path;
use crate::core::diagnostics::push_warning;
use crate::error::{Result, XbergError};
use crate::extractors::security::{SecurityLimits, ZipBombValidator};
use crate::types::ProcessingWarning;

/// Maximum bytes read from a single PPTX zip member (slide XML, relationship
/// XML, chart/diagram part, or embedded image).
///
/// The pinned `zip` crate (`zip-2.4.2/src/read.rs:444`) wraps a decompressed entry in
/// `Crc32Reader::new(Decompressor::new(..), crc32, ..)` with no `Take` on the
/// decompressed side -- only the *compressed* stream is bounded, and the CRC is
/// checked at EOF, i.e. after the whole entry is already in memory. A member's
/// declared uncompressed size (what `ZipBombValidator`/`check_entry_count` see from
/// the central directory) is therefore not a bound on what reading it actually
/// produces: deflate can expand a small compressed payload by roughly 1032:1. Bounding
/// every `read_to_end` call with `Read::take` (the same pattern as
/// `docx::MAX_UNCOMPRESSED_FILE_SIZE` and `hwpx::MAX_HWPX_MEMBER_SIZE`) is what actually
/// caps memory here.
const MAX_PPTX_MEMBER_SIZE: u64 = 100 * 1024 * 1024;

pub(super) struct PptxContainer<R: Read + Seek> {
    pub(super) archive: ZipArchive<R>,
    slide_paths: Vec<String>,
}

impl PptxContainer<std::fs::File> {
    /// `limits` is the caller's configured `SecurityLimits` (see
    /// `PptxExtractionOptions::security_limits`), not hardcoded ceilings.
    ///
    /// `check_entry_count` and [`ZipBombValidator::validate`] both run here, once per
    /// open. They are not redundant: `check_entry_count` produces the specific
    /// entry-count error message existing callers assert on, while `ZipBombValidator`
    /// additionally bounds aggregate declared uncompressed size and compression ratio,
    /// neither of which PPTX checked before (GH gap: PPTX had no ratio/size check at
    /// all, unlike ODT/ODP -- see `extractors::odt`/`extractors::odp`).
    pub(super) fn open<P: AsRef<Path>>(path: P, limits: &SecurityLimits) -> Result<Self> {
        // IO errors must bubble up unchanged - file access issues need user reports ~keep
        let file = std::fs::File::open(path)?;

        let mut archive = match ZipArchive::new(file) {
            Ok(arc) => arc,
            Err(zip::result::ZipError::Io(io_err)) => return Err(io_err.into()), // Bubble up IO errors ~keep
            Err(e) => {
                return Err(XbergError::parsing(format!(
                    "Failed to read PPTX archive (invalid format): {}",
                    e
                )));
            }
        };
        check_entry_count(&archive, limits.max_files_in_archive)?;
        ZipBombValidator::new(limits.clone()).validate(&mut archive)?;

        let slide_paths = Self::find_slide_paths(&mut archive)?;

        Ok(Self { archive, slide_paths })
    }
}

impl PptxContainer<Cursor<Vec<u8>>> {
    /// `limits` is the caller's configured `SecurityLimits` (see
    /// `PptxExtractionOptions::security_limits`), not hardcoded ceilings.
    ///
    /// See `PptxContainer::open` (the file-based constructor above) for why both
    /// `check_entry_count` and `ZipBombValidator::validate` run here.
    pub(super) fn from_bytes(data: &[u8], limits: &SecurityLimits) -> Result<Self> {
        let cursor = Cursor::new(data.to_vec());

        let mut archive = match ZipArchive::new(cursor) {
            Ok(arc) => arc,
            Err(zip::result::ZipError::Io(io_err)) => return Err(io_err.into()), // Bubble up IO errors ~keep
            Err(e) => {
                return Err(XbergError::parsing(format!(
                    "Failed to read PPTX archive (invalid format): {}",
                    e
                )));
            }
        };
        check_entry_count(&archive, limits.max_files_in_archive)?;
        ZipBombValidator::new(limits.clone()).validate(&mut archive)?;

        let slide_paths = Self::find_slide_paths(&mut archive)?;

        Ok(Self { archive, slide_paths })
    }
}

/// Reject an archive that declares more entries than the caller's configured limit.
///
/// Unlike the OOXML embedded-object path (`extraction::ooxml_embedded`), which can
/// truncate a list of independent embedded files and preserve partial results, the
/// top-level container cannot: slide/relationship resolution depends on specific named
/// parts (`ppt/presentation.xml`, `ppt/_rels/...`) that may not survive an arbitrary
/// truncation of the ZIP's central directory. Erroring out here matches the sibling
/// DOCX container check (`extraction::docx::parser::validate_archive_security`).
fn check_entry_count<R: Read + Seek>(archive: &ZipArchive<R>, max_entries: usize) -> Result<()> {
    if archive.len() > max_entries {
        return Err(XbergError::validation(format!(
            "PPTX archive contains {} entries, exceeds configured limit of {}",
            archive.len(),
            max_entries
        )));
    }
    Ok(())
}

impl<R: Read + Seek> PptxContainer<R> {
    pub(super) fn slide_paths(&self) -> &[String] {
        &self.slide_paths
    }

    pub(super) fn read_file(&mut self, path: &str) -> Result<Vec<u8>> {
        match self.archive.by_name(path) {
            Ok(file) => {
                let mut contents = Vec::new();
                // IO errors must bubble up - file read issues need user reports ~keep
                file.take(MAX_PPTX_MEMBER_SIZE).read_to_end(&mut contents)?;
                Ok(contents)
            }
            Err(zip::result::ZipError::FileNotFound) => {
                Err(XbergError::parsing("File not found in archive".to_string()))
            }
            Err(zip::result::ZipError::Io(io_err)) => Err(io_err.into()), // Bubble up IO errors ~keep
            Err(e) => Err(XbergError::parsing(format!("Zip error: {}", e))),
        }
    }

    pub(super) fn get_slide_rels_path(&self, slide_path: &str) -> String {
        super::image_handling::get_slide_rels_path(slide_path)
    }

    fn find_slide_paths(archive: &mut ZipArchive<R>) -> Result<Vec<String>> {
        if let Ok(rels_data) = Self::read_file_from_archive(archive, "ppt/_rels/presentation.xml.rels")
            && let Ok(paths) = super::parser::parse_presentation_rels(&rels_data)
        {
            return Ok(paths);
        }

        let mut slide_paths = Vec::new();
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                let name = file
                    .name()
                    .map_err(|error| XbergError::parsing(format!("Invalid ZIP entry name: {error}")))?;
                if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                    slide_paths.push(name.into_owned());
                }
            }
        }

        slide_paths.sort_by(|a, b| {
            fn slide_num(s: &str) -> u32 {
                s.rsplit('/')
                    .next()
                    .unwrap_or("")
                    .strip_prefix("slide")
                    .unwrap_or("")
                    .strip_suffix(".xml")
                    .unwrap_or("")
                    .parse()
                    .unwrap_or(u32::MAX)
            }
            slide_num(a).cmp(&slide_num(b))
        });
        Ok(slide_paths)
    }

    fn read_file_from_archive(archive: &mut ZipArchive<R>, path: &str) -> Result<Vec<u8>> {
        let file = match archive.by_name(path) {
            Ok(f) => f,
            Err(zip::result::ZipError::Io(io_err)) => return Err(io_err.into()), // Bubble up IO errors ~keep
            Err(e) => {
                return Err(XbergError::parsing(format!("Failed to read file from archive: {}", e)));
            }
        };
        let mut contents = Vec::new();
        // IO errors must bubble up - file read issues need user reports ~keep
        file.take(MAX_PPTX_MEMBER_SIZE).read_to_end(&mut contents)?;
        Ok(contents)
    }
}

pub(super) struct SlideIterator<R: Read + Seek> {
    container: PptxContainer<R>,
    current_index: usize,
    total_slides: usize,
}

impl<R: Read + Seek> SlideIterator<R> {
    pub(super) fn new(container: PptxContainer<R>) -> Self {
        let total_slides = container.slide_paths().len();
        Self {
            container,
            current_index: 0,
            total_slides,
        }
    }

    pub(super) fn slide_count(&self) -> usize {
        self.total_slides
    }

    /// Read and parse the next slide, skipping (and warning about) any slide
    /// whose XML part could not be read or parsed rather than aborting the
    /// whole presentation for one bad slide (#91).
    pub(super) fn next_slide(&mut self, warnings: &mut Vec<ProcessingWarning>) -> Result<Option<Slide>> {
        while self.current_index < self.total_slides {
            let slide_path = self.container.slide_paths()[self.current_index].clone();
            let slide_number = (self.current_index + 1) as u32;
            self.current_index += 1;

            let xml_data = match self.container.read_file(&slide_path) {
                Ok(data) => data,
                Err(e) => {
                    push_warning(
                        warnings,
                        "pptx",
                        format!(
                            "Could not read slide {} ('{}'): {}; slide content was not extracted",
                            slide_number, slide_path, e
                        ),
                    );
                    continue;
                }
            };

            let rels_path = self.container.get_slide_rels_path(&slide_path);
            let rels_data = self.container.read_file(&rels_path).ok();

            let mut slide = match Slide::from_xml(slide_number, &xml_data, rels_data.as_deref()) {
                Ok(slide) => slide,
                Err(e) => {
                    push_warning(
                        warnings,
                        "pptx",
                        format!(
                            "Could not parse slide {} ('{}'): {}; slide content was not extracted",
                            slide_number, slide_path, e
                        ),
                    );
                    continue;
                }
            };

            Self::resolve_graphic_frame_text(
                &mut self.container,
                &mut slide.elements,
                &slide.rel_targets,
                &slide_path,
                warnings,
            );

            return Ok(Some(slide));
        }

        Ok(None)
    }

    /// Resolve chart and SmartArt/diagram text that lives in a separate ZIP
    /// part, referenced from the slide only by relationship ID (#80). Failures
    /// to read or parse the referenced part are surfaced as warnings rather
    /// than silently dropping the chart/diagram's text.
    fn resolve_graphic_frame_text(
        container: &mut PptxContainer<R>,
        elements: &mut [SlideElement],
        rel_targets: &ahash::AHashMap<String, String>,
        slide_path: &str,
        warnings: &mut Vec<ProcessingWarning>,
    ) {
        for elem in elements.iter_mut() {
            match elem {
                SlideElement::Chart(chart_ref, _) => {
                    let Some(target) = rel_targets.get(&chart_ref.rel_id) else {
                        continue;
                    };
                    let full_path = get_full_image_path(slide_path, target);
                    match container.read_file(&full_path) {
                        Ok(bytes) => match super::parser::parse_chart_text(&bytes) {
                            Ok(text) => chart_ref.resolved_text = text,
                            Err(e) => push_warning(
                                warnings,
                                "pptx",
                                format!(
                                    "Could not parse chart part '{}': {}; chart text was not extracted",
                                    full_path, e
                                ),
                            ),
                        },
                        Err(e) => push_warning(
                            warnings,
                            "pptx",
                            format!(
                                "Could not read chart part '{}': {}; chart text was not extracted",
                                full_path, e
                            ),
                        ),
                    }
                }
                SlideElement::SmartArt(diagram_ref, _) => {
                    let Some(target) = rel_targets.get(&diagram_ref.rel_id) else {
                        continue;
                    };
                    let full_path = get_full_image_path(slide_path, target);
                    match container.read_file(&full_path) {
                        Ok(bytes) => match super::parser::parse_diagram_text(&bytes) {
                            Ok(text) => diagram_ref.resolved_text = text,
                            Err(e) => push_warning(
                                warnings,
                                "pptx",
                                format!(
                                    "Could not parse SmartArt data part '{}': {}; diagram text was not extracted",
                                    full_path, e
                                ),
                            ),
                        },
                        Err(e) => push_warning(
                            warnings,
                            "pptx",
                            format!(
                                "Could not read SmartArt data part '{}': {}; diagram text was not extracted",
                                full_path, e
                            ),
                        ),
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn get_slide_images(&mut self, slide: &Slide) -> Result<HashMap<String, Vec<u8>>> {
        let mut image_data = HashMap::new();

        for img_ref in &slide.images {
            let slide_path = &self.container.slide_paths()[slide.slide_number as usize - 1];
            let full_path = get_full_image_path(slide_path, &img_ref.target);

            if let Ok(data) = self.container.read_file(&full_path) {
                image_data.insert(img_ref.id.clone(), data);
            }
        }

        Ok(image_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Build a minimal ZIP with `entry_count` empty files, none of them PPTX parts.
    /// The archive is invalid as a presentation, so these tests exercise only the
    /// entry-count gate that runs before any slide/relationship lookup.
    fn build_zip_with_entries(entry_count: usize) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);
        for i in 0..entry_count {
            zip.start_file(format!("file_{}.txt", i), options).unwrap();
            zip.write_all(b"").unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    /// GH#639: PPTX had no entry-count check at all, so this fails against the
    /// unfixed code (open() never returns an error here, regardless of `max_entries`).
    #[test]
    fn test_from_bytes_rejects_too_many_entries_under_configured_limit() {
        let data = build_zip_with_entries(5);
        let limits = SecurityLimits {
            max_files_in_archive: 3,
            ..Default::default()
        };
        let result = PptxContainer::from_bytes(&data, &limits);
        assert!(
            result.is_err(),
            "5 entries must be rejected against a configured limit of 3"
        );
        let err_msg = result.err().unwrap().to_string();
        assert!(
            err_msg.contains('5') && err_msg.contains('3'),
            "error should mention actual and configured limit counts, got: {}",
            err_msg
        );
    }

    /// Sibling of the rejection test above: the same entry count under a limit
    /// that comfortably fits it must pass the gate (and fail later, harmlessly,
    /// on slide discovery since this fixture has no real PPTX parts).
    #[test]
    fn test_check_entry_count_allows_entries_within_configured_limit() {
        let data = build_zip_with_entries(5);
        let cursor = Cursor::new(data);
        let archive = ZipArchive::new(cursor).unwrap();
        assert!(
            check_entry_count(&archive, 10).is_ok(),
            "5 entries must pass against a configured limit of 10"
        );
    }

    /// `check_entry_count` only inspects the ZIP central directory (entry sizes as
    /// *declared*, never decompressed); it has no way to bound what a single member
    /// actually expands to when read. This builds one oversized member (padded past
    /// `MAX_PPTX_MEMBER_SIZE` with a marker after the cap) and proves `read_file`
    /// itself stops at the cap: without `Read::take(MAX_PPTX_MEMBER_SIZE)` in
    /// `read_file`, the whole member -- including the post-cap marker -- would come
    /// back.
    #[test]
    fn test_read_file_bounds_an_oversized_member() {
        let marker_before = b"BEFORE-CAP";
        let marker_after = b"AFTER-CAP-MARKER";
        let padding_len = MAX_PPTX_MEMBER_SIZE as usize + 4096 - marker_before.len() - marker_after.len();
        let mut payload = Vec::with_capacity(MAX_PPTX_MEMBER_SIZE as usize + 4096);
        payload.extend_from_slice(marker_before);
        payload.extend(vec![b'x'; padding_len]);
        payload.extend_from_slice(marker_after);

        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("ppt/slides/oversized.bin", options).unwrap();
        zip.write_all(&payload).unwrap();
        let data = zip.finish().unwrap().into_inner();

        let limits = SecurityLimits {
            max_files_in_archive: 10,
            ..Default::default()
        };
        let mut container =
            PptxContainer::from_bytes(&data, &limits).expect("archive under the entry-count limit opens");
        let contents = container
            .read_file("ppt/slides/oversized.bin")
            .expect("a truncated read must still succeed, not error");

        assert_eq!(
            contents.len(),
            MAX_PPTX_MEMBER_SIZE as usize,
            "read_file must stop at exactly MAX_PPTX_MEMBER_SIZE bytes"
        );
        assert!(
            contents.starts_with(marker_before),
            "content before the cap must be preserved"
        );
        assert!(
            !contents.ends_with(marker_after.as_slice()),
            "content after the cap must never be read; finding the marker means the read was not bounded"
        );
    }
}
