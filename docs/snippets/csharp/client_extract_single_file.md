```csharp
using System;
using System.IO;
using System.Net.Http;

var client = new HttpClient();

using (var fileStream = File.OpenRead("document.pdf"))
{
    using (var content = new MultipartFormDataContent())
    {
        content.Add(new StreamContent(fileStream), "files", "document.pdf");

        var response = await client.PostAsync("http://localhost:8000/extract", content);
        var json = await response.Content.ReadAsStringAsync();

        Console.WriteLine(json);
    }
}
```
