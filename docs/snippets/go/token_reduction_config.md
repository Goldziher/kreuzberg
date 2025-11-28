```go
package main

import (
	"fmt"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	config := &kreuzberg.ExtractionConfig{
		TokenReduction: &kreuzberg.TokenReductionConfig{
			Mode:                   "moderate",
			PreserveImportantWords: kreuzberg.BoolPtr(true),
		},
	}

	fmt.Printf("Mode: %s, Preserve Important Words: %v\n",
		config.TokenReduction.Mode,
		*config.TokenReduction.PreserveImportantWords)
}
```
