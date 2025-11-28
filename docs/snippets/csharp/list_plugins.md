```csharp
using Kreuzberg;

var extractors = KreuzbergClient.ListDocumentExtractors();
var processors = KreuzbergClient.ListPostProcessors();
var ocrBackends = KreuzbergClient.ListOcrBackends();
var validators = KreuzbergClient.ListValidators();

Console.WriteLine($"Extractors: {string.Join(", ", extractors)}");
Console.WriteLine($"Processors: {string.Join(", ", processors)}");
Console.WriteLine($"OCR backends: {string.Join(", ", ocrBackends)}");
Console.WriteLine($"Validators: {string.Join(", ", validators)}");
```
