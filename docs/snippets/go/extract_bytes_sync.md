```go
package main

import (
	"log"
	"os"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
)

func main() {
	data, err := os.ReadFile("document.pdf")
	if err != nil {
		log.Fatalf("read file: %v", err)
	}

	result, err := kreuzberg.ExtractBytesSync(data, "application/pdf", nil)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Println(result.Content)
}
```
