![X-Ray](https://raw.githubusercontent.com/curiosity-ai/x-ray-content/main/dotnet/src/XRay.Content/logo.png)

# X-Ray.Content

[![NuGet](https://img.shields.io/nuget/v/X-Ray.Content?label=NuGet&color=5c2d91&logo=nuget&logoColor=white)](https://www.nuget.org/packages/X-Ray.Content/)
[![Downloads](https://img.shields.io/nuget/dt/X-Ray.Content?label=downloads&color=5c2d91)](https://www.nuget.org/packages/X-Ray.Content/)
[![License](https://img.shields.io/badge/license-MIT%20AND%20Apache--2.0-007ec6)](https://github.com/curiosity-ai/x-ray-content/blob/main/dotnet/THIRD_PARTY_NOTICES.md)

Content extraction for .NET. Point it at a document — Office, PDF, HTML, email, an archive —
and get back the text, metadata, tables, images and a structured element tree, in whichever
output format you ask for.

`X-Ray.Content` is the content-extraction package of the **X-Ray** family of .NET libraries.
The engine is a native C# port of the Rust [`xberg`](https://github.com/xberg-io/xberg)
extraction engine: every extractor, type and renderer is reimplemented in managed C#, not
wrapped over a native library.

## Install

```sh
dotnet add package X-Ray.Content
```

The package id is hyphenated; the API lives under the `XRay.Content` namespace, since a hyphen is not
valid in a C# identifier.

Targets `net10.0`.

## Extract a document

```csharp
using XRay.Content.Core;

var extractor = new Extractor();

var result = extractor.Extract(
    ExtractInput.FromUri("quarterly-report.pdf"),
    new ExtractionConfig { OutputFormat = OutputFormat.Markdown });

var doc = result.Results[0];

Console.WriteLine(doc.MimeType);              // application/pdf
Console.WriteLine(doc.Content);               // the rendered Markdown
Console.WriteLine(doc.Metadata.Title);
Console.WriteLine(doc.Tables.Count);
```

Bytes work the same way when you already hold the file:

```csharp
var input = ExtractInput.FromBytes(bytes, "application/vnd.openxmlformats-officedocument.wordprocessingml.document");
```

`Extract` never throws for a bad document: a failure lands in `result.Errors` with a type and
code, and an unsupported MIME type comes back as an empty document carrying a
`ProcessingWarning`. `ExtractAsync` is available for the same call.

## Output formats

`OutputFormat` selects how `ExtractedDocument.Content` is rendered — the extracted document
model is the same either way:

| Format | Value |
|---|---|
| Plain text (default) | `OutputFormat.Plain` |
| Markdown (GFM) | `OutputFormat.Markdown` |
| HTML | `OutputFormat.Html` |
| Djot | `OutputFormat.Djot` |
| JSON section tree | `OutputFormat.Json` |
| Structured element list | `OutputFormat.Structured` |
| DocTags | `OutputFormat.DocTags` |

Set `IncludeDocumentStructure = true` to also get the `Document` heading tree, and
`ResultFormat.ElementBased` to get the flat element stream instead of one rendered string.

## Formats it reads

Word, Excel, PowerPoint (modern and legacy binary), OpenDocument, RTF, WordPerfect, PDF,
HTML/XML, Markdown/Djot/AsciiDoc/reStructuredText/Org/Typst/LaTeX, EPUB and FictionBook,
email (EML, MSG, PST), iWork, HWP/HWPX, images (metadata and EXIF), Photoshop PSD/PSB (layer tree
and type-layer text), CSV/TSV/DBF, JSON/YAML/TOML,
BibTeX and citation formats, Jupyter notebooks, source code, and ZIP/TAR/7z/GZip archives
(extracted recursively as child documents).

## Optional OCR

Extraction is pure managed and pulls no native dependencies. OCR is the one exception, and it
is **off by default**:

```csharp
var config = new ExtractionConfig
{
    Ocr = new OcrOptions { Mode = OcrMode.ScanOnly },   // or AllImages
};
```

`ScanOnly` recognises the PDF pages a scan detector flags; `AllImages` also reads every
embedded image and inserts its text inline after the image it came from. OCR is additive —
native text is never replaced — and never fatal: a missing checkpoint or an undecodable image
becomes a `ProcessingWarning`, not an exception.

Turning it on loads [PaddleOCR](https://github.com/curiosity-ai/PaddleOCR/), which brings
SkiaSharp and PDFium. A caller that never sets `Ocr` loads neither. Nothing is downloaded on
your behalf: point `OcrOptions.ModelDirectory` at a checkpoint you already have.

## Links

- [Source](https://github.com/curiosity-ai/x-ray-content/tree/main/dotnet) — the port lives under `dotnet/` in the `curiosity-ai/x-ray-content` repository
- [Architecture, scope and conventions](https://github.com/curiosity-ai/x-ray-content/blob/main/CLAUDE.md)
- [Port status and work breakdown](https://github.com/curiosity-ai/x-ray-content/blob/main/dotnet/TODO.md)
- [Issues](https://github.com/curiosity-ai/x-ray-content/issues)

## License

`MIT AND Apache-2.0`. The package is MIT, except for a handful of files that are derivative
works of Apache-2.0-only Rust crates and remain under that license — see
[THIRD_PARTY_NOTICES.md](https://github.com/curiosity-ai/x-ray-content/blob/main/dotnet/THIRD_PARTY_NOTICES.md)
for exactly which files and which upstreams.
