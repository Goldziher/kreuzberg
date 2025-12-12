```go title="Go"
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	targetDPI := 300
	config := &kreuzberg.ExtractionConfig{
		OCR: &kreuzberg.OCRConfig{
			Tesseract: &kreuzberg.TesseractConfig{
				Preprocessing: &kreuzberg.ImagePreprocessingConfig{
					TargetDPI:         &targetDPI,
					Denoise:           kreuzberg.BoolPtr(true),
					Deskew:            kreuzberg.BoolPtr(true),
					ContrastEnhance:   kreuzberg.BoolPtr(true),
					BinarizationMode:  kreuzberg.StringPtr("otsu"),
				},
			},
		},
	}

	result, err := kreuzberg.ExtractFileSync("document.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Println("content length:", len(result.Content))
}
```
