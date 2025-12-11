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
                DetectMultiple = false
            }
        };

        try
        {
            var result = await KreuzbergClient.ExtractFileAsync("document.pdf", config);

            if (result.DetectedLanguages?.Count > 0)
            {
                Console.WriteLine($"Detected Language: {result.DetectedLanguages[0]}");
            }
            else
            {
                Console.WriteLine("No language detected");
            }

            Console.WriteLine($"Content length: {result.Content.Length} characters");
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Extraction failed: {ex.Message}");
        }
    }
}
