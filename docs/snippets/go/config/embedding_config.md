```go title="Go"
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	maxChars := 1000
	batchSize := 16

	cfg := &kreuzberg.ExtractionConfig{
		Chunking: &kreuzberg.ChunkingConfig{
			MaxChars: &maxChars,
			Embedding: &kreuzberg.EmbeddingConfig{
				Model: &kreuzberg.EmbeddingModelType{
					Type: "preset",
					Name: "all-mpnet-base-v2",
				},
				BatchSize:            &batchSize,
				Normalize:            kreuzberg.BoolPtr(true),
				ShowDownloadProgress: kreuzberg.BoolPtr(true),
			},
		},
	}

	result, err := kreuzberg.ExtractFileSync("document.pdf", cfg)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}
	log.Println("content length:", len(result.Content))
}
```
