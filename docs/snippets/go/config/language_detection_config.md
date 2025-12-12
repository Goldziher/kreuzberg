```go title="Go"
package main

import (
	"fmt"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	minConfidence := 0.8
	config := &kreuzberg.ExtractionConfig{
		LanguageDetection: &kreuzberg.LanguageDetectionConfig{
			Enabled:        true,
			MinConfidence:  &minConfidence,
			DetectMultiple: false,
		},
	}

	fmt.Printf("Language detection enabled: %v\n", config.LanguageDetection.Enabled)
	fmt.Printf("Min confidence: %f\n", *config.LanguageDetection.MinConfidence)
}
```
