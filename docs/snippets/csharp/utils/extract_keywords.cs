```csharp
using Kreuzberg;
using System;
using System.Collections.Generic;
using System.Linq;

// Extract keywords using YAKE algorithm
var config = new ExtractionConfig
{
    Keywords = new KeywordConfig
    {
        Algorithm = KeywordAlgorithm.YAKE,
        MaxKeywords = 10,
        MinScore = 0.3
    }
};

var result = KreuzbergClient.ExtractFileSync("research_paper.pdf", config);

Console.WriteLine("Extracted Keywords:");
if (result.Metadata.Keywords != null)
{
    foreach (var keyword in result.Metadata.Keywords.OrderByDescending(k => k.Score))
    {
        Console.WriteLine($"  - {keyword.Text}: {keyword.Score:F3}");
    }
}
else
{
    Console.WriteLine("  (No keywords extracted)");
}

// Extract keywords with TF-IDF
var tfidfConfig = new ExtractionConfig
{
    Keywords = new KeywordConfig
    {
        Algorithm = KeywordAlgorithm.TfIdf,
        MaxKeywords = 15,
        MinScore = 0.2
    }
};

var tfidfResult = KreuzbergClient.ExtractFileSync("document.pdf", tfidfConfig);

Console.WriteLine("\nTF-IDF Keywords:");
if (tfidfResult.Metadata.Keywords != null)
{
    var topKeywords = tfidfResult.Metadata.Keywords
        .OrderByDescending(k => k.Score)
        .Take(10)
        .ToList();

    foreach (var keyword in topKeywords)
    {
        Console.WriteLine($"  - {keyword.Text}: {keyword.Score:F3}");
    }
}

// Summary
Console.WriteLine($"\nKeyword Extraction Summary:");
Console.WriteLine($"  - Algorithm: YAKE");
Console.WriteLine($"  - Total Keywords: {result.Metadata.Keywords?.Count ?? 0}");
Console.WriteLine($"  - Top Keyword: {result.Metadata.Keywords?.FirstOrDefault()?.Text}");
```
