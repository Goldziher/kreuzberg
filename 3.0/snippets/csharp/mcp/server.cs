```csharp
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Threading.Tasks;

class McpServer
{
    public static async Task Main(string[] args)
    {
        // Start the Kreuzberg MCP server
        var processInfo = new ProcessStartInfo
        {
            FileName = "kreuzberg",
            Arguments = "mcp",
            UseShellExecute = false,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
        };

        var process = Process.Start(processInfo);

        // Keep server running
        await Task.Delay(Timeout.Infinite);
    }
}

// Or programmatically with async support:
using System.Net.Http;
using System.Text.Json;

class McpServerProgram
{
    public static async Task Main()
    {
        // Initialize Kreuzberg MCP server
        var server = new KreuzbergMcpServer();

        // Register tools
        server.RegisterTool("extract_file", new Dictionary<string, object>
        {
            { "description", "Extract text from a document file" },
            { "parameters", new { path = "string" } }
        });

        server.RegisterTool("extract_bytes", new Dictionary<string, object>
        {
            { "description", "Extract text from document bytes" },
            { "parameters", new { data = "string", mimeType = "string" } }
        });

        // Start listening
        await server.StartAsync();
    }
}
```
