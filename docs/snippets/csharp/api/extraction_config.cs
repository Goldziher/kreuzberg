```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true,
    ForceOcr = false,
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        Language = "eng+fra",
        TesseractConfig = new TesseractConfig
        {
            Psm = 3,
            Oem = 3,
            MinConfidence = 0.8,
            Preprocessing = new ImagePreprocessingConfig
            {
                TargetDpi = 300,
                Denoise = true,
                Deskew = true,
                ContrastEnhance = true,
            },
            EnableTableDetection = true,
        },
    },
    PdfOptions = new PdfConfig
    {
        ExtractImages = true,
        ExtractMetadata = true,
    },
    Images = new ImageExtractionConfig
    {
        ExtractImages = true,
        TargetDpi = 150,
        MaxImageDimension = 4096,
    },
    LanguageDetection = new LanguageDetectionConfig
    {
        Enabled = true,
        MinConfidence = 0.8,
        DetectMultiple = false,
    },
};
```
