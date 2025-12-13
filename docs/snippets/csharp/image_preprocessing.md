```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        TesseractConfig = new TesseractConfig
        {
            Preprocessing = new ImagePreprocessingConfig
            {
                TargetDpi = 300,
                Denoise = true,
                Deskew = true,
                ContrastEnhance = true,
                BinarizationMethod = "otsu"
            }
        }
    }
};

var result = await KreuzbergClient.ExtractFileAsync("scanned.pdf", config);
Console.WriteLine($"Content: {result.Content[..Math.Min(100, result.Content.Length)]}");
```
