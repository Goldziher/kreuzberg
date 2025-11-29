use image::{DynamicImage, ImageBuffer, Luma};
use imageproc::contrast::adaptive_threshold;
use imageproc::filter::filter3x3;
use kreuzberg_tesseract::{
    TessOrientation, TessPageIteratorLevel, TessPageSegMode, TessWritingDirection, TesseractAPI,
};
use std::path::{Path, PathBuf};

fn get_default_tessdata_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        let home_dir = std::env::var("HOME").expect("HOME environment variable not set");
        PathBuf::from(home_dir)
            .join("Library")
            .join("Application Support")
            .join("tesseract-rs")
            .join("tessdata")
    } else if cfg!(target_os = "linux") {
        let home_dir = std::env::var("HOME").expect("HOME environment variable not set");
        PathBuf::from(home_dir).join(".tesseract-rs").join("tessdata")
    } else if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("APPDATA").expect("APPDATA environment variable not set"))
            .join("tesseract-rs")
            .join("tessdata")
    } else {
        panic!("Unsupported operating system");
    }
}

fn get_tessdata_dir() -> PathBuf {
    match std::env::var("TESSDATA_PREFIX") {
        Ok(dir) => {
            let path = PathBuf::from(dir);
            println!("Using TESSDATA_PREFIX directory: {:?}", path);
            path
        }
        Err(_) => {
            let default_dir = get_default_tessdata_dir();
            println!("TESSDATA_PREFIX not set, using default directory: {:?}", default_dir);
            default_dir
        }
    }
}

fn preprocess_image(img: &DynamicImage) -> ImageBuffer<Luma<u8>, Vec<u8>> {
    let luma_img = img.to_luma8();

    let contrast_adjusted = adaptive_threshold(&luma_img, 2);

    filter3x3(&contrast_adjusted, &[-1, -1, -1, -1, 9, -1, -1, -1, -1])
}

fn load_test_image(filename: &str) -> Result<(Vec<u8>, u32, u32), Box<dyn std::error::Error>> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("test_images");
    path.push(filename);

    let img = image::open(&path)
        .map_err(|e| format!("Failed to open test image {}: {}", path.display(), e))?
        .to_rgb8();
    let (width, height) = img.dimensions();
    Ok((img.into_raw(), width, height))
}

#[test]
fn test_multiple_languages_with_lstm() {
    let tessdata_dir = get_tessdata_dir();

    let eng_traineddata = tessdata_dir.join("eng.traineddata");
    let tur_traineddata = tessdata_dir.join("tur.traineddata");
    assert!(eng_traineddata.exists(), "eng.traineddata not found");
    assert!(tur_traineddata.exists(), "tur.traineddata not found");

    let api = TesseractAPI::new();
    let res = api.set_variable("debug_file", "/dev/null");
    assert!(res.is_ok());

    api.init(tessdata_dir.to_str().unwrap(), "tur")
        .expect("Failed to initialize Tesseract with multiple languages");

    api.set_variable("tessedit_ocr_engine_mode", "4")
        .expect("Failed to set LSTM mode");

    api.set_variable("tessedit_pageseg_mode", "1")
        .expect("Failed to set PSM");
    //api.set_variable("tessedit_char_blacklist", "!?@#$%&*()_+-=[]{}|\\")
    //    .expect("Failed to set char blacklist");

    let img = image::open("tests/test_images/multilang_sample.png").expect("Failed to open image");
    let preprocessed = preprocess_image(&img);
    let (width, height) = preprocessed.dimensions();

    let res = api.set_image(preprocessed.as_raw(), width as i32, height as i32, 1, width as i32);
    assert!(res.is_ok());
    let text = api.get_utf8_text().expect("Failed to perform OCR");
    println!("Recognized text: {}", text);

    assert!(!text.is_empty());
    assert!(
        text.to_lowercase().contains("hello") && text.to_lowercase().contains("dünya"),
        "Text does not contain expected words. Found: {}",
        text
    );

    let confidences = api.get_word_confidences();
    println!("Word confidences: {:?}", confidences);
    assert!(confidences.is_ok(), "No word confidences returned");
    assert!(
        confidences.unwrap().iter().any(|&c| c > 80),
        "No high confidence words found"
    );
}

#[test]
fn test_ocr_on_real_image() {
    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    let (image_data, width, height) = load_test_image("sample_text.png").expect("Failed to load test image");
    let res = api.set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32);
    assert!(res.is_ok());
    let text = api.get_utf8_text().expect("Failed to perform OCR");
    assert!(!text.is_empty());
    assert!(text.contains("This is a sample text for OCR testing."));

    let confidences = api.get_word_confidences();
    assert!(confidences.is_ok());
    assert!(confidences.unwrap().iter().all(|&c| c > 0));
}

#[test]
fn test_multiple_languages() {
    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();
    api.init(tessdata_dir.to_str().unwrap(), "tur+eng")
        .expect("Failed to initialize Tesseract with multiple languages");
    api.set_variable("tessedit_pageseg_mode", "1")
        .expect("Failed to set PSM");
    //api.set_variable("tessedit_char_blacklist", "!?@#$%&*()_+-=[]{}").expect("Failed to set char blacklist");
    api.set_variable("tessedit_enable_dict_correction", "1")
        .expect("Failed to enable dictionary correction");

    api.set_variable("preserve_interword_spaces", "1")
        .expect("Failed to set preserve_interword_spaces");

    api.set_variable(
        "tessedit_char_whitelist",
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZçğıiöşüÇĞİIÖŞÜ.,! ",
    )
    .expect("Failed to set char whitelist");

    let (image_data, width, height) = load_test_image("multilang_sample.png").expect("Failed to load test image");
    let res = api.set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32);
    assert!(res.is_ok());
    let text = api.get_utf8_text().expect("Failed to perform OCR");
    assert!(!text.is_empty());
    assert!(text.contains("Hello") && text.contains("Dünya"));
}

#[test]
fn test_digit_recognition() {
    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");
    api.set_variable("tessedit_char_whitelist", "0123456789")
        .expect("Failed to set whitelist");

    let (image_data, width, height) = load_test_image("digits.png").expect("Failed to load test image");
    let res = api.set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32);
    assert!(res.is_ok());

    let text = api.get_utf8_text().expect("Failed to perform OCR");
    assert!(!text.is_empty());
    assert!(text.chars().all(|c| c.is_ascii_digit() || c.is_whitespace()));
}

#[test]
fn test_error_handling() {
    let api = TesseractAPI::new();

    let init_result = api.init("/invalid/path", "eng");
    assert!(init_result.is_err());

    if init_result.is_err() {}
}

#[test]
fn test_image_operation_errors() {
    let api = TesseractAPI::new();
    let tessdata_dir = get_tessdata_dir();

    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    let (image_data, width, height) = load_test_image("sample_text.png").expect("Failed to load test image");

    let res = api.set_image(
        &image_data,
        0, // Invalid width
        height as i32,
        3,
        3 * width as i32,
    );
    assert!(res.is_err());
}

#[test]
fn test_invalid_language_code() {
    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();

    // Test invalid language code
    let result = api.init(tessdata_dir.to_str().unwrap(), "invalid_lang");
    assert!(result.is_err());
}

#[test]
fn test_empty_image_data() {
    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    // Test with empty image data
    let empty_data: Vec<u8> = Vec::new();
    let res = api.set_image(&empty_data, 100, 100, 3, 300);
    assert!(res.is_err());
}

#[test]
fn test_invalid_image_parameters() {
    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    let (image_data, width, height) = load_test_image("sample_text.png").expect("Failed to load test image");

    // Test negative dimensions
    let res = api.set_image(&image_data, -1, height as i32, 3, 3 * width as i32);
    assert!(res.is_err());

    // Test zero height
    let res = api.set_image(&image_data, width as i32, 0, 3, 3 * width as i32);
    assert!(res.is_err());

    // Test invalid bytes_per_pixel
    let res = api.set_image(&image_data, width as i32, height as i32, 0, 3 * width as i32);
    assert!(res.is_err());

    // Test mismatched bytes_per_line
    let res = api.set_image(&image_data, width as i32, height as i32, 3, width as i32); // Should be 3 * width
    assert!(res.is_err());
}

#[test]
fn test_variable_setting() {
    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    // Test invalid variable name
    let res = api.set_variable("invalid_variable_name", "1");
    assert!(res.is_err());

    // Test empty variable value
    let res = api.set_variable("tessedit_char_whitelist", "");
    assert!(res.is_ok()); // Empty whitelist is actually valid

    // Test valid variable settings
    assert!(api.set_variable("tessedit_pageseg_mode", "1").is_ok());
    assert!(api.set_variable("tessedit_ocr_engine_mode", "1").is_ok());
}

#[test]
fn test_multiple_operations() {
    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    let (image_data, width, height) = load_test_image("sample_text.png").expect("Failed to load test image");

    // Set image multiple times
    for _ in 0..3 {
        let res = api.set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32);
        assert!(res.is_ok());
        let text = api.get_utf8_text().expect("Failed to perform OCR");
        assert!(!text.is_empty());
    }
}

#[test]
fn test_preprocessing_effects() {
    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    let img = image::open("tests/test_images/sample_text.png").expect("Failed to open image");

    // Test with preprocessed image
    let preprocessed = preprocess_image(&img);
    let (width, height) = preprocessed.dimensions();

    let res = api.set_image(preprocessed.as_raw(), width as i32, height as i32, 1, width as i32);
    assert!(res.is_ok());

    let text = api.get_utf8_text().expect("Failed to perform OCR");
    assert!(!text.is_empty());
}

#[test]
fn test_concurrent_access() {
    use std::thread;

    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    // Multiple threads trying to access the API simultaneously
    let mut handles = vec![];

    for i in 0..3 {
        let api_clone = api.clone();
        let handle = thread::spawn(move || {
            match i % 3 {
                0 => {
                    let res = api_clone.set_variable("tessedit_pageseg_mode", "1");
                    assert!(res.is_ok());
                }
                1 => {
                    let res = api_clone.set_variable("tessedit_char_whitelist", "0123456789");
                    assert!(res.is_ok());
                }
                2 => {
                    let text = api_clone.get_utf8_text();
                    // Text might be empty since we haven't set an image, but it shouldn't panic
                    assert!(text.is_err());
                }
                _ => unreachable!(),
            }
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_thread_safety_with_image() {
    use std::sync::Arc;
    use std::thread;

    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();

    // Ana API'yi configure et
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");
    api.set_variable("tessedit_pageseg_mode", "1")
        .expect("Failed to set PSM");

    let (image_data, width, height) = load_test_image("sample_text.png").expect("Failed to load test image");

    // Image'ı ana thread'de set et
    let res = api.set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32);
    assert!(res.is_ok());

    let image_data = Arc::new(image_data);
    let mut handles = vec![];

    // Thread'lerde clone'lanmış API'yi kullan
    for _ in 0..3 {
        let api_clone = api.clone(); // Bu artık tüm konfigürasyonu da kopyalayacak
        let image_data = Arc::clone(&image_data);

        let handle = thread::spawn(move || {
            let res = api_clone.set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32);
            assert!(res.is_ok());

            let text = api_clone.get_utf8_text().expect("Failed to get text");
            assert!(!text.is_empty());
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_thread_safety_init() {
    use std::thread;

    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();

    let mut handles = vec![];

    // Try to initialize from multiple threads
    for i in 0..3 {
        let api_clone = api.clone();
        let tessdata_dir = tessdata_dir.clone();

        let handle = thread::spawn(move || {
            let lang = match i % 3 {
                0 => "eng",
                1 => "tur",
                2 => "eng+tur",
                _ => unreachable!(),
            };

            let res = api_clone.init(tessdata_dir.to_str().unwrap(), lang);
            // Note: Only one initialization should succeed due to mutex
            if res.is_err() {
                println!("Init failed for lang {}, which is expected in some cases", lang);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_dynamic_image_setting() {
    let api = TesseractAPI::new();

    // Get tessdata directory (uses default location or TESSDATA_PREFIX if set)
    let tessdata_dir = get_tessdata_dir();
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize API");

    // Create a 24x24 pixel test image with the number 9 (black on white background)
    let width = 24;
    let height = 24;
    let bytes_per_pixel = 1;
    let bytes_per_line = width * bytes_per_pixel;

    // Initialize image data with all white pixels
    let mut image_data = vec![255u8; width * height];

    // Draw the number 9 (simplified version)
    for y in 4..19 {
        for x in 7..17 {
            // Top bar
            if y == 4 && (8..=15).contains(&x) {
                image_data[y * width + x] = 0;
            }
            // Top curve left side
            if (4..=10).contains(&y) && x == 7 {
                image_data[y * width + x] = 0;
            }
            // Top curve right side
            if (4..=11).contains(&y) && x == 16 {
                image_data[y * width + x] = 0;
            }
            // Middle bar
            if y == 11 && (8..=15).contains(&x) {
                image_data[y * width + x] = 0;
            }
            // Bottom right vertical line
            if (11..=18).contains(&y) && x == 16 {
                image_data[y * width + x] = 0;
            }
            // Bottom bar
            if y == 18 && (8..=15).contains(&x) {
                image_data[y * width + x] = 0;
            }
        }
    }

    // Set the image data
    api.set_image(
        &image_data,
        width.try_into().unwrap(),
        height.try_into().unwrap(),
        bytes_per_pixel.try_into().unwrap(),
        bytes_per_line.try_into().unwrap(),
    )
    .expect("Failed to set image");

    // Set whitelist for digits only
    api.set_variable("tessedit_char_whitelist", "0123456789")
        .expect("Failed to set whitelist");

    // Get the recognized text
    let text = api.get_utf8_text().expect("Failed to get text");
    println!("Recognized text: {}", text.trim());

    // Check if the result contains the digit 9
    assert!(!text.trim().is_empty(), "OCR result is empty");
    assert!(text.trim().contains("9"), "Expected digit '9' not found");
}

#[test]
fn test_iterators_provide_word_metadata() {
    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    let (image_data, width, height) = load_test_image("sample_text.png").expect("Failed to load test image");
    api.set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32)
        .expect("Failed to set image");

    let page_iter = api.analyze_layout().expect("Failed to obtain page iterator");
    page_iter.begin();
    match page_iter.orientation() {
        Ok((orientation, writing_direction, _, _)) => {
            assert_eq!(orientation, TessOrientation::ORIENTATION_PAGE_UP);
            assert_eq!(writing_direction, TessWritingDirection::WRITING_DIRECTION_LEFT_TO_RIGHT);
        }
        Err(err) => {
            eprintln!(
                "Orientation metadata unavailable ({}); continuing without strict assertions",
                err
            );
        }
    }

    let mut bounding_boxes = 0;
    loop {
        let (left, top, right, bottom) = page_iter
            .bounding_box(TessPageIteratorLevel::RIL_WORD)
            .expect("Expected bounding box for word");
        assert!(left < right);
        assert!(top < bottom);
        bounding_boxes += 1;

        let has_more_page = page_iter.next(TessPageIteratorLevel::RIL_WORD);
        if !has_more_page {
            break;
        }
    }
    assert!(bounding_boxes > 0);

    api.recognize().expect("Recognition step failed");
    let result_iter = api.get_iterator().expect("Failed to obtain result iterator");
    let mut words = Vec::new();
    loop {
        let word = result_iter
            .get_utf8_text(TessPageIteratorLevel::RIL_WORD)
            .expect("Expected word text");
        let trimmed = word.trim();
        if !trimmed.is_empty() {
            words.push(trimmed.to_string());
            let confidence = result_iter
                .confidence(TessPageIteratorLevel::RIL_WORD)
                .expect("Expected confidence value");
            assert!((0.0..=100.0).contains(&confidence));
        }

        if !result_iter
            .next(TessPageIteratorLevel::RIL_WORD)
            .expect("Iterator advancement should succeed")
        {
            break;
        }
    }

    assert!(
        words.iter().any(|word| word.eq_ignore_ascii_case("This")),
        "Expected to capture known words, got {:?}",
        words
    );
}

#[test]
fn test_result_iterator_numeric_detection() {
    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");
    api.set_variable("tessedit_char_whitelist", "0123456789")
        .expect("Failed to whitelist digits");

    let (image_data, width, height) = load_test_image("digits.png").expect("Failed to load test image");
    api.set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32)
        .expect("Failed to set digits image");

    api.recognize().expect("Recognition step failed");

    let result_iter = api.get_iterator().expect("Iterator acquisition failed");
    let mut saw_numeric = false;

    loop {
        let word = result_iter
            .get_utf8_text(TessPageIteratorLevel::RIL_WORD)
            .expect("Expected recognized text");
        if !word.trim().is_empty() {
            let numeric = result_iter.word_is_numeric().expect("Numeric flag lookup failed");
            if numeric {
                saw_numeric = true;
                assert!(
                    word.trim().chars().all(|c| c.is_ascii_digit()),
                    "Expected numeric token, got {}",
                    word
                );
            }
        }

        if !result_iter
            .next(TessPageIteratorLevel::RIL_WORD)
            .expect("Iterator advancement should succeed")
        {
            break;
        }
    }

    assert!(saw_numeric, "Expected numeric words in digits image");
}

#[test]
fn test_language_and_confidence_helpers() {
    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    let (image_data, width, height) = load_test_image("sample_text.png").expect("Failed to load test image");
    api.set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32)
        .expect("Failed to set image");
    api.recognize().expect("Recognition failed");
    let full_text = api.get_utf8_text().expect("Failed to get full OCR output");

    assert!(
        full_text.contains("sample text"),
        "Unexpected OCR output: {}",
        full_text
    );

    let word_confidences = api.get_word_confidences().expect("Failed to gather word confidences");
    assert!(
        !word_confidences.is_empty(),
        "Expected confidences for recognized words"
    );
    assert!(word_confidences.iter().all(|&c| (0..=100).contains(&c)));

    let all_confidences = api
        .all_word_confidences()
        .expect("Failed to gather all word confidences");
    assert_eq!(
        word_confidences.len(),
        all_confidences.len(),
        "Different confidence lengths"
    );

    let mean_conf = api.mean_text_conf().expect("Failed to get mean confidence");
    assert!((0..=100).contains(&mean_conf));

    let loaded = api.get_loaded_languages().expect("Failed to query loaded languages");
    assert!(
        loaded.iter().any(|lang| lang == "eng"),
        "Expected 'eng' in loaded languages: {:?}",
        loaded
    );

    let available = api
        .get_available_languages()
        .expect("Failed to query available languages");
    assert!(
        available.iter().any(|lang| lang == "eng"),
        "Expected 'eng' to be available: {:?}",
        available
    );
    assert!(
        available.iter().any(|lang| lang == "tur"),
        "Expected 'tur' to be available: {:?}",
        available
    );

    let init_langs = api
        .get_init_languages_as_string()
        .expect("Failed to fetch initialized languages");
    assert!(
        init_langs.contains("eng"),
        "Initialized languages missing 'eng': {}",
        init_langs
    );

    let datapath = api.get_datapath().expect("Failed to fetch datapath");
    assert!(Path::new(&datapath).exists(), "Datapath does not exist: {}", datapath);

    api.set_input_name("sample_text.png").expect("Failed to set input name");
    let input_name = api.get_input_name().expect("Failed to read back input name");
    assert!(
        input_name.ends_with("sample_text.png"),
        "Input name mismatch: {}",
        input_name
    );

    api.clear_adaptive_classifier()
        .expect("Failed to clear adaptive classifier");
}

#[test]
fn test_structured_outputs_and_rectangles() {
    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    let (image_data, width, height) = load_test_image("sample_text.png").expect("Failed to load test image");
    api.set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32)
        .expect("Failed to set image");
    api.recognize().expect("Recognition failed");
    let full_text = api
        .get_utf8_text()
        .expect("Failed to get full OCR output for rectangle comparison");

    let hocr = api.get_hocr_text(0).expect("Failed to get hOCR output");
    assert!(hocr.contains("ocr_page"), "hOCR output missing expected marker");

    let tsv = api.get_tsv_text(0).expect("Failed to get TSV output");
    assert!(
        tsv.contains('\t'),
        "TSV output missing expected tab separation: {}",
        tsv
    );

    let unlv = api.get_unlv_text().expect("Failed to get UNLV output");
    assert!(
        unlv.to_lowercase().contains("sample"),
        "UNLV output missing text content: {}",
        unlv
    );

    let api_with_rect = TesseractAPI::new();
    api_with_rect
        .init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize API for rectangle test");
    api_with_rect
        .set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32)
        .expect("Failed to set image for rectangle test");
    api_with_rect
        .set_rectangle(0, 0, (width / 2) as i32, height as i32)
        .expect("Failed to set rectangle");
    let partial_text = api_with_rect.get_utf8_text().expect("Failed to run OCR on rectangle");
    assert!(
        partial_text.len() < full_text.len(),
        "Rectangle-based recognition should reduce extracted text"
    );
}

#[test]
fn test_page_seg_mode_roundtrip() {
    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    api.set_page_seg_mode(TessPageSegMode::PSM_SINGLE_BLOCK)
        .expect("Failed to set PSM");
    let mode = api.get_page_seg_mode().expect("Failed to fetch PSM");
    assert_eq!(mode, TessPageSegMode::PSM_SINGLE_BLOCK);

    api.set_page_seg_mode(TessPageSegMode::PSM_AUTO_OSD)
        .expect("Failed to set second PSM");
    let second = api.get_page_seg_mode().expect("Failed to fetch second PSM");
    assert_eq!(second, TessPageSegMode::PSM_AUTO_OSD);
}

#[test]
fn test_thresholded_image_helpers() {
    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    api.set_source_resolution(300).expect("Failed to set DPI");

    let (image_data, width, height) = load_test_image("sample_text.png").expect("Failed to load test image");
    api.set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32)
        .expect("Failed to set image");
    api.recognize().expect("Recognition failed");

    let thresholded_image = api.get_thresholded_image().expect("Failed to obtain thresholded image");
    assert!(
        !thresholded_image.is_null(),
        "Expected thresholded image pointer to be non-null"
    );

    let scale_factor = api
        .get_thresholded_image_scale_factor()
        .expect("Failed to get threshold scale factor");
    assert!(
        scale_factor >= 1,
        "Unexpected scale factor returned from Tesseract: {}",
        scale_factor
    );

    let y_res = api
        .get_source_y_resolution()
        .expect("Failed to query source resolution");
    assert!(
        y_res >= 70,
        "Expected Tesseract to report a plausible DPI, got {}",
        y_res
    );
}

#[test]
fn test_text_direction_metrics() {
    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    let (image_data, width, height) = load_test_image("sample_text.png").expect("Failed to load test image");
    api.set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32)
        .expect("Failed to set image");
    api.recognize().expect("Recognition failed");

    let (text_dir_deg, text_dir_conf) = api.get_text_direction().expect("Failed to query text direction");
    assert!(
        text_dir_conf.is_finite(),
        "Text direction confidence should be a finite number"
    );
    assert!(
        (0..=360).contains(&text_dir_deg),
        "Text direction degrees should fall within one rotation, got {}",
        text_dir_deg
    );
}

#[test]
fn test_is_valid_word_and_clear() {
    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new();
    api.init(tessdata_dir.to_str().unwrap(), "eng")
        .expect("Failed to initialize Tesseract");

    let (image_data, width, height) = load_test_image("sample_text.png").expect("Failed to load test image");
    api.set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32)
        .expect("Failed to set image");

    api.recognize().expect("Recognition failed");
    let text = api.get_utf8_text().expect("Failed to gather text");
    assert!(!text.is_empty(), "Expected non-empty OCR output");

    let validity = api.is_valid_word("sample").expect("Failed to validate word");
    assert!(
        validity > 0,
        "Expected 'sample' to be recognised as a valid dictionary word"
    );

    api.clear().expect("Failed to clear OCR engine");
    api.set_image(&image_data, width as i32, height as i32, 3, 3 * width as i32)
        .expect("Failed to reset image after clear");
    api.recognize().expect("Recognition after clear failed");
    let rerun_text = api.get_utf8_text().expect("Failed to gather text after clear");
    assert!(
        !rerun_text.is_empty(),
        "Expected OCR output after clearing and re-recognizing"
    );
}
