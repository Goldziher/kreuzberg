```go
package main

import (
	"log"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	psm := 6
	maxChars := 1000
	maxOverlap := 200
	useCache := true

	config := &kreuzberg.ExtractionConfig{
		UseCache: &useCache,
		OCR: &kreuzberg.OCRConfig{
			Backend: "tesseract",
			Tesseract: &kreuzberg.TesseractConfig{
				PSM: &psm,
			},
		},
		Chunking: &kreuzberg.ChunkingConfig{
			MaxChars:   &maxChars,
			MaxOverlap: &maxOverlap,
		},
	}

	result, err := kreuzberg.ExtractFileSync("document.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Printf("Content length: %d", len(result.Content))
}
```
