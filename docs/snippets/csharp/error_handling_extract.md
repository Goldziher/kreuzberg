```csharp
using System;
using System.IO;
using System.Net.Http;
using System.Text.Json;

var client = new HttpClient();

try
{
    using (var fileStream = File.OpenRead("document.pdf"))
    {
        using (var content = new MultipartFormDataContent())
        {
            content.Add(new StreamContent(fileStream), "files", "document.pdf");

            var response = await client.PostAsync("http://localhost:8000/extract", content);

            if (!response.IsSuccessStatusCode)
            {
                var errorJson = await response.Content.ReadAsStringAsync();
                var errorDoc = JsonDocument.Parse(errorJson);
                var errorType = errorDoc.RootElement.GetProperty("error_type").GetString();
                var message = errorDoc.RootElement.GetProperty("message").GetString();

                Console.WriteLine($"Error: {errorType}: {message}");
                return;
            }

            var json = await response.Content.ReadAsStringAsync();
            Console.WriteLine($"Success: {json}");
        }
    }
}
catch (HttpRequestException e)
{
    Console.WriteLine($"Request failed: {e.Message}");
}
```
