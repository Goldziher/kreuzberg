```go title="Go"
package main

import (
	"fmt"
	"log"
	"os"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	files := []struct {
		Path string
		MIME string
	}{
		{"doc1.pdf", "application/pdf"},
		{"doc2.docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document"},
	}

	items := make([]kreuzberg.BytesWithMime, 0, len(files))
	for _, file := range files {
		data, err := os.ReadFile(file.Path)
		if err != nil {
			log.Fatalf("read %s: %v", file.Path, err)
		}
		items = append(items, kreuzberg.BytesWithMime{
			Data:     data,
			MimeType: file.MIME,
		})
	}

	results, err := kreuzberg.BatchExtractBytesSync(items, nil)
	if err != nil {
		log.Fatalf("batch extract failed: %v", err)
	}

	for i, result := range results {
		if result == nil {
			continue
		}
		fmt.Printf("Document %d: %d characters\n", i+1, len(result.Content))
	}
}
```
