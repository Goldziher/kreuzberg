```go title="Go"
package main

import (
	"fmt"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

type VectorRecord struct {
	ID        string
	Embedding []float32
	Content   string
	Metadata  map[string]string
}

func extractAndVectorize(documentPath string, documentID string) ([]VectorRecord, error) {
	maxChars := 512
	maxOverlap := 50
	normalize := true
	batchSize := int32(32)

	config := &kreuzberg.ExtractionConfig{
		Chunking: &kreuzberg.ChunkingConfig{
			MaxChars:   &maxChars,
			MaxOverlap: &maxOverlap,
			Embedding: &kreuzberg.EmbeddingConfig{
				Model:     kreuzberg.EmbeddingModelType_Preset("balanced"),
				Normalize: &normalize,
				BatchSize: &batchSize,
			},
		},
	}

	result, err := kreuzberg.ExtractFileSync(documentPath, config)
	if err != nil {
		return nil, err
	}

	var vectorRecords []VectorRecord
	for index, chunk := range result.Chunks {
		record := VectorRecord{
			ID:        fmt.Sprintf("%s_chunk_%d", documentID, index),
			Content:   chunk.Content,
			Embedding: chunk.Embedding,
			Metadata: map[string]string{
				"document_id":  documentID,
				"chunk_index":  fmt.Sprintf("%d", index),
				"content_length": fmt.Sprintf("%d", len(chunk.Content)),
			},
		}
		vectorRecords = append(vectorRecords, record)
	}

	storeInVectorDatabase(vectorRecords)
	return vectorRecords, nil
}

func storeInVectorDatabase(records []VectorRecord) {
	for _, record := range records {
		if len(record.Embedding) > 0 {
			fmt.Printf("Storing %s: %d chars, %d dims\n",
				record.ID, len(record.Content), len(record.Embedding))
		}
	}
}
```
