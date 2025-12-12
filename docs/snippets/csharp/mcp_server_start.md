```csharp title="C#"
using System;
using System.Diagnostics;
using System.Threading.Tasks;

var processInfo = new ProcessStartInfo
{
    FileName = "kreuzberg",
    Arguments = "mcp",
    UseShellExecute = false,
    RedirectStandardOutput = true,
    RedirectStandardError = true
};

var mcpProcess = Process.Start(processInfo);

Console.WriteLine($"MCP server started with PID: {mcpProcess?.Id}");
await Task.Delay(1000);
Console.WriteLine("Server is running, listening for connections");

mcpProcess?.WaitForExit();
```
