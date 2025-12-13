using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

namespace Kreuzberg;

/// <summary>
/// High-level .NET API for the Kreuzberg document intelligence engine.
/// </summary>
public static class KreuzbergClient
{
    private static readonly ConcurrentDictionary<string, GCHandle> RegisteredPostProcessors = new(StringComparer.OrdinalIgnoreCase);
    private static readonly ConcurrentDictionary<string, GCHandle> RegisteredValidators = new(StringComparer.OrdinalIgnoreCase);
    private static readonly ConcurrentDictionary<string, GCHandle> RegisteredOcrBackends = new(StringComparer.OrdinalIgnoreCase);

    /// <summary>
    /// Detects the MIME type of raw document bytes by examining file signatures.
    /// </summary>
    /// <param name="data">Document bytes to analyze. Must not be empty.</param>
    /// <returns>MIME type string (e.g., "application/pdf", "application/vnd.openxmlformats-officedocument.wordprocessingml.document")</returns>
    /// <exception cref="KreuzbergValidationException">If data is empty</exception>
    /// <exception cref="KreuzbergException">If MIME detection fails</exception>
    /// <example>
    /// <code>
    /// var data = File.ReadAllBytes("document");
    /// var mimeType = KreuzbergClient.DetectMimeType(data);
    /// Console.WriteLine(mimeType); // "application/pdf"
    /// </code>
    /// </example>
    public static string DetectMimeType(ReadOnlySpan<byte> data)
    {
        if (data.IsEmpty)
        {
            throw new KreuzbergValidationException("data cannot be empty");
        }

        unsafe
        {
            fixed (byte* ptr = data)
            {
                var resultPtr = NativeMethods.DetectMimeTypeFromBytes((IntPtr)ptr, (UIntPtr)data.Length);
                if (resultPtr == IntPtr.Zero)
                {
                    ThrowLastError();
                }

                var mime = InteropUtilities.ReadUtf8(resultPtr) ?? string.Empty;
                NativeMethods.FreeString(resultPtr);
                return mime;
            }
        }
    }

    /// <summary>
    /// Detects the MIME type of a file from its path by reading file signatures.
    /// </summary>
    /// <param name="path">Absolute or relative path to the file to analyze. Must not be empty.</param>
    /// <returns>MIME type string (e.g., "application/pdf")</returns>
    /// <exception cref="KreuzbergValidationException">If path is null, empty, or file not found</exception>
    /// <exception cref="KreuzbergException">If MIME detection fails</exception>
    /// <example>
    /// <code>
    /// var mimeType = KreuzbergClient.DetectMimeTypeFromPath("/path/to/file.pdf");
    /// Console.WriteLine(mimeType); // "application/pdf"
    /// </code>
    /// </example>
    public static string DetectMimeTypeFromPath(string path)
    {
        if (string.IsNullOrWhiteSpace(path))
        {
            throw new KreuzbergValidationException("path cannot be empty");
        }

        var pathPtr = InteropUtilities.AllocUtf8(path);
        try
        {
            var resultPtr = NativeMethods.DetectMimeTypeFromPath(pathPtr);
            if (resultPtr == IntPtr.Zero)
            {
                ThrowLastError();
            }

            var mime = InteropUtilities.ReadUtf8(resultPtr) ?? string.Empty;
            NativeMethods.FreeString(resultPtr);
            return mime;
        }
        finally
        {
            InteropUtilities.FreeUtf8(pathPtr);
        }
    }

    /// <summary>
    /// Gets file extensions associated with a MIME type.
    /// </summary>
    /// <param name="mimeType">MIME type string (e.g., "application/pdf"). Must not be empty.</param>
    /// <returns>List of file extensions (e.g., [".pdf"])</returns>
    /// <exception cref="KreuzbergValidationException">If mimeType is null or empty</exception>
    /// <exception cref="KreuzbergException">If MIME type is not recognized</exception>
    /// <example>
    /// <code>
    /// var extensions = KreuzbergClient.GetExtensionsForMime("application/pdf");
    /// Console.WriteLine(string.Join(", ", extensions)); // ".pdf"
    /// </code>
    /// </example>
    public static IReadOnlyList<string> GetExtensionsForMime(string mimeType)
    {
        if (string.IsNullOrWhiteSpace(mimeType))
        {
            throw new KreuzbergValidationException("mimeType cannot be empty");
        }

        var mimePtr = InteropUtilities.AllocUtf8(mimeType);
        try
        {
            var resultPtr = NativeMethods.GetExtensionsForMime(mimePtr);
            if (resultPtr == IntPtr.Zero)
            {
                ThrowLastError();
            }

            var json = InteropUtilities.ReadUtf8(resultPtr);
            NativeMethods.FreeString(resultPtr);
            if (string.IsNullOrWhiteSpace(json))
            {
                return Array.Empty<string>();
            }

            var parsed = JsonSerializer.Deserialize<List<string>>(json, Serialization.Options);
            return parsed ?? new List<string>();
        }
        finally
        {
            InteropUtilities.FreeUtf8(mimePtr);
        }
    }

    /// <summary>
    /// Discovers extraction configuration by walking parent directories for kreuzberg.toml, kreuzberg.yaml, or kreuzberg.json.
    /// </summary>
    /// <returns>ExtractionConfig if config file is found, null otherwise</returns>
    /// <exception cref="KreuzbergException">If config file exists but is malformed</exception>
    /// <example>
    /// <code>
    /// var config = KreuzbergClient.DiscoverExtractionConfig();
    /// if (config != null)
    /// {
    ///     var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
    /// }
    /// </code>
    /// </example>
    public static ExtractionConfig? DiscoverExtractionConfig()
    {
        var configPtr = NativeMethods.ConfigDiscover();
        if (configPtr == IntPtr.Zero)
        {
            return null;
        }

        var json = InteropUtilities.ReadUtf8(configPtr);
        NativeMethods.FreeString(configPtr);

        return string.IsNullOrWhiteSpace(json) ? null : Serialization.ParseConfig(json!);
    }

    /// <summary>
    /// Extracts text, metadata, and structured information from a file synchronously.
    /// </summary>
    /// <param name="path">Absolute or relative path to the file to extract from. Must not be empty.</param>
    /// <param name="config">Optional extraction configuration. If null, uses default settings.</param>
    /// <returns>ExtractionResult containing content, metadata, tables, and detected languages</returns>
    /// <exception cref="ArgumentException">If path is null or empty</exception>
    /// <exception cref="KreuzbergValidationException">If configuration is invalid</exception>
    /// <exception cref="KreuzbergParsingException">If document parsing fails</exception>
    /// <exception cref="KreuzbergOcrException">If OCR processing fails</exception>
    /// <exception cref="KreuzbergException">If extraction fails</exception>
    /// <example>
    /// <code>
    /// var result = KreuzbergClient.ExtractFileSync("document.pdf");
    /// Console.WriteLine(result.Content);
    /// Console.WriteLine($"MIME Type: {result.MimeType}");
    /// </code>
    /// </example>
    public static ExtractionResult ExtractFileSync(string path, ExtractionConfig? config = null)
    {
        if (string.IsNullOrWhiteSpace(path))
        {
            throw new ArgumentException("path cannot be null or empty", nameof(path));
        }

        var pathPtr = InteropUtilities.AllocUtf8(path);
        var configPtr = SerializeConfig(config);

        try
        {
            var resultPtr = configPtr != IntPtr.Zero
                ? NativeMethods.ExtractFileSyncWithConfig(pathPtr, configPtr)
                : NativeMethods.ExtractFileSync(pathPtr);

            if (resultPtr == IntPtr.Zero)
            {
                ThrowLastError();
            }

            return ConvertResult(resultPtr);
        }
        finally
        {
            InteropUtilities.FreeUtf8(pathPtr);
            InteropUtilities.FreeUtf8(configPtr);
        }
    }

    /// <summary>
    /// Extracts text, metadata, and structured information from in-memory document bytes synchronously.
    /// </summary>
    /// <param name="data">Document bytes to extract from. Must not be empty.</param>
    /// <param name="mimeType">MIME type of the document (e.g., "application/pdf"). Must not be empty.</param>
    /// <param name="config">Optional extraction configuration. If null, uses default settings.</param>
    /// <returns>ExtractionResult containing content, metadata, tables, and detected languages</returns>
    /// <exception cref="KreuzbergValidationException">If data is empty, mimeType is empty, or configuration is invalid</exception>
    /// <exception cref="KreuzbergParsingException">If document parsing fails</exception>
    /// <exception cref="KreuzbergOcrException">If OCR processing fails</exception>
    /// <exception cref="KreuzbergException">If extraction fails</exception>
    /// <example>
    /// <code>
    /// var data = File.ReadAllBytes("document.pdf");
    /// var result = KreuzbergClient.ExtractBytesSync(data, "application/pdf");
    /// Console.WriteLine(result.Content);
    /// </code>
    /// </example>
    public static ExtractionResult ExtractBytesSync(ReadOnlySpan<byte> data, string mimeType, ExtractionConfig? config = null)
    {
        if (data.IsEmpty)
        {
            throw new KreuzbergValidationException("data cannot be empty");
        }
        if (string.IsNullOrWhiteSpace(mimeType))
        {
            throw new KreuzbergValidationException("mimeType is required");
        }

        var mimePtr = InteropUtilities.AllocUtf8(mimeType);
        var configPtr = SerializeConfig(config);

        unsafe
        {
            fixed (byte* dataPtr = data)
            {
                try
                {
                    var resultPtr = configPtr != IntPtr.Zero
                        ? NativeMethods.ExtractBytesSyncWithConfig((IntPtr)dataPtr, (UIntPtr)data.Length, mimePtr, configPtr)
                        : NativeMethods.ExtractBytesSync((IntPtr)dataPtr, (UIntPtr)data.Length, mimePtr);

                    if (resultPtr == IntPtr.Zero)
                    {
                        ThrowLastError();
                    }

                    return ConvertResult(resultPtr);
                }
                finally
                {
                    InteropUtilities.FreeUtf8(mimePtr);
                    InteropUtilities.FreeUtf8(configPtr);
                }
            }
        }
    }

    /// <summary>
    /// Extracts multiple files using the optimized batch pipeline synchronously.
    /// </summary>
    /// <param name="paths">List of file paths to extract from. Must not be null or empty.</param>
    /// <param name="config">Optional extraction configuration applied to all files. If null, uses default settings.</param>
    /// <returns>List of ExtractionResult objects, one per input file, in same order</returns>
    /// <exception cref="ArgumentNullException">If paths is null</exception>
    /// <exception cref="KreuzbergValidationException">If any path is empty or configuration is invalid</exception>
    /// <exception cref="KreuzbergException">If batch extraction fails</exception>
    /// <example>
    /// <code>
    /// var files = new[] { "doc1.pdf", "doc2.docx", "doc3.xlsx" };
    /// var results = KreuzbergClient.BatchExtractFilesSync(files);
    /// foreach (var result in results)
    /// {
    ///     Console.WriteLine($"Content: {result.Content.Length} characters");
    /// }
    /// </code>
    /// </example>
    public static IReadOnlyList<ExtractionResult> BatchExtractFilesSync(IReadOnlyList<string> paths, ExtractionConfig? config = null)
    {
        if (paths == null)
        {
            throw new ArgumentNullException(nameof(paths));
        }
        if (paths.Count == 0)
        {
            return Array.Empty<ExtractionResult>();
        }

        var pathPtrs = new IntPtr[paths.Count];
        for (var i = 0; i < paths.Count; i++)
        {
            if (string.IsNullOrWhiteSpace(paths[i]))
            {
                throw new KreuzbergValidationException($"path at index {i} is empty");
            }
            pathPtrs[i] = InteropUtilities.AllocUtf8(paths[i]);
        }

        var configPtr = SerializeConfig(config);

        try
        {
            var handle = GCHandle.Alloc(pathPtrs, GCHandleType.Pinned);
            try
            {
                var resultPtr = NativeMethods.BatchExtractFilesSync(handle.AddrOfPinnedObject(), (UIntPtr)paths.Count, configPtr);
                if (resultPtr == IntPtr.Zero)
                {
                    ThrowLastError();
                }
                return ConvertBatchResult(resultPtr);
            }
            finally
            {
                handle.Free();
            }
        }
        finally
        {
            foreach (var ptr in pathPtrs)
            {
                InteropUtilities.FreeUtf8(ptr);
            }
            InteropUtilities.FreeUtf8(configPtr);
        }
    }

    /// <summary>
    /// Extracts multiple in-memory documents using the batch pipeline synchronously.
    /// </summary>
    /// <param name="items">List of byte data with MIME types. Must not be null or empty.</param>
    /// <param name="config">Optional extraction configuration applied to all documents. If null, uses default settings.</param>
    /// <returns>List of ExtractionResult objects, one per input document, in same order</returns>
    /// <exception cref="ArgumentNullException">If items is null</exception>
    /// <exception cref="KreuzbergValidationException">If any item is null, data is empty, MIME type is empty, or configuration is invalid</exception>
    /// <exception cref="KreuzbergException">If batch extraction fails</exception>
    /// <example>
    /// <code>
    /// var items = new[]
    /// {
    ///     new BytesWithMime(File.ReadAllBytes("doc1.pdf"), "application/pdf"),
    ///     new BytesWithMime(File.ReadAllBytes("doc2.docx"), "application/vnd.openxmlformats-officedocument.wordprocessingml.document")
    /// };
    /// var results = KreuzbergClient.BatchExtractBytesSync(items);
    /// </code>
    /// </example>
    public static IReadOnlyList<ExtractionResult> BatchExtractBytesSync(IReadOnlyList<BytesWithMime> items, ExtractionConfig? config = null)
    {
        if (items == null)
        {
            throw new ArgumentNullException(nameof(items));
        }
        if (items.Count == 0)
        {
            return Array.Empty<ExtractionResult>();
        }

        var cItems = new NativeMethods.CBytesWithMime[items.Count];
        var pinnedBuffers = new List<GCHandle>(items.Count);
        var mimePtrs = new List<IntPtr>(items.Count);

        try
        {
            for (var i = 0; i < items.Count; i++)
            {
                var item = items[i] ?? throw new KreuzbergValidationException($"item at index {i} is null");
                if (item.Data.Length == 0)
                {
                    throw new KreuzbergValidationException($"data at index {i} is empty");
                }
                if (string.IsNullOrWhiteSpace(item.MimeType))
                {
                    throw new KreuzbergValidationException($"mimeType at index {i} is empty");
                }

                var bufferHandle = GCHandle.Alloc(item.Data, GCHandleType.Pinned);
                pinnedBuffers.Add(bufferHandle);
                var mimePtr = InteropUtilities.AllocUtf8(item.MimeType);
                mimePtrs.Add(mimePtr);

                cItems[i] = new NativeMethods.CBytesWithMime
                {
                    Data = bufferHandle.AddrOfPinnedObject(),
                    DataLen = (UIntPtr)item.Data.Length,
                    MimeType = mimePtr,
                };
            }

            var itemsHandle = GCHandle.Alloc(cItems, GCHandleType.Pinned);
            var configPtr = SerializeConfig(config);
            try
            {
                var resultPtr = NativeMethods.BatchExtractBytesSync(itemsHandle.AddrOfPinnedObject(), (UIntPtr)items.Count, configPtr);
                if (resultPtr == IntPtr.Zero)
                {
                    ThrowLastError();
                }
                return ConvertBatchResult(resultPtr);
            }
            finally
            {
                itemsHandle.Free();
                InteropUtilities.FreeUtf8(configPtr);
            }
        }
        finally
        {
            foreach (var handle in pinnedBuffers)
            {
                if (handle.IsAllocated)
                {
                    handle.Free();
                }
            }
            foreach (var ptr in mimePtrs)
            {
                InteropUtilities.FreeUtf8(ptr);
            }
        }
    }

    /// <summary>
    /// Asynchronously extracts text, metadata, and structured information from a file.
    /// </summary>
    /// <param name="path">Absolute or relative path to the file to extract from. Must not be empty.</param>
    /// <param name="config">Optional extraction configuration. If null, uses default settings.</param>
    /// <param name="cancellationToken">Cancellation token to cancel the operation. Default is CancellationToken.None.</param>
    /// <returns>Task that completes with ExtractionResult containing content, metadata, tables, and detected languages</returns>
    /// <exception cref="ArgumentException">If path is null or empty</exception>
    /// <exception cref="KreuzbergValidationException">If configuration is invalid</exception>
    /// <exception cref="KreuzbergParsingException">If document parsing fails</exception>
    /// <exception cref="KreuzbergOcrException">If OCR processing fails</exception>
    /// <exception cref="KreuzbergException">If extraction fails</exception>
    /// <exception cref="OperationCanceledException">If cancellationToken is canceled</exception>
    /// <example>
    /// <code>
    /// var result = await KreuzbergClient.ExtractFileAsync("document.pdf");
    /// Console.WriteLine(result.Content);
    /// </code>
    /// </example>
    public static Task<ExtractionResult> ExtractFileAsync(string path, ExtractionConfig? config = null, CancellationToken cancellationToken = default)
    {
        return Task.Run(() => ExtractFileSync(path, config), cancellationToken);
    }

    /// <summary>
    /// Asynchronously extracts text, metadata, and structured information from in-memory document bytes.
    /// </summary>
    /// <param name="data">Document bytes to extract from. Must not be null or empty.</param>
    /// <param name="mimeType">MIME type of the document (e.g., "application/pdf"). Must not be empty.</param>
    /// <param name="config">Optional extraction configuration. If null, uses default settings.</param>
    /// <param name="cancellationToken">Cancellation token to cancel the operation. Default is CancellationToken.None.</param>
    /// <returns>Task that completes with ExtractionResult</returns>
    /// <exception cref="KreuzbergValidationException">If data is empty, mimeType is empty, or configuration is invalid</exception>
    /// <exception cref="KreuzbergParsingException">If document parsing fails</exception>
    /// <exception cref="KreuzbergOcrException">If OCR processing fails</exception>
    /// <exception cref="KreuzbergException">If extraction fails</exception>
    /// <exception cref="OperationCanceledException">If cancellationToken is canceled</exception>
    /// <example>
    /// <code>
    /// var data = File.ReadAllBytes("document.pdf");
    /// var result = await KreuzbergClient.ExtractBytesAsync(data, "application/pdf");
    /// Console.WriteLine(result.Content);
    /// </code>
    /// </example>
    public static Task<ExtractionResult> ExtractBytesAsync(byte[] data, string mimeType, ExtractionConfig? config = null, CancellationToken cancellationToken = default)
    {
        return Task.Run(() => ExtractBytesSync(data, mimeType, config), cancellationToken);
    }

    /// <summary>
    /// Asynchronously extracts multiple files using the optimized batch pipeline.
    /// </summary>
    /// <param name="paths">List of file paths to extract from. Must not be null or empty.</param>
    /// <param name="config">Optional extraction configuration applied to all files. If null, uses default settings.</param>
    /// <param name="cancellationToken">Cancellation token to cancel the operation. Default is CancellationToken.None.</param>
    /// <returns>Task that completes with list of ExtractionResult objects, one per input file, in same order</returns>
    /// <exception cref="ArgumentNullException">If paths is null</exception>
    /// <exception cref="KreuzbergValidationException">If any path is empty or configuration is invalid</exception>
    /// <exception cref="KreuzbergException">If batch extraction fails</exception>
    /// <exception cref="OperationCanceledException">If cancellationToken is canceled</exception>
    /// <example>
    /// <code>
    /// var files = new[] { "doc1.pdf", "doc2.pdf", "doc3.pdf" };
    /// var results = await KreuzbergClient.BatchExtractFilesAsync(files);
    /// </code>
    /// </example>
    public static Task<IReadOnlyList<ExtractionResult>> BatchExtractFilesAsync(IReadOnlyList<string> paths, ExtractionConfig? config = null, CancellationToken cancellationToken = default)
    {
        return Task.Run(() => BatchExtractFilesSync(paths, config), cancellationToken);
    }

    /// <summary>
    /// Asynchronously extracts multiple in-memory documents using the batch pipeline.
    /// </summary>
    /// <param name="items">List of byte data with MIME types. Must not be null or empty.</param>
    /// <param name="config">Optional extraction configuration applied to all documents. If null, uses default settings.</param>
    /// <param name="cancellationToken">Cancellation token to cancel the operation. Default is CancellationToken.None.</param>
    /// <returns>Task that completes with list of ExtractionResult objects, one per input document, in same order</returns>
    /// <exception cref="ArgumentNullException">If items is null</exception>
    /// <exception cref="KreuzbergValidationException">If any item is null, data is empty, MIME type is empty, or configuration is invalid</exception>
    /// <exception cref="KreuzbergException">If batch extraction fails</exception>
    /// <exception cref="OperationCanceledException">If cancellationToken is canceled</exception>
    /// <example>
    /// <code>
    /// var items = new[]
    /// {
    ///     new BytesWithMime(File.ReadAllBytes("doc1.pdf"), "application/pdf"),
    ///     new BytesWithMime(File.ReadAllBytes("doc2.docx"), "application/vnd.openxmlformats-officedocument.wordprocessingml.document")
    /// };
    /// var results = await KreuzbergClient.BatchExtractBytesAsync(items);
    /// </code>
    /// </example>
    public static Task<IReadOnlyList<ExtractionResult>> BatchExtractBytesAsync(IReadOnlyList<BytesWithMime> items, ExtractionConfig? config = null, CancellationToken cancellationToken = default)
    {
        return Task.Run(() => BatchExtractBytesSync(items, config), cancellationToken);
    }

    /// <summary>
    /// Loads an extraction configuration from a TOML, YAML, or JSON file.
    /// </summary>
    /// <param name="path">Path to configuration file (must be .toml, .yaml, .yml, or .json). Must not be empty.</param>
    /// <returns>ExtractionConfig deserialized from file</returns>
    /// <exception cref="KreuzbergValidationException">If path is null, empty, or file not found</exception>
    /// <exception cref="KreuzbergException">If configuration file is malformed or cannot be parsed</exception>
    /// <example>
    /// <code>
    /// var config = KreuzbergClient.LoadExtractionConfigFromFile("kreuzberg.toml");
    /// var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
    /// </code>
    /// </example>
    public static ExtractionConfig LoadExtractionConfigFromFile(string path)
    {
        if (string.IsNullOrWhiteSpace(path))
        {
            throw new KreuzbergValidationException("config path cannot be empty");
        }

        var pathPtr = InteropUtilities.AllocUtf8(path);
        try
        {
            var jsonPtr = NativeMethods.LoadExtractionConfigFromFile(pathPtr);
            if (jsonPtr == IntPtr.Zero)
            {
                ThrowLastError();
            }

            var json = InteropUtilities.ReadUtf8(jsonPtr) ?? "{}";
            return Serialization.ParseConfig(json);
        }
        finally
        {
            InteropUtilities.FreeUtf8(pathPtr);
        }
    }

    /// <summary>
    /// Returns the version string of the native Kreuzberg library.
    /// </summary>
    /// <returns>Version string in format "4.0.0-rc1" or similar</returns>
    /// <example>
    /// <code>
    /// var version = KreuzbergClient.GetVersion();
    /// Console.WriteLine($"Kreuzberg {version}");
    /// </code>
    /// </example>
    public static string GetVersion()
    {
        var ptr = NativeMethods.Version();
        return InteropUtilities.ReadUtf8(ptr) ?? "unknown";
    }

    /// <summary>
    /// Registers a custom post-processor to process extraction results.
    /// </summary>
    /// <param name="processor">Implementation of IPostProcessor. Must not be null.</param>
    /// <exception cref="ArgumentNullException">If processor is null</exception>
    /// <exception cref="KreuzbergException">If registration fails</exception>
    /// <remarks>
    /// Post-processors are called after extraction completes and can modify the ExtractionResult.
    /// Multiple processors can be registered and will be called in priority order (higher priority first).
    /// This method is thread-safe and can be called from multiple threads concurrently.
    /// </remarks>
    /// <example>
    /// <code>
    /// public class UppercaseProcessor : IPostProcessor
    /// {
    ///     public string Name => "uppercase";
    ///     public int Priority => 10;
    ///     public ExtractionResult Process(ExtractionResult result)
    ///     {
    ///         result.Content = result.Content.ToUpper();
    ///         return result;
    ///     }
    /// }
    /// KreuzbergClient.RegisterPostProcessor(new UppercaseProcessor());
    /// </code>
    /// </example>
    public static void RegisterPostProcessor(IPostProcessor processor)
    {
        if (processor == null)
        {
            throw new ArgumentNullException(nameof(processor));
        }
        if (string.IsNullOrWhiteSpace(processor.Name))
        {
            throw new KreuzbergValidationException("PostProcessor name cannot be empty");
        }

        NativeMethods.PostProcessorCallback callback = jsonPtr =>
        {
            try
            {
                var inputJson = InteropUtilities.ReadUtf8(jsonPtr) ?? "{}";
                var result = Serialization.ParseResult(inputJson);
                var processed = processor.Process(result);
                var serialized = Serialization.SerializeResult(processed);
                return AllocateReturnString(serialized);
            }
            catch
            {
                return IntPtr.Zero;
            }
        };

        var namePtr = InteropUtilities.AllocUtf8(processor.Name);
        try
        {
            var handle = GCHandle.Alloc(callback);
            if (!NativeMethods.RegisterPostProcessor(namePtr, callback, processor.Priority))
            {
                handle.Free();
                ThrowLastError();
            }

            if (RegisteredPostProcessors.TryGetValue(processor.Name, out var existing))
            {
                if (existing.IsAllocated)
                {
                    existing.Free();
                }
                RegisteredPostProcessors.TryRemove(processor.Name, out _);
            }
            RegisteredPostProcessors[processor.Name] = handle;
        }
        finally
        {
            InteropUtilities.FreeUtf8(namePtr);
        }
    }

    /// <summary>
    /// Lists the names of all registered post-processors.
    /// </summary>
    /// <returns>List of post-processor names</returns>
    /// <example>
    /// <code>
    /// var processors = KreuzbergClient.ListPostProcessors();
    /// Console.WriteLine($"Registered processors: {string.Join(", ", processors)}");
    /// </code>
    /// </example>
    public static IReadOnlyList<string> ListPostProcessors()
    {
        var ptr = NativeMethods.ListPostProcessors();
        return ParseStringListAndFree(ptr);
    }

    /// <summary>
    /// Unregisters and clears all registered post-processors.
    /// </summary>
    /// <exception cref="KreuzbergException">If clearing fails</exception>
    public static void ClearPostProcessors()
    {
        if (!NativeMethods.ClearPostProcessors())
        {
            ThrowLastError();
        }

        FreeHandles(RegisteredPostProcessors);
    }

    /// <summary>
    /// Unregisters a previously registered post-processor by name.
    /// </summary>
    /// <param name="name">Name of the post-processor to unregister. Must not be empty.</param>
    /// <exception cref="ArgumentException">If name is null or empty</exception>
    /// <exception cref="KreuzbergException">If unregistration fails</exception>
    public static void UnregisterPostProcessor(string name)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            throw new ArgumentException("name cannot be empty", nameof(name));
        }

        var namePtr = InteropUtilities.AllocUtf8(name);
        try
        {
            if (!NativeMethods.UnregisterPostProcessor(namePtr))
            {
                ThrowLastError();
            }
        }
        finally
        {
            InteropUtilities.FreeUtf8(namePtr);
            if (RegisteredPostProcessors.TryRemove(name, out var handle) && handle.IsAllocated)
            {
                handle.Free();
            }
        }
    }

    /// <summary>
    /// Registers a custom validator to validate extraction results.
    /// </summary>
    /// <param name="validator">Implementation of IValidator. Must not be null.</param>
    /// <exception cref="ArgumentNullException">If validator is null</exception>
    /// <exception cref="KreuzbergException">If registration fails</exception>
    /// <remarks>
    /// Validators are called after extraction completes and can throw exceptions to reject invalid results.
    /// Multiple validators can be registered and will be called in priority order (higher priority first).
    /// This method is thread-safe and can be called from multiple threads concurrently.
    /// </remarks>
    public static void RegisterValidator(IValidator validator)
    {
        if (validator == null)
        {
            throw new ArgumentNullException(nameof(validator));
        }
        if (string.IsNullOrWhiteSpace(validator.Name))
        {
            throw new KreuzbergValidationException("Validator name cannot be empty");
        }

        NativeMethods.ValidatorCallback callback = jsonPtr =>
        {
            try
            {
                var inputJson = InteropUtilities.ReadUtf8(jsonPtr) ?? "{}";
                var result = Serialization.ParseResult(inputJson);
                validator.Validate(result);
                return IntPtr.Zero;
            }
            catch (Exception ex)
            {
                return AllocateReturnString(ex.Message);
            }
        };

        var namePtr = InteropUtilities.AllocUtf8(validator.Name);
        try
        {
            var handle = GCHandle.Alloc(callback);
            if (!NativeMethods.RegisterValidator(namePtr, callback, validator.Priority))
            {
                handle.Free();
                ThrowLastError();
            }

            if (RegisteredValidators.TryGetValue(validator.Name, out var existing))
            {
                if (existing.IsAllocated)
                {
                    existing.Free();
                }
                RegisteredValidators.TryRemove(validator.Name, out _);
            }
            RegisteredValidators[validator.Name] = handle;
        }
        finally
        {
            InteropUtilities.FreeUtf8(namePtr);
        }
    }

    public static IReadOnlyList<string> ListValidators()
    {
        var ptr = NativeMethods.ListValidators();
        return ParseStringListAndFree(ptr);
    }

    public static void ClearValidators()
    {
        if (!NativeMethods.ClearValidators())
        {
            ThrowLastError();
        }

        FreeHandles(RegisteredValidators);
    }

    public static void UnregisterValidator(string name)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            throw new ArgumentException("name cannot be empty", nameof(name));
        }

        var namePtr = InteropUtilities.AllocUtf8(name);
        try
        {
            if (!NativeMethods.UnregisterValidator(namePtr))
            {
                ThrowLastError();
            }
        }
        finally
        {
            InteropUtilities.FreeUtf8(namePtr);
            if (RegisteredValidators.TryRemove(name, out var handle) && handle.IsAllocated)
            {
                handle.Free();
            }
        }
    }

    /// <summary>
    /// Registers a custom OCR backend implemented in C#.
    /// </summary>
    public static void RegisterOcrBackend(IOcrBackend backend)
    {
        if (backend == null)
        {
            throw new ArgumentNullException(nameof(backend));
        }
        if (string.IsNullOrWhiteSpace(backend.Name))
        {
            throw new KreuzbergValidationException("OcrBackend name cannot be empty");
        }

        NativeMethods.OcrBackendCallback callback = (bytesPtr, length, configPtr) =>
        {
            try
            {
                var resultSpan = ConvertOcrInput(bytesPtr, length);
                var configJson = InteropUtilities.ReadUtf8(configPtr);
                var ocrConfig = string.IsNullOrWhiteSpace(configJson)
                    ? null
                    : JsonSerializer.Deserialize<OcrConfig>(configJson!, Serialization.Options);
                var output = backend.Process(resultSpan, ocrConfig);
                return AllocateReturnString(output);
            }
            catch
            {
                return IntPtr.Zero;
            }
        };

        var namePtr = InteropUtilities.AllocUtf8(backend.Name);
        try
        {
            var handle = GCHandle.Alloc(callback);
            if (!NativeMethods.RegisterOcrBackend(namePtr, callback))
            {
                handle.Free();
                ThrowLastError();
            }

            if (RegisteredOcrBackends.TryGetValue(backend.Name, out var existing))
            {
                if (existing.IsAllocated)
                {
                    existing.Free();
                }
                RegisteredOcrBackends.TryRemove(backend.Name, out _);
            }
            RegisteredOcrBackends[backend.Name] = handle;
        }
        finally
        {
            InteropUtilities.FreeUtf8(namePtr);
        }
    }

    public static IReadOnlyList<string> ListOcrBackends()
    {
        var ptr = NativeMethods.ListOcrBackends();
        return ParseStringListAndFree(ptr);
    }

    public static void ClearOcrBackends()
    {
        if (!NativeMethods.ClearOcrBackends())
        {
            ThrowLastError();
        }

        FreeHandles(RegisteredOcrBackends);
    }

    public static void UnregisterOcrBackend(string name)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            throw new ArgumentException("name cannot be empty", nameof(name));
        }

        var namePtr = InteropUtilities.AllocUtf8(name);
        try
        {
            if (!NativeMethods.UnregisterOcrBackend(namePtr))
            {
                ThrowLastError();
            }
        }
        finally
        {
            InteropUtilities.FreeUtf8(namePtr);
            if (RegisteredOcrBackends.TryRemove(name, out var handle) && handle.IsAllocated)
            {
                handle.Free();
            }
        }
    }

    public static IReadOnlyList<string> ListDocumentExtractors()
    {
        var ptr = NativeMethods.ListDocumentExtractors();
        return ParseStringListAndFree(ptr);
    }

    public static void UnregisterDocumentExtractor(string name)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            throw new ArgumentException("name cannot be empty", nameof(name));
        }

        var namePtr = InteropUtilities.AllocUtf8(name);
        try
        {
            if (!NativeMethods.UnregisterDocumentExtractor(namePtr))
            {
                ThrowLastError();
            }
        }
        finally
        {
            InteropUtilities.FreeUtf8(namePtr);
        }
    }

    public static void ClearDocumentExtractors()
    {
        if (!NativeMethods.ClearDocumentExtractors())
        {
            ThrowLastError();
        }
    }

    /// <summary>
    /// Lists the names of all available embedding presets.
    /// </summary>
    /// <returns>List of embedding preset names (e.g., ["default", "openai", "sentence-transformers"])</returns>
    /// <exception cref="KreuzbergException">If preset enumeration fails</exception>
    /// <example>
    /// <code>
    /// var presets = KreuzbergClient.ListEmbeddingPresets();
    /// Console.WriteLine($"Available: {string.Join(", ", presets)}");
    /// </code>
    /// </example>
    public static List<string> ListEmbeddingPresets()
    {
        var resultPtr = NativeMethods.ListEmbeddingPresets();
        if (resultPtr == IntPtr.Zero)
            ThrowLastError();

        try
        {
            var json = InteropUtilities.ReadUtf8(resultPtr) ?? "[]";
            return JsonSerializer.Deserialize<List<string>>(json, Serialization.Options) ?? new List<string>();
        }
        finally
        {
            NativeMethods.FreeString(resultPtr);
        }
    }

    /// <summary>
    /// Gets an embedding preset by name.
    /// </summary>
    /// <param name="name">The name of the embedding preset (e.g., "default", "openai"). Must not be empty.</param>
    /// <returns>The EmbeddingPreset with matching name, or null if not found</returns>
    /// <exception cref="KreuzbergValidationException">If name is null or empty</exception>
    /// <exception cref="KreuzbergException">If preset retrieval fails</exception>
    /// <example>
    /// <code>
    /// var preset = KreuzbergClient.GetEmbeddingPreset("default");
    /// if (preset != null)
    /// {
    ///     Console.WriteLine($"Model: {preset.ModelName}");
    ///     Console.WriteLine($"Dimensions: {preset.Dimensions}");
    ///     Console.WriteLine($"Chunk Size: {preset.ChunkSize}");
    /// }
    /// </code>
    /// </example>
    public static EmbeddingPreset? GetEmbeddingPreset(string name)
    {
        if (string.IsNullOrWhiteSpace(name))
            throw new KreuzbergValidationException("preset name cannot be empty");

        var namePtr = InteropUtilities.AllocUtf8(name);
        try
        {
            var resultPtr = NativeMethods.GetEmbeddingPreset(namePtr);
            if (resultPtr == IntPtr.Zero)
                return null;

            var json = InteropUtilities.ReadUtf8(resultPtr) ?? "null";
            NativeMethods.FreeString(resultPtr);
            return JsonSerializer.Deserialize<EmbeddingPreset>(json, Serialization.Options);
        }
        finally
        {
            InteropUtilities.FreeUtf8(namePtr);
        }
    }

    private static unsafe ReadOnlySpan<byte> ConvertOcrInput(IntPtr bytesPtr, UIntPtr length)
    {
        if (bytesPtr == IntPtr.Zero || length == UIntPtr.Zero)
        {
            return ReadOnlySpan<byte>.Empty;
        }

        if (length > (UIntPtr)int.MaxValue)
        {
            throw new KreuzbergException(
                KreuzbergErrorKind.Validation,
                $"Image data too large for processing (max {int.MaxValue} bytes)"
            );
        }

        return new ReadOnlySpan<byte>((void*)bytesPtr, (int)length);
    }

    private static ExtractionResult ConvertResult(IntPtr resultPtr, bool free = true)
    {
        try
        {
            var cRes = Marshal.PtrToStructure<NativeMethods.CExtractionResult>(resultPtr);
            var result = new ExtractionResult
            {
                Content = InteropUtilities.ReadUtf8(cRes.Content) ?? string.Empty,
                MimeType = InteropUtilities.ReadUtf8(cRes.MimeType) ?? string.Empty,
                Success = cRes.Success,
            };

            result.Tables = DeserializeField<List<Table>>(cRes.TablesJson) ?? new List<Table>();
            result.DetectedLanguages = DeserializeField<List<string>>(cRes.DetectedLanguagesJson);
            result.Metadata = Serialization.ParseMetadata(InteropUtilities.ReadUtf8(cRes.MetadataJson));
            result.Chunks = DeserializeField<List<Chunk>>(cRes.ChunksJson);
            result.Images = DeserializeField<List<ExtractedImage>>(cRes.ImagesJson);

            if (result.Metadata.Pages == null && cRes.PageStructureJson != IntPtr.Zero)
            {
                result.Metadata.Pages = DeserializeField<PageStructure>(cRes.PageStructureJson);
            }
            if (result.Metadata.Language == null && cRes.Language != IntPtr.Zero)
            {
                result.Metadata.Language = InteropUtilities.ReadUtf8(cRes.Language);
            }
            if (result.Metadata.Date == null && cRes.Date != IntPtr.Zero)
            {
                result.Metadata.Date = InteropUtilities.ReadUtf8(cRes.Date);
            }
            if (result.Metadata.Subject == null && cRes.Subject != IntPtr.Zero)
            {
                result.Metadata.Subject = InteropUtilities.ReadUtf8(cRes.Subject);
            }

            return result;
        }
        finally
        {
            if (free)
            {
                NativeMethods.FreeResult(resultPtr);
            }
        }
    }

    private static IReadOnlyList<ExtractionResult> ConvertBatchResult(IntPtr batchPtr)
    {
        try
        {
            var cBatch = Marshal.PtrToStructure<NativeMethods.CBatchResult>(batchPtr);
            var count = checked((int)cBatch.Count);
            if (count == 0 || cBatch.Results == IntPtr.Zero)
            {
                return Array.Empty<ExtractionResult>();
            }

            var resultPtrs = InteropUtilities.ReadPointerArray(cBatch.Results, count);
            var results = new List<ExtractionResult>(count);
            foreach (var ptr in resultPtrs)
            {
                results.Add(ptr == IntPtr.Zero ? new ExtractionResult { Success = false } : ConvertResult(ptr, free: false));
            }
            return results;
        }
        finally
        {
            NativeMethods.FreeBatchResult(batchPtr);
        }
    }

    private static IntPtr SerializeConfig(ExtractionConfig? config)
    {
        if (config == null)
        {
            return IntPtr.Zero;
        }
        var json = JsonSerializer.Serialize(config, Serialization.Options);
        return InteropUtilities.AllocUtf8(json);
    }

    private static T? DeserializeField<T>(IntPtr ptr)
    {
        var json = InteropUtilities.ReadUtf8(ptr);
        if (string.IsNullOrWhiteSpace(json))
        {
            return default;
        }

        try
        {
            return JsonSerializer.Deserialize<T>(json, Serialization.Options);
        }
        catch (JsonException ex)
        {
            throw new KreuzbergSerializationException($"failed to deserialize payload: {ex.Message}", ex);
        }
    }

    private static void ThrowLastError()
    {
        var errorPtr = NativeMethods.LastError();
        var message = InteropUtilities.ReadUtf8(errorPtr);
        throw ErrorMapper.FromNativeError(message);
    }

    private static unsafe IntPtr AllocateReturnString(string value)
    {
        if (value == null)
        {
            return IntPtr.Zero;
        }

        var bytes = Encoding.UTF8.GetBytes(value);
        var tmp = (byte*)NativeMemory.Alloc((nuint)(bytes.Length + 1));
        try
        {
            bytes.CopyTo(new Span<byte>(tmp, bytes.Length));
            tmp[bytes.Length] = 0;
            return NativeMethods.CloneString((IntPtr)tmp);
        }
        finally
        {
            NativeMemory.Free(tmp);
        }
    }

    private static IReadOnlyList<string> ParseStringListAndFree(IntPtr ptr)
    {
        if (ptr == IntPtr.Zero)
        {
            return Array.Empty<string>();
        }

        var json = InteropUtilities.ReadUtf8(ptr);
        NativeMethods.FreeString(ptr);
        if (string.IsNullOrWhiteSpace(json))
        {
            return Array.Empty<string>();
        }

        try
        {
            var parsed = JsonSerializer.Deserialize<List<string>>(json, Serialization.Options);
            return parsed ?? new List<string>();
        }
        catch (JsonException ex)
        {
            throw new KreuzbergSerializationException($"failed to parse string list: {ex.Message}", ex);
        }
    }

    private static void FreeHandles(ConcurrentDictionary<string, GCHandle> handles)
    {
        foreach (var handle in handles.Values)
        {
            if (handle.IsAllocated)
            {
                handle.Free();
            }
        }
        handles.Clear();
    }
}
