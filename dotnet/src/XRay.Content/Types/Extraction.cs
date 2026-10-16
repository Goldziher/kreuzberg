using System.Text.Json;
using System.Text.Json.Serialization;

namespace XRay.Content.Types;

/// <summary>Bounding box in document coordinates.</summary>
[JsonConverter(typeof(BoundingBoxConverter))]
public sealed class BoundingBox
{
    public double X0 { get; set; }
    public double Y0 { get; set; }
    public double X1 { get; set; }
    public double Y1 { get; set; }
}

/// <summary>Serializes <see cref="BoundingBox"/> coordinates the way Rust's `serde_json`
/// renders `f64`: whole numbers keep a trailing `.0` (e.g. <c>72.0</c>, not <c>72</c>), so
/// the golden reference files compare equal. System.Text.Json's default `double` writer
/// drops the fractional part for integral values, which broke byte-exact parity for every
/// table/image bounding box with integer edges.</summary>
public sealed class BoundingBoxConverter : JsonConverter<BoundingBox>
{
    public override BoundingBox Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        double x0 = 0, y0 = 0, x1 = 0, y1 = 0;
        if (reader.TokenType != JsonTokenType.StartObject) throw new JsonException();
        while (reader.Read() && reader.TokenType != JsonTokenType.EndObject)
        {
            if (reader.TokenType != JsonTokenType.PropertyName) continue;
            string name = reader.GetString()!;
            reader.Read();
            double v = reader.TokenType == JsonTokenType.Number ? reader.GetDouble() : 0;
            switch (name)
            {
                case "x0": x0 = v; break;
                case "y0": y0 = v; break;
                case "x1": x1 = v; break;
                case "y1": y1 = v; break;
            }
        }
        return new BoundingBox { X0 = x0, Y0 = y0, X1 = x1, Y1 = y1 };
    }

    public override void Write(Utf8JsonWriter writer, BoundingBox value, JsonSerializerOptions options)
    {
        writer.WriteStartObject();
        WriteCoord(writer, "x0", value.X0);
        WriteCoord(writer, "y0", value.Y0);
        WriteCoord(writer, "x1", value.X1);
        WriteCoord(writer, "y1", value.Y1);
        writer.WriteEndObject();
    }

    private static void WriteCoord(Utf8JsonWriter writer, string name, double v)
    {
        writer.WritePropertyName(name);
        if (double.IsFinite(v) && v == Math.Floor(v) && Math.Abs(v) < 9.2e18)
            writer.WriteRawValue(((long)v).ToString(System.Globalization.CultureInfo.InvariantCulture) + ".0");
        else
            writer.WriteNumberValue(v);
    }
}

/// <summary>Extraction method. Serialized as a bare snake_case string.</summary>
public enum ExtractionMethod
{
    Native,
    Ocr,
    Mixed,
}

/// <summary>Result output shape. Default = Unified.</summary>
public enum ResultFormat
{
    Unified,
    ElementBased,
}

/// <summary>Coarse classification of an extracted image. Serialized as bare snake_case string.</summary>
public enum ImageKind
{
    Photograph,
    Diagram,
    Chart,
    Drawing,
    TextBlock,
    Decoration,
    Logo,
    Icon,
    TileFragment,
    Mask,
    PageRaster,
    Unknown,
}

/// <summary>Serializes a byte[] as a JSON array of u8 numbers (matches serde's default for `Bytes`).</summary>
public sealed class BytesAsU8ArrayConverter : JsonConverter<byte[]>
{
    public override byte[] Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        var list = new List<byte>();
        if (reader.TokenType == JsonTokenType.StartArray)
        {
            while (reader.Read() && reader.TokenType != JsonTokenType.EndArray)
                list.Add(reader.GetByte());
        }
        return list.ToArray();
    }

    public override void Write(Utf8JsonWriter writer, byte[] value, JsonSerializerOptions options)
    {
        writer.WriteStartArray();
        foreach (var b in value)
            writer.WriteNumberValue(b);
        writer.WriteEndArray();
    }
}

/// <summary>An extracted image (binary data plus metadata). Referenced by index from elements.</summary>
public sealed class ExtractedImage
{
    [JsonConverter(typeof(BytesAsU8ArrayConverter))]
    public byte[] Data { get; set; } = Array.Empty<byte>();

    public string Format { get; set; } = "";
    public uint ImageIndex { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public uint? PageNumber { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public uint? Width { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public uint? Height { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Colorspace { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public uint? BitsPerComponent { get; set; }
    public bool IsMask { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Description { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public ExtractedDocument? OcrResult { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public BoundingBox? BoundingBox { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? SourcePath { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public ImageKind? ImageKind { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public float? KindConfidence { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public uint? ClusterId { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? Caption { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public List<object>? QrCodes { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public string? DataBase64 { get; set; }
}

/// <summary>Non-fatal warning collected during extraction.</summary>
public sealed class ProcessingWarning
{
    public string Source { get; set; } = "";
    public string Message { get; set; } = "";
}

/// <summary>Chunk-for-RAG (stub — not populated by the native path).</summary>
public sealed class Chunk
{
    public string Content { get; set; } = "";
    public string ChunkType { get; set; } = "unknown";
}

/// <summary>Element type for the element-based output. Serialized as bare snake_case string.</summary>
public enum ElementType
{
    Title,
    NarrativeText,
    Heading,
    ListItem,
    Table,
    Image,
    PageBreak,
    CodeBlock,
    BlockQuote,
    Footer,
    Header,
}

public sealed class ElementMetadata
{
    public uint? PageNumber { get; set; }
    public string? Filename { get; set; }
    public BoundingBox? Coordinates { get; set; }
    public long? ElementIndex { get; set; }
    public Dictionary<string, string> Additional { get; set; } = new();
}

/// <summary>Element-based output item.</summary>
public sealed class Element
{
    public ElementType ElementType { get; set; }
    public string Text { get; set; } = "";
    public ElementMetadata Metadata { get; set; } = new();
}

/// <summary>An archive child result.</summary>
public sealed class ArchiveEntry
{
    public string Path { get; set; } = "";
    public string MimeType { get; set; } = "";
    public ExtractedDocument Result { get; set; } = new();
}

/// <summary>
/// The public extraction result. Only the content-extraction fields are kept in the port
/// (chunks/embeddings/ocr_elements/keywords/quality_score/llm_usage are dropped).
/// </summary>
public sealed class ExtractedDocument
{
    public string Content { get; set; } = "";
    public string MimeType { get; set; } = "";
    public Metadata Metadata { get; set; } = new();

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public ExtractionMethod? ExtractionMethod { get; set; }

    public List<Table> Tables { get; set; } = new();

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public List<string>? DetectedLanguages { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public List<ExtractedImage>? Images { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public List<PageContent>? Pages { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public List<Element>? Elements { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public object? DjotContent { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public DocumentStructure? Document { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)]
    public List<ProcessingWarning> ProcessingWarnings { get; set; } = new();

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public List<PdfAnnotation>? Annotations { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public List<ArchiveEntry>? Children { get; set; }

    /// <summary>
    /// Files attached to the document (a PDF's embedded files, file-attachment annotations and
    /// portfolio members): name, type, size and, when asked for, bytes. Null when there are none.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public List<ExtractedAttachment>? Attachments { get; set; }

    /// <summary>
    /// The interactive form fields of a PDF (text, checkbox, radio, choice, button, signature)
    /// with their values, options, page and rectangle. Null when the document has no AcroForm.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public List<PdfFormField>? FormFields { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public List<ExtractedUri>? Uris { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public List<object>? Revisions { get; set; }

    /// <summary>Per-format render produced by derive; the pipeline swaps it into <see cref="Content"/>.</summary>
    [JsonIgnore]
    public string? FormattedContent { get; set; }

    /// <summary>
    /// The document as plain text, whatever <see cref="ExtractionConfig.OutputFormat"/> asked for.
    /// </summary>
    /// <remarks>
    /// Derive renders plain text on the way to every format — the formatted render is layered over
    /// it — so a caller that wants the document in markup *and* as text can have both out of one
    /// extraction instead of running the whole pipeline twice. That matters most when the
    /// extraction is expensive for reasons other than parsing: the OCR pass recognises once.
    /// It is the same string as <see cref="Content"/> when the output format is plain, and
    /// <c>[JsonIgnore]</c> like <see cref="FormattedContent"/>, so nothing about the wire format
    /// changes.
    /// </remarks>
    [JsonIgnore]
    public string? PlainContent { get; set; }
}
