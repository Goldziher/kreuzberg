```go title="Go"
package main

import (
	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

enableQualityProcessing := true

config := &kreuzberg.ExtractionConfig{
	EnableQualityProcessing: &enableQualityProcessing,
}
```
