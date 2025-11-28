```csharp
using System;
using System.CommandLine;
using System.CommandLine.Invocation;
using System.Threading.Tasks;
using Kreuzberg;

var rootCommand = new RootCommand("Kreuzberg document extraction CLI");

// extract-file command
var extractFileCommand = new Command("extract-file", "Extract text from a document file");
var filePath = new Argument<string>("path", "Path to the document file");
var outputFormat = new Option<string>(
    new[] { "-f", "--format" },
    getDefaultValue: () => "text",
    "Output format (text, json)"
);

extractFileCommand.AddArgument(filePath);
extractFileCommand.AddOption(outputFormat);

extractFileCommand.SetHandler(async (path, format) =>
{
    try
    {
        var result = await KreuzbergClient.ExtractFileAsync(path);

        if (format == "json")
        {
            Console.WriteLine(System.Text.Json.JsonSerializer.Serialize(result));
        }
        else
        {
            Console.WriteLine(result.Content);
        }
    }
    catch (Exception ex)
    {
        Console.Error.WriteLine($"Error: {ex.Message}");
        Environment.Exit(1);
    }
}, filePath, outputFormat);

rootCommand.AddCommand(extractFileCommand);

return await rootCommand.InvokeAsync(args);
```
