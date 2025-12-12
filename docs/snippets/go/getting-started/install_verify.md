```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	fmt.Println("Kreuzberg CGO bindings loaded successfully")

	result, err := kreuzberg.ExtractFileSync("sample.pdf", nil)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Println("Installation verified!")
	fmt.Printf("Extracted %d characters\n", len(result.Content))
}
```
