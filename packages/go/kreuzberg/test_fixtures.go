package kreuzberg

import (
	"fmt"
	"os"
	"path/filepath"
)

// getValidPDFBytes returns a valid PDF byte content for testing.
// This is used instead of minimal PDF headers that PDFium cannot parse.
// It reads from the test_documents directory in the kreuzberg repo root.
func getValidPDFBytes() ([]byte, error) {
	// Find kreuzberg repo root
	// When test runs from packages/go, working dir is packages/go
	// packages/go (wd) -> .. = packages -> .. = repo root
	wd, err := os.Getwd()
	if err != nil {
		return nil, fmt.Errorf("failed to get working directory: %w", err)
	}

	// Navigate from packages/go to repo root
	repoRoot := filepath.Join(wd, "..", "..")
	testPDF := filepath.Join(repoRoot, "test_documents", "pdfs_with_tables", "tiny.pdf")

	// #nosec G304 -- Test fixture reads from known test_documents directory
	data, err := os.ReadFile(testPDF)
	if err != nil {
		return nil, fmt.Errorf("failed to read test PDF from %s: %w", testPDF, err)
	}
	return data, nil
}

// writeValidPDFToFile writes a valid PDF to a temporary file for testing.
// Returns the file path and any error encountered.
func writeValidPDFToFile(dir string, filename string) (string, error) {
	pdfData, err := getValidPDFBytes()
	if err != nil {
		return "", err
	}

	path := filepath.Join(dir, filename)
	if err := os.WriteFile(path, pdfData, 0o600); err != nil {
		return "", fmt.Errorf("failed to write PDF file: %w", err)
	}

	return path, nil
}
