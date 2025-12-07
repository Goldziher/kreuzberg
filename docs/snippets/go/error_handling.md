```go
package main

import (
	"errors"
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	result, err := kreuzberg.ExtractFileSync("document.pdf", nil)
	if err != nil {
		switch {
		case errors.As(err, new(*kreuzberg.ValidationError)):
			log.Fatalf("invalid configuration: %v", err)
		case errors.As(err, new(*kreuzberg.ParsingError)):
			log.Fatalf("failed to parse document: %v", err)
		case errors.As(err, new(*kreuzberg.OCRError)):
			log.Fatalf("OCR processing failed: %v", err)
		case errors.As(err, new(*kreuzberg.MissingDependencyError)):
			log.Fatalf("missing dependency: %v", err)
		default:
			log.Fatalf("extraction error: %v", err)
		}
	}

	fmt.Println(result.Content)
}
```
