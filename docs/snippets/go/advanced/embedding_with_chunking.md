```go
package main

import (
	"fmt"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

maxChars := 512
maxOverlap := 50
normalize := true
batchSize := int32(32)
showProgress := false

config := &kreuzberg.ExtractionConfig{
	Chunking: &kreuzberg.ChunkingConfig{
		MaxChars:   &maxChars,
		MaxOverlap: &maxOverlap,
		Embedding: &kreuzberg.EmbeddingConfig{
			Model:                 kreuzberg.EmbeddingModelType_Preset("balanced"),
			Normalize:             &normalize,
			BatchSize:             &batchSize,
			ShowDownloadProgress:  &showProgress,
		},
	},
}

result, err := kreuzberg.ExtractFileSync("document.pdf", config)
if err != nil {
	fmt.Printf("Error: %v\n", err)
	return
}

for index, chunk := range result.Chunks {
	chunkID := fmt.Sprintf("doc_chunk_%d", index)
	content := chunk.Content
	if len(content) > 50 {
		content = content[:50]
	}
	fmt.Printf("Chunk %s: %s\n", chunkID, content)

	if chunk.Embedding != nil && len(chunk.Embedding) > 0 {
		fmt.Printf("  Embedding dimensions: %d\n", len(chunk.Embedding))
	}
}
```
