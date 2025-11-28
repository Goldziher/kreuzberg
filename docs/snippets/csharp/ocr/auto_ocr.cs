using Kreuzberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "auto",
        Language = "en"
    }
};

var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
Console.WriteLine(result.Content);
