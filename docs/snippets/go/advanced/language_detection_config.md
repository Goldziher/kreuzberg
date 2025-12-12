```go title="Go"
package main

import (
	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

enabled := true
detectMultiple := false
minConfidence := 0.8

config := &kreuzberg.ExtractionConfig{
	LanguageDetection: &kreuzberg.LanguageDetectionConfig{
		Enabled:         &enabled,
		MinConfidence:   &minConfidence,
		DetectMultiple:  &detectMultiple,
	},
}
```
