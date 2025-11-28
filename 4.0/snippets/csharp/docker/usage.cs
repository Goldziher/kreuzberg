```csharp
using System;
using System.Diagnostics;
using System.IO;
using System.Text.Json;
using System.Threading.Tasks;

class DockerKreuzbergClient
{
    private const string ContainerName = "kreuzberg-api";
    private const string ContainerImage = "kreuzberg:latest";
    private const int ApiPort = 8000;

    public async Task StartContainerAsync()
    {
        Console.WriteLine("Starting Kreuzberg Docker container...");

        var processInfo = new ProcessStartInfo
        {
            FileName = "docker",
            Arguments = $"run -d --name {ContainerName} -p {ApiPort}:8000 {ContainerImage}",
            UseShellExecute = false,
            RedirectStandardOutput = true,
        };

        using (var process = Process.Start(processInfo))
        {
            await process.WaitForExitAsync();
        }

        Console.WriteLine($"Container started on http://localhost:{ApiPort}");
    }

    public async Task<string> ExtractFileAsync(string filePath)
    {
        using (var client = new HttpClient())
        {
            var fileBytes = await File.ReadAllBytesAsync(filePath);
            using (var content = new MultipartFormDataContent())
            {
                content.Add(new ByteArrayContent(fileBytes), "file", Path.GetFileName(filePath));

                var response = await client.PostAsync(
                    $"http://localhost:{ApiPort}/api/extract",
                    content
                );

                response.EnsureSuccessStatusCode();
                var json = await response.Content.ReadAsStringAsync();
                var result = JsonSerializer.Deserialize<JsonElement>(json);
                return result.GetProperty("content").GetString();
            }
        }
    }

    public async Task StopContainerAsync()
    {
        Console.WriteLine("Stopping Kreuzberg Docker container...");

        var processInfo = new ProcessStartInfo
        {
            FileName = "docker",
            Arguments = $"stop {ContainerName}",
            UseShellExecute = false,
        };

        using (var process = Process.Start(processInfo))
        {
            await process.WaitForExitAsync();
        }

        processInfo.Arguments = $"rm {ContainerName}";
        using (var process = Process.Start(processInfo))
        {
            await process.WaitForExitAsync();
        }

        Console.WriteLine("Container stopped and removed");
    }
}

// Usage
var dockerClient = new DockerKreuzbergClient();

try
{
    await dockerClient.StartContainerAsync();
    await Task.Delay(2000); // Wait for container to be ready

    var content = await dockerClient.ExtractFileAsync("document.pdf");
    Console.WriteLine($"Extracted content:\n{content}");
}
finally
{
    await dockerClient.StopContainerAsync();
}
```
