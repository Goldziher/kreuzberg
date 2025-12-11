```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    EnableQualityProcessing = true
};

var result = KreuzbergClient.ExtractFile(
    "scanned_document.pdf",
    config
);

var qualityScore = result.Metadata.ContainsKey("quality_score")
    ? (double)result.Metadata["quality_score"]
    : 0.0;

if (qualityScore < 0.5)
{
    Console.WriteLine(
        $"Warning: Low quality extraction ({qualityScore:F2})"
    );
    Console.WriteLine(
        "Consider re-scanning with higher DPI or adjusting OCR settings"
    );
}
else
{
    Console.WriteLine($"Quality score: {qualityScore:F2}");
}
```
