```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    Chunking = new ChunkingConfig
    {
        MaxChars = 1000,
        Embedding = new EmbeddingConfig
        {
            Model = EmbeddingModelType.Preset("all-mpnet-base-v2"),
            BatchSize = 16,
            Normalize = true,
            ShowDownloadProgress = true
        }
    }
};
```
