using Kreuzberg;

var config = new ExtractionConfig
{
    Chunking = new ChunkingConfig { ChunkSize = 500, Overlap = 50 },
    Pages = new PageConfig { ExtractPages = true }
};

var result = Kreuzberg.ExtractFileSync("document.pdf", config);

if (result.Chunks != null)
{
    foreach (var chunk in result.Chunks)
    {
        if (chunk.Metadata.FirstPage.HasValue)
        {
            var pageRange = chunk.Metadata.FirstPage == chunk.Metadata.LastPage
                ? $"Page {chunk.Metadata.FirstPage}"
                : $"Pages {chunk.Metadata.FirstPage}-{chunk.Metadata.LastPage}";

            Console.WriteLine($"Chunk: {chunk.Text[..50]}... ({pageRange})");
        }
    }
}
