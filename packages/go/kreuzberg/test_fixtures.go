package kreuzberg

import (
	"embed"
	"fmt"
	"os"
	"path/filepath"
)

//go:embed .kreuzberg/sample.pdf
var testFixtures embed.FS

// getValidPDFBytes returns a valid PDF byte content for testing.
// This is used instead of minimal PDF headers that PDFium cannot parse.
func getValidPDFBytes() ([]byte, error) {
	data, err := testFixtures.ReadFile(".kreuzberg/sample.pdf")
	if err != nil {
		return nil, fmt.Errorf("failed to load embedded PDF fixture: %w", err)
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
