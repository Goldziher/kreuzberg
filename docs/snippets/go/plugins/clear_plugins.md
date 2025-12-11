```go
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	// Clear all plugins of a specific type
	if err := kreuzberg.ClearPostProcessors(); err != nil {
		log.Fatalf("clear post processors: %v", err)
	}
	log.Println("Post processors cleared")

	if err := kreuzberg.ClearValidators(); err != nil {
		log.Fatalf("clear validators: %v", err)
	}
	log.Println("Validators cleared")

	if err := kreuzberg.ClearOCRBackends(); err != nil {
		log.Fatalf("clear OCR backends: %v", err)
	}
	log.Println("OCR backends cleared")

	if err := kreuzberg.ClearDocumentExtractors(); err != nil {
		log.Fatalf("clear document extractors: %v", err)
	}
	log.Println("Document extractors cleared")
}
```
