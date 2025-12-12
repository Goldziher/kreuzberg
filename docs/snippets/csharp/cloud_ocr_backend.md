```csharp title="C#"
using Kreuzberg;
using System.Net.Http;
using System.Text.Json;

public class CloudOcrBackend : IOcrBackend
{
    private readonly string _apiKey;
    private readonly List<string> _langs = new() { "eng", "deu", "fra" };

    public CloudOcrBackend(string apiKey)
    {
        _apiKey = apiKey;
    }

    public string Name() => "cloud-ocr";
    public string Version() => "1.0.0";
    public List<string> SupportedLanguages() => _langs;

    public Dictionary<string, object> ProcessImage(byte[] imageBytes, Dictionary<string, object> config)
    {
        using (var client = new HttpClient())
        {
            using (var form = new MultipartFormDataContent())
            {
                form.Add(new ByteArrayContent(imageBytes), "image");
                var lang = config.ContainsKey("language") ? config["language"].ToString() : "eng";
                form.Add(new StringContent(lang), "language");

                var response = client.PostAsync("https://api.example.com/ocr", form).Result;
                var json = response.Content.ReadAsStringAsync().Result;
                var doc = JsonDocument.Parse(json);
                var text = doc.RootElement.GetProperty("text").GetString();

                return new Dictionary<string, object>
                {
                    { "content", text },
                    { "mime_type", "text/plain" }
                };
            }
        }
    }

    public void Initialize() { }
    public void Shutdown() { }
}

var backend = new CloudOcrBackend(apiKey: "your-api-key");
KreuzbergClient.RegisterOcrBackend(backend);
```
