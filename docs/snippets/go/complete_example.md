```go
package main

import (
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	config := &kreuzberg.ExtractionConfig{
		UseCache:                kreuzberg.BoolPtr(true),
		EnableQualityProcessing: kreuzberg.BoolPtr(true),
		ForceOCR:                kreuzberg.BoolPtr(false),
		OCR: &kreuzberg.OCRConfig{
			Backend:   "tesseract",
			Language:  kreuzberg.StringPtr("eng+fra"),
			Tesseract: &kreuzberg.TesseractConfig{
				PSM:                  kreuzberg.IntPtr(3),
				OEM:                  kreuzberg.IntPtr(3),
				MinConfidence:        kreuzberg.FloatPtr(0.8),
				EnableTableDetection: kreuzberg.BoolPtr(true),
			},
		},
		PdfOptions: &kreuzberg.PdfConfig{
			ExtractImages:   kreuzberg.BoolPtr(true),
			ExtractMetadata: kreuzberg.BoolPtr(true),
		},
		Images: &kreuzberg.ImageExtractionConfig{
			ExtractImages:     kreuzberg.BoolPtr(true),
			TargetDPI:         kreuzberg.IntPtr(150),
			MaxImageDimension: kreuzberg.IntPtr(4096),
		},
		Chunking: &kreuzberg.ChunkingConfig{
			MaxChars:   kreuzberg.IntPtr(1000),
			MaxOverlap: kreuzberg.IntPtr(200),
		},
		TokenReduction: &kreuzberg.TokenReductionConfig{
			Mode:                   "moderate",
			PreserveImportantWords: kreuzberg.BoolPtr(true),
		},
		LanguageDetection: &kreuzberg.LanguageDetectionConfig{
			Enabled:        kreuzberg.BoolPtr(true),
			MinConfidence:  kreuzberg.FloatPtr(0.8),
			DetectMultiple: kreuzberg.BoolPtr(false),
		},
	}

	result, err := kreuzberg.ExtractFileSync("document.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Printf("Extracted content length: %d\n", len(result.Content))
}
```
