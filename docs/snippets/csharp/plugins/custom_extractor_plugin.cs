using Kreuzberg;
using System.Text.Json;

// NOTE: IDocumentExtractor interface is not available in C# bindings
// Custom extractors can only be registered in Rust core
// This example shows how to manually process JSON and wrap in ExtractionResult

class CustomJsonProcessor
{
    public static ExtractionResult ProcessJson(byte[] content, string mimeType)
    {
        try
        {
            var jsonContent = System.Text.Encoding.UTF8.GetString(content);
            var document = JsonDocument.Parse(jsonContent);

            var text = ExtractText(document.RootElement);

            return new ExtractionResult
            {
                Content = text,
                MimeType = mimeType,
                Metadata = new Metadata(),
                Tables = new List<Table>(),
                Success = true
            };
        }
        catch (JsonException ex)
        {
            throw new KreuzbergParsingException($"Failed to parse JSON: {ex.Message}");
        }
    }

    private static string ExtractText(JsonElement element)
    {
        return element.ValueKind switch
        {
            JsonValueKind.String => element.GetString() + "\n",
            JsonValueKind.Array => string.Concat(
                element.EnumerateArray().Select(ExtractText)
            ),
            JsonValueKind.Object => string.Concat(
                element.EnumerateObject()
                    .Select(p => ExtractText(p.Value))
            ),
            _ => ""
        };
    }
}

class Program
{
    static void Main()
    {
        try
        {
            var jsonData = new { message = "Hello, world!", timestamp = DateTime.UtcNow };
            var jsonBytes = System.Text.Encoding.UTF8.GetBytes(
                JsonSerializer.Serialize(jsonData)
            );

            // Process manually since custom extractors aren't supported in C# bindings
            var result = CustomJsonProcessor.ProcessJson(jsonBytes, "application/json");

            Console.WriteLine($"Extracted: {result.Content}");
            Console.WriteLine($"MIME type: {result.MimeType}");
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
