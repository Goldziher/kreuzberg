```csharp
using Kreuzberg;

public class QualityValidator : IValidator
{
    public string Name() => "quality-validator";
    public string Version() => "1.0.0";

    public void Validate(Dictionary<string, object> result)
    {
        var metadata = (Dictionary<string, object>)result["metadata"];
        var score = metadata.ContainsKey("quality_score")
            ? Convert.ToDouble(metadata["quality_score"])
            : 0.0;

        if (score < 0.5)
            throw new ValidationError($"Quality score too low: {score:F2}");
    }

    public bool ShouldValidate(Dictionary<string, object> result) => true;
    public int Priority() => 100;
    public void Initialize() { }
    public void Shutdown() { }
}

var validator = new QualityValidator();
KreuzbergClient.RegisterValidator(validator);
```
