```go title="Go"
package main

import (
    "log"
    "os/exec"
)

func main() {
    cmd := exec.Command("kreuzberg", "serve", "--host", "0.0.0.0", "--port", "3000")
    cmd.Stdout = log.Writer()
    cmd.Stderr = log.Writer()
    if err := cmd.Run(); err != nil {
        log.Fatalf("server exited: %v", err)
    }
}

```
