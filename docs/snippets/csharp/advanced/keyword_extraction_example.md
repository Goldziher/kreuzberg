```csharp
using Kreuzberg;
using System.Collections.Generic;

var config = new ExtractionConfig
{
    Keywords = new KeywordConfig
    {
        Algorithm = KeywordAlgorithm.Yake,
        MaxKeywords = 10,
        MinScore = 0.3
    }
};

var result = await KreuzbergClient.ExtractFileAsync(
    "research_paper.pdf",
    config
);

if (result.Metadata.ContainsKey("keywords"))
{
    var keywords = (List<Dictionary<string, object>>)result.Metadata["keywords"];
    foreach (var kw in keywords)
    {
        var text = (string)kw["text"];
        var score = (double)kw["score"];
        Console.WriteLine($"{text}: {score:F3}");
    }
}
```
