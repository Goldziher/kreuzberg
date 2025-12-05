```go
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	lang := "eng+deu+fra"
	result, err := kreuzberg.ExtractFileSync("multilingual.pdf", &kreuzberg.ExtractionConfig{
		OCR: &kreuzberg.OCRConfig{
			Backend:  "tesseract",
			Language: &lang,
		},
	})
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Println(result.Content)
}
```
