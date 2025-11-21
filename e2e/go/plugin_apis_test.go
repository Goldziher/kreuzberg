package e2e

// Auto-generated plugin API tests for Go binding.
//
// E2E tests for plugin registration and management APIs.
//
// Tests all plugin types:
// - Validators
// - Post-processors
// - OCR backends
// - Document extractors
//
// Tests all management operations:
// - Registration
// - Unregistration
// - Listing
// - Clearing

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	kreuzberg "github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func TestListValidators(t *testing.T) {
	validators, err := kreuzberg.ListValidators()
	if err != nil {
		t.Fatalf("ListValidators failed: %v", err)
	}
	if validators == nil {
		t.Fatal("Validators list should not be nil")
	}
}

func TestClearValidators(t *testing.T) {
	err := kreuzberg.ClearValidators()
	if err != nil {
		t.Fatalf("ClearValidators failed: %v", err)
	}

	validators, err := kreuzberg.ListValidators()
	if err != nil {
		t.Fatalf("ListValidators failed: %v", err)
	}
	if len(validators) != 0 {
		t.Errorf("Expected empty validators list after clear, got %d items", len(validators))
	}
}

func TestListPostProcessors(t *testing.T) {
	processors, err := kreuzberg.ListPostProcessors()
	if err != nil {
		t.Fatalf("ListPostProcessors failed: %v", err)
	}
	if processors == nil {
		t.Fatal("Post-processors list should not be nil")
	}
}

func TestClearPostProcessors(t *testing.T) {
	err := kreuzberg.ClearPostProcessors()
	if err != nil {
		t.Fatalf("ClearPostProcessors failed: %v", err)
	}

	processors, err := kreuzberg.ListPostProcessors()
	if err != nil {
		t.Fatalf("ListPostProcessors failed: %v", err)
	}
	if len(processors) != 0 {
		t.Errorf("Expected empty post-processors list after clear, got %d items", len(processors))
	}
}

func TestListOCRBackends(t *testing.T) {
	backends, err := kreuzberg.ListOCRBackends()
	if err != nil {
		t.Fatalf("ListOCRBackends failed: %v", err)
	}
	if backends == nil {
		t.Fatal("OCR backends list should not be nil")
	}

	// Should include built-in backends
	found := false
	for _, backend := range backends {
		if backend == "tesseract" {
			found = true
			break
		}
	}
	if !found {
		t.Error("Expected 'tesseract' in OCR backends list")
	}
}

func TestUnregisterOCRBackend(t *testing.T) {
	// Should handle nonexistent backend gracefully
	err := kreuzberg.UnregisterOCRBackend("nonexistent-backend-xyz")
	if err != nil {
		t.Errorf("UnregisterOCRBackend should not error on nonexistent backend: %v", err)
	}
}

func TestClearOCRBackends(t *testing.T) {
	err := kreuzberg.ClearOCRBackends()
	if err != nil {
		t.Fatalf("ClearOCRBackends failed: %v", err)
	}

	backends, err := kreuzberg.ListOCRBackends()
	if err != nil {
		t.Fatalf("ListOCRBackends failed: %v", err)
	}
	if len(backends) != 0 {
		t.Errorf("Expected empty OCR backends list after clear, got %d items", len(backends))
	}
}

func TestListDocumentExtractors(t *testing.T) {
	// Ensure extractors are initialized by using one first
	tmpDir := t.TempDir()
	testFile := filepath.Join(tmpDir, "test.pdf")
	pdfContent := []byte("%PDF-1.4\n%EOF\n")
	if err := os.WriteFile(testFile, pdfContent, 0644); err != nil {
		t.Fatalf("Failed to write test PDF file: %v", err)
	}

	// This will initialize the PDF extractor
	_, _ = kreuzberg.ExtractFileSync(testFile, nil)

	extractors, err := kreuzberg.ListDocumentExtractors()
	if err != nil {
		t.Fatalf("ListDocumentExtractors failed: %v", err)
	}
	if extractors == nil {
		t.Fatal("Document extractors list should not be nil")
	}

	// Should include built-in extractors
	found := false
	for _, extractor := range extractors {
		if strings.Contains(strings.ToLower(extractor), "pdf") {
			found = true
			break
		}
	}
	if !found {
		t.Error("Expected a PDF extractor in document extractors list")
	}
}

func TestUnregisterDocumentExtractor(t *testing.T) {
	// Should handle nonexistent extractor gracefully
	err := kreuzberg.UnregisterDocumentExtractor("nonexistent-extractor-xyz")
	if err != nil {
		t.Errorf("UnregisterDocumentExtractor should not error on nonexistent extractor: %v", err)
	}
}

func TestClearDocumentExtractors(t *testing.T) {
	err := kreuzberg.ClearDocumentExtractors()
	if err != nil {
		t.Fatalf("ClearDocumentExtractors failed: %v", err)
	}

	extractors, err := kreuzberg.ListDocumentExtractors()
	if err != nil {
		t.Fatalf("ListDocumentExtractors failed: %v", err)
	}
	if len(extractors) != 0 {
		t.Errorf("Expected empty document extractors list after clear, got %d items", len(extractors))
	}
}

func TestConfigFromFile(t *testing.T) {
	tmpDir := t.TempDir()
	configPath := filepath.Join(tmpDir, "test_config.toml")

	configContent := `
[chunking]
max_chars = 100
max_overlap = 20

[language_detection]
enabled = false
`
	if err := os.WriteFile(configPath, []byte(configContent), 0644); err != nil {
		t.Fatalf("Failed to write config file: %v", err)
	}

	config, err := kreuzberg.ConfigFromFile(configPath)
	if err != nil {
		t.Fatalf("ConfigFromFile failed: %v", err)
	}

	if config.Chunking == nil {
		t.Fatal("Config should have chunking settings")
	}
	if config.Chunking.MaxChars == nil || *config.Chunking.MaxChars != 100 {
		if config.Chunking.MaxChars != nil {
			t.Errorf("Expected MaxChars=100, got %d", *config.Chunking.MaxChars)
		} else {
			t.Error("Expected MaxChars=100, got nil")
		}
	}
	if config.Chunking.MaxOverlap == nil || *config.Chunking.MaxOverlap != 20 {
		if config.Chunking.MaxOverlap != nil {
			t.Errorf("Expected MaxOverlap=20, got %d", *config.Chunking.MaxOverlap)
		} else {
			t.Error("Expected MaxOverlap=20, got nil")
		}
	}

	if config.LanguageDetection == nil {
		t.Fatal("Config should have language detection settings")
	}
	if config.LanguageDetection.Enabled != nil && *config.LanguageDetection.Enabled {
		t.Error("Expected language detection to be disabled")
	}
}

func TestConfigDiscover(t *testing.T) {
	tmpDir := t.TempDir()
	configPath := filepath.Join(tmpDir, "kreuzberg.toml")

	configContent := `
[chunking]
max_chars = 50
`
	if err := os.WriteFile(configPath, []byte(configContent), 0644); err != nil {
		t.Fatalf("Failed to write config file: %v", err)
	}

	// Create subdirectory
	subDir := filepath.Join(tmpDir, "subdir")
	if err := os.MkdirAll(subDir, 0755); err != nil {
		t.Fatalf("Failed to create subdirectory: %v", err)
	}

	// Change to subdirectory
	originalDir, err := os.Getwd()
	if err != nil {
		t.Fatalf("Failed to get current directory: %v", err)
	}
	defer os.Chdir(originalDir)

	if err := os.Chdir(subDir); err != nil {
		t.Fatalf("Failed to change directory: %v", err)
	}

	config, err := kreuzberg.ConfigDiscover()
	if err != nil {
		t.Fatalf("ConfigDiscover failed: %v", err)
	}

	if config == nil {
		t.Fatal("Config should be discovered from parent directory")
	}

	if config.Chunking == nil {
		t.Fatal("Config should have chunking settings")
	}
	if config.Chunking.MaxChars == nil || *config.Chunking.MaxChars != 50 {
		if config.Chunking.MaxChars != nil {
			t.Errorf("Expected MaxChars=50, got %d", *config.Chunking.MaxChars)
		} else {
			t.Error("Expected MaxChars=50, got nil")
		}
	}
}

func TestDetectMimeType(t *testing.T) {
	// PDF magic bytes
	pdfBytes := []byte("%PDF-1.4\n")
	mimeType, err := kreuzberg.DetectMimeType(pdfBytes)
	if err != nil {
		t.Fatalf("DetectMimeType failed: %v", err)
	}

	if !strings.Contains(strings.ToLower(mimeType), "pdf") {
		t.Errorf("Expected MIME type to contain 'pdf', got %s", mimeType)
	}
}

func TestDetectMimeTypeFromPath(t *testing.T) {
	tmpDir := t.TempDir()
	testFile := filepath.Join(tmpDir, "test.txt")

	if err := os.WriteFile(testFile, []byte("Hello, world!"), 0644); err != nil {
		t.Fatalf("Failed to write test file: %v", err)
	}

	mimeType, err := kreuzberg.DetectMimeTypeFromPath(testFile)
	if err != nil {
		t.Fatalf("DetectMimeTypeFromPath failed: %v", err)
	}

	if !strings.Contains(strings.ToLower(mimeType), "text") {
		t.Errorf("Expected MIME type to contain 'text', got %s", mimeType)
	}
}

func TestGetExtensionsForMime(t *testing.T) {
	extensions, err := kreuzberg.GetExtensionsForMime("application/pdf")
	if err != nil {
		t.Fatalf("GetExtensionsForMime failed: %v", err)
	}

	if extensions == nil {
		t.Fatal("Extensions list should not be nil")
	}

	found := false
	for _, ext := range extensions {
		if ext == "pdf" {
			found = true
			break
		}
	}
	if !found {
		t.Error("Expected 'pdf' in extensions list")
	}
}
