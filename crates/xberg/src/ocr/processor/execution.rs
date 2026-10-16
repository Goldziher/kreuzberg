//! OCR execution and result processing.
//!
//! This module handles the core OCR execution logic, including image processing,
//! text extraction, and result formatting.

use super::api_pool::TesseractApiPool;
use super::config::{apply_tesseract_variables, hash_config};
use super::validation::{
    resolve_all_installed_languages, resolve_tessdata_path, strip_control_characters, validate_language_and_traineddata,
};
use crate::core::config::ExtractionConfig;
/// GH#1630/GH#1621: the explicit-hint-then-embedded-PNG-density-then-`None` precedence is
/// defined once and shared with `extractors::image::normalize_image_bytes_for_ocr`.
use crate::extraction::image::resolve_known_source_dpi;
use crate::extractors::security::SecurityLimits;
use crate::image::normalize_image_dpi_owned;
use crate::ocr::cache::OcrCache;
use crate::ocr::conversion::{TsvRow, iterator_word_to_element, tesseract_elements_with_lines, tsv_row_to_element};
use crate::ocr::error::OcrError;
use crate::ocr::hocr_parser::{
    DictionaryLineFilter, RetainedWordConfidenceStats, parse_hocr_to_internal_document_with_page_offset_and_stats,
};
use crate::ocr::preprocessing::preprocess_pix;
#[cfg(test)]
use crate::ocr::preprocessing::should_invert_for_polarity;
#[cfg(feature = "pdf")]
use crate::ocr::table::post_process_table;
use crate::ocr::table::{
    TableWords, drop_document_elements_claimed_by_tables, extract_table_words_from_tsv, extract_words_from_tsv,
    orient_table_for_reading_order, reconstruct_table_with_columns, shading_mark_keep_mask, should_adopt_table_rebuild,
    table_to_markdown,
};
#[cfg(test)]
use crate::ocr::types::BatchItemResult;
use crate::ocr::types::TesseractConfig;
use crate::types::internal::{ElementKind, InternalDocument};
use crate::types::{OcrExtractionResult, OcrTable, OcrTableBoundingBox};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use xberg_tesseract::{TessPageSegMode, TessPolyBlockType, TesseractAPI};

/// Process-global document-orientation classifier (ONNX PP-LCNet), shared by
/// every tesseract-backend auto-rotate call. Session initialization is lazy and
/// internally synchronized; `detect` is thread-safe.
#[cfg(auto_rotate)]
fn doc_orientation_detector() -> &'static crate::doc_orientation::DocOrientationDetector {
    static DETECTOR: std::sync::LazyLock<crate::doc_orientation::DocOrientationDetector> =
        std::sync::LazyLock::new(|| {
            crate::doc_orientation::DocOrientationDetector::with_acceleration(
                crate::doc_orientation::resolve_cache_dir(),
                None,
            )
        });
    &DETECTOR
}

use crate::table_core::{
    HocrWord, MIN_TABLE_CANDIDATE_WORDS, cluster_word_indices_into_table_regions, detect_rows,
    drop_leading_caption_row, hocr_words_are_rtl, median_word_height, merge_disjoint_numeric_columns,
};
use crate::types::OcrElement;

#[cfg(auto_rotate)]
/// Rotate raw RGB image data by the given degrees (0, 90, 180, 270).
///
/// Returns the rotated pixel data and the new (width, height).
/// For 90° and 270° rotations, width and height are swapped.
fn rotate_rgb_image_data(data: &[u8], width: u32, height: u32, degrees: i32) -> (Vec<u8>, u32, u32) {
    let bpp = 3usize;
    let w = width as usize;
    let h = height as usize;

    match degrees {
        0 => (data.to_vec(), width, height),
        90 => {
            let new_w = h;
            let new_h = w;
            let mut out = vec![0u8; new_w * new_h * bpp];
            for y in 0..h {
                for x in 0..w {
                    let src_idx = (y * w + x) * bpp;
                    let dst_x = h - 1 - y;
                    let dst_y = x;
                    let dst_idx = (dst_y * new_w + dst_x) * bpp;
                    out[dst_idx..dst_idx + bpp].copy_from_slice(&data[src_idx..src_idx + bpp]);
                }
            }
            (out, new_w as u32, new_h as u32)
        }
        180 => {
            let mut out = vec![0u8; w * h * bpp];
            for y in 0..h {
                for x in 0..w {
                    let src_idx = (y * w + x) * bpp;
                    let dst_x = w - 1 - x;
                    let dst_y = h - 1 - y;
                    let dst_idx = (dst_y * w + dst_x) * bpp;
                    out[dst_idx..dst_idx + bpp].copy_from_slice(&data[src_idx..src_idx + bpp]);
                }
            }
            (out, width, height)
        }
        270 => {
            let new_w = h;
            let new_h = w;
            let mut out = vec![0u8; new_w * new_h * bpp];
            for y in 0..h {
                for x in 0..w {
                    let src_idx = (y * w + x) * bpp;
                    let dst_x = y;
                    let dst_y = w - 1 - x;
                    let dst_idx = (dst_y * new_w + dst_x) * bpp;
                    out[dst_idx..dst_idx + bpp].copy_from_slice(&data[src_idx..src_idx + bpp]);
                }
            }
            (out, new_w as u32, new_h as u32)
        }
        _ => {
            tracing::warn!("Unsupported rotation angle: {}°, skipping rotation", degrees);
            (data.to_vec(), width, height)
        }
    }
}

/// Parse Tesseract TSV output into structured OcrElements.
///
/// TSV format columns: level, page_num, block_num, par_num, line_num, word_num, left, top, width, height, conf, text
///
/// # Arguments
///
/// * `tsv_data` - Raw TSV output from Tesseract
/// * `min_confidence` - Minimum confidence threshold (0-100 scale)
/// * `page_number` - The true, 1-indexed page number of the source document this TSV came
///   from. Tesseract's own `page_num` TSV column is always `1` for a single-image call (see
///   `TesseractConfig::page_number`'s doc comment), so it is used only to build each
///   element's opaque `parent_id`, never as the returned elements' page number.
///
/// # Returns
///
/// One line element per text line, then the word elements. Tesseract's own line rows carry no
/// text or confidence, so each line is built from its words.
fn parse_tsv_to_elements(tsv_data: &str, min_confidence: f64, page_number: u32) -> Vec<OcrElement> {
    let mut words = Vec::new();

    for line in tsv_data.lines().skip(1) {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 12 {
            continue;
        }

        let level = fields[0].parse::<i32>().unwrap_or(0);
        let page_num = fields[1].parse::<i32>().unwrap_or(1);
        let block_num = fields[2].parse::<i32>().unwrap_or(0);
        let par_num = fields[3].parse::<i32>().unwrap_or(0);
        let line_num = fields[4].parse::<i32>().unwrap_or(0);
        let word_num = fields[5].parse::<i32>().unwrap_or(0);
        let left = fields[6].parse::<u32>().unwrap_or(0);
        let top = fields[7].parse::<u32>().unwrap_or(0);
        let width = fields[8].parse::<u32>().unwrap_or(0);
        let height = fields[9].parse::<u32>().unwrap_or(0);
        let conf = fields[10].parse::<f64>().unwrap_or(-1.0);
        let text = fields[11].to_string();

        if conf < 0.0 || conf < min_confidence || text.trim().is_empty() {
            continue;
        }

        if level != 5 {
            continue;
        }

        let tsv_row = TsvRow {
            level,
            page_num,
            block_num,
            par_num,
            line_num,
            word_num,
            left,
            top,
            width,
            height,
            conf,
            text,
        };

        let mut element = tsv_row_to_element(&tsv_row);
        element.page_number = page_number;
        words.push(((page_num, block_num, par_num, line_num), element));
    }

    tesseract_elements_with_lines(words)
}

/// CI debug logging utility.
///
/// Logs debug messages when XBERG_CI_DEBUG environment variable is set.
fn log_ci_debug<F>(enabled: bool, stage: &str, details: F)
where
    F: FnOnce() -> String,
{
    if !enabled {
        return;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    tracing::debug!(stage, timestamp = format!("{timestamp:.3}"), "{}", details());
}

const MIN_QUANTITY_COLUMN_SUPPORT: usize = 2;
const MAX_RETRY_QUANTITY_DIGITS: usize = 8;
const MIN_SOURCE_QUANTITY_RETRY_CONFIDENCE: f64 = 50.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QuantityRetryRegion {
    row: usize,
    column: usize,
    left: u32,
    top: u32,
    width: u32,
    height: u32,
    word_left: u32,
    word_top: u32,
    word_width: u32,
    word_height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QuantityRetryColumn {
    index: usize,
    left: u32,
    right: u32,
    median_height: u32,
    word_width: u32,
    cell_left: u32,
    cell_right: u32,
}

fn nearest_position_index(positions: &[u32], value: u32) -> Option<usize> {
    if positions.is_empty() {
        return None;
    }
    match positions.binary_search(&value) {
        Ok(index) => Some(index),
        Err(0) => Some(0),
        Err(index) if index == positions.len() => Some(index - 1),
        Err(index) => {
            let left = index - 1;
            Some(if positions[left].abs_diff(value) <= positions[index].abs_diff(value) {
                left
            } else {
                index
            })
        }
    }
}

/// Retry only an empty cell in a literal `QTY` column supported by at least two recognized
/// integer quantities. The retry reads pixels from that cell; it never derives a value from
/// prices or totals, so an intentionally blank quantity remains blank. ~keep
fn quantity_retry_region(
    words: &[HocrWord],
    quantity_column: usize,
    blank_row: usize,
    row_positions: &[u32],
    column_positions: &[u32],
    image_width: u32,
    image_height: u32,
) -> Option<QuantityRetryRegion> {
    if blank_row >= row_positions.len() || quantity_column >= column_positions.len() {
        return None;
    }
    let column = quantity_retry_column(words, column_positions, quantity_column, image_width);
    retry_region_for_row(words, blank_row, column, row_positions, image_width, image_height)
}

fn quantity_retry_column_index(table: &[Vec<String>]) -> Option<(usize, usize)> {
    let quantity_column = table
        .first()?
        .iter()
        .position(|header| header.trim().eq_ignore_ascii_case("qty"))?;
    let mut support = 0usize;
    let mut blank_row = None;
    for (row_index, row) in table.iter().enumerate().skip(1) {
        let quantity = row.get(quantity_column)?.trim();
        if quantity.is_empty() {
            if blank_row.replace(row_index).is_some() {
                return None;
            }
        } else if quantity.bytes().all(|byte| byte.is_ascii_digit()) {
            support += 1;
        } else {
            return None;
        }
    }
    (support >= MIN_QUANTITY_COLUMN_SUPPORT).then_some((quantity_column, blank_row?))
}

fn quantity_retry_column(
    words: &[HocrWord],
    column_positions: &[u32],
    index: usize,
    image_width: u32,
) -> QuantityRetryColumn {
    let (left, right, median_height, word_width) = quantity_column_geometry(words, column_positions, index);
    let (cell_left, cell_right) = cell_axis_bounds(column_positions, index, image_width);
    QuantityRetryColumn {
        index,
        left,
        right,
        median_height,
        word_width,
        cell_left,
        cell_right,
    }
}

fn quantity_column_geometry(words: &[HocrWord], column_positions: &[u32], index: usize) -> (u32, u32, u32, u32) {
    let mean_height = nonzero_mean(words.iter().map(|word| word.height), 1);
    let mut left = None;
    let mut right = None;
    let mut width_sum = 0u64;
    let mut width_count = 0u64;
    for word in words {
        if nearest_position_index(column_positions, word.left) != Some(index) {
            continue;
        }
        left = Some(left.map_or(word.left, |current: u32| current.min(word.left)));
        let word_right = word.left.saturating_add(word.width);
        right = Some(right.map_or(word_right, |current: u32| current.max(word_right)));
        if word.width > 0 {
            width_sum += u64::from(word.width);
            width_count += 1;
        }
    }
    let left = left.unwrap_or(column_positions[index]);
    let right = right.unwrap_or(column_positions[index].saturating_add(mean_height));
    let word_width = width_sum
        .checked_div(width_count)
        .map_or(mean_height, |mean| u32::try_from(mean).unwrap_or(u32::MAX));
    (left, right, mean_height, word_width)
}

fn nonzero_mean(values: impl Iterator<Item = u32>, fallback: u32) -> u32 {
    let (sum, count) = values
        .filter(|&value| value > 0)
        .fold((0u64, 0u64), |(sum, count), value| (sum + u64::from(value), count + 1));
    sum.checked_div(count)
        .map_or(fallback, |mean| u32::try_from(mean).unwrap_or(u32::MAX))
}

fn cell_axis_bounds(positions: &[u32], index: usize, limit: u32) -> (u32, u32) {
    let midpoint = |left: u32, right: u32| u32::try_from((u64::from(left) + u64::from(right)) / 2).unwrap_or(limit);
    let lower = index
        .checked_sub(1)
        .map_or(0, |left| midpoint(positions[left], positions[index]));
    let upper = positions
        .get(index + 1)
        .map_or(limit, |&right| midpoint(positions[index], right));
    (lower.min(limit), upper.min(limit))
}

fn retry_region_for_row(
    words: &[HocrWord],
    row: usize,
    column: QuantityRetryColumn,
    row_positions: &[u32],
    image_width: u32,
    image_height: u32,
) -> Option<QuantityRetryRegion> {
    let mut row_top = None;
    let mut row_bottom = None;
    for word in words {
        if nearest_position_index(row_positions, word.y_center() as u32) != Some(row) {
            continue;
        }
        row_top = Some(row_top.map_or(word.top, |current: u32| current.min(word.top)));
        let word_bottom = word.top.saturating_add(word.height);
        row_bottom = Some(row_bottom.map_or(word_bottom, |current: u32| current.max(word_bottom)));
    }
    let row_top = row_top?;
    let row_bottom = row_bottom?;
    let horizontal_padding = column.median_height;
    let vertical_padding = column.median_height / 2;
    let (row_cell_top, row_cell_bottom) = cell_axis_bounds(row_positions, row, image_height);
    let left = column.left.saturating_sub(horizontal_padding).max(column.cell_left);
    let right = column
        .right
        .saturating_add(horizontal_padding)
        .min(image_width)
        .min(column.cell_right);
    let top = row_top.saturating_sub(vertical_padding).max(row_cell_top);
    let bottom = row_bottom
        .saturating_add(vertical_padding)
        .min(image_height)
        .min(row_cell_bottom);
    if right <= left || bottom <= top {
        return None;
    }
    Some(QuantityRetryRegion {
        row,
        column: column.index,
        left,
        top,
        width: right - left,
        height: bottom - top,
        word_left: column.left,
        word_top: row_positions[row].saturating_sub(column.median_height / 2),
        word_width: column.word_width,
        word_height: column.median_height,
    })
}

fn parse_retry_quantity(text: &str) -> Option<&str> {
    let quantity = text.trim();
    (!quantity.is_empty()
        && quantity.len() <= MAX_RETRY_QUANTITY_DIGITS
        && quantity.bytes().all(|byte| byte.is_ascii_digit())
        && quantity.bytes().any(|byte| byte != b'0'))
    .then_some(quantity)
}

fn recover_blank_quantity_word(
    api: &TesseractAPI,
    config: &TesseractConfig,
    region: Option<&QuantityRetryRegion>,
    image_width: u32,
    image_height: u32,
) -> Option<HocrWord> {
    let region = region?;
    if api.set_page_seg_mode(TessPageSegMode::PSM_SINGLE_LINE).is_err() {
        return None;
    }
    let recovered = recognize_quantity_region(api, config, region);
    let _ = api.set_rectangle(
        0,
        0,
        i32::try_from(image_width).unwrap_or(i32::MAX),
        i32::try_from(image_height).unwrap_or(i32::MAX),
    );
    let _ = api.set_page_seg_mode(TessPageSegMode::from_int(config.psm as i32));
    recovered
}

fn source_retry_crop(
    region: &QuantityRetryRegion,
    processed_width: u32,
    processed_height: u32,
    source_width: u32,
    source_height: u32,
) -> Option<(u32, u32, u32, u32)> {
    if processed_width == 0 || processed_height == 0 || source_width == 0 || source_height == 0 {
        return None;
    }
    let scale = |coordinate: u32, source: u32, processed: u32, round_up: bool| {
        let numerator = u64::from(coordinate) * u64::from(source);
        let denominator = u64::from(processed);
        let offset = if round_up { denominator - 1 } else { 0 };
        u32::try_from((numerator + offset) / denominator)
            .ok()
            .map(|value| value.min(source))
    };
    let left = scale(region.left, source_width, processed_width, false)?;
    let top = scale(region.top, source_height, processed_height, false)?;
    let right = scale(
        region.left.saturating_add(region.width),
        source_width,
        processed_width,
        true,
    )?;
    let bottom = scale(
        region.top.saturating_add(region.height),
        source_height,
        processed_height,
        true,
    )?;
    (right > left && bottom > top).then_some((left, top, right - left, bottom - top))
}

fn recover_blank_quantity_from_source(
    image_bytes: &[u8],
    config: &TesseractConfig,
    region: &QuantityRetryRegion,
    processed_width: u32,
    processed_height: u32,
    tessdata_path: &str,
    security_limits: &SecurityLimits,
) -> Option<HocrWord> {
    if config.auto_rotate
        || config
            .preprocessing
            .as_ref()
            .is_some_and(|preprocessing| preprocessing.auto_rotate)
    {
        return None;
    }
    // ~keep DPI normalization is scale-only; optional deskew can shift glyphs slightly, so the
    // mapped crop is bounded to the missing cell and accepts only a high-confidence integer.
    let source = crate::extraction::image::load_image_for_ocr(image_bytes, security_limits)
        .ok()?
        .into_rgb8();
    let (left, top, width, height) = source_retry_crop(
        region,
        processed_width,
        processed_height,
        source.width(),
        source.height(),
    )?;
    let crop = image::imageops::crop_imm(&source, left, top, width, height).to_image();
    let api = TesseractAPI::new().ok()?;
    api.init(tessdata_path, &config.language).ok()?;
    api.set_page_seg_mode(TessPageSegMode::PSM_SINGLE_LINE).ok()?;
    api.set_image(
        crop.as_raw(),
        i32::try_from(width).ok()?,
        i32::try_from(height).ok()?,
        3,
        i32::try_from(width.checked_mul(3)?).ok()?,
    )
    .ok()?;
    api.recognize().ok()?;
    let text = api.get_utf8_text().ok()?;
    let quantity = parse_retry_quantity(&text)?;
    let confidence = f64::from(api.mean_text_conf().ok()?);
    if confidence < config.table_min_confidence.max(MIN_SOURCE_QUANTITY_RETRY_CONFIDENCE) {
        return None;
    }
    Some(HocrWord {
        text: quantity.to_string(),
        left: region.word_left,
        top: region.word_top,
        width: region.word_width,
        height: region.word_height,
        confidence,
    })
}

fn recognize_quantity_region(
    api: &TesseractAPI,
    config: &TesseractConfig,
    region: &QuantityRetryRegion,
) -> Option<HocrWord> {
    let left = i32::try_from(region.left).ok()?;
    let top = i32::try_from(region.top).ok()?;
    let width = i32::try_from(region.width).ok()?;
    let height = i32::try_from(region.height).ok()?;
    api.set_rectangle(left, top, width, height).ok()?;
    api.recognize().ok()?;
    let text = api.get_utf8_text().ok()?;
    let quantity = parse_retry_quantity(&text)?;
    let confidence = api.mean_text_conf().unwrap_or(-1);
    if f64::from(confidence) < config.table_min_confidence {
        return None;
    }
    Some(HocrWord {
        text: quantity.to_string(),
        left: region.word_left,
        top: region.word_top,
        width: region.word_width,
        height: region.word_height,
        confidence: f64::from(confidence),
    })
}

/// The words of one table region, each with the box Tesseract read for it at the same index.
struct TableRegion {
    words: Vec<HocrWord>,
    read_boxes: Vec<HocrWord>,
}

/// A table needs at least two words with a letter or number. This rejects punctuation-only scan
/// fragments while retaining their words in the surrounding OCR text. ~keep
fn has_semantic_ocr_table_content(words: &[HocrWord]) -> bool {
    words
        .iter()
        .filter(|word| word.text.chars().any(char::is_alphanumeric))
        .take(2)
        .count()
        == 2
}

impl TableRegion {
    fn push(&mut self, word: HocrWord) {
        self.read_boxes.push(word.clone());
        self.words.push(word);
    }

    /// Drop the shading marks Tesseract read as words, from `words` and `read_boxes` together.
    ///
    /// The decision reads `read_boxes`, not `words`: `words` may already have been moved onto its
    /// row's line band, which pulls a mark's height towards the row median and hides it from the
    /// filter. Filtering both vectors with one mask keeps the same-index invariant `bounding_box`
    /// relies on. ~keep
    fn drop_shading_marks(&mut self, row_threshold_ratio: f64) {
        let keep = shading_mark_keep_mask(&self.read_boxes, row_threshold_ratio);
        debug_assert_eq!(
            self.words.len(),
            self.read_boxes.len(),
            "TableRegion vectors must stay parallel"
        );
        self.words = std::mem::take(&mut self.words)
            .into_iter()
            .zip(&keep)
            .filter(|(_, keep)| **keep)
            .map(|(word, _)| word)
            .collect();
        self.read_boxes = std::mem::take(&mut self.read_boxes)
            .into_iter()
            .zip(&keep)
            .filter(|(_, keep)| **keep)
            .map(|(word, _)| word)
            .collect();
    }

    /// Remove the frame strokes produced by normalized shaded bands when Tesseract reads them as
    /// leading `[` glyphs or outer-edge `|` words. Requiring repeated bracket-prefixed alphabetic
    /// words at the same left edge distinguishes this raster artifact from real bracketed content;
    /// the caller additionally gates it on shaded-row normalization. ~keep
    fn drop_normalized_band_edge_glyphs(&mut self) {
        if self.read_boxes.len() < 2 {
            return;
        }
        let left = self.read_boxes.iter().map(|word| word.left).min().unwrap_or(0);
        let right = self
            .read_boxes
            .iter()
            .map(|word| word.left.saturating_add(word.width))
            .max()
            .unwrap_or(0);
        let edge_tolerance = median_word_height(&self.read_boxes).max(1);
        let is_bracket_prefix = |word: &HocrWord| {
            word.left <= left.saturating_add(edge_tolerance)
                && word
                    .text
                    .strip_prefix('[')
                    .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|character| character.is_alphabetic()))
        };
        if self.read_boxes.iter().filter(|word| is_bracket_prefix(word)).count() < 2 {
            return;
        }

        let keep: Vec<bool> = self
            .read_boxes
            .iter()
            .map(|word| {
                let word_right = word.left.saturating_add(word.width);
                word.text != "|"
                    || !(word.left <= left.saturating_add(edge_tolerance)
                        || word_right.saturating_add(edge_tolerance) >= right)
            })
            .collect();
        for (word, read_box) in self.words.iter_mut().zip(&mut self.read_boxes) {
            if is_bracket_prefix(read_box) {
                word.text.remove(0);
                read_box.text.remove(0);
            }
        }
        self.words = std::mem::take(&mut self.words)
            .into_iter()
            .zip(&keep)
            .filter(|(_, keep)| **keep)
            .map(|(word, _)| word)
            .collect();
        self.read_boxes = std::mem::take(&mut self.read_boxes)
            .into_iter()
            .zip(&keep)
            .filter(|(_, keep)| **keep)
            .map(|(word, _)| word)
            .collect();
    }

    /// The box around the words as Tesseract read them, underscore marks included. The text
    /// outside the table is picked by the centres of the same boxes, so every word of the table
    /// falls inside it. ~keep
    fn bounding_box(&self) -> OcrTableBoundingBox {
        OcrTableBoundingBox {
            left: self.read_boxes.iter().map(|w| w.left).min().unwrap_or(0),
            top: self.read_boxes.iter().map(|w| w.top).min().unwrap_or(0),
            right: self.read_boxes.iter().map(|w| w.left + w.width).max().unwrap_or(0),
            bottom: self.read_boxes.iter().map(|w| w.top + w.height).max().unwrap_or(0),
        }
    }
}

/// Split the table words into vertically separated regions, one table each.
fn table_regions(table_words: &TableWords) -> Vec<TableRegion> {
    cluster_word_indices_into_table_regions(&table_words.words)
        .into_iter()
        .map(|indices| TableRegion {
            words: indices.iter().map(|&i| table_words.words[i].clone()).collect(),
            read_boxes: indices.iter().map(|&i| table_words.read_boxes[i].clone()).collect(),
        })
        .collect()
}

/// Build content with OCR tables inlined at their correct vertical positions.
///
/// Parses TSV word positions to separate table words from non-table words,
/// groups non-table words into lines and paragraphs, then interleaves
/// paragraphs and table markdown sorted by Y-position.
fn build_content_with_inline_tables(tsv_data: &str, tables: &[OcrTable], min_confidence: f64) -> String {
    let words = match extract_words_from_tsv(tsv_data, min_confidence) {
        Ok(w) => w,
        Err(_) => return String::new(),
    };

    if words.is_empty() {
        return String::new();
    }

    let table_bboxes: Vec<_> = tables.iter().filter_map(|t| t.bounding_box.as_ref()).collect();

    let mut non_table_words = Vec::new();
    for word in &words {
        let in_table = table_bboxes.iter().any(|bbox| {
            let word_cx = word.left + word.width / 2;
            let word_cy = word.top + word.height / 2;
            word_cx >= bbox.left && word_cx <= bbox.right && word_cy >= bbox.top && word_cy <= bbox.bottom
        });
        if !in_table {
            non_table_words.push(word);
        }
    }

    if non_table_words.is_empty() && tables.is_empty() {
        return String::new();
    }

    let mut sorted_words = non_table_words;
    sorted_words.sort_by_key(|word| word.top + word.height / 2);

    let avg_height = if sorted_words.is_empty() {
        20
    } else {
        let total_h: u32 = sorted_words.iter().map(|w| w.height).sum();
        (total_h / sorted_words.len() as u32).max(1)
    };
    let line_threshold = avg_height / 2;

    struct TextLine<'a> {
        y_center: u32,
        y_sum: u64,
        words: Vec<&'a HocrWord>,
    }

    let mut lines: Vec<TextLine> = Vec::new();
    for word in sorted_words {
        let word_y = word.top + word.height / 2;
        if let Some(last_line) = lines.last_mut()
            && word_y.abs_diff(last_line.y_center) <= line_threshold
        {
            last_line.words.push(word);
            last_line.y_sum += word_y as u64;
            last_line.y_center = (last_line.y_sum / last_line.words.len() as u64) as u32;
            continue;
        }
        lines.push(TextLine {
            y_center: word_y,
            y_sum: word_y as u64,
            words: vec![word],
        });
    }
    for line in &mut lines {
        sort_ocr_line_word_refs(&mut line.words);
    }

    let paragraph_gap = avg_height * 2;

    struct Paragraph {
        y_start: u32,
        text: String,
    }

    let mut paragraphs: Vec<Paragraph> = Vec::new();
    for line in &lines {
        if let Some(last_para) = paragraphs.last_mut() {
            let last_y = last_para.y_start;
            if line.y_center.saturating_sub(last_y) <= paragraph_gap {
                last_para.text.push('\n');
                last_para.text.push_str(
                    &line
                        .words
                        .iter()
                        .map(|word| word.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                );
                last_para.y_start = line.y_center;
                continue;
            }
        }
        paragraphs.push(Paragraph {
            y_start: line.y_center,
            text: line
                .words
                .iter()
                .map(|word| word.text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
        });
    }

    enum ContentElement<'a> {
        Paragraph { y: u32, text: String },
        Table { y: u32, markdown: &'a str },
    }

    let mut elements: Vec<ContentElement> = Vec::new();

    {
        let mut para_idx = 0;
        let mut line_idx = 0;
        for para in &paragraphs {
            let line_count = para.text.matches('\n').count() + 1;
            let first_y = if line_idx < lines.len() {
                lines[line_idx].y_center
            } else {
                para.y_start
            };
            elements.push(ContentElement::Paragraph {
                y: first_y,
                text: para.text.clone(),
            });
            line_idx += line_count;
            para_idx += 1;
        }
        let _ = para_idx;
    }

    for table in tables {
        if let Some(ref bbox) = table.bounding_box {
            elements.push(ContentElement::Table {
                y: bbox.top,
                markdown: &table.markdown,
            });
        } else {
            elements.push(ContentElement::Table {
                y: u32::MAX,
                markdown: &table.markdown,
            });
        }
    }

    elements.sort_by_key(|e| match e {
        ContentElement::Paragraph { y, .. } => *y,
        ContentElement::Table { y, .. } => *y,
    });

    let mut output = String::new();
    for elem in &elements {
        let text = match elem {
            ContentElement::Paragraph { text, .. } => text.as_str(),
            ContentElement::Table { markdown, .. } => markdown,
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !output.is_empty() {
            output.push_str("\n\n");
        }
        output.push_str(trimmed);
    }

    output
}

/// Flatten hOCR-derived paragraph elements to unformatted text, joined by
/// blank lines (#185).
///
/// This used to also promote large-font, short, single-line paragraphs to
/// `## ` markdown headings using the average `x_fsize` captured per
/// paragraph by `hocr_parser::parse_hocr_to_internal_document`. That baked
/// markdown syntax directly into `OcrExtractionResult::content`, which is
/// consumed unrendered by callers that expect `Plain` output (e.g. the
/// `flat_ocr_page_document` fallback in `extractors/pdf/ocr.rs`), leaking
/// `## ` into supposedly-plain text. It also duplicated the document-global
/// structure pipeline, which classifies headings from the same `x_fsize`
/// attribute across the whole document and is strictly better informed than
/// a single page's local heuristic. That pipeline (driven by
/// `hocr_document`/`internal_document`, not this string) is now the sole
/// owner of heading detection for routes that have it; this function only
/// flattens text. The standalone-image path, which has no structure
/// pipeline behind it, keeps its own font-size-based heading promotion in
/// `extractors/image.rs`, expressed as real `Heading` elements rather than
/// pre-escaped markdown text.
fn flatten_hocr_elements_to_text(elements: &[crate::types::internal::InternalElement]) -> String {
    elements
        .iter()
        .filter(|e| !matches!(e.kind, ElementKind::PageBreak) && !e.text.is_empty())
        .map(|e| e.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Tesseract's call into [`drop_document_elements_claimed_by_tables`] (#1571). `hocr_document` is parsed from the raw
/// hOCR before table detection runs, and `build_content_with_inline_tables` repairs only the `content`
/// string (and is skipped for Plain/Djot output), so the element tree needs this regardless of
/// `output_format`. ~keep
fn filter_elements_covered_by_tables(document: &mut InternalDocument, tables: &[OcrTable]) {
    drop_document_elements_claimed_by_tables(
        document,
        tables.iter().filter_map(|table| {
            let bbox = table.bounding_box.as_ref()?;
            let bbox = crate::types::extraction::BoundingBox {
                x0: bbox.left as f64,
                y0: bbox.top as f64,
                x1: bbox.right as f64,
                y1: bbox.bottom as f64,
            };
            Some((bbox, table.cells.as_slice()))
        }),
    )
}

/// Minimum confidence for accepting orientation detection results.
///
/// Keep in sync with `doc_orientation::MIN_CONFIDENCE` (module is feature-gated,
/// this const is not): the PP-LCNet classifier reports correct 90°/270° rotations
/// on real documents at ~0.45, so a 0.5 cutoff rejects valid detections while
/// 0° false positives above 0.35 are rare.
const MIN_ORIENTATION_CONFIDENCE: f32 = 0.35;

#[cfg(auto_rotate)]
const _: () = assert!(MIN_ORIENTATION_CONFIDENCE == crate::doc_orientation::MIN_CONFIDENCE);

/// Source resolution assumed when OCR receives the original image unchanged and the caller did
/// not tell us what resolution it really is.
///
/// Only a fallback. `TesseractConfig::source_dpi`, when the caller knows the true value, takes
/// precedence — see [`prepare_ocr_image`].
const RAW_IMAGE_SOURCE_DPI: i32 = 72;
/// Source resolution retained when explicit DPI normalization fails.
const PREPROCESSING_FALLBACK_SOURCE_DPI: i32 = 300;
/// Pixel stride used by phase-sensitive classifier regression fixtures.
#[cfg(test)]
const DEFAULT_PREPROCESSING_SAMPLE_STRIDE: usize = 4;
/// Minimum normalized mean luminance for automatic document-page preprocessing.
const CLEAN_PAGE_MEAN_LUMINANCE_THRESHOLD: f64 = 0.90;
/// Normalized luminance at which a sampled pixel counts as near-white.
const CLEAN_PAGE_LIGHT_PIXEL_THRESHOLD: f64 = 0.90;
/// Minimum summed RGB channel value equivalent to the light-pixel threshold.
const CLEAN_PAGE_LIGHT_CHANNEL_SUM_THRESHOLD: u16 =
    (CLEAN_PAGE_LIGHT_PIXEL_THRESHOLD * RGB_CHANNEL_MAX * RGB_CHANNEL_COUNT as f64).ceil() as u16;
/// Minimum near-white sample fraction for automatic document-page preprocessing.
const CLEAN_PAGE_LIGHT_PIXEL_FRACTION_THRESHOLD: f64 = 0.80;
/// Minimum separation between background and foreground luminance modes. ~keep
const CLEAN_PAGE_MIN_FOREGROUND_CONTRAST: f64 = 0.20;
/// Minimum image fraction required before a lower foreground mode counts as significant. ~keep
const CLEAN_PAGE_DARK_PIXEL_FRACTION_THRESHOLD: f64 = 0.005;
/// Minimum population required for the lower foreground mode on small rasters. ~keep
const CLEAN_PAGE_DARK_PIXEL_COUNT_THRESHOLD: usize = 8;
const CLEAN_PAGE_TEXT_COMPONENT_MAX_FILL_NUMERATOR: usize = 3;
const CLEAN_PAGE_TEXT_COMPONENT_MAX_FILL_DENOMINATOR: usize = 4;
const CLEAN_PAGE_TEXT_COMPONENT_MIN_NARROW_ASPECT_RATIO: usize = 3;
/// Maximum value of an 8-bit RGB channel.
const RGB_CHANNEL_MAX: f64 = u8::MAX as f64;
/// Number of channels in an RGB pixel.
const RGB_CHANNEL_COUNT: usize = 3;

struct PreparedOcrImage {
    data: Vec<u8>,
    width: u32,
    height: u32,
    source_dpi: i32,
    apply_pix_preprocessing: bool,
    preprocessing: Option<crate::types::ImagePreprocessingConfig>,
    image_preprocessing: Option<crate::types::ImagePreprocessingMetadata>,
}

/// Prepare the raster Tesseract will recognize, and report the resolution it should be told.
///
/// `known_source_dpi` is the true resolution of `rgb_data` when the caller knows it (the PDF OCR
/// route derives it from the render, and [`resolve_known_source_dpi`] also reads it from PNG
/// density metadata), and `None` when it genuinely does not (raw images handed in by a user).
/// Both branches below honour it: the unpreprocessed branch reports it verbatim instead of the
/// [`RAW_IMAGE_SOURCE_DPI`] assumption, and the preprocessed branch feeds it to DPI normalization
/// so the resize scales from the real resolution.
///
/// `known_full_page_scan` is the PDF OCR route's own scan-detection density check result, when
/// the caller knows it (`TesseractConfig::known_full_page_scan`); `false` for every caller that
/// does not, including bare images handed in by a user. When `true`, the default preprocessing
/// path is taken unconditionally instead of deferring to [`should_apply_default_preprocessing`]'s
/// pixel-brightness heuristic -- a known scan page must not skip the default resample,
/// binarisation and deskew just because shaded table rows or a grey background read as dark
/// (GH#1894). The heuristic remains the only signal for a caller with no scan-detection result of
/// its own.
#[allow(clippy::too_many_arguments)]
fn prepare_ocr_image(
    rgb_data: Vec<u8>,
    width: u32,
    height: u32,
    preprocessing: Option<&crate::types::ImagePreprocessingConfig>,
    images_config: Option<&crate::core::config::ImageExtractionConfig>,
    ci_debug_enabled: bool,
    known_source_dpi: Option<f64>,
    known_full_page_scan: bool,
) -> PreparedOcrImage {
    let Some(preprocessing) = preprocessing else {
        if known_full_page_scan || should_apply_default_preprocessing(&rgb_data, width, height) {
            return prepare_preprocessed_ocr_image(
                rgb_data,
                width,
                height,
                &crate::types::ImagePreprocessingConfig::default(),
                images_config,
                ci_debug_enabled,
                known_source_dpi,
            );
        }
        return PreparedOcrImage {
            data: rgb_data,
            width,
            height,
            // The image is passed through untouched, so its resolution is whatever the caller
            // says it is; 72 remains the assumption only when nobody knows.
            source_dpi: known_source_dpi.map_or(RAW_IMAGE_SOURCE_DPI, |dpi| dpi.round() as i32),
            apply_pix_preprocessing: false,
            preprocessing: None,
            image_preprocessing: None,
        };
    };

    prepare_preprocessed_ocr_image(
        rgb_data,
        width,
        height,
        preprocessing,
        images_config,
        ci_debug_enabled,
        known_source_dpi,
    )
}

/// Classify bright, page-like RGB images that benefit from the default OCR preprocessing path.
fn should_apply_default_preprocessing(rgb_data: &[u8], width: u32, height: u32) -> bool {
    let Some(pixel_count) = (width as usize).checked_mul(height as usize) else {
        return false;
    };
    let Some(expected_len) = pixel_count.checked_mul(RGB_CHANNEL_COUNT) else {
        return false;
    };
    if pixel_count == 0 || rgb_data.len() != expected_len {
        return false;
    }

    let mut channel_sum = 0u128;
    let mut light_channel_sum = 0u128;
    let mut foreground_channel_sum = 0u128;
    let mut light_pixels = 0usize;
    let mut foreground_pixels = 0usize;
    let mut foreground_histogram = [0usize; 256];

    for pixel in rgb_data.as_chunks::<RGB_CHANNEL_COUNT>().0 {
        let pixel_channel_sum = u16::from(pixel[0]) + u16::from(pixel[1]) + u16::from(pixel[2]);
        channel_sum += u128::from(pixel_channel_sum);
        if pixel_channel_sum >= CLEAN_PAGE_LIGHT_CHANNEL_SUM_THRESHOLD {
            light_channel_sum += u128::from(pixel_channel_sum);
            light_pixels += 1;
        } else {
            foreground_channel_sum += u128::from(pixel_channel_sum);
            foreground_pixels += 1;
            foreground_histogram[((pixel_channel_sum + 1) / RGB_CHANNEL_COUNT as u16) as usize] += 1;
        }
    }
    let pixel_count_f64 = pixel_count as f64;
    let max_pixel_channel_sum = RGB_CHANNEL_MAX * RGB_CHANNEL_COUNT as f64;
    if channel_sum as f64 / (pixel_count_f64 * max_pixel_channel_sum) < CLEAN_PAGE_MEAN_LUMINANCE_THRESHOLD
        || light_pixels as f64 / pixel_count_f64 < CLEAN_PAGE_LIGHT_PIXEL_FRACTION_THRESHOLD
    {
        return false;
    }
    if foreground_pixels == 0 {
        return true;
    }

    let background_luminance = light_channel_sum as f64 / (light_pixels as f64 * max_pixel_channel_sum);
    let foreground_luminance = foreground_channel_sum as f64 / (foreground_pixels as f64 * max_pixel_channel_sum);
    if background_luminance - foreground_luminance >= CLEAN_PAGE_MIN_FOREGROUND_CONTRAST {
        return true;
    }

    let Some((lower_mode_max, lower_mode_pixels)) = foreground_lower_mode(&foreground_histogram) else {
        return false;
    };
    lower_mode_pixels >= CLEAN_PAGE_DARK_PIXEL_COUNT_THRESHOLD
        && lower_mode_pixels as f64 / pixel_count_f64 >= CLEAN_PAGE_DARK_PIXEL_FRACTION_THRESHOLD
        && lower_mode_has_text_structure(
            rgb_data,
            width as usize,
            height as usize,
            lower_mode_max,
            lower_mode_pixels,
        )
}

fn foreground_lower_mode(histogram: &[usize; 256]) -> Option<(u8, usize)> {
    let total_count: usize = histogram.iter().sum();
    let total_weight: u128 = histogram
        .iter()
        .enumerate()
        .map(|(value, count)| value as u128 * *count as u128)
        .sum();
    let mut lower_count = 0usize;
    let mut lower_weight = 0u128;
    let mut best: Option<(f64, u8, usize, f64, f64)> = None;

    for (threshold, &count) in histogram.iter().enumerate().take(u8::MAX as usize) {
        lower_count += count;
        lower_weight += threshold as u128 * count as u128;
        let upper_count = total_count - lower_count;
        if lower_count == 0 || upper_count == 0 {
            continue;
        }
        let lower_mean = lower_weight as f64 / lower_count as f64;
        let upper_mean = (total_weight - lower_weight) as f64 / upper_count as f64;
        let separation = upper_mean - lower_mean;
        let between_class_variance = lower_count as f64 * upper_count as f64 * separation * separation;
        if best
            .as_ref()
            .is_none_or(|(variance, ..)| between_class_variance > *variance)
        {
            best = Some((
                between_class_variance,
                threshold as u8,
                lower_count,
                lower_mean,
                upper_mean,
            ));
        }
    }

    let (_, threshold, lower_count, lower_mean, upper_mean) = best?;
    ((upper_mean - lower_mean) / RGB_CHANNEL_MAX >= CLEAN_PAGE_MIN_FOREGROUND_CONTRAST)
        .then_some((threshold, lower_count))
}

fn lower_mode_has_text_structure(
    rgb_data: &[u8],
    width: usize,
    height: usize,
    lower_mode_max: u8,
    lower_mode_pixels: usize,
) -> bool {
    let Some(pixel_count) = width.checked_mul(height) else {
        return false;
    };
    let lower_mode_mask: Vec<bool> = rgb_data
        .as_chunks::<RGB_CHANNEL_COUNT>()
        .0
        .iter()
        .map(|pixel| {
            let channel_sum = u16::from(pixel[0]) + u16::from(pixel[1]) + u16::from(pixel[2]);
            (channel_sum + 1) / RGB_CHANNEL_COUNT as u16 <= u16::from(lower_mode_max)
        })
        .collect();
    if lower_mode_mask.len() != pixel_count {
        return false;
    }

    let mut visited = vec![false; pixel_count];
    let mut component_stack = Vec::new();
    let mut structured_pixels = 0usize;
    for start in 0..pixel_count {
        if !lower_mode_mask[start] || visited[start] {
            continue;
        }
        let component_pixels = structured_component_size(
            &lower_mode_mask,
            &mut visited,
            &mut component_stack,
            start,
            width,
            height,
        );
        let Some(total) = structured_pixels.checked_add(component_pixels) else {
            return false;
        };
        structured_pixels = total;
    }
    structured_pixels >= lower_mode_pixels.div_ceil(2)
}

fn structured_component_size(
    mask: &[bool],
    visited: &mut [bool],
    stack: &mut Vec<usize>,
    start: usize,
    width: usize,
    height: usize,
) -> usize {
    stack.clear();
    stack.push(start);
    visited[start] = true;
    let (mut min_x, mut max_x) = (start % width, start % width);
    let (mut min_y, mut max_y) = (start / width, start / width);
    let mut size = 0usize;

    while let Some(index) = stack.pop() {
        let Some(next_size) = size.checked_add(1) else {
            return 0;
        };
        size = next_size;
        let x = index % width;
        let y = index / width;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
        let left = x.checked_sub(1).and_then(|_| index.checked_sub(1));
        let right = x
            .checked_add(1)
            .filter(|next_x| *next_x < width)
            .and_then(|_| index.checked_add(1));
        let above = y.checked_sub(1).and_then(|_| index.checked_sub(width));
        let below = y
            .checked_add(1)
            .filter(|next_y| *next_y < height)
            .and_then(|_| index.checked_add(width));
        let neighbors = [left, right, above, below];
        for neighbor in neighbors.into_iter().flatten() {
            if mask[neighbor] && !visited[neighbor] {
                visited[neighbor] = true;
                stack.push(neighbor);
            }
        }
    }

    if component_is_text_like(size, min_x, max_x, min_y, max_y) {
        size
    } else {
        0
    }
}

fn component_is_text_like(size: usize, min_x: usize, max_x: usize, min_y: usize, max_y: usize) -> bool {
    if min_x >= max_x || min_y >= max_y {
        return false;
    }
    let component_width = max_x - min_x + 1;
    let component_height = max_y - min_y + 1;
    let Some(area) = component_width.checked_mul(component_height) else {
        return false;
    };
    let Some(scaled_size) = size.checked_mul(CLEAN_PAGE_TEXT_COMPONENT_MAX_FILL_DENOMINATOR) else {
        return false;
    };
    let Some(maximum_fill) = area.checked_mul(CLEAN_PAGE_TEXT_COMPONENT_MAX_FILL_NUMERATOR) else {
        return false;
    };
    let short_side = component_width.min(component_height);
    let long_side = component_width.max(component_height);
    let is_narrow = short_side
        .checked_mul(CLEAN_PAGE_TEXT_COMPONENT_MIN_NARROW_ASPECT_RATIO)
        .is_some_and(|minimum_long_side| long_side >= minimum_long_side);
    scaled_size <= maximum_fill || is_narrow
}

fn prepare_preprocessed_ocr_image(
    rgb_data: Vec<u8>,
    width: u32,
    height: u32,
    preprocessing: &crate::types::ImagePreprocessingConfig,
    images_config: Option<&crate::core::config::ImageExtractionConfig>,
    ci_debug_enabled: bool,
    known_source_dpi: Option<f64>,
) -> PreparedOcrImage {
    // `target_dpi` always comes from the (Tesseract-specific) `preprocessing` config, which
    // takes precedence when explicitly set. The dimension/auto-adjust limits have no home in
    // `ImagePreprocessingConfig`, so they come from the real `ImageExtractionConfig` when the
    // caller has one, instead of being silently defaulted (issue #209).
    let dpi_config = match images_config {
        Some(images_config) => crate::types::ImageDpiConfig {
            target_dpi: preprocessing.target_dpi,
            ..crate::types::ImageDpiConfig::from(images_config)
        },
        None => crate::types::ImageDpiConfig {
            target_dpi: preprocessing.target_dpi,
            ..Default::default()
        },
    };
    // `known_source_dpi` is the whole point of this parameter: passing `None` here makes
    // `normalize_image_dpi_owned` assume 72 DPI, which on a page rendered at 150 inflates the
    // scale factor to `target_dpi / 72` and drives a Letter page into the `max_image_dimension`
    // clamp — 6x the pixels of the correct resize, carrying no more information, and reported to
    // Tesseract as roughly half the raster's real `scan_res`.
    match normalize_image_dpi_owned(rgb_data, width as usize, height as usize, &dpi_config, known_source_dpi) {
        Ok(result) => {
            let normalized_width = result.dimensions.0 as u32;
            let normalized_height = result.dimensions.1 as u32;
            let source_dpi = result.metadata.final_dpi;
            log_ci_debug(ci_debug_enabled, "dpi_normalization", || {
                format!(
                    "original={}x{} normalized={}x{} target_dpi={} final_dpi={} resized={}",
                    width,
                    height,
                    normalized_width,
                    normalized_height,
                    result.metadata.target_dpi,
                    source_dpi,
                    !result.metadata.skipped_resize
                )
            });
            PreparedOcrImage {
                data: result.rgb_data,
                width: normalized_width,
                height: normalized_height,
                source_dpi,
                apply_pix_preprocessing: true,
                preprocessing: Some(preprocessing.clone()),
                image_preprocessing: Some(result.metadata),
            }
        }
        Err((error, data)) => {
            tracing::warn!("DPI normalization failed, using original image: {}", error);
            PreparedOcrImage {
                data,
                width,
                height,
                // The image comes back unresized here, so its resolution is still the source
                // one. The 300 fallback is a guess for when there is nothing better.
                source_dpi: known_source_dpi.map_or(PREPROCESSING_FALLBACK_SOURCE_DPI, |dpi| dpi.round() as i32),
                apply_pix_preprocessing: true,
                preprocessing: Some(preprocessing.clone()),
                image_preprocessing: None,
            }
        }
    }
}

/// Check whether a center point (x, y) lies within a bounding box.
fn point_in_bbox(x: i32, y: i32, left: i32, top: i32, right: i32, bottom: i32) -> bool {
    x >= left && x <= right && y >= top && y <= bottom
}

/// Result of [`extract_elements_via_iterator`]: the extracted elements plus
/// counters distinguishing "nothing to extract" from "extraction degraded"
/// (#192, #180).
struct IteratorExtractionResult {
    elements: Vec<OcrElement>,
    retained_text_confidence_stats: Option<RetainedWordConfidenceStats>,
    /// Words the Tesseract result iterator itself failed to extract
    /// (null pointer / invalid parameter / invalid UTF-8 per word), per
    /// `ResultIterator::extract_all_words` (#192).
    skipped_words: usize,
    /// Words whose parent block is a non-flowing-text type (noise, an
    /// embedded image region, or a ruling line) that are now retained in
    /// `elements` instead of being silently dropped (#180).
    non_text_block_word_count: usize,
    /// Fraction of this page's dictionary-checkable words that Tesseract's own DAWG
    /// dictionary (`TesseractAPI::is_valid_word`) rejects as not-a-word. See
    /// [`dictionary_invalid_word_ratio`] for what counts as checkable and why `None`
    /// (rather than `0.0`) means "too few checkable words to be meaningful".
    dict_invalid_word_ratio: Option<f64>,
}

/// Minimum letters a word must have before Tesseract's dictionary lookup is meaningful.
/// Digits, single letters, and punctuation are never in the DAWG dictionary regardless of
/// legitimacy, so scoring them would bias the ratio toward "invalid" on output OCR read
/// perfectly correctly.
const MIN_WORD_LEN_FOR_DICT_CHECK: usize = 3;

/// Minimum number of dictionary-checkable words on a page before
/// [`dictionary_invalid_word_ratio`] reports a ratio at all, mirroring the same
/// conservatism as `OcrQualityThresholds::min_words_for_ocr_output_check`: a short page
/// (a signature block, an exhibit title) does not carry enough checkable words for the
/// ratio to mean anything.
const MIN_DICT_CANDIDATES_FOR_RATIO: usize = 5;

/// Fraction of a page's alphabetic, dictionary-checkable words that Tesseract's own
/// dictionary rejects as not-a-word.
///
/// This is the dictionary-validity supplement to the recognition-noise gate
/// (`is_ocr_recognition_noise` in `extractors::pdf::ocr`): a page of scanned line art
/// noise ("LAAALDLI", "AEA") is read by Tesseract with confident-looking output, so
/// per-word OCR confidence and the fragmented-word-ratio heuristic do not always catch
/// it — but few of its "words" are real dictionary words. `TesseractAPI::is_valid_word`
/// is Tesseract's own DAWG dictionary lookup, and is only usable while the `TesseractAPI`
/// handle that performed this page's OCR is still open (the dictionary lives inside that
/// engine instance) — callers must call this before the API is dropped.
///
/// Returns `None`, never `0.0`, when there are too few dictionary-checkable words to make
/// a ratio meaningful: a `0.0` here would read as "every word is valid" to a caller that
/// cannot tell "no evidence" from "good evidence", and this signal is a veto input, so
/// that conflation would suppress real content.
fn dictionary_invalid_word_ratio(api: &TesseractAPI, words: &[xberg_tesseract::WordData]) -> Option<f64> {
    invalid_word_ratio_from(words, |text| match api.is_valid_word(text) {
        Ok(0) => Some(false),
        Ok(_) => Some(true),
        // Lookup failed (e.g. interior NUL byte); don't let it bias the ratio either way.
        Err(_) => None,
    })
}

/// Pure core of [`dictionary_invalid_word_ratio`], taking dictionary lookup as an injected
/// function (`Some(true)` valid, `Some(false)` invalid, `None` lookup failed / skip) so it
/// is unit-testable without a live `TesseractAPI` handle.
fn invalid_word_ratio_from(
    words: &[xberg_tesseract::WordData],
    is_valid: impl Fn(&str) -> Option<bool>,
) -> Option<f64> {
    let mut candidates = 0usize;
    let mut invalid = 0usize;
    for word in words {
        let text = word.text.trim();
        if text.chars().count() < MIN_WORD_LEN_FOR_DICT_CHECK || !text.chars().all(|c| c.is_alphabetic()) {
            continue;
        }
        match is_valid(text) {
            Some(true) => candidates += 1,
            Some(false) => {
                candidates += 1;
                invalid += 1;
            }
            None => {}
        }
    }
    if candidates < MIN_DICT_CANDIDATES_FOR_RATIO {
        return None;
    }
    Some(invalid as f64 / candidates as f64)
}

/// Block types that historically caused a word to be dropped entirely from
/// `ocr_elements`. Words in these blocks are now retained (tagged with their
/// `block_type` via `iterator_word_to_element`) so text baked into figures,
/// chart axis labels, pull-out captions, and ruling-line regions is not lost
/// from OCR output (#180). Kept as a named set purely to identify and count
/// them for the `non_text_block_word_count` signal.
const NON_TEXT_BLOCK_TYPES: [TessPolyBlockType; 6] = [
    TessPolyBlockType::PT_NOISE,
    TessPolyBlockType::PT_FLOWING_IMAGE,
    TessPolyBlockType::PT_HEADING_IMAGE,
    TessPolyBlockType::PT_PULLOUT_IMAGE,
    TessPolyBlockType::PT_HORZ_LINE,
    TessPolyBlockType::PT_VERT_LINE,
];

/// Whether `auto_rotate` was requested by the caller but orientation detection
/// could not run because the `auto-rotate` build feature is not compiled in.
///
/// Extracted as a pure function, separate from the surrounding OCR pipeline and
/// its FFI calls, so the request-vs-availability logic is unit-testable without
/// a live Tesseract API instance (#309).
fn is_auto_rotate_requested_but_unavailable(auto_rotate_enabled: bool) -> bool {
    #[cfg(auto_rotate)]
    {
        let _ = auto_rotate_enabled;
        false
    }
    #[cfg(not(auto_rotate))]
    {
        auto_rotate_enabled
    }
}

/// Inserts the `word_iterator_skipped_count` metadata key only when at least one
/// word was actually skipped by the Tesseract result iterator, so callers can use
/// the key's absence (rather than a `0` value) as the "clean extraction" signal.
///
/// Extracted as its own function, separate from the surrounding OCR pipeline, so
/// the presence/absence and exact-value behaviour is unit-testable without a live
/// Tesseract API instance (#192).
fn insert_word_iterator_skipped_count_metadata(
    metadata: &mut HashMap<String, serde_json::Value>,
    skipped_words: usize,
) {
    if skipped_words > 0 {
        metadata.insert(
            "word_iterator_skipped_count".to_string(),
            serde_json::Value::Number(skipped_words.into()),
        );
    }
}

fn insert_retained_word_confidence_metadata(
    metadata: &mut HashMap<String, serde_json::Value>,
    stats: &RetainedWordConfidenceStats,
) {
    for key in [
        "word_count",
        "low_conf_word_count",
        "mean_text_conf",
        "median_word_conf",
        "p10_word_conf",
    ] {
        metadata.remove(key);
    }
    metadata.insert(
        "word_count".to_string(),
        serde_json::Value::Number(stats.word_count().into()),
    );
    metadata.insert(
        "low_conf_word_count".to_string(),
        serde_json::Value::Number(stats.low_confidence_word_count().into()),
    );
    if let Some(mean) = stats.mean() {
        metadata.insert("mean_text_conf".to_string(), serde_json::Value::Number(mean.into()));
    }
    if let Some(median) = stats.median() {
        metadata.insert("median_word_conf".to_string(), serde_json::Value::Number(median.into()));
    }
    if let Some(p10) = stats.p10() {
        metadata.insert("p10_word_conf".to_string(), serde_json::Value::Number(p10.into()));
    }
}

fn retained_text_word_confidence_stats(
    content: &str,
    words: &[xberg_tesseract::WordData],
) -> RetainedWordConfidenceStats {
    let retained_tokens = content.split_whitespace().collect::<Vec<_>>();
    let mut retained_index = 0usize;
    let mut stats = RetainedWordConfidenceStats::default();
    for word in words {
        let cleaned = strip_control_characters(&word.text);
        let word_tokens = cleaned.split_whitespace().collect::<Vec<_>>();
        if word_tokens.is_empty() {
            continue;
        }
        let Some(next_retained_index) = find_token_sequence_end(&retained_tokens, retained_index, &word_tokens) else {
            break;
        };
        stats.record(f64::from(word.confidence));
        retained_index = next_retained_index;
    }
    stats
}

fn find_token_sequence_end(haystack: &[&str], start: usize, needle: &[&str]) -> Option<usize> {
    if needle.is_empty() {
        return Some(start);
    }

    let mut prefix_lengths = vec![0usize; needle.len()];
    for index in 1..needle.len() {
        let mut matched = prefix_lengths[index - 1];
        while matched > 0 && needle[index] != needle[matched] {
            matched = prefix_lengths[matched - 1];
        }
        if needle[index] == needle[matched] {
            matched += 1;
        }
        prefix_lengths[index] = matched;
    }

    let mut matched = 0usize;
    for (offset, token) in haystack[start..].iter().enumerate() {
        while matched > 0 && *token != needle[matched] {
            matched = prefix_lengths[matched - 1];
        }
        if *token == needle[matched] {
            matched += 1;
            if matched == needle.len() {
                return Some(start + offset + 1);
            }
        }
    }
    None
}

/// Extract OcrElements via Tesseract's iterator APIs with rich metadata.
///
/// Uses ResultIterator for word-level text, bounding boxes, confidence, and font
/// attributes, plus PageIterator for block type and paragraph info. This replaces
/// TSV-based extraction with significantly richer metadata.
fn extract_elements_via_iterator(
    api: &TesseractAPI,
    page_number: u32,
    min_confidence: f64,
    retained_text: Option<&str>,
) -> Result<IteratorExtractionResult, OcrError> {
    let empty = || IteratorExtractionResult {
        elements: Vec::new(),
        retained_text_confidence_stats: None,
        skipped_words: 0,
        non_text_block_word_count: 0,
        dict_invalid_word_ratio: None,
    };

    let page_iter = match api.get_page_iterator() {
        Ok(iter) => iter,
        Err(e) => {
            tracing::warn!(error = %e, "Tesseract page iterator unavailable; falling back to TSV-based OCR element extraction");
            return Ok(empty());
        }
    };

    let blocks = match page_iter.extract_all_blocks() {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to extract Tesseract page blocks; falling back to TSV-based OCR element extraction");
            return Ok(empty());
        }
    };

    let paragraph_extraction = match page_iter.extract_all_paragraphs() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to extract Tesseract page paragraphs; falling back to TSV-based OCR element extraction");
            return Ok(empty());
        }
    };

    if paragraph_extraction.skipped_no_para_info > 0 || paragraph_extraction.skipped_no_bbox > 0 {
        tracing::warn!(
            skipped_no_para_info = paragraph_extraction.skipped_no_para_info,
            skipped_no_bbox = paragraph_extraction.skipped_no_bbox,
            extracted_paragraphs = paragraph_extraction.paragraphs.len(),
            "Tesseract declined to describe some paragraphs; their words lose the is_crown, \
             is_list_item and justification metadata"
        );
    }

    let paragraphs = paragraph_extraction.paragraphs;

    let result_iter = match api.get_iterator() {
        Ok(iter) => iter,
        Err(e) => {
            tracing::warn!(error = %e, "Tesseract result iterator unavailable; falling back to TSV-based OCR element extraction");
            return Ok(empty());
        }
    };

    let (word_extraction, line_starts) = match result_iter.extract_all_words_with_line_starts() {
        Ok(words) => words,
        Err(e) => {
            tracing::warn!(error = %e, "Tesseract result iterator failed; falling back to TSV-based OCR element extraction");
            return Ok(empty());
        }
    };

    if word_extraction.skipped > 0 {
        tracing::warn!(
            skipped_words = word_extraction.skipped,
            extracted_words = word_extraction.words.len(),
            "Tesseract result iterator failed to extract some words (null pointer, invalid \
             parameter, or invalid UTF-8); they were dropped from OCR output"
        );
    }

    let dict_invalid_word_ratio = dictionary_invalid_word_ratio(api, &word_extraction.words);
    let retained_text_confidence_stats =
        retained_text.map(|content| retained_text_word_confidence_stats(content, &word_extraction.words));

    let mut words = Vec::new();
    let mut non_text_block_word_count = 0usize;
    let mut line_index = 0usize;

    for (word, &starts_line) in word_extraction.words.iter().zip(&line_starts) {
        line_index += usize::from(starts_line);
        if (word.confidence as f64) < min_confidence {
            continue;
        }

        if word.text.trim().is_empty() {
            continue;
        }

        let cx = (word.left + word.right) / 2;
        let cy = (word.top + word.bottom) / 2;

        let parent_block = blocks
            .iter()
            .find(|b| point_in_bbox(cx, cy, b.left, b.top, b.right, b.bottom));

        let block_type = parent_block.map(|b| b.block_type);

        if let Some(bt) = block_type
            && NON_TEXT_BLOCK_TYPES.contains(&bt)
        {
            non_text_block_word_count += 1;
        }

        let para_info = paragraphs
            .iter()
            .find(|p| point_in_bbox(cx, cy, p.left, p.top, p.right, p.bottom));

        let element = iterator_word_to_element(word, block_type, para_info, page_number);
        words.push((line_index, element));
    }

    Ok(IteratorExtractionResult {
        elements: tesseract_elements_with_lines(words),
        retained_text_confidence_stats,
        skipped_words: word_extraction.skipped,
        non_text_block_word_count,
        dict_invalid_word_ratio,
    })
}

/// Resolve the `SecurityLimits` to apply when decoding an image for OCR.
///
/// `TesseractConfig::security_limits` wins because it is the only channel that survives
/// the `OcrBackend` trait boundary: a backend receives `&OcrConfig` and no
/// `ExtractionConfig`, so on every real backend route the `extraction_config` argument is
/// a synthetic value built here to carry `output_format` and nothing else (GH#1651). The
/// `extraction_config` fallback is retained for the direct/in-process call sites that do
/// pass a real one.
///
/// `None` on both means no configured limit reached this call, not that limits should be
/// waived — this falls back to the same default a configured caller gets when they never
/// set `security_limits` explicitly (GH#1554: `load_image_for_ocr` previously hardcoded
/// this default unconditionally, ignoring a caller's own configured, possibly higher,
/// limit). ~keep
/// A region's grid with the two cleanups that every reader of it depends on: the GH#1649
/// leading section-caption row dropped, then disjoint right-aligned numeric columns merged.
///
/// Extracted because the blank-quantity retry path rebuilt the grid with a bare
/// `reconstruct_table_with_columns` and silently discarded both, so any table that hit the
/// retry got its caption row back in row 0 and its amount columns un-merged -- and left
/// `column_positions` describing a grid that no longer existed. Both call sites must run all
/// three steps or neither. ~keep
fn reconstruct_cleaned_table(words: &[HocrWord], config: &TesseractConfig) -> (Vec<Vec<String>>, Vec<u32>) {
    let (mut table, mut column_positions) =
        reconstruct_table_with_columns(words, config.table_column_threshold, config.table_row_threshold_ratio);
    drop_leading_caption_row(&mut table);
    merge_disjoint_numeric_columns(&mut table, &mut column_positions, median_word_height(words));
    (table, column_positions)
}

#[cfg(test)]
fn sort_ocr_line_words(words: &mut [HocrWord]) {
    let rtl_line = hocr_words_are_rtl(words.iter());
    words.sort_by(|a, b| {
        if rtl_line {
            b.left.cmp(&a.left)
        } else {
            a.left.cmp(&b.left)
        }
    });
}

fn sort_ocr_line_word_refs(words: &mut [&HocrWord]) {
    let rtl_line = hocr_words_are_rtl(words.iter().copied());
    words.sort_by(|a, b| {
        if rtl_line {
            b.left.cmp(&a.left)
        } else {
            a.left.cmp(&b.left)
        }
    });
}

fn security_limits_for_ocr(
    tesseract_config: &TesseractConfig,
    extraction_config: Option<&ExtractionConfig>,
) -> SecurityLimits {
    tesseract_config
        .security_limits
        .clone()
        .or_else(|| extraction_config.and_then(|config| config.security_limits.clone()))
        .unwrap_or_default()
}

/// The symbols of every word with an underscore, which table reconstruction reads to cut a mark
/// out of a word. Empty when Tesseract gives no result iterator or cannot read the symbols: the
/// cut then estimates each piece's box from its share of the word's characters. ~keep
fn underscore_word_symbols(api: &TesseractAPI) -> Vec<xberg_tesseract::WordSymbols> {
    match api
        .get_iterator()
        .and_then(|iterator| iterator.extract_word_symbols(|text| text.contains('_')))
    {
        Ok(words) => words,
        Err(error) => {
            tracing::debug!(
                target: "xberg::ocr::tables",
                %error,
                "Tesseract symbol boxes unavailable; underscore marks are cut by character share"
            );
            Vec::new()
        }
    }
}

/// Perform OCR on an image using Tesseract.
///
/// This function handles the complete OCR pipeline:
/// 1. Image loading and preprocessing
/// 2. Tesseract initialization and configuration
/// 3. Text recognition
/// 4. Output formatting (text, markdown, hOCR, or TSV)
/// 5. Optional table detection
///
/// # Arguments
///
/// * `image_bytes` - Raw image data
/// * `config` - OCR configuration
/// * `extraction_config` - Optional extraction config for output format (markdown vs djot)
///
/// # Returns
///
/// OCR extraction result containing text and optional tables
pub(super) fn perform_ocr(
    image_bytes: &[u8],
    config: &TesseractConfig,
    api_pool: &Arc<TesseractApiPool>,
    extraction_config: Option<&ExtractionConfig>,
) -> Result<OcrExtractionResult, OcrError> {
    let ci_debug_enabled = env::var_os("XBERG_CI_DEBUG").is_some();
    log_ci_debug(ci_debug_enabled, "perform_ocr:start", || {
        format!(
            "bytes={} language={} output={} use_cache={}",
            image_bytes.len(),
            config.language,
            config.output_format,
            config.use_cache
        )
    });

    let security_limits = security_limits_for_ocr(config, extraction_config);
    let rgb_image = {
        let img = crate::extraction::image::load_image_for_ocr(image_bytes, &security_limits)
            .map_err(|e| OcrError::ImageProcessingFailed(e.to_string()))?;
        img.into_rgb8()
    };
    let (orig_width, orig_height) = rgb_image.dimensions();
    let rgb_data = rgb_image.into_raw();

    log_ci_debug(ci_debug_enabled, "image", || {
        format!("dimensions={}x{} color_type=RGB8", orig_width, orig_height)
    });

    let images_config = extraction_config.and_then(|extraction_config| extraction_config.images.as_ref());
    let known_source_dpi = resolve_known_source_dpi(config.source_dpi, image_bytes, orig_width, orig_height);
    let prepared_image = prepare_ocr_image(
        rgb_data,
        orig_width,
        orig_height,
        config.preprocessing.as_ref(),
        images_config,
        ci_debug_enabled,
        known_source_dpi,
        config.known_full_page_scan,
    );
    #[cfg_attr(not(auto_rotate), allow(unused_mut))]
    let mut image_data = prepared_image.data;
    #[cfg_attr(not(auto_rotate), allow(unused_mut))]
    let mut width = prepared_image.width;
    #[cfg_attr(not(auto_rotate), allow(unused_mut))]
    let mut height = prepared_image.height;
    let source_dpi = prepared_image.source_dpi;
    let preprocessing = prepared_image.preprocessing;
    let image_preprocessing = prepared_image.image_preprocessing;
    #[cfg_attr(not(auto_rotate), allow(unused_mut))]
    let mut ocr_image_width = width;
    #[cfg_attr(not(auto_rotate), allow(unused_mut))]
    let mut ocr_image_height = height;

    let bytes_per_pixel: u32 = 3;

    let languages: Vec<String> = config.language.split('+').map(|lang| lang.trim().to_string()).collect();
    let tessdata_path = resolve_tessdata_path(&languages, config.tessdata_path.as_deref())?;

    log_ci_debug(ci_debug_enabled, "tessdata", || {
        let path_preview = env::var_os("PATH").map(|paths| {
            env::split_paths(&paths)
                .take(6)
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        });
        let resolved_exists = !tessdata_path.is_empty() && std::path::Path::new(&tessdata_path).exists();

        format!(
            "env={:?} resolved={} exists={} path_preview={:?}",
            env::var("TESSDATA_PREFIX").ok(),
            if tessdata_path.is_empty() {
                "unset"
            } else {
                &tessdata_path
            },
            resolved_exists,
            path_preview
        )
    });

    log_ci_debug(ci_debug_enabled, "tesseract_version", || {
        format!("version={}", TesseractAPI::version())
    });

    validate_language_and_traineddata(&config.language, &tessdata_path)?;

    let api = api_pool.checkout(&tessdata_path, &config.language)?;

    log_ci_debug(ci_debug_enabled, "init", || {
        format!("language={} datapath='{}'", config.language, tessdata_path)
    });

    if ci_debug_enabled {
        match api.get_available_languages() {
            Ok(languages) => {
                log_ci_debug(ci_debug_enabled, "available_languages", move || {
                    let preview = languages.iter().take(10).cloned().collect::<Vec<_>>();
                    format!("count={} preview={:?}", languages.len(), preview)
                });
            }
            Err(err) => {
                log_ci_debug(ci_debug_enabled, "available_languages_error", move || {
                    format!("error={:?}", err)
                });
            }
        }
    }

    let psm_mode = TessPageSegMode::from_int(config.psm as i32);
    let psm_result = api.set_page_seg_mode(psm_mode);
    log_ci_debug(ci_debug_enabled, "set_psm", || match &psm_result {
        Ok(_) => format!("mode={}", config.psm),
        Err(err) => format!("error={:?}", err),
    });
    psm_result.map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set PSM mode: {}", e)))?;

    apply_tesseract_variables(&api, config)?;
    let source_dpi = source_dpi.max(70);

    // Only (degrees, confidence): the ONNX PP-LCNet orientation classifier used
    // here has no script-detection capability, unlike Tesseract's own
    // `DetectOrientationScript()`/OSD, which this crate does not call (#184).
    #[cfg_attr(not(auto_rotate), allow(unused_mut))]
    let mut detected_orientation: Option<(i32, f32)> = None;
    let auto_rotate_enabled =
        config.preprocessing.as_ref().map(|p| p.auto_rotate).unwrap_or(false) || config.auto_rotate;

    // Recorded into `metadata` below (once `metadata` exists) under the
    // `auto_rotate_unavailable` key, mirroring `pre_formatted` and
    // `word_iterator_skipped_count`: `OcrExtractionResult` has no dedicated
    // warnings field, so `TesseractBackend` reads this key back out and turns
    // it into a `ProcessingWarning` the caller actually sees (#309). Without
    // it, a user's explicit `auto_rotate = true` silently does nothing on a
    // build without the `auto-rotate` feature.
    let auto_rotate_unavailable = is_auto_rotate_requested_but_unavailable(auto_rotate_enabled);

    #[cfg(not(auto_rotate))]
    if auto_rotate_enabled {
        tracing::warn!(
            "auto_rotate requested but the `auto-rotate` feature is not compiled in; skipping orientation detection"
        );
    }

    #[cfg(auto_rotate)]
    if auto_rotate_enabled {
        let expected_rgb_len = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixels| pixels.checked_mul(bytes_per_pixel as usize));
        let orientation_result = if expected_rgb_len != Some(image_data.len()) {
            Err(crate::error::XbergError::Ocr {
                message: "auto_rotate: image buffer does not match dimensions".to_string(),
                source: None,
            })
        } else {
            let image = image::RgbImage::from_raw(width, height, std::mem::take(&mut image_data))
                .expect("validated RGB buffer length must match dimensions");
            let result = doc_orientation_detector().detect(&image);
            image_data = image.into_raw();
            result
        };

        match orientation_result {
            Err(e) => {
                tracing::warn!("Orientation detection failed, proceeding without rotation: {}", e);
            }
            Ok(orientation) => {
                let orient_deg = orientation.degrees as i32;
                let orient_conf = orientation.confidence;
                log_ci_debug(ci_debug_enabled, "orientation_detection", || {
                    format!("orientation={}° confidence={:.2}", orient_deg, orient_conf)
                });
                detected_orientation = Some((orient_deg, orient_conf));

                if orient_deg != 0 && orient_conf > MIN_ORIENTATION_CONFIDENCE {
                    tracing::info!(
                        "Auto-rotating image by {} degrees (confidence: {:.2})",
                        orient_deg,
                        orient_conf
                    );

                    let correction_deg = (360 - orient_deg).rem_euclid(360);
                    let (rotated_data, new_width, new_height) =
                        rotate_rgb_image_data(&image_data, width, height, correction_deg);
                    image_data = rotated_data;
                    width = new_width;
                    height = new_height;
                    ocr_image_width = new_width;
                    ocr_image_height = new_height;

                    log_ci_debug(ci_debug_enabled, "auto_rotate", || {
                        format!("rotated={}° new_dimensions={}x{}", orient_deg, new_width, new_height)
                    });
                } else {
                    tracing::debug!(
                        degrees = orient_deg,
                        confidence = orient_conf,
                        threshold = MIN_ORIENTATION_CONFIDENCE,
                        "auto_rotate: keeping original orientation"
                    );
                }
            }
        }
    }

    let processed_pix: Option<xberg_tesseract::Pix> =
        if let (true, Some(preprocessing)) = (prepared_image.apply_pix_preprocessing, preprocessing.as_ref()) {
            match xberg_tesseract::Pix::from_raw_rgb(&image_data, width, height) {
                Ok(pix) => match preprocess_pix(pix, preprocessing) {
                    Ok(processed) => Some(processed),
                    Err(error) => {
                        tracing::debug!(%error, "Leptonica preprocessing failed; using raw image");
                        None
                    }
                },
                Err(error) => {
                    tracing::debug!(%error, "Leptonica Pix creation failed; using raw image");
                    None
                }
            }
        } else {
            None
        };

    if let Some(ref pix) = processed_pix {
        api.set_image_2(pix.as_ptr())
            .map_err(|error| OcrError::ProcessingFailed(format!("Failed to set preprocessed image: {error}")))?;
    } else {
        let bytes_per_line = width * bytes_per_pixel;
        api.set_image(
            &image_data,
            width as i32,
            height as i32,
            bytes_per_pixel as i32,
            bytes_per_line as i32,
        )
        .map_err(|error| OcrError::ProcessingFailed(format!("Failed to set image: {error}")))?;
    }
    drop(processed_pix);

    api.set_source_resolution(source_dpi)
        .map_err(|error| OcrError::ProcessingFailed(format!("Failed to set source resolution: {error}")))?;

    log_ci_debug(ci_debug_enabled, "set_image", || {
        format!(
            "width={} height={} bytes_per_pixel={} source_dpi={}",
            width, height, bytes_per_pixel, source_dpi
        )
    });

    drop(image_data);

    api.recognize()
        .map_err(|e| OcrError::ProcessingFailed(format!("Failed to recognize text: {}", e)))?;

    let mean_text_conf = api.mean_text_conf().unwrap_or(-1);

    log_ci_debug(ci_debug_enabled, "recognize", || {
        format!("completed mean_text_conf={}", mean_text_conf)
    });

    let word_confidence_stats = if matches!(config.output_format.as_str(), "markdown" | "text") {
        None
    } else {
        match api.all_word_confidences() {
            Ok(confidences) if !confidences.is_empty() => {
                let word_count = confidences.len();
                let low_conf_word_count = confidences.iter().filter(|&&c| c < 50).count();

                let mut sorted = confidences.clone();
                sorted.sort_unstable();
                let median_word_conf = if word_count % 2 == 0 {
                    (sorted[word_count / 2 - 1] + sorted[word_count / 2]) / 2
                } else {
                    sorted[word_count / 2]
                };

                let p10_idx = ((word_count as f64 - 1.0) * 0.1).floor() as usize;
                let p10_word_conf = sorted[p10_idx.min(word_count - 1)];

                Some((median_word_conf, p10_word_conf, word_count, low_conf_word_count))
            }
            Ok(_) => match api.mean_text_conf() {
                Ok(mean_conf) => Some((mean_conf, mean_conf, 0usize, 0usize)),
                Err(_) => None,
            },
            Err(_) => match api.mean_text_conf() {
                Ok(mean_conf) => Some((mean_conf, mean_conf, 0usize, 0usize)),
                Err(_) => None,
            },
        }
    };

    let tsv_data_for_tables = if config.enable_table_detection || config.output_format == "tsv" {
        Some(
            api.get_tsv_text(0)
                .map_err(|e| OcrError::ProcessingFailed(format!("Failed to extract TSV: {}", e)))?,
        )
    } else {
        None
    };
    let table_mark_symbols = match tsv_data_for_tables.as_deref() {
        Some(tsv) if config.enable_table_detection && tsv.contains('_') => underscore_word_symbols(&api),
        _ => Vec::new(),
    };

    let mut hocr_document: Option<InternalDocument> = None;
    let mut dictionary_filtered_line_count = 0usize;
    let mut retained_hocr_confidence_stats = None;

    let (raw_content, mime_type) = match config.output_format.as_str() {
        "text" => {
            let text = api
                .get_utf8_text()
                .map_err(|e| OcrError::ProcessingFailed(format!("Failed to extract text: {}", e)))?;
            (text, "text/plain".to_string())
        }
        "markdown" => {
            let hocr = api
                .get_hocr_text(0)
                .map_err(|e| OcrError::ProcessingFailed(format!("Failed to extract hOCR: {}", e)))?;

            // Per-line dictionary-invalid noise filter (#783). Built here, immediately
            // before parsing, using `hocr_parser::DEFAULT_DICT_INVALID_LINE_RATIO` -- see
            // that constant's doc comment for why this is not yet a configurable
            // `OcrQualityThresholds` field.
            let is_valid_word = |text: &str| match api.is_valid_word(text) {
                Ok(0) => Some(false),
                Ok(_) => Some(true),
                Err(_) => None,
            };
            let dictionary_filter = DictionaryLineFilter {
                is_valid_word: &is_valid_word,
                max_invalid_ratio: crate::ocr::hocr_parser::DEFAULT_DICT_INVALID_LINE_RATIO,
            };

            let parse_result = parse_hocr_to_internal_document_with_page_offset_and_stats(
                &hocr,
                Some(&dictionary_filter),
                config.page_number,
            );
            dictionary_filtered_line_count = parse_result.dictionary_filtered_line_count;
            let content = flatten_hocr_elements_to_text(&parse_result.document.elements);
            retained_hocr_confidence_stats = Some(parse_result.retained_word_confidence_stats);
            hocr_document = Some(parse_result.document);

            let mime_type = extraction_config
                .map(|c| match c.output_format {
                    crate::core::config::OutputFormat::Djot => "text/djot",
                    _ => "text/markdown",
                })
                .unwrap_or("text/markdown");

            (content, mime_type.to_string())
        }
        "hocr" => {
            let hocr = api
                .get_hocr_text(0)
                .map_err(|e| OcrError::ProcessingFailed(format!("Failed to extract hOCR: {}", e)))?;
            (hocr, "text/html".to_string())
        }
        "tsv" => {
            let tsv = tsv_data_for_tables
                .as_ref()
                .ok_or_else(|| OcrError::ProcessingFailed("TSV data not available".to_string()))?
                .clone();
            (tsv, "text/plain".to_string())
        }
        _ => {
            return Err(OcrError::InvalidConfiguration(format!(
                "Unsupported output format: {}",
                config.output_format
            )));
        }
    };

    let mut metadata = HashMap::new();
    if let Some(image_preprocessing) = image_preprocessing {
        let value = serde_json::to_value(image_preprocessing).map_err(|error| {
            OcrError::ProcessingFailed(format!("Failed to serialize image preprocessing metadata: {error}"))
        })?;
        metadata.insert(
            crate::ocr_metadata_keys::OCR_IMAGE_PREPROCESSING_METADATA_KEY.to_string(),
            value,
        );
    }
    metadata.insert(
        crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.to_string(),
        serde_json::Value::Number(ocr_image_width.into()),
    );
    metadata.insert(
        crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.to_string(),
        serde_json::Value::Number(ocr_image_height.into()),
    );
    metadata.insert(
        "language".to_string(),
        serde_json::Value::String(config.language.clone()),
    );
    metadata.insert("psm".to_string(), serde_json::Value::String(config.psm.to_string()));
    metadata.insert("table_count".to_string(), serde_json::Value::Number(0.into()));
    metadata.insert("tables_detected".to_string(), serde_json::Value::Number(0.into()));
    if config.output_format == "markdown" {
        metadata.insert(
            "source_format".to_string(),
            serde_json::Value::String("hocr".to_string()),
        );
    }
    if auto_rotate_unavailable {
        metadata.insert("auto_rotate_unavailable".to_string(), serde_json::Value::Bool(true));
    }
    if dictionary_filtered_line_count > 0 {
        metadata.insert(
            "dictionary_filtered_line_count".to_string(),
            serde_json::Value::Number(dictionary_filtered_line_count.into()),
        );
    }

    if let Some(stats) = retained_hocr_confidence_stats.as_ref() {
        insert_retained_word_confidence_metadata(&mut metadata, stats);
    } else if config.output_format != "text" {
        if mean_text_conf >= 0 {
            metadata.insert(
                "mean_text_conf".to_string(),
                serde_json::Value::Number(serde_json::Number::from(mean_text_conf)),
            );
        }

        if let Some((median_conf, p10_conf, word_count, low_conf_count)) = word_confidence_stats {
            metadata.insert(
                "median_word_conf".to_string(),
                serde_json::Value::Number(serde_json::Number::from(median_conf)),
            );
            metadata.insert(
                "p10_word_conf".to_string(),
                serde_json::Value::Number(serde_json::Number::from(p10_conf)),
            );
            metadata.insert(
                "word_count".to_string(),
                serde_json::Value::Number(serde_json::Number::from(word_count)),
            );
            metadata.insert(
                "low_conf_word_count".to_string(),
                serde_json::Value::Number(serde_json::Number::from(low_conf_count)),
            );
        }
    }

    // No `script_name`/`script_confidence` metadata: the ONNX PP-LCNet
    // orientation classifier used here has no script-detection capability,
    // unlike Tesseract's own `DetectOrientationScript()`/OSD (which this crate
    // does not call). Emitting a placeholder script name would misrepresent
    // real detection as having happened (#184). ~keep
    if let Some((orient_deg, orient_conf)) = detected_orientation {
        metadata.insert(
            crate::ocr_metadata_keys::OCR_ORIENTATION_DEGREES_METADATA_KEY.to_string(),
            serde_json::Value::Number(serde_json::Number::from(orient_deg)),
        );
        metadata.insert(
            crate::ocr_metadata_keys::OCR_ORIENTATION_CONFIDENCE_METADATA_KEY.to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(orient_conf as f64).unwrap_or(serde_json::Number::from(0)),
            ),
        );
        if orient_deg != 0 && orient_conf > MIN_ORIENTATION_CONFIDENCE {
            metadata.insert(
                crate::ocr_metadata_keys::OCR_AUTO_ROTATED_METADATA_KEY.to_string(),
                serde_json::Value::Bool(true),
            );
        }
    }

    let mut content = strip_control_characters(&raw_content).into_owned();
    let retained_text = (config.output_format == "text").then_some(content.as_str());
    let iterator_extraction =
        extract_elements_via_iterator(&api, config.page_number, config.min_confidence, retained_text);
    let mut ocr_elements = None;
    if let Ok(extraction) = &iterator_extraction
        && let Some(stats) = extraction.retained_text_confidence_stats.as_ref()
    {
        insert_retained_word_confidence_metadata(&mut metadata, stats);
    }
    match iterator_extraction {
        Ok(extraction) if !extraction.elements.is_empty() => {
            insert_word_iterator_skipped_count_metadata(&mut metadata, extraction.skipped_words);
            if extraction.non_text_block_word_count > 0 {
                metadata.insert(
                    "non_text_block_word_count".to_string(),
                    serde_json::Value::Number(extraction.non_text_block_word_count.into()),
                );
            }
            if let Some(ratio) = extraction.dict_invalid_word_ratio {
                metadata.insert(
                    crate::ocr_metadata_keys::OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY.to_string(),
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(ratio).unwrap_or(serde_json::Number::from(0)),
                    ),
                );
            }
            ocr_elements = Some(extraction.elements);
        }
        _ => {
            if let Some(ref tsv_data) = tsv_data_for_tables {
                let elements = parse_tsv_to_elements(tsv_data, config.min_confidence, config.page_number);
                if !elements.is_empty() {
                    ocr_elements = Some(elements);
                }
            }
        }
    }

    let mut tables = Vec::new();

    if config.enable_table_detection {
        let tsv_data = tsv_data_for_tables.as_ref().unwrap();

        let words = extract_table_words_from_tsv(tsv_data, config.table_min_confidence, &table_mark_symbols)?;

        for (region_index, mut region) in table_regions(&words).into_iter().enumerate() {
            if region.words.len() < MIN_TABLE_CANDIDATE_WORDS {
                tracing::debug!(
                    target: "xberg::ocr::tables",
                    region_index,
                    word_count = region.words.len(),
                    min_required = MIN_TABLE_CANDIDATE_WORDS,
                    "OCR table region skipped: below MIN_TABLE_CANDIDATE_WORDS"
                );
                continue;
            }
            // The minimum above counts every word Tesseract read, before shading marks are
            // dropped, so a dropped mark changes a table's cells but never whether a table is
            // attempted at all (GH#1858). ~keep
            region.drop_shading_marks(config.table_row_threshold_ratio);
            if config
                .preprocessing
                .as_ref()
                .is_some_and(|preprocessing| preprocessing.normalize_shaded_rows)
            {
                region.drop_normalized_band_edge_glyphs();
            }
            if !has_semantic_ocr_table_content(&region.words) {
                tracing::debug!(
                    target: "xberg::ocr::tables",
                    region_index,
                    word_count = region.words.len(),
                    "OCR table region skipped: insufficient semantic content"
                );
                continue;
            }

            let region_left = region.words.iter().map(|w| w.left).min().unwrap_or(0);
            let region_top = region.words.iter().map(|w| w.top).min().unwrap_or(0);
            let region_right = region.words.iter().map(|w| w.left + w.width).max().unwrap_or(0);
            let region_bottom = region.words.iter().map(|w| w.top + w.height).max().unwrap_or(0);
            let word_preview: String = region
                .words
                .iter()
                .take(12)
                .map(|w| w.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(200)
                .collect();

            let (mut table, mut column_positions) = reconstruct_cleaned_table(&region.words, config);
            let retry_region = if let Some((quantity_column, blank_row)) = quantity_retry_column_index(&table) {
                let row_positions = detect_rows(&region.words, config.table_row_threshold_ratio);
                if row_positions.len() == table.len() {
                    quantity_retry_region(
                        &region.words,
                        quantity_column,
                        blank_row,
                        &row_positions,
                        &column_positions,
                        width,
                        height,
                    )
                } else {
                    None
                }
            } else {
                None
            };
            let recovered =
                recover_blank_quantity_word(&api, config, retry_region.as_ref(), width, height).or_else(|| {
                    if !prepared_image.apply_pix_preprocessing {
                        return None;
                    }
                    let retry = retry_region.as_ref()?;
                    recover_blank_quantity_from_source(
                        image_bytes,
                        config,
                        retry,
                        width,
                        height,
                        &tessdata_path,
                        &security_limits,
                    )
                });
            if let Some(recovered) = recovered {
                region.push(recovered);
                (table, column_positions) = reconstruct_cleaned_table(&region.words, config);
            }
            orient_table_for_reading_order(&mut table, &mut column_positions, &region.words);
            tracing::debug!(
                target: "xberg::ocr::tables",
                region_index,
                word_count = region.words.len(),
                left = region_left,
                top = region_top,
                right = region_right,
                bottom = region_bottom,
                word_preview = %word_preview,
                raw_rows = table.len(),
                raw_cols = table.first().map_or(0, Vec::len),
                "OCR table region reconstructed"
            );

            if table.is_empty() || table[0].is_empty() {
                continue;
            }

            #[cfg(feature = "pdf")]
            let cleaned = post_process_table(table, false, false);
            #[cfg(not(feature = "pdf"))]
            let cleaned = Some(table);
            let Some(cleaned) = cleaned else {
                continue;
            };

            let markdown_table = table_to_markdown(&cleaned);

            tables.push(OcrTable {
                cells: cleaned,
                markdown: markdown_table,
                page_number: config.page_number,
                bounding_box: Some(region.bounding_box()),
            });
        }

        // Real count, not hardcoded to at-most-one (#177): a page can contain
        // several independent tables separated by prose.
        metadata.insert(
            "table_count".to_string(),
            serde_json::Value::Number(tables.len().into()),
        );
        metadata.insert(
            "tables_detected".to_string(),
            serde_json::Value::Number(tables.len().into()),
        );
        if let Some(first) = tables.first() {
            metadata.insert(
                "table_rows".to_string(),
                serde_json::Value::Number(first.cells.len().into()),
            );
            metadata.insert(
                "table_cols".to_string(),
                serde_json::Value::Number(first.cells.first().map_or(0, Vec::len).into()),
            );
        }
    }

    if let Some(document) = hocr_document.as_mut() {
        filter_elements_covered_by_tables(document, &tables);
    }

    let is_markdown_output = extraction_config
        .map(|c| c.output_format == crate::core::config::OutputFormat::Markdown)
        .unwrap_or(config.output_format == "markdown");

    if !tables.is_empty()
        && is_markdown_output
        && let Some(ref tsv_data) = tsv_data_for_tables
    {
        let rebuilt = build_content_with_inline_tables(tsv_data, &tables, config.table_min_confidence);
        if !rebuilt.is_empty() {
            if should_adopt_table_rebuild(&content, &rebuilt) {
                content = rebuilt;
                metadata.insert(
                    "pre_formatted".to_string(),
                    serde_json::Value::String("markdown".to_string()),
                );
            } else {
                tracing::warn!(
                    target: "xberg::ocr::tables",
                    original_word_count = content.split_whitespace().count(),
                    rebuilt_word_count = rebuilt.split_whitespace().count(),
                    "OCR markdown table rebuild dropped content relative to the original; keeping original content"
                );
            }
        }
    }

    drop(api);

    Ok(OcrExtractionResult {
        content,
        mime_type,
        metadata,
        tables,
        ocr_elements,
        internal_document: hocr_document,
    })
}

/// Process an image file and return OCR results.
///
/// # Arguments
///
/// * `file_path` - Path to image file
/// * `config` - OCR configuration
/// * `cache` - Cache instance
/// * `output_format` - Optional output format (Plain, Markdown, Djot) for proper mime_type handling
///
/// # Returns
///
/// OCR extraction result
pub(super) fn process_image_file_with_cache(
    file_path: &str,
    config: &TesseractConfig,
    cache: &OcrCache,
    api_pool: &Arc<TesseractApiPool>,
    output_format: Option<crate::core::config::OutputFormat>,
) -> Result<OcrExtractionResult, OcrError> {
    let image_bytes = std::fs::read(file_path)
        .map_err(|e| OcrError::IOError(format!("Failed to read file '{}': {}", file_path, e)))?;
    process_image_with_cache(&image_bytes, config, cache, api_pool, output_format)
}

/// Check if a language value is the "all" wildcard (case-insensitive).
fn is_all_languages(lang: &str) -> bool {
    let lower = lang.to_ascii_lowercase();
    lower == "all" || lower == "*"
}

/// Resolve the "all"/"*" wildcard in a config's language field.
///
/// If the language is a wildcard, scans the tessdata directory for installed
/// languages and returns a new config with the resolved language string.
/// Otherwise returns `None`, indicating the original config should be used as-is.
fn resolve_config_language(config: &TesseractConfig) -> Result<Option<TesseractConfig>, OcrError> {
    if is_all_languages(&config.language) {
        let bootstrap_langs = vec!["eng".to_string()];
        let tessdata_path = resolve_tessdata_path(&bootstrap_langs, config.tessdata_path.as_deref())?;
        let resolved = resolve_all_installed_languages(&tessdata_path)?;
        let mut resolved_config = config.clone();
        resolved_config.language = resolved;
        Ok(Some(resolved_config))
    } else {
        Ok(None)
    }
}

/// Process an image and return OCR results, using cache if enabled.
///
/// Resolves the `"all"` / `"*"` language wildcard, then delegates to
/// [`process_image_resolved`] for caching and OCR execution.
///
/// # Arguments
///
/// * `image_bytes` - Raw image data
/// * `config` - OCR configuration
/// * `cache` - Cache instance
/// * `output_format` - Optional output format (Plain, Markdown, Djot) for proper mime_type handling
///
/// # Returns
///
/// OCR extraction result
pub(super) fn process_image_with_cache(
    image_bytes: &[u8],
    config: &TesseractConfig,
    cache: &OcrCache,
    api_pool: &Arc<TesseractApiPool>,
    output_format: Option<crate::core::config::OutputFormat>,
) -> Result<OcrExtractionResult, OcrError> {
    config.validate().map_err(OcrError::InvalidConfiguration)?;

    let resolved = resolve_config_language(config)?;
    let config = resolved.as_ref().unwrap_or(config);

    process_image_resolved(image_bytes, config, cache, api_pool, output_format)
}

/// Inner implementation operating on an already-resolved config.
///
/// Handles cache lookup, OCR execution, and cache storage. Callers are
/// responsible for validating and resolving wildcards in the config before
/// calling this function.
fn process_image_resolved(
    image_bytes: &[u8],
    config: &TesseractConfig,
    cache: &OcrCache,
    api_pool: &Arc<TesseractApiPool>,
    output_format: Option<crate::core::config::OutputFormat>,
) -> Result<OcrExtractionResult, OcrError> {
    let image_hash = crate::cache::blake3_hash_bytes(image_bytes);

    // #1787: the cache key must fold in the tessdata directory OCR will actually run against,
    // not only the optional `config.tessdata_path` override — otherwise two calls that resolve
    // to different directories through `TESSDATA_PREFIX` or another fallback in the search
    // chain (see `resolve_tessdata_path`) hash identically and share a cache entry even though
    // a different tessdata model produced the cached text. Resolved once, up front, so the
    // lookup below is keyed on the same directory `perform_ocr` will use on a miss.
    let languages: Vec<String> = config.language.split('+').map(|lang| lang.trim().to_string()).collect();
    let resolved_tessdata_path = resolve_tessdata_path(&languages, config.tessdata_path.as_deref())?;

    let config_str = hash_config(config, &resolved_tessdata_path);

    // `output_format` is part of the cache identity: it selects the renderer and
    // therefore the `content` and `mime_type` of the result. Omitting it served a
    // document cached as one format for a request for another (#205).
    if config.use_cache
        && let Some(cached_result) =
            cache.get_cached_result(&image_hash, "tesseract", &config_str, output_format.as_ref())?
    {
        #[cfg(feature = "otel")]
        tracing::Span::current().record("cache.hit", true);
        return Ok(cached_result);
    }

    #[cfg(feature = "otel")]
    tracing::Span::current().record("cache.hit", false);

    let extraction_config = output_format.as_ref().map(|fmt| ExtractionConfig {
        output_format: fmt.clone(),
        ..Default::default()
    });

    let result = perform_ocr(image_bytes, config, api_pool, extraction_config.as_ref())?;

    if config.use_cache
        && let Err(e) = cache.set_cached_result(&image_hash, "tesseract", &config_str, output_format.as_ref(), &result)
    {
        tracing::warn!(error = %e, "Failed to cache the OCR result; the next identical request will re-run OCR");
    }

    Ok(result)
}

/// Process multiple image files in parallel using Rayon.
///
/// Validates and resolves the language wildcard once, then processes all files
/// in parallel using [`process_image_resolved`] directly (skipping redundant
/// per-image resolution).
///
/// Results are returned in the same order as the input file paths.
#[cfg(test)]
pub(super) fn process_image_files_batch(
    file_paths: Vec<String>,
    config: &TesseractConfig,
    cache: &OcrCache,
    api_pool: &Arc<TesseractApiPool>,
) -> Vec<BatchItemResult> {
    #[cfg(not(target_arch = "wasm32"))]
    use rayon::prelude::*;

    if let Err(e) = config.validate().map_err(OcrError::InvalidConfiguration) {
        return file_paths
            .into_iter()
            .map(|path| BatchItemResult {
                file_path: path,
                success: false,
                result: None,
                error: Some(e.to_string()),
            })
            .collect();
    }

    let resolved = match resolve_config_language(config) {
        Ok(r) => r,
        Err(e) => {
            return file_paths
                .into_iter()
                .map(|path| BatchItemResult {
                    file_path: path,
                    success: false,
                    result: None,
                    error: Some(e.to_string()),
                })
                .collect();
        }
    };
    let config = resolved.as_ref().unwrap_or(config);

    #[cfg(not(target_arch = "wasm32"))]
    {
        file_paths
            .par_iter()
            .map(|path| {
                let image_bytes = match std::fs::read(path) {
                    Ok(b) => b,
                    Err(e) => {
                        return BatchItemResult {
                            file_path: path.clone(),
                            success: false,
                            result: None,
                            error: Some(
                                OcrError::IOError(format!("Failed to read file '{}': {}", path, e)).to_string(),
                            ),
                        };
                    }
                };
                match process_image_resolved(&image_bytes, config, cache, api_pool, None) {
                    Ok(result) => BatchItemResult {
                        file_path: path.clone(),
                        success: true,
                        result: Some(result),
                        error: None,
                    },
                    Err(e) => BatchItemResult {
                        file_path: path.clone(),
                        success: false,
                        result: None,
                        error: Some(e.to_string()),
                    },
                }
            })
            .collect()
    }
    #[cfg(target_arch = "wasm32")]
    {
        file_paths
            .iter()
            .map(|path| {
                let image_bytes = match std::fs::read(path) {
                    Ok(b) => b,
                    Err(e) => {
                        return BatchItemResult {
                            file_path: path.clone(),
                            success: false,
                            result: None,
                            error: Some(
                                OcrError::IOError(format!("Failed to read file '{}': {}", path, e)).to_string(),
                            ),
                        };
                    }
                };
                match process_image_resolved(&image_bytes, config, cache, api_pool, None) {
                    Ok(result) => BatchItemResult {
                        file_path: path.clone(),
                        success: true,
                        result: Some(result),
                        error: None,
                    },
                    Err(e) => BatchItemResult {
                        file_path: path.clone(),
                        success: false,
                        result: None,
                        error: Some(e.to_string()),
                    },
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocr::hocr_parser::{
        HOCR_FONT_SIZE_ATTRIBUTE, parse_hocr_to_internal_document_with_page_offset_and_stats,
    };
    use crate::table_core::cluster_words_into_table_regions;
    use serial_test::serial;
    use tempfile::tempdir;

    #[test]
    fn ocr_line_sort_uses_each_lines_direction_and_preserves_identifiers() {
        let word = |text: &str, left| HocrWord {
            text: text.to_string(),
            left,
            top: 0,
            width: 40,
            height: 20,
            confidence: 95.0,
        };
        let mut rtl = vec![word("כמות", 100), word("REQ-2026-104", 200), word("מצב", 300)];
        let mut ltr = vec![word("world", 300), word("שלום", 200), word("hello", 100)];
        let mut numeric = vec![word("42", 300), word("17", 100), word("09", 200)];
        let mut rtl_with_url = vec![
            word("https://example.test/a/very/long/resource-name", 100),
            word("الحالة", 300),
        ];
        let mut rtl_edge_tie = vec![word("status", 100), word("الحالة", 300)];
        let mut ltr_edge_tie = vec![word("الحالة", 100), word("status", 300)];

        sort_ocr_line_words(&mut rtl);
        sort_ocr_line_words(&mut ltr);
        sort_ocr_line_words(&mut numeric);
        sort_ocr_line_words(&mut rtl_with_url);
        sort_ocr_line_words(&mut rtl_edge_tie);
        sort_ocr_line_words(&mut ltr_edge_tie);

        assert_eq!(
            rtl.iter().map(|word| word.text.as_str()).collect::<Vec<_>>(),
            ["מצב", "REQ-2026-104", "כמות"]
        );
        assert_eq!(
            ltr.iter().map(|word| word.text.as_str()).collect::<Vec<_>>(),
            ["hello", "שלום", "world"]
        );
        assert_eq!(
            numeric.iter().map(|word| word.text.as_str()).collect::<Vec<_>>(),
            ["17", "09", "42"]
        );
        assert_eq!(
            rtl_with_url.iter().map(|word| word.text.as_str()).collect::<Vec<_>>(),
            ["الحالة", "https://example.test/a/very/long/resource-name"]
        );
        assert_eq!(
            rtl_edge_tie.iter().map(|word| word.text.as_str()).collect::<Vec<_>>(),
            ["الحالة", "status"]
        );
        assert_eq!(
            ltr_edge_tie.iter().map(|word| word.text.as_str()).collect::<Vec<_>>(),
            ["الحالة", "status"]
        );
    }

    #[test]
    fn inline_content_groups_y_jitter_before_per_line_direction_sorting() {
        let tsv = format!(
            "{TSV_HEADER}\
5\t1\t1\t1\t1\t1\t100\t103\t60\t20\t90\tالثاني\n\
5\t1\t1\t1\t1\t2\t300\t100\t60\t20\t90\tالأول\n\
5\t1\t1\t1\t2\t1\t300\t203\t60\t20\t90\tworld\n\
5\t1\t1\t1\t2\t2\t100\t200\t60\t20\t90\thello\n"
        );

        let content = build_content_with_inline_tables(&tsv, &[], 0.0);

        assert_eq!(content, "الأول الثاني\n\nhello world");
    }

    #[test]
    fn rtl_table_geometry_stays_ascending_until_final_presentation() {
        let word = |text: &str, left, top| HocrWord {
            text: text.to_string(),
            left,
            top,
            width: 40,
            height: 20,
            confidence: 95.0,
        };
        let words = vec![
            word("כמות", 100, 0),
            word("קוד", 200, 0),
            word("מצב", 300, 0),
            word("42", 100, 60),
            word("REQ-2026-104", 200, 60),
            word("מוכן", 300, 60),
        ];
        let (mut table, mut positions) = reconstruct_cleaned_table(&words, &TesseractConfig::default());

        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));

        orient_table_for_reading_order(&mut table, &mut positions, &words);

        assert_eq!(table[0], ["מצב", "קוד", "כמות"]);
        assert!(positions.windows(2).all(|pair| pair[0] > pair[1]));
    }

    #[cfg(feature = "bundle-tessdata-eng")]
    fn recover_quantity_from_image(image: &image::RgbImage) -> Option<HocrWord> {
        let tessdata = tempfile::tempdir().expect("temporary tessdata directory must be created");
        std::fs::write(
            tessdata.path().join("eng.traineddata"),
            xberg_tesseract::bundled_eng_traineddata().expect("English tessdata must be bundled for this test"),
        )
        .expect("bundled English tessdata must be materialized");
        let api = xberg_tesseract::TesseractAPI::new().expect("Tesseract API must initialize");
        api.init(
            tessdata.path().to_str().expect("temporary tessdata path must be UTF-8"),
            "eng",
        )
        .expect("English tessdata must initialize");
        api.set_page_seg_mode(TessPageSegMode::PSM_SPARSE_TEXT)
            .expect("sparse page segmentation mode must apply");
        api.set_image(
            image.as_raw(),
            image.width() as i32,
            image.height() as i32,
            3,
            (image.width() * 3) as i32,
        )
        .expect("quantity image must load");
        api.recognize().expect("initial sparse recognition must complete");
        let config = TesseractConfig {
            psm: TessPageSegMode::PSM_SPARSE_TEXT as u8,
            ..TesseractConfig::default()
        };
        let region = QuantityRetryRegion {
            row: 0,
            column: 0,
            left: 0,
            top: 0,
            width: image.width(),
            height: image.height(),
            word_left: 0,
            word_top: 0,
            word_width: image.width(),
            word_height: image.height(),
        };
        recover_blank_quantity_word(&api, &config, Some(&region), image.width(), image.height())
    }

    #[cfg(feature = "bundle-tessdata-eng")]
    fn isolated_quantity_image() -> image::RgbImage {
        image::load_from_memory(include_bytes!("../../../test_data/ocr/isolated_quantity_cell.png"))
            .expect("embedded quantity PNG must load")
            .into_rgb8()
    }

    #[test]
    #[cfg(feature = "bundle-tessdata-eng")]
    fn blank_quantity_retry_recognizes_single_and_multi_digit_cells() {
        let single_digit = isolated_quantity_image();
        let recovered = recover_quantity_from_image(&single_digit).expect("isolated quantity must be recovered");
        assert_eq!(recovered.text, "5");

        let digit = image::imageops::crop_imm(&single_digit, 106, 25, 24, 36).to_image();
        let mut multi_digit = image::RgbImage::from_pixel(170, 91, image::Rgb([255, 255, 255]));
        for left in [45_i64, 69, 93] {
            image::imageops::overlay(&mut multi_digit, &digit, left, 25);
        }
        let recovered = recover_quantity_from_image(&multi_digit).expect("multi-digit quantity must be recovered");
        assert_eq!(recovered.text, "555");
    }

    #[test]
    fn source_retry_crop_maps_processed_cell_into_original_pixels() {
        let region = QuantityRetryRegion {
            row: 3,
            column: 1,
            left: 1_662,
            top: 1_828,
            width: 164,
            height: 81,
            word_left: 1_699,
            word_top: 1_850,
            word_width: 73,
            word_height: 37,
        };
        assert_eq!(
            source_retry_crop(&region, 3_200, 4_089, 1_800, 2_300),
            Some((934, 1_028, 94, 46))
        );
        assert_eq!(source_retry_crop(&region, 0, 4_089, 1_800, 2_300), None);
    }

    #[test]
    #[cfg(feature = "bundle-tessdata-eng")]
    fn source_quantity_retry_reads_native_pixels_and_rejects_blank_pixels() {
        let tessdata = tempfile::tempdir().expect("temporary tessdata directory");
        std::fs::write(
            tessdata.path().join("eng.traineddata"),
            xberg_tesseract::bundled_eng_traineddata().expect("bundled English tessdata"),
        )
        .expect("write tessdata");
        let path = tessdata.path().to_str().expect("UTF-8 tessdata path");
        let region = QuantityRetryRegion {
            row: 0,
            column: 0,
            left: 180,
            top: 30,
            width: 120,
            height: 120,
            word_left: 212,
            word_top: 50,
            word_width: 48,
            word_height: 72,
        };
        let source = include_bytes!("../../../test_data/ocr/isolated_quantity_cell.png");
        let config = TesseractConfig::default();
        let recovered =
            recover_blank_quantity_from_source(source, &config, &region, 340, 182, path, &SecurityLimits::default())
                .expect("native crop recovers the digit");
        assert_eq!(recovered.text, "5");
        assert!(recovered.confidence >= MIN_SOURCE_QUANTITY_RETRY_CONFIDENCE);

        let mut rotated_config = config.clone();
        rotated_config.preprocessing = Some(crate::types::ImagePreprocessingConfig {
            auto_rotate: true,
            ..Default::default()
        });
        assert!(
            recover_blank_quantity_from_source(
                source,
                &rotated_config,
                &region,
                340,
                182,
                path,
                &SecurityLimits::default(),
            )
            .is_none(),
            "source coordinates must not be reused after auto-rotation"
        );

        let mut strict_config = config.clone();
        strict_config.table_min_confidence = recovered.confidence + 1.0;
        assert!(
            recover_blank_quantity_from_source(
                source,
                &strict_config,
                &region,
                340,
                182,
                path,
                &SecurityLimits::default(),
            )
            .is_none(),
            "a digit below the configured confidence threshold must stay blank"
        );

        let blank = image::RgbImage::from_pixel(170, 91, image::Rgb([255, 255, 255]));
        let mut blank_png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(blank)
            .write_to(&mut blank_png, image::ImageFormat::Png)
            .expect("encode blank PNG");
        assert!(
            recover_blank_quantity_from_source(
                blank_png.get_ref(),
                &config,
                &region,
                340,
                182,
                path,
                &SecurityLimits::default(),
            )
            .is_none(),
            "a blank quantity must remain blank"
        );
    }

    #[test]
    fn parse_tsv_to_elements_yields_lines_at_the_default_element_level() {
        let tsv = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n\
                   1\t1\t0\t0\t0\t0\t0\t0\t200\t100\t-1\t\n\
                   2\t1\t1\t0\t0\t0\t10\t5\t60\t35\t-1\t\n\
                   3\t1\t1\t1\t0\t0\t10\t5\t60\t35\t-1\t\n\
                   4\t1\t1\t1\t1\t0\t10\t5\t60\t12\t-1\t\n\
                   5\t1\t1\t1\t1\t1\t10\t5\t20\t10\t90\tTotal\n\
                   5\t1\t1\t1\t1\t2\t40\t7\t30\t10\t60\tdue\n\
                   4\t1\t1\t1\t2\t0\t10\t30\t20\t10\t-1\t\n\
                   5\t1\t1\t1\t2\t1\t10\t30\t20\t10\t80\t42\n";
        let default_level = crate::types::OcrElementConfig {
            include_elements: true,
            ..Default::default()
        };

        let elements = parse_tsv_to_elements(tsv, 0.0, 4);
        let selected = default_level.select_elements(&elements);

        assert_eq!(
            selected.iter().map(|element| element.text.as_str()).collect::<Vec<_>>(),
            ["Total due", "42"]
        );
        assert!(selected.iter().all(|element| element.page_number == 4));
        assert_eq!(
            elements
                .iter()
                .filter(|element| element.level == crate::types::OcrElementLevel::Word)
                .count(),
            3
        );
    }

    fn confidence_word(text: &str, confidence: f32) -> xberg_tesseract::WordData {
        xberg_tesseract::WordData {
            text: text.to_string(),
            right: 10,
            bottom: 10,
            confidence,
            ..Default::default()
        }
    }

    #[test]
    fn should_publish_only_retained_hocr_word_confidence_metadata() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocr_line">
                    <span class="ocrx_word" title="x_wconf 20">CLEAR</span>
                    <span class="ocrx_word" title="x_wconf 90">WORDS</span>
                </span>
                <span class="ocr_line">
                    <span class="ocrx_word" title="x_wconf 0">OWATS</span>
                    <span class="ocrx_word" title="x_wconf 0">DNDEVET</span>
                </span>
            </p>
        </div>"#;
        let is_valid_word = |word: &str| Some(matches!(word, "CLEAR" | "WORDS"));
        let filter = DictionaryLineFilter {
            is_valid_word: &is_valid_word,
            max_invalid_ratio: crate::ocr::hocr_parser::DEFAULT_DICT_INVALID_LINE_RATIO,
        };
        let result = parse_hocr_to_internal_document_with_page_offset_and_stats(hocr, Some(&filter), 1);
        let mut metadata = HashMap::new();

        insert_retained_word_confidence_metadata(&mut metadata, &result.retained_word_confidence_stats);

        assert_eq!(flatten_hocr_elements_to_text(&result.document.elements), "CLEAR WORDS");
        assert_eq!(metadata.get("word_count"), Some(&serde_json::json!(2)));
        assert_eq!(metadata.get("mean_text_conf"), Some(&serde_json::json!(55)));
        assert_eq!(metadata.get("median_word_conf"), Some(&serde_json::json!(55)));
        assert_eq!(metadata.get("p10_word_conf"), Some(&serde_json::json!(20)));
        assert_eq!(metadata.get("low_conf_word_count"), Some(&serde_json::json!(1)));
        assert_eq!(metadata.len(), 5);
    }

    /// GH#1554 regression: `perform_ocr` must use the caller's `ExtractionConfig.security_limits`
    /// rather than always decoding under `SecurityLimits::default()`, which silently refused
    /// ordinary high-DPI scans a caller had explicitly configured a higher limit to permit.
    #[test]
    fn should_use_configured_security_limits_when_extraction_config_present() {
        let config = ExtractionConfig {
            security_limits: Some(SecurityLimits {
                max_content_size: 200 * 1024 * 1024,
                ..Default::default()
            }),
            ..Default::default()
        };

        let resolved = security_limits_for_ocr(&TesseractConfig::default(), Some(&config));

        assert_eq!(resolved.max_content_size, 200 * 1024 * 1024);
    }

    /// `None` (no `ExtractionConfig` reached the call) must fall back to
    /// `SecurityLimits::default()`, not to an unbounded/disabled check.
    #[test]
    fn should_fall_back_to_default_security_limits_when_extraction_config_absent() {
        let resolved = security_limits_for_ocr(&TesseractConfig::default(), None);

        assert_eq!(resolved.max_content_size, SecurityLimits::default().max_content_size);
    }

    /// GH#1651. The two tests above only ever exercised this helper directly, which is why
    /// they stayed green while every real backend route decoded under the default: a
    /// backend gets `&OcrConfig` and no `ExtractionConfig`, so the only value that can
    /// reach here from a caller is the one `config_to_tesseract` copies onto
    /// `TesseractConfig`. That value must therefore win. ~keep
    #[test]
    fn should_prefer_the_tesseract_config_limits_over_the_extraction_config_limits() {
        let tesseract_config = TesseractConfig {
            security_limits: Some(SecurityLimits {
                max_content_size: 300 * 1024 * 1024,
                ..Default::default()
            }),
            ..Default::default()
        };
        let extraction_config = ExtractionConfig {
            security_limits: Some(SecurityLimits {
                max_content_size: 200 * 1024 * 1024,
                ..Default::default()
            }),
            ..Default::default()
        };

        let resolved = security_limits_for_ocr(&tesseract_config, Some(&extraction_config));

        assert_eq!(resolved.max_content_size, 300 * 1024 * 1024);
    }

    /// A backend route supplies a synthetic `ExtractionConfig` carrying only `output_format`,
    /// so the caller's limits arrive solely on `TesseractConfig` and must still be honoured. ~keep
    #[test]
    fn should_use_the_tesseract_config_limits_when_no_extraction_config_is_present() {
        let tesseract_config = TesseractConfig {
            security_limits: Some(SecurityLimits {
                max_content_size: 300 * 1024 * 1024,
                ..Default::default()
            }),
            ..Default::default()
        };

        let resolved = security_limits_for_ocr(&tesseract_config, None);

        assert_eq!(resolved.max_content_size, 300 * 1024 * 1024);
    }

    #[test]
    fn should_omit_hocr_confidence_quantiles_when_every_word_is_rejected() {
        let hocr = r#"<div class="ocr_page" title="ppageno 0">
            <p class="ocr_par">
                <span class="ocr_line">
                    <span class="ocrx_word" title="x_wconf 10">OWATS</span>
                    <span class="ocrx_word" title="x_wconf 90">DNDEVET</span>
                </span>
            </p>
        </div>"#;
        let always_invalid = |_: &str| Some(false);
        let filter = DictionaryLineFilter {
            is_valid_word: &always_invalid,
            max_invalid_ratio: crate::ocr::hocr_parser::DEFAULT_DICT_INVALID_LINE_RATIO,
        };
        let result = parse_hocr_to_internal_document_with_page_offset_and_stats(hocr, Some(&filter), 1);
        let mut metadata = HashMap::new();

        insert_retained_word_confidence_metadata(&mut metadata, &result.retained_word_confidence_stats);

        assert!(flatten_hocr_elements_to_text(&result.document.elements).is_empty());
        assert_eq!(metadata.get("word_count"), Some(&serde_json::json!(0)));
        assert_eq!(metadata.get("low_conf_word_count"), Some(&serde_json::json!(0)));
        assert!(!metadata.contains_key("mean_text_conf"));
        assert!(!metadata.contains_key("median_word_conf"));
        assert!(!metadata.contains_key("p10_word_conf"));
        assert_eq!(metadata.len(), 2);
    }

    #[test]
    fn should_publish_zero_text_confidence_words_when_every_iterator_word_is_filtered() {
        let words = [confidence_word("\u{001f}", 95.0)];
        let stats = retained_text_word_confidence_stats("", &words);
        let mut metadata = HashMap::from([
            ("word_count".to_string(), serde_json::json!(1)),
            ("low_conf_word_count".to_string(), serde_json::json!(0)),
            ("mean_text_conf".to_string(), serde_json::json!(95)),
            ("median_word_conf".to_string(), serde_json::json!(95)),
            ("p10_word_conf".to_string(), serde_json::json!(95)),
        ]);

        insert_retained_word_confidence_metadata(&mut metadata, &stats);

        assert_eq!(metadata.get("word_count"), Some(&serde_json::json!(0)));
        assert_eq!(metadata.get("low_conf_word_count"), Some(&serde_json::json!(0)));
        assert!(!metadata.contains_key("mean_text_conf"));
        assert!(!metadata.contains_key("median_word_conf"));
        assert!(!metadata.contains_key("p10_word_conf"));
    }

    #[test]
    fn should_publish_only_partially_retained_text_word_confidences() {
        let words = [
            confidence_word("RETAINED", 20.0),
            confidence_word("\u{001f}", 95.0),
            confidence_word("WORDS", 90.0),
        ];
        let stats = retained_text_word_confidence_stats("RETAINED WORDS", &words);
        let mut metadata = HashMap::from([
            ("word_count".to_string(), serde_json::json!(3)),
            ("low_conf_word_count".to_string(), serde_json::json!(1)),
            ("mean_text_conf".to_string(), serde_json::json!(68)),
            ("median_word_conf".to_string(), serde_json::json!(90)),
            ("p10_word_conf".to_string(), serde_json::json!(20)),
        ]);

        insert_retained_word_confidence_metadata(&mut metadata, &stats);

        assert_eq!(metadata.get("word_count"), Some(&serde_json::json!(2)));
        assert_eq!(metadata.get("mean_text_conf"), Some(&serde_json::json!(55)));
        assert_eq!(metadata.get("median_word_conf"), Some(&serde_json::json!(55)));
        assert_eq!(metadata.get("p10_word_conf"), Some(&serde_json::json!(20)));
        assert_eq!(metadata.get("low_conf_word_count"), Some(&serde_json::json!(1)));
    }

    #[test]
    fn should_align_words_after_a_retained_iterator_gap() {
        let words = [confidence_word("FIRST", 10.0), confidence_word("LAST", 90.0)];

        let stats = retained_text_word_confidence_stats("FIRST OMITTED LAST", &words);

        assert_eq!(stats.word_count(), 2);
        assert_eq!(stats.mean(), Some(50));
        assert_eq!(stats.median(), Some(50));
        assert_eq!(stats.p10(), Some(10));
        assert_eq!(stats.low_confidence_word_count(), 1);
    }

    #[test]
    fn should_align_repeated_words_without_losing_later_words() {
        let words = [confidence_word("A", 20.0), confidence_word("B", 80.0)];

        let stats = retained_text_word_confidence_stats("A A B", &words);

        assert_eq!(stats.word_count(), 2);
        assert_eq!(stats.mean(), Some(50));
        assert_eq!(stats.median(), Some(50));
        assert_eq!(stats.p10(), Some(20));
        assert_eq!(stats.low_confidence_word_count(), 1);
    }

    #[test]
    fn should_align_punctuation_and_unicode_exactly() {
        let words = [
            confidence_word("Hello,", 60.0),
            confidence_word("Gr\u{00fc}\u{00df}e", 70.0),
            confidence_word("\u{4e16}\u{754c}!", 80.0),
        ];

        let stats = retained_text_word_confidence_stats("Hello, Gr\u{00fc}\u{00df}e \u{4e16}\u{754c}!", &words);

        assert_eq!(stats.word_count(), 3);
        assert_eq!(stats.mean(), Some(70));
        assert_eq!(stats.median(), Some(70));
        assert_eq!(stats.p10(), Some(60));
        assert_eq!(stats.low_confidence_word_count(), 0);
    }

    #[test]
    fn should_align_multi_token_words_across_whitespace_variants() {
        let words = [confidence_word("ALPHA\tBETA", 75.0), confidence_word("GAMMA", 85.0)];

        let stats = retained_text_word_confidence_stats("ALPHA  \n BETA\r\nGAMMA", &words);

        assert_eq!(stats.word_count(), 2);
        assert_eq!(stats.mean(), Some(80));
        assert_eq!(stats.median(), Some(80));
        assert_eq!(stats.p10(), Some(75));
        assert_eq!(stats.low_confidence_word_count(), 0);
    }

    /// Exact count: a known number of skipped words must produce exactly the
    /// matching `word_iterator_skipped_count` value, not just a truthy presence
    /// (#192 — catches off-by-one or double-counting regressions).
    #[test]
    fn should_insert_exact_skipped_count_when_words_were_skipped() {
        let mut metadata = HashMap::new();

        insert_word_iterator_skipped_count_metadata(&mut metadata, 3);

        assert_eq!(
            metadata.get("word_iterator_skipped_count"),
            Some(&serde_json::Value::Number(3.into()))
        );
    }

    /// When nothing was skipped, the metadata key must be entirely ABSENT, not
    /// present with a value of `0` — callers rely on key absence as the
    /// clean-extraction signal (#192).
    #[test]
    fn should_omit_skipped_count_key_when_nothing_was_skipped() {
        let mut metadata = HashMap::new();

        insert_word_iterator_skipped_count_metadata(&mut metadata, 0);

        assert!(
            !metadata.contains_key("word_iterator_skipped_count"),
            "key must be absent, not present with value 0"
        );
        assert!(metadata.is_empty());
    }

    /// When `auto_rotate` is not requested, the flag must be `false`
    /// regardless of which build compiled this test (#309).
    #[test]
    fn should_report_auto_rotate_available_when_not_requested() {
        assert!(!is_auto_rotate_requested_but_unavailable(false));
    }

    /// When `auto_rotate` is requested, the flag reflects whether *this* build
    /// has the `auto-rotate` feature compiled in — `true` (unavailable) on a
    /// build without it, `false` (available) on a build with it (#309).
    #[test]
    fn should_report_auto_rotate_unavailable_only_without_the_feature() {
        assert_eq!(is_auto_rotate_requested_but_unavailable(true), cfg!(not(auto_rotate)));
    }

    fn dict_word(text: &str) -> xberg_tesseract::WordData {
        xberg_tesseract::WordData {
            text: text.to_string(),
            confidence: 95.0,
            ..Default::default()
        }
    }

    /// A page dominated by non-dictionary "words" (the LAAALDLI-style noise a scanned
    /// drawing produces) must report a HIGH invalid ratio, not `None` and not a low one.
    #[test]
    fn should_report_high_ratio_when_most_words_are_dictionary_invalid() {
        let words = ["LAAALDLI", "sky", "AEA", "ails", "Bri", "ENT", "FRONT", "ELEVATION"].map(dict_word);
        // Mirror the ordinance_2197 page-13 evidence: only FRONT/ELEVATION are real words.
        let is_valid = |text: &str| Some(matches!(text, "FRONT" | "ELEVATION"));

        let ratio = invalid_word_ratio_from(&words, is_valid).expect("enough candidates to score");

        assert_eq!(
            ratio, 0.75,
            "6 of 8 candidates are dictionary-invalid, expected exactly 0.75"
        );
    }

    /// A page of genuine prose (all real dictionary words) must report a LOW invalid
    /// ratio, not one indistinguishable from the noise case above.
    #[test]
    fn should_report_low_ratio_when_all_words_are_dictionary_valid() {
        let words = ["EXHIBIT", "PLANT", "LIST", "DATE", "November", "Nineteen"].map(dict_word);
        let is_valid = |_: &str| Some(true);

        let ratio = invalid_word_ratio_from(&words, is_valid).expect("enough candidates to score");

        assert_eq!(ratio, 0.0, "an all-valid page must score exactly 0.0, not merely 'low'");
    }

    /// Words below `MIN_WORD_LEN_FOR_DICT_CHECK` or containing non-alphabetic characters
    /// (numbers, punctuation) are never dictionary words regardless of legitimacy, so they
    /// must not count as candidates at all -- scoring them would bias real content toward
    /// "invalid".
    #[test]
    fn should_exclude_short_and_non_alphabetic_words_from_candidates() {
        let words = ["12", "a", "Bo", "3.14", "PLANT", "LIST", "DATE", "November", "Nineteen"].map(dict_word);
        let is_valid = |_: &str| Some(true);

        let ratio = invalid_word_ratio_from(&words, is_valid).expect("enough candidates to score");

        assert_eq!(
            ratio, 0.0,
            "only the 5 alphabetic 3+ char words should count as candidates"
        );
    }

    /// Too few dictionary-checkable words must yield `None`, never `0.0` -- a page this
    /// short is not evidence that "every word is valid".
    #[test]
    fn should_return_none_when_too_few_candidates() {
        let words = ["PLANT", "LIST"].map(dict_word);
        let is_valid = |_: &str| Some(true);

        assert_eq!(
            invalid_word_ratio_from(&words, is_valid),
            None,
            "fewer than MIN_DICT_CANDIDATES_FOR_RATIO checkable words must not produce a ratio"
        );
    }

    /// A failed dictionary lookup must be excluded from both the numerator and the
    /// denominator, not silently counted as invalid (which would bias the ratio) or as
    /// valid (which would hide real noise).
    #[test]
    fn should_exclude_failed_lookups_from_candidates() {
        let words = ["PLANT", "LIST", "DATE", "November", "Nineteen", "BROKEN"].map(dict_word);
        let is_valid = |text: &str| if text == "BROKEN" { None } else { Some(true) };

        let ratio = invalid_word_ratio_from(&words, is_valid).expect("enough candidates to score");

        assert_eq!(ratio, 0.0, "the failed lookup must not count as invalid");
    }

    fn word_at(left: u32, top: u32, width: u32, height: u32, text: &str) -> crate::table_core::HocrWord {
        crate::table_core::HocrWord {
            text: text.to_string(),
            left,
            top,
            width,
            height,
            confidence: 95.0,
        }
    }

    /// An invented scanned table: a label column and four right-aligned amount columns. Each
    /// amount column holds long values (`12,345`), short values (`85`) and nil dashes, so its
    /// tokens' left edges fall into two groups by digit width, and the drift fold has to join
    /// them back into one column. `junk` replaces the nil dash of `Item 6` in `Column A` with an
    /// OCR misread that holds a letter. `second_header_row` adds a `Year N` label under each column
    /// header, as a two-row scanned header band has.
    #[cfg(feature = "pdf")]
    fn split_amount_table_words(junk: Option<&str>, second_header_row: bool) -> Vec<crate::table_core::HocrWord> {
        const CHAR_WIDTH: u32 = 15;
        const HEIGHT: u32 = 25;
        const RIGHT_EDGES: [u32; 4] = [700, 920, 1140, 1360];
        let first_data_row = if second_header_row { 2 } else { 1 };
        let row_top = |row: u32| 200 + row * 45;
        let right_aligned = |right: u32, row: u32, text: &str| {
            let width = CHAR_WIDTH * text.chars().count() as u32;
            word_at(right - width, row_top(row), width, HEIGHT, text)
        };
        let mut words = vec![word_at(100, row_top(0), 60, HEIGHT, "Item")];
        for (column, right) in ["A", "B", "C", "D"].into_iter().zip(RIGHT_EDGES) {
            words.push(word_at(right - 115, row_top(0), 90, HEIGHT, "Column"));
            words.push(right_aligned(right, 0, column));
            if second_header_row {
                words.push(word_at(right - 90, row_top(1), 60, HEIGHT, "Year"));
                words.push(right_aligned(right, 1, &(right / 220 - 2).to_string()));
            }
        }
        let long = [
            "12,345", "20,118", "31,902", "47,260", "58,431", "63,077", "71,594", "86,213",
        ];
        for row in 1..=12u32 {
            let top = row + first_data_row - 1;
            words.push(word_at(100, row_top(top), 60, HEIGHT, "Item"));
            words.push(word_at(170, row_top(top), CHAR_WIDTH * 2, HEIGHT, &row.to_string()));
            for (column, right) in RIGHT_EDGES.into_iter().enumerate() {
                let text = match row {
                    3 => "85".to_string(),
                    9 => "40".to_string(),
                    6 if column == 0 => junk.unwrap_or("-").to_string(),
                    6 | 12 => "-".to_string(),
                    _ => long[(row as usize + column) % long.len()].to_string(),
                };
                words.push(right_aligned(right, top, &text));
            }
        }
        words
    }

    /// An invented scanned table whose header labels read as text and are wider than the amounts
    /// under them: `Period 1` to `Period 4` over seven-character amounts (`112,345`), two-character
    /// amounts and nil dashes. Each label starts more than the threshold left of the long amounts,
    /// so it forms a header-only track, and the long amounts' left edges sit nearer that track than
    /// the short amounts'. ~keep
    #[cfg(feature = "pdf")]
    fn wide_label_table_words() -> Vec<crate::table_core::HocrWord> {
        const CHAR_WIDTH: u32 = 15;
        const HEIGHT: u32 = 25;
        const RIGHT_EDGES: [u32; 4] = [700, 920, 1140, 1360];
        let row_top = |row: u32| 200 + row * 45;
        let right_aligned = |right: u32, row: u32, text: &str| {
            let width = CHAR_WIDTH * text.chars().count() as u32;
            word_at(right - width, row_top(row), width, HEIGHT, text)
        };
        let mut words = vec![word_at(100, row_top(0), 60, HEIGHT, "Item")];
        for (period, right) in RIGHT_EDGES.into_iter().enumerate() {
            words.push(word_at(
                right - 165,
                row_top(0),
                165,
                HEIGHT,
                &format!("Period {}", period + 1),
            ));
        }
        let long = [
            "112,345", "120,118", "131,902", "147,260", "158,431", "163,077", "171,594", "186,213",
        ];
        for row in 1..=12u32 {
            words.push(word_at(100, row_top(row), 60, HEIGHT, "Item"));
            words.push(word_at(170, row_top(row), CHAR_WIDTH * 2, HEIGHT, &row.to_string()));
            for (column, right) in RIGHT_EDGES.into_iter().enumerate() {
                let text = match row {
                    3 => "85".to_string(),
                    9 => "40".to_string(),
                    6 | 12 => "-".to_string(),
                    _ => long[(row as usize + column) % long.len()].to_string(),
                };
                words.push(right_aligned(right, row, &text));
            }
        }
        words
    }

    /// A header label wider than its amounts formed a header-only track that never folded, the
    /// column's long amounts followed its left edge, the short amounts were left in a headerless
    /// track that failed the sparse-column check, and the whole table was dropped.
    ///
    /// TEST HONESTY: without the fix the grid has 9 columns for 5 on the page and is rejected. ~keep
    #[cfg(feature = "pdf")]
    #[test]
    fn a_header_label_wider_than_its_amounts_keeps_the_table() {
        let (grid, kept) = ocr_route_table(&wide_label_table_words());
        let kept = kept.unwrap_or_else(|| {
            panic!(
                "the table is dropped; its grid has {} columns for 5 on the page: {grid:?}",
                grid[0].len()
            )
        });
        assert_eq!(grid[0].len(), 5, "one column per label: {grid:?}");
        for value in [
            "112,345", "120,118", "131,902", "147,260", "158,431", "163,077", "171,594", "186,213", "85", "40",
        ] {
            assert!(
                kept.iter().flatten().any(|cell| cell.contains(value)),
                "value {value} is lost: {kept:?}"
            );
        }
        for period in ["Period 1", "Period 2", "Period 3", "Period 4"] {
            assert!(
                kept[0].iter().any(|cell| cell == period),
                "header {period} is not in the header row: {:?}",
                kept[0]
            );
        }
    }

    /// The OCR table route on `words`: the region's cleaned grid, then the table cleanup that
    /// keeps or rejects it.
    #[cfg(feature = "pdf")]
    fn ocr_route_table(words: &[crate::table_core::HocrWord]) -> (Vec<Vec<String>>, Option<Vec<Vec<String>>>) {
        let (grid, _) = reconstruct_cleaned_table(words, &TesseractConfig::default());
        let kept = post_process_table(grid.clone(), false, false);
        (grid, kept)
    }

    /// xberg-io/xberg#1952: one OCR junk cell with a letter stopped the right-aligned fold, the
    /// split amount track failed the sparse-column check, and the whole table was dropped.
    ///
    /// TEST HONESTY: without the fix the grid has 6 columns for 5 on the page, `Column A` split in
    /// two with `["Item 6", "", "a", "-", "-", "-"]`, and the table is rejected.
    #[cfg(feature = "pdf")]
    #[test]
    fn issue_1952_split_amount_column_with_one_junk_cell_keeps_the_table() {
        let (clean_grid, clean) = ocr_route_table(&split_amount_table_words(None, false));
        assert_eq!(
            clean_grid[0].len(),
            5,
            "control: the fold joins every amount column: {clean_grid:?}"
        );
        assert!(clean.is_some(), "control: the clean table is kept: {clean_grid:?}");

        let (grid, kept) = ocr_route_table(&split_amount_table_words(Some("a"), false));
        assert_table_kept_with_every_value(&grid, kept);
    }

    /// xberg-io/xberg#1952: the labels of a second header row counted as data tokens, so no
    /// amount column read as values, none folded, and the table was dropped.
    ///
    /// TEST HONESTY: without the fix the grid has 9 columns for 5 on the page and is rejected.
    #[cfg(feature = "pdf")]
    #[test]
    fn issue_1952_split_amount_column_under_a_two_row_header_keeps_the_table() {
        let (grid, kept) = ocr_route_table(&split_amount_table_words(None, true));
        assert_table_kept_with_every_value(&grid, kept);
    }

    #[cfg(feature = "pdf")]
    fn assert_table_kept_with_every_value(grid: &[Vec<String>], kept: Option<Vec<Vec<String>>>) {
        let kept = kept.unwrap_or_else(|| {
            panic!(
                "the table is dropped; its grid has {} columns for 5 on the page: {grid:?}",
                grid[0].len()
            )
        });
        for value in [
            "12,345", "20,118", "31,902", "47,260", "58,431", "63,077", "71,594", "86,213", "85", "40",
        ] {
            assert!(
                kept.iter().flatten().any(|cell| cell.contains(value)),
                "value {value} is lost: {kept:?}"
            );
        }
    }

    /// Builds a grid of `rows * cols` words starting at `(left, top)`, each
    /// cell `40x20` pixels, simulating one table's worth of OCR words.
    fn table_grid_words(left: u32, top: u32, rows: u32, cols: u32) -> Vec<crate::table_core::HocrWord> {
        let mut words = Vec::new();
        for row in 0..rows {
            for col in 0..cols {
                words.push(word_at(
                    left + col * 60,
                    top + row * 30,
                    40,
                    20,
                    &format!("r{row}c{col}"),
                ));
            }
        }
        words
    }

    /// A row of `values` grid words plus one tall low-confidence `=` mark between the first two.
    fn value_row_with_a_mark(values: u32) -> Vec<crate::table_core::HocrWord> {
        let mut words = table_grid_words(0, 100, 1, values);
        words.push(crate::table_core::HocrWord {
            confidence: 10.0,
            ..word_at(45, 88, 10, 45, "=")
        });
        words
    }

    #[test]
    fn a_region_counts_its_words_before_its_marks_are_dropped() {
        let at_minimum = value_row_with_a_mark(MIN_TABLE_CANDIDATE_WORDS as u32 - 1);
        assert_eq!(
            at_minimum.len(),
            MIN_TABLE_CANDIDATE_WORDS,
            "the mark is what brings this region up to the minimum, which is the point of the test"
        );

        let mut region = TableRegion {
            words: at_minimum.clone(),
            read_boxes: at_minimum,
        };
        region.drop_shading_marks(0.5);

        assert_eq!(region.words.len(), MIN_TABLE_CANDIDATE_WORDS - 1, "the mark is dropped");
        assert_eq!(
            region.read_boxes.len(),
            region.words.len(),
            "both vectors stay parallel"
        );
        assert!(region.words.iter().all(|word| word.text != "="));
    }

    #[test]
    fn punctuation_only_footer_is_not_an_ocr_table_candidate() {
        let words = vec![
            word_at(461, 2494, 44, 24, "["),
            word_at(915, 2494, 37, 25, "『"),
            word_at(1390, 2494, 63, 25, "!"),
            word_at(1993, 2494, 78, 25, "|"),
            word_at(461, 2519, 37, 6, "|"),
            word_at(749, 2519, 81, 6, "|"),
            word_at(915, 2518, 37, 7, "ー"),
            word_at(2088, 2519, 78, 6, "|"),
        ];

        assert!(!has_semantic_ocr_table_content(&words));
    }

    #[test]
    fn numeric_symbol_heavy_grid_is_an_ocr_table_candidate() {
        let words = vec![
            word_at(100, 100, 30, 20, "$"),
            word_at(200, 100, 30, 20, "%"),
            word_at(300, 100, 40, 20, "Q1"),
            word_at(400, 100, 40, 20, "Q2"),
            word_at(100, 140, 30, 20, "¥"),
            word_at(200, 140, 30, 20, "—"),
            word_at(300, 140, 70, 20, "1,200"),
            word_at(400, 140, 50, 20, "12%"),
        ];

        assert!(has_semantic_ocr_table_content(&words));
    }

    /// The line-row pass (GH#1834) rewrites each word's box onto its row's band but leaves
    /// `read_boxes` as Tesseract read them. A mark whose normalised box matches the row must still
    /// be dropped, or GH#1858's filter silently stops firing whenever GH#1834 runs ahead of it.
    /// Keyed on `words` instead, this region keeps the `=` and the two values glue into one cell.
    #[test]
    fn a_mark_normalised_onto_its_row_band_is_still_dropped() {
        let words = value_row_with_a_mark(MIN_TABLE_CANDIDATE_WORDS as u32 - 1);
        let read_boxes = words.clone();
        let band_top = words.iter().map(|word| word.top).min().expect("the row has words");
        let band_height = words.iter().map(|word| word.height).max().expect("the row has words");
        let normalised: Vec<_> = words
            .into_iter()
            .map(|word| crate::table_core::HocrWord {
                top: band_top,
                height: band_height,
                ..word
            })
            .collect();
        assert!(
            normalised.iter().all(|word| word.height == band_height),
            "the normalisation must erase the height anomaly, or this proves nothing"
        );

        let mut region = TableRegion {
            words: normalised,
            read_boxes,
        };
        region.drop_shading_marks(0.5);

        assert!(
            region.words.iter().all(|word| word.text != "="),
            "the mark must be dropped on its read box, not its normalised one: {:?}",
            region.words.iter().map(|word| word.text.as_str()).collect::<Vec<_>>()
        );
        assert_eq!(
            region.read_boxes.len(),
            region.words.len(),
            "both vectors stay parallel"
        );
    }

    #[test]
    fn normalized_band_frame_glyphs_are_removed_from_table_edges() {
        let words = vec![
            word_at(95, 100, 3, 20, "|"),
            word_at(100, 100, 70, 20, "[TOTAL"),
            word_at(300, 100, 60, 20, "1,000"),
            word_at(700, 100, 3, 20, "|"),
            word_at(100, 140, 70, 20, "[TOTAL"),
            word_at(300, 140, 60, 20, "2,000"),
            word_at(700, 140, 3, 20, "|"),
        ];
        let mut region = TableRegion {
            read_boxes: words.clone(),
            words,
        };

        region.drop_normalized_band_edge_glyphs();

        let texts: Vec<&str> = region.words.iter().map(|word| word.text.as_str()).collect();
        assert_eq!(texts, ["TOTAL", "1,000", "TOTAL", "2,000"]);
        assert_eq!(region.read_boxes.len(), region.words.len());
    }

    #[test]
    fn one_bracketed_label_does_not_license_edge_glyph_cleanup() {
        let words = vec![
            word_at(100, 100, 90, 20, "[Pending]"),
            word_at(300, 100, 60, 20, "1,000"),
            word_at(700, 100, 3, 20, "|"),
            word_at(100, 140, 70, 20, "Account"),
            word_at(300, 140, 60, 20, "2,000"),
        ];
        let mut region = TableRegion {
            read_boxes: words.clone(),
            words,
        };

        region.drop_normalized_band_edge_glyphs();

        let texts: Vec<&str> = region.words.iter().map(|word| word.text.as_str()).collect();
        assert_eq!(texts, ["[Pending]", "1,000", "|", "Account", "2,000"]);
        assert_eq!(region.read_boxes.len(), region.words.len());
    }

    #[test]
    fn cluster_words_into_table_regions_separates_two_distant_tables() {
        // Two 3x3 grids of words (9 words each) far apart vertically: a real
        // page with two independent tables separated by a paragraph of prose.
        let mut words = table_grid_words(0, 0, 3, 3);
        words.extend(table_grid_words(0, 1000, 3, 3));

        let regions = cluster_words_into_table_regions(&words);

        assert_eq!(regions.len(), 2, "two spatially distant grids should form two regions");
        assert_eq!(regions[0].len(), 9);
        assert_eq!(regions[1].len(), 9);
    }

    #[test]
    fn cluster_words_into_table_regions_does_not_merge_a_distant_small_fragment() {
        let mut words = table_grid_words(0, 0, 2, 3);
        words.extend(table_grid_words(0, 10_000, 1, 2));

        let regions = cluster_words_into_table_regions(&words);

        assert_eq!(regions.len(), 2, "distance must bound aligned-fragment merging");
        assert_eq!(regions[0].len(), 6);
        assert_eq!(regions[1].len(), 2);
    }

    #[test]
    fn cluster_words_into_table_regions_keeps_one_table_as_a_single_region() {
        let words = table_grid_words(0, 0, 4, 3);

        let regions = cluster_words_into_table_regions(&words);

        assert_eq!(
            regions.len(),
            1,
            "a single table's own row gaps must not be split into regions"
        );
        assert_eq!(regions[0].len(), 12);
    }

    #[test]
    fn cluster_words_into_table_regions_empty_input_yields_no_regions() {
        assert!(cluster_words_into_table_regions(&[]).is_empty());
    }

    /// A four-column invoice whose rows are spaced widely enough to exceed the region gap
    /// threshold: the header forms one region and every line item becomes a one-row region of
    /// four words, below `MIN_TABLE_CANDIDATE_WORDS`, which the table branch then drops. After
    /// GH#1957 the aligned fragments attach to the header region instead of being discarded.
    fn widely_spaced_invoice_words() -> Vec<crate::table_core::HocrWord> {
        let columns = [100_u32, 500, 800, 1100];
        let mut words = Vec::new();
        for (text, column) in ["DESCRIPTION", "QTY", "UNIT", "LINE"].into_iter().zip(columns) {
            words.push(word_at(column, 100, 80, 30, text));
        }
        for (row, item) in ["Widget", "Gadget", "Doohickey"].into_iter().enumerate() {
            let top = 300 + row as u32 * 300;
            words.push(word_at(columns[0], top, 80, 30, item));
            words.push(word_at(columns[1], top, 40, 30, "10"));
            words.push(word_at(columns[2], top, 60, 30, "5.00"));
            words.push(word_at(columns[3], top, 80, 30, "50.00"));
        }
        words
    }

    #[test]
    fn cluster_words_into_table_regions_keeps_wide_aligned_rows_with_their_table() {
        let words = widely_spaced_invoice_words();

        let regions = cluster_words_into_table_regions(&words);

        assert_eq!(
            regions.len(),
            1,
            "the aligned line items must not become separate, dropped regions: {:?}",
            regions.iter().map(|region| region.len()).collect::<Vec<_>>()
        );
        assert_eq!(regions[0].len(), words.len());
    }

    /// GH#1957 end to end through the region the table branch reconstructs: the widely spaced
    /// invoice keeps its header and all three line items instead of only the header.
    #[test]
    fn widely_spaced_invoice_reconstructs_every_line_item() {
        let words = widely_spaced_invoice_words();
        let region = crate::table_core::cluster_words_into_table_regions(&words)
            .into_iter()
            .next()
            .expect("one region");

        let (table, _columns) = reconstruct_cleaned_table(&region, &TesseractConfig::default());

        assert_eq!(table.len(), 4, "header plus three line items: {table:?}");
        for item in ["Widget", "Gadget", "Doohickey"] {
            assert!(
                table.iter().flatten().any(|cell| cell.contains(item)),
                "{item} was lost: {table:?}"
            );
        }
        assert_eq!(
            table.iter().flatten().filter(|cell| cell.as_str() == "10").count(),
            3,
            "every line item's quantity must survive: {table:?}"
        );
    }

    fn invoice_rows_with_full_sized_fragments() -> Vec<crate::table_core::HocrWord> {
        let row = |top, values: &[(u32, u32, &str)]| {
            values
                .iter()
                .map(|&(left, width, text)| word_at(left, top, width, 20, text))
                .collect::<Vec<_>>()
        };
        let mut words = row(
            775,
            &[
                (157, 173, "DESCRIPTION"),
                (956, 50, "QTY"),
                (1179, 59, "UNIT"),
                (1248, 76, "PRICE"),
                (1498, 55, "LINE"),
                (1563, 80, "TOTAL"),
            ],
        );
        words.extend(row(
            857,
            &[
                (158, 113, "Widget"),
                (281, 77, "Premium"),
                (367, 116, "Blend"),
                (494, 77, "Large"),
                (978, 28, "10"),
                (1241, 84, "$45.00"),
                (1543, 100, "$450.00"),
            ],
        ));
        words.extend(row(
            949,
            &[
                (157, 150, "Cups"),
                (317, 65, "with"),
                (391, 17, "&"),
                (417, 50, "Lids"),
                (961, 45, "200"),
                (1256, 69, "$1.20"),
                (1543, 100, "$240.00"),
            ],
        ));
        words.extend(row(
            1041,
            &[
                (157, 107, "Cleaning"),
                (274, 88, "Tablets"),
                (993, 13, "5"),
                (1241, 84, "$18.50"),
                (1559, 84, "$92.50"),
            ],
        ));
        words
    }

    #[test]
    fn invoice_full_sized_rows_keep_four_columns_and_every_quantity() {
        let words = invoice_rows_with_full_sized_fragments();
        let regions = cluster_words_into_table_regions(&words);
        assert_eq!(regions.len(), 1, "one invoice table must stay one region: {regions:?}");

        let (grid, _) = reconstruct_cleaned_table(&regions[0], &TesseractConfig::default());
        assert_eq!(grid.len(), 4, "header and three item rows: {grid:?}");
        assert_eq!(grid[0], ["DESCRIPTION", "QTY", "UNIT PRICE", "LINE TOTAL"]);
        assert_eq!(grid[1][1], "10");
        assert_eq!(grid[2][1], "200");
        assert_eq!(grid[3], ["Cleaning Tablets", "5", "$18.50", "$92.50"]);
    }

    #[test]
    fn nearby_aligned_complete_tables_remain_separate() {
        let mut words = table_grid_words(100, 100, 2, 4);
        words.extend(table_grid_words(100, 270, 2, 4));

        let regions = cluster_words_into_table_regions(&words);
        assert_eq!(regions.len(), 2, "a table-sized gap separates two complete grids");
        assert_eq!(regions[0].len(), 8);
        assert_eq!(regions[1].len(), 8);
    }

    #[test]
    fn adjacent_aligned_tables_with_repeated_headers_remain_separate() {
        let mut first = invoice_rows_with_full_sized_fragments();
        first.truncate(13);
        let mut second = first.clone();
        for word in &mut second {
            word.top += 172;
        }
        first.extend(second);

        let regions = cluster_words_into_table_regions(&first);
        assert_eq!(regions.len(), 2, "a new header begins a separate aligned table");
        assert_eq!(regions[0].len(), 13);
        assert_eq!(regions[1].len(), 13);
    }

    const TSV_HEADER: &str =
        "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n";

    /// The last row's label is two words on one Tesseract line, and shading stretched the first
    /// word's box down past the table. Its row takes the line's typical box, so a bounding box
    /// built from those boxes ends above the centre of the stretched word as Tesseract read it,
    /// and the text rebuild printed that word a second time below the table.
    #[test]
    fn a_stretched_edge_row_word_is_inside_its_table_and_prints_once() {
        let tsv = format!(
            "{TSV_HEADER}\
5\t1\t1\t1\t1\t1\t100\t120\t90\t26\t90\tAlpha\n\
5\t1\t2\t1\t1\t1\t600\t120\t60\t26\t90\t10\n\
5\t1\t3\t1\t1\t1\t800\t120\t60\t26\t90\t20\n\
5\t1\t4\t1\t1\t1\t100\t170\t90\t26\t90\tBravo\n\
5\t1\t5\t1\t1\t1\t600\t170\t60\t26\t90\t30\n\
5\t1\t6\t1\t1\t1\t800\t170\t60\t26\t90\t40\n\
5\t1\t7\t1\t1\t1\t100\t220\t100\t62\t90\tCharlie\n\
5\t1\t7\t1\t1\t2\t210\t220\t80\t26\t90\tTail\n\
5\t1\t8\t1\t1\t1\t600\t220\t60\t26\t90\t50\n\
5\t1\t9\t1\t1\t1\t800\t220\t60\t26\t90\t60\n"
        );
        let words = extract_table_words_from_tsv(&tsv, 0.0, &[]).unwrap();
        let regions = table_regions(&words);
        assert_eq!(regions.len(), 1);
        let region = &regions[0];
        let cells = reconstruct_table_with_columns(&region.words, 20, 0.5).0;
        let tables = [OcrTable {
            markdown: table_to_markdown(&cells),
            cells,
            page_number: 1,
            bounding_box: Some(region.bounding_box()),
        }];

        let content = build_content_with_inline_tables(&tsv, &tables, 0.0);

        assert!(content.contains("Charlie Tail"), "the label shares its row: {content}");
        assert_eq!(
            content.matches("Charlie").count(),
            1,
            "the stretched word prints once, in the table: {content}"
        );
    }

    /// A last-column value arrives with a long underscore mark fused on. The table cuts the mark
    /// off, but the text rebuild reads the whole word, whose centre sits right of the value. The
    /// bounding box covers the whole word, so the value prints once, in the table.
    #[test]
    fn an_edge_value_cut_from_its_underscore_mark_prints_once() {
        let tsv = format!(
            "{TSV_HEADER}\
5\t1\t1\t1\t1\t1\t100\t120\t90\t26\t90\tAlpha\n\
5\t1\t2\t1\t1\t1\t600\t120\t60\t26\t90\t10\n\
5\t1\t3\t1\t1\t1\t800\t120\t60\t26\t90\t20\n\
5\t1\t4\t1\t1\t1\t100\t170\t90\t26\t90\tBravo\n\
5\t1\t5\t1\t1\t1\t600\t170\t60\t26\t90\t30\n\
5\t1\t6\t1\t1\t1\t800\t170\t320\t26\t90\t47______________\n\
5\t1\t7\t1\t1\t1\t100\t220\t90\t26\t90\tCharlie\n\
5\t1\t8\t1\t1\t1\t600\t220\t60\t26\t90\t50\n\
5\t1\t9\t1\t1\t1\t800\t220\t60\t26\t90\t60\n"
        );
        let words = extract_table_words_from_tsv(&tsv, 0.0, &[]).unwrap();
        let regions = table_regions(&words);
        assert_eq!(regions.len(), 1);
        let region = &regions[0];
        let cells = reconstruct_table_with_columns(&region.words, 20, 0.5).0;
        let tables = [OcrTable {
            markdown: table_to_markdown(&cells),
            cells,
            page_number: 1,
            bounding_box: Some(region.bounding_box()),
        }];

        let content = build_content_with_inline_tables(&tsv, &tables, 0.0);

        assert!(
            tables[0].cells.iter().flatten().any(|cell| cell == "47"),
            "the value sits in its table cell: {:?}",
            tables[0].cells
        );
        assert_eq!(
            content.matches("47").count(),
            1,
            "the value prints once, in the table: {content}"
        );
    }

    #[test]
    fn a_word_pushed_onto_a_table_region_widens_its_bounding_box() {
        let mut region = TableRegion {
            words: table_grid_words(0, 0, 2, 2),
            read_boxes: table_grid_words(0, 0, 2, 2),
        };
        let before = region.bounding_box();
        let below = HocrWord {
            text: "7".to_string(),
            left: before.left,
            top: before.bottom + 40,
            width: 10,
            height: 10,
            confidence: 90.0,
        };

        region.push(below.clone());

        assert_eq!(region.bounding_box().bottom, below.top + below.height);
        assert_eq!(region.words.len(), region.read_boxes.len());
    }

    fn text_element_with_font_size(text: &str, font_size: Option<f64>) -> crate::types::internal::InternalElement {
        let mut elem = crate::types::internal::InternalElement::text(
            ElementKind::OcrText {
                level: crate::types::OcrElementLevel::Block,
            },
            text,
            0,
        );
        if let Some(size) = font_size {
            elem.attributes
                .get_or_insert_with(Default::default)
                .insert(HOCR_FONT_SIZE_ATTRIBUTE.to_string(), size.to_string());
        }
        elem
    }

    #[test]
    fn flatten_hocr_elements_to_text_never_bakes_heading_syntax_regardless_of_font_size() {
        // Regression test for the `Plain`-format markdown leak: a large-font,
        // short, single-line paragraph used to be promoted to a `## ` heading
        // right in this string, and `OcrExtractionResult::content` (built
        // from this string) is consumed unrendered by some callers, so the
        // markdown syntax leaked into `Plain` output. This must fail against
        // the old behavior, which produced
        // "## Chapter One\n\nBody text at normal size." instead.
        let elements = vec![
            text_element_with_font_size("Chapter One", Some(28.0)),
            text_element_with_font_size("Body text at normal size.", Some(12.0)),
        ];

        let flattened = flatten_hocr_elements_to_text(&elements);

        assert_eq!(flattened, "Chapter One\n\nBody text at normal size.");
        assert!(
            !flattened.contains('#'),
            "flattened OCR text must contain no markdown heading syntax: {flattened:?}"
        );
    }

    fn paragraph_with_bbox(text: &str, x0: f64, y0: f64, x1: f64, y1: f64) -> crate::types::internal::InternalElement {
        let mut elem = crate::types::internal::InternalElement::text(ElementKind::Paragraph, text, 0);
        elem.bbox = Some(crate::types::extraction::BoundingBox { x0, y0, x1, y1 });
        elem
    }

    /// A table at a bbox whose cells are given explicitly, because `filter_elements_covered_by_tables`
    /// now decides per table whether its cells kept the covered lines' words (#1884), so the cells are
    /// part of every case's input and not incidental filler. ~keep
    fn table_at_with_cells(left: u32, top: u32, right: u32, bottom: u32, cells: Vec<Vec<String>>) -> OcrTable {
        OcrTable {
            markdown: table_to_markdown(&cells),
            cells,
            page_number: 1,
            bounding_box: Some(OcrTableBoundingBox {
                left,
                top,
                right,
                bottom,
            }),
        }
    }

    /// One row of cells from whitespace-separated text, for tables that must retain a paragraph's words.
    fn cells_of(text: &str) -> Vec<Vec<String>> {
        vec![text.split_whitespace().map(str::to_string).collect()]
    }

    #[test]
    fn filter_elements_covered_by_tables_keeps_paragraph_adjacent_to_table() {
        // Precision guard (#1571): a paragraph that merely overlaps a table's bbox edge,
        // with its centre outside the bbox, must survive -- the word-centre rule must not
        // over-delete prose that sits next to (not inside) a table.
        let mut document = InternalDocument::new("pdf");
        document.elements = vec![
            paragraph_with_bbox("Vehicle Maintenance Guide", 10.0, 0.0, 90.0, 15.0),
            paragraph_with_bbox("Apple 50 10 00", 10.0, 50.0, 90.0, 70.0),
        ];
        let tables = vec![table_at_with_cells(0, 40, 100, 140, cells_of("Apple 50 10 00"))];

        filter_elements_covered_by_tables(&mut document, &tables);

        assert_eq!(
            document.elements.len(),
            1,
            "only the paragraph centred inside the table bbox should be dropped"
        );
        assert_eq!(document.elements[0].text, "Vehicle Maintenance Guide");
        assert!(
            !document.ocr_text_claimed_by_tables,
            "the page keeps text outside the table"
        );
    }

    #[test]
    fn flatten_hocr_elements_to_text_leaves_content_unchanged_without_font_sizes() {
        let elements = vec![
            text_element_with_font_size("First paragraph.", None),
            text_element_with_font_size("Second paragraph.", None),
        ];

        let markdown = flatten_hocr_elements_to_text(&elements);

        assert_eq!(markdown, "First paragraph.\n\nSecond paragraph.");
    }

    #[test]
    fn test_is_all_languages() {
        assert!(is_all_languages("all"));
        assert!(is_all_languages("ALL"));
        assert!(is_all_languages("All"));
        assert!(is_all_languages("*"));
        assert!(!is_all_languages("eng"));
        assert!(!is_all_languages("eng+fra"));
        assert!(!is_all_languages(""));
    }

    #[test]
    fn test_resolve_config_language_passthrough() {
        let config = TesseractConfig {
            language: "eng".to_string(),
            ..TesseractConfig::default()
        };
        let resolved = resolve_config_language(&config).unwrap();
        assert!(resolved.is_none(), "non-wildcard should return None (no clone)");
    }

    #[test]
    fn test_compute_image_hash_deterministic() {
        let image_bytes = vec![1, 2, 3, 4, 5];

        let hash1 = crate::cache::blake3_hash_bytes(&image_bytes);
        let hash2 = crate::cache::blake3_hash_bytes(&image_bytes);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 32);
    }

    #[test]
    fn test_compute_image_hash_different_data() {
        let image_bytes1 = vec![1, 2, 3, 4, 5];
        let image_bytes2 = vec![5, 4, 3, 2, 1];

        let hash1 = crate::cache::blake3_hash_bytes(&image_bytes1);
        let hash2 = crate::cache::blake3_hash_bytes(&image_bytes2);

        assert_ne!(hash1, hash2);
    }

    fn invoice_word(text: &str, left: u32, top: u32, width: u32) -> crate::table_core::HocrWord {
        crate::table_core::HocrWord {
            text: text.to_string(),
            left,
            top,
            width,
            height: 40,
            confidence: 95.0,
        }
    }

    #[test]
    fn blank_quantity_retry_is_bounded_to_the_missing_quantity_cell() {
        let words = vec![
            invoice_word("DESCRIPTION", 279, 100, 250),
            invoice_word("QTY", 1_699, 100, 90),
            invoice_word("UNIT PRICE", 2_096, 100, 258),
            invoice_word("LINE TOTAL", 2_663, 100, 258),
            invoice_word("Espresso Beans", 279, 200, 400),
            invoice_word("10", 1_740, 200, 48),
            invoice_word("$45.00", 2_206, 200, 150),
            invoice_word("$450.00", 2_744, 200, 175),
            invoice_word("Cups", 279, 300, 100),
            invoice_word("200", 1_709, 300, 75),
            invoice_word("$1.20", 2_233, 300, 125),
            invoice_word("$240.00", 2_744, 300, 175),
            invoice_word("Cleaning Tablets", 279, 400, 360),
            invoice_word("$18.50", 2_206, 400, 150),
            invoice_word("$92.50", 2_772, 400, 150),
        ];
        let table = vec![
            vec!["DESCRIPTION", "QTY", "UNIT PRICE", "LINE TOTAL"],
            vec!["Espresso Beans", "10", "$45.00", "$450.00"],
            vec!["Cups", "200", "$1.20", "$240.00"],
            vec!["Cleaning Tablets", "", "$18.50", "$92.50"],
        ]
        .into_iter()
        .map(|row| row.into_iter().map(str::to_string).collect())
        .collect::<Vec<Vec<String>>>();

        let (quantity_column, blank_row) =
            quantity_retry_column_index(&table).expect("fixture qualifies for one retry");
        let region = quantity_retry_region(
            &words,
            quantity_column,
            blank_row,
            &[120, 220, 320, 420],
            &[279, 1_709, 2_219, 2_744],
            3_200,
            4_089,
        )
        .expect("captured invoice has one bounded retry region");

        assert_eq!(region.row, 3);
        assert_eq!(region.column, 1);
        assert_eq!((region.left, region.top), (1_659, 380));
        assert_eq!((region.width, region.height), (170, 80));
    }

    #[test]
    fn blank_quantity_retry_requires_two_recognized_quantity_rows() {
        let table = vec![
            vec!["DESCRIPTION", "QTY", "UNIT PRICE", "LINE TOTAL"],
            vec!["Espresso Beans", "10", "$45.00", "$450.00"],
            vec!["Cleaning Tablets", "", "$18.50", "$92.50"],
        ]
        .into_iter()
        .map(|row| row.into_iter().map(str::to_string).collect())
        .collect::<Vec<Vec<String>>>();

        assert!(quantity_retry_column_index(&table).is_none());
    }

    #[test]
    fn blank_quantity_retry_skips_tables_with_multiple_blank_quantities() {
        let table = vec![
            vec!["DESCRIPTION", "QTY", "PRICE"],
            vec!["Item A", "10", "$10.00"],
            vec!["Item B", "20", "$20.00"],
            vec!["Shipping", "", "$25.00"],
            vec!["Tax", "", "$5.00"],
        ]
        .into_iter()
        .map(|row| row.into_iter().map(str::to_string).collect())
        .collect::<Vec<Vec<String>>>();

        assert!(quantity_retry_column_index(&table).is_none());
    }

    #[test]
    fn blank_quantity_retry_clamps_crop_to_adjacent_cells() {
        let words = vec![
            invoice_word("DESCRIPTION", 100, 100, 160),
            invoice_word("QTY", 500, 100, 40),
            invoice_word("PRICE", 560, 100, 90),
            invoice_word("Item A", 100, 180, 100),
            invoice_word("10", 500, 180, 40),
            invoice_word("$10", 560, 180, 60),
            invoice_word("Missing", 100, 210, 120),
            invoice_word("$5", 560, 210, 50),
            invoice_word("Item B", 100, 240, 100),
            invoice_word("20", 500, 240, 40),
            invoice_word("$20", 560, 240, 60),
        ];
        let table = vec![
            vec!["DESCRIPTION", "QTY", "PRICE"],
            vec!["Item A", "10", "$10"],
            vec!["Missing", "", "$5"],
            vec!["Item B", "20", "$20"],
        ]
        .into_iter()
        .map(|row| row.into_iter().map(str::to_string).collect())
        .collect::<Vec<Vec<String>>>();
        let (quantity_column, blank_row) =
            quantity_retry_column_index(&table).expect("fixture qualifies for one retry");

        let region = quantity_retry_region(
            &words,
            quantity_column,
            blank_row,
            &[120, 200, 230, 260],
            &[100, 500, 560],
            800,
            600,
        )
        .expect("tight table has one bounded retry region");

        assert_eq!((region.left, region.top), (460, 215));
        assert_eq!((region.width, region.height), (70, 30));
    }

    #[test]
    fn non_quantity_table_does_not_enter_retry_geometry() {
        let table = vec![
            vec!["NAME".to_string(), "PRICE".to_string()],
            vec!["Item A".to_string(), "$10".to_string()],
            vec!["Item B".to_string(), "$20".to_string()],
        ];

        assert!(quantity_retry_column_index(&table).is_none());
    }

    #[test]
    fn quantity_retry_text_accepts_only_a_short_positive_integer() {
        assert_eq!(parse_retry_quantity(" 5\n"), Some("5"));
        assert_eq!(parse_retry_quantity("200"), Some("200"));
        assert_eq!(parse_retry_quantity(""), None);
        assert_eq!(parse_retry_quantity("$5"), None);
        assert_eq!(parse_retry_quantity("5.0"), None);
        assert_eq!(parse_retry_quantity("000000000"), None);
    }

    #[test]
    fn test_log_ci_debug_disabled() {
        log_ci_debug(false, "test_stage", || "test message".to_string());
    }

    #[test]
    fn test_log_ci_debug_enabled() {
        log_ci_debug(true, "test_stage", || "test message".to_string());
    }

    #[test]
    fn test_process_image_file_nonexistent() {
        let temp_dir = tempdir().unwrap();
        let cache = OcrCache::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = TesseractConfig {
            output_format: "text".to_string(),
            enable_table_detection: false,
            use_cache: false,
            ..TesseractConfig::default()
        };

        let api_pool = TesseractApiPool::new();
        let result = process_image_file_with_cache("/nonexistent/file.png", &config, &cache, &api_pool, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to read file"));
    }

    #[test]
    fn test_process_image_invalid_image_data() {
        let temp_dir = tempdir().unwrap();
        let cache = OcrCache::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = TesseractConfig {
            output_format: "text".to_string(),
            enable_table_detection: false,
            use_cache: false,
            ..TesseractConfig::default()
        };

        let invalid_data = vec![0, 1, 2, 3, 4];
        let api_pool = TesseractApiPool::new();
        let result = process_image_with_cache(&invalid_data, &config, &cache, &api_pool, None);

        assert!(result.is_err());
    }

    /// Builds a tiny valid PNG so `process_image_with_cache` runs Tesseract on real
    /// (if content-free) image bytes instead of erroring out on invalid data.
    fn tiny_test_png_bytes() -> Vec<u8> {
        use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
        use std::io::Cursor;

        let img: RgbImage = ImageBuffer::from_pixel(32, 32, Rgb([255, 255, 255]));
        let mut bytes: Vec<u8> = Vec::new();
        img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png).unwrap();
        bytes
    }

    /// Regression test for #687 PART 2: `TesseractConfig::use_cache = false` must be an
    /// end-to-end bypass, not just a lookup skip — it must neither read a pre-existing
    /// entry nor write a new one. Uses a real (non-mocked) `TesseractAPI`, matching the
    /// pattern of `config::tests::test_apply_tesseract_variables_enables_hocr_font_info`:
    /// gracefully skips when no Tesseract/tessdata is available in this environment.
    ///
    /// This exercises a genuinely new code path (an explicit cache pre-seed followed by a
    /// `use_cache: false` call) that has no equivalent before this change, so there is
    /// nothing "unfixed" to run it against — the bypass plumbing this proves either exists
    /// or the test cannot be written.
    // `#[serial]`: this test resolves tessdata through the default (no-override) path, which
    // reads the process-global `XBERG_CACHE_DIR`/`TESSDATA_PREFIX` env vars that
    // `tesseract_backend::tests` mutates with `std::env::set_var` under its own `#[serial]`
    // tests. Joining the same lock group prevents this test from resolving a directory that
    // mutation is deleting mid-scan (an uncaught C++ `filesystem_error` aborting the whole
    // test process, not a logic bug in the code under test). ~keep
    #[test]
    #[serial]
    fn process_image_with_cache_does_not_read_or_write_the_cache_when_use_cache_is_false() {
        let api = match xberg_tesseract::TesseractAPI::new() {
            Ok(api) => api,
            Err(_) => return, // no Tesseract/Leptonica available in this environment
        };
        if api.init("", "eng").is_err() {
            return; // no "eng" tessdata available in this environment
        }

        let temp_dir = tempdir().unwrap();
        let cache = OcrCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let image_bytes = tiny_test_png_bytes();
        let config = TesseractConfig {
            output_format: "text".to_string(),
            enable_table_detection: false,
            use_cache: false,
            ..TesseractConfig::default()
        };

        // Pre-seed the exact entry `process_image_resolved` would look up if caching were
        // enabled, carrying an obviously-wrong marker. If the bypass ever regresses into a
        // read, this is what would come back instead of a fresh OCR result.
        let image_hash = crate::cache::blake3_hash_bytes(&image_bytes);
        let languages: Vec<String> = config.language.split('+').map(|lang| lang.trim().to_string()).collect();
        let resolved_tessdata_path =
            resolve_tessdata_path(&languages, config.tessdata_path.as_deref()).expect("tessdata must resolve");
        let config_str = hash_config(&config, &resolved_tessdata_path);
        let marker = OcrExtractionResult {
            content: "STALE MARKER: use_cache=false must never return this".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: Vec::new(),
            ocr_elements: None,
            internal_document: None,
        };
        cache
            .set_cached_result(&image_hash, "tesseract", &config_str, None, &marker)
            .unwrap();

        let api_pool = TesseractApiPool::new();
        let result = process_image_with_cache(&image_bytes, &config, &cache, &api_pool, None)
            .expect("OCR on a valid image must succeed");

        assert_ne!(
            result.content, marker.content,
            "use_cache=false must not read the pre-seeded cache entry"
        );

        let stats = cache.get_stats().unwrap();
        assert_eq!(
            stats.total_files, 1,
            "use_cache=false must not write a new cache entry (only the pre-seeded marker file may exist)"
        );
    }

    /// Copies a real, working `eng.traineddata` (resolved the same way production OCR does)
    /// into a fresh temp directory, so tests can build two DIFFERENT resolved tessdata
    /// directories that both OCR successfully, without depending on any specific host path.
    /// Returns `None` when no Tesseract/tessdata is available in this environment.
    /// Sources real `eng.traineddata` bytes from the sibling `xberg-tesseract` build's own
    /// `OUT_DIR`, mirroring `tesseract_backend::tests::real_eng_traineddata_bytes_from_sibling_build_dir`.
    fn real_eng_traineddata_bytes_from_sibling_build_dir() -> Option<Vec<u8>> {
        let this_out_dir = std::path::PathBuf::from(env!("OUT_DIR"));
        let build_dir = this_out_dir.parent()?.parent()?;
        let mut entries: Vec<_> = std::fs::read_dir(build_dir).ok()?.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let name = entry.file_name();
            if !name.to_string_lossy().starts_with("xberg-tesseract-") {
                continue;
            }
            let candidate = entry.path().join("out").join("eng.traineddata");
            if let Ok(bytes) = std::fs::read(&candidate) {
                return Some(bytes);
            }
        }
        None
    }

    fn copy_working_eng_tessdata_into(dest_dir: &std::path::Path) -> Option<()> {
        let bytes = real_eng_traineddata_bytes_from_sibling_build_dir()?;
        std::fs::write(dest_dir.join("eng.traineddata"), bytes).ok()
    }

    /// Regression test for #1787 part 2: two `TesseractConfig`s that are identical except for
    /// resolving to different tessdata directories (via the `tessdata_path` override here, which
    /// exercises the same `resolve_tessdata_path` call `TESSDATA_PREFIX` also goes through) must
    /// not share a cache entry. Before the fix, `hash_config` only ever saw `config.tessdata_path`
    /// itself, so this collided and the second call silently read the first directory's result.
    #[test]
    fn process_image_with_cache_does_not_share_entries_across_different_resolved_tessdata_directories() {
        let dir_a = tempdir().unwrap();
        let dir_b = tempdir().unwrap();
        if copy_working_eng_tessdata_into(dir_a.path()).is_none()
            || copy_working_eng_tessdata_into(dir_b.path()).is_none()
        {
            return; // no Tesseract/tessdata available in this environment
        }

        let cache_dir = tempdir().unwrap();
        let cache = OcrCache::new(Some(cache_dir.path().to_path_buf())).unwrap();
        let api_pool = TesseractApiPool::new();
        let image_bytes = tiny_test_png_bytes();

        let config_a = TesseractConfig {
            output_format: "text".to_string(),
            enable_table_detection: false,
            use_cache: true,
            tessdata_path: Some(dir_a.path().to_path_buf()),
            ..TesseractConfig::default()
        };
        let config_b = TesseractConfig {
            tessdata_path: Some(dir_b.path().to_path_buf()),
            ..config_a.clone()
        };

        process_image_with_cache(&image_bytes, &config_a, &cache, &api_pool, None)
            .expect("OCR against dir_a must succeed");
        process_image_with_cache(&image_bytes, &config_b, &cache, &api_pool, None)
            .expect("OCR against dir_b must succeed");

        let stats = cache.get_stats().unwrap();
        assert_eq!(
            stats.total_files, 2,
            "two different resolved tessdata directories must write two separate cache entries \
             (#1787), not share one"
        );
    }

    /// Negative control for the test above: it must not become so specific that two calls
    /// resolving to the SAME tessdata directory stop sharing a cache entry.
    #[test]
    fn process_image_with_cache_still_shares_entries_for_the_same_resolved_tessdata_directory() {
        let dir_a = tempdir().unwrap();
        if copy_working_eng_tessdata_into(dir_a.path()).is_none() {
            return; // no Tesseract/tessdata available in this environment
        }

        let cache_dir = tempdir().unwrap();
        let cache = OcrCache::new(Some(cache_dir.path().to_path_buf())).unwrap();
        let api_pool = TesseractApiPool::new();
        let image_bytes = tiny_test_png_bytes();

        let config = TesseractConfig {
            output_format: "text".to_string(),
            enable_table_detection: false,
            use_cache: true,
            tessdata_path: Some(dir_a.path().to_path_buf()),
            ..TesseractConfig::default()
        };

        process_image_with_cache(&image_bytes, &config, &cache, &api_pool, None).expect("first OCR call must succeed");
        process_image_with_cache(&image_bytes, &config, &cache, &api_pool, None).expect("second OCR call must succeed");

        let stats = cache.get_stats().unwrap();
        assert_eq!(
            stats.total_files, 1,
            "two calls resolving to the same tessdata directory must share one cache entry, \
             not write a second"
        );
    }

    #[test]
    fn test_preprocess_pix_produces_grayscale_with_valid_resolution() {
        let width = 32;
        let height = 32;
        let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
        for y in 0..height {
            for x in 0..width {
                let value = if (x + y) % 2 == 0 { 32 } else { 224 };
                rgb_data.extend_from_slice(&[value, value, value]);
            }
        }
        let mut pix = xberg_tesseract::Pix::from_raw_rgb(&rgb_data, width, height).unwrap();
        pix.set_resolution(300, 300).unwrap();

        let processed = preprocess_pix(pix, &preprocessing_config()).unwrap();

        assert_eq!(processed.width(), width as i32);
        assert_eq!(processed.height(), height as i32);
        assert_eq!(processed.depth(), 1);
        assert_eq!(processed.get_resolution().unwrap(), (300, 300));
    }

    fn preprocessing_fixture() -> (Vec<u8>, u32, u32) {
        const WIDTH: u32 = 640;
        const HEIGHT: u32 = 480;
        const CHANNELS: usize = 3;
        let mut rgb_data = Vec::with_capacity(WIDTH as usize * HEIGHT as usize * CHANNELS);

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let background = 176 + ((x * 48) / WIDTH) as u8;
                let skewed_line = (40..=390).step_by(50).any(|line_y| y.abs_diff(line_y + x / 16) <= 2);
                let noise = (x * 37 + y * 19).is_multiple_of(97);
                let value = if noise {
                    16
                } else if skewed_line && (20..620).contains(&x) {
                    72
                } else {
                    background
                };
                rgb_data.extend_from_slice(&[value, value, value]);
            }
        }

        (rgb_data, WIDTH, HEIGHT)
    }

    fn preprocessing_config() -> crate::types::ImagePreprocessingConfig {
        crate::types::ImagePreprocessingConfig {
            target_dpi: 300,
            auto_rotate: false,
            deskew: false,
            denoise: false,
            contrast_enhance: false,
            binarization_method: "otsu".to_string(),
            invert_colors: false,
            normalize_shaded_rows: false,
        }
    }

    fn preprocess_fixture_with_config(config: &crate::types::ImagePreprocessingConfig) -> xberg_tesseract::Pix {
        let (rgb_data, width, height) = preprocessing_fixture();
        preprocess_rgb_with_config(rgb_data, width, height, config)
    }

    fn preprocess_rgb_with_config(
        rgb_data: Vec<u8>,
        width: u32,
        height: u32,
        config: &crate::types::ImagePreprocessingConfig,
    ) -> xberg_tesseract::Pix {
        let pix = xberg_tesseract::Pix::from_raw_rgb(&rgb_data, width, height).unwrap();
        preprocess_pix(pix, config).unwrap()
    }

    fn sampled_pixel_signature(pix: &xberg_tesseract::Pix) -> Vec<u32> {
        const SAMPLE_STEP: usize = 8;
        let mut signature = vec![pix.width() as u32, pix.height() as u32, pix.depth() as u32];
        for y in (0..pix.height()).step_by(SAMPLE_STEP) {
            for x in (0..pix.width()).step_by(SAMPLE_STEP) {
                let sample = pix.clip_rectangle(x, y, 1, 1).unwrap();
                let (mean, _) = sample.grayscale_stats(1, 1).unwrap();
                signature.push(mean.round() as u32);
            }
        }
        signature
    }

    fn binary_pixel_value(pix: &xberg_tesseract::Pix, x: i32, y: i32) -> u32 {
        let sample = pix.clip_rectangle(x, y, 1, 1).unwrap();
        let (mean, _) = sample.grayscale_stats(1, 1).unwrap();
        mean.round() as u32
    }

    #[test]
    fn should_change_preprocessed_pixels_when_deskew_is_enabled() {
        let disabled = preprocessing_config();
        let enabled = crate::types::ImagePreprocessingConfig {
            deskew: true,
            ..disabled.clone()
        };

        let disabled_signature = sampled_pixel_signature(&preprocess_fixture_with_config(&disabled));
        let enabled_signature = sampled_pixel_signature(&preprocess_fixture_with_config(&enabled));

        assert_ne!(
            enabled_signature, disabled_signature,
            "deskew must affect the OCR raster"
        );
    }

    #[test]
    fn should_change_preprocessed_pixels_when_denoise_is_enabled() {
        let disabled = preprocessing_config();
        let enabled = crate::types::ImagePreprocessingConfig {
            denoise: true,
            ..disabled.clone()
        };

        let disabled_signature = sampled_pixel_signature(&preprocess_fixture_with_config(&disabled));
        let enabled_signature = sampled_pixel_signature(&preprocess_fixture_with_config(&enabled));

        assert_ne!(
            enabled_signature, disabled_signature,
            "denoise must affect the OCR raster"
        );
    }

    #[test]
    fn should_change_preprocessed_pixels_when_contrast_enhance_is_enabled() {
        let disabled = crate::types::ImagePreprocessingConfig {
            binarization_method: "adaptive".to_string(),
            ..preprocessing_config()
        };
        let enabled = crate::types::ImagePreprocessingConfig {
            contrast_enhance: true,
            ..disabled.clone()
        };

        let disabled_signature = sampled_pixel_signature(&preprocess_fixture_with_config(&disabled));
        let enabled_signature = sampled_pixel_signature(&preprocess_fixture_with_config(&enabled));

        assert_ne!(
            enabled_signature, disabled_signature,
            "contrast enhancement must affect the OCR raster"
        );
    }

    #[test]
    fn should_change_preprocessed_pixels_when_binarization_method_changes() {
        let otsu = preprocessing_config();
        let adaptive = crate::types::ImagePreprocessingConfig {
            binarization_method: "adaptive".to_string(),
            ..otsu.clone()
        };

        let otsu_signature = sampled_pixel_signature(&preprocess_fixture_with_config(&otsu));
        let adaptive_signature = sampled_pixel_signature(&preprocess_fixture_with_config(&adaptive));

        assert_ne!(
            adaptive_signature, otsu_signature,
            "binarization_method must select a distinct preprocessing operation"
        );
    }

    #[test]
    fn should_change_preprocessed_pixels_when_sauvola_is_selected() {
        const WIDTH: u32 = 256;
        const HEIGHT: u32 = 128;
        let mut rgb_data = Vec::with_capacity(WIDTH as usize * HEIGHT as usize * RGB_CHANNEL_COUNT);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let background = 72 + ((x * 168) / WIDTH) as u8;
                let is_text = (20..=100).step_by(20).any(|line| y.abs_diff(line) <= 1) && (8..248).contains(&x);
                let value = if is_text {
                    background.saturating_sub(32)
                } else {
                    background
                };
                rgb_data.extend_from_slice(&[value, value, value]);
            }
        }
        let otsu = preprocessing_config();
        let sauvola = crate::types::ImagePreprocessingConfig {
            binarization_method: "sauvola".to_string(),
            ..otsu.clone()
        };

        let otsu_signature =
            sampled_pixel_signature(&preprocess_rgb_with_config(rgb_data.clone(), WIDTH, HEIGHT, &otsu));
        let sauvola_signature = sampled_pixel_signature(&preprocess_rgb_with_config(rgb_data, WIDTH, HEIGHT, &sauvola));

        assert_ne!(
            sauvola_signature, otsu_signature,
            "Sauvola must select a distinct operation"
        );
    }

    #[test]
    fn test_prepare_ocr_image_without_config_preserves_shadowed_rgb() {
        let rgb_data = vec![0, 1, 2, 3, 4, 5];

        let prepared = prepare_ocr_image(rgb_data.clone(), 2, 1, None, None, false, None, false);

        assert_eq!(prepared.data, rgb_data);
        assert_eq!(prepared.width, 2);
        assert_eq!(prepared.height, 1);
        assert_eq!(prepared.source_dpi, RAW_IMAGE_SOURCE_DPI);
        assert!(!prepared.apply_pix_preprocessing);
    }

    /// GH#1894: a page the PDF OCR route already knows is a whole-page scan must take the
    /// default preprocessing regardless of how dark the raster reads, rather than being judged
    /// by `should_apply_default_preprocessing`'s pixel-brightness heuristic. This all-black
    /// fixture fails that heuristic outright (mean luminance 0.0), so the only thing that can
    /// route it through preprocessing is `known_full_page_scan`.
    ///
    /// Negative control (flip `true` to `false` on the call below): fails with
    /// `assertion failed: prepared.apply_pix_preprocessing` — proving the pixel test alone
    /// would have rejected this fixture, which is exactly the GH#1894 defect.
    #[test]
    fn should_apply_default_preprocessing_for_a_known_scan_page_however_dark() {
        const SAMPLE_PIXEL_COUNT: usize = 16;
        let rgb_data = vec![0u8; SAMPLE_PIXEL_COUNT * RGB_CHANNEL_COUNT];
        assert!(
            !should_apply_default_preprocessing(&rgb_data, SAMPLE_PIXEL_COUNT as u32, 1),
            "fixture must fail the pixel-brightness heuristic on its own"
        );

        let prepared = prepare_ocr_image(rgb_data, SAMPLE_PIXEL_COUNT as u32, 1, None, None, false, None, true);

        assert!(
            prepared.apply_pix_preprocessing,
            "a known scan page must take default preprocessing even when the pixels read as dark"
        );
    }

    /// Control for the test above: the identical dark fixture with no scan-detection signal
    /// (`known_full_page_scan: false`, the value every non-PDF caller passes) must keep the
    /// pre-GH#1894 behaviour and skip preprocessing, so bare images handed to the standalone
    /// image extractor are unaffected by this fix.
    ///
    /// Negative control (flip `false` to `true` on the call below): fails with
    /// `assertion failed: !prepared.apply_pix_preprocessing` — proving this test actually
    /// distinguishes the two code paths rather than passing regardless of the flag.
    #[test]
    fn should_keep_pixel_test_for_a_dark_image_with_no_scan_signal() {
        const SAMPLE_PIXEL_COUNT: usize = 16;
        let rgb_data = vec![0u8; SAMPLE_PIXEL_COUNT * RGB_CHANNEL_COUNT];

        let prepared = prepare_ocr_image(rgb_data, SAMPLE_PIXEL_COUNT as u32, 1, None, None, false, None, false);

        assert!(
            !prepared.apply_pix_preprocessing,
            "a bare dark image with no scan signal must still be judged by pixel brightness"
        );
    }

    #[test]
    fn test_clean_white_rgb_selects_default_preprocessing() {
        const SAMPLE_PIXEL_COUNT: usize = 16;
        let rgb_data = vec![u8::MAX; SAMPLE_PIXEL_COUNT * RGB_CHANNEL_COUNT];

        assert!(should_apply_default_preprocessing(
            &rgb_data,
            SAMPLE_PIXEL_COUNT as u32,
            1
        ));
    }

    const CONTRAST_FIXTURE_WIDTH: u32 = 512;
    const CONTRAST_FIXTURE_HEIGHT: u32 = 128;
    const CONTRAST_FIXTURE_FILL_WIDTH: u32 = 80;
    const PALE_TEXT: [u8; RGB_CHANNEL_COUNT] = [200, 220, 245];
    const DARK_TEXT: [u8; RGB_CHANNEL_COUNT] = [30, 30, 30];
    const BRIGHT_BLUE_FILL: [u8; RGB_CHANNEL_COUNT] = [210, 220, 240];

    fn glyph_mask(x: u32, y: u32, offset: u32) -> bool {
        const TOP: u32 = 48;
        const HEIGHT: u32 = 24;
        const WIDTH: u32 = 20;
        const STROKE: u32 = 2;
        for left in [4 + offset, 30 + offset, 56 + offset] {
            let local_x = x.checked_sub(left);
            let local_y = y.checked_sub(TOP);
            if let (Some(local_x), Some(local_y)) = (local_x, local_y)
                && local_x < WIDTH
                && local_y < HEIGHT
                && (!(STROKE..WIDTH - STROKE).contains(&local_x)
                    || (HEIGHT / 2..HEIGHT / 2 + STROKE).contains(&local_y))
            {
                return true;
            }
        }
        false
    }

    fn contrast_fixture(text: [u8; RGB_CHANNEL_COUNT], fill: bool, offset: u32) -> (Vec<u8>, usize) {
        let mut rgb_data =
            Vec::with_capacity(CONTRAST_FIXTURE_WIDTH as usize * CONTRAST_FIXTURE_HEIGHT as usize * RGB_CHANNEL_COUNT);
        let mut glyph_pixels = 0usize;
        for y in 0..CONTRAST_FIXTURE_HEIGHT {
            for x in 0..CONTRAST_FIXTURE_WIDTH {
                let pixel = if glyph_mask(x, y, offset) {
                    glyph_pixels += 1;
                    text
                } else if fill && x < CONTRAST_FIXTURE_FILL_WIDTH {
                    BRIGHT_BLUE_FILL
                } else {
                    [u8::MAX; RGB_CHANNEL_COUNT]
                };
                rgb_data.extend_from_slice(&pixel);
            }
        }
        (rgb_data, glyph_pixels)
    }

    fn aggregate_foreground_contrast(rgb_data: &[u8]) -> f64 {
        let mut background_sum = 0.0;
        let mut background_count = 0usize;
        let mut foreground_sum = 0.0;
        let mut foreground_count = 0usize;
        for pixel in rgb_data.as_chunks::<RGB_CHANNEL_COUNT>().0 {
            let luminance = pixel.iter().map(|channel| f64::from(*channel)).sum::<f64>()
                / (RGB_CHANNEL_MAX * RGB_CHANNEL_COUNT as f64);
            if luminance >= CLEAN_PAGE_LIGHT_PIXEL_THRESHOLD {
                background_sum += luminance;
                background_count += 1;
            } else {
                foreground_sum += luminance;
                foreground_count += 1;
            }
        }
        background_sum / background_count as f64 - foreground_sum / foreground_count as f64
    }

    #[test]
    fn should_preserve_pale_glyphs_without_implicit_otsu() {
        let (rgb_data, glyph_pixels) = contrast_fixture(PALE_TEXT, false, 0);
        assert_eq!(
            glyph_pixels, 384,
            "fixture must contain the intended nonempty glyph mask"
        );

        let prepared = prepare_ocr_image(
            rgb_data.clone(),
            CONTRAST_FIXTURE_WIDTH,
            CONTRAST_FIXTURE_HEIGHT,
            None,
            None,
            false,
            Some(300.0),
            false,
        );

        assert!(!prepared.apply_pix_preprocessing);
        assert!(prepared.preprocessing.is_none());
        assert!(prepared.image_preprocessing.is_none());
        assert_eq!(
            prepared.data, rgb_data,
            "implicit preprocessing must preserve pale glyph pixels"
        );
    }

    #[test]
    fn should_apply_implicit_otsu_to_dark_glyphs_over_bright_fill_at_every_sample_phase() {
        for offset in 0..DEFAULT_PREPROCESSING_SAMPLE_STRIDE as u32 {
            let (rgb_data, glyph_pixels) = contrast_fixture(DARK_TEXT, true, offset);
            assert_eq!(glyph_pixels, 384, "offset {offset} changed the glyph population");
            assert!(
                aggregate_foreground_contrast(&rgb_data) < CLEAN_PAGE_MIN_FOREGROUND_CONTRAST,
                "fixture must fail the legacy aggregate-contrast gate at offset {offset}"
            );

            let prepared = prepare_ocr_image(
                rgb_data,
                CONTRAST_FIXTURE_WIDTH,
                CONTRAST_FIXTURE_HEIGHT,
                None,
                None,
                false,
                Some(300.0),
                false,
            );

            assert!(
                prepared.apply_pix_preprocessing,
                "dark glyphs were missed at offset {offset}"
            );
            assert_eq!(prepared.width, CONTRAST_FIXTURE_WIDTH);
            assert_eq!(prepared.height, CONTRAST_FIXTURE_HEIGHT);
            assert_eq!(
                prepared
                    .preprocessing
                    .as_ref()
                    .map(|config| config.binarization_method.as_str()),
                Some("otsu")
            );
        }
    }

    #[test]
    fn should_reject_scattered_speckles_with_the_same_dark_pixel_count_as_glyphs() {
        let (mut rgb_data, glyph_pixels) = contrast_fixture(BRIGHT_BLUE_FILL, true, 0);
        let mut speckles = 0usize;
        'rows: for y in (0..CONTRAST_FIXTURE_HEIGHT).step_by(2) {
            for x in (0..CONTRAST_FIXTURE_WIDTH).step_by(DEFAULT_PREPROCESSING_SAMPLE_STRIDE) {
                let pixel_index = (y as usize * CONTRAST_FIXTURE_WIDTH as usize + x as usize) * RGB_CHANNEL_COUNT;
                rgb_data[pixel_index..pixel_index + RGB_CHANNEL_COUNT].copy_from_slice(&DARK_TEXT);
                speckles += 1;
                if speckles == glyph_pixels {
                    break 'rows;
                }
            }
        }
        assert_eq!(
            speckles, glyph_pixels,
            "control must have the same dark-pixel population"
        );

        let prepared = prepare_ocr_image(
            rgb_data.clone(),
            CONTRAST_FIXTURE_WIDTH,
            CONTRAST_FIXTURE_HEIGHT,
            None,
            None,
            false,
            Some(300.0),
            false,
        );
        assert!(
            !prepared.apply_pix_preprocessing,
            "isolated speckles are not text structure"
        );
        assert_eq!(prepared.data, rgb_data);
    }

    #[test]
    fn should_reject_a_contiguous_blob_with_the_same_dark_pixel_count_as_glyphs() {
        const BLOB_WIDTH: u32 = 16;
        const BLOB_HEIGHT: u32 = 24;
        let (mut rgb_data, glyph_pixels) = contrast_fixture(BRIGHT_BLUE_FILL, true, 0);
        assert_eq!(BLOB_WIDTH as usize * BLOB_HEIGHT as usize, glyph_pixels);

        for y in 48..48 + BLOB_HEIGHT {
            for x in 4..4 + BLOB_WIDTH {
                let pixel_index = (y as usize * CONTRAST_FIXTURE_WIDTH as usize + x as usize) * RGB_CHANNEL_COUNT;
                rgb_data[pixel_index..pixel_index + RGB_CHANNEL_COUNT].copy_from_slice(&DARK_TEXT);
            }
        }

        let prepared = prepare_ocr_image(
            rgb_data.clone(),
            CONTRAST_FIXTURE_WIDTH,
            CONTRAST_FIXTURE_HEIGHT,
            None,
            None,
            false,
            Some(300.0),
            false,
        );
        assert!(!prepared.apply_pix_preprocessing, "a solid blob is not text structure");
        assert_eq!(prepared.data, rgb_data);
    }

    #[test]
    fn should_preserve_three_narrow_solid_glyphs_with_the_same_dark_pixel_count() {
        const GLYPH_WIDTH: u32 = 4;
        const GLYPH_HEIGHT: u32 = 32;
        let (mut rgb_data, glyph_pixels) = contrast_fixture(BRIGHT_BLUE_FILL, true, 0);
        assert_eq!(3 * GLYPH_WIDTH as usize * GLYPH_HEIGHT as usize, glyph_pixels);

        for left in [4, 12, 20] {
            for y in 44..44 + GLYPH_HEIGHT {
                for x in left..left + GLYPH_WIDTH {
                    let pixel_index = (y as usize * CONTRAST_FIXTURE_WIDTH as usize + x as usize) * RGB_CHANNEL_COUNT;
                    rgb_data[pixel_index..pixel_index + RGB_CHANNEL_COUNT].copy_from_slice(&DARK_TEXT);
                }
            }
        }

        let prepared = prepare_ocr_image(
            rgb_data,
            CONTRAST_FIXTURE_WIDTH,
            CONTRAST_FIXTURE_HEIGHT,
            None,
            None,
            false,
            Some(300.0),
            false,
        );
        assert!(
            prepared.apply_pix_preprocessing,
            "narrow solid glyphs remain text structure"
        );
    }

    #[test]
    fn should_reject_a_small_defect_among_equal_count_scattered_speckles() {
        const DEFECT_PIXELS: usize = 4;
        let (mut rgb_data, glyph_pixels) = contrast_fixture(BRIGHT_BLUE_FILL, true, 0);
        for y in 0..2 {
            for x in 0..2 {
                let pixel_index = (y * CONTRAST_FIXTURE_WIDTH as usize + x) * RGB_CHANNEL_COUNT;
                rgb_data[pixel_index..pixel_index + RGB_CHANNEL_COUNT].copy_from_slice(&DARK_TEXT);
            }
        }

        let mut speckles = 0usize;
        'rows: for y in (4..CONTRAST_FIXTURE_HEIGHT).step_by(2) {
            for x in (4..CONTRAST_FIXTURE_WIDTH).step_by(DEFAULT_PREPROCESSING_SAMPLE_STRIDE) {
                let pixel_index = (y as usize * CONTRAST_FIXTURE_WIDTH as usize + x as usize) * RGB_CHANNEL_COUNT;
                rgb_data[pixel_index..pixel_index + RGB_CHANNEL_COUNT].copy_from_slice(&DARK_TEXT);
                speckles += 1;
                if speckles + DEFECT_PIXELS == glyph_pixels {
                    break 'rows;
                }
            }
        }
        assert_eq!(speckles + DEFECT_PIXELS, glyph_pixels);

        let prepared = prepare_ocr_image(
            rgb_data.clone(),
            CONTRAST_FIXTURE_WIDTH,
            CONTRAST_FIXTURE_HEIGHT,
            None,
            None,
            false,
            Some(300.0),
            false,
        );
        assert!(
            !prepared.apply_pix_preprocessing,
            "a small connected defect does not make scattered noise text-like"
        );
        assert_eq!(prepared.data, rgb_data);
    }

    #[test]
    fn should_reject_malformed_or_overflowing_rgb_dimensions() {
        assert!(!should_apply_default_preprocessing(&[u8::MAX; 3], 2, 2));
        assert!(!should_apply_default_preprocessing(&[], u32::MAX, u32::MAX));
    }

    #[test]
    fn should_select_default_preprocessing_for_black_text_on_white_page() {
        const WIDTH: u32 = 80;
        const HEIGHT: u32 = 20;
        const FOREGROUND_WIDTH: u32 = 4;
        let mut rgb_data = Vec::with_capacity(WIDTH as usize * HEIGHT as usize * RGB_CHANNEL_COUNT);
        for _y in 0..HEIGHT {
            for x in 0..WIDTH {
                let pixel = if x < FOREGROUND_WIDTH {
                    [0; RGB_CHANNEL_COUNT]
                } else {
                    [u8::MAX; RGB_CHANNEL_COUNT]
                };
                rgb_data.extend_from_slice(&pixel);
            }
        }

        assert!(should_apply_default_preprocessing(&rgb_data, WIDTH, HEIGHT));
    }

    #[test]
    fn test_prepare_ocr_image_without_config_preprocesses_clean_white_rgb() {
        const WIDTH: u32 = 4;
        const HEIGHT: u32 = 4;
        let rgb_data = vec![u8::MAX; WIDTH as usize * HEIGHT as usize * RGB_CHANNEL_COUNT];

        let prepared = prepare_ocr_image(rgb_data, WIDTH, HEIGHT, None, None, false, None, false);

        assert!(prepared.apply_pix_preprocessing);
        let preprocessing = prepared
            .preprocessing
            .expect("implicit preprocessing must retain its effective configuration");
        assert!(preprocessing.deskew);
        assert_eq!(preprocessing.binarization_method, "otsu");
    }

    #[test]
    fn test_shadowed_rgb_skips_default_preprocessing() {
        const SAMPLE_PIXEL_COUNT: usize = 16;
        const SHADOWED_CHANNEL_VALUE: u8 = 128;
        let rgb_data = vec![SHADOWED_CHANNEL_VALUE; SAMPLE_PIXEL_COUNT * RGB_CHANNEL_COUNT];

        assert!(!should_apply_default_preprocessing(
            &rgb_data,
            SAMPLE_PIXEL_COUNT as u32,
            1
        ));
    }

    #[test]
    fn test_prepare_ocr_image_with_config_keeps_preprocessing_enabled() {
        let rgb_data = vec![255; 12];
        let preprocessing = crate::types::ImagePreprocessingConfig {
            target_dpi: 72,
            ..Default::default()
        };

        let prepared = prepare_ocr_image(rgb_data, 2, 2, Some(&preprocessing), None, false, None, false);

        assert!(prepared.apply_pix_preprocessing);
        assert_eq!(prepared.preprocessing.unwrap().target_dpi, 72);
    }

    /// #209: when a caller supplies `ImageExtractionConfig`, its `max_image_dimension`
    /// and `auto_adjust_dpi` must actually reach the DPI-normalization step instead of
    /// being silently replaced by `ImageDpiConfig::default()`.
    #[test]
    fn test_prepare_preprocessed_ocr_image_honours_image_extraction_config_limits() {
        const SOURCE_DIMENSION: u32 = 4;
        let rgb_data = vec![255; SOURCE_DIMENSION as usize * SOURCE_DIMENSION as usize * RGB_CHANNEL_COUNT];
        let preprocessing = crate::types::ImagePreprocessingConfig {
            target_dpi: 300,
            ..Default::default()
        };
        let images_config = crate::core::config::ImageExtractionConfig {
            max_image_dimension: 2,
            auto_adjust_dpi: false,
            ..Default::default()
        };

        let prepared = prepare_preprocessed_ocr_image(
            rgb_data,
            SOURCE_DIMENSION,
            SOURCE_DIMENSION,
            &preprocessing,
            Some(&images_config),
            false,
            None,
        );

        assert_eq!(prepared.width, 2, "max_image_dimension=2 must clamp the resized width");
        assert_eq!(
            prepared.height, 2,
            "max_image_dimension=2 must clamp the resized height"
        );
        let metadata = prepared
            .image_preprocessing
            .expect("successful normalization must retain its metadata");
        assert_eq!(metadata.original_dimensions.width, SOURCE_DIMENSION as usize);
        assert_eq!(metadata.original_dimensions.height, SOURCE_DIMENSION as usize);
        assert_eq!(
            metadata.new_dimensions.as_ref().map(|dimensions| dimensions.width),
            Some(2)
        );
        assert_eq!(
            metadata.new_dimensions.as_ref().map(|dimensions| dimensions.height),
            Some(2)
        );
        assert!(metadata.dimension_clamped);
    }

    /// Pixel width of a US Letter page (612pt wide) rendered at 150 DPI -- an arbitrary
    /// non-72, non-target render resolution exercising `known_source_dpi`, not tied to
    /// whatever DPI the PDF OCR route actually renders at (`effective_pdf_render_dpi`, #1577).
    const LETTER_AT_150_DPI_WIDTH_PX: u32 = 1275;
    /// Pixel height of the same page (792pt tall) at 150 DPI.
    const LETTER_AT_150_DPI_HEIGHT_PX: u32 = 1650;

    /// A clean white Letter-sized raster, the shape the PDF OCR route actually hands over: it
    /// passes `should_apply_default_preprocessing`, so both the explicit and the implicit
    /// preprocessing branch reach DPI normalization.
    fn letter_page_raster_at_150_dpi() -> Vec<u8> {
        vec![u8::MAX; LETTER_AT_150_DPI_WIDTH_PX as usize * LETTER_AT_150_DPI_HEIGHT_PX as usize * RGB_CHANNEL_COUNT]
    }

    /// The defect: a page rendered at 150 DPI was normalized as if it were 72 DPI, so the scale
    /// factor became `target_dpi / 72` instead of `target_dpi / 150`.
    ///
    /// With the real 150 handed in, `calculate_smart_dpi` sees a 8.5x11in page (612x792pt),
    /// finds 300 DPI fits inside `max_image_dimension` (11in * 300 = 3300px <= 4096), and
    /// returns the full 300. The resize is then a clean 2.0x to exactly 2550x3300 with no
    /// dimension clamp, and Tesseract is told 300 — which is what the raster now is.
    ///
    /// Fails against unfixed code: `prepare_ocr_image` has no `known_source_dpi` parameter at
    /// all there, so this does not compile. Restoring the old six-argument call (dropping the
    /// `Some(150.0)`) makes it compile and then fail on the first assertion, because the 72
    /// assumption yields `final_dpi = 179` and a 3165x4096 clamped raster:
    /// `assertion \`left == right\` failed: left: 4096, right: 3300`.
    #[test]
    fn should_honour_known_source_dpi_instead_of_assuming_72() {
        const KNOWN_RENDER_DPI: f64 = 150.0;
        const EXPECTED_WIDTH_PX: u32 = 2550;
        const EXPECTED_HEIGHT_PX: u32 = 3300;
        const EXPECTED_SOURCE_DPI: i32 = 300;

        let prepared = prepare_ocr_image(
            letter_page_raster_at_150_dpi(),
            LETTER_AT_150_DPI_WIDTH_PX,
            LETTER_AT_150_DPI_HEIGHT_PX,
            Some(&crate::types::ImagePreprocessingConfig::default()),
            None,
            false,
            Some(KNOWN_RENDER_DPI),
            false,
        );

        assert_eq!(
            prepared.height, EXPECTED_HEIGHT_PX,
            "a 150 DPI Letter page scaled to the 300 DPI target is 3300px tall, not the 4096px \
             the max_image_dimension clamp produces when the source is mistaken for 72 DPI"
        );
        assert_eq!(
            prepared.width, EXPECTED_WIDTH_PX,
            "the matching width for a clean 2.0x resize"
        );
        assert_eq!(
            prepared.source_dpi, EXPECTED_SOURCE_DPI,
            "Tesseract must be told the raster's real resolution, not the 179 the 72 assumption \
             derives"
        );
    }

    /// The contrast leg, pinning the arithmetic the defect produced so a regression is visible
    /// rather than merely different: with no known source DPI the 72 assumption still applies,
    /// the smart-DPI step derives 179 from an apparent 17.7x22.9in page, and the resize runs
    /// into the 4096px `max_image_dimension` clamp.
    ///
    /// Fails against unfixed code only by not compiling (the seventh argument does not exist);
    /// its values are identical either way, which is the point — this leg proves the fix changed
    /// nothing for callers that do not supply a DPI.
    #[test]
    fn should_keep_assuming_72_dpi_when_source_dpi_is_unknown() {
        const CLAMPED_DIMENSION_PX: u32 = 4096;
        const DERIVED_SOURCE_DPI: i32 = 179;

        let prepared = prepare_ocr_image(
            letter_page_raster_at_150_dpi(),
            LETTER_AT_150_DPI_WIDTH_PX,
            LETTER_AT_150_DPI_HEIGHT_PX,
            Some(&crate::types::ImagePreprocessingConfig::default()),
            None,
            false,
            None,
            false,
        );

        assert_eq!(
            prepared.height, CLAMPED_DIMENSION_PX,
            "without a known source DPI the 72 assumption drives the resize into the clamp"
        );
        assert_eq!(
            prepared.source_dpi, DERIVED_SOURCE_DPI,
            "and reports the DPI that assumption implies"
        );
    }

    /// A raw image the caller knows nothing about still gets the 72 fallback on the
    /// pass-through branch, so standalone image OCR is untouched by the new parameter.
    ///
    /// Fails against unfixed code by not compiling; behaviourally it is the pre-existing
    /// contract, asserted here so the new parameter cannot quietly displace it.
    #[test]
    fn should_default_to_72_dpi_for_unpreprocessed_raw_image_without_known_dpi() {
        let rgb_data = vec![0, 1, 2, 3, 4, 5];

        let prepared = prepare_ocr_image(rgb_data.clone(), 2, 1, None, None, false, None, false);

        assert_eq!(prepared.data, rgb_data, "the raster must pass through untouched");
        assert_eq!(prepared.source_dpi, RAW_IMAGE_SOURCE_DPI);
    }

    /// The same pass-through branch reports a known resolution verbatim rather than the 72
    /// fallback: the bytes are unchanged, so their resolution is whatever the caller measured.
    ///
    /// Fails against unfixed code: there is no way to express this at all — `prepare_ocr_image`
    /// hardcodes `RAW_IMAGE_SOURCE_DPI` on this branch, so the reported DPI is 72 and the
    /// assertion reads `assertion \`left == right\` failed: left: 72, right: 150`.
    #[test]
    fn should_report_known_source_dpi_on_the_unpreprocessed_branch() {
        const KNOWN_RENDER_DPI: f64 = 150.0;
        let rgb_data = vec![0, 1, 2, 3, 4, 5];

        let prepared = prepare_ocr_image(rgb_data, 2, 1, None, None, false, Some(KNOWN_RENDER_DPI), false);

        assert_eq!(prepared.source_dpi, 150);
        assert!(!prepared.apply_pix_preprocessing);
    }

    // GH#1630: standalone image OCR discarded PNG `pHYs` density and always assumed 72 DPI,
    // resampling a genuine 300 DPI scan even when the requested `target_dpi` was already 300.
    mod png_source_dpi {
        use super::*;

        /// Standard PNG CRC-32 (polynomial 0xEDB88320), needed to splice a well-formed `pHYs`
        /// chunk into a real encoded PNG.
        fn png_crc32(data: &[u8]) -> u32 {
            let mut crc: u32 = 0xFFFF_FFFF;
            for &byte in data {
                crc ^= u32::from(byte);
                for _ in 0..8 {
                    let mask = (crc & 1).wrapping_neg();
                    crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
                }
            }
            !crc
        }

        fn png_chunk(chunk_type: &[u8; 4], data: &[u8]) -> Vec<u8> {
            let mut body = Vec::with_capacity(4 + data.len());
            body.extend_from_slice(chunk_type);
            body.extend_from_slice(data);
            let mut out = Vec::with_capacity(8 + body.len());
            out.extend_from_slice(&(u32::try_from(data.len()).unwrap()).to_be_bytes());
            out.extend_from_slice(&body);
            out.extend_from_slice(&png_crc32(&body).to_be_bytes());
            out
        }

        fn encode_png(width: u32, height: u32) -> Vec<u8> {
            let img = image::RgbImage::from_pixel(width, height, image::Rgb([255, 255, 255]));
            let mut png = Vec::new();
            {
                use image::ImageEncoder;
                image::codecs::png::PngEncoder::new(&mut png)
                    .write_image(img.as_raw(), width, height, image::ExtendedColorType::Rgb8)
                    .expect("encode PNG");
            }
            png
        }

        /// Splice a `pHYs` chunk expressing `ppu` pixels-per-metre (both axes, unit = meter)
        /// into a real encoded PNG, immediately after IHDR (signature 8 bytes + IHDR chunk 25
        /// bytes) and always before IDAT.
        fn png_with_phys_density(width: u32, height: u32, ppu: u32) -> Vec<u8> {
            const SIGNATURE_AND_IHDR_LEN: usize = 8 + 25;
            let mut phys_data = Vec::with_capacity(9);
            phys_data.extend_from_slice(&ppu.to_be_bytes());
            phys_data.extend_from_slice(&ppu.to_be_bytes());
            phys_data.push(1); // unit = meter
            let phys_chunk = png_chunk(b"pHYs", &phys_data);

            let png = encode_png(width, height);
            let mut out = Vec::with_capacity(png.len() + phys_chunk.len());
            out.extend_from_slice(&png[..SIGNATURE_AND_IHDR_LEN]);
            out.extend_from_slice(&phys_chunk);
            out.extend_from_slice(&png[SIGNATURE_AND_IHDR_LEN..]);
            out
        }

        /// 11811 px/m is the reporter's fixture value: 11811 * 0.0254 = 299.9994 DPI, not an
        /// exact 300 — every assertion below tolerates that rather than requiring an exact
        /// integer match.
        const REPORTER_FIXTURE_PPU: u32 = 11_811;
        const EXPECTED_DPI_FROM_REPORTER_FIXTURE: f64 = 299.999_4;
        const DPI_TOLERANCE: f64 = 0.001;
        const TEST_IMAGE_SIDE: u32 = 4;

        fn rgb_fixture() -> Vec<u8> {
            vec![255u8; TEST_IMAGE_SIDE as usize * TEST_IMAGE_SIDE as usize * RGB_CHANNEL_COUNT]
        }

        fn preprocessing_targeting(target_dpi: i32) -> crate::types::ImagePreprocessingConfig {
            crate::types::ImagePreprocessingConfig {
                target_dpi,
                ..Default::default()
            }
        }

        fn images_config_without_auto_adjust() -> crate::core::config::ImageExtractionConfig {
            crate::core::config::ImageExtractionConfig {
                auto_adjust_dpi: false,
                ..Default::default()
            }
        }

        #[test]
        fn should_prefer_explicit_source_dpi_over_embedded_png_density() {
            let png = png_with_phys_density(TEST_IMAGE_SIDE, TEST_IMAGE_SIDE, REPORTER_FIXTURE_PPU);

            let resolved = resolve_known_source_dpi(Some(150.0), &png, TEST_IMAGE_SIDE, TEST_IMAGE_SIDE);

            assert_eq!(
                resolved,
                Some(150.0),
                "a caller-supplied source_dpi hint must win over embedded PNG metadata"
            );
        }

        #[test]
        fn should_decode_source_density_from_embedded_png_metadata() {
            let png = png_with_phys_density(TEST_IMAGE_SIDE, TEST_IMAGE_SIDE, REPORTER_FIXTURE_PPU);

            let resolved = resolve_known_source_dpi(None, &png, TEST_IMAGE_SIDE, TEST_IMAGE_SIDE)
                .expect("pHYs density must be detected");

            assert!(
                (resolved - EXPECTED_DPI_FROM_REPORTER_FIXTURE).abs() < DPI_TOLERANCE,
                "expected ~{EXPECTED_DPI_FROM_REPORTER_FIXTURE} DPI, got {resolved}"
            );
        }

        #[test]
        fn should_resolve_none_when_neither_explicit_hint_nor_embedded_density_exist() {
            let png = encode_png(TEST_IMAGE_SIDE, TEST_IMAGE_SIDE);

            assert_eq!(
                resolve_known_source_dpi(None, &png, TEST_IMAGE_SIDE, TEST_IMAGE_SIDE),
                None,
                "a PNG without density metadata must not fabricate a source DPI"
            );
        }

        /// No resize when the requested target already matches the embedded source density: the
        /// GH#1630 reporter's own repro (`target_dpi=300` against a genuine 300 DPI scan).
        #[test]
        fn should_skip_resize_when_target_dpi_matches_known_png_density() {
            let png = png_with_phys_density(TEST_IMAGE_SIDE, TEST_IMAGE_SIDE, REPORTER_FIXTURE_PPU);
            let known_source_dpi = resolve_known_source_dpi(None, &png, TEST_IMAGE_SIDE, TEST_IMAGE_SIDE);
            let images_config = images_config_without_auto_adjust();
            let preprocessing = preprocessing_targeting(300);

            let prepared = prepare_ocr_image(
                rgb_fixture(),
                TEST_IMAGE_SIDE,
                TEST_IMAGE_SIDE,
                Some(&preprocessing),
                Some(&images_config),
                false,
                known_source_dpi,
                false,
            );

            assert_eq!(prepared.width, TEST_IMAGE_SIDE);
            assert_eq!(prepared.height, TEST_IMAGE_SIDE);
            let metadata = prepared
                .image_preprocessing
                .expect("preprocessing metadata is recorded");
            assert!(
                metadata.skipped_resize,
                "a 300 target against a ~300 detected source must not resize"
            );
        }

        /// A different target DPI still resizes correctly once the true source density is known,
        /// rather than scaling from the wrong 72 assumption.
        #[test]
        fn should_resize_correctly_for_a_different_target_dpi() {
            let png = png_with_phys_density(TEST_IMAGE_SIDE, TEST_IMAGE_SIDE, REPORTER_FIXTURE_PPU);
            let known_source_dpi = resolve_known_source_dpi(None, &png, TEST_IMAGE_SIDE, TEST_IMAGE_SIDE);
            let images_config = images_config_without_auto_adjust();
            let preprocessing = preprocessing_targeting(150);

            let prepared = prepare_ocr_image(
                rgb_fixture(),
                TEST_IMAGE_SIDE,
                TEST_IMAGE_SIDE,
                Some(&preprocessing),
                Some(&images_config),
                false,
                known_source_dpi,
                false,
            );

            assert_eq!(
                prepared.width, 2,
                "halving from a ~300 DPI source to a 150 DPI target halves the raster"
            );
            assert_eq!(prepared.height, 2);
        }

        /// Control: a PNG carrying no density metadata must keep defaulting to 72 DPI, exactly
        /// as before this fix.
        #[test]
        fn should_default_to_72_dpi_when_png_has_no_density_metadata() {
            let png = encode_png(TEST_IMAGE_SIDE, TEST_IMAGE_SIDE);
            let known_source_dpi = resolve_known_source_dpi(None, &png, TEST_IMAGE_SIDE, TEST_IMAGE_SIDE);
            assert_eq!(known_source_dpi, None);

            let images_config = images_config_without_auto_adjust();
            let preprocessing = preprocessing_targeting(300);
            let prepared = prepare_ocr_image(
                rgb_fixture(),
                TEST_IMAGE_SIDE,
                TEST_IMAGE_SIDE,
                Some(&preprocessing),
                Some(&images_config),
                false,
                known_source_dpi,
                false,
            );

            let metadata = prepared
                .image_preprocessing
                .expect("preprocessing metadata is recorded");
            assert_eq!(
                metadata.original_dpi,
                crate::types::ImageDpi::from((f64::from(RAW_IMAGE_SOURCE_DPI), f64::from(RAW_IMAGE_SOURCE_DPI))),
                "no embedded metadata must leave the historical 72 assumption unchanged"
            );
        }
    }

    #[test]
    fn test_should_invert_for_polarity_dark_background_with_text() {
        // Dark background (mean well below threshold) with a clear light-text
        // fraction should trigger auto-detected inversion.
        assert!(should_invert_for_polarity(40.0, 0.05, false));
    }

    #[test]
    fn test_should_invert_for_polarity_light_background_not_inverted() {
        // Typical dark-text-on-light-paper scan: high mean, no auto-invert.
        assert!(!should_invert_for_polarity(220.0, 0.9, false));
    }

    #[test]
    fn test_should_invert_for_polarity_uniformly_dark_image_not_inverted() {
        // Dark mean but essentially no light pixels: a uniformly dark, non-text
        // image (e.g. a dark photo) should NOT be auto-inverted.
        assert!(!should_invert_for_polarity(20.0, 0.0, false));
    }

    #[test]
    fn test_should_invert_for_polarity_force_true_overrides_light_background() {
        // Explicit invert_colors=true forces inversion even on a light-mean image.
        assert!(should_invert_for_polarity(220.0, 0.9, true));
    }

    #[test]
    fn test_should_invert_for_polarity_force_false_still_auto_detects() {
        assert!(should_invert_for_polarity(10.0, 0.5, false));
    }

    #[test]
    fn test_preprocess_pix_inverts_light_on_dark_image() {
        let width = 40u32;
        let height = 40u32;
        let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
        for y in 0..height {
            for x in 0..width {
                let is_text = (10..13).contains(&y) && (5..25).contains(&x);
                let value = if is_text { 230u8 } else { 20u8 };
                rgb_data.extend_from_slice(&[value, value, value]);
            }
        }

        let pix = xberg_tesseract::Pix::from_raw_rgb(&rgb_data, width, height).unwrap();
        let processed = preprocess_pix(pix, &preprocessing_config()).unwrap();

        assert_eq!(processed.depth(), 1);
        assert_eq!(binary_pixel_value(&processed, 0, 0), 0, "background must be white");
        assert_eq!(binary_pixel_value(&processed, 10, 11), 1, "text must be black");
    }

    #[test]
    fn test_preprocess_pix_does_not_invert_dark_on_light_image() {
        let width = 40u32;
        let height = 40u32;
        let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
        for y in 0..height {
            for x in 0..width {
                let is_text = (10..13).contains(&y) && (5..25).contains(&x);
                let value = if is_text { 20u8 } else { 230u8 };
                rgb_data.extend_from_slice(&[value, value, value]);
            }
        }

        let pix = xberg_tesseract::Pix::from_raw_rgb(&rgb_data, width, height).unwrap();
        let processed = preprocess_pix(pix, &preprocessing_config()).unwrap();

        assert_eq!(processed.depth(), 1);
        assert_eq!(binary_pixel_value(&processed, 0, 0), 0, "background must be white");
        assert_eq!(binary_pixel_value(&processed, 10, 11), 1, "text must be black");
    }

    #[cfg(auto_rotate)]
    #[test]
    #[cfg(auto_rotate)]
    fn test_rotate_rgb_image_data_identity() {
        let data: Vec<u8> = (0..18).collect();
        let (out, w, h) = rotate_rgb_image_data(&data, 2, 3, 0);
        assert_eq!(out, data);
        assert_eq!(w, 2);
        assert_eq!(h, 3);
    }

    #[cfg(auto_rotate)]
    #[test]
    #[cfg(auto_rotate)]
    fn test_rotate_rgb_image_data_180() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let (out, w, h) = rotate_rgb_image_data(&data, 2, 2, 180);
        assert_eq!(w, 2);
        assert_eq!(h, 2);
        assert_eq!(out, vec![10, 11, 12, 7, 8, 9, 4, 5, 6, 1, 2, 3]);
    }

    #[cfg(auto_rotate)]
    #[test]
    #[cfg(auto_rotate)]
    fn test_rotate_rgb_image_data_90_swaps_dimensions() {
        let data: Vec<u8> = (0..18).collect();
        let (_, w, h) = rotate_rgb_image_data(&data, 2, 3, 90);
        assert_eq!(w, 3);
        assert_eq!(h, 2);
    }

    #[cfg(auto_rotate)]
    #[test]
    #[cfg(auto_rotate)]
    fn test_rotate_rgb_image_data_270_swaps_dimensions() {
        let data: Vec<u8> = (0..18).collect();
        let (_, w, h) = rotate_rgb_image_data(&data, 2, 3, 270);
        assert_eq!(w, 3);
        assert_eq!(h, 2);
    }

    #[cfg(auto_rotate)]
    #[test]
    #[cfg(auto_rotate)]
    fn test_rotate_rgb_image_data_90_then_270_is_identity() {
        let data: Vec<u8> = (0..18).collect();
        let (rotated_90, w1, h1) = rotate_rgb_image_data(&data, 2, 3, 90);
        let (back, w2, h2) = rotate_rgb_image_data(&rotated_90, w1, h1, 270);
        assert_eq!(w2, 2);
        assert_eq!(h2, 3);
        assert_eq!(back, data);
    }

    #[cfg(auto_rotate)]
    #[test]
    #[cfg(auto_rotate)]
    fn test_rotate_rgb_image_data_unsupported_angle() {
        let data: Vec<u8> = (0..12).collect();
        let (out, w, h) = rotate_rgb_image_data(&data, 2, 2, 45);
        assert_eq!(out, data);
        assert_eq!(w, 2);
        assert_eq!(h, 2);
    }
}
