using Kreuzberg;

class Program
{
    static async Task Main()
    {
        // Extract from file with comprehensive error handling
        try
        {
            var result = await KreuzbergClient.ExtractFileAsync("document.pdf");
            Console.WriteLine($"Extracted {result.Content.Length} characters");
        }
        catch (KreuzbergParsingException ex)
        {
            Console.WriteLine($"Failed to parse document: {ex.Message}");
        }
        catch (KreuzbergOcrException ex)
        {
            Console.WriteLine($"OCR processing failed: {ex.Message}");
        }
        catch (KreuzbergMissingDependencyException ex)
        {
            Console.WriteLine($"Missing dependency: {ex.Message}");
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Extraction error: {ex.Message}");
        }

        // Extract from bytes with configuration
        try
        {
            var config = new ExtractionConfig();
            var pdfBytes = new byte[] { 0x25, 0x50, 0x44, 0x46 }; // %PDF

            var result = await KreuzbergClient.ExtractBytesAsync(
                pdfBytes,
                "application/pdf",
                config
            );

            var preview = result.Content.Length > 100
                ? result.Content[..100] + "..."
                : result.Content;

            Console.WriteLine($"Extracted: {preview}");
        }
        catch (KreuzbergValidationException ex)
        {
            Console.WriteLine($"Invalid configuration: {ex.Message}");
        }
        catch (KreuzbergOcrException ex)
        {
            Console.WriteLine($"OCR failed: {ex.Message}");
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Extraction failed: {ex.Message}");
        }

        // Handle specific file not found
        try
        {
            var result = await KreuzbergClient.ExtractFileAsync("nonexistent.pdf");
        }
        catch (KreuzbergIOException)
        {
            Console.WriteLine("File not found");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Unexpected error: {ex.Message}");
        }
    }
}
