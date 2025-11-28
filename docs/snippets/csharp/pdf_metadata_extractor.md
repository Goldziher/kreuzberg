```csharp
using Kreuzberg;

public class PdfMetadataExtractor : IPostProcessor
{
    private int _processedCount = 0;

    public string Name() => "pdf_metadata_extractor";
    public string Version() => "1.0.0";
    public string Description() => "Extracts and enriches PDF metadata";
    public string ProcessingStage() => "early";

    public bool ShouldProcess(ExtractionResult result)
        => result.MimeType == "application/pdf";

    public ExtractionResult Process(ExtractionResult result)
    {
        _processedCount++;
        return result;
    }

    public void Initialize()
    {
        Console.WriteLine("PDF metadata extractor initialized");
    }

    public void Shutdown()
    {
        Console.WriteLine($"Processed {_processedCount} PDFs");
    }
}

var processor = new PdfMetadataExtractor();
KreuzbergClient.RegisterPostProcessor(processor);
```
