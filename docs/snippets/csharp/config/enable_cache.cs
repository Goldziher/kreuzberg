using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = true
};

var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
