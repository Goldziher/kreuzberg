package kreuzberg

import (
	"os"
	"path/filepath"
	"testing"
)

func TestDetectMimeTypePDF(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "sample.pdf")
	if err := os.WriteFile(path, []byte("%PDF-1.7\n%"), 0o644); err != nil {
		t.Fatalf("write temp pdf: %v", err)
	}

	mime, err := DetectMimeTypeFromPath(path)
	if err != nil {
		t.Fatalf("detect mime: %v", err)
	}
	if mime != "application/pdf" {
		t.Fatalf("expected application/pdf, got %s", mime)
	}
}

func TestDetectMimeTypeMissingFile(t *testing.T) {
	_, err := DetectMimeTypeFromPath("nonexistent-file.docx")
	if err == nil {
		t.Fatalf("expected error for missing file")
	}
}

func TestValidateMimeType(t *testing.T) {
	mime, err := ValidateMimeType("application/pdf")
	if err != nil {
		t.Fatalf("validate mime: %v", err)
	}
	if mime != "application/pdf" {
		t.Fatalf("unexpected validated mime: %s", mime)
	}

	if _, err := ValidateMimeType("video/mp4"); err == nil {
		t.Fatalf("expected unsupported format error")
	}
}
