```go
package main

import (
	"fmt"
	"log"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	config := &kreuzberg.ExtractionConfig{
		Keywords: &kreuzberg.KeywordConfig{
			Algorithm:   "YAKE",
			MaxKeywords: 10,
			MinScore:    0.3,
		},
	}

	result, err := kreuzberg.ExtractFileSync("research_paper.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	if keywords, ok := result.Metadata.Additional["keywords"]; ok {
		fmt.Printf("Keywords: %v\n", keywords)
	}
}
```
