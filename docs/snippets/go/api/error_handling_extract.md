```go title="Go"
package main

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
)

func main() {
	resp, err := http.Post("http://localhost:8000/extract", "application/json", nil)
	if err != nil {
		fmt.Println("Request failed:", err)
		return
	}
	defer resp.Body.Close()

	if resp.StatusCode >= 400 {
		var error map[string]interface{}
		body, _ := io.ReadAll(resp.Body)
		json.Unmarshal(body, &error)
		fmt.Printf("Error: %v: %v\n", error["error_type"], error["message"])
	} else {
		var results []map[string]interface{}
		body, _ := io.ReadAll(resp.Body)
		json.Unmarshal(body, &results)
		// Process results
	}
}
```
