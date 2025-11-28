```csharp
using Kreuzberg;

public class CustomExtractor : IDocumentExtractor
{
    public string Name() => "custom";
    public string Version() => "1.0.0";

    public Dictionary<string, object> ExtractBytes(byte[] data, string mimeType, Dictionary<string, object> config)
    {
        return new Dictionary<string, object>
        {
            { "content", "Extracted content" },
            { "mime_type", mimeType }
        };
    }
}

var extractor = new CustomExtractor();
KreuzbergClient.RegisterDocumentExtractor(extractor);
Console.WriteLine("Extractor registered");
```
