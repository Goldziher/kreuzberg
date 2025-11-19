package kreuzberg

import (
	"fmt"
	"testing"
	"time"
)

func TestRegisterPostProcessorLifecycle(t *testing.T) {
	name := fmt.Sprintf("go-post-%d", time.Now().UnixNano())
	if err := RegisterPostProcessor(name, 50, testPostProcessorCallback); err != nil {
		t.Fatalf("register post processor: %v", err)
	}
	if err := UnregisterPostProcessor(name); err != nil {
		t.Fatalf("unregister post processor: %v", err)
	}
}

func TestRegisterValidatorLifecycle(t *testing.T) {
	name := fmt.Sprintf("go-validator-%d", time.Now().UnixNano())
	if err := RegisterValidator(name, 10, testValidatorCallback); err != nil {
		t.Fatalf("register validator: %v", err)
	}
	if err := UnregisterValidator(name); err != nil {
		t.Fatalf("unregister validator: %v", err)
	}
}

func TestRegisterOCRBackend(t *testing.T) {
	name := fmt.Sprintf("go-ocr-%d", time.Now().UnixNano())
	if err := RegisterOCRBackend(name, testOcrBackendCallback); err != nil {
		t.Fatalf("register ocr backend: %v", err)
	}
}

func TestRegisterValidatorGuards(t *testing.T) {
	if err := RegisterValidator("", 0, nil); err == nil {
		t.Fatalf("expected validation error for empty name")
	}
	if err := RegisterPostProcessor("", 0, nil); err == nil {
		t.Fatalf("expected validation error for empty post processor")
	}
	if err := RegisterOCRBackend("", nil); err == nil {
		t.Fatalf("expected validation error for empty OCR backend name")
	}
}
