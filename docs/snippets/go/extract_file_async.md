```go
package main

import (
	"context"
	"fmt"
	"log"
	"time"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	result, err := kreuzberg.ExtractFile(ctx, "document.pdf", nil)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Println(result.Content)
}
```
