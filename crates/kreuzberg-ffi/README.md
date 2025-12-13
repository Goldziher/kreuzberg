# kreuzberg-ffi

C Foreign Function Interface (FFI) bindings for the Kreuzberg document intelligence library.

## Overview

This crate provides a C-compatible API that bridges the high-performance Rust core of Kreuzberg with multiple programming languages and FFI systems. It is the foundation for language bindings in Java (Panama FFI), Go (cgo), C# (P/Invoke), and other languages with C interoperability.

The FFI exposes extraction functions, configuration management, plugin registration, and error handling through a stable C interface with thread-safe callbacks.

## Architecture

### FFI Bridge Layers

```
Language-Specific Bindings
    ↓
Kreuzberg FFI C Library (crates/kreuzberg-ffi) ← This crate
    ↓
Rust Core Library (crates/kreuzberg)
    ↓
Document Extraction Engines
```

### Binding Support

This FFI layer is consumed by:

- **Java** (packages/java): Using Java 25 Foreign Function & Memory API (Panama FFI)
- **Go** (packages/go): Using cgo wrapper bindings
- **C#** (packages/csharp): Using P/Invoke interop
- **Zig** and other C-compatible languages

### Key Components

- **Core Extraction** (`extract_file`, `extract_bytes`): Document text and data extraction
- **Batch Operations**: Parallel processing of multiple documents
- **MIME Detection**: File format identification
- **Configuration Management**: Loading and applying extraction settings
- **Plugin System**: OCR backend registration and callbacks
- **Error Handling**: Thread-local error message storage
- **Memory Management**: Safe pointer handling and FFI boundaries

## Installation

### Build from Source

```bash
cargo build --release -p kreuzberg-ffi
```

### Output Artifacts

After building, you will have:

- **Dynamic Library**: `target/release/libkreuzberg_ffi.{so,dylib,dll}`
  - For loading at runtime
  - Platform-specific extensions (`.so` Linux, `.dylib` macOS, `.dll` Windows)

- **Static Library**: `target/release/libkreuzberg_ffi.{a,lib}`
  - For static linking into applications
  - Platform-specific extensions (`.a` Unix, `.lib` Windows)

- **Header File**: Auto-generated via `cbindgen` during build

### Header Generation

The C header file is automatically generated during the build process via `cbindgen`:

```bash
cargo build --release -p kreuzberg-ffi
# Header is generated at build time based on #[no_mangle] functions
```

For manual header generation:

```bash
cargo build --features html,embeddings -p kreuzberg-ffi
```

### pkg-config File Generation

The build process automatically generates pkg-config files for library discovery:

```bash
cargo build --release -p kreuzberg-ffi
```

This creates two variants in `crates/kreuzberg-ffi/`:
- **kreuzberg-ffi.pc**: Development version (prefix points to repository)
- **kreuzberg-ffi-install.pc**: Installation version (prefix=/usr/local)

The development variant enables monorepo developers to use pkg-config:

```bash
export PKG_CONFIG_PATH="$PWD/crates/kreuzberg-ffi:$PKG_CONFIG_PATH"
pkg-config --cflags kreuzberg-ffi  # Returns -I/path/to/repo/crates/kreuzberg-ffi
pkg-config --libs kreuzberg-ffi    # Returns -L/path/to/repo/target/release -lkreuzberg_ffi
```

The installation variant is used in release artifacts for third-party use.

### Installing from Release Artifacts

Pre-built binaries are available for Linux, macOS, and Windows (MinGW) from the [releases page](https://github.com/kreuzberg-dev/kreuzberg/releases).

Each `go-ffi-{platform}.tar.gz` archive contains:
- `lib/`: Shared libraries (kreuzberg-ffi, pdfium, onnxruntime)
- `include/`: C header file (kreuzberg.h)
- `share/pkgconfig/`: pkg-config file for library discovery
- `README.md`: Installation instructions

Installation:

```bash
# Download and extract
tar -xzf go-ffi-linux-x86_64.tar.gz
cd kreuzberg-ffi

# System-wide installation (requires sudo)
sudo cp -r lib/* /usr/local/lib/
sudo cp -r include/* /usr/local/include/
sudo cp -r share/* /usr/local/share/
sudo ldconfig  # Linux only

# Verify
pkg-config --modversion kreuzberg-ffi
```

For user-local installation or custom prefix, see the README.md included in the archive.

## Quick Start: C Example

### Basic Extraction

```c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Include the auto-generated Kreuzberg FFI header
#include "kreuzberg_ffi.h"

int main() {
    // Extract text from a PDF file
    const char* file_path = "document.pdf";
    const char* mime_type = NULL;  // Auto-detect
    const char* config_json = "{}";  // Empty config uses defaults

    // Perform extraction
    ExtractionResult result = kreuzberg_extract_file(
        file_path,
        mime_type,
        config_json
    );

    // Check for errors
    if (result.error != NULL) {
        fprintf(stderr, "Extraction failed: %s\n", result.error);
        kreuzberg_free_string(result.error);
        return 1;
    }

    // Process the result
    printf("Extracted content:\n%s\n", result.content);
    printf("MIME Type: %s\n", result.mime_type);

    // Free resources
    kreuzberg_free_extraction_result(result);

    return 0;
}
```

### Compilation

```bash
# Link against the dynamic library
gcc -o extract_example extract.c \
    -L/path/to/kreuzberg/target/release \
    -lkreuzberg_ffi
```

## API Reference

### Core Extraction Functions

All functions return results via out-parameters or result structs. Error messages are accessible via `kreuzberg_get_last_error()`.

#### `ExtractionResult kreuzberg_extract_file(const char *path, const char *mime_type, const char *config_json)`

Extract text and metadata from a file.

**Parameters:**
- `path`: Absolute or relative file path (must exist)
- `mime_type`: Optional MIME type hint (e.g., "application/pdf"). Pass NULL for auto-detection.
- `config_json`: JSON string with extraction configuration

**Returns:**
- `ExtractionResult`: Contains `content` (extracted text), `mime_type`, `metadata` (JSON), and optional `error`

**Example:**

```c
const char* config = "{\"use_cache\": true, \"enable_quality_processing\": true}";
ExtractionResult result = kreuzberg_extract_file("document.pdf", NULL, config);

if (result.error) {
    printf("Error: %s\n", result.error);
} else {
    printf("Content: %s\n", result.content);
    printf("MIME: %s\n", result.mime_type);
}

kreuzberg_free_extraction_result(result);
```

#### `ExtractionResult kreuzberg_extract_bytes(const char *data, size_t data_len, const char *mime_type, const char *config_json)`

Extract from a byte buffer (e.g., file in memory).

**Parameters:**
- `data`: Pointer to byte buffer
- `data_len`: Buffer length in bytes
- `mime_type`: Optional MIME type hint (required if auto-detection not desired)
- `config_json`: JSON extraction configuration

**Returns:**
- `ExtractionResult`: Same as `extract_file`

**Example:**

```c
// Read file into buffer
FILE* f = fopen("document.pdf", "rb");
fseek(f, 0, SEEK_END);
size_t size = ftell(f);
fseek(f, 0, SEEK_SET);
unsigned char* buffer = malloc(size);
fread(buffer, 1, size, f);
fclose(f);

// Extract from buffer
const char* config = "{}";
ExtractionResult result = kreuzberg_extract_bytes(
    (const char*)buffer,
    size,
    "application/pdf",
    config
);

if (!result.error) {
    printf("Content: %s\n", result.content);
}

kreuzberg_free_extraction_result(result);
free(buffer);
```

#### `BatchExtractionResult kreuzberg_batch_extract_files(const char **paths, size_t paths_count, const char *config_json)`

Process multiple files in parallel.

**Parameters:**
- `paths`: Array of file path strings
- `paths_count`: Number of paths in array
- `config_json`: JSON extraction configuration (applied to all files)

**Returns:**
- `BatchExtractionResult`: Contains array of `ExtractionResult` and error status

**Example:**

```c
const char* files[] = {"doc1.pdf", "doc2.docx", "doc3.xlsx"};
size_t count = 3;

BatchExtractionResult batch_result = kreuzberg_batch_extract_files(
    files,
    count,
    "{}"
);

if (batch_result.error) {
    printf("Batch error: %s\n", batch_result.error);
} else {
    for (size_t i = 0; i < batch_result.count; i++) {
        printf("File %zu: %s\n", i, batch_result.results[i].mime_type);
    }
}

kreuzberg_free_batch_extraction_result(batch_result);
```

#### `char* kreuzberg_detect_mime_type(const char *path, bool use_cache)`

Identify the MIME type of a file using magic bytes and file signatures.

**Parameters:**
- `path`: File path to analyze
- `use_cache`: Whether to use cached results for the same path

**Returns:**
- `char*`: MIME type string (must be freed with `kreuzberg_free_string`)

**Example:**

```c
char* mime = kreuzberg_detect_mime_type("unknown-file", true);
if (mime) {
    printf("MIME Type: %s\n", mime);
    kreuzberg_free_string(mime);
}
```

### Configuration Management

#### `char* kreuzberg_get_default_config_json()`

Get default extraction configuration as JSON.

**Returns:**
- `char*`: JSON string (must be freed with `kreuzberg_free_string`)

**Example:**

```c
char* default_config = kreuzberg_get_default_config_json();
printf("Default config: %s\n", default_config);
kreuzberg_free_string(default_config);
```

#### Configuration JSON Schema

```json
{
  "use_cache": true,
  "enable_quality_processing": false,
  "force_ocr": false,
  "ocr": {
    "backend": "tesseract",
    "language": "eng",
    "tesseract_config": {
      "enable_table_detection": true,
      "psm": 6,
      "min_confidence": 50.0
    }
  },
  "chunking": {
    "max_chars": 1000,
    "max_overlap": 200
  },
  "language_detection": {
    "enabled": false,
    "min_confidence": 0.8,
    "detect_multiple": false
  }
}
```

### Plugin System

#### `void kreuzberg_register_ocr_backend(const char *backend_name, OcrBackendCallback callback)`

Register a custom OCR backend implemented in the calling language.

**Parameters:**
- `backend_name`: Unique identifier for the backend (e.g., "custom-ocr")
- `callback`: Function pointer to OCR processing implementation

**Example (C):**

```c
// Define the callback function
OcrBackendResult custom_ocr_callback(
    const char* image_data,
    size_t image_size,
    const char* language,
    void* user_data
) {
    // Call out to Python, Go, C#, etc. to perform OCR
    // Implementation depends on the host language

    OcrBackendResult result = {0};
    result.content = "Extracted text from OCR";
    result.metadata = "{}";
    return result;
}

// Register the backend
kreuzberg_register_ocr_backend("easyocr", custom_ocr_callback);

// Now use it in extraction config:
const char* config = "{\"ocr\": {\"backend\": \"easyocr\", \"language\": \"eng\"}}";
ExtractionResult result = kreuzberg_extract_file("scanned.pdf", NULL, config);
```

### Error Handling

#### `char* kreuzberg_get_last_error()`

Retrieve the last error message.

**Returns:**
- `char*`: Error message string (NULL if no error)

Note: Error messages are thread-local and persist until the next Kreuzberg function call.

**Example:**

```c
ExtractionResult result = kreuzberg_extract_file("invalid.pdf", NULL, "{}");

if (result.error == NULL) {
    // Also check global error state
    char* error = kreuzberg_get_last_error();
    if (error) {
        printf("Error: %s\n", error);
    }
}
```

### Memory Management

#### `void kreuzberg_free_string(char *ptr)`

Free a string allocated by FFI functions.

**Note:** Only use for strings returned by FFI functions, not for caller-allocated strings.

**Example:**

```c
char* mime = kreuzberg_detect_mime_type("file.pdf", true);
kreuzberg_free_string(mime);

char* error = kreuzberg_get_last_error();
if (error) {
    kreuzberg_free_string(error);
}
```

#### `void kreuzberg_free_extraction_result(ExtractionResult result)`

Free memory associated with an extraction result.

**Example:**

```c
ExtractionResult result = kreuzberg_extract_file("doc.pdf", NULL, "{}");
// ... use result ...
kreuzberg_free_extraction_result(result);
```

#### `void kreuzberg_free_batch_extraction_result(BatchExtractionResult result)`

Free memory associated with a batch extraction result.

**Example:**

```c
BatchExtractionResult batch = kreuzberg_batch_extract_files(files, 3, "{}");
// ... use batch.results ...
kreuzberg_free_batch_extraction_result(batch);
```

## Type Definitions

### ExtractionResult

```c
typedef struct {
    char* content;           // Extracted text
    char* mime_type;         // MIME type of document
    char* metadata;          // JSON metadata string
    char* error;             // Error message (NULL if success)
    // ... tables, images, chunks as needed
} ExtractionResult;
```

### BatchExtractionResult

```c
typedef struct {
    ExtractionResult* results;  // Array of extraction results
    size_t count;               // Number of results
    char* error;                // Batch-level error (NULL if success)
} BatchExtractionResult;
```

### OcrBackendCallback

```c
typedef OcrBackendResult (*OcrBackendCallback)(
    const char* image_data,
    size_t image_size,
    const char* language,
    void* user_data
);
```

## FFI Safety Guidelines

### Thread Safety

All FFI functions are **thread-safe**. The Kreuzberg core uses Arc, Mutex, and RwLock for safe concurrent access:

```c
// Safe to call from multiple threads
for (int i = 0; i < 10; i++) {
    pthread_t thread;
    pthread_create(&thread, NULL, extract_worker, (void*)(intptr_t)i);
}
```

### Pointer Validation

The FFI layer validates all input pointers:

```c
// Safe: NULL pointers are handled gracefully
ExtractionResult result = kreuzberg_extract_file(NULL, NULL, "{}");
// Returns error in result.error

// Safe: Invalid paths return errors
ExtractionResult result = kreuzberg_extract_file("/nonexistent", NULL, "{}");
// Returns error in result.error
```

### Memory Lifetime

- **Returned strings** must be freed with `kreuzberg_free_string()`
- **Result structs** must be freed with appropriate cleanup functions
- **Input parameters** are copied internally; caller retains ownership

```c
// Correct: Free returned values
char* mime = kreuzberg_detect_mime_type("file.pdf", true);
kreuzberg_free_string(mime);

// Correct: Input paths can be freed immediately after call
{
    char path[256];
    snprintf(path, sizeof(path), "document.pdf");
    ExtractionResult result = kreuzberg_extract_file(path, NULL, "{}");
    // path can be freed here; FFI has already copied it
}

// Correct: Free result structures
ExtractionResult result = kreuzberg_extract_file("doc.pdf", NULL, "{}");
// ... use result ...
kreuzberg_free_extraction_result(result);
```

### Error Handling Pattern

```c
ExtractionResult result = kreuzberg_extract_file(path, NULL, config);

// Check return value first
if (result.error != NULL) {
    fprintf(stderr, "FFI Error: %s\n", result.error);
    kreuzberg_free_extraction_result(result);
    return;
}

// Process successful result
printf("Content: %s\n", result.content);
printf("MIME: %s\n", result.mime_type);

// Always cleanup
kreuzberg_free_extraction_result(result);
```

## Building from C

### Static Linking

```bash
# Build the library
cargo build --release -p kreuzberg-ffi

# Create your C program
gcc -c -o myapp.o myapp.c

# Link statically
gcc -o myapp myapp.o \
    -L/path/to/kreuzberg/target/release \
    -l:libkreuzberg_ffi.a \
    -pthread -ldl  # May need additional system libs

# Run
./myapp
```

### Dynamic Linking

```bash
# Build the library
cargo build --release -p kreuzberg-ffi

# Create your C program
gcc -c -o myapp.o myapp.c

# Link dynamically
gcc -o myapp myapp.o \
    -L/path/to/kreuzberg/target/release \
    -lkreuzberg_ffi

# Set library path and run
export LD_LIBRARY_PATH=/path/to/kreuzberg/target/release:$LD_LIBRARY_PATH
./myapp
```

### macOS Considerations

```bash
# Build
cargo build --release -p kreuzberg-ffi

# Link
gcc -o myapp myapp.o \
    -L/path/to/kreuzberg/target/release \
    -lkreuzberg_ffi

# Set runtime path
export DYLD_LIBRARY_PATH=/path/to/kreuzberg/target/release:$DYLD_LIBRARY_PATH
./myapp
```

## Language Binding Integration

### Java Integration (Panama FFI)

The FFI is wrapped in Java 25's Foreign Function & Memory API:

```java
// Java code that calls FFI
Arena arena = Arena.ofConfined();
MemorySegment path = arena.allocateUtf8String("document.pdf");
MemorySegment config = arena.allocateUtf8String("{}");

ExtractionResult result = KreuzbergFFI.extract_file(path, MemorySegment.NULL, config);
```

### Go Integration (cgo)

The FFI is exposed through cgo bindings:

```go
// Go code that calls FFI
C.kreuzberg_extract_file(C.CString("document.pdf"), nil, C.CString("{}"))
```

### C# Integration (P/Invoke)

The FFI is declared in C# through P/Invoke:

```csharp
[DllImport("kreuzberg_ffi", CharSet = CharSet.Ansi)]
private static extern IntPtr kreuzberg_extract_file(
    string path,
    string mimeType,
    string configJson
);
```

## Supported Features

### Default Features
- `html`: HTML to Markdown conversion support
- `embeddings`: Text embedding extraction via fastembed-rs (requires ONNX Runtime)

### Core Feature (Windows MinGW Compatibility)
- `core`: Minimal feature set for cross-platform compatibility
  - Includes: `html` (HTML to Markdown conversion)
  - Excludes: `embeddings` (ONNX Runtime not available on MinGW)
  - Use case: Windows Go bindings with MinGW toolchain

### Platform-Specific Build Requirements

**Windows MinGW (Go bindings):**

The Windows ONNX Runtime library only provides MSVC-compatible .lib files. MinGW cannot link against these, requiring the core feature:

```bash
# Windows MinGW - Use core feature
cargo build --release -p kreuzberg-ffi --target x86_64-pc-windows-gnu --no-default-features --features core

# Windows MSVC - Full features available
cargo build --release -p kreuzberg-ffi --target x86_64-pc-windows-msvc

# Unix (Linux/macOS) - Full features available
cargo build --release -p kreuzberg-ffi
```

**Why MinGW Requires core Feature:**
- ONNX Runtime distributes Windows binaries compiled with MSVC toolchain
- MSVC .lib files use different name mangling and linking conventions than MinGW
- MinGW's GNU toolchain cannot consume MSVC import libraries
- The `core` feature excludes the `embeddings` dependency, which depends on ort-sys (ONNX Runtime)
- HTML support (via html-to-markdown-rs) is pure Rust and works on all platforms

### Building with Features

```bash
# Build with HTML and embeddings support (default)
cargo build --release -p kreuzberg-ffi

# Build with core feature only (Windows MinGW compatibility)
cargo build --release -p kreuzberg-ffi --no-default-features --features core

# Build without any features (minimal FFI)
cargo build --release -p kreuzberg-ffi --no-default-features
```

## Performance Characteristics

- **Single extraction**: 10-100ms (varies by file size and format)
- **Batch processing**: Near-linear scaling with CPU cores
- **OCR processing**: 100-500ms per page
- **Memory overhead**: ~2-5MB per extraction operation
- **Thread safety**: Zero synchronization overhead on single-threaded extraction

## Key Files

- `src/lib.rs`: FFI function implementations and plugin system
- `Cargo.toml`: Dependencies and features
- Generated header: Auto-created by cbindgen during build

## Building

### Development Build

```bash
cargo build -p kreuzberg-ffi
```

### Release Build

```bash
cargo build --release -p kreuzberg-ffi
```

### With All Features

```bash
cargo build --release -p kreuzberg-ffi --features html,embeddings
```

## Testing

```bash
# Run FFI tests
cargo test -p kreuzberg-ffi

# With logging
RUST_LOG=debug cargo test -p kreuzberg-ffi -- --nocapture
```

## Troubleshooting

### Library Not Found

Ensure the built library is in the linker search path:

```bash
# Check for built libraries
ls -la target/release/libkreuzberg_ffi*

# Add to library path
export LD_LIBRARY_PATH=target/release:$LD_LIBRARY_PATH
export DYLD_LIBRARY_PATH=target/release:$DYLD_LIBRARY_PATH  # macOS
```

### Undefined Reference Errors

Ensure you're linking against the FFI library, not the core library:

```bash
# Correct
gcc -o app app.c -lkreuzberg_ffi

# Incorrect
gcc -o app app.c -lkreuzberg  # Wrong library
```

### Memory Leaks

Always free returned strings and result structures:

```c
// Problem: Memory leak
char* mime = kreuzberg_detect_mime_type("file.pdf", true);
printf("%s\n", mime);
// mime not freed!

// Solution: Free the string
char* mime = kreuzberg_detect_mime_type("file.pdf", true);
printf("%s\n", mime);
kreuzberg_free_string(mime);
```

### Thread-Local Error Messages

Each thread has its own error message storage. Check both return values and `kreuzberg_get_last_error()`:

```c
// Safe across threads
#pragma omp parallel for
for (int i = 0; i < 10; i++) {
    ExtractionResult result = kreuzberg_extract_file(files[i], NULL, "{}");
    if (result.error) {
        printf("Thread %d error: %s\n", i, result.error);
    }
}
```

## References

- **Kreuzberg Core**: `../kreuzberg/`
- **C FFI Standards**: https://en.cppreference.com/w/c
- **cbindgen Documentation**: https://rust-lang.github.io/cbindgen/
- **Project Homepage**: https://kreuzberg.dev
- **GitHub Repository**: https://github.com/kreuzberg-dev/kreuzberg

## Contributing

We welcome contributions! Please see the main Kreuzberg repository for contribution guidelines.

## License

MIT
