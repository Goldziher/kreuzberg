```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    Keywords = new KeywordConfig
    {
        Algorithm = KeywordAlgorithm.Yake,
        MaxKeywords = 10,
        MinScore = 0.3,
        NgramRange = (1, 3),
        Language = "en"
    }
};
```
