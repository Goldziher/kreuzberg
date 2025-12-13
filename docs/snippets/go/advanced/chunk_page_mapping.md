package main

import (
    "fmt"
    "kreuzberg"
)

func main() {
    config := &kreuzberg.ExtractionConfig{
        Chunking: &kreuzberg.ChunkingConfig{
            ChunkSize: 500,
            Overlap:   50,
        },
        Pages: &kreuzberg.PageConfig{
            ExtractPages: true,
        },
    }

    result, _ := kreuzberg.ExtractFileSync("document.pdf", config)

    if result.Chunks != nil {
        for _, chunk := range result.Chunks {
            if chunk.Metadata.FirstPage != nil {
                pageRange := fmt.Sprintf("Page %d", *chunk.Metadata.FirstPage)
                if *chunk.Metadata.FirstPage != *chunk.Metadata.LastPage {
                    pageRange = fmt.Sprintf("Pages %d-%d",
                        *chunk.Metadata.FirstPage,
                        *chunk.Metadata.LastPage)
                }

                fmt.Printf("Chunk: %s... (%s)\n", chunk.Text[:50], pageRange)
            }
        }
    }
}
