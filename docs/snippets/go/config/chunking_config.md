```go title="Go"
package main

import (
	"fmt"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	maxChars := 1000
	maxOverlap := 200
	config := &kreuzberg.ExtractionConfig{
		Chunking: &kreuzberg.ChunkingConfig{
			MaxChars:   &maxChars,
			MaxOverlap: &maxOverlap,
		},
	}

	fmt.Printf("Config: MaxChars=%d, MaxOverlap=%d\n", *config.Chunking.MaxChars, *config.Chunking.MaxOverlap)
}
```
