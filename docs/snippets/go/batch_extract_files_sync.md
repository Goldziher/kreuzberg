```go
package main

import (
	"fmt"
	"log"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	files := []string{"doc1.pdf", "doc2.docx", "doc3.pptx"}

	results, err := kreuzberg.BatchExtractFilesSync(files, nil)
	if err != nil {
		log.Fatalf("batch extract failed: %v", err)
	}

	for i, result := range results {
		if result == nil {
			continue
		}
		fmt.Printf("File %d: %d characters\n", i+1, len(result.Content))
	}
}
```
