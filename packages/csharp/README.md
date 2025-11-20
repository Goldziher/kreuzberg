# Kreuzberg C# Bindings

High-performance Kreuzberg document intelligence for .NET via the Rust core.

## Installation

```bash
dotnet add package Goldziher.Kreuzberg
```

> The native library `kreuzberg_ffi` must be loadable (place `libkreuzberg_ffi.{dll|dylib|so}` on PATH/DYLD_LIBRARY_PATH/LD_LIBRARY_PATH).

## Usage

```csharp
using Kreuzberg;

var result = KreuzbergClient.ExtractFileSync("document.pdf");
Console.WriteLine(result.Content);
Console.WriteLine(result.MimeType);
```

### Batch

```csharp
var files = new[] { "doc1.pdf", "doc2.docx" };
var results = KreuzbergClient.BatchExtractFilesSync(files);
```

### Bytes

```csharp
var data = await File.ReadAllBytesAsync("document.pdf");
var result = KreuzbergClient.ExtractBytesSync(data, "application/pdf");
```

### Async wrappers

```csharp
var result = await KreuzbergClient.ExtractFileAsync("document.pdf");
```

### Version

```csharp
Console.WriteLine(KreuzbergClient.GetVersion());
```
