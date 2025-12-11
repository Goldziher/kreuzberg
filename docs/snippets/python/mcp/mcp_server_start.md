```python
import subprocess
import time
from typing import Optional

mcp_process: subprocess.Popen = subprocess.Popen(
    ["python", "-m", "kreuzberg", "mcp"],
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
)

pid: Optional[int] = mcp_process.pid
print(f"MCP server started with PID: {pid}")

time.sleep(1)
print("Server is running, listening for connections")
```
