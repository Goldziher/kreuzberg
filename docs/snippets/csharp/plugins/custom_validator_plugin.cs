using Kreuzberg;

class MinLengthValidator : IValidator
{
    private readonly int _minLength;

    public MinLengthValidator(int minLength)
    {
        _minLength = minLength;
    }

    public string Name => "min-length";
    public int Priority => 10;

    public void Validate(ExtractionResult result)
    {
        if (result.Content.Length < _minLength)
        {
            throw new KreuzbergValidationException(
                $"Content too short: {result.Content.Length} < {_minLength}"
            );
        }
    }
}

class QualityScoreValidator : IValidator
{
    private readonly double _minScore;

    public QualityScoreValidator(double minScore)
    {
        _minScore = minScore;
    }

    public string Name => "quality-score";
    public int Priority => 5;

    public void Validate(ExtractionResult result)
    {
        if (result.Metadata.Additional != null &&
            result.Metadata.Additional.TryGetValue("quality_score", out var scoreNode))
        {
            var score = scoreNode?.GetValue<double>() ?? 0.0;

            if (score < _minScore)
            {
                throw new KreuzbergValidationException(
                    $"Quality score too low: {score:F2} < {_minScore:F2}"
                );
            }
        }
    }
}

class ContentValidValidator : IValidator
{
    public string Name => "content-valid";
    public int Priority => 20;

    public void Validate(ExtractionResult result)
    {
        if (string.IsNullOrWhiteSpace(result.Content))
        {
            throw new KreuzbergValidationException("Extracted content is empty or whitespace");
        }

        if (result.Content.Length < 10)
        {
            throw new KreuzbergValidationException("Extracted content is too short (minimum 10 characters)");
        }
    }
}

class Program
{
    static void Main()
    {
        var minLengthValidator = new MinLengthValidator(minLength: 50);
        var qualityValidator = new QualityScoreValidator(minScore: 0.7);
        var contentValidator = new ContentValidValidator();

        KreuzbergClient.RegisterValidator(minLengthValidator);
        KreuzbergClient.RegisterValidator(qualityValidator);
        KreuzbergClient.RegisterValidator(contentValidator);

        try
        {
            var config = new ExtractionConfig
            {
                EnableQualityProcessing = true
            };

            var result = KreuzbergClient.ExtractFileSync("document.pdf", config);

            Console.WriteLine("All validations passed");
            Console.WriteLine($"Content length: {result.Content.Length}");
        }
        catch (KreuzbergValidationException ex)
        {
            Console.WriteLine($"Validation failed: {ex.Message}");
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
