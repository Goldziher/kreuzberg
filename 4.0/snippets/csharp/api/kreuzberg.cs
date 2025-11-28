```csharp
using Kreuzberg;
using System.Threading.Tasks;

// Extract from file synchronously
var result = KreuzbergClient.ExtractFileSync("document.pdf");
Console.WriteLine(result.Content);

// Extract from file asynchronously
var resultAsync = await KreuzbergClient.ExtractFileAsync("document.pdf");
Console.WriteLine(resultAsync.Content);

// Extract from bytes
var data = await File.ReadAllBytesAsync("document.pdf");
var resultBytes = KreuzbergClient.ExtractBytesSync(data, "application/pdf");
Console.WriteLine(resultBytes.Content);

// Extract with custom configuration
var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true,
};
var resultConfig = await KreuzbergClient.ExtractFileAsync("document.pdf", config);
Console.WriteLine(resultConfig.Content);

// Batch extract multiple files
var files = new[] { "doc1.pdf", "doc2.pdf", "doc3.pdf" };
var results = await KreuzbergClient.BatchExtractFilesAsync(files);
foreach (var res in results)
{
    Console.WriteLine($"Extracted {res.Metadata.FormatType}: {res.Content.Length} chars");
}
```
