using Kreuzberg;

class Program
{
    static async Task Main()
    {
        try
        {
            // Extract from file asynchronously
            var result = await KreuzbergClient.ExtractFileAsync("document.pdf");

            Console.WriteLine($"Content length: {result.Content.Length}");
            Console.WriteLine($"MIME type: {result.MimeType}");

            // Extract multiple files concurrently
            var tasks = new[]
            {
                KreuzbergClient.ExtractFileAsync("file1.pdf"),
                KreuzbergClient.ExtractFileAsync("file2.pdf"),
                KreuzbergClient.ExtractFileAsync("file3.pdf")
            };

            var results = await Task.WhenAll(tasks);

            foreach (var r in results)
            {
                Console.WriteLine($"Extracted {r.Content.Length} characters");
            }
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Extraction failed: {ex.Message}");
        }
    }
}
