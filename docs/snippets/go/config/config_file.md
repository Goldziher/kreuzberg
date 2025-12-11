```go
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	config, err := kreuzberg.LoadExtractionConfigFromFile("kreuzberg.toml")
	if err != nil {
		log.Fatalf("load config failed: %v", err)
	}

	result, err := kreuzberg.ExtractFileSync("document.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Printf("Detected MIME: %s", result.MimeType)
}
```
