```go
package main

import (
	"encoding/json"
	"log"
	"strings"
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

// wordCountProcessor adds word count metadata to extraction results
//export wordCountProcessor
func wordCountProcessor(resultJSON *C.char) *C.char {
	jsonStr := C.GoString(resultJSON)
	var result map[string]interface{}

	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		return C.CString("{\"error\":\"Failed to parse result JSON\"}")
	}

	// Extract content
	content, ok := result["content"].(string)
	if !ok {
		return C.CString("{\"error\":\"Missing content field\"}")
	}

	// Count words by splitting on whitespace
	words := strings.Fields(content)
	wordCount := len(words)

	// Ensure metadata exists
	metadata, ok := result["metadata"].(map[string]interface{})
	if !ok {
		metadata = make(map[string]interface{})
	}

	// Add word count to metadata
	metadata["word_count"] = wordCount

	// Update result
	result["metadata"] = metadata

	// Serialize back to JSON
	outputJSON, err := json.Marshal(result)
	if err != nil {
		return C.CString("{\"error\":\"Failed to serialize result\"}")
	}

	return C.CString(string(outputJSON))
}

func main() {
	// Register the post-processor with priority 100, early stage
	if err := kreuzberg.RegisterPostProcessor("word_count_processor", 100,
		(C.PostProcessorCallback)(C.wordCountProcessor)); err != nil {
		log.Fatalf("failed to register post-processor: %v", err)
	}
	defer func() {
		if err := kreuzberg.UnregisterPostProcessor("word_count_processor"); err != nil {
			log.Printf("warning: failed to unregister post-processor: %v", err)
		}
	}()

	// Extract document
	result, err := kreuzberg.ExtractFileSync("document.pdf", nil)
	if err != nil {
		log.Fatalf("extraction failed: %v", err)
	}

	// Access word count from metadata
	var metadata map[string]interface{}
	if metaJSON, ok := result.MetadataJSON.(string); ok {
		if err := json.Unmarshal([]byte(metaJSON), &metadata); err == nil {
			if wordCount, ok := metadata["word_count"].(float64); ok {
				log.Printf("Word count: %.0f", wordCount)
			}
		}
	}
}
```
