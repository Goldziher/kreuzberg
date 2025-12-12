package kreuzberg

import (
	"strings"
	"testing"
)

func TestClassifyNativeErrorReturnsValidationError(t *testing.T) {
	err := classifyNativeError("Validation error: Document did not pass schema", ErrorCodeInvalidArgument, nil)
	valErr, ok := err.(*ValidationError)
	if !ok {
		t.Fatalf("expected ValidationError, got %T", err)
	}
	if valErr.Kind() != ErrorKindValidation {
		t.Fatalf("unexpected kind: %s", valErr.Kind())
	}
	if !strings.Contains(valErr.Error(), "Validation error") {
		t.Fatalf("missing validation message: %s", valErr.Error())
	}
}

func TestClassifyNativeErrorMissingDependency(t *testing.T) {
	err := classifyNativeError("Missing dependency: tesseract", ErrorCodeMissingDependency, nil)
	missing, ok := err.(*MissingDependencyError)
	if !ok {
		t.Fatalf("expected MissingDependencyError, got %T", err)
	}
	if missing.Dependency != "tesseract" {
		t.Fatalf("unexpected dependency: %s", missing.Dependency)
	}
}

func TestClassifyNativeErrorPlugin(t *testing.T) {
	err := classifyNativeError("Plugin error in 'custom': failed to register", ErrorCodeGenericError, nil)
	pluginErr, ok := err.(*PluginError)
	if !ok {
		t.Fatalf("expected PluginError, got %T", err)
	}
	if pluginErr.PluginName != "custom" {
		t.Fatalf("unexpected plugin name: %s", pluginErr.PluginName)
	}
}

func TestErrorWithPanicContext(t *testing.T) {
	panicCtx := &PanicContext{
		File:         "src/core.rs",
		Line:         42,
		Function:     "process_document",
		Message:      "unexpected state",
		TimestampSec: 1234567890,
	}
	err := classifyNativeError("OCR error: processing failed", ErrorCodePanic, panicCtx)
	ocrErr, ok := err.(*OCRError)
	if !ok {
		t.Fatalf("expected OCRError, got %T", err)
	}
	if ocrErr.PanicCtx() == nil {
		t.Fatalf("expected panic context to be set")
	}
	if ocrErr.PanicCtx().File != "src/core.rs" {
		t.Fatalf("expected panic context file to be src/core.rs, got %s", ocrErr.PanicCtx().File)
	}
	if ocrErr.PanicCtx().Line != 42 {
		t.Fatalf("expected panic context line to be 42, got %d", ocrErr.PanicCtx().Line)
	}
	if !strings.Contains(ocrErr.Error(), "src/core.rs:42") {
		t.Fatalf("expected panic context in error message, got: %s", ocrErr.Error())
	}
}

func TestExtractBytesSyncValidationErrors(t *testing.T) {
	if _, err := ExtractBytesSync(nil, "text/plain", nil); err == nil {
		t.Fatalf("expected error for empty data")
	} else {
		if _, ok := err.(*ValidationError); !ok {
			t.Fatalf("expected ValidationError for empty data, got %T", err)
		}
	}

	if _, err := ExtractBytesSync([]byte("hello"), "", nil); err == nil {
		t.Fatalf("expected error for empty mime type")
	} else {
		if _, ok := err.(*ValidationError); !ok {
			t.Fatalf("expected ValidationError for empty mime type, got %T", err)
		}
	}
}

func TestLoadExtractionConfigFromFileValidation(t *testing.T) {
	_, err := LoadExtractionConfigFromFile("")
	if err == nil {
		t.Fatalf("expected validation error for empty config path")
	}
	if _, ok := err.(*ValidationError); !ok {
		t.Fatalf("expected ValidationError, got %T", err)
	}
}
