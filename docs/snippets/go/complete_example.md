```go
package main

import (
	"fmt"
	"log"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	backend := "tesseract"
	language := "eng+fra"
	psm := 3
	oem := 3
	minConfidence := 0.8
	targetDpi := 300
	denoise := true
	deskew := true
	contrastEnhance := true
	enableTableDetection := true
	extractImages := true
	extractMetadata := true
	imageDpi := 150
	maxImageDimension := 4096
	maxChars := 1000
	maxOverlap := 200
	batchSize := 32
	mode := "moderate"
	preserveImportantWords := true
	enabled := true
	minLangConfidence := 0.8
	detectMultiple := false
	useCache := true
	enableQuality := true
	forceOcr := false

	config := &kreuzberg.ExtractionConfig{
		UseCache:                &useCache,
		EnableQualityProcessing: &enableQuality,
		ForceOcr:                &forceOcr,
		OCR: &kreuzberg.OCRConfig{
			Backend:   backend,
			Language:  &language,
			Tesseract: &kreuzberg.TesseractConfig{
				PSM:                  &psm,
				OEM:                  &oem,
				MinConfidence:        &minConfidence,
				EnableTableDetection: &enableTableDetection,
			},
		},
		PDFOptions: &kreuzberg.PdfConfig{
			ExtractImages:   &extractImages,
			ExtractMetadata: &extractMetadata,
		},
		ImageExtraction: &kreuzberg.ImageExtractionConfig{
			ExtractImages:     &extractImages,
			TargetDpi:         &imageDpi,
			MaxImageDimension: &maxImageDimension,
		},
		Chunking: &kreuzberg.ChunkingConfig{
			MaxChars:   &maxChars,
			MaxOverlap: &maxOverlap,
		},
		TokenReduction: &kreuzberg.TokenReductionConfig{
			Mode:                    &mode,
			PreserveImportantWords: &preserveImportantWords,
		},
		LanguageDetection: &kreuzberg.LanguageDetectionConfig{
			Enabled:        &enabled,
			MinConfidence:  &minLangConfidence,
			DetectMultiple: &detectMultiple,
		},
	}

	result, err := kreuzberg.ExtractFileSync("document.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Printf("Extracted content length: %d\n", len(result.Content))
}
```
