using System.Runtime.InteropServices;

namespace Kreuzberg;

internal static partial class NativeMethods
{
    private const string LibraryName = "kreuzberg_ffi";

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

    [DllImport(LibraryName, EntryPoint = "kreuzberg_unregister_post_processor", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool UnregisterPostProcessor(IntPtr name);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_register_validator", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool RegisterValidator(IntPtr name, ValidatorCallback callback, int priority);

    [DllImport(LibraryName, EntryPoint = "kreuzberg_unregister_validator", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool UnregisterValidator(IntPtr name);
}
