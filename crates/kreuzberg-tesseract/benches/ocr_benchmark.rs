use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::path::PathBuf;
use tesseract_rs::TesseractAPI;

fn get_default_tessdata_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home_dir)
            .join("Library")
            .join("Application Support")
            .join("tesseract-rs")
            .join("tessdata")
    } else if cfg!(target_os = "linux") {
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home_dir).join(".tesseract-rs").join("tessdata")
    } else if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("APPDATA").unwrap_or_else(|_| "C:\\temp".to_string()))
            .join("tesseract-rs")
            .join("tessdata")
    } else {
        PathBuf::from("/tmp/tessdata")
    }
}

fn benchmark_simple_ocr(c: &mut Criterion) {
    let tessdata_dir = get_default_tessdata_dir();

    c.bench_function("simple_ocr", |b| {
        let api = TesseractAPI::new();
        api.init(tessdata_dir.to_str().unwrap(), "eng").unwrap();

        // Create a simple test image (24x24 white image with a black digit)
        let width = 24;
        let height = 24;
        let mut image_data = vec![255u8; width * height];

        // Draw a simple pattern
        for y in 8..16 {
            for x in 8..16 {
                if y == 8 || y == 15 || x == 8 || x == 15 {
                    image_data[y * width + x] = 0;
                }
            }
        }

        b.iter(|| {
            api.set_image(
                black_box(&image_data),
                black_box(width as i32),
                black_box(height as i32),
                black_box(1),
                black_box(width as i32),
            )
            .unwrap();

            let _text = api.get_utf8_text().unwrap();
        });
    });
}

fn benchmark_with_variables(c: &mut Criterion) {
    let tessdata_dir = get_default_tessdata_dir();

    c.bench_function("ocr_with_variables", |b| {
        let api = TesseractAPI::new();
        api.init(tessdata_dir.to_str().unwrap(), "eng").unwrap();

        let width = 24;
        let height = 24;
        let image_data = vec![255u8; width * height];

        b.iter(|| {
            api.set_variable("tessedit_char_whitelist", "0123456789").unwrap();
            api.set_variable("tessedit_pageseg_mode", "10").unwrap();

            api.set_image(
                black_box(&image_data),
                black_box(width as i32),
                black_box(height as i32),
                black_box(1),
                black_box(width as i32),
            )
            .unwrap();

            let _text = api.get_utf8_text().unwrap();
        });
    });
}

fn benchmark_api_creation(c: &mut Criterion) {
    c.bench_function("api_creation", |b| {
        b.iter(|| {
            let _api = black_box(TesseractAPI::new());
        });
    });
}

fn benchmark_api_clone(c: &mut Criterion) {
    let api = TesseractAPI::new();

    c.bench_function("api_clone", |b| {
        b.iter(|| {
            let _cloned = black_box(api.clone());
        });
    });
}

criterion_group!(
    benches,
    benchmark_simple_ocr,
    benchmark_with_variables,
    benchmark_api_creation,
    benchmark_api_clone
);
criterion_main!(benches);
