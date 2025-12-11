```go
package main

import (
	"fmt"
	"log"
	"strings"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

enabled := true
detectMultiple := true
minConfidence := 0.8

config := &kreuzberg.ExtractionConfig{
	LanguageDetection: &kreuzberg.LanguageDetectionConfig{
		Enabled:         &enabled,
		MinConfidence:   &minConfidence,
		DetectMultiple:  &detectMultiple,
	},
}

result, err := kreuzberg.ExtractFileSync("multilingual_document.pdf", config)
if err != nil {
	log.Fatalf("Processing failed: %v", err)
}

languages := result.DetectedLanguages
if len(languages) > 0 {
	fmt.Printf("Detected %d language(s): %s\n", len(languages), strings.Join(languages, ", "))
} else {
	fmt.Println("No languages detected")
}

fmt.Printf("Total content: %d characters\n", len(result.Content))
fmt.Printf("MIME type: %s\n", result.MimeType)
```
