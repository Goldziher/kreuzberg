```go
package main

import (
	"fmt"
	"os/exec"
)

func extractWithCli(filePath string, outputFormat string) (string, error) {
	cmd := exec.Command("kreuzberg", "extract", filePath, "--format", outputFormat)

	output, err := cmd.CombinedOutput()
	if err != nil {
		return "", fmt.Errorf("CLI error: %w, output: %s", err, string(output))
	}

	return string(output), nil
}

func main() {
	document := "document.pdf"

	textOutput, err := extractWithCli(document, "text")
	if err != nil {
		panic(err)
	}
	fmt.Printf("Extracted: %d characters\n", len(textOutput))

	jsonOutput, err := extractWithCli(document, "json")
	if err != nil {
		panic(err)
	}
	fmt.Printf("JSON output received: %d bytes\n", len(jsonOutput))
}
```
