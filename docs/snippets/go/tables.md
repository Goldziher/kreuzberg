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

	// Iterate over tables
	for _, table := range result.Tables {
		fmt.Printf("Table with %d rows\n", len(table.Cells))
		fmt.Println(table.Markdown) // Markdown representation

		// Access cells
		for _, row := range table.Cells {
			fmt.Println(row)
		}
	}
}
```
