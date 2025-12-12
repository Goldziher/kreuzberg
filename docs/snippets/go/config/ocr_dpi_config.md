```go title="Go"
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	targetDPI := 300
	result, err := kreuzberg.ExtractFileSync("scanned.pdf", &kreuzberg.ExtractionConfig{
		OCR: &kreuzberg.OCRConfig{
			Backend: "tesseract",
			Tesseract: &kreuzberg.TesseractConfig{
				Preprocessing: &kreuzberg.ImagePreprocessingConfig{
					TargetDPI: &targetDPI,
				},
			},
		},
	})
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Println("content length:", len(result.Content))
}
```
