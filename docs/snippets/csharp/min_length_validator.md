```csharp title="C#"
using Kreuzberg;

public class MinLengthValidator : IValidator
{
    private readonly int _minLength;

    public MinLengthValidator(int minLength = 100)
    {
        _minLength = minLength;
    }

    public string Name() => "min_length_validator";
    public string Version() => "1.0.0";
    public int Priority() => 100;

    public void Validate(Dictionary<string, object> result)
    {
        var contentLength = result["content"].ToString()?.Length ?? 0;
        if (contentLength < _minLength)
            throw new ValidationError($"Content too short: {contentLength}");
    }

    public bool ShouldValidate(Dictionary<string, object> result) => true;
    public void Initialize() { }
    public void Shutdown() { }
}

var validator = new MinLengthValidator(minLength: 100);
KreuzbergClient.RegisterValidator(validator);
```
