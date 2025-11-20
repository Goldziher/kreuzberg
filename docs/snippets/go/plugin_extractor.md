```go
package main

import (
	"context"
	"fmt"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

type CustomExtractor struct{}

func (c *CustomExtractor) Name() string   { return "custom-json" }
func (c *CustomExtractor) Version() string { return "1.0.0" }

func (c *CustomExtractor) Supports(mimeType string) bool {
	return mimeType == "application/custom+json"
}

func (c *CustomExtractor) Extract(ctx context.Context, input kreuzberg.ExtractorInput) (kreuzberg.ExtractorOutput, error) {
	return kreuzberg.ExtractorOutput{
		Content: fmt.Sprintf("file: %s", input.Path),
		Mime:    "application/custom+json",
	}, nil
}

func main() {
	priority := 60
	reg := kreuzberg.GetDocumentExtractorRegistry()
	reg.Register(&CustomExtractor{}, priority)
}
```
