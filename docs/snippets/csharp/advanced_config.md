```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig { Backend = "tesseract", Language = "eng+deu" },
    Chunking = new ChunkingConfig { MaxChars = 1000, MaxOverlap = 100 },
    TokenReduction = new TokenReductionConfig { Enabled = true },
    LanguageDetection = new LanguageDetectionConfig
    {
        Enabled = true,
        DetectMultiple = true
    },
    UseCache = true,
    EnableQualityProcessing = true
};

var result = KreuzbergClient.ExtractFileSync("document.pdf", config);

foreach (var chunk in result.Chunks)
{
    Console.WriteLine($"Chunk: {chunk.Content[..Math.Min(100, chunk.Content.Length)]}");
}

if (result.DetectedLanguages?.Count > 0)
{
    Console.WriteLine($"Languages: {string.Join(", ", result.DetectedLanguages)}");
}
```
