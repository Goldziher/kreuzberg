use criterion::{Criterion, criterion_group, criterion_main};
use kreuzberg_tesseract::TesseractAPI;
use std::hint::black_box;
use std::path::PathBuf;

fn get_default_tessdata_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        let home_dir = std::env::var("HOME").expect("HOME environment variable not set");
        PathBuf::from(home_dir)
            .join("Library")
            .join("Application Support")
            .join("tesseract-rs")
            .join("tessdata")
    } else if cfg!(target_os = "linux") {
        // Try system tesseract installation first (Ubuntu/Debian), then user home
        let system_paths = [
            PathBuf::from("/usr/share/tesseract-ocr/5/tessdata"),
            PathBuf::from("/usr/share/tesseract-ocr/tessdata"),
        ];
        for path in &system_paths {
            if path.exists() {
                return path.clone();
            }
        }
        // Fallback to user home directory
        let home_dir = std::env::var("HOME").expect("HOME environment variable not set");
        PathBuf::from(home_dir).join(".tesseract-rs").join("tessdata")
    } else if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("APPDATA").expect("APPDATA environment variable not set"))
            .join("tesseract-rs")
            .join("tessdata")
    } else {
        PathBuf::from("/tmp/tessdata")
    }
}

fn get_tessdata_dir() -> PathBuf {
    match std::env::var("TESSDATA_PREFIX") {
        Ok(dir) => {
            let prefix_path = PathBuf::from(dir);
            // TESSDATA_PREFIX can point to either:
            // 1. The tessdata directory itself (/path/to/tessdata)
            // 2. The parent directory (/path/to where tessdata is the subdirectory)
            let tessdata_path = if prefix_path.ends_with("tessdata") {
                prefix_path
            } else {
                prefix_path.join("tessdata")
            };
            println!("Using TESSDATA_PREFIX directory: {:?}", tessdata_path);
            tessdata_path
        }
        Err(_) => {
            let default_dir = get_default_tessdata_dir();
            println!("TESSDATA_PREFIX not set, using default directory: {:?}", default_dir);
            default_dir
        }
    }
}

fn benchmark_simple_ocr(c: &mut Criterion) {
    let tessdata_dir = get_tessdata_dir();

    c.bench_function("simple_ocr", |b| {
        let api = TesseractAPI::new();
        api.init(
            tessdata_dir.to_str().expect("tessdata path contains invalid UTF-8"),
            "eng",
        )
        .expect("Failed to initialize Tesseract");

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
            .expect("Failed to set image");

            let _text = api.get_utf8_text().expect("Failed to perform OCR");
        });
    });
}

fn benchmark_with_variables(c: &mut Criterion) {
    let tessdata_dir = get_tessdata_dir();

    c.bench_function("ocr_with_variables", |b| {
        let api = TesseractAPI::new();
        api.init(
            tessdata_dir.to_str().expect("tessdata path contains invalid UTF-8"),
            "eng",
        )
        .expect("Failed to initialize Tesseract");

        let width = 24;
        let height = 24;
        let image_data = vec![255u8; width * height];

        b.iter(|| {
            api.set_variable("tessedit_char_whitelist", "0123456789")
                .expect("Failed to set char whitelist");
            api.set_variable("tessedit_pageseg_mode", "10")
                .expect("Failed to set page seg mode");

            api.set_image(
                black_box(&image_data),
                black_box(width as i32),
                black_box(height as i32),
                black_box(1),
                black_box(width as i32),
            )
            .expect("Failed to set image");

            let _text = api.get_utf8_text().expect("Failed to perform OCR");
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
