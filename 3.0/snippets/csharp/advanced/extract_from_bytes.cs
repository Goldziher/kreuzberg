using Kreuzberg;

class Program
{
    static async Task Main()
    {
        try
        {
            // Read file into bytes
            var pdfBytes = await File.ReadAllBytesAsync("document.pdf");

            // Extract from bytes without config (uses defaults)
            var result = await KreuzbergClient.ExtractBytesAsync(
                pdfBytes,
                "application/pdf"
            );

            Console.WriteLine($"Content: {result.Content}");
            Console.WriteLine($"MIME type: {result.MimeType}");

            // Extract from bytes with custom configuration
            var config = new ExtractionConfig
            {
                UseCache = true,
                EnableQualityProcessing = true
            };

            var result2 = await KreuzbergClient.ExtractBytesAsync(
                pdfBytes,
                "application/pdf",
                config
            );

            Console.WriteLine($"Configured extraction: {result2.Content.Length} chars");

            // Extract from in-memory data
            var imageBytes = new byte[] { /* JPEG bytes */ };

            var imageResult = await KreuzbergClient.ExtractBytesAsync(
                imageBytes,
                "image/jpeg"
            );

            Console.WriteLine($"Image text: {imageResult.Content}");

            // Batch extraction from bytes
            var multipleFiles = new Dictionary<string, (byte[], string)>
            {
                { "file1", (await File.ReadAllBytesAsync("file1.pdf"), "application/pdf") },
                { "file2", (await File.ReadAllBytesAsync("file2.pdf"), "application/pdf") }
            };

            foreach (var (name, (bytes, mimeType)) in multipleFiles)
            {
                var extractResult = await KreuzbergClient.ExtractBytesAsync(
                    bytes,
                    mimeType
                );
                Console.WriteLine($"{name}: {extractResult.Content.Length} chars");
            }
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Extraction error: {ex.Message}");
        }
        catch (IOException ex)
        {
            Console.WriteLine($"File I/O error: {ex.Message}");
        }
    }
}
