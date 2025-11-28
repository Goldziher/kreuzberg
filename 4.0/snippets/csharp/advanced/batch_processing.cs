using Kreuzberg;
using System.Collections.Generic;

class Program
{
    static async Task Main()
    {
        var config = new ExtractionConfig
        {
            UseCache = true,
            EnableQualityProcessing = true
        };

        var filePaths = new[]
        {
            "document1.pdf",
            "document2.pdf",
            "document3.pdf"
        };

        try
        {
            var batchResults = new List<ExtractionResult>();

            // Process files sequentially with configuration
            foreach (var filePath in filePaths)
            {
                var result = await KreuzbergClient.ExtractFileAsync(filePath, config);
                batchResults.Add(result);
                Console.WriteLine($"Processed {filePath}: {result.Content.Length} chars");
            }

            // Or process concurrently
            var tasks = filePaths.Select(path =>
                KreuzbergClient.ExtractFileAsync(path, config)
            ).ToArray();

            var results = await Task.WhenAll(tasks);

            var totalChars = results.Sum(r => r.Content.Length);
            Console.WriteLine($"Total extracted: {totalChars} characters");
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Batch processing error: {ex.Message}");
        }
    }
}
