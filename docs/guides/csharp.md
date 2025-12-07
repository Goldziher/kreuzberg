# C# Bindings for Kreuzberg

High-performance document intelligence for .NET applications. Extract text, metadata, and structured information from PDFs, Office documents, images, and 50+ formats.

**Powered by a Rust core** – Native performance for document extraction with P/Invoke interoperability.

> **Version 4.0.0 Release Candidate**
> This is a pre-release version. We invite you to test the library and [report any issues](https://github.com/kreuzberg-dev/kreuzberg/issues) you encounter.

## Installation

Install the Kreuzberg NuGet package:

```bash
dotnet add package Kreuzberg
```

### System Requirements

- **.NET 6.0 or higher**
- **Windows, macOS, or Linux**

### Optional System Dependencies

**Tesseract OCR** (Required for OCR functionality):
```bash
# macOS
brew install tesseract

# Ubuntu/Debian
sudo apt-get install tesseract-ocr

# Windows
# Download from https://github.com/tesseract-ocr/tesseract/wiki/Downloads
```

**LibreOffice** (Optional, for legacy Office formats .doc, .ppt):
```bash
# macOS
brew install libreoffice

# Ubuntu/Debian
sudo apt-get install libreoffice
```


## Quick Start

### Simple File Extraction

```csharp
using Kreuzberg;

var result = KreuzbergClient.ExtractFileSync("document.pdf");
Console.WriteLine(result.Content);
```

### Extract with Configuration

```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true
};

var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
Console.WriteLine($"Content: {result.Content}");
Console.WriteLine($"MIME Type: {result.MimeType}");
Console.WriteLine($"Success: {result.Success}");
```

### Async Extraction

```csharp
using Kreuzberg;

var result = await KreuzbergClient.ExtractFileAsync("document.pdf");
Console.WriteLine(result.Content);
```

### Batch Processing

```csharp
using Kreuzberg;

var files = new[] { "doc1.pdf", "doc2.docx", "doc3.xlsx" };
var results = KreuzbergClient.BatchExtractFilesSync(files);

foreach (var result in results)
{
    Console.WriteLine($"{result.MimeType}: {result.Content.Length} characters");
}
```

## Configuration

### Basic Configuration

```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true,
    ForceOcr = false
};

var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
```

### OCR Configuration

```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        Language = "eng",
        TesseractConfig = new TesseractConfig
        {
            Psm = 6,
            EnableTableDetection = true,
            MinConfidence = 50.0
        }
    }
};

var result = KreuzbergClient.ExtractFileSync("scanned.pdf", config);
```

### Table Extraction with Tesseract

```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        TesseractConfig = new TesseractConfig
        {
            EnableTableDetection = true
        }
    }
};

var result = KreuzbergClient.ExtractFileSync("invoice.pdf", config);

foreach (var table in result.Tables)
{
    Console.WriteLine(table.Markdown);
    Console.WriteLine($"Page: {table.PageNumber}");
}
```

### Complete Configuration Example

```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true,
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        Language = "eng",
        TesseractConfig = new TesseractConfig
        {
            Psm = 6,
            EnableTableDetection = true,
            MinConfidence = 50.0
        }
    },
    ForceOcr = false,
    Chunking = new ChunkingConfig
    {
        MaxChars = 1000,
        MaxOverlap = 200
    },
    Images = new ImageExtractionConfig
    {
        ExtractImages = true,
        TargetDpi = 300,
        MaxImageDimension = 4096,
        AutoAdjustDpi = true
    },
    PdfOptions = new PdfConfig
    {
        ExtractImages = true,
        Passwords = new List<string> { "password1", "password2" },
        ExtractMetadata = true
    },
    TokenReduction = new TokenReductionConfig
    {
        Mode = "moderate",
        PreserveImportantWords = true
    },
    LanguageDetection = new LanguageDetectionConfig
    {
        Enabled = true,
        MinConfidence = 0.8,
        DetectMultiple = false
    }
};

var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
```

### Loading Configuration from File

```csharp
using Kreuzberg;

// Loads kreuzberg.toml/yaml/json by walking parent directories
var discoveredConfig = KreuzbergClient.DiscoverExtractionConfig();

// Load from specific file path
var config = KreuzbergClient.LoadExtractionConfigFromFile("config/extraction.toml");
var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
```

## Extract from Bytes

```csharp
using Kreuzberg;

var data = File.ReadAllBytes("document.pdf");
var result = KreuzbergClient.ExtractBytesSync(data, "application/pdf");
Console.WriteLine(result.Content);
```

### Batch Extract from Bytes

```csharp
using Kreuzberg;

var items = new[]
{
    new BytesWithMime(File.ReadAllBytes("doc1.pdf"), "application/pdf"),
    new BytesWithMime(File.ReadAllBytes("doc2.docx"), "application/vnd.openxmlformats-officedocument.wordprocessingml.document")
};

var results = KreuzbergClient.BatchExtractBytesSync(items);
```

## MIME Type Detection

### Detect from File

```csharp
using Kreuzberg;

var mimeType = KreuzbergClient.DetectMimeTypeFromPath("document.pdf");
Console.WriteLine(mimeType); // "application/pdf"

// Get extensions for MIME type
var extensions = KreuzbergClient.GetExtensionsForMime("application/pdf");
Console.WriteLine(string.Join(", ", extensions)); // ".pdf"
```

### Detect from Bytes

```csharp
using Kreuzberg;

var data = File.ReadAllBytes("document");
var mimeType = KreuzbergClient.DetectMimeType(data);
Console.WriteLine(mimeType);
```

## Metadata Extraction

```csharp
using Kreuzberg;

var result = KreuzbergClient.ExtractFileSync("document.pdf");

// Language detection
if (result.Metadata.Language != null)
{
    Console.WriteLine($"Language: {result.Metadata.Language}");
}

// PDF metadata
if (result.Metadata.Format?.Pdf != null)
{
    var pdf = result.Metadata.Format.Pdf;
    Console.WriteLine($"Title: {pdf.Title}");
    Console.WriteLine($"Author: {pdf.Author}");
    Console.WriteLine($"Pages: {pdf.PageCount}");
    Console.WriteLine($"Created: {pdf.CreationDate}");
}

// Format type
Console.WriteLine($"Format: {result.Metadata.FormatType}");
```

## Password-Protected PDFs

```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    PdfOptions = new PdfConfig
    {
        Passwords = new List<string> { "password1", "password2", "password3" }
    }
};

var result = KreuzbergClient.ExtractFileSync("protected.pdf", config);
```

## Language Detection

```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    LanguageDetection = new LanguageDetectionConfig
    {
        Enabled = true,
        MinConfidence = 0.8
    }
};

var result = KreuzbergClient.ExtractFileSync("multilingual.pdf", config);

if (result.DetectedLanguages != null)
{
    foreach (var lang in result.DetectedLanguages)
    {
        Console.WriteLine(lang);
    }
}
```

## Text Chunking

```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    Chunking = new ChunkingConfig
    {
        MaxChars = 1000,
        MaxOverlap = 200
    }
};

var result = KreuzbergClient.ExtractFileSync("long_document.pdf", config);

if (result.Chunks != null)
{
    foreach (var chunk in result.Chunks)
    {
        Console.WriteLine($"Chunk {chunk.Metadata.ChunkIndex + 1}/{chunk.Metadata.TotalChunks}");
        Console.WriteLine($"Tokens: {chunk.Metadata.TokenCount ?? 0}");
        Console.WriteLine(chunk.Content);
    }
}
```

## Image Extraction

```csharp
using Kreuzberg;

var result = KreuzbergClient.ExtractFileSync("document.pdf");

if (result.Images != null && result.Images.Count > 0)
{
    Console.WriteLine($"Extracted {result.Images.Count} images");

    foreach (var image in result.Images)
    {
        Console.WriteLine($"Format: {image.Format}, Page: {image.PageNumber}");
        File.WriteAllBytes($"image_{image.ImageIndex}.{image.Format.ToLower()}", image.Data);
    }
}
```

## Embedding Presets

### List Available Presets

```csharp
using Kreuzberg;

var presets = KreuzbergClient.ListEmbeddingPresets();
Console.WriteLine($"Available presets: {string.Join(", ", presets)}");
```

### Get Specific Preset

```csharp
using Kreuzberg;

var preset = KreuzbergClient.GetEmbeddingPreset("default");
if (preset != null)
{
    Console.WriteLine($"Preset: {preset.Name}");
    Console.WriteLine($"Chunk Size: {preset.ChunkSize}");
    Console.WriteLine($"Model: {preset.ModelName}");
    Console.WriteLine($"Dimensions: {preset.Dimensions}");
}
```

## Custom Post-Processors

```csharp
using Kreuzberg;

public class CustomProcessor : IPostProcessor
{
    public string Name => "custom_processor";
    public int Priority => 10;

    public ExtractionResult Process(ExtractionResult result)
    {
        // Modify result
        result.Content = result.Content.ToUpper();
        return result;
    }
}

// Register processor
KreuzbergClient.RegisterPostProcessor(new CustomProcessor());

// Use processor
var result = KreuzbergClient.ExtractFileSync("document.pdf");

// List processors
var processors = KreuzbergClient.ListPostProcessors();
Console.WriteLine($"Registered: {string.Join(", ", processors)}");

// Unregister
KreuzbergClient.UnregisterPostProcessor("custom_processor");
```

## Custom Validators

```csharp
using Kreuzberg;

public class CustomValidator : IValidator
{
    public string Name => "custom_validator";
    public int Priority => 10;

    public void Validate(ExtractionResult result)
    {
        if (string.IsNullOrWhiteSpace(result.Content))
        {
            throw new InvalidOperationException("Content cannot be empty");
        }
    }
}

// Register validator
KreuzbergClient.RegisterValidator(new CustomValidator());

// Validators run automatically during extraction
var result = KreuzbergClient.ExtractFileSync("document.pdf");
```

## Custom OCR Backends

```csharp
using Kreuzberg;

public class CustomOcrBackend : IOcrBackend
{
    public string Name => "custom_ocr";

    public string Process(ReadOnlySpan<byte> imageBytes, OcrConfig? config)
    {
        // Process image and return JSON result
        return "{}";
    }
}

// Register backend
KreuzbergClient.RegisterOcrBackend(new CustomOcrBackend());

// Use custom backend
var extractConfig = new ExtractionConfig
{
    Ocr = new OcrConfig { Backend = "custom_ocr" }
};
var result = KreuzbergClient.ExtractFileSync("document.pdf", extractConfig);
```

## Exception Handling

```csharp
using Kreuzberg;
using System;

try
{
    var result = KreuzbergClient.ExtractFileSync("document.pdf");
}
catch (KreuzbergValidationException ex)
{
    Console.WriteLine($"Configuration validation error: {ex.Message}");
}
catch (KreuzbergParsingException ex)
{
    Console.WriteLine($"Document parsing failed: {ex.Message}");
}
catch (KreuzbergOcrException ex)
{
    Console.WriteLine($"OCR processing failed: {ex.Message}");
}
catch (KreuzbergMissingDependencyException ex)
{
    Console.WriteLine($"Missing optional dependency: {ex.Message}");
}
catch (KreuzbergException ex)
{
    Console.WriteLine($"General Kreuzberg error: {ex.Message}");
}
```

### Exception Hierarchy

- **KreuzbergException** - Base exception for all Kreuzberg errors
  - **KreuzbergValidationException** - Invalid configuration or input
  - **KreuzbergParsingException** - Document parsing failure
  - **KreuzbergOcrException** - OCR processing failure
  - **KreuzbergMissingDependencyException** - Missing optional dependency
  - **KreuzbergSerializationException** - JSON serialization failure

## API Reference

### Extraction Methods

- `ExtractFileSync(string path, ExtractionConfig? config = null)` - Synchronous file extraction
- `ExtractFileAsync(string path, ExtractionConfig? config = null, CancellationToken cancellationToken = default)` - Asynchronous file extraction
- `ExtractBytesSync(ReadOnlySpan<byte> data, string mimeType, ExtractionConfig? config = null)` - Synchronous bytes extraction
- `ExtractBytesAsync(byte[] data, string mimeType, ExtractionConfig? config = null, CancellationToken cancellationToken = default)` - Asynchronous bytes extraction
- `BatchExtractFilesSync(IReadOnlyList<string> paths, ExtractionConfig? config = null)` - Batch file extraction
- `BatchExtractFilesAsync(IReadOnlyList<string> paths, ExtractionConfig? config = null, CancellationToken cancellationToken = default)` - Async batch file extraction
- `BatchExtractBytesSync(IReadOnlyList<BytesWithMime> items, ExtractionConfig? config = null)` - Batch bytes extraction
- `BatchExtractBytesAsync(IReadOnlyList<BytesWithMime> items, ExtractionConfig? config = null, CancellationToken cancellationToken = default)` - Async batch bytes extraction

### MIME Type Detection

- `DetectMimeType(ReadOnlySpan<byte> data)` - Detect MIME from bytes
- `DetectMimeTypeFromPath(string path)` - Detect MIME from file path
- `GetExtensionsForMime(string mimeType)` - Get file extensions for MIME type

### Configuration Discovery

- `DiscoverExtractionConfig()` - Discover config by walking parent directories for kreuzberg.toml/yaml/json
- `LoadExtractionConfigFromFile(string path)` - Load config from specific file

### Plugin Management

- `RegisterPostProcessor(IPostProcessor processor)` - Register custom post-processor
- `UnregisterPostProcessor(string name)` - Unregister post-processor
- `ClearPostProcessors()` - Clear all post-processors
- `ListPostProcessors()` - List registered post-processors
- `RegisterValidator(IValidator validator)` - Register custom validator
- `UnregisterValidator(string name)` - Unregister validator
- `ClearValidators()` - Clear all validators
- `ListValidators()` - List registered validators
- `RegisterOcrBackend(IOcrBackend backend)` - Register custom OCR backend
- `UnregisterOcrBackend(string name)` - Unregister OCR backend
- `ClearOcrBackends()` - Clear all OCR backends
- `ListOcrBackends()` - List registered OCR backends
- `ListDocumentExtractors()` - List document extractors
- `UnregisterDocumentExtractor(string name)` - Unregister extractor
- `ClearDocumentExtractors()` - Clear all extractors

### Embedding Presets

- `ListEmbeddingPresets()` - Get all available embedding presets
- `GetEmbeddingPreset(string name)` - Get specific preset by name

### Utility

- `GetVersion()` - Get native library version string

## Result Types

### ExtractionResult

```csharp
public sealed class ExtractionResult
{
    public string Content { get; set; }           // Extracted text content
    public string MimeType { get; set; }          // Detected MIME type
    public Metadata Metadata { get; set; }        // Document metadata
    public List<Table> Tables { get; set; }       // Extracted tables
    public List<string>? DetectedLanguages { get; set; }
    public List<Chunk>? Chunks { get; set; }      // Text chunks
    public List<ExtractedImage>? Images { get; set; }
    public bool Success { get; set; }             // Extraction success flag
}
```

### Metadata

Contains language, date, subject, and format-specific metadata (PDF, Excel, Email, PPTX, Archive, Image, XML, Text, HTML, OCR).

### Table

```csharp
public sealed class Table
{
    public List<List<string>> Cells { get; set; } // Table cell contents
    public string Markdown { get; set; }          // Markdown representation
    public int PageNumber { get; set; }           // Page number
}
```

### Chunk

```csharp
public sealed class Chunk
{
    public string Content { get; set; }                // Chunk text
    public float[]? Embedding { get; set; }           // Embedding vector
    public ChunkMetadata Metadata { get; set; }       // Chunk metadata
}
```

## Thread Safety

The Kreuzberg C# binding is **thread-safe** at the API level:

- **KreuzbergClient static methods** are safe to call from multiple threads concurrently
- **Configuration objects** (ExtractionConfig, OcrConfig, etc.) are thread-safe for reading
- **Post-Processors, Validators, OCR Backends** registrations use thread-safe collections
- **No synchronization needed** for concurrent extraction calls

Note: Individual ExtractionResult objects should not be modified after creation if accessed from multiple threads.

## P/Invoke Interoperability

The C# binding uses P/Invoke to call native Rust code:

### NativeMethods Pattern

- **NativeMethods** - Pinvoke declarations mapping to kreuzberg-ffi C library
- **InteropUtilities** - Helper functions for UTF-8 string marshaling
- **Serialization** - JSON serialization wrapper using System.Text.Json

### Memory Management

- **AllocUtf8** - Allocate UTF-8 string in native memory
- **FreeUtf8** - Free allocated UTF-8 strings
- **FreeString** - Free native strings from library
- **FreeResult** - Free ExtractionResult structures
- **FreeBatchResult** - Free batch result arrays

All memory allocation/deallocation is handled automatically by try/finally blocks.

### Error Handling

- **ErrorMapper** - Converts native error strings to C# exceptions
- **ThrowLastError** - Retrieves and throws last error from native library
- All FFI boundaries validate pointer returns (check for IntPtr.Zero)

## Examples

### Process Multiple Files with Error Handling

```csharp
using Kreuzberg;
using System;
using System.IO;

var files = Directory.GetFiles("documents", "*.pdf");

foreach (var file in files)
{
    try
    {
        Console.Write($"Processing {Path.GetFileName(file)}...");
        var result = KreuzbergClient.ExtractFileSync(file);

        if (result.Success)
        {
            Console.WriteLine($" OK ({result.Content.Length} chars)");
            File.WriteAllText($"{file}.txt", result.Content);
        }
        else
        {
            Console.WriteLine(" FAILED");
        }
    }
    catch (Exception ex)
    {
        Console.WriteLine($" ERROR: {ex.Message}");
    }
}
```

### Extract and Save Metadata

```csharp
using Kreuzberg;
using System.Text.Json;

var result = KreuzbergClient.ExtractFileSync("document.pdf");

var metadata = new
{
    language = result.Metadata.Language,
    format = result.Metadata.FormatType,
    tables = result.Tables.Count,
    images = result.Images?.Count ?? 0,
    success = result.Success
};

var json = JsonSerializer.Serialize(metadata, new JsonSerializerOptions { WriteIndented = true });
File.WriteAllText("metadata.json", json);
```

### Concurrent Batch Processing

```csharp
using Kreuzberg;
using System.Threading.Tasks;

var files = new[] { "doc1.pdf", "doc2.pdf", "doc3.pdf", "doc4.pdf" };

var tasks = files.Select(file => Task.Run(() =>
{
    try
    {
        return KreuzbergClient.ExtractFileSync(file);
    }
    catch (Exception ex)
    {
        Console.WriteLine($"Error extracting {file}: {ex.Message}");
        return null;
    }
})).ToList();

var results = await Task.WhenAll(tasks);

var successful = results.Count(r => r?.Success == true);
Console.WriteLine($"Successfully processed {successful}/{files.Length} files");
```

## Troubleshooting

### DLL Not Found

If you get "DLL not found" errors, ensure the native library is in your runtime directory:

```bash
# Check library path
echo $LD_LIBRARY_PATH     # Linux
echo $DYLD_LIBRARY_PATH   # macOS
```

The library should be located in `runtimes/{rid}/native/` in the NuGet package.

### P/Invoke Errors

If P/Invoke calls fail, verify:
1. Native library is properly installed
2. Architecture matches (x64, arm64)
3. Dependencies are available (Tesseract, LibreOffice if needed)

### OCR Not Working

Ensure Tesseract is installed and in PATH:

```bash
tesseract --version
```

### Memory Issues

For large documents, consider:
1. Enabling chunking to process in smaller pieces
2. Using batch extraction for memory efficiency
3. Calling GC.Collect() after processing large batches

## Complete Documentation

[https://kreuzberg.dev](https://kreuzberg.dev)

- [Installation Guide](https://kreuzberg.dev/getting-started/installation/)
- [User Guide](https://kreuzberg.dev/user-guide/)
- [API Reference](https://kreuzberg.dev/api-reference/)
- [Format Support](https://kreuzberg.dev/formats/)
- [OCR Backends](https://kreuzberg.dev/user-guide/ocr-backends/)

## License

MIT License - see the [LICENSE](https://github.com/kreuzberg-dev/kreuzberg/blob/main/LICENSE) file in the repository for details.
