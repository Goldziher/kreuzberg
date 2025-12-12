```go title="Go"
package main

import (
	"encoding/json"
	"log"
	"sync"
	"unsafe"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

/*
#cgo CFLAGS: -I${SRCDIR}/../../../crates/kreuzberg-ffi
#cgo LDFLAGS: -L${SRCDIR}/../../../target/release -L${SRCDIR}/../../../target/debug -lkreuzberg_ffi
#include "../../../crates/kreuzberg-ffi/kreuzberg.h"
#include <stdlib.h>
*/
import "C"

// PluginState manages thread-safe state for the stateful plugin
type PluginState struct {
	mu           sync.Mutex
	callCount    int
	cache        map[string]string
	lastMimeType string
}

// globalState holds the plugin's persistent state across calls
var globalState = &PluginState{
	cache: make(map[string]string),
}

// statefulPlugin demonstrates a thread-safe plugin with persistent state
//export statefulPlugin
func statefulPlugin(resultJSON *C.char) *C.char {
	jsonStr := C.GoString(resultJSON)
	var result map[string]interface{}

	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		return C.CString("{\"error\":\"Failed to parse result JSON\"}")
	}

	// Acquire lock to safely modify state
	globalState.mu.Lock()
	defer globalState.mu.Unlock()

	// Increment call counter
	globalState.callCount++

	// Extract and store MIME type
	if mimeType, ok := result["mime_type"].(string); ok {
		globalState.lastMimeType = mimeType
		globalState.cache[mimeType] = "processed"
	}

	// Ensure metadata exists
	metadata, ok := result["metadata"].(map[string]interface{})
	if !ok {
		metadata = make(map[string]interface{})
	}

	// Add state information to metadata
	metadata["plugin_call_count"] = globalState.callCount
	metadata["last_mime_type"] = globalState.lastMimeType
	metadata["cached_types_count"] = len(globalState.cache)
	metadata["plugin_version"] = "1.0.0"

	result["metadata"] = metadata

	// Serialize back to JSON
	outputJSON, err := json.Marshal(result)
	if err != nil {
		return C.CString("{\"error\":\"Failed to serialize result\"}")
	}

	return C.CString(string(outputJSON))
}

// GetPluginStats safely retrieves the current plugin state for logging
func GetPluginStats() (int, string, []string) {
	globalState.mu.Lock()
	defer globalState.mu.Unlock()

	callCount := globalState.callCount
	lastMime := globalState.lastMimeType

	mimeTypes := make([]string, 0, len(globalState.cache))
	for mimeType := range globalState.cache {
		mimeTypes = append(mimeTypes, mimeType)
	}

	return callCount, lastMime, mimeTypes
}

// ResetPluginState clears the plugin state - useful for testing
func ResetPluginState() {
	globalState.mu.Lock()
	defer globalState.mu.Unlock()

	globalState.callCount = 0
	globalState.lastMimeType = ""
	globalState.cache = make(map[string]string)
}

func main() {
	// Register the stateful post-processor with priority 60
	if err := kreuzberg.RegisterPostProcessor("stateful_plugin", 60,
		(C.PostProcessorCallback)(C.statefulPlugin)); err != nil {
		log.Fatalf("failed to register post-processor: %v", err)
	}
	defer func() {
		if err := kreuzberg.UnregisterPostProcessor("stateful_plugin"); err != nil {
			log.Printf("warning: failed to unregister post-processor: %v", err)
		}

		// Print final statistics
		callCount, lastMime, mimeTypes := GetPluginStats()
		log.Printf("Plugin Statistics:")
		log.Printf("  Total calls: %d", callCount)
		log.Printf("  Last MIME type: %s", lastMime)
		log.Printf("  Unique MIME types processed: %d", len(mimeTypes))
		if len(mimeTypes) > 0 {
			log.Printf("  Processed types: %v", mimeTypes)
		}
	}()

	// Process multiple documents to demonstrate state accumulation
	files := []string{
		"document1.pdf",
		"document2.pdf",
		"image.png",
		"document3.txt",
	}

	for _, file := range files {
		log.Printf("Processing: %s", file)
		result, err := kreuzberg.ExtractFileSync(file, nil)
		if err != nil {
			log.Printf("  Warning: extraction failed: %v", err)
			continue
		}

		// Parse and display metadata
		var metadata map[string]interface{}
		if metaJSON, ok := result.MetadataJSON.(string); ok {
			if err := json.Unmarshal([]byte(metaJSON), &metadata); err == nil {
				if callCount, ok := metadata["plugin_call_count"].(float64); ok {
					log.Printf("  Plugin call count: %.0f", callCount)
				}
				if cachedCount, ok := metadata["cached_types_count"].(float64); ok {
					log.Printf("  Cached MIME types: %.0f", cachedCount)
				}
			}
		}
	}

	// Demonstrate thread-safe state access
	callCount, lastMime, mimeTypes := GetPluginStats()
	log.Printf("\nFinal Plugin State:")
	log.Printf("  Total calls: %d", callCount)
	log.Printf("  Last MIME type: %s", lastMime)
	log.Printf("  Processed MIME types: %v", mimeTypes)
}
```
