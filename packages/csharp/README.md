# Kreuzberg for C# / .NET

[![Crates.io](https://img.shields.io/crates/v/kreuzberg)](https://crates.io/crates/kreuzberg)
[![PyPI](https://img.shields.io/pypi/v/kreuzberg)](https://pypi.org/project/kreuzberg/)
[![npm](https://img.shields.io/npm/v/kreuzberg)](https://www.npmjs.com/package/kreuzberg)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Documentation](https://img.shields.io/badge/docs-kreuzberg.dev-blue)](https://kreuzberg.dev)

High-performance document intelligence for .NET. Extract text, metadata, and structured information from PDFs, Office documents, images, and 56 formats.

**Powered by a Rust core** – Native performance for document extraction with safe P/Invoke bindings.

> **Version 4.0.0 Release Candidate**
> This is a pre-release version. We invite you to test the library and [report any issues](https://github.com/kreuzberg-dev/kreuzberg/issues) you encounter.

## Features

- **56 File Formats**: PDF, DOCX, PPTX, XLSX, images, HTML, Markdown, XML, JSON, and more
- **OCR Support**: Built-in Tesseract OCR for scanned documents and images
- **High Performance**: Rust-powered extraction for native-level performance
- **Table Extraction**: Extract structured tables from documents
- **Language Detection**: Automatic language detection for extracted text
- **Text Chunking**: Split long documents into manageable chunks
- **Image Extraction**: Extract images embedded in documents
- **Metadata**: Comprehensive format-specific metadata extraction
- **Thread-Safe**: Concurrent extraction with safe P/Invoke patterns
- **Fully Async**: Task-based async API alongside sync methods

## Installation

```bash
dotnet add package Kreuzberg
```
Published under the `kreuzberg.dev` organization.

```xml
<ItemGroup>
	    <PackageReference Include="Kreuzberg" Version="4.0.0-rc.6" />
</ItemGroup>
```

## Requirements

- **.NET 6.0 or higher**
- **Windows, macOS, or Linux** (x64, ARM64)
- **Tesseract OCR** (optional, for OCR functionality)
- **LibreOffice** (optional, for legacy Office formats)

## Quick Start

### Simple File Extraction

```csharp
using Kreuzberg;

var result = KreuzbergClient.ExtractFileSync("document.pdf");
Console.WriteLine(result.Content);
```

### Async Extraction (Recommended)

```csharp
using Kreuzberg;

var result = await KreuzbergClient.ExtractFileAsync("document.pdf");
Console.WriteLine(result.Content);
```

### With Configuration

```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true
};

var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
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

## OCR Support

### Tesseract (Default)

```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        Language = "eng"
    }
};

var result = KreuzbergClient.ExtractFileSync("scanned.pdf", config);
```

### Table Extraction

```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        TesseractConfig = new TesseractConfig
        {
            EnableTableDetection = true,
            Psm = 6
        }
    }
};

var result = KreuzbergClient.ExtractFileSync("invoice.pdf", config);

foreach (var table in result.Tables)
{
    Console.WriteLine(table.Markdown);
}
```

## Configuration

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
    LanguageDetection = new LanguageDetectionConfig
    {
        Enabled = true,
        MinConfidence = 0.8
    }
};

var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
```

## Metadata Extraction

```csharp
using Kreuzberg;

var result = KreuzbergClient.ExtractFileSync("document.pdf");

// Language detection
Console.WriteLine($"Language: {result.Metadata.Language}");

// PDF metadata
if (result.Metadata.Format?.Pdf != null)
{
    Console.WriteLine($"Title: {result.Metadata.Format.Pdf.Title}");
    Console.WriteLine($"Author: {result.Metadata.Format.Pdf.Author}");
    Console.WriteLine($"Pages: {result.Metadata.Format.Pdf.PageCount}");
}
```

## Password-Protected PDFs

```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    PdfOptions = new PdfConfig
    {
        Passwords = new List<string> { "password1", "password2" }
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
        DetectMultiple = true
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
        Console.WriteLine($"Chunk {chunk.Metadata.ChunkIndex}/{chunk.Metadata.TotalChunks}");
        Console.WriteLine(chunk.Content);
    }
}
```

## Extract from Bytes

```csharp
using Kreuzberg;

var data = File.ReadAllBytes("document.pdf");
var result = KreuzbergClient.ExtractBytesSync(data, "application/pdf");
Console.WriteLine(result.Content);
```

## MIME Type Detection

```csharp
using Kreuzberg;

// Detect from file
var mimeType = KreuzbergClient.DetectMimeTypeFromPath("document.pdf");
Console.WriteLine(mimeType); // "application/pdf"

// Detect from bytes
var data = File.ReadAllBytes("document");
var mime = KreuzbergClient.DetectMimeType(data);

// Get extensions for MIME type
var extensions = KreuzbergClient.GetExtensionsForMime("application/pdf");
Console.WriteLine(string.Join(", ", extensions)); // ".pdf"
```

## Exception Handling

```csharp
using Kreuzberg;

try
{
    var result = KreuzbergClient.ExtractFileSync("document.pdf");
}
catch (KreuzbergValidationException ex)
{
    Console.WriteLine($"Validation error: {ex.Message}");
}
catch (KreuzbergParsingException ex)
{
    Console.WriteLine($"Parsing error: {ex.Message}");
}
catch (KreuzbergOcrException ex)
{
    Console.WriteLine($"OCR error: {ex.Message}");
}
catch (KreuzbergMissingDependencyException ex)
{
    Console.WriteLine($"Missing dependency: {ex.Message}");
}
catch (KreuzbergException ex)
{
    Console.WriteLine($"Kreuzberg error: {ex.Message}");
}
```

## API Overview

### Extraction Methods
- `ExtractFileSync()` - Extract from file synchronously
- `ExtractFileAsync()` - Extract from file asynchronously
- `ExtractBytesSync()` - Extract from bytes synchronously
- `ExtractBytesAsync()` - Extract from bytes asynchronously
- `BatchExtractFilesSync()` - Batch extract multiple files
- `BatchExtractFilesAsync()` - Async batch extract
- `BatchExtractBytesSync()` - Batch extract from bytes
- `BatchExtractBytesAsync()` - Async batch extract from bytes

### MIME Type Detection
- `DetectMimeType()` - Detect MIME from bytes
- `DetectMimeTypeFromPath()` - Detect MIME from file path
- `GetExtensionsForMime()` - Get extensions for MIME type

### Configuration
- `DiscoverExtractionConfig()` - Auto-discover config file
- `LoadExtractionConfigFromFile()` - Load config from file

### Plugin System
- `RegisterPostProcessor()` / `UnregisterPostProcessor()` - Custom post-processors
- `RegisterValidator()` / `UnregisterValidator()` - Custom validators
- `RegisterOcrBackend()` / `UnregisterOcrBackend()` - Custom OCR backends

### Utility
- `GetVersion()` - Get native library version
- `ListEmbeddingPresets()` - List embedding presets
- `GetEmbeddingPreset()` - Get specific embedding preset

## System Dependencies

### Tesseract OCR (Optional)

**macOS:**
```bash
brew install tesseract
```

**Ubuntu/Debian:**
```bash
sudo apt-get install tesseract-ocr
```

**Windows:**
Download from [Tesseract GitHub](https://github.com/tesseract-ocr/tesseract/wiki/Downloads)

### LibreOffice (Optional, for legacy Office formats)

**macOS:**
```bash
brew install libreoffice
```

**Ubuntu/Debian:**
```bash
sudo apt-get install libreoffice
```

## Build & Test

### Build

```bash
dotnet build
```

### Run Tests

```bash
dotnet test
```

### Build Documentation

```bash
dotnet build --configuration Release
```

## Thread Safety

All KreuzbergClient static methods are thread-safe and can be called concurrently from multiple threads. The binding uses safe P/Invoke patterns with proper memory management.

## Examples

### Process Files in Directory

```csharp
using Kreuzberg;

var files = Directory.GetFiles("documents", "*.pdf");

foreach (var file in files)
{
    try
    {
        var result = KreuzbergClient.ExtractFileSync(file);
        if (result.Success)
        {
            File.WriteAllText($"{file}.txt", result.Content);
        }
    }
    catch (Exception ex)
    {
        Console.WriteLine($"Error processing {file}: {ex.Message}");
    }
}
```

### Concurrent Processing with Task Parallel Library

```csharp
using Kreuzberg;
using System.Threading.Tasks;

var files = new[] { "doc1.pdf", "doc2.pdf", "doc3.pdf" };

var results = await Task.WhenAll(files.Select(file =>
    KreuzbergClient.ExtractFileAsync(file)
));

foreach (var result in results)
{
    Console.WriteLine($"Processed: {result.Content.Length} characters");
}
```

### Custom Post-Processor

```csharp
using Kreuzberg;

public class UppercaseProcessor : IPostProcessor
{
    public string Name => "uppercase";
    public int Priority => 10;

    public ExtractionResult Process(ExtractionResult result)
    {
        result.Content = result.Content.ToUpper();
        return result;
    }
}

KreuzbergClient.RegisterPostProcessor(new UppercaseProcessor());
var result = KreuzbergClient.ExtractFileSync("document.pdf");
KreuzbergClient.UnregisterPostProcessor("uppercase");
```

## Contributing

We welcome contributions! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

For more details, see the main [CONTRIBUTING.md](../../CONTRIBUTING.md).

## Troubleshooting

### DLL Not Found

Ensure the native library is properly installed in your runtime directory:

```bash
# Check library paths
echo $LD_LIBRARY_PATH     # Linux
echo $DYLD_LIBRARY_PATH   # macOS
```

### OCR Not Working

Verify Tesseract is installed and in PATH:

```bash
tesseract --version
```

### P/Invoke Errors

Check that:
1. Native library is installed
2. Architecture matches (x64, ARM64)
3. Dependencies are available if needed

## Complete Documentation

[https://kreuzberg.dev](https://kreuzberg.dev)

- [Installation Guide](https://kreuzberg.dev/getting-started/installation/)
- [User Guide](https://kreuzberg.dev/guides/extraction/)
- [API Reference](https://kreuzberg.dev/reference/api-csharp/)
- [Format Support](https://kreuzberg.dev/reference/formats/)
- [OCR Backends](https://kreuzberg.dev/guides/ocr/)

## License

MIT License - see [LICENSE](../../LICENSE) for details.
