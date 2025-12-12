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
		log.Fatalf("extract pdf: %v", err)
	}

	// Access PDF metadata
	if pdf, ok := result.Metadata.PdfMetadata(); ok {
		if pdf.PageCount != nil {
			fmt.Printf("Pages: %d\n", *pdf.PageCount)
		}
		if pdf.Author != nil {
			fmt.Printf("Author: %s\n", *pdf.Author)
		}
		if pdf.Title != nil {
			fmt.Printf("Title: %s\n", *pdf.Title)
		}
	}

	// Access HTML metadata
	htmlResult, err := kreuzberg.ExtractFileSync("page.html", nil)
	if err != nil {
		log.Fatalf("extract html: %v", err)
	}
	if html, ok := htmlResult.Metadata.HTMLMetadata(); ok {
		if html.Title != nil {
			fmt.Printf("Title: %s\n", *html.Title)
		}
		if html.Description != nil {
			fmt.Printf("Description: %s\n", *html.Description)
		}
		if html.OGImage != nil {
			fmt.Printf("Open Graph Image: %s\n", *html.OGImage)
		}
	}
}
```
