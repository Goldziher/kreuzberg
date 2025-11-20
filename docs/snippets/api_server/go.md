```go
package main

import (
    "log"
    "os/exec"
)

func main() {
    cmd := exec.Command("kreuzberg", "serve", "-H", "0.0.0.0", "-p", "8000")
    cmd.Stdout = log.Writer()
    cmd.Stderr = log.Writer()
    if err := cmd.Run(); err != nil {
        log.Fatalf("failed to start server: %v", err)
    }
}

```
