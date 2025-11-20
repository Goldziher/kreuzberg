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

func TestListValidators(t *testing.T) {
	validators, err := ListValidators()
	if err != nil {
		t.Fatalf("list validators: %v", err)
	}
	if validators == nil {
		t.Fatalf("validators should not be nil")
	}
}

func TestListPostProcessors(t *testing.T) {
	processors, err := ListPostProcessors()
	if err != nil {
		t.Fatalf("list post processors: %v", err)
	}
	if processors == nil {
		t.Fatalf("processors should not be nil")
	}
}

func TestClearValidators(t *testing.T) {
	name := fmt.Sprintf("go-validator-clear-%d", time.Now().UnixNano())
	if err := RegisterValidator(name, 10, testValidatorCallback); err != nil {
		t.Fatalf("register validator: %v", err)
	}

	if err := ClearValidators(); err != nil {
		t.Fatalf("clear validators: %v", err)
	}

	validators, err := ListValidators()
	if err != nil {
		t.Fatalf("list validators: %v", err)
	}

	for _, v := range validators {
		if v == name {
			t.Fatalf("validator %s should have been cleared", name)
		}
	}
}

func TestClearPostProcessors(t *testing.T) {
	name := fmt.Sprintf("go-post-clear-%d", time.Now().UnixNano())
	if err := RegisterPostProcessor(name, 50, testPostProcessorCallback); err != nil {
		t.Fatalf("register post processor: %v", err)
	}

	if err := ClearPostProcessors(); err != nil {
		t.Fatalf("clear post processors: %v", err)
	}

	processors, err := ListPostProcessors()
	if err != nil {
		t.Fatalf("list post processors: %v", err)
	}

	for _, p := range processors {
		if p == name {
			t.Fatalf("post processor %s should have been cleared", name)
		}
	}
}

func TestValidatorListLifecycle(t *testing.T) {
	name := fmt.Sprintf("go-validator-list-%d", time.Now().UnixNano())

	if err := RegisterValidator(name, 10, testValidatorCallback); err != nil {
		t.Fatalf("register validator: %v", err)
	}

	validators, err := ListValidators()
	if err != nil {
		t.Fatalf("list validators: %v", err)
	}

	found := false
	for _, v := range validators {
		if v == name {
			found = true
			break
		}
	}
	if !found {
		t.Fatalf("validator %s should be in the list", name)
	}

	if err := UnregisterValidator(name); err != nil {
		t.Fatalf("unregister validator: %v", err)
	}

	validators, err = ListValidators()
	if err != nil {
		t.Fatalf("list validators after unregister: %v", err)
	}

	for _, v := range validators {
		if v == name {
			t.Fatalf("validator %s should not be in the list after unregister", name)
		}
	}
}

func TestPostProcessorListLifecycle(t *testing.T) {
	name := fmt.Sprintf("go-post-list-%d", time.Now().UnixNano())

	if err := RegisterPostProcessor(name, 50, testPostProcessorCallback); err != nil {
		t.Fatalf("register post processor: %v", err)
	}

	processors, err := ListPostProcessors()
	if err != nil {
		t.Fatalf("list post processors: %v", err)
	}

	found := false
	for _, p := range processors {
		if p == name {
			found = true
			break
		}
	}
	if !found {
		t.Fatalf("post processor %s should be in the list", name)
	}

	if err := UnregisterPostProcessor(name); err != nil {
		t.Fatalf("unregister post processor: %v", err)
	}

	processors, err = ListPostProcessors()
	if err != nil {
		t.Fatalf("list post processors after unregister: %v", err)
	}

	for _, p := range processors {
		if p == name {
			t.Fatalf("post processor %s should not be in the list after unregister", name)
		}
	}
}

func TestListOCRBackends(t *testing.T) {
	backends, err := ListOCRBackends()
	if err != nil {
		t.Fatalf("ListOCRBackends failed: %v", err)
	}
	if backends == nil {
		t.Fatal("Expected non-nil backends list")
	}
}

func TestUnregisterOCRBackend(t *testing.T) {
	err := UnregisterOCRBackend("nonexistent-backend")
	_ = err
}
