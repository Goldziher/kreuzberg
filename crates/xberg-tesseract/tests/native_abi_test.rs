//! Behavioural checks for the Tesseract C entry points whose return types were declared
//! incorrectly: `process_pages`, `get_text_direction`, orientation and paragraph metadata,
//! word font attributes, and choice-iterator text ownership. They need `eng.traineddata`
//! resolved from `TESSDATA_PREFIX`.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test binaries print by design; org logging policy exempts tests
use std::ffi::c_void;
use std::path::PathBuf;
use xberg_tesseract::{
    ChoiceIterator, TessOrientation, TessParagraphJustification, TessTextlineOrder, TessWritingDirection, TesseractAPI,
    TesseractError,
};

unsafe extern "C-unwind" {
    fn TessResultIteratorGetChoiceIterator(handle: *const c_void) -> *mut c_void;
}

fn tessdata_dir() -> PathBuf {
    let prefix =
        std::env::var("TESSDATA_PREFIX").expect("TESSDATA_PREFIX must point at a directory with eng.traineddata");
    let prefix = PathBuf::from(prefix);
    let dir = if prefix.ends_with("tessdata") {
        prefix
    } else {
        prefix.join("tessdata")
    };
    assert!(
        dir.join("eng.traineddata").exists(),
        "eng.traineddata missing in {}",
        dir.display()
    );
    dir
}

fn image_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_documents/images")
        .join(name)
        .canonicalize()
        .expect("test image must exist")
}

fn new_engine() -> TesseractAPI {
    let api = TesseractAPI::new().expect("create engine");
    api.init(tessdata_dir().to_str().expect("utf-8 tessdata path"), "eng")
        .expect("init engine");
    api
}

fn set_image(api: &TesseractAPI, name: &str) {
    let img = image::open(image_path(name)).expect("open image").to_rgb8();
    let (width, height) = img.dimensions();
    api.set_image(img.as_raw(), width as i32, height as i32, 3, 3 * width as i32)
        .expect("set image");
}

/// Writes `name` as a binary PPM; the vendored Leptonica reads PNM without optional codecs.
fn write_ppm(name: &str, dir: &std::path::Path) -> PathBuf {
    let img = image::open(image_path(name)).expect("open image").to_rgb8();
    let (width, height) = img.dimensions();
    let mut bytes = format!("P6\n{width} {height}\n255\n").into_bytes();
    bytes.extend_from_slice(img.as_raw());
    let path = dir.join(format!("{name}.ppm"));
    std::fs::write(&path, bytes).expect("write ppm");
    path
}

#[test]
fn process_pages_returns_the_text_of_every_page() {
    let dir = tempfile::tempdir().expect("scratch dir");
    let list = dir.path().join("pages.txt");
    let first = write_ppm("test_hello_world.png", dir.path());
    let last = write_ppm("invoice_image.png", dir.path());
    std::fs::write(&list, format!("{}\n{}\n", first.display(), last.display())).expect("write page list");

    let api = new_engine();

    let text = api
        .process_pages(list.to_str().expect("utf-8 path"), None, 0)
        .expect("process pages")
        .to_lowercase();

    let pages: Vec<&str> = text.split('\u{c}').filter(|page| !page.trim().is_empty()).collect();
    assert_eq!(pages.len(), 2, "expected one text block per page in: {text:?}");
    assert!(pages[0].contains("hello"), "first page missing from: {text:?}");
    assert!(pages[1].contains("invoice"), "last page missing from: {text:?}");
}

#[test]
fn process_pages_reports_a_failed_run_as_an_error() {
    let api = new_engine();
    let missing = tempfile::tempdir().expect("scratch dir").path().join("absent.png");
    let result = api.process_pages(missing.to_str().expect("utf-8 path"), None, 0);
    assert!(matches!(result, Err(TesseractError::ProcessPagesError)), "{result:?}");
}

#[test]
fn text_direction_reports_a_horizontal_baseline() {
    let api = new_engine();
    set_image(&api, "test_hello_world.png");
    let (offset, slope) = api.get_text_direction().expect("text direction");
    assert!(offset > 0, "baseline offset {offset}");
    assert!(slope.is_finite() && slope.abs() < 0.05, "baseline slope {slope}");
}

#[test]
fn text_direction_fails_when_layout_analysis_has_nothing_to_analyse() {
    let api = new_engine();
    let blank = vec![255u8; 64 * 64 * 3];
    api.set_image(&blank, 64, 64, 3, 3 * 64).expect("set blank image");
    assert!(matches!(
        api.get_text_direction(),
        Err(TesseractError::AnalyseLayoutError)
    ));
}

#[test]
fn paragraphs_without_a_model_are_skipped_with_safe_defaults() {
    let api = new_engine();
    set_image(&api, "simple_table.png");
    let layout = api.analyse_layout().expect("layout iterator");

    // The native wrapper copies uninitialized booleans on this path, so a single pass could
    // pass by luck; repeating makes a leaked stack value overwhelmingly likely to show. ~keep
    for _ in 0..50 {
        let outcome = layout.extract_all_paragraphs().expect("paragraphs");
        assert!(outcome.paragraphs.is_empty(), "{:?}", outcome.paragraphs);
        assert_eq!(outcome.skipped_no_para_info, 1);
        assert_eq!(outcome.skipped_no_bbox, 0);
        assert!(matches!(
            layout.paragraph_info(),
            Err(TesseractError::InvalidParameterError)
        ));
    }
}

#[test]
fn orientation_is_reported_without_a_native_status() {
    let api = new_engine();
    set_image(&api, "simple_table.png");
    let layout = api.analyse_layout().expect("layout iterator");
    let (orientation, writing_direction, textline_order, _deskew) = layout.orientation().expect("orientation");
    assert_eq!(orientation, TessOrientation::ORIENTATION_PAGE_UP);
    assert_eq!(writing_direction, TessWritingDirection::WRITING_DIRECTION_LEFT_TO_RIGHT);
    assert_eq!(textline_order, TessTextlineOrder::TEXTLINE_ORDER_TOP_TO_BOTTOM);
}

#[test]
fn paragraph_metadata_is_read_from_the_native_outputs() {
    let api = new_engine();
    set_image(&api, "invoice_image.png");
    api.recognize().expect("recognize");
    let (page, _words) = api.get_iterators().expect("iterators");

    let outcome = page.extract_all_paragraphs().expect("paragraphs");
    assert_eq!(outcome.skipped_no_para_info, 0);
    assert_eq!(outcome.skipped_no_bbox, 0);
    assert_eq!(outcome.paragraphs.len(), 3);
    for paragraph in &outcome.paragraphs {
        assert_eq!(paragraph.justification, TessParagraphJustification::JUSTIFICATION_LEFT);
        assert!(paragraph.right > paragraph.left && paragraph.bottom > paragraph.top);
    }

    page.begin().expect("rewind");
    let (justification, _is_list_item, _is_crown, indent) = page.paragraph_info().expect("paragraph info");
    assert_eq!(justification, TessParagraphJustification::JUSTIFICATION_LEFT);
    assert_eq!(indent, outcome.paragraphs[0].first_line_indent);
}

#[test]
fn lstm_words_have_no_font_attributes() {
    let api = new_engine();
    set_image(&api, "test_hello_world.png");
    api.recognize().expect("recognize");
    let words = api.get_iterator().expect("result iterator");

    assert!(matches!(
        words.word_font_attributes(),
        Err(TesseractError::InvalidParameterError)
    ));
    let outcome = words.extract_all_words().expect("words");
    assert!(!outcome.words.is_empty());
    assert!(outcome.words.iter().all(|word| word.font_attrs.is_none()));
}

#[test]
fn choice_text_can_be_read_repeatedly_without_being_freed() {
    let api = new_engine();
    api.set_variable("lstm_choice_mode", "1").expect("choice mode");
    set_image(&api, "test_hello_world.png");
    api.recognize().expect("recognize");
    let words = api.get_iterator().expect("result iterator");
    let handle = *words.handle.lock().expect("lock iterator");
    let raw = unsafe { TessResultIteratorGetChoiceIterator(handle) };
    assert!(!raw.is_null());
    let choices = ChoiceIterator::new(raw);

    let first = choices.get_utf8_text().expect("choice text");
    assert_eq!(first, "H");
    for _ in 0..100 {
        assert_eq!(choices.get_utf8_text().expect("choice text"), first);
    }
    assert!(choices.confidence().expect("confidence") > 0.0);
}
