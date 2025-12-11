```csharp
using Kreuzberg;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;

public class VectorDatabaseIntegration
{
    public class VectorRecord
    {
        public string Id { get; set; }
        public float[] Embedding { get; set; }
        public string Content { get; set; }
        public Dictionary<string, string> Metadata { get; set; }
    }

    public async Task<List<VectorRecord>> ExtractAndVectorize(
        string documentPath,
        string documentId)
    {
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
                    BatchSize = 32
                }
            }
        };

        var result = await Kreuzberg.ExtractFileAsync(documentPath, config);
        var chunks = result.Chunks ?? new List<Chunk>();

        var vectorRecords = chunks
            .Select((chunk, index) => new VectorRecord
            {
                Id = $"{documentId}_chunk_{index}",
                Content = chunk.Content,
                Embedding = chunk.Embedding,
                Metadata = new Dictionary<string, string>
                {
                    { "document_id", documentId },
                    { "chunk_index", index.ToString() },
                    { "content_length", chunk.Content.Length.ToString() }
                }
            })
            .ToList();

        await StoreInVectorDatabase(vectorRecords);
        return vectorRecords;
    }

    private async Task StoreInVectorDatabase(List<VectorRecord> records)
    {
        foreach (var record in records)
        {
            if (record.Embedding != null && record.Embedding.Length > 0)
            {
                Console.WriteLine(
                    $"Storing {record.Id}: {record.Content.Length} chars, " +
                    $"{record.Embedding.Length} dims");
            }
        }

        await Task.CompletedTask;
    }
}
```
