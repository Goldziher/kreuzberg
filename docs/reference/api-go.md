# Go API Reference

Complete reference for the Kreuzberg Go bindings using cgo to access the Rust-powered extraction pipeline.

The Go binding exposes the same extraction capabilities as the other languages through C FFI bindings to `kreuzberg-ffi`. You get identical metadata extraction, OCR processing, chunking, embeddings, and plugin support—with synchronous and context-aware async APIs.

## Requirements

- **Go 1.25+** (with cgo support)
- **Rust toolchain** (builds `kreuzberg-ffi`)
- **C compiler** (gcc/clang for cgo compilation)
- **libkreuzberg_ffi** native library (staged in `target/release`)
- **libpdfium** runtime (auto-discovered via `target/release`)
- **Tesseract/EasyOCR/PaddleOCR** (optional, for OCR functionality)

## Installation

Add the package to your `go.mod`:

```bash
go get github.com/Goldziher/kreuzberg/packages/go/kreuzberg@latest
```

Build the FFI library and set library paths:

```bash
# Build the FFI crate
cargo build -p kreuzberg-ffi --release

# Configure library path for your platform
# Linux
export LD_LIBRARY_PATH=$PWD/target/release:$LD_LIBRARY_PATH

# macOS
export DYLD_FALLBACK_LIBRARY_PATH=$PWD/target/release:$DYLD_FALLBACK_LIBRARY_PATH

# Windows
# Add target\release to PATH environment variable
set PATH=%CD%\target\release;%PATH%
```

## Quickstart

### Basic file extraction (synchronous)

```go
package main

import (
	"fmt"
	"log"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	result, err := kreuzberg.ExtractFileSync("document.pdf", nil)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Printf("Format: %s\n", result.MimeType)
	fmt.Printf("Content length: %d\n", len(result.Content))
	fmt.Printf("Success: %v\n", result.Success)
}
```

### Async extraction with timeout

```go
package main

import (
	"context"
	"errors"
	"log"
	"time"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()

	result, err := kreuzberg.ExtractFile(ctx, "large-document.pdf", nil)
	if errors.Is(err, context.DeadlineExceeded) {
		log.Println("extraction timed out")
		return
	}
	if err != nil {
		log.Fatalf("extraction failed: %v", err)
	}

	log.Printf("Extracted %d characters\n", len(result.Content))
}
```

---

## Core Functions

### ExtractFileSync

Extract content and metadata from a file synchronously.

**Signature:**

```go
func ExtractFileSync(path string, config *ExtractionConfig) (*ExtractionResult, error)
```

**Parameters:**

- `path` (string): Path to the file to extract (absolute or relative)
- `config` (*ExtractionConfig): Optional extraction configuration; uses defaults if nil

**Returns:**

- `*ExtractionResult`: Populated result containing content, metadata, tables, chunks, and images
- `error`: KreuzbergError or standard Go error (see Error Handling section)

**Error Handling:**

- `ValidationError`: If path is empty
- `IOError`: If file not found or not readable
- `ParsingError`: If document parsing fails
- `MissingDependencyError`: If required OCR/processing library is missing
- `UnsupportedFormatError`: If MIME type is not supported

**Example - Extract PDF:**

```go
result, err := kreuzberg.ExtractFileSync("report.pdf", nil)
if err != nil {
	log.Fatalf("extraction failed: %v", err)
}

fmt.Printf("Title: %s\n", *result.Metadata.PdfMetadata().Title)
fmt.Printf("Page count: %d\n", *result.Metadata.PdfMetadata().PageCount)
fmt.Printf("Content preview: %s...\n", result.Content[:100])
```

**Example - Extract with configuration:**

```go
cfg := &kreuzberg.ExtractionConfig{
	UseCache: boolPtr(true),
	OCR: &kreuzberg.OCRConfig{
		Backend:  "tesseract",
		Language: stringPtr("eng"),
	},
}

result, err := kreuzberg.ExtractFileSync("scanned.pdf", cfg)
if err != nil {
	log.Fatalf("extraction failed: %v", err)
}
```

---

### ExtractFile

Extract content from a file asynchronously with context support.

**Signature:**

```go
func ExtractFile(ctx context.Context, path string, config *ExtractionConfig) (*ExtractionResult, error)
```

**Parameters:**

- `ctx` (context.Context): Context for cancellation and timeout
- `path` (string): Path to the file
- `config` (*ExtractionConfig): Optional configuration

**Returns:**

- `*ExtractionResult`: Extraction result
- `error`: May include context errors (context.DeadlineExceeded, context.Canceled)

**Note:** Context cancellation is best-effort. The underlying C call cannot be interrupted, but the function returns immediately with ctx.Err() when the context deadline is exceeded or cancelled.

**Example - With deadline:**

```go
ctx, cancel := context.WithDeadline(context.Background(), time.Now().Add(30*time.Second))
defer cancel()

result, err := kreuzberg.ExtractFile(ctx, "large.docx", nil)
if errors.Is(err, context.DeadlineExceeded) {
	log.Println("extraction took too long")
	return
}
if err != nil {
	log.Fatalf("extraction failed: %v", err)
}
```

---

### ExtractBytesSync

Extract content from an in-memory byte slice with specified MIME type.

**Signature:**

```go
func ExtractBytesSync(data []byte, mimeType string, config *ExtractionConfig) (*ExtractionResult, error)
```

**Parameters:**

- `data` ([]byte): Document bytes
- `mimeType` (string): MIME type (e.g., "application/pdf", "text/plain")
- `config` (*ExtractionConfig): Optional configuration

**Returns:**

- `*ExtractionResult`: Extraction result
- `error`: KreuzbergError on extraction failure

**Example - Extract from downloaded PDF:**

```go
httpResp, err := http.Get("https://example.com/document.pdf")
if err != nil {
	log.Fatal(err)
}
defer httpResp.Body.Close()

data, err := io.ReadAll(httpResp.Body)
if err != nil {
	log.Fatal(err)
}

result, err := kreuzberg.ExtractBytesSync(data, "application/pdf", nil)
if err != nil {
	log.Fatalf("extraction failed: %v", err)
}

fmt.Printf("Extracted %d words\n", len(strings.Fields(result.Content)))
```

---

### ExtractBytes

Extract content from in-memory bytes asynchronously.

**Signature:**

```go
func ExtractBytes(ctx context.Context, data []byte, mimeType string, config *ExtractionConfig) (*ExtractionResult, error)
```

**Parameters:**

- `ctx` (context.Context): Context for cancellation and timeout
- `data` ([]byte): Document bytes
- `mimeType` (string): MIME type
- `config` (*ExtractionConfig): Optional configuration

**Returns:**

- `*ExtractionResult`: Extraction result
- `error`: KreuzbergError or context error

**Example:**

```go
ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
defer cancel()

result, err := kreuzberg.ExtractBytes(ctx, data, "text/html", nil)
if err != nil {
	log.Fatalf("extraction failed: %v", err)
}
```

---

### BatchExtractFilesSync

Extract multiple files sequentially using the optimized batch pipeline.

**Signature:**

```go
func BatchExtractFilesSync(paths []string, config *ExtractionConfig) ([]*ExtractionResult, error)
```

**Parameters:**

- `paths` ([]string): Slice of file paths
- `config` (*ExtractionConfig): Configuration applied to all files

**Returns:**

- `[]*ExtractionResult`: Slice of results (one per input file; may contain nils for failed extractions)
- `error`: Returned only if batch setup fails; individual file errors are captured in ErrorMetadata

**Example - Batch extract multiple PDFs:**

```go
files := []string{"doc1.pdf", "doc2.pdf", "doc3.pdf"}

results, err := kreuzberg.BatchExtractFilesSync(files, nil)
if err != nil {
	log.Fatalf("batch extraction setup failed: %v", err)
}

for i, result := range results {
	if result == nil {
		fmt.Printf("File %d: extraction failed\n", i)
		continue
	}

	if result.Metadata.Error != nil {
		fmt.Printf("File %d: %s (%s)\n", i, result.Metadata.Error.ErrorType, result.Metadata.Error.Message)
		continue
	}

	fmt.Printf("File %d: extracted %d chars\n", i, len(result.Content))
}
```

---

### BatchExtractFiles

Batch extract multiple files asynchronously.

**Signature:**

```go
func BatchExtractFiles(ctx context.Context, paths []string, config *ExtractionConfig) ([]*ExtractionResult, error)
```

**Parameters:**

- `ctx` (context.Context): Context for cancellation
- `paths` ([]string): File paths
- `config` (*ExtractionConfig): Configuration for all files

**Returns:**

- `[]*ExtractionResult`: Results slice
- `error`: Context or setup errors

---

### BatchExtractBytesSync

Extract multiple in-memory documents in a single batch operation.

**Signature:**

```go
func BatchExtractBytesSync(items []BytesWithMime, config *ExtractionConfig) ([]*ExtractionResult, error)
```

**Parameters:**

- `items` ([]BytesWithMime): Slice of {Data, MimeType} pairs
- `config` (*ExtractionConfig): Configuration applied to all items

**Returns:**

- `[]*ExtractionResult`: Results slice
- `error`: Setup error or validation error

**BytesWithMime structure:**

```go
type BytesWithMime struct {
	Data     []byte
	MimeType string
}
```

**Example - Batch extract multiple formats:**

```go
items := []kreuzberg.BytesWithMime{
	{Data: pdfData, MimeType: "application/pdf"},
	{Data: docxData, MimeType: "application/vnd.openxmlformats-officedocument.wordprocessingml.document"},
	{Data: htmlData, MimeType: "text/html"},
}

results, err := kreuzberg.BatchExtractBytesSync(items, nil)
if err != nil {
	log.Fatalf("batch extraction failed: %v", err)
}

for i, result := range results {
	if result == nil || !result.Success {
		log.Printf("Item %d extraction failed\n", i)
		continue
	}
	log.Printf("Item %d: %s format\n", i, result.MimeType)
}
```

---

### BatchExtractBytes

Batch extract in-memory documents asynchronously.

**Signature:**

```go
func BatchExtractBytes(ctx context.Context, items []BytesWithMime, config *ExtractionConfig) ([]*ExtractionResult, error)
```

**Parameters:**

- `ctx` (context.Context): Context for cancellation
- `items` ([]BytesWithMime): Document slice
- `config` (*ExtractionConfig): Configuration

**Returns:**

- `[]*ExtractionResult`: Results slice
- `error`: Context or setup errors

---

### LibraryVersion

Get the version of the underlying Rust library.

**Signature:**

```go
func LibraryVersion() string
```

**Returns:**

- `string`: Version string (e.g., "4.0.0-rc.1")

**Example:**

```go
fmt.Printf("Kreuzberg version: %s\n", kreuzberg.LibraryVersion())
```

---

## Configuration

### ExtractionConfig

Root configuration struct for all extraction operations. All fields are optional (pointers); omitted fields use Kreuzberg defaults.

**Signature:**

```go
type ExtractionConfig struct {
	UseCache                 *bool                    // Enable result caching
	EnableQualityProcessing  *bool                    // Run quality improvements
	OCR                      *OCRConfig               // OCR backend and settings
	ForceOCR                 *bool                    // Force OCR even for text-extractable docs
	Chunking                 *ChunkingConfig          // Text chunking and embeddings
	Images                   *ImageExtractionConfig   // Image extraction from docs
	PdfOptions               *PdfConfig               // PDF-specific options
	TokenReduction           *TokenReductionConfig    // Token pruning before embeddings
	LanguageDetection        *LanguageDetectionConfig // Language detection settings
	Keywords                 *KeywordConfig           // Keyword extraction
	Postprocessor            *PostProcessorConfig     // Post-processor selection
	HTMLOptions              *HTMLConversionOptions   // HTML-to-Markdown conversion
	MaxConcurrentExtractions *int                     // Batch concurrency limit
}
```

---

### OCRConfig

Configure OCR backend selection and language.

**Signature:**

```go
type OCRConfig struct {
	Backend   string           // OCR backend name: "tesseract", "easyocr", "paddle", etc.
	Language  *string          // Language code (e.g., "eng", "deu", "fra")
	Tesseract *TesseractConfig // Tesseract-specific fine-tuning
}
```

**Example:**

```go
cfg := &kreuzberg.ExtractionConfig{
	OCR: &kreuzberg.OCRConfig{
		Backend:  "tesseract",
		Language: stringPtr("eng"),
		Tesseract: &kreuzberg.TesseractConfig{
			PSM: intPtr(3),
			MinConfidence: float64Ptr(0.5),
		},
	},
}
```

---

### TesseractConfig

Fine-grained Tesseract OCR tuning.

**Signature:**

```go
type TesseractConfig struct {
	Language                       string                    // Language code
	PSM                            *int                      // Page segmentation mode (0-13)
	OutputFormat                   string                    // Output format: "text", "pdf", "hocr"
	OEM                            *int                      // Engine mode (0-3)
	MinConfidence                  *float64                  // Confidence threshold (0.0-1.0)
	Preprocessing                  *ImagePreprocessingConfig // Image preprocessing
	EnableTableDetection           *bool                     // Detect and extract tables
	TableMinConfidence             *float64                  // Table detection confidence
	TableColumnThreshold           *int                      // Column separation threshold
	TableRowThresholdRatio         *float64                  // Row separation ratio
	UseCache                       *bool                     // Cache OCR results
	// Additional Tesseract parameters...
	TesseditCharWhitelist          string                    // Character whitelist
	TesseditCharBlacklist          string                    // Character blacklist
}
```

---

### ImagePreprocessingConfig

Configure OCR image preprocessing (DPI normalization, rotation, denoising, etc.).

**Signature:**

```go
type ImagePreprocessingConfig struct {
	TargetDPI        *int   // Target DPI for OCR (typically 300)
	AutoRotate       *bool  // Auto-detect and correct image rotation
	Deskew           *bool  // Correct skewed text
	Denoise          *bool  // Remove noise
	ContrastEnhance  *bool  // Enhance contrast
	BinarizationMode string // Binarization method: "otsu", "adaptive"
	InvertColors     *bool  // Invert black/white
}
```

---

### ChunkingConfig

Configure text chunking for RAG and retrieval workloads.

**Signature:**

```go
type ChunkingConfig struct {
	MaxChars     *int             // Maximum characters per chunk
	MaxOverlap   *int             // Overlap between chunks
	ChunkSize    *int             // Alias for MaxChars
	ChunkOverlap *int             // Alias for MaxOverlap
	Preset       *string          // Preset: "semantic", "sliding", "recursive"
	Embedding    *EmbeddingConfig // Embedding generation
	Enabled      *bool            // Enable chunking
}
```

---

### ImageExtractionConfig

Configure image extraction from documents.

**Signature:**

```go
type ImageExtractionConfig struct {
	ExtractImages     *bool // Extract embedded images
	TargetDPI         *int  // Target DPI for extraction
	MaxImageDimension *int  // Maximum dimension (width/height)
	AutoAdjustDPI     *bool // Auto-adjust DPI for small images
	MinDPI            *int  // Minimum DPI threshold
	MaxDPI            *int  // Maximum DPI threshold
}
```

---

### PdfConfig

PDF-specific extraction options.

**Signature:**

```go
type PdfConfig struct {
	ExtractImages   *bool    // Extract embedded images
	Passwords       []string // List of passwords for encrypted PDFs
	ExtractMetadata *bool    // Extract document metadata
}
```

---

### EmbeddingConfig

Configure embedding generation for chunks.

**Signature:**

```go
type EmbeddingConfig struct {
	Model                *EmbeddingModelType // Model selection
	Normalize            *bool               // L2 normalization
	BatchSize            *int                // Batch size for inference
	ShowDownloadProgress *bool               // Show download progress
	CacheDir             *string             // Cache directory
}

type EmbeddingModelType struct {
	Type       string // "preset", "fastembed", "custom"
	Name       string // For preset models
	Model      string // For fastembed/custom
	ModelID    string // Alias for custom
	Dimensions *int   // Embedding dimensions
}
```

---

### KeywordConfig

Configure keyword extraction.

**Signature:**

```go
type KeywordConfig struct {
	Algorithm   string      // "yake" or "rake"
	MaxKeywords *int        // Maximum keywords to extract
	MinScore    *float64    // Minimum keyword score
	NgramRange  *[2]int     // N-gram range: [min, max]
	Language    *string     // Language code
	Yake        *YakeParams // YAKE-specific tuning
	Rake        *RakeParams // RAKE-specific tuning
}

type YakeParams struct {
	WindowSize *int
}

type RakeParams struct {
	MinWordLength     *int
	MaxWordsPerPhrase *int
}
```

---

### PostProcessorConfig

Configure post-processing steps.

**Signature:**

```go
type PostProcessorConfig struct {
	Enabled            *bool    // Enable post-processing
	EnabledProcessors  []string // Specific processors to run
	DisabledProcessors []string // Processors to skip
}
```

---

## Results & Types

### ExtractionResult

The main result struct containing all extracted data.

**Signature:**

```go
type ExtractionResult struct {
	Content           string           // Extracted text content
	MimeType          string           // Detected MIME type
	Metadata          Metadata         // Document metadata
	Tables            []Table          // Extracted tables
	DetectedLanguages []string         // Detected languages
	Chunks            []Chunk          // Text chunks (if enabled)
	Images            []ExtractedImage // Embedded images (if enabled)
	Success           bool             // Extraction success flag
}
```

**Example - Accessing results:**

```go
result, err := kreuzberg.ExtractFileSync("report.pdf", nil)
if err != nil || !result.Success {
	log.Fatal("extraction failed")
}

fmt.Printf("Detected MIME type: %s\n", result.MimeType)
fmt.Printf("Content length: %d\n", len(result.Content))
fmt.Printf("Detected languages: %v\n", result.DetectedLanguages)
fmt.Printf("Number of tables: %d\n", len(result.Tables))
fmt.Printf("Number of chunks: %d\n", len(result.Chunks))
fmt.Printf("Number of images: %d\n", len(result.Images))
```

---

### Metadata

Aggregated document metadata with format-specific fields.

**Signature:**

```go
type Metadata struct {
	Language           *string                     // Detected language code
	Date               *string                     // Extracted document date
	Subject            *string                     // Document subject
	Format             FormatMetadata              // Format-specific metadata
	ImagePreprocessing *ImagePreprocessingMetadata // OCR preprocessing info
	JSONSchema         json.RawMessage             // JSON Schema if available
	Error              *ErrorMetadata              // Error info for batch operations
	Additional         map[string]json.RawMessage  // Custom/additional fields
}
```

**Access format-specific metadata:**

```go
// Type discriminator
fmt.Println("Format type:", result.Metadata.FormatType())

// Type-safe accessors
if pdfMeta, ok := result.Metadata.PdfMetadata(); ok {
	fmt.Printf("Title: %s\n", *pdfMeta.Title)
	fmt.Printf("Pages: %d\n", *pdfMeta.PageCount)
	fmt.Printf("Author: %s\n", *pdfMeta.Authors[0])
}

if excelMeta, ok := result.Metadata.ExcelMetadata(); ok {
	fmt.Printf("Sheets: %d\n", excelMeta.SheetCount)
	fmt.Printf("Sheet names: %v\n", excelMeta.SheetNames)
}

if htmlMeta, ok := result.Metadata.HTMLMetadata(); ok {
	fmt.Printf("Page title: %s\n", *htmlMeta.Title)
	fmt.Printf("OG image: %s\n", *htmlMeta.OGImage)
}
```

---

### Table

Extracted table structure.

**Signature:**

```go
type Table struct {
	Cells      [][]string // 2D cell array [row][col]
	Markdown   string     // Markdown representation
	PageNumber int        // Page number (PDF/Image documents)
}
```

**Example:**

```go
for tableIdx, table := range result.Tables {
	fmt.Printf("Table %d (page %d):\n", tableIdx, table.PageNumber)
	for _, row := range table.Cells {
		fmt.Println(strings.Join(row, " | "))
	}
	fmt.Println("Markdown:", table.Markdown)
}
```

---

### Chunk

Text chunk with optional embeddings and metadata.

**Signature:**

```go
type Chunk struct {
	Content   string        // Chunk text
	Embedding []float32     // Embedding vector (if enabled)
	Metadata  ChunkMetadata // Chunk positioning
}

type ChunkMetadata struct {
	CharStart   int  // Character offset in original content
	CharEnd     int  // End character offset
	TokenCount  *int // Token count (if available)
	ChunkIndex  int  // Index in chunk sequence
	TotalChunks int  // Total number of chunks
}
```

**Example:**

```go
for _, chunk := range result.Chunks {
	fmt.Printf("Chunk %d/%d\n", chunk.Metadata.ChunkIndex, chunk.Metadata.TotalChunks)
	fmt.Printf("Content: %s...\n", chunk.Content[:min(50, len(chunk.Content))])
	fmt.Printf("Tokens: %d\n", *chunk.Metadata.TokenCount)
	if len(chunk.Embedding) > 0 {
		fmt.Printf("Embedding dim: %d\n", len(chunk.Embedding))
		fmt.Printf("First 5 values: %v\n", chunk.Embedding[:5])
	}
}
```

---

### ExtractedImage

Image extracted from document with optional OCR results.

**Signature:**

```go
type ExtractedImage struct {
	Data             []byte            // Raw image bytes
	Format           string            // Image format: "jpeg", "png", "webp"
	ImageIndex       int               // Index in images list
	PageNumber       *int              // Page number (if applicable)
	Width            *uint32           // Image width in pixels
	Height           *uint32           // Image height in pixels
	Colorspace       *string           // Colorspace (sRGB, CMYK, etc.)
	BitsPerComponent *uint32           // Bits per color component
	IsMask           bool              // Is image a mask?
	Description      *string           // Image description/alt text
	OCRResult        *ExtractionResult // Nested OCR extraction
}
```

**Example:**

```go
for imgIdx, img := range result.Images {
	fmt.Printf("Image %d: %s, %dx%d\n", imgIdx, img.Format, *img.Width, *img.Height)

	// Save image
	filename := fmt.Sprintf("image_%d.%s", imgIdx, img.Format)
	os.WriteFile(filename, img.Data, 0644)

	// OCR if available
	if img.OCRResult != nil {
		fmt.Printf("Image %d OCR: %s\n", imgIdx, img.OCRResult.Content)
	}
}
```

---

## Error Handling

### Error Types

Kreuzberg defines a type hierarchy of errors via the `KreuzbergError` interface:

```go
type KreuzbergError interface {
	error
	Kind() ErrorKind
}

type ErrorKind string

const (
	ErrorKindUnknown           ErrorKind = "unknown"
	ErrorKindIO                ErrorKind = "io"
	ErrorKindValidation        ErrorKind = "validation"
	ErrorKindParsing           ErrorKind = "parsing"
	ErrorKindOCR               ErrorKind = "ocr"
	ErrorKindCache             ErrorKind = "cache"
	ErrorKindImageProcessing   ErrorKind = "image_processing"
	ErrorKindSerialization     ErrorKind = "serialization"
	ErrorKindMissingDependency ErrorKind = "missing_dependency"
	ErrorKindPlugin            ErrorKind = "plugin"
	ErrorKindUnsupportedFormat ErrorKind = "unsupported_format"
	ErrorKindRuntime           ErrorKind = "runtime"
)
```

**Error type classes:**

- `ValidationError`: Input validation failed (empty paths, missing MIME types)
- `ParsingError`: Document parsing failed (malformed file, unsupported format)
- `OCRError`: OCR backend failure (library missing, invalid language)
- `CacheError`: Cache operation failed
- `ImageProcessingError`: Image manipulation failed
- `SerializationError`: JSON encoding/decoding failed
- `MissingDependencyError`: Required library not found (Tesseract, EasyOCR, etc.)
- `PluginError`: Plugin registration or execution failed
- `UnsupportedFormatError`: MIME type not supported
- `IOError`: File I/O failure
- `RuntimeError`: Unexpected runtime failure (lock poisoning, etc.)

---

### Error Classification

Errors are automatically classified based on native error messages. Use `errors.As()` and `errors.Is()` to handle specific error types:

```go
import (
	"errors"
	"log"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

result, err := kreuzberg.ExtractFileSync("document.pdf", nil)
if err != nil {
	// Check specific error type
	var parsingErr *kreuzberg.ParsingError
	if errors.As(err, &parsingErr) {
		log.Printf("Parsing failed: %v\n", parsingErr)
		return
	}

	var missingDep *kreuzberg.MissingDependencyError
	if errors.As(err, &missingDep) {
		log.Printf("Missing dependency: %s\n", missingDep.Dependency)
		return
	}

	// Generic error handling
	log.Printf("Extraction failed: %v\n", err)
}
```

---

### Error Unwrapping

All Kreuzberg errors support error unwrapping via `errors.Unwrap()`:

```go
result, err := kreuzberg.ExtractFileSync("doc.pdf", nil)
if err != nil {
	// Check root cause
	rootErr := errors.Unwrap(err)
	if rootErr != nil {
		log.Printf("Root cause: %v\n", rootErr)
	}

	// Check error kind
	if krErr, ok := err.(kreuzberg.KreuzbergError); ok {
		log.Printf("Error kind: %v\n", krErr.Kind())
	}
}
```

---

### Error Handling Examples

**Handle file not found:**

```go
result, err := kreuzberg.ExtractFileSync("missing.pdf", nil)
if err != nil {
	var ioErr *kreuzberg.IOError
	if errors.As(err, &ioErr) {
		log.Println("File not found or unreadable")
		return
	}
	log.Fatalf("unexpected error: %v\n", err)
}
```

**Handle missing OCR dependency:**

```go
cfg := &kreuzberg.ExtractionConfig{
	OCR: &kreuzberg.OCRConfig{
		Backend:  "tesseract",
		Language: stringPtr("eng"),
	},
}

result, err := kreuzberg.ExtractFileSync("scanned.pdf", cfg)
if err != nil {
	var missingDep *kreuzberg.MissingDependencyError
	if errors.As(err, &missingDep) {
		log.Printf("Install %s to use OCR\n", missingDep.Dependency)
		return
	}
	log.Fatalf("extraction failed: %v\n", err)
}
```

**Batch error handling:**

```go
results, err := kreuzberg.BatchExtractFilesSync(files, nil)
if err != nil {
	log.Fatalf("batch setup failed: %v\n", err)
}

for i, result := range results {
	if result == nil {
		log.Printf("File %d: extraction failed (nil result)\n", i)
		continue
	}

	// Check for per-file errors
	if result.Metadata.Error != nil {
		log.Printf("File %d: %s - %s\n", i, result.Metadata.Error.ErrorType, result.Metadata.Error.Message)
		continue
	}

	if !result.Success {
		log.Printf("File %d: extraction unsuccessful\n", i)
		continue
	}

	log.Printf("File %d: success (%d chars)\n", i, len(result.Content))
}
```

---

## Advanced Usage

### MIME Type Detection

Detect MIME type from file extension or content:

```go
// Detect from filename (requires kreuzberg-ffi binding support)
// Use system tools or your own MIME database
mimeType := "application/pdf" // e.g., use mime.TypeByExtension(".pdf")
```

---

### CGO-Specific Patterns

#### Memory Management

Go's cgo automatically manages C memory for simple types. Kreuzberg handles C pointer cleanup internally via `defer` statements:

```go
// Safe: strings are copied to Go memory, C strings freed internally
result, err := kreuzberg.ExtractFileSync("doc.pdf", nil)

// Safe: byte slices are copied, C buffers freed internally
result, err := kreuzberg.ExtractBytesSync(data, "application/pdf", nil)
```

#### Library Path Configuration

Set library paths before running your program:

**Linux:**

```bash
export LD_LIBRARY_PATH=$PWD/target/release:$LD_LIBRARY_PATH
go run main.go
```

**macOS:**

```bash
export DYLD_FALLBACK_LIBRARY_PATH=$PWD/target/release:$DYLD_FALLBACK_LIBRARY_PATH
go run main.go
```

**Windows:**

```cmd
set PATH=%CD%\target\release;%PATH%
go run main.go
```

#### Configuration as JSON

Internally, ExtractionConfig is serialized to JSON and passed to the C FFI:

```go
// This internally becomes:
// {
//   "use_cache": true,
//   "ocr": {
//     "backend": "tesseract",
//     "language": "eng"
//   }
// }

cfg := &kreuzberg.ExtractionConfig{
	UseCache: boolPtr(true),
	OCR: &kreuzberg.OCRConfig{
		Backend:  "tesseract",
		Language: stringPtr("eng"),
	},
}

result, err := kreuzberg.ExtractFileSync("doc.pdf", cfg)
```

---

### Custom Post-Processors

Register custom post-processing logic in Go:

```go
package main

import (
	"C"
	"encoding/json"
	"log"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

//export myCustomProcessor
func myCustomProcessor(resultJSON *C.char) *C.char {
	// Parse JSON result
	jsonStr := C.GoString(resultJSON)
	var result kreuzberg.ExtractionResult
	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		// Return error as C string (Rust will free it)
		errMsg := C.CString("failed to parse JSON")
		return errMsg
	}

	// Modify content
	result.Content = strings.ToUpper(result.Content)

	// Serialize back to JSON
	modified, _ := json.Marshal(result)
	return C.CString(string(modified))
}

func init() {
	err := kreuzberg.RegisterPostProcessor(
		"go-uppercase",
		100, // priority
		(C.PostProcessorCallback)(C.myCustomProcessor),
	)
	if err != nil {
		log.Fatalf("failed to register post-processor: %v\n", err)
	}
}

func main() {
	cfg := &kreuzberg.ExtractionConfig{
		Postprocessor: &kreuzberg.PostProcessorConfig{
			EnabledProcessors: []string{"go-uppercase"},
		},
	}

	result, _ := kreuzberg.ExtractFileSync("doc.pdf", cfg)
	// Content is now uppercase
}
```

---

### Custom Validators

Validate extraction results:

```go
//export myValidator
func myValidator(resultJSON *C.char) *C.char {
	jsonStr := C.GoString(resultJSON)
	var result kreuzberg.ExtractionResult
	json.Unmarshal([]byte(jsonStr), &result)

	// Validation logic
	if len(result.Content) == 0 {
		errMsg := C.CString("content is empty")
		return errMsg
	}

	// NULL means validation passed
	return nil
}

func init() {
	kreuzberg.RegisterValidator(
		"content-not-empty",
		50,
		(C.ValidatorCallback)(C.myValidator),
	)
}
```

---

### Custom OCR Backends

Register a custom OCR backend:

```go
//export customOCR
func customOCR(imageData *C.uint8_t, width C.uint32_t, height C.uint32_t, lang *C.char) *C.char {
	// Call your OCR library
	// Return JSON-encoded ExtractionResult
	result := kreuzberg.ExtractionResult{
		Content:  "extracted text from custom OCR",
		MimeType: "text/plain",
		Success:  true,
	}
	data, _ := json.Marshal(result)
	return C.CString(string(data))
}

func init() {
	kreuzberg.RegisterOCRBackend(
		"custom-ocr",
		(C.OcrBackendCallback)(C.customOCR),
	)
}
```

---

### Plugin Management

List and manage registered plugins:

```go
// List validators
validators, err := kreuzberg.ListValidators()
if err == nil {
	fmt.Printf("Validators: %v\n", validators)
}

// List post-processors
processors, err := kreuzberg.ListPostProcessors()
if err == nil {
	fmt.Printf("Post-processors: %v\n", processors)
}

// List OCR backends
backends, err := kreuzberg.ListOCRBackends()
if err == nil {
	fmt.Printf("OCR backends: %v\n", backends)
}

// Clear all validators
if err := kreuzberg.ClearValidators(); err != nil {
	log.Fatalf("failed to clear validators: %v\n", err)
}

// Unregister specific validator
if err := kreuzberg.UnregisterValidator("my-validator"); err != nil {
	log.Fatalf("failed to unregister: %v\n", err)
}
```

---

### Performance Tips

1. **Batch Processing**: Use `BatchExtractFilesSync()` for multiple files to leverage internal optimizations
2. **Context Timeouts**: Set realistic timeouts; OCR can be slow on large documents
3. **Caching**: Enable `UseCache: boolPtr(true)` to cache frequently extracted documents
4. **Library Paths**: Ensure `LD_LIBRARY_PATH`/`DYLD_FALLBACK_LIBRARY_PATH` is set before Go initialization
5. **Configuration Reuse**: Create and reuse ExtractionConfig objects across multiple calls
6. **Goroutines**: Use `ExtractFile()` / `ExtractBytes()` variants in goroutines for concurrency

---

## Troubleshooting

### Library Loading Errors

**Error:** `cannot open shared object file: No such file or directory`

**Solution:**

```bash
# Verify library exists
ls -la target/release/libkreuzberg_ffi.*

# Set library path
export LD_LIBRARY_PATH=$PWD/target/release:$LD_LIBRARY_PATH

# Test with ldd (Linux)
ldd target/release/libkreuzberg_ffi.so
```

---

### CGO Compilation Errors

**Error:** `error: kreuzberg.h: No such file or directory`

**Solution:**

Ensure kreuzberg-ffi is built before building your Go module:

```bash
cargo build -p kreuzberg-ffi --release
go build ./...
```

---

### Missing OCR Library

**Error:** `MissingDependencyError: Missing dependency: tesseract`

**Solution:**

Install Tesseract or use a different OCR backend:

```bash
# macOS
brew install tesseract

# Debian/Ubuntu
apt-get install tesseract-ocr

# Or use EasyOCR/PaddleOCR (Python packages)
```

---

### Context Timeout on Large Documents

**Issue:** Extraction times out before completion

**Solution:**

Increase timeout or disable OCR for large documents:

```go
ctx, cancel := context.WithTimeout(context.Background(), 5*time.Minute)
defer cancel()

cfg := &kreuzberg.ExtractionConfig{
	ForceOCR: boolPtr(false), // Disable OCR if not needed
}

result, err := kreuzberg.ExtractFile(ctx, "large.pdf", cfg)
```

---

## Testing

Run the test suite:

```bash
# Unit tests (from packages/go)
task go:test

# Lint (gofmt + golangci-lint)
task go:lint

# E2E tests (from e2e/go, auto-generated from fixtures)
task e2e:go:verify

# Manual test with library path
export LD_LIBRARY_PATH=$PWD/target/release:$LD_LIBRARY_PATH
go test -v ./packages/go/kreuzberg
```

---

## Helper Functions

Add these utility functions to your code:

```go
func stringPtr(s string) *string {
	return &s
}

func boolPtr(b bool) *bool {
	return &b
}

func intPtr(i int) *int {
	return &i
}

func float64Ptr(f float64) *float64 {
	return &f
}

func uint32Ptr(u uint32) *uint32 {
	return &u
}
```

---

## Related Resources

- **Source:** `packages/go/kreuzberg/` (Go binding implementation)
- **FFI Bridge:** `crates/kreuzberg-ffi/` (C FFI layer)
- **Rust Core:** `crates/kreuzberg/` (extraction logic)
- **E2E Tests:** `e2e/go/` (auto-generated test fixtures)
- **CI:** `.github/workflows/go-test.yml` (test pipeline)
