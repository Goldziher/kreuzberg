//! Diagnostic test to trace PaddleOCR detection pipeline.
//!
//! This test isolates each step to determine where empty results originate.
//! Since this crate doesn't have PNG/image decoder features, we create test
//! images programmatically.

#![cfg(feature = "ort")] // ~keep: this diagnostic drives the `ort` session API directly
#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests

use std::path::PathBuf;

fn get_workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap().parent().unwrap().to_path_buf()
}

fn get_model_dir() -> PathBuf {
    get_workspace_root().join(".xberg/paddle-ocr")
}

/// Create a simple test image with black text "HELLO" on white background.
/// This avoids needing PNG decoder features.
fn create_test_image() -> image::RgbImage {
    let width = 200u32;
    let height = 100u32;
    let mut img = image::RgbImage::from_pixel(width, height, image::Rgb([255, 255, 255]));

    let black = image::Rgb([0, 0, 0]);

    for y in 20..80 {
        img.put_pixel(20, y, black);
        img.put_pixel(21, y, black);
        img.put_pixel(22, y, black);
    }
    for y in 20..80 {
        img.put_pixel(55, y, black);
        img.put_pixel(56, y, black);
        img.put_pixel(57, y, black);
    }
    for x in 20..58 {
        img.put_pixel(x, 48, black);
        img.put_pixel(x, 49, black);
        img.put_pixel(x, 50, black);
    }

    for y in 30..70 {
        for x in 80..180 {
            img.put_pixel(x, y, black);
        }
    }

    img
}

#[test]
fn diagnostic_detection_pipeline() {
    let model_dir = get_model_dir();

    if !model_dir.join("det/model.onnx").exists() {
        eprintln!("SKIP: Models not downloaded at {:?}", model_dir);
        return;
    }

    discover_ort();

    eprintln!("=== PaddleOCR Diagnostic Test ===");
    eprintln!("Model dir: {:?}", model_dir);

    let img = create_test_image();
    eprintln!("Step 1 - Test image created: {}x{}", img.width(), img.height());

    let mut ocr_engine = xberg_paddle_ocr::PaddleOcrEngine::new();
    let det_path = model_dir.join("det/model.onnx");
    let cls_path = model_dir.join("cls/model.onnx");
    let rec_path = model_dir.join("rec/model.onnx");

    let init_result = ocr_engine.init_models(
        det_path.to_str().unwrap(),
        cls_path.to_str().unwrap(),
        rec_path.to_str().unwrap(),
        1,
    );

    match &init_result {
        Ok(()) => eprintln!("Step 2 - Models initialized successfully"),
        Err(e) => {
            eprintln!("Step 2 - FAILED to init models: {:?}", e);
            panic!("Model initialization failed: {:?}", e);
        }
    }

    let test_cases = vec![
        ("A: Default params", 50u32, 960u32, 0.3f32, 0.5f32, 1.6f32, true, false),
        ("B: Very low thresholds", 50, 960, 0.01, 0.01, 1.6, false, false),
        ("C: No padding + low", 0, 960, 0.01, 0.01, 1.6, false, false),
        ("D: Higher unclip ratio", 50, 960, 0.1, 0.1, 3.0, false, false),
        ("E: No padding + medium", 0, 960, 0.1, 0.3, 2.0, false, false),
    ];

    let mut any_detected = false;

    for (name, padding, max_side, box_score, box_thresh, unclip, do_angle, most_angle) in &test_cases {
        eprintln!("\n--- Test {} ---", name);
        eprintln!(
            "  padding={}, max_side={}, box_score={}, box_thresh={}, unclip={}",
            padding, max_side, box_score, box_thresh, unclip
        );

        let result = ocr_engine.detect(
            &img,
            *padding,
            *max_side,
            *box_score,
            *box_thresh,
            *unclip,
            *do_angle,
            *most_angle,
        );

        match &result {
            Ok(ocr_result) => {
                eprintln!("  Result: {} text blocks", ocr_result.text_blocks.len());
                for (i, block) in ocr_result.text_blocks.iter().enumerate() {
                    eprintln!(
                        "    Block {}: text='{}', text_score={:.3}, box_score={:.3}",
                        i, block.text, block.text_score, block.box_score
                    );
                    any_detected = true;
                }
            }
            Err(e) => {
                eprintln!("  FAILED: {:?}", e);
            }
        }
    }

    eprintln!("\n=== Diagnosis ===");
    if !any_detected {
        eprintln!("RESULT: Detection model produces NO output regardless of thresholds.");
        eprintln!("This strongly suggests an ORT version compatibility issue.");
        eprintln!("  ort crate version: check Cargo.lock for current version");
        eprintln!("  ORT_DYLIB_PATH: {:?}", std::env::var("ORT_DYLIB_PATH"));
    } else {
        eprintln!("RESULT: Detection works. Issue may be threshold-related or image-specific.");
    }
}

/// Also test with raw ONNX inference to check if ORT works at all.
#[test]
fn diagnostic_raw_ort_inference() {
    let model_dir = get_model_dir();
    let det_model = model_dir.join("det/model.onnx");

    if !det_model.exists() {
        eprintln!("SKIP: Detection model not found at {:?}", det_model);
        return;
    }

    discover_ort();

    eprintln!("=== Raw ORT Inference Test ===");

    use ort::session::Session;

    let mut session = Session::builder(&ort::init().build().unwrap())
        .unwrap()
        .commit_from_file(&det_model)
        .unwrap();

    eprintln!("Model loaded successfully");
    eprintln!("Inputs:");
    for input in session.inputs() {
        eprintln!("  name='{}'", input.name());
    }
    eprintln!("Outputs:");
    for output in session.outputs() {
        eprintln!("  name='{}'", output.name());
    }

    let input_data: Vec<f32> = vec![0.5; 3 * 32 * 32];
    let tensor =
        ort::value::Tensor::from_array(ndarray::Array::from_shape_vec((1, 3, 32, 32), input_data).unwrap()).unwrap();

    let input_name = session.inputs()[0].name().to_string();
    eprintln!("\nRunning inference with 32x32 gray image...");

    let outputs = session.run(ort::inputs![input_name => tensor]).unwrap();

    let (output_name, output_value) = outputs.iter().next().unwrap();
    eprintln!("Output name: {}", output_name);

    let output_tensor = output_value.try_extract_tensor::<f32>().unwrap();
    let output_shape = output_tensor.0;
    let output_data = output_tensor.1;

    eprintln!("Output shape: {:?}", output_shape);
    eprintln!("Output len: {}", output_data.len());

    if !output_data.is_empty() {
        let min = output_data.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = output_data.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let sum: f32 = output_data.iter().sum();
        let mean = sum / output_data.len() as f32;
        let non_zero = output_data.iter().filter(|&&v| v > 0.001).count();

        eprintln!("Output stats: min={:.6}, max={:.6}, mean={:.6}", min, max, mean);
        eprintln!("Non-zero values (>0.001): {} / {}", non_zero, output_data.len());

        if max < 0.001 {
            eprintln!("\nDIAGNOSIS: Model outputs are essentially all zeros.");
            eprintln!("This confirms an ORT compatibility issue - model isn't executing correctly.");
        } else {
            eprintln!("\nDIAGNOSIS: Model produces non-zero output. ORT is working.");
        }
    }
}

/// Diagnostic: test the CRNN recognition model directly.
#[test]
fn diagnostic_crnn_model_output() {
    let model_dir = get_model_dir();
    let rec_model = model_dir.join("rec/model.onnx");

    if !rec_model.exists() {
        eprintln!("SKIP: Recognition model not found");
        return;
    }

    discover_ort();

    eprintln!("=== CRNN Recognition Model Diagnostic ===");

    use ort::session::Session;

    let mut session = Session::builder(&ort::init().build().unwrap())
        .unwrap()
        .commit_from_file(&rec_model)
        .unwrap();

    eprintln!("Model loaded successfully");
    eprintln!("Inputs:");
    for input in session.inputs() {
        eprintln!("  name='{}'", input.name());
    }
    eprintln!("Outputs:");
    for output in session.outputs() {
        eprintln!("  name='{}'", output.name());
    }

    {
        let metadata = session.metadata().unwrap();

        eprintln!("Model metadata:");
        eprintln!("  description: {:?}", metadata.description());
        eprintln!("  producer: {:?}", metadata.producer());

        match metadata.custom("character") {
            Some(chars) => {
                let bytes = chars.as_bytes();
                let char_count = chars.split('\n').count();
                eprintln!(
                    "  custom('character'): len={}, bytes={}, split_count={}",
                    chars.len(),
                    bytes.len(),
                    char_count
                );
                if chars.len() < 500 {
                    eprintln!("  value: {:?}", chars);
                } else {
                    let preview: String = chars.chars().take(100).collect();
                    eprintln!("  preview (first 100 chars): {:?}", preview);
                }

                let null_count = bytes.iter().filter(|&&b| b == 0).count();
                if null_count > 0 {
                    eprintln!("  WARNING: {} null bytes found in character string!", null_count);
                }
            }
            None => {
                eprintln!("  ERROR: No 'character' key in model metadata!");
            }
        }

        for key in [
            "character",
            "characters",
            "dict",
            "dictionary",
            "labels",
            "vocab",
            "alphabet",
        ] {
            if let Some(val) = metadata.custom(key) {
                eprintln!(
                    "  custom('{}'): len={}, preview={:?}",
                    key,
                    val.len(),
                    &val[..val.len().min(80)]
                );
            }
        }
    }

    let h = 48usize;
    let w = 200usize;

    let mut input_data: Vec<f32> = vec![0.0; 3 * h * w];
    for c in 0..3 {
        for y in 10..38 {
            for x in (20..180).step_by(2) {
                input_data[c * h * w + y * w + x] = -1.0;
            }
        }
    }

    let tensor =
        ort::value::Tensor::from_array(ndarray::Array::from_shape_vec((1, 3, h, w), input_data).unwrap()).unwrap();

    let input_name = session.inputs()[0].name().to_string();
    eprintln!("\nRunning CRNN with striped pattern (48x200)...");

    let outputs = session.run(ort::inputs![input_name => tensor]).unwrap();

    let (_, output_value) = outputs.iter().next().unwrap();
    let (shape, data) = output_value.try_extract_tensor::<f32>().unwrap();

    eprintln!("Output shape: {:?}", shape);
    eprintln!("Output total values: {}", data.len());

    if shape.len() >= 3 {
        let time_steps = shape[1] as usize;
        let vocab_size = shape[2] as usize;
        eprintln!("Time steps: {}, Vocabulary size: {}", time_steps, vocab_size);

        let data_vec: Vec<f32> = data.to_vec();
        let min = data_vec.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = data_vec.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let mean: f32 = data_vec.iter().sum::<f32>() / data_vec.len() as f32;
        eprintln!("Overall stats: min={:.6}, max={:.6}, mean={:.6}", min, max, mean);

        let mut argmax_zero_count = 0;
        let mut argmax_nonzero_count = 0;
        for t in 0..time_steps {
            let start = t * vocab_size;
            let end = start + vocab_size;
            let slice = &data_vec[start..end.min(data_vec.len())];

            let (max_idx, max_val) =
                slice.iter().enumerate().fold(
                    (0, f32::MIN),
                    |(mi, mv), (i, &v)| if v > mv { (i, v) } else { (mi, mv) },
                );

            if max_idx == 0 {
                argmax_zero_count += 1;
            } else {
                argmax_nonzero_count += 1;
            }

            if t < 5 || (t > time_steps - 3) {
                eprintln!("  Step {}: argmax={}, max_val={:.4}", t, max_idx, max_val);
            } else if t == 5 {
                eprintln!("  ... (skipping middle steps)");
            }
        }

        eprintln!(
            "\nArgmax distribution: {} blank (idx=0), {} non-blank",
            argmax_zero_count, argmax_nonzero_count
        );

        if argmax_nonzero_count == 0 {
            eprintln!("\nDIAGNOSIS: CRNN model outputs all blanks.");
            eprintln!("Possible causes:");
            eprintln!("  1. ORT version incompatibility with CRNN model");
            eprintln!("  2. Model is not executing graph correctly");
            eprintln!("  3. Input normalization mismatch");
        } else {
            eprintln!("\nDIAGNOSIS: CRNN model produces non-blank output. Recognition works.");
        }
    }

    drop(outputs);

    let white_data: Vec<f32> = vec![1.0; 3 * h * w];
    let white_tensor =
        ort::value::Tensor::from_array(ndarray::Array::from_shape_vec((1, 3, h, w), white_data).unwrap()).unwrap();

    let input_name2 = session.inputs()[0].name().to_string();
    eprintln!("\nRunning CRNN with uniform white (48x200)...");
    let white_outputs = session.run(ort::inputs![input_name2 => white_tensor]).unwrap();
    let (_, white_val) = white_outputs.iter().next().unwrap();
    let (_, white_data_out) = white_val.try_extract_tensor::<f32>().unwrap();
    let white_vec: Vec<f32> = white_data_out.to_vec();
    let white_max = white_vec.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let white_min = white_vec.iter().cloned().fold(f32::INFINITY, f32::min);
    eprintln!("White image output: min={:.6}, max={:.6}", white_min, white_max);
}

fn discover_ort() {
    if let Ok(path) = std::env::var("ORT_DYLIB_PATH")
        && std::path::Path::new(&path).exists()
    {
        eprintln!("ORT found via ORT_DYLIB_PATH: {}", path);
        return;
    }

    let candidates = [
        "/opt/homebrew/lib/libonnxruntime.dylib",
        "/usr/local/lib/libonnxruntime.dylib",
    ];

    for candidate in &candidates {
        if std::path::Path::new(candidate).exists() {
            eprintln!("Setting ORT_DYLIB_PATH={}", candidate);
            unsafe { std::env::set_var("ORT_DYLIB_PATH", candidate) };
            return;
        }
    }

    eprintln!("WARNING: Could not find ORT library!");
}
