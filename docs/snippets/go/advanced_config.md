```go
package main

import (
	"fmt"
	"log"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	lang := "eng+deu" // Multiple languages
	chunkSize := 1000
	chunkOverlap := 100
	useCache := true
	enableQuality := true
	detectMultiple := true

	config := &kreuzberg.ExtractionConfig{
		OCR: &kreuzberg.OCRConfig{
			Backend:  "tesseract",
			Language: &lang,
		},
		Chunking: &kreuzberg.ChunkingConfig{
			ChunkSize:    &chunkSize,
			ChunkOverlap: &chunkOverlap,
		},
		LanguageDetection: &kreuzberg.LanguageDetectionConfig{
			Enabled:        &useCache,
			DetectMultiple: &detectMultiple,
		},
		UseCache:                &useCache,
		EnableQualityProcessing: &enableQuality,
	}

	result, err := kreuzberg.ExtractFileSync("document.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	// Access chunks
	if len(result.Chunks) > 0 {
		snippet := result.Chunks[0].Content
		if len(snippet) > 100 {
			snippet = snippet[:100]
		}
		fmt.Printf("First chunk: %s...\n", snippet)
	}

	// Access detected languages
	if len(result.DetectedLanguages) > 0 {
		fmt.Printf("Languages: %v\n", result.DetectedLanguages)
	}
}
```
