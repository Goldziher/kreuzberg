```go
package main

import (
	"fmt"
	"log"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	result, err := kreuzberg.ExtractFileSync("document.pdf", nil)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Println(result.Content)
	fmt.Printf("Tables: %d\n", len(result.Tables))
	fmt.Printf("Metadata: %+v\n", result.Metadata)
}
```
