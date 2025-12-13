```csharp title="C#"
using Xunit;

public class CustomExtractorTests
{
    [Fact]
    public void TestCustomExtractor()
    {
        var extractor = new CustomJsonExtractor();
        var jsonData = System.Text.Encoding.UTF8.GetBytes(@"{""message"": ""Hello, world!""}");
        var config = new Dictionary<string, object>();

        var result = extractor.ExtractBytes(jsonData, "application/json", config);

        Assert.Contains("Hello, world!", (string)result["content"]);
        Assert.Equal("application/json", (string)result["mime_type"]);
    }
}
```
