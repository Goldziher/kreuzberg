# Kreuzberg

[![Rust](https://img.shields.io/crates/v/kreuzberg?label=Rust)](https://crates.io/crates/kreuzberg)
[![Python](https://img.shields.io/pypi/v/kreuzberg?label=Python)](https://pypi.org/project/kreuzberg/)
[![TypeScript](https://img.shields.io/npm/v/@kreuzberg/node?label=TypeScript)](https://www.npmjs.com/package/@kreuzberg/node)
[![WASM](https://img.shields.io/npm/v/@kreuzberg/wasm?label=WASM)](https://www.npmjs.com/package/@kreuzberg/wasm)
[![Ruby](https://img.shields.io/gem/v/kreuzberg?label=Ruby)](https://rubygems.org/gems/kreuzberg)
[![Java](https://img.shields.io/maven-central/v/dev.kreuzberg/kreuzberg?label=Java)](https://central.sonatype.com/artifact/dev.kreuzberg/kreuzberg)
[![Go](https://img.shields.io/github/v/tag/kreuzberg-dev/kreuzberg?label=Go)](https://pkg.go.dev/github.com/kreuzberg-dev/kreuzberg)
[![C#](https://img.shields.io/nuget/v/Goldziher.Kreuzberg?label=C%23)](https://www.nuget.org/packages/Goldziher.Kreuzberg/)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Documentation](https://img.shields.io/badge/docs-kreuzberg.dev-blue)](https://kreuzberg.dev/)
[![Discord](https://img.shields.io/badge/Discord-Join%20our%20community-7289da)](https://discord.gg/pXxagNK2zN)

High-performance document intelligence library for Java with native Rust bindings via FFM API.

## Features

- **PDF Extraction**: Extract text, tables, metadata, and images from PDF documents
- **Office Documents**: Support for DOCX, XLSX, PPTX, and other Office formats
- **OCR**: Built-in OCR support with multiple backends (Tesseract, PaddleOCR, EasyOCR)
- **Table Extraction**: Advanced table detection and extraction
- **Image Processing**: Extract and process embedded images
- **Metadata**: Comprehensive metadata extraction
- **Fast**: Native Rust implementation for maximum performance
- **Modern Java**: Uses Java 25+ Foreign Function & Memory API (no JNI)

## Requirements

- Java 25 or higher
- Native libraries are bundled with the package (Linux/macOS/Windows)

If you need to override native discovery (e.g., custom builds), set `KREUZBERG_FFI_DIR` to a directory containing the
native libraries (`libkreuzberg_ffi` and `libpdfium` for your platform).

## Installation

### Maven

```xml
<dependency>
    <groupId>dev.kreuzberg</groupId>
    <artifactId>kreuzberg</artifactId>
    <version>4.0.0-rc.7</version>
</dependency>
```

### Gradle

```gradle
implementation 'dev.kreuzberg:kreuzberg:4.0.0-rc.7'
```

## Usage

```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.KreuzbergException;
import java.io.IOException;

public class Example {
    public static void main(String[] args) {
        try {
            var result = Kreuzberg.extractFile("document.pdf");
            System.out.println(result.content());
            System.out.println(result.mimeType());
        } catch (IOException e) {
            e.printStackTrace();
        } catch (KreuzbergException e) {
            e.printStackTrace();
        }
    }
}
```

### With Custom Configuration

```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.KreuzbergException;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.OcrConfig;
import java.io.IOException;

ExtractionConfig config = ExtractionConfig.builder()
    .ocr(OcrConfig.builder()
        .backend("tesseract")
        .language("eng")
        .build())
    .build();

try {
    var result = Kreuzberg.extractFile(java.nio.file.Path.of("scanned.pdf"), config);
    System.out.println(result.content());
} catch (IOException e) {
    e.printStackTrace();
} catch (KreuzbergException e) {
    e.printStackTrace();
}
```

## Documentation

For full documentation, visit [https://kreuzberg.dev](https://kreuzberg.dev)

## License

MIT
