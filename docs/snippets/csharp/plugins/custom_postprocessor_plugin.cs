using Kreuzberg;
using System.Text.RegularExpressions;

class WordCountPostProcessor : IPostProcessor
{
    public string Name => "word-count";
    public int Priority => 10;

    public ExtractionResult Process(ExtractionResult result)
    {
        if (string.IsNullOrEmpty(result.Content))
        {
            return result;
        }

        var wordCount = result.Content.Split(
            new[] { ' ', '\n', '\r', '\t' },
            StringSplitOptions.RemoveEmptyEntries
        ).Length;

        if (result.Metadata.Additional == null)
        {
            result.Metadata.Additional = new Dictionary<string, System.Text.Json.Nodes.JsonNode?>();
        }
        result.Metadata.Additional["word_count"] = System.Text.Json.Nodes.JsonValue.Create(wordCount);

        return result;
    }
}

class CleanupPostProcessor : IPostProcessor
{
    public string Name => "text-cleanup";
    public int Priority => 5;

    public ExtractionResult Process(ExtractionResult result)
    {
        if (string.IsNullOrEmpty(result.Content))
        {
            return result;
        }

        // Remove extra whitespace
        var cleaned = Regex.Replace(result.Content, @"\s+", " ").Trim();

        // Remove special characters but keep punctuation
        cleaned = Regex.Replace(cleaned, @"[^\w\s\.\,\!\?\-]", "");

        result.Content = cleaned;

        return result;
    }
}

class LanguageDetectionPostProcessor : IPostProcessor
{
    public string Name => "language-detection";
    public int Priority => 1;

    public ExtractionResult Process(ExtractionResult result)
    {
        if (string.IsNullOrEmpty(result.Content))
        {
            return result;
        }

        var detectedLanguage = DetectLanguage(result.Content);

        if (result.Metadata.Additional == null)
        {
            result.Metadata.Additional = new Dictionary<string, System.Text.Json.Nodes.JsonNode?>();
        }
        result.Metadata.Additional["detected_language"] = System.Text.Json.Nodes.JsonValue.Create(detectedLanguage);

        return result;
    }

    private string DetectLanguage(string text)
    {
        // Placeholder - replace with real language detection
        var commonEnglishWords = new[] { "the", "is", "and", "to", "of", "a", "in", "that" };
        var lowerText = text.ToLower();
        var matches = commonEnglishWords.Count(word =>
            Regex.IsMatch(lowerText, $@"\b{word}\b")
        );

        return matches > 5 ? "en" : "unknown";
    }
}

class Program
{
    static void Main()
    {
        var wordCountProcessor = new WordCountPostProcessor();
        var cleanupProcessor = new CleanupPostProcessor();
        var languageProcessor = new LanguageDetectionPostProcessor();

        KreuzbergClient.RegisterPostProcessor(wordCountProcessor);
        KreuzbergClient.RegisterPostProcessor(cleanupProcessor);
        KreuzbergClient.RegisterPostProcessor(languageProcessor);

        try
        {
            var config = new ExtractionConfig();
            var result = KreuzbergClient.ExtractFileSync("document.pdf", config);

            Console.WriteLine($"Original content length: {result.Content.Length}");

            if (result.Metadata.Additional != null)
            {
                if (result.Metadata.Additional.TryGetValue("word_count", out var wc))
                {
                    Console.WriteLine($"Word count: {wc}");
                }
                if (result.Metadata.Additional.TryGetValue("detected_language", out var lang))
                {
                    Console.WriteLine($"Detected language: {lang}");
                }
            }
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
