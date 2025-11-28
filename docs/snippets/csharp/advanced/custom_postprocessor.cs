using Kreuzberg;

class WordCountPostProcessor : IPostProcessor
{
    public string Name => "word-count";
    public int Priority => 10;

    public ExtractionResult Process(ExtractionResult result)
    {
        var wordCount = result.Content.Split(
            new[] { ' ', '\n', '\r', '\t' },
            StringSplitOptions.RemoveEmptyEntries
        ).Length;

        // Store in Additional metadata
        if (result.Metadata.Additional == null)
        {
            result.Metadata.Additional = new Dictionary<string, System.Text.Json.Nodes.JsonNode?>();
        }
        result.Metadata.Additional["word_count"] = System.Text.Json.Nodes.JsonValue.Create(wordCount);

        return result;
    }
}

class SentimentPostProcessor : IPostProcessor
{
    public string Name => "sentiment-analyzer";
    public int Priority => 5;

    public ExtractionResult Process(ExtractionResult result)
    {
        // Placeholder for sentiment analysis
        var sentiment = AnalyzeSentiment(result.Content);

        // Store in Additional metadata
        if (result.Metadata.Additional == null)
        {
            result.Metadata.Additional = new Dictionary<string, System.Text.Json.Nodes.JsonNode?>();
        }
        result.Metadata.Additional["sentiment"] = System.Text.Json.Nodes.JsonValue.Create(sentiment);

        return result;
    }

    private string AnalyzeSentiment(string text)
    {
        // Simple heuristic - replace with real sentiment analysis
        return text.Length > 0 ? "neutral" : "unknown";
    }
}

class Program
{
    static void Main()
    {
        var wordCountProcessor = new WordCountPostProcessor();
        var sentimentProcessor = new SentimentPostProcessor();

        KreuzbergClient.RegisterPostProcessor(wordCountProcessor);
        KreuzbergClient.RegisterPostProcessor(sentimentProcessor);

        try
        {
            var result = KreuzbergClient.ExtractFileSync("document.pdf");

            if (result.Metadata.Additional != null)
            {
                if (result.Metadata.Additional.TryGetValue("word_count", out var wordCount))
                {
                    Console.WriteLine($"Word count: {wordCount}");
                }
                if (result.Metadata.Additional.TryGetValue("sentiment", out var sentiment))
                {
                    Console.WriteLine($"Sentiment: {sentiment}");
                }
            }
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
