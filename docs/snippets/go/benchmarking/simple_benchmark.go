```go
package main

import (
	"fmt"
	"sync"
	"time"

	"kreuzberg"
)

func main() {
	config := &kreuzberg.ExtractionConfig{
		UseCache: false,
	}
	client, err := kreuzberg.New(config)
	if err != nil {
		panic(err)
	}
	defer client.Close()

	filePath := "document.pdf"
	numRuns := 10

	fmt.Printf("Sync extraction (%d runs):\n", numRuns)
	start := time.Now()
	for i := 0; i < numRuns; i++ {
		_, err := client.ExtractFile(filePath)
		if err != nil {
			panic(err)
		}
	}
	syncDuration := time.Since(start).Seconds()
	avgSync := syncDuration / float64(numRuns)
	fmt.Printf("  - Total time: %.3fs\n", syncDuration)
	fmt.Printf("  - Average: %.3fs per extraction\n", avgSync)

	fmt.Printf("\nAsync extraction (%d parallel runs):\n", numRuns)
	start = time.Now()
	var wg sync.WaitGroup
	wg.Add(numRuns)
	for i := 0; i < numRuns; i++ {
		go func() {
			defer wg.Done()
			_, err := client.ExtractFile(filePath)
			if err != nil {
				panic(err)
			}
		}()
	}
	wg.Wait()
	asyncDuration := time.Since(start).Seconds()
	fmt.Printf("  - Total time: %.3fs\n", asyncDuration)
	fmt.Printf("  - Average: %.3fs per extraction\n", asyncDuration/float64(numRuns))
	fmt.Printf("  - Speedup: %.1fx\n", syncDuration/asyncDuration)

	cacheConfig := &kreuzberg.ExtractionConfig{
		UseCache: true,
	}
	clientCached, err := kreuzberg.New(cacheConfig)
	if err != nil {
		panic(err)
	}
	defer clientCached.Close()

	fmt.Println("\nFirst extraction (populates cache)...")
	start = time.Now()
	_, err = clientCached.ExtractFile(filePath)
	if err != nil {
		panic(err)
	}
	firstDuration := time.Since(start).Seconds()
	fmt.Printf("  - Time: %.3fs\n", firstDuration)

	fmt.Println("Second extraction (from cache)...")
	start = time.Now()
	_, err = clientCached.ExtractFile(filePath)
	if err != nil {
		panic(err)
	}
	cachedDuration := time.Since(start).Seconds()
	fmt.Printf("  - Time: %.3fs\n", cachedDuration)
	fmt.Printf("  - Cache speedup: %.1fx\n", firstDuration/cachedDuration)
}
```
