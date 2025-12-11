```go
package main

import (
	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

maxKeywords := int32(10)
minScore := 0.3
language := "en"

config := &kreuzberg.ExtractionConfig{
	Keywords: &kreuzberg.KeywordConfig{
		Algorithm:   kreuzberg.KeywordAlgorithm_YAKE,
		MaxKeywords: &maxKeywords,
		MinScore:    &minScore,
		Language:    &language,
	},
}
```
