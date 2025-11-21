```go
package main

import (
	"log"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	// Register custom extractor with priority 50
	if err := kreuzberg.RegisterDocumentExtractor("custom-json-extractor", 50); err != nil {
		log.Fatalf("register extractor failed: %v", err)
	}

	result, err := kreuzberg.ExtractFileSync("document.json", nil)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}
	log.Printf("Extracted content length: %d", len(result.Content))
}
```
