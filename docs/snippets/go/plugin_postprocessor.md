```go
package main

import (
	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

type WordCountProcessor struct{}

func (p *WordCountProcessor) Name() string    { return "word-count" }
func (p *WordCountProcessor) Version() string { return "1.0.0" }

func (p *WordCountProcessor) ProcessingStage() string { return "early" }

func (p *WordCountProcessor) ShouldProcess(result *kreuzberg.ExtractionResult) bool {
	return result.Content != ""
}

func (p *WordCountProcessor) Process(result *kreuzberg.ExtractionResult, _ *kreuzberg.ExtractionConfig) error {
	result.Metadata["word_count"] = len(result.Content)
	return nil
}

func (p *WordCountProcessor) Initialize() error { return nil }
func (p *WordCountProcessor) Shutdown() error   { return nil }

func main() {
	reg := kreuzberg.GetPostProcessorRegistry()
	reg.Register(&WordCountProcessor{}, 50)
}
```
