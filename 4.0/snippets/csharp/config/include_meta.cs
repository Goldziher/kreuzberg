using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = true,
    Ocr = new OcrConfig
    {
        Backend = "tesseract",
        Language = "eng"
    }
};

var result = KreuzbergClient.ExtractFileSync("document.pdf", config);

if (result.Metadata != null)
{
    var language = result.Metadata.Language;
    var format = result.Metadata.FormatType;
}
