package main

import (
    "fmt"
    "kreuzberg"
)

func main() {
    config := &kreuzberg.ExtractionConfig{
        Pages: &kreuzberg.PageConfig{
            ExtractPages: true,
        },
    }

    result, err := kreuzberg.ExtractFileSync("document.pdf", config)
    if err != nil {
        panic(err)
    }

    if result.Pages != nil {
        for _, page := range result.Pages {
            fmt.Printf("Page %d:\n", page.PageNumber)
            fmt.Printf("  Content: %d chars\n", len(page.Content))
            fmt.Printf("  Tables: %d\n", len(page.Tables))
            fmt.Printf("  Images: %d\n", len(page.Images))
        }
    }
}
