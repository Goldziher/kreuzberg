```csharp
using System;
using System.Threading.Tasks;
using Kreuzberg;

async Task RunRagPipeline()
{
    var config = new ExtractionConfig
    {
        EnableQualityProcessing = true,

        LanguageDetection = new LanguageDetectionConfig
        {
            Enabled = true,
            DetectMultiple = true,
            MinConfidence = 0.8,
        },

        TokenReduction = new TokenReductionConfig
        {
            Mode = "moderate",
            PreserveImportantWords = true,
        },

        Chunking = new ChunkingConfig
        {
            MaxChars = 512,
            MaxOverlap = 50,
            Embedding = new Dictionary<string, object?>
            {
                { "preset", "balanced" },
            },
            Enabled = true,
        },

        Keywords = new KeywordConfig
        {
            Algorithm = "yake",
            MaxKeywords = 10,
        },
    };

    var result = await KreuzbergClient.ExtractFileAsync("document.pdf", config);

    Console.WriteLine($"Content length: {result.Content.Length} characters");

    if (result.DetectedLanguages?.Count > 0)
    {
        Console.WriteLine($"Languages: {string.Join(", ", result.DetectedLanguages)}");
    }

    if (result.Chunks?.Count > 0)
    {
        Console.WriteLine($"Total chunks: {result.Chunks.Count}");
        var firstChunk = result.Chunks[0];
        Console.WriteLine($"First chunk tokens: {firstChunk.Metadata.TokenCount}");
        if (firstChunk.Embedding?.Length > 0)
        {
            Console.WriteLine($"Embedding dimensions: {firstChunk.Embedding.Length}");
        }
    }

    if (result.Metadata?.Additional?.ContainsKey("quality_score") == true)
    {
        Console.WriteLine($"Quality score: {result.Metadata.Additional["quality_score"]}");
    }

    if (result.Metadata?.Additional?.ContainsKey("keywords") == true)
    {
        Console.WriteLine($"Keywords: {result.Metadata.Additional["keywords"]}");
    }
}

await RunRagPipeline();
```
