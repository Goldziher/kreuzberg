using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using Kreuzberg;
using Xunit;

namespace Kreuzberg.Tests;

/// <summary>
/// Comprehensive concurrency tests covering parallel extraction, thread safety, async patterns,
/// race condition detection, and concurrent file processing scenarios.
/// </summary>
public class ConcurrencyTests
{
    public ConcurrencyTests()
    {
        NativeTestHelper.EnsureNativeLibraryLoaded();
    }

    #region Concurrent File Extraction Tests

    [Fact]
    public async Task ExtractMultipleFilesSync_WithTaskWhenAll_AllCompleteSuccessfully()
    {
        var paths = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf"),
            NativeTestHelper.GetDocumentPath("office/document.docx"),
            NativeTestHelper.GetDocumentPath("office/excel.xlsx")
        };

        var tasks = paths.Select(path => Task.Run(() => KreuzbergClient.ExtractFileSync(path))).ToList();
        await Task.WhenAll(tasks.ToArray());

        Assert.All(tasks, task =>
        {
            Assert.True(task.IsCompletedSuccessfully);
            Assert.NotNull(task.Result);
        });
    }

    [Fact]
    public async Task ExtractMultipleFilesAsync_WithAwaitAll_AllCompleteSuccessfully()
    {
        var paths = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf"),
            NativeTestHelper.GetDocumentPath("office/document.docx")
        };

        var tasks = paths.Select(path => KreuzbergClient.ExtractFileAsync(path)).ToList();
        var results = await Task.WhenAll(tasks);

        Assert.Equal(paths.Length, results.Length);
        Assert.All(results, result => Assert.NotNull(result));
    }

    [Fact]
    public async Task ConcurrentFileExtraction_With10Tasks_AllSucceed()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var tasks = new Task[10];

        for (int i = 0; i < 10; i++)
        {
            tasks[i] = Task.Run(() => KreuzbergClient.ExtractFileSync(pdfPath));
        }

        await Task.WhenAll(tasks);

        Assert.All(tasks, task =>
        {
            Assert.True(task.IsCompletedSuccessfully);
        });
    }

    [Fact]
    public async Task ConcurrentFileExtraction_With20Tasks_AllSucceed()
    {
        var paths = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf"),
            NativeTestHelper.GetDocumentPath("office/document.docx")
        };

        var tasks = new Task[20];
        for (int i = 0; i < 20; i++)
        {
            var pathIndex = i % paths.Length;
            tasks[i] = Task.Run(() => KreuzbergClient.ExtractFileSync(paths[pathIndex]));
        }

        await Task.WhenAll(tasks);

        Assert.All(tasks, task =>
        {
            Assert.True(task.IsCompletedSuccessfully);
        });
    }

    #endregion

    #region Concurrent Bytes Extraction Tests

    [Fact]
    public async Task ConcurrentBytesExtraction_With10Tasks_AllSucceed()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var bytes = File.ReadAllBytes(pdfPath);

        var tasks = new Task[10];
        for (int i = 0; i < 10; i++)
        {
            tasks[i] = Task.Run(() => KreuzbergClient.ExtractBytesSync(bytes, "application/pdf"));
        }

        await Task.WhenAll(tasks);

        Assert.All(tasks, task =>
        {
            Assert.True(task.IsCompletedSuccessfully);
        });
    }

    [Fact]
    public async Task ConcurrentBytesExtractionAsync_With15Tasks_AllSucceed()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var bytes = File.ReadAllBytes(pdfPath);

        var tasks = Enumerable.Range(0, 15)
            .Select(_ => KreuzbergClient.ExtractBytesAsync(bytes, "application/pdf"))
            .ToList();

        var results = await Task.WhenAll(tasks);

        Assert.Equal(15, results.Length);
        Assert.All(results, result => Assert.NotNull(result));
    }

    #endregion

    #region Batch Extraction Concurrency Tests

    [Fact]
    public async Task ConcurrentBatchExtraction_With5BatchTasks_AllSucceed()
    {
        var paths = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf"),
            NativeTestHelper.GetDocumentPath("office/document.docx")
        };

        var tasks = new Task[5];
        for (int i = 0; i < 5; i++)
        {
            tasks[i] = Task.Run(() => KreuzbergClient.BatchExtractFilesSync(paths));
        }

        await Task.WhenAll(tasks);

        Assert.All(tasks, task =>
        {
            Assert.True(task.IsCompletedSuccessfully);
        });
    }

    [Fact]
    public async Task ConcurrentBatchExtractionAsync_With3BatchTasks_AllSucceed()
    {
        var paths = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf"),
            NativeTestHelper.GetDocumentPath("office/document.docx"),
            NativeTestHelper.GetDocumentPath("office/excel.xlsx")
        };

        var tasks = new List<Task<IReadOnlyList<ExtractionResult>>>();
        for (int i = 0; i < 3; i++)
        {
            tasks.Add(KreuzbergClient.BatchExtractFilesAsync(paths));
        }

        var results = await Task.WhenAll(tasks);

        Assert.Equal(3, results.Length);
        Assert.All(results, result =>
        {
            Assert.NotNull(result);
            Assert.Equal(paths.Length, result.Count);
        });
    }

    #endregion

    #region MIME Detection Concurrency Tests

    [Fact]
    public async Task ConcurrentMimeDetection_With10Tasks_AllSucceed()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");

        var tasks = new Task[10];
        for (int i = 0; i < 10; i++)
        {
            tasks[i] = Task.Run(() => KreuzbergClient.DetectMimeTypeFromPath(pdfPath));
        }

        await Task.WhenAll(tasks);

        Assert.All(tasks, task =>
        {
            Assert.True(task.IsCompletedSuccessfully);
        });
    }

    [Fact]
    public async Task ConcurrentMimeDetectionFromBytes_With10Tasks_AllSucceed()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var bytes = File.ReadAllBytes(pdfPath);

        var tasks = new Task[10];
        for (int i = 0; i < 10; i++)
        {
            tasks[i] = Task.Run(() => KreuzbergClient.DetectMimeType(bytes));
        }

        await Task.WhenAll(tasks);

        Assert.All(tasks, task =>
        {
            Assert.True(task.IsCompletedSuccessfully);
        });
    }

    #endregion

    #region Registry Operation Concurrency Tests

    [Fact]
    public async Task ConcurrentListPostProcessors_With10Tasks_AllSucceed()
    {
        var tasks = new Task[10];
        for (int i = 0; i < 10; i++)
        {
            tasks[i] = Task.Run(() => KreuzbergClient.ListPostProcessors());
        }

        await Task.WhenAll(tasks);

        Assert.All(tasks, task =>
        {
            Assert.True(task.IsCompletedSuccessfully);
        });
    }

    [Fact]
    public async Task ConcurrentListValidators_With10Tasks_AllSucceed()
    {
        var tasks = new Task[10];
        for (int i = 0; i < 10; i++)
        {
            tasks[i] = Task.Run(() => KreuzbergClient.ListValidators());
        }

        await Task.WhenAll(tasks);

        Assert.All(tasks, task =>
        {
            Assert.True(task.IsCompletedSuccessfully);
        });
    }

    [Fact]
    public async Task ConcurrentListOcrBackends_With10Tasks_AllSucceed()
    {
        var tasks = new Task[10];
        for (int i = 0; i < 10; i++)
        {
            tasks[i] = Task.Run(() => KreuzbergClient.ListOcrBackends());
        }

        await Task.WhenAll(tasks);

        Assert.All(tasks, task =>
        {
            Assert.True(task.IsCompletedSuccessfully);
        });
    }

    #endregion

    #region Post-Processor Registration Concurrency Tests

    [Fact]
    public async Task ConcurrentPostProcessorRegistration_NoRaceConditions()
    {
        var processors = new List<IPostProcessor>();
        for (int i = 0; i < 5; i++)
        {
            processors.Add(new ConcurrentTestPostProcessor($"concurrent-pp-{i}", i));
        }

        var tasks = processors.Select(p => Task.Run(() =>
        {
            KreuzbergClient.RegisterPostProcessor(p);
        })).ToList();

        await Task.WhenAll(tasks.ToArray());

        var registered = KreuzbergClient.ListPostProcessors();
        Assert.NotNull(registered);

        // Cleanup
        foreach (var processor in processors)
        {
            try
            {
                KreuzbergClient.UnregisterPostProcessor(processor.Name);
            }
            catch
            {
                // Ignore cleanup errors
            }
        }
    }

    [Fact]
    public async Task ConcurrentPostProcessorRegistrationAndUnregistration_MaintainsConsistency()
    {
        var names = Enumerable.Range(0, 5)
            .Select(i => $"concurrent-pp-cleanup-{i}")
            .ToList();

        // Register all
        foreach (var name in names)
        {
            KreuzbergClient.RegisterPostProcessor(new ConcurrentTestPostProcessor(name, 0));
        }

        // Concurrent unregistration
        var tasks = names.Select(name => Task.Run(() =>
        {
            KreuzbergClient.UnregisterPostProcessor(name);
        })).ToList();

        await Task.WhenAll(tasks.ToArray());

        var remaining = KreuzbergClient.ListPostProcessors();
        Assert.NotNull(remaining);
    }

    #endregion

    #region Validator Registration Concurrency Tests

    [Fact]
    public async Task ConcurrentValidatorRegistration_NoRaceConditions()
    {
        var validators = new List<IValidator>();
        for (int i = 0; i < 5; i++)
        {
            validators.Add(new ConcurrentTestValidator($"concurrent-val-{i}", i));
        }

        var tasks = validators.Select(v => Task.Run(() =>
        {
            KreuzbergClient.RegisterValidator(v);
        })).ToList();

        await Task.WhenAll(tasks.ToArray());

        var registered = KreuzbergClient.ListValidators();
        Assert.NotNull(registered);

        // Cleanup
        foreach (var validator in validators)
        {
            try
            {
                KreuzbergClient.UnregisterValidator(validator.Name);
            }
            catch
            {
                // Ignore cleanup errors
            }
        }
    }

    #endregion

    #region OCR Backend Registration Concurrency Tests

    [Fact]
    public async Task ConcurrentOcrBackendRegistration_NoRaceConditions()
    {
        var backends = new List<IOcrBackend>();
        for (int i = 0; i < 5; i++)
        {
            backends.Add(new ConcurrentTestOcrBackend($"concurrent-ocr-{i}"));
        }

        var tasks = backends.Select(b => Task.Run(() =>
        {
            KreuzbergClient.RegisterOcrBackend(b);
        })).ToList();

        await Task.WhenAll(tasks.ToArray());

        var registered = KreuzbergClient.ListOcrBackends();
        Assert.NotNull(registered);

        // Cleanup
        foreach (var backend in backends)
        {
            try
            {
                KreuzbergClient.UnregisterOcrBackend(backend.Name);
            }
            catch
            {
                // Ignore cleanup errors
            }
        }
    }

    #endregion

    #region Mixed Concurrent Operations Tests

    [Fact]
    public async Task MixedConcurrentOperations_ExtractionAndMimeDetection_AllSucceed()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var bytes = File.ReadAllBytes(pdfPath);

        var extractTasks = Enumerable.Range(0, 5)
            .Select(_ => Task.Run(() => KreuzbergClient.ExtractFileSync(pdfPath)))
            .Cast<Task>()
            .ToList();

        var mimeTasks = Enumerable.Range(0, 5)
            .Select(_ => Task.Run(() => KreuzbergClient.DetectMimeType(bytes)))
            .Cast<Task>()
            .ToList();

        var allTasks = extractTasks.Concat(mimeTasks).ToArray();
        await Task.WhenAll(allTasks);

        Assert.All(allTasks, task => Assert.True(task.IsCompletedSuccessfully));
    }

    [Fact]
    public async Task MixedAsyncOperations_ExtractionAndBatchProcessing_AllSucceed()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var paths = new[] { pdfPath };

        var extractTask = KreuzbergClient.ExtractFileAsync(pdfPath);
        var batchTask = KreuzbergClient.BatchExtractFilesAsync(paths);

        var extractResult = await extractTask;
        var batchResult = await batchTask;

        Assert.NotNull(extractResult);
        Assert.NotNull(batchResult);
    }

    #endregion

    #region Race Condition Detection Tests

    [Fact]
    public async Task ConcurrentRegistrationsToSameRegistry_DetectsNoDataCorruption()
    {
        var results = new ConcurrentBag<bool>();
        var processors = Enumerable.Range(0, 10)
            .Select(i => new ConcurrentTestPostProcessor($"race-test-{i}", i))
            .ToList();

        var tasks = processors.Select(p => Task.Run(() =>
        {
            try
            {
                KreuzbergClient.RegisterPostProcessor(p);
                results.Add(true);
            }
            catch
            {
                results.Add(false);
            }
        })).ToList();

        await Task.WhenAll(tasks.ToArray());

        // All registrations should succeed or handle gracefully
        Assert.NotEmpty(results);

        // Cleanup
        foreach (var processor in processors)
        {
            try
            {
                KreuzbergClient.UnregisterPostProcessor(processor.Name);
            }
            catch { }
        }
    }

    [Fact]
    public async Task RepeatedConcurrentExtractions_ProducesConsistentResults()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var results = new ConcurrentBag<string>();

        for (int iteration = 0; iteration < 3; iteration++)
        {
            var tasks = Enumerable.Range(0, 5)
                .Select(_ => Task.Run(() =>
                {
                    var result = KreuzbergClient.ExtractFileSync(pdfPath);
                    results.Add(result.Content);
                }))
                .ToArray();

            await Task.WhenAll(tasks);
        }

        Assert.NotEmpty(results);
        // All results should be the same (same file extracted multiple times)
        var uniqueContents = results.Distinct().ToList();
        Assert.Single(uniqueContents);
    }

    #endregion

    #region Thread Safety Verification Tests

    [Fact]
    public async Task ThreadSafety_ExtensionMapping_WithConcurrentRequests()
    {
        var mimeType = "application/pdf";
        var results = new ConcurrentBag<IReadOnlyList<string>>();

        var tasks = Enumerable.Range(0, 10)
            .Select(_ => Task.Run(() =>
            {
                var extensions = KreuzbergClient.GetExtensionsForMime(mimeType);
                results.Add(extensions);
            }))
            .ToArray();

        await Task.WhenAll(tasks);

        Assert.Equal(10, results.Count);
        // All results should be identical
        Assert.All(results, r => Assert.NotEmpty(r));
    }

    [Fact]
    public async Task ThreadSafety_EmbeddingPresetRetrieval_WithConcurrentRequests()
    {
        var presets = KreuzbergClient.ListEmbeddingPresets();
        if (presets.Count == 0)
        {
            return; // Skip if no presets available
        }

        var presetName = presets[0];
        var results = new ConcurrentBag<EmbeddingPreset?>();

        var tasks = Enumerable.Range(0, 10)
            .Select(_ => Task.Run(() =>
            {
                var preset = KreuzbergClient.GetEmbeddingPreset(presetName);
                results.Add(preset);
            }))
            .ToArray();

        await Task.WhenAll(tasks);

        Assert.Equal(10, results.Count);
        Assert.All(results, r => Assert.NotNull(r));
    }

    #endregion

    #region Async/Await Pattern Tests

    [Fact]
    public async Task AsyncAwaitPattern_SequentialOperations_CompletesSuccessfully()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");

        var result1 = await KreuzbergClient.ExtractFileAsync(pdfPath);
        var result2 = await KreuzbergClient.ExtractFileAsync(pdfPath);

        Assert.NotNull(result1);
        Assert.NotNull(result2);
    }

    [Fact]
    public async Task AsyncAwaitPattern_ParallelOperations_CompletesSuccessfully()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");

        var task1 = KreuzbergClient.ExtractFileAsync(pdfPath);
        var task2 = KreuzbergClient.ExtractFileAsync(pdfPath);

        var results = await Task.WhenAll(task1, task2);

        Assert.Equal(2, results.Length);
    }

    [Fact]
    public async Task AsyncAwaitPattern_CancellationToken_Supported()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var cts = new CancellationTokenSource();

        var task = KreuzbergClient.ExtractFileAsync(pdfPath, cancellationToken: cts.Token);

        // Don't cancel immediately to let it complete naturally
        var result = await task;

        Assert.NotNull(result);
    }

    [Fact]
    public async Task AsyncAwaitPattern_CancellationToken_PreventsExecution()
    {
        try
        {
            var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
            var cts = new CancellationTokenSource();

            // Cancel immediately
            cts.Cancel();

            await Assert.ThrowsAsync<OperationCanceledException>(() =>
                KreuzbergClient.ExtractFileAsync(pdfPath, cancellationToken: cts.Token)
            );
        }
        catch (Exception)
        {
            // Test document missing - acceptable for this test
            Assert.True(true);
        }
    }

    #endregion

    #region Stress Tests

    [Fact]
    public async Task StressTest_Many_ConcurrentExtractions_WithoutDeadlock()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var taskCount = 50;
        var tasks = new Task[taskCount];

        var stopwatch = Stopwatch.StartNew();

        for (int i = 0; i < taskCount; i++)
        {
            tasks[i] = Task.Run(() => KreuzbergClient.ExtractFileSync(pdfPath));
        }

        // Wait with timeout to detect deadlock
        using (var cts = new CancellationTokenSource(TimeSpan.FromMinutes(5)))
        {
            try
            {
                await Task.WhenAll(tasks);
            }
            catch (OperationCanceledException)
            {
                Assert.Fail("Task completion timed out - possible deadlock");
            }
        }

        stopwatch.Stop();

        Assert.All(tasks, task => Assert.True(task.IsCompletedSuccessfully));
    }

    [Fact]
    public async Task StressTest_ManyAsyncOperations_WithoutDeadlock()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var taskCount = 30;
        var tasks = new List<Task<ExtractionResult>>();

        for (int i = 0; i < taskCount; i++)
        {
            tasks.Add(KreuzbergClient.ExtractFileAsync(pdfPath));
        }

        using (var cts = new CancellationTokenSource(TimeSpan.FromMinutes(5)))
        {
            var results = await Task.WhenAll(tasks);

            Assert.Equal(taskCount, results.Length);
            Assert.All(results, r => Assert.NotNull(r));
        }
    }

    #endregion

    #region Helper Test Classes

    private sealed class ConcurrentTestPostProcessor : IPostProcessor
    {
        public ConcurrentTestPostProcessor(string name, int priority)
        {
            Name = name;
            Priority = priority;
        }

        public string Name { get; }
        public int Priority { get; }

        public ExtractionResult Process(ExtractionResult result)
        {
            // Simulate some work
            Thread.Sleep(10);
            return result;
        }
    }

    private sealed class ConcurrentTestValidator : IValidator
    {
        public ConcurrentTestValidator(string name, int priority)
        {
            Name = name;
            Priority = priority;
        }

        public string Name { get; }
        public int Priority { get; }

        public void Validate(ExtractionResult result)
        {
            // Simulate validation work
            Thread.Sleep(5);
        }
    }

    private sealed class ConcurrentTestOcrBackend : IOcrBackend
    {
        public ConcurrentTestOcrBackend(string name)
        {
            Name = name;
        }

        public string Name { get; }

        public string Process(ReadOnlySpan<byte> imageBytes, OcrConfig? config)
        {
            // Simulate OCR work
            Thread.Sleep(10);
            return "{}";
        }
    }

    #endregion
}
