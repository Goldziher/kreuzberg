//! CSV and TSV extractor.
//!
//! Parses CSV/TSV files into structured table data and clean text output.
//! Handles RFC 4180 quoted fields with embedded commas and newlines.

use std::borrow::Cow;
use std::sync::LazyLock;

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::security::SecurityBudget;
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::text::utf8_validation;
use crate::types::Table;
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::metadata::{CsvMetadata, FormatMetadata, Metadata};
use async_trait::async_trait;

static DATE_RE_ISO: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"^\d{4}-\d{2}-\d{2}").unwrap());
static DATE_RE_US: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"^\d{1,2}/\d{1,2}/\d{2,4}").unwrap());
static DATE_RE_EU: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"^\d{1,2}\.\d{1,2}\.\d{2,4}").unwrap());

#[cfg_attr(alef, alef(skip))]
/// CSV/TSV extractor with proper field parsing.
///
/// Replaces raw text passthrough with structured CSV parsing,
/// producing space-separated text output and populated `tables` field.
pub struct CsvExtractor;

impl CsvExtractor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for CsvExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for CsvExtractor {
    fn name(&self) -> &str {
        "csv-extractor"
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
        "CSV/TSV text extraction with table structure"
    }

    fn author(&self) -> &str {
        "Xberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for CsvExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        tracing::debug!(format = "csv", size_bytes = content.len(), "extraction starting");
        let mut budget = SecurityBudget::from_config(config);
        let (rows, delimiter) = parse_budgeted_csv(content, mime_type, config, &mut budget)?;
        let (table, csv_metadata) = build_csv_table_and_metadata(rows, delimiter);

        let mut builder = InternalDocumentBuilder::new("csv");

        // Unlike the plain `from_utf8_lossy` used by html/rtf/text, `decode_csv_bytes`
        // cannot leave a detectable "genuinely undecodable" signal in either build (#171):
        // without `quality`, `decode_csv_bytes_fallback`'s encoding list ends in
        // windows-1252/iso-8859-1, which the WHATWG Encoding Standard defines a mapping
        // for every byte 0x00-0xFF, so it always succeeds -- the bytes are reinterpreted
        // under a (possibly wrong) encoding, never dropped, and the trailing
        // `String::from_utf8_lossy` fallback is unreachable dead code. With `quality`,
        // `crate::utils::safe_decode` returns a string that has already had every
        // replacement character stripped by its internal mojibake cleanup, so no
        // marker of the loss survives to check for here either. Neither build can be
        // told apart from a clean decode without changing that shared helper (out of
        // this extractor's scope), so no lossy-decode warning is emitted for CSV. ~keep

        let content_text = render_plain_text(&table.cells);
        let table_element = builder.push_table(table, None, None);
        builder.set_text(table_element, &content_text);

        let mut doc = builder.build();
        doc.mime_type = mime_type.to_string();

        doc.metadata = Metadata {
            format: Some(FormatMetadata::Csv(csv_metadata)),
            ..Default::default()
        };

        tracing::debug!(
            element_count = doc.elements.len(),
            format = "csv",
            "extraction complete"
        );
        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/csv", "text/tab-separated-values"]
    }

    fn priority(&self) -> i32 {
        60
    }
}

/// Decodes, strips comment lines, resolves the delimiter, parses the CSV rows, and
/// accounts every row/cell against `budget`. Split out of
/// [`CsvExtractor::extract_content`] purely to shorten that method.
fn parse_budgeted_csv(
    content: &[u8],
    mime_type: &str,
    config: &ExtractionConfig,
    budget: &mut SecurityBudget,
) -> Result<(Vec<Vec<String>>, char)> {
    let text = decode_csv_bytes(content);
    let csv_config = config.csv.as_ref();
    let comment_prefixes: &[String] = csv_config.map(|c| c.comment_prefixes.as_slice()).unwrap_or(&[]);
    let configured_delimiter = csv_config
        .and_then(|c| c.delimiter.as_deref())
        .and_then(|d| d.chars().next());

    let filtered_text = strip_comment_lines(&text, comment_prefixes);

    let delimiter = if mime_type == "text/tab-separated-values" {
        '\t'
    } else if let Some(delimiter) = configured_delimiter {
        delimiter
    } else {
        detect_delimiter(&filtered_text)
    };

    let rows = parse_csv(&filtered_text, delimiter);

    for row in &rows {
        budget.step()?;
        budget.add_cells(row.len())?;
        for cell in row {
            budget.check_entity(cell)?;
            budget.account_text(cell.len())?;
        }
    }

    Ok((rows, delimiter))
}

/// Builds the rendered [`Table`] and [`CsvMetadata`] from parsed rows. Split out of
/// [`CsvExtractor::extract_content`] purely to shorten that method.
fn build_csv_table_and_metadata(rows: Vec<Vec<String>>, delimiter: char) -> (Table, CsvMetadata) {
    let row_count = rows.len();
    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let has_header = detect_header(&rows);
    let column_types = infer_column_types(&rows, has_header);

    let markdown = build_markdown_table(&rows, has_header);
    let columns = has_header.then(|| rows.first().cloned()).flatten();

    let table = Table {
        cells: rows,
        markdown,
        page_number: 1,
        bounding_box: None,
        columns,
        ..Default::default()
    };

    let csv_metadata = CsvMetadata {
        row_count: row_count as u32,
        column_count: col_count as u32,
        delimiter: if delimiter != ',' {
            Some(delimiter.to_string())
        } else {
            None
        },
        has_header,
        column_types: if column_types.is_empty() {
            None
        } else {
            Some(column_types)
        },
    };

    (table, csv_metadata)
}

/// Maximum number of non-blank lines sampled by [`detect_delimiter`].
///
/// Widened from the original 10-line sample (xberg-io/xberg#164): a short
/// sample is easily dominated by a handful of narrow leading rows (e.g. a
/// title block) and picks the wrong delimiter for the rest of the file.
const DELIMITER_SAMPLE_LINES: usize = 50;

/// Auto-detect CSV delimiter using consistency-based approach.
/// Tests each candidate delimiter and picks the one producing the most
/// consistent column count across sample lines.
///
/// Blank lines and `#`-prefixed comment lines are skipped when building the
/// sample so spacer rows and leading comments don't dilute the consistency
/// score used to pick the delimiter.
fn detect_delimiter(text: &str) -> char {
    const CANDIDATES: &[char] = &[',', '\t', '|', ';'];
    let mut best_delimiter = ',';
    let mut best_score = 0usize;

    let sample: String = text
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .take(DELIMITER_SAMPLE_LINES)
        .collect::<Vec<_>>()
        .join("\n");

    for &candidate in CANDIDATES {
        let rows = parse_csv(&sample, candidate);
        if rows.len() < 2 {
            continue;
        }
        let col_counts: Vec<usize> = rows.iter().map(|r| r.len()).collect();
        let first_count = col_counts[0];
        if first_count <= 1 {
            continue;
        }
        let consistent_rows = col_counts.iter().filter(|&&c| c == first_count).count();
        let score = consistent_rows * first_count;
        if score > best_score {
            best_score = score;
            best_delimiter = candidate;
        }
    }
    best_delimiter
}

/// Remove lines whose trimmed start matches one of `prefixes` from `text`.
///
/// Comment lines are dropped entirely (not just their content), so row
/// indices in the remaining data are unaffected by their removal. Preserves
/// each surviving line's original terminator (`\n` or `\r\n`) so downstream
/// CRLF handling in [`parse_csv`] is unaffected.
///
/// Returns the input unchanged (borrowed, no allocation) when `prefixes` is
/// empty — the default when [`crate::core::config::CsvConfig`] is unset —
/// so existing behavior is preserved byte-for-byte.
fn strip_comment_lines<'a>(text: &'a str, prefixes: &[String]) -> Cow<'a, str> {
    if prefixes.is_empty() {
        return Cow::Borrowed(text);
    }

    let mut result = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let trimmed_start = line.trim_start();
        let is_comment = prefixes.iter().any(|prefix| trimmed_start.starts_with(prefix.as_str()));
        if !is_comment {
            result.push_str(line);
        }
    }
    Cow::Owned(result)
}

/// Parse CSV text into rows of fields, handling RFC 4180 quoted fields.
fn parse_csv(text: &str, delimiter: char) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_field = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    current_field.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                current_field.push(c);
            }
        } else {
            match c {
                '"' if current_field.is_empty() => {
                    in_quotes = true;
                }
                c if c == delimiter => {
                    current_row.push(current_field.clone());
                    current_field.clear();
                }
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    current_row.push(current_field.clone());
                    current_field.clear();
                    // A genuinely blank line (e.g. a spacer row mid-file) must be kept as its
                    // own row so subsequent row indices don't shift (xberg-io/xberg#164).
                    rows.push(current_row);
                    current_row = Vec::new();
                }
                '\n' => {
                    current_row.push(current_field.clone());
                    current_field.clear();
                    rows.push(current_row);
                    current_row = Vec::new();
                }
                _ => {
                    current_field.push(c);
                }
            }
        }
    }

    if !current_field.is_empty() || !current_row.is_empty() {
        current_row.push(current_field);
        rows.push(current_row);
    }

    rows
}

/// Decode raw CSV bytes with encoding detection.
///
/// Tries UTF-8 first (zero-copy fast path). When the bytes are not valid UTF-8,
/// attempts to detect and decode using common encodings (Shift-JIS, cp932,
/// windows-1252, etc.) using encoding_rs.
///
/// When the `quality` feature is enabled, uses chardetng for more sophisticated
/// encoding detection. Without it, tries common encodings in order.
fn decode_csv_bytes(content: &[u8]) -> String {
    if let Ok(s) = utf8_validation::from_utf8(content) {
        return crate::utils::strip_bom(s).to_string();
    }

    #[cfg(feature = "quality")]
    {
        crate::utils::strip_bom(&crate::utils::safe_decode(content, None)).to_string()
    }

    #[cfg(not(feature = "quality"))]
    {
        decode_csv_bytes_fallback(content)
    }
}

/// Fallback encoding detection for CSV files without the `quality` feature.
///
/// Tries common CSV encodings (Shift-JIS, cp932, windows-1252, etc.) in order,
/// selecting the first one that decodes without errors.
#[cfg(not(feature = "quality"))]
fn decode_csv_bytes_fallback(content: &[u8]) -> String {
    let encoding_labels = [
        "shift_jis",
        "windows-31j",
        "gb18030",
        "big5",
        "windows-1252",
        "iso-8859-1",
    ];

    for label in &encoding_labels {
        if let Some(encoding) = encoding_rs::Encoding::for_label(label.as_bytes()) {
            let (decoded, _, had_errors) = encoding.decode(content);
            if !had_errors {
                return decoded.into_owned();
            }
        }
    }

    if let Some(shift_jis) = encoding_rs::Encoding::for_label(b"shift_jis") {
        let (decoded, _, _) = shift_jis.decode(content);
        return decoded.into_owned();
    }

    String::from_utf8_lossy(content).into_owned()
}

/// Whether a CSV cell is a real number. `str::parse::<f64>` also accepts the
/// tokens "NaN", "inf", "infinity" (case-insensitive), so a header cell or a
/// column of those words would be misclassified as numeric — flipping header
/// and column-type detection (xberg-io/xberg#1223). Reject those spellings.
fn is_csv_number(cell: &str) -> bool {
    let trimmed = cell.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    let lower = lower.strip_prefix(['+', '-']).unwrap_or(&lower);
    if matches!(lower, "nan" | "inf" | "infinity") {
        return false;
    }
    trimmed.parse::<f64>().is_ok()
}

/// Detect whether the first row is a header row.
///
/// Heuristic: the first row is considered a header if:
/// - There are at least 2 rows and the first row has at least 2 columns
/// - No cell in the first row looks numeric (all text/labels)
///
/// A numeric-looking first row is treated as data (headerless). An all-text
/// first row is treated as a header even when the data rows are also all text:
/// that is the dominant CSV convention, and the previous heuristic — which also
/// required at least one numeric data cell — misclassified all-text tables such
/// as `Name,City / Alice,NYC` as headerless, rendering a broken blank header row
/// (xberg-io/xberg#1369).
fn detect_header(rows: &[Vec<String>]) -> bool {
    if rows.len() < 2 {
        return false;
    }

    let first_row = &rows[0];
    if first_row.len() < 2 {
        return false;
    }

    !first_row.iter().any(|cell| is_csv_number(cell))
}

/// Infer column types by scanning the first N data rows.
///
/// Returns a vector of type strings: "numeric", "text", or "date" per column.
fn infer_column_types(rows: &[Vec<String>], has_header: bool) -> Vec<String> {
    if rows.is_empty() {
        return Vec::new();
    }

    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if col_count == 0 {
        return Vec::new();
    }

    let data_start = if has_header { 1 } else { 0 };
    let scan_end = rows.len().min(data_start + 20);
    if data_start >= scan_end {
        return vec!["text".to_string(); col_count];
    }

    let data_rows = &rows[data_start..scan_end];

    let date_patterns: &[&regex::Regex] = &[&DATE_RE_ISO, &DATE_RE_US, &DATE_RE_EU];

    (0..col_count)
        .map(|col_idx| {
            let mut numeric_count = 0usize;
            let mut date_count = 0usize;
            let mut non_empty_count = 0usize;

            for row in data_rows {
                let cell = row.get(col_idx).map(|s| s.trim()).unwrap_or("");
                if cell.is_empty() {
                    continue;
                }
                non_empty_count += 1;

                if is_csv_number(cell) {
                    numeric_count += 1;
                } else {
                    for re in date_patterns {
                        if re.is_match(cell) {
                            date_count += 1;
                            break;
                        }
                    }
                }
            }

            if non_empty_count == 0 {
                "text".to_string()
            } else if numeric_count * 2 >= non_empty_count {
                "numeric".to_string()
            } else if date_count * 2 >= non_empty_count {
                "date".to_string()
            } else {
                "text".to_string()
            }
        })
        .collect()
}

/// Render rows as canonical space-separated plain text for `result.content`.
///
/// Matches `rendering::common::render_table_plain` except that rows where every
/// cell is empty after trimming are omitted entirely, rather than surviving as a
/// blank-looking line of bare separators. The Markdown table and `result.tables`
/// keep such rows, since they still convey real structure there.
fn render_plain_text(cells: &[Vec<String>]) -> String {
    let mut out = String::new();
    for row in cells {
        if row.iter().all(|cell| cell.trim().is_empty()) {
            continue;
        }
        out.push_str(&row.join(" "));
        out.push('\n');
    }
    out
}

/// Build a Markdown table from parsed rows.
fn build_markdown_table(rows: &[Vec<String>], has_header: bool) -> String {
    if rows.is_empty() {
        return String::new();
    }

    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if col_count == 0 {
        return String::new();
    }

    let mut markdown = String::new();
    if !has_header {
        markdown.push('|');
        for _ in 0..col_count {
            markdown.push_str("  |");
        }
        markdown.push('\n');
        markdown.push('|');
        for _ in 0..col_count {
            markdown.push_str(" --- |");
        }
        markdown.push('\n');
    }

    for (i, row) in rows.iter().enumerate() {
        markdown.push('|');
        for j in 0..col_count {
            let cell = row.get(j).map(|s| s.trim()).unwrap_or("");
            markdown.push(' ');
            crate::rendering::common::push_escaped_cell(&mut markdown, cell);
            markdown.push_str(" |");
        }
        markdown.push('\n');

        if has_header && i == 0 {
            markdown.push('|');
            for _ in 0..col_count {
                markdown.push_str(" --- |");
            }
            markdown.push('\n');
        }
    }

    markdown
}

#[cfg(test)]
mod tests;
