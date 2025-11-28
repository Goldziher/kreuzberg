```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true,
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        Language = "eng+fra",
        TesseractConfig = new TesseractConfig { Psm = 3 }
    },
    PdfOptions = new PdfConfig { ExtractImages = true },
    Chunking = new ChunkingConfig
    {
        MaxChars = 1000,
        MaxOverlap = 200,
        Embedding = new EmbeddingConfig
        {
            Model = EmbeddingModelType.Preset("all-MiniLM-L6-v2")
        }
    }
};

var result = await KreuzbergClient.ExtractFileAsync("document.pdf", config);
Console.WriteLine($"Content: {result.Content[..Math.Min(100, result.Content.Length)]}");
```
