```go title="Go"
package main

import (
    "log"
    "os/exec"
)

func main() {
    cmd := exec.Command("kreuzberg", "mcp")
    cmd.Stdout = log.Writer()
    cmd.Stderr = log.Writer()
    if err := cmd.Run(); err != nil {
        log.Fatalf("mcp exited: %v", err)
    }
}

```
