```csharp
using Kreuzberg;

var config = new ExtractionConfig
{
    Chunking = new ChunkingConfig
    {
        MaxChars = 1500,
        MaxOverlap = 200,
        Embedding = new EmbeddingConfig
        {
            Model = EmbeddingModelType.Preset("all-minilm-l6-v2")
        }
    }
};
```
