# Rust Examples

This page provides comprehensive examples of using the Kreuzberg Rust core library. All example code is available in the [`examples/rust/`](https://github.com/Goldziher/kreuzberg/tree/main/examples/rust) directory.

The Kreuzberg Rust core is a standalone library that can be used directly in Rust projects for high-performance document extraction.

## Installation

Add Kreuzberg to your `Cargo.toml`:

```toml
[dependencies]
kreuzberg = "4.0"
tokio = { version = "1", features = ["rt", "macros"] }
async-trait = "0.1"
```

## Basic Extraction

The `basic.rs` example demonstrates fundamental extraction patterns including synchronous and asynchronous extraction, working with bytes, accessing metadata, and batch processing.

### Simple Synchronous Extraction

```rust
--8<-- "examples/rust/basic.rs:10:18"
```

### Extraction with Configuration

```rust
--8<-- "examples/rust/basic.rs:19:28"
```

### Async Extraction

```rust
--8<-- "examples/rust/basic.rs:29:33"
```

### Extract from Bytes

```rust
--8<-- "examples/rust/basic.rs:34:39"
```

### Extract from Bytes (Async)

```rust
--8<-- "examples/rust/basic.rs:40:45"
```

### Accessing Metadata

```rust
--8<-- "examples/rust/basic.rs:46:58"
```

### OCR Extraction

```rust
--8<-- "examples/rust/basic.rs:59:71"
```

### Batch Processing

```rust
--8<-- "examples/rust/basic.rs:72:79"
```

### Async Batch Processing

```rust
--8<-- "examples/rust/basic.rs:80:85"
```

### Content Chunking

```rust
--8<-- "examples/rust/basic.rs:86:102"
```

### Error Handling

```rust
--8<-- "examples/rust/basic.rs:103:109"
```

## Custom Document Extractors

The `custom_extractor.rs` example demonstrates how to implement custom document extractors for file formats not supported by the built-in extractors.

### Implementing a CSV Extractor

```rust
--8<-- "examples/rust/custom_extractor.rs:16:69"
```

### Async Implementation

```rust
--8<-- "examples/rust/custom_extractor.rs:71:183"
```

### Binary Format Extractor

```rust
--8<-- "examples/rust/custom_extractor.rs:185:248"
```

### Registering and Using Custom Extractors

```rust
--8<-- "examples/rust/custom_extractor.rs:250:295"
```

## Configuration Options

### ExtractionConfig

The `ExtractionConfig` struct controls extraction behavior:

```rust
use kreuzberg::{ExtractionConfig, OcrConfig, ChunkingConfig};

let config = ExtractionConfig {
    // Quality processing
    enable_quality_processing: true,

    // Caching
    use_cache: true,

    // OCR configuration
    ocr: Some(OcrConfig {
        backend: "tesseract".to_string(),
        language: "eng".to_string(),
        ..Default::default()
    }),

    // Force OCR even for text-based PDFs
    force_ocr: false,

    // Chunking for large documents
    chunking: Some(ChunkingConfig {
        max_chars: 1000,
        max_overlap: 100,
    }),

    ..Default::default()
};
```

### OcrConfig

Configure OCR behavior:

```rust
use kreuzberg::OcrConfig;

let ocr_config = OcrConfig {
    backend: "tesseract".to_string(),  // OCR backend to use
    language: "eng".to_string(),       // Language code
    ..Default::default()
};
```

### ChunkingConfig

Configure content chunking for large documents:

```rust
use kreuzberg::ChunkingConfig;

let chunking_config = ChunkingConfig {
    max_chars: 1000,   // Maximum characters per chunk
    max_overlap: 100,  // Overlap between chunks
};
```

## Working with Results

### ExtractionResult

The `ExtractionResult` struct contains all extraction information:

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig};

let result = extract_file_sync("document.pdf", None, &ExtractionConfig::default())?;

// Extracted text content
println!("Content: {}", result.content);

// MIME type
println!("MIME type: {}", result.mime_type);

// Metadata (varies by document type)
if let Some(pdf_metadata) = &result.metadata.pdf {
    println!("Pages: {}", pdf_metadata.page_count);
    if let Some(author) = &pdf_metadata.author {
        println!("Author: {}", author);
    }
    if let Some(title) = &pdf_metadata.title {
        println!("Title: {}", title);
    }
}

// Extracted tables
for table in &result.tables {
    println!("Table markdown:\n{}", table.markdown);
}

// Detected languages (if language detection enabled)
if let Some(languages) = &result.detected_languages {
    println!("Languages: {:?}", languages);
}

// Chunks (if chunking enabled)
if let Some(chunks) = &result.chunks {
    println!("Chunk count: {}", chunks.len());
    for (i, chunk) in chunks.iter().enumerate() {
        println!("Chunk {}: {} chars", i + 1, chunk.len());
    }
}
```

## Error Handling

All errors are variants of `KreuzbergError`:

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig, KreuzbergError};

match extract_file_sync("document.pdf", None, &ExtractionConfig::default()) {
    Ok(result) => {
        println!("Extracted {} characters", result.content.len());
    }
    Err(KreuzbergError::Validation { message, .. }) => {
        eprintln!("Validation error: {}", message);
    }
    Err(KreuzbergError::Parsing { message, .. }) => {
        eprintln!("Parsing error: {}", message);
    }
    Err(KreuzbergError::Ocr { message, .. }) => {
        eprintln!("OCR error: {}", message);
    }
    Err(KreuzbergError::MissingDependency { message, .. }) => {
        eprintln!("Missing dependency: {}", message);
    }
    Err(KreuzbergError::Io(e)) => {
        eprintln!("I/O error: {}", e);
    }
    Err(e) => {
        eprintln!("Extraction error: {}", e);
    }
}
```

## Advanced Topics

### Plugin System

Kreuzberg uses a trait-based plugin system. You can implement custom plugins for:

- **DocumentExtractor** - Custom file format extractors
- **OcrBackend** - Custom OCR engines
- **PostProcessor** - Data transformation and enrichment
- **Validator** - Fail-fast validation

#### DocumentExtractor Trait

```rust
use async_trait::async_trait;
use kreuzberg::plugins::extractor::DocumentExtractor;
use kreuzberg::{ExtractionConfig, ExtractionResult, KreuzbergError};

struct MyExtractor;

#[async_trait]
impl DocumentExtractor for MyExtractor {
    fn name(&self) -> &str {
        "my_extractor"
    }

    fn supported_mime_types(&self) -> Vec<String> {
        vec!["application/x-custom".to_string()]
    }

    fn priority(&self) -> i32 {
        100  // Higher priority than built-in extractors
    }

    async fn extract(
        &self,
        data: &[u8],
        _mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult, KreuzbergError> {
        // Implementation here
        todo!()
    }

    fn extract_sync(
        &self,
        data: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult, KreuzbergError> {
        // Synchronous implementation
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.extract(data, mime_type, config))
    }
}
```

#### Registering Custom Plugins

```rust
use kreuzberg::plugins::registry::get_document_extractor_registry;
use std::sync::Arc;

let registry = get_document_extractor_registry();
let mut registry = registry.write().unwrap();

let extractor = Arc::new(MyExtractor) as Arc<dyn DocumentExtractor>;
registry.register(extractor)?;
```

### Performance Tips

1. **Use release builds** - `cargo build --release` for 10-50x performance improvements
2. **Use batch processing** for multiple files with `batch_extract_files`
3. **Enable caching** for repeated extractions with `use_cache: true`
4. **Use async APIs** for I/O-bound workloads to maximize concurrency
5. **Configure OCR DPI** appropriately (300 DPI is usually sufficient)
6. **Use quality processing** only when needed (adds overhead)
7. **Streaming parsers** - XML and text parsers stream for memory efficiency

### Memory Management

Kreuzberg is designed for efficient memory usage:

- **Streaming parsers** for XML and text files (handles multi-GB files)
- **Zero-copy** operations where possible using `&str` and `&[u8]`
- **Arc for shared data** - Efficient shared ownership without cloning
- **RAII patterns** - Automatic cleanup with Drop trait

### Async/Await Patterns

```rust
use kreuzberg::{extract_file, ExtractionConfig};
use tokio;

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    // Async extraction
    let result = extract_file("document.pdf", None, &ExtractionConfig::default()).await?;
    println!("Content: {}", result.content);

    // Concurrent extractions
    let files = vec!["doc1.pdf", "doc2.pdf", "doc3.pdf"];
    let futures: Vec<_> = files.iter()
        .map(|&file| extract_file(file, None, &ExtractionConfig::default()))
        .collect();

    let results = futures::future::try_join_all(futures).await?;
    println!("Extracted {} files", results.len());

    Ok(())
}
```

### Thread Safety

All Kreuzberg types are thread-safe:

- **Plugin registries** use `RwLock` for concurrent access
- **All plugins** must be `Send + Sync`
- **Arc<dyn Trait>** pattern for shared plugin ownership
- **No global mutable state**

### Testing

Example test using Tokio:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use kreuzberg::{extract_file_sync, ExtractionConfig};

    #[test]
    fn test_basic_extraction() {
        let result = extract_file_sync(
            "tests/fixtures/sample.pdf",
            None,
            &ExtractionConfig::default()
        ).expect("Extraction failed");

        assert!(!result.content.is_empty());
        assert_eq!(result.mime_type, "application/pdf");
    }

    #[tokio::test]
    async fn test_async_extraction() {
        let result = extract_file(
            "tests/fixtures/sample.pdf",
            None,
            &ExtractionConfig::default()
        ).await.expect("Extraction failed");

        assert!(!result.content.is_empty());
    }
}
```

## Cargo Features

Kreuzberg supports feature flags for optional dependencies:

```toml
[dependencies]
kreuzberg = { version = "4.0", features = ["full"] }

# Or specific features
kreuzberg = { version = "4.0", features = ["pdf", "ocr", "excel"] }
```

Available features:
- `pdf` - PDF extraction support
- `ocr` - OCR support with Tesseract
- `excel` - Excel/spreadsheet extraction
- `email` - Email extraction
- `html` - HTML to markdown conversion
- `full` - All features

## Integration Examples

### Web Server (Actix-web)

```rust
use actix_web::{web, App, HttpResponse, HttpServer};
use kreuzberg::{extract_bytes, ExtractionConfig};

async fn extract_endpoint(bytes: web::Bytes) -> HttpResponse {
    match extract_bytes(
        &bytes,
        "application/pdf",
        &ExtractionConfig::default()
    ).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/extract", web::post().to(extract_endpoint))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

### Command-Line Tool

```rust
use clap::Parser;
use kreuzberg::{extract_file_sync, ExtractionConfig};

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    file: String,

    #[arg(short, long, default_value = "false")]
    ocr: bool,
}

fn main() -> kreuzberg::Result<()> {
    let args = Args::parse();

    let config = ExtractionConfig {
        ocr: if args.ocr {
            Some(Default::default())
        } else {
            None
        },
        ..Default::default()
    };

    let result = extract_file_sync(&args.file, None, &config)?;
    println!("{}", result.content);

    Ok(())
}
```

## Next Steps

- **[Python Examples](python.md)** - Examples for Python
- **[TypeScript Examples](typescript.md)** - Examples for Node.js/TypeScript
- **[Rust API Documentation](https://docs.rs/kreuzberg)** - Complete Rust API documentation
- **[Plugin Development](../plugins/rust-extractor.md)** - Deep dive into Rust plugin development
