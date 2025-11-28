```csharp
using Kreuzberg;
using System;
using System.Collections.Generic;

// Single language detection
var config = new ExtractionConfig
{
    LanguageDetection = new LanguageDetectionConfig
    {
        Enabled = true,
        MinConfidence = 0.9,
        DetectMultiple = false
    }
};

var result = KreuzbergClient.ExtractFileSync("document.pdf", config);

Console.WriteLine("Detected Language:");
foreach (var lang in result.DetectedLanguages)
{
    Console.WriteLine($"  - {lang}");
}

// Multiple language detection
var multiLangConfig = new ExtractionConfig
{
    LanguageDetection = new LanguageDetectionConfig
    {
        Enabled = true,
        MinConfidence = 0.8,
        DetectMultiple = true
    }
};

var multiResult = KreuzbergClient.ExtractFileSync("multilingual_document.pdf", multiLangConfig);

Console.WriteLine("Detected Languages:");
foreach (var lang in multiResult.DetectedLanguages)
{
    Console.WriteLine($"  - {lang}");
}

// Language detection with metadata
Console.WriteLine($"\nLanguage Detection Summary:");
Console.WriteLine($"  - Content: {multiResult.Content.Substring(0, 100)}...");
Console.WriteLine($"  - Languages: {string.Join(", ", multiResult.DetectedLanguages)}");
Console.WriteLine($"  - Quality Score: {multiResult.Metadata.QualityScore}");
```
