```go
package main

import "github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"

func main() {
	language := "eng+fra"
	psm := 3

	_ = &kreuzberg.ExtractionConfig{
		OCR: &kreuzberg.OCRConfig{
			Backend:  "tesseract",
			Language: &language,
			Tesseract: &kreuzberg.TesseractConfig{
				PSM: &psm,
			},
		},
	}
}
```
