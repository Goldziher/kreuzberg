```go
package main

import (
    "log"

    "github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
    targetDPI := 200
    maxDim := 2048
    result, err := kreuzberg.ExtractFileSync("document.pdf", &kreuzberg.ExtractionConfig{
        ImageExtraction: &kreuzberg.ImageExtractionConfig{
            ExtractImages:     kreuzberg.BoolPtr(true),
            TargetDPI:         &targetDPI,
            MaxImageDimension: &maxDim,
            AutoAdjustDPI:     kreuzberg.BoolPtr(true),
        },
    })
    if err != nil {
        log.Fatalf("extract failed: %v", err)
    }

    log.Println("content length:", len(result.Content))
}
```
