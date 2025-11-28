using Kreuzberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "easyocr",
        Language = "en",
        UseGpu = true
    }
};

var result = KreuzbergClient.ExtractFileSync("scanned.pdf", config);
Console.WriteLine(result.Content);
