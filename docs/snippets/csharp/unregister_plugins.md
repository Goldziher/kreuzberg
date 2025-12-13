```csharp title="C#"
using Kreuzberg;

var names = new List<string>
{
    "custom-json-extractor",
    "word_count",
    "cloud-ocr",
    "min_length_validator"
};

KreuzbergClient.UnregisterDocumentExtractor(names[0]);
KreuzbergClient.UnregisterPostProcessor(names[1]);
KreuzbergClient.UnregisterOcrBackend(names[2]);
KreuzbergClient.UnregisterValidator(names[3]);
```
