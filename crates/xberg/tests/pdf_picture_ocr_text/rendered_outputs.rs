use super::*;
use crate::helpers::{FixedLineRenderer, PageWritingRenderer, pages_written};
use xberg::plugins::register_renderer;

/// The name of the registered custom renderer, whose output is one line without the words.
const FIXED_LINE_RENDERER: &str = "pdf-picture-text-fixed-line";
/// What that renderer renders for every document.
const FIXED_LINE: &str = "A rendering of the report.";

/// The `images` settings each test of built page content runs with: `append_ocr_text` on (the
/// default) and off.
const APPEND_ON_AND_OFF: [(&str, bool); 2] = [("append_ocr_text on", true), ("append_ocr_text off", false)];

/// Extract [`text_page_with_picture`] without page extraction, so the extractor stores no pages
/// and the content of page 1 is built from the elements of the page.
fn extract_with_page_content_built_from_elements(
    backend: &'static str,
    output_format: OutputFormat,
    append_ocr_text: bool,
) -> Run {
    let run = extract(backend, &text_page_with_picture(), WORDS_FROM_THE_PICTURE, |config| {
        config.output_format = output_format;
        config.pages = None;
        config.images = Some(ImageExtractionConfig {
            append_ocr_text,
            ..Default::default()
        });
    });
    assert_eq!(run.picture_calls, 1, "{backend}: one OCR call, on the picture");
    assert!(
        run.page_content(1).contains(NATIVE_HEADING),
        "{backend}, page 1: {:?}",
        run.page_content(1)
    );
    run
}

/// The picture's words in `content` and in the content of page 1, for the outputs whose
/// `content` is not the plain rendering and whose page content no renderer writes again: JSON
/// output, the built-in custom renderer `dot`, and a registered custom renderer. Each row:
/// format, setting, words in `content`, words in page 1.
fn counts_in_rendered_outputs(backends: [&'static str; 6]) -> Vec<(&'static str, &'static str, usize, usize)> {
    register_renderer(Arc::new(FixedLineRenderer {
        name: FIXED_LINE_RENDERER,
        line: FIXED_LINE,
    }))
    .expect("the renderer registers");
    let formats = [
        ("json", OutputFormat::Json),
        ("dot", OutputFormat::Custom("dot".to_string())),
        ("custom renderer", OutputFormat::Custom(FIXED_LINE_RENDERER.to_string())),
    ];
    let mut backends = backends.into_iter();
    let mut counts = Vec::new();
    for (format_name, format) in formats {
        for (setting, append_ocr_text) in APPEND_ON_AND_OFF {
            let backend = backends.next().expect("one stub for each cell");
            let run = extract_with_page_content_built_from_elements(backend, format.clone(), append_ocr_text);
            if format_name == "custom renderer" {
                assert_eq!(run.document.content, FIXED_LINE, "{setting}");
            }
            counts.push((
                format_name,
                setting,
                words_in(&run.document.content),
                words_in(run.page_content(1)),
            ));
        }
    }
    counts
}

#[test]
fn json_dot_and_custom_renderer_page_content_built_from_elements_has_no_picture_words() {
    let counts = counts_in_rendered_outputs([
        "picture-text-built-page-json-append-on",
        "picture-text-built-page-json-append-off",
        "picture-text-built-page-dot-append-on",
        "picture-text-built-page-dot-append-off",
        "picture-text-built-page-custom-append-on",
        "picture-text-built-page-custom-append-off",
    ]);

    assert_eq!(
        counts,
        vec![
            ("json", "append_ocr_text on", 0, 0),
            ("json", "append_ocr_text off", 0, 0),
            ("dot", "append_ocr_text on", 0, 0),
            ("dot", "append_ocr_text off", 0, 0),
            ("custom renderer", "append_ocr_text on", 0, 0),
            ("custom renderer", "append_ocr_text off", 0, 0),
        ],
        "each row: format, setting, picture words in `content`, picture words in page 1"
    );
}

#[test]
fn plain_page_content_built_from_elements_has_the_picture_words_as_plain_content_has() {
    let run = extract_with_page_content_built_from_elements(
        "picture-text-built-page-plain-append-off",
        OutputFormat::Plain,
        false,
    );

    assert_eq!(words_in(&run.document.content), 1, "{:?}", run.document.content);
    assert_eq!(words_in(run.page_content(1)), 1, "page 1: {:?}", run.page_content(1));
}

#[test]
fn a_custom_format_name_with_no_renderer_gives_plain_page_content_built_from_elements_with_the_words() {
    let backends = [
        "picture-text-built-page-no-renderer-append-on",
        "picture-text-built-page-no-renderer-append-off",
    ];

    for ((setting, append_ocr_text), backend) in APPEND_ON_AND_OFF.into_iter().zip(backends) {
        let run = extract_with_page_content_built_from_elements(
            backend,
            OutputFormat::Custom("no-such-renderer".to_string()),
            append_ocr_text,
        );

        assert_eq!(
            words_in(&run.document.content),
            1,
            "{setting}: {:?}",
            run.document.content
        );
        assert_eq!(
            words_in(run.page_content(1)),
            1,
            "{setting}, page 1: {:?}",
            run.page_content(1)
        );
    }
}

/// The name of the registered custom renderer whose output is the pages it receives.
const PAGE_WRITING_RENDERER: &str = "pdf-picture-text-page-writer";

#[test]
fn a_registered_renderer_receives_the_page_content_of_the_result_without_the_picture_words() {
    register_renderer(Arc::new(PageWritingRenderer {
        name: PAGE_WRITING_RENDERER,
    }))
    .expect("the renderer registers");

    let run = extract_with_page_content_built_from_elements(
        "picture-text-built-page-page-writer",
        OutputFormat::Custom(PAGE_WRITING_RENDERER.to_string()),
        true,
    );

    assert!(
        run.document.content.contains(NATIVE_HEADING),
        "{:?}",
        run.document.content
    );
    assert_eq!(words_in(&run.document.content), 0, "{:?}", run.document.content);
    assert_eq!(
        run.document.content,
        pages_written(run.document.pages.as_deref().unwrap_or_default())
    );
}
