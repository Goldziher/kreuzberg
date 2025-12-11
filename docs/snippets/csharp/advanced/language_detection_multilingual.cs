using Kreuzberg;

class Program
{
    static async Task Main()
    {
        var config = new ExtractionConfig
        {
            LanguageDetection = new LanguageDetectionConfig
            {
                Enabled = true,
                MinConfidence = 0.8m,
                DetectMultiple = true
            }
        };

        try
        {
            var result = await KreuzbergClient.ExtractFileAsync("multilingual_document.pdf", config);

            var languages = result.DetectedLanguages ?? new List<string>();

            if (languages.Count > 0)
            {
                Console.WriteLine($"Detected {languages.Count} language(s): {string.Join(", ", languages)}");
            }
            else
            {
                Console.WriteLine("No languages detected");
            }

            Console.WriteLine($"Total content: {result.Content.Length} characters");
            Console.WriteLine($"MIME type: {result.MimeType}");
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Processing failed: {ex.Message}");
        }
    }
}
