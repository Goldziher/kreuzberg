using Kreuzberg;
using System.IO;

class Program
{
    static async Task Main()
    {
        try
        {
            // Stream processing from large file
            var filePath = "large_document.pdf";

            await ProcessLargeFileAsync(filePath);
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }

    static async Task ProcessLargeFileAsync(string filePath)
    {
        var config = new ExtractionConfig
        {
            EnableQualityProcessing = true
        };

        var result = await KreuzbergClient.ExtractFileAsync(filePath, config);

        // Process extracted content in chunks
        var contentChunks = ChunkContent(result.Content, chunkSize: 1000);

        Console.WriteLine($"Processing {contentChunks.Count} chunks");

        foreach (var (index, chunk) in contentChunks.Select((c, i) => (i, c)))
        {
            Console.WriteLine($"Chunk {index}: {chunk.Length} characters");
            // Process each chunk
            await ProcessChunkAsync(chunk);
        }
    }

    static async Task ProcessChunkAsync(string chunk)
    {
        // Simulate async processing
        var wordCount = chunk.Split(
            new[] { ' ', '\n', '\r' },
            StringSplitOptions.RemoveEmptyEntries
        ).Length;

        Console.WriteLine($"  Words: {wordCount}");

        await Task.Delay(10); // Simulate I/O
    }

    static List<string> ChunkContent(string content, int chunkSize)
    {
        var chunks = new List<string>();

        for (int i = 0; i < content.Length; i += chunkSize)
        {
            var chunk = content.Substring(
                i,
                Math.Min(chunkSize, content.Length - i)
            );
            chunks.Add(chunk);
        }

        return chunks;
    }

    // Alternative: Stream chunks from extraction result
    static async IAsyncEnumerable<string> StreamExtractedChunksAsync(
        string filePath)
    {
        var result = await KreuzbergClient.ExtractFileAsync(filePath);

        // If result has chunks with embeddings, yield them
        if (result.Chunks?.Any() == true)
        {
            foreach (var chunk in result.Chunks)
            {
                yield return chunk.Content;
                // Simulate async processing
                await Task.Yield();
            }
        }
        else
        {
            // Otherwise, create chunks from content
            var content = result.Content;
            const int chunkSize = 512;

            for (int i = 0; i < content.Length; i += chunkSize)
            {
                var chunk = content.Substring(
                    i,
                    Math.Min(chunkSize, content.Length - i)
                );
                yield return chunk;
                await Task.Yield();
            }
        }
    }

    static async Task StreamProcessingExample()
    {
        var streamEnumerator = StreamExtractedChunksAsync("document.pdf");

        int index = 0;
        await foreach (var chunk in streamEnumerator)
        {
            Console.WriteLine($"Chunk {index++}: {chunk[..50]}...");
        }
    }
}
