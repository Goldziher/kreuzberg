package main

import (
    "fmt"
    "kreuzberg"
)

func main() {
    result, _ := kreuzberg.ExtractFileSync("document.pdf", nil)

    if result.Metadata.Pages != nil && result.Metadata.Pages.Boundaries != nil {
        contentBytes := []byte(result.Content)

        for i, boundary := range result.Metadata.Pages.Boundaries {
            if i >= 3 {
                break
            }

            pageBytes := contentBytes[boundary.ByteStart:boundary.ByteEnd]
            pageText := string(pageBytes)

            fmt.Printf("Page %d:\n", boundary.PageNumber)
            fmt.Printf("  Byte range: %d-%d\n", boundary.ByteStart, boundary.ByteEnd)
            fmt.Printf("  Preview: %s...\n", pageText[:100])
        }
    }
}
