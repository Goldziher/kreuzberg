# Kreuzberg C# Bindings Architecture

This document describes the design and implementation of the Kreuzberg C# binding, focusing on the P/Invoke interoperability layer, memory management, error handling, and thread safety patterns.

## Overview

The C# binding for Kreuzberg provides a safe, idiomatic .NET interface to the high-performance Rust extraction engine. The architecture follows a thin wrapper pattern:

```
C# Application
     ↓
KreuzbergClient (Public API)
     ↓
InteropUtilities (UTF-8 Marshaling)
     ↓
NativeMethods (P/Invoke Declarations)
     ↓
kreuzberg-ffi (Native C Library)
     ↓
Rust Core (kreuzberg crate)
```

Key principles:
- **Rust core is the single source of truth** – All extraction logic lives in Rust
- **C# provides language-idiomatic wrappers** – Configuration builders, async/await, exceptions
- **Safe P/Invoke patterns** – No unsafe code in public API, proper memory management
- **Full thread safety** – Concurrent calls are safe with thread-safe collections for plugin registration

## Architecture Components

### 1. NativeMethods Pattern

**File**: `NativeMethods.cs`

Declares P/Invoke wrappers to the native C library (`kreuzberg-ffi`):

```csharp
internal static class NativeMethods
{
    private const string DllName = "kreuzberg_ffi";

    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr ExtractFileSync(IntPtr pathPtr);

    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr LastError();

    // ... more declarations
}
```

**Design decisions**:
- All pointers are `IntPtr` (not `void*` for safety)
- Return null pointers indicate errors (call `LastError()` to get message)
- No automatic marshaling – explicit `InteropUtilities` calls for strings
- `CallingConvention.Cdecl` for cross-platform compatibility

### 2. InteropUtilities

**File**: `InteropUtilities.cs`

Provides safe UTF-8 string marshaling between managed and native code:

#### AllocUtf8 (Managed → Native)
```csharp
public static IntPtr AllocUtf8(string? str)
{
    if (string.IsNullOrEmpty(str))
        return IntPtr.Zero;

    var bytes = Encoding.UTF8.GetBytes(str);
    var buffer = (byte*)NativeMemory.Alloc((nuint)(bytes.Length + 1));
    bytes.CopyTo(new Span<byte>(buffer, bytes.Length));
    buffer[bytes.Length] = 0;  // null terminator
    return (IntPtr)buffer;
}
```

**Guarantees**:
- Null-terminated UTF-8 strings (C standard)
- Safe native memory allocation using `NativeMemory`
- Caller responsible for calling `FreeUtf8()` in finally block

#### ReadUtf8 (Native → Managed)
```csharp
public static string? ReadUtf8(IntPtr ptr)
{
    if (ptr == IntPtr.Zero)
        return null;

    var length = strlen((byte*)ptr);  // C standard strlen
    if (length == 0)
        return string.Empty;

    return Encoding.UTF8.GetString((byte*)ptr, length);
}
```

**Guarantees**:
- Safe null pointer handling
- Correct UTF-8 decoding
- No buffer overruns (uses `strlen` for length detection)

#### FreeUtf8 & FreeString
```csharp
public static void FreeUtf8(IntPtr ptr)
{
    if (ptr != IntPtr.Zero)
        NativeMemory.Free((void*)ptr);
}

public static void FreeString(IntPtr ptr)
{
    // Frees strings allocated by native library
    NativeMethods.FreeString(ptr);
}
```

**Pattern**: Always pair allocation with deallocation in try/finally:
```csharp
var ptr = InteropUtilities.AllocUtf8(input);
try
{
    // use ptr
}
finally
{
    InteropUtilities.FreeUtf8(ptr);
}
```

### 3. Memory Management Strategy

The C# binding uses **RAII (Resource Acquisition Is Initialization)** pattern with try/finally blocks:

#### For Single Strings
```csharp
public static string DetectMimeTypeFromPath(string path)
{
    var pathPtr = InteropUtilities.AllocUtf8(path);  // ACQUIRE
    try
    {
        var resultPtr = NativeMethods.DetectMimeTypeFromPath(pathPtr);
        if (resultPtr == IntPtr.Zero)
            ThrowLastError();

        var mime = InteropUtilities.ReadUtf8(resultPtr) ?? string.Empty;
        NativeMethods.FreeString(resultPtr);  // RELEASE native result
        return mime;
    }
    finally
    {
        InteropUtilities.FreeUtf8(pathPtr);  // RELEASE input
    }
}
```

#### For Multiple String Arrays
```csharp
var pathPtrs = new IntPtr[paths.Count];
for (var i = 0; i < paths.Count; i++)
{
    pathPtrs[i] = InteropUtilities.AllocUtf8(paths[i]);
}

var handle = GCHandle.Alloc(pathPtrs, GCHandleType.Pinned);
try
{
    // Use handle.AddrOfPinnedObject() for native call
}
finally
{
    handle.Free();
    foreach (var ptr in pathPtrs)
    {
        InteropUtilities.FreeUtf8(ptr);
    }
}
```

#### For Marshaling Binary Data
```csharp
unsafe
{
    fixed (byte* dataPtr = data)
    {
        try
        {
            var resultPtr = NativeMethods.ExtractBytesSync(
                (IntPtr)dataPtr,
                (UIntPtr)data.Length,
                mimePtr
            );
            // Process result
        }
        finally
        {
            // fixed block and scope exit handle deallocation
        }
    }
}
```

### 4. Error Handling Strategy

All errors from the native library are converted to C# exceptions using the `ErrorMapper` pattern:

**Hierarchy**:
```
KreuzbergException (base)
├── KreuzbergValidationException
├── KreuzbergParsingException
├── KreuzbergOcrException
├── KreuzbergMissingDependencyException
└── KreuzbergSerializationException
```

**Implementation**:
```csharp
private static void ThrowLastError()
{
    var errorPtr = NativeMethods.LastError();
    var message = InteropUtilities.ReadUtf8(errorPtr);
    throw ErrorMapper.FromNativeError(message);
}

public static class ErrorMapper
{
    public static Exception FromNativeError(string? message)
    {
        var msg = message ?? "unknown error";

        return msg switch
        {
            _ when msg.Contains("validation") => new KreuzbergValidationException(msg),
            _ when msg.Contains("parsing") => new KreuzbergParsingException(msg),
            _ when msg.Contains("ocr") => new KreuzbergOcrException(msg),
            _ when msg.Contains("dependency") => new KreuzbergMissingDependencyException(msg),
            _ => new KreuzbergException(msg)
        };
    }
}
```

**Contract**:
- All public methods return `IntPtr.Zero` on error
- Call `ThrowLastError()` immediately to get error message
- Error message is only valid until next native call
- Each FFI boundary has validation and error handling

### 5. JSON Serialization

**File**: `Serialization.cs`

Uses `System.Text.Json` with custom `JsonSerializerOptions`:

```csharp
public static class Serialization
{
    public static JsonSerializerOptions Options { get; } = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        PropertyNameCaseInsensitive = true,
        WriteIndented = false,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
    };
}
```

**Rationale**:
- Snake_case naming matches Rust/native conventions
- Case-insensitive parsing for robustness
- Null values omitted for compact serialization
- No external dependencies (System.Text.Json included in .NET 6+)

**Config Serialization**:
```csharp
private static IntPtr SerializeConfig(ExtractionConfig? config)
{
    if (config == null)
        return IntPtr.Zero;

    var json = JsonSerializer.Serialize(config, Serialization.Options);
    return InteropUtilities.AllocUtf8(json);
}
```

**Result Deserialization**:
```csharp
var cRes = Marshal.PtrToStructure<NativeMethods.CExtractionResult>(resultPtr);
result.Tables = DeserializeField<List<Table>>(cRes.TablesJson) ?? new List<Table>();
result.DetectedLanguages = DeserializeField<List<string>>(cRes.DetectedLanguagesJson);
```

### 6. Thread Safety

All public APIs are thread-safe through:

#### Static Method Thread Safety
- `KreuzbergClient` is entirely static
- No mutable instance state
- Each call operates on its own memory context
- Safe concurrent calls from multiple threads

#### Plugin Registry Thread Safety
```csharp
private static readonly ConcurrentDictionary<string, GCHandle> RegisteredPostProcessors
    = new(StringComparer.OrdinalIgnoreCase);
```

- `ConcurrentDictionary` for atomic operations
- `GCHandle` pinning ensures callback delegates stay alive
- Unregister safely removes and frees handles:

```csharp
public static void UnregisterPostProcessor(string name)
{
    // ...
    if (RegisteredPostProcessors.TryRemove(name, out var handle) && handle.IsAllocated)
    {
        handle.Free();
    }
}
```

#### Callback Thread Safety
```csharp
NativeMethods.PostProcessorCallback callback = jsonPtr =>
{
    var inputJson = InteropUtilities.ReadUtf8(jsonPtr) ?? "{}";
    var result = Serialization.ParseResult(inputJson);
    var processed = processor.Process(result);  // May be called from Rust thread
    var serialized = Encoding.UTF8.GetBytes(Serialization.SerializeResult(processed));
    return AllocateReturnString(serialized);
};
```

- Callbacks may execute from any thread (Rust scheduler)
- No shared mutable state access
- Each callback is stateless (functional design)
- Memory allocation per callback invocation

### 7. Async/Await Pattern

**Design**: Task wrappers over sync P/Invoke calls:

```csharp
public static Task<ExtractionResult> ExtractFileAsync(
    string path,
    ExtractionConfig? config = null,
    CancellationToken cancellationToken = default)
{
    return Task.Run(() => ExtractFileSync(path, config), cancellationToken);
}
```

**Rationale**:
- Avoids blocking thread pool threads
- `Task.Run` offloads to background thread
- Supports `CancellationToken` for cancellation
- No async P/Invoke needed (sync calls are fast)

**Benefits**:
- UI applications can await without blocking
- Compatible with async/await patterns
- `ConfigureAwait(false)` optimization available
- Cancellation support via `CancellationToken`

### 8. Batch Processing

**Design**: Optimized array marshaling for bulk operations:

```csharp
var pathPtrs = new IntPtr[paths.Count];
for (var i = 0; i < paths.Count; i++)
{
    pathPtrs[i] = InteropUtilities.AllocUtf8(paths[i]);
}

var handle = GCHandle.Alloc(pathPtrs, GCHandleType.Pinned);
try
{
    var resultPtr = NativeMethods.BatchExtractFilesSync(
        handle.AddrOfPinnedObject(),
        (UIntPtr)paths.Count,
        configPtr
    );
}
finally
{
    handle.Free();
    // Free all string pointers
}
```

**Performance**:
- Single array allocation for paths
- Pinned to prevent GC moves
- Single native call (no per-item overhead)
- Batch processing in Rust core

## Safety Guarantees

### No Unsafe Code in Public API
- All unsafe blocks isolated to `InteropUtilities`, `NativeMethods`
- Public classes and methods are 100% safe C#
- Fixed blocks scoped to minimize unsafe regions

### Memory Safety
- No buffer overruns (strlen validation)
- No use-after-free (try/finally cleanup)
- No dangling pointers (immediate conversion to managed strings)
- GC handles pinned while in use, unpinned in finally

### Type Safety
- Strong typing for configuration objects
- JSON deserialization validates structure
- Enum types for known values (FormatType, etc.)
- Sealed classes prevent inheritance issues

### Exception Safety
- All exceptions inherit from `KreuzbergException`
- No silent failures (IntPtr.Zero checked immediately)
- Cleanup happens even on exceptions (finally blocks)
- Proper exception propagation to caller

## Interoperability with Rust Core

### FFI Boundary Contract

**C Function Signature** (from kreuzberg-ffi):
```c
const char* extract_file_sync(const char* path);
const char* last_error();
void free_string(const char* ptr);
```

**C# P/Invoke Declaration**:
```csharp
[DllImport("kreuzberg_ffi", CallingConvention = CallingConvention.Cdecl)]
public static extern IntPtr ExtractFileSync(IntPtr pathPtr);
```

**Type Mapping**:
- `const char*` ↔ `IntPtr` (pointer representation)
- UTF-8 encoded strings (C standard)
- Caller allocates input, native library allocates output
- Explicit deallocation via `free_string()` or `NativeMemory.Free()`

### Serialization Round-Trip

Rust to C# and back:
```
Rust: ExtractionConfig → JSON (Serde)
  ↓
C: char* (JSON string)
  ↓
C#: JSON → ExtractionConfig (System.Text.Json)
  ↓
C#: ExtractionResult → JSON (System.Text.Json)
  ↓
C: char* (JSON string)
  ↓
Rust: JSON → ExtractionResult (Serde)
```

All values use snake_case in JSON for Rust convention compatibility.

## Performance Considerations

### Memory Allocation Overhead
- Strings allocated per call (no pooling)
- Acceptable for typical extraction workflows
- Batch operations amortize allocation cost

### Async Overhead
- `Task.Run` has minimal overhead
- Benefits justify cost for UI applications
- Sync path available for high-throughput scenarios

### GCHandle Pinning
- Plugin callbacks pinned while registered
- Prevents GC moves (negligible performance cost)
- Freed immediately on unregister

## Testing Strategies

### Unit Tests
- Configuration serialization round-trips
- Error mapping for all exception types
- Memory cleanup in edge cases

### Integration Tests
- File extraction with real documents
- Batch processing correctness
- Concurrent calls from multiple threads
- Plugin registration/unregistration

### Stress Tests
- Large batch operations (100+ files)
- Long-running concurrent extractions
- Plugin chaining and priority ordering

## Extension Points

### Custom Post-Processors
Users can implement `IPostProcessor`:
```csharp
public interface IPostProcessor
{
    string Name { get; }
    int Priority { get; }
    ExtractionResult Process(ExtractionResult result);
}

KreuzbergClient.RegisterPostProcessor(new MyProcessor());
```

### Custom Validators
```csharp
public interface IValidator
{
    string Name { get; }
    int Priority { get; }
    void Validate(ExtractionResult result);
}

KreuzbergClient.RegisterValidator(new MyValidator());
```

### Custom OCR Backends
```csharp
public interface IOcrBackend
{
    string Name { get; }
    string Process(ReadOnlySpan<byte> imageBytes, OcrConfig? config);
}

KreuzbergClient.RegisterOcrBackend(new MyOcrBackend());
```

All callbacks are stateless and support concurrent invocation.

## Future Improvements

1. **Native Async**: If Rust core adds native async FFI, eliminate `Task.Run`
2. **Source Generators**: Auto-generate P/Invoke declarations from header file
3. **NativeAOT**: Make compatible with .NET NativeAOT compilation
4. **Span<char>**: Use `Span<char>` instead of string where appropriate
5. **ValueTask**: Use `ValueTask` for zero-allocation async in high-throughput scenarios

## Related Files

- `KreuzbergClient.cs` - Public API
- `Models.cs` - Configuration and result types
- `Errors.cs` - Exception hierarchy
- `NativeMethods.cs` - P/Invoke declarations
- `InteropUtilities.cs` - UTF-8 marshaling
- `Serialization.cs` - JSON serialization
