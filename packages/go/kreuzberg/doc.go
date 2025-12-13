// Package kreuzberg provides a high-performance document intelligence library for Go
// backed by the Rust core that powers every Kreuzberg binding.
//
// Kreuzberg is a polyglot document intelligence library that extracts content,
// metadata, and structure from a wide range of document formats including PDFs,
// Office documents, emails, images, archives, and more. The Go binding uses cgo
// to efficiently wrap the kreuzberg-ffi C library, which is built from the Rust core.
//
// # Key Features
//
// - Unified API for 50+ document formats (PDF, PPTX, DOCX, XLSX, EML, images, etc.)
// - Automatic format detection via MIME type
// - Metadata extraction (author, dates, language, keywords, custom properties, etc.)
// - Table detection and structured extraction
// - Image extraction with optional OCR via Tesseract, EasyOCR, or PaddleOCR
// - Language detection (80+ languages)
// - Document chunking with optional embeddings
// - Batch processing for multiple files
// - Plugin architecture for custom validators and post-processors
// - Full context support for timeout/cancellation (best-effort)
// - Caching with optional Redis/Memcached backends
// - Zero unsafe code in Go bindings; memory safety delegated to Rust
//
// # Installation
//
// 1. Fetch the Go module:
//
//	go get github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg@latest
//
// 2. Build the native library (kreuzberg-ffi) for your platform:
//
//	cargo build -p kreuzberg-ffi --release
//
// 3. Make the library discoverable at runtime:
//
// On macOS:
//
//	export DYLD_FALLBACK_LIBRARY_PATH=$PWD/target/release
//
// On Linux:
//
//	export LD_LIBRARY_PATH=$PWD/target/release
//
// On Windows:
//
//	set PATH=%cd%\target\release;%PATH%
//
// Pdfium is bundled in target/release, so no extra system packages are required
// unless you customize the build.
//
// # Quick Start
//
// Extract text and metadata from a PDF:
//
//	result, err := kreuzberg.ExtractFileSync("report.pdf", nil)
//	if err != nil {
//		log.Fatal(err)
//	}
//
//	fmt.Println("Content:", result.Content)
//	fmt.Println("MIME Type:", result.MimeType)
//	fmt.Println("Format:", result.Metadata.FormatType())
//
// If the file is a PDF, access PDF-specific metadata:
//
//	if pdf, ok := result.Metadata.PdfMetadata(); ok {
//		fmt.Println("Title:", *pdf.Title)
//		fmt.Println("Pages:", *pdf.PageCount)
//	}
//
// # Configuration
//
// Advanced extraction settings are passed via ExtractionConfig:
//
//	lang := "eng"
//	cfg := &kreuzberg.ExtractionConfig{
//		UseCache:        true,
//		ForceOCR:        false,
//		ImageExtraction: &kreuzberg.ImageExtractionConfig{Enabled: true},
//		OCR: &kreuzberg.OcrConfig{
//			Backend:   "tesseract",
//			Language:  &lang,
//			PageRange: nil,
//		},
//		Chunking: &kreuzberg.ChunkingConfig{
//			Enabled:    true,
//			ChunkSize:  1024,
//			Overlap:    100,
//		},
//	}
//	result, err := kreuzberg.ExtractFileSync("scanned.pdf", cfg)
//	if err != nil {
//		log.Fatal(err)
//	}
//
// # Batch Processing
//
// Process multiple files efficiently:
//
//	paths := []string{"doc1.pdf", "doc2.docx", "report.xlsx"}
//	results, err := kreuzberg.BatchExtractFilesSync(paths, nil)
//	if err != nil {
//		log.Fatal(err)
//	}
//	for i, res := range results {
//		if res == nil {
//			continue
//		}
//		fmt.Printf("[%d] %s => %d bytes\n", i, res.MimeType, len(res.Content))
//	}
//
// # Concurrency and Goroutines
//
// All extraction functions are synchronous and block until completion.
// For concurrent processing, spawn goroutines manually:
//
//	var wg sync.WaitGroup
//	results := make([]*kreuzberg.ExtractionResult, len(files))
//
//	for i, path := range files {
//		wg.Add(1)
//		go func(idx int, p string) {
//			defer wg.Done()
//			result, err := kreuzberg.ExtractFileSync(p, nil)
//			if err != nil {
//				log.Printf("Error extracting %s: %v", p, err)
//				return
//			}
//			results[idx] = result
//		}(i, path)
//	}
//	wg.Wait()
//
// Note: Extraction operations cannot be canceled once started. If you need
// timeouts, implement them at the application level (e.g., using channels
// with time.After or dedicated timeout goroutines).
//
// # Error Handling
//
// Kreuzberg errors are domain-specific and can be type-asserted for precise handling:
//
//	result, err := kreuzberg.ExtractFileSync("doc.pdf", nil)
//	if err != nil {
//		var missDepErr *kreuzberg.MissingDependencyError
//		var valErr *kreuzberg.ValidationError
//		var parseErr *kreuzberg.ParsingError
//
//		if errors.As(err, &missDepErr) {
//			// Install missing OCR backend, e.g., tesseract
//			log.Println("Missing:", missDepErr.Message)
//		} else if errors.As(err, &valErr) {
//			log.Println("Validation failed:", valErr.Message)
//		} else if errors.As(err, &parseErr) {
//			log.Println("Parsing failed:", parseErr.Message)
//		} else {
//			log.Fatal(err)
//		}
//	}
//
// # Metadata Types
//
// Each document format supports format-specific metadata. Use the FormatType() method
// to discriminate:
//
//	formatType := result.Metadata.FormatType()
//	switch formatType {
//	case kreuzberg.FormatPDF:
//		pdf, _ := result.Metadata.PdfMetadata()
//		// Access pdf.Title, pdf.PageCount, etc.
//	case kreuzberg.FormatExcel:
//		excel, _ := result.Metadata.ExcelMetadata()
//		// Access excel.SheetNames, excel.SheetCount
//	case kreuzberg.FormatEmail:
//		email, _ := result.Metadata.EmailMetadata()
//		// Access email.FromEmail, email.ToEmails, etc.
//	case kreuzberg.FormatImage:
//		img, _ := result.Metadata.ImageMetadata()
//		// Access img.Width, img.Height, img.Format, img.EXIF
//	}
//
// # Plugin System
//
// Register custom validators to validate or transform extraction results.
// Validators must be exported C functions decorated with //export:
//
//	//export customValidator
//	func customValidator(resultJSON *C.char) *C.char {
//		// Parse resultJSON, validate, and return NULL if ok or error string if invalid
//		return nil
//	}
//
//	func init() {
//		if err := kreuzberg.RegisterValidator("my-validator", 50, (C.ValidatorCallback)(C.customValidator)); err != nil {
//			log.Fatalf("validator registration failed: %v", err)
//		}
//	}
//
// Validators are invoked after extraction and can modify the result payload.
// Priority controls execution order (higher = runs first).
//
// # Chunking and Embeddings
//
// Extract documents in semantic chunks with optional embeddings:
//
//	cfg := &kreuzberg.ExtractionConfig{
//		Chunking: &kreuzberg.ChunkingConfig{
//			Enabled:    true,
//			ChunkSize:  512,
//			Overlap:    50,
//		},
//	}
//	result, err := kreuzberg.ExtractFileSync("doc.pdf", cfg)
//	if err != nil {
//		log.Fatal(err)
//	}
//	for i, chunk := range result.Chunks {
//		fmt.Printf("Chunk %d: %s\n", i, chunk.Content)
//		if chunk.Embedding != nil {
//			fmt.Printf("  Embedding dims: %d\n", len(chunk.Embedding))
//		}
//	}
//
// # Images and OCR
//
// Extract images and apply OCR (e.g., for scanned PDFs):
//
//	cfg := &kreuzberg.ExtractionConfig{
//		ImageExtraction: &kreuzberg.ImageExtractionConfig{Enabled: true},
//		OCR: &kreuzberg.OcrConfig{
//			Backend: "tesseract",
//		},
//	}
//	result, err := kreuzberg.ExtractFileSync("scanned.pdf", cfg)
//	if err != nil {
//		log.Fatal(err)
//	}
//	for _, img := range result.Images {
//		fmt.Printf("Image %d (%s): %dx%d\n", img.ImageIndex, img.Format, *img.Width, *img.Height)
//		if img.OCRResult != nil {
//			fmt.Printf("  OCR text: %s\n", img.OCRResult.Content)
//		}
//	}
//
// # Supported Formats
//
// Kreuzberg supports 50+ formats across multiple categories:
//
//	Documents: PDF, DOCX, DOC, DOCM, ODT, RTF, PPTX, PPT, POTX, XLSX, XLS, CSV, TSV, ODS
//	Web: HTML, XML, JSON, YAML, TOML, INI
//	Email: EML, MSG
//	Archives: ZIP, RAR, 7Z, GZ, BZ2, TAR
//	Images: PNG, JPG, GIF, WEBP, TIFF, BMP, ICO, SVG
//	Other: Plain text, Markdown, PostScript, DWF, etc.
//
// # Logging and Debugging
//
// The Rust core uses structured logging. Control verbosity via environment variables:
//
//	RUST_LOG=kreuzberg=debug go test ./...
//
// # Thread Safety
//
// All Kreuzberg API functions are thread-safe. The underlying Rust core and FFI
// layer handle synchronization internally.
//
// # Performance Considerations
//
// - Use batch APIs (BatchExtractFilesSync, BatchExtractBytesSync) for multiple documents
// - Enable caching (UseCache: true) for repeated extractors on the same file
// - For I/O-bound workloads, spawn goroutines and use async variants with context
// - Large files benefit from streaming extraction and chunking
// - OCR is CPU-intensive; consider dedicated worker pools for high throughput
//
// # Resources
//
// - Documentation: https://kreuzberg.dev
// - GitHub: https://github.com/kreuzberg-dev/kreuzberg
// - Issue tracker: https://github.com/kreuzberg-dev/kreuzberg/issues
// - Discord: https://discord.gg/pXxagNK2zN
//
// # Troubleshooting
//
// Library not found (macOS):
//
//	runtime/cgo: dlopen(/libkreuzberg_ffi.dylib, 0x0001): image not found
//
// Solution: Set DYLD_FALLBACK_LIBRARY_PATH to point at target/release.
//
// Missing OCR backend:
//
//	MissingDependencyError: tesseract backend not available
//
// Solution: Install the OCR backend (e.g., `brew install tesseract` on macOS,
// `apt-get install tesseract-ocr` on Linux) and ensure it is on PATH.
//
// Build fails with "undefined: C.customValidator":
//
// Solution: Export the callback function in a *_cgo.go file before using it
// in RegisterValidator/RegisterPostProcessor.
//
// Tests fail with "library not found":
//
// Solution: Set LD_LIBRARY_PATH (Linux) or DYLD_FALLBACK_LIBRARY_PATH (macOS)
// before running `go test ./...`:
//
//	LD_LIBRARY_PATH=$PWD/target/release go test ./...
//
// # FFI Architecture
//
// The Go binding is a thin wrapper around the kreuzberg-ffi C library:
//
//	Go SDK → cgo wrapper → kreuzberg-ffi (C) → Rust core
//
// Core extraction logic lives in the Rust kreuzberg crate. The C FFI layer
// (kreuzberg-ffi) exposes a minimal API, and the Go binding provides idiomatic
// types, error handling, and utilities. This design ensures consistency across
// all Kreuzberg bindings (Python, TypeScript, Ruby, Java, Go) while maximizing
// performance and code reuse.
//
// # Version
//
// This binding targets Kreuzberg 4.0.0-rc.7 (https://github.com/kreuzberg-dev/kreuzberg).
package kreuzberg
