```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    Images = new ImageExtractionConfig
    {
        ExtractImages = true,
        TargetDpi = 200,
        MaxImageDimension = 2048,
        AutoAdjustDpi = true
    }
};

var result = await KreuzbergClient.ExtractFileAsync("document.pdf", config);
Console.WriteLine($"Extracted: {result.Content[..Math.Min(100, result.Content.Length)]}");
```
