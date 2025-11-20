```go
package main

import (
	"errors"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

type QualityValidator struct{}

func (v *QualityValidator) Name() string    { return "quality-validator" }
func (v *QualityValidator) Version() string { return "1.0.0" }

func (v *QualityValidator) Validate(result *kreuzberg.ExtractionResult, _ *kreuzberg.ExtractionConfig) error {
	if len(result.Content) == 0 {
		return errors.New("empty content")
	}
	return nil
}

func (v *QualityValidator) Initialize() error { return nil }
func (v *QualityValidator) Shutdown() error   { return nil }

func main() {
	reg := kreuzberg.GetValidatorRegistry()
	reg.Register(&QualityValidator{}, 50)
}
```
