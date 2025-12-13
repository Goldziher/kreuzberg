```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Language = "eng+fra+deu",
        TesseractConfig = new TesseractConfig
        {
            Psm = 6,
            Oem = 1,
            MinConfidence = 0.8m,
            EnableTableDetection = true
        }
    }
};

var result = await KreuzbergClient.ExtractFileAsync("document.pdf", config);
Console.WriteLine($"Content: {result.Content[..Math.Min(100, result.Content.Length)]}");
```
