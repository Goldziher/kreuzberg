```go title="Go"
package main

import (
	"C"
	"encoding/json"
	"fmt"
	"testing"
	"unsafe"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

// TestPostProcessor tests custom post processor behavior
func TestPostProcessor(t *testing.T) {
	// Create a post processor that adds metadata
	metricsMap := make(map[string]int64)

	//export testPostProcessor
	testPostProcessor := func(resultJSON *C.char) *C.char {
		jsonStr := C.GoString(resultJSON)
		var result map[string]interface{}
		if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
			return nil
		}

		if content, ok := result["content"].(string); ok {
			metricsMap["content_length"] = int64(len(content))
			metricsMap["processed"] = 1
		}

		return nil
	}

	// Register the processor
	err := kreuzberg.RegisterPostProcessor(
		"test-processor",
		10,
		(C.PostProcessorCallback)(C.testPostProcessor),
	)
	if err != nil {
		t.Fatalf("Failed to register post processor: %v", err)
	}

	// Simulate a mock result
	mockResult := map[string]interface{}{
		"content":           "Test extraction content",
		"mime_type":         "text/plain",
		"metadata":          map[string]interface{}{},
		"tables":            []interface{}{},
		"detected_languages": []interface{}{},
	}

	resultJSON, err := json.Marshal(mockResult)
	if err != nil {
		t.Fatalf("Failed to marshal mock result: %v", err)
	}
	cResultJSON := C.CString(string(resultJSON))
	defer C.free(unsafe.Pointer(cResultJSON))

	// Call the processor
	testPostProcessor(cResultJSON)

	// Verify metrics were recorded
	if metricsMap["content_length"] != 22 {
		t.Errorf("Expected content_length 22, got %d", metricsMap["content_length"])
	}
	if metricsMap["processed"] != 1 {
		t.Errorf("Expected processed flag to be 1")
	}

	// Cleanup
	_ = kreuzberg.UnregisterPostProcessor("test-processor")
}

// TestValidator tests custom validator behavior
func TestValidator(t *testing.T) {
	validatorCalled := false

	//export testValidator
	testValidator := func(resultJSON *C.char) *C.char {
		validatorCalled = true
		jsonStr := C.GoString(resultJSON)
		var result map[string]interface{}
		if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
			return C.CString("Failed to parse validation input")
		}

		if content, ok := result["content"].(string); ok {
			if len(content) < 10 {
				return C.CString("Content too short")
			}
		}

		return nil // Success
	}

	// Register the validator
	err := kreuzberg.RegisterValidator(
		"test-validator",
		50,
		(C.ValidatorCallback)(C.testValidator),
	)
	if err != nil {
		t.Fatalf("Failed to register validator: %v", err)
	}

	// Test 1: Valid content
	validContent := map[string]interface{}{
		"content":           "This is valid content",
		"mime_type":         "text/plain",
		"metadata":          map[string]interface{}{},
		"tables":            []interface{}{},
		"detected_languages": []interface{}{},
	}

	validJSON, err := json.Marshal(validContent)
	if err != nil {
		t.Fatalf("Failed to marshal valid content: %v", err)
	}
	cValidJSON := C.CString(string(validJSON))
	defer C.free(unsafe.Pointer(cValidJSON))

	result := testValidator(cValidJSON)
	if result != nil {
		t.Errorf("Expected nil (success), got error: %s", C.GoString(result))
	}

	if !validatorCalled {
		t.Errorf("Validator was not called")
	}

	// Test 2: Invalid content (too short)
	invalidContent := map[string]interface{}{
		"content":           "Short",
		"mime_type":         "text/plain",
		"metadata":          map[string]interface{}{},
		"tables":            []interface{}{},
		"detected_languages": []interface{}{},
	}

	invalidJSON, err := json.Marshal(invalidContent)
	if err != nil {
		t.Fatalf("Failed to marshal invalid content: %v", err)
	}
	cInvalidJSON := C.CString(string(invalidJSON))
	defer C.free(unsafe.Pointer(cInvalidJSON))

	result = testValidator(cInvalidJSON)
	if result == nil {
		t.Errorf("Expected error for short content, got nil")
	} else {
		errorMsg := C.GoString(result)
		if errorMsg != "Content too short" {
			t.Errorf("Expected 'Content too short', got: %s", errorMsg)
		}
	}

	// Cleanup
	_ = kreuzberg.UnregisterValidator("test-validator")
}

// TestValidatorIntegration tests validator with actual extraction
func TestValidatorIntegration(t *testing.T) {
	//export integrationValidator
	integrationValidator := func(resultJSON *C.char) *C.char {
		jsonStr := C.GoString(resultJSON)
		var result map[string]interface{}
		if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
			return C.CString(fmt.Sprintf("Parse error: %v", err))
		}

		// Validate that mime_type is set
		if _, ok := result["mime_type"]; !ok {
			return C.CString("Missing mime_type in result")
		}

		return nil
	}

	// Register validator
	err := kreuzberg.RegisterValidator(
		"integration-validator",
		100,
		(C.ValidatorCallback)(C.integrationValidator),
	)
	if err != nil {
		t.Fatalf("Failed to register validator: %v", err)
	}

	// The validator will be called automatically during extraction
	// This test verifies the registration was successful
	validators, err := kreuzberg.ListValidators()
	if err != nil {
		t.Fatalf("Failed to list validators: %v", err)
	}

	found := false
	for _, v := range validators {
		if v == "integration-validator" {
			found = true
			break
		}
	}

	if !found {
		t.Errorf("Validator not found in registered validators list")
	}

	// Cleanup
	_ = kreuzberg.UnregisterValidator("integration-validator")
}
```
