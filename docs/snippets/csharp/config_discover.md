```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig();
var result = await KreuzbergClient.ExtractFileAsync("document.pdf", config);

Console.WriteLine(result.Content[..Math.Min(100, result.Content.Length)]);
Console.WriteLine($"Total length: {result.Content.Length}");
```
