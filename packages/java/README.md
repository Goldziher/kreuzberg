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
