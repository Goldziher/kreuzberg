```go title="Go"
package main

import (
	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

maxChars := 1000
maxOverlap := 200
normalize := true
batchSize := int32(32)

config := &kreuzberg.ExtractionConfig{
	Chunking: &kreuzberg.ChunkingConfig{
		MaxChars:   &maxChars,
		MaxOverlap: &maxOverlap,
		Embedding: &kreuzberg.EmbeddingConfig{
			Model:      kreuzberg.EmbeddingModelType_Preset("all-minilm-l6-v2"),
			Normalize:  &normalize,
			BatchSize:  &batchSize,
		},
	},
}
```
