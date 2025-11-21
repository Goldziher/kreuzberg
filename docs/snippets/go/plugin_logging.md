```go
package main

import (
	"C"
	"encoding/json"
	"log"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

//export loggingPostProcessor
func loggingPostProcessor(resultJSON *C.char) *C.char {
	log.Println("[PostProcessor] Processing extraction result")

	jsonStr := C.GoString(resultJSON)
	var result map[string]interface{}
	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		log.Printf("[PostProcessor] Error parsing result: %v", err)
		return nil
	}

	if content, ok := result["content"].(string); ok {
		log.Printf("[PostProcessor] Content length: %d bytes", len(content))
		if len(content) == 0 {
			log.Println("[PostProcessor] Warning: Empty content extracted")
		}
	}

	if mimeType, ok := result["mime_type"].(string); ok {
		log.Printf("[PostProcessor] Processing %s", mimeType)
	}

	// Return NULL to indicate success (no modification)
	return nil
}

//export loggingValidator
func loggingValidator(resultJSON *C.char) *C.char {
	log.Println("[Validator] Validating extraction result")

	jsonStr := C.GoString(resultJSON)
	var result map[string]interface{}
	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		log.Printf("[Validator] Error parsing result: %v", err)
		errMsg := "Failed to parse validation input"
		return C.CString(errMsg)
	}

	if content, ok := result["content"].(string); ok {
		log.Printf("[Validator] Content length: %d bytes", len(content))
		if len(content) < 50 {
			log.Println("[Validator] Error: Content below minimum threshold")
			errMsg := "Content too short (minimum 50 characters)"
			return C.CString(errMsg)
		}
	}

	// Return NULL to indicate validation passed
	return nil
}

func main() {
	// Register post processor with logging
	if err := kreuzberg.RegisterPostProcessor(
		"logging-processor",
		100, // priority
		(C.PostProcessorCallback)(C.loggingPostProcessor),
	); err != nil {
		log.Fatalf("register post processor failed: %v", err)
	}
	log.Println("[Main] PostProcessor registered with logging enabled")

	// Register validator with logging
	if err := kreuzberg.RegisterValidator(
		"logging-validator",
		50, // priority
		(C.ValidatorCallback)(C.loggingValidator),
	); err != nil {
		log.Fatalf("register validator failed: %v", err)
	}
	log.Println("[Main] Validator registered with logging enabled")

	// Extract with logging
	result, err := kreuzberg.ExtractFileSync("document.pdf", nil)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Printf("[Main] Extraction complete: %d bytes content", len(result.Content))
}
```
