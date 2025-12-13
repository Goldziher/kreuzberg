```csharp title="C#"
using Kreuzberg;

KreuzbergClient.ClearPostProcessors();
KreuzbergClient.ClearValidators();
KreuzbergClient.ClearOcrBackends();
KreuzbergClient.ClearDocumentExtractors();

Console.WriteLine("All plugins cleared");
```
