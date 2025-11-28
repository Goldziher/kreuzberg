using Kreuzberg;
using System.Net.Http;

class Program
{
    static async Task Main()
    {
        using var httpClient = new HttpClient();

        try
        {
            // Download document from URL
            var url = "https://example.com/document.pdf";
            var documentBytes = await httpClient.GetByteArrayAsync(url);

            // Extract from downloaded content
            var result = await KreuzbergClient.ExtractBytesAsync(
                documentBytes,
                "application/pdf"
            );

            Console.WriteLine($"Extracted from URL: {result.Content.Length} chars");

            // Extract with configuration
            var config = new ExtractionConfig
            {
                EnableQualityProcessing = true
            };

            var result2 = await KreuzbergClient.ExtractBytesAsync(
                documentBytes,
                "application/pdf",
                config
            );

            Console.WriteLine($"Quality score: {result2.Metadata["quality_score"]}");

            // Batch download and extract
            var urls = new[]
            {
                "https://example.com/doc1.pdf",
                "https://example.com/doc2.pdf",
                "https://example.com/doc3.pdf"
            };

            var downloadTasks = urls.Select(async u =>
            {
                try
                {
                    var bytes = await httpClient.GetByteArrayAsync(u);
                    return await KreuzbergClient.ExtractBytesAsync(
                        bytes,
                        "application/pdf"
                    );
                }
                catch (HttpRequestException ex)
                {
                    Console.WriteLine($"Download failed for {u}: {ex.Message}");
                    return null;
                }
            });

            var results = await Task.WhenAll(downloadTasks);

            var successCount = results.Count(r => r != null);
            Console.WriteLine($"Successfully processed {successCount} documents");
        }
        catch (HttpRequestException ex)
        {
            Console.WriteLine($"HTTP error: {ex.Message}");
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Extraction error: {ex.Message}");
        }
    }
}
