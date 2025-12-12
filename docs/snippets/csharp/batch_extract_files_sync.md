```csharp title="C#"
using Kreuzberg;

var files = new[] { "doc1.pdf", "doc2.docx", "doc3.pptx" };
var results = KreuzbergClient.BatchExtractFilesSync(files, new ExtractionConfig());

foreach (var result in results)
{
    Console.WriteLine($"Content length: {result.Content.Length}");
}
```
