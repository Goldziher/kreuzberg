```go
package main

import (
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	result, err := kreuzberg.ExtractFileSync("document.pdf", nil)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	content := result.Content
	tables := result.Tables
	images := result.Images
	metadata := result.Metadata

	fmt.Printf("Content: %d characters\n", len(content))
	fmt.Printf("Tables: %d\n", len(tables))
	fmt.Printf("Images: %d\n", len(images))

	if metadata != nil {
		fmt.Print("Metadata keys: ")
		for key := range metadata {
			fmt.Print(key + " ")
		}
		fmt.Println()
	}
}
```
