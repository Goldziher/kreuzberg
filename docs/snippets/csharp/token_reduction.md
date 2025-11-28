```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    TokenReduction = new TokenReductionConfig
    {
        Mode = "moderate",
        PreserveImportantWords = true
    }
};

var result = await KreuzbergClient.ExtractFileAsync("document.pdf", config);
Console.WriteLine($"Content length: {result.Content.Length}");
```
