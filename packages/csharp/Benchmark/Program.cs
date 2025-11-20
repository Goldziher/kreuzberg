using System.Text;
using System.Text.Json;
using Kreuzberg;

var argsSpan = args.AsSpan();
string? filePath = null;
int iterations = 5;

for (var i = 0; i < argsSpan.Length; i++)
{
    switch (argsSpan[i])
    {
        case "--file":
            if (i + 1 < argsSpan.Length)
            {
                filePath = argsSpan[++i];
            }
            break;
        case "--iterations":
            if (i + 1 < argsSpan.Length && int.TryParse(argsSpan[++i], out var parsed))
            {
                iterations = parsed;
            }
            break;
    }
}

if (string.IsNullOrWhiteSpace(filePath))
{
    Console.Error.WriteLine("Error: --file is required");
    return 1;
}

var content = await File.ReadAllBytesAsync(filePath);
var mimeType = GuessMimeType(filePath);

// Warmup
_ = KreuzbergClient.ExtractBytesSync(content, mimeType);

var sw = System.Diagnostics.Stopwatch.StartNew();
for (var i = 0; i < iterations; i++)
{
    _ = KreuzbergClient.ExtractBytesSync(content, mimeType);
}
sw.Stop();

var elapsedSeconds = sw.Elapsed.TotalSeconds;
var bytesProcessed = content.Length * (long)iterations;
var opsPerSec = iterations / elapsedSeconds;
var mbPerSec = (bytesProcessed / (1024.0 * 1024.0)) / elapsedSeconds;

var result = new
{
    language = "csharp",
    fixture = Path.GetFileName(filePath),
    fixture_path = filePath,
    iterations,
    elapsed_seconds = elapsedSeconds,
    ops_per_sec = opsPerSec,
    mb_per_sec = mbPerSec,
    bytes_processed = bytesProcessed,
};

var json = JsonSerializer.Serialize(result);
Console.WriteLine(json);
return 0;

static string GuessMimeType(string path)
{
    var ext = Path.GetExtension(path).ToLowerInvariant();
    return ext switch
    {
        ".pdf" => "application/pdf",
        ".docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ".pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        ".xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ".json" => "application/json",
        ".html" => "text/html",
        ".xml" => "application/xml",
        ".txt" => "text/plain",
        ".md" => "text/markdown",
        ".jpg" or ".jpeg" => "image/jpeg",
        ".png" => "image/png",
        ".tiff" or ".tif" => "image/tiff",
        _ => "application/octet-stream",
    };
}
