```go title="Go"
package main

import (
	"fmt"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	maxChars := 1024
	maxOverlap := 100
	batchSize := int32(32)
	config := &kreuzberg.ExtractionConfig{
		Chunking: &kreuzberg.ChunkingConfig{
			MaxChars:   &maxChars,
			MaxOverlap: &maxOverlap,
			Embedding: &kreuzberg.EmbeddingConfig{
				Model:                   "balanced",
				Normalize:               true,
				BatchSize:               &batchSize,
				ShowDownloadProgress:    false,
			},
		},
	}

	fmt.Printf("Config: MaxChars=%d, MaxOverlap=%d, Model=%s\n",
		*config.Chunking.MaxChars,
		*config.Chunking.MaxOverlap,
		config.Chunking.Embedding.Model)
}
```
