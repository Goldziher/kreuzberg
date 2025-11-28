```csharp
using Kreuzberg;

var result = KreuzbergClient.ExtractFileSync("document.pdf", new ExtractionConfig());

foreach (var table in result.Tables)
{
    Console.WriteLine($"Table with {table.Cells.Count} rows");
    Console.WriteLine(table.Markdown);

    foreach (var row in table.Cells)
    {
        Console.WriteLine(string.Join(" | ", row));
    }
}
```
