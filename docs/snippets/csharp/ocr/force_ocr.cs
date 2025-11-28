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

var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
Console.WriteLine(result.Content);
