```go
package main

import (
	"fmt"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	config := &kreuzberg.ExtractionConfig{
		Keywords: &kreuzberg.KeywordConfig{
			Algorithm:  "YAKE",
			MaxKeywords: 10,
			MinScore:   0.3,
			NgramRange: "1,3",
			Language:   "en",
		},
	}

	fmt.Printf("Keywords config: Algorithm=%s, MaxKeywords=%d, MinScore=%f\n",
		config.Keywords.Algorithm,
		config.Keywords.MaxKeywords,
		config.Keywords.MinScore)
}
```
