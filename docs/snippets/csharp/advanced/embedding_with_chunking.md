```csharp title="C#"
using Kreuzberg;
using System;
using System.Collections.Generic;
using System.Threading.Tasks;

var config = new ExtractionConfig
{
    Chunking = new ChunkingConfig
    {
        MaxChars = 512,
        MaxOverlap = 50,
        Embedding = new EmbeddingConfig
        {
            Model = EmbeddingModelType.Preset("balanced"),
            Normalize = true,
            BatchSize = 32,
            ShowDownloadProgress = false
        }
    }
};

var result = await Kreuzberg.ExtractFileAsync("document.pdf", config);

var chunks = result.Chunks ?? new List<Chunk>();
foreach (var (index, chunk) in chunks.WithIndex())
{
    var chunkId = $"doc_chunk_{index}";
    Console.WriteLine($"Chunk {chunkId}: {chunk.Content[..Math.Min(50, chunk.Content.Length)]}");

    if (chunk.Embedding != null)
    {
        Console.WriteLine($"  Embedding dimensions: {chunk.Embedding.Length}");
    }
}

internal static class EnumerableExtensions
{
    public static IEnumerable<(int Index, T Item)> WithIndex<T>(
        this IEnumerable<T> items)
    {
        var index = 0;
        foreach (var item in items)
        {
            yield return (index++, item);
        }
    }
}
```
