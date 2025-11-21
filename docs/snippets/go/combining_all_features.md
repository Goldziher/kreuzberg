```go
package main

import (
	"fmt"
	"log"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	maxChars := 512
	maxOverlap := 50
	minConfidence := 0.8
	config := &kreuzberg.ExtractionConfig{
		EnableQualityProcessing: true,

		LanguageDetection: &kreuzberg.LanguageDetectionConfig{
			Enabled:        true,
			MinConfidence:  &minConfidence,
			DetectMultiple: true,
		},

		TokenReduction: &kreuzberg.TokenReductionConfig{
			Mode:             "moderate",
			PreserveMarkdown: true,
		},

		Chunking: &kreuzberg.ChunkingConfig{
			MaxChars:   &maxChars,
			MaxOverlap: &maxOverlap,
			Embedding: &kreuzberg.EmbeddingConfig{
				Model:     "balanced",
				Normalize: true,
			},
		},

		Keywords: &kreuzberg.KeywordConfig{
			Algorithm:   "YAKE",
			MaxKeywords: 10,
		},
	}

	result, err := kreuzberg.ExtractFileSync("document.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Printf("Quality: %v\n", result.Metadata.Additional["quality_score"])
	fmt.Printf("Languages: %v\n", result.DetectedLanguages)
	fmt.Printf("Keywords: %v\n", result.Metadata.Additional["keywords"])
	if result.Chunks != nil && len(result.Chunks) > 0 && result.Chunks[0].Embedding != nil {
		fmt.Printf("Chunks: %d with %d dimensions\n", len(result.Chunks), len(result.Chunks[0].Embedding))
	}
}
```
