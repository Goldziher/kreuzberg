```go
package main

import (
	"fmt"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	config := &kreuzberg.ExtractionConfig{
		TokenReduction: &kreuzberg.TokenReductionConfig{
			Mode:             "moderate",
			PreserveMarkdown: true,
			PreserveCode:     true,
			LanguageHint:     "eng",
		},
	}

	fmt.Printf("Mode: %s, Preserve Markdown: %v, Preserve Code: %v\n",
		config.TokenReduction.Mode,
		config.TokenReduction.PreserveMarkdown,
		config.TokenReduction.PreserveCode)
}
```
