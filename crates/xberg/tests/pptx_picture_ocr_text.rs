//! The OCR text of a picture on a PPTX slide.
//!
//! Each deck is built here: slides with a title, optional text boxes and one picture. A stub
//! OCR backend returns fixed words for the picture and counts its calls, so each test can
//! see where the words land in `content` and in the content of each slide.

#![cfg(all(feature = "office", feature = "ocr"))]

mod helpers;

use async_trait::async_trait;
use helpers::{extract_bytes_document_blocking, extract_uri_document_blocking, get_test_file_path, skip_if_missing};
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use xberg::ExtractedDocument;
use xberg::core::config::{ExtractionConfig, ImageExtractionConfig, OcrConfig, OutputFormat, PageConfig};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use zip::write::{SimpleFileOptions, ZipWriter};

const POWERPOINT_MIME_TYPE: &str = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
/// The words the stub reads from the picture.
const PICTURE_WORDS: &str = "Crate 17 holds forty blue lanterns";
const FIRST_TITLE: &str = "Stock list";
const SECOND_TITLE: &str = "Bay plan";
/// Text that an author typed on a slide. It has the form of a Markdown image reference.
const AUTHORED_IMAGE_REFERENCE: &str = "![label](label.png)";

struct SlideSpec {
    title: &'static str,
    /// One text box for each entry, below the title.
    text_boxes: &'static [&'static str],
    /// A picture below the text boxes.
    picture: bool,
}

fn title_and_picture() -> Vec<SlideSpec> {
    vec![SlideSpec {
        title: FIRST_TITLE,
        text_boxes: &[],
        picture: true,
    }]
}

fn picture_png() -> Vec<u8> {
    let picture = image::RgbImage::from_pixel(120, 40, image::Rgb([236, 214, 160]));
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(picture)
        .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
        .expect("the picture encodes as PNG");
    bytes
}

fn slide_xml(slide: &SlideSpec) -> String {
    let mut shapes = format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>
<p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="6000000" cy="800000"/></a:xfrm></p:spPr>
<p:txBody><a:bodyPr/><a:p><a:r><a:t>{}</a:t></a:r></a:p></p:txBody></p:sp>"#,
        slide.title
    );
    for (index, text) in slide.text_boxes.iter().enumerate() {
        shapes.push_str(&format!(
            r#"<p:sp><p:nvSpPr><p:cNvPr id="{}" name="Text"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
<p:spPr><a:xfrm><a:off x="0" y="{}"/><a:ext cx="6000000" cy="800000"/></a:xfrm></p:spPr>
<p:txBody><a:bodyPr/><a:p><a:r><a:t>{text}</a:t></a:r></a:p></p:txBody></p:sp>"#,
            10 + index,
            1_000_000 * (index + 1),
        ));
    }
    if slide.picture {
        shapes.push_str(
            r#"<p:pic><p:nvPicPr><p:cNvPr id="3" name="Picture"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
<p:blipFill><a:blip r:embed="rId2"/></p:blipFill>
<p:spPr><a:xfrm><a:off x="0" y="5000000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm></p:spPr></p:pic>"#,
        );
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<p:cSld><p:spTree>{shapes}</p:spTree></p:cSld>
</p:sld>"#
    )
}

/// A deck with one slide for each spec. Every picture is the same PNG.
fn deck(slides: &[SlideSpec]) -> Vec<u8> {
    deck_with_media(slides, true)
}

/// The deck of [`deck`]. Without `media`, the archive has no file for the picture.
fn deck_with_media(slides: &[SlideSpec], media: bool) -> Vec<u8> {
    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default();
        let mut part = |name: &str, bytes: &[u8]| {
            zip.start_file(name, options).expect("the part starts");
            zip.write_all(bytes).expect("the part is written");
        };

        part(
            "[Content_Types].xml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="xml" ContentType="application/xml"/>
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="png" ContentType="image/png"/>
</Types>"#,
        );
        part(
            "_rels/.rels",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
        );
        part("ppt/presentation.xml", b"<?xml version=\"1.0\"?><presentation/>");
        let slide_relationships: String = (1..=slides.len())
            .map(|number| {
                format!(
                    r#"<Relationship Id="rId{number}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{number}.xml"/>"#
                )
            })
            .collect();
        part(
            "ppt/_rels/presentation.xml.rels",
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{slide_relationships}</Relationships>"#
            )
            .as_bytes(),
        );
        for (index, slide) in slides.iter().enumerate() {
            let number = index + 1;
            part(&format!("ppt/slides/slide{number}.xml"), slide_xml(slide).as_bytes());
            if slide.picture {
                part(
                    &format!("ppt/slides/_rels/slide{number}.xml.rels"),
                    br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/label.png"/>
</Relationships>"#,
                );
            }
        }
        if media {
            part("ppt/media/label.png", &picture_png());
        }
        part(
            "docProps/core.xml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Stock</dc:title></cp:coreProperties>"#,
        );
        part(
            "docProps/app.xml",
            format!(
                r#"<?xml version="1.0"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Slides>{}</Slides></Properties>"#,
                slides.len()
            )
            .as_bytes(),
        );
        zip.finish().expect("the deck is complete");
    }
    buffer
}

struct StubBackend {
    name: &'static str,
    reply: &'static str,
    calls: Arc<AtomicUsize>,
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
    async fn process_image(&self, _image_bytes: &[u8], _config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut document = ExtractedDocument::default();
        document.content = self.reply.to_string();
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
    ocr_calls: usize,
}

impl Run {
    fn slide_content(&self, slide_number: u32) -> &str {
        self.document
            .pages
            .as_ref()
            .expect("page extraction is on, so the result has pages")
            .iter()
            .find(|page| page.page_number == slide_number)
            .unwrap_or_else(|| panic!("the result has no slide {slide_number}"))
            .content
            .as_str()
    }
}

/// How many times the picture's words are in `text`.
fn words_in(text: &str) -> usize {
    text.matches(PICTURE_WORDS).count()
}

/// Extract the bytes of `deck` with a stub backend registered as `backend` that reads `reply`
/// from each picture. The config has an `ocr` block for the stub, page extraction on, plain
/// output and no `images` block; `configure` changes it.
fn extract(
    backend: &'static str,
    deck: &[u8],
    reply: &'static str,
    configure: impl FnOnce(&mut ExtractionConfig),
) -> Run {
    extract_through(backend, reply, configure, |config| {
        extract_bytes_document_blocking(deck, POWERPOINT_MIME_TYPE, config)
    })
}

/// The run of [`extract`], with the deck read by `read`.
fn extract_through(
    backend: &'static str,
    reply: &'static str,
    configure: impl FnOnce(&mut ExtractionConfig),
    read: impl FnOnce(&ExtractionConfig) -> xberg::Result<ExtractedDocument>,
) -> Run {
    let calls = Arc::new(AtomicUsize::new(0));
    let _ = unregister_ocr_backend(backend);
    register_ocr_backend(Arc::new(StubBackend {
        name: backend,
        reply,
        calls: Arc::clone(&calls),
    }))
    .expect("the stub backend registers");
    let _guard = BackendGuard(backend);

    let mut config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: backend.to_string(),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        output_format: OutputFormat::Plain,
        use_cache: false,
        ..Default::default()
    };
    configure(&mut config);
    let document = read(&config).expect("the deck extracts");
    Run {
        document,
        ocr_calls: calls.load(Ordering::SeqCst),
    }
}

#[test]
fn plain_output_has_the_picture_words_once_in_content_and_slide_content() {
    let run = extract("pptx-picture-plain", &deck(&title_and_picture()), PICTURE_WORDS, |_| {});

    assert_eq!(run.ocr_calls, 1);
    assert!(run.document.content.contains(FIRST_TITLE), "{:?}", run.document.content);
    assert_eq!(words_in(&run.document.content), 1, "{:?}", run.document.content);
    assert!(run.slide_content(1).contains(FIRST_TITLE), "{:?}", run.slide_content(1));
    assert_eq!(words_in(run.slide_content(1)), 1, "{:?}", run.slide_content(1));
}

#[test]
fn plain_output_of_a_deck_read_from_a_file_has_the_picture_words_once() {
    let directory = tempfile::tempdir().expect("the directory is made");
    let path = directory.path().join("stock.pptx");
    std::fs::write(&path, deck(&title_and_picture())).expect("the deck is written");

    let run = extract_through(
        "pptx-picture-plain-file",
        PICTURE_WORDS,
        |_| {},
        |config| extract_uri_document_blocking(&path, Some(POWERPOINT_MIME_TYPE), config),
    );

    assert_eq!(run.ocr_calls, 1);
    assert!(run.document.content.contains(FIRST_TITLE), "{:?}", run.document.content);
    assert_eq!(words_in(&run.document.content), 1, "{:?}", run.document.content);
    assert_eq!(words_in(run.slide_content(1)), 1, "{:?}", run.slide_content(1));
}

#[test]
fn plain_output_has_the_words_with_every_images_block_setting() {
    let settings: [(&str, Option<ImageExtractionConfig>); 5] = [
        ("no images block", None),
        ("default images block", Some(ImageExtractionConfig::default())),
        (
            "append_ocr_text off",
            Some(ImageExtractionConfig {
                append_ocr_text: false,
                ..Default::default()
            }),
        ),
        (
            "ocr_text_only on",
            Some(ImageExtractionConfig {
                ocr_text_only: true,
                ..Default::default()
            }),
        ),
        (
            "inject_placeholders off",
            Some(ImageExtractionConfig {
                inject_placeholders: false,
                ..Default::default()
            }),
        ),
    ];
    let deck = deck(&title_and_picture());

    for (label, images) in settings {
        let run = extract("pptx-picture-plain-settings", &deck, PICTURE_WORDS, |config| {
            config.images = images;
        });

        assert_eq!(run.ocr_calls, 1, "{label}");
        assert_eq!(
            words_in(&run.document.content),
            1,
            "{label}: {:?}",
            run.document.content
        );
        assert_eq!(words_in(run.slide_content(1)), 1, "{label}: {:?}", run.slide_content(1));
    }
}

#[test]
fn markup_output_with_placeholders_off_has_the_words_and_no_placeholder() {
    let formats = [
        (OutputFormat::Markdown, "!["),
        (OutputFormat::Djot, "!["),
        (OutputFormat::Html, "<img"),
    ];
    let deck = deck(&title_and_picture());

    for (format, placeholder_start) in formats {
        let label = format!("{format:?}");
        let run = extract("pptx-picture-no-placeholder", &deck, PICTURE_WORDS, |config| {
            config.output_format = format;
            config.images = Some(ImageExtractionConfig {
                inject_placeholders: false,
                ..Default::default()
            });
        });
        let content = &run.document.content;

        assert!(content.contains(FIRST_TITLE), "{label}: {content:?}");
        assert!(!content.contains(placeholder_start), "{label}: {content:?}");
        assert_eq!(words_in(content), 1, "{label}: {content:?}");
        assert_eq!(words_in(run.slide_content(1)), 1, "{label}: {:?}", run.slide_content(1));
    }
}

#[test]
fn picture_words_land_on_the_slide_that_holds_the_picture() {
    let slides = [
        SlideSpec {
            title: FIRST_TITLE,
            text_boxes: &[],
            picture: false,
        },
        SlideSpec {
            title: SECOND_TITLE,
            text_boxes: &[],
            picture: true,
        },
    ];

    let run = extract("pptx-picture-second-slide", &deck(&slides), PICTURE_WORDS, |_| {});

    assert_eq!(run.ocr_calls, 1);
    assert_eq!(words_in(&run.document.content), 1, "{:?}", run.document.content);
    assert!(run.slide_content(1).contains(FIRST_TITLE), "{:?}", run.slide_content(1));
    assert_eq!(words_in(run.slide_content(1)), 0, "{:?}", run.slide_content(1));
    assert!(
        run.slide_content(2).contains(SECOND_TITLE),
        "{:?}",
        run.slide_content(2)
    );
    assert_eq!(words_in(run.slide_content(2)), 1, "{:?}", run.slide_content(2));
}

#[test]
fn a_picture_with_no_recognized_text_leaves_plain_content_unchanged() {
    let deck = deck(&title_and_picture());
    let without_picture_ocr = extract("pptx-picture-no-text-reference", &deck, "", |config| {
        config.ocr_embedded_images = Some(false);
    });

    let run = extract("pptx-picture-no-text", &deck, "", |_| {});

    assert_eq!(without_picture_ocr.ocr_calls, 0);
    assert_eq!(run.ocr_calls, 1);
    assert!(run.document.content.contains(FIRST_TITLE), "{:?}", run.document.content);
    assert_eq!(run.document.content, without_picture_ocr.document.content);
    assert_eq!(run.slide_content(1), without_picture_ocr.slide_content(1));
}

#[test]
fn authored_image_reference_text_does_not_repeat_the_picture_words() {
    let slides = [SlideSpec {
        title: FIRST_TITLE,
        text_boxes: &[AUTHORED_IMAGE_REFERENCE],
        picture: true,
    }];

    let run = extract("pptx-picture-authored-reference", &deck(&slides), PICTURE_WORDS, |_| {});
    let content = &run.document.content;

    assert_eq!(run.ocr_calls, 1);
    assert!(content.contains(AUTHORED_IMAGE_REFERENCE), "{content:?}");
    assert_eq!(words_in(content), 1, "{content:?}");
    assert_eq!(words_in(run.slide_content(1)), 1, "{:?}", run.slide_content(1));
}

#[test]
fn a_picture_with_no_file_in_the_archive_adds_no_text_to_plain_content() {
    let run = extract(
        "pptx-picture-no-file",
        &deck_with_media(&title_and_picture(), false),
        PICTURE_WORDS,
        |_| {},
    );

    assert_eq!(run.ocr_calls, 0);
    assert_eq!(run.document.content.trim(), FIRST_TITLE);
    assert_eq!(run.slide_content(1).trim(), FIRST_TITLE);
}

#[test]
fn markdown_default_output_keeps_placeholder_then_words_once() {
    let run = extract(
        "pptx-picture-markdown-default",
        &deck(&title_and_picture()),
        PICTURE_WORDS,
        |config| config.output_format = OutputFormat::Markdown,
    );
    let content = &run.document.content;

    assert_eq!(run.ocr_calls, 1);
    assert_eq!(words_in(content), 1, "{content:?}");
    let placeholder = content.find("![").expect("the picture has a placeholder");
    let words = content.find(PICTURE_WORDS).expect("the words are in the content");
    assert!(placeholder < words, "{content:?}");
    assert_eq!(words_in(run.slide_content(1)), 1, "{:?}", run.slide_content(1));
}

#[test]
fn plain_output_of_a_corpus_deck_has_the_words_of_each_picture_once() {
    const FIXTURE: &str = "pptx/powerpoint_with_image.pptx";
    if skip_if_missing(FIXTURE) {
        return;
    }
    let deck = std::fs::read(get_test_file_path(FIXTURE)).expect("the corpus deck is read");

    let run = extract("pptx-picture-corpus", &deck, PICTURE_WORDS, |_| {});
    let slide_total: usize = run
        .document
        .pages
        .as_ref()
        .expect("page extraction is on, so the result has pages")
        .iter()
        .map(|page| words_in(&page.content))
        .sum();

    assert!(run.ocr_calls >= 1, "no picture of the deck reached OCR");
    assert_eq!(
        words_in(&run.document.content),
        run.ocr_calls,
        "{:?}",
        run.document.content
    );
    assert_eq!(slide_total, run.ocr_calls);
}
