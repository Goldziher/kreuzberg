```go
package main

import "github.com/Goldziher/kreuzberg/packages/go/kreuzberg"

func main() {
	enabled := true
	cfg := &kreuzberg.ExtractionConfig{
		Postprocessor: &kreuzberg.PostProcessorConfig{
			Enabled:            &enabled,
			EnabledProcessors:  []string{"deduplication", "whitespace_normalization"},
			DisabledProcessors: []string{"mojibake_fix"},
		},
	}

	_ = cfg
}
```
