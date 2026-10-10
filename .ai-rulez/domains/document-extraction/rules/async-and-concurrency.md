---
type: Decision
title: Async And Concurrency
x-ai-rulez:
  kind: rule
  id: async-and-concurrency
  domain: document-extraction
  metadata:
    priority: high
---

- All extraction paths must be fully async using tokio
- Never block the async runtime — use spawn_blocking for CPU-intensive work
- All public types must be Send + Sync
- Use `tokio::time::timeout` for timeout handling on extraction operations. `tokio::select!` is used only in the MCP server and API startup paths, never on an extraction path.
- Cross-platform: test on Linux (amd64, arm64) and macOS at minimum
