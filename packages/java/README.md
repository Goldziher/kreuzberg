# Kreuzberg for Java

High-performance document intelligence library for Java with native Rust bindings via FFM API.

## Features

- **PDF Extraction**: Extract text, tables, metadata, and images from PDF documents
- **Office Documents**: Support for DOCX, XLSX, PPTX, and other Office formats
- **OCR**: Built-in OCR support with multiple backends (Tesseract, PaddleOCR, EasyOCR)
- **Table Extraction**: Advanced table detection and extraction
- **Image Processing**: Extract and process embedded images
- **Metadata**: Comprehensive metadata extraction
- **Fast**: Native Rust implementation for maximum performance
- **Modern Java**: Uses Java 22+ Foreign Function & Memory API (no JNI)

## Requirements

- Java 22 or higher
- Native libraries are bundled with the package

## Installation

### Maven

```xml
<dependency>
    <groupId>dev.kreuzberg</groupId>
    <artifactId>kreuzberg</artifactId>
    <version>4.0.0-rc.1</version>
</dependency>
```

### Gradle

```gradle
implementation 'dev.kreuzberg:kreuzberg:4.0.0-rc.1'
```

## Usage

```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.config.KreuzbergConfig;
import dev.kreuzberg.model.ExtractionResult;

public class Example {
    public static void main(String[] args) {
        // Initialize with default configuration
        try (var kreuzberg = new Kreuzberg(KreuzbergConfig.builder().build())) {
            // Extract from a file
            ExtractionResult result = kreuzberg.extractFile("document.pdf");

            System.out.println(result.getContent());
            System.out.println(result.getTables());
            System.out.println(result.getMetadata());
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}
```

### With Custom Configuration

```java
import dev.kreuzberg.config.KreuzbergConfig;
import dev.kreuzberg.config.OcrConfig;

KreuzbergConfig config = KreuzbergConfig.builder()
    .withOcr(OcrConfig.builder()
        .backend("tesseract")
        .language("eng")
        .build())
    .build();

try (var kreuzberg = new Kreuzberg(config)) {
    ExtractionResult result = kreuzberg.extractFile("scanned.pdf");
    System.out.println(result.getContent());
}
```

## Documentation

For full documentation, visit [https://kreuzberg.dev](https://kreuzberg.dev)

## License

MIT
