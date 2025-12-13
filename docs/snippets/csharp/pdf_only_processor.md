```csharp title="C#"
using Kreuzberg;

public class PdfOnlyProcessor : IPostProcessor
{
    public string Name() => "pdf-only-processor";
    public string Version() => "1.0.0";

    public ExtractionResult Process(ExtractionResult result) => result;

    public bool ShouldProcess(ExtractionResult result)
        => result.MimeType == "application/pdf";
}

var processor = new PdfOnlyProcessor();
KreuzbergClient.RegisterPostProcessor(processor);
```
