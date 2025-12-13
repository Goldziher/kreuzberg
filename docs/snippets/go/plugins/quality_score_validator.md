```go title="Go"
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

// qualityThreshold is the minimum acceptable quality score
const qualityThreshold = 0.5

// qualityScoreValidator validates that extraction quality meets minimum threshold
//export qualityScoreValidator
func qualityScoreValidator(resultJSON *C.char) *C.char {
	jsonStr := C.GoString(resultJSON)
	var result map[string]interface{}

	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		return C.CString("Failed to parse result JSON")
	}

	// Extract metadata object
	metadata, ok := result["metadata"].(map[string]interface{})
	if !ok {
		// No metadata is not an error, just skip quality check
		return nil
	}

	// Get quality score from metadata
	qualityScore := 0.0
	if score, ok := metadata["quality_score"].(float64); ok {
		qualityScore = score
	}

	// Validate against threshold
	if qualityScore < qualityThreshold {
		errMsg := fmt.Sprintf("Quality score too low: %.0f%% < %.0f%%", qualityScore*100, qualityThreshold*100)
		return C.CString(errMsg)
	}

	// Validation passed
	return nil
}

func main() {
	// Register the validator with priority 50
	if err := kreuzberg.RegisterValidator("quality_score_validator", 50,
		(C.ValidatorCallback)(C.qualityScoreValidator)); err != nil {
		log.Fatalf("failed to register validator: %v", err)
	}
	defer func() {
		if err := kreuzberg.UnregisterValidator("quality_score_validator"); err != nil {
			log.Printf("warning: failed to unregister validator: %v", err)
		}
	}()

	// Extract and validate
	result, err := kreuzberg.ExtractFileSync("document.pdf", nil)
	if err != nil {
		log.Fatalf("extraction or validation failed: %v", err)
	}

	log.Printf("Quality validation passed for: %s", result.MimeType)
}
```
