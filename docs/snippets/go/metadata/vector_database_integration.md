```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	maxChars := 512
	maxOverlap := 50
	config := &kreuzberg.ExtractionConfig{
		Chunking: &kreuzberg.ChunkingConfig{
			MaxChars:   &maxChars,
			MaxOverlap: &maxOverlap,
			Embedding: &kreuzberg.EmbeddingConfig{
				Model:     "balanced",
				Normalize: true,
			},
		},
	}

	result, err := kreuzberg.ExtractFileSync("document.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	if result.Chunks != nil {
		for i, chunk := range result.Chunks {
			if chunk.Embedding != nil {
				fmt.Printf("Chunk %d: %d dimensions\n", i, len(chunk.Embedding))
				// Store in vector database
			}
		}
	}
}
```
