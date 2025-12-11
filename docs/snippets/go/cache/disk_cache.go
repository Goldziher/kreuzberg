```go
package main

import (
	"fmt"
	"os"
	"path/filepath"

	"kreuzberg"
)

func main() {
	cacheDir := filepath.Join(os.Getenv("HOME"), ".cache", "kreuzberg")
	os.MkdirAll(cacheDir, 0755)

	config := &kreuzberg.ExtractionConfig{
		UseCache: true,
		CacheConfig: &kreuzberg.CacheConfig{
			CachePath:       cacheDir,
			MaxCacheSize:    500 * 1024 * 1024,
			CacheTTLSeconds: 7 * 86400,
			EnableCompression: true,
		},
	}

	client, err := kreuzberg.New(config)
	if err != nil {
		panic(err)
	}
	defer client.Close()

	fmt.Println("First extraction (will be cached)...")
	result1, err := client.ExtractFile("document.pdf")
	if err != nil {
		panic(err)
	}
	fmt.Printf("  - Content length: %d\n", len(result1.Content))
	fmt.Printf("  - Cached: %v\n", result1.Metadata["was_cached"])

	fmt.Println("\nSecond extraction (from cache)...")
	result2, err := client.ExtractFile("document.pdf")
	if err != nil {
		panic(err)
	}
	fmt.Printf("  - Content length: %d\n", len(result2.Content))
	fmt.Printf("  - Cached: %v\n", result2.Metadata["was_cached"])

	fmt.Printf("\nResults are identical: %v\n", result1.Content == result2.Content)

	stats, err := client.GetCacheStats()
	if err != nil {
		panic(err)
	}
	fmt.Println("\nCache Statistics:")
	fmt.Printf("  - Total entries: %d\n", stats.TotalEntries)
	fmt.Printf("  - Cache size: %.1f MB\n", float64(stats.CacheSizeBytes)/1024/1024)
	fmt.Printf("  - Hit rate: %.1f%%\n", stats.HitRate*100)
}
```
