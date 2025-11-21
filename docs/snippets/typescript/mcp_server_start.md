```typescript
import { spawn } from 'child_process';

const mcpProcess = spawn('kreuzberg', ['mcp']);

mcpProcess.stdout.on('data', (data) => {
  console.log(`MCP Server: ${data}`);
});

mcpProcess.stderr.on('data', (data) => {
  console.error(`MCP Error: ${data}`);
});

mcpProcess.on('error', (err) => {
  console.error(`Failed to start MCP server: ${err.message}`);
});
```
