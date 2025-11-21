# Error Handling

Kreuzberg uses a comprehensive error system based on the `KreuzbergError` enum. All errors preserve the error chain for debugging and include context about what went wrong.

## Error Handling Philosophy

**System errors MUST always bubble up unchanged:**

- `KreuzbergError::Io` (from `std::io::Error`) - File system errors, permission errors
- These indicate real system problems that users need to know about
- Never wrap or suppress these - they must surface to enable bug reports

**Application errors are wrapped with context:**

- `Parsing` - Document format errors, corrupt files
- `Validation` - Invalid configuration or parameters
- `Ocr` - OCR processing failures
- `MissingDependency` - Missing optional system dependencies

## Error Variants

### KreuzbergError::Io

**When Raised:** File system operations, I/O failures, permission errors

**Source:** Automatically converted from `std::io::Error`

**Handling:** These errors MUST always bubble up unchanged. Never wrap or suppress I/O errors - they indicate real system problems that need to be reported.

**Common Causes:**

- File not found
- Permission denied
- Disk full
- Network mount unavailable
- Invalid file path

**Example (Rust):**

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig, KreuzbergError};

fn process_file(path: &str) -> kreuzberg::Result<String> {
    let config = ExtractionConfig::default();

    // IO errors bubble up automatically via ?
    let result = extract_file_sync(path, None, &config)?;

    Ok(result.content)
}

// Handle errors
match process_file("/nonexistent/file.pdf") {
    Ok(content) => println!("Content: {}", content),
    Err(KreuzbergError::Io(e)) => {
        // System error - log and report to user
        eprintln!("File system error: {}", e);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

**Example (Python):**

```python
from kreuzberg import extract_file, ExtractionConfig

try:
    result = extract_file("/nonexistent/file.pdf")
except OSError as e:
    # System error - log and report to user
    print(f"File system error: {e}")
except Exception as e:
    print(f"Other error: {e}")
```

**Example (Ruby):**

```ruby
require 'kreuzberg'

begin
  result = Kreuzberg.extract_file_sync('/nonexistent/file.pdf')
  puts result.content
rescue Kreuzberg::Errors::IOError => e
  # System error - log and report to user
  puts "File system error: #{e.message}"
rescue Kreuzberg::Errors::Error => e
  puts "Extraction error: #{e.message}"
end
```

**Example (Go):**

```go
import (
	"errors"
	"fmt"
	"kreuzberg"
)

result, err := kreuzberg.ExtractFileSync("/nonexistent/file.pdf", nil)
if err != nil {
	var ioErr *kreuzberg.IOError
	if errors.As(err, &ioErr) {
		// System error - log and report to user
		fmt.Printf("File system error: %v\n", err)
		return
	}
	fmt.Printf("Extraction error: %v\n", err)
	return
}
fmt.Println(result.Content)
```

---

### KreuzbergError::Parsing

**When Raised:** Document parsing failures, corrupt files, unsupported file features

**Source:** Various document parsing libraries (pdfium, calamine, mail-parser, etc.)

**Context:** Error message includes file type, parsing stage, and underlying cause

**Common Causes:**

- Corrupt or truncated files
- Invalid file format
- Unsupported file features
- Malformed XML/HTML
- Invalid Excel formulas
- Damaged PDF structure

**Example (Rust):**

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig, KreuzbergError};

fn extract_with_error_handling(path: &str) -> kreuzberg::Result<String> {
    let config = ExtractionConfig::default();

    match extract_file_sync(path, None, &config) {
        Ok(result) => Ok(result.content),
        Err(KreuzbergError::Parsing { message, source }) => {
            eprintln!("Parsing failed: {}", message);
            if let Some(src) = source {
                eprintln!("Caused by: {}", src);
            }
            Err(KreuzbergError::Parsing { message, source })
        }
        Err(e) => Err(e),
    }
}
```

**Example (Python):**

```python
from kreuzberg import extract_file, ParsingError

try:
    result = extract_file("corrupt.pdf")
except ParsingError as e:
    print(f"Document is corrupt or invalid: {e}")
    # Try alternative extraction method or notify user
```

**Example (Ruby):**

```ruby
require 'kreuzberg'

begin
  result = Kreuzberg.extract_file_sync('corrupt.pdf')
  puts result.content
rescue Kreuzberg::Errors::ParsingError => e
  puts "Document is corrupt or invalid: #{e.message}"
  puts "Context: #{e.context}" if e.context
  # Try alternative extraction method or notify user
rescue Kreuzberg::Errors::Error => e
  puts "Extraction error: #{e.message}"
end
```

**Example (Go):**

```go
import (
	"errors"
	"fmt"
	"kreuzberg"
)

result, err := kreuzberg.ExtractFileSync("corrupt.pdf", nil)
if err != nil {
	var parseErr *kreuzberg.ParsingError
	if errors.As(err, &parseErr) {
		fmt.Printf("Document is corrupt or invalid: %v\n", err)
		// Try alternative extraction method or notify user
		return
	}
	fmt.Printf("Extraction error: %v\n", err)
	return
}
fmt.Println(result.Content)
```

---

### KreuzbergError::Ocr

**When Raised:** OCR processing failures, Tesseract errors, image preprocessing issues

**Context:** Includes OCR configuration, image details, and failure reason

**Common Causes:**

- Tesseract not installed
- Unsupported language model
- Image too large or too small
- Invalid image format for OCR
- OCR timeout
- Table detection failures

**Example (Rust):**

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig, OcrConfig, KreuzbergError};

fn extract_with_ocr(path: &str) -> kreuzberg::Result<String> {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig::default()),
        force_ocr: false,
        ..Default::default()
    };

    match extract_file_sync(path, None, &config) {
        Ok(result) => Ok(result.content),
        Err(KreuzbergError::Ocr { message, .. }) => {
            eprintln!("OCR failed: {}", message);
            // Fall back to extraction without OCR
            let config_no_ocr = ExtractionConfig::default();
            extract_file_sync(path, None, &config_no_ocr)
        }
        Err(e) => Err(e),
    }
}
```

**Example (Python):**

```python
from kreuzberg import extract_file, ExtractionConfig, OcrConfig, OCRError

config = ExtractionConfig(ocr=OcrConfig())

try:
    result = extract_file("scanned.pdf", config=config)
except OCRError as e:
    print(f"OCR failed: {e}")
    # Fall back to extraction without OCR
    result = extract_file("scanned.pdf")
```

**Example (Ruby):**

```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  ocr: Kreuzberg::Config::OCR.new
)

begin
  result = Kreuzberg.extract_file_sync('scanned.pdf', config: config)
  puts result.content
rescue Kreuzberg::Errors::OCRError => e
  puts "OCR failed: #{e.message}"
  puts "Context: #{e.context}" if e.context
  # Fall back to extraction without OCR
  result = Kreuzberg.extract_file_sync('scanned.pdf')
  puts result.content
rescue Kreuzberg::Errors::Error => e
  puts "Extraction error: #{e.message}"
end
```

**Example (Go):**

```go
import (
	"errors"
	"fmt"
	"kreuzberg"
)

config := &kreuzberg.ExtractionConfig{
	OCR: &kreuzberg.OcrConfig{},
}

result, err := kreuzberg.ExtractFileSync("scanned.pdf", config)
if err != nil {
	var ocrErr *kreuzberg.OCRError
	if errors.As(err, &ocrErr) {
		fmt.Printf("OCR failed: %v\n", err)
		// Fall back to extraction without OCR
		result, _ = kreuzberg.ExtractFileSync("scanned.pdf", nil)
		fmt.Println(result.Content)
		return
	}
	fmt.Printf("Extraction error: %v\n", err)
	return
}
fmt.Println(result.Content)
```

---

### KreuzbergError::Validation

**When Raised:** Invalid configuration, invalid parameters, constraint violations

**Context:** Includes parameter name, invalid value, and validation rule

**Common Causes:**

- Invalid file path
- Negative dimensions or DPI values
- Unsupported language codes
- Invalid chunk size
- Invalid PSM/OEM values for Tesseract
- Empty required fields

**Example (Rust):**

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig, ImageExtractionConfig, KreuzbergError};

fn create_config() -> kreuzberg::Result<ExtractionConfig> {
    let images = ImageExtractionConfig {
        extract_images: true,
        target_dpi: -100,  // Invalid!
        ..Default::default()
    };

    // Validation happens during extraction
    Ok(ExtractionConfig {
        images: Some(images),
        ..Default::default()
    })
}

match create_config() {
    Ok(config) => println!("Config valid"),
    Err(KreuzbergError::Validation { message, .. }) => {
        eprintln!("Invalid configuration: {}", message);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

**Example (Python):**

```python
from kreuzberg import extract_file, ExtractionConfig, ImageExtractionConfig, ValidationError

try:
    config = ExtractionConfig(
        images=ImageExtractionConfig(target_dpi=-100)  # Invalid!
    )
    result = extract_file("document.pdf", config=config)
except ValidationError as e:
    print(f"Invalid configuration: {e}")
```

---

### KreuzbergError::Cache

**When Raised:** Cache read/write failures, cache corruption

**Handling:** Cache errors are generally **non-fatal** - operations continue without cache

**Common Causes:**

- Cache directory not writable
- Disk full
- Cache file corruption
- Concurrent cache access issues

**Example (Rust):**

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig, KreuzbergError};

fn extract_with_cache_handling(path: &str) -> kreuzberg::Result<String> {
    let config = ExtractionConfig {
        use_cache: true,
        ..Default::default()
    };

    // Cache errors are logged but don't prevent extraction
    match extract_file_sync(path, None, &config) {
        Ok(result) => Ok(result.content),
        Err(KreuzbergError::Cache { message, .. }) => {
            // Cache error - logged internally, extraction continues
            eprintln!("Cache warning (non-fatal): {}", message);
            Err(KreuzbergError::Cache { message, source: None })
        }
        Err(e) => Err(e),
    }
}
```

**Example (Python):**

```python
from kreuzberg import extract_file, ExtractionConfig

# Cache errors are silently handled internally
config = ExtractionConfig(use_cache=True)
result = extract_file("document.pdf", config=config)
# Extraction succeeds even if cache fails
```

---

### KreuzbergError::ImageProcessing

**When Raised:** Image manipulation failures, format conversion errors

**Context:** Includes image format, operation type, and dimensions

**Common Causes:**

- Unsupported image format
- Invalid image data
- Image too large for processing
- Color space conversion failure
- DPI adjustment failure

**Example (Rust):**

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig, ImageExtractionConfig, KreuzbergError};

fn extract_images(path: &str) -> kreuzberg::Result<String> {
    let config = ExtractionConfig {
        images: Some(ImageExtractionConfig {
            extract_images: true,
            max_image_dimension: 4096,
            ..Default::default()
        }),
        ..Default::default()
    };

    match extract_file_sync(path, None, &config) {
        Ok(result) => Ok(result.content),
        Err(KreuzbergError::ImageProcessing { message, .. }) => {
            eprintln!("Image processing failed: {}", message);
            // Try without image extraction
            let config_no_images = ExtractionConfig::default();
            extract_file_sync(path, None, &config_no_images)
        }
        Err(e) => Err(e),
    }
}
```

**Example (Python):**

```python
from kreuzberg import extract_file, ExtractionConfig, ImageExtractionConfig, ImageProcessingError

config = ExtractionConfig(images=ImageExtractionConfig(extract_images=True))

try:
    result = extract_file("document.pdf", config=config)
except ImageProcessingError as e:
    print(f"Image processing failed: {e}")
    # Fall back to extraction without images
    result = extract_file("document.pdf")
```

---

### KreuzbergError::Serialization

**When Raised:** JSON/MessagePack serialization/deserialization failures

**Source:** `serde_json`, `rmp_serde` errors

**Common Causes:**

- Invalid JSON structure
- MessagePack decoding failure
- Type mismatch during deserialization
- Unsupported data types

**Example (Rust):**

```rust
use kreuzberg::{ExtractionConfig, KreuzbergError};

fn load_config_from_json(json_str: &str) -> kreuzberg::Result<ExtractionConfig> {
    match serde_json::from_str::<ExtractionConfig>(json_str) {
        Ok(config) => Ok(config),
        Err(e) => Err(KreuzbergError::serialization(format!(
            "Failed to parse config: {}",
            e
        ))),
    }
}
```

**Example (Python):**

```python
from kreuzberg import ExtractionConfig
import json

try:
    config_dict = json.loads('{"invalid": json}')
    config = ExtractionConfig(**config_dict)
except (json.JSONDecodeError, TypeError) as e:
    print(f"Configuration parsing failed: {e}")
```

---

### KreuzbergError::MissingDependency

**When Raised:** Required system dependencies not found

**Context:** Includes dependency name and installation instructions

**Common Causes:**

- Tesseract not installed (for OCR)
- Pandoc not installed (for DOCX, EPUB, LaTeX)
- LibreOffice not installed (for .doc, .ppt)

**Example (Rust):**

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig, OcrConfig, KreuzbergError};

fn extract_with_dependency_check(path: &str) -> kreuzberg::Result<String> {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig::default()),
        ..Default::default()
    };

    match extract_file_sync(path, None, &config) {
        Ok(result) => Ok(result.content),
        Err(KreuzbergError::MissingDependency(dep)) => {
            eprintln!("Missing dependency: {}", dep);
            eprintln!("Install with: brew install tesseract (macOS)");
            Err(KreuzbergError::MissingDependency(dep))
        }
        Err(e) => Err(e),
    }
}
```

**Example (Python):**

```python
from kreuzberg import extract_file, ExtractionConfig, OcrConfig, MissingDependencyError

config = ExtractionConfig(ocr=OcrConfig())

try:
    result = extract_file("scanned.pdf", config=config)
except MissingDependencyError as e:
    print(f"Missing system dependency: {e}")
    print("Install with: brew install tesseract (macOS)")
```

**Example (Ruby):**

```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  ocr: Kreuzberg::Config::OCR.new
)

begin
  result = Kreuzberg.extract_file_sync('scanned.pdf', config: config)
  puts result.content
rescue Kreuzberg::Errors::MissingDependencyError => e
  puts "Missing system dependency: #{e.message}"
  puts "Dependency: #{e.dependency}" if e.dependency
  puts "Install with: brew install tesseract (macOS)"
rescue Kreuzberg::Errors::Error => e
  puts "Extraction error: #{e.message}"
end
```

**Example (Go):**

```go
import (
	"errors"
	"fmt"
	"kreuzberg"
)

config := &kreuzberg.ExtractionConfig{
	OCR: &kreuzberg.OcrConfig{},
}

result, err := kreuzberg.ExtractFileSync("scanned.pdf", config)
if err != nil {
	var depErr *kreuzberg.MissingDependencyError
	if errors.As(err, &depErr) {
		fmt.Printf("Missing system dependency: %v\n", err)
		fmt.Printf("Dependency: %s\n", depErr.Dependency)
		fmt.Println("Install with: brew install tesseract (macOS)")
		return
	}
	fmt.Printf("Extraction error: %v\n", err)
	return
}
fmt.Println(result.Content)
```

---

### KreuzbergError::Plugin

**When Raised:** Plugin-specific errors, plugin registration failures

**Context:** Includes plugin name and error details

**Common Causes:**

- Plugin initialization failure
- Duplicate plugin registration
- Invalid plugin implementation
- Plugin execution error

**Example (Rust):**

```rust
use kreuzberg::plugins::registry::get_document_extractor_registry;
use kreuzberg::KreuzbergError;
use std::sync::Arc;

fn register_custom_plugin() -> kreuzberg::Result<()> {
    let registry = get_document_extractor_registry();

    // Attempt plugin registration
    match registry.write() {
        Ok(mut reg) => {
            // Registration logic here
            Ok(())
        }
        Err(e) => Err(KreuzbergError::Plugin {
            message: format!("Failed to register plugin: {}", e),
            plugin_name: "custom-extractor".to_string(),
        }),
    }
}
```

**Example (Python):**

```python
from kreuzberg import get_document_extractor_registry, DocumentExtractor

class CustomExtractor(DocumentExtractor):
    def name(self) -> str:
        return "custom-extractor"

    def supported_mime_types(self) -> list[str]:
        return ["application/x-custom"]

registry = get_document_extractor_registry()

try:
    registry.register(CustomExtractor())
except Exception as e:
    print(f"Plugin registration failed: {e}")
```

---

### KreuzbergError::LockPoisoned

**When Raised:** Mutex/RwLock poisoning (should not occur in normal operation)

**Context:** Includes lock name or resource description

**Common Causes:**

- Thread panic while holding lock
- Internal library bug
- Concurrent access violation

**Handling:** This indicates a serious internal error. Report to library maintainers.

**Example (Rust):**

```rust
use kreuzberg::{KreuzbergError};

fn access_registry() -> kreuzberg::Result<()> {
    // Lock poisoning is handled internally and converted to KreuzbergError
    // Users should report these errors as bugs
    Err(KreuzbergError::LockPoisoned(
        "Registry lock poisoned - this is a bug, please report".to_string()
    ))
}
```

---

### KreuzbergError::UnsupportedFormat

**When Raised:** Unsupported MIME type or file format

**Context:** Includes MIME type and list of supported formats

**Common Causes:**

- File extension not recognized
- MIME type not registered
- Feature flag not enabled for format
- Truly unsupported format

**Example (Rust):**

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig, KreuzbergError};

fn extract_with_format_check(path: &str) -> kreuzberg::Result<String> {
    let config = ExtractionConfig::default();

    match extract_file_sync(path, None, &config) {
        Ok(result) => Ok(result.content),
        Err(KreuzbergError::UnsupportedFormat(mime)) => {
            eprintln!("Unsupported format: {}", mime);
            eprintln!("Check if required feature flag is enabled");
            Err(KreuzbergError::UnsupportedFormat(mime))
        }
        Err(e) => Err(e),
    }
}
```

**Example (Python):**

```python
from kreuzberg import extract_file, UnsupportedFormatError

try:
    result = extract_file("video.mp4")  # Video not supported
except UnsupportedFormatError as e:
    print(f"Format not supported: {e}")
    print("Kreuzberg supports documents, images, and archives - not video files")
```

---

### KreuzbergError::Other

**When Raised:** Uncommon errors that don't fit other categories

**Context:** Includes descriptive error message

**Common Causes:**

- Unexpected edge cases
- Third-party library errors
- Runtime environment issues

**Example (Rust):**

```rust
use kreuzberg::KreuzbergError;

fn handle_unexpected_error() -> kreuzberg::Result<()> {
    Err(KreuzbergError::Other(
        "An unexpected error occurred".to_string()
    ))
}
```

## Error Handling Best Practices

### 1. Don't Suppress System Errors

```rust
// L Bad - wraps IO error
match std::fs::read("file.txt") {
    Ok(data) => process(data),
    Err(e) => Err(KreuzbergError::parsing(format!("Failed: {}", e))),
}

//  Good - lets IO error bubble up
let data = std::fs::read("file.txt")?;
process(data)
```

### 2. Add Context to Application Errors

```rust
// L Bad - no context
return Err(KreuzbergError::parsing("parse failed"));

//  Good - includes context
return Err(KreuzbergError::parsing(format!(
    "Failed to parse {} at line {}: {}",
    file_type, line_num, reason
)));
```

### 3. Use Error Chains

```rust
//  Preserve error chain
match parse_document(bytes) {
    Ok(doc) => Ok(doc),
    Err(e) => Err(KreuzbergError::parsing_with_source(
        format!("Document parsing failed for {}", mime_type),
        e
    )),
}
```

### 4. Handle Errors Appropriately by Type

```rust
match extract_file_sync(path, None, &config) {
    Ok(result) => Ok(result),
    Err(KreuzbergError::Io(e)) => {
        // System error - must be reported
        Err(KreuzbergError::Io(e))
    }
    Err(KreuzbergError::Cache { .. }) => {
        // Non-fatal - can be ignored
        Ok(fallback_result)
    }
    Err(KreuzbergError::MissingDependency(dep)) => {
        // User-actionable - provide installation instructions
        eprintln!("Please install {}", dep);
        Err(KreuzbergError::MissingDependency(dep))
    }
    Err(e) => Err(e),
}
```

## Python Error Mapping

Kreuzberg errors are mapped to appropriate Python exceptions:

| Rust Error | Python Exception |
|-----------|------------------|
| `KreuzbergError::Io` | `OSError` |
| `KreuzbergError::Parsing` | `ParsingError` (inherits from `KreuzbergError`) |
| `KreuzbergError::Ocr` | `OCRError` (inherits from `KreuzbergError`) |
| `KreuzbergError::Validation` | `ValidationError` (inherits from `KreuzbergError`) |
| `KreuzbergError::Cache` | `CacheError` (inherits from `KreuzbergError`) |
| `KreuzbergError::ImageProcessing` | `ImageProcessingError` (inherits from `KreuzbergError`) |
| `KreuzbergError::Serialization` | `SerializationError` (inherits from `KreuzbergError`) |
| `KreuzbergError::MissingDependency` | `MissingDependencyError` (inherits from `KreuzbergError`) |
| `KreuzbergError::Plugin` | `PluginError` (inherits from `KreuzbergError`) |
| `KreuzbergError::LockPoisoned` | `RuntimeError` |
| `KreuzbergError::UnsupportedFormat` | `UnsupportedFormatError` (inherits from `KreuzbergError`) |
| `KreuzbergError::Other` | `KreuzbergError` |

All Python exceptions inherit from the base `KreuzbergError` class and include a `context` parameter with debugging information.

## TypeScript Error Mapping

TypeScript errors are thrown as standard JavaScript `Error` objects with appropriate names:

```typescript
try {
  const result = await extractFile('document.pdf');
} catch (error) {
  if (error.name === 'ParsingError') {
    console.error('Document is corrupt:', error.message);
  } else if (error.name === 'MissingDependencyError') {
    console.error('System dependency missing:', error.message);
  } else {
    console.error('Unexpected error:', error);
  }
}
```

## See Also

- [Configuration Reference](configuration.md) - Configuration options that affect error handling
- [Format Support](formats.md) - Supported formats and their limitations
- [OCR Guide](../guides/ocr.md) - OCR-specific error handling
