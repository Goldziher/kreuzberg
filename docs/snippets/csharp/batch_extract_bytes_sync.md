```csharp title="C#"
using Kreuzberg;

var documents = new[]
{
    new BytesWithMime(await File.ReadAllBytesAsync("doc1.pdf"), "application/pdf"),
    new BytesWithMime(await File.ReadAllBytesAsync("doc2.docx"), "application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
};

var results = KreuzbergClient.BatchExtractBytesSync(documents, new ExtractionConfig());

Console.WriteLine($"Processed {results.Count} documents");
```
