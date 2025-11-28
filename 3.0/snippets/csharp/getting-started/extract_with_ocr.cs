using Kreuzberg;

var config = new ExtractionConfig
{
    ForceOcr = true,
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        Language = "eng"
    }
};

var result = KreuzbergClient.ExtractFileSync("scanned.pdf", config);

Console.WriteLine(result.Content);
Console.WriteLine($"Detected Languages: {string.Join(", ", result.DetectedLanguages ?? new List<string>())}");
