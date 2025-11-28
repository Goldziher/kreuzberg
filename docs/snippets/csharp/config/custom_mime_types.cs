using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true
};

var result = KreuzbergClient.ExtractBytesSync(
    new BytesWithMime(fileBytes, "application/pdf"),
    config
);

var mimeType = result.MimeType;
