//! Structured data extractor (JSON, JSONL, YAML, TOML).

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::core::mime::GEOJSON_MIME_TYPE;
use crate::extractors::security::{SecurityBudget, SecurityError, SecurityLimits};
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::metadata::Metadata;
use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;
use std::collections::BTreeSet;
#[cfg(feature = "tokio-runtime")]
use std::path::Path;

const MAX_GEOJSON_PROPERTY_KEYS: usize = 256;
const MAX_GEOJSON_PROPERTY_KEY_BYTES: usize = 4 * 1024;
const MAX_GEOJSON_NAME_BYTES: usize = 256;

#[derive(Debug, Default)]
struct GeometryTypeCounts {
    point: usize,
    multi_point: usize,
    line_string: usize,
    multi_line_string: usize,
    polygon: usize,
    multi_polygon: usize,
    geometry_collection: usize,
    unknown: usize,
}

impl GeometryTypeCounts {
    fn record(&mut self, geometry_type: &str) -> bool {
        let (count, unknown) = match geometry_type {
            "Point" => (&mut self.point, false),
            "MultiPoint" => (&mut self.multi_point, false),
            "LineString" => (&mut self.line_string, false),
            "MultiLineString" => (&mut self.multi_line_string, false),
            "Polygon" => (&mut self.polygon, false),
            "MultiPolygon" => (&mut self.multi_polygon, false),
            "GeometryCollection" => (&mut self.geometry_collection, false),
            _ => (&mut self.unknown, true),
        };
        *count = count.saturating_add(1);
        unknown
    }

    fn into_value(self) -> serde_json::Value {
        serde_json::json!({
            "Point": self.point,
            "MultiPoint": self.multi_point,
            "LineString": self.line_string,
            "MultiLineString": self.multi_line_string,
            "Polygon": self.polygon,
            "MultiPolygon": self.multi_polygon,
            "GeometryCollection": self.geometry_collection,
            "Unknown": self.unknown,
        })
    }
}

#[derive(Debug, Default)]
struct GeoJsonSummary {
    document_type: String,
    name: Option<String>,
    name_truncated: bool,
    feature_count: usize,
    geometry_count: usize,
    null_geometry_count: usize,
    invalid_geometry_count: usize,
    invalid_feature_count: usize,
    position_count: usize,
    malformed_position_array_count: usize,
    empty_coordinate_array_count: usize,
    property_count: usize,
    property_keys: BTreeSet<String>,
    property_key_bytes: usize,
    property_keys_truncated: bool,
    geometry_types: GeometryTypeCounts,
    bounds: Option<[f64; 4]>,
    discarded_coordinate_arrays: bool,
    discarded_feature_structure: bool,
    discarded_geometry_structure: bool,
    discarded_feature_properties: bool,
    discarded_feature_ids: bool,
    discarded_bounding_boxes: bool,
    discarded_foreign_members: bool,
    discarded_unknown_type_values: bool,
    discarded_non_object_root: bool,
}

impl GeoJsonSummary {
    fn record_position(&mut self, x: f64, y: f64) {
        self.position_count = self.position_count.saturating_add(1);
        match self.bounds.as_mut() {
            Some(bounds) => {
                bounds[0] = bounds[0].min(x);
                bounds[1] = bounds[1].min(y);
                bounds[2] = bounds[2].max(x);
                bounds[3] = bounds[3].max(y);
            }
            None => self.bounds = Some([x, y, x, y]),
        }
    }

    fn record_property_key(&mut self, key: &str) {
        self.property_count = self.property_count.saturating_add(1);
        if self.property_keys.contains(key) {
            return;
        }
        if key.len() > MAX_GEOJSON_PROPERTY_KEY_BYTES.saturating_sub(self.property_key_bytes) {
            self.property_keys_truncated = true;
            return;
        }
        let encoded_bytes = serde_json::to_string(key).map_or(usize::MAX, |encoded| encoded.len());
        let next_bytes = self.property_key_bytes.saturating_add(encoded_bytes);
        if self.property_keys.len() >= MAX_GEOJSON_PROPERTY_KEYS || next_bytes > MAX_GEOJSON_PROPERTY_KEY_BYTES {
            self.property_keys_truncated = true;
            return;
        }
        self.property_keys.insert(key.to_string());
        self.property_key_bytes = next_bytes;
    }

    fn into_value(self) -> serde_json::Value {
        let mut discarded_categories = Vec::new();
        for (discarded, category) in [
            (self.discarded_coordinate_arrays, "coordinate_arrays"),
            (self.discarded_feature_structure, "feature_structure"),
            (self.discarded_geometry_structure, "geometry_structure"),
            (self.discarded_feature_properties, "feature_property_values"),
            (self.discarded_feature_ids, "feature_ids"),
            (self.discarded_bounding_boxes, "declared_bounding_boxes"),
            (self.discarded_foreign_members, "foreign_members"),
            (self.discarded_unknown_type_values, "unknown_type_values"),
            (self.discarded_non_object_root, "non_object_root"),
        ] {
            if discarded {
                discarded_categories.push(category);
            }
        }
        serde_json::json!({
            "type": self.document_type,
            "name": self.name,
            "name_truncated": self.name_truncated,
            "feature_count": self.feature_count,
            "geometry_count": self.geometry_count,
            "null_geometry_count": self.null_geometry_count,
            "invalid_geometry_count": self.invalid_geometry_count,
            "invalid_feature_count": self.invalid_feature_count,
            "geometry_types": self.geometry_types.into_value(),
            "property_count": self.property_count,
            "property_keys": self.property_keys,
            "property_keys_truncated": self.property_keys_truncated,
            "position_count": self.position_count,
            "malformed_position_array_count": self.malformed_position_array_count,
            "empty_coordinate_array_count": self.empty_coordinate_array_count,
            "bounds": self.bounds,
            "discarded_categories": discarded_categories,
        })
    }
}

fn validate_geojson_value(value: &serde_json::Value, budget: &mut SecurityBudget) -> Result<()> {
    budget.step()?;
    match value {
        serde_json::Value::String(text) => budget.check_entity(text)?,
        serde_json::Value::Array(values) => {
            budget.enter()?;
            for value in values {
                validate_geojson_value(value, budget)?;
            }
            budget.leave();
        }
        serde_json::Value::Object(object) => {
            budget.enter()?;
            for (key, value) in object {
                budget.check_entity(key)?;
                validate_geojson_value(value, budget)?;
            }
            budget.leave();
        }
        _ => {}
    }
    Ok(())
}

fn truncate_utf8(value: &str, max_bytes: usize) -> (String, bool) {
    if value.len() <= max_bytes {
        return (value.to_string(), false);
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    (value[..end].to_string(), true)
}

fn record_discarded_members(
    summary: &mut GeoJsonSummary,
    object: &serde_json::Map<String, serde_json::Value>,
    known: &[&str],
) {
    summary.discarded_bounding_boxes |= object.get("bbox").is_some_and(value_has_content);
    summary.discarded_foreign_members |= object
        .iter()
        .any(|(key, value)| !known.contains(&key.as_str()) && value_has_content(value));
}

fn value_has_content(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::Array(values) => !values.is_empty(),
        serde_json::Value::Object(values) => !values.is_empty(),
        _ => true,
    }
}

fn scan_geojson_position(value: &serde_json::Value, summary: &mut GeoJsonSummary) -> bool {
    let serde_json::Value::Array(values) = value else {
        summary.malformed_position_array_count = summary.malformed_position_array_count.saturating_add(1);
        return false;
    };
    if values.is_empty() {
        summary.empty_coordinate_array_count = summary.empty_coordinate_array_count.saturating_add(1);
        return false;
    }
    if values.len() < 2 || !values.iter().all(serde_json::Value::is_number) {
        summary.malformed_position_array_count = summary.malformed_position_array_count.saturating_add(1);
        return false;
    }
    let (Some(x), Some(y)) = (values[0].as_f64(), values[1].as_f64()) else {
        summary.malformed_position_array_count = summary.malformed_position_array_count.saturating_add(1);
        return false;
    };
    summary.record_position(x, y);
    true
}

fn scan_geojson_line(
    value: &serde_json::Value,
    minimum_positions: usize,
    closed: bool,
    summary: &mut GeoJsonSummary,
) -> bool {
    let Some(positions) = value.as_array() else {
        summary.malformed_position_array_count = summary.malformed_position_array_count.saturating_add(1);
        return false;
    };
    if positions.len() < minimum_positions {
        if positions.is_empty() {
            summary.empty_coordinate_array_count = summary.empty_coordinate_array_count.saturating_add(1);
        } else {
            summary.malformed_position_array_count = summary.malformed_position_array_count.saturating_add(1);
        }
        return false;
    }
    let mut positions_valid = true;
    for position in positions {
        positions_valid = scan_geojson_position(position, summary) && positions_valid;
    }
    let closure_valid = !closed || positions.first() == positions.last();
    if !closure_valid {
        summary.malformed_position_array_count = summary.malformed_position_array_count.saturating_add(1);
    }
    positions_valid && closure_valid
}

fn scan_geojson_nested(
    value: &serde_json::Value,
    summary: &mut GeoJsonSummary,
    scan_child: impl Fn(&serde_json::Value, &mut GeoJsonSummary) -> bool,
) -> bool {
    let Some(children) = value.as_array() else {
        summary.malformed_position_array_count = summary.malformed_position_array_count.saturating_add(1);
        return false;
    };
    if children.is_empty() {
        summary.empty_coordinate_array_count = summary.empty_coordinate_array_count.saturating_add(1);
        return true;
    }
    let mut children_valid = true;
    for child in children {
        children_valid = scan_child(child, summary) && children_valid;
    }
    children_valid
}

fn merge_valid_position_scan(summary: &mut GeoJsonSummary, scan: GeoJsonSummary, valid: bool) {
    summary.malformed_position_array_count = summary
        .malformed_position_array_count
        .saturating_add(scan.malformed_position_array_count);
    summary.empty_coordinate_array_count = summary
        .empty_coordinate_array_count
        .saturating_add(scan.empty_coordinate_array_count);
    if !valid {
        return;
    }
    summary.position_count = summary.position_count.saturating_add(scan.position_count);
    if let Some([min_x, min_y, max_x, max_y]) = scan.bounds {
        match summary.bounds.as_mut() {
            Some(bounds) => {
                bounds[0] = bounds[0].min(min_x);
                bounds[1] = bounds[1].min(min_y);
                bounds[2] = bounds[2].max(max_x);
                bounds[3] = bounds[3].max(max_y);
            }
            None => summary.bounds = Some([min_x, min_y, max_x, max_y]),
        }
    }
}

fn scan_geojson_coordinates(geometry_type: &str, value: &serde_json::Value, summary: &mut GeoJsonSummary) -> bool {
    let mut scan = GeoJsonSummary::default();
    let valid = match geometry_type {
        "Point" => scan_geojson_position(value, &mut scan),
        "MultiPoint" => scan_geojson_line(value, 0, false, &mut scan),
        "LineString" => scan_geojson_line(value, 2, false, &mut scan),
        "MultiLineString" => {
            scan_geojson_nested(value, &mut scan, |line, scan| scan_geojson_line(line, 2, false, scan))
        }
        "Polygon" => scan_geojson_nested(value, &mut scan, |ring, scan| scan_geojson_line(ring, 4, true, scan)),
        "MultiPolygon" => scan_geojson_nested(value, &mut scan, |polygon, scan| {
            scan_geojson_nested(polygon, scan, |ring, scan| scan_geojson_line(ring, 4, true, scan))
        }),
        _ => false,
    };
    merge_valid_position_scan(summary, scan, valid);
    valid
}

fn scan_geojson_geometry(geometry: &serde_json::Value, summary: &mut GeoJsonSummary, root: bool) {
    if geometry.is_null() {
        summary.null_geometry_count = summary.null_geometry_count.saturating_add(1);
        return;
    }
    let Some(object) = geometry.as_object() else {
        summary.invalid_geometry_count = summary.invalid_geometry_count.saturating_add(1);
        return;
    };
    let known = if root {
        &["type", "name", "coordinates", "geometries", "bbox"][..]
    } else {
        &["type", "coordinates", "geometries", "bbox"][..]
    };
    record_discarded_members(summary, object, known);
    let geometry_type = object.get("type").and_then(serde_json::Value::as_str);
    if let Some(geometry_type) = geometry_type {
        summary.geometry_count = summary.geometry_count.saturating_add(1);
        summary.discarded_unknown_type_values |= summary.geometry_types.record(geometry_type);
    }
    let valid = match geometry_type {
        Some("GeometryCollection") => {
            let coordinates_absent = !object.contains_key("coordinates");
            if !coordinates_absent {
                summary.discarded_coordinate_arrays |= object.get("coordinates").is_some_and(value_has_content);
            }
            let geometries_valid = match object.get("geometries") {
                Some(serde_json::Value::Array(geometries)) => {
                    summary.discarded_geometry_structure = true;
                    for child in geometries {
                        scan_geojson_geometry(child, summary, false);
                    }
                    true
                }
                Some(_) => {
                    summary.discarded_geometry_structure = true;
                    false
                }
                None => false,
            };
            coordinates_absent && geometries_valid
        }
        Some(
            geometry_type @ ("Point" | "MultiPoint" | "LineString" | "MultiLineString" | "Polygon" | "MultiPolygon"),
        ) => {
            let coordinates_valid = if let Some(coordinates) = object.get("coordinates") {
                summary.discarded_coordinate_arrays |= value_has_content(coordinates);
                scan_geojson_coordinates(geometry_type, coordinates, summary)
            } else {
                false
            };
            if object.contains_key("geometries") {
                summary.discarded_geometry_structure = true;
            }
            coordinates_valid && !object.contains_key("geometries")
        }
        Some(_) | None => false,
    };
    if !valid {
        summary.invalid_geometry_count = summary.invalid_geometry_count.saturating_add(1);
    }
}

fn scan_geojson_feature(feature: &serde_json::Value, summary: &mut GeoJsonSummary, root: bool) {
    let Some(object) = feature.as_object() else {
        summary.invalid_feature_count = summary.invalid_feature_count.saturating_add(1);
        return;
    };
    let known = if root {
        &["type", "name", "properties", "geometry", "id", "bbox"][..]
    } else {
        &["type", "properties", "geometry", "id", "bbox"][..]
    };
    record_discarded_members(summary, object, known);
    summary.discarded_feature_structure |= root;
    if object.get("type").and_then(serde_json::Value::as_str) != Some("Feature") {
        summary.invalid_feature_count = summary.invalid_feature_count.saturating_add(1);
        summary.discarded_unknown_type_values |= object.contains_key("type");
    }
    if let Some(properties) = object.get("properties") {
        if let Some(properties) = properties.as_object() {
            summary.discarded_feature_properties |= !properties.is_empty();
            for key in properties.keys() {
                summary.record_property_key(key);
            }
        } else if !properties.is_null() {
            summary.discarded_feature_properties = true;
        }
    }
    summary.discarded_feature_ids |= object.get("id").is_some_and(|id| !id.is_null());
    if let Some(geometry) = object.get("geometry") {
        summary.discarded_geometry_structure |= value_has_content(geometry);
        scan_geojson_geometry(geometry, summary, false);
    }
}

fn summarize_geojson(value: &serde_json::Value) -> serde_json::Value {
    let Some(object) = value.as_object() else {
        return GeoJsonSummary {
            document_type: "Unknown".to_string(),
            discarded_non_object_root: true,
            ..GeoJsonSummary::default()
        }
        .into_value();
    };
    let document_type = object
        .get("type")
        .and_then(serde_json::Value::as_str)
        .filter(|value| {
            matches!(
                *value,
                "FeatureCollection"
                    | "Feature"
                    | "Point"
                    | "MultiPoint"
                    | "LineString"
                    | "MultiLineString"
                    | "Polygon"
                    | "MultiPolygon"
                    | "GeometryCollection"
            )
        })
        .unwrap_or("Unknown");
    let (name, name_truncated) = object
        .get("name")
        .and_then(serde_json::Value::as_str)
        .map(|name| truncate_utf8(name, MAX_GEOJSON_NAME_BYTES))
        .map_or((None, false), |(name, truncated)| (Some(name), truncated));
    let mut summary = GeoJsonSummary {
        document_type: document_type.to_string(),
        name,
        name_truncated,
        ..GeoJsonSummary::default()
    };
    match document_type {
        "FeatureCollection" => {
            record_discarded_members(&mut summary, object, &["type", "name", "features", "bbox"]);
            if let Some(features) = object.get("features") {
                summary.discarded_feature_structure = true;
                if let Some(features) = features.as_array() {
                    summary.feature_count = features.len();
                    for feature in features {
                        scan_geojson_feature(feature, &mut summary, false);
                    }
                } else {
                    summary.invalid_feature_count = summary.invalid_feature_count.saturating_add(1);
                }
            }
        }
        "Feature" => {
            summary.feature_count = 1;
            scan_geojson_feature(value, &mut summary, true);
        }
        "Unknown" => {
            record_discarded_members(&mut summary, object, &["type", "name", "bbox"]);
            summary.discarded_unknown_type_values |= object.contains_key("type");
        }
        _ => scan_geojson_geometry(value, &mut summary, true),
    }
    summary.into_value()
}

/// Builds the `metadata.additional` map for a structured-format extraction: field/format
/// counts, the GeoJSON summarization flags when applicable, the flattened `path: value`
/// view (xberg-io/xberg#166), and every parser-reported metadata entry. Split out of
/// [`StructuredExtractor::extract_content`] purely to shorten that method.
fn build_additional_metadata(
    structured_result: &crate::extraction::structured::StructuredDataResult,
    is_geojson: bool,
    summarize_geojson: bool,
) -> AHashMap<Cow<'static, str>, serde_json::Value> {
    let mut additional = AHashMap::new();
    additional.insert(
        Cow::Borrowed("field_count"),
        serde_json::json!(structured_result.text_fields.len()),
    );
    additional.insert(
        Cow::Borrowed("data_format"),
        serde_json::json!(structured_result.format),
    );
    if is_geojson {
        additional.insert(
            Cow::Borrowed("geojson_summarized"),
            serde_json::json!(summarize_geojson),
        );
        if summarize_geojson && let Some(summary) = structured_result.value.as_ref() {
            additional.insert(Cow::Borrowed("geojson_summary"), summary.clone());
        }
    }
    // Surface the full flattened `path: value` view instead of discarding it
    // (xberg-io/xberg#166): the structured renderer above only emits headings/lists for
    // a subset of shapes, so this is the one place a consumer can always get every leaf
    // field as text, regardless of source format or nesting.
    if !structured_result.flattened.is_empty() {
        additional.insert(
            Cow::Borrowed("flattened_fields"),
            serde_json::json!(structured_result.flattened),
        );
    }

    for (key, value) in &structured_result.metadata {
        additional.insert(Cow::Owned(key.clone()), serde_json::json!(value));
    }

    additional
}

fn parse_geojson_summary(
    content: &[u8],
    config: &ExtractionConfig,
) -> Result<crate::extraction::structured::StructuredDataResult> {
    let default_limits;
    let limits = match config.security_limits.as_ref() {
        Some(limits) => limits,
        None => {
            default_limits = SecurityLimits::default();
            &default_limits
        }
    };
    if content.len() > limits.max_content_size {
        return Err(SecurityError::ContentTooLarge {
            size: content.len(),
            max: limits.max_content_size,
        }
        .into());
    }
    let value: serde_json::Value = serde_json::from_slice(content)
        .map_err(|error| crate::XbergError::parsing(format!("Failed to parse GeoJSON: {error}")))?;
    let mut budget = SecurityBudget::from_limits(limits);
    validate_geojson_value(&value, &mut budget)?;
    let summary = summarize_geojson(&value);
    let encoded = serde_json::to_vec(&summary)
        .map_err(|error| crate::XbergError::parsing(format!("Failed to encode GeoJSON summary: {error}")))?;
    crate::extraction::structured::parse_json(&encoded, None)
}

/// Build an `InternalDocument` from a structured data result.
///
/// For JSON objects: top-level keys become headings, nested objects become
/// sub-headings, arrays become lists. Falls back to a code block for other formats.
///
/// `budget` enforces hostile-input limits (nesting depth, iteration count, entity
/// length, cumulative content size). Any limit violation is converted into a
/// `XbergError::Security` via the `?` operator.
fn build_internal_document(
    result: &crate::extraction::structured::StructuredDataResult,
    mime_type: &str,
    budget: &mut SecurityBudget,
) -> Result<InternalDocument> {
    let source_format = match mime_type {
        "application/json" | "text/json" | "application/csl+json" | GEOJSON_MIME_TYPE | "application/vnd.geo+json" => {
            "json"
        }
        "application/x-ndjson" | "application/jsonl" | "application/x-jsonlines" => "jsonl",
        "application/yaml" | "application/x-yaml" | "text/yaml" | "text/x-yaml" => "yaml",
        "application/toml" | "text/toml" => "toml",
        _ => "structured",
    };

    let language = match source_format {
        "json" | "jsonl" => Some("json"),
        "yaml" => Some("yaml"),
        "toml" => Some("toml"),
        _ => None,
    };

    let mut builder = InternalDocumentBuilder::new(source_format);

    // Render document structure (headings, sub-headings, lists) from the parsed value for
    // every structured format, not just JSON objects: YAML, TOML, and JSONL already parse
    // into the same `serde_json::Value` shape, and a top-level array (JSONL's natural shape,
    // and valid JSON on its own) gets per-item structure instead of an opaque code block
    // (xberg-io/xberg#155). ~keep
    if matches!(source_format, "json" | "jsonl" | "yaml" | "toml")
        && let Some(value) = result.value.as_ref()
    {
        match value {
            serde_json::Value::Object(_) => {
                build_json_internal_structure(value, &mut builder, 1, budget)?;
                return Ok(builder.build());
            }
            serde_json::Value::Array(items) if !items.is_empty() => {
                build_json_array(items, &mut builder, 1, budget)?;
                return Ok(builder.build());
            }
            _ => {}
        }
    }

    budget.account_text(result.content.len())?;
    builder.push_code(&result.content, language, None, None);
    Ok(builder.build())
}

/// Recursively build internal document structure from a JSON value.
///
/// `budget` enforces nesting depth, iteration count, entity length, and
/// cumulative text size limits against hostile input.
fn build_json_internal_structure(
    value: &serde_json::Value,
    builder: &mut InternalDocumentBuilder,
    depth: u8,
    budget: &mut SecurityBudget,
) -> Result<()> {
    let level = depth.min(6);
    match value {
        serde_json::Value::Object(map) => {
            budget.enter()?;
            for (key, val) in map {
                budget.step()?;
                budget.check_entity(key)?;
                match val {
                    serde_json::Value::Object(_) => {
                        builder.push_heading(level, key, None, None);
                        build_json_internal_structure(val, builder, depth + 1, budget)?;
                    }
                    serde_json::Value::Array(arr) => {
                        builder.push_heading(level, key, None, None);
                        build_json_array(arr, builder, depth + 1, budget)?;
                    }
                    serde_json::Value::String(s) => {
                        budget.check_entity(s)?;
                        let rendered = format!("{}: {}", key, s);
                        budget.account_text(rendered.len())?;
                        builder.push_paragraph(&rendered, vec![], None, None);
                    }
                    other => {
                        let rendered = format!("{}: {}", key, other);
                        budget.account_text(rendered.len())?;
                        builder.push_paragraph(&rendered, vec![], None, None);
                    }
                }
            }
            budget.leave();
        }
        serde_json::Value::Array(arr) => {
            build_json_array(arr, builder, depth, budget)?;
        }
        serde_json::Value::String(s) => {
            budget.check_entity(s)?;
            budget.account_text(s.len())?;
            builder.push_paragraph(s, vec![], None, None);
        }
        other => {
            let rendered = other.to_string();
            budget.account_text(rendered.len())?;
            builder.push_paragraph(&rendered, vec![], None, None);
        }
    }
    Ok(())
}

/// Render array scalars as list items and recursively expand structured items.
///
/// Lists are closed before an object or nested array is rendered so headings
/// and paragraphs do not become implicit children of the preceding list item.
fn build_json_array(
    values: &[serde_json::Value],
    builder: &mut InternalDocumentBuilder,
    depth: u8,
    budget: &mut SecurityBudget,
) -> Result<()> {
    const ARRAY_ITEM_LABEL: &str = "Item";

    budget.enter()?;
    let mut list_is_open = false;
    for (index, value) in values.iter().enumerate() {
        budget.step()?;
        match value {
            serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                if list_is_open {
                    builder.end_list();
                    list_is_open = false;
                }
                let label = format!("{ARRAY_ITEM_LABEL} {}", index + 1);
                budget.account_text(label.len())?;
                builder.push_heading(depth.min(6), &label, None, None);
                build_json_internal_structure(value, builder, depth + 1, budget)?;
            }
            serde_json::Value::String(text) => {
                budget.check_entity(text)?;
                budget.account_text(text.len())?;
                if !list_is_open {
                    builder.push_list(false);
                    list_is_open = true;
                }
                builder.push_list_item(text, false, vec![], None, None);
            }
            scalar => {
                let rendered = scalar.to_string();
                budget.account_text(rendered.len())?;
                if !list_is_open {
                    builder.push_list(false);
                    list_is_open = true;
                }
                builder.push_list_item(&rendered, false, vec![], None, None);
            }
        }
    }
    if list_is_open {
        builder.end_list();
    }
    budget.leave();
    Ok(())
}

/// Structured data extractor supporting JSON, JSONL/NDJSON, YAML, and TOML.
#[cfg_attr(alef, alef(skip))]
pub struct StructuredExtractor;

impl Default for StructuredExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl StructuredExtractor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Plugin for StructuredExtractor {
    fn name(&self) -> &str {
        "structured-extractor"
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
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for StructuredExtractor {
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        // A UTF-8 byte order mark is not part of the data. `serde_json` and the YAML parser reject
        // it, so a document saved with one fell back to plain text. ~keep
        let content = content.strip_prefix(b"\xEF\xBB\xBF".as_slice()).unwrap_or(content);
        let is_geojson = matches!(mime_type, GEOJSON_MIME_TYPE | "application/vnd.geo+json");
        let summarize_geojson = is_geojson
            && !config
                .geojson
                .as_ref()
                .is_some_and(|options| options.include_full_coordinates);
        let structured_result = match mime_type {
            GEOJSON_MIME_TYPE | "application/vnd.geo+json" if summarize_geojson => {
                parse_geojson_summary(content, config)?
            }
            "application/json"
            | "text/json"
            | "application/csl+json"
            | GEOJSON_MIME_TYPE
            | "application/vnd.geo+json" => crate::extraction::structured::parse_json(content, None)?,
            "application/x-ndjson" | "application/jsonl" | "application/x-jsonlines" => {
                crate::extraction::structured::parse_jsonl(content, None)?
            }
            "application/yaml" | "application/x-yaml" | "text/yaml" | "text/x-yaml" => {
                crate::extraction::structured::parse_yaml(content)?
            }
            "application/toml" | "text/toml" => crate::extraction::structured::parse_toml(content)?,
            _ => return Err(crate::XbergError::UnsupportedFormat(mime_type.to_string())),
        };

        let additional = build_additional_metadata(&structured_result, is_geojson, summarize_geojson);

        let mut budget = SecurityBudget::from_config(config);
        let mut doc = build_internal_document(&structured_result, mime_type, &mut budget)?;
        doc.mime_type = mime_type.to_string();

        doc.metadata = Metadata {
            additional,
            ..Default::default()
        };
        if summarize_geojson {
            doc.processing_warnings.push(crate::types::ProcessingWarning {
                source: Cow::Borrowed("geojson"),
                message: Cow::Borrowed(
                    "GeoJSON was replaced by a bounded aggregate summary; inspect \
                     `metadata.geojson_summary.discarded_categories` for omitted data, or set \
                     `geojson.include_full_coordinates` to true to preserve the full document.",
                ),
            });
        }

        Ok(doc)
    }

    #[cfg(feature = "tokio-runtime")]
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, path, config),
        fields(
            extractor.name = self.name(),
        )
    ))]
    async fn extract_path(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        let bytes = crate::core::io::read_file_async(path).await?;
        self.extract_content(&bytes, mime_type, config).await
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "application/json",
            "text/json",
            "application/csl+json",
            GEOJSON_MIME_TYPE,
            "application/vnd.geo+json",
            "application/x-ndjson",
            "application/jsonl",
            "application/x-jsonlines",
            "application/yaml",
            "application/x-yaml",
            "text/yaml",
            "text/x-yaml",
            "application/toml",
            "text/toml",
        ]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests;
