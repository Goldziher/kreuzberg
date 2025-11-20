```csharp
using Kreuzberg;

try
{
    var result = KreuzbergClient.ExtractFileSync("missing.pdf");
    Console.WriteLine(result.Content);
}
catch (KreuzbergValidationException ex)
{
    Console.Error.WriteLine($"Validation error: {ex.Message}");
}
catch (KreuzbergIOException ex)
{
    Console.Error.WriteLine($"IO error: {ex.Message}");
    throw;
}
catch (KreuzbergException ex)
{
    Console.Error.WriteLine($"Extraction failed: {ex.Message}");
    throw;
}
```
