using Kreuzberg;

var config = new ExtractionConfig
{
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        Language = "eng+fra"
    }
};

var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
