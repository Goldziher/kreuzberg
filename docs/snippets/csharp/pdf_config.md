```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    PdfOptions = new PdfConfig
    {
        ExtractImages = true,
        ExtractMetadata = true,
        Passwords = new List<string> { "password1", "password2" }
    }
};

var result = await KreuzbergClient.ExtractFileAsync("document.pdf", config);
Console.WriteLine($"Content: {result.Content[..Math.Min(100, result.Content.Length)]}");
```
