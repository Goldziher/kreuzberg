using Kreuzberg;

class Program
{
    static async Task Main()
    {
        var config = new ExtractionConfig
        {
            Chunking = new ChunkingConfig
            {
                MaxChars = 1000,
                MaxOverlap = 200,
                Embedding = new EmbeddingConfig
                {
                    Model = EmbeddingModelType.Preset("all-minilm-l6-v2"),
                    Normalize = true,
                    BatchSize = 32
                }
            }
        };

        try
        {
            var result = await KreuzbergClient.ExtractFileAsync(
                "document.pdf",
                config
            ).ConfigureAwait(false);

            Console.WriteLine($"Chunks: {result.Chunks.Count}");
            foreach (var chunk in result.Chunks)
            {
                Console.WriteLine($"Content length: {chunk.Content.Length}");
                if (chunk.Embedding != null)
                {
                    Console.WriteLine($"Embedding dimensions: {chunk.Embedding.Length}");
                }
            }
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
