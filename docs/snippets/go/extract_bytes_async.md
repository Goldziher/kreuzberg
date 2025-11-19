```go
package main

import (
	"context"
	"log"
	"os"
	"time"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	data, err := os.ReadFile("document.pdf")
	if err != nil {
		log.Fatalf("read file: %v", err)
	}

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	result, err := kreuzberg.ExtractBytes(ctx, data, "application/pdf", nil)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}
	log.Println(result.Content)
}
```
