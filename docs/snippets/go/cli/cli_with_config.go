```go
package main

import (
	"encoding/json"
	"fmt"
	"os/exec"
)

type ExtractionResult struct {
	Content   string   `json:"content"`
	Format    string   `json:"format"`
	Languages []string `json:"languages"`
}

func extractWithConfig(filePath string, configPath string) (*ExtractionResult, error) {
	cmd := exec.Command(
		"kreuzberg",
		"extract",
		filePath,
		"--config",
		configPath,
		"--format",
		"json",
	)

	output, err := cmd.CombinedOutput()
	if err != nil {
		return nil, fmt.Errorf("CLI error: %w, output: %s", err, string(output))
	}

	var result ExtractionResult
	if err := json.Unmarshal(output, &result); err != nil {
		return nil, fmt.Errorf("JSON parse error: %w", err)
	}

	return &result, nil
}

func main() {
	configFile := "kreuzberg.toml"
	document := "document.pdf"

	fmt.Printf("Extracting %s with config %s\n", document, configFile)
	result, err := extractWithConfig(document, configFile)
	if err != nil {
		panic(err)
	}

	fmt.Printf("Content length: %d\n", len(result.Content))
	fmt.Printf("Format: %s\n", result.Format)
	fmt.Printf("Languages: %v\n", result.Languages)
}
```
