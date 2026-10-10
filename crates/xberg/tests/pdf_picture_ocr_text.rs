//! The OCR text of a picture on a PDF page that also has a text layer.
//!
//! Each fixture is a PDF built here: pages with Helvetica text, and one grey picture that covers
//! a small part of a page. A stub OCR backend returns fixed words for the picture and other
//! fixed text for a page render. The picture is not as large as any page render, so the stub
//! tells the two calls apart by the image size, and counts each.

#![cfg(all(feature = "pdf", feature = "ocr"))]

mod helpers;

use async_trait::async_trait;
use helpers::extract_bytes_document_blocking;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use xberg::ExtractedDocument;
use xberg::core::config::{ExtractionConfig, ImageExtractionConfig, OcrConfig, OutputFormat, PageConfig, PdfConfig};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};

/// Picture size in pixels. No page render has this size.
const PICTURE_WIDTH_PX: u32 = 300;
const PICTURE_HEIGHT_PX: u32 = 120;
/// The words the stub reads from the picture.
const PICTURE_WORDS: &str = "CRATE 17 KEEP DRY UNLOAD AT GATE B";
/// The first line of text on every page that has a text layer.
const NATIVE_HEADING: &str = "Warehouse report";
/// A line of page text that has the form of a Markdown image reference.
const IMAGE_REFERENCE_LINE: &str = "![label](label.png)";

/// Where a page paints the picture: width, height, x and y in points on a 595 x 842 page.
type PictureBox = [u32; 4];
/// A picture that covers a small part of the page, below the text.
const SMALL_PICTURE: PictureBox = [360, 144, 72, 500];
/// A picture that covers about a third of the page.
const LARGE_PICTURE: PictureBox = [400, 400, 100, 300];

const FIRST_PAGE_LINES: &[&str] = &[
    NATIVE_HEADING,
    "This paragraph is real text in the text layer of the page.",
    "A reader can select it and copy it.",
    "The label below is a picture and its words are pixels.",
];
const OTHER_PAGE_LINES: &[&str] = &[
    NATIVE_HEADING,
    "This page lists the pallets that left the depot on Tuesday.",
    "Every pallet has a number and a destination.",
    "The driver signs the list at the gate.",
];

struct PageSpec {
    /// The text at the top of the page.
    lines: &'static [&'static str],
    picture: Option<PictureBox>,
    /// The text below the place of the picture.
    lines_below: &'static [&'static str],
}

/// `line` as the body of a PDF literal string.
fn pdf_string(line: &str) -> String {
    line.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)")
}

/// A PDF with one page for each spec. Object 3 is the font and object 4 is the picture.
fn pdf(pages: &[PageSpec]) -> Vec<u8> {
    let kids = (0..pages.len())
        .map(|index| format!("{} 0 R", 5 + 2 * index))
        .collect::<Vec<_>>()
        .join(" ");
    let mut objects: Vec<Vec<u8>> = vec![
        b"<</Type /Catalog /Pages 2 0 R>>".to_vec(),
        format!("<</Type /Pages /Kids [{kids}] /Count {}>>", pages.len()).into_bytes(),
        b"<</Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding>>".to_vec(),
    ];

    let raster = vec![0xA0u8; (PICTURE_WIDTH_PX * PICTURE_HEIGHT_PX) as usize];
    let mut picture = format!(
        "<</Type /XObject /Subtype /Image /Width {PICTURE_WIDTH_PX} /Height {PICTURE_HEIGHT_PX} \
         /ColorSpace /DeviceGray /BitsPerComponent 8 /Length {}>>\nstream\n",
        raster.len()
    )
    .into_bytes();
    picture.extend_from_slice(&raster);
    picture.extend_from_slice(b"\nendstream");
    objects.push(picture);

    for (index, page) in pages.iter().enumerate() {
        let mut content = String::new();
        if !page.lines.is_empty() {
            content.push_str("BT /F1 12 Tf 18 TL 72 770 Td\n");
            for line in page.lines {
                content.push_str(&format!("({}) Tj T*\n", pdf_string(line)));
            }
            content.push_str("ET\n");
        }
        if let Some([width, height, x, y]) = page.picture {
            content.push_str(&format!("q {width} 0 0 {height} {x} {y} cm /Im0 Do Q\n"));
        }
        if !page.lines_below.is_empty() {
            content.push_str("BT /F1 12 Tf 18 TL 72 300 Td\n");
            for line in page.lines_below {
                content.push_str(&format!("({}) Tj T*\n", pdf_string(line)));
            }
            content.push_str("ET\n");
        }
        objects.push(
            format!(
                "<</Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents {} 0 R \
                 /Resources <</Font <</F1 3 0 R>> /XObject <</Im0 4 0 R>> >> >>",
                6 + 2 * index
            )
            .into_bytes(),
        );
        objects.push(format!("<</Length {}>>\nstream\n{content}endstream", content.len()).into_bytes());
    }

    let mut buf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(buf.len());
        buf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        buf.extend_from_slice(object);
        buf.extend_from_slice(b"\nendobj\n");
    }
    let xref_offset = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
    for offset in &offsets {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<</Size {} /Root 1 0 R>>\n", offsets.len() + 1).as_bytes());
    buf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());
    buf
}

/// One page with a text layer and a small picture: the page of the report.
fn text_page_with_picture() -> Vec<u8> {
    pdf(&[PageSpec {
        lines: FIRST_PAGE_LINES,
        picture: Some(SMALL_PICTURE),
        lines_below: &[],
    }])
}

/// The page of the report with one more line of text, alone below the picture: an image
/// reference.
fn text_page_with_picture_and_image_reference_line() -> Vec<u8> {
    pdf(&[PageSpec {
        lines: FIRST_PAGE_LINES,
        picture: Some(SMALL_PICTURE),
        lines_below: &[IMAGE_REFERENCE_LINE],
    }])
}

/// Two pages with a text layer each, and the picture on the page with this number.
fn two_text_pages_with_picture_on(page_number: usize) -> Vec<u8> {
    pdf(&[
        PageSpec {
            lines: FIRST_PAGE_LINES,
            picture: (page_number == 1).then_some(SMALL_PICTURE),
            lines_below: &[],
        },
        PageSpec {
            lines: OTHER_PAGE_LINES,
            picture: (page_number == 2).then_some(SMALL_PICTURE),
            lines_below: &[],
        },
    ])
}

/// The width and height from a PNG header, or `None` for bytes that are not a PNG.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((width, height))
}

/// What the stub returns for the picture and for a page render.
#[derive(Clone, Copy)]
struct Replies {
    picture: &'static str,
    page_render: &'static str,
}

/// The stub reads the words from the picture and nothing from a page render.
const WORDS_FROM_THE_PICTURE: Replies = Replies {
    picture: PICTURE_WORDS,
    page_render: "",
};

struct StubBackend {
    name: &'static str,
    replies: Replies,
    picture_calls: Arc<AtomicUsize>,
    page_render_calls: Arc<AtomicUsize>,
}

impl Plugin for StubBackend {
    fn name(&self) -> &str {
        self.name
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> xberg::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> xberg::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl OcrBackend for StubBackend {
    async fn process_image(&self, image_bytes: &[u8], _config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        let is_picture = png_size(image_bytes) == Some((PICTURE_WIDTH_PX, PICTURE_HEIGHT_PX));
        let (calls, reply) = if is_picture {
            (&self.picture_calls, self.replies.picture)
        } else {
            (&self.page_render_calls, self.replies.page_render)
        };
        calls.fetch_add(1, Ordering::SeqCst);
        let mut document = ExtractedDocument::default();
        document.content = reply.to_string();
        Ok(document)
    }
    fn supports_language(&self, _language: &str) -> bool {
        true
    }
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

struct BackendGuard(&'static str);

impl Drop for BackendGuard {
    fn drop(&mut self) {
        let _ = unregister_ocr_backend(self.0);
    }
}

/// One extraction and the OCR calls it made.
struct Run {
    document: ExtractedDocument,
    picture_calls: usize,
    page_render_calls: usize,
}

impl Run {
    fn page_content(&self, page_number: u32) -> &str {
        self.document
            .pages
            .as_ref()
            .expect("page extraction is on, so the result has pages")
            .iter()
            .find(|page| page.page_number == page_number)
            .unwrap_or_else(|| panic!("the result has no page {page_number}"))
            .content
            .as_str()
    }

    /// The text of every hierarchy block, for each page.
    fn hierarchy_block_texts(&self) -> Vec<Vec<&str>> {
        self.document
            .pages
            .as_ref()
            .expect("page extraction is on, so the result has pages")
            .iter()
            .map(|page| {
                page.hierarchy
                    .iter()
                    .flat_map(|hierarchy| hierarchy.blocks.iter().map(|block| block.text.as_str()))
                    .collect()
            })
            .collect()
    }

    fn page_contents(&self) -> Vec<&str> {
        self.document
            .pages
            .as_ref()
            .expect("page extraction is on, so the result has pages")
            .iter()
            .map(|page| page.content.as_str())
            .collect()
    }
}

/// How many times the picture's words are in `text`.
fn words_in(text: &str) -> usize {
    text.matches(PICTURE_WORDS).count()
}

/// An `ocr` block for the stub named `backend`, page extraction on, and no `images` block.
fn ocr_config(backend: &str) -> ExtractionConfig {
    ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: backend.to_string(),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    }
}

/// Extract `pdf` with a stub backend registered as `backend`. `configure` changes the config
/// that [`ocr_config`] gives.
fn extract(backend: &'static str, pdf: &[u8], replies: Replies, configure: impl FnOnce(&mut ExtractionConfig)) -> Run {
    let picture_calls = Arc::new(AtomicUsize::new(0));
    let page_render_calls = Arc::new(AtomicUsize::new(0));
    let _ = unregister_ocr_backend(backend);
    register_ocr_backend(Arc::new(StubBackend {
        name: backend,
        replies,
        picture_calls: Arc::clone(&picture_calls),
        page_render_calls: Arc::clone(&page_render_calls),
    }))
    .expect("the stub backend registers");
    let _guard = BackendGuard(backend);

    let mut config = ocr_config(backend);
    configure(&mut config);
    let document = extract_bytes_document_blocking(pdf, "application/pdf", &config).expect("the PDF extracts");
    Run {
        document,
        picture_calls: picture_calls.load(Ordering::SeqCst),
        page_render_calls: page_render_calls.load(Ordering::SeqCst),
    }
}

/// The same extraction with OCR of embedded pictures off: the reference for "no picture words".
fn extract_without_picture_ocr(
    backend: &'static str,
    pdf: &[u8],
    replies: Replies,
    configure: impl FnOnce(&mut ExtractionConfig),
) -> Run {
    extract(backend, pdf, replies, |config| {
        configure(config);
        config.ocr_embedded_images = Some(false);
    })
}

/// One row for each output format: its name, the count of the picture's words in `content`,
/// their count in the content of page 1, and the count of [`IMAGE_REFERENCE_LINE`] in `content`.
type FormatCounts = Vec<(&'static str, usize, usize, usize)>;
/// What each number of a [`FormatCounts`] row counts, and the count that is right.
const FORMAT_COUNTS_LEGEND: &str =
    "each row: format, picture words in `content`, picture words in page 1, reference lines in `content`; 1 is right";

/// The picture's words one time in `content` and in the page content, and the image reference
/// line one time in `content`, in each output format.
fn once_in_every_output_format() -> FormatCounts {
    vec![
        ("plain", 1, 1, 1),
        ("markdown", 1, 1, 1),
        ("djot", 1, 1, 1),
        ("html", 1, 1, 1),
    ]
}

/// Extract `pdf` in each text output format and count the picture's words and the image
/// reference line. The picture is sent to OCR one time in each run.
fn counts_in_every_output_format(
    backend: &'static str,
    pdf: &[u8],
    configure: impl Fn(&mut ExtractionConfig),
) -> FormatCounts {
    [
        (OutputFormat::Plain, "plain"),
        (OutputFormat::Markdown, "markdown"),
        (OutputFormat::Djot, "djot"),
        (OutputFormat::Html, "html"),
    ]
    .into_iter()
    .map(|(format, format_name)| {
        let run = extract(backend, pdf, WORDS_FROM_THE_PICTURE, |config| {
            configure(config);
            config.output_format = format;
        });
        assert_eq!(
            (run.picture_calls, run.page_render_calls),
            (1, 0),
            "{format_name}: one OCR call, on the picture"
        );
        let content = &run.document.content;
        (
            format_name,
            words_in(content),
            words_in(run.page_content(1)),
            content.matches(IMAGE_REFERENCE_LINE).count(),
        )
    })
    .collect()
}

#[test]
fn picture_words_reach_content_and_page_content_without_an_images_block() {
    let run = extract(
        "picture-text-no-images-block",
        &text_page_with_picture(),
        WORDS_FROM_THE_PICTURE,
        |_| {},
    );

    assert_eq!(
        (run.picture_calls, run.page_render_calls),
        (1, 0),
        "one OCR call, on the picture"
    );
    assert!(run.document.content.contains(NATIVE_HEADING));
    assert_eq!(
        words_in(&run.document.content),
        1,
        "content: {:?}",
        run.document.content
    );
    assert_eq!(words_in(run.page_content(1)), 1, "page 1: {:?}", run.page_content(1));
    assert_eq!(
        run.document.images.as_ref().map(Vec::len),
        None,
        "no `images` block, so no images in the result"
    );
}

#[test]
fn picture_words_reach_page_content_with_an_images_block() {
    let no_bytes = extract(
        "picture-text-images-without-bytes",
        &text_page_with_picture(),
        WORDS_FROM_THE_PICTURE,
        |config| {
            config.images = Some(ImageExtractionConfig {
                extract_images: false,
                ..Default::default()
            });
        },
    );
    assert_eq!(no_bytes.picture_calls, 1);
    assert_eq!(words_in(&no_bytes.document.content), 1);
    assert_eq!(
        words_in(no_bytes.page_content(1)),
        1,
        "page 1: {:?}",
        no_bytes.page_content(1)
    );
    assert_eq!(no_bytes.document.images.as_ref().map(Vec::len), None);

    let with_bytes = extract(
        "picture-text-images-with-bytes",
        &text_page_with_picture(),
        WORDS_FROM_THE_PICTURE,
        |config| config.images = Some(ImageExtractionConfig::default()),
    );
    assert_eq!(with_bytes.picture_calls, 1);
    assert_eq!(words_in(&with_bytes.document.content), 1);
    assert_eq!(
        words_in(with_bytes.page_content(1)),
        1,
        "page 1: {:?}",
        with_bytes.page_content(1)
    );
    let images = with_bytes.document.images.as_ref().expect("the images are returned");
    assert_eq!(images.len(), 1);
    assert_eq!(
        images[0].ocr_result.as_ref().map(|result| result.content.as_str()),
        Some(PICTURE_WORDS)
    );
}

#[test]
fn picture_words_reach_content_when_placeholders_are_off() {
    let run = extract(
        "picture-text-placeholders-off",
        &text_page_with_picture(),
        WORDS_FROM_THE_PICTURE,
        |config| {
            config.images = Some(ImageExtractionConfig {
                inject_placeholders: false,
                ..Default::default()
            });
        },
    );

    assert_eq!(run.picture_calls, 1);
    assert_eq!(
        words_in(&run.document.content),
        1,
        "content: {:?}",
        run.document.content
    );
    assert_eq!(words_in(run.page_content(1)), 1, "page 1: {:?}", run.page_content(1));
    assert_eq!(run.document.images.as_ref().map(Vec::len), Some(1));
}

#[test]
fn markdown_without_an_images_block_has_the_words_and_no_placeholder() {
    let run = extract(
        "picture-text-markdown-no-images-block",
        &text_page_with_picture(),
        WORDS_FROM_THE_PICTURE,
        |config| config.output_format = OutputFormat::Markdown,
    );

    assert_eq!(run.picture_calls, 1);
    assert!(run.document.content.contains(NATIVE_HEADING));
    assert_eq!(
        words_in(&run.document.content),
        1,
        "content: {:?}",
        run.document.content
    );
    assert_eq!(words_in(run.page_content(1)), 1, "page 1: {:?}", run.page_content(1));
    assert!(
        !run.document.content.contains("!["),
        "content: {:?}",
        run.document.content
    );
    assert!(!run.page_content(1).contains("!["), "page 1: {:?}", run.page_content(1));
}

/// A structured document has the words where the picture is, before the text below it.
#[test]
fn markdown_puts_the_words_at_the_position_of_the_picture() {
    const BELOW_PICTURE_LINES: &[&str] = &[
        "The yard closes at six in the evening.",
        "Late deliveries wait until the next morning.",
    ];
    let pdf = pdf(&[PageSpec {
        lines: FIRST_PAGE_LINES,
        picture: Some(SMALL_PICTURE),
        lines_below: BELOW_PICTURE_LINES,
    }]);

    let run = extract(
        "picture-text-markdown-position",
        &pdf,
        WORDS_FROM_THE_PICTURE,
        |config| config.output_format = OutputFormat::Markdown,
    );

    let content = &run.document.content;
    let words = content.find(PICTURE_WORDS);
    let text_below = content.find("The yard closes");
    assert!(words.is_some() && text_below.is_some(), "content: {content:?}");
    assert!(words < text_below, "content: {content:?}");
}

/// The words follow the placeholder: `append_ocr_text` is on by default.
#[test]
fn markdown_with_an_images_block_has_the_words_after_the_placeholder() {
    let run = extract(
        "picture-text-markdown-images-block",
        &text_page_with_picture(),
        WORDS_FROM_THE_PICTURE,
        |config| {
            config.output_format = OutputFormat::Markdown;
            config.images = Some(ImageExtractionConfig::default());
        },
    );
    assert_eq!(run.picture_calls, 1);
    for (field, text) in [
        ("content", run.document.content.as_str()),
        ("page 1", run.page_content(1)),
    ] {
        let placeholder = text.find("![").unwrap_or_else(|| panic!("{field}: {text:?}"));
        let words = text.find(PICTURE_WORDS).unwrap_or_else(|| panic!("{field}: {text:?}"));
        assert!(placeholder < words, "{field}: {text:?}");
        assert_eq!(words_in(text), 1, "{field}: {text:?}");
    }
    let images = run.document.images.as_ref().expect("the images are returned");
    assert_eq!(
        images[0].ocr_result.as_ref().map(|result| result.content.as_str()),
        Some(PICTURE_WORDS)
    );
}

#[test]
fn markdown_has_the_placeholder_alone_when_append_ocr_text_is_off() {
    let run = extract(
        "picture-text-markdown-append-off",
        &text_page_with_picture(),
        WORDS_FROM_THE_PICTURE,
        |config| {
            config.output_format = OutputFormat::Markdown;
            config.images = Some(ImageExtractionConfig {
                append_ocr_text: false,
                ..Default::default()
            });
        },
    );
    assert_eq!(run.picture_calls, 1);
    for (field, text) in [
        ("content", run.document.content.as_str()),
        ("page 1", run.page_content(1)),
    ] {
        assert!(text.contains("!["), "{field}: {text:?}");
        assert_eq!(words_in(text), 0, "{field}: {text:?}");
    }
    let images = run.document.images.as_ref().expect("the images are returned");
    assert_eq!(
        images[0].ocr_result.as_ref().map(|result| result.content.as_str()),
        Some(PICTURE_WORDS)
    );
}

#[test]
fn markdown_has_the_words_and_no_placeholder_with_ocr_text_only() {
    let run = extract(
        "picture-text-markdown-text-only",
        &text_page_with_picture(),
        WORDS_FROM_THE_PICTURE,
        |config| {
            config.output_format = OutputFormat::Markdown;
            config.images = Some(ImageExtractionConfig {
                ocr_text_only: true,
                ..Default::default()
            });
        },
    );
    assert_eq!(run.picture_calls, 1);
    for (field, text) in [
        ("content", run.document.content.as_str()),
        ("page 1", run.page_content(1)),
    ] {
        assert!(!text.contains("!["), "{field}: {text:?}");
        assert_eq!(words_in(text), 1, "{field}: {text:?}");
    }
}

#[test]
fn no_picture_ocr_when_embedded_image_ocr_is_off() {
    let off = extract_without_picture_ocr(
        "picture-text-embedded-ocr-off",
        &text_page_with_picture(),
        WORDS_FROM_THE_PICTURE,
        |_| {},
    );
    let no_ocr_block = extract(
        "picture-text-no-ocr-block",
        &text_page_with_picture(),
        WORDS_FROM_THE_PICTURE,
        |config| config.ocr = None,
    );

    assert_eq!((off.picture_calls, off.page_render_calls), (0, 0));
    assert!(off.document.content.contains(NATIVE_HEADING));
    assert_eq!(words_in(&off.document.content), 0);
    assert_eq!(words_in(off.page_content(1)), 0);
    assert_eq!(off.document.content, no_ocr_block.document.content);
    assert_eq!(off.page_contents(), no_ocr_block.page_contents());
    assert_eq!(off.document.images.as_ref().map(Vec::len), None);
}

#[test]
fn picture_words_land_on_the_page_that_holds_the_picture() {
    for (format, format_name) in [(OutputFormat::Plain, "plain"), (OutputFormat::Markdown, "markdown")] {
        for (picture_page, other_page) in [(1u32, 2u32), (2, 1)] {
            let pdf = two_text_pages_with_picture_on(picture_page as usize);
            let run = extract("picture-text-two-pages", &pdf, WORDS_FROM_THE_PICTURE, |config| {
                config.output_format = format.clone();
            });
            let reference = extract_without_picture_ocr(
                "picture-text-two-pages-reference",
                &pdf,
                WORDS_FROM_THE_PICTURE,
                |config| config.output_format = format.clone(),
            );
            let case = format!("{format_name}, picture on page {picture_page}");

            assert_eq!(run.picture_calls, 1, "{case}");
            assert_eq!(words_in(&run.document.content), 1, "{case}: {:?}", run.document.content);
            assert_eq!(
                words_in(run.page_content(picture_page)),
                1,
                "{case}: {:?}",
                run.page_content(picture_page)
            );
            assert!(run.page_content(other_page).contains(NATIVE_HEADING), "{case}");
            assert_eq!(
                run.page_content(other_page),
                reference.page_content(other_page),
                "{case}: the page without the picture"
            );
        }
    }
}

/// The hierarchy of a page lists the blocks of its native text. A picture that is sent to OCR
/// adds no block to it.
#[test]
fn picture_ocr_adds_no_block_to_the_page_hierarchy() {
    for (format, format_name) in [(OutputFormat::Plain, "plain"), (OutputFormat::Markdown, "markdown")] {
        let run = extract(
            "picture-text-hierarchy",
            &text_page_with_picture(),
            WORDS_FROM_THE_PICTURE,
            |config| config.output_format = format.clone(),
        );
        let reference = extract_without_picture_ocr(
            "picture-text-hierarchy-reference",
            &text_page_with_picture(),
            WORDS_FROM_THE_PICTURE,
            |config| config.output_format = format.clone(),
        );

        assert_eq!(run.picture_calls, 1, "{format_name}");
        let blocks = run.hierarchy_block_texts();
        assert!(
            blocks[0].iter().any(|text| text.contains("Warehouse")),
            "{format_name}: {blocks:?}"
        );
        assert_eq!(blocks, reference.hierarchy_block_texts(), "{format_name}");
    }
}

#[test]
fn a_document_without_a_picture_is_the_same_with_and_without_picture_ocr() {
    let pdf = pdf(&[
        PageSpec {
            lines: FIRST_PAGE_LINES,
            picture: None,
            lines_below: &[],
        },
        PageSpec {
            lines: OTHER_PAGE_LINES,
            picture: None,
            lines_below: &[],
        },
    ]);
    for (format, format_name) in [(OutputFormat::Plain, "plain"), (OutputFormat::Markdown, "markdown")] {
        let run = extract("picture-text-no-picture", &pdf, WORDS_FROM_THE_PICTURE, |config| {
            config.output_format = format.clone();
        });
        let reference = extract_without_picture_ocr(
            "picture-text-no-picture-reference",
            &pdf,
            WORDS_FROM_THE_PICTURE,
            |config| config.output_format = format.clone(),
        );

        assert_eq!(run.picture_calls, 0, "{format_name}");
        assert!(run.document.content.contains(NATIVE_HEADING), "{format_name}");
        assert_eq!(run.document.content, reference.document.content, "{format_name}");
        assert_eq!(run.page_contents(), reference.page_contents(), "{format_name}");
    }
}

#[test]
fn a_picture_with_no_recognized_text_leaves_content_unchanged() {
    const NOTHING: Replies = Replies {
        picture: "",
        page_render: "",
    };
    for (format, format_name) in [(OutputFormat::Plain, "plain"), (OutputFormat::Markdown, "markdown")] {
        let run = extract(
            "picture-text-empty-reply",
            &text_page_with_picture(),
            NOTHING,
            |config| {
                config.output_format = format.clone();
            },
        );
        let reference = extract_without_picture_ocr(
            "picture-text-empty-reply-reference",
            &text_page_with_picture(),
            NOTHING,
            |config| config.output_format = format.clone(),
        );

        assert_eq!(run.picture_calls, 1, "{format_name}: the picture is sent to OCR");
        assert!(run.document.content.contains(NATIVE_HEADING), "{format_name}");
        assert_eq!(run.document.content, reference.document.content, "{format_name}");
        assert_eq!(run.page_contents(), reference.page_contents(), "{format_name}");
    }
}

/// A page with a picture and no text layer, between two pages of text. The structured document
/// starts a new page for it. When the picture gives no text, the page has no content, as it has
/// with OCR of embedded pictures off.
#[test]
fn a_page_with_only_a_textless_picture_adds_no_page_break() {
    const NOTHING: Replies = Replies {
        picture: "",
        page_render: "",
    };
    let pdf = pdf(&[
        PageSpec {
            lines: FIRST_PAGE_LINES,
            picture: None,
            lines_below: &[],
        },
        PageSpec {
            lines: &[],
            picture: Some(SMALL_PICTURE),
            lines_below: &[],
        },
        PageSpec {
            lines: OTHER_PAGE_LINES,
            picture: None,
            lines_below: &[],
        },
    ]);
    let structured_plain = |config: &mut ExtractionConfig| config.include_document_structure = true;

    let run = extract("picture-text-picture-only-page", &pdf, NOTHING, structured_plain);
    let reference = extract_without_picture_ocr(
        "picture-text-picture-only-page-reference",
        &pdf,
        NOTHING,
        structured_plain,
    );

    assert!(run.picture_calls >= 1, "the picture is sent to OCR");
    assert!(run.document.content.contains("signs the list at the gate"));
    assert_eq!(run.document.content, reference.document.content);
    assert_eq!(run.page_contents(), reference.page_contents());
}

/// A page with no text layer gets its text from OCR of the page render, and the render shows
/// the picture. The picture's own OCR text must not be added again.
#[test]
fn a_picture_on_an_ocr_page_is_not_added_twice() {
    const WORDS_FROM_EVERY_CALL: Replies = Replies {
        picture: PICTURE_WORDS,
        page_render: PICTURE_WORDS,
    };
    let pdf = pdf(&[PageSpec {
        lines: &[],
        picture: Some(LARGE_PICTURE),
        lines_below: &[],
    }]);
    let cases = [
        (OutputFormat::Plain, None, "plain, no images block"),
        (OutputFormat::Markdown, None, "markdown, no images block"),
        (
            OutputFormat::Plain,
            Some(ImageExtractionConfig::default()),
            "plain, images block",
        ),
        (
            OutputFormat::Markdown,
            Some(ImageExtractionConfig::default()),
            "markdown, images block",
        ),
        (
            OutputFormat::Markdown,
            Some(ImageExtractionConfig {
                ocr_text_only: true,
                ..Default::default()
            }),
            "markdown, images block, ocr_text_only",
        ),
    ];
    for (format, images, case) in cases {
        let run = extract("picture-text-ocr-page", &pdf, WORDS_FROM_EVERY_CALL, |config| {
            config.output_format = format;
            config.images = images;
        });

        assert!(run.page_render_calls >= 1, "{case}: the page is sent to OCR");
        assert!(run.picture_calls >= 1, "{case}: the picture is sent to OCR");
        assert_eq!(words_in(&run.document.content), 1, "{case}: {:?}", run.document.content);
        assert_eq!(words_in(run.page_content(1)), 1, "{case}: {:?}", run.page_content(1));
    }
}

/// One page with a text layer and one page without, which page OCR reads. The picture is on the
/// page that page OCR reads, so its own OCR text is not added there.
#[test]
fn a_picture_on_the_ocr_page_of_a_mixed_document_is_not_added_twice() {
    const WORDS_FROM_EVERY_CALL: Replies = Replies {
        picture: PICTURE_WORDS,
        page_render: PICTURE_WORDS,
    };
    let pdf = pdf(&[
        PageSpec {
            lines: FIRST_PAGE_LINES,
            picture: None,
            lines_below: &[],
        },
        PageSpec {
            lines: &[],
            picture: Some(LARGE_PICTURE),
            lines_below: &[],
        },
    ]);

    let run = extract("picture-text-mixed-ocr-page", &pdf, WORDS_FROM_EVERY_CALL, |_| {});

    assert!(run.page_render_calls >= 1, "the second page is sent to OCR");
    assert!(run.picture_calls >= 1, "the picture is sent to OCR");
    assert!(run.page_content(1).contains(NATIVE_HEADING));
    assert_eq!(words_in(run.page_content(1)), 0, "page 1: {:?}", run.page_content(1));
    assert_eq!(
        words_in(&run.document.content),
        1,
        "content: {:?}",
        run.document.content
    );
    assert_eq!(words_in(run.page_content(2)), 1, "page 2: {:?}", run.page_content(2));
}

/// `pdf_options.ocr_inline_images` reads the pictures in the extractor, and the pipeline reads
/// them again. The words are still in `content` and in the page content one time each.
#[test]
fn inline_image_ocr_adds_the_words_once() {
    let run = extract(
        "picture-text-inline-images",
        &text_page_with_picture(),
        WORDS_FROM_THE_PICTURE,
        |config| {
            config.pdf_options = Some(PdfConfig {
                ocr_inline_images: true,
                ..Default::default()
            });
        },
    );

    assert!(run.document.content.contains(NATIVE_HEADING));
    assert_eq!(
        words_in(&run.document.content),
        1,
        "content: {:?}",
        run.document.content
    );
    assert_eq!(words_in(run.page_content(1)), 1, "page 1: {:?}", run.page_content(1));
}

/// A line of text on the page has the form of an image reference. It is text, not a placeholder
/// of the picture, so the picture's words are not added after it a second time.
#[test]
fn text_in_the_form_of_an_image_reference_does_not_repeat_the_picture_words() {
    let counts = counts_in_every_output_format(
        "picture-text-reference-line",
        &text_page_with_picture_and_image_reference_line(),
        |_| {},
    );

    assert_eq!(counts, once_in_every_output_format(), "{FORMAT_COUNTS_LEGEND}");
}

/// With `ocr_text_only` on and placeholders off, the picture's words stand where the picture
/// is. The line of text stays as it is: the words do not replace it.
#[test]
fn ocr_text_only_does_not_replace_text_in_the_form_of_an_image_reference() {
    let counts = counts_in_every_output_format(
        "picture-text-reference-line-text-only",
        &text_page_with_picture_and_image_reference_line(),
        |config| {
            config.images = Some(ImageExtractionConfig {
                inject_placeholders: false,
                ocr_text_only: true,
                ..Default::default()
            });
        },
    );

    assert_eq!(counts, once_in_every_output_format(), "{FORMAT_COUNTS_LEGEND}");
}
