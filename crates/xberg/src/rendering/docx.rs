//! Office Open XML (`.docx`) writer.
//!
//! Builds a WordprocessingML package from Markdown rather than from the element tree.
//! Redaction and the other post-processors rewrite text, and a zip archive is not text,
//! so the pipeline carries the Markdown rendering through every processor stage and
//! only packages it here, after the last one has run.

use std::collections::HashMap;
use std::io::{Cursor, Write};

use comrak::arena_tree::NodeEdge;
use comrak::nodes::{ListType, NodeValue, TableAlignment};
use comrak::{Arena, parse_document};
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

use super::common::external_link_target;
use super::markdown::comrak_options;
use crate::{Result, XbergError};

const CONTENT_TYPES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/></Types>"#;

const PACKAGE_RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const STYLES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults><w:rPrDefault><w:rPr><w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="120"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style><w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="360" w:after="120"/><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:b/><w:sz w:val="36"/><w:szCs w:val="36"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="240" w:after="120"/><w:outlineLvl w:val="1"/></w:pPr><w:rPr><w:b/><w:sz w:val="32"/><w:szCs w:val="32"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Heading3"><w:name w:val="heading 3"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="240" w:after="120"/><w:outlineLvl w:val="2"/></w:pPr><w:rPr><w:b/><w:sz w:val="28"/><w:szCs w:val="28"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Heading4"><w:name w:val="heading 4"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="240" w:after="120"/><w:outlineLvl w:val="3"/></w:pPr><w:rPr><w:b/><w:sz w:val="24"/><w:szCs w:val="24"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Heading5"><w:name w:val="heading 5"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="240" w:after="120"/><w:outlineLvl w:val="4"/></w:pPr><w:rPr><w:b/><w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Heading6"><w:name w:val="heading 6"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="240" w:after="120"/><w:outlineLvl w:val="5"/></w:pPr><w:rPr><w:b/><w:i/><w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="SourceCode"><w:name w:val="Source Code"/><w:basedOn w:val="Normal"/><w:qFormat/><w:pPr><w:spacing w:after="120"/></w:pPr><w:rPr><w:rFonts w:ascii="Courier New" w:hAnsi="Courier New" w:cs="Courier New"/><w:sz w:val="20"/><w:szCs w:val="20"/></w:rPr></w:style><w:style w:type="character" w:styleId="Hyperlink"><w:name w:val="Hyperlink"/><w:rPr><w:color w:val="0563C1"/><w:u w:val="single"/></w:rPr></w:style><w:style w:type="table" w:styleId="TableGrid"><w:name w:val="Table Grid"/><w:tblPr><w:tblBorders><w:top w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:left w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:bottom w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:right w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:insideH w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:insideV w:val="single" w:sz="4" w:space="0" w:color="auto"/></w:tblBorders><w:tblCellMar><w:left w:w="108" w:type="dxa"/><w:right w:w="108" w:type="dxa"/></w:tblCellMar></w:tblPr><w:tblStylePr w:type="firstRow"><w:rPr><w:b/></w:rPr></w:tblStylePr></w:style></w:styles>"#;

const WORDPROCESSINGML_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const RELATIONSHIPS_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const HYPERLINK_REL_TYPE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink";

/// `rId`s 1 and 2 belong to the styles and numbering parts; hyperlinks follow.
const FIRST_HYPERLINK_REL_ID: usize = 3;
/// Word defines list levels 0 through 8, so nesting deeper than nine renders at the ninth.
const MAX_LEVEL: usize = 8;
const INDENT_STEP_TWIPS: usize = 720;
const HANGING_INDENT_TWIPS: usize = 360;
/// Text width of a Letter page with one-inch margins, split evenly across a table's columns.
const TABLE_WIDTH_TWIPS: usize = 9360;
const BULLET_GLYPHS: [&str; 3] = ["\u{2022}", "\u{25E6}", "\u{25AA}"];

/// Build a `.docx` package holding `markdown`.
pub(crate) fn render_docx(markdown: &str) -> Result<Vec<u8>> {
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &comrak_options());

    let mut body = BodyWriter::default();
    for edge in root.traverse() {
        match edge {
            NodeEdge::Start(node) => body.start(&node.data.borrow().value),
            NodeEdge::End(node) => body.end(&node.data.borrow().value),
        }
    }
    body.finish();

    package(&body)
}

fn package(body: &BodyWriter) -> Result<Vec<u8>> {
    let parts = [
        ("[Content_Types].xml", CONTENT_TYPES_XML.to_string()),
        ("_rels/.rels", PACKAGE_RELS_XML.to_string()),
        ("word/_rels/document.xml.rels", body.document_rels_xml()),
        ("word/document.xml", body.document_xml()),
        ("word/styles.xml", STYLES_XML.to_string()),
        ("word/numbering.xml", body.numbering_xml()),
    ];

    // A fixed timestamp keeps the package a function of its content alone.
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default());
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, xml) in parts {
        zip.start_file(name, options).map_err(package_error)?;
        zip.write_all(xml.as_bytes()).map_err(package_error)?;
    }
    Ok(zip.finish().map_err(package_error)?.into_inner())
}

fn package_error(error: impl std::error::Error + Send + Sync + 'static) -> XbergError {
    XbergError::Serialization {
        message: format!("failed to write DOCX package: {error}"),
        source: Some(Box::new(error)),
    }
}

#[derive(Default)]
struct RunStyle {
    bold: usize,
    italic: usize,
    strike: usize,
    underline: usize,
    highlight: usize,
    superscript: usize,
    subscript: usize,
    code: usize,
}

enum ParagraphKind {
    Body,
    Heading(u8),
    Code,
}

/// One list tree's numbering: the format and start value of each level it uses.
///
/// A nested list shares its parent's `numId` at a deeper `ilvl`, the way Word writes
/// lists; readers that key a list on its `numId` (xberg's own among them) otherwise see
/// every nested list as the start of a new one.
#[derive(Default)]
struct Numbering {
    levels: [Option<(ListType, usize)>; MAX_LEVEL + 1],
}

struct TableState {
    alignments: Vec<TableAlignment>,
    columns: usize,
    column: usize,
}

/// Streams WordprocessingML for one document body from comrak traversal edges.
///
/// Word allows runs only inside a paragraph and requires every table cell to end in one,
/// so block nodes close any open paragraph and inline nodes open one on demand.
#[derive(Default)]
struct BodyWriter {
    xml: String,
    paragraph_open: bool,
    hyperlink_open: bool,
    run: RunStyle,
    quote_depth: usize,
    details_depth: usize,
    /// Index into `numberings` of each list enclosing the current position, innermost last.
    lists: Vec<usize>,
    numberings: Vec<Numbering>,
    /// Per enclosing list item: true until its first paragraph has carried the list marker.
    item_marker_pending: Vec<bool>,
    /// External link targets in relationship order, and each target's index in it.
    hyperlinks: Vec<String>,
    hyperlink_index: HashMap<String, usize>,
    tables: Vec<TableState>,
    footnote_label: Option<String>,
    last_block_was_table: bool,
}

impl BodyWriter {
    fn start(&mut self, value: &NodeValue) {
        match value {
            NodeValue::Paragraph => self.open_paragraph(ParagraphKind::Body),
            NodeValue::Heading(heading) => {
                self.close_paragraph();
                self.open_paragraph(ParagraphKind::Heading(heading.level.clamp(1, 6)));
            }
            NodeValue::CodeBlock(block) => self.literal_block(&block.literal, ParagraphKind::Code),
            NodeValue::FrontMatter(text) => self.literal_block(text, ParagraphKind::Code),
            NodeValue::HtmlBlock(block) => self.literal_block(&block.literal, ParagraphKind::Body),
            NodeValue::ThematicBreak => {
                self.close_paragraph();
                self.xml.push_str(r#"<w:p><w:pPr><w:pBdr><w:bottom w:val="single" w:sz="6" w:space="1" w:color="auto"/></w:pBdr></w:pPr></w:p>"#);
                self.last_block_was_table = false;
            }
            NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_) => {
                self.close_paragraph();
                self.quote_depth += 1;
            }
            NodeValue::Alert(alert) => {
                self.close_paragraph();
                self.quote_depth += 1;
                let title = alert
                    .title
                    .clone()
                    .unwrap_or_else(|| alert.alert_type.default_title().to_string());
                self.run.bold += 1;
                self.open_paragraph(ParagraphKind::Body);
                self.text(&title);
                self.close_paragraph();
                self.run.bold -= 1;
            }
            NodeValue::List(list) => {
                self.close_paragraph();
                self.flush_item_marker();
                let numbering = self.numbering_for(list.list_type, list.start);
                self.lists.push(numbering);
            }
            NodeValue::Item(_) | NodeValue::TaskItem(_) => {
                self.close_paragraph();
                self.item_marker_pending.push(true);
            }
            NodeValue::DescriptionTerm => {
                self.close_paragraph();
                self.run.bold += 1;
            }
            NodeValue::DescriptionDetails => {
                self.close_paragraph();
                self.details_depth += 1;
            }
            NodeValue::FootnoteDefinition(definition) => {
                self.close_paragraph();
                self.footnote_label = Some(definition.name.clone());
            }
            NodeValue::Table(table) => {
                self.close_paragraph();
                self.flush_item_marker();
                self.open_table(table.alignments.clone(), table.num_columns);
            }
            NodeValue::TableRow(header) => {
                self.xml.push_str("<w:tr>");
                if *header {
                    self.xml.push_str("<w:trPr><w:tblHeader/></w:trPr>");
                }
                if let Some(table) = self.tables.last_mut() {
                    table.column = 0;
                }
            }
            NodeValue::TableCell => self.open_cell(),
            _ => self.start_inline(value),
        }
    }

    fn start_inline(&mut self, value: &NodeValue) {
        match value {
            NodeValue::Text(text) => self.text(text),
            NodeValue::SoftBreak => self.text(" "),
            NodeValue::LineBreak => {
                self.ensure_paragraph();
                self.xml.push_str("<w:r><w:br/></w:r>");
            }
            NodeValue::Code(code) => {
                self.run.code += 1;
                self.text(&code.literal);
                self.run.code -= 1;
            }
            NodeValue::Math(math) => self.text(&math.literal),
            NodeValue::HtmlInline(text) | NodeValue::Raw(text) => self.text(text),
            NodeValue::EscapedTag(text) => self.text(text),
            NodeValue::FootnoteReference(reference) => {
                self.run.superscript += 1;
                self.text(&reference.name);
                self.run.superscript -= 1;
            }
            NodeValue::Emph => self.run.italic += 1,
            NodeValue::Strong => self.run.bold += 1,
            NodeValue::Strikethrough => self.run.strike += 1,
            NodeValue::Underline | NodeValue::Insert => self.run.underline += 1,
            NodeValue::Highlight => self.run.highlight += 1,
            NodeValue::Superscript => self.run.superscript += 1,
            NodeValue::Subscript => self.run.subscript += 1,
            NodeValue::Link(link) => self.open_hyperlink(&link.url),
            _ => {}
        }
    }

    fn end(&mut self, value: &NodeValue) {
        match value {
            NodeValue::Paragraph | NodeValue::Heading(_) => self.close_paragraph(),
            NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_) | NodeValue::Alert(_) => {
                self.close_paragraph();
                self.quote_depth -= 1;
            }
            NodeValue::List(_) => {
                self.close_paragraph();
                self.lists.pop();
            }
            NodeValue::Item(_) | NodeValue::TaskItem(_) => {
                self.close_paragraph();
                self.flush_item_marker();
                self.item_marker_pending.pop();
            }
            NodeValue::DescriptionTerm => {
                self.close_paragraph();
                self.run.bold -= 1;
            }
            NodeValue::DescriptionDetails => {
                self.close_paragraph();
                self.details_depth -= 1;
            }
            NodeValue::FootnoteDefinition(_) => {
                self.close_paragraph();
                self.footnote_label = None;
            }
            NodeValue::Table(_) => {
                self.tables.pop();
                self.xml.push_str("</w:tbl>");
                self.last_block_was_table = true;
            }
            NodeValue::TableRow(_) => {
                self.pad_row();
                self.xml.push_str("</w:tr>");
            }
            NodeValue::TableCell => {
                self.ensure_paragraph();
                self.close_paragraph();
                self.xml.push_str("</w:tc>");
                if let Some(table) = self.tables.last_mut() {
                    table.column += 1;
                }
            }
            NodeValue::Emph => self.run.italic -= 1,
            NodeValue::Strong => self.run.bold -= 1,
            NodeValue::Strikethrough => self.run.strike -= 1,
            NodeValue::Underline | NodeValue::Insert => self.run.underline -= 1,
            NodeValue::Highlight => self.run.highlight -= 1,
            NodeValue::Superscript => self.run.superscript -= 1,
            NodeValue::Subscript => self.run.subscript -= 1,
            NodeValue::Link(_) => self.close_hyperlink(),
            _ => {}
        }
    }

    fn finish(&mut self) {
        self.close_paragraph();
        // Word expects the body to end in a paragraph; a trailing table otherwise gets one
        // inserted and the document is reported as modified on open.
        if self.xml.is_empty() || self.last_block_was_table {
            self.xml.push_str("<w:p/>");
        }
    }

    fn open_paragraph(&mut self, kind: ParagraphKind) {
        self.close_paragraph();
        self.paragraph_open = true;
        self.last_block_was_table = false;

        let mut properties = String::new();
        match kind {
            ParagraphKind::Heading(level) => properties.push_str(&format!(r#"<w:pStyle w:val="Heading{level}"/>"#)),
            ParagraphKind::Code => properties.push_str(r#"<w:pStyle w:val="SourceCode"/>"#),
            ParagraphKind::Body => {}
        }

        let numbered = self.take_item_marker();
        if let Some((num_id, level)) = numbered {
            properties.push_str(&format!(
                r#"<w:numPr><w:ilvl w:val="{level}"/><w:numId w:val="{num_id}"/></w:numPr>"#
            ));
        }

        let depth = if self.tables.is_empty() {
            (self.quote_depth + self.lists.len() + self.details_depth).min(MAX_LEVEL + 1)
        } else {
            0
        };
        if numbered.is_some() {
            properties.push_str(&format!(
                r#"<w:ind w:left="{}" w:hanging="{HANGING_INDENT_TWIPS}"/>"#,
                depth * INDENT_STEP_TWIPS
            ));
        } else if depth > 0 {
            properties.push_str(&format!(r#"<w:ind w:left="{}"/>"#, depth * INDENT_STEP_TWIPS));
        }

        if let Some(table) = self.tables.last() {
            match table.alignments.get(table.column) {
                Some(TableAlignment::Center) => properties.push_str(r#"<w:jc w:val="center"/>"#),
                Some(TableAlignment::Right) => properties.push_str(r#"<w:jc w:val="right"/>"#),
                _ => {}
            }
        }

        self.xml.push_str("<w:p>");
        if !properties.is_empty() {
            self.xml.push_str("<w:pPr>");
            self.xml.push_str(&properties);
            self.xml.push_str("</w:pPr>");
        }

        if let Some(label) = self.footnote_label.take() {
            self.run.superscript += 1;
            self.text(&label);
            self.run.superscript -= 1;
            self.text(" ");
        }
    }

    fn ensure_paragraph(&mut self) {
        if !self.paragraph_open {
            self.open_paragraph(ParagraphKind::Body);
        }
    }

    fn close_paragraph(&mut self) {
        if !self.paragraph_open {
            return;
        }
        self.close_hyperlink();
        self.xml.push_str("</w:p>");
        self.paragraph_open = false;
    }

    /// The list marker belongs to an item's first paragraph. Returns its `(numId, ilvl)`.
    fn take_item_marker(&mut self) -> Option<(usize, usize)> {
        let pending = self.item_marker_pending.last_mut()?;
        if !*pending {
            return None;
        }
        *pending = false;
        let numbering = *self.lists.last()?;
        Some((numbering + 1, (self.lists.len() - 1).min(MAX_LEVEL)))
    }

    /// The numbering a list opening at the current depth belongs to: its parent's when the
    /// parent's tree leaves this level free or already numbers it the same way, a new one
    /// otherwise.
    fn numbering_for(&mut self, list_type: ListType, start: usize) -> usize {
        let level = self.lists.len().min(MAX_LEVEL);
        let inherited =
            self.lists.last().copied().filter(|&index| {
                self.numberings[index].levels[level].is_none_or(|(existing, _)| existing == list_type)
            });
        let index = inherited.unwrap_or_else(|| {
            self.numberings.push(Numbering::default());
            self.numberings.len() - 1
        });
        self.numberings[index].levels[level].get_or_insert((list_type, start));
        index
    }

    /// Emit an empty marker paragraph for an item whose content does not open with a
    /// paragraph (an empty item, or one that starts with a nested list or a table), so
    /// the item keeps its bullet or number and it lands before that content.
    fn flush_item_marker(&mut self) {
        if self.item_marker_pending.last().copied().unwrap_or(false) {
            self.open_paragraph(ParagraphKind::Body);
            self.close_paragraph();
        }
    }

    fn literal_block(&mut self, literal: &str, kind: ParagraphKind) {
        self.close_paragraph();
        self.open_paragraph(kind);
        let literal = literal.strip_suffix('\n').unwrap_or(literal);
        for (index, line) in literal.split('\n').enumerate() {
            if index > 0 {
                self.xml.push_str("<w:r><w:br/></w:r>");
            }
            self.text(line);
        }
        self.close_paragraph();
    }

    fn text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.ensure_paragraph();
        self.xml.push_str("<w:r>");
        self.push_run_properties();
        let mut segments = text.split('\t');
        if let Some(first) = segments.next() {
            self.push_text_element(first);
        }
        for segment in segments {
            self.xml.push_str("<w:tab/>");
            self.push_text_element(segment);
        }
        self.xml.push_str("</w:r>");
    }

    fn push_text_element(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.xml.push_str(r#"<w:t xml:space="preserve">"#);
        push_escaped(&mut self.xml, text);
        self.xml.push_str("</w:t>");
    }

    /// Children of `w:rPr` in the order the schema requires.
    fn push_run_properties(&mut self) {
        let run = &self.run;
        let mut properties = String::new();
        if self.hyperlink_open {
            properties.push_str(r#"<w:rStyle w:val="Hyperlink"/>"#);
        }
        if run.code > 0 {
            properties.push_str(r#"<w:rFonts w:ascii="Courier New" w:hAnsi="Courier New" w:cs="Courier New"/>"#);
        }
        if run.bold > 0 {
            properties.push_str("<w:b/>");
        }
        if run.italic > 0 {
            properties.push_str("<w:i/>");
        }
        if run.strike > 0 {
            properties.push_str("<w:strike/>");
        }
        if run.highlight > 0 {
            properties.push_str(r#"<w:highlight w:val="yellow"/>"#);
        }
        if run.underline > 0 {
            properties.push_str(r#"<w:u w:val="single"/>"#);
        }
        if run.superscript > 0 {
            properties.push_str(r#"<w:vertAlign w:val="superscript"/>"#);
        } else if run.subscript > 0 {
            properties.push_str(r#"<w:vertAlign w:val="subscript"/>"#);
        }
        if !properties.is_empty() {
            self.xml.push_str("<w:rPr>");
            self.xml.push_str(&properties);
            self.xml.push_str("</w:rPr>");
        }
    }

    fn open_hyperlink(&mut self, url: &str) {
        if self.hyperlink_open {
            return;
        }
        let Some(target) = external_link_target(url) else {
            return;
        };
        self.ensure_paragraph();
        let next = self.hyperlinks.len();
        let index = *self.hyperlink_index.entry(target.clone()).or_insert(next);
        if index == next {
            self.hyperlinks.push(target);
        }
        self.xml.push_str(&format!(
            r#"<w:hyperlink r:id="rId{}" w:history="1">"#,
            FIRST_HYPERLINK_REL_ID + index
        ));
        self.hyperlink_open = true;
    }

    fn close_hyperlink(&mut self) {
        if self.hyperlink_open {
            self.xml.push_str("</w:hyperlink>");
            self.hyperlink_open = false;
        }
    }

    fn open_table(&mut self, alignments: Vec<TableAlignment>, columns: usize) {
        let columns = columns.max(1);
        let width = TABLE_WIDTH_TWIPS / columns;
        self.xml.push_str(r#"<w:tbl><w:tblPr><w:tblStyle w:val="TableGrid"/><w:tblW w:w="5000" w:type="pct"/><w:tblLook w:val="0620" w:firstRow="1" w:lastRow="0" w:firstColumn="0" w:lastColumn="0" w:noHBand="1" w:noVBand="1"/></w:tblPr><w:tblGrid>"#);
        for _ in 0..columns {
            self.xml.push_str(&format!(r#"<w:gridCol w:w="{width}"/>"#));
        }
        self.xml.push_str("</w:tblGrid>");
        self.tables.push(TableState {
            alignments,
            columns,
            column: 0,
        });
    }

    fn open_cell(&mut self) {
        let width = self
            .tables
            .last()
            .map_or(TABLE_WIDTH_TWIPS, |table| TABLE_WIDTH_TWIPS / table.columns);
        self.xml.push_str(&format!(
            r#"<w:tc><w:tcPr><w:tcW w:w="{width}" w:type="dxa"/></w:tcPr>"#
        ));
        self.open_paragraph(ParagraphKind::Body);
    }

    /// GFM lets a body row hold fewer cells than the header; Word needs the grid filled.
    fn pad_row(&mut self) {
        let missing = self
            .tables
            .last()
            .map_or(0, |table| table.columns.saturating_sub(table.column));
        for _ in 0..missing {
            self.open_cell();
            self.close_paragraph();
            self.xml.push_str("</w:tc>");
            if let Some(table) = self.tables.last_mut() {
                table.column += 1;
            }
        }
    }

    fn document_xml(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="{WORDPROCESSINGML_NS}" xmlns:r="{RELATIONSHIPS_NS}"><w:body>{}</w:body></w:document>"#,
            self.xml
        )
    }

    fn document_rels_xml(&self) -> String {
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/>"#,
        );
        for (index, url) in self.hyperlinks.iter().enumerate() {
            xml.push_str(&format!(
                r#"<Relationship Id="rId{}" Type="{HYPERLINK_REL_TYPE}" Target=""#,
                FIRST_HYPERLINK_REL_ID + index
            ));
            push_escaped(&mut xml, url);
            xml.push_str(r#"" TargetMode="External"/>"#);
        }
        xml.push_str("</Relationships>");
        xml
    }

    fn numbering_xml(&self) -> String {
        let mut xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="{WORDPROCESSINGML_NS}">"#
        );
        for (index, numbering) in self.numberings.iter().enumerate() {
            xml.push_str(&format!(
                r#"<w:abstractNum w:abstractNumId="{index}"><w:multiLevelType w:val="multilevel"/>"#
            ));
            for (level, definition) in numbering.levels.iter().enumerate() {
                let (list_type, start) = definition.unwrap_or((ListType::Bullet, 1));
                let (num_format, text) = match list_type {
                    ListType::Bullet => ("bullet", BULLET_GLYPHS[level % BULLET_GLYPHS.len()].to_string()),
                    ListType::Ordered => ("decimal", format!("%{}.", level + 1)),
                };
                xml.push_str(&format!(
                    r#"<w:lvl w:ilvl="{level}"><w:start w:val="{start}"/><w:numFmt w:val="{num_format}"/><w:lvlText w:val="{text}"/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="{}" w:hanging="{HANGING_INDENT_TWIPS}"/></w:pPr></w:lvl>"#,
                    (level + 1) * INDENT_STEP_TWIPS
                ));
            }
            xml.push_str("</w:abstractNum>");
        }
        // Every `w:abstractNum` must precede the first `w:num`.
        for index in 0..self.numberings.len() {
            xml.push_str(&format!(
                r#"<w:num w:numId="{}"><w:abstractNumId w:val="{index}"/></w:num>"#,
                index + 1
            ));
        }
        xml.push_str("</w:numbering>");
        xml
    }
}

/// Escape `text` for XML character data or an attribute value, dropping the control
/// characters XML 1.0 cannot represent at all. Word refuses to open a part that holds one.
fn push_escaped(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'.. => out.push(c),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn parts(markdown: &str) -> Vec<(String, String)> {
        let bytes = render_docx(markdown).expect("the package should build");
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("the package should be a zip archive");
        (0..archive.len())
            .map(|index| {
                let mut entry = archive.by_index(index).expect("every entry should be readable");
                let mut xml = String::new();
                entry.read_to_string(&mut xml).expect("every part should be UTF-8");
                (entry.name().expect("every entry name should decode").into_owned(), xml)
            })
            .collect()
    }

    fn part(markdown: &str, name: &str) -> String {
        parts(markdown)
            .into_iter()
            .find(|(part_name, _)| part_name == name)
            .map(|(_, xml)| xml)
            .unwrap_or_else(|| panic!("the package should contain {name}"))
    }

    fn body_text(document_xml: &str) -> String {
        let document = roxmltree::Document::parse(document_xml).expect("document.xml should be well-formed");
        document
            .descendants()
            .filter(|node| node.has_tag_name((WORDPROCESSINGML_NS, "t")))
            .filter_map(|node| node.text())
            .collect()
    }

    #[test]
    fn should_write_every_package_part_as_well_formed_xml() {
        let markdown = "# Title\n\nText with **bold** and a [link](https://example.com).\n\n- a\n  1. b\n\n| x | y |\n| --- | --- |\n| 1 | 2 |\n";
        let parts = parts(markdown);

        let names: Vec<&str> = parts.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            [
                "[Content_Types].xml",
                "_rels/.rels",
                "word/_rels/document.xml.rels",
                "word/document.xml",
                "word/styles.xml",
                "word/numbering.xml",
            ]
        );
        for (name, xml) in &parts {
            if let Err(error) = roxmltree::Document::parse(xml) {
                panic!("{name} is not well-formed XML: {error}\n{xml}");
            }
        }
    }

    #[test]
    fn should_escape_markup_and_drop_characters_xml_cannot_hold() {
        let document = part("a < b & \"c\" 'd' \u{1}e\u{FFFE}f\n", "word/document.xml");
        assert_eq!(body_text(&document), "a < b & \"c\" 'd' ef");
    }

    #[test]
    fn should_map_headings_to_the_heading_styles_the_docx_reader_recognises() {
        let document = part("# One\n\n###### Six\n", "word/document.xml");
        assert!(document.contains(r#"<w:pStyle w:val="Heading1"/>"#), "{document}");
        assert!(document.contains(r#"<w:pStyle w:val="Heading6"/>"#), "{document}");
    }

    #[test]
    fn should_link_external_targets_and_keep_the_text_of_fragment_links() {
        let markdown = "[site](https://example.com/a?b=1&c=2) and [section](#local)\n";
        let document = part(markdown, "word/document.xml");
        let rels = part(markdown, "word/_rels/document.xml.rels");

        assert_eq!(document.matches("<w:hyperlink ").count(), 1, "{document}");
        assert!(document.contains(r#"<w:hyperlink r:id="rId3""#), "{document}");
        assert!(
            rels.contains(r#"Target="https://example.com/a?b=1&amp;c=2" TargetMode="External""#),
            "{rels}"
        );
        assert_eq!(body_text(&document), "site and section");
    }

    #[test]
    fn should_number_a_nested_list_under_its_parent_list() {
        let markdown = "3. first\n   - inner\n4. second\n";
        let document = part(markdown, "word/document.xml");
        let numbering = part(markdown, "word/numbering.xml");

        assert!(
            document.contains(r#"<w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr>"#),
            "{document}"
        );
        assert!(
            document.contains(r#"<w:numPr><w:ilvl w:val="1"/><w:numId w:val="1"/></w:numPr>"#),
            "nested items share the parent's numId one level down: {document}"
        );
        assert_eq!(numbering.matches("<w:num ").count(), 1, "{numbering}");
        assert!(
            numbering.contains(r#"<w:lvl w:ilvl="0"><w:start w:val="3"/><w:numFmt w:val="decimal"/>"#),
            "{numbering}"
        );
        assert!(
            numbering.contains(r#"<w:lvl w:ilvl="1"><w:start w:val="1"/><w:numFmt w:val="bullet"/>"#),
            "{numbering}"
        );
    }

    #[test]
    fn should_give_sibling_sublists_of_different_kinds_their_own_numbering() {
        let markdown = "- a\n  1. ordered\n- b\n  - bullet\n";
        let numbering = part(markdown, "word/numbering.xml");
        assert_eq!(numbering.matches("<w:num ").count(), 2, "{numbering}");
    }

    #[test]
    fn should_keep_the_marker_of_an_item_that_opens_with_a_nested_list() {
        let document = part("- - inner\n", "word/document.xml");
        let first = document
            .find(r#"<w:ilvl w:val="0"/>"#)
            .expect("the outer item keeps its marker");
        let second = document
            .find(r#"<w:ilvl w:val="1"/>"#)
            .expect("the inner item has its own");
        assert!(first < second, "the outer marker precedes the nested list: {document}");
    }

    #[test]
    fn should_pad_short_table_rows_to_the_column_count() {
        let document = part("| a | b | c |\n| --- | --- | --- |\n| only |\n", "word/document.xml");
        let xml = roxmltree::Document::parse(&document).expect("document.xml should be well-formed");
        let cells_per_row: Vec<usize> = xml
            .descendants()
            .filter(|node| node.has_tag_name((WORDPROCESSINGML_NS, "tr")))
            .map(|row| {
                row.children()
                    .filter(|node| node.has_tag_name((WORDPROCESSINGML_NS, "tc")))
                    .count()
            })
            .collect();
        assert_eq!(cells_per_row, [3, 3]);
        assert!(
            document.ends_with("</w:tbl><w:p/></w:body></w:document>"),
            "a trailing table is followed by a paragraph: {document}"
        );
    }

    #[test]
    fn should_write_a_valid_document_for_empty_input() {
        let document = part("", "word/document.xml");
        assert!(document.contains("<w:body><w:p/></w:body>"), "{document}");
    }

    #[test]
    fn should_produce_identical_bytes_for_identical_input() {
        let markdown = "# Same\n\nInput.\n";
        assert_eq!(render_docx(markdown).unwrap(), render_docx(markdown).unwrap());
    }
}
