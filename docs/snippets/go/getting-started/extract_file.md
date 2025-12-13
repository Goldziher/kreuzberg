```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	useCache := true
	enableQP := true

	config := &kreuzberg.ExtractionConfig{
		UseCache:                &useCache,
		EnableQualityProcessing: &enableQP,
	}

	result, err := kreuzberg.ExtractFileSync("contract.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Printf("Extracted %d characters\n", len(result.Content))
	fmt.Printf("Quality score: %v\n", result.Metadata["quality_score"])
	fmt.Printf("Processing time: %vms\n", result.Metadata["processing_time"])
}
```
