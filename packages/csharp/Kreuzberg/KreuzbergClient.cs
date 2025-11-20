using System.Collections.Concurrent;
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
    /// Extracts text and metadata from a file.
    /// </summary>
    /// <param name="path">Path to the file.</param>
    /// <param name="config">Optional extraction configuration.</param>
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
    /// Extracts text and metadata from an in-memory document.
    /// </summary>
    /// <param name="data">Document bytes.</param>
    /// <param name="mimeType">MIME type of the content.</param>
    /// <param name="config">Optional extraction configuration.</param>
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
    /// Extracts multiple files using the optimized batch pipeline.
    /// </summary>
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
    /// Extracts multiple in-memory documents using the batch pipeline.
    /// </summary>
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
    /// Asynchronously extracts a file (Task wrapper over sync API).
    /// </summary>
    public static Task<ExtractionResult> ExtractFileAsync(string path, ExtractionConfig? config = null, CancellationToken cancellationToken = default)
    {
        return Task.Run(() => ExtractFileSync(path, config), cancellationToken);
    }

    /// <summary>
    /// Asynchronously extracts bytes (Task wrapper over sync API).
    /// </summary>
    public static Task<ExtractionResult> ExtractBytesAsync(byte[] data, string mimeType, ExtractionConfig? config = null, CancellationToken cancellationToken = default)
    {
        return Task.Run(() => ExtractBytesSync(data, mimeType, config), cancellationToken);
    }

    /// <summary>
    /// Asynchronously extracts multiple files.
    /// </summary>
    public static Task<IReadOnlyList<ExtractionResult>> BatchExtractFilesAsync(IReadOnlyList<string> paths, ExtractionConfig? config = null, CancellationToken cancellationToken = default)
    {
        return Task.Run(() => BatchExtractFilesSync(paths, config), cancellationToken);
    }

    /// <summary>
    /// Asynchronously extracts multiple byte documents.
    /// </summary>
    public static Task<IReadOnlyList<ExtractionResult>> BatchExtractBytesAsync(IReadOnlyList<BytesWithMime> items, ExtractionConfig? config = null, CancellationToken cancellationToken = default)
    {
        return Task.Run(() => BatchExtractBytesSync(items, config), cancellationToken);
    }

    /// <summary>
    /// Loads an extraction configuration from a TOML/YAML/JSON file.
    /// </summary>
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
    /// Returns the Kreuzberg version string from the native library.
    /// </summary>
    public static string GetVersion()
    {
        var ptr = NativeMethods.Version();
        return InteropUtilities.ReadUtf8(ptr) ?? "unknown";
    }

    /// <summary>
    /// Registers a custom PostProcessor implemented in C#.
    /// </summary>
    public static void RegisterPostProcessor(IPostProcessor processor)
    {
        if (processor == null)
        {
            throw new ArgumentNullException(nameof(processor));
        }
        var namePtr = InteropUtilities.AllocUtf8(processor.Name);
        NativeMethods.PostProcessorCallback callback = jsonPtr =>
        {
            var inputJson = InteropUtilities.ReadUtf8(jsonPtr) ?? "{}";
            var result = Serialization.ParseResult(inputJson);
            var processed = processor.Process(result);
            var serialized = Encoding.UTF8.GetBytes(Serialization.SerializeResult(processed));
            return AllocateReturnString(serialized);
        };

        var handle = GCHandle.Alloc(callback);
        if (!NativeMethods.RegisterPostProcessor(namePtr, callback, processor.Priority))
        {
            handle.Free();
            InteropUtilities.FreeUtf8(namePtr);
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
        InteropUtilities.FreeUtf8(namePtr);
    }

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
    /// Registers a custom Validator implemented in C#.
    /// </summary>
    public static void RegisterValidator(IValidator validator)
    {
        if (validator == null)
        {
            throw new ArgumentNullException(nameof(validator));
        }

        var namePtr = InteropUtilities.AllocUtf8(validator.Name);
        NativeMethods.ValidatorCallback callback = jsonPtr =>
        {
            var inputJson = InteropUtilities.ReadUtf8(jsonPtr) ?? "{}";
            var result = Serialization.ParseResult(inputJson);
            try
            {
                validator.Validate(result);
                return IntPtr.Zero;
            }
            catch (Exception ex)
            {
                var bytes = Encoding.UTF8.GetBytes(ex.Message);
                return AllocateReturnString(bytes);
            }
        };

        var handle = GCHandle.Alloc(callback);
        if (!NativeMethods.RegisterValidator(namePtr, callback, validator.Priority))
        {
            handle.Free();
            InteropUtilities.FreeUtf8(namePtr);
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
        InteropUtilities.FreeUtf8(namePtr);
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

        var namePtr = InteropUtilities.AllocUtf8(backend.Name);
        NativeMethods.OcrBackendCallback callback = (bytesPtr, length, configPtr) =>
        {
            var resultSpan = ConvertOcrInput(bytesPtr, length);
            var configJson = InteropUtilities.ReadUtf8(configPtr);
            var ocrConfig = string.IsNullOrWhiteSpace(configJson)
                ? null
                : JsonSerializer.Deserialize<OcrConfig>(configJson!, Serialization.Options);
            var output = backend.Process(resultSpan, ocrConfig);
            var bytes = Encoding.UTF8.GetBytes(output);
            return AllocateReturnString(bytes);
        };

        var handle = GCHandle.Alloc(callback);
        if (!NativeMethods.RegisterOcrBackend(namePtr, callback))
        {
            handle.Free();
            InteropUtilities.FreeUtf8(namePtr);
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
        InteropUtilities.FreeUtf8(namePtr);
    }

    private static unsafe ReadOnlySpan<byte> ConvertOcrInput(IntPtr bytesPtr, UIntPtr length)
    {
        if (bytesPtr == IntPtr.Zero || length == UIntPtr.Zero)
        {
            return ReadOnlySpan<byte>.Empty;
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

    private static unsafe IntPtr AllocateReturnString(byte[] bytes)
    {
        var buffer = (byte*)NativeMemory.Alloc((nuint)(bytes.Length + 1));
        bytes.CopyTo(new Span<byte>(buffer, bytes.Length));
        buffer[bytes.Length] = 0;
        return (IntPtr)buffer;
    }
}
