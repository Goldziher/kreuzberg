```go
package main

import (
	"bufio"
	"encoding/json"
	"fmt"
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
	stdin, _ := cmd.StdinPipe()
	stdout, _ := cmd.StdoutPipe()

	cmd.Start()

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

	data, _ := json.Marshal(request)
	fmt.Fprintf(stdin, "%s\n", string(data))

	scanner := bufio.NewScanner(stdout)
	if scanner.Scan() {
		fmt.Println(scanner.Text())
	}

	cmd.Wait()
}
```
