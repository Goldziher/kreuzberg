# Java API Reference

Complete reference for the Kreuzberg Java bindings using Java 22+ Foreign Function & Memory API (FFM/Panama).

## Installation

Add the dependency to your Maven `pom.xml`:

```xml
<dependency>
    <groupId>dev.kreuzberg</groupId>
    <artifactId>kreuzberg</artifactId>
    <version>4.0.0-rc.1</version>
</dependency>
```

Or with Gradle:

```gradle
dependencies {
    implementation 'dev.kreuzberg:kreuzberg:4.0.0-rc.1'
}
```

**Requirements:**
- Java 22 or later
- libkreuzberg_ffi native library (auto-loaded)
- Optional: Tesseract or EasyOCR/PaddleOCR for OCR functionality

---

## Core Functions

### extractFile()

Extract content from a file (synchronous).

**Signature:**

```java
public static ExtractionResult extractFile(String path) throws IOException, KreuzbergException
public static ExtractionResult extractFile(Path path) throws IOException, KreuzbergException
public static ExtractionResult extractFile(Path path, ExtractionConfig config) throws IOException, KreuzbergException
```

**Parameters:**

- `path` (String | Path): Path to the file to extract
- `config` (ExtractionConfig): Optional extraction configuration. Uses defaults if null

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

**Throws:**

- `IOException`: If file not found or not readable
- `KreuzbergException`: Base exception for all extraction errors (subclasses: `ParsingException`, `OcrException`, `MissingDependencyException`)

**Example - Basic usage:**

```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;

try {
    ExtractionResult result = Kreuzberg.extractFile("document.pdf");
    System.out.println(result.getContent());
    System.out.println("MIME Type: " + result.getMimeType());
} catch (IOException e) {
    System.err.println("File error: " + e.getMessage());
} catch (KreuzbergException e) {
    System.err.println("Extraction failed: " + e.getMessage());
}
```

**Example - With OCR:**

```java
import dev.kreuzberg.*;
import dev.kreuzberg.config.*;

ExtractionConfig config = ExtractionConfig.builder()
    .ocr(OcrConfig.builder()
        .backend("tesseract")
        .language("eng")
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("scanned.pdf", config);
System.out.println(result.getContent());
```

**Example - With multiple options:**

```java
ExtractionConfig config = ExtractionConfig.builder()
    .useCache(true)
    .forceOcr(false)
    .enableQualityProcessing(true)
    .ocr(OcrConfig.builder()
        .backend("tesseract")
        .language("eng+fra")
        .build())
    .pdfOptions(PdfConfig.builder()
        .extractImages(true)
        .extractMetadata(true)
        .build())
    .chunking(ChunkingConfig.builder()
        .maxChars(1000)
        .maxOverlap(200)
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);
```

---

### extractBytes()

Extract content from byte array (synchronous).

**Signature:**

```java
public static ExtractionResult extractBytes(byte[] data, String mimeType, ExtractionConfig config)
    throws KreuzbergException
```

**Parameters:**

- `data` (byte[]): File content as bytes (must not be empty)
- `mimeType` (String): MIME type of the data (required for format detection)
- `config` (ExtractionConfig): Optional extraction configuration

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

**Throws:**

- `KreuzbergException`: If extraction or validation fails

**Example - Basic usage:**

```java
import dev.kreuzberg.Kreuzberg;

byte[] pdfBytes = /* read from file or stream */;
ExtractionResult result = Kreuzberg.extractBytes(pdfBytes, "application/pdf", null);
System.out.println(result.getContent());
```

**Example - With configuration:**

```java
ExtractionConfig config = ExtractionConfig.builder()
    .enableQualityProcessing(true)
    .build();

byte[] docxBytes = /* ... */;
ExtractionResult result = Kreuzberg.extractBytes(docxBytes, "application/vnd.openxmlformats-officedocument.wordprocessingml.document", config);
```

---

### batchExtractFiles()

Extract content from multiple files in parallel (synchronous).

**Signature:**

```java
public static List<ExtractionResult> batchExtractFiles(List<String> paths, ExtractionConfig config)
    throws KreuzbergException
```

**Parameters:**

- `paths` (List<String>): List of file paths to extract
- `config` (ExtractionConfig): Optional extraction configuration applied to all files

**Returns:**

- `List<ExtractionResult>`: List of extraction results (one per file)

**Throws:**

- `KreuzbergException`: If batch extraction fails

**Example:**

```java
import dev.kreuzberg.Kreuzberg;
import java.util.List;

List<String> filePaths = List.of(
    "doc1.pdf",
    "doc2.docx",
    "doc3.xlsx"
);

List<ExtractionResult> results = Kreuzberg.batchExtractFiles(filePaths, null);

for (int i = 0; i < filePaths.size(); i++) {
    System.out.println(filePaths.get(i) + ": " + results.get(i).getContent().length() + " characters");
}
```

---

### batchExtractBytes()

Extract content from multiple byte arrays in parallel (synchronous).

**Signature:**

```java
public static List<ExtractionResult> batchExtractBytes(List<BytesWithMime> items, ExtractionConfig config)
    throws KreuzbergException
```

**Parameters:**

- `items` (List<BytesWithMime>): List of byte data with MIME types
- `config` (ExtractionConfig): Optional extraction configuration

**Returns:**

- `List<ExtractionResult>`: List of extraction results

**Throws:**

- `KreuzbergException`: If batch extraction fails

**Example:**

```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.BytesWithMime;
import java.util.List;

List<BytesWithMime> items = List.of(
    new BytesWithMime(pdfBytes, "application/pdf"),
    new BytesWithMime(docxBytes, "application/vnd.openxmlformats-officedocument.wordprocessingml.document")
);

List<ExtractionResult> results = Kreuzberg.batchExtractBytes(items, null);
```

---

### extractFileAsync()

Extract content from a file (asynchronous).

**Signature:**

```java
public static CompletableFuture<ExtractionResult> extractFileAsync(Path path, ExtractionConfig config)
```

**Parameters:**

- `path` (Path): File path to extract
- `config` (ExtractionConfig): Optional extraction configuration

**Returns:**

- `CompletableFuture<ExtractionResult>`: Future that completes with the extraction result

**Example:**

```java
Kreuzberg.extractFileAsync(Path.of("document.pdf"), null)
    .thenAccept(result -> System.out.println(result.getContent()))
    .exceptionally(e -> {
        System.err.println("Error: " + e.getMessage());
        return null;
    });
```

---

### extractBytesAsync()

Extract content from bytes (asynchronous).

**Signature:**

```java
public static CompletableFuture<ExtractionResult> extractBytesAsync(
    byte[] data,
    String mimeType,
    ExtractionConfig config
)
```

**Returns:**

- `CompletableFuture<ExtractionResult>`: Future that completes with the extraction result

---

### batchExtractFilesAsync()

Extract multiple files in parallel (asynchronous).

**Signature:**

```java
public static CompletableFuture<List<ExtractionResult>> batchExtractFilesAsync(
    List<String> paths,
    ExtractionConfig config
)
```

**Returns:**

- `CompletableFuture<List<ExtractionResult>>`: Future that completes with extraction results

---

### batchExtractBytesAsync()

Extract multiple byte arrays in parallel (asynchronous).

**Signature:**

```java
public static CompletableFuture<List<ExtractionResult>> batchExtractBytesAsync(
    List<BytesWithMime> items,
    ExtractionConfig config
)
```

**Returns:**

- `CompletableFuture<List<ExtractionResult>>`: Future that completes with extraction results

---

## Configuration

### ExtractionConfig

Main extraction configuration using builder pattern.

**Builder Methods:**

```java
ExtractionConfig config = ExtractionConfig.builder()
    .useCache(true)                                    // Enable caching (default: true)
    .enableQualityProcessing(false)                    // Enable quality processing (default: false)
    .forceOcr(false)                                   // Force OCR on all pages (default: false)
    .ocr(OcrConfig)                                    // OCR configuration
    .chunking(ChunkingConfig)                          // Text chunking configuration
    .languageDetection(LanguageDetectionConfig)        // Language detection settings
    .pdfOptions(PdfConfig)                             // PDF-specific options
    .imageExtraction(ImageExtractionConfig)            // Image extraction settings
    .imagePreprocessing(ImagePreprocessingConfig)      // Image preprocessing
    .postprocessor(PostProcessorConfig)                // Post-processor settings
    .tokenReduction(TokenReductionConfig)              // Token reduction configuration
    .htmlOptions(HtmlOptions)                          // HTML conversion options
    .keywords(KeywordConfig)                           // Keyword extraction settings
    .maxConcurrentExtractions(4)                       // Max concurrent extractions
    .build();
```

**Static Methods:**

```java
// Load from file (TOML, YAML, or JSON)
ExtractionConfig config = ExtractionConfig.fromFile("kreuzberg.toml");

// Discover from current/parent directories
ExtractionConfig config = ExtractionConfig.discover(); // Returns null if not found
```

---

### OcrConfig

OCR configuration for text extraction from images.

**Builder Methods:**

```java
OcrConfig ocr = OcrConfig.builder()
    .backend("tesseract")          // "tesseract", "easyocr", "paddleocr", etc.
    .language("eng")               // Language code(s), comma-separated for multiple
    .tesseractConfig(config)       // Tesseract-specific configuration
    .build();
```

**Example - Multi-language OCR:**

```java
OcrConfig ocr = OcrConfig.builder()
    .backend("tesseract")
    .language("eng+fra+deu")       // English, French, and German
    .build();
```

---

### OcrBackend Interface

Custom OCR backend implementation for cloud-based or specialized OCR.

**Interface:**

```java
public interface OcrBackend {
    /**
     * Process image and extract text.
     *
     * @param imageData raw image bytes
     * @param configJson OCR configuration as JSON
     * @return extracted text, or null if processing fails
     */
    String processImage(byte[] imageData, String configJson) throws Exception;

    /**
     * Languages supported by this backend.
     *
     * @return list of language codes (empty for all languages)
     */
    List<String> supportedLanguages();
}
```

**Example - Custom OCR Backend:**

```java
class CustomOcrBackend implements OcrBackend {
    @Override
    public String processImage(byte[] imageData, String configJson) throws Exception {
        // Call custom OCR service
        return callCustomOcrService(imageData);
    }

    @Override
    public List<String> supportedLanguages() {
        return List.of("eng", "fra", "deu");
    }
}

// Register backend
OcrBackend backend = new CustomOcrBackend();
Kreuzberg.registerOcrBackend("custom-ocr", backend);
```

---

### ChunkingConfig

Configuration for splitting extracted text into chunks.

**Builder Methods:**

```java
ChunkingConfig chunking = ChunkingConfig.builder()
    .maxChars(1000)              // Maximum characters per chunk
    .maxOverlap(200)             // Character overlap between chunks
    .preset("large")             // Preset: "small", "medium", "large"
    .enabled(true)               // Enable chunking (default: true)
    .embedding(embeddingMap)     // Embedding configuration
    .build();
```

---

### LanguageDetectionConfig

Configuration for automatic language detection.

**Builder Methods:**

```java
LanguageDetectionConfig langDetect = LanguageDetectionConfig.builder()
    .enabled(true)               // Enable language detection
    .minConfidence(0.8)          // Minimum confidence threshold (0.0-1.0)
    .build();
```

---

### PdfConfig

PDF-specific extraction options.

**Builder Methods:**

```java
PdfConfig pdf = PdfConfig.builder()
    .extractImages(true)         // Extract images from PDF
    .extractMetadata(true)       // Extract PDF metadata
    .renderImages(false)         // Render pages as images for processing
    .build();
```

---

### ImageExtractionConfig

Configuration for image extraction from documents.

**Builder Methods:**

```java
ImageExtractionConfig images = ImageExtractionConfig.builder()
    .extractImages(true)         // Enable image extraction
    .targetDpi(150)              // Target DPI for extraction
    .maxImageDimension(4096)     // Maximum image dimension in pixels
    .build();
```

---

### ImagePreprocessingConfig

Configuration for preprocessing images before OCR.

**Builder Methods:**

```java
ImagePreprocessingConfig preproc = ImagePreprocessingConfig.builder()
    .targetDpi(300)              // Target DPI for OCR
    .denoise(true)               // Apply denoising
    .deskew(true)                // Deskew images
    .contrastEnhance(true)       // Enhance contrast
    .build();
```

---

### TokenReductionConfig

Configuration for token reduction (reducing extracted text size).

**Builder Methods:**

```java
TokenReductionConfig tokenReduce = TokenReductionConfig.builder()
    .mode("moderate")            // Mode: "none", "light", "moderate", "aggressive"
    .preserveImportantWords(true) // Preserve important words
    .build();
```

---

### PostProcessorConfig

Configuration for post-processing.

**Builder Methods:**

```java
PostProcessorConfig postproc = PostProcessorConfig.builder()
    .enabled(true)               // Enable post-processing
    .build();
```

---

### HtmlOptions

Configuration for HTML to Markdown conversion.

**Builder Methods:**

```java
HtmlOptions html = HtmlOptions.builder()
    .headingStyle("atx")         // "atx", "underlined", "atx_closed"
    .codeBlockStyle("backticks") // "indented", "backticks", "tildes"
    .build();
```

---

### KeywordConfig

Configuration for keyword extraction.

**Builder Methods:**

```java
KeywordConfig keywords = KeywordConfig.builder()
    .enabled(true)
    .maxKeywords(10)
    .minKeywordLength(3)
    .build();
```

---

## Results & Types

### ExtractionResult

Result of a document extraction operation.

**Accessors:**

```java
String content = result.getContent();                    // Extracted text content
String mimeType = result.getMimeType();                  // Detected MIME type
Map<String, Object> metadata = result.getMetadata();    // Document metadata
List<Table> tables = result.getTables();                // Extracted tables
List<String> languages = result.getDetectedLanguages(); // Detected languages
List<Chunk> chunks = result.getChunks();                // Text chunks
List<ExtractedImage> images = result.getImages();       // Extracted images
boolean success = result.isSuccess();                    // Extraction success flag

// Common metadata fields
Optional<String> language = result.getLanguage();       // Primary language
Optional<String> date = result.getDate();               // Document date
Optional<String> subject = result.getSubject();         // Document subject
```

**Example - Accessing results:**

```java
ExtractionResult result = Kreuzberg.extractFile("document.pdf");

System.out.println("Content length: " + result.getContent().length());
System.out.println("MIME: " + result.getMimeType());
System.out.println("Tables: " + result.getTables().size());
System.out.println("Languages: " + result.getDetectedLanguages());

// Access metadata
Object pageCount = result.getMetadata().get("page_count");
Object author = result.getMetadata().get("author");

// Access chunks for RAG
for (Chunk chunk : result.getChunks()) {
    System.out.println("Chunk " + chunk.getIndex() + ": " + chunk.getContent());
}
```

---

### Table

Represents a table extracted from a document.

**Accessors:**

```java
List<List<String>> cells = table.getCells();           // 2D list of cell values
String markdown = table.getMarkdown();                 // Markdown representation
int pageNumber = table.getPageNumber();                // Page number (1-indexed)

// Helper methods
int rows = table.getRowCount();                        // Number of rows
int cols = table.getColumnCount();                     // Number of columns
String cell = table.getCell(row, col);                // Get cell value
List<String> row = table.getRow(rowIndex);            // Get row
```

**Example:**

```java
List<Table> tables = result.getTables();

for (Table table : tables) {
    System.out.println("Table on page " + table.getPageNumber() + ":");
    System.out.println("Size: " + table.getRowCount() + " x " + table.getColumnCount());
    System.out.println(table.getMarkdown());

    // Iterate cells
    for (int r = 0; r < table.getRowCount(); r++) {
        for (int c = 0; c < table.getColumnCount(); c++) {
            System.out.print(table.getCell(r, c) + " | ");
        }
        System.out.println();
    }
}
```

---

### Chunk

Represents a chunk of extracted text (for RAG/embeddings).

**Accessors:**

```java
String content = chunk.getContent();                   // Chunk text
int index = chunk.getIndex();                          // Chunk index
Optional<Map<String, Object>> metadata = chunk.getMetadata(); // Chunk metadata
```

**Example:**

```java
ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(1000)
        .maxOverlap(200)
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);

for (Chunk chunk : result.getChunks()) {
    System.out.println("Chunk " + chunk.getIndex() + ": " + chunk.getContent().substring(0, 50) + "...");
}
```

---

### ExtractedImage

Represents an image extracted from a document.

**Accessors:**

```java
byte[] data = image.getData();                         // Image binary data
String format = image.getFormat();                     // Image format (png, jpg, etc.)
String mimeType = image.getMimeType();                 // MIME type
int pageNumber = image.getPageNumber();                // Page number
Optional<String> caption = image.getCaption();         // Image caption
```

---

## Extensibility

### Custom Post-Processors

Post-processors enrich extraction results by transforming content or adding metadata.

**Interface:**

```java
@FunctionalInterface
public interface PostProcessor {
    /**
     * Process and enrich an extraction result.
     *
     * @param result the extraction result
     * @return the processed result
     */
    ExtractionResult process(ExtractionResult result) throws KreuzbergException;

    /**
     * Compose with another processor.
     *
     * @param after the next processor
     * @return composed processor
     */
    default PostProcessor andThen(PostProcessor after) {
        return result -> after.process(this.process(result));
    }

    /**
     * Execution stage.
     *
     * @return the processing stage (EARLY, MIDDLE, LATE)
     */
    default ProcessingStage processingStage() {
        return ProcessingStage.MIDDLE;
    }

    /**
     * Execution priority within stage (higher = earlier).
     *
     * @return priority value
     */
    default int priority() {
        return 0;
    }
}
```

**Example - Word count processor:**

```java
import dev.kreuzberg.*;

PostProcessor wordCount = result -> {
    long count = result.getContent().split("\\s+").length;

    Map<String, Object> metadata = new HashMap<>(result.getMetadata());
    metadata.put("word_count", count);

    return new ExtractionResult(
        result.getContent(),
        result.getMimeType(),
        metadata,
        result.getTables(),
        result.getDetectedLanguages(),
        result.getChunks(),
        result.getImages(),
        result.isSuccess()
    );
};

// Register processor
Kreuzberg.registerPostProcessor("word-count", wordCount, 50, ProcessingStage.MIDDLE);

// Use in extraction
ExtractionResult result = Kreuzberg.extractFile("document.pdf");
System.out.println("Word count: " + result.getMetadata().get("word_count"));
```

**Example - Uppercase transformer:**

```java
PostProcessor uppercase = result -> {
    return new ExtractionResult(
        result.getContent().toUpperCase(),
        result.getMimeType(),
        result.getMetadata(),
        result.getTables(),
        result.getDetectedLanguages(),
        result.getChunks(),
        result.getImages(),
        result.isSuccess()
    );
};

Kreuzberg.registerPostProcessor("uppercase", uppercase);
```

---

### Custom Validators

Validators check extraction results for quality or completeness.

**Interface:**

```java
@FunctionalInterface
public interface Validator {
    /**
     * Validate an extraction result.
     *
     * @param result the extraction result
     * @throws ValidationException if validation fails
     */
    void validate(ExtractionResult result) throws ValidationException;

    /**
     * Compose with another validator.
     *
     * @param after the next validator
     * @return composed validator
     */
    default Validator andThen(Validator after) {
        return result -> {
            this.validate(result);
            after.validate(result);
        };
    }

    /**
     * Execution priority (higher = earlier).
     *
     * @return priority value
     */
    default int priority() {
        return 0;
    }
}
```

**Example - Minimum content length validator:**

```java
Validator minLength = result -> {
    if (result.getContent().length() < 100) {
        throw new ValidationException(
            "Content too short: " + result.getContent().length() + " < 100"
        );
    }
};

Kreuzberg.registerValidator("min-length", minLength);
```

**Example - Quality score validator:**

```java
Validator qualityValidator = result -> {
    double score = result.getMetadata().containsKey("quality_score")
        ? ((Number) result.getMetadata().get("quality_score")).doubleValue()
        : 0.0;

    if (score < 0.5) {
        throw new ValidationException(
            String.format("Quality score too low: %.2f < 0.50", score)
        );
    }
};

Kreuzberg.registerValidator("quality", qualityValidator);
```

---

### Plugin Management

Register, list, and unregister plugins.

**Post-Processor Management:**

```java
// Register with defaults
Kreuzberg.registerPostProcessor("processor-name", processor);

// Register with custom priority and stage
Kreuzberg.registerPostProcessor("processor-name", processor, 100, ProcessingStage.EARLY);

// Unregister specific processor
Kreuzberg.unregisterPostProcessor("processor-name");

// List all registered processors
List<String> processors = Kreuzberg.listPostProcessors();

// Clear all processors
Kreuzberg.clearPostProcessors();
```

**Validator Management:**

```java
// Register with defaults
Kreuzberg.registerValidator("validator-name", validator);

// Register with custom priority
Kreuzberg.registerValidator("validator-name", validator, 100);

// Unregister specific validator
Kreuzberg.unregisterValidator("validator-name");

// List all registered validators
List<String> validators = Kreuzberg.listValidators();

// Clear all validators
Kreuzberg.clearValidators();
```

**OCR Backend Management:**

```java
// Register backend
Kreuzberg.registerOcrBackend("backend-name", backend);

// Register with language filtering
Kreuzberg.registerOcrBackend("backend-name", backend, List.of("eng", "fra", "deu"));

// Unregister backend
Kreuzberg.unregisterOCRBackend("backend-name");

// List registered backends
List<String> backends = Kreuzberg.listOCRBackends();

// Clear all backends
Kreuzberg.clearOCRBackends();
```

---

## MIME Type Detection

### detectMimeType()

Detect MIME type from file or bytes.

**Signatures:**

```java
public static String detectMimeType(String path) throws KreuzbergException
public static String detectMimeType(String path, boolean checkExists) throws KreuzbergException
public static String detectMimeType(byte[] data) throws KreuzbergException
public static String detectMimeTypeFromPath(String path) throws KreuzbergException
```

**Example:**

```java
// From file path
String mimeType = Kreuzberg.detectMimeType("document.pdf");

// From file path without checking existence
String mimeType = Kreuzberg.detectMimeType("document.pdf", false);

// From raw bytes
byte[] data = /* ... */;
String mimeType = Kreuzberg.detectMimeType(data);
```

---

### validateMimeType()

Validate and normalize a MIME type string.

**Signature:**

```java
public static String validateMimeType(String mimeType) throws KreuzbergException
```

**Example:**

```java
String validated = Kreuzberg.validateMimeType("application/pdf");
System.out.println(validated); // "application/pdf"
```

---

### getExtensionsForMime()

Get file extensions for a given MIME type.

**Signature:**

```java
public static List<String> getExtensionsForMime(String mimeType) throws KreuzbergException
```

**Example:**

```java
List<String> extensions = Kreuzberg.getExtensionsForMime("application/pdf");
System.out.println(extensions); // ["pdf"]

List<String> extensions = Kreuzberg.getExtensionsForMime("image/jpeg");
System.out.println(extensions); // ["jpg", "jpeg"]
```

---

## Embeddings & Presets

### getEmbeddingPreset()

Get embedding preset configuration by name.

**Signature:**

```java
public static Optional<EmbeddingPreset> getEmbeddingPreset(String name) throws KreuzbergException
```

**Example:**

```java
Optional<EmbeddingPreset> preset = Kreuzberg.getEmbeddingPreset("default");
if (preset.isPresent()) {
    EmbeddingPreset p = preset.get();
    System.out.println("Model: " + p.getModel());
    System.out.println("Dimensions: " + p.getDimensions());
}
```

---

### listEmbeddingPresets()

List all available embedding presets.

**Signature:**

```java
public static List<String> listEmbeddingPresets() throws KreuzbergException
```

**Example:**

```java
List<String> presets = Kreuzberg.listEmbeddingPresets();
for (String preset : presets) {
    System.out.println("Available: " + preset);
}
```

---

## Error Handling

### Exception Hierarchy

Kreuzberg uses a checked exception model for error handling.

```
Exception
├── IOException (from java.io)
├── KreuzbergException
│   ├── ParsingException
│   ├── OcrException
│   ├── MissingDependencyException
│   ├── ValidationException
│   ├── PluginException
│   ├── CacheException
│   └── ImageProcessingException
```

### Specific Exceptions

**KreuzbergException** - Base exception for all Kreuzberg errors.

```java
try {
    ExtractionResult result = Kreuzberg.extractFile("document.pdf");
} catch (KreuzbergException e) {
    System.err.println("Extraction failed: " + e.getMessage());
    if (e.getCause() != null) {
        e.getCause().printStackTrace();
    }
}
```

**ParsingException** - Document parsing failure.

```java
try {
    ExtractionResult result = Kreuzberg.extractFile("corrupted.pdf");
} catch (ParsingException e) {
    System.err.println("Failed to parse document: " + e.getMessage());
}
```

**OcrException** - OCR processing failure.

```java
try {
    ExtractionConfig config = ExtractionConfig.builder()
        .forceOcr(true)
        .build();
    ExtractionResult result = Kreuzberg.extractFile("image.png", config);
} catch (OcrException e) {
    System.err.println("OCR failed: " + e.getMessage());
}
```

**MissingDependencyException** - Required system dependency not found.

```java
try {
    ExtractionResult result = Kreuzberg.extractFile("document.pdf");
} catch (MissingDependencyException e) {
    System.err.println("Missing dependency: " + e.getMessage());
    System.err.println("Install Tesseract or configure alternative OCR backend");
}
```

**ValidationException** - Configuration or validation failure.

```java
try {
    validator.validate(result);
} catch (ValidationException e) {
    System.err.println("Validation failed: " + e.getMessage());
}
```

### Comprehensive Error Handling

```java
try {
    ExtractionConfig config = ExtractionConfig.builder()
        .ocr(OcrConfig.builder().backend("tesseract").language("eng").build())
        .build();

    ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);
    System.out.println("Success: " + result.getContent().length() + " characters");

} catch (ParsingException e) {
    System.err.println("Document format not supported or corrupted");
    e.printStackTrace();
} catch (OcrException e) {
    System.err.println("OCR processing failed");
    e.printStackTrace();
} catch (MissingDependencyException e) {
    System.err.println("Missing required dependency");
    System.err.println("Message: " + e.getMessage());
} catch (ValidationException e) {
    System.err.println("Configuration validation failed");
} catch (IOException e) {
    System.err.println("File not found or not readable: " + e.getMessage());
} catch (KreuzbergException e) {
    System.err.println("Extraction failed: " + e.getMessage());
} finally {
    // Clean up if needed
}
```

---

## Utility Methods

### getVersion()

Get the Kreuzberg library version.

**Signature:**

```java
public static String getVersion()
```

**Example:**

```java
String version = Kreuzberg.getVersion();
System.out.println("Kreuzberg version: " + version);
```

---

## Advanced Usage

### Configuration Discovery

Automatically discover configuration from kreuzberg.toml, kreuzberg.yaml, or kreuzberg.json in the current or parent directories.

```java
ExtractionConfig config = ExtractionConfig.discover();

if (config != null) {
    System.out.println("Configuration discovered!");
    ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);
} else {
    System.out.println("No configuration file found, using defaults");
}
```

---

### Configuration from File

Load configuration from a file explicitly.

```java
ExtractionConfig config = ExtractionConfig.fromFile("kreuzberg.toml");
ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);
```

---

### Complex Configuration Example

```java
import dev.kreuzberg.*;
import dev.kreuzberg.config.*;
import java.nio.file.Path;

public class ComplexExample {
    public static void main(String[] args) throws Exception {
        // Configure everything
        ExtractionConfig config = ExtractionConfig.builder()
            .useCache(true)
            .enableQualityProcessing(true)
            .forceOcr(false)

            // OCR settings
            .ocr(OcrConfig.builder()
                .backend("tesseract")
                .language("eng+fra")
                .build())

            // PDF settings
            .pdfOptions(PdfConfig.builder()
                .extractImages(true)
                .extractMetadata(true)
                .build())

            // Image processing
            .imagePreprocessing(ImagePreprocessingConfig.builder()
                .targetDpi(300)
                .denoise(true)
                .deskew(true)
                .contrastEnhance(true)
                .build())

            // Chunking for RAG
            .chunking(ChunkingConfig.builder()
                .maxChars(1000)
                .maxOverlap(200)
                .enabled(true)
                .build())

            // Language detection
            .languageDetection(LanguageDetectionConfig.builder()
                .enabled(true)
                .minConfidence(0.8)
                .build())

            .build();

        // Register custom plugins
        PostProcessor uppercaser = result -> new ExtractionResult(
            result.getContent().toUpperCase(),
            result.getMimeType(),
            result.getMetadata(),
            result.getTables(),
            result.getDetectedLanguages(),
            result.getChunks(),
            result.getImages(),
            result.isSuccess()
        );

        Validator minLength = result -> {
            if (result.getContent().length() < 100) {
                throw new ValidationException("Content too short");
            }
        };

        Kreuzberg.registerPostProcessor("uppercase", uppercaser, 100, ProcessingStage.EARLY);
        Kreuzberg.registerValidator("min-length", minLength);

        // Extract
        ExtractionResult result = Kreuzberg.extractFile("document.pdf", config);

        // Process results
        System.out.println("Content: " + result.getContent().substring(0, 100));
        System.out.println("Tables: " + result.getTables().size());
        System.out.println("Images: " + result.getImages().size());
        System.out.println("Chunks: " + result.getChunks().size());
        System.out.println("Language: " + result.getLanguage());
        System.out.println("MIME: " + result.getMimeType());
    }
}
```

---

### Batch Processing with Error Handling

```java
import dev.kreuzberg.*;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

public class BatchProcessor {
    public static void main(String[] args) throws Exception {
        List<Path> files = Files.list(Path.of("documents/"))
            .filter(p -> p.toString().endsWith(".pdf"))
            .toList();

        List<String> filePaths = new ArrayList<>();
        for (Path file : files) {
            filePaths.add(file.toString());
        }

        ExtractionConfig config = ExtractionConfig.builder()
            .useCache(true)
            .build();

        try {
            List<ExtractionResult> results = Kreuzberg.batchExtractFiles(filePaths, config);

            for (int i = 0; i < filePaths.size(); i++) {
                ExtractionResult result = results.get(i);
                Path file = files.get(i);

                if (result.isSuccess()) {
                    System.out.println(file + ": " + result.getContent().length() + " chars");
                } else {
                    System.err.println(file + ": extraction failed");
                }
            }
        } catch (KreuzbergException e) {
            System.err.println("Batch extraction failed: " + e.getMessage());
        }
    }
}
```

---

## Performance Tips

1. **Reuse configurations** - Create one `ExtractionConfig` and use it for multiple extractions
2. **Batch processing** - Use `batchExtractFiles()` for multiple files instead of individual calls
3. **Cache enabled** - Keep caching enabled for repeated document processing
4. **OCR selective** - Only enable OCR on pages that need it (`forceOcr = false`)
5. **Image preprocessing** - Enable image preprocessing for better OCR accuracy
6. **Async operations** - Use async methods for non-blocking extraction in concurrent scenarios

---

## Supported File Formats

- **Documents**: PDF, DOCX, DOC, XLSX, XLS, PPTX, PPT, ODT, ODP, ODS
- **Images**: PNG, JPG, JPEG, GIF, BMP, WebP, TIFF
- **Web**: HTML, MHTML
- **Text**: TXT, CSV

---

## Java FFM API Details

The Kreuzberg Java bindings use Java's Foreign Function & Memory (FFM) API for direct FFI without JNI overhead.

**Memory Management:**

```java
try (Arena arena = Arena.ofConfined()) {
    // FFI operations use arena for memory management
    ExtractionResult result = Kreuzberg.extractFile("document.pdf");
} // Arena automatically cleaned up
```

**Arena Types:**

- `Arena.ofConfined()` - Thread-confined arena (recommended)
- `Arena.ofShared()` - Shared arena for multi-threaded access

---

## Troubleshooting

**"Failed to load native library"** - Ensure libkreuzberg_ffi is in system library path.

```bash
export LD_LIBRARY_PATH=/path/to/libkreuzberg_ffi:$LD_LIBRARY_PATH  # Linux/Unix
export DYLD_LIBRARY_PATH=/path/to/libkreuzberg_ffi:$DYLD_LIBRARY_PATH  # macOS
set PATH=C:\path\to\libkreuzberg_ffi;%PATH%  # Windows
```

**"Tesseract not found"** - Install Tesseract OCR:

```bash
# Ubuntu/Debian
sudo apt-get install tesseract-ocr

# macOS
brew install tesseract

# Windows
# Download from https://github.com/UB-Mannheim/tesseract/wiki
```

**"OutOfMemoryError with large files"** - Use streaming or batch processing with smaller batches.
