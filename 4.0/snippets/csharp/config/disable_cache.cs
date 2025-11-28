using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = false
};

var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
