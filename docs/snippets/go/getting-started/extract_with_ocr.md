```go
package main

import (
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	backend := "tesseract"
	language := "eng"

	ocrConfig := &kreuzberg.OCRConfig{
		Backend:  &backend,
		Language: &language,
	}

	config := &kreuzberg.ExtractionConfig{
		OCR: ocrConfig,
	}

	result, err := kreuzberg.ExtractFileSync("scanned.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Println("Extracted text from scanned document:")
	fmt.Println(result.Content)
	fmt.Println("Used OCR backend: tesseract")
}
```
