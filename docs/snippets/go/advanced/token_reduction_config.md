```go title="Go"
package main

import (
	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

preserveMarkdown := true
preserveCode := true
mode := "moderate"
languageHint := "eng"

config := &kreuzberg.ExtractionConfig{
	TokenReduction: &kreuzberg.TokenReductionConfig{
		Mode:              &mode,
		PreserveMarkdown:  &preserveMarkdown,
		PreserveCode:      &preserveCode,
		LanguageHint:      &languageHint,
	},
}
```
