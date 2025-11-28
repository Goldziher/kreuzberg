```go
package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"log"
	"os/exec"
)

type MCPRequest struct {
	Method string      `json:"method"`
	Params MCPParams   `json:"params"`
}

type MCPParams struct {
	Name      string                 `json:"name"`
	Arguments map[string]interface{} `json:"arguments"`
}

func main() {
	cmd := exec.Command("kreuzberg", "mcp")
	stdin, err := cmd.StdinPipe()
	if err != nil {
		log.Fatalf("create stdin pipe: %v", err)
	}
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		log.Fatalf("create stdout pipe: %v", err)
	}

	if err := cmd.Start(); err != nil {
		log.Fatalf("start command: %v", err)
	}

	request := MCPRequest{
		Method: "tools/call",
		Params: MCPParams{
			Name: "extract_file",
			Arguments: map[string]interface{}{
				"path":  "document.pdf",
				"async": true,
			},
		},
	}

	data, err := json.Marshal(request)
	if err != nil {
		log.Fatalf("marshal request: %v", err)
	}
	fmt.Fprintf(stdin, "%s\n", string(data))

	scanner := bufio.NewScanner(stdout)
	if scanner.Scan() {
		fmt.Println(scanner.Text())
	}

	if err := cmd.Wait(); err != nil {
		log.Fatalf("wait for command: %v", err)
	}
}
```
