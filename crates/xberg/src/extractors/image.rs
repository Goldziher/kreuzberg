//! Image extractors for various image formats.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::image::extract_image_metadata_with_security_limits;
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::metadata::Metadata;
use async_trait::async_trait;

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
const WHOLE_IMAGE_TESSERACT_PSM: i32 = 11;

// Tesseract's automatic layout modes can hang under the single-threaded WASM
// runtime, so retain the existing single-block default there. ~keep
#[cfg(all(
    target_arch = "wasm32",
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
const WHOLE_IMAGE_TESSERACT_PSM: i32 = 6;

#[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
const VERTICAL_BLOCK_TESSERACT_PSM: i32 = 5;

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
const SPARSE_IMAGE_OCR_WORD_LIMIT: usize = 20;

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
const SPARSE_IMAGE_OCR_FALLBACK_PSM: i32 = 3;

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
const RTL_SPARSE_IMAGE_OCR_UNIQUE_TOKEN_LIMIT: usize = 32;

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
const RTL_SPARSE_IMAGE_OCR_TOTAL_TOKEN_LIMIT: usize = 48;

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
const RTL_SPARSE_IMAGE_OCR_FALLBACK_PSM: i32 = 6;

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
const RTL_IDENTIFIER_RECOVERY_MAX_CROPS: usize = 6;
#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
const RTL_IDENTIFIER_RECOVERY_MIN_CONFIDENCE: f64 = 0.25;
#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
const RTL_IDENTIFIER_RECOVERY_MAX_CROP_PIXELS: u64 = 500_000;

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
const RTL_SPARSE_IMAGE_OCR_MAX_CONFIDENCE_DROP: f64 = 0.10;

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
const RTL_SPARSE_IMAGE_OCR_MIN_PRIMARY_TOKEN_RETENTION: f64 = 0.80;

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
const SPARSE_IMAGE_OCR_MIN_WORD_CONFIDENCE: f64 = 0.30;

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
const SPARSE_IMAGE_OCR_MAX_LOW_CONFIDENCE_RATIO: f64 = 0.30;

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
const SPARSE_IMAGE_OCR_CONFIDENCE_PERCENTILE: f64 = 0.10;

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
const LAYOUT_REGION_TESSERACT_PSM: i32 = 6;

#[cfg(any(test, all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm"))))]
const MIN_LAYOUT_OCR_ALPHANUMERIC_TOKEN_RETENTION: f64 = 0.80;

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
const REQUIRED_CACHED_LAYOUT_TOKEN_RETENTION: f64 = 1.0;

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
const LAYOUT_READING_ORDER_ROW_HEIGHT_RATIO: f32 = 0.05;

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
const MIN_LAYOUT_CROP_DIMENSION: u32 = 4;

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
const MIN_LAYOUT_OCR_ELEMENT_INTERSECTION_OVER_WORD_AREA: f32 = 0.2;

#[cfg(feature = "ocr-pipeline")]
const NORMALIZED_PNG_ENCODE_BYTES_PER_PIXEL: u64 = 4;
#[cfg(feature = "ocr-pipeline")]
const NORMALIZED_PNG_ENCODE_FIXED_BYTES: u64 = 256 * 1024;

// ~keep The cfg is the union of both call sites': `whole_image_ocr_coordinate_transform` here
// (`layout-detection` + `ocr`) and `pdf::ocr::document::ocr_render_scale_is_trustworthy`
// (`layout-detection` + `ocr`/`ocr-wasm`). One constant, because the two routes must apply the
// same bar to the same question.
#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
pub(crate) const MAX_OCR_COORDINATE_SCALE_RELATIVE_DIFFERENCE: f64 = 0.01;

#[cfg(any(test, all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm"))))]
fn internal_document_text(doc: &InternalDocument) -> String {
    doc.elements
        .iter()
        .filter_map(|element| {
            let text = element.text.trim();
            (!text.is_empty()).then_some(text)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(any(test, all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm"))))]
fn alphanumeric_tokens(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_lowercase())
        .collect()
}

#[cfg(any(test, all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm"))))]
fn alphanumeric_token_retention(layout_text: &str, whole_image_text: &str) -> f64 {
    let whole_image_tokens = alphanumeric_tokens(whole_image_text);
    if whole_image_tokens.is_empty() {
        return 1.0;
    }

    let mut layout_token_counts = std::collections::HashMap::<String, usize>::new();
    for token in alphanumeric_tokens(layout_text) {
        *layout_token_counts.entry(token).or_default() += 1;
    }

    let retained = whole_image_tokens
        .iter()
        .filter(|token| {
            let Some(count) = layout_token_counts.get_mut(*token) else {
                return false;
            };
            if *count == 0 {
                return false;
            }
            *count -= 1;
            true
        })
        .count();

    retained as f64 / whole_image_tokens.len() as f64
}

#[cfg(any(test, all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm"))))]
fn image_ocr_quality_score(text: &str) -> f64 {
    #[cfg(feature = "quality")]
    {
        crate::text::quality::calculate_quality_score(text, None)
    }

    #[cfg(not(feature = "quality"))]
    {
        let _ = text;
        0.0
    }
}

#[cfg(any(test, all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm"))))]
fn select_image_ocr_result(
    layout_doc: InternalDocument,
    whole_image_result: Result<InternalDocument>,
) -> InternalDocument {
    let whole_image_doc = match whole_image_result {
        Ok(doc) => doc,
        Err(error) => {
            tracing::warn!(%error, "Whole-image OCR quality comparison failed; retaining layout-region OCR");
            return layout_doc;
        }
    };
    let layout_text = internal_document_text(&layout_doc);
    let whole_image_text = internal_document_text(&whole_image_doc);
    let layout_score = image_ocr_quality_score(&layout_text);
    let whole_image_score = image_ocr_quality_score(&whole_image_text);
    let token_retention = alphanumeric_token_retention(&layout_text, &whole_image_text);

    if layout_score < whole_image_score || token_retention < MIN_LAYOUT_OCR_ALPHANUMERIC_TOKEN_RETENTION {
        tracing::debug!(
            layout_score,
            whole_image_score,
            token_retention,
            "Whole-image OCR retained because layout-region OCR reduced text quality"
        );
        whole_image_doc
    } else {
        layout_doc
    }
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn cached_whole_image_after_layout_error(
    whole_image_result: &Result<InternalDocument>,
    error: crate::XbergError,
) -> Result<InternalDocument> {
    let whole_image_doc = match whole_image_result {
        Ok(doc) => doc,
        Err(whole_image_error) => {
            return Err(crate::XbergError::Other(format!(
                "Image OCR failed in both paths; whole-image OCR: {whole_image_error}; layout-region OCR: {error}"
            )));
        }
    };
    tracing::warn!(
        %error,
        "Layout-region OCR failed after whole-image OCR succeeded; retaining whole-image output"
    );
    let mut retained = whole_image_doc.clone();
    retained.processing_warnings.push(crate::types::ProcessingWarning {
        source: std::borrow::Cow::Borrowed("layout-ocr"),
        message: std::borrow::Cow::Borrowed(
            "Layout-region OCR failed after whole-image OCR succeeded; retained whole-image output",
        ),
    });
    Ok(retained)
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn ocr_geometry_bounds(geometry: &crate::types::OcrBoundingGeometry) -> (u32, u32, u32, u32) {
    match geometry {
        crate::types::OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height,
        } => (*left, *top, *width, *height),
        crate::types::OcrBoundingGeometry::Quadrilateral { points } => {
            let min_x = points.iter().map(|point| point.x).min().unwrap_or(0);
            let max_x = points.iter().map(|point| point.x).max().unwrap_or(0);
            let min_y = points.iter().map(|point| point.y).min().unwrap_or(0);
            let max_y = points.iter().map(|point| point.y).max().unwrap_or(0);
            (min_x, min_y, max_x.saturating_sub(min_x), max_y.saturating_sub(min_y))
        }
    }
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
#[derive(Clone, Copy)]
struct OcrCoordinateTransform {
    processed_width: u64,
    processed_height: u64,
    scale_x: f64,
    scale_y: f64,
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn whole_image_ocr_coordinate_transform(
    doc: &InternalDocument,
    image_width: u32,
    image_height: u32,
) -> Option<OcrCoordinateTransform> {
    #[cfg(feature = "ocr")]
    {
        let additional = &doc.metadata.additional;
        let processed_width = additional
            .get(crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY)
            .and_then(serde_json::Value::as_u64);
        let processed_height = additional
            .get(crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY)
            .and_then(serde_json::Value::as_u64);
        let auto_rotated = additional
            .get(crate::ocr_metadata_keys::OCR_AUTO_ROTATED_METADATA_KEY)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let (processed_width, processed_height) = (processed_width?, processed_height?);
        if auto_rotated || processed_width == 0 || processed_height == 0 || image_width == 0 || image_height == 0 {
            return None;
        }
        let scale_x = f64::from(image_width) / processed_width as f64;
        let scale_y = f64::from(image_height) / processed_height as f64;
        let relative_difference = (scale_x - scale_y).abs() / scale_x.max(scale_y);
        (relative_difference <= MAX_OCR_COORDINATE_SCALE_RELATIVE_DIFFERENCE).then_some(OcrCoordinateTransform {
            processed_width,
            processed_height,
            scale_x,
            scale_y,
        })
    }

    #[cfg(not(feature = "ocr"))]
    {
        let _ = (doc, image_width, image_height);
        None
    }
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn transformed_ocr_bounds(
    geometry: &crate::types::OcrBoundingGeometry,
    transform: OcrCoordinateTransform,
) -> Option<(f32, f32, f32, f32)> {
    let (left, top, width, height) = ocr_geometry_bounds(geometry);
    let right = u64::from(left) + u64::from(width);
    let bottom = u64::from(top) + u64::from(height);
    if width == 0 || height == 0 || right > transform.processed_width || bottom > transform.processed_height {
        return None;
    }
    Some((
        (f64::from(left) * transform.scale_x) as f32,
        (f64::from(top) * transform.scale_y) as f32,
        (right as f64 * transform.scale_x) as f32,
        (bottom as f64 * transform.scale_y) as f32,
    ))
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn preferred_ocr_elements(
    elements: &[crate::types::OcrElement],
    preferred_level: crate::types::OcrElementLevel,
) -> Vec<&crate::types::OcrElement> {
    let fallback_level = match preferred_level {
        crate::types::OcrElementLevel::Line => crate::types::OcrElementLevel::Word,
        crate::types::OcrElementLevel::Word => crate::types::OcrElementLevel::Line,
        _ => preferred_level,
    };
    let meaningful = elements
        .iter()
        .filter(|element| !element.text.trim().is_empty())
        .collect::<Vec<_>>();
    let (line_count, vertical_line_count) = meaningful
        .iter()
        .filter(|element| element.level == crate::types::OcrElementLevel::Line)
        .fold((0usize, 0usize), |(line_count, vertical_count), element| {
            let (_, _, width, height) = ocr_geometry_bounds(&element.geometry);
            (
                line_count + 1,
                vertical_count + usize::from(u64::from(height) > u64::from(width) * 2),
            )
        });
    let preferred_lines_are_vertical = preferred_level == crate::types::OcrElementLevel::Line
        && line_count > 0
        && vertical_line_count * 2 >= line_count;
    let selected_level =
        if preferred_lines_are_vertical && meaningful.iter().any(|element| element.level == fallback_level) {
            Some(fallback_level)
        } else if meaningful.iter().any(|element| element.level == preferred_level) {
            Some(preferred_level)
        } else if meaningful.iter().any(|element| element.level == fallback_level) {
            Some(fallback_level)
        } else {
            None
        };

    meaningful
        .into_iter()
        .filter(|element| selected_level.is_none_or(|level| element.level == level))
        .collect()
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn transformed_ocr_elements(
    elements: &[crate::types::OcrElement],
    transform: OcrCoordinateTransform,
    preferred_level: crate::types::OcrElementLevel,
) -> Option<Vec<crate::types::OcrElement>> {
    preferred_ocr_elements(elements, preferred_level)
        .into_iter()
        .map(|element| {
            let (left, top, right, bottom) = transformed_ocr_bounds(&element.geometry, transform)?;
            let mut transformed = element.clone();
            transformed.geometry = crate::types::OcrBoundingGeometry::Rectangle {
                left: left.round() as u32,
                top: top.round() as u32,
                width: (right - left).round() as u32,
                height: (bottom - top).round() as u32,
            };
            Some(transformed)
        })
        .collect()
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn ocr_element_has_unique_full_containment(
    element: &crate::types::OcrElement,
    detections: &[crate::layout::LayoutDetection],
    transform: OcrCoordinateTransform,
) -> bool {
    let Some((left, top, right, bottom)) = transformed_ocr_bounds(&element.geometry, transform) else {
        return false;
    };
    let mut matches = detections.iter().filter(|detection| {
        left >= detection.bbox.x1
            && right <= detection.bbox.x2
            && top >= detection.bbox.y1
            && bottom <= detection.bbox.y2
    });
    matches.next().is_some() && matches.next().is_none()
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn whole_image_layout_mapping_retention(
    doc: &InternalDocument,
    detections: &[crate::layout::LayoutDetection],
    image_width: u32,
    image_height: u32,
) -> Option<f64> {
    let pages = doc.prebuilt_pages.as_ref()?;
    if pages.len() != 1 || pages[0].page_number != 1 {
        return None;
    }
    let elements = doc.prebuilt_ocr_elements.as_ref()?;
    let meaningful = preferred_ocr_elements(elements, crate::types::OcrElementLevel::Line);
    if meaningful.is_empty() || meaningful.iter().any(|element| element.page_number != 1) {
        return None;
    }
    let transform = whole_image_ocr_coordinate_transform(doc, image_width, image_height)?;
    let mut total_tokens = 0;
    let mut mapped_tokens = 0;
    for element in meaningful {
        let token_count = alphanumeric_tokens(&element.text).len();
        total_tokens += token_count;
        if ocr_element_has_unique_full_containment(element, detections, transform) {
            mapped_tokens += token_count;
        }
    }
    (total_tokens > 0).then_some(mapped_tokens as f64 / total_tokens as f64)
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn source_image_is_proven_single_frame(content: &[u8], mime_type: &str) -> bool {
    match mime_type {
        "image/png" | "image/webp" => {
            crate::extraction::image_decode::standard_image_is_single_frame(content, mime_type)
        }
        "image/jpeg" | "image/jpg" | "image/pjpeg" => !content.windows(4).any(|window| window == b"MPF\0"),
        "image/bmp"
        | "image/x-bmp"
        | "image/x-ms-bmp"
        | "image/x-portable-anymap"
        | "image/x-portable-bitmap"
        | "image/x-portable-graymap"
        | "image/x-portable-pixmap" => true,
        #[cfg(feature = "ocr")]
        "image/tiff" | "image/x-tiff" => {
            crate::extraction::image_decode::standard_image_is_single_frame(content, mime_type)
        }
        _ => false,
    }
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn split_leading_numbered_list_marker(text: &str) -> Option<(&str, &str)> {
    let digits_end = text.find(|character: char| !character.is_ascii_digit())?;
    if digits_end == 0 {
        return None;
    }
    let suffix = &text[digits_end..];
    let delimiter = suffix
        .chars()
        .next()
        .filter(|character| matches!(character, '.' | ')'))?;
    let marker_end = digits_end + delimiter.len_utf8();
    let remainder = &text[marker_end..];
    let content = remainder.trim_start();
    (content.len() < remainder.len() && !content.is_empty()).then_some((&text[..marker_end], content))
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn push_mapped_layout_text(
    builder: &mut InternalDocumentBuilder,
    formulas: &mut Vec<crate::types::Formula>,
    detection: &crate::layout::LayoutDetection,
    text: &str,
) -> bool {
    use crate::layout::LayoutClass;
    use crate::types::internal::{ElementKind, InternalElement};

    match detection.class_name {
        LayoutClass::Title => {
            builder.push_heading(1, text, None, None);
        }
        LayoutClass::SectionHeader => {
            builder.push_heading(2, text, None, None);
        }
        LayoutClass::Code => {
            builder.push_code(text, None, None, None);
        }
        LayoutClass::Formula => {
            formulas.push(crate::types::Formula {
                latex: text.to_string(),
                bbox: Some(crate::types::BoundingBox {
                    x0: detection.bbox.x1 as f64,
                    y0: detection.bbox.y1 as f64,
                    x1: detection.bbox.x2 as f64,
                    y1: detection.bbox.y2 as f64,
                }),
                page: Some(1),
            });
            builder.push_element(InternalElement::text(ElementKind::Formula, text, 0));
        }
        LayoutClass::ListItem => {
            if let Some((source_label, content)) = split_leading_numbered_list_marker(text) {
                let index = builder.push_list_item(content, true, vec![], None, None);
                builder.set_list_item_source_label(index, source_label);
            } else {
                builder.push_list_item(text, false, vec![], None, None);
            }
        }
        LayoutClass::CheckboxSelected | LayoutClass::CheckboxUnselected => {
            builder.push_list_item(text, false, vec![], None, None);
        }
        LayoutClass::PageHeader | LayoutClass::PageFooter | LayoutClass::Picture | LayoutClass::Chart => return false,
        _ => {
            builder.push_paragraph(text, vec![], None, None);
        }
    }
    true
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn bbox_contains_element(bbox: crate::layout::BBox, element: &crate::types::OcrElement) -> bool {
    let (left, top, width, height) = ocr_geometry_bounds(&element.geometry);
    let element_left = left as f32;
    let element_top = top as f32;
    let element_right = element_left + width as f32;
    let element_bottom = element_top + height as f32;
    let element_area = width as f32 * height as f32;
    if element_area <= 0.0 {
        let center_x = element_left + width as f32 / 2.0;
        let center_y = element_top + height as f32 / 2.0;
        return center_x >= bbox.x1 && center_x <= bbox.x2 && center_y >= bbox.y1 && center_y <= bbox.y2;
    }
    let intersection_width = (element_right.min(bbox.x2) - element_left.max(bbox.x1)).max(0.0);
    let intersection_height = (element_bottom.min(bbox.y2) - element_top.max(bbox.y1)).max(0.0);
    intersection_width * intersection_height / element_area >= MIN_LAYOUT_OCR_ELEMENT_INTERSECTION_OVER_WORD_AREA
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn text_from_positioned_elements(elements: &[&crate::types::OcrElement]) -> String {
    let mut positioned = elements
        .iter()
        .map(|element| {
            let (left, top, _, height) = ocr_geometry_bounds(&element.geometry);
            (*element, left, top, height)
        })
        .collect::<Vec<_>>();
    positioned.sort_by(|left, right| left.2.cmp(&right.2).then_with(|| left.1.cmp(&right.1)));

    let mut text = String::new();
    let mut previous_line: Option<(u32, u32)> = None;
    for (element, _, top, height) in positioned {
        if !text.is_empty() {
            let is_new_line = previous_line.is_some_and(|(previous_top, previous_height)| {
                top.abs_diff(previous_top) > previous_height.max(height) / 2
            });
            text.push(if is_new_line { '\n' } else { ' ' });
        }
        text.push_str(element.text.trim());
        previous_line = Some((top, height));
    }
    text
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn push_cached_layout_region(
    builder: &mut InternalDocumentBuilder,
    formulas: &mut Vec<crate::types::Formula>,
    detection: &crate::layout::LayoutDetection,
    recognized_tables: &[crate::RecognizedTable],
    elements: &[crate::types::OcrElement],
    assigned: &mut [bool],
) {
    if let Some(recognized) = recognized_tables
        .iter()
        .find(|table| table.detection_bbox == detection.bbox)
    {
        builder.push_table(
            crate::types::Table {
                cells: recognized.cells.clone(),
                markdown: recognized.markdown.clone(),
                page_number: 1,
                bounding_box: Some(crate::types::BoundingBox {
                    x0: recognized.detection_bbox.x1 as f64,
                    y0: recognized.detection_bbox.y1 as f64,
                    x1: recognized.detection_bbox.x2 as f64,
                    y1: recognized.detection_bbox.y2 as f64,
                }),
                // `table_id`/`columns` are assigned once, in document push order, by
                // `finish_cached_layout_document` after all regions for this image have
                // been pushed — see that function for the deterministic scheme.
                ..Default::default()
            },
            Some(1),
            None,
        );
        return;
    }

    let region_elements = elements
        .iter()
        .enumerate()
        .filter_map(|(index, element)| {
            (!assigned[index] && bbox_contains_element(detection.bbox, element)).then_some((index, element))
        })
        .collect::<Vec<_>>();
    let positioned = region_elements.iter().map(|(_, element)| *element).collect::<Vec<_>>();
    let text = text_from_positioned_elements(&positioned);
    if text.trim().is_empty() {
        return;
    }
    if push_mapped_layout_text(builder, formulas, detection, &text) {
        for (index, _) in region_elements {
            assigned[index] = true;
        }
    }
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn finish_cached_layout_document(
    builder: InternalDocumentBuilder,
    whole_image_doc: &InternalDocument,
    detections: &[crate::layout::LayoutDetection],
    formulas: Vec<crate::types::Formula>,
    image_width: u32,
    image_height: u32,
) -> InternalDocument {
    let mut assembled = builder.build();
    // Deterministic id: `"table-N"` where N is this table's 1-based position among
    // this image's tables, in document (push) order — never randomness/wall-clock.
    // See `crate::types::Table::table_id` for the shared scheme doc.
    for (index, table) in assembled.tables.iter_mut().enumerate() {
        table.table_id = Some(format!("table-{}", index + 1));
        if table.columns.is_none() {
            table.columns = table.cells.first().cloned();
        }
    }
    assembled.metadata = whole_image_doc.metadata.clone();
    assembled.processing_warnings = whole_image_doc.processing_warnings.clone();
    assembled.prebuilt_ocr_elements = whole_image_doc.prebuilt_ocr_elements.clone();
    assembled.formulas = if formulas.is_empty() {
        whole_image_doc.formulas.clone()
    } else {
        formulas
    };
    let page_content = crate::rendering::render_plain(&assembled);
    assembled.prebuilt_pages = Some(vec![crate::types::PageContent {
        page_number: 1,
        content: page_content,
        tables: assembled.tables.iter().cloned().map(std::sync::Arc::new).collect(),
        image_indices: vec![],
        hierarchy: None,
        is_blank: None,
        layout_regions: Some(layout_regions_from_detections(detections, image_width, image_height)),
        speaker_notes: None,
        section_name: None,
        sheet_name: None,
        ocr_confidence: None,
        native_content: None,
        image_preprocessing: whole_image_doc.metadata.image_preprocessing.clone(),
    }]);
    ImageExtractor::mark_ocr_extraction(&mut assembled);
    assembled
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn cached_layout_adds_structure(
    detections: &[crate::layout::LayoutDetection],
    recognized_tables: &[crate::RecognizedTable],
) -> bool {
    use crate::layout::LayoutClass;

    !recognized_tables.is_empty()
        || detections.iter().any(|detection| {
            matches!(
                detection.class_name,
                LayoutClass::Title
                    | LayoutClass::SectionHeader
                    | LayoutClass::Code
                    | LayoutClass::Formula
                    | LayoutClass::ListItem
                    | LayoutClass::CheckboxSelected
                    | LayoutClass::CheckboxUnselected
            )
        })
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn layout_detection_accepts_text(detection: &crate::layout::LayoutDetection) -> bool {
    !matches!(
        detection.class_name,
        crate::layout::LayoutClass::PageHeader
            | crate::layout::LayoutClass::PageFooter
            | crate::layout::LayoutClass::Picture
            | crate::layout::LayoutClass::Chart
    )
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn element_claimed_by_layout(
    element: &crate::types::OcrElement,
    detections: &[crate::layout::LayoutDetection],
    recognized_tables: &[crate::RecognizedTable],
) -> bool {
    recognized_tables
        .iter()
        .any(|table| bbox_contains_element(table.detection_bbox, element))
        || detections
            .iter()
            .any(|detection| layout_detection_accepts_text(detection) && bbox_contains_element(detection.bbox, element))
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn ocr_element_position(element: &crate::types::OcrElement) -> (u32, u32) {
    let (left, top, _, _) = ocr_geometry_bounds(&element.geometry);
    (top, left)
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn push_unmatched_ocr(builder: &mut InternalDocumentBuilder, elements: &[&crate::types::OcrElement]) {
    let text = text_from_positioned_elements(elements);
    if !text.trim().is_empty() {
        builder.push_paragraph(text.trim(), vec![], Some(1), None);
    }
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn cached_layout_elements(
    whole_image_doc: &InternalDocument,
    image_width: u32,
    image_height: u32,
) -> Option<Vec<crate::types::OcrElement>> {
    let pages = whole_image_doc.prebuilt_pages.as_ref()?;
    if pages.len() != 1 || pages[0].page_number != 1 {
        return None;
    }
    let source_elements = whole_image_doc.prebuilt_ocr_elements.as_ref()?;
    let transform = whole_image_ocr_coordinate_transform(whole_image_doc, image_width, image_height)?;
    let elements = transformed_ocr_elements(source_elements, transform, crate::types::OcrElementLevel::Line)?;
    if elements.iter().any(|element| element.page_number != 1) {
        return None;
    }
    Some(elements)
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn ordered_cached_layout_items<'a>(
    detections: &'a [crate::layout::LayoutDetection],
    elements: &'a [crate::types::OcrElement],
    recognized_tables: &[crate::RecognizedTable],
) -> (
    Vec<&'a crate::layout::LayoutDetection>,
    Vec<&'a crate::types::OcrElement>,
) {
    // `prepare_layout_ocr` already applies the page-aware row/column ordering. ~keep
    let ordered_detections = detections.iter().collect::<Vec<_>>();
    let mut unmatched = elements
        .iter()
        .filter(|element| !element_claimed_by_layout(element, detections, recognized_tables))
        .collect::<Vec<_>>();
    unmatched.sort_by_key(|element| ocr_element_position(element));
    (ordered_detections, unmatched)
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn assemble_cached_layout_elements(
    whole_image_doc: &InternalDocument,
    detections: &[crate::layout::LayoutDetection],
    recognized_tables: &[crate::RecognizedTable],
    elements: &[crate::types::OcrElement],
    image_width: u32,
    image_height: u32,
) -> InternalDocument {
    let mut assigned = elements
        .iter()
        .map(|element| {
            recognized_tables
                .iter()
                .any(|table| bbox_contains_element(table.detection_bbox, element))
        })
        .collect::<Vec<_>>();
    let mut builder = InternalDocumentBuilder::new("image");
    let mut formulas = Vec::new();
    let (ordered_detections, unmatched) = ordered_cached_layout_items(detections, elements, recognized_tables);
    let mut unmatched_index = 0;
    for detection in ordered_detections {
        let detection_position = (detection.bbox.y1.max(0.0) as u32, detection.bbox.x1.max(0.0) as u32);
        let next_index = unmatched[unmatched_index..]
            .partition_point(|element| ocr_element_position(element) < detection_position)
            + unmatched_index;
        push_unmatched_ocr(&mut builder, &unmatched[unmatched_index..next_index]);
        unmatched_index = next_index;
        push_cached_layout_region(
            &mut builder,
            &mut formulas,
            detection,
            recognized_tables,
            elements,
            &mut assigned,
        );
    }
    push_unmatched_ocr(&mut builder, &unmatched[unmatched_index..]);

    finish_cached_layout_document(
        builder,
        whole_image_doc,
        detections,
        formulas,
        image_width,
        image_height,
    )
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn cached_document_has_structure(document: &InternalDocument) -> bool {
    use crate::types::internal::ElementKind;

    document.tables.iter().any(|table| {
        !table.markdown.trim().is_empty() || table.cells.iter().flatten().any(|cell| !cell.trim().is_empty())
    }) || document.elements.iter().any(|element| {
        matches!(
            element.kind,
            ElementKind::Heading { .. } | ElementKind::Code | ElementKind::Formula | ElementKind::ListItem { .. }
        )
    })
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn try_assemble_cached_layout_document(
    whole_image_doc: &InternalDocument,
    detections: &[crate::layout::LayoutDetection],
    recognized_tables: &[crate::RecognizedTable],
    image_width: u32,
    image_height: u32,
) -> Option<InternalDocument> {
    if !cached_layout_adds_structure(detections, recognized_tables) {
        return None;
    }
    let elements = cached_layout_elements(whole_image_doc, image_width, image_height)?;
    let assembled = assemble_cached_layout_elements(
        whole_image_doc,
        detections,
        recognized_tables,
        &elements,
        image_width,
        image_height,
    );
    if !cached_document_has_structure(&assembled) {
        return None;
    }
    let retained_tokens = alphanumeric_token_retention(
        &crate::rendering::render_plain(&assembled),
        &internal_document_text(whole_image_doc),
    );
    (retained_tokens >= REQUIRED_CACHED_LAYOUT_TOKEN_RETENTION).then_some(assembled)
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn layout_regions_from_detections(
    detections: &[crate::layout::LayoutDetection],
    image_width: u32,
    image_height: u32,
) -> Vec<crate::types::LayoutRegion> {
    let page_area = f64::from(image_width) * f64::from(image_height);
    if page_area == 0.0 {
        return Vec::new();
    }
    detections
        .iter()
        .filter_map(|detection| {
            let bbox = clipped_layout_bbox(detection.bbox, image_width, image_height)?;
            Some(crate::types::LayoutRegion {
                class_name: detection.class_name.to_string(),
                confidence: f64::from(detection.confidence),
                bounding_box: crate::types::BoundingBox {
                    x0: f64::from(bbox.x1),
                    y0: f64::from(bbox.y1),
                    x1: f64::from(bbox.x2),
                    y1: f64::from(bbox.y2),
                },
                area_fraction: f64::from(bbox.area()) / page_area,
            })
        })
        .collect()
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn clipped_layout_bbox(bbox: crate::layout::BBox, image_width: u32, image_height: u32) -> Option<crate::layout::BBox> {
    if ![bbox.x1, bbox.y1, bbox.x2, bbox.y2]
        .iter()
        .all(|value| value.is_finite())
    {
        return None;
    }
    let max_x = image_width as f32;
    let max_y = image_height as f32;
    let clipped = crate::layout::BBox::new(
        bbox.x1.clamp(0.0, max_x),
        bbox.y1.clamp(0.0, max_y),
        bbox.x2.clamp(0.0, max_x),
        bbox.y2.clamp(0.0, max_y),
    );
    (clipped.x2 > clipped.x1 && clipped.y2 > clipped.y1).then_some(clipped)
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn sanitize_layout_detections(
    detections: &mut Vec<crate::layout::LayoutDetection>,
    image_width: u32,
    image_height: u32,
) {
    detections.retain_mut(|detection| {
        let Some(bbox) = clipped_layout_bbox(detection.bbox, image_width, image_height) else {
            return false;
        };
        detection.bbox = bbox;
        true
    });
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn try_retain_canonical_whole_image_ocr(
    whole_image_doc: &InternalDocument,
    detections: &[crate::layout::LayoutDetection],
    image_width: u32,
    image_height: u32,
    source_is_single_frame: bool,
) -> Option<InternalDocument> {
    if !source_is_single_frame {
        return None;
    }
    let retention = whole_image_layout_mapping_retention(whole_image_doc, detections, image_width, image_height);
    let mut retained = whole_image_doc.clone();
    let pages = retained.prebuilt_pages.as_mut()?;
    if pages.len() != 1 || pages[0].page_number != 1 {
        return None;
    }

    // A successful single-frame whole-image OCR is the lossless terminal fallback;
    // repeating OCR per region is expensive and can discard canonical tokens. ~keep
    pages[0].layout_regions = Some(layout_regions_from_detections(detections, image_width, image_height));
    tracing::debug!(
        ?retention,
        "Retained canonical whole-image OCR after cached layout assembly was unavailable"
    );
    Some(retained)
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
async fn detect_image_layout(
    content: &[u8],
    layout_config: crate::core::config::LayoutDetectionConfig,
    thread_budget: usize,
    security_limits: crate::extractors::security::SecurityLimits,
) -> Result<(image::RgbImage, crate::layout::DetectionResult)> {
    let layout_content = content.to_vec();
    tokio::task::spawn_blocking(move || -> Result<_> {
        let rgb =
            crate::extraction::image::decode_image_to_rgb8_with_security_limits(&layout_content, &security_limits)?;
        drop(layout_content);
        let mut engine = crate::layout::take_or_create_engine(&layout_config, thread_budget)
            .map_err(|error| crate::XbergError::Other(format!("Layout engine init failed: {error}")))?;
        let detection = engine.detect_with_security_limits(&rgb, &security_limits);
        crate::layout::return_engine(engine);
        let detection =
            detection.map_err(|error| crate::XbergError::Other(format!("Layout detection failed: {error}")))?;
        Ok((rgb, detection))
    })
    .await
    .map_err(|error| crate::XbergError::Other(format!("Image layout worker failed: {error}")))?
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn sort_layout_detections(detections: &mut [crate::layout::LayoutDetection], image_height: u32) {
    let row_threshold = (image_height as f32 * LAYOUT_READING_ORDER_ROW_HEIGHT_RATIO).max(1.0);
    detections.sort_by(|left, right| {
        let left_y = (left.bbox.y1 + left.bbox.y2) / 2.0;
        let right_y = (right.bbox.y1 + right.bbox.y2) / 2.0;
        let left_row = (left_y / row_threshold) as i64;
        let right_row = (right_y / row_threshold) as i64;
        left_row.cmp(&right_row).then_with(|| {
            let left_x = (left.bbox.x1 + left.bbox.x2) / 2.0;
            let right_x = (right.bbox.x1 + right.bbox.x2) / 2.0;
            left_x.total_cmp(&right_x)
        })
    });
}

/// Shared blocking-wrapper entry in the recognition module.
#[cfg(all(feature = "formula-recognition", any(feature = "ocr", feature = "ocr-wasm")))]
use crate::formula_recognition::recognize_crop_blocking as recognize_formula_crop_blocking;

/// Recognize the formula regions of an assembled cached-layout document,
/// replacing the plain OCR text in both the side channel and the matching
/// element. Failures keep the OCR text.
#[cfg(all(feature = "formula-recognition", any(feature = "ocr", feature = "ocr-wasm")))]
async fn recognize_assembled_formula_regions(
    doc: &mut InternalDocument,
    rgb: &image::RgbImage,
    detections: &[crate::layout::LayoutDetection],
    layout: &crate::core::config::LayoutDetectionConfig,
) {
    if layout.formula_model.is_none() {
        return;
    }
    for detection in detections
        .iter()
        .filter(|d| matches!(d.class_name, crate::layout::LayoutClass::Formula))
    {
        let Some(crop) = crop_layout_region(rgb, detection) else {
            continue;
        };
        match recognize_formula_crop_blocking(crop, layout.acceleration.clone()).await {
            Ok(Some(latex)) => {
                let target = crate::types::BoundingBox {
                    x0: detection.bbox.x1 as f64,
                    y0: detection.bbox.y1 as f64,
                    x1: detection.bbox.x2 as f64,
                    y1: detection.bbox.y2 as f64,
                };
                // The layout pass pushed the side-channel formula and its
                // element together from the same detection, so both hold the
                // detection's values; match the formula by its own bbox and
                // its element by the not-yet-replaced OCR text, front to back,
                // so repeated text on one page pairs positionally.
                for formula in doc.formulas.iter_mut() {
                    if formula.bbox == Some(target) && formula.latex != latex {
                        let ocr_text = formula.latex.clone();
                        if let Some(element) = doc.elements.iter_mut().find(|e| {
                            matches!(e.kind, crate::types::internal::ElementKind::Formula) && e.text == ocr_text
                        }) {
                            element.text = latex.clone();
                        }
                        formula.latex = latex.clone();
                        break;
                    }
                }
            }
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(error = %error, "cached-path formula recognition failed; keeping OCR text");
            }
        }
    }
}

/// Crop the detection's region out of the page raster. `None` when the
/// clamped region is empty.
#[cfg(all(feature = "formula-recognition", any(feature = "ocr", feature = "ocr-wasm")))]
fn crop_layout_region(rgb: &image::RgbImage, detection: &crate::layout::LayoutDetection) -> Option<image::RgbImage> {
    let (x, y, w, h) = detection.bbox.clamp_to_image(rgb.width(), rgb.height())?;
    Some(image::imageops::crop_imm(rgb, x, y, w, h).to_image())
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn encode_layout_region(rgb: &image::RgbImage, detection: &crate::layout::LayoutDetection) -> Result<Option<Vec<u8>>> {
    use image::ImageEncoder;

    if matches!(
        detection.class_name,
        crate::layout::LayoutClass::Picture | crate::layout::LayoutClass::Chart
    ) {
        return Ok(None);
    }
    let Some((x1, y1, crop_width, crop_height)) = detection.bbox.clamp_to_image(rgb.width(), rgb.height()) else {
        return Ok(None);
    };
    if crop_width < MIN_LAYOUT_CROP_DIMENSION || crop_height < MIN_LAYOUT_CROP_DIMENSION {
        return Ok(None);
    }
    let crop = image::imageops::crop_imm(rgb, x1, y1, crop_width, crop_height).to_image();
    let mut png = std::io::Cursor::new(Vec::new());
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(
            crop.as_raw(),
            crop.width(),
            crop.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|error| crate::XbergError::Other(format!("Failed to encode crop as PNG: {error}")))?;
    Ok(Some(png.into_inner()))
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn build_region_ocr_document(
    builder: InternalDocumentBuilder,
    formulas: Vec<crate::types::Formula>,
    processing_warnings: Vec<crate::types::ProcessingWarning>,
) -> InternalDocument {
    let mut doc = builder.build();
    doc.metadata = Metadata {
        output_format: Some("markdown".to_string()),
        ..Default::default()
    };
    doc.formulas = formulas;
    doc.processing_warnings = processing_warnings;
    ImageExtractor::mark_ocr_extraction(&mut doc);
    doc
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
async fn extract_layout_regions(
    backend: std::sync::Arc<dyn crate::plugins::OcrBackend>,
    rgb: &image::RgbImage,
    detections: &[crate::layout::LayoutDetection],
    ocr_config: &crate::core::config::OcrConfig,
    layout_config: Option<&crate::core::config::LayoutDetectionConfig>,
    cancel_token: Option<&crate::cancellation::CancellationToken>,
) -> Result<InternalDocument> {
    let mut builder = InternalDocumentBuilder::new("image");
    let mut formulas = Vec::new();
    let mut processing_warnings = Vec::new();
    #[cfg(not(feature = "formula-recognition"))]
    let _ = layout_config;
    for detection in detections {
        // A timed-out extraction cancels this token (see
        // `ExtractionConfig::ensure_cancel_token`); each region can involve a
        // formula-recognition or OCR call, so stop dispatching further regions
        // once cancelled rather than continuing until the last one finishes.
        if cancel_token.is_some_and(crate::cancellation::CancellationToken::is_cancelled) {
            processing_warnings.push(crate::core::diagnostics::warning(
                "layout-ocr",
                "extraction cancelled; remaining layout regions were not OCR'd".to_string(),
            ));
            break;
        }
        // A configured formula model takes the region crop directly; the
        // plain OCR text is the fallback when recognition yields nothing.
        #[cfg(feature = "formula-recognition")]
        if matches!(detection.class_name, crate::layout::LayoutClass::Formula)
            && let Some(layout) = layout_config
            && layout.formula_model.is_some()
            && let Some(crop) = crop_layout_region(rgb, detection)
        {
            match recognize_formula_crop_blocking(crop, layout.acceleration.clone()).await {
                Ok(Some(latex)) => {
                    push_mapped_layout_text(&mut builder, &mut formulas, detection, &latex);
                    continue;
                }
                Ok(None) => {}
                Err(error) => {
                    tracing::warn!(error = %error, "formula recognition failed; using OCR text for the region");
                    processing_warnings.push(crate::core::diagnostics::warning(
                        "formula-recognition",
                        format!("formula region fell back to OCR text: {error}"),
                    ));
                }
            }
        }
        let Some(crop_bytes) = encode_layout_region(rgb, detection)? else {
            continue;
        };
        let ocr_result = backend.process_image(&crop_bytes, ocr_config).await?;
        processing_warnings.extend(ocr_result.processing_warnings);
        let text = ocr_result.content.trim();
        if text.is_empty() {
            continue;
        }
        tracing::trace!(
            class = ?detection.class_name,
            confidence = detection.confidence,
            text_len = text.len(),
            "OCR result for layout region"
        );
        push_mapped_layout_text(&mut builder, &mut formulas, detection, text);
    }
    Ok(build_region_ocr_document(builder, formulas, processing_warnings))
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
async fn extract_selected_image_ocr_path<Layout, LayoutFuture, Whole, WholeFuture>(
    use_layout: bool,
    layout: Layout,
    whole: Whole,
) -> Result<InternalDocument>
where
    Layout: FnOnce() -> LayoutFuture,
    LayoutFuture: std::future::Future<Output = Result<InternalDocument>>,
    Whole: FnOnce() -> WholeFuture,
    WholeFuture: std::future::Future<Output = Result<InternalDocument>>,
{
    if use_layout { layout().await } else { whole().await }
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
enum LayoutOcrPreparation {
    Complete(InternalDocument),
    Detected {
        whole_image_result: Result<InternalDocument>,
        rgb: std::sync::Arc<image::RgbImage>,
        detections: Vec<crate::layout::LayoutDetection>,
    },
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn share_detected_image(rgb: image::RgbImage) -> std::sync::Arc<image::RgbImage> {
    std::sync::Arc::new(rgb)
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
async fn prepare_layout_ocr(
    extractor: &ImageExtractor,
    content: &[u8],
    mime_type: &str,
    config: &ExtractionConfig,
    layout_config: crate::core::config::LayoutDetectionConfig,
) -> Result<LayoutOcrPreparation> {
    let whole_image_result = extractor.extract_with_ocr(content, mime_type, config).await;
    let thread_budget = crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());
    let security_limits = config.security_limits.clone().unwrap_or_default();
    let (rgb, detection) = match detect_image_layout(content, layout_config, thread_budget, security_limits).await {
        Ok(result) => result,
        Err(error) => {
            return cached_whole_image_after_layout_error(&whole_image_result, error)
                .map(LayoutOcrPreparation::Complete);
        }
    };
    let rgb = share_detected_image(rgb);
    tracing::info!(
        detections = detection.detections.len(),
        img_width = rgb.width(),
        img_height = rgb.height(),
        "Layout detection completed for image"
    );
    if detection.detections.is_empty() {
        tracing::debug!("No layout regions detected, retaining whole-image OCR");
        return whole_image_result.map(LayoutOcrPreparation::Complete);
    }
    let mut detections = detection.detections;
    sanitize_layout_detections(&mut detections, rgb.width(), rgb.height());
    if detections.is_empty() {
        tracing::debug!("No valid layout regions detected, retaining whole-image OCR");
        return whole_image_result.map(LayoutOcrPreparation::Complete);
    }
    sort_layout_detections(&mut detections, rgb.height());
    Ok(LayoutOcrPreparation::Detected {
        whole_image_result,
        rgb,
        detections,
    })
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn configured_region_ocr(
    config: &ExtractionConfig,
    ocr_config: &crate::core::config::OcrConfig,
) -> Result<(
    std::sync::Arc<dyn crate::plugins::OcrBackend>,
    crate::core::config::OcrConfig,
)> {
    crate::plugins::ensure_ocr_backends_initialized();
    let registry = crate::plugins::registry::get_ocr_backend_registry();
    let backend = registry.read().get(&ocr_config.backend)?;
    let mut region_config = ocr_config.clone();
    region_config.output_format = Some(crate::core::config::OutputFormat::Plain);
    if region_config.backend == "tesseract" {
        apply_default_tesseract_psm(&mut region_config, LAYOUT_REGION_TESSERACT_PSM);
        // Layout assembly consumes region text only; skip redundant Tesseract hOCR and
        // document-level table reconstruction when region OCR is unavoidable. ~keep
        let tesseract_config = region_config.tesseract_config.get_or_insert_default();
        tesseract_config.output_format = "text".to_string();
        tesseract_config.enable_table_detection = false;
    }
    if region_config.acceleration.is_none() {
        region_config.acceleration = config.acceleration.clone();
    }
    Ok((backend, region_config))
}

/// Applies `psm` whenever the caller has not explicitly set one, whether or not a
/// `TesseractConfig` is present. Keyed on the `psm` field itself, not on struct
/// presence (#1573) — an explicitly materialised `TesseractConfig` that leaves `psm`
/// unset still gets the automatic default, and any other field the caller did set is
/// preserved rather than overwritten.
#[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
fn apply_default_tesseract_psm(config: &mut crate::core::config::OcrConfig, psm: i32) {
    if config.backend != "tesseract" {
        return;
    }

    let tesseract_config = config
        .tesseract_config
        .get_or_insert_with(|| crate::types::TesseractConfig {
            language: config.language.clone(),
            ..Default::default()
        });
    if tesseract_config.psm.is_none() {
        tesseract_config.psm = Some(psm);
    }
}

/// Also applied by the PDF OCR routes to a page that is one full-page raster, so a scanned
/// page and the same page OCR'd as an image use the same segmentation mode (#1786).
#[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
pub(crate) fn apply_default_whole_image_tesseract_psm(config: &mut crate::core::config::OcrConfig) {
    let psm = if has_vertical_tesseract_language(config) {
        VERTICAL_BLOCK_TESSERACT_PSM
    } else {
        WHOLE_IMAGE_TESSERACT_PSM
    };
    apply_default_tesseract_psm(config, psm);
}

/// Checks the same reconciled language `config_to_tesseract` resolves (#1572), so a
/// `jpn_vert` set only on `tesseract_config.language` still selects the vertical PSM.
#[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
fn has_vertical_tesseract_language(config: &crate::core::config::OcrConfig) -> bool {
    config
        .effective_tesseract_language()
        .iter()
        .flat_map(|language| language.split('+'))
        .any(|language| language.trim().to_ascii_lowercase().ends_with("_vert"))
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn usable_word_confidences(result: &crate::types::ExtractedDocument) -> Vec<f64> {
    result
        .ocr_elements
        .iter()
        .flatten()
        .filter(|element| {
            element.level == crate::types::OcrElementLevel::Word
                && !element.text.trim().is_empty()
                && element.confidence.recognition.is_finite()
        })
        .map(|element| element.confidence.recognition)
        .collect()
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn should_retry_sparse_image_ocr(
    config: &crate::core::config::OcrConfig,
    result: &crate::types::ExtractedDocument,
) -> bool {
    is_implicit_horizontal_tesseract(config)
        && usable_word_confidences(result).len() <= SPARSE_IMAGE_OCR_WORD_LIMIT
        && !has_robust_word_confidence_distribution(result)
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn is_implicit_horizontal_tesseract(config: &crate::core::config::OcrConfig) -> bool {
    config.backend == "tesseract"
        && config.tesseract_config.as_ref().and_then(|c| c.psm).is_none()
        && !has_vertical_tesseract_language(config)
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn has_robust_word_confidence_distribution(result: &crate::types::ExtractedDocument) -> bool {
    let mut confidences = usable_word_confidences(result);
    if confidences.is_empty() {
        return false;
    }
    confidences.sort_by(f64::total_cmp);
    let percentile_index = ((confidences.len() as f64 - 1.0) * SPARSE_IMAGE_OCR_CONFIDENCE_PERCENTILE).floor() as usize;
    let low_confidence_count = confidences
        .iter()
        .filter(|confidence| **confidence < SPARSE_IMAGE_OCR_MIN_WORD_CONFIDENCE)
        .count();
    let low_confidence_ratio = low_confidence_count as f64 / confidences.len() as f64;

    confidences[percentile_index] >= SPARSE_IMAGE_OCR_MIN_WORD_CONFIDENCE
        && low_confidence_ratio <= SPARSE_IMAGE_OCR_MAX_LOW_CONFIDENCE_RATIO
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn sparse_image_ocr_fallback_config(
    whole_image_config: &crate::core::config::OcrConfig,
) -> crate::core::config::OcrConfig {
    let mut fallback_config = whole_image_config.clone();
    let tesseract_config = fallback_config.tesseract_config.get_or_insert_default();
    tesseract_config.psm = Some(SPARSE_IMAGE_OCR_FALLBACK_PSM);
    let preprocessing = crate::types::ImagePreprocessingConfig {
        deskew: false,
        contrast_enhance: true,
        binarization_method: "none".to_string(),
        ..Default::default()
    };
    tesseract_config.preprocessing = Some(preprocessing);
    fallback_config
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn is_implicit_strong_rtl_tesseract(config: &crate::core::config::OcrConfig) -> bool {
    is_implicit_horizontal_tesseract(config)
        && config
            .effective_tesseract_language()
            .iter()
            .any(|language| crate::core::config::ocr::is_strong_rtl_tesseract_language(language))
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn rtl_sparse_image_ocr_fallback_config(
    whole_image_config: &crate::core::config::OcrConfig,
) -> crate::core::config::OcrConfig {
    let mut fallback_config = whole_image_config.clone();
    fallback_config.tesseract_config.get_or_insert_default().psm = Some(RTL_SPARSE_IMAGE_OCR_FALLBACK_PSM);
    fallback_config
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn is_strong_rtl_letter(character: char) -> bool {
    character.is_alphabetic()
        && matches!(
            character as u32,
            0x0590..=0x05ff | 0x0600..=0x06ff | 0x0750..=0x077f | 0x08a0..=0x08ff
                | 0xfb1d..=0xfdff | 0xfe70..=0xfeff
        )
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn normalized_ocr_tokens(content: &str) -> Vec<String> {
    content
        .split_whitespace()
        .filter_map(|token| {
            let normalized = token
                .trim_matches(|character: char| !character.is_alphanumeric() && !":/_@.-".contains(character))
                .to_lowercase();
            (!normalized.is_empty()).then_some(normalized)
        })
        .collect()
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn normalized_identifier_slot(token: &str) -> Option<String> {
    let token = token.trim_matches(|character: char| !character.is_alphanumeric());
    let characters = token.chars().collect::<Vec<_>>();
    let leading_digits = characters.iter().take_while(|character| character.is_numeric()).count();
    let attached_rtl_list_item = (1..=2).contains(&leading_digits)
        && characters
            .get(leading_digits)
            .is_some_and(|character| matches!(*character, '.' | ')'))
        && characters
            .get(leading_digits + 1)
            .is_some_and(|character| character.is_alphabetic())
        && characters[leading_digits + 1..]
            .iter()
            .any(|character| is_strong_rtl_letter(*character));
    if attached_rtl_list_item {
        return None;
    }
    let alphanumeric = characters
        .iter()
        .filter(|character| character.is_alphanumeric())
        .count();
    if alphanumeric < 3 || token.contains('\u{fffd}') {
        return None;
    }
    let has_internal_separator = characters.iter().enumerate().any(|(index, character)| {
        !character.is_alphanumeric()
            && characters[..index].iter().any(|character| character.is_alphanumeric())
            && characters[index + 1..]
                .iter()
                .any(|character| character.is_alphanumeric())
    });
    let digits = characters.iter().filter(|character| character.is_ascii_digit()).count();
    let letters = characters
        .iter()
        .filter(|character| character.is_ascii_alphabetic())
        .count();
    let is_long_numeric = digits >= 4 && letters == 0;
    let is_compact_identifier = digits > 0 && letters > 0 && alphanumeric >= 6;
    (has_internal_separator || is_long_numeric || is_compact_identifier).then(|| token.to_lowercase())
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn identifier_slot_shape(token: &str) -> String {
    let mut shape = String::new();
    let mut previous_class = None;
    for character in token.chars() {
        let class = if character.is_alphabetic() {
            Some('a')
        } else if character.is_numeric() {
            Some('0')
        } else {
            None
        };
        if let Some(class) = class {
            if previous_class != Some(class) {
                shape.push(class);
            }
            previous_class = Some(class);
        } else {
            shape.push(character);
            previous_class = None;
        }
    }
    shape
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn normalized_edit_distance(left: &str, right: &str) -> usize {
    let right = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    for (left_index, left_character) in left.chars().enumerate() {
        let mut current = Vec::with_capacity(right.len() + 1);
        current.push(left_index + 1);
        for (right_index, right_character) in right.iter().enumerate() {
            current.push(std::cmp::min(
                std::cmp::min(current[right_index] + 1, previous[right_index + 1] + 1),
                previous[right_index] + usize::from(left_character != *right_character),
            ));
        }
        previous = current;
    }
    previous[right.len()]
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn identifier_slots(content: &str) -> Vec<String> {
    content
        .split_whitespace()
        .filter_map(normalized_identifier_slot)
        .collect()
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn identifier_slots_are_preserved(primary: &str, candidate: &str) -> bool {
    fn compatible(primary: &str, candidate: &str) -> bool {
        if identifier_slot_shape(primary) != identifier_slot_shape(candidate) {
            return false;
        }
        let length = primary.chars().count().max(candidate.chars().count());
        normalized_edit_distance(primary, candidate) <= length.div_ceil(5).clamp(1, 2)
    }

    fn assign(
        primary_index: usize,
        compatibility: &[Vec<bool>],
        seen: &mut [bool],
        candidate_owner: &mut [Option<usize>],
    ) -> bool {
        for candidate_index in 0..candidate_owner.len() {
            if !compatibility[primary_index][candidate_index] || seen[candidate_index] {
                continue;
            }
            seen[candidate_index] = true;
            let can_assign = match candidate_owner[candidate_index] {
                Some(owner) => assign(owner, compatibility, seen, candidate_owner),
                None => true,
            };
            if can_assign {
                candidate_owner[candidate_index] = Some(primary_index);
                return true;
            }
        }
        false
    }

    let primary = identifier_slots(primary);
    let candidate = identifier_slots(candidate);
    if primary.len() != candidate.len() {
        return false;
    }
    let compatibility = primary
        .iter()
        .map(|primary| {
            candidate
                .iter()
                .map(|candidate| compatible(primary, candidate))
                .collect()
        })
        .collect::<Vec<Vec<_>>>();
    let mut candidate_owner = vec![None; candidate.len()];
    (0..primary.len()).all(|primary_index| {
        let mut seen = vec![false; candidate.len()];
        assign(primary_index, &compatibility, &mut seen, &mut candidate_owner)
    })
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn is_rtl_identifier_element(element: &crate::types::OcrElement) -> bool {
    element.level == crate::types::OcrElementLevel::Word
        && normalized_identifier_slot(&element.text).is_some()
        && element
            .backend_metadata
            .get("word_language")
            .and_then(serde_json::Value::as_str)
            .is_some_and(crate::core::config::ocr::is_strong_rtl_tesseract_language)
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn identifier_slots_are_preserved_after_recovery(
    primary: &crate::types::ExtractedDocument,
    candidate: &crate::types::ExtractedDocument,
) -> bool {
    if identifier_slots_are_preserved(&primary.content, &candidate.content) {
        return true;
    }
    let mut primary_slots = identifier_slots(&primary.content);
    let mut candidate_slots = identifier_slots(&candidate.content);
    for recovered in candidate.ocr_elements.iter().flatten().filter(|element| {
        element
            .backend_metadata
            .get("recovered_from")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|source| compatible_identifier_recovery(source, &element.text))
    }) {
        let Some(candidate_slot) = normalized_identifier_slot(&recovered.text) else {
            return false;
        };
        if primary_slots.contains(&candidate_slot) {
            continue;
        }
        let mut primary_matches = primary
            .ocr_elements
            .iter()
            .flatten()
            .filter(|element| compatible_identifier_recovery(&element.text, &recovered.text));
        let Some(primary_element) = primary_matches.next() else {
            return false;
        };
        if primary_matches.next().is_some() {
            return false;
        }
        let Some(candidate_index) = candidate_slots.iter().position(|slot| slot == &candidate_slot) else {
            return false;
        };
        if let Some(primary_slot) = normalized_identifier_slot(&primary_element.text) {
            let Some(primary_index) = primary_slots.iter().position(|slot| slot == &primary_slot) else {
                return false;
            };
            primary_slots.remove(primary_index);
        }
        candidate_slots.remove(candidate_index);
    }
    identifier_slots_are_preserved(&primary_slots.join(" "), &candidate_slots.join(" "))
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
pub(crate) fn is_mixed_ltr_recovery_eligible_config(config: &crate::core::config::OcrConfig) -> bool {
    let languages = config
        .effective_tesseract_language()
        .into_iter()
        .flat_map(|language| {
            language
                .split('+')
                .map(|part| part.trim().to_string())
                .collect::<Vec<_>>()
        })
        .collect::<std::collections::HashSet<_>>();
    is_implicit_strong_rtl_tesseract(config)
        && languages.contains("eng")
        && languages
            .iter()
            .any(|language| crate::core::config::ocr::is_strong_rtl_tesseract_language(language))
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn internal_rtl_recovery_config(config: &crate::core::config::OcrConfig) -> crate::core::config::OcrConfig {
    let mut config = config.clone();
    enable_image_ocr_elements(&mut config, true);
    config.element_config.get_or_insert_default().min_confidence = 0.0;
    config
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn apply_rtl_capture_public_policy(
    config: &crate::core::config::OcrConfig,
    mut document: crate::types::ExtractedDocument,
) -> crate::types::ExtractedDocument {
    if is_mixed_ltr_recovery_eligible_config(config) {
        document.ocr_elements = config.select_public_elements(document.ocr_elements.take());
    }
    document
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn compatible_identifier_recovery(source: &str, candidate: &str) -> bool {
    let source = source.trim_matches(|character: char| !character.is_ascii_alphanumeric());
    let candidate = candidate.trim_matches(|character: char| !character.is_ascii_alphanumeric());
    let alnum = |text: &str| {
        text.chars()
            .filter(char::is_ascii_alphanumeric)
            .map(|character| character.to_ascii_lowercase())
            .collect::<String>()
    };
    if alnum(source) == alnum(candidate) {
        return source != candidate;
    }
    candidate.char_indices().any(|(index, character)| {
        if !character.is_ascii_digit() {
            return false;
        }
        let suffix_index = candidate[..index]
            .char_indices()
            .next_back()
            .filter(|(_, character)| "-._/".contains(*character))
            .map_or(index, |(separator_index, _)| separator_index);
        let suffix = &candidate[suffix_index..];
        if suffix.len() < 3
            || suffix.chars().filter(char::is_ascii_digit).count() < 2
            || !suffix.chars().any(|character| "-._/".contains(character))
            || !source.ends_with(suffix)
        {
            return false;
        }
        let source_prefix = source[..source.len() - suffix.len()]
            .chars()
            .filter(|character| !"-._/".contains(*character))
            .collect::<String>();
        let candidate_prefix = candidate[..suffix_index]
            .chars()
            .filter(|character| !"-._/".contains(*character))
            .collect::<String>();
        (1..=6).contains(&source_prefix.len())
            && (1..=5).contains(&candidate_prefix.len())
            && source_prefix.chars().all(|character| character.is_ascii_digit())
            && candidate_prefix
                .chars()
                .all(|character| character.is_ascii_alphabetic())
    })
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn token_occurrences(text: &str, token: &str) -> Vec<usize> {
    text.match_indices(token)
        .filter_map(|(start, _)| {
            let end = start + token.len();
            (text[..start]
                .chars()
                .next_back()
                .is_none_or(|character| !character.is_ascii_alphanumeric())
                && text[end..]
                    .chars()
                    .next()
                    .is_none_or(|character| !character.is_ascii_alphanumeric()))
            .then_some(start)
        })
        .collect()
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn replace_unique_token(text: &mut String, source: &str, replacement: &str) -> std::result::Result<bool, ()> {
    let occurrences = token_occurrences(text, source);
    match occurrences.as_slice() {
        [] => Ok(false),
        [offset] => {
            text.replace_range(*offset..*offset + source.len(), replacement);
            Ok(true)
        }
        _ => Err(()),
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn update_recovery_tables(
    tables: &mut [crate::types::Table],
    source: &str,
    replacement: &str,
) -> std::result::Result<bool, ()> {
    let cell_occurrences = tables
        .iter()
        .flat_map(|table| table.cells.iter().flatten())
        .map(|cell| token_occurrences(cell, source).len())
        .sum::<usize>();
    let column_occurrences = tables
        .iter()
        .flat_map(|table| table.columns.iter().flatten())
        .map(|column| token_occurrences(column, source).len())
        .sum::<usize>();
    if cell_occurrences > 1 || column_occurrences > 1 {
        return Err(());
    }
    let mut updated = false;
    for table in tables {
        let markdown_contained_source = !token_occurrences(&table.markdown, source).is_empty();
        let mut table_updated = false;
        for row in &mut table.cells {
            for cell in row {
                table_updated |= replace_unique_token(cell, source, replacement)?;
            }
        }
        if let Some(columns) = table.columns.as_mut() {
            for column in columns {
                table_updated |= replace_unique_token(column, source, replacement)?;
            }
        }
        if markdown_contained_source && !table_updated {
            return Err(());
        }
        if table_updated {
            table.markdown = crate::rendering::common::render_table_markdown(&table.cells);
            updated = true;
        }
    }
    Ok(updated)
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn apply_identifier_recovery_transaction(
    selected: &mut crate::types::ExtractedDocument,
    index: usize,
    source: &str,
    evidence: &crate::types::OcrElement,
) -> bool {
    let source_core = source.trim_matches(|character: char| !character.is_ascii_alphanumeric());
    let evidence_core = evidence
        .text
        .trim_matches(|character: char| !character.is_ascii_alphanumeric());
    let prefix_len = source.find(source_core).unwrap_or(0);
    let replacement_element = format!(
        "{}{}{}",
        &source[..prefix_len],
        evidence_core,
        &source[prefix_len + source_core.len()..]
    );
    let mut updated = selected.clone();
    if replace_unique_token(&mut updated.content, source_core, evidence_core) != Ok(true) {
        return false;
    }
    let Some(element) = updated
        .ocr_elements
        .as_mut()
        .and_then(|elements| elements.get_mut(index))
    else {
        return false;
    };
    if element.text != source {
        return false;
    }
    element.text = replacement_element;
    element.confidence = evidence.confidence.clone();
    element.backend_metadata = evidence.backend_metadata.clone();
    element
        .backend_metadata
        .insert("recovered_from".to_string(), serde_json::json!(source));

    if update_recovery_tables(&mut updated.tables, source_core, evidence_core).is_err() {
        return false;
    }
    if let Some(internal) = updated.ocr_internal_document.as_mut() {
        let mut element_updates = 0;
        for element in &mut internal.elements {
            match replace_unique_token(&mut element.text, source_core, evidence_core) {
                Ok(true) => element_updates += 1,
                Ok(false) => {}
                Err(()) => return false,
            }
        }
        if element_updates > 1 || update_recovery_tables(&mut internal.tables, source_core, evidence_core).is_err() {
            return false;
        }
    }
    *selected = updated;
    true
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn unique_compatible_recovery_evidence<'a>(
    source: &str,
    elements: impl Iterator<Item = &'a crate::types::OcrElement>,
) -> Option<&'a crate::types::OcrElement> {
    let mut matches = elements.filter(|element| {
        element.level == crate::types::OcrElementLevel::Word
            && element.confidence.recognition >= RTL_IDENTIFIER_RECOVERY_MIN_CONFIDENCE
            && compatible_identifier_recovery(
                source,
                element
                    .text
                    .trim_matches(|character: char| !character.is_ascii_alphanumeric()),
            )
    });
    let evidence = matches.next()?;
    matches.next().is_none().then_some(evidence)
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn is_rtl_recovery_suspect(element: &crate::types::OcrElement, has_lost_prefix: bool) -> bool {
    if element.level != crate::types::OcrElementLevel::Word || !element.text.is_ascii() || !has_lost_prefix {
        return false;
    }
    let identifier = element
        .text
        .trim_matches(|character: char| !character.is_ascii_alphanumeric());
    let parts = identifier.split('-').collect::<Vec<_>>();
    let iso_date = parts.len() == 3
        && [4, 2, 2]
            .into_iter()
            .zip(&parts)
            .all(|(length, part)| part.len() == length && part.chars().all(|character| character.is_ascii_digit()));
    if iso_date {
        return false;
    }
    let Some(language) = element
        .backend_metadata
        .get("word_language")
        .and_then(serde_json::Value::as_str)
    else {
        return false;
    };
    let language_is_rtl = crate::core::config::ocr::is_strong_rtl_tesseract_language(language);
    let no_letters = !identifier.chars().any(|character| character.is_ascii_alphabetic());
    let separators = identifier
        .chars()
        .filter(|character| "-._/".contains(*character))
        .count();
    let lost_prefix_fragment = language_is_rtl && no_letters && separators >= 2;
    let short_numeric_fragment = identifier.len() <= 5
        && no_letters
        && separators >= 1
        && identifier.chars().any(|character| character.is_ascii_digit());
    let compact_unseparated_fragment = identifier.len() <= 6
        && identifier.chars().all(|character| character.is_ascii_alphanumeric())
        && identifier.chars().any(|character| character.is_ascii_alphabetic())
        && identifier.chars().any(|character| character.is_ascii_digit());
    (lost_prefix_fragment || short_numeric_fragment || compact_unseparated_fragment)
        && (normalized_identifier_slot(&element.text).is_some() || compact_unseparated_fragment)
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn rtl_recovery_suspects(
    selected: &crate::types::ExtractedDocument,
) -> Vec<(usize, String, crate::types::OcrBoundingGeometry)> {
    let elements = selected.ocr_elements.as_deref().unwrap_or_default();
    let has_lost_prefix = elements.iter().any(|element| {
        is_rtl_identifier_element(element)
            && !element.text.chars().any(|character| character.is_ascii_alphabetic())
            && element
                .text
                .chars()
                .filter(|character| "-._/".contains(*character))
                .count()
                >= 2
    });
    elements
        .iter()
        .enumerate()
        .filter(|(_, element)| is_rtl_recovery_suspect(element, has_lost_prefix))
        .take(RTL_IDENTIFIER_RECOVERY_MAX_CROPS)
        .map(|(index, element)| (index, element.text.clone(), element.geometry.clone()))
        .collect()
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
async fn build_rtl_recovery_crops(
    image_bytes: std::sync::Arc<Vec<u8>>,
    suspects: Vec<(usize, String, crate::types::OcrBoundingGeometry)>,
    processed_width: Option<u32>,
    processed_height: Option<u32>,
    security_limits: crate::extractors::security::SecurityLimits,
) -> crate::Result<Vec<(usize, String, Vec<u8>)>> {
    tokio::task::spawn_blocking(move || {
        let image = crate::extraction::image::decode_image_to_rgb8_with_security_limits(
            image_bytes.as_slice(),
            &security_limits,
        )?;
        let scale_x = image.width() as f64 / f64::from(processed_width.unwrap_or(image.width()));
        let scale_y = image.height() as f64 / f64::from(processed_height.unwrap_or(image.height()));
        let mut crops = Vec::with_capacity(suspects.len());
        for (index, source, geometry) in suspects {
            let crate::types::OcrBoundingGeometry::Rectangle {
                left,
                top,
                width,
                height,
            } = geometry
            else {
                continue;
            };
            let left = (f64::from(left) * scale_x).round() as u32;
            let top = (f64::from(top) * scale_y).round() as u32;
            let width = (f64::from(width) * scale_x).round().max(1.0) as u32;
            let height = (f64::from(height) * scale_y).round().max(1.0) as u32;
            let x = left.saturating_sub(width / 2);
            let y = top.saturating_sub(height);
            let right = left.saturating_add(width + width / 2).min(image.width());
            let bottom = top.saturating_add(height.saturating_mul(2)).min(image.height());
            if right <= x
                || bottom <= y
                || u64::from(right - x) * u64::from(bottom - y) > RTL_IDENTIFIER_RECOVERY_MAX_CROP_PIXELS
            {
                continue;
            }
            let crop = image::imageops::crop_imm(&image, x, y, right - x, bottom - y).to_image();
            let mut encoded = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgb8(crop)
                .write_to(&mut encoded, image::ImageFormat::Png)
                .map_err(|error| crate::XbergError::Other(format!("Failed to encode OCR recovery crop: {error}")))?;
            crops.push((index, source, encoded.into_inner()));
        }
        Ok(crops)
    })
    .await
    .map_err(|error| crate::XbergError::Other(format!("OCR recovery crop worker failed: {error}")))?
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn rtl_identifier_recovery_config(fallback_config: &crate::core::config::OcrConfig) -> crate::core::config::OcrConfig {
    let mut config = fallback_config.clone();
    config.language = vec!["eng".to_string()];
    let tesseract = config.tesseract_config.get_or_insert_default();
    tesseract.language = vec!["eng".to_string()];
    tesseract.psm = Some(7);
    enable_image_ocr_elements(&mut config, true);
    config.element_config.get_or_insert_default().min_confidence = 0.0;
    config
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
async fn recover_selected_rtl_identifiers(
    backend: &std::sync::Arc<dyn crate::plugins::OcrBackend>,
    image_bytes: std::sync::Arc<Vec<u8>>,
    selected: &mut crate::types::ExtractedDocument,
    fallback_config: &crate::core::config::OcrConfig,
    cancel_token: Option<&crate::cancellation::CancellationToken>,
) -> crate::Result<()> {
    if cancel_token.is_some_and(crate::cancellation::CancellationToken::is_cancelled) {
        return Err(crate::XbergError::Cancelled);
    }
    let suspects = rtl_recovery_suspects(selected);
    if suspects.is_empty() {
        return Ok(());
    }
    let processed_dimension = |key: &str| {
        selected
            .metadata
            .additional
            .get(key)
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| *value > 0)
    };
    let processed_width = processed_dimension("ocr_processed_image_width");
    let processed_height = processed_dimension("ocr_processed_image_height");
    let config = rtl_identifier_recovery_config(fallback_config);
    let security_limits = fallback_config.security_limits.clone().unwrap_or_default();
    let crops = build_rtl_recovery_crops(
        image_bytes,
        suspects,
        processed_width,
        processed_height,
        security_limits,
    )
    .await?;
    if cancel_token.is_some_and(crate::cancellation::CancellationToken::is_cancelled) {
        return Err(crate::XbergError::Cancelled);
    }
    let mut recoveries = Vec::with_capacity(crops.len());
    for (index, source, encoded) in crops {
        recoveries.push((index, source, backend.process_image(&encoded, &config).await?));
        if cancel_token.is_some_and(crate::cancellation::CancellationToken::is_cancelled) {
            return Err(crate::XbergError::Cancelled);
        }
    }
    for (index, source, evidence) in recoveries {
        let Some(evidence) = unique_compatible_recovery_evidence(&source, evidence.ocr_elements.iter().flatten())
        else {
            continue;
        };
        apply_identifier_recovery_transaction(selected, index, &source, evidence);
    }
    Ok(())
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
async fn recover_selected_rtl_identifiers_fail_open(
    backend: &std::sync::Arc<dyn crate::plugins::OcrBackend>,
    image_bytes: std::sync::Arc<Vec<u8>>,
    selected: &mut crate::types::ExtractedDocument,
    fallback_config: &crate::core::config::OcrConfig,
    cancel_token: Option<&crate::cancellation::CancellationToken>,
) -> crate::Result<()> {
    match recover_selected_rtl_identifiers(backend, image_bytes, selected, fallback_config, cancel_token).await {
        Ok(()) => Ok(()),
        Err(error @ crate::XbergError::Cancelled) => Err(error),
        Err(error) => {
            tracing::warn!(
                backend = backend.name(),
                stage = "rtl_identifier_recovery",
                %error,
                "optional RTL identifier OCR recovery failed"
            );
            Ok(())
        }
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn normalized_tesseract_mean_confidence(result: &crate::types::ExtractedDocument) -> Option<f64> {
    if let Some(value) = result.metadata.additional.get("mean_text_conf")
        && let Some(confidence) = value
            .as_f64()
            .or_else(|| value.as_i64().map(|value| value as f64))
            .or_else(|| value.as_u64().map(|value| value as f64))
            .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
    {
        return Some(confidence / 100.0);
    }

    let confidences = usable_word_confidences(result)
        .into_iter()
        .filter(|confidence| (0.0..=1.0).contains(confidence))
        .collect::<Vec<_>>();
    (!confidences.is_empty()).then(|| confidences.iter().sum::<f64>() / confidences.len() as f64)
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn has_new_pathological_line_repetition(primary: &str, candidate: &str) -> bool {
    fn lines(content: &str) -> std::collections::HashMap<String, usize> {
        let mut counts = std::collections::HashMap::new();
        for line in content.lines().filter_map(|line| {
            let normalized = normalized_ocr_tokens(line).join(" ");
            (!normalized.is_empty()).then_some(normalized)
        }) {
            *counts.entry(line).or_default() += 1;
        }
        counts
    }

    let primary_lines = lines(primary);
    lines(candidate)
        .into_iter()
        .any(|(line, count)| count >= 3 && count > primary_lines.get(&line).copied().unwrap_or_default())
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn has_new_pathological_trigram_repetition(primary: &str, candidate: &str) -> bool {
    fn trigrams(content: &str) -> std::collections::HashMap<[String; 3], usize> {
        let mut counts = std::collections::HashMap::new();
        for line in content.lines() {
            let tokens = normalized_ocr_tokens(line);
            for words in tokens.windows(3) {
                *counts
                    .entry([words[0].clone(), words[1].clone(), words[2].clone()])
                    .or_default() += 1;
            }
        }
        counts
    }

    let primary_trigrams = trigrams(primary);
    trigrams(candidate)
        .into_iter()
        .any(|(trigram, count)| count >= 3 && count > primary_trigrams.get(&trigram).copied().unwrap_or_default())
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn rtl_retry_candidate_improves(
    primary: &crate::types::ExtractedDocument,
    candidate: &crate::types::ExtractedDocument,
) -> bool {
    let primary_tokens = normalized_ocr_tokens(&primary.content);
    let candidate_tokens = normalized_ocr_tokens(&candidate.content);
    let mut primary_unique = primary_tokens.iter().collect::<std::collections::HashSet<_>>();
    let mut candidate_unique = candidate_tokens.iter().collect::<std::collections::HashSet<_>>();
    for (result, tokens) in [(primary, &mut primary_unique), (candidate, &mut candidate_unique)] {
        for element in result
            .ocr_elements
            .iter()
            .flatten()
            .filter(|element| is_rtl_identifier_element(element))
        {
            for token in normalized_ocr_tokens(&element.text) {
                tokens.remove(&token);
            }
        }
    }
    let primary_rtl = primary_unique
        .iter()
        .filter(|token| token.chars().any(is_strong_rtl_letter))
        .count();
    let candidate_rtl = candidate_unique
        .iter()
        .filter(|token| token.chars().any(is_strong_rtl_letter))
        .count();
    let retained_primary = primary_unique.intersection(&candidate_unique).count();
    let primary_retention = if primary_unique.is_empty() {
        1.0
    } else {
        retained_primary as f64 / primary_unique.len() as f64
    };
    if candidate_rtl <= primary_rtl
        || candidate_unique.len() < primary_unique.len()
        || primary_retention < RTL_SPARSE_IMAGE_OCR_MIN_PRIMARY_TOKEN_RETENTION
    {
        return false;
    }

    if has_new_pathological_line_repetition(&primary.content, &candidate.content)
        || has_new_pathological_trigram_repetition(&primary.content, &candidate.content)
    {
        return false;
    }

    match (
        normalized_tesseract_mean_confidence(primary),
        normalized_tesseract_mean_confidence(candidate),
    ) {
        (Some(primary), Some(candidate)) => candidate + RTL_SPARSE_IMAGE_OCR_MAX_CONFIDENCE_DROP >= primary,
        (Some(_), None) => false,
        _ => true,
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn merge_primary_warnings_into_selected(
    primary: &crate::types::ExtractedDocument,
    selected: &mut crate::types::ExtractedDocument,
) {
    let selected_warnings = std::mem::take(&mut selected.processing_warnings);
    selected.processing_warnings = primary.processing_warnings.clone();
    for warning in selected_warnings {
        crate::core::diagnostics::push_warning_deduped(&mut selected.processing_warnings, warning);
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
fn should_retry_sparse_rtl_ocr(
    original_config: &crate::core::config::OcrConfig,
    effective_config: &crate::core::config::OcrConfig,
    primary: &crate::types::ExtractedDocument,
) -> bool {
    let tokens = normalized_ocr_tokens(&primary.content);
    let unique_tokens = tokens.iter().collect::<std::collections::HashSet<_>>().len();
    let suspect_identifier = primary.ocr_elements.iter().flatten().any(|element| {
        is_rtl_identifier_element(element)
            && !element.text.chars().any(|character| character.is_ascii_alphabetic())
            && element
                .text
                .chars()
                .filter(|character| "-._/".contains(*character))
                .count()
                >= 2
    });
    is_implicit_strong_rtl_tesseract(original_config)
        && effective_config.tesseract_config.as_ref().and_then(|config| config.psm) == Some(WHOLE_IMAGE_TESSERACT_PSM)
        && (suspect_identifier
            || (unique_tokens <= RTL_SPARSE_IMAGE_OCR_UNIQUE_TOKEN_LIMIT
                && tokens.len() <= RTL_SPARSE_IMAGE_OCR_TOTAL_TOKEN_LIMIT))
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
async fn select_rtl_retry_result(
    backend: &std::sync::Arc<dyn crate::plugins::OcrBackend>,
    image_bytes: std::sync::Arc<Vec<u8>>,
    configs: (&crate::core::config::OcrConfig, &crate::core::config::OcrConfig),
    primary: crate::types::ExtractedDocument,
    fallback: crate::types::ExtractedDocument,
    cancel_token: Option<&crate::cancellation::CancellationToken>,
) -> crate::Result<crate::types::ExtractedDocument> {
    let (original_config, fallback_config) = configs;
    let select_fallback = rtl_retry_candidate_improves(&primary, &fallback);
    let recover_identifiers = is_mixed_ltr_recovery_eligible_config(original_config);
    if !select_fallback && !recover_identifiers {
        return Ok(apply_rtl_capture_public_policy(original_config, primary));
    }
    let mut selected = if select_fallback { fallback } else { primary.clone() };
    if select_fallback {
        merge_primary_warnings_into_selected(&primary, &mut selected);
    }
    if recover_identifiers {
        recover_selected_rtl_identifiers_fail_open(backend, image_bytes, &mut selected, fallback_config, cancel_token)
            .await?;
    }
    if select_fallback && !identifier_slots_are_preserved_after_recovery(&primary, &selected) {
        return Ok(apply_rtl_capture_public_policy(original_config, primary));
    }
    Ok(apply_rtl_capture_public_policy(original_config, selected))
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
))]
pub(crate) async fn process_whole_image_ocr_with_sparse_retry(
    backend: &std::sync::Arc<dyn crate::plugins::OcrBackend>,
    image_bytes: std::sync::Arc<Vec<u8>>,
    original_config: &crate::core::config::OcrConfig,
    effective_config: &crate::core::config::OcrConfig,
    whole_page_raster: bool,
    allow_standard_retry: bool,
    cancel_token: Option<&crate::cancellation::CancellationToken>,
) -> crate::Result<crate::types::ExtractedDocument> {
    if cancel_token.is_some_and(crate::cancellation::CancellationToken::is_cancelled) {
        return Err(crate::XbergError::Cancelled);
    }
    let effective_config = if is_mixed_ltr_recovery_eligible_config(original_config) {
        std::borrow::Cow::Owned(internal_rtl_recovery_config(effective_config))
    } else {
        std::borrow::Cow::Borrowed(effective_config)
    };
    let primary = backend
        .process_image_owned(std::sync::Arc::clone(&image_bytes), effective_config.as_ref())
        .await?;
    let rtl_retry = should_retry_sparse_rtl_ocr(original_config, effective_config.as_ref(), &primary);
    if !whole_page_raster && !rtl_retry {
        return Ok(apply_rtl_capture_public_policy(original_config, primary));
    }

    let standard_retry = allow_standard_retry && !rtl_retry && should_retry_sparse_image_ocr(original_config, &primary);
    if !rtl_retry && !standard_retry {
        return Ok(apply_rtl_capture_public_policy(original_config, primary));
    }
    if cancel_token.is_some_and(crate::cancellation::CancellationToken::is_cancelled) {
        return Err(crate::XbergError::Cancelled);
    }

    let fallback_config = if rtl_retry {
        rtl_sparse_image_ocr_fallback_config(effective_config.as_ref())
    } else {
        sparse_image_ocr_fallback_config(effective_config.as_ref())
    };
    let fallback = match backend
        .process_image_owned(std::sync::Arc::clone(&image_bytes), &fallback_config)
        .await
    {
        Ok(fallback) => fallback,
        Err(error @ crate::XbergError::Cancelled) => return Err(error),
        Err(error) => {
            tracing::warn!(%error, "sparse whole-image OCR fallback failed");
            return Ok(apply_rtl_capture_public_policy(original_config, primary));
        }
    };
    if rtl_retry {
        return select_rtl_retry_result(
            backend,
            image_bytes,
            (original_config, &fallback_config),
            primary,
            fallback,
            cancel_token,
        )
        .await;
    }
    if !has_robust_word_confidence_distribution(&fallback) {
        return Ok(apply_rtl_capture_public_policy(original_config, primary));
    }
    let mut selected = fallback;
    merge_primary_warnings_into_selected(&primary, &mut selected);
    Ok(apply_rtl_capture_public_policy(original_config, selected))
}

/// Resize/re-DPI raw image bytes for OCR using `ExtractionConfig::images`
/// (`ImageExtractionConfig`) before handing them to an OCR backend.
///
/// OCR backends only ever see `OcrConfig` (via the `OcrBackend` trait), and
/// `OcrConfig` has no field that traces back to `ExtractionConfig::images` — so
/// `target_dpi`, `max_image_dimension`, `auto_adjust_dpi`, `min_dpi`, and
/// `max_dpi` were parsed into config but silently dropped before reaching any
/// backend (issue #209). Normalizing the bytes once, here, at the extractor
/// boundary makes the setting effective for every backend without touching the
/// backend trait or its config types.
///
/// Falls back to the original bytes unchanged if decoding or normalization
/// fails; OCR should still be attempted on the original image rather than
/// aborting the extraction.
#[cfg(feature = "ocr-pipeline")]
struct NormalizedOcrImage {
    bytes: Vec<u8>,
    metadata: Option<crate::types::ImagePreprocessingMetadata>,
}

#[cfg(feature = "ocr-pipeline")]
fn unchanged_ocr_image(content: &[u8]) -> NormalizedOcrImage {
    NormalizedOcrImage {
        bytes: content.to_vec(),
        metadata: None,
    }
}

#[cfg(feature = "ocr-pipeline")]
fn normalize_image_bytes_for_ocr(
    content: &[u8],
    images_config: &crate::core::config::ImageExtractionConfig,
    security_limits: &crate::extractors::security::SecurityLimits,
    ocr_config: &crate::core::config::OcrConfig,
) -> Result<NormalizedOcrImage> {
    let rgb = match crate::extraction::image::decode_image_to_rgb8_with_security_limits(content, security_limits) {
        Ok(decoded) => decoded,
        Err(error @ crate::XbergError::Validation { .. }) => return Err(error),
        Err(_) => return Ok(unchanged_ocr_image(content)),
    };
    let (width, height) = rgb.dimensions();
    let dpi_config = crate::types::ImageDpiConfig::from(images_config);
    // GH#1630: an explicit `OcrConfig.backend_options["source_dpi"]` hint wins, then embedded
    // PNG `pHYs` density, then `None` (the historical 72 DPI assumption). Shared with
    // `ocr::processor::execution::perform_ocr` (GH#1621: no second density scanner).
    let explicit_source_dpi = crate::extraction::image::explicit_source_dpi_from_ocr_config(ocr_config);
    let known_source_dpi =
        crate::extraction::image::resolve_known_source_dpi(explicit_source_dpi, content, width, height);
    let (planned_width, planned_height) =
        crate::image::preprocessing::normalized_image_dimensions(width, height, &dpi_config, known_source_dpi);
    let encoded_source_bytes = u64::try_from(content.len())
        .map_err(|_| crate::extraction::image_decode::image_dimension_error(width, height, u64::MAX, u64::MAX))?;
    let current_bytes = u64::try_from(rgb.as_raw().len())
        .ok()
        .and_then(|bytes| bytes.checked_add(encoded_source_bytes))
        .ok_or_else(|| crate::extraction::image_decode::image_dimension_error(width, height, u64::MAX, u64::MAX))?;
    let planned_bytes = crate::extraction::image_decode::decoded_byte_count(planned_width, planned_height, 3)?;
    if (planned_width, planned_height) != (width, height) {
        crate::extraction::image_decode::validate_image_live_bytes(
            width,
            height,
            current_bytes,
            planned_bytes,
            security_limits,
        )?;
    }
    let result = match crate::image::preprocessing::normalize_image_dpi_owned(
        rgb.into_raw(),
        width as usize,
        height as usize,
        &dpi_config,
        known_source_dpi,
    ) {
        Ok(result) => result,
        Err((error, _)) => {
            tracing::warn!(%error, "failed to normalize OCR image; using original image");
            return Ok(unchanged_ocr_image(content));
        }
    };
    let (new_width, new_height) = result.dimensions;
    let new_width = u32::try_from(new_width)
        .map_err(|_| crate::extraction::image_decode::image_dimension_error(u32::MAX, u32::MAX, u64::MAX, u64::MAX))?;
    let new_height = u32::try_from(new_height)
        .map_err(|_| crate::extraction::image_decode::image_dimension_error(new_width, u32::MAX, u64::MAX, u64::MAX))?;
    let normalized_bytes = u64::try_from(result.rgb_data.len())
        .ok()
        .and_then(|bytes| bytes.checked_add(encoded_source_bytes))
        .ok_or_else(|| {
            crate::extraction::image_decode::image_dimension_error(new_width, new_height, u64::MAX, u64::MAX)
        })?;
    let encode_live_bytes = crate::extraction::image_decode::decoded_byte_count(
        new_width,
        new_height,
        NORMALIZED_PNG_ENCODE_BYTES_PER_PIXEL,
    )?
    .checked_add(NORMALIZED_PNG_ENCODE_FIXED_BYTES)
    .ok_or_else(|| crate::extraction::image_decode::image_dimension_error(new_width, new_height, u64::MAX, u64::MAX))?;
    crate::extraction::image_decode::validate_image_live_bytes(
        new_width,
        new_height,
        normalized_bytes,
        encode_live_bytes,
        security_limits,
    )?;
    let bytes = match encode_rgb_as_png(&result.rgb_data, new_width, new_height) {
        Ok(bytes) => bytes,
        Err(error) => {
            tracing::warn!(%error, "failed to encode normalized OCR image; using original image");
            return Ok(unchanged_ocr_image(content));
        }
    };
    Ok(NormalizedOcrImage {
        bytes,
        metadata: Some(result.metadata),
    })
}

#[cfg(feature = "ocr-pipeline")]
fn encode_rgb_as_png(rgb_data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    use image::ImageEncoder;

    let mut png = std::io::Cursor::new(Vec::new());
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(rgb_data, width, height, image::ExtendedColorType::Rgb8)
        .map_err(|error| crate::XbergError::Other(format!("Failed to encode normalized image as PNG: {error}")))?;
    Ok(png.into_inner())
}

#[cfg(all(
    feature = "layout-detection",
    feature = "pdf",
    any(feature = "ocr", feature = "ocr-wasm")
))]
fn uses_tatr_image_table_recognition(table_model: crate::core::config::layout::TableModel) -> bool {
    use crate::core::config::layout::TableModel;

    match table_model {
        TableModel::Tatr => true,
        TableModel::Disabled => false,
        // TODO: SLANeXT image integration needs the same model routing and cell-assignment
        // contract as the PDF structure pipeline. Preserve the existing OCR fallback
        // until that can be shared without duplicating model orchestration. ~keep
        TableModel::SlanetWired | TableModel::SlanetWireless | TableModel::SlanetPlus | TableModel::SlanetAuto => false,
    }
}

#[cfg(all(
    feature = "layout-detection",
    feature = "pdf",
    any(feature = "ocr", feature = "ocr-wasm")
))]
async fn recognize_cached_image_tables(
    whole_image_doc: &InternalDocument,
    rgb: &std::sync::Arc<image::RgbImage>,
    detections: &[crate::layout::LayoutDetection],
    config: &ExtractionConfig,
) -> Vec<crate::RecognizedTable> {
    let Some(layout_config) = config.layout.as_ref() else {
        return Vec::new();
    };
    if !uses_tatr_image_table_recognition(layout_config.table_model) {
        return Vec::new();
    }
    let Some(source_elements) = whole_image_doc.prebuilt_ocr_elements.as_ref() else {
        return Vec::new();
    };
    let Some(transform) = whole_image_ocr_coordinate_transform(whole_image_doc, rgb.width(), rgb.height()) else {
        return Vec::new();
    };
    let Some(elements) = transformed_ocr_elements(source_elements, transform, crate::types::OcrElementLevel::Word)
    else {
        return Vec::new();
    };

    let page_image = std::sync::Arc::clone(rgb);
    let detection = crate::layout::DetectionResult {
        page_width: rgb.width(),
        page_height: rgb.height(),
        detections: detections.to_vec(),
    };
    let acceleration = config.resolved_layout_acceleration().cloned();
    let thread_budget = crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref());
    tokio::task::spawn_blocking(move || {
        let Some(mut model) = crate::layout::take_or_create_tatr(acceleration.as_ref(), thread_budget) else {
            return Vec::new();
        };
        crate::ocr::layout_assembly::recognize_page_tables(&page_image, &detection, &elements, &mut model)
    })
    .await
    .unwrap_or_else(|error| {
        tracing::warn!(%error, "Image table recognition worker failed; retaining OCR fallback");
        Vec::new()
    })
}

/// Returns `true` when the OCR backend configured in `config` self-declares that it
/// emits structured markdown directly. End-to-end VLM backends (PaddleOCR-VL,
/// future GOT-OCR / GLM-OCR) emit markdown in one forward pass and should
/// bypass the layout-detection + region-cropping + table-reconstruction stages.
///
/// Returns `false` when no OCR config is set, no backend is registered under the
/// configured name, or the backend uses the default classical contract.
#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn ocr_backend_emits_structured_markdown(config: &ExtractionConfig) -> bool {
    let Some(ocr) = config.ocr.as_ref() else {
        return false;
    };
    crate::plugins::ensure_ocr_backends_initialized();
    let registry = crate::plugins::registry::get_ocr_backend_registry();
    let registry = registry.read();
    registry
        .get(&ocr.backend)
        .map(|b| b.emits_structured_markdown())
        .unwrap_or(false)
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
fn should_use_layout_ocr(config: &ExtractionConfig) -> bool {
    config.layout.is_some() && config.ocr.is_some() && !ocr_backend_emits_structured_markdown(config)
}

#[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
fn enable_image_ocr_elements(config: &mut crate::core::config::OcrConfig, include_words: bool) {
    let element_config = config
        .element_config
        .get_or_insert_with(crate::types::OcrElementConfig::default);
    element_config.include_elements = true;
    if include_words {
        element_config.min_level = crate::types::OcrElementLevel::Word;
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
fn apply_public_image_ocr_element_policy(document: &mut InternalDocument, config: &crate::core::config::OcrConfig) {
    document.prebuilt_ocr_elements = config.select_public_elements(document.prebuilt_ocr_elements.take());
}

/// GH#1789's numeric-token repair, applied to every string representation of a standalone
/// image's OCR run that can end up in the final document.
///
/// A single flat-string repair on the OCR backend's plain `content` is not enough on this
/// route: when hOCR paragraph structure is available (the common case), the final page text is
/// rebuilt from `internal_doc.elements`, not from that flat string, so the repair has to run on
/// every element's `text` too. Table cells are a second, independent string representation of
/// the same OCR run, including the cell text baked into each table's `markdown` field, and need
/// the same repair applied separately -- fixing `cells` alone would leave a stale, unrepaired
/// `markdown` string. OCR-produced elements never populate `InternalElement::annotations`
/// (grep-verified against this crate), so rewriting `.text` in place cannot desynchronize a
/// byte-range annotation the way it could for hand-authored or PDF-native text.
///
/// `content_is_prose` is false when the backend's `output_format` makes `content` markup
/// (`"hocr"`, `"tsv"`) rather than text -- the repair then skips it, because the separator rule
/// re-punctuates the bare four-digit coordinate integers in a `bbox` or TSV column and the markup
/// stops parsing (GH#1836). The element and table repairs are unaffected either way: those are
/// recognized text in both cases, and `internal_document` is `None` on the markup routes anyway,
/// since only the `"markdown"` arm parses hOCR into one. ~keep
#[cfg(all(feature = "ocr", feature = "pdf"))]
fn apply_numeric_repair_to_standalone_image_ocr(
    content: &mut String,
    content_is_prose: bool,
    internal_document: Option<&mut InternalDocument>,
    tables: &mut [crate::types::Table],
) {
    use crate::extractors::pdf::ocr::repair_ocr_numeric_tokens;

    if content_is_prose && let std::borrow::Cow::Owned(repaired) = repair_ocr_numeric_tokens(content) {
        *content = repaired;
    }
    if let Some(internal_doc) = internal_document {
        for element in &mut internal_doc.elements {
            if let std::borrow::Cow::Owned(repaired) = repair_ocr_numeric_tokens(&element.text) {
                element.text = repaired;
            }
        }
    }
    for table in tables {
        for row in &mut table.cells {
            for cell in row {
                if let std::borrow::Cow::Owned(repaired) = repair_ocr_numeric_tokens(cell) {
                    *cell = repaired;
                }
            }
        }
        if let std::borrow::Cow::Owned(repaired) = repair_ocr_numeric_tokens(&table.markdown) {
            table.markdown = repaired;
        }
    }
}

#[cfg_attr(alef, alef(skip))]
/// Image extractor for various image formats.
///
/// Supports: PNG, JPEG, WebP, BMP, TIFF, GIF.
/// Extracts dimensions, format, and EXIF metadata.
/// Optionally runs OCR when configured.
/// When layout detection is also enabled, uses per-region OCR with
/// markdown formatting based on detected layout classes.
pub struct ImageExtractor;

impl ImageExtractor {
    /// Create a new image extractor.
    pub(crate) fn new() -> Self {
        Self
    }

    fn mark_ocr_extraction(doc: &mut InternalDocument) {
        doc.metadata.ocr_used = true;
        doc.metadata.additional.insert(
            std::borrow::Cow::Borrowed("extraction_method"),
            serde_json::Value::String(crate::types::ExtractionMethod::Ocr.as_str().to_string()),
        );
    }

    /// Extract text from image using OCR with optional page tracking for multi-frame TIFFs.
    #[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
    async fn extract_with_ocr(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        use crate::plugins::registry::get_ocr_backend_registry;

        let default_ocr_config;
        let ocr_config = match config.ocr.as_ref() {
            Some(c) => c,
            None => {
                default_ocr_config = crate::core::config::OcrConfig::default();
                &default_ocr_config
            }
        };

        // `vlm_fallback` and explicit multi-stage pipelines were only honoured by the
        // PDF extractor, so a bare image upload always used a single backend and never
        // fell back to the VLM (issue #1339). When such a policy is configured, route
        // the image through the same shared pipeline runner. Gated on `pdf` because the
        // runner lives in that module; a build without `pdf` keeps the single-backend
        // path unchanged. The default (no fallback, no explicit pipeline) also keeps the
        // fast path, so ordinary image OCR is unaffected.
        #[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
        {
            let wants_pipeline = ocr_config.vlm_fallback != crate::core::config::VlmFallbackPolicy::Disabled
                || ocr_config.pipeline.is_some();
            if wants_pipeline && let Some(pipeline) = ocr_config.effective_pipeline() {
                return self.extract_with_ocr_pipeline(content, config, &pipeline).await;
            }
        }

        let backend = {
            crate::plugins::ensure_ocr_backends_initialized();
            let registry = get_ocr_backend_registry();
            let registry = registry.read();
            registry.get(&ocr_config.backend)?
        };

        let mut ocr_config_with_format = ocr_config.clone();
        apply_default_whole_image_tesseract_psm(&mut ocr_config_with_format);
        ocr_config_with_format.output_format = Some(config.output_format.clone());
        ocr_config_with_format.acceleration = config.acceleration.clone();
        // GH#1651: the sibling injection the embedded-image route already does
        // (`extraction::image_ocr`). Without it a standalone image's OCR decode never sees
        // the caller's `ExtractionConfig::security_limits`. Assigned conditionally so an
        // `OcrConfig::security_limits` the caller set directly is not silently replaced by
        // `None`; `ExtractionConfig` still wins whenever it carries a value. ~keep
        if let Some(security_limits) = config.security_limits.clone() {
            ocr_config_with_format.security_limits = Some(security_limits);
        }
        // #1787: the top-level `ExtractionConfig.use_cache = false` only ever bypassed the
        // whole-extraction cache (`extract_file_with_extractor`); the OCR backend's own result
        // cache is governed independently by `TesseractConfig.use_cache` (default `true`), which
        // `OcrBackend::process_image` never sees `ExtractionConfig` to read. Propagate through
        // `backend_options["use_cache"]`, the channel `TesseractBackend::config_to_tesseract`
        // already reads for exactly this purpose. Only the "off" direction is forwarded: a
        // caller who leaves the top-level flag at its default keeps whatever OCR-specific cache
        // setting they configured directly, unmodified. ~keep
        if !config.use_cache {
            let mut options = ocr_config_with_format
                .backend_options
                .take()
                .and_then(|value| match value {
                    serde_json::Value::Object(map) => Some(map),
                    _ => None,
                })
                .unwrap_or_default();
            options.insert("use_cache".to_string(), serde_json::Value::Bool(false));
            ocr_config_with_format.backend_options = Some(serde_json::Value::Object(options));
        }
        #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
        let include_words = should_use_layout_ocr(config);
        #[cfg(not(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm"))))]
        let include_words = false;
        enable_image_ocr_elements(&mut ocr_config_with_format, include_words);
        #[cfg(not(target_arch = "wasm32"))]
        if is_implicit_horizontal_tesseract(ocr_config) {
            enable_image_ocr_elements(&mut ocr_config_with_format, true);
        }

        // OCR backends only see `OcrConfig`, which has no route back to
        // `ExtractionConfig::images`, so DPI/dimension normalization from
        // `ImageExtractionConfig` has to happen here, once, before any backend
        // ever sees the bytes (issue #209). ~keep
        #[cfg(feature = "ocr-pipeline")]
        let default_security_limits = crate::extractors::security::SecurityLimits::default();
        #[cfg(feature = "ocr-pipeline")]
        let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
        #[cfg(feature = "ocr-pipeline")]
        let normalized_ocr_bytes = config
            .images
            .as_ref()
            .map(|images_config| normalize_image_bytes_for_ocr(content, images_config, security_limits, ocr_config))
            .transpose()?;
        #[cfg(feature = "ocr-pipeline")]
        let ocr_input: &[u8] = normalized_ocr_bytes
            .as_ref()
            .map(|normalized| normalized.bytes.as_slice())
            .unwrap_or(content);
        #[cfg(not(feature = "ocr-pipeline"))]
        let ocr_input: &[u8] = content;

        #[cfg(not(target_arch = "wasm32"))]
        let ocr_result = process_whole_image_ocr_with_sparse_retry(
            &backend,
            std::sync::Arc::new(ocr_input.to_vec()),
            ocr_config,
            &ocr_config_with_format,
            true,
            true,
            config.cancel_token.as_ref(),
        )
        .await?;
        #[cfg(target_arch = "wasm32")]
        let ocr_result = backend.process_image(ocr_input, &ocr_config_with_format).await?;
        #[cfg(feature = "ocr-pipeline")]
        let ocr_result = {
            let mut ocr_result = ocr_result;
            if let Some(metadata) = normalized_ocr_bytes.and_then(|normalized| normalized.metadata) {
                ocr_result.metadata.image_preprocessing = Some(metadata);
            }
            ocr_result
        };

        // Only the GH#1789 numeric repair below mutates this, and that is gated on
        // `ocr` + `pdf`; an unconditional `mut` is a dead `mut` on every narrower leg,
        // which the formula-recognition+pdf clippy gate rejects. Same shape as
        // `ocr_tables`/`ocr_internal_document` just below. ~keep
        #[cfg(all(feature = "ocr", feature = "pdf"))]
        let mut ocr_content = ocr_result.content;
        #[cfg(not(all(feature = "ocr", feature = "pdf")))]
        let ocr_content = ocr_result.content;
        let ocr_metadata = ocr_result.metadata;
        let ocr_elements = ocr_result.ocr_elements;
        let ocr_formulas = ocr_result.formulas;
        let processing_warnings = ocr_result.processing_warnings;
        // `ExtractedDocument::tables` (tables reconstructed by OCR table
        // detection) used to be dropped here entirely: nothing on the
        // standalone-image path ever read it, so a table found in a bare
        // image never reached the output.
        // Only the GH#1789 repair below mutates these two, and it is gated on `ocr` + `pdf`,
        // so binding them `mut` under `ocr` alone is a dead `mut` on the ocr-without-pdf leg. ~keep
        #[cfg(all(feature = "ocr", feature = "pdf"))]
        let mut ocr_tables = ocr_result.tables;
        #[cfg(all(feature = "ocr", feature = "pdf"))]
        let mut ocr_internal_document = ocr_result.ocr_internal_document;
        #[cfg(all(feature = "ocr", not(feature = "pdf")))]
        let ocr_tables = ocr_result.tables;
        #[cfg(all(feature = "ocr", not(feature = "pdf")))]
        let ocr_internal_document = ocr_result.ocr_internal_document;

        // GH#1789: opt-in, same gate and rationale as the PDF mixed-OCR route
        // (`extractors::pdf::ocr::pipeline::numeric_repair_enabled`). See
        // `apply_numeric_repair_to_standalone_image_ocr`'s own doc comment for why this has to
        // touch three separate string representations of the same OCR run, not just
        // `ocr_content`.
        #[cfg(all(feature = "ocr", feature = "pdf"))]
        if ocr_config.numeric_repair {
            apply_numeric_repair_to_standalone_image_ocr(
                &mut ocr_content,
                crate::extractors::pdf::ocr::ocr_content_is_repairable_prose(ocr_config),
                ocr_internal_document.as_mut(),
                &mut ocr_tables,
            );
        }

        // ~keep The whole image is one OCR run, so its confidence describes page 1 and only
        // page 1. The multi-frame TIFF branch below splits that single run's text by byte
        // offset into several pages with no per-frame OCR of their own, so those pages keep
        // `ocr_confidence: None` rather than repeating a figure that was never measured for
        // them (#1568). Gated on `pdf` because the shared builder lives in that module.
        #[cfg(all(feature = "ocr", feature = "pdf"))]
        let whole_image_ocr_confidence = crate::extractors::pdf::ocr::page_ocr_confidence(
            backend.confidence_semantics(),
            crate::extractors::pdf::ocr::mean_text_conf_of(&ocr_metadata.additional),
            crate::extractors::pdf::ocr::word_count_of(&ocr_metadata.additional).unwrap_or(0),
            backend.name(),
        );
        #[cfg(all(feature = "ocr", not(feature = "pdf")))]
        let whole_image_ocr_confidence: Option<crate::types::page::PageOcrConfidence> = None;

        #[cfg(feature = "ocr")]
        {
            let ocr_extraction_result = crate::extraction::image::extract_text_from_image_with_ocr(
                content,
                mime_type,
                ocr_content,
                config.pages.as_ref(),
            )?;

            // The standalone-image path has no document-global structure pipeline
            // behind it (unlike the PDF mixed route), so it is the one place that
            // still needs to detect headings itself. Prefer the hOCR-derived
            // element tree — which still carries per-paragraph `x_fsize` — so
            // headings become real `Heading` elements the renderer understands,
            // rather than a pre-escaped `## ` string baked into plain text (which
            // produced `\#\#` under markdown escaping and leaked `## ` into
            // `Plain` output). Only usable when OCR ran on the whole image in one
            // pass: multi-frame TIFF page tracking slices `content` by byte
            // offset and has no per-frame correspondence to hOCR elements, so it
            // keeps the flat paragraph-split fallback.
            //
            // `internal_doc.elements` can be legitimately empty even when OCR found
            // text: `perform_ocr` (execution.rs) drops any hOCR paragraph entirely
            // claimed by a detected table before this code ever sees it (#1571), and
            // a page that is nothing but a table empties the list that way. Falling
            // back to `ocr_extraction_result.content` in that case would resurrect
            // the same table text as prose alongside the `OcrTable` pushed below --
            // exactly the duplication being fixed. Once any table was detected, trust
            // the (possibly empty) filtered element list instead of that fallback. ~keep
            let use_hocr_headings = ocr_extraction_result.page_contents.is_none();
            let hocr_has_content_or_tables =
                |internal_doc: &InternalDocument| !internal_doc.elements.is_empty() || !ocr_tables.is_empty();
            let mut doc = match &ocr_internal_document {
                Some(internal_doc) if use_hocr_headings && hocr_has_content_or_tables(internal_doc) => {
                    build_image_internal_document_from_hocr_elements(&internal_doc.elements)
                }
                _ => build_image_internal_document(Some(&ocr_extraction_result.content), None),
            };
            doc.metadata = ocr_metadata;
            doc.formulas = ocr_formulas;
            doc.processing_warnings = processing_warnings;
            Self::mark_ocr_extraction(&mut doc);

            doc.prebuilt_ocr_elements = ocr_elements;

            let page_tables: Vec<std::sync::Arc<crate::types::Table>> = ocr_tables
                .into_iter()
                .map(|table| {
                    let bbox = table.bounding_box;
                    let table_index = doc.push_table(table.clone());
                    let mut element = crate::types::internal::InternalElement::text(
                        crate::types::internal::ElementKind::Table { table_index },
                        "",
                        0,
                    );
                    element.bbox = bbox;
                    doc.push_element(element);
                    std::sync::Arc::new(table)
                })
                .collect();

            if let Some(pages) = ocr_extraction_result.page_contents {
                doc.prebuilt_pages = Some(pages);
            } else {
                let text = ocr_extraction_result.content.trim().to_string();
                if !text.is_empty() {
                    doc.prebuilt_pages = Some(vec![crate::types::PageContent {
                        page_number: 1,
                        content: text,
                        tables: page_tables,
                        image_indices: vec![],
                        image_preprocessing: None,
                        hierarchy: None,
                        is_blank: None,
                        layout_regions: None,
                        speaker_notes: None,
                        section_name: None,
                        sheet_name: None,
                        ocr_confidence: whole_image_ocr_confidence,
                        native_content: None,
                    }]);
                }
            }

            Ok(doc)
        }

        #[cfg(not(feature = "ocr"))]
        {
            let _ = mime_type;
            let mut doc = build_image_internal_document(Some(&ocr_content), None);
            doc.metadata = ocr_metadata;
            doc.formulas = ocr_formulas;
            doc.processing_warnings = processing_warnings;
            Self::mark_ocr_extraction(&mut doc);
            doc.prebuilt_ocr_elements = ocr_elements;
            let text = ocr_content.trim().to_string();
            if !text.is_empty() {
                doc.prebuilt_pages = Some(vec![crate::types::PageContent {
                    page_number: 1,
                    content: text,
                    tables: vec![],
                    image_indices: vec![],
                    image_preprocessing: None,
                    hierarchy: None,
                    is_blank: None,
                    layout_regions: None,
                    speaker_notes: None,
                    section_name: None,
                    sheet_name: None,
                    ocr_confidence: None,
                    native_content: None,
                }]);
            }
            Ok(doc)
        }
    }

    /// Route a single image through the multi-stage OCR pipeline (issue #1339).
    ///
    /// Decodes the image and reuses the PDF extractor's pipeline runner so that
    /// `vlm_fallback` / explicit `pipeline` policies apply to bare images the same
    /// way they do to PDF pages. Gated on `pdf` because the runner lives there.
    #[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
    async fn extract_with_ocr_pipeline(
        &self,
        content: &[u8],
        config: &ExtractionConfig,
        pipeline: &crate::core::config::OcrPipelineConfig,
    ) -> Result<InternalDocument> {
        let default_security_limits = crate::extractors::security::SecurityLimits::default();
        let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
        let image = crate::extraction::image::decode_image_to_rgb8_with_security_limits(content, security_limits)?;
        let source_dpi = crate::extraction::image::resolve_known_source_dpi(
            config
                .ocr
                .as_ref()
                .and_then(crate::extraction::image::explicit_source_dpi_from_ocr_config),
            content,
            image.width(),
            image.height(),
        );
        let rgb_bytes = u64::try_from(image.as_raw().len()).map_err(|_| {
            crate::extraction::image_decode::image_dimension_error(image.width(), image.height(), u64::MAX, u64::MAX)
        })?;
        crate::extraction::image_decode::validate_image_live_bytes(
            image.width(),
            image.height(),
            rgb_bytes,
            rgb_bytes,
            security_limits,
        )?;
        let image = image::DynamicImage::ImageRgb8(image);
        let images = [image];

        let (
            text,
            _tables,
            ocr_elements,
            pipeline_doc,
            llm_usage,
            _page_texts,
            _rasters,
            formulas,
            _,
            mut pipeline_ocr_confidence,
        ) = Box::pin(crate::extractors::pdf::ocr::run_ocr_pipeline(
            None,
            Some(&images),
            #[cfg(feature = "layout-detection")]
            None,
            config,
            pipeline,
            None,
            Some(crate::extractors::pdf::ocr::PageOcrHints {
                source_dpi,
                known_full_page_scan: true,
                single_block_pages: None,
            }),
        ))
        .await?;

        // Build a clean image document from the pipeline text (keeping the "image"
        // doc type and shape the rest of the image path produces), then carry over
        // the pipeline's elements, formulas, usage, and — crucially — any processing
        // warnings, so a fallback backend that failed is visible to the caller
        // rather than silently swapped for the classical result (issue #1339).
        let mut doc = build_image_internal_document(Some(&text), None);
        Self::mark_ocr_extraction(&mut doc);
        if !ocr_elements.is_empty() {
            doc.prebuilt_ocr_elements = Some(ocr_elements);
        }
        if !formulas.is_empty() {
            doc.formulas = formulas;
        }
        if !llm_usage.is_empty() {
            doc.llm_usage = Some(llm_usage);
        }
        if let Some(pipeline_doc) = pipeline_doc {
            doc.processing_warnings.extend(pipeline_doc.processing_warnings);
            doc.ocr_page_failures.extend(pipeline_doc.ocr_page_failures);
        }

        let trimmed = text.trim().to_string();
        if !trimmed.is_empty() {
            doc.prebuilt_pages = Some(vec![crate::types::PageContent {
                page_number: 1,
                content: trimmed,
                tables: vec![],
                image_indices: vec![],
                image_preprocessing: None,
                hierarchy: None,
                is_blank: None,
                layout_regions: None,
                speaker_notes: None,
                section_name: None,
                sheet_name: None,
                // ~keep The lone image was handed to the runner as page 0, so the winning
                // stage keys its summary at page 1 -- the same page this builds (#1568).
                ocr_confidence: pipeline_ocr_confidence.remove(&1),
                native_content: None,
            }]);
        }

        Ok(doc)
    }

    /// Extract text from image using layout detection + per-region OCR.
    ///
    /// Runs layout detection to identify document regions (headings, text,
    /// code, formulas, etc.), then OCRs each region individually and
    /// assembles the results into structured markdown.
    #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
    async fn extract_with_layout_ocr(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let layout_config = config.layout.as_ref().ok_or_else(|| crate::XbergError::Parsing {
            message: "Layout config required for layout-enhanced OCR".to_string(),
            source: None,
        })?;

        let ocr_config = config.ocr.as_ref().ok_or_else(|| crate::XbergError::Parsing {
            message: "OCR config required for layout-enhanced OCR".to_string(),
            source: None,
        })?;

        let preparation = prepare_layout_ocr(self, content, mime_type, config, layout_config.clone()).await?;
        let (whole_image_result, rgb, detections) = match preparation {
            LayoutOcrPreparation::Complete(doc) => return Ok(doc),
            LayoutOcrPreparation::Detected {
                whole_image_result,
                rgb,
                detections,
            } => (whole_image_result, rgb, detections),
        };
        #[cfg(feature = "pdf")]
        let recognized_tables = match &whole_image_result {
            Ok(whole_image_doc) => recognize_cached_image_tables(whole_image_doc, &rgb, &detections, config).await,
            Err(_) => Vec::new(),
        };
        #[cfg(not(feature = "pdf"))]
        let recognized_tables = Vec::new();

        if let Ok(whole_image_doc) = &whole_image_result
            && source_image_is_proven_single_frame(content, mime_type)
            && let Some(structured) = try_assemble_cached_layout_document(
                whole_image_doc,
                &detections,
                &recognized_tables,
                rgb.width(),
                rgb.height(),
            )
        {
            #[cfg(feature = "formula-recognition")]
            let mut structured = structured;
            #[cfg(feature = "formula-recognition")]
            if let Some(layout) = config.layout.as_ref() {
                recognize_assembled_formula_regions(&mut structured, &rgb, &detections, layout).await;
            }
            tracing::debug!(
                tables = structured.tables.len(),
                "Assembled cached image OCR with layout structure"
            );
            return Ok(structured);
        }
        if let Ok(whole_image_doc) = &whole_image_result
            && let Some(structured) = try_retain_canonical_whole_image_ocr(
                whole_image_doc,
                &detections,
                rgb.width(),
                rgb.height(),
                source_image_is_proven_single_frame(content, mime_type),
            )
        {
            tracing::debug!(
                elements = whole_image_doc.prebuilt_ocr_elements.as_ref().map_or(0, Vec::len),
                "Retained canonical whole-image OCR without per-region OCR"
            );
            return Ok(structured);
        }

        let (backend, region_ocr_config) = match configured_region_ocr(config, ocr_config) {
            Ok(configured) => configured,
            Err(error) => return cached_whole_image_after_layout_error(&whole_image_result, error),
        };
        let region_doc = match extract_layout_regions(
            backend,
            &rgb,
            &detections,
            &region_ocr_config,
            config.layout.as_ref(),
            config.cancel_token.as_ref(),
        )
        .await
        {
            Ok(doc) => doc,
            Err(error) => return cached_whole_image_after_layout_error(&whole_image_result, error),
        };
        Ok(select_image_ocr_result(region_doc, whole_image_result))
    }
}

/// Minimum ratio of a paragraph's average hOCR font size (`x_fsize`) to the
/// image's median paragraph font size before the paragraph is promoted to a
/// heading (#185).
///
/// The standalone-image path is the last OCR route that still needs this
/// heuristic locally: it has no document-global structure pipeline behind it
/// (unlike the PDF mixed route, which classifies headings itself from the
/// same `x_fsize` attribute across the whole document). See
/// `ocr::processor::execution::flatten_hocr_elements_to_text` for why that
/// function no longer bakes headings into its output string.
#[cfg(feature = "ocr")]
const STANDALONE_IMAGE_HEADING_FONT_SIZE_RATIO: f64 = 1.3;

/// Maximum word count for a large-font paragraph to still be treated as a
/// heading; see [`STANDALONE_IMAGE_HEADING_FONT_SIZE_RATIO`].
#[cfg(feature = "ocr")]
const STANDALONE_IMAGE_HEADING_MAX_WORD_COUNT: usize = 12;

/// Read the paragraph-average hOCR font size stored by `hocr_parser`.
#[cfg(feature = "ocr")]
fn standalone_image_element_font_size(element: &crate::types::internal::InternalElement) -> Option<f64> {
    element
        .attributes
        .as_ref()?
        .get(crate::ocr::hocr_parser::HOCR_FONT_SIZE_ATTRIBUTE)?
        .parse::<f64>()
        .ok()
}

/// Build an `InternalDocument` from hOCR-derived paragraph elements.
///
/// Promotes large-font, short, single-line paragraphs to real `Heading`
/// elements rather than baking `## ` into a plain-text string: a pre-escaped
/// string is indistinguishable from literal user text once it reaches a
/// renderer, so it either leaks markdown syntax into `Plain` output or gets
/// escaped to `\#\#` by the markdown renderer. Always pushes the image
/// itself as an `Image` node, matching `build_image_internal_document`.
#[cfg(feature = "ocr")]
fn build_image_internal_document_from_hocr_elements(
    elements: &[crate::types::internal::InternalElement],
) -> InternalDocument {
    use crate::types::internal::ElementKind;

    let mut font_sizes: Vec<f64> = elements.iter().filter_map(standalone_image_element_font_size).collect();
    let median_font_size = if font_sizes.is_empty() {
        None
    } else {
        font_sizes.sort_by(f64::total_cmp);
        Some(font_sizes[font_sizes.len() / 2])
    };

    let mut builder = InternalDocumentBuilder::new("image");
    for element in elements {
        if matches!(element.kind, ElementKind::PageBreak) || element.text.is_empty() {
            continue;
        }
        let is_heading = median_font_size.is_some_and(|median| {
            standalone_image_element_font_size(element).is_some_and(|size| {
                size >= median * STANDALONE_IMAGE_HEADING_FONT_SIZE_RATIO
                    && !element.text.contains('\n')
                    && element.text.split_whitespace().count() <= STANDALONE_IMAGE_HEADING_MAX_WORD_COUNT
            })
        });
        if is_heading {
            builder.push_heading(1, &element.text, None, None);
        } else {
            builder.push_paragraph(&element.text, vec![], None, None);
        }
    }
    push_image_placeholder(&mut builder);
    builder.build()
}

/// Push a placeholder `Image` element carrying no binary data.
fn push_image_placeholder(builder: &mut InternalDocumentBuilder) {
    use crate::types::document_structure::ContentLayer;
    use crate::types::internal::{ElementKind, InternalElement, InternalElementId};

    let kind = ElementKind::Image { image_index: 0 };
    let id = InternalElementId::generate(kind.discriminant(), "", None, 0);
    builder.push_element(InternalElement {
        id,
        kind,
        text: String::new(),
        depth: 0,
        page: None,
        bbox: None,
        layer: ContentLayer::Body,
        annotations: Vec::new(),
        attributes: None,
        anchor: None,
        ocr_geometry: None,
        ocr_confidence: None,
        ocr_rotation: None,
    });
}

/// Build a simple `InternalDocument` for an image extraction result.
///
/// If OCR text is available, preserves its blank-line-delimited paragraphs. Always pushes
/// the image itself as an `Image` node. When `image_data` is provided,
/// the binary data is stored in `InternalDocument::images` and the
/// element references it by index.
fn build_image_internal_document(
    ocr_text: Option<&str>,
    image_data: Option<crate::types::ExtractedImage>,
) -> InternalDocument {
    let mut builder = InternalDocumentBuilder::new("image");
    if let Some(text) = ocr_text
        && !text.trim().is_empty()
    {
        for paragraph in split_ocr_paragraphs(text) {
            builder.push_paragraph(&paragraph, vec![], None, None);
        }
    }
    if let Some(img) = image_data {
        builder.push_image(None, img, None, None);
    } else {
        push_image_placeholder(&mut builder);
    }
    builder.build()
}

fn split_ocr_paragraphs(text: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut current = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            if !current.is_empty() {
                paragraphs.push(current.join("\n"));
                current.clear();
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        paragraphs.push(current.join("\n"));
    }
    paragraphs
}

impl Default for ImageExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ImageExtractor {
    fn name(&self) -> &str {
        "image-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Extracts dimensions, format, and EXIF data from images (PNG, JPEG, WebP, BMP, TIFF, GIF)"
    }

    fn author(&self) -> &str {
        "Xberg Team"
    }
}

/// Reject a multi-frame TIFF whose frame count exceeds `security_limits.max_pages`
/// before per-frame OCR begins (#1451).
///
/// TIFF is the only image container this crate treats as multi-page: frames are
/// OCR'd and their text tracked per page (see
/// `crate::extraction::image::extract_text_from_image_with_ocr`), while an
/// animated PNG/WebP/JPEG is always OCR'd as one whole image regardless of frame
/// count. Counting is only possible under the `ocr` feature, the sole feature
/// that pulls in the `tiff` crate `detect_tiff_frame_count` needs to walk IFDs
/// without decoding any raster data; other OCR feature combinations
/// (`ocr-wasm`, `ocr-pipeline` alone) have no way to count frames cheaply, so
/// this is a no-op there and `max_pages` goes unenforced on TIFF in those
/// builds.
#[cfg(feature = "ocr")]
fn enforce_image_page_limit(content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<()> {
    let max_pages = config.security_limits.as_ref().and_then(|limits| limits.max_pages);
    let Some(max_pages) = max_pages else {
        return Ok(());
    };
    if !mime_type.to_lowercase().contains("tiff") {
        return Ok(());
    }
    let Ok(frame_count) = crate::extraction::image::detect_tiff_frame_count(content) else {
        return Ok(());
    };
    Ok(crate::extractors::security::enforce_page_count(
        frame_count,
        Some(max_pages),
    )?)
}

/// Guards the one-time "max_pages is unenforced for TIFF" warning so it fires
/// at most once per process, not once per extraction — same idiom as
/// `DEFAULT_CAP_WARNED` in `core::config::concurrency`.
#[cfg(not(feature = "ocr"))]
static UNENFORCED_TIFF_PAGE_LIMIT_WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Emit the "`max_pages` is unenforced for TIFF in this build" warning at most
/// once per `already_warned` guard.
///
/// The guard is a parameter rather than a direct read of
/// [`UNENFORCED_TIFF_PAGE_LIMIT_WARNED`] so the once-per-process property can be
/// tested against a guard the test owns, following
/// `warn_default_thread_cap_once` in `core::config::concurrency`: asserting on
/// the real global from more than one test would be unfixably racy.
#[cfg(not(feature = "ocr"))]
fn warn_unenforced_tiff_page_limit_once(already_warned: &std::sync::atomic::AtomicBool, max_pages: usize) {
    if already_warned
        .compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        )
        .is_ok()
    {
        tracing::warn!(
            max_pages,
            "security_limits.max_pages is set but cannot be enforced on TIFF images in this \
             build: counting TIFF frames requires the `ocr` feature, which this build does not \
             have enabled. TIFF page limits are silently unenforced until the binary is rebuilt \
             with `--features ocr`."
        );
    }
}

/// No-op stub for builds without the `ocr` feature: there is no `tiff` crate
/// dependency available to count frames, so `max_pages` is not enforced on TIFF
/// images at all in these builds (see `SecurityLimits::max_pages`'s doc
/// comment). A one-shot warning surfaces the gap instead of silently ignoring
/// a limit the caller explicitly set.
#[cfg(not(feature = "ocr"))]
fn enforce_image_page_limit(_content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<()> {
    enforce_image_page_limit_stub_with_guard(mime_type, config, &UNENFORCED_TIFF_PAGE_LIMIT_WARNED)
}

/// Pure core of the `not(feature = "ocr")` [`enforce_image_page_limit`], parameterized on the
/// one-shot warning guard so tests can exercise it without touching process-global state.
#[cfg(not(feature = "ocr"))]
fn enforce_image_page_limit_stub_with_guard(
    mime_type: &str,
    config: &ExtractionConfig,
    already_warned: &std::sync::atomic::AtomicBool,
) -> Result<()> {
    let max_pages = config.security_limits.as_ref().and_then(|limits| limits.max_pages);
    let Some(max_pages) = max_pages else {
        return Ok(());
    };
    if !mime_type.to_lowercase().contains("tiff") {
        return Ok(());
    }
    warn_unenforced_tiff_page_limit_once(already_warned, max_pages);
    Ok(())
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for ImageExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        tracing::debug!(format = "image", size_bytes = content.len(), "extraction starting");
        enforce_image_page_limit(content, mime_type, config)?;
        let default_security_limits = crate::extractors::security::SecurityLimits::default();
        let security_limits = config.security_limits.as_ref().unwrap_or(&default_security_limits);
        let extraction_metadata = extract_image_metadata_with_security_limits(content, security_limits)?;
        // Computed against the original bytes (before any HEIC->PNG rebinding
        // below) so it reflects the same input `extract_image_metadata` saw. ~keep
        let exif_warning = crate::extraction::exif::extract_exif_warning(content);

        #[cfg(feature = "heic")]
        let owned_png;
        #[cfg(feature = "heic")]
        let (content, mime_type): (&[u8], &str) = if crate::extraction::heif::is_heif_container(content) {
            owned_png = crate::extraction::heif::decode_heic_to_png(content, security_limits)?;
            (owned_png.as_slice(), "image/png")
        } else {
            (content, mime_type)
        };

        let format_str = extraction_metadata.format;
        let image_metadata = crate::types::ImageMetadata {
            width: extraction_metadata.width,
            height: extraction_metadata.height,
            format: format_str.clone(),
            exif: extraction_metadata.exif_data,
        };

        let (image_kind, kind_confidence) =
            crate::extraction::image_kind::classify(crate::extraction::image_kind::ImageClassifyInput {
                bytes: content,
                format: &format_str,
                width: Some(extraction_metadata.width),
                height: Some(extraction_metadata.height),
                colorspace: None,
                bits_per_component: None,
                is_mask: false,
            });

        let extracted_image = crate::types::ExtractedImage {
            data: bytes::Bytes::copy_from_slice(content),
            format: std::borrow::Cow::Owned(format_str),
            image_index: 0,
            page_number: None,
            width: Some(extraction_metadata.width),
            height: Some(extraction_metadata.height),
            colorspace: None,
            bits_per_component: None,
            is_mask: false,
            description: None,
            ocr_result: None,
            bounding_box: None,
            source_path: None,
            image_kind: Some(image_kind),
            kind_confidence: Some(kind_confidence),
            cluster_id: None,
            caption: None,
            qr_codes: None,
            data_base64: None,
        };

        if config.effective_disable_ocr() {
            let attach_image = if config.wants_own_bytes_in_result() {
                Some(extracted_image)
            } else {
                None
            };
            let mut doc = build_image_internal_document(None, attach_image);
            doc.metadata = Metadata {
                format: Some(crate::types::FormatMetadata::Image(image_metadata)),
                ..Default::default()
            };
            doc.mime_type = mime_type.to_string();
            if let Some(warning) = exif_warning.clone() {
                doc.processing_warnings.push(warning);
            }
            tracing::debug!(
                format = "image",
                "OCR disabled via disable_ocr, returning metadata only"
            );
            return Ok(doc);
        }

        {
            #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
            {
                let use_layout = should_use_layout_ocr(config);
                let mut doc = extract_selected_image_ocr_path(
                    use_layout,
                    || self.extract_with_layout_ocr(content, mime_type, config),
                    || self.extract_with_ocr(content, mime_type, config),
                )
                .await?;
                if let Some(ocr_config) = config.ocr.as_ref() {
                    apply_public_image_ocr_element_policy(&mut doc, ocr_config);
                }
                Self::mark_ocr_extraction(&mut doc);
                doc.metadata.format = Some(crate::types::FormatMetadata::Image(image_metadata));
                doc.mime_type = mime_type.to_string();
                if config.wants_own_bytes_in_result() {
                    doc.images.push(extracted_image);
                }
                if let Some(warning) = exif_warning.clone() {
                    doc.processing_warnings.push(warning);
                }
                return Ok(doc);
            }

            #[cfg(all(
                any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"),
                not(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))
            ))]
            {
                let mut doc = self.extract_with_ocr(content, mime_type, config).await?;
                if let Some(ocr_config) = config.ocr.as_ref() {
                    apply_public_image_ocr_element_policy(&mut doc, ocr_config);
                }
                Self::mark_ocr_extraction(&mut doc);
                doc.metadata.format = Some(crate::types::FormatMetadata::Image(image_metadata));
                doc.mime_type = mime_type.to_string();
                if config.wants_own_bytes_in_result() {
                    doc.images.push(extracted_image);
                }
                if let Some(warning) = exif_warning.clone() {
                    doc.processing_warnings.push(warning);
                }
                return Ok(doc);
            }
        }

        #[cfg(not(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")))]
        {
            let mut doc = build_image_internal_document(None, Some(extracted_image));
            doc.metadata = Metadata {
                format: Some(crate::types::FormatMetadata::Image(image_metadata)),
                ..Default::default()
            };
            doc.mime_type = mime_type.to_string();
            if let Some(warning) = exif_warning.clone() {
                doc.processing_warnings.push(warning);
            }

            tracing::debug!(
                element_count = doc.elements.len(),
                format = "image",
                "extraction complete"
            );
            Ok(doc)
        }
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "image/png",
            "image/jpeg",
            "image/jpg",
            "image/pjpeg",
            "image/webp",
            "image/bmp",
            "image/x-bmp",
            "image/x-ms-bmp",
            "image/tiff",
            "image/x-tiff",
            "image/gif",
            "image/jp2",
            "image/j2c",
            "image/x-jbig2",
            "image/x-portable-anymap",
            "image/x-portable-bitmap",
            "image/x-portable-graymap",
            "image/x-portable-pixmap",
            "image/heic",
            "image/heic-sequence",
            "image/heif",
            "image/heif-sequence",
            "image/avif",
            "image/avcs",
        ]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
    #[test]
    fn image_layout_preserves_an_ordered_list_item_source_number() {
        let detection = crate::layout::LayoutDetection::new(
            crate::layout::LayoutClass::ListItem,
            0.9,
            crate::layout::BBox {
                x1: 0.0,
                y1: 0.0,
                x2: 100.0,
                y2: 20.0,
            },
        );
        let mut builder = InternalDocumentBuilder::new("image/png");
        let mut formulas = Vec::new();

        assert!(push_mapped_layout_text(
            &mut builder,
            &mut formulas,
            &detection,
            "1. Template for day 1",
        ));

        let document = builder.build();
        assert!(matches!(
            document.elements[0].kind,
            crate::types::internal::ElementKind::ListItem { ordered: true }
        ));
        assert_eq!(
            crate::rendering::render_markdown(&document),
            "- 1\\. Template for day 1\n"
        );
    }

    #[tokio::test]
    async fn rejects_declared_image_dimensions_over_configured_security_budget() {
        let content = crate::extraction::image_decode::bmp_with_declared_dimensions(100, 100);
        let config = ExtractionConfig {
            disable_ocr: true,
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_content_size: 1024,
                ..Default::default()
            }),
            ..Default::default()
        };

        let error = ImageExtractor::new()
            .extract_content(&content, "image/bmp", &config)
            .await
            .expect_err("the extractor must apply the configured budget before decoding pixels");

        assert!(matches!(error, crate::XbergError::Validation { .. }));
        assert!(error.to_string().contains("100x100"));
        assert!(error.to_string().contains("security_limits.max_content_size"));
    }

    /// #860: `normalize_image_bytes_for_ocr` applies `ImageExtractionConfig`'s
    /// `max_image_dimension` / DPI settings at the extractor boundary, because OCR
    /// backends only ever see `OcrConfig` and have no route back to them (#209).
    ///
    /// It used to be gated on `feature = "ocr"`, while `extract_with_ocr` is gated on
    /// `ocr` OR `ocr-wasm` OR `ocr-pipeline`. Every `candle-*` backend — and any
    /// `liter-llm` VLM-only build — implies `ocr-pipeline` but NOT `ocr`, so those
    /// builds took the `#[cfg(not(feature = "ocr"))]` arm and passed the raw bytes
    /// through with the settings silently dropped.
    ///
    /// Run this under a candle-only feature set to see the fix: before it, the
    /// function did not exist in such a build at all.
    #[cfg(feature = "ocr-pipeline")]
    #[test]
    fn normalizes_a_standalone_image_to_the_configured_maximum_dimension() {
        let oversized = image::RgbImage::from_pixel(1200, 600, image::Rgb([255, 255, 255]));
        let png = encode_rgb_as_png(oversized.as_raw(), 1200, 600).expect("encode the fixture");

        let images_config = crate::core::config::ImageExtractionConfig {
            max_image_dimension: 300,
            auto_adjust_dpi: false,
            ..Default::default()
        };
        let normalized = normalize_image_bytes_for_ocr(
            &png,
            &images_config,
            &crate::extractors::security::SecurityLimits::default(),
            &crate::core::config::OcrConfig::default(),
        )
        .expect("normal image should remain within the default decode budget");

        assert_ne!(
            normalized.bytes, png,
            "the oversized image should not pass through unchanged"
        );
        let decoded = image::load_from_memory(&normalized.bytes).expect("decode the normalized image");
        assert_eq!(
            (decoded.width(), decoded.height()),
            (300, 150),
            "the long edge should be clamped to max_image_dimension, preserving the 2:1 aspect ratio"
        );
        let metadata = normalized
            .metadata
            .expect("successful standalone-image normalization must retain metadata");
        assert_eq!(metadata.original_dimensions.width, 1200);
        assert_eq!(metadata.original_dimensions.height, 600);
        assert_eq!(
            metadata.new_dimensions.as_ref().map(|dimensions| dimensions.width),
            Some(300)
        );
        assert_eq!(
            metadata.new_dimensions.as_ref().map(|dimensions| dimensions.height),
            Some(150)
        );
        assert!(metadata.dimension_clamped);
    }

    // GH#1630: standalone image OCR discarded PNG `pHYs` density and always assumed 72 DPI, so
    // a genuine 300 DPI scan was resampled even when the requested `target_dpi` was already 300.
    // This is the actual site the reporter's repro exercised: `extract_with_ocr` normalizes the
    // bytes here, at the extractor boundary, before any backend (including `perform_ocr`, fixed
    // separately) ever sees them.
    mod png_source_dpi_at_extractor_boundary {
        use super::*;

        /// Standard PNG CRC-32 (polynomial 0xEDB88320), needed to splice a well-formed `pHYs`
        /// chunk into a real encoded PNG. Test-fixture construction only — the parser being
        /// exercised, `extraction::image::png_pixel_density_dpi`, is defined once and shared;
        /// see GH#1621.
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

        /// Splice a `pHYs` chunk expressing `ppu` pixels-per-metre (both axes, unit = meter)
        /// into a real encoded PNG, immediately after IHDR (signature 8 bytes + IHDR chunk 25
        /// bytes) and always before IDAT.
        fn png_with_phys_density(width: u32, height: u32, ppu: u32) -> Vec<u8> {
            const SIGNATURE_AND_IHDR_LEN: usize = 8 + 25;
            let img = image::RgbImage::from_pixel(width, height, image::Rgb([255, 255, 255]));
            let png = encode_rgb_as_png(img.as_raw(), width, height).expect("encode fixture PNG");

            let mut phys_data = Vec::with_capacity(9);
            phys_data.extend_from_slice(&ppu.to_be_bytes());
            phys_data.extend_from_slice(&ppu.to_be_bytes());
            phys_data.push(1); // unit = meter
            let phys_chunk = png_chunk(b"pHYs", &phys_data);

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

        fn images_config_targeting(target_dpi: i32) -> crate::core::config::ImageExtractionConfig {
            crate::core::config::ImageExtractionConfig {
                target_dpi,
                auto_adjust_dpi: false,
                ..Default::default()
            }
        }

        #[cfg(all(feature = "pdf", feature = "ocr-pipeline"))]
        #[tokio::test]
        #[serial_test::serial]
        async fn standalone_image_pipeline_keeps_whole_image_scan_hints() {
            use crate::core::config::{OcrConfig, OcrPipelineConfig, OcrPipelineStage, OcrQualityThresholds};
            use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
            use crate::types::ExtractedDocument;
            use std::sync::{Arc, Mutex};

            struct TesseractHintCapturingBackend {
                config: Mutex<Option<OcrConfig>>,
            }

            #[async_trait::async_trait]
            impl OcrBackend for TesseractHintCapturingBackend {
                fn backend_type(&self) -> OcrBackendType {
                    OcrBackendType::Custom
                }
                fn supports_language(&self, _: &str) -> bool {
                    true
                }
                async fn process_image(&self, _: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
                    *self.config.lock().unwrap() = Some(config.clone());
                    Ok(ExtractedDocument {
                        content: "standalone image text".to_string(),
                        ..Default::default()
                    })
                }
            }

            impl Plugin for TesseractHintCapturingBackend {
                fn name(&self) -> &str {
                    "tesseract"
                }
                fn version(&self) -> String {
                    "1.0.0".to_string()
                }
                fn initialize(&self) -> crate::Result<()> {
                    Ok(())
                }
                fn shutdown(&self) -> crate::Result<()> {
                    Ok(())
                }
            }

            crate::plugins::unregister_ocr_backend("tesseract").unwrap();
            let backend = Arc::new(TesseractHintCapturingBackend {
                config: Mutex::new(None),
            });
            crate::plugins::register_ocr_backend(backend.clone()).unwrap();

            let pipeline = OcrPipelineConfig {
                stages: vec![OcrPipelineStage {
                    backend: "tesseract".to_string(),
                    priority: 100,
                    language: None,
                    tesseract_config: None,
                    paddle_ocr_config: None,
                    paddle_ocr_settings: None,
                    vlm_config: None,
                    backend_options: None,
                }],
                quality_thresholds: OcrQualityThresholds {
                    pipeline_min_quality: 0.0,
                    ..Default::default()
                },
            };
            let config = ExtractionConfig {
                ocr: Some(OcrConfig {
                    pipeline: Some(pipeline.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            let png = png_with_phys_density(16, 16, REPORTER_FIXTURE_PPU);

            ImageExtractor::new()
                .extract_with_ocr_pipeline(&png, &config, &pipeline)
                .await
                .expect("standalone image pipeline succeeds");

            crate::plugins::unregister_ocr_backend("tesseract").unwrap();
            crate::plugins::ensure_ocr_backends_initialized();

            let captured = backend.config.lock().unwrap().clone().expect("backend was called");
            let source_dpi = captured
                .backend_options
                .as_ref()
                .and_then(|options| options.get("source_dpi"))
                .and_then(serde_json::Value::as_f64);
            assert!(
                source_dpi.is_some_and(|dpi| (dpi - EXPECTED_DPI_FROM_REPORTER_FIXTURE).abs() < DPI_TOLERANCE),
                "pipeline must preserve the standalone image's embedded source DPI; got {source_dpi:?}"
            );
            assert_eq!(
                captured
                    .backend_options
                    .as_ref()
                    .and_then(|options| options.get("known_full_page_scan"))
                    .and_then(serde_json::Value::as_bool),
                Some(true)
            );
            assert!(
                captured
                    .tesseract_config
                    .as_ref()
                    .and_then(|config| config.psm)
                    .is_some()
            );
        }

        /// Unit-level check on `normalize_image_bytes_for_ocr` directly: a `target_dpi=300`
        /// request against a genuine ~300 DPI PNG must not resize, and must report the true
        /// embedded density rather than the fabricated 72 assumption.
        #[cfg(feature = "ocr-pipeline")]
        #[test]
        fn does_not_resize_when_target_dpi_matches_known_png_density() {
            let png = png_with_phys_density(16, 16, REPORTER_FIXTURE_PPU);
            let images_config = images_config_targeting(300);

            let normalized = normalize_image_bytes_for_ocr(
                &png,
                &images_config,
                &crate::extractors::security::SecurityLimits::default(),
                &crate::core::config::OcrConfig::default(),
            )
            .expect("normalization must succeed");

            let metadata = normalized.metadata.expect("preprocessing metadata is recorded");
            assert!(
                metadata.skipped_resize,
                "a 300 target against a ~300 detected source must not resize"
            );
            assert!(
                (metadata.original_dpi.horizontal - EXPECTED_DPI_FROM_REPORTER_FIXTURE).abs() < DPI_TOLERANCE,
                "expected ~{EXPECTED_DPI_FROM_REPORTER_FIXTURE} DPI, got {}",
                metadata.original_dpi.horizontal
            );
        }

        /// Control: a PNG with no density metadata must still default to 72 DPI and still
        /// resize against a 300 target — the pre-fix, and still-correct-for-this-case, behaviour.
        #[cfg(feature = "ocr-pipeline")]
        #[test]
        fn still_resizes_and_assumes_72_dpi_when_png_has_no_density_metadata() {
            let img = image::RgbImage::from_pixel(16, 16, image::Rgb([255, 255, 255]));
            let png = encode_rgb_as_png(img.as_raw(), 16, 16).expect("encode fixture PNG");
            let images_config = images_config_targeting(300);

            let normalized = normalize_image_bytes_for_ocr(
                &png,
                &images_config,
                &crate::extractors::security::SecurityLimits::default(),
                &crate::core::config::OcrConfig::default(),
            )
            .expect("normalization must succeed");

            let metadata = normalized.metadata.expect("preprocessing metadata is recorded");
            assert!(
                !metadata.skipped_resize,
                "a 300 target against an assumed-72 source must resize"
            );
            assert_eq!(
                metadata.original_dpi,
                crate::types::ImageDpi::from((72.0, 72.0)),
                "no embedded metadata must leave the historical 72 assumption unchanged"
            );
        }

        /// The decisive, end-to-end regression test: the reporter's own repro (minus corpus
        /// bytes — a synthetic PNG built with valid CRCs and an explicit `pHYs`), run through
        /// the real public `xberg::extract` API with `target_dpi=300`, `auto_adjust_dpi=false`,
        /// `use_cache=false`, `force_ocr=true`. Before the fix this reports
        /// `original_dpi: 72.0`, `scale_factor: ~4.17`, and a resized raster; after the fix it
        /// must report `original_dpi ≈ 300` and no resize.
        #[cfg(feature = "ocr")]
        #[tokio::test]
        async fn end_to_end_extract_honours_embedded_png_density_at_the_reporters_target_dpi() {
            let png = png_with_phys_density(16, 16, REPORTER_FIXTURE_PPU);

            let config = ExtractionConfig {
                images: Some(images_config_targeting(300)),
                ocr: Some(crate::core::config::OcrConfig::default()),
                force_ocr: true,
                use_cache: false,
                disable_ocr: false,
                ..Default::default()
            };

            let input = crate::core::config::ExtractInput::from_bytes(png, "image/png", Some("gh1630.png".to_string()));
            let result = crate::extract(input, &config).await.expect("extraction must succeed");
            let doc = result.results.first().expect("one document must be returned");
            let metadata = doc
                .metadata
                .image_preprocessing
                .clone()
                .expect("image_preprocessing metadata must be recorded for OCR'd images");

            assert!(
                (metadata.original_dpi.horizontal - EXPECTED_DPI_FROM_REPORTER_FIXTURE).abs() < DPI_TOLERANCE,
                "GH#1630: expected original_dpi ~{EXPECTED_DPI_FROM_REPORTER_FIXTURE}, got {}",
                metadata.original_dpi.horizontal
            );
            assert!(
                metadata.skipped_resize,
                "GH#1630: target_dpi=300 against a genuine ~300 DPI source must not resize \
                 (scale_factor was previously target/72 = ~4.17)"
            );
        }
    }

    /// #1787 part 1: the top-level `ExtractionConfig.use_cache = false` only bypassed the
    /// whole-extraction cache (`extract_file_with_extractor`); the OCR backend's own result
    /// cache is governed independently by `TesseractConfig.use_cache`, which defaults to
    /// `true`, so a caller asking for a fresh measurement through the top-level flag still got
    /// a cached OCR result. `extract_with_ocr` must propagate `use_cache: false` into
    /// `OcrConfig.backend_options["use_cache"]`, the channel `TesseractBackend::config_to_tesseract`
    /// already reads (`use_cache_from_backend_options`) — this test proves the image extractor
    /// actually sets it, using a capturing mock backend so no real Tesseract/tessdata is needed.
    // Gated on the module, not on each test: every item below is a helper for the two
    // `ocr` tests, so a feature combination that keeps no tests leaves them all dead and
    // fails the narrow `formula-recognition,pdf` clippy leg under `-D warnings`. ~keep
    #[cfg(feature = "ocr")]
    mod use_cache_propagation {
        use super::*;
        use crate::core::config::OcrConfig;
        use crate::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
        use crate::types::ExtractedDocument;
        use std::sync::Mutex;

        // Each test registers under its own name: `register_ocr_backend`/`unregister_ocr_backend`
        // act on the process-wide registry, and `#[tokio::test]`s in this binary run
        // concurrently, so a shared name would let one test's unregister race another's lookup.
        struct CapturingBackend {
            name: &'static str,
            seen: Mutex<Option<OcrConfig>>,
        }

        impl CapturingBackend {
            fn new(name: &'static str) -> Self {
                Self {
                    name,
                    seen: Mutex::new(None),
                }
            }
        }

        #[async_trait::async_trait]
        impl OcrBackend for CapturingBackend {
            fn backend_type(&self) -> OcrBackendType {
                OcrBackendType::Custom
            }
            fn supports_language(&self, _: &str) -> bool {
                true
            }
            async fn process_image(&self, _: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
                *self.seen.lock().unwrap() = Some(config.clone());
                Ok(ExtractedDocument {
                    content: "captured".to_string(),
                    ..Default::default()
                })
            }
        }

        impl Plugin for CapturingBackend {
            fn name(&self) -> &str {
                self.name
            }
            fn version(&self) -> String {
                "0.0.0".to_string()
            }
            fn initialize(&self) -> crate::Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> crate::Result<()> {
                Ok(())
            }
        }

        fn tiny_png() -> Vec<u8> {
            let mut buf = std::io::Cursor::new(Vec::new());
            image::ImageBuffer::<image::Rgb<u8>, _>::from_pixel(1, 1, image::Rgb([255u8, 255, 255]))
                .write_to(&mut buf, image::ImageFormat::Png)
                .expect("failed to encode test PNG");
            buf.into_inner()
        }

        fn use_cache_option_seen(seen: &OcrConfig) -> Option<bool> {
            seen.backend_options
                .as_ref()
                .and_then(|options| options.get("use_cache"))
                .and_then(serde_json::Value::as_bool)
        }

        #[tokio::test]
        async fn top_level_use_cache_false_disables_the_ocr_backend_cache() {
            const NAME: &str = "capturing-1787a-disables-cache-test";
            let backend = std::sync::Arc::new(CapturingBackend::new(NAME));
            register_ocr_backend(backend.clone()).unwrap();

            let config = ExtractionConfig {
                ocr: Some(OcrConfig {
                    backend: NAME.to_string(),
                    ..Default::default()
                }),
                use_cache: false,
                ..Default::default()
            };

            let extractor = ImageExtractor::new();
            extractor
                .extract_content(&tiny_png(), "image/png", &config)
                .await
                .unwrap();

            let seen = backend
                .seen
                .lock()
                .unwrap()
                .clone()
                .expect("backend must have been called");
            assert_eq!(
                use_cache_option_seen(&seen),
                Some(false),
                "top-level use_cache=false must turn off the OCR backend's own result cache, \
                 not just the whole-extraction cache (#1787)"
            );

            unregister_ocr_backend(NAME).unwrap();
        }

        /// Negative control: a caller who leaves the top-level `use_cache` at its default
        /// (`true`) must not have anything injected into `backend_options` — the OCR-specific
        /// cache setting they may have configured on `tesseract_config.use_cache` directly must
        /// keep governing, unmodified by this propagation.
        #[tokio::test]
        async fn default_top_level_use_cache_leaves_backend_options_untouched() {
            const NAME: &str = "capturing-1787a-default-cache-test";
            let backend = std::sync::Arc::new(CapturingBackend::new(NAME));
            register_ocr_backend(backend.clone()).unwrap();

            let config = ExtractionConfig {
                ocr: Some(OcrConfig {
                    backend: NAME.to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            };
            assert!(config.use_cache, "test assumes the default is true");

            let extractor = ImageExtractor::new();
            extractor
                .extract_content(&tiny_png(), "image/png", &config)
                .await
                .unwrap();

            let seen = backend
                .seen
                .lock()
                .unwrap()
                .clone()
                .expect("backend must have been called");
            assert!(
                use_cache_option_seen(&seen).is_none(),
                "a default (non-disabling) top-level use_cache must not inject a backend_options \
                 override, so the OCR cache still works as configured (negative control for #1787)"
            );

            unregister_ocr_backend(NAME).unwrap();
        }
    }

    #[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
    #[test]
    fn should_apply_vertical_block_psm_to_default_vertical_tesseract_config() {
        let mut ocr_config = crate::core::config::OcrConfig {
            language: vec!["jpn_vert".to_string()],
            ..Default::default()
        };

        apply_default_whole_image_tesseract_psm(&mut ocr_config);

        let tesseract_config = ocr_config
            .tesseract_config
            .as_ref()
            .expect("whole-image OCR must materialize Tesseract configuration");
        assert_eq!(tesseract_config.psm, Some(VERTICAL_BLOCK_TESSERACT_PSM));
        assert_eq!(tesseract_config.language, vec!["jpn_vert"]);
    }

    #[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
    #[test]
    fn should_apply_default_whole_image_psm_to_horizontal_tesseract_config() {
        let mut ocr_config = crate::core::config::OcrConfig {
            language: vec!["eng".to_string()],
            ..Default::default()
        };

        apply_default_whole_image_tesseract_psm(&mut ocr_config);

        let tesseract_config = ocr_config
            .tesseract_config
            .as_ref()
            .expect("whole-image OCR must materialize Tesseract configuration");
        assert_eq!(tesseract_config.psm, Some(WHOLE_IMAGE_TESSERACT_PSM));
        assert_eq!(tesseract_config.language, vec!["eng"]);
    }

    #[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
    #[test]
    fn should_preserve_explicit_whole_image_tesseract_psm() {
        let mut ocr_config = crate::core::config::OcrConfig {
            language: vec!["jpn_vert".to_string()],
            tesseract_config: Some(crate::types::TesseractConfig {
                language: vec!["jpn_vert".to_string()],
                psm: Some(4),
                ..Default::default()
            }),
            ..Default::default()
        };

        apply_default_whole_image_tesseract_psm(&mut ocr_config);

        let tesseract_config = ocr_config
            .tesseract_config
            .as_ref()
            .expect("explicit config must remain");
        assert_eq!(tesseract_config.psm, Some(4));
        assert_eq!(tesseract_config.language, vec!["jpn_vert"]);
    }

    // Regression test for #1573: `TesseractConfig()` with every field left at its
    // default must produce the SAME effective PSM as no `TesseractConfig` at all, not
    // fall back to the internal engine default (PSM 3 native / 6 wasm). Previously
    // `apply_default_tesseract_psm` keyed off `tesseract_config.is_some()`, so a caller
    // who materialized the struct (even with every field default) silently lost the
    // whole-image PSM.
    #[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
    #[test]
    fn should_apply_same_default_psm_whether_or_not_tesseract_config_struct_is_present() {
        let mut without_struct = crate::core::config::OcrConfig {
            language: vec!["eng".to_string()],
            ..Default::default()
        };
        let mut with_default_struct = crate::core::config::OcrConfig {
            language: vec!["eng".to_string()],
            tesseract_config: Some(crate::types::TesseractConfig::default()),
            ..Default::default()
        };

        apply_default_whole_image_tesseract_psm(&mut without_struct);
        apply_default_whole_image_tesseract_psm(&mut with_default_struct);

        let psm_without_struct = without_struct
            .tesseract_config
            .expect("whole-image OCR must materialize Tesseract configuration")
            .psm;
        let psm_with_default_struct = with_default_struct
            .tesseract_config
            .expect("struct was already present")
            .psm;

        assert_eq!(psm_without_struct, Some(WHOLE_IMAGE_TESSERACT_PSM));
        assert_eq!(
            psm_without_struct, psm_with_default_struct,
            "TesseractConfig::default() must resolve to the same effective PSM as no TesseractConfig at all"
        );
    }

    // An explicitly set `psm`, alongside another explicitly set field, must still be
    // honoured and not overwritten by the automatic default (#1573).
    #[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
    #[test]
    fn should_not_overwrite_explicit_psm_when_another_field_is_also_set() {
        let mut ocr_config = crate::core::config::OcrConfig {
            language: vec!["eng".to_string()],
            tesseract_config: Some(crate::types::TesseractConfig {
                psm: Some(7),
                enable_table_detection: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        apply_default_whole_image_tesseract_psm(&mut ocr_config);

        let tesseract_config = ocr_config.tesseract_config.expect("explicit config must remain");
        assert_eq!(tesseract_config.psm, Some(7));
        assert!(!tesseract_config.enable_table_detection);
    }

    #[cfg(feature = "ocr")]
    fn hocr_text_element_with_font_size(text: &str, font_size: Option<f64>) -> crate::types::internal::InternalElement {
        let mut element = crate::types::internal::InternalElement::text(
            crate::types::internal::ElementKind::OcrText {
                level: crate::types::OcrElementLevel::Block,
            },
            text,
            0,
        );
        if let Some(size) = font_size {
            element.attributes.get_or_insert_with(Default::default).insert(
                crate::ocr::hocr_parser::HOCR_FONT_SIZE_ATTRIBUTE.to_string(),
                size.to_string(),
            );
        }
        element
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn standalone_image_plain_output_never_contains_markdown_heading_syntax() {
        // Regression test: `Plain` is the default output format and must never
        // contain markdown syntax. The standalone-image OCR path used to feed a
        // pre-baked `## Nasdaq & AMEX` string (produced by the now-removed
        // heading heuristic in `ocr::processor::execution`) straight into
        // `build_image_internal_document`, which stored it as literal paragraph
        // text. `render_plain` then emitted that text unchanged, leaking `## `
        // into `Plain` output. This must fail against that behavior.
        let elements = vec![
            hocr_text_element_with_font_size("Nasdaq & AMEX", Some(28.0)),
            hocr_text_element_with_font_size("Body text at normal size.", Some(12.0)),
        ];

        let doc = build_image_internal_document_from_hocr_elements(&elements);
        let plain = crate::rendering::render_plain(&doc);

        assert!(
            !plain.contains('#'),
            "Plain output must contain no markdown syntax: {plain:?}"
        );
        assert_eq!(plain, "Nasdaq & AMEX\n\nBody text at normal size.");
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn standalone_image_markdown_output_emits_unescaped_heading() {
        // Regression test: feeding a pre-baked `## ` string into
        // `build_image_internal_document` (the old behavior) made the markdown
        // renderer treat it as literal paragraph text and escape it to
        // `\#\# Nasdaq & AMEX`, instead of a real level-1 `# Nasdaq & AMEX`
        // heading, because the renderer has no way to distinguish "this text
        // happens to start with #" from "this is a heading" once it has been
        // flattened to a string. Expressing the promotion as a real `Heading`
        // element (this function) avoids that ambiguity. This must fail
        // against the old flat-text path, which produces
        // "\\#\\# Nasdaq & AMEX" instead.
        // Three elements, not two: the promotion is median-based, so the body size must be the
        // median. With a single body element the median IS the heading size and nothing can exceed it.
        let elements = vec![
            hocr_text_element_with_font_size("Nasdaq & AMEX", Some(28.0)),
            hocr_text_element_with_font_size("Body text at normal size.", Some(12.0)),
            hocr_text_element_with_font_size("More body text at normal size.", Some(12.0)),
        ];

        let doc = build_image_internal_document_from_hocr_elements(&elements);
        let markdown = crate::rendering::render_markdown(&doc);

        assert!(
            markdown.contains("# Nasdaq & AMEX"),
            "markdown output must contain an unescaped level-1 heading: {markdown:?}"
        );
        assert!(
            !markdown.contains(r"\#"),
            "markdown output must not escape the heading marker: {markdown:?}"
        );
    }

    #[cfg(all(
        not(target_arch = "wasm32"),
        any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline")
    ))]
    mod sparse_image_ocr_fallback_tests {
        use super::*;

        fn result_with_word_confidences(confidences: &[f64]) -> crate::types::ExtractedDocument {
            let ocr_elements = confidences
                .iter()
                .enumerate()
                .map(|(index, confidence)| crate::types::OcrElement {
                    text: format!("word-{index}"),
                    confidence: crate::types::OcrConfidence {
                        recognition: *confidence,
                        ..Default::default()
                    },
                    level: crate::types::OcrElementLevel::Word,
                    ..Default::default()
                })
                .collect();
            crate::types::ExtractedDocument {
                ocr_elements: Some(ocr_elements),
                ..Default::default()
            }
        }

        fn result_with_content_and_confidence(content: &str, confidence: f64) -> crate::types::ExtractedDocument {
            let mut result = crate::types::ExtractedDocument {
                content: content.to_string(),
                ..Default::default()
            };
            result
                .metadata
                .additional
                .insert("mean_text_conf".into(), serde_json::json!(confidence));
            result
        }

        #[test]
        fn should_retry_implicit_horizontal_tesseract_when_words_are_sparse() {
            let config = crate::core::config::OcrConfig::default();
            let confidences = vec![0.10; SPARSE_IMAGE_OCR_WORD_LIMIT];
            let result = result_with_word_confidences(&confidences);

            assert!(should_retry_sparse_image_ocr(&config, &result));
        }

        #[test]
        fn should_not_retry_implicit_horizontal_tesseract_when_words_are_dense() {
            let config = crate::core::config::OcrConfig::default();
            let confidences = vec![0.10; SPARSE_IMAGE_OCR_WORD_LIMIT + 1];
            let result = result_with_word_confidences(&confidences);

            assert!(!should_retry_sparse_image_ocr(&config, &result));
        }

        #[test]
        fn should_not_retry_sparse_primary_with_robust_word_confidences() {
            let config = crate::core::config::OcrConfig::default();
            let result = result_with_word_confidences(&[0.90, 0.95]);

            assert!(!should_retry_sparse_image_ocr(&config, &result));
        }

        #[test]
        fn should_exclude_explicit_psm_and_vertical_tesseract_from_sparse_retry() {
            let result = result_with_word_confidences(&[0.10]);
            let explicit_psm_config = crate::core::config::OcrConfig {
                tesseract_config: Some(crate::types::TesseractConfig {
                    psm: Some(4),
                    ..Default::default()
                }),
                ..Default::default()
            };
            let vertical_config = crate::core::config::OcrConfig {
                language: vec!["jpn_vert".to_string()],
                ..Default::default()
            };
            let other_backend_config = crate::core::config::OcrConfig {
                backend: "paddle-ocr".to_string(),
                ..Default::default()
            };

            assert!(!should_retry_sparse_image_ocr(&explicit_psm_config, &result));
            assert!(!should_retry_sparse_image_ocr(&vertical_config, &result));
            assert!(!should_retry_sparse_image_ocr(&other_backend_config, &result));
        }

        // Regression test for #1573: a `TesseractConfig` may be present for a reason
        // unrelated to `psm` (e.g. table detection toggled off). The sparse-text retry
        // must still trigger as long as `psm` itself is unset — struct presence alone
        // no longer disables it. Previously `is_implicit_horizontal_tesseract` keyed off
        // `tesseract_config.is_none()`, so this case never retried.
        #[test]
        fn should_retry_sparse_image_ocr_when_tesseract_config_present_but_psm_unset() {
            let config = crate::core::config::OcrConfig {
                tesseract_config: Some(crate::types::TesseractConfig {
                    enable_table_detection: false,
                    ..Default::default()
                }),
                ..Default::default()
            };
            let confidences = vec![0.10; SPARSE_IMAGE_OCR_WORD_LIMIT];
            let result = result_with_word_confidences(&confidences);

            assert!(should_retry_sparse_image_ocr(&config, &result));
        }

        #[test]
        fn should_reject_high_mean_fallback_with_bad_tenth_percentile() {
            let mut confidences = vec![0.95; 16];
            confidences.extend([0.14, 0.14]);
            let mean = confidences.iter().sum::<f64>() / confidences.len() as f64;
            let result = result_with_word_confidences(&confidences);

            assert!(mean > 0.80, "fixture must model misleadingly high mean confidence");
            assert!(!has_robust_word_confidence_distribution(&result));
        }

        #[test]
        fn should_select_fallback_with_robust_word_confidences() {
            let mut confidences = vec![0.95; 9];
            confidences.push(SPARSE_IMAGE_OCR_MIN_WORD_CONFIDENCE);
            let result = result_with_word_confidences(&confidences);

            assert!(has_robust_word_confidence_distribution(&result));
        }

        #[test]
        fn should_build_psm3_fallback_with_explicit_grayscale_enhancement() {
            let mut whole_image_config = crate::core::config::OcrConfig::default();
            apply_default_whole_image_tesseract_psm(&mut whole_image_config);

            let fallback_config = sparse_image_ocr_fallback_config(&whole_image_config);
            let tesseract_config = fallback_config
                .tesseract_config
                .expect("fallback must materialize Tesseract configuration");

            assert_eq!(tesseract_config.psm, Some(SPARSE_IMAGE_OCR_FALLBACK_PSM));
            assert_eq!(
                tesseract_config
                    .preprocessing
                    .as_ref()
                    .expect("fallback must materialize preprocessing")
                    .binarization_method,
                "none"
            );
            let preprocessing = tesseract_config.preprocessing.unwrap();
            assert!(!preprocessing.deskew);
            assert!(preprocessing.contrast_enhance);
        }

        #[test]
        fn should_normalize_tesseract_metadata_confidence_from_zero_to_one() {
            for (raw, expected) in [(0.0, 0.0), (1.0, 0.01), (91.0, 0.91), (100.0, 1.0)] {
                let result = result_with_content_and_confidence("نص", raw);
                assert_eq!(normalized_tesseract_mean_confidence(&result), Some(expected));
            }
        }

        #[test]
        fn should_select_rtl_retry_only_when_grounded_tokens_increase() {
            let primary = result_with_content_and_confidence("تقرير عربي REQ-2026-104 42", 91.0);
            let candidate = result_with_content_and_confidence("تقرير عربي كامل REQ-2026-104 42", 84.0);

            assert!(rtl_retry_candidate_improves(&primary, &candidate));
        }

        #[test]
        fn should_reject_unrecovered_rtl_retry_that_loses_or_duplicates_primary_identifiers() {
            let primary = result_with_content_and_confidence("تقرير عربي REQ-2026-104 42", 91.0);
            let lost = result_with_content_and_confidence("تقرير عربي كامل 42", 92.0);
            let duplicated = result_with_content_and_confidence("تقرير عربي كامل REQ-2026-104 REQ-2026-104 42", 92.0);

            assert!(!identifier_slots_are_preserved_after_recovery(&primary, &lost));
            assert!(!identifier_slots_are_preserved_after_recovery(&primary, &duplicated));
        }

        #[test]
        fn should_match_identifier_slots_with_close_ocr_substitution() {
            assert!(identifier_slots_are_preserved(
                "طلب REQ-2026-104 OCR-v2.1",
                "1. طلب REO-2026-104 OCR-v2.1 42"
            ));
            assert!(!identifier_slots_are_preserved(
                "طلب REQ-2026-104 OCR-v2.1",
                "طلب OCR-v2.1"
            ));
            assert!(!identifier_slots_are_preserved(
                "طلب REQ-2026-104 OCR-v2.1",
                "طلب REO-2026-104 REO-2026-104 OCR-v2.1"
            ));
            assert_eq!(
                identifier_slots("1.افتح 3.احتفظ REQ-2026-104 1,250.75 2026"),
                ["req-2026-104", "1,250.75", "2026"]
            );
        }

        #[test]
        fn should_require_compatible_identifier_evidence() {
            assert!(compatible_identifier_recovery("A17", "A-17"));
            assert!(compatible_identifier_recovery("880-2026-104", "REQ-2026-104"));
            assert!(compatible_identifier_recovery("8-09", "B-09"));
            assert!(compatible_identifier_recovery("001-2.1", "OCR-v2.1"));
            assert!(!compatible_identifier_recovery("BAD-2026", "GOOD-2026"));
            assert!(!compatible_identifier_recovery("1234567-2026-104", "REQ-2026-104"));
            assert!(!compatible_identifier_recovery("2026-10-08", "REQ-2026-104"));

            let mixed = crate::core::config::OcrConfig {
                language: vec!["ara".to_string(), "eng".to_string()],
                ..Default::default()
            };
            assert!(is_mixed_ltr_recovery_eligible_config(&mixed));
            assert!(!is_mixed_ltr_recovery_eligible_config(
                &crate::core::config::OcrConfig::default()
            ));

            let word = |text: &str, language: Option<&str>| {
                let mut element = crate::types::OcrElement {
                    text: text.to_string(),
                    level: crate::types::OcrElementLevel::Word,
                    ..Default::default()
                };
                if let Some(language) = language {
                    element
                        .backend_metadata
                        .insert("word_language".to_string(), serde_json::json!(language));
                }
                element
            };
            assert!(is_rtl_recovery_suspect(&word("880-2026-104", Some("ara")), true));
            assert!(is_rtl_recovery_suspect(&word("A17", Some("eng")), true));
            assert!(!is_rtl_recovery_suspect(&word("A17", None), true));
            assert!(!is_rtl_recovery_suspect(&word("2026-10-08", Some("ara")), true));
            assert!(!is_rtl_recovery_suspect(&word("ordinary", Some("ara")), true));
            assert!(!is_rtl_recovery_suspect(&word("A17", Some("eng")), false));

            let mut evidence = word("A-17", Some("eng"));
            evidence.confidence.recognition = 0.9;
            let duplicate = evidence.clone();
            assert!(unique_compatible_recovery_evidence("2-17", [&evidence].into_iter()).is_some());
            assert!(unique_compatible_recovery_evidence("2-17", [&evidence, &duplicate].into_iter()).is_none());
        }

        #[test]
        fn should_apply_identifier_recovery_transactionally_to_all_representations() {
            let source = "(2-17),";
            let mut selected = crate::types::ExtractedDocument {
                content: "code (2-17),".to_string(),
                ocr_elements: Some(vec![crate::types::OcrElement {
                    text: source.to_string(),
                    level: crate::types::OcrElementLevel::Word,
                    ..Default::default()
                }]),
                tables: vec![crate::types::Table {
                    cells: vec![vec!["(2-17),".to_string()]],
                    markdown: "| (2-17), |".to_string(),
                    columns: Some(vec!["(2-17),".to_string()]),
                    ..Default::default()
                }],
                ..Default::default()
            };
            selected.ocr_internal_document = Some(crate::types::internal::InternalDocument {
                elements: vec![crate::types::internal::InternalElement::text(
                    crate::types::internal::ElementKind::Paragraph,
                    "code (2-17),",
                    0,
                )],
                tables: selected.tables.clone(),
                ..Default::default()
            });
            let evidence = crate::types::OcrElement {
                text: "A-17".to_string(),
                confidence: crate::types::OcrConfidence {
                    recognition: 0.9,
                    ..Default::default()
                },
                level: crate::types::OcrElementLevel::Word,
                ..Default::default()
            };

            assert!(apply_identifier_recovery_transaction(
                &mut selected,
                0,
                source,
                &evidence
            ));
            assert_eq!(selected.content, "code (A-17),");
            assert_eq!(selected.ocr_elements.as_ref().unwrap()[0].text, "(A-17),");
            assert_eq!(selected.tables[0].cells[0][0], "(A-17),");
            assert_eq!(selected.tables[0].columns.as_ref().unwrap()[0], "(A-17),");
            assert!(selected.tables[0].markdown.contains("(A-17),"));
            let internal = selected.ocr_internal_document.as_ref().unwrap();
            assert_eq!(internal.elements[0].text, "code (A-17),");
            assert_eq!(internal.tables[0].cells[0][0], "(A-17),");

            let mut ambiguous = selected.clone();
            ambiguous.content = "2-17 and 2-17".to_string();
            ambiguous.ocr_elements.as_mut().unwrap()[0].text = "2-17".to_string();
            let before = ambiguous.clone();
            assert!(!apply_identifier_recovery_transaction(
                &mut ambiguous,
                0,
                "2-17",
                &evidence
            ));
            assert_eq!(ambiguous.content, before.content);
            assert_eq!(
                ambiguous.ocr_elements.unwrap()[0].text,
                before.ocr_elements.unwrap()[0].text
            );
        }

        #[test]
        fn should_capture_internal_words_below_public_confidence_threshold() {
            let public = crate::core::config::OcrConfig {
                language: vec!["ara".to_string(), "eng".to_string()],
                element_config: Some(crate::types::OcrElementConfig {
                    include_elements: true,
                    min_level: crate::types::OcrElementLevel::Word,
                    min_confidence: 0.95,
                    ..Default::default()
                }),
                ..Default::default()
            };
            let internal = internal_rtl_recovery_config(&public);
            assert_eq!(internal.element_config.as_ref().unwrap().min_confidence, 0.0);
            let low_confidence = crate::types::OcrElement {
                text: "2-17".to_string(),
                confidence: crate::types::OcrConfidence {
                    recognition: 0.5,
                    ..Default::default()
                },
                level: crate::types::OcrElementLevel::Word,
                ..Default::default()
            };
            assert!(public.select_public_elements(Some(vec![low_confidence])).is_none());
        }

        #[tokio::test]
        async fn should_apply_public_confidence_only_after_internal_recovery_capture() {
            use crate::plugins::{OcrBackend, OcrBackendType, Plugin};

            struct CapturingBackend(std::sync::Mutex<Option<crate::core::config::OcrConfig>>);

            #[async_trait::async_trait]
            impl OcrBackend for CapturingBackend {
                fn backend_type(&self) -> OcrBackendType {
                    OcrBackendType::Custom
                }
                fn supports_language(&self, _: &str) -> bool {
                    true
                }
                async fn process_image(
                    &self,
                    _: &[u8],
                    config: &crate::core::config::OcrConfig,
                ) -> crate::Result<crate::types::ExtractedDocument> {
                    *self.0.lock().unwrap() = Some(config.clone());
                    Ok(crate::types::ExtractedDocument {
                        content: std::iter::repeat_n("كلمة", 60).collect::<Vec<_>>().join(" "),
                        ocr_elements: Some(vec![crate::types::OcrElement {
                            text: "كلمة".to_string(),
                            confidence: crate::types::OcrConfidence {
                                recognition: 0.5,
                                ..Default::default()
                            },
                            level: crate::types::OcrElementLevel::Word,
                            ..Default::default()
                        }]),
                        ..Default::default()
                    })
                }
            }

            impl Plugin for CapturingBackend {
                fn name(&self) -> &str {
                    "capturing-recovery"
                }
                fn version(&self) -> String {
                    "1.0.0".to_string()
                }
                fn initialize(&self) -> crate::Result<()> {
                    Ok(())
                }
                fn shutdown(&self) -> crate::Result<()> {
                    Ok(())
                }
            }

            let original = crate::core::config::OcrConfig {
                language: vec!["ara".to_string(), "eng".to_string()],
                element_config: Some(crate::types::OcrElementConfig {
                    include_elements: true,
                    min_level: crate::types::OcrElementLevel::Word,
                    min_confidence: 0.95,
                    ..Default::default()
                }),
                ..Default::default()
            };
            let mut effective = original.clone();
            apply_default_whole_image_tesseract_psm(&mut effective);
            let backend = std::sync::Arc::new(CapturingBackend(std::sync::Mutex::new(None)));
            let erased: std::sync::Arc<dyn OcrBackend> = backend.clone();
            let result = process_whole_image_ocr_with_sparse_retry(
                &erased,
                std::sync::Arc::new(vec![1]),
                &original,
                &effective,
                true,
                false,
                None,
            )
            .await
            .expect("capture must succeed");

            assert_eq!(
                backend
                    .0
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .element_config
                    .as_ref()
                    .unwrap()
                    .min_confidence,
                0.0
            );
            assert!(result.ocr_elements.is_none());
        }

        #[tokio::test]
        async fn should_rollback_rtl_fallback_when_identifier_corruption_has_no_verified_recovery() {
            use crate::plugins::{OcrBackend, OcrBackendType, Plugin};

            struct SequenceBackend(std::sync::Mutex<std::collections::VecDeque<crate::types::ExtractedDocument>>);

            #[async_trait::async_trait]
            impl OcrBackend for SequenceBackend {
                fn backend_type(&self) -> OcrBackendType {
                    OcrBackendType::Custom
                }
                fn supports_language(&self, _: &str) -> bool {
                    true
                }
                async fn process_image(
                    &self,
                    _: &[u8],
                    _: &crate::core::config::OcrConfig,
                ) -> crate::Result<crate::types::ExtractedDocument> {
                    self.0
                        .lock()
                        .unwrap()
                        .pop_front()
                        .ok_or_else(|| crate::XbergError::Other("unexpected OCR call".to_string()))
                }
            }

            impl Plugin for SequenceBackend {
                fn name(&self) -> &str {
                    "sequence"
                }
                fn version(&self) -> String {
                    "1.0.0".to_string()
                }
                fn initialize(&self) -> crate::Result<()> {
                    Ok(())
                }
                fn shutdown(&self) -> crate::Result<()> {
                    Ok(())
                }
            }

            let result = |content: &str, identifier: &str, language: &str| {
                let mut word = crate::types::OcrElement {
                    text: identifier.to_string(),
                    level: crate::types::OcrElementLevel::Word,
                    geometry: crate::types::OcrBoundingGeometry::Rectangle {
                        left: 10,
                        top: 10,
                        width: 40,
                        height: 10,
                    },
                    ..Default::default()
                };
                word.backend_metadata
                    .insert("word_language".to_string(), serde_json::json!(language));
                let mut result = crate::types::ExtractedDocument {
                    content: content.to_string(),
                    ocr_elements: Some(vec![word]),
                    ..Default::default()
                };
                result
                    .metadata
                    .additional
                    .insert("mean_text_conf".into(), serde_json::json!(92.0));
                result
            };
            let primary = result("تقرير 880-2026-104", "880-2026-104", "ara");
            let fallback = result("تقرير عربي كامل BAD-2026-104", "BAD-2026-104", "eng");
            let backend: std::sync::Arc<dyn OcrBackend> = std::sync::Arc::new(SequenceBackend(std::sync::Mutex::new(
                std::collections::VecDeque::from([primary.clone(), fallback]),
            )));
            let original = crate::core::config::OcrConfig {
                language: vec!["ara".to_string(), "eng".to_string()],
                ..Default::default()
            };
            let mut effective = original.clone();
            apply_default_whole_image_tesseract_psm(&mut effective);

            let selected = process_whole_image_ocr_with_sparse_retry(
                &backend,
                std::sync::Arc::new(vec![1]),
                &original,
                &effective,
                true,
                false,
                None,
            )
            .await
            .expect("fallback evaluation must succeed");

            assert_eq!(selected.content, primary.content);
            assert!(!selected.content.contains("BAD-2026-104"));
        }

        #[tokio::test]
        async fn should_recover_primary_identifier_when_rtl_fallback_is_rejected() {
            use crate::plugins::{OcrBackend, OcrBackendType, Plugin};

            struct SequenceBackend(std::sync::Mutex<std::collections::VecDeque<crate::types::ExtractedDocument>>);

            #[async_trait::async_trait]
            impl OcrBackend for SequenceBackend {
                fn backend_type(&self) -> OcrBackendType {
                    OcrBackendType::Custom
                }
                fn supports_language(&self, _: &str) -> bool {
                    true
                }
                async fn process_image(
                    &self,
                    _: &[u8],
                    _: &crate::core::config::OcrConfig,
                ) -> crate::Result<crate::types::ExtractedDocument> {
                    self.0
                        .lock()
                        .unwrap()
                        .pop_front()
                        .ok_or_else(|| crate::XbergError::Other("unexpected OCR call".to_string()))
                }
            }

            impl Plugin for SequenceBackend {
                fn name(&self) -> &str {
                    "primary-recovery-sequence"
                }
                fn version(&self) -> String {
                    "1.0.0".to_string()
                }
                fn initialize(&self) -> crate::Result<()> {
                    Ok(())
                }
                fn shutdown(&self) -> crate::Result<()> {
                    Ok(())
                }
            }

            let mut identifier = crate::types::OcrElement {
                text: "880-2026-104".to_string(),
                level: crate::types::OcrElementLevel::Word,
                geometry: crate::types::OcrBoundingGeometry::Rectangle {
                    left: 10,
                    top: 10,
                    width: 40,
                    height: 10,
                },
                ..Default::default()
            };
            identifier
                .backend_metadata
                .insert("word_language".to_string(), serde_json::json!("ara"));
            let primary = crate::types::ExtractedDocument {
                content: "تقرير 880-2026-104".to_string(),
                ocr_elements: Some(vec![identifier]),
                ..Default::default()
            };
            let fallback = primary.clone();
            let evidence = crate::types::ExtractedDocument {
                ocr_elements: Some(vec![crate::types::OcrElement {
                    text: "REQ-2026-104".to_string(),
                    confidence: crate::types::OcrConfidence {
                        recognition: 0.9,
                        ..Default::default()
                    },
                    level: crate::types::OcrElementLevel::Word,
                    ..Default::default()
                }]),
                ..Default::default()
            };
            let backend: std::sync::Arc<dyn OcrBackend> = std::sync::Arc::new(SequenceBackend(std::sync::Mutex::new(
                std::collections::VecDeque::from([primary, fallback, evidence]),
            )));
            let original = crate::core::config::OcrConfig {
                language: vec!["ara".to_string(), "eng".to_string()],
                ..Default::default()
            };
            let mut effective = original.clone();
            apply_default_whole_image_tesseract_psm(&mut effective);
            let mut encoded = std::io::Cursor::new(Vec::new());
            image::DynamicImage::new_rgb8(100, 50)
                .write_to(&mut encoded, image::ImageFormat::Png)
                .expect("test image must encode");

            let selected = process_whole_image_ocr_with_sparse_retry(
                &backend,
                std::sync::Arc::new(encoded.into_inner()),
                &original,
                &effective,
                true,
                false,
                None,
            )
            .await
            .expect("primary recovery must succeed");

            assert_eq!(selected.content, "تقرير REQ-2026-104");
        }

        #[tokio::test]
        async fn should_preserve_selected_ocr_when_optional_identifier_recovery_fails() {
            use crate::plugins::{OcrBackend, OcrBackendType, Plugin};

            struct FailingRecoveryBackend;

            #[async_trait::async_trait]
            impl OcrBackend for FailingRecoveryBackend {
                fn backend_type(&self) -> OcrBackendType {
                    OcrBackendType::Custom
                }
                fn supports_language(&self, _: &str) -> bool {
                    true
                }
                async fn process_image(
                    &self,
                    _: &[u8],
                    _: &crate::core::config::OcrConfig,
                ) -> crate::Result<crate::types::ExtractedDocument> {
                    Err(crate::XbergError::Other("recovery failed".to_string()))
                }
            }

            impl Plugin for FailingRecoveryBackend {
                fn name(&self) -> &str {
                    "failing-recovery"
                }
                fn version(&self) -> String {
                    "1.0.0".to_string()
                }
                fn initialize(&self) -> crate::Result<()> {
                    Ok(())
                }
                fn shutdown(&self) -> crate::Result<()> {
                    Ok(())
                }
            }

            let mut word = crate::types::OcrElement {
                text: "880-2026-104".to_string(),
                level: crate::types::OcrElementLevel::Word,
                geometry: crate::types::OcrBoundingGeometry::Rectangle {
                    left: 1,
                    top: 1,
                    width: 8,
                    height: 8,
                },
                ..Default::default()
            };
            word.backend_metadata
                .insert("word_language".to_string(), serde_json::json!("ara"));
            let mut selected = crate::types::ExtractedDocument {
                content: "880-2026-104".to_string(),
                ocr_elements: Some(vec![word]),
                ..Default::default()
            };
            let mut encoded = std::io::Cursor::new(Vec::new());
            image::DynamicImage::new_rgb8(10, 10)
                .write_to(&mut encoded, image::ImageFormat::Png)
                .expect("test image must encode");
            let image = encoded.into_inner();
            let backend: std::sync::Arc<dyn OcrBackend> = std::sync::Arc::new(FailingRecoveryBackend);

            recover_selected_rtl_identifiers_fail_open(
                &backend,
                std::sync::Arc::new(image.clone()),
                &mut selected,
                &crate::core::config::OcrConfig::default(),
                None,
            )
            .await
            .expect("optional recovery failure must remain non-fatal");

            assert_eq!(selected.content, "880-2026-104");

            let cancelled = crate::cancellation::CancellationToken::new();
            cancelled.cancel();
            assert!(matches!(
                recover_selected_rtl_identifiers_fail_open(
                    &backend,
                    std::sync::Arc::new(image.clone()),
                    &mut selected,
                    &crate::core::config::OcrConfig::default(),
                    Some(&cancelled),
                )
                .await,
                Err(crate::XbergError::Cancelled)
            ));

            struct CancellingRecoveryBackend(crate::cancellation::CancellationToken);

            #[async_trait::async_trait]
            impl OcrBackend for CancellingRecoveryBackend {
                fn backend_type(&self) -> OcrBackendType {
                    OcrBackendType::Custom
                }
                fn supports_language(&self, _: &str) -> bool {
                    true
                }
                async fn process_image(
                    &self,
                    _: &[u8],
                    _: &crate::core::config::OcrConfig,
                ) -> crate::Result<crate::types::ExtractedDocument> {
                    self.0.cancel();
                    Ok(crate::types::ExtractedDocument::default())
                }
            }

            impl Plugin for CancellingRecoveryBackend {
                fn name(&self) -> &str {
                    "cancelling-recovery"
                }
                fn version(&self) -> String {
                    "1.0.0".to_string()
                }
                fn initialize(&self) -> crate::Result<()> {
                    Ok(())
                }
                fn shutdown(&self) -> crate::Result<()> {
                    Ok(())
                }
            }

            let cancellation = crate::cancellation::CancellationToken::new();
            let backend: std::sync::Arc<dyn OcrBackend> =
                std::sync::Arc::new(CancellingRecoveryBackend(cancellation.clone()));
            assert!(matches!(
                recover_selected_rtl_identifiers_fail_open(
                    &backend,
                    std::sync::Arc::new(image),
                    &mut selected,
                    &crate::core::config::OcrConfig::default(),
                    Some(&cancellation),
                )
                .await,
                Err(crate::XbergError::Cancelled)
            ));
        }

        #[test]
        fn should_reject_longer_rtl_retry_with_repeated_lines_or_low_confidence() {
            let primary = result_with_content_and_confidence("تقرير عربي REQ-2026-104", 91.0);
            let repeated =
                result_with_content_and_confidence("تقرير عربي REQ-2026-104\nكلام زائد\nكلام زائد\nكلام زائد", 92.0);
            let repeated_phrase = result_with_content_and_confidence(
                "تقرير عربي REQ-2026-104 كلام زائد جدا كلام زائد جدا كلام زائد جدا",
                92.0,
            );
            let low_confidence = result_with_content_and_confidence("تقرير عربي كامل REQ-2026-104", 80.0);

            assert!(!rtl_retry_candidate_improves(&primary, &repeated));
            assert!(!rtl_retry_candidate_improves(&primary, &repeated_phrase));
            assert!(!rtl_retry_candidate_improves(&primary, &low_confidence));
        }

        #[test]
        fn should_reject_rtl_retry_that_replaces_too_many_primary_tokens() {
            let primary =
                result_with_content_and_confidence("واحد اثنان ثلاثة أربعة خمسة ستة سبعة ثمانية تسعة عشرة", 91.0);
            let candidate =
                result_with_content_and_confidence("واحد اثنان ثلاثة أربعة خمسة ستة سبعة جديد إضافي كامل مفيد", 92.0);

            assert!(!rtl_retry_candidate_improves(&primary, &candidate));
        }

        #[test]
        fn should_bound_rtl_retry_by_unique_and_total_content_tokens() {
            let original = crate::core::config::OcrConfig {
                language: vec!["ara".to_string()],
                ..Default::default()
            };
            let mut effective = original.clone();
            apply_default_whole_image_tesseract_psm(&mut effective);
            for (unique, total, expected) in [(32, 48, true), (33, 33, false), (32, 49, false)] {
                let tokens = (0..total)
                    .map(|index| format!("كلمة{}", index % unique))
                    .collect::<Vec<_>>();
                let primary = crate::types::ExtractedDocument {
                    content: tokens.join(" "),
                    ..Default::default()
                };
                assert_eq!(should_retry_sparse_rtl_ocr(&original, &effective, &primary), expected);
            }
        }

        #[test]
        fn should_retry_only_implicit_strong_rtl_tesseract_with_psm11_primary() {
            let implicit_arabic = crate::core::config::OcrConfig {
                language: vec!["ara".to_string()],
                ..Default::default()
            };
            let explicit_arabic = crate::core::config::OcrConfig {
                language: vec!["ara".to_string()],
                tesseract_config: Some(crate::types::TesseractConfig {
                    psm: Some(11),
                    language: vec!["ara".to_string()],
                    ..Default::default()
                }),
                ..Default::default()
            };
            let implicit_english = crate::core::config::OcrConfig::default();

            assert!(is_implicit_strong_rtl_tesseract(&implicit_arabic));
            assert!(!is_implicit_strong_rtl_tesseract(&explicit_arabic));
            assert!(!is_implicit_strong_rtl_tesseract(&implicit_english));
        }

        #[cfg(feature = "ocr")]
        #[tokio::test]
        #[ignore = "requires local Tesseract Arabic and Hebrew traineddata"]
        async fn should_improve_arabic_and_preserve_hebrew_ground_truth_coverage() {
            use crate::plugins::OcrBackend;

            fn correct_token_count(actual: &str, expected: &str) -> usize {
                let actual = normalized_ocr_tokens(actual)
                    .into_iter()
                    .collect::<std::collections::HashSet<_>>();
                normalized_ocr_tokens(expected)
                    .into_iter()
                    .filter(|token| actual.contains(token))
                    .count()
            }

            let backend: std::sync::Arc<dyn OcrBackend> =
                std::sync::Arc::new(crate::ocr::tesseract_backend::TesseractBackend::new());
            let mut executed = 0;
            for (languages, image_path, ground_truth_path, strict_improvement) in [
                (
                    vec!["ara".to_string(), "eng".to_string()],
                    "images/rtl_arabic_300dpi.png",
                    "ground_truth/images/rtl_arabic_300dpi.txt",
                    true,
                ),
                (
                    vec!["heb".to_string()],
                    "images/rtl_hebrew_300dpi.png",
                    "ground_truth/images/rtl_hebrew_300dpi.txt",
                    false,
                ),
            ] {
                let language = languages[0].clone();
                let image = crate::utils::read_test_fixture(image_path).expect("required RTL image fixture is missing");
                let ground_truth = crate::utils::read_test_fixture(ground_truth_path)
                    .expect("required RTL ground-truth fixture is missing");
                let ground_truth = String::from_utf8(ground_truth).expect("ground truth must be UTF-8");
                let original = crate::core::config::OcrConfig {
                    language: languages.clone(),
                    tesseract_config: Some(crate::types::TesseractConfig {
                        language: languages,
                        use_cache: false,
                        ..Default::default()
                    }),
                    ..Default::default()
                };
                let mut effective = original.clone();
                apply_default_whole_image_tesseract_psm(&mut effective);
                enable_image_ocr_elements(&mut effective, true);
                let image = std::sync::Arc::new(image);
                let baseline = backend
                    .process_image_owned(std::sync::Arc::clone(&image), &effective)
                    .await
                    .expect("PSM 11 baseline must complete");
                let selected = process_whole_image_ocr_with_sparse_retry(
                    &backend, image, &original, &effective, true, false, None,
                )
                .await
                .expect("RTL sparse retry must complete");
                let baseline_correct = correct_token_count(&baseline.content, &ground_truth);
                let selected_correct = correct_token_count(&selected.content, &ground_truth);
                let expected_identifiers = ["REQ-2026-104", "OCR-v2.1", "A-17", "B-09"];
                let selected_exact_identifiers = expected_identifiers
                    .iter()
                    .filter(|identifier| selected.content.contains(*identifier))
                    .count();
                let selected_slots = identifier_slots(&selected.content);
                let selected_unique_slots = selected_slots.iter().collect::<std::collections::HashSet<_>>();

                assert!(
                    baseline_correct > 0,
                    "{language} baseline assertion must be non-vacuous"
                );
                if strict_improvement {
                    assert!(
                        selected_correct > baseline_correct,
                        "Arabic correct-token count must improve: {baseline_correct} -> {selected_correct}: {}",
                        selected.content
                    );
                    assert_eq!(
                        selected
                            .metadata
                            .additional
                            .get("psm")
                            .and_then(serde_json::Value::as_str),
                        Some("6"),
                        "Arabic improvement must come from the PSM 6 retry"
                    );
                    assert_eq!(selected_exact_identifiers, expected_identifiers.len());
                } else {
                    assert!(
                        selected_correct >= baseline_correct,
                        "Hebrew correct-token count must not regress"
                    );
                }
                assert_eq!(
                    selected_slots.len(),
                    selected_unique_slots.len(),
                    "selected OCR must not duplicate structural identifier slots"
                );
                executed += 1;
            }
            assert_eq!(executed, 2, "both grounded RTL fixture cases must execute");
        }

        #[cfg(feature = "pdf")]
        #[tokio::test]
        #[ignore = "requires local Tesseract Arabic traineddata"]
        async fn should_preserve_recovered_identifiers_in_scanned_pdf_content() {
            let pdf = crate::utils::read_test_fixture("pdf_scanned/rtl_arabic_scanned.pdf")
                .expect("required scanned Arabic PDF fixture is missing");
            let languages = vec!["ara".to_string(), "eng".to_string()];
            let config = crate::core::config::ExtractionConfig {
                force_ocr: true,
                use_cache: false,
                ocr: Some(crate::core::config::OcrConfig {
                    language: languages.clone(),
                    tesseract_config: Some(crate::types::TesseractConfig {
                        language: languages,
                        use_cache: false,
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            };
            let mut primary_config = config.clone();
            primary_config
                .ocr
                .as_mut()
                .and_then(|ocr| ocr.tesseract_config.as_mut())
                .expect("primary config must contain Tesseract settings")
                .psm = Some(WHOLE_IMAGE_TESSERACT_PSM);
            let primary = crate::extract(
                crate::core::config::ExtractInput::from_bytes(pdf.clone(), "application/pdf", None),
                &primary_config,
            )
            .await
            .expect("explicit primary PDF extraction must succeed");
            let primary_content = &primary
                .results
                .first()
                .expect("one primary PDF result is required")
                .content;
            let input = crate::core::config::ExtractInput::from_bytes(pdf, "application/pdf", None);
            let result = crate::extract(input, &config)
                .await
                .expect("scanned PDF extraction must succeed");
            let content = &result.results.first().expect("one PDF result is required").content;
            let ground_truth = String::from_utf8(
                crate::utils::read_test_fixture("ground_truth/images/rtl_arabic_300dpi.txt")
                    .expect("required Arabic ground truth is missing"),
            )
            .expect("Arabic ground truth must be UTF-8");
            let actual = normalized_ocr_tokens(content);
            let actual_unique = actual.iter().collect::<std::collections::HashSet<_>>();
            let primary_unique = normalized_ocr_tokens(primary_content)
                .into_iter()
                .collect::<std::collections::HashSet<_>>();
            let correct = normalized_ocr_tokens(&ground_truth)
                .iter()
                .filter(|token| actual_unique.contains(token))
                .count();
            let primary_correct = normalized_ocr_tokens(&ground_truth)
                .iter()
                .filter(|token| primary_unique.contains(*token))
                .count();
            assert!(actual.len() >= 41, "scanned PDF must preserve output coverage");
            assert!(
                correct >= primary_correct,
                "scanned PDF correct-token count must not regress: {primary_correct} -> {correct}: {content}"
            );

            for identifier in ["REQ-2026-104", "OCR-v2.1", "A-17", "B-09"] {
                assert!(content.contains(identifier), "missing {identifier}: {content}");
            }
        }

        /// Regression test for the standalone-image-OCR variant of the `TesseractConfig`
        /// duplication hazard: `apply_default_whole_image_tesseract_psm` materializes
        /// `crate::types::TesseractConfig::default()` (the public struct) whenever the
        /// caller never set `tesseract_config` explicitly, which is the default path for
        /// every plain image (PNG/JPG/TIFF/...) OCR request. That materialized value is
        /// later converted into `crate::ocr::types::TesseractConfig` (the internal,
        /// engine-facing struct) via `From`, which carries the value across as-is — it does
        /// NOT fall back to the internal struct's own `Default`. So if the public struct's
        /// default ever drifts from the internal struct's default, standalone image OCR
        /// silently keeps using the stale public value while other callers that reach the
        /// internal `Default` directly (e.g. PDF-embedded OCR, which never calls this
        /// function) get the current one.
        ///
        /// Against the unfixed code, `crate::types::TesseractConfig::default()` still has
        /// `language_model_ngram_on: false`, so this assertion fails with `false` even
        /// though `crate::ocr::types::TesseractConfig::default()` already has `true`.
        #[test]
        fn should_apply_public_ngram_default_to_whole_image_tesseract_config() {
            let mut whole_image_config = crate::core::config::OcrConfig::default();
            apply_default_whole_image_tesseract_psm(&mut whole_image_config);

            let tesseract_config = whole_image_config
                .tesseract_config
                .expect("default whole-image OCR must materialize a Tesseract configuration");

            assert!(
                tesseract_config.language_model_ngram_on,
                "standalone image OCR must use the same language_model_ngram_on default as the \
                 internal TesseractConfig, not a stale value baked in from the public struct's \
                 own Default impl"
            );
        }
    }

    fn image_ocr_document(text: &str) -> InternalDocument {
        build_image_internal_document(Some(text), None)
    }

    #[test]
    fn should_mark_metadata_when_standalone_image_ocr_succeeds() {
        let mut doc = image_ocr_document("recognized text");

        ImageExtractor::mark_ocr_extraction(&mut doc);
        let result =
            crate::extraction::derive::derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);

        assert!(result.metadata.ocr_used);
        assert_eq!(result.extraction_method, Some(crate::types::ExtractionMethod::Ocr));
    }

    #[tokio::test]
    async fn should_not_mark_metadata_when_standalone_image_ocr_is_disabled() {
        let mut png = std::io::Cursor::new(Vec::new());
        image::ImageBuffer::<image::Rgb<u8>, _>::from_pixel(1, 1, image::Rgb([255u8, 255, 255]))
            .write_to(&mut png, image::ImageFormat::Png)
            .expect("failed to encode test PNG");
        let config = ExtractionConfig {
            disable_ocr: true,
            ..Default::default()
        };

        let doc = ImageExtractor::new()
            .extract_content(&png.into_inner(), "image/png", &config)
            .await
            .expect("metadata-only image extraction must succeed");
        let result =
            crate::extraction::derive::derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);

        assert!(!result.metadata.ocr_used);
        assert_eq!(result.extraction_method, None);
    }

    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn should_recover_receipt_header_with_implicit_sparse_image_fallback() {
        let Some(receipt) = crate::utils::read_test_fixture("images/cord_receipt_02.jpg") else {
            return;
        };
        let config = ExtractionConfig {
            use_cache: false,
            force_ocr: true,
            ocr: Some(crate::core::config::OcrConfig::default()),
            ..Default::default()
        };

        let doc = ImageExtractor::new()
            .extract_content(&receipt, "image/jpeg", &config)
            .await
            .expect("receipt OCR must succeed");
        let result =
            crate::extraction::derive::derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
        let normalized = result.content.to_ascii_lowercase();

        assert!(
            normalized.contains("j.stb promo"),
            "implicit sparse-image fallback must recover the grounded receipt header; got {normalized:?}"
        );
    }

    #[test]
    fn image_ocr_preserves_blank_line_paragraph_boundaries() {
        let doc = image_ocr_document("\nfirst line\nwrapped line\n\n\nsecond paragraph\n");
        let paragraphs = doc
            .elements
            .iter()
            .filter(|element| matches!(element.kind, crate::types::internal::ElementKind::Paragraph))
            .map(|element| element.text.as_str())
            .collect::<Vec<_>>();

        assert_eq!(paragraphs, ["first line\nwrapped line", "second paragraph"]);
    }

    #[test]
    fn image_ocr_keeps_single_newline_wrapping_in_one_paragraph() {
        let doc = image_ocr_document("first line\nwrapped line");
        let paragraphs = doc
            .elements
            .iter()
            .filter(|element| matches!(element.kind, crate::types::internal::ElementKind::Paragraph))
            .map(|element| element.text.as_str())
            .collect::<Vec<_>>();

        assert_eq!(paragraphs, ["first line\nwrapped line"]);
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    fn positioned_word(text: &str, left: u32, top: u32) -> crate::types::OcrElement {
        positioned_word_box(text, left, top, 40, 20)
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    fn positioned_word_box(text: &str, left: u32, top: u32, width: u32, height: u32) -> crate::types::OcrElement {
        crate::types::OcrElement::new(
            text,
            crate::types::OcrBoundingGeometry::Rectangle {
                left,
                top,
                width,
                height,
            },
            crate::types::OcrConfidence::from_tesseract(95.0),
        )
        .with_level(crate::types::OcrElementLevel::Word)
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    fn positioned_line_box(text: &str, left: u32, top: u32, width: u32, height: u32) -> crate::types::OcrElement {
        crate::types::OcrElement::new(
            text,
            crate::types::OcrBoundingGeometry::Rectangle {
                left,
                top,
                width,
                height,
            },
            crate::types::OcrConfidence::from_tesseract(95.0),
        )
        .with_level(crate::types::OcrElementLevel::Line)
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    fn whole_image_doc_with_elements(
        text: &str,
        elements: Vec<crate::types::OcrElement>,
        width: u32,
        height: u32,
    ) -> InternalDocument {
        let mut doc = image_ocr_document(text);
        doc.prebuilt_ocr_elements = Some(elements);
        doc.prebuilt_pages = Some(vec![crate::types::PageContent {
            page_number: 1,
            content: text.to_string(),
            tables: vec![],
            image_indices: vec![],
            image_preprocessing: None,
            hierarchy: None,
            is_blank: None,
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
            ocr_confidence: None,
            native_content: None,
        }]);
        doc.metadata.additional.insert(
            std::borrow::Cow::Borrowed(crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY),
            serde_json::json!(width),
        );
        doc.metadata.additional.insert(
            std::borrow::Cow::Borrowed(crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY),
            serde_json::json!(height),
        );
        doc
    }

    #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
    #[test]
    fn shared_detected_image_preserves_content_dimensions_and_backing_buffer() {
        let rgb = image::RgbImage::from_raw(2, 1, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let pixels = rgb.as_raw().as_ptr();

        let shared = share_detected_image(rgb);

        assert_eq!(shared.dimensions(), (2, 1));
        assert_eq!(shared.as_raw(), &[1, 2, 3, 4, 5, 6]);
        assert_eq!(shared.as_raw().as_ptr(), pixels);

        let task_image = std::sync::Arc::clone(&shared);
        assert!(std::sync::Arc::ptr_eq(&task_image, &shared));
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn should_preserve_requested_element_level_without_layout() {
        let mut config = crate::core::config::OcrConfig {
            element_config: Some(crate::types::OcrElementConfig {
                min_level: crate::types::OcrElementLevel::Line,
                ..Default::default()
            }),
            ..Default::default()
        };

        enable_image_ocr_elements(&mut config, false);

        let element_config = config.element_config.unwrap();
        assert!(element_config.include_elements);
        assert_eq!(element_config.min_level, crate::types::OcrElementLevel::Line);
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_request_word_elements_for_layout_assembly() {
        let mut config = crate::core::config::OcrConfig::default();

        enable_image_ocr_elements(&mut config, true);

        let element_config = config.element_config.unwrap();
        assert!(element_config.include_elements);
        assert_eq!(element_config.min_level, crate::types::OcrElementLevel::Word);
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_select_line_text_for_semantics_and_word_boxes_for_tables() {
        let elements = vec![
            positioned_line_box("Hello, world!", 10, 10, 120, 20),
            positioned_word_box("Hello,", 10, 10, 50, 20),
            positioned_word_box("world!", 70, 10, 60, 20),
        ];
        let transform = OcrCoordinateTransform {
            processed_width: 200,
            processed_height: 100,
            scale_x: 1.0,
            scale_y: 1.0,
        };

        let semantic = transformed_ocr_elements(&elements, transform, crate::types::OcrElementLevel::Line).unwrap();
        let semantic_refs = semantic.iter().collect::<Vec<_>>();
        assert_eq!(semantic.len(), 1);
        assert_eq!(text_from_positioned_elements(&semantic_refs), "Hello, world!");

        let table = transformed_ocr_elements(&elements, transform, crate::types::OcrElementLevel::Word).unwrap();
        assert_eq!(
            table.iter().map(|element| element.text.as_str()).collect::<Vec<_>>(),
            ["Hello,", "world!"]
        );
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_fall_back_when_preferred_ocr_granularity_is_missing() {
        let lines = vec![positioned_line_box("Line only", 10, 10, 80, 20)];
        let words = vec![positioned_word("Word", 10, 10)];

        assert_eq!(
            preferred_ocr_elements(&lines, crate::types::OcrElementLevel::Word)[0].level,
            crate::types::OcrElementLevel::Line
        );
        assert_eq!(
            preferred_ocr_elements(&words, crate::types::OcrElementLevel::Line)[0].level,
            crate::types::OcrElementLevel::Word
        );
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_use_word_geometry_when_tesseract_lines_are_vertical_columns() {
        let elements = vec![
            positioned_line_box("vertical column", 120, 10, 20, 140),
            positioned_word_box("vertical", 120, 10, 20, 50),
            positioned_word_box("column", 120, 70, 20, 50),
        ];

        let selected = preferred_ocr_elements(&elements, crate::types::OcrElementLevel::Line);

        assert_eq!(
            selected.iter().map(|element| element.level).collect::<Vec<_>>(),
            [crate::types::OcrElementLevel::Word, crate::types::OcrElementLevel::Word]
        );
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_keep_tesseract_lines_when_their_geometry_is_horizontal() {
        let elements = vec![
            positioned_line_box("horizontal line", 10, 20, 140, 20),
            positioned_word_box("horizontal", 10, 20, 70, 20),
            positioned_word_box("line", 90, 20, 40, 20),
        ];

        let selected = preferred_ocr_elements(&elements, crate::types::OcrElementLevel::Line);

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].level, crate::types::OcrElementLevel::Line);
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_measure_mixed_level_layout_retention_once_at_line_granularity() {
        let detections = vec![crate::layout::LayoutDetection::new(
            crate::layout::LayoutClass::Text,
            0.96,
            crate::layout::BBox::new(0.0, 0.0, 100.0, 50.0),
        )];
        let elements = vec![
            positioned_line_box("inside outside", 10, 10, 80, 20),
            positioned_word_box("inside", 10, 10, 35, 20),
            positioned_word_box("outside", 140, 10, 45, 20),
        ];
        let whole = whole_image_doc_with_elements("inside outside", elements, 200, 100);

        assert_eq!(
            whole_image_layout_mapping_retention(&whole, &detections, 200, 100),
            Some(1.0)
        );
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_retain_canonical_whole_image_when_layout_coverage_is_low() {
        let detections = vec![crate::layout::LayoutDetection::new(
            crate::layout::LayoutClass::Title,
            0.96,
            crate::layout::BBox::new(0.0, 0.0, 60.0, 60.0),
        )];
        let elements = vec![positioned_word("inside", 10, 20), positioned_word("outside", 100, 20)];
        let mut whole = whole_image_doc_with_elements("Inside, outside!", elements, 200, 100);
        whole.relationships.push(crate::types::internal::Relationship {
            source: 0,
            target: crate::types::internal::RelationshipTarget::Index(0),
            kind: crate::types::document_structure::RelationshipKind::CrossReference,
        });
        let original = whole.clone();

        let retained = try_retain_canonical_whole_image_ocr(&whole, &detections, 200, 100, true)
            .expect("50% layout coverage must retain canonical whole-image OCR");

        assert_eq!(retained.elements, original.elements);
        assert_eq!(retained.relationships, original.relationships);
        assert_eq!(
            serde_json::to_value(&retained.prebuilt_ocr_elements).unwrap(),
            serde_json::to_value(&original.prebuilt_ocr_elements).unwrap()
        );
        assert_eq!(retained.prebuilt_pages.as_ref().unwrap()[0].content, "Inside, outside!");
        assert_eq!(
            retained.prebuilt_pages.as_ref().unwrap()[0]
                .layout_regions
                .as_ref()
                .map(Vec::len),
            Some(1)
        );
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_retain_canonical_whole_image_when_layout_coverage_is_sufficient() {
        let detections = vec![crate::layout::LayoutDetection::new(
            crate::layout::LayoutClass::Text,
            0.96,
            crate::layout::BBox::new(0.0, 0.0, 200.0, 100.0),
        )];
        let elements = vec![positioned_word("inside", 10, 20)];
        let whole = whole_image_doc_with_elements("inside", elements, 200, 100);

        let retained = try_retain_canonical_whole_image_ocr(&whole, &detections, 200, 100, true)
            .expect("successful single-frame OCR must avoid repeated region OCR");

        assert_eq!(retained.elements, whole.elements);
        assert_eq!(retained.prebuilt_pages.as_ref().unwrap()[0].content, "inside");
        assert_eq!(
            retained.prebuilt_pages.as_ref().unwrap()[0]
                .layout_regions
                .as_ref()
                .map(Vec::len),
            Some(1)
        );
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_disable_redundant_tesseract_analysis_for_region_ocr() {
        let extraction_config = ExtractionConfig::default();
        let ocr_config = crate::core::config::OcrConfig::default();

        let (_, region_config) = configured_region_ocr(&extraction_config, &ocr_config).unwrap();
        let tesseract_config = region_config
            .tesseract_config
            .expect("region OCR must materialize Tesseract configuration");

        assert_eq!(
            region_config.output_format,
            Some(crate::core::config::OutputFormat::Plain)
        );
        assert_eq!(tesseract_config.output_format, "text");
        assert_eq!(tesseract_config.psm, Some(6));
        assert!(!tesseract_config.enable_table_detection);
        assert!(ocr_config.tesseract_config.is_none());
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_preserve_explicit_tesseract_psm_for_region_ocr() {
        let extraction_config = ExtractionConfig::default();
        let ocr_config = crate::core::config::OcrConfig {
            tesseract_config: Some(crate::types::TesseractConfig {
                psm: Some(4),
                ..Default::default()
            }),
            ..Default::default()
        };

        let (_, region_config) = configured_region_ocr(&extraction_config, &ocr_config).unwrap();

        assert_eq!(
            region_config.tesseract_config.expect("explicit config must remain").psm,
            Some(4)
        );
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_assemble_cached_headings_without_recognized_tables() {
        let detections = vec![
            crate::layout::LayoutDetection::new(
                crate::layout::LayoutClass::Title,
                0.98,
                crate::layout::BBox::new(0.0, 0.0, 200.0, 30.0),
            ),
            crate::layout::LayoutDetection::new(
                crate::layout::LayoutClass::SectionHeader,
                0.96,
                crate::layout::BBox::new(0.0, 30.0, 200.0, 60.0),
            ),
        ];
        let elements = vec![
            positioned_word("Annual Report", 10, 5),
            positioned_word("Summary", 10, 35),
        ];
        let whole = whole_image_doc_with_elements("Annual Report Summary", elements, 200, 100);

        let assembled = try_assemble_cached_layout_document(&whole, &detections, &[], 200, 100)
            .expect("semantic detections must structure cached whole-image OCR");
        let markdown = crate::rendering::render_markdown(&assembled);

        assert_eq!(
            assembled.elements[0].kind,
            crate::types::internal::ElementKind::Heading { level: 1 }
        );
        assert_eq!(
            assembled.elements[1].kind,
            crate::types::internal::ElementKind::Heading { level: 2 }
        );
        assert!(markdown.contains("# Annual Report"));
        assert!(markdown.contains("## Summary"));
        assert_eq!(
            assembled.prebuilt_pages.as_ref().unwrap()[0]
                .layout_regions
                .as_ref()
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(alphanumeric_token_retention(&markdown, "Annual Report Summary"), 1.0);
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_keep_unmapped_invoice_tokens_when_assembling_cached_structure() {
        let detections = vec![
            crate::layout::LayoutDetection::new(
                crate::layout::LayoutClass::Title,
                0.98,
                crate::layout::BBox::new(0.0, 0.0, 100.0, 30.0),
            ),
            crate::layout::LayoutDetection::new(
                crate::layout::LayoutClass::SectionHeader,
                0.96,
                crate::layout::BBox::new(0.0, 30.0, 100.0, 60.0),
            ),
        ];
        let elements = vec![
            positioned_word("INVOICE", 10, 5),
            positioned_word("Bill To", 10, 35),
            positioned_word("John Doe", 10, 70),
            positioned_word("Invoice 123", 120, 35),
            positioned_word("Date 2025", 120, 70),
        ];
        let canonical = "INVOICE Bill To John Doe Invoice 123 Date 2025";
        let whole = whole_image_doc_with_elements(canonical, elements, 240, 100);

        let assembled = try_assemble_cached_layout_document(&whole, &detections, &[], 240, 100)
            .expect("partial invoice layout must preserve cached OCR");
        let markdown = crate::rendering::render_markdown(&assembled);

        assert_eq!(
            assembled.elements[0].kind,
            crate::types::internal::ElementKind::Heading { level: 1 }
        );
        assert_eq!(
            assembled.elements[1].kind,
            crate::types::internal::ElementKind::Heading { level: 2 }
        );
        assert!(markdown.contains("# INVOICE"));
        assert!(markdown.contains("## Bill To"));
        assert!(markdown.contains("John Doe"));
        assert!(markdown.contains("Invoice 123"));
        assert!(markdown.contains("Date 2025"));
        assert_eq!(
            assembled.prebuilt_pages.as_ref().unwrap()[0]
                .layout_regions
                .as_ref()
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(alphanumeric_token_retention(&markdown, canonical), 1.0);
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_interleave_unmatched_ocr_with_structural_regions() {
        let detections = vec![
            crate::layout::LayoutDetection::new(
                crate::layout::LayoutClass::Title,
                0.98,
                crate::layout::BBox::new(0.0, 30.0, 100.0, 55.0),
            ),
            crate::layout::LayoutDetection::new(
                crate::layout::LayoutClass::SectionHeader,
                0.96,
                crate::layout::BBox::new(0.0, 70.0, 100.0, 95.0),
            ),
        ];
        let elements = vec![
            positioned_word("Before", 150, 5),
            positioned_word("Title", 10, 35),
            positioned_word("Between", 150, 60),
            positioned_word("Section", 10, 75),
            positioned_word("After", 150, 100),
        ];
        let canonical = "Before Title Between Section After";
        let whole = whole_image_doc_with_elements(canonical, elements, 240, 130);

        let assembled = try_assemble_cached_layout_document(&whole, &detections, &[], 240, 130)
            .expect("populated structure must retain spatially interleaved OCR");
        let texts = assembled
            .elements
            .iter()
            .map(|element| element.text.as_str())
            .collect::<Vec<_>>();

        assert_eq!(texts, vec!["Before", "Title", "Between", "Section", "After"]);
        assert_eq!(
            alphanumeric_token_retention(&crate::rendering::render_plain(&assembled), canonical),
            1.0
        );
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_retain_fallback_when_structural_detection_has_no_text() {
        let detections = vec![crate::layout::LayoutDetection::new(
            crate::layout::LayoutClass::Title,
            0.98,
            crate::layout::BBox::new(0.0, 0.0, 50.0, 30.0),
        )];
        let whole = whole_image_doc_with_elements(
            "outside title",
            vec![positioned_word("outside title", 100, 40)],
            200,
            100,
        );

        assert!(try_assemble_cached_layout_document(&whole, &detections, &[], 200, 100).is_none());
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_reject_non_table_structure_with_partial_token_retention() {
        let detections = vec![crate::layout::LayoutDetection::new(
            crate::layout::LayoutClass::Title,
            0.98,
            crate::layout::BBox::new(0.0, 0.0, 60.0, 30.0),
        )];
        let elements = vec![
            positioned_word("one", 10, 5),
            positioned_word("two", 100, 35),
            positioned_word("three", 100, 55),
            positioned_word("four", 100, 75),
        ];
        let whole = whole_image_doc_with_elements("one two three four five", elements, 200, 100);

        assert!(try_assemble_cached_layout_document(&whole, &detections, &[], 200, 100).is_none());
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_reject_table_structure_at_exact_four_of_five_token_retention() {
        let table_bbox = crate::layout::BBox::new(0.0, 0.0, 100.0, 120.0);
        let detections = vec![crate::layout::LayoutDetection::new(
            crate::layout::LayoutClass::Table,
            0.98,
            table_bbox,
        )];
        let elements = vec![
            positioned_word("one", 10, 5),
            positioned_word("two", 10, 25),
            positioned_word("three", 10, 45),
            positioned_word("four", 10, 65),
            positioned_word("five", 10, 85),
        ];
        let whole = whole_image_doc_with_elements("one two three four five", elements, 100, 120);
        let recognized = vec![crate::RecognizedTable {
            detection_bbox: table_bbox,
            cells: vec![
                vec!["one".to_string(), "two".to_string()],
                vec!["three".to_string(), "four".to_string()],
            ],
            markdown: "| one | two |\n| --- | --- |\n| three | four |".to_string(),
        }];

        let cached_elements = cached_layout_elements(&whole, 100, 120).unwrap();
        let assembled = assemble_cached_layout_elements(&whole, &detections, &recognized, &cached_elements, 100, 120);
        let retention = alphanumeric_token_retention(
            &crate::rendering::render_plain(&assembled),
            &internal_document_text(&whole),
        );
        assert_eq!(retention, MIN_LAYOUT_OCR_ALPHANUMERIC_TOKEN_RETENTION);

        assert!(try_assemble_cached_layout_document(&whole, &detections, &recognized, 100, 120).is_none());
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_keep_line_elements_for_non_table_text_with_recognized_table() {
        let table_bbox = crate::layout::BBox::new(0.0, 0.0, 100.0, 60.0);
        let detections = vec![crate::layout::LayoutDetection::new(
            crate::layout::LayoutClass::Table,
            0.98,
            table_bbox,
        )];
        let elements = vec![
            positioned_line_box("Header", 10, 10, 50, 20),
            positioned_line_box("Outside", 120, 10, 70, 20),
        ];
        let whole = whole_image_doc_with_elements("Header Outside", elements, 200, 100);
        let recognized = vec![crate::RecognizedTable {
            detection_bbox: table_bbox,
            cells: vec![vec!["Header".to_string()]],
            markdown: "| Header |\n| --- |".to_string(),
        }];

        let assembled = try_assemble_cached_layout_document(&whole, &detections, &recognized, 200, 100)
            .expect("line geometry must preserve canonical text outside the recognized table");

        assert_eq!(
            alphanumeric_token_retention(&crate::rendering::render_plain(&assembled), "Header Outside"),
            1.0
        );
        assert_eq!(
            crate::rendering::render_markdown(&assembled).matches("Header").count(),
            1
        );
        assert_eq!(
            crate::rendering::render_markdown(&assembled).matches("Outside").count(),
            1
        );
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_assemble_recognized_table_without_duplicate_fallback_text() {
        let table_bbox = crate::layout::BBox::new(0.0, 0.0, 100.0, 100.0);
        let detections = vec![
            crate::layout::LayoutDetection::new(crate::layout::LayoutClass::Table, 0.98, table_bbox),
            crate::layout::LayoutDetection::new(
                crate::layout::LayoutClass::Text,
                0.95,
                crate::layout::BBox::new(100.0, 0.0, 200.0, 100.0),
            ),
        ];
        let elements = vec![
            positioned_word_box("Header", 90, 10, 20, 20),
            positioned_word_box("Value", 50, 10, 30, 20),
            positioned_word_box("Total", 120, 10, 40, 20),
        ];
        let whole = whole_image_doc_with_elements("Header Value Total", elements, 200, 100);
        let recognized = vec![crate::RecognizedTable {
            detection_bbox: table_bbox,
            cells: vec![
                vec!["Header".to_string(), "Value".to_string()],
                vec!["A".to_string(), "1".to_string()],
            ],
            markdown: "| Header | Value |\n| --- | --- |\n| A | 1 |".to_string(),
        }];

        let assembled = try_assemble_cached_layout_document(&whole, &detections, &recognized, 200, 100)
            .expect("successful recognition must assemble a structured image table");
        let markdown = crate::rendering::render_markdown(&assembled);

        assert_eq!(assembled.tables.len(), 1);
        assert_eq!(assembled.tables[0].cells[1], vec!["A".to_string(), "1".to_string()]);
        assert_eq!(
            markdown.matches("Header").count(),
            1,
            "table OCR text must not be duplicated"
        );
        assert!(markdown.contains("Total"), "non-table OCR text must be retained");
        assert_eq!(
            serde_json::to_value(&assembled.prebuilt_ocr_elements).unwrap(),
            serde_json::to_value(&whole.prebuilt_ocr_elements).unwrap()
        );
        assert_eq!(assembled.prebuilt_pages.as_ref().unwrap()[0].tables.len(), 1);
    }

    /// Issue #181: a TATR-recognized table assembled from cached layout must carry
    /// a deterministic `table_id`, `columns`, and `bounding_box` derived from the
    /// detection's `detection_bbox` — not `..Default::default()` blanks.
    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn recognized_table_gets_table_id_columns_and_bounding_box() {
        let table_bbox = crate::layout::BBox::new(0.0, 0.0, 100.0, 100.0);
        let detections = vec![
            crate::layout::LayoutDetection::new(crate::layout::LayoutClass::Table, 0.98, table_bbox),
            crate::layout::LayoutDetection::new(
                crate::layout::LayoutClass::Text,
                0.95,
                crate::layout::BBox::new(100.0, 0.0, 200.0, 100.0),
            ),
        ];
        let elements = vec![
            positioned_word_box("Header", 90, 10, 20, 20),
            positioned_word_box("Value", 50, 10, 30, 20),
            positioned_word_box("Total", 120, 10, 40, 20),
        ];
        let whole = whole_image_doc_with_elements("Header Value Total", elements, 200, 100);
        let recognized = vec![crate::RecognizedTable {
            detection_bbox: table_bbox,
            cells: vec![
                vec!["Header".to_string(), "Value".to_string()],
                vec!["A".to_string(), "1".to_string()],
            ],
            markdown: "| Header | Value |\n| --- | --- |\n| A | 1 |".to_string(),
        }];

        let assembled = try_assemble_cached_layout_document(&whole, &detections, &recognized, 200, 100)
            .expect("successful recognition must assemble a structured image table");

        assert_eq!(assembled.tables.len(), 1);
        assert_eq!(assembled.tables[0].table_id.as_deref(), Some("table-1"));
        assert_eq!(
            assembled.tables[0].columns,
            Some(vec!["Header".to_string(), "Value".to_string()])
        );
        let bbox = assembled.tables[0]
            .bounding_box
            .expect("bounding box must be populated from detection_bbox");
        assert_eq!(bbox.x0, 0.0);
        assert_eq!(bbox.y0, 0.0);
        assert_eq!(bbox.x1, 100.0);
        assert_eq!(bbox.y1, 100.0);
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_reject_recognized_table_when_it_drops_cached_ocr_text() {
        let table_bbox = crate::layout::BBox::new(0.0, 0.0, 100.0, 100.0);
        let detections = vec![crate::layout::LayoutDetection::new(
            crate::layout::LayoutClass::Table,
            0.98,
            table_bbox,
        )];
        let elements = vec![
            positioned_word("one", 10, 10),
            positioned_word("two", 10, 25),
            positioned_word("three", 10, 40),
            positioned_word("four", 10, 55),
            positioned_word("five", 10, 70),
        ];
        let whole = whole_image_doc_with_elements("one two three four five", elements, 100, 100);
        let recognized = vec![crate::RecognizedTable {
            detection_bbox: table_bbox,
            cells: vec![vec!["one".to_string()]],
            markdown: "| one |\n| --- |".to_string(),
        }];

        assert!(try_assemble_cached_layout_document(&whole, &detections, &recognized, 100, 100).is_none());
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_preserve_existing_fallback_when_no_table_was_recognized() {
        let detections = vec![crate::layout::LayoutDetection::new(
            crate::layout::LayoutClass::Table,
            0.98,
            crate::layout::BBox::new(0.0, 0.0, 100.0, 100.0),
        )];
        let whole = whole_image_doc_with_elements(
            "unstructured fallback",
            vec![positioned_word("unstructured", 10, 10)],
            100,
            100,
        );

        assert!(try_assemble_cached_layout_document(&whole, &detections, &[], 100, 100).is_none());
    }

    #[cfg(all(feature = "layout-detection", feature = "pdf", feature = "ocr"))]
    #[test]
    fn should_respect_disabled_and_preserve_slanet_fallback_for_image_tables() {
        use crate::core::config::layout::TableModel;

        assert!(uses_tatr_image_table_recognition(TableModel::Tatr));
        assert!(!uses_tatr_image_table_recognition(TableModel::Disabled));
        assert!(!uses_tatr_image_table_recognition(TableModel::SlanetWired));
        assert!(!uses_tatr_image_table_recognition(TableModel::SlanetWireless));
        assert!(!uses_tatr_image_table_recognition(TableModel::SlanetPlus));
        assert!(!uses_tatr_image_table_recognition(TableModel::SlanetAuto));
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_reject_multiframe_whole_image_fast_path() {
        let detections = vec![crate::layout::LayoutDetection::new(
            crate::layout::LayoutClass::Text,
            0.96,
            crate::layout::BBox::new(0.0, 0.0, 20.0, 20.0),
        )];
        let elements = vec![positioned_word("outside", 100, 20)];
        let mut whole = whole_image_doc_with_elements("outside", elements, 200, 100);
        let mut second_page = whole.prebuilt_pages.as_ref().unwrap()[0].clone();
        second_page.page_number = 2;
        whole.prebuilt_pages.as_mut().unwrap().push(second_page);

        assert!(try_retain_canonical_whole_image_ocr(&whole, &detections, 200, 100, true).is_none());
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_reject_real_multiframe_tiff_when_ocr_synthesizes_one_page() {
        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut encoder = tiff::encoder::TiffEncoder::new(&mut cursor).unwrap();
            encoder
                .write_image::<tiff::encoder::colortype::Gray8>(1, 1, &[0])
                .unwrap();
            encoder
                .write_image::<tiff::encoder::colortype::Gray8>(1, 1, &[255])
                .unwrap();
        }
        let source = cursor.into_inner();
        let decoder = tiff::decoder::Decoder::new(std::io::Cursor::new(&source)).unwrap();
        assert!(decoder.more_images(), "test input must contain multiple TIFF frames");

        let detections = vec![crate::layout::LayoutDetection::new(
            crate::layout::LayoutClass::Text,
            0.96,
            crate::layout::BBox::new(0.0, 0.0, 20.0, 20.0),
        )];
        let elements = vec![positioned_word("outside", 100, 20)];
        let whole = whole_image_doc_with_elements("outside", elements, 200, 100);
        let source_is_single_frame = source_image_is_proven_single_frame(&source, "image/tiff");

        assert!(!source_is_single_frame);
        assert!(try_retain_canonical_whole_image_ocr(&whole, &detections, 200, 100, source_is_single_frame).is_none());
    }

    /// Build an in-memory, single-pixel-per-frame TIFF with `frame_count` frames.
    #[cfg(feature = "ocr")]
    fn build_multiframe_tiff(frame_count: usize) -> Vec<u8> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut encoder = tiff::encoder::TiffEncoder::new(&mut cursor).unwrap();
            for i in 0..frame_count {
                encoder
                    .write_image::<tiff::encoder::colortype::Gray8>(1, 1, &[i as u8])
                    .unwrap();
            }
        }
        cursor.into_inner()
    }

    /// #1451: `max_pages` must reject a multi-frame TIFF once its frame count is
    /// known, before per-frame OCR begins. Against unfixed code
    /// `enforce_image_page_limit` does not exist at all, so this fails to compile;
    /// once it exists but does not check frame count, it returns `Ok(())` for a
    /// 3-frame TIFF under a configured limit of 2 instead of the expected
    /// `SecurityError::TooManyPages`.
    #[cfg(feature = "ocr")]
    #[test]
    fn should_reject_tiff_exceeding_max_pages() {
        let tiff_bytes = build_multiframe_tiff(3);
        let config = ExtractionConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_pages: Some(2),
                ..Default::default()
            }),
            ..Default::default()
        };

        let error = enforce_image_page_limit(&tiff_bytes, "image/tiff", &config)
            .expect_err("a TIFF with more frames than max_pages must be rejected");
        let message = error.to_string();
        assert!(
            message.contains("too many pages") || message.contains("max_pages"),
            "error must name the limit that was hit: {message}"
        );
    }

    /// A TIFF exactly at the configured `max_pages` ceiling must not be rejected --
    /// the off-by-one boundary case #1451 asked to get right.
    #[cfg(feature = "ocr")]
    #[test]
    fn should_accept_tiff_at_max_pages_boundary() {
        let tiff_bytes = build_multiframe_tiff(3);
        let config = ExtractionConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_pages: Some(3),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert!(
            enforce_image_page_limit(&tiff_bytes, "image/tiff", &config).is_ok(),
            "a TIFF exactly at max_pages must not be rejected"
        );
    }

    /// The default `SecurityLimits` (`max_pages: None`) must never reject a
    /// multi-frame TIFF: a real ceiling here is opt-in, not implied by #1451.
    #[cfg(feature = "ocr")]
    #[test]
    fn should_accept_tiff_under_default_max_pages() {
        let tiff_bytes = build_multiframe_tiff(3);
        let config = ExtractionConfig::default();

        assert!(
            enforce_image_page_limit(&tiff_bytes, "image/tiff", &config).is_ok(),
            "default security limits (max_pages: None) must not reject a multi-frame TIFF"
        );
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_clip_layout_regions_to_image_bounds_and_drop_invalid_boxes() {
        let detections = vec![
            crate::layout::LayoutDetection::new(
                crate::layout::LayoutClass::Text,
                0.96,
                crate::layout::BBox::new(-20.0, -10.0, 220.0, 110.0),
            ),
            crate::layout::LayoutDetection::new(
                crate::layout::LayoutClass::Text,
                0.80,
                crate::layout::BBox::new(f32::NAN, 0.0, 10.0, 10.0),
            ),
            crate::layout::LayoutDetection::new(
                crate::layout::LayoutClass::Text,
                0.70,
                crate::layout::BBox::new(30.0, 30.0, 20.0, 20.0),
            ),
        ];

        let regions = layout_regions_from_detections(&detections, 200, 100);

        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].bounding_box.x0, 0.0);
        assert_eq!(regions[0].bounding_box.y0, 0.0);
        assert_eq!(regions[0].bounding_box.x1, 200.0);
        assert_eq!(regions[0].bounding_box.y1, 100.0);
        assert_eq!(regions[0].area_fraction, 1.0);
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_retain_cached_whole_image_when_region_ocr_fails() {
        let mut whole = image_ocr_document("cached whole-image text");
        whole.metadata.additional.insert(
            std::borrow::Cow::Borrowed("ocr_candidate"),
            serde_json::json!("whole-image"),
        );

        let retained = cached_whole_image_after_layout_error(
            &Ok(whole),
            crate::XbergError::Other("region backend failed".to_string()),
        )
        .expect("cached whole-image OCR must remain usable");

        assert_eq!(internal_document_text(&retained), "cached whole-image text");
        assert_eq!(
            retained.metadata.additional.get("ocr_candidate"),
            Some(&serde_json::json!("whole-image"))
        );
        assert_eq!(retained.processing_warnings.len(), 1);
        assert_eq!(
            retained.processing_warnings[0].message,
            "Layout-region OCR failed after whole-image OCR succeeded; retained whole-image output"
        );
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[test]
    fn should_report_both_ocr_failures() {
        let result = cached_whole_image_after_layout_error(
            &Err(crate::XbergError::Other("whole backend failed".to_string())),
            crate::XbergError::Other("region backend failed".to_string()),
        );

        let message = result
            .expect_err("both failed OCR paths must return an error")
            .to_string();
        assert!(message.contains("whole-image OCR: whole backend failed"));
        assert!(message.contains("layout-region OCR: region backend failed"));
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[tokio::test]
    async fn should_not_retry_whole_image_ocr_after_layout_path_failure() {
        let layout_calls = std::cell::Cell::new(0);
        let whole_calls = std::cell::Cell::new(0);

        let result = extract_selected_image_ocr_path(
            true,
            || {
                layout_calls.set(layout_calls.get() + 1);
                std::future::ready(Err(crate::XbergError::Other("layout path failed".to_string())))
            },
            || {
                whole_calls.set(whole_calls.get() + 1);
                std::future::ready(Ok(image_ocr_document("unexpected retry")))
            },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(layout_calls.get(), 1);
        assert_eq!(whole_calls.get(), 0);
    }

    #[cfg(all(feature = "layout-detection", feature = "ocr"))]
    #[tokio::test]
    async fn should_use_default_whole_image_ocr_when_layout_has_no_ocr_config() {
        let config = ExtractionConfig {
            layout: Some(crate::core::config::LayoutDetectionConfig::default()),
            ocr: None,
            ..Default::default()
        };
        let layout_calls = std::cell::Cell::new(0);
        let whole_calls = std::cell::Cell::new(0);

        let result = extract_selected_image_ocr_path(
            should_use_layout_ocr(&config),
            || {
                layout_calls.set(layout_calls.get() + 1);
                std::future::ready(Err(crate::XbergError::Other("unexpected layout path".to_string())))
            },
            || {
                whole_calls.set(whole_calls.get() + 1);
                std::future::ready(Ok(image_ocr_document("default whole-image OCR")))
            },
        )
        .await
        .expect("missing explicit OCR config must retain the default whole-image path");

        assert_eq!(internal_document_text(&result), "default whole-image OCR");
        assert_eq!(layout_calls.get(), 0);
        assert_eq!(whole_calls.get(), 1);
    }

    #[test]
    fn should_use_whole_image_ocr_when_invoice_regions_drop_fields() {
        let layout = image_ocr_document("Invoice 1042 Acme Total 1250 USD");
        let mut whole = image_ocr_document("Invoice 1042 Acme Corporation Date July 31 Total 1250 USD");
        whole.metadata.additional.insert(
            std::borrow::Cow::Borrowed("ocr_candidate"),
            serde_json::Value::String("whole-image".to_string()),
        );
        whole.processing_warnings.push(crate::types::ProcessingWarning {
            source: std::borrow::Cow::Borrowed("ocr"),
            message: std::borrow::Cow::Borrowed("whole-image warning"),
        });

        let selected = select_image_ocr_result(layout, Ok(whole));

        assert_eq!(
            internal_document_text(&selected),
            "Invoice 1042 Acme Corporation Date July 31 Total 1250 USD"
        );
        assert_eq!(
            selected.metadata.additional.get("ocr_candidate"),
            Some(&serde_json::Value::String("whole-image".to_string()))
        );
        assert_eq!(selected.processing_warnings.len(), 1);
        assert_eq!(selected.processing_warnings[0].message, "whole-image warning");
    }

    #[test]
    fn should_keep_layout_ocr_for_complete_simple_line() {
        let mut layout = image_ocr_document("The quick brown fox jumps over the lazy dog");
        layout.metadata.additional.insert(
            std::borrow::Cow::Borrowed("ocr_candidate"),
            serde_json::Value::String("layout".to_string()),
        );
        let whole = image_ocr_document("The quick brown fox jumps over the lazy dog");

        let selected = select_image_ocr_result(layout, Ok(whole));

        assert_eq!(
            selected.metadata.additional.get("ocr_candidate"),
            Some(&serde_json::Value::String("layout".to_string()))
        );
    }

    #[cfg(feature = "quality")]
    #[test]
    fn should_use_whole_image_ocr_when_complex_layout_text_scores_lower() {
        let layout =
            image_ocr_document("Quarterly   revenue   increased   while   operating   expenses   remained   stable.");
        let whole = image_ocr_document("Quarterly revenue increased while operating expenses remained stable.");

        assert_eq!(
            alphanumeric_token_retention(&internal_document_text(&layout), &internal_document_text(&whole)),
            1.0
        );
        let selected = select_image_ocr_result(layout, Ok(whole));

        assert_eq!(
            internal_document_text(&selected),
            "Quarterly revenue increased while operating expenses remained stable."
        );
    }

    #[test]
    fn should_count_duplicate_alphanumeric_tokens_individually() {
        let retention = alphanumeric_token_retention("invoice total", "invoice invoice total");

        assert!((retention - (2.0 / 3.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn should_accept_exact_minimum_alphanumeric_token_retention() {
        let retention = alphanumeric_token_retention("one two three four", "one two three four five");

        assert!((retention - MIN_LAYOUT_OCR_ALPHANUMERIC_TOKEN_RETENTION).abs() < f64::EPSILON);
    }

    #[test]
    fn should_keep_layout_ocr_when_whole_image_comparison_fails() {
        let mut layout = image_ocr_document("Invoice 1042 Total 1250 USD");
        layout.metadata.additional.insert(
            std::borrow::Cow::Borrowed("ocr_candidate"),
            serde_json::Value::String("layout".to_string()),
        );
        layout.processing_warnings.push(crate::types::ProcessingWarning {
            source: std::borrow::Cow::Borrowed("layout-ocr"),
            message: std::borrow::Cow::Borrowed("layout warning"),
        });

        let selected = select_image_ocr_result(
            layout,
            Err(crate::XbergError::Other("comparison OCR failed".to_string())),
        );

        assert_eq!(
            selected.metadata.additional.get("ocr_candidate"),
            Some(&serde_json::Value::String("layout".to_string()))
        );
        assert_eq!(selected.processing_warnings.len(), 1);
        assert_eq!(selected.processing_warnings[0].message, "layout warning");
    }

    /// Regression test for #705: a backend that gates ocr_elements on include_elements
    /// (e.g. paddle-ocr) must still produce non-empty pages[].
    ///
    /// extract_with_ocr forces include_elements=true before calling the backend so that
    /// elements are available; prebuilt_pages is then set from the HOCR content string,
    /// ensuring pages[] is populated regardless of the original config.
    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_extract_with_ocr_populates_pages_for_elements_gated_backend() {
        use crate::core::config::OcrConfig;
        use crate::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
        use crate::types::{
            ExtractedDocument, OcrBoundingGeometry, OcrConfidence, OcrElement, OcrElementConfig, OcrElementLevel,
        };

        let mut png_buf = std::io::Cursor::new(Vec::new());
        image::ImageBuffer::<image::Rgb<u8>, _>::from_pixel(1, 1, image::Rgb([255u8, 255, 255]))
            .write_to(&mut png_buf, image::ImageFormat::Png)
            .expect("failed to encode test PNG");
        let png_1x1 = png_buf.into_inner();

        /// A mock backend that behaves like paddle-ocr: returns ocr_elements only when
        /// include_elements is true. This is the exact contract that caused issue #705.
        struct GatedElementsBackend;

        #[async_trait::async_trait]
        impl OcrBackend for GatedElementsBackend {
            fn backend_type(&self) -> OcrBackendType {
                OcrBackendType::Custom
            }
            fn supports_language(&self, _: &str) -> bool {
                true
            }
            async fn process_image(&self, _: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
                let include_elements = config.element_config.as_ref().is_some_and(|ec| ec.include_elements);

                let elements = if include_elements {
                    let element = |text: &str, level: OcrElementLevel, confidence: f64, left: u32, width: u32| {
                        OcrElement::new(
                            text.to_string(),
                            OcrBoundingGeometry::Rectangle {
                                left,
                                top: 0,
                                width,
                                height: 20,
                            },
                            OcrConfidence::from_tesseract(confidence),
                        )
                        .with_level(level)
                        .with_page_number(1)
                    };
                    Some(vec![
                        element("hello world", OcrElementLevel::Line, 99.0, 0, 100),
                        element("hello", OcrElementLevel::Word, 95.0, 5, 30),
                        element("noise", OcrElementLevel::Word, 20.0, 60, 30),
                    ])
                } else {
                    None
                };

                Ok(ExtractedDocument {
                    content: "hello world".to_string(),
                    ocr_elements: elements,
                    ..Default::default()
                })
            }
        }

        impl Plugin for GatedElementsBackend {
            fn name(&self) -> &str {
                "gated-elements-test"
            }
            fn version(&self) -> String {
                "0.0.0".to_string()
            }
            fn initialize(&self) -> crate::Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> crate::Result<()> {
                Ok(())
            }
        }

        register_ocr_backend(std::sync::Arc::new(GatedElementsBackend)).unwrap();

        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "gated-elements-test".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let extractor = ImageExtractor::new();
        let internal_doc = extractor.extract_content(&png_1x1, "image/png", &config).await.unwrap();

        let result = crate::extraction::derive::derive_extraction_result(
            internal_doc,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        assert!(
            result.metadata.ocr_used,
            "successful image OCR must be reflected in metadata"
        );
        assert_eq!(result.extraction_method, Some(crate::types::ExtractionMethod::Ocr));
        assert!(
            result.ocr_elements.is_none(),
            "elements forced for internal page assembly must remain private unless requested"
        );
        let pages = result
            .pages
            .as_ref()
            .expect("pages must be populated (regression of #705)");
        assert!(!pages.is_empty(), "pages[] must not be empty (regression of #705)");
        assert_eq!(pages[0].content.trim(), "hello world");

        let explicitly_disabled = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "gated-elements-test".to_string(),
                element_config: Some(OcrElementConfig::default()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let disabled_doc = extractor
            .extract_content(&png_1x1, "image/png", &explicitly_disabled)
            .await
            .unwrap();
        assert!(disabled_doc.prebuilt_ocr_elements.is_none());

        let filtered_config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "gated-elements-test".to_string(),
                element_config: Some(OcrElementConfig {
                    include_elements: true,
                    min_level: OcrElementLevel::Word,
                    min_confidence: 0.8,
                    build_hierarchy: true,
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let filtered_doc = extractor
            .extract_content(&png_1x1, "image/png", &filtered_config)
            .await
            .unwrap();
        let filtered = filtered_doc
            .prebuilt_ocr_elements
            .expect("requested custom-backend elements must be published");
        assert_eq!(
            filtered.iter().map(|element| element.text.as_str()).collect::<Vec<_>>(),
            ["hello world", "hello"]
        );
        let line = filtered.iter().find(|element| element.text == "hello world").unwrap();
        let word = filtered.iter().find(|element| element.text == "hello").unwrap();
        assert!(line.parent_id.is_none());
        assert!(word.parent_id.is_some());

        unregister_ocr_backend("gated-elements-test").unwrap();
    }

    /// Regression test for #706: pages[0].content must be the coherent HOCR-rendered
    /// text, not a word-by-word dump assembled from raw OcrText elements.
    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_extract_with_ocr_page_content_matches_top_level_content() {
        use crate::core::config::OcrConfig;
        use crate::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
        use crate::types::{ExtractedDocument, OcrBoundingGeometry, OcrConfidence, OcrElement, OcrElementLevel};

        let mut png_buf = std::io::Cursor::new(Vec::new());
        image::ImageBuffer::<image::Rgb<u8>, _>::from_pixel(1, 1, image::Rgb([255u8, 255, 255]))
            .write_to(&mut png_buf, image::ImageFormat::Png)
            .expect("failed to encode test PNG");
        let png_1x1 = png_buf.into_inner();

        const COHERENT: &str = "Sales Report 2024\n\nThis report contains quarterly sales data.";

        struct TesseractLikeBackend;

        #[async_trait::async_trait]
        impl OcrBackend for TesseractLikeBackend {
            fn backend_type(&self) -> OcrBackendType {
                OcrBackendType::Custom
            }
            fn supports_language(&self, _: &str) -> bool {
                true
            }
            async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
                let content = COHERENT.to_string();
                let words = [
                    "Sales",
                    "Report",
                    "2024",
                    "This",
                    "report",
                    "contains",
                    "quarterly",
                    "sales",
                    "data.",
                ];
                let mut elements = Vec::new();
                for (i, word) in words.iter().enumerate() {
                    let geo = OcrBoundingGeometry::Rectangle {
                        left: i as u32 * 60,
                        top: 0,
                        width: 50,
                        height: 20,
                    };
                    let elem = OcrElement::new(word.to_string(), geo, OcrConfidence::from_tesseract(99.0))
                        .with_level(OcrElementLevel::Word)
                        .with_page_number(1);
                    elements.push(elem);
                }
                Ok(ExtractedDocument {
                    content,
                    ocr_elements: Some(elements),
                    ..Default::default()
                })
            }
        }

        impl Plugin for TesseractLikeBackend {
            fn name(&self) -> &str {
                "tesseract-like-706"
            }
            fn version(&self) -> String {
                "0.0.0".to_string()
            }
            fn initialize(&self) -> crate::Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> crate::Result<()> {
                Ok(())
            }
        }

        register_ocr_backend(std::sync::Arc::new(TesseractLikeBackend)).unwrap();

        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "tesseract-like-706".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let extractor = ImageExtractor::new();
        let internal_doc = extractor.extract_content(&png_1x1, "image/png", &config).await.unwrap();

        let result = crate::extraction::derive::derive_extraction_result(
            internal_doc,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        assert_eq!(result.content.trim(), COHERENT, "top-level content mismatch");

        let pages = result
            .pages
            .as_ref()
            .expect("pages must be populated (regression of #706)");
        assert!(!pages.is_empty(), "pages must not be empty");
        assert_eq!(
            pages[0].content.trim(),
            COHERENT,
            "pages[0].content is a word-by-word dump instead of coherent text (regression of #706)"
        );

        unregister_ocr_backend("tesseract-like-706").unwrap();
    }

    #[tokio::test]
    async fn test_image_extractor_invalid_image() {
        let extractor = ImageExtractor::new();
        let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
        let config = ExtractionConfig::default();

        let result = extractor.extract_content(&invalid_bytes, "image/png", &config).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_image_plugin_interface() {
        let extractor = ImageExtractor::new();
        assert_eq!(extractor.name(), "image-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert!(extractor.supported_mime_types().contains(&"image/png"));
        assert!(extractor.supported_mime_types().contains(&"image/jpeg"));
        assert!(extractor.supported_mime_types().contains(&"image/webp"));
        assert_eq!(extractor.priority(), 50);
    }

    #[test]
    fn test_image_extractor_default() {
        let extractor = ImageExtractor;
        assert_eq!(extractor.name(), "image-extractor");
    }

    #[test]
    fn test_image_extractor_supports_alias_mime_types() {
        let extractor = ImageExtractor::new();
        let supported = extractor.supported_mime_types();
        assert!(supported.contains(&"image/pjpeg"));
        assert!(supported.contains(&"image/x-bmp"));
        assert!(supported.contains(&"image/x-ms-bmp"));
        assert!(supported.contains(&"image/x-tiff"));
        assert!(supported.contains(&"image/x-portable-anymap"));
    }

    /// Regression test for #732: when captioning is configured and OCR runs,
    /// doc.images must contain the raw image bytes so CaptioningProcessor can act.
    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_extract_with_ocr_populates_images_for_captioning() {
        use crate::core::config::{CaptioningConfig, LlmConfig, OcrConfig};
        use crate::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
        use crate::types::ExtractedDocument;

        let mut png_buf = std::io::Cursor::new(Vec::new());
        image::ImageBuffer::<image::Rgb<u8>, _>::from_pixel(1, 1, image::Rgb([255u8, 255, 255]))
            .write_to(&mut png_buf, image::ImageFormat::Png)
            .expect("failed to encode test PNG");
        let png_1x1 = png_buf.into_inner();

        /// A minimal mock OCR backend that returns empty text.
        struct EmptyOcrBackend;

        #[async_trait::async_trait]
        impl OcrBackend for EmptyOcrBackend {
            fn backend_type(&self) -> OcrBackendType {
                OcrBackendType::Custom
            }
            fn supports_language(&self, _: &str) -> bool {
                true
            }
            async fn process_image(&self, _: &[u8], _config: &OcrConfig) -> crate::Result<ExtractedDocument> {
                Ok(ExtractedDocument {
                    content: String::new(),
                    ..Default::default()
                })
            }
        }

        impl Plugin for EmptyOcrBackend {
            fn name(&self) -> &str {
                "empty-ocr-732"
            }
            fn version(&self) -> String {
                "0.0.0".to_string()
            }
            fn initialize(&self) -> crate::Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> crate::Result<()> {
                Ok(())
            }
        }

        register_ocr_backend(std::sync::Arc::new(EmptyOcrBackend)).unwrap();
        struct BackendGuard(&'static str);
        impl Drop for BackendGuard {
            fn drop(&mut self) {
                let _ = unregister_ocr_backend(self.0);
            }
        }
        let _guard = BackendGuard("empty-ocr-732");

        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "empty-ocr-732".to_string(),
                ..Default::default()
            }),
            captioning: Some(CaptioningConfig {
                llm: LlmConfig {
                    model: "openai/gpt-4o-mini".to_string(),
                    ..Default::default()
                },
                prompt: None,
                min_image_area: 0,
                alt_text: Default::default(),
            }),
            ..Default::default()
        };

        let extractor = ImageExtractor::new();
        let doc = extractor.extract_content(&png_1x1, "image/png", &config).await.unwrap();

        assert_eq!(
            doc.images.len(),
            1,
            "doc.images must contain the raw image (regression of #732)"
        );
        assert!(!doc.images[0].data.is_empty(), "image data must be non-empty");
    }

    /// REV-C6 regression for the #1662 review: OCR alone must NOT put a standalone
    /// image's own bytes in the result. `needs_image_data` gained an OCR disjunct so
    /// containers would read embedded images for OCR (#1662); `wants_own_bytes_in_result`
    /// is the narrower predicate `extractors/image.rs` uses instead, so a standalone
    /// image's public `images` output stays exactly as it was before #1662 unless the
    /// caller also asked for image extraction, captioning, or QR codes. `doc.images` must
    /// stay empty here, in contrast to
    /// `test_extract_with_ocr_populates_images_for_captioning` immediately above, whose
    /// only difference is a `captioning` config.
    #[tokio::test]
    async fn test_extract_with_ocr_only_does_not_populate_images() {
        use crate::core::config::OcrConfig;
        use crate::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
        use crate::types::ExtractedDocument;

        let mut png_buf = std::io::Cursor::new(Vec::new());
        image::ImageBuffer::<image::Rgb<u8>, _>::from_pixel(1, 1, image::Rgb([255u8, 255, 255]))
            .write_to(&mut png_buf, image::ImageFormat::Png)
            .expect("failed to encode test PNG");
        let png_1x1 = png_buf.into_inner();

        struct EmptyOcrBackend1662;

        #[async_trait::async_trait]
        impl OcrBackend for EmptyOcrBackend1662 {
            fn backend_type(&self) -> OcrBackendType {
                OcrBackendType::Custom
            }
            fn supports_language(&self, _: &str) -> bool {
                true
            }
            async fn process_image(&self, _: &[u8], _config: &OcrConfig) -> crate::Result<ExtractedDocument> {
                Ok(ExtractedDocument {
                    content: String::new(),
                    ..Default::default()
                })
            }
        }

        impl Plugin for EmptyOcrBackend1662 {
            fn name(&self) -> &str {
                "empty-ocr-1662"
            }
            fn version(&self) -> String {
                "0.0.0".to_string()
            }
            fn initialize(&self) -> crate::Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> crate::Result<()> {
                Ok(())
            }
        }

        register_ocr_backend(std::sync::Arc::new(EmptyOcrBackend1662)).unwrap();
        struct BackendGuard(&'static str);
        impl Drop for BackendGuard {
            fn drop(&mut self) {
                let _ = unregister_ocr_backend(self.0);
            }
        }
        let _guard = BackendGuard("empty-ocr-1662");

        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "empty-ocr-1662".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let extractor = ImageExtractor::new();
        let doc = extractor.extract_content(&png_1x1, "image/png", &config).await.unwrap();

        assert!(
            doc.images.is_empty(),
            "OCR alone must not echo a standalone image's bytes into the public result: {:?}",
            doc.images
        );
    }

    /// Full-pipeline regression for #732: InternalDocument.images must survive the
    /// derive.rs conversion so ExtractedDocument.images is Some after run_pipeline.
    // Holds a `ProcessorSnapshotLease` for the whole `run_pipeline` call. Without `#[serial]`
    // this raced the `#[serial]`-guarded lifecycle tests in `core::pipeline::tests`, whose
    // mutations are refused while any snapshot lease is live. ~keep
    #[cfg(feature = "ocr")]
    #[tokio::test]
    #[serial_test::serial]
    async fn test_pipeline_images_some_after_ocr_with_captioning() {
        use crate::core::config::{CaptioningConfig, LlmConfig, OcrConfig};
        use crate::core::pipeline::run_pipeline;
        use crate::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
        use crate::types::ExtractedDocument;

        let mut png_buf = std::io::Cursor::new(Vec::new());
        image::ImageBuffer::<image::Rgb<u8>, _>::from_pixel(1, 1, image::Rgb([255u8, 255, 255]))
            .write_to(&mut png_buf, image::ImageFormat::Png)
            .expect("failed to encode test PNG");
        let png_1x1 = png_buf.into_inner();

        struct EmptyOcrBackend732b;

        #[async_trait::async_trait]
        impl OcrBackend for EmptyOcrBackend732b {
            fn backend_type(&self) -> OcrBackendType {
                OcrBackendType::Custom
            }
            fn supports_language(&self, _: &str) -> bool {
                true
            }
            async fn process_image(&self, _: &[u8], _config: &OcrConfig) -> crate::Result<ExtractedDocument> {
                Ok(ExtractedDocument {
                    content: String::new(),
                    ..Default::default()
                })
            }
        }
        impl Plugin for EmptyOcrBackend732b {
            fn name(&self) -> &str {
                "empty-ocr-732b"
            }
            fn version(&self) -> String {
                "0.0.0".to_string()
            }
            fn initialize(&self) -> crate::Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> crate::Result<()> {
                Ok(())
            }
        }

        register_ocr_backend(std::sync::Arc::new(EmptyOcrBackend732b)).unwrap();
        struct BackendGuard(&'static str);
        impl Drop for BackendGuard {
            fn drop(&mut self) {
                let _ = unregister_ocr_backend(self.0);
            }
        }
        let _guard = BackendGuard("empty-ocr-732b");

        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "empty-ocr-732b".to_string(),
                ..Default::default()
            }),
            captioning: Some(CaptioningConfig {
                llm: LlmConfig {
                    model: "openai/gpt-4o-mini".to_string(),
                    ..Default::default()
                },
                prompt: None,
                min_image_area: u32::MAX,
                alt_text: Default::default(),
            }),
            ..Default::default()
        };

        let extractor = ImageExtractor::new();
        let doc = extractor.extract_content(&png_1x1, "image/png", &config).await.unwrap();
        assert_eq!(doc.images.len(), 1, "InternalDocument must have image before pipeline");

        let result = run_pipeline(doc, &config).await.unwrap();
        assert!(
            result.images.is_some(),
            "ExtractedDocument.images must be Some after pipeline — regression of #732"
        );
        assert_eq!(
            result.images.unwrap().len(),
            1,
            "image count must survive the derive.rs conversion"
        );
    }

    /// Regression test for task #709: `extract_layout_regions` must stop dispatching
    /// OCR calls for further layout regions once cancellation is signalled, instead of
    /// working through every remaining region regardless.
    ///
    /// Proves the checkpoint actually stops work (not merely that an error comes back):
    /// a counting backend records how many regions it was actually asked to OCR. With a
    /// token cancelled *before* the call, the loop's first-iteration check must break
    /// before the backend is invoked for *any* region, so the counter stays at 0 out of
    /// 3 regions — against code with the checkpoint removed, this backend is invoked for
    /// all 3 regions regardless of cancellation, so the counter would be 3, not 0.
    #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
    #[tokio::test]
    async fn extract_layout_regions_stops_dispatching_once_cancelled() {
        use crate::cancellation::CancellationToken;
        use crate::core::config::OcrConfig;
        use crate::layout::{BBox, LayoutClass, LayoutDetection};
        use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
        use crate::types::ExtractedDocument;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        /// A backend that counts how many regions it was actually asked to OCR,
        /// standing in for a real (slow) OCR call so the test stays fast and
        /// deterministic while still proving the loop-level checkpoint's effect.
        struct CountingOcrBackend(Arc<AtomicUsize>);

        #[async_trait::async_trait]
        impl OcrBackend for CountingOcrBackend {
            fn backend_type(&self) -> OcrBackendType {
                OcrBackendType::Custom
            }
            fn supports_language(&self, _: &str) -> bool {
                true
            }
            async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(ExtractedDocument {
                    content: "region text".to_string(),
                    ..Default::default()
                })
            }
        }

        impl Plugin for CountingOcrBackend {
            fn name(&self) -> &str {
                "counting-ocr-test-709"
            }
            fn version(&self) -> String {
                "0.0.0".to_string()
            }
            fn initialize(&self) -> crate::Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> crate::Result<()> {
                Ok(())
            }
        }

        let rgb = image::RgbImage::from_pixel(100, 100, image::Rgb([255u8, 255, 255]));
        let detections = vec![
            LayoutDetection::new(LayoutClass::Text, 0.9, BBox::new(0.0, 0.0, 20.0, 20.0)),
            LayoutDetection::new(LayoutClass::Text, 0.9, BBox::new(30.0, 0.0, 50.0, 20.0)),
            LayoutDetection::new(LayoutClass::Text, 0.9, BBox::new(60.0, 0.0, 80.0, 20.0)),
        ];
        let ocr_config = OcrConfig::default();

        let calls = Arc::new(AtomicUsize::new(0));
        let backend: Arc<dyn OcrBackend> = Arc::new(CountingOcrBackend(Arc::clone(&calls)));
        let cancelled_token = CancellationToken::new();
        cancelled_token.cancel();

        let doc = extract_layout_regions(
            Arc::clone(&backend),
            &rgb,
            &detections,
            &ocr_config,
            None,
            Some(&cancelled_token),
        )
        .await
        .expect("extract_layout_regions must not error when cancelled, only skip remaining work");

        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "no region should be OCR'd once the token is already cancelled"
        );
        assert!(
            doc.processing_warnings
                .iter()
                .any(|warning| warning.message.contains("cancelled")),
            "a cancelled run should explain why regions were skipped, got {:?}",
            doc.processing_warnings
        );

        // Sanity check: the same 3 regions, uncancelled, all reach the backend — this
        // rules out the zero count above being an unrelated bug (e.g. a bbox rejected
        // by `encode_layout_region`) rather than the cancellation checkpoint.
        calls.store(0, Ordering::SeqCst);
        let _ = extract_layout_regions(Arc::clone(&backend), &rgb, &detections, &ocr_config, None, None)
            .await
            .expect("uncancelled extract_layout_regions must succeed");
        assert_eq!(
            calls.load(Ordering::SeqCst),
            3,
            "all 3 regions should be OCR'd when nothing is cancelled"
        );
    }

    // -- enforce_image_page_limit stub (not(feature = "ocr")) -------------------
    //
    // The `ocr`-enabled sibling counts real TIFF frames and is exercised
    // elsewhere; these tests cover the no-op stub compiled when the `tiff`
    // crate isn't available, which instead emits a one-shot warning so a
    // caller-set `max_pages` going unenforced is discoverable (#1451-adjacent).
    //
    // `warn_unenforced_tiff_page_limit_once` takes its "already warned" guard
    // as a parameter rather than reading `UNENFORCED_TIFF_PAGE_LIMIT_WARNED`
    // directly, mirroring `warn_default_thread_cap_once` in
    // `core::config::concurrency`: a shared process-global guard can only be
    // asserted from a single test without the assertion racing every other
    // test in the binary that reaches the same code path. Three of the four
    // tests below use a guard they own, so they are independent of each other
    // and of test execution order; only the "fires" test below exercises the
    // real global-backed entry point (`enforce_image_page_limit`), and it is
    // the only test in this binary that does so.
    #[cfg(not(feature = "ocr"))]
    mod enforce_image_page_limit_stub {
        use std::sync::atomic::AtomicBool;
        use std::sync::{Arc, Mutex};

        use tracing_subscriber::layer::SubscriberExt as _;
        use tracing_subscriber::{EnvFilter, Layer};

        use super::super::{ExtractionConfig, enforce_image_page_limit, enforce_image_page_limit_stub_with_guard};
        use crate::extractors::security::SecurityLimits;

        /// A tracing `Layer` that records the level of every emitted event.
        #[derive(Clone, Default)]
        struct EventCapture {
            levels: Arc<Mutex<Vec<tracing::Level>>>,
        }

        impl<S> Layer<S> for EventCapture
        where
            S: tracing::Subscriber,
        {
            fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
                self.levels.lock().unwrap().push(*event.metadata().level());
            }
        }

        fn warn_event_count(capture: &EventCapture) -> usize {
            capture
                .levels
                .lock()
                .unwrap()
                .iter()
                .filter(|level| **level == tracing::Level::WARN)
                .count()
        }

        fn config_with_max_pages(max_pages: usize) -> ExtractionConfig {
            ExtractionConfig {
                security_limits: Some(SecurityLimits {
                    max_pages: Some(max_pages),
                    ..Default::default()
                }),
                ..Default::default()
            }
        }

        /// Changing this to `_content.is_empty()` (as opposed to inspecting
        /// `mime_type`) would make this test pass for the wrong reason; the
        /// content bytes are irrelevant to the stub, which is the point.
        const TIFF_MIME: &str = "image/tiff";

        /// The real, process-global-backed entry point warns exactly once
        /// across repeated calls when `max_pages` is set on a TIFF. Bundled
        /// into a single test (rather than a separate "fires" and "fires
        /// once" test) because both assertions read the same global guard;
        /// splitting them would make the pass/fail outcome depend on which
        /// test the harness happens to run first.
        ///
        /// Fails if: the `if !mime_type...contains("tiff")` check in
        /// `enforce_image_page_limit_stub_with_guard` is removed or inverted;
        /// the `let Some(max_pages) = max_pages else { return Ok(()) };` guard
        /// is removed; `warn_unenforced_tiff_page_limit_once` is changed to
        /// unconditionally warn (no longer once); or the call to
        /// `warn_unenforced_tiff_page_limit_once` is deleted altogether.
        #[test]
        fn fires_exactly_once_for_tiff_with_max_pages_set_across_repeated_calls() {
            let capture = EventCapture::default();
            let subscriber = tracing_subscriber::registry()
                .with(EnvFilter::new("warn"))
                .with(capture.clone());
            let config = config_with_max_pages(3);

            tracing::subscriber::with_default(subscriber, || {
                for _ in 0..5 {
                    let result = enforce_image_page_limit(b"unused", TIFF_MIME, &config);
                    assert!(result.is_ok(), "the stub warns but never rejects extraction");
                }
            });

            assert_eq!(
                warn_event_count(&capture),
                1,
                "expected exactly one WARN across repeated calls, got {:?}",
                capture.levels.lock().unwrap()
            );
        }

        /// Fails if: the `let Some(max_pages) = max_pages else { return
        /// Ok(()) };` early return in `enforce_image_page_limit_stub_with_guard`
        /// is removed (or its condition inverted), causing an unset
        /// `max_pages` to warn anyway.
        #[test]
        fn does_not_fire_when_max_pages_is_unset() {
            let already_warned = AtomicBool::new(false);
            let capture = EventCapture::default();
            let subscriber = tracing_subscriber::registry()
                .with(EnvFilter::new("warn"))
                .with(capture.clone());
            let config = ExtractionConfig::default();
            assert!(config.security_limits.is_none());

            let result = tracing::subscriber::with_default(subscriber, || {
                enforce_image_page_limit_stub_with_guard(TIFF_MIME, &config, &already_warned)
            });

            assert!(result.is_ok());
            assert_eq!(warn_event_count(&capture), 0);
        }

        /// Fails if: the `if !mime_type.to_lowercase().contains("tiff") {
        /// return Ok(()); }` guard in `enforce_image_page_limit_stub_with_guard`
        /// is removed (or its condition inverted), causing a non-TIFF mime type
        /// to warn.
        #[test]
        fn does_not_fire_for_non_tiff_mime() {
            let already_warned = AtomicBool::new(false);
            let capture = EventCapture::default();
            let subscriber = tracing_subscriber::registry()
                .with(EnvFilter::new("warn"))
                .with(capture.clone());
            let config = config_with_max_pages(3);

            let result = tracing::subscriber::with_default(subscriber, || {
                enforce_image_page_limit_stub_with_guard("image/png", &config, &already_warned)
            });

            assert!(result.is_ok());
            assert_eq!(warn_event_count(&capture), 0);
        }

        /// A second, independently-guarded call for a TIFF with `max_pages`
        /// set still succeeds and still warns — proving the "does not fire"
        /// tests above are silent because their inputs don't qualify, not
        /// because the warning path is broken. Fails under the same mutations
        /// as `fires_exactly_once_for_tiff_with_max_pages_set_across_repeated_calls`.
        #[test]
        fn fires_when_max_pages_is_set_and_mime_is_tiff_with_a_fresh_guard() {
            let already_warned = AtomicBool::new(false);
            let capture = EventCapture::default();
            let subscriber = tracing_subscriber::registry()
                .with(EnvFilter::new("warn"))
                .with(capture.clone());
            let config = config_with_max_pages(3);

            let result = tracing::subscriber::with_default(subscriber, || {
                enforce_image_page_limit_stub_with_guard(TIFF_MIME, &config, &already_warned)
            });

            assert!(result.is_ok());
            assert_eq!(warn_event_count(&capture), 1);
        }
    }
}
