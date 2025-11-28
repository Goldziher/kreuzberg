```csharp
using Kreuzberg;

var result = KreuzbergClient.ExtractFileSync("document.pdf");

// Access extraction results
Console.WriteLine($"Content: {result.Content}");
Console.WriteLine($"MIME Type: {result.MimeType}");
Console.WriteLine($"Format Type: {result.Metadata.FormatType}");

// Access extracted tables
Console.WriteLine($"Tables found: {result.Tables.Count}");
foreach (var table in result.Tables)
{
    Console.WriteLine($"  - Rows: {table.Rows}, Columns: {table.Columns}");
}

// Access extracted images
Console.WriteLine($"Images found: {result.Images.Count}");
foreach (var image in result.Images)
{
    Console.WriteLine($"  - Size: {image.Width}x{image.Height}, Format: {image.Format}");
}

// Access metadata
Console.WriteLine($"Detected Languages: {string.Join(", ", result.DetectedLanguages)}");
Console.WriteLine($"Quality Score: {result.Metadata.QualityScore}");

// Access document information
Console.WriteLine($"Page Count: {result.Metadata.PageCount}");
Console.WriteLine($"Extraction Duration: {result.Metadata.ExtractionDurationMs}ms");
```
