```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    TokenReduction = new TokenReductionConfig
    {
        Mode = "moderate",
        PreserveMarkdown = true
    }
};

var result = await KreuzbergClient.ExtractFileAsync(
    "verbose_document.pdf",
    config
);

var original = result.Metadata.ContainsKey("original_token_count")
    ? (int)result.Metadata["original_token_count"]
    : 0;

var reduced = result.Metadata.ContainsKey("token_count")
    ? (int)result.Metadata["token_count"]
    : 0;

var ratio = result.Metadata.ContainsKey("token_reduction_ratio")
    ? (double)result.Metadata["token_reduction_ratio"]
    : 0.0;

Console.WriteLine($"Reduced from {original} to {reduced} tokens");
Console.WriteLine($"Reduction: {ratio * 100:F1}%");
```
