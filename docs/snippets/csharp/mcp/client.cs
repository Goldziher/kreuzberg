```csharp
using System;
using System.Diagnostics;
using System.IO;
using System.Text.Json;
using System.Threading.Tasks;

class McpClient
{
    private Process _mcpProcess;
    private StreamReader _reader;
    private StreamWriter _writer;

    public async Task StartAsync()
    {
        var processInfo = new ProcessStartInfo
        {
            FileName = "kreuzberg",
            Arguments = "mcp",
            UseShellExecute = false,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
        };

        _mcpProcess = Process.Start(processInfo);
        _reader = _mcpProcess.StandardOutput;
        _writer = _mcpProcess.StandardInput;
    }

    public async Task<string> ExtractFileAsync(string path)
    {
        var request = new
        {
            method = "tools/call",
            @params = new
            {
                name = "extract_file",
                arguments = new { path, @async = true }
            }
        };

        var jsonRequest = JsonSerializer.Serialize(request);
        await _writer.WriteLineAsync(jsonRequest);
        await _writer.FlushAsync();

        var response = await _reader.ReadLineAsync();
        var json = JsonDocument.Parse(response);
        return json.RootElement.GetProperty("result").GetProperty("content").GetString();
    }

    public void Stop()
    {
        _writer?.Dispose();
        _reader?.Dispose();
        _mcpProcess?.Kill();
    }
}

// Usage
var client = new McpClient();
await client.StartAsync();
var content = await client.ExtractFileAsync("document.pdf");
Console.WriteLine(content);
client.Stop();
```
