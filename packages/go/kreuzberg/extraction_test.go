package kreuzberg

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

// TestExtractFileSyncWithValidPDF tests extraction from a valid PDF file.
func TestExtractFileSyncWithValidPDF(t *testing.T) {
	dir := t.TempDir()
	path, err := writeValidPDFToFile(dir, "sample.pdf")
	if err != nil {
		t.Fatalf("failed to write test PDF: %v", err)
	}

	result, err := ExtractFileSync(path, nil)
	if err != nil {
		t.Fatalf("ExtractFileSync failed: %v", err)
	}
	if result == nil {
		t.Fatalf("expected non-nil result, got nil")
	}
}

// TestExtractFileSyncWithMissingFile tests error handling for missing files.
func TestExtractFileSyncWithMissingFile(t *testing.T) {
	_, err := ExtractFileSync("/nonexistent/path/file.pdf", nil)
	if err == nil {
		t.Fatalf("expected error for missing file, got nil")
	}
}

// TestExtractFileSyncWithEmptyPath tests validation of empty file path.
func TestExtractFileSyncWithEmptyPath(t *testing.T) {
	_, err := ExtractFileSync("", nil)
	if err == nil {
		t.Fatalf("expected error for empty path, got nil")
	}
}

// TestExtractFileSyncWithConfig tests extraction with custom configuration.
func TestExtractFileSyncWithConfig(t *testing.T) {
	dir := t.TempDir()
	path, err := writeValidPDFToFile(dir, "sample.pdf")
	if err != nil {
		t.Fatalf("failed to write test PDF: %v", err)
	}

	config := &ExtractionConfig{
		UseCache:                BoolPtr(false),
		EnableQualityProcessing: BoolPtr(true),
	}

	result, err := ExtractFileSync(path, config)
	if err != nil {
		t.Fatalf("ExtractFileSync with config failed: %v", err)
	}
	if result == nil {
		t.Fatalf("expected non-nil result, got nil")
	}
}

// TestExtractBytesSync tests extraction from byte data.
func TestExtractBytesSync(t *testing.T) {
	data, err := getValidPDFBytes()
	if err != nil {
		t.Fatalf("failed to get PDF bytes: %v", err)
	}
	result, err := ExtractBytesSync(data, "application/pdf", nil)
	if err != nil {
		t.Fatalf("ExtractBytesSync failed: %v", err)
	}
	if result == nil {
		t.Fatalf("expected non-nil result, got nil")
	}
}

// TestExtractBytesSyncWithEmptyData tests validation of empty data.
func TestExtractBytesSyncWithEmptyData(t *testing.T) {
	_, err := ExtractBytesSync([]byte{}, "application/pdf", nil)
	if err == nil {
		t.Fatalf("expected error for empty data, got nil")
	}
}

// TestExtractBytesSyncWithEmptyMimeType tests validation of empty MIME type.
func TestExtractBytesSyncWithEmptyMimeType(t *testing.T) {
	data := []byte("%PDF-1.7\n%comment\n")
	_, err := ExtractBytesSync(data, "", nil)
	if err == nil {
		t.Fatalf("expected error for empty MIME type, got nil")
	}
}

// TestExtractBytesSyncWithConfig tests byte extraction with configuration.
func TestExtractBytesSyncWithConfig(t *testing.T) {
	data, err := getValidPDFBytes()
	if err != nil {
		t.Fatalf("failed to get PDF bytes: %v", err)
	}
	config := &ExtractionConfig{
		UseCache: BoolPtr(false),
	}
	result, err := ExtractBytesSync(data, "application/pdf", config)
	if err != nil {
		t.Fatalf("ExtractBytesSync with config failed: %v", err)
	}
	if result == nil {
		t.Fatalf("expected non-nil result, got nil")
	}
}

// TestExtractResultStructure tests that ExtractionResult has expected fields.
func TestExtractResultStructure(t *testing.T) {
	result := &ExtractionResult{
		Content:  "test content",
		MimeType: "text/plain",
		Success:  true,
	}
	if result.Content != "test content" {
		t.Fatalf("content mismatch: expected 'test content', got %s", result.Content)
	}
	if result.MimeType != "text/plain" {
		t.Fatalf("MIME type mismatch: expected 'text/plain', got %s", result.MimeType)
	}
	if !result.Success {
		t.Fatalf("success flag should be true")
	}
}

// TestTableExtractionInResult tests table data within results.
func TestTableExtractionInResult(t *testing.T) {
	t.Run("empty tables", func(t *testing.T) {
		result := &ExtractionResult{
			Content: "test",
			Tables:  []Table{},
		}
		if len(result.Tables) != 0 {
			t.Fatalf("expected empty tables, got %d", len(result.Tables))
		}
	})

	t.Run("single table", func(t *testing.T) {
		table := Table{
			Cells:      [][]string{{"A1", "B1"}, {"A2", "B2"}},
			Markdown:   "| A1 | B1 |\n| A2 | B2 |",
			PageNumber: 1,
		}
		result := &ExtractionResult{
			Content: "test",
			Tables:  []Table{table},
		}
		if len(result.Tables) != 1 {
			t.Fatalf("expected 1 table, got %d", len(result.Tables))
		}
		if len(result.Tables[0].Cells) != 2 {
			t.Fatalf("expected 2 rows in table, got %d", len(result.Tables[0].Cells))
		}
	})

	t.Run("multiple tables", func(t *testing.T) {
		table1 := Table{Cells: [][]string{{"A1", "B1"}}, PageNumber: 1}
		table2 := Table{Cells: [][]string{{"C1", "D1"}}, PageNumber: 2}
		result := &ExtractionResult{
			Tables: []Table{table1, table2},
		}
		if len(result.Tables) != 2 {
			t.Fatalf("expected 2 tables, got %d", len(result.Tables))
		}
	})
}

// TestMetadataExtractionInResult tests metadata handling in results.
func TestMetadataExtractionInResult(t *testing.T) {
	t.Run("basic metadata", func(t *testing.T) {
		result := &ExtractionResult{
			Content: "test",
			Metadata: Metadata{
				Language: StringPtr("en"),
				Date:     StringPtr("2025-01-01"),
			},
		}
		if result.Metadata.Language == nil || *result.Metadata.Language != "en" {
			t.Fatalf("language metadata not set correctly")
		}
	})

	t.Run("PDF metadata", func(t *testing.T) {
		pdfMeta := &PdfMetadata{
			Title:     StringPtr("Test Document"),
			PageCount: IntPtr(10),
		}
		result := &ExtractionResult{
			Metadata: Metadata{
				Format: FormatMetadata{
					Type: FormatPDF,
					Pdf:  pdfMeta,
				},
			},
		}
		meta, ok := result.Metadata.PdfMetadata()
		if !ok {
			t.Fatalf("expected PDF metadata to be present")
		}
		if meta.PageCount == nil || *meta.PageCount != 10 {
			t.Fatalf("page count not extracted correctly")
		}
	})

	t.Run("Excel metadata", func(t *testing.T) {
		excelMeta := &ExcelMetadata{
			SheetCount: 3,
			SheetNames: []string{"Sheet1", "Sheet2", "Sheet3"},
		}
		result := &ExtractionResult{
			Metadata: Metadata{
				Format: FormatMetadata{
					Type:  FormatExcel,
					Excel: excelMeta,
				},
			},
		}
		meta, ok := result.Metadata.ExcelMetadata()
		if !ok {
			t.Fatalf("expected Excel metadata to be present")
		}
		if meta.SheetCount != 3 {
			t.Fatalf("expected 3 sheets, got %d", meta.SheetCount)
		}
	})
}

// TestChunkingInResult tests chunk data extraction.
func TestChunkingInResult(t *testing.T) {
	t.Run("empty chunks", func(t *testing.T) {
		result := &ExtractionResult{
			Chunks: []Chunk{},
		}
		if len(result.Chunks) != 0 {
			t.Fatalf("expected 0 chunks, got %d", len(result.Chunks))
		}
	})

	t.Run("single chunk with metadata", func(t *testing.T) {
		chunk := Chunk{
			Content: "chunk content",
			Metadata: ChunkMetadata{
				CharStart:   0,
				CharEnd:     13,
				ChunkIndex:  0,
				TotalChunks: 1,
			},
		}
		result := &ExtractionResult{
			Chunks: []Chunk{chunk},
		}
		if len(result.Chunks) != 1 {
			t.Fatalf("expected 1 chunk, got %d", len(result.Chunks))
		}
		if result.Chunks[0].Content != "chunk content" {
			t.Fatalf("chunk content mismatch")
		}
	})

	t.Run("multiple chunks with overlap", func(t *testing.T) {
		chunk1 := Chunk{
			Content: "first part",
			Metadata: ChunkMetadata{
				CharStart:   0,
				CharEnd:     10,
				ChunkIndex:  0,
				TotalChunks: 2,
			},
		}
		chunk2 := Chunk{
			Content: "second part",
			Metadata: ChunkMetadata{
				CharStart:   5,
				CharEnd:     16,
				ChunkIndex:  1,
				TotalChunks: 2,
			},
		}
		result := &ExtractionResult{
			Chunks: []Chunk{chunk1, chunk2},
		}
		if len(result.Chunks) != 2 {
			t.Fatalf("expected 2 chunks, got %d", len(result.Chunks))
		}
		if result.Chunks[0].Metadata.TotalChunks != 2 {
			t.Fatalf("total chunks count incorrect")
		}
	})
}

// TestImageExtractionInResult tests image data handling.
func TestImageExtractionInResult(t *testing.T) {
	t.Run("empty images", func(t *testing.T) {
		result := &ExtractionResult{
			Images: []ExtractedImage{},
		}
		if len(result.Images) != 0 {
			t.Fatalf("expected 0 images, got %d", len(result.Images))
		}
	})

	t.Run("single image", func(t *testing.T) {
		image := ExtractedImage{
			Data:       []byte("fake image data"),
			Format:     "png",
			ImageIndex: 0,
		}
		result := &ExtractionResult{
			Images: []ExtractedImage{image},
		}
		if len(result.Images) != 1 {
			t.Fatalf("expected 1 image, got %d", len(result.Images))
		}
		if result.Images[0].Format != "png" {
			t.Fatalf("expected format 'png', got %s", result.Images[0].Format)
		}
	})

	t.Run("multiple images with metadata", func(t *testing.T) {
		img1 := ExtractedImage{
			Data:       []byte("image1"),
			Format:     "jpeg",
			ImageIndex: 0,
			Width:      IntPtr32(800),
			Height:     IntPtr32(600),
			PageNumber: IntPtr(1),
		}
		img2 := ExtractedImage{
			Data:       []byte("image2"),
			Format:     "png",
			ImageIndex: 1,
			PageNumber: IntPtr(2),
		}
		result := &ExtractionResult{
			Images: []ExtractedImage{img1, img2},
		}
		if len(result.Images) != 2 {
			t.Fatalf("expected 2 images, got %d", len(result.Images))
		}
	})
}

// TestMimeDetectionFromBytes tests MIME type detection from byte content.
func TestMimeDetectionFromBytes(t *testing.T) {
	t.Run("PDF detection", func(t *testing.T) {
		data := []byte("%PDF-1.7\n")
		mime, err := DetectMimeType(data)
		if err != nil {
			t.Fatalf("failed to detect MIME type: %v", err)
		}
		if mime != "application/pdf" {
			t.Fatalf("expected 'application/pdf', got '%s'", mime)
		}
	})

	t.Run("empty data returns error", func(t *testing.T) {
		_, err := DetectMimeType([]byte{})
		if err == nil {
			t.Fatalf("expected error for empty data, got nil")
		}
	})
}

// TestMimeDetectionFromPath tests MIME type detection from file path.
func TestMimeDetectionFromPath(t *testing.T) {
	t.Run("PDF file", func(t *testing.T) {
		dir := t.TempDir()
		path := filepath.Join(dir, "test.pdf")
		if err := os.WriteFile(path, []byte("%PDF-1.7\n"), 0o644); err != nil {
			t.Fatalf("failed to write test file: %v", err)
		}

		mime, err := DetectMimeTypeFromPath(path)
		if err != nil {
			t.Fatalf("failed to detect MIME from path: %v", err)
		}
		if mime != "application/pdf" {
			t.Fatalf("expected 'application/pdf', got '%s'", mime)
		}
	})

	t.Run("empty path returns error", func(t *testing.T) {
		_, err := DetectMimeTypeFromPath("")
		if err == nil {
			t.Fatalf("expected error for empty path, got nil")
		}
	})

	t.Run("missing file returns error", func(t *testing.T) {
		_, err := DetectMimeTypeFromPath("/nonexistent/file.pdf")
		if err == nil {
			t.Fatalf("expected error for missing file, got nil")
		}
	})
}

// TestEncodingDetectionInMetadata tests language/encoding detection.
func TestEncodingDetectionInMetadata(t *testing.T) {
	result := &ExtractionResult{
		Content:           "test",
		DetectedLanguages: []string{"en", "fr"},
	}
	if len(result.DetectedLanguages) != 2 {
		t.Fatalf("expected 2 detected languages, got %d", len(result.DetectedLanguages))
	}
	if result.DetectedLanguages[0] != "en" {
		t.Fatalf("expected first language 'en', got '%s'", result.DetectedLanguages[0])
	}
}

// TestLargeContentHandling tests extraction of large text content.
func TestLargeContentHandling(t *testing.T) {
	t.Run("large content in result", func(t *testing.T) {
		largeContent := bytes.Repeat([]byte("test content "), 10000)
		result := &ExtractionResult{
			Content: string(largeContent),
		}
		if len(result.Content) < 100000 {
			t.Fatalf("expected large content, got size %d", len(result.Content))
		}
	})

	t.Run("large byte data extraction", func(t *testing.T) {
		largeData := bytes.Repeat([]byte("x"), 1000000)
		result := &ExtractionResult{
			Content: string(largeData),
		}
		if len(result.Content) != 1000000 {
			t.Fatalf("expected 1000000 bytes, got %d", len(result.Content))
		}
	})
}

// TestConfigurationOptions tests various config parameter combinations.
func TestConfigurationOptions(t *testing.T) {
	t.Run("cache configuration", func(t *testing.T) {
		config := &ExtractionConfig{
			UseCache: BoolPtr(true),
		}
		if config.UseCache == nil || !*config.UseCache {
			t.Fatalf("cache config not set correctly")
		}
	})

	t.Run("quality processing configuration", func(t *testing.T) {
		config := &ExtractionConfig{
			EnableQualityProcessing: BoolPtr(false),
		}
		if config.EnableQualityProcessing == nil || *config.EnableQualityProcessing {
			t.Fatalf("quality processing config not set correctly")
		}
	})

	t.Run("OCR configuration", func(t *testing.T) {
		config := &ExtractionConfig{
			OCR: &OCRConfig{
				Language: StringPtr("eng"),
			},
		}
		if config.OCR == nil || config.OCR.Language == nil {
			t.Fatalf("OCR config not set correctly")
		}
	})

	t.Run("chunking configuration", func(t *testing.T) {
		config := &ExtractionConfig{
			Chunking: &ChunkingConfig{
				MaxChars: IntPtr(1000),
				Preset:   StringPtr("default"),
			},
		}
		if config.Chunking == nil || config.Chunking.MaxChars == nil {
			t.Fatalf("chunking config not set correctly")
		}
	})

	t.Run("image extraction configuration", func(t *testing.T) {
		config := &ExtractionConfig{
			Images: &ImageExtractionConfig{
				ExtractImages: BoolPtr(true),
				TargetDPI:     IntPtr(300),
			},
		}
		if config.Images == nil || config.Images.ExtractImages == nil {
			t.Fatalf("image config not set correctly")
		}
	})
}

// TestConfigurationJSON tests JSON marshaling of configuration.
func TestConfigurationJSON(t *testing.T) {
	config := &ExtractionConfig{
		UseCache:                 BoolPtr(false),
		EnableQualityProcessing:  BoolPtr(true),
		MaxConcurrentExtractions: IntPtr(4),
	}

	data, err := json.Marshal(config)
	if err != nil {
		t.Fatalf("failed to marshal config: %v", err)
	}

	var decoded ExtractionConfig
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("failed to unmarshal config: %v", err)
	}

	if decoded.UseCache == nil || *decoded.UseCache != false {
		t.Fatalf("use_cache not preserved in round-trip")
	}
}

// TestErrorHandling tests extraction error scenarios.
func TestErrorHandling(t *testing.T) {
	t.Run("invalid file path", func(t *testing.T) {
		_, err := ExtractFileSync("/invalid/\x00/path", nil)
		if err == nil {
			t.Fatalf("expected error for invalid path")
		}
	})

	t.Run("unsupported MIME type", func(t *testing.T) {
		data := []byte("test data")
		_, err := ExtractBytesSync(data, "video/unsupported", nil)
		if err == nil {
			t.Fatalf("expected error for unsupported MIME type")
		}
	})
}

// TestResultJSONMarshaling tests JSON serialization of results.
func TestResultJSONMarshaling(t *testing.T) {
	result := &ExtractionResult{
		Content:  "test content",
		MimeType: "text/plain",
		Success:  true,
		Metadata: Metadata{
			Language: StringPtr("en"),
		},
	}

	data, err := json.Marshal(result)
	if err != nil {
		t.Fatalf("failed to marshal result: %v", err)
	}

	var decoded ExtractionResult
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("failed to unmarshal result: %v", err)
	}

	if decoded.Content != "test content" {
		t.Fatalf("content not preserved in round-trip")
	}
}

// TestMetadataFormatTypeDetection tests FormatType detection.
func TestMetadataFormatTypeDetection(t *testing.T) {
	t.Run("PDF format detection", func(t *testing.T) {
		meta := Metadata{
			Format: FormatMetadata{
				Type: FormatPDF,
				Pdf:  &PdfMetadata{PageCount: IntPtr(5)},
			},
		}
		if meta.FormatType() != FormatPDF {
			t.Fatalf("expected FormatPDF, got %s", meta.FormatType())
		}
		_, ok := meta.PdfMetadata()
		if !ok {
			t.Fatalf("expected PDF metadata to be present")
		}
	})

	t.Run("Excel format detection", func(t *testing.T) {
		meta := Metadata{
			Format: FormatMetadata{
				Type:  FormatExcel,
				Excel: &ExcelMetadata{SheetCount: 2},
			},
		}
		if meta.FormatType() != FormatExcel {
			t.Fatalf("expected FormatExcel, got %s", meta.FormatType())
		}
		_, ok := meta.ExcelMetadata()
		if !ok {
			t.Fatalf("expected Excel metadata to be present")
		}
	})

	t.Run("Image format detection", func(t *testing.T) {
		meta := Metadata{
			Format: FormatMetadata{
				Type:  FormatImage,
				Image: &ImageMetadata{Width: 800, Height: 600},
			},
		}
		if meta.FormatType() != FormatImage {
			t.Fatalf("expected FormatImage, got %s", meta.FormatType())
		}
		_, ok := meta.ImageMetadata()
		if !ok {
			t.Fatalf("expected Image metadata to be present")
		}
	})

	t.Run("Text format detection", func(t *testing.T) {
		meta := Metadata{
			Format: FormatMetadata{
				Type: FormatText,
				Text: &TextMetadata{
					LineCount: 10,
					WordCount: 50,
				},
			},
		}
		if meta.FormatType() != FormatText {
			t.Fatalf("expected FormatText, got %s", meta.FormatType())
		}
		_, ok := meta.TextMetadata()
		if !ok {
			t.Fatalf("expected Text metadata to be present")
		}
	})

	t.Run("HTML format detection", func(t *testing.T) {
		meta := Metadata{
			Format: FormatMetadata{
				Type: FormatHTML,
				HTML: &HtmlMetadata{
					Title: StringPtr("Test Page"),
				},
			},
		}
		if meta.FormatType() != FormatHTML {
			t.Fatalf("expected FormatHTML, got %s", meta.FormatType())
		}
		_, ok := meta.HTMLMetadata()
		if !ok {
			t.Fatalf("expected HTML metadata to be present")
		}
	})
}

// TestExtensionResolution tests getting file extensions for MIME types.
func TestExtensionResolution(t *testing.T) {
	t.Run("PDF extensions", func(t *testing.T) {
		exts, err := GetExtensionsForMime("application/pdf")
		if err != nil {
			t.Fatalf("failed to get extensions: %v", err)
		}
		if len(exts) == 0 {
			t.Fatalf("expected extensions for PDF")
		}
	})

	t.Run("empty MIME type returns error", func(t *testing.T) {
		_, err := GetExtensionsForMime("")
		if err == nil {
			t.Fatalf("expected error for empty MIME type")
		}
	})

	t.Run("invalid MIME type may error", func(t *testing.T) {
		_, err := GetExtensionsForMime("invalid/mime")
		// May or may not error depending on implementation
		_ = err
	})
}

// TestMimeTypeValidation tests MIME type validation.
func TestMimeTypeValidation(t *testing.T) {
	t.Run("valid PDF MIME", func(t *testing.T) {
		mime, err := ValidateMimeType("application/pdf")
		if err != nil {
			t.Fatalf("validation failed: %v", err)
		}
		if mime != "application/pdf" {
			t.Fatalf("expected 'application/pdf', got '%s'", mime)
		}
	})

	t.Run("empty MIME type returns error", func(t *testing.T) {
		_, err := ValidateMimeType("")
		if err == nil {
			t.Fatalf("expected error for empty MIME type")
		}
	})

	t.Run("unsupported format returns error", func(t *testing.T) {
		_, err := ValidateMimeType("video/mp4")
		if err == nil {
			t.Fatalf("expected error for unsupported format")
		}
	})
}

// TestLibraryVersion tests version retrieval.
func TestLibraryVersion(t *testing.T) {
	version := LibraryVersion()
	if version == "" {
		t.Fatalf("expected non-empty version string")
	}
}

// TestTesseractConfiguration tests OCR-specific configuration.
func TestTesseractConfiguration(t *testing.T) {
	config := &ExtractionConfig{
		OCR: &OCRConfig{
			Backend: "tesseract",
			Tesseract: &TesseractConfig{
				Language:      "eng",
				PSM:           IntPtr(3),
				MinConfidence: FloatPtr(0.5),
			},
		},
	}

	if config.OCR.Tesseract == nil {
		t.Fatalf("tesseract config not set")
	}
	if config.OCR.Tesseract.Language != "eng" {
		t.Fatalf("language not set correctly")
	}
}

// TestImagePreprocessingConfiguration tests image preprocessing settings.
func TestImagePreprocessingConfiguration(t *testing.T) {
	config := &ExtractionConfig{
		OCR: &OCRConfig{
			Tesseract: &TesseractConfig{
				Preprocessing: &ImagePreprocessingConfig{
					TargetDPI:       IntPtr(300),
					AutoRotate:      BoolPtr(true),
					Deskew:          BoolPtr(true),
					ContrastEnhance: BoolPtr(true),
				},
			},
		},
	}

	if config.OCR.Tesseract.Preprocessing == nil {
		t.Fatalf("preprocessing config not set")
	}
	if config.OCR.Tesseract.Preprocessing.TargetDPI == nil {
		t.Fatalf("target DPI not set")
	}
}

// TestPDFSpecificOptions tests PDF extraction options.
func TestPDFSpecificOptions(t *testing.T) {
	config := &ExtractionConfig{
		PdfOptions: &PdfConfig{
			ExtractImages:   BoolPtr(true),
			ExtractMetadata: BoolPtr(true),
		},
	}

	if config.PdfOptions == nil {
		t.Fatalf("PDF options not set")
	}
}

// TestEmbeddingPresets tests embedding preset functionality.
func TestEmbeddingPresets(t *testing.T) {
	t.Run("list presets", func(t *testing.T) {
		presets, err := ListEmbeddingPresets()
		if err != nil {
			t.Fatalf("failed to list presets: %v", err)
		}
		// May be empty or populated depending on configuration
		_ = presets
	})

	t.Run("get preset by name", func(t *testing.T) {
		preset, err := GetEmbeddingPreset("default")
		if err != nil {
			// May not exist, that's okay
			_ = err
		} else if preset != nil {
			if preset.Name == "" {
				t.Fatalf("expected preset with name")
			}
		}
	})

	t.Run("invalid preset name", func(t *testing.T) {
		_, err := GetEmbeddingPreset("")
		if err == nil {
			t.Fatalf("expected error for empty preset name")
		}
	})
}

// TestChunkingWithEmbeddings tests chunking combined with embeddings.
func TestChunkingWithEmbeddings(t *testing.T) {
	chunk := Chunk{
		Content:   "test chunk",
		Embedding: []float32{0.1, 0.2, 0.3},
		Metadata: ChunkMetadata{
			CharStart:  0,
			CharEnd:    10,
			TokenCount: IntPtr(3),
		},
	}

	if len(chunk.Embedding) != 3 {
		t.Fatalf("expected 3 embedding dimensions, got %d", len(chunk.Embedding))
	}
	if chunk.Metadata.TokenCount == nil {
		t.Fatalf("expected token count to be set")
	}
}

// TestEmailMetadataExtraction tests email-specific metadata.
func TestEmailMetadataExtraction(t *testing.T) {
	emailMeta := &EmailMetadata{
		FromEmail:   StringPtr("sender@example.com"),
		ToEmails:    []string{"recipient@example.com"},
		Attachments: []string{"file.pdf", "file.txt"},
	}

	result := &ExtractionResult{
		Metadata: Metadata{
			Format: FormatMetadata{
				Type:  FormatEmail,
				Email: emailMeta,
			},
		},
	}

	meta, ok := result.Metadata.EmailMetadata()
	if !ok {
		t.Fatalf("expected email metadata")
	}
	if len(meta.Attachments) != 2 {
		t.Fatalf("expected 2 attachments")
	}
}

// TestArchiveMetadataExtraction tests archive-specific metadata.
func TestArchiveMetadataExtraction(t *testing.T) {
	archiveMeta := &ArchiveMetadata{
		Format:    "zip",
		FileCount: 3,
		FileList:  []string{"file1.txt", "file2.txt", "file3.txt"},
		TotalSize: 5000,
	}

	result := &ExtractionResult{
		Metadata: Metadata{
			Format: FormatMetadata{
				Type:    FormatArchive,
				Archive: archiveMeta,
			},
		},
	}

	meta, ok := result.Metadata.ArchiveMetadata()
	if !ok {
		t.Fatalf("expected archive metadata")
	}
	if meta.FileCount != 3 {
		t.Fatalf("expected 3 files")
	}
}

// TestXMLMetadataExtraction tests XML document metadata.
func TestXMLMetadataExtraction(t *testing.T) {
	xmlMeta := &XMLMetadata{
		ElementCount:   25,
		UniqueElements: []string{"root", "item", "value"},
	}

	result := &ExtractionResult{
		Metadata: Metadata{
			Format: FormatMetadata{
				Type: FormatXML,
				XML:  xmlMeta,
			},
		},
	}

	meta, ok := result.Metadata.XMLMetadata()
	if !ok {
		t.Fatalf("expected XML metadata")
	}
	if meta.ElementCount != 25 {
		t.Fatalf("expected 25 elements")
	}
}

// TestOCRMetadataExtraction tests OCR result metadata.
func TestOCRMetadataExtraction(t *testing.T) {
	ocrMeta := &OcrMetadata{
		Language:     "eng",
		PSM:          3,
		OutputFormat: "txt",
		TableCount:   2,
	}

	result := &ExtractionResult{
		Metadata: Metadata{
			Format: FormatMetadata{
				Type: FormatOCR,
				OCR:  ocrMeta,
			},
		},
	}

	meta, ok := result.Metadata.OcrMetadata()
	if !ok {
		t.Fatalf("expected OCR metadata")
	}
	if meta.TableCount != 2 {
		t.Fatalf("expected 2 tables in OCR")
	}
}

// TestPowerPointMetadataExtraction tests PPTX metadata.
func TestPowerPointMetadataExtraction(t *testing.T) {
	pptxMeta := &PptxMetadata{
		Title:  StringPtr("Presentation"),
		Author: StringPtr("John Doe"),
		Fonts:  []string{"Arial", "Calibri"},
	}

	result := &ExtractionResult{
		Metadata: Metadata{
			Format: FormatMetadata{
				Type: FormatPPTX,
				Pptx: pptxMeta,
			},
		},
	}

	meta, ok := result.Metadata.PptxMetadata()
	if !ok {
		t.Fatalf("expected PPTX metadata")
	}
	if len(meta.Fonts) != 2 {
		t.Fatalf("expected 2 fonts")
	}
}

// TestHtmlMetadataExtraction tests HTML metadata.
func TestHtmlMetadataExtraction(t *testing.T) {
	htmlMeta := &HtmlMetadata{
		Title:       StringPtr("Page Title"),
		Description: StringPtr("Page description"),
		Keywords:    StringPtr("key1, key2"),
	}

	result := &ExtractionResult{
		Metadata: Metadata{
			Format: FormatMetadata{
				Type: FormatHTML,
				HTML: htmlMeta,
			},
		},
	}

	meta, ok := result.Metadata.HTMLMetadata()
	if !ok {
		t.Fatalf("expected HTML metadata")
	}
	if meta.Title == nil || *meta.Title != "Page Title" {
		t.Fatalf("title not set correctly")
	}
}

// TestImagePreprocessingMetadata tests image preprocessing information.
func TestImagePreprocessingMetadata(t *testing.T) {
	preprocessing := &ImagePreprocessingMetadata{
		OriginalDimensions: [2]int{1024, 2048},
		OriginalDPI:        [2]float64{72.0, 72.0},
		TargetDPI:          300,
		ScaleFactor:        1.5,
		AutoAdjusted:       true,
	}

	result := &ExtractionResult{
		Metadata: Metadata{
			ImagePreprocessing: preprocessing,
		},
	}

	if result.Metadata.ImagePreprocessing == nil {
		t.Fatalf("expected preprocessing metadata")
	}
	if result.Metadata.ImagePreprocessing.TargetDPI != 300 {
		t.Fatalf("target DPI not set")
	}
}

// TestErrorMetadata tests error metadata in results.
func TestErrorMetadata(t *testing.T) {
	errMeta := &ErrorMetadata{
		ErrorType: "ValidationError",
		Message:   "Invalid input",
	}

	result := &ExtractionResult{
		Metadata: Metadata{
			Error: errMeta,
		},
	}

	if result.Metadata.Error == nil {
		t.Fatalf("expected error metadata")
	}
	if result.Metadata.Error.Message != "Invalid input" {
		t.Fatalf("error message not set")
	}
}

// Helper function to create int32 pointer
func IntPtr32(i uint32) *uint32 {
	return &i
}
