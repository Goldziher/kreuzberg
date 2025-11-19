package kreuzberg

import (
	"strings"
	"testing"
)

func TestClassifyNativeErrorReturnsValidationError(t *testing.T) {
	err := classifyNativeError("Validation error: Document did not pass schema")
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
	err := classifyNativeError("Missing dependency: tesseract")
	missing, ok := err.(*MissingDependencyError)
	if !ok {
		t.Fatalf("expected MissingDependencyError, got %T", err)
	}
	if missing.Dependency != "tesseract" {
		t.Fatalf("unexpected dependency: %s", missing.Dependency)
	}
}

func TestClassifyNativeErrorPlugin(t *testing.T) {
	err := classifyNativeError("Plugin error in 'custom': failed to register")
	pluginErr, ok := err.(*PluginError)
	if !ok {
		t.Fatalf("expected PluginError, got %T", err)
	}
	if pluginErr.PluginName != "custom" {
		t.Fatalf("unexpected plugin name: %s", pluginErr.PluginName)
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
