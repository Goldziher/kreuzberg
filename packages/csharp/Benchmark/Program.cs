using System.Text;
using System.Text.Json;
using Kreuzberg;

var debug = Environment.GetEnvironmentVariable("KREUZBERG_BENCHMARK_DEBUG") == "true";
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

if (debug)
{
    Console.Error.WriteLine("[DEBUG] Starting C# benchmark");
    Console.Error.WriteLine($"[DEBUG] File: {filePath}");
    Console.Error.WriteLine($"[DEBUG] Iterations: {iterations}");
    Console.Error.WriteLine($"[DEBUG] KREUZBERG_FFI_DIR: {Environment.GetEnvironmentVariable("KREUZBERG_FFI_DIR") ?? "(not set)"}");
    Console.Error.WriteLine($"[DEBUG] LD_LIBRARY_PATH: {Environment.GetEnvironmentVariable("LD_LIBRARY_PATH") ?? "(not set)"}");
    Console.Error.WriteLine($"[DEBUG] DYLD_LIBRARY_PATH: {Environment.GetEnvironmentVariable("DYLD_LIBRARY_PATH") ?? "(not set)"}");
    Console.Error.WriteLine($"[DEBUG] PATH: {Environment.GetEnvironmentVariable("PATH") ?? "(not set)"}");
    Console.Error.WriteLine($"[DEBUG] AppContext.BaseDirectory: {AppContext.BaseDirectory}");
}

if (!File.Exists(filePath))
{
    Console.Error.WriteLine($"Error: File not found: {filePath}");
    return 1;
}

var content = await File.ReadAllBytesAsync(filePath);
var mimeType = GuessMimeType(filePath);

if (debug)
{
    Console.Error.WriteLine($"[DEBUG] File size: {content.Length} bytes");
    Console.Error.WriteLine($"[DEBUG] MIME type: {mimeType}");
}

try
{
    if (debug)
    {
        Console.Error.WriteLine("[DEBUG] Attempting warmup extraction...");
    }
    var warmupResult = KreuzbergClient.ExtractBytesSync(content, mimeType);
    if (debug)
    {
        Console.Error.WriteLine($"[DEBUG] Warmup succeeded, extracted {warmupResult.Content.Length} chars");
    }
}
catch (Exception ex)
{
    Console.Error.WriteLine($"Error during warmup: {ex.GetType().Name}: {ex.Message}");
    if (debug)
    {
        Console.Error.WriteLine($"[DEBUG] Full exception: {ex}");
    }
    return 1;
}

var sw = System.Diagnostics.Stopwatch.StartNew();
for (var i = 0; i < iterations; i++)
{
    try
    {
        _ = KreuzbergClient.ExtractBytesSync(content, mimeType);
    }
    catch (Exception ex)
    {
        sw.Stop();
        Console.Error.WriteLine($"Error during iteration {i}: {ex.GetType().Name}: {ex.Message}");
        if (debug)
        {
            Console.Error.WriteLine($"[DEBUG] Full exception: {ex}");
        }
        return 1;
    }
}
sw.Stop();

if (debug)
{
    Console.Error.WriteLine($"[DEBUG] Extraction completed in {sw.ElapsedMilliseconds}ms");
}

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
