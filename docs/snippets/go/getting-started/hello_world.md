```go title="Go"
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

	fmt.Println("Extracted content:")
	if len(result.Content) > 200 {
		fmt.Println(result.Content[:200])
	} else {
		fmt.Println(result.Content)
	}
}
```
