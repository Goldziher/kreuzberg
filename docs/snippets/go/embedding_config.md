```go
package main

import (
	"log"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	maxChars := 1000
	batchSize := 16
	modelName := "all-mpnet-base-v2"
	modelType := "preset"

	cfg := &kreuzberg.ExtractionConfig{
		Chunking: &kreuzberg.ChunkingConfig{
			MaxChars: &maxChars,
			Embedding: &kreuzberg.EmbeddingConfig{
				Model: &kreuzberg.EmbeddingModelType{
					Type: &modelType,
					Name: &modelName,
				},
				BatchSize: &batchSize,
				Normalize: kreuzberg.BoolPtr(true),
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
