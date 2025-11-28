```csharp
using System;
using System.CommandLine;
using System.Text.Json;
using System.Threading.Tasks;
using Kreuzberg;

var rootCommand = new RootCommand("Kreuzberg with configuration");

var extractCommand = new Command("extract", "Extract with custom configuration");
var filePath = new Argument<string>("path", "Document file path");
var configPath = new Option<string>(
    new[] { "-c", "--config" },
    "Path to JSON configuration file"
);
var forceOcr = new Option<bool>(
    new[] { "--force-ocr" },
    "Force OCR processing"
);
var useCache = new Option<bool>(
    new[] { "--use-cache" },
    getDefaultValue: () => true,
    "Use caching (default: true)"
);

extractCommand.AddArgument(filePath);
extractCommand.AddOption(configPath);
extractCommand.AddOption(forceOcr);
extractCommand.AddOption(useCache);

extractCommand.SetHandler(async (path, config, ocr, cache) =>
{
    try
    {
        ExtractionConfig extractionConfig;

        if (!string.IsNullOrEmpty(config))
        {
            var json = await System.IO.File.ReadAllTextAsync(config);
            extractionConfig = JsonSerializer.Deserialize<ExtractionConfig>(json);
        }
        else
        {
            extractionConfig = new ExtractionConfig
            {
                UseCache = cache,
                ForceOcr = ocr,
            };
        }

        Console.WriteLine("Extracting with configuration:");
        Console.WriteLine($"  - File: {path}");
        Console.WriteLine($"  - Force OCR: {extractionConfig.ForceOcr}");
        Console.WriteLine($"  - Use Cache: {extractionConfig.UseCache}");

        var result = await KreuzbergClient.ExtractFileAsync(path, extractionConfig);

        Console.WriteLine($"\nExtraction complete:");
        Console.WriteLine($"  - Content length: {result.Content.Length}");
        Console.WriteLine($"  - Format: {result.Metadata.FormatType}");
        Console.WriteLine($"  - Languages: {string.Join(", ", result.DetectedLanguages)}");

        Console.WriteLine($"\n{result.Content}");
    }
    catch (Exception ex)
    {
        Console.Error.WriteLine($"Error: {ex.Message}");
        Environment.Exit(1);
    }
}, filePath, configPath, forceOcr, useCache);

rootCommand.AddCommand(extractCommand);

return await rootCommand.InvokeAsync(args);
```
