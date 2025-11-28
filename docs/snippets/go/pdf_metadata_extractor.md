```go
package main

import (
	"encoding/json"
	"log"
	"sync/atomic"
	"unsafe"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

/*
#cgo CFLAGS: -I${SRCDIR}/../../../crates/kreuzberg-ffi
#cgo LDFLAGS: -L${SRCDIR}/../../../target/release -L${SRCDIR}/../../../target/debug -lkreuzberg_ffi
#include "../../../crates/kreuzberg-ffi/kreuzberg.h"
#include <stdlib.h>
*/
import "C"

// pdfMetadataState tracks statistics about PDF processing
var pdfMetadataState = struct {
	processedCount int64
}{
	processedCount: 0,
}

// pdfMetadataExtractor enriches PDF extraction results with additional metadata
//export pdfMetadataExtractor
func pdfMetadataExtractor(resultJSON *C.char) *C.char {
	jsonStr := C.GoString(resultJSON)
	var result map[string]interface{}

	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		return C.CString("{\"error\":\"Failed to parse result JSON\"}")
	}

	// Only process PDFs
	mimeType, ok := result["mime_type"].(string)
	if !ok || mimeType != "application/pdf" {
		// Return unchanged for non-PDF documents
		outputJSON, err := json.Marshal(result)
		if err != nil {
			return C.CString("{\"error\":\"Failed to serialize result\"}")
		}
		return C.CString(string(outputJSON))
	}

	// Process PDF-specific metadata
	metadata, ok := result["metadata"].(map[string]interface{})
	if !ok {
		metadata = make(map[string]interface{})
	}

	// Mark as processed by this processor
	metadata["pdf_processed"] = true

	// Add content statistics
	content, ok := result["content"].(string)
	if ok {
		metadata["content_length"] = len(content)
	}

	// Increment processed count atomically
	atomic.AddInt64(&pdfMetadataState.processedCount, 1)
	metadata["pdf_processor_version"] = "1.0.0"

	result["metadata"] = metadata

	// Serialize back to JSON
	outputJSON, err := json.Marshal(result)
	if err != nil {
		return C.CString("{\"error\":\"Failed to serialize result\"}")
	}

	return C.CString(string(outputJSON))
}

func main() {
	// Register the post-processor with priority 80, early stage
	if err := kreuzberg.RegisterPostProcessor("pdf_metadata_extractor", 80,
		(C.PostProcessorCallback)(C.pdfMetadataExtractor)); err != nil {
		log.Fatalf("failed to register post-processor: %v", err)
	}
	defer func() {
		if err := kreuzberg.UnregisterPostProcessor("pdf_metadata_extractor"); err != nil {
			log.Printf("warning: failed to unregister post-processor: %v", err)
		}

		log.Printf("Total PDFs processed: %d", atomic.LoadInt64(&pdfMetadataState.processedCount))
	}()

	// Extract PDF document
	result, err := kreuzberg.ExtractFileSync("document.pdf", nil)
	if err != nil {
		log.Fatalf("extraction failed: %v", err)
	}

	log.Printf("PDF MIME type: %s", result.MimeType)

	// Parse and display metadata
	var metadata map[string]interface{}
	if metaJSON, ok := result.MetadataJSON.(string); ok {
		if err := json.Unmarshal([]byte(metaJSON), &metadata); err == nil {
			if pdfProcessed, ok := metadata["pdf_processed"].(bool); ok && pdfProcessed {
				log.Printf("PDF metadata extracted successfully")
				if contentLen, ok := metadata["content_length"].(float64); ok {
					log.Printf("Content length: %.0f bytes", contentLen)
				}
			}
		}
	}
}
```
