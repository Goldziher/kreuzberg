```go
package main

import (
	"log"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	pw := []string{"password1", "password2"}
	result, err := kreuzberg.ExtractFileSync("document.pdf", &kreuzberg.ExtractionConfig{
		PdfOptions: &kreuzberg.PdfConfig{
			ExtractImages:   kreuzberg.BoolPtr(true),
			ExtractMetadata: kreuzberg.BoolPtr(true),
			Passwords:       pw,
		},
	})
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Println("content length:", len(result.Content))
}
```
