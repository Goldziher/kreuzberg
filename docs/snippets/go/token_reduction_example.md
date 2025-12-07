```go
package main

import (
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	config := &kreuzberg.ExtractionConfig{
		TokenReduction: &kreuzberg.TokenReductionConfig{
			Mode:             "moderate",
			PreserveMarkdown: true,
		},
	}

	result, err := kreuzberg.ExtractFileSync("verbose_document.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Printf("Original tokens: %v\n", result.Metadata.Additional["original_token_count"])
	fmt.Printf("Reduced tokens: %v\n", result.Metadata.Additional["token_count"])
	fmt.Printf("Reduction ratio: %v\n", result.Metadata.Additional["token_reduction_ratio"])
}
```
