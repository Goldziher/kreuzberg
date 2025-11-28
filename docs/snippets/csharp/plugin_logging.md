```csharp
using Kreuzberg;
using Microsoft.Extensions.Logging;

public class MyPlugin
{
    private readonly ILogger _logger;

    public MyPlugin(ILogger logger)
    {
        _logger = logger;
    }

    public string Name() => "my-plugin";
    public string Version() => "1.0.0";

    public void Initialize()
    {
        _logger.LogInformation($"Initializing plugin: {Name()}");
    }

    public void Shutdown()
    {
        _logger.LogInformation($"Shutting down plugin: {Name()}");
    }

    public Dictionary<string, object> ExtractBytes(
        byte[] content, string mimeType, Dictionary<string, object> config)
    {
        _logger.LogInformation($"Extracting {mimeType} ({content.Length} bytes)");
        var result = new Dictionary<string, object> { { "content", "" }, { "mime_type", mimeType } };
        if (string.IsNullOrEmpty((string?)result["content"]))
        {
            _logger.LogWarning("Extraction resulted in empty content");
        }
        return result;
    }
}
```
