using System;
using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.CompilerServices;

namespace Kreuzberg;

internal static partial class NativeMethods
{
    private const string LibraryName = "kreuzberg_ffi";
    private static readonly Lazy<IntPtr> LibraryHandle = new(() => LoadNativeLibrary());

    [ModuleInitializer]
    [SuppressMessage("Usage", "CA2255:The 'ModuleInitializer' attribute should not be used in libraries",
        Justification = "Required for native library resolution before P/Invoke calls. NativeLibrary.SetDllImportResolver must run before any P/Invoke.")]
    internal static void InitResolver()
    {
        NativeLibrary.SetDllImportResolver(typeof(NativeMethods).Assembly, ResolveLibrary);
    }

    [StructLayout(LayoutKind.Sequential)]
    internal struct CExtractionResult
    {
        public IntPtr Content;
        public IntPtr MimeType;
        public IntPtr Language;
        public IntPtr Date;
        public IntPtr Subject;
        public IntPtr TablesJson;
        public IntPtr DetectedLanguagesJson;
        public IntPtr MetadataJson;
        public IntPtr ChunksJson;
        public IntPtr ImagesJson;
        public IntPtr PageStructureJson;

        [MarshalAs(UnmanagedType.I1)]
        public bool Success;
    }

    [StructLayout(LayoutKind.Sequential)]
    internal struct CBatchResult
    {
        public IntPtr Results;
        public UIntPtr Count;

        [MarshalAs(UnmanagedType.I1)]
        public bool Success;
    }

    [StructLayout(LayoutKind.Sequential)]
    internal struct CBytesWithMime
    {
        public IntPtr Data;
        public UIntPtr DataLen;
        public IntPtr MimeType;
    }

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    internal delegate IntPtr OcrBackendCallback(IntPtr imageBytes, UIntPtr imageLength, IntPtr configJson);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    internal delegate IntPtr PostProcessorCallback(IntPtr resultJson);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    internal delegate IntPtr ValidatorCallback(IntPtr resultJson);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_detect_mime_type_from_bytes", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr DetectMimeTypeFromBytes(IntPtr data, UIntPtr dataLen);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_detect_mime_type_from_path", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr DetectMimeTypeFromPath(IntPtr filePath);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_get_extensions_for_mime", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr GetExtensionsForMime(IntPtr mimeType);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_config_discover", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ConfigDiscover();

    [DllImport(LibraryName, EntryPoint = "kreuzberg_extract_file_sync", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ExtractFileSync(IntPtr filePath);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_extract_file_sync_with_config", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ExtractFileSyncWithConfig(IntPtr filePath, IntPtr configJson);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_extract_bytes_sync", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ExtractBytesSync(IntPtr data, UIntPtr dataLen, IntPtr mimeType);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_extract_bytes_sync_with_config", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ExtractBytesSyncWithConfig(IntPtr data, UIntPtr dataLen, IntPtr mimeType, IntPtr configJson);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_batch_extract_files_sync", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr BatchExtractFilesSync(IntPtr filePaths, UIntPtr count, IntPtr configJson);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_batch_extract_bytes_sync", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr BatchExtractBytesSync(IntPtr items, UIntPtr count, IntPtr configJson);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_load_extraction_config_from_file", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr LoadExtractionConfigFromFile(IntPtr filePath);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_free_batch_result", CallingConvention = CallingConvention.Cdecl)]
    internal static extern void FreeBatchResult(IntPtr batchResult);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_free_string", CallingConvention = CallingConvention.Cdecl)]
    internal static extern void FreeString(IntPtr ptr);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_free_result", CallingConvention = CallingConvention.Cdecl)]
    internal static extern void FreeResult(IntPtr result);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_last_error", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr LastError();

    [DllImport(LibraryName, EntryPoint = "kreuzberg_version", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr Version();

    [DllImport(LibraryName, EntryPoint = "kreuzberg_register_ocr_backend", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool RegisterOcrBackend(IntPtr name, OcrBackendCallback callback);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_register_post_processor", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool RegisterPostProcessor(IntPtr name, PostProcessorCallback callback, int priority);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_unregister_ocr_backend", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool UnregisterOcrBackend(IntPtr name);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_list_ocr_backends", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ListOcrBackends();

    [DllImport(LibraryName, EntryPoint = "kreuzberg_clear_ocr_backends", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool ClearOcrBackends();

    [DllImport(LibraryName, EntryPoint = "kreuzberg_unregister_post_processor", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool UnregisterPostProcessor(IntPtr name);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_clear_post_processors", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool ClearPostProcessors();

    [DllImport(LibraryName, EntryPoint = "kreuzberg_list_post_processors", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ListPostProcessors();

    [DllImport(LibraryName, EntryPoint = "kreuzberg_register_validator", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool RegisterValidator(IntPtr name, ValidatorCallback callback, int priority);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_unregister_validator", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool UnregisterValidator(IntPtr name);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_clear_validators", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool ClearValidators();

    [DllImport(LibraryName, EntryPoint = "kreuzberg_list_validators", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ListValidators();

    [DllImport(LibraryName, EntryPoint = "kreuzberg_list_document_extractors", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ListDocumentExtractors();

    [DllImport(LibraryName, EntryPoint = "kreuzberg_clear_document_extractors", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool ClearDocumentExtractors();

    [DllImport(LibraryName, EntryPoint = "kreuzberg_unregister_document_extractor", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool UnregisterDocumentExtractor(IntPtr name);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_list_embedding_presets", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr ListEmbeddingPresets();

    [DllImport(LibraryName, EntryPoint = "kreuzberg_get_embedding_preset", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr GetEmbeddingPreset(IntPtr name);

    private static IntPtr ResolveLibrary(string libraryName, Assembly assembly, DllImportSearchPath? _)
    {
        if (!string.Equals(libraryName, LibraryName, StringComparison.Ordinal))
        {
            return IntPtr.Zero;
        }

        return LibraryHandle.Value;
    }

    private static IntPtr LoadNativeLibrary()
    {
        var fileName = GetLibraryFileName();
        var debug = Environment.GetEnvironmentVariable("KREUZBERG_BENCHMARK_DEBUG") == "true";
        var probePaths = GetProbePaths(fileName).ToList();

        if (debug)
        {
            System.Console.Error.WriteLine($"[DEBUG] Looking for native library: {fileName}");
            System.Console.Error.WriteLine($"[DEBUG] Probe paths:");
            foreach (var path in probePaths)
            {
                var exists = System.IO.File.Exists(path);
                System.Console.Error.WriteLine($"[DEBUG]   {path} (exists: {exists})");
            }
        }

        foreach (var path in probePaths)
        {
            if (NativeLibrary.TryLoad(path, out var handle))
            {
                if (debug)
                {
                    System.Console.Error.WriteLine($"[DEBUG] Successfully loaded native library from: {path}");
                }
                return handle;
            }
        }

        var pathsStr = string.Join(", ", probePaths);
        throw new DllNotFoundException($"Unable to locate {fileName}. Checked: {pathsStr}. Set KREUZBERG_FFI_DIR or place the library in target/release.");
    }

    private static IEnumerable<string> GetProbePaths(string fileName)
    {
        var envDir = Environment.GetEnvironmentVariable("KREUZBERG_FFI_DIR");
        if (!string.IsNullOrWhiteSpace(envDir))
        {
            yield return Path.Combine(envDir, fileName);
        }

        // Current base directory (e.g., test bin)
        yield return Path.Combine(AppContext.BaseDirectory, fileName);

        var cwd = Directory.GetCurrentDirectory();
        yield return Path.Combine(cwd, fileName);

        // Also check CWD for target subdirectories
        var cwdRelease = Path.Combine(cwd, "target", "release", fileName);
        if (File.Exists(cwdRelease))
        {
            yield return cwdRelease;
        }

        var cwdDebug = Path.Combine(cwd, "target", "debug", fileName);
        if (File.Exists(cwdDebug))
        {
            yield return cwdDebug;
        }

        // Walk up from base directory to workspace target/{release,debug}
        string? dir = AppContext.BaseDirectory;
        for (var i = 0; i < 5 && dir != null; i++)
        {
            var release = Path.Combine(dir, "target", "release", fileName);
            if (File.Exists(release))
            {
                yield return release;
            }

            var debugPath = Path.Combine(dir, "target", "debug", fileName);
            if (File.Exists(debugPath))
            {
                yield return debugPath;
            }

            dir = Directory.GetParent(dir)?.FullName;
        }
    }

    private static string GetLibraryFileName()
    {
        if (OperatingSystem.IsWindows())
        {
            return "kreuzberg_ffi.dll";
        }

        if (OperatingSystem.IsMacOS())
        {
            return "libkreuzberg_ffi.dylib";
        }

        return "libkreuzberg_ffi.so";
    }
}
