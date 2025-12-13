```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

enableQualityProcessing := true

config := &kreuzberg.ExtractionConfig{
	EnableQualityProcessing: &enableQualityProcessing,
}

result, err := kreuzberg.ExtractFileSync("scanned_document.pdf", config)
if err != nil {
	log.Fatalf("extraction failed: %v", err)
}

qualityScore := 0.0
if val, ok := result.Metadata["quality_score"]; ok {
	qualityScore = val.(float64)
}

if qualityScore < 0.5 {
	fmt.Printf("Warning: Low quality extraction (%.2f)\n", qualityScore)
	fmt.Println("Consider re-scanning with higher DPI or adjusting OCR settings")
} else {
	fmt.Printf("Quality score: %.2f\n", qualityScore)
}
```
