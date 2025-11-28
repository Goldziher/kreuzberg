```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true
};

var result = await KreuzbergClient.ExtractFileAsync("document.pdf", config);
Console.WriteLine(result.Content);
```
