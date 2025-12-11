```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    EnableQualityProcessing = true
};

var result = await KreuzbergClient.ExtractFileAsync(
    "document.pdf",
    config
);

var qualityScore = result.Metadata.ContainsKey("quality_score")
    ? (double)result.Metadata["quality_score"]
    : 0.0;

Console.WriteLine($"Quality score: {qualityScore:F2}");
```
