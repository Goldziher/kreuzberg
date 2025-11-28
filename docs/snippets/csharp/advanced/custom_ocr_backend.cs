using Kreuzberg;
using System.Net.Http;
using System.Text.Json;

class CloudOcrBackend : IOcrBackend
{
    private readonly string _apiKey;
    private readonly HttpClient _httpClient;

    public CloudOcrBackend(string apiKey)
    {
        _apiKey = apiKey;
        _httpClient = new HttpClient();
    }

    public string Name => "cloud-ocr";

    public string Process(ReadOnlySpan<byte> imageBytes, OcrConfig? config)
    {
        // Since we need async HTTP, we use Task.Run to bridge sync/async
        return Task.Run(async () =>
        {
            try
            {
                var bytes = imageBytes.ToArray();
                using var content = new MultipartFormDataContent();
                content.Add(new ByteArrayContent(bytes), "image");

                var request = new HttpRequestMessage(
                    HttpMethod.Post,
                    "https://api.example.com/ocr"
                )
                {
                    Content = content,
                    Headers =
                    {
                        { "Authorization", $"Bearer {_apiKey}" }
                    }
                };

                var response = await _httpClient.SendAsync(request);
                response.EnsureSuccessStatusCode();

                var jsonContent = await response.Content.ReadAsStringAsync();
                return jsonContent;
            }
            catch (HttpRequestException ex)
            {
                throw new KreuzbergOcrException($"Cloud OCR service error: {ex.Message}");
            }
        }).GetAwaiter().GetResult();
    }

    public void Dispose()
    {
        _httpClient?.Dispose();
    }
}

class Program
{
    static void Main()
    {
        using var backend = new CloudOcrBackend("your-api-key");
        KreuzbergClient.RegisterOcrBackend(backend);

        try
        {
            var config = new ExtractionConfig
            {
                Ocr = new OcrConfig
                {
                    Backend = "cloud-ocr"
                }
            };

            var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
            Console.WriteLine($"OCR text: {result.Content}");
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
