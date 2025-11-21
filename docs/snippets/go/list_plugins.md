```go
package main

import (
	"fmt"
	"log"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	// List all registered document extractors
	extractors, err := kreuzberg.ListDocumentExtractors()
	if err != nil {
		log.Fatalf("list document extractors: %v", err)
	}
	fmt.Println("Document Extractors:")
	for _, extractor := range extractors {
		fmt.Printf("  - %s\n", extractor)
	}

	// List all registered post-processors
	processors, err := kreuzberg.ListPostProcessors()
	if err != nil {
		log.Fatalf("list post processors: %v", err)
	}
	fmt.Println("\nPost-Processors:")
	for _, processor := range processors {
		fmt.Printf("  - %s\n", processor)
	}

	// List all registered OCR backends
	backends, err := kreuzberg.ListOCRBackends()
	if err != nil {
		log.Fatalf("list OCR backends: %v", err)
	}
	fmt.Println("\nOCR Backends:")
	for _, backend := range backends {
		fmt.Printf("  - %s\n", backend)
	}

	// List all registered validators
	validators, err := kreuzberg.ListValidators()
	if err != nil {
		log.Fatalf("list validators: %v", err)
	}
	fmt.Println("\nValidators:")
	for _, validator := range validators {
		fmt.Printf("  - %s\n", validator)
	}
}
```
