```go
package main

import (
	"encoding/json"
	"fmt"
	"log"
	"unsafe"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

/*
#cgo CFLAGS: -I${SRCDIR}/../../../crates/kreuzberg-ffi
#cgo LDFLAGS: -L${SRCDIR}/../../../target/release -L${SRCDIR}/../../../target/debug -lkreuzberg_ffi
#include "../../../crates/kreuzberg-ffi/kreuzberg.h"
#include <stdlib.h>
*/
import "C"

// minLengthConfig holds the configuration for the minimum length validator
var minLengthConfig = struct {
	minLength int
}{
	minLength: 100,
}

// minLengthValidator validates that extracted content meets minimum length requirement
//export minLengthValidator
func minLengthValidator(resultJSON *C.char) *C.char {
	jsonStr := C.GoString(resultJSON)
	var result map[string]interface{}

	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		return C.CString("Failed to parse result JSON")
	}

	content, ok := result["content"].(string)
	if !ok {
		return C.CString("Missing content field in result")
	}

	if len(content) < minLengthConfig.minLength {
		errMsg := fmt.Sprintf("Content too short: %d < %d", len(content), minLengthConfig.minLength)
		return C.CString(errMsg)
	}

	// Validation passed
	return nil
}

func main() {
	// Register the validator with priority 100 (runs early)
	if err := kreuzberg.RegisterValidator("min_length_validator", 100,
		(C.ValidatorCallback)(C.minLengthValidator)); err != nil {
		log.Fatalf("failed to register validator: %v", err)
	}
	defer func() {
		if err := kreuzberg.UnregisterValidator("min_length_validator"); err != nil {
			log.Printf("warning: failed to unregister validator: %v", err)
		}
	}()

	// Extract and validate
	result, err := kreuzberg.ExtractFileSync("document.pdf", nil)
	if err != nil {
		log.Fatalf("extraction failed: %v", err)
	}

	log.Printf("Validation passed. Content length: %d", len(result.Content))
}
```
