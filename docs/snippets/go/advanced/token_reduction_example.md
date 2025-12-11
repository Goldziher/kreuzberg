```go
package main

import (
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

preserveMarkdown := true
mode := "moderate"

config := &kreuzberg.ExtractionConfig{
	TokenReduction: &kreuzberg.TokenReductionConfig{
		Mode:             &mode,
		PreserveMarkdown: &preserveMarkdown,
	},
}

result, err := kreuzberg.ExtractFileSync("verbose_document.pdf", config)
if err != nil {
	log.Fatalf("extraction failed: %v", err)
}

original := 0
reduced := 0
ratio := 0.0

if val, ok := result.Metadata["original_token_count"]; ok {
	original = val.(int)
}

if val, ok := result.Metadata["token_count"]; ok {
	reduced = val.(int)
}

if val, ok := result.Metadata["token_reduction_ratio"]; ok {
	ratio = val.(float64)
}

fmt.Printf("Reduced from %d to %d tokens\n", original, reduced)
fmt.Printf("Reduction: %.1f%%\n", ratio*100)
```
