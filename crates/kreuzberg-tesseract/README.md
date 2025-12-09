# kreuzberg-tesseract

Rust bindings for Tesseract OCR with built-in compilation of Tesseract and Leptonica libraries. Provides a safe and idiomatic Rust interface to Tesseract's functionality while handling the complexity of compiling the underlying C++ libraries.

Based on the original [tesseract-rs](https://github.com/cafercangundogdu/tesseract-rs) by Cafer Can Gündoğdu, this maintained version adds critical improvements for production use:

- **C++17 Support**: Upgraded for Tesseract 5.5.1 which requires C++17 filesystem
- **Cross-Compilation**: Fixed CXX compiler detection for cross-platform builds
- **Architecture Validation**: Validates target architecture before using cached libraries
- **Windows Static Linking**: Fixed MSVC static linking issues
- **Build Caching**: Improved caching with OUT_DIR-based cache directory
- **MinGW Support**: Added support for MinGW toolchains

## Features

- Safe Rust bindings for Tesseract OCR
- **Multiple linking options:**
  - **Static linking** (default): Built-in compilation with no runtime dependencies
  - **Dynamic linking**: Link to system-installed libraries for faster builds
- Uses existing Tesseract training data (expects English data for tests)
- High-level Rust API for common OCR tasks
- Caching of compiled libraries for faster subsequent builds
- Support for multiple operating systems (Linux, macOS, Windows)

## Installation

### Static Linking (Default)

Static linking builds Tesseract and Leptonica from source and embeds them in your binary. No runtime dependencies required:

```toml
[dependencies]
kreuzberg-tesseract = "1.0.0-rc.1"
# or explicitly:
kreuzberg-tesseract = { version = "1.0.0-rc.1", features = ["static-linking"] }
```

### Dynamic Linking

Dynamic linking uses system-installed Tesseract and Leptonica libraries. Faster builds, but requires libraries installed on the system:

```toml
[dependencies]
kreuzberg-tesseract = { version = "1.0.0-rc.1", features = ["dynamic-linking"], default-features = false }
```

**System requirements for dynamic linking:**
- Tesseract 5.x libraries installed (`libtesseract`, `libleptonica`)
- macOS: `brew install tesseract leptonica`
- Ubuntu/Debian: `sudo apt-get install libtesseract-dev libleptonica-dev`
- RHEL/CentOS/Fedora: `sudo dnf install tesseract-devel leptonica-devel`
- Windows: Install from [Tesseract releases](https://github.com/tesseract-ocr/tesseract/releases) or vcpkg

### Development Dependencies

For development and testing, you'll also need these dependencies:

```toml
[dev-dependencies]
image = "0.25.5"
```

## System Requirements

### For Static Linking (Default)

When building with static linking, the crate will compile Tesseract and Leptonica from source. You need:

- Rust 1.85.0 or later
- A C++ compiler (e.g., gcc, clang, MSVC on Windows)
- CMake 3.x or later
- Internet connection (for downloading Tesseract source code)

### For Dynamic Linking

When using dynamic linking with system-installed libraries, you need:

- Rust 1.85.0 or later
- Tesseract 5.x and Leptonica libraries installed on your system (see Installation section)
- Internet connection (for downloading Tesseract source code)

No C++ compiler or CMake required for dynamic linking builds.

For a full development environment checklist (including optional tooling suggestions), see [CONTRIBUTING.md](CONTRIBUTING.md).

## Environment Variables

The following environment variables affect the build and test process:

### Build Variables

- `CARGO_CLEAN`: If set, cleans the cache directory before building
- `RUSTC_WRAPPER`: If set to "sccache", enables compiler caching with sccache
- `CC`: Compiler selection for C code (affects Linux builds)
- `HOME` (Unix) or `APPDATA` (Windows): Used to determine cache directory location
- `TESSERACT_RS_CACHE_DIR`: Optional override for the cache root. When unset or not writable, the build falls back to the default OS-specific directory, and if that still fails, a temporary directory under the system temp folder is used automatically.

### Test Variables

- `TESSDATA_PREFIX` (Optional): Path to override the default tessdata directory. If not set, the crate will use its default cache directory.

## Cache and Data Directories

The crate uses the following directory structure based on your operating system:

- macOS: `~/Library/Application Support/tesseract-rs`
- Linux: `~/.tesseract-rs`
- Windows: `%APPDATA%/tesseract-rs`

The cache includes:

- Compiled Tesseract and Leptonica libraries
- Third-party source code

Training data is not downloaded during the build. Provide `eng.traineddata` (and any other languages you need) via `TESSDATA_PREFIX` or your system Tesseract installation.

## Testing

The project includes several integration tests that verify OCR functionality. To run the tests:

1. Ensure you have the required test dependencies:

   ```toml
   [dev-dependencies]
   image = "0.25.9"
   ```

2. Run the tests:
   ```bash
   cargo test
   ```

Note: Make sure `eng.traineddata` is available in your tessdata directory before running tests. If `TESSDATA_PREFIX` is not set, the tests look in the default cache location. You can point the tests at a custom tessdata directory by setting:

```bash
# Linux/macOS
export TESSDATA_PREFIX=/path/to/custom/tessdata

# Windows (PowerShell)
$env:TESSDATA_PREFIX="C:\path\to\custom\tessdata"
```

Available test cases:

- OCR on English sample images
- Error handling and invalid input coverage

Test images are sourced from the shared `test_documents/` directory in the repository:

- `images/test_hello_world.png`: Simple English text
- `tables/simple_table.png`: Basic table with English headers

## Usage

Here's a basic example of how to use `tesseract-rs`:

```rust
use std::path::PathBuf;
use std::error::Error;
use kreuzberg_tesseract::TesseractAPI;

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
        PathBuf::from(home_dir)
            .join(".tesseract-rs")
            .join("tessdata")
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
            println!(
                "TESSDATA_PREFIX not set, using default directory: {:?}",
                default_dir
            );
            default_dir
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let api = TesseractAPI::new()?;

    // Get tessdata directory (uses default location or TESSDATA_PREFIX if set)
    let tessdata_dir = get_tessdata_dir();
    api.init(tessdata_dir.to_str().unwrap(), "eng")?;

    let width = 24;
    let height = 24;
    let bytes_per_pixel = 1;
    let bytes_per_line = width * bytes_per_pixel;

    // Initialize image data with all white pixels
    let mut image_data = vec![255u8; width * height];

    // Draw number 9 with clearer distinction
    for y in 4..19 {
        for x in 7..17 {
            // Top bar
            if y == 4 && x >= 8 && x <= 15 {
                image_data[y * width + x] = 0;
            }
            // Top curve left side
            if y >= 4 && y <= 10 && x == 7 {
                image_data[y * width + x] = 0;
            }
            // Top curve right side
            if y >= 4 && y <= 11 && x == 16 {
                image_data[y * width + x] = 0;
            }
            // Middle bar
            if y == 11 && x >= 8 && x <= 15 {
                image_data[y * width + x] = 0;
            }
            // Bottom right vertical line
            if y >= 11 && y <= 18 && x == 16 {
                image_data[y * width + x] = 0;
            }
            // Bottom bar
            if y == 18 && x >= 8 && x <= 15 {
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
    )?;

    // Set whitelist for digits only
    api.set_variable("tessedit_char_whitelist", "0123456789")?;

    // Set PSM mode to single character
    api.set_variable("tessedit_pageseg_mode", "10")?;

    // Get the recognized text
    let text = api.get_utf8_text()?;
    println!("Recognized text: {}", text.trim());

    Ok(())
}
```

## Advanced Usage

The API provides additional functionality for more complex OCR tasks, including thread-safe operations:

```rust
use kreuzberg_tesseract::TesseractAPI;
use std::sync::Arc;
use std::thread;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let tessdata_dir = get_tessdata_dir();
    let api = TesseractAPI::new()?;

    // Initialize the main API
    api.init(tessdata_dir.to_str().unwrap(), "eng")?;
    api.set_variable("tessedit_pageseg_mode", "1")?;

    // Load and prepare image data
    let (image_data, width, height) = load_test_image("sample_text.png")?;

    // Share image data across threads
    let image_data = Arc::new(image_data);
    let mut handles = vec![];

    // Spawn multiple threads for parallel OCR processing
    for _ in 0..3 {
        let api_clone = api.clone(); // Clones the API with all configurations
        let image_data = Arc::clone(&image_data);

        let handle = thread::spawn(move || {
            // Set image in each thread
            let res = api_clone.set_image(
                &image_data,
                width as i32,
                height as i32,
                3,
                3 * width as i32,
            );
            assert!(res.is_ok());

            // Perform OCR in parallel
            let text = api_clone.get_utf8_text()
                .expect("Failed to get text");
            println!("Thread result: {}", text);
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    Ok(())
}

// Helper function to get tessdata directory
fn get_tessdata_dir() -> PathBuf {
    // ... (implementation as shown in basic example)
}

// Helper function to load test image
fn load_test_image(filename: &str) -> Result<(Vec<u8>, u32, u32), Box<dyn Error>> {
    let img = image::open(filename)?
        .to_rgb8();
    let (width, height) = img.dimensions();
    Ok((img.into_raw(), width, height))
}
```

## Building

### Static Linking (Default)

With static linking, the crate will automatically download and compile Tesseract and Leptonica during the build process. This may take some time on the first build (5-10 minutes), but subsequent builds will use the cached libraries.

To clean the cache and force a rebuild:

```bash
CARGO_CLEAN=1 cargo build
```

### Dynamic Linking

With dynamic linking, the build is much faster (seconds instead of minutes) since it only links against system-installed libraries:

```bash
cargo build --no-default-features --features dynamic-linking
```

**Note**: Dynamic linking requires Tesseract and Leptonica to be installed on your system (see Installation section).

## Documentation

For more detailed information, please check the [API documentation](https://docs.rs/kreuzberg-tesseract).

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgements

This project is based on the original [tesseract-rs](https://github.com/cafercangundogdu/tesseract-rs) by [Cafer Can Gündoğdu](https://github.com/cafercangundogdu). We are grateful for the foundational work that made this project possible.

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Quick Start for Contributors

1. Fork and clone the repository
2. Install uv and set up git hooks:
   ```bash
   curl -LsSf https://astral.sh/uv/install.sh | sh
   uvx prek install
   ```
3. Make your changes following our commit message format
4. Run tests: `cargo test`
5. Submit a Pull Request

Our commit messages follow the [Conventional Commits](https://www.conventionalcommits.org/) specification.

## Acknowledgements

This project uses [Tesseract OCR](https://github.com/tesseract-ocr/tesseract) and [Leptonica](http://leptonica.org/). We are grateful to the maintainers and contributors of these projects.

```

```
