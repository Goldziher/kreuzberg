// Auto-generated from fixtures/plugin_api/ - DO NOT EDIT
//
// E2E tests for plugin/config/utility APIs.
//
// Generated from plugin API fixtures.
// To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang go

package e2e

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	kreuzberg "github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

// Configuration Tests

func TestDiscover(t *testing.T) {
	tmpDir := t.TempDir()
	configPath := filepath.Join(tmpDir, "kreuzberg.toml")

	if err := os.WriteFile(configPath, []byte(`[chunking]
max_chars = 50
`), 0644); err != nil {
		t.Fatalf("Failed to write config file: %v", err)
	}

	subDir := filepath.Join(tmpDir, "subdir")
	if err := os.MkdirAll(subDir, 0755); err != nil {
		t.Fatalf("Failed to create subdirectory: %v", err)
	}

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
		t.Fatalf("Discover failed: %v", err)
	}

	if config == nil {
		t.Fatal("Config should be discovered from parent directory")
	}
	if config.Chunking == nil {
		t.Fatal("Config should have chunking property")
	}
	if config.Chunking.MaxChars == nil || *config.Chunking.MaxChars != 50 {
		t.Errorf("Expected chunking.max_chars=50, got %v", *config.Chunking.MaxChars)
	}
}

func TestFromFile(t *testing.T) {
	tmpDir := t.TempDir()
	configPath := filepath.Join(tmpDir, "test_config.toml")

	configContent := `[chunking]
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
		t.Fatalf("FromFile failed: %v", err)
	}

	if config.Chunking == nil {
		t.Fatal("Config should have chunking property")
	}
	if config.Chunking.MaxChars == nil || *config.Chunking.MaxChars != 100 {
		t.Errorf("Expected chunking.max_chars=100, got %v", *config.Chunking.MaxChars)
	}
	if config.Chunking.MaxOverlap == nil || *config.Chunking.MaxOverlap != 20 {
		t.Errorf("Expected chunking.max_overlap=20, got %v", *config.Chunking.MaxOverlap)
	}
	if config.LanguageDetection == nil {
		t.Fatal("Config should have language_detection property")
	}
	if config.LanguageDetection.Enabled == nil || *config.LanguageDetection.Enabled != false {
		t.Errorf("Expected language_detection.enabled=false, got %v", *config.LanguageDetection.Enabled)
	}
}

// Document Extractor Management Tests

func TestClearDocumentExtractors(t *testing.T) {
	err := kreuzberg.ClearDocumentExtractors()
	if err != nil {
		t.Fatalf("ClearDocumentExtractors failed: %v", err)
	}

	result, err := kreuzberg.ListDocumentExtractors()
	if err != nil {
		t.Fatalf("ListDocumentExtractors failed: %v", err)
	}
	if len(result) != 0 {
		t.Errorf("Expected empty list after clear, got %d items", len(result))
	}
}

func TestListDocumentExtractors(t *testing.T) {
	tmpDir := t.TempDir()
	testFile := filepath.Join(tmpDir, "test.pdf")
	pdfContent := []byte("%PDF-1.4\\n%EOF\\n")
	if err := os.WriteFile(testFile, pdfContent, 0644); err != nil {
		t.Fatalf("Failed to write test PDF file: %v", err)
	}

	_, _ = kreuzberg.ExtractFileSync(testFile, nil)

	result, err := kreuzberg.ListDocumentExtractors()
	if err != nil {
		t.Fatalf("ListDocumentExtractors failed: %v", err)
	}
	if result == nil {
		t.Fatal("Result should not be nil")
	}
}

func TestUnregisterDocumentExtractor(t *testing.T) {
	err := kreuzberg.UnregisterDocumentExtractor("nonexistent-extractor-xyz")
	if err != nil {
		t.Errorf("UnregisterDocumentExtractor should not error for nonexistent item: %v", err)
	}
}

// Mime Utilities Tests

func TestDetectMimeType(t *testing.T) {
	testData := []byte("%PDF-1.4\\n")
	mime, err := kreuzberg.DetectMimeType(testData)
	if err != nil {
		t.Fatalf("DetectMimeType failed: %v", err)
	}

	if !strings.Contains(strings.ToLower(mime), "pdf") {
		t.Errorf("Expected MIME to contain 'pdf', got %q", mime)
	}
}

func TestDetectMimeTypeFromPath(t *testing.T) {
	tmpDir := t.TempDir()
	testFile := filepath.Join(tmpDir, "test.txt")
	if err := os.WriteFile(testFile, []byte("Hello, world!"), 0644); err != nil {
		t.Fatalf("Failed to write test file: %v", err)
	}

	mime, err := kreuzberg.DetectMimeTypeFromPath(testFile)
	if err != nil {
		t.Fatalf("DetectMimeTypeFromPath failed: %v", err)
	}

	if !strings.Contains(strings.ToLower(mime), "text") {
		t.Errorf("Expected MIME to contain 'text', got %q", mime)
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
		t.Errorf("Expected extensions to contain 'pdf', got %v", extensions)
	}
}

// Ocr Backend Management Tests

func TestClearOCRBackends(t *testing.T) {
	err := kreuzberg.ClearOCRBackends()
	if err != nil {
		t.Fatalf("ClearOCRBackends failed: %v", err)
	}

	result, err := kreuzberg.ListOCRBackends()
	if err != nil {
		t.Fatalf("ListOCRBackends failed: %v", err)
	}
	if len(result) != 0 {
		t.Errorf("Expected empty list after clear, got %d items", len(result))
	}
}

func TestListOCRBackends(t *testing.T) {
	result, err := kreuzberg.ListOCRBackends()
	if err != nil {
		t.Fatalf("ListOCRBackends failed: %v", err)
	}
	if result == nil {
		t.Fatal("Result should not be nil")
	}
}

func TestUnregisterOCRBackend(t *testing.T) {
	err := kreuzberg.UnregisterOCRBackend("nonexistent-backend-xyz")
	if err != nil {
		t.Errorf("UnregisterOCRBackend should not error for nonexistent item: %v", err)
	}
}

// Post Processor Management Tests

func TestClearPostProcessors(t *testing.T) {
	err := kreuzberg.ClearPostProcessors()
	if err != nil {
		t.Fatalf("ClearPostProcessors failed: %v", err)
	}

	result, err := kreuzberg.ListPostProcessors()
	if err != nil {
		t.Fatalf("ListPostProcessors failed: %v", err)
	}
	if len(result) != 0 {
		t.Errorf("Expected empty list after clear, got %d items", len(result))
	}
}

func TestListPostProcessors(t *testing.T) {
	result, err := kreuzberg.ListPostProcessors()
	if err != nil {
		t.Fatalf("ListPostProcessors failed: %v", err)
	}
	if result == nil {
		t.Fatal("Result should not be nil")
	}
}

// Validator Management Tests

func TestClearValidators(t *testing.T) {
	err := kreuzberg.ClearValidators()
	if err != nil {
		t.Fatalf("ClearValidators failed: %v", err)
	}

	result, err := kreuzberg.ListValidators()
	if err != nil {
		t.Fatalf("ListValidators failed: %v", err)
	}
	if len(result) != 0 {
		t.Errorf("Expected empty list after clear, got %d items", len(result))
	}
}

func TestListValidators(t *testing.T) {
	result, err := kreuzberg.ListValidators()
	if err != nil {
		t.Fatalf("ListValidators failed: %v", err)
	}
	if result == nil {
		t.Fatal("Result should not be nil")
	}
}
