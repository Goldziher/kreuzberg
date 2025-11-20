# Extraction Basics

```mermaid
flowchart LR
    Input[Input] --> FileOrBytes{File or Bytes?}
    FileOrBytes -->|File Path| FileFunctions[File Functions]
    FileOrBytes -->|In-Memory Data| BytesFunctions[Bytes Functions]

    FileFunctions --> SingleOrBatch{Single or Batch?}
    BytesFunctions --> SingleOrBatch

    SingleOrBatch -->|Single| SingleMode[Single Extraction]
    SingleOrBatch -->|Multiple| BatchMode[Batch Processing]

    SingleMode --> SyncAsync{Sync or Async?}
    BatchMode --> SyncAsync

    SyncAsync -->|Sync| SyncFuncs[extract_file_sync<br/>extract_bytes_sync<br/>batch_extract_files_sync<br/>batch_extract_bytes_sync]
    SyncAsync -->|Async| AsyncFuncs[extract_file<br/>extract_bytes<br/>batch_extract_files<br/>batch_extract_bytes]

    style FileFunctions fill:#87CEEB
    style BytesFunctions fill:#FFD700
    style SyncFuncs fill:#90EE90
    style AsyncFuncs fill:#FFB6C1
```

Kreuzberg provides 8 core extraction functions organized into 4 categories: file extraction, bytes extraction, batch file extraction, and batch bytes extraction. Each has both sync and async variants.

## Extract from Files

Extract text, tables, and metadata from a file on disk.

### Synchronous

=== "Python"

    --8<-- "snippets/python/extract_file_sync.md"

=== "TypeScript"

    --8<-- "snippets/typescript/extract_file_sync.md"

=== "Rust"

    --8<-- "snippets/rust/extract_file_sync.md"

=== "Ruby"

    --8<-- "snippets/ruby/extract_file_sync.md"

=== "Java"

    --8<-- "snippets/java/extract_file_sync.md"

=== "Go"

    --8<-- "snippets/go/extract_file_sync.md"

=== "C#"

    --8<-- "snippets/csharp/extract_file_sync.md"

### Asynchronous

=== "Python"

    --8<-- "snippets/python/extract_file_async.md"

=== "TypeScript"

    --8<-- "snippets/typescript/extract_file_async.md"

=== "Rust"

    --8<-- "snippets/rust/extract_file_async.md"

=== "Ruby"

    --8<-- "snippets/ruby/extract_file_async.md"

=== "Java"

    --8<-- "snippets/java/extract_file_async.md"

=== "Go"

    --8<-- "snippets/go/extract_file_async.md"

=== "C#"

    --8<-- "snippets/csharp/extract_file_async.md"

```mermaid
flowchart TD
    Start[Choose Sync or Async] --> Context{Already in<br/>Async Context?}

    Context -->|Yes| UseAsync[Use Async Variants]
    Context -->|No| CheckUse{Use Case}

    CheckUse -->|Simple Script| UseSyncSimple[Use Sync<br/>Simpler, same speed]
    CheckUse -->|Multiple Files| CheckConcurrency{Concurrent<br/>Processing?}
    CheckUse -->|Single File| UseSyncSingle[Use Sync<br/>Easier to read]

    CheckConcurrency -->|Yes| UseAsyncConcurrent[Use Async<br/>Better concurrency]
    CheckConcurrency -->|No| UseSyncBatch[Use Sync<br/>Adequate]

    UseAsync --> AsyncFunctions[extract_file<br/>extract_bytes<br/>batch_extract_files<br/>batch_extract_bytes]
    UseSyncSimple --> SyncFunctions[extract_file_sync<br/>extract_bytes_sync<br/>batch_extract_files_sync<br/>batch_extract_bytes_sync]
    UseSyncSingle --> SyncFunctions
    UseAsyncConcurrent --> AsyncFunctions
    UseSyncBatch --> SyncFunctions

    style UseAsync fill:#FFB6C1
    style UseAsyncConcurrent fill:#FFB6C1
    style UseSyncSimple fill:#90EE90
    style UseSyncSingle fill:#90EE90
    style UseSyncBatch fill:#90EE90
```

## TypeScript / Node.js {#typescript-nodejs}

All TypeScript/Node.js examples in this guide use the `@goldziher/kreuzberg` package. Import synchronous APIs from the root module and asynchronous helpers from the same namespace. See the [TypeScript API Reference](../reference/api-typescript.md) for complete type definitions.

```typescript
import { extractFileSync, ExtractionConfig } from '@goldziher/kreuzberg';

const result = extractFileSync('document.pdf', null, new ExtractionConfig());
console.log(result.content);
```

## Ruby {#ruby}

Ruby bindings mirror the same function names (`extract_file_sync`, `extract_bytes`, `batch_extract_files`, etc.) under the `Kreuzberg` module. Configuration objects live under `Kreuzberg::Config`. See the [Ruby API Reference](../reference/api-ruby.md) for details.

```ruby
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(force_ocr: true)
result = Kreuzberg.extract_file_sync('document.pdf', config: config)
puts result.content
```

!!! tip "When to Use Async"
    Use async variants when you're already in an async context or processing multiple files concurrently. For simple scripts, sync variants are simpler and just as fast.

## Extract from Bytes

Extract from data already loaded in memory.

### Synchronous

=== "Python"

    --8<-- "snippets/python/extract_bytes_sync.md"

=== "TypeScript"

    --8<-- "snippets/typescript/extract_bytes_sync.md"

=== "Rust"

    --8<-- "snippets/rust/extract_bytes_sync.md"

=== "Ruby"

    --8<-- "snippets/ruby/extract_bytes_sync.md"

=== "Java"

    --8<-- "snippets/java/extract_bytes_sync.md"

=== "Go"

    --8<-- "snippets/go/extract_bytes_sync.md"

=== "C#"

    --8<-- "snippets/csharp/extract_bytes_sync.md"

### Asynchronous

=== "Python"

    --8<-- "snippets/python/extract_bytes_async.md"

=== "TypeScript"

    --8<-- "snippets/typescript/extract_bytes_async.md"

=== "Rust"

    --8<-- "snippets/rust/extract_bytes_async.md"

=== "Ruby"

    --8<-- "snippets/ruby/extract_bytes_async.md"

=== "Java"

    --8<-- "snippets/java/extract_bytes_async.md"

=== "Go"

    --8<-- "snippets/go/extract_bytes_async.md"

=== "C#"

    --8<-- "snippets/csharp/extract_bytes_async.md"

!!! note "MIME Type Detection"
    Kreuzberg automatically detects MIME types from file extensions. When extracting from bytes, you must provide the MIME type explicitly.

## Batch Processing

Process multiple files concurrently for better performance.

### Batch Extract Files

=== "Python"

    --8<-- "snippets/python/batch_extract_files_sync.md"

=== "TypeScript"

    --8<-- "snippets/typescript/batch_extract_files_sync.md"

=== "Rust"

    --8<-- "snippets/rust/batch_extract_files_sync.md"

=== "Ruby"

    --8<-- "snippets/ruby/batch_extract_files_sync.md"

=== "Java"

    --8<-- "snippets/java/batch_extract_files_sync.md"

=== "Go"

    --8<-- "snippets/go/batch_extract_files_sync.md"

=== "C#"

    --8<-- "snippets/csharp/batch_extract_files_sync.md"

### Batch Extract Bytes

=== "Python"

    --8<-- "snippets/python/batch_extract_bytes_sync.md"

=== "TypeScript"

    --8<-- "snippets/typescript/batch_extract_bytes_sync.md"

=== "Rust"

    --8<-- "snippets/rust/batch_extract_bytes_sync.md"

=== "Ruby"

    --8<-- "snippets/ruby/batch_extract_bytes_sync.md"

=== "Java"

    --8<-- "snippets/java/batch_extract_bytes_sync.md"

=== "Go"

    --8<-- "snippets/go/batch_extract_bytes_sync.md"

=== "C#"

    --8<-- "snippets/csharp/batch_extract_bytes_sync.md"

```mermaid
flowchart TD
    Start[Multiple Files to Process] --> Method{Processing Method}

    Method -->|Sequential| Sequential[Process One by One]
    Method -->|Batch| Batch[Batch Processing]

    Sequential --> Seq1[File 1: 1.0s]
    Seq1 --> Seq2[File 2: 1.0s]
    Seq2 --> Seq3[File 3: 1.0s]
    Seq3 --> Seq4[File 4: 1.0s]
    Seq4 --> SeqTotal[Total: 4.0s]

    Batch --> Parallel[Automatic Parallelization]
    Parallel --> Par1[File 1: 1.0s]
    Parallel --> Par2[File 2: 1.0s]
    Parallel --> Par3[File 3: 1.0s]
    Parallel --> Par4[File 4: 1.0s]

    Par1 --> ParTotal[Total: ~1.2s]
    Par2 --> ParTotal
    Par3 --> ParTotal
    Par4 --> ParTotal

    SeqTotal --> Result[Sequential: Slow]
    ParTotal --> ResultFast[Batch: 2-5x Faster]

    style Sequential fill:#FFB6C1
    style Batch fill:#90EE90
    style SeqTotal fill:#FF6B6B
    style ResultFast fill:#4CAF50
```

!!! tip "Performance"
    Batch processing provides automatic parallelization. For large sets of files, this can be 2-5x faster than processing files sequentially.

## Supported Formats

Kreuzberg supports 118+ file formats across 8 categories:

| Format | Extensions | Notes |
|--------|-----------|-------|
| **PDF** | `.pdf` | Native text + OCR for scanned pages |
| **Images** | `.png`, `.jpg`, `.jpeg`, `.tiff`, `.bmp`, `.webp` | Requires OCR backend |
| **Office** | `.docx`, `.pptx`, `.xlsx` | Modern formats via native parsers |
| **Legacy Office** | `.doc`, `.ppt` | Requires LibreOffice |
| **Email** | `.eml`, `.msg` | Full support including attachments |
| **Web** | `.html`, `.htm` | Converted to Markdown with metadata |
| **Text** | `.md`, `.txt`, `.xml`, `.json`, `.yaml`, `.toml`, `.csv` | Direct extraction |
| **Archives** | `.zip`, `.tar`, `.tar.gz`, `.tar.bz2` | Recursive extraction |

See the [installation guide](../getting-started/installation.md#system-dependencies) for optional dependencies (Tesseract, LibreOffice, Pandoc).

## Error Handling

All extraction functions raise exceptions on failure:

=== "Python"

    --8<-- "snippets/python/error_handling.md"

=== "TypeScript"

    --8<-- "snippets/typescript/error_handling.md"

=== "Rust"

    --8<-- "snippets/rust/error_handling.md"

=== "Ruby"

    --8<-- "snippets/ruby/error_handling.md"

=== "Java"

    --8<-- "snippets/java/error_handling.md"

=== "Go"

    --8<-- "snippets/go/error_handling.md"

=== "C#"

    --8<-- "snippets/csharp/error_handling.md"

!!! warning "System Errors"
    `OSError` (Python), `IOException` (Rust), and system-level errors always bubble up to users. These indicate real system problems that need to be addressed (permissions, disk space, etc.).

## Next Steps

- [Configuration](configuration.md) - Learn about all configuration options
- [OCR Guide](ocr.md) - Set up optical character recognition
- [Advanced Features](advanced.md) - Chunking, language detection, and more
- [API Reference](../reference/api-python.md) - Detailed API documentation
