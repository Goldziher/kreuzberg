```go title="Go"
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	psm := 6
	oem := 1
	minConf := 0.8
	lang := "eng+fra+deu"
	whitelist := "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,!?"

	config := &kreuzberg.ExtractionConfig{
		OCR: &kreuzberg.OCRConfig{
			Backend:  "tesseract",
			Language: &lang,
			Tesseract: &kreuzberg.TesseractConfig{
				PSM:              &psm,
				OEM:              &oem,
				MinConfidence:    &minConf,
				EnableTableDetection: kreuzberg.BoolPtr(true),
				TesseditCharWhitelist: whitelist,
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
