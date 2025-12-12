```csharp title="C#"
using Kreuzberg;

var data = await File.ReadAllBytesAsync("document.pdf");
var result = await KreuzbergClient.ExtractBytesAsync(data, "application/pdf");

Console.WriteLine(result.Content);
Console.WriteLine(result.MimeType);
```
