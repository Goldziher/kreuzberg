```csharp
using Kreuzberg;

var result = await KreuzbergClient.ExtractFileAsync("document.pdf");

Console.WriteLine(result.Content);
Console.WriteLine(result.MimeType);
```
