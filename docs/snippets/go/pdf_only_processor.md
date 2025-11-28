```go
package main

import (
	"encoding/json"
	"log"
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

// pdfOnlyProcessor applies PDF-specific processing logic only to PDF documents
//export pdfOnlyProcessor
func pdfOnlyProcessor(resultJSON *C.char) *C.char {
	jsonStr := C.GoString(resultJSON)
	var result map[string]interface{}

	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		return C.CString("{\"error\":\"Failed to parse result JSON\"}")
	}

	// Check MIME type - only process PDFs
	mimeType, ok := result["mime_type"].(string)
	if !ok || mimeType != "application/pdf" {
		// Return unchanged for non-PDF documents
		outputJSON, err := json.Marshal(result)
		if err != nil {
			return C.CString("{\"error\":\"Failed to serialize result\"}")
		}
		return C.CString(string(outputJSON))
	}

	// Perform PDF-specific processing
	metadata, ok := result["metadata"].(map[string]interface{})
	if !ok {
		metadata = make(map[string]interface{})
	}

	// Example PDF-specific processing:
	// - Extract tables as structured data
	// - Handle PDF-specific formatting
	// - Preserve document hierarchy

	metadata["pdf_specific_processing"] = true
	metadata["processor_type"] = "pdf_only"

	// Check for tables in PDF
	if tablesJSON, ok := result["tables_json"].(string); ok && tablesJSON != "" {
		var tables []interface{}
		if err := json.Unmarshal([]byte(tablesJSON), &tables); err == nil {
			metadata["table_count"] = len(tables)
		}
	}

	result["metadata"] = metadata

	// Serialize back to JSON
	outputJSON, err := json.Marshal(result)
	if err != nil {
		return C.CString("{\"error\":\"Failed to serialize result\"}")
	}

	return C.CString(string(outputJSON))
}

func main() {
	// Register the post-processor with priority 70
	if err := kreuzberg.RegisterPostProcessor("pdf_only_processor", 70,
		(C.PostProcessorCallback)(C.pdfOnlyProcessor)); err != nil {
		log.Fatalf("failed to register post-processor: %v", err)
	}
	defer func() {
		if err := kreuzberg.UnregisterPostProcessor("pdf_only_processor"); err != nil {
			log.Printf("warning: failed to unregister post-processor: %v", err)
		}
	}()

	// Process multiple documents - processor will only affect PDFs
	files := []string{
		"document.pdf",
		"image.jpg",
		"spreadsheet.xlsx",
	}

	for _, file := range files {
		result, err := kreuzberg.ExtractFileSync(file, nil)
		if err != nil {
			log.Printf("Warning: extraction failed for %s: %v", file, err)
			continue
		}

		// Parse metadata to check if PDF processing occurred
		var metadata map[string]interface{}
		if metaJSON, ok := result.MetadataJSON.(string); ok {
			if err := json.Unmarshal([]byte(metaJSON), &metadata); err == nil {
				if pdfProcessing, ok := metadata["pdf_specific_processing"].(bool); ok && pdfProcessing {
					log.Printf("PDF-specific processing applied to: %s", file)
					if tableCount, ok := metadata["table_count"].(float64); ok {
						log.Printf("  Tables found: %.0f", tableCount)
					}
				} else {
					log.Printf("Skipped PDF processor for: %s (MIME: %s)", file, result.MimeType)
				}
			}
		}
	}
}
```
