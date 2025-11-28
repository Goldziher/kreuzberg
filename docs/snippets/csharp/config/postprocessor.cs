using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = true,
    Postprocessor = new PostProcessorConfig
    {
        Enabled = true,
        EnabledProcessors = new List<string> { "normalize_whitespace", "remove_diacritics" }
    }
};

var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
