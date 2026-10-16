using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization;

namespace XRay.Content.Types;

/// <summary>Document-level metadata. Field order matches Rust `Metadata`.</summary>
public sealed class Metadata
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Title { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Subject { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public List<string>? Authors { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public List<string>? Keywords { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Language { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? CreatedAt { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? ModifiedAt { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? CreatedBy { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? ModifiedBy { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public PageStructure? Pages { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public FormatMetadata? Format { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public object? ImagePreprocessing { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public JsonElement? JsonSchema { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public ErrorMetadata? Error { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public ulong? ExtractionDurationMs { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Category { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public List<string>? Tags { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? DocumentVersion { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? AbstractText { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? OutputFormat { get; set; }
    public bool OcrUsed { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)]
    public Dictionary<string, JsonElement> Additional { get; set; } = new();
}

public sealed class ErrorMetadata
{
    public string ErrorType { get; set; } = "";
    public string Message { get; set; } = "";
}

/// <summary>
/// Format-specific metadata. Internally-tagged on "format_type"; the converter writes the tag
/// followed by the payload struct's fields (flattened), matching Rust's serde output.
/// </summary>
[JsonConverter(typeof(FormatMetadataConverter))]
public sealed class FormatMetadata
{
    public string FormatType { get; set; } = "text";
    public object? Payload { get; set; }

    public static FormatMetadata Text(TextMetadata m) => new() { FormatType = "text", Payload = m };
    public static FormatMetadata Excel(ExcelMetadata m) => new() { FormatType = "excel", Payload = m };
    public static FormatMetadata Csv(CsvMetadata m) => new() { FormatType = "csv", Payload = m };
    public static FormatMetadata Html(HtmlMetadata m) => new() { FormatType = "html", Payload = m };
    public static FormatMetadata Xml(XmlMetadata m) => new() { FormatType = "xml", Payload = m };
    public static FormatMetadata Image(ImageMetadata m) => new() { FormatType = "image", Payload = m };
    public static FormatMetadata Code(CodeMetadata m) => new() { FormatType = "code", Payload = m };

    private static readonly Dictionary<string, Type> PayloadTypes = new()
    {
        ["text"] = typeof(TextMetadata),
        ["excel"] = typeof(ExcelMetadata),
        ["csv"] = typeof(CsvMetadata),
        ["html"] = typeof(HtmlMetadata),
        ["xml"] = typeof(XmlMetadata),
        ["image"] = typeof(ImageMetadata),
        ["email"] = typeof(EmailMetadata),
        ["pptx"] = typeof(PptxMetadata),
        ["docx"] = typeof(DocxMetadata),
        ["archive"] = typeof(ArchiveMetadata),
        ["pdf"] = typeof(PdfMetadata),
        ["bibtex"] = typeof(BibtexMetadata),
        ["citation"] = typeof(CitationMetadata),
        ["fiction_book"] = typeof(FictionBookMetadata),
        ["dbf"] = typeof(DbfMetadata),
        ["jats"] = typeof(JatsMetadata),
        ["epub"] = typeof(EpubMetadata),
        ["pst"] = typeof(PstMetadata),
    };

    public static Type? TypeForTag(string tag) => PayloadTypes.TryGetValue(tag, out var t) ? t : null;
}

public sealed class FormatMetadataConverter : JsonConverter<FormatMetadata>
{
    public override FormatMetadata Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using var doc = JsonDocument.ParseValue(ref reader);
        var root = doc.RootElement;
        string tag = root.GetProperty("format_type").GetString()!;
        var payloadType = FormatMetadata.TypeForTag(tag);
        object? payload = null;
        if (payloadType is not null)
            payload = root.Deserialize(payloadType, options);
        return new FormatMetadata { FormatType = tag, Payload = payload };
    }

    public override void Write(Utf8JsonWriter writer, FormatMetadata value, JsonSerializerOptions options)
    {
        writer.WriteStartObject();
        writer.WriteString("format_type", value.FormatType);
        if (value.Payload is not null)
        {
            var node = JsonSerializer.SerializeToNode(value.Payload, value.Payload.GetType(), options);
            if (node is JsonObject obj)
            {
                foreach (var kvp in obj)
                {
                    writer.WritePropertyName(kvp.Key);
                    (kvp.Value ?? JsonValue.Create((string?)null)).WriteTo(writer, options);
                }
            }
        }
        writer.WriteEndObject();
    }
}

// ============================================================================
// Format metadata payload structs
// ============================================================================

public sealed class TextMetadata
{
    public uint LineCount { get; set; }
    public uint WordCount { get; set; }
    public uint CharacterCount { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public List<string>? Headers { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public List<MarkdownLink>? Links { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public List<MarkdownCodeBlock>? CodeBlocks { get; set; }
}

/// <summary>A Markdown link's visible text and destination.</summary>
/// <remarks>
/// Serialized as a named object. Upstream carried these as positional two-element arrays until
/// <c>fix(types): serialize named structs as JSON objects, not tuples</c>, which is the shape
/// every shipped binding — this one included — already expected.
/// </remarks>
public sealed class MarkdownLink
{
    public string Text { get; set; } = "";
    public string Url { get; set; } = "";
}

/// <summary>A fenced code block's language tag and body.</summary>
public sealed class MarkdownCodeBlock
{
    public string Language { get; set; } = "";
    public string Code { get; set; } = "";
}

public sealed class ExcelMetadata
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public uint? SheetCount { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public List<string>? SheetNames { get; set; }
}

public sealed class CsvMetadata
{
    public uint RowCount { get; set; }
    public uint ColumnCount { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Delimiter { get; set; }
    public bool HasHeader { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public List<string>? ColumnTypes { get; set; }
}

public sealed class XmlMetadata
{
    public uint ElementCount { get; set; }
    public List<string> UniqueElements { get; set; } = new();
}

/// <summary>
/// Source-code metadata. <c>chunks</c> is always serialized, even when empty, because upstream's
/// <c>Vec</c> is.
/// </summary>
public sealed class CodeMetadata
{
    /// <summary>Structural chunks — function, class and module boundaries.</summary>
    [JsonPropertyName("chunks")]
    public List<CodeChunkInfo> Chunks { get; set; } = new();

    /// <summary>The key/value tree recovered from data-format source, when that was asked for.</summary>
    [JsonPropertyName("data")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public object? Data { get; set; }
}

/// <summary>One structurally-meaningful chunk of source.</summary>
public sealed class CodeChunkInfo
{
    [JsonPropertyName("text")] public string Text { get; set; } = "";
    [JsonPropertyName("context_path")] public List<string> ContextPath { get; set; } = new();
    [JsonPropertyName("node_types")] public List<string> NodeTypes { get; set; } = new();
    [JsonPropertyName("byte_start")] public uint ByteStart { get; set; }
    [JsonPropertyName("byte_end")] public uint ByteEnd { get; set; }
}

public sealed class ImageMetadata
{
    public uint Width { get; set; }
    public uint Height { get; set; }
    public string Format { get; set; } = "";
    public Dictionary<string, string> Exif { get; set; } = new();
}

public enum TextDirection
{
    [JsonStringEnumMemberName("ltr")] LeftToRight,
    [JsonStringEnumMemberName("rtl")] RightToLeft,
    [JsonStringEnumMemberName("auto")] Auto,
}

public enum LinkType
{
    [JsonStringEnumMemberName("anchor")] Anchor,
    [JsonStringEnumMemberName("internal")] Internal,
    [JsonStringEnumMemberName("external")] External,
    [JsonStringEnumMemberName("email")] Email,
    [JsonStringEnumMemberName("phone")] Phone,
    [JsonStringEnumMemberName("other")] Other,
}

public enum ImageTypeKind
{
    [JsonStringEnumMemberName("data-uri")] DataUri,
    [JsonStringEnumMemberName("inline-svg")] InlineSvg,
    [JsonStringEnumMemberName("external")] External,
    [JsonStringEnumMemberName("relative")] Relative,
}

public enum StructuredDataType
{
    [JsonStringEnumMemberName("json-ld")] JsonLd,
    [JsonStringEnumMemberName("microdata")] Microdata,
    [JsonStringEnumMemberName("rdfa")] RDFa,
}

public sealed class HtmlMetadata
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Title { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Description { get; set; }
    public List<string> Keywords { get; set; } = new();
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Author { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? CanonicalUrl { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? BaseHref { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Language { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public TextDirection? TextDirection { get; set; }
    public Dictionary<string, string> OpenGraph { get; set; } = new();
    public Dictionary<string, string> TwitterCard { get; set; } = new();
    public Dictionary<string, string> MetaTags { get; set; } = new();
    public List<object> Headers { get; set; } = new();
    public List<object> Links { get; set; } = new();
    public List<object> Images { get; set; } = new();
    public List<object> StructuredData { get; set; } = new();
}

// --- Stub payloads for the less-common format variants ---

public sealed class EmailMetadata
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? FromEmail { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? FromName { get; set; }
    public List<string> ToEmails { get; set; } = new();
    public List<string> CcEmails { get; set; } = new();
    public List<string> BccEmails { get; set; } = new();
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? MessageId { get; set; }
    public List<string> Attachments { get; set; } = new();
}

public sealed class PptxMetadata
{
    public uint SlideCount { get; set; }
    public List<string> SlideNames { get; set; } = new();
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public uint? ImageCount { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public uint? TableCount { get; set; }
}

public sealed class DocxMetadata
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public JsonElement? CoreProperties { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public JsonElement? AppProperties { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public Dictionary<string, JsonElement>? CustomProperties { get; set; }
}

public sealed class ArchiveMetadata
{
    public string Format { get; set; } = "";
    public uint FileCount { get; set; }
    public List<string> FileList { get; set; } = new();
    public ulong TotalSize { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public ulong? CompressedSize { get; set; }
}

public sealed class PdfMetadata
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? PdfVersion { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Producer { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public bool? IsEncrypted { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public long? Width { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public long? Height { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public uint? PageCount { get; set; }

    /// <summary>Highest per-page scanned-page confidence in the document, in [0, 1].</summary>
    /// <remarks>
    /// Held as a double but always assigned from a <c>float</c>, so it serializes with the same
    /// digits Rust's <c>f32</c> produces once serde widens it — 0.85f prints as
    /// <c>0.8500000238418579</c>, not <c>0.85</c>. The value is a single-precision score either
    /// way; only the printed form differs, and the two must agree byte for byte.
    /// </remarks>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public double? ScannedConfidence { get; set; }

    /// <summary>One-based page numbers graded at or above the configured confidence.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public List<uint>? ScannedPages { get; set; }

    // ---- Port additions (no upstream counterpart); null for an unencrypted document. ----

    /// <summary>
    /// Whether a password beyond the empty one was needed to open the document. Always false
    /// when it is set at all, because a document that needed one and was not given it is not
    /// extracted: it fails with <see cref="Core.PdfPasswordRequiredException"/>.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public bool? RequiresPassword { get; set; }

    /// <summary>Which password opened it: <c>user</c> (including the empty one) or <c>owner</c>.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? AuthenticatedAs { get; set; }

    /// <summary><c>RC4</c>, <c>AESV2</c>, <c>AESV3</c> or <c>Identity</c>.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? EncryptionAlgorithm { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public int? EncryptionKeyBits { get; set; }

    /// <summary>The permission flags the owner set (print, copy, modify…).</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public PdfPermissions? Permissions { get; set; }

    /// <summary>The document outline (bookmarks) in order, with nesting depth and destination page.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public List<PdfOutlineItem>? Outline { get; set; }
}

/// <summary>The permission flags of an encrypted PDF's <c>/P</c> entry.</summary>
public sealed class PdfPermissions
{
    public int Raw { get; set; }
    public bool Print { get; set; }
    public bool Modify { get; set; }
    public bool Copy { get; set; }
    public bool Annotate { get; set; }
    public bool FillForms { get; set; }
    public bool ExtractForAccessibility { get; set; }
    public bool Assemble { get; set; }
    public bool PrintHighQuality { get; set; }
}

/// <summary>One bookmark of a PDF's outline.</summary>
public sealed class PdfOutlineItem
{
    public string Title { get; set; } = "";
    /// <summary>Nesting depth, 0 for a top-level bookmark.</summary>
    public int Level { get; set; }
    /// <summary>One-based destination page when it resolves within the document.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public int? Page { get; set; }
}

/// <summary>Year range for bibliographic metadata (Rust `YearRange`).</summary>
public sealed class YearRange
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public uint? Min { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public uint? Max { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)] public List<uint> Years { get; set; } = new();
}

public sealed class BibtexMetadata
{
    public long EntryCount { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)] public List<string> CitationKeys { get; set; } = new();
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)] public List<string> Authors { get; set; } = new();
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public YearRange? YearRange { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public SortedDictionary<string, long>? EntryTypes { get; set; }
}

public sealed class CitationMetadata
{
    public long CitationCount { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Format { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)] public List<string> Authors { get; set; } = new();
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public YearRange? YearRange { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)] public List<string> Dois { get; set; } = new();
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)] public List<string> Keywords { get; set; } = new();
}

public sealed class FictionBookMetadata
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)] public List<string> Genres { get; set; } = new();
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)] public List<string> Sequences { get; set; } = new();
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Annotation { get; set; }
}

public sealed class DbfFieldInfo
{
    public string Name { get; set; } = "";
    public string FieldType { get; set; } = "";
}

public sealed class DbfMetadata
{
    public long RecordCount { get; set; }
    public long FieldCount { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)] public List<DbfFieldInfo> Fields { get; set; } = new();
}

public sealed class ContributorRole
{
    public string Name { get; set; } = "";
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Role { get; set; }
}

public sealed class JatsMetadata
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Copyright { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? License { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)] public SortedDictionary<string, string> HistoryDates { get; set; } = new();
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)] public List<ContributorRole> ContributorRoles { get; set; } = new();
}
public sealed class EpubMetadata
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Coverage { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? DcFormat { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Relation { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Source { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? DcType { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? CoverImage { get; set; }
}
public sealed class PstMetadata { public long MessageCount { get; set; } }
