```go
package main

import (
	"fmt"
	"log"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	minConfidence := 0.8
	config := &kreuzberg.ExtractionConfig{
		LanguageDetection: &kreuzberg.LanguageDetectionConfig{
			Enabled:        true,
			MinConfidence:  &minConfidence,
			DetectMultiple: true,
		},
	}

	result, err := kreuzberg.ExtractFileSync("multilingual_document.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Printf("Detected languages: %v\n", result.DetectedLanguages)
	// Output: [eng fra deu]
}
```
