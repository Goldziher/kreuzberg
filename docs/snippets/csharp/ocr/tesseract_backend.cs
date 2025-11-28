using Kreuzberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        Language = "eng+deu+fra",
        TesseractConfig = new TesseractConfig
        {
            Psm = 3
        }
    }
};

var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
Console.WriteLine(result.Content);
