use lopdf::content::Content;
use lopdf::{Dictionary, Document, Object};

use super::blocks::parse;
use super::font::{Face, Metrics};
use super::layout::{PAGE_WIDTH, TextRun, lay_out};
use super::render_pdf;

const MARGIN: f32 = 72.0;

fn load(markdown: &str) -> Document {
    let bytes = render_pdf(markdown).expect("the document should build");
    assert!(bytes.starts_with(b"%PDF-1.7"), "not a PDF file");
    Document::load_mem(&bytes).expect("the document should parse")
}

/// Each page's text as lopdf extracts it through the fonts' `ToUnicode` maps, with runs of
/// whitespace collapsed.
fn page_texts(document: &Document) -> Vec<String> {
    (1..=document.get_pages().len() as u32)
        .map(|page| {
            let text = document.extract_text(&[page]).expect("the page text should extract");
            text.split_whitespace().collect::<Vec<_>>().join(" ")
        })
        .collect()
}

fn text(markdown: &str) -> String {
    page_texts(&load(markdown)).join(" ")
}

fn fonts(document: &Document) -> Vec<&Dictionary> {
    document
        .objects
        .values()
        .filter_map(|object| object.as_dict().ok())
        .filter(|dictionary| dictionary.get(b"Type").and_then(Object::as_name).ok() == Some(b"Font".as_slice()))
        .collect()
}

fn name<'a>(dictionary: &'a Dictionary, key: &[u8]) -> &'a [u8] {
    dictionary.get(key).and_then(Object::as_name).unwrap_or_default()
}

fn stream(document: &Document, dictionary: &Dictionary, key: &[u8]) -> Vec<u8> {
    let id = dictionary
        .get(key)
        .and_then(Object::as_reference)
        .expect("a stream reference");
    let stream = document.get_object(id).and_then(Object::as_stream).expect("a stream");
    stream.decompressed_content().unwrap_or_else(|_| stream.content.clone())
}

fn sans_font(document: &Document) -> (&Dictionary, &Dictionary) {
    let type0 = fonts(document)
        .into_iter()
        .find(|font| name(font, b"Subtype") == b"Type0")
        .expect("the embedded DejaVu Sans font");
    let descendants = type0
        .get(b"DescendantFonts")
        .and_then(Object::as_array)
        .expect("descendants");
    let descendant = document
        .get_object(descendants[0].as_reference().expect("a reference"))
        .and_then(Object::as_dict)
        .expect("the CID font");
    (type0, descendant)
}

fn all_runs(markdown: &str) -> Vec<TextRun> {
    let layout = lay_out(&parse(markdown)).expect("the layout should succeed");
    layout.pages.into_iter().flat_map(|page| page.texts).collect()
}

fn run_width(metrics: &Metrics, run: &TextRun) -> f32 {
    run.text.chars().map(|c| metrics.advance(run.face, c)).sum::<f32>() * run.size / 1000.0
}

#[test]
fn should_write_a_single_page_for_empty_input() {
    let document = load("");
    assert_eq!(document.get_pages().len(), 1);
    let catalog = document.catalog().expect("a catalog");
    assert!(matches!(catalog.get(b"Pages"), Ok(Object::Reference(_))));
}

#[test]
fn should_keep_text_extractable_in_reading_order() {
    let extracted = text("# Title\n\nFirst paragraph with **bold**, *italic* and `code`.\n\n- one\n- two\n");
    assert_eq!(
        extracted,
        "Title First paragraph with bold, italic and code. \u{2022} one \u{2022} two"
    );
}

#[test]
fn should_embed_dejavu_sans_as_a_subset_type0_font_with_a_to_unicode_map() {
    let document = load("Hello Ελληνικά and Русский.\n");
    let (type0, descendant) = sans_font(&document);

    assert_eq!(name(type0, b"Encoding"), b"Identity-H");
    let base_font = std::str::from_utf8(name(type0, b"BaseFont")).unwrap();
    assert_eq!(base_font.len(), "ABCDEF+DejaVuSans".len(), "{base_font}");
    assert!(base_font.ends_with("+DejaVuSans"), "{base_font}");
    assert!(
        base_font[..6].bytes().all(|byte| byte.is_ascii_uppercase()),
        "{base_font}"
    );
    assert_eq!(name(descendant, b"Subtype"), b"CIDFontType2");

    let descriptor = document
        .get_object(
            descendant
                .get(b"FontDescriptor")
                .and_then(Object::as_reference)
                .unwrap(),
        )
        .and_then(Object::as_dict)
        .unwrap();
    let font_file_id = descriptor.get(b"FontFile2").and_then(Object::as_reference).unwrap();
    let font_file = document.get_object(font_file_id).and_then(Object::as_stream).unwrap();
    let subset = font_file.decompressed_content().unwrap();
    assert_eq!(
        font_file.dict.get(b"Length1").and_then(Object::as_i64).unwrap(),
        subset.len() as i64
    );
    assert!(
        subset.len() < xberg_native_pdf::fonts::bundled::DEJAVU_SANS.len() / 10,
        "the font should be subset, got {} bytes",
        subset.len()
    );

    let cmap = String::from_utf8(stream(&document, type0, b"ToUnicode")).unwrap();
    assert!(cmap.contains("<0395>"), "Ε is mapped back: {cmap}");
    assert!(cmap.contains("<0420>"), "Р is mapped back: {cmap}");
}

#[test]
fn should_keep_characters_without_a_glyph_extractable() {
    let document = load("Before 日本 🎉 after\n");
    assert_eq!(page_texts(&document), ["Before 日本 🎉 after"]);

    let (type0, descendant) = sans_font(&document);
    let cmap = String::from_utf8(stream(&document, type0, b"ToUnicode")).unwrap();
    assert!(cmap.contains("<65E5>"), "日 keeps its own code: {cmap}");
    assert!(cmap.contains("<D83CDF89>"), "🎉 maps to its surrogate pair: {cmap}");

    // Codes follow first use: B e f o r (5), space (6), 日 (7).
    let cid_to_gid = stream(&document, descendant, b"CIDToGIDMap");
    let glyph = |code: usize| u16::from_be_bytes([cid_to_gid[2 * code], cid_to_gid[2 * code + 1]]);
    assert_ne!(glyph(1), 0, "B has a glyph");
    assert_eq!(glyph(7), 0, "日 has no glyph in DejaVu Sans and draws as .notdef");
}

#[test]
fn should_set_code_in_courier_with_explicit_widths() {
    let document = load("```\nlet x = 1;\n```\n");
    let courier = fonts(&document)
        .into_iter()
        .find(|font| name(font, b"BaseFont") == b"Courier")
        .expect("a Courier font");
    assert_eq!(name(courier, b"Encoding"), b"WinAnsiEncoding");
    let widths = courier.get(b"Widths").and_then(Object::as_array).unwrap();
    assert_eq!(widths.len(), 224);
    assert!(widths.iter().all(|width| width.as_i64().ok() == Some(600)));
    assert!(
        fonts(&document).iter().all(|font| name(font, b"Subtype") != b"Type0"),
        "no DejaVu Sans without text that needs it"
    );
}

#[test]
fn should_draw_code_characters_courier_cannot_encode_in_dejavu_sans() {
    let runs = all_runs("`a λ b`\n");
    let faces: Vec<(Face, &str)> = runs.iter().map(|run| (run.face, run.text.as_str())).collect();
    let courier = Face::Courier {
        bold: false,
        italic: false,
    };
    assert_eq!(faces, [(courier, "a "), (Face::Sans, "λ"), (courier, " b")]);
    assert_eq!(text("`a λ b`\n"), "a λ b");
}

#[test]
fn should_set_code_math_as_text_rather_than_code() {
    let markdown = "Let $`a<b`$ hold.\n\n```math\nx^2\n```\n";
    assert!(all_runs(markdown).iter().all(|run| run.face == Face::Sans));
    assert_eq!(text(markdown), "Let a<b hold. x^2");

    let runs = all_runs("```math\nx^2\ny^2\n```\n");
    let lines: Vec<(&str, Face)> = runs.iter().map(|run| (run.text.as_str(), run.face)).collect();
    assert_eq!(lines, [("x^2", Face::Sans), ("y^2", Face::Sans)]);
    assert_ne!(runs[0].y, runs[1].y, "each line of the fence is its own line");
}

#[test]
fn should_draw_bold_and_italic_from_the_regular_face() {
    let runs = all_runs("**bold** *italic* plain\n");
    let flags: Vec<(&str, bool, bool)> = runs
        .iter()
        .map(|run| (run.text.as_str(), run.synthetic_bold, run.synthetic_italic))
        .collect();
    assert_eq!(
        flags,
        [
            ("bold", true, false),
            (" ", false, false),
            ("italic", false, true),
            (" plain", false, false)
        ]
    );
}

#[test]
fn should_raise_a_superscript_without_moving_it_off_its_line() {
    let runs = all_runs("x^2^\n");
    assert_eq!(runs.len(), 2, "{runs:?}");
    assert_eq!(runs[0].y, runs[1].y, "both runs share the line's baseline");
    assert!(runs[1].rise > 0.0);

    let document = load("x^2^\n");
    let page = document.get_pages()[&1];
    let content = Content::decode(&document.get_page_content(page)).unwrap();
    assert!(content.operations.iter().any(|operation| operation.operator == "Ts"));
}

#[test]
fn should_keep_every_run_inside_the_margins() {
    let long_word = "x".repeat(400);
    let markdown = format!(
        "{long_word}\n\n- a\n  - b\n    - c {long_word}\n\n| a | b |\n| --- | --- |\n| {long_word} | y |\n\n```\n{long_word}\n```\n"
    );
    let metrics = Metrics::load().unwrap();
    for run in all_runs(&markdown) {
        assert!(run.x >= MARGIN - 0.01, "{run:?} starts left of the margin");
        let right = run.x + run_width(&metrics, &run);
        assert!(right <= PAGE_WIDTH - MARGIN + 0.01, "{run:?} ends at {right}");
    }
}

#[test]
fn should_expand_tabs_and_keep_code_indentation() {
    let runs = all_runs("```\n\tx\n  y\n```\n");
    let lines: Vec<&str> = runs.iter().map(|run| run.text.as_str()).collect();
    assert_eq!(lines, ["    x", "  y"]);
}

#[test]
fn should_wrap_a_long_code_line_at_a_space_and_keep_every_character() {
    let line = format!("{} tail", "word ".repeat(30));
    let runs = all_runs(&format!("```\n{line}\n```\n"));
    assert!(runs.len() > 1, "{runs:?}");
    for run in &runs[..runs.len() - 1] {
        assert!(run.text.ends_with(' '), "{:?} breaks inside a word", run.text);
    }
    assert_eq!(runs.iter().map(|run| run.text.as_str()).collect::<String>(), line);

    let unbroken = "x".repeat(200);
    let runs = all_runs(&format!("```\n{unbroken}\n```\n"));
    assert!(runs.len() > 1);
    assert_eq!(runs.iter().map(|run| run.text.as_str()).collect::<String>(), unbroken);
}

#[test]
fn should_link_external_targets_and_keep_the_text_of_fragment_links() {
    let document = load("[site](https://example.com/a?b=1&c=2) and [section](#local)\n");
    let page = document.get_pages()[&1];
    let annotations = document.get_page_annotations(page).expect("the page's annotations");
    assert_eq!(annotations.len(), 1);
    let action = annotations[0].get(b"A").and_then(Object::as_dict).unwrap();
    assert_eq!(
        action.get(b"URI").and_then(Object::as_str).unwrap(),
        b"https://example.com/a?b=1&c=2"
    );
    assert_eq!(page_texts(&document), ["site and section"]);
}

#[test]
fn should_list_headings_in_a_nested_outline() {
    let document = load("# One\n\n## Two\n\n### Three\n\n## Four\n\n# Fünf\n");
    let catalog = document.catalog().unwrap();
    let root = document
        .get_object(catalog.get(b"Outlines").and_then(Object::as_reference).unwrap())
        .and_then(Object::as_dict)
        .unwrap();
    assert_eq!(root.get(b"Count").and_then(Object::as_i64).unwrap(), 5);

    let entry = |id: &Object| {
        document
            .get_object(id.as_reference().unwrap())
            .and_then(Object::as_dict)
            .unwrap()
    };
    let title = |entry: &Dictionary| {
        let bytes = entry.get(b"Title").and_then(Object::as_str).unwrap();
        if let Some(utf16) = bytes.strip_prefix(&[0xFE, 0xFF]) {
            let units: Vec<u16> = utf16
                .chunks(2)
                .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
                .collect();
            String::from_utf16(&units).unwrap()
        } else {
            String::from_utf8(bytes.to_vec()).unwrap()
        }
    };

    let one = entry(root.get(b"First").unwrap());
    assert_eq!(title(one), "One");
    assert_eq!(one.get(b"Count").and_then(Object::as_i64).unwrap(), 3);
    let two = entry(one.get(b"First").unwrap());
    assert_eq!(title(two), "Two");
    assert_eq!(title(entry(two.get(b"First").unwrap())), "Three");
    assert_eq!(title(entry(two.get(b"Next").unwrap())), "Four");
    let last = entry(root.get(b"Last").unwrap());
    assert_eq!(title(last), "Fünf");
    assert_eq!(title(entry(last.get(b"Prev").unwrap())), "One");
}

#[test]
fn should_break_long_text_across_pages() {
    let document = load(&"A sentence that fills the page. ".repeat(1500));
    let pages = page_texts(&document);
    assert!(pages.len() > 3, "{} pages", pages.len());
    let sentences = pages.join(" ").matches("A sentence that fills the page.").count();
    assert_eq!(sentences, 1500);
}

#[test]
fn should_repeat_the_header_row_on_every_page_a_table_continues_onto() {
    let rows: String = (1..=200).map(|row| format!("| {row} | value {row} |\n")).collect();
    let document = load(&format!("| Key | Value |\n| --- | --- |\n{rows}"));
    let pages = page_texts(&document);
    assert!(pages.len() > 1);
    for page in &pages {
        assert!(page.starts_with("Key Value"), "{page}");
    }
    assert!(pages.concat().contains("200 value 200"));
}

#[test]
fn should_split_a_row_taller_than_a_page() {
    let cell = "word ".repeat(3000);
    let document = load(&format!("| Long |\n| --- |\n| {cell} |\n"));
    let pages = page_texts(&document);
    assert!(pages.len() > 2, "{} pages", pages.len());
    let words = pages.iter().map(|page| page.matches("word").count()).sum::<usize>();
    assert_eq!(words, 3000);
}

#[test]
fn should_drop_control_characters() {
    assert_eq!(text("a\u{1}b\u{7}c\n"), "abc");
}

#[test]
fn should_write_no_document_properties_beyond_the_producer() {
    let document = load("# Secret title\n");
    let info = document
        .get_object(document.trailer.get(b"Info").and_then(Object::as_reference).unwrap())
        .and_then(Object::as_dict)
        .unwrap();
    let keys: Vec<&[u8]> = info.iter().map(|(key, _)| key.as_slice()).collect();
    assert_eq!(keys, [b"Producer".as_slice()]);
}

#[test]
fn should_produce_identical_bytes_for_identical_input() {
    let markdown = "# Same\n\nInput with [a link](https://example.com) and Ελληνικά.\n";
    assert_eq!(render_pdf(markdown).unwrap(), render_pdf(markdown).unwrap());
}

/// The coverage the PDF output guide describes.
#[test]
fn should_have_the_glyph_coverage_the_guide_describes() {
    let metrics = Metrics::load().unwrap();
    for c in ['A', 'é', 'ệ', 'λ', 'Ж', 'א', 'ب'] {
        assert!(metrics.has_glyph(c), "{c} should have a glyph");
    }
    for c in ['日', '한', 'न', '🎉'] {
        assert!(!metrics.has_glyph(c), "{c} should have no glyph");
    }
}
