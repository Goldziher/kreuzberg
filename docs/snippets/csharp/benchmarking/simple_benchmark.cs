```csharp
using BenchmarkDotNet.Attributes;
using BenchmarkDotNet.Running;
using Kreuzberg;
using System;
using System.Diagnostics;
using System.Threading.Tasks;

[MemoryDiagnoser]
[SimpleJob(warmupCount: 3, targetCount: 5)]
public class KreuzbergBenchmark
{
    private string _testFilePath;
    private ExtractionConfig _config;

    [GlobalSetup]
    public void Setup()
    {
        _testFilePath = "document.pdf";
        _config = new ExtractionConfig
        {
            UseCache = false,
            EnableQualityProcessing = true,
        };
    }

    [Benchmark]
    public void ExtractFileSync()
    {
        var result = KreuzbergClient.ExtractFileSync(_testFilePath, _config);
        _ = result.Content.Length;
    }

    [Benchmark]
    public async Task ExtractFileAsync()
    {
        var result = await KreuzbergClient.ExtractFileAsync(_testFilePath, _config);
        _ = result.Content.Length;
    }

    [Benchmark]
    public async Task ExtractWithOcr()
    {
        var ocrConfig = new ExtractionConfig
        {
            ForceOcr = true,
            Ocr = new OcrConfig
            {
                Backend = "tesseract",
                Language = "eng",
            }
        };

        var result = await KreuzbergClient.ExtractFileAsync(_testFilePath, ocrConfig);
        _ = result.Content.Length;
    }

    [Benchmark]
    public async Task ExtractWithCache()
    {
        var cacheConfig = new ExtractionConfig
        {
            UseCache = true,
            EnableQualityProcessing = true,
        };

        var result = await KreuzbergClient.ExtractFileAsync(_testFilePath, cacheConfig);
        _ = result.Content.Length;
    }
}

// Alternative: Manual benchmark
public class ManualBenchmark
{
    public static async Task Main(string[] args)
    {
        var filePath = "document.pdf";
        var config = new ExtractionConfig();

        // Warmup
        await KreuzbergClient.ExtractFileAsync(filePath, config);

        // Benchmark sync extraction
        var sw = Stopwatch.StartNew();
        for (int i = 0; i < 10; i++)
        {
            KreuzbergClient.ExtractFileSync(filePath, config);
        }
        sw.Stop();
        Console.WriteLine($"Sync extraction (10 runs): {sw.ElapsedMilliseconds}ms avg {sw.ElapsedMilliseconds / 10f}ms");

        // Benchmark async extraction
        sw.Restart();
        var tasks = new System.Collections.Generic.List<Task>();
        for (int i = 0; i < 10; i++)
        {
            tasks.Add(KreuzbergClient.ExtractFileAsync(filePath, config));
        }
        await Task.WhenAll(tasks);
        sw.Stop();
        Console.WriteLine($"Async extraction (10 parallel runs): {sw.ElapsedMilliseconds}ms");

        // Run BenchmarkDotNet
        var summary = BenchmarkRunner.Run<KreuzbergBenchmark>();
    }
}
```
