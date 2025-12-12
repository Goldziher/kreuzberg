```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	maxChars := 500
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

	result, err := kreuzberg.ExtractFileSync("research_paper.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	for i, chunk := range result.Chunks {
		fmt.Printf("Chunk %d/%d (%d-%d)\n", i+1, chunk.Metadata.TotalChunks, chunk.Metadata.CharStart, chunk.Metadata.CharEnd)
		fmt.Printf("Content: %s...\n", chunk.Content[:min(len(chunk.Content), 100)])
		if chunk.Embedding != nil {
			fmt.Printf("Embedding: %d dimensions\n", len(chunk.Embedding))
		}
	}
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}
```
