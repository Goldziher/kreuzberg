using Kreuzberg;
using System.Collections.Generic;

class Program
{
    static void Main()
    {
        try
        {
            // List document extractors (built-in)
            var extractors = KreuzbergClient.ListDocumentExtractors();
            Console.WriteLine("Registered Document Extractors:");
            foreach (var extractor in extractors)
            {
                Console.WriteLine($"  - {extractor}");
            }

            // List OCR backends
            var ocrBackends = KreuzbergClient.ListOcrBackends();
            Console.WriteLine("\nRegistered OCR Backends:");
            foreach (var backend in ocrBackends)
            {
                Console.WriteLine($"  - {backend}");
            }

            // List post-processors
            var processors = KreuzbergClient.ListPostProcessors();
            Console.WriteLine("\nRegistered Post-Processors:");
            foreach (var processor in processors)
            {
                Console.WriteLine($"  - {processor}");
            }

            // List validators
            var validators = KreuzbergClient.ListValidators();
            Console.WriteLine("\nRegistered Validators:");
            foreach (var validator in validators)
            {
                Console.WriteLine($"  - {validator}");
            }

            // Register custom post-processor
            var customProcessor = new CustomPostProcessor();
            KreuzbergClient.RegisterPostProcessor(customProcessor);
            Console.WriteLine($"\nRegistered custom post-processor: {customProcessor.Name}");

            // Unregister post-processor
            KreuzbergClient.UnregisterPostProcessor(customProcessor.Name);
            Console.WriteLine($"Unregistered post-processor: {customProcessor.Name}");

            // Clear all validators
            KreuzbergClient.ClearValidators();
            Console.WriteLine("All validators cleared");
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Plugin registry error: {ex.Message}");
        }
    }
}

class CustomPostProcessor : IPostProcessor
{
    public string Name => "custom-processor";
    public int Priority => 50;

    public ExtractionResult Process(ExtractionResult result)
    {
        // Custom processing logic
        result.Content = result.Content.ToUpper();
        return result;
    }
}
