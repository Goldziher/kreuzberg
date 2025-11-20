```csharp
using Kreuzberg;

var result = KreuzbergClient.ExtractFileSync("document.pdf", new ExtractionConfig());

Console.WriteLine(result.Content);
Console.WriteLine($"Tables: {result.Tables.Count}");
Console.WriteLine($"Metadata: {result.Metadata.FormatType}");
```
